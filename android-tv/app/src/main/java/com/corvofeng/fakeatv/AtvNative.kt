package com.corvofeng.fakeatv

object AtvNative {
    init {
        try {
            System.loadLibrary("atv_android")
        } catch (e: UnsatisfiedLinkError) {
            e.printStackTrace()
        }
    }

    interface Callback {
        fun onButton(name: String)
        fun onTouch(dx: Double, dy: Double, phase: Int)
        fun onAudio(volume: Double, muted: Boolean)
        fun onEvent(kind: String, detail: String)

        /**
         * The phone's keyboard edited the focused text field.
         * kind: "insert" (text, Return is "\n"), "delete" (count chars before the cursor)
         * or "replace" (text is the whole new field content).
         */
        fun onText(kind: String, text: String, count: Int)
    }

    external fun nativeStartServer(
        name: String,
        pin: Int,
        deviceId: String,
        serverId: String,
        localIp: String,
        callback: Callback
    ): Boolean

    external fun nativeStopServer()
    external fun nativeIsRunning(): Boolean

    /**
     * Report whether an editable field is focused on the TV, which one (field is a stable key)
     * and its current text. fromPhone marks text that only changed because edits from the
     * phone were applied. Returns false when the server is not running.
     */
    external fun nativeSetTextFocus(
        focused: Boolean,
        field: String,
        text: String,
        fromPhone: Boolean
    ): Boolean
}
