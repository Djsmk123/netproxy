# JSON Viewer Interaction Specification

**Purpose:** First-class JSON inspection with professional features matching VS Code/Postman quality.

**Date:** 2026-10-17  
**Status:** Design Phase  
**Priority:** **P0 - Critical for launch**

---

## Design Principles

1. **Performance First** - Handle 10MB+ payloads smoothly
2. **Developer-Friendly** - Copy path, search, navigate like an IDE
3. **Graceful Degradation** - Show invalid JSON with diagnostics
4. **Native Feel** - macOS-native controls, dark mode, accessibility

---

## View Modes

### 1. Tree View (Default)

**Purpose:** Hierarchical navigation with expand/collapse

```
┌────────────────────────────────────────────────────────────────┐
│ [Tree] [Raw] [Table] [Preview]          🔍 Search  [⚙]        │
├────────────────────────────────────────────────────────────────┤
│                                                                 │
│ ▼ root                                           object (5)    │
│   ▼ user                                         object (4)    │
│       login                  "djsmk123"          string        │
│       id                      12345              number        │
│       verified               true                boolean       │
│       bio                    null                null          │
│   ▼ repos                                        array [42]    │
│       ▼ [0]                                      object (3)    │
│           name               "netproxy"          string        │
│           stars              156                 number        │
│           private            false               boolean       │
│       ▼ [1]                                      object (3)    │
│           name               "awesome-flutter"   string        │
│           stars              2.1k                number        │
│           private            false               boolean       │
│       ... 40 more items                                        │
│   created_at                 "2026-10-17T..."    string        │
│   ▶ metadata                                     object (8)    │
│   ▶ permissions                                  object (3)    │
│                                                                 │
│                                                                 │
│ 47 items total │ 5 expanded │ Path: root.repos[0].name         │
└────────────────────────────────────────────────────────────────┘
```

**Features:**
- **Expand/collapse** - Click arrow or key
- **Type indicators** - Color-coded by type
- **Array indexing** - `[0]`, `[1]`, etc.
- **Lazy loading** - Large arrays load on demand
- **Keyboard navigation** - ↑↓ to move, → to expand, ← to collapse
- **Copy path** - Right-click or ⌘⌥C

**Context Menu (Right-click):**
- Copy Path (`user.repos[0].name`)
- Copy JSONPointer (`/user/repos/0/name`)
- Copy Value
- Copy Key
- Expand All Children
- Collapse All Children
- Filter to This Path

**Visual Design:**
```
Colors (Dark Mode):
- String:  #89D185 (green)
- Number:  #4FC1FF (blue)
- Boolean: #C586C0 (purple)
- Null:    #808080 (gray)
- Key:     #FFFFFF (white)

Icons:
- Object:  { } 
- Array:   [ ]
- Expanded: ▼
- Collapsed: ▶
```

### 2. Raw View

**Purpose:** Pretty-printed JSON with syntax highlighting

```
┌────────────────────────────────────────────────────────────────┐
│ [Tree] [Raw] [Table] [Preview]          🔍 Search  [⚙]        │
├────────────────────────────────────────────────────────────────┤
│ 1  {                                                            │
│ 2    "user": {                                                  │
│ 3      "login": "djsmk123",                                     │
│ 4      "id": 12345,                                             │
│ 5      "verified": true,                                        │
│ 6      "bio": null                                              │
│ 7    },                                                         │
│ 8    "repos": [                                                 │
│ 9      {                                                        │
│10        "name": "netproxy",                                    │
│11        "stars": 156,                                          │
│12        "private": false                                       │
│13      },                                                       │
│14      ...                                                      │
│                                                                 │
│ [Format] [Minify] [Validate] [Copy]                           │
│                                                                 │
│ 1,247 lines │ 42.8 KB │ Valid JSON                             │
└────────────────────────────────────────────────────────────────┘
```

