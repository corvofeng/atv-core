#!/usr/bin/env python3
"""
Generate comprehensive English diagrams and UI screenshots for English documentation (README.md):
- docs/images/en/page_navigation_movement.svg
- docs/images/en/emulator_operation_flow.svg
- docs/images/en/prerequisites_and_permissions.svg
- docs/images/en/mac_browser_inspector.png
- docs/images/en/mac_debug_web_page.png
- docs/images/en/android_tv_main_screen_ready.png
- docs/images/en/android_tv_injection_mode_dialog.png
- docs/images/en/android_tv_menu_binding_dialog.png
And copies shared English screenshots.
"""

import os
import shutil
import subprocess

os.makedirs("docs/images/en", exist_ok=True)
os.makedirs("docs/images/zh", exist_ok=True)

# 1. English: page_navigation_movement.svg
svg_page_nav_en = """<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 1000 620" width="100%" height="100%">
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
    <filter id="glow" x="-20%" y="-20%" width="140%" height="140%">
      <feGaussianBlur stdDeviation="8" result="blur" />
      <feComposite in="SourceGraphic" in2="blur" operator="over"/>
    </filter>
    <filter id="subtle-shadow" x="-5%" y="-5%" width="110%" height="115%">
      <feDropShadow dx="0" dy="8" stdDeviation="12" flood-color="#000000" flood-opacity="0.5"/>
    </filter>
  </defs>

  <style>
    .title { font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, Helvetica, Arial, sans-serif; font-size: 23px; font-weight: 700; fill: #f3f4f6; }
    .subtitle { font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, Helvetica, Arial, sans-serif; font-size: 13px; fill: #9ca3af; }
    .card-title { font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, Helvetica, Arial, sans-serif; font-size: 15px; font-weight: 600; fill: #ffffff; }
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
    <text class="title">Remote Swipe Gesture &amp; Page Navigation Flow</text>
    <text y="24" class="subtitle">iOS Control Center Apple TV Remote gestures translate into Android TV focus shifts and macOS cursor motion</text>
  </g>

  <!-- Left: iPhone Apple TV Remote Mockup -->
  <g transform="translate(50, 110)" filter="url(#subtle-shadow)">
    <rect width="240" height="460" rx="36" fill="#18181b" stroke="#3f3f46" stroke-width="2"/>
    <rect width="232" height="452" x="4" y="4" rx="32" fill="#09090b"/>

    <text x="120" y="38" text-anchor="middle" class="card-title" font-size="14">Apple TV Remote</text>
    <text x="120" y="54" text-anchor="middle" class="card-sub" font-size="11">Connected • TLS Encrypted</text>

    <!-- Trackpad Area -->
    <rect x="25" y="75" width="190" height="210" rx="20" fill="#18181b" stroke="#3b82f6" stroke-width="1.5" stroke-dasharray="4 3"/>
    
    <!-- Gesture Visualization -->
    <path d="M 120 220 L 120 120" stroke="#60a5fa" stroke-width="3" stroke-linecap="round"/>
    <polygon points="113,124 120,108 127,124" fill="#60a5fa"/>
    <circle cx="120" cy="220" r="14" fill="#3b82f6" fill-opacity="0.3" stroke="#60a5fa" stroke-width="2"/>
    <circle cx="120" cy="220" r="6" fill="#60a5fa"/>
    <text x="120" y="255" text-anchor="middle" class="label" fill="#93c5fd">Swipe UP</text>
    <text x="120" y="272" text-anchor="middle" class="code" font-size="10">dx=0, dy=-38.5, phase=Moved</text>

    <!-- Buttons Grid -->
    <rect x="35" y="305" width="75" height="42" rx="21" fill="#27272a"/>
    <text x="72" y="331" text-anchor="middle" class="label" font-size="14">&lt;</text>
    
    <rect x="130" y="305" width="75" height="42" rx="21" fill="#27272a"/>
    <text x="167" y="330" text-anchor="middle" class="label" font-size="12">📺</text>

    <rect x="35" y="357" width="75" height="42" rx="21" fill="#27272a"/>
    <text x="72" y="382" text-anchor="middle" class="label" font-size="13">⏯</text>
    
    <rect x="130" y="357" width="75" height="42" rx="21" fill="#27272a"/>
    <text x="167" y="382" text-anchor="middle" class="label" font-size="13">🔇</text>

    <rect x="35" y="409" width="75" height="34" rx="17" fill="#27272a"/>
    <text x="72" y="430" text-anchor="middle" class="label" font-size="11">VOL +/-</text>
    
    <rect x="130" y="409" width="75" height="34" rx="17" fill="#3b82f6" fill-opacity="0.2" stroke="#3b82f6" stroke-width="1"/>
    <text x="167" y="430" text-anchor="middle" class="label" fill="#93c5fd" font-size="11">Mode / Siri</text>
  </g>

  <!-- Middle: Dispatch & Protocol Engine -->
  <g transform="translate(325, 120)">
    <rect width="250" height="440" rx="16" fill="url(#card-grad)" stroke="#374151" stroke-width="1.5" filter="url(#subtle-shadow)"/>
    
    <text x="125" y="32" text-anchor="middle" class="card-title">atv-core Dispatch Engine</text>
    <text x="125" y="52" text-anchor="middle" class="card-sub">Companion Link / MRP State Machine</text>

    <!-- Step 1 -->
    <g transform="translate(18, 75)">
      <rect width="214" height="68" rx="8" fill="#1f2937" stroke="#4b5563" stroke-width="1"/>
      <text x="12" y="22" class="label" fill="#60a5fa" font-weight="700">1. OPACK Decryption</text>
      <text x="12" y="40" class="card-sub">ChaCha20-Poly1305 Payload</text>
      <text x="12" y="56" class="code">_touchMove / _hidEvent</text>
    </g>

    <!-- Step 2 -->
    <g transform="translate(18, 155)">
      <rect width="214" height="90" rx="8" fill="#1f2937" stroke="#8b5cf6" stroke-width="1"/>
      <text x="12" y="22" class="label" fill="#a78bfa" font-weight="700">2. Mode &amp; Ballistics Parsing</text>
      <text x="12" y="40" class="card-sub">D-pad Mode (Grid Threshold)</text>
      <text x="12" y="58" class="card-sub">Mouse Mode (Dynamic Ballistics)</text>
      <text x="12" y="76" class="code">dy &lt; -25px =&gt; KEY_UP / Y -= 38</text>
    </g>

    <!-- Step 3 -->
    <g transform="translate(18, 258)">
      <rect width="214" height="85" rx="8" fill="#1f2937" stroke="#10b981" stroke-width="1"/>
      <text x="12" y="22" class="label" fill="#34d399" font-weight="700">3. High-Fidelity Dispatch</text>
      <text x="12" y="40" class="card-sub">TV: LocalAdbDriver / A11y</text>
      <text x="12" y="58" class="card-sub">Mac: CGEvent / NX_KEYTYPE</text>
      <text x="12" y="74" class="code">Latency &lt; 8ms • Near-Zero Delay</text>
    </g>

    <!-- Dispatch status badge -->
    <g transform="translate(18, 360)">
      <rect width="214" height="60" rx="8" fill="#111827" stroke="#374151" stroke-width="1"/>
      <circle cx="22" cy="30" r="5" fill="#10b981"/>
      <text x="35" y="26" class="label">Local Accumulator Ready</text>
      <text x="35" y="44" class="card-sub">Zero jitter &amp; sub-millisecond sync</text>
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
      <text x="20" y="28" class="card-title" font-size="14">Android TV: Seamless Focus Navigation</text>
      <text x="20" y="44" class="card-sub">Swipe immediately shifts active UI card in Launcher</text>

      <g transform="translate(18, 55)">
        <rect width="309" height="135" rx="8" fill="#030712" stroke="#1f2937" stroke-width="1"/>
        
        <!-- Top navigation bar -->
        <rect x="12" y="10" width="45" height="14" rx="7" fill="#374151"/>
        <rect x="65" y="10" width="55" height="14" rx="7" fill="#3b82f6"/>
        <text x="92" y="21" text-anchor="middle" class="badge" fill="#ffffff">For you</text>
        <rect x="128" y="10" width="40" height="14" rx="7" fill="#374151"/>

        <!-- Old Focus -->
        <rect x="12" y="32" width="135" height="52" rx="6" fill="#1f2937" stroke="#374151" stroke-width="1"/>
        <text x="22" y="55" class="card-sub" font-size="10">Previous Focus</text>
        <text x="22" y="70" class="label" font-size="11">Featured Content</text>

        <!-- New Focus (Glowing Border) -->
        <rect x="160" y="32" width="135" height="52" rx="6" fill="#1e293b" stroke="#60a5fa" stroke-width="2" filter="url(#glow)"/>
        <text x="170" y="55" class="label" fill="#93c5fd" font-size="10">Active Focus</text>
        <text x="170" y="72" class="card-title" font-size="12">YouTube / Media</text>

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
      <text x="20" y="28" class="card-title" font-size="14">macOS: Dynamic Ballistics Cursor</text>
      <text x="20" y="44" class="card-sub">Pixel-perfect micro adjustments, flick across 4K dual screens</text>

      <g transform="translate(18, 55)">
        <rect width="309" height="125" rx="8" fill="#030712" stroke="#1f2937" stroke-width="1"/>
        
        <!-- Menu bar -->
        <rect width="309" height="16" rx="4" fill="#1e293b"/>
        <text x="12" y="12" class="label" font-size="9" fill="#9ca3af"> Finder  File  Edit  View  Window  Help</text>
        <circle cx="285" cy="8" r="4" fill="#10b981"/>

        <!-- Trajectory curve -->
        <path d="M 40 100 Q 140 30 240 60" fill="none" stroke="#8b5cf6" stroke-width="2" stroke-dasharray="3 3"/>
        <circle cx="40" cy="100" r="4" fill="#6b7280"/>
        <circle cx="140" cy="55" r="5" fill="#a78bfa"/>
        <text x="110" y="80" class="code" font-size="9">Accel x3.5</text>
        <path d="M 240 60 L 240 76 L 245 72 L 252 82 L 255 80 L 248 70 L 255 70 Z" fill="#ffffff" stroke="#000000" stroke-width="1"/>
        <text x="220" y="105" class="label" fill="#c4b5fd" font-size="10">Click to Select (Enter)</text>
      </g>
    </g>
  </g>

  <!-- Connectors -->
  <path d="M 575 220 L 615 220" stroke="#60a5fa" stroke-width="2"/>
  <polygon points="610,216 618,220 610,224" fill="#60a5fa"/>

  <path d="M 575 350 L 615 350" stroke="#a78bfa" stroke-width="2"/>
  <polygon points="610,346 618,350 610,354" fill="#a78bfa"/>
</svg>
"""

