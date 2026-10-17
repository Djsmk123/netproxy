# NetProxy Documentation

Complete documentation for NetProxy - the modern HTTP/HTTPS proxy for development and testing.

---

## 🚀 Quick Start

### For End Users

- **[iOS Device Setup Guide](IOS-SETUP-GUIDE.md)** - Connect your iPhone/iPad
- **[MCP Proxy Setup](MCP-PROXY-SETUP.md)** - macOS system proxy via MCP tools

### For AI/Automation (MCP Integration)

- **[Claude Desktop MCP Setup](CLAUDE-MCP-SETUP.md)** - Integrate with Claude Desktop app
- **[Claude Code MCP Setup](CLAUDE-CODE-MCP-SETUP.md)** - Integrate with Claude Code CLI
- **[MCP Proxy Tools Reference](MCP-PROXY-SETUP.md)** - Available MCP tools and examples

---

## 📚 Core Documentation

### Architecture & Development

- **[Architecture Overview](architecture.md)** - System design and components
- **[Development Guide](development.md)** - Build, test, and contribute
- **[Control API Reference](control-api.md)** - JSON-RPC API documentation

### User Interface

- **[UI Feature Roadmap](TODO-UI-FEATURES.md)** - Planned features and implementation status
- **[UI Redesign Plans](ui-redesign/)** - Detailed UI specifications
  - [Charles Feature Parity](ui-redesign/01-charles-feature-parity.md)
  - [Screen Hierarchy](ui-redesign/02-screen-hierarchy.md)
  - [JSON Viewer Spec](ui-redesign/03-json-viewer-spec.md)
  - [Component Inventory](ui-redesign/04-component-inventory.md)
  - [Implementation Plan](ui-redesign/05-implementation-plan.md)

---

## 🎯 Quick Links by Use Case

### "I want to test my iOS app"
→ [iOS Setup Guide](IOS-SETUP-GUIDE.md)

### "I want to use Claude to control NetProxy"
→ [Claude Desktop Setup](CLAUDE-MCP-SETUP.md) or [Claude Code Setup](CLAUDE-CODE-MCP-SETUP.md)

### "I want to automate proxy configuration"
→ [MCP Proxy Tools](MCP-PROXY-SETUP.md)

### "I want to contribute to NetProxy"
→ [Development Guide](development.md) + [Architecture](architecture.md)

### "I want to build on top of NetProxy"
→ [Control API Reference](control-api.md)

---

## 📖 Documentation Index

### Setup Guides
| Guide | Description | Platform |
|-------|-------------|----------|
| [iOS Setup](IOS-SETUP-GUIDE.md) | Configure iOS devices to use NetProxy | iOS |
| [MCP Proxy Setup](MCP-PROXY-SETUP.md) | System proxy via MCP tools | macOS |
| [Claude Desktop MCP](CLAUDE-MCP-SETUP.md) | AI-powered proxy control | macOS |
| [Claude Code MCP](CLAUDE-CODE-MCP-SETUP.md) | CLI/IDE integration | Cross-platform |

### Technical Documentation
| Doc | Description |
|-----|-------------|
| [Architecture](architecture.md) | System design overview |
| [Development](development.md) | Build and contribute |
| [Control API](control-api.md) | JSON-RPC API reference |

### Planning & Roadmap
| Doc | Description |
|-----|-------------|
| [UI Roadmap](TODO-UI-FEATURES.md) | Feature implementation status |
| [UI Redesign](ui-redesign/) | Detailed UI specifications |

---

## 🔧 MCP Integration Overview

NetProxy provides comprehensive MCP (Model Context Protocol) integration for AI-powered proxy control:

### Available MCP Tools (34 total)

**Engine Control:**
- `engine_start`, `engine_stop`, `engine_status`

**System Proxy (macOS):**
- `system_proxy_set`, `system_proxy_clear`, `system_proxy_status`

**Traffic Management:**
- `flows_list`, `flows_get`, `flows_filter`

**Mock Rules:**
- `mocks_add`, `mocks_list`, `mocks_update`, `mocks_remove`, `mocks_from_flow`

**Mobile Devices:**
- `adb_set_proxy`, `ios_sim_set_proxy` (and clear variants)

**Browser Control:**
- `chrome_launch`, `firefox_launch`

**Session Management:**
- `runs_list`, `runs_delete`, `sessions_save`, `sessions_load`

**See full list:** [MCP Proxy Setup](MCP-PROXY-SETUP.md)

---

## 🎨 UI Implementation Status

### ✅ Completed
- Session deletion (permanent data removal)
- Enhanced request context menu (save, copy, filter)
- Map Local/Remote dialogs (create mocks from UI)
- iOS device support with certificate trust

### 🚧 In Progress
- Multi-session selection and bulk delete
- Session import/export (HAR, JSON)
- Request breakpoints
- Complete macOS menu bar

### 📋 Planned
- Breakpoint management UI
- Mock rule manager
- Settings/Preferences window
- Help and About windows

**See full roadmap:** [TODO-UI-FEATURES.md](TODO-UI-FEATURES.md)

---

## 🤝 Contributing

1. Read [Development Guide](development.md)
2. Check [UI Roadmap](TODO-UI-FEATURES.md) for features to implement
3. Review [Architecture](architecture.md) to understand the system
4. See [Control API](control-api.md) for backend integration

---

## 📝 Documentation Standards

### File Naming
- `ALLCAPS-WITH-DASHES.md` - User-facing guides
- `lowercase-with-dashes.md` - Technical documentation
- `##-descriptive-name.md` - Numbered series (UI redesign)

### Structure
- Start with one-line description
- Include quick start section
- Provide examples
- Add troubleshooting section
- Keep under 500 lines (split if longer)

---

## 🔄 Keeping Docs Updated

**When adding features:**
1. Update relevant guide
2. Add to [UI Roadmap](TODO-UI-FEATURES.md) if UI-related
3. Update [Control API](control-api.md) if adding API endpoints
4. Update [MCP Proxy Setup](MCP-PROXY-SETUP.md) if adding MCP tools

**When writing new docs:**
1. Add entry to this README
2. Follow naming conventions
3. Include examples
4. Link from relevant docs

---

## 📞 Getting Help

**Found outdated docs?** Please file an issue!

**Need clarification?** Check the guides first, then ask in discussions.

**Want to improve docs?** PRs welcome!

---

**Last Updated:** 2026-10-17  
**NetProxy Version:** 1.0  
**Documentation Version:** 2.0
