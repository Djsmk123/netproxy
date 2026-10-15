# M0 — Engine Spike Plan

Source of truth: `~/Downloads/plan.html`. This document is planning only. No implementation code is written until the exit gate at the bottom is met.

## Decision update: Rust engine + SwiftUI app

You chose a Rust engine with a SwiftUI front end. This changes M0 from a three-way comparison to **validating Rust against the plan's gates**, with the others kept only as fallbacks.

- **Engine:** Rust (hyper + rustls + tokio; evaluate `hudsucker` 0.25 as a base versus a thin custom MITM layer on hyper). Toolchain here: rustc/cargo 1.91.1, `aarch64-apple-darwin` installed.
- **App:** SwiftUI. Swift never touches sockets; it talks to the engine through the control API.
- **Process model (to confirm in the spike):**
  - *Helper process* (default): a signed `netproxyd` binary embedded in the app bundle, with the control API on a Unix domain socket (authenticated, per the plan's "no unauthenticated control endpoint").
  - *In-process via UniFFI* (0.32.x): alternative if latency or lifecycle favours it. The spike measures both only if the helper fails a gate.
- **Why a helper is the default:** crash isolation, one signed binary, and the same control API serves SwiftUI, CLI and MCP.
- **Retired from M0:** the Swift-native (A) and mitmproxy (B1) spikes. They remain §8 fallbacks if Rust fails a hard gate.
- **Revised M0 work:** harness (2 d), Rust PoC (3–5 d), device checks (1–2 d), packaging/notarization of an app embedding the helper (1–2 d), report (1 d).
- **New unknowns:** whether `hudsucker` handles HTTP/2 and WebSocket correctly enough to build on; rustls CA/leaf-cert generation (`rcgen`) performance; HTTP/2 flow correlation (a fork `ideamans-hudsucker` exists for this, so check its maintenance and licence before depending on it); Swift↔Rust event-stream schema (JSON lines versus protobuf) cost under load.
- **Still open:** distribution target, physical devices, repo layout (see §10).

The sections below were written for the three-way comparison. Read them as the shared test method; the Rust PoC is the primary candidate.

## 0. Scope guard

Unchanged from the plan: explicit proxy first, local + remote fixtures, local MCP over stdio, SwiftUI app. M0 produces a **decision and a throwaway proof of concept**, not product code.

Proposed deviations (none adopted without your approval):

| Proposal | Impact |
|---|---|
| Add a **third candidate**, a Rust engine (hyper + rustls, e.g. `hudsucker`), because `cargo` is already installed here. | One more spike (~2–3 days). It is "Existing engine" under Option B, not new scope. Drop it if you want to stay strictly Swift vs mitmproxy. |
| Hybrid is evaluated **only** if both A and B each fail a hard gate. | Matches the plan's wording. |

## 1. Environment facts (verified on this machine)

- macOS 26.5.2, arm64. Swift 6.3.3, Xcode installed.
- iOS 18.6 simulators available (iPhone 16 family, iPad Pro).
- `cargo` present. `mitmproxy`, `node`, `go` and `adb` are **not** installed. Android needs `adb` or Android Studio emulator tooling; the `android` CLI skill exists for that.
- No physical-device inventory known. **Open question for you:** which physical iOS/Android devices are available for the device-connectivity gate?

## 2. Candidates

### A. Native Swift (SwiftNIO stack)
- `swift-nio` + `swift-nio-ssl` (TLS), `swift-nio-http2`, `swift-nio-extras` (CONNECT handling), `swift-certificates`/`swift-crypto` (on-the-fly leaf certs, CA).
- Everything is Apache-2.0, and the engine can be in-process or a helper.
- Strengths: single toolchain, native lifecycle, no runtime to package, tight typed event model.
- Risks: MITM forwarding logic is ours (CONNECT, ALPN negotiation, HTTP/1↔2 mismatch, upstream connection pooling, WebSocket upgrade passthrough, chunked/streaming bodies, backpressure). Highest correctness-effort cost.

### B1. mitmproxy (existing engine)
- Mature MITM, HTTP/1/2, WebSocket, TLS cert generation, addon API in Python.
- Licence: MIT (confirm current licence text at pin time).
- Integration shape: run `mitmdump` as a helper process; addon streams flow events to the control plane over a local Unix socket.
- Risks: bundling Python or its standalone binary inside a signed, notarized app; helper-process lifecycle; added latency from the Python addon path; upgrade cadence; event-schema translation.

### B2. Rust engine (optional, see §0)
- Strengths: performance, memory safety, single static binary is simpler to sign than a Python runtime.
- Risks: smaller ecosystem for MITM specifics; we would own more protocol glue than with mitmproxy; adds a third language to the repo.

### Hybrid (conditional)
Only if A fails correctness/effort gates and B fails packaging/latency gates. Likely shape: mitmproxy engine behind the same `ProxyEngine` contract, with Swift owning everything else. Not pursued unless data forces it.

## 3. Smallest proof of concept (per candidate)

Each PoC implements the **same** thin contract so results are comparable:

1. Listen on `127.0.0.1:<port>` as an explicit HTTP proxy.
2. Plain HTTP forward.
3. HTTPS via CONNECT with MITM using a locally generated CA and per-host leaf certs.
4. HTTP/2 to client and upstream, with ALPN.
5. WebSocket upgrade passthrough with message-level visibility.
6. Emit an ordered JSON-lines event stream: `flow_started`, `request_headers`, `request_body_chunk`, `response_headers`, `response_body_chunk`, `flow_completed|failed`, each carrying monotonic timestamps and a flow ID.
7. One hard-coded mock decision (respond-with-fixture for one host+path) to test the interception point.

Not in the PoC: UI, SQLite, MCP, remote fixtures, rule schema. The PoC must not assume anything beyond the event stream and the one mock hook.

## 4. Test harness (shared, built first)

A controlled **upstream server** plus a **load/correctness client**, independent of any candidate:

- Local HTTPS upstream with a test CA, HTTP/1.1 and HTTP/2 endpoints.
- Endpoints: small JSON, 100 MB streaming body, slow-drip body, mid-stream reset, TLS failure (expired / wrong host), redirect, chunked, gzip, WebSocket echo.
- Client script drives N concurrent requests and records ground truth (expected bytes/hash/order) to compare against captured events.
- Run each candidate on the same machine, same workload, N=5 runs, report median and spread.

## 5. Benchmark matrix, thresholds and decision rules

Thresholds are fixed **before** running, and come from the plan's performance targets.

| Test | Metric | Hard gate (fail = candidate rejected) | Target |
|---|---|---|---|
| HTTP/1.1 + HTTPS correctness | Byte-exact bodies, headers preserved, zero missing/duplicate flows | 100% on reference suite | same |
| Added latency, small warm requests | p50 / p95 vs direct | p50 ≤ 10 ms, p95 ≤ 25 ms (plan targets) | p50 ≤ 5 ms |
| HTTP/2 concurrency (100 streams) | Correct per-stream results, throughput | No cross-stream corruption | ≥ 80% of direct throughput |
| WebSocket | Upgrade works, ordered message capture | Works, no dropped frames | — |
| Large body (100 MB) | Peak RSS, streaming | Memory stays bounded (< ~2× chunk window, not body size) | — |
| Slow/interrupted | Cancellation, timeouts, recovery | Proxy survives; flow recorded as failed with reason | — |
| TLS failures | Diagnostic quality | Distinct, accurate error classes (untrusted, pinned/handshake abort, expired) | — |
| Sustained load (10 min) | RSS growth, event loss | No unbounded growth, zero event loss | — |
| Event ordering | Per-flow ordering, cross-flow timestamps | Strictly ordered per flow | — |
| Device setup | iOS simulator, Android emulator, ≥1 physical each if available | HTTPS decrypted after CA install, pinning failure surfaced | — |
| Packaging | Sign + notarize a minimal app with the engine embedded | Passes `spctl`/notarization, launches clean on a second Mac | — |
| Licensing | Dependency licence audit | No copyleft that blocks distribution | — |
| Effort / maintenance | Engineer-days to pass all gates; LOC we own | Recorded, not gated | — |

**Decision rule:** among candidates that pass every hard gate, pick the one with the lowest packaging complexity and owned-code burden. If two tie, prefer Swift (single toolchain). If none pass, consider the hybrid, then re-plan.

## 6. Unknowns and risks to resolve

1. Whether SwiftNIO HTTP/2 can proxy HTTP/2 streams without re-framing bugs. Needs the spike, not opinion.
2. mitmproxy's current HTTP/3/QUIC behaviour, and whether it should be explicitly disabled. QUIC is out of scope, so the app must fall back cleanly, and iOS/Android apps may try QUIC first.
3. Hardened-runtime + notarization with an embedded Python interpreter or a PyInstaller-style binary: entitlements needed? (`com.apple.security.cs.allow-unsigned-executable-memory` etc.)
4. App Sandbox vs Developer ID distribution: a sandboxed app must have network server and client entitlements; helper-process spawning is constrained. Likely Developer ID, **not** Mac App Store, for v1. Confirm with you.
5. macOS local-network privacy prompts for devices on the LAN, and firewall prompt behaviour on first listen.
6. CA lifecycle: key storage in Keychain, user-trust setup flow on macOS, iOS profile install + "Certificate Trust Settings" step, Android 7+ user-CA limits (apps ignore user CAs unless they opt in). The Android limit directly affects the plan's acceptance criterion, so it must be tested early.
7. Android cleartext/network-security-config in test apps; need a debug-build test app, which can be a trivial one.
8. Licensing: mitmproxy MIT, SwiftNIO Apache-2.0 (verify at pin time); any Rust crates' licences.

## 7. Work breakdown (dependency order, no calendar dates)

1. Write the shared contract (event stream schema + one mock hook) — 0.5 d.
2. Build the upstream server and ground-truth client harness — 2 d.
3. Spike A (Swift) — 4–6 d.
4. Spike B1 (mitmproxy) — 2 d.
5. Spike B2 (Rust, if approved) — 3 d.
6. Device connectivity checks (iOS sim, Android emulator, physical if available) — 1–2 d, run per surviving candidate.
7. Packaging trial for surviving candidates — 1–2 d.
8. Run benchmark matrix, write comparison — 1 d.

The estimates are rough. The plan says calendar estimates should follow the spike, so treat these as effort sizing only.

## 8. Fallbacks

- If Swift fails correctness at HTTP/2 only: ship HTTP/1.1 + HTTPS first via Swift and downgrade ALPN to h1 toward clients (a visible, documented limitation), re-evaluating h2 later. This is a **scope change** and needs your approval.
- If mitmproxy packaging fails: Rust engine, then hybrid.
- If all fail the latency gate: revisit the targets with data before changing the engine.

## 9. Exit criteria for M0

- [ ] Shared harness runs reproducibly with one command.
- [ ] At least two candidates PoC'd and run through the full matrix.
- [ ] Written comparison with measured numbers, not impressions.
- [ ] Selected engine + process model (in-process vs helper) + fallback recorded.
- [ ] Device connectivity verified on iOS simulator and Android emulator (physical where available), including a documented pinning-failure case.
- [ ] Signed + notarized minimal app with the chosen engine embedded.
- [ ] Licence audit complete.
- [ ] Updated risk list and any plan changes flagged for approval.

## 10. Decisions needed from you before I start building the harness

1. Include the Rust candidate (B2), yes or no?
2. Distribution target: Developer ID only for v1, or Mac App Store?
3. Physical devices available for testing (iOS / Android)?
4. Repo layout: OK to use `~/Desktop/workspace/netproxy/` with `docs/`, `harness/` and `spikes/{swift,mitmproxy,rust}/`?
