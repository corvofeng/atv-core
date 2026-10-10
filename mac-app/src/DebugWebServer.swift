import Foundation
import Network
import AppKit

final class DebugWebServer {
    static let shared = DebugWebServer()

    private var listener: NWListener?
    private let queue = DispatchQueue(label: "com.corvofeng.AppleTVRemote.DebugWebServer", qos: .userInitiated)
    private(set) var port: UInt16 = 8766
    private(set) var isRunning: Bool = false

    private init() {}

    func start(port: UInt16 = 8766) {
        guard !isRunning else { return }
        self.port = port

        do {
            let nwPort = NWEndpoint.Port(rawValue: port) ?? 8766
            let params = NWParameters.tcp
            params.allowLocalEndpointReuse = true
            // Diagnostics can restart the engine and empty the log, so they only answer on this Mac
            // (http://127.0.0.1), not to other devices on the network.
            params.requiredLocalEndpoint = NWEndpoint.hostPort(host: .ipv4(.loopback), port: nwPort)

            let l = try NWListener(using: params)
            l.newConnectionHandler = { [weak self] connection in
                self?.handleConnection(connection)
            }
            l.stateUpdateHandler = { state in
                switch state {
                case .ready:
                    print("🌐 [Swift Host] Debug Web Server listening on http://127.0.0.1:\(port)/")
                case .failed(let err):
                    print("⚠️ [Swift Host] Debug Web Server listener failed: \(err)")
                case .cancelled:
                    print("ℹ️ [Swift Host] Debug Web Server listener cancelled")
                default:
                    break
                }
            }
            l.start(queue: queue)
            self.listener = l
            self.isRunning = true
        } catch {
            print("❌ [Swift Host] Failed to start Debug Web Server on port \(port): \(error)")
        }
    }

    func stop() {
        listener?.cancel()
        listener = nil
        isRunning = false
    }

    private func handleConnection(_ connection: NWConnection) {
        connection.start(queue: queue)
        receiveRequest(connection: connection)
    }

    private func receiveRequest(connection: NWConnection) {
        connection.receive(minimumIncompleteLength: 1, maximumLength: 65536) { [weak self] data, _, _, error in
            guard let self = self, let data = data, error == nil,
                  let requestStr = String(data: data, encoding: .utf8) else {
                connection.cancel()
                return
            }

            self.dispatchRequest(requestStr: requestStr, connection: connection)
        }
    }

    private func dispatchRequest(requestStr: String, connection: NWConnection) {
        let firstLine = requestStr.components(separatedBy: "\r\n").first ?? ""
        let parts = firstLine.split(separator: " ")
        guard parts.count >= 2 else {
            sendResponse(connection: connection, statusCode: 400, statusText: "Bad Request", contentType: "text/plain", body: "Bad Request".data(using: .utf8)!)
            return
        }

        let method = String(parts[0])
        let rawUri = String(parts[1])
        let uriParts = rawUri.split(separator: "?", maxSplits: 1, omittingEmptySubsequences: false)
        let path = String(uriParts[0])
        let queryString = uriParts.count > 1 ? String(uriParts[1]) : ""

        switch (method, path) {
        case ("GET", "/"), ("GET", "/debug"), ("GET", "/index.html"):
            handleServeDebugPage(connection: connection)

        case ("GET", "/api/logs"):
            handleApiLogs(queryString: queryString, connection: connection)

        case ("GET", "/api/status"):
            handleApiStatus(connection: connection)

        case ("POST", "/api/restart"):
            handleApiRestart(connection: connection)

        case ("POST", "/api/clear_log"), ("POST", "/api/clear"):
            handleApiClearLog(connection: connection)

        default:
            sendResponse(connection: connection, statusCode: 404, statusText: "Not Found", contentType: "text/plain", body: "Not Found".data(using: .utf8)!)
        }
    }

    // MARK: - Handlers

    private func handleServeDebugPage(connection: NWConnection) {
        let htmlData = DebugWebPage.html.data(using: .utf8) ?? Data()
        sendResponse(
            connection: connection,
            statusCode: 200,
            statusText: "OK",
            contentType: "text/html; charset=utf-8",
            body: htmlData
        )
    }

    private func handleApiLogs(queryString: String, connection: NWConnection) {
        let params = parseQueryParams(queryString)
        let isDownload = params["download"] == "1" || params["download"] == "true"
        let isRaw = params["raw"] == "1" || params["raw"] == "true"

        let tailLimit: Int?
        if let tailStr = params["tail"] {
            if tailStr == "all" {
                tailLimit = nil
            } else {
                tailLimit = Int(tailStr) ?? 300
            }
        } else {
            tailLimit = 300
        }

        let logUrl = ProcessManager.shared.logFilePath
        let (lines, size, exists) = readLogFileTail(url: logUrl, limit: tailLimit)

        if isDownload {
            let joined = lines.joined(separator: "\n").data(using: .utf8) ?? Data()
            sendResponse(
                connection: connection,
                statusCode: 200,
                statusText: "OK",
                contentType: "text/plain; charset=utf-8",
                extraHeaders: ["Content-Disposition": "attachment; filename=\"atv-core.log\""],
                body: joined
            )
            return
        }

        if isRaw {
            let joined = lines.joined(separator: "\n").data(using: .utf8) ?? Data()
            sendResponse(
                connection: connection,
                statusCode: 200,
                statusText: "OK",
                contentType: "text/plain; charset=utf-8",
                body: joined
            )
            return
        }

        // Return JSON
        let dict: [String: Any] = [
            "status": "ok",
            "log_path": logUrl.path,
            "exists": exists,
            "size_bytes": size,
            "returned_lines": lines.count,
            "lines": lines
        ]

        if let jsonData = try? JSONSerialization.data(withJSONObject: dict, options: []) {
            sendResponse(
                connection: connection,
                statusCode: 200,
                statusText: "OK",
                contentType: "application/json; charset=utf-8",
                body: jsonData
            )
        } else {
            sendResponse(connection: connection, statusCode: 500, statusText: "Internal Error", contentType: "text/plain", body: Data())
        }
    }

