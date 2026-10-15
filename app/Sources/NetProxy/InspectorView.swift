import SwiftUI
import AppKit
import NetProxyKit

struct InspectorView: View {
    @Environment(CaptureModel.self) private var model
    @State private var tab: Tab = {
        #if DEBUG
        if let t = DemoHooks.tab, let m = Tab.allCases.first(where: { $0.rawValue.lowercased() == t }) { return m }
        #endif
        return .overview
    }()
    @State private var detail: FlowDetail?

    enum Tab: String, CaseIterable, Identifiable { case overview = "Overview", request = "Request", response = "Response", timeline = "Timeline"; var id: String { rawValue } }

    private var row: FlowRow? { model.selection.flatMap { id in model.flows.first { $0.id == id } } }

    var body: some View {
        if let row {
            VStack(spacing: 0) {
                header(row)
                Divider()
                Group {
                    switch tab {
                    case .overview: OverviewPane(row: row, detail: detail)
                    case .request: MessagePane(row: row, detail: detail, dir: .req)
                    case .response: MessagePane(row: row, detail: detail, dir: .res)
                    case .timeline: TimelinePane(row: row, detail: detail)
                    }
                }
            }
            // reload when the flow changes or finishes (rows update in place while traffic is live)
            .task(id: "\(row.id)/\(row.lastSeq)") { detail = await model.detail(for: row) }
        } else {
            ContentUnavailableView("Select a request", systemImage: "arrow.up.left.and.arrow.down.right",
                                   description: Text("Details, headers and bodies appear here."))
        }
    }

    private func header(_ r: FlowRow) -> some View {
        HStack(spacing: 10) {
            Picker("", selection: $tab) { ForEach(Tab.allCases) { Text($0.rawValue).tag($0) } }
                .pickerStyle(.segmented).labelsHidden().frame(maxWidth: 360)
            Spacer()
            Text("\(r.method ?? "") \(r.host ?? "")\(r.path ?? "")").font(.caption).foregroundStyle(.secondary).lineLimit(1).truncationMode(.middle)
            Button { copyToPasteboard(r.uri ?? "") } label: { Image(systemName: "link") }.help("Copy URL").buttonStyle(.borderless)
        }
        .padding(.horizontal, 10).padding(.vertical, 6).background(.bar)
    }
}

// MARK: overview

struct OverviewPane: View {
    var row: FlowRow
    var detail: FlowDetail?

    var body: some View {
        ScrollView {
            Grid(alignment: .leadingFirstTextBaseline, horizontalSpacing: 14, verticalSpacing: 6) {
                kv("URL", row.uri ?? "—")
                kv("Status", statusLine)
                kv("Method", row.method ?? "—")
                kv("Protocol", row.httpVersion ?? "—")
                kv("Client", row.client ?? "—")
                kv("Started", Format.time(us: row.startedUs))
                kv("Duration", Format.duration(us: row.durationUs))
                kv("Time to first byte", Format.duration(us: row.ttfbUs))
                kv("Request size", Format.bytes(row.reqBytes))
                kv("Response size", Format.bytes(row.resBytes))
                if let rule = row.rule { kv("Mock rule", rule) }
                if let sha = row.resSha256 { kv("Response SHA-256", sha) }
            }
            .padding(14)
            if let e = Explain.failure(row.failClass) {
                VStack(alignment: .leading, spacing: 4) {
                    Label(e.title, systemImage: "xmark.octagon.fill").font(.headline).foregroundStyle(.red)
                    Text(e.hint)
                    if let d = row.failDetail { Text(d).font(.caption.monospaced()).foregroundStyle(.secondary).textSelection(.enabled) }
                }
                .padding(12).frame(maxWidth: .infinity, alignment: .leading)
                .background(Color.red.opacity(0.08), in: RoundedRectangle(cornerRadius: 8)).padding(.horizontal, 14).padding(.bottom, 14)
            } else if row.state == "aborted" {
                Label("The transfer ended early (client cancelled or the server reset the connection). Bytes received so far were kept.", systemImage: "exclamationmark.circle")
                    .padding(12).frame(maxWidth: .infinity, alignment: .leading)
                    .background(Color.orange.opacity(0.1), in: RoundedRectangle(cornerRadius: 8)).padding(.horizontal, 14).padding(.bottom, 14)
            }
        }
    }

