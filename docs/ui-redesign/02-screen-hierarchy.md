# NetProxy Screen Hierarchy & Navigation

**Purpose:** Define the complete screen structure, navigation patterns, and window organization.

**Date:** 2026-10-17  
**Status:** Design Phase

---

## Design Principles

1. **Traffic List is Central** - Always visible, never hidden
2. **Inspector is Contextual** - Shows detail for selected flow
3. **Tools are Accessible** - Dedicated panels/windows for advanced features
4. **No Overcrowding** - Split complex features across workspaces
5. **Native macOS** - Follow HIG, use standard patterns

---

## Main Window Structure

```
┌─────────────────────────────────────────────────────────────────┐
│  NetProxy                                        [🔄] [⏸] [🗑️]  │  ← Title Bar + Toolbar
├─────────────┬───────────────────────────────────────────────────┤
│             │                                                    │
│   Sidebar   │              Traffic List                         │
│             │                                                    │
│  ┌─────┐    │  Host       Path        Method  Status   Size    │
│  │Flows│    │  ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━   │
│  └─────┘    │  api.github  /user      GET     200     1.2 KB   │
│  Filters    │  httpbin.org /json      GET     200     428 B    │
│  Mocks      │  example.com /api/data  POST    201     5.4 KB   │
│  Sessions   │                                                    │
│  Devices    │                                                    │
│             │                                                    │
│             ├────────────────────────────────────────────────────┤
│             │                                                    │
│             │              Request Inspector                     │
│             │                                                    │
│             │  [Headers] [Body] [Cookies] [Query] [Timing]     │
│             │  ┌────────────────────────────────────────────┐   │
│             │  │  JSON Viewer                                │   │
│             │  │  [Tree] [Raw] [Table] [Preview]             │   │
│             │  │  {                                           │   │
│             │  │    "login": "djsmk123",                      │   │
│             │  │    "id": 12345,              Copy Path ⌘C   │   │
│             │  │    "repos": 42                               │   │
│             │  │  }                                           │   │
│             │  └────────────────────────────────────────────┘   │
│             │                                                    │
│             ├────────────────────────────────────────────────────┤
│             │                                                    │
│             │              Response Inspector                    │
│             │                                                    │
│             │  [Headers] [Body] [Cookies] [Timing] [TLS]       │
│             │  (Same JSON viewer component)                     │
│             │                                                    │
├─────────────┴────────────────────────────────────────────────────┤
│  ⚫ Recording  │  45 flows  │  12.4 MB  │  🐌 3G  │  ⏸ BP ON   │  ← Status Bar
└──────────────────────────────────────────────────────────────────┘
```

---

## Screen Hierarchy

### 1. Main Window (Always Open)

**Components:**
- **Sidebar** (collapsible, 200-250px)
  - Flow list navigation
  - Quick filters
  - Mock manager
  - Session list
  - Devices
  
- **Traffic List** (main content, 50% height min)
  - All captured flows
  - Sortable columns
  - Context menu
  - Search bar
  
- **Request/Response Inspector** (split, 25% each)
  - Tabbed views
  - JSON viewer
  - Headers, cookies, etc.
  - Timing waterfall
  
- **Toolbar** (top)
  - Record/Stop (⌘R)
  - Clear (⌘K)
  - Throttling dropdown
  - Breakpoints toggle
  - Export menu
  
- **Status Bar** (bottom)
  - Capture status
  - Flow count
  - Storage size
  - Throttling indicator
  - Breakpoints indicator

### 2. Secondary Windows (Floating, Optional)

#### Mock Manager Window
**Purpose:** Manage mock rules, fixtures, validation

```
┌─────────────────────────────────────────────────────────┐
│  Mock Manager                                           │
├─────────────┬───────────────────────────────────────────┤
│             │                                            │
│  Rules      │  Rule: github-user-profile                │
│  ─────      │                                            │
│  [✓] api-1  │  Priority: 100          [✓] Enabled       │
│  [✓] api-2  │  Match:                                   │
│  [ ] api-3  │    Host: api.github.com                   │
│             │    Path: /user                            │
│  Fixtures   │    Method: GET                            │
│  ─────      │                                            │
│  user.json  │  Action: Respond                          │
│  error.json │    Status: 200                            │
│             │    Headers: { content-type: application... │
│  Validation │    Body: [file] user.json                 │
│  ─────      │                                            │
│  ⚠ 2 issues │  [Convert from Flow...] [Validate]       │
│             │                                            │
└─────────────┴───────────────────────────────────────────┘
```

