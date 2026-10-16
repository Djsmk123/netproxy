# Changelog

All notable changes to NetProxy will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added (M4 Phase 1) - 2026-10-16
- **Dynamic Mock Registry** (`DynamicMockRegistry`)
  - Runtime rule management (add, update, remove dynamic rules)
  - Combines static (YAML) and dynamic (runtime) rules
  - Thread-safe with read-optimized RwLock
  - Priority-based rule merging
  - Hot reload support for YAML config
- **Validation Framework**
  - Comprehensive rule validation with severity levels (Error, Warning, Info)
  - Detects duplicate IDs, missing fixtures, invalid regex, unreachable rules
  - Conflict detection for overlapping patterns
  - Pre-validation on add/update operations
- **File Watcher for Hot Reload**
  - Automatic config reload on mocks.yaml changes
  - 500ms debounce for atomic file writes
  - Validates after each reload
  - Clean shutdown support
- **Control API - 10 New Methods**
  - `mocks.list` - List all rules
  - `mocks.list_dynamic` / `mocks.list_static` - List by source
  - `mocks.get` - Get rule by ID
  - `mocks.add` - Add dynamic rule with validation
  - `mocks.update` - Update dynamic rule
  - `mocks.remove` - Remove dynamic rule
  - `mocks.clear_dynamic` - Clear all runtime rules
  - `mocks.reload` - Manual config reload
  - `mocks.validate` - Validate all rules
- **Daemon Integration**
  - Auto-starts file watcher when engine starts
  - Validates rules on startup (fails if errors)
  - Graceful watcher shutdown
- **Serialization Support**
  - All schema types now implement `Serialize`
  - Validation results serializable for API responses

### Changed
- `DaemonConfig` now uses `DynamicMockRegistry` instead of static resolver
- Mock config validation errors now fail daemon startup
- Added `tracing` dependency to `np-mock`
- Added `notify` crate for file watching

### Added (M4 Phase 2) - 2026-10-17
- **Flow-to-Mock Conversion** ✅ Complete
  - Convert captured HTTP request/response flows to mock rules
  - Extract response bodies as fixture files (text/binary support)
  - Smart match pattern generation (host, path, method, query, headers)
  - Configurable inline threshold for small bodies
  - **Control API - 2 New Methods:**
    - `mocks.from_flow` - Convert flow to rule (preview mode)
    - `mocks.from_flow_and_add` - Convert, validate, and add atomically
  - **Flow Converter Module** (`netproxyd/src/flow_converter.rs`)
    - Extracts request/response from event stream
    - Content-type based fixture naming (.json, .html, .png, etc.)
    - Header filtering (skips hop-by-hop headers)
    - UTF-8 detection for text content
    - Comprehensive warning messages
  - **Features:**
    - Query parameter matching (optional)
    - Header-based matching (configurable)
    - Fixture file auto-generation
    - Base64 preview body decoding
    - Validation before adding to registry
  - See `docs/m4-phase2-flow-conversion.md` for complete guide

### Added (M4 Phase 4) - 2026-10-17
- **MCP Integration** ✅ Complete
  - Model Context Protocol server for AI-assisted mock management
  - Enables Claude and other AI assistants to programmatically manage mocks
  - **New Crate:** `np-mcp` - MCP server binary
  - **Control Client Library** (`netproxyd/src/control_client.rs`)
    - Reusable client for connecting to netproxyd control API
    - JSON-RPC 2.0 over Unix domain socket
    - Automatic authentication handshake
  - **MCP Tools (9 total):**
    - `list_flows` - List captured HTTP requests
    - `get_flow` - Get flow details
    - `list_mocks` - List active mocks (all/static/dynamic)
    - `create_mock` - Create new mock rule
    - `update_mock` - Update existing mock
    - `delete_mock` - Remove mock
    - `flow_to_mock` - Convert flow to mock (with add_to_registry option)
    - `validate_mocks` - Check for issues
    - `create_fixture` - Write fixture file
  - **Features:**
    - Stdio-based JSON-RPC protocol (MCP 2024-11-05)
    - Proper error handling with MCP error codes
    - Path traversal protection for fixtures
    - Claude Desktop integration ready
    - Comprehensive README with configuration examples
  - See `crates/np-mcp/README.md` for setup guide
  - See `docs/m4-phase4-mcp-integration.md` for design

### Planned (M4 Phase 3+)
- Smart pattern extraction (detect IDs, UUIDs, slugs)
- Batch flow conversion
- Remote fixture fetching with caching
- Simulation mode (dry-run without activating)

