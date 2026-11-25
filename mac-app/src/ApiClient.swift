import Foundation
import AppKit

struct ServerStatus: Codable {
    let status: String?
    let server_online: Bool?
    let mode: String?
    let mouse_mode: Bool?
    let device_name: String?
    let mouse_speed: Double?
    let mouse_accel: Bool?
    let verbose_events: Bool?
    let screen_size: [Int]?
    let client_connected: Bool?
    let client_peer: String?
    let session_ready: Bool?
    let audio: AudioState?

    var currentTrackpadMode: String {
        if let m = mode {
            if m == "idle" || m == "preview" { return "idle" }
            if m == "direction" { return "direction" }
            if m == "mouse" { return "mouse" }
        }
        return (mouse_mode == true) ? "mouse" : "direction"
    }

    struct AudioState: Codable {
        let volume: Double?
        let muted: Bool?
    }
}

final class ApiClient {
    static let shared = ApiClient()
    var baseUrl = "http://127.0.0.1:\(AppConfig.uiPort)"

    private let session: URLSession

    private init() {
        let config = URLSessionConfiguration.default
        config.timeoutIntervalForRequest = 2.0
        config.timeoutIntervalForResource = 3.0
        config.requestCachePolicy = .reloadIgnoringLocalCacheData
        self.session = URLSession(configuration: config)
    }

    func fetchStatus(completion: @escaping (ServerStatus?) -> Void) {
        guard let url = URL(string: "\(baseUrl)/api/status") else {
            completion(nil)
            return
        }

        var req = URLRequest(url: url)
        req.httpMethod = "GET"
        req.setValue("application/json", forHTTPHeaderField: "Accept")

        session.dataTask(with: req) { data, resp, err in
            guard let data = data, err == nil else {
                completion(nil)
                return
            }
            do {
                let status = try JSONDecoder().decode(ServerStatus.self, from: data)
                completion(status)
            } catch {
                completion(nil)
            }
        }.resume()
    }

    func updateSettings(
        mode: String? = nil,
        mouseMode: Bool? = nil,
        speed: Double? = nil,
        accel: Bool? = nil,
        verbose: Bool? = nil,
        completion: ((Bool) -> Void)? = nil
    ) {
        guard let url = URL(string: "\(baseUrl)/api/touchpad_settings") else {
            completion?(false)
            return
        }

        var dict: [String: Any] = [:]
        if let m = mode {
            dict["mode"] = m
            dict["mouse_mode"] = (m == "mouse")
        } else if let mm = mouseMode {
            dict["mode"] = mm ? "mouse" : "direction"
            dict["mouse_mode"] = mm
        }
        if let s = speed { dict["speed"] = s }
        if let a = accel { dict["accel"] = a }
        if let v = verbose { dict["verbose_events"] = v }

        guard let body = try? JSONSerialization.data(withJSONObject: dict) else {
            completion?(false)
            return
        }

        var req = URLRequest(url: url)
        req.httpMethod = "POST"
        req.setValue("application/json", forHTTPHeaderField: "Content-Type")
        req.httpBody = body

        session.dataTask(with: req) { _, resp, err in
            let success = (err == nil && (resp as? HTTPURLResponse)?.statusCode == 200)
            completion?(success)
        }.resume()
    }

    func openWebDashboard() {
        if let url = URL(string: baseUrl) {
            NSWorkspace.shared.open(url)
        }
    }

    func openDebugPage() {
        if let url = URL(string: "http://127.0.0.1:\(AppConfig.webPort)/") {
            NSWorkspace.shared.open(url)
        }
    }
}
