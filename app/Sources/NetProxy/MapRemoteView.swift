import SwiftUI
import NetProxyKit

struct MapRemoteView: View {
    @Environment(\.dismiss) private var dismiss
    @Environment(CaptureModel.self) private var model

    let flow: FlowRow

    @State private var fromPattern: String = ""
    @State private var toURL: String = ""
    @State private var preservePath: Bool = true
    @State private var preserveQuery: Bool = true
    @State private var enabled: Bool = true
    @State private var description: String = ""

    var body: some View {
        VStack(spacing: 0) {
            // Header
            VStack(alignment: .leading, spacing: 8) {
                Text("Map Remote URL")
                    .font(.title2)
                    .fontWeight(.semibold)
                Text("Redirect requests from one URL to another")
                    .font(.caption)
                    .foregroundStyle(.secondary)
            }
            .frame(maxWidth: .infinity, alignment: .leading)
            .padding()
            .background(.regularMaterial)

            ScrollView {
                VStack(alignment: .leading, spacing: 20) {
                    // From Pattern
                    VStack(alignment: .leading, spacing: 6) {
                        Text("From URL Pattern")
                            .font(.headline)
                        TextField("https://api.example.com/*", text: $fromPattern)
                            .textFieldStyle(.roundedBorder)
                        Text("Requests matching this pattern will be redirected")
                            .font(.caption)
                            .foregroundStyle(.secondary)
                    }

                    Divider()

                    // To URL
                    VStack(alignment: .leading, spacing: 6) {
                        Text("To URL")
                            .font(.headline)
                        TextField("https://staging.example.com", text: $toURL)
                            .textFieldStyle(.roundedBorder)
                        Text("Requests will be sent here instead")
                            .font(.caption)
                            .foregroundStyle(.secondary)
                    }

                    Divider()

                    // Options
                    VStack(alignment: .leading, spacing: 12) {
                        Text("Options")
                            .font(.headline)

                        Toggle("Preserve original path", isOn: $preservePath)
                        Text("If enabled, /api/users → staging.example.com/api/users")
                            .font(.caption)
                            .foregroundStyle(.secondary)
                            .padding(.leading, 24)

                        Toggle("Preserve query parameters", isOn: $preserveQuery)
                        Text("If enabled, ?id=123 will be kept in the redirected request")
                            .font(.caption)
                            .foregroundStyle(.secondary)
                            .padding(.leading, 24)
                    }

                    Divider()

                    // Examples
                    VStack(alignment: .leading, spacing: 8) {
                        Text("Example")
                            .font(.headline)

                        GroupBox {
                            VStack(alignment: .leading, spacing: 4) {
                                HStack {
                                    Text("Original:")
                                        .fontWeight(.medium)
                                        .frame(width: 80, alignment: .leading)
                                    Text(flow.uri ?? "https://api.example.com/users/123?detail=true")
                                        .font(.system(.caption, design: .monospaced))
                                }

                                Image(systemName: "arrow.down")
                                    .foregroundStyle(.secondary)
                                    .padding(.leading, 35)

                                HStack {
                                    Text("Redirected:")
                                        .fontWeight(.medium)
                                        .frame(width: 80, alignment: .leading)
                                    Text(buildExampleURL())
                                        .font(.system(.caption, design: .monospaced))
                                        .foregroundStyle(.green)
                                }
                            }
                            .padding(8)
                        }
                    }

                    Divider()

                    // Description
                    VStack(alignment: .leading, spacing: 6) {
                        Text("Description (Optional)")
                            .font(.headline)
                        TextField("Redirect to staging server", text: $description)
                            .textFieldStyle(.roundedBorder)
                    }

                    Divider()

                    // Enable toggle
                    Toggle("Enable this mapping immediately", isOn: $enabled)
                }
                .padding()
            }

            // Footer buttons
            HStack {
                Button("Cancel") {
                    dismiss()
                }
                .keyboardShortcut(.cancelAction)

                Spacer()

                Button("Create Mapping") {
                    createMapping()
                }
                .keyboardShortcut(.defaultAction)
                .disabled(fromPattern.isEmpty || toURL.isEmpty)
            }
            .padding()
            .background(.regularMaterial)
        }
        .frame(width: 650, height: 550)
        .onAppear {
            // Pre-fill from pattern with the flow's URL
            if let uri = flow.uri {
                fromPattern = uri
            } else if let host = flow.host, let path = flow.path {
                fromPattern = "https://\(host)\(path)"
            }
            description = "Map Remote: \(flow.host ?? "unknown")"
        }
    }

    private func buildExampleURL() -> String {
        guard !toURL.isEmpty else { return "Enter a destination URL..." }

        var result = toURL

        // Add path if preservePath is enabled
        if preservePath, let path = flow.path, !path.isEmpty {
            // Remove trailing slash from toURL if present
            if result.hasSuffix("/") {
                result.removeLast()
            }
            // Ensure path starts with /
            if !path.hasPrefix("/") {
                result += "/"
            }
            result += path
        }

        // Add query if preserveQuery is enabled
        if preserveQuery, let uri = flow.uri, let queryStart = uri.firstIndex(of: "?") {
            result += String(uri[queryStart...])
        }

        return result
    }

    private func createMapping() {
        Task {
            let success = await model.createMapRemote(
                fromPattern: fromPattern,
                toURL: toURL,
                preservePath: preservePath,
                preserveQuery: preserveQuery,
                description: description,
                enabled: enabled
            )

            if success {
                print("✅ Map Remote created successfully!")
                print("  From: \(fromPattern)")
                print("  To: \(toURL)")
                dismiss()
            } else {
                print("❌ Failed to create Map Remote rule")
                // Could show an alert here
            }
        }
    }
}
