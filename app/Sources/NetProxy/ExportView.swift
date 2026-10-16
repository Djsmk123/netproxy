import SwiftUI
import NetProxyKit
import UniformTypeIdentifiers

struct ExportView: View {
    @Environment(\.dismiss) private var dismiss
    @Environment(CaptureModel.self) private var model
    @State private var exportFormat: ExportFormat = .har
    @State private var includeAllFlows = true
    @State private var isExporting = false
    @State private var errorMessage: String?

    enum ExportFormat: String, CaseIterable {
        case har = "HAR (HTTP Archive)"
        case json = "JSON"
        case csv = "CSV"

        var fileExtension: String {
            switch self {
            case .har: return "har"
            case .json: return "json"
            case .csv: return "csv"
            }
        }
    }

    var body: some View {
        VStack(spacing: 0) {
            // Header
            HStack {
                Text("Export Data").font(.title2).fontWeight(.semibold)
                Spacer()
                Button("Cancel") { dismiss() }
            }
            .padding()

            Divider()

            // Content
            Form {
                Section {
                    Picker("Format", selection: $exportFormat) {
                        ForEach(ExportFormat.allCases, id: \.self) { format in
                            Text(format.rawValue).tag(format)
                        }
                    }
                    .pickerStyle(.radioGroup)
                } header: {
                    Text("Export Format")
                }

                Section {
                    Toggle("Include all flows", isOn: $includeAllFlows)
                    if !includeAllFlows {
                        Text("Only selected flows will be exported")
                            .font(.caption)
                            .foregroundStyle(.secondary)
                    }
                } header: {
                    Text("Scope")
                }

                Section {
                    HStack {
                        Image(systemName: "doc.text")
                        VStack(alignment: .leading, spacing: 4) {
                            Text("\(model.flows.count) flows")
                                .font(.body)
                            if let storage = model.storage {
                                Text("Total size: \(Format.bytes(Int64(storage.usedBytes)))")
                                    .font(.caption)
                                    .foregroundStyle(.secondary)
                            }
                        }
                        Spacer()
                    }
                } header: {
                    Text("Export Summary")
                }

                if let error = errorMessage {
                    Section {
                        Label(error, systemImage: "exclamationmark.triangle")
                            .foregroundStyle(.red)
                    }
                }

                Section {
                    Label("HAR export requires backend support", systemImage: "info.circle")
                        .font(.caption)
                        .foregroundStyle(.secondary)
                }
            }
            .formStyle(.grouped)
            .scrollContentBackground(.hidden)

            // Footer
            HStack {
                Spacer()
                Button("Export...") {
                    exportData()
                }
                .buttonStyle(.borderedProminent)
                .disabled(isExporting || model.flows.isEmpty)
            }
            .padding()
        }
        .frame(width: 480, height: 460)
    }

    private func exportData() {
        isExporting = true
        errorMessage = nil

        let panel = NSSavePanel()
        panel.allowedContentTypes = [UTType(filenameExtension: exportFormat.fileExtension) ?? .data]
        panel.nameFieldStringValue = "netproxy-export-\(Date().formatted(.iso8601)).\(exportFormat.fileExtension)"

        panel.begin { response in
            guard response == .OK, let url = panel.url else {
                isExporting = false
                return
            }

            Task {
                do {
                    // TODO: Implement actual export based on format
                    // For now, just show a placeholder
                    let placeholder = "Export functionality requires backend API support"
                    try placeholder.write(to: url, atomically: true, encoding: .utf8)

                    await MainActor.run {
                        isExporting = false
                        dismiss()
                    }
                } catch {
                    await MainActor.run {
                        errorMessage = error.localizedDescription
                        isExporting = false
                    }
                }
            }
        }
    }
}
