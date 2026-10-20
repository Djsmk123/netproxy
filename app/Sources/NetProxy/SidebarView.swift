import SwiftUI
import NetProxyKit

struct SidebarView: View {
    @Environment(CaptureModel.self) private var model
    @State private var selectedSessions: Set<String> = []
    @State private var showBulkDeleteConfirmation = false

    var body: some View {
        @Bindable var model = model
        List {
            Section("Capture") {
                CaptureStatusRow()
            }
            Section("Sessions") {
                SessionRow(title: "Current", subtitle: model.status?.isRunning == true ? "Recording" : "Latest", live: model.status?.isRunning == true, count: nil, selected: model.selectedRun == nil)
                    .contentShape(Rectangle())
                    .onTapGesture {
                        selectedSessions.removeAll()
                        model.selectedRun = nil
                    }
                ForEach(model.runs.dropFirst(model.status?.isRunning == true ? 1 : 0)) { run in
                    let isSelected = selectedSessions.contains(run.run) || (selectedSessions.isEmpty && model.selectedRun == run.run)
                    SessionRow(title: Format.time(us: run.startedUs), subtitle: run.status == "interrupted" ? "Interrupted" : "\(run.flows) requests",
                               live: false, count: nil, selected: isSelected)
                        .contentShape(Rectangle())
                        .onTapGesture {
                            handleSessionTap(run.run)
                        }
                        .contextMenu {
                            Button("Select All Sessions") {
                                selectedSessions = Set(model.runs.map(\.run))
                            }

                            if !selectedSessions.isEmpty {
                                Button("Clear Selection") {
                                    selectedSessions.removeAll()
                                }
                            }

                            Divider()

                            if selectedSessions.count > 1 {
                                Button("Delete \(selectedSessions.count) Sessions", role: .destructive) {
                                    showBulkDeleteConfirmation = true
                                }
                            } else if selectedSessions.count == 1 {
                                Button("Delete Selected Session", role: .destructive) {
                                    Task {
                                        await model.deleteSessions(selectedSessions)
                                        selectedSessions.removeAll()
                                    }
                                }
                            } else {
                                Button("Delete This Session", role: .destructive) {
                                    Task {
                                        await model.deleteSessions([run.run])
                                    }
                                }
                            }
                        }
                }
            }
            .confirmationDialog(
                "Delete \(selectedSessions.count) Sessions?",
                isPresented: $showBulkDeleteConfirmation,
                titleVisibility: .visible
            ) {
                Button("Delete \(selectedSessions.count) Sessions Permanently", role: .destructive) {
                    Task {
                        await model.deleteSessions(selectedSessions)
                        selectedSessions.removeAll()
                    }
                }
                Button("Cancel", role: .cancel) {}
            } message: {
                Text("This will permanently delete \(selectedSessions.count) sessions with all their flows and cannot be undone.")
            }

            Section("Filter") {
                FilterRow(label: "All traffic", icon: "list.bullet", active: !model.filter.isActive || model.filter == defaultFilter) { model.filter = FilterState() }
                FilterRow(label: "Problems", icon: "exclamationmark.triangle", active: model.filter.onlyProblems) { var f = FilterState(); f.onlyProblems = true; model.filter = f }
                FilterRow(label: "Mocked", icon: "theatermasks", active: model.filter.mockedOnly) { var f = FilterState(); f.mockedOnly = true; model.filter = f }
                Divider()
                Group {
                    statusClassRow(2, label: "Success (2xx)")
                    statusClassRow(3, label: "Redirect (3xx)")
                    statusClassRow(4, label: "Client Error (4xx)")
                    statusClassRow(5, label: "Server Error (5xx)")
                }
            }
            if !methods.isEmpty {
                Section("Methods") {
                    ForEach(methods, id: \.0) { method, n in
                        FilterRow(label: method, icon: "arrow.right.circle", active: model.filter.method == method, tint: Style.methodColor(method), trailing: "\(n)") {
                            var f = FilterState(); f.method = method; model.filter = f
                        }
                    }
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

    private func statusClassRow(_ c: Int, label: String? = nil) -> some View {
        let tint: Color
        switch c {
        case 2: tint = .green
        case 3: tint = .blue
        case 4: tint = .orange
        default: tint = .red
        }
        return FilterRow(label: label ?? "\(c)xx", icon: "circle.fill", active: model.filter.statusClass == c, tint: tint) {
            var f = FilterState()
            f.statusClass = c
            model.filter = f
        }
    }

    private var methods: [(String, Int)] {
        Dictionary(grouping: model.flows.compactMap(\.method), by: { $0 }).map { ($0.key, $0.value.count) }
            .sorted { $0.1 > $1.1 }.prefix(8).map { $0 }
    }

    private var hosts: [(String, Int)] {
        Dictionary(grouping: model.flows.compactMap(\.host), by: { $0 }).map { ($0.key, $0.value.count) }
            .sorted { $0.1 > $1.1 }.prefix(12).map { $0 }
    }

    private func handleSessionTap(_ run: String) {
        let cmdPressed = NSEvent.modifierFlags.contains(.command)
        let shiftPressed = NSEvent.modifierFlags.contains(.shift)

        if cmdPressed {
            // Cmd+Click: Toggle selection
            if selectedSessions.contains(run) {
                selectedSessions.remove(run)
            } else {
                selectedSessions.insert(run)
            }
        } else if shiftPressed {
            // Shift+Click: Select range
            let allRuns = model.runs.map(\.run)
            if let currentRun = model.selectedRun,
               let currentIdx = allRuns.firstIndex(of: currentRun),
               let targetIdx = allRuns.firstIndex(of: run) {
                let range = min(currentIdx, targetIdx)...max(currentIdx, targetIdx)
                selectedSessions = Set(allRuns[range])
            } else {
                selectedSessions = [run]
            }
        } else {
            // Normal click: Clear selection and select this one
            selectedSessions.removeAll()
            model.selectedRun = run
        }
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
            HStack(spacing: 4) {
                Image(systemName: "internaldrive").font(.caption)
                Text("\(Format.bytes(Int64(st.usedBytes)))").font(.caption)
                if let m = st.maxBytes { Text("/ \(Format.bytes(Int64(m)))").font(.caption).foregroundStyle(.secondary) }
                Spacer()
            }
            .foregroundStyle(.secondary).padding(.horizontal, 12).padding(.vertical, 6)
            .background(.regularMaterial)
        }
    }
}
