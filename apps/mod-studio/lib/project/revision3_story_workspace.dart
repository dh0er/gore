import 'dart:async';

import 'package:flutter/material.dart';

import 'revision3_content_index.dart';
import 'revision3_story_entity_workbench.dart';

typedef Revision3StoryWorkspaceLoader =
    Future<Revision3ContentIndex> Function();
typedef Revision3StoryWorkspaceCreateAction = Future<void> Function();
typedef Revision3StoryWorkspaceEntityAction =
    Future<void> Function(
      Revision3ContentIndex index,
      Revision3ContentEntity entity,
    );
typedef Revision3StoryWorkspaceRemoveDraftAction =
    Future<void> Function({
      required Revision3ContentIndex index,
      required Revision3ContentEntity draft,
      required Revision3ContentEntity scriptModule,
    });
typedef Revision3StoryQuestTranscriptBuilder =
    Widget Function({
      required Revision3ContentIndex index,
      required Revision3ContentEntity quest,
      required String? selectedLineId,
      required ValueChanged<String?> onSelectedLineChanged,
    });
typedef Revision3StoryNpcDialogVoiceBuilder =
    Widget Function({
      required Revision3ContentIndex index,
      required Revision3ContentEntity npc,
      required String? selectedLineId,
      required ValueChanged<String?> onSelectedLineChanged,
    });
typedef Revision3StoryQuestJourneyBuilder =
    Widget Function({
      required Revision3ContentIndex index,
      required Revision3ContentEntity quest,
      required ValueChanged<String> onOpenDialogLine,
    });

@immutable
final class Revision3StoryDraftRemovalBlocker {
  const Revision3StoryDraftRemovalBlocker({
    required this.source,
    required this.reference,
  });

  final Revision3ContentEntity source;
  final Revision3ContentReference reference;
}

/// Exact UI preflight for the bounded Draft + generated ScriptModule removal.
///
/// This is deliberately advisory. The native prepare operation repeats the
/// ownership-closure proof against the exact current project head before any
/// publication can occur.
@immutable
final class Revision3StoryDraftRemovalPreflight {
  const Revision3StoryDraftRemovalPreflight._({
    required this.draft,
    required this.scriptModule,
    required this.blockers,
  });

  final Revision3ContentEntity draft;
  final Revision3ContentEntity? scriptModule;
  final List<Revision3StoryDraftRemovalBlocker> blockers;

  bool get hasExactPair => scriptModule != null;
  bool get canRemove => scriptModule != null && blockers.isEmpty;

  factory Revision3StoryDraftRemovalPreflight.fromIndex({
    required Revision3ContentIndex index,
    required Revision3ContentEntity draft,
  }) {
    if (draft.kind != Revision3ContentEntityKind.npcDraft &&
        draft.kind != Revision3ContentEntityKind.questDraft) {
      return Revision3StoryDraftRemovalPreflight._(
        draft: draft,
        scriptModule: null,
        blockers: const <Revision3StoryDraftRemovalBlocker>[],
      );
    }

    final localModuleEdges = draft.references
        .where(
          (reference) =>
              reference.role == 'draft_script_module' &&
              reference.qualifier == null &&
              reference.target.projectId == index.projectId &&
              reference.target.expectedKind ==
                  Revision3ContentEntityKind.scriptModule &&
              reference.resolution ==
                  Revision3ContentReferenceResolution.resolved,
        )
        .toList(growable: false);
    if (localModuleEdges.length != 1) {
      return Revision3StoryDraftRemovalPreflight._(
        draft: draft,
        scriptModule: null,
        blockers: const <Revision3StoryDraftRemovalBlocker>[],
      );
    }

    final draftModuleEdge = localModuleEdges.single;
    final module = index.entityById(draftModuleEdge.target.entityId);
    if (module == null ||
        module.kind != Revision3ContentEntityKind.scriptModule) {
      return Revision3StoryDraftRemovalPreflight._(
        draft: draft,
        scriptModule: null,
        blockers: const <Revision3StoryDraftRemovalBlocker>[],
      );
    }

    bool isExactOwnerReference(
      Revision3ContentReference reference,
      String role,
    ) =>
        reference.role == role &&
        reference.qualifier == null &&
        reference.target.projectId == index.projectId &&
        reference.target.entityId == draft.id &&
        reference.target.expectedKind == draft.kind &&
        reference.resolution == Revision3ContentReferenceResolution.resolved;

    final exactOriginOwnerEdges = module.references
        .where((reference) => isExactOwnerReference(reference, 'origin_owner'))
        .toList(growable: false);
    final exactScriptOwnerEdges = module.references
        .where((reference) => isExactOwnerReference(reference, 'script_owner'))
        .toList(growable: false);
    final generatedOwner = module.origin.generatedOwner;
    if (module.origin.type != 'generated' ||
        generatedOwner == null ||
        generatedOwner.projectId != index.projectId ||
        generatedOwner.entityId != draft.id ||
        generatedOwner.expectedKind != draft.kind ||
        exactOriginOwnerEdges.length != 1 ||
        exactScriptOwnerEdges.length != 1) {
      return Revision3StoryDraftRemovalPreflight._(
        draft: draft,
        scriptModule: null,
        blockers: const <Revision3StoryDraftRemovalBlocker>[],
      );
    }

    final allowedEdges = <Revision3ContentReference>{
      draftModuleEdge,
      exactOriginOwnerEdges.single,
      exactScriptOwnerEdges.single,
    };
    final pairIds = <String>{draft.id, module.id};
    final blockers = <Revision3StoryDraftRemovalBlocker>[];
    for (final source in index.entities) {
      for (final reference in source.references) {
        final sourceIsRemoved = pairIds.contains(source.id);
        final targetIsRemoved = pairIds.contains(reference.target.entityId);
        final ownedTranscriptLineDisappearsWithQuest =
            draft.kind == Revision3ContentEntityKind.questDraft &&
            source.id == draft.id &&
            reference.role == 'quest_transcript_line' &&
            reference.target.projectId == index.projectId &&
            reference.target.expectedKind ==
                Revision3ContentEntityKind.dialogLine &&
            reference.resolution ==
                Revision3ContentReferenceResolution.resolved &&
            index.entityById(reference.target.entityId)?.kind ==
                Revision3ContentEntityKind.dialogLine;
        if (reference.target.projectId != index.projectId ||
            (!sourceIsRemoved && !targetIsRemoved) ||
            ownedTranscriptLineDisappearsWithQuest ||
            allowedEdges.contains(reference)) {
          continue;
        }
        blockers.add(
          Revision3StoryDraftRemovalBlocker(
            source: source,
            reference: reference,
          ),
        );
      }
    }
    return Revision3StoryDraftRemovalPreflight._(
      draft: draft,
      scriptModule: module,
      blockers: List<Revision3StoryDraftRemovalBlocker>.unmodifiable(blockers),
    );
  }
}

/// Localized, author-facing copy for the direct managed-R3 Story workspace.
///
/// The workspace deliberately owns no fallback strings: its embedding surface
/// supplies every user-visible claim and the Workbench copy as one unit.
@immutable
final class Revision3StoryWorkspaceCopy {
  const Revision3StoryWorkspaceCopy({
    required this.title,
    required this.loadingLabel,
    required this.authorityNotice,
    required this.searchHint,
    required this.clearSearchLabel,
    required this.allFilterLabel,
    required this.npcFilterLabel,
    required this.questFilterLabel,
    required this.createNpcOpeningLabel,
    required this.createNpcLabel,
    required this.createQuestLabel,
    required this.creatingNpcOpeningLabel,
    required this.creatingNpcLabel,
    required this.creatingQuestLabel,
    required this.createQuestOpeningLabel,
    required this.creatingQuestOpeningLabel,
    required this.createAdvancedLabel,
    required this.createQuestAdvancedLabel,
    required this.noStoryDrafts,
    required this.noMatchingStoryDrafts,
    required this.selectDraftLabel,
    required this.retryLabel,
    required this.loadErrorTitle,
    required this.checkpointMismatchError,
    required this.checkpointSummary,
    required this.loadErrorDetails,
    required this.createErrorDetails,
    required this.detailsSheetLabel,
    required this.removeDraftPairUnavailable,
    required this.removeDraftBusy,
    required this.removeDraftBlocked,
    required this.removeDraftDialogTitle,
    required this.removeDraftDialogSummary,
    required this.removeDraftNoUndo,
    required this.removeDraftBoundary,
    required this.removeDraftCancel,
    required this.removeDraftConfirm,
    required this.removeDraftBlockedTitle,
    required this.removeDraftBlockedDescription,
    required this.removeDraftBlockerLabel,
    required this.removeDraftOpenBlocker,
    required this.removeDraftBlockedClose,
    required this.removeDraftSucceeded,
    required this.removeDraftErrorDetails,
    required this.workbench,
  });

