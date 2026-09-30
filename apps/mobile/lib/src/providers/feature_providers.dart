import 'package:flutter_riverpod/flutter_riverpod.dart';

import 'auth_provider.dart';
import 'system_controller.dart';
import 'process_controller.dart';
import 'library_controller.dart';

/// Fine-grained feature projections used by pages and widgets.
final connectionSliceProvider = Provider((ref) => ref.watch(connectionProvider));

final systemSliceProvider = Provider((ref) => ref.watch(systemControllerProvider));

final processSliceProvider = Provider((ref) => ref.watch(processControllerProvider));

final librarySliceProvider = Provider((ref) => ref.watch(libraryControllerProvider));
