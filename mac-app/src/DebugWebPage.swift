import Foundation

enum DebugWebPage {
    static let html = """
<!doctype html>
<html lang="zh-CN">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width,initial-scale=1">
  <title>Apple TV Remote - 运行日志与系统诊断 (Swift Host)</title>
  <style>
    :root {
      color-scheme: dark;
      --bg: #090b10;
      --panel: rgba(22, 26, 35, .85);
      --panel-solid: #161a23;
      --line: rgba(255, 255, 255, .1);
      --line-bright: rgba(255, 255, 255, .2);
      --muted: #8c96aa;
      --text: #f4f7ff;
      --blue: #5da9ff;
      --cyan: #70f1dc;
      --red: #ff6b7d;
      --green: #4caf50;
      --orange: #ffa726;
      --purple: #d19a66;
    }
    * { box-sizing: border-box; }
    body {
      margin: 0;
      min-height: 100vh;
      color: var(--text);
      background:
        radial-gradient(circle at 18% 0%, #172342 0, transparent 38%),
        radial-gradient(circle at 90% 15%, #132e30 0, transparent 35%),
        var(--bg);
      font: 14px/1.5 -apple-system, BlinkMacSystemFont, "SF Pro Text", "Segoe UI", Roboto, sans-serif;
    }
    main {
      width: min(1440px, calc(100% - 40px));
      margin: 0 auto;
      padding: 24px 0 40px;
    }
    header {
      display: flex;
      align-items: center;
      justify-content: space-between;
      margin-bottom: 20px;
      flex-wrap: wrap;
      gap: 14px;
    }
    .header-left {
      display: flex;
      align-items: center;
      gap: 16px;
    }
    .back-btn {
      display: inline-flex;
      align-items: center;
      gap: 6px;
      background: rgba(255, 255, 255, 0.08);
      border: 1px solid var(--line);
      color: var(--text);
      text-decoration: none;
      padding: 7px 14px;
      border-radius: 999px;
      font-size: 13px;
      font-weight: 500;
      transition: all .18s ease;
    }
    .back-btn:hover {
      background: rgba(255, 255, 255, 0.16);
      border-color: var(--line-bright);
      transform: translateY(-1px);
    }
    h1 {
      margin: 0;
      font-size: 24px;
      letter-spacing: -.03em;
      display: flex;
      align-items: center;
      gap: 10px;
    }
    .subtitle {
      margin-top: 3px;
      color: var(--muted);
      font-size: 13px;
    }
    .header-right {
      display: flex;
      align-items: center;
      gap: 10px;
      flex-wrap: wrap;
    }

    .badge {
      display: inline-flex;
      align-items: center;
      gap: 6px;
      font-size: 12px;
      font-weight: 600;
      padding: 5px 12px;
      border-radius: 999px;
      border: 1px solid var(--line);
      background: rgba(255, 255, 255, 0.05);
      color: var(--muted);
    }
    .badge.online {
      background: rgba(112, 241, 220, 0.12);
      border-color: rgba(112, 241, 220, 0.35);
      color: var(--cyan);
    }
    .badge.offline {
      background: rgba(255, 107, 125, 0.12);
      border-color: rgba(255, 107, 125, 0.35);
      color: var(--red);
    }
    .badge.connected {
      background: rgba(112, 241, 220, 0.18);
      border-color: rgba(112, 241, 220, 0.4);
      color: var(--cyan);
    }
    .badge.swift-tag {
      background: rgba(255, 107, 43, 0.15);
      border-color: rgba(255, 107, 43, 0.4);
      color: #ff8a50;
    }

    .card {
      background: var(--panel);
      border: 1px solid var(--line);
      border-radius: 14px;
      padding: 16px 20px;
      backdrop-filter: blur(16px);
      box-shadow: 0 10px 30px rgba(0, 0, 0, 0.25);
      margin-bottom: 18px;
    }

    .diag-grid {
      display: grid;
      grid-template-columns: repeat(auto-fit, minmax(280px, 1fr));
      gap: 14px;
    }
    .diag-item {
      background: rgba(255, 255, 255, 0.03);
      border: 1px solid var(--line);
      border-radius: 10px;
      padding: 12px 14px;
      display: flex;
      flex-direction: column;
      gap: 6px;
    }
    .diag-label {
      font-size: 11px;
      font-weight: 600;
      text-transform: uppercase;
      letter-spacing: .06em;
      color: var(--muted);
      display: flex;
      align-items: center;
      justify-content: space-between;
    }
    .diag-val {
      font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace;
      font-size: 13px;
      color: var(--text);
      word-break: break-all;
      display: flex;
      align-items: center;
      justify-content: space-between;
      gap: 8px;
    }
    .copy-chip {
      background: rgba(93, 169, 255, 0.14);
      border: 1px solid rgba(93, 169, 255, 0.3);
      color: var(--blue);
      border-radius: 6px;
      font-size: 11px;
      padding: 2px 7px;
      cursor: pointer;
      user-select: none;
      white-space: nowrap;
      transition: all .15s ease;
    }
    .copy-chip:hover {
      background: rgba(93, 169, 255, 0.28);
      color: #fff;
    }

    .toolbar {
      display: flex;
      align-items: center;
      justify-content: space-between;
      gap: 12px;
      flex-wrap: wrap;
      margin-bottom: 12px;
    }
    .toolbar-group {
      display: flex;
      align-items: center;
      gap: 8px;
      flex-wrap: wrap;
    }
    .btn {
      display: inline-flex;
      align-items: center;
      gap: 6px;
      padding: 6px 12px;
      font-size: 12px;
      font-weight: 600;
      border-radius: 8px;
      border: 1px solid var(--line);
      background: rgba(255, 255, 255, 0.06);
      color: var(--text);
      cursor: pointer;
      user-select: none;
      transition: all .16s ease;
    }
    .btn:hover {
      background: rgba(255, 255, 255, 0.14);
      border-color: var(--line-bright);
      transform: translateY(-1px);
    }
    .btn:active {
      transform: translateY(0);
    }
    .btn.primary {
      background: rgba(93, 169, 255, 0.18);
      border-color: rgba(93, 169, 255, 0.45);
      color: var(--blue);
    }
    .btn.primary:hover {
      background: rgba(93, 169, 255, 0.32);
      color: #fff;
      box-shadow: 0 0 12px rgba(93, 169, 255, 0.3);
    }
    .btn.active {
      background: rgba(112, 241, 220, 0.2);
      border-color: var(--cyan);
      color: var(--cyan);
    }
    .btn.danger {
      background: rgba(255, 107, 125, 0.15);
      border-color: rgba(255, 107, 125, 0.35);
      color: var(--red);
    }
    .btn.danger:hover {
      background: rgba(255, 107, 125, 0.28);
      color: #fff;
    }

    .search-box {
      position: relative;
      display: flex;
      align-items: center;
    }
    .search-input {
      background: rgba(0, 0, 0, 0.4);
      border: 1px solid var(--line);
      border-radius: 8px;
      color: var(--text);
      padding: 6px 12px 6px 28px;
      font-size: 12px;
      outline: none;
      width: 200px;
      transition: all .2s ease;
    }
    .search-input:focus {
      border-color: var(--blue);
      width: 260px;
      box-shadow: 0 0 10px rgba(93, 169, 255, 0.25);
    }
    .search-icon {
      position: absolute;
      left: 9px;
      font-size: 12px;
      color: var(--muted);
      pointer-events: none;
    }
    .search-count {
      margin-left: 6px;
      font-size: 11px;
      color: var(--muted);
    }

    .level-pills {
      display: flex;
      background: rgba(0, 0, 0, 0.35);
      border: 1px solid var(--line);
      border-radius: 8px;
      padding: 2px;
      gap: 2px;
    }
    .level-pill {
      border: none;
      background: transparent;
      color: var(--muted);
      font-size: 11px;
      font-weight: 600;
      padding: 4px 10px;
      border-radius: 6px;
      cursor: pointer;
      transition: all .14s ease;
    }
    .level-pill:hover {
      color: var(--text);
    }
    .level-pill.active {
      background: rgba(255, 255, 255, 0.12);
      color: var(--text);
    }
    .level-pill[data-level="INFO"].active {
      background: rgba(112, 241, 220, 0.2);
      color: var(--cyan);
    }
    .level-pill[data-level="WARN"].active {
      background: rgba(255, 167, 38, 0.2);
      color: var(--orange);
    }
    .level-pill[data-level="ERROR"].active {
      background: rgba(255, 107, 125, 0.2);
      color: var(--red);
    }

    select.select-custom {
      background: rgba(0, 0, 0, 0.4);
      border: 1px solid var(--line);
      border-radius: 8px;
      color: var(--text);
      padding: 5px 8px;
      font-size: 12px;
      outline: none;
      cursor: pointer;
    }

    .terminal-card {
      background: #06080d;
      border: 1px solid var(--line);
      border-radius: 12px;
      overflow: hidden;
      display: flex;
      flex-direction: column;
      height: 68vh;
      min-height: 480px;
      box-shadow: 0 14px 40px rgba(0, 0, 0, 0.5);
    }
    .terminal-header {
      background: #11141c;
      border-bottom: 1px solid var(--line);
      padding: 9px 14px;
      display: flex;
      align-items: center;
      justify-content: space-between;
      font-size: 12px;
      color: var(--muted);
    }
    .terminal-stats {
      display: flex;
      align-items: center;
      gap: 12px;
    }
    .terminal-body {
      flex: 1;
      overflow-y: auto;
      overflow-x: auto;
      padding: 12px 0;
      font-family: ui-monospace, SFMono-Regular, "Cascadia Code", Menlo, Monaco, Consolas, monospace;
      font-size: 12.5px;
      line-height: 1.6;
    }
    .log-line {
      display: flex;
      padding: 1px 16px;
      white-space: pre-wrap;
      word-break: break-all;
    }
    .log-line:hover {
      background: rgba(255, 255, 255, 0.035);
    }
    .line-no {
      color: rgba(255, 255, 255, 0.22);
      user-select: none;
      width: 44px;
      text-align: right;
      padding-right: 14px;
      flex-shrink: 0;
    }
    .log-content {
      flex: 1;
    }

    /* Badges & Tokens */
    .log-ts {
      color: #5d677d;
      margin-right: 10px;
      font-size: 11.5px;
      letter-spacing: -0.01em;
      user-select: none;
    }
    .lvl-tag {
      font-weight: 700;
      font-size: 10.5px;
      padding: 1px 7px;
      border-radius: 4px;
      margin-right: 10px;
      display: inline-block;
      text-align: center;
      min-width: 46px;
      letter-spacing: 0.04em;
    }
    .lvl-INFO {
      background: rgba(112, 241, 220, 0.14);
      color: var(--cyan);
      border: 1px solid rgba(112, 241, 220, 0.3);
      box-shadow: 0 0 8px rgba(112, 241, 220, 0.18);
    }
    .lvl-WARN {
      background: rgba(255, 167, 38, 0.16);
      color: var(--orange);
      border: 1px solid rgba(255, 167, 38, 0.35);
      box-shadow: 0 0 8px rgba(255, 167, 38, 0.2);
    }
    .lvl-ERROR {
      background: rgba(255, 107, 125, 0.18);
      color: var(--red);
      border: 1px solid rgba(255, 107, 125, 0.4);
      box-shadow: 0 0 10px rgba(255, 107, 125, 0.25);
    }
    .lvl-DEBUG {
      background: rgba(199, 146, 234, 0.14);
      color: #c792ea;
      border: 1px solid rgba(199, 146, 234, 0.3);
    }
    .log-target {
      color: #82b1ff;
      background: rgba(255, 255, 255, 0.05);
      border: 1px solid rgba(255, 255, 255, 0.08);
      padding: 1px 7px;
      border-radius: 4px;
      font-size: 11px;
      margin-right: 8px;
    }
    .log-msg {
      color: #e2e8f5;
    }

    .tok-arrow {
      color: var(--cyan);
      font-weight: 700;
      text-shadow: 0 0 8px rgba(112, 241, 220, 0.45);
    }
    .tok-verb {
      color: #79c0ff;
      font-weight: 600;
    }
    .tok-ip {
      color: #ffcc80;
      font-family: ui-monospace, SFMono-Regular, monospace;
      background: rgba(255, 167, 38, 0.12);
      padding: 1px 5px;
      border-radius: 4px;
      border: 1px solid rgba(255, 167, 38, 0.25);
    }
    .tok-uuid {
      color: #d1b3ff;
      font-family: ui-monospace, SFMono-Regular, monospace;
      background: rgba(209, 179, 255, 0.1);
      padding: 1px 5px;
      border-radius: 4px;
      border: 1px solid rgba(209, 179, 255, 0.2);
    }
    .tok-key {
      color: #7ee787;
    }
    .tok-val {
      color: #a5d6ff;
    }
    .tok-status-ok {
      color: #56d364;
      font-weight: 600;
    }
    .tok-status-warn {
      color: #e3b341;
      font-weight: 600;
    }
    .tok-status-err {
      color: #f85149;
      font-weight: 600;
    }
    .log-banner {
      color: #5da9ff;
      font-weight: 600;
    }

    .highlight {
      background: rgba(255, 204, 0, 0.35);
      color: #fff;
      padding: 0 2px;
      border-radius: 2px;
    }

    .empty-state {
      padding: 60px 20px;
      text-align: center;
      color: var(--muted);
    }
    .empty-state h3 {
      color: var(--text);
      margin: 0 0 8px;
    }

    .toast {
      position: fixed;
      bottom: 24px;
      right: 24px;
      background: #1e2430;
      border: 1px solid var(--blue);
      color: #fff;
      padding: 10px 18px;
      border-radius: 8px;
      box-shadow: 0 8px 24px rgba(0, 0, 0, 0.4);
      font-size: 13px;
      opacity: 0;
      transform: translateY(10px);
      transition: all .2s cubic-bezier(0.16, 1, 0.3, 1);
      pointer-events: none;
      z-index: 1000;
    }
    .toast.show {
      opacity: 1;
      transform: translateY(0);
    }

    .api-card {
      margin-top: 18px;
      font-size: 12px;
      color: var(--muted);
      display: flex;
      align-items: center;
      justify-content: space-between;
      flex-wrap: wrap;
      gap: 12px;
    }
    .api-links {
      display: flex;
      gap: 14px;
      flex-wrap: wrap;
    }
    .api-links a {
      color: var(--blue);
      text-decoration: none;
      font-family: ui-monospace, SFMono-Regular, monospace;
    }
    .api-links a:hover {
      text-decoration: underline;
    }
  </style>
</head>
<body>
  <main>
    <header>
      <div class="header-left">
        <a href="http://127.0.0.1:8765/" class="back-btn" title="返回遥控控制台 (Port 8765)">← 返回遥控面板 (8765)</a>
        <div>
          <h1>🛠️ Apple TV Remote 系统运行日志</h1>
          <div class="subtitle">macOS Swift Host Web Service (Port 8766) · Runtime Logs & Process Diagnostics</div>
        </div>
      </div>
      <div class="header-right">
        <div id="swiftBadge" class="badge swift-tag">⚡ Swift Web Server :8766</div>
        <div id="serverBadge" class="badge">● 后台进程检查中...</div>
        <div id="clientBadge" class="badge">📱 手机状态检查中...</div>
      </div>
    </header>

    <!-- Diagnostics Overview -->
    <div class="card">
      <div class="diag-grid">
        <div class="diag-item">
          <div class="diag-label">
            <span>日志物理文件路径 (Swift Managed)</span>
            <span id="fileSizeBadge" style="color: var(--blue);">0 KB</span>
          </div>
          <div class="diag-val">
            <span id="logPathText" style="overflow: hidden; text-overflow: ellipsis; white-space: nowrap;">~/Library/Logs/AppleTVRemote/atv-core.log</span>
            <button class="copy-chip" onclick="copyText(state.logPath || '~/Library/Logs/AppleTVRemote/atv-core.log')">复制路径</button>
          </div>
        </div>
        <div class="diag-item">
          <div class="diag-label">
            <span>终端实时查看命令 (Terminal Follow)</span>
            <span>CLI</span>
          </div>
          <div class="diag-val">
            <span>tail -f ~/Library/Logs/AppleTVRemote/atv-core.log</span>
            <button class="copy-chip" onclick="copyText('tail -f ~/Library/Logs/AppleTVRemote/atv-core.log')">复制命令</button>
          </div>
        </div>
        <div class="diag-item">
          <div class="diag-label">
            <span>架构与服务端口 (Architecture & Ports)</span>
            <span id="procPidBadge" style="color: var(--cyan);">PID: --</span>
          </div>
          <div class="diag-val">
            <span>Core API: 8765 | Web Debug: 8766 | MRP: 49152</span>
            <button class="copy-chip" onclick="restartCoreProcess()">重启引擎</button>
          </div>
        </div>
      </div>
    </div>

    <!-- Toolbar -->
    <div class="toolbar">
      <div class="toolbar-group">
        <button id="refreshBtn" class="btn primary" onclick="fetchLogs(true)">🔄 刷新日志</button>
        <button id="autoRefreshBtn" class="btn active" onclick="toggleAutoRefresh()">⏱️ 自动刷新 (2s)</button>
        <button class="btn" onclick="restartCoreProcess()" title="向 Swift 发起重启后台 atv-cli 进程请求">🔄 重启进程</button>
        <button class="btn danger" onclick="clearLogFile()" title="清空本地 atv-core.log 文件内容">🧹 清空文件</button>
        <div class="level-pills">
          <button class="level-pill active" data-level="ALL" onclick="setLevelFilter('ALL')">全部</button>
          <button class="level-pill" data-level="INFO" onclick="setLevelFilter('INFO')">INFO</button>
          <button class="level-pill" data-level="WARN" onclick="setLevelFilter('WARN')">WARN</button>
          <button class="level-pill" data-level="ERROR" onclick="setLevelFilter('ERROR')">ERROR</button>
          <button class="level-pill" data-level="DEBUG" onclick="setLevelFilter('DEBUG')">DEBUG</button>
        </div>
        <div class="search-box">
          <span class="search-icon">🔍</span>
          <input type="text" id="searchInput" class="search-input" placeholder="过滤关键词..." oninput="handleSearch(this.value)">
        </div>
        <span id="matchCount" class="search-count"></span>
      </div>

      <div class="toolbar-group">
        <select id="tailSelect" class="select-custom" onchange="onTailChange(this.value)">
          <option value="100">显示最近 100 行</option>
          <option value="300" selected>显示最近 300 行</option>
          <option value="500">显示最近 500 行</option>
          <option value="1000">显示最近 1000 行</option>
          <option value="all">显示全部行</option>
        </select>
        <label style="display: inline-flex; align-items: center; gap: 4px; font-size: 12px; cursor: pointer; color: var(--muted); user-select: none;">
          <input type="checkbox" id="autoScrollCheck" checked> 自动滚屏
        </label>
        <button class="btn" onclick="copyFilteredLogs()" title="复制当前筛选出的日志文本">📋 复制</button>
        <button class="btn" onclick="downloadLogs()" title="下载完整日志文件">⬇️ 导出文件</button>
        <button class="btn" onclick="clearView()" title="清空当前界面的展示">🗑️ 清屏</button>
      </div>
    </div>

    <!-- Terminal Container -->
    <div class="terminal-card">
      <div class="terminal-header">
        <div class="terminal-stats">
          <span>📄 <strong id="logFileName">atv-core.log</strong></span>
          <span id="renderedCount">正在加载...</span>
          <span id="lastUpdated">更新时间: --:--:--</span>
        </div>
        <div style="font-size: 11px; color: var(--muted);">
          快捷键: <kbd style="background: rgba(255,255,255,0.1); padding: 1px 4px; border-radius: 3px;">R</kbd> 刷新 |
          <kbd style="background: rgba(255,255,255,0.1); padding: 1px 4px; border-radius: 3px;">F</kbd> 搜索
        </div>
      </div>
      <div id="terminalBody" class="terminal-body">
        <div class="empty-state">
          <h3>正在获取系统运行日志...</h3>
          <p>请稍候，Swift Web Server 正在读取日志文件。</p>
        </div>
      </div>
    </div>

    <!-- Footer API Docs -->
    <div class="card api-card">
      <div>
        <strong>💡 Swift 宿主调试接口 (Swift API Endpoints on :8766):</strong>
      </div>
      <div class="api-links">
        <a href="/api/logs?tail=300" target="_blank">/api/logs?tail=300 (JSON)</a>
        <a href="/api/logs?raw=1&tail=500" target="_blank">/api/logs?raw=1 (Raw Text)</a>
        <a href="/api/logs?download=1" target="_blank">/api/logs?download=1 (Download)</a>
        <a href="/api/status" target="_blank">/api/status (Host Process Status)</a>
      </div>
    </div>
  </main>

  <div id="toast" class="toast">已成功复制到剪贴板</div>

  <script>
    const state = {
      rawLines: [],
      filteredLines: [],
      levelFilter: 'ALL',
      searchKeyword: '',
      tail: '300',
      autoRefresh: true,
      autoRefreshInterval: null,
      logPath: '',
      fileSize: 0,
      isFetching: false
    };

    const $ = id => document.getElementById(id);

    // ===== Language: Chinese when the browser prefers it, English otherwise =====
    const I18N_ZH = ((navigator.languages && navigator.languages[0]) || navigator.language || '').toLowerCase().startsWith('zh');
    // English for the page's Chinese text, keyed by the Chinese.
    const I18N_EN = {
      "Apple TV Remote - 运行日志与系统诊断 (Swift Host)": "Apple TV Remote - Logs and Diagnostics (Swift Host)",
      "返回遥控控制台 (Port 8765)": "Back to the remote console (port 8765)",
      "← 返回遥控面板 (8765)": "← Back to Remote Panel (8765)",
      "🛠️ Apple TV Remote 系统运行日志": "🛠️ Apple TV Remote System Log",
      "● 后台进程检查中...": "● Checking background process...",
      "📱 手机状态检查中...": "📱 Checking iPhone...",
      "日志物理文件路径 (Swift Managed)": "Log file path (managed by the Swift app)",
      "复制路径": "Copy Path",
      "终端实时查看命令 (Terminal Follow)": "Follow in Terminal",
      "复制命令": "Copy Command",
      "架构与服务端口 (Architecture & Ports)": "Architecture & Ports",
      "重启引擎": "Restart Engine",
      "🔄 刷新日志": "🔄 Refresh Log",
      "⏱️ 自动刷新 (2s)": "⏱️ Auto Refresh (2s)",
      "向 Swift 发起重启后台 atv-cli 进程请求": "Ask the Swift app to restart the atv-cli background process",
      "🔄 重启进程": "🔄 Restart Process",
      "清空本地 atv-core.log 文件内容": "Empty the local atv-core.log file",
      "🧹 清空文件": "🧹 Empty File",
      "全部": "All",
      "过滤关键词...": "Filter...",
      "显示最近 100 行": "Last 100 lines",
      "显示最近 300 行": "Last 300 lines",
      "显示最近 500 行": "Last 500 lines",
      "显示最近 1000 行": "Last 1000 lines",
      "显示全部行": "All lines",
      "自动滚屏": "Auto-scroll",
      "复制当前筛选出的日志文本": "Copy the filtered log lines",
      "📋 复制": "📋 Copy",
      "下载完整日志文件": "Download the whole log file",
      "⬇️ 导出文件": "⬇️ Export",
      "清空当前界面的展示": "Clear what this page shows",
      "🗑️ 清屏": "🗑️ Clear View",
      "正在加载...": "Loading...",
      "更新时间: --:--:--": "Updated: --:--:--",
      "快捷键:": "Shortcuts:",
      "刷新 |": "refresh |",
      "搜索": "search",
      "正在获取系统运行日志...": "Fetching the system log...",
      "请稍候，Swift Web Server 正在读取日志文件。": "One moment, the Swift web server is reading the log file.",
      "💡 Swift 宿主调试接口 (Swift API Endpoints on :8766):": "💡 Swift host API endpoints (on :8766):",
      "已成功复制到剪贴板": "Copied to the clipboard",
      "已复制: ": "Copied: ",
      "复制失败，请手动选择": "Couldn't copy. Please select the text by hand.",
      "⏸️ 自动刷新 (已暂停)": "⏸️ Auto Refresh (paused)",
      "当前没有匹配的日志行": "No log lines match",
      "确定要通过 Swift 宿主重启后台 atv-cli 核心引擎吗？": "Restart the atv-cli engine through the Swift app?",
      "正在请求重启后台服务...": "Restarting the background service...",
      "✅ 后台服务已成功重启": "✅ Background service restarted",
      "⚠️ 重启未成功: ": "⚠️ Restart failed: ",
      "未知错误": "unknown error",
      "❌ 请求重启失败": "❌ Restart request failed",
      "确定要清空磁盘上的 atv-core.log 日志文件吗？此操作无法撤销。": "Empty the atv-core.log file on disk? This can't be undone.",
      "🧹 日志文件已成功清空": "🧹 Log file emptied",
      "清空失败": "Couldn't empty the file",
      "● 后台引擎运行中 (PID ": "● Engine running (PID ",
      "○ 后台引擎已停止": "○ Engine stopped",
      "未运行": "Not running",
      "📱 手机已连接 (": "📱 iPhone connected (",
      "就绪": "ready",
      "📱 等待手机连接": "📱 Waiting for iPhone",
      "⏳ 加载中...": "⏳ Loading...",
      "更新时间: ": "Updated: ",
      "暂无符合条件的日志": "No matching log lines",
      "日志文件当前为空。通过 Apple TV Remote 触发操作后将显示实时日志。": "The log file is empty. Use Apple TV Remote and its log will appear here.",
      "未找到匹配当前筛选级别的日志内容。": "No log lines match the current level filter."
    };
    /** The English for a piece of the page's Chinese text, unless the browser prefers Chinese. */
    const i18n = zh => (I18N_ZH ? zh : (I18N_EN[zh] ?? zh));
    /** The Chinese or the English version of a text put together on the fly. */
    const i18nPick = (zh, en) => (I18N_ZH ? zh : en);
    // Translate the page's fixed text and tooltips.
    (function translateFixedText() {
      if (I18N_ZH) return;
      document.documentElement.lang = 'en';
      const walker = document.createTreeWalker(document.documentElement, NodeFilter.SHOW_TEXT);
      for (let node = walker.nextNode(); node; node = walker.nextNode()) {
        const key = node.nodeValue.trim();
        if (key && I18N_EN[key] !== undefined) node.nodeValue = node.nodeValue.replace(key, () => I18N_EN[key]);
      }
      for (const el of document.querySelectorAll('[title], [placeholder], [aria-label]')) {
        for (const attr of ['title', 'placeholder', 'aria-label']) {
          const value = el.getAttribute(attr);
          if (value && I18N_EN[value] !== undefined) el.setAttribute(attr, I18N_EN[value]);
        }
      }
    })();

    function showToast(msg) {
      const t = $('toast');
      t.textContent = msg;
      t.classList.add('show');
      setTimeout(() => t.classList.remove('show'), 2200);
    }

    function copyText(text) {
      if (!text) return;
      navigator.clipboard.writeText(text).then(() => {
        showToast(i18n('已复制: ') + text);
      }).catch(() => {
        showToast(i18n('复制失败，请手动选择'));
      });
    }

    function downloadLogs() {
      window.location.href = '/api/logs?download=1';
    }

    function setLevelFilter(lvl) {
      state.levelFilter = lvl;
      document.querySelectorAll('.level-pill').forEach(pill => {
        pill.classList.toggle('active', pill.dataset.level === lvl);
      });
      applyFilters();
    }

    function handleSearch(val) {
      state.searchKeyword = (val || '').trim().toLowerCase();
      applyFilters();
    }

    function onTailChange(val) {
      state.tail = val;
      fetchLogs(true);
    }

    function toggleAutoRefresh() {
      state.autoRefresh = !state.autoRefresh;
      const btn = $('autoRefreshBtn');
      btn.classList.toggle('active', state.autoRefresh);
      if (state.autoRefresh) {
        btn.textContent = i18n('⏱️ 自动刷新 (2s)');
        startAutoRefresh();
      } else {
        btn.textContent = i18n('⏸️ 自动刷新 (已暂停)');
        stopAutoRefresh();
      }
    }

    function startAutoRefresh() {
      stopAutoRefresh();
      state.autoRefreshInterval = setInterval(() => {
        fetchLogs(false);
      }, 2000);
    }

    function stopAutoRefresh() {
      if (state.autoRefreshInterval) {
        clearInterval(state.autoRefreshInterval);
        state.autoRefreshInterval = null;
      }
    }

    function clearView() {
      state.rawLines = [];
      state.filteredLines = [];
      renderLogs();
    }

    function copyFilteredLogs() {
      if (state.filteredLines.length === 0) {
        showToast(i18n('当前没有匹配的日志行'));
        return;
      }
      const text = state.filteredLines.join('\\n');
      navigator.clipboard.writeText(text).then(() => {
        showToast(i18nPick(`已成功复制 ${state.filteredLines.length} 行日志`, `Copied ${state.filteredLines.length} log lines`));
      });
    }

    async function restartCoreProcess() {
      if (!confirm(i18n('确定要通过 Swift 宿主重启后台 atv-cli 核心引擎吗？'))) return;
      showToast(i18n('正在请求重启后台服务...'));
      try {
        const res = await fetch('/api/restart', { method: 'POST' });
        const data = await res.json();
        if (data.status === 'ok') {
          showToast(i18n('✅ 后台服务已成功重启'));
          setTimeout(() => { fetchStatus(); fetchLogs(true); }, 1000);
        } else {
          showToast(i18n('⚠️ 重启未成功: ') + (data.message || i18n('未知错误')));
        }
      } catch (e) {
        showToast(i18n('❌ 请求重启失败'));
      }
    }

    async function clearLogFile() {
      if (!confirm(i18n('确定要清空磁盘上的 atv-core.log 日志文件吗？此操作无法撤销。'))) return;
      try {
        const res = await fetch('/api/clear_log', { method: 'POST' });
        const data = await res.json();
        if (data.status === 'ok') {
          showToast(i18n('🧹 日志文件已成功清空'));
          fetchLogs(true);
        }
      } catch (e) {
        showToast(i18n('清空失败'));
      }
    }

    async function fetchStatus() {
      try {
        const res = await fetch('/api/status');
        if (!res.ok) return;
        const data = await res.json();
        
        const sBadge = $('serverBadge');
        if (data.service_running) {
          sBadge.textContent = i18n('● 后台引擎运行中 (PID ') + (data.pid || '--') + ')';
          sBadge.className = 'badge online';
          $('procPidBadge').textContent = 'PID: ' + (data.pid || '--');
        } else {
          sBadge.textContent = i18n('○ 后台引擎已停止');
          sBadge.className = 'badge offline';
          $('procPidBadge').textContent = i18n('未运行');
        }

        // Also probe Rust core on 8765 if reachable
        try {
          const coreRes = await fetch('http://127.0.0.1:8765/api/status', { mode: 'cors' });
          if (coreRes.ok) {
            const coreData = await coreRes.json();
            const cBadge = $('clientBadge');
            if (coreData.client_connected) {
              cBadge.textContent = i18n('📱 手机已连接 (') + (coreData.client_peer || i18n('就绪')) + ')';
              cBadge.className = 'badge connected';
            } else {
              cBadge.textContent = i18n('📱 等待手机连接');
              cBadge.className = 'badge';
            }
          }
        } catch (e) {}
      } catch (e) {}
    }

    async function fetchLogs(manual = false) {
      if (state.isFetching) return;
      state.isFetching = true;
      if (manual) {
        $('refreshBtn').textContent = i18n('⏳ 加载中...');
      }

      try {
        const res = await fetch(`/api/logs?tail=${state.tail}`);
        if (!res.ok) throw new Error('HTTP ' + res.status);
        const data = await res.json();

        state.logPath = data.log_path || '';
        $('logPathText').textContent = state.logPath || '~/Library/Logs/AppleTVRemote/atv-core.log';

        state.fileSize = data.size_bytes || 0;
        const sizeKb = (state.fileSize / 1024).toFixed(1);
        $('fileSizeBadge').textContent = sizeKb > 1024 ? (sizeKb / 1024).toFixed(2) + ' MB' : sizeKb + ' KB';

        state.rawLines = data.lines || [];
        applyFilters();

        const now = new Date();
        $('lastUpdated').textContent = i18n('更新时间: ') + now.toTimeString().split(' ')[0];
      } catch (err) {
        console.error('Failed to fetch logs from Swift server:', err);
      } finally {
        state.isFetching = false;
        if (manual) {
          $('refreshBtn').textContent = i18n('🔄 刷新日志');
        }
      }
    }

    function stripAnsi(str) {
      if (!str) return '';
      return str.replace(/(?:\\u001b|\\x1b)?\\[[0-9;]*[a-zA-Z]/g, '');
    }

    function ansiToHtml(str) {
      const colorMap = {
        '30': 'color: #4e5569;',
        '31': 'color: #ff6b7d;',
        '32': 'color: #70f1dc;',
        '33': 'color: #ffa726;',
        '34': 'color: #5da9ff;',
        '35': 'color: #c792ea;',
        '36': 'color: #89ddff;',
        '37': 'color: #ffffff;',
        '90': 'color: #636d83;',
        '91': 'color: #ff8b9a;',
        '92': 'color: #a3f7e9;',
        '93': 'color: #ffcc80;',
        '94': 'color: #82b1ff;',
        '95': 'color: #d1b3ff;',
        '96': 'color: #b2ebf2;',
        '97': 'color: #ffffff;',
        '1': 'font-weight: 700;',
        '2': 'opacity: 0.75;'
      };

      let openSpans = 0;
      let res = '';
      const regex = /(?:\\u001b|\\x1b)?\\[([0-9;]*)m/g;
      let lastIdx = 0;
      let match;

      while ((match = regex.exec(str)) !== null) {
        res += escapeHtml(str.substring(lastIdx, match.index));
        lastIdx = regex.lastIndex;

        const codes = match[1] ? match[1].split(';') : ['0'];
        for (const code of codes) {
          if (code === '0' || code === '') {
            while (openSpans > 0) {
              res += '</span>';
              openSpans--;
            }
          } else if (colorMap[code]) {
            res += `<span style="${colorMap[code]}">`;
            openSpans++;
          }
        }
      }

      res += escapeHtml(str.substring(lastIdx));
      while (openSpans > 0) {
        res += '</span>';
        openSpans--;
      }

      return res;
    }

    function formatLogMessage(msg, searchKeyword) {
      let escaped = escapeHtml(msg);

      // Protocol directions & verbs: << PvStart, << PvNext, << _systemInfo, >>
      escaped = escaped.replace(/(&lt;&lt;|&gt;&gt;)\\s+([a-zA-Z0-9_]+)/g, '<span class="tok-arrow">$1</span> <span class="tok-verb">$2</span>');

      // IP:Port addresses: e.g. 192.168.101.206:62253
      escaped = escaped.replace(/\\b(\\d{1,3}\\.\\d{1,3}\\.\\d{1,3}\\.\\d{1,3})(:\\d+)?\\b/g, '<span class="tok-ip">$1$2</span>');

      // UUIDs: e.g. EE5426A1-FB95-4329-83A4-41F2D767DC28
      escaped = escaped.replace(/\\b([0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12})\\b/g, '<span class="tok-uuid">$1</span>');

      // Key=value pairs: e.g. seq=1, session=4263664943, renewed=false
      escaped = escaped.replace(/\\b([a-zA-Z_][a-zA-Z0-9_]*)=([^\\s,;)]+)/g, '<span class="tok-key">$1</span>=<span class="tok-val">$2</span>');

      // Status keywords
      escaped = escaped.replace(/\\b(companion client connected|iPhone connected|connected)\\b/gi, '<span class="tok-status-ok">$1</span>');
      escaped = escaped.replace(/\\b(disconnected|closed|closing)\\b/gi, '<span class="tok-status-err">$1</span>');
      escaped = escaped.replace(/\\b(timed out|timeout|warning|failed)\\b/gi, '<span class="tok-status-warn">$1</span>');
      escaped = escaped.replace(/\\b(verification complete|registered new device|ready)\\b/gi, '<span class="tok-status-ok">$1</span>');

      if (searchKeyword) {
        const re = new RegExp(`(${escapeRegex(searchKeyword)})`, 'gi');
        escaped = escaped.replace(re, '<mark class="highlight">$1</mark>');
      }

      return escaped;
    }

    function formatBannerLine(rawLine, searchKeyword) {
      const clean = stripAnsi(rawLine);
      let formatted = '';

      const colonIdx = clean.indexOf(' : ');
      if (colonIdx !== -1 && colonIdx < 30) {
        const k = clean.substring(0, colonIdx).trim();
        const v = clean.substring(colonIdx + 3).trim();
        formatted = `<span class="log-banner">${escapeHtml(k)}</span> <span style="color:var(--muted)">:</span> <span style="color:#fff; font-weight:600;">${escapeHtml(v)}</span>`;
      } else if (clean.includes('atv-cli ready')) {
        formatted = `<span class="tok-status-ok" style="font-weight:700;">🚀 ${escapeHtml(clean)}</span>`;
      } else {
        formatted = ansiToHtml(rawLine);
      }

      if (searchKeyword) {
        const re = new RegExp(`(${escapeRegex(searchKeyword)})`, 'gi');
        formatted = formatted.replace(re, '<mark class="highlight">$1</mark>');
      }

      return formatted;
    }

    function applyFilters() {
      let lines = state.rawLines;

      if (state.levelFilter !== 'ALL') {
        const lvl = state.levelFilter;
        lines = lines.filter(l => {
          const clean = stripAnsi(l);
          return clean.includes(` ${lvl} `) || clean.includes(`[${lvl}]`);
        });
      }

      if (state.searchKeyword) {
        const kw = state.searchKeyword;
        lines = lines.filter(l => {
          const clean = stripAnsi(l).toLowerCase();
          return clean.includes(kw);
        });
        $('matchCount').textContent = i18nPick(`(匹配 ${lines.length} / ${state.rawLines.length} 行)`, `(${lines.length} of ${state.rawLines.length} lines match)`);
      } else {
        $('matchCount').textContent = '';
      }

      state.filteredLines = lines;
      renderLogs();
    }

    function escapeHtml(str) {
      return str
        .replace(/&/g, "&amp;")
        .replace(/</g, "&lt;")
        .replace(/>/g, "&gt;")
        .replace(/"/g, "&quot;")
        .replace(/'/g, "&#039;");
    }

    function renderLogs() {
      const container = $('terminalBody');
      const lines = state.filteredLines;

      $('renderedCount').textContent = i18nPick(`显示 ${lines.length} 行 (总计 ${state.rawLines.length} 行)`, `Showing ${lines.length} lines (${state.rawLines.length} in total)`);

      if (lines.length === 0) {
        container.innerHTML = `
          <div class="empty-state">
            <h3>${i18n('暂无符合条件的日志')}</h3>
            <p>${state.rawLines.length === 0 ? i18n('日志文件当前为空。通过 Apple TV Remote 触发操作后将显示实时日志。') : i18n('未找到匹配当前筛选级别的日志内容。')}</p>
          </div>
        `;
        return;
      }

      const kw = state.searchKeyword;
      const html = lines.map((line, idx) => {
        let parsed = parseLogLine(line);
        let contentHtml = '';

        if (parsed.isStructured) {
          if (parsed.timestamp) {
            contentHtml += `<span class="log-ts">${escapeHtml(parsed.timestamp)}</span>`;
          }
          if (parsed.level) {
            contentHtml += `<span class="lvl-tag lvl-${parsed.level}">${parsed.level}</span>`;
          }
          if (parsed.target) {
            contentHtml += `<span class="log-target">${escapeHtml(parsed.target)}:</span>`;
          }
          contentHtml += `<span class="log-msg">${formatLogMessage(parsed.message, kw)}</span>`;
        } else {
          contentHtml += formatBannerLine(line, kw);
        }

        return `
          <div class="log-line">
            <span class="line-no">${idx + 1}</span>
            <div class="log-content">${contentHtml}</div>
          </div>
        `;
      }).join('');

      const shouldScroll = $('autoScrollCheck').checked;
      const wasAtBottom = (container.scrollHeight - container.scrollTop <= container.clientHeight + 60);

      container.innerHTML = html;

      if (shouldScroll && (wasAtBottom || lines.length <= 100)) {
        container.scrollTop = container.scrollHeight;
      }
    }

    function escapeRegex(string) {
      return string.replace(/[.*+?^${}()|[\\]\\\\]/g, '\\\\$&');
    }

    function parseLogLine(rawLine) {
      const clean = stripAnsi(rawLine).trim();

      const isoRegex = /^(\\d{4}-\\d{2}-\\d{2}T[\\d:\\.]+Z)\\s+([A-Z]+)\\s+([^:]+):\\s*(.*)$/;
      const match = clean.match(isoRegex);
      if (match) {
        return {
          isStructured: true,
          timestamp: match[1].substring(11, 23),
          level: match[2].trim(),
          target: match[3].trim(),
          message: match[4]
        };
      }

      for (const lvl of ['INFO', 'WARN', 'ERROR', 'DEBUG']) {
        const idx = clean.indexOf(` ${lvl} `);
        if (idx !== -1) {
          const before = clean.substring(0, idx).trim();
          const after = clean.substring(idx + lvl.length + 2).trim();
          const colonIdx = after.indexOf(':');
          let target = '';
          let message = after;
          if (colonIdx !== -1 && colonIdx < 40) {
            target = after.substring(0, colonIdx).trim();
            message = after.substring(colonIdx + 1).trim();
          }
          return {
            isStructured: true,
            timestamp: before.length >= 12 ? before.substring(before.length - 12) : before,
            level: lvl,
            target: target,
            message: message
          };
        }
      }

      return {
        isStructured: false,
        timestamp: '',
        level: '',
        target: '',
        message: rawLine
      };
    }

    document.addEventListener('keydown', e => {
      if (e.key === 'r' || e.key === 'R') {
        if (document.activeElement.tagName !== 'INPUT') {
          fetchLogs(true);
        }
      } else if (e.key === 'f' || e.key === 'F') {
        if (document.activeElement.tagName !== 'INPUT') {
          e.preventDefault();
          $('searchInput').focus();
        }
      }
    });

    fetchStatus();
    fetchLogs(true);
    startAutoRefresh();
    setInterval(fetchStatus, 3000);
  </script>
</body>
</html>
"""
}
