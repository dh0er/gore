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
import 'dataasset/ui/dataasset_semantic_edit_panel.dart';
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
import 'project/revision3_content_library.dart';
import 'project/revision3_dataasset_authoring.dart';
import 'project/revision3_dataasset_stage_panel.dart';
import 'project/revision3_npc_authoring.dart';
import 'project/revision3_npc_wizard.dart';
import 'project/revision3_quest_authoring.dart';
import 'project/revision3_quest_wizard.dart';
import 'project/revision3_voice_authoring.dart';
import 'project/revision3_voice_build_dialog.dart';
import 'project/revision3_voice_target_dialog.dart';
import 'project/revision3_voice_wizard.dart';
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

/// Optional picker seam for alternate shells and deterministic widget tests.
/// Normal app use keeps the qualification-aware built-in NPC archetype picker.
final managedRevision3NpcArchetypeChooserProvider =
    Provider<Revision3NpcArchetypeChooser?>((ref) => null);

/// Optional picker seam for alternate shells and deterministic widget tests.
/// Normal app use delegates to the platform file picker owned by the panel.
final managedRevision3DataAssetPatchReceiptPickerProvider =
    Provider<Revision3DataAssetPatchReceiptPicker?>((ref) => null);

/// Optional seams for the guided managed-R3 DataAsset value wizard. Normal
/// app use keeps the native inspector and platform file pickers owned by the
/// wizard; tests and alternate shells can replace them independently.
final managedRevision3DataAssetSemanticInspectorProvider =
    Provider<DataAssetInspector?>((ref) => null);
final managedRevision3DataAssetSemanticUassetPickerProvider =
    Provider<DataAssetFilePicker?>((ref) => null);
final managedRevision3DataAssetSemanticUsmapPickerProvider =
    Provider<DataAssetFilePicker?>((ref) => null);
final managedRevision3DataAssetExtractReceiptPickerProvider =
    Provider<DataAssetExtractReceiptPicker?>((ref) => null);

