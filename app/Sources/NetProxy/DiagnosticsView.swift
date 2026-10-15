import SwiftUI
import NetProxyKit

struct DiagnosticsView: View {
    @Environment(CaptureModel.self) private var model
    @Environment(\.dismiss) private var dismiss

    var body: some View {
        let d = model.diagnostics
        VStack(spacing: 0) {
            HStack {
                Text("Diagnostics").font(.title2.weight(.semibold))
                Spacer()
                Button { Task { await model.refreshDiagnostics() } } label: { Image(systemName: "arrow.clockwise") }.help("Reload from this session's history")
                Button("Done") { dismiss() }.keyboardShortcut(.defaultAction)
            }.padding()
            Divider()
            ScrollView {
                VStack(alignment: .leading, spacing: 16) {
                    summary(d)
                    if d.problems.isEmpty {
                        ContentUnavailableView("No TLS problems", systemImage: "checkmark.shield",
                                               description: Text("Every client that connected accepted NetProxy's certificate."))
                            .frame(maxWidth: .infinity)
                    } else {
                        Text("Clients that refused NetProxy's certificate").font(.headline)
                        ForEach(d.problems) { p in problem(p) }
                    }
                    coverage
                }.padding()
            }
        }
        .frame(width: 640, height: 600)
        .task { await model.refreshDiagnostics() }
    }

    private func summary(_ d: Diagnostics) -> some View {
        HStack(spacing: 12) {
            stat("\(d.connections)", "connections")
            stat("\(d.problems.reduce(0) { $0 + $1.count })", "TLS refusals")
            stat("\(d.silentConnections)", "connections with no request")
            stat("\(d.droppedEvents)", "events dropped", warn: d.droppedEvents > 0)
        }
    }

    private func stat(_ value: String, _ label: String, warn: Bool = false) -> some View {
        VStack(alignment: .leading) {
            Text(value).font(.title.weight(.semibold)).foregroundStyle(warn ? Color.orange : Color.primary)
            Text(label).font(.caption).foregroundStyle(.secondary)
        }
        .padding(12).frame(maxWidth: .infinity, alignment: .leading)
        .background(.quaternary.opacity(0.4), in: RoundedRectangle(cornerRadius: 10))
    }

    private func problem(_ p: TLSProblem) -> some View {
        let e = Explain.clientTLS(p.lastClass)
        return VStack(alignment: .leading, spacing: 6) {
            HStack {
                Text(p.host).font(.headline)
                Pill(text: "\(p.count)×", color: .red)
                Spacer()
                Text(p.clients.sorted().joined(separator: ", ")).font(.caption).foregroundStyle(.secondary)
            }
            Text(e.title).fontWeight(.medium)
            Text(e.hint).font(.callout).foregroundStyle(.secondary)
            Text(p.lastDetail).font(.caption2.monospaced()).foregroundStyle(.tertiary).textSelection(.enabled)
        }
        .padding(12).frame(maxWidth: .infinity, alignment: .leading)
        .background(Color.red.opacity(0.06), in: RoundedRectangle(cornerRadius: 10))
    }

    private var coverage: some View {
        VStack(alignment: .leading, spacing: 6) {
            Label("What NetProxy can't see", systemImage: "eye.slash").font(.headline)
            Text("• Apps that pin certificates, and Android apps that ignore user-installed certificates, refuse the connection — you'll see them above, never their contents.")
            Text("• Traffic that doesn't use this proxy (apps that ignore proxy settings), QUIC/HTTP/3 and non-HTTP protocols aren't captured at all.")
            Text("• A quiet list doesn't prove nothing happened: check this page when you expect traffic that isn't there.")
        }
        .font(.callout).foregroundStyle(.secondary)
        .padding(12).background(.quaternary.opacity(0.3), in: RoundedRectangle(cornerRadius: 10))
    }
}