**Keyboard:** ⌘⇧M to open

#### Request Composer Window
**Purpose:** Create new HTTP requests from scratch

```
┌────────────────────────────────────────────────────────┐
│  Request Composer                                       │
├────────────────────────────────────────────────────────┤
│  [POST ▾]  https://api.example.com/users               │
├────────────────────────────────────────────────────────┤
│  Headers                                                │
│  ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━  │
│  Content-Type: application/json                         │
│  Authorization: Bearer [...]                            │
│  + Add Header                                           │
├────────────────────────────────────────────────────────┤
│  Body  [JSON ▾]                                         │
│  ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━  │
│  {                                                       │
│    "name": "Test User",                                 │
│    "email": "test@example.com"                          │
│  }                                                       │
│                                                          │
│  [Import cURL] [Clear] [Send] [Save as Template]       │
└────────────────────────────────────────────────────────┘
```

**Keyboard:** ⌘N for new request

#### Devices Window
**Purpose:** Configure proxy on devices (iOS, Android, Chrome)

```
┌────────────────────────────────────────────────────────┐
│  Device Configuration                                   │
├────────────────────────────────────────────────────────┤
│  macOS System Proxy                                     │
│  ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━  │
│  ⚫ Active on Wi-Fi (127.0.0.1:8899)                    │
│  [Set System Proxy] [Clear]                             │
├────────────────────────────────────────────────────────┤
│  iOS Simulator                                          │
│  ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━  │
│  [Simulator ▾]  iPhone 15 Pro (iOS 17.0)               │
│  [ ] Not configured                                     │
│  [Configure Proxy] [Install Certificate]               │
├────────────────────────────────────────────────────────┤
│  Android Devices (ADB)                                  │
│  ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━  │
│  [Device ▾]  emulator-5554                              │
│  [ ] Not configured                                     │
│  [Configure Proxy] [Install Certificate]               │
├────────────────────────────────────────────────────────┤
│  Browsers                                               │
│  ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━  │
│  [Chrome] [Firefox] [Safari]                            │
│  [Launch Chrome with Proxy]                             │
└────────────────────────────────────────────────────────┘
```

**Keyboard:** ⌘⇧D to open

#### Session Manager
**Purpose:** Save, load, organize capture sessions

```
┌────────────────────────────────────────────────────────┐
│  Sessions                                               │
├─────────────┬──────────────────────────────────────────┤
│             │                                            │
│  Saved      │  Session: API Testing - Oct 17            │
│  ─────      │                                            │
│  API Test   │  Created: 2026-10-17 14:30                │
│  GraphQL    │  Flows: 45                                │
│  Mobile App │  Mocks: 8 rules                           │
│             │  Size: 12.4 MB                            │
│  Active     │                                            │
│  ─────      │  Includes:                                │
│  Current    │  • All flows from run abc123              │
│             │  • 8 dynamic mock rules                   │
│             │  • 3 fixture files                        │
│             │                                            │
│             │  [Load] [Delete] [Export HAR]             │
│             │                                            │
└─────────────┴──────────────────────────────────────────┘
```

**Keyboard:** ⌘⇧S to save, ⌘O to load

### 3. Sheets & Modals (Temporary Overlays)

#### Breakpoint Editor
**Trigger:** Flow hits breakpoint  
**Type:** Modal sheet

```
┌────────────────────────────────────────────────────────┐
│  🛑 Breakpoint Hit                                      │
├────────────────────────────────────────────────────────┤
│  Request to: https://api.github.com/user               │
│                                                          │
│  [Request ▾]  [Response ▾]                              │
│  ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━  │
│  Method:  [GET ▾]                                        │
│  Headers: (editable table)                              │
│  Body:    (editable JSON/text)                          │
│                                                          │
│  [Cancel Request] [Continue] [Edit & Continue]          │
└────────────────────────────────────────────────────────┘
```

