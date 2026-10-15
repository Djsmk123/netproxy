# M1 — Foundation: engine contract and traffic model

**Exit gate (plan §11): "reliable ordered capture events independent of UI" — met**, with automated proof.
Run everything: `cargo test --workspace` (19 tests, ~15 s) and `cargo test -p np-store --release -- --ignored --nocapture`.

## What exists (`crates/`)
| Crate | Responsibility |
|---|---|
| `np-model` | Canonical `Event`/`Envelope` (18 event kinds), IDs, `FlowState` ordering state machine. Pure data, no I/O. |
| `np-engine` | `ProxyEngine` trait + `MockResolver`/`BodySink` hooks; sequenced `Bus`; `HudsuckerEngine` (hudsucker 0.25 behind the trait); CA management; TLS-rejection bridge. |
| `np-store` | SQLite (WAL): append-only event log + `flows` projection in one transaction; keyset queries; crash recovery; retention. |
| `netproxyd` | Daemon: run lifecycle, file-backed body store, authenticated Unix-socket control API (`docs/control-api.md`). |
| `harness` | Deterministic upstream (h1/h2, big/slow/reset/redirect/echo/WebSocket, bad-cert hosts) as a library + bench client. |

## Design decisions that matter
- **Capture never blocks.** Producers `try_send` into a bounded queue; overflow is counted and re-materialised as an `events_dropped`
  marker at the right place in `seq`. Conservation (`delivered + dropped == emitted`) is a unit test.
- **The store is the source of truth for subscribers.** Live clients read the log in order and are woken by commits, so there is no
  "lagged subscriber" failure mode.
- **Everything behind traits** (`ProxyEngine`, `MockResolver`, `BodySink`): hudsucker is replaceable; M3's rule runtime plugs into
  `MockResolver`; the real mock decision stays synchronous and I/O-free on the data path.
- **Honest limits stay visible:** client-side TLS rejections become `client_tls_failed` events with a class (`unknown_ca`,
  `certificate` for Android, `closed_during_handshake` for pinning-style closes).

## Defects found and fixed while building the gate
| Found by | Defect | Fix |
|---|---|---|
| Spike review | Flows with empty response bodies (redirects, 204) never reached a terminal state; WebSocket handshakes never completed | Terminal events emitted explicitly (mutation-tested: removing the fix fails the suite) |
| Conformance | Drop marker could land at the wrong position in `seq` | Drop count travels with the next queued event |
| Soak | Pruning from a second connection starved the capture writer (33–50 % events dropped) | Writer prunes itself in time-boxed steps |
| Profiling | Flow-pruning scanned the whole table each step (6 s on 414k rows) | Bound the delete by the chunk's flow-id range |
| Soak | `runs.list` was O(events) (23 ms at 400k events, growing) | Incremental `flow_count` + seq span: 0.36 ms, constant |
| Body soak | Disk filled (my own scripts leaked ~19 GB of scratch dirs) | Scripts now clean up unless `KEEP=1`; per-file cleanup verified |
| Body soak | Body write queue bounded by count (256 MiB worst case); cap enforcement overshot 60 % | Byte budget (32 MiB), 1 s enforcement; RSS 146 → 74–87 MB; bodies 984 MB vs 1024 MB cap |

## Measured (Apple Silicon, loopback; client + proxy + upstream share the machine)
| Metric | Result | Target |
|---|---|---|
| Added latency, full pipeline (engine + sequencer + SQLite + body capture) | p50 0.031–0.042 ms, p95 0.035–0.055 ms (h1); p50 ≈ 0.05 ms (h2) | ≤ 10 / 25 ms |
| Sustained event persistence | 61 k events/s for 120 s, **0 dropped**, RSS flat 58 MB (no retention) | no event loss |
| Retention on, ~10 k flows/s for 60–100 s | **0 dropped**; DB pinned at cap (312 MB of 314 MB); file flat | bounded disk |
| Retention under stress (~12 k flows/s, 64 concurrent) | ~4 % of events dropped (counted, marked) | explicit loss only |
| 100 k-flow store, indexed queries | p95 ≤ 40 ms (text search); latest page 0.13 ms | < 250 ms |
| Insert / raw delete throughput | 199 k / 324 k events/s | – |
| Body store | Byte-exact request and response bodies, hash covers the full body when storage is capped; inline ≤ 4 KiB bodies need no file | – |

## Known limitations (carry into M2+)
1. **CA private key is a 0600 file**, not in the Keychain; no rotation / per-device profile flow yet (M2: certificate lifecycle).
2. **TLS-rejection detection parses hudsucker's `tracing` output.** It is tested but couples us to its log format; keep hudsucker pinned and consider a patch.
3. **WebSocket frames (`ws_message`) are not linked to their handshake flow**, only to the client address. Mocked flows do not capture request bodies.
4. **Bodies are stored as on the wire** (no content-encoding decode); the inspector must decode gzip/br itself.
5. **Header values are lossy UTF-8**; binary header bytes are not preserved.
6. **No scoped authorization or audit table yet** (`agent_action` in the plan): any authenticated client may start/stop the engine. Required before MCP (M4).
7. Text search is a `LIKE` scan (≈ 40 ms at 100 k flows); SQLite FTS5 if it becomes a bottleneck.
8. `retention` bounds the SQLite database (live pages), not the file on disk, which stays at its high-water mark; bodies have their own cap.
9. Under extreme sustained overload (>10 k flows/s with a tight DB cap) events are dropped (and marked) while traffic is unaffected — by design, but the UI must surface `events_dropped`.
10. Still unverified: physical iOS/Android devices, Chrome on Android, HTTP/3/QUIC fallback, Developer ID signing + notarization.

## Next: M2 (native macOS inspector)
SwiftUI app consuming `docs/control-api.md`: traffic list (`flows.list` + `events.subscribe`), request inspector (`flows.get`, `bodies.get`),
filters/search, session persistence (already durable), device setup + certificate diagnostics (`client_tls_failed`, `ca.get`).
Prerequisites to do first in M2: Keychain-backed CA and install/trust flow; daemon supervision from the app bundle (helper already proven in M0).
