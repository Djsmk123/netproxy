//! Standalone upstream for manual runs. Usage: np-harness [--port 9443] [--ca-out upstream-ca.pem]
fn args(k: &str, d: &str) -> String {
    let a: Vec<String> = std::env::args().collect();
    a.iter()
        .position(|x| x == k)
        .and_then(|i| a.get(i + 1))
        .cloned()
        .unwrap_or(d.into())
}

#[tokio::main]
async fn main() {
    let base: u16 = args("--port", "9443").parse().unwrap();
    let up = np_harness::start(base).await;
    std::fs::write(args("--ca-out", "upstream-ca.pem"), &up.ca_pem).unwrap();
    std::future::pending::<()>().await;
}
