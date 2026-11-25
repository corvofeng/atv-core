import AppKit

final class AppDelegate: NSObject, NSApplicationDelegate {
    func applicationDidFinishLaunching(_ notification: Notification) {
        // Run as menu bar accessory app (no dock icon clutter)
        NSApplication.shared.setActivationPolicy(.accessory)

        // Setup status item in system menu bar
        StatusItemManager.shared.setup()

        // Check accessibility permission on startup
        DispatchQueue.main.asyncAfter(deadline: .now() + 0.6) {
            AccessibilityHelper.checkAndPrompt(userInitiated: false)
        }

        // Start native Swift Debug Web Server on the configured port (default 38766)
        DebugWebServer.shared.start(port: AppConfig.webPort)

        // Start the atv-core Rust backend process if not already running
        ProcessManager.shared.startCoreIfNeeded { success in
            if success {
                print("✅ Apple TV Remote core service is ready.")
            } else {
                print("⚠️ Apple TV Remote core service could not be started.")
            }
            StatusItemManager.shared.refresh()
        }
    }

    func applicationWillTerminate(_ notification: Notification) {
        DebugWebServer.shared.stop()
        ProcessManager.shared.stopCore()
    }
}
