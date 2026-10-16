# Reusable SwiftUI Component Inventory

**Purpose:** Define all reusable UI components for consistent, maintainable code.

**Date:** 2026-10-17  
**Status:** Design Phase

---

## Component Categories

1. **Data Display** - Lists, tables, trees
2. **Inspection** - Viewers for different content types
3. **Input** - Forms, filters, search
4. **Navigation** - Sidebar, tabs, breadcrumbs
5. **Feedback** - Toasts, alerts, loading states
6. **Layout** - Split views, panels, sheets

---

## 1. Data Display Components

### FlowListView
**Purpose:** Display list of captured HTTP flows

```swift
struct FlowListView: View {
    @Binding var flows: [Flow]
    @Binding var selection: Flow?
    let onClear: () -> Void
    
    var body: some View {
        Table(flows, selection: $selection) {
            TableColumn("Host") { Text($0.host) }
            TableColumn("Path") { Text($0.path).lineLimit(1) }
            TableColumn("Method") { MethodBadge($0.method) }
            TableColumn("Status") { StatusBadge($0.status) }
            TableColumn("Size") { Text($0.sizeFormatted) }
        }
        .contextMenu {
            FlowContextMenu(flow: selection)
        }
    }
}
```

**Features:**
- Sortable columns
- Multi-selection
- Context menu
- Copy as cURL
- Keyboard navigation

**Usage:**
```swift
FlowListView(
    flows: $captureModel.flows,
    selection: $selectedFlow,
    onClear: { captureModel.clear() }
)
```

### TreeView
**Purpose:** Generic hierarchical data display

```swift
struct TreeView<Item: TreeNode>: View {
    let root: Item
    @Binding var expanded: Set<String>
    @Binding var selection: Item?
    let rowView: (Item) -> any View
    
    var body: some View {
        ScrollView {
            LazyVStack(alignment: .leading, spacing: 2) {
                ForEach(visibleNodes) { node in
                    TreeRow(
                        node: node,
                        level: node.depth,
                        isExpanded: expanded.contains(node.id),
                        onToggle: { toggleExpanded(node) },
                        content: rowView(node)
                    )
                }
            }
        }
    }
}

protocol TreeNode: Identifiable {
    var children: [Self]? { get }
    var depth: Int { get }
    var hasChildren: Bool { get }
}
```

**Features:**
- Lazy loading
- Expand/collapse
- Keyboard navigation (←→ for collapse/expand)
- Generic content

**Usage:**
```swift
TreeView(
    root: jsonRoot,
    expanded: $expandedPaths,
    selection: $selectedNode,
    rowView: { JSONTreeRow(node: $0) }
)
```

---

## 2. Inspection Components

### JSONViewer
**Purpose:** Inspect JSON with multiple view modes

```swift
struct JSONViewer: View {
    let json: String
    @State private var mode: JSONViewMode = .tree
    @State private var searchQuery: String = ""
    @State private var expandedPaths: Set<String> = []
    
    var body: some View {
        VStack(spacing: 0) {
            // Toolbar
            JSONViewerToolbar(
                mode: $mode,
                search: $searchQuery,
                onFormat: formatJSON,
                onCopy: copyJSON
            )
            
            Divider()
            
            // Content
            Group {
                switch mode {
                case .tree:
                    JSONTreeView(json: parsedJSON, search: searchQuery, expanded: $expandedPaths)
                case .raw:
                    JSONRawView(json: json, search: searchQuery)
                case .table:
                    JSONTableView(json: parsedJSON)
                case .preview:
                    JSONPreviewView(json: parsedJSON)
                }
            }
            
            Divider()
            
            // Status
            JSONViewerStatus(json: parsedJSON, mode: mode)
        }
    }
}

enum JSONViewMode {
    case tree, raw, table, preview
}
```

**Subcomponents:**
- `JSONTreeView` - Tree with expand/collapse
- `JSONRawView` - Syntax-highlighted text
- `JSONTableView` - Spreadsheet for arrays
- `JSONPreviewView` - Formatted display

### HTTPHeadersView
**Purpose:** Display HTTP headers as key-value pairs

