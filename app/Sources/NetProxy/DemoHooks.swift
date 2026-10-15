import SwiftUI
import AppKit
import NetProxyKit

#if DEBUG
/// Debug-only automation: preselect UI state and render the window to a PNG (no screen-recording
/// permission needed) so the UI can be reviewed and regression-checked headlessly.
///   NETPROXY_SELECT=<substring of a path>  NETPROXY_TAB=overview|request|response|timeline
///   NETPROXY_SHEET=setup|diagnostics        NETPROXY_SNAPSHOT=<png path> [NETPROXY_SNAPSHOT_DELAY=seconds]
enum DemoHooks {
    static let env = ProcessInfo.processInfo.environment
    static var select: String? { env["NETPROXY_SELECT"] }
    static var tab: String? { env["NETPROXY_TAB"] }
    static var sheet: String? { env["NETPROXY_SHEET"] }
    static var snapshotPath: String? { env["NETPROXY_SNAPSHOT"] }

    @MainActor static func scheduleSnapshot() {
        guard let path = snapshotPath else { return }
        let delay = Double(env["NETPROXY_SNAPSHOT_DELAY"] ?? "") ?? 6
        DispatchQueue.main.asyncAfter(deadline: .now() + delay) {
            guard let window = NSApp.windows.first(where: { $0.contentView != nil && $0.isVisible || $0.contentView != nil }) else { exit(3) }
            let target = window.attachedSheet ?? window
            guard let view = target.contentView,
                  let rep = view.bitmapImageRepForCachingDisplay(in: view.bounds) else { exit(4) }
            view.cacheDisplay(in: view.bounds, to: rep)
            if let png = rep.representation(using: .png, properties: [:]) { try? png.write(to: URL(fileURLWithPath: path)) }
            exit(0)
        }
    }
}
#endif
