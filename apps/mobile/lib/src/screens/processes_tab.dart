import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../models/process_info.dart';
import '../providers/auth_provider.dart';
import '../providers/process_controller.dart';
import '../theme/app_theme.dart';
import '../utils/app_dialogs.dart';
import '../widgets/app_ui.dart';
import '../widgets/unconnected_state.dart';

class ProcessesTab extends ConsumerStatefulWidget {
  const ProcessesTab({super.key});

  @override
  ConsumerState<ProcessesTab> createState() => _ProcessesTabState();
}

class _ProcessesTabState extends ConsumerState<ProcessesTab>
    with AutomaticKeepAliveClientMixin {
  final _searchController = TextEditingController();
  final _scrollController = ScrollController();
  Timer? _searchDebounce;
  String _query = '';
  String _filter = 'user';

  @override
  bool get wantKeepAlive => true;

  @override
  void initState() {
    super.initState();
    _scrollController.addListener(() {
      if (_scrollController.hasClients &&
          _scrollController.position.extentAfter < 240) {
        ref.read(processControllerProvider.notifier).loadMore();
      }
    });
    Future.delayed(const Duration(milliseconds: 150), () {
      if (mounted) {
        ref.read(processControllerProvider.notifier).refresh(filter: _filter);
      }
    });
  }

  @override
  void dispose() {
    _searchController.dispose();
    _searchDebounce?.cancel();
    _scrollController.dispose();
    super.dispose();
  }

  void _reload() {
    ref
        .read(processControllerProvider.notifier)
        .refresh(
          search: _query.isEmpty ? null : _query,
          filter: _filter,
        );
  }

  void _setFilter(String filter) {
    if (_filter == filter) return;
    setState(() => _filter = filter);
    _reload();
  }

  @override
  Widget build(BuildContext context) {
    super.build(context);
    final isAuthenticated = ref.watch(
      connectionProvider.select((state) => state.isAuthenticated),
    );
    final processState = ref.watch(processControllerProvider);
    final processes = processState.items;
    final isLoading = processState.loading;
    final processHasMore = processState.hasMore;

    if (!isAuthenticated) {
      return const UnconnectedState();
    }

    // 与 PC 任务管理器一致：应用 / 后台进程 / Windows 进程。
    final rows = <Object>[];
    String? currentGroup;
    for (final process in processes) {
      final group = process.group ?? 'background';
      if (group != currentGroup) {
        currentGroup = group;
        rows.add(_groupLabel(group));
      }
      rows.add(process);
    }

    return Scaffold(
      appBar: AppBar(
        title: const Text('进程管理'),
        actions: [
          IconBtn(
            icon: Icons.refresh_rounded,
            tooltip: '刷新',
            onPressed: _reload,
          ),
          const SizedBox(width: 8),
        ],
      ),
      body: Column(
        children: [
          const SizedBox(height: 10),
          Padding(
            padding: const EdgeInsets.symmetric(horizontal: 18),
            child: SearchField(
            controller: _searchController,
            hint: '搜索进程或 PID',
            onChanged: (v) {
              _query = v.trim();
              setState(() {});
              _searchDebounce?.cancel();
              _searchDebounce = Timer(const Duration(milliseconds: 300), () {
                ref
                    .read(processControllerProvider.notifier)
                    .refresh(
                      search: _query.isEmpty ? null : _query,
                      filter: _filter,
                    );
              });
            },
            onClear: () {
              _searchController.clear();
              _query = '';
              setState(() {});
              _reload();
            },
          ),
        ),
        const SizedBox(height: 10),
        Padding(
          padding: const EdgeInsets.symmetric(horizontal: 18),
          child: Row(
            children: [
              FilterChipX(
                label: '用户',
                active: _filter == 'user',
                onTap: () => _setFilter('user'),
              ),
              const SizedBox(width: 7),
              FilterChipX(
                label: '全部',
                active: _filter == 'all',
                onTap: () => _setFilter('all'),
              ),
            ],
          ),
        ),
        const SizedBox(height: 12),
        Expanded(
          child: Column(
            children: [
              if (isLoading && processes.isNotEmpty)
                const LinearProgressIndicator(
                  minHeight: 2,
                  color: AppColors.primary,
                ),
              Expanded(
                child: isLoading && processes.isEmpty
                    ? const Center(
                        child: CircularProgressIndicator(
                          color: AppColors.primary,
                        ),
                      )
                    : processes.isEmpty
                    ? EmptyState(
                        icon: Icons.check_circle_outline_rounded,
                        title: '没有找到进程',
                        message: _query.isEmpty ? null : '试试其他名称或 PID。',
                      )
                    : RefreshIndicator(
                        onRefresh: () async => _reload(),
                        color: AppColors.primary,
                        child: ListView.builder(
                          controller: _scrollController,
                          padding: const EdgeInsets.fromLTRB(18, 0, 18, 24),
                          itemCount: rows.length + (processHasMore ? 1 : 0),
                          itemBuilder: (context, index) {
                            if (index >= rows.length) {
                              return const Center(
                                child: Padding(
                                  padding: EdgeInsets.all(16),
                                  child: CircularProgressIndicator(),
                                ),
                              );
                            }
                            final row = rows[index];
                            if (row is String) {
                              return _GroupHeader(title: row);
                            }
                            final process = row as ProcessInfo;
                            return Padding(
                              padding: const EdgeInsets.only(bottom: 6),
                              child: _ProcessItem(
                                process: process,
                                onKill: () => confirmKillProcess(
                                  context,
                                  ref,
                                  pid: process.pid,
                                  name: process.name,
                                ),
                              ),
                            );
                          },
                        ),
                      ),
              ),
            ],
          ),
        ),
      ],
      ),
    );
  }

  static String _groupLabel(String group) {
    switch (group) {
      case 'app':
        return '应用';
      case 'system':
        return 'Windows 进程';
      default:
        return '后台进程';
    }
  }
}

