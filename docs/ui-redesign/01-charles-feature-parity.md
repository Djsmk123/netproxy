# Charles Proxy Feature Parity Matrix

**Purpose:** Audit Charles Proxy's complete feature set and map to NetProxy screens and implementation priorities.

**Date:** 2026-10-17  
**Status:** Design Phase

---

## Feature Categories

### 1. Traffic Capture & Inspection

| Feature | Charles | NetProxy Current | NetProxy Screen | Priority | Dependencies | Status |
|---------|---------|------------------|-----------------|----------|--------------|--------|
| **HTTP/HTTPS capture** | ✅ | ✅ | Main Window | P0 | ✅ Engine | ✅ Done |
| **WebSocket** | ✅ | ❌ | Main Window | P1 | Engine upgrade | 📋 Planned |
| **HTTP/2** | ✅ | ✅ Partial | Main Window | P1 | Engine | 🔄 Partial |
| **gRPC** | ✅ | ❌ | Main Window | P2 | Protocol decoder | 📋 Planned |
| **GraphQL** | ✅ (addon) | ❌ | GraphQL Inspector | P1 | Body parser | 📋 Planned |
| **Request/Response bodies** | ✅ | ✅ | Inspector Panel | P0 | ✅ Store | ✅ Done |
| **Headers** | ✅ | ✅ | Inspector Panel | P0 | ✅ Store | ✅ Done |
| **Cookies** | ✅ | ❌ | Inspector Panel | P1 | Header parser | 📋 Planned |
| **Query parameters** | ✅ | ❌ | Inspector Panel | P1 | URL parser | 📋 Planned |
| **Form data** | ✅ | ❌ | Inspector Panel | P1 | Body parser | 📋 Planned |
| **Timing breakdown** | ✅ | ❌ | Timing Panel | P1 | Event timing | 📋 Planned |
| **Sizes (compressed/uncompressed)** | ✅ | ❌ | Inspector Panel | P1 | Headers | 📋 Planned |

### 2. TLS/SSL

| Feature | Charles | NetProxy Current | NetProxy Screen | Priority | Dependencies | Status |
|---------|---------|------------------|-----------------|----------|--------------|--------|
| **MITM TLS interception** | ✅ | ✅ | Settings | P0 | ✅ Engine | ✅ Done |
| **Certificate generation** | ✅ | ✅ | Settings | P0 | ✅ CA | ✅ Done |
| **Certificate pinning bypass** | ✅ | ✅ Automatic | - | P0 | ✅ CA | ✅ Done |
| **TLS version display** | ✅ | ❌ | Inspector > TLS | P2 | Engine metadata | 📋 Planned |
| **Cipher suite display** | ✅ | ❌ | Inspector > TLS | P2 | Engine metadata | 📋 Planned |
| **Certificate chain view** | ✅ | ❌ | Inspector > TLS | P2 | Engine metadata | 📋 Planned |
| **Client certificates** | ✅ | ❌ | Settings | P3 | Engine upgrade | 📋 Planned |

### 3. Filtering & Search

| Feature | Charles | NetProxy Current | NetProxy Screen | Priority | Dependencies | Status |
|---------|---------|------------------|-----------------|----------|--------------|--------|
| **Filter by host** | ✅ | ✅ Basic | Sidebar | P0 | ✅ UI | ✅ Done |
| **Filter by method** | ✅ | ✅ | Sidebar | P0 | ✅ UI | ✅ Done |
| **Filter by status** | ✅ | ✅ | Sidebar | P0 | ✅ UI | ✅ Done |
| **Filter by content-type** | ✅ | ❌ | Sidebar | P1 | UI | 📋 Planned |
| **Search body** | ✅ | ❌ | Search Panel | P1 | Full-text search | 📋 Planned |
| **Search headers** | ✅ | ❌ | Search Panel | P1 | UI | 📋 Planned |
| **Regex search** | ✅ | ❌ | Search Panel | P1 | Regex engine | 📋 Planned |
| **Find & replace** | ❌ | ❌ | - | - | - | - |
| **Focus mode** | ✅ | ❌ | Toolbar | P2 | UI | 📋 Planned |
| **Include/Exclude rules** | ✅ | ❌ | Filter Settings | P2 | UI + Store | 📋 Planned |

### 4. Request Editing & Replay

