/// Typed, tolerant views over the manager FFI JSON contract (the `mgr_*`
/// commands of gore_ffi.dll).
///
/// Parsing is deliberately defensive: unknown enum tags are preserved as raw
/// strings and missing fields fall back to empty values instead of throwing,
/// so a newer (or older) gore_ffi.dll never crashes the UI. Where a view does
/// not model every field, the unparsed JSON is kept in `raw`.
library;

import 'dart:convert';

const _maxManagerOwnedItems = 128;
const _maxManagerOwnedSourceBytes = 64 * 1024;
const _maxManagerOwnedItemBytes = 4 * 1024;

String? _optString(Object? value) =>
    value is String && value.isNotEmpty ? value : null;

String _stringOr(Object? value, String fallback) =>
    value is String ? value : fallback;

List<String> _stringList(Object? value) =>
    value is List ? value.whereType<String>().toList() : const [];

List<Map<String, Object?>> _mapList(Object? value) => value is List
    ? [for (final item in value.whereType<Map>()) item.cast<String, Object?>()]
    : const [];

/// ManagerStatus serializes loadouts on the wire as plain ARRAYS of entries
/// (`[{id, enabled}, ...]`); the library file / `mgr_set_loadout` shape is
/// the `{format, entries}` map. Accept both; anything else is null.
LoadoutView? _loadoutOrNull(Object? value) {
  if (value is List) {
    return LoadoutView(
      entries: [for (final m in _mapList(value)) LoadoutEntryView.fromJson(m)],
    );
  }
  if (value is Map) {
    return LoadoutView.fromJson(value.cast<String, Object?>());
  }
  return null;
}

ManagerOwnedPathGroupView? _managerOwnedGroupOrNull(Object? value) {
  if (value is! Map) return null;
  final rawItems = value['items'];
  final total = value['total'];
  final truncated = value['truncated'];
  if (rawItems is! List ||
      rawItems.length > _maxManagerOwnedItems ||
      rawItems.any((item) => item is! String) ||
      total is! int ||
      total < rawItems.length ||
      truncated is! bool ||
      truncated != (rawItems.length < total)) {
    return null;
  }
  final items = rawItems.cast<String>();
  var sourceBytes = 0;
  try {
    for (final item in items) {
      final itemBytes = utf8.encode(item).length;
      if (itemBytes > _maxManagerOwnedItemBytes) return null;
      sourceBytes += itemBytes;
      if (sourceBytes > _maxManagerOwnedSourceBytes) return null;
    }
  } on FormatException {
    return null;
  }
  return ManagerOwnedPathGroupView(
    items: List.unmodifiable(items),
    total: total,
    truncated: truncated,
  );
}

ManagerOwnedDeploymentView? _managerOwnedOrNull(Object? value) {
  if (value is! Map) return null;
  final live = _managerOwnedGroupOrNull(value['live']);
  final backups = _managerOwnedGroupOrNull(value['backups']);
  final additive = _managerOwnedGroupOrNull(value['additive']);
  final ue4ss = _managerOwnedGroupOrNull(value['ue4ss']);
  final recovery = _managerOwnedGroupOrNull(value['recovery']);
  if (live == null ||
      backups == null ||
      additive == null ||
      ue4ss == null ||
      recovery == null) {
    return null;
  }
  return ManagerOwnedDeploymentView(
    live: live,
    backups: backups,
    additive: additive,
    ue4ss: ue4ss,
    recovery: recovery,
  );
}

/// Library metadata for one installed mod (`ModEntryMeta` on the Rust side).
class ModEntryMetaView {
  const ModEntryMetaView({
    required this.id,
    required this.kind,
    required this.name,
    this.version,
    this.author,
    this.importedAt,
    this.source,
    this.components = const [],
  });

  factory ModEntryMetaView.fromJson(Map<String, Object?> json) {
    return ModEntryMetaView(
      id: _stringOr(json['id'], ''),
      kind: _stringOr(json['kind'], ''),
      name: _stringOr(json['name'], ''),
      version: _optString(json['version']),
      author: _optString(json['author']),
      importedAt: _optString(json['imported_at']),
      source: _optString(json['source']),
      components: [
        for (final m in _mapList(json['components'])) ComponentView.fromJson(m),
      ],
    );
  }

  final String id;

