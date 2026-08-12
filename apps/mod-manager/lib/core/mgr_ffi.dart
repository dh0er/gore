import '../library/domain/models.dart';
import '../preflight/domain/models.dart';
import 'core_service.dart';

/// Typed wrappers over the gore-ffi mod-manager commands (`mgr_*`).
///
/// Every command returns the standard envelope `{ok, ...}` /
/// `{ok: false, error: {code, message}}`; failures throw [MgrFfiException]
/// tagged with the command name.
class MgrFfi {
  MgrFfi(this._core);
  final GoreCoreFfiService _core;

  Future<Map<String, Object?>> _call(
    String cmd,
    Map<String, Object?> payload,
  ) async {
    final r = await _core.execute(cmd, payload: payload);
    if (r['ok'] != true) {
      final e = r['error'];
      if (e is Map) {
        final code = switch (e['code']) {
          final String code when code.isNotEmpty => code,
          _ => 'UNKNOWN',
        };
        throw MgrFfiException('$cmd: ${e['message'] ?? e}', code: code);
      }
      // Non-map error value (string, number, null, ...): stringify it.
      throw MgrFfiException('$cmd: ${e ?? 'unknown error'}');
    }
    return r;
  }

  /// The mod library plus the current loadout order.
  Future<(List<ModEntryMetaView>, LoadoutView)> libraryList() async {
    final r = await _call('mgr_library_list', const {});
    final rawMods = r['mods'];
    final rawLoadout = r['loadout'];
    if (rawMods is! List || rawLoadout is! Map) {
      throw MgrFfiException('mgr_library_list: malformed store snapshot');
    }
    final mods = <ModEntryMetaView>[];
    final modIds = <String>{};
    for (final raw in rawMods) {
      if (raw is! Map ||
          raw['id'] is! String ||
          raw['kind'] is! String ||
          raw['name'] is! String ||
          raw['components'] is! List ||
          (raw['components']! as List).any(
            (component) => component is! Map || component['type'] is! String,
          )) {
        throw MgrFfiException('mgr_library_list: malformed library entry');
      }
      final parsed = raw.cast<String, Object?>();
      final id = parsed['id']! as String;
      if (id.isEmpty || !modIds.add(id)) {
        throw MgrFfiException('mgr_library_list: invalid library id set');
      }
      mods.add(ModEntryMetaView.fromJson(parsed));
    }
    final loadoutMap = rawLoadout.cast<String, Object?>();
    final format = loadoutMap['format'];
    if (format is! int || format != 1 || loadoutMap['entries'] is! List) {
      throw MgrFfiException('mgr_library_list: malformed loadout');
    }
    final entries = loadoutMap['entries']! as List;
    final loadoutIds = <String>{};
    for (final raw in entries) {
      if (raw is! Map ||
          raw['id'] is! String ||
          (raw['id']! as String).isEmpty ||
          raw['enabled'] is! bool ||
          !loadoutIds.add(raw['id']! as String)) {
        throw MgrFfiException('mgr_library_list: malformed loadout entry');
      }
    }
    if (loadoutIds.length != modIds.length || !loadoutIds.containsAll(modIds)) {
      throw MgrFfiException('mgr_library_list: inconsistent store snapshot');
    }
    return (mods, LoadoutView.fromJson(loadoutMap));
  }

  /// Import a mod file/folder into the library; returns its library entry.
  Future<ModEntryMetaView> import(String path) async {
    final r = await _call('mgr_import', {'path': path});
    final entry = r['entry'];
    if (entry is! Map) {
      throw MgrFfiException('mgr_import: response is missing entry');
    }
    return ModEntryMetaView.fromJson(entry.cast<String, Object?>());
  }

  /// Remove a mod from the library. True when an entry was actually removed.
  Future<bool> remove(String id) async {
    final r = await _call('mgr_remove', {'id': id});
    return _truthy(r['removed']);
  }

  /// Persist the loadout (order + enabled flags).
  Future<void> setLoadout(LoadoutView loadout) =>
      _call('mgr_set_loadout', {'loadout': loadout.toJson()});

  /// Conflicts across the enabled mods of the current loadout.
  Future<List<ConflictView>> analyze() async {
    final r = await _call('mgr_analyze', const {});
    return [for (final m in _maps(r['conflicts'])) ConflictView.fromJson(m)];
  }

  /// Declaratively apply the current loadout to the game install.
  Future<ApplyReportView> apply(String gameRoot) async {
    final r = await _call('mgr_apply', {'game_root': gameRoot});
    final report = r['report'];
    return report is Map
        ? ApplyReportView.fromJson(report.cast<String, Object?>())
        : const ApplyReportView();
  }

  /// Deployment status of the install relative to the current loadout.
  Future<ManagerStatusView> status(String gameRoot) async {
    final r = await _call('mgr_status', {'game_root': gameRoot});
    final status = r['status'];
    return ManagerStatusView.fromJson(
      status is Map ? status.cast<String, Object?>() : const {},
    );
  }

  /// Read-only, bounded setup/deployment evidence for one explicit game root.
  Future<ManagerPreflightView> preflight(String gameRoot) async {
    final r = await _call('mgr_preflight_v1', {'game_root': gameRoot});
    try {
      return ManagerPreflightView.fromJson(r['preflight']);
    } on FormatException catch (error) {
      throw MgrFfiException(
        'mgr_preflight_v1: invalid response: ${error.message}',
        code: 'MGR_PREFLIGHT_INVALID_RESPONSE',
      );
    }
  }

  /// Remove everything the manager deployed from the install. True when
  /// anything was actually removed.
  Future<bool> undeployAll(String gameRoot) async {
    final r = await _call('mgr_undeploy_all', {'game_root': gameRoot});
    return _truthy(r['removed']);
  }
}

/// True for `true` and for positive counts — tolerates the Rust side
/// reporting `removed` as either a bool or a count.
bool _truthy(Object? value) => value == true || (value is num && value > 0);

/// Non-throwing list-of-maps accessor for response arrays.
List<Map<String, Object?>> _maps(Object? value) => value is List
    ? [for (final item in value.whereType<Map>()) item.cast<String, Object?>()]
    : const [];

class MgrFfiException implements Exception {
  MgrFfiException(this.message, {this.code = 'UNKNOWN'});

  final String message;

  /// Machine-readable code from the FFI error envelope (`error.code`);
  /// 'UNKNOWN' when absent. The UI branches on codes such as
  /// STUDIO_DEPLOY_ACTIVE.
  final String code;

  @override
  String toString() => message;
}
