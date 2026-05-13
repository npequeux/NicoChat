package com.nicochat.app

import android.os.Bundle
import android.webkit.WebChromeClient
import android.webkit.WebView
import android.webkit.WebViewClient
import android.widget.Button
import android.widget.EditText
import androidx.appcompat.app.AppCompatActivity

class MainActivity : AppCompatActivity() {
    private lateinit var webView: WebView

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
            val rawUrl = addressInput.text.toString().trim()
            val url = if (rawUrl.startsWith("http://") || rawUrl.startsWith("https://")) {
                rawUrl
            } else {
                "http://$rawUrl"
            }

            preferences.edit().putString("server_url", url).apply()
            webView.loadUrl(url)
        }

        if (savedInstanceState == null) {
            webView.loadUrl(defaultUrl)
        } else {
            webView.restoreState(savedInstanceState)
        }
    }

    override fun onSaveInstanceState(outState: Bundle) {
        super.onSaveInstanceState(outState)
        webView.saveState(outState)
    }

    @Deprecated("Use OnBackPressedDispatcher instead.")
    override fun onBackPressed() {
        if (webView.canGoBack()) {
            webView.goBack()
        } else {
            super.onBackPressed()
        }
    }
}