  final String title;
  final String loadingLabel;
  final String authorityNotice;
  final String searchHint;
  final String clearSearchLabel;
  final String allFilterLabel;
  final String npcFilterLabel;
  final String questFilterLabel;
  final String createNpcOpeningLabel;
  final String createNpcLabel;
  final String createQuestLabel;
  final String creatingNpcOpeningLabel;
  final String creatingNpcLabel;
  final String creatingQuestLabel;
  final String createQuestOpeningLabel;
  final String creatingQuestOpeningLabel;
  final String createAdvancedLabel;
  final String createQuestAdvancedLabel;
  final String noStoryDrafts;
  final String noMatchingStoryDrafts;
  final String selectDraftLabel;
  final String retryLabel;
  final String loadErrorTitle;
  final String checkpointMismatchError;
  final String Function(int count, int projectRevision) checkpointSummary;
  final String Function(Object error) loadErrorDetails;
  final String Function(Object error) createErrorDetails;
  final String Function(String entityName) detailsSheetLabel;
  final String removeDraftPairUnavailable;
  final String removeDraftBusy;
  final String Function(int count) removeDraftBlocked;
  final String removeDraftDialogTitle;
  final String Function(String draftName, String scriptName)
  removeDraftDialogSummary;
  final String removeDraftNoUndo;
  final String removeDraftBoundary;
  final String removeDraftCancel;
  final String removeDraftConfirm;
  final String removeDraftBlockedTitle;
  final String removeDraftBlockedDescription;
  final String Function(String sourceName, String role) removeDraftBlockerLabel;
  final String removeDraftOpenBlocker;
  final String removeDraftBlockedClose;
  final String Function(String draftName) removeDraftSucceeded;
  final String Function(Object error) removeDraftErrorDetails;
  final Revision3StoryEntityWorkbenchCopy workbench;
}

/// Exact pending navigation for a newly-created Story draft.
///
/// The expected project revision is mandatory so a delayed request can never
/// select a coincidentally matching entity from a later checkpoint. Callers
/// with an exact publication receipt may additionally bind the canonical head,
/// closing same-revision head drift. Requests only attach while the matching
/// workspace is mounted; a newer request supersedes an older one. Call
/// [dispose] when the owning project surface is permanently released.
final class Revision3StoryWorkspaceController {
  Object? _attachment;
  Future<bool> Function(
    String entityId,
    int projectRevision,
    String? projectHeadCanonicalJson,
    Revision3StoryWorkbenchSection? section,
    String? selectedLineId,
  )?
  _selectEntityAtRevision;
  VoidCallback? _cancelPendingSelection;
  bool _disposed = false;

  Future<bool> selectEntityAtRevision({
    required String entityId,
    required int projectRevision,
    String? projectHeadCanonicalJson,
    Revision3StoryWorkbenchSection? section,
    String? selectedLineId,
  }) {
    final select = _selectEntityAtRevision;
    if (_disposed ||
        select == null ||
        entityId.isEmpty ||
        projectRevision < 1 ||
        (selectedLineId != null &&
            (selectedLineId.isEmpty ||
                section != Revision3StoryWorkbenchSection.dialogVoice))) {
      return Future<bool>.value(false);
    }
    return select(
      entityId,
      projectRevision,
      projectHeadCanonicalJson,
      section,
      selectedLineId,
    );
  }

  void dispose() {
    if (_disposed) return;
    _disposed = true;
    _cancelPendingSelection?.call();
    _attachment = null;
    _selectEntityAtRevision = null;
    _cancelPendingSelection = null;
  }

  bool _attach(
    Object attachment,
    Future<bool> Function(
      String entityId,
      int projectRevision,
      String? projectHeadCanonicalJson,
      Revision3StoryWorkbenchSection? section,
      String? selectedLineId,
    )
    selectEntityAtRevision,
    VoidCallback cancelPendingSelection,
  ) {
    if (_disposed ||
        (_attachment != null && !identical(_attachment, attachment))) {
      return false;
    }
    _attachment = attachment;
    _selectEntityAtRevision = selectEntityAtRevision;
    _cancelPendingSelection = cancelPendingSelection;
    return true;
  }

  void _detach(Object attachment) {
    if (!identical(_attachment, attachment)) return;
    _attachment = null;
    _selectEntityAtRevision = null;
    _cancelPendingSelection = null;
  }
}

enum _StoryFilter { all, npc, quest }

enum _StoryCreateKind { npcOpening, npcDraft, questOpening, questDraft }

enum _StoryCreateAdvancedAction { npcDraft, questDraft }

@immutable
final class _StoryCheckpoint {
  const _StoryCheckpoint({
    required this.projectRoot,
    required this.projectId,
    required this.projectRevision,
    required this.projectHeadCanonicalJson,
  });

  final String projectRoot;
  final String projectId;
  final int projectRevision;
  final String projectHeadCanonicalJson;

  @override
  bool operator ==(Object other) =>
      other is _StoryCheckpoint &&
      projectRoot == other.projectRoot &&
      projectId == other.projectId &&
      projectRevision == other.projectRevision &&
      projectHeadCanonicalJson == other.projectHeadCanonicalJson;

  @override
  int get hashCode => Object.hash(
    projectRoot,
    projectId,
    projectRevision,
    projectHeadCanonicalJson,
  );
}

final class _PendingStorySelection {
  _PendingStorySelection({
    required this.entityId,
    required this.projectRevision,
    required this.projectHeadCanonicalJson,
    required this.section,
    required this.selectedLineId,
  });

  final String entityId;
  final int projectRevision;
  final String? projectHeadCanonicalJson;
  final Revision3StoryWorkbenchSection? section;
  final String? selectedLineId;
  final Completer<bool> result = Completer<bool>();
}

/// Direct Quest/NPC authoring surface for one exact managed-R3 checkpoint.
///
/// This surface projects only `NpcDraft` and `QuestDraft` entities. It routes
/// bounded authoring callbacks supplied by its owner and does not create build,
/// deployment, runtime, save-game, or game-installation authority.
final class Revision3StoryWorkspace extends StatefulWidget {
  const Revision3StoryWorkspace({
    required this.projectRoot,
    required this.projectId,
    required this.projectRevision,
    required this.projectHeadCanonicalJson,
    required this.load,
    required this.copy,
    required this.onOpenExternalEntity,
    required this.onOpenExternalAsset,
    this.controller,
    this.createNpcOpening,
    this.createNpcDraft,
    this.createQuestOpening,
    this.createQuestDraft,
    this.createNpcOpeningDisabledReason,
    this.createNpcDraftDisabledReason,
    this.createQuestOpeningDisabledReason,
    this.createQuestDraftDisabledReason,
    this.editQuestOutline,
    this.editNpcProfile,
    this.editQuestContext,
    this.editQuestTransitions,
    this.inspectQuestSource,
    this.inspectNpcSource,
    this.editQuestOutlineDisabledReason,
    this.editNpcProfileDisabledReason,
    this.editQuestContextDisabledReason,
    this.editQuestTransitionsDisabledReason,
    this.inspectQuestSourceDisabledReason,
    this.inspectNpcSourceDisabledReason,
    this.removeDraft,
    this.removeDraftDisabledReason,
    this.questJourneyBuilder,
    this.questTranscriptBuilder,
    this.npcDialogVoiceBuilder,
    super.key,
  }) : assert(projectRoot != ''),
       assert(projectId != ''),
       assert(projectRevision >= 1),
       assert(projectHeadCanonicalJson != ''),
       assert(
         createNpcDraft != null ||
             (createNpcDraftDisabledReason != null &&
                 createNpcDraftDisabledReason != ''),
       ),
       assert(
         createQuestDraft != null ||
             (createQuestDraftDisabledReason != null &&
                 createQuestDraftDisabledReason != ''),
       );

  final String projectRoot;
  final String projectId;
  final int projectRevision;
  final String projectHeadCanonicalJson;
  final Revision3StoryWorkspaceLoader load;
  final Revision3StoryWorkspaceCopy copy;
  final ValueChanged<String> onOpenExternalEntity;
  final ValueChanged<String> onOpenExternalAsset;
  final Revision3StoryWorkspaceController? controller;
  final Revision3StoryWorkspaceCreateAction? createNpcOpening;
  final Revision3StoryWorkspaceCreateAction? createNpcDraft;
  final Revision3StoryWorkspaceCreateAction? createQuestOpening;
  final Revision3StoryWorkspaceCreateAction? createQuestDraft;
  final String? createNpcOpeningDisabledReason;
  final String? createNpcDraftDisabledReason;
  final String? createQuestOpeningDisabledReason;
  final String? createQuestDraftDisabledReason;
  final Revision3StoryWorkspaceEntityAction? editQuestOutline;
  final Revision3StoryWorkspaceEntityAction? editNpcProfile;
  final Revision3StoryWorkspaceEntityAction? editQuestContext;
  final Revision3StoryWorkspaceEntityAction? editQuestTransitions;
  final Revision3StoryWorkspaceEntityAction? inspectQuestSource;
  final Revision3StoryWorkspaceEntityAction? inspectNpcSource;
  final String? editQuestOutlineDisabledReason;
  final String? editNpcProfileDisabledReason;
  final String? editQuestContextDisabledReason;
  final String? editQuestTransitionsDisabledReason;
  final String? inspectQuestSourceDisabledReason;
  final String? inspectNpcSourceDisabledReason;
  final Revision3StoryWorkspaceRemoveDraftAction? removeDraft;
  final String? removeDraftDisabledReason;

  /// Builds one exact-current, read-only Quest journey. Its dialog-line handoff
  /// stays inside this workspace so the matching transcript row is selected.
  final Revision3StoryQuestJourneyBuilder? questJourneyBuilder;