**Features:**
- **Line numbers** - For error messages
- **Syntax highlighting** - Same colors as Tree
- **Format/Minify** - Toggle beautification
- **Select all** - ⌘A
- **Find** - ⌘F with highlights
- **Copy** - ⌘C selected text
- **Scroll to line** - ⌘L

**Performance:**
- **Virtualized** - Only render visible lines
- **Lazy syntax** - Highlight on-demand
- **Max 10MB** - Warn for larger, offer to save

### 3. Table View

**Purpose:** Array of objects as spreadsheet

```
┌────────────────────────────────────────────────────────────────┐
│ [Tree] [Raw] [Table] [Preview]          🔍 Search  [⚙]        │
├────────────────────────────────────────────────────────────────┤
│                                                                 │
│ Viewing: repos (42 items)                                       │
│                                                                 │
│ [#]  name              stars    private  created_at            │
│ ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━  │
│  0   netproxy         156      false    2026-10-01T...         │
│  1   awesome-flutter  2.1k     false    2025-03-15T...         │
│  2   flutter-tools    892      true     2024-11-20T...         │
│  3   api-client       45       false    2024-09-10T...         │
│                                                                 │
│                                                                 │
│ [Export CSV] [Sort ▾]                                          │
│                                                                 │
│ 42 rows │ 4 columns                                            │
└────────────────────────────────────────────────────────────────┘
```

**Features:**
- **Auto-detect arrays** - Only shows for array at root
- **Column headers** - Sortable
- **Cell editing** - Not in viewer (read-only)
- **Export CSV** - For analysis
- **Path selection** - Choose which array to table-ify

**When to Show:**
- Root value is array of objects
- OR user selects array in tree and clicks "View as Table"

### 4. Preview Mode

**Purpose:** Formatted display for common data types

```
┌────────────────────────────────────────────────────────────────┐
│ [Tree] [Raw] [Table] [Preview]          🔍 Search  [⚙]        │
├────────────────────────────────────────────────────────────────┤
│                                                                 │
│                    User Profile                                 │
│                                                                 │
│ Login:        djsmk123                                          │
│ ID:           12345                                             │
│ Verified:     ✓ Yes                                             │
│ Bio:          (not set)                                         │
│                                                                 │
│ Repositories: 42 total                                          │
│                                                                 │
│ • netproxy (156 ⭐)                                             │
│ • awesome-flutter (2.1k ⭐)                                     │
│ • flutter-tools (892 ⭐)                                        │
│                                                                 │
│ Created:      October 17, 2026 at 2:30 PM                      │
│                                                                 │
│ [Back to Tree]                                                  │
│                                                                 │
└────────────────────────────────────────────────────────────────┘
```

**Features:**
- **Schema detection** - Recognize common formats (user, repo, etc.)
- **Formatted dates** - Human-readable
- **Formatted numbers** - Commas, K/M suffixes
- **Icons for booleans** - ✓/✗ instead of true/false
- **Not always available** - Falls back to Tree if schema unknown

**Supported Schemas:**
- User profiles (GitHub, etc.)
- Repository lists
- API responses (pagination, metadata)
- Error responses

---

## Search & Filter

### Search Bar

```
┌────────────────────────────────────────────────────────────────┐
│ 🔍 [Search keys and values...          ]  [ 🔍 Keys | Values ] │
├────────────────────────────────────────────────────────────────┤
│ ▼ repos                                        array [3/42]    │
│   ▼ [0]                                        ✨ MATCH        │
│       name               "netproxy"            ✨ MATCH        │
│       stars              156                                    │
│   [1] (collapsed, no match)                                     │
│   ▼ [2]                                        ✨ MATCH        │
│       name               "netproxy-ui"         ✨ MATCH        │
│                                                                 │
│ 3 matches in 2 items                                           │
└────────────────────────────────────────────────────────────────┘
```

**Features:**
- **Search scope** - Keys only, Values only, or Both
- **Case-insensitive** - By default
- **Regex support** - Optional (toggle in settings menu)
- **Highlight matches** - Yellow background
- **Navigate** - ↑↓ to jump between matches
- **Filter view** - Hide non-matching items (toggle)

