import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'app/domain/asset_entry_tracker.dart';
import 'app/domain/ui_settings.dart';
import 'app/game_paths.dart';
import 'app/ui/about_dialog.dart';
import 'app/ui/game_path_scope.dart';
import 'app/ui/keep_alive_tab.dart';
import 'app/ui/tab_entry_listener.dart';
import 'app/ui/window_chrome.dart';
import 'catalog/ui/items_tab.dart';
import 'core/mod_ffi.dart';
import 'core/providers.dart';
import 'audio/ui/audio_tab.dart';
import 'dialog/ui/dialoge_tab.dart';
import 'editor/domain/overrides_notifier.dart';
import 'editor/ui/changes_tab.dart';
import 'export/ui/build_deploy_dialog.dart';
import 'l10n/app_localizations.dart';
import 'loc/domain/loc_catalog_provider.dart';
import 'loc/domain/loc_notifier.dart';
import 'loc/ui/loc_extract_flow.dart';
import 'project/project_controller.dart';
import 'scripts/domain/script_modules_provider.dart';
import 'scripts/ui/script_tab.dart';
import 'settings/ui/settings_tab.dart';
import 'story/domain/story_workspace_launcher.dart';
import 'story/ui/story_workspace_flow.dart';
import 'textures/domain/texture_index_provider.dart';
import 'textures/ui/texture_tab.dart';

/// Main tab indices, matching the [TabBar] tab order in [HomePage].
const _texturesTabIndex = 3;
const _scriptsTabIndex = 4;
const _changesTabIndex = 5;

/// Entry refresh for the kept-alive main tabs (see the [TabEntryListener]
/// in [HomePage]): invalidates the install-bound data providers backing the
/// entered tab so it refetches — the pre-keep-alive freshness semantics.
///
/// Runs on EVERY settled tab entry, first entries included; whether an
/// entry actually refreshes is the session-wide [AssetEntryTracker]'s call.
/// A per-tab "first entry = fresh build, skip" shortcut would be wrong
/// here: the Changes tab embeds the same Textures/Scripts views, and while
/// its embed keeps the shared `autoDispose` provider alive a deploy,
/// undeploy, or game patch can stale the value before the standalone tab is
/// ever opened. Only the very first display of an asset kind ANYWHERE
/// skips the invalidate — that build creates the provider fresh, so
/// invalidating would double-fetch.
///
/// For the Changes tab, in parity with the standalone tab cases, only the
/// provider of the asset section it CURRENTLY displays is refreshed (no
/// over-fetch for sections that aren't on screen; nothing for
/// All/Items/Dialogs/Audio). Section entry INSIDE the Changes tab is
/// handled by [ChangesTab]'s own selection logic, against the same tracker.
///
/// Top-level so tests can exercise the real mapping against a real
/// [TabEntryListener] without pumping the FFI-heavy [HomePage].
void handleMainTabEntered(WidgetRef ref, int index) {
  final tracker = ref.read(assetEntryTrackerProvider);
  switch (index) {
    case _texturesTabIndex:
      if (tracker.shouldInvalidateOnEntry(AssetKind.textureIndex)) {
        ref.invalidate(textureIndexProvider);
      }
    case _scriptsTabIndex:
      if (tracker.shouldInvalidateOnEntry(AssetKind.scriptModules)) {
        ref.invalidate(scriptModulesProvider);
      }
    case _changesTabIndex:
      switch (ref.read(changesAssetSectionProvider)) {
        case ChangesAssetSection.textures:
          if (tracker.shouldInvalidateOnEntry(AssetKind.textureIndex)) {
            ref.invalidate(textureIndexProvider);
          }
        case ChangesAssetSection.scripts:
          if (tracker.shouldInvalidateOnEntry(AssetKind.scriptModules)) {
            ref.invalidate(scriptModulesProvider);
          }
        case null:
          break;
      }
  }
}

class HomePage extends ConsumerStatefulWidget {
  const HomePage({super.key});

  @override
  ConsumerState<HomePage> createState() => _HomePageState();
}

