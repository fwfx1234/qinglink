# Dart/Flutter 编码规范 — QingLink 移动端

基于 [Effective Dart](https://dart.dev/effective-dart) 和 [Flutter 风格指南](https://github.com/flutter/flutter/wiki/Style-guide-for-Flutter-repo)。

---

## 1. 命名规范

遵循 [Effective Dart: Style](https://dart.dev/effective-dart/style)。

| 项目 | 规范 | 示例 | 来源 |
|------|------|------|------|
| 文件 | `snake_case.dart` | `remote_session.dart` | Dart |
| 类/枚举/mixin/扩展 | `PascalCase` | `WsClient`, `WsConnectionState` | Dart |
| 函数/方法 | `lowerCamelCase` | `connect()`, `validateToken()` | Dart |
| 变量/字段 | `lowerCamelCase` | `reconnectAttempt`, `baseUrl` | Dart |
| 私有成员 | `_lowerCamelCase` | `_channel`, `_pendingRequests` | Dart |
| 常量 | `lowerCamelCase` | `maxBackoffSeconds`, `defaultTimeout` | Dart |
| 类型参数 | 单个大写字母 | `T`, `E`, `R` | Dart |
| 库/前缀 | `lowercase_with_underscore` | `app_utils.dart` → `import '...' as app_utils` | Dart |
| 缩写 | 当作普通单词 | `HttpServer` 而非 `HTTPServer`，`getUrl` 而非 `getURL` | Dart |

**使用完整单词** — 不使用歧义缩写：
- `message` 而非 `msg`
- `connection` 而非 `conn`
- `index` 而非 `idx`

**例外**（标准缩写）：`id`、`io`、`db` 单独出现时可接受。

---

## 2. Import 组织

遵循 [Effective Dart: Ordering](https://dart.dev/effective-dart/style#ordering)：

```dart
// 1. Dart SDK 导入
import 'dart:async';
import 'dart:convert';
import 'dart:io';

// 2. Flutter 框架
import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

// 3. 外部包（字母序）
import 'package:multicast_dns/multicast_dns.dart';
import 'package:shared_preferences/shared_preferences.dart';
import 'package:web_socket_channel/web_socket_channel.dart';

// 4. 内部导入（package 导入，字母序）
import 'package:qinglink/models/auth.dart';
import 'package:qinglink/models/device.dart';
import 'package:qinglink/services/ws_client.dart';
```

**规则：**
- 每组之间用空行分隔
- 每组内按字母序排序
- **禁止** `part` / `part of` 指令 — 使用直接导入
- **禁止** `export`（除了桶文件 `lib/qinglink.dart`）
- 使用 `show`/`hide` 选择性导入：`import 'dart:async' show Future, Stream;`

---

## 3. 类型安全

遵循 [Effective Dart: Usage](https://dart.dev/effective-dart/usage) — 类型注解和安全。

### 3.1 避免 `dynamic`

```dart
// ❌ 错误 — dynamic 使静态分析失效
dynamic parseData(String raw) { ... }
Future<dynamic> fetchConfig() async { ... }

// ✅ 正确 — 始终注解类型
Map<String, Object?> parseData(String raw) { ... }
Future<Config> fetchConfig() async { ... }
```

### 3.2 协议消息必须使用类型化 DTO

为每个协议消息定义请求/响应类型：

```dart
/// 发送给桌面端的鼠标移动请求。
@JsonSerializable()
class MouseMoveRequest {
  static const String type = 'mouse_move';
  
  final int dx;
  final int dy;
  final String requestId;

  const MouseMoveRequest({
    required this.dx,
    required this.dy,
    required this.requestId,
  });

  Map<String, Object?> toJson() => {
    'type': type,
    'dx': dx,
    'dy': dy,
    'request_id': requestId,
  };
}

// ❌ 错误 — 协议消息使用原始 Map
final payload = <String, dynamic>{
  'type': 'mouse_move',
  'dx': dx,
  'dy': dy,
};
```

### 3.3 使用 Sealed Class 处理响应类型

```dart
/// 来自桌面端的 WebSocket 响应。
sealed class WsResponse {
  const WsResponse();
}

class HealthOkResponse extends WsResponse {
  final int timestamp;
  const HealthOkResponse(this.timestamp);
  
  factory HealthOkResponse.fromJson(Map<String, Object?> json) {
    return HealthOkResponse(json['timestamp'] as int);
  }
}

class ErrorResponse extends WsResponse {
  final String code;
  final String message;
  const ErrorResponse(this.code, this.message);
  
  factory ErrorResponse.fromJson(Map<String, Object?> json) {
    return ErrorResponse(
      json['code'] as String,
      json['message'] as String,
    );
  }
}

class AckResponse extends WsResponse {
  final String requestId;
  const AckResponse(this.requestId);
}

// 使用 switch 表达式做穷尽解析（Dart 3）
WsResponse parseResponse(Map<String, Object?> json) {
  return switch (json['type']) {
    'health_ok' => HealthOkResponse.fromJson(json),
    'error' => ErrorResponse.fromJson(json),
    'ack' => AckResponse(json['request_id'] as String),
    _ => throw FormatException('未知响应类型: ${json['type']}'),
  };
}
```

### 3.4 优先使用 `final` 和 `const`

```dart
// ✅ 正确
final name = 'device';      // 运行时常量
const maxSize = 1024;       // 编译时常量
const defaultDuration = Duration(seconds: 30);  // 编译时常量

// ❌ 错误
var name = 'device';        // 不需要可变时使用了 mutable
int maxSize = 1024;         // 编译时已知值未用 const
```

### 3.5 空安全

始终利用 Dart 的空安全。没有明确不变量时绝不使用 `!`（空断言）：

```dart
// ✅ 正确 — 空安全访问 + 回退
final token = prefs.getString('auth_token') ?? '';

// ✅ 正确 — 显式空检查
final channel = _channel;
if (channel != null) {
  await channel.sink.close();
}

// ❌ 错误 — 盲目空断言
final token = prefs.getString('auth_token')!;  // 空值时崩溃
```

---

## 4. 错误处理

遵循 [Effective Dart: 错误处理](https://dart.dev/effective-dart/usage#handle-exceptions)。

### 4.1 禁止使用空 catch

```dart
// ❌ 错误 — 静默吞掉所有错误，包括编程错误
try {
  await ws.send(data);
} catch (_) {}

// ❌ 错误 — 捕获所有异常，仅打印不记录
try {
  await ws.send(data);
} catch (e) {
  print(e);  // 不是正确的日志器
}

// ✅ 正确 — 捕获特定异常，记录并处理
try {
  await ws.send(data);
} on SocketException catch (e) {
  _logger.warning('发送帧时网络错误', e);
  _scheduleReconnect();
} on TimeoutException catch (e) {
  _logger.warning('发送超时: ${e.duration}', e);
  _scheduleReconnect();
}
```

### 4.2 应用特定异常类型

```dart
/// 所有 QingLink 异常的基类。
abstract class QingLinkException implements Exception {
  String get message;
  String? get code;
}

class AuthException implements QingLinkException {
  @override
  final String message;
  @override
  final String? code;
  final int? statusCode;
  
  const AuthException(this.message, {this.code, this.statusCode});
}

class ConnectionException implements QingLinkException {
  @override
  final String message;
  @override
  final String? code;
  final Object? cause;
  
  const ConnectionException(this.message, {this.code, this.cause});
  
  @override
  String toString() => 'ConnectionException: $message';
}
```

### 4.3 对预期失败使用 Result 类型

```dart
/// 对可能失败但不异常的操作（如用户输入验证）。
sealed class Result<T> {
  const Result();
}

class Success<T> extends Result<T> {
  final T value;
  const Success(this.value);
}

class Failure<T> extends Result<T> {
  final String message;
  final String? code;
  const Failure(this.message, {this.code});
}

// 使用
Result<AuthToken> validateAndParseToken(String? raw) {
  if (raw == null || raw.isEmpty) {
    return const Failure('Token 为空', code: 'EMPTY_TOKEN');
  }
  if (!raw.startsWith('QL_')) {
    return const Failure('Token 格式无效', code: 'BAD_FORMAT');
  }
  return Success(AuthToken(raw));
}
```

### 4.4 异步错误传播

```dart
/// 始终用上下文包装异步操作。
Future<DeviceInfo> fetchDeviceInfo(String host) async {
  final response = await httpClient
      .get(Uri.parse('$host/api/v2/system/info'))
      .timeout(const Duration(seconds: 5));
  
  if (response.statusCode != 200) {
    throw AuthException(
      '获取设备信息失败: ${response.statusCode}',
      statusCode: response.statusCode,
    );
  }
  
  try {
    return DeviceInfo.fromJson(jsonDecode(response.body) as Map<String, Object?>);
  } on FormatException catch (e) {
    throw AuthException('设备信息响应格式无效: $e');
  }
}
```

### 4.5 Widget 错误处理

```dart
@override
Widget build(BuildContext context, WidgetRef ref) {
  final asyncDevices = ref.watch(deviceListProvider);
  
  return asyncDevices.when(
    data: (devices) => DeviceList(devices: devices),
    loading: () => const Center(child: CircularProgressIndicator()),
    error: (error, stack) => ErrorView(
      title: '连接失败',
      message: _formatErrorMessage(error),
      onRetry: () => ref.invalidate(deviceListProvider),
    ),
  );
}

String _formatErrorMessage(Object error) {
  return switch (error) {
    AuthException e => e.message,
    ConnectionException e => '无法连接到电脑: ${e.message}',
    TimeoutException() => '连接超时，请检查网络',
    _ => '发生未知错误',
  };
}
```

---

## 5. 常量和配置

遵循 [Effective Dart: Usage](https://dart.dev/effective-dart/usage#avoid-littering-the-namespace) — 类级常量使用 `static const`。

### 5.1 集中配置值

```dart
/// 网络和时序配置常量。
/// 
/// 所有超时和持续时间值集中在此，确保应用行为一致。
class AppConfig {
  AppConfig._(); // 防止实例化
  
  // WebSocket
  static const Duration defaultRequestTimeout = Duration(seconds: 10);
  static const Duration defaultHealthInterval = Duration(seconds: 10);
  static const Duration defaultHealthTimeout = Duration(seconds: 5);
  static const Duration maxReconnectBackoff = Duration(seconds: 30);
  static const Duration initialReconnectDelay = Duration(milliseconds: 500);
  static const Duration stableConnectionWindow = Duration(seconds: 10);
  
  // 服务发现
  static const Duration mdnsScanTimeout = Duration(seconds: 3);
  static const Duration mdnsRetryDelay = Duration(seconds: 1);
  
  // WOL
  static const Duration wolCooldown = Duration(seconds: 60);
  static const Duration resumeSuppressionWindow = Duration(seconds: 30);
  
  // 协议
  static const int protocolVersion = 2;
  static const int maxMouseMoveDelta = 1000;
}
```

### 5.2 禁止魔法数字

```dart
// ❌ 错误
await Future.delayed(const Duration(seconds: 5));
if (retries > 3) { ... }

// ✅ 正确
await Future.delayed(AppConfig.defaultHealthTimeout);
if (retries > AppConfig.maxRetries) { ... }
```

---

## 6. 状态管理（Riverpod）

遵循 [Riverpod 最佳实践](https://riverpod.dev/docs/basics/best_practices)。

### 6.1 Provider 声明

```dart
/// 设备列表 provider — 不再被监听时自动释放。
@riverpod
Future<List<Device>> deviceList(DeviceListRef ref) async {
  final discovery = ref.watch(discoveryServiceProvider);
  final devices = await discovery.scan();
  
  // 依赖变化时自动刷新
  ref.onDispose(() {
    _logger.fine('deviceList provider 已释放');
  });
  
  return devices;
}
```

### 6.2 Notifier 模式（带逻辑的状态）

```dart
@riverpod
class AuthNotifier extends _$AuthNotifier {
  WsClient? _client;
  
  @override
  AuthState build() => const AuthState.disconnected();
  
  Future<void> connect(String host, String pin) async {
    if (state is AuthConnecting) return;
    
    state = const AuthState.connecting();
    
    try {
      _client?.dispose();
      _client = WsClient(
        baseUrl: host,
        token: '',
        onLog: (msg) => _logger.fine('[WsClient] $msg'),
      );
      
      final token = await _client!.authenticate(pin);
      state = AuthState.connected(token);
    } on AuthException catch (e) {
      state = AuthState.error(e.message);
      rethrow;
    }
  }
  
  @override
  void dispose() {
    _client?.dispose();
    super.dispose();
  }
}
```

### 6.3 Provider 规则

| 规则 | 原因 |
|------|------|
| 永远不在 `build()` 中创建 provider | 每次重建创建新 provider 实例 |
| 临时状态使用 `autoDispose` | 防止内存泄漏 |
| 使用 `.select()` 做细粒度重建 | 仅特定字段变化时重建 |
| 优先 `Notifier` 而非 `StateNotifier` | 现代 Riverpod API |
| 用 `ref.listenSelf()` 做副作用 | 响应状态变化而不重建 |

### 6.4 Provider 类型决策矩阵

| 使用场景 | Provider 类型 |
|----------|--------------|
| 不可变计算值 | `provider` |
| 异步获取，缓存结果 | `@riverpod autoDispose` |
| 带方法的可变状态 | `@riverpod class Notifier` |
| 事件流 | `StreamProvider` |
| 相关 provider 族 | `@riverpod family` |

---

## 7. Widget 模式

遵循 [Flutter 风格指南](https://github.com/flutter/flutter/wiki/Style-guide-for-Flutter-repo)。

### 7.1 ConsumerWidget

```dart
class DeviceListTile extends ConsumerWidget {
  final Device device;
  
  const DeviceListTile({super.key, required this.device});
  
  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final isConnected = ref.watch(
      connectionProvider.select((s) => s.currentDevice?.id == device.id),
    );
    
    return ListTile(
      title: Text(device.name),
      subtitle: Text(device.host),
      trailing: isConnected 
        ? const Icon(Icons.check_circle, color: Colors.green)
        : const Icon(Icons.circle_outlined),
      onTap: () => ref.read(authNotifierProvider.notifier).connect(
        device.host,
        device.pin,
      ),
    );
  }
}
```

### 7.2 Const 构造函数

```dart
// ✅ 正确 — 始终使用 const
const SizedBox(height: 16)
const Padding(
  padding: EdgeInsets.all(8),
  child: Text('Hello'),
)

// ❌ 错误 — 缺少 const
SizedBox(height: 16)
Padding(
  padding: EdgeInsets.all(8),
  child: Text('Hello'),
)
```

### 7.3 提取 Widget 实现复用

```dart
// ✅ 正确 — 提取为私有 widget 类
class _StatusIndicator extends StatelessWidget {
  final ConnectionStatus status;
  const _StatusIndicator({required this.status});
  
  @override
  Widget build(BuildContext context) {
    return Row(
      children: [
        _buildDot(status.color),
        const SizedBox(width: 8),
        Text(status.label),
      ],
    );
  }
  
  Widget _buildDot(Color color) => Container(
    width: 8,
    height: 8,
    decoration: BoxDecoration(color: color, shape: BoxShape.circle),
  );
}
```

### 7.4 列表项模式

```dart
// ✅ 正确 — 长列表使用 ListView.builder
ListView.builder(
  itemCount: devices.length,
  itemBuilder: (context, index) => DeviceListTile(device: devices[index]),
)

// ✅ 正确 — 短固定列表使用 Column
Column(
  children: devices.map((d) => DeviceListTile(device: d)).toList(),
)
```

---

## 8. 注释

遵循 [Effective Dart: Documentation](https://dart.dev/effective-dart/documentation)。

### 8.1 注释风格

```dart
/// 使用 `///` 文档注释记录公开 API。
/// 
/// 用 [ClassName] 引用其他类型。
/// 用 `code` 引用成员。
/// 
/// 示例：
/// ```dart
/// final client = WsClient(baseUrl: 'http://...');
/// await client.connect();
/// ```
Future<void> connect() async {
  // 使用 `//` 做实现注释
  // FIXME: 在此添加重试逻辑
}
```

### 8.2 规则

- 代码级注释使用英文
- UI 显示字符串使用中文（集中在 `l10n/` 管理）
- 所有公开 API 必须有 `///` 文档注释
- `//` 用于实现注释
- 禁止提交注释掉的代码
- 使用 `// FIXME:` 标记已知问题（不用 `TODO:` 或 `HACK:`）
- 用 `[paramName]` 方式引用参数含义

---

## 9. 测试

遵循 [Flutter 测试指南](https://flutter.dev/docs/testing)。

### 9.1 测试组织

```
test/
  unit/
    models/auth_test.dart
    services/ws_client_test.dart
  widget/
    screens/home_screen_test.dart
    widgets/device_tile_test.dart
  integration/
    pairing_flow_test.dart
  common/
    mocks.dart
    fixtures.dart
```

### 9.2 测试模式

```dart
// 单元测试
void main() {
  group('WsClient', () {
    late WsClient client;
    
    setUp(() {
      client = WsClient(
        baseUrl: 'ws://localhost:8080',
        token: 'test-token',
      );
    });
    
    tearDown(() {
      client.dispose();
    });
    
    test('连接成功时应发出 connected 状态', () async {
      // arrange
      final server = await HttpServer.bind('localhost', 8080);
      // ... 设置 WS 处理器
      
      // act
      await client.connect();
      
      // assert
      expect(client.state, WsConnectionState.connected);
      
      await server.close();
    });
  });
}

// Widget 测试
void main() {
  testWidgets('连接时显示加载指示器', (tester) async {
    await tester.pumpWidget(
      ProviderScope(
        child: MaterialApp(home: const HomeScreen()),
      ),
    );
    
    // 点击连接按钮
    await tester.tap(find.text('连接'));
    await tester.pump();
    
    expect(find.byType(CircularProgressIndicator), findsOneWidget);
  });
}
```

### 9.3 Golden 测试

为复杂 UI 组件添加 golden 文件测试：
```dart
testWidgets('DeviceTile 匹配 golden', (tester) async {
  await tester.pumpWidget(DeviceListTile(device: testDevice));
  await expectLater(
    find.byType(DeviceListTile),
    matchesGoldenFile('goldens/device_tile.png'),
  );
});
```

---

## 10. Analysis Options

扩展 `analysis_options.yaml`：

```yaml
include: package:flutter_lints/flutter.yaml

analyzer:
  exclude:
    - "**/*.g.dart"
    - "**/*.freezed.dart"
  language:
    strict-casts: true
    strict-inference: true
    strict-raw-types: true

linter:
  rules:
    // 错误处理
    avoid_catches_without_on_clauses: true
    avoid_catching_errors: true
    
    // 风格
    prefer_const_constructors: true
    prefer_const_declarations: true
    prefer_const_literals_to_create_immutables: true
    unnecessary_late: true
    unnecessary_null_aware_assignments: true
    unnecessary_parenthesis: true
    
    // 类型
    always_specify_types: false  // Dart 风格：省略局部变量类型
    avoid_dynamic_calls: true
    omit_local_variable_types: true
    
    // 异步
    avoid_returning_null_for_future: true
    unawaited_futures: true
    
    // 公开 API
    public_member_api_docs: true
    lines_longer_than_80_chars: false  // 宽屏使用 100
```
