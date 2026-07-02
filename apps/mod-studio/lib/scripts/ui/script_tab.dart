import 'dart:io';

import 'package:file_selector/file_selector.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_riverpod/legacy.dart'; // StateProvider
import 'package:path/path.dart' as p;

import '../../app/domain/ui_settings.dart'; // gameExePathProvider
import '../../app/game_paths.dart'; // gameRootFromExe
import '../../app/ui/path_tree.dart';
import '../../core/mod_ffi.dart';
import '../../core/providers.dart';
import '../domain/script_mods_notifier.dart';
import '../domain/script_modules_provider.dart';

/// Selected script path: a vanilla module's game-relative path (tree leaf) or a
/// staged mod's key (= relPath). One store for both — they share the key space,
/// so selecting a staged edit highlights the same leaf in the vanilla tree.
final _selectedModuleProvider = StateProvider<String?>((ref) => null);

/// Game-relative path for a vanilla module. Some cache entries have no recorded
/// file — fall back to `<name>.as` at the tree root (the same rule staging uses,
/// so tree paths and staged-mod keys line up).
String _moduleRelPath(ScriptModuleInfo m) =>
    m.file.isEmpty ? '${m.name}.as' : m.file;

class ScriptTab extends ConsumerStatefulWidget {
  const ScriptTab({super.key});

  @override
  ConsumerState<ScriptTab> createState() => _ScriptTabState();
}

class _ScriptTabState extends ConsumerState<ScriptTab> {
  String _query = '';
  final TextEditingController _searchController = TextEditingController();

  // Identity-stable tree-path list + relPath→module lookup, rebuilt only when
  // the modules LIST identity changes (i.e. the provider reloaded).
  // PathTreeBrowser caches its built tree by list identity, so passing a fresh
  // list per build would rebuild the ~7k-leaf tree every frame.
  List<ScriptModuleInfo>? _cacheSource;
  List<String>? _treePaths;
  Map<String, ScriptModuleInfo>? _byRelPath;

  @override
  void dispose() {
    _searchController.dispose();
    super.dispose();
  }

  void _refreshCaches(List<ScriptModuleInfo> modules) {
    if (identical(_cacheSource, modules) && _treePaths != null) return;
    _cacheSource = modules;
    _treePaths = [for (final m in modules) _moduleRelPath(m)];
    _byRelPath = {for (final m in modules) _moduleRelPath(m): m};
  }

  @override
  Widget build(BuildContext context) {
    final modulesAsync = ref.watch(scriptModulesProvider);
    final state = ref.watch(scriptModsProvider);
    final selectedKey = ref.watch(_selectedModuleProvider);
    final scheme = Theme.of(context).colorScheme;

    return Column(
      children: [
        Expanded(
          child: modulesAsync.when(
            loading: () => const Center(child: CircularProgressIndicator()),
            error: (e, _) =>
                Center(child: SelectableText('Module list error: $e')),
            data: (modules) {
              _refreshCaches(modules);
              return Row(
                children: [
                  Expanded(
                    flex: 2,
                    child: _browser(modules, state, selectedKey, scheme),
                  ),
                  const VerticalDivider(width: 1),
                  Expanded(flex: 3, child: _detail(state, selectedKey, scheme)),
                ],
              );
            },
          ),
        ),
        const Divider(height: 1),
        const _StagedScriptsPanel(),
      ],
    );
  }

  // -- Browser: lazy tree (browse) + flat list (search) --------------------

  void _select(String relPath) =>
      ref.read(_selectedModuleProvider.notifier).state = relPath;

