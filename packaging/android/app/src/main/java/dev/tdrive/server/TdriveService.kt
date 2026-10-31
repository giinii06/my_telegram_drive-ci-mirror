package dev.tdrive.server

import android.app.Notification
import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.PendingIntent
import android.app.Service
import android.content.Context
import android.content.Intent
import android.os.Build
import android.os.IBinder
import android.util.Log
import java.io.File
import java.security.SecureRandom

/**
 * Runs the bundled Go server (libtdrive.so) as a foreground process and keeps
 * its web UI reachable on loopback.
 *
 * The binary lives in nativeLibraryDir because that is the one app-owned
 * directory Android lets an app execute from. All state stays in filesDir:
 * HOME, XDG_RUNTIME_DIR (the daemon's unix socket) and the web token.
 */
class TdriveService : Service() {

    companion object {
        const val PORT = 8080

        private const val TAG = "TdriveService"
        private const val CHANNEL_ID = "tdrive"
        private const val NOTIFICATION_ID = 1
        private const val LOG_NAME = "server.log"

        @Volatile
        var webUrl: String? = null
            private set

        @Volatile
        var lastError: String? = null
            private set

        @Volatile
        var running: Boolean = false
            private set

        fun start(context: Context) {
            val intent = Intent(context, TdriveService::class.java)
            if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
                context.startForegroundService(intent)
            } else {
                context.startService(intent)
            }
        }

        fun stop(context: Context) {
            context.stopService(Intent(context, TdriveService::class.java))
        }

        fun logTail(context: Context, lines: Int = 12): String {
            return try {
                val file = File(context.filesDir, LOG_NAME)
                if (!file.exists()) "" else file.readLines().takeLast(lines).joinToString("\n")
            } catch (e: Exception) {
                Log.w(TAG, "read server log", e)
                ""
            }
        }
    }

    private var process: Process? = null
    private var stopping = false

    override fun onBind(intent: Intent?): IBinder? = null

    override fun onCreate() {
        super.onCreate()
        createChannel()
    }

    override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
        startForeground(NOTIFICATION_ID, buildNotification())
        if (process == null) startServer()
        return START_STICKY
    }

    override fun onDestroy() {
        stopping = true
        stopServer()
        running = false
        webUrl = null
        super.onDestroy()
    }

    private fun startServer() {
        lastError = null
        val binary = binary()
        if (!binary.exists()) {
            lastError = "서버 바이너리를 찾을 수 없습니다: ${binary.absolutePath}"
            return
        }
        val home = filesDir
        val log = File(home, LOG_NAME)
        val token = readOrCreateToken()
        webUrl = "http://127.0.0.1:$PORT/?token=$token"
        try {
            val builder = ProcessBuilder(
                binary.absolutePath,
                "web",
                "--listen", "127.0.0.1",
                "--port", PORT.toString(),
                "--token", token
            )
            builder.directory(home)
            builder.redirectErrorStream(true)
            builder.redirectOutput(ProcessBuilder.Redirect.appendTo(log))
            val env = builder.environment()
            env["HOME"] = home.absolutePath
            env["TMPDIR"] = cacheDir.absolutePath
            env["XDG_RUNTIME_DIR"] = home.absolutePath
            env["TDRIVE_NO_OS_MOUNT"] = "1"
            val started = builder.start()
            process = started
            running = true
            Thread({
                val code = try {
                    started.waitFor()
                } catch (e: InterruptedException) {
                    -1
                }
                running = false
                if (!stopping) {
                    lastError = "서버가 종료되었습니다 (code $code)\n" + logTail(this, 12)
                    Log.w(TAG, "server exited with code $code")
                }
            }, "tdrive-wait").start()
        } catch (e: Exception) {
            process = null
            running = false
            lastError = "서버를 시작하지 못했습니다: ${e.message}"
            Log.e(TAG, "start server", e)
        }
    }

    private fun stopServer() {
        val web = process
        process = null
        if (web != null) {
            try {
                web.destroy()
            } catch (e: Exception) {
                Log.w(TAG, "stop web server", e)
            }
        }
        val binary = binary()
        if (!binary.exists()) return
        Thread({
            try {
                val builder = ProcessBuilder(binary.absolutePath, "daemon", "stop")
                builder.directory(filesDir)
                builder.redirectErrorStream(true)
                builder.redirectOutput(ProcessBuilder.Redirect.appendTo(File(cacheDir, "stop.log")))
                val env = builder.environment()
                env["HOME"] = filesDir.absolutePath
                env["TMPDIR"] = cacheDir.absolutePath
                env["XDG_RUNTIME_DIR"] = filesDir.absolutePath
                builder.start().waitFor()
            } catch (e: Exception) {
                Log.w(TAG, "stop daemon", e)
            }
        }, "tdrive-stop").start()
    }

    private fun binary(): File = File(applicationInfo.nativeLibraryDir, "libtdrive.so")

    private fun readOrCreateToken(): String {
        val file = File(filesDir, "web.token")
        try {
            if (file.exists()) {
                val existing = file.readText().trim()
                if (existing.isNotEmpty()) return existing
            }
        } catch (e: Exception) {
            Log.w(TAG, "read web token", e)
        }
        val random = SecureRandom()
        val bytes = ByteArray(16)
        random.nextBytes(bytes)
        val token = bytes.joinToString("") { String.format("%02x", it.toInt() and 0xff) }
        try {
            file.writeText(token + "\n")
        } catch (e: Exception) {
            Log.w(TAG, "write web token", e)
        }
        return token
    }

    private fun createChannel() {
        if (Build.VERSION.SDK_INT < Build.VERSION_CODES.O) return
        val manager = getSystemService(Context.NOTIFICATION_SERVICE) as NotificationManager
        val channel = NotificationChannel(
            CHANNEL_ID,
            getString(R.string.notification_channel),
            NotificationManager.IMPORTANCE_LOW
        )
        manager.createNotificationChannel(channel)
    }

    private fun buildNotification(): Notification {
        val flags = PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE
        val pending = PendingIntent.getActivity(this, 0, Intent(this, MainActivity::class.java), flags)
        val builder = if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
            Notification.Builder(this, CHANNEL_ID)
        } else {
            Notification.Builder(this)
        }
        return builder
            .setSmallIcon(android.R.drawable.stat_sys_upload_done)
            .setContentTitle(getString(R.string.app_name))
            .setContentText(getString(R.string.notification_running))
            .setContentIntent(pending)
            .setOngoing(true)
            .build()
    }
}
