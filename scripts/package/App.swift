// M0 packaging shell: SwiftUI app that embeds and supervises the Rust engine as a helper process.
// `NetProxy --selftest` starts the helper, drives a mocked request through it and exits 0/1.
import SwiftUI
import Foundation

final class Engine: ObservableObject {
    @Published var status = "stopped"
    @Published var flows: [String] = []
    private var proc: Process?
    let port = 8897

    static var helperURL: URL { Bundle.main.bundleURL.appendingPathComponent("Contents/Helpers/netproxyd") }

    func start() -> Bool {
        let p = Process()
        p.executableURL = Engine.helperURL
        let support = FileManager.default.urls(for: .applicationSupportDirectory, in: .userDomainMask)[0].appendingPathComponent("NetProxyM0")
        try? FileManager.default.createDirectory(at: support, withIntermediateDirectories: true)
        p.arguments = ["--port", "\(port)", "--ca-out", support.appendingPathComponent("ca.pem").path]
        let out = Pipe(), err = Pipe()
        p.standardOutput = out; p.standardError = err
        let ready = DispatchSemaphore(value: 0)
        err.fileHandleForReading.readabilityHandler = { h in
            if String(decoding: h.availableData, as: UTF8.self).contains("listening") { ready.signal() }
        }
        var buf = Data()
        out.fileHandleForReading.readabilityHandler = { [weak self] h in
            buf.append(h.availableData)
            while let nl = buf.firstIndex(of: 10) {
                let line = buf.prefix(upTo: nl); buf.removeSubrange(...nl)
                if let o = try? JSONSerialization.jsonObject(with: line) as? [String: Any],
                   o["ev"] as? String == "request_headers", let uri = o["uri"] as? String {
                    DispatchQueue.main.async { self?.flows.append(uri) }
                }
            }
        }
        p.terminationHandler = { [weak self] q in DispatchQueue.main.async { self?.status = "helper exited (\(q.terminationStatus))" } }
        do { try p.run() } catch { status = "launch failed: \(error)"; return false }
        proc = p
        let ok = ready.wait(timeout: .now() + 5) == .success
        status = ok ? "running on 127.0.0.1:\(port)" : "helper did not become ready"
        return ok
    }
    func stop() { proc?.interrupt(); proc = nil }
}

@main struct NetProxyApp: App {
    @StateObject private var engine = Engine()
    init() {
        if CommandLine.arguments.contains("--selftest") { Self.selftest() }
    }
    var body: some Scene {
        WindowGroup {
            VStack(alignment: .leading, spacing: 8) {
                Text("NetProxy M0 shell").font(.headline)
                Text(engine.status).foregroundStyle(.secondary)
                List(engine.flows, id: \.self) { Text($0).font(.system(.caption, design: .monospaced)) }
            }.padding().frame(minWidth: 520, minHeight: 320)
             .onAppear { _ = engine.start() }
        }
    }
    static func selftest() -> Never {
        let e = Engine()
        guard e.start() else { print("SELFTEST FAIL: \(e.status)"); exit(1) }
        // curl, not URLSession: App Transport Security would block the plain-HTTP test URL in-app.
        let c = Process(); c.executableURL = URL(fileURLWithPath: "/usr/bin/curl")
        c.arguments = ["-s", "-x", "http://127.0.0.1:\(e.port)", "--max-time", "5", "http://example.invalid/__np_mock"]
        let pipe = Pipe(); c.standardOutput = pipe
        try? c.run(); c.waitUntilExit()
        let ok = String(decoding: pipe.fileHandleForReading.readDataToEndOfFile(), as: UTF8.self).contains("mocked")
        e.stop()
        print(ok ? "SELFTEST PASS: helper started from signed bundle, mock served through it" : "SELFTEST FAIL: no mocked response")
        exit(ok ? 0 : 1)
    }
}
