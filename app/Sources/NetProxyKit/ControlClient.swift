import Foundation

public enum ControlError: Error, LocalizedError {
    case notConnected
    case connectFailed(String)
    case closed
    case rpc(code: Int, message: String)
    case malformed(String)

    public var errorDescription: String? {
        switch self {
        case .notConnected: return "Not connected to the capture engine."
        case .connectFailed(let m): return "Cannot reach the capture engine: \(m)"
        case .closed: return "The capture engine connection closed."
        case .rpc(_, let m): return m
        case .malformed(let m): return "Unexpected reply from the capture engine: \(m)"
        }
    }
}

struct RPCErrorBody: Decodable { var code: Int; var message: String }
struct RPCReply<T: Decodable>: Decodable { var result: T?; var error: RPCErrorBody? }

/// A live event delivered for an `events.subscribe` subscription.
public struct EventNotification: Sendable {
    public var subscription: Int
    public var envelope: Envelope
}

/// JSON-RPC 2.0 client for netproxyd over its Unix domain socket (`docs/control-api.md`).
/// Requests are matched to replies by id; `events.subscribe` notifications arrive on `notifications`.
public actor ControlClient {
    private let socketPath: String
    private let token: String
    private var fd: Int32 = -1
    private var nextId = 0
    private var pending: [Int: CheckedContinuation<Data, Error>] = [:]
    private var readTask: Task<Void, Never>?
    private var closed = false

    public nonisolated let notifications: AsyncStream<EventNotification>
    private nonisolated let notify: AsyncStream<EventNotification>.Continuation
    public nonisolated let disconnects: AsyncStream<Void>
    private nonisolated let disconnect: AsyncStream<Void>.Continuation

    public init(socketPath: String, token: String) {
        self.socketPath = socketPath
        self.token = token
        var n: AsyncStream<EventNotification>.Continuation!
        notifications = AsyncStream(bufferingPolicy: .unbounded) { n = $0 }
        notify = n
        var d: AsyncStream<Void>.Continuation!
        disconnects = AsyncStream { d = $0 }
        disconnect = d
    }

    /// Connects and performs the authenticated `hello` handshake.
    public func connect(clientName: String = "NetProxy.app") async throws -> HelloInfo {
        let sock = socket(AF_UNIX, SOCK_STREAM, 0)
        guard sock >= 0 else { throw ControlError.connectFailed(String(cString: strerror(errno))) }
        var addr = sockaddr_un()
        addr.sun_family = sa_family_t(AF_UNIX)
        let bytes = Array(socketPath.utf8CString)
        guard bytes.count <= MemoryLayout.size(ofValue: addr.sun_path) else {
            Darwin.close(sock); throw ControlError.connectFailed("socket path too long")
        }
        withUnsafeMutablePointer(to: &addr.sun_path) {
            $0.withMemoryRebound(to: CChar.self, capacity: bytes.count) { dst in
                for (i, b) in bytes.enumerated() { dst[i] = b }
            }
        }
        let rc = withUnsafePointer(to: &addr) {
            $0.withMemoryRebound(to: sockaddr.self, capacity: 1) {
                Darwin.connect(sock, $0, socklen_t(MemoryLayout<sockaddr_un>.size))
            }
        }
        guard rc == 0 else {
            let m = String(cString: strerror(errno)); Darwin.close(sock); throw ControlError.connectFailed(m)
        }
        var one: Int32 = 1
        setsockopt(sock, SOL_SOCKET, SO_NOSIGPIPE, &one, socklen_t(MemoryLayout<Int32>.size))
        fd = sock
        closed = false
        startReader(fd: sock)
        let hello: HelloInfo = try await call("hello", ["token": token, "api_version": 1, "client": ["name": clientName]] as [String: any Sendable])
        return hello
    }

    public func close() {
        guard !closed else { return }
        closed = true
        if fd >= 0 { shutdown(fd, SHUT_RDWR) }   // unblocks the reader thread
        failAll(ControlError.closed)
    }

    // MARK: calls

    public func call<T: Decodable>(_ method: String, _ params: [String: any Sendable]? = nil, as: T.Type = T.self) async throws -> T {
        let raw = try await send(method, params)
        let reply: RPCReply<T>
        do { reply = try Wire.decoder.decode(RPCReply<T>.self, from: raw) }
        catch { throw ControlError.malformed("\(method): \(error)") }
        if let e = reply.error { throw ControlError.rpc(code: e.code, message: e.message) }
        guard let r = reply.result else { throw ControlError.malformed("\(method): empty result") }
        return r
    }

    /// For methods whose result we ignore (`engine.stop`, `events.unsubscribe`, …).
    public func callVoid(_ method: String, _ params: [String: any Sendable]? = nil) async throws {
        struct OnlyError: Decodable { var error: RPCErrorBody? }
        let raw = try await send(method, params)
        if let e = try? Wire.decoder.decode(OnlyError.self, from: raw).error { throw ControlError.rpc(code: e.code, message: e.message) }
    }

    private func send(_ method: String, _ params: [String: any Sendable]?) async throws -> Data {
        guard fd >= 0, !closed else { throw ControlError.notConnected }
        nextId += 1
        let id = nextId
        var obj: [String: Any] = ["jsonrpc": "2.0", "id": id, "method": method]
        if let params { obj["params"] = params }
        var line = try JSONSerialization.data(withJSONObject: obj)
        line.append(0x0A)
        return try await withCheckedThrowingContinuation { (cont: CheckedContinuation<Data, Error>) in
            pending[id] = cont
            let ok = line.withUnsafeBytes { buf -> Bool in
                var off = 0
                while off < buf.count {
                    let n = Darwin.write(fd, buf.baseAddress! + off, buf.count - off)
                    if n <= 0 { return false }
                    off += n
                }
                return true
            }
            if !ok, let c = pending.removeValue(forKey: id) { c.resume(throwing: ControlError.closed) }
        }
    }

    // MARK: reading

    private func startReader(fd: Int32) {
        let lines = AsyncStream<Data>(bufferingPolicy: .unbounded) { cont in
            let t = Thread {
                var buf = [UInt8](repeating: 0, count: 64 * 1024)
                var acc = Data()
                while true {
                    let n = Darwin.read(fd, &buf, buf.count)
                    if n <= 0 { break }
                    acc.append(contentsOf: buf[0..<n])
                    while let nl = acc.firstIndex(of: 0x0A) {
                        cont.yield(acc[acc.startIndex..<nl])
                        acc.removeSubrange(acc.startIndex...nl)
                    }
                }
                cont.finish()
            }
            t.name = "netproxy-control-reader"
            t.start()
        }
        readTask = Task { [weak self] in
            for await line in lines { await self?.dispatch(Data(line)) }
            await self?.readerEnded()
        }
    }

    private struct Probe: Decodable { var id: Int?; var method: String? }

    private func dispatch(_ line: Data) {
        guard let probe = try? Wire.decoder.decode(Probe.self, from: line) else { return }
        if let id = probe.id {
            pending.removeValue(forKey: id)?.resume(returning: line)
        } else if probe.method == "event" {
            struct N: Decodable { struct P: Decodable { var subscription: Int; var event: Envelope }; var params: P }
            if let n = try? Wire.decoder.decode(N.self, from: line) {
                notify.yield(EventNotification(subscription: n.params.subscription, envelope: n.params.event))
            }
        }
    }

    private func readerEnded() {
        if fd >= 0 { Darwin.close(fd); fd = -1 }
        let wasClosed = closed
        closed = true
        failAll(ControlError.closed)
        if !wasClosed { disconnect.yield(()) }
    }

    private func failAll(_ error: Error) {
        let p = pending
        pending.removeAll()
        for (_, c) in p { c.resume(throwing: error) }
    }
}
