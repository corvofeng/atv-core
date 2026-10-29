package com.corvofeng.fakeatv

import android.content.Context
import android.util.Log

enum class InjectionMode(val key: String, val displayName: String) {
    AUTO("auto", "自动自适应 (硬件 -> ADB -> 无障碍)"),
    EVENT("event", "仅硬件事件 (/dev/input)"),
    ADB("adb", "仅本地 ADB (127.0.0.1:5555)"),
    A11Y("a11y", "仅无障碍服务 (Accessibility)");

    companion object {
        fun fromKey(key: String): InjectionMode {
            return values().find { it.key.equals(key, ignoreCase = true) } ?: AUTO
        }
    }
}

/**
 * Outcome of one key: [engine] is the dispatcher's own log text, [channel] the same outcome as a
 * key that [KeyDispatcher.channelLabel] names in the UI language.
 */
data class DispatchResult(val engine: String, val channel: String)

object KeyDispatcher {
    private const val TAG = "KeyDispatcher"

    @Volatile
    var currentMode: InjectionMode = InjectionMode.AUTO

    @Volatile
    var menuBindingButton: String = "none"

    // Remote button that toggles the cursor mode instead of sending its key.
    @Volatile
    var modeToggleButton: String = "power"

    @Volatile
    var lastUsedEngine: String = "未执行"
        private set

    // Same outcome as lastUsedEngine, as a key the UI names in its own language:
    // "event", "adb", "a11y", "event_failed", "adb_failed", "a11y_failed", "failed",
    // or "" before the first key.
    @Volatile
    var lastUsedChannel: String = ""
        private set

    /** Names a channel key from [DispatchResult.channel] in the current UI language. */
    fun channelLabel(context: Context, channel: String): String = context.getString(
        when (channel) {
            "event" -> R.string.engine_event
            "adb" -> R.string.engine_adb
            "a11y" -> R.string.engine_a11y
            "event_failed" -> R.string.engine_event_failed
            "adb_failed" -> R.string.engine_adb_failed
            "a11y_failed" -> R.string.engine_a11y_failed
            "failed" -> R.string.engine_failed
            else -> R.string.engine_none
        }
    )

    fun init(context: Context) {
        val savedModeKey = DeviceInfo.getInjectionMode(context)
        currentMode = InjectionMode.fromKey(savedModeKey)
        menuBindingButton = DeviceInfo.getMenuButtonBinding(context)
        modeToggleButton = DeviceInfo.getModeToggleButton(context)
        Log.i(TAG, "KeyDispatcher initialized: mode=$currentMode, menuBinding=$menuBindingButton")

        // Pre-warm local ADB connection in background if mode is AUTO or ADB
        if (currentMode == InjectionMode.AUTO || currentMode == InjectionMode.ADB) {
            LocalAdbDriver.init(context)
        }
    }

    fun setMode(context: Context, mode: InjectionMode) {
        currentMode = mode
        DeviceInfo.setInjectionMode(context, mode.key)
        Log.i(TAG, "InjectionMode updated to: $mode")
        if (mode == InjectionMode.AUTO || mode == InjectionMode.ADB) {
            LocalAdbDriver.init(context)
        }
    }

    fun dispatchButton(name: String): DispatchResult {
        val lowerName = name.lowercase().trim()
        val isMappedToMenu = menuBindingButton.isNotBlank() &&
                menuBindingButton != "none" &&
                menuBindingButton.equals(lowerName, ignoreCase = true)

        var executedEngine = "none"
        var channel = ""

        when (currentMode) {
            InjectionMode.AUTO -> {
                // Tier 1: Hardware Event (/dev/input)
                if (RemoteController.isHardwareAvailable && RemoteController.sendKey(lowerName, isMappedToMenu)) {
                    executedEngine = "硬件事件驱动 (/dev/input)"
                    channel = "event"
                }
                // Tier 2: Local ADB (127.0.0.1:5555)
                else if (LocalAdbDriver.sendKey(lowerName, isMappedToMenu)) {
                    executedEngine = "本地 ADB (127.0.0.1:5555)"
                    channel = "adb"
                }
                // Tier 3: Accessibility Service fallback
                else if (AtvAccessibilityService.instance?.sendKey(lowerName, isMappedToMenu) == true) {
                    executedEngine = "无障碍服务 (Accessibility)"
                    channel = "a11y"
                } else {
                    executedEngine = "分发失败 (无可用通道)"
                    channel = "failed"
                }
            }

            InjectionMode.EVENT -> {
                if (RemoteController.sendKey(lowerName, isMappedToMenu)) {
                    executedEngine = "硬件事件驱动"
                    channel = "event"
                } else {
                    executedEngine = "硬件驱动失败 (无权限或设备不存在)"
                    channel = "event_failed"
                }
            }

            InjectionMode.ADB -> {
                if (LocalAdbDriver.sendKey(lowerName, isMappedToMenu)) {
                    executedEngine = "本地 ADB"
                    channel = "adb"
                } else {
                    executedEngine = "本地 ADB 失败 (端口未连接或已关闭)"
                    channel = "adb_failed"
                }
            }

            InjectionMode.A11Y -> {
                if (AtvAccessibilityService.instance?.sendKey(lowerName, isMappedToMenu) == true) {
                    executedEngine = "无障碍服务"
                    channel = "a11y"
                } else {
                    executedEngine = "无障碍服务失败 (服务未开启)"
                    channel = "a11y_failed"
                }
            }
        }

        lastUsedEngine = executedEngine
        lastUsedChannel = channel
        val display = if (isMappedToMenu) "$name -> MENU" else name
        Log.i(TAG, "dispatchButton: $display via [$executedEngine]")
        return DispatchResult(executedEngine, channel)
    }
}
