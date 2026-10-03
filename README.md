# Fake Apple TV (atv-core) 📺

> **English Version** | [中文版本](README_zh.md)

**Turn the Apple TV Remote that's already built into your iPhone into a controller for Android TV and Mac.** Pull down Control Center, pick your device, and start navigating — no jailbreak, no third-party app, no extra hardware.

<!-- TODO: replace the image below with a 15–30s screen recording:
     Control Center → Apple TV Remote → select device → navigate the TV / Mac.
     Save it as docs/images/en/demo.gif and swap the <img> src. -->

### iPhone: Find the Remote and Start Using

Swipe down from the top-right corner to open Control Center, tap "Apple TV Remote", select the device running the receiver, pair it, and you're ready to swipe and click.

| **Find the Remote** | **Use the Remote** |
| :---: | :---: |
| <img src="docs/images/en/ios_tv_control.jpg" width="220" alt="Apple TV Remote in Control Center" /> | <img src="docs/images/en/ios_apple_tv_remote.png" width="220" alt="Native Apple TV Remote Interface" /> |
| **Swipe down from top-right** to open Control Center, tap the circled remote icon<br>*(If missing, go to **Settings → Control Center** to add it)* | Touchpad on top for swipes / cursor navigation<br>Buttons below for Select, Back, Play/Pause, Volume, etc. |

