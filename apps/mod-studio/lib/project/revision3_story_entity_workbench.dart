import 'package:flutter/material.dart';

import 'revision3_content_index.dart';

typedef Revision3StoryWorkbenchAction = Future<void> Function();

String _englishReferenceProblemCount(int count) =>
    '$count unresolved project reference${count == 1 ? '' : 's'}';

/// Author-facing copy for the bounded Quest/NPC workbench.
///
/// The English constructor is intentionally const so embedding surfaces can
/// adopt the workbench before their localization mapping is wired. No project
/// data or readiness claim is synthesized from this copy.
@immutable
final class Revision3StoryEntityWorkbenchCopy {
  const Revision3StoryEntityWorkbenchCopy({
    required this.draftBadge,
    required this.buildBlockedBadge,
    required this.runtimeUnqualifiedBadge,
    required this.overviewTab,
    required this.profileTab,
    required this.storyTab,
    required this.logicTab,
    required this.routineTab,
    required this.inventoryTab,
    required this.dialogVoiceTab,
    required this.referencesTab,
    required this.problemsChecksTab,
    required this.editOverview,
    this.editNpcProfile = 'Edit name & archetype',
    this.npcDialogVoiceNextStepTitle = 'Next step: Dialog & Voice',
    this.npcDialogVoiceNextStepDescription =
        'Draft only: continue with greeting lines, text, and voice. This only links project content; it does not create playable dialog or verify runtime behavior.',
    this.continueToNpcDialogVoice = 'Continue to Dialog & Voice',
    required this.editStory,
    required this.editLogic,
    required this.inspectQuest,
    required this.inspectNpc,
    required this.moreActions,
    required this.removeDraft,
    required this.removingDraft,
    required this.reviewRemovalBlockers,
    required this.capabilityUnavailable,
    this.actionFailed = 'Could not open this editor. Please try again.',
    required this.npcStoryUnavailable,
    required this.npcRoutineUnavailable,
    required this.npcInventoryUnavailable,
    required this.npcDialogVoiceUnavailable,
    required this.questDialogVoiceUnavailable,
    required this.noReferenceProblems,
    required this.referenceProblemCount,
    required this.referenceScopeNotice,
    required this.technicalDetails,
    required this.questKindLabel,
    required this.npcKindLabel,
    required this.questTitleLabel,
    this.npcDisplayNameLabel = 'Character name',
    required this.technicalIdLabel,
    required this.objectivesLabel,
    required this.uniqueNameLabel,
    required this.moduleNamespaceLabel,
    required this.questGiverLabel,
    required this.runtimeParentLabel,
    required this.logicDescription,
    required this.outgoingHeading,
    required this.noOutgoingReferences,
    required this.incomingHeading,
    required this.noIncomingReferences,
    required this.semanticIdentityLabel,
    required this.originLabel,
    required this.entityRevisionLabel,
    required this.stableIdLabel,
    required this.referenceResolvedLabel,
    required this.referenceUnresolvedLabel,
  });

  const Revision3StoryEntityWorkbenchCopy.english({
    this.actionFailed = 'Could not open this editor. Please try again.',
  }) : draftBadge = 'Draft only',
       buildBlockedBadge = 'Build blocked',
       runtimeUnqualifiedBadge = 'Runtime not verified',
       overviewTab = 'Overview',
       profileTab = 'Profile',
       storyTab = 'Story',
       logicTab = 'Logic',
       routineTab = 'Routine',
       inventoryTab = 'Inventory',
       dialogVoiceTab = 'Dialog & Voice',
       referencesTab = 'References',
       problemsChecksTab = 'Problems & Checks',
       editOverview = 'Edit name & objectives',
       editNpcProfile = 'Edit name & archetype',
       npcDialogVoiceNextStepTitle = 'Next step: Dialog & Voice',
       npcDialogVoiceNextStepDescription =
           'Draft only: continue with greeting lines, text, and voice. This only links project content; it does not create playable dialog or verify runtime behavior.',
       continueToNpcDialogVoice = 'Continue to Dialog & Voice',
       editStory = 'Edit description & connections',
       editLogic = 'Edit states & transitions',
       inspectQuest = 'Open source & compiler checks',
       inspectNpc = 'Open profile & compiler checks',
       moreActions = 'More actions',
       removeDraft = 'Remove draft…',
       removingDraft = 'Removing draft…',
       reviewRemovalBlockers = 'Review removal blockers',
       capabilityUnavailable = 'Not modeled yet',
       npcStoryUnavailable =
           'Quest and story relationships are not modeled for NPC drafts yet.',
       npcRoutineUnavailable =
           'Routine and world placement are not modeled yet.',
       npcInventoryUnavailable =
           'Inventory, equipment, and trading are not modeled yet.',
       npcDialogVoiceUnavailable =
           'Dialog, localization, and voice relationships are not modeled for NPC drafts yet.',
       questDialogVoiceUnavailable =
           'Dialog, localization, and voice relationships are not modeled for Quest drafts yet.',
       noReferenceProblems = 'No unresolved project references',
       referenceProblemCount = _englishReferenceProblemCount,
       referenceScopeNotice =
           'Reference status only; this is not build or runtime readiness.',
       technicalDetails = 'Technical details',
       questKindLabel = 'Quest draft',
       npcKindLabel = 'NPC draft',
       questTitleLabel = 'Quest title',
       npcDisplayNameLabel = 'Character name',
       technicalIdLabel = 'Technical ID',
       objectivesLabel = 'Objectives',
       uniqueNameLabel = 'Unique name',
       moduleNamespaceLabel = 'Module namespace',
       questGiverLabel = 'Quest giver',
       runtimeParentLabel = 'Runtime parent',
       logicDescription =
           'Quest lifecycle states, triggers, conditions, and effects are edited as one exact-current atomic operation.',
       outgoingHeading = 'Outgoing',
       noOutgoingReferences = 'No projected references',
       incomingHeading = 'Incoming',
       noIncomingReferences = 'No incoming project references',
       semanticIdentityLabel = 'Semantic identity',
       originLabel = 'Origin',
       entityRevisionLabel = 'Entity revision',
       stableIdLabel = 'Stable ID',
       referenceResolvedLabel = 'Reference resolved',
       referenceUnresolvedLabel = 'Reference unresolved';