with open("docs/images/en/page_navigation_movement.svg", "w", encoding="utf-8") as f:
    f.write(svg_page_nav_en.strip())


# 2. English: emulator_operation_flow.svg
svg_emulator_flow_en = """<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 1000 620" width="100%" height="100%">
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
    .title { font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, Helvetica, Arial, sans-serif; font-size: 23px; font-weight: 700; fill: #f8fafc; }
    .subtitle { font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, Helvetica, Arial, sans-serif; font-size: 13px; fill: #94a3b8; }
    .step-num { font-family: ui-monospace, monospace; font-size: 14px; font-weight: 800; fill: #38bdf8; }
    .heading { font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, Helvetica, Arial, sans-serif; font-size: 14px; font-weight: 600; fill: #f1f5f9; }
    .desc { font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, Helvetica, Arial, sans-serif; font-size: 12px; fill: #94a3b8; }
    .cmd { font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace; font-size: 11px; fill: #34d399; }
    .port { font-family: ui-monospace, monospace; font-size: 11px; font-weight: 600; fill: #fbbf24; }
  </style>

  <rect width="1000" height="620" rx="16" fill="url(#bg-grad2)"/>
  <rect width="998" height="618" x="1" y="1" rx="15" fill="none" stroke="#334155" stroke-width="1.5" stroke-opacity="0.6"/>

  <g transform="translate(40, 45)">
    <text class="title">Android TV Emulator Debugging &amp; Bridge Workflow</text>
    <text y="24" class="subtitle">Bridges host-only emulator sandbox NAT (10.0.2.15) to physical Wi-Fi network for physical iPhone discovery</text>
  </g>

  <!-- Step 1 -->
  <g transform="translate(40, 105)" filter="url(#shadow2)">
    <rect width="210" height="220" rx="12" fill="url(#box-grad)" stroke="#334155" stroke-width="1.5"/>
    <rect x="14" y="14" width="28" height="28" rx="6" fill="#0284c7" fill-opacity="0.2"/>
    <text x="28" y="33" text-anchor="middle" class="step-num">01</text>
    <text x="52" y="33" class="heading">Launch TV Emulator</text>
    <text x="16" y="65" class="desc">Start Android TV AVD</text>
    <text x="16" y="83" class="desc">via Studio or Terminal</text>
    <text x="16" y="110" class="cmd">$ emulator -avd \</text>
    <text x="24" y="126" class="cmd">  Android_TV_4K</text>
    <rect x="14" y="150" width="182" height="52" rx="6" fill="#090d16" stroke="#1e293b"/>
    <text x="24" y="170" class="desc" font-size="11">Virtual IP: 10.0.2.15</text>
    <text x="24" y="188" class="desc" font-size="11">ADB Port: 5554 (Internal 5555)</text>
  </g>

  <!-- Arrow 1->2 -->
  <path d="M 252 215 L 283 215" stroke="#38bdf8" stroke-width="2"/>
  <polygon points="280,211 288,215 280,219" fill="#38bdf8"/>

  <!-- Step 2 -->
  <g transform="translate(285, 105)" filter="url(#shadow2)">
    <rect width="210" height="220" rx="12" fill="url(#box-grad)" stroke="#0284c7" stroke-width="1.5"/>
    <rect x="14" y="14" width="28" height="28" rx="6" fill="#0284c7" fill-opacity="0.3"/>
    <text x="28" y="33" text-anchor="middle" class="step-num">02</text>
    <text x="52" y="33" class="heading">Run Bridge Suite</text>
    <text x="16" y="65" class="desc">Auto ADB port forwarding</text>
    <text x="16" y="83" class="desc">&amp; proxy Bonjour mDNS</text>
    <text x="16" y="110" class="cmd">$ python3 scripts/ \</text>
    <text x="24" y="126" class="cmd">  bridge_emulator.py start</text>
    <rect x="14" y="150" width="182" height="52" rx="6" fill="#090d16" stroke="#1e293b"/>
    <text x="24" y="170" class="port">adb forward tcp:49152</text>
    <text x="24" y="188" class="port">adb forward tcp:49153</text>
  </g>

  <!-- Arrow 2->3 -->
  <path d="M 497 215 L 528 215" stroke="#38bdf8" stroke-width="2"/>
  <polygon points="525,211 533,215 525,219" fill="#38bdf8"/>

  <!-- Step 3 -->
  <g transform="translate(530, 105)" filter="url(#shadow2)">
    <rect width="210" height="220" rx="12" fill="url(#box-grad)" stroke="#334155" stroke-width="1.5"/>
    <rect x="14" y="14" width="28" height="28" rx="6" fill="#8b5cf6" fill-opacity="0.2"/>
    <text x="28" y="33" text-anchor="middle" class="step-num" fill="#a78bfa">03</text>
    <text x="52" y="33" class="heading">Host mDNS Broadcast</text>
    <text x="16" y="65" class="desc">Proxy advertises Bonjour</text>
    <text x="16" y="83" class="desc">services on host Wi-Fi LAN</text>
    <text x="16" y="110" class="code" font-size="10">_mediaremotetv._tcp</text>
    <text x="16" y="128" class="code" font-size="10">_companion-link._tcp</text>
    <rect x="14" y="150" width="182" height="52" rx="6" fill="#090d16" stroke="#1e293b"/>
    <text x="24" y="170" class="desc" font-size="11">Device: Android TV</text>
    <text x="24" y="188" class="desc" font-size="11">IP: Mac Host Physical LAN IP</text>
  </g>

  <!-- Arrow 3->4 -->
  <path d="M 742 215 L 773 215" stroke="#38bdf8" stroke-width="2"/>
  <polygon points="770,211 778,215 770,219" fill="#38bdf8"/>

  <!-- Step 4 -->
  <g transform="translate(775, 105)" filter="url(#shadow2)">
    <rect width="185" height="220" rx="12" fill="url(#box-grad)" stroke="#10b981" stroke-width="1.5"/>
    <rect x="14" y="14" width="28" height="28" rx="6" fill="#10b981" fill-opacity="0.2"/>
    <text x="28" y="33" text-anchor="middle" class="step-num" fill="#34d399">04</text>
    <text x="52" y="33" class="heading">Discover &amp; Pair</text>
    <text x="16" y="65" class="desc">Swipe down iOS Control</text>
    <text x="16" y="83" class="desc">Center &amp; tap TV Remote</text>
    <rect x="14" y="105" width="157" height="36" rx="6" fill="#1e293b"/>
    <text x="24" y="127" class="label" fill="#93c5fd">Code shown on screen</text>
    <rect x="14" y="150" width="157" height="52" rx="6" fill="#090d16" stroke="#1e293b"/>
    <text x="24" y="170" class="desc" font-size="11" fill="#34d399">● TLS Session Active</text>
    <text x="24" y="188" class="desc" font-size="11">Direct zero-lag control</text>
  </g>

  <!-- Bottom Panel -->
  <g transform="translate(40, 350)" filter="url(#shadow2)">
    <rect width="920" height="230" rx="14" fill="url(#box-grad)" stroke="#334155" stroke-width="1.5"/>
    <text x="24" y="32" class="heading" font-size="16">Underlying Network Port Forwarding Topology</text>
    
    <!-- Left Lane -->
    <g transform="translate(24, 55)">
      <rect width="250" height="150" rx="8" fill="#0f172a" stroke="#3b82f6" stroke-width="1.2"/>
      <text x="16" y="26" class="heading" fill="#60a5fa" font-size="13">Physical Wi-Fi Network</text>
      <text x="16" y="44" class="desc">iPhone / iPad Control Center</text>
      <path d="M 16 58 L 234 58" stroke="#1e293b" stroke-width="1"/>
      <text x="16" y="78" class="desc">mDNS Discovery: UDP 5353</text>
      <text x="16" y="98" class="desc">TCP Target: Host Physical LAN IP</text>
      <text x="16" y="125" class="code" font-size="10">Ports: 49152 (MRP) / 49153 (C-Link)</text>
    </g>

    <!-- Mid Lane -->
    <g transform="translate(335, 55)">
      <rect width="250" height="150" rx="8" fill="#0f172a" stroke="#0284c7" stroke-width="1.2"/>
      <text x="16" y="26" class="heading" fill="#38bdf8" font-size="13">Host Mac (Bridge Suite)</text>
      <text x="16" y="44" class="desc">bridge_emulator.py + ADB</text>
      <path d="M 16 58 L 234 58" stroke="#1e293b" stroke-width="1"/>
      <text x="16" y="80" class="port">adb forward tcp:49152</text>
      <text x="16" y="102" class="port">adb forward tcp:49153</text>
      <text x="16" y="124" class="port">adb forward tcp:49154</text>
      <text x="16" y="142" class="desc" font-size="10">Transparent port forwarding</text>
    </g>

    <!-- Right Lane -->
    <g transform="translate(645, 55)">
      <rect width="250" height="150" rx="8" fill="#0f172a" stroke="#10b981" stroke-width="1.2"/>
      <text x="16" y="26" class="heading" fill="#34d399" font-size="13">Guest AVD Sandbox</text>
      <text x="16" y="44" class="desc">com.corvofeng.fakeatv</text>
      <path d="M 16 58 L 234 58" stroke="#1e293b" stroke-width="1"/>
      <text x="16" y="78" class="desc">libatv_android.so binds 0.0.0.0</text>
      <text x="16" y="98" class="desc">dadb connects 127.0.0.1:5555</text>
      <text x="16" y="122" class="cmd" font-size="10">input keyevent KEYCODE_*</text>
      <text x="16" y="138" class="desc" font-size="10">or Accessibility fallback</text>
    </g>

    <path d="M 276 130 L 333 130" stroke="#38bdf8" stroke-width="2" stroke-dasharray="3 3"/>
    <polygon points="328,126 335,130 328,134" fill="#38bdf8"/>

    <path d="M 587 130 L 643 130" stroke="#10b981" stroke-width="2" stroke-dasharray="3 3"/>
    <polygon points="638,126 645,130 638,134" fill="#10b981"/>
  </g>
</svg>
"""