```swift
struct HTTPHeadersView: View {
    let headers: [(String, String)]
    @State private var searchQuery: String = ""
    
    var body: some View {
        VStack {
            SearchBar(text: $searchQuery, placeholder: "Filter headers...")
            
            List(filteredHeaders, id: \.0) { header in
                HStack {
                    Text(header.0).foregroundColor(.secondary)
                    Spacer()
                    Text(header.1).textSelection(.enabled)
                }
                .contextMenu {
                    Button("Copy Name") { copy(header.0) }
                    Button("Copy Value") { copy(header.1) }
                    Button("Copy Both") { copy("\(header.0): \(header.1)") }
                }
            }
        }
    }
    
    var filteredHeaders: [(String, String)] {
        headers.filter { searchQuery.isEmpty || $0.0.contains(searchQuery) || $0.1.contains(searchQuery) }
    }
}
```

### CookiesView
**Purpose:** Display cookies with expiry, domain, etc.

```swift
struct CookiesView: View {
    let cookies: [Cookie]
    
    var body: some View {
        Table(cookies) {
            TableColumn("Name") { Text($0.name) }
            TableColumn("Value") { Text($0.value).lineLimit(1) }
            TableColumn("Domain") { Text($0.domain) }
            TableColumn("Path") { Text($0.path) }
            TableColumn("Expires") { Text($0.expiresFormatted) }
            TableColumn("Secure") { Image(systemName: $0.secure ? "checkmark" : "xmark") }
            TableColumn("HttpOnly") { Image(systemName: $0.httpOnly ? "checkmark" : "xmark") }
        }
    }
}

struct Cookie {
    let name: String
    let value: String
    let domain: String
    let path: String
    let expires: Date?
    let secure: Bool
    let httpOnly: Bool
}
```

### TimingWaterfallView
**Purpose:** Timing breakdown visualization

```swift
struct TimingWaterfallView: View {
    let timing: FlowTiming
    
    var body: some View {
        VStack(alignment: .leading, spacing: 12) {
            // Total time
            HStack {
                Text("Total").foregroundColor(.secondary)
                Spacer()
                Text(timing.totalFormatted).font(.system(.body, design: .monospaced))
            }
            
            // Waterfall
            GeometryReader { geometry in
                VStack(alignment: .leading, spacing: 4) {
                    TimingBar(phase: "DNS", duration: timing.dns, total: timing.total, width: geometry.size.width)
                    TimingBar(phase: "Connect", duration: timing.connect, total: timing.total, width: geometry.size.width)
                    TimingBar(phase: "TLS", duration: timing.tls, total: timing.total, width: geometry.size.width)
                    TimingBar(phase: "Send", duration: timing.send, total: timing.total, width: geometry.size.width)
                    TimingBar(phase: "Wait", duration: timing.wait, total: timing.total, width: geometry.size.width)
                    TimingBar(phase: "Receive", duration: timing.receive, total: timing.total, width: geometry.size.width)
                }
            }
        }
    }
}

struct TimingBar: View {
    let phase: String
    let duration: TimeInterval
    let total: TimeInterval
    let width: CGFloat
    
    var body: some View {
        HStack(spacing: 8) {
            Text(phase).frame(width: 80, alignment: .leading).foregroundColor(.secondary)
            
            ZStack(alignment: .leading) {
                Rectangle()
                    .fill(Color.gray.opacity(0.2))
                    .frame(height: 16)
                
                Rectangle()
                    .fill(phaseColor)
                    .frame(width: width * (duration / total), height: 16)
            }
            
            Text(formatDuration(duration))
                .font(.system(.caption, design: .monospaced))
                .foregroundColor(.secondary)
                .frame(width: 60, alignment: .trailing)
        }
    }
    
    var phaseColor: Color {
        switch phase {
        case "DNS": return .purple
        case "Connect": return .blue
        case "TLS": return .green
        case "Send": return .orange
        case "Wait": return .red
        case "Receive": return .cyan
        default: return .gray
        }
    }
}
```

---

## 3. Input Components

### SearchBar
**Purpose:** Unified search input with clear button