  final String draftBadge;
  final String buildBlockedBadge;
  final String runtimeUnqualifiedBadge;
  final String overviewTab;
  final String profileTab;
  final String storyTab;
  final String logicTab;
  final String routineTab;
  final String inventoryTab;
  final String dialogVoiceTab;
  final String referencesTab;
  final String problemsChecksTab;
  final String editOverview;
  final String editNpcProfile;
  final String npcDialogVoiceNextStepTitle;
  final String npcDialogVoiceNextStepDescription;
  final String continueToNpcDialogVoice;
  final String editStory;
  final String editLogic;
  final String inspectQuest;
  final String inspectNpc;
  final String moreActions;
  final String removeDraft;
  final String removingDraft;
  final String reviewRemovalBlockers;
  final String capabilityUnavailable;
  final String actionFailed;
  final String npcStoryUnavailable;
  final String npcRoutineUnavailable;
  final String npcInventoryUnavailable;
  final String npcDialogVoiceUnavailable;
  final String questDialogVoiceUnavailable;
  final String noReferenceProblems;
  final String Function(int count) referenceProblemCount;
  final String referenceScopeNotice;
  final String technicalDetails;
  final String questKindLabel;
  final String npcKindLabel;
  final String questTitleLabel;
  final String npcDisplayNameLabel;
  final String technicalIdLabel;
  final String objectivesLabel;
  final String uniqueNameLabel;
  final String moduleNamespaceLabel;
  final String questGiverLabel;
  final String runtimeParentLabel;
  final String logicDescription;
  final String outgoingHeading;
  final String noOutgoingReferences;
  final String incomingHeading;
  final String noIncomingReferences;
  final String semanticIdentityLabel;
  final String originLabel;
  final String entityRevisionLabel;
  final String stableIdLabel;
  final String referenceResolvedLabel;
  final String referenceUnresolvedLabel;
}

/// All navigation and mutation affordances exposed by the workbench.
///
/// Callers retain authority for each atomic action. Null edit/inspection
/// callbacks stay visible but disabled, while exact reference navigation is
/// always routed back through the owning content library.
@immutable
final class Revision3StoryEntityWorkbenchActions {
  const Revision3StoryEntityWorkbenchActions({
    required this.openEntity,
    required this.openAsset,
    this.editOverview,
    this.editNpcProfile,
    this.editStory,
    this.editLogic,
    this.inspectQuest,
    this.inspectNpc,
    this.editOverviewDisabledReason,
    this.editNpcProfileDisabledReason,
    this.editStoryDisabledReason,
    this.editLogicDisabledReason,
    this.inspectQuestDisabledReason,
    this.inspectNpcDisabledReason,
    this.removeDraft,
    this.reviewRemovalBlockers,
    this.removeDraftDisabledReason,
    this.removingDraft = false,
  });

  final ValueChanged<String> openEntity;
  final ValueChanged<String> openAsset;
  final Revision3StoryWorkbenchAction? editOverview;
  final Revision3StoryWorkbenchAction? editNpcProfile;
  final Revision3StoryWorkbenchAction? editStory;
  final Revision3StoryWorkbenchAction? editLogic;
  final Revision3StoryWorkbenchAction? inspectQuest;
  final Revision3StoryWorkbenchAction? inspectNpc;
  final String? editOverviewDisabledReason;
  final String? editNpcProfileDisabledReason;
  final String? editStoryDisabledReason;
  final String? editLogicDisabledReason;
  final String? inspectQuestDisabledReason;
  final String? inspectNpcDisabledReason;
  final Revision3StoryWorkbenchAction? removeDraft;
  final Revision3StoryWorkbenchAction? reviewRemovalBlockers;
  final String? removeDraftDisabledReason;
  final bool removingDraft;
}

enum _Revision3StoryWorkbenchMenuAction { removeDraft, reviewRemovalBlockers }

enum _Revision3QuestContextAction { overview, story, logic }

enum Revision3StoryWorkbenchSection {
  overview,
  profile,
  story,
  logic,
  routine,
  inventory,
  dialogVoice,
  references,
  problemsChecks,
}

/// A bounded authoring workbench for exact-current QuestDraft and NpcDraft
/// projections.
///
/// It does not create build, deployment, runtime, save, or game-installation
/// authority. Unmodeled capabilities remain visible and honestly disabled.
final class Revision3StoryEntityWorkbench extends StatefulWidget {
  Revision3StoryEntityWorkbench({
    required this.projectId,
    required this.index,
    required this.entity,
    required this.selectedSection,
    required this.onSectionChanged,
    required this.actions,
    this.questJourney,
    this.questTranscript,
    this.npcDialogVoice,
    this.copy = const Revision3StoryEntityWorkbenchCopy.english(),
    super.key,
  }) : assert(
         entity.kind == Revision3ContentEntityKind.questDraft ||
             entity.kind == Revision3ContentEntityKind.npcDraft,
       ),
       assert(projectId == index.projectId),
       assert(index.entityById(entity.id) == entity);

  final String projectId;
  final Revision3ContentIndex index;
  final Revision3ContentEntity entity;
  final Revision3StoryWorkbenchSection selectedSection;
  final ValueChanged<Revision3StoryWorkbenchSection> onSectionChanged;
  final Revision3StoryEntityWorkbenchActions actions;

  /// Friendly Quest-only journey supplied by the owning workspace.
  ///
  /// When present, this replaces the fragmented technical overview with one
  /// coherent read-only projection and contextual hand-offs to the existing
  /// exact editors. Content surfaces that have not adopted the journey retain
  /// the bounded overview below.
  final Widget? questJourney;

