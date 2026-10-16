# NetProxy UI Implementation Plan

**Purpose:** Phased approach to build the comprehensive Charles-alternative UI.

**Date:** 2026-10-17  
**Status:** Design Phase

---

## Implementation Strategy

**Approach:** Incremental, feature-by-feature  
**Testing:** Build and test each phase before moving to next  
**Deployment:** Ship usable increments, not all-or-nothing  

**DO NOT:**
- Rewrite the entire app in one pass
- Break existing functionality
- Ship incomplete features

**DO:**
- Build one feature at a time
- Test thoroughly before next phase
- Maintain backward compatibility

---

## Phase 0: Foundation (Week 1)

**Goal:** Set up architecture, component library, and design system

### Tasks

1. **Design System Setup**
   - [ ] Create `DesignSystem.swift` with colors, fonts, spacing
   - [ ] Define dark mode colors
   - [ ] Define SF Symbol icon set
   - [ ] Create style guide document

2. **Component Library Scaffold**
   - [ ] Create folder structure (`Views/Common`, `Views/Inspector`, etc.)
   - [ ] Set up component playground/preview
   - [ ] Create base components:
     - [ ] `SearchBar`
     - [ ] `ToastView`
     - [ ] `LoadingOverlay`
     - [ ] `MethodBadge`
     - [ ] `StatusBadge`

3. **Model Updates**
   - [ ] Add `Cookie` model
   - [ ] Add `FlowTiming` model
   - [ ] Add `JSONNode` model for tree view
   - [ ] Update `Flow` with parsed query params, cookies

4. **Testing**
   - [ ] All base components render in preview
   - [ ] Dark mode works correctly
   - [ ] Components follow HIG

**Acceptance Criteria:**
✅ Component library structure in place  
✅ Design system documented  
✅ All base components built and previewed  
✅ Zero compilation errors  

---

## Phase 1: JSON Viewer (Week 2-3) **PRIORITY**

**Goal:** Build world-class JSON inspection

### Tasks

1. **JSON Parsing**
   - [ ] Create `JSONParser` that handles invalid JSON
   - [ ] Build `JSONNode` tree structure
   - [ ] Implement lazy loading for large arrays
   - [ ] Add error diagnostics (line, column)

2. **Tree View**
   - [ ] Build `TreeView` generic component
   - [ ] Build `JSONTreeView` with expand/collapse
   - [ ] Add syntax highlighting colors
   - [ ] Implement keyboard navigation (←→↑↓)
   - [ ] Add context menu (copy path, copy value)
   - [ ] Virtual scrolling for large trees

3. **Raw View**
   - [ ] Build `JSONRawView` with syntax highlighting
   - [ ] Add line numbers
   - [ ] Implement format/minify
   - [ ] Add search with highlights

4. **Table View**
   - [ ] Build `JSONTableView` for arrays
   - [ ] Sortable columns
   - [ ] Export to CSV

5. **Search**
   - [ ] Search bar in toolbar
   - [ ] Filter keys/values/both
   - [ ] Navigate between matches
   - [ ] Regex support (optional)

6. **Actions**
   - [ ] Format/Minify buttons
   - [ ] Copy entire JSON
   - [ ] Save as fixture
   - [ ] Validate

**Testing:**
- [ ] Test with valid JSON (1KB, 100KB, 1MB, 10MB)
- [ ] Test with invalid JSON (show error location)
- [ ] Test search across 1MB payload
- [ ] Test copy path/value
- [ ] Performance: 60fps scrolling on 10MB JSON

**Acceptance Criteria:**
✅ Tree view with expand/collapse works  
✅ Raw view with syntax highlighting  
✅ Search finds matches instantly  
✅ Copy path generates correct JSONPath  
✅ Handles 10MB JSON without freezing  
✅ Invalid JSON shows precise error  

---

## Phase 2: Request/Response Inspector (Week 4)

**Goal:** Complete inspector panel for flow details

### Tasks

1. **Inspector Tabs**
   - [ ] Build tabbed inspector component
   - [ ] Request tabs: Headers, Body, Cookies, Query
   - [ ] Response tabs: Headers, Body, Cookies, Timing, TLS

2. **Headers View**
   - [ ] Build `HTTPHeadersView` with search
   - [ ] Context menu (copy name, value, both)
   - [ ] Filtered view

3. **Cookies View**
   - [ ] Parse Set-Cookie headers
   - [ ] Build `CookiesView` table
   - [ ] Show expiry, domain, path, secure, httpOnly

4. **Query Params View**
   - [ ] Parse URL query string
   - [ ] Build key-value table
   - [ ] Copy as object, copy as string