#### Export Dialog
**Trigger:** Export button  
**Type:** Save panel with format options

```
┌────────────────────────────────────────────────────────┐
│  Export Flows                                           │
├────────────────────────────────────────────────────────┤
│  Format:  [HAR ▾]  (HAR, JSON, CSV, cURL)              │
│  Scope:   [Current View ▾]  (All, Selected, Host)      │
│  Include: [✓] Request bodies                            │
│           [✓] Response bodies                           │
│           [ ] Binary bodies                             │
│                                                          │
│  [Cancel] [Export...]                                   │
└────────────────────────────────────────────────────────┘
```

#### Clear Confirmation
**Trigger:** Clear button  
**Type:** Alert sheet

```
┌────────────────────────────────────────────────────────┐
│  Clear Flows?                                           │
├────────────────────────────────────────────────────────┤
│  This will clear 45 flows from the list.               │
│                                                          │
│  Data remains in storage and can be restored            │
│  by changing filters.                                   │
│                                                          │
│  [Clear by Host...] [Cancel] [Clear All]               │
└────────────────────────────────────────────────────────┘
```

---

## Navigation Patterns

### Primary Navigation (Sidebar)

1. **Flows** (default view)
   - All captured traffic
   - Filter by host, method, status
   - Search bar

2. **Mocks** 
   - Manage mock rules
   - View/edit fixtures
   - Validation results

3. **Sessions**
   - Saved sessions
   - Current session info

4. **Devices**
   - Configure proxy on devices
   - Certificate management

### Secondary Navigation (Windows)

**Window Menu:**
- Mock Manager (⌘⇧M)
- Request Composer (⌘N)
- Devices (⌘⇧D)
- Sessions (⌘O)

### Context Menus

**Traffic List Row:**
- Filter by Host
- Filter by Method
- Filter by Status Class
- Copy as cURL
- Repeat Request
- Convert to Mock
- Delete Flow

**JSON Viewer:**
- Copy Path
- Copy Value
- Copy JSON Pointer
- Expand All
- Collapse All
- Format JSON
- Save as Fixture

---

## Keyboard Shortcuts

### Global
- `⌘R` - Start/Stop Recording
- `⌘K` - Clear Flows
- `⌘F` - Search
- `⌘N` - New Request (Composer)
- `⌘O` - Open Session
- `⌘S` - Save Session
- `⌘W` - Close Window
- `⌘,` - Preferences

### Windows
- `⌘⇧M` - Mock Manager
- `⌘⇧D` - Devices
- `⌘⇧E` - Export
- `⌘⇧B` - Toggle Breakpoints
- `⌘⇧T` - Throttling Settings

### Traffic List
- `↑/↓` - Navigate flows
- `⌘A` - Select all
- `⌫` - Delete selected
- `Space` - Quick preview

### Inspector
- `⌘1-5` - Switch tabs
- `⌘C` - Copy (context-aware)
- `⌘⌥C` - Copy Path (JSON)

---

## Split View Sizes

### Default Proportions
- Sidebar: 200px (min 150px, max 350px)
- Traffic List: 50% of remaining height
- Request Inspector: 25% of remaining height
- Response Inspector: 25% of remaining height

### Collapsible
- Sidebar can collapse to icon bar (40px)
- Inspectors can be minimized to headers only
- All splits are user-resizable and remembered

---

## Window States

### Main Window
- **Minimum size:** 980×600
- **Default size:** 1280×800
- **Full screen:** Supported
- **Multi-window:** Only one main window

### Secondary Windows
- **Floating:** Always on top of main window
- **Resizable:** Yes, with minimum sizes
- **Remembered:** Position and size saved
- **Multi-instance:** One per type

---

## Next Steps

1. Design JSON Viewer (see 03-json-viewer-spec.md)
2. Design Component Inventory (see 04-component-inventory.md)
3. Create Implementation Plan (see 05-implementation-plan.md)
