package com.corvofeng.fakeatv

import android.content.Context
import android.content.SharedPreferences
import android.net.ConnectivityManager
import android.os.Build
import android.provider.Settings
import java.net.Inet4Address
import java.net.NetworkInterface
import java.util.UUID

object DeviceInfo {

    const val PREF_MENU_BUTTON_BINDING = "menu_button_binding"
    const val PREF_INJECTION_MODE = "injection_mode"
    const val PREF_MODE_TOGGLE_BUTTON = "mode_toggle_button"

    fun getPrefs(context: Context): SharedPreferences {
        val storageContext = if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.N && !context.isDeviceProtectedStorage) {
            try {
                context.createDeviceProtectedStorageContext()
            } catch (_: Exception) {
                context
            }
        } else {
            context
        }
        return storageContext.getSharedPreferences("fakeatv_device_prefs", Context.MODE_PRIVATE)
    }

    fun getMenuButtonBinding(context: Context): String {
        return getPrefs(context).getString(PREF_MENU_BUTTON_BINDING, "none") ?: "none"
    }

    fun setMenuButtonBinding(context: Context, buttonName: String) {
        getPrefs(context).edit().putString(PREF_MENU_BUTTON_BINDING, buttonName.lowercase()).apply()
    }

    /** Remote button that toggles the cursor mode instead of sending its key ("none" disables). */
    fun getModeToggleButton(context: Context): String {
        return getPrefs(context).getString(PREF_MODE_TOGGLE_BUTTON, "power") ?: "power"
    }

    fun setModeToggleButton(context: Context, buttonName: String) {
        getPrefs(context).edit().putString(PREF_MODE_TOGGLE_BUTTON, buttonName.lowercase()).apply()
    }

    fun getInjectionMode(context: Context): String {
        return getPrefs(context).getString(PREF_INJECTION_MODE, "auto") ?: "auto"
    }

    fun setInjectionMode(context: Context, mode: String) {
        getPrefs(context).edit().putString(PREF_INJECTION_MODE, mode.lowercase()).apply()
    }

    fun getDeviceName(context: Context): String {
        val prefs = getPrefs(context)
        val customName = prefs.getString("custom_device_name", null)
        if (!customName.isNullOrBlank()) {
            return customName
        }

        // Check if device is Dangbei
        if (Build.BRAND.equals("Dangbei", ignoreCase = true) ||
            Build.MANUFACTURER.equals("Dangbei", ignoreCase = true) ||
            Build.MODEL.startsWith("DBBOX", ignoreCase = true)
        ) {
            return "当贝盒子"
        }

        val systemName = try {
            Settings.Global.getString(context.contentResolver, Settings.Global.DEVICE_NAME)
        } catch (_: Exception) {
            null
        }

        if (!systemName.isNullOrBlank() && systemName != "Android TV") {
            return systemName
        }

        return Build.MODEL?.takeIf { it.isNotBlank() } ?: "Android TV"
    }

    fun getDeviceId(context: Context): String {
        val prefs = getPrefs(context)
        val cached = prefs.getString("device_id", null)
        if (!cached.isNullOrBlank()) {
            return cached
        }

        // Try reading hardware MAC address
        val mac = getHardwareMac()
        val deviceId = if (mac != null) {
            mac.uppercase()
        } else {
            // Fallback: derive 6-byte hex from android_id
            val androidId = try {
                Settings.Secure.getString(context.contentResolver, Settings.Secure.ANDROID_ID)
            } catch (_: Exception) {
                null
            } ?: UUID.randomUUID().toString().replace("-", "")

            val clean = androidId.padEnd(12, '0').take(12).uppercase()
            clean.chunked(2).joinToString(":")
        }

        prefs.edit().putString("device_id", deviceId).apply()
        return deviceId
    }

    fun getServerIdentifier(context: Context, deviceId: String): String {
        val prefs = getPrefs(context)
        val cached = prefs.getString("server_uuid", null)
        if (!cached.isNullOrBlank()) {
            return cached
        }

        val uuid = UUID.nameUUIDFromBytes("FakeAtv:$deviceId".toByteArray()).toString().uppercase()
        prefs.edit().putString("server_uuid", uuid).apply()
        return uuid
    }

    fun getLocalIpAddress(context: Context): String? {
        // 1. Try ConnectivityManager on API 23+
        try {
            val cm = context.getSystemService(Context.CONNECTIVITY_SERVICE) as? ConnectivityManager
            if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.M) {
                val network = cm?.activeNetwork
                if (network != null) {
                    val linkProps = cm.getLinkProperties(network)
                    if (linkProps != null) {
                        for (linkAddr in linkProps.linkAddresses) {
                            val addr = linkAddr.address
                            if (addr is Inet4Address && !addr.isLoopbackAddress) {
                                val host = addr.hostAddress
                                if (!host.isNullOrBlank() && host != "0.0.0.0" && host != "127.0.0.1") {
                                    return host
                                }
                            }
                        }
                    }
                }
            }
        } catch (_: Exception) {}

        // 2. Iterate NetworkInterfaces (works for both Wi-Fi and Ethernet on all Android versions)
        try {
            val interfaces = NetworkInterface.getNetworkInterfaces() ?: return null
            for (intf in interfaces) {
                if (intf.isLoopback || !intf.isUp) continue
                for (addr in intf.inetAddresses) {
                    if (addr is Inet4Address && !addr.isLoopbackAddress) {
                        val host = addr.hostAddress
                        if (!host.isNullOrBlank() && host != "0.0.0.0" && host != "127.0.0.1") {
                            return host
                        }
                    }
                }
            }
        } catch (_: Exception) {}

        return null
    }

    private fun getHardwareMac(): String? {
        return try {
            val interfaces = NetworkInterface.getNetworkInterfaces() ?: return null
            for (intf in interfaces) {
                if (intf.isLoopback) continue
                val name = intf.name.lowercase()
                if (name.startsWith("wlan") || name.startsWith("eth")) {
                    val addr = intf.hardwareAddress ?: continue
                    if (addr.size == 6 && !isDummyMac(addr)) {
                        return addr.joinToString(":") { "%02X".format(it) }
                    }
                }
            }
            null
        } catch (_: Exception) {
            null
        }
    }

    private fun isDummyMac(bytes: ByteArray): Boolean {
        return bytes.all { it == 0.toByte() } ||
                (bytes[0] == 2.toByte() && bytes.drop(1).all { it == 0.toByte() })
    }
}
