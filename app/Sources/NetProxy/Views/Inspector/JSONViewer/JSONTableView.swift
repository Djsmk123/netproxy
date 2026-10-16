import SwiftUI

struct JSONTableView: View {
    let root: JSONNode
    @State private var sortColumn: String?
    @State private var sortAscending: Bool = true

    var body: some View {
        VStack(spacing: 0) {
            // Header
            HStack {
                if root.isArray, let count = root.arrayCount {
                    Text("Viewing: \(root.key) (\(count) items)")
                        .font(.caption)
                        .foregroundColor(.secondary)
                } else {
                    Text("Table view is only available for arrays")
                        .font(.caption)
                        .foregroundColor(.secondary)
                }

                Spacer()

                Button("Export CSV") {
                    exportToCSV()
                }
                .buttonStyle(.link)
                .font(.caption)
            }
            .padding(.horizontal, 10)
            .padding(.vertical, 5)
            .background(.bar)

            Divider()

            // Content
            if root.isArray, let children = root.children, !children.isEmpty {
                ScrollView([.vertical, .horizontal]) {
                    tableContent(for: children)
                        .padding(10)
                }
            } else {
                ContentUnavailableView(
                    "Table view not available",
                    systemImage: "tablecells",
                    description: Text("Select an array of objects to view as a table")
                )
            }

            Divider()

            // Status
            HStack {
                if let count = root.arrayCount {
                    Text("\(count) rows │ \(columnKeys.count) columns")
                        .font(.caption)
                        .foregroundColor(.secondary)
                }
                Spacer()
            }
            .padding(.horizontal, 10)
            .padding(.vertical, 5)
            .background(.bar)
        }
    }

    @ViewBuilder
    private func tableContent(for items: [JSONNode]) -> some View {
        if items.allSatisfy({ $0.isObject }) {
            VStack(alignment: .leading, spacing: 0) {
                // Header row
                HStack(spacing: 0) {
                    Text("#")
                        .font(.caption.weight(.semibold))
                        .foregroundColor(.secondary)
                        .frame(width: 50, alignment: .leading)
                        .padding(8)

                    ForEach(columnKeys, id: \.self) { key in
                        VStack(spacing: 0) {
                            Button(action: {
                                if sortColumn == key {
                                    sortAscending.toggle()
                                } else {
                                    sortColumn = key
                                    sortAscending = true
                                }
                            }) {
                                HStack {
                                    Text(key)
                                        .font(.caption.weight(.semibold))
                                        .foregroundColor(.secondary)

                                    if sortColumn == key {
                                        Image(systemName: sortAscending ? "chevron.up" : "chevron.down")
                                            .font(.caption2)
                                            .foregroundColor(.secondary)
                                    }
                                }
                            }
                            .buttonStyle(.plain)
                        }
                        .frame(minWidth: 100, alignment: .leading)
                        .padding(8)
                    }
                }
                .background(Color.gray.opacity(0.1))

                Divider()

                // Data rows
                ForEach(Array(items.enumerated()), id: \.offset) { index, item in
                    HStack(spacing: 0) {
                        Text("\(index)")
                            .font(DesignSystem.Typography.monospacedCaptionFont)
                            .foregroundColor(.secondary)
                            .frame(width: 50, alignment: .leading)
                            .padding(8)

                        ForEach(columnKeys, id: \.self) { key in
                            cellContent(for: item, key: key)
                                .frame(minWidth: 100, alignment: .leading)
                                .padding(8)
                        }
                    }

                    if index < items.count - 1 {
                        Divider()
                    }
                }
            }
        } else {
            Text("Array contains non-object items")
                .foregroundColor(.secondary)
        }
    }

    @ViewBuilder
    private func cellContent(for item: JSONNode, key: String) -> some View {
        if let child = item.children?.first(where: { $0.key == key }) {
            if let value = child.value {
                Text(value)
                    .font(DesignSystem.Typography.monospacedCaptionFont)
                    .foregroundColor(child.color)
                    .lineLimit(1)
                    .textSelection(.enabled)
            } else {
                Text(child.typeLabel)
                    .font(DesignSystem.Typography.monospacedCaptionFont)
                    .foregroundColor(.secondary)
                    .lineLimit(1)
            }
        } else {
            Text("—")
                .foregroundColor(.secondary)
        }
    }

    private var columnKeys: [String] {
        guard root.isArray, let children = root.children else { return [] }

        var keys = Set<String>()
        for item in children {
            if let itemChildren = item.children {
                for child in itemChildren {
                    keys.insert(child.key)
                }
            }
        }

        return keys.sorted()
    }

    private func exportToCSV() {
        guard root.isArray, let children = root.children else { return }

        var csv = columnKeys.joined(separator: ",") + "\n"

        for item in children {
            let row = columnKeys.map { key -> String in
                guard let child = item.children?.first(where: { $0.key == key }) else { return "" }
                return child.value?.replacingOccurrences(of: "\"", with: "\"\"") ?? ""
            }
            csv += row.joined(separator: ",") + "\n"
        }

        // Copy to clipboard
        copyToPasteboard(csv)
    }
}
