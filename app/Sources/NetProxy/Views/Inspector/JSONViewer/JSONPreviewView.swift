import SwiftUI

struct JSONPreviewView: View {
    let root: JSONNode

    var body: some View {
        VStack(spacing: 0) {
            // Header
            HStack {
                Text("Preview")
                    .font(.caption.weight(.semibold))
                    .foregroundColor(.secondary)

                Spacer()

                Text("Schema-based preview (coming soon)")
                    .font(.caption)
                    .foregroundStyle(.tertiary)
            }
            .padding(.horizontal, 10)
            .padding(.vertical, 5)
            .background(.bar)

            Divider()

            // Content
            ContentUnavailableView(
                "Preview Mode",
                systemImage: "doc.text.magnifyingglass",
                description: Text("Formatted preview for common schemas will be available in a future update. For now, use Tree or Raw view.")
            )
        }
    }
}
