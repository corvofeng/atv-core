package com.corvofeng.fakeatv

import android.content.ActivityNotFoundException
import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import android.content.IntentFilter
import android.os.Build
import android.os.Bundle
import android.provider.Settings
import android.view.View
import android.widget.Button
import android.widget.TextView
import android.widget.Toast
import androidx.annotation.ArrayRes
import androidx.annotation.StringRes
import androidx.appcompat.app.AlertDialog
import androidx.appcompat.app.AppCompatActivity
import androidx.core.content.ContextCompat

class MainActivity : AppCompatActivity() {

    companion object {
        // Remote buttons that can toggle the cursor mode, in the order of R.array.cursor_toggle_options.
        // Mute is not offered: the core flips its own mute state and reports it to the phone before
        // the app sees it. Siri is not offered either: the iPhone remote has no Siri button, and
        // holding the phone's side button does not reach the TV.
        private val MODE_TOGGLE_VALUES = arrayOf("power", "none")

        // Remote buttons that can act as the Android Menu key, in the order of
        // R.array.menu_binding_options and R.array.menu_binding_values.
        private val MENU_BINDING_VALUES = arrayOf("none", "play_pause", "home", "mute", "menu")

        private const val ADB_ENABLE_COMMANDS =
            "adb shell settings put secure enabled_accessibility_services com.corvofeng.fakeatv/.AtvAccessibilityService\n" +
                "adb shell settings put secure accessibility_enabled 1"

        private const val LOG_LINES = 15
    }

    private lateinit var tvStatus: TextView
    private lateinit var tvDeviceName: TextView
    private lateinit var tvIp: TextView
    private lateinit var pairSteps: View
    private lateinit var tvPairSteps: TextView
    private lateinit var pairCodeBox: View
    private lateinit var tvPairCode: TextView
    private lateinit var tvAccessibility: TextView
    private lateinit var tvAccessibilityNote: TextView
    private lateinit var tvModeToggle: TextView
    private lateinit var tvMenuBinding: TextView
    private lateinit var tvInjectionMode: TextView
    private lateinit var btnToggle: Button
    private lateinit var btnAccessibility: Button

    private val logEntries = mutableListOf<String>()

    // Content view of the technical details dialog while it is open; updateUi() keeps it live.
    private var detailsView: View? = null

    private val receiver = object : BroadcastReceiver() {
        override fun onReceive(context: Context?, intent: Intent?) {
            val type = intent?.getStringExtra(AtvService.EXTRA_EVENT_TYPE) ?: return
            val detail = intent.getStringExtra(AtvService.EXTRA_EVENT_DETAIL) ?: ""
            appendLog("[$type] $detail")
            updateUi()
        }
    }

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        setContentView(R.layout.activity_main)

        KeyDispatcher.init(this)

        tvStatus = findViewById(R.id.tvStatus)
        tvDeviceName = findViewById(R.id.tvDeviceName)
        tvIp = findViewById(R.id.tvIp)
        pairSteps = findViewById(R.id.pairSteps)
        tvPairSteps = findViewById(R.id.tvPairSteps)
        pairCodeBox = findViewById(R.id.pairCodeBox)
        tvPairCode = findViewById(R.id.tvPairCode)
        tvAccessibility = findViewById(R.id.tvAccessibility)
        tvAccessibilityNote = findViewById(R.id.tvAccessibilityNote)
        tvModeToggle = findViewById(R.id.tvModeToggle)
        tvMenuBinding = findViewById(R.id.tvMenuBinding)
        tvInjectionMode = findViewById(R.id.tvInjectionMode)
        btnToggle = findViewById(R.id.btnToggle)
        btnAccessibility = findViewById(R.id.btnAccessibility)

        findViewById<View>(R.id.rowModeToggle).setOnClickListener { showModeToggleDialog() }
        findViewById<View>(R.id.rowMenuBinding).setOnClickListener { showMenuBindingDialog() }
        findViewById<View>(R.id.rowInjectionMode).setOnClickListener { showInjectionModeDialog() }
        findViewById<View>(R.id.rowDetails).setOnClickListener { showDetailsDialog() }

        btnToggle.setOnClickListener {
            if (AtvService.isRunning) {
                val intent = Intent(this, AtvService::class.java).apply {
                    action = AtvService.ACTION_STOP
                }
                startService(intent)
            } else {
                startAtvService()
            }
            btnToggle.postDelayed({ updateUi() }, 500)
        }

