//! Latency/throughput client. Compares direct vs via-proxy against the harness upstream.
//! Usage: np-bench --upstream-ca up-ca.pem --proxy-ca px-ca.pem [--proxy http://127.0.0.1:8899] [--n 5000]
use std::time::{Duration, Instant};

fn arg(k: &str, d: &str) -> String {
    let a: Vec<String> = std::env::args().collect();
    a.iter()
        .position(|x| x == k)
        .and_then(|i| a.get(i + 1))
        .cloned()
        .unwrap_or(d.into())
}

fn pct(v: &[f64], p: f64) -> f64 {
    v[((v.len() as f64 - 1.0) * p) as usize]
}

fn client(ca: &str, proxy: Option<&str>, h2: bool) -> reqwest::Client {
    let cert = reqwest::Certificate::from_pem(&std::fs::read(ca).unwrap()).unwrap();
    let mut b = reqwest::Client::builder()
        .add_root_certificate(cert)
        .pool_max_idle_per_host(128);
    b = if h2 {
        b.http2_prior_knowledge()
    } else {
        b.http1_only()
    };
    if let Some(p) = proxy {
        b = b.proxy(reqwest::Proxy::all(p).unwrap());
    }
    b.build().unwrap()
}

async fn seq(c: &reqwest::Client, url: &str, n: usize) -> Vec<f64> {
    for _ in 0..300 {
        let _ = c.get(url).send().await.unwrap().bytes().await.unwrap();
    }
    let mut v = Vec::with_capacity(n);
    for _ in 0..n {
        let t = Instant::now();
        let _ = c.get(url).send().await.unwrap().bytes().await.unwrap();
        v.push(t.elapsed().as_secs_f64() * 1000.0);
    }
    v.sort_by(|a, b| a.partial_cmp(b).unwrap());
    v
}

async fn conc(c: &reqwest::Client, url: &str, total: usize, par: usize) -> (f64, usize) {
    let t = Instant::now();
    let sem = std::sync::Arc::new(tokio::sync::Semaphore::new(par));
    let mut hs = vec![];
    for _ in 0..total {
        let (c, url, sem) = (c.clone(), url.to_string(), sem.clone());
        hs.push(tokio::spawn(async move {
            let _p = sem.acquire().await.unwrap();
            match c.get(url).send().await {
                Ok(r) => r.bytes().await.is_err(),
                Err(_) => true,
            }
        }));
    }
    let mut errs = 0;
    for h in hs {
        if h.await.unwrap_or(true) {
            errs += 1;
        }
    }
    (total as f64 / t.elapsed().as_secs_f64(), errs)
}

#[tokio::main]
async fn main() {
    let soak: u64 = arg("--soak", "0").parse().unwrap();
    if soak > 0 {
        // Sustained mixed load via proxy for `soak` seconds (small + 5 MiB bodies, h1 and h2).
        let pca = arg("--proxy-ca", "px-ca.pem");
        let proxy = arg("--proxy", "http://127.0.0.1:8899");
        let direct = arg("--direct", "0") == "1";
        let par: usize = arg("--par", "64").parse().unwrap();
        let uca = arg("--upstream-ca", "up-ca.pem");
        let px = if direct { None } else { Some(proxy.as_str()) };
        let ca = if direct { uca.as_str() } else { pca.as_str() };
        let (c1, c2) = (client(ca, px, false), client(ca, px, true));
        let mut errs = [0usize; 2];
        let end = Instant::now() + Duration::from_secs(soak);
        let mut done = 0usize;
        while Instant::now() < end {
            for (i, c) in [&c1, &c2].into_iter().enumerate() {
                errs[i] += conc(c, "https://localhost:9443/small", 4000, par).await.1;
                errs[i] += conc(c, "https://localhost:9443/big?mb=5", 16, 8.min(par))
                    .await
                    .1;
                done += 4016;
            }
        }
        println!(
            "soak done: {done} requests in {soak}s; errors h1={} h2={}",
            errs[0], errs[1]
        );
        return;
    }
    let n: usize = arg("--n", "5000").parse().unwrap();
    let url = "https://localhost:9443/small";
    let proxy = arg("--proxy", "http://127.0.0.1:8899");
    let (uca, pca) = (
        arg("--upstream-ca", "up-ca.pem"),
        arg("--proxy-ca", "px-ca.pem"),
    );
    for (label, h2) in [("HTTP/1.1", false), ("HTTP/2", true)] {
        let d = seq(&client(&uca, None, h2), url, n).await;
        let p = seq(&client(&pca, Some(&proxy), h2), url, n).await;
        println!("{label} sequential n={n} (ms)   direct p50={:.3} p95={:.3} p99={:.3} | proxy p50={:.3} p95={:.3} p99={:.3} | ADDED p50={:.3} p95={:.3}",
            pct(&d,0.5), pct(&d,0.95), pct(&d,0.99), pct(&p,0.5), pct(&p,0.95), pct(&p,0.99),
            pct(&p,0.5)-pct(&d,0.5), pct(&p,0.95)-pct(&d,0.95));
        let (dr, _) = conc(&client(&uca, None, h2), url, 20000, 64).await;
        let (pr, _) = conc(&client(&pca, Some(&proxy), h2), url, 20000, 64).await;
        println!(
            "{label} throughput c=64: direct {dr:.0} rps | proxy {pr:.0} rps ({:.0}%)",
            pr / dr * 100.0
        );
    }
    let _ = Duration::from_secs(0);
}
