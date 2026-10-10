import AppKit
import Foundation
import UserNotifications

final class StatusItemManager: NSObject, NSMenuDelegate {
    static let shared = StatusItemManager()

    private var statusItem: NSStatusItem!
    private var menu: NSMenu!
    private var syncTimer: Timer?
    private var currentStatus: ServerStatus?
    private var isServiceRunning = false
    private var isBusy = false
    private var previousClientConnected = false

    private var showStatusTextInBar: Bool {
        get {
            if UserDefaults.standard.object(forKey: "ShowStatusTextInBar") == nil {
                return true // Default on
            }
            return UserDefaults.standard.bool(forKey: "ShowStatusTextInBar")
        }
        set {
            UserDefaults.standard.set(newValue, forKey: "ShowStatusTextInBar")
        }
    }

    override private init() {
        super.init()
    }

    func setup() {
        statusItem = NSStatusBar.system.statusItem(withLength: NSStatusItem.variableLength)
        menu = NSMenu()
        menu.delegate = self
        statusItem.menu = menu

        updateStatusItemUI(running: false, connected: false, busy: true)
        rebuildMenu()

        // Request local notification permissions
        UNUserNotificationCenter.current().requestAuthorization(options: [.alert, .sound]) { _, _ in }

        // Start background polling
        DispatchQueue.main.async {
            self.refresh()
            self.syncTimer = Timer.scheduledTimer(withTimeInterval: 2.0, repeats: true) { [weak self] _ in
                self?.refresh()
            }
        }
    }

    func updateStatusItemUI(running: Bool, connected: Bool, busy: Bool = false) {
        let size = NSSize(width: 18, height: 18)
        let img = NSImage(size: size)
        img.lockFocus()

        // Draw remote silhouette
        let remoteRect = NSRect(x: 4.5, y: 1, width: 9, height: 16)
        let remotePath = NSBezierPath(roundedRect: remoteRect, xRadius: 2.5, yRadius: 2.5)

        // Draw touchpad ring on remote
        let padRect = NSRect(x: 6, y: 10.5, width: 6, height: 5)
        let padPath = NSBezierPath(roundedRect: padRect, xRadius: 1.5, yRadius: 1.5)

        if busy {
            // Busy / transitioning state: Orange/Yellow dot
            NSColor.secondaryLabelColor.setStroke()
            remotePath.lineWidth = 1.3
            remotePath.stroke()

            NSColor.secondaryLabelColor.setFill()
            padPath.fill()

            let dot = NSBezierPath(ovalIn: NSRect(x: 12.5, y: 2, width: 5, height: 5))
            NSColor(red: 0.98, green: 0.70, blue: 0.15, alpha: 1.0).setFill()
            dot.fill()
        } else if !running {
            // Service stopped: Dimmed/gray silhouette with distinct red dot
            NSColor.secondaryLabelColor.setStroke()
            remotePath.lineWidth = 1.2
            remotePath.stroke()

            NSColor.secondaryLabelColor.setFill()
            padPath.fill()

            let dot = NSBezierPath(ovalIn: NSRect(x: 12.5, y: 2, width: 5, height: 5))
            NSColor(red: 0.95, green: 0.25, blue: 0.25, alpha: 1.0).setFill()
            dot.fill()
        } else if connected {
            // Running & iPhone connected: Glowing cyan/green with bright green badge
            NSColor(red: 0.20, green: 0.85, blue: 0.70, alpha: 1.0).setStroke()
            remotePath.lineWidth = 1.4
            remotePath.stroke()

            NSColor(red: 0.20, green: 0.85, blue: 0.70, alpha: 1.0).setFill()
            padPath.fill()

            let dot = NSBezierPath(ovalIn: NSRect(x: 12.5, y: 2, width: 5, height: 5))
            NSColor(red: 0.15, green: 0.90, blue: 0.50, alpha: 1.0).setFill()
            dot.fill()
        } else {
            // Running & waiting for iPhone: Clean white/template with blue dot
            NSColor.labelColor.setStroke()
            remotePath.lineWidth = 1.3
            remotePath.stroke()

            NSColor.labelColor.setFill()
            padPath.fill()

            let dot = NSBezierPath(ovalIn: NSRect(x: 12.5, y: 2, width: 5, height: 5))
            NSColor(red: 0.35, green: 0.65, blue: 0.95, alpha: 0.85).setFill()
            dot.fill()
        }

        img.unlockFocus()
        img.isTemplate = false
        statusItem.button?.image = img

        // Update status bar title text based on user preference
        if showStatusTextInBar {
            statusItem.button?.imagePosition = .imageLeading
            if busy {
                statusItem.button?.title = L10n.t(" ⏳处理中...", " ⏳Working...")
            } else if !running {
                statusItem.button?.title = L10n.t(" 已停止", " Stopped")
            } else if connected {
                statusItem.button?.title = L10n.t(" 已连接", " Connected")
            } else {
                statusItem.button?.title = L10n.t(" 运行中", " Running")
            }
        } else {
            statusItem.button?.title = ""
            statusItem.button?.imagePosition = .imageOnly
        }
    }