class _GroupHeader extends StatelessWidget {
  const _GroupHeader({required this.title});
  final String title;

  @override
  Widget build(BuildContext context) {
    final palette = context.palette;
    return Padding(
      padding: const EdgeInsets.only(top: 10, bottom: 8),
      child: Text(
        title,
        style: AppText.label.copyWith(
          fontWeight: FontWeight.w700,
          color: palette.textSubtle,
        ),
      ),
    );
  }
}

class _ProcessItem extends StatelessWidget {
  const _ProcessItem({required this.process, required this.onKill});

  final ProcessInfo process;
  final VoidCallback onKill;

  @override
  Widget build(BuildContext context) {
    final palette = context.palette;
    final isSystem = process.group == 'system' || process.pid <= 4;
    return AppCard(
      minHeight: 60,
      radius: AppRadius.md,
      padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 8),
      child: Row(
        children: [
          TileIcon(
            icon: Icons.memory_rounded,
            color: palette.textMuted,
            size: 32,
            radius: 9,
            iconSize: 16,
            alpha: 0,
            filled: false,
          ),
          const SizedBox(width: 8),
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              mainAxisSize: MainAxisSize.min,
              children: [
                Text(
                  process.displayLabel,
                  maxLines: 1,
                  overflow: TextOverflow.ellipsis,
                  style: AppText.body.copyWith(
                    fontWeight: FontWeight.w700,
                    color: palette.textPrimary,
                  ),
                ),
                const SizedBox(height: 3),
                Text(
                  process.hasFriendlyLabel
                      ? '${process.name} · PID ${process.pid} · ${process.memoryText}'
                      : 'PID ${process.pid} · ${process.memoryText}',
                  maxLines: 1,
                  overflow: TextOverflow.ellipsis,
                  style: AppText.meta.copyWith(color: palette.textSubtle),
                ),
              ],
            ),
          ),
          if (isSystem)
            const StatusTag('系统')
          else
            IconBtn(
              icon: Icons.stop_rounded,
              tooltip: '结束进程',
              size: 36,
              iconSize: 16,
              radius: 8,
              color: AppColors.red,
              onPressed: onKill,
            ),
        ],
      ),
    );
  }
}