| Feature | Charles | NetProxy Current | NetProxy Screen | Priority | Dependencies | Status |
|---------|---------|------------------|-----------------|----------|--------------|--------|
| **Breakpoints** | ✅ | 🔄 UI only | Inspector | P1 | Engine integration | 🔄 In Progress |
| **Edit request** | ✅ | ❌ | Breakpoint Modal | P1 | Engine queue | 📋 Planned |
| **Edit response** | ✅ | ❌ | Breakpoint Modal | P1 | Engine queue | 📋 Planned |
| **Repeat request** | ✅ | ❌ | Context Menu | P1 | HTTP client | 📋 Planned |
| **Advanced repeat** | ✅ | ❌ | Repeat Tool | P2 | HTTP client | 📋 Planned |
| **Compose new request** | ✅ | ❌ | Composer Window | P1 | HTTP client | 📋 Planned |
| **Import cURL** | ✅ | ❌ | Composer | P2 | cURL parser | 📋 Planned |
| **Copy as cURL** | ✅ | ✅ | Context Menu | P0 | ✅ UI | ✅ Done |

### 5. Rewriting & Mapping

| Feature | Charles | NetProxy Current | NetProxy Screen | Priority | Dependencies | Status |
|---------|---------|------------------|-----------------|----------|--------------|--------|
| **Map Remote** | ✅ | ❌ | Rewrite Rules | P2 | Rule engine | 📋 Planned |
| **Map Local** | ✅ | ✅ (via mocks) | Mock Manager | P1 | ✅ Mock system | ✅ Done |
| **Rewrite headers** | ✅ | ❌ | Rewrite Rules | P2 | Rule engine | 📋 Planned |
| **Rewrite body** | ✅ | ❌ | Rewrite Rules | P2 | Rule engine | 📋 Planned |
| **Rewrite URL** | ✅ | ❌ | Rewrite Rules | P2 | Rule engine | 📋 Planned |
| **Block/Allow list** | ✅ | ❌ | Block Rules | P2 | Rule engine | 📋 Planned |
| **Block cookies** | ✅ | ❌ | Block Rules | P2 | Rule engine | 📋 Planned |

### 6. Throttling & Simulation

| Feature | Charles | NetProxy Current | NetProxy Screen | Priority | Dependencies | Status |
|---------|---------|------------------|-----------------|----------|--------------|--------|
| **Bandwidth throttling** | ✅ | 🔄 UI only | Toolbar Dropdown | P1 | Engine integration | 🔄 In Progress |
| **Latency simulation** | ✅ | 🔄 UI only | Throttle Settings | P1 | Engine integration | 🔄 In Progress |
| **Preset profiles** | ✅ | ✅ | Toolbar | P1 | ✅ UI | ✅ Done |
| **Custom profiles** | ✅ | ✅ | Throttle Settings | P1 | ✅ UI | ✅ Done |
| **Per-host throttling** | ✅ | ❌ | Throttle Settings | P2 | Engine + Rules | 📋 Planned |
| **Stability % (packet loss)** | ✅ | ❌ | Throttle Settings | P3 | Engine upgrade | 📋 Planned |

### 7. Mocking & Recording

| Feature | Charles | NetProxy Current | NetProxy Screen | Priority | Dependencies | Status |
|---------|---------|------------------|-----------------|----------|--------------|--------|
| **Mock responses** | ✅ | ✅ | Mock Manager | P0 | ✅ Mock system | ✅ Done |
| **Pattern matching** | ✅ | ✅ | Mock Rules | P0 | ✅ Mock system | ✅ Done |
| **Fixture files** | ✅ | ✅ | Mock Manager | P0 | ✅ Fixtures | ✅ Done |
| **Record & replay** | ✅ | ✅ | Flow → Mock | P1 | ✅ Converter | ✅ Done |
| **Mock priorities** | ✅ | ✅ | Mock Rules | P0 | ✅ Mock system | ✅ Done |
| **Auto-mock from capture** | ✅ | ✅ | Context Menu | P1 | ✅ Flow converter | ✅ Done |
| **Conditional mocking** | ✅ | ✅ | Mock Rules | P1 | ✅ Match specs | ✅ Done |
| **Mock failure scenarios** | ✅ | ✅ | Mock Actions | P1 | ✅ Mock system | ✅ Done |
| **Smart pattern detection** | ❌ | ✅ | Flow Converter | - | ✅ Pattern detector | ✅ **Better than Charles!** |

### 8. Session & Export

