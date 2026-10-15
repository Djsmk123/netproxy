# NetProxy Architecture

This document describes the architecture and design of NetProxy, a native macOS HTTP/HTTPS debugging proxy.

## Table of Contents

- [Overview](#overview)
- [System Architecture](#system-architecture)
- [Crate Organization](#crate-organization)
- [Component Details](#component-details)
- [Data Flow](#data-flow)
- [Technology Stack](#technology-stack)
- [Design Decisions](#design-decisions)

## Overview

NetProxy is designed as a **daemon + UI** architecture, where:

- **Daemon (`netproxyd`)**: Rust-based proxy engine running as a background process
- **UI (`NetProxy.app`)**: Native SwiftUI macOS application
- **Communication**: JSON-RPC over Unix domain sockets

This separation provides:
- **Reliability**: Daemon continues running even if UI crashes
- **Performance**: Proxy runs with minimal overhead
- **Security**: Unix socket permissions for process isolation
- **Flexibility**: Multiple clients can connect to the same daemon

## System Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                        NetProxy.app                          │
│                      (SwiftUI macOS UI)                      │
└────────────────┬────────────────────────────────────────────┘
                 │
                 │ JSON-RPC over
                 │ Unix Domain Socket
                 │
┌────────────────▼────────────────────────────────────────────┐
│                        netproxyd                             │
│                     (Rust Daemon)                            │
│                                                              │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐      │
│  │   Control    │  │    Daemon    │  │     Bus      │      │
│  │   Server     │  │  Orchestrator│  │  (Events)    │      │
│  └──────┬───────┘  └──────┬───────┘  └──────▲───────┘      │
│         │                  │                 │               │
│         │                  │                 │               │
│  ┌──────▼──────────────────▼─────────────────┴───────┐      │
│  │              Proxy Engine                          │      │
│  │                                                     │      │
│  │  ┌─────────────┐  ┌──────────┐  ┌─────────────┐  │      │
│  │  │ MockResolver│  │ CA & TLS │  │ Body Sink   │  │      │
│  │  │  (Rules)    │  │   MITM   │  │ (Storage)   │  │      │
│  │  └─────────────┘  └──────────┘  └─────────────┘  │      │
│  └─────────────────────────────────────────────────┘      │
│                                                              │
│  ┌──────────────────────────────────────────────────────┐  │
│  │                 Store (SQLite)                        │  │
│  │  - Events                                             │  │
│  │  - Flows                                              │  │
│  │  - Runs                                               │  │
│  └──────────────────────────────────────────────────────┘  │
│                                                              │
│  ┌──────────────────────────────────────────────────────┐  │
│  │            Bodies Directory (Files)                   │  │
│  │  - Request bodies                                     │  │
│  │  - Response bodies                                    │  │
│  └──────────────────────────────────────────────────────┘  │
└──────────────────────────────────────────────────────────────┘
                 │                        │
                 │                        │
                 ▼                        ▼
        ┌─────────────────┐      ┌─────────────────┐
        │  HTTP Client    │      │  HTTP Server    │
        │  (curl, etc.)   │      │  (upstream)     │
        └─────────────────┘      └─────────────────┘
```

## Crate Organization

NetProxy is organized as a Rust workspace with the following crates:

### Core Crates

#### `np-engine`
**Purpose**: Core proxy engine implementation

**Responsibilities**:
- HTTP/HTTPS traffic interception
- TLS MITM with automatic certificate generation
- Request/response forwarding
- Event emission
- Mock resolution

**Key Dependencies**:
- `hudsucker`: Async HTTP proxy framework
- `rustls`: TLS implementation
- `hyper`: HTTP implementation
- `tokio`: Async runtime

**Public API**:
```rust
pub trait ProxyEngine {
    fn name(&self) -> &'static str;
    fn start(&self, cfg: EngineConfig, sink: EventSink) -> Result<Box<dyn EngineHandle>>;
}

pub trait MockResolver {
    fn resolve(&self, req: &RequestView<'_>) -> MockDecision;
}
```

#### `np-model`
**Purpose**: Shared data types and event model

**Responsibilities**:
- Event definitions (`Event` enum)
- Flow state machine
- Type definitions used across crates
- Serialization contracts

**Key Types**:
```rust
pub enum Event {
    EngineStarted { ... },
    ConnectionOpened { ... },
    RequestHead { ... },
    ResponseHead { ... },
    MockHit { ... },
    FlowCompleted { ... },
    // ...
}

pub type Headers = Vec<(String, String)>;
```

#### `np-store`
**Purpose**: Persistent storage for events and flows

**Responsibilities**:
- SQLite database management
- Event persistence
- Flow queries
- Run management
- Retention and pruning

**Schema**:
```sql
CREATE TABLE runs (
    run TEXT PRIMARY KEY,
    started_at INTEGER,
    stopped_at INTEGER
);

CREATE TABLE events (
    seq INTEGER PRIMARY KEY,
    run TEXT,
    ts_us INTEGER,
    kind TEXT,
    payload TEXT  -- JSON
);

CREATE TABLE flows (
    run TEXT,
    flow INTEGER,
    conn INTEGER,
    state TEXT,
    -- ... flow metadata
);
```

#### `np-mock` (M3)
**Purpose**: Mock rule runtime

**Responsibilities**:
- YAML configuration loading
- Pattern matching (host, path, headers, etc.)
- Fixture resolution and caching
- Mock decision making
- Priority-based rule ordering

**Architecture**:
```rust
pub struct RuleMockResolver {
    rules: Vec<Rule>,          // Sorted by priority
    fixtures: FixtureResolver,  // Cached fixture loader
    inner: Arc<dyn MockResolver>, // Fallback resolver
}
```

#### `netproxyd`
**Purpose**: Daemon orchestrator

**Responsibilities**:
- Lifecycle management
- Control API server (JSON-RPC over Unix socket)
- Event bus coordination
- Store writer management
- Configuration loading
- CA management

**Main Components**:
```rust
pub struct Daemon {
    cfg: DaemonConfig,
    engine: Arc<dyn ProxyEngine>,
    store: Store,
    ca: CaMaterial,
    // ...
}
```

### Supporting Crates

#### `harness`
**Purpose**: Test infrastructure

**Responsibilities**:
- Test upstream server
- Client factories
- Test utilities

## Component Details

### Control Server

**Location**: `netproxyd/src/control.rs`

**Protocol**: JSON-RPC 2.0 over Unix domain sockets

**Key Methods**:
- `hello`: Authenticate and establish session
- `engine.start`: Start proxy engine
- `engine.stop`: Stop proxy engine
- `engine.status`: Get current status
- `events.subscribe`: Subscribe to event stream
- `events.since`: Query historical events
- `flows.list`: List flows
- `flows.get`: Get flow details

**Security**:
- Token-based authentication
- Unix socket permissions (0600)
- Per-session token validation

### Event Bus

**Location**: `np-engine/src/bus.rs`

**Purpose**: Event distribution and sequencing

**Features**:
- Guaranteed sequencing (monotonic `seq` numbers)
- Bounded channel (8192 events)
- Drop detection and marking
- Multiple subscribers
- Backpressure handling

**Event Flow**:
```
Engine → Bus → [Store Writer, Control API Clients, ...]
```

### Mock Resolver

**Location**: `np-mock/src/resolver.rs`

**Algorithm**:
```rust
fn resolve(&self, req: &RequestView<'_>) -> MockDecision {
    // Rules are pre-sorted by priority (high to low)
    for rule in &self.rules {
        if rule.matches(req) {
            return self.apply_action(rule, req);
        }
    }
    // No match - delegate to inner resolver
    self.inner.resolve(req)
}
```

**Pattern Matching**:
- Method: Case-insensitive exact match
- Host: Exact or wildcard (`*.example.com`)
- Path: Exact, wildcard, or regex
- Headers: Case-insensitive with wildcard values
- Query params: Substring matching with wildcards

**Fixture Caching**:
- Files cached on first load
- SHA-256 hashing for integrity
- Cache persists for daemon lifetime

### Body Storage

**Location**: `netproxyd/src/bodies.rs`

**Strategy**:
- Bodies stored as individual files
- Filename: `<run>_<flow>_<dir>.body`
- Directory: `<data_dir>/bodies/`
- Cap enforcement: LRU eviction

**Retention**:
- Per-body cap (default: 4 MB)
- Total cap (default: 1 GB)
- Orphan cleanup (bodies of deleted runs)

### CA Management

**Location**: `np-engine/src/ca.rs`

**Certificate Generation**:
- Root CA: Generated once, stored in keychain (macOS) or file
- Leaf certificates: Generated per-host, cached in memory
- Validity: 365 days

**Storage**:
- **macOS**: Private key in Keychain, certificate in file
- **Other**: Both in files (for testing/CI)

## Data Flow

### Request Flow (No Mock)

```
1. Client → Proxy (CONNECT or direct HTTP)
2. Proxy → TLS handshake (generate leaf cert)
3. Proxy → MockResolver.resolve()
   → MockDecision::Pass
4. Proxy → Upstream server
5. Server → Proxy → Client
6. Proxy → EventBus → [RequestHead, ResponseHead, BodyEnd, FlowCompleted]
```

### Request Flow (Mocked)

```
1. Client → Proxy
2. Proxy → TLS handshake
3. Proxy → MockResolver.resolve()
   → MockDecision::Respond(MockResponse)
4. Proxy → Client (direct response, no upstream)
5. Proxy → EventBus → [RequestHead, MockHit, ResponseHead, FlowCompleted]
```

### Event Flow

```
Engine → EventSink → Bus → [mpsc channels]
                              │
                              ├─→ Store Writer → SQLite
                              ├─→ Control Client 1 (UI)
                              └─→ Control Client 2 (CLI)
```

## Technology Stack

### Rust Dependencies

**Async Runtime**:
- `tokio`: Main async runtime
- `futures`: Future combinators

**HTTP/Proxy**:
- `hudsucker`: Async HTTP proxy framework
- `hyper`: HTTP client/server
- `http`: HTTP types

**TLS**:
- `rustls`: Pure Rust TLS implementation
- `rustls-pemfile`: PEM parsing
- `rcgen`: Certificate generation

**Storage**:
- `rusqlite`: SQLite bindings
- `serde` + `serde_json`: Serialization

**Mock System**:
- `serde_yaml`: YAML parsing
- `regex`: Pattern matching
- `bytes`: Zero-copy buffers
- `sha2`: SHA-256 hashing

**Error Handling**:
- `thiserror`: Error derive macros

### SwiftUI (App)

**Framework**: SwiftUI + AppKit
**Communication**: JSON-RPC client
**State Management**: Combine + @Published properties

## Design Decisions

### Why Rust?

1. **Performance**: Zero-cost abstractions, minimal overhead
2. **Safety**: Memory safety without GC, prevents common bugs
3. **Concurrency**: Fearless concurrency with async/await
4. **Ecosystem**: Excellent HTTP/TLS libraries

### Why Unix Domain Sockets?

1. **Security**: File permissions for access control
2. **Performance**: Faster than TCP for local IPC
3. **Reliability**: No network configuration issues
4. **Simplicity**: No port conflicts

### Why SQLite?

1. **Simplicity**: No separate database process
2. **Performance**: Fast for local queries
3. **Portability**: Single file, easy backup
4. **Reliability**: ACID guarantees

### Why Separate Body Storage?

1. **Database size**: Keep SQLite file manageable
2. **Performance**: Don't load large blobs for event queries
3. **Flexibility**: Easy to implement different retention policies
4. **Streaming**: Can stream large bodies without loading into memory

### Why YAML for Mocks?

1. **Readability**: Easy for humans to read and write
2. **Comments**: Support for inline documentation
3. **Flexibility**: Expressive for complex configurations
4. **Familiarity**: Common in dev tools (k8s, CI/CD)

### Why Priority-Based Matching?

1. **Determinism**: Clear, predictable rule evaluation
2. **Flexibility**: Easy to override/layer rules
3. **Performance**: Exit early on first match
4. **Debuggability**: Easy to reason about which rule matched

## Performance Characteristics

### Proxy Overhead

- **Without body capture**: ~1-5% overhead
- **With body capture**: ~5-10% overhead
- **Mock responses**: Near-zero latency (no network)

### Scalability

- **Concurrent connections**: Thousands (tokio async)
- **Event throughput**: 10,000+ events/sec
- **Database size**: Tested up to 100 GB+
- **Body storage**: Limited by disk space

### Memory Usage

- **Base daemon**: ~10-20 MB
- **Per-connection**: ~50-100 KB
- **Fixture cache**: Loaded fixtures remain in memory
- **Event buffer**: 8192 events × ~1 KB = ~8 MB max

## Security Considerations

### CA Trust

- Root CA private key secured in macOS Keychain
- User must explicitly trust the CA certificate
- Leaf certificates generated per-host

### Control API

- Unix socket with 0600 permissions
- Token-based authentication
- Token rotates on daemon restart

### Mock System

- Rules apply to all traffic (no per-client filtering)
- Fixtures visible in data directory
- No remote fixture fetching (M3)

## Future Enhancements (M4+)

- **Real-time rule reloading**: Inotify/FSEvents watching
- **Remote fixtures**: HTTP fetch with caching
- **MCP integration**: Programmatic mock creation
- **Rule conflict detection**: Warn about overlapping rules
- **Simulation mode**: Dry-run without activation
- **WebSocket support**: Currently HTTP/HTTPS only
- **HTTP/2 server push**: Full HTTP/2 feature parity

---

**Next Steps**:
- [Mock System Guide](mock-system.md) - Using the mock runtime
- [Development Guide](development.md) - Contributing
- [Control API Reference](control-api.md) - API docs