  Widget _browser(List<ScriptModuleInfo> modules, ScriptModsState staged,
      String? selectedKey, ColorScheme scheme) {
    if (modules.isEmpty) {
      // No cache found (or no game configured) — same hint the old picker gave.
      return Center(
        child: Padding(
          padding: const EdgeInsets.all(16),
          child: Text(
            'No vanilla modules — set the game path in Settings.',
            textAlign: TextAlign.center,
            style: TextStyle(color: scheme.onSurfaceVariant),
          ),
        ),
      );
    }
    final q = _query.toLowerCase();
    final matches = _query.isEmpty
        ? const <ScriptModuleInfo>[]
        : (modules
            .where((m) =>
                m.name.toLowerCase().contains(q) ||
                _moduleRelPath(m).toLowerCase().contains(q))
            .toList()
          ..sort((a, b) => _moduleRelPath(a)
              .toLowerCase()
              .compareTo(_moduleRelPath(b).toLowerCase())));
    return Column(
      children: [
        Padding(
          padding: const EdgeInsets.all(8),
          child: TextField(
            controller: _searchController,
            decoration: InputDecoration(
              prefixIcon: const Icon(Icons.search),
              hintText: 'Search scripts',
              suffixIcon: _query.isEmpty
                  ? null
                  : IconButton(
                      icon: const Icon(Icons.clear),
                      tooltip: 'Clear',
                      onPressed: () {
                        _searchController.clear();
                        setState(() => _query = '');
                      },
                    ),
            ),
            onChanged: (v) => setState(() => _query = v),
          ),
        ),
        Expanded(
          // Browse = lazy folder tree; an active search = flat hit list
          // (name OR path matched anywhere). The tree stays mounted (just
          // offstage) during a search so its expansion state and built tree
          // survive the search being cleared — same pattern as the Textures tab.
          child: Stack(
            children: [
              Offstage(
                offstage: _query.isNotEmpty,
                child: PathTreeBrowser(
                  paths: _treePaths!,
                  selectedPath: selectedKey,
                  onSelect: _select,
                  leafIcon: Icons.description_outlined,
                  markedPaths: staged.items.keys.toSet(),
                ),
              ),
              if (_query.isNotEmpty) _flatList(matches, staged, selectedKey),
            ],
          ),
        ),
        Text(
          _query.isEmpty
              ? '${modules.length} modules'
              : '${matches.length} match / ${modules.length} total',
          style: Theme.of(context).textTheme.bodySmall,
        ),
      ],
    );
  }

  Widget _flatList(List<ScriptModuleInfo> matches, ScriptModsState staged,
      String? selectedKey) {
    return ListView.builder(
      itemCount: matches.length,
      itemBuilder: (c, i) {
        final m = matches[i];
        final rel = _moduleRelPath(m);
        return ListTile(
          dense: true,
          selected: rel == selectedKey,
          title: Text(m.name, maxLines: 1, overflow: TextOverflow.ellipsis),
          subtitle: Text(rel, maxLines: 1, overflow: TextOverflow.ellipsis),
          trailing: staged.items.containsKey(rel)
              ? const Icon(Icons.check, size: 16)
              : null,
          onTap: () => _select(rel),
        );
      },
    );
  }

  // -- Detail pane ----------------------------------------------------------

  Widget _detail(ScriptModsState state, String? selectedKey, ColorScheme scheme) {
    final placeholder = Center(
      child: Text('Select or add a script mod',
          style: TextStyle(color: scheme.onSurfaceVariant)),
    );
    if (selectedKey == null) return placeholder;
    final staged = state.items[selectedKey];
    if (staged != null) {
      // Key the detail pane to the selected mod so switching selection builds a
      // FRESH _ModDetailState — otherwise the old state (and its _busy/_status/
      // _error compile UI) is reused for the next mod.
      return _ModDetail(key: ValueKey(staged.key), mod: staged);
    }
    // Not staged: either a vanilla module (show info + Edit) or a dangling
    // selection (e.g. a staged 'add' that was removed — its path isn't in the
    // vanilla tree, so fall back to the placeholder).
    final module = _byRelPath?[selectedKey];
    if (module == null) return placeholder;
    // Keyed so the emit-busy state resets when the selection changes.
    return _VanillaModuleDetail(
        key: ValueKey(selectedKey), module: module, relPath: selectedKey);
  }
}

/// Detail pane for a vanilla (not yet staged) module: name + path info and an
/// Edit action that stages a [ScriptOp.edit] mod pre-filled with the module's
/// emitted source.
class _VanillaModuleDetail extends ConsumerStatefulWidget {
  const _VanillaModuleDetail(
      {super.key, required this.module, required this.relPath});
  final ScriptModuleInfo module;
  final String relPath;

  @override
  ConsumerState<_VanillaModuleDetail> createState() =>
      _VanillaModuleDetailState();
}

class _VanillaModuleDetailState extends ConsumerState<_VanillaModuleDetail> {
  bool _busy = false;

