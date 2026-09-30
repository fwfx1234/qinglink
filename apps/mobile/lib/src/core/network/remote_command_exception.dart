class RemoteCommandException implements Exception {
  const RemoteCommandException(this.code, this.message);

  factory RemoteCommandException.fromResponse(Map<String, dynamic> response) {
    final data = response['data'];
    final error = data is Map ? Map<String, dynamic>.from(data) : const <String, dynamic>{};
    return RemoteCommandException(
      error['code'] as String? ?? 'REMOTE_COMMAND_FAILED',
      error['message'] as String? ?? 'PC 未能完成该操作',
    );
  }

  final String code;
  final String message;

  @override
  String toString() => '$code: $message';
}
