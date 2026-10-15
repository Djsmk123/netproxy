//! Sequenced event bus.
//!
//! Producers (the proxy data plane) call [`EventSink::emit`], which **never blocks and never
//! allocates unbounded memory**: when the bounded queue is full the event is dropped and counted.
//! A single sequencer task assigns contiguous `seq` numbers in queue order, so per-flow causal
//! order is preserved, and fans each envelope out to
//!   * an optional *durable* consumer (the store) with backpressure, and
//!   * live subscribers (`broadcast`), which may lag and must resync from the store.
//!
//! Drops are made visible: the next event that makes it into the queue carries the number of
//! events dropped just before it, and the sequencer materialises that as an `EventsDropped`
//! envelope immediately ahead of it.

use np_model::{Envelope, Event};
use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc,
};
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::{broadcast, mpsc, oneshot};

pub fn now_us() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_micros() as u64
}

enum Msg {
    Event { ts_us: u64, dropped_before: u64, event: Event },
    Close(oneshot::Sender<()>),
}

#[derive(Default)]
pub struct BusStats {
    pub sequenced: AtomicU64,
    pub dropped: AtomicU64,
}

#[derive(Clone)]
pub struct EventSink {
    tx: mpsc::Sender<Msg>,
    pending_drops: Arc<AtomicU64>,
    stats: Arc<BusStats>,
}

impl EventSink {
    /// Non-blocking. Safe to call from the data path.
    pub fn emit(&self, event: Event) {
        let dropped_before = self.pending_drops.swap(0, Ordering::AcqRel);
        let msg = Msg::Event { ts_us: now_us(), dropped_before, event };
        if self.tx.try_send(msg).is_err() {
            // Queue full (or closed): count this event plus whatever we just tried to hand over.
            self.pending_drops.fetch_add(dropped_before + 1, Ordering::AcqRel);
            self.stats.dropped.fetch_add(1, Ordering::Relaxed);
        }
    }
}

pub struct BusConfig {
    pub queue: usize,
    pub live_capacity: usize,
}

impl Default for BusConfig {
    fn default() -> Self {
        Self { queue: 65_536, live_capacity: 8_192 }
    }
}

pub struct Bus {
    run: String,
    sink: EventSink,
    live: broadcast::Sender<Arc<Envelope>>,
    stats: Arc<BusStats>,
}

impl Bus {
    /// `durable`: every envelope is *awaited* into this channel (backpressure on the sequencer,
    /// which surfaces as producer-side drops, never as data-path stalls).
    pub fn spawn(run: String, cfg: BusConfig, durable: Option<mpsc::Sender<Arc<Envelope>>>) -> Bus {
        let (tx, mut rx) = mpsc::channel::<Msg>(cfg.queue);
        let (live, _) = broadcast::channel(cfg.live_capacity);
        let stats = Arc::new(BusStats::default());
        let pending_drops = Arc::new(AtomicU64::new(0));
        let sink = EventSink { tx, pending_drops: pending_drops.clone(), stats: stats.clone() };

        let (run2, live2, stats2) = (run.clone(), live.clone(), stats.clone());
        tokio::spawn(async move {
            let mut seq = 0u64;
            let forward = |env: Envelope| {
                let env = Arc::new(env);
                stats2.sequenced.fetch_add(1, Ordering::Relaxed);
                let _ = live2.send(env.clone());
                env
            };
            while let Some(msg) = rx.recv().await {
                match msg {
                    Msg::Event { ts_us, dropped_before, event } => {
                        let mut batch = Vec::with_capacity(2);
                        if dropped_before > 0 {
                            seq += 1;
                            batch.push(forward(Envelope { run: run2.clone(), seq, ts_us, event: Event::EventsDropped { count: dropped_before } }));
                        }
                        seq += 1;
                        batch.push(forward(Envelope { run: run2.clone(), seq, ts_us, event }));
                        if let Some(d) = &durable {
                            for env in batch {
                                if d.send(env).await.is_err() {
                                    break;
                                }
                            }
                        }
                    }
                    Msg::Close(done) => {
                        // Account for drops that happened after the last delivered event.
                        let tail = pending_drops.swap(0, Ordering::AcqRel);
                        if tail > 0 {
                            seq += 1;
                            let env = forward(Envelope { run: run2.clone(), seq, ts_us: now_us(), event: Event::EventsDropped { count: tail } });
                            if let Some(d) = &durable {
                                let _ = d.send(env).await;
                            }
                        }
                        let _ = done.send(());
                        return;
                    }
                }
            }
        });
        Bus { run, sink, live, stats }
    }