    func refresh() {
        guard !isBusy else { return }

        ApiClient.shared.fetchStatus { [weak self] status in
            DispatchQueue.main.async {
                guard let self = self else { return }
                self.currentStatus = status
                self.isServiceRunning = (status != nil)
                let connected = status?.client_connected ?? false

                self.updateStatusItemUI(running: self.isServiceRunning, connected: connected)

                // Notify user on connection state changes
                if self.isServiceRunning && (connected != self.previousClientConnected) {
                    self.previousClientConnected = connected
                    self.postConnectionNotification(connected: connected, peer: status?.client_peer)
                }

                self.rebuildMenu()
            }
        }
    }

    private func postConnectionNotification(connected: Bool, peer: String?) {
        let content = UNMutableNotificationContent()
        if connected {
            content.title = L10n.t("📱 iPhone 已连接", "📱 iPhone connected")
            content.body = L10n.t("已成功接入 Apple TV Remote，可通过控制中心触控板操控 Mac。", "Apple TV Remote is connected. Use the touch area in Control Center to control your Mac.")
            if let p = peer { content.subtitle = L10n.t("设备来自: \(p)", "From: \(p)") }
        } else {
            content.title = L10n.t("📱 iPhone 已断开连接", "📱 iPhone disconnected")
            content.body = L10n.t("Apple TV Remote 控制会话已结束。", "The Apple TV Remote session has ended.")
        }

        let request = UNNotificationRequest(identifier: UUID().uuidString, content: content, trigger: nil)
        UNUserNotificationCenter.current().add(request, withCompletionHandler: nil)
    }

    func menuNeedsUpdate(_ menu: NSMenu) {
        rebuildMenu()
    }

