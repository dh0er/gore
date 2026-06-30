import 'package:flutter_riverpod/legacy.dart';

/// Whether a staged script mod adds a brand-new module or edits an existing one.
enum ScriptOp { add, edit }

ScriptOp scriptOpFromString(String s) => s == 'edit' ? ScriptOp.edit : ScriptOp.add;
String scriptOpToString(ScriptOp o) => o == ScriptOp.edit ? 'edit' : 'add';

/// One staged AngelScript mod: compile [asPath] into [miniPath] (a 1-module mini-cache), then
/// splice (add) / replace (edit) module [moduleName] into the precompiled cache at deploy.
class ScriptMod {
  const ScriptMod({
    required this.op,
    required this.moduleName,
    required this.relPath,
    required this.asPath,
    this.miniPath = '',
  });

  final ScriptOp op;
  final String moduleName; // Modules TMap key
  final String relPath;    // ScriptRelativeFilename, e.g. AI/Foo.as
  final String asPath;     // .as source on disk (embedded in the .goremod)
  final String miniPath;   // compiled mini-cache on disk ('' until compiled)

  String get key => moduleName;
  bool get compiled => miniPath.isNotEmpty;

  Map<String, Object?> toJson() => {
        'op': scriptOpToString(op),
        'module': moduleName,
        'rel_path': relPath,
        'as_path': asPath,
        'mini_path': miniPath,
      };

  factory ScriptMod.fromJson(Map<String, Object?> j) => ScriptMod(
        op: scriptOpFromString((j['op'] as String?) ?? 'add'),
        moduleName: j['module'] as String,
        relPath: (j['rel_path'] as String?) ?? '',
        asPath: j['as_path'] as String,
        miniPath: (j['mini_path'] as String?) ?? '',
      );

  ScriptMod withAsPath(String path) =>
      ScriptMod(op: op, moduleName: moduleName, relPath: relPath, asPath: path, miniPath: miniPath);
  ScriptMod withMiniPath(String path) =>
      ScriptMod(op: op, moduleName: moduleName, relPath: relPath, asPath: asPath, miniPath: path);
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
