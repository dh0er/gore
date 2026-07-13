import 'dart:async';
import 'dart:io';

import 'package:file_selector/file_selector.dart';
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
import 'dataasset/ui/dataasset_lab.dart';
import 'audio/ui/audio_tab.dart';
import 'dialog/ui/dialoge_tab.dart';
import 'editor/domain/overrides_notifier.dart';
import 'editor/ui/changes_tab.dart';
import 'export/ui/build_deploy_dialog.dart';
import 'l10n/app_localizations.dart';
import 'loc/domain/loc_catalog_provider.dart';
import 'loc/domain/loc_notifier.dart';
import 'loc/ui/loc_extract_flow.dart';
import 'project/current_project_controller.dart';
import 'project/project_controller.dart';
import 'scripts/domain/script_modules_provider.dart';
import 'scripts/ui/script_tab.dart';
import 'settings/ui/settings_tab.dart';
import 'story/domain/story_workspace_launcher.dart';
import 'story/ui/story_workspace_flow.dart';
import 'textures/domain/texture_index_provider.dart';
import 'textures/ui/texture_tab.dart';

typedef ManagedRevision3DirectoryPicker =
    Future<String?> Function(String confirmButtonText);

/// Injectable selection boundary; opening and adoption remain owned by the
/// app-wide [CurrentProjectCoordinator].
final managedRevision3DirectoryPickerProvider =
    Provider<ManagedRevision3DirectoryPicker>(
      (ref) =>
          (confirmButtonText) =>
              getDirectoryPath(confirmButtonText: confirmButtonText),
    );

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
  bool _projectActionBusy = false;

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

  Future<void> _runProjectAction(Future<void> Function() action) async {
    if (_projectActionBusy) return;
    setState(() => _projectActionBusy = true);
    try {
      await action();
    } finally {
      if (mounted) setState(() => _projectActionBusy = false);
    }
  }

  Future<void> _saveProject() => _runProjectAction(() async {
    final current = ref.read(currentProjectCoordinatorProvider);
    final coordinator = ref.read(currentProjectCoordinatorProvider.notifier);
    final l10n = AppLocalizations.of(context);
    try {
      switch (current) {
        case LegacyCurrentProjectState(:final path):
          if (path == null) {
            await _saveLegacyProjectAs(coordinator);
            return;
          }
          final saved = await coordinator.saveCurrent();
          _snack(
            'Saved project to ${(saved as LegacyCurrentProjectState).path}',
          );
        case ManagedRevision3CurrentProjectState(:final requiresReopen):
          if (requiresReopen) {
            _snack(l10n.projectManagedRevision3VerifyBlocked);
            return;
          }
          final verified =
              await coordinator.saveCurrent()
                  as ManagedRevision3CurrentProjectState;
          _snack(
            l10n.projectManagedRevision3Verified(verified.head.snapshotSha256),
          );
        case NoCurrentProjectState():
          _snack('There is no current project to save.');
      }
    } catch (e) {
      if (current is ManagedRevision3CurrentProjectState) {
        _snack(l10n.projectManagedRevision3VerifyFailed('$e'));
      } else {
        _snack('Save failed: $e');
      }
    }
  });

  Future<void> _saveProjectAs() => _runProjectAction(() async {
    try {
      final coordinator = ref.read(currentProjectCoordinatorProvider.notifier);
      await _saveLegacyProjectAs(coordinator);
    } catch (e) {
      _snack('Save failed: $e');
    }
  });

  Future<void> _saveLegacyProjectAs(
    CurrentProjectCoordinator coordinator,
  ) async {
    final path = await pickProjectSavePath(ref);
    if (path == null) return;
    final saved = await coordinator.saveLegacyToPath(path);
    _snack('Saved project to ${saved.path}');
  }

  // Unsaved = there is staged content AND it differs from the last saved/loaded project, so a
  // project that was just saved doesn't prompt to discard on New/Open.
  bool _hasUnsavedEdits() {
    if (ref.read(currentProjectCoordinatorProvider)
        is! LegacyCurrentProjectState) {
      return false;
    }
    return hasUnsavedChanges(ref);
  }

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

  Future<void> _newProject() => _runProjectAction(() async {
    if (!await _confirmDiscardIfDirty()) return;
    try {
      final coordinator = ref.read(currentProjectCoordinatorProvider.notifier);
      final cleanupFailuresBefore = coordinator.terminalCleanupFailures.length;
      await coordinator.newLegacyProject();
      _showTransitionCleanupWarningIfAdded(coordinator, cleanupFailuresBefore);
    } catch (e) {
      _snack('New project failed: $e');
    }
  });

  Future<void> _openProject() => _runProjectAction(() async {
    if (!await _confirmDiscardIfDirty()) return;
    try {
      final path = await pickProjectOpenPath();
      if (path == null) return;
      final coordinator = ref.read(currentProjectCoordinatorProvider.notifier);
      final cleanupFailuresBefore = coordinator.terminalCleanupFailures.length;
      final opened = await coordinator.openLegacyFromPath(path);
      if (!_showTransitionCleanupWarningIfAdded(
        coordinator,
        cleanupFailuresBefore,
      )) {
        _snack('Loaded project ${opened.path}');
      }
    } catch (e) {
      _snack('Open failed: $e');
    }
  });

  Future<void> _openManagedRevision3Project() => _runProjectAction(() async {
    if (!await _confirmDiscardIfDirty() || !mounted) return;
    final l10n = AppLocalizations.of(context);
    try {
      final path = await ref.read(managedRevision3DirectoryPickerProvider)(
        l10n.projectOpenManagedRevision3,
      );
      if (path == null || !mounted) return;
      final coordinator = ref.read(currentProjectCoordinatorProvider.notifier);
      final cleanupFailuresBefore = coordinator.terminalCleanupFailures.length;
      final opened = await coordinator.openManagedRevision3(Directory(path));
      if (!_showTransitionCleanupWarningIfAdded(
        coordinator,
        cleanupFailuresBefore,
      )) {
        _snack(l10n.projectManagedRevision3Opened(opened.projectId));
      }
    } catch (e) {
      _snack(l10n.projectManagedRevision3OpenFailed('$e'));
    }
  });

  bool _showTransitionCleanupWarningIfAdded(
    CurrentProjectCoordinator coordinator,
    int failuresBefore,
  ) {
    if (coordinator.terminalCleanupFailures.length <= failuresBefore) {
      return false;
    }
    if (!mounted) return true;
    _snack(AppLocalizations.of(context).projectTransitionCleanupWarning);
    return true;
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
      if (prev != next &&
          ref.read(currentProjectCoordinatorProvider)
              is LegacyCurrentProjectState) {
        ref.read(overridesProvider.notifier).clearAll();
        ref.read(selectedItemProvider.notifier).state = null;
      }
    });

    final currentProject = ref.watch(currentProjectCoordinatorProvider);
    final legacyCurrent = currentProject is LegacyCurrentProjectState;
    final managedCurrent =
        currentProject is ManagedRevision3CurrentProjectState;
    final managedVerificationBlocked =
        currentProject is ManagedRevision3CurrentProjectState &&
        currentProject.requiresReopen;
    // Never read a hidden compatibility graph as build input while a managed
    // project is authoritative. Short-circuiting also drops those provider
    // subscriptions after the format switch.
    final dirty = legacyCurrent && projectIsDirty(ref);
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
                case 'openManagedRevision3':
                  await _openManagedRevision3Project();
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
            itemBuilder: (_) => <PopupMenuEntry<String>>[
              PopupMenuItem(
                value: 'new',
                enabled: !_projectActionBusy,
                child: const Text('New project'),
              ),
              PopupMenuItem(
                value: 'open',
                enabled: !_projectActionBusy,
                child: Text(l10n.projectOpenLegacy),
              ),
              PopupMenuItem(
                key: const Key('project-open-managed-revision3'),
                value: 'openManagedRevision3',
                enabled: !_projectActionBusy,
                child: Text(l10n.projectOpenManagedRevision3),
              ),
              const PopupMenuDivider(),
              PopupMenuItem(
                key: const Key('project-save'),
                value: 'save',
                enabled:
                    !_projectActionBusy &&
                    currentProject is! NoCurrentProjectState &&
                    !managedVerificationBlocked,
                child: Text(
                  managedCurrent
                      ? l10n.projectVerifyCurrentHead
                      : 'Save project',
                ),
              ),
              PopupMenuItem(
                key: const Key('project-save-as'),
                value: 'saveAs',
                enabled: !_projectActionBusy && legacyCurrent,
                child: const Text('Save project as…'),
              ),
              const PopupMenuDivider(),
              PopupMenuItem(
                key: const Key('project-create-story-workspace'),
                value: 'storyCreate',
                enabled: !_projectActionBusy && legacyCurrent,
                child: const Text('Create Story workspace (drafts)...'),
              ),
              PopupMenuItem(
                key: const Key('project-open-story-workspace'),
                value: 'storyOpen',
                enabled: !_projectActionBusy && legacyCurrent,
                child: const Text('Open Story workspace (drafts)...'),
              ),
            ],
          ),
          if (legacyCurrent)
            Padding(
              padding: const EdgeInsets.symmetric(horizontal: 8),
              child: FilledButton.icon(
                icon: const Icon(Icons.rocket_launch_outlined, size: 18),
                label: const Text('Build / Deploy'),
                onPressed: !_projectActionBusy && (dirty || gameConfigured)
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
      body: switch (currentProject) {
        ManagedRevision3CurrentProjectState() => _ManagedRevision3ProjectView(
          project: currentProject,
        ),
        NoCurrentProjectState() => const _NoCurrentProjectView(),
        LegacyCurrentProjectState() => DefaultTabController(
          length: 8,
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
                            const Tab(
                              icon: Icon(Icons.data_object_outlined),
                              text: 'DataAsset Lab',
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
                      const KeepAliveTab(
                        child: GamePathScope(child: AudioTab()),
                      ),
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
                      // Bounded offline DataAsset evidence (never stages edits).
                      const KeepAliveTab(child: DataAssetLab()),
                    ],
                  ),
                ),
              ],
            ),
          ),
        ),
      },
    );

    return CallbackShortcuts(
      bindings: <ShortcutActivator, VoidCallback>{
        const SingleActivator(LogicalKeyboardKey.keyS, control: true): () {
          unawaited(_saveProject());
        },
      },
      child: Focus(
        autofocus: true,
        debugLabel: 'GORE Mod Studio project shortcuts',
        child: scaffold,
      ),
    );
  }
}

