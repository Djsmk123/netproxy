import SwiftUI
import NetProxyKit

struct ThrottlingView: View {
    @Environment(\.dismiss) private var dismiss
    @State private var enabled = false
    @State private var preset: ThrottlePreset = .wifi
    @State private var customDownload = 1000
    @State private var customUpload = 500
    @State private var latency = 50

    enum ThrottlePreset: String, CaseIterable {
        case threeG = "3G"
        case fourG = "4G"
        case wifi = "WiFi"
        case custom = "Custom"

        var downloadSpeed: Int {
            switch self {
            case .threeG: return 384
            case .fourG: return 10240
            case .wifi: return 51200
            case .custom: return 0
            }
        }

        var uploadSpeed: Int {
            switch self {
            case .threeG: return 128
            case .fourG: return 5120
            case .wifi: return 25600
            case .custom: return 0
            }
        }

        var typicalLatency: Int {
            switch self {
            case .threeG: return 200
            case .fourG: return 50
            case .wifi: return 10
            case .custom: return 0
            }
        }
    }

    var body: some View {
        VStack(spacing: 0) {
            // Header
            HStack {
                Text("Bandwidth Throttling").font(.title2).fontWeight(.semibold)
                Spacer()
                Button("Done") { dismiss() }.keyboardShortcut(.defaultAction)
            }
            .padding()

            Divider()

            // Content
            Form {
                Section {
                    Toggle("Enable Throttling", isOn: $enabled)
                        .toggleStyle(.switch)
                } header: {
                    Text("Status")
                }

                if enabled {
                    Section {
                        Picker("Preset", selection: $preset) {
                            ForEach(ThrottlePreset.allCases, id: \.self) { p in
                                Text(p.rawValue).tag(p)
                            }
                        }
                        .pickerStyle(.segmented)

                        if preset != .custom {
                            HStack {
                                Image(systemName: "arrow.down.circle")
                                Text("Download")
                                Spacer()
                                Text("\(preset.downloadSpeed) KB/s")
                                    .foregroundStyle(.secondary)
                                    .monospacedDigit()
                            }

                            HStack {
                                Image(systemName: "arrow.up.circle")
                                Text("Upload")
                                Spacer()
                                Text("\(preset.uploadSpeed) KB/s")
                                    .foregroundStyle(.secondary)
                                    .monospacedDigit()
                            }

                            HStack {
                                Image(systemName: "timer")
                                Text("Latency")
                                Spacer()
                                Text("\(preset.typicalLatency) ms")
                                    .foregroundStyle(.secondary)
                                    .monospacedDigit()
                            }
                        }
                    } header: {
                        Text("Network Profile")
                    }

                    if preset == .custom {
                        Section {
                            HStack {
                                Text("Download Speed")
                                Spacer()
                                TextField("KB/s", value: $customDownload, format: .number)
                                    .frame(width: 80)
                                    .multilineTextAlignment(.trailing)
                                Text("KB/s").foregroundStyle(.secondary)
                            }

                            HStack {
                                Text("Upload Speed")
                                Spacer()
                                TextField("KB/s", value: $customUpload, format: .number)
                                    .frame(width: 80)
                                    .multilineTextAlignment(.trailing)
                                Text("KB/s").foregroundStyle(.secondary)
                            }

                            HStack {
                                Text("Latency")
                                Spacer()
                                TextField("ms", value: $latency, format: .number)
                                    .frame(width: 80)
                                    .multilineTextAlignment(.trailing)
                                Text("ms").foregroundStyle(.secondary)
                            }
                        } header: {
                            Text("Custom Settings")
                        }
                    }

                    Section {
                        Label("Throttling requires backend support", systemImage: "info.circle")
                            .font(.caption)
                            .foregroundStyle(.secondary)
                    }
                }
            }
            .formStyle(.grouped)
            .scrollContentBackground(.hidden)
        }
        .frame(width: 480, height: 420)
    }
}
