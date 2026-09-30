import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../models/app_info.dart';
import 'remote_command_bus.dart';

class LibraryState {
  const LibraryState({this.items = const [], this.recent = const [], this.kind = 'all', this.query = '', this.hasMore = false, this.page = 0, this.loading = false, this.loadingMore = false, this.error});
  final List<LauncherItem> items;
  final List<LauncherItem> recent;
  final String kind;
  final String query;
  final bool hasMore;
  final int page;
  final bool loading;
  final bool loadingMore;
  final String? error;
  LibraryState copyWith({List<LauncherItem>? items, List<LauncherItem>? recent, String? kind, String? query, bool? hasMore, int? page, bool? loading, bool? loadingMore, Object? error = _unset}) => LibraryState(items: items ?? this.items, recent: recent ?? this.recent, kind: kind ?? this.kind, query: query ?? this.query, hasMore: hasMore ?? this.hasMore, page: page ?? this.page, loading: loading ?? this.loading, loadingMore: loadingMore ?? this.loadingMore, error: identical(error, _unset) ? this.error : error as String?);

  static const _unset = Object();
}

class LibraryController extends StateNotifier<LibraryState> {
  LibraryController(this._bus) : super(const LibraryState());
  final RemoteCommandBus _bus;

  Future<bool> refresh({String? kind, String? query, bool append = false}) async {
    final selectedKind = kind ?? state.kind;
    final selectedQuery = query ?? (append ? state.query : '');
    state = state.copyWith(loading: !append, loadingMore: append, kind: selectedKind, query: selectedQuery, error: null);
    try {
      final response = await _bus.request({'type': 'list_launcher_items', 'kind': selectedKind, if (selectedQuery.trim().isNotEmpty) 'query': selectedQuery.trim(), 'page': append ? state.page + 1 : 0, 'page_size': 50});
      if (response['type'] != 'launcher_items') return false;
      final page = LauncherItemPage.fromJson(response['data'] as Map<String, dynamic>);
      state = state.copyWith(items: append ? [...state.items, ...page.items] : page.items, hasMore: page.hasMore, page: page.page, loading: false, loadingMore: false);
      return true;
    } catch (error) {
      state = state.copyWith(loading: false, loadingMore: false, error: error.toString());
      return false;
    }
  }

  Future<bool> refreshRecent() async {
    try {
      final response = await _bus.request({'type': 'get_recent_launcher_items', 'kind': 'all'});
      if (response['type'] != 'recent_launcher_items' || response['data'] is! List) return false;
      final recent = (response['data'] as List).whereType<Map>().map((e) => LauncherItem.fromJson(Map<String, dynamic>.from(e))).toList(growable: false);
      state = state.copyWith(recent: recent);
      return true;
    } catch (_) {
      return false;
    }
  }
}

final libraryControllerProvider = StateNotifierProvider<LibraryController, LibraryState>((ref) => LibraryController(ref.watch(remoteCommandBusProvider)));