class _ManagedRevision3ProjectView extends StatelessWidget {
  const _ManagedRevision3ProjectView({required this.project});

  final ManagedRevision3CurrentProjectState project;

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context);
    return Center(
      child: ConstrainedBox(
        constraints: const BoxConstraints(maxWidth: 860),
        child: Card(
          margin: const EdgeInsets.all(32),
          child: Padding(
            padding: const EdgeInsets.all(24),
            child: Column(
              key: const Key('managed-revision3-project-view'),
              mainAxisSize: MainAxisSize.min,
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(
                  l10n.projectManagedRevision3Title,
                  style: Theme.of(context).textTheme.headlineSmall,
                ),
                const SizedBox(height: 8),
                Text(l10n.projectManagedRevision3IdentityOnly),
                if (project.requiresReopen) ...[
                  const SizedBox(height: 16),
                  Container(
                    key: const Key('managed-project-requires-reopen-warning'),
                    padding: const EdgeInsets.all(12),
                    decoration: BoxDecoration(
                      color: Theme.of(context).colorScheme.errorContainer,
                      borderRadius: BorderRadius.circular(8),
                    ),
                    child: Row(
                      crossAxisAlignment: CrossAxisAlignment.start,
                      children: [
                        Icon(
                          Icons.warning_amber_rounded,
                          color: Theme.of(context).colorScheme.onErrorContainer,
                        ),
                        const SizedBox(width: 10),
                        Expanded(
                          child: Text(
                            l10n.projectManagedRevision3RequiresReopen,
                            style: TextStyle(
                              color: Theme.of(
                                context,
                              ).colorScheme.onErrorContainer,
                            ),
                          ),
                        ),
                      ],
                    ),
                  ),
                ],
                const SizedBox(height: 24),
                _ProjectFact(
                  label: l10n.projectRoot,
                  value: project.root.path,
                  valueKey: const Key('managed-project-root'),
                ),
                _ProjectFact(
                  label: l10n.projectId,
                  value: project.projectId,
                  valueKey: const Key('managed-project-id'),
                ),
                _ProjectFact(
                  label: l10n.projectRevision,
                  value: '${project.projectRevision}',
                  valueKey: const Key('managed-project-revision'),
                ),
                _ProjectFact(
                  label: l10n.projectHeadSha256,
                  value: project.head.snapshotSha256,
                  valueKey: const Key('managed-project-head'),
                ),
                _ProjectFact(
                  label: l10n.projectSnapshotBytes,
                  value: '${project.head.snapshotByteLength}',
                  valueKey: const Key('managed-project-head-bytes'),
                ),
              ],
            ),
          ),
        ),
      ),
    );
  }
}

class _ProjectFact extends StatelessWidget {
  const _ProjectFact({
    required this.label,
    required this.value,
    required this.valueKey,
  });

  final String label;
  final String value;
  final Key valueKey;

  @override
  Widget build(BuildContext context) => Padding(
    padding: const EdgeInsets.only(bottom: 14),
    child: Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Text(label, style: Theme.of(context).textTheme.labelLarge),
        const SizedBox(height: 2),
        SelectableText(value, key: valueKey),
      ],
    ),
  );
}

class _NoCurrentProjectView extends StatelessWidget {
  const _NoCurrentProjectView();

  @override
  Widget build(BuildContext context) =>
      Center(child: Text(AppLocalizations.of(context).projectNoCurrent));
}
