import 'dart:convert';
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
import '../domain/script_compile_install_state_provider.dart';
import '../domain/script_compile_report.dart';
import '../domain/script_mods_notifier.dart';
import '../domain/script_modules_provider.dart';
import 'script_compile_install_state_banner.dart';

/// Selected script path: a browse TREE path (tree/flat-list taps) or a staged
/// mod's key = REAL relPath (staged-panel taps, staging). The two spaces are
/// identical except for collision-disambiguated leaves; `_selectedTreePath`
/// maps a real relPath back onto its first owning leaf for highlighting.
///
/// App-scoped and deliberately SHARED between the main Scripts tab and the
/// Changes>Scripts embed — both views point at the same modules, and either
/// may set the selection before the other ever builds. So NO view resets it
/// on mount (a main-tab initState reset used to clobber a selection made in
/// the embed before the main tab first built). Stale selections are handled
/// at RENDER time instead: after a game-path change (GamePathScope remounts
/// the tab while scriptModulesProvider reloads the new install's list) a
/// selection that resolves to neither a staged key nor a current module
/// highlights no leaf and `_detail` falls back to the action-less
/// placeholder. A stale relPath that also exists in the new install simply
/// selects that install's module — a valid selection there.
final _selectedModuleProvider = StateProvider<String?>((ref) => null);

/// Game-relative path for a vanilla module. Some cache entries have no recorded
/// file — fall back to `<name>.as` at the tree root (the same rule staging uses,
/// so tree paths and staged-mod keys line up).
String _moduleRelPath(ScriptModuleInfo m) =>
    m.file.isEmpty ? '${m.name}.as' : m.file;

/// A vanilla module plus its (possibly disambiguated) browse path and lowered
/// search keys, precomputed once per modules load so an active search doesn't
/// re-lowercase ~7k names on every rebuild.
class _ModuleEntry {
  const _ModuleEntry(this.module, this.treePath, this.nameLc, this.pathLc);
  final ScriptModuleInfo module;
  final String treePath;
  final String nameLc;
  final String pathLc;
}

class ScriptTab extends ConsumerStatefulWidget {
  const ScriptTab({super.key, this.onlyStaged = false});

  /// Changes-tab mode: the left browser (tree, flat search list, count
  /// caption) shows only modules whose REAL relPath is a staged key. Staged
  /// 'add' mods have no vanilla leaf, so they stay reachable via the staged
  /// bottom panel (unchanged). Default false = the full vanilla browser.
  final bool onlyStaged;

  @override
  ConsumerState<ScriptTab> createState() => _ScriptTabState();
}

class _ScriptTabState extends ConsumerState<ScriptTab> {
  String _query = '';
  final TextEditingController _searchController = TextEditingController();

  // Identity-stable tree-path list + treePath→module lookup + lowered search
  // entries, rebuilt only when the modules LIST identity changes (i.e. the
  // provider reloaded). PathTreeBrowser caches its built tree by list identity,
  // so passing a fresh list per build would rebuild the ~7k-leaf tree every
  // frame.
  List<ScriptModuleInfo>? _cacheSource;
  List<String>? _treePaths;
  Map<String, ScriptModuleInfo>? _byTreePath;
  // Real relPath → its FIRST owning leaf's tree path (colliding modules share
  // one real path; the first occurrence keeps it pristine).
  Map<String, String>? _treePathByRelPath;
  List<_ModuleEntry>? _searchEntries;
  // Search-match memo, valid per (query, modules identity); cleared on reload.
  String? _matchQuery;
  List<_ModuleEntry>? _matchResult;
  // Marked-leaf memo, valid per (staged items identity, modules identity).
  Object? _markedSource;
  Set<String>? _markedTreePaths;
  // onlyStaged: the filtered browse view (identity-stable tree-path list +
  // search entries restricted to staged real relPaths), memoized per (staged
  // items identity, modules identity — the latter reset by [_refreshCaches]).
  Object? _stagedFilterSource;
  List<String>? _stagedTreePaths;
  List<_ModuleEntry>? _stagedSearchEntries;