  /// Raw kind tag: `goremod`, `foreign_triplet`, `foreign_pak`,
  /// `foreign_ue4ss`, `foreign_rawfile`, `foreign_mixed`, or a future value.
  final String kind;
  final String name;
  final String? version;
  final String? author;
  final String? importedAt;
  final String? source;
  final List<ComponentView> components;
}

/// How completely a component's metadata describes its conflict footprint.
enum FootprintCoverage { exact, partial, advisory, opaque }

FootprintCoverage _footprintCoverage(
  Map<String, Object?> json,
  String kind,
  List<String> targets,
  bool opaque,
) {
  // A present but future/invalid value is not safe to infer through. Missing is
  // different: it means an older DLL, for which the current native derivation
  // can be reproduced conservatively from the unchanged component contract.
  if (json.containsKey('coverage')) {
    return switch (json['coverage']) {
      'exact' => FootprintCoverage.exact,
      'partial' => FootprintCoverage.partial,
      'advisory' => FootprintCoverage.advisory,
      'opaque' => FootprintCoverage.opaque,
      _ => FootprintCoverage.opaque,
    };
  }

  return switch (kind) {
    'ue4ss_lua' =>
      !opaque
          ? FootprintCoverage.exact
          : targets.isNotEmpty
          ? FootprintCoverage.partial
          : FootprintCoverage.opaque,
    'triplet' =>
      targets.isNotEmpty
          ? FootprintCoverage.advisory
          : FootprintCoverage.opaque,
    'loose_pak' =>
      targets.isNotEmpty ? FootprintCoverage.exact : FootprintCoverage.opaque,
    'loc_patch' ||
    'audio_patch' ||
    'texture_patch' ||
    'angel_script_patch' ||
    'file_patch' ||
    'pak_file_patch' ||
    'voice_archive_patch' ||
    'raw_file' => FootprintCoverage.exact,
    _ => FootprintCoverage.opaque,
  };
}

/// One deployable component of a mod. [kind] is the raw serde `type` tag:
/// `ue4ss_lua` (name, rel, targets, opaque), `loc_patch` / `audio_patch` /
/// `texture_patch` / `angel_script_patch` (rel, targets), `triplet`
/// (rel_base, targets), `loose_pak` (rel, targets), `raw_file`
/// (rel, target_file) — unknown kinds parse into a generic view with the raw
/// tag preserved instead of throwing.
class ComponentView {
  const ComponentView({
    required this.kind,
    required this.coverage,
    this.name,
    this.rel,
    this.targets = const [],
    this.opaque = false,
    this.rawFileTarget,
    this.raw = const {},
  });

  factory ComponentView.fromJson(Map<String, Object?> json) {
    final kind = _stringOr(json['type'], 'unknown');
    final targets = _stringList(json['targets']);
    final opaque = json['opaque'] == true;
    return ComponentView(
      kind: kind,
      coverage: _footprintCoverage(json, kind, targets, opaque),
      name: _optString(json['name']),
      rel: _optString(json['rel']) ?? _optString(json['rel_base']),
      targets: targets,
      opaque: opaque,
      rawFileTarget: kind == 'raw_file'
          ? RawFileTargetView.fromJson(json['target_file'])
          : null,
      raw: Map.unmodifiable(json),
    );
  }

  /// Raw component type tag (see class doc); never null, `unknown` when the
  /// tag is missing.
  final String kind;

  /// How completely [targets] (or [rawFileTarget]) describe this component's
  /// footprint. This does not claim anything about runtime precedence.
  final FootprintCoverage coverage;

  /// Script name (`ue4ss_lua` only).
  final String? name;

  /// Relative path inside the mod's library storage (`rel`, or `rel_base`
  /// for triplets).
  final String? rel;

  /// Conflict-analysis footprint keys this component claims — NOT file
  /// paths: loc entries as `id|set`, audio samples as `bank|sample`, CDO
  /// overrides as `Class.Field`, texture/pak content as asset/package
  /// paths, AngelScript patches as module names. Empty for `raw_file`,
  /// which uses [rawFileTarget] instead.
  final List<String> targets;

  /// `ue4ss_lua` only: true when [targets] is a known subset rather than the
  /// component's complete conflict-analysis footprint.
  final bool opaque;

  /// Destination descriptor, `raw_file` components only.
  final RawFileTargetView? rawFileTarget;

  /// The full unparsed component JSON, for fields this view doesn't model.
  final Map<String, Object?> raw;

  /// Short human label: script name, else the relative path, else the kind.
  String get displayLabel => name ?? rel ?? kind;
}