```swift
struct SearchBar: View {
    @Binding var text: String
    let placeholder: String
    let onSubmit: (() -> Void)?
    
    var body: some View {
        HStack {
            Image(systemName: "magnifyingglass").foregroundColor(.secondary)
            
            TextField(placeholder, text: $text)
                .textFieldStyle(.plain)
                .onSubmit { onSubmit?() }
            
            if !text.isEmpty {
                Button(action: { text = "" }) {
                    Image(systemName: "xmark.circle.fill")
                        .foregroundColor(.secondary)
                }
                .buttonStyle(.plain)
            }
        }
        .padding(8)
        .background(Color.gray.opacity(0.1))
        .cornerRadius(8)
    }
}
```

### FilterPicker
**Purpose:** Quick filter selection

```swift
struct FilterPicker<T: Hashable>: View {
    let title: String
    @Binding var selection: Set<T>
    let options: [T]
    let label: (T) -> String
    
    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            Text(title).font(.caption).foregroundColor(.secondary)
            
            ScrollView {
                VStack(alignment: .leading, spacing: 4) {
                    ForEach(options, id: \.self) { option in
                        Toggle(label(option), isOn: Binding(
                            get: { selection.contains(option) },
                            set: { if $0 { selection.insert(option) } else { selection.remove(option) } }
                        ))
                    }
                }
            }
        }
    }
}
```

---

## 4. Navigation Components

### SidebarItem
**Purpose:** Sidebar navigation items

```swift
struct SidebarItem: View {
    let icon: String
    let title: String
    let badge: Int?
    let isSelected: Bool
    let onTap: () -> Void
    
    var body: some View {
        Button(action: onTap) {
            HStack {
                Image(systemName: icon)
                Text(title)
                Spacer()
                if let badge = badge {
                    Text("\(badge)")
                        .font(.caption)
                        .padding(.horizontal, 6)
                        .padding(.vertical, 2)
                        .background(Color.blue)
                        .foregroundColor(.white)
                        .cornerRadius(10)
                }
            }
            .padding(8)
            .background(isSelected ? Color.blue.opacity(0.2) : Color.clear)
            .cornerRadius(6)
        }
        .buttonStyle(.plain)
    }
}
```

### BreadcrumbNavigation
**Purpose:** Hierarchical path navigation

```swift
struct BreadcrumbNavigation: View {
    let path: [String]
    let onNavigate: (Int) -> Void
    
    var body: some View {
        HStack(spacing: 4) {
            ForEach(path.indices, id: \.self) { index in
                Button(path[index]) {
                    onNavigate(index)
                }
                .buttonStyle(.plain)
                
                if index < path.count - 1 {
                    Image(systemName: "chevron.right")
                        .foregroundColor(.secondary)
                        .font(.caption)
                }
            }
        }
        .padding(8)
        .background(Color.gray.opacity(0.1))
        .cornerRadius(6)
    }
}
```

---

## 5. Feedback Components

### ToastView
**Purpose:** Temporary success/error messages

```swift
struct ToastView: View {
    let message: String
    let type: ToastType
    @Binding var isShowing: Bool
    
    var body: some View {
        HStack {
            Image(systemName: type.icon)
            Text(message)
        }
        .padding()
        .background(type.backgroundColor)
        .foregroundColor(.white)
        .cornerRadius(8)
        .shadow(radius: 4)
        .transition(.move(edge: .top).combined(with: .opacity))
        .onAppear {
            DispatchQueue.main.asyncAfter(deadline: .now() + 3) {
                withAnimation {
                    isShowing = false
                }
            }
        }
    }
}

enum ToastType {
    case success, error, info
    
    var icon: String {
        switch self {
        case .success: return "checkmark.circle.fill"
        case .error: return "xmark.circle.fill"
        case .info: return "info.circle.fill"
        }
    }
    
    var backgroundColor: Color {
        switch self {
        case .success: return .green
        case .error: return .red
        case .info: return .blue
        }
    }
}
```

### LoadingOverlay
**Purpose:** Full-screen loading indicator