## [0.3.0] - M3: Mock Runtime - 2024-10-16

### Added
- **Mock Rule System** - YAML-based declarative mock rules
  - Pattern matching for host, path, method, headers, and query parameters
  - Support for wildcards (`*.example.com`, `/api/*`)
  - Regex path matching (`/api/users/\d+`)
  - Priority-based rule ordering
  - Enable/disable per rule
- **Mock Actions**
  - `respond`: Return mock responses with custom status, headers, and body
  - `pass`: Let requests through to real servers
  - `fail`: Simulate connection failures
- **Fixture System**
  - File-based fixtures with caching
  - Inline body support
  - SHA-256 content hashing
  - Relative path resolution
- **Mock Events**
  - `MockHit` events for tracking rule matches
  - `mocked: true` flag on `ResponseHead` events
- **New Crate: `np-mock`**
  - Schema definitions for YAML config
  - Pattern matching engine
  - Fixture resolver with caching
  - `RuleMockResolver` implementation

### Changed
- Daemon now auto-loads `mocks.yaml` from data directory on startup
- Mock resolver wraps existing mock stack (composable design)

### Documentation
- Added comprehensive mock system guide
- Added examples directory with sample configurations
- Updated architecture documentation

## [0.2.0] - M2: Store & Control API - 2024-10-15

### Added
- **Persistent Storage** - SQLite-based event and flow storage
  - Events table with JSON payloads
  - Flows table with metadata
  - Runs table for capture sessions
  - Retention and pruning policies
- **Control API** - JSON-RPC over Unix domain sockets
  - Engine lifecycle control (`start`, `stop`, `status`)
  - Event streaming (`subscribe`, `since`)
  - Flow queries (`list`, `get`)
  - Token-based authentication
- **Body Storage**
  - File-based body storage separate from SQLite
  - Per-body and total caps
  - LRU eviction
  - Orphan cleanup
- **New Crates**
  - `np-store`: Storage layer
  - `netproxyd`: Daemon orchestrator

### Changed
- Refactored engine to emit events through `EventSink`
- CA material now managed by daemon
- Test infrastructure improved with harness crate

### Documentation
- Added control API reference
- Added architecture overview

## [0.1.0] - M1: Production Engine - 2024-10-15

### Added
- **Production-Ready Proxy Engine**
  - HTTP/1.1 and HTTP/2 support
  - HTTPS with MITM TLS interception
  - Automatic CA generation and management
  - Request/response capture
- **Event Model**
  - Structured events for all traffic
  - Flow state machine with validation
  - Connection lifecycle tracking
  - Body streaming with chunking
- **CA Management**
  - macOS Keychain integration for private key storage
  - Per-host leaf certificate generation
  - Trust chain validation
- **Conformance Tests**
  - End-to-end integration tests
  - Flow state validation
  - Error handling verification
  - TLS functionality tests

### Changed
- Migrated from spike to production engine implementation
- Formalized event schema (`np-model` crate)
- Separated engine from daemon concerns

### Documentation
- Added M1 implementation report
- Documented conformance test suite

## [0.0.1] - M0: Spike - 2024-10-15

### Added
- **Initial Spike Implementation**
  - Basic proxy using hudsucker
  - HTTPS MITM proof of concept
  - Simple event capture
  - Exploratory tests

### Documentation
- M0 spike plan
- M0 findings report
- Initial architectural decisions

## Migration Notes

### Upgrading from M2 to M3

No database changes required. To use mocks:

1. Create `mocks.yaml` in your data directory
2. Create `fixtures/` directory for fixture files
3. Restart netproxyd

Configuration is backward compatible - daemon works normally without `mocks.yaml`.

### Upgrading from M1 to M2

Database changes required:
- Events now stored in SQLite (previously in-memory)
- New control API replaces direct engine access
- Bodies now stored as files (not in database)

Migration is automatic on first run.

### Upgrading from M0 to M1

Complete rewrite - no migration path. M0 was a spike.

## Version History

- **M0 (2024-10-15)**: Initial spike and proof of concept
- **M1 (2024-10-15)**: Production engine with conformance tests
- **M2 (2024-10-15)**: Store and control API
- **M3 (2024-10-16)**: Deterministic mock rule runtime
- **M4 (Planned)**: MCP integration and advanced features

## Links

- [Repository](https://github.com/yourusername/netproxy)
- [Documentation](docs/)
- [Issues](https://github.com/yourusername/netproxy/issues)

---

**Note**: This project is under active development. APIs may change between releases.
