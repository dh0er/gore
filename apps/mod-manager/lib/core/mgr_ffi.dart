import '../library/domain/models.dart';
import '../preflight/domain/models.dart';
import 'core_service.dart';

const _maxImportErrorCandidates = 2;
const _maxImportCandidateRunes = 256;

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
        throw MgrFfiException(
          '$cmd: ${e['message'] ?? e}',
          code: code,
          details: _mgrErrorDetails(code, e['details']),
        );
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
      final parsed = _libraryEntry(raw, command: 'mgr_library_list');
      final id = parsed.id;
      if (id.isEmpty || !modIds.add(id)) {
        throw MgrFfiException('mgr_library_list: invalid library id set');
      }
      mods.add(parsed);
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

  /// Import a mod file/folder into the library and retain Native's exact
  /// disposition and verified match method.
  Future<MgrImportOutcome> import(String path) async {
    final r = await _call('mgr_import', {'path': path});
    final entry = r['entry'];
    final disposition = MgrImportDisposition.fromWire(r['disposition']);
    final matchedBy = MgrImportMatchedBy.fromWire(r['matched_by']);
    if (disposition == null ||
        matchedBy == null ||
        !_validImportOutcome(disposition, matchedBy)) {
      throw MgrFfiException(
        'mgr_import: malformed import outcome',
        code: 'IMPORT_INVALID_RESPONSE',
      );
    }
    return MgrImportOutcome(
      entry: _libraryEntry(
        entry,
        command: 'mgr_import',
        errorCode: 'IMPORT_INVALID_RESPONSE',
      ),
      disposition: disposition,
      matchedBy: matchedBy,
    );
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

  /// Recover one exact interrupted Manager install mutation. Native rechecks
  /// both the selected root and the opaque guard id before changing anything.
  Future<MgrInstallRecoveryOutcome> recoverInstall(
    String gameRoot,
    String expectedGuardId,
  ) async {
    final r = await _call('mgr_recover_install_v1', {
      'game_root': gameRoot,
      'expected_guard_id': expectedGuardId,
    });
    final outcome = MgrInstallRecoveryOutcome.fromWire(r['outcome']);
    if (outcome == null) {
      throw MgrFfiException(
        'mgr_recover_install_v1: malformed recovery outcome',
        code: 'MGR_RECOVER_INSTALL_INVALID_RESPONSE',
      );
    }
    return outcome;
  }

  /// Remove everything the manager deployed from the install. True when
  /// anything was actually removed.
  Future<bool> undeployAll(String gameRoot) async {
    final r = await _call('mgr_undeploy_all', {'game_root': gameRoot});
    return _truthy(r['removed']);
  }
}

enum MgrInstallRecoveryOutcome {
  alreadyClean,
  busy,
  preMutationLockCleared,
  recoveredToPristine,
  completedApplyPreserved,
  completedUndeployConfirmed,
  compileRecoveryRequired,
  inspectionFailed;

  static MgrInstallRecoveryOutcome? fromWire(Object? value) => switch (value) {
    'already_clean' => alreadyClean,
    'busy' => busy,
    'pre_mutation_lock_cleared' => preMutationLockCleared,
    'recovered_to_pristine' => recoveredToPristine,
    'completed_apply_preserved' => completedApplyPreserved,
    'completed_undeploy_confirmed' => completedUndeployConfirmed,
    'compile_recovery_required' => compileRecoveryRequired,
    'inspection_failed' => inspectionFailed,
    _ => null,
  };
}

bool _validImportOutcome(
  MgrImportDisposition disposition,
  MgrImportMatchedBy matchedBy,
) => switch (disposition) {
  MgrImportDisposition.created => matchedBy == MgrImportMatchedBy.none,
  MgrImportDisposition.updated ||
  MgrImportDisposition.unchanged => matchedBy != MgrImportMatchedBy.none,
};

/// True for `true` and for positive counts — tolerates the Rust side
/// reporting `removed` as either a bool or a count.
bool _truthy(Object? value) => value == true || (value is num && value > 0);

/// Non-throwing list-of-maps accessor for response arrays.
List<Map<String, Object?>> _maps(Object? value) => value is List
    ? [for (final item in value.whereType<Map>()) item.cast<String, Object?>()]
    : const [];

ModEntryMetaView _libraryEntry(
  Object? raw, {
  required String command,
  String errorCode = 'UNKNOWN',
}) {
  if (raw is! Map ||
      raw.keys.any((key) => key is! String) ||
      raw['id'] is! String ||
      (raw['id']! as String).isEmpty ||
      raw['kind'] is! String ||
      raw['name'] is! String ||
      raw['components'] is! List ||
      (raw['components']! as List).any(
        (component) => component is! Map || component['type'] is! String,
      )) {
    throw MgrFfiException('$command: malformed library entry', code: errorCode);
  }
  return ModEntryMetaView.fromJson(raw.cast<String, Object?>());
}

MgrFfiErrorDetails? _mgrErrorDetails(String code, Object? raw) {
  if (raw is! Map || raw.keys.any((key) => key is! String)) return null;
  final details = raw.cast<String, Object?>();
  return switch (code) {
    'IMPORT_DUPLICATE_AMBIGUOUS' => _duplicateImportDetails(details),
    'IMPORT_IDENTITY_CONFLICT' => _identityConflictDetails(details),
    _ => null,
  };
}