  /// Builds the exact-current Quest transcript UI without granting this
  /// workspace native publication or navigation authority.
  final Revision3StoryQuestTranscriptBuilder? questTranscriptBuilder;

  /// Builds the exact-current NPC greeting/Voice UI without granting this
  /// workspace native publication, build, deployment, or runtime authority.
  final Revision3StoryNpcDialogVoiceBuilder? npcDialogVoiceBuilder;

  @override
  State<Revision3StoryWorkspace> createState() =>
      _Revision3StoryWorkspaceState();
}

class _Revision3StoryWorkspaceState extends State<Revision3StoryWorkspace> {
  final TextEditingController _search = TextEditingController();
  Revision3ContentIndex? _index;
  Object? _loadError;
  bool _loading = false;
  bool _creatingNpcOpening = false;
  bool _creatingNpcDraft = false;
  bool _creatingQuestOpening = false;
  bool _creatingQuest = false;
  bool _removingDraft = false;
  bool _removalConfirmationOpen = false;
  int _loadGeneration = 0;
  int _detailsPresentationEpoch = 0;
  _StoryFilter _filter = _StoryFilter.all;
  String? _selectedEntityId;
  final Map<String, Revision3StoryWorkbenchSection> _sections = {};
  final Map<String, String> _selectedTranscriptLines = {};
  _PendingStorySelection? _pendingSelection;
  bool _usesDetailsSheet = false;
  bool _openingDetailsSheet = false;
  Route<void>? _detailsSheetRoute;

  bool get _createActionBusy =>
      _creatingNpcOpening ||
      _creatingNpcDraft ||
      _creatingQuestOpening ||
      _creatingQuest;

  bool get _storyActionBusy =>
      _createActionBusy || _removingDraft || _removalConfirmationOpen;

  _StoryCheckpoint get _checkpoint => _StoryCheckpoint(
    projectRoot: widget.projectRoot,
    projectId: widget.projectId,
    projectRevision: widget.projectRevision,
    projectHeadCanonicalJson: widget.projectHeadCanonicalJson,
  );

  @override
  void initState() {
    super.initState();
    _search.addListener(_searchChanged);
    _attachController(widget.controller);
    unawaited(_reload());
  }

  @override
  void didUpdateWidget(covariant Revision3StoryWorkspace oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (!identical(oldWidget.controller, widget.controller)) {
      oldWidget.controller?._detach(this);
      _cancelPendingSelection();
      _attachController(widget.controller);
    }

    final oldCheckpoint = _StoryCheckpoint(
      projectRoot: oldWidget.projectRoot,
      projectId: oldWidget.projectId,
      projectRevision: oldWidget.projectRevision,
      projectHeadCanonicalJson: oldWidget.projectHeadCanonicalJson,
    );
    if (oldCheckpoint == _checkpoint) return;

    final changedProject =
        oldWidget.projectRoot != widget.projectRoot ||
        oldWidget.projectId != widget.projectId;
    if (changedProject) {
      _cancelPendingSelection();
      _search.clear();
      _filter = _StoryFilter.all;
      _selectedEntityId = null;
      _sections.clear();
      _selectedTranscriptLines.clear();
      _usesDetailsSheet = false;
    } else {
      final pending = _pendingSelection;
      if (pending != null &&
          (widget.projectRevision > pending.projectRevision ||
              (pending.projectHeadCanonicalJson != null &&
                  widget.projectHeadCanonicalJson !=
                      pending.projectHeadCanonicalJson))) {
        _completePendingSelection(false);
      }
    }
    _removingDraft = false;
    _removalConfirmationOpen = false;
    unawaited(_reload(clearCurrent: true));
  }

  @override
  void dispose() {
    _detailsPresentationEpoch++;
    _loadGeneration++;
    _closeRouteIfPresent(_detailsSheetRoute, removeImmediately: true);
    _clearDetailsSheetBinding();
    _cancelPendingSelection();
    widget.controller?._detach(this);
    _search
      ..removeListener(_searchChanged)
      ..dispose();
    super.dispose();
  }

  void _attachController(Revision3StoryWorkspaceController? controller) {
    controller?._attach(this, _selectEntityAtRevision, _cancelPendingSelection);
  }

  void _searchChanged() {
    if (mounted) setState(() {});
  }

  Future<void> _reload({bool clearCurrent = false}) async {
    _invalidateDetailsSheet();
    final generation = ++_loadGeneration;
    final checkpoint = _checkpoint;
    setState(() {
      _loading = true;
      _loadError = null;
      if (clearCurrent) _index = null;
    });
    try {
      final index = await Future<Revision3ContentIndex>.sync(widget.load);
      if (!mounted ||
          generation != _loadGeneration ||
          checkpoint != _checkpoint) {
        return;
      }
      if (index.projectId != checkpoint.projectId ||
          index.projectRevision != checkpoint.projectRevision) {
        throw _StoryCheckpointMismatch(widget.copy.checkpointMismatchError);
      }
      final story = _storyEntities(index);
      setState(() {
        _index = index;
        _loading = false;
        _loadError = null;
        final selected = _selectedEntityId;
        _selectedEntityId =
            selected != null && story.any((entity) => entity.id == selected)
            ? selected
            : story.firstOrNull?.id;
        _sections.removeWhere(
          (entityId, _) => !story.any((entity) => entity.id == entityId),
        );
        _selectedTranscriptLines.removeWhere(
          (entityId, _) => !story.any((entity) => entity.id == entityId),
        );
      });
      _resolvePendingSelection(index);
    } catch (error) {
      if (!mounted ||
          generation != _loadGeneration ||
          checkpoint != _checkpoint) {
        return;
      }
      setState(() {
        _loading = false;
        _loadError = error;
      });
    }
  }

  Future<bool> _selectEntityAtRevision(
    String entityId,
    int projectRevision,
    String? projectHeadCanonicalJson,
    Revision3StoryWorkbenchSection? section,
    String? selectedLineId,
  ) {
    if (!mounted ||
        entityId.isEmpty ||
        projectRevision < 1 ||
        (projectRevision == widget.projectRevision &&
            projectHeadCanonicalJson != null &&
            projectHeadCanonicalJson != widget.projectHeadCanonicalJson)) {
      return Future<bool>.value(false);
    }
    final index = _index;
    if (index != null && index.projectRevision == projectRevision) {
      return Future<bool>.value(
        _resolveExactSelection(index, entityId, section, selectedLineId),
      );
    }
    if (projectRevision < widget.projectRevision) {
      return Future<bool>.value(false);
    }
    _completePendingSelection(false);
    final pending = _PendingStorySelection(
      entityId: entityId,
      projectRevision: projectRevision,
      projectHeadCanonicalJson: projectHeadCanonicalJson,
      section: section,
      selectedLineId: selectedLineId,
    );
    _pendingSelection = pending;
    return pending.result.future;
  }

  void _resolvePendingSelection(Revision3ContentIndex index) {
    final pending = _pendingSelection;
    if (pending == null) return;
    if (index.projectRevision < pending.projectRevision) return;
    if (index.projectRevision > pending.projectRevision) {
      _completePendingSelection(false);
      return;
    }
    if (pending.projectHeadCanonicalJson != null &&
        pending.projectHeadCanonicalJson != widget.projectHeadCanonicalJson) {
      _completePendingSelection(false);
      return;
    }
    final resolved = _resolveExactSelection(
      index,
      pending.entityId,
      pending.section,
      pending.selectedLineId,
    );
    _completePendingSelection(resolved);
  }

  bool _resolveExactSelection(
    Revision3ContentIndex index,
    String entityId,
    Revision3StoryWorkbenchSection? section,
    String? selectedLineId,
  ) {
    final entity = index.entityById(entityId);
    if (entity == null || !_isStoryEntity(entity)) return false;
    if (section != null &&
        !Revision3StoryEntityWorkbench.supportsSection(entity, section)) {
      return false;
    }
    if (selectedLineId != null &&
        (section != Revision3StoryWorkbenchSection.dialogVoice ||
            !_hasExactDialogVoiceLine(index, entity, selectedLineId))) {
      return false;
    }
    _selectEntity(
      index,
      entity,
      section: section,
      selectedLineId: selectedLineId,
      reveal: true,
    );
    _scheduleDetailsPresentation(index, entity.id);
    return true;
  }

  void _completePendingSelection(bool resolved) {
    final pending = _pendingSelection;
    _pendingSelection = null;
    if (pending != null && !pending.result.isCompleted) {
      pending.result.complete(resolved);
    }
  }

  void _cancelPendingSelection() => _completePendingSelection(false);

  void _selectEntity(
    Revision3ContentIndex index,
    Revision3ContentEntity entity, {
    Revision3StoryWorkbenchSection? section,
    String? selectedLineId,
    bool reveal = false,
  }) {
    if (!identical(index, _index) || !_isStoryEntity(entity)) return;
    if (reveal && _search.text.isNotEmpty) _search.clear();
    setState(() {
      if (reveal) _filter = _StoryFilter.all;
      _selectedEntityId = entity.id;
      if (section != null) _sections[entity.id] = section;
      if (selectedLineId != null) {
        _selectedTranscriptLines[entity.id] = selectedLineId;
      }
    });
  }