5. **Timing View**
   - [ ] Build `TimingWaterfallView`
   - [ ] Phase breakdown (DNS, Connect, TLS, Send, Wait, Receive)
   - [ ] Color-coded bars
   - [ ] Total time display

6. **TLS View**
   - [ ] Show TLS version
   - [ ] Cipher suite
   - [ ] Certificate chain (if available from backend)

7. **Body Type Detection**
   - [ ] Auto-detect JSON → JSONViewer
   - [ ] Auto-detect XML → XMLViewer (placeholder)
   - [ ] Auto-detect Image → ImageViewer (placeholder)
   - [ ] Auto-detect HTML → HTMLViewer (placeholder)
   - [ ] Fallback to text viewer

**Testing:**
- [ ] Test with real API responses (GitHub, httpbin, etc.)
- [ ] Verify cookies parse correctly
- [ ] Check timing waterfall renders
- [ ] Test with various content-types

**Acceptance Criteria:**
✅ All 6 tabs implemented  
✅ Headers, cookies, query params display correctly  
✅ JSON auto-detects and uses JSONViewer  
✅ Timing waterfall shows breakdown  
✅ Copy operations work  

---

## Phase 3: Enhanced Traffic List (Week 5)

**Goal:** Professional traffic list with filtering, search, context menu

### Tasks

1. **Enhanced Table**
   - [ ] Update `FlowListView` with better columns
   - [ ] Add mock/rule indicators (pills)
   - [ ] Add size formatting (bytes → KB/MB)
   - [ ] Sortable columns
   - [ ] Multi-selection

2. **Context Menu**
   - [ ] Copy as cURL (✅ already exists)
   - [ ] Filter by Host
   - [ ] Filter by Method
   - [ ] Filter by Status Class
   - [ ] Repeat Request
   - [ ] Convert to Mock
   - [ ] Delete Flow

3. **Search Bar**
   - [ ] Add search above traffic list
   - [ ] Search by host, path, method, status
   - [ ] Highlight matches

4. **Visual Indicators**
   - [ ] MOCK badge for mocked flows
   - [ ] RULE badge showing which rule hit
   - [ ] Breakpoint icon for paused flows
   - [ ] Modified icon for edited flows

**Testing:**
- [ ] Test filtering by various criteria
- [ ] Test search with 1000+ flows
- [ ] Verify context menu actions work
- [ ] Test multi-selection delete

**Acceptance Criteria:**
✅ Context menu has all actions  
✅ Search filters correctly  
✅ Visual indicators show  
✅ Performance: 60fps scrolling with 1000+ flows  

---

## Phase 4: Mock Manager (Week 6)

**Goal:** Complete mock management UI

### Tasks

1. **Mock Manager Window**
   - [ ] Build floating window (⌘⇧M)
   - [ ] Split view: rules list + detail editor
   - [ ] Validation panel

2. **Rules List**
   - [ ] Show all rules (static + dynamic)
   - [ ] Enable/disable toggle per rule
   - [ ] Priority sorting
   - [ ] Search/filter rules

3. **Rule Editor**
   - [ ] Edit match spec (host, path, method, query, headers)
   - [ ] Edit action (respond, pass, fail)
   - [ ] Edit status, headers, body
   - [ ] Fixture picker
   - [ ] Priority slider

4. **Validation Panel**
   - [ ] Show validation issues
   - [ ] Severity icons (error, warning, info)
   - [ ] Click to jump to rule

5. **Quick Actions**
   - [ ] Convert from Flow (context menu in traffic list)
   - [ ] Pattern analysis integration (Phase 3 feature)
   - [ ] Bulk enable/disable
   - [ ] Import/Export

**Testing:**
- [ ] Create, edit, delete rules
- [ ] Validation shows errors
- [ ] Convert flow to mock works
- [ ] Fixtures load correctly

**Acceptance Criteria:**
✅ Mock manager window functional  
✅ Can create/edit/delete rules  
✅ Validation shows issues  
✅ Convert from flow works  
✅ Pattern analysis integration  

---

## Phase 5: Devices & Proxy Setup (Week 7)

**Goal:** One-click device configuration

### Tasks

1. **Devices Window**
   - [ ] Build floating window (⌘⇧D)
   - [ ] Sections: macOS, iOS, Android, Browsers

2. **macOS System Proxy**
   - [ ] Show current proxy status
   - [ ] Set/Clear buttons
   - [ ] Use MCP system_proxy tools

3. **iOS Simulator**
   - [ ] List available simulators
   - [ ] Configure proxy button
   - [ ] Install certificate button
   - [ ] Use MCP ios_sim tools

