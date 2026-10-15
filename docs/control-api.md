# netproxyd control API — v1

One API serves SwiftUI, the CLI and MCP. `API_VERSION = 1`, event `SCHEMA_VERSION = 1` (`crates/np-model`).

## Transport and security
- JSON-RPC 2.0, **one JSON object per line**, over a Unix domain socket: `<data-dir>/run/control.sock`
  (default data dir `~/Library/Application Support/NetProxy`). Unix socket paths are limited to ~104 bytes on macOS, so when
  that path is too long the daemon binds `/tmp/np-<uid>/<hash>.sock` (dir `0700`) instead. **Always read the real path from
  `<data-dir>/run/control.endpoint`** (one line) rather than assuming `control.sock`.
- `run/` is mode `0700`, the socket `0600`, and the daemon verifies it owns the directory. A connecting
  process must have the **same effective uid** as the daemon, otherwise the connection is dropped.
- A fresh random token is written to `run/control.token` (`0600`) at every daemon start.
  The **first message must be `hello` with that token**. Anything else (missing/wrong token, malformed JSON,
  oversize line > 1 MiB, no message within 5 s) closes the connection **without a reply**.
- There is no TCP listener. Removing the socket on exit is best-effort; a stale socket is replaced at start.
- Mutating methods (`engine.start`, `engine.stop`, `storage.prune`) log the caller's `client.name` via `tracing`.
  **Not yet implemented:** per-client scopes and a persisted audit trail (planned for M4, when MCP arrives).

## Handshake
```json
{"jsonrpc":"2.0","id":1,"method":"hello","params":{"token":"…","api_version":1,"client":{"name":"swiftui","version":"0.1"}}}
→ {"jsonrpc":"2.0","id":1,"result":{"api_version":1,"schema_version":1,"daemon_version":"0.1.0","status":{…}}}
```
A wrong `api_version` returns error `-32002` and closes.

## Methods
| Method | Params | Result |
|---|---|---|
| `ping` | – | `{pong:true}` |
| `engine.status` | – | `{state:"running"\|"stopped", run, listen, sequenced, dropped}` |
| `engine.start` | `{listen?: "ip:port"}` | status; error if already running |
| `engine.stop` | – | final status of the run (`sequenced`, `dropped`); error if not running |
| `ca.get` | – | `{cert_pem}` (public certificate only; the key never leaves the daemon) |
| `runs.list` | – | `[{run, started_us, ended_us, engine, listen, status: running\|stopped\|interrupted, events, flows}]` (O(1) per run) |
| `flows.list` | `{filter?, limit?≤500, cursor?}` | `{flows:[FlowRow], next: Cursor\|null}` newest first |
| `flows.get` | `{run?, flow}` | `{flow: FlowRow\|null, events:[Envelope]}` |
| `events.since` | `{run?, seq, limit?≤1000}` | `{events:[Envelope]}` with `seq > seq` |
| `events.subscribe` | `{run?, since?}` | `{subscription, run, after_seq}`; then `event` notifications |
| `events.unsubscribe` | `{subscription}` | `{unsubscribed:bool}` |
| `bodies.get` | `{run?, flow, dir:"req"\|"res", offset?, length?≤1MiB}` | `{data_b64, offset, size, eof, evicted, inline?}` |
| `storage.status` | – | `{used_bytes, max_bytes}` |
| `storage.prune` | `{max_bytes}` | `PruneReport` |

`run` may be omitted or `"current"` (= running run, else latest). In `flows.list`, `filter.run:"current"` works too.
`filter` fields: `run, host, method, status, status_class(2..5), state(open|completed|failed|aborted), mocked, text (URI substring, case-insensitive), since_us, until_us`.

## Event stream
Notifications: `{"jsonrpc":"2.0","method":"event","params":{"subscription":N,"event":Envelope}}`.
`Envelope = {run, seq, ts_us, kind, …event fields}` (flattened; see `crates/np-model/src/lib.rs`).

Guarantees (verified by `crates/np-engine/tests/conformance.rs` and `crates/netproxyd/tests/control.rs`):
1. `seq` starts at 1 per run and is **contiguous**; a gap in capture is always explained by an `events_dropped` envelope.
2. Per flow the events obey the state machine in `np_model::FlowState`; every flow ends `completed`, `failed` or `aborted`.
3. Subscribers read the **durable log**, woken by store commits. A slow client is paced by its own socket (the daemon never
   buffers unboundedly for it), cannot lose, duplicate or reorder events, and cannot slow capture.
4. To resume after a disconnect: `events.subscribe {since: last_seen_seq}`. Without `since` you get the tail (new events only).
5. Retention can trim the head of a run: `events.since` for a trimmed `seq` returns from the oldest retained event.

## Errors
`-32700` parse, `-32600` invalid request, `-32601` unknown method, `-32602` invalid params, `-32000` application error
(message explains), `-32002` unsupported api_version. Auth failures have no response by design.

## Try it
```
netproxyd --data-dir /tmp/np --listen 127.0.0.1:8899 --autostart
python3 scripts/ctl.py /tmp/np engine.status
python3 scripts/ctl.py /tmp/np flows.list '{"filter":{"run":"current","status_class":5},"limit":20}'
```
Flags: `--data-dir`, `--listen`, `--upstream-ca <pem>`, `--max-db-mb N` (0 = unbounded; default 2048), `--autostart`.
