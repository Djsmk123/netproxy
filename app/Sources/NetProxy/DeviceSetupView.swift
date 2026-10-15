import SwiftUI
import AppKit
import NetProxyKit

struct DeviceSetupView: View {
    @Environment(CaptureModel.self) private var model
    @Environment(\.dismiss) private var dismiss
    @State private var allowDevices = false
    @State private var portText = "8899"
    @State private var platform: Platform = .ios
    @State private var working = false
    @State private var message: String?

    enum Platform: String, CaseIterable, Identifiable { case ios = "iPhone / iPad", android = "Android", sim = "Simulators", mac = "This Mac"; var id: String { rawValue } }

    private var port: Int { Int(portText) ?? 8899 }
    private var running: Bool { model.status?.isRunning == true }
    private var caPath: String { (model.supervisor?.dataDir ?? DaemonSupervisor.defaultDataDir()).appendingPathComponent("ca/ca.pem").path }

    var body: some View {
        VStack(spacing: 0) {
            HStack {
                Text("Connect a device").font(.title2.weight(.semibold))
                Spacer()
                Button("Done") { dismiss() }.keyboardShortcut(.defaultAction)
            }.padding()
            Divider()
            ScrollView {
                VStack(alignment: .leading, spacing: 18) {
                    network
                    certificate
                    steps
                    devicesSeen
                }.padding()
            }
        }
        .frame(width: 640, height: 720)
        .onAppear { allowDevices = model.listenAllInterfaces; portText = String(model.proxyPort ?? 8899) }
    }

    // 1 ─ network

    private var network: some View {
        card("1. Proxy address", icon: "network") {
            Toggle("Let phones and other computers on my network connect", isOn: Binding(get: { allowDevices }, set: { v in allowDevices = v; apply() }))
                .toggleStyle(.switch)
            if allowDevices {
                Label("Anyone on this network can use this proxy while it's open. Use only on networks you trust, and stop capturing when you're done.", systemImage: "exclamationmark.triangle.fill")
                    .font(.caption).foregroundStyle(.orange)
            }
            HStack {
                Text("Port"); TextField("8899", text: $portText).frame(width: 70).textFieldStyle(.roundedBorder)
                    .onSubmit { apply() }
                Button("Apply") { apply() }.disabled(working)
                if working { ProgressView().controlSize(.small) }
            }
            if let message { Text(message).font(.caption).foregroundStyle(.red) }
            if allowDevices {
                ForEach(NetworkInfo.lanAddresses()) { a in
                    addressRow("\(a.ip):\(port)", detail: a.name)
                }
                if NetworkInfo.lanAddresses().isEmpty { Text("No network connection found.").foregroundStyle(.secondary) }
            } else {
                addressRow("127.0.0.1:\(port)", detail: "this Mac only")
            }
            if !running { Label("Capture is stopped. Applying these settings starts it.", systemImage: "info.circle").font(.caption).foregroundStyle(.secondary) }
        }
    }

    // 2 ─ certificate

    private var certificate: some View {
        card("2. Trust the NetProxy certificate", icon: "checkmark.seal") {
            Text("To read HTTPS traffic, each device must trust NetProxy's root certificate. With the device's proxy set to this Mac, open this address in its browser:")
            addressRow("http://np.pro", detail: "download page for iOS, Android and PEM")
            if let pem = model.caCertPEM, let fp = CertInfo.sha256Fingerprint(pem: pem) {
                VStack(alignment: .leading, spacing: 2) {
                    Text("SHA-256 fingerprint — compare it with what the device shows before trusting").font(.caption).foregroundStyle(.secondary)
                    Text(fp).font(.system(.caption2, design: .monospaced)).textSelection(.enabled)
                }
            }
            HStack {
                Button("Export Certificate…") { export() }
                Button("Install on This Mac…") { NSWorkspace.shared.open(URL(fileURLWithPath: caPath)) }
                    .help("Opens Keychain Access; set the certificate to “Always Trust”")
            }
        }
    }

    // 3 ─ per platform steps

