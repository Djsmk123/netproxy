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
                    Label("Export includes timing data and metadata", systemImage: "info.circle")
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
                    guard let client = model.client else {
                        throw ExportError.noClient
                    }

                    let run = model.selectedRun ?? "current"
                    let params: [String: any Sendable] = [
                        "run": run,
                        "format": exportFormat.apiFormat
                    ]
                    let data: Data
                    switch exportFormat {
                    case .json:
                        let result: ExportResult = try await client.call("runs.export", params)
                        let encoder = JSONEncoder()
                        encoder.outputFormatting = [.prettyPrinted, .sortedKeys]
                        data = try encoder.encode(result)
                    case .csv:
                        let result: CSVExportResult = try await client.call("runs.export", params)
                        data = result.csv.data(using: .utf8) ?? Data()
                    case .har:
                        let result: HARExportResult = try await client.call("runs.export", params)
                        let encoder = JSONEncoder()
                        encoder.outputFormatting = [.prettyPrinted, .sortedKeys]
                        data = try encoder.encode(result)
                    }

                    try data.write(to: url, options: .atomic)

                    await MainActor.run {
                        isExporting = false
                        dismiss()
                    }
                } catch let error as ExportError {
                    await MainActor.run {
                        errorMessage = error.localizedDescription
                        isExporting = false
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

struct ExportParams: Encodable {
    let run: String
    let format: String
}

struct ExportResult: Codable {
    let flows: [FlowRow]
    let format: String
    let version: Int
}

struct CSVExportResult: Codable {
    let csv: String
    let format: String
    let rows: Int
}

struct HARExportResult: Codable {
    let log: HARLog
}

struct HARLog: Codable {
    let version: String
    let creator: HARCreator
    let entries: [HAREntry]
}

struct HARCreator: Codable {
    let name: String
    let version: String
}

struct HAREntry: Codable {
    let startedDateTime: String
    let time: Double
    let request: HARRequest
    let response: HARResponse
    let cache: HARCache
    let timings: HARTimings
}

struct HARRequest: Codable {
    let method: String
    let url: String
    let httpVersion: String
    let headers: [String]
    let queryString: [String]
    let cookies: [String]
    let headersSize: Int
    let bodySize: Int64
}

struct HARResponse: Codable {
    let status: Int
    let statusText: String
    let httpVersion: String
    let headers: [String]
    let cookies: [String]
    let content: HARContent
    let redirectURL: String
    let headersSize: Int
    let bodySize: Int64
}

struct HARContent: Codable {
    let size: Int64
    let mimeType: String
}

struct HARCache: Codable {
}

struct HARTimings: Codable {
    let send: Double
    let wait: Double
    let receive: Double
}

enum ExportError: LocalizedError {
    case noClient
    case formatNotSupported(String)

    var errorDescription: String? {
        switch self {
        case .noClient:
            return "Not connected to proxy engine"
        case .formatNotSupported(let format):
            return "\(format) export is not yet implemented"
        }
    }
}

extension ExportView.ExportFormat {
    var apiFormat: String {
        switch self {
        case .har: return "har"
        case .json: return "json"
        case .csv: return "csv"
        }
    }
}
