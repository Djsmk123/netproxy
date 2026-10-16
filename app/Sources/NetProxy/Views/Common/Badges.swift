import SwiftUI

struct MethodBadge: View {
    let method: String

    var body: some View {
        Text(method)
            .font(.system(.caption, design: .monospaced))
            .padding(.horizontal, 6)
            .padding(.vertical, 2)
            .background(methodColor)
            .foregroundColor(.white)
            .cornerRadius(4)
    }

    var methodColor: Color {
        switch method.uppercased() {
        case "GET": return DesignSystem.Colors.methodGET
        case "POST": return DesignSystem.Colors.methodPOST
        case "PUT": return DesignSystem.Colors.methodPUT
        case "DELETE": return DesignSystem.Colors.methodDELETE
        case "PATCH": return .purple
        default: return .gray
        }
    }
}

struct StatusBadge: View {
    let status: Int

    var body: some View {
        Text("\(status)")
            .font(.system(.caption, design: .monospaced))
            .padding(.horizontal, 6)
            .padding(.vertical, 2)
            .background(statusColor)
            .foregroundColor(.white)
            .cornerRadius(4)
    }

    var statusColor: Color {
        switch status {
        case 200..<300: return DesignSystem.Colors.status2xx
        case 300..<400: return DesignSystem.Colors.status3xx
        case 400..<500: return DesignSystem.Colors.status4xx
        case 500..<600: return DesignSystem.Colors.status5xx
        default: return .gray
        }
    }
}