    pub fn run(&self) -> &str {
        &self.run
    }

    pub fn sink(&self) -> EventSink {
        self.sink.clone()
    }

    pub fn subscribe(&self) -> broadcast::Receiver<Arc<Envelope>> {
        self.live.subscribe()
    }

    pub fn stats(&self) -> Arc<BusStats> {
        self.stats.clone()
    }

    /// Flushes everything queued so far (awaiting, not dropping) and stops the sequencer.
    pub async fn close(&self) {
        let (tx, rx) = oneshot::channel();
        if self.sink.tx.send(Msg::Close(tx)).await.is_ok() {
            let _ = rx.await;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use np_model::FlowId;

    fn ev(i: u64) -> Event {
        Event::MockHit { flow: FlowId(i), rule: "r".into() }
    }

    #[tokio::test]
    async fn seq_is_contiguous_and_in_order() {
        let (dtx, mut drx) = mpsc::channel(1024);
        let bus = Bus::spawn("r".into(), BusConfig::default(), Some(dtx));
        let sink = bus.sink();
        for i in 0..1000 {
            sink.emit(ev(i));
        }
        bus.close().await;
        let mut expect = 1;
        while let Ok(e) = drx.try_recv() {
            assert_eq!(e.seq, expect);
            expect += 1;
        }
        assert_eq!(expect, 1001);
    }

    #[tokio::test]
    async fn drops_are_marked_and_seq_stays_contiguous() {
        // Tiny queue + a durable consumer that is not reading => sequencer stalls => producers drop.
        let (dtx, mut drx) = mpsc::channel(1);
        let bus = Bus::spawn("r".into(), BusConfig { queue: 8, live_capacity: 16 }, Some(dtx));
        let sink = bus.sink();
        for i in 0..200 {
            sink.emit(ev(i));
        }
        assert!(bus.stats().dropped.load(Ordering::Relaxed) > 0, "expected producer-side drops");
        // Now drain concurrently while closing.
        let drain = tokio::spawn(async move {
            let mut out = vec![];
            while let Some(e) = drx.recv().await {
                out.push(e);
            }
            out
        });
        bus.close().await;
        drop(bus);
        let out = drain.await.unwrap();
        // contiguous seq
        for (i, e) in out.iter().enumerate() {
            assert_eq!(e.seq, i as u64 + 1);
        }
        // conservation: delivered + dropped == emitted
        let delivered = out.iter().filter(|e| !matches!(e.event, Event::EventsDropped { .. })).count() as u64;
        let dropped: u64 = out.iter().map(|e| if let Event::EventsDropped { count } = e.event { count } else { 0 }).sum();
        assert_eq!(delivered + dropped, 200, "every emitted event is either delivered or counted");
    }

    #[tokio::test]
    async fn emit_never_blocks_when_nobody_consumes() {
        let (dtx, _keep) = mpsc::channel(1);
        let bus = Bus::spawn("r".into(), BusConfig { queue: 4, live_capacity: 4 }, Some(dtx));
        let sink = bus.sink();
        let t = std::time::Instant::now();
        for i in 0..100_000 {
            sink.emit(ev(i));
        }
        assert!(t.elapsed() < std::time::Duration::from_secs(2));
    }
}
