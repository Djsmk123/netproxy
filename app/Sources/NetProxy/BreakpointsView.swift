import SwiftUI
import NetProxyKit

struct BreakpointsView: View {
    @Environment(\.dismiss) private var dismiss
    @State private var enabled = false
    @State private var breakOnRequest = true
    @State private var breakOnResponse = true
    @State private var hostFilter = ""
    @State private var pathFilter = ""

    var body: some View {
        VStack(spacing: 0) {
            // Header
            HStack {
                Text("Breakpoints").font(.title2).fontWeight(.semibold)
                Spacer()
                Button("Done") { dismiss() }.keyboardShortcut(.defaultAction)
            }
            .padding()

            Divider()

            // Content
            Form {
                Section {
                    Toggle("Enable Breakpoints", isOn: $enabled)
                        .toggleStyle(.switch)
                } header: {
                    Text("Status")
                } footer: {
                    Text("Pause matching requests/responses for inspection and editing")
                }

                if enabled {
                    Section {
                        Toggle("Break on Request", isOn: $breakOnRequest)
                        Toggle("Break on Response", isOn: $breakOnResponse)
                    } header: {
                        Text("Break On")
                    }

                    Section {
                        LabeledContent("Host") {
                            TextField("e.g., api.example.com", text: $hostFilter)
                                .textFieldStyle(.roundedBorder)
                                .frame(width: 240)
                        }

                        LabeledContent("Path contains") {
                            TextField("e.g., /api/", text: $pathFilter)
                                .textFieldStyle(.roundedBorder)
                                .frame(width: 240)
                        }
                    } header: {
                        Text("Filters (Optional)")
                    } footer: {
                        Text("Leave empty to break on all traffic")
                    }

                    Section {
                        Label("Breakpoints require backend support", systemImage: "info.circle")
                            .font(.caption)
                            .foregroundStyle(.secondary)

                        Text("When a request matches, it will pause and show an editor where you can modify headers and body before continuing.")
                            .font(.caption)
                            .foregroundStyle(.secondary)
                    }
                }
            }
            .formStyle(.grouped)
            .scrollContentBackground(.hidden)
        }
        .frame(width: 520, height: 440)
    }
}
