package com.qinglink.mobile

import android.content.Context
import android.content.pm.PackageManager
import android.hardware.ConsumerIrManager
import android.os.Handler
import android.os.Looper
import io.flutter.embedding.engine.FlutterEngine
import io.flutter.embedding.android.FlutterActivity
import io.flutter.plugin.common.MethodChannel

class MainActivity : FlutterActivity() {
    private val channelName = "com.qinglink.mobile/ir"

    override fun configureFlutterEngine(flutterEngine: FlutterEngine) {
        super.configureFlutterEngine(flutterEngine)
        MethodChannel(flutterEngine.dartExecutor.binaryMessenger, channelName)
            .setMethodCallHandler { call, result ->
                val manager = getSystemService(Context.CONSUMER_IR_SERVICE) as? ConsumerIrManager
                when (call.method) {
                    "getCapabilities" -> {
                        val featureSupported = packageManager.hasSystemFeature(
                            PackageManager.FEATURE_CONSUMER_IR,
                        )
                        val serviceSupported = manager != null
                        // A ConsumerIrManager service can be registered on devices
                        // without an IR LED. Query both the feature flag and the
                        // manager, while tolerating OEM implementations that throw.
                        val emitterSupported = try {
                            manager?.hasIrEmitter() == true
                        } catch (_: Exception) {
                            false
                        }
                        val ranges = try {
                            manager?.carrierFrequencies?.map { range ->
                                listOf(range.minFrequency, range.maxFrequency)
                            } ?: emptyList()
                        } catch (_: Exception) {
                            emptyList()
                        }
                        val frequencies = ranges.flatMap { range ->
                            listOf(
                                range[0],
                                (range[0] + range[1]) / 2,
                                range[1],
                            )
                        }.distinct()
                        // Some ROMs incorrectly report hasIrEmitter() and the
                        // feature flag as false even when the registered
                        // ConsumerIrManager can transmit. Treating the service
                        // as a compatibility fallback avoids a false negative;
                        // transmit() remains the authoritative hardware check.
                        val supported = manager != null &&
                            (featureSupported || serviceSupported || emitterSupported || ranges.isNotEmpty())
                        result.success(mapOf(
                            "supported" to supported,
                            "featureSupported" to featureSupported,
                            "serviceSupported" to serviceSupported,
                            "emitterSupported" to emitterSupported,
                            "carrierFrequencies" to frequencies,
                            "carrierRanges" to ranges,
                        ))
                    }
                    "transmit" -> {
                        if (manager == null) {
                            result.error("UNSUPPORTED", "当前手机没有红外发射器", null)
                            return@setMethodCallHandler
                        }
                        val carrier = call.argument<Number>("carrierHz")?.toInt()
                        val rawPattern = call.argument<List<*>>("patternUs")
                        val pattern = rawPattern
                            ?.takeIf { values -> values.all { it is Number } }
                            ?.map { (it as Number).toInt() }
                        val repeatCount = call.argument<Number>("repeatCount")?.toInt() ?: 1
                        if (carrier == null || carrier <= 0 || carrier > 500_000 ||
                            pattern == null || pattern.isEmpty() || pattern.size % 2 != 0 ||
                            pattern.any { it <= 0 } || pattern.size > 4096 ||
                            pattern.sumOf { it.toLong() } > 2_000_000L || repeatCount !in 1..5) {
                            result.error("INVALID_PATTERN", "红外参数无效", null)
                            return@setMethodCallHandler
                        }
                        // ConsumerIrManager.transmit may block for the duration of
                        // a long pattern. Keep the Flutter/UI thread responsive.
                        Thread {
                            try {
                                repeat(repeatCount) {
                                    manager.transmit(carrier, pattern.toIntArray())
                                    if (repeatCount > 1) Thread.sleep(40)
                                }
                                Handler(Looper.getMainLooper()).post { result.success(null) }
                            } catch (error: SecurityException) {
                                Handler(Looper.getMainLooper()).post {
                                    result.error(
                                        "PERMISSION_DENIED",
                                        "缺少红外发射权限，请重新安装应用以授予 TRANSMIT_IR",
                                        null,
                                    )
                                }
                            } catch (error: Exception) {
                                Handler(Looper.getMainLooper()).post {
                                    result.error("TRANSMIT_FAILED", error.message ?: "红外发送失败", null)
                                }
                            }
                        }.start()
                    }
                    else -> result.notImplemented()
                }
            }
    }
}
