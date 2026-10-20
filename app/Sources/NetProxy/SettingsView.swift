import SwiftUI

struct SettingsView: View {
    @AppStorage("theme") private var theme = "auto"
    @AppStorage("defaultProxyPort") private var defaultProxyPort = 18899
    @AppStorage("autoStartRecording") private var autoStartRecording = true
    @AppStorage("showNotifications") private var showNotifications = false
    @AppStorage("defaultSaveLocation") private var defaultSaveLocation = ""
    @AppStorage("maxSessionSizeMB") private var maxSessionSizeMB = 500
    @AppStorage("autoPruneOldSessions") private var autoPruneOldSessions = false
    @AppStorage("upstreamProxy") private var upstreamProxy = ""
    @AppStorage("logLevel") private var logLevel = "info"

    var body: some View {
        TabView {
            GeneralSettingsView(
                theme: $theme,
                defaultProxyPort: $defaultProxyPort,
                autoStartRecording: $autoStartRecording,
                showNotifications: $showNotifications
            )
            .tabItem {
                Label("General", systemImage: "gear")
            }

            RecordingSettingsView(
                defaultSaveLocation: $defaultSaveLocation,
                maxSessionSizeMB: $maxSessionSizeMB,
                autoPruneOldSessions: $autoPruneOldSessions
            )
            .tabItem {
                Label("Recording", systemImage: "record.circle")
            }

            NetworkSettingsView(
                upstreamProxy: $upstreamProxy
            )
            .tabItem {
                Label("Network", systemImage: "network")
            }

            AdvancedSettingsView(
                logLevel: $logLevel
            )
            .tabItem {
                Label("Advanced", systemImage: "slider.horizontal.3")
            }
        }
        .frame(width: 500, height: 400)
    }
}

struct GeneralSettingsView: View {
    @Binding var theme: String
    @Binding var defaultProxyPort: Int
    @Binding var autoStartRecording: Bool
    @Binding var showNotifications: Bool

    var body: some View {
        Form {
            Section("Appearance") {
                Picker("Theme", selection: $theme) {
                    Text("Auto").tag("auto")
                    Text("Light").tag("light")
                    Text("Dark").tag("dark")
                }
                .pickerStyle(.segmented)
            }

            Section("Proxy") {
                HStack {
                    Text("Default Port")
                    Spacer()
                    TextField("Port", value: $defaultProxyPort, format: .number)
                        .frame(width: 100)
                        .textFieldStyle(.roundedBorder)
                }
            }

            Section("Behavior") {
                Toggle("Auto-start recording on launch", isOn: $autoStartRecording)
                Toggle("Show notifications", isOn: $showNotifications)
            }
        }
        .formStyle(.grouped)
        .padding()
    }
}

struct RecordingSettingsView: View {
    @Binding var defaultSaveLocation: String
    @Binding var maxSessionSizeMB: Int
    @Binding var autoPruneOldSessions: Bool

    var body: some View {
        Form {
            Section("Storage") {
                HStack {
                    Text("Default save location")
                    Spacer()
                    Text(defaultSaveLocation.isEmpty ? "~/Downloads" : defaultSaveLocation)
                        .foregroundStyle(.secondary)
                        .lineLimit(1)
                        .truncationMode(.middle)
                    Button("Choose...") {
                        chooseFolder()
                    }
                }

                HStack {
                    Text("Max session size")
                    Spacer()
                    TextField("MB", value: $maxSessionSizeMB, format: .number)
                        .frame(width: 80)
                        .textFieldStyle(.roundedBorder)
                    Text("MB")
                }
            }

            Section("Maintenance") {
                Toggle("Auto-prune old sessions", isOn: $autoPruneOldSessions)
                    .help("Automatically delete old sessions when storage limit is reached")
            }
        }
        .formStyle(.grouped)
        .padding()
    }

    private func chooseFolder() {
        let panel = NSOpenPanel()
        panel.canChooseFiles = false
        panel.canChooseDirectories = true
        panel.allowsMultipleSelection = false

        if panel.runModal() == .OK, let url = panel.url {
            defaultSaveLocation = url.path
        }
    }
}

struct NetworkSettingsView: View {
    @Binding var upstreamProxy: String

    var body: some View {
        Form {
            Section("Upstream Proxy") {
                HStack {
                    Text("Proxy URL")
                    Spacer()
                    TextField("http://proxy.example.com:8080", text: $upstreamProxy)
                        .frame(width: 300)
                        .textFieldStyle(.roundedBorder)
                }
                Text("Leave empty to connect directly to upstream servers")
                    .font(.caption)
                    .foregroundStyle(.secondary)
            }

            Section("SSL/TLS") {
                Text("Certificate management is handled via macOS Keychain")
                    .font(.caption)
                    .foregroundStyle(.secondary)
            }
        }
        .formStyle(.grouped)
        .padding()
    }
}

struct AdvancedSettingsView: View {
    @Binding var logLevel: String

    var body: some View {
        Form {
            Section("Logging") {
                Picker("Log Level", selection: $logLevel) {
                    Text("Error").tag("error")
                    Text("Warn").tag("warn")
                    Text("Info").tag("info")
                    Text("Debug").tag("debug")
                    Text("Trace").tag("trace")
                }
                .pickerStyle(.segmented)

                Text("Higher log levels may impact performance")
                    .font(.caption)
                    .foregroundStyle(.secondary)
            }

            Section("Database") {
                HStack {
                    Text("Location")
                    Spacer()
                    Text("~/Library/Application Support/NetProxy")
                        .foregroundStyle(.secondary)
                        .font(.caption)
                }

                Button("Open in Finder") {
                    let url = FileManager.default.homeDirectoryForCurrentUser
                        .appendingPathComponent("Library/Application Support/NetProxy")
                    NSWorkspace.shared.open(url)
                }
            }
        }
        .formStyle(.grouped)
        .padding()
    }
}

#Preview {
    SettingsView()
}
