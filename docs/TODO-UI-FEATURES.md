# NetProxy UI Feature Implementation Plan

Comprehensive roadmap for completing Charles Proxy parity and MCP exposure.

---

## ✅ Completed Features

### Session Management
- [x] Delete single session permanently
- [x] Clear UI view
- [x] Auto-select next session after delete

### Request Context Menu  
- [x] Copy URL
- [x] Copy as cURL
- [x] Save Request to file
- [x] Save Response to file
- [x] Save Both to file
- [x] Filter by Host/Method/Status

### Device Support
- [x] iOS device proxy configuration
- [x] Certificate installation and trust
- [x] Remember "Allow Devices" setting

---

## 🚧 In Progress / Planned Features

### 1. Multi-Session Management

**Select All & Bulk Delete:**
- [ ] Add "Select All" option in sessions context menu
- [ ] Multi-select sessions (Cmd+Click, Shift+Click)
- [ ] Delete multiple sessions at once
- [ ] Confirmation dialog showing count

**Implementation:**
```swift
// CaptureModel.swift
public func deleteMultipleSessions(_ runs: [String]) async
```

---

### 2. Session Import/Export

**Export Session:**
- [ ] HAR format export (already partially exists in ExportView)
- [ ] JSON export (complete flow data)
- [ ] cURL script export (all requests)
- [ ] Context menu on session: "Export Session..."
- [ ] Top menu: File → Export Session...

**Import Session:**
- [ ] Import HAR files
- [ ] Import from other proxies (Charles, Fiddler)
- [ ] Top menu: File → Import Session...
- [ ] Drag & drop HAR files

**Implementation:**
```swift
// SessionManagementView.swift - enhance existing
// Add import/export buttons
// Support HAR, JSON, cURL formats
```

---

### 3. Request Breakpoints

**Breakpoint on Specific Request:**
- [ ] Right-click → "Breakpoint on Request"
- [ ] Match by URL pattern
- [ ] Pause before sending request
- [ ] Allow editing request
- [ ] Continue/Abort/Edit options

**Implementation:**
- Backend: Add breakpoint matching in proxy engine
- MCP: `breakpoints.add`, `breakpoints.list`, `breakpoints.remove`
- UI: Breakpoint indicator in flow list
- UI: Edit dialog when breakpoint hits

**Files to modify:**
```
crates/netproxyd/src/engine.rs - add breakpoint checks
app/Sources/NetProxy/BreakpointManager.swift - NEW
app/Sources/NetProxy/BreakpointEditView.swift - NEW
```

---

### 4. Map Local & Map Remote

**Map Local:**
- [ ] Right-click → "Map Local..."
- [ ] Choose local file to serve
- [ ] Match URL pattern
- [ ] Store in mock rules

**Map Remote:**
- [ ] Right-click → "Map Remote..."
- [ ] Redirect to different URL
- [ ] Preserve path/query
- [ ] Store in redirect rules

**Implementation:**
```swift
// Use existing mock system
// Add UI for easy rule creation from flows
// MapLocalView.swift - NEW
// MapRemoteView.swift - NEW
```

**Backend:**
```rust
// Already have mock rules - just expose via UI
// Add redirect action type to mocks
```

---

### 5. Enhanced Menu Bar

**File Menu:**
- [ ] New Session
- [ ] Open Session...
- [ ] Save Session As...
- [ ] Import Session...
- [ ] Export Session...
- [ ] Recent Sessions submenu
- [ ] Close Window

**Edit Menu:**
- [ ] Copy
- [ ] Select All (flows)
- [ ] Find... (search flows)
- [ ] Clear Filters

**View Menu:**
- [ ] Show/Hide Sidebar
- [ ] Show/Hide Inspector
- [ ] Zoom In/Out
- [ ] Enter Full Screen

**Capture Menu:** (existing, enhance)
- [x] Start/Stop Capture
- [ ] Start with Options...
- [x] Clear List
- [ ] Clear Session Data
- [ ] Breakpoints submenu

**Tools Menu:**
- [x] Toggle Breakpoints
- [ ] Map Local...
- [ ] Map Remote...
- [ ] Mock Manager...
- [x] Export...
- [ ] Settings...

**Window Menu:**
- [ ] Minimize
- [ ] Zoom
- [ ] Bring All to Front

