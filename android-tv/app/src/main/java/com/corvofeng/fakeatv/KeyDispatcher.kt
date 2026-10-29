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

    fun dispatchButton(name: String): String {
        val lowerName = name.lowercase().trim()
        val isMappedToMenu = menuBindingButton.isNotBlank() &&
                menuBindingButton != "none" &&
                menuBindingButton.equals(lowerName, ignoreCase = true)

        var executedEngine = "none"

        when (currentMode) {
            InjectionMode.AUTO -> {
                // Tier 1: Hardware Event (/dev/input)
                if (RemoteController.isHardwareAvailable && RemoteController.sendKey(lowerName, isMappedToMenu)) {
                    executedEngine = "硬件事件驱动 (/dev/input)"
                }
                // Tier 2: Local ADB (127.0.0.1:5555)
                else if (LocalAdbDriver.sendKey(lowerName, isMappedToMenu)) {
                    executedEngine = "本地 ADB (127.0.0.1:5555)"
                }
                // Tier 3: Accessibility Service fallback
                else if (AtvAccessibilityService.instance?.sendKey(lowerName, isMappedToMenu) == true) {
                    executedEngine = "无障碍服务 (Accessibility)"
                } else {
                    executedEngine = "分发失败 (无可用通道)"
                }
            }

            InjectionMode.EVENT -> {
                if (RemoteController.sendKey(lowerName, isMappedToMenu)) {
                    executedEngine = "硬件事件驱动"
                } else {
                    executedEngine = "硬件驱动失败 (无权限或设备不存在)"
                }
            }

            InjectionMode.ADB -> {
                if (LocalAdbDriver.sendKey(lowerName, isMappedToMenu)) {
                    executedEngine = "本地 ADB"
                } else {
                    executedEngine = "本地 ADB 失败 (端口未连接或已关闭)"
                }
            }

            InjectionMode.A11Y -> {
                if (AtvAccessibilityService.instance?.sendKey(lowerName, isMappedToMenu) == true) {
                    executedEngine = "无障碍服务"
                } else {
                    executedEngine = "无障碍服务失败 (服务未开启)"
                }
            }
        }

        lastUsedEngine = executedEngine
        val display = if (isMappedToMenu) "$name -> MENU" else name
        Log.i(TAG, "dispatchButton: $display via [$executedEngine]")
        return executedEngine
    }
}