/// Destination of a `raw_file` component. Serde externally-tagged snake_case:
/// unit variants arrive as plain strings (`"lcache"`, `"script_cache"`), the
/// bank variant as `{"bank": {"name": "SFX"}}`. Anything unrecognized parses
/// as kind `unknown` rather than throwing.
class RawFileTargetView {
  const RawFileTargetView({required this.kind, this.bankName});

  factory RawFileTargetView.fromJson(Object? json) {
    if (json is String && json.isNotEmpty) {
      return RawFileTargetView(kind: json);
    }
    if (json is Map) {
      final map = json.cast<String, Object?>();
      if (map['bank'] case final Map bank) {
        return RawFileTargetView(
          kind: 'bank',
          bankName: _optString(bank.cast<String, Object?>()['name']),
        );
      }
      // Tolerate future externally-tagged variants: {"<tag>": {...}}.
      if (map.length == 1) {
        return RawFileTargetView(kind: map.keys.first);
      }
    }
    return const RawFileTargetView(kind: 'unknown');
  }

  /// `lcache`, `script_cache`, `bank`, or a future/unknown tag.
  final String kind;

  /// FMOD bank name, set only when [kind] == `bank`.
  final String? bankName;
}

/// One loadout slot: a library mod id plus its enabled flag.
class LoadoutEntryView {
  const LoadoutEntryView({required this.id, this.enabled = true});

  factory LoadoutEntryView.fromJson(Map<String, Object?> json) {
    return LoadoutEntryView(
      id: _stringOr(json['id'], ''),
      enabled: json['enabled'] != false,
    );
  }

  final String id;
  final bool enabled;

  Map<String, Object?> toJson() => {'id': id, 'enabled': enabled};
}

/// The ordered loadout (load order is the list order, first = lowest
/// priority on the Rust side's terms — ordering semantics live there).
class LoadoutView {
  const LoadoutView({this.format = 1, this.entries = const []});

  factory LoadoutView.fromJson(Map<String, Object?> json) {
    return LoadoutView(
      format: switch (json['format']) {
        final num value => value.toInt(),
        _ => 1,
      },
      entries: [
        for (final m in _mapList(json['entries'])) LoadoutEntryView.fromJson(m),
      ],
    );
  }

  final int format;
  final List<LoadoutEntryView> entries;

  Map<String, Object?> toJson() => {
    'format': format,
    'entries': [for (final entry in entries) entry.toJson()],
  };
}

/// One conflict between mods of the current loadout (`mgr_analyze`).
class ConflictView {
  const ConflictView({
    required this.kind,
    required this.target,
    this.modIds = const [],
    required this.severity,
  });

  factory ConflictView.fromJson(Map<String, Object?> json) {
    return ConflictView(
      kind: _stringOr(json['kind'], ''),
      target: _stringOr(json['target'], ''),
      modIds: _stringList(json['mods']),
      severity: _stringOr(json['severity'], ''),
    );
  }

  final String kind;
  final String target;
  final List<String> modIds;
  final String severity;
}

/// Result of a declarative apply (`mgr_apply`).
class ApplyReportView {
  const ApplyReportView({this.applied = const [], this.warnings = const []});

  factory ApplyReportView.fromJson(Map<String, Object?> json) {
    return ApplyReportView(
      applied: _stringList(json['applied']),
      warnings: _stringList(json['warnings']),
    );
  }

  /// Names of the mods that were applied, in apply order.
  final List<String> applied;
  final List<String> warnings;
}

/// One bounded group of paths recorded as Manager-owned deployment evidence.
/// Paths are display-only and do not assert that the named object still exists.
class ManagerOwnedPathGroupView {
  const ManagerOwnedPathGroupView({
    required this.items,
    required this.total,
    required this.truncated,
  });

  final List<String> items;
  final int total;
  final bool truncated;
}

/// The five fixed path groups projected from one validated Manager deploy record.
class ManagerOwnedDeploymentView {
  const ManagerOwnedDeploymentView({
    required this.live,
    required this.backups,
    required this.additive,
    required this.ue4ss,
    required this.recovery,
  });

  final ManagerOwnedPathGroupView live;
  final ManagerOwnedPathGroupView backups;
  final ManagerOwnedPathGroupView additive;
  final ManagerOwnedPathGroupView ue4ss;
  final ManagerOwnedPathGroupView recovery;
}

