import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'core_service.dart';
import 'mgr_ffi.dart';

/// Overridden in main() with the real or stub service.
final coreServiceProvider = Provider<GoreCoreFfiService>(
  (ref) => throw UnimplementedError('coreServiceProvider must be overridden'),
);

/// Typed mod-manager FFI bridge over [coreServiceProvider].
final mgrFfiProvider = Provider<MgrFfi>(
  (ref) => MgrFfi(ref.watch(coreServiceProvider)),
);
