# NetProxy

> A modern, native macOS HTTP/HTTPS debugging proxy with built-in mock capabilities

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.70+-orange.svg)](https://www.rust-lang.org/)
[![macOS](https://img.shields.io/badge/platform-macOS-lightgrey.svg)](https://www.apple.com/macos)

NetProxy is a high-performance network proxy designed for developers who need to inspect, debug, and mock HTTP/HTTPS traffic. Built with Rust for speed and reliability, with a native SwiftUI interface for a seamless macOS experience.

## ✨ Features

### Core Capabilities
- 🔍 **Traffic Inspection** - Real-time HTTP/HTTPS request/response capture
- 🔐 **TLS Interception** - Automatic MITM certificate generation and management
- 💾 **Persistent Storage** - SQLite-based event storage with efficient querying
- 🚀 **High Performance** - Rust-powered proxy engine with minimal overhead
- 🎯 **Mock Runtime** - YAML-based rule system for intercepting and mocking requests

### Mock System (M3)
- 📝 **Declarative Rules** - Define mocks in simple YAML configuration
- 🎭 **Pattern Matching** - Support for wildcards, regex, headers, query params
- 📦 **Fixture Support** - File-based or inline response bodies
- ⚡ **Priority System** - Control rule evaluation order
- 🔄 **Actions** - Respond, Pass, or Fail actions for flexible behavior
- 📊 **Event Tracking** - Full observability with MockHit events

## 🚀 Quick Start

### Installation

```bash
# Clone the repository
git clone https://github.com/yourusername/netproxy.git
cd netproxy

# Build the project
cargo build --release

# Run the daemon
./target/release/netproxyd
```

### Basic Usage

1. **Start the daemon**
   ```bash
   netproxyd --data-dir ~/Library/Application\ Support/NetProxy
   ```

2. **Configure your HTTP client** to use the proxy:
   - Proxy: `http://127.0.0.1:8899`
   - Install CA certificate from: `~/Library/Application Support/NetProxy/ca/`

3. **Start capturing** traffic through the control API or UI

## 📚 Documentation

- [Mock System Guide](docs/mock-system.md) - Complete guide to the mock runtime
- [Control API Reference](docs/control-api.md) - JSON-RPC API documentation
- [Architecture Overview](docs/architecture.md) - System design and components
- [Development Guide](docs/development.md) - Contributing and development setup

## 🎯 Mock System Example

Create a `mocks.yaml` in your data directory:

```yaml
version: 1
fixtures_dir: ./fixtures

rules:
  # Mock a JSON API endpoint
  - id: api-users
    priority: 100
    enabled: true
    match:
      method: GET
      host: "api.example.com"
      path: "/api/users"
    action:
      type: respond
      status: 200
      headers:
        content-type: application/json
      body:
        file: users.json

  # Mock with inline response
  - id: hello-world
    priority: 100
    enabled: true
    match:
      host: "*.test.local"
      path: "/hello"
    action:
      type: respond
      status: 200
      body:
        inline: "Hello from NetProxy!"

  # Wildcard matching
  - id: catch-all
    priority: 50
    enabled: true
    match:
      host: "staging.example.com"
      path: "/api/*"
    action:
      type: respond
      status: 200
      body:
        inline: '{"status":"mocked"}'
```

See the [Mock System Guide](docs/mock-system.md) for complete documentation.

## 🏗️ Architecture

NetProxy is organized into several Rust crates:

- **`np-engine`** - Core proxy engine using hudsucker
- **`np-model`** - Shared data models and event types
- **`np-store`** - SQLite-based event persistence
- **`np-mock`** - Mock rule runtime (M3)
- **`netproxyd`** - Daemon orchestrating all components
- **`harness`** - Test infrastructure

The SwiftUI app lives in `app/` and communicates with the daemon via a JSON-RPC control API over Unix domain sockets.

## 🧪 Testing

```bash
# Run all tests
cargo test --workspace

# Run mock system tests
cargo test --package np-mock

# Run integration tests
cargo test --package netproxyd --test mocks
```

## 🛣️ Roadmap

### Completed Milestones
- ✅ **M0**: Spike - Proof of concept with hudsucker
- ✅ **M1**: Production engine with conformance tests
- ✅ **M2**: Store and control API
- ✅ **M3**: Deterministic mock rule runtime

### Planned (M4)
- [ ] MCP integration for mock authoring
- [ ] Recorded-response conversion
- [ ] Real-time rule reloading
- [ ] Rule conflict detection
- [ ] Simulation mode (dry-run)

See [CHANGELOG.md](CHANGELOG.md) for detailed release notes.

## 🤝 Contributing

Contributions are welcome! Please see our [Development Guide](docs/development.md) for:
- Setting up your development environment
- Code style and conventions
- Testing guidelines
- Submitting pull requests

## 📄 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## 🙏 Acknowledgments

- Built with [hudsucker](https://github.com/omjadas/hudsucker) for the proxy engine
- Uses [rustls](https://github.com/rustls/rustls) for TLS
- CA management via [rcgen](https://github.com/est31/rcgen)

## 📧 Contact

- Author: MD Mobin
- Email: djsmk123@gmail.com
- GitHub: [@Djsmk123](https://github.com/Djsmk123)

---

**Built with ❤️ using Rust and SwiftUI**
