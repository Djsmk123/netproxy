import Foundation

// Wire model of netproxyd's control API (docs/control-api.md). Keys arrive snake_case and are
// converted by `Wire.decoder`.

public enum Wire {
    public static let decoder: JSONDecoder = {
        let d = JSONDecoder()
        d.keyDecodingStrategy = .convertFromSnakeCase
        return d
    }()
    public static let encoder: JSONEncoder = {
        let e = JSONEncoder()
        e.keyEncodingStrategy = .convertToSnakeCase
        return e
    }()
}

public typealias Headers = [[String]]  // [[name, value]] as serialized by Rust (Vec<(String,String)>)

public struct FlowRow: Codable, Identifiable, Hashable, Sendable {
    public var run: String
    public var flow: UInt64
    public var conn: UInt64?
    public var state: String            // open | completed | failed | aborted
    public var client: String?
    public var method: String?
    public var uri: String?
    public var host: String?
    public var path: String?
    public var httpVersion: String?
    public var startedUs: Int64
    public var endedUs: Int64?
    public var status: Int?
    public var ttfbUs: Int64?
    public var reqBytes: Int64
    public var resBytes: Int64
    public var reqSha256: String?
    public var resSha256: String?
    public var mocked: Bool
    public var rule: String?
    public var failClass: String?
    public var failDetail: String?
    public var firstSeq: UInt64
    public var lastSeq: UInt64

    public var id: String { "\(run)#\(flow)" }
    public var durationUs: Int64? { endedUs.map { $0 - startedUs } }
    public var isSetupPage: Bool { rule == "np-setup" }
}

public struct RunRow: Codable, Identifiable, Hashable, Sendable {
    public var run: String
    public var startedUs: Int64
    public var endedUs: Int64?
    public var engine: String?
    public var listen: String?
    public var status: String           // running | stopped | interrupted
    public var events: Int64
    public var flows: Int64
    public var id: String { run }
}

public struct EngineStatus: Codable, Hashable, Sendable {
    public var state: String            // running | stopped
    public var run: String?
    public var listen: String?
    public var sequenced: UInt64
    public var dropped: UInt64
    public var isRunning: Bool { state == "running" }
}

public struct FlowCursor: Codable, Hashable, Sendable {
    public var startedUs: Int64
    public var flow: UInt64
}

public struct FlowFilter: Codable, Equatable, Sendable {
    public var run: String?
    public var host: String?
    public var method: String?
    public var status: Int?
    public var statusClass: Int?
    public var state: String?
    public var mocked: Bool?
    public var text: String?
    public init(run: String? = nil, host: String? = nil, method: String? = nil, status: Int? = nil,
                statusClass: Int? = nil, state: String? = nil, mocked: Bool? = nil, text: String? = nil) {
        self.run = run; self.host = host; self.method = method; self.status = status
        self.statusClass = statusClass; self.state = state; self.mocked = mocked; self.text = text
    }
}

public struct FlowPage: Codable, Sendable {
    public var flows: [FlowRow]
    public var next: FlowCursor?
}

public struct FlowDetail: Decodable, Sendable {
    public var flow: FlowRow?
    public var events: [Envelope]
}

public struct BodyChunk: Codable, Sendable {
    public var dataB64: String
    public var offset: UInt64
    public var size: UInt64
    public var eof: Bool
    public var evicted: Bool
    public var inline: Bool?
    public var data: Data { Data(base64Encoded: dataB64) ?? Data() }
}

public struct StorageStatus: Codable, Sendable {
    public var usedBytes: UInt64
    public var maxBytes: UInt64?
}

public struct HelloInfo: Codable, Sendable {
    public var apiVersion: Int
    public var schemaVersion: Int
    public var daemonVersion: String
    public var status: EngineStatus
}

// MARK: events

public enum BodyDir: String, Codable, Sendable { case req, res }

public enum ClientTlsClass: String, Codable, Sendable {
    case rejectedUnknownCa = "rejected_unknown_ca"
    case rejectedCertificate = "rejected_certificate"
    case closedDuringHandshake = "closed_during_handshake"
    case other
}

public struct Envelope: Decodable, Sendable {
    public var run: String
    public var seq: UInt64
    public var tsUs: UInt64
    public var kind: String
    public var event: Event
    public var flow: UInt64? { event.flow }

    private enum Keys: String, CodingKey { case run, seq, tsUs, kind }
    public init(from decoder: Decoder) throws {
        let c = try decoder.container(keyedBy: Keys.self)
        run = try c.decode(String.self, forKey: .run)
        seq = try c.decode(UInt64.self, forKey: .seq)
        tsUs = try c.decode(UInt64.self, forKey: .tsUs)
        kind = try c.decode(String.self, forKey: .kind)
        event = try Event.decode(kind: kind, from: decoder)
    }
}