    func rebuildMenu() {
        menu.removeAllItems()

        let deviceName = currentStatus?.device_name ?? "Mac Remote"
        let isOnline = isServiceRunning
        let isConnected = currentStatus?.client_connected ?? false
        let isMouse = currentStatus?.mouse_mode ?? true
        let speed = currentStatus?.mouse_speed ?? 0.5
        let accel = currentStatus?.mouse_accel ?? true
        let verbose = currentStatus?.verbose_events ?? false

        // 1. App Title Header
        let titleItem = NSMenuItem(title: "Apple TV Remote", action: nil, keyEquivalent: "")
        titleItem.attributedTitle = NSAttributedString(
            string: "Apple TV Remote",
            attributes: [.font: NSFont.boldSystemFont(ofSize: 13)]
        )
        menu.addItem(titleItem)

        // 2. Status Information
        if isOnline {
            let serverItem = NSMenuItem(title: L10n.t("● 服务状态: 🟢 运行中 (\(deviceName))", "● Service: 🟢 Running (\(deviceName))"), action: nil, keyEquivalent: "")
            serverItem.isEnabled = false
            menu.addItem(serverItem)

            let clientText = isConnected
                ? L10n.t("📱 手机连接: 🟢 已连接 (\(currentStatus?.client_peer ?? "就绪"))", "📱 iPhone: 🟢 Connected (\(currentStatus?.client_peer ?? "ready"))")
                : L10n.t("📱 手机连接: 🟡 等待连接 (PIN: 1111)", "📱 iPhone: 🟡 Waiting to connect (PIN: 1111)")
            let clientItem = NSMenuItem(title: clientText, action: nil, keyEquivalent: "")
            clientItem.isEnabled = false
            menu.addItem(clientItem)
        } else {
            let serverItem = NSMenuItem(title: L10n.t("○ 服务状态: 🔴 已停止", "○ Service: 🔴 Stopped"), action: nil, keyEquivalent: "")
            serverItem.isEnabled = false
            menu.addItem(serverItem)

            let clientItem = NSMenuItem(title: L10n.t("📱 手机连接: 服务未启动", "📱 iPhone: service not running"), action: nil, keyEquivalent: "")
            clientItem.isEnabled = false
            menu.addItem(clientItem)
        }

        menu.addItem(NSMenuItem.separator())

        // 3. Service Control Actions (Start / Stop / Restart)
        if isOnline {
            let stopItem = NSMenuItem(title: L10n.t("⏸ 停止服务 (Stop)", "⏸ Stop Service"), action: #selector(onStopService), keyEquivalent: "s")
            stopItem.target = self
            menu.addItem(stopItem)

            let restartItem = NSMenuItem(title: L10n.t("🔄 重启服务 (Restart)", "🔄 Restart Service"), action: #selector(onRestartService), keyEquivalent: "")
            restartItem.target = self
            menu.addItem(restartItem)
        } else {
            let startItem = NSMenuItem(title: L10n.t("▶ 启动服务 (Start)", "▶ Start Service"), action: #selector(onStartService), keyEquivalent: "s")
            startItem.target = self
            menu.addItem(startItem)
        }

        menu.addItem(NSMenuItem.separator())

        // 4. Touchpad Controls (Active only when service is running - 3-way single choice radio)
        let modeHeader = NSMenuItem(title: L10n.t("触控板运行模式 (单选)", "Touch Area Mode"), action: nil, keyEquivalent: "")
        modeHeader.isEnabled = false
        menu.addItem(modeHeader)

        let curMode = currentStatus?.currentTrackpadMode ?? (isMouse ? "mouse" : "direction")

        let mouseItem = NSMenuItem(title: L10n.t("  🖱️ 鼠标光标模式 (Mouse)", "  🖱️ Mouse Pointer"), action: #selector(onSelectMouseMode), keyEquivalent: "")
        mouseItem.target = self
        mouseItem.state = (isOnline && curMode == "mouse") ? .on : .off
        mouseItem.isEnabled = isOnline
        menu.addItem(mouseItem)

        let dirItem = NSMenuItem(title: L10n.t("  ◀▲▼▶ 方向键模式 (D-pad)", "  ◀▲▼▶ Arrow Keys (D-pad)"), action: #selector(onSelectDirectionMode), keyEquivalent: "")
        dirItem.target = self
        dirItem.state = (isOnline && curMode == "direction") ? .on : .off
        dirItem.isEnabled = isOnline
        menu.addItem(dirItem)

        let idleItem = NSMenuItem(title: L10n.t("  👁️ 触控板空置模式 (仅看轨迹实时预览)", "  👁️ Idle (live preview only)"), action: #selector(onSelectIdleMode), keyEquivalent: "")
        idleItem.target = self
        idleItem.state = (isOnline && curMode == "idle") ? .on : .off
        idleItem.isEnabled = isOnline
        menu.addItem(idleItem)

        menu.addItem(NSMenuItem.separator())

        // 5. Sensitivity Speed Submenu
        let speedMenu = NSMenu()
        let presets: [(String, Double)] = [
            (L10n.t("🐢 0.5x 慢速精准 (默认)", "🐢 0.5x Slow and precise (default)"), 0.5),
            (L10n.t("🎯 1.0x 标准手感", "🎯 1.0x Standard"), 1.0),
            (L10n.t("⚡ 1.5x 快速顺畅", "⚡ 1.5x Fast and smooth"), 1.5),
            (L10n.t("🚀 2.0x 极速大屏", "🚀 2.0x Fastest, for large screens"), 2.0)
        ]
        for (label, val) in presets {
            let item = NSMenuItem(title: label, action: #selector(onSelectSpeedPreset(_:)), keyEquivalent: "")
            item.target = self
            item.representedObject = val
            item.state = (isOnline && abs(speed - val) < 0.08) ? .on : .off
            speedMenu.addItem(item)
        }

        let speedTitle = String(format: L10n.t("指针灵敏度 (当前 %.2fx)", "Pointer Speed (now %.2fx)"), speed)
        let speedSubItem = NSMenuItem(title: speedTitle, action: nil, keyEquivalent: "")
        speedSubItem.submenu = speedMenu
        speedSubItem.isEnabled = isOnline
        menu.addItem(speedSubItem)

        // 6. Dynamic Acceleration & Verbose Logs
        let accelItem = NSMenuItem(
            title: L10n.t("⚡ 动态人体工学加速度", "⚡ Dynamic Acceleration"),
            action: #selector(onToggleAccel),
            keyEquivalent: ""
        )
        accelItem.target = self
        accelItem.state = (isOnline && accel) ? .on : .off
        accelItem.isEnabled = isOnline
        menu.addItem(accelItem)

        let verboseItem = NSMenuItem(
            title: L10n.t("🔇 终端事件日志静默 (Debug)", "🔇 Quiet Event Log (Debug)"),
            action: #selector(onToggleVerbose),
            keyEquivalent: ""
        )
        verboseItem.target = self
        verboseItem.state = (isOnline && !verbose) ? .on : .off
        verboseItem.isEnabled = isOnline
        menu.addItem(verboseItem)

        menu.addItem(NSMenuItem.separator())

        // 7. Status Bar Display Settings & Utilities
        let showTextItem = NSMenuItem(
            title: L10n.t("在状态栏显示文字 (如 已连接/运行中)", "Show Status Text in Menu Bar (e.g. Connected/Running)"),
            action: #selector(onToggleStatusText),
            keyEquivalent: ""
        )
        showTextItem.target = self
        showTextItem.state = showStatusTextInBar ? .on : .off
        menu.addItem(showTextItem)

        let webItem = NSMenuItem(title: L10n.t("🌐 打开 Web 遥控控制台...", "🌐 Open Web Remote Console..."), action: #selector(onOpenWeb), keyEquivalent: "")
        webItem.target = self
        webItem.isEnabled = isOnline
        menu.addItem(webItem)

        let debugItem = NSMenuItem(title: L10n.t("🛠️ 打开 Web 系统日志 (/debug)...", "🛠️ Open Web System Log (/debug)..."), action: #selector(onOpenDebugWeb), keyEquivalent: "")
        debugItem.target = self
        debugItem.isEnabled = isOnline
        menu.addItem(debugItem)

        let logFileItem = NSMenuItem(title: L10n.t("📄 打开本地日志文件 (atv-core.log)...", "📄 Open Log File (atv-core.log)..."), action: #selector(onOpenLogFile), keyEquivalent: "")
        logFileItem.target = self
        menu.addItem(logFileItem)

        let permItem = NSMenuItem(title: L10n.t("🛡️ 检查辅助功能权限...", "🛡️ Check Accessibility Permission..."), action: #selector(onCheckPermissions), keyEquivalent: "")
        permItem.target = self
        menu.addItem(permItem)

        let refreshItem = NSMenuItem(title: L10n.t("🔄 刷新服务状态", "🔄 Refresh Service Status"), action: #selector(onRefreshManual), keyEquivalent: "r")
        refreshItem.target = self
        menu.addItem(refreshItem)

        menu.addItem(NSMenuItem.separator())

        // 8. Quit Application
        let quitItem = NSMenuItem(title: L10n.t("🚪 退出 Apple TV Remote", "🚪 Quit Apple TV Remote"), action: #selector(onQuit), keyEquivalent: "q")
        quitItem.target = self
        menu.addItem(quitItem)
    }

    // MARK: - Actions

    @objc private func onStartService() {
        isBusy = true
        updateStatusItemUI(running: false, connected: false, busy: true)
        rebuildMenu()

        ProcessManager.shared.startService { [weak self] success in
            DispatchQueue.main.async {
                self?.isBusy = false
                self?.isServiceRunning = success
                self?.refresh()

                let content = UNMutableNotificationContent()
                content.title = success ? L10n.t("✅ 服务已启动", "✅ Service started") : L10n.t("❌ 服务启动失败", "❌ Service failed to start")
                content.body = success
                    ? L10n.t("Apple TV Remote 服务现已在后台运行。", "The Apple TV Remote service is now running in the background.")
                    : L10n.t("未能启动 atv-core 后台服务，请检查端口是否被占用。", "Couldn't start the atv-core background service. Check whether its port is already in use.")
                let req = UNNotificationRequest(identifier: UUID().uuidString, content: content, trigger: nil)
                UNUserNotificationCenter.current().add(req, withCompletionHandler: nil)
            }
        }
    }

    @objc private func onStopService() {
        isBusy = true
        updateStatusItemUI(running: false, connected: false, busy: true)
        rebuildMenu()

        ProcessManager.shared.stopService { [weak self] in
            DispatchQueue.main.async {
                self?.isBusy = false
                self?.isServiceRunning = false
                self?.currentStatus = nil
                self?.updateStatusItemUI(running: false, connected: false, busy: false)
                self?.rebuildMenu()

                let content = UNMutableNotificationContent()
                content.title = L10n.t("⏸ 服务已停止", "⏸ Service stopped")
                content.body = L10n.t("Apple TV Remote 后台服务已关闭，端口已释放。", "The Apple TV Remote background service has stopped and freed its port.")
                let req = UNNotificationRequest(identifier: UUID().uuidString, content: content, trigger: nil)
                UNUserNotificationCenter.current().add(req, withCompletionHandler: nil)
            }
        }
    }

    @objc private func onRestartService() {
        isBusy = true
        updateStatusItemUI(running: false, connected: false, busy: true)
        rebuildMenu()

        ProcessManager.shared.restartService { [weak self] success in
            DispatchQueue.main.async {
                self?.isBusy = false
                self?.isServiceRunning = success
                self?.refresh()

                let content = UNMutableNotificationContent()
                content.title = success ? L10n.t("🔄 服务重启成功", "🔄 Service restarted") : L10n.t("❌ 服务重启失败", "❌ Service failed to restart")
                content.body = success
                    ? L10n.t("Apple TV Remote 服务已重新启动。", "The Apple TV Remote service has restarted.")
                    : L10n.t("服务未能成功重启，请检查日志。", "The service didn't restart. Check the log.")
                let req = UNNotificationRequest(identifier: UUID().uuidString, content: content, trigger: nil)
                UNUserNotificationCenter.current().add(req, withCompletionHandler: nil)
            }
        }
    }

    @objc private func onToggleStatusText() {
        showStatusTextInBar = !showStatusTextInBar
        let connected = currentStatus?.client_connected ?? false
        updateStatusItemUI(running: isServiceRunning, connected: connected)
        rebuildMenu()
    }

    @objc private func onSelectMouseMode() {
        ApiClient.shared.updateSettings(mode: "mouse") { [weak self] _ in
            self?.refresh()
        }
    }

    @objc private func onSelectDirectionMode() {
        ApiClient.shared.updateSettings(mode: "direction") { [weak self] _ in
            self?.refresh()
        }
    }

    @objc private func onSelectIdleMode() {
        ApiClient.shared.updateSettings(mode: "idle") { [weak self] _ in
            self?.refresh()
        }
    }

    @objc private func onSelectSpeedPreset(_ sender: NSMenuItem) {
        if let speedVal = sender.representedObject as? Double {
            ApiClient.shared.updateSettings(speed: speedVal) { [weak self] _ in
                self?.refresh()
            }
        }
    }

    @objc private func onToggleAccel() {
        let current = currentStatus?.mouse_accel ?? true
        ApiClient.shared.updateSettings(accel: !current) { [weak self] _ in
            self?.refresh()
        }
    }

    @objc private func onToggleVerbose() {
        let current = currentStatus?.verbose_events ?? false
        ApiClient.shared.updateSettings(verbose: !current) { [weak self] _ in
            self?.refresh()
        }
    }

    @objc private func onOpenWeb() {
        ApiClient.shared.openWebDashboard()
    }

    @objc private func onOpenDebugWeb() {
        ApiClient.shared.openDebugPage()
    }

    @objc private func onOpenLogFile() {
        ProcessManager.shared.openLogFile()
    }

    @objc private func onCheckPermissions() {
        AccessibilityHelper.checkAndPrompt(userInitiated: true)
    }

    @objc private func onRefreshManual() {
        refresh()
    }

    @objc private func onQuit() {
        NSApplication.shared.terminate(nil)
    }
}
