import XCTest
@testable import NetProxyKit

/// Drives the real netproxyd + harness upstream (build them first: `cargo build -p netproxyd -p np-harness`).
@MainActor
final class IntegrationTests: XCTestCase {
    static let root = URL(fileURLWithPath: #filePath).deletingLastPathComponent().deletingLastPathComponent().deletingLastPathComponent().deletingLastPathComponent()
    static var daemon: URL { root.appendingPathComponent("target/debug/netproxyd") }
    static var harness: URL { root.appendingPathComponent("target/debug/np-harness") }

    var tmp: URL!
    var upstream: Process?
    var supervisor: DaemonSupervisor!
    var model: CaptureModel!
    var upPort = 0
    var proxyCA: URL { tmp.appendingPathComponent("proxy-ca.pem") }

    override func setUp() async throws {
        for b in [Self.daemon, Self.harness] where !FileManager.default.isExecutableFile(atPath: b.path) {
            throw XCTSkip("build the Rust binaries first: cargo build -p netproxyd -p np-harness (\(b.lastPathComponent) missing)")
        }
        tmp = FileManager.default.temporaryDirectory.appendingPathComponent("np-ui-\(UUID().uuidString.prefix(8))")
        try FileManager.default.createDirectory(at: tmp, withIntermediateDirectories: true)
        upPort = Int.random(in: 30000...50000) / 8 * 8
        let up = Process()
        up.executableURL = Self.harness
        up.arguments = ["--port", "\(upPort)", "--ca-out", tmp.appendingPathComponent("up-ca.pem").path]
        up.standardError = FileHandle.nullDevice
        try up.run(); upstream = up
        for _ in 0..<50 where !FileManager.default.fileExists(atPath: tmp.appendingPathComponent("up-ca.pem").path) { try await Task.sleep(nanoseconds: 100_000_000) }
        supervisor = DaemonSupervisor(daemonURL: Self.daemon, dataDir: tmp.appendingPathComponent("data"),
                                      extraArguments: ["--ca-in-file", "--upstream-ca", tmp.appendingPathComponent("up-ca.pem").path])
        model = CaptureModel(supervisor: supervisor)
    }

    override func tearDown() async throws {
        await model?.shutdown()
        upstream?.terminate()
        if let tmp { try? FileManager.default.removeItem(at: tmp) }
    }

    // MARK: helpers

    @discardableResult
    func curl(_ url: String, trustProxyCA: Bool = true, extra: [String] = []) async throws -> (code: String, out: String) {
        let port = model.proxyPort!
        try Data((model.caCertPEM ?? "").utf8).write(to: proxyCA)
        let p = Process(); p.executableURL = URL(fileURLWithPath: "/usr/bin/curl")
        p.arguments = ["-s", "-x", "http://127.0.0.1:\(port)", "-w", "\n%{http_code}", "--max-time", "10"] + (trustProxyCA ? ["--cacert", proxyCA.path] : []) + extra + [url]
        let o = Pipe(); p.standardOutput = o; p.standardError = FileHandle.nullDevice
        try p.run()
        let data = await Task.detached { o.fileHandleForReading.readDataToEndOfFile() }.value
        p.waitUntilExit()
        let s = String(decoding: data, as: UTF8.self)
        let parts = s.components(separatedBy: "\n")
        return (parts.last ?? "", parts.dropLast().joined(separator: "\n"))
    }

    func eventually(_ what: String, timeout: TimeInterval = 10, _ cond: @escaping () -> Bool) async throws {
        let end = Date().addingTimeInterval(timeout)
        while Date() < end { if cond() { return }; try await Task.sleep(nanoseconds: 50_000_000) }
        XCTFail("timed out waiting for: \(what)")
    }

    func url(_ path: String, offset: Int = 0) -> String { "https://localhost:\(upPort + offset)\(path)" }

    func startedModel() async throws {
        await model.bootstrap()
        guard case .connected = model.phase else { return XCTFail("not connected: \(model.phase)") }
        await model.startCapture(port: 0)
        XCTAssertEqual(model.status?.isRunning, true)
        XCTAssertNotNil(model.proxyPort)
    }

    // MARK: tests

    func testLiveFlowsAppearInOrderAndFiltersApply() async throws {
        try await startedModel()
        for _ in 0..<12 { try await curl(url("/small")) }
        try await curl(url("/nope"))
        try await curl(url("/small", offset: 1))          // untrusted upstream => 502 / failed flow
        try await eventually("14 flows in the list, all finished (rows update in place)") {
            self.model.flows.count == 14 && self.model.flows.allSatisfy { $0.state != "open" }
        }
        XCTAssertEqual(model.flows.first?.path, "/small", "newest first")
        XCTAssertEqual(model.flows.filter { $0.state == "failed" }.count, 1)
        XCTAssertEqual(model.flows.filter { $0.status == 404 }.count, 1)

        model.filter.statusClass = 4
        try await eventually("only the 404") { self.model.flows.count == 1 }
        XCTAssertEqual(model.flows[0].status, 404)

        model.filter.statusClass = nil
        model.filter.onlyProblems = true
        try await eventually("problems = 404 + failed") { self.model.flows.count == 2 }

        model.filter = FilterState()
        model.filter.text = "nope"
        try await eventually("text search") { self.model.flows.count == 1 && self.model.flows[0].path == "/nope" }

        // a live flow that does not match the active filter must not flash into the list
        try await curl(url("/small"))
        try await Task.sleep(nanoseconds: 600_000_000)
        XCTAssertEqual(model.flows.count, 1)
        model.filter = FilterState()
        try await eventually("filter cleared => all 15") { self.model.flows.count == 15 }
    }

    func testDetailAndBodiesIncludingLargeAndGzip() async throws {
        try await startedModel()
        try await curl(url("/small"))
        try await curl(url("/big?mb=1"))
        try await curl(url("/echo"), extra: ["-d", #"{"hello":"world"}"#, "-H", "content-type: application/json"])
        try await eventually("3 completed flows") { self.model.flows.count == 3 && self.model.flows.allSatisfy { $0.state == "completed" } }

        let small = model.flows.first { $0.path == "/small" }!
        let d = await model.detail(for: small)
        let kinds = d?.events.map(\.kind) ?? []
        XCTAssertEqual(kinds.first, "request_head")
        XCTAssertTrue(kinds.contains("response_head") && kinds.contains("flow_completed"), "\(kinds)")
        let sb = await model.body(for: small, dir: .res)
        XCTAssertEqual(String(decoding: sb.data, as: UTF8.self), #"{"ok":true,"n":42}"#, "small bodies are served inline")

        let big = model.flows.first { $0.path == "/big" }!
        let bb = await model.body(for: big, dir: .res)
        XCTAssertEqual(bb.data.count, 1 << 20)
        XCTAssertEqual(bb.size, 1 << 20)
        XCTAssertNil(big.resBytes == 1 << 20 ? nil : "size mismatch")

        let echo = model.flows.first { $0.path == "/echo" }!
        let eb = await model.body(for: echo, dir: .req)
        XCTAssertEqual(String(decoding: eb.data, as: UTF8.self), #"{"hello":"world"}"#)
        let decoded = BodyDecoder.decode(eb.data, headers: [["content-type", "application/json"]], truncated: false)
        XCTAssertEqual(decoded.kind, .json)
    }

    func testDiagnosticsExplainWhyAClientCannotBeInspected() async throws {
        try await startedModel()
        // A client that does not trust our CA: curl without --cacert
        let r = try? await curl(url("/small"), trustProxyCA: false)
        XCTAssertNotEqual(r?.code, "200")
        try await eventually("TLS problem recorded") { !self.model.diagnostics.problems.isEmpty }
        let p = model.diagnostics.problems[0]
        XCTAssertEqual(p.host, "localhost")
        XCTAssertTrue([.rejectedUnknownCa, .rejectedCertificate, .closedDuringHandshake].contains(p.lastClass), "\(p.lastClass)")
        XCTAssertTrue(p.clients.contains("127.0.0.1"))

        // and the history is rebuilt from the store after a refresh (e.g. relaunch)
        await model.refreshDiagnostics()
        XCTAssertEqual(model.diagnostics.problems.first?.host, "localhost")
        XCTAssertGreaterThanOrEqual(model.diagnostics.connections, 1)
    }

    func testStopStartKeepsHistoryAndSwitchesRuns() async throws {
        try await startedModel()
        for _ in 0..<3 { try await curl(url("/small")) }
        try await eventually("3 flows") { self.model.flows.count == 3 }
        let firstRun = model.status!.run!
        await model.stopCapture()
        XCTAssertEqual(model.status?.isRunning, false)
        XCTAssertEqual(model.runs.first?.status, "stopped")
        XCTAssertEqual(model.flows.count, 3, "history stays visible after stop")

        await model.startCapture(port: 0)
        XCTAssertNotEqual(model.status?.run, firstRun)
        try await eventually("view switched to the new (empty) run") { self.model.flows.isEmpty }
        try await curl(url("/small"))
        try await eventually("new run has 1 flow") { self.model.flows.count == 1 }
        XCTAssertEqual(model.runs.count, 2)

        model.selectedRun = firstRun                       // browse the old session
        try await eventually("old run shows its 3 flows") { self.model.flows.count == 3 }
        XCTAssertFalse(model.viewingLiveRun)
    }

    func testSupervisorAttachesToRunningDaemonAndDaemonDiesWithItsParent() async throws {
        await model.bootstrap()
        guard case .connected = model.phase else { return XCTFail("\(model.phase)") }
        // second supervisor on the same data dir => attaches (daemon exits 75), same token
        let second = DaemonSupervisor(daemonURL: Self.daemon, dataDir: supervisor.dataDir, extraArguments: ["--ca-in-file"])
        let ep = try await second.start()
        XCTAssertEqual(ep.token, supervisor.endpoint.token)
        let c = ControlClient(socketPath: ep.socketPath, token: ep.token)
        _ = try await c.connect()
        let st: EngineStatus = try await c.call("engine.status")
        XCTAssertEqual(st.state, "stopped")
        await c.close()

        // a helper started on behalf of a parent that dies must exit by itself
        let sleeper = Process(); sleeper.executableURL = URL(fileURLWithPath: "/bin/sleep"); sleeper.arguments = ["60"]
        try sleeper.run()
        let dir = tmp.appendingPathComponent("orphan")
        let d = Process(); d.executableURL = Self.daemon
        d.arguments = ["--data-dir", dir.path, "--ca-in-file", "--parent-pid", "\(sleeper.processIdentifier)"]
        d.standardError = FileHandle.nullDevice
        try d.run()
        try await eventually("orphan daemon up") { FileManager.default.fileExists(atPath: dir.appendingPathComponent("run/control.sock").path) }
        XCTAssertTrue(d.isRunning)
        sleeper.terminate(); sleeper.waitUntilExit()
        try await eventually("daemon exits after parent death", timeout: 8) { !d.isRunning }
        XCTAssertEqual(d.terminationStatus, 0, "clean shutdown")
    }

    func testLosingTheDaemonIsReported() async throws {
        await model.bootstrap()
        guard case .connected = model.phase else { return XCTFail("\(model.phase)") }
        supervisor.stop()
        try await eventually("failed phase after daemon death", timeout: 10) { if case .failed = self.model.phase { return true } else { return false } }
    }
}