| Feature | Charles | NetProxy Current | NetProxy Screen | Priority | Dependencies | Status |
|---------|---------|------------------|-----------------|----------|--------------|--------|
| **Save session** | ✅ | ❌ | File Menu | P1 | Serialization | 📋 Planned |
| **Load session** | ✅ | ❌ | File Menu | P1 | Deserialization | 📋 Planned |
| **Auto-save** | ✅ | ❌ | Preferences | P2 | UI | 📋 Planned |
| **Export HAR** | ✅ | 🔄 UI only | Export Menu | P1 | HAR generator | 🔄 In Progress |
| **Export JSON** | ✅ | 🔄 UI only | Export Menu | P1 | Serialization | 🔄 In Progress |
| **Export CSV** | ✅ | 🔄 UI only | Export Menu | P2 | Serialization | 🔄 In Progress |
| **Export text** | ✅ | ❌ | Export Menu | P2 | Formatter | 📋 Planned |
| **Export cURL commands** | ✅ | ❌ | Export Menu | P2 | Formatter | 📋 Planned |
| **Clear session** | ✅ | ✅ | Toolbar | P0 | ✅ UI | ✅ Done |

### 9. Device Configuration

| Feature | Charles | NetProxy Current | NetProxy Screen | Priority | Dependencies | Status |
|---------|---------|------------------|-----------------|----------|--------------|--------|
| **macOS system proxy** | ✅ Manual | 🔄 MCP only | Devices Tool | P1 | System API | 🔄 In Progress |
| **iOS proxy config** | ✅ Manual | 🔄 MCP only | Devices Tool | P1 | simctl | 🔄 In Progress |
| **Android proxy (ADB)** | ✅ Manual | 🔄 MCP only | Devices Tool | P1 | ADB commands | 🔄 In Progress |
| **Browser proxy launch** | ✅ Manual | 🔄 MCP only | Devices Tool | P1 | Process spawn | 🔄 In Progress |
| **Certificate install guide** | ✅ | ❌ | Help > Setup | P1 | Documentation | 📋 Planned |
| **QR code for mobile setup** | ✅ | ❌ | Devices Tool | P2 | QR generator | 📋 Planned |
| **Remote connections** | ✅ | ❌ | Settings | P2 | Network config | 📋 Planned |

### 10. JSON Inspection (Priority!)

| Feature | Charles | NetProxy Current | NetProxy Screen | Priority | Dependencies | Status |
|---------|---------|------------------|-----------------|----------|--------------|--------|
| **JSON syntax highlighting** | ✅ | ❌ | JSON Viewer | **P0** | Syntax highlighter | 📋 **URGENT** |
| **Pretty print** | ✅ | ❌ | JSON Viewer | **P0** | Formatter | 📋 **URGENT** |
| **Tree view** | ✅ | ❌ | JSON Viewer | **P0** | Tree component | 📋 **URGENT** |
| **Expand/collapse all** | ✅ | ❌ | JSON Viewer | **P0** | Tree controls | 📋 **URGENT** |
| **Search keys/values** | ✅ | ❌ | JSON Viewer | **P0** | Search UI | 📋 **URGENT** |
| **Copy path** | ✅ | ❌ | JSON Viewer | **P0** | Path builder | 📋 **URGENT** |
| **Copy value** | ✅ | ❌ | JSON Viewer | **P0** | Clipboard | 📋 **URGENT** |
| **JSON Pointer support** | ❌ | ❌ | JSON Viewer | P1 | RFC 6901 | 📋 Planned |
| **Table mode** | ✅ | ❌ | JSON Viewer | P1 | Table renderer | 📋 Planned |
| **Raw mode** | ✅ | ❌ | JSON Viewer | **P0** | Text editor | 📋 **URGENT** |
| **Invalid JSON diagnostics** | ✅ | ❌ | JSON Viewer | **P0** | Parser | 📋 **URGENT** |
| **Large payload handling** | ✅ | ❌ | JSON Viewer | **P0** | Virtualization | 📋 **URGENT** |
| **Format/Validate** | ✅ | ❌ | JSON Viewer | P1 | Validator | 📋 Planned |
| **Compare** | ✅ | ❌ | Diff Tool | P2 | Diff engine | 📋 Planned |
| **Save as fixture** | ❌ | ❌ | JSON Viewer | P1 | File writer | 📋 Planned |

### 11. Other Body Formats