4. **Android (ADB)**
   - [ ] List connected devices
   - [ ] Configure proxy button
   - [ ] Install certificate button
   - [ ] Use MCP adb tools

5. **Browsers**
   - [ ] Launch Chrome/Firefox/Safari with proxy
   - [ ] Use MCP browser tools

6. **Certificate Management**
   - [ ] Export CA certificate
   - [ ] Show install instructions
   - [ ] Check trust status

**Testing:**
- [ ] Test macOS proxy set/clear
- [ ] Test iOS Simulator proxy (if available)
- [ ] Test ADB proxy (with emulator)
- [ ] Test Chrome launch with proxy

**Acceptance Criteria:**
✅ Devices window shows all sections  
✅ macOS proxy works  
✅ ADB configuration works  
✅ Chrome launch works  
✅ CA export works  

---

## Phase 6: Session Management (Week 8)

**Goal:** Save, load, organize sessions

### Tasks

1. **Session Manager**
   - [ ] Build Sessions window
   - [ ] List saved sessions
   - [ ] Show session details (flows count, mocks, size)

2. **Save Session**
   - [ ] Save current flows + mocks
   - [ ] Session metadata (name, date, tags)
   - [ ] Use MCP session_save

3. **Load Session**
   - [ ] Restore flows and mocks
   - [ ] Merge or replace current
   - [ ] Use MCP session_load

4. **Session List**
   - [ ] Searchable/filterable
   - [ ] Delete session
   - [ ] Export as HAR

5. **Auto-Save**
   - [ ] Optional auto-save on stop
   - [ ] Preference setting

**Testing:**
- [ ] Save session with flows + mocks
- [ ] Load session, verify flows restored
- [ ] Delete session
- [ ] Export as HAR

**Acceptance Criteria:**
✅ Can save current session  
✅ Can load saved session  
✅ Session list shows all saved sessions  
✅ Export as HAR works  

---

## Phase 7: Advanced Features (Week 9-10)

**Goal:** Breakpoints, throttling, repeat, composer

### Tasks

1. **Breakpoints (Backend Integration)**
   - [ ] Implement engine-side request queue
   - [ ] Control API: `breakpoint.pause`, `breakpoint.continue`, `breakpoint.cancel`
   - [ ] UI: Breakpoint modal editor
   - [ ] Edit request/response in modal
   - [ ] Continue/Cancel buttons

2. **Throttling (Backend Integration)**
   - [ ] Implement engine-side bandwidth throttling
   - [ ] Control API: `throttle.set`, `throttle.clear`
   - [ ] UI: Dropdown already exists, wire it up

3. **Repeat Request**
   - [ ] HTTP client to resend requests
   - [ ] Context menu: Repeat Request
   - [ ] Show result in traffic list (with indicator)

4. **Request Composer**
   - [ ] Build Composer window (⌘N)
   - [ ] Method, URL, headers, body editors
   - [ ] Send button (uses HTTP client)
   - [ ] Import cURL
   - [ ] Save as template

5. **HAR Export (Backend)**
   - [ ] Implement HAR generator in backend
   - [ ] Control API: `export.har`
   - [ ] UI: Export dialog already exists, wire it up

**Testing:**
- [ ] Breakpoint pauses request
- [ ] Edit and continue works
- [ ] Throttling slows traffic
- [ ] Repeat request succeeds
- [ ] Composer sends request
- [ ] HAR export valid

**Acceptance Criteria:**
✅ Breakpoints pause and allow editing  
✅ Throttling actually throttles  
✅ Repeat request works  
✅ Composer sends requests  
✅ HAR export generates valid HAR  

---

## Phase 8: Polish & Performance (Week 11)

**Goal:** Optimize, accessibility, keyboard shortcuts

### Tasks

1. **Performance**
   - [ ] Profile with Instruments
   - [ ] Optimize tree view rendering
   - [ ] Optimize JSON parsing
   - [ ] Lazy load everything
   - [ ] Virtual scrolling everywhere

2. **Accessibility**
   - [ ] VoiceOver support for all components
   - [ ] Keyboard navigation for all screens
   - [ ] High contrast mode
   - [ ] Respect system text size

3. **Keyboard Shortcuts**
   - [ ] Document all shortcuts
   - [ ] Add to menu bar
   - [ ] Customizable shortcuts (future)

