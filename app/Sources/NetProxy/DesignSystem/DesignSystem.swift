import SwiftUI

enum DesignSystem {
    // Colors (Dark Mode)
    enum Colors {
        static let jsonString = Color(hex: "89D185")  // Green
        static let jsonNumber = Color(hex: "4FC1FF")  // Blue
        static let jsonBoolean = Color(hex: "C586C0") // Purple
        static let jsonNull = Color.gray
        static let jsonKey = Color.white

        static let methodGET = Color.blue
        static let methodPOST = Color.green
        static let methodPUT = Color.orange
        static let methodDELETE = Color.red

        static let status2xx = Color.green
        static let status3xx = Color.blue
        static let status4xx = Color.orange
        static let status5xx = Color.red
    }

    // Typography
    enum Typography {
        static let monospacedFont = Font.system(.body, design: .monospaced)
        static let monospacedCaptionFont = Font.system(.caption, design: .monospaced)
    }

    // Spacing
    enum Spacing {
        static let xs: CGFloat = 4
        static let sm: CGFloat = 8
        static let md: CGFloat = 12
        static let lg: CGFloat = 16
        static let xl: CGFloat = 24
    }
}

extension Color {
    init(hex: String) {
        let scanner = Scanner(string: hex)
        var rgb: UInt64 = 0
        scanner.scanHexInt64(&rgb)

        let r = Double((rgb >> 16) & 0xFF) / 255.0
        let g = Double((rgb >> 8) & 0xFF) / 255.0
        let b = Double(rgb & 0xFF) / 255.0

        self.init(red: r, green: g, blue: b)
    }
}
