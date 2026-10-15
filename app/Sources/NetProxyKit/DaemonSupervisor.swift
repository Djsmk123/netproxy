import Foundation

public struct DaemonEndpoint: Sendable {
    public var socketPath: String
    public var token: String
}

public enum DaemonError: Error, LocalizedError {
    case binaryMissing(String)
    case exitedEarly(Int32, String)
    case timeout

    public var errorDescription: String? {
        switch self {
        case .binaryMissing(let p): return "The capture engine is missing from the app bundle (\(p))."
        case .exitedEarly(let c, let m): return "The capture engine exited with status \(c). \(m)"
        case .timeout: return "The capture engine did not start in time."
        }
    }
}

/// Starts (or attaches to) the netproxyd helper for a data directory.
///
/// * If a daemon already owns the directory (exit status 75 from a second instance) we attach to it.
/// * The helper is told our pid and exits by itself if we vanish, so a crashed UI cannot leave an
///   unattended proxy running.
public final class DaemonSupervisor: @unchecked Sendable {
    public let daemonURL: URL
    public let dataDir: URL
    public var extraArguments: [String]
    private var process: Process?
    private var stderrTail = LockedBuffer()

    public init(daemonURL: URL, dataDir: URL, extraArguments: [String] = []) {
        self.daemonURL = daemonURL
        self.dataDir = dataDir
        self.extraArguments = extraArguments
    }

    public static func defaultDataDir() -> URL {
        FileManager.default.urls(for: .applicationSupportDirectory, in: .userDomainMask)[0].appendingPathComponent("NetProxy", isDirectory: true)
    }

    public var endpoint: DaemonEndpoint {
        let run = dataDir.appendingPathComponent("run")
        let token = (try? String(contentsOf: run.appendingPathComponent("control.token"), encoding: .utf8)) ?? ""
        // The daemon publishes where its socket really is (it may use a short /tmp path when the data
        // directory path is too long for a Unix socket address).
        let advertised = (try? String(contentsOf: run.appendingPathComponent("control.endpoint"), encoding: .utf8))?.trimmingCharacters(in: .whitespacesAndNewlines)
        let socket = (advertised?.isEmpty == false ? advertised! : run.appendingPathComponent("control.sock").path)
        return DaemonEndpoint(socketPath: socket, token: token.trimmingCharacters(in: .whitespacesAndNewlines))
    }

    /// Returns once the control socket is accepting authenticated connections.
    public func start(timeout: TimeInterval = 15) async throws -> DaemonEndpoint {
        guard FileManager.default.isExecutableFile(atPath: daemonURL.path) else { throw DaemonError.binaryMissing(daemonURL.path) }
        try FileManager.default.createDirectory(at: dataDir, withIntermediateDirectories: true)
        let p = Process()
        p.executableURL = daemonURL
        p.arguments = ["--data-dir", dataDir.path, "--parent-pid", "\(getpid())"] + extraArguments
        let err = Pipe()
        p.standardError = err
        p.standardOutput = FileHandle.nullDevice
        err.fileHandleForReading.readabilityHandler = { [stderrTail] h in
            let d = h.availableData
            if !d.isEmpty { stderrTail.append(d) }
        }
        try p.run()
        process = p

        let deadline = Date().addingTimeInterval(timeout)
        while Date() < deadline {
            if !p.isRunning {
                if p.terminationStatus == 75 {            // another daemon owns this directory: attach
                    process = nil
                    if let ep = await waitForEndpoint(until: deadline) { return ep }
                    throw DaemonError.timeout
                }
                throw DaemonError.exitedEarly(p.terminationStatus, stderrTail.text)
            }
            let ep = endpoint
            if !ep.token.isEmpty, FileManager.default.fileExists(atPath: ep.socketPath), await canConnect(ep) { return ep }
            try await Task.sleep(nanoseconds: 100_000_000)
        }
        stop()
        throw DaemonError.timeout
    }

    private func waitForEndpoint(until deadline: Date) async -> DaemonEndpoint? {
        while Date() < deadline {
            let ep = endpoint
            if !ep.token.isEmpty, await canConnect(ep) { return ep }
            try? await Task.sleep(nanoseconds: 100_000_000)
        }
        return nil
    }

    private func canConnect(_ ep: DaemonEndpoint) async -> Bool {
        let c = ControlClient(socketPath: ep.socketPath, token: ep.token)
        defer { Task { await c.close() } }
        return (try? await c.connect(clientName: "probe")) != nil
    }

    /// Asks the helper to shut down cleanly (SIGTERM), escalating if it lingers.
    public func stop() {
        guard let p = process, p.isRunning else { return }
        p.terminate()
        DispatchQueue.global().asyncAfter(deadline: .now() + 5) { if p.isRunning { kill(p.processIdentifier, SIGKILL) } }
        process = nil
    }
}

final class LockedBuffer: @unchecked Sendable {
    private let lock = NSLock()
    private var data = Data()
    func append(_ d: Data) { lock.lock(); data.append(d); if data.count > 8192 { data = data.suffix(8192) }; lock.unlock() }
    var text: String { lock.lock(); defer { lock.unlock() }; return String(decoding: data, as: UTF8.self) }
}