  void _scheduleDetailsPresentation(
    Revision3ContentIndex index,
    String entityId,
  ) {
    final epoch = ++_detailsPresentationEpoch;
    WidgetsBinding.instance.addPostFrameCallback((_) {
      if (!mounted ||
          epoch != _detailsPresentationEpoch ||
          !_usesDetailsSheet ||
          !identical(index, _index) ||
          _selectedEntityId != entityId) {
        return;
      }
      final entity = index.entityById(entityId);
      if (entity != null && _isStoryEntity(entity)) {
        unawaited(_showDetailsSheet(index, entity));
      }
    });
  }

  Future<void> _runCreate(
    Revision3StoryWorkspaceCreateAction action, {
    required _StoryCreateKind kind,
  }) async {
    if (_storyActionBusy) return;
    setState(() {
      switch (kind) {
        case _StoryCreateKind.npcOpening:
          _creatingNpcOpening = true;
        case _StoryCreateKind.npcDraft:
          _creatingNpcDraft = true;
        case _StoryCreateKind.questOpening:
          _creatingQuestOpening = true;
        case _StoryCreateKind.questDraft:
          _creatingQuest = true;
      }
    });
    try {
      await action();
    } catch (error) {
      if (mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(content: Text(widget.copy.createErrorDetails(error))),
        );
      }
    } finally {
      if (mounted) {
        setState(() {
          switch (kind) {
            case _StoryCreateKind.npcOpening:
              _creatingNpcOpening = false;
            case _StoryCreateKind.npcDraft:
              _creatingNpcDraft = false;
            case _StoryCreateKind.questOpening:
              _creatingQuestOpening = false;
            case _StoryCreateKind.questDraft:
              _creatingQuest = false;
          }
        });
      }
    }
  }

  Future<void> _requestRemoveDraft(
    Revision3ContentIndex index,
    Revision3StoryDraftRemovalPreflight preflight,
  ) async {
    final action = widget.removeDraft;
    final module = preflight.scriptModule;
    if (action == null ||
        module == null ||
        !preflight.canRemove ||
        !_isExactCurrentIndex(index) ||
        _createActionBusy ||
        _removingDraft ||
        _removalConfirmationOpen) {
      return;
    }

    final confirmationCheckpoint = _checkpoint;
    setState(() => _removalConfirmationOpen = true);
    final confirmed = await showDialog<bool>(
      context: context,
      barrierDismissible: false,
      builder: (dialogContext) => AlertDialog(
        key: const Key('revision3-story-remove-dialog'),
        title: Text(widget.copy.removeDraftDialogTitle),
        content: SingleChildScrollView(
          child: Column(
            mainAxisSize: MainAxisSize.min,
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Text(
                widget.copy.removeDraftDialogSummary(
                  _entityName(preflight.draft),
                  _entityName(module),
                ),
              ),
              const SizedBox(height: 12),
              _RemovalEntityRow(
                key: const Key('revision3-story-remove-draft-name'),
                icon:
                    preflight.draft.kind == Revision3ContentEntityKind.npcDraft
                    ? Icons.person_outline
                    : Icons.assignment_outlined,
                name: _entityName(preflight.draft),
              ),
              const SizedBox(height: 6),
              _RemovalEntityRow(
                key: const Key('revision3-story-remove-script-name'),
                icon: Icons.code_outlined,
                name: _entityName(module),
              ),
              const SizedBox(height: 14),
              Text(
                widget.copy.removeDraftNoUndo,
                style: TextStyle(
                  color: Theme.of(dialogContext).colorScheme.error,
                  fontWeight: FontWeight.w600,
                ),
              ),
              const SizedBox(height: 8),
              Text(widget.copy.removeDraftBoundary),
            ],
          ),
        ),
        actions: [
          TextButton(
            key: const Key('revision3-story-remove-cancel'),
            onPressed: () => Navigator.of(dialogContext).pop(false),
            child: Text(widget.copy.removeDraftCancel),
          ),
          FilledButton(
            key: const Key('revision3-story-remove-confirm'),
            onPressed: () => Navigator.of(dialogContext).pop(true),
            child: Text(widget.copy.removeDraftConfirm),
          ),
        ],
      ),
    );
    if (!mounted) return;
    if (confirmed != true) {
      setState(() => _removalConfirmationOpen = false);
      return;
    }

    final currentIndex = _index;
    final currentDraft = currentIndex?.entityById(preflight.draft.id);
    final refreshedPreflight = currentIndex == null || currentDraft == null
        ? null
        : Revision3StoryDraftRemovalPreflight.fromIndex(
            index: currentIndex,
            draft: currentDraft,
          );
    if (confirmationCheckpoint != _checkpoint ||
        widget.removeDraft == null ||
        currentIndex == null ||
        !_isExactCurrentIndex(currentIndex) ||
        refreshedPreflight == null ||
        !refreshedPreflight.canRemove ||
        refreshedPreflight.scriptModule?.id != module.id ||
        currentDraft!.revision != preflight.draft.revision ||
        refreshedPreflight.scriptModule!.revision != module.revision) {
      setState(() => _removalConfirmationOpen = false);
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(content: Text(widget.copy.checkpointMismatchError)),
      );
      return;
    }

    setState(() {
      _removalConfirmationOpen = false;
      _removingDraft = true;
    });
    try {
      await widget.removeDraft!(
        index: currentIndex,
        draft: currentDraft,
        scriptModule: refreshedPreflight.scriptModule!,
      );
      if (!mounted) return;
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(
          content: Text(
            widget.copy.removeDraftSucceeded(_entityName(preflight.draft)),
          ),
        ),
      );
      await WidgetsBinding.instance.endOfFrame;
      if (mounted && confirmationCheckpoint == _checkpoint) {
        // The owner normally rebuilds this workspace with the published head.
        // If that rebuild is delayed, leave the old exact checkpoint visible
        // instead of pretending that the removed draft is already gone.
        setState(() => _removingDraft = false);
      }
    } catch (error) {
      if (!mounted) return;
      await WidgetsBinding.instance.endOfFrame;
      if (!mounted) return;
      final refreshed = await _refreshRemovalFailure();
      if (!mounted) return;
      final refreshedDraft = refreshed?.entityById(preflight.draft.id);
      final refreshedRemoval = refreshed == null || refreshedDraft == null
          ? null
          : Revision3StoryDraftRemovalPreflight.fromIndex(
              index: refreshed,
              draft: refreshedDraft,
            );
      if (refreshedRemoval != null && refreshedRemoval.blockers.isNotEmpty) {
        await _showRemovalBlockers(
          refreshed!,
          refreshedRemoval,
          fromSheet: false,
        );
      } else if (mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(content: Text(widget.copy.removeDraftErrorDetails(error))),
        );
      }
    } finally {
      if (mounted) setState(() => _removingDraft = false);
    }
  }

  Future<Revision3ContentIndex?> _refreshRemovalFailure() async {
    final checkpoint = _checkpoint;
    try {
      final refreshed = await Future<Revision3ContentIndex>.sync(widget.load);
      if (!mounted || checkpoint != _checkpoint) return _index;
      if (refreshed.projectId != checkpoint.projectId ||
          refreshed.projectRevision != checkpoint.projectRevision) {
        return _index;
      }
      final story = _storyEntities(refreshed);
      _invalidateDetailsSheet();
      setState(() {
        _index = refreshed;
        final selected = _selectedEntityId;
        _selectedEntityId =
            selected != null && story.any((entity) => entity.id == selected)
            ? selected
            : story.firstOrNull?.id;
      });
      return refreshed;
    } catch (_) {
      return _index;
    }
  }

  Future<void> _showRemovalBlockers(
    Revision3ContentIndex index,
    Revision3StoryDraftRemovalPreflight preflight, {
    required bool fromSheet,
  }) async {
    if (!_isExactCurrentIndex(index) || preflight.blockers.isEmpty) return;
    String? selectedSourceId;
    final blockerListHeight = (preflight.blockers.length * 72.0)
        .clamp(72.0, 280.0)
        .toDouble();
    await showDialog<void>(
      context: context,
      builder: (dialogContext) => AlertDialog(
        key: const Key('revision3-story-remove-blockers-dialog'),
        title: Text(widget.copy.removeDraftBlockedTitle),
        content: SizedBox(
          width: 480,
          child: Column(
            mainAxisSize: MainAxisSize.min,
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: [
              Text(widget.copy.removeDraftBlockedDescription),
              const SizedBox(height: 10),
              SizedBox(
                height: blockerListHeight,
                child: ListView.builder(
                  itemCount: preflight.blockers.length,
                  itemBuilder: (context, blockerIndex) {
                    final blocker = preflight.blockers[blockerIndex];
                    return ListTile(
                      key: Key(
                        'revision3-story-remove-blocker-${blocker.source.id}-${blocker.reference.role}-$blockerIndex',
                      ),
                      leading: const Icon(Icons.link_outlined),
                      title: Text(
                        widget.copy.removeDraftBlockerLabel(
                          _entityName(blocker.source),
                          blocker.reference.role,
                        ),
                      ),
                      trailing: Tooltip(
                        message: widget.copy.removeDraftOpenBlocker,
                        child: const Icon(Icons.open_in_new_outlined),
                      ),
                      onTap: () {
                        selectedSourceId = blocker.source.id;
                        Navigator.of(dialogContext).pop();
                      },
                    );
                  },
                ),
              ),
            ],
          ),
        ),
        actions: [
          TextButton(
            key: const Key('revision3-story-remove-blockers-close'),
            onPressed: () => Navigator.of(dialogContext).pop(),
            child: Text(widget.copy.removeDraftBlockedClose),
          ),
        ],
      ),
    );
    final sourceId = selectedSourceId;
    if (!mounted || sourceId == null || !_isExactCurrentIndex(index)) return;
    _openEntityReference(index, sourceId, fromSheet: fromSheet);
  }

  @override
  Widget build(BuildContext context) => LayoutBuilder(
    builder: (context, constraints) {
      _usesDetailsSheet =
          constraints.maxWidth < 900 || constraints.maxHeight < 430;
      final denseChrome =
          constraints.maxWidth < 720 || constraints.maxHeight < 520;
      final scrollTightChrome =
          constraints.hasBoundedHeight && constraints.maxHeight < 520;
      final index = _index;
      final storyActionBusy = _storyActionBusy;
      final showNpcOpening =
          widget.createNpcOpening != null ||
          widget.createNpcOpeningDisabledReason != null;
      final createNpcOpening =
          widget.createNpcOpening == null || storyActionBusy
          ? null
          : () => _runCreate(
              widget.createNpcOpening!,
              kind: _StoryCreateKind.npcOpening,
            );
      final showQuestOpening =
          widget.createQuestOpening != null ||
          widget.createQuestOpeningDisabledReason != null;
      final createQuestOpening =
          widget.createQuestOpening == null || storyActionBusy
          ? null
          : () => _runCreate(
              widget.createQuestOpening!,
              kind: _StoryCreateKind.questOpening,
            );
      final header = _StoryHeader(
        copy: widget.copy,
        index: index,
        loading: _loading,
        dense: denseChrome,
        actionsBusy: storyActionBusy,
        creatingNpcOpening: _creatingNpcOpening,
        creatingNpcDraft: _creatingNpcDraft,
        creatingQuestOpening: _creatingQuestOpening,
        creatingQuest: _creatingQuest,
        showNpcOpening: showNpcOpening,
        showQuestOpening: showQuestOpening,
        createNpcOpening: createNpcOpening,
        createNpcDraft: widget.createNpcDraft == null || storyActionBusy
            ? null
            : () => _runCreate(
                widget.createNpcDraft!,
                kind: _StoryCreateKind.npcDraft,
              ),
        createQuestOpening: createQuestOpening,
        createQuest: widget.createQuestDraft == null || storyActionBusy
            ? null
            : () => _runCreate(
                widget.createQuestDraft!,
                kind: _StoryCreateKind.questDraft,
              ),
        createNpcOpeningDisabledReason: widget.createNpcOpening == null
            ? widget.createNpcOpeningDisabledReason
            : null,
        createNpcDraftDisabledReason: widget.createNpcDraft == null
            ? widget.createNpcDraftDisabledReason
            : null,
        createQuestOpeningDisabledReason: widget.createQuestOpening == null
            ? widget.createQuestOpeningDisabledReason
            : null,
        createQuestDisabledReason: widget.createQuestDraft == null
            ? widget.createQuestDraftDisabledReason
            : null,
      );
      final loaded = _loadError == null && index != null;
      final searchAndFilters = loaded
          ? _StorySearchAndFilters(
              copy: widget.copy,
              controller: _search,
              filter: _filter,
              dense: denseChrome,
              onFilterChanged: (value) => setState(() => _filter = value),
            )
          : null;
      final body = _loadError != null
          ? _StoryLoadError(
              copy: widget.copy,
              error: _loadError!,
              retry: _loading ? null : _reload,
            )
          : index == null
          ? Center(
              child: Semantics(
                liveRegion: true,
                label: widget.copy.loadingLabel,
                child: const CircularProgressIndicator(
                  key: Key('revision3-story-workspace-loading'),
                ),
              ),
            )
          : _buildLoaded(
              index,
              showQuestOpening: showQuestOpening,
              createQuestOpening: createQuestOpening,
            );

      if (!scrollTightChrome) {
        return Column(
          key: const Key('revision3-story-workspace'),
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            header,
            ?searchAndFilters,
            Expanded(child: body),
          ],
        );
      }

      final reservedBodyHeight = constraints.maxHeight < 260
          ? constraints.maxHeight * 0.45
          : 128.0;
      final chromeHeight = (constraints.maxHeight - reservedBodyHeight)
          .clamp(0.0, 280.0)
          .toDouble();
      return Column(
        key: const Key('revision3-story-workspace'),
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          SizedBox(
            height: chromeHeight,
            child: SingleChildScrollView(
              key: const Key('revision3-story-workspace-tight-chrome-scroll'),
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.stretch,
                children: [header, ?searchAndFilters],
              ),
            ),
          ),
          Expanded(child: body),
        ],
      );
    },
  );

  Widget _buildLoaded(
    Revision3ContentIndex index, {
    required bool showQuestOpening,
    required VoidCallback? createQuestOpening,
  }) {
    final allStory = _storyEntities(index);
    final foldedQuery = _search.text.trim().toLowerCase();
    final visible = allStory
        .where(
          (entity) => switch (_filter) {
            _StoryFilter.all => true,
            _StoryFilter.npc =>
              entity.kind == Revision3ContentEntityKind.npcDraft,
            _StoryFilter.quest =>
              entity.kind == Revision3ContentEntityKind.questDraft,
          },
        )
        .where((entity) => entity.matches(foldedQuery))
        .toList(growable: false);
    final selected = visible
        .where((entity) => entity.id == _selectedEntityId)
        .firstOrNull;
    final list = _StoryEntityList(
      copy: widget.copy,
      entities: visible,
      selectedId: selected?.id,
      emptyLabel: allStory.isEmpty
          ? widget.copy.noStoryDrafts
          : widget.copy.noMatchingStoryDrafts,
      emptyAction: allStory.isEmpty && showQuestOpening
          ? _StoryQuestOpeningButton(
              buttonKey: const Key(
                'revision3-story-workspace-empty-create-quest-opening',
              ),
              copy: widget.copy,
              dense: true,
              creating: _creatingQuestOpening,
              onPressed: createQuestOpening,
              disabledReason: widget.createQuestOpening == null
                  ? widget.createQuestOpeningDisabledReason
                  : null,
            )
          : null,
      emptyActionDisabledReason: allStory.isEmpty && showQuestOpening
          ? widget.createQuestOpening == null
                ? widget.createQuestOpeningDisabledReason
                : null
          : null,
      onSelected: (entity) {
        _selectEntity(index, entity);
        if (_usesDetailsSheet) unawaited(_showDetailsSheet(index, entity));
      },
    );
    if (_usesDetailsSheet) return list;
    return Row(
      key: const Key('revision3-story-workspace-wide'),
      children: [
        Expanded(flex: 2, child: list),
        const VerticalDivider(width: 1),
        Expanded(
          flex: 3,
          child: selected == null
              ? _StoryEmptyDetails(label: widget.copy.selectDraftLabel)
              : _buildWorkbench(index, selected),
        ),
      ],
    );
  }

  Widget _buildWorkbench(
    Revision3ContentIndex index,
    Revision3ContentEntity entity, {
    bool sheet = false,
    StateSetter? sheetSetState,
  }) {
    final removal = Revision3StoryDraftRemovalPreflight.fromIndex(
      index: index,
      draft: entity,
    );
    final removeBusy = _storyActionBusy;
    final removeDisabledReason = removeBusy
        ? widget.copy.removeDraftBusy
        : widget.removeDraft == null
        ? widget.removeDraftDisabledReason ??
              widget.copy.removeDraftPairUnavailable
        : !removal.hasExactPair
        ? widget.copy.removeDraftPairUnavailable
        : removal.blockers.isNotEmpty
        ? widget.copy.removeDraftBlocked(removal.blockers.length)
        : null;

    Future<void> run(Revision3StoryWorkspaceEntityAction action) async {
      if (!_isExactCurrentIndex(index)) return;
      if (sheet && mounted && Navigator.of(context).canPop()) {
        Navigator.of(context).pop();
      }
      await action(index, entity);
    }

    void updatePresentation(VoidCallback update) {
      final updateSheet = sheetSetState;
      if (updateSheet != null) {
        updateSheet(update);
      } else {
        setState(update);
      }
    }

    return Revision3StoryEntityWorkbench(
      key: ValueKey(
        'revision3-story-workspace-workbench-${widget.projectId}-${entity.id}',
      ),
      projectId: widget.projectId,
      index: index,
      entity: entity,
      selectedSection:
          _sections[entity.id] ??
          Revision3StoryEntityWorkbench.defaultSectionFor(entity),
      onSectionChanged: (section) {
        if (_isExactCurrentIndex(index)) {
          updatePresentation(() => _sections[entity.id] = section);
        }
      },
      actions: Revision3StoryEntityWorkbenchActions(
        openEntity: (entityId) =>
            _openEntityReference(index, entityId, fromSheet: sheet),
        openAsset: (sha256) {
          if (!_isExactCurrentIndex(index)) return;
          if (sheet && mounted && Navigator.of(context).canPop()) {
            Navigator.of(context).pop();
          }
          widget.onOpenExternalAsset(sha256);
        },
        editOverview:
            entity.kind == Revision3ContentEntityKind.questDraft &&
                widget.editQuestOutline != null
            ? () => run(widget.editQuestOutline!)
            : null,
        editNpcProfile:
            entity.kind == Revision3ContentEntityKind.npcDraft &&
                widget.editNpcProfile != null
            ? () => run(widget.editNpcProfile!)
            : null,
        editStory:
            entity.kind == Revision3ContentEntityKind.questDraft &&
                widget.editQuestContext != null
            ? () => run(widget.editQuestContext!)
            : null,
        editLogic:
            entity.kind == Revision3ContentEntityKind.questDraft &&
                widget.editQuestTransitions != null
            ? () => run(widget.editQuestTransitions!)
            : null,
        inspectQuest:
            entity.kind == Revision3ContentEntityKind.questDraft &&
                widget.inspectQuestSource != null
            ? () => run(widget.inspectQuestSource!)
            : null,
        inspectNpc:
            entity.kind == Revision3ContentEntityKind.npcDraft &&
                widget.inspectNpcSource != null
            ? () => run(widget.inspectNpcSource!)
            : null,
        editOverviewDisabledReason: widget.editQuestOutlineDisabledReason,
        editNpcProfileDisabledReason: widget.editNpcProfileDisabledReason,
        editStoryDisabledReason: widget.editQuestContextDisabledReason,
        editLogicDisabledReason: widget.editQuestTransitionsDisabledReason,
        inspectQuestDisabledReason: widget.inspectQuestSourceDisabledReason,
        inspectNpcDisabledReason: widget.inspectNpcSourceDisabledReason,
        removeDraft: removeDisabledReason == null
            ? () => _requestRemoveDraft(index, removal)
            : null,
        reviewRemovalBlockers: removal.blockers.isEmpty
            ? null
            : () => _showRemovalBlockers(index, removal, fromSheet: sheet),
        removeDraftDisabledReason: removeDisabledReason,
        removingDraft: _removingDraft,
      ),
      questJourney:
          entity.kind == Revision3ContentEntityKind.questDraft &&
              widget.questJourneyBuilder != null
          ? widget.questJourneyBuilder!(
              index: index,
              quest: entity,
              onOpenDialogLine: (lineId) {
                if (!_isExactCurrentIndex(index) || lineId.isEmpty) return;
                if (!_hasExactTranscriptLine(index, entity, lineId)) return;
                updatePresentation(() {
                  _sections[entity.id] =
                      Revision3StoryWorkbenchSection.dialogVoice;
                  _selectedTranscriptLines[entity.id] = lineId;
                });
              },
            )
          : null,
      questTranscript:
          entity.kind == Revision3ContentEntityKind.questDraft &&
              widget.questTranscriptBuilder != null
          ? widget.questTranscriptBuilder!(
              index: index,
              quest: entity,
              selectedLineId: _selectedTranscriptLines[entity.id],
              onSelectedLineChanged: (lineId) {
                if (!_isExactCurrentIndex(index)) return;
                updatePresentation(() {
                  if (lineId == null || lineId.isEmpty) {
                    _selectedTranscriptLines.remove(entity.id);
                  } else {
                    _selectedTranscriptLines[entity.id] = lineId;
                  }
                });
              },
            )
          : null,
      npcDialogVoice:
          entity.kind == Revision3ContentEntityKind.npcDraft &&
              widget.npcDialogVoiceBuilder != null
          ? widget.npcDialogVoiceBuilder!(
              index: index,
              npc: entity,
              selectedLineId: _selectedTranscriptLines[entity.id],
              onSelectedLineChanged: (lineId) {
                if (!_isExactCurrentIndex(index)) return;
                updatePresentation(() {
                  if (lineId == null || lineId.isEmpty) {
                    _selectedTranscriptLines.remove(entity.id);
                  } else {
                    _selectedTranscriptLines[entity.id] = lineId;
                  }
                });
              },
            )
          : null,
      copy: widget.copy.workbench,
    );
  }

  void _openEntityReference(
    Revision3ContentIndex index,
    String entityId, {
    required bool fromSheet,
  }) {
    if (!_isExactCurrentIndex(index)) return;
    final target = index.entityById(entityId);
    if (target == null) return;
    if (!_isStoryEntity(target)) {
      if (fromSheet && mounted && Navigator.of(context).canPop()) {
        Navigator.of(context).pop();
      }
      widget.onOpenExternalEntity(entityId);
      return;
    }
    if (fromSheet && mounted && Navigator.of(context).canPop()) {
      Navigator.of(context).pop();
    }
    _selectEntity(index, target, reveal: true);
    if (_usesDetailsSheet) _scheduleDetailsPresentation(index, target.id);
  }

  Future<void> _showDetailsSheet(
    Revision3ContentIndex index,
    Revision3ContentEntity entity,
  ) async {
    if (!_isExactCurrentIndex(index) || _openingDetailsSheet) return;
    _openingDetailsSheet = true;
    final presentationEpoch = ++_detailsPresentationEpoch;
    Route<void>? route;
    try {
      await showModalBottomSheet<void>(
        context: context,
        isScrollControlled: true,
        showDragHandle: true,
        builder: (sheetContext) {
          route ??= ModalRoute.of(sheetContext) as Route<void>?;
          if (!mounted ||
              presentationEpoch != _detailsPresentationEpoch ||
              !_isExactCurrentIndex(index)) {
            WidgetsBinding.instance.addPostFrameCallback((_) {
              _closeRouteIfPresent(route);
            });
            return const SizedBox.shrink();
          }
          _detailsSheetRoute = route;
          return SafeArea(
            child: Semantics(
              container: true,
              explicitChildNodes: true,
              label: widget.copy.detailsSheetLabel(_entityName(entity)),
              child: SizedBox(
                key: const Key('revision3-story-workspace-details-sheet'),
                height: MediaQuery.sizeOf(sheetContext).height * 0.9,
                child: StatefulBuilder(
                  builder: (context, setSheetState) => _buildWorkbench(
                    index,
                    entity,
                    sheet: true,
                    sheetSetState: setSheetState,
                  ),
                ),
              ),
            ),
          );
        },
      );
      if (route case final TransitionRoute<void> transitionRoute) {
        await transitionRoute.completed;
      }
    } finally {
      if (identical(_detailsSheetRoute, route)) _clearDetailsSheetBinding();
      _openingDetailsSheet = false;
    }
  }

  bool _isExactCurrentIndex(Revision3ContentIndex index) =>
      mounted &&
      !_loading &&
      identical(_index, index) &&
      index.projectId == widget.projectId &&
      index.projectRevision == widget.projectRevision;

  void _invalidateDetailsSheet() {
    _detailsPresentationEpoch++;
    final route = _detailsSheetRoute;
    _closeRouteIfPresent(route);
    _clearDetailsSheetBinding();
  }

  void _clearDetailsSheetBinding() {
    _detailsSheetRoute = null;
  }

  void _closeRouteIfPresent(
    Route<void>? route, {
    bool removeImmediately = false,
  }) {
    if (route == null || !route.isActive) return;
    final navigator = route.navigator;
    if (navigator == null) return;
    if (route.isCurrent && !removeImmediately) {
      navigator.pop();
    } else {
      navigator.removeRoute(route);
    }
  }
}

