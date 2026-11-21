import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../models/process_info.dart';
import '../providers/auth_provider.dart';
import '../providers/process_controller.dart';
import '../providers/process_icon_controller.dart';
import '../theme/app_theme.dart';
import '../utils/app_dialogs.dart';
import '../widgets/app_ui.dart';
import '../widgets/process_icon.dart';
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
    // 显式刷新时放掉上一次解析失败的记录，让这些行重新试一次。
    ref.read(processIconControllerProvider.notifier).clearFailures();
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

  /// 搜索输入变化：更新查询词，并以 300ms 防抖刷新进程列表。
  void _onQueryChanged(String value) {
    _query = value.trim();
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
  }

  /// 清空搜索框：重置查询词并立即刷新进程列表。
  void _clearQuery() {
    _searchController.clear();
    _query = '';
    setState(() {});
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
          _ProcessSearchBar(
            controller: _searchController,
            onChanged: _onQueryChanged,
            onClear: _clearQuery,
          ),
          const SizedBox(height: 10),
          _ProcessFilterBar(filter: _filter, onChanged: _setFilter),
          const SizedBox(height: 12),
          Expanded(
            child: _ProcessList(
              processes: processes,
              isLoading: isLoading,
              hasMore: processHasMore,
              query: _query,
              scrollController: _scrollController,
              onRefresh: _reload,
            ),
          ),
        ],
      ),
    );
  }
}

/// 进程页搜索框：输入交给外部防抖处理，清空立即触发刷新。
class _ProcessSearchBar extends StatelessWidget {
  const _ProcessSearchBar({
    required this.controller,
    required this.onChanged,
    required this.onClear,
  });

  final TextEditingController controller;
  final ValueChanged<String> onChanged;
  final VoidCallback onClear;

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.symmetric(horizontal: AppMetrics.pagePadding),
      child: SearchField(
        controller: controller,
        hint: '搜索进程或 PID',
        onChanged: onChanged,
        onClear: onClear,
      ),
    );
  }
}

/// 进程页筛选条：用户进程 / 全部进程。
class _ProcessFilterBar extends StatelessWidget {
  const _ProcessFilterBar({required this.filter, required this.onChanged});

  final String filter;
  final ValueChanged<String> onChanged;

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.symmetric(horizontal: AppMetrics.pagePadding),
      child: Row(
        children: [
          FilterChipX(
            label: '用户',
            active: filter == 'user',
            onTap: () => onChanged('user'),
          ),
          const SizedBox(width: AppSpacing.sm),
          FilterChipX(
            label: '全部',
            active: filter == 'all',
            onTap: () => onChanged('all'),
          ),
        ],
      ),
    );
  }
}

/// 进程列表区：负责分组、加载态、空态与上拉加载占位。
class _ProcessList extends StatelessWidget {
  const _ProcessList({
    required this.processes,
    required this.isLoading,
    required this.hasMore,
    required this.query,
    required this.scrollController,
    required this.onRefresh,
  });

  final List<ProcessInfo> processes;
  final bool isLoading;
  final bool hasMore;
  final String query;
  final ScrollController scrollController;
  final VoidCallback onRefresh;