with open("docs/images/en/emulator_operation_flow.svg", "w", encoding="utf-8") as f:
    f.write(svg_emulator_flow_en.strip())


# 3. English: prerequisites_and_permissions.svg
svg_prerequisites_en = """<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 1000 640" width="100%" height="100%">
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
    .title { font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, Helvetica, Arial, sans-serif; font-size: 23px; font-weight: 700; fill: #f8fafc; }
    .subtitle { font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, Helvetica, Arial, sans-serif; font-size: 13px; fill: #94a3b8; }
    .heading { font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, Helvetica, Arial, sans-serif; font-size: 15px; font-weight: 600; fill: #f8fafc; }
    .badge { font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, Helvetica, Arial, sans-serif; font-size: 11px; font-weight: 700; }
    .desc { font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, Helvetica, Arial, sans-serif; font-size: 12px; fill: #cbd5e1; line-height: 1.5; }
    .alert-txt { font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, Helvetica, Arial, sans-serif; font-size: 11px; font-weight: 600; fill: #fbbf24; }
    .code { font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace; font-size: 11px; fill: #38bdf8; }
  </style>

  <rect width="1000" height="640" rx="16" fill="url(#bg-grad3)"/>
  <rect width="998" height="638" x="1" y="1" rx="15" fill="none" stroke="#334155" stroke-width="1.5" stroke-opacity="0.6"/>

  <g transform="translate(40, 42)">
    <text class="title">Prerequisites &amp; Permissions Checklist Guide</text>
    <text y="24" class="subtitle">Ensure required system debugging, security authorizations, and accessibility privileges are properly configured</text>
  </g>

  <!-- Pillar 1 -->
  <g transform="translate(40, 95)" filter="url(#shadow3)">
    <rect width="445" height="240" rx="12" fill="url(#box-grad3)" stroke="#f59e0b" stroke-width="1.5"/>
    <rect x="18" y="16" width="70" height="22" rx="4" fill="#f59e0b" fill-opacity="0.2"/>
    <text x="53" y="32" text-anchor="middle" class="badge" fill="#fbbf24">REQUIRED 1</text>
    <text x="98" y="33" class="heading">Android TV: Enable Network ADB</text>

    <text x="18" y="66" class="desc">● Why must ADB debugging be enabled?</text>
    <text x="32" y="85" class="desc" fill="#94a3b8">The app embeds a zero-overhead Dadb driver connecting locally</text>
    <text x="32" y="103" class="desc" fill="#94a3b8">to 127.0.0.1:5555 to inject D-pad, Home, Back, and volume keys.</text>
    
    <rect x="18" y="118" width="409" height="64" rx="6" fill="#090d16" stroke="#1e293b"/>
    <text x="28" y="138" class="desc" font-weight="600" fill="#f8fafc">Configuration Steps (System Settings):</text>
    <text x="28" y="156" class="desc">About -&gt; Click Build Number 7 times to enable Developer Options</text>
    <text x="28" y="172" class="desc">Developer Options -&gt; Turn ON "Network ADB" &amp; "USB Debugging"</text>

    <rect x="18" y="190" width="409" height="34" rx="6" fill="#172554"/>
    <text x="28" y="211" class="code">Verify from PC: adb connect &lt;TV_IP&gt;:5555</text>
  </g>

  <!-- Pillar 2 -->
  <g transform="translate(515, 95)" filter="url(#shadow3)">
    <rect width="445" height="240" rx="12" fill="url(#box-grad3)" stroke="#10b981" stroke-width="1.5"/>
    <rect x="18" y="16" width="85" height="22" rx="4" fill="#10b981" fill-opacity="0.2"/>
    <text x="60" y="32" text-anchor="middle" class="badge" fill="#34d399">CRITICAL STEP 2</text>
    <text x="110" y="33" class="heading">Accept ADB RSA Key Authorization</text>

    <text x="18" y="66" class="desc">● System security prompt appears on first connection:</text>
    <text x="32" y="85" class="alert-txt">⚠️ Check "Always allow from this computer" and click ALLOW!</text>
    <text x="32" y="103" class="desc" fill="#94a3b8">Dismissing will cause local ADB connection to fail with Unauthorized.</text>

    <rect x="18" y="118" width="409" height="106" rx="6" fill="#090d16" stroke="#374151"/>
    <text x="32" y="142" class="heading" font-size="13">Allow USB debugging? / Allow Debugging?</text>
    <text x="32" y="162" class="desc" font-size="11" fill="#94a3b8">RSA key fingerprint: SHA256:4K...AppleTVRemote</text>
    
    <rect x="32" y="174" width="16" height="16" rx="3" fill="#10b981"/>
    <path d="M 35 182 L 39 186 L 45 178" stroke="#ffffff" stroke-width="2" fill="none"/>
    <text x="56" y="187" class="desc" font-weight="600" fill="#f8fafc">Always allow from this computer</text>

    <rect x="330" y="170" width="75" height="28" rx="4" fill="#10b981"/>
    <text x="367" y="188" text-anchor="middle" class="label" fill="#ffffff">ALLOW</text>
  </g>

  <!-- Pillar 3 -->
  <g transform="translate(40, 360)" filter="url(#shadow3)">
    <rect width="445" height="240" rx="12" fill="url(#box-grad3)" stroke="#3b82f6" stroke-width="1.5"/>
    <rect x="18" y="16" width="70" height="22" rx="4" fill="#3b82f6" fill-opacity="0.2"/>
    <text x="53" y="32" text-anchor="middle" class="badge" fill="#60a5fa">REQUIRED 3</text>
    <text x="98" y="33" class="heading">Enable Accessibility Service</text>

    <text x="18" y="66" class="desc">● Global Back / Home key simulation &amp; fallback executor:</text>
    <text x="32" y="85" class="desc" fill="#94a3b8">On TVs without root or restricted ADB, accessibility provides</text>
    <text x="32" y="103" class="desc" fill="#94a3b8">system-wide GLOBAL_ACTION_BACK and node click events.</text>

    <rect x="18" y="118" width="409" height="64" rx="6" fill="#090d16" stroke="#1e293b"/>
    <text x="28" y="138" class="desc" font-weight="600" fill="#f8fafc">Setup Instructions:</text>
    <text x="28" y="156" class="desc">Click "ACCESSIBILITY SETTINGS" in the TV App dashboard</text>
    <text x="28" y="172" class="desc">Select "Apple TV Remote Receiver" -&gt; Toggle ON -&gt; Click OK</text>

    <rect x="18" y="190" width="409" height="34" rx="6" fill="#1e1b4b"/>
    <text x="28" y="211" class="code" font-size="10">Fast CLI enable: adb shell settings put secure accessibility_enabled 1</text>
  </g>

  <!-- Pillar 4 -->
  <g transform="translate(515, 360)" filter="url(#shadow3)">
    <rect width="445" height="240" rx="12" fill="url(#box-grad3)" stroke="#a855f7" stroke-width="1.5"/>
    <rect x="18" y="16" width="70" height="22" rx="4" fill="#a855f7" fill-opacity="0.2"/>
    <text x="53" y="32" text-anchor="middle" class="badge" fill="#c084fc">REQUIRED 4</text>
    <text x="98" y="33" class="heading">macOS: Code Signing &amp; Accessibility</text>

    <text x="18" y="66" class="desc">● Gatekeeper trust &amp; CGEvent mouse synthesis privileges:</text>
    <text x="32" y="85" class="desc" fill="#94a3b8">macOS blocks unnotarized apps by default. Import certificate or</text>
    <text x="32" y="103" class="desc" fill="#94a3b8">clear quarantine; grant Accessibility to move cursor and media keys.</text>

    <rect x="18" y="118" width="409" height="106" rx="6" fill="#090d16" stroke="#1e293b"/>
    <text x="28" y="136" class="code" font-size="10"># Method 1: Remove quarantine attribute (Instant)</text>
    <text x="28" y="152" class="code" font-size="10">xattr -dr com.apple.quarantine /Applications/AppleTVRemote.app</text>
    <text x="28" y="172" class="code" font-size="10"># Method 2: Import included self-signed certificate</text>
    <text x="28" y="188" class="code" font-size="10">./scripts/import_certificate.sh Corvo_Development.p12</text>
    <text x="28" y="208" class="desc" font-size="11" fill="#c084fc">Then enable AppleTVRemote in System Settings -&gt; Accessibility</text>
  </g>
</svg>
"""

with open("docs/images/en/prerequisites_and_permissions.svg", "w", encoding="utf-8") as f:
    f.write(svg_prerequisites_en.strip())

print("SVG generation done!")