### Filter to Path

**Trigger:** Right-click on tree node → "Filter to This Path"

**Effect:** Shows only that subtree, with breadcrumb navigation

```
┌────────────────────────────────────────────────────────────────┐
│ root > repos > [0]                           [Clear Filter]    │
├────────────────────────────────────────────────────────────────┤
│ ▼ [0]                                        object (3)        │
│     name               "netproxy"            string            │
│     stars              156                   number            │
│     private            false                 boolean           │
│                                                                 │
└────────────────────────────────────────────────────────────────┘
```

---

## Large Payload Handling

### Size Thresholds

- **< 1 MB**: Render fully, instant
- **1-10 MB**: Lazy tree expansion, warn user
- **> 10 MB**: Offer to save to file instead

### Lazy Expansion

**For arrays > 100 items:**
```
▼ repos                                        array [1,247]
  [0-99]                                       loaded
  [100-199]                                    click to load...
  [200-299]                                    click to load...
  ...
```

### Virtual Scrolling

- **Raw view**: Only render visible lines ±50
- **Tree view**: Only render visible nodes ±20
- **Smooth scrolling**: Native scroll bounce

---

## Invalid JSON Handling

### Error Display

```
┌────────────────────────────────────────────────────────────────┐
│ ⚠️  Invalid JSON                                                │
├────────────────────────────────────────────────────────────────┤
│ Error at line 5, column 12:                                    │
│   Unexpected token '}', expecting ','                          │
│                                                                 │
│ [View Raw] [Try to Parse Anyway]                              │
├────────────────────────────────────────────────────────────────┤
│ 1  {                                                            │
│ 2    "user": {                                                  │
│ 3      "login": "djsmk123",                                     │
│ 4      "id": 12345                                              │
│ 5    }                        ← ERROR HERE                     │
│ 6    "repos": [                                                 │
│ 7      ...                                                      │
│                                                                 │
│ Raw body is still accessible in [Raw] tab                      │
└────────────────────────────────────────────────────────────────┘
```

**Features:**
- **Precise error location** - Line and column
- **Context** - Show surrounding lines
- **Fallback** - Raw view always works
- **Lenient parsing** - Offer to try with relaxed parser (allow trailing commas, comments)

---

## Actions Menu (⚙ Gear Icon)

```
┌─────────────────────────────────┐
│ Format JSON           ⌘⇧F       │
│ Minify JSON           ⌘⇧M       │
│ Validate              ⌘⇧V       │
│ ──────────────────────────────  │
│ Expand All            ⌘⌥→       │
│ Collapse All          ⌘⌥←       │
│ ──────────────────────────────  │
│ Save as Fixture...              │
│ Copy Entire JSON      ⌘C        │
│ Compare with...                 │
│ ──────────────────────────────  │
│ Settings              ⌘,        │
└─────────────────────────────────┘
```

### Save as Fixture

**Dialog:**
```
┌────────────────────────────────────────────────────────────────┐
│ Save JSON as Fixture                                            │
├────────────────────────────────────────────────────────────────┤
│ Filename: [user-response.json                               ]  │
│                                                                 │
│ Location: [./fixtures/                     ] [Choose...]       │
│                                                                 │
│ [✓] Format (pretty-print)                                       │
│ [ ] Minify                                                      │
│                                                                 │
│ [Cancel] [Save]                                                 │
└────────────────────────────────────────────────────────────────┘
```

### Compare

**Trigger:** Actions menu → Compare with...

**Flow:**
1. Open file picker or select another flow
2. Show side-by-side diff
3. Highlight additions (green), deletions (red), changes (yellow)

