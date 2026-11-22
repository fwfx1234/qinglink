package com.qinglink.mobile

import android.app.Activity
import android.app.PendingIntent
import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import android.content.pm.PackageInfo
import android.content.pm.PackageInstaller
import android.content.pm.PackageManager
import android.net.Uri
import android.os.Build
import android.os.Handler
import android.os.Looper
import android.provider.Settings
import io.flutter.plugin.common.MethodCall
import io.flutter.plugin.common.MethodChannel
import java.io.File
import java.security.MessageDigest
import java.util.concurrent.Executors

/**
 * 应用内安装 APK 的原生通道（`com.qinglink.mobile/install`）。
 *
 * 为什么不继续用 open_filex 的 ACTION_VIEW：它只回「打开了/没权限/失败」三种结果，
 * 系统安装器真正的原因（例如 INSTALL_PARSE_FAILED_NO_CERTIFICATES）全被丢掉，
 * 用户只能看到一句「安装包未包含任何证书」，无从判断是文件没下完还是签名不一致。
 *
 * 这里改走 PackageInstaller 会话，安装结果通过 [InstallResultReceiver] 回来的
 * EXTRA_STATUS / EXTRA_STATUS_MESSAGE 原样上报；同时在安装前提供
 * `inspect`/`current`，让 Dart 侧能先确认「这包是谁、版本号多少、签名跟已装的一致吗」。
 */
class InstallChannel(private val activity: Activity) : MethodChannel.MethodCallHandler {

    private val main = Handler(Looper.getMainLooper())
    private val executor = Executors.newSingleThreadExecutor()
    private var pending: MethodChannel.Result? = null

    init {
        InstallResultBridge.listener = { intent -> main.post { handleInstallResult(intent) } }
    }

    override fun onMethodCall(call: MethodCall, result: MethodChannel.Result) {
        when (call.method) {
            "inspect" -> inspectArchive(call.argument<String>("path"), result)
            "current" -> result.success(installedPackage())
            "install" -> startInstall(call.argument<String>("path"), result)
            "openInstallSettings" -> openInstallSettings(result)
            else -> result.notImplemented()
        }
    }

    /** 读取一个尚未安装的 APK 的包名 / 版本 / 签名者；读不出来说明文件已损坏。 */
    private fun inspectArchive(path: String?, result: MethodChannel.Result) {
        if (path.isNullOrEmpty()) {
            result.error("INVALID_ARGUMENT", "path 为空", null)
            return
        }
        try {
            result.success(archivePackage(path))
        } catch (error: Exception) {
            result.error("INSPECT_FAILED", error.message ?: "读取安装包信息失败", null)
        }
    }

