/// 认证失败错误码的分类与提示文案。
///
/// 只做纯映射，不触碰状态与设置；由会话连接与配对流程共用，保证同一个错误码
/// 在两处给出同样的提示。
class AuthFailurePolicy {
  const AuthFailurePolicy._();

  /// 服务端拒绝已保存令牌的错误码。
  static bool isInvalidTokenCode(String code) {
    return code == 'INVALID_TOKEN' ||
        code == 'TOKEN_EXPIRED' ||
        code == 'AUTH_FAILED';
  }

  /// 需要提示给用户的文案；无需提示时返回 `null`。
  static String? messageFor(String code) {
    if (code == 'REPLACED') {
      return '此设备已在其他客户端建立连接';
    }
    // The PC withdrew its listener (pause or service restart). The pairing is
    // still valid and RemoteSession keeps retrying, so only surface a hint.
    if (code == 'SERVICE_PAUSED') {
      return '电脑已暂停远程控制，恢复后会自动重连';
    }
    if (code == 'SERVICE_STOPPING') {
      return '电脑端服务正在重启，恢复后会自动重连';
    }
    return null;
  }
}