/// Deployment status of the game install (`mgr_status`). Sealed hierarchy —
/// switch on the subtype. Unknown / future states parse as
/// [ManagerStatusUnknown] so a newer DLL never breaks status display; each
/// variant also keeps the full status JSON in [raw] for fields these views
/// don't model yet.
sealed class ManagerStatusView {
  const ManagerStatusView(this.raw);

  factory ManagerStatusView.fromJson(Map<String, Object?> json) {
    return switch (json['state']) {
      'nothing_deployed' => ManagerStatusNothingDeployed(json),
      'recovery_required' => ManagerStatusRecoveryRequired.fromJson(json),
      'studio_deploy_active' => ManagerStatusStudioDeployActive(json),
      'in_sync' => ManagerStatusInSync(json),
      'changes_pending' => ManagerStatusChangesPending(json),
      'game_updated' => ManagerStatusGameUpdated(json),
      _ => ManagerStatusUnknown(json),
    };
  }

  /// The full unparsed status JSON.
  final Map<String, Object?> raw;

  /// The raw state tag.
  String get state;

  /// Optional display-only paths from an exact Manager-owned record. Unknown,
  /// Nothing, and Studio states never adopt this field even if malformed or
  /// future wire data includes it.
  ManagerOwnedDeploymentView? get managerOwned => null;
}

/// No manager deployment exists in the game install.
class ManagerStatusNothingDeployed extends ManagerStatusView {
  const ManagerStatusNothingDeployed(super.raw);

  @override
  String get state => 'nothing_deployed';
}

/// An interrupted deployment must be recovered through undeploy before any new
/// deployment is safe.
class ManagerStatusRecoveryRequired extends ManagerStatusView {
  const ManagerStatusRecoveryRequired(super.raw) : managerOwned = null;

  ManagerStatusRecoveryRequired.fromJson(super.raw)
    : managerOwned = _managerOwnedOrNull(raw['manager_owned']);

  @override
  final ManagerOwnedDeploymentView? managerOwned;

  @override
  String get state => 'recovery_required';
}

/// mod-studio's own single-mod deployment is active; the manager must not
/// touch the install until that deploy is undone on the studio side.
class ManagerStatusStudioDeployActive extends ManagerStatusView {
  ManagerStatusStudioDeployActive(super.raw)
    : modName = _optString(raw['mod_name']) ?? '';

  /// Name of the studio-deployed mod ('' when the DLL omitted it).
  final String modName;

  @override
  String get state => 'studio_deploy_active';
}

/// The deployed state matches the current loadout.
class ManagerStatusInSync extends ManagerStatusView {
  ManagerStatusInSync(super.raw)
    : loadout = _loadoutOrNull(raw['loadout']),
      managerOwned = _managerOwnedOrNull(raw['manager_owned']);

  /// The deployed loadout (null when the DLL sent an unexpected shape).
  final LoadoutView? loadout;

  @override
  final ManagerOwnedDeploymentView? managerOwned;

  @override
  String get state => 'in_sync';
}

/// The loadout was edited since the last apply: deployed != target.
class ManagerStatusChangesPending extends ManagerStatusView {
  ManagerStatusChangesPending(super.raw)
    : deployed = _loadoutOrNull(raw['deployed']),
      target = _loadoutOrNull(raw['target']),
      managerOwned = _managerOwnedOrNull(raw['manager_owned']);

  final LoadoutView? deployed;
  final LoadoutView? target;

  @override
  final ManagerOwnedDeploymentView? managerOwned;

  @override
  String get state => 'changes_pending';
}

/// A game update overwrote deployed files; a re-apply is needed.
class ManagerStatusGameUpdated extends ManagerStatusView {
  ManagerStatusGameUpdated(super.raw)
    : drifted = _stringList(raw['drifted']),
      managerOwned = _managerOwnedOrNull(raw['manager_owned']);

  /// Game-relative paths whose on-disk content drifted from what the manager
  /// deployed (empty when the DLL reported drift another way — see [raw]).
  final List<String> drifted;

  @override
  final ManagerOwnedDeploymentView? managerOwned;

  @override
  String get state => 'game_updated';
}

/// A state tag this app version doesn't know; [raw] carries everything.
class ManagerStatusUnknown extends ManagerStatusView {
  const ManagerStatusUnknown(super.raw);

  @override
  String get state => _stringOr(raw['state'], 'unknown');
}