    /**
     * 发起安装。结果可能是：
     * - `{"status":"permission"}`：还没被允许「安装未知应用」，需引导用户去设置；
     * - `{"status":"success"}`：装成功了；
     * - `{"status":"failed","code":<int>,"message":"..."}`：失败，message 是系统原话。
     *
     * 中间会有一次 STATUS_PENDING_USER_ACTION（系统要求用户点确认），此时先把
     * 确认框拉起来，等最终结果回来再 complete 这次 MethodChannel 调用。
     */
    private fun startInstall(path: String?, result: MethodChannel.Result) {
        if (pending != null) {
            result.error("BUSY", "上一次安装还没有结束", null)
            return
        }
        val file = path?.let { File(it) }
        if (file == null || !file.isFile || file.length() <= 0L) {
            result.error("INVALID_APK", "安装包不存在或为空：$path", null)
            return
        }
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O &&
            !activity.packageManager.canRequestPackageInstalls()
        ) {
            // 权限归用户，代码不能代劳；返回专用状态让 Dart 侧给出明确入口。
            result.success(mapOf("status" to "permission"))
            return
        }
        pending = result
        executor.execute { runInstall(file) }
    }

    private fun runInstall(file: File) {
        try {
            val installer = activity.packageManager.packageInstaller
            val params = PackageInstaller.SessionParams(
                PackageInstaller.SessionParams.MODE_FULL_INSTALL,
            )
            val sessionId = installer.createSession(params)
            installer.openSession(sessionId).use { session ->
                file.inputStream().use { input ->
                    session.openWrite("base.apk", 0, file.length()).use { output ->
                        input.copyTo(output, DEFAULT_BUFFER_SIZE)
                        session.fsync(output)
                    }
                }
                val intent = Intent(activity, InstallResultReceiver::class.java)
                    .setAction(ACTION_INSTALL_RESULT)
                    .putExtra(EXTRA_SESSION_ID, sessionId)
                val flags = PendingIntent.FLAG_UPDATE_CURRENT or
                    if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.S) {
                        PendingIntent.FLAG_MUTABLE
                    } else {
                        0
                    }
                val pendingIntent = PendingIntent.getBroadcast(activity, sessionId, intent, flags)
                session.commit(pendingIntent.intentSender)
            }
        } catch (error: Exception) {
            finish(
                mapOf(
                    "status" to "failed",
                    "message" to (error.message ?: "创建安装会话失败"),
                ),
            )
        }
    }

    private fun handleInstallResult(intent: Intent) {
        val status = intent.getIntExtra(PackageInstaller.EXTRA_STATUS, PackageInstaller.STATUS_FAILURE)
        if (status == PackageInstaller.STATUS_PENDING_USER_ACTION) {
            // 系统需要用户点一次「安装」，把它的确认界面拉起来；最终结果会再回调一次。
            val confirm = if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU) {
                intent.getParcelableExtra(Intent.EXTRA_INTENT, Intent::class.java)
            } else {
                @Suppress("DEPRECATION")
                intent.getParcelableExtra(Intent.EXTRA_INTENT)
            }
            if (confirm == null) {
                finish(mapOf("status" to "failed", "message" to "系统没有返回安装确认界面"))
                return
            }
            confirm.addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)
            try {
                activity.startActivity(confirm)
            } catch (error: Exception) {
                finish(
                    mapOf(
                        "status" to "failed",
                        "message" to "无法打开系统安装确认框：${error.message}",
                    ),
                )
            }
            return
        }
        if (status == PackageInstaller.STATUS_SUCCESS) {
            finish(mapOf("status" to "success"))
            return
        }
        // EXTRA_STATUS_MESSAGE 才是 INSTALL_PARSE_FAILED_NO_CERTIFICATES 这类具体原因，
        // 只报状态码会变成另一句「安装失败」，等于没修。
        finish(
            mapOf(
                "status" to "failed",
                "code" to status,
                "message" to (intent.getStringExtra(PackageInstaller.EXTRA_STATUS_MESSAGE) ?: ""),
            ),
        )
    }

    private fun openInstallSettings(result: MethodChannel.Result) {
        try {
            val intent = Intent(
                Settings.ACTION_MANAGE_UNKNOWN_APP_SOURCES,
                Uri.parse("package:${activity.packageName}"),
            ).addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)
            activity.startActivity(intent)
            result.success(null)
        } catch (error: Exception) {
            result.error("OPEN_SETTINGS_FAILED", error.message, null)
        }
    }

    private fun finish(payload: Map<String, Any?>) {
        main.post {
            pending?.success(payload)
            pending = null
        }
    }

    // ---- 包信息读取 ----

    private fun signerFlags(): Int =
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.P) {
            PackageManager.GET_SIGNING_CERTIFICATES
        } else {
            @Suppress("DEPRECATION")
            PackageManager.GET_SIGNATURES
        }

    private fun archivePackage(path: String): Map<String, Any?>? {
        val info = activity.packageManager.getPackageArchiveInfo(path, signerFlags()) ?: return null
        return describe(info)
    }

    private fun installedPackage(): Map<String, Any?>? {
        val info = try {
            activity.packageManager.getPackageInfo(activity.packageName, signerFlags())
        } catch (_: PackageManager.NameNotFoundException) {
            return null
        }
        return describe(info)
    }

    private fun describe(info: PackageInfo): Map<String, Any?> {
        val versionCode = if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.P) {
            info.longVersionCode
        } else {
            @Suppress("DEPRECATION")
            info.versionCode.toLong()
        }
        return mapOf(
            "packageName" to info.packageName,
            "versionCode" to versionCode,
            "versionName" to info.versionName,
            "signerSha256" to signerDigests(info),
        )
    }

    @Suppress("DEPRECATION")
    private fun signerDigests(info: PackageInfo): List<String> {
        val signatures = if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.P) {
            val signing = info.signingInfo ?: return emptyList()
            if (signing.hasMultipleSigners()) {
                signing.apkContentsSigners
            } else {
                signing.signingCertificateHistory
            }
        } else {
            info.signatures ?: return emptyList()
        }
        return signatures.map { sha256(it.toByteArray()) }
    }

    private fun sha256(bytes: ByteArray): String =
        MessageDigest.getInstance("SHA-256").digest(bytes)
            .joinToString("") { "%02x".format(it) }

    companion object {
        const val CHANNEL = "com.qinglink.mobile/install"
        private const val ACTION_INSTALL_RESULT = "com.qinglink.mobile.INSTALL_RESULT"
        private const val EXTRA_SESSION_ID = "session_id"
    }
}

/**
 * PendingIntent 的结果落点。由 PackageInstaller 以本应用身份发出，因此
 * manifest 里 `android:exported="false"` 即可收到，不需要对外暴露。
 */
object InstallResultBridge {
    var listener: ((Intent) -> Unit)? = null
}

class InstallResultReceiver : BroadcastReceiver() {
    override fun onReceive(context: Context, intent: Intent) {
        InstallResultBridge.listener?.invoke(intent)
    }
}