/// Native, read-only verification boundary for the selected ExtractReceipt.
/// It exposes only the exact target and input seals needed to bind the guided
/// edit to the inspection; raw selector/value bytes stay inside the native
/// preparation lane.
final managedRevision3DataAssetExtractReceiptInspectorProvider =
    Provider<DataAssetExtractReceiptInspector>(
      (ref) =>
          (extractReceiptPath) => ModFfi(ref.read(coreServiceProvider))
              .authoringReadDataAssetExtractReceiptV2(
                extractReceiptPath: extractReceiptPath,
              ),
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
    final gameRoot = gameRootFromExe(ref.watch(gameExePathProvider));
    final gameConfigured = gameRoot != null;
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
            key: const Key('project-menu'),
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
          gameRoot: gameRoot,
          loadContentIndex: () => ref
              .read(currentProjectCoordinatorProvider.notifier)
              .readCurrentRevision3ContentIndex(),
          publishVoiceTake:
              ({
                required expectedProjectId,
                required expectedProjectRevision,
                required plan,
              }) {
                final configuredGameRoot = gameRoot;
                if (configuredGameRoot == null) {
                  throw StateError(
                    'Configure the Gothic 1 Remake installation before adding a Voice take.',
                  );
                }
                return ref
                    .read(currentProjectCoordinatorProvider.notifier)
                    .addCurrentRevision3VoiceTake(
                      expectedRoot: currentProject.root.path,
                      expectedProjectId: expectedProjectId,
                      expectedProjectRevision: expectedProjectRevision,
                      expectedHead: currentProject.head,
                      gameRoot: configuredGameRoot,
                      plan: plan,
                    );
              },
          publishVoiceTarget:
              ({
                required expectedProjectId,
                required expectedProjectRevision,
                required plan,
              }) {
                final configuredGameRoot = gameRoot;
                if (configuredGameRoot == null) {
                  throw StateError(
                    'Configure the Gothic 1 Remake installation before resolving a Voice target.',
                  );
                }
                return ref
                    .read(currentProjectCoordinatorProvider.notifier)
                    .resolveCurrentRevision3VoiceTarget(
                      expectedRoot: currentProject.root.path,
                      expectedProjectId: expectedProjectId,
                      expectedProjectRevision: expectedProjectRevision,
                      expectedHead: currentProject.head,
                      gameRoot: configuredGameRoot,
                      plan: plan,
                    );
              },
          buildVoiceBundle: (output) {
            final configuredGameRoot = gameRoot;
            if (configuredGameRoot == null) {
              throw StateError(
                'Configure the Gothic 1 Remake installation before building a Voice bundle.',
              );
            }
            return ref
                .read(currentProjectCoordinatorProvider.notifier)
                .buildCurrentRevision3Voice(
                  expectedRoot: currentProject.root.path,
                  expectedProjectId: currentProject.projectId,
                  expectedProjectRevision: currentProject.projectRevision,
                  expectedHead: currentProject.head,
                  gameRoot: configuredGameRoot,
                  output: output,
                );
          },
          pickVoiceBuildParent: () => ref.read(
            managedRevision3DirectoryPickerProvider,
          )('Choose Voice bundle parent'),
          loadDataAssetStages: () => ref
              .read(currentProjectCoordinatorProvider.notifier)
              .listCurrentRevision3DataAssetStages(
                expectedRoot: currentProject.root.path,
                expectedProjectId: currentProject.projectId,
                expectedProjectRevision: currentProject.projectRevision,
                expectedHead: currentProject.head,
              ),
          publishDataAssetStage: ({required patchReceiptPath}) => ref
              .read(currentProjectCoordinatorProvider.notifier)
              .addCurrentRevision3DataAssetStage(
                expectedRoot: currentProject.root.path,
                expectedProjectId: currentProject.projectId,
                expectedProjectRevision: currentProject.projectRevision,
                expectedHead: currentProject.head,
                patchReceiptPath: patchReceiptPath,
              ),
          publishDataAssetSemanticEdit: (intent) async {
            try {
              final publication = await ref
                  .read(currentProjectCoordinatorProvider.notifier)
                  .addCurrentRevision3DataAssetEdit(
                    expectedRoot: currentProject.root.path,
                    expectedProjectId: currentProject.projectId,
                    expectedProjectRevision: currentProject.projectRevision,
                    expectedHead: currentProject.head,
                    intent: intent,
                  );
              return DataAssetSemanticStagePublication(
                targetPath: publication.stage.targetPath,
                revision: publication.projectRevision,
              );
            } on Revision3DataAssetStaleCheckpointException {
              throw const DataAssetSemanticStageUnavailableException.staleCheckpoint();
            } on Revision3DataAssetRequiresReopenException {
              throw const DataAssetSemanticStageUnavailableException.requiresReopen();
            }
          },
          removeDataAssetStage: ({required targetPath}) => ref
              .read(currentProjectCoordinatorProvider.notifier)
              .removeCurrentRevision3DataAssetStage(
                expectedRoot: currentProject.root.path,
                expectedProjectId: currentProject.projectId,
                expectedProjectRevision: currentProject.projectRevision,
                expectedHead: currentProject.head,
                targetPath: targetPath,
              ),
          pickDataAssetPatchReceipt: ref.read(
            managedRevision3DataAssetPatchReceiptPickerProvider,
          ),
          inspectDataAssetSemanticEdit: ref.read(
            managedRevision3DataAssetSemanticInspectorProvider,
          ),
          pickDataAssetSemanticUasset: ref.read(
            managedRevision3DataAssetSemanticUassetPickerProvider,
          ),
          pickDataAssetSemanticUsmap: ref.read(
            managedRevision3DataAssetSemanticUsmapPickerProvider,
          ),
          pickDataAssetExtractReceipt: ref.read(
            managedRevision3DataAssetExtractReceiptPickerProvider,
          ),
          inspectDataAssetExtractReceipt: ref.read(
            managedRevision3DataAssetExtractReceiptInspectorProvider,
          ),
          loadNpcCatalog: ref.read(revision3NpcCatalogLoaderProvider),
          chooseNpcArchetype: ref.read(
            managedRevision3NpcArchetypeChooserProvider,
          ),
          publishNpcDraft: ({required gameRoot, required input}) => ref
              .read(currentProjectCoordinatorProvider.notifier)
              .createCurrentRevision3NpcDraft(
                expectedRoot: currentProject.root.path,
                expectedHead: currentProject.head,
                expectedProjectId: currentProject.projectId,
                expectedProjectRevision: currentProject.projectRevision,
                gameRoot: gameRoot,
                input: input,
              ),
          loadQuestCatalog: ref.read(revision3QuestCatalogLoaderProvider),
          publishQuestDraft: ({required gameRoot, required input}) => ref
              .read(currentProjectCoordinatorProvider.notifier)
              .createCurrentRevision3QuestDraft(
                expectedRoot: currentProject.root.path,
                expectedProjectId: currentProject.projectId,
                expectedProjectRevision: currentProject.projectRevision,
                expectedHead: currentProject.head,
                gameRoot: gameRoot,
                input: input,
              ),
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
  const _ManagedRevision3ProjectView({
    required this.project,
    required this.gameRoot,
    required this.loadContentIndex,
    required this.publishVoiceTake,
    required this.publishVoiceTarget,
    required this.buildVoiceBundle,
    required this.pickVoiceBuildParent,
    required this.loadDataAssetStages,
    required this.publishDataAssetStage,
    required this.publishDataAssetSemanticEdit,
    required this.removeDataAssetStage,
    required this.pickDataAssetPatchReceipt,
    required this.inspectDataAssetSemanticEdit,
    required this.pickDataAssetSemanticUasset,
    required this.pickDataAssetSemanticUsmap,
    required this.pickDataAssetExtractReceipt,
    required this.inspectDataAssetExtractReceipt,
    required this.loadNpcCatalog,
    required this.chooseNpcArchetype,
    required this.publishNpcDraft,
    required this.loadQuestCatalog,
    required this.publishQuestDraft,
  });

  final ManagedRevision3CurrentProjectState project;
  final String? gameRoot;
  final Revision3ContentIndexLoader loadContentIndex;
  final Revision3VoiceTechnicalPublisher publishVoiceTake;
  final Revision3VoiceTargetTechnicalPublisher publishVoiceTarget;
  final Revision3VoiceExactBuild buildVoiceBundle;
  final Revision3VoiceBuildParentDirectoryPicker pickVoiceBuildParent;
  final Revision3DataAssetStageLoader loadDataAssetStages;
  final Revision3DataAssetStagePublisher publishDataAssetStage;
  final DataAssetSemanticStagePublisher publishDataAssetSemanticEdit;
  final Revision3DataAssetStageRemover removeDataAssetStage;
  final Revision3DataAssetPatchReceiptPicker? pickDataAssetPatchReceipt;
  final DataAssetInspector? inspectDataAssetSemanticEdit;
  final DataAssetFilePicker? pickDataAssetSemanticUasset;
  final DataAssetFilePicker? pickDataAssetSemanticUsmap;
  final DataAssetExtractReceiptPicker? pickDataAssetExtractReceipt;
  final DataAssetExtractReceiptInspector inspectDataAssetExtractReceipt;
  final Revision3NpcCatalogLoader loadNpcCatalog;
  final Revision3NpcArchetypeChooser? chooseNpcArchetype;
  final Revision3NpcDraftPublisher publishNpcDraft;
  final Revision3QuestCatalogLoader loadQuestCatalog;
  final Revision3QuestDraftPublisher publishQuestDraft;

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context);
    return Column(
      key: const Key('managed-revision3-project-view'),
      children: [
        Card(
          margin: const EdgeInsets.fromLTRB(16, 16, 16, 8),
          child: Padding(
            padding: const EdgeInsets.fromLTRB(20, 16, 20, 10),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                LayoutBuilder(
                  builder: (context, constraints) {
                    final identity = Column(
                      crossAxisAlignment: CrossAxisAlignment.start,
                      children: [
                        Text(
                          l10n.projectManagedRevision3Title,
                          style: Theme.of(context).textTheme.headlineSmall,
                        ),
                        const SizedBox(height: 4),
                        Text(
                          project.requiresReopen
                              ? 'Last owned managed checkpoint. Reopen the project to verify its current content.'
                              : 'Exact-current managed identity and semantic project content.',
                        ),
                      ],
                    );
                    final actions = Wrap(
                      key: const Key('managed-project-actions'),
                      spacing: 8,
                      runSpacing: 8,
                      children: [
                        OutlinedButton.icon(
                          key: const Key('managed-open-settings'),
                          onPressed: () => _openSettings(context),
                          icon: const Icon(Icons.settings_outlined),
                          label: const Text('Settings'),
                        ),
                        FilledButton.icon(
                          key: const Key('managed-add-voice-take'),
                          onPressed: project.requiresReopen || gameRoot == null
                              ? null
                              : () => _openVoiceWizard(context),
                          icon: const Icon(Icons.record_voice_over_outlined),
                          label: const Text('Voice take'),
                        ),
                        PopupMenuButton<String>(
                          key: const Key('managed-voice-tools'),
                          enabled: !project.requiresReopen && gameRoot != null,
                          tooltip: 'Voice target and bundle tools',
                          icon: const Icon(Icons.graphic_eq_outlined),
                          onSelected: (value) {
                            switch (value) {
                              case 'target':
                                unawaited(_openVoiceTargetResolver(context));
                              case 'build':
                                unawaited(_openVoiceBuild(context));
                            }
                          },
                          itemBuilder: (_) => const [
                            PopupMenuItem(
                              key: Key('managed-resolve-voice-target'),
                              value: 'target',
                              child: ListTile(
                                contentPadding: EdgeInsets.zero,
                                leading: Icon(Icons.link_outlined),
                                title: Text('Resolve Voice target'),
                              ),
                            ),
                            PopupMenuItem(
                              key: Key('managed-build-voice-bundle'),
                              value: 'build',
                              child: ListTile(
                                contentPadding: EdgeInsets.zero,
                                leading: Icon(Icons.inventory_2_outlined),
                                title: Text('Build Voice bundle'),
                              ),
                            ),
                          ],
                        ),
                        FilledButton.icon(
                          key: const Key('managed-create-npc-draft'),
                          onPressed: project.requiresReopen || gameRoot == null
                              ? null
                              : () => _openNpcWizard(context),
                          icon: const Icon(Icons.person_add_alt_1_outlined),
                          label: const Text('New NPC'),
                        ),
                        FilledButton.icon(
                          key: const Key('managed-create-quest-draft'),
                          onPressed: project.requiresReopen || gameRoot == null
                              ? null
                              : () => _openQuestWizard(context),
                          icon: const Icon(Icons.assignment_add),
                          label: const Text('New Quest'),
                        ),
                      ],
                    );
                    if (constraints.maxWidth < 720) {
                      return Column(
                        crossAxisAlignment: CrossAxisAlignment.stretch,
                        children: [
                          identity,
                          const SizedBox(height: 12),
                          Align(
                            alignment: Alignment.centerLeft,
                            child: actions,
                          ),
                        ],
                      );
                    }
                    return Row(
                      crossAxisAlignment: CrossAxisAlignment.start,
                      children: [
                        Expanded(child: identity),
                        const SizedBox(width: 16),
                        actions,
                      ],
                    );
                  },
                ),
                if (gameRoot == null && !project.requiresReopen) ...[
                  const SizedBox(height: 8),
                  const Text(
                    'Configure the Gothic 1 Remake installation in Settings to author, resolve, or build Voice content and to create NPC and Quest drafts.',
                    key: Key('managed-quest-game-required'),
                  ),
                ],
                if (project.requiresReopen) ...[
                  const SizedBox(height: 12),
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
                const SizedBox(height: 14),
                Wrap(
                  spacing: 24,
                  runSpacing: 2,
                  children: [
                    SizedBox(
                      width: 360,
                      child: _ProjectFact(
                        label: l10n.projectRoot,
                        value: project.root.path,
                        valueKey: const Key('managed-project-root'),
                      ),
                    ),
                    SizedBox(
                      width: 300,
                      child: _ProjectFact(
                        label: l10n.projectId,
                        value: project.projectId,
                        valueKey: const Key('managed-project-id'),
                      ),
                    ),
                    SizedBox(
                      width: 160,
                      child: _ProjectFact(
                        label: l10n.projectRevision,
                        value: '${project.projectRevision}',
                        valueKey: const Key('managed-project-revision'),
                      ),
                    ),
                    SizedBox(
                      width: 460,
                      child: _ProjectFact(
                        label: l10n.projectHeadSha256,
                        value: project.head.snapshotSha256,
                        valueKey: const Key('managed-project-head'),
                      ),
                    ),
                    SizedBox(
                      width: 160,
                      child: _ProjectFact(
                        label: l10n.projectSnapshotBytes,
                        value: '${project.head.snapshotByteLength}',
                        valueKey: const Key('managed-project-head-bytes'),
                      ),
                    ),
                  ],
                ),
              ],
            ),
          ),
        ),
        Expanded(
          child: project.requiresReopen
              ? Center(
                  child: Padding(
                    padding: const EdgeInsets.all(24),
                    child: Column(
                      mainAxisSize: MainAxisSize.min,
                      children: [
                        const Icon(Icons.lock_reset_outlined, size: 36),
                        const SizedBox(height: 10),
                        const Text(
                          'Reopen the managed project before reading its content.',
                          textAlign: TextAlign.center,
                        ),
                      ],
                    ),
                  ),
                )
              : DefaultTabController(
                  length: 2,
                  child: Column(
                    children: [
                      const TabBar(
                        key: Key('managed-revision3-workspace-tabs'),
                        tabs: [
                          Tab(
                            key: Key('managed-revision3-library-tab'),
                            icon: Icon(Icons.library_books_outlined),
                            text: 'Library',
                          ),
                          Tab(
                            key: Key('managed-revision3-dataasset-tab'),
                            icon: Icon(Icons.data_object_outlined),
                            text: 'DataAsset edits',
                          ),
                        ],
                      ),
                      Expanded(
                        child: TabBarView(
                          children: [
                            Revision3ContentLibrary(
                              projectRoot: project.root.path,
                              projectId: project.projectId,
                              projectRevision: project.projectRevision,
                              projectHeadCanonicalJson:
                                  project.head.canonicalJson,
                              load: loadContentIndex,
                            ),
                            Revision3DataAssetStagePanel(
                              projectRoot: project.root.path,
                              projectId: project.projectId,
                              projectRevision: project.projectRevision,
                              projectHead: project.head,
                              load: loadDataAssetStages,
                              publish: publishDataAssetStage,
                              publishSemanticEdit: publishDataAssetSemanticEdit,
                              remove: removeDataAssetStage,
                              pickPatchReceipt: pickDataAssetPatchReceipt,
                              semanticInspector: inspectDataAssetSemanticEdit,
                              semanticUassetPicker: pickDataAssetSemanticUasset,
                              semanticUsmapPicker: pickDataAssetSemanticUsmap,
                              semanticExtractReceiptPicker:
                                  pickDataAssetExtractReceipt,
                              semanticExtractReceiptInspector:
                                  inspectDataAssetExtractReceipt,
                            ),
                          ],
                        ),
                      ),
                    ],
                  ),
                ),
        ),
      ],
    );
  }

  Future<void> _openVoiceWizard(BuildContext context) async {
    if (gameRoot == null || project.requiresReopen) return;
    final publication = await showDialog<Revision3VoiceTakePublication>(
      context: context,
      builder: (context) => Revision3VoiceTakeDialog(
        service: Revision3VoiceAuthoringService(
          loadContentIndex: loadContentIndex,
          publishTechnicalPlan: publishVoiceTake,
        ),
      ),
    );
    if (!context.mounted || publication == null) return;
    ScaffoldMessenger.of(context).showSnackBar(
      SnackBar(
        content: Text(
          'Voice take saved in project revision ${publication.projectRevision}. It is saved to the project only and is not yet usable in game.',
        ),
      ),
    );
  }

  Future<void> _openVoiceTargetResolver(BuildContext context) async {
    if (gameRoot == null || project.requiresReopen) return;
    final publication = await showDialog<Revision3VoiceTargetPublication>(
      context: context,
      builder: (context) => Revision3VoiceTargetDialog(
        service: Revision3VoiceTargetAuthoringService(
          loadContentIndex: loadContentIndex,
          publishTechnicalPlan: publishVoiceTarget,
        ),
      ),
    );
    if (!context.mounted || publication == null) return;
    final outcome = switch (publication.resolution) {
      AuthoringRevision3VoiceTargetResolutionState.unresolved =>
        'No installed archive member matched',
      AuthoringRevision3VoiceTargetResolutionState.resolved =>
        'One installed archive member was sealed',
      AuthoringRevision3VoiceTargetResolutionState.ambiguous =>
        '${publication.matchCount} installed archive members matched; nothing was chosen implicitly',
    };
    ScaffoldMessenger.of(context).showSnackBar(
      SnackBar(
        content: Text(
          '$outcome. Voice target evidence saved in project revision ${publication.projectRevision}.',
        ),
      ),
    );
  }

  Future<void> _openVoiceBuild(BuildContext context) async {
    if (gameRoot == null || project.requiresReopen) return;
    final result = await showDialog<AuthoringRevision3VoiceBuildResult>(
      context: context,
      builder: (context) => Revision3VoiceBuildDialog(
        build: buildVoiceBundle,
        pickExistingParentDirectory: pickVoiceBuildParent,
      ),
    );
    if (!context.mounted || result == null) return;
    final message = result.isBuilt
        ? 'Sealed Voice bundle built at ${result.output}. Deployment was not performed.'
        : 'Voice build blocked by ${result.report!.blockers.length} exact requirement(s). No bundle was created or deployed.';
    ScaffoldMessenger.of(
      context,
    ).showSnackBar(SnackBar(content: Text(message)));
  }

  Future<void> _openQuestWizard(BuildContext context) async {
    final configuredGameRoot = gameRoot;
    if (configuredGameRoot == null || project.requiresReopen) return;
    final publication = await showDialog<Revision3QuestDraftPublication>(
      context: context,
      builder: (context) => Revision3QuestWizardDialog(
        gameRoot: configuredGameRoot,
        loadCatalog: loadQuestCatalog,
        publish: publishQuestDraft,
      ),
    );
    if (!context.mounted || publication == null) return;
    ScaffoldMessenger.of(context).showSnackBar(
      SnackBar(
        content: Text(
          'Quest draft saved in project revision ${publication.projectRevision}. It remains build-blocked and runtime-unqualified.',
        ),
      ),
    );
  }

  Future<void> _openNpcWizard(BuildContext context) async {
    final configuredGameRoot = gameRoot;
    if (configuredGameRoot == null || project.requiresReopen) return;
    final publication = await showDialog<Revision3NpcDraftPublication>(
      context: context,
      builder: (context) => Revision3NpcWizardDialog(
        gameRoot: configuredGameRoot,
        loadCatalog: loadNpcCatalog,
        publish: publishNpcDraft,
        chooseArchetype: chooseNpcArchetype,
      ),
    );
    if (!context.mounted || publication == null) return;
    ScaffoldMessenger.of(context).showSnackBar(
      SnackBar(
        content: Text(
          'NPC draft saved in project revision ${publication.projectRevision}. It remains build-blocked, runtime-unqualified, and is not spawned.',
        ),
      ),
    );
  }

  Future<void> _openSettings(BuildContext context) => showDialog<void>(
    context: context,
    builder: (dialogContext) => Dialog(
      key: const Key('managed-settings-dialog'),
      child: SizedBox(
        width: 800,
        height: 700,
        child: Column(
          children: [
            Padding(
              padding: const EdgeInsets.fromLTRB(24, 16, 12, 8),
              child: Row(
                children: [
                  const Icon(Icons.settings_outlined),
                  const SizedBox(width: 10),
                  Text(
                    'Settings',
                    style: Theme.of(dialogContext).textTheme.titleLarge,
                  ),
                  const Spacer(),
                  IconButton(
                    key: const Key('managed-settings-close'),
                    onPressed: () => Navigator.of(dialogContext).pop(),
                    tooltip: 'Close Settings',
                    icon: const Icon(Icons.close),
                  ),
                ],
              ),
            ),
            const Divider(height: 1),
            const Expanded(child: SettingsTab()),
          ],
        ),
      ),
    ),
  );
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
