import SwiftUI
import NetProxyKit

enum Style {
    static func statusColor(_ r: FlowRow) -> Color {
        if r.state == "failed" { return .red }
        if r.state == "aborted" { return .orange }
        if r.state == "open" { return .secondary }
        switch (r.status ?? 0) / 100 {
        case 2: return .green
        case 3: return .blue
        case 4: return .orange
        case 5: return .red
        default: return .secondary
        }
    }

    static func statusText(_ r: FlowRow) -> String {
        switch r.state {
        case "failed": return "Failed"
        case "aborted": return r.status.map(String.init) ?? "Aborted"
        case "open": return r.status.map(String.init) ?? "…"
        default: return r.status.map(String.init) ?? "—"
        }
    }

    static func methodColor(_ m: String?) -> Color {
        switch m?.uppercased() {
        case "GET": return .primary
        case "POST", "PUT", "PATCH": return .indigo
        case "DELETE": return .red
        default: return .secondary
        }
    }
}

struct Pill: View {
    var text: String
    var color: Color = .secondary
    var body: some View {
        Text(text).font(.caption2.weight(.semibold)).padding(.horizontal, 6).padding(.vertical, 2)
            .background(color.opacity(0.15), in: Capsule()).foregroundStyle(color)
    }
}

func copyToPasteboard(_ s: String) {
    NSPasteboard.general.clearContents()
    NSPasteboard.general.setString(s, forType: .string)
}
