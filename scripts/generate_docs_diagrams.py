#!/usr/bin/env python3
"""
Generate elegant SVG diagrams for atv-core documentation:
1. docs/images/page_navigation_movement.svg - Visualizes remote swipe gesture translation to TV focus move & Mac cursor.
2. docs/images/emulator_operation_flow.svg - Visualizes emulator bridge operations, ADB forwarding, and mDNS discovery.
3. docs/images/prerequisites_and_permissions.svg - Visualizes prerequisites, ADB enabling, permission dialogs, and code signing.
"""

import os

os.makedirs("docs/images", exist_ok=True)

# 1. Page Navigation & Movement Diagram
svg_page_nav = """<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 1000 620" width="100%" height="100%">
  <defs>
    <linearGradient id="bg-grad" x1="0%" y1="0%" x2="100%" y2="100%">
      <stop offset="0%" stop-color="#0b0f19"/>
      <stop offset="50%" stop-color="#111827"/>
      <stop offset="100%" stop-color="#0b0f19"/>
    </linearGradient>
    <linearGradient id="card-grad" x1="0%" y1="0%" x2="100%" y2="100%">
      <stop offset="0%" stop-color="#1f2937" stop-opacity="0.8"/>
      <stop offset="100%" stop-color="#111827" stop-opacity="0.9"/>
    </linearGradient>
    <linearGradient id="accent-blue" x1="0%" y1="0%" x2="100%" y2="0%">
      <stop offset="0%" stop-color="#3b82f6"/>
      <stop offset="100%" stop-color="#60a5fa"/>
    </linearGradient>
    <linearGradient id="accent-purple" x1="0%" y1="0%" x2="100%" y2="0%">
      <stop offset="0%" stop-color="#8b5cf6"/>
      <stop offset="100%" stop-color="#a78bfa"/>
    </linearGradient>
    <linearGradient id="accent-emerald" x1="0%" y1="0%" x2="100%" y2="0%">
      <stop offset="0%" stop-color="#10b981"/>
      <stop offset="100%" stop-color="#34d399"/>
    </linearGradient>
    <filter id="glow" x="-20%" y="-20%" width="140%" height="140%">
      <feGaussianBlur stdDeviation="8" result="blur" />
      <feComposite in="SourceGraphic" in2="blur" operator="over"/>
    </filter>
    <filter id="subtle-shadow" x="-5%" y="-5%" width="110%" height="115%">
      <feDropShadow dx="0" dy="8" stdDeviation="12" flood-color="#000000" flood-opacity="0.5"/>
    </filter>
  </defs>

  <style>
    .title { font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, Helvetica, Arial, sans-serif; font-size: 24px; font-weight: 700; fill: #f3f4f6; }
    .subtitle { font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, Helvetica, Arial, sans-serif; font-size: 13px; fill: #9ca3af; }
    .card-title { font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, Helvetica, Arial, sans-serif; font-size: 16px; font-weight: 600; fill: #ffffff; }
    .card-sub { font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, Helvetica, Arial, sans-serif; font-size: 12px; fill: #9ca3af; }
    .label { font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, Helvetica, Arial, sans-serif; font-size: 11px; font-weight: 500; fill: #e5e7eb; }
    .code { font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace; font-size: 11px; fill: #60a5fa; }
    .badge { font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, Helvetica, Arial, sans-serif; font-size: 10px; font-weight: 700; }
  </style>

  <!-- Background -->
  <rect width="1000" height="620" rx="16" fill="url(#bg-grad)"/>
  <rect width="998" height="618" x="1" y="1" rx="15" fill="none" stroke="#374151" stroke-width="1.5" stroke-opacity="0.6"/>

  <!-- Header -->
  <g transform="translate(40, 45)">
    <text class="title">遥控手势与页面移动原理 / Remote Swipe &amp; Navigation Flow</text>
    <text y="24" class="subtitle">iOS 控制中心 Apple TV 遥控器手势实时转换为 Android TV 焦点移动与 macOS 光标位移</text>
  </g>

  <!-- Left: iPhone Apple TV Remote Mockup -->
  <g transform="translate(50, 110)" filter="url(#subtle-shadow)">
    <!-- Remote Body -->
    <rect width="240" height="460" rx="36" fill="#18181b" stroke="#3f3f46" stroke-width="2"/>
    <rect width="232" height="452" x="4" y="4" rx="32" fill="#09090b"/>

    <!-- Dynamic Island / Remote Header -->
    <text x="120" y="38" text-anchor="middle" class="card-title" font-size="14">Apple TV 遥控器</text>
    <text x="120" y="54" text-anchor="middle" class="card-sub" font-size="11">已连接 • 局域网加密链路</text>

    <!-- Trackpad Area -->
    <rect x="25" y="75" width="190" height="210" rx="20" fill="#18181b" stroke="#3b82f6" stroke-width="1.5" stroke-dasharray="4 3"/>
    
    <!-- Gesture Visualization -->
    <!-- Swipe arrow -->
    <path d="M 120 220 L 120 120" stroke="#60a5fa" stroke-width="3" stroke-linecap="round"/>
    <polygon points="113,124 120,108 127,124" fill="#60a5fa"/>
    <circle cx="120" cy="220" r="14" fill="#3b82f6" fill-opacity="0.3" stroke="#60a5fa" stroke-width="2"/>
    <circle cx="120" cy="220" r="6" fill="#60a5fa"/>
    <text x="120" y="255" text-anchor="middle" class="label" fill="#93c5fd">向上轻扫 (Swipe UP)</text>
    <text x="120" y="272" text-anchor="middle" class="code" font-size="10">dx=0, dy=-38.5, phase=Moved</text>

    <!-- Buttons Grid -->
    <!-- Row 1: Back & TV/Home -->
    <rect x="35" y="305" width="75" height="42" rx="21" fill="#27272a"/>
    <text x="72" y="331" text-anchor="middle" class="label" font-size="14">&lt;</text>
    
    <rect x="130" y="305" width="75" height="42" rx="21" fill="#27272a"/>
    <text x="167" y="330" text-anchor="middle" class="label" font-size="12">📺</text>

    <!-- Row 2: Play/Pause & Mute -->
    <rect x="35" y="357" width="75" height="42" rx="21" fill="#27272a"/>
    <text x="72" y="382" text-anchor="middle" class="label" font-size="13">⏯</text>
    
    <rect x="130" y="357" width="75" height="42" rx="21" fill="#27272a"/>
    <text x="167" y="382" text-anchor="middle" class="label" font-size="13">🔇</text>

    <!-- Row 3: Volume rocker & Siri -->
    <rect x="35" y="409" width="75" height="34" rx="17" fill="#27272a"/>
    <text x="72" y="430" text-anchor="middle" class="label" font-size="11">VOL +/-</text>
    
    <rect x="130" y="409" width="75" height="34" rx="17" fill="#3b82f6" fill-opacity="0.2" stroke="#3b82f6" stroke-width="1"/>
    <text x="167" y="430" text-anchor="middle" class="label" fill="#93c5fd" font-size="11">模式/Siri</text>
  </g>

  <!-- Middle: Dispatch & Protocol Engine -->
  <g transform="translate(325, 120)">
    <!-- Event translation box -->
    <rect width="250" height="440" rx="16" fill="url(#card-grad)" stroke="#374151" stroke-width="1.5" filter="url(#subtle-shadow)"/>
    
    <text x="125" y="32" text-anchor="middle" class="card-title">atv-core 事件分发引擎</text>
    <text x="125" y="52" text-anchor="middle" class="card-sub">Companion Link / MRP 状态机</text>

    <!-- Step 1 -->
    <g transform="translate(18, 75)">
      <rect width="214" height="68" rx="8" fill="#1f2937" stroke="#4b5563" stroke-width="1"/>
      <text x="12" y="22" class="label" fill="#60a5fa" font-weight="700">1. OPACK 消息解密</text>
      <text x="12" y="40" class="card-sub">ChaCha20-Poly1305 解包</text>
      <text x="12" y="56" class="code">_touchMove / _hidEvent</text>
    </g>

    <!-- Step 2 -->
    <g transform="translate(18, 155)">
      <rect width="214" height="90" rx="8" fill="#1f2937" stroke="#8b5cf6" stroke-width="1"/>
      <text x="12" y="22" class="label" fill="#a78bfa" font-weight="700">2. 模式与动力学解析</text>
      <text x="12" y="40" class="card-sub">方向键模式 (D-Pad Threshold)</text>
      <text x="12" y="58" class="card-sub">鼠标模式 (Dynamic Ballistics)</text>
      <text x="12" y="76" class="code">dy &lt; -25px =&gt; KEY_UP / Y -= 38</text>
    </g>

    <!-- Step 3 -->
    <g transform="translate(18, 258)">
      <rect width="214" height="85" rx="8" fill="#1f2937" stroke="#10b981" stroke-width="1"/>
      <text x="12" y="22" class="label" fill="#34d399" font-weight="700">3. 双端高保真映射</text>
      <text x="12" y="40" class="card-sub">TV: LocalAdbDriver / A11y</text>
      <text x="12" y="58" class="card-sub">Mac: CGEvent / NX_KEYTYPE</text>
      <text x="12" y="74" class="code">延迟 &lt; 8ms • 极速直达</text>
    </g>

    <!-- Dispatch status badge -->
    <g transform="translate(18, 360)">
      <rect width="214" height="60" rx="8" fill="#111827" stroke="#374151" stroke-width="1"/>
      <circle cx="22" cy="30" r="5" fill="#10b981"/>
      <text x="35" y="26" class="label">零抖动累加器就绪</text>
      <text x="35" y="44" class="card-sub">三端配置实时秒级热同步</text>
    </g>
  </g>

  <!-- Connectors from Remote to Engine -->
  <path d="M 290 220 L 325 220" stroke="#3b82f6" stroke-width="2" stroke-dasharray="4 2"/>
  <polygon points="320,216 328,220 320,224" fill="#3b82f6"/>

  <!-- Right: Target Response (Android TV & Mac) -->
  <g transform="translate(615, 120)">
    <!-- Android TV Screen Box -->
    <g filter="url(#subtle-shadow)">
      <rect width="345" height="210" rx="14" fill="url(#card-grad)" stroke="#374151" stroke-width="1.5"/>
      <!-- TV Screen Frame -->
      <text x="20" y="28" class="card-title" font-size="14">Android TV: 焦点页面平滑切换</text>
      <text x="20" y="44" class="card-sub">轻扫立即驱动 Launcher / App 焦点网格迁移</text>

      <!-- TV UI Preview Mockup -->
      <g transform="translate(18, 55)">
        <rect width="309" height="135" rx="8" fill="#030712" stroke="#1f2937" stroke-width="1"/>
        
        <!-- Top navigation bar -->
        <rect x="12" y="10" width="45" height="14" rx="7" fill="#374151"/>
        <rect x="65" y="10" width="55" height="14" rx="7" fill="#3b82f6"/>
        <text x="92" y="21" text-anchor="middle" class="badge" fill="#ffffff">For you</text>
        <rect x="128" y="10" width="40" height="14" rx="7" fill="#374151"/>

        <!-- Hero Card with Movement Indicator -->
        <!-- Old Focus (Gray) -->
        <rect x="12" y="32" width="135" height="52" rx="6" fill="#1f2937" stroke="#374151" stroke-width="1"/>
        <text x="22" y="55" class="card-sub" font-size="10">上一焦点 (Previous)</text>
        <text x="22" y="70" class="label" font-size="11">推荐影片</text>

        <!-- New Focus (Glowing Border) -->
        <rect x="160" y="32" width="135" height="52" rx="6" fill="#1e293b" stroke="#60a5fa" stroke-width="2" filter="url(#glow)"/>
        <text x="170" y="55" class="label" fill="#93c5fd" font-size="10">当前聚焦 (Active Focus)</text>
        <text x="170" y="72" class="card-title" font-size="12">YouTube / 媒体</text>

        <!-- Jump Arrow -->
        <path d="M 148 58 L 158 58" stroke="#60a5fa" stroke-width="2"/>
        <polygon points="156,55 162,58 156,61" fill="#60a5fa"/>

        <!-- App row -->
        <rect x="12" y="94" width="60" height="30" rx="4" fill="#1f2937"/>
        <rect x="80" y="94" width="60" height="30" rx="4" fill="#1f2937"/>
        <rect x="148" y="94" width="60" height="30" rx="4" fill="#1f2937"/>
        <text x="220" y="112" class="code" font-size="10">&lt;= KEYCODE_DPAD_*</text>
      </g>
    </g>

    <!-- macOS Display Response Box -->
    <g transform="translate(0, 230)" filter="url(#subtle-shadow)">
      <rect width="345" height="200" rx="14" fill="url(#card-grad)" stroke="#374151" stroke-width="1.5"/>
      <text x="20" y="28" class="card-title" font-size="14">macOS: 动态加速度光标穿透</text>
      <text x="20" y="44" class="card-sub">微移精准像素级定位，甩动手势极速跨越 4K 双屏</text>

      <!-- macOS Desktop Preview -->
      <g transform="translate(18, 55)">
        <rect width="309" height="125" rx="8" fill="#030712" stroke="#1f2937" stroke-width="1"/>
        
        <!-- Menu bar -->
        <rect width="309" height="16" rx="4" fill="#1e293b"/>
        <text x="12" y="12" class="label" font-size="9" fill="#9ca3af"> Finder  文件  编辑  显示  窗口  帮助</text>
        <circle cx="285" cy="8" r="4" fill="#10b981"/>

        <!-- Trajectory curve -->
        <path d="M 40 100 Q 140 30 240 60" fill="none" stroke="#8b5cf6" stroke-width="2" stroke-dasharray="3 3"/>
        <!-- Start point -->
        <circle cx="40" cy="100" r="4" fill="#6b7280"/>
        <!-- Accelerated Point -->
        <circle cx="140" cy="55" r="5" fill="#a78bfa"/>
        <text x="110" y="80" class="code" font-size="9">加速度 x3.5</text>
        <!-- End Cursor -->
        <path d="M 240 60 L 240 76 L 245 72 L 252 82 L 255 80 L 248 70 L 255 70 Z" fill="#ffffff" stroke="#000000" stroke-width="1"/>
        <text x="220" y="105" class="label" fill="#c4b5fd" font-size="10">单击直接确认 (Select)</text>
      </g>
    </g>
  </g>

  <!-- Connectors from Engine to Targets -->
  <path d="M 575 220 L 615 220" stroke="#60a5fa" stroke-width="2"/>
  <polygon points="610,216 618,220 610,224" fill="#60a5fa"/>

  <path d="M 575 350 L 615 350" stroke="#a78bfa" stroke-width="2"/>
  <polygon points="610,346 618,350 610,354" fill="#a78bfa"/>
</svg>
"""

