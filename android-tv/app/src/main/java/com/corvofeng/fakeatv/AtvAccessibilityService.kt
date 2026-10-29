package com.corvofeng.fakeatv

import android.accessibilityservice.AccessibilityService
import android.accessibilityservice.GestureDescription
import android.content.Context
import android.content.Intent
import android.graphics.Color
import android.graphics.Path
import android.graphics.PixelFormat
import android.graphics.Typeface
import android.graphics.drawable.GradientDrawable
import android.os.Build
import android.os.Handler
import android.os.Looper
import android.util.DisplayMetrics
import android.util.Log
import android.util.TypedValue
import android.view.Gravity
import android.view.View
import android.view.WindowManager
import android.view.accessibility.AccessibilityEvent
import android.view.accessibility.AccessibilityNodeInfo
import android.widget.LinearLayout
import android.widget.TextView

class AtvAccessibilityService : AccessibilityService() {

    companion object {
        private const val TAG = "AtvAccessibility"
        var instance: AtvAccessibilityService? = null
            private set

        val isServiceRunning: Boolean
            get() = instance != null
    }

    private var screenWidth = 1920
    private var screenHeight = 1080
    private var pointerX = 960f
    private var pointerY = 540f

    override fun onServiceConnected() {
        super.onServiceConnected()
        instance = this
        menuBindingButton = DeviceInfo.getMenuButtonBinding(this)
        Log.i(TAG, "AtvAccessibilityService connected, menuBindingButton=$menuBindingButton")

        val wm = getSystemService(Context.WINDOW_SERVICE) as? WindowManager
        if (wm != null) {
            val metrics = DisplayMetrics()
            @Suppress("DEPRECATION")
            wm.defaultDisplay.getMetrics(metrics)
            screenWidth = metrics.widthPixels
            screenHeight = metrics.heightPixels
            pointerX = screenWidth / 2f
            pointerY = screenHeight / 2f
            Log.i(TAG, "Screen resolution: ${screenWidth}x${screenHeight}")
        }

        // Auto-start AtvService if not already running
        if (!AtvService.isRunning) {
            val deviceName = DeviceInfo.getDeviceName(this)
            val serviceIntent = Intent(this, AtvService::class.java).apply {
                action = AtvService.ACTION_START
                putExtra(AtvService.EXTRA_NAME, deviceName)
                putExtra(AtvService.EXTRA_PIN, 1111)
            }
            try {
                if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
                    startForegroundService(serviceIntent)
                } else {
                    startService(serviceIntent)
                }
                Log.i(TAG, "AtvService auto-started from AtvAccessibilityService")
            } catch (e: Exception) {
                Log.e(TAG, "Failed to start AtvService from AtvAccessibilityService", e)
            }
        }
    }

    override fun onAccessibilityEvent(event: AccessibilityEvent?) {
        // No-op
    }

    override fun onInterrupt() {
        Log.w(TAG, "AtvAccessibilityService interrupted")
    }

    override fun onDestroy() {
        removePinOverlay()
        super.onDestroy()
        instance = null
        Log.i(TAG, "AtvAccessibilityService destroyed")
    }

    private val pinHandler = Handler(Looper.getMainLooper())
    private var pinOverlay: View? = null

    /**
     * Show the pairing code over whatever is on screen until [hidePairingPin].
     * [onFailure] runs on the main thread when the window cannot be added.
     */
    fun showPairingPin(pin: String, onFailure: () -> Unit) {
        pinHandler.post {
            removePinOverlay()
            val wm = getSystemService(Context.WINDOW_SERVICE) as? WindowManager
            if (wm == null) {
                onFailure()
                return@post
            }
            val density = resources.displayMetrics.density
            val padding = (32 * density).toInt()
            val overlay = LinearLayout(this).apply {
                orientation = LinearLayout.VERTICAL
                gravity = Gravity.CENTER_HORIZONTAL
                setPadding(padding * 2, padding, padding * 2, padding)
                background = GradientDrawable().apply {
                    setColor(0xE6121212.toInt())
                    cornerRadius = 24 * density
                }
                addView(TextView(context).apply {
                    text = pin
                    setTextColor(Color.WHITE)
                    setTextSize(TypedValue.COMPLEX_UNIT_SP, 96f)
                    typeface = Typeface.DEFAULT_BOLD
                    letterSpacing = 0.2f
                    gravity = Gravity.CENTER
                })
                addView(TextView(context).apply {
                    text = "Enter this code on your iPhone"
                    setTextColor(0xFFECEFF1.toInt())
                    setTextSize(TypedValue.COMPLEX_UNIT_SP, 24f)
                    gravity = Gravity.CENTER
                })
            }
            val params = WindowManager.LayoutParams(
                WindowManager.LayoutParams.WRAP_CONTENT,
                WindowManager.LayoutParams.WRAP_CONTENT,
                WindowManager.LayoutParams.TYPE_ACCESSIBILITY_OVERLAY,
                WindowManager.LayoutParams.FLAG_NOT_FOCUSABLE or
                    WindowManager.LayoutParams.FLAG_NOT_TOUCHABLE,
                PixelFormat.TRANSLUCENT
            ).apply { gravity = Gravity.CENTER }
            try {
                wm.addView(overlay, params)
                pinOverlay = overlay
                Log.i(TAG, "Pairing code shown")
            } catch (e: Exception) {
                Log.e(TAG, "Failed to show the pairing code", e)
                onFailure()
            }
        }
    }

    fun hidePairingPin() {
        pinHandler.post { removePinOverlay() }
    }

    private fun removePinOverlay() {
        val overlay = pinOverlay ?: return
        pinOverlay = null
        try {
            (getSystemService(Context.WINDOW_SERVICE) as? WindowManager)?.removeView(overlay)
            Log.i(TAG, "Pairing code hidden")
        } catch (e: Exception) {
            Log.w(TAG, "Failed to remove the pairing code overlay", e)
        }
    }

    @Volatile
    var menuBindingButton: String = "none"

    fun sendKey(name: String, isMappedToMenu: Boolean = false): Boolean {
        val lowerName = name.lowercase().trim()
        val mappedMenu = isMappedToMenu || (menuBindingButton.isNotBlank() &&
                menuBindingButton != "none" &&
                menuBindingButton.equals(lowerName, ignoreCase = true))

        if (mappedMenu) {
            Log.d(TAG, "Button $name redirected to KEYCODE_MENU")
            return execInputKeyEvent("KEYCODE_MENU") || performGlobalAction(GLOBAL_ACTION_BACK)
        }

        return when (lowerName) {
            "menu" -> performGlobalAction(GLOBAL_ACTION_BACK)
            "home" -> performGlobalAction(GLOBAL_ACTION_HOME)
            "power" -> performGlobalAction(GLOBAL_ACTION_POWER_DIALOG)
            "select" -> {
                val focused = findFocus(AccessibilityNodeInfo.FOCUS_INPUT)
                    ?: findFocus(AccessibilityNodeInfo.FOCUS_ACCESSIBILITY)
                if (focused != null && focused.performAction(AccessibilityNodeInfo.ACTION_CLICK)) {
                    Log.d(TAG, "Clicked focused node")
                    true
                } else {
                    execInputKeyEvent("KEYCODE_DPAD_CENTER")
                }
            }
            "up" -> navigateDpad(AccessibilityNodeInfo.FOCUS_INPUT, "KEYCODE_DPAD_UP", 0, -200)
            "down" -> navigateDpad(AccessibilityNodeInfo.FOCUS_INPUT, "KEYCODE_DPAD_DOWN", 0, 200)
            "left" -> navigateDpad(AccessibilityNodeInfo.FOCUS_INPUT, "KEYCODE_DPAD_LEFT", -200, 0)
            "right" -> navigateDpad(AccessibilityNodeInfo.FOCUS_INPUT, "KEYCODE_DPAD_RIGHT", 200, 0)
            "play_pause" -> execInputKeyEvent("KEYCODE_MEDIA_PLAY_PAUSE")
            "volume_up" -> execInputKeyEvent("KEYCODE_VOLUME_UP")
            "volume_down" -> execInputKeyEvent("KEYCODE_VOLUME_DOWN")
            "mute" -> execInputKeyEvent("KEYCODE_VOLUME_MUTE")
            "siri" -> execInputKeyEvent("KEYCODE_SEARCH")
            else -> execInputKeyEvent("KEYCODE_${lowerName.uppercase()}")
        }
    }

    fun handleButton(name: String) {
        sendKey(name)
    }

    private fun navigateDpad(focusType: Int, fallbackKeycode: String, swipeDx: Int, swipeDy: Int): Boolean {
        val focused = findFocus(focusType)
        val direction = when (fallbackKeycode) {
            "KEYCODE_DPAD_UP" -> android.view.View.FOCUS_UP
            "KEYCODE_DPAD_DOWN" -> android.view.View.FOCUS_DOWN
            "KEYCODE_DPAD_LEFT" -> android.view.View.FOCUS_LEFT
            "KEYCODE_DPAD_RIGHT" -> android.view.View.FOCUS_RIGHT
            else -> 0
        }

        if (focused != null && direction != 0) {
            val next = focused.focusSearch(direction)
            if (next != null && next.performAction(AccessibilityNodeInfo.ACTION_FOCUS)) {
                return true
            }
        }

        // Fallback: try executing input keyevent if available
        if (execInputKeyEvent(fallbackKeycode)) {
            return true
        }

        // Alternatively perform swipe gesture
        val cx = screenWidth / 2f
        val cy = screenHeight / 2f
        swipe(cx, cy, cx + swipeDx, cy + swipeDy, 100)
        return true
    }

    private fun execInputKeyEvent(keycode: String): Boolean {
        return try {
            val process = Runtime.getRuntime().exec(arrayOf("input", "keyevent", keycode))
            process.waitFor() == 0
        } catch (e: Exception) {
            false
        }
    }

    fun handleTouch(dx: Double, dy: Double, phase: Int) {
        if (phase == 2) { // Moved
            val sensitivity = 1.5f
            pointerX = (pointerX + dx.toFloat() * sensitivity).coerceIn(0f, screenWidth.toFloat())
            pointerY = (pointerY + dy.toFloat() * sensitivity).coerceIn(0f, screenHeight.toFloat())
        }
    }

    fun handleMouseClick() {
        tap(pointerX, pointerY)
    }

    private fun tap(x: Float, y: Float) {
        val path = Path().apply {
            moveTo(x, y)
        }
        val stroke = GestureDescription.StrokeDescription(path, 0, 50)
        val gesture = GestureDescription.Builder().addStroke(stroke).build()
        dispatchGesture(gesture, null, null)
    }

    private fun swipe(startX: Float, startY: Float, endX: Float, endY: Float, durationMs: Long) {
        val path = Path().apply {
            moveTo(startX, startY)
            lineTo(endX, endY)
        }
        val stroke = GestureDescription.StrokeDescription(path, 0, durationMs)
        val gesture = GestureDescription.Builder().addStroke(stroke).build()
        dispatchGesture(gesture, null, null)
    }
}
