import SwiftUI
import NetProxyKit

struct ContentView: View {
    @Environment(CaptureModel.self) private var model

    var body: some View {
        switch model.phase {
        case .starting:
            VStack(spacing: 12) { ProgressView(); Text("Starting capture engine…").foregroundStyle(.secondary) }
                .frame(maxWidth: .infinity, maxHeight: .infinity)
        case .failed(let message):
            ContentUnavailableView {
                Label("Capture engine unavailable", systemImage: "exclamationmark.triangle")
            } description: {
                Text(message)
            } actions: {
                Button("Try Again") { Task { await model.bootstrap() } }.keyboardShortcut(.defaultAction)
            }
        case .connected:
            MainView()
        }
    }
}

struct MainView: View {
    @Environment(CaptureModel.self) private var model
    @State private var showSetup = false
    @State private var showDiagnostics = false

    var body: some View {
        @Bindable var model = model
        NavigationSplitView {
            SidebarView()
                .navigationSplitViewColumnWidth(min: 190, ideal: 230, max: 320)
        } detail: {
            VSplitView {
                FlowListView()
                    .frame(minHeight: 200)
                InspectorView()
                    .frame(minHeight: 180, idealHeight: 320)
            }
        }
        .searchable(text: $model.filter.text, placement: .toolbar, prompt: "Filter URLs")
        .toolbar {
            ToolbarItemGroup(placement: .primaryAction) {
                CaptureButton()
                Button { showDiagnostics = true } label: {
                    Label("Diagnostics", systemImage: model.diagnostics.problems.isEmpty ? "stethoscope" : "exclamationmark.shield")
                }
                .help("Why can't some traffic be inspected?")
                .badge(model.diagnostics.problems.count)
                Button { showSetup = true } label: { Label("Device Setup", systemImage: "iphone.and.arrow.forward") }
                    .help("Connect a phone, simulator or emulator")
                Button { model.clearView() } label: { Label("Clear", systemImage: "trash") }
                    .help("Clear the list (captured data stays in the session)")
            }
        }
        #if DEBUG
        .task {
            if DemoHooks.sheet == "setup" { try? await Task.sleep(nanoseconds: 1_500_000_000); showSetup = true }
            if DemoHooks.sheet == "diagnostics" { try? await Task.sleep(nanoseconds: 1_500_000_000); showDiagnostics = true }
            if let want = DemoHooks.select {
                for _ in 0..<40 {
                    if let r = model.flows.first(where: { ($0.path ?? "").contains(want) && $0.state != "open" }) { model.selection = r.id; break }
                    try? await Task.sleep(nanoseconds: 250_000_000)
                }
            }
        }
        #endif
        .sheet(isPresented: $showSetup) { DeviceSetupView() }
        .sheet(isPresented: $showDiagnostics) { DiagnosticsView() }
        .overlay(alignment: .top) { Banners() }
    }
}

struct CaptureButton: View {
    @Environment(CaptureModel.self) private var model
    var body: some View {
        let running = model.status?.isRunning == true
        Button {
            Task { running ? await model.stopCapture() : await model.startCapture() }
        } label: {
            Label(running ? "Stop" : "Record", systemImage: running ? "stop.circle.fill" : "record.circle")
                .foregroundStyle(running ? Color.red : Color.primary)
        }
        .help(running ? "Stop capturing (⌘R)" : "Start capturing (⌘R)")
    }
}

/// Things the user must not miss: data loss, engine errors.
struct Banners: View {
    @Environment(CaptureModel.self) private var model
    var body: some View {
        VStack(spacing: 6) {
            if model.diagnostics.droppedEvents > 0 {
                banner("\(model.diagnostics.droppedEvents) events were dropped because the engine was overloaded. The list may be incomplete.", "exclamationmark.triangle.fill", .orange)
            }
            if let e = model.lastError {
                banner(e, "xmark.octagon.fill", .red)
            }
        }.padding(.top, 8).padding(.horizontal, 12)
    }

    func banner(_ text: String, _ icon: String, _ color: Color) -> some View {
        Label(text, systemImage: icon).font(.callout).padding(8)
            .frame(maxWidth: .infinity, alignment: .leading)
            .background(color.opacity(0.15), in: RoundedRectangle(cornerRadius: 8))
            .overlay(RoundedRectangle(cornerRadius: 8).stroke(color.opacity(0.4)))
    }
}