```swift
struct LoadingOverlay: View {
    let message: String
    
    var body: some View {
        ZStack {
            Color.black.opacity(0.4)
                .edgesIgnoringSafeArea(.all)
            
            VStack(spacing: 16) {
                ProgressView()
                    .scaleEffect(1.5)
                    .progressViewStyle(.circular)
                
                Text(message)
                    .foregroundColor(.white)
            }
            .padding(40)
            .background(Color.gray.opacity(0.9))
            .cornerRadius(12)
        }
    }
}
```

---

## 6. Layout Components

### ResizableSplitView
**Purpose:** Three-pane resizable layout

```swift
struct ResizableSplitView<Sidebar: View, Content: View, Detail: View>: View {
    @Binding var sidebarWidth: CGFloat
    @Binding var contentHeight: CGFloat
    let sidebar: Sidebar
    let content: Content
    let detail: Detail
    
    var body: some View {
        GeometryReader { geometry in
            HStack(spacing: 0) {
                // Sidebar
                sidebar
                    .frame(width: sidebarWidth)
                
                Divider()
                
                // Content + Detail
                VStack(spacing: 0) {
                    content
                        .frame(height: contentHeight)
                    
                    Divider()
                    
                    detail
                        .frame(height: geometry.size.height - contentHeight - 1)
                }
            }
        }
    }
}
```

---

## 7. Utility Components

### MethodBadge
**Purpose:** HTTP method pill

```swift
struct MethodBadge: View {
    let method: String
    
    var body: some View {
        Text(method)
            .font(.system(.caption, design: .monospaced))
            .padding(.horizontal, 6)
            .padding(.vertical, 2)
            .background(methodColor)
            .foregroundColor(.white)
            .cornerRadius(4)
    }
    
    var methodColor: Color {
        switch method {
        case "GET": return .blue
        case "POST": return .green
        case "PUT": return .orange
        case "DELETE": return .red
        case "PATCH": return .purple
        default: return .gray
        }
    }
}
```

### StatusBadge
**Purpose:** HTTP status code indicator

```swift
struct StatusBadge: View {
    let status: Int
    
    var body: some View {
        Text("\(status)")
            .font(.system(.caption, design: .monospaced))
            .padding(.horizontal, 6)
            .padding(.vertical, 2)
            .background(statusColor)
            .foregroundColor(.white)
            .cornerRadius(4)
    }
    
    var statusColor: Color {
        switch status {
        case 200..<300: return .green
        case 300..<400: return .blue
        case 400..<500: return .orange
        case 500..<600: return .red
        default: return .gray
        }
    }
}
```

---

## Component Library Structure

```
NetProxy/Sources/
├── Views/
│   ├── FlowList/
│   │   ├── FlowListView.swift
│   │   ├── FlowRow.swift
│   │   └── FlowContextMenu.swift
│   ├── Inspector/
│   │   ├── InspectorView.swift
│   │   ├── HeadersView.swift
│   │   ├── CookiesView.swift
│   │   ├── TimingView.swift
│   │   └── JSONViewer/
│   │       ├── JSONViewer.swift
│   │       ├── JSONTreeView.swift
│   │       ├── JSONRawView.swift
│   │       ├── JSONTableView.swift
│   │       └── JSONPreviewView.swift
│   ├── Common/
│   │   ├── SearchBar.swift
│   │   ├── ToastView.swift
│   │   ├── LoadingOverlay.swift
│   │   ├── TreeView.swift
│   │   └── Badges.swift
│   └── Layout/
│       ├── SidebarView.swift
│       ├── ToolbarView.swift
│       ├── StatusBarView.swift
│       └── ResizableSplitView.swift
```

---

## Reusability Checklist

✅ **Generic where possible** - TreeView, FilterPicker  
✅ **Composable** - Small, focused components  
✅ **Themed** - Consistent colors, fonts  
✅ **Accessible** - VoiceOver, keyboard nav  
✅ **Performant** - Lazy loading, virtualization  
✅ **Testable** - Pure View logic, no business logic  

---

## Next Steps

1. Create Implementation Plan (see 05-implementation-plan.md)
2. Set up component playground for testing
3. Build core components first (TreeView, JSONViewer)
