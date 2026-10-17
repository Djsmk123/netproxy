import SwiftUI
import AppKit
import NetProxyKit

struct MapLocalView: View {
    @Environment(\.dismiss) private var dismiss
    @Environment(CaptureModel.self) private var model

    let flow: FlowRow

    @State private var urlPattern: String = ""
    @State private var localFilePath: String = ""
    @State private var contentType: String = ""
    @State private var statusCode: Int = 200
    @State private var enabled: Bool = true
    @State private var description: String = ""
    @State private var showFilePicker = false

    var body: some View {
        VStack(spacing: 0) {
            // Header
            VStack(alignment: .leading, spacing: 8) {
                Text("Map Local File")
                    .font(.title2)
                    .fontWeight(.semibold)
                Text("Serve a local file instead of the actual server response")
                    .font(.caption)
                    .foregroundStyle(.secondary)
            }
            .frame(maxWidth: .infinity, alignment: .leading)
            .padding()
            .background(.regularMaterial)

            ScrollView {
                VStack(alignment: .leading, spacing: 20) {
                    // URL Pattern
                    VStack(alignment: .leading, spacing: 6) {
                        Text("URL Pattern")
                            .font(.headline)
                        TextField("https://api.example.com/users/*", text: $urlPattern)
                            .textFieldStyle(.roundedBorder)
                        Text("Use * for wildcards. Example: */api/users/* matches any host and path")
                            .font(.caption)
                            .foregroundStyle(.secondary)
                    }

                    Divider()

                    // Local File
                    VStack(alignment: .leading, spacing: 6) {
                        Text("Local File")
                            .font(.headline)
                        HStack {
                            TextField("Choose a file to serve...", text: $localFilePath)
                                .textFieldStyle(.roundedBorder)
                                .disabled(true)
                            Button("Browse...") {
                                pickLocalFile()
                            }
                        }
                        if !localFilePath.isEmpty {
                            Text("✓ Selected: \(URL(fileURLWithPath: localFilePath).lastPathComponent)")
                                .font(.caption)
                                .foregroundStyle(.green)
                        }
                    }

                    Divider()

                    // Response Settings
                    VStack(alignment: .leading, spacing: 12) {
                        Text("Response Settings")
                            .font(.headline)

                        HStack {
                            Text("Status Code:")
                                .frame(width: 100, alignment: .leading)
                            TextField("200", value: $statusCode, format: .number)
                                .textFieldStyle(.roundedBorder)
                                .frame(width: 80)
                            Text("(200 = OK, 404 = Not Found, etc.)")
                                .font(.caption)
                                .foregroundStyle(.secondary)
                        }

                        HStack {
                            Text("Content-Type:")
                                .frame(width: 100, alignment: .leading)
                            TextField("application/json", text: $contentType)
                                .textFieldStyle(.roundedBorder)
                        }
                    }

                    Divider()

                    // Description
                    VStack(alignment: .leading, spacing: 6) {
                        Text("Description (Optional)")
                            .font(.headline)
                        TextField("Mock user API response", text: $description)
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
                .disabled(urlPattern.isEmpty || localFilePath.isEmpty)
            }
            .padding()
            .background(.regularMaterial)
        }
        .frame(width: 600, height: 500)
        .onAppear {
            // Pre-fill URL pattern from the flow
            urlPattern = flow.uri ?? ""
            if let contentTypeHeader = extractContentType() {
                contentType = contentTypeHeader
            }
            description = "Map Local: \(flow.host ?? "unknown")"
        }
    }

    private func extractContentType() -> String? {
        // Try to guess content type from the flow
        if let path = flow.path?.lowercased() {
            if path.hasSuffix(".json") { return "application/json" }
            if path.hasSuffix(".xml") { return "application/xml" }
            if path.hasSuffix(".html") { return "text/html" }
            if path.hasSuffix(".css") { return "text/css" }
            if path.hasSuffix(".js") { return "application/javascript" }
            if path.hasSuffix(".png") { return "image/png" }
            if path.hasSuffix(".jpg") || path.hasSuffix(".jpeg") { return "image/jpeg" }
        }
        return "application/octet-stream"
    }

    private func pickLocalFile() {
        let panel = NSOpenPanel()
        panel.canChooseFiles = true
        panel.canChooseDirectories = false
        panel.allowsMultipleSelection = false
        panel.message = "Choose a file to serve for this URL pattern"

        panel.begin { response in
            if response == .OK, let url = panel.url {
                localFilePath = url.path

                // Try to infer content type from file extension
                let ext = url.pathExtension.lowercased()
                switch ext {
                case "json": contentType = "application/json"
                case "xml": contentType = "application/xml"
                case "html", "htm": contentType = "text/html"
                case "css": contentType = "text/css"
                case "js": contentType = "application/javascript"
                case "png": contentType = "image/png"
                case "jpg", "jpeg": contentType = "image/jpeg"
                case "gif": contentType = "image/gif"
                case "svg": contentType = "image/svg+xml"
                case "txt": contentType = "text/plain"
                default: break
                }
            }
        }
    }

    private func createMapping() {
        Task {
            // TODO: Implement actual mock creation via Control API
            // For now, just show success and close

            print("Creating Map Local:")
            print("  URL Pattern: \(urlPattern)")
            print("  Local File: \(localFilePath)")
            print("  Content-Type: \(contentType)")
            print("  Status: \(statusCode)")
            print("  Enabled: \(enabled)")

            // TODO: Call model.createMapLocal() or similar

            dismiss()
        }
    }
}