with open("docs/images/page_navigation_movement.svg", "w", encoding="utf-8") as f:
    f.write(svg_page_nav.strip())


# 2. Emulator Operation Flow Diagram
svg_emulator_flow = """<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 1000 620" width="100%" height="100%">
  <defs>
    <linearGradient id="bg-grad2" x1="0%" y1="0%" x2="100%" y2="100%">
      <stop offset="0%" stop-color="#090d16"/>
      <stop offset="50%" stop-color="#0f172a"/>
      <stop offset="100%" stop-color="#090d16"/>
    </linearGradient>
    <linearGradient id="box-grad" x1="0%" y1="0%" x2="100%" y2="100%">
      <stop offset="0%" stop-color="#1e293b" stop-opacity="0.9"/>
      <stop offset="100%" stop-color="#0f172a" stop-opacity="0.95"/>
    </linearGradient>
    <filter id="shadow2" x="-5%" y="-5%" width="110%" height="115%">
      <feDropShadow dx="0" dy="6" stdDeviation="10" flood-color="#000000" flood-opacity="0.6"/>
    </filter>
  </defs>

  <style>
    .title { font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, Helvetica, Arial, sans-serif; font-size: 24px; font-weight: 700; fill: #f8fafc; }
    .subtitle { font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, Helvetica, Arial, sans-serif; font-size: 13px; fill: #94a3b8; }
    .step-num { font-family: ui-monospace, monospace; font-size: 14px; font-weight: 800; fill: #38bdf8; }
    .heading { font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, Helvetica, Arial, sans-serif; font-size: 15px; font-weight: 600; fill: #f1f5f9; }
    .desc { font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, Helvetica, Arial, sans-serif; font-size: 12px; fill: #94a3b8; }
    .cmd { font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace; font-size: 11px; fill: #34d399; }
    .port { font-family: ui-monospace, monospace; font-size: 11px; font-weight: 600; fill: #fbbf24; }
  </style>

  <!-- Background -->
  <rect width="1000" height="620" rx="16" fill="url(#bg-grad2)"/>
  <rect width="998" height="618" x="1" y="1" rx="15" fill="none" stroke="#334155" stroke-width="1.5" stroke-opacity="0.6"/>

  <!-- Header -->
  <g transform="translate(40, 45)">
    <text class="title">Android TV 模拟器联调与 Bridge 桥接工作流</text>
    <text y="24" class="subtitle">解决 Android 模拟器运行在宿主机内部私有虚拟子网 (10.0.2.15) 无法被局域网真实 iPhone 发现的问题</text>
  </g>

  <!-- Step Cards (4-step Pipeline across top and middle) -->
  
  <!-- Step 1: Start Emulator -->
  <g transform="translate(40, 105)" filter="url(#shadow2)">
    <rect width="210" height="220" rx="12" fill="url(#box-grad)" stroke="#334155" stroke-width="1.5"/>
    <rect x="14" y="14" width="28" height="28" rx="6" fill="#0284c7" fill-opacity="0.2"/>
    <text x="28" y="33" text-anchor="middle" class="step-num">01</text>
    <text x="52" y="33" class="heading">启动电视模拟器</text>
    <text x="16" y="65" class="desc">在 Android Studio 或命令行中</text>
    <text x="16" y="83" class="desc">启动 Android TV AVD 镜像</text>
    <text x="16" y="110" class="cmd">$ emulator -avd \</text>
    <text x="24" y="126" class="cmd">  Android_TV_4K</text>
    <rect x="14" y="150" width="182" height="52" rx="6" fill="#090d16" stroke="#1e293b"/>
    <text x="24" y="170" class="desc" font-size="11">虚拟 IP: 10.0.2.15</text>
    <text x="24" y="188" class="desc" font-size="11">ADB 端口: 5554 (内部 5555)</text>
  </g>

  <!-- Arrow 1->2 -->
  <path d="M 252 215 L 283 215" stroke="#38bdf8" stroke-width="2"/>
  <polygon points="280,211 288,215 280,219" fill="#38bdf8"/>

  <!-- Step 2: Bridge Emulator Script -->
  <g transform="translate(285, 105)" filter="url(#shadow2)">
    <rect width="210" height="220" rx="12" fill="url(#box-grad)" stroke="#0284c7" stroke-width="1.5"/>
    <rect x="14" y="14" width="28" height="28" rx="6" fill="#0284c7" fill-opacity="0.3"/>
    <text x="28" y="33" text-anchor="middle" class="step-num">02</text>
    <text x="52" y="33" class="heading">运行 Bridge 脚本</text>
    <text x="16" y="65" class="desc">自动创建 ADB 端口映射</text>
    <text x="16" y="83" class="desc">并接管宿主机网络广播</text>
    <text x="16" y="110" class="cmd">$ python3 scripts/ \</text>
    <text x="24" y="126" class="cmd">  bridge_emulator.py start</text>
    <rect x="14" y="150" width="182" height="52" rx="6" fill="#090d16" stroke="#1e293b"/>
    <text x="24" y="170" class="port">adb forward tcp:49152</text>
    <text x="24" y="188" class="port">adb forward tcp:49153</text>
  </g>

  <!-- Arrow 2->3 -->
  <path d="M 497 215 L 528 215" stroke="#38bdf8" stroke-width="2"/>
  <polygon points="525,211 533,215 525,219" fill="#38bdf8"/>

  <!-- Step 3: macOS mDNS Broadcast -->
  <g transform="translate(530, 105)" filter="url(#shadow2)">
    <rect width="210" height="220" rx="12" fill="url(#box-grad)" stroke="#334155" stroke-width="1.5"/>
    <rect x="14" y="14" width="28" height="28" rx="6" fill="#8b5cf6" fill-opacity="0.2"/>
    <text x="28" y="33" text-anchor="middle" class="step-num" fill="#a78bfa">03</text>
    <text x="52" y="33" class="heading">宿主机 mDNS 广播</text>
    <text x="16" y="65" class="desc">利用 dns-sd 代理向局域网</text>
    <text x="16" y="83" class="desc">物理 Wi-Fi 广播 Bonjour 服务</text>
    <text x="16" y="110" class="code" font-size="10">_mediaremotetv._tcp</text>
    <text x="16" y="128" class="code" font-size="10">_companion-link._tcp</text>
    <rect x="14" y="150" width="182" height="52" rx="6" fill="#090d16" stroke="#1e293b"/>
    <text x="24" y="170" class="desc" font-size="11">设备名: Android TV</text>
    <text x="24" y="188" class="desc" font-size="11">广播 IP: 宿主机真实 LAN IP</text>
  </g>

  <!-- Arrow 3->4 -->
  <path d="M 742 215 L 773 215" stroke="#38bdf8" stroke-width="2"/>
  <polygon points="770,211 778,215 770,219" fill="#38bdf8"/>

  <!-- Step 4: iPhone Remote Discovery & Pairing -->
  <g transform="translate(775, 105)" filter="url(#shadow2)">
    <rect width="185" height="220" rx="12" fill="url(#box-grad)" stroke="#10b981" stroke-width="1.5"/>
    <rect x="14" y="14" width="28" height="28" rx="6" fill="#10b981" fill-opacity="0.2"/>
    <text x="28" y="33" text-anchor="middle" class="step-num" fill="#34d399">04</text>
    <text x="52" y="33" class="heading">手机发现与配对</text>
    <text x="16" y="65" class="desc">iPhone 下拉控制中心</text>
    <text x="16" y="83" class="desc">自动显示模拟器设备项</text>
    <rect x="14" y="105" width="157" height="36" rx="6" fill="#1e293b"/>
    <text x="24" y="127" class="label" fill="#93c5fd">配对码显示在屏幕上</text>
    <rect x="14" y="150" width="157" height="52" rx="6" fill="#090d16" stroke="#1e293b"/>
    <text x="24" y="170" class="desc" font-size="11" fill="#34d399">● 加密会话建立成功</text>
    <text x="24" y="188" class="desc" font-size="11">直连模拟器流畅操控</text>
  </g>

  <!-- Bottom Panel: In-Depth Port Mapping & Command Architecture -->
  <g transform="translate(40, 350)" filter="url(#shadow2)">
    <rect width="920" height="230" rx="14" fill="url(#box-grad)" stroke="#334155" stroke-width="1.5"/>
    
    <text x="24" y="32" class="heading" font-size="16">模拟器底层网络端口转发拓扑图 / Emulator Port Forwarding Topology</text>
    
    <!-- Left Lane: Physical Network -->
    <g transform="translate(24, 55)">
      <rect width="250" height="150" rx="8" fill="#0f172a" stroke="#3b82f6" stroke-width="1.2"/>
      <text x="16" y="26" class="heading" fill="#60a5fa" font-size="13">物理网络 (Physical Wi-Fi)</text>
      <text x="16" y="44" class="desc">iPhone / iPad 控制中心</text>
      <path d="M 16 58 L 234 58" stroke="#1e293b" stroke-width="1"/>
      <text x="16" y="78" class="desc">广播接收: Bonjour UDP 5353</text>
      <text x="16" y="98" class="desc">TCP 连接目标: 宿主机 LAN IP</text>
      <text x="16" y="125" class="code" font-size="10">端口: 49152 (MRP) / 49153 (C-Link)</text>
    </g>

    <!-- Mid Lane: Host Mac (Bridge) -->
    <g transform="translate(335, 55)">
      <rect width="250" height="150" rx="8" fill="#0f172a" stroke="#0284c7" stroke-width="1.2"/>
      <text x="16" y="26" class="heading" fill="#38bdf8" font-size="13">宿主机 (Host Mac)</text>
      <text x="16" y="44" class="desc">bridge_emulator.py + ADB</text>
      <path d="M 16 58 L 234 58" stroke="#1e293b" stroke-width="1"/>
      <text x="16" y="80" class="port">adb forward tcp:49152</text>
      <text x="16" y="102" class="port">adb forward tcp:49153</text>
      <text x="16" y="124" class="port">adb forward tcp:49154</text>
      <text x="16" y="142" class="desc" font-size="10">无缝透传到 Emulator NAT</text>
    </g>

    <!-- Right Lane: Android TV Emulator Guest -->
    <g transform="translate(645, 55)">
      <rect width="250" height="150" rx="8" fill="#0f172a" stroke="#10b981" stroke-width="1.2"/>
      <text x="16" y="26" class="heading" fill="#34d399" font-size="13">模拟器沙盒 (Guest AVD)</text>
      <text x="16" y="44" class="desc">com.corvofeng.fakeatv</text>
      <path d="M 16 58 L 234 58" stroke="#1e293b" stroke-width="1"/>
      <text x="16" y="78" class="desc">libatv_android.so 监听 0.0.0.0</text>
      <text x="16" y="98" class="desc">dadb 直连 127.0.0.1:5555</text>
      <text x="16" y="122" class="cmd" font-size="10">input keyevent KEYCODE_*</text>
      <text x="16" y="138" class="desc" font-size="10">或无障碍辅助驱动注入</text>
    </g>

    <!-- Connectors -->
    <path d="M 276 130 L 333 130" stroke="#38bdf8" stroke-width="2" stroke-dasharray="3 3"/>
    <polygon points="328,126 335,130 328,134" fill="#38bdf8"/>

    <path d="M 587 130 L 643 130" stroke="#10b981" stroke-width="2" stroke-dasharray="3 3"/>
    <polygon points="638,126 645,130 638,134" fill="#10b981"/>
  </g>
</svg>
"""

