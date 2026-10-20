import SwiftUI

struct AboutView: View {
    @Environment(\.dismiss) private var dismiss

    private var version: String {
        Bundle.main.infoDictionary?["CFBundleShortVersionString"] as? String ?? "1.0.0"
    }

    private var build: String {
        Bundle.main.infoDictionary?["CFBundleVersion"] as? String ?? "1"
    }

    var body: some View {
        VStack(spacing: 20) {
            // App icon
            Image(systemName: "network")
                .font(.system(size: 80))
                .foregroundStyle(.blue)

            // App name and version
            VStack(spacing: 4) {
                Text("NetProxy")
                    .font(.largeTitle)
                    .fontWeight(.semibold)

                Text("Version \(version) (\(build))")
                    .font(.subheadline)
                    .foregroundStyle(.secondary)
            }

            // Description
            Text("Native macOS HTTP/HTTPS proxy for network inspection, debugging, and API mocking")
                .font(.body)
                .foregroundStyle(.secondary)
                .multilineTextAlignment(.center)
                .padding(.horizontal)

            Divider()
                .padding(.vertical, 8)

            // Links
            VStack(spacing: 12) {
                Link(destination: URL(string: "https://github.com/djsmk123/netproxy")!) {
                    Label("GitHub Repository", systemImage: "arrow.up.forward.square")
                }

                Link(destination: URL(string: "https://github.com/djsmk123/netproxy/releases")!) {
                    Label("Release Notes", systemImage: "doc.text")
                }

                Link(destination: URL(string: "https://github.com/djsmk123/netproxy/issues")!) {
                    Label("Report Issue", systemImage: "exclamationmark.bubble")
                }
            }
            .font(.body)

            Spacer()

            // Credits
            VStack(spacing: 4) {
                Text("Built with Swift, Rust, and SwiftUI")
                    .font(.caption)
                    .foregroundStyle(.tertiary)

                Text("© 2024-2026 MD Mobin. MIT License.")
                    .font(.caption)
                    .foregroundStyle(.tertiary)
            }

            // Close button
            Button("Close") {
                dismiss()
            }
            .keyboardShortcut(.defaultAction)
            .padding(.top, 8)
        }
        .frame(width: 400, height: 500)
        .padding(24)
    }
}

#Preview {
    AboutView()
}
