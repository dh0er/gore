import 'dart:convert';
import 'dart:ffi';

import 'package:ffi/ffi.dart';
import 'package:goresave/features/editor/domain/core_service.dart';

typedef _Execute = Pointer<Utf8> Function(Pointer<Utf8>);
typedef _FreeNative = Void Function(Pointer<Utf8>);
typedef _Free = void Function(Pointer<Utf8>);

/// Widget fixtures keep their synthetic save reads/writes, while the pure edit
/// planner is the real shared Rust implementation. Planning is synchronous in
/// this test adapter so pumpAndSettle observes the same save-loading transition
/// without waiting for an external worker under Flutter's simulated clock.
GoresaveCoreService withSharedPlanner(GoresaveCoreService fixture) =>
    _PlanningFixture(fixture);

class _PlanningFixture implements GoresaveCoreService {
  _PlanningFixture(this.fixture);
  final GoresaveCoreService fixture;

  @override
  bool get isAvailable => fixture.isAvailable;
  @override
  String get description => fixture.description;

  @override
  Future<Map<String, Object?>> execute(
    String command, {
    Map<String, Object?> payload = const {},
  }) {
    if (command != 'plan_edits') {
      return fixture.execute(command, payload: payload);
    }
    final native = NativeGoresaveCoreService.tryCreate();
    if (native == null) {
      throw StateError('Build gore-save before running shared planner tests');
    }
    final library = DynamicLibrary.open(native.description);
    final execute = library.lookupFunction<_Execute, _Execute>(
      'goresave_execute',
    );
    final free = library.lookupFunction<_FreeNative, _Free>('goresave_free');
    final request = jsonEncode({
      'command': command,
      'payload': payload,
    }).toNativeUtf8();
    Pointer<Utf8>? response;
    try {
      response = execute(request);
      return Future.value(
        (jsonDecode(response.toDartString()) as Map).cast<String, Object?>(),
      );
    } finally {
      malloc.free(request);
      if (response != null) free(response);
    }
  }
}
