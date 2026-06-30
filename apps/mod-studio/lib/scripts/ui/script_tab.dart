import 'dart:io';

import 'package:file_selector/file_selector.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_riverpod/legacy.dart'; // StateProvider
import 'package:path/path.dart' as p;

import '../../app/domain/ui_settings.dart'; // gameExePathProvider
import '../../app/game_paths.dart'; // gameRootFromExe
import '../../core/mod_ffi.dart';
import '../../core/providers.dart';
import '../domain/script_mods_notifier.dart';
import '../domain/script_modules_provider.dart';

final _selectedModuleProvider = StateProvider<String?>((ref) => null);

class ScriptTab extends ConsumerWidget {
  const ScriptTab({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final state = ref.watch(scriptModsProvider);
    final selectedKey = ref.watch(_selectedModuleProvider);
    final selected = selectedKey == null ? null : state.items[selectedKey];
    final scheme = Theme.of(context).colorScheme;

    return Row(
      children: [
        SizedBox(
          width: 360,
          child: _StagedList(state: state, selectedKey: selectedKey),
        ),
        const VerticalDivider(width: 1),
        Expanded(
          child: selected == null
              ? Center(child: Text('Select or add a script mod',
                  style: TextStyle(color: scheme.onSurfaceVariant)))
              // Key the detail pane to the selected mod so switching selection builds a FRESH
              // _ModDetailState — otherwise the old state (and its _busy/_status/_error compile UI)
              // is reused for the next mod.
              : _ModDetail(key: ValueKey(selected.key), mod: selected),
        ),
      ],
    );
  }
}

class _StagedList extends ConsumerWidget {
  const _StagedList({required this.state, required this.selectedKey});
  final ScriptModsState state;
  final String? selectedKey;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final scheme = Theme.of(context).colorScheme;
    return Column(
      children: [
        Padding(
          padding: const EdgeInsets.all(8),
          child: Row(
            children: [
              Expanded(
                child: OutlinedButton.icon(
                  icon: const Icon(Icons.add, size: 18),
                  label: const Text('Add new'),
                  onPressed: () => _addNew(context, ref),
                ),
              ),
              const SizedBox(width: 8),
              Expanded(
                child: OutlinedButton.icon(
                  icon: const Icon(Icons.edit_outlined, size: 18),
                  label: const Text('Edit existing'),
                  onPressed: () => _editExisting(context, ref),
                ),
              ),
            ],
          ),
        ),
        const Divider(height: 1),
        Expanded(
          child: state.count == 0
              ? Center(child: Text('No script mods staged',
                  style: TextStyle(color: scheme.onSurfaceVariant)))
              : ListView(
                  children: [
                    for (final m in state.entries)
                      ListTile(
                        selected: m.key == selectedKey,
                        leading: Icon(m.op == ScriptOp.add ? Icons.add_box_outlined : Icons.edit_note_outlined),
                        title: Text(m.moduleName, maxLines: 1, overflow: TextOverflow.ellipsis),
                        subtitle: Builder(builder: (_) {
                          final fresh = scriptCompileFresh(m);
                          return Text(
                            fresh ? 'compiled' : 'not compiled / edited — recompile',
                            style: TextStyle(
                              color: fresh ? scheme.primary : scheme.error, fontSize: 12),
                          );
                        }),
                        trailing: IconButton(
                          icon: const Icon(Icons.remove_circle_outline, size: 18),
                          onPressed: () => ref.read(scriptModsProvider.notifier).remove(m.key),
                        ),
                        onTap: () => ref.read(_selectedModuleProvider.notifier).state = m.key,
                      ),
                  ],
                ),
        ),
      ],
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

  Future<void> _editExisting(BuildContext context, WidgetRef ref) async {
    final modules = await ref.read(scriptModulesProvider.future);
    if (!context.mounted) return;
    if (modules.isEmpty) {
      ScaffoldMessenger.of(context).showSnackBar(
        const SnackBar(content: Text('No vanilla modules — set the game path in Settings.')));
      return;
    }
    final picked = await showDialog<ScriptModuleInfo>(
      context: context,
      builder: (ctx) => _ModulePicker(modules: modules),
    );
    if (picked == null) return;
    // Pre-fill the editable .as by emitting the vanilla module to a temp file.
    final cache = scriptCachePath(ref);
    String asPath = '';
    if (cache != null) {
      try {
        final src = await ModFfi(ref.read(coreServiceProvider)).scriptEmitModule(cache, picked.name);
        final dir = await Directory.systemTemp.createTemp('goremod_emit_');
        final f = File(p.join(dir.path, p.basename(picked.file.isEmpty ? '${picked.name}.as' : picked.file)));
        await f.create(recursive: true);
        await f.writeAsString(src);
        asPath = f.path;
      } catch (_) {/* leave asPath empty; user can pick a file in the detail pane */}
    }
    final mod = ScriptMod(
      op: ScriptOp.edit, moduleName: picked.name,
      relPath: picked.file.isEmpty ? '${picked.name}.as' : picked.file, asPath: asPath);
    ref.read(scriptModsProvider.notifier).setMod(mod);
    ref.read(_selectedModuleProvider.notifier).state = mod.key;
  }
}

class _ModulePicker extends StatefulWidget {
  const _ModulePicker({required this.modules});
  final List<ScriptModuleInfo> modules;
  @override
  State<_ModulePicker> createState() => _ModulePickerState();
}

class _ModulePickerState extends State<_ModulePicker> {
  String _q = '';
  @override
  Widget build(BuildContext context) {
    final filtered = widget.modules
        .where((m) => m.name.toLowerCase().contains(_q.toLowerCase()))
        .take(200)
        .toList();
    return AlertDialog(
      title: const Text('Pick a module to edit'),
      content: SizedBox(
        width: 480,
        height: 420,
        child: Column(
          children: [
            TextField(
              decoration: const InputDecoration(hintText: 'Search modules', isDense: true),
              onChanged: (v) => setState(() => _q = v),
            ),
            const SizedBox(height: 8),
            Expanded(
              child: ListView(
                children: [
                  for (final m in filtered)
                    ListTile(
                      dense: true,
                      title: Text(m.name, maxLines: 1, overflow: TextOverflow.ellipsis),
                      onTap: () => Navigator.pop(context, m),
                    ),
                ],
              ),
            ),
          ],
        ),
      ),
      actions: [TextButton(onPressed: () => Navigator.pop(context), child: const Text('Cancel'))],
    );
  }
}

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
          _kv('Module', mod.moduleName),
          _kv('Path', mod.relPath),
          _kv('Source', mod.asPath.isEmpty ? '(none — pick a .as)' : p.basename(mod.asPath)),
          _kv('Compiled', scriptCompileFresh(mod)
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

  Widget _kv(String k, String v) => Padding(
        padding: const EdgeInsets.symmetric(vertical: 2),
        child: Row(crossAxisAlignment: CrossAxisAlignment.start, children: [
          SizedBox(width: 90, child: Text(k, style: const TextStyle(fontWeight: FontWeight.w600))),
          Expanded(child: Text(v, style: const TextStyle(fontFamily: 'Consolas', fontSize: 12))),
        ]),
      );

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
    final selNotifier = ref.read(_selectedModuleProvider.notifier);
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
      // The compile may resolve the real module name (esp. for "add"); update + re-key.
      final updated = ScriptMod(
        op: mod.op, moduleName: resolvedName, relPath: mod.relPath,
        asPath: mod.asPath, miniPath: mini, compiledHash: hash);
      if (resolvedName != mod.key) notifier.remove(mod.key);
      notifier.setMod(updated);
      // Only move the selection if this state is still mounted (i.e. still the active mod);
      // otherwise the user has navigated away and we mustn't yank their selection.
      if (mounted) selNotifier.state = updated.key;
      if (mounted) setState(() => _status = 'Compiled ✓');
    } catch (e) {
      if (mounted) setState(() { _error = true; _status = '$e'; });
    } finally {
      if (mounted) setState(() => _busy = false);
    }
  }
}
