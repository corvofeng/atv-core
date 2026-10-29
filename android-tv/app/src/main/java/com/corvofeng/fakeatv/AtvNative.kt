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
        fun onTrackpadModeChanged(mode: Int)
    }

    // Trackpad modes as numbered by the core's TrackpadMode.
    const val TRACKPAD_DIRECTION = 0
    const val TRACKPAD_MOUSE = 1

    external fun nativeStartServer(
        name: String,
        pin: Int,
        deviceId: String,
        serverId: String,
        localIp: String,
        callback: Callback
    ): Boolean

    external fun nativeStopServer()
    external fun nativeSetTrackpadMode(mode: Int): Boolean
    external fun nativeIsRunning(): Boolean
}
