import SwiftUI
import NetProxyKit

struct FlowListView: View {
    @Environment(CaptureModel.self) private var model

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
                        Button("Copy URL") { copyToPasteboard(r.uri ?? "") }
                        if let h = r.host {
                            Button("Filter by Host: \(h)") { var f = FilterState(); f.host = h; model.filter = f }
                        }
                        if let m = r.method {
                            Button("Filter by Method: \(m)") { var f = FilterState(); f.method = m; model.filter = f }
                        }
                        Divider()
                        if let status = r.status {
                            let statusClass = status / 100
                            Button("Filter \(statusClass)xx Responses") { var f = FilterState(); f.statusClass = statusClass; model.filter = f }
                        }
                        Divider()
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
                    }
                }
                footer
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
}