MgrImportDuplicateAmbiguousDetails? _duplicateImportDetails(
  Map<String, Object?> details,
) {
  final rawCandidates = details['candidate_ids'];
  if (rawCandidates is! List ||
      rawCandidates.isEmpty ||
      rawCandidates.length > _maxImportErrorCandidates) {
    return null;
  }
  final candidates = <MgrImportCandidate>[];
  for (final raw in rawCandidates) {
    final id = _boundedImportCandidate(raw);
    if (id == null) return null;
    candidates.add(MgrImportCandidate(id: id));
  }
  return MgrImportDuplicateAmbiguousDetails(
    candidates: List.unmodifiable(candidates),
  );
}

MgrImportIdentityConflictDetails? _identityConflictDetails(
  Map<String, Object?> details,
) {
  final rawCandidates = details['candidates'];
  if (rawCandidates is! List ||
      rawCandidates.isEmpty ||
      rawCandidates.length > _maxImportErrorCandidates) {
    return null;
  }
  final candidates = <MgrImportCandidate>[];
  for (final raw in rawCandidates) {
    if (raw is! Map || raw.keys.any((key) => key is! String)) return null;
    final candidate = raw.cast<String, Object?>();
    final id = _boundedImportCandidate(candidate['id']);
    final rawMatchedBy = candidate['matched_by'];
    if (id == null ||
        rawMatchedBy is! List ||
        rawMatchedBy.isEmpty ||
        rawMatchedBy.length > 3) {
      return null;
    }
    final matchedBy = <MgrImportMatchedBy>[];
    for (final role in rawMatchedBy) {
      final parsed = MgrImportMatchedBy.fromWire(role);
      if (parsed == null ||
          parsed == MgrImportMatchedBy.none ||
          matchedBy.contains(parsed)) {
        return null;
      }
      matchedBy.add(parsed);
    }
    candidates.add(
      MgrImportCandidate(id: id, matchedBy: List.unmodifiable(matchedBy)),
    );
  }
  return MgrImportIdentityConflictDetails(
    candidates: List.unmodifiable(candidates),
  );
}

String? _boundedImportCandidate(Object? raw) {
  if (raw is! String || raw.isEmpty) return null;
  final runes = raw.runes.take(_maxImportCandidateRunes + 1).toList();
  if (runes.length > _maxImportCandidateRunes) return null;
  return raw;
}

enum MgrImportDisposition {
  created,
  updated,
  unchanged;

  static MgrImportDisposition? fromWire(Object? value) => switch (value) {
    'created' => created,
    'updated' => updated,
    'unchanged' => unchanged,
    _ => null,
  };
}

enum MgrImportMatchedBy {
  none,
  source,
  content,
  entryId;

  static MgrImportMatchedBy? fromWire(Object? value) => switch (value) {
    'none' => none,
    'source' => source,
    'content' => content,
    'entry_id' => entryId,
    _ => null,
  };

  String get wireName => switch (this) {
    none => 'none',
    source => 'source',
    content => 'content',
    entryId => 'entry_id',
  };
}

class MgrImportOutcome {
  const MgrImportOutcome({
    required this.entry,
    required this.disposition,
    required this.matchedBy,
  });

  final ModEntryMetaView entry;
  final MgrImportDisposition disposition;
  final MgrImportMatchedBy matchedBy;

  MgrImportOutcome withEntry(ModEntryMetaView authoritativeEntry) =>
      MgrImportOutcome(
        entry: authoritativeEntry,
        disposition: disposition,
        matchedBy: matchedBy,
      );
}

sealed class MgrFfiErrorDetails {
  const MgrFfiErrorDetails();
}

sealed class MgrImportRefusalDetails extends MgrFfiErrorDetails {
  const MgrImportRefusalDetails(this.candidates);

  final List<MgrImportCandidate> candidates;
}

class MgrImportDuplicateAmbiguousDetails extends MgrImportRefusalDetails {
  const MgrImportDuplicateAmbiguousDetails({
    required List<MgrImportCandidate> candidates,
  }) : super(candidates);
}

class MgrImportIdentityConflictDetails extends MgrImportRefusalDetails {
  const MgrImportIdentityConflictDetails({
    required List<MgrImportCandidate> candidates,
  }) : super(candidates);
}

class MgrImportCandidate {
  const MgrImportCandidate({this.id = '', this.matchedBy = const []});

  final String id;
  final List<MgrImportMatchedBy> matchedBy;
}

class MgrFfiException implements Exception {
  MgrFfiException(this.message, {this.code = 'UNKNOWN', this.details});

  final String message;

  /// Machine-readable code from the FFI error envelope (`error.code`);
  /// 'UNKNOWN' when absent. The UI branches on codes such as
  /// STUDIO_DEPLOY_ACTIVE.
  final String code;

  /// Machine-readable, bounded facts for known failures. Unknown or malformed
  /// detail objects stay null; callers never recover facts from [message].
  final MgrFfiErrorDetails? details;

  @override
  String toString() => message;
}
