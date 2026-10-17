# NetProxy

> Modern macOS HTTP/HTTPS proxy with AI-powered control and comprehensive mock system

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.70+-orange.svg)](https://www.rust-lang.org/)
[![macOS](https://img.shields.io/badge/platform-macOS-lightgrey.svg)](https://www.apple.com/macos)
[![MCP](https://img.shields.io/badge/MCP-enabled-brightgreen.svg)](docs/MCP-PROXY-SETUP.md)

NetProxy is a high-performance network proxy for developers who need to inspect, debug, and mock HTTP/HTTPS traffic. Built with Rust for speed and reliability, with a native SwiftUI interface and full AI integration via MCP.

---

## ✨ Features

### 🔍 Traffic Inspection
- Real-time HTTP/HTTPS request/response capture
- Advanced JSON viewer with tree, raw, table, and preview modes
- Powerful filtering and search capabilities
- Session management with persistent storage

### 🎭 Mock System
- Create mocks from UI (Map Local/Remote)
- Convert captured requests to mocks
- Pattern-based URL matching
- File-based or inline response bodies
- Priority-based rule evaluation

### 🤖 AI Integration (MCP)
- **34 MCP tools** for complete automation
- Control via Claude Desktop or Claude Code
- Automated proxy setup and configuration
- Natural language traffic analysis
- AI-assisted mock creation

### 📱 Mobile Support
- iOS device proxy configuration
- Certificate installation and trust
- Android device support via ADB
- iOS Simulator integration

### 🎨 Native macOS UI
- SwiftUI interface
- JSON viewer with syntax highlighting
- Request/response inspection
- Mock management
- Session export (HAR, JSON, cURL)

---

## 🚀 Quick Start

### Option 1: Use with AI (Recommended!)

**Setup Claude Desktop:**
```bash
./scripts/setup-claude-mcp.sh
# Restart Claude Desktop
```

**Setup Claude Code:**
```bash
./scripts/setup-claude-code-mcp.sh
# Then run: claude
```

**Then just ask Claude:**
```
> "Set up NetProxy for me"
> "Show me all requests to api.example.com"
> "Create a mock that returns an error for /api/users"
```

See: [Claude Desktop MCP Setup](docs/CLAUDE-MCP-SETUP.md) | [Claude Code MCP Setup](docs/CLAUDE-CODE-MCP-SETUP.md)

---

### Option 2: Manual Setup

**1. Build NetProxy:**
```bash
git clone https://github.com/Djsmk123/netproxy.git
cd netproxy
cargo build --release
```

**2. Run the app:**
```bash
cd app
swift run
```

**3. Start capturing:**
- Click "Device Setup" → Enable "Allow device connections"
- Click "Start Recording" (⌘R)
- Configure your device to use Mac IP + port 8899

**4. View traffic:**
- Requests appear in real-time
- Click any request to inspect
- JSON responses show in beautiful tree view

---

## 📚 Documentation

### 🎯 Quick Links

**I want to...**
- **Test my iOS app** → [iOS Setup Guide](docs/IOS-SETUP-GUIDE.md)
- **Use Claude to control NetProxy** → [MCP Setup](docs/CLAUDE-MCP-SETUP.md)
- **Automate proxy setup** → [MCP Tools Reference](docs/MCP-PROXY-SETUP.md)
- **Contribute to NetProxy** → [Development Guide](docs/development.md)
- **Understand the architecture** → [Architecture Overview](docs/architecture.md)

### 📖 Complete Documentation

**[📘 View Full Documentation Wiki →](https://djsmk123.github.io/netproxy/)**

Or browse by category:
- **Setup Guides:** iOS, MCP, Claude integration
- **Technical Docs:** Architecture, API, Development
- **Planning:** UI Roadmap, Feature specs

**Documentation Index:** [docs/README.md](docs/README.md)

---

## 🎨 Screenshots

### Main Interface
- Traffic list with real-time updates
- JSON viewer with 4 modes (Tree, Raw, Table, Preview)
- Request/response inspector

### Map Local/Remote
- Create mocks from captured requests
- Serve local files for any URL
- Redirect requests to different servers

### AI Control via Claude
- Natural language proxy control
- Automated device setup
- Intelligent traffic analysis

---

## 🤖 MCP Integration

NetProxy provides **34 MCP tools** for complete automation:

### Key Tools
- `system_proxy_set` - Configure macOS system proxy
- `engine_start` / `engine_stop` - Control proxy engine
- `flows_list` / `flows_get` - Analyze captured traffic
- `mocks_add` / `mocks_from_flow` - Create mock rules
- `adb_set_proxy` - Configure Android devices
- `ios_sim_set_proxy` - Configure iOS Simulator

### Example Usage (via Claude)

```
You: "Set up my Mac to use NetProxy"
Claude: [Uses system_proxy_set tool]
        Done! Your Mac is now proxying through NetProxy on port 8899.

You: "Show me all API calls to stripe.com"
Claude: [Uses flows_list with filter]
        Found 23 requests to stripe.com:
        - 15 GET /v1/customers
        - 5 POST /v1/charges
        - 3 GET /v1/balance

You: "Create a mock that makes /api/users return an error"
Claude: [Uses mocks_add tool]
        Created mock rule: requests to /api/users will now return 500 error.
```

**See:** [MCP Tools Documentation](docs/MCP-PROXY-SETUP.md)

---

## 🏗️ Architecture

```
┌─────────────────────────────────────────────────┐
│  SwiftUI App (macOS)                           │
│  - Native UI                                    │
│  - JSON Viewer                                  │
│  - Mock Management                              │
└───────────────┬─────────────────────────────────┘
                │ JSON-RPC over Unix Socket
┌───────────────▼─────────────────────────────────┐
│  netproxyd (Rust Daemon)                       │
│  ┌──────────────────────────────────────────┐  │
│  │  Control API                             │  │
│  │  - Engine control                        │  │
│  │  - Flow queries                          │  │
│  │  - Mock management                       │  │
│  └──────────────────────────────────────────┘  │
│  ┌──────────────────────────────────────────┐  │
│  │  Proxy Engine (np-proxy)                │  │
│  │  - HTTP/HTTPS interception              │  │
│  │  - TLS MITM                              │  │
│  │  - Mock resolution                       │  │
│  └──────────────────────────────────────────┘  │
│  ┌──────────────────────────────────────────┐  │
│  │  Storage (np-store)                      │  │
│  │  - SQLite event log                      │  │
│  │  - Flow aggregation                      │  │
│  └──────────────────────────────────────────┘  │
│  ┌──────────────────────────────────────────┐  │
│  │  MCP Server (np-mcp)                     │  │
│  │  - 34 automation tools                   │  │
│  │  - System integration                    │  │
│  └──────────────────────────────────────────┘  │
└─────────────────────────────────────────────────┘
```

**Learn more:** [Architecture Documentation](docs/architecture.md)

---

## 🚧 Current Status

### ✅ Completed Features
- [x] Core proxy engine with TLS interception
- [x] Native SwiftUI macOS app
- [x] JSON viewer (4 modes: Tree, Raw, Table, Preview)
- [x] Session management with persistent storage
- [x] Mock system with pattern matching
- [x] Map Local/Remote from UI
- [x] iOS device support with certificate trust
- [x] MCP integration (34 tools)
- [x] Claude Desktop & Claude Code integration
- [x] macOS system proxy automation
- [x] Android (ADB) and iOS Simulator support

### 🚧 In Progress
- [ ] Multi-session selection and bulk delete
- [ ] Session import/export UI
- [ ] Request breakpoints
- [ ] Complete macOS menu bar

### 📋 Planned
- [ ] Mock rule manager UI
- [ ] Settings/Preferences window
- [ ] Windows and Linux support
- [ ] Browser extensions

**Full Roadmap:** [UI Features TODO](docs/TODO-UI-FEATURES.md)

---

## 🛠️ Development

### Prerequisites
- Rust 1.70+
- Swift 5.9+ (macOS 14.0+)
- Xcode 15+

### Build
```bash
# Build Rust components
cargo build --release

# Build SwiftUI app
cd app
swift build

# Run app
swift run
```

### Testing
```bash
# Run Rust tests
cargo test

# Test MCP integration
./scripts/test-mcp-proxy.sh

# Test iOS setup
./scripts/diagnose-ios.sh
```

**Full Guide:** [Development Documentation](docs/development.md)

---

## 📦 Project Structure

```
netproxy/
├── crates/
│   ├── netproxyd/          # Main daemon
│   ├── np-proxy/           # Proxy engine
│   ├── np-store/           # SQLite storage
│   ├── np-mock/            # Mock system
│   └── np-mcp/             # MCP server
├── app/
│   └── Sources/NetProxy/   # SwiftUI app
├── docs/                   # Documentation
│   ├── wiki.html          # Documentation viewer
│   └── *.md               # Guides and references
└── scripts/               # Setup and testing scripts
```

---

## 🤝 Contributing

We welcome contributions! Please:

1. Read the [Development Guide](docs/development.md)
2. Check the [UI Roadmap](docs/TODO-UI-FEATURES.md) for features to implement
3. Review the [Architecture](docs/architecture.md)
4. Submit a pull request

---

## 📄 License

MIT License - see [LICENSE](LICENSE) file for details

---

## 🙏 Acknowledgments

Built with:
- **Rust** - Proxy engine and daemon
- **SwiftUI** - Native macOS interface
- **SQLite** - Persistent storage
- **MCP** - AI integration protocol
- **Claude** - AI-powered automation

---

## 📞 Support

- **Documentation:** [https://djsmk123.github.io/netproxy/](https://djsmk123.github.io/netproxy/)
- **Issues:** [GitHub Issues](https://github.com/Djsmk123/netproxy/issues)
- **Discussions:** [GitHub Discussions](https://github.com/Djsmk123/netproxy/discussions)

---

**Made with ❤️ by MD Mobin**  
**AI-Assisted Development with Claude Sonnet 4.5**
