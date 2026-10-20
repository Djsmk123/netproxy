import SwiftUI
import NetProxyKit

struct BreakpointsView: View {
    @Environment(\.dismiss) private var dismiss
    @Environment(CaptureModel.self) private var model
    @State private var breakpoints: [Breakpoint] = []
    @State private var showAddBreakpoint = false
    @State private var newPattern = ""
    @State private var isLoading = false
    @State private var errorMessage: String?

    var body: some View {
        VStack(spacing: 0) {
            // Header
            HStack {
                Text("Request Breakpoints").font(.title2).fontWeight(.semibold)
                Spacer()
                Button("Done") { dismiss() }
            }
            .padding()

            Divider()

            // Content
            VStack(spacing: 0) {
                if breakpoints.isEmpty {
                    ContentUnavailableView {
                        Label("No Breakpoints", systemImage: "pause.circle")
                    } description: {
                        Text("Add breakpoints to pause requests matching URL patterns")
                    } actions: {
                        Button("Add Breakpoint") {
                            showAddBreakpoint = true
                        }
                        .buttonStyle(.borderedProminent)
                    }
                } else {
                    List {
                        ForEach(breakpoints) { bp in
                            BreakpointRow(breakpoint: bp) {
                                Task { await toggleBreakpoint(bp) }
                            } onDelete: {
                                Task { await removeBreakpoint(bp) }
                            }
                        }
                    }
                    .listStyle(.inset)
                }
                
                if let error = errorMessage {
                    HStack {
                        Label(error, systemImage: "exclamationmark.triangle")
                            .foregroundStyle(.red)
                            .font(.caption)
                        Spacer()
                        Button("Dismiss") { errorMessage = nil }
                            .buttonStyle(.borderless)
                            .font(.caption)
                    }
                    .padding()
                    .background(.red.opacity(0.1))
                }
            }
        }
        .frame(minWidth: 500, minHeight: 400)
        .toolbar {
            ToolbarItem(placement: .primaryAction) {
                Button {
                    showAddBreakpoint = true
                } label: {
                    Label("Add Breakpoint", systemImage: "plus")
                }
            }
            ToolbarItem {
                Button {
                    Task { await loadBreakpoints() }
                } label: {
                    Label("Reload", systemImage: "arrow.clockwise")
                }
                .disabled(isLoading)
            }
        }
        .task {
            await loadBreakpoints()
        }
        .sheet(isPresented: $showAddBreakpoint) {
            AddBreakpointView { pattern in
                Task { await addBreakpoint(pattern) }
            }
        }
    }

    private func loadBreakpoints() async {
        isLoading = true
        defer { isLoading = false }

        guard let client = model.client else { return }
        do {
            let response: BreakpointsResponse = try await client.call("breakpoints.list")
            breakpoints = response.breakpoints
        } catch {
            errorMessage = "Failed to load breakpoints: \(error.localizedDescription)"
        }
    }

    private func addBreakpoint(_ pattern: String) async {
        guard let client = model.client else { return }
        do {
            let params: [String: any Sendable] = ["pattern": pattern, "enabled": true]
            let bp: Breakpoint = try await client.call("breakpoints.add", params)
            breakpoints.append(bp)
            showAddBreakpoint = false
        } catch {
            errorMessage = "Failed to add breakpoint: \(error.localizedDescription)"
        }
    }

    private func toggleBreakpoint(_ bp: Breakpoint) async {
        guard let client = model.client else { return }
        do {
            let params: [String: any Sendable] = ["id": bp.id, "enabled": !bp.enabled]
            let updated: Breakpoint = try await client.call("breakpoints.toggle", params)
            if let index = breakpoints.firstIndex(where: { $0.id == bp.id }) {
                breakpoints[index] = updated
            }
        } catch {
            errorMessage = "Failed to toggle breakpoint: \(error.localizedDescription)"
        }
    }

    private func removeBreakpoint(_ bp: Breakpoint) async {
        guard let client = model.client else { return }
        do {
            let params: [String: any Sendable] = ["id": bp.id]
            try await client.callVoid("breakpoints.remove", params)
            breakpoints.removeAll { $0.id == bp.id }
        } catch {
            errorMessage = "Failed to remove breakpoint: \(error.localizedDescription)"
        }
    }
}

struct BreakpointRow: View {
    let breakpoint: Breakpoint
    let onToggle: () -> Void
    let onDelete: () -> Void

    var body: some View {
        HStack {
            Toggle(isOn: .constant(breakpoint.enabled)) {
                VStack(alignment: .leading, spacing: 4) {
                    Text(breakpoint.pattern)
                        .font(.body)
                    Text("Pauses requests matching this pattern")
                        .font(.caption)
                        .foregroundStyle(.secondary)
                }
            }
            .toggleStyle(.switch)
            .onChange(of: breakpoint.enabled) { _, _ in
                onToggle()
            }
            
            Spacer()
            
            Button(role: .destructive) {
                onDelete()
            } label: {
                Image(systemName: "trash")
                    .foregroundStyle(.red)
            }
            .buttonStyle(.borderless)
        }
        .padding(.vertical, 4)
    }
}

struct AddBreakpointView: View {
    @Environment(\.dismiss) private var dismiss
    @State private var pattern = ""
    let onAdd: (String) -> Void

    var body: some View {
        VStack(spacing: 0) {
            HStack {
                Text("Add Breakpoint").font(.headline)
                Spacer()
                Button("Cancel") { dismiss() }
            }
            .padding()
            
            Divider()
            
            VStack(alignment: .leading, spacing: 16) {
                VStack(alignment: .leading, spacing: 8) {
                    Text("URL Pattern")
                        .font(.subheadline)
                        .fontWeight(.medium)
                    
                    TextField("https://api.example.com/*", text: $pattern)
                        .textFieldStyle(.roundedBorder)
                    
                    Text("Use * for wildcards. Examples:")
                        .font(.caption)
                        .foregroundStyle(.secondary)
                    
                    VStack(alignment: .leading, spacing: 4) {
                        Text("• */api/* - Any URL containing /api/")
                        Text("• https://api.example.com/* - Specific host")
                        Text("• */users/*/profile - Specific path pattern")
                    }
                    .font(.caption)
                    .foregroundStyle(.secondary)
                }
                
                Spacer()
            }
            .padding()
            
            Divider()
            
            HStack {
                Spacer()
                Button("Add") {
                    onAdd(pattern)
                    dismiss()
                }
                .buttonStyle(.borderedProminent)
                .disabled(pattern.isEmpty)
            }
            .padding()
        }
        .frame(width: 450, height: 300)
    }
}

struct Breakpoint: Codable, Identifiable {
    let id: String
    let pattern: String
    let enabled: Bool
}

struct BreakpointsResponse: Codable {
    let breakpoints: [Breakpoint]
}