```
┌────────────────────────┬────────────────────────┐
│ Current Response       │ Previous Response      │
├────────────────────────┼────────────────────────┤
│ {                      │ {                      │
│   "login": "djsmk123", │   "login": "djsmk123", │
│   "id": 12345,         │   "id": 12345,         │
│   "verified": true,    │   "verified": false,   │ ← Changed
│   "repos": 42          │   "repos": 38          │ ← Changed
│                        │   "bio": "old"         │ ← Deleted
│ }                      │ }                      │
└────────────────────────┴────────────────────────┘
```

---

## Component Architecture

### SwiftUI Components

```swift
// Top-level viewer
struct JSONViewer: View {
    let json: String
    @State private var mode: ViewMode = .tree
    @State private var searchQuery: String = ""
    @State private var expandedPaths: Set<String> = []
    
    var body: some View {
        VStack {
            // Mode switcher + search + actions
            JSONViewerToolbar(mode: $mode, search: $searchQuery)
            
            // Content
            switch mode {
            case .tree:
                JSONTreeView(json: parsed, search: searchQuery, expanded: $expandedPaths)
            case .raw:
                JSONRawView(json: json, search: searchQuery)
            case .table:
                JSONTableView(json: parsed)
            case .preview:
                JSONPreviewView(json: parsed)
            }
            
            // Status bar
            JSONViewerStatus(json: parsed, mode: mode)
        }
    }
}

// Tree view with lazy loading
struct JSONTreeView: View {
    let root: JSONNode
    let search: String
    @Binding var expanded: Set<String>
    
    var body: some View {
        ScrollView {
            LazyVStack(alignment: .leading, spacing: 2) {
                ForEach(visibleNodes) { node in
                    JSONTreeRow(node: node, expanded: $expanded)
                }
            }
        }
    }
}

// Individual tree row
struct JSONTreeRow: View {
    let node: JSONNode
    @Binding var expanded: Set<String>
    
    var body: some View {
        HStack {
            // Expand/collapse arrow
            if node.hasChildren {
                Button { toggleExpanded() } label: {
                    Image(systemName: isExpanded ? "chevron.down" : "chevron.right")
                }
            }
            
            // Key
            Text(node.key).foregroundColor(.white)
            
            // Value or type indicator
            if !node.hasChildren {
                Text(node.value).foregroundColor(node.color)
            } else {
                Text(node.typeLabel).foregroundColor(.secondary)
            }
        }
        .contextMenu {
            Button("Copy Path") { copyPath() }
            Button("Copy Value") { copyValue() }
            Divider()
            Button("Expand All") { expandAll() }
            Button("Collapse All") { collapseAll() }
        }
    }
}
```

### Model

```swift
// Parsed JSON node
struct JSONNode: Identifiable {
    let id: String  // Path like "root.user.login"
    let key: String
    let value: String?  // Leaf nodes only
    let type: JSONType
    let children: [JSONNode]?
    let arrayCount: Int?
    
    var hasChildren: Bool { children != nil }
    var isArray: Bool { type == .array }
    var isObject: Bool { type == .object }
    var color: Color {
        switch type {
        case .string: return .green
        case .number: return .blue
        case .boolean: return .purple
        case .null: return .gray
        default: return .primary
        }
    }
}

enum JSONType {
    case object, array, string, number, boolean, null
}

enum ViewMode {
    case tree, raw, table, preview
}
```

---

## Performance Requirements

- **Parse time**: < 100ms for 1MB JSON
- **Render time**: < 50ms for visible nodes
- **Scroll FPS**: 60fps native scroll
- **Search time**: < 200ms for 1MB JSON
- **Memory**: < 2× JSON size in memory

---

## Accessibility

- **VoiceOver**: All tree nodes readable
- **Keyboard**: Full navigation without mouse
- **Contrast**: WCAG AA compliant colors
- **Font size**: Respects system text size

---

## Next Steps

1. Create Component Inventory (see 04-component-inventory.md)
2. Create Implementation Plan (see 05-implementation-plan.md)
3. Build prototype of JSONTreeView
4. Test with large payloads (10MB+)