4. **Menu Bar**
   - [ ] **File Menu:**
     - New Request (⌘N)
     - Open Session (⌘O)
     - Save Session (⌘S)
     - Export... (⌘⇧E)
     - Close Window (⌘W)
   - [ ] **Edit Menu:**
     - Copy (⌘C)
     - Copy Path (⌘⌥C)
     - Clear List (⌘K)
     - Find... (⌘F)
   - [ ] **View Menu:**
     - Start/Stop Recording (⌘R)
     - Show Sidebar (⌘⌥S)
     - Actual Size (⌘0)
     - Zoom In (⌘+)
     - Zoom Out (⌘-)
   - [ ] **Tools Menu:**
     - Mock Manager (⌘⇧M)
     - Devices (⌘⇧D)
     - Throttling (⌘⇧T)
     - Breakpoints (⌘⇧B)
   - [ ] **Window Menu:**
     - Minimize (⌘M)
     - Bring All to Front
   - [ ] **Help Menu:**
     - NetProxy Help
     - Certificate Setup Guide
     - GitHub Repository
     - Report Issue
     - ---
     - About NetProxy

5. **Error Handling**
   - [ ] Graceful error messages
   - [ ] Retry logic for network errors
   - [ ] Crash recovery

**Testing:**
- [ ] Test with VoiceOver
- [ ] Test all keyboard shortcuts
- [ ] Performance test with 10,000 flows
- [ ] Accessibility audit

**Acceptance Criteria:**
✅ All shortcuts work  
✅ VoiceOver reads all UI  
✅ Menu bar complete  
✅ No crashes under load  
✅ 60fps everywhere  

---

## Phase 9: Documentation (Week 12)

**Goal:** User guide, developer docs, marketing

### Tasks

1. **User Guide**
   - [ ] Getting Started
   - [ ] Certificate Setup
   - [ ] Basic Traffic Capture
   - [ ] JSON Inspection
   - [ ] Creating Mocks
   - [ ] Device Configuration
   - [ ] Breakpoints & Throttling
   - [ ] Sessions

2. **Developer Docs**
   - [ ] Architecture overview
   - [ ] Component library docs
   - [ ] Contributing guide
   - [ ] MCP integration guide

3. **Marketing Materials**
   - [ ] Screenshot gallery
   - [ ] Feature comparison (vs Charles)
   - [ ] Video demo
   - [ ] README update

4. **Release Notes**
   - [ ] Complete changelog
   - [ ] Migration guide (if needed)
   - [ ] Known issues

**Acceptance Criteria:**
✅ User guide complete  
✅ Developer docs complete  
✅ Screenshots taken  
✅ README updated  

---

## Dependencies & Risks

### Backend Dependencies

| Feature | Backend API Required | Status | Risk |
|---------|---------------------|--------|------|
| JSON Viewer | None (parsing in UI) | ✅ No risk | Low |
| Breakpoints | `breakpoint.pause/continue/cancel` | ❌ Not built | High |
| Throttling | `throttle.set/clear` | ❌ Not built | High |
| HAR Export | `export.har` | ❌ Not built | Medium |
| Repeat Request | HTTP client | ❌ Not built | Medium |
| WebSocket | Engine upgrade | ❌ Not built | High |
| GraphQL | Body parser | ❌ Not built | Low |

### Mitigation

- **Breakpoints/Throttling**: Mock UI first, backend later
- **HAR Export**: Client-side generation possible
- **Repeat**: Use URLSession directly
- **WebSocket**: Defer to later release

---

## Success Metrics

| Metric | Target |
|--------|--------|
| **JSON parsing** | < 100ms for 1MB |
| **UI render** | 60fps scrolling with 1000+ flows |
| **Search** | < 200ms across 10MB JSON |
| **Memory** | < 2× dataset size |
| **Startup** | < 2 seconds |
| **Feature parity** | 80% of Charles features |

---

## Release Strategy

### Alpha (After Phase 1-3)
- Internal testing
- JSON viewer + Inspector working
- Traffic list enhanced

### Beta (After Phase 4-6)
- Public testing
- Mock manager + Devices working
- Session management working

### 1.0 (After Phase 7-9)
- Public release
- All P0 and P1 features
- Documentation complete

---

## Summary

**Timeline:** 12 weeks (3 months)  
**Team Size:** 1-2 developers  
**Approach:** Incremental, ship early and often  

**Phases:**
1. ✅ Foundation (1 week)
2. **JSON Viewer (2 weeks)** ← START HERE
3. Inspector (1 week)
4. Traffic List (1 week)
5. Mock Manager (1 week)
6. Devices (1 week)
7. Sessions (1 week)
8. Advanced (2 weeks)
9. Polish (1 week)
10. Docs (1 week)

**Next Action:** Begin Phase 0 - Foundation setup
