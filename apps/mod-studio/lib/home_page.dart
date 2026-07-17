import 'dart:async';
import 'dart:io';

import 'package:file_selector/file_selector.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:path/path.dart' as p;
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
import 'project/managed_project_session.dart';
import 'project/project_controller.dart';
import 'project/revision3_base_game_content_browser.dart';
import 'project/revision3_content_index.dart';
import 'project/revision3_content_library.dart';
import 'project/revision3_content_workspace.dart';
import 'project/revision3_story_entity_workbench.dart';
import 'project/revision3_story_workspace.dart';
import 'project/revision3_dataasset_authoring.dart';
import 'project/revision3_dataasset_build_dialog.dart';
import 'project/revision3_dataasset_stage_panel.dart';
import 'project/revision3_dialog_line_authoring.dart';
import 'project/revision3_dialog_line_dialog.dart';
import 'project/revision3_dialog_localization_authoring.dart';
import 'project/revision3_dialog_voice_slot_creation_authoring.dart';
import 'project/revision3_dialog_voice_slot_removal_authoring.dart';
import 'project/revision3_global_content_search.dart';
import 'project/revision3_global_content_search_view.dart';
import 'project/revision3_npc_authoring.dart';
import 'project/revision3_npc_profile_edit_authoring.dart';
import 'project/revision3_npc_profile_edit_dialog.dart';
import 'project/revision3_npc_profile_dialog.dart';
import 'project/revision3_managed_compiler_check_panel.dart';
import 'project/revision3_npc_opening_recipe.dart';
import 'project/revision3_npc_wizard.dart';
import 'project/revision3_quest_authoring.dart';
import 'project/revision3_quest_context_authoring.dart';
import 'project/revision3_quest_context_dialog.dart';
import 'project/revision3_quest_journey_panel.dart';
import 'project/revision3_quest_journey_service.dart';
import 'project/revision3_quest_journey_view.dart';
import 'project/revision3_quest_opening_recipe.dart';
import 'project/revision3_quest_outline_authoring.dart';
import 'project/revision3_quest_outline_dialog.dart';
import 'project/revision3_quest_source_inspection_dialog.dart';
import 'project/revision3_quest_transcript_authoring.dart';
import 'project/revision3_quest_transcript_panel.dart';
import 'project/revision3_npc_greeting_authoring.dart';
import 'project/revision3_npc_dialog_voice_panel.dart';
import 'project/revision3_quest_transitions_authoring.dart';
import 'project/revision3_quest_transitions_dialog.dart';
import 'project/revision3_quest_wizard.dart';
import 'project/revision3_project_create_dialog.dart';
import 'project/revision3_project_dashboard.dart';
import 'project/revision3_project_export_dialog.dart';
import 'project/revision3_project_history.dart';
import 'project/revision3_project_history_page.dart';
import 'project/revision3_project_problems.dart';
import 'project/revision3_project_problems_view.dart';
import 'project/revision3_project_command_bar.dart';
import 'project/revision3_project_section_page.dart';
import 'project/revision3_project_workspace.dart';
import 'project/revision3_scoped_content_browser.dart';
import 'project/revision3_settings_expert_page.dart';
import 'project/revision3_installed_content_browser.dart';
import 'project/revision3_localization_voice_workspace.dart';
import 'project/revision3_voice_authoring.dart';
import 'project/revision3_voice_production_card.dart';
import 'project/revision3_voice_production_queue_view.dart';
import 'project/revision3_voice_build_dialog.dart';
import 'project/revision3_voice_build_readiness_panel.dart';
import 'project/revision3_voice_folder_authoring.dart';
import 'project/revision3_voice_folder_import_dialog.dart';
import 'project/revision3_voice_folder_managed_adapter.dart';
import 'project/revision3_voice_take_removal_authoring.dart';
import 'project/revision3_voice_take_media_qa_service.dart';
import 'project/revision3_voice_take_preview_authoring.dart';
import 'project/revision3_voice_take_preview_playback.dart';
import 'project/revision3_voice_take_selection_authoring.dart';
import 'project/revision3_voice_take_selection_dialog.dart';
import 'project/revision3_voice_take_status_authoring.dart';
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

typedef ManagedRevision3RecoveryAction =
    Future<ManagedRevision3RecoveryCheckpoint> Function();

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

final class _ManagedStorySelectionOrigin {
  const _ManagedStorySelectionOrigin({
    required this.projectRoot,
    required this.projectId,
    required this.controller,
  });

  final String projectRoot;
  final String projectId;
  final Revision3StoryWorkspaceController controller;
}

enum _StoryDraftHandoffOutcome { opened, stale, selectionFailed }

enum _ProjectQuickCreateAction {
  npcOpening,
  questOpening,
  npcDraft,
  dialogLine,
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

/// A completed folder publication may notify only the exact project that
/// opened its dialog and only after that exact receipt became current.
bool revision3VoiceFolderPublicationMatchesCurrent(
  CurrentProjectState current, {
  required String originRoot,
  required String originProjectId,
  required Revision3VoiceFolderImportPublication publication,
}) =>
    current is ManagedRevision3CurrentProjectState &&
    !current.requiresReopen &&
    current.root.path == originRoot &&
    current.projectId == originProjectId &&
    current.projectId == publication.projectId &&
    current.projectRevision == publication.projectRevision &&
    current.head.canonicalJson == publication.projectHead;

class HomePage extends ConsumerStatefulWidget {
  const HomePage({super.key});

