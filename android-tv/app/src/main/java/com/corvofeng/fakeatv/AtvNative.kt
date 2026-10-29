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
        fun onShowPin(pin: String)
        fun onHidePin()
    }

    external fun nativeStartServer(
        name: String,
        pin: Int,
        deviceId: String,
        serverId: String,
        localIp: String,
        filesDir: String,
        callback: Callback
    ): Boolean

    external fun nativeStopServer()
    external fun nativeIsRunning(): Boolean
}
