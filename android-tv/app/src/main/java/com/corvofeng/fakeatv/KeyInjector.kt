package com.corvofeng.fakeatv

import android.os.SystemClock
import android.view.InputDevice
import android.view.InputEvent
import android.view.KeyCharacterMap
import android.view.KeyEvent

/**
 * Key injector that runs outside the app, as the ADB shell user, started by [LocalAdbDriver]:
 *
 *   CLASSPATH=<apk> app_process /system/bin com.corvofeng.fakeatv.KeyInjector
 *
 * It reads one keycode per line from stdin (a KEYCODE_* name or a number) and injects DOWN+UP
 * through InputManager.injectInputEvent, as scrcpy does. A key then costs one binder call instead
 * of `input keyevent` starting a new VM every time (about 400 ms per key on a TV box).
 *
 * This process has no app context, so nothing here may use one.
 */
object KeyInjector {
    // InputManager.INJECT_INPUT_EVENT_MODE_ASYNC: keys still land in the order they were injected.
    private const val INJECT_MODE_ASYNC = 0

    @JvmStatic
    fun main(args: Array<String>) {
        // Android 14 moved injection to InputManagerGlobal; InputManager.getInstance() there needs
        // an Application, which this process does not have.
        val inputManagerClass = try {
            Class.forName("android.hardware.input.InputManagerGlobal")
        } catch (_: ClassNotFoundException) {
            Class.forName("android.hardware.input.InputManager")
        }
        val inputManager = inputManagerClass.getDeclaredMethod("getInstance").invoke(null)
        val inject = inputManagerClass.getMethod(
            "injectInputEvent", InputEvent::class.java, Int::class.javaPrimitiveType
        )
        // LocalAdbDriver switches to this process once it sees this line.
        println("ready")
        System.out.flush()

        val reader = System.`in`.bufferedReader()
        while (true) {
            val line = reader.readLine() ?: break
            val keyCode = KeyEvent.keyCodeFromString(line.trim())
            if (keyCode == KeyEvent.KEYCODE_UNKNOWN) continue
            // Same events as `input keyevent`: a virtual keyboard key, down then up.
            val now = SystemClock.uptimeMillis()
            for (action in intArrayOf(KeyEvent.ACTION_DOWN, KeyEvent.ACTION_UP)) {
                val event = KeyEvent(
                    now, now, action, keyCode, 0, 0,
                    KeyCharacterMap.VIRTUAL_KEYBOARD, 0, 0, InputDevice.SOURCE_KEYBOARD
                )
                inject.invoke(inputManager, event, INJECT_MODE_ASYNC)
            }
        }
    }
}
