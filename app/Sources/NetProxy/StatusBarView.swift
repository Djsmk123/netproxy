import SwiftUI
import NetProxyKit

struct StatusBarView: View {
    @Environment(CaptureModel.self) private var model
    var throttlingStatus: String?
    var breakpointsEnabled: Bool = false

    var body: some View {
        HStack(spacing: 16) {
            // Capture status
            HStack(spacing: 4) {
                Circle()
                    .fill(model.status?.isRunning == true ? Color.red : Color.secondary.opacity(0.5))
                    .frame(width: 8, height: 8)
                Text(model.status?.isRunning == true ? "Recording" : "Stopped")
                    .font(.caption)
                    .foregroundStyle(.secondary)
            }

            Divider().frame(height: 12)

            // Flow count
            HStack(spacing: 4) {
                Image(systemName: "arrow.left.arrow.right")
                    .font(.caption)
                    .foregroundStyle(.secondary)
                Text("\(model.flows.count) flow\(model.flows.count == 1 ? "" : "s")")
                    .font(.caption)
                    .monospacedDigit()
                    .foregroundStyle(.secondary)
            }

            // Events seen
            if model.eventsSeen > 0 {
                Divider().frame(height: 12)
                HStack(spacing: 4) {
                    Image(systemName: "gauge")
                        .font(.caption)
                        .foregroundStyle(.secondary)
                    Text("\(model.eventsSeen) events")
                        .font(.caption)
                        .monospacedDigit()
                        .foregroundStyle(.secondary)
                }
            }

            // Storage
            if let storage = model.storage {
                Divider().frame(height: 12)
                HStack(spacing: 4) {
                    Image(systemName: "internaldrive")
                        .font(.caption)
                        .foregroundStyle(.secondary)
                    Text(Format.bytes(Int64(storage.usedBytes)))
                        .font(.caption)
                        .monospacedDigit()
                        .foregroundStyle(.secondary)
                    if let max = storage.maxBytes {
                        Text("/ \(Format.bytes(Int64(max)))")
                            .font(.caption)
                            .monospacedDigit()
                            .foregroundStyle(.tertiary)
                    }
                }
            }

            // Proxy address
            if let listen = model.status?.listen, model.status?.isRunning == true {
                Divider().frame(height: 12)
                HStack(spacing: 4) {
                    Image(systemName: "network")
                        .font(.caption)
                        .foregroundStyle(.secondary)
                    Text("Proxy: \(listen)")
                        .font(.caption.monospaced())
                        .foregroundStyle(.secondary)
                        .textSelection(.enabled)
                }
            }

            Spacer()

            // Throttling indicator
            if let status = throttlingStatus {
                HStack(spacing: 4) {
                    Image(systemName: "tortoise.fill")
                        .font(.caption)
                        .foregroundStyle(.orange)
                    Text(status)
                        .font(.caption)
                        .foregroundStyle(.orange)
                }
            }

            // Breakpoints indicator
            if breakpointsEnabled {
                HStack(spacing: 4) {
                    Image(systemName: "pause.circle.fill")
                        .font(.caption)
                        .foregroundStyle(.purple)
                    Text("Breakpoints")
                        .font(.caption)
                        .foregroundStyle(.purple)
                }
            }

            // Filter indicator
            if model.filter.isActive {
                HStack(spacing: 4) {
                    Image(systemName: "line.3.horizontal.decrease.circle.fill")
                        .font(.caption)
                        .foregroundStyle(.blue)
                    Text("Filtered")
                        .font(.caption)
                        .foregroundStyle(.blue)
                }
            }
        }
        .padding(.horizontal, 12)
        .padding(.vertical, 4)
        .background(.bar)
    }
}
