import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../models/process_info.dart';
import 'remote_command_bus.dart';

class ProcessState {
  const ProcessState({this.items = const [], this.hasMore = false, this.page = 0, this.query = '', this.filter = 'user', this.loading = false, this.error});
  final List<ProcessInfo> items;
  final bool hasMore;
  final int page;
  final String query;
  final String filter;
  final bool loading;
  final String? error;
  ProcessState copyWith({List<ProcessInfo>? items, bool? hasMore, int? page, String? query, String? filter, bool? loading, Object? error = _unset}) => ProcessState(items: items ?? this.items, hasMore: hasMore ?? this.hasMore, page: page ?? this.page, query: query ?? this.query, filter: filter ?? this.filter, loading: loading ?? this.loading, error: identical(error, _unset) ? this.error : error as String?);

  static const _unset = Object();
}

class ProcessController extends StateNotifier<ProcessState> {
  ProcessController(this._bus) : super(const ProcessState());
  final RemoteCommandBus _bus;

  Future<void> refresh({String? search, String? filter, int page = 0, bool append = false}) async {
    final effectiveFilter = filter ?? state.filter;
    state = state.copyWith(loading: true, error: null);
    try {
      final response = await _bus.request({'type': 'get_processes', if (search != null && search.isNotEmpty) 'search': search, 'page': page, 'page_size': 30, 'filter': effectiveFilter, 'sort': 'active'});
      if (response['type'] != 'process_list') return;
      final data = response['data'] as Map<String, dynamic>;
      final items = (data['processes'] as List).map((e) => ProcessInfo.fromJson(e as Map<String, dynamic>)).toList(growable: false);
      state = state.copyWith(items: append ? [...state.items, ...items] : items, hasMore: data['has_more'] as bool? ?? false, page: page, query: search ?? '', filter: effectiveFilter, loading: false);
    } catch (error) {
      state = state.copyWith(loading: false, error: error.toString());
    }
  }

  Future<void> loadMore() async {
    if (state.loading || !state.hasMore) return;
    await refresh(search: state.query.isEmpty ? null : state.query, page: state.page + 1, append: true);
  }

  Future<void> kill(int pid) async {
    await _bus.request({'type': 'kill_process', 'pid': pid});
    await refresh(search: state.query.isEmpty ? null : state.query, filter: state.filter);
  }
}

final processControllerProvider = StateNotifierProvider<ProcessController, ProcessState>((ref) => ProcessController(ref.watch(remoteCommandBusProvider)));
