import SwiftUI
import NetProxyKit

struct PausedRequest: Codable, Identifiable {
    let id: String
    let flowId: UInt64
    let method: String
    let uri: String
    let host: String?
    let headers: [String: String]
    let bodyPreview: String?
    let breakpointId: String
    let pausedAt: Date

    enum CodingKeys: String, CodingKey {
        case id
        case flowId = "flow_id"
        case method
        case uri
        case host
        case headers
        case bodyPreview = "body_preview"
        case breakpointId = "breakpoint_id"
        case pausedAt = "paused_at"
    }
}

struct PausedRequestsResponse: Codable {
    let paused: [PausedRequest]
}

@MainActor
@Observable
class PausedRequestsViewModel {
    var pausedRequests: [PausedRequest] = []
    var isLoading = false
    var error: String?
    var selectedRequest: PausedRequest?
    var showingDetail = false

    private var pollTimer: Timer?

    func startPolling(client: ControlClient) {
        // Poll every second for paused requests
        pollTimer?.invalidate()
        pollTimer = Timer.scheduledTimer(withTimeInterval: 1.0, repeats: true) { [weak self] _ in
            Task { @MainActor in
                await self?.fetchPausedRequests(client: client)
            }
        }
        // Fetch immediately
        Task {
            await fetchPausedRequests(client: client)
        }
    }

    func stopPolling() {
        pollTimer?.invalidate()
        pollTimer = nil
    }

    func fetchPausedRequests(client: ControlClient) async {
        guard !isLoading else { return }
        isLoading = true
        error = nil

        do {
            let response: PausedRequestsResponse = try await client.call("breakpoints.paused", [:])
            pausedRequests = response.paused
        } catch {
            self.error = "Failed to fetch paused requests: \(error.localizedDescription)"
        }

        isLoading = false
    }

    func continueRequest(id: String, client: ControlClient) async {
        do {
            let params: [String: Any] = ["id": id]
            let _: [String: String] = try await client.call("breakpoints.continue", params)
            // Remove from list
            pausedRequests.removeAll { $0.id == id }
        } catch {
            self.error = "Failed to continue request: \(error.localizedDescription)"
        }
    }

    func cancelRequest(id: String, status: Int = 502, message: String = "Cancelled by user", client: ControlClient) async {
        do {
            let params: [String: Any] = [
                "id": id,
                "status": status,
                "message": message
            ]
            let _: [String: String] = try await client.call("breakpoints.cancel", params)
            // Remove from list
            pausedRequests.removeAll { $0.id == id }
        } catch {
            self.error = "Failed to cancel request: \(error.localizedDescription)"
        }
    }
}

struct PausedRequestsView: View {
    @Environment(\.dismiss) private var dismiss
    @State private var viewModel = PausedRequestsViewModel()
    let client: ControlClient

    var body: some View {
        VStack(spacing: 0) {
            // Header
            HStack {
                Image(systemName: "pause.circle.fill")
                    .foregroundColor(.orange)
                Text("Paused Requests")
                    .font(.headline)
                Spacer()
                if viewModel.isLoading {
                    ProgressView()
                        .scaleEffect(0.8)
                }
                Text("\(viewModel.pausedRequests.count)")
                    .foregroundColor(.secondary)
                    .font(.caption)
                    .padding(.horizontal, 8)
                    .padding(.vertical, 4)
                    .background(Color.secondary.opacity(0.2))
                    .cornerRadius(8)
            }
            .padding()
            .background(Color(nsColor: .controlBackgroundColor))

            Divider()

            // Content
            if viewModel.pausedRequests.isEmpty {
                emptyState
            } else {
                requestsList
            }

            // Error banner
            if let error = viewModel.error {
                errorBanner(error)
            }
        }
        .frame(minWidth: 600, minHeight: 400)
        .onAppear {
            viewModel.startPolling(client: client)
        }
        .onDisappear {
            viewModel.stopPolling()
        }
        .sheet(isPresented: $viewModel.showingDetail) {
            if let request = viewModel.selectedRequest {
                RequestDetailView(request: request, client: client, viewModel: viewModel)
            }
        }
    }

