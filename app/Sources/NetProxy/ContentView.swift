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
    @State private var showSessionManagement = false
    @State private var showThrottling = false
    @State private var showBreakpoints = false
    @State private var showExport = false
    @State private var showClearConfirmation = false
    @State private var breakpointsEnabled = false
    @State private var throttlingPreset: ThrottlePreset = .none
    @State private var showToast = false
    @State private var toastMessage = ""
    @State private var toastIcon = "checkmark.circle.fill"

    var body: some View {
        @Bindable var model = model
        VStack(spacing: 0) {
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
            .searchable(text: $model.filter.text, placement: .toolbar, prompt: "Filter URLs, hosts, or paths")
            .toolbar {
                ToolbarItemGroup(placement: .primaryAction) {
                    CaptureButton()

                    Button {
                        showClearConfirmation = true
                    } label: {
                        Label("Clear", systemImage: "trash")
                    }
                    .help("Clear current view")
                    .disabled(model.flows.isEmpty)

                    // Simplified throttling dropdown
                    Menu {
                        ForEach(ThrottlePreset.allCases, id: \.self) { preset in
                            Button {
                                throttlingPreset = preset
                                if preset != .none {
                                    showToast(message: "Throttling: \(preset.displayName)", icon: "speedometer")
                                } else {
                                    showToast(message: "Throttling disabled", icon: "checkmark.circle.fill")
                                }
                            } label: {
                                HStack {
                                    Text(preset.displayName)
                                    if throttlingPreset == preset {
                                        Image(systemName: "checkmark")
                                    }
                                }
                            }
                        }
                    } label: {
                        Label("Throttling", systemImage: "speedometer")
                    }
                    .help("Network throttling")

                    // Simplified breakpoints toggle
                    Toggle(isOn: $breakpointsEnabled) {
                        Label("Breakpoints", systemImage: breakpointsEnabled ? "pause.circle.fill" : "pause.circle")
                    }
                    .help("Enable breakpoints")
                    .onChange(of: breakpointsEnabled) { _, enabled in
                        showToast(message: enabled ? "Breakpoints enabled" : "Breakpoints disabled",
                                 icon: enabled ? "pause.circle.fill" : "checkmark.circle.fill")
                    }

                    Button {
                        showExport = true
                    } label: {
                        Label("Export", systemImage: "square.and.arrow.up")
                    }
                    .help("Export HAR or other formats (⌘⇧E)")
                    .keyboardShortcut("e", modifiers: [.command, .shift])
                    .disabled(model.flows.isEmpty)

                    Button { showDiagnostics = true } label: {
                        Label("Diagnostics", systemImage: model.diagnostics.problems.isEmpty ? "stethoscope" : "exclamationmark.shield")
                    }
                    .help("Why can't some traffic be inspected?")
                    .badge(model.diagnostics.problems.count)

                    Button { showSetup = true } label: {
                        Label("Device Setup", systemImage: "iphone.and.arrow.forward")
                    }
                    .help("Connect a phone, simulator or emulator")
                }
            }

            StatusBarView(
                throttlingStatus: throttlingPreset != .none ? throttlingPreset.rawValue : nil,
                breakpointsEnabled: breakpointsEnabled
            )
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
        .sheet(isPresented: $showExport) { ExportView() }
        .confirmationDialog(
            "Delete Session Data?",
            isPresented: $showClearConfirmation,
            titleVisibility: .visible
        ) {
            Button("Delete \(model.flows.count) flows permanently", role: .destructive) {
                let count = model.flows.count
                Task {
                    await model.deleteCurrentSession()
                    showToast(message: "Deleted \(count) flows from database", icon: "checkmark.circle.fill")
                }
            }
            Button("Cancel", role: .cancel) {}
        } message: {
            Text("This will permanently delete all \(model.flows.count) flows and events from this session. This cannot be undone.")
        }
        .overlay(alignment: .top) { Banners() }
        .overlay(alignment: .bottom) {
            if showToast {
                ToastView(message: toastMessage, icon: toastIcon)
                    .transition(.move(edge: .bottom).combined(with: .opacity))
                    .padding(.bottom, 50)
            }
        }
        // Hidden buttons for keyboard shortcuts
        .background {
            Button("Toggle Breakpoints") {
                breakpointsEnabled.toggle()
            }
            .keyboardShortcut("b", modifiers: [.command, .shift])
            .hidden()
        }
    }

    private func showToast(message: String, icon: String) {
        toastMessage = message
        toastIcon = icon
        withAnimation(.spring(response: 0.3)) {
            showToast = true
        }
        Task {
            try? await Task.sleep(nanoseconds: 2_000_000_000)
            withAnimation(.spring(response: 0.3)) {
                showToast = false
            }
        }
    }
}

struct CaptureButton: View {
    @Environment(CaptureModel.self) private var model
    var body: some View {
        let running = model.status?.isRunning == true
        Button {
            Task { running ? await model.stopCapture() : await model.startCapture(allowDevices: model.listenAllInterfaces) }
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
            if let e = model.lastError, !e.contains("already running") {
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

// MARK: - Throttle Presets

enum ThrottlePreset: String, CaseIterable {
    case none = "No Throttling"
    case threeG = "3G (750 KB/s)"
    case fourG = "4G (4 MB/s)"
    case wifi = "WiFi (20 MB/s)"
    case custom = "Custom..."

    var displayName: String { rawValue }

    var downloadSpeed: Int {
        switch self {
        case .none: return 0
        case .threeG: return 750
        case .fourG: return 4096
        case .wifi: return 20480
        case .custom: return 0
        }
    }

    var uploadSpeed: Int {
        switch self {
        case .none: return 0
        case .threeG: return 384
        case .fourG: return 2048
        case .wifi: return 10240
        case .custom: return 0
        }
    }
}

// MARK: - Toast View

struct ToastView: View {
    let message: String
    let icon: String

    var body: some View {
        HStack(spacing: 8) {
            Image(systemName: icon)
                .foregroundStyle(.green)
            Text(message)
                .font(.callout)
        }
        .padding(.horizontal, 16)
        .padding(.vertical, 10)
        .background(.regularMaterial, in: RoundedRectangle(cornerRadius: 10))
        .shadow(color: .black.opacity(0.1), radius: 8, y: 4)
    }
}
