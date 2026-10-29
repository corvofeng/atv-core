package com.corvofeng.fakeatv

import android.accessibilityservice.AccessibilityService
import android.accessibilityservice.GestureDescription
import android.content.Context
import android.content.Intent
import android.graphics.Canvas
import android.graphics.Color
import android.graphics.Paint
import android.graphics.Path
import android.graphics.PixelFormat
import android.graphics.Typeface
import android.graphics.drawable.GradientDrawable
import android.os.Build
import android.os.Bundle
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

    private val mainHandler = Handler(Looper.getMainLooper())
    private var cursorView: View? = null
    private var cursorParams: WindowManager.LayoutParams? = null

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

    private val textFocusHandler = Handler(Looper.getMainLooper())

    // Last text-field state the core accepted; touched only on the main thread.
    private var reportedTextFocus = false
    private var reportedField = ""
    private var reportedText = ""

    // Guards the fields below and every write into the focused field, so the main
    // thread never sees a write without its bookkeeping.
    private val textLock = Any()

    // What handleTextOp last wrote and into which field, so the text change it causes
    // is reported as the phone's own edit rather than as a new document.
    private var writtenField: String? = null
    private var writtenText: String? = null

    // A password field exposes dots instead of its text, so the service builds new
    // values from its own copy of what it typed into the current one.
    private var passwordField: String? = null
    private var passwordText = ""

    private class TextFocusReport(
        val focused: Boolean,
        val field: String,
        val text: String,
        val fromPhone: Boolean
    )

    override fun onAccessibilityEvent(event: AccessibilityEvent?) {
        when (event?.eventType) {
            AccessibilityEvent.TYPE_WINDOWS_CHANGED,
            AccessibilityEvent.TYPE_WINDOW_STATE_CHANGED,
            AccessibilityEvent.TYPE_VIEW_FOCUSED,
            AccessibilityEvent.TYPE_VIEW_TEXT_CHANGED -> updateTextFocus(force = false)
        }
    }

    /** Report the text-field state again, e.g. to a native server that just started. */
    fun resendTextFocus() {
        textFocusHandler.post { updateTextFocus(force = true) }
    }

    /**
     * A text field counts as focused when the input-focused node is editable. On Android TV
     * input focus follows the D-pad highlight, so the phone keyboard comes up as soon as a
     * field is highlighted, even while the TV keyboard is hidden. The IME window cannot be
     * the signal: some boxes do not show it to accessibility even while it is up.
     * Changes go to the core, which tells the phone to raise, reload or drop its keyboard.
     */
    private fun updateTextFocus(force: Boolean) {
        val report = synchronized(textLock) {
            val field = findEditableFocus()
            val key = field?.let { fieldKey(it) } ?: ""
            // A password is never sent to the phone (and is not readable anyway).
            val text = if (field == null || field.isPassword) "" else fieldText(field)
            if (key != passwordField) {
                passwordField = null
                passwordText = ""
            }
            val focused = field != null
            if (!force && focused == reportedTextFocus && key == reportedField &&
                text == reportedText
            ) {
                return
            }
            val fromPhone = key == writtenField && text == writtenText
            writtenField = null
            writtenText = null
            TextFocusReport(focused, key, text, fromPhone)
        }
        val accepted = try {
            AtvNative.nativeSetTextFocus(report.focused, report.field, report.text, report.fromPhone)
        } catch (e: Throwable) {
            // UnsatisfiedLinkError when the native library failed to load.
            Log.w(TAG, "nativeSetTextFocus failed", e)
            false
        }
        Log.i(
            TAG,
            "Text field focused=${report.focused} field=${report.field} " +
                "(${report.text.length} chars, fromPhone=${report.fromPhone}) accepted=$accepted"
        )
        // Not running yet: keep the old state so the next event or resend reports again.
        if (accepted) {
            reportedTextFocus = report.focused
            reportedField = report.field
            reportedText = report.text
        }
    }

    private fun findEditableFocus(): AccessibilityNodeInfo? =
        findFocus(AccessibilityNodeInfo.FOCUS_INPUT)?.takeIf { it.isEditable }

    // AccessibilityNodeInfo.hashCode() is derived from the view's accessibility id and
    // the window id, so it names the same view across lookups. viewIdResourceName would
    // need flagReportViewIds and is null for views without an id.
    private fun fieldKey(field: AccessibilityNodeInfo): String =
        "${field.windowId}:${field.hashCode()}"

    // An empty field reports its hint as text.
    private fun fieldText(field: AccessibilityNodeInfo): String =
        if (field.isShowingHintText) "" else field.text?.toString() ?: ""

    /** Apply one edit typed on the phone's keyboard to the focused field. */
    @Synchronized
    fun handleTextOp(kind: String, text: String, count: Int) {
        val field = findEditableFocus()
        if (field == null) {
            Log.w(TAG, "Text $kind dropped: no editable field in focus")
            return
        }
        val key = fieldKey(field)
        val current = synchronized(textLock) {
            if (field.isPassword) {
                if (key != passwordField) {
                    passwordField = key
                    passwordText = ""
                }
                passwordText
            } else {
                fieldText(field)
            }
        }
        when (kind) {
            "insert" -> insertText(field, key, current, text)
            "delete" -> setFieldText(field, key, dropLastChars(current, count))
            "replace" -> setFieldText(field, key, text)
            else -> Log.w(TAG, "Unknown text op: $kind")
        }
    }

    // Return arrives as "\n": the text around it goes into the field and each
    // newline becomes an Enter press, which starts the search in leanback search bars.
    private fun insertText(field: AccessibilityNodeInfo, key: String, current: String, text: String) {
        val lines = text.split('\n')
        var buffer = current
        lines.forEachIndexed { index, line ->
            if (line.isNotEmpty()) {
                buffer += line
                setFieldText(field, key, buffer)
            }
            if (index < lines.lastIndex) {
                KeyDispatcher.dispatchButton("enter")
            }
        }
    }

    // Drops UTF-16 units without leaving half of a surrogate pair behind.
    private fun dropLastChars(text: String, count: Int): String {
        var end = (text.length - count).coerceAtLeast(0)
        if (end > 0 && Character.isHighSurrogate(text[end - 1])) end--
        return text.substring(0, end)
    }

    private fun setFieldText(field: AccessibilityNodeInfo, key: String, value: String) {
        val args = Bundle().apply {
            putCharSequence(AccessibilityNodeInfo.ACTION_ARGUMENT_SET_TEXT_CHARSEQUENCE, value)
        }
        synchronized(textLock) {
            if (!field.performAction(AccessibilityNodeInfo.ACTION_SET_TEXT, args)) {
                Log.w(TAG, "ACTION_SET_TEXT failed")
                return
            }
            writtenField = key
            if (field.isPassword) {
                passwordText = value
                writtenText = "" // passwords are reported empty
            } else {
                writtenText = value
            }
        }
    }

    override fun onInterrupt() {
        Log.w(TAG, "AtvAccessibilityService interrupted")
    }

    override fun onDestroy() {
        removePinOverlay()
        super.onDestroy()
        cursorView?.let { (getSystemService(Context.WINDOW_SERVICE) as? WindowManager)?.removeView(it) }
        cursorView = null
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
            mainHandler.post { moveCursor() }
        }
    }

    /** Shows the pointer overlay in cursor mode and hides it otherwise; callable from any thread. */
    fun setCursorVisible(visible: Boolean) {
        mainHandler.post {
            val wm = getSystemService(Context.WINDOW_SERVICE) as? WindowManager ?: return@post
            val view = cursorView
            if (visible && view == null) {
                val params = WindowManager.LayoutParams(
                    PointerView.WIDTH,
                    PointerView.HEIGHT,
                    WindowManager.LayoutParams.TYPE_ACCESSIBILITY_OVERLAY,
                    WindowManager.LayoutParams.FLAG_NOT_FOCUSABLE or
                        WindowManager.LayoutParams.FLAG_NOT_TOUCHABLE or
                        WindowManager.LayoutParams.FLAG_LAYOUT_IN_SCREEN or
                        WindowManager.LayoutParams.FLAG_LAYOUT_NO_LIMITS,
                    PixelFormat.TRANSLUCENT
                ).apply {
                    gravity = Gravity.TOP or Gravity.START
                    x = pointerX.toInt()
                    y = pointerY.toInt()
                }
                val pointer = PointerView(this)
                wm.addView(pointer, params)
                cursorView = pointer
                cursorParams = params
            } else if (!visible && view != null) {
                wm.removeView(view)
                cursorView = null
                cursorParams = null
            }
        }
    }

    // Main thread only.
    private fun moveCursor() {
        val view = cursorView ?: return
        val params = cursorParams ?: return
        params.x = pointerX.toInt()
        params.y = pointerY.toInt()
        (getSystemService(Context.WINDOW_SERVICE) as? WindowManager)?.updateViewLayout(view, params)
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

/** Arrow pointer whose tip sits at the view's top-left corner, the point a click taps. */
private class PointerView(context: Context) : View(context) {
    companion object {
        const val WIDTH = 36
        const val HEIGHT = 57
    }

    private val arrow = Path().apply {
        moveTo(1.5f, 1.5f)
        lineTo(1.5f, 49.5f)
        lineTo(13.5f, 37.5f)
        lineTo(22.5f, 55.5f)
        lineTo(28.5f, 52.5f)
        lineTo(19.5f, 34.5f)
        lineTo(34.5f, 34.5f)
        close()
    }
    private val fill = Paint(Paint.ANTI_ALIAS_FLAG).apply {
        color = Color.WHITE
        style = Paint.Style.FILL
    }
    private val outline = Paint(Paint.ANTI_ALIAS_FLAG).apply {
        color = Color.BLACK
        style = Paint.Style.STROKE
        strokeWidth = 2.5f
        strokeJoin = Paint.Join.ROUND
    }

    override fun onDraw(canvas: Canvas) {
        canvas.drawPath(arrow, fill)
        canvas.drawPath(arrow, outline)
    }
}
