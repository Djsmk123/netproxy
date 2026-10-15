import SwiftUI
import NetProxyKit

struct SidebarView: View {
    @Environment(CaptureModel.self) private var model

    var body: some View {
        @Bindable var model = model
        List {
            Section("Capture") {
                CaptureStatusRow()
            }
            Section("Sessions") {
                SessionRow(title: "Current", subtitle: model.status?.isRunning == true ? "Recording" : "Latest", live: model.status?.isRunning == true, count: nil, selected: model.selectedRun == nil)
                    .contentShape(Rectangle())
                    .onTapGesture { model.selectedRun = nil }
                ForEach(model.runs.dropFirst(model.status?.isRunning == true ? 1 : 0)) { run in
                    SessionRow(title: Format.time(us: run.startedUs), subtitle: run.status == "interrupted" ? "Interrupted" : "\(run.flows) requests",
                               live: false, count: nil, selected: model.selectedRun == run.run)
                        .contentShape(Rectangle())
                        .onTapGesture { model.selectedRun = run.run }
                }
            }
            Section("Filter") {
                FilterRow(label: "All traffic", icon: "list.bullet", active: !model.filter.isActive || model.filter == defaultFilter) { model.filter = FilterState() }
                FilterRow(label: "Problems", icon: "exclamationmark.triangle", active: model.filter.onlyProblems) { var f = FilterState(); f.onlyProblems = true; model.filter = f }
                FilterRow(label: "Mocked", icon: "theatermasks", active: model.filter.mockedOnly) { var f = FilterState(); f.mockedOnly = true; model.filter = f }
                ForEach([2, 3, 4, 5], id: \.self) { c in
                    statusClassRow(c)
                }
            }
            if !hosts.isEmpty {
                Section("Hosts") {
                    ForEach(hosts, id: \.0) { host, n in
                        FilterRow(label: host, icon: "network", active: model.filter.host == host, trailing: "\(n)") {
                            var f = FilterState(); f.host = host; model.filter = f
                        }
                    }
                }
            }
        }
        .listStyle(.sidebar)
        .safeAreaInset(edge: .bottom) { StorageFooter() }
    }

    private var defaultFilter: FilterState { FilterState() }

    private func statusClassRow(_ c: Int) -> some View {
        let tint: Color
        switch c {
        case 2: tint = .green
        case 3: tint = .blue
        case 4: tint = .orange
        default: tint = .red
        }
        return FilterRow(label: "\(c)xx", icon: "circle.fill", active: model.filter.statusClass == c, tint: tint) {
            var f = FilterState()
            f.statusClass = c
            model.filter = f
        }
    }

    private var hosts: [(String, Int)] {
        Dictionary(grouping: model.flows.compactMap(\.host), by: { $0 }).map { ($0.key, $0.value.count) }
            .sorted { $0.1 > $1.1 }.prefix(12).map { $0 }
    }
}

struct CaptureStatusRow: View {
    @Environment(CaptureModel.self) private var model
    var body: some View {
        let s = model.status
        VStack(alignment: .leading, spacing: 4) {
            HStack(spacing: 6) {
                Circle().fill(s?.isRunning == true ? Color.red : Color.secondary.opacity(0.5)).frame(width: 9, height: 9)
                Text(s?.isRunning == true ? "Recording" : "Stopped").font(.headline)
            }
            if let l = s?.listen, s?.isRunning == true {
                Text("Proxy \(l)").font(.caption).foregroundStyle(.secondary).textSelection(.enabled)
                if model.listenAllInterfaces { Pill(text: "Open to your network", color: .orange) }
            } else {
                Text("Press Record to start capturing").font(.caption).foregroundStyle(.secondary)
            }
        }.padding(.vertical, 2)
    }
}

struct SessionRow: View {
    var title: String
    var subtitle: String
    var live: Bool
    var count: Int?
    var selected: Bool
    var body: some View {
        HStack {
            VStack(alignment: .leading, spacing: 1) {
                Text(title).fontWeight(selected ? .semibold : .regular)
                Text(subtitle).font(.caption).foregroundStyle(.secondary)
            }
            Spacer()
            if live { Circle().fill(.red).frame(width: 7, height: 7) }
        }
        .padding(.vertical, 1)
        .listRowBackground(selected ? Color.accentColor.opacity(0.18) : Color.clear)
    }
}

struct FilterRow: View {
    var label: String
    var icon: String
    var active: Bool
    var tint: Color = .secondary
    var trailing: String? = nil
    var action: () -> Void
    var body: some View {
        Button(action: action) {
            HStack {
                Image(systemName: icon).foregroundStyle(tint).frame(width: 18)
                Text(label).lineLimit(1).truncationMode(.middle)
                Spacer()
                if let t = trailing { Text(t).font(.caption).foregroundStyle(.secondary) }
            }
            .fontWeight(active ? .semibold : .regular)
        }
        .buttonStyle(.plain)
        .listRowBackground(active ? Color.accentColor.opacity(0.18) : Color.clear)
    }
}

struct StorageFooter: View {
    @Environment(CaptureModel.self) private var model
    var body: some View {
        if let st = model.storage {
            HStack {
                Image(systemName: "internaldrive")
                Text("\(Format.bytes(Int64(st.usedBytes)))" + (st.maxBytes.map { " of \(Format.bytes(Int64($0)))" } ?? ""))
                Spacer()
            }
            .font(.caption).foregroundStyle(.secondary).padding(8).background(.bar)
        }
    }
}