    private var statusLine: String {
        switch row.state {
        case "failed": return "Failed — no response"
        case "open": return "In progress…"
        case "aborted": return "\(row.status.map(String.init) ?? "—") (transfer aborted)"
        default: return row.status.map(String.init) ?? "—"
        }
    }

    private func kv(_ k: String, _ v: String) -> some View {
        GridRow {
            Text(k).foregroundStyle(.secondary).gridColumnAlignment(.trailing)
            Text(v).textSelection(.enabled).frame(maxWidth: .infinity, alignment: .leading)
        }
    }
}

// MARK: request / response

struct MessagePane: View {
    @Environment(CaptureModel.self) private var model
    var row: FlowRow
    var detail: FlowDetail?
    var dir: BodyDir

    @State private var decoded: DecodedBody?
    @State private var loading = false
    @State private var evicted = false
    @State private var mode: Mode = .pretty
    enum Mode: String, CaseIterable { case pretty = "Pretty", raw = "Raw", hex = "Hex" }

    private var headers: Headers {
        guard let d = detail else { return [] }
        for e in d.events {
            switch (dir, e.event) {
            case (.req, .requestHead(let r)): return r.headers
            case (.res, .responseHead(_, _, _, let h, _)): return h
            default: continue
            }
        }
        return []
    }

    private var bodyEnd: Event.BodyEnd? {
        detail?.events.lazy.compactMap { if case .bodyEnd(let b) = $0.event, b.dir == dir { return b } else { return nil } }.first
    }

    private var hasResponse: Bool { detail?.events.contains { if case .responseHead = $0.event { return true } else { return false } } ?? false }

    var body: some View {
        if dir == .res && !hasResponse && detail != nil {
            ContentUnavailableView(row.state == "failed" ? "No response" : "Waiting for response", systemImage: row.state == "failed" ? "xmark.circle" : "hourglass")
        } else {
            HSplitView {
                headersList.frame(minWidth: 220, idealWidth: 330)
                bodyPane.frame(minWidth: 260)
            }
            .task(id: "\(row.id)/\(row.lastSeq)/\(dir.rawValue)/\(detail == nil)") { await loadBody() }
        }
    }

    private var headersList: some View {
        VStack(spacing: 0) {
            HStack {
                Text("Headers (\(headers.count))").font(.caption.weight(.semibold)).foregroundStyle(.secondary)
                Spacer()
                if dir == .req {
                    Button("Copy cURL") { copyToPasteboard(CurlExport.command(method: row.method ?? "GET", url: row.uri ?? "", headers: headers, body: decoded?.data)) }
                        .buttonStyle(.link).font(.caption)
                }
            }.padding(.horizontal, 10).padding(.vertical, 5).background(.bar)
            ScrollView {
                if headers.isEmpty {
                    Text("No headers").foregroundStyle(.secondary).padding()
                } else {
                    Grid(alignment: .topLeading, horizontalSpacing: 10, verticalSpacing: 4) {
                        ForEach(Array(headers.enumerated()), id: \.offset) { _, h in
                            GridRow {
                                Text(h.first ?? "").font(.caption.weight(.semibold)).foregroundStyle(.secondary).gridColumnAlignment(.trailing)
                                Text(h.count > 1 ? h[1] : "").font(.caption.monospaced()).textSelection(.enabled).frame(maxWidth: .infinity, alignment: .leading)
                            }
                        }
                    }.padding(10)
                }
            }
        }
    }

    private var bodyPane: some View {
        VStack(spacing: 0) {
            HStack {
                Text("Body").font(.caption.weight(.semibold)).foregroundStyle(.secondary)
                if let b = bodyEnd { Text(Format.bytes(Int64(b.total))).font(.caption).foregroundStyle(.secondary) }
                if let ct = decoded?.contentType { Text(ct).font(.caption).foregroundStyle(.tertiary).lineLimit(1) }
                Spacer()
                if let d = decoded, d.kind == .json || d.kind == .text || d.kind == .binary {
                    Picker("", selection: $mode) { ForEach(Mode.allCases, id: \.self) { Text($0.rawValue).tag($0) } }
                        .pickerStyle(.segmented).labelsHidden().frame(width: 170)
                }
                if let d = decoded, d.kind != .empty {
                    Button("Copy") { copyToPasteboard(d.text ?? String(decoding: d.data, as: UTF8.self)) }.buttonStyle(.link).font(.caption)
                }
            }.padding(.horizontal, 10).padding(.vertical, 5).background(.bar)
            content
        }
    }

