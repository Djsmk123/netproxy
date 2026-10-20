import SwiftUI
import NetProxyKit

struct MockManagerView: View {
    @Environment(CaptureModel.self) private var model
    @Environment(\.dismiss) private var dismiss
    @State private var rules: [MockRule] = []
    @State private var selectedRule: MockRule?
    @State private var showAddRule = false
    @State private var showEditRule = false
    @State private var isLoading = false
    @State private var errorMessage: String?

    var body: some View {
        NavigationSplitView {
            // Rules list
            List(selection: $selectedRule) {
                Section("Mock Rules") {
                    if rules.isEmpty {
                        Text("No mock rules configured")
                            .foregroundStyle(.secondary)
                            .font(.subheadline)
                    } else {
                        ForEach(rules) { rule in
                            RuleRow(rule: rule)
                                .tag(rule)
                                .contextMenu {
                                    Button("Edit") {
                                        selectedRule = rule
                                        showEditRule = true
                                    }
                                    Button("Duplicate") {
                                        Task { await duplicateRule(rule) }
                                    }
                                    Divider()
                                    Button("Delete", role: .destructive) {
                                        Task { await deleteRule(rule) }
                                    }
                                }
                        }
                    }
                }
            }
            .navigationTitle("Mock Rules")
            .toolbar {
                ToolbarItem(placement: .primaryAction) {
                    Button {
                        showAddRule = true
                    } label: {
                        Label("Add Rule", systemImage: "plus")
                    }
                }
                ToolbarItem {
                    Button {
                        Task { await loadRules() }
                    } label: {
                        Label("Reload", systemImage: "arrow.clockwise")
                    }
                    .disabled(isLoading)
                }
            }
        } detail: {
            // Rule details
            if let rule = selectedRule {
                RuleDetailView(rule: rule, onUpdate: {
                    Task { await loadRules() }
                })
            } else {
                ContentUnavailableView(
                    "No Rule Selected",
                    systemImage: "doc.text",
                    description: Text("Select a rule to view details")
                )
            }
        }
        .frame(minWidth: 800, minHeight: 500)
        .task {
            await loadRules()
        }
        .sheet(isPresented: $showAddRule) {
            RuleEditorView(mode: .create) { newRule in
                Task { await addRule(newRule) }
            }
        }
        .sheet(isPresented: $showEditRule) {
            if let rule = selectedRule {
                RuleEditorView(mode: .edit(rule)) { updatedRule in
                    Task { await updateRule(updatedRule) }
                }
            }
        }
        .alert("Error", isPresented: .constant(errorMessage != nil), presenting: errorMessage) { _ in
            Button("OK") { errorMessage = nil }
        } message: { message in
            Text(message)
        }
    }

    private func loadRules() async {
        isLoading = true
        defer { isLoading = false }

        guard let client = model.client else { return }
        do {
            let response: MockRulesResponse = try await client.call("mocks.list")
            rules = response.rules
        } catch {
            errorMessage = "Failed to load rules: \(error.localizedDescription)"
        }
    }

    private func addRule(_ rule: MockRule) async {
        guard let client = model.client else { return }
        do {
            try await client.callVoid("mocks.add", ["rule": try encodeRule(rule)])
            await loadRules()
            showAddRule = false
        } catch {
            errorMessage = "Failed to add rule: \(error.localizedDescription)"
        }
    }

    private func updateRule(_ rule: MockRule) async {
        guard let client = model.client else { return }
        do {
            try await client.callVoid("mocks.update", ["rule": try encodeRule(rule)])
            await loadRules()
            showEditRule = false
        } catch {
            errorMessage = "Failed to update rule: \(error.localizedDescription)"
        }
    }

    private func deleteRule(_ rule: MockRule) async {
        guard let client = model.client else { return }
        do {
            try await client.callVoid("mocks.remove", ["id": rule.id])
            await loadRules()
            if selectedRule?.id == rule.id {
                selectedRule = nil
            }
        } catch {
            errorMessage = "Failed to delete rule: \(error.localizedDescription)"
        }
    }

    private func duplicateRule(_ rule: MockRule) async {
        var duplicate = rule
        duplicate.id = "\(rule.id)-copy-\(Int.random(in: 1000...9999))"
        duplicate.description = (rule.description ?? "") + " (Copy)"
        await addRule(duplicate)
    }

    private func encodeRule(_ rule: MockRule) throws -> [String: Any] {
        let data = try Wire.encoder.encode(rule)
        return try JSONSerialization.jsonObject(with: data) as! [String: Any]
    }
}

struct RuleRow: View {
    let rule: MockRule

    var body: some View {
        HStack {
            VStack(alignment: .leading, spacing: 4) {
                HStack {
                    Text(rule.id)
                        .font(.headline)
                    if !rule.enabled {
                        Text("Disabled")
                            .font(.caption)
                            .foregroundStyle(.secondary)
                            .padding(.horizontal, 6)
                            .padding(.vertical, 2)
                            .background(.secondary.opacity(0.2))
                            .clipShape(Capsule())
                    }
                }

                if let desc = rule.description {
                    Text(desc)
                        .font(.caption)
                        .foregroundStyle(.secondary)
                }

                HStack(spacing: 8) {
                    if let method = rule.matchSpec.method {
                        Label(method, systemImage: "arrow.right")
                            .font(.caption)
                            .foregroundStyle(.blue)
                    }
                    if let host = rule.matchSpec.host {
                        Label(host, systemImage: "globe")
                            .font(.caption)
                            .foregroundStyle(.green)
                    }
                    if let path = rule.matchSpec.path {
                        Label(path, systemImage: "doc")
                            .font(.caption)
                            .foregroundStyle(.orange)
                    }
                }
            }

            Spacer()

            Text("Priority \(rule.priority)")
                .font(.caption)
                .foregroundStyle(.secondary)
        }
        .opacity(rule.enabled ? 1.0 : 0.6)
    }
}