  /// Friendly Quest-only transcript editor supplied by the owning workspace.
  ///
  /// The workbench deliberately does not manufacture transcript authority.
  /// When no exact-current editor is supplied, the previous unavailable state
  /// remains visible.
  final Widget? questTranscript;

  /// Friendly NPC-only greeting and Voice editor supplied by the owning
  /// workspace. The workbench grants no publication or runtime authority when
  /// it hosts this widget; an absent editor keeps the bounded unavailable
  /// state visible.
  final Widget? npcDialogVoice;
  final Revision3StoryEntityWorkbenchCopy copy;

  static Revision3StoryWorkbenchSection defaultSectionFor(
    Revision3ContentEntity entity,
  ) => entity.kind == Revision3ContentEntityKind.questDraft
      ? Revision3StoryWorkbenchSection.overview
      : Revision3StoryWorkbenchSection.profile;

  /// Whether [section] is a valid incoming selection for [entity].
  ///
  /// Legacy Quest Story/Logic values remain accepted for API compatibility;
  /// the workbench normalizes both to the canonical Journey/Overview. Use
  /// [sectionsFor] for the tabs that should actually be presented to authors.
  static bool supportsSection(
    Revision3ContentEntity entity,
    Revision3StoryWorkbenchSection section,
  ) =>
      sectionsFor(entity).contains(section) ||
      (entity.kind == Revision3ContentEntityKind.questDraft &&
          (section == Revision3StoryWorkbenchSection.story ||
              section == Revision3StoryWorkbenchSection.logic));

  static List<Revision3StoryWorkbenchSection> sectionsFor(
    Revision3ContentEntity entity,
  ) => entity.kind == Revision3ContentEntityKind.questDraft
      ? const <Revision3StoryWorkbenchSection>[
          Revision3StoryWorkbenchSection.overview,
          Revision3StoryWorkbenchSection.dialogVoice,
          Revision3StoryWorkbenchSection.references,
          Revision3StoryWorkbenchSection.problemsChecks,
        ]
      : const <Revision3StoryWorkbenchSection>[
          Revision3StoryWorkbenchSection.profile,
          Revision3StoryWorkbenchSection.story,
          Revision3StoryWorkbenchSection.routine,
          Revision3StoryWorkbenchSection.inventory,
          Revision3StoryWorkbenchSection.dialogVoice,
          Revision3StoryWorkbenchSection.references,
          Revision3StoryWorkbenchSection.problemsChecks,
        ];

  @override
  State<Revision3StoryEntityWorkbench> createState() =>
      _Revision3StoryEntityWorkbenchState();
}

