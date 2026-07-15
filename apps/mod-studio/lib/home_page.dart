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
import 'dataasset/ui/installed_package_browser_dialog.dart';
import 'dataasset/ui/installed_dataasset_semantic_edit_dialog.dart';
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
import 'project/revision3_base_game_content_browser.dart';
import 'project/revision3_content_index.dart';
import 'project/revision3_content_library.dart';
import 'project/revision3_content_workspace.dart';
import 'project/revision3_story_entity_workbench.dart';
import 'project/revision3_dataasset_authoring.dart';
import 'project/revision3_dataasset_stage_panel.dart';
import 'project/revision3_global_content_search.dart';
import 'project/revision3_global_content_search_view.dart';
import 'project/revision3_npc_authoring.dart';
import 'project/revision3_npc_profile_dialog.dart';
import 'project/revision3_managed_compiler_check_panel.dart';
import 'project/revision3_npc_wizard.dart';
import 'project/revision3_quest_authoring.dart';
import 'project/revision3_quest_context_authoring.dart';
import 'project/revision3_quest_context_dialog.dart';
import 'project/revision3_quest_outline_authoring.dart';
import 'project/revision3_quest_outline_dialog.dart';
import 'project/revision3_quest_source_inspection_dialog.dart';
import 'project/revision3_quest_transitions_authoring.dart';
import 'project/revision3_quest_transitions_dialog.dart';
import 'project/revision3_quest_wizard.dart';
import 'project/revision3_project_create_dialog.dart';
import 'project/revision3_project_dashboard.dart';
import 'project/revision3_project_section_page.dart';
import 'project/revision3_project_workspace.dart';
import 'project/revision3_scoped_content_browser.dart';
import 'project/revision3_settings_expert_page.dart';
import 'project/revision3_installed_content_browser.dart';
import 'project/revision3_voice_authoring.dart';
import 'project/revision3_voice_build_dialog.dart';
import 'project/revision3_voice_take_selection_authoring.dart';
import 'project/revision3_voice_take_selection_dialog.dart';
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

final class _Revision3ManagedCompilerSelection {
  const _Revision3ManagedCompilerSelection({
    required this.entityId,
    required this.entityRevision,
    required this.moduleId,
    required this.moduleRevision,
  });

  final String entityId;
  final int entityRevision;
  final String moduleId;
  final int moduleRevision;
}

_Revision3ManagedCompilerSelection? _revision3ManagedCompilerSelection({
  required Revision3ContentIndex index,
  required Revision3ContentEntity entity,
  required Revision3ContentEntityKind expectedKind,
}) {
  if (entity.kind != expectedKind) return null;
  Revision3ContentReference? moduleReference;
  for (final reference in entity.references) {
    if (reference.role != 'draft_script_module' ||
        reference.resolution != Revision3ContentReferenceResolution.resolved ||
        reference.target.projectId != index.projectId ||
        reference.target.expectedKind !=
            Revision3ContentEntityKind.scriptModule) {
      continue;
    }
    if (moduleReference != null) return null;
    moduleReference = reference;
  }
  if (moduleReference == null) return null;
  final module = index.entityById(moduleReference.target.entityId);
  if (module == null ||
      module.kind != Revision3ContentEntityKind.scriptModule) {
    return null;
  }
  return _Revision3ManagedCompilerSelection(
    entityId: entity.id,
    entityRevision: entity.revision,
    moduleId: module.id,
    moduleRevision: module.revision,
  );
}

/// Injectable selection boundary; opening and adoption remain owned by the
/// app-wide [CurrentProjectCoordinator].
final managedRevision3DirectoryPickerProvider =
    Provider<ManagedRevision3DirectoryPicker>(
      (ref) =>
          (confirmButtonText) =>
              getDirectoryPath(confirmButtonText: confirmButtonText),
    );

typedef ManagedRevision3ProjectCreatePrompt =
    Future<Revision3ProjectCreateFormResult?> Function(BuildContext context);

