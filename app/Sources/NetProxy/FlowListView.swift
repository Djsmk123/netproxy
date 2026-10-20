import SwiftUI
import AppKit
import NetProxyKit

struct FlowListView: View {
    @Environment(CaptureModel.self) private var model
    @State private var showMapLocal = false
    @State private var showMapRemote = false
    @State private var selectedFlowForMapping: FlowRow?
    @State private var breakpointPattern: String?
    @State private var showBreakpointAdded = false

    var body: some View {
        @Bindable var model = model
        VStack(spacing: 0) {
            if model.flows.isEmpty {
                emptyState
            } else {
                Table(model.flows, selection: $model.selection) {
                    TableColumn("") { r in
                        Image(systemName: icon(r)).foregroundStyle(Style.statusColor(r)).help(r.state)
                    }.width(22)
                    TableColumn("Status") { r in
                        Text(Style.statusText(r)).foregroundStyle(Style.statusColor(r)).monospacedDigit()
                    }.width(min: 48, ideal: 56, max: 70)
                    TableColumn("Method") { r in
                        Text(r.method ?? "").foregroundStyle(Style.methodColor(r.method)).fontWeight(.medium)
                    }.width(min: 52, ideal: 62, max: 80)
                    TableColumn("Host") { r in
                        HStack(spacing: 4) {
                            Text(r.host ?? "—").lineLimit(1)
                            if r.mocked { Pill(text: "MOCK", color: .purple) }
                            if r.rule != nil { Pill(text: "RULE", color: .green) }
                        }
                    }.width(min: 120, ideal: 200)
                    TableColumn("Path") { r in
                        Text(r.path ?? "").lineLimit(1).truncationMode(.middle).foregroundStyle(.secondary)
                    }.width(min: 140, ideal: 300)
                    TableColumn("Size") { r in
                        Text(r.resBytes > 0 ? Format.bytes(r.resBytes) : "—").monospacedDigit().foregroundStyle(.secondary)
                    }.width(min: 56, ideal: 66, max: 90)
                    TableColumn("Time") { r in
                        Text(Format.duration(us: r.durationUs)).monospacedDigit().foregroundStyle(.secondary)
                    }.width(min: 56, ideal: 66, max: 90)
                    TableColumn("Started") { r in
                        Text(Format.time(us: r.startedUs)).monospacedDigit().foregroundStyle(.secondary)
                    }.width(min: 64, ideal: 76, max: 90)
                }
                .contextMenu(forSelectionType: FlowRow.ID.self) { ids in
                    if let id = ids.first, let r = model.flows.first(where: { $0.id == id }) {
                        // Breakpoint
                        Button("Add Breakpoint...") {
                            if let uri = r.uri {
                                // Extract pattern from URI
                                let pattern = uri.components(separatedBy: "?").first ?? uri
                                addBreakpointPattern(pattern)
                            }
                        }

                        Divider()

                        // Copy actions
                        Button("Copy URL") { copyToPasteboard(r.uri ?? "") }
                        Button("Copy as cURL") {
                            Task {
                                if let detail = await model.detail(for: r) {
                                    let headers = detail.events.lazy.compactMap { e -> Headers? in
                                        if case .requestHead(let req) = e.event { return req.headers }
                                        return nil
                                    }.first ?? []
                                    let body = await model.body(for: r, dir: .req).data
                                    let curl = CurlExport.command(method: r.method ?? "GET", url: r.uri ?? "", headers: headers, body: body)
                                    copyToPasteboard(curl)
                                }
                            }
                        }

                        Divider()

                        // Save request/response
                        Button("Save Request...") {
                            Task {
                                await saveRequest(for: r)
                            }
                        }
                        Button("Save Response...") {
                            Task {
                                await saveResponse(for: r)
                            }
                        }
                        Button("Save Both...") {
                            Task {
                                await saveRequestAndResponse(for: r)
                            }
                        }

                        Divider()

                        // Mocking
                        Button("Map Local...") {
                            selectedFlowForMapping = r
                            showMapLocal = true
                        }
                        Button("Map Remote...") {
                            selectedFlowForMapping = r
                            showMapRemote = true
                        }

                        Divider()

                        // Filtering
                        if let h = r.host {
                            Button("Filter by Host: \(h)") { var f = FilterState(); f.host = h; model.filter = f }
                        }
                        if let m = r.method {
                            Button("Filter by Method: \(m)") { var f = FilterState(); f.method = m; model.filter = f }
                        }
                        if let status = r.status {
                            let statusClass = status / 100
                            Button("Filter \(statusClass)xx Responses") { var f = FilterState(); f.statusClass = statusClass; model.filter = f }
                        }
                    }
                }
                footer
            }
        }
        .sheet(isPresented: $showMapLocal) {
            if let flow = selectedFlowForMapping {
                MapLocalView(flow: flow)
                    .environment(model)
            }
        }
        .sheet(isPresented: $showMapRemote) {
            if let flow = selectedFlowForMapping {
                MapRemoteView(flow: flow)
                    .environment(model)
            }
        }
    }

    private func icon(_ r: FlowRow) -> String {
        switch r.state {
        case "failed": return "xmark.circle.fill"
        case "aborted": return "exclamationmark.circle.fill"
        case "open": return "circle.dotted"
        default: return "checkmark.circle.fill"
        }
    }

