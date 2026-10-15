import Foundation
import Observation

/// Filters applied server-side by `flows.list`.
public struct FilterState: Equatable, Sendable {
    public var text = ""
    public var statusClass: Int?          // 2,3,4,5
    public var method: String?
    public var host: String?
    public var onlyProblems = false       // failed / aborted / 4xx / 5xx
    public var mockedOnly = false
    public var hideSetupPages = true      // np.pro device-setup traffic
    public init() {}

    public var isActive: Bool { self != FilterState() }

    func wire(run: String) -> FlowFilter {
        FlowFilter(run: run, host: host, method: method, status: nil, statusClass: statusClass,
                   state: nil, mocked: mockedOnly ? true : nil, text: text.isEmpty ? nil : text)
    }
}

/// One host that clients are refusing to talk TLS to, aggregated for the diagnostics screen.
public struct TLSProblem: Identifiable, Sendable, Equatable {
    public var id: String { host }
    public var host: String
    public var count: Int
    public var lastClass: ClientTlsClass
    public var lastDetail: String
    public var clients: Set<String>
    public var lastTsUs: UInt64
}

public struct Diagnostics: Sendable, Equatable {
    public var problems: [TLSProblem] = []
    public var droppedEvents: UInt64 = 0
    public var connections = 0
    public var silentConnections = 0      // closed without ever sending a request
}

@MainActor @Observable
public final class CaptureModel {
    public enum Phase: Equatable { case starting, connected, failed(String) }

    public private(set) var phase: Phase = .starting
    public private(set) var status: EngineStatus?
    public private(set) var runs: [RunRow] = []
    public private(set) var flows: [FlowRow] = []
    public private(set) var hasMore = false
    public private(set) var loading = false
    public private(set) var diagnostics = Diagnostics()
    public private(set) var storage: StorageStatus?
    public private(set) var lastError: String?
    public var selectedRun: String? { didSet { if oldValue != selectedRun { Task { await switchRun() } } } }
    public var filter = FilterState() { didSet { if oldValue != filter { scheduleReload() } } }
    public var selection: FlowRow.ID?
    public private(set) var caCertPEM: String?
    public private(set) var listenAllInterfaces = false
    public private(set) var eventsSeen: UInt64 = 0

    public let supervisor: DaemonSupervisor?
    private var client: ControlClient?
    private var cursor: FlowCursor?
    private var subscription: Int?
    private var lastSeq: UInt64 = 0
    private var liveRun: String?
    private var dirty = Set<UInt64>()
    private var pumpTask: Task<Void, Never>?
    private var tickTask: Task<Void, Never>?
    private var reloadTask: Task<Void, Never>?
    private var sniByConn: [UInt64: String] = [:]
    private var targetByConn: [UInt64: String] = [:]
    private var clientByConn: [UInt64: String] = [:]
    private var diagByHost: [String: TLSProblem] = [:]
    private let memoryCap = 5000

    public init(supervisor: DaemonSupervisor?) { self.supervisor = supervisor }

    /// For tests/previews that already have an endpoint.
    public func connect(endpoint: DaemonEndpoint) async {
        phase = .starting
        let c = ControlClient(socketPath: endpoint.socketPath, token: endpoint.token)
        do {
            let hello = try await c.connect()
            client = c
            status = hello.status
            phase = .connected
            caCertPEM = (try? await c.call("ca.get", as: CAReply.self))?.certPem
            await refreshRuns()
            await reloadFlows()
            startPump(c)
            await attachLive()
            watchDisconnect(c)
        } catch {
            phase = .failed(error.localizedDescription)
        }
    }

    public func bootstrap() async {
        guard let supervisor else { return }
        phase = .starting
        do {
            let ep = try await supervisor.start()
            await connect(endpoint: ep)
        } catch {
            phase = .failed(error.localizedDescription)
        }
    }

    public func shutdown() async {
        pumpTask?.cancel(); tickTask?.cancel()
        await client?.close()
        supervisor?.stop()
    }

    private struct CAReply: Decodable { var certPem: String }

    // MARK: engine control

    public func startCapture(port: Int = 8899, allowDevices: Bool = false) async {
        guard let c = client else { return }
        listenAllInterfaces = allowDevices
        do {
            let st: EngineStatus = try await c.call("engine.start", ["listen": "\(allowDevices ? "0.0.0.0" : "127.0.0.1"):\(port)"])
            status = st
            lastError = nil
            await refreshRuns()
            selectedRun = nil
            await switchRun()
        } catch { lastError = error.localizedDescription }
    }

    public func stopCapture() async {
        guard let c = client else { return }
        do {
            try await c.callVoid("engine.stop")
            status = try await c.call("engine.status")
            await refreshRuns()
        } catch { lastError = error.localizedDescription }
    }

    public var proxyPort: Int? { status?.listen.flatMap { Int($0.split(separator: ":").last ?? "") } }
    public var effectiveRun: String? { selectedRun ?? status?.run ?? runs.first?.run }
    public var viewingLiveRun: Bool { status?.isRunning == true && effectiveRun == status?.run }

