import Foundation

/// Shared port configuration for the macOS host.
///
/// Reads the same settings file as the Rust core, so both sides always agree:
/// `.atv-settings.json` (cwd) -> `$XDG_CONFIG_HOME/atv/settings.json` -> `~/.config/atv/settings.json`.
///
/// Supported keys:
///   - `ui_port`  : Rust Debug Web UI / Inspector port (default 38765)
///   - `web_port` : this Swift diagnostics server port (default 38766)
///
/// Example:
/// ```json
/// { "mouse_speed": 0.50, "ui_port": 38765, "web_port": 38766 }
/// ```
enum AppConfig {
    static let defaultUIPort: UInt16 = 38765
    static let defaultWebPort: UInt16 = 38766

    static let uiPort: UInt16 = port(forKey: "ui_port") ?? defaultUIPort
    static let webPort: UInt16 = port(forKey: "web_port") ?? defaultWebPort

    static var settingsURL: URL {
        let fm = FileManager.default

        let local = URL(fileURLWithPath: fm.currentDirectoryPath)
            .appendingPathComponent(".atv-settings.json")
        if fm.fileExists(atPath: local.path) {
            return local
        }

        if let xdg = ProcessInfo.processInfo.environment["XDG_CONFIG_HOME"]?
            .trimmingCharacters(in: .whitespacesAndNewlines), !xdg.isEmpty {
            return URL(fileURLWithPath: xdg).appendingPathComponent("atv/settings.json")
        }

        return fm.homeDirectoryForCurrentUser.appendingPathComponent(".config/atv/settings.json")
    }

    private static func port(forKey key: String) -> UInt16? {
        guard let data = try? Data(contentsOf: settingsURL),
              let object = try? JSONSerialization.jsonObject(with: data) as? [String: Any] else {
            return nil
        }

        let value: Int?
        switch object[key] {
        case let number as NSNumber:
            value = number.intValue
        case let text as String:
            value = Int(text.trimmingCharacters(in: .whitespacesAndNewlines))
        default:
            value = nil
        }

        guard let port = value, (1...65_535).contains(port) else {
            return nil
        }
        return UInt16(port)
    }
}