    private var steps: some View {
        card("3. Point the device at the proxy", icon: "list.number") {
            Picker("", selection: $platform) { ForEach(Platform.allCases) { Text($0.rawValue).tag($0) } }.pickerStyle(.segmented).labelsHidden()
            switch platform {
            case .ios:
                step("Join the same Wi-Fi as this Mac, then Settings › Wi-Fi › (i) › Configure Proxy › Manual. Enter the address and port above.")
                step("In Safari open http://np.pro and tap “iOS / iPadOS profile”. Allow the download.")
                step("Settings › General › VPN & Device Management › install the NetProxy profile.")
                step("Settings › General › About › Certificate Trust Settings › switch on full trust for NetProxy Root CA.")
                note("Apps that pin their certificates will still refuse the connection; Diagnostics shows which.")
            case .android:
                step("Wi-Fi settings › the network › Proxy › Manual. Enter the address and port above.")
                step("Open http://np.pro in Chrome, download “Android certificate (.crt)”, then Settings › Security › Encryption & credentials › Install a certificate › CA certificate.")
                note("Android 7+ apps ignore user-installed certificates unless they opt in (debug builds with a network security config). Chrome and most browsers work; many release apps will not. NetProxy reports these in Diagnostics.")
            case .sim:
                command("iOS Simulator — trust the certificate", "xcrun simctl keychain booted add-root-cert '\(caPath)'")
                note("The simulator uses this Mac's network, so set this Mac's proxy (or the app's proxy) to 127.0.0.1:\(port).")
                command("Android Emulator — use the proxy", "adb shell settings put global http_proxy 10.0.2.2:\(port)")
                command("Android Emulator — undo", "adb shell settings put global http_proxy :0")
            case .mac:
                step("System Settings › Network › your network › Details… › Proxies. Turn on Web Proxy (HTTP) and Secure Web Proxy (HTTPS) with 127.0.0.1 and port \(port).")
                step("Install the certificate with “Install on This Mac…” and set it to Always Trust.")
                note("Turn the proxies off again when you stop capturing, or this Mac's internet will stop working.")
            }
        }
    }

    // 4 ─ devices seen

    private var devicesSeen: some View {
        let groups = Dictionary(grouping: model.flows.compactMap { r -> (String, FlowRow)? in
            guard let c = r.client?.split(separator: ":").first.map(String.init), c != "127.0.0.1", c != "::1" else { return nil }
            return (c, r)
        }, by: { $0.0 })
        return card("Devices seen in this session", icon: "iphone") {
            if groups.isEmpty {
                Text("None yet. Remote devices appear here as soon as they send a request.").foregroundStyle(.secondary)
            } else {
                ForEach(groups.keys.sorted(), id: \.self) { ip in
                    HStack {
                        Image(systemName: "iphone"); Text(ip).monospaced(); Spacer()
                        Text("\(groups[ip]!.count) requests").foregroundStyle(.secondary)
                    }
                }
            }
        }
    }

    // MARK: pieces

    private func card<C: View>(_ title: String, icon: String, @ViewBuilder _ content: () -> C) -> some View {
        VStack(alignment: .leading, spacing: 10) {
            Label(title, systemImage: icon).font(.headline)
            content()
        }
        .padding(14).frame(maxWidth: .infinity, alignment: .leading)
        .background(.quaternary.opacity(0.4), in: RoundedRectangle(cornerRadius: 10))
    }

    private func addressRow(_ text: String, detail: String) -> some View {
        HStack {
            Text(text).font(.system(.body, design: .monospaced)).textSelection(.enabled)
            Text(detail).font(.caption).foregroundStyle(.secondary)
            Spacer()
            Button { copyToPasteboard(text) } label: { Image(systemName: "doc.on.doc") }.buttonStyle(.borderless).help("Copy")
        }
    }

    private func step(_ s: String) -> some View { Label { Text(s) } icon: { Image(systemName: "chevron.right.circle").foregroundStyle(.secondary) } }
    private func note(_ s: String) -> some View { Label(s, systemImage: "info.circle").font(.caption).foregroundStyle(.secondary) }
    private func command(_ title: String, _ cmd: String) -> some View {
        VStack(alignment: .leading, spacing: 3) {
            Text(title).font(.caption).foregroundStyle(.secondary)
            HStack {
                Text(cmd).font(.system(.caption, design: .monospaced)).textSelection(.enabled).lineLimit(2)
                Spacer()
                Button { copyToPasteboard(cmd) } label: { Image(systemName: "doc.on.doc") }.buttonStyle(.borderless)
            }.padding(8).background(.black.opacity(0.06), in: RoundedRectangle(cornerRadius: 6))
        }
    }

    // MARK: actions

    private func apply() {
        guard (1...65535).contains(port) else { message = "Enter a port between 1 and 65535."; return }
        message = nil; working = true
        Task {
            if running { await model.stopCapture() }
            await model.startCapture(port: port, allowDevices: allowDevices)
            working = false
            if model.status?.isRunning != true { message = model.lastError ?? "Could not start capture on that port." }
        }
    }

    private func export() {
        let panel = NSSavePanel()
        panel.nameFieldStringValue = "NetProxy-CA.pem"
        panel.canCreateDirectories = true
        if panel.runModal() == .OK, let url = panel.url, let pem = model.caCertPEM {
            try? pem.write(to: url, atomically: true, encoding: .utf8)
        }
    }
}
