import SwiftUI
import NetProxyKit

struct SessionManagementView: View {
    @Environment(\.dismiss) private var dismiss
    @Environment(CaptureModel.self) private var model
    @State private var showClearAllConfirmation = false
    @State private var expandedHosts = Set<String>()

    private var flowsByHost: [(host: String, flows: [FlowRow])] {
        let grouped = Dictionary(grouping: model.flows, by: { $0.host ?? "Unknown" })
        return grouped.map { (host: $0.key, flows: $0.value) }
            .sorted { $0.flows.count > $1.flows.count }
    }

    var body: some View {
        VStack(spacing: 0) {
            // Header
            HStack {
                Text("Clear Sessions").font(.title2).fontWeight(.semibold)
                Spacer()
                Button("Clear All") { showClearAllConfirmation = true }
                    .buttonStyle(.borderedProminent)
                    .tint(.red)
                Button("Done") { dismiss() }
                    .keyboardShortcut(.defaultAction)
            }
            .padding()

            Divider()

            // Content
            if model.flows.isEmpty {
                ContentUnavailableView(
                    "No Flows to Clear",
                    systemImage: "tray",
                    description: Text("Start capturing to see flows grouped by host.")
                )
            } else {
                ScrollView {
                    LazyVStack(spacing: 0, pinnedViews: .sectionHeaders) {
                        Section {
                            VStack(spacing: 8) {
                                ForEach(flowsByHost, id: \.host) { item in
                                    HostGroupRow(
                                        host: item.host,
                                        flowCount: item.flows.count,
                                        isExpanded: expandedHosts.contains(item.host),
                                        onToggle: {
                                            if expandedHosts.contains(item.host) {
                                                expandedHosts.remove(item.host)
                                            } else {
                                                expandedHosts.insert(item.host)
                                            }
                                        },
                                        onClear: {
                                            clearHost(item.host)
                                        }
                                    )

                                    if expandedHosts.contains(item.host) {
                                        VStack(spacing: 4) {
                                            ForEach(item.flows.prefix(10)) { flow in
                                                FlowSummaryRow(flow: flow)
                                            }
                                            if item.flows.count > 10 {
                                                Text("... and \(item.flows.count - 10) more")
                                                    .font(.caption)
                                                    .foregroundStyle(.secondary)
                                                    .padding(.vertical, 4)
                                            }
                                        }
                                        .padding(.leading, 32)
                                    }
                                }
                            }
                            .padding()
                        } header: {
                            HStack {
                                Text("\(flowsByHost.count) host\(flowsByHost.count == 1 ? "" : "s"), \(model.flows.count) total flows")
                                    .font(.caption)
                                    .foregroundStyle(.secondary)
                                Spacer()
                            }
                            .padding(.horizontal)
                            .padding(.vertical, 6)
                            .background(.bar)
                        }
                    }
                }
            }
        }
        .frame(width: 600, height: 500)
        .confirmationDialog(
            "Clear All Sessions?",
            isPresented: $showClearAllConfirmation,
            titleVisibility: .visible
        ) {
            Button("Clear All Flows", role: .destructive) {
                model.clearView()
                dismiss()
            }
            Button("Cancel", role: .cancel) {}
        } message: {
            Text("This will remove all \(model.flows.count) flows from the current view. The data remains in the session storage.")
        }
    }

    private func clearHost(_ host: String) {
        // For now, this clears from the view by filtering
        // In a full implementation, this would call a backend API to remove specific flows
        var newFilter = model.filter
        newFilter.host = host == "Unknown" ? nil : host
        model.filter = newFilter

        // Then clear the view (which respects the filter)
        // Note: This is a simplified version. A proper implementation would need
        // backend support to actually delete flows by host from storage
        DispatchQueue.main.asyncAfter(deadline: .now() + 0.1) {
            model.clearView()
            model.filter = FilterState() // Reset filter
        }
    }
}

struct HostGroupRow: View {
    let host: String
    let flowCount: Int
    let isExpanded: Bool
    let onToggle: () -> Void
    let onClear: () -> Void

    var body: some View {
        HStack(spacing: 12) {
            Button(action: onToggle) {
                Image(systemName: isExpanded ? "chevron.down" : "chevron.right")
                    .font(.caption)
                    .foregroundStyle(.secondary)
                    .frame(width: 16)
            }
            .buttonStyle(.plain)

            Image(systemName: "network")
                .foregroundStyle(.blue)

            VStack(alignment: .leading, spacing: 2) {
                Text(host)
                    .font(.body)
                    .lineLimit(1)
                Text("\(flowCount) flow\(flowCount == 1 ? "" : "s")")
                    .font(.caption)
                    .foregroundStyle(.secondary)
            }

            Spacer()

            Button("Clear") { onClear() }
                .buttonStyle(.bordered)
                .controlSize(.small)
        }
        .padding(.vertical, 8)
        .padding(.horizontal, 12)
        .background(Color.secondary.opacity(0.05))
        .cornerRadius(8)
    }
}

struct FlowSummaryRow: View {
    let flow: FlowRow

    var body: some View {
        HStack(spacing: 8) {
            Image(systemName: statusIcon)
                .font(.caption)
                .foregroundStyle(Style.statusColor(flow))
                .frame(width: 16)

            Text(flow.method ?? "—")
                .font(.caption.monospaced().weight(.medium))
                .foregroundStyle(Style.methodColor(flow.method))
                .frame(width: 50, alignment: .leading)

            Text(flow.path ?? "—")
                .font(.caption)
                .foregroundStyle(.secondary)
                .lineLimit(1)
                .truncationMode(.middle)

            Spacer()

            if let status = flow.status {
                Text("\(status)")
                    .font(.caption.monospacedDigit())
                    .foregroundStyle(Style.statusColor(flow))
            }
        }
        .padding(.vertical, 2)
        .padding(.horizontal, 8)
    }

    private var statusIcon: String {
        switch flow.state {
        case "failed": return "xmark.circle.fill"
        case "aborted": return "exclamationmark.circle.fill"
        case "open": return "circle.dotted"
        default: return "checkmark.circle.fill"
        }
    }
}