class _Revision3StoryEntityWorkbenchState
    extends State<Revision3StoryEntityWorkbench> {
  late Revision3StoryWorkbenchSection _section = _normalizedSection(
    widget.entity,
    widget.selectedSection,
  );
  _Revision3QuestContextAction? _activeQuestContextAction;
  var _questContextActionEpoch = 0;

  @override
  void didUpdateWidget(covariant Revision3StoryEntityWorkbench oldWidget) {
    super.didUpdateWidget(oldWidget);
    final selectedEntityChanged =
        oldWidget.projectId != widget.projectId ||
        oldWidget.entity.id != widget.entity.id ||
        oldWidget.entity.kind != widget.entity.kind;
    final actionAuthorityChanged =
        selectedEntityChanged ||
        !identical(oldWidget.index, widget.index) ||
        oldWidget.entity.revision != widget.entity.revision;
    if (actionAuthorityChanged) {
      _questContextActionEpoch++;
      _activeQuestContextAction = null;
    }
    if (selectedEntityChanged ||
        oldWidget.selectedSection != widget.selectedSection) {
      _section = _normalizedSection(widget.entity, widget.selectedSection);
    }
  }

  @override
  Widget build(BuildContext context) {
    final entity = widget.entity;
    final copy = widget.copy;
    final sections = Revision3StoryEntityWorkbench.sectionsFor(entity);
    return KeyedSubtree(
      key: ValueKey('revision3-content-entity-details-${entity.id}'),
      child: Column(
        key: const Key('revision3-content-entity-details'),
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          Padding(
            padding: const EdgeInsets.fromLTRB(20, 20, 20, 12),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Row(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Icon(_kindIcon(entity.kind), size: 36),
                    const SizedBox(width: 12),
                    Expanded(
                      child: Column(
                        crossAxisAlignment: CrossAxisAlignment.start,
                        children: [
                          Semantics(
                            header: true,
                            child: Text(
                              _entityTitle(entity),
                              style: Theme.of(context).textTheme.titleLarge,
                            ),
                          ),
                          Text(
                            entity.kind == Revision3ContentEntityKind.questDraft
                                ? copy.questKindLabel
                                : copy.npcKindLabel,
                          ),
                        ],
                      ),
                    ),
                    const SizedBox(width: 8),
                    PopupMenuButton<_Revision3StoryWorkbenchMenuAction>(
                      key: Key('revision3-story-workbench-more-${entity.id}'),
                      tooltip: copy.moreActions,
                      onSelected: (action) {
                        switch (action) {
                          case _Revision3StoryWorkbenchMenuAction.removeDraft:
                            widget.actions.removeDraft?.call();
                          case _Revision3StoryWorkbenchMenuAction
                              .reviewRemovalBlockers:
                            widget.actions.reviewRemovalBlockers?.call();
                        }
                      },
                      itemBuilder: (context) => [
                        PopupMenuItem<_Revision3StoryWorkbenchMenuAction>(
                          key: Key(
                            'revision3-story-workbench-remove-${entity.id}',
                          ),
                          value: _Revision3StoryWorkbenchMenuAction.removeDraft,
                          enabled: widget.actions.removeDraft != null,
                          child: Row(
                            crossAxisAlignment: CrossAxisAlignment.start,
                            children: [
                              const Icon(Icons.delete_outline),
                              const SizedBox(width: 12),
                              Expanded(
                                child: Column(
                                  crossAxisAlignment: CrossAxisAlignment.start,
                                  children: [
                                    Text(
                                      widget.actions.removingDraft
                                          ? copy.removingDraft
                                          : copy.removeDraft,
                                    ),
                                    if (widget.actions.removeDraft == null &&
                                        widget
                                                .actions
                                                .removeDraftDisabledReason !=
                                            null) ...[
                                      const SizedBox(height: 2),
                                      Text(
                                        widget
                                            .actions
                                            .removeDraftDisabledReason!,
                                        style: Theme.of(
                                          context,
                                        ).textTheme.bodySmall,
                                      ),
                                    ],
                                  ],
                                ),
                              ),
                            ],
                          ),
                        ),
                        if (widget.actions.reviewRemovalBlockers != null)
                          PopupMenuItem<_Revision3StoryWorkbenchMenuAction>(
                            key: Key(
                              'revision3-story-workbench-review-remove-blockers-${entity.id}',
                            ),
                            value: _Revision3StoryWorkbenchMenuAction
                                .reviewRemovalBlockers,
                            child: Row(
                              children: [
                                const Icon(Icons.link_off_outlined),
                                const SizedBox(width: 12),
                                Expanded(
                                  child: Text(copy.reviewRemovalBlockers),
                                ),
                              ],
                            ),
                          ),
                      ],
                    ),
                  ],
                ),
                const SizedBox(height: 12),
                Wrap(
                  key: Key('revision3-story-workbench-status-${entity.id}'),
                  spacing: 8,
                  runSpacing: 8,
                  children: [
                    _StatusBadge(
                      key: Key('revision3-story-workbench-draft-${entity.id}'),
                      icon: Icons.edit_note_outlined,
                      label: copy.draftBadge,
                    ),
                    _StatusBadge(
                      key: Key(
                        'revision3-story-workbench-build-blocked-${entity.id}',
                      ),
                      icon: Icons.block_outlined,
                      label: copy.buildBlockedBadge,
                    ),
                    _StatusBadge(
                      key: Key(
                        'revision3-story-workbench-runtime-unqualified-${entity.id}',
                      ),
                      icon: Icons.science_outlined,
                      label: copy.runtimeUnqualifiedBadge,
                    ),
                  ],
                ),
              ],
            ),
          ),
          const Divider(height: 1),
          SingleChildScrollView(
            key: Key('revision3-story-workbench-tabs-${entity.id}'),
            scrollDirection: Axis.horizontal,
            padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 8),
            child: Row(
              children: [
                for (final section in sections) ...[
                  ChoiceChip(
                    key: Key(
                      'revision3-story-workbench-tab-${section.name}-${entity.id}',
                    ),
                    label: Text(_sectionLabel(copy, section)),
                    selected: section == _section,
                    onSelected: (_) => _selectSection(section),
                  ),
                  const SizedBox(width: 8),
                ],
              ],
            ),
          ),
          const Divider(height: 1),
          Expanded(child: _buildSection(context)),
        ],
      ),
    );
  }

  void _selectSection(Revision3StoryWorkbenchSection section) {
    if (_section == section) return;
    setState(() => _section = section);
    widget.onSectionChanged(section);
  }

  Future<void> _runQuestContextAction(
    _Revision3QuestContextAction contextAction,
    Revision3StoryWorkbenchAction action,
    int authorityEpoch,
  ) async {
    if (_activeQuestContextAction != null ||
        authorityEpoch != _questContextActionEpoch) {
      return;
    }
    setState(() => _activeQuestContextAction = contextAction);
    try {
      await action();
    } on Object {
      if (mounted && authorityEpoch == _questContextActionEpoch) {
        ScaffoldMessenger.maybeOf(context)?.showSnackBar(
          SnackBar(
            content: Text(
              widget.copy.actionFailed,
              key: Key(
                'revision3-story-workbench-action-error-${widget.entity.id}',
              ),
            ),
          ),
        );
      }
    } finally {
      if (mounted && authorityEpoch == _questContextActionEpoch) {
        setState(() => _activeQuestContextAction = null);
      }
    }
  }

  Widget _buildSection(BuildContext context) {
    final entity = widget.entity;
    final sectionKey = Key(
      'revision3-story-workbench-section-${_section.name}-${entity.id}',
    );
    final journey = widget.questJourney;
    if (_section == Revision3StoryWorkbenchSection.overview &&
        entity.kind == Revision3ContentEntityKind.questDraft &&
        journey != null) {
      return KeyedSubtree(key: sectionKey, child: journey);
    }
    return ListView(
      key: sectionKey,
      padding: const EdgeInsets.all(20),
      children: switch (_section) {
        Revision3StoryWorkbenchSection.overview => _questOverview(context),
        Revision3StoryWorkbenchSection.profile => _npcProfile(context),
        Revision3StoryWorkbenchSection.story => _story(context),
        Revision3StoryWorkbenchSection.logic => _questLogic(context),
        Revision3StoryWorkbenchSection.routine => <Widget>[
          _UnavailableCapability(
            title: widget.copy.routineTab,
            description: widget.copy.npcRoutineUnavailable,
            fallback: widget.copy.capabilityUnavailable,
          ),
        ],
        Revision3StoryWorkbenchSection.inventory => <Widget>[
          _UnavailableCapability(
            title: widget.copy.inventoryTab,
            description: widget.copy.npcInventoryUnavailable,
            fallback: widget.copy.capabilityUnavailable,
          ),
        ],
        Revision3StoryWorkbenchSection.dialogVoice => <Widget>[
          if (entity.kind == Revision3ContentEntityKind.questDraft &&
              widget.questTranscript != null)
            widget.questTranscript!
          else if (entity.kind == Revision3ContentEntityKind.npcDraft &&
              widget.npcDialogVoice != null)
            widget.npcDialogVoice!
          else
            _UnavailableCapability(
              title: widget.copy.dialogVoiceTab,
              description: entity.kind == Revision3ContentEntityKind.questDraft
                  ? widget.copy.questDialogVoiceUnavailable
                  : widget.copy.npcDialogVoiceUnavailable,
              fallback: widget.copy.capabilityUnavailable,
            ),
        ],
        Revision3StoryWorkbenchSection.references => _references(context),
        Revision3StoryWorkbenchSection.problemsChecks => _problems(context),
      },
    );
  }

  List<Widget> _questOverview(BuildContext context) {
    final entity = widget.entity;
    final quest = entity.summary.questDraft!;
    final editOverview = widget.actions.editOverview;
    final editStory = widget.actions.editStory;
    final editLogic = widget.actions.editLogic;
    final actionEpoch = _questContextActionEpoch;
    final actionLaneBusy = _activeQuestContextAction != null;
    return <Widget>[
      _SectionHeading(widget.copy.overviewTab),
      _AtomicActionCard(
        key: Key('revision3-story-workbench-action-edit-overview-${entity.id}'),
        icon: Icons.format_list_bulleted_outlined,
        title: widget.copy.editOverview,
        unavailable:
            widget.actions.editOverviewDisabledReason ??
            widget.copy.capabilityUnavailable,
        busy:
            _activeQuestContextAction == _Revision3QuestContextAction.overview,
        blocked: actionLaneBusy,
        onPressed: editOverview == null
            ? null
            : () => _runQuestContextAction(
                _Revision3QuestContextAction.overview,
                editOverview,
                actionEpoch,
              ),
      ),
      const SizedBox(height: 12),
      _AtomicActionCard(
        key: Key('revision3-story-workbench-action-edit-story-${entity.id}'),
        icon: Icons.account_tree_outlined,
        title: widget.copy.editStory,
        unavailable:
            widget.actions.editStoryDisabledReason ??
            widget.copy.capabilityUnavailable,
        busy: _activeQuestContextAction == _Revision3QuestContextAction.story,
        blocked: actionLaneBusy,
        onPressed: editStory == null
            ? null
            : () => _runQuestContextAction(
                _Revision3QuestContextAction.story,
                editStory,
                actionEpoch,
              ),
      ),
      const SizedBox(height: 12),
      _AtomicActionCard(
        key: Key('revision3-story-workbench-action-edit-logic-${entity.id}'),
        icon: Icons.schema_outlined,
        title: widget.copy.editLogic,
        unavailable:
            widget.actions.editLogicDisabledReason ??
            widget.copy.capabilityUnavailable,
        busy: _activeQuestContextAction == _Revision3QuestContextAction.logic,
        blocked: actionLaneBusy,
        onPressed: editLogic == null
            ? null
            : () => _runQuestContextAction(
                _Revision3QuestContextAction.logic,
                editLogic,
                actionEpoch,
              ),
      ),
      const SizedBox(height: 12),
      _Fact(label: widget.copy.questTitleLabel, value: quest.title),
      _Fact(label: widget.copy.technicalIdLabel, value: quest.technicalId),
      _Fact(
        label: widget.copy.objectivesLabel,
        value: quest.objectiveTitles
            .asMap()
            .entries
            .map((entry) => '${entry.key + 1}. ${entry.value}')
            .join('\n'),
      ),
      const SizedBox(height: 4),
      _TechnicalDetails(entity: entity, copy: widget.copy),
    ];
  }

  List<Widget> _npcProfile(BuildContext context) {
    final entity = widget.entity;
    return <Widget>[
      _SectionHeading(widget.copy.profileTab),
      _AtomicActionCard(
        key: Key(
          'revision3-story-workbench-action-edit-npc-profile-${entity.id}',
        ),
        icon: Icons.edit_outlined,
        title: widget.copy.editNpcProfile,
        unavailable:
            widget.actions.editNpcProfileDisabledReason ??
            widget.copy.capabilityUnavailable,
        onPressed: widget.actions.editNpcProfile,
      ),
      const SizedBox(height: 12),
      _Fact(label: widget.copy.npcDisplayNameLabel, value: entity.displayName),
      if (widget.npcDialogVoice != null) ...[
        const SizedBox(height: 4),
        _NpcDialogVoiceNextStepCard(
          key: Key(
            'revision3-story-workbench-npc-dialog-next-step-${entity.id}',
          ),
          title: widget.copy.npcDialogVoiceNextStepTitle,
          description: widget.copy.npcDialogVoiceNextStepDescription,
          actionLabel: widget.copy.continueToNpcDialogVoice,
          onPressed: () =>
              _selectSection(Revision3StoryWorkbenchSection.dialogVoice),
        ),
        const SizedBox(height: 4),
      ],
      const SizedBox(height: 4),
      _TechnicalDetails(entity: entity, copy: widget.copy),
    ];
  }

  List<Widget> _story(BuildContext context) {
    final entity = widget.entity;
    if (entity.kind == Revision3ContentEntityKind.npcDraft) {
      return <Widget>[
        _UnavailableCapability(
          title: widget.copy.storyTab,
          description: widget.copy.npcStoryUnavailable,
          fallback: widget.copy.capabilityUnavailable,
        ),
      ];
    }
    final quest = entity.summary.questDraft!;
    return <Widget>[
      _SectionHeading(widget.copy.storyTab),
      _AtomicActionCard(
        key: Key('revision3-story-workbench-action-edit-story-${entity.id}'),
        icon: Icons.account_tree_outlined,
        title: widget.copy.editStory,
        unavailable:
            widget.actions.editStoryDisabledReason ??
            widget.copy.capabilityUnavailable,
        onPressed: widget.actions.editStory,
      ),
      const SizedBox(height: 12),
      _Fact(
        label: widget.copy.questGiverLabel,
        value: quest.giverRuntimeUniqueName,
      ),
      _Fact(
        label: widget.copy.runtimeParentLabel,
        value: quest.parentRuntimeClass,
      ),
      _Fact(
        label: widget.copy.moduleNamespaceLabel,
        value: quest.moduleNamespace,
      ),
    ];
  }

  List<Widget> _questLogic(BuildContext context) {
    final entity = widget.entity;
    return <Widget>[
      _SectionHeading(widget.copy.logicTab),
      Text(widget.copy.logicDescription),
      const SizedBox(height: 12),
      _AtomicActionCard(
        key: Key('revision3-story-workbench-action-edit-logic-${entity.id}'),
        icon: Icons.schema_outlined,
        title: widget.copy.editLogic,
        unavailable:
            widget.actions.editLogicDisabledReason ??
            widget.copy.capabilityUnavailable,
        onPressed: widget.actions.editLogic,
      ),
    ];
  }

  List<Widget> _references(BuildContext context) {
    final entity = widget.entity;
    final backlinks = widget.index.backlinksToEntity(entity.id);
    return <Widget>[
      _SectionHeading(widget.copy.referencesTab),
      Text(
        widget.copy.outgoingHeading,
        style: Theme.of(context).textTheme.titleSmall,
      ),
      const SizedBox(height: 8),
      if (entity.references.isEmpty && entity.assetReferences.isEmpty)
        Text(widget.copy.noOutgoingReferences)
      else ...[
        for (var index = 0; index < entity.references.length; index++)
          _EntityReferenceTile(
            key: Key(
              'revision3-story-workbench-outgoing-${entity.id}-${entity.references[index].role}-$index',
            ),
            index: widget.index,
            reference: entity.references[index],
            onOpen: widget.actions.openEntity,
            resolvedStatusLabel: widget.copy.referenceResolvedLabel,
            unresolvedStatusLabel: widget.copy.referenceUnresolvedLabel,
          ),
        for (var index = 0; index < entity.assetReferences.length; index++)
          _AssetReferenceTile(
            key: Key(
              'revision3-story-workbench-outgoing-asset-${entity.id}-${entity.assetReferences[index].role}-$index',
            ),
            index: widget.index,
            reference: entity.assetReferences[index],
            onOpen: widget.actions.openAsset,
            resolvedStatusLabel: widget.copy.referenceResolvedLabel,
            unresolvedStatusLabel: widget.copy.referenceUnresolvedLabel,
          ),
      ],
      const Divider(height: 28),
      Row(
        children: [
          Expanded(
            child: Text(
              widget.copy.incomingHeading,
              style: Theme.of(context).textTheme.titleSmall,
            ),
          ),
          Text('${backlinks.length}'),
        ],
      ),
      const SizedBox(height: 8),
      if (backlinks.isEmpty)
        Text(widget.copy.noIncomingReferences)
      else
        for (var index = 0; index < backlinks.length; index++)
          _WorkbenchReferenceTile(
            key: Key(
              'revision3-content-backlink-${entity.id}-${backlinks[index].source.id}-${backlinks[index].reference.role}-$index',
            ),
            icon: _kindIcon(backlinks[index].source.kind),
            title: _entityTitle(backlinks[index].source),
            subtitle:
                '${backlinks[index].reference.role.replaceAll('_', ' ')} / ${backlinks[index].source.kind.displayName}',
            ok:
                backlinks[index].reference.resolution ==
                Revision3ContentReferenceResolution.resolved,
            statusSemanticLabel:
                backlinks[index].reference.resolution ==
                    Revision3ContentReferenceResolution.resolved
                ? widget.copy.referenceResolvedLabel
                : widget.copy.referenceUnresolvedLabel,
            onTap: () => widget.actions.openEntity(backlinks[index].source.id),
          ),
    ];
  }

  List<Widget> _problems(BuildContext context) {
    final entity = widget.entity;
    final brokenEntityReferences = entity.references
        .where(
          (reference) =>
              reference.resolution !=
              Revision3ContentReferenceResolution.resolved,
        )
        .toList(growable: false);
    final brokenAssetReferences = entity.assetReferences
        .where(
          (reference) =>
              reference.resolution !=
              Revision3ContentAssetReferenceResolution.resolved,
        )
        .toList(growable: false);
    return <Widget>[
      _SectionHeading(widget.copy.problemsChecksTab),
      _AtomicActionCard(
        key: Key(
          'revision3-story-workbench-action-inspect-${entity.kind.wireName}-${entity.id}',
        ),
        icon: Icons.fact_check_outlined,
        title: entity.kind == Revision3ContentEntityKind.questDraft
            ? widget.copy.inspectQuest
            : widget.copy.inspectNpc,
        unavailable: entity.kind == Revision3ContentEntityKind.questDraft
            ? widget.actions.inspectQuestDisabledReason ??
                  widget.copy.capabilityUnavailable
            : widget.actions.inspectNpcDisabledReason ??
                  widget.copy.capabilityUnavailable,
        onPressed: entity.kind == Revision3ContentEntityKind.questDraft
            ? widget.actions.inspectQuest
            : widget.actions.inspectNpc,
      ),
      const SizedBox(height: 12),
      Semantics(
        liveRegion: true,
        child: Text(
          entity.problemCount == 0
              ? widget.copy.noReferenceProblems
              : widget.copy.referenceProblemCount(entity.problemCount),
          style: Theme.of(context).textTheme.titleSmall,
        ),
      ),
      if (entity.problemCount != 0) ...[
        const SizedBox(height: 8),
        for (final reference in brokenEntityReferences)
          _WorkbenchReferenceTile(
            icon: Icons.link_off_outlined,
            title: reference.role.replaceAll('_', ' '),
            subtitle:
                '${reference.resolution.wireName} / ${reference.target.expectedKind.displayName} / ${reference.target.entityId}',
            ok: false,
            statusSemanticLabel: widget.copy.referenceUnresolvedLabel,
          ),
        for (final reference in brokenAssetReferences)
          _WorkbenchReferenceTile(
            icon: Icons.inventory_2_outlined,
            title: reference.role.replaceAll('_', ' '),
            subtitle:
                '${reference.resolution.wireName} / ${reference.logicalName ?? reference.sha256} / ${reference.expectedMediaType}',
            ok: false,
            statusSemanticLabel: widget.copy.referenceUnresolvedLabel,
          ),
      ],
      const SizedBox(height: 12),
      Card(
        child: Padding(
          padding: const EdgeInsets.all(12),
          child: Row(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              const Icon(Icons.info_outline),
              const SizedBox(width: 10),
              Expanded(child: Text(widget.copy.referenceScopeNotice)),
            ],
          ),
        ),
      ),
    ];
  }
}

