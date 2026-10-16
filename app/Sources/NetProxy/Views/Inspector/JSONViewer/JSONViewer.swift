import SwiftUI

struct JSONViewer: View {
    let json: String
    @State private var mode: ViewMode = .tree
    @State private var searchQuery: String = ""
    @State private var expandedPaths: Set<String> = ["root"]
    @State private var parseResult: Result<JSONNode, JSONParseError>?

    enum ViewMode: String, CaseIterable, Identifiable {
        case tree = "Tree"
        case raw = "Raw"
        case table = "Table"
        case preview = "Preview"

        var id: String { rawValue }
    }

    var body: some View {
        VStack(spacing: 0) {
            // Toolbar
            toolbar

            Divider()

            // Content
            if let result = parseResult {
                switch result {
                case .success(let root):
                    contentView(for: root)
                case .failure(let error):
                    errorView(error)
                }
            } else {
                ProgressView()
                    .frame(maxWidth: .infinity, maxHeight: .infinity)
            }
        }
        .task {
            parseJSON()
        }
    }

    private var toolbar: some View {
        HStack {
            // Mode switcher
            Picker("", selection: $mode) {
                ForEach(ViewMode.allCases) { mode in
                    Text(mode.rawValue).tag(mode)
                }
            }
            .pickerStyle(.segmented)
            .labelsHidden()
            .frame(width: 280)

            Spacer()

            // Search bar
            SearchBar(text: $searchQuery, placeholder: "Search keys and values...")
                .frame(width: 240)

            // Actions menu
            Menu {
                Button("Format JSON") {
                    formatJSON()
                }

                Button("Minify JSON") {
                    minifyJSON()
                }

                Button("Validate") {
                    validateJSON()
                }

                Divider()

                Button("Expand All") {
                    expandAll()
                }

                Button("Collapse All") {
                    collapseAll()
                }

                Divider()

                Button("Copy Entire JSON") {
                    copyToPasteboard(json)
                }
            } label: {
                Image(systemName: "gear")
            }
            .menuStyle(.borderlessButton)
        }
        .padding(.horizontal, 10)
        .padding(.vertical, 6)
        .background(.bar)
    }

    @ViewBuilder
    private func contentView(for root: JSONNode) -> some View {
        Group {
            switch mode {
            case .tree:
                JSONTreeView(root: root, search: searchQuery, expandedPaths: $expandedPaths)
            case .raw:
                JSONRawView(json: json, search: searchQuery)
            case .table:
                JSONTableView(root: root)
            case .preview:
                JSONPreviewView(root: root)
            }
        }
    }

    private func errorView(_ error: JSONParseError) -> some View {
        VStack(spacing: DesignSystem.Spacing.lg) {
            Image(systemName: "exclamationmark.triangle.fill")
                .font(.system(size: 48))
                .foregroundColor(.orange)

            Text("Invalid JSON")
                .font(.headline)

            Text(error.errorDescription ?? "Unknown error")
                .font(.caption)
                .foregroundColor(.secondary)
                .multilineTextAlignment(.center)

            HStack {
                Button("View Raw") {
                    mode = .raw
                }
                .buttonStyle(.borderedProminent)
            }
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
        .padding()
    }

    // MARK: - Actions

    private func parseJSON() {
        parseResult = JSONParser.parse(json)
    }

    private func formatJSON() {
        guard let data = json.data(using: .utf8),
              let obj = try? JSONSerialization.jsonObject(with: data),
              let formatted = try? JSONSerialization.data(withJSONObject: obj, options: [.prettyPrinted, .sortedKeys]),
              let formattedString = String(data: formatted, encoding: .utf8) else {
            return
        }

        copyToPasteboard(formattedString)
    }

    private func minifyJSON() {
        guard let data = json.data(using: .utf8),
              let obj = try? JSONSerialization.jsonObject(with: data),
              let minified = try? JSONSerialization.data(withJSONObject: obj, options: []),
              let minifiedString = String(data: minified, encoding: .utf8) else {
            return
        }

        copyToPasteboard(minifiedString)
    }

    private func validateJSON() {
        // Validation is already done via parseResult
        // This is a no-op for now
    }

    private func expandAll() {
        guard case .success(let root) = parseResult else { return }
        var paths = Set<String>()
        collectPaths(node: root, into: &paths)
        expandedPaths = paths
    }

    private func collapseAll() {
        expandedPaths = ["root"]
    }

    private func collectPaths(node: JSONNode, into paths: inout Set<String>) {
        paths.insert(node.id)
        if let children = node.children {
            for child in children {
                collectPaths(node: child, into: &paths)
            }
        }
    }

}
