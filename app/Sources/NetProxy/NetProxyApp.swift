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
        }
        .defaultSize(width: 1280, height: 780)
        .commands {
            CommandGroup(replacing: .newItem) {}
            CommandMenu("Capture") {
                Button(model.status?.isRunning == true ? "Stop Capture" : "Start Capture") {
                    Task { if model.status?.isRunning == true { await model.stopCapture() } else { await model.startCapture() } }
                }.keyboardShortcut("r", modifiers: [.command])
                Button("Clear List") { model.clearView() }.keyboardShortcut("k", modifiers: [.command])
            }
        }
    }
}