Revision3StoryWorkbenchSection _normalizedSection(
  Revision3ContentEntity entity,
  Revision3StoryWorkbenchSection candidate,
) {
  // Story and Logic used to be standalone Quest tabs. Their exact editors now
  // live behind contextual hand-offs in the canonical Journey/Overview, but
  // the enum values remain public so persisted and in-flight selections from
  // older callers can be accepted safely.
  if (entity.kind == Revision3ContentEntityKind.questDraft &&
      (candidate == Revision3StoryWorkbenchSection.story ||
          candidate == Revision3StoryWorkbenchSection.logic)) {
    return Revision3StoryWorkbenchSection.overview;
  }
  return Revision3StoryEntityWorkbench.supportsSection(entity, candidate)
      ? candidate
      : Revision3StoryEntityWorkbench.defaultSectionFor(entity);
}

String _sectionLabel(
  Revision3StoryEntityWorkbenchCopy copy,
  Revision3StoryWorkbenchSection section,
) => switch (section) {
  Revision3StoryWorkbenchSection.overview => copy.overviewTab,
  Revision3StoryWorkbenchSection.profile => copy.profileTab,
  Revision3StoryWorkbenchSection.story => copy.storyTab,
  Revision3StoryWorkbenchSection.logic => copy.logicTab,
  Revision3StoryWorkbenchSection.routine => copy.routineTab,
  Revision3StoryWorkbenchSection.inventory => copy.inventoryTab,
  Revision3StoryWorkbenchSection.dialogVoice => copy.dialogVoiceTab,
  Revision3StoryWorkbenchSection.references => copy.referencesTab,
  Revision3StoryWorkbenchSection.problemsChecks => copy.problemsChecksTab,
};