    private var footer: some View {
        HStack {
            Text("\(model.flows.count) request\(model.flows.count == 1 ? "" : "s")")
            if model.loading { ProgressView().controlSize(.small) }
            Spacer()
            if model.hasMore { Button("Load older…") { Task { await model.loadMore() } }.buttonStyle(.link) }
        }
        .font(.caption).foregroundStyle(.secondary)
        .padding(.horizontal, 10).padding(.vertical, 5).background(.bar)
    }

    // MARK: - Save Request/Response

    private func saveRequest(for row: FlowRow) async {
        guard let detail = await model.detail(for: row) else { return }
        let headers = detail.events.lazy.compactMap { e -> Headers? in
            if case .requestHead(let req) = e.event { return req.headers }
            return nil
        }.first ?? []
        let body = await model.body(for: row, dir: BodyDir.req).data

        let content = formatHTTP(method: row.method ?? "GET", url: row.uri ?? "", headers: headers, body: body)
        saveToFile(content: content, suggestedName: "\(row.host ?? "request")_\(row.flow)_request.txt")
    }

    private func saveResponse(for row: FlowRow) async {
        guard let detail = await model.detail(for: row) else { return }
        let headers = detail.events.lazy.compactMap { e -> Headers? in
            if case .responseHead(let resp) = e.event { return resp.headers }
            return nil
        }.first ?? []
        let body = await model.body(for: row, dir: BodyDir.res).data

        let content = formatHTTPResponse(status: row.status ?? 0, headers: headers, body: body)
        saveToFile(content: content, suggestedName: "\(row.host ?? "response")_\(row.flow)_response.txt")
    }

    private func saveRequestAndResponse(for row: FlowRow) async {
        guard let detail = await model.detail(for: row) else { return }

        let reqHeaders = detail.events.lazy.compactMap { e -> Headers? in
            if case .requestHead(let req) = e.event { return req.headers }
            return nil
        }.first ?? []
        let reqBody = await model.body(for: row, dir: .req).data

        let respHeaders = detail.events.lazy.compactMap { e -> Headers? in
            if case .responseHead(let resp) = e.event { return resp.headers }
            return nil
        }.first ?? []
        let respBody = await model.body(for: row, dir: .res).data

        var content = "REQUEST\n"
        content += "=======\n\n"
        content += formatHTTP(method: row.method ?? "GET", url: row.uri ?? "", headers: reqHeaders, body: reqBody)
        content += "\n\n\nRESPONSE\n"
        content += "========\n\n"
        content += formatHTTPResponse(status: row.status ?? 0, headers: respHeaders, body: respBody)

        saveToFile(content: content, suggestedName: "\(row.host ?? "flow")_\(row.flow).txt")
    }

    private func formatHTTP(method: String, url: String, headers: Headers, body: Data) -> String {
        var text = "\(method) \(url) HTTP/1.1\n"
        for header in headers {
            if header.count >= 2 {
                text += "\(header[0]): \(header[1])\n"
            }
        }
        text += "\n"
        if !body.isEmpty {
            text += String(data: body, encoding: .utf8) ?? "[Binary data: \(body.count) bytes]"
        }
        return text
    }

    private func formatHTTPResponse(status: Int, headers: Headers, body: Data) -> String {
        var text = "HTTP/1.1 \(status)\n"
        for header in headers {
            if header.count >= 2 {
                text += "\(header[0]): \(header[1])\n"
            }
        }
        text += "\n"
        if !body.isEmpty {
            text += String(data: body, encoding: .utf8) ?? "[Binary data: \(body.count) bytes]"
        }
        return text
    }

    private func saveToFile(content: String, suggestedName: String) {
        let panel = NSSavePanel()
        panel.nameFieldStringValue = suggestedName
        panel.canCreateDirectories = true
        panel.begin { response in
            if response == .OK, let url = panel.url {
                try? content.write(to: url, atomically: true, encoding: .utf8)
            }
        }
    }

    @ViewBuilder private var emptyState: some View {
        if model.filter.isActive {
            ContentUnavailableView("No matching requests", systemImage: "line.3.horizontal.decrease.circle",
                                   description: Text("Nothing in this session matches the current filter."))
        } else if model.status?.isRunning == true {
            ContentUnavailableView {
                Label("Waiting for traffic", systemImage: "antenna.radiowaves.left.and.right")
            } description: {
                Text("Point a device or app at the proxy \(model.status?.listen ?? ""), then make a request.")
            } actions: {
                Text("Phones: open Device Setup in the toolbar.").font(.caption).foregroundStyle(.secondary)
            }
        } else {
            ContentUnavailableView {
                Label("Not recording", systemImage: "record.circle")
            } description: {
                Text("Start capturing to see HTTP and HTTPS requests here.")
            } actions: {
                Button("Start Capture") { Task { await model.startCapture(allowDevices: model.listenAllInterfaces) } }.buttonStyle(.borderedProminent)
            }
        }
    }

    private func addBreakpointPattern(_ pattern: String) {
        breakpointPattern = pattern

        Task {
            guard let client = model.client else { return }
            do {
                let params: [String: any Sendable] = ["pattern": pattern, "enabled": true]
                try await client.callVoid("breakpoints.add", params)
                showBreakpointAdded = true

                // Hide after 2 seconds
                try? await Task.sleep(nanoseconds: 2_000_000_000)
                showBreakpointAdded = false
            } catch {
                // Silently fail
            }
        }
    }
}
