import Darwin
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
/// The menu bar can edit both at runtime; changes are written back to the
/// settings file and picked up by the Rust core on its next start.
///
/// Example:
/// ```json
/// { "mouse_speed": 0.50, "ui_port": 38765, "web_port": 38766 }
/// ```
enum AppConfig {
    static let defaultUIPort: UInt16 = 38765
    static let defaultWebPort: UInt16 = 38766

    private(set) static var uiPort: UInt16 = loadedPort(forKey: "ui_port") ?? defaultUIPort
    private(set) static var webPort: UInt16 = loadedPort(forKey: "web_port") ?? defaultWebPort

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

    /// Re-read the ports from disk.
    static func reload() {
        uiPort = loadedPort(forKey: "ui_port") ?? defaultUIPort
        webPort = loadedPort(forKey: "web_port") ?? defaultWebPort
    }

    /// Persist both ports, keeping every other key in the settings file intact.
    @discardableResult
    static func save(uiPort newUIPort: UInt16, webPort newWebPort: UInt16) throws -> URL {
        let url = settingsURL
        try FileManager.default.createDirectory(
            at: url.deletingLastPathComponent(),
            withIntermediateDirectories: true
        )

        var object: [String: Any] = [:]
        if let data = try? Data(contentsOf: url),
           let existing = (try? JSONSerialization.jsonObject(with: data)) as? [String: Any] {
            object = existing
        }

        object["ui_port"] = Int(newUIPort)
        object["web_port"] = Int(newWebPort)

        let data = try JSONSerialization.data(
            withJSONObject: object,
            options: [.prettyPrinted, .sortedKeys]
        )
        try data.write(to: url, options: .atomic)

        uiPort = newUIPort
        webPort = newWebPort
        return url
    }

    /// Best-effort check whether a TCP port can be bound on the wildcard address.
    /// A port that is already listening fails immediately; TIME_WAIT may cause a
    /// false negative, so callers should treat `false` as a warning, not a veto.
    static func isPortAvailable(_ port: UInt16) -> Bool {
        let fd = socket(AF_INET, SOCK_STREAM, 0)
        guard fd >= 0 else { return true }
        defer { close(fd) }

        var addr = sockaddr_in()
        addr.sin_len = UInt8(MemoryLayout<sockaddr_in>.size)
        addr.sin_family = sa_family_t(AF_INET)
        addr.sin_port = port.bigEndian
        addr.sin_addr = in_addr(s_addr: 0)

        let result = withUnsafePointer(to: &addr) { pointer in
            pointer.withMemoryRebound(to: sockaddr.self, capacity: 1) { sockaddrPointer in
                bind(fd, sockaddrPointer, socklen_t(MemoryLayout<sockaddr_in>.size))
            }
        }
        return result == 0
    }

    private static func loadedPort(forKey key: String) -> UInt16? {
        guard let data = try? Data(contentsOf: settingsURL),
              let object = (try? JSONSerialization.jsonObject(with: data)) as? [String: Any] else {
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