class _HomePageState extends ConsumerState<HomePage>
    with WidgetsBindingObserver {
  @override
  void initState() {
    super.initState();
    WidgetsBinding.instance.addObserver(this);
    // First-run, optional: after the first frame, if no localized text has
    // been extracted yet and the user hasn't been prompted before, offer to
    // extract it.
    WidgetsBinding.instance.addPostFrameCallback((_) => _maybeFirstRunPrompt());
    WidgetsBinding.instance.addPostFrameCallback(
      (_) => _maybeAutoDetectGamePath(),
    );
  }

  /// On first run, if no game path is set, auto-detect the Steam install and save it.
  Future<void> _maybeAutoDetectGamePath() async {
    if (ref.read(gameExePathProvider) != null) return;
    try {
      final exe = await ModFfi(ref.read(coreServiceProvider)).findGameExe();
      if (!mounted || exe == null) return;
      if (ref.read(gameExePathProvider) == null) {
        ref.read(gameExePathProvider.notifier).set(exe);
      }
    } catch (_) {
      // best-effort; user can still set it manually in Settings
    }
  }

  @override
  void dispose() {
    WidgetsBinding.instance.removeObserver(this);
    super.dispose();
  }

  @override
  void didChangeAppLifecycleState(AppLifecycleState state) {
    // Another tool (or `gore-cli loc extract`) may have written the shared
    // loc_catalog.json while this app was in the background. Re-read it on
    // resume so item names pick up a catalog that appeared after first load.
    if (state == AppLifecycleState.resumed) {
      ref.invalidate(locCatalogProvider);
    }
  }

  Future<void> _maybeFirstRunPrompt() async {
    if (ref.read(locExtractPromptedProvider)) return;
    final present = await ref.read(locProvider.notifier).status();
    // Only prompt when the catalog is definitively absent: a null status means
    // the query failed (e.g. core unavailable), where extraction can't work, so
    // don't nag with the dialog.
    if (!mounted || present != false) return;
    final shouldExtract = await showLocFirstRunDialog(context);
    if (!mounted || !shouldExtract) return;
    // Record only once the user actually chose to extract, so the prompt isn't
    // marked as shown when the dialog never appeared, and deferring ("Not now")
    // lets the optional prompt offer again on a later launch.
    ref.read(locExtractPromptedProvider.notifier).markPrompted();
    await runLocExtractFlow(context, ref);
  }

  void _snack(String message) {
    if (!mounted) return;
    ScaffoldMessenger.of(
      context,
    ).showSnackBar(SnackBar(content: Text(message)));
  }

  Future<void> _saveProject() async {
    try {
      final path = await saveProjectInteractive(ref);
      if (path != null) _snack('Saved project to $path');
    } catch (e) {
      _snack('Save failed: $e');
    }
  }

  Future<void> _saveProjectAs() async {
    try {
      final path = await saveProjectAsInteractive(ref);
      if (path != null) _snack('Saved project to $path');
    } catch (e) {
      _snack('Save failed: $e');
    }
  }

  // Unsaved = there is staged content AND it differs from the last saved/loaded project, so a
  // project that was just saved doesn't prompt to discard on New/Open.
  bool _hasUnsavedEdits() => hasUnsavedChanges(ref);

  /// Confirm before discarding staged (unsaved) edits. Returns true to proceed.
  Future<bool> _confirmDiscardIfDirty() async {
    if (!_hasUnsavedEdits()) return true;
    final ok = await showDialog<bool>(
      context: context,
      builder: (ctx) => AlertDialog(
        title: const Text('Discard unsaved changes?'),
        content: const Text(
          'You have staged edits that are not saved to a project. Continue and discard them?',
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.pop(ctx, false),
            child: const Text('Cancel'),
          ),
          FilledButton(
            onPressed: () => Navigator.pop(ctx, true),
            child: const Text('Discard'),
          ),
        ],
      ),
    );
    return ok ?? false;
  }

  Future<void> _newProject() async {
    if (await _confirmDiscardIfDirty()) await newProject(ref);
  }

  Future<void> _openProject() async {
    if (!await _confirmDiscardIfDirty()) return;
    try {
      final path = await openProjectInteractive(ref);
      if (path != null) _snack('Loaded project $path');
    } catch (e) {
      _snack('Open failed: $e');
    }
  }

  Future<void> _openStoryWorkspace(StoryWorkspaceFlowMode mode) async {
    final configuredGamePath = ref.read(gameExePathProvider);
    if (configuredGamePath == null || configuredGamePath.isEmpty) {
      _snack('Set the Gothic 1 Remake game path in Settings first.');
      return;
    }
    await runStoryWorkspaceFlow(
      context: context,
      mode: mode,
      // Pass the configured value through unchanged. The hardened Story
      // launcher owns exact install-root/executable resolution.
      configuredGamePath: configuredGamePath,
      launcher: ManagedStoryWorkspaceFlowLauncher(
        StoryWorkspaceLauncher(ModFfi(ref.read(coreServiceProvider))),
      ),
    );
  }

  @override
  Widget build(BuildContext context) {
    // Switching the model data source invalidates pending overrides and the
    // current selection: fields may be removed/renamed or enum backing values
    // may change, so exporting old assignments could be wrong. Clear both when
    // the dump source changes.
    ref.listen(dumpPathProvider, (prev, next) {
      if (prev != next) {
        ref.read(overridesProvider.notifier).clearAll();
        ref.read(selectedItemProvider.notifier).state = null;
      }
    });

    // Keep the AppBar gate on the same all-domain definition as project
    // save/discard handling. Duplicating the provider list here previously
    // left newly added domains (notably dialog topics) unreachable.
    final dirty = projectIsDirty(ref);
    // Keep Build/Deploy reachable when a game is configured even with no staged edits, so the
    // dialog's Undeploy (restore *.gore-bak) stays available to GUI users.
    final gameConfigured =
        gameRootFromExe(ref.watch(gameExePathProvider)) != null;
    final themeModeNotifier = ref.read(themeModeProvider.notifier);
    final scheme = Theme.of(context).colorScheme;
    final isDark = Theme.of(context).brightness == Brightness.dark;
    final l10n = AppLocalizations.of(context);

    final scaffold = Scaffold(
      appBar: AppBar(
        title: WindowDragArea(
          child: Row(
            children: [
              const SizedBox(width: 16),
              Image.asset('assets/gore_studio_icon.png', height: 32),
              const SizedBox(width: 10),
              const Text('GORE Mod Studio'),
              const Expanded(child: SizedBox()),
            ],
          ),
        ),
        titleSpacing: 0,
        centerTitle: false,
        scrolledUnderElevation: 0,
        actions: [
          IconButton(
            icon: Icon(isDark ? Icons.light_mode : Icons.dark_mode),
            tooltip: isDark ? l10n.lightMode : l10n.darkMode,
            onPressed: () {
              themeModeNotifier.setThemeMode(
                isDark ? ThemeMode.light : ThemeMode.dark,
              );
            },
          ),
          PopupMenuButton<String>(
            icon: const Icon(Icons.folder_open_outlined),
            tooltip: 'Project',
            onSelected: (value) async {
              switch (value) {
                case 'new':
                  await _newProject();
                case 'open':
                  await _openProject();
                case 'save':
                  await _saveProject();
                case 'saveAs':
                  await _saveProjectAs();
                case 'storyCreate':
                  await _openStoryWorkspace(StoryWorkspaceFlowMode.create);
                case 'storyOpen':
                  await _openStoryWorkspace(StoryWorkspaceFlowMode.open);
              }
            },
            itemBuilder: (_) => const <PopupMenuEntry<String>>[
              PopupMenuItem(value: 'new', child: Text('New project')),
              PopupMenuItem(value: 'open', child: Text('Open project…')),
              PopupMenuItem(value: 'save', child: Text('Save project')),
              PopupMenuItem(value: 'saveAs', child: Text('Save project as…')),
              PopupMenuDivider(),
              PopupMenuItem(
                key: Key('project-create-story-workspace'),
                value: 'storyCreate',
                child: Text('Create Story workspace (drafts)...'),
              ),
              PopupMenuItem(
                key: Key('project-open-story-workspace'),
                value: 'storyOpen',
                child: Text('Open Story workspace (drafts)...'),
              ),
            ],
          ),
          Padding(
            padding: const EdgeInsets.symmetric(horizontal: 8),
            child: FilledButton.icon(
              icon: const Icon(Icons.rocket_launch_outlined, size: 18),
              label: const Text('Build / Deploy'),
              onPressed: (dirty || gameConfigured)
                  ? () => showDialog(
                      context: context,
                      builder: (_) => const BuildDeployDialog(),
                    )
                  : null,
            ),
          ),
          IconButton(
            icon: const Icon(Icons.info_outline),
            tooltip: l10n.about,
            onPressed: () => showDialog<void>(
              context: context,
              builder: (_) => const GoreStudioAboutDialog(),
            ),
          ),
          const WindowControls(),
          const SizedBox(width: 8),
        ],
      ),
      body: DefaultTabController(
        length: 7,
        // KeepAliveTab keeps every tab (and its autoDispose providers)
        // mounted across switches, so the texture index / script module list
        // would go stale after a deploy, undeploy, or game patch. Entering
        // those tabs refetches (tracker-gated: only an asset kind's very
        // first display anywhere builds fresh instead) — the pre-keep-alive
        // freshness semantics — while the tabs' UI state survives.
        child: TabEntryListener(
          onTabEntered: (index) => handleMainTabEntered(ref, index),
          child: Column(
            children: [
              Container(
                color: scheme.surfaceContainerLowest,
                child: Row(
                  children: [
                    Expanded(
                      child: TabBar(
                        isScrollable: true,
                        // Material 3 defaults scrollable tab bars to a 52px
                        // leading inset (TabAlignment.startOffset); start flush
                        // with just a small gap instead.
                        tabAlignment: TabAlignment.start,
                        padding: const EdgeInsetsDirectional.only(start: 4),
                        tabs: [
                          Tab(
                            icon: const Icon(Icons.inventory_2_outlined),
                            text: l10n.tabItems,
                          ),
                          Tab(
                            icon: const Icon(Icons.forum_outlined),
                            text: l10n.tabDialogs,
                          ),
                          Tab(
                            icon: const Icon(Icons.audiotrack_outlined),
                            text: l10n.tabAudio,
                          ),
                          Tab(
                            icon: const Icon(Icons.texture),
                            text: l10n.tabTextures,
                          ),
                          Tab(
                            icon: const Icon(Icons.code),
                            text: l10n.tabScripts,
                          ),
                          Tab(
                            icon: const Icon(Icons.edit_note_outlined),
                            text: l10n.tabOverrides,
                          ),
                          Tab(
                            icon: const Icon(Icons.settings_outlined),
                            text: l10n.tabSettings,
                          ),
                        ],
                      ),
                    ),
                  ],
                ),
              ),
              Expanded(
                child: TabBarView(
                  children: [
                    // Items: catalog browser + field editor.
                    const KeepAliveTab(child: ItemsTab()),
                    // Dialoge: localized dialog/bark line editor.
                    const KeepAliveTab(child: DialogeTab()),
                    // Audio: FMOD bank sample browser + replacement.
                    // (GamePathScope: these three tabs' kept UI state is
                    // bound to the configured install and resets when the
                    // game path changes.)
                    const KeepAliveTab(child: GamePathScope(child: AudioTab())),
                    // Textures: texture asset browser + replacement.
                    const KeepAliveTab(
                      child: GamePathScope(child: TextureTab()),
                    ),
                    // AngelScript: stage .as mods, compile, splice.
                    const KeepAliveTab(
                      child: GamePathScope(child: ScriptTab()),
                    ),
                    // Changes: per-domain sidebar over all staged changes
                    // ("All" = the flat OverridesPanel list, other sections =
                    // the main-tab views filtered to staged entries).
                    const KeepAliveTab(child: ChangesTab()),
                    // Settings.
                    const KeepAliveTab(child: SettingsTab()),
                  ],
                ),
              ),
            ],
          ),
        ),
      ),
    );

    return CallbackShortcuts(
      bindings: <ShortcutActivator, VoidCallback>{
        const SingleActivator(LogicalKeyboardKey.keyS, control: true): () {
          _saveProject();
        },
      },
      child: scaffold,
    );
  }
}
