/// User-facing text for the stable pairing error codes in `protocol/v2.md`.
///
/// The server rejects pairing with a non-2xx response carrying
/// `{"ok":false,"error":{"code":...,"message":...}}`; those codes are contract,
/// not copy, so they must never reach the UI verbatim.
String pairingFailureText(String code, [String? fallback]) {
  switch (code) {
    case 'PAIRING_REQUIRES_DESKTOP':
      return '请在电脑上打开并解锁轻联，再试一次';
    case 'PAIRING_BUSY':
      return '电脑上待确认的配对请求过多，请稍后再试';
    case 'INVALID_REQUEST':
      return '设备信息无效，请重启 App 后重试';
    case 'PAIRING_EXPIRED':
    case 'PAIRING_NOT_FOUND':
      return '配对请求已失效，请重新配对';
    case 'INVALID_PIN':
      return '配对码不正确，请重新输入';
    case 'RATE_LIMITED':
      return '尝试次数过多，请重新发起配对';
    case 'DEVICE_MISMATCH':
      return '设备标识不匹配，请重新发起配对';
    case 'PAIRING_UNAVAILABLE':
    case 'TOKEN_CREATE_FAILED':
      return '电脑端配对服务不可用，请稍后重试';
    case 'PAIRING_NETWORK_ERROR':
      return '无法连接到电脑，请检查网络';
  }
  if (fallback != null && fallback.isNotEmpty) return fallback;
  return '配对失败，请重试';
}