    private var emptyState: some View {
        VStack(spacing: 16) {
            Image(systemName: "hourglass")
                .font(.system(size: 48))
                .foregroundColor(.secondary)
            Text("No Paused Requests")
                .font(.title2)
                .foregroundColor(.secondary)
            Text("Requests that hit breakpoints will appear here")
                .font(.caption)
                .foregroundColor(.secondary)
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
    }

    private var requestsList: some View {
        ScrollView {
            LazyVStack(spacing: 8) {
                ForEach(viewModel.pausedRequests) { request in
                    PausedRequestRow(request: request, viewModel: viewModel, client: client)
                }
            }
            .padding()
        }
    }

    private func errorBanner(_ message: String) -> some View {
        HStack {
            Image(systemName: "exclamationmark.triangle.fill")
                .foregroundColor(.orange)
            Text(message)
                .font(.caption)
                .foregroundColor(.secondary)
            Spacer()
            Button("Dismiss") {
                viewModel.error = nil
            }
            .buttonStyle(.plain)
            .foregroundColor(.blue)
        }
        .padding(8)
        .background(Color.orange.opacity(0.1))
    }
}

struct PausedRequestRow: View {
    let request: PausedRequest
    let viewModel: PausedRequestsViewModel
    let client: ControlClient

    @State private var isExpanded = false

    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            // Header row
            HStack {
                // Method badge
                Text(request.method)
                    .font(.system(.caption, design: .monospaced).weight(.bold))
                    .foregroundColor(.white)
                    .padding(.horizontal, 8)
                    .padding(.vertical, 4)
                    .background(methodColor(request.method))
                    .cornerRadius(4)

                // URI
                Text(request.uri)
                    .font(.system(.body, design: .monospaced))
                    .lineLimit(1)
                    .truncationMode(.middle)

                Spacer()

                // Elapsed time
                Text(elapsedTime(from: request.pausedAt))
                    .font(.caption)
                    .foregroundColor(.orange)

                // Expand button
                Button {
                    withAnimation {
                        isExpanded.toggle()
                    }
                } label: {
                    Image(systemName: isExpanded ? "chevron.up" : "chevron.down")
                        .foregroundColor(.secondary)
                }
                .buttonStyle(.plain)
            }

            // Expanded details
            if isExpanded {
                VStack(alignment: .leading, spacing: 8) {
                    if let host = request.host {
                        LabeledContent("Host", value: host)
                            .font(.caption)
                    }

                    LabeledContent("Flow ID", value: String(request.flowId))
                        .font(.caption)

                    LabeledContent("Breakpoint", value: request.breakpointId)
                        .font(.caption)

                    // Headers
                    if !request.headers.isEmpty {
                        DisclosureGroup("Headers (\(request.headers.count))") {
                            VStack(alignment: .leading, spacing: 4) {
                                ForEach(Array(request.headers.keys.sorted()), id: \.self) { key in
                                    HStack(alignment: .top) {
                                        Text(key)
                                            .font(.system(.caption, design: .monospaced))
                                            .foregroundColor(.secondary)
                                            .frame(width: 120, alignment: .trailing)
                                        Text(request.headers[key] ?? "")
                                            .font(.system(.caption, design: .monospaced))
                                            .textSelection(.enabled)
                                    }
                                }
                            }
                            .padding(.leading)
                        }
                        .font(.caption)
                    }
                }
                .padding(.top, 4)
            }

            // Action buttons
            HStack(spacing: 12) {
                Button {
                    Task {
                        await viewModel.continueRequest(id: request.id, client: client)
                    }
                } label: {
                    Label("Continue", systemImage: "play.fill")
                }
                .buttonStyle(.borderedProminent)
                .tint(.green)

                Button {
                    Task {
                        await viewModel.cancelRequest(id: request.id, client: client)
                    }
                } label: {
                    Label("Cancel", systemImage: "xmark.circle.fill")
                }
                .buttonStyle(.bordered)
                .tint(.red)

                Button {
                    viewModel.selectedRequest = request
                    viewModel.showingDetail = true
                } label: {
                    Label("Inspect", systemImage: "doc.text.magnifyingglass")
                }
                .buttonStyle(.bordered)

                Spacer()
            }
            .padding(.top, 8)
        }
        .padding()
        .background(Color(nsColor: .controlBackgroundColor))
        .cornerRadius(8)
        .overlay(
            RoundedRectangle(cornerRadius: 8)
                .stroke(Color.orange.opacity(0.3), lineWidth: 1)
        )
    }

    private func methodColor(_ method: String) -> Color {
        switch method {
        case "GET": return .blue
        case "POST": return .green
        case "PUT": return .orange
        case "DELETE": return .red
        case "PATCH": return .purple
        default: return .gray
        }
    }

    private func elapsedTime(from date: Date) -> String {
        let elapsed = Date().timeIntervalSince(date)
        if elapsed < 60 {
            return String(format: "%.0fs ago", elapsed)
        } else if elapsed < 3600 {
            return String(format: "%.0fm ago", elapsed / 60)
        } else {
            return String(format: "%.1fh ago", elapsed / 3600)
        }
    }
}