struct RuleDetailView: View {
    let rule: MockRule
    let onUpdate: () -> Void
    @Environment(CaptureModel.self) private var model
    @State private var isEnabled: Bool

    init(rule: MockRule, onUpdate: @escaping () -> Void) {
        self.rule = rule
        self.onUpdate = onUpdate
        self._isEnabled = State(initialValue: rule.enabled)
    }

    var body: some View {
        Form {
            Section("Rule Information") {
                LabeledContent("ID", value: rule.id)
                LabeledContent("Priority", value: String(rule.priority))
                if let desc = rule.description {
                    LabeledContent("Description", value: desc)
                }
                Toggle("Enabled", isOn: $isEnabled)
                    .onChange(of: isEnabled) { _, newValue in
                        Task { await toggleEnabled(newValue) }
                    }
            }

            Section("Match Criteria") {
                if let method = rule.matchSpec.method {
                    LabeledContent("Method", value: method)
                }
                if let host = rule.matchSpec.host {
                    LabeledContent("Host", value: host)
                }
                if let path = rule.matchSpec.path {
                    LabeledContent("Path", value: path)
                }
                if let regex = rule.matchSpec.pathRegex {
                    LabeledContent("Path Regex", value: regex)
                }
            }

            Section("Action") {
                LabeledContent("Type", value: rule.action.type)
                if case .respond(let details) = rule.action {
                    LabeledContent("Status", value: String(details.status))
                    if !details.headers.isEmpty {
                        LabeledContent("Headers") {
                            VStack(alignment: .leading) {
                                ForEach(Array(details.headers.keys.sorted()), id: \.self) { key in
                                    Text("\(key): \(details.headers[key] ?? "")")
                                        .font(.caption)
                                        .foregroundStyle(.secondary)
                                }
                            }
                        }
                    }
                    if let delay = details.delayMs {
                        LabeledContent("Delay", value: "\(delay) ms")
                    }
                }
            }
        }
        .formStyle(.grouped)
        .navigationTitle(rule.id)
    }

    private func toggleEnabled(_ enabled: Bool) async {
        var updated = rule
        updated.enabled = enabled

        guard let client = model.client else { return }
        do {
            let data = try Wire.encoder.encode(updated)
            let dict = try JSONSerialization.jsonObject(with: data) as! [String: Any]
            try await client.callVoid("mocks.update", ["rule": dict])
            onUpdate()
        } catch {
            isEnabled = !enabled // Revert on error
        }
    }
}

struct RuleEditorView: View {
    enum Mode {
        case create
        case edit(MockRule)
    }

    let mode: Mode
    let onSave: (MockRule) -> Void

    @Environment(\.dismiss) private var dismiss
    @State private var id: String = ""
    @State private var description: String = ""
    @State private var priority: Int = 100
    @State private var enabled: Bool = true
    @State private var method: String = ""
    @State private var host: String = ""
    @State private var path: String = ""
    @State private var status: Int = 200
    @State private var responseBody: String = ""

    var body: some View {
        NavigationStack {
            Form {
                Section("Basic Info") {
                    TextField("Rule ID", text: $id)
                    TextField("Description (optional)", text: $description)
                    Stepper("Priority: \(priority)", value: $priority, in: 1...999)
                    Toggle("Enabled", isOn: $enabled)
                }

                Section("Match Criteria") {
                    TextField("Method (e.g., GET, POST)", text: $method)
                    TextField("Host (e.g., api.example.com)", text: $host)
                    TextField("Path (e.g., /api/users)", text: $path)
                }

                Section("Response") {
                    Stepper("Status Code: \(status)", value: $status, in: 100...599)
                    TextEditor(text: $responseBody)
                        .frame(height: 150)
                        .font(.system(.body, design: .monospaced))
                }
            }
            .navigationTitle(mode.title)
            .toolbar {
                ToolbarItem(placement: .cancellationAction) {
                    Button("Cancel") { dismiss() }
                }
                ToolbarItem(placement: .confirmationAction) {
                    Button("Save") {
                        saveRule()
                    }
                    .disabled(id.isEmpty)
                }
            }
        }
        .frame(width: 600, height: 550)
        .onAppear {
            if case .edit(let rule) = mode {
                loadRule(rule)
            }
        }
    }

    private func loadRule(_ rule: MockRule) {
        id = rule.id
        description = rule.description ?? ""
        priority = rule.priority
        enabled = rule.enabled
        method = rule.matchSpec.method ?? ""
        host = rule.matchSpec.host ?? ""
        path = rule.matchSpec.path ?? ""

        if case .respond(let details) = rule.action {
            status = details.status
            if case .inline(let content) = details.body {
                responseBody = content
            }
        }
    }

    private func saveRule() {
        let rule = MockRule(
            id: id,
            priority: priority,
            enabled: enabled,
            description: description.isEmpty ? nil : description,
            matchSpec: MockMatchSpec(
                method: method.isEmpty ? nil : method,
                host: host.isEmpty ? nil : host,
                path: path.isEmpty ? nil : path,
                pathRegex: nil,
                query: nil,
                headers: nil
            ),
            action: .respond(MockRespondAction(
                status: status,
                headers: ["content-type": "application/json"],
                body: .inline(responseBody),
                delayMs: nil
            ))
        )

        onSave(rule)
        dismiss()
    }
}

extension RuleEditorView.Mode {
    var title: String {
        switch self {
        case .create: return "New Mock Rule"
        case .edit: return "Edit Mock Rule"
        }
    }
}

#Preview {
    MockManagerView()
        .environment(CaptureModel(supervisor: DaemonLocator.supervisor()))
}
