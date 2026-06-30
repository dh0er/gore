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
              : _ModDetail(mod: selected),
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
                        subtitle: Text(
                          m.compiled ? 'compiled' : 'not compiled — press Compile',
                          style: TextStyle(
                            color: m.compiled ? scheme.primary : scheme.error, fontSize: 12),
                        ),
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
    // Derive module name + rel path from the filename; the game confirms the real module name
    // when the mod is compiled (Task 11 result updates moduleName if needed).
    final base = p.basename(file.path);
    final name = p.basenameWithoutExtension(file.path);
    final mod = ScriptMod(op: ScriptOp.add, moduleName: name, relPath: base, asPath: file.path);
    ref.read(scriptModsProvider.notifier).setMod(mod);
    ref.read(_selectedModuleProvider.notifier).state = mod.key;
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
  const _ModDetail({required this.mod});
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
          _kv('Compiled', mod.compiled ? p.basename(mod.miniPath) : 'no'),
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
    final file = await openFile(acceptedTypeGroups: const [
      XTypeGroup(label: 'AngelScript', extensions: ['as']),
    ]);
    if (file == null) return;
    // Changing the source invalidates any prior compile.
    ref.read(scriptModsProvider.notifier)
        .setMod(widget.mod.withAsPath(file.path).withMiniPath(''));
  }

  Future<void> _compile() async {
    final gameRoot = gameRootFromExe(ref.read(gameExePathProvider));
    if (gameRoot == null) {
      setState(() { _error = true; _status = 'Set the game path in Settings to compile.'; });
      return;
    }
    setState(() { _busy = true; _error = false; _status = 'Compiling via game…'; });
    try {
      final work = await Directory.systemTemp.createTemp('goremod_as_compile_');
      final r = await ModFfi(ref.read(coreServiceProvider)).scriptCompile(
        gameDir: gameRoot,
        op: scriptOpToString(widget.mod.op),
        moduleName: widget.mod.moduleName,
        relPath: widget.mod.relPath,
        asPath: widget.mod.asPath,
        workDir: work.path,
      );
      final mini = r['mini_path'] as String;
      final resolvedName = (r['module'] as String?) ?? widget.mod.moduleName;
      // The compile may resolve the real module name (esp. for "add"); update + re-key.
      final updated = ScriptMod(
        op: widget.mod.op, moduleName: resolvedName, relPath: widget.mod.relPath,
        asPath: widget.mod.asPath, miniPath: mini);
      final notifier = ref.read(scriptModsProvider.notifier);
      if (resolvedName != widget.mod.moduleName) notifier.remove(widget.mod.key);
      notifier.setMod(updated);
      ref.read(_selectedModuleProvider.notifier).state = updated.key;
      if (mounted) setState(() => _status = 'Compiled ✓');
    } catch (e) {
      if (mounted) setState(() { _error = true; _status = '$e'; });
    } finally {
      if (mounted) setState(() => _busy = false);
    }
  }
}