final class _StoryCheckpointMismatch implements Exception {
  const _StoryCheckpointMismatch(this.message);
  final String message;

  @override
  String toString() => message;
}

class _StoryHeader extends StatelessWidget {
  const _StoryHeader({
    required this.copy,
    required this.index,
    required this.loading,
    required this.dense,
    required this.actionsBusy,
    required this.creatingNpcOpening,
    required this.creatingNpcDraft,
    required this.creatingQuestOpening,
    required this.creatingQuest,
    required this.showNpcOpening,
    required this.showQuestOpening,
    required this.createNpcOpening,
    required this.createNpcDraft,
    required this.createQuestOpening,
    required this.createQuest,
    required this.createNpcOpeningDisabledReason,
    required this.createNpcDraftDisabledReason,
    required this.createQuestOpeningDisabledReason,
    required this.createQuestDisabledReason,
  });

  final Revision3StoryWorkspaceCopy copy;
  final Revision3ContentIndex? index;
  final bool loading;
  final bool dense;
  final bool actionsBusy;
  final bool creatingNpcOpening;
  final bool creatingNpcDraft;
  final bool creatingQuestOpening;
  final bool creatingQuest;
  final bool showNpcOpening;
  final bool showQuestOpening;
  final VoidCallback? createNpcOpening;
  final VoidCallback? createNpcDraft;
  final VoidCallback? createQuestOpening;
  final VoidCallback? createQuest;
  final String? createNpcOpeningDisabledReason;
  final String? createNpcDraftDisabledReason;
  final String? createQuestOpeningDisabledReason;
  final String? createQuestDisabledReason;

