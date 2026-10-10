import AppKit
import ApplicationServices

enum AccessibilityHelper {
    /// Check whether macOS Accessibility permissions are granted
    static func isTrusted() -> Bool {
        return AXIsProcessTrusted()
    }

    /// Triggers macOS native permission prompt (non-blocking)
    static func requestSystemPrompt() {
        let options = [kAXTrustedCheckOptionPrompt.takeUnretainedValue() as String: true] as CFDictionary
        let _ = AXIsProcessTrustedWithOptions(options)
    }

    /// Open System Settings directly into the Accessibility (辅助功能) sub-pane
    static func openAccessibilitySettings() {
        // 1. Try modern macOS (macOS 13, 14, 15+) Settings deep link
        if let url = URL(string: "x-apple.systempreferences:com.apple.settings.PrivacySecurity.extension?Privacy_Accessibility") {
            NSWorkspace.shared.open(url)
        } else if let url = URL(string: "x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility") {
            NSWorkspace.shared.open(url)
        }

        // 2. Also run AppleScript to reveal the exact sub-anchor inside Device Control and Data Access
        let script = """
        tell application "System Settings"
            activate
            try
                reveal anchor "Privacy_Accessibility" of pane id "com.apple.settings.PrivacySecurity.extension"
            end try
        end tell
        """
        if let appleScript = NSAppleScript(source: script) {
            var error: NSDictionary?
            appleScript.executeAndReturnError(&error)
        }
    }

    /// Check and guide the user
    static func checkAndPrompt(userInitiated: Bool = false) {
        if isTrusted() {
            if userInitiated {
                let alert = NSAlert()
                alert.messageText = L10n.t("✅ 辅助功能权限已启用", "✅ Accessibility permission is on")
                alert.informativeText = L10n.t("Apple TV Remote 已获得系统的辅助功能权限，鼠标光标滑动、点击与媒体按键均可正常工作。", "Apple TV Remote has Accessibility permission, so moving the pointer, clicking and the media keys all work.")
                alert.alertStyle = .informational
                alert.addButton(withTitle: L10n.t("确定", "OK"))
                alert.runModal()
            }
            return
        }

        // Non-intrusively trigger native OS prompt
        requestSystemPrompt()

        if userInitiated {
            let alert = NSAlert()
            alert.messageText = L10n.t("需要「辅助功能」权限", "Accessibility Permission Needed")
            alert.informativeText = L10n.isChinese ? """
Apple TV Remote 需要使用辅助功能模拟 Mac 鼠标指针滑动与键盘按键。

【系统页面名称说明】
在 macOS 15 (Sequoia) 及最新系统中，系统的辅助功能全面控制权限页面名称已更新为：
👉「Device Control and Data Access」（设备控制与数据访问）
此页面即为系统的核心控制权限页面，权限类别完全正确。

【如何开启与生效】
1. 点击下方「打开系统设置」按钮进入该页面；
2. 找到列表中的「AppleTVRemote」，将右侧开关开启为蓝色；
3. ⚠️ 重要提示：如果开关已经是开启状态但仍报未授权，是由于版本重编后系统底层签名缓存未同步，请将「AppleTVRemote」开关先关闭一次、再重新打开即可立即生效！
""" : """
Apple TV Remote uses Accessibility to move your Mac's pointer and press keys.

[Where to find it]
On macOS 15 (Sequoia) and later, the settings page for this permission may be named:
👉 "Device Control and Data Access"
That is the right page for this permission.

[How to turn it on]
1. Click "Open System Settings" below to go to that page.
2. Find "AppleTVRemote" in the list and turn its switch on (blue).
3. ⚠️ Important: if the switch is already on but the app still says it isn't allowed, macOS is holding on to the signature of an older build. Turn the "AppleTVRemote" switch off and on again, and it takes effect right away.
"""
            alert.alertStyle = .warning
            alert.addButton(withTitle: L10n.t("打开系统设置", "Open System Settings"))
            alert.addButton(withTitle: L10n.t("稍后", "Later"))

            let response = alert.runModal()
            if response == .alertFirstButtonReturn {
                openAccessibilitySettings()
            }
        }
    }
}