struct RequestDetailView: View {
    let request: PausedRequest
    let client: ControlClient
    @Bindable var viewModel: PausedRequestsViewModel
    @Environment(\.dismiss) private var dismiss

    @State private var customStatus = 502
    @State private var customMessage = "Cancelled by user"

    var body: some View {
        VStack(spacing: 0) {
            // Header
            HStack {
                Text("Request Details")
                    .font(.headline)
                Spacer()
                Button("Close") {
                    dismiss()
                }
            }
            .padding()
            .background(Color(nsColor: .controlBackgroundColor))

            Divider()

            // Content
            ScrollView {
                VStack(alignment: .leading, spacing: 16) {
                    // Basic info
                    GroupBox("Request Info") {
                        VStack(alignment: .leading, spacing: 8) {
                            LabeledContent("Method", value: request.method)
                            LabeledContent("URI", value: request.uri)
                            if let host = request.host {
                                LabeledContent("Host", value: host)
                            }
                            LabeledContent("Flow ID", value: String(request.flowId))
                        }
                        .font(.system(.body, design: .monospaced))
                    }

                    // Headers
                    GroupBox("Headers") {
                        VStack(alignment: .leading, spacing: 4) {
                            ForEach(Array(request.headers.keys.sorted()), id: \.self) { key in
                                HStack(alignment: .top) {
                                    Text(key)
                                        .font(.system(.caption, design: .monospaced))
                                        .foregroundColor(.secondary)
                                        .frame(width: 150, alignment: .trailing)
                                    Text(request.headers[key] ?? "")
                                        .font(.system(.caption, design: .monospaced))
                                        .textSelection(.enabled)
                                }
                            }
                        }
                    }

                    // Cancel with custom response
                    GroupBox("Cancel with Custom Response") {
                        VStack(alignment: .leading, spacing: 12) {
                            HStack {
                                Text("Status Code:")
                                TextField("502", value: $customStatus, format: .number)
                                    .textFieldStyle(.roundedBorder)
                                    .frame(width: 80)
                            }

                            VStack(alignment: .leading) {
                                Text("Message:")
                                TextEditor(text: $customMessage)
                                    .font(.system(.body, design: .monospaced))
                                    .frame(height: 60)
                                    .border(Color.secondary.opacity(0.3))
                            }

                            Button {
                                Task {
                                    await viewModel.cancelRequest(
                                        id: request.id,
                                        status: customStatus,
                                        message: customMessage,
                                        client: client
                                    )
                                    dismiss()
                                }
                            } label: {
                                Label("Cancel with Custom Response", systemImage: "xmark.circle.fill")
                            }
                            .buttonStyle(.bordered)
                            .tint(.red)
                        }
                    }
                }
                .padding()
            }

            Divider()

            // Actions
            HStack {
                Spacer()

                Button {
                    Task {
                        await viewModel.continueRequest(id: request.id, client: client)
                        dismiss()
                    }
                } label: {
                    Label("Continue Request", systemImage: "play.fill")
                }
                .buttonStyle(.borderedProminent)
                .tint(.green)

                Button {
                    Task {
                        await viewModel.cancelRequest(id: request.id, client: client)
                        dismiss()
                    }
                } label: {
                    Label("Cancel (502)", systemImage: "xmark.circle.fill")
                }
                .buttonStyle(.bordered)
                .tint(.red)
            }
            .padding()
            .background(Color(nsColor: .controlBackgroundColor))
        }
        .frame(width: 600, height: 500)
    }
}

#Preview {
    @Previewable @State var client = ControlClient(socketPath: "/tmp/netproxy-preview", token: "preview-token")
    PausedRequestsView(client: client)
}