/// Injectable metadata-form boundary. Project creation, generation discovery,
/// and filesystem mutation remain owned by the current-project coordinator.
final managedRevision3ProjectCreatePromptProvider =
    Provider<ManagedRevision3ProjectCreatePrompt>(
      (ref) => showRevision3ProjectCreateDialog,
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

  Future<void> _newManagedRevision3Project() => _runProjectAction(() async {
    if (!await _confirmDiscardIfDirty() || !mounted) return;
    final l10n = AppLocalizations.of(context);
    final gameRoot = gameRootFromExe(ref.read(gameExePathProvider));
    if (gameRoot == null) {
      _snack(l10n.projectCreateGamePathRequired);
      return;
    }
    final Revision3ProjectCreateFormResult? form;
    final String? path;
    try {
      form = await ref.read(managedRevision3ProjectCreatePromptProvider)(
        context,
      );
      if (form == null || !mounted) return;
      path = await ref.read(managedRevision3DirectoryPickerProvider)(
        l10n.projectCreateDirectoryPickerTitle,
      );
      if (path == null || !mounted) return;
    } catch (e) {
      _snack(l10n.projectManagedRevision3CreateFailed('$e'));
      return;
    }

    final coordinator = ref.read(currentProjectCoordinatorProvider.notifier);
    final cleanupFailuresBefore = coordinator.terminalCleanupFailures.length;
    final ManagedRevision3CurrentProjectState created;
    try {
      created = await coordinator.createManagedRevision3(
        ManagedRevision3ProjectCreateRequest(
          root: Directory(path),
          gameRoot: gameRoot,
          name: form.name,
          version: form.version,
          author: form.author,
          authoringLocales: form.authoringLocales,
        ),
      );
    } catch (e) {
      _snack(l10n.projectManagedRevision3CreateFailed('$e'));
      return;
    }

    if (_showTransitionCleanupWarningIfAdded(
      coordinator,
      cleanupFailuresBefore,
    )) {
      return;
    }
    if (form.starter == Revision3ProjectStarter.empty) {
      _snack(l10n.projectManagedRevision3Created(created.projectId));
      return;
    }
    try {
      await _openManagedProjectStarter(
        starter: form.starter,
        created: created,
        gameRoot: gameRoot,
      );
    } catch (_) {
      if (mounted) {
        final outcomeIsExactCreated = _starterOutcomeIsExactCreated(created);
        _snack(
          outcomeIsExactCreated
              ? AppLocalizations.of(
                  context,
                ).projectStarterSetupOpenFailed(created.projectId)
              : AppLocalizations.of(
                  context,
                ).projectStarterOutcomeUnverified(created.projectId),
        );
      }
    }
  });

  Future<void> _openManagedProjectStarter({
    required Revision3ProjectStarter starter,
    required ManagedRevision3CurrentProjectState created,
    required String gameRoot,
  }) async {
    final coordinator = ref.read(currentProjectCoordinatorProvider.notifier);
    switch (starter) {
      case Revision3ProjectStarter.empty:
        _snack(
          AppLocalizations.of(
            context,
          ).projectManagedRevision3Created(created.projectId),
        );
        return;
      case Revision3ProjectStarter.npcDraft:
        final publication = await showDialog<Revision3NpcDraftPublication>(
          context: context,
          builder: (dialogContext) => Revision3NpcWizardDialog(
            gameRoot: gameRoot,
            loadCatalog: ref.read(revision3NpcCatalogLoaderProvider),
            chooseArchetype: ref.read(
              managedRevision3NpcArchetypeChooserProvider,
            ),
            publish: ({required gameRoot, required input}) =>
                coordinator.createCurrentRevision3NpcDraft(
                  expectedRoot: created.root.path,
                  expectedHead: created.head,
                  expectedProjectId: created.projectId,
                  expectedProjectRevision: created.projectRevision,
                  gameRoot: gameRoot,
                  input: input,
                ),
          ),
        );
        if (!mounted) return;
        final outcomeIsExactCreated = _starterOutcomeIsExactCreated(created);
        _snack(
          publication == null
              ? outcomeIsExactCreated
                    ? AppLocalizations.of(
                        context,
                      ).projectStarterNpcCancelled(created.projectId)
                    : AppLocalizations.of(
                        context,
                      ).projectStarterOutcomeUnverified(created.projectId)
              : AppLocalizations.of(
                  context,
                ).projectStarterNpcSaved(publication.projectRevision),
        );
        return;
      case Revision3ProjectStarter.questDraft:
        final publication = await showDialog<Revision3QuestDraftPublication>(
          context: context,
          builder: (dialogContext) => Revision3QuestWizardDialog(
            gameRoot: gameRoot,
            loadCatalog: ref.read(revision3QuestCatalogLoaderProvider),
            publish: ({required gameRoot, required input}) =>
                coordinator.createCurrentRevision3QuestDraft(
                  expectedRoot: created.root.path,
                  expectedProjectId: created.projectId,
                  expectedProjectRevision: created.projectRevision,
                  expectedHead: created.head,
                  gameRoot: gameRoot,
                  input: input,
                ),
          ),
        );
        if (!mounted) return;
        final outcomeIsExactCreated = _starterOutcomeIsExactCreated(created);
        _snack(
          publication == null
              ? outcomeIsExactCreated
                    ? AppLocalizations.of(
                        context,
                      ).projectStarterQuestCancelled(created.projectId)
                    : AppLocalizations.of(
                        context,
                      ).projectStarterOutcomeUnverified(created.projectId)
              : AppLocalizations.of(
                  context,
                ).projectStarterQuestSaved(publication.projectRevision),
        );
        return;
    }
  }

  bool _starterOutcomeIsExactCreated(
    ManagedRevision3CurrentProjectState created,
  ) {
    final current = ref.read(currentProjectCoordinatorProvider);
    return current is ManagedRevision3CurrentProjectState &&
        !current.requiresReopen &&
        current.root.path == created.root.path &&
        current.projectId == created.projectId &&
        current.projectRevision == created.projectRevision &&
        current.head.canonicalJson == created.head.canonicalJson;
  }

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
    final compactWindowChrome = MediaQuery.sizeOf(context).width < 760;
    final legacyEntryBanner = _ManagedProjectEntryBanner(
      bannerKey: const Key('legacy-compatibility-banner'),
      title: l10n.managedProjectLandingTitle,
      description: l10n.managedProjectLandingDescription,
      createLabel: l10n.projectNewManagedRevision3,
      openLabel: l10n.projectOpenManagedRevision3,
      legacyTitle: l10n.legacyCompatibilityToolsTitle,
      legacyDescription: l10n.legacyCompatibilityToolsDescription,
      onCreateManaged: _projectActionBusy
          ? null
          : () => unawaited(_newManagedRevision3Project()),
      onOpenManaged: _projectActionBusy
          ? null
          : () => unawaited(_openManagedRevision3Project()),
    );

    final scaffold = Scaffold(
      appBar: AppBar(
        title: WindowDragArea(
          child: Row(
            children: [
              SizedBox(width: compactWindowChrome ? 8 : 16),
              Image.asset(
                'assets/gore_studio_icon.png',
                height: compactWindowChrome ? 26 : 32,
              ),
              if (!compactWindowChrome) ...[
                const SizedBox(width: 10),
                const Text('GORE Mod Studio'),
              ],
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
                case 'newManagedRevision3':
                  await _newManagedRevision3Project();
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
                key: const Key('project-new-managed-revision3'),
                value: 'newManagedRevision3',
                enabled: !_projectActionBusy,
                child: Text(l10n.projectNewManagedRevision3),
              ),
              PopupMenuItem(
                key: const Key('project-new-legacy'),
                value: 'new',
                enabled: !_projectActionBusy,
                child: Text(l10n.projectNewLegacy),
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
          if (legacyCurrent && compactWindowChrome)
            IconButton(
              key: const Key('legacy-build-deploy-compact'),
              icon: const Icon(Icons.rocket_launch_outlined),
              tooltip: 'Build / Deploy',
              onPressed: !_projectActionBusy && (dirty || gameConfigured)
                  ? () => showDialog(
                      context: context,
                      builder: (_) => const BuildDeployDialog(),
                    )
                  : null,
            ),
          if (legacyCurrent && !compactWindowChrome)
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
          verifyCurrentHead: _saveProject,
          loadContentIndex: () => ref
              .read(currentProjectCoordinatorProvider.notifier)
              .readCurrentRevision3ContentIndex(),
          loadBaseGameCatalog: ref.read(
            revision3BaseGameContentCatalogLoaderProvider,
          ),
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
          publishVoiceTakeSelection:
              ({
                required expectedProjectId,
                required expectedProjectRevision,
                required plan,
              }) => ref
                  .read(currentProjectCoordinatorProvider.notifier)
                  .selectCurrentRevision3VoiceTake(
                    expectedRoot: currentProject.root.path,
                    expectedProjectId: expectedProjectId,
                    expectedProjectRevision: expectedProjectRevision,
                    expectedHead: currentProject.head,
                    plan: plan,
                  ),
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
          loadInstalledPackageIndex: ({required gameRoot}) => ref
              .read(currentProjectCoordinatorProvider.notifier)
              .readCurrentRevision3DataAssetPackageIndex(
                expectedRoot: currentProject.root.path,
                expectedProjectId: currentProject.projectId,
                expectedProjectRevision: currentProject.projectRevision,
                expectedHead: currentProject.head,
                gameRoot: gameRoot,
              ),
          inspectInstalledDataAsset:
              ({
                required gameRoot,
                required expectedSnapshot,
                required candidate,
              }) => ref
                  .read(currentProjectCoordinatorProvider.notifier)
                  .inspectCurrentRevision3InstalledDataAsset(
                    expectedRoot: currentProject.root.path,
                    expectedProjectId: currentProject.projectId,
                    expectedProjectRevision: currentProject.projectRevision,
                    expectedHead: currentProject.head,
                    gameRoot: gameRoot,
                    expectedSnapshot: expectedSnapshot,
                    candidate: candidate,
                  ),
          publishInstalledDataAssetSemanticEdit: (intent) async {
            final configuredGameRoot = gameRoot;
            if (configuredGameRoot == null) {
              throw const DataAssetSemanticStageUnavailableException.staleCheckpoint();
            }
            try {
              final publication = await ref
                  .read(currentProjectCoordinatorProvider.notifier)
                  .addCurrentRevision3InstalledDataAssetEdit(
                    expectedRoot: currentProject.root.path,
                    expectedProjectId: currentProject.projectId,
                    expectedProjectRevision: currentProject.projectRevision,
                    expectedHead: currentProject.head,
                    gameRoot: configuredGameRoot,
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
            } on Revision3InstalledDataAssetEditSourceEvidenceStaleException {
              throw const DataAssetSemanticStageUnavailableException.sourceEvidenceStale();
            } on Revision3InstalledDataAssetEditRejectedException catch (
              error
            ) {
              throw switch (error.reason) {
                Revision3InstalledDataAssetEditRejectionReason
                    .targetAlreadyStaged =>
                  const DataAssetSemanticStageUnavailableException.targetAlreadyStaged(),
                Revision3InstalledDataAssetEditRejectionReason
                    .preparationFailed =>
                  const DataAssetSemanticStageUnavailableException.preparationRejected(),
              };
            }
          },
          publishReviewedInstalledDataAssetEdit: (intent) async {
            final configuredGameRoot = gameRoot;
            if (configuredGameRoot == null) {
              throw const DataAssetSemanticStageUnavailableException.staleCheckpoint();
            }
            try {
              final publication = await ref
                  .read(currentProjectCoordinatorProvider.notifier)
                  .addCurrentRevision3ReviewedInstalledDataAssetEdit(
                    expectedRoot: currentProject.root.path,
                    expectedProjectId: currentProject.projectId,
                    expectedProjectRevision: currentProject.projectRevision,
                    expectedHead: currentProject.head,
                    gameRoot: configuredGameRoot,
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
            } on Revision3InstalledDataAssetEditSourceEvidenceStaleException {
              throw const DataAssetSemanticStageUnavailableException.sourceEvidenceStale();
            } on Revision3InstalledDataAssetEditRejectedException catch (
              error
            ) {
              throw switch (error.reason) {
                Revision3InstalledDataAssetEditRejectionReason
                    .targetAlreadyStaged =>
                  const DataAssetSemanticStageUnavailableException.targetAlreadyStaged(),
                Revision3InstalledDataAssetEditRejectionReason
                    .preparationFailed =>
                  const DataAssetSemanticStageUnavailableException.preparationRejected(),
              };
            }
          },
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
          editQuestOutline: ({required input}) => ref
              .read(currentProjectCoordinatorProvider.notifier)
              .editCurrentRevision3QuestOutline(
                expectedRoot: currentProject.root.path,
                expectedProjectId: currentProject.projectId,
                expectedProjectRevision: currentProject.projectRevision,
                expectedHead: currentProject.head,
                input: input,
              ),
          loadQuestTransitionsSeed:
              ({
                required questId,
                required expectedQuestRevision,
                required expectedModuleId,
                required expectedModuleRevision,
              }) => ref
                  .read(currentProjectCoordinatorProvider.notifier)
                  .readCurrentRevision3QuestTransitionsSeed(
                    expectedRoot: currentProject.root.path,
                    expectedProjectId: currentProject.projectId,
                    expectedProjectRevision: currentProject.projectRevision,
                    expectedHead: currentProject.head,
                    questId: questId,
                    expectedQuestRevision: expectedQuestRevision,
                    expectedModuleId: expectedModuleId,
                    expectedModuleRevision: expectedModuleRevision,
                  ),
          editQuestTransitions: ({required plan}) => ref
              .read(currentProjectCoordinatorProvider.notifier)
              .editCurrentRevision3QuestTransitions(
                expectedRoot: currentProject.root.path,
                expectedProjectId: currentProject.projectId,
                expectedProjectRevision: currentProject.projectRevision,
                expectedHead: currentProject.head,
                plan: plan,
              ),
          loadQuestContextSeed:
              ({
                required questId,
                required expectedQuestRevision,
                required expectedModuleId,
                required expectedModuleRevision,
                required expectedParentRuntimeClass,
                required expectedGiverRuntimeUniqueName,
              }) => ref
                  .read(currentProjectCoordinatorProvider.notifier)
                  .readCurrentRevision3QuestContextSeed(
                    expectedRoot: currentProject.root.path,
                    expectedProjectId: currentProject.projectId,
                    expectedProjectRevision: currentProject.projectRevision,
                    expectedHead: currentProject.head,
                    questId: questId,
                    expectedQuestRevision: expectedQuestRevision,
                    expectedModuleId: expectedModuleId,
                    expectedModuleRevision: expectedModuleRevision,
                    expectedParentRuntimeClass: expectedParentRuntimeClass,
                    expectedGiverRuntimeUniqueName:
                        expectedGiverRuntimeUniqueName,
                  ),
          editQuestContext: ({required gameRoot, required plan}) => ref
              .read(currentProjectCoordinatorProvider.notifier)
              .editCurrentRevision3QuestContext(
                expectedRoot: currentProject.root.path,
                expectedProjectId: currentProject.projectId,
                expectedProjectRevision: currentProject.projectRevision,
                expectedHead: currentProject.head,
                gameRoot: gameRoot,
                plan: plan,
              ),
          inspectQuestSource: ({required gameRoot, required questId}) => ref
              .read(currentProjectCoordinatorProvider.notifier)
              .inspectCurrentRevision3QuestSource(
                expectedRoot: currentProject.root.path,
                expectedProjectId: currentProject.projectId,
                expectedProjectRevision: currentProject.projectRevision,
                expectedHead: currentProject.head,
                gameRoot: gameRoot,
                questId: questId,
              ),
          inspectNpcSource: ({required npcId}) => ref
              .read(currentProjectCoordinatorProvider.notifier)
              .inspectCurrentRevision3NpcSource(
                expectedRoot: currentProject.root.path,
                expectedProjectId: currentProject.projectId,
                expectedProjectRevision: currentProject.projectRevision,
                expectedHead: currentProject.head,
                npcId: npcId,
              ),
          checkManagedCompiler:
              ({
                required entityKind,
                required entityId,
                required expectedEntityRevision,
                required expectedModuleId,
                required expectedModuleRevision,
                required gameRoot,
              }) => ref
                  .read(currentProjectCoordinatorProvider.notifier)
                  .checkCurrentRevision3ManagedCompiler(
                    expectedRoot: currentProject.root.path,
                    expectedProjectId: currentProject.projectId,
                    expectedProjectRevision: currentProject.projectRevision,
                    expectedHead: currentProject.head,
                    entityKind: entityKind,
                    entityId: entityId,
                    expectedEntityRevision: expectedEntityRevision,
                    expectedModuleId: expectedModuleId,
                    expectedModuleRevision: expectedModuleRevision,
                    gameRoot: gameRoot,
                  ),
        ),
        NoCurrentProjectState() => _NoCurrentProjectView(
          onCreateManaged: _projectActionBusy
              ? null
              : () => unawaited(_newManagedRevision3Project()),
          onOpenManaged: _projectActionBusy
              ? null
              : () => unawaited(_openManagedRevision3Project()),
          onOpenSettings: _projectActionBusy
              ? null
              : () => unawaited(_showModStudioSettingsDialog(context)),
        ),
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
                Padding(
                  padding: const EdgeInsets.fromLTRB(12, 12, 12, 4),
                  child: compactWindowChrome
                      ? ConstrainedBox(
                          constraints: const BoxConstraints(maxHeight: 220),
                          child: SingleChildScrollView(
                            key: const Key(
                              'legacy-compatibility-banner-scroll',
                            ),
                            child: legacyEntryBanner,
                          ),
                        )
                      : legacyEntryBanner,
                ),
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
    required this.verifyCurrentHead,
    required this.loadContentIndex,
    required this.loadBaseGameCatalog,
    required this.publishVoiceTake,
    required this.publishVoiceTakeSelection,
    required this.publishVoiceTarget,
    required this.buildVoiceBundle,
    required this.pickVoiceBuildParent,
    required this.loadDataAssetStages,
    required this.loadInstalledPackageIndex,
    required this.inspectInstalledDataAsset,
    required this.publishInstalledDataAssetSemanticEdit,
    required this.publishReviewedInstalledDataAssetEdit,
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
    required this.editQuestOutline,
    required this.loadQuestTransitionsSeed,
    required this.editQuestTransitions,
    required this.loadQuestContextSeed,
    required this.editQuestContext,
    required this.inspectQuestSource,
    required this.inspectNpcSource,
    required this.checkManagedCompiler,
  });

  final ManagedRevision3CurrentProjectState project;
  final String? gameRoot;
  final Future<void> Function() verifyCurrentHead;
  final Revision3ContentIndexLoader loadContentIndex;
  final Revision3BaseGameContentCatalogLoader loadBaseGameCatalog;
  final Revision3VoiceTechnicalPublisher publishVoiceTake;
  final Revision3VoiceTakeSelectionTechnicalPublisher publishVoiceTakeSelection;
  final Revision3VoiceTargetTechnicalPublisher publishVoiceTarget;
  final Revision3VoiceExactBuild buildVoiceBundle;
  final Revision3VoiceBuildParentDirectoryPicker pickVoiceBuildParent;
  final Revision3DataAssetStageLoader loadDataAssetStages;
  final Revision3InstalledPackageIndexLoader loadInstalledPackageIndex;
  final Revision3InstalledDataAssetInspector inspectInstalledDataAsset;
  final InstalledDataAssetSemanticStagePublisher
  publishInstalledDataAssetSemanticEdit;
  final ReviewedInstalledDataAssetStagePublisher
  publishReviewedInstalledDataAssetEdit;
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
  final Revision3QuestOutlineEditPublisher editQuestOutline;
  final Revision3QuestTransitionsSeedLoader loadQuestTransitionsSeed;
  final Revision3QuestTransitionsTechnicalPublisher editQuestTransitions;
  final Revision3QuestContextSeedLoader loadQuestContextSeed;
  final Revision3QuestContextTechnicalPublisher editQuestContext;
  final Revision3QuestSourceInspectionLoader inspectQuestSource;
  final Revision3NpcSourceInspectionLoader inspectNpcSource;
  final Revision3ManagedCompilerPublisher checkManagedCompiler;

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
                              ? l10n.managedProjectRecoveryContentLocked
                              : l10n.managedProjectSubtitle,
                        ),
                      ],
                    );
                    final settings = OutlinedButton.icon(
                      key: const Key('managed-open-settings'),
                      onPressed: () => unawaited(_openSettings(context)),
                      icon: const Icon(Icons.settings_outlined),
                      label: Text(l10n.managedActionSettingsTitle),
                    );
                    if (constraints.maxWidth < 680) {
                      return Column(
                        crossAxisAlignment: CrossAxisAlignment.stretch,
                        children: [
                          identity,
                          const SizedBox(height: 12),
                          Align(
                            alignment: Alignment.centerLeft,
                            child: settings,
                          ),
                        ],
                      );
                    }
                    return Row(
                      crossAxisAlignment: CrossAxisAlignment.start,
                      children: [
                        Expanded(child: identity),
                        const SizedBox(width: 16),
                        settings,
                      ],
                    );
                  },
                ),
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
                ExpansionTile(
                  key: const Key('managed-project-technical-details'),
                  tilePadding: EdgeInsets.zero,
                  childrenPadding: const EdgeInsets.only(top: 4),
                  title: Text(l10n.managedProjectTechnicalDetails),
                  children: [
                    Align(
                      alignment: Alignment.centerLeft,
                      child: Wrap(
                        spacing: 20,
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
                        Text(
                          l10n.managedProjectRecoveryContentLocked,
                          textAlign: TextAlign.center,
                        ),
                      ],
                    ),
                  ),
                )
              : Revision3ProjectWorkspace(
                  projectIdentity: (project.root.path, project.projectId),
                  destinations: [
                    Revision3ProjectWorkspaceDestination(
                      section: Revision3ProjectWorkspaceSection.home,
                      label: l10n.managedWorkspaceHomeLabel,
                      icon: Icons.home_outlined,
                      selectedIcon: Icons.home,
                      pageBuilder: (workspaceContext, _) =>
                          _buildDashboard(workspaceContext, l10n),
                    ),
                    Revision3ProjectWorkspaceDestination(
                      section: Revision3ProjectWorkspaceSection.content,
                      label: l10n.managedWorkspaceContentLabel,
                      icon: Icons.account_tree_outlined,
                      selectedIcon: Icons.account_tree,
                      pageBuilder: (workspaceContext, location) =>
                          _buildContentWorkspace(
                            workspaceContext,
                            location,
                            l10n,
                          ),
                    ),
                    Revision3ProjectWorkspaceDestination(
                      section: Revision3ProjectWorkspaceSection.story,
                      label: l10n.managedWorkspaceStoryLabel,
                      icon: Icons.menu_book_outlined,
                      selectedIcon: Icons.menu_book,
                      pageBuilder: (workspaceContext, _) =>
                          _buildStorySection(workspaceContext, l10n),
                    ),
                    Revision3ProjectWorkspaceDestination(
                      section: Revision3ProjectWorkspaceSection.world,
                      label: l10n.managedWorkspaceWorldLabel,
                      icon: Icons.public_outlined,
                      selectedIcon: Icons.public,
                      pageBuilder: (_, _) => _buildWorldSection(l10n),
                    ),
                    Revision3ProjectWorkspaceDestination(
                      section:
                          Revision3ProjectWorkspaceSection.localizationVoice,
                      label: l10n.managedWorkspaceLocalizationVoiceLabel,
                      icon: Icons.record_voice_over_outlined,
                      selectedIcon: Icons.record_voice_over,
                      pageBuilder: (workspaceContext, _) =>
                          _buildLocalizationVoiceSection(
                            workspaceContext,
                            l10n,
                          ),
                    ),
                    Revision3ProjectWorkspaceDestination(
                      section: Revision3ProjectWorkspaceSection.validateTest,
                      label: l10n.managedWorkspaceValidateTestLabel,
                      icon: Icons.fact_check_outlined,
                      selectedIcon: Icons.fact_check,
                      pageBuilder: (workspaceContext, _) =>
                          _buildValidateTestSection(workspaceContext, l10n),
                    ),
                    Revision3ProjectWorkspaceDestination(
                      section: Revision3ProjectWorkspaceSection.buildRelease,
                      label: l10n.managedWorkspaceBuildReleaseLabel,
                      icon: Icons.inventory_2_outlined,
                      selectedIcon: Icons.inventory_2,
                      pageBuilder: (workspaceContext, _) =>
                          _buildReleaseSection(workspaceContext, l10n),
                    ),
                    Revision3ProjectWorkspaceDestination(
                      section: Revision3ProjectWorkspaceSection.settingsExpert,
                      label: l10n.managedWorkspaceSettingsExpertLabel,
                      icon: Icons.settings_outlined,
                      selectedIcon: Icons.settings,
                      pageBuilder: (_, _) => Revision3SettingsExpertPage(
                        title: l10n.managedWorkspaceSettingsExpertLabel,
                        description:
                            l10n.managedSectionSettingsExpertDescription,
                        expertStatusLabel: l10n.managedCapabilityUnavailable,
                        expertStatusDescription:
                            l10n.managedSectionSettingsExpertDescription,
                        settings: const SettingsTab(),
                      ),
                    ),
                  ],
                ),
        ),
      ],
    );
  }

  Widget _buildContentWorkspace(
    BuildContext context,
    Revision3ProjectWorkspaceLocation location,
    AppLocalizations l10n,
  ) => Revision3ContentWorkspace(
    location: location,
    libraryLabel: l10n.managedContentWorkspaceBrowseLabel,
    dataAssetsLabel: l10n.managedContentWorkspaceVerifiedEditsLabel,
    library: _ManagedRevision3GlobalContentHost(
      sourceIdentity: Revision3GlobalContentSearchSourceIdentity(
        project: '${project.root.path}\u0000${project.projectId}',
        thisMod:
            '${project.projectRevision}\u0000${project.head.canonicalJson}',
        baseGame: gameRoot ?? '<game-unconfigured>',
        installed:
            '${project.projectRevision}\u0000${project.head.canonicalJson}\u0000${gameRoot ?? '<game-unconfigured>'}',
      ),
      loadThisMod: loadContentIndex,
      loadBaseGame: () {
        final configuredGameRoot = gameRoot;
        if (configuredGameRoot == null) {
          throw const FormatException(
            'A configured game installation is required.',
          );
        }
        return loadBaseGameCatalog(configuredGameRoot);
      },
      loadInstalled: () {
        final configuredGameRoot = gameRoot;
        if (configuredGameRoot == null) {
          throw const FormatException(
            'A configured game installation is required.',
          );
        }
        return loadInstalledPackageIndex(gameRoot: configuredGameRoot);
      },
      builder: (context, contentLibraryController, globalSearchController) =>
          Revision3ScopedContentBrowser(
            projectIdentity: (project.root.path, project.projectId),
            thisModLabel: l10n.managedContentWorkspaceLibraryLabel,
            baseGameLabel: l10n.managedContentScopeBaseGameLabel,
            installedLabel: l10n.managedContentScopeInstalledLabel,
            allSourcesLabel: l10n.managedGlobalSearchScopeLabel,
            thisMod: Revision3ContentLibrary(
              projectRoot: project.root.path,
              projectId: project.projectId,
              projectRevision: project.projectRevision,
              projectHeadCanonicalJson: project.head.canonicalJson,
              load: loadContentIndex,
              editQuestOutline: (index, quest) =>
                  _openQuestOutlineEditor(context, index, quest),
              editQuestContext: gameRoot == null
                  ? null
                  : (index, quest) =>
                        _openQuestContextEditor(context, index, quest),
              editQuestTransitions: (index, quest) =>
                  _openQuestTransitionsEditor(context, index, quest),
              inspectQuestSource: gameRoot == null
                  ? null
                  : (index, quest) =>
                        _openQuestSourceInspection(context, index, quest),
              inspectNpcSource: (index, npc) =>
                  _openNpcProfile(context, index, npc),
              editQuestContextDisabledReason: gameRoot == null
                  ? l10n.managedDashboardMissingGameDescription
                  : null,
              inspectQuestSourceDisabledReason: gameRoot == null
                  ? l10n.managedDashboardMissingGameDescription
                  : null,
              controller: contentLibraryController,
              storyWorkbenchCopy: Revision3StoryEntityWorkbenchCopy(
                draftBadge: l10n.managedStoryWorkbenchDraftBadge,
                buildBlockedBadge: l10n.managedStoryWorkbenchBuildBlockedBadge,
                runtimeUnqualifiedBadge:
                    l10n.managedStoryWorkbenchRuntimeUnqualifiedBadge,
                overviewTab: l10n.managedStoryWorkbenchOverviewTab,
                profileTab: l10n.managedStoryWorkbenchProfileTab,
                storyTab: l10n.managedStoryWorkbenchStoryTab,
                logicTab: l10n.managedStoryWorkbenchLogicTab,
                routineTab: l10n.managedStoryWorkbenchRoutineTab,
                inventoryTab: l10n.managedStoryWorkbenchInventoryTab,
                dialogVoiceTab: l10n.managedStoryWorkbenchDialogVoiceTab,
                referencesTab: l10n.managedStoryWorkbenchReferencesTab,
                problemsChecksTab: l10n.managedStoryWorkbenchProblemsChecksTab,
                editOverview: l10n.managedStoryWorkbenchEditOverview,
                editStory: l10n.managedStoryWorkbenchEditStory,
                editLogic: l10n.managedStoryWorkbenchEditLogic,
                inspectQuest: l10n.managedStoryWorkbenchInspectQuest,
                inspectNpc: l10n.managedStoryWorkbenchInspectNpc,
                capabilityUnavailable:
                    l10n.managedStoryWorkbenchCapabilityUnavailable,
                npcStoryUnavailable:
                    l10n.managedStoryWorkbenchNpcStoryUnavailable,
                npcRoutineUnavailable:
                    l10n.managedStoryWorkbenchNpcRoutineUnavailable,
                npcInventoryUnavailable:
                    l10n.managedStoryWorkbenchNpcInventoryUnavailable,
                npcDialogVoiceUnavailable:
                    l10n.managedStoryWorkbenchNpcDialogVoiceUnavailable,
                questDialogVoiceUnavailable:
                    l10n.managedStoryWorkbenchQuestDialogVoiceUnavailable,
                noReferenceProblems:
                    l10n.managedStoryWorkbenchNoReferenceProblems,
                referenceProblemCount:
                    l10n.managedStoryWorkbenchReferenceProblemCount,
                referenceScopeNotice:
                    l10n.managedStoryWorkbenchReferenceScopeNotice,
                technicalDetails: l10n.managedStoryWorkbenchTechnicalDetails,
                questKindLabel: l10n.managedStoryWorkbenchQuestKindLabel,
                npcKindLabel: l10n.managedStoryWorkbenchNpcKindLabel,
                questTitleLabel: l10n.managedStoryWorkbenchQuestTitleLabel,
                technicalIdLabel: l10n.managedStoryWorkbenchTechnicalIdLabel,
                objectivesLabel: l10n.managedStoryWorkbenchObjectivesLabel,
                uniqueNameLabel: l10n.managedStoryWorkbenchUniqueNameLabel,
                moduleNamespaceLabel:
                    l10n.managedStoryWorkbenchModuleNamespaceLabel,
                questGiverLabel: l10n.managedStoryWorkbenchQuestGiverLabel,
                runtimeParentLabel:
                    l10n.managedStoryWorkbenchRuntimeParentLabel,
                logicDescription: l10n.managedStoryWorkbenchLogicDescription,
                outgoingHeading: l10n.managedStoryWorkbenchOutgoingHeading,
                noOutgoingReferences:
                    l10n.managedStoryWorkbenchNoOutgoingReferences,
                incomingHeading: l10n.managedStoryWorkbenchIncomingHeading,
                noIncomingReferences:
                    l10n.managedStoryWorkbenchNoIncomingReferences,
                semanticIdentityLabel:
                    l10n.managedStoryWorkbenchSemanticIdentityLabel,
                originLabel: l10n.managedStoryWorkbenchOriginLabel,
                entityRevisionLabel:
                    l10n.managedStoryWorkbenchEntityRevisionLabel,
                stableIdLabel: l10n.managedStoryWorkbenchStableIdLabel,
                referenceResolvedLabel:
                    l10n.managedStoryWorkbenchReferenceResolvedLabel,
                referenceUnresolvedLabel:
                    l10n.managedStoryWorkbenchReferenceUnresolvedLabel,
              ),
            ),
            baseGame: Revision3BaseGameContentBrowser(
              gameRoot: gameRoot,
              sourceIdentity: (project.root.path, project.projectId, gameRoot),
              loader: loadBaseGameCatalog,
              copy: Revision3BaseGameContentBrowserCopy(
                title: l10n.managedBaseGameBrowserTitle,
                description: l10n.managedBaseGameBrowserDescription,
                missingGameTitle: l10n.managedDashboardMissingGameTitle,
                missingGameDescription:
                    l10n.managedDashboardMissingGameDescription,
                configureGame: l10n.managedActionSettingsTitle,
                loading: l10n.managedBaseGameBrowserLoading,
                refresh: l10n.managedBaseGameBrowserRefresh,
                searchLabel: l10n.managedBaseGameBrowserSearchLabel,
                filterAll: l10n.changesAll,
                filterNpcs: l10n.managedBaseGameBrowserFilterNpcs,
                filterQuests: l10n.managedBaseGameBrowserFilterQuests,
                npcSectionTitle: l10n.managedBaseGameBrowserNpcSectionTitle,
                questSectionTitle: l10n.managedBaseGameBrowserQuestSectionTitle,
                experimentalNpcSectionTitle:
                    l10n.managedBaseGameBrowserExperimentalNpcSectionTitle,
                searchForExperimental:
                    l10n.managedBaseGameBrowserSearchForExperimental,
                empty: l10n.managedBaseGameBrowserEmpty,
                loadErrorTitle: l10n.managedBaseGameBrowserLoadErrorTitle,
                loadErrorDescription:
                    l10n.managedBaseGameBrowserLoadErrorDescription,
                retry: l10n.managedDashboardRetry,
                baseGameSourceBadge: l10n.managedContentScopeBaseGameLabel,
                offlineDraftBadge: l10n.managedBaseGameBrowserOfflineDraftBadge,
                runtimeUnqualifiedBadge:
                    l10n.managedDashboardRuntimeUnqualifiedTitle,
                inspectOnlyBadge: l10n.managedBaseGameBrowserInspectOnlyBadge,
                createNpcDraft: l10n.managedBaseGameBrowserCreateNpcDraft,
                createQuestDraft: l10n.managedBaseGameBrowserCreateQuestDraft,
                spawnClass: l10n.managedBaseGameBrowserSpawnClass,
                actorBlueprint: l10n.managedBaseGameBrowserActorBlueprint,
                experimentalResultsCapped:
                    l10n.managedBaseGameBrowserExperimentalResultsCapped,
              ),
              openSettings: () => Revision3ProjectWorkspace.navigate(
                context,
                const Revision3ProjectWorkspaceLocation(
                  Revision3ProjectWorkspaceSection.settingsExpert,
                ),
              ),
              createNpcDraft: (catalogId) => unawaited(
                _openNpcWizard(context, initialCatalogId: catalogId),
              ),
              createQuestDraft: (parentCatalogId) => unawaited(
                _openQuestWizard(
                  context,
                  initialParentCatalogId: parentCatalogId,
                ),
              ),
            ),
            installed: Revision3InstalledContentBrowser(
              gameRoot: gameRoot,
              sourceIdentity: gameRoot == null
                  ? null
                  : (
                      project.root.path,
                      project.projectId,
                      project.projectRevision,
                      project.head.canonicalJson,
                      gameRoot,
                    ),
              loader: loadInstalledPackageIndex,
              copy: Revision3InstalledContentBrowserCopy(
                setupTitle: l10n.managedDashboardMissingGameTitle,
                setupDescription: l10n.managedDashboardMissingGameDescription,
                setupActionLabel: l10n.managedActionSettingsTitle,
                loadingLabel: l10n.managedInstalledBrowserLoading,
                completeSummary: l10n.managedInstalledBrowserCompleteSummary,
                partialSummary: l10n.managedInstalledBrowserPartialSummary,
                completeDescription:
                    l10n.managedInstalledBrowserCompleteDescription,
                partialDescription:
                    l10n.managedInstalledBrowserPartialDescription,
                authorityNotice: l10n.managedInstalledBrowserAuthorityNotice,
                refreshTooltip: l10n.managedInstalledBrowserRefresh,
                searchLabel: l10n.managedInstalledBrowserSearchLabel,
                searchHint: l10n.managedInstalledBrowserSearchHint,
                searchPrompt: l10n.managedInstalledBrowserSearchPrompt,
                noMatchesTitle: l10n.managedInstalledBrowserNoMatchesTitle,
                noMatchesDescription:
                    l10n.managedInstalledBrowserNoMatchesDescription,
                resultLimitDescription:
                    l10n.managedInstalledBrowserResultLimitDescription,
                kindBadgeLabel: l10n.managedInstalledBrowserKindBadge,
                sourceBadgeLabel: l10n.managedContentScopeInstalledLabel,
                readinessBadgeLabel:
                    l10n.managedInstalledBrowserMetadataOnlyBadge,
                openInspectorLabel: l10n.managedInstalledBrowserOpenInspector,
                errorTitle: l10n.managedInstalledBrowserErrorTitle,
                errorDescription: l10n.managedInstalledBrowserErrorDescription,
                retryLabel: l10n.managedDashboardRetry,
              ),
              openSettings: () => Revision3ProjectWorkspace.navigate(
                context,
                const Revision3ProjectWorkspaceLocation(
                  Revision3ProjectWorkspaceSection.settingsExpert,
                ),
              ),
              openInspector: gameRoot == null
                  ? null
                  : (targetPath) => unawaited(
                      _openInstalledPackageBrowser(
                        context,
                        gameRoot!,
                        initialQuery: targetPath,
                      ),
                    ),
            ),
            allSources: Builder(
              builder: (globalContext) => Revision3GlobalContentSearchView(
                controller: globalSearchController,
                copy: _globalContentSearchCopy(l10n),
                callbacks: Revision3GlobalContentSearchCallbacks(
                  openThisModEntity: (entityId) {
                    Revision3ScopedContentBrowser.navigate(
                      globalContext,
                      Revision3ScopedContentScope.thisMod,
                    );
                    unawaited(
                      contentLibraryController.openEntityById(entityId).then((
                        open,
                      ) {
                        if (!open && context.mounted) {
                          ScaffoldMessenger.of(context).showSnackBar(
                            SnackBar(
                              content: Text(
                                l10n.managedGlobalSearchResultStale,
                              ),
                            ),
                          );
                        }
                      }),
                    );
                  },
                  openThisModAsset: (sha256) {
                    Revision3ScopedContentBrowser.navigate(
                      globalContext,
                      Revision3ScopedContentScope.thisMod,
                    );
                    unawaited(
                      contentLibraryController.openAssetBySha256(sha256).then((
                        open,
                      ) {
                        if (!open && context.mounted) {
                          ScaffoldMessenger.of(context).showSnackBar(
                            SnackBar(
                              content: Text(
                                l10n.managedGlobalSearchResultStale,
                              ),
                            ),
                          );
                        }
                      }),
                    );
                  },
                  createBaseNpcDraft: (catalogId) => unawaited(
                    _openNpcWizard(context, initialCatalogId: catalogId),
                  ),
                  createBaseQuestDraft: (catalogId) => unawaited(
                    _openQuestWizard(
                      context,
                      initialParentCatalogId: catalogId,
                    ),
                  ),
                  inspectInstalledDataAsset: (targetPath) {
                    final configuredGameRoot = gameRoot;
                    if (configuredGameRoot == null) return;
                    unawaited(
                      _openInstalledPackageBrowser(
                        context,
                        configuredGameRoot,
                        initialQuery: targetPath,
                        initialTargetPath: targetPath,
                      ),
                    );
                  },
                ),
              ),
            ),
          ),
    ),
    dataAssets: Revision3DataAssetStagePanel(
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
      semanticExtractReceiptPicker: pickDataAssetExtractReceipt,
      semanticExtractReceiptInspector: inspectDataAssetExtractReceipt,
      browseInstalledPackages: gameRoot == null
          ? null
          : () => _openInstalledPackageBrowser(context, gameRoot!),
    ),
  );

  Widget _buildStorySection(BuildContext context, AppLocalizations l10n) {
    final gameConfigured = gameRoot != null;
    return Revision3ProjectSectionPage(
      sectionId: 'story',
      icon: Icons.menu_book_outlined,
      title: l10n.managedWorkspaceStoryLabel,
      description: l10n.managedSectionStoryDescription,
      notice: gameConfigured
          ? null
          : l10n.managedDashboardMissingGameDescription,
      actionHeading: l10n.managedSectionActionsHeading,
      actionCards: [
        Revision3ProjectSectionActionCard(
          id: 'create-npc-draft',
          icon: Icons.person_add_alt_1_outlined,
          title: l10n.managedActionNewNpcTitle,
          description: l10n.managedActionNewNpcDescription,
          badge: l10n.managedCapabilityPartial,
          onPressed: gameConfigured
              ? () => unawaited(_openNpcWizard(context))
              : null,
        ),
        Revision3ProjectSectionActionCard(
          id: 'create-quest-draft',
          icon: Icons.assignment_add,
          title: l10n.managedActionNewQuestTitle,
          description: l10n.managedActionNewQuestDescription,
          badge: l10n.managedCapabilityPartial,
          onPressed: gameConfigured
              ? () => unawaited(_openQuestWizard(context))
              : null,
        ),
        Revision3ProjectSectionActionCard(
          id: 'browse-story-content',
          icon: Icons.account_tree_outlined,
          title: l10n.managedWorkspaceContentLabel,
          description: l10n.managedActionBrowseProjectContentDescription,
          badge: l10n.managedCapabilityAvailable,
          onPressed: () => Revision3ProjectWorkspace.navigate(
            context,
            const Revision3ProjectWorkspaceLocation(
              Revision3ProjectWorkspaceSection.content,
            ),
          ),
        ),
      ],
    );
  }

  Widget _buildWorldSection(AppLocalizations l10n) =>
      Revision3ProjectSectionPage(
        sectionId: 'world',
        icon: Icons.public_outlined,
        title: l10n.managedWorkspaceWorldLabel,
        description: l10n.managedSectionWorldDescription,
        statusHeading: l10n.managedSectionStatusHeading,
        statusCards: [
          Revision3ProjectSectionStatusCard(
            id: 'world-authoring',
            icon: Icons.construction_outlined,
            title: l10n.managedCapabilityPlanned,
            description: l10n.managedSectionWorldDescription,
          ),
        ],
      );

  Widget _buildLocalizationVoiceSection(
    BuildContext context,
    AppLocalizations l10n,
  ) {
    final gameConfigured = gameRoot != null;
    return Revision3ProjectSectionPage(
      sectionId: 'localization-voice',
      icon: Icons.record_voice_over_outlined,
      title: l10n.managedWorkspaceLocalizationVoiceLabel,
      description: l10n.managedSectionLocalizationVoiceDescription,
      notice: gameConfigured
          ? null
          : l10n.managedDashboardMissingGameDescription,
      statusHeading: l10n.managedSectionStatusHeading,
      statusCards: [
        Revision3ProjectSectionStatusCard(
          id: 'managed-localization',
          icon: Icons.translate_outlined,
          title: l10n.managedCapabilityPlanned,
          description: l10n.managedSectionLocalizationVoiceDescription,
        ),
      ],
      actionHeading: l10n.managedSectionActionsHeading,
      actionCards: [
        Revision3ProjectSectionActionCard(
          id: 'add-voice-take',
          icon: Icons.mic_none_outlined,
          title: l10n.managedActionAddVoiceTakeTitle,
          description: l10n.managedActionAddVoiceTakeDescription,
          badge: l10n.managedCapabilityPartial,
          onPressed: gameConfigured
              ? () => unawaited(_openVoiceWizard(context))
              : null,
        ),
        Revision3ProjectSectionActionCard(
          id: 'manage-voice-takes',
          icon: Icons.library_music_outlined,
          title: l10n.managedActionManageVoiceTakesTitle,
          description: l10n.managedActionManageVoiceTakesDescription,
          badge: l10n.managedCapabilityAvailable,
          onPressed: () => unawaited(_openVoiceTakeSelection(context)),
        ),
        Revision3ProjectSectionActionCard(
          id: 'resolve-voice-target',
          icon: Icons.link_outlined,
          title: l10n.managedActionResolveVoiceTargetTitle,
          description: l10n.managedActionResolveVoiceTargetDescription,
          badge: l10n.managedCapabilityPartial,
          onPressed: gameConfigured
              ? () => unawaited(_openVoiceTargetResolver(context))
              : null,
        ),
      ],
    );
  }

  Widget _buildValidateTestSection(
    BuildContext context,
    AppLocalizations l10n,
  ) => Revision3ProjectSectionPage(
    sectionId: 'validate-test',
    icon: Icons.fact_check_outlined,
    title: l10n.managedWorkspaceValidateTestLabel,
    description: l10n.managedSectionValidateTestDescription,
    statusHeading: l10n.managedSectionStatusHeading,
    statusCards: [
      Revision3ProjectSectionStatusCard(
        id: 'reference-integrity',
        icon: Icons.account_tree_outlined,
        title: l10n.managedDashboardReferenceIntegrityTitle,
        description: l10n.managedActionBrowseProjectContentDescription,
      ),
      Revision3ProjectSectionStatusCard(
        id: 'runtime-test',
        icon: Icons.science_outlined,
        title: l10n.managedDashboardRuntimeUnqualifiedTitle,
        description: l10n.managedDashboardRuntimeUnqualifiedDescription,
        severity: Revision3ProjectSectionStatusSeverity.warning,
      ),
    ],
    actionHeading: l10n.managedSectionActionsHeading,
    actionCards: [
      Revision3ProjectSectionActionCard(
        id: 'verify-current-head',
        icon: Icons.verified_user_outlined,
        title: l10n.projectVerifyCurrentHead,
        description: l10n.managedSectionValidateTestDescription,
        badge: l10n.managedCapabilityAvailable,
        onPressed: () => unawaited(verifyCurrentHead()),
      ),
      Revision3ProjectSectionActionCard(
        id: 'inspect-references',
        icon: Icons.account_tree_outlined,
        title: l10n.managedDashboardReferenceIntegrityTitle,
        description: l10n.managedActionBrowseProjectContentDescription,
        badge: l10n.managedCapabilityAvailable,
        onPressed: () => Revision3ProjectWorkspace.navigate(
          context,
          const Revision3ProjectWorkspaceLocation(
            Revision3ProjectWorkspaceSection.content,
          ),
        ),
      ),
    ],
  );

  Widget _buildReleaseSection(BuildContext context, AppLocalizations l10n) =>
      Revision3ProjectSectionPage(
        sectionId: 'build-release',
        icon: Icons.inventory_2_outlined,
        title: l10n.managedWorkspaceBuildReleaseLabel,
        description: l10n.managedSectionBuildReleaseDescription,
        notice: gameRoot == null
            ? l10n.managedDashboardMissingGameDescription
            : null,
        statusHeading: l10n.managedSectionStatusHeading,
        statusCards: [
          Revision3ProjectSectionStatusCard(
            id: 'full-mod-build',
            icon: Icons.block_outlined,
            title: l10n.managedDashboardGeneralBuildBlockedTitle,
            description: l10n.managedDashboardGeneralBuildBlockedDescription,
            severity: Revision3ProjectSectionStatusSeverity.blocked,
          ),
          Revision3ProjectSectionStatusCard(
            id: 'runtime-qualification',
            icon: Icons.science_outlined,
            title: l10n.managedDashboardRuntimeUnqualifiedTitle,
            description: l10n.managedDashboardRuntimeUnqualifiedDescription,
            severity: Revision3ProjectSectionStatusSeverity.warning,
          ),
        ],
        actionHeading: l10n.managedSectionActionsHeading,
        actionCards: [
          Revision3ProjectSectionActionCard(
            id: 'build-voice-bundle',
            icon: Icons.library_music_outlined,
            title: l10n.managedActionBuildVoiceBundleTitle,
            description: l10n.managedActionBuildVoiceBundleDescription,
            badge: l10n.managedCapabilityPartial,
            onPressed: gameRoot == null
                ? null
                : () => unawaited(_openVoiceBuild(context)),
          ),
          Revision3ProjectSectionActionCard(
            id: 'build-playable-mod',
            icon: Icons.rocket_launch_outlined,
            title: l10n.managedDashboardGeneralBuildBlockedTitle,
            description: l10n.managedDashboardGeneralBuildBlockedDescription,
            badge: l10n.managedCapabilityUnavailable,
          ),
        ],
      );

  Widget _buildDashboard(BuildContext context, AppLocalizations l10n) {
    final gameConfigured = gameRoot != null;
    VoidCallback? requiresGame(Future<void> Function(BuildContext) action) =>
        gameConfigured ? () => unawaited(action(context)) : null;

    return Revision3ProjectDashboard(
      projectId: project.projectId,
      projectRevision: project.projectRevision,
      load: loadContentIndex,
      gameConfigured: gameConfigured,
      copy: Revision3ProjectDashboardCopy(
        untitledProjectLabel: l10n.managedDashboardUntitledProject,
        draftStatusLabel: l10n.managedDashboardDraftStatus,
        projectVersionLabel: l10n.managedDashboardProjectVersion,
        projectAuthorLabel: l10n.managedDashboardProjectAuthor,
        notProvidedLabel: l10n.managedDashboardNotProvided,
        contentCountsHeading: l10n.managedDashboardContentCounts,
        npcDraftCountLabel: l10n.managedDashboardNpcDrafts,
        questDraftCountLabel: l10n.managedDashboardQuestDrafts,
        dialogLineCountLabel: l10n.managedDashboardDialogLines,
        voiceTakeCountLabel: l10n.managedDashboardVoiceTakes,
        assetCountLabel: l10n.managedDashboardAssets,
        unresolvedReferenceCountLabel:
            l10n.managedDashboardUnresolvedReferences,
        readinessHeading: l10n.managedDashboardReadiness,
        offlineAuthoringTitle: l10n.managedDashboardOfflineAuthoringTitle,
        offlineAuthoringDescription:
            l10n.managedDashboardOfflineAuthoringDescription,
        generalBuildBlockedTitle: l10n.managedDashboardGeneralBuildBlockedTitle,
        generalBuildBlockedDescription:
            l10n.managedDashboardGeneralBuildBlockedDescription,
        runtimeUnqualifiedTitle: l10n.managedDashboardRuntimeUnqualifiedTitle,
        runtimeUnqualifiedDescription:
            l10n.managedDashboardRuntimeUnqualifiedDescription,
        referenceIntegrityTitle: l10n.managedDashboardReferenceIntegrityTitle,
        referenceIntegrityDescription:
            l10n.managedDashboardReferenceIntegrityDescription,
        missingGameTitle: l10n.managedDashboardMissingGameTitle,
        missingGameDescription: l10n.managedDashboardMissingGameDescription,
        createHeading: l10n.managedDashboardCreateHeading,
        toolsHeading: l10n.managedDashboardToolsHeading,
        loadingSemanticsLabel: l10n.managedDashboardLoading,
        loadErrorSemanticsLabel: l10n.managedDashboardLoadError,
        loadErrorTitle: l10n.managedDashboardLoadError,
        loadErrorDescription: l10n.managedDashboardLoadErrorDescription,
        retryLabel: l10n.managedDashboardRetry,
      ),
      createActions: [
        Revision3ProjectDashboardAction(
          id: 'create-npc-draft',
          controlKey: const Key('managed-create-npc-draft'),
          icon: Icons.person_add_alt_1_outlined,
          title: l10n.managedActionNewNpcTitle,
          description: l10n.managedActionNewNpcDescription,
          onPressed: requiresGame(_openNpcWizard),
        ),
        Revision3ProjectDashboardAction(
          id: 'create-quest-draft',
          controlKey: const Key('managed-create-quest-draft'),
          icon: Icons.assignment_add,
          title: l10n.managedActionNewQuestTitle,
          description: l10n.managedActionNewQuestDescription,
          onPressed: requiresGame(_openQuestWizard),
        ),
        Revision3ProjectDashboardAction(
          id: 'add-voice-take',
          controlKey: const Key('managed-add-voice-take'),
          icon: Icons.record_voice_over_outlined,
          title: l10n.managedActionAddVoiceTakeTitle,
          description: l10n.managedActionAddVoiceTakeDescription,
          onPressed: requiresGame(_openVoiceWizard),
        ),
      ],
      toolActions: [
        Revision3ProjectDashboardAction(
          id: 'manage-voice-takes',
          controlKey: const Key('managed-manage-voice-takes'),
          icon: Icons.library_music_outlined,
          title: l10n.managedActionManageVoiceTakesTitle,
          description: l10n.managedActionManageVoiceTakesDescription,
          onPressed: () => unawaited(_openVoiceTakeSelection(context)),
        ),
        Revision3ProjectDashboardAction(
          id: 'resolve-voice-target',
          controlKey: const Key('managed-resolve-voice-target'),
          icon: Icons.link_outlined,
          title: l10n.managedActionResolveVoiceTargetTitle,
          description: l10n.managedActionResolveVoiceTargetDescription,
          onPressed: requiresGame(_openVoiceTargetResolver),
        ),
        Revision3ProjectDashboardAction(
          id: 'build-voice-bundle',
          controlKey: const Key('managed-build-voice-bundle'),
          icon: Icons.inventory_2_outlined,
          title: l10n.managedActionBuildVoiceBundleTitle,
          description: l10n.managedActionBuildVoiceBundleDescription,
          onPressed: requiresGame(_openVoiceBuild),
        ),
        Revision3ProjectDashboardAction(
          id: 'verified-dataasset-edits',
          icon: Icons.data_object_outlined,
          title: l10n.managedActionDataAssetsTitle,
          description: l10n.managedActionDataAssetsDescription,
          onPressed: () => Revision3ProjectWorkspace.navigate(
            context,
            const Revision3ProjectWorkspaceLocation(
              Revision3ProjectWorkspaceSection.content,
              secondary: 'data-assets',
            ),
          ),
        ),
      ],
      settingsAction: Revision3ProjectDashboardAction(
        id: 'open-settings',
        icon: Icons.settings_outlined,
        title: l10n.managedActionSettingsTitle,
        description: l10n.managedActionSettingsDescription,
        onPressed: () => Revision3ProjectWorkspace.navigate(
          context,
          const Revision3ProjectWorkspaceLocation(
            Revision3ProjectWorkspaceSection.settingsExpert,
          ),
        ),
      ),
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

  Future<void> _openVoiceTakeSelection(BuildContext context) async {
    if (project.requiresReopen) return;
    final publication =
        await showDialog<Revision3VoiceTakeSelectionPublication>(
          context: context,
          builder: (context) => Revision3VoiceTakeSelectionDialog(
            service: Revision3VoiceTakeSelectionAuthoringService(
              loadContentIndex: loadContentIndex,
              publishTechnicalPlan: publishVoiceTakeSelection,
            ),
          ),
        );
    if (!context.mounted || publication == null) return;
    final outcome = publication.cleared
        ? 'Voice selection cleared'
        : 'Approved Voice take selected';
    ScaffoldMessenger.of(context).showSnackBar(
      SnackBar(
        content: Text(
          '$outcome in project revision ${publication.projectRevision}. Voice build remains a separate offline step; runtime remains unqualified.',
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

  Future<void> _openQuestWizard(
    BuildContext context, {
    String? initialParentCatalogId,
    String? initialGiverCatalogId,
  }) async {
    final configuredGameRoot = gameRoot;
    if (configuredGameRoot == null || project.requiresReopen) return;
    final publication = await showDialog<Revision3QuestDraftPublication>(
      context: context,
      builder: (context) => Revision3QuestWizardDialog(
        gameRoot: configuredGameRoot,
        loadCatalog: loadQuestCatalog,
        publish: publishQuestDraft,
        initialParentCatalogId: initialParentCatalogId,
        initialGiverCatalogId: initialGiverCatalogId,
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

  Future<void> _openQuestOutlineEditor(
    BuildContext context,
    Revision3ContentIndex index,
    Revision3ContentEntity quest,
  ) async {
    if (project.requiresReopen) return;
    final publication = await showDialog<Revision3QuestOutlineEditPublication>(
      context: context,
      builder: (context) => Revision3QuestOutlineEditDialog(
        index: index,
        quest: quest,
        publish: editQuestOutline,
        loadTransitionSeed: loadQuestTransitionsSeed,
      ),
    );
    if (!context.mounted || publication == null) return;
    ScaffoldMessenger.of(context).showSnackBar(
      SnackBar(
        content: Text(
          'Quest outline saved in project revision ${publication.projectRevision}. Build remains blocked; runtime remains unqualified.',
        ),
      ),
    );
  }

  Future<void> _openQuestContextEditor(
    BuildContext context,
    Revision3ContentIndex index,
    Revision3ContentEntity quest,
  ) async {
    final configuredGameRoot = gameRoot;
    if (configuredGameRoot == null || project.requiresReopen) return;
    final publication = await showDialog<Revision3QuestContextEditPublication>(
      context: context,
      barrierDismissible: false,
      builder: (context) => Revision3QuestContextEditDialog(
        index: index,
        quest: quest,
        gameRoot: configuredGameRoot,
        service: Revision3QuestContextAuthoringService(
          loadSeed: loadQuestContextSeed,
          loadCatalog: loadQuestCatalog,
          publishTechnicalPlan: editQuestContext,
        ),
      ),
    );
    if (!context.mounted || publication == null) return;
    ScaffoldMessenger.of(context).showSnackBar(
      SnackBar(
        content: Text(
          'Quest description and connections saved in project revision ${publication.projectRevision}. Build remains blocked; runtime remains unqualified.',
        ),
      ),
    );
  }

  Future<void> _openQuestTransitionsEditor(
    BuildContext context,
    Revision3ContentIndex index,
    Revision3ContentEntity quest,
  ) async {
    if (project.requiresReopen) return;
    final publication =
        await showDialog<Revision3QuestTransitionsEditPublication>(
          context: context,
          barrierDismissible: false,
          builder: (context) => Revision3QuestTransitionsEditDialog(
            index: index,
            quest: quest,
            service: Revision3QuestTransitionsAuthoringService(
              loadSeed: loadQuestTransitionsSeed,
              publishTechnicalPlan: editQuestTransitions,
            ),
          ),
        );
    if (!context.mounted || publication == null) return;
    ScaffoldMessenger.of(context).showSnackBar(
      SnackBar(
        content: Text(
          'Quest states and transitions saved in project revision ${publication.projectRevision}. Build remains blocked; runtime remains unqualified.',
        ),
      ),
    );
  }

  Future<void> _openQuestSourceInspection(
    BuildContext context,
    Revision3ContentIndex index,
    Revision3ContentEntity quest,
  ) async {
    final configuredGameRoot = gameRoot;
    if (configuredGameRoot == null || project.requiresReopen) return;
    final compilerSelection = _revision3ManagedCompilerSelection(
      index: index,
      entity: quest,
      expectedKind: Revision3ContentEntityKind.questDraft,
    );
    await showDialog<void>(
      context: context,
      barrierDismissible: false,
      builder: (context) => Revision3QuestSourceInspectionDialog(
        questTitle: quest.summary.primaryIdentity,
        questId: quest.id,
        gameRoot: configuredGameRoot,
        inspect: inspectQuestSource,
        checkCompiler: compilerSelection == null
            ? null
            : () => checkManagedCompiler(
                entityKind:
                    AuthoringRevision3ManagedCompilerEntityKind.questDraft,
                entityId: compilerSelection.entityId,
                expectedEntityRevision: compilerSelection.entityRevision,
                expectedModuleId: compilerSelection.moduleId,
                expectedModuleRevision: compilerSelection.moduleRevision,
                gameRoot: configuredGameRoot,
              ),
      ),
    );
  }

  Future<void> _openNpcProfile(
    BuildContext context,
    Revision3ContentIndex index,
    Revision3ContentEntity npc,
  ) async {
    if (project.requiresReopen) return;
    final configuredGameRoot = gameRoot;
    final compilerSelection = configuredGameRoot == null
        ? null
        : _revision3ManagedCompilerSelection(
            index: index,
            entity: npc,
            expectedKind: Revision3ContentEntityKind.npcDraft,
          );
    await showDialog<void>(
      context: context,
      barrierDismissible: false,
      builder: (context) => Revision3NpcProfileDialog(
        npcTitle: npc.summary.primaryIdentity,
        npcId: npc.id,
        inspect: inspectNpcSource,
        gameRoot: configuredGameRoot,
        checkCompiler: compilerSelection == null || configuredGameRoot == null
            ? null
            : () => checkManagedCompiler(
                entityKind:
                    AuthoringRevision3ManagedCompilerEntityKind.npcDraft,
                entityId: compilerSelection.entityId,
                expectedEntityRevision: compilerSelection.entityRevision,
                expectedModuleId: compilerSelection.moduleId,
                expectedModuleRevision: compilerSelection.moduleRevision,
                gameRoot: configuredGameRoot,
              ),
      ),
    );
  }

  Future<void> _openInstalledPackageBrowser(
    BuildContext context,
    String configuredGameRoot, {
    String initialQuery = '',
    String? initialTargetPath,
  }) => showDialog<void>(
    context: context,
    builder: (context) => InstalledPackageBrowserDialog(
      gameRoot: configuredGameRoot,
      load: loadInstalledPackageIndex,
      inspect: inspectInstalledDataAsset,
      publish: publishInstalledDataAssetSemanticEdit,
      publishReviewed: publishReviewedInstalledDataAssetEdit,
      initialQuery: initialQuery,
      initialTargetPath: initialTargetPath,
    ),
  );

  Future<void> _openNpcWizard(
    BuildContext context, {
    String? initialCatalogId,
  }) async {
    final configuredGameRoot = gameRoot;
    if (configuredGameRoot == null || project.requiresReopen) return;
    final publication = await showDialog<Revision3NpcDraftPublication>(
      context: context,
      builder: (context) => Revision3NpcWizardDialog(
        gameRoot: configuredGameRoot,
        loadCatalog: loadNpcCatalog,
        publish: publishNpcDraft,
        chooseArchetype: chooseNpcArchetype,
        initialCatalogId: initialCatalogId,
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

  Future<void> _openSettings(BuildContext context) =>
      _showModStudioSettingsDialog(context);
}

class _ManagedRevision3GlobalContentHost extends StatefulWidget {
  const _ManagedRevision3GlobalContentHost({
    required this.sourceIdentity,
    required this.loadThisMod,
    required this.loadBaseGame,
    required this.loadInstalled,
    required this.builder,
  });

  final Revision3GlobalContentSearchSourceIdentity sourceIdentity;
  final Revision3GlobalThisModContentLoader loadThisMod;
  final Revision3GlobalBaseGameContentLoader loadBaseGame;
  final Revision3GlobalInstalledContentLoader loadInstalled;
  final Widget Function(
    BuildContext context,
    Revision3ContentLibraryController contentLibraryController,
    Revision3GlobalContentSearchController globalSearchController,
  )
  builder;

  @override
  State<_ManagedRevision3GlobalContentHost> createState() =>
      _ManagedRevision3GlobalContentHostState();
}

class _ManagedRevision3GlobalContentHostState
    extends State<_ManagedRevision3GlobalContentHost> {
  final _contentLibraryController = Revision3ContentLibraryController();
  late final Revision3GlobalContentSearchController _globalSearchController;

  @override
  void initState() {
    super.initState();
    _globalSearchController = Revision3GlobalContentSearchController(
      loadThisMod: widget.loadThisMod,
      loadBaseGame: widget.loadBaseGame,
      loadInstalled: widget.loadInstalled,
      sourceIdentity: widget.sourceIdentity,
    );
  }

  @override
  void didUpdateWidget(covariant _ManagedRevision3GlobalContentHost oldWidget) {
    super.didUpdateWidget(oldWidget);
    _globalSearchController.updateSources(
      loadThisMod: widget.loadThisMod,
      loadBaseGame: widget.loadBaseGame,
      loadInstalled: widget.loadInstalled,
      sourceIdentity: widget.sourceIdentity,
    );
  }

  @override
  void dispose() {
    _globalSearchController.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) => widget.builder(
    context,
    _contentLibraryController,
    _globalSearchController,
  );
}

Revision3GlobalContentSearchCopy _globalContentSearchCopy(
  AppLocalizations l10n,
) => Revision3GlobalContentSearchCopy(
  title: l10n.managedGlobalSearchTitle,
  searchLabel: l10n.managedGlobalSearchLabel,
  searchAction: l10n.managedGlobalSearchAction,
  clearAction: l10n.managedGlobalSearchClear,
  emptyPrompt: l10n.managedGlobalSearchPrompt,
  noResults: l10n.managedGlobalSearchNoResults,
  loading: l10n.managedGlobalSearchLoading,
  loadFailed: l10n.managedGlobalSearchFailed,
  retry: l10n.managedDashboardRetry,
  partial: l10n.managedGlobalSearchPartial,
  complete: l10n.managedGlobalSearchComplete,
  truncated: l10n.managedGlobalSearchTruncated,
  openAction: l10n.managedGlobalSearchOpen,
  createDraftAction: l10n.managedGlobalSearchCreateDraft,
  inspectAction: l10n.managedGlobalSearchInspect,
  sourceLabels: <Revision3GlobalContentSource, String>{
    Revision3GlobalContentSource.thisMod:
        l10n.managedContentWorkspaceLibraryLabel,
    Revision3GlobalContentSource.baseGame:
        l10n.managedContentScopeBaseGameLabel,
    Revision3GlobalContentSource.installed:
        l10n.managedContentScopeInstalledLabel,
  },
  kindLabels: <Revision3GlobalContentKind, String>{
    Revision3GlobalContentKind.thisModEntity:
        l10n.managedGlobalSearchKindModEntity,
    Revision3GlobalContentKind.thisModAsset:
        l10n.managedGlobalSearchKindModAsset,
    Revision3GlobalContentKind.baseNpc: l10n.managedGlobalSearchKindBaseNpc,
    Revision3GlobalContentKind.baseQuest: l10n.managedGlobalSearchKindBaseQuest,
    Revision3GlobalContentKind.experimentalBaseNpc:
        l10n.managedGlobalSearchKindExperimentalNpc,
    Revision3GlobalContentKind.installedDataAsset:
        l10n.managedInstalledBrowserKindBadge,
  },
  readinessLabels: <Revision3GlobalContentReadiness, String>{
    Revision3GlobalContentReadiness.exactCurrent:
        l10n.managedGlobalSearchReadinessExact,
    Revision3GlobalContentReadiness.exactCurrentWithProblems:
        l10n.managedGlobalSearchReadinessProblems,
    Revision3GlobalContentReadiness.offlineDraftRuntimeUnqualified:
        l10n.managedBaseGameBrowserOfflineDraftBadge,
    Revision3GlobalContentReadiness.inspectOnlyRuntimeUnqualified:
        l10n.managedBaseGameBrowserInspectOnlyBadge,
    Revision3GlobalContentReadiness.metadataOnlyRuntimeUnqualified:
        l10n.managedInstalledBrowserMetadataOnlyBadge,
  },
);

Future<void> _showModStudioSettingsDialog(BuildContext context) =>
    showDialog<void>(
      context: context,
      builder: (dialogContext) => Dialog(
        key: const Key('managed-settings-dialog'),
        child: ConstrainedBox(
          constraints: const BoxConstraints(maxWidth: 800, maxHeight: 700),
          child: Column(
            children: [
              Padding(
                padding: const EdgeInsets.fromLTRB(24, 16, 12, 8),
                child: Row(
                  children: [
                    const Icon(Icons.settings_outlined),
                    const SizedBox(width: 10),
                    Expanded(
                      child: Text(
                        AppLocalizations.of(
                          dialogContext,
                        ).managedActionSettingsTitle,
                        style: Theme.of(dialogContext).textTheme.titleLarge,
                      ),
                    ),
                    IconButton(
                      key: const Key('managed-settings-close'),
                      onPressed: () => Navigator.of(dialogContext).pop(),
                      tooltip: AppLocalizations.of(dialogContext).close,
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
  const _NoCurrentProjectView({
    required this.onCreateManaged,
    required this.onOpenManaged,
    required this.onOpenSettings,
  });

  final VoidCallback? onCreateManaged;
  final VoidCallback? onOpenManaged;
  final VoidCallback? onOpenSettings;

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context);
    return SingleChildScrollView(
      key: const Key('managed-project-landing-scroll'),
      padding: const EdgeInsets.all(24),
      child: Align(
        alignment: Alignment.topCenter,
        child: ConstrainedBox(
          constraints: const BoxConstraints(maxWidth: 920),
          child: _ManagedProjectEntryBanner(
            bannerKey: const Key('managed-project-landing'),
            title: l10n.managedProjectLandingTitle,
            description: l10n.managedProjectLandingDescription,
            createLabel: l10n.projectNewManagedRevision3,
            openLabel: l10n.projectOpenManagedRevision3,
            settingsLabel: l10n.managedActionSettingsTitle,
            onCreateManaged: onCreateManaged,
            onOpenManaged: onOpenManaged,
            onOpenSettings: onOpenSettings,
          ),
        ),
      ),
    );
  }
}

class _ManagedProjectEntryBanner extends StatelessWidget {
  const _ManagedProjectEntryBanner({
    required this.bannerKey,
    required this.title,
    required this.description,
    required this.createLabel,
    required this.openLabel,
    required this.onCreateManaged,
    required this.onOpenManaged,
    this.settingsLabel,
    this.onOpenSettings,
    this.legacyTitle,
    this.legacyDescription,
  }) : assert(
         (legacyTitle == null) == (legacyDescription == null),
         'Legacy title and description must be provided together.',
       );

  final Key bannerKey;
  final String title;
  final String description;
  final String createLabel;
  final String openLabel;
  final String? settingsLabel;
  final VoidCallback? onCreateManaged;
  final VoidCallback? onOpenManaged;
  final VoidCallback? onOpenSettings;
  final String? legacyTitle;
  final String? legacyDescription;

  @override
  Widget build(BuildContext context) {
    final scheme = Theme.of(context).colorScheme;
    final identity = Row(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Container(
          width: 44,
          height: 44,
          decoration: BoxDecoration(
            color: scheme.primaryContainer,
            borderRadius: BorderRadius.circular(12),
          ),
          child: Icon(
            Icons.dashboard_customize_outlined,
            color: scheme.onPrimaryContainer,
          ),
        ),
        const SizedBox(width: 12),
        Expanded(
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Semantics(
                container: true,
                header: true,
                child: Text(
                  title,
                  key: const Key('managed-project-entry-title'),
                  style: Theme.of(context).textTheme.titleLarge,
                ),
              ),
              const SizedBox(height: 4),
              Text(
                description,
                key: const Key('managed-project-entry-description'),
              ),
            ],
          ),
        ),
      ],
    );
    final actions = Wrap(
      spacing: 8,
      runSpacing: 8,
      alignment: WrapAlignment.end,
      children: [
        FilledButton.icon(
          key: const Key('managed-project-entry-create'),
          onPressed: onCreateManaged,
          icon: const Icon(Icons.add),
          label: Text(createLabel),
        ),
        OutlinedButton.icon(
          key: const Key('managed-project-entry-open'),
          onPressed: onOpenManaged,
          icon: const Icon(Icons.folder_open_outlined),
          label: Text(openLabel),
        ),
        if (settingsLabel != null)
          TextButton.icon(
            key: const Key('managed-project-entry-settings'),
            onPressed: onOpenSettings,
            icon: const Icon(Icons.settings_outlined),
            label: Text(settingsLabel!),
          ),
      ],
    );

    return Material(
      key: bannerKey,
      color: scheme.surfaceContainerLow,
      shape: RoundedRectangleBorder(
        borderRadius: BorderRadius.circular(16),
        side: BorderSide(color: scheme.outlineVariant),
      ),
      clipBehavior: Clip.antiAlias,
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: LayoutBuilder(
          builder: (context, constraints) => Column(
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: [
              if (constraints.maxWidth < 1080) ...[
                identity,
                const SizedBox(height: 12),
                Align(alignment: Alignment.centerLeft, child: actions),
              ] else
                Row(
                  crossAxisAlignment: CrossAxisAlignment.center,
                  children: [
                    Expanded(child: identity),
                    const SizedBox(width: 20),
                    ConstrainedBox(
                      constraints: const BoxConstraints(maxWidth: 640),
                      child: actions,
                    ),
                  ],
                ),
              if (legacyTitle != null) ...[
                const SizedBox(height: 14),
                const Divider(height: 1),
                const SizedBox(height: 10),
                Row(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Icon(
                      Icons.build_circle_outlined,
                      size: 20,
                      color: scheme.onSurfaceVariant,
                    ),
                    const SizedBox(width: 8),
                    Expanded(
                      child: Text.rich(
                        TextSpan(
                          children: [
                            TextSpan(
                              text: '$legacyTitle — ',
                              style: const TextStyle(
                                fontWeight: FontWeight.w600,
                              ),
                            ),
                            TextSpan(text: legacyDescription),
                          ],
                        ),
                        key: const Key(
                          'legacy-compatibility-tools-description',
                        ),
                        style: TextStyle(color: scheme.onSurfaceVariant),
                      ),
                    ),
                  ],
                ),
              ],
            ],
          ),
        ),
      ),
    );
  }
}
