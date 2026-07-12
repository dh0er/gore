import '../audio/domain/audio_replacements_notifier.dart';
import '../editor/domain/override_entry.dart';
import '../scripts/domain/script_mods_notifier.dart';
import '../textures/domain/texture_replacements_notifier.dart';
import '../voice/domain/voice_edits_notifier.dart';
import 'dialog_topics_notifier.dart';

/// A saveable/loadable mod-studio project: the union of all editor domains. Serializes to
/// `project.json` inside a `.goremod` zip (see project_io.dart), with source media embedded.
class ModProject {
  ModProject({
    required this.name,
    this.version = '',
    this.author = '',
    this.delayMs = 0,
    this.overrides = const [],
    this.locEdits = const {},
    this.audio = const [],
    this.textures = const [],
    this.scripts = const [],
    this.dialogTopics = const [],
    this.voice = const [],
  });

  final String name;
  final String version;
  final String author;
  final int delayMs;
  final List<OverrideEntry> overrides;
  final Map<String, Map<String, String>> locEdits;
  final List<AudioReplacement> audio;
  final List<TextureReplacement> textures;
  final List<ScriptMod> scripts;
  final List<DialogTopicDefinition> dialogTopics;
  final List<VoiceArchiveEdit> voice;

  /// Rejects identities which map-backed editor state cannot represent without
  /// silently replacing an earlier entry.
  ///
  /// The comparison keys intentionally follow the portable deployment
  /// identities, rather than Dart's case-sensitive in-memory strings. Callers
  /// which accept a [ModProject] from outside the normal JSON path can use this
  /// before applying it to providers.
  void validateUniqueTargets() {
    _rejectDuplicateRecords<(String, String), OverrideEntry>(
      values: overrides,
      keyOf: (entry) =>
          (entry.classId.toLowerCase(), entry.field.toLowerCase()),
      describe: (entry) => '${entry.classId}.${entry.field}',
      domain: 'override target',
    );
    _rejectDuplicateRecords<(String, String), AudioReplacement>(
      values: audio,
      // The bank is a Windows filename, while FSB5 sample lookup is exact and
      // case-sensitive. Two samples which differ only by case may be distinct.
      keyOf: (entry) => (entry.bank.toLowerCase(), entry.sample),
      describe: (entry) => '${entry.bank}/${entry.sample}',
      domain: 'audio target',
    );
    _rejectDuplicateRecords<String, TextureReplacement>(
      values: textures,
      keyOf: (entry) => entry.asset.toLowerCase(),
      describe: (entry) => entry.asset,
      domain: 'texture target',
    );
    _rejectDuplicateRecords<String, ScriptMod>(
      values: scripts,
      keyOf: (entry) => _portableScriptPathIdentity(entry.relPath),
      describe: (entry) => entry.relPath,
      domain: 'script target',
    );
    _rejectDuplicateRecords<String, DialogTopicDefinition>(
      values: dialogTopics,
      keyOf: (entry) => entry.id.toLowerCase(),
      describe: (entry) => entry.id,
      domain: 'dialog topic id',
    );
    VoiceEditsNotifier.validateAll(voice);
  }

  ModProject copyWith({
    List<AudioReplacement>? audio,
    List<TextureReplacement>? textures,
    List<ScriptMod>? scripts,
    List<DialogTopicDefinition>? dialogTopics,
    List<VoiceArchiveEdit>? voice,
  }) => ModProject(
    name: name,
    version: version,
    author: author,
    delayMs: delayMs,
    overrides: overrides,
    locEdits: locEdits,
    audio: audio ?? this.audio,
    textures: textures ?? this.textures,
    scripts: scripts ?? this.scripts,
    dialogTopics: dialogTopics ?? this.dialogTopics,
    voice: voice ?? this.voice,
  );

