part of 'remote_control_provider.dart';

/// 进程列表、任务与进程操作相关动作。
mixin _ProcessActions on _RemoteControlBase {
  // ========== Processes ==========

  @override
  Future<void> refreshProcesses({
    String? search,
    int page = 0,
    bool append = false,
    String? filter,
  }) async {
    final effectiveFilter = filter ?? state.processFilter;
    _setState(state.copyWith(isLoading: true));
    try {
      final resp = await _wsRequest({
        'type': 'get_processes',
        if (search != null && search.isNotEmpty) 'search': search,
        'page': page,
        'page_size': 30,
        'filter': effectiveFilter,
        'sort': 'active',
      });
      if (resp['type'] == 'process_list') {
        final data = resp['data'] as Map<String, dynamic>;
        final list = (data['processes'] as List)
            .map((e) => ProcessInfo.fromJson(e as Map<String, dynamic>))
            .toList();
        _setState(
          state.copyWith(
            processes: append ? [...state.processes, ...list] : list,
            processHasMore: data['has_more'] as bool? ?? false,
            processPage: page,
            processQuery: search ?? '',
            processFilter: effectiveFilter,
            isLoading: false,
          ),
        );
      } else {
        _setState(state.copyWith(isLoading: false));
      }
    } catch (e) {
      _setState(state.copyWith(isLoading: false, errorMessage: e.toString()));
    }
  }

  Future<void> loadMoreProcesses() async {
    if (state.isLoading || !state.processHasMore) return;
    await refreshProcesses(
      search: state.processQuery.isEmpty ? null : state.processQuery,
      page: state.processPage + 1,
      append: true,
    );
  }

  // ========== Process Commands ==========

  Future<void> killProcess(int pid) async {
    await _wsCommand({'type': 'kill_process', 'pid': pid});
    await refreshProcesses(
      search: state.processQuery.isEmpty ? null : state.processQuery,
    );
  }

  // ========== Tasks ==========

  Future<void> refreshTasks({String? search, int page = 0}) async {
    _setState(state.copyWith(isLoading: true));
    try {
      final resp = await _wsRequest({
        'type': 'get_tasks',
        if (search != null && search.isNotEmpty) 'search': search,
        'page': page,
        'page_size': 200,
      });
      if (resp['type'] == 'tasks') {
        final data = resp['data'] as Map<String, dynamic>;
        final list = (data['processes'] as List)
            .map((e) => ProcessInfo.fromJson(e as Map<String, dynamic>))
            .toList();
        _setState(state.copyWith(processes: list, isLoading: false));
      } else {
        _setState(state.copyWith(isLoading: false));
      }
    } catch (e) {
      _setState(state.copyWith(isLoading: false, errorMessage: e.toString()));
    }
  }

  Future<void> setTaskPriority(int pid, String priority) async {
    await _wsCommand({
      'type': 'set_task_priority',
      'pid': pid,
      'priority': priority,
    });
    await refreshTasks();
  }
}
