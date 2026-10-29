package com.corvofeng.fakeatv

import android.app.Notification
import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.PendingIntent
import android.app.Service
import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import android.content.IntentFilter
import android.content.pm.ServiceInfo
import android.net.ConnectivityManager
import android.net.Network
import android.net.wifi.WifiManager
import android.os.Build
import android.os.IBinder
import android.os.PowerManager
import android.util.Log
import androidx.core.app.NotificationCompat

class AtvService : Service(), AtvNative.Callback {

    companion object {
        private const val TAG = "AtvService"
        const val CHANNEL_ID = "fake_atv_service_channel"
        const val NOTIFICATION_ID = 1001

        const val ACTION_START = "com.corvofeng.fakeatv.ACTION_START"
        const val ACTION_STOP = "com.corvofeng.fakeatv.ACTION_STOP"
        const val ACTION_SET_MENU_BINDING = "com.corvofeng.fakeatv.SET_MENU_BINDING"
        const val ACTION_SET_MODE_TOGGLE = "com.corvofeng.fakeatv.SET_MODE_TOGGLE"
        const val BROADCAST_EVENT = "com.corvofeng.fakeatv.EVENT_BROADCAST"

        const val EXTRA_NAME = "extra_name"
        const val EXTRA_PIN = "extra_pin"
        const val EXTRA_BUTTON = "extra_button"
        const val EXTRA_EVENT_TYPE = "extra_event_type"
        const val EXTRA_EVENT_DETAIL = "extra_event_detail"

        @Volatile
        var isRunning = false
            private set
    }

    private var multicastLock: WifiManager.MulticastLock? = null
    private var wakeLock: PowerManager.WakeLock? = null

    private var currentName: String? = null
    private var currentPin: Int = 1111

    // Last trackpad mode reported by the native server.
    @Volatile
    private var trackpadMode = AtvNative.TRACKPAD_DIRECTION

    @Volatile
    private var isStopping = false
    private var startThread: Thread? = null
    private var networkCallback: ConnectivityManager.NetworkCallback? = null
    private var screenReceiver: BroadcastReceiver? = null

    override fun onCreate() {
        super.onCreate()
        createNotificationChannel()
        protectProcess()
        KeyDispatcher.init(this)
        initMenuBinding()
        registerNetworkCallback()
        registerScreenReceiver()
    }

    private fun initMenuBinding() {
        val binding = DeviceInfo.getMenuButtonBinding(this)
        KeyDispatcher.menuBindingButton = binding
        RemoteController.menuBindingButton = binding
        AtvAccessibilityService.instance?.menuBindingButton = binding
        Log.i(TAG, "Initialized menu button binding: $binding")
    }

    private fun applyMenuBinding(button: String) {
        val lower = button.lowercase().trim()
        DeviceInfo.setMenuButtonBinding(this, lower)
        KeyDispatcher.menuBindingButton = lower
        RemoteController.menuBindingButton = lower
        AtvAccessibilityService.instance?.menuBindingButton = lower
        Log.i(TAG, "Applied menu button binding: $lower")
        broadcast("MenuBinding", lower)
    }

    private fun applyModeToggle(button: String) {
        val lower = button.lowercase().trim()
        DeviceInfo.setModeToggleButton(this, lower)
        KeyDispatcher.modeToggleButton = lower
        Log.i(TAG, "Applied cursor mode toggle button: $lower")
        broadcast("ModeToggle", lower)
    }

    private fun protectProcess() {
        try {
            val uri = android.net.Uri.parse("content://com.dangbei.dangbeiref.data/process_protect_list_info")
            val cv = android.content.ContentValues().apply {
                put("appPackageName", packageName)
                put("appName", "FakeAtv")
                put("appVersion", 1)
            }
            contentResolver.insert(uri, cv)
            Log.i(TAG, "Registered in Dangbei process protect list")
        } catch (_: Exception) {
            // Ignore if content provider not available
        }
    }

    private fun registerNetworkCallback() {
        try {
            val cm = getSystemService(Context.CONNECTIVITY_SERVICE) as? ConnectivityManager
            if (cm != null && Build.VERSION.SDK_INT >= Build.VERSION_CODES.N) {
                val callback = object : ConnectivityManager.NetworkCallback() {
                    override fun onAvailable(network: Network) {
                        Log.i(TAG, "Network became available on Android TV")
                        if (!isRunning && !isStopping) {
                            val name = currentName ?: DeviceInfo.getDeviceName(this@AtvService)
                            startServer(name, currentPin)
                        }
                    }
                }
                cm.registerDefaultNetworkCallback(callback)
                networkCallback = callback
            }
        } catch (e: Exception) {
            Log.w(TAG, "Failed to register network callback", e)
        }
    }

