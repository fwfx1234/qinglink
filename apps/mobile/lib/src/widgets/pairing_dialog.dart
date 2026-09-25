import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../providers/auth_provider.dart';
import '../theme/app_theme.dart';

class PairingDialog extends ConsumerStatefulWidget {
  final BuildContext dialogContext;

  const PairingDialog({super.key, required this.dialogContext});

  @override
  ConsumerState<PairingDialog> createState() => _PairingDialogState();
}

class _PairingDialogState extends ConsumerState<PairingDialog> {
  final _pinController = TextEditingController();
  bool _popScheduled = false;

  @override
  void dispose() {
    _pinController.dispose();
    super.dispose();
  }

  void _safePop(bool result) {
    if (_popScheduled) return;
    _popScheduled = true;
    WidgetsBinding.instance.addPostFrameCallback((_) {
      if (mounted && Navigator.canPop(widget.dialogContext)) {
        FocusManager.instance.primaryFocus?.unfocus();
        Navigator.pop(widget.dialogContext, result);
      }
    });
  }

  @override
  Widget build(BuildContext context) {
    final authState = ref.watch(authProvider);
    final theme = Theme.of(context);

    // 配对成功自动关闭
    if (authState.isAuthenticated) {
      _safePop(true);
    }

    final pin = _pinController.text.trim();
    final isPinValid = pin.length == 6 && RegExp(r'^\d{6}$').hasMatch(pin);
    final isPairing = authState.isAuthenticating;

    return AlertDialog(
      shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(24)),
      title: Row(
        children: [
          Icon(
            isPairing ? Icons.sync_rounded : Icons.lock_open_rounded,
            color: AppColors.primary,
          ),
          const SizedBox(width: 12),
          Text(isPairing ? '配对中...' : '配对连接'),
        ],
      ),
      content: Column(
        mainAxisSize: MainAxisSize.min,
        children: [
          if (!isPairing) ...[
            Text(
              '请输入 QingLink 电脑上显示的 6 位匹配码',
              style: theme.textTheme.bodyMedium?.copyWith(
                color: theme.colorScheme.onSurface.withValues(alpha: 0.6),
              ),
            ),
            const SizedBox(height: 8),
            Text(
              '匹配码有效期 60 秒；关闭电脑上的配对提示即可取消',
              style: theme.textTheme.bodySmall?.copyWith(
                color: theme.colorScheme.onSurface.withValues(alpha: 0.45),
              ),
            ),
            const SizedBox(height: 16),
            TextField(
              controller: _pinController,
              keyboardType: TextInputType.number,
              textAlign: TextAlign.center,
              maxLength: 6,
              style: const TextStyle(
                fontSize: 28,
                letterSpacing: 8,
                fontWeight: FontWeight.bold,
              ),
              decoration: InputDecoration(
                hintText: '000000',
                border: OutlineInputBorder(
                  borderRadius: BorderRadius.circular(16),
                ),
                counterText: '',
              ),
              onChanged: (_) => setState(() {}),
            ),
          ] else ...[
            const SizedBox(height: 8),
            const SizedBox(
              width: 36,
              height: 36,
              child: CircularProgressIndicator(strokeWidth: 3),
            ),
            const SizedBox(height: 16),
            Text(
              '正在配对...',
              style: theme.textTheme.bodyMedium?.copyWith(
                color: theme.colorScheme.onSurface.withValues(alpha: 0.6),
              ),
            ),
            const SizedBox(height: 8),
          ],
          if (authState.errorMessage != null && !isPairing) ...[
            const SizedBox(height: 12),
            Text(
              authState.errorMessage!,
              style: TextStyle(color: theme.colorScheme.error, fontSize: 13),
            ),
          ],
        ],
      ),
      actions: [
        TextButton(
          onPressed: isPairing
              ? null
              : () {
                  ref.read(authProvider.notifier).cancelPairing();
                  _safePop(false);
                },
          child: const Text('取消'),
        ),
        FilledButton(
          onPressed: isPairing || !isPinValid
              ? null
              : () {
                  ref
                      .read(authProvider.notifier)
                      .pair(_pinController.text.trim());
                },
          style: FilledButton.styleFrom(backgroundColor: AppColors.primary),
          child: isPairing
              ? const SizedBox(
                  width: 18,
                  height: 18,
                  child: CircularProgressIndicator(
                    strokeWidth: 2,
                    color: Colors.white,
                  ),
                )
              : const Text('确认'),
        ),
      ],
    );
  }
}