class _StatusBadge extends StatelessWidget {
  const _StatusBadge({required this.icon, required this.label, super.key});

  final IconData icon;
  final String label;

  @override
  Widget build(BuildContext context) => Chip(
    avatar: Icon(icon, size: 16),
    label: Text(label),
    visualDensity: VisualDensity.compact,
  );
}

class _SectionHeading extends StatelessWidget {
  const _SectionHeading(this.label);

  final String label;

  @override
  Widget build(BuildContext context) => Padding(
    padding: const EdgeInsets.only(bottom: 12),
    child: Semantics(
      header: true,
      child: Text(label, style: Theme.of(context).textTheme.titleMedium),
    ),
  );
}

class _AtomicActionCard extends StatelessWidget {
  const _AtomicActionCard({
    required this.icon,
    required this.title,
    required this.unavailable,
    required this.onPressed,
    this.busy = false,
    this.blocked = false,
    super.key,
  });

  final IconData icon;
  final String title;
  final String unavailable;
  final Revision3StoryWorkbenchAction? onPressed;
  final bool busy;
  final bool blocked;

  @override
  Widget build(BuildContext context) => Card(
    margin: EdgeInsets.zero,
    child: ListTile(
      leading: Icon(icon),
      title: Text(title),
      subtitle: onPressed == null ? Text(unavailable) : null,
      trailing: busy
          ? const SizedBox.square(
              dimension: 20,
              child: CircularProgressIndicator(strokeWidth: 2),
            )
          : const Icon(Icons.chevron_right),
      enabled: onPressed != null && !blocked,
      onTap: blocked ? null : onPressed,
    ),
  );
}