    private fun registerScreenReceiver() {
        try {
            val receiver = object : BroadcastReceiver() {
                override fun onReceive(context: Context?, intent: Intent?) {
                    val action = intent?.action ?: return
                    Log.i(TAG, "Received broadcast: $action")
                    if (action == ACTION_SET_MENU_BINDING) {
                        val btn = intent.getStringExtra(EXTRA_BUTTON)
                            ?: intent.getStringExtra("button")
                            ?: "none"
                        applyMenuBinding(btn)
                        return
                    }
                    if (action == ACTION_SET_MODE_TOGGLE) {
                        applyModeToggle(intent.getStringExtra(EXTRA_BUTTON) ?: "none")
                        return
                    }
                    if (!isRunning && !isStopping) {
                        val name = currentName ?: DeviceInfo.getDeviceName(this@AtvService)
                        startServer(name, currentPin)
                    }
                }
            }
            val filter = IntentFilter().apply {
                addAction(Intent.ACTION_SCREEN_ON)
                addAction(Intent.ACTION_USER_PRESENT)
                addAction(ACTION_SET_MENU_BINDING)
                addAction(ACTION_SET_MODE_TOGGLE)
            }
            registerReceiver(receiver, filter)
            screenReceiver = receiver
        } catch (e: Exception) {
            Log.w(TAG, "Failed to register screen receiver", e)
        }
    }

