import SwiftUI

struct JSONTreeView: View {
    let root: JSONNode
    let search: String
    @Binding var expandedPaths: Set<String>

    var body: some View {
        ScrollView {
            LazyVStack(alignment: .leading, spacing: 2) {
                ForEach(visibleNodes(root, level: 0)) { node in
                    JSONTreeRow(
                        node: node,
                        level: node.id.components(separatedBy: ".").count - 1,
                        isExpanded: expandedPaths.contains(node.id),
                        search: search,
                        onToggle: {
                            if expandedPaths.contains(node.id) {
                                expandedPaths.remove(node.id)
                            } else {
                                expandedPaths.insert(node.id)
                            }
                        }
                    )
                }
            }
            .padding(DesignSystem.Spacing.sm)
        }
    }

    private func visibleNodes(_ node: JSONNode, level: Int) -> [JSONNode] {
        var result: [JSONNode] = [node]

        if expandedPaths.contains(node.id), let children = node.children {
            // For large arrays, implement lazy loading
            let limit = 100
            let displayChildren = children.count > limit ? Array(children.prefix(limit)) : children

            for child in displayChildren {
                result.append(contentsOf: visibleNodes(child, level: level + 1))
            }

            // Show "load more" indicator if needed
            if children.count > limit {
                // Create a placeholder node for "load more"
                // This would be implemented in a future iteration
            }
        }

        return result
    }
}

struct JSONTreeRow: View {
    let node: JSONNode
    let level: Int
    let isExpanded: Bool
    let search: String
    let onToggle: () -> Void

    var body: some View {
        HStack(spacing: 4) {
            // Indentation
            Color.clear.frame(width: CGFloat(level) * 16)

            // Expand/collapse arrow
            if node.hasChildren {
                Button(action: onToggle) {
                    Image(systemName: isExpanded ? "chevron.down" : "chevron.right")
                        .font(.caption)
                        .foregroundColor(.secondary)
                        .frame(width: 12)
                }
                .buttonStyle(.plain)
            } else {
                Color.clear.frame(width: 12)
            }

            // Key
            Text(node.key)
                .font(DesignSystem.Typography.monospacedCaptionFont)
                .foregroundColor(DesignSystem.Colors.jsonKey)

            // Value or type indicator
            if !node.hasChildren, let value = node.value {
                Text(value)
                    .font(DesignSystem.Typography.monospacedCaptionFont)
                    .foregroundColor(node.color)
                    .textSelection(.enabled)
            } else {
                Text(node.typeLabel)
                    .font(DesignSystem.Typography.monospacedCaptionFont)
                    .foregroundColor(.secondary)
            }

            Spacer()
        }
        .padding(.vertical, 2)
        .background(matchesSearch ? Color.yellow.opacity(0.2) : Color.clear)
        .contextMenu {
            Button("Copy Path") {
                copyToPasteboard(node.id.replacingOccurrences(of: "root.", with: ""))
            }

            Button("Copy JSONPointer") {
                let pointer = "/" + node.id.replacingOccurrences(of: "root.", with: "")
                    .replacingOccurrences(of: ".", with: "/")
                    .replacingOccurrences(of: "[", with: "/")
                    .replacingOccurrences(of: "]", with: "")
                copyToPasteboard(pointer)
            }

            if let value = node.value {
                Button("Copy Value") {
                    copyToPasteboard(value)
                }
            }

            Button("Copy Key") {
                copyToPasteboard(node.key)
            }

            if node.hasChildren {
                Divider()
                Button("Expand All Children") {
                    // This would be implemented to expand all descendants
                }

                Button("Collapse All Children") {
                    // This would be implemented to collapse all descendants
                }
            }
        }
    }

    private var matchesSearch: Bool {
        guard !search.isEmpty else { return false }
        let searchLower = search.lowercased()
        return node.key.lowercased().contains(searchLower) ||
               (node.value?.lowercased().contains(searchLower) ?? false)
    }
}