class _NpcDialogVoiceNextStepCard extends StatelessWidget {
  const _NpcDialogVoiceNextStepCard({
    required this.title,
    required this.description,
    required this.actionLabel,
    required this.onPressed,
    super.key,
  });

  final String title;
  final String description;
  final String actionLabel;
  final VoidCallback onPressed;

  @override
  Widget build(BuildContext context) => Card(
    margin: EdgeInsets.zero,
    child: Padding(
      padding: const EdgeInsets.all(16),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Row(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              const Icon(Icons.chat_bubble_outline),
              const SizedBox(width: 12),
              Expanded(
                child: Text(
                  title,
                  style: Theme.of(context).textTheme.titleSmall,
                ),
              ),
            ],
          ),
          const SizedBox(height: 8),
          Text(description),
          const SizedBox(height: 12),
          SizedBox(
            width: double.infinity,
            child: FilledButton(
              onPressed: onPressed,
              child: Padding(
                padding: const EdgeInsets.symmetric(vertical: 4),
                child: Text(actionLabel, textAlign: TextAlign.center),
              ),
            ),
          ),
        ],
      ),
    ),
  );
}

class _UnavailableCapability extends StatelessWidget {
  const _UnavailableCapability({
    required this.title,
    required this.description,
    required this.fallback,
  });

  final String title;
  final String description;
  final String fallback;

  @override
  Widget build(BuildContext context) => Card(
    key: ValueKey('revision3-story-workbench-unavailable-$title'),
    child: Padding(
      padding: const EdgeInsets.all(16),
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          const Icon(Icons.lock_outline),
          const SizedBox(width: 12),
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(title, style: Theme.of(context).textTheme.titleSmall),
                const SizedBox(height: 4),
                Text(description.isEmpty ? fallback : description),
              ],
            ),
          ),
        ],
      ),
    ),
  );
}

