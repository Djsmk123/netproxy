# M0 findings (Rust / hudsucker 0.25 PoC)

macOS 26.5.2 arm64, loopback, client + proxy + upstream on one machine. Reproduce with `scripts/m0-smoke.sh [release]` (correctness) and `np-bench` (latency).

## Correctness (scripts/m0-smoke.sh)
| Check | Result |
|---|---|
| HTTPS MITM via CONNECT, per-host leaf certs (rcgen) | Pass |
| HTTP/1.1 and HTTP/2 through proxy | Pass |
| POST echo, redirect | Pass |
| 100 MB streamed body, proxy-computed SHA-256 vs upstream ground truth | Pass, byte-exact |
| 100 concurrent h2 streams: 106 flows, 105 responses, 105 completed, 0 duplicates | Pass |
| Mock hit (HTTPS + HTTP), recorded as `mock_hit` | Pass |
| Mid-body upstream reset | Pass: `body_error` event, client sees truncated body + h2 error |
| Events dropped under bounded queue (65k) | 0 in all runs |

## Latency / throughput (release build, bench: 5000 sequential small GETs after 300 warmup)
| | direct p50 / p95 | proxy p50 / p95 | **added p50 / p95** | gate |
|---|---|---|---|---|
| HTTP/1.1 | 0.042 / 0.053 ms | 0.110 / 0.129 ms | **0.068 / 0.076 ms** | ≤10 / ≤25 ms |
| HTTP/2 | 0.055 / 0.065 ms | 0.124 / 0.145 ms | **0.069 / 0.080 ms** | ≤10 / ≤25 ms |

Throughput at concurrency 64 (20,000 requests): h1 38k rps via proxy vs 94k direct (41%); h2 30k vs 68k (44%). The proxy was also writing about 46 MB of JSON events to disk at the time.

Caveats: loopback with a 18-byte body flatters latency; real-device and larger-payload numbers will be higher. The h2 client used prior-knowledge through a CONNECT proxy, so confirm the negotiated protocol on both legs. Single machine, so the client, proxy and upstream compete for CPU.

## Resolved since the first draft
- **Flow correlation:** hudsucker clones the handler per request (`internal.rs`: `self.clone().proxy(req)`), so a flow ID stored on the handler is exact for h1 and h2. No fork is needed.
- **CONNECT:** now a separate `connection_opened` event.
- **Bodies:** streamed through a tap that records chunk sizes, total, SHA-256 and a 4 KiB bounded preview. It finalizes on `is_end_stream` (hyper may never poll again once a content-length body completes) and emits `body_aborted` on drop.
- **Backpressure:** events go through a bounded channel with `try_send` and a drop counter, so a slow consumer can never stall traffic.

## Edge cases (scripts/m0-edge.sh) and soak (scripts/m0-soak.sh)
| Check | Result |
|---|---|
| TLS failure classes (untrusted issuer, hostname mismatch, expired, connection refused) | Pass: four distinct `flow_failed.class` values, client gets 502 |
| Slow-drip 2 s body | Pass (2.02 s, 200) |
| Client cancel mid-body | Pass: `body_aborted` recorded |
| WebSocket echo x50 through MITM | Pass (50/50, 101 upgrade, 50 c2s + 51 s2c message events) after giving hudsucker's WS connector the upstream trust config; without it the tunnel closed right after 101 |
| 10-minute soak, mixed h1/h2, small + 5 MiB bodies | **Pass:** 8.29 M requests, 0 errors, 0 events dropped, proxy RSS flat at 35-42 MB |
| iOS 18.6 simulator (URLSession, CA added with `simctl keychain add-root-cert`) | Pass: HTTPS MITM and mock work; a pinned client cancels the handshake |

## Defect found and fixed during the soak
The first soak died with h2 `GOAWAY too_many_data_frames` (ENHANCE_YOUR_CALM) on 5 MiB bodies. The proxy forwarded upstream chunks as-is, producing frames below 16 KiB (average about 15 KB); the `h2` 0.4.20 client counts sub-threshold frames that sit unread against a budget and kills the connection. Direct runs had 0 errors, so this was caused by the proxy, and it would hit any strict HTTP/2 client. Fix: the body tap now coalesces chunks into full 16 KiB frames **without ever waiting** (buffers only while upstream data is immediately ready, flushes when it would block, so SSE/streaming is not delayed). Result: 0 h2 errors, throughput up. The tap also forwards `size_hint` and trailers.

