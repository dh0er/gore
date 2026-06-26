import '../audio/domain/audio_replacements_notifier.dart';
import '../editor/domain/override_entry.dart';

/// A saveable/loadable mod-studio project: the union of all editor domains. Serializes to
/// `project.json` inside a `.goremod` zip (see project_io.dart), with source WAVs embedded.
class ModProject {
  ModProject({
    required this.name,
    this.version = '',
    this.author = '',
    this.delayMs = 0,
    this.overrides = const [],
    this.locEdits = const {},
    this.audio = const [],
  });

  final String name;
  final String version;
  final String author;
  final int delayMs;
  final List<OverrideEntry> overrides;
  final Map<String, Map<String, String>> locEdits;
  final List<AudioReplacement> audio;

  ModProject copyWith({List<AudioReplacement>? audio}) => ModProject(
        name: name,
        version: version,
        author: author,
        delayMs: delayMs,
        overrides: overrides,
        locEdits: locEdits,
        audio: audio ?? this.audio,
      );

  Map<String, Object?> toJson() => {
        'format': 1,
        'mod': {'name': name, 'version': version, 'author': author},
        'delay_ms': delayMs,
        'overrides': [
          for (final o in overrides)
            {'class': o.classId, 'field': o.field, 'old': o.oldValue, 'new': o.newValue}
        ],
        'loc_edits': locEdits,
        'audio': [for (final a in audio) a.toJson()],
      };

  factory ModProject.fromJson(Map<String, Object?> j) {
    final mod = (j['mod'] as Map?)?.cast<String, Object?>() ?? const {};
    return ModProject(
      name: (mod['name'] as String?) ?? 'Mod',
      version: (mod['version'] as String?) ?? '',
      author: (mod['author'] as String?) ?? '',
      delayMs: (j['delay_ms'] as num?)?.toInt() ?? 0,
      overrides: [
        for (final o in (j['overrides'] as List? ?? const []))
          _overrideFrom((o as Map).cast<String, Object?>())
      ],
      locEdits: _locFrom(j['loc_edits']),
      audio: [
        for (final a in (j['audio'] as List? ?? const []))
          AudioReplacement.fromJson((a as Map).cast<String, Object?>())
      ],
    );
  }

  /// The `BuildSpec` JSON for the `mod_build` FFI command.
  Map<String, Object?> toBuildSpec() => {
        'meta': {'name': name, 'version': version, 'author': author},
        'delay_ms': delayMs,
        'overrides': [for (final o in overrides) o.toFfiJson()],
        'loc_edits': locEdits,
        'audio': [for (final a in audio) a.toJson()],
      };
}

OverrideEntry _overrideFrom(Map<String, Object?> j) => OverrideEntry(
      classId: j['class'] as String,
      field: j['field'] as String,
      oldValue: j['old'],
      newValue: j['new'] as Object,
    );

Map<String, Map<String, String>> _locFrom(Object? v) {
  final out = <String, Map<String, String>>{};
  if (v is Map) {
    v.forEach((id, sets) {
      if (id is String && sets is Map) {
        final inner = <String, String>{};
        sets.forEach((s, t) {
          if (s is String && t is String) inner[s] = t;
        });
        out[id] = inner;
      }
    });
  }
  return out;
}