    // MARK: data

    public func refreshRuns() async {
        guard let c = client else { return }
        if let r: [RunRow] = try? await c.call("runs.list") { runs = r }
        if let s: EngineStatus = try? await c.call("engine.status") { status = s }
        if let st: StorageStatus = try? await c.call("storage.status") { storage = st }
    }

    private func scheduleReload() {
        reloadTask?.cancel()
        reloadTask = Task { [weak self] in
            try? await Task.sleep(nanoseconds: 200_000_000)   // debounce typing
            guard !Task.isCancelled else { return }
            await self?.reloadFlows()
        }
    }

    private func currentFlowFilter(_ run: String) -> FlowFilter {
        var f = filter.wire(run: run)
        if filter.onlyProblems { f.state = nil }
        return f
    }

    public func reloadFlows() async {
        guard let c = client, let run = effectiveRun else { flows = []; hasMore = false; return }
        loading = true; defer { loading = false }
        do {
            let page: FlowPage = try await c.call("flows.list", ["filter": try encodeFilter(currentFlowFilter(run)), "limit": 200] as [String: any Sendable])
            flows = post(page.flows)
            cursor = page.next
            hasMore = page.next != nil
        } catch { lastError = error.localizedDescription }
    }

    public func loadMore() async {
        guard let c = client, let run = effectiveRun, let cur = cursor, !loading else { return }
        loading = true; defer { loading = false }
        do {
            let page: FlowPage = try await c.call("flows.list", ["filter": try encodeFilter(currentFlowFilter(run)), "limit": 200, "cursor": ["started_us": cur.startedUs, "flow": cur.flow]] as [String: any Sendable])
            flows += post(page.flows)
            cursor = page.next
            hasMore = page.next != nil
            if flows.count > memoryCap * 4 { flows = Array(flows.prefix(memoryCap * 4)) }
        } catch { lastError = error.localizedDescription }
    }

    /// Client-side filters the server cannot express.
    private func post(_ rows: [FlowRow]) -> [FlowRow] {
        rows.filter { r in
            if filter.hideSetupPages && r.isSetupPage { return false }
            if filter.onlyProblems {
                let bad = r.state == "failed" || r.state == "aborted" || (r.status ?? 0) >= 400
                if !bad { return false }
            }
            return true
        }
    }

    private func encodeFilter(_ f: FlowFilter) throws -> [String: any Sendable] {
        let data = try Wire.encoder.encode(f)
        let obj = try JSONSerialization.jsonObject(with: data) as? [String: Any] ?? [:]
        return obj as! [String: any Sendable]
    }

    public func detail(for row: FlowRow) async -> FlowDetail? {
        guard let c = client else { return nil }
        return try? await c.call("flows.get", ["run": row.run, "flow": row.flow] as [String: any Sendable])
    }

    /// Reads up to `limit` bytes of a stored body (in 256 KiB ranges).
    public func body(for row: FlowRow, dir: BodyDir, limit: Int = 2 << 20) async -> (data: Data, evicted: Bool, size: UInt64) {
        guard let c = client else { return (Data(), false, 0) }
        var out = Data(), offset: UInt64 = 0, size: UInt64 = 0
        while out.count < limit {
            guard let ch: BodyChunk = try? await c.call("bodies.get", ["run": row.run, "flow": row.flow, "dir": dir.rawValue, "offset": offset, "length": 256 * 1024] as [String: any Sendable]) else { break }
            if ch.evicted { return (Data(), true, 0) }
            out.append(ch.data); offset += UInt64(ch.data.count); size = ch.size
            if ch.eof || ch.data.isEmpty { break }
        }
        return (out, false, size)
    }

    // MARK: live stream

    private func switchRun() async {
        selection = nil
        await reloadFlows()
        await attachLive()
        await refreshDiagnostics()
    }

    private func attachLive() async {
        guard let c = client, let run = effectiveRun else { return }
        if let old = subscription { try? await c.callVoid("events.unsubscribe", ["subscription": old]); subscription = nil }
        guard status?.run == run, status?.isRunning == true else { liveRun = nil; return }
        do {
            struct Sub: Decodable { var subscription: Int; var afterSeq: UInt64 }
            let sub: Sub = try await c.call("events.subscribe", ["run": run])
            subscription = sub.subscription
            lastSeq = sub.afterSeq
            liveRun = run
        } catch { lastError = error.localizedDescription }
    }

    private func startPump(_ c: ControlClient) {
        pumpTask?.cancel(); tickTask?.cancel()
        pumpTask = Task { [weak self] in
            for await n in c.notifications {
                guard let self, !Task.isCancelled else { return }
                self.ingest(n)
            }
        }
        tickTask = Task { [weak self] in
            while !Task.isCancelled {
                try? await Task.sleep(nanoseconds: 150_000_000)
                await self?.flushDirty()
            }
        }
    }

