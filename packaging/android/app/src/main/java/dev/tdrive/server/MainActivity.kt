package dev.tdrive.server

import android.annotation.SuppressLint
import android.app.Activity
import android.content.ActivityNotFoundException
import android.content.Intent
import android.content.pm.PackageManager
import android.net.Uri
import android.os.Build
import android.os.Bundle
import android.os.Handler
import android.os.Looper
import android.view.Gravity
import android.view.View
import android.view.ViewGroup
import android.webkit.CookieManager
import android.webkit.WebChromeClient
import android.webkit.WebView
import android.webkit.WebViewClient
import android.widget.Button
import android.widget.FrameLayout
import android.widget.LinearLayout
import android.widget.ProgressBar
import android.widget.TextView
import android.widget.Toast
import java.net.InetSocketAddress
import java.net.Socket

/**
 * Hosts the TDrive web UI against the loopback server started by
 * [TdriveService]. Onboarding, browsing, previews and uploads all happen in
 * that page, so the activity only waits for the port, shows failures, and
 * keeps the WebView on the app's own origin.
 */
class MainActivity : Activity() {

    private lateinit var webView: WebView
    private lateinit var overlay: LinearLayout
    private lateinit var message: TextView
    private lateinit var progress: ProgressBar
    private lateinit var retry: Button
    private val handler = Handler(Looper.getMainLooper())
    private var loaded = false
    private var probing = false

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        buildUi()
        requestNotificationPermission()
        TdriveService.start(this)
        probe(0)
    }

    @SuppressLint("SetJavaScriptEnabled")
    private fun buildUi() {
        val root = FrameLayout(this)

        webView = WebView(this)
        webView.setBackgroundColor(0xFFFFFFFF.toInt())
        webView.settings.javaScriptEnabled = true
        webView.settings.domStorageEnabled = true
        webView.settings.mediaPlaybackRequiresUserGesture = false
        webView.settings.allowFileAccess = false
        webView.settings.allowContentAccess = false
        webView.webViewClient = WebViewClient()
        webView.webChromeClient = WebChromeClient()
        webView.setDownloadListener { url, _, _, _, _ ->
            // /file URLs already carry the access token, so the system browser
            // can finish the download the WebView itself cannot save.
            try {
                startActivity(Intent(Intent.ACTION_VIEW, Uri.parse(url)))
            } catch (e: ActivityNotFoundException) {
                Toast.makeText(this, R.string.download_failed, Toast.LENGTH_SHORT).show()
            }
        }
        CookieManager.getInstance().setAcceptCookie(true)
        root.addView(
            webView,
            FrameLayout.LayoutParams(
                ViewGroup.LayoutParams.MATCH_PARENT,
                ViewGroup.LayoutParams.MATCH_PARENT
            )
        )

        overlay = LinearLayout(this)
        overlay.orientation = LinearLayout.VERTICAL
        overlay.gravity = Gravity.CENTER
        overlay.setBackgroundColor(0xFFFFFFFF.toInt())
        overlay.setPadding(dp(32), dp(32), dp(32), dp(32))

        progress = ProgressBar(this)
        message = TextView(this)
        message.text = getString(R.string.starting)
        message.gravity = Gravity.CENTER
        message.setPadding(0, dp(16), 0, dp(16))
        retry = Button(this)
        retry.text = getString(R.string.retry)
        retry.visibility = View.GONE
        retry.setOnClickListener { restart() }

        overlay.addView(progress)
        overlay.addView(message)
        overlay.addView(retry)
        root.addView(
            overlay,
            FrameLayout.LayoutParams(
                ViewGroup.LayoutParams.MATCH_PARENT,
                ViewGroup.LayoutParams.MATCH_PARENT
            )
        )
        setContentView(root)
    }

    private fun dp(value: Int): Int = (value * resources.displayMetrics.density).toInt()

    private fun probe(attempt: Int) {
        if (probing) return
        val failure = TdriveService.lastError
        if (failure != null) {
            showError(failure)
            return
        }
        probing = true
        Thread({
            var up = false
            try {
                Socket().use { socket ->
                    socket.connect(InetSocketAddress("127.0.0.1", TdriveService.PORT), 300)
                    up = true
                }
            } catch (e: Exception) {
                up = false
            }
            handler.post {
                probing = false
                val error = TdriveService.lastError
                when {
                    up -> loadUi()
                    error != null -> showError(error)
                    attempt >= 100 -> showError(
                        getString(R.string.timeout) + "\n\n" + TdriveService.logTail(this)
                    )
                    else -> handler.postDelayed({ probe(attempt + 1) }, 200)
                }
            }
        }, "tdrive-probe").start()
    }

    private fun loadUi() {
        val url = TdriveService.webUrl ?: return
        if (!loaded) {
            loaded = true
            webView.loadUrl(url)
        }
        overlay.visibility = View.GONE
    }

    private fun showError(text: String) {
        progress.visibility = View.GONE
        retry.visibility = View.VISIBLE
        message.text = text
        overlay.visibility = View.VISIBLE
    }

    private fun restart() {
        message.text = getString(R.string.starting)
        progress.visibility = View.VISIBLE
        retry.visibility = View.GONE
        loaded = false
        TdriveService.start(this)
        probe(0)
    }

    private fun requestNotificationPermission() {
        if (Build.VERSION.SDK_INT < Build.VERSION_CODES.TIRAMISU) return
        val granted = checkSelfPermission(android.Manifest.permission.POST_NOTIFICATIONS) ==
            PackageManager.PERMISSION_GRANTED
        if (!granted) {
            requestPermissions(arrayOf(android.Manifest.permission.POST_NOTIFICATIONS), 1)
        }
    }

    override fun onResume() {
        super.onResume()
        if (this::webView.isInitialized) webView.onResume()
    }

    override fun onPause() {
        if (this::webView.isInitialized) webView.onPause()
        super.onPause()
    }

    @Suppress("DEPRECATION")
    override fun onBackPressed() {
        if (this::webView.isInitialized && webView.canGoBack()) {
            webView.goBack()
            return
        }
        super.onBackPressed()
    }

    override fun onDestroy() {
        handler.removeCallbacksAndMessages(null)
        if (this::webView.isInitialized) {
            webView.stopLoading()
            webView.destroy()
        }
        if (isFinishing) TdriveService.stop(this)
        super.onDestroy()
    }
}