  @override
  Widget build(BuildContext context) {
    final scheme = Theme.of(context).colorScheme;
    // Watched in build (not in the tap callback) so the pristine-cache path
    // tracks the configured game.
    final cache = scriptCachePath(ref);
    return Padding(
      padding: const EdgeInsets.all(16),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Text(widget.module.name,
              style: Theme.of(context).textTheme.titleMedium),
          Text('Vanilla module — not staged',
              style: TextStyle(color: scheme.onSurfaceVariant)),
          const SizedBox(height: 12),
          _kvRow('Module', widget.module.name),
          _kvRow('Path', widget.relPath),
          const SizedBox(height: 12),
          FilledButton.icon(
            icon: const Icon(Icons.edit_outlined, size: 18),
            label: const Text('Edit'),
            onPressed: _busy ? null : () => _stageEdit(cache),
          ),
          if (_busy)
            const Padding(
              padding: EdgeInsets.symmetric(vertical: 8),
              child: LinearProgressIndicator(),
            ),
        ],
      ),
    );
  }

  /// Stage an edit of this vanilla module: emit its recompilable .as source to
  /// a temp file (best-effort) and stage a [ScriptOp.edit] mod for it. Same
  /// body the old "Edit existing" picker flow used, minus the picker.
  Future<void> _stageEdit(String? cache) async {
    // Capture the target + notifiers BEFORE any await: staging switches the
    // detail pane to _ModDetail, which disposes this state while the emit may
    // still be in flight — reading widget/ref afterwards would throw.
    final module = widget.module;
    final relPath = widget.relPath;
    final mods = ref.read(scriptModsProvider.notifier);
    final selection = ref.read(_selectedModuleProvider.notifier);
    setState(() => _busy = true);
    String asPath = '';
    if (cache != null) {
      final ffi = ModFfi(ref.read(coreServiceProvider));
      try {
        final src = await ffi.scriptEmitModule(cache, module.name);
        final dir = await Directory.systemTemp.createTemp('goremod_emit_');
        final f = File(p.join(dir.path, p.basename(relPath)));
        await f.create(recursive: true);
        await f.writeAsString(src);
        asPath = f.path;
      } catch (_) {/* leave asPath empty; user can pick a .as in the detail pane */}
    }
    mods.setMod(ScriptMod(
        op: ScriptOp.edit,
        moduleName: module.name,
        relPath: relPath,
        asPath: asPath));
    // Selection already points at relPath; keep it explicit so the detail pane
    // lands on the freshly staged mod even if the user clicked elsewhere.
    selection.state = relPath;
    if (mounted) setState(() => _busy = false);
  }
}

/// Collapsible bottom panel listing every staged script mod, plus the
/// "Add new .as" entry point for brand-new modules (which have no vanilla tree
/// leaf, so they appear only here).
class _StagedScriptsPanel extends ConsumerWidget {
  const _StagedScriptsPanel();

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final theme = Theme.of(context);
    final scheme = theme.colorScheme;
    final state = ref.watch(scriptModsProvider);
    final selectedKey = ref.watch(_selectedModuleProvider);
    final entries = state.entries;