  /// 按 group 切成连续的分组块，每块单独渲染成一张卡片（设计稿 S6）。
  static List<_ProcessGroup> _groupedBlocks(List<ProcessInfo> processes) {
    // 与 PC 任务管理器一致：应用 / 后台进程 / Windows 进程。
    final blocks = <_ProcessGroup>[];
    for (final process in processes) {
      final group = process.group ?? 'background';
      if (blocks.isEmpty || blocks.last.key != group) {
        blocks.add(
          _ProcessGroup(key: group, label: _groupLabel(group), items: [process]),
        );
      } else {
        blocks.last.items.add(process);
      }
    }
    return blocks;
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

  @override
  Widget build(BuildContext context) {
    final blocks = _groupedBlocks(processes);
    return Column(
      children: [
        if (isLoading && processes.isNotEmpty)
          LinearProgressIndicator(
            minHeight: 2,
            color: AppColors.primary,
            backgroundColor: Colors.transparent,
          ),
        Expanded(
          child: isLoading && processes.isEmpty
              ? const Center(
                  child: CircularProgressIndicator(),
                )
              : processes.isEmpty
              ? EmptyState(
                  icon: Icons.check_circle_outline_rounded,
                  title: '没有找到进程',
                  message: query.isEmpty ? null : '试试其他名称或 PID。',
                )
              : RefreshIndicator(
                  onRefresh: () async => onRefresh(),
                  color: AppColors.primary,
                  child: ListView.builder(
                    controller: scrollController,
                    padding: const EdgeInsets.fromLTRB(
                      AppMetrics.pagePadding,
                      0,
                      AppMetrics.pagePadding,
                      AppSpacing.xl,
                    ),
                    itemCount: blocks.length + (hasMore ? 1 : 0),
                    itemBuilder: (context, index) => index >= blocks.length
                        ? const Center(
                            child: Padding(
                              padding: EdgeInsets.all(AppSpacing.lg),
                              child: CircularProgressIndicator(),
                            ),
                          )
                        : _ProcessGroupBlock(group: blocks[index]),
                  ),
                ),
        ),
      ],
    );
  }
}

/// 一个连续的进程分组（标题 + 该组的进程）。
class _ProcessGroup {
  _ProcessGroup({
    required this.key,
    required this.label,
    required List<ProcessInfo> items,
  }) : items = <ProcessInfo>[...items];

  /// 服务端给的分组标识（app / background / system）。
  final String key;
  final String label;
  final List<ProcessInfo> items;
}

/// 分组块：标题在上，进程行收在一张整卡里，行间用分隔线。
class _ProcessGroupBlock extends ConsumerWidget {
  const _ProcessGroupBlock({required this.group});

  final _ProcessGroup group;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final palette = context.palette;
    return Padding(
      padding: const EdgeInsets.only(bottom: AppSpacing.lg),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          _GroupHeader(title: group.label),
          AppCard(
            radius: AppRadius.md,
            padding: EdgeInsets.zero,
            child: ClipRRect(
              // 卡片描边是直角画在外层，圆角在这里裁，避免 decoration
              // 同时带 border + borderRadius 时整块内容不绘制的问题。
              borderRadius: BorderRadius.circular(AppRadius.md),
              child: Column(
                children: [
                  for (var i = 0; i < group.items.length; i++) ...[
                    if (i > 0) Divider(height: 1, color: palette.line),
                    _ProcessItem(
                      process: group.items[i],
                      onKill: () => confirmKillProcess(
                        context,
                        ref,
                        pid: group.items[i].pid,
                        name: group.items[i].name,
                      ),
                    ),
                  ],
                ],
              ),
            ),
          ),
        ],
      ),
    );
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
        style: AppText.label.copyWith(color: palette.textMuted),
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
    return Container(
      constraints: const BoxConstraints(minHeight: AppMetrics.listRow),
      padding: const EdgeInsets.symmetric(
        horizontal: AppSpacing.md,
        vertical: AppSpacing.sm,
      ),
      child: Row(
        children: [
          ProcessIcon(
            path: process.path,
            pid: process.pid,
            size: 40,
            radius: AppRadius.sm,
            iconSize: 20,
            color: palette.textMuted,
          ),
          const SizedBox(width: 12),
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              mainAxisSize: MainAxisSize.min,
              children: [
                Text(
                  process.displayLabel,
                  maxLines: 1,
                  overflow: TextOverflow.ellipsis,
                  style: AppText.rowTitle.copyWith(color: palette.textPrimary),
                ),
                const SizedBox(height: 3),
                Text(
                  process.hasFriendlyLabel
                      ? '${process.name} · PID ${process.pid} · ${process.memoryText}'
                      : 'PID ${process.pid} · ${process.memoryText}',
                  maxLines: 1,
                  overflow: TextOverflow.ellipsis,
                  style: AppText.meta.copyWith(color: palette.textMuted),
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
              radius: AppRadius.sm,
              background: palette.surface2,
              color: palette.textMuted,
              onPressed: onKill,
            ),
        ],
      ),
    );
  }
}
