package com.corvofeng.fakeatv

import android.content.Context
import android.util.Log
import dadb.AdbKeyPair
import dadb.AdbShellPacket
import dadb.AdbShellStream
import dadb.Dadb
import java.io.File
import java.util.concurrent.Executors
import java.util.concurrent.atomic.AtomicBoolean

object LocalAdbDriver {
    private const val TAG = "LocalAdbDriver"
    private const val ADB_HOST = "127.0.0.1"
    private const val ADB_PORT = 5555

    // Runs KeyInjector from the installed APK as the shell user.
    private const val INJECTOR_COMMAND =
        "CLASSPATH=\$(pm path com.corvofeng.fakeatv | cut -d: -f2) " +
            "exec app_process /system/bin com.corvofeng.fakeatv.KeyInjector"
    private const val DRAIN_MARKER = "fakeatv-shell-drained"

    private val executor = Executors.newSingleThreadExecutor()
    private var appContext: Context? = null
    private var dadbInstance: Dadb? = null
    private var shellStream: AdbShellStream? = null
    private var injectorStream: AdbShellStream? = null

    // True once KeyInjector is running and the shell has no `input keyevent` left to run.
    @Volatile
    private var injectorUp = false

    val isConnected = AtomicBoolean(false)

    fun init(context: Context) {
        appContext = context.applicationContext
        initAsync()
    }

    fun initAsync(callback: ((Boolean) -> Unit)? = null) {
        executor.execute {
            val ok = connectInternal()
            callback?.invoke(ok)
        }
    }

    @Synchronized
    fun connectInternal(): Boolean {
        if (isConnected.get() && shellStream != null) {
            return true
        }
        closeInternal()
        try {
            Log.i(TAG, "Attempting to connect to local ADB at $ADB_HOST:$ADB_PORT...")
            val context = appContext
            val d = if (context != null) {
                val keyDir = File(context.filesDir, "adb")
                keyDir.mkdirs()
                val privFile = File(keyDir, "adbkey")
                val pubFile = File(keyDir, "adbkey.pub")
                if (!privFile.exists() || !pubFile.exists()) {
                    Log.i(TAG, "Generating internal ADB keypair at $keyDir...")
                    AdbKeyPair.generate(privFile, pubFile)
                }
                val keyPair = AdbKeyPair.read(privFile, pubFile)
                Dadb.create(ADB_HOST, ADB_PORT, keyPair)
            } else {
                Dadb.create(ADB_HOST, ADB_PORT)
            }

            val stream = d.openShell()
            dadbInstance = d
            shellStream = stream
            isConnected.set(true)
            Log.i(TAG, "Successfully connected to local ADB daemon!")
            startInjector(d)
            return true
        } catch (e: Exception) {
            Log.w(TAG, "Failed to connect to local ADB ($ADB_HOST:$ADB_PORT): ${e.message}")
            closeInternal()
            return false
        }
    }

    @Synchronized
    fun closeInternal() {
        try { injectorStream?.close() } catch (_: Exception) {}
        try { shellStream?.close() } catch (_: Exception) {}
        try { dadbInstance?.close() } catch (_: Exception) {}
        injectorUp = false
        injectorStream = null
        shellStream = null
        dadbInstance = null
        isConnected.set(false)
    }

    /** Starts KeyInjector over the ADB connection; keys go through `input keyevent` until it is up. */
    private fun startInjector(d: Dadb) {
        val stream = try {
            d.openShell(INJECTOR_COMMAND)
        } catch (e: Exception) {
            Log.w(TAG, "Failed to start key injector: ${e.message}")
            return
        }
        injectorStream = stream
        Thread({ readInjector(stream) }, "KeyInjectorReader").start()
    }

    private fun readInjector(stream: AdbShellStream) {
        try {
            while (true) {
                when (val packet = stream.read()) {
                    is AdbShellPacket.StdOut ->
                        if (String(packet.payload).contains("ready")) switchToInjector(stream)
                    is AdbShellPacket.StdError ->
                        Log.w(TAG, "Key injector: ${String(packet.payload).trim()}")
                    is AdbShellPacket.Exit -> break
                }
            }
        } catch (_: Exception) {}
        synchronized(this) {
            if (injectorStream === stream) injectorUp = false
        }
        Log.w(TAG, "Key injector stopped, keys go through input keyevent")
    }

    /**
     * Sends keys to the injector from now on. The `input keyevent` commands still queued in the
     * shell run first, so a fast injected key never lands ahead of a slow key sent before it.
     */
    private fun switchToInjector(stream: AdbShellStream) {
        synchronized(this) {
            if (injectorStream !== stream) return
            val shell = shellStream ?: return
            try {
                shell.write("echo $DRAIN_MARKER\n")
                while (true) {
                    val packet = shell.read()
                    if (packet is AdbShellPacket.Exit) return
                    if (packet is AdbShellPacket.StdOut && String(packet.payload).contains(DRAIN_MARKER)) break
                }
            } catch (e: Exception) {
                Log.w(TAG, "Failed to drain the ADB shell: ${e.message}")
                return
            }
            injectorUp = true
            Log.i(TAG, "Key injector is up")
        }
    }

    private val KEY_MAP = mapOf(
        "up" to "KEYCODE_DPAD_UP",
        "down" to "KEYCODE_DPAD_DOWN",
        "left" to "KEYCODE_DPAD_LEFT",
        "right" to "KEYCODE_DPAD_RIGHT",
        "select" to "KEYCODE_DPAD_CENTER",
        "menu" to "KEYCODE_BACK",
        "home" to "KEYCODE_HOME",
        "play_pause" to "KEYCODE_MEDIA_PLAY_PAUSE",
        "volume_up" to "KEYCODE_VOLUME_UP",
        "volume_down" to "KEYCODE_VOLUME_DOWN",
        "mute" to "KEYCODE_VOLUME_MUTE",
        "power" to "KEYCODE_POWER",
        "siri" to "KEYCODE_SEARCH"
    )

    fun sendKey(buttonName: String, isMappedToMenu: Boolean = false): Boolean {
        val keycode = if (isMappedToMenu) {
            "KEYCODE_MENU"
        } else {
            KEY_MAP[buttonName.lowercase()] ?: "KEYCODE_${buttonName.uppercase()}"
        }

        if (!isConnected.get()) {
            if (!connectInternal()) {
                return false
            }
        }

        return synchronized(this) {
            val injector = injectorStream
            if (injectorUp && injector != null) {
                try {
                    injector.write("$keycode\n")
                    Log.d(TAG, "Injector sent key: $keycode")
                    return true
                } catch (e: Exception) {
                    Log.w(TAG, "Error writing to key injector: ${e.message}, falling back to input keyevent")
                    injectorUp = false
                }
            }
            try {
                val stream = shellStream ?: return false
                val cmd = "input keyevent $keycode\n"
                stream.write(cmd)
                Log.d(TAG, "ADB sent keyevent: $keycode")
                true
            } catch (e: Exception) {
                Log.w(TAG, "Error writing to ADB shell stream: ${e.message}, will retry reconnection")
                closeInternal()
                if (connectInternal()) {
                    try {
                        shellStream?.write("input keyevent $keycode\n")
                        return true
                    } catch (_: Exception) {}
                }
                false
            }
        }
    }
}
