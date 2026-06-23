import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'core_service.dart';

/// Overridden in main() with the real or stub service.
final coreServiceProvider = Provider<GoreCoreFfiService>(
  (ref) => throw UnimplementedError('coreServiceProvider must be overridden'),
);
