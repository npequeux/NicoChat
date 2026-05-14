package com.nicochat.app

import android.os.Bundle
import android.util.Log
import android.webkit.WebChromeClient
import android.webkit.WebView
import android.webkit.WebViewClient
import android.widget.Button
import android.widget.EditText
import androidx.activity.OnBackPressedCallback
import androidx.appcompat.app.AppCompatActivity
import java.net.HttpURLConnection
import java.net.Inet4Address
import java.net.NetworkInterface
import java.net.URL
import java.util.concurrent.Callable
import java.util.concurrent.ExecutorCompletionService
import java.util.concurrent.Executors

class MainActivity : AppCompatActivity() {
    companion object {
        private const val TAG = "NicoChatMainActivity"
        private const val SERVER_PORT = 5000
        private const val SCAN_THREAD_POOL_SIZE = 24
        private const val DETECTION_TIMEOUT_MS = 250
    }

    private lateinit var webView: WebView
    private val detectionExecutor = Executors.newSingleThreadExecutor()

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        setContentView(R.layout.activity_main)

        val addressInput = findViewById<EditText>(R.id.addressInput)
        val connectButton = findViewById<Button>(R.id.connectButton)
        webView = findViewById(R.id.chatWebView)

        webView.settings.javaScriptEnabled = true
        webView.settings.domStorageEnabled = true
        webView.webViewClient = WebViewClient()
        webView.webChromeClient = WebChromeClient()

        val preferences = getSharedPreferences("nicochat", MODE_PRIVATE)
        val defaultUrl = preferences.getString("server_url", "http://10.0.2.2:5000").orEmpty()
        addressInput.setText(defaultUrl)

        connectButton.setOnClickListener {
            val url = normalizeUrl(addressInput.text.toString())

            preferences.edit().putString("server_url", url).apply()
            webView.loadUrl(url)
        }

        if (savedInstanceState == null) {
            detectionExecutor.execute {
                val initialUrl = if (isServerReachable(defaultUrl)) {
                    defaultUrl
                } else {
                    detectServerOnLocalNetwork() ?: defaultUrl
                }

                runOnUiThread {
                    addressInput.setText(initialUrl)
                    preferences.edit().putString("server_url", initialUrl).apply()
                    webView.loadUrl(initialUrl)
                }
            }
        } else {
            webView.restoreState(savedInstanceState)
        }

        onBackPressedDispatcher.addCallback(this, object : OnBackPressedCallback(true) {
            override fun handleOnBackPressed() {
                if (webView.canGoBack()) {
                    webView.goBack()
                } else {
                    isEnabled = false
                    onBackPressedDispatcher.onBackPressed()
                }
            }
        })
    }

    override fun onSaveInstanceState(outState: Bundle) {
        super.onSaveInstanceState(outState)
        webView.saveState(outState)
    }

    override fun onDestroy() {
        detectionExecutor.shutdownNow()
        super.onDestroy()
    }

    private fun normalizeUrl(rawUrl: String): String {
        val value = rawUrl.trim()
        return if (value.startsWith("http://") || value.startsWith("https://")) {
            value
        } else {
            "http://$value"
        }
    }

    private fun detectServerOnLocalNetwork(): String? {
        val candidates = linkedSetOf("http://10.0.2.2:$SERVER_PORT")
        for (prefix in localSubnetPrefixes()) {
            for (host in 1..254) {
                candidates.add("http://$prefix.$host:$SERVER_PORT")
            }
        }

        val scanner = Executors.newFixedThreadPool(SCAN_THREAD_POOL_SIZE)
        val completion = ExecutorCompletionService<String?>(scanner)
        return try {
            candidates.forEach { baseUrl ->
                completion.submit(Callable {
                    if (isServerReachable(baseUrl)) baseUrl else null
                })
            }

            var found: String? = null
            for (_ in 0 until candidates.size) {
                val result = completion.take().get()
                if (result != null) {
                    found = result
                    break
                }
            }
            found
        } catch (exc: Exception) {
            Log.w(TAG, "Server detection failed", exc)
            null
        } finally {
            scanner.shutdownNow()
        }
    }

    private fun localSubnetPrefixes(): Set<String> {
        val prefixes = linkedSetOf<String>()
        try {
            val interfaces = NetworkInterface.getNetworkInterfaces() ?: return prefixes
            while (interfaces.hasMoreElements()) {
                val networkInterface = interfaces.nextElement()
                val addresses = networkInterface.inetAddresses
                while (addresses.hasMoreElements()) {
                    val address = addresses.nextElement()
                    if (address is Inet4Address && !address.isLoopbackAddress && address.isSiteLocalAddress) {
                        val host = address.hostAddress ?: continue
                        val parts = host.split(".")
                        if (parts.size == 4) {
                            prefixes.add("${parts[0]}.${parts[1]}.${parts[2]}")
                        }
                    }
                }
            }
        } catch (exc: Exception) {
            Log.w(TAG, "Failed to enumerate local subnets", exc)
            return prefixes
        }
        return prefixes
    }

    private fun isServerReachable(baseUrl: String): Boolean {
        val normalizedBase = normalizeUrl(baseUrl).removeSuffix("/")
        val connection = (URL("$normalizedBase/models").openConnection() as HttpURLConnection).apply {
            requestMethod = "GET"
            connectTimeout = DETECTION_TIMEOUT_MS
            readTimeout = DETECTION_TIMEOUT_MS
            instanceFollowRedirects = true
        }

        return try {
            val code = connection.responseCode
            code in 200..399
        } catch (_: Exception) {
            false
        } finally {
            connection.disconnect()
        }
    }
}