    private func handleApiStatus(connection: NWConnection) {
        let logUrl = ProcessManager.shared.logFilePath
        let size = (try? FileManager.default.attributesOfItem(atPath: logUrl.path)[.size] as? UInt64) ?? 0
        let isRunning = ProcessManager.shared.isProcessAlive
        let pid = ProcessManager.shared.currentPID

        var dict: [String: Any] = [
            "status": "ok",
            "service_running": isRunning,
            "log_path": logUrl.path,
            "size_bytes": size,
            "swift_host_port": port,
            "core_api_port": 8765
        ]
        if let p = pid {
            dict["pid"] = p
        }

        let jsonData = (try? JSONSerialization.data(withJSONObject: dict, options: [])) ?? Data()
        sendResponse(
            connection: connection,
            statusCode: 200,
            statusText: "OK",
            contentType: "application/json; charset=utf-8",
            body: jsonData
        )
    }

    private func handleApiRestart(connection: NWConnection) {
        ProcessManager.shared.restartService { success in
            let dict: [String: Any] = [
                "status": success ? "ok" : "error",
                "message": success ? "Service restarted successfully" : "Failed to restart service"
            ]
            let jsonData = (try? JSONSerialization.data(withJSONObject: dict, options: [])) ?? Data()
            self.sendResponse(
                connection: connection,
                statusCode: success ? 200 : 500,
                statusText: success ? "OK" : "Error",
                contentType: "application/json; charset=utf-8",
                body: jsonData
            )
        }
    }

    private func handleApiClearLog(connection: NWConnection) {
        ProcessManager.shared.clearLogFile()
        let dict: [String: Any] = ["status": "ok", "message": "Log file cleared"]
        let jsonData = (try? JSONSerialization.data(withJSONObject: dict, options: [])) ?? Data()
        sendResponse(
            connection: connection,
            statusCode: 200,
            statusText: "OK",
            contentType: "application/json; charset=utf-8",
            body: jsonData
        )
    }

    // MARK: - Helpers

    private func parseQueryParams(_ query: String) -> [String: String] {
        var res: [String: String] = [:]
        for item in query.components(separatedBy: "&") {
            let pair = item.components(separatedBy: "=")
            if pair.count == 2 {
                res[pair[0]] = pair[1]
            } else if pair.count == 1 && !pair[0].isEmpty {
                res[pair[0]] = ""
            }
        }
        return res
    }

    private func readLogFileTail(url: URL, limit: Int?) -> (lines: [String], size: UInt64, exists: Bool) {
        let fm = FileManager.default
        guard fm.fileExists(atPath: url.path) else {
            return (["[Notice] Log file not found yet at: \(url.path)"], 0, false)
        }

        guard let attrs = try? fm.attributesOfItem(atPath: url.path),
              let fileSize = attrs[.size] as? UInt64 else {
            return ([], 0, false)
        }

        guard let fileHandle = try? FileHandle(forReadingFrom: url) else {
            return ([], fileSize, false)
        }
        defer { try? fileHandle.close() }

        // Read up to last 2MB for responsiveness
        let maxRead: UInt64 = 2 * 1024 * 1024
        let offset = fileSize > maxRead ? fileSize - maxRead : 0

        if offset > 0 {
            try? fileHandle.seek(toOffset: offset)
        }

        let data = fileHandle.readDataToEndOfFile()
        guard let text = String(data: data, encoding: .utf8) ?? String(data: data, encoding: .isoLatin1) else {
            return ([], fileSize, true)
        }

        var lines = text.components(separatedBy: "\n")
        if offset > 0 && !lines.isEmpty {
            lines.removeFirst() // Remove truncated initial line
        }

        if let l = limit, lines.count > l {
            lines = Array(lines.suffix(l))
        }

        return (lines, fileSize, true)
    }

    private func sendResponse(
        connection: NWConnection,
        statusCode: Int,
        statusText: String,
        contentType: String,
        extraHeaders: [String: String] = [:],
        body: Data
    ) {
        var headerStr = "HTTP/1.1 \(statusCode) \(statusText)\r\n"
        headerStr += "Content-Type: \(contentType)\r\n"
        headerStr += "Content-Length: \(body.count)\r\n"
        headerStr += "Access-Control-Allow-Origin: *\r\n"
        headerStr += "Connection: close\r\n"

        for (k, v) in extraHeaders {
            headerStr += "\(k): \(v)\r\n"
        }
        headerStr += "\r\n"

        var responseData = headerStr.data(using: .utf8) ?? Data()
        responseData.append(body)

        connection.send(content: responseData, completion: .contentProcessed({ _ in
            connection.cancel()
        }))
    }
}