  @override
  Widget build(BuildContext context) {
    final scheme = Theme.of(context).colorScheme;
    final storyCount = index == null ? 0 : _storyEntities(index!).length;
    final disabledReasons = <Widget>[];
    final seenDisabledReasons = <String>{};
    void addDisabledReason(String? reason, Key key) {
      if (reason == null || !seenDisabledReasons.add(reason)) return;
      disabledReasons.add(
        Tooltip(
          message: dense ? reason : '',
          child: Text(
            reason,
            key: key,
            style: Theme.of(context).textTheme.bodySmall,
            maxLines: dense ? 2 : null,
            overflow: dense ? TextOverflow.ellipsis : null,
          ),
        ),
      );
    }

    if (showNpcOpening) {
      addDisabledReason(
        createNpcOpeningDisabledReason,
        const Key(
          'revision3-story-workspace-create-npc-opening-disabled-reason',
        ),
      );
    }
    if (showQuestOpening) {
      addDisabledReason(
        createQuestOpeningDisabledReason,
        const Key(
          'revision3-story-workspace-create-quest-opening-disabled-reason',
        ),
      );
    }
    addDisabledReason(
      createNpcDraftDisabledReason,
      const Key('revision3-story-workspace-create-npc-disabled-reason'),
    );
    addDisabledReason(
      createQuestDisabledReason,
      const Key('revision3-story-workspace-create-quest-disabled-reason'),
    );
    return Material(
      color: scheme.surfaceContainerLowest,
      child: Padding(
        padding: dense
            ? const EdgeInsets.fromLTRB(10, 8, 10, 7)
            : const EdgeInsets.fromLTRB(16, 12, 16, 10),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            Wrap(
              spacing: dense ? 8 : 12,
              runSpacing: dense ? 6 : 10,
              crossAxisAlignment: WrapCrossAlignment.center,
              alignment: WrapAlignment.spaceBetween,
              children: [
                ConstrainedBox(
                  constraints: const BoxConstraints(minWidth: 180),
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      Semantics(
                        header: true,
                        child: Text(
                          copy.title,
                          key: const Key('revision3-story-workspace-title'),
                          style: Theme.of(context).textTheme.titleLarge,
                          maxLines: dense ? 1 : null,
                          overflow: dense ? TextOverflow.ellipsis : null,
                        ),
                      ),
                      Semantics(
                        liveRegion: loading,
                        child: Text(
                          index == null
                              ? copy.loadingLabel
                              : copy.checkpointSummary(
                                  storyCount,
                                  index!.projectRevision,
                                ),
                          key: const Key(
                            'revision3-story-workspace-checkpoint-summary',
                          ),
                          maxLines: dense ? 1 : null,
                          overflow: dense ? TextOverflow.ellipsis : null,
                        ),
                      ),
                    ],
                  ),
                ),
                Wrap(
                  spacing: 8,
                  runSpacing: 8,
                  crossAxisAlignment: WrapCrossAlignment.center,
                  children: [
                    if (showNpcOpening)
                      _StoryNpcOpeningButton(
                        buttonKey: const Key(
                          'revision3-story-workspace-create-npc-opening',
                        ),
                        copy: copy,
                        dense: dense,
                        creating: creatingNpcOpening,
                        onPressed: createNpcOpening,
                        disabledReason: createNpcOpeningDisabledReason,
                      ),
                    if (showQuestOpening)
                      _StoryQuestOpeningButton(
                        buttonKey: const Key(
                          'revision3-story-workspace-create-quest-opening',
                        ),
                        copy: copy,
                        dense: dense,
                        creating: creatingQuestOpening,
                        onPressed: createQuestOpening,
                        disabledReason: createQuestOpeningDisabledReason,
                      ),
                    PopupMenuButton<_StoryCreateAdvancedAction>(
                      key: const Key(
                        'revision3-story-workspace-create-advanced',
                      ),
                      enabled: !actionsBusy,
                      tooltip: creatingNpcDraft
                          ? copy.creatingNpcLabel
                          : creatingQuest
                          ? copy.creatingQuestLabel
                          : copy.createAdvancedLabel,
                      icon: creatingNpcDraft || creatingQuest
                          ? const SizedBox.square(
                              dimension: 18,
                              child: CircularProgressIndicator(strokeWidth: 2),
                            )
                          : const Icon(Icons.more_horiz),
                      onSelected: (action) => switch (action) {
                        _StoryCreateAdvancedAction.npcDraft =>
                          createNpcDraft?.call(),
                        _StoryCreateAdvancedAction.questDraft =>
                          createQuest?.call(),
                      },
                      itemBuilder: (context) => [
                        PopupMenuItem<_StoryCreateAdvancedAction>(
                          key: const Key(
                            'revision3-story-workspace-create-npc',
                          ),
                          value: _StoryCreateAdvancedAction.npcDraft,
                          enabled: createNpcDraft != null,
                          child: Column(
                            crossAxisAlignment: CrossAxisAlignment.start,
                            mainAxisSize: MainAxisSize.min,
                            children: [
                              Text(copy.createNpcLabel),
                              if (createNpcDraftDisabledReason != null)
                                Text(
                                  createNpcDraftDisabledReason!,
                                  style: Theme.of(context).textTheme.bodySmall,
                                ),
                            ],
                          ),
                        ),
                        PopupMenuItem<_StoryCreateAdvancedAction>(
                          key: const Key(
                            'revision3-story-workspace-create-quest',
                          ),
                          value: _StoryCreateAdvancedAction.questDraft,
                          enabled: createQuest != null,
                          child: Column(
                            crossAxisAlignment: CrossAxisAlignment.start,
                            mainAxisSize: MainAxisSize.min,
                            children: [
                              Text(copy.createQuestAdvancedLabel),
                              if (createQuestDisabledReason != null)
                                Text(
                                  createQuestDisabledReason!,
                                  style: Theme.of(context).textTheme.bodySmall,
                                ),
                            ],
                          ),
                        ),
                      ],
                    ),
                  ],
                ),
              ],
            ),
            SizedBox(height: dense ? 5 : 8),
            Container(
              key: const Key('revision3-story-workspace-authority-notice'),
              padding: EdgeInsets.symmetric(
                horizontal: dense ? 8 : 10,
                vertical: dense ? 5 : 7,
              ),
              decoration: BoxDecoration(
                color: scheme.secondaryContainer,
                borderRadius: BorderRadius.circular(8),
              ),
              child: Row(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Icon(
                    Icons.edit_note_outlined,
                    size: 18,
                    color: scheme.onSecondaryContainer,
                  ),
                  const SizedBox(width: 8),
                  Expanded(
                    child: Tooltip(
                      message: dense ? copy.authorityNotice : '',
                      child: Text(
                        copy.authorityNotice,
                        style: TextStyle(color: scheme.onSecondaryContainer),
                        maxLines: dense ? 2 : null,
                        overflow: dense ? TextOverflow.ellipsis : null,
                      ),
                    ),
                  ),
                ],
              ),
            ),
            if (disabledReasons.isNotEmpty) ...[
              SizedBox(height: dense ? 4 : 6),
              ...disabledReasons,
            ],
          ],
        ),
      ),
    );
  }
}