## Fixes made after the first soak (all re-verified with smoke, edge and soak scripts)
1. **Event volume:** per-chunk events replaced by `body_first_byte` plus at most one `body_progress` per 250 ms per body; `body_end` carries total, chunk count, SHA-256 and a base64 preview (4 KiB cap). Bodies of 5 MiB no longer produce hundreds of events.
2. **Client TLS rejection is now explicit and per connection.** `should_intercept_tls` emits `tls_hello` (SNI, ALPN); a `tracing` bridge maps hudsucker's per-connection span to our connection id and emits `client_tls_failed` with a class:
   - `client_rejected_unknown_ca`: client sent a `UnknownCA` alert (curl without the CA).
   - `client_rejected_certificate`: `CertificateUnknown`/`BadCertificate` alert (**Android** untrusting apps).
   - `client_closed_during_handshake`: client closed silently (**iOS** pinning-style cancel).
   - A 5 s `client_tls_no_request` watchdog remains as a safety net (it is redundant when a failure event exists; dedupe in M1).
   - Cost of the tracing layer: added p50 0.071 ms vs 0.062 ms before.
3. **Request-to-connection link:** `request_headers` now carries `conn`.

## Android (Android 16 / API 36 `google_apis` userdebug emulator, probe APK built with raw SDK tools in `scripts/android/`)
| Scenario | Result |
|---|---|
| No proxy CA installed | Fails: `Trust anchor for certification path not found`; proxy logs `client_rejected_certificate` |
| CA in **user** store, default app (targetSdk 34) | **Still fails.** Confirms the plan's risk: apps ignore user CAs by default |
| CA in user store, app with debug `network-security-config` trusting `user` | Works: 200 and mock hit |
| CA in **system** store (conscrypt apex bind-mount trick, root emulator only) | Works for the default app: 200 and mock hit |

**Implication for the plan:** the Android acceptance criterion holds only for (a) debug builds that opt in via `network-security-config`, (b) rooted/emulator system-CA installs, or (c) browsers/WebViews that honor user CAs. It cannot hold for arbitrary release apps on stock devices. The app must say so in its device-setup flow and show `client_rejected_certificate` as "this app does not trust user-installed CAs". Not tested: a physical Android device, Chrome.

## iOS simulator
HTTPS MITM and mocking work after `simctl keychain add-root-cert`. Pinned client shows `client_closed_during_handshake`. Not tested: a physical iOS device (needs the profile install plus Settings > General > About > Certificate Trust Settings step).

## Packaging (`scripts/package/build-app.sh`)
SwiftUI app with the Rust engine embedded at `Contents/Helpers/netproxyd`, signed inside-out with the **hardened runtime**. `codesign --verify --deep --strict` passes, and `NetProxy --selftest` starts the helper from the signed bundle, serves a mocked response through it and shuts it down cleanly. Bundle is 8.6 MB (helper 8.4 MB).
- **Not done:** Developer ID signing and notarization. Only an *Apple Development* identity exists on this Mac, so `spctl` rejects the app, as expected. Notarization needs a Developer ID Application certificate plus notarytool credentials; do this before M5, but it is low-risk (hardened runtime already works with no special entitlements).
- ATS gotcha: an in-app `URLSession` cannot make plain-HTTP requests to arbitrary hosts without an ATS exception, which matters if the app ever issues its own test requests.

## Licence audit (`cargo metadata`, 261 third-party crates in the spike)
All permissive (MIT / Apache-2.0 / BSD / ISC / Unicode / CDLA-Permissive-2.0). Notes: `r-efi` offers an LGPL option but is also MIT/Apache and only used for UEFI targets; `webpki-roots` is CDLA-Permissive (data); `aws-lc-sys` carries OpenSSL-style attribution terms. Action for M5: ship a third-party notices file; run `cargo deny` in CI.

## M0 verdict: Rust + SwiftUI is viable. Proceed to M1.
Hard gates: all passed (correctness, latency 0.07 ms vs 10 ms, h1/h2, WebSocket, large bodies, cancellation, TLS failures, 10-minute soak, iOS simulator, Android emulator, hardened-runtime packaging, licences).

### Carry-forward into M1
1. **Replace the PoC handler with the real flow model** (conn/flow/event schema, ordering, persistence); the PoC's JSON-lines stdout is not the control API.
2. **hudsucker dependency decision:** it works and exposes the needed hooks, but the TLS-failure signal comes from `tracing` span parsing, which is fragile across upgrades. Pin the version, wrap it behind a `ProxyEngine` trait, and keep the option to fork/replace.
3. **Control API:** authenticated Unix socket (peer-credential check, per-launch token); not started.
4. **Event pipeline:** bounded queue with drop accounting already proven; add per-flow sequence numbers so consumers can detect gaps.
5. **Strict mock mode, rule runtime, fixtures:** M3, untouched by M0 apart from the single hook.
6. **Open verification items (not blockers):** physical iOS and Android devices; Chrome on Android; HTTP/3/QUIC fallback on real devices; Developer ID signing + notarization; a gRPC (h2 trailers) smoke test (trailers are forwarded by the new tap but untested).