📖 Full walkthrough & demo: [Blog Post](https://corvo.myseu.cn/2026/09/27/2026-09-27-%E7%94%A8iPhone%E9%81%A5%E6%8E%A7%E5%99%A8%E6%8E%A7%E5%88%B6%E4%BD%A0%E7%9A%84Android-TV%E5%92%8CMac/)

---

## Download

Prebuilt binaries are attached to the [latest release](https://github.com/corvofeng/atv-core/releases/latest):

| Platform | Download |
| :--- | :--- |
| macOS (Apple Silicon) | [`AppleTVRemote-arm64.dmg`](https://github.com/corvofeng/atv-core/releases/latest/download/AppleTVRemote-arm64.dmg) |
| macOS (Intel) | [`AppleTVRemote-x86_64.dmg`](https://github.com/corvofeng/atv-core/releases/latest/download/AppleTVRemote-x86_64.dmg) |
| Android TV / Google TV | [`FakeAtv-release.apk`](https://github.com/corvofeng/atv-core/releases/latest/download/FakeAtv-release.apk) |

Prefer to build it yourself? See [Build from source](#build-from-source).

---

## Compatibility

| | Supported | Notes |
| :--- | :--- | :--- |
| **Controller** | iPhone / iPad with the native **Apple TV Remote** in Control Center | No app install needed on the phone |
| **macOS targets** | Apple Silicon & Intel Macs (menu bar app + CLI) | Requires Accessibility permission |
| **Android targets** | Android TV, Google TV, TV boxes, emulators | Requires network ADB; see [limitations](#limitations) |
| **Network** | Controller and target on the same LAN | AP isolation / guest Wi-Fi must be off |

**Tested on** <!-- TODO: fill in your actual test matrix, e.g. iOS 17.x / macOS 14 (Apple Silicon) / Sony Bravia Google TV, Xiaomi Box, emulator API 30 -->:

- iOS / iPadOS: _to be listed_
- macOS: _to be listed_
- Android TV devices: _to be listed_

### Limitations

- **Android TV needs network ADB.** Android blocks ordinary apps from injecting D-pad / Select keys into other apps, so key injection relies on a local ADB channel. The Accessibility Service is only a fallback and handles global actions (Back / Home). Full setup: [Android TV prerequisites](#android-tv).
- **Pairing uses a fixed PIN.** When the iPhone asks for a pairing code, enter **`1111`**.
- The macOS app is signed with a self-signed certificate, so Gatekeeper will warn on first launch. See [macOS prerequisites](#macos).

---

## Quick start

1. **Install** the DMG (Mac) or APK (Android TV) from [Download](#download).
2. **Put your iPhone and the target on the same Wi-Fi** (AP isolation off).
3. **Grant permissions** — Accessibility on macOS; network ADB + the on-screen "Always allow" prompt on Android TV. Details below.
4. **Open Control Center to find the Remote**:
   - Swipe down from the top-right corner of your screen to open **Control Center**, then tap the circled **Apple TV Remote** icon.
   - *(If the icon is not in Control Center, go to iOS **Settings → Control Center** and add "Apple TV Remote" from "More Controls")*.
   - Select your target device from the top dropdown, and enter PIN **`1111`** when prompted.

### macOS

After installing, macOS Gatekeeper will flag the self-signed app. Clear it once:

```bash
# Option A (recommended): trust the bundled certificate
./scripts/import_certificate.sh Corvo_Development.p12

# Option B: strip the quarantine attribute
xattr -dr com.apple.quarantine /Applications/AppleTVRemote.app
```

Then grant **System Settings → Privacy & Security → Accessibility → AppleTVRemote** (or your terminal if running the CLI). Launch the app and use the menu bar icon.

### Android TV

1. **Enable developer options**: Settings → About → tap **Build Number** 7 times.
2. **Enable USB Debugging and Network Debugging**: Settings → System → Developer Options.
3. Install and launch the APK. On first run, an Android security dialog appears — with the TV remote, check **"Always allow from this computer"** and tap **Allow**. (Rejecting it causes an `Unauthorized` error and no input.)
4. *(Optional fallback)* Enable the Accessibility Service: main dashboard → **ACCESSIBILITY SETTINGS** → find **Apple TV Remote Receiver** → turn it On.

> **Why ADB?** Android prevents regular apps from injecting global D-pad / Select keys into third-party streaming apps. The bundled `dadb` driver connects to `127.0.0.1:5555` and dispatches Linux keycodes directly (measured <8 ms over local loopback). The Accessibility Service can't do this on its own — it only serves as a fallback and for global Back / Home actions.

---

## Key & gesture mapping

| Apple TV Remote action | Android TV | macOS |
| :--- | :--- | :--- |
| **Swipe (D-pad mode)** | `KEYCODE_DPAD_UP/DOWN/LEFT/RIGHT` | Arrow keys `↑ ↓ ← →` |
| **Swipe (cursor mode)** | Touch drag / cursor | Smooth cursor via `CGEvent` |
| **Tap / SELECT** | `KEYCODE_DPAD_CENTER` | Enter / left click |
| **Back (`<`)** | `GLOBAL_ACTION_BACK` | `Escape` |
| **TV / Home** | `GLOBAL_ACTION_HOME` | Desktop / configurable |
| **Single click ⏯** | `KEYCODE_MEDIA_PLAY_PAUSE` | Play / Pause (`NX_KEYTYPE_PLAY`) |
| **Double click ⏯** | Next / fast-forward | Next track (⏭) |
| **Triple click ⏯** | Previous / rewind | Previous track (⏮) |
| **Volume +/-** | `KEYCODE_VOLUME_UP/DOWN` | Master volume + native HUD |
| **Mute** | `KEYCODE_VOLUME_MUTE` | System mute |
| **Side Siri key** | Voice search (`KEYCODE_SEARCH`) | Toggle cursor ⇄ D-pad / Siri |
| **Power** | `GLOBAL_ACTION_POWER_DIALOG` | Display sleep / wake |

---

## Build from source

```bash
# macOS — build the standalone DMG
./scripts/build_dmg.sh
# → build/AppleTVRemote-arm64.dmg ; drag to /Applications

# Android TV — build and install the APK
./scripts/build_android.sh
adb install -r android-tv/app/build/outputs/apk/debug/app-debug.apk
adb shell am start -n com.corvofeng.fakeatv/.MainActivity
```

---

<details>
<summary><h2 style="display:inline">Developer docs & internals</h2></summary>

### Android emulator bridge

For testing in an Android Studio Android TV AVD (physical devices can't discover the emulator's isolated NAT subnet, `10.0.2.15`):

```bash
# 1. Launch an Android TV AVD (API 30+), verify with: adb devices
# 2. Install the receiver
./scripts/build_android.sh
adb -s emulator-5554 install -r android-tv/app/build/outputs/apk/debug/app-debug.apk
adb -s emulator-5554 shell am start -n com.corvofeng.fakeatv/.MainActivity

# 3. Start the bridge daemon
python3 scripts/bridge_emulator.py start        # or: start -f (foreground, live logs)
python3 scripts/bridge_emulator.py status       # port-forward & mDNS state
python3 scripts/bridge_emulator.py logs -f      # tail logs
python3 scripts/bridge_emulator.py stop         # stop daemon
```

Then pair from iPhone → Control Center → Apple TV Remote → **`Android TV Emulator`** → PIN `1111`.

The bridge forwards ports `49152/49153/49154` via `adb forward` and proxies the Bonjour records (`_mediaremotetv._tcp`, `_companion-link._tcp`) onto the physical Wi-Fi, so the iPhone connects to the host and is transparently routed to the emulator.

![Emulator bridge workflow](docs/images/en/emulator_operation_flow.svg)

### macOS Web Inspector (port 8765)

With the daemon or menu bar app running, open **`http://127.0.0.1:8765`**:

![macOS Web Inspector](docs/images/en/mac_browser_inspector.png)

- Live touch-vector canvas: finger coordinates, gesture phases (`Began`/`Moved`/`Ended`), velocity vectors, direction detection.
- On-page virtual remote to trigger Mac/TV responses without holding the phone.
- Ballistics presets: `0.5x Precise`, `1.0x Standard`, `1.5x Fast`, `2.2x Ultra-Wide`.
- Connected-device metadata and live volume telemetry.

### macOS host diagnostics (port 8766)

Open **`http://127.0.0.1:8766`** to inspect process health, port bindings, and live protocol handshakes:

![macOS host diagnostics](docs/images/en/mac_debug_web_page.png)

- Session tracing: Companion client connections, SRP auth, MRP encrypted channel init.
- Process telemetry: Core API (`8765`), Web debug (`8766`), MediaRemote (`49152`), background PID, log filter, restart controls.

### Touchpad gestures & focus accumulation

The Companion Link protocol streams high-frequency delta coordinates. `atv-core` turns them into grid focus shifts (Android TV) or accelerated cursor motion (macOS):

![Remote swipe & navigation flow](docs/images/en/page_navigation_movement.svg)

1. **Delta accumulator** — aggregates micro-displacements to suppress jitter and accidental touches.
2. **Direction deadzone** — resolves horizontal vs. vertical intent and filters diagonal noise.
3. **Platform dispatch** — Android TV emits `KEYCODE_DPAD_*`; macOS computes velocity-based ballistics and emits `CGEvent` motion.

### Driver dispatch hierarchy

![Driver hierarchy & permissions](docs/images/en/prerequisites_and_permissions.svg)

- **Android** — Primary: `dadb` loopback to `127.0.0.1:5555` for direct keycode dispatch. Fallback: Accessibility Service for global window actions.
- **macOS** — Accessibility API (`CGEvent`) for cursor/keystrokes; CoreAudio / MediaRemote for hardware volume and media HUD.

### Android TV dashboard & configuration

| Main dashboard | Input injection mode | Menu key binding |
| :---: | :---: | :---: |
| ![Main dashboard](docs/images/en/android_tv_main_screen_ready.png) | ![Injection mode](docs/images/en/android_tv_injection_mode_dialog.png) | ![Menu key binding](docs/images/en/android_tv_menu_binding_dialog.png) |

- **Driver health** for `/dev/input/event*`, local `dadb` (`127.0.0.1:5555`), and `AtvAccessibilityService`.
- **Injection strategy**: *Local ADB Only*, *Accessibility Only*, *ADB Preferred (Accessibility fallback)*, or *Hardware Preferred*.
- **Menu key rebinding**: remap Play/Pause, Home, or Mute to `KEYCODE_MENU` for legacy TV apps.

**Accessibility setup screens:**

| Service list | Confirmation dialog | Ready |
| :---: | :---: | :---: |
| ![Service list](docs/images/en/android_tv_accessibility_service_list.png) | ![Permission dialog](docs/images/en/android_tv_permission_dialog.png) | ![Ready](docs/images/en/android_tv_main_screen_ready.png) |

</details>

---

## Troubleshooting & FAQ

**Device doesn't appear in Control Center?**
1. Confirm iPhone and target are on the same Wi-Fi subnet (disable AP isolation / guest mode).
2. On macOS, run `dns-sd -B _mediaremotetv._tcp` to verify Bonjour advertisement.
3. For the emulator, check the bridge: `python3 scripts/bridge_emulator.py status`.

**Keys don't respond on TV / ADB connection error?**
1. Verify "Network ADB" and "USB Debugging" are on in Developer Options.
2. Relaunch the app and watch for the RSA fingerprint dialog — check **"Always allow"** and tap **Allow**.
3. If no prompt appears, run `adb connect <TV_IP>:5555` from a computer to trigger initial trust.

**Can't find Accessibility settings on Android TV?**
Some TV skins hide the menu. Use the "ACCESSIBILITY SETTINGS" dialog in the app, or run:
```bash
adb shell settings put secure enabled_accessibility_services com.corvofeng.fakeatv/.AtvAccessibilityService
adb shell settings put secure accessibility_enabled 1
```

**macOS says the app is damaged / from an unidentified developer?**
```bash
xattr -dr com.apple.quarantine /Applications/AppleTVRemote.app
# or trust the bundled certificate:
./scripts/import_certificate.sh Corvo_Development.p12
```

---

## License

MIT. Intended for educational and local device interoperability research.
