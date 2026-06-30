import 'dart:io';

import 'package:flutter_riverpod/legacy.dart';

/// Whether a staged script mod adds a brand-new module or edits an existing one.
enum ScriptOp { add, edit }

ScriptOp scriptOpFromString(String s) => s == 'edit' ? ScriptOp.edit : ScriptOp.add;
String scriptOpToString(ScriptOp o) => o == ScriptOp.edit ? 'edit' : 'add';

/// Dependency-free stable hash (FNV-1a 64-bit) of [bytes] as zero-padded hex. Used to fingerprint
/// the .as content at compile time so an edited source can be detected without relying on mtime
/// (a loaded .goremod re-extracts the source to a new temp path with a fresh mtime, but identical
/// bytes — so a content hash still matches after re-extraction).
String fnv1aHex(List<int> bytes) {
  var h = BigInt.parse('cbf29ce484222325', radix: 16);
  final mask = (BigInt.one << 64) - BigInt.one;
  final prime = BigInt.parse('100000001b3', radix: 16);
  for (final b in bytes) {
    h = (h ^ BigInt.from(b & 0xff)) & mask;
    h = (h * prime) & mask;
  }
  return h.toRadixString(16).padLeft(16, '0');
}

/// One staged AngelScript mod: compile [asPath] into [miniPath] (a 1-module mini-cache), then
/// splice (add) / replace (edit) module [moduleName] into the precompiled cache at deploy.
class ScriptMod {
  const ScriptMod({
    required this.op,
    required this.moduleName,
    required this.relPath,
    required this.asPath,
    this.miniPath = '',
    this.compiledHash = '',
  });

  final ScriptOp op;
  final String moduleName;   // Modules TMap key
  final String relPath;      // ScriptRelativeFilename, e.g. AI/Foo.as
  final String asPath;       // .as source on disk (embedded in the .goremod)
  final String miniPath;     // compiled mini-cache on disk ('' until compiled)
  final String compiledHash; // FNV-1a of the .as content at compile time ('' until compiled)

  /// Staging identity: the unique game-relative path. NOT [moduleName] — two distinct paths can
  /// share a basename (e.g. AI/Foo.as and Quest/Foo.as both → module `Foo`), and keying by name
  /// would silently overwrite one with the other. [relPath] is stable across compile (only
  /// [moduleName] may change when the regen resolves the real name), so it's also a stable map key.
  String get key => relPath;
  bool get compiled => miniPath.isNotEmpty;

  Map<String, Object?> toJson() => {
        'op': scriptOpToString(op),
        'module': moduleName,
        'rel_path': relPath,
        'as_path': asPath,
        'mini_path': miniPath,
        'compiled_hash': compiledHash,
      };

  factory ScriptMod.fromJson(Map<String, Object?> j) => ScriptMod(
        op: scriptOpFromString((j['op'] as String?) ?? 'add'),
        moduleName: j['module'] as String,
        relPath: (j['rel_path'] as String?) ?? '',
        asPath: j['as_path'] as String,
        miniPath: (j['mini_path'] as String?) ?? '',
        compiledHash: (j['compiled_hash'] as String?) ?? '',
      );

  /// Path-only rewrite of the .as location (used by project_io when re-extracting the bundle).
  /// Preserves the compile (miniPath + compiledHash) — the bytes are unchanged, only the path is.
  ScriptMod withAsPath(String path) => ScriptMod(
      op: op, moduleName: moduleName, relPath: relPath, asPath: path,
      miniPath: miniPath, compiledHash: compiledHash);

  /// Path-only rewrite of the mini-cache location (used by project_io). Preserves compiledHash.
  ScriptMod withMiniPath(String path) => ScriptMod(
      op: op, moduleName: moduleName, relPath: relPath, asPath: asPath,
      miniPath: path, compiledHash: compiledHash);

  /// Records a fresh compile: sets the mini-cache and the hash of the .as content it was built from.
  ScriptMod withCompiled(String miniPath, String compiledHash) => ScriptMod(
      op: op, moduleName: moduleName, relPath: relPath, asPath: asPath,
      miniPath: miniPath, compiledHash: compiledHash);

  /// Points at a new .as source, invalidating any prior compile (clears miniPath + compiledHash).
  ScriptMod withSource(String asPath) => ScriptMod(
      op: op, moduleName: moduleName, relPath: relPath, asPath: asPath,
      miniPath: '', compiledHash: '');
}

/// True only if the mod has a compiled mini AND the on-disk .as still matches the content
/// that was compiled (so an edited source reads as not-fresh). IO errors => not fresh.
bool scriptCompileFresh(ScriptMod m) {
  if (m.miniPath.isEmpty || m.compiledHash.isEmpty) return false;
  try {
    return fnv1aHex(File(m.asPath).readAsBytesSync()) == m.compiledHash;
  } catch (_) {
    return false;
  }
}

class ScriptModsState {
  const ScriptModsState({this.items = const {}});
  final Map<String, ScriptMod> items;
  int get count => items.length;
  List<ScriptMod> get entries => items.values.toList()
    ..sort((a, b) => a.moduleName.compareTo(b.moduleName));
  ScriptModsState copyWith({Map<String, ScriptMod>? items}) =>
      ScriptModsState(items: items ?? this.items);
}

class ScriptModsNotifier extends StateNotifier<ScriptModsState> {
  ScriptModsNotifier() : super(const ScriptModsState());
  void setMod(ScriptMod m) {
    final items = Map<String, ScriptMod>.from(state.items);
    items[m.key] = m;
    state = state.copyWith(items: items);
  }
  void remove(String key) {
    if (!state.items.containsKey(key)) return;
    final items = Map<String, ScriptMod>.from(state.items)..remove(key);
    state = state.copyWith(items: items);
  }
  void clearAll() {
    if (state.items.isEmpty) return;
    state = const ScriptModsState();
  }
  void loadAll(List<ScriptMod> list) {
    state = ScriptModsState(items: {for (final m in list) m.key: m});
  }
}

final scriptModsProvider =
    StateNotifierProvider<ScriptModsNotifier, ScriptModsState>((ref) => ScriptModsNotifier());