  @override
  void didUpdateWidget(ScriptTab oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.onlyStaged != widget.onlyStaged) {
      // The match memo is keyed on the ACTIVE entries source (filtered vs
      // full), so a mode flip invalidates it even with query + data unchanged.
      _matchQuery = null;
      _matchResult = null;
    }
  }

  @override
  void dispose() {
    _searchController.dispose();
    super.dispose();
  }

  void _refreshCaches(List<ScriptModuleInfo> modules) {
    if (identical(_cacheSource, modules) && _treePaths != null) return;
    _cacheSource = modules;
    // relPaths are NOT guaranteed unique: two modules can share a `file`, and
    // the '<name>.as' empty-file fallback can collide with a real root-level
    // file. A plain map would silently last-win (one tree leaf for two modules,
    // Edit staging the wrong one) — so the FIRST occurrence keeps the pristine
    // path and later collisions get a display-only ' (n)' suffix. Staging
    // always uses the module's REAL relPath; only the browse path differs.
    final treePaths = <String>[];
    final byTreePath = <String, ScriptModuleInfo>{};
    final treePathByRelPath = <String, String>{};
    final entries = <_ModuleEntry>[];
    for (final m in modules) {
      final real = _moduleRelPath(m);
      var path = real;
      for (var n = 2; byTreePath.containsKey(path); n++) {
        path = _suffixedPath(real, n);
      }
      treePaths.add(path);
      byTreePath[path] = m;
      treePathByRelPath.putIfAbsent(real, () => path);
      entries.add(
        _ModuleEntry(m, path, m.name.toLowerCase(), path.toLowerCase()),
      );
    }
    _treePaths = treePaths;
    _byTreePath = byTreePath;
    _treePathByRelPath = treePathByRelPath;
    _searchEntries = entries;
    _matchQuery = null;
    _matchResult = null;
    _markedSource = null;
    _markedTreePaths = null;
    _stagedFilterSource = null;
    _stagedTreePaths = null;
    _stagedSearchEntries = null;
  }

  /// The tree leaf for the current selection. The selection store mixes key
  /// spaces — browser taps store TREE paths, while the staged panel and
  /// staging itself store REAL relPaths.
  ///
  /// Precedence: a key that is currently STAGED is a real relPath by
  /// construction, so it resolves through the relPath map FIRST — a
  /// pathological vanilla list can contain a real path that literally equals
  /// another module's generated ' (n)' collision leaf, and the tree-path
  /// pass-through would then highlight/open that OTHER module. Everything
  /// else: known tree paths pass through, real relPaths map to their first
  /// owning leaf (all colliding leaves share one staging key anyway, so "the
  /// first" is the only sensible highlight).
  String? _selectedTreePath(String? selectedKey, ScriptModsState staged) {
    if (selectedKey == null) return null;
    if (staged.items.containsKey(selectedKey)) {
      // Staged 'add' keys have no vanilla leaf → null (the detail pane still
      // resolves the mod through the staged-items lookup).
      return _treePathByRelPath![selectedKey];
    }
    if (_byTreePath!.containsKey(selectedKey)) return selectedKey;
    return _treePathByRelPath![selectedKey];
  }

  /// Tree paths whose module's REAL relPath is staged. Staged keys are real
  /// relPaths, so disambiguated leaves (and every leaf sharing a staged real
  /// path) need this indirection to show their marker. Memoized per (staged
  /// items identity, modules identity — the latter reset by [_refreshCaches]).
  Set<String> _markedFor(ScriptModsState staged) {
    if (!identical(_markedSource, staged.items) || _markedTreePaths == null) {
      _markedSource = staged.items;
      _markedTreePaths = {
        for (final e in _searchEntries!)
          if (staged.items.containsKey(_moduleRelPath(e.module))) e.treePath,
      };
    }
    return _markedTreePaths!;
  }

  /// onlyStaged: rebuild the filtered browse view — only modules whose REAL
  /// relPath is a staged key (derived from [_markedFor], so colliding leaves
  /// that share a staged real path all stay visible). Memoized per (staged
  /// items identity, modules identity): PathTreeBrowser rebuilds its tree per
  /// paths-list IDENTITY, so a fresh list every build would rebuild the tree
  /// every frame. The search-match memo is keyed on the entries this produces,
  /// so it resets alongside (un-staging during an active search must re-filter).
  void _refreshStagedFilter(ScriptModsState staged) {
    if (identical(_stagedFilterSource, staged.items) &&
        _stagedTreePaths != null) {
      return;
    }
    _stagedFilterSource = staged.items;
    final marked = _markedFor(staged);
    final entries = <_ModuleEntry>[
      for (final e in _searchEntries!)
        if (marked.contains(e.treePath)) e,
    ];
    _stagedSearchEntries = entries;
    _stagedTreePaths = [for (final e in entries) e.treePath];
    _matchQuery = null;
    _matchResult = null;
  }

  /// 'Dir/Foo.as' + n → 'Dir/Foo (n).as' (suffix before the extension).
  static String _suffixedPath(String path, int n) {
    final slash = path.lastIndexOf('/');
    final dir = slash < 0 ? '' : path.substring(0, slash + 1);
    final base = slash < 0 ? path : path.substring(slash + 1);
    final dot = base.lastIndexOf('.');
    return dot <= 0
        ? '$dir$base ($n)'
        : '$dir${base.substring(0, dot)} ($n)${base.substring(dot)}';
  }

  /// Matches for [query], memoized so rebuilds while a search is active (e.g.
  /// selection changes) don't re-filter + re-sort the ~7k entries. onlyStaged
  /// searches the FILTERED entries, so hits are staged modules only.
  List<_ModuleEntry> _matchesFor(String query) {
    final q = query.toLowerCase();
    if (_matchQuery == q && _matchResult != null) return _matchResult!;
    _matchQuery = q;
    final entries = widget.onlyStaged ? _stagedSearchEntries! : _searchEntries!;
    return _matchResult =
        (entries
            .where((e) => e.nameLc.contains(q) || e.pathLc.contains(q))
            .toList()
          ..sort((a, b) => a.pathLc.compareTo(b.pathLc)));
  }

  @override
  Widget build(BuildContext context) {
    final modulesAsync = ref.watch(scriptModulesProvider);
    final state = ref.watch(scriptModsProvider);
    final installSafety = ref.watch(scriptCompileInstallSafetyProvider);
    final selectedKey = ref.watch(_selectedModuleProvider);
    final scheme = Theme.of(context).colorScheme;

    return Column(
      children: [
        if (installSafety.showBlockingBanner)
          ScriptCompileInstallStateBanner(
            state: installSafety,
            onRecheck: () =>
                ref.read(scriptCompileInstallSafetyProvider.notifier).refresh(),
            onViewRecoveryReport: installSafety.recoveryReport == null
                ? null
                : () => showScriptCompileReportDialog(
                    context,
                    installSafety.recoveryReport!,
                  ),
          ),
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

  Widget _browser(
    List<ScriptModuleInfo> modules,
    ScriptModsState staged,
    String? selectedKey,
    ColorScheme scheme,
  ) {
    if (widget.onlyStaged) {
      // Changes-tab mode: browse only the staged slice of the vanilla list.
      // build watches scriptModsProvider, so un-staging refreshes this live.
      _refreshStagedFilter(staged);
    } else if (modules.isEmpty) {
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
    final treePaths = widget.onlyStaged ? _stagedTreePaths! : _treePaths!;
    if (treePaths.isEmpty) {
      // onlyStaged with nothing to browse: nothing staged, or only 'add' mods
      // (no vanilla leaf — those live in the staged panel below), or no module
      // list at all. Only reachable in onlyStaged mode: the full list is
      // non-empty here (the modules.isEmpty hint above returned already).
      return Center(
        child: Padding(
          padding: const EdgeInsets.all(16),
          child: Text(
            'No staged edits of vanilla modules.\n'
            'Staged new .as modules appear in the panel below.',
            textAlign: TextAlign.center,
            style: TextStyle(color: scheme.onSurfaceVariant),
          ),
        ),
      );
    }
    final matches = _query.isEmpty
        ? const <_ModuleEntry>[]
        : _matchesFor(_query);
    // Staged keys are REAL relPaths — map both the marker set and the selected
    // highlight into tree-path space so disambiguated leaves behave.
    final marked = _markedFor(staged);
    final selectedTreePath = _selectedTreePath(selectedKey, staged);
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
                // Offstage skips paint/hit-test/semantics but NOT focus
                // traversal — without this, Tab could reach the hidden tree's
                // tiles during a search.
                child: ExcludeFocus(
                  excluding: _query.isNotEmpty,
                  child: PathTreeBrowser(
                    paths: treePaths,
                    selectedPath: selectedTreePath,
                    onSelect: _select,
                    leafIcon: Icons.description_outlined,
                    markedPaths: marked,
                  ),
                ),
              ),
              if (_query.isNotEmpty)
                _flatList(matches, marked, selectedTreePath),
            ],
          ),
        ),
        Text(
          // Tree-path count (== one leaf per module, incl. disambiguated
          // ones); in onlyStaged mode both counts cover the staged slice only.
          _query.isEmpty
              ? '${treePaths.length} modules'
              : '${matches.length} match / ${treePaths.length} total',
          style: Theme.of(context).textTheme.bodySmall,
        ),
      ],
    );
  }

  Widget _flatList(
    List<_ModuleEntry> matches,
    Set<String> marked,
    String? selectedTreePath,
  ) {
    return ListView.builder(
      itemCount: matches.length,
      itemBuilder: (c, i) {
        final e = matches[i];
        return ListTile(
          dense: true,
          selected: e.treePath == selectedTreePath,
          title: Text(
            e.module.name,
            maxLines: 1,
            overflow: TextOverflow.ellipsis,
          ),
          subtitle: Text(
            e.treePath,
            maxLines: 1,
            overflow: TextOverflow.ellipsis,
          ),
          // [marked] is keyed by tree path but derived from the module's REAL
          // relPath (the staging key), so disambiguated hits mark correctly.
          trailing: marked.contains(e.treePath)
              ? const Icon(Icons.check, size: 16)
              : null,
          onTap: () => _select(e.treePath),
        );
      },
    );
  }

  // -- Detail pane ----------------------------------------------------------

  Widget _detail(
    ScriptModsState state,
    String? selectedKey,
    ColorScheme scheme,
  ) {
    final placeholder = Center(
      child: Text(
        'Select or add a script mod',
        style: TextStyle(color: scheme.onSurfaceVariant),
      ),
    );
    if (selectedKey == null) return placeholder;
    // Resolve the selection to its tree leaf's module (null for staged 'add'
    // keys with no vanilla leaf, and for dangling selections).
    final treePath = _selectedTreePath(selectedKey, state);
    final module = treePath == null ? null : _byTreePath?[treePath];
    // Staged lookup: the selection may be a staged key itself (a REAL relPath —
    // staged-panel taps, staging) or a TREE path, so also resolve through the
    // leaf module's real relPath (the staging key). Without that indirection a
    // collision-disambiguated leaf ('Foo (2).as') would miss the shared staged
    // mod, claim "vanilla", and its Edit would silently overwrite the staged
    // mod with a fresh vanilla emit.
    final staged =
        state.items[selectedKey] ??
        (module == null ? null : state.items[_moduleRelPath(module)]);
    if (staged != null) {
      // Key the detail pane to the selected mod so switching selection builds a
      // FRESH _ModDetailState — otherwise the old state (and its _busy/_status/
      // _error compile UI) is reused for the next mod.
      return _ModDetail(key: ValueKey(staged.key), mod: staged);
    }
    // onlyStaged (Changes embed): the selection is SHARED with the main tab,
    // so it can point at a module that is not staged at all (selected on the
    // main tab, or its mod was just un-staged). The filtered browser lists no
    // such entry — a vanilla editor here would contradict the view, so show
    // the placeholder instead. View-level only: the shared provider is NOT
    // cleared, the main tab keeps its selection.
    if (widget.onlyStaged) return placeholder;
    // Not staged: either a vanilla module (show info + Edit) or a dangling
    // selection (e.g. a staged 'add' that was removed — its path isn't in the
    // vanilla tree, so fall back to the placeholder).
    if (module == null) return placeholder;
    // Keyed by the TREE path so the emit-busy state resets when the selection
    // changes; the detail itself works on the module's REAL relPath (identical
    // except for collision-disambiguated leaves).
    return _VanillaModuleDetail(
      key: ValueKey(treePath),
      module: module,
      relPath: _moduleRelPath(module),
    );
  }
}