| Feature | Charles | NetProxy Current | NetProxy Screen | Priority | Dependencies | Status |
|---------|---------|------------------|-----------------|----------|--------------|--------|
| **XML viewer** | ✅ | ❌ | XML Viewer | P1 | XML parser | 📋 Planned |
| **HTML preview** | ✅ | ❌ | HTML Viewer | P2 | WebView | 📋 Planned |
| **Image preview** | ✅ | ❌ | Image Viewer | P1 | Image renderer | 📋 Planned |
| **Binary hex view** | ✅ | ❌ | Hex Viewer | P2 | Hex editor | 📋 Planned |
| **Form data parser** | ✅ | ❌ | Form Viewer | P1 | URL decoder | 📋 Planned |
| **Protobuf decoder** | ✅ | ❌ | Protobuf Viewer | P3 | Proto defs | 📋 Planned |
| **Text viewer** | ✅ | ❌ | Text Viewer | P1 | Text editor | 📋 Planned |

### 12. Extensibility & Integration

| Feature | Charles | NetProxy Current | NetProxy Screen | Priority | Dependencies | Status |
|---------|---------|------------------|-----------------|----------|--------------|--------|
| **MCP integration** | ❌ | ✅ | - | - | ✅ MCP server | ✅ **Better than Charles!** |
| **AI mock authoring** | ❌ | ✅ | - | - | ✅ MCP tools | ✅ **Better than Charles!** |
| **Custom tools (plugins)** | ✅ | ❌ | - | P3 | Plugin system | 📋 Planned |
| **Scripting** | ✅ (limited) | ❌ | - | P3 | Scripting engine | 📋 Planned |
| **CLI access** | ✅ | ✅ (via MCP) | - | P1 | ✅ Control API | ✅ Done |

### 13. UI/UX

| Feature | Charles | NetProxy Current | NetProxy Screen | Priority | Dependencies | Status |
|---------|---------|------------------|-----------------|----------|--------------|--------|
| **Dark mode** | ✅ | ✅ | - | P0 | ✅ SwiftUI | ✅ Done |
| **Keyboard shortcuts** | ✅ | ✅ Partial | - | P1 | UI | 🔄 In Progress |
| **Customizable layout** | ✅ | ❌ | - | P2 | Layout engine | 📋 Planned |
| **Quick filters** | ✅ | ✅ | Sidebar | P0 | ✅ UI | ✅ Done |
| **Structure/Sequence view** | ✅ | ❌ | View Toggle | P1 | Tree view | 📋 Planned |
| **Focused view** | ✅ | ❌ | Toolbar | P2 | UI state | 📋 Planned |

---

## Priority Legend

- **P0**: Core functionality, must have for launch
- **P1**: Important features, next phase
- **P2**: Nice to have, future releases
- **P3**: Low priority, may defer indefinitely

## Status Legend

- ✅ **Done**: Feature complete and tested
- 🔄 **In Progress**: Partially implemented
- 📋 **Planned**: Designed, ready to implement
- ❌ **Not Started**: No implementation yet

---

## NetProxy Advantages Over Charles

1. **✅ Smart Pattern Detection** - Automatic ID/UUID/slug detection for mock rules
2. **✅ MCP Integration** - AI-assisted mock management (Claude integration)
3. **✅ Modern Architecture** - Rust backend, SwiftUI frontend
4. **✅ Native macOS UI** - First-class macOS experience
5. **✅ Hot Reload** - Mock rules reload without restart
6. **✅ Advanced Validation** - Mock rule conflict detection

---

## Feature Gap Summary

### Critical Gaps (P0 - Blocking launch)
1. **JSON Viewer** - No syntax highlighting, tree view, or formatting
2. **Request/Response Inspector** - Limited to raw view
3. **Body parsing** - No JSON/XML/Form parsing
4. **Query/Cookie inspection** - Not parsed or displayed

### Important Gaps (P1 - Needed for feature parity)
1. **Breakpoints** - UI only, no engine integration
2. **Throttling** - UI only, no actual throttling
3. **WebSocket** - Not supported
4. **GraphQL** - No GraphQL-specific tooling
5. **Repeat/Replay** - Cannot resend requests
6. **Composer** - No request composition tool
7. **Timing breakdown** - No waterfall view
8. **HAR export** - UI only, no actual export
9. **Session save/load** - Not implemented

### Nice-to-Have (P2 - Competitive features)
1. **Rewrite rules** - Map Remote, header/body rewriting
2. **Advanced search** - Regex, full-text body search
3. **Structure view** - Tree-based navigation
4. **HTML/Image preview** - Better body viewers
5. **Custom profiles** - Saved throttling/filter presets

---

## Next Steps

1. **Design JSON Viewer** (see 03-json-viewer-spec.md)
2. **Design Inspector Panel** (all body types)
3. **Design Screen Hierarchy** (see 02-screen-hierarchy.md)
4. **Design Component Library** (see 04-component-inventory.md)
5. **Create Implementation Plan** (see 05-implementation-plan.md)