    override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
        when (intent?.action) {
            ACTION_STOP -> {
                stopServer()
                stopForeground(STOP_FOREGROUND_REMOVE)
                stopSelf()
            }
            ACTION_SET_MENU_BINDING -> {
                val btn = intent.getStringExtra(EXTRA_BUTTON)
                    ?: intent.getStringExtra("button")
                    ?: "none"
                applyMenuBinding(btn)
            }
            ACTION_SET_MODE_TOGGLE -> {
                applyModeToggle(intent.getStringExtra(EXTRA_BUTTON) ?: "none")
            }
            else -> {
                val name = intent?.getStringExtra(EXTRA_NAME) ?: DeviceInfo.getDeviceName(this)
                val pin = intent?.getIntExtra(EXTRA_PIN, 1111) ?: 1111
                startServer(name, pin)
            }
        }
        return START_STICKY
    }

    @Synchronized
    private fun startServer(name: String, pin: Int) {
        currentName = name
        currentPin = pin
        isStopping = false

        if (isRunning) return

        try {
            if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.Q) {
                startForeground(
                    NOTIFICATION_ID,
                    buildNotification(name, pin),
                    ServiceInfo.FOREGROUND_SERVICE_TYPE_CONNECTED_DEVICE
                )
            } else {
                startForeground(NOTIFICATION_ID, buildNotification(name, pin))
            }
        } catch (e: Exception) {
            Log.e(TAG, "Failed to startForeground", e)
        }

        // Acquire MulticastLock to allow Bonjour mDNS discovery on Android TV Wi-Fi
        if (multicastLock == null || !multicastLock!!.isHeld) {
            val wm = applicationContext.getSystemService(Context.WIFI_SERVICE) as? WifiManager
            multicastLock = wm?.createMulticastLock("FakeAtvMulticastLock")?.apply {
                setReferenceCounted(true)
                acquire()
            }
        }

        // Acquire WakeLock to keep networking responsive (indefinite for TV)
        if (wakeLock == null || !wakeLock!!.isHeld) {
            val pm = getSystemService(Context.POWER_SERVICE) as? PowerManager
            wakeLock = pm?.newWakeLock(PowerManager.PARTIAL_WAKE_LOCK, "FakeAtv:WakeLock")?.apply {
                acquire()
            }
        }

        val deviceId = DeviceInfo.getDeviceId(this)
        val serverId = DeviceInfo.getServerIdentifier(this, deviceId)

        if (startThread?.isAlive == true) {
            return
        }

        startThread = Thread {
            var attempts = 0
            val maxAttempts = 60 // 60 attempts * 2.5s = 150 seconds (handles boot Wi-Fi delay)
            while (!isRunning && !isStopping && attempts < maxAttempts) {
                attempts++
                val localIp = DeviceInfo.getLocalIpAddress(this)
                val hasValidIp = !localIp.isNullOrBlank()

                if (hasValidIp) {
                    Log.i(TAG, "Attempt $attempts: starting native server with IP $localIp (Name: $name, PIN: $pin)")
                    val started = AtvNative.nativeStartServer(name, pin, deviceId, serverId, localIp!!, this)
                    if (started) {
                        isRunning = true
                        broadcast("ServerStatus", "running")
                        Log.i(TAG, "Native server started successfully on attempt $attempts with IP $localIp")
                        break
                    } else {
                        Log.w(TAG, "nativeStartServer returned false on attempt $attempts, will retry in 2.5s")
                    }
                } else {
                    Log.i(TAG, "Waiting for network IP before starting server (attempt $attempts/$maxAttempts)...")
                }

                try {
                    Thread.sleep(2500)
                } catch (_: InterruptedException) {
                    break
                }
            }

            if (!isRunning && !isStopping) {
                Log.e(TAG, "Failed to start native server after $attempts attempts")
                broadcast("ServerStatus", "failed")
            }
        }.apply {
            this.name = "AtvServerStartThread"
            start()
        }
    }

    @Synchronized
    private fun stopServer() {
        isStopping = true
        startThread?.interrupt()
        startThread = null

        if (!isRunning) return
        Log.i(TAG, "Stopping native server")
        AtvNative.nativeStopServer()
        isRunning = false

        multicastLock?.let {
            if (it.isHeld) it.release()
        }
        wakeLock?.let {
            if (it.isHeld) it.release()
        }
        broadcast("ServerStatus", "stopped")
    }

    override fun onButton(name: String) {
        val lowerName = name.lowercase().trim()
        if (KeyDispatcher.modeToggleButton != "none" && KeyDispatcher.modeToggleButton == lowerName) {
            // The bound button toggles the cursor instead of sending its key.
            val next = if (trackpadMode == AtvNative.TRACKPAD_MOUSE) {
                AtvNative.TRACKPAD_DIRECTION
            } else {
                AtvNative.TRACKPAD_MOUSE
            }
            AtvNative.nativeSetTrackpadMode(next)
            broadcast("Button", "$name -> cursor mode toggle")
            return
        }
        val isMapped = KeyDispatcher.menuBindingButton.isNotBlank() &&
                KeyDispatcher.menuBindingButton != "none" &&
                KeyDispatcher.menuBindingButton.equals(lowerName, ignoreCase = true)

        val display = if (isMapped) "$name -> MENU" else name
        val engine = KeyDispatcher.dispatchButton(name)
        Log.i(TAG, "onButton: $display via [$engine]")
        broadcast("Button", "$display [$engine]")
    }

    override fun onTouch(dx: Double, dy: Double, phase: Int) {
        AtvAccessibilityService.instance?.handleTouch(dx, dy, phase)
    }

    override fun onAudio(volume: Double, muted: Boolean) {
        broadcast("Audio", "vol=$volume muted=$muted")
    }

    override fun onEvent(kind: String, detail: String) {
        Log.i(TAG, "onEvent: $kind $detail")
        if (kind == "MouseClick") {
            AtvAccessibilityService.instance?.handleMouseClick()
        }
        broadcast("Event", "$kind: $detail")
    }

    override fun onTrackpadModeChanged(mode: Int) {
        trackpadMode = mode
        AtvAccessibilityService.instance?.setCursorVisible(mode == AtvNative.TRACKPAD_MOUSE)
        Log.i(TAG, "onTrackpadModeChanged: $mode")
        broadcast("TrackpadMode", when (mode) {
            AtvNative.TRACKPAD_DIRECTION -> "direction"
            AtvNative.TRACKPAD_MOUSE -> "cursor"
            else -> "idle"
        })
    }

    private fun broadcast(type: String, detail: String) {
        val intent = Intent(BROADCAST_EVENT).apply {
            putExtra(EXTRA_EVENT_TYPE, type)
            putExtra(EXTRA_EVENT_DETAIL, detail)
            setPackage(packageName)
        }
        sendBroadcast(intent)
    }

    private fun createNotificationChannel() {
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
            val channel = NotificationChannel(
                CHANNEL_ID,
                "Apple TV Remote Service",
                NotificationManager.IMPORTANCE_LOW
            ).apply {
                description = "Runs the fake Apple TV receiver on Android TV"
            }
            val manager = getSystemService(NotificationManager::class.java)
            manager?.createNotificationChannel(channel)
        }
    }

    private fun buildNotification(name: String, pin: Int): Notification {
        val pendingIntent = PendingIntent.getActivity(
            this,
            0,
            Intent(this, MainActivity::class.java),
            PendingIntent.FLAG_IMMUTABLE
        )

        return NotificationCompat.Builder(this, CHANNEL_ID)
            .setContentTitle("Apple TV Remote Receiver")
            .setContentText("Listening as \"$name\" (PIN: $pin)")
            .setSmallIcon(android.R.drawable.ic_menu_compass)
            .setContentIntent(pendingIntent)
            .setOngoing(true)
            .build()
    }

    override fun onBind(intent: Intent?): IBinder? = null

    override fun onDestroy() {
        super.onDestroy()
        stopServer()

        networkCallback?.let {
            try {
                val cm = getSystemService(Context.CONNECTIVITY_SERVICE) as? ConnectivityManager
                cm?.unregisterNetworkCallback(it)
            } catch (_: Exception) {}
            networkCallback = null
        }

        screenReceiver?.let {
            try {
                unregisterReceiver(it)
            } catch (_: Exception) {}
            screenReceiver = null
        }
    }
}