    return Theme(
      data: theme.copyWith(dividerColor: Colors.transparent),
      child: ExpansionTile(
        initiallyExpanded: false,
        leading: const Icon(Icons.layers),
        title: Row(
          children: [
            Expanded(child: Text('Staged script mods (${entries.length})')),
            TextButton.icon(
              icon: const Icon(Icons.add, size: 18),
              label: const Text('Add new .as'),
              onPressed: () => _addNew(context, ref),
            ),
          ],
        ),
        childrenPadding: const EdgeInsets.only(bottom: 8),
        children: [
          if (entries.isEmpty)
            const Padding(
              padding: EdgeInsets.symmetric(horizontal: 16, vertical: 8),
              child: Align(
                alignment: Alignment.centerLeft,
                child: Text('No script mods staged yet'),
              ),
            )
          else
            for (final m in entries)
              ListTile(
                dense: true,
                selected: m.key == selectedKey,
                leading: Icon(m.op == ScriptOp.add
                    ? Icons.add_box_outlined
                    : Icons.edit_note_outlined),
                title: Text(m.moduleName,
                    maxLines: 1, overflow: TextOverflow.ellipsis),
                subtitle: Builder(builder: (_) {
                  final fresh = scriptCompileFresh(m);
                  return Text.rich(
                    TextSpan(children: [
                      TextSpan(
                          text: m.relPath,
                          style: TextStyle(color: scheme.onSurfaceVariant)),
                      const TextSpan(text: '  ·  '),
                      TextSpan(
                        text: fresh
                            ? 'compiled'
                            : 'not compiled / edited — recompile',
                        style: TextStyle(
                            color: fresh ? scheme.primary : scheme.error),
                      ),
                    ]),
                    maxLines: 1,
                    overflow: TextOverflow.ellipsis,
                    style: const TextStyle(fontSize: 12),
                  );
                }),
                trailing: IconButton(
                  icon: const Icon(Icons.delete_outline, size: 18),
                  tooltip: 'Remove',
                  onPressed: () =>
                      ref.read(scriptModsProvider.notifier).remove(m.key),
                ),
                onTap: () =>
                    ref.read(_selectedModuleProvider.notifier).state = m.key,
              ),
        ],
      ),
    );
  }

  Future<void> _addNew(BuildContext context, WidgetRef ref) async {
    final file = await openFile(acceptedTypeGroups: const [
      XTypeGroup(label: 'AngelScript', extensions: ['as']),
    ]);
    if (file == null) return;
    final base = p.basename(file.path);
    if (!context.mounted) return;
    // A module may need to live in a subdirectory (e.g. AI/Foo.as). Ask for the game-relative
    // path so it isn't flattened to the tree root; default to the picked file's basename.
    final entered = await _promptRelPath(context, base);
    if (entered == null) return; // cancelled: abort the add
    // Normalize backslashes and strip a leading slash; fall back to the basename if empty.
    var relPath = entered.replaceAll('\\', '/').replaceAll(RegExp(r'^/+'), '').trim();
    if (relPath.isEmpty) relPath = base;
    // The module name is the final segment's basename-without-extension.
    final name = p.basenameWithoutExtension(p.basename(relPath));
    // The game confirms the real module name when the mod is compiled (it may resolve a different
    // name and re-key the staged mod).
    final mod = ScriptMod(op: ScriptOp.add, moduleName: name, relPath: relPath, asPath: file.path);
    ref.read(scriptModsProvider.notifier).setMod(mod);
    ref.read(_selectedModuleProvider.notifier).state = mod.key;
  }

  Future<String?> _promptRelPath(BuildContext context, String base) {
    final controller = TextEditingController(text: base);
    return showDialog<String>(
      context: context,
      builder: (ctx) => AlertDialog(
        title: const Text('Module path'),
        content: TextField(
          controller: controller,
          autofocus: true,
          decoration: const InputDecoration(
            hintText: 'Game-relative path, e.g. AI/Foo.as',
            isDense: true,
          ),
          onSubmitted: (v) => Navigator.pop(ctx, v),
        ),
        actions: [
          TextButton(onPressed: () => Navigator.pop(ctx), child: const Text('Cancel')),
          FilledButton(
            onPressed: () => Navigator.pop(ctx, controller.text),
            child: const Text('Add'),
          ),
        ],
      ),
    );
  }
}

/// Key/value info row shared by the staged and vanilla detail panes.
Widget _kvRow(String k, String v) => Padding(
      padding: const EdgeInsets.symmetric(vertical: 2),
      child: Row(crossAxisAlignment: CrossAxisAlignment.start, children: [
        SizedBox(width: 90, child: Text(k, style: const TextStyle(fontWeight: FontWeight.w600))),
        Expanded(child: Text(v, style: const TextStyle(fontFamily: 'Consolas', fontSize: 12))),
      ]),
    );

class _ModDetail extends ConsumerStatefulWidget {
  const _ModDetail({super.key, required this.mod});
  final ScriptMod mod;
  @override
  ConsumerState<_ModDetail> createState() => _ModDetailState();
}

class _ModDetailState extends ConsumerState<_ModDetail> {
  bool _busy = false;
  String? _status;
  bool _error = false;