class _StoryNpcOpeningButton extends StatelessWidget {
  const _StoryNpcOpeningButton({
    required this.buttonKey,
    required this.copy,
    required this.dense,
    required this.creating,
    required this.onPressed,
    required this.disabledReason,
  });

  final Key buttonKey;
  final Revision3StoryWorkspaceCopy copy;
  final bool dense;
  final bool creating;
  final VoidCallback? onPressed;
  final String? disabledReason;

  @override
  Widget build(BuildContext context) => Tooltip(
    message: disabledReason ?? (dense ? copy.createNpcOpeningLabel : ''),
    child: FilledButton.icon(
      key: buttonKey,
      onPressed: onPressed,
      style: dense
          ? FilledButton.styleFrom(
              visualDensity: VisualDensity.compact,
              maximumSize: const Size.fromWidth(300),
            )
          : null,
      icon: creating
          ? const SizedBox.square(
              dimension: 16,
              child: CircularProgressIndicator(strokeWidth: 2),
            )
          : const Icon(Icons.record_voice_over_outlined),
      label: Text(
        creating ? copy.creatingNpcOpeningLabel : copy.createNpcOpeningLabel,
        maxLines: 1,
        overflow: TextOverflow.ellipsis,
      ),
    ),
  );
}

class _StoryQuestOpeningButton extends StatelessWidget {
  const _StoryQuestOpeningButton({
    required this.buttonKey,
    required this.copy,
    required this.dense,
    required this.creating,
    required this.onPressed,
    required this.disabledReason,
  });

  final Key buttonKey;
  final Revision3StoryWorkspaceCopy copy;
  final bool dense;
  final bool creating;
  final VoidCallback? onPressed;
  final String? disabledReason;

  @override
  Widget build(BuildContext context) => Tooltip(
    message: disabledReason ?? (dense ? copy.createQuestOpeningLabel : ''),
    child: FilledButton.icon(
      key: buttonKey,
      onPressed: onPressed,
      style: dense
          ? FilledButton.styleFrom(
              visualDensity: VisualDensity.compact,
              maximumSize: const Size.fromWidth(300),
            )
          : null,
      icon: creating
          ? const SizedBox.square(
              dimension: 16,
              child: CircularProgressIndicator(strokeWidth: 2),
            )
          : const Icon(Icons.forum_outlined),
      label: Text(
        creating
            ? copy.creatingQuestOpeningLabel
            : copy.createQuestOpeningLabel,
        maxLines: 1,
        overflow: TextOverflow.ellipsis,
      ),
    ),
  );
}

class _StorySearchAndFilters extends StatelessWidget {
  const _StorySearchAndFilters({
    required this.copy,
    required this.controller,
    required this.filter,
    required this.dense,
    required this.onFilterChanged,
  });