**Help Menu:**
- [ ] NetProxy Help
- [ ] Device Setup Guide
- [ ] Certificate Installation
- [ ] Keyboard Shortcuts
- [ ] Report Issue
- [ ] Check for Updates
- [ ] About NetProxy

**Implementation:**
```swift
// NetProxyApp.swift - expand CommandMenu sections
// Add HelpView.swift, AboutView.swift, SettingsView.swift
```

---

### 6. About Window

**Information to Display:**
- [ ] NetProxy version
- [ ] Build number
- [ ] macOS version compatibility
- [ ] License information
- [ ] Contributors
- [ ] GitHub repository link
- [ ] Release notes link

**Implementation:**
```swift
// AboutView.swift - NEW
struct AboutView: View {
    var version: String = Bundle.main.infoDictionary?["CFBundleShortVersionString"] as? String ?? "1.0"
    var build: String = Bundle.main.infoDictionary?["CFBundleVersion"] as? String ?? "1"
}
```

---

### 7. Settings/Preferences Window

**General:**
- [ ] Theme (Light/Dark/Auto)
- [ ] Default proxy port
- [ ] Auto-start recording
- [ ] Show notifications

**Recording:**
- [ ] Auto-capture on launch
- [ ] Default save location
- [ ] Maximum session size
- [ ] Auto-prune old sessions

**Network:**
- [ ] Upstream proxy
- [ ] SSL/TLS settings
- [ ] Ignore list (domains to skip)

**Advanced:**
- [ ] Database location
- [ ] Log level
- [ ] Performance tuning

**Implementation:**
```swift
// SettingsView.swift - NEW
// Use @AppStorage for persistence
```

---

### 8. MCP Tool Exposure

**All UI features should be accessible via MCP for AI automation.**

**Session Management:**
```json
{
  "sessions.list": "List all recorded sessions",
  "sessions.delete": "Delete session(s) by ID",
  "sessions.export": "Export session to HAR/JSON",
  "sessions.import": "Import HAR file as new session"
}
```

**Request Operations:**
```json
{
  "requests.save": "Save request/response to file",
  "requests.copy_curl": "Generate cURL command",
  "requests.filter": "Filter requests by criteria"
}
```

**Breakpoints:**
```json
{
  "breakpoints.add": "Add breakpoint on URL pattern",
  "breakpoints.list": "List all breakpoints",
  "breakpoints.remove": "Remove breakpoint",
  "breakpoints.enable": "Enable/disable breakpoint"
}
```

**Mocking:**
```json
{
  "mocks.map_local": "Create local file mapping",
  "mocks.map_remote": "Create remote URL redirect",
  "mocks.from_flow": "Convert captured flow to mock"
}
```

**Implementation:**
```rust
// crates/np-mcp/src/tools.rs - add new tools
// Each UI action gets corresponding MCP tool
// Reuse existing Control API methods
```

---

## 📊 Priority Order

### Phase 1: Core UX (Next Sprint)
1. ✅ Enhanced request context menu
2. Multi-session delete
3. Enhanced menu bar (Help, About)
4. Settings window

### Phase 2: Advanced Features
5. Breakpoints UI & backend
6. Map Local/Remote dialogs
7. Session import/export UI

### Phase 3: MCP Integration
8. Expose all features via MCP
9. Add MCP documentation
10. Create automation examples

---

## 🎯 Success Criteria

- [ ] All Charles Proxy equivalents implemented
- [ ] Complete macOS menu bar (standard menus)
- [ ] All features accessible via MCP
- [ ] Comprehensive keyboard shortcuts
- [ ] Help documentation complete
- [ ] Settings persist across sessions
- [ ] Import/export works with HAR standard

---

## 📝 Notes

**Design Philosophy:**
- Keep UI simple and intuitive
- Match macOS design guidelines
- Every UI action = MCP tool
- Keyboard shortcuts for power users
- Progressive disclosure (hide complexity)

**Testing:**
- Manual UI testing for each feature
- MCP tool integration tests
- Import/export format validation
- Performance testing (large sessions)

**Documentation:**
- Update user guide
- MCP tool examples
- Video tutorials
- Keyboard shortcut reference

---

**Last Updated:** 2026-10-17  
**Status:** In active development