        btnAccessibility.setOnClickListener {
            // Even with the hardware key driver the service is needed: it draws the cursor, clicks,
            // types text from the iPhone keyboard and shows the pairing code over other apps.
            if (AtvAccessibilityService.isServiceRunning) {
                Toast.makeText(this, R.string.toast_a11y_already_on, Toast.LENGTH_SHORT).show()
                return@setOnClickListener
            }
            try {
                val intent = Intent(Settings.ACTION_ACCESSIBILITY_SETTINGS)
                startActivity(intent)
            } catch (e: ActivityNotFoundException) {
                AlertDialog.Builder(this)
                    .setTitle(R.string.a11y_no_settings_title)
                    .setMessage(getString(R.string.a11y_no_settings_message, ADB_ENABLE_COMMANDS))
                    .setPositiveButton(R.string.btn_ok, null)
                    .show()
            }
        }

        // The first action gets the remote's focus, so the screen is usable without a pointer.
        btnAccessibility.requestFocus()

        // Auto-start service on app launch
        if (!AtvService.isRunning) {
            startAtvService()
        }
    }

    override fun onResume() {
        super.onResume()
        val filter = IntentFilter(AtvService.BROADCAST_EVENT)
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU) {
            registerReceiver(receiver, filter, Context.RECEIVER_NOT_EXPORTED)
        } else {
            registerReceiver(receiver, filter)
        }
        updateUi()
    }

    override fun onPause() {
        super.onPause()
        unregisterReceiver(receiver)
    }

    private fun startAtvService() {
        val deviceName = DeviceInfo.getDeviceName(this)
        val intent = Intent(this, AtvService::class.java).apply {
            action = AtvService.ACTION_START
            putExtra(AtvService.EXTRA_NAME, deviceName)
            putExtra(AtvService.EXTRA_PIN, 1111)
        }
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
            startForegroundService(intent)
        } else {
            startService(intent)
        }
    }

    private fun updateUi() {
        val running = AtvService.isRunning
        val deviceName = DeviceInfo.getDeviceName(this)

        // Status: server state, the name the iPhone sees, the address
        tvStatus.setText(if (running) R.string.status_running else R.string.status_stopped)
        tvStatus.setTextColor(color(if (running) R.color.status_ok else R.color.status_error))
        btnToggle.setText(if (running) R.string.btn_stop_server else R.string.btn_start_server)
        tvDeviceName.text = deviceName
        tvIp.text = getString(R.string.connect_ip, getWifiIpAddress())

        // Pairing: the code itself while the overlay cannot show it, the steps otherwise
        val code = AtvService.fallbackPairingCode
        if (code != null) {
            tvPairCode.text = code
            pairCodeBox.visibility = View.VISIBLE
            pairSteps.visibility = View.GONE
        } else {
            pairCodeBox.visibility = View.GONE
            pairSteps.visibility = View.VISIBLE
            tvPairSteps.text = if (running) {
                getString(R.string.pair_steps, deviceName)
            } else {
                getString(R.string.pair_stopped)
            }
        }

        val accessOk = AtvAccessibilityService.isServiceRunning
        tvAccessibility.setText(if (accessOk) R.string.a11y_on else R.string.a11y_off)
        tvAccessibility.setTextColor(color(if (accessOk) R.color.status_ok else R.color.status_warn))
        tvAccessibilityNote.setText(if (accessOk) R.string.a11y_on_note else R.string.a11y_off_note)

        // Settings rows show their current values
        val toggleIndex = MODE_TOGGLE_VALUES.indexOf(DeviceInfo.getModeToggleButton(this))
        tvModeToggle.text = if (toggleIndex >= 0) {
            resources.getStringArray(R.array.cursor_toggle_options)[toggleIndex]
        } else {
            DeviceInfo.getModeToggleButton(this)
        }
        tvMenuBinding.text = getMenuBindingDisplayName(DeviceInfo.getMenuButtonBinding(this))
        val modeIndex = InjectionMode.values().indexOf(KeyDispatcher.currentMode)
        tvInjectionMode.text = resources.getStringArray(R.array.injection_mode_values)[modeIndex]

        updateDetails()
    }

    private fun showDetailsDialog() {
        val view = layoutInflater.inflate(R.layout.dialog_details, null)
        detailsView = view
        AlertDialog.Builder(this)
            .setTitle(R.string.setting_details)
            .setView(view)
            .setPositiveButton(R.string.btn_close, null)
            .setOnDismissListener { if (detailsView === view) detailsView = null }
            .show()
        updateDetails()
    }

    /** Fills the technical details dialog, if it is open. */
    private fun updateDetails() {
        val view = detailsView ?: return
        view.findViewById<TextView>(R.id.tvDriverStatus).text = getString(
            R.string.details_channels,
            getString(if (RemoteController.isHardwareAvailable) R.string.channel_hw_on else R.string.channel_hw_off),
            getString(if (LocalAdbDriver.isConnected.get()) R.string.channel_adb_on else R.string.channel_adb_off),
            getString(if (AtvAccessibilityService.isServiceRunning) R.string.channel_a11y_on else R.string.channel_a11y_off)
        )
        view.findViewById<TextView>(R.id.tvLastKey).text =
            getString(R.string.details_last_key, KeyDispatcher.channelLabel(this, KeyDispatcher.lastUsedChannel))
        view.findViewById<TextView>(R.id.tvDeviceId).text =
            getString(R.string.details_device_id, DeviceInfo.getDeviceId(this))
        view.findViewById<TextView>(R.id.tvLog).text =
            if (logEntries.isEmpty()) getString(R.string.log_waiting) else logEntries.joinToString("\n")
    }

    private fun showInjectionModeDialog() {
        val modes = InjectionMode.values()
        val currentIndex = modes.indexOf(KeyDispatcher.currentMode).let { if (it >= 0) it else 0 }

        showChoiceDialog(R.string.dialog_injection_mode_title, R.array.injection_mode_options, currentIndex) { which, label ->
            KeyDispatcher.setMode(this, modes[which])
            appendLog(getString(R.string.log_config_injection, label))
            updateUi()
            Toast.makeText(this, getString(R.string.toast_switched, label), Toast.LENGTH_SHORT).show()
        }
    }

    private fun showMenuBindingDialog() {
        val current = DeviceInfo.getMenuButtonBinding(this)
        val currentIndex = MENU_BINDING_VALUES.indexOf(current).let { if (it >= 0) it else 0 }

        showChoiceDialog(R.string.dialog_menu_binding_title, R.array.menu_binding_options, currentIndex) { which, label ->
            val selected = MENU_BINDING_VALUES[which]
            DeviceInfo.setMenuButtonBinding(this, selected)
            RemoteController.menuBindingButton = selected
            AtvAccessibilityService.instance?.menuBindingButton = selected

            // Notify running AtvService
            val intent = Intent(this, AtvService::class.java).apply {
                action = AtvService.ACTION_SET_MENU_BINDING
                putExtra(AtvService.EXTRA_BUTTON, selected)
            }
            startService(intent)

            appendLog(getString(R.string.log_config_menu, label))
            updateUi()
            Toast.makeText(this, getString(R.string.toast_set, label), Toast.LENGTH_SHORT).show()
        }
    }

    private fun showModeToggleDialog() {
        val current = DeviceInfo.getModeToggleButton(this)
        val currentIndex = MODE_TOGGLE_VALUES.indexOf(current).let { if (it >= 0) it else 0 }

        showChoiceDialog(R.string.dialog_cursor_toggle_title, R.array.cursor_toggle_options, currentIndex) { which, label ->
            val selected = MODE_TOGGLE_VALUES[which]
            DeviceInfo.setModeToggleButton(this, selected)
            KeyDispatcher.modeToggleButton = selected

            // Notify running AtvService
            val intent = Intent(this, AtvService::class.java).apply {
                action = AtvService.ACTION_SET_MODE_TOGGLE
                putExtra(AtvService.EXTRA_BUTTON, selected)
            }
            startService(intent)

            appendLog(getString(R.string.log_config_cursor, label))
            updateUi()
            Toast.makeText(this, getString(R.string.toast_set, label), Toast.LENGTH_SHORT).show()
        }
    }

    /** Single-choice dialog whose focused option is clearly highlighted for the remote. */
    private fun showChoiceDialog(
        @StringRes title: Int,
        @ArrayRes options: Int,
        checked: Int,
        onPick: (which: Int, label: String) -> Unit
    ) {
        val labels = resources.getStringArray(options)
        val dialog = AlertDialog.Builder(this)
            .setTitle(title)
            .setSingleChoiceItems(labels, checked) { dialog, which ->
                onPick(which, labels[which])
                dialog.dismiss()
            }
            .setNegativeButton(R.string.btn_cancel, null)
            .show()
        dialog.listView?.selector = ContextCompat.getDrawable(this, R.drawable.bg_dialog_item)
    }

    private fun getMenuBindingDisplayName(binding: String): String {
        val key = binding.lowercase().ifEmpty { "none" }
        val index = MENU_BINDING_VALUES.indexOf(key)
        return if (index >= 0) resources.getStringArray(R.array.menu_binding_values)[index] else binding
    }

    private fun color(id: Int): Int = ContextCompat.getColor(this, id)

    private fun appendLog(line: String) {
        logEntries.add(0, line)
        if (logEntries.size > LOG_LINES) {
            logEntries.removeLast()
        }
        updateDetails()
    }

    private fun getWifiIpAddress(): String {
        return DeviceInfo.getLocalIpAddress(this) ?: getString(R.string.ip_unknown)
    }
}