  Map<String, Object?> toJson() {
    validateUniqueTargets();
    return {
      'format': 1,
      'mod': {'name': name, 'version': version, 'author': author},
      'delay_ms': delayMs,
      'overrides': [
        for (final o in overrides)
          {
            'class': o.classId,
            'field': o.field,
            'old': o.oldValue,
            'new': o.newValue,
          },
      ],
      'loc_edits': locEdits,
      'audio': [for (final a in audio) a.toJson()],
      'textures': [for (final t in textures) t.toJson()],
      'scripts': [for (final s in scripts) s.toJson()],
      'dialog_topics': [for (final topic in dialogTopics) topic.toJson()],
      if (voice.isNotEmpty) 'voice': [for (final edit in voice) edit.toJson()],
    };
  }

  factory ModProject.fromJson(Map<String, Object?> j) {
    if (!j.containsKey('format')) {
      throw const FormatException('missing project format; expected integer 1');
    }
    final format = j['format'];
    if (format is! int) {
      throw const FormatException('project format must be the integer 1');
    }
    if (format != 1) {
      throw FormatException('unsupported project format $format; expected 1');
    }

    final mod = (j['mod'] as Map?)?.cast<String, Object?>() ?? const {};
    final voice = [
      for (final edit in (j['voice'] as List? ?? const []))
        VoiceArchiveEdit.fromJson((edit as Map).cast<String, Object?>()),
    ];
    final project = ModProject(
      name: (mod['name'] as String?) ?? 'Mod',
      version: (mod['version'] as String?) ?? '',
      author: (mod['author'] as String?) ?? '',
      delayMs: (j['delay_ms'] as num?)?.toInt() ?? 0,
      overrides: [
        for (final o in (j['overrides'] as List? ?? const []))
          _overrideFrom((o as Map).cast<String, Object?>()),
      ],
      locEdits: _locFrom(j['loc_edits']),
      audio: [
        for (final a in (j['audio'] as List? ?? const []))
          AudioReplacement.fromJson((a as Map).cast<String, Object?>()),
      ],
      textures: [
        for (final t in (j['textures'] as List? ?? const []))
          TextureReplacement.fromJson((t as Map).cast<String, Object?>()),
      ],
      scripts: [
        for (final s in (j['scripts'] as List? ?? const []))
          ScriptMod.fromJson((s as Map).cast<String, Object?>()),
      ],
      dialogTopics: [
        for (final topic in (j['dialog_topics'] as List? ?? const []))
          DialogTopicDefinition.fromJson(
            (topic as Map).cast<String, Object?>(),
          ),
      ],
      voice: voice,
    );
    project.validateUniqueTargets();
    return project;
  }

  /// The `BuildSpec` JSON for the `mod_build` FFI command.
  Map<String, Object?> toBuildSpec() {
    validateUniqueTargets();
    for (final edit in voice) {
      if (edit.operation == VoicePatchOperation.add) {
        throw FormatException(
          'voice add cannot be built because new-member runtime binding is '
          'not qualified: ${edit.archive}/${edit.archivePath}',
        );
      }
    }
    return {
      'meta': {'name': name, 'version': version, 'author': author},
      'delay_ms': delayMs,
      'overrides': [for (final o in overrides) o.toFfiJson()],
      'loc_edits': locEdits,
      'audio': [for (final a in audio) a.toJson()],
      'texture': [for (final t in textures) t.toJson()],
      'scripts': [
        for (final s in scripts)
          {
            'op': scriptOpToString(s.op),
            'module_name': s.moduleName,
            'mini_cache': s.miniPath,
          },
      ],
      'dialog_topics': [for (final topic in dialogTopics) topic.toJson()],
      'voice': [for (final edit in voice) edit.toBuildJson()],
    };
  }
}

void _rejectDuplicateRecords<K, V>({
  required Iterable<V> values,
  required K Function(V value) keyOf,
  required String Function(V value) describe,
  required String domain,
}) {
  final seen = <K>{};
  for (final value in values) {
    if (!seen.add(keyOf(value))) {
      throw FormatException('duplicate $domain: ${describe(value)}');
    }
  }
}

String _portableScriptPathIdentity(String value) {
  final components = <String>[];
  for (final component in value.replaceAll(r'\', '/').split('/')) {
    if (component.isEmpty || component == '.') continue;
    if (component == '..' && components.isNotEmpty && components.last != '..') {
      components.removeLast();
    } else {
      components.add(component);
    }
  }
  return components.join('/').toLowerCase();
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