  final Revision3StoryWorkspaceCopy copy;
  final TextEditingController controller;
  final _StoryFilter filter;
  final bool dense;
  final ValueChanged<_StoryFilter> onFilterChanged;

  @override
  Widget build(BuildContext context) => Padding(
    padding: dense
        ? const EdgeInsets.fromLTRB(8, 5, 8, 4)
        : const EdgeInsets.fromLTRB(12, 8, 12, 6),
    child: Column(
      children: [
        TextField(
          key: const Key('revision3-story-workspace-search'),
          controller: controller,
          decoration: InputDecoration(
            isDense: true,
            prefixIcon: const Icon(Icons.search),
            hintText: copy.searchHint,
            suffixIcon: controller.text.isEmpty
                ? null
                : IconButton(
                    key: const Key('revision3-story-workspace-clear-search'),
                    tooltip: copy.clearSearchLabel,
                    onPressed: controller.clear,
                    icon: const Icon(Icons.close),
                  ),
          ),
        ),
        SizedBox(height: dense ? 3 : 6),
        SingleChildScrollView(
          key: const Key('revision3-story-workspace-filters-scroll'),
          scrollDirection: Axis.horizontal,
          child: Row(
            children: [
              ChoiceChip(
                key: const Key('revision3-story-workspace-filter-all'),
                label: Text(copy.allFilterLabel),
                selected: filter == _StoryFilter.all,
                onSelected: (_) => onFilterChanged(_StoryFilter.all),
                visualDensity: dense ? VisualDensity.compact : null,
              ),
              const SizedBox(width: 6),
              ChoiceChip(
                key: const Key('revision3-story-workspace-filter-npc'),
                label: Text(copy.npcFilterLabel),
                selected: filter == _StoryFilter.npc,
                onSelected: (_) => onFilterChanged(_StoryFilter.npc),
                visualDensity: dense ? VisualDensity.compact : null,
              ),
              const SizedBox(width: 6),
              ChoiceChip(
                key: const Key('revision3-story-workspace-filter-quest'),
                label: Text(copy.questFilterLabel),
                selected: filter == _StoryFilter.quest,
                onSelected: (_) => onFilterChanged(_StoryFilter.quest),
                visualDensity: dense ? VisualDensity.compact : null,
              ),
            ],
          ),
        ),
      ],
    ),
  );
}

class _StoryEntityList extends StatelessWidget {
  const _StoryEntityList({
    required this.copy,
    required this.entities,
    required this.selectedId,
    required this.emptyLabel,
    required this.emptyAction,
    required this.emptyActionDisabledReason,
    required this.onSelected,
  });

  final Revision3StoryWorkspaceCopy copy;
  final List<Revision3ContentEntity> entities;
  final String? selectedId;
  final String emptyLabel;
  final Widget? emptyAction;
  final String? emptyActionDisabledReason;
  final ValueChanged<Revision3ContentEntity> onSelected;

  @override
  Widget build(BuildContext context) {
    if (entities.isEmpty) {
      return _StoryEmptyDetails(
        key: const Key('revision3-story-workspace-empty'),
        label: emptyLabel,
        action: emptyAction,
        actionDisabledReason: emptyActionDisabledReason,
      );
    }
    return ListView.builder(
      key: const Key('revision3-story-workspace-list'),
      itemCount: entities.length,
      itemBuilder: (context, index) {
        final entity = entities[index];
        final selected = entity.id == selectedId;
        return Semantics(
          button: true,
          selected: selected,
          child: ListTile(
            key: Key('revision3-story-workspace-entity-${entity.id}'),
            selected: selected,
            leading: Icon(
              entity.kind == Revision3ContentEntityKind.npcDraft
                  ? Icons.person_outline
                  : Icons.assignment_outlined,
            ),
            title: Text(_entityName(entity)),
            subtitle: Text(
              entity.kind == Revision3ContentEntityKind.npcDraft
                  ? copy.workbench.npcKindLabel
                  : copy.workbench.questKindLabel,
            ),
            trailing: entity.problemCount == 0
                ? const Icon(Icons.check_circle_outline, size: 18)
                : Badge(
                    label: Text('${entity.problemCount}'),
                    child: const Icon(Icons.warning_amber_rounded),
                  ),
            onTap: () => onSelected(entity),
          ),
        );
      },
    );
  }
}

class _StoryLoadError extends StatelessWidget {
  const _StoryLoadError({
    required this.copy,
    required this.error,
    required this.retry,
  });

  final Revision3StoryWorkspaceCopy copy;
  final Object error;
  final VoidCallback? retry;

  @override
  Widget build(BuildContext context) => Center(
    child: SingleChildScrollView(
      padding: const EdgeInsets.all(24),
      child: Column(
        key: const Key('revision3-story-workspace-error'),
        mainAxisSize: MainAxisSize.min,
        children: [
          const Icon(Icons.error_outline, size: 40),
          const SizedBox(height: 10),
          Text(
            copy.loadErrorTitle,
            style: Theme.of(context).textTheme.titleMedium,
            textAlign: TextAlign.center,
          ),
          const SizedBox(height: 6),
          Text(copy.loadErrorDetails(error), textAlign: TextAlign.center),
          const SizedBox(height: 14),
          FilledButton.icon(
            key: const Key('revision3-story-workspace-retry'),
            onPressed: retry,
            icon: const Icon(Icons.refresh),
            label: Text(copy.retryLabel),
          ),
        ],
      ),
    ),
  );
}

class _StoryEmptyDetails extends StatelessWidget {
  const _StoryEmptyDetails({
    required this.label,
    this.action,
    this.actionDisabledReason,
    super.key,
  });

  final String label;
  final Widget? action;
  final String? actionDisabledReason;

  @override
  Widget build(BuildContext context) => Center(
    child: SingleChildScrollView(
      padding: const EdgeInsets.all(20),
      child: Column(
        mainAxisSize: MainAxisSize.min,
        children: [
          Text(label, textAlign: TextAlign.center),
          if (action != null) ...[const SizedBox(height: 14), action!],
          if (actionDisabledReason != null) ...[
            const SizedBox(height: 8),
            Text(
              actionDisabledReason!,
              key: const Key(
                'revision3-story-workspace-empty-create-quest-opening-disabled-reason',
              ),
              style: Theme.of(context).textTheme.bodySmall,
              textAlign: TextAlign.center,
            ),
          ],
        ],
      ),
    ),
  );
}

class _RemovalEntityRow extends StatelessWidget {
  const _RemovalEntityRow({required this.icon, required this.name, super.key});

  final IconData icon;
  final String name;

  @override
  Widget build(BuildContext context) => Row(
    children: [
      Icon(icon, size: 20),
      const SizedBox(width: 8),
      Expanded(
        child: Text(name, style: Theme.of(context).textTheme.titleSmall),
      ),
    ],
  );
}

List<Revision3ContentEntity> _storyEntities(Revision3ContentIndex index) =>
    index.entities.where(_isStoryEntity).toList(growable: false);

bool _isStoryEntity(Revision3ContentEntity entity) =>
    entity.kind == Revision3ContentEntityKind.npcDraft ||
    entity.kind == Revision3ContentEntityKind.questDraft;

bool _hasExactTranscriptLine(
  Revision3ContentIndex index,
  Revision3ContentEntity quest,
  String lineId,
) {
  final line = index.entityById(lineId);
  return quest.kind == Revision3ContentEntityKind.questDraft &&
      lineId.isNotEmpty &&
      line?.kind == Revision3ContentEntityKind.dialogLine &&
      quest.references.any(
        (reference) =>
            reference.role == 'quest_transcript_line' &&
            reference.resolution ==
                Revision3ContentReferenceResolution.resolved &&
            reference.target.projectId == index.projectId &&
            reference.target.entityId == lineId &&
            reference.target.expectedKind ==
                Revision3ContentEntityKind.dialogLine,
      );
}

bool _hasExactNpcGreetingLine(
  Revision3ContentIndex index,
  Revision3ContentEntity npc,
  String lineId,
) {
  final line = index.entityById(lineId);
  return npc.kind == Revision3ContentEntityKind.npcDraft &&
      lineId.isNotEmpty &&
      line?.kind == Revision3ContentEntityKind.dialogLine &&
      npc.references.any(
        (reference) =>
            reference.role == 'npc_greeting_line' &&
            reference.qualifier == null &&
            reference.resolution ==
                Revision3ContentReferenceResolution.resolved &&
            reference.target.projectId == index.projectId &&
            reference.target.entityId == lineId &&
            reference.target.expectedKind ==
                Revision3ContentEntityKind.dialogLine,
      );
}

bool _hasExactDialogVoiceLine(
  Revision3ContentIndex index,
  Revision3ContentEntity entity,
  String lineId,
) => switch (entity.kind) {
  Revision3ContentEntityKind.questDraft => _hasExactTranscriptLine(
    index,
    entity,
    lineId,
  ),
  Revision3ContentEntityKind.npcDraft => _hasExactNpcGreetingLine(
    index,
    entity,
    lineId,
  ),
  _ => false,
};

String _entityName(Revision3ContentEntity entity) => entity.displayName.isEmpty
    ? entity.summary.primaryIdentity
    : entity.displayName;