class _TechnicalDetails extends StatelessWidget {
  const _TechnicalDetails({required this.entity, required this.copy});

  final Revision3ContentEntity entity;
  final Revision3StoryEntityWorkbenchCopy copy;

  @override
  Widget build(BuildContext context) => ExpansionTile(
    key: Key('revision3-story-workbench-technical-${entity.id}'),
    tilePadding: EdgeInsets.zero,
    title: Text(copy.technicalDetails),
    children: [
      if (entity.kind == Revision3ContentEntityKind.npcDraft) ...[
        _Fact(
          label: copy.uniqueNameLabel,
          value: entity.summary.primaryIdentity,
        ),
        _Fact(
          label: copy.moduleNamespaceLabel,
          value: entity.summary.secondaryText,
        ),
      ] else
        _Fact(
          label: copy.semanticIdentityLabel,
          value: entity.summary.primaryIdentity,
        ),
      _Fact(
        label: copy.originLabel,
        value: '${entity.origin.type}: ${entity.origin.label}',
      ),
      _Fact(label: copy.entityRevisionLabel, value: '${entity.revision}'),
      _Fact(label: copy.stableIdLabel, value: entity.id, selectable: true),
    ],
  );
}

class _Fact extends StatelessWidget {
  const _Fact({
    required this.label,
    required this.value,
    this.selectable = false,
  });

  final String label;
  final String value;
  final bool selectable;

  @override
  Widget build(BuildContext context) => Padding(
    padding: const EdgeInsets.only(bottom: 10),
    child: Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Text(label, style: Theme.of(context).textTheme.labelMedium),
        const SizedBox(height: 2),
        if (selectable) SelectableText(value) else Text(value),
      ],
    ),
  );
}

class _EntityReferenceTile extends StatelessWidget {
  const _EntityReferenceTile({
    required this.index,
    required this.reference,
    required this.onOpen,
    required this.resolvedStatusLabel,
    required this.unresolvedStatusLabel,
    super.key,
  });

  final Revision3ContentIndex index;
  final Revision3ContentReference reference;
  final ValueChanged<String> onOpen;
  final String resolvedStatusLabel;
  final String unresolvedStatusLabel;

  @override
  Widget build(BuildContext context) {
    final resolved =
        reference.resolution == Revision3ContentReferenceResolution.resolved;
    final locallyNavigable =
        resolved &&
        reference.target.projectId == index.projectId &&
        index.entityById(reference.target.entityId) != null;
    return _WorkbenchReferenceTile(
      icon: resolved ? Icons.link : Icons.link_off,
      title: reference.role.replaceAll('_', ' '),
      subtitle:
          '${reference.target.expectedKind.displayName} / ${reference.target.entityId}${reference.qualifier == null ? '' : ' / ${reference.qualifier}'}',
      ok: resolved,
      statusSemanticLabel: resolved
          ? resolvedStatusLabel
          : unresolvedStatusLabel,
      onTap: locallyNavigable ? () => onOpen(reference.target.entityId) : null,
    );
  }
}

class _AssetReferenceTile extends StatelessWidget {
  const _AssetReferenceTile({
    required this.index,
    required this.reference,
    required this.onOpen,
    required this.resolvedStatusLabel,
    required this.unresolvedStatusLabel,
    super.key,
  });

  final Revision3ContentIndex index;
  final Revision3ContentAssetReference reference;
  final ValueChanged<String> onOpen;
  final String resolvedStatusLabel;
  final String unresolvedStatusLabel;

  @override
  Widget build(BuildContext context) {
    final resolved =
        reference.resolution ==
        Revision3ContentAssetReferenceResolution.resolved;
    final locallyNavigable =
        resolved && index.assetBySha256(reference.sha256) != null;
    return _WorkbenchReferenceTile(
      icon: Icons.inventory_2_outlined,
      title: reference.role.replaceAll('_', ' '),
      subtitle: reference.logicalName ?? reference.sha256,
      ok: resolved,
      statusSemanticLabel: resolved
          ? resolvedStatusLabel
          : unresolvedStatusLabel,
      onTap: locallyNavigable ? () => onOpen(reference.sha256) : null,
    );
  }
}

class _WorkbenchReferenceTile extends StatelessWidget {
  const _WorkbenchReferenceTile({
    required this.icon,
    required this.title,
    required this.subtitle,
    required this.ok,
    required this.statusSemanticLabel,
    this.onTap,
    super.key,
  });

  final IconData icon;
  final String title;
  final String subtitle;
  final bool ok;
  final String statusSemanticLabel;
  final VoidCallback? onTap;

  @override
  Widget build(BuildContext context) => ListTile(
    contentPadding: EdgeInsets.zero,
    leading: Icon(icon),
    title: Text(title),
    subtitle: Text(subtitle),
    trailing: Icon(
      ok ? Icons.check_circle_outline : Icons.error_outline,
      semanticLabel: statusSemanticLabel,
      color: ok
          ? Theme.of(context).colorScheme.primary
          : Theme.of(context).colorScheme.error,
    ),
    enabled: onTap != null,
    onTap: onTap,
  );
}

String _entityTitle(Revision3ContentEntity entity) => entity.displayName.isEmpty
    ? entity.summary.primaryIdentity
    : entity.displayName;

IconData _kindIcon(Revision3ContentEntityKind kind) => switch (kind) {
  Revision3ContentEntityKind.questDraft => Icons.assignment_outlined,
  Revision3ContentEntityKind.npcDraft => Icons.person_outline,
  Revision3ContentEntityKind.localizationEntry => Icons.translate,
  Revision3ContentEntityKind.dialogLine => Icons.chat_bubble_outline,
  Revision3ContentEntityKind.voiceSlot => Icons.mic_none,
  Revision3ContentEntityKind.voiceTake => Icons.graphic_eq,
  Revision3ContentEntityKind.scriptModule => Icons.code,
};