  @override
  ConsumerState<HomePage> createState() => _HomePageState();
}

class _HomePageState extends ConsumerState<HomePage>
    with WidgetsBindingObserver {
  bool _projectActionBusy = false;
  bool _managedWorkspaceDirty = false;
  bool _managedDirtyRebuildScheduled = false;
  late final ValueChanged<bool> _managedWorkspaceDirtyListener;

  void _onManagedWorkspaceDirtyChanged(bool dirty) {
    if (_managedWorkspaceDirty == dirty) return;
    _managedWorkspaceDirty = dirty;
    if (!mounted || _managedDirtyRebuildScheduled) return;
    _managedDirtyRebuildScheduled = true;
    WidgetsBinding.instance.addPostFrameCallback((_) {
      _managedDirtyRebuildScheduled = false;
      if (mounted) setState(() {});
    });
  }

  @override
  void initState() {
    super.initState();
    _managedWorkspaceDirtyListener = _onManagedWorkspaceDirtyChanged;
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
          await coordinator.saveCurrent();
          _snack(l10n.projectManagedRevision3Verified);
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

  Future<ManagedRevision3RecoveryCheckpoint> _recoverManagedRevision3Project(
    ManagedRevision3CurrentProjectState expected,
  ) async {
    if (_projectActionBusy) {
      throw const CurrentProjectCoordinatorException(
        'another project action is already in progress',
      );
    }
    final current = ref.read(currentProjectCoordinatorProvider);
    if (current is! ManagedRevision3CurrentProjectState ||
        !current.requiresReopen ||
        current.root.path != expected.root.path ||
        current.projectId != expected.projectId ||
        current.projectRevision != expected.projectRevision ||
        current.head.canonicalJson != expected.head.canonicalJson) {
      throw const CurrentProjectCoordinatorException(
        'the project changed before recovery started',
      );
    }
    setState(() => _projectActionBusy = true);
    try {
      return await ref
          .read(currentProjectCoordinatorProvider.notifier)
          .recoverCurrentRevision3(
            expectedRoot: expected.root.path,
            expectedProjectId: expected.projectId,
            expectedProjectRevision: expected.projectRevision,
            expectedHead: expected.head,
          );
    } finally {
      if (mounted) setState(() => _projectActionBusy = false);
    }
  }

  Future<Revision3ProjectHistoryRestorePublication>
  _restoreManagedRevision3ProjectHistory(
    ManagedRevision3CurrentProjectState expected,
    Revision3ProjectHistorySnapshot expectedHistory,
    Revision3ProjectHistoryEntry target,
  ) async {
    if (_projectActionBusy || _managedWorkspaceDirty) {
      throw const Revision3ProjectHistoryStaleCheckpointException();
    }
    final current = ref.read(currentProjectCoordinatorProvider);
    if (current is! ManagedRevision3CurrentProjectState ||
        current.requiresReopen ||
        current.root.path != expected.root.path ||
        current.projectId != expected.projectId ||
        current.projectRevision != expected.projectRevision ||
        current.head.canonicalJson != expected.head.canonicalJson) {
      throw const Revision3ProjectHistoryStaleCheckpointException();
    }
    setState(() => _projectActionBusy = true);
    try {
      return await ref
          .read(currentProjectCoordinatorProvider.notifier)
          .restoreCurrentRevision3ProjectHistory(
            expectedRoot: expected.root.path,
            expectedProjectId: expected.projectId,
            expectedProjectRevision: expected.projectRevision,
            expectedHead: expected.head,
            expectedHistory: expectedHistory,
            target: target,
          );
    } finally {
      if (mounted) setState(() => _projectActionBusy = false);
    }
  }

  Future<void> _saveProjectAs() => _runProjectAction(() async {
    try {
      final coordinator = ref.read(currentProjectCoordinatorProvider.notifier);
      await _saveLegacyProjectAs(coordinator);
    } catch (e) {
      _snack('Save failed: $e');
    }
  });

  Future<void> _closeProject() => _runProjectAction(() async {
    final l10n = AppLocalizations.of(context);
    if (!await _confirmDiscardIfDirty()) return;
    try {
      await ref.read(currentProjectCoordinatorProvider.notifier).closeCurrent();
    } catch (e) {
      _snack(l10n.projectCloseFailed('$e'));
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
    return switch (ref.read(currentProjectCoordinatorProvider)) {
      LegacyCurrentProjectState() => hasUnsavedChanges(ref),
      ManagedRevision3CurrentProjectState() => _managedWorkspaceDirty,
      NoCurrentProjectState() => false,
    };
  }

  /// Confirm before discarding staged (unsaved) edits. Returns true to proceed.
  Future<bool> _confirmDiscardIfDirty() async {
    if (!_hasUnsavedEdits()) return true;
    final managed =
        ref.read(currentProjectCoordinatorProvider)
            is ManagedRevision3CurrentProjectState;
    final l10n = AppLocalizations.of(context);
    final ok = await showDialog<bool>(
      context: context,
      builder: (ctx) => AlertDialog(
        title: Text(
          managed
              ? l10n.managedLocalizationUnsavedTitle
              : 'Discard unsaved changes?',
        ),
        content: Text(
          managed
              ? l10n.managedLocalizationUnsavedDescription
              : 'You have staged edits that are not saved to a project. Continue and discard them?',
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.pop(ctx, false),
            child: Text(
              managed ? l10n.managedLocalizationKeepEditing : 'Cancel',
            ),
          ),
          FilledButton(
            onPressed: () => Navigator.pop(ctx, true),
            child: Text(managed ? l10n.managedLocalizationDiscard : 'Discard'),
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
      await coordinator.openManagedRevision3(Directory(path));
      if (!_showTransitionCleanupWarningIfAdded(
        coordinator,
        cleanupFailuresBefore,
      )) {
        _snack(l10n.projectManagedRevision3Opened);
      }
    } catch (e) {
      _snack(l10n.projectManagedRevision3OpenFailed('$e'));
    }
  });

  Future<void> _exportManagedRevision3Project(
    ManagedRevision3CurrentProjectState expected,
  ) => _runProjectAction(() async {
    final l10n = AppLocalizations.of(context);
    if (_managedWorkspaceDirty) {
      _snack(l10n.projectExportActionDirtyBlocked);
      return;
    }
    final current = ref.read(currentProjectCoordinatorProvider);
    if (current is! ManagedRevision3CurrentProjectState ||
        current.root.path != expected.root.path ||
        current.projectId != expected.projectId ||
        current.projectRevision != expected.projectRevision ||
        current.head.canonicalJson != expected.head.canonicalJson) {
      _snack(l10n.projectExportStale);
      return;
    }
    if (current.requiresReopen) {
      _snack(l10n.projectExportRequiresReopen);
      return;
    }
    await showDialog<AuthoringRevision3ExactSnapshotExportResult>(
      context: context,
      barrierDismissible: false,
      builder: (_) => Revision3ProjectExportDialog(
        projectRevision: current.projectRevision,
        pickExistingParentDirectory: () => ref.read(
          managedRevision3DirectoryPickerProvider,
        )(l10n.projectExportChooseDestination),
        export: (output) => ref
            .read(currentProjectCoordinatorProvider.notifier)
            .exportCurrentRevision3ExactSnapshot(
              expectedRoot: current.root.path,
              expectedProjectId: current.projectId,
              expectedProjectRevision: current.projectRevision,
              expectedHead: current.head,
              output: output,
            ),
      ),
    );
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
                case 'exportManagedRevision3':
                  if (currentProject
                      case final ManagedRevision3CurrentProjectState project) {
                    await _exportManagedRevision3Project(project);
                  }
                case 'saveAs':
                  await _saveProjectAs();
                case 'close':
                  await _closeProject();
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
              if (managedCurrent)
                PopupMenuItem(
                  key: const Key('project-export-managed-revision3'),
                  value: 'exportManagedRevision3',
                  enabled:
                      !_projectActionBusy &&
                      !managedVerificationBlocked &&
                      !_managedWorkspaceDirty,
                  child: Text(l10n.projectExportActionTitle),
                ),
              PopupMenuItem(
                key: const Key('project-close'),
                value: 'close',
                enabled:
                    !_projectActionBusy &&
                    currentProject is! NoCurrentProjectState,
                child: Text(l10n.projectClose),
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
          readCurrentManagedProject: () {
            final latest = ref.read(currentProjectCoordinatorProvider);
            return latest is ManagedRevision3CurrentProjectState
                ? latest
                : null;
          },
          gameRoot: gameRoot,
          recoveryBusy: _projectActionBusy,
          managedWorkspaceDirty: _managedWorkspaceDirty,
          recoverProject: () => _recoverManagedRevision3Project(currentProject),
          onDialogLocalizationDirtyChanged: _managedWorkspaceDirtyListener,
          verifyCurrentHead: _saveProject,
          loadContentIndex: () => ref
              .read(currentProjectCoordinatorProvider.notifier)
              .readCurrentRevision3ContentIndex(),
          loadProjectHistory: () => ref
              .read(currentProjectCoordinatorProvider.notifier)
              .readCurrentRevision3ProjectHistory(
                expectedRoot: currentProject.root.path,
                expectedProjectId: currentProject.projectId,
                expectedProjectRevision: currentProject.projectRevision,
                expectedHead: currentProject.head,
              ),
          restoreProjectHistory: (expectedHistory, target) =>
              _restoreManagedRevision3ProjectHistory(
                currentProject,
                expectedHistory,
                target,
              ),
          canRestoreProjectHistory:
              !_projectActionBusy &&
              !currentProject.requiresReopen &&
              !_managedWorkspaceDirty,
          historyRestoreDisabledReason: _managedWorkspaceDirty
              ? l10n.managedProjectHistoryDirtyBlocked
              : _projectActionBusy
              ? l10n.managedProjectHistoryBusy
              : null,
          removeStoryDraft: _projectActionBusy || currentProject.requiresReopen
              ? null
              : ({
                  required index,
                  required draft,
                  required scriptModule,
                }) async {
                  final liveProject = ref.read(
                    currentProjectCoordinatorProvider,
                  );
                  if (_projectActionBusy ||
                      _managedWorkspaceDirty ||
                      liveProject is! ManagedRevision3CurrentProjectState ||
                      liveProject.requiresReopen ||
                      liveProject.root.path != currentProject.root.path ||
                      liveProject.projectId != currentProject.projectId ||
                      liveProject.projectRevision !=
                          currentProject.projectRevision ||
                      liveProject.head.canonicalJson !=
                          currentProject.head.canonicalJson) {
                    throw const Revision3StoryDraftRemovalStaleCheckpointException();
                  }
                  final draftKind = switch (draft.kind) {
                    Revision3ContentEntityKind.npcDraft =>
                      AuthoringStoryDraftKind.npcDraft,
                    Revision3ContentEntityKind.questDraft =>
                      AuthoringStoryDraftKind.questDraft,
                    _ => throw const FormatException(
                      'Story Draft removal received a non-Story entity.',
                    ),
                  };
                  if (index.projectId != currentProject.projectId ||
                      index.projectRevision != currentProject.projectRevision ||
                      scriptModule.kind !=
                          Revision3ContentEntityKind.scriptModule) {
                    throw const Revision3StoryDraftRemovalStaleCheckpointException();
                  }
                  final publication = await ref
                      .read(currentProjectCoordinatorProvider.notifier)
                      .removeCurrentRevision3StoryDraft(
                        expectedRoot: currentProject.root.path,
                        expectedProjectId: currentProject.projectId,
                        expectedProjectRevision: currentProject.projectRevision,
                        expectedHead: currentProject.head,
                        draftId: draft.id,
                        draftKind: draftKind,
                        expectedDraftRevision: draft.revision,
                        scriptModuleId: scriptModule.id,
                        expectedScriptModuleRevision: scriptModule.revision,
                      );
                  if (publication.projectId != currentProject.projectId ||
                      publication.projectRevision !=
                          currentProject.projectRevision + 1 ||
                      publication.head.canonicalJson ==
                          currentProject.head.canonicalJson ||
                      publication.removedDraftId != draft.id ||
                      publication.removedDraftKind != draftKind ||
                      publication.removedDraftRevision != draft.revision ||
                      publication.removedScriptModuleId != scriptModule.id ||
                      publication.removedScriptModuleRevision !=
                          scriptModule.revision) {
                    throw const FormatException(
                      'Story Draft removal publication disagrees with the confirmed pair.',
                    );
                  }
                },
          removeStoryDraftDisabledReason: currentProject.requiresReopen
              ? l10n.managedStoryWorkspaceRemoveRequiresReopen
              : _projectActionBusy
              ? l10n.managedStoryWorkspaceRemoveBusy
              : null,
          loadBaseGameCatalog: ref.read(
            revision3BaseGameContentCatalogLoaderProvider,
          ),
          readDialogLocalization:
              ({
                required expectedProjectId,
                required expectedProjectRevision,
                required localizationId,
                required expectedLocalizationRevision,
                required expectedLocId,
              }) async {
                try {
                  return await ref
                      .read(currentProjectCoordinatorProvider.notifier)
                      .readCurrentRevision3DialogLocalization(
                        expectedRoot: currentProject.root.path,
                        expectedProjectId: expectedProjectId,
                        expectedProjectRevision: expectedProjectRevision,
                        expectedHead: currentProject.head,
                        localizationId: localizationId,
                        expectedLocalizationRevision:
                            expectedLocalizationRevision,
                        expectedLocId: expectedLocId,
                      );
                } on Revision3DialogLocalizationReadRequiresReopenException {
                  throw const Revision3DialogLineEntryRequiresReopenException();
                } on Revision3DialogLocalizationReadStaleCheckpointException {
                  throw const Revision3DialogLineEntryStaleCheckpointException();
                }
              },
          loadDialogLocalizationEditSeed:
              ({
                required expectedProjectId,
                required expectedProjectRevision,
                required localizationId,
                required expectedLocalizationRevision,
                required expectedLocId,
              }) => ref
                  .read(currentProjectCoordinatorProvider.notifier)
                  .readCurrentRevision3DialogLocalizationEditSeed(
                    expectedRoot: currentProject.root.path,
                    expectedProjectId: expectedProjectId,
                    expectedProjectRevision: expectedProjectRevision,
                    expectedHead: currentProject.head,
                    localizationId: localizationId,
                    expectedLocalizationRevision: expectedLocalizationRevision,
                    expectedLocId: expectedLocId,
                  ),
          publishDialogLocalizationEdit:
              ({
                required expectedProjectId,
                required expectedProjectRevision,
                required plan,
              }) => ref
                  .read(currentProjectCoordinatorProvider.notifier)
                  .prepareAndPublishCurrentRevision3DialogLocalizationEdit(
                    expectedRoot: currentProject.root.path,
                    expectedProjectId: expectedProjectId,
                    expectedProjectRevision: expectedProjectRevision,
                    expectedHead: currentProject.head,
                    plan: plan,
                  ),
          publishDialogLine:
              ({
                required expectedProjectId,
                required expectedProjectRevision,
                required plan,
              }) => ref
                  .read(currentProjectCoordinatorProvider.notifier)
                  .createCurrentRevision3DialogLine(
                    expectedRoot: currentProject.root.path,
                    expectedProjectId: expectedProjectId,
                    expectedProjectRevision: expectedProjectRevision,
                    expectedHead: currentProject.head,
                    plan: plan,
                  ),
          publishQuestTranscriptReplace:
              ({
                required expectedProjectId,
                required expectedProjectRevision,
                required expectedHead,
                required plan,
              }) => ref
                  .read(currentProjectCoordinatorProvider.notifier)
                  .replaceCurrentRevision3QuestTranscript(
                    expectedRoot: currentProject.root.path,
                    expectedProjectId: expectedProjectId,
                    expectedProjectRevision: expectedProjectRevision,
                    expectedHead: expectedHead,
                    plan: plan,
                  ),
          publishQuestTranscriptCreate:
              ({
                required expectedProjectId,
                required expectedProjectRevision,
                required expectedHead,
                required plan,
              }) => ref
                  .read(currentProjectCoordinatorProvider.notifier)
                  .createCurrentRevision3QuestTranscriptLine(
                    expectedRoot: currentProject.root.path,
                    expectedProjectId: expectedProjectId,
                    expectedProjectRevision: expectedProjectRevision,
                    expectedHead: expectedHead,
                    plan: plan,
                  ),
          publishNpcGreetingReplace:
              ({
                required expectedProjectId,
                required expectedProjectRevision,
                required expectedHead,
                required plan,
              }) => ref
                  .read(currentProjectCoordinatorProvider.notifier)
                  .replaceCurrentRevision3NpcGreeting(
                    expectedRoot: currentProject.root.path,
                    expectedProjectId: expectedProjectId,
                    expectedProjectRevision: expectedProjectRevision,
                    expectedHead: expectedHead,
                    plan: plan,
                  ),
          publishNpcGreetingCreate:
              ({
                required expectedProjectId,
                required expectedProjectRevision,
                required expectedHead,
                required plan,
              }) => ref
                  .read(currentProjectCoordinatorProvider.notifier)
                  .createCurrentRevision3NpcGreetingLine(
                    expectedRoot: currentProject.root.path,
                    expectedProjectId: expectedProjectId,
                    expectedProjectRevision: expectedProjectRevision,
                    expectedHead: expectedHead,
                    plan: plan,
                  ),
          isQuestTranscriptCheckpointCurrent:
              ({required projectId, required projectRevision}) {
                final latest = ref.read(currentProjectCoordinatorProvider);
                return latest is ManagedRevision3CurrentProjectState &&
                    latest.root.path == currentProject.root.path &&
                    latest.projectId == currentProject.projectId &&
                    latest.projectId == projectId &&
                    latest.projectRevision == projectRevision &&
                    !latest.requiresReopen;
              },
          isNpcGreetingCheckpointCurrent:
              ({required projectId, required projectRevision}) {
                final latest = ref.read(currentProjectCoordinatorProvider);
                return latest is ManagedRevision3CurrentProjectState &&
                    latest.root.path == currentProject.root.path &&
                    latest.projectId == currentProject.projectId &&
                    latest.projectId == projectId &&
                    latest.projectRevision == projectRevision &&
                    !latest.requiresReopen;
              },
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
          planVoiceFolder: ({required sourceFolder, required locale}) {
            final configuredGameRoot = gameRoot;
            final latest = ref.read(currentProjectCoordinatorProvider);
            if (configuredGameRoot == null ||
                latest is! ManagedRevision3CurrentProjectState ||
                latest.root.path != currentProject.root.path ||
                latest.projectId != currentProject.projectId ||
                latest.projectRevision != currentProject.projectRevision ||
                latest.head.canonicalJson !=
                    currentProject.head.canonicalJson) {
              throw const Revision3VoiceBatchStaleCheckpointException();
            }
            return ref
                .read(currentProjectCoordinatorProvider.notifier)
                .planCurrentRevision3VoiceBatchV1(
                  expectedRoot: currentProject.root.path,
                  expectedProjectId: currentProject.projectId,
                  expectedProjectRevision: currentProject.projectRevision,
                  expectedHead: currentProject.head,
                  gameRoot: configuredGameRoot,
                  sourceFolder: sourceFolder,
                  locale: locale,
                );
          },
          publishVoiceFolder: ({required sourceFolder, required plan}) {
            final configuredGameRoot = gameRoot;
            final latest = ref.read(currentProjectCoordinatorProvider);
            if (configuredGameRoot == null ||
                latest is! ManagedRevision3CurrentProjectState ||
                latest.root.path != currentProject.root.path ||
                latest.projectId != currentProject.projectId ||
                latest.projectRevision != currentProject.projectRevision ||
                latest.head.canonicalJson !=
                    currentProject.head.canonicalJson) {
              throw const Revision3VoiceBatchStaleCheckpointException();
            }
            return ref
                .read(currentProjectCoordinatorProvider.notifier)
                .importCurrentRevision3VoiceBatchV1(
                  expectedRoot: currentProject.root.path,
                  expectedProjectId: currentProject.projectId,
                  expectedProjectRevision: currentProject.projectRevision,
                  expectedHead: currentProject.head,
                  gameRoot: configuredGameRoot,
                  sourceFolder: sourceFolder,
                  plan: plan,
                );
          },
          pickVoiceFolder: () => ref.read(
            managedRevision3DirectoryPickerProvider,
          )(l10n.managedVoiceFolderImportChooseFolder),
          isVoiceFolderPublicationCurrent: (publication) {
            final latest = ref.read(currentProjectCoordinatorProvider);
            return revision3VoiceFolderPublicationMatchesCurrent(
              latest,
              originRoot: currentProject.root.path,
              originProjectId: currentProject.projectId,
              publication: publication,
            );
          },
          materializeVoiceTakePreview:
              ({
                required expectedProjectId,
                required expectedProjectRevision,
                required plan,
              }) {
                final latest = ref.read(currentProjectCoordinatorProvider);
                if (latest is! ManagedRevision3CurrentProjectState ||
                    latest.root.path != currentProject.root.path ||
                    latest.projectId != currentProject.projectId) {
                  throw const Revision3VoiceTakePreviewStaleCheckpointException();
                }
                return ref
                    .read(currentProjectCoordinatorProvider.notifier)
                    .materializeCurrentRevision3VoiceTakePreview(
                      expectedRoot: currentProject.root.path,
                      expectedProjectId: expectedProjectId,
                      expectedProjectRevision: expectedProjectRevision,
                      expectedHead: latest.head,
                      plan: plan,
                    );
              },
          inspectVoiceTakeMediaQa:
              ({
                required expectedProjectId,
                required expectedProjectRevision,
                required plan,
              }) {
                final latest = ref.read(currentProjectCoordinatorProvider);
                if (latest is! ManagedRevision3CurrentProjectState ||
                    latest.root.path != currentProject.root.path ||
                    latest.projectId != currentProject.projectId) {
                  throw const Revision3VoiceTakeMediaQaStaleCheckpointException();
                }
                return ref
                    .read(currentProjectCoordinatorProvider.notifier)
                    .inspectCurrentRevision3VoiceTakeMediaQa(
                      expectedRoot: currentProject.root.path,
                      expectedProjectId: expectedProjectId,
                      expectedProjectRevision: expectedProjectRevision,
                      expectedHead: latest.head,
                      plan: plan,
                    );
              },
          publishVoiceTakeSelection:
              ({
                required expectedProjectId,
                required expectedProjectRevision,
                required plan,
              }) {
                final latest = ref.read(currentProjectCoordinatorProvider);
                if (latest is! ManagedRevision3CurrentProjectState ||
                    latest.root.path != currentProject.root.path ||
                    latest.projectId != currentProject.projectId) {
                  throw const Revision3VoiceTakeSelectionStaleCheckpointException();
                }
                return ref
                    .read(currentProjectCoordinatorProvider.notifier)
                    .selectCurrentRevision3VoiceTake(
                      expectedRoot: currentProject.root.path,
                      expectedProjectId: expectedProjectId,
                      expectedProjectRevision: expectedProjectRevision,
                      expectedHead: latest.head,
                      plan: plan,
                    );
              },
          publishVoiceTakeStatus:
              ({
                required expectedProjectId,
                required expectedProjectRevision,
                required plan,
              }) {
                final latest = ref.read(currentProjectCoordinatorProvider);
                if (latest is! ManagedRevision3CurrentProjectState ||
                    latest.root.path != currentProject.root.path ||
                    latest.projectId != currentProject.projectId) {
                  throw const Revision3VoiceTakeStatusStaleCheckpointException();
                }
                return ref
                    .read(currentProjectCoordinatorProvider.notifier)
                    .editCurrentRevision3VoiceTakeStatus(
                      expectedRoot: currentProject.root.path,
                      expectedProjectId: expectedProjectId,
                      expectedProjectRevision: expectedProjectRevision,
                      expectedHead: latest.head,
                      plan: plan,
                    );
              },
          publishVoiceTakeRemoval:
              ({
                required expectedProjectId,
                required expectedProjectRevision,
                required plan,
              }) {
                final latest = ref.read(currentProjectCoordinatorProvider);
                if (latest is! ManagedRevision3CurrentProjectState ||
                    latest.root.path != currentProject.root.path ||
                    latest.projectId != currentProject.projectId) {
                  throw const Revision3VoiceTakeRemovalStaleCheckpointException();
                }
                return ref
                    .read(currentProjectCoordinatorProvider.notifier)
                    .removeCurrentRevision3VoiceTake(
                      expectedRoot: currentProject.root.path,
                      expectedProjectId: expectedProjectId,
                      expectedProjectRevision: expectedProjectRevision,
                      expectedHead: latest.head,
                      plan: plan,
                    );
              },
          publishDialogVoiceSlotRemoval:
              ({
                required expectedProjectId,
                required expectedProjectRevision,
                required plan,
              }) {
                final latest = ref.read(currentProjectCoordinatorProvider);
                if (latest is! ManagedRevision3CurrentProjectState ||
                    latest.root.path != currentProject.root.path ||
                    latest.projectId != currentProject.projectId) {
                  throw const Revision3DialogVoiceSlotRemovalStaleCheckpointException();
                }
                return ref
                    .read(currentProjectCoordinatorProvider.notifier)
                    .removeCurrentRevision3DialogVoiceSlot(
                      expectedRoot: currentProject.root.path,
                      expectedProjectId: expectedProjectId,
                      expectedProjectRevision: expectedProjectRevision,
                      expectedHead: latest.head,
                      plan: plan,
                    );
              },
          publishDialogVoiceSlotCreation:
              ({
                required expectedProjectId,
                required expectedProjectRevision,
                required plan,
              }) {
                final latest = ref.read(currentProjectCoordinatorProvider);
                if (latest is! ManagedRevision3CurrentProjectState ||
                    latest.root.path != currentProject.root.path ||
                    latest.projectId != currentProject.projectId) {
                  throw const Revision3DialogVoiceSlotCreationStaleCheckpointException();
                }
                return ref
                    .read(currentProjectCoordinatorProvider.notifier)
                    .createCurrentRevision3DialogVoiceSlot(
                      expectedRoot: currentProject.root.path,
                      expectedProjectId: expectedProjectId,
                      expectedProjectRevision: expectedProjectRevision,
                      expectedHead: latest.head,
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
          planVoiceBuild: () => ref
              .read(currentProjectCoordinatorProvider.notifier)
              .planCurrentRevision3Voice(
                expectedRoot: currentProject.root.path,
                expectedProjectId: currentProject.projectId,
                expectedProjectRevision: currentProject.projectRevision,
                expectedHead: currentProject.head,
              ),
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
          )(l10n.managedVoiceBuildParentPickerTitle),
          buildReviewedDataAsset:
              ({required targetPath, required packName, required output}) {
                final configuredGameRoot = gameRoot;
                if (configuredGameRoot == null) {
                  throw StateError(
                    'Configure the Gothic 1 Remake installation before building DataAsset files.',
                  );
                }
                return ref
                    .read(currentProjectCoordinatorProvider.notifier)
                    .buildCurrentRevision3ReviewedDataAsset(
                      expectedRoot: currentProject.root.path,
                      expectedProjectId: currentProject.projectId,
                      expectedProjectRevision: currentProject.projectRevision,
                      expectedHead: currentProject.head,
                      gameRoot: configuredGameRoot,
                      targetPath: targetPath,
                      packName: packName,
                      output: output,
                    );
              },
          pickDataAssetBuildParent: () => ref.read(
            managedRevision3DirectoryPickerProvider,
          )('Choose DataAsset build parent'),
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
          loadNpcProfileEditSeed:
              ({
                required npcId,
                required expectedNpcRevision,
                required expectedScriptModuleId,
                required expectedScriptModuleRevision,
                required expectedUniqueName,
                required expectedModuleNamespace,
                required expectedParentCharacterDefinition,
                required expectedParentAiAgentConfig,
                required expectedParentSpawnDefinition,
              }) => ref
                  .read(currentProjectCoordinatorProvider.notifier)
                  .readCurrentRevision3NpcProfileEditSeed(
                    expectedRoot: currentProject.root.path,
                    expectedProjectId: currentProject.projectId,
                    expectedProjectRevision: currentProject.projectRevision,
                    expectedHead: currentProject.head,
                    npcId: npcId,
                    expectedNpcRevision: expectedNpcRevision,
                    expectedScriptModuleId: expectedScriptModuleId,
                    expectedScriptModuleRevision: expectedScriptModuleRevision,
                    expectedUniqueName: expectedUniqueName,
                    expectedModuleNamespace: expectedModuleNamespace,
                    expectedParentCharacterDefinition:
                        expectedParentCharacterDefinition,
                    expectedParentAiAgentConfig: expectedParentAiAgentConfig,
                    expectedParentSpawnDefinition:
                        expectedParentSpawnDefinition,
                  ),
          publishNpcProfileEdit: ({required gameRoot, required plan}) => ref
              .read(currentProjectCoordinatorProvider.notifier)
              .editCurrentRevision3NpcProfile(
                expectedRoot: currentProject.root.path,
                expectedProjectId: currentProject.projectId,
                expectedProjectRevision: currentProject.projectRevision,
                expectedHead: currentProject.head,
                gameRoot: gameRoot,
                plan: plan,
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

class _ManagedRevision3ProjectView extends StatefulWidget {
  const _ManagedRevision3ProjectView({
    required this.project,
    required this.readCurrentManagedProject,
    required this.gameRoot,
    required this.recoveryBusy,
    required this.managedWorkspaceDirty,
    required this.recoverProject,
    required this.onDialogLocalizationDirtyChanged,
    required this.verifyCurrentHead,
    required this.loadContentIndex,
    required this.loadProjectHistory,
    required this.restoreProjectHistory,
    required this.canRestoreProjectHistory,
    required this.historyRestoreDisabledReason,
    required this.removeStoryDraft,
    required this.removeStoryDraftDisabledReason,
    required this.loadBaseGameCatalog,
    required this.readDialogLocalization,
    required this.loadDialogLocalizationEditSeed,
    required this.publishDialogLocalizationEdit,
    required this.publishDialogLine,
    required this.publishQuestTranscriptReplace,
    required this.publishQuestTranscriptCreate,
    required this.isQuestTranscriptCheckpointCurrent,
    required this.publishNpcGreetingReplace,
    required this.publishNpcGreetingCreate,
    required this.isNpcGreetingCheckpointCurrent,
    required this.publishVoiceTake,
    required this.planVoiceFolder,
    required this.publishVoiceFolder,
    required this.pickVoiceFolder,
    required this.isVoiceFolderPublicationCurrent,
    required this.materializeVoiceTakePreview,
    required this.inspectVoiceTakeMediaQa,
    required this.publishVoiceTakeSelection,
    required this.publishVoiceTakeStatus,
    required this.publishVoiceTakeRemoval,
    required this.publishDialogVoiceSlotRemoval,
    required this.publishDialogVoiceSlotCreation,
    required this.publishVoiceTarget,
    required this.planVoiceBuild,
    required this.buildVoiceBundle,
    required this.pickVoiceBuildParent,
    required this.buildReviewedDataAsset,
    required this.pickDataAssetBuildParent,
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
    required this.loadNpcProfileEditSeed,
    required this.publishNpcProfileEdit,
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
  final ManagedRevision3CurrentProjectState? Function()
  readCurrentManagedProject;
  final String? gameRoot;
  final bool recoveryBusy;
  final bool managedWorkspaceDirty;
  final ManagedRevision3RecoveryAction recoverProject;
  final ValueChanged<bool> onDialogLocalizationDirtyChanged;
  final Future<void> Function() verifyCurrentHead;
  final Revision3ContentIndexLoader loadContentIndex;
  final Revision3ProjectHistoryLoader loadProjectHistory;
  final Revision3ProjectHistoryRestorer restoreProjectHistory;
  final bool canRestoreProjectHistory;
  final String? historyRestoreDisabledReason;
  final Revision3StoryWorkspaceRemoveDraftAction? removeStoryDraft;
  final String? removeStoryDraftDisabledReason;
  final Revision3BaseGameContentCatalogLoader loadBaseGameCatalog;
  final Revision3DialogLineEntryLocalizationReader readDialogLocalization;
  final Revision3DialogLocalizationEditSeedLoader
  loadDialogLocalizationEditSeed;
  final Revision3DialogLocalizationEditTechnicalPublisher
  publishDialogLocalizationEdit;
  final Revision3DialogLineEntryTechnicalPublisher publishDialogLine;
  final Revision3QuestTranscriptReplacePublisher publishQuestTranscriptReplace;
  final Revision3QuestTranscriptCreatePublisher publishQuestTranscriptCreate;
  final bool Function({required String projectId, required int projectRevision})
  isQuestTranscriptCheckpointCurrent;
  final Revision3NpcGreetingReplacePublisher publishNpcGreetingReplace;
  final Revision3NpcGreetingCreatePublisher publishNpcGreetingCreate;
  final bool Function({required String projectId, required int projectRevision})
  isNpcGreetingCheckpointCurrent;
  final Revision3VoiceTechnicalPublisher publishVoiceTake;
  final Revision3VoiceFolderNativePlanner planVoiceFolder;
  final Revision3VoiceFolderNativePublisher publishVoiceFolder;
  final Revision3VoiceFolderDirectoryPicker pickVoiceFolder;
  final bool Function(Revision3VoiceFolderImportPublication publication)
  isVoiceFolderPublicationCurrent;
  final Revision3VoiceTakePreviewTechnicalMaterializer
  materializeVoiceTakePreview;
  final Revision3VoiceTakeMediaQaTechnicalInspector inspectVoiceTakeMediaQa;
  final Revision3VoiceTakeSelectionTechnicalPublisher publishVoiceTakeSelection;
  final Revision3VoiceTakeStatusTechnicalPublisher publishVoiceTakeStatus;
  final Revision3VoiceTakeRemovalTechnicalPublisher publishVoiceTakeRemoval;
  final Revision3DialogVoiceSlotRemovalTechnicalPublisher
  publishDialogVoiceSlotRemoval;
  final Revision3DialogVoiceSlotCreationTechnicalPublisher
  publishDialogVoiceSlotCreation;
  final Revision3VoiceTargetTechnicalPublisher publishVoiceTarget;
  final Revision3VoiceBuildPlanLoader planVoiceBuild;
  final Revision3VoiceExactBuild buildVoiceBundle;
  final Revision3VoiceBuildParentDirectoryPicker pickVoiceBuildParent;
  final Revision3ReviewedDataAssetStageBuilder buildReviewedDataAsset;
  final Revision3DataAssetBuildParentDirectoryPicker pickDataAssetBuildParent;
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
  final Revision3NpcProfileEditSeedLoader loadNpcProfileEditSeed;
  final Revision3NpcProfileEditTechnicalPublisher publishNpcProfileEdit;
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
  State<_ManagedRevision3ProjectView> createState() =>
      _ManagedRevision3ProjectViewState();
}

class _ManagedRevision3ProjectViewState
    extends State<_ManagedRevision3ProjectView> {
  late Revision3ContentLibraryController _contentLibraryController;
  late Revision3StoryWorkspaceController _storyWorkspaceController;
  late Revision3LocalizationVoiceWorkspaceController
  _localizationVoiceWorkspaceController;
  late Revision3DataAssetStagePanelController _dataAssetStagePanelController;
  late Revision3ScopedContentBrowserController _scopedContentBrowserController;
  late FocusNode _globalSearchQueryFocusNode;
  late final VoidCallback _activateGlobalSearchQuery;
  bool _recoveryStarting = false;
  bool _recoveryTerminal = false;
  String? _recoveryError;
  int _storyAuthorityEpoch = 0;
  final Revision3NpcOpeningRecipe _npcOpeningRecipe =
      Revision3NpcOpeningRecipe();
  final Revision3QuestOpeningRecipe _questOpeningRecipe =
      Revision3QuestOpeningRecipe();
  bool _npcOpeningRecipeUiBusy = false;
  bool _questOpeningRecipeUiBusy = false;
  String? _projectDisplayName;

  ManagedRevision3CurrentProjectState get project => widget.project;
  ManagedRevision3CurrentProjectState? get currentManagedProject =>
      widget.readCurrentManagedProject();
  String? get gameRoot => widget.gameRoot;
  Future<void> Function() get verifyCurrentHead => widget.verifyCurrentHead;
  Revision3ContentIndexLoader get loadContentIndex =>
      _loadContentIndexAndRememberProjectName;
  Revision3StoryWorkspaceRemoveDraftAction? get removeStoryDraft =>
      widget.removeStoryDraft;
  String? get removeStoryDraftDisabledReason =>
      widget.removeStoryDraftDisabledReason;
  Revision3BaseGameContentCatalogLoader get loadBaseGameCatalog =>
      widget.loadBaseGameCatalog;
  Revision3DialogLineEntryLocalizationReader get readDialogLocalization =>
      widget.readDialogLocalization;
  Revision3DialogLocalizationEditSeedLoader
  get loadDialogLocalizationEditSeed => widget.loadDialogLocalizationEditSeed;
  Revision3DialogLocalizationEditTechnicalPublisher
  get publishDialogLocalizationEdit => widget.publishDialogLocalizationEdit;
  Revision3DialogLineEntryTechnicalPublisher get publishDialogLine =>
      widget.publishDialogLine;
  Revision3QuestTranscriptReplacePublisher get publishQuestTranscriptReplace =>
      widget.publishQuestTranscriptReplace;
  Revision3QuestTranscriptCreatePublisher get publishQuestTranscriptCreate =>
      widget.publishQuestTranscriptCreate;
  Revision3NpcGreetingReplacePublisher get publishNpcGreetingReplace =>
      widget.publishNpcGreetingReplace;
  Revision3NpcGreetingCreatePublisher get publishNpcGreetingCreate =>
      widget.publishNpcGreetingCreate;
  Revision3VoiceTechnicalPublisher get publishVoiceTake =>
      widget.publishVoiceTake;
  Revision3VoiceFolderNativePlanner get planVoiceFolder =>
      widget.planVoiceFolder;
  Revision3VoiceFolderNativePublisher get publishVoiceFolder =>
      widget.publishVoiceFolder;
  Revision3VoiceFolderDirectoryPicker get pickVoiceFolder =>
      widget.pickVoiceFolder;
  Revision3VoiceTakePreviewTechnicalMaterializer
  get materializeVoiceTakePreview => widget.materializeVoiceTakePreview;
  Revision3VoiceTakeMediaQaTechnicalInspector get inspectVoiceTakeMediaQa =>
      widget.inspectVoiceTakeMediaQa;
  Revision3VoiceTakeSelectionTechnicalPublisher get publishVoiceTakeSelection =>
      widget.publishVoiceTakeSelection;
  Revision3VoiceTakeStatusTechnicalPublisher get publishVoiceTakeStatus =>
      widget.publishVoiceTakeStatus;
  Revision3VoiceTakeRemovalTechnicalPublisher get publishVoiceTakeRemoval =>
      widget.publishVoiceTakeRemoval;
  Revision3DialogVoiceSlotRemovalTechnicalPublisher
  get publishDialogVoiceSlotRemoval => widget.publishDialogVoiceSlotRemoval;
  Revision3DialogVoiceSlotCreationTechnicalPublisher
  get publishDialogVoiceSlotCreation => widget.publishDialogVoiceSlotCreation;
  Revision3VoiceTargetTechnicalPublisher get publishVoiceTarget =>
      widget.publishVoiceTarget;
  Revision3VoiceBuildPlanLoader get planVoiceBuild => widget.planVoiceBuild;
  Revision3VoiceExactBuild get buildVoiceBundle => widget.buildVoiceBundle;
  Revision3VoiceBuildParentDirectoryPicker get pickVoiceBuildParent =>
      widget.pickVoiceBuildParent;
  Revision3ReviewedDataAssetStageBuilder get buildReviewedDataAsset =>
      widget.buildReviewedDataAsset;
  Revision3DataAssetBuildParentDirectoryPicker get pickDataAssetBuildParent =>
      widget.pickDataAssetBuildParent;
  Revision3DataAssetStageLoader get loadDataAssetStages =>
      widget.loadDataAssetStages;
  Revision3InstalledPackageIndexLoader get loadInstalledPackageIndex =>
      widget.loadInstalledPackageIndex;
  Revision3InstalledDataAssetInspector get inspectInstalledDataAsset =>
      widget.inspectInstalledDataAsset;
  InstalledDataAssetSemanticStagePublisher
  get publishInstalledDataAssetSemanticEdit =>
      widget.publishInstalledDataAssetSemanticEdit;
  ReviewedInstalledDataAssetStagePublisher
  get publishReviewedInstalledDataAssetEdit =>
      widget.publishReviewedInstalledDataAssetEdit;
  Revision3DataAssetStagePublisher get publishDataAssetStage =>
      widget.publishDataAssetStage;
  DataAssetSemanticStagePublisher get publishDataAssetSemanticEdit =>
      widget.publishDataAssetSemanticEdit;
  Revision3DataAssetStageRemover get removeDataAssetStage =>
      widget.removeDataAssetStage;
  Revision3DataAssetPatchReceiptPicker? get pickDataAssetPatchReceipt =>
      widget.pickDataAssetPatchReceipt;
  DataAssetInspector? get inspectDataAssetSemanticEdit =>
      widget.inspectDataAssetSemanticEdit;
  DataAssetFilePicker? get pickDataAssetSemanticUasset =>
      widget.pickDataAssetSemanticUasset;
  DataAssetFilePicker? get pickDataAssetSemanticUsmap =>
      widget.pickDataAssetSemanticUsmap;
  DataAssetExtractReceiptPicker? get pickDataAssetExtractReceipt =>
      widget.pickDataAssetExtractReceipt;
  DataAssetExtractReceiptInspector get inspectDataAssetExtractReceipt =>
      widget.inspectDataAssetExtractReceipt;
  Revision3NpcCatalogLoader get loadNpcCatalog => widget.loadNpcCatalog;
  Revision3NpcArchetypeChooser? get chooseNpcArchetype =>
      widget.chooseNpcArchetype;
  Revision3NpcDraftPublisher get publishNpcDraft => widget.publishNpcDraft;
  Revision3NpcProfileEditSeedLoader get loadNpcProfileEditSeed =>
      widget.loadNpcProfileEditSeed;
  Revision3NpcProfileEditTechnicalPublisher get publishNpcProfileEdit =>
      widget.publishNpcProfileEdit;
  Revision3QuestCatalogLoader get loadQuestCatalog => widget.loadQuestCatalog;
  Revision3QuestDraftPublisher get publishQuestDraft =>
      widget.publishQuestDraft;
  Revision3QuestOutlineEditPublisher get editQuestOutline =>
      widget.editQuestOutline;
  Revision3QuestTransitionsSeedLoader get loadQuestTransitionsSeed =>
      widget.loadQuestTransitionsSeed;
  Revision3QuestTransitionsTechnicalPublisher get editQuestTransitions =>
      widget.editQuestTransitions;
  Revision3QuestContextSeedLoader get loadQuestContextSeed =>
      widget.loadQuestContextSeed;
  Revision3QuestContextTechnicalPublisher get editQuestContext =>
      widget.editQuestContext;
  Revision3QuestSourceInspectionLoader get inspectQuestSource =>
      widget.inspectQuestSource;
  Revision3NpcSourceInspectionLoader get inspectNpcSource =>
      widget.inspectNpcSource;
  Revision3ManagedCompilerPublisher get checkManagedCompiler =>
      widget.checkManagedCompiler;

  Future<Revision3ContentIndex>
  _loadContentIndexAndRememberProjectName() async {
    final expected = project;
    final index = await widget.loadContentIndex();
    final name = index.projectName.trim();
    if (name.isNotEmpty &&
        mounted &&
        index.projectId == expected.projectId &&
        index.projectRevision == expected.projectRevision &&
        _isCurrentQuestOpeningCheckpoint(expected) &&
        _projectDisplayName != name) {
      setState(() => _projectDisplayName = name);
    }
    return index;
  }

  Revision3ContentProjectIdentity get _contentProjectIdentity =>
      Revision3ContentProjectIdentity(
        projectRoot: project.root.path,
        projectId: project.projectId,
      );

  _ManagedStorySelectionOrigin get _storySelectionOrigin =>
      _ManagedStorySelectionOrigin(
        projectRoot: project.root.path,
        projectId: project.projectId,
        controller: _storyWorkspaceController,
      );

  bool _isCurrentStorySelectionOrigin(_ManagedStorySelectionOrigin origin) =>
      mounted &&
      project.root.path == origin.projectRoot &&
      project.projectId == origin.projectId &&
      identical(_storyWorkspaceController, origin.controller);

  @override
  void initState() {
    super.initState();
    _contentLibraryController = Revision3ContentLibraryController(
      projectIdentity: _contentProjectIdentity,
    );
    _storyWorkspaceController = Revision3StoryWorkspaceController();
    _localizationVoiceWorkspaceController =
        Revision3LocalizationVoiceWorkspaceController();
    _dataAssetStagePanelController = Revision3DataAssetStagePanelController();
    _scopedContentBrowserController = Revision3ScopedContentBrowserController(
      projectIdentity: (project.root.path, project.projectId),
    );
    _globalSearchQueryFocusNode = FocusNode(
      debugLabel: 'managed project global content search',
    );
    _activateGlobalSearchQuery = () {
      final expected = project;
      if (mounted && _isCurrentQuestOpeningCheckpoint(expected)) {
        _globalSearchQueryFocusNode.requestFocus();
      }
    };
  }

  @override
  void didUpdateWidget(covariant _ManagedRevision3ProjectView oldWidget) {
    super.didUpdateWidget(oldWidget);
    final identityChanged =
        oldWidget.project.root.path != project.root.path ||
        oldWidget.project.projectId != project.projectId;
    final checkpointChanged =
        oldWidget.project.projectRevision != project.projectRevision ||
        oldWidget.project.head.canonicalJson != project.head.canonicalJson;
    if (identityChanged) {
      _storyAuthorityEpoch = 0;
    } else if (oldWidget.project.requiresReopen && !project.requiresReopen) {
      _storyAuthorityEpoch++;
    }
    if (identityChanged || checkpointChanged) {
      _projectDisplayName = null;
    }
    if (identityChanged || checkpointChanged || !project.requiresReopen) {
      _recoveryStarting = false;
      _recoveryTerminal = false;
      _recoveryError = null;
    }
    if (!identityChanged) {
      return;
    }
    _contentLibraryController.dispose();
    _contentLibraryController = Revision3ContentLibraryController(
      projectIdentity: _contentProjectIdentity,
    );
    _storyWorkspaceController.dispose();
    _storyWorkspaceController = Revision3StoryWorkspaceController();
    _localizationVoiceWorkspaceController.dispose();
    _localizationVoiceWorkspaceController =
        Revision3LocalizationVoiceWorkspaceController();
    _dataAssetStagePanelController.dispose();
    _dataAssetStagePanelController = Revision3DataAssetStagePanelController();
    _scopedContentBrowserController.dispose();
    _scopedContentBrowserController = Revision3ScopedContentBrowserController(
      projectIdentity: (project.root.path, project.projectId),
    );
    _globalSearchQueryFocusNode.unfocus();
  }

  bool _stillShowsRecoveryFor(ManagedRevision3CurrentProjectState expected) =>
      mounted &&
      project.requiresReopen &&
      project.root.path == expected.root.path &&
      project.projectId == expected.projectId &&
      project.projectRevision == expected.projectRevision &&
      project.head.canonicalJson == expected.head.canonicalJson;

  bool _recoveryMatchesExpected(
    ManagedRevision3CurrentProjectState expected,
    ManagedRevision3RecoveryCheckpoint checkpoint,
  ) {
    final unchanged =
        checkpoint.recoveredProjectRevision == expected.projectRevision &&
        checkpoint.recoveredHead.canonicalJson == expected.head.canonicalJson;
    final advanced =
        checkpoint.recoveredProjectRevision == expected.projectRevision + 1 &&
        checkpoint.recoveredHead.canonicalJson != expected.head.canonicalJson;
    return checkpoint.projectId == expected.projectId &&
        checkpoint.previousProjectRevision == expected.projectRevision &&
        checkpoint.previousHead.canonicalJson == expected.head.canonicalJson &&
        (unchanged || advanced);
  }

  Future<void> _tryRecovery() async {
    if (_recoveryStarting ||
        widget.recoveryBusy ||
        _recoveryTerminal ||
        !project.requiresReopen) {
      return;
    }
    final expected = project;
    setState(() {
      _recoveryStarting = true;
      _recoveryError = null;
    });
    try {
      final checkpoint = await widget.recoverProject();
      if (!_recoveryMatchesExpected(expected, checkpoint)) {
        if (_stillShowsRecoveryFor(expected)) {
          setState(() {
            _recoveryTerminal = true;
            _recoveryError = AppLocalizations.of(
              context,
            ).managedProjectRecoveryUnavailable;
          });
        }
        return;
      }
      if (!mounted ||
          project.root.path != expected.root.path ||
          project.projectId != expected.projectId) {
        return;
      }
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(
          content: Text(
            AppLocalizations.of(context).managedProjectRecoverySucceeded,
          ),
        ),
      );
    } on Revision3RecoveryNotSupportedException {
      if (_stillShowsRecoveryFor(expected)) {
        setState(() {
          _recoveryTerminal = true;
          _recoveryError = AppLocalizations.of(
            context,
          ).managedProjectRecoveryUnavailable;
        });
      }
    } catch (_) {
      if (_stillShowsRecoveryFor(expected)) {
        setState(() {
          _recoveryTerminal = false;
          _recoveryError = AppLocalizations.of(
            context,
          ).managedProjectRecoveryFailed;
        });
      }
    } finally {
      if (_stillShowsRecoveryFor(expected)) {
        setState(() => _recoveryStarting = false);
      }
    }
  }

  @override
  void dispose() {
    _contentLibraryController.dispose();
    _storyWorkspaceController.dispose();
    _localizationVoiceWorkspaceController.dispose();
    _dataAssetStagePanelController.dispose();
    _scopedContentBrowserController.dispose();
    _globalSearchQueryFocusNode.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context);
    return Column(
      key: const Key('managed-revision3-project-view'),
      children: [
        if (project.requiresReopen)
          Card(
            margin: const EdgeInsets.fromLTRB(12, 8, 12, 6),
            child: Padding(
              padding: const EdgeInsets.fromLTRB(16, 8, 10, 2),
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
                            style: Theme.of(context).textTheme.titleMedium,
                          ),
                          if (project.requiresReopen) ...[
                            const SizedBox(height: 2),
                            Text(l10n.managedProjectRecoveryContentLocked),
                          ],
                        ],
                      );
                      final settings = IconButton.outlined(
                        key: const Key('managed-open-settings'),
                        onPressed: widget.recoveryBusy || _recoveryStarting
                            ? null
                            : () => unawaited(_openSettings(context)),
                        tooltip: l10n.managedActionSettingsTitle,
                        icon: const Icon(Icons.settings_outlined),
                      );
                      if (constraints.maxWidth < 500) {
                        return Column(
                          crossAxisAlignment: CrossAxisAlignment.stretch,
                          children: [
                            identity,
                            const SizedBox(height: 6),
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
                            color: Theme.of(
                              context,
                            ).colorScheme.onErrorContainer,
                          ),
                          const SizedBox(width: 10),
                          Expanded(
                            child: Column(
                              crossAxisAlignment: CrossAxisAlignment.start,
                              children: [
                                Text(
                                  l10n.managedProjectRecoveryDescription,
                                  style: TextStyle(
                                    color: Theme.of(
                                      context,
                                    ).colorScheme.onErrorContainer,
                                  ),
                                ),
                                const SizedBox(height: 6),
                                Text(
                                  l10n.managedProjectRecoveryAlternative,
                                  style: TextStyle(
                                    color: Theme.of(
                                      context,
                                    ).colorScheme.onErrorContainer,
                                  ),
                                ),
                                if (_recoveryError case final error?) ...[
                                  const SizedBox(height: 8),
                                  Text(
                                    error,
                                    key: const Key(
                                      'managed-project-recovery-error',
                                    ),
                                    style: TextStyle(
                                      color: Theme.of(
                                        context,
                                      ).colorScheme.onErrorContainer,
                                      fontWeight: FontWeight.w600,
                                    ),
                                  ),
                                ],
                                const SizedBox(height: 10),
                                FilledButton.icon(
                                  key: const Key(
                                    'managed-project-try-recovery',
                                  ),
                                  onPressed:
                                      _recoveryStarting ||
                                          widget.recoveryBusy ||
                                          _recoveryTerminal
                                      ? null
                                      : () => unawaited(_tryRecovery()),
                                  icon: _recoveryStarting
                                      ? const SizedBox.square(
                                          key: Key(
                                            'managed-project-recovery-progress',
                                          ),
                                          dimension: 16,
                                          child: CircularProgressIndicator(
                                            strokeWidth: 2,
                                          ),
                                        )
                                      : const Icon(Icons.restart_alt),
                                  label: Text(
                                    _recoveryStarting
                                        ? l10n.managedProjectRecoveryTrying
                                        : l10n.managedProjectRecoveryTry,
                                  ),
                                ),
                              ],
                            ),
                          ),
                        ],
                      ),
                    ),
                  ],
                  ExpansionTile(
                    key: const Key('managed-project-technical-details'),
                    dense: true,
                    visualDensity: VisualDensity.compact,
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
                                valueKey: const Key(
                                  'managed-project-head-bytes',
                                ),
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
                  chromeBuilder: (workspaceContext, location) =>
                      _buildProjectCommandBar(workspaceContext, location, l10n),
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
                      section: Revision3ProjectWorkspaceSection.history,
                      label: l10n.managedWorkspaceHistoryLabel,
                      icon: Icons.history_outlined,
                      selectedIcon: Icons.history,
                      pageBuilder: (_, _) => _buildHistorySection(l10n),
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
                        settings: _buildManagedSettingsArea(l10n),
                      ),
                    ),
                  ],
                ),
        ),
      ],
    );
  }

  Widget _buildProjectCommandBar(
    BuildContext context,
    Revision3ProjectWorkspaceLocation location,
    AppLocalizations l10n,
  ) {
    final mutationDisabledReason = _storyMutationDisabledReason(l10n);
    final busy = widget.recoveryBusy
        ? Revision3ProjectCommandBarBusyState(
            label: l10n.managedProjectHistoryBusy,
            disabledReason: l10n.managedProjectHistoryBusy,
          )
        : null;
    return Padding(
      padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 2),
      child: Revision3ProjectCommandBar(
        projectDisplayName: _projectCommandBarDisplayName(l10n),
        currentSectionLabel: _workspaceSectionLabel(l10n, location.section),
        searchCommand: Revision3ProjectCommand.enabled(
          () => _openProjectSearch(context),
        ),
        createCommand: mutationDisabledReason == null
            ? Revision3ProjectCommand.enabled(
                () => _openProjectQuickCreate(context, l10n),
              )
            : Revision3ProjectCommand.disabled(mutationDisabledReason),
        problemsCommand: Revision3ProjectCommand.enabled(
          () => Revision3ProjectWorkspace.navigate(
            context,
            const Revision3ProjectWorkspaceLocation(
              Revision3ProjectWorkspaceSection.validateTest,
            ),
          ),
        ),
        settingsCommand: Revision3ProjectCommand.enabled(
          () => _openSettings(context),
        ),
        busy: busy,
        copy: l10n.localeName.startsWith('de')
            ? Revision3ProjectCommandBarCopy.german
            : const Revision3ProjectCommandBarCopy(),
      ),
    );
  }

  Widget _buildManagedProjectTechnicalDetails(AppLocalizations l10n) =>
      ExpansionTile(
        key: const Key('managed-project-technical-details'),
        dense: true,
        visualDensity: VisualDensity.compact,
        tilePadding: const EdgeInsets.symmetric(horizontal: 16),
        childrenPadding: const EdgeInsets.fromLTRB(16, 0, 16, 8),
        title: Text(l10n.managedProjectTechnicalDetails),
        children: [
          LayoutBuilder(
            builder: (context, constraints) {
              double factWidth(double preferred) {
                final available = constraints.maxWidth;
                return available.isFinite && available < preferred
                    ? available
                    : preferred;
              }

              return Align(
                alignment: Alignment.centerLeft,
                child: Wrap(
                  spacing: 20,
                  runSpacing: 2,
                  children: [
                    SizedBox(
                      width: factWidth(360),
                      child: _ProjectFact(
                        label: l10n.projectRoot,
                        value: project.root.path,
                        valueKey: const Key('managed-project-root'),
                      ),
                    ),
                    SizedBox(
                      width: factWidth(300),
                      child: _ProjectFact(
                        label: l10n.projectId,
                        value: project.projectId,
                        valueKey: const Key('managed-project-id'),
                      ),
                    ),
                    SizedBox(
                      width: factWidth(160),
                      child: _ProjectFact(
                        label: l10n.projectRevision,
                        value: '${project.projectRevision}',
                        valueKey: const Key('managed-project-revision'),
                      ),
                    ),
                    SizedBox(
                      width: factWidth(460),
                      child: _ProjectFact(
                        label: l10n.projectHeadSha256,
                        value: project.head.snapshotSha256,
                        valueKey: const Key('managed-project-head'),
                      ),
                    ),
                    SizedBox(
                      width: factWidth(160),
                      child: _ProjectFact(
                        label: l10n.projectSnapshotBytes,
                        value: '${project.head.snapshotByteLength}',
                        valueKey: const Key('managed-project-head-bytes'),
                      ),
                    ),
                  ],
                ),
              );
            },
          ),
        ],
      );

  Widget _buildManagedSettingsArea(AppLocalizations l10n) => LayoutBuilder(
    builder: (context, constraints) {
      final detailsMaximumHeight = constraints.maxHeight.isFinite
          ? constraints.maxHeight * 0.45
          : 320.0;
      return Column(
        children: [
          ConstrainedBox(
            constraints: BoxConstraints(maxHeight: detailsMaximumHeight),
            child: SingleChildScrollView(
              key: const Key('managed-project-technical-details-scroll'),
              primary: false,
              child: _buildManagedProjectTechnicalDetails(l10n),
            ),
          ),
          const Expanded(child: SettingsTab()),
        ],
      );
    },
  );

  String _projectCommandBarDisplayName(AppLocalizations l10n) {
    final loaded = _projectDisplayName?.trim();
    if (loaded != null && loaded.isNotEmpty) return loaded;
    final folderName = p.basename(p.normalize(project.root.path)).trim();
    return folderName.isEmpty || folderName == '.'
        ? l10n.managedDashboardUntitledProject
        : folderName;
  }

  String _workspaceSectionLabel(
    AppLocalizations l10n,
    Revision3ProjectWorkspaceSection section,
  ) => switch (section) {
    Revision3ProjectWorkspaceSection.home => l10n.managedWorkspaceHomeLabel,
    Revision3ProjectWorkspaceSection.content =>
      l10n.managedWorkspaceContentLabel,
    Revision3ProjectWorkspaceSection.story => l10n.managedWorkspaceStoryLabel,
    Revision3ProjectWorkspaceSection.world => l10n.managedWorkspaceWorldLabel,
    Revision3ProjectWorkspaceSection.localizationVoice =>
      l10n.managedWorkspaceLocalizationVoiceLabel,
    Revision3ProjectWorkspaceSection.validateTest =>
      l10n.managedWorkspaceValidateTestLabel,
    Revision3ProjectWorkspaceSection.buildRelease =>
      l10n.managedWorkspaceBuildReleaseLabel,
    Revision3ProjectWorkspaceSection.history =>
      l10n.managedWorkspaceHistoryLabel,
    Revision3ProjectWorkspaceSection.settingsExpert =>
      l10n.managedWorkspaceSettingsExpertLabel,
  };

  Future<void> _openProjectSearch(BuildContext context) async {
    final expected = project;
    if (!_isCurrentQuestOpeningCheckpoint(expected)) return;
    Revision3ProjectWorkspace.navigate(
      context,
      const Revision3ProjectWorkspaceLocation(
        Revision3ProjectWorkspaceSection.content,
        secondary: Revision3ScopedContentBrowser.searchAllSecondaryRoute,
      ),
    );
    await WidgetsBinding.instance.endOfFrame;
    if (!context.mounted || !_isCurrentQuestOpeningCheckpoint(expected)) return;
    await _scopedContentBrowserController.openSearchAll();
  }

  Future<void> _openProjectQuickCreate(
    BuildContext context,
    AppLocalizations l10n,
  ) async {
    final expected = project;
    if (!_isCurrentQuestOpeningCheckpoint(expected) ||
        !_storyMutationsEnabled) {
      return;
    }
    final gameConfigured = gameRoot != null;
    final gameRequiredReason = l10n.managedDashboardMissingGameDescription;
    final action = await showModalBottomSheet<_ProjectQuickCreateAction>(
      context: context,
      showDragHandle: true,
      isScrollControlled: true,
      builder: (sheetContext) {
        Widget option({
          required Key key,
          required IconData icon,
          required String title,
          required String description,
          required _ProjectQuickCreateAction action,
          String? disabledReason,
        }) {
          final enabled = disabledReason == null;
          return Tooltip(
            message: disabledReason ?? title,
            child: Semantics(
              button: true,
              enabled: enabled,
              label: title,
              hint: disabledReason ?? description,
              child: ListTile(
                key: key,
                enabled: enabled,
                leading: Icon(icon),
                title: Text(title),
                subtitle: Text(disabledReason ?? description),
                trailing: Icon(
                  enabled ? Icons.chevron_right : Icons.lock_outline,
                ),
                onTap: enabled
                    ? () => Navigator.of(sheetContext).pop(action)
                    : null,
              ),
            ),
          );
        }

        return SafeArea(
          child: ConstrainedBox(
            constraints: const BoxConstraints(maxHeight: 560),
            child: SingleChildScrollView(
              padding: const EdgeInsets.fromLTRB(12, 0, 12, 16),
              child: Column(
                mainAxisSize: MainAxisSize.min,
                crossAxisAlignment: CrossAxisAlignment.stretch,
                children: [
                  Padding(
                    padding: const EdgeInsets.fromLTRB(12, 0, 12, 8),
                    child: Text(
                      l10n.managedDashboardCreateHeading,
                      style: Theme.of(sheetContext).textTheme.titleLarge,
                    ),
                  ),
                  option(
                    key: const Key('managed-project-create-npc-opening'),
                    icon: Icons.record_voice_over_outlined,
                    title: l10n.managedStoryWorkspaceCreateNpcOpening,
                    description: l10n.managedNpcOpeningRecipeDescription,
                    action: _ProjectQuickCreateAction.npcOpening,
                    disabledReason: gameConfigured ? null : gameRequiredReason,
                  ),
                  option(
                    key: const Key('managed-project-create-quest-opening'),
                    icon: Icons.auto_stories_outlined,
                    title: l10n.managedStoryWorkspaceCreateQuestOpening,
                    description: l10n.managedQuestOpeningRecipeDescription,
                    action: _ProjectQuickCreateAction.questOpening,
                    disabledReason: gameConfigured ? null : gameRequiredReason,
                  ),
                  Padding(
                    padding: const EdgeInsets.fromLTRB(12, 12, 12, 4),
                    child: Text(
                      l10n.managedStoryWorkspaceCreateAdvanced,
                      style: Theme.of(sheetContext).textTheme.labelLarge,
                    ),
                  ),
                  option(
                    key: const Key('managed-project-create-npc'),
                    icon: Icons.person_add_alt_1_outlined,
                    title: l10n.managedActionNewNpcTitle,
                    description: l10n.managedActionNewNpcDescription,
                    action: _ProjectQuickCreateAction.npcDraft,
                    disabledReason: gameConfigured ? null : gameRequiredReason,
                  ),
                  option(
                    key: const Key('managed-project-create-dialog-line'),
                    icon: Icons.chat_bubble_outline,
                    title: l10n.managedActionNewDialogLineTitle,
                    description: l10n.managedActionNewDialogLineDescription,
                    action: _ProjectQuickCreateAction.dialogLine,
                  ),
                ],
              ),
            ),
          ),
        );
      },
    );
    if (action == null || !context.mounted) return;
    if (!_isCurrentQuestOpeningCheckpoint(expected) ||
        !_storyMutationsEnabled) {
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(content: Text(l10n.managedStoryWorkspaceCheckpointMismatch)),
      );
      return;
    }
    switch (action) {
      case _ProjectQuickCreateAction.npcOpening:
        await _openNpcOpeningRecipe(context);
        return;
      case _ProjectQuickCreateAction.npcDraft:
        await _openNpcWizard(context, selectPublishedInStory: true);
        return;
      case _ProjectQuickCreateAction.questOpening:
        await _openQuestOpeningRecipe(context);
        return;
      case _ProjectQuickCreateAction.dialogLine:
        Revision3ProjectWorkspace.navigate(
          context,
          const Revision3ProjectWorkspaceLocation(
            Revision3ProjectWorkspaceSection.localizationVoice,
          ),
        );
        await WidgetsBinding.instance.endOfFrame;
        if (context.mounted && _isCurrentQuestOpeningCheckpoint(expected)) {
          await _openDialogLineEntry(context);
        }
        return;
    }
  }

  Widget _buildHistorySection(AppLocalizations l10n) =>
      Revision3ProjectHistoryPage(
        checkpointIdentity: (
          project.projectRevision,
          project.head.canonicalJson,
        ),
        load: widget.loadProjectHistory,
        restore: widget.restoreProjectHistory,
        canRestore: widget.canRestoreProjectHistory,
        restoreDisabledReason: widget.historyRestoreDisabledReason,
        copy: Revision3ProjectHistoryPageCopy(
          title: l10n.managedProjectHistoryTitle,
          description: l10n.managedProjectHistoryDescription,
          projectOnlyBoundary: l10n.managedProjectHistoryBoundary,
          refresh: l10n.managedProjectHistoryRefresh,
          loading: l10n.managedProjectHistoryLoading,
          loadFailedTitle: l10n.managedProjectHistoryLoadFailed,
          retry: l10n.managedProjectHistoryRetry,
          currentVersion: l10n.managedProjectHistoryCurrentVersion,
          previousVersions: l10n.managedProjectHistoryPreviousVersions,
          undoLastChange: l10n.managedProjectHistoryUndo,
          restoreVersion: l10n.managedProjectHistoryRestoreVersion,
          restoreDialogTitle: l10n.managedProjectHistoryRestoreTitle,
          restoreDialogBody: l10n.managedProjectHistoryRestoreBody,
          restoreProjectOnlyBoundary: l10n.managedProjectHistoryRestoreBoundary,
          cancel: l10n.managedProjectHistoryCancel,
          restore: l10n.managedProjectHistoryRestore,
          restoring: l10n.managedProjectHistoryRestoring,
          restoreFailed: l10n.managedProjectHistoryRestoreFailed,
          restoreSucceeded: l10n.managedProjectHistoryRestoreSucceeded,
          noPreviousVersions: l10n.managedProjectHistoryEmpty,
          recordingStartsAt: l10n.managedProjectHistoryRecordingStartsAt,
          olderVersionsExpired: l10n.managedProjectHistoryTruncated,
          revisionLabel: l10n.managedProjectHistoryRevision,
          currentBadge: l10n.managedProjectHistoryCurrentBadge,
        ),
      );

  Widget _buildContentWorkspace(
    BuildContext context,
    Revision3ProjectWorkspaceLocation location,
    AppLocalizations l10n,
  ) => Revision3ContentWorkspace(
    location: location,
    libraryLabel: l10n.managedContentWorkspaceBrowseLabel,
    dataAssetsLabel: l10n.managedContentWorkspaceVerifiedEditsLabel,
    library: _ManagedRevision3GlobalContentHost(
      contentLibraryController: _contentLibraryController,
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
            controller: _scopedContentBrowserController,
            onAllSourcesActivated: _activateGlobalSearchQuery,
            initialScope:
                location.secondary ==
                    Revision3ScopedContentBrowser.searchAllSecondaryRoute
                ? Revision3ScopedContentScope.allSources
                : Revision3ScopedContentScope.thisMod,
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
              openStoryDraftInStory: project.requiresReopen
                  ? null
                  : (index, entity) =>
                        _openLibraryDraftInStory(context, index, entity),
              openStoryDraftInStoryDisabledReason: project.requiresReopen
                  ? l10n.managedContentOpenInStoryRequiresReopen
                  : null,
              openStoryDraftLabel: l10n.managedContentOpenInStory,
              openStoryDraftDescription:
                  l10n.managedContentOpenInStoryDescription,
              openStoryDraftFailureMessage:
                  l10n.managedContentOpenInStoryFailed,
              controller: contentLibraryController,
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
                _openNpcWizard(
                  context,
                  initialCatalogId: catalogId,
                  selectPublishedInStory: true,
                ),
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
                queryFocusNode: _globalSearchQueryFocusNode,
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
                    _openNpcWizard(
                      context,
                      initialCatalogId: catalogId,
                      selectPublishedInStory: true,
                    ),
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
      controller: _dataAssetStagePanelController,
      projectRoot: project.root.path,
      projectId: project.projectId,
      projectRevision: project.projectRevision,
      projectHead: project.head,
      requiresReopen: project.requiresReopen,
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
      buildReviewedStage: gameRoot == null || project.requiresReopen
          ? null
          : buildReviewedDataAsset,
      pickBuildParentDirectory: gameRoot == null || project.requiresReopen
          ? null
          : pickDataAssetBuildParent,
      buildUnavailableReason: project.requiresReopen
          ? 'Reopen this managed project before building files.'
          : gameRoot == null
          ? 'Choose the Gothic 1 Remake installation in Settings before building files.'
          : null,
      browseInstalledPackages: gameRoot == null
          ? null
          : () => _openInstalledPackageBrowser(context, gameRoot!),
    ),
  );

  Widget _buildStorySection(BuildContext context, AppLocalizations l10n) {
    final gameConfigured = gameRoot != null;
    final gameRequiredReason = l10n.managedDashboardMissingGameDescription;
    final canMutateStory = _storyMutationsEnabled;
    final storyMutationDisabledReason = _storyMutationDisabledReason(l10n);
    final canCreateStory = gameConfigured && canMutateStory;
    final createStoryDisabledReason =
        storyMutationDisabledReason ?? gameRequiredReason;
    return Revision3StoryWorkspace(
      projectRoot: project.root.path,
      projectId: project.projectId,
      projectRevision: project.projectRevision,
      projectHeadCanonicalJson: project.head.canonicalJson,
      load: loadContentIndex,
      copy: _storyWorkspaceCopy(l10n),
      controller: _storyWorkspaceController,
      removeDraft: canMutateStory ? removeStoryDraft : null,
      removeDraftDisabledReason:
          storyMutationDisabledReason ?? removeStoryDraftDisabledReason,
      createNpcOpening: canCreateStory
          ? () => _openNpcOpeningRecipe(context)
          : null,
      createNpcDraft: canCreateStory
          ? () => _openNpcWizard(context, selectPublishedInStory: true)
          : null,
      createQuestOpening: canCreateStory
          ? () => _openQuestOpeningRecipe(context)
          : null,
      createQuestDraft: canCreateStory
          ? () => _openQuestWizard(context, selectPublishedInStory: true)
          : null,
      createNpcOpeningDisabledReason: canCreateStory
          ? null
          : createStoryDisabledReason,
      createNpcDraftDisabledReason: canCreateStory
          ? null
          : createStoryDisabledReason,
      createQuestOpeningDisabledReason: canCreateStory
          ? null
          : createStoryDisabledReason,
      createQuestDraftDisabledReason: canCreateStory
          ? null
          : createStoryDisabledReason,
      editQuestOutline: canMutateStory
          ? (index, quest) => _openQuestOutlineEditor(context, index, quest)
          : null,
      editQuestOutlineDisabledReason: storyMutationDisabledReason,
      editQuestContext: gameConfigured && canMutateStory
          ? (index, quest) => _openQuestContextEditor(context, index, quest)
          : null,
      editQuestContextDisabledReason: gameConfigured && canMutateStory
          ? null
          : createStoryDisabledReason,
      editQuestTransitions: canMutateStory
          ? (index, quest) => _openQuestTransitionsEditor(context, index, quest)
          : null,
      editQuestTransitionsDisabledReason: storyMutationDisabledReason,
      editNpcProfile: gameConfigured && canMutateStory
          ? (index, npc) => _openNpcProfileEditor(context, index, npc)
          : null,
      editNpcProfileDisabledReason: gameConfigured && canMutateStory
          ? null
          : createStoryDisabledReason,
      inspectQuestSource: gameConfigured
          ? (index, quest) => _openQuestSourceInspection(context, index, quest)
          : null,
      inspectQuestSourceDisabledReason: gameConfigured
          ? null
          : gameRequiredReason,
      inspectNpcSource: (index, npc) => _openNpcProfile(context, index, npc),
      questJourneyBuilder:
          ({required index, required quest, required onOpenDialogLine}) =>
              _buildQuestJourneyView(
                context,
                l10n,
                index: index,
                quest: quest,
                onOpenDialogLine: onOpenDialogLine,
              ),
      questTranscriptBuilder:
          ({
            required index,
            required quest,
            required selectedLineId,
            required onSelectedLineChanged,
          }) => _buildQuestTranscriptPanel(
            context,
            l10n,
            index: index,
            quest: quest,
            selectedLineId: selectedLineId,
            onSelectedLineChanged: onSelectedLineChanged,
          ),
      npcDialogVoiceBuilder:
          ({
            required index,
            required npc,
            required selectedLineId,
            required onSelectedLineChanged,
          }) => _buildNpcDialogVoicePanel(
            context,
            l10n,
            index: index,
            npc: npc,
            selectedLineId: selectedLineId,
            onSelectedLineChanged: onSelectedLineChanged,
          ),
      onOpenExternalEntity: (entityId) =>
          _openStoryExternalEntity(context, entityId),
      onOpenExternalAsset: (assetSha256) =>
          _openStoryExternalAsset(context, assetSha256),
    );
  }

  bool get _storyMutationsEnabled =>
      !project.requiresReopen &&
      !widget.managedWorkspaceDirty &&
      !widget.recoveryBusy;

  String? _storyMutationDisabledReason(AppLocalizations l10n) =>
      project.requiresReopen
      ? l10n.managedStoryWorkspaceMutationRequiresReopen
      : widget.managedWorkspaceDirty
      ? l10n.managedStoryWorkspaceMutationDirtyBlocked
      : widget.recoveryBusy
      ? l10n.managedProjectHistoryBusy
      : null;

  Widget _buildQuestJourneyView(
    BuildContext context,
    AppLocalizations l10n, {
    required Revision3ContentIndex index,
    required Revision3ContentEntity quest,
    required ValueChanged<String> onOpenDialogLine,
  }) {
    final basisProjectId = project.projectId;
    final basisProjectRevision = project.projectRevision;
    final basisHead = project.head;
    final transcript = _createQuestTranscriptService(
      basisProjectId: basisProjectId,
      basisProjectRevision: basisProjectRevision,
      basisHead: basisHead,
    );
    final service = Revision3QuestJourneyService(
      transitions: Revision3QuestTransitionsAuthoringService(
        loadSeed: loadQuestTransitionsSeed,
        publishTechnicalPlan: editQuestTransitions,
      ),
      transcript: transcript,
    );
    final copy = l10n.localeName.startsWith('de')
        ? const Revision3QuestJourneyPanelCopy.german()
        : const Revision3QuestJourneyPanelCopy.english();
    final canEdit = _storyMutationsEnabled;
    return Revision3QuestJourneyView(
      projectId: basisProjectId,
      projectRevision: basisProjectRevision,
      checkpointIdentity: basisHead.canonicalJson,
      authorityEpoch: _storyAuthorityEpoch,
      index: index,
      quest: quest,
      service: service,
      onEditNameObjectives: canEdit
          ? () => _openQuestOutlineEditor(context, index, quest)
          : null,
      onEditDescriptionConnections: !canEdit || gameRoot == null
          ? null
          : () => _openQuestContextEditor(context, index, quest),
      onEditStatesTransitions: canEdit
          ? () => _openQuestTransitionsEditor(context, index, quest)
          : null,
      editDisabledReason: _storyMutationDisabledReason(l10n),
      editDescriptionConnectionsDisabledReason: gameRoot == null
          ? l10n.managedDashboardMissingGameDescription
          : null,
      onOpenDialogLine: (row) => onOpenDialogLine(row.lineId),
      copy: copy,
    );
  }

  Widget _buildQuestTranscriptPanel(
    BuildContext context,
    AppLocalizations l10n, {
    required Revision3ContentIndex index,
    required Revision3ContentEntity quest,
    required String? selectedLineId,
    required ValueChanged<String?> onSelectedLineChanged,
  }) {
    final basisProjectId = project.projectId;
    final basisProjectRevision = project.projectRevision;
    final basisHead = project.head;
    final german = l10n.localeName.startsWith('de');
    final copy = german
        ? Revision3QuestTranscriptPanelCopy.german
        : Revision3QuestTranscriptPanelCopy.english;
    final mutationsEnabled =
        !project.requiresReopen &&
        !widget.managedWorkspaceDirty &&
        !widget.recoveryBusy;
    final mutationDisabledReason = project.requiresReopen
        ? copy.requiresReopen
        : widget.managedWorkspaceDirty
        ? (german
              ? 'Speichere oder verwirf die offenen Text\u00e4nderungen, bevor du das Quest-Transkript bearbeitest.'
              : 'Save or discard the open text edits before changing the Quest transcript.')
        : widget.recoveryBusy
        ? (german
              ? 'Warte, bis die laufende Projektaktion abgeschlossen ist.'
              : 'Wait for the current project action to finish.')
        : null;
    final service = _createQuestTranscriptService(
      basisProjectId: basisProjectId,
      basisProjectRevision: basisProjectRevision,
      basisHead: basisHead,
    );
    return Revision3QuestTranscriptPanel(
      projectId: index.projectId,
      projectRevision: index.projectRevision,
      projectCheckpointIdentity: basisHead.canonicalJson,
      questId: quest.id,
      questRevision: quest.revision,
      service: service,
      selectedLineId: selectedLineId,
      onSelectedLineChanged: onSelectedLineChanged,
      onCreateLine:
          ({
            required projection,
            required insertionIndex,
            required objectiveSlot,
            required publishTechnicalPlan,
          }) => _openQuestTranscriptLineEntry(
            context,
            projection: projection,
            publishTechnicalPlan: publishTechnicalPlan,
          ),
      onOpenTextVoice: ({required projection, required row, required locale}) =>
          _openQuestTranscriptTextVoice(
            context,
            projection: projection,
            row: row,
            locale: locale,
          ),
      onPublished: (publication) =>
          _onQuestTranscriptPublished(context, publication),
      mutationsEnabled: mutationsEnabled,
      mutationDisabledReason: mutationDisabledReason,
      copy: copy,
    );
  }

  Revision3QuestTranscriptAuthoringService _createQuestTranscriptService({
    required String basisProjectId,
    required int basisProjectRevision,
    required AuthoringWorkingHead basisHead,
  }) {
    final loadContentForBasis = loadContentIndex;
    final readLocalizationForBasis = readDialogLocalization;
    return Revision3QuestTranscriptAuthoringService(
      expectedHead: basisHead,
      loadContentIndex: () async {
        final loaded = await loadContentForBasis();
        if (!mounted ||
            project.projectId != basisProjectId ||
            project.projectRevision != basisProjectRevision ||
            project.head.canonicalJson != basisHead.canonicalJson ||
            loaded.projectId != basisProjectId ||
            loaded.projectRevision != basisProjectRevision) {
          throw const Revision3QuestTranscriptStaleCheckpointException();
        }
        return loaded;
      },
      readExactLocalization:
          ({
            required expectedProjectId,
            required expectedProjectRevision,
            required expectedHead,
            required localizationId,
            required expectedLocalizationRevision,
            required expectedLocId,
          }) async {
            if (expectedProjectId != basisProjectId ||
                expectedProjectRevision != basisProjectRevision ||
                expectedHead.canonicalJson != basisHead.canonicalJson) {
              throw const Revision3QuestTranscriptStaleCheckpointException();
            }
            try {
              final loaded = await readLocalizationForBasis(
                expectedProjectId: expectedProjectId,
                expectedProjectRevision: expectedProjectRevision,
                localizationId: localizationId,
                expectedLocalizationRevision: expectedLocalizationRevision,
                expectedLocId: expectedLocId,
              );
              if (!mounted ||
                  project.projectId != basisProjectId ||
                  project.projectRevision != basisProjectRevision ||
                  project.head.canonicalJson != basisHead.canonicalJson) {
                throw const Revision3QuestTranscriptStaleCheckpointException();
              }
              return loaded;
            } on Revision3DialogLineEntryRequiresReopenException {
              throw const Revision3QuestTranscriptRequiresReopenException();
            } on Revision3DialogLineEntryStaleCheckpointException {
              throw const Revision3QuestTranscriptStaleCheckpointException();
            }
          },
      publishReplace: publishQuestTranscriptReplace,
      publishCreate: publishQuestTranscriptCreate,
    );
  }

  Widget _buildNpcDialogVoicePanel(
    BuildContext context,
    AppLocalizations l10n, {
    required Revision3ContentIndex index,
    required Revision3ContentEntity npc,
    required String? selectedLineId,
    required ValueChanged<String?> onSelectedLineChanged,
  }) {
    final basisProjectId = project.projectId;
    final basisProjectRevision = project.projectRevision;
    final basisHead = project.head;
    final german = l10n.localeName.startsWith('de');
    final copy = german
        ? Revision3NpcDialogVoicePanelCopy.german
        : Revision3NpcDialogVoicePanelCopy.english;
    final mutationsEnabled = _storyMutationsEnabled;
    final mutationDisabledReason = project.requiresReopen
        ? copy.requiresReopen
        : widget.managedWorkspaceDirty
        ? (german
              ? 'Speichere oder verwirf die offenen Text\u00e4nderungen, bevor du die NPC-Begr\u00fc\u00dfungen bearbeitest.'
              : 'Save or discard the open text edits before changing NPC greetings.')
        : widget.recoveryBusy
        ? (german
              ? 'Warte, bis die laufende Projektaktion abgeschlossen ist.'
              : 'Wait for the current project action to finish.')
        : null;
    final service = _createNpcGreetingService(
      basisProjectId: basisProjectId,
      basisProjectRevision: basisProjectRevision,
      basisHead: basisHead,
    );
    return Revision3NpcDialogVoicePanel(
      projectId: index.projectId,
      projectRevision: index.projectRevision,
      projectCheckpointIdentity: basisHead.canonicalJson,
      npcId: npc.id,
      npcRevision: npc.revision,
      service: service,
      selectedLineId: selectedLineId,
      onSelectedLineChanged: onSelectedLineChanged,
      onCreateLine:
          ({
            required projection,
            required insertionIndex,
            required publishTechnicalPlan,
          }) => _openNpcGreetingLineEntry(
            context,
            projection: projection,
            publishTechnicalPlan: publishTechnicalPlan,
          ),
      onOpenTextVoice: ({required projection, required row, required locale}) =>
          _openNpcGreetingTextVoice(
            context,
            projection: projection,
            row: row,
            locale: locale,
          ),
      onPublished: (publication) =>
          _onNpcGreetingPublished(context, publication),
      mutationsEnabled: mutationsEnabled,
      mutationDisabledReason: mutationDisabledReason,
      copy: copy,
    );
  }

  Revision3NpcGreetingAuthoringService _createNpcGreetingService({
    required String basisProjectId,
    required int basisProjectRevision,
    required AuthoringWorkingHead basisHead,
  }) {
    final loadContentForBasis = loadContentIndex;
    final readLocalizationForBasis = readDialogLocalization;
    return Revision3NpcGreetingAuthoringService(
      expectedHead: basisHead,
      loadContentIndex: () async {
        final loaded = await loadContentForBasis();
        if (!mounted ||
            project.projectId != basisProjectId ||
            project.projectRevision != basisProjectRevision ||
            project.head.canonicalJson != basisHead.canonicalJson ||
            loaded.projectId != basisProjectId ||
            loaded.projectRevision != basisProjectRevision) {
          throw const Revision3NpcGreetingStaleCheckpointException();
        }
        return loaded;
      },
      readExactLocalization:
          ({
            required expectedProjectId,
            required expectedProjectRevision,
            required expectedHead,
            required localizationId,
            required expectedLocalizationRevision,
            required expectedLocId,
          }) async {
            if (expectedProjectId != basisProjectId ||
                expectedProjectRevision != basisProjectRevision ||
                expectedHead.canonicalJson != basisHead.canonicalJson) {
              throw const Revision3NpcGreetingStaleCheckpointException();
            }
            try {
              final loaded = await readLocalizationForBasis(
                expectedProjectId: expectedProjectId,
                expectedProjectRevision: expectedProjectRevision,
                localizationId: localizationId,
                expectedLocalizationRevision: expectedLocalizationRevision,
                expectedLocId: expectedLocId,
              );
              if (!mounted ||
                  project.projectId != basisProjectId ||
                  project.projectRevision != basisProjectRevision ||
                  project.head.canonicalJson != basisHead.canonicalJson) {
                throw const Revision3NpcGreetingStaleCheckpointException();
              }
              return loaded;
            } on Revision3DialogLineEntryRequiresReopenException {
              throw const Revision3NpcGreetingRequiresReopenException();
            } on Revision3DialogLineEntryStaleCheckpointException {
              throw const Revision3NpcGreetingStaleCheckpointException();
            }
          },
      publishReplace: publishNpcGreetingReplace,
      publishCreate: publishNpcGreetingCreate,
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
    return _ManagedRevision3VoiceCatalogGate(
      projectId: project.projectId,
      projectRevision: project.projectRevision,
      projectHeadCanonicalJson: project.head.canonicalJson,
      load: loadContentIndex,
      builder: (context, availability) {
        final intactVoiceLine = availability.hasIntactVoiceLine;
        return Revision3LocalizationVoiceWorkspace(
          projectId: project.projectId,
          projectRevision: project.projectRevision,
          projectCheckpointIdentity: project.head.canonicalJson,
          enableProductionQueue: true,
          controller: _localizationVoiceWorkspaceController,
          service: Revision3DialogLocalizationEditAuthoringService(
            loadContentIndex: availability.loadContentIndex,
            loadExactSeed: loadDialogLocalizationEditSeed,
            publishTechnicalPlan: publishDialogLocalizationEdit,
          ),
          loadVoiceCatalog: () async => Revision3VoiceCatalog.fromContentIndex(
            await availability.loadContentIndex(),
          ),
          voiceProductionCopy: l10n.localeName.startsWith('de')
              ? Revision3VoiceProductionCardCopy.german
              : Revision3VoiceProductionCardCopy.english,
          voiceProductionQueueCopy: l10n.localeName.startsWith('de')
              ? Revision3VoiceProductionQueueCopy.german
              : const Revision3VoiceProductionQueueCopy(),
          copy: _localizationVoiceWorkspaceCopy(l10n),
          onDirtyChanged: widget.onDialogLocalizationDirtyChanged,
          notice: !gameConfigured
              ? l10n.managedDashboardMissingGameDescription
              : availability.status == _ManagedVoiceCatalogGateStatus.loaded &&
                    !intactVoiceLine
              ? l10n.managedActionAddVoiceTakeRequiresDialogLine
              : availability.status ==
                    _ManagedVoiceCatalogGateStatus.unavailable
              ? l10n.managedDashboardLoadErrorDescription
              : null,
          onCreateDialogLine: project.requiresReopen
              ? null
              : () => _openDialogLineEntry(context),
          onAddVoiceTake:
              gameConfigured && intactVoiceLine && !project.requiresReopen
              ? () => _openVoiceWizard(context)
              : null,
          onImportVoiceFolder: gameConfigured && !project.requiresReopen
              ? () => _openVoiceFolderImport(context)
              : null,
          onManageVoiceTakes: intactVoiceLine && !project.requiresReopen
              ? () => _openVoiceTakeSelection(context)
              : null,
          onResolveVoiceTarget:
              gameConfigured && intactVoiceLine && !project.requiresReopen
              ? () => _openVoiceTargetResolver(context)
              : null,
          onAddVoiceTakeFor:
              gameConfigured && intactVoiceLine && !project.requiresReopen
              ? ({required initialLineId, required initialLocale}) =>
                    _openVoiceWizard(
                      context,
                      initialLineId: initialLineId,
                      initialLocale: initialLocale,
                      fixedContext: true,
                    )
              : null,
          onPlanRecordingFor: intactVoiceLine && !project.requiresReopen
              ? ({required initialLineId, required initialLocale}) =>
                    _planVoiceRecording(
                      context,
                      lineId: initialLineId,
                      locale: initialLocale,
                    )
              : null,
          onManageVoiceTakesFor: intactVoiceLine && !project.requiresReopen
              ? ({required initialLineId, required initialLocale}) =>
                    _openVoiceTakeSelection(
                      context,
                      initialLineId: initialLineId,
                      initialLocale: initialLocale,
                      fixedContext: true,
                    )
              : null,
          onResolveVoiceTargetFor:
              gameConfigured && intactVoiceLine && !project.requiresReopen
              ? ({required initialLineId, required initialLocale}) =>
                    _openVoiceTargetResolver(
                      context,
                      initialLineId: initialLineId,
                      initialLocale: initialLocale,
                      fixedContext: true,
                    )
              : null,
          onReviewVoiceChecksFor: !project.requiresReopen
              ? ({required initialLineId, required initialLocale}) =>
                    Revision3ProjectWorkspace.navigate(
                      context,
                      const Revision3ProjectWorkspaceLocation(
                        Revision3ProjectWorkspaceSection.validateTest,
                      ),
                    )
              : null,
          addVoiceDisabledReason: !gameConfigured
              ? l10n.managedDashboardMissingGameDescription
              : !intactVoiceLine
              ? l10n.managedActionAddVoiceTakeRequiresDialogLine
              : null,
          manageVoiceDisabledReason: !intactVoiceLine
              ? l10n.managedActionAddVoiceTakeRequiresDialogLine
              : null,
          resolveVoiceDisabledReason: !gameConfigured
              ? l10n.managedDashboardMissingGameDescription
              : !intactVoiceLine
              ? l10n.managedActionAddVoiceTakeRequiresDialogLine
              : null,
        );
      },
    );
  }

  Widget _buildValidateTestSection(
    BuildContext context,
    AppLocalizations l10n,
  ) {
    final gameConfigured = gameRoot != null;
    final problemProjectRoot = project.root.path;
    final problemProjectId = project.projectId;
    final problemProjectRevision = project.projectRevision;
    final problemProjectHeadCanonicalJson = project.head.canonicalJson;
    return Column(
      children: [
        Padding(
          padding: const EdgeInsets.fromLTRB(16, 16, 16, 8),
          child: Revision3VoiceBuildReadinessPanel(
            projectId: project.projectId,
            projectRevision: project.projectRevision,
            plan: planVoiceBuild,
            copy: _voiceBuildReadinessCopy(l10n),
            gameConfigured: gameConfigured,
            onResolveVoiceTarget: gameConfigured && !project.requiresReopen
                ? ({required initialLineId, required initialLocale}) =>
                      _openVoiceTargetResolver(
                        context,
                        initialLineId: initialLineId,
                        initialLocale: initialLocale,
                        fixedContext: true,
                      )
                : null,
            onManageVoiceTakes: !project.requiresReopen
                ? ({required initialLineId, required initialLocale}) =>
                      _openVoiceTakeSelection(
                        context,
                        initialLineId: initialLineId,
                        initialLocale: initialLocale,
                        fixedContext: true,
                      )
                : null,
            onBuild: gameConfigured && !project.requiresReopen
                ? () => _openVoiceBuild(context)
                : null,
          ),
        ),
        Expanded(
          child: Revision3ProjectProblemsView(
            projectRoot: project.root.path,
            projectId: project.projectId,
            projectRevision: project.projectRevision,
            projectHeadCanonicalJson: project.head.canonicalJson,
            loadContent: loadContentIndex,
            loadDataAssetStages: loadDataAssetStages,
            gameConfigured: gameConfigured,
            copy: _projectProblemsCopy(l10n),
            actions: Revision3ProjectProblemsActions(
              openEntity: (entityId) => _openProblemEntity(
                context,
                entityId: entityId,
                expectedProjectRoot: problemProjectRoot,
                expectedProjectId: problemProjectId,
                expectedProjectRevision: problemProjectRevision,
                expectedProjectHeadCanonicalJson:
                    problemProjectHeadCanonicalJson,
              ),
              openAsset: (assetSha256) => _openProblemAsset(
                context,
                assetSha256: assetSha256,
                expectedProjectRoot: problemProjectRoot,
                expectedProjectId: problemProjectId,
                expectedProjectRevision: problemProjectRevision,
                expectedProjectHeadCanonicalJson:
                    problemProjectHeadCanonicalJson,
              ),
              openDataAssetStage: (targetPath) => _openProblemDataAssetStage(
                context,
                targetPath: targetPath,
                expectedProjectRoot: problemProjectRoot,
                expectedProjectId: problemProjectId,
                expectedProjectRevision: problemProjectRevision,
                expectedProjectHeadCanonicalJson:
                    problemProjectHeadCanonicalJson,
              ),
              openSettings: () => Revision3ProjectWorkspace.navigate(
                context,
                const Revision3ProjectWorkspaceLocation(
                  Revision3ProjectWorkspaceSection.settingsExpert,
                ),
              ),
              verifyCurrentProject: verifyCurrentHead,
            ),
          ),
        ),
      ],
    );
  }

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
    bool hasStoryDraft(Revision3ContentIndex index) => index.entities.any(
      (entity) =>
          entity.kind == Revision3ContentEntityKind.npcDraft ||
          entity.kind == Revision3ContentEntityKind.questDraft,
    );
    void navigate(Revision3ProjectWorkspaceSection section) =>
        Revision3ProjectWorkspace.navigate(
          context,
          Revision3ProjectWorkspaceLocation(section),
        );

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
        continueHeading: l10n.managedDashboardContinueHeading,
        loadingSemanticsLabel: l10n.managedDashboardLoading,
        loadErrorSemanticsLabel: l10n.managedDashboardLoadError,
        loadErrorTitle: l10n.managedDashboardLoadError,
        loadErrorDescription: l10n.managedDashboardLoadErrorDescription,
        retryLabel: l10n.managedDashboardRetry,
      ),
      tasks: [
        Revision3ProjectDashboardAction(
          id: 'story',
          controlKey: const Key('managed-home-story'),
          icon: Icons.auto_stories_outlined,
          title: l10n.managedHomeStoryContinueTitle,
          titleBuilder: (index) => hasStoryDraft(index)
              ? l10n.managedHomeStoryContinueTitle
              : l10n.managedHomeStoryEmptyTitle,
          description: l10n.managedHomeStoryDescription,
          onPressed: () => navigate(Revision3ProjectWorkspaceSection.story),
        ),
        Revision3ProjectDashboardAction(
          id: 'dialog-voice',
          controlKey: const Key('managed-home-dialog-voice'),
          icon: Icons.record_voice_over_outlined,
          title: l10n.managedHomeDialogVoiceTitle,
          description: l10n.managedHomeDialogVoiceDescription,
          onPressed: () =>
              navigate(Revision3ProjectWorkspaceSection.localizationVoice),
        ),
        Revision3ProjectDashboardAction(
          id: 'problems',
          controlKey: const Key('managed-home-problems'),
          icon: Icons.rule_folder_outlined,
          title: l10n.managedHomeProblemsTitle,
          description: l10n.managedHomeProblemsDescription,
          onPressed: () =>
              navigate(Revision3ProjectWorkspaceSection.validateTest),
        ),
        Revision3ProjectDashboardAction(
          id: 'content',
          controlKey: const Key('managed-home-content'),
          icon: Icons.account_tree_outlined,
          title: l10n.managedHomeContentTitle,
          description: l10n.managedHomeContentDescription,
          onPressed: () => navigate(Revision3ProjectWorkspaceSection.content),
        ),
        Revision3ProjectDashboardAction(
          id: 'build',
          controlKey: const Key('managed-home-build'),
          icon: Icons.inventory_2_outlined,
          title: l10n.managedHomeBuildTitle,
          description: l10n.managedHomeBuildDescription,
          onPressed: () =>
              navigate(Revision3ProjectWorkspaceSection.buildRelease),
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

  bool _isCurrentQuestTranscriptProjection(
    Revision3QuestTranscriptProjection projection,
  ) =>
      mounted &&
      projection.projectId == project.projectId &&
      projection.projectRevision == project.projectRevision &&
      projection.checkpointIdentity == project.head.canonicalJson;

  Future<bool> _openQuestTranscriptLineEntry(
    BuildContext context, {
    required Revision3QuestTranscriptProjection projection,
    required Revision3DialogLineEntryTechnicalPublisher publishTechnicalPlan,
  }) async {
    if (project.requiresReopen ||
        widget.managedWorkspaceDirty ||
        widget.recoveryBusy ||
        !_isCurrentQuestTranscriptProjection(projection)) {
      return false;
    }
    final l10n = AppLocalizations.of(context);
    final result = await showDialog<Revision3DialogLineEntryDialogResult>(
      context: context,
      barrierDismissible: false,
      builder: (context) => Revision3DialogLineEntryDialog(
        service: Revision3DialogLineEntryAuthoringService(
          loadContentIndex: loadContentIndex,
          readExactLocalization: readDialogLocalization,
          publishTechnicalPlan: publishTechnicalPlan,
        ),
        copy: _dialogLineEntryCopy(l10n),
        allowOpenVoiceNext: false,
      ),
    );
    if (!context.mounted || result == null) return false;
    final publication = result.publication;
    if (publication.projectId != projection.projectId ||
        publication.projectRevision != projection.projectRevision + 1 ||
        !widget.isQuestTranscriptCheckpointCurrent(
          projectId: publication.projectId,
          projectRevision: publication.projectRevision,
        )) {
      throw const Revision3QuestTranscriptRequiresReopenException();
    }
    ScaffoldMessenger.of(context).showSnackBar(
      SnackBar(
        content: Text(
          l10n.managedActionNewDialogLineSaved(publication.projectRevision),
        ),
      ),
    );
    return true;
  }

  Future<void> _onQuestTranscriptPublished(
    BuildContext context,
    Revision3QuestTranscriptPublication publication,
  ) async {
    if (!widget.isQuestTranscriptCheckpointCurrent(
      projectId: publication.projectId,
      projectRevision: publication.projectRevision,
    )) {
      throw const Revision3QuestTranscriptRequiresReopenException();
    }
    if (!context.mounted) return;
    final german = AppLocalizations.of(context).localeName.startsWith('de');
    ScaffoldMessenger.of(context).showSnackBar(
      SnackBar(
        content: Text(
          german
              ? 'Quest-Transkript in Projektrevision ${publication.projectRevision} gespeichert. Build bleibt blockiert; Runtime bleibt unqualifiziert.'
              : 'Quest transcript saved in project revision ${publication.projectRevision}. Build remains blocked; runtime remains unqualified.',
        ),
      ),
    );
  }

  Future<bool> _openQuestTranscriptTextVoice(
    BuildContext context, {
    required Revision3QuestTranscriptProjection projection,
    required Revision3QuestTranscriptRow row,
    required String locale,
  }) async {
    if (!_isCurrentQuestTranscriptProjection(projection) ||
        !projection.rows.any((candidate) => identical(candidate, row))) {
      return false;
    }
    Revision3ProjectWorkspace.navigate(
      context,
      const Revision3ProjectWorkspaceLocation(
        Revision3ProjectWorkspaceSection.localizationVoice,
      ),
    );
    await WidgetsBinding.instance.endOfFrame;
    if (!context.mounted || !_isCurrentQuestTranscriptProjection(projection)) {
      return false;
    }
    return _localizationVoiceWorkspaceController.openExactTarget(
      Revision3LocalizationVoiceTarget(
        projectId: projection.projectId,
        projectRevision: projection.projectRevision,
        projectCheckpointIdentity: projection.checkpointIdentity,
        localizationStableKey: row.localizationStableKey,
        lineId: row.lineId,
        locale: locale,
      ),
    );
  }

  bool _isCurrentNpcGreetingProjection(
    Revision3NpcGreetingProjection projection,
  ) =>
      mounted &&
      projection.projectId == project.projectId &&
      projection.projectRevision == project.projectRevision &&
      projection.checkpointIdentity == project.head.canonicalJson;

  Future<bool> _openNpcGreetingLineEntry(
    BuildContext context, {
    required Revision3NpcGreetingProjection projection,
    required Revision3DialogLineEntryTechnicalPublisher publishTechnicalPlan,
  }) async {
    if (project.requiresReopen ||
        widget.managedWorkspaceDirty ||
        widget.recoveryBusy ||
        !_isCurrentNpcGreetingProjection(projection)) {
      return false;
    }
    final l10n = AppLocalizations.of(context);
    final result = await showDialog<Revision3DialogLineEntryDialogResult>(
      context: context,
      barrierDismissible: false,
      builder: (context) => Revision3DialogLineEntryDialog(
        service: Revision3DialogLineEntryAuthoringService(
          loadContentIndex: loadContentIndex,
          readExactLocalization: readDialogLocalization,
          publishTechnicalPlan: publishTechnicalPlan,
        ),
        copy: _dialogLineEntryCopy(l10n),
        allowOpenVoiceNext: false,
      ),
    );
    if (!context.mounted || result == null) return false;
    final publication = result.publication;
    if (publication.projectId != projection.projectId ||
        publication.projectRevision != projection.projectRevision + 1 ||
        !widget.isNpcGreetingCheckpointCurrent(
          projectId: publication.projectId,
          projectRevision: publication.projectRevision,
        )) {
      throw const Revision3NpcGreetingRequiresReopenException();
    }
    ScaffoldMessenger.of(context).showSnackBar(
      SnackBar(
        content: Text(
          l10n.managedActionNewDialogLineSaved(publication.projectRevision),
        ),
      ),
    );
    return true;
  }

  Future<void> _onNpcGreetingPublished(
    BuildContext context,
    Revision3NpcGreetingPublication publication,
  ) async {
    if (!widget.isNpcGreetingCheckpointCurrent(
      projectId: publication.projectId,
      projectRevision: publication.projectRevision,
    )) {
      throw const Revision3NpcGreetingRequiresReopenException();
    }
    if (!context.mounted) return;
    final german = AppLocalizations.of(context).localeName.startsWith('de');
    ScaffoldMessenger.of(context).showSnackBar(
      SnackBar(
        content: Text(
          german
              ? 'NPC-Begr\u00fc\u00dfungen in Projektrevision ${publication.projectRevision} gespeichert. Das sind Authoring-Metadaten; Build bleibt blockiert und Runtime unqualifiziert.'
              : 'NPC greetings saved in project revision ${publication.projectRevision}. These are authoring metadata; build remains blocked and runtime remains unqualified.',
        ),
      ),
    );
  }

  Future<bool> _openNpcGreetingTextVoice(
    BuildContext context, {
    required Revision3NpcGreetingProjection projection,
    required Revision3NpcGreetingRow row,
    required String locale,
  }) async {
    if (!_isCurrentNpcGreetingProjection(projection) ||
        !projection.rows.any((candidate) => identical(candidate, row))) {
      return false;
    }
    Revision3ProjectWorkspace.navigate(
      context,
      const Revision3ProjectWorkspaceLocation(
        Revision3ProjectWorkspaceSection.localizationVoice,
      ),
    );
    await WidgetsBinding.instance.endOfFrame;
    if (!context.mounted || !_isCurrentNpcGreetingProjection(projection)) {
      return false;
    }
    return _localizationVoiceWorkspaceController.openExactTarget(
      Revision3LocalizationVoiceTarget(
        projectId: projection.projectId,
        projectRevision: projection.projectRevision,
        projectCheckpointIdentity: projection.checkpointIdentity,
        localizationStableKey: row.localizationStableKey,
        lineId: row.lineId,
        locale: locale,
      ),
    );
  }

  Future<void> _openDialogLineEntry(BuildContext context) async {
    if (project.requiresReopen) return;
    final l10n = AppLocalizations.of(context);
    final result = await showDialog<Revision3DialogLineEntryDialogResult>(
      context: context,
      barrierDismissible: false,
      builder: (context) => Revision3DialogLineEntryDialog(
        service: Revision3DialogLineEntryAuthoringService(
          loadContentIndex: loadContentIndex,
          readExactLocalization: readDialogLocalization,
          publishTechnicalPlan: publishDialogLine,
        ),
        copy: _dialogLineEntryCopy(l10n),
        allowOpenVoiceNext: gameRoot != null,
      ),
    );
    if (!context.mounted || result == null) return;
    ScaffoldMessenger.of(context).showSnackBar(
      SnackBar(
        content: Text(
          l10n.managedActionNewDialogLineSaved(
            result.publication.projectRevision,
          ),
        ),
      ),
    );
    if (!result.openVoiceNext || gameRoot == null) return;

    // Publication refreshes the managed project controller before it returns.
    // Wait for this child to receive the new head-bound callbacks; reusing the
    // old callback would correctly fail closed as a stale transaction.
    await WidgetsBinding.instance.endOfFrame;
    if (!context.mounted ||
        project.requiresReopen ||
        project.projectRevision != result.publication.projectRevision) {
      return;
    }
    await _openVoiceWizard(
      context,
      initialLineId: result.publication.lineId,
      initialLocale: result.publication.locale,
      fixedContext: true,
    );
  }

  Future<void> _openVoiceWizard(
    BuildContext context, {
    String? initialLineId,
    String? initialLocale,
    bool fixedContext = false,
  }) async {
    if (gameRoot == null || project.requiresReopen) return;
    final l10n = AppLocalizations.of(context);
    final german = l10n.localeName.startsWith('de');
    final publication = await showDialog<Revision3VoiceTakePublication>(
      context: context,
      builder: (context) => Revision3VoiceTakeDialog(
        service: Revision3VoiceAuthoringService(
          loadContentIndex: loadContentIndex,
          publishTechnicalPlan: publishVoiceTake,
        ),
        copy: german
            ? const Revision3VoiceTakeDialogCopy.german()
            : const Revision3VoiceTakeDialogCopy.english(),
        initialLineId: initialLineId,
        initialLocale: initialLocale,
        fixedContext: fixedContext,
      ),
    );
    if (!context.mounted || publication == null) return;
    ScaffoldMessenger.of(context).showSnackBar(
      SnackBar(
        content: Text(l10n.managedVoiceTakeSaved(publication.projectRevision)),
      ),
    );
  }

  Future<void> _planVoiceRecording(
    BuildContext context, {
    required String lineId,
    required String locale,
  }) async {
    if (project.requiresReopen) return;
    final service = Revision3DialogVoiceSlotCreationAuthoringService(
      loadContentIndex: loadContentIndex,
      publishTechnicalPlan: publishDialogVoiceSlotCreation,
    );
    final checkpoint = await service.loadCatalog();
    if (checkpoint.projectId != project.projectId ||
        checkpoint.projectRevision != project.projectRevision) {
      throw const Revision3DialogVoiceSlotCreationStaleCheckpointException();
    }
    await service.publish(
      checkpoint: checkpoint,
      lineId: lineId,
      locale: locale,
    );
    if (!context.mounted) return;
    ScaffoldMessenger.of(context).showSnackBar(
      SnackBar(
        content: Text(AppLocalizations.of(context).managedVoiceSlotPlanSuccess),
      ),
    );
  }

  Future<void> _openVoiceFolderImport(BuildContext context) async {
    if (gameRoot == null || project.requiresReopen) return;
    final l10n = AppLocalizations.of(context);
    final adapter = Revision3VoiceFolderManagedAdapter(
      expectedProjectId: project.projectId,
      expectedProjectRevision: project.projectRevision,
      expectedProjectHead: project.head.canonicalJson,
      loadContentIndex: loadContentIndex,
      planNative: planVoiceFolder,
      publishNative: publishVoiceFolder,
    );
    final localeName = l10n.localeName;
    final initialLocale = revision3VoiceLocaleIsCanonical(localeName)
        ? localeName
        : '';
    final publication = await showRevision3VoiceFolderImportDialog(
      context: context,
      projectId: project.projectId,
      projectRevision: project.projectRevision,
      projectHead: project.head.canonicalJson,
      checkpointToken: adapter.expectedCheckpointToken,
      service: adapter.service,
      copy: localeName.startsWith('de')
          ? const Revision3VoiceFolderImportDialogCopy.german()
          : const Revision3VoiceFolderImportDialogCopy.english(),
      pickFolder: pickVoiceFolder,
      initialLocale: initialLocale,
    );
    if (publication == null ||
        !context.mounted ||
        !widget.isVoiceFolderPublicationCurrent(publication)) {
      return;
    }
    ScaffoldMessenger.of(context).showSnackBar(
      SnackBar(
        content: Text(
          l10n.managedVoiceFolderImportSaved(
            publication.importedCount,
            publication.projectRevision,
          ),
        ),
      ),
    );
  }

  Future<void> _openVoiceTakeSelection(
    BuildContext context, {
    String? initialLineId,
    String? initialLocale,
    bool fixedContext = false,
  }) async {
    if (project.requiresReopen) return;
    final l10n = AppLocalizations.of(context);
    final messenger = ScaffoldMessenger.of(context);
    final german = l10n.localeName.startsWith('de');
    final previewService = Revision3VoiceTakePreviewAuthoringService(
      loadContentIndex: loadContentIndex,
      materializeTechnicalPlan: materializeVoiceTakePreview,
    );
    final mediaQaService = Revision3VoiceTakeMediaQaAuthoringService(
      loadContentIndex: loadContentIndex,
      inspectTechnicalPlan: inspectVoiceTakeMediaQa,
    );
    final previewPlayback =
        Revision3VoiceTakePreviewPlaybackController.standard();
    try {
      final publication =
          await showDialog<Revision3VoiceTakeSelectionPublication>(
            context: context,
            builder: (context) => Revision3VoiceTakeSelectionDialog(
              service: Revision3VoiceTakeSelectionAuthoringService(
                loadContentIndex: loadContentIndex,
                publishTechnicalPlan: publishVoiceTakeSelection,
              ),
              statusService: Revision3VoiceTakeStatusAuthoringService(
                loadContentIndex: loadContentIndex,
                publishTechnicalPlan: publishVoiceTakeStatus,
              ),
              removalService: Revision3VoiceTakeRemovalAuthoringService(
                loadContentIndex: loadContentIndex,
                publishTechnicalPlan: publishVoiceTakeRemoval,
              ),
              slotRemovalService:
                  Revision3DialogVoiceSlotRemovalAuthoringService(
                    loadContentIndex: loadContentIndex,
                    publishTechnicalPlan: publishDialogVoiceSlotRemoval,
                  ),
              previewPlayback: previewPlayback,
              mediaQaInspect:
                  ({
                    required checkpoint,
                    required lineId,
                    required locale,
                    required takeId,
                  }) async =>
                      Revision3VoiceTakeMediaQaDialogResult.fromAuthoring(
                        await mediaQaService.inspect(
                          checkpoint: checkpoint,
                          lineId: lineId,
                          locale: locale,
                          takeId: takeId,
                        ),
                      ),
              previewMaterialize:
                  ({
                    required checkpoint,
                    required lineId,
                    required locale,
                    required takeId,
                  }) async {
                    final capability = await previewService.materialize(
                      checkpoint: checkpoint,
                      lineId: lineId,
                      locale: locale,
                      takeId: takeId,
                    );
                    return Revision3VoiceTakePreviewPlaybackLease(
                      path: capability.path,
                      isClosed: () => capability.isClosed,
                      close: capability.close,
                    );
                  },
              copy: german
                  ? Revision3VoiceTakeSelectionDialogCopy.german
                  : Revision3VoiceTakeSelectionDialogCopy.english,
              initialLineId: initialLineId,
              initialLocale: initialLocale,
              fixedContext: fixedContext,
            ),
          );
      if (!messenger.mounted || publication == null) return;
      messenger.showSnackBar(
        SnackBar(
          content: Text(
            publication.cleared
                ? l10n.managedVoiceSelectionCleared(publication.projectRevision)
                : l10n.managedVoiceSelectionSelected(
                    publication.projectRevision,
                  ),
          ),
        ),
      );
    } finally {
      await previewPlayback.dispose();
    }
  }

  Future<void> _openVoiceTargetResolver(
    BuildContext context, {
    String? initialLineId,
    String? initialLocale,
    bool fixedContext = false,
  }) async {
    if (gameRoot == null || project.requiresReopen) return;
    final l10n = AppLocalizations.of(context);
    final german = l10n.localeName.startsWith('de');
    final publication = await showDialog<Revision3VoiceTargetPublication>(
      context: context,
      builder: (context) => Revision3VoiceTargetDialog(
        service: Revision3VoiceTargetAuthoringService(
          loadContentIndex: loadContentIndex,
          publishTechnicalPlan: publishVoiceTarget,
        ),
        copy: german
            ? Revision3VoiceTargetDialogCopy.german
            : Revision3VoiceTargetDialogCopy.english,
        initialLineId: initialLineId,
        initialLocale: initialLocale,
        fixedContext: fixedContext,
      ),
    );
    if (!context.mounted || publication == null) return;
    final message = switch (publication.resolution) {
      AuthoringRevision3VoiceTargetResolutionState.unresolved =>
        l10n.managedVoiceTargetUnresolvedSaved(publication.projectRevision),
      AuthoringRevision3VoiceTargetResolutionState.resolved =>
        l10n.managedVoiceTargetResolvedSaved(publication.projectRevision),
      AuthoringRevision3VoiceTargetResolutionState.ambiguous =>
        l10n.managedVoiceTargetAmbiguousSaved(
          publication.matchCount,
          publication.projectRevision,
        ),
    };
    ScaffoldMessenger.of(
      context,
    ).showSnackBar(SnackBar(content: Text(message)));
  }

  Future<void> _openVoiceBuild(BuildContext context) async {
    if (gameRoot == null || project.requiresReopen) return;
    final l10n = AppLocalizations.of(context);
    final copy = _voiceBuildDialogCopy(l10n);
    final messenger = ScaffoldMessenger.of(context);
    final result = await showDialog<AuthoringRevision3VoiceBuildResult>(
      context: context,
      builder: (_) => Revision3VoiceBuildDialog(
        plan: planVoiceBuild,
        build: buildVoiceBundle,
        pickExistingParentDirectory: pickVoiceBuildParent,
        copy: copy,
        onDeepLinkFailure: () {
          if (!messenger.mounted) return;
          messenger.hideCurrentSnackBar();
          messenger.showSnackBar(
            SnackBar(content: Text(copy.readiness.workflowOpenFailed)),
          );
        },
        onResolveVoiceTarget:
            ({required initialLineId, required initialLocale}) =>
                _openVoiceTargetResolver(
                  context,
                  initialLineId: initialLineId,
                  initialLocale: initialLocale,
                  fixedContext: true,
                ),
        onManageVoiceTakes:
            ({required initialLineId, required initialLocale}) =>
                _openVoiceTakeSelection(
                  context,
                  initialLineId: initialLineId,
                  initialLocale: initialLocale,
                  fixedContext: true,
                ),
      ),
    );
    if (!context.mounted || result == null) return;
    final message = result.isBuilt
        ? l10n.managedVoiceBuildBuiltMessage(result.output!)
        : l10n.managedVoiceBuildBlockedMessage(result.report!.blockers.length);
    ScaffoldMessenger.of(
      context,
    ).showSnackBar(SnackBar(content: Text(message)));
  }

  Future<void> _openNpcOpeningRecipe(BuildContext context) async {
    final configuredGameRoot = gameRoot;
    if (configuredGameRoot == null ||
        !_storyMutationsEnabled ||
        _npcOpeningRecipeUiBusy ||
        _npcOpeningRecipe.isRunning) {
      return;
    }
    final l10n = AppLocalizations.of(context);
    final openingCheckpoint = project;
    final selectionOrigin = _storySelectionOrigin;
    setState(() => _npcOpeningRecipeUiBusy = true);
    try {
      final confirmed =
          await showDialog<bool>(
            context: context,
            builder: (dialogContext) => AlertDialog(
              key: const Key('managed-npc-opening-recipe-intro'),
              scrollable: true,
              title: Text(l10n.managedNpcOpeningRecipeTitle),
              content: Text(l10n.managedNpcOpeningRecipeIntroduction),
              actions: [
                TextButton(
                  key: const Key('managed-npc-opening-recipe-cancel'),
                  onPressed: () => Navigator.of(dialogContext).pop(false),
                  child: Text(l10n.cancel),
                ),
                FilledButton.icon(
                  key: const Key('managed-npc-opening-recipe-start'),
                  onPressed: () => Navigator.of(dialogContext).pop(true),
                  icon: const Icon(Icons.arrow_forward),
                  label: Text(l10n.managedNpcOpeningRecipeStart),
                ),
              ],
            ),
          ) ??
          false;
      if (!mounted || !context.mounted || !confirmed) return;

      final outcome = await _npcOpeningRecipe.run(
        openingCheckpoint: openingCheckpoint,
        readCurrentCheckpoint: () async => currentManagedProject,
        createNpc: ({required expectedCheckpoint}) async {
          _requireNpcOpeningCheckpoint(expectedCheckpoint);
          final publication = await showDialog<Revision3NpcDraftPublication>(
            context: context,
            builder: (dialogContext) => Revision3NpcWizardDialog(
              gameRoot: configuredGameRoot,
              loadCatalog: loadNpcCatalog,
              publish: publishNpcDraft,
              chooseArchetype: chooseNpcArchetype,
              copy: l10n.localeName.startsWith('de')
                  ? Revision3NpcWizardCopy.german
                  : Revision3NpcWizardCopy.english,
            ),
          );
          if (!mounted || !context.mounted || publication == null) return null;
          await WidgetsBinding.instance.endOfFrame;
          final checkpoint = currentManagedProject;
          if (checkpoint == null) {
            throw const Revision3NpcDraftStaleCheckpointException();
          }
          return Revision3NpcOpeningRecipeNpcStep(
            publication: publication,
            checkpoint: checkpoint,
          );
        },
        createGreeting: ({required handoff}) =>
            _createNpcOpeningGreeting(context, l10n, handoff),
      );
      if (!mounted || !context.mounted) return;
      await _handleNpcOpeningOutcome(context, l10n, selectionOrigin, outcome);
    } finally {
      if (mounted) setState(() => _npcOpeningRecipeUiBusy = false);
    }
  }

  Future<Revision3NpcOpeningRecipeGreetingStep?> _createNpcOpeningGreeting(
    BuildContext context,
    AppLocalizations l10n,
    Revision3NpcOpeningRecipeHandoff handoff,
  ) async {
    final checkpoint = handoff.npcCheckpoint;
    _requireNpcOpeningCheckpoint(checkpoint);
    final index = await _loadNpcOpeningContent(checkpoint);
    final npc = index.entityById(handoff.npcPublication.npcId);
    if (npc == null ||
        npc.kind != Revision3ContentEntityKind.npcDraft ||
        npc.summary.npcDraft?.greetingCount != 0 ||
        !npc.references.any(
          (reference) =>
              reference.role == 'draft_script_module' &&
              reference.resolution ==
                  Revision3ContentReferenceResolution.resolved &&
              reference.target.projectId == checkpoint.projectId &&
              reference.target.entityId ==
                  handoff.npcPublication.scriptModuleId &&
              reference.target.expectedKind ==
                  Revision3ContentEntityKind.scriptModule,
        )) {
      throw const Revision3NpcGreetingStaleCheckpointException();
    }

    Future<AuthoringRevision3DialogLocalizationReadResult> readExact({
      required String expectedProjectId,
      required int expectedProjectRevision,
      required AuthoringWorkingHead expectedHead,
      required String localizationId,
      required int expectedLocalizationRevision,
      required String expectedLocId,
    }) async {
      if (expectedProjectId != checkpoint.projectId ||
          expectedProjectRevision != checkpoint.projectRevision ||
          expectedHead.canonicalJson != checkpoint.head.canonicalJson) {
        throw const Revision3NpcGreetingStaleCheckpointException();
      }
      _requireNpcOpeningCheckpoint(checkpoint);
      try {
        final loaded = await readDialogLocalization(
          expectedProjectId: expectedProjectId,
          expectedProjectRevision: expectedProjectRevision,
          localizationId: localizationId,
          expectedLocalizationRevision: expectedLocalizationRevision,
          expectedLocId: expectedLocId,
        );
        _requireNpcOpeningCheckpoint(checkpoint);
        return loaded;
      } on Revision3DialogLineEntryRequiresReopenException {
        throw const Revision3NpcGreetingRequiresReopenException();
      } on Revision3DialogLineEntryStaleCheckpointException {
        throw const Revision3NpcGreetingStaleCheckpointException();
      }
    }

    Revision3NpcGreetingPublication? greetingPublication;
    final greetingService = Revision3NpcGreetingAuthoringService(
      expectedHead: checkpoint.head,
      loadContentIndex: () => _loadNpcOpeningContent(checkpoint),
      readExactLocalization: readExact,
      publishReplace:
          ({
            required expectedProjectId,
            required expectedProjectRevision,
            required expectedHead,
            required plan,
          }) {
            _requireNpcOpeningCheckpoint(checkpoint);
            return publishNpcGreetingReplace(
              expectedProjectId: expectedProjectId,
              expectedProjectRevision: expectedProjectRevision,
              expectedHead: expectedHead,
              plan: plan,
            );
          },
      publishCreate:
          ({
            required expectedProjectId,
            required expectedProjectRevision,
            required expectedHead,
            required plan,
          }) async {
            _requireNpcOpeningCheckpoint(checkpoint);
            final publication = await publishNpcGreetingCreate(
              expectedProjectId: expectedProjectId,
              expectedProjectRevision: expectedProjectRevision,
              expectedHead: expectedHead,
              plan: plan,
            );
            greetingPublication = publication;
            return publication;
          },
    );
    final projection = await greetingService.load(
      npcId: npc.id,
      expectedNpcRevision: npc.revision,
    );
    if (projection.rows.isNotEmpty || !mounted || !context.mounted) {
      if (projection.rows.isNotEmpty) {
        throw const Revision3NpcGreetingStaleCheckpointException();
      }
      return null;
    }

    final result = await showDialog<Revision3DialogLineEntryDialogResult>(
      context: context,
      barrierDismissible: false,
      builder: (dialogContext) => Revision3DialogLineEntryDialog(
        service: Revision3DialogLineEntryAuthoringService(
          loadContentIndex: () => _loadNpcOpeningContent(checkpoint),
          readExactLocalization:
              ({
                required expectedProjectId,
                required expectedProjectRevision,
                required localizationId,
                required expectedLocalizationRevision,
                required expectedLocId,
              }) => readExact(
                expectedProjectId: expectedProjectId,
                expectedProjectRevision: expectedProjectRevision,
                expectedHead: checkpoint.head,
                localizationId: localizationId,
                expectedLocalizationRevision: expectedLocalizationRevision,
                expectedLocId: expectedLocId,
              ),
          publishTechnicalPlan: greetingService.createAndInsertPublisher(
            projection: projection,
            index: 0,
          ),
        ),
        copy: _dialogLineEntryCopy(l10n).copyWith(
          title: l10n.managedNpcOpeningGreetingTitle,
          introduction: l10n.managedNpcOpeningGreetingIntroduction,
        ),
        allowOpenVoiceNext: false,
      ),
    );
    if (!mounted || !context.mounted || result == null) return null;
    final publication = greetingPublication;
    if (publication == null ||
        result.publication.projectId != publication.projectId ||
        result.publication.projectRevision != publication.projectRevision ||
        result.publication.lineId != publication.createdLineId ||
        result.publication.localizationId !=
            publication.createdLocalizationId ||
        result.publication.voiceSlotId != publication.createdVoiceSlotId) {
      throw const Revision3NpcGreetingRequiresReopenException();
    }
    await WidgetsBinding.instance.endOfFrame;
    final current = currentManagedProject;
    if (current == null) {
      throw const Revision3NpcGreetingStaleCheckpointException();
    }
    return Revision3NpcOpeningRecipeGreetingStep(
      publication: publication,
      checkpoint: current,
    );
  }

  Future<Revision3ContentIndex> _loadNpcOpeningContent(
    ManagedRevision3CurrentProjectState checkpoint,
  ) async {
    _requireNpcOpeningCheckpoint(checkpoint);
    final index = await loadContentIndex();
    _requireNpcOpeningCheckpoint(checkpoint);
    if (index.projectId != checkpoint.projectId ||
        index.projectRevision != checkpoint.projectRevision) {
      throw const Revision3NpcGreetingStaleCheckpointException();
    }
    return index;
  }

  void _requireNpcOpeningCheckpoint(
    ManagedRevision3CurrentProjectState expected,
  ) {
    final current = currentManagedProject;
    if (current != null &&
        current.root.path == expected.root.path &&
        current.projectId == expected.projectId &&
        current.requiresReopen) {
      throw const Revision3NpcGreetingRequiresReopenException();
    }
    if (current == null ||
        current.root.path != expected.root.path ||
        current.projectId != expected.projectId ||
        current.projectRevision != expected.projectRevision ||
        current.head.canonicalJson != expected.head.canonicalJson) {
      throw const Revision3NpcGreetingStaleCheckpointException();
    }
  }

  Future<void> _handleNpcOpeningOutcome(
    BuildContext context,
    AppLocalizations l10n,
    _ManagedStorySelectionOrigin selectionOrigin,
    Revision3NpcOpeningRecipeOutcome outcome,
  ) async {
    final messenger = ScaffoldMessenger.of(context)..removeCurrentSnackBar();
    switch (outcome) {
      case Revision3NpcOpeningRecipeNoChangeOutcome(:final reason):
        if (reason == Revision3NpcOpeningRecipeNoChangeReason.failed) {
          messenger.showSnackBar(
            SnackBar(content: Text(l10n.managedNpcOpeningRecipeFailed)),
          );
        }
      case Revision3NpcOpeningRecipeNpcOnlyOutcome(:final npcStep):
        messenger.showSnackBar(
          SnackBar(
            content: Text(
              l10n.managedNpcOpeningRecipePartial(
                npcStep.checkpoint.projectRevision,
              ),
            ),
          ),
        );
        await _openNpcOpeningStory(
          context,
          selectionOrigin,
          npcId: npcStep.publication.npcId,
          checkpoint: npcStep.checkpoint,
        );
      case Revision3NpcOpeningRecipeCompletedOutcome(
        :final npcStep,
        :final greetingStep,
      ):
        messenger.showSnackBar(
          SnackBar(
            content: Text(
              l10n.managedNpcOpeningRecipeComplete(
                greetingStep.checkpoint.projectRevision,
              ),
            ),
          ),
        );
        await _openNpcOpeningStory(
          context,
          selectionOrigin,
          npcId: npcStep.publication.npcId,
          checkpoint: greetingStep.checkpoint,
          selectedLineId: greetingStep.publication.createdLineId,
        );
      case Revision3NpcOpeningRecipeLockedOutcome():
        messenger.showSnackBar(
          SnackBar(content: Text(l10n.managedNpcOpeningRecipeStopped)),
        );
        // A locked result may carry a rejected or otherwise unverified NPC
        // receipt. Never consume it for an exact-selection handoff.
        _openCurrentStoryIfPossible(context, selectionOrigin);
      case Revision3NpcOpeningRecipeRequiresReopenOutcome():
        messenger.showSnackBar(
          SnackBar(content: Text(l10n.managedNpcOpeningRecipeRequiresReopen)),
        );
    }
  }

  Future<void> _openNpcOpeningStory(
    BuildContext context,
    _ManagedStorySelectionOrigin origin, {
    required String npcId,
    required ManagedRevision3CurrentProjectState checkpoint,
    String? selectedLineId,
  }) async {
    if (!_isCurrentStorySelectionOrigin(origin) ||
        !_isCurrentNpcOpeningCheckpoint(checkpoint)) {
      return;
    }
    Revision3ProjectWorkspace.navigate(
      context,
      const Revision3ProjectWorkspaceLocation(
        Revision3ProjectWorkspaceSection.story,
      ),
    );
    await WidgetsBinding.instance.endOfFrame;
    if (!mounted ||
        !context.mounted ||
        !_isCurrentStorySelectionOrigin(origin) ||
        !_isCurrentNpcOpeningCheckpoint(checkpoint)) {
      return;
    }
    _selectPublishedStoryEntity(
      origin,
      entityId: npcId,
      projectRevision: checkpoint.projectRevision,
      projectHeadCanonicalJson: checkpoint.head.canonicalJson,
      section: Revision3StoryWorkbenchSection.dialogVoice,
      selectedLineId: selectedLineId,
    );
  }

  bool _isCurrentNpcOpeningCheckpoint(
    ManagedRevision3CurrentProjectState expected,
  ) {
    final current = currentManagedProject;
    return current != null &&
        !current.requiresReopen &&
        current.root.path == expected.root.path &&
        current.projectId == expected.projectId &&
        current.projectRevision == expected.projectRevision &&
        current.head.canonicalJson == expected.head.canonicalJson;
  }

  Future<void> _openQuestOpeningRecipe(BuildContext context) async {
    final configuredGameRoot = gameRoot;
    if (configuredGameRoot == null ||
        !_storyMutationsEnabled ||
        _questOpeningRecipeUiBusy ||
        _questOpeningRecipe.isRunning) {
      return;
    }
    final l10n = AppLocalizations.of(context);
    final openingCheckpoint = project;
    final selectionOrigin = _storySelectionOrigin;
    setState(() => _questOpeningRecipeUiBusy = true);
    try {
      final confirmed =
          await showDialog<bool>(
            context: context,
            builder: (dialogContext) => AlertDialog(
              key: const Key('managed-quest-opening-recipe-intro'),
              scrollable: true,
              title: Text(l10n.managedQuestOpeningRecipeTitle),
              content: Text(l10n.managedQuestOpeningRecipeIntroduction),
              actions: [
                TextButton(
                  key: const Key('managed-quest-opening-recipe-cancel'),
                  onPressed: () => Navigator.of(dialogContext).pop(false),
                  child: Text(l10n.cancel),
                ),
                FilledButton.icon(
                  key: const Key('managed-quest-opening-recipe-start'),
                  onPressed: () => Navigator.of(dialogContext).pop(true),
                  icon: const Icon(Icons.arrow_forward),
                  label: Text(l10n.managedQuestOpeningRecipeStart),
                ),
              ],
            ),
          ) ??
          false;
      if (!mounted || !context.mounted || !confirmed) return;

      final outcome = await _questOpeningRecipe.run(
        openingCheckpoint: openingCheckpoint,
        readCurrentCheckpoint: () async => currentManagedProject,
        createQuest: ({required expectedCheckpoint}) async {
          _requireQuestOpeningCheckpoint(expectedCheckpoint);
          final publication = await showDialog<Revision3QuestDraftPublication>(
            context: context,
            builder: (dialogContext) => Revision3QuestWizardDialog(
              gameRoot: configuredGameRoot,
              loadCatalog: loadQuestCatalog,
              publish: publishQuestDraft,
            ),
          );
          if (!mounted || !context.mounted || publication == null) return null;
          await WidgetsBinding.instance.endOfFrame;
          final checkpoint = currentManagedProject;
          if (checkpoint == null) {
            throw const Revision3QuestDraftStaleCheckpointException();
          }
          return Revision3QuestOpeningRecipeQuestStep(
            publication: publication,
            checkpoint: checkpoint,
          );
        },
        createOpeningLine: ({required handoff}) =>
            _createQuestOpeningLine(context, l10n, handoff),
      );
      if (!mounted || !context.mounted) return;
      await _handleQuestOpeningOutcome(context, l10n, selectionOrigin, outcome);
    } finally {
      if (mounted) setState(() => _questOpeningRecipeUiBusy = false);
    }
  }

  Future<Revision3QuestOpeningRecipeLineStep?> _createQuestOpeningLine(
    BuildContext context,
    AppLocalizations l10n,
    Revision3QuestOpeningRecipeHandoff handoff,
  ) async {
    final checkpoint = handoff.questCheckpoint;
    _requireQuestOpeningCheckpoint(checkpoint);
    final messenger = ScaffoldMessenger.of(context)
      ..removeCurrentSnackBar()
      ..showSnackBar(
        SnackBar(
          content: Text(
            l10n.managedQuestOpeningRecipePreparing(checkpoint.projectRevision),
          ),
        ),
      );
    final index = await _loadQuestOpeningContent(checkpoint);
    final quest = index.entityById(handoff.questPublication.questId);
    if (quest == null ||
        quest.kind != Revision3ContentEntityKind.questDraft ||
        !quest.references.any(
          (reference) =>
              reference.role == 'draft_script_module' &&
              reference.resolution ==
                  Revision3ContentReferenceResolution.resolved &&
              reference.target.projectId == checkpoint.projectId &&
              reference.target.entityId ==
                  handoff.questPublication.scriptModuleId &&
              reference.target.expectedKind ==
                  Revision3ContentEntityKind.scriptModule,
        )) {
      throw const Revision3QuestTranscriptStaleCheckpointException();
    }

    Future<AuthoringRevision3DialogLocalizationReadResult> readExact({
      required String expectedProjectId,
      required int expectedProjectRevision,
      required AuthoringWorkingHead expectedHead,
      required String localizationId,
      required int expectedLocalizationRevision,
      required String expectedLocId,
    }) async {
      if (expectedProjectId != checkpoint.projectId ||
          expectedProjectRevision != checkpoint.projectRevision ||
          expectedHead.canonicalJson != checkpoint.head.canonicalJson) {
        throw const Revision3QuestTranscriptStaleCheckpointException();
      }
      _requireQuestOpeningCheckpoint(checkpoint);
      try {
        final loaded = await readDialogLocalization(
          expectedProjectId: expectedProjectId,
          expectedProjectRevision: expectedProjectRevision,
          localizationId: localizationId,
          expectedLocalizationRevision: expectedLocalizationRevision,
          expectedLocId: expectedLocId,
        );
        _requireQuestOpeningCheckpoint(checkpoint);
        return loaded;
      } on Revision3DialogLineEntryRequiresReopenException {
        throw const Revision3QuestTranscriptRequiresReopenException();
      } on Revision3DialogLineEntryStaleCheckpointException {
        throw const Revision3QuestTranscriptStaleCheckpointException();
      }
    }

    Revision3QuestTranscriptPublication? transcriptPublication;
    final transcriptService = Revision3QuestTranscriptAuthoringService(
      expectedHead: checkpoint.head,
      loadContentIndex: () => _loadQuestOpeningContent(checkpoint),
      readExactLocalization: readExact,
      publishReplace:
          ({
            required expectedProjectId,
            required expectedProjectRevision,
            required expectedHead,
            required plan,
          }) {
            _requireQuestOpeningCheckpoint(checkpoint);
            return publishQuestTranscriptReplace(
              expectedProjectId: expectedProjectId,
              expectedProjectRevision: expectedProjectRevision,
              expectedHead: expectedHead,
              plan: plan,
            );
          },
      publishCreate:
          ({
            required expectedProjectId,
            required expectedProjectRevision,
            required expectedHead,
            required plan,
          }) async {
            _requireQuestOpeningCheckpoint(checkpoint);
            final publication = await publishQuestTranscriptCreate(
              expectedProjectId: expectedProjectId,
              expectedProjectRevision: expectedProjectRevision,
              expectedHead: expectedHead,
              plan: plan,
            );
            transcriptPublication = publication;
            return publication;
          },
    );
    final projection = await transcriptService.load(
      questId: quest.id,
      expectedQuestRevision: quest.revision,
    );
    if (!mounted || !context.mounted) return null;
    messenger.removeCurrentSnackBar();
    final result = await showDialog<Revision3DialogLineEntryDialogResult>(
      context: context,
      barrierDismissible: false,
      builder: (dialogContext) => Revision3DialogLineEntryDialog(
        service: Revision3DialogLineEntryAuthoringService(
          loadContentIndex: () => _loadQuestOpeningContent(checkpoint),
          readExactLocalization:
              ({
                required expectedProjectId,
                required expectedProjectRevision,
                required localizationId,
                required expectedLocalizationRevision,
                required expectedLocId,
              }) => readExact(
                expectedProjectId: expectedProjectId,
                expectedProjectRevision: expectedProjectRevision,
                expectedHead: checkpoint.head,
                localizationId: localizationId,
                expectedLocalizationRevision: expectedLocalizationRevision,
                expectedLocId: expectedLocId,
              ),
          publishTechnicalPlan: transcriptService.createAndInsertPublisher(
            projection: projection,
            index: 0,
            objectiveSlot: null,
          ),
        ),
        copy: _dialogLineEntryCopy(l10n).copyWith(
          title: l10n.managedQuestOpeningLineTitle,
          introduction: l10n.managedQuestOpeningLineIntroduction,
        ),
        allowOpenVoiceNext: false,
      ),
    );
    if (!mounted || !context.mounted || result == null) return null;
    final publication = transcriptPublication;
    if (publication == null ||
        result.publication.projectId != publication.projectId ||
        result.publication.projectRevision != publication.projectRevision ||
        result.publication.lineId != publication.createdLineId ||
        result.publication.localizationId !=
            publication.createdLocalizationId ||
        result.publication.voiceSlotId != publication.createdVoiceSlotId) {
      throw const Revision3QuestTranscriptRequiresReopenException();
    }
    await WidgetsBinding.instance.endOfFrame;
    final current = currentManagedProject;
    if (current == null) {
      throw const Revision3QuestTranscriptStaleCheckpointException();
    }
    return Revision3QuestOpeningRecipeLineStep(
      publication: publication,
      checkpoint: current,
    );
  }

  Future<Revision3ContentIndex> _loadQuestOpeningContent(
    ManagedRevision3CurrentProjectState checkpoint,
  ) async {
    _requireQuestOpeningCheckpoint(checkpoint);
    final index = await loadContentIndex();
    _requireQuestOpeningCheckpoint(checkpoint);
    if (index.projectId != checkpoint.projectId ||
        index.projectRevision != checkpoint.projectRevision) {
      throw const Revision3QuestTranscriptStaleCheckpointException();
    }
    return index;
  }

  void _requireQuestOpeningCheckpoint(
    ManagedRevision3CurrentProjectState expected,
  ) {
    final current = currentManagedProject;
    if (current != null &&
        current.root.path == expected.root.path &&
        current.projectId == expected.projectId &&
        current.requiresReopen) {
      throw const Revision3QuestTranscriptRequiresReopenException();
    }
    if (current == null ||
        current.root.path != expected.root.path ||
        current.projectId != expected.projectId ||
        current.projectRevision != expected.projectRevision ||
        current.head.canonicalJson != expected.head.canonicalJson) {
      throw const Revision3QuestTranscriptStaleCheckpointException();
    }
  }

  Future<void> _handleQuestOpeningOutcome(
    BuildContext context,
    AppLocalizations l10n,
    _ManagedStorySelectionOrigin selectionOrigin,
    Revision3QuestOpeningRecipeOutcome outcome,
  ) async {
    final messenger = ScaffoldMessenger.of(context)..removeCurrentSnackBar();
    switch (outcome) {
      case Revision3QuestOpeningRecipeNoChangeOutcome(:final reason):
        if (reason == Revision3QuestOpeningRecipeNoChangeReason.failed) {
          messenger.showSnackBar(
            SnackBar(content: Text(l10n.managedQuestOpeningRecipeFailed)),
          );
        }
      case Revision3QuestOpeningRecipeQuestOnlyOutcome(:final questStep):
        messenger.showSnackBar(
          SnackBar(
            content: Text(
              l10n.managedQuestOpeningRecipePartial(
                questStep.checkpoint.projectRevision,
              ),
            ),
          ),
        );
        await _openQuestOpeningStory(
          context,
          selectionOrigin,
          questId: questStep.publication.questId,
          checkpoint: questStep.checkpoint,
        );
      case Revision3QuestOpeningRecipeCompletedOutcome(
        :final questStep,
        :final lineStep,
      ):
        messenger.showSnackBar(
          SnackBar(
            content: Text(
              l10n.managedQuestOpeningRecipeComplete(
                lineStep.checkpoint.projectRevision,
              ),
            ),
          ),
        );
        await _openQuestOpeningStory(
          context,
          selectionOrigin,
          questId: questStep.publication.questId,
          checkpoint: lineStep.checkpoint,
          selectedLineId: lineStep.publication.createdLineId,
        );
      case Revision3QuestOpeningRecipeLockedOutcome():
        messenger.showSnackBar(
          SnackBar(content: Text(l10n.managedQuestOpeningRecipeStopped)),
        );
        _openCurrentStoryIfPossible(context, selectionOrigin);
      case Revision3QuestOpeningRecipeRequiresReopenOutcome():
        messenger.showSnackBar(
          SnackBar(content: Text(l10n.managedQuestOpeningRecipeRequiresReopen)),
        );
    }
  }

  Future<void> _openQuestOpeningStory(
    BuildContext context,
    _ManagedStorySelectionOrigin origin, {
    required String questId,
    required ManagedRevision3CurrentProjectState checkpoint,
    String? selectedLineId,
  }) async {
    if (!_isCurrentStorySelectionOrigin(origin) ||
        !_isCurrentQuestOpeningCheckpoint(checkpoint)) {
      return;
    }
    Revision3ProjectWorkspace.navigate(
      context,
      const Revision3ProjectWorkspaceLocation(
        Revision3ProjectWorkspaceSection.story,
      ),
    );
    await WidgetsBinding.instance.endOfFrame;
    if (!mounted ||
        !context.mounted ||
        !_isCurrentStorySelectionOrigin(origin) ||
        !_isCurrentQuestOpeningCheckpoint(checkpoint)) {
      return;
    }
    _selectPublishedStoryEntity(
      origin,
      entityId: questId,
      projectRevision: checkpoint.projectRevision,
      projectHeadCanonicalJson: checkpoint.head.canonicalJson,
      section: Revision3StoryWorkbenchSection.dialogVoice,
      selectedLineId: selectedLineId,
    );
  }

  bool _isCurrentQuestOpeningCheckpoint(
    ManagedRevision3CurrentProjectState expected,
  ) {
    final current = currentManagedProject;
    return current != null &&
        !current.requiresReopen &&
        current.root.path == expected.root.path &&
        current.projectId == expected.projectId &&
        current.projectRevision == expected.projectRevision &&
        current.head.canonicalJson == expected.head.canonicalJson;
  }

  void _openCurrentStoryIfPossible(
    BuildContext context,
    _ManagedStorySelectionOrigin origin,
  ) {
    if (!_isCurrentStorySelectionOrigin(origin)) return;
    Revision3ProjectWorkspace.navigate(
      context,
      const Revision3ProjectWorkspaceLocation(
        Revision3ProjectWorkspaceSection.story,
      ),
    );
  }

  Future<void> _openQuestWizard(
    BuildContext context, {
    String? initialParentCatalogId,
    String? initialGiverCatalogId,
    bool selectPublishedInStory = false,
  }) async {
    final configuredGameRoot = gameRoot;
    if (configuredGameRoot == null || !_storyMutationsEnabled) return;
    final selectionOrigin = _storySelectionOrigin;
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
    if (selectPublishedInStory &&
        publication.projectId == selectionOrigin.projectId &&
        _isCurrentStorySelectionOrigin(selectionOrigin)) {
      _selectPublishedStoryEntity(
        selectionOrigin,
        entityId: publication.questId,
        projectRevision: publication.projectRevision,
      );
    }
  }

  Future<void> _openQuestOutlineEditor(
    BuildContext context,
    Revision3ContentIndex index,
    Revision3ContentEntity quest,
  ) async {
    if (!_storyMutationsEnabled) return;
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
    if (configuredGameRoot == null || !_storyMutationsEnabled) return;
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
    if (!_storyMutationsEnabled) return;
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
        npcTitle: npc.displayName.isEmpty
            ? npc.summary.primaryIdentity
            : npc.displayName,
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

  Future<void> _openNpcProfileEditor(
    BuildContext context,
    Revision3ContentIndex index,
    Revision3ContentEntity npc,
  ) async {
    final configuredGameRoot = gameRoot;
    if (configuredGameRoot == null || !_storyMutationsEnabled) return;
    final l10n = AppLocalizations.of(context);
    final publication = await showDialog<Revision3NpcProfileEditPublication>(
      context: context,
      barrierDismissible: false,
      builder: (context) => Revision3NpcProfileEditDialog(
        index: index,
        npc: npc,
        gameRoot: configuredGameRoot,
        service: Revision3NpcProfileEditAuthoringService(
          loadSeed: loadNpcProfileEditSeed,
          loadCatalog: loadNpcCatalog,
          publishTechnicalPlan: publishNpcProfileEdit,
        ),
        copy: _npcProfileEditDialogCopy(l10n),
      ),
    );
    if (!context.mounted || publication == null) return;
    ScaffoldMessenger.of(context).showSnackBar(
      SnackBar(
        content: Text(
          l10n.managedNpcProfileEditSaved(
            publication.displayName,
            publication.projectRevision,
          ),
        ),
      ),
    );
  }

  Future<DataAssetSemanticStagePublication?> _openInstalledPackageBrowser(
    BuildContext context,
    String configuredGameRoot, {
    String initialQuery = '',
    String? initialTargetPath,
  }) => showDialog<DataAssetSemanticStagePublication>(
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
    bool selectPublishedInStory = false,
  }) async {
    final configuredGameRoot = gameRoot;
    if (configuredGameRoot == null || !_storyMutationsEnabled) return;
    final l10n = AppLocalizations.of(context);
    final selectionOrigin = _storySelectionOrigin;
    final publication = await showDialog<Revision3NpcDraftPublication>(
      context: context,
      builder: (context) => Revision3NpcWizardDialog(
        gameRoot: configuredGameRoot,
        loadCatalog: loadNpcCatalog,
        publish: publishNpcDraft,
        chooseArchetype: chooseNpcArchetype,
        initialCatalogId: initialCatalogId,
        copy: l10n.localeName.startsWith('de')
            ? Revision3NpcWizardCopy.german
            : Revision3NpcWizardCopy.english,
      ),
    );
    if (!context.mounted || publication == null) return;
    await WidgetsBinding.instance.endOfFrame;
    if (!mounted ||
        !context.mounted ||
        !_isCurrentStorySelectionOrigin(selectionOrigin) ||
        !_isCurrentPublishedNpc(selectionOrigin, publication)) {
      return;
    }
    ScaffoldMessenger.of(context).showSnackBar(
      SnackBar(
        content: Text(l10n.managedNpcDraftSaved(publication.projectRevision)),
      ),
    );
    if (selectPublishedInStory) {
      await _openPublishedNpcInStory(context, selectionOrigin, publication);
    }
  }

  Future<void> _openPublishedNpcInStory(
    BuildContext context,
    _ManagedStorySelectionOrigin origin,
    Revision3NpcDraftPublication publication,
  ) async {
    // Let the coordinator publication rebuild this managed-project owner before
    // binding the continuation to its newly published exact checkpoint.
    await WidgetsBinding.instance.endOfFrame;
    if (!mounted ||
        !context.mounted ||
        !_isCurrentStorySelectionOrigin(origin)) {
      return;
    }
    final checkpoint = currentManagedProject;
    if (checkpoint == null ||
        !_isCurrentPublishedNpc(origin, publication) ||
        !_isCurrentQuestOpeningCheckpoint(checkpoint)) {
      return;
    }

    // Story pages are mounted lazily. Navigate first so the controller can
    // attach, reload the exact new index, and resolve the pending deep link.
    Revision3ProjectWorkspace.navigate(
      context,
      const Revision3ProjectWorkspaceLocation(
        Revision3ProjectWorkspaceSection.story,
      ),
    );
    await WidgetsBinding.instance.endOfFrame;
    if (!mounted ||
        !context.mounted ||
        !_isCurrentStorySelectionOrigin(origin) ||
        !_isCurrentPublishedNpc(origin, publication) ||
        !_isCurrentQuestOpeningCheckpoint(checkpoint)) {
      return;
    }
    _selectPublishedStoryEntity(
      origin,
      entityId: publication.npcId,
      projectRevision: publication.projectRevision,
      projectHeadCanonicalJson: publication.head.canonicalJson,
      section: Revision3StoryWorkbenchSection.dialogVoice,
    );
  }

  bool _isCurrentPublishedNpc(
    _ManagedStorySelectionOrigin origin,
    Revision3NpcDraftPublication publication,
  ) {
    final current = currentManagedProject;
    return current != null &&
        !current.requiresReopen &&
        current.root.path == origin.projectRoot &&
        current.projectId == origin.projectId &&
        current.projectId == publication.projectId &&
        current.projectRevision == publication.projectRevision &&
        current.head.canonicalJson == publication.head.canonicalJson;
  }

  void _selectPublishedStoryEntity(
    _ManagedStorySelectionOrigin origin, {
    required String entityId,
    required int projectRevision,
    String? projectHeadCanonicalJson,
    Revision3StoryWorkbenchSection? section,
    String? selectedLineId,
  }) {
    final messenger = ScaffoldMessenger.of(context);
    final staleMessage = AppLocalizations.of(
      context,
    ).managedStoryWorkspacePublishedSelectionStale;
    unawaited(
      origin.controller
          .selectEntityAtRevision(
            entityId: entityId,
            projectRevision: projectRevision,
            projectHeadCanonicalJson: projectHeadCanonicalJson,
            section: section,
            selectedLineId: selectedLineId,
          )
          .then((selected) {
            if (selected || !_isCurrentStorySelectionOrigin(origin)) return;
            messenger.removeCurrentSnackBar();
            messenger.showSnackBar(SnackBar(content: Text(staleMessage)));
          }),
    );
  }

  Future<void> _openLibraryDraftInStory(
    BuildContext context,
    Revision3ContentIndex index,
    Revision3ContentEntity entity,
  ) async {
    final outcome = await _tryOpenDraftInStory(
      context,
      index,
      entity,
      section: Revision3StoryWorkbenchSection.overview,
    );
    if (outcome != _StoryDraftHandoffOutcome.selectionFailed ||
        !context.mounted) {
      return;
    }
    final messenger = ScaffoldMessenger.of(context)..removeCurrentSnackBar();
    messenger.showSnackBar(
      SnackBar(
        content: Text(
          AppLocalizations.of(
            context,
          ).managedStoryWorkspacePublishedSelectionStale,
        ),
      ),
    );
  }

  Future<_StoryDraftHandoffOutcome> _tryOpenDraftInStory(
    BuildContext context,
    Revision3ContentIndex index,
    Revision3ContentEntity entity, {
    required Revision3StoryWorkbenchSection section,
  }) async {
    final origin = _storySelectionOrigin;
    final expected = currentManagedProject;
    if (expected == null) return _StoryDraftHandoffOutcome.stale;

    bool isExactCurrentDraft() =>
        context.mounted &&
        _isCurrentStorySelectionOrigin(origin) &&
        _isCurrentQuestOpeningCheckpoint(expected) &&
        index.projectId == expected.projectId &&
        index.projectRevision == expected.projectRevision &&
        identical(index.entityById(entity.id), entity) &&
        (entity.kind == Revision3ContentEntityKind.questDraft ||
            entity.kind == Revision3ContentEntityKind.npcDraft);

    if (!isExactCurrentDraft()) return _StoryDraftHandoffOutcome.stale;
    Revision3ProjectWorkspace.navigate(
      context,
      const Revision3ProjectWorkspaceLocation(
        Revision3ProjectWorkspaceSection.story,
      ),
    );
    await WidgetsBinding.instance.endOfFrame;
    if (!isExactCurrentDraft()) return _StoryDraftHandoffOutcome.stale;
    final selected = await origin.controller.selectEntityAtRevision(
      entityId: entity.id,
      projectRevision: expected.projectRevision,
      projectHeadCanonicalJson: expected.head.canonicalJson,
      section: section,
    );
    if (selected) return _StoryDraftHandoffOutcome.opened;
    return isExactCurrentDraft()
        ? _StoryDraftHandoffOutcome.selectionFailed
        : _StoryDraftHandoffOutcome.stale;
  }

  void _openStoryExternalEntity(BuildContext context, String entityId) {
    final open = _contentLibraryController.openEntityById(entityId);
    Revision3ProjectWorkspace.navigate(
      context,
      const Revision3ProjectWorkspaceLocation(
        Revision3ProjectWorkspaceSection.content,
      ),
    );
    unawaited(
      open.then((opened) {
        if (opened || !context.mounted) return;
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(
            content: Text(
              AppLocalizations.of(context).managedGlobalSearchResultStale,
            ),
          ),
        );
      }),
    );
  }

  void _openStoryExternalAsset(BuildContext context, String assetSha256) {
    final open = _contentLibraryController.openAssetBySha256(assetSha256);
    Revision3ProjectWorkspace.navigate(
      context,
      const Revision3ProjectWorkspaceLocation(
        Revision3ProjectWorkspaceSection.content,
      ),
    );
    unawaited(
      open.then((opened) {
        if (opened || !context.mounted) return;
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(
            content: Text(
              AppLocalizations.of(context).managedGlobalSearchResultStale,
            ),
          ),
        );
      }),
    );
  }

  Future<void> _openProblemEntity(
    BuildContext context, {
    required String entityId,
    required String expectedProjectRoot,
    required String expectedProjectId,
    required int expectedProjectRevision,
    required String expectedProjectHeadCanonicalJson,
  }) async {
    bool isExactCurrentProject() {
      final current = currentManagedProject;
      return context.mounted &&
          current != null &&
          !current.requiresReopen &&
          current.root.path == expectedProjectRoot &&
          current.projectId == expectedProjectId &&
          current.projectRevision == expectedProjectRevision &&
          current.head.canonicalJson == expectedProjectHeadCanonicalJson;
    }

    if (!isExactCurrentProject()) {
      throw StateError('The exact project entity is no longer available.');
    }
    final index = await loadContentIndex();
    if (!context.mounted) {
      throw StateError('The exact project entity is no longer available.');
    }
    if (!isExactCurrentProject() ||
        index.projectId != expectedProjectId ||
        index.projectRevision != expectedProjectRevision) {
      throw StateError('The exact project entity is no longer available.');
    }
    final entity = index.entityById(entityId);
    if (entity == null) {
      throw StateError('The exact project entity is no longer available.');
    }
    if (entity.kind == Revision3ContentEntityKind.questDraft ||
        entity.kind == Revision3ContentEntityKind.npcDraft) {
      final outcome = await _tryOpenDraftInStory(
        context,
        index,
        entity,
        section: Revision3StoryWorkbenchSection.problemsChecks,
      );
      if (outcome == _StoryDraftHandoffOutcome.opened) return;
      throw StateError('The exact project entity is no longer available.');
    }

    final openProblems = _contentLibraryController
        .openEntityProblemsByIdAtCheckpoint(
          entityId,
          projectRevision: expectedProjectRevision,
          projectHeadCanonicalJson: expectedProjectHeadCanonicalJson,
        );
    Revision3ProjectWorkspace.navigate(
      context,
      const Revision3ProjectWorkspaceLocation(
        Revision3ProjectWorkspaceSection.content,
      ),
    );
    if (await openProblems && isExactCurrentProject()) {
      return;
    }
    if (!isExactCurrentProject()) {
      throw StateError('The exact project entity is no longer available.');
    }
    if (await _contentLibraryController.openEntityByIdAtCheckpoint(
          entityId,
          projectRevision: expectedProjectRevision,
          projectHeadCanonicalJson: expectedProjectHeadCanonicalJson,
        ) &&
        isExactCurrentProject()) {
      return;
    }
    throw StateError('The exact project entity is no longer available.');
  }

  Future<void> _openProblemAsset(
    BuildContext context, {
    required String assetSha256,
    required String expectedProjectRoot,
    required String expectedProjectId,
    required int expectedProjectRevision,
    required String expectedProjectHeadCanonicalJson,
  }) async {
    bool isExactCurrentProject() {
      final current = currentManagedProject;
      return context.mounted &&
          current != null &&
          !current.requiresReopen &&
          current.root.path == expectedProjectRoot &&
          current.projectId == expectedProjectId &&
          current.projectRevision == expectedProjectRevision &&
          current.head.canonicalJson == expectedProjectHeadCanonicalJson;
    }

    if (!isExactCurrentProject()) {
      throw StateError('The exact project asset is no longer available.');
    }
    final openAsset = _contentLibraryController.openAssetBySha256AtCheckpoint(
      assetSha256,
      projectRevision: expectedProjectRevision,
      projectHeadCanonicalJson: expectedProjectHeadCanonicalJson,
    );
    Revision3ProjectWorkspace.navigate(
      context,
      const Revision3ProjectWorkspaceLocation(
        Revision3ProjectWorkspaceSection.content,
      ),
    );
    if (await openAsset && isExactCurrentProject()) return;
    throw StateError('The exact project asset is no longer available.');
  }

  Future<void> _openProblemDataAssetStage(
    BuildContext context, {
    required String targetPath,
    required String expectedProjectRoot,
    required String expectedProjectId,
    required int expectedProjectRevision,
    required String expectedProjectHeadCanonicalJson,
  }) async {
    bool isExactCurrentProject() {
      final current = currentManagedProject;
      return context.mounted &&
          current != null &&
          !current.requiresReopen &&
          current.root.path == expectedProjectRoot &&
          current.projectId == expectedProjectId &&
          current.projectRevision == expectedProjectRevision &&
          current.head.canonicalJson == expectedProjectHeadCanonicalJson;
    }

    if (!isExactCurrentProject()) {
      throw StateError('The exact DataAsset edit is no longer available.');
    }
    final openStage = _dataAssetStagePanelController.openStageByIdAtCheckpoint(
      targetPath,
      projectId: expectedProjectId,
      projectRevision: expectedProjectRevision,
      projectHeadCanonicalJson: expectedProjectHeadCanonicalJson,
    );
    Revision3ProjectWorkspace.navigate(
      context,
      const Revision3ProjectWorkspaceLocation(
        Revision3ProjectWorkspaceSection.content,
        secondary: 'data-assets',
      ),
    );
    if (await openStage && isExactCurrentProject()) return;
    throw StateError('The exact DataAsset edit is no longer available.');
  }

  Future<void> _openSettings(BuildContext context) =>
      _showModStudioSettingsDialog(context);
}

Revision3DialogLineEntryDialogCopy _dialogLineEntryCopy(
  AppLocalizations l10n,
) => Revision3DialogLineEntryDialogCopy(
  title: l10n.managedActionNewDialogLineTitle,
  introduction: l10n.managedDialogLineIntroduction,
  projectOnlyBoundary: l10n.managedDialogLineBoundary,
  createMode: l10n.managedDialogLineCreateMode,
  reuseMode: l10n.managedDialogLineReuseMode,
  lineNameLabel: l10n.managedDialogLineNameLabel,
  lineNameHint: l10n.managedDialogLineNameHint,
  speakerLabel: l10n.managedDialogLineSpeakerLabel,
  speakerHint: l10n.managedDialogLineSpeakerHint,
  localeLabel: l10n.managedDialogLineLocaleLabel,
  textLabel: l10n.managedDialogLineTextLabel,
  reuseSearchLabel: l10n.managedDialogLineReuseSearch,
  noReusableText: l10n.managedDialogLineNoReusableText,
  createVoiceSlotLabel: l10n.managedDialogLineCreateSlotLabel,
  createVoiceSlotHelp: l10n.managedDialogLineCreateSlotHelp,
  cancel: l10n.managedDialogLineCancel,
  save: l10n.managedDialogLineSave,
  saving: l10n.managedDialogLineSaving,
  loading: l10n.managedDialogLineLoading,
  loadFailed: l10n.managedDialogLineLoadFailed,
  retry: l10n.managedDialogLineRetry,
  stale: l10n.managedDialogLineStale,
  requiresReopen: l10n.managedDialogLineRequiresReopen,
  invalidInput: l10n.managedDialogLineInvalidInput,
  saveFailed: l10n.managedDialogLineSaveFailed,
  saved: l10n.managedActionNewDialogLineSaved,
  done: l10n.managedDialogLineDone,
  addRecording: l10n.managedDialogLineAddRecording,
);

Revision3StoryEntityWorkbenchCopy _storyWorkbenchCopy(
  AppLocalizations l10n,
) => Revision3StoryEntityWorkbenchCopy(
  actionFailed: l10n.managedStoryWorkbenchActionFailed,
  draftBadge: l10n.managedStoryWorkbenchDraftBadge,
  buildBlockedBadge: l10n.managedStoryWorkbenchBuildBlockedBadge,
  runtimeUnqualifiedBadge: l10n.managedStoryWorkbenchRuntimeUnqualifiedBadge,
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
  editNpcProfile: l10n.managedStoryWorkbenchEditNpcProfile,
  npcDialogVoiceNextStepTitle:
      l10n.managedStoryWorkbenchNpcDialogVoiceNextStepTitle,
  npcDialogVoiceNextStepDescription:
      l10n.managedStoryWorkbenchNpcDialogVoiceNextStepDescription,
  continueToNpcDialogVoice: l10n.managedStoryWorkbenchContinueToNpcDialogVoice,
  editStory: l10n.managedStoryWorkbenchEditStory,
  editLogic: l10n.managedStoryWorkbenchEditLogic,
  inspectQuest: l10n.managedStoryWorkbenchInspectQuest,
  inspectNpc: l10n.managedStoryWorkbenchInspectNpc,
  moreActions: l10n.managedStoryWorkbenchMoreActions,
  removeDraft: l10n.managedStoryWorkbenchRemoveDraft,
  removingDraft: l10n.managedStoryWorkbenchRemovingDraft,
  reviewRemovalBlockers: l10n.managedStoryWorkbenchReviewRemovalBlockers,
  capabilityUnavailable: l10n.managedStoryWorkbenchCapabilityUnavailable,
  npcStoryUnavailable: l10n.managedStoryWorkbenchNpcStoryUnavailable,
  npcRoutineUnavailable: l10n.managedStoryWorkbenchNpcRoutineUnavailable,
  npcInventoryUnavailable: l10n.managedStoryWorkbenchNpcInventoryUnavailable,
  npcDialogVoiceUnavailable:
      l10n.managedStoryWorkbenchNpcDialogVoiceUnavailable,
  questDialogVoiceUnavailable:
      l10n.managedStoryWorkbenchQuestDialogVoiceUnavailable,
  noReferenceProblems: l10n.managedStoryWorkbenchNoReferenceProblems,
  referenceProblemCount: l10n.managedStoryWorkbenchReferenceProblemCount,
  referenceScopeNotice: l10n.managedStoryWorkbenchReferenceScopeNotice,
  technicalDetails: l10n.managedStoryWorkbenchTechnicalDetails,
  questKindLabel: l10n.managedStoryWorkbenchQuestKindLabel,
  npcKindLabel: l10n.managedStoryWorkbenchNpcKindLabel,
  questTitleLabel: l10n.managedStoryWorkbenchQuestTitleLabel,
  npcDisplayNameLabel: l10n.managedStoryWorkbenchNpcDisplayNameLabel,
  technicalIdLabel: l10n.managedStoryWorkbenchTechnicalIdLabel,
  objectivesLabel: l10n.managedStoryWorkbenchObjectivesLabel,
  uniqueNameLabel: l10n.managedStoryWorkbenchUniqueNameLabel,
  moduleNamespaceLabel: l10n.managedStoryWorkbenchModuleNamespaceLabel,
  questGiverLabel: l10n.managedStoryWorkbenchQuestGiverLabel,
  runtimeParentLabel: l10n.managedStoryWorkbenchRuntimeParentLabel,
  logicDescription: l10n.managedStoryWorkbenchLogicDescription,
  outgoingHeading: l10n.managedStoryWorkbenchOutgoingHeading,
  noOutgoingReferences: l10n.managedStoryWorkbenchNoOutgoingReferences,
  incomingHeading: l10n.managedStoryWorkbenchIncomingHeading,
  noIncomingReferences: l10n.managedStoryWorkbenchNoIncomingReferences,
  semanticIdentityLabel: l10n.managedStoryWorkbenchSemanticIdentityLabel,
  originLabel: l10n.managedStoryWorkbenchOriginLabel,
  entityRevisionLabel: l10n.managedStoryWorkbenchEntityRevisionLabel,
  stableIdLabel: l10n.managedStoryWorkbenchStableIdLabel,
  referenceResolvedLabel: l10n.managedStoryWorkbenchReferenceResolvedLabel,
  referenceUnresolvedLabel: l10n.managedStoryWorkbenchReferenceUnresolvedLabel,
);

Revision3NpcProfileEditDialogCopy _npcProfileEditDialogCopy(
  AppLocalizations l10n,
) => Revision3NpcProfileEditDialogCopy(
  title: l10n.managedNpcProfileEditTitle,
  description: l10n.managedNpcProfileEditDescription,
  nameLabel: l10n.managedNpcProfileEditNameLabel,
  nameHint: l10n.managedNpcProfileEditNameHint,
  archetypeLabel: l10n.managedNpcProfileEditArchetypeLabel,
  archetypeHelp: l10n.managedNpcProfileEditArchetypeHelp,
  boundary: l10n.managedNpcProfileEditBoundary,
  loading: l10n.managedNpcProfileEditLoading,
  cancel: l10n.managedNpcProfileEditCancel,
  close: l10n.managedNpcProfileEditClose,
  save: l10n.managedNpcProfileEditSave,
  saving: l10n.managedNpcProfileEditSaving,
  retry: l10n.managedNpcProfileEditRetry,
  loadFailed: l10n.managedNpcProfileEditLoadFailed,
  catalogChanged: l10n.managedNpcProfileEditCatalogChanged,
  currentArchetypeUnavailable:
      l10n.managedNpcProfileEditCurrentArchetypeUnavailable,
  stale: l10n.managedNpcProfileEditStale,
  requiresReopen: l10n.managedNpcProfileEditRequiresReopen,
  saveFailed: l10n.managedNpcProfileEditSaveFailed,
  nameRequired: l10n.managedNpcProfileEditNameRequired,
  nameTooLong: l10n.managedNpcProfileEditNameTooLong,
  nameControl: l10n.managedNpcProfileEditNameControl,
  reviewSelection: l10n.managedNpcProfileEditReviewSelection,
  discardTitle: l10n.managedNpcProfileEditDiscardTitle,
  discardBody: l10n.managedNpcProfileEditDiscardBody,
  keepEditing: l10n.managedNpcProfileEditKeepEditing,
  discard: l10n.managedNpcProfileEditDiscard,
);

Revision3StoryWorkspaceCopy _storyWorkspaceCopy(AppLocalizations l10n) =>
    Revision3StoryWorkspaceCopy(
      title: l10n.managedWorkspaceStoryLabel,
      loadingLabel: l10n.managedStoryWorkspaceLoading,
      authorityNotice: l10n.managedStoryWorkspaceAuthorityNotice,
      searchHint: l10n.managedStoryWorkspaceSearchHint,
      clearSearchLabel: l10n.managedGlobalSearchClear,
      allFilterLabel: l10n.changesAll,
      npcFilterLabel: l10n.managedBaseGameBrowserFilterNpcs,
      questFilterLabel: l10n.managedBaseGameBrowserFilterQuests,
      createNpcOpeningLabel: l10n.managedStoryWorkspaceCreateNpcOpening,
      createNpcLabel: l10n.managedStoryWorkspaceCreateNpcAdvanced,
      createQuestLabel: l10n.managedActionNewQuestTitle,
      creatingNpcOpeningLabel: l10n.managedStoryWorkspaceCreatingNpcOpening,
      creatingNpcLabel: l10n.managedStoryWorkspaceCreatingNpc,
      creatingQuestLabel: l10n.managedStoryWorkspaceCreatingQuest,
      createQuestOpeningLabel: l10n.managedStoryWorkspaceCreateQuestOpening,
      creatingQuestOpeningLabel: l10n.managedStoryWorkspaceCreatingQuestOpening,
      createAdvancedLabel: l10n.managedStoryWorkspaceCreateAdvanced,
      createQuestAdvancedLabel: l10n.managedStoryWorkspaceCreateQuestAdvanced,
      noStoryDrafts: l10n.managedStoryWorkspaceEmpty,
      noMatchingStoryDrafts: l10n.managedStoryWorkspaceNoMatches,
      selectDraftLabel: l10n.managedStoryWorkspaceSelectDraft,
      retryLabel: l10n.managedDashboardRetry,
      loadErrorTitle: l10n.managedStoryWorkspaceLoadErrorTitle,
      checkpointMismatchError: l10n.managedStoryWorkspaceCheckpointMismatch,
      checkpointSummary: l10n.managedStoryWorkspaceCheckpointSummary,
      loadErrorDetails: (error) =>
          l10n.managedStoryWorkspaceLoadErrorDetails('$error'),
      createErrorDetails: (error) =>
          l10n.managedStoryWorkspaceCreateErrorDetails('$error'),
      detailsSheetLabel: l10n.managedStoryWorkspaceDetailsSheetLabel,
      removeDraftPairUnavailable:
          l10n.managedStoryWorkspaceRemovePairUnavailable,
      removeDraftBusy: l10n.managedStoryWorkspaceRemoveBusy,
      removeDraftBlocked: l10n.managedStoryWorkspaceRemoveBlocked,
      removeDraftDialogTitle: l10n.managedStoryWorkspaceRemoveDialogTitle,
      removeDraftDialogSummary: l10n.managedStoryWorkspaceRemoveDialogSummary,
      removeDraftNoUndo: l10n.managedStoryWorkspaceRemoveNoUndo,
      removeDraftBoundary: l10n.managedStoryWorkspaceRemoveBoundary,
      removeDraftCancel: l10n.managedStoryWorkspaceRemoveCancel,
      removeDraftConfirm: l10n.managedStoryWorkspaceRemoveConfirm,
      removeDraftBlockedTitle: l10n.managedStoryWorkspaceRemoveBlockedTitle,
      removeDraftBlockedDescription:
          l10n.managedStoryWorkspaceRemoveBlockedDescription,
      removeDraftBlockerLabel: l10n.managedStoryWorkspaceRemoveBlockerLabel,
      removeDraftOpenBlocker: l10n.managedStoryWorkspaceRemoveOpenBlocker,
      removeDraftBlockedClose: l10n.managedStoryWorkspaceRemoveBlockedClose,
      removeDraftSucceeded: l10n.managedStoryWorkspaceRemoveSucceeded,
      removeDraftErrorDetails: (error) =>
          l10n.managedStoryWorkspaceRemoveError('$error'),
      workbench: _storyWorkbenchCopy(l10n),
    );

Revision3LocalizationVoiceWorkspaceCopy _localizationVoiceWorkspaceCopy(
  AppLocalizations l10n,
) {
  final usePreciseGlobalVoiceLabels =
      l10n.localeName.startsWith('en') || l10n.localeName.startsWith('de');
  return Revision3LocalizationVoiceWorkspaceCopy(
    title: l10n.managedWorkspaceLocalizationVoiceLabel,
    description: l10n.managedSectionLocalizationVoiceDescription,
    projectTextsLabel: l10n.managedLocalizationProjectTextsLabel,
    searchLabel: l10n.managedLocalizationSearchLabel,
    refreshLabel: l10n.managedLocalizationRefresh,
    newLineLabel: l10n.managedActionNewDialogLineTitle,
    addVoiceLabel: usePreciseGlobalVoiceLabels
        ? l10n.managedLocalizationGlobalAddVoice
        : l10n.managedActionAddVoiceTakeTitle,
    importVoiceFolderLabel: l10n.managedVoiceFolderImportTitle,
    manageVoiceLabel: usePreciseGlobalVoiceLabels
        ? l10n.managedLocalizationGlobalManageVoice
        : l10n.managedActionManageVoiceTakesTitle,
    resolveVoiceLabel: usePreciseGlobalVoiceLabels
        ? l10n.managedLocalizationGlobalResolveVoice
        : l10n.managedActionResolveVoiceTargetTitle,
    loadingLabel: l10n.managedDialogLineLoading,
    emptyTitle: l10n.managedLocalizationEmptyTitle,
    emptyDescription: l10n.managedLocalizationEmptyDescription,
    loadFailedTitle: l10n.managedLocalizationLoadFailed,
    retryLabel: l10n.managedDialogLineRetry,
    selectTextLabel: l10n.managedLocalizationSelectText,
    languagesLabel: l10n.managedLocalizationLanguagesLabel,
    usedByLinesLabel: l10n.managedLocalizationUsedByLines,
    voiceContextTitle: l10n.managedLocalizationVoiceContextTitle,
    voiceSelectLineLabel: l10n.managedLocalizationVoiceSelectLine,
    voiceSetupExistsLabel: l10n.managedLocalizationVoiceSetupExists,
    voiceSetupMissingLabel: l10n.managedLocalizationVoiceSetupMissing,
    noLineLabel: l10n.managedLocalizationNoLine,
    speakerLabel: l10n.managedLocalizationSpeakerLabel,
    addLanguageLabel: l10n.managedLocalizationAddLanguage,
    removeLanguageLabel: l10n.managedLocalizationRemoveLanguage,
    languageCodeLabel: l10n.managedDialogLineLocaleLabel,
    languageCodeHint: l10n.managedLocalizationLanguageHint,
    languageExistsMessage: l10n.managedLocalizationLanguageExists,
    dialogTextLabel: l10n.managedDialogLineTextLabel,
    addLabel: l10n.managedLocalizationAdd,
    cancelLabel: l10n.managedDialogLineCancel,
    saveLabel: l10n.managedDialogLineSave,
    savingLabel: l10n.managedDialogLineSaving,
    savedLabel: l10n.managedLocalizationSaved,
    voiceLockedLabel: l10n.managedLocalizationVoiceLocked,
    voiceSlotRemovalLockedLabel: l10n.managedLocalizationVoiceSlotRemovalLocked,
    minimumLanguageLockedLabel: l10n.managedLocalizationMinimumLanguageLocked,
    sharedTextNotice: l10n.managedLocalizationSharedNotice,
    offlineNotice: l10n.managedLocalizationOfflineNotice,
    unsavedTitle: l10n.managedLocalizationUnsavedTitle,
    unsavedDescription: l10n.managedLocalizationUnsavedDescription,
    discardLabel: l10n.managedLocalizationDiscard,
    keepEditingLabel: l10n.managedLocalizationKeepEditing,
    voiceUnsavedTitle: l10n.managedLocalizationVoiceUnsavedTitle,
    voiceUnsavedDescription: l10n.managedLocalizationVoiceUnsavedDescription,
    discardAndContinueLabel: l10n.managedLocalizationDiscardAndContinue,
    saveAndContinueLabel: l10n.managedLocalizationSaveAndContinue,
    staleMessage: l10n.managedLocalizationStale,
    reopenMessage: l10n.managedLocalizationReopen,
    invalidInputMessage: l10n.managedLocalizationInvalid,
    genericFailureMessage: l10n.managedLocalizationSaveFailed,
    voiceActionFailedMessage: l10n.managedLocalizationVoiceActionFailed,
  );
}

Revision3VoiceBuildReadinessCopy _voiceBuildReadinessCopy(
  AppLocalizations l10n,
) => Revision3VoiceBuildReadinessCopy(
  title: l10n.managedVoiceBuildReadinessTitle,
  refreshTooltip: l10n.managedVoiceBuildReadinessRefresh,
  checkingSemanticsLabel: l10n.managedVoiceBuildReadinessChecking,
  loadError: l10n.managedVoiceBuildReadinessLoadError,
  retryLabel: l10n.managedDashboardRetry,
  readyTitle: l10n.managedVoiceBuildReadinessReadyTitle,
  blockedTitle: l10n.managedVoiceBuildReadinessBlockedTitle,
  readyCount: l10n.managedVoiceBuildReadinessCount,
  blockedBoundary: l10n.managedVoiceBuildReadinessBlockedBoundary,
  buildBundleLabel: l10n.managedVoiceBuildReadinessBuildBundle,
  readyBuildReleaseGuidance:
      l10n.managedVoiceBuildReadinessBuildReleaseGuidance,
  readyConfigureGameGuidance:
      l10n.managedVoiceBuildReadinessConfigureGameGuidance,
  hideBlockersLabel: l10n.managedVoiceBuildReadinessHideBlockers,
  showBlockersLabel: l10n.managedVoiceBuildReadinessShowBlockers,
  workflowOpenFailed: l10n.managedVoiceBuildReadinessWorkflowFailed,
  buildWorkflowOpenFailed: l10n.managedVoiceBuildReadinessBuildWorkflowFailed,
  exactProjectRevision: l10n.managedVoiceBuildReadinessExactRevision,
  resolveTargetLabel: l10n.managedVoiceBuildReadinessResolveTarget,
  manageTakesLabel: l10n.managedVoiceBuildReadinessManageTakes,
  blockerTitle: (reason) => switch (reason) {
    AuthoringRevision3VoiceBuildBlockReason.noVoiceSlots =>
      l10n.managedVoiceBuildBlockerNoSlots,
    AuthoringRevision3VoiceBuildBlockReason.voicePayloadBudgetExceeded =>
      l10n.managedVoiceBuildBlockerPayloadBudget,
    AuthoringRevision3VoiceBuildBlockReason.unresolvedTarget =>
      l10n.managedVoiceBuildBlockerUnresolvedTarget,
    AuthoringRevision3VoiceBuildBlockReason.ambiguousTarget =>
      l10n.managedVoiceBuildBlockerAmbiguousTarget,
    AuthoringRevision3VoiceBuildBlockReason.unqualifiedAdd =>
      l10n.managedVoiceBuildBlockerUnqualifiedAdd,
    AuthoringRevision3VoiceBuildBlockReason.missingSelectedTake =>
      l10n.managedVoiceBuildBlockerMissingTake,
    AuthoringRevision3VoiceBuildBlockReason.selectedTakeNotApproved =>
      l10n.managedVoiceBuildBlockerTakeNotApproved,
    AuthoringRevision3VoiceBuildBlockReason.selectedTakeCodecUnqualified =>
      l10n.managedVoiceBuildBlockerCodecUnqualified,
    AuthoringRevision3VoiceBuildBlockReason.voiceSlotLimitExceeded =>
      l10n.managedVoiceBuildBlockerSlotLimit,
  },
);

Revision3VoiceBuildDialogCopy _voiceBuildDialogCopy(AppLocalizations l10n) =>
    Revision3VoiceBuildDialogCopy(
      readiness: _voiceBuildReadinessCopy(l10n),
      title: l10n.managedActionBuildVoiceBundleTitle,
      offlineNotice: l10n.managedVoiceBuildOfflineNotice,
      newFolderNameLabel: l10n.managedVoiceBuildNewFolderName,
      newFolderNameHelp: l10n.managedVoiceBuildNewFolderHelp,
      chooseParentFolderLabel: l10n.managedVoiceBuildChooseParent,
      noParentFolderSelected: l10n.managedVoiceBuildNoParentSelected,
      newOutputLabel: l10n.managedVoiceBuildNewOutput,
      cancelLabel: l10n.cancel,
      closeLabel: l10n.close,
      buildOfflineBundleLabel: l10n.managedVoiceBuildOfflineBundle,
      parentInspectFailed: l10n.managedVoiceBuildParentInspectFailed,
      chooseExistingParent: l10n.managedVoiceBuildChooseExistingParent,
      targetSymlink: l10n.managedVoiceBuildTargetSymlink,
      targetExists: l10n.managedVoiceBuildTargetExists,
      requiresReopen: l10n.managedVoiceBuildRequiresReopen,
      staleCheckpoint: l10n.managedVoiceBuildStaleCheckpoint,
      buildFailed: l10n.managedVoiceBuildFailed,
      planRequiresReopen: l10n.managedVoiceBuildRequiresReopen,
      planStaleCheckpoint: l10n.managedVoiceBuildStaleCheckpoint,
      planFailed: l10n.managedVoiceBuildPlanFailed,
      parentMustBeAbsolute: l10n.managedVoiceBuildParentAbsolute,
      parentSymlink: l10n.managedVoiceBuildParentSymlink,
      parentMustExist: l10n.managedVoiceBuildChooseExistingParent,
      folderNameRequired: l10n.managedVoiceBuildFolderRequired,
      folderNameWhitespace: l10n.managedVoiceBuildFolderWhitespace,
      folderNameTooLong: l10n.managedVoiceBuildFolderTooLong,
      folderNamePortable: l10n.managedVoiceBuildFolderPortable,
      folderNameWindowsReserved: l10n.managedVoiceBuildFolderWindowsReserved,
      executableUnavailable: l10n.managedVoiceBuildExecutableUnavailable,
      executableMismatch: l10n.managedVoiceBuildExecutableMismatch,
      gameUnavailable: l10n.managedVoiceBuildGameUnavailable,
      storeGameAlias: l10n.managedVoiceBuildStoreGameAlias,
      gameOutputAlias: l10n.managedVoiceBuildGameOutputAlias,
      storeOutputAlias: l10n.managedVoiceBuildStoreOutputAlias,
      outputUnavailable: l10n.managedVoiceBuildOutputUnavailable,
      outputFailed: l10n.managedVoiceBuildOutputFailed,
      promotionFailed: l10n.managedVoiceBuildPromotionFailed,
      cleanupFailed: l10n.managedVoiceBuildCleanupFailed,
      publicationUnconfirmed: l10n.managedVoiceBuildPublicationUnconfirmed,
      storeRootChanged: l10n.managedVoiceBuildStoreRootChanged,
      gameRootChanged: l10n.managedVoiceBuildGameRootChanged,
      outputRootChanged: l10n.managedVoiceBuildOutputRootChanged,
      verifyFailed: l10n.managedVoiceBuildVerifyFailed,
      bundleInvalid: l10n.managedVoiceBuildBundleInvalid,
      inputInvalid: l10n.managedVoiceBuildInputInvalid,
      responseLimit: l10n.managedVoiceBuildResponseLimit,
      builtTitle: l10n.managedVoiceBuildBuiltTitle,
      offlineReceipt: l10n.managedVoiceBuildOfflineReceipt,
      basisRevisionLabel: l10n.managedVoiceBuildBasisRevision,
      outputLabel: l10n.managedVoiceBuildOutputLabel,
      archiveEditsLabel: l10n.managedVoiceBuildArchiveEdits,
      bundleFilesLabel: l10n.managedVoiceBuildBundleFiles,
      sealedBytesLabel: l10n.managedVoiceBuildSealedBytes,
      bundleSha256Label: l10n.managedVoiceBuildBundleSha256,
    );

bool _hasIntactVoiceLine(Revision3ContentIndex index) {
  try {
    return Revision3VoiceCatalog.fromContentIndex(index).lines.isNotEmpty;
  } catch (_) {
    return false;
  }
}

enum _ManagedVoiceCatalogGateStatus { loading, loaded, unavailable }

typedef _ManagedVoiceCatalogGateBuilder =
    Widget Function(
      BuildContext context,
      ({
        _ManagedVoiceCatalogGateStatus status,
        bool hasIntactVoiceLine,
        Revision3ContentIndexLoader loadContentIndex,
      })
      availability,
    );

/// Resolves the Voice prerequisite from the exact-current content projection.
///
/// This deliberately fails closed: stale, mismatched, or unavailable content
/// must never leave an author-facing Voice action enabled just because an
/// earlier project revision contained an intact Voice-authorable line.
class _ManagedRevision3VoiceCatalogGate extends StatefulWidget {
  const _ManagedRevision3VoiceCatalogGate({
    required this.projectId,
    required this.projectRevision,
    required this.projectHeadCanonicalJson,
    required this.load,
    required this.builder,
  });

  final String projectId;
  final int projectRevision;
  final String projectHeadCanonicalJson;
  final Revision3ContentIndexLoader load;
  final _ManagedVoiceCatalogGateBuilder builder;

  @override
  State<_ManagedRevision3VoiceCatalogGate> createState() =>
      _ManagedRevision3VoiceCatalogGateState();
}

class _ManagedRevision3VoiceCatalogGateState
    extends State<_ManagedRevision3VoiceCatalogGate> {
  _ManagedVoiceCatalogGateStatus _status =
      _ManagedVoiceCatalogGateStatus.loading;
  bool _intactVoiceLineAvailable = false;
  Future<Revision3ContentIndex>? _currentLoad;
  int _loadEpoch = 0;

  @override
  void initState() {
    super.initState();
    _reload(notify: false);
  }

  @override
  void didUpdateWidget(covariant _ManagedRevision3VoiceCatalogGate oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.projectId == widget.projectId &&
        oldWidget.projectRevision == widget.projectRevision &&
        oldWidget.projectHeadCanonicalJson == widget.projectHeadCanonicalJson) {
      return;
    }
    _reload(notify: false);
  }

  @override
  void dispose() {
    _loadEpoch++;
    super.dispose();
  }

  void _reload({required bool notify}) {
    final epoch = ++_loadEpoch;
    final expectedProjectId = widget.projectId;
    final expectedProjectRevision = widget.projectRevision;
    final loader = widget.load;
    final currentLoad = Future<Revision3ContentIndex>.sync(loader);
    _currentLoad = currentLoad;

    void markLoading() {
      _status = _ManagedVoiceCatalogGateStatus.loading;
      _intactVoiceLineAvailable = false;
    }

    if (notify) {
      setState(markLoading);
    } else {
      markLoading();
    }

    unawaited(() async {
      try {
        final index = await currentLoad;
        if (!mounted || epoch != _loadEpoch) return;
        if (index.projectId != expectedProjectId ||
            index.projectRevision != expectedProjectRevision) {
          setState(() {
            _status = _ManagedVoiceCatalogGateStatus.unavailable;
            _intactVoiceLineAvailable = false;
          });
          return;
        }
        setState(() {
          _status = _ManagedVoiceCatalogGateStatus.loaded;
          _intactVoiceLineAvailable = _hasIntactVoiceLine(index);
        });
      } catch (_) {
        if (!mounted || epoch != _loadEpoch) return;
        setState(() {
          _status = _ManagedVoiceCatalogGateStatus.unavailable;
          _intactVoiceLineAvailable = false;
        });
      }
    }());
  }

  Future<Revision3ContentIndex> _loadExactCurrent() {
    if (_status == _ManagedVoiceCatalogGateStatus.unavailable) {
      _reload(notify: true);
    }
    return _currentLoad ??
        Future<Revision3ContentIndex>.error(
          StateError('managed Voice content load is unavailable'),
        );
  }

  @override
  Widget build(BuildContext context) => widget.builder(context, (
    status: _status,
    hasIntactVoiceLine: _intactVoiceLineAvailable,
    loadContentIndex: _loadExactCurrent,
  ));
}

Revision3ProjectProblemsCopy _projectProblemsCopy(AppLocalizations l10n) =>
    Revision3ProjectProblemsCopy(
      title: l10n.managedProblemsTitle,
      description: l10n.managedProblemsDescription,
      scopeNotice: l10n.managedProblemsScopeNotice,
      refreshTooltip: l10n.managedProblemsRefresh,
      loadingSemanticsLabel: l10n.managedDashboardLoading,
      loadErrorSemanticsLabel: l10n.managedDashboardLoadError,
      loadErrorTitle: l10n.managedDashboardLoadError,
      loadErrorDescription: l10n.managedDashboardLoadErrorDescription,
      retryLabel: l10n.managedDashboardRetry,
      partialTitle: l10n.managedProblemsPartialTitle,
      dataAssetsUnavailableDescription:
          l10n.managedProblemsDataAssetsUnavailable,
      overviewHeading: l10n.managedProblemsOverviewHeading,
      scopeTitle: (scope) => _projectProblemScopeTitle(l10n, scope),
      scopeDescription: (scope) => _projectProblemScopeDescription(l10n, scope),
      readinessName: (readiness) =>
          _projectProblemReadinessName(l10n, readiness),
      evidenceName: (evidence) => _projectProblemEvidenceName(l10n, evidence),
      problemTitle: (problem) => _projectProblemTitle(l10n, problem),
      problemDescription: (problem) =>
          _projectProblemDescription(l10n, problem),
      categoryName: (category) => _projectProblemCategoryName(l10n, category),
      severityName: (severity) => _projectProblemSeverityName(l10n, severity),
      searchLabel: l10n.managedProblemsSearchLabel,
      clearSearchTooltip: l10n.managedProblemsClearSearch,
      filterAllLabel: l10n.changesAll,
      listHeading: l10n.managedProblemsListHeading,
      emptyTitle: l10n.managedProblemsEmptyTitle,
      emptyDescription: l10n.managedProblemsEmptyDescription,
      emptyBoundaryDescription: l10n.managedProblemsEmptyBoundary,
      filteredEmptyTitle: l10n.managedProblemsFilteredEmptyTitle,
      filteredEmptyDescription: l10n.managedProblemsFilteredEmptyDescription,
      selectProblemTitle: l10n.managedProblemsSelectTitle,
      selectProblemDescription: l10n.managedProblemsSelectDescription,
      detailHeading: l10n.managedProblemsDetailHeading,
      closeDetailTooltip: l10n.managedProblemsCloseDetail,
      categoryLabel: l10n.managedProblemsCategoryLabel,
      severityLabel: l10n.managedProblemsSeverityLabel,
      sourceLabel: l10n.managedProblemsSourceLabel,
      openEntityLabel: l10n.managedProblemsOpenSourceEntity,
      openAssetLabel: l10n.managedProblemsOpenReferencedAsset,
      openDataAssetStageLabel: l10n.managedProblemsOpenDataAssetEdits,
      openSettingsLabel: l10n.managedActionSettingsTitle,
      verifyCurrentProjectLabel: l10n.projectVerifyCurrentHead,
      actionFailedMessage: l10n.managedProblemsActionFailed,
      actionInProgressSemanticsLabel: l10n.managedProblemsActionProgress,
    );

String _projectProblemCategoryName(
  AppLocalizations l10n,
  Revision3ProjectProblemCategory category,
) => switch (category) {
  Revision3ProjectProblemCategory.references =>
    l10n.managedProblemsCategoryReferences,
  Revision3ProjectProblemCategory.setup => l10n.managedProblemsCategorySetup,
  Revision3ProjectProblemCategory.dataAssets =>
    l10n.managedProblemsCategoryDataAssets,
};

String _projectProblemSeverityName(
  AppLocalizations l10n,
  Revision3ProjectProblemSeverity severity,
) => switch (severity) {
  Revision3ProjectProblemSeverity.information =>
    l10n.managedProblemsSeverityInformation,
  Revision3ProjectProblemSeverity.warning =>
    l10n.managedProblemsSeverityWarning,
  Revision3ProjectProblemSeverity.blocking =>
    l10n.managedProblemsSeverityBlocking,
};

String _projectProblemScopeTitle(
  AppLocalizations l10n,
  Revision3ProjectProblemScope scope,
) => switch (scope) {
  Revision3ProjectProblemScope.referenceIntegrity =>
    l10n.managedProblemsScopeReferencesTitle,
  Revision3ProjectProblemScope.dataAssetRegistry =>
    l10n.managedProblemsScopeDataAssetsTitle,
  Revision3ProjectProblemScope.gameConfiguration =>
    l10n.managedProblemsScopeGameTitle,
  Revision3ProjectProblemScope.compilerEvidence =>
    l10n.managedProblemsScopeCompilerTitle,
  Revision3ProjectProblemScope.managedBuild =>
    l10n.managedProblemsScopeBuildTitle,
  Revision3ProjectProblemScope.runtime => l10n.managedProblemsScopeRuntimeTitle,
};

String _projectProblemScopeDescription(
  AppLocalizations l10n,
  Revision3ProjectProblemScope scope,
) => switch (scope) {
  Revision3ProjectProblemScope.referenceIntegrity =>
    l10n.managedProblemsScopeReferencesDescription,
  Revision3ProjectProblemScope.dataAssetRegistry =>
    l10n.managedProblemsScopeDataAssetsDescription,
  Revision3ProjectProblemScope.gameConfiguration =>
    l10n.managedProblemsScopeGameDescription,
  Revision3ProjectProblemScope.compilerEvidence =>
    l10n.managedProblemsScopeCompilerDescription,
  Revision3ProjectProblemScope.managedBuild =>
    l10n.managedProblemsScopeBuildDescription,
  Revision3ProjectProblemScope.runtime =>
    l10n.managedProblemsScopeRuntimeDescription,
};

String _projectProblemReadinessName(
  AppLocalizations l10n,
  Revision3ProjectProblemReadiness readiness,
) => switch (readiness) {
  Revision3ProjectProblemReadiness.clear => l10n.managedProblemsReadinessClear,
  Revision3ProjectProblemReadiness.issues =>
    l10n.managedProblemsReadinessIssues,
  Revision3ProjectProblemReadiness.unavailable =>
    l10n.managedProblemsReadinessUnavailable,
  Revision3ProjectProblemReadiness.notEvaluated =>
    l10n.managedProblemsReadinessNotEvaluated,
  Revision3ProjectProblemReadiness.blocked =>
    l10n.managedProblemsReadinessBlocked,
  Revision3ProjectProblemReadiness.unqualified =>
    l10n.managedProblemsReadinessUnqualified,
};

String _projectProblemEvidenceName(
  AppLocalizations l10n,
  Revision3ProjectProblemEvidence evidence,
) => switch (evidence) {
  Revision3ProjectProblemEvidence.exactContentIndex =>
    l10n.managedProblemsEvidenceContent,
  Revision3ProjectProblemEvidence.exactDataAssetRegistry =>
    l10n.managedProblemsEvidenceDataAssets,
  Revision3ProjectProblemEvidence.configurationState =>
    l10n.managedProblemsEvidenceConfiguration,
  Revision3ProjectProblemEvidence.sourceUnavailable =>
    l10n.managedProblemsEvidenceUnavailable,
  Revision3ProjectProblemEvidence.capabilityBoundary =>
    l10n.managedProblemsEvidenceBoundary,
};

String _projectProblemTitle(
  AppLocalizations l10n,
  Revision3ProjectProblem problem,
) => switch (problem.code) {
  Revision3ProjectProblemCode.foreignEntityReference =>
    l10n.managedProblemsForeignReferenceTitle,
  Revision3ProjectProblemCode.missingEntityReference =>
    l10n.managedProblemsMissingEntityTitle,
  Revision3ProjectProblemCode.entityKindMismatch =>
    l10n.managedProblemsEntityKindTitle,
  Revision3ProjectProblemCode.missingAssetReference =>
    l10n.managedProblemsMissingAssetTitle,
  Revision3ProjectProblemCode.assetByteLengthMismatch =>
    l10n.managedProblemsAssetLengthTitle,
  Revision3ProjectProblemCode.assetMediaTypeMismatch =>
    l10n.managedProblemsAssetTypeTitle,
  Revision3ProjectProblemCode.gameNotConfigured =>
    l10n.managedProblemsGameSetupTitle,
  Revision3ProjectProblemCode.dataAssetRegistryUnavailable =>
    l10n.managedProblemsDataAssetRegistryTitle,
  Revision3ProjectProblemCode.dataAssetStageOfflineOnly =>
    l10n.managedProblemsDataAssetOfflineTitle,
};

String _projectProblemDescription(
  AppLocalizations l10n,
  Revision3ProjectProblem problem,
) {
  final details = problem.details;
  if (details is Revision3EntityReferenceProblemDetails) {
    final source = details.sourceDisplayName.trim().isEmpty
        ? l10n.managedProblemsCategoryReferences
        : details.sourceDisplayName;
    return l10n.managedProblemsEntityReferenceDescription(source);
  }
  if (details is Revision3AssetReferenceProblemDetails) {
    final source = details.sourceDisplayName.trim().isEmpty
        ? l10n.managedProblemsCategoryReferences
        : details.sourceDisplayName;
    return l10n.managedProblemsAssetReferenceDescription(source);
  }
  if (details is Revision3DataAssetStageProblemDetails) {
    return l10n.managedProblemsDataAssetOfflineDescription(details.targetPath);
  }
  return switch (problem.code) {
    Revision3ProjectProblemCode.gameNotConfigured =>
      l10n.managedDashboardMissingGameDescription,
    Revision3ProjectProblemCode.dataAssetRegistryUnavailable =>
      l10n.managedProblemsDataAssetRegistryDescription,
    Revision3ProjectProblemCode.dataAssetStageOfflineOnly =>
      l10n.managedProblemsScopeNotice,
    _ => l10n.managedProblemsScopeNotice,
  };
}

class _ManagedRevision3GlobalContentHost extends StatefulWidget {
  const _ManagedRevision3GlobalContentHost({
    required this.contentLibraryController,
    required this.sourceIdentity,
    required this.loadThisMod,
    required this.loadBaseGame,
    required this.loadInstalled,
    required this.builder,
  });

  final Revision3ContentLibraryController contentLibraryController;
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
    widget.contentLibraryController,
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