/// Detail pane for a vanilla (not yet staged) module: name + path info and an
/// Edit action that stages a [ScriptOp.edit] mod pre-filled with the module's
/// emitted source.
class _VanillaModuleDetail extends ConsumerStatefulWidget {
  const _VanillaModuleDetail({
    super.key,
    required this.module,
    required this.relPath,
  });
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
    return SingleChildScrollView(
      padding: const EdgeInsets.all(16),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Text(
            widget.module.name,
            style: Theme.of(context).textTheme.titleMedium,
          ),
          Text(
            'Vanilla module — not staged',
            style: TextStyle(color: scheme.onSurfaceVariant),
          ),
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
  /// a temp file and stage a [ScriptOp.edit] mod for it. Same body the old
  /// "Edit existing" picker flow used, minus the picker.
  Future<void> _stageEdit(String? cache) async {
    // Capture the target + notifiers/messenger BEFORE any await: staging
    // switches the detail pane to _ModDetail, which disposes this state while
    // the emit may still be in flight — reading widget/ref/context afterwards
    // would throw.
    final module = widget.module;
    final relPath = widget.relPath;
    final mods = ref.read(scriptModsProvider.notifier);
    final selection = ref.read(_selectedModuleProvider.notifier);
    final messenger = ScaffoldMessenger.of(context);
    setState(() => _busy = true);
    String asPath = '';
    if (cache != null) {
      final ffi = ModFfi(ref.read(coreServiceProvider));
      try {
        final src = await ffi.scriptEmitModule(cache, module.name);
        // Emit to a STABLE per-module path so re-clicking Edit overwrites
        // instead of leaking one fresh temp dir per click. The relPath hash
        // guards against two distinct relPaths sanitizing to the same name
        // (e.g. 'AI/Foo.as' vs a literal 'AI_Foo.as').
        final safe = relPath.replaceAll(RegExp(r'[\\/]+'), '_');
        final tag = fnv1aHex(utf8.encode(relPath)).substring(0, 8);
        final f = File(
          p.join(Directory.systemTemp.path, 'goremod_emit', '${tag}_$safe'),
        );
        await f.parent.create(recursive: true);
        await f.writeAsString(src);
        asPath = f.path;
      } catch (e) {
        // Emit failed: surface it and stage NOTHING — a silently staged edit
        // with an empty source would only fail later with no hint why.
        messenger.showSnackBar(
          SnackBar(content: Text('Could not emit ${module.name}: $e')),
        );
        if (mounted) setState(() => _busy = false);
        return;
      }
    }
    mods.setMod(
      ScriptMod(
        op: ScriptOp.edit,
        moduleName: module.name,
        relPath: relPath,
        asPath: asPath,
      ),
    );
    // Point the selection at the staged mod's key (the REAL relPath — for a
    // collision-disambiguated tree leaf this differs from the tree path).
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
            // Cap the entries area so a long staged list scrolls inside the
            // panel instead of growing unbounded under the page Column and
            // overflowing the tab (the old fixed-width _StagedList scrolled).
            // shrinkWrap keeps short lists at their natural height; the cap
            // only bites once the rows exceed it.
            ConstrainedBox(
              constraints: const BoxConstraints(maxHeight: 240),
              child: ListView.builder(
                shrinkWrap: true,
                itemCount: entries.length,
                itemBuilder: (context, i) =>
                    _stagedTile(ref, entries[i], selectedKey, scheme),
              ),
            ),
        ],
      ),
    );
  }

  Widget _stagedTile(
    WidgetRef ref,
    ScriptMod m,
    String? selectedKey,
    ColorScheme scheme,
  ) {
    final fresh = scriptCompileFresh(m);
    return ListTile(
      dense: true,
      selected: m.key == selectedKey,
      leading: Icon(
        m.op == ScriptOp.add
            ? Icons.add_box_outlined
            : Icons.edit_note_outlined,
      ),
      title: Text(m.moduleName, maxLines: 1, overflow: TextOverflow.ellipsis),
      subtitle: Text.rich(
        TextSpan(
          children: [
            TextSpan(
              text: m.relPath,
              style: TextStyle(color: scheme.onSurfaceVariant),
            ),
            const TextSpan(text: '  ·  '),
            TextSpan(
              text: fresh ? 'compiled' : 'not compiled / edited — recompile',
              style: TextStyle(color: fresh ? scheme.primary : scheme.error),
            ),
          ],
        ),
        maxLines: 1,
        overflow: TextOverflow.ellipsis,
        style: const TextStyle(fontSize: 12),
      ),
      trailing: IconButton(
        icon: const Icon(Icons.delete_outline, size: 18),
        tooltip: 'Remove',
        onPressed: () => ref.read(scriptModsProvider.notifier).remove(m.key),
      ),
      onTap: () => ref.read(_selectedModuleProvider.notifier).state = m.key,
    );
  }

  Future<void> _addNew(BuildContext context, WidgetRef ref) async {
    final file = await openFile(
      acceptedTypeGroups: const [
        XTypeGroup(label: 'AngelScript', extensions: ['as']),
      ],
    );
    if (file == null) return;
    final base = p.basename(file.path);
    if (!context.mounted) return;
    // A module may need to live in a subdirectory (e.g. AI/Foo.as). Ask for the game-relative
    // path so it isn't flattened to the tree root; default to the picked file's basename.
    final entered = await _promptRelPath(context, base);
    if (entered == null) return; // cancelled: abort the add
    // Normalize backslashes and strip a leading slash; fall back to the basename if empty.
    var relPath = entered
        .replaceAll('\\', '/')
        .replaceAll(RegExp(r'^/+'), '')
        .trim();
    if (relPath.isEmpty) relPath = base;
    // The module name is the final segment's basename-without-extension.
    final name = p.basenameWithoutExtension(p.basename(relPath));
    // The game confirms the real module name when the mod is compiled (it may resolve a different
    // name and re-key the staged mod).
    final mod = ScriptMod(
      op: ScriptOp.add,
      moduleName: name,
      relPath: relPath,
      asPath: file.path,
    );
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
          TextButton(
            onPressed: () => Navigator.pop(ctx),
            child: const Text('Cancel'),
          ),
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
  child: Row(
    crossAxisAlignment: CrossAxisAlignment.start,
    children: [
      SizedBox(
        width: 90,
        child: Text(k, style: const TextStyle(fontWeight: FontWeight.w600)),
      ),
      Expanded(
        child: Text(
          v,
          style: const TextStyle(fontFamily: 'Consolas', fontSize: 12),
        ),
      ),
    ],
  ),
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
  ScriptCompileReport? _compileReport;

  @override
  Widget build(BuildContext context) {
    final mod = widget.mod;
    final installSafety = ref.watch(scriptCompileInstallSafetyProvider);
    final visibleReport = installSafety.recoveryReport ?? _compileReport;
    final scheme = Theme.of(context).colorScheme;
    return SingleChildScrollView(
      padding: const EdgeInsets.all(16),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Text(mod.moduleName, style: Theme.of(context).textTheme.titleMedium),
          Text(
            mod.op == ScriptOp.add ? 'New module' : 'Edit existing module',
            style: TextStyle(color: scheme.onSurfaceVariant),
          ),
          const SizedBox(height: 12),
          _kvRow('Module', mod.moduleName),
          _kvRow('Path', mod.relPath),
          _kvRow(
            'Source',
            mod.asPath.isEmpty ? '(none — pick a .as)' : p.basename(mod.asPath),
          ),
          _kvRow(
            'Compiled',
            scriptCompileFresh(mod)
                ? p.basename(mod.miniPath)
                : (mod.compiled ? 'not compiled / edited — recompile' : 'no'),
          ),
          SwitchListTile.adaptive(
            contentPadding: EdgeInsets.zero,
            dense: true,
            title: const Text('Allow new symbols'),
            subtitle: Text(
              mod.op == ScriptOp.add
                  ? 'Required for a new module; keeps only its new class/function/name rows.'
                  : 'Enable only when this edit introduces a new class, function, global, or name.',
            ),
            value: mod.allowNewSymbols,
            onChanged: _busy
                ? null
                : (value) {
                    ref
                        .read(scriptModsProvider.notifier)
                        .setMod(mod.withAllowNewSymbols(value));
                    setState(() {
                      _error = false;
                      _status = 'Symbol policy changed — compile again.';
                      _compileReport = null;
                    });
                  },
          ),
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
                onPressed:
                    (_busy ||
                        mod.asPath.isEmpty ||
                        !installSafety.liveMutationAllowed)
                    ? null
                    : _compile,
              ),
            ],
          ),
          if (_busy)
            const Padding(
              padding: EdgeInsets.symmetric(vertical: 8),
              child: LinearProgressIndicator(),
            ),
          if (_status != null)
            Padding(
              padding: const EdgeInsets.only(top: 8),
              child: Text(
                _status!,
                style: TextStyle(
                  color: _error ? scheme.error : scheme.onSurfaceVariant,
                ),
              ),
            ),
          if (visibleReport != null)
            Padding(
              padding: const EdgeInsets.only(top: 8),
              child: OutlinedButton.icon(
                icon: const Icon(Icons.receipt_long_outlined, size: 18),
                label: const Text('Compiler report'),
                onPressed: () =>
                    showScriptCompileReportDialog(context, visibleReport),
              ),
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
    final file = await openFile(
      acceptedTypeGroups: const [
        XTypeGroup(label: 'AngelScript', extensions: ['as']),
      ],
    );
    if (file == null) return;
    // Changing the source invalidates any prior compile (clears mini + hash). Operate on the
    // captured mod.
    notifier.setMod(mod.withSource(file.path));
    if (mounted) {
      setState(() {
        _error = false;
        _status = 'Source changed — compile again.';
        _compileReport = null;
      });
    }
  }

  Future<void> _compile() async {
    // Capture the target mod + every provider handle BEFORE any await. With the per-mod Key,
    // a selection change during the (long) compile disposes this state, so reading widget.mod
    // or ref afterwards would write the result to the wrong mod (or throw on the disposed ref).
    final mod = widget.mod;
    final notifier = ref.read(scriptModsProvider.notifier);
    final installSafety = ref.read(scriptCompileInstallSafetyProvider.notifier);
    final gameRoot = gameRootFromExe(ref.read(gameExePathProvider));
    final ffi = ModFfi(ref.read(coreServiceProvider));
    if (gameRoot == null) {
      setState(() {
        _error = true;
        _status = 'Set the game path in Settings to compile.';
      });
      return;
    }
    final confirmed = await _confirmCompile();
    if (confirmed != true || !mounted) return;
    setState(() {
      _busy = true;
      _error = false;
      _status = 'Rechecking game installation safety…';
      if (ref.read(scriptCompileInstallSafetyProvider).recoveryReport == null) {
        _compileReport = null;
      }
    });
    Directory? work;
    try {
      final checked = await installSafety.refresh();
      if (!checked.liveMutationAllowed ||
          checked.gameRoot == null ||
          !p.equals(
            p.normalize(p.absolute(checked.gameRoot!)),
            p.normalize(p.absolute(gameRoot)),
          )) {
        if (mounted) {
          setState(() {
            _error = true;
            _status =
                'Compile blocked: close the game or resolve the recovery/inspection warning, then choose Recheck.';
          });
        }
        return;
      }
      if (mounted) {
        setState(
          () => _status =
              'Compiling standalone first; the game remains available as fallback…',
        );
      }
      work = await Directory.systemTemp.createTemp('goremod_as_compile_');
      final report = await ffi.scriptCompileReportV2(
        gameDir: gameRoot,
        op: scriptOpToString(mod.op),
        moduleName: mod.moduleName,
        relPath: mod.relPath,
        asPath: mod.asPath,
        workDir: work.path,
        compilerBackend: ScriptCompilerBackendMode.productDefault,
        allowNewSymbols: mod.allowNewSymbols,
      );
      installSafety.recordCompileReport(report, gameRoot: gameRoot);
      await installSafety.refresh();
      if (!report.compiled) {
        var cleanupWarning = '';
        if (!report.recoveryRequired) {
          try {
            await work.delete(recursive: true);
          } catch (_) {
            cleanupWarning =
                ' Temporary compiler workspace could not be removed: ${work.path}';
          }
        }
        if (mounted) {
          setState(() {
            if (!report.recoveryRequired) _compileReport = report;
            _error = true;
            _status = '${_failedCompileSummary(report)}$cleanupWarning';
          });
        }
        return;
      }
      final mini = report.miniPath!;
      final resolvedName = report.module!;
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
        try {
          await work.delete(recursive: true);
        } catch (_) {
          // The result is intentionally discarded. A leftover temp workspace is preferable to
          // resurrecting a removed mod or deleting any path outside the directory we created.
        }
        if (mounted) {
          setState(() {
            _status = 'Compiled, but the mod was removed — discarded.';
          });
        }
        return;
      }
      // The key is relPath (stable across compile); only moduleName may change as the regen
      // resolves the real name. So just update in place under the SAME key — no re-key needed.
      final updated = ScriptMod(
        op: mod.op,
        moduleName: resolvedName,
        relPath: mod.relPath,
        asPath: mod.asPath,
        allowNewSymbols: mod.allowNewSymbols,
        miniPath: mini,
        compiledHash: hash,
      );
      notifier.setMod(updated);
      // Selection stores mod.key (relPath) and is unchanged by the compile, so it stays valid.
      // Be honest when fingerprinting the .as failed (hash == ''): compiledHash is empty, so
      // scriptCompileFresh is false and Build/Deploy stays disabled — don't claim "Compiled ✓".
      if (mounted) {
        setState(() {
          _compileReport = report;
          _status = hash.isEmpty
              ? 'Compiled, but could not fingerprint the source — re-pick or edit the .as to enable deploy.'
              : _compiledSummary(report);
        });
      }
    } catch (e) {
      final checked = await installSafety.refresh();
      if (work != null && checked.liveMutationAllowed) {
        try {
          await work.delete(recursive: true);
        } catch (_) {}
      }
      if (mounted) {
        setState(() {
          _error = true;
          _status = '$e';
        });
      }
    } finally {
      if (mounted) setState(() => _busy = false);
    }
  }

  Future<bool?> _confirmCompile() => showDialog<bool>(
    context: context,
    builder: (dialogContext) => AlertDialog(
      title: const Text('Compile standalone first'),
      content: const Text(
        'Mod Studio first uses the qualified standalone compiler. If that compiler is unavailable '
        'or rejects its output, Mod Studio shows the reason and may fall back to the game compiler. '
        'Keep Gothic 1 Remake closed: the fallback temporarily stages a complete AngelScript tree '
        'in the game installation and then restores every touched path. Neither route loads or '
        'changes a save. If exact restoration cannot be proven, Mod Studio stops and shows '
        'recovery details.',
      ),
      actions: [
        TextButton(
          onPressed: () => Navigator.pop(dialogContext, false),
          child: const Text('Cancel'),
        ),
        FilledButton(
          onPressed: () => Navigator.pop(dialogContext, true),
          child: const Text('Compile'),
        ),
      ],
    ),
  );

  String _failedCompileSummary(ScriptCompileReport report) {
    if (report.recoveryRequired) {
      return 'Compilation stopped: exact game-install restoration could not be proven. Open the compiler report before another compile.';
    }
    final messages = report.diagnostics?.messages ?? const [];
    final firstErrors = messages.where(
      (message) => message.severity == ScriptCompilerDiagnosticSeverity.error,
    );
    if (firstErrors.isNotEmpty) {
      final diagnostic = firstErrors.first;
      return '${diagnostic.location}: ${diagnostic.message}';
    }
    final failure = report.failure?.message ?? 'Compilation failed.';
    final fallback = report.backend?.fallbackReason;
    return fallback == null
        ? failure
        : '$failure Standalone compiler fallback reason: ${fallback.detail}';
  }

  String _compiledSummary(ScriptCompileReport report) {
    final diagnostics = report.diagnostics!;
    final backend = report.backend;
    final backendSummary = switch (backend?.resultBackend) {
      ScriptCompilerBackendName.standalone =>
        'Compiled ✓ with the standalone compiler',
      ScriptCompilerBackendName.game when backend?.fallbackReason != null =>
        'Compiled ✓ with the game fallback — standalone compiler: ${backend!.fallbackReason!.detail}',
      ScriptCompilerBackendName.game => 'Compiled ✓ with the game compiler',
      null => 'Compiled ✓',
    };
    if (diagnostics.usedNormalFallback) {
      return '$backendSummary — diagnostics hook unavailable; normal diagnostic path used.';
    }
    final count = diagnostics.messages.length;
    if (count > 0) {
      final omitted = diagnostics.omitted == 0
          ? ''
          : ' (+${diagnostics.omitted} omitted)';
      return '$backendSummary — $count compiler message(s)$omitted.';
    }
    return backendSummary;
  }
}