  @override
  Widget build(BuildContext context) {
    final mod = widget.mod;
    final scheme = Theme.of(context).colorScheme;
    return Padding(
      padding: const EdgeInsets.all(16),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Text(mod.moduleName, style: Theme.of(context).textTheme.titleMedium),
          Text(mod.op == ScriptOp.add ? 'New module' : 'Edit existing module',
              style: TextStyle(color: scheme.onSurfaceVariant)),
          const SizedBox(height: 12),
          _kvRow('Module', mod.moduleName),
          _kvRow('Path', mod.relPath),
          _kvRow('Source', mod.asPath.isEmpty ? '(none — pick a .as)' : p.basename(mod.asPath)),
          _kvRow('Compiled', scriptCompileFresh(mod)
              ? p.basename(mod.miniPath)
              : (mod.compiled ? 'not compiled / edited — recompile' : 'no')),
          const SizedBox(height: 12),
          Wrap(
            spacing: 8,
            children: [
              OutlinedButton.icon(
                icon: const Icon(Icons.file_open_outlined, size: 18),
                label: const Text('Choose .as'),
                onPressed: _busy ? null : _pickSource,
              ),
              FilledButton.icon(
                icon: const Icon(Icons.build_outlined, size: 18),
                label: const Text('Compile'),
                onPressed: (_busy || mod.asPath.isEmpty) ? null : _compile,
              ),
            ],
          ),
          if (_busy) const Padding(
            padding: EdgeInsets.symmetric(vertical: 8), child: LinearProgressIndicator()),
          if (_status != null)
            Padding(
              padding: const EdgeInsets.only(top: 8),
              child: Text(_status!,
                  style: TextStyle(color: _error ? scheme.error : scheme.onSurfaceVariant)),
            ),
        ],
      ),
    );
  }

  Future<void> _pickSource() async {
    // Capture the target mod + notifier BEFORE the await. With the per-mod Key, switching
    // selection during the file picker disposes this state; reading widget.mod/ref afterwards
    // would target the wrong mod (or throw on a disposed ref).
    final mod = widget.mod;
    final notifier = ref.read(scriptModsProvider.notifier);
    final file = await openFile(acceptedTypeGroups: const [
      XTypeGroup(label: 'AngelScript', extensions: ['as']),
    ]);
    if (file == null) return;
    // Changing the source invalidates any prior compile (clears mini + hash). Operate on the
    // captured mod.
    notifier.setMod(mod.withSource(file.path));
  }

  Future<void> _compile() async {
    // Capture the target mod + every provider handle BEFORE any await. With the per-mod Key,
    // a selection change during the (long) compile disposes this state, so reading widget.mod
    // or ref afterwards would write the result to the wrong mod (or throw on the disposed ref).
    final mod = widget.mod;
    final notifier = ref.read(scriptModsProvider.notifier);
    final gameRoot = gameRootFromExe(ref.read(gameExePathProvider));
    final ffi = ModFfi(ref.read(coreServiceProvider));
    if (gameRoot == null) {
      setState(() { _error = true; _status = 'Set the game path in Settings to compile.'; });
      return;
    }
    setState(() { _busy = true; _error = false; _status = 'Compiling via game…'; });
    try {
      final work = await Directory.systemTemp.createTemp('goremod_as_compile_');
      final r = await ffi.scriptCompile(
        gameDir: gameRoot,
        op: scriptOpToString(mod.op),
        moduleName: mod.moduleName,
        relPath: mod.relPath,
        asPath: mod.asPath,
        workDir: work.path,
      );
      final mini = r['mini_path'] as String;
      final resolvedName = (r['module'] as String?) ?? mod.moduleName;
      // Fingerprint the .as that was just compiled (using the CAPTURED mod) so a later edit to the
      // source reads as not-fresh. IO failure => empty hash, which scriptCompileFresh treats as
      // not-fresh (safe: blocks deploy until a clean recompile).
      final hash = () {
        try {
          return fnv1aHex(File(mod.asPath).readAsBytesSync());
        } catch (_) {
          return '';
        }
      }();
      // The user may have removed this mod (or cleared the list) while the game was compiling.
      // Don't resurrect a deleted mod with the late result — discard it instead. Check via the
      // captured notifier (using ref here could throw if this state was disposed). Reading the
      // notifier's protected `state` directly is intentional — `ref` is off-limits post-dispose.
      // ignore: invalid_use_of_protected_member, invalid_use_of_visible_for_testing_member
      if (!notifier.state.items.containsKey(mod.key)) {
        if (mounted) setState(() { _status = 'Compiled, but the mod was removed — discarded.'; });
        return;
      }
      // The key is relPath (stable across compile); only moduleName may change as the regen
      // resolves the real name. So just update in place under the SAME key — no re-key needed.
      final updated = ScriptMod(
        op: mod.op, moduleName: resolvedName, relPath: mod.relPath,
        asPath: mod.asPath, miniPath: mini, compiledHash: hash);
      notifier.setMod(updated);
      // Selection stores mod.key (relPath) and is unchanged by the compile, so it stays valid.
      // Be honest when fingerprinting the .as failed (hash == ''): compiledHash is empty, so
      // scriptCompileFresh is false and Build/Deploy stays disabled — don't claim "Compiled ✓".
      if (mounted) {
        setState(() => _status = hash.isEmpty
            ? 'Compiled, but could not fingerprint the source — re-pick or edit the .as to enable deploy.'
            : 'Compiled ✓');
      }
    } catch (e) {
      if (mounted) setState(() { _error = true; _status = '$e'; });
    } finally {
      if (mounted) setState(() => _busy = false);
    }
  }
}