    private func watchDisconnect(_ c: ControlClient) {
        Task { [weak self] in
            for await _ in c.disconnects { self?.phase = .failed("Lost connection to the capture engine."); return }
        }
    }

    func ingest(_ n: EventNotification) {
        guard n.subscription == subscription else { return }
        lastSeq = n.envelope.seq
        eventsSeen += 1
        if let f = n.envelope.flow { dirty.insert(f) }
        switch n.envelope.event {
        case .eventsDropped(let c): diagnostics.droppedEvents += c
        case .connectionOpened(let conn, let client, let target):
            diagnostics.connections += 1
            clientByConn[conn] = String(client.split(separator: ":").first ?? "")
            targetByConn[conn] = target
        case .tlsHello(let conn, let sni, _): if let sni { sniByConn[conn] = sni }
        case .clientTlsFailed(let conn, let cls, let detail): recordTLS(conn: conn, cls: cls, detail: detail, ts: n.envelope.tsUs)
        case .connectionClosed(let conn, let requests):
            if requests == 0 { diagnostics.silentConnections += 1 }
            sniByConn[conn] = nil; targetByConn[conn] = nil; clientByConn[conn] = nil
        default: break
        }
    }

    private func recordTLS(conn: UInt64?, cls: ClientTlsClass, detail: String, ts: UInt64) {
        let host = conn.flatMap { sniByConn[$0] } ?? conn.flatMap { targetByConn[$0] }?.split(separator: ":").first.map(String.init) ?? "unknown host"
        let client = conn.flatMap { clientByConn[$0] } ?? "?"
        var p = diagByHost[host] ?? TLSProblem(host: host, count: 0, lastClass: cls, lastDetail: detail, clients: [], lastTsUs: ts)
        p.count += 1; p.lastClass = cls; p.lastDetail = detail; p.clients.insert(client); p.lastTsUs = ts
        diagByHost[host] = p
        diagnostics.problems = diagByHost.values.sorted { $0.lastTsUs > $1.lastTsUs }
    }

    /// Rebuilds diagnostics from the stored history (used when switching runs / on first connect).
    public func refreshDiagnostics() async {
        guard let c = client, let run = effectiveRun else { return }
        struct R: Decodable { var events: [Envelope] }
        guard let r: R = try? await c.call("events.diagnostics", ["run": run, "limit": 3000] as [String: any Sendable]) else { return }
        diagByHost = [:]; sniByConn = [:]; targetByConn = [:]; clientByConn = [:]
        diagnostics = Diagnostics()
        for e in r.events {
            switch e.event {
            case .eventsDropped(let n): diagnostics.droppedEvents += n
            case .connectionOpened(let conn, let client, let target):
                diagnostics.connections += 1
                clientByConn[conn] = String(client.split(separator: ":").first ?? ""); targetByConn[conn] = target
            case .tlsHello(let conn, let sni, _): if let sni { sniByConn[conn] = sni }
            case .clientTlsFailed(let conn, let cls, let detail): recordTLS(conn: conn, cls: cls, detail: detail, ts: e.tsUs)
            case .connectionClosed(_, let requests): if requests == 0 { diagnostics.silentConnections += 1 }
            default: break
            }
        }
    }

    /// Pulls fresh rows for flows touched since the last tick and merges them into the list.
    func flushDirty() async {
        guard let c = client, !dirty.isEmpty, let run = liveRun, run == effectiveRun else { dirty.removeAll(); return }
        let ids = Array(dirty.sorted(by: >).prefix(300))
        dirty.subtract(ids)
        guard let reply: FlowsReply = try? await c.call("flows.get_many", ["run": run, "flows": ids] as [String: any Sendable]) else { return }
        merge(reply.flows)
        if status?.isRunning == true, let s: EngineStatus = try? await c.call("engine.status") { status = s }
    }

    private struct FlowsReply: Decodable { var flows: [FlowRow] }

    private func merge(_ fresh: [FlowRow]) {
        let accepted = post(fresh).filter { matchesLocally($0) }
        guard !accepted.isEmpty else { return }
        var byId = Dictionary(uniqueKeysWithValues: flows.map { ($0.flow, $0) })
        for r in accepted { byId[r.flow] = r }
        flows = byId.values.sorted { ($0.startedUs, $0.flow) > ($1.startedUs, $1.flow) }
        if flows.count > memoryCap { flows = Array(flows.prefix(memoryCap)) }
    }

    /// Mirror of the server-side filter, so live rows that don't match never flash into the list.
    private func matchesLocally(_ r: FlowRow) -> Bool {
        if let sc = filter.statusClass, (r.status ?? 0) / 100 != sc { return false }
        if let m = filter.method, r.method?.uppercased() != m.uppercased() { return false }
        if let h = filter.host, r.host != h { return false }
        if filter.mockedOnly && !r.mocked { return false }
        if !filter.text.isEmpty, !(r.uri ?? "").localizedCaseInsensitiveContains(filter.text) { return false }
        return true
    }

    public func clearView() { flows = []; selection = nil }
}
