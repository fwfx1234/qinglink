/// 「库」分区的筛选枚举与展示标签。
library;

/// 「库」的分区。
///
/// 只保留用户真正会用的分组：PC 上的目录按「软件 / 游戏」两分就够了，之前
/// 的 Steam / 其他 两个分区只是把同一批程序拆得更碎，反而更难找。
enum LibraryFilter { common, recent, software, games, blocked }

/// [LibraryFilter] 的服务端分类与界面标签。
extension LibraryFilterInfo on LibraryFilter {
  /// 服务端统一库对应的分类；常用/最近/已屏蔽在本地或专用接口处理。
  String? get serverKind => switch (this) {
    LibraryFilter.software => 'app',
    LibraryFilter.games => 'game',
    _ => null,
  };

  /// 筛选标签上显示的中文名。
  String get label => switch (this) {
    LibraryFilter.common => '常用',
    LibraryFilter.recent => '最近',
    LibraryFilter.software => '软件',
    LibraryFilter.games => '游戏',
    LibraryFilter.blocked => '已屏蔽',
  };
}
