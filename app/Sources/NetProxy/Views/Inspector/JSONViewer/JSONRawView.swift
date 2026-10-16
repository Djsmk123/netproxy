import SwiftUI

struct JSONRawView: View {
    let json: String
    let search: String
    @State private var isFormatted: Bool = true

    var body: some View {
        VStack(spacing: 0) {
            // Toolbar
            HStack {
                Spacer()

                Button(isFormatted ? "Minify" : "Format") {
                    isFormatted.toggle()
                }
                .buttonStyle(.link)
                .font(.caption)

                Button("Copy") {
                    copyToPasteboard(displayedJSON)
                }
                .buttonStyle(.link)
                .font(.caption)
            }
            .padding(.horizontal, 10)
            .padding(.vertical, 5)
            .background(.bar)

            Divider()

            // Content
            ScrollView([.vertical, .horizontal]) {
                VStack(alignment: .leading, spacing: 0) {
                    ForEach(Array(lines.enumerated()), id: \.offset) { index, line in
                        HStack(alignment: .top, spacing: 8) {
                            // Line number
                            Text("\(index + 1)")
                                .font(DesignSystem.Typography.monospacedCaptionFont)
                                .foregroundColor(.secondary)
                                .frame(width: 40, alignment: .trailing)

                            // Syntax-highlighted line
                            syntaxHighlightedText(line)
                                .font(DesignSystem.Typography.monospacedCaptionFont)
                                .textSelection(.enabled)
                        }
                        .background(lineMatchesSearch(line) ? Color.yellow.opacity(0.2) : Color.clear)
                    }
                }
                .padding(10)
                .frame(maxWidth: .infinity, alignment: .leading)
            }

            Divider()

            // Status bar
            HStack {
                Text("\(lines.count) lines")
                    .font(.caption)
                    .foregroundColor(.secondary)

                Text("│")
                    .foregroundColor(.secondary)

                Text(formatBytes(json.utf8.count))
                    .font(.caption)
                    .foregroundColor(.secondary)

                Spacer()
            }
            .padding(.horizontal, 10)
            .padding(.vertical, 5)
            .background(.bar)
        }
    }

    private var displayedJSON: String {
        if isFormatted {
            return formatJSON(json) ?? json
        } else {
            return minifyJSON(json) ?? json
        }
    }

    private var lines: [String] {
        displayedJSON.components(separatedBy: .newlines)
    }

    private func syntaxHighlightedText(_ line: String) -> Text {
        // Simple syntax highlighting
        var result = Text("")
        var current = ""
        var inString = false
        var escaped = false

        for char in line {
            if escaped {
                current.append(char)
                escaped = false
                continue
            }

            if char == "\\" && inString {
                escaped = true
                current.append(char)
                continue
            }

            if char == "\"" {
                if inString {
                    current.append(char)
                    result = result + Text(current).foregroundColor(DesignSystem.Colors.jsonString)
                    current = ""
                    inString = false
                } else {
                    if !current.isEmpty {
                        result = result + Text(current)
                        current = ""
                    }
                    current.append(char)
                    inString = true
                }
            } else if inString {
                current.append(char)
            } else {
                // Check for keywords
                if char.isWhitespace || char == "," || char == ":" || char == "{" || char == "}" || char == "[" || char == "]" {
                    if !current.isEmpty {
                        result = result + coloredText(current)
                        current = ""
                    }
                    result = result + Text(String(char))
                } else {
                    current.append(char)
                }
            }
        }

        if !current.isEmpty {
            if inString {
                result = result + Text(current).foregroundColor(DesignSystem.Colors.jsonString)
            } else {
                result = result + coloredText(current)
            }
        }

        return result
    }

    private func coloredText(_ text: String) -> Text {
        let trimmed = text.trimmingCharacters(in: .whitespaces)

        if trimmed == "true" || trimmed == "false" {
            return Text(text).foregroundColor(DesignSystem.Colors.jsonBoolean)
        } else if trimmed == "null" {
            return Text(text).foregroundColor(DesignSystem.Colors.jsonNull)
        } else if Double(trimmed) != nil {
            return Text(text).foregroundColor(DesignSystem.Colors.jsonNumber)
        } else {
            return Text(text)
        }
    }

    private func lineMatchesSearch(_ line: String) -> Bool {
        guard !search.isEmpty else { return false }
        return line.lowercased().contains(search.lowercased())
    }

    private func formatJSON(_ json: String) -> String? {
        guard let data = json.data(using: .utf8) else { return nil }
        guard let obj = try? JSONSerialization.jsonObject(with: data) else { return nil }
        guard let formatted = try? JSONSerialization.data(withJSONObject: obj, options: [.prettyPrinted, .sortedKeys]) else { return nil }
        return String(data: formatted, encoding: .utf8)
    }

    private func minifyJSON(_ json: String) -> String? {
        guard let data = json.data(using: .utf8) else { return nil }
        guard let obj = try? JSONSerialization.jsonObject(with: data) else { return nil }
        guard let minified = try? JSONSerialization.data(withJSONObject: obj, options: []) else { return nil }
        return String(data: minified, encoding: .utf8)
    }

    private func formatBytes(_ bytes: Int) -> String {
        let kb = Double(bytes) / 1024.0
        let mb = kb / 1024.0

        if mb >= 1 {
            return String(format: "%.2f MB", mb)
        } else if kb >= 1 {
            return String(format: "%.2f KB", kb)
        } else {
            return "\(bytes) bytes"
        }
    }
}