    @ViewBuilder private var content: some View {
        if loading && decoded == nil {
            ProgressView().frame(maxWidth: .infinity, maxHeight: .infinity)
        } else if evicted {
            ContentUnavailableView("Body no longer stored", systemImage: "externaldrive.badge.xmark", description: Text("It was removed to stay within the storage limit."))
        } else if let d = decoded {
            VStack(alignment: .leading, spacing: 0) {
                ForEach(d.notes + (bodyEnd?.truncated == true && !d.notes.contains { $0.contains("first part") } ? ["Only the first part of this body was stored."] : []), id: \.self) { n in
                    Label(n, systemImage: "info.circle").font(.caption).foregroundStyle(.secondary).padding(.horizontal, 10).padding(.top, 6)
                }
                switch d.kind {
                case .empty: Text("No body").foregroundStyle(.secondary).padding()
                case .image:
                    if let img = NSImage(data: d.data) {
                        ScrollView([.horizontal, .vertical]) { Image(nsImage: img).padding() }
                    } else { Text("Image could not be displayed").foregroundStyle(.secondary).padding() }
                case .json, .text, .form: textView(for: d)
                case .binary: monospaced(mode == .hex || true ? BodyDecoder.hexDump(d.data) : "")
                }
            }
        } else {
            Text("—").foregroundStyle(.secondary).frame(maxWidth: .infinity, maxHeight: .infinity)
        }
    }

    private func textView(for d: DecodedBody) -> some View {
        let shown: String
        switch mode {
        case .pretty: shown = d.text ?? String(decoding: d.data, as: UTF8.self)
        case .raw: shown = String(decoding: d.data, as: UTF8.self)
        case .hex: shown = BodyDecoder.hexDump(d.data)
        }
        return monospaced(shown)
    }

    private func monospaced(_ s: String) -> some View {
        let cap = 200_000
        return ScrollView([.vertical, .horizontal]) {
            VStack(alignment: .leading) {
                Text(s.count > cap ? String(s.prefix(cap)) : s).font(.system(.caption, design: .monospaced)).textSelection(.enabled)
                if s.count > cap { Text("… \(s.count - cap) more characters not shown").font(.caption).foregroundStyle(.secondary) }
            }.padding(10).frame(maxWidth: .infinity, alignment: .leading)
        }
    }

    private func loadBody() async {
        guard detail != nil else { return }
        loading = true; defer { loading = false }
        let r = await model.body(for: row, dir: dir)
        evicted = r.evicted
        decoded = BodyDecoder.decode(r.data, headers: headers, truncated: bodyEnd?.truncated ?? false)
    }
}

// MARK: timeline

struct TimelinePane: View {
    var row: FlowRow
    var detail: FlowDetail?
    var body: some View {
        let events = detail?.events ?? []
        List(events, id: \.seq) { e in
            HStack(alignment: .firstTextBaseline) {
                Text("+" + Format.duration(us: Int64(e.tsUs) - row.startedUs)).font(.caption.monospacedDigit()).foregroundStyle(.secondary).frame(width: 80, alignment: .trailing)
                Text(e.kind).font(.caption.monospaced().weight(.semibold))
                Text(describe(e.event)).font(.caption).foregroundStyle(.secondary).lineLimit(2)
            }
        }
    }

    private func describe(_ e: Event) -> String {
        switch e {
        case .requestHead(let r): return "\(r.method) \(r.uri) (\(r.httpVersion))"
        case .responseHead(_, let s, let ttfb, _, let m): return "\(s)\(m ? " mocked" : "") — first byte after \(Format.duration(us: Int64(ttfb)))"
        case .bodyEnd(let b): return "\(b.dir.rawValue) \(Format.bytes(Int64(b.total))) in \(b.chunks) chunks\(b.truncated ? ", stored partially" : "")"
        case .bodyError(_, let d, let t, let err): return "\(d.rawValue) after \(Format.bytes(Int64(t))): \(err)"
        case .bodyAborted(_, let d, let t): return "\(d.rawValue) aborted after \(Format.bytes(Int64(t)))"
        case .mockHit(_, let rule): return "rule \(rule)"
        case .flowCompleted(_, let s, _): return s.map { "status \($0)" } ?? ""
        case .flowFailed(_, let c, let d): return "\(c): \(d)"
        default: return ""
        }
    }
}
