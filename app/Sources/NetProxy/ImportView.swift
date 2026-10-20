import SwiftUI
import NetProxyKit
import UniformTypeIdentifiers

struct ImportView: View {
    @Environment(\.dismiss) private var dismiss
    @Environment(CaptureModel.self) private var model
    @State private var isImporting = false
    @State private var errorMessage: String?
    @State private var selectedFileURL: URL?
    @State private var importSuccess = false
    @State private var importedCount = 0

    var body: some View {
        VStack(spacing: 0) {
            // Header
            HStack {
                Text("Import Session").font(.title2).fontWeight(.semibold)
                Spacer()
                Button("Cancel") { dismiss() }
            }
            .padding()

            Divider()

            // Content
            VStack(spacing: 20) {
                if importSuccess {
                    // Success state
                    VStack(spacing: 12) {
                        Image(systemName: "checkmark.circle.fill")
                            .font(.system(size: 48))
                            .foregroundStyle(.green)

                        Text("Import Successful")
                            .font(.headline)

                        Text("Imported \(importedCount) entries")
                            .font(.subheadline)
                            .foregroundStyle(.secondary)

                        Button("Done") {
                            dismiss()
                        }
                        .buttonStyle(.borderedProminent)
                        .padding(.top)
                    }
                    .frame(maxWidth: .infinity, maxHeight: .infinity)
                } else {
                    // Import state
                    VStack(alignment: .leading, spacing: 16) {
                        Label("Select HAR File", systemImage: "doc.badge.arrow.up")
                            .font(.headline)

                        if let url = selectedFileURL {
                            HStack {
                                Image(systemName: "doc.text")
                                    .foregroundStyle(.blue)
                                Text(url.lastPathComponent)
                                    .lineLimit(1)
                                Spacer()
                                Button("Change") {
                                    selectFile()
                                }
                                .buttonStyle(.borderless)
                            }
                            .padding()
                            .background(.quaternary, in: RoundedRectangle(cornerRadius: 8))
                        } else {
                            Button {
                                selectFile()
                            } label: {
                                HStack {
                                    Image(systemName: "folder")
                                    Text("Choose HAR File...")
                                }
                                .frame(maxWidth: .infinity)
                                .padding()
                                .background(.quaternary, in: RoundedRectangle(cornerRadius: 8))
                            }
                            .buttonStyle(.plain)
                        }

                        Divider()

                        VStack(alignment: .leading, spacing: 8) {
                            Label("Supported Formats", systemImage: "info.circle")
                                .font(.subheadline)
                                .foregroundStyle(.secondary)

                            VStack(alignment: .leading, spacing: 4) {
                                Text("• HAR (HTTP Archive) 1.2")
                                Text("• JSON format with .har or .json extension")
                            }
                            .font(.caption)
                            .foregroundStyle(.secondary)
                        }

                        if let error = errorMessage {
                            Label(error, systemImage: "exclamationmark.triangle")
                                .foregroundStyle(.red)
                                .padding()
                                .background(.red.opacity(0.1), in: RoundedRectangle(cornerRadius: 8))
                        }
                    }
                    .padding()

                    Spacer()

                    // Footer
                    HStack {
                        Spacer()
                        Button("Import") {
                            importFile()
                        }
                        .buttonStyle(.borderedProminent)
                        .disabled(selectedFileURL == nil || isImporting)
                    }
                    .padding()
                }
            }
        }
        .frame(width: 480, height: 400)
    }

    private func selectFile() {
        let panel = NSOpenPanel()
        panel.allowedContentTypes = [
            UTType(filenameExtension: "har") ?? .json,
            .json
        ]
        panel.allowsMultipleSelection = false
        panel.canChooseDirectories = false
        panel.canChooseFiles = true

        panel.begin { response in
            if response == .OK, let url = panel.url {
                selectedFileURL = url
                errorMessage = nil
            }
        }
    }

    private func importFile() {
        guard let url = selectedFileURL else { return }

        isImporting = true
        errorMessage = nil

        Task {
            do {
                guard let client = model.client else {
                    throw ImportError.noClient
                }

                // Read HAR file
                let harData = try Data(contentsOf: url)
                let harString = String(data: harData, encoding: .utf8) ?? ""

                // Validate it's valid JSON
                _ = try JSONSerialization.jsonObject(with: harData)

                // Call import API
                let params: [String: any Sendable] = [
                    "har_json": harString
                ]

                let result: ImportResult = try await client.call("runs.import", params)

                await MainActor.run {
                    importedCount = result.entries
                    importSuccess = true
                    isImporting = false
                }
            } catch let error as ImportError {
                await MainActor.run {
                    errorMessage = error.localizedDescription
                    isImporting = false
                }
            } catch {
                await MainActor.run {
                    errorMessage = error.localizedDescription
                    isImporting = false
                }
            }
        }
    }
}

struct ImportResult: Codable {
    let imported: String
    let entries: Int
    let message: String
}

enum ImportError: LocalizedError {
    case noClient
    case invalidFormat

    var errorDescription: String? {
        switch self {
        case .noClient:
            return "Not connected to proxy engine"
        case .invalidFormat:
            return "Invalid HAR file format"
        }
    }
}
