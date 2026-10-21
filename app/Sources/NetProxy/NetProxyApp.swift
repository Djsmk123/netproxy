import SwiftUI
import AppKit
import NetProxyKit

/// Finds the bundled capture engine (`Contents/Helpers/netproxyd`) or, during development, a cargo build.
enum DaemonLocator {
    static func url() -> URL {
        let fm = FileManager.default
        if let env = ProcessInfo.processInfo.environment["NETPROXY_DAEMON"] { return URL(fileURLWithPath: env) }
        let bundled = Bundle.main.bundleURL.appendingPathComponent("Contents/Helpers/netproxyd")
        if fm.isExecutableFile(atPath: bundled.path) { return bundled }
        // `swift run` from the repo: walk up from the executable looking for target/{release,debug}/netproxyd
        var dir = URL(fileURLWithPath: Bundle.main.executablePath ?? fm.currentDirectoryPath).deletingLastPathComponent()
        for _ in 0..<8 {
            for profile in ["release", "debug"] {
                let c = dir.appendingPathComponent("target/\(profile)/netproxyd")
                if fm.isExecutableFile(atPath: c.path) { return c }
            }
            dir.deleteLastPathComponent()
        }
        return bundled
    }

    static func supervisor() -> DaemonSupervisor {
        let env = ProcessInfo.processInfo.environment
        let data = env["NETPROXY_DATA_DIR"].map { URL(fileURLWithPath: $0) } ?? DaemonSupervisor.defaultDataDir()
        var args: [String] = []
        if env["NETPROXY_CA_IN_FILE"] != nil { args.append("--ca-in-file") }      // dev/headless: skip the Keychain
        if let up = env["NETPROXY_UPSTREAM_CA"] { args += ["--upstream-ca", up] }
        return DaemonSupervisor(daemonURL: url(), dataDir: data, extraArguments: args)
    }
}

@MainActor
final class AppModel {
    static let shared = AppModel()
    let capture = CaptureModel(supervisor: DaemonLocator.supervisor())
}

final class AppDelegate: NSObject, NSApplicationDelegate {
    func applicationDidFinishLaunching(_ n: Notification) {
        NSApp.setActivationPolicy(.regular)
        NSApp.activate(ignoringOtherApps: true)
    }
    func applicationShouldTerminateAfterLastWindowClosed(_ app: NSApplication) -> Bool { true }
    func applicationWillTerminate(_ n: Notification) {
        // The helper also exits on its own when this process disappears; this just makes it prompt.
        MainActor.assumeIsolated { AppModel.shared.capture.supervisor?.stop() }
    }
}

@main
struct NetProxyApp: App {
    @NSApplicationDelegateAdaptor(AppDelegate.self) private var delegate
    @State private var model = AppModel.shared.capture
    @State private var showAbout = false
    @State private var showSettings = false
    @State private var showDeviceSetup = false
    @State private var showMockManager = false
    @State private var showExport = false
    @State private var showImport = false
    @State private var showBreakpoints = false
    @State private var showPausedRequests = false

    var body: some Scene {
        WindowGroup("NetProxy") {
            ContentView()
                .environment(model)
                .frame(minWidth: 980, minHeight: 600)
                .task {
                    await model.bootstrap()
                    // Demo/automation hook: NETPROXY_AUTOSTART=<port> starts capturing on launch.
                    if let p = ProcessInfo.processInfo.environment["NETPROXY_AUTOSTART"], let port = Int(p), case .connected = model.phase {
                        await model.startCapture(port: port)
                    }
                    #if DEBUG
                    DemoHooks.scheduleSnapshot()
                    #endif
                }
                .sheet(isPresented: $showAbout) {
                    AboutView()
                }
                .sheet(isPresented: $showSettings) {
                    SettingsView()
                }
                .sheet(isPresented: $showDeviceSetup) {
                    DeviceSetupView()
                        .environment(model)
                }
                .sheet(isPresented: $showMockManager) {
                    MockManagerView()
                        .environment(model)
                }
                .sheet(isPresented: $showExport) {
                    ExportView()
                        .environment(model)
                }
                .sheet(isPresented: $showImport) {
                    ImportView()
                        .environment(model)
                }
                .sheet(isPresented: $showBreakpoints) {
                    BreakpointsView()
                        .environment(model)
                }
                .sheet(isPresented: $showPausedRequests) {
                    if let client = model.client {
                        PausedRequestsView(client: client)
                    }
                }
        }
        .defaultSize(width: 1280, height: 780)
        .commands {
            CommandGroup(replacing: .appInfo) {
                Button("About NetProxy") { showAbout = true }
            }
            CommandGroup(replacing: .newItem) {}
            CommandMenu("File") {
                Button("Import Session...") { showImport = true }
                    .keyboardShortcut("i", modifiers: [.command, .shift])
            }
            CommandMenu("Edit") {
                Button("Clear Filters") {
                    model.filter = FilterState()
                }
                .keyboardShortcut("l", modifiers: [.command])
                .disabled(!model.filter.isActive)
            }
            CommandMenu("View") {
                Button("Reload") {
                    Task {
                        await model.refreshRuns()
                        await model.reloadFlows()
                    }
                }
                .keyboardShortcut("r", modifiers: [.command, .shift])
                Divider()
                Button("Enter Full Screen") {
                    NSApp.windows.first?.toggleFullScreen(nil)
                }
                .keyboardShortcut("f", modifiers: [.command, .control])
            }
            CommandMenu("Capture") {
                Button(model.status?.isRunning == true ? "Stop Capture" : "Start Capture") {
                    Task { if model.status?.isRunning == true { await model.stopCapture() } else { await model.startCapture(allowDevices: model.listenAllInterfaces) } }
                }.keyboardShortcut("r", modifiers: [.command])
                Divider()
                Button("Clear List") { model.clearView() }.keyboardShortcut("k", modifiers: [.command])
            }
            CommandMenu("Tools") {
                Button("Map Local...") {}
                    .disabled(true)
                Button("Map Remote...") {}
                    .disabled(true)
                Button("Mock Manager...") {
                    showMockManager = true
                }
                .keyboardShortcut("m", modifiers: [.command, .shift])
                Divider()
                Button("Manage Breakpoints...") { showBreakpoints = true }
                    .keyboardShortcut("b", modifiers: [.command, .shift])
                Button("Paused Requests...") { showPausedRequests = true }
                    .keyboardShortcut("p", modifiers: [.command, .shift])
                Divider()
                Button("Export...") { showExport = true }
                    .keyboardShortcut("e", modifiers: [.command, .shift])
                Divider()
                Button("Settings...") { showSettings = true }
                    .keyboardShortcut(",", modifiers: [.command])
            }
            CommandGroup(replacing: .help) {
                Button("NetProxy Help") {
                    if let url = URL(string: "https://github.com/djsmk123/netproxy/wiki") {
                        NSWorkspace.shared.open(url)
                    }
                }
                Button("Device Setup Guide") {
                    showDeviceSetup = true
                }
                Button("Keyboard Shortcuts") {
                    if let url = URL(string: "https://github.com/djsmk123/netproxy/wiki/Shortcuts") {
                        NSWorkspace.shared.open(url)
                    }
                }
                Divider()
                Button("Report Issue") {
                    if let url = URL(string: "https://github.com/djsmk123/netproxy/issues/new") {
                        NSWorkspace.shared.open(url)
                    }
                }
            }
        }
    }
}