with open("docs/images/emulator_operation_flow.svg", "w", encoding="utf-8") as f:
    f.write(svg_emulator_flow.strip())


# 3. Prerequisites & Permissions Diagram
svg_prerequisites = """<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 1000 640" width="100%" height="100%">
  <defs>
    <linearGradient id="bg-grad3" x1="0%" y1="0%" x2="100%" y2="100%">
      <stop offset="0%" stop-color="#0a0f1d"/>
      <stop offset="50%" stop-color="#0e1726"/>
      <stop offset="100%" stop-color="#0a0f1d"/>
    </linearGradient>
    <linearGradient id="box-grad3" x1="0%" y1="0%" x2="100%" y2="100%">
      <stop offset="0%" stop-color="#1e293b" stop-opacity="0.95"/>
      <stop offset="100%" stop-color="#0f172a" stop-opacity="0.98"/>
    </linearGradient>
    <filter id="shadow3" x="-5%" y="-5%" width="110%" height="115%">
      <feDropShadow dx="0" dy="6" stdDeviation="10" flood-color="#000000" flood-opacity="0.5"/>
    </filter>
  </defs>

  <style>
    .title { font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, Helvetica, Arial, sans-serif; font-size: 24px; font-weight: 700; fill: #f8fafc; }
    .subtitle { font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, Helvetica, Arial, sans-serif; font-size: 13px; fill: #94a3b8; }
    .heading { font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, Helvetica, Arial, sans-serif; font-size: 16px; font-weight: 600; fill: #f8fafc; }
    .badge { font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, Helvetica, Arial, sans-serif; font-size: 11px; font-weight: 700; }
    .desc { font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, Helvetica, Arial, sans-serif; font-size: 12px; fill: #cbd5e1; line-height: 1.5; }
    .alert-txt { font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, Helvetica, Arial, sans-serif; font-size: 11px; font-weight: 600; fill: #fbbf24; }
    .code { font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace; font-size: 11px; fill: #38bdf8; }
  </style>

  <!-- Background -->
  <rect width="1000" height="640" rx="16" fill="url(#bg-grad3)"/>
  <rect width="998" height="638" x="1" y="1" rx="15" fill="none" stroke="#334155" stroke-width="1.5" stroke-opacity="0.6"/>

  <!-- Header -->
  <g transform="translate(40, 42)">
    <text class="title">前置条件与核心权限核对指南 / Prerequisites &amp; Permissions</text>
    <text y="24" class="subtitle">确保 Android TV / 模拟器与 macOS 端各项系统级调试与辅助权限正确开启</text>
  </g>

  <!-- 4 Pillars of Requirements -->

  <!-- Pillar 1: Android TV ADB Network Debugging -->
  <g transform="translate(40, 95)" filter="url(#shadow3)">
    <rect width="445" height="240" rx="12" fill="url(#box-grad3)" stroke="#f59e0b" stroke-width="1.5"/>
    
    <!-- Title & Badge -->
    <rect x="18" y="16" width="60" height="22" rx="4" fill="#f59e0b" fill-opacity="0.2"/>
    <text x="48" y="32" text-anchor="middle" class="badge" fill="#fbbf24">必要条件 1</text>
    <text x="88" y="33" class="heading">Android TV: 开启网络 ADB 调试</text>

    <!-- Details -->
    <text x="18" y="66" class="desc">● 为什么必须开启 ADB？</text>
    <text x="32" y="85" class="desc" fill="#94a3b8">App 内置 Dadb 高性能驱动，需直连 127.0.0.1:5555 注入全局</text>
    <text x="32" y="103" class="desc" fill="#94a3b8">按键 (D-pad/Select/Home/Back/Volume) 并支持光标触摸。</text>
    
    <!-- Action steps -->
    <rect x="18" y="118" width="409" height="64" rx="6" fill="#090d16" stroke="#1e293b"/>
    <text x="28" y="138" class="desc" font-weight="600" fill="#f8fafc">开启路径 (系统设置):</text>
    <text x="28" y="156" class="desc">关于 -&gt; 连续点击「内部版本号 (Build)」7 次开启开发者选项</text>
    <text x="28" y="172" class="desc">开发者选项 -&gt; 启用「网络调试 (Network ADB)」与「USB 调试」</text>

    <!-- Quick terminal command -->
    <rect x="18" y="190" width="409" height="34" rx="6" fill="#172554"/>
    <text x="28" y="211" class="code">命令行验证: adb connect &lt;TV_IP&gt;:5555</text>
  </g>

  <!-- Pillar 2: Android TV ADB Authorization Dialog -->
  <g transform="translate(515, 95)" filter="url(#shadow3)">
    <rect width="445" height="240" rx="12" fill="url(#box-grad3)" stroke="#10b981" stroke-width="1.5"/>
    
    <!-- Title & Badge -->
    <rect x="18" y="16" width="60" height="22" rx="4" fill="#10b981" fill-opacity="0.2"/>
    <text x="48" y="32" text-anchor="middle" class="badge" fill="#34d399">关键步骤 2</text>
    <text x="88" y="33" class="heading">接受 ADB 调试授权与签名</text>

    <!-- Details -->
    <text x="18" y="66" class="desc">● 首次连接必定弹出系统安全指纹授权窗：</text>
    <text x="32" y="85" class="alert-txt">⚠️ 务必勾选「始终允许来自此计算机」并点击「允许 / OK」！</text>
    <text x="32" y="103" class="desc" fill="#94a3b8">若拒绝或未勾选，本地 ADB 守护进程将由于 Unauthorized 报错。</text>

    <!-- Mock Dialog -->
    <rect x="18" y="118" width="409" height="106" rx="6" fill="#090d16" stroke="#374151"/>
    <text x="32" y="142" class="heading" font-size="13">Allow USB debugging? / 允许调试吗？</text>
    <text x="32" y="162" class="desc" font-size="11" fill="#94a3b8">RSA key fingerprint: SHA256:4K...AppleTVRemote</text>
    
    <!-- Checkbox checked -->
    <rect x="32" y="174" width="16" height="16" rx="3" fill="#10b981"/>
    <path d="M 35 182 L 39 186 L 45 178" stroke="#ffffff" stroke-width="2" fill="none"/>
    <text x="56" y="187" class="desc" font-weight="600" fill="#f8fafc">Always allow from this computer (始终允许)</text>

    <!-- Allow Button -->
    <rect x="330" y="170" width="75" height="28" rx="4" fill="#10b981"/>
    <text x="367" y="188" text-anchor="middle" class="label" fill="#ffffff">ALLOW 允许</text>
  </g>

  <!-- Pillar 3: Accessibility Permission -->
  <g transform="translate(40, 360)" filter="url(#shadow3)">
    <rect width="445" height="240" rx="12" fill="url(#box-grad3)" stroke="#3b82f6" stroke-width="1.5"/>
    
    <!-- Title & Badge -->
    <rect x="18" y="16" width="60" height="22" rx="4" fill="#3b82f6" fill-opacity="0.2"/>
    <text x="48" y="32" text-anchor="middle" class="badge" fill="#60a5fa">必要条件 3</text>
    <text x="88" y="33" class="heading">开启无障碍服务权限 (Accessibility)</text>

    <!-- Details -->
    <text x="18" y="66" class="desc">● 兜底按键与系统级 Back / Home 交互支持：</text>
    <text x="32" y="85" class="desc" fill="#94a3b8">在部分未开启 ADB 的受限电视，无障碍服务作为安全执行器，</text>
    <text x="32" y="103" class="desc" fill="#94a3b8">可派发 GLOBAL_ACTION_BACK 与点击事件。</text>

    <!-- Steps -->
    <rect x="18" y="118" width="409" height="64" rx="6" fill="#090d16" stroke="#1e293b"/>
    <text x="28" y="138" class="desc" font-weight="600" fill="#f8fafc">开启步骤:</text>
    <text x="28" y="156" class="desc">点击 App 主界面「ACCESSIBILITY SETTINGS」按钮</text>
    <text x="28" y="172" class="desc">找到「Apple TV Remote Receiver」-&gt; 开启并点击「确定」</text>

    <!-- ADB fast enable -->
    <rect x="18" y="190" width="409" height="34" rx="6" fill="#1e1b4b"/>
    <text x="28" y="211" class="code" font-size="10">极速开启: adb shell settings put secure accessibility_enabled 1</text>
  </g>

  <!-- Pillar 4: macOS Gatekeeper & Permissions -->
  <g transform="translate(515, 360)" filter="url(#shadow3)">
    <rect width="445" height="240" rx="12" fill="url(#box-grad3)" stroke="#a855f7" stroke-width="1.5"/>
    
    <!-- Title & Badge -->
    <rect x="18" y="16" width="60" height="22" rx="4" fill="#a855f7" fill-opacity="0.2"/>
    <text x="48" y="32" text-anchor="middle" class="badge" fill="#c084fc">必要条件 4</text>
    <text x="88" y="33" class="heading">macOS: 签名信任与辅助功能</text>

    <!-- Details -->
    <text x="18" y="66" class="desc">● 自签名证书受 Gatekeeper 拦截与光标控制授权：</text>
    <text x="32" y="85" class="desc" fill="#94a3b8">macOS 默认阻止未公证 App。需导入自签名证书或移除隔离属性；</text>
    <text x="32" y="103" class="desc" fill="#94a3b8">控制鼠标光标与多媒体键需在「隐私与安全性 -&gt; 辅助功能」中授权。</text>

    <!-- Solutions -->
    <rect x="18" y="118" width="409" height="106" rx="6" fill="#090d16" stroke="#1e293b"/>
    <text x="28" y="136" class="code" font-size="10"># 方案 1: 移除隔离标记 (推荐快捷操作)</text>
    <text x="28" y="152" class="code" font-size="10">xattr -dr com.apple.quarantine /Applications/AppleTVRemote.app</text>
    <text x="28" y="172" class="code" font-size="10"># 方案 2: 导入项目自签名根证书</text>
    <text x="28" y="188" class="code" font-size="10">./scripts/import_certificate.sh Corvo_Development.p12</text>
    <text x="28" y="208" class="desc" font-size="11" fill="#c084fc">并在「系统设置 -&gt; 隐私与安全性 -&gt; 辅助功能」勾选 AppleTVRemote</text>
  </g>
</svg>
"""

with open("docs/images/prerequisites_and_permissions.svg", "w", encoding="utf-8") as f:
    f.write(svg_prerequisites.strip())

print("Successfully generated all diagrams!")
