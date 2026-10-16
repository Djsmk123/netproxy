import SwiftUI

struct SearchBar: View {
    @Binding var text: String
    let placeholder: String
    let onSubmit: (() -> Void)?

    init(text: Binding<String>, placeholder: String = "Search...", onSubmit: (() -> Void)? = nil) {
        self._text = text
        self.placeholder = placeholder
        self.onSubmit = onSubmit
    }

    var body: some View {
        HStack {
            Image(systemName: "magnifyingglass").foregroundColor(.secondary)

            TextField(placeholder, text: $text)
                .textFieldStyle(.plain)
                .onSubmit { onSubmit?() }

            if !text.isEmpty {
                Button(action: { text = "" }) {
                    Image(systemName: "xmark.circle.fill")
                        .foregroundColor(.secondary)
                }
                .buttonStyle(.plain)
            }
        }
        .padding(DesignSystem.Spacing.sm)
        .background(Color.gray.opacity(0.1))
        .cornerRadius(8)
    }
}
