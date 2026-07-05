/// Typed, tolerant views over the manager FFI JSON contract (the `mgr_*`
/// commands of gore_ffi.dll).
///
/// Parsing is deliberately defensive: unknown enum tags are preserved as raw
/// strings and missing fields fall back to empty values instead of throwing,
/// so a newer (or older) gore_ffi.dll never crashes the UI. Where a view does
/// not model every field, the unparsed JSON is kept in `raw`.
library;

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

/// One deployable component of a mod. [kind] is the raw serde `type` tag:
/// `ue4ss_lua` (name, rel, targets, opaque), `loc_patch` / `audio_patch` /
/// `texture_patch` / `angel_script_patch` (rel, targets), `triplet`
/// (rel_base, targets), `loose_pak` (rel, targets), `raw_file`
/// (rel, target_file) — unknown kinds parse into a generic view with the raw
/// tag preserved instead of throwing.
class ComponentView {
  const ComponentView({
    required this.kind,
    this.name,
    this.rel,
    this.targets = const [],
    this.opaque = false,
    this.rawFileTarget,
    this.raw = const {},
  });

  factory ComponentView.fromJson(Map<String, Object?> json) {
    final kind = _stringOr(json['type'], 'unknown');
    return ComponentView(
      kind: kind,
      name: _optString(json['name']),
      rel: _optString(json['rel']) ?? _optString(json['rel_base']),
      targets: _stringList(json['targets']),
      opaque: json['opaque'] == true,
      rawFileTarget: kind == 'raw_file'
          ? RawFileTargetView.fromJson(json['target_file'])
          : null,
      raw: Map.unmodifiable(json),
    );
  }

  /// Raw component type tag (see class doc); never null, `unknown` when the
  /// tag is missing.
  final String kind;

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

  /// `ue4ss_lua` only: true when the script body wasn't parseable.
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
}

/// No manager deployment exists in the game install.
class ManagerStatusNothingDeployed extends ManagerStatusView {
  const ManagerStatusNothingDeployed(super.raw);

  @override
  String get state => 'nothing_deployed';
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
  ManagerStatusInSync(super.raw) : loadout = _loadoutOrNull(raw['loadout']);

  /// The deployed loadout (null when the DLL sent an unexpected shape).
  final LoadoutView? loadout;

  @override
  String get state => 'in_sync';
}

/// The loadout was edited since the last apply: deployed != target.
class ManagerStatusChangesPending extends ManagerStatusView {
  ManagerStatusChangesPending(super.raw)
      : deployed = _loadoutOrNull(raw['deployed']),
        target = _loadoutOrNull(raw['target']);

  final LoadoutView? deployed;
  final LoadoutView? target;

  @override
  String get state => 'changes_pending';
}

/// A game update overwrote deployed files; a re-apply is needed.
class ManagerStatusGameUpdated extends ManagerStatusView {
  ManagerStatusGameUpdated(super.raw) : drifted = _stringList(raw['drifted']);

  /// Game-relative paths whose on-disk content drifted from what the manager
  /// deployed (empty when the DLL reported drift another way — see [raw]).
  final List<String> drifted;

  @override
  String get state => 'game_updated';
}

/// A state tag this app version doesn't know; [raw] carries everything.
class ManagerStatusUnknown extends ManagerStatusView {
  const ManagerStatusUnknown(super.raw);

  @override
  String get state => _stringOr(raw['state'], 'unknown');
}