public enum Event: Sendable {
    case engineStarted(listen: String)
    case engineStopped(reason: String)
    case eventsDropped(count: UInt64)
    case connectionOpened(conn: UInt64, client: String, target: String)
    case tlsHello(conn: UInt64, sni: String?, alpn: [String])
    case clientTlsFailed(conn: UInt64?, cls: ClientTlsClass, detail: String)
    case connectionClosed(conn: UInt64, requests: UInt64)
    case requestHead(RequestHead)
    case responseHead(flow: UInt64, status: Int, ttfbUs: UInt64, headers: Headers, mocked: Bool)
    case bodyEnd(BodyEnd)
    case bodyError(flow: UInt64, dir: BodyDir, total: UInt64, error: String)
    case bodyAborted(flow: UInt64, dir: BodyDir, total: UInt64)
    case mockHit(flow: UInt64, rule: String)
    case flowCompleted(flow: UInt64, status: Int?, mocked: Bool)
    case flowFailed(flow: UInt64, cls: String, detail: String)
    case wsMessage(client: String, dir: String, bytes: UInt64)
    case progress(flow: UInt64)         // body_first_byte / body_progress
    case unknown(kind: String)

    public struct RequestHead: Decodable, Sendable {
        public var flow: UInt64
        public var conn: UInt64?
        public var client: String
        public var method: String
        public var uri: String
        public var host: String?
        public var httpVersion: String
        public var headers: Headers
    }
    public struct BodyEnd: Decodable, Sendable {
        public var flow: UInt64
        public var dir: BodyDir
        public var total: UInt64
        public var chunks: UInt64
        public var sha256: String?
        public var previewB64: String
        public var storedBytes: UInt64
        public var truncated: Bool
    }

    public var flow: UInt64? {
        switch self {
        case .requestHead(let r): return r.flow
        case .responseHead(let f, _, _, _, _): return f
        case .bodyEnd(let b): return b.flow
        case .bodyError(let f, _, _, _), .bodyAborted(let f, _, _), .mockHit(let f, _),
             .flowCompleted(let f, _, _), .flowFailed(let f, _, _), .progress(let f): return f
        default: return nil
        }
    }

    // One payload struct per kind, decoded from the same flattened object.
    private struct P: Decodable {
        var listen: String?; var reason: String?; var count: UInt64?
        var conn: UInt64?; var client: String?; var target: String?
        var sni: String?; var alpn: [String]?
        var `class`: String?; var detail: String?
        var requests: UInt64?
        var flow: UInt64?; var status: Int?; var ttfbUs: UInt64?; var headers: Headers?; var mocked: Bool?
        var dir: String?; var total: UInt64?; var error: String?; var rule: String?; var bytes: UInt64?
    }

    static func decode(kind: String, from d: Decoder) throws -> Event {
        switch kind {
        case "request_head": return .requestHead(try RequestHead(from: d))
        case "body_end": return .bodyEnd(try BodyEnd(from: d))
        default: break
        }
        let p = try P(from: d)
        let dir = BodyDir(rawValue: p.dir ?? "res") ?? .res
        switch kind {
        case "engine_started": return .engineStarted(listen: p.listen ?? "")
        case "engine_stopped": return .engineStopped(reason: p.reason ?? "")
        case "events_dropped": return .eventsDropped(count: p.count ?? 0)
        case "connection_opened": return .connectionOpened(conn: p.conn ?? 0, client: p.client ?? "", target: p.target ?? "")
        case "tls_hello": return .tlsHello(conn: p.conn ?? 0, sni: p.sni, alpn: p.alpn ?? [])
        case "client_tls_failed": return .clientTlsFailed(conn: p.conn, cls: ClientTlsClass(rawValue: p.class ?? "") ?? .other, detail: p.detail ?? "")
        case "connection_closed": return .connectionClosed(conn: p.conn ?? 0, requests: p.requests ?? 0)
        case "response_head": return .responseHead(flow: p.flow ?? 0, status: p.status ?? 0, ttfbUs: p.ttfbUs ?? 0, headers: p.headers ?? [], mocked: p.mocked ?? false)
        case "body_error": return .bodyError(flow: p.flow ?? 0, dir: dir, total: p.total ?? 0, error: p.error ?? "")
        case "body_aborted": return .bodyAborted(flow: p.flow ?? 0, dir: dir, total: p.total ?? 0)
        case "mock_hit": return .mockHit(flow: p.flow ?? 0, rule: p.rule ?? "")
        case "flow_completed": return .flowCompleted(flow: p.flow ?? 0, status: p.status, mocked: p.mocked ?? false)
        case "flow_failed": return .flowFailed(flow: p.flow ?? 0, cls: p.class ?? "other", detail: p.detail ?? "")
        case "ws_message": return .wsMessage(client: p.client ?? "", dir: p.dir ?? "", bytes: p.bytes ?? 0)
        case "body_first_byte", "body_progress": return .progress(flow: p.flow ?? 0)
        default: return .unknown(kind: kind)
        }
    }
}
