/// Typed failures shared by repositories and application controllers.
sealed class QingLinkFailure implements Exception {
  const QingLinkFailure(this.message, {this.cause, this.stackTrace});

  final String message;
  final Object? cause;
  final StackTrace? stackTrace;

  @override
  String toString() => message;
}

final class NetworkFailure extends QingLinkFailure {
  const NetworkFailure(super.message, {super.cause, super.stackTrace});
}

final class AuthenticationFailure extends QingLinkFailure {
  const AuthenticationFailure(
    super.message, {
    this.code,
    super.cause,
    super.stackTrace,
  });

  final String? code;
}

final class ProtocolFailure extends QingLinkFailure {
  const ProtocolFailure(super.message, {super.cause, super.stackTrace});
}

final class StorageFailure extends QingLinkFailure {
  const StorageFailure(super.message, {super.cause, super.stackTrace});
}

final class NativeFailure extends QingLinkFailure {
  const NativeFailure(super.message, {super.cause, super.stackTrace});
}

final class UnknownFailure extends QingLinkFailure {
  const UnknownFailure(super.message, {super.cause, super.stackTrace});
}
