import 'dart:async';

import 'package:flutter/material.dart';

import 'revision3_content_index.dart';
import 'revision3_voice_production_queue.dart';

typedef Revision3VoiceQueueAddLanguage =
    FutureOr<void> Function(
      String choiceStableKey,
      String locale,
      Revision3VoiceMissingLanguageQueueItem item,
    );

typedef Revision3VoiceQueueVoiceAction =
    FutureOr<void> Function(
      String lineId,
      String locale,
      Revision3VoiceSlotQueueItem item,
    );

typedef Revision3VoiceQueueDisabledReason =
    String? Function(Revision3VoiceProductionQueueItem item);

enum Revision3VoiceProductionQueueFilter {
  all,
  needsAction,
  missingLanguages,
  voice,
  complete,
}

/// Author-facing copy for [Revision3VoiceProductionQueueView].
///
/// The English defaults keep the view directly reusable in fixtures. The
/// managed workspace should replace this object from its localization layer.
@immutable
final class Revision3VoiceProductionQueueCopy {
  const Revision3VoiceProductionQueueCopy({
    this.title = 'Work list',
    this.description =
        'See what each project text or recording needs next, then continue in one click.',
    this.searchLabel = 'Search texts and languages',
    this.clearSearchTooltip = 'Clear search',
    this.filterAllLabel = 'All',
    this.filterNeedsActionLabel = 'Needs action',
    this.filterMissingLanguagesLabel = 'Languages',
    this.filterVoiceLabel = 'Recordings',
    this.filterCompleteLabel = 'Decisions complete',
    this.oneItemTemplate = '{count} item',
    this.manyItemsTemplate = '{count} items',
    this.needsActionTemplate = '{count} need action',
    this.completeTemplate = '{count} decisions complete',
    this.showingTemplate = 'Showing {visible} of {total}',
    this.partialTitle = 'The work list is limited',
    this.partialDescriptionTemplate =
        '{count} more items are not shown. Work through the visible items, then refresh the list.',
    this.voiceUnavailableTitle = 'Recording work could not be checked',
    this.voiceUnavailableDescription =
        'Language work is shown, but existing recording work is temporarily unavailable. Refresh the project before relying on the count.',
    this.emptyTitle = 'Nothing needs organizing yet',
    this.emptyDescription =
        'Project texts and existing Voice setups will appear here when they are available.',
    this.filteredEmptyTitle = 'No matching work',
    this.filteredEmptyDescription =
        'Try another search or choose a different filter.',
    this.missingLanguageKindLabel = 'Language not added',
    this.voiceKindLabel = 'Voice production',
    this.languageTemplate = 'Language: {locale}',
    this.nextStepLabel = 'Next step',
    this.addLanguageTitle = 'Add this language',
    this.addLanguageDescription =
        'This project language has not been added to the selected text.',
    this.addRecordingTitle = 'Add a recording',
    this.addRecordingDescription =
        'This Voice setup exists but does not have a recording yet.',
    this.reviewAndApproveTitle = 'Review and approve a recording',
    this.reviewAndApproveDescription =
        'Listen to the available recordings and approve the one you want to use.',
    this.selectOrRepairTitle = 'Choose an approved recording',
    this.selectOrRepairDescription =
        'Select an approved recording, or repair the current selection.',
    this.resolveTargetTitle = 'Resolve the Voice target',
    this.resolveTargetDescription =
        'Connect this recording choice to one unambiguous installed Voice target.',
    this.productionCompleteTitle = 'Production decisions complete',
    this.productionCompleteDescription =
        'The recording choice and target are set. Review Problems & Checks when you want to verify the project.',
    this.addLanguageActionLabel = 'Add language',
    this.addRecordingActionLabel = 'Add recording',
    this.reviewAndApproveActionLabel = 'Review recordings',
    this.selectOrRepairActionLabel = 'Manage recordings',
    this.resolveTargetActionLabel = 'Resolve target',
    this.reviewChecksActionLabel = 'Review checks',
    this.oneRecordingTemplate = '{count} recording',
    this.manyRecordingsTemplate = '{count} recordings',
    this.approvedTemplate = '{count} approved',
    this.noSelectionLabel = 'No recording selected',
    this.selectionNeedsAttentionLabel = 'Selection needs attention',
    this.approvedSelectionLabel = 'Approved recording selected',
    this.targetUnresolvedLabel = 'Target unresolved',
    this.targetAmbiguousLabel = 'Target ambiguous',
    this.targetResolvedLabel = 'Target resolved',
    this.unreviewedAlternativesLabel =
        'Other recordings are still available to review.',
    this.busyLabel = 'Finishing the current Voice action…',
    this.busyDisabledReason = 'Wait for the current Voice action to finish.',
    this.actionUnavailableReason =
        'This action is not available in the current project context.',
    this.actionFailedMessage =
        'The Voice action could not be completed. Refresh the project and try again.',
  });

  static const german = Revision3VoiceProductionQueueCopy(
    title: 'Arbeitsliste',
    description:
        'Sieh auf einen Blick, was jeder Projekttext oder jede Sprachaufnahme als Nächstes braucht.',
    searchLabel: 'Dialogtexte und Sprachen durchsuchen',
    clearSearchTooltip: 'Suche löschen',
    filterAllLabel: 'Alle',
    filterNeedsActionLabel: 'Handlungsbedarf',
    filterMissingLanguagesLabel: 'Sprachen',
    filterVoiceLabel: 'Aufnahmen',
    filterCompleteLabel: 'Entscheidungen abgeschlossen',
    oneItemTemplate: '{count} Eintrag',
    manyItemsTemplate: '{count} Einträge',
    needsActionTemplate: '{count} offen',
    completeTemplate: '{count} Entscheidungen abgeschlossen',
    showingTemplate: '{visible} von {total} werden angezeigt',
    partialTitle: 'Die Arbeitsliste ist begrenzt',
    partialDescriptionTemplate:
        '{count} weitere Einträge werden nicht angezeigt. Bearbeite zuerst die sichtbaren Einträge und aktualisiere danach die Liste.',
    voiceUnavailableTitle: 'Aufnahmearbeit konnte nicht geprüft werden',
    voiceUnavailableDescription:
        'Spracharbeit wird angezeigt, aber vorhandene Aufnahmen sind vorübergehend nicht verfügbar. Aktualisiere das Projekt, bevor du dich auf die Anzahl verlässt.',
    emptyTitle: 'Noch gibt es nichts zu organisieren',
    emptyDescription:
        'Projekttexte und vorhandene Voice-Einrichtungen erscheinen hier, sobald sie verfügbar sind.',
    filteredEmptyTitle: 'Keine passende Arbeit gefunden',
    filteredEmptyDescription:
        'Versuche eine andere Suche oder wähle einen anderen Filter.',
    missingLanguageKindLabel: 'Sprache nicht hinzugefügt',
    voiceKindLabel: 'Voice-Produktion',
    languageTemplate: 'Sprache: {locale}',
    nextStepLabel: 'Nächster Schritt',
    addLanguageTitle: 'Diese Sprache hinzufügen',
    addLanguageDescription:
        'Diese Projektsprache wurde dem ausgewählten Text noch nicht hinzugefügt.',
    addRecordingTitle: 'Eine Aufnahme hinzufügen',
    addRecordingDescription:
        'Diese Voice-Einrichtung ist vorhanden, enthält aber noch keine Aufnahme.',
    reviewAndApproveTitle: 'Eine Aufnahme prüfen und freigeben',
    reviewAndApproveDescription:
        'Höre die vorhandenen Aufnahmen an und gib die gewünschte Aufnahme frei.',
    selectOrRepairTitle: 'Eine freigegebene Aufnahme auswählen',
    selectOrRepairDescription:
        'Wähle eine freigegebene Aufnahme aus oder korrigiere die aktuelle Auswahl.',
    resolveTargetTitle: 'Das Voice-Ziel auflösen',
    resolveTargetDescription:
        'Verbinde die Aufnahmeauswahl eindeutig mit einem installierten Voice-Ziel.',
    productionCompleteTitle: 'Produktionsentscheidungen abgeschlossen',
    productionCompleteDescription:
        'Aufnahmeauswahl und Ziel sind festgelegt. Unter „Probleme & Prüfungen“ kannst du das Projekt anschließend prüfen.',
    addLanguageActionLabel: 'Sprache hinzufügen',
    addRecordingActionLabel: 'Aufnahme hinzufügen',
    reviewAndApproveActionLabel: 'Aufnahmen prüfen',
    selectOrRepairActionLabel: 'Aufnahmen verwalten',
    resolveTargetActionLabel: 'Ziel auflösen',
    reviewChecksActionLabel: 'Prüfungen ansehen',
    oneRecordingTemplate: '{count} Aufnahme',
    manyRecordingsTemplate: '{count} Aufnahmen',
    approvedTemplate: '{count} freigegeben',
    noSelectionLabel: 'Keine Aufnahme ausgewählt',
    selectionNeedsAttentionLabel: 'Auswahl muss geprüft werden',
    approvedSelectionLabel: 'Freigegebene Aufnahme ausgewählt',
    targetUnresolvedLabel: 'Ziel nicht aufgelöst',
    targetAmbiguousLabel: 'Ziel ist mehrdeutig',
    targetResolvedLabel: 'Ziel aufgelöst',
    unreviewedAlternativesLabel:
        'Weitere Aufnahmen können noch geprüft werden.',
    busyLabel: 'Die aktuelle Voice-Aktion wird abgeschlossen …',
    busyDisabledReason:
        'Warte, bis die aktuelle Voice-Aktion abgeschlossen ist.',
    actionUnavailableReason:
        'Diese Aktion ist im aktuellen Projektkontext nicht verfügbar.',
    actionFailedMessage:
        'Die Voice-Aktion konnte nicht abgeschlossen werden. Aktualisiere das Projekt und versuche es erneut.',
  );

  final String title;
  final String description;
  final String searchLabel;
  final String clearSearchTooltip;
  final String filterAllLabel;
  final String filterNeedsActionLabel;
  final String filterMissingLanguagesLabel;
  final String filterVoiceLabel;
  final String filterCompleteLabel;
  final String oneItemTemplate;
  final String manyItemsTemplate;
  final String needsActionTemplate;
  final String completeTemplate;
  final String showingTemplate;
  final String partialTitle;
  final String partialDescriptionTemplate;
  final String voiceUnavailableTitle;
  final String voiceUnavailableDescription;
  final String emptyTitle;
  final String emptyDescription;
  final String filteredEmptyTitle;
  final String filteredEmptyDescription;
  final String missingLanguageKindLabel;
  final String voiceKindLabel;
  final String languageTemplate;
  final String nextStepLabel;
  final String addLanguageTitle;
  final String addLanguageDescription;
  final String addRecordingTitle;
  final String addRecordingDescription;
  final String reviewAndApproveTitle;
  final String reviewAndApproveDescription;
  final String selectOrRepairTitle;
  final String selectOrRepairDescription;
  final String resolveTargetTitle;
  final String resolveTargetDescription;
  final String productionCompleteTitle;
  final String productionCompleteDescription;
  final String addLanguageActionLabel;
  final String addRecordingActionLabel;
  final String reviewAndApproveActionLabel;
  final String selectOrRepairActionLabel;
  final String resolveTargetActionLabel;
  final String reviewChecksActionLabel;
  final String oneRecordingTemplate;
  final String manyRecordingsTemplate;
  final String approvedTemplate;
  final String noSelectionLabel;
  final String selectionNeedsAttentionLabel;
  final String approvedSelectionLabel;
  final String targetUnresolvedLabel;
  final String targetAmbiguousLabel;
  final String targetResolvedLabel;
  final String unreviewedAlternativesLabel;
  final String busyLabel;
  final String busyDisabledReason;
  final String actionUnavailableReason;
  final String actionFailedMessage;

  String itemCount(int count) =>
      _countTemplate(count == 1 ? oneItemTemplate : manyItemsTemplate, count);

  String needsActionCount(int count) =>
      _countTemplate(needsActionTemplate, count);

  String completeCount(int count) => _countTemplate(completeTemplate, count);

  String showing({required int visible, required int total}) => showingTemplate
      .replaceAll('{visible}', '$visible')
      .replaceAll('{total}', '$total');

  String partialDescription(int count) =>
      _countTemplate(partialDescriptionTemplate, count);

  String language(String locale) =>
      languageTemplate.replaceAll('{locale}', locale);

  String recordingCount(int count) => _countTemplate(
    count == 1 ? oneRecordingTemplate : manyRecordingsTemplate,
    count,
  );

  String approvedCount(int count) => _countTemplate(approvedTemplate, count);

  String filterLabel(Revision3VoiceProductionQueueFilter filter) =>
      switch (filter) {
        Revision3VoiceProductionQueueFilter.all => filterAllLabel,
        Revision3VoiceProductionQueueFilter.needsAction =>
          filterNeedsActionLabel,
        Revision3VoiceProductionQueueFilter.missingLanguages =>
          filterMissingLanguagesLabel,
        Revision3VoiceProductionQueueFilter.voice => filterVoiceLabel,
        Revision3VoiceProductionQueueFilter.complete => filterCompleteLabel,
      };

  String kindLabel(Revision3VoiceProductionQueueItemKind kind) =>
      switch (kind) {
        Revision3VoiceProductionQueueItemKind.missingLanguage =>
          missingLanguageKindLabel,
        Revision3VoiceProductionQueueItemKind.voiceSlot => voiceKindLabel,
      };

  String stepTitle(Revision3VoiceProductionNextStep step) => switch (step) {
    Revision3VoiceProductionNextStep.addLanguage => addLanguageTitle,
    Revision3VoiceProductionNextStep.addRecording => addRecordingTitle,
    Revision3VoiceProductionNextStep.reviewAndApprove => reviewAndApproveTitle,
    Revision3VoiceProductionNextStep.selectOrRepair => selectOrRepairTitle,
    Revision3VoiceProductionNextStep.resolveTarget => resolveTargetTitle,
    Revision3VoiceProductionNextStep.productionDecisionsComplete =>
      productionCompleteTitle,
  };

  String stepDescription(
    Revision3VoiceProductionNextStep step,
  ) => switch (step) {
    Revision3VoiceProductionNextStep.addLanguage => addLanguageDescription,
    Revision3VoiceProductionNextStep.addRecording => addRecordingDescription,
    Revision3VoiceProductionNextStep.reviewAndApprove =>
      reviewAndApproveDescription,
    Revision3VoiceProductionNextStep.selectOrRepair =>
      selectOrRepairDescription,
    Revision3VoiceProductionNextStep.resolveTarget => resolveTargetDescription,
    Revision3VoiceProductionNextStep.productionDecisionsComplete =>
      productionCompleteDescription,
  };

  String actionLabel(Revision3VoiceProductionNextStep step) => switch (step) {
    Revision3VoiceProductionNextStep.addLanguage => addLanguageActionLabel,
    Revision3VoiceProductionNextStep.addRecording => addRecordingActionLabel,
    Revision3VoiceProductionNextStep.reviewAndApprove =>
      reviewAndApproveActionLabel,
    Revision3VoiceProductionNextStep.selectOrRepair =>
      selectOrRepairActionLabel,
    Revision3VoiceProductionNextStep.resolveTarget => resolveTargetActionLabel,
    Revision3VoiceProductionNextStep.productionDecisionsComplete =>
      reviewChecksActionLabel,
  };

  String _countTemplate(String template, int count) =>
      template.replaceAll('{count}', '$count');
}

/// Friendly, responsive presentation of the next authoring action for every
/// retained localization/Voice work item.
///
/// This widget deliberately owns no project checkpoint or mutation authority.
/// The host supplies exact callbacks and revalidates their identities. A
/// callback receives both the visible item and its exact hidden line/text key.
final class Revision3VoiceProductionQueueView extends StatefulWidget {
  const Revision3VoiceProductionQueueView({
    required this.queue,
    this.copy = const Revision3VoiceProductionQueueCopy(),
    this.busy = false,
    this.onAddLanguage,
    this.onAddRecording,
    this.onReviewAndApprove,
    this.onSelectOrRepair,
    this.onResolveTarget,
    this.onReviewChecks,
    this.disabledReasonFor,
    super.key,
  });

  final Revision3VoiceProductionQueue queue;
  final Revision3VoiceProductionQueueCopy copy;
  final bool busy;
  final Revision3VoiceQueueAddLanguage? onAddLanguage;
  final Revision3VoiceQueueVoiceAction? onAddRecording;
  final Revision3VoiceQueueVoiceAction? onReviewAndApprove;
  final Revision3VoiceQueueVoiceAction? onSelectOrRepair;
  final Revision3VoiceQueueVoiceAction? onResolveTarget;
  final Revision3VoiceQueueVoiceAction? onReviewChecks;
  final Revision3VoiceQueueDisabledReason? disabledReasonFor;

  @override
  State<Revision3VoiceProductionQueueView> createState() =>
      _Revision3VoiceProductionQueueViewState();
}

final class _Revision3VoiceProductionQueueViewState
    extends State<Revision3VoiceProductionQueueView> {
  final TextEditingController _search = TextEditingController();
  Revision3VoiceProductionQueueFilter _filter =
      Revision3VoiceProductionQueueFilter.all;
  bool _invoking = false;
  String? _actionFailure;

  bool get _busy => widget.busy || _invoking;

  @override
  void initState() {
    super.initState();
    _search.addListener(_searchChanged);
  }

  @override
  void didUpdateWidget(covariant Revision3VoiceProductionQueueView oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.queue.projectId != widget.queue.projectId ||
        oldWidget.queue.projectRevision != widget.queue.projectRevision) {
      _actionFailure = null;
    }
  }

  @override
  void dispose() {
    _search
      ..removeListener(_searchChanged)
      ..dispose();
    super.dispose();
  }

  void _searchChanged() => setState(() {});

  @override
  Widget build(BuildContext context) => LayoutBuilder(
    builder: (context, constraints) {
      final compact = constraints.maxWidth < 620;
      final items = widget.queue.items
          .where(_matchesFilter)
          .where(_matchesSearch)
          .toList(growable: false);
      final actionCount = _actionCount(widget.queue);
      final completeCount = widget.queue.countFor(
        Revision3VoiceProductionNextStep.productionDecisionsComplete,
      );
      return Semantics(
        container: true,
        label: widget.copy.title,
        child: CustomScrollView(
          key: const Key('revision3-voice-production-queue'),
          keyboardDismissBehavior: ScrollViewKeyboardDismissBehavior.onDrag,
          slivers: [
            SliverToBoxAdapter(
              child: _bounded(
                _QueueHeader(
                  copy: widget.copy,
                  totalCount: widget.queue.totalItemCount,
                  actionCount: actionCount,
                  completeCount: completeCount,
                  compact: compact,
                ),
              ),
            ),
            SliverToBoxAdapter(
              child: _bounded(
                _QueueControls(
                  search: _search,
                  filter: _filter,
                  queue: widget.queue,
                  copy: widget.copy,
                  selectFilter: (filter) => setState(() => _filter = filter),
                ),
              ),
            ),
            SliverToBoxAdapter(
              child: _bounded(
                _QueueStatus(
                  copy: widget.copy,
                  busy: _busy,
                  visibleCount: items.length,
                  totalCount: widget.queue.totalItemCount,
                  actionFailure: _actionFailure,
                ),
              ),
            ),
            if (!widget.queue.voiceCatalogAvailable)
              SliverToBoxAdapter(
                child: _bounded(
                  _QueueVoiceUnavailableNotice(copy: widget.copy),
                ),
              ),
            if (widget.queue.isPartial)
              SliverToBoxAdapter(
                child: _bounded(
                  _QueuePartialNotice(
                    copy: widget.copy,
                    omittedCount: widget.queue.omittedItemCount,
                  ),
                ),
              ),
            if (widget.queue.items.isEmpty)
              SliverFillRemaining(
                hasScrollBody: false,
                child: _QueueEmpty(
                  icon: Icons.task_alt,
                  title: widget.copy.emptyTitle,
                  description: widget.copy.emptyDescription,
                ),
              )
            else if (items.isEmpty)
              SliverFillRemaining(
                hasScrollBody: false,
                child: _QueueEmpty(
                  icon: Icons.search_off,
                  title: widget.copy.filteredEmptyTitle,
                  description: widget.copy.filteredEmptyDescription,
                ),
              )
            else
              SliverPadding(
                padding: const EdgeInsets.fromLTRB(12, 4, 12, 20),
                sliver: SliverList.separated(
                  itemCount: items.length,
                  separatorBuilder: (_, _) => const SizedBox(height: 8),
                  itemBuilder: (context, index) => Center(
                    child: ConstrainedBox(
                      constraints: const BoxConstraints(maxWidth: 960),
                      child: _QueueItemCard(
                        item: items[index],
                        copy: widget.copy,
                        compact: compact,
                        action: _actionFor(items[index]),
                        disabledReason: _disabledReason(items[index]),
                        runAction: _runAction,
                      ),
                    ),
                  ),
                ),
              ),
          ],
        ),
      );
    },
  );

  Widget _bounded(Widget child) => Center(
    child: ConstrainedBox(
      constraints: const BoxConstraints(maxWidth: 984),
      child: child,
    ),
  );

  bool _matchesFilter(Revision3VoiceProductionQueueItem item) =>
      switch (_filter) {
        Revision3VoiceProductionQueueFilter.all => true,
        Revision3VoiceProductionQueueFilter.needsAction => item.isActionable,
        Revision3VoiceProductionQueueFilter.missingLanguages =>
          item.kind == Revision3VoiceProductionQueueItemKind.missingLanguage,
        Revision3VoiceProductionQueueFilter.voice =>
          item.kind == Revision3VoiceProductionQueueItemKind.voiceSlot,
        Revision3VoiceProductionQueueFilter.complete =>
          item.nextStep ==
              Revision3VoiceProductionNextStep.productionDecisionsComplete,
      };

  bool _matchesSearch(Revision3VoiceProductionQueueItem item) {
    final query = _search.text.trim().toLowerCase();
    if (query.isEmpty) return true;
    return <String>[
      item.displayLabel,
      item.locale,
      widget.copy.kindLabel(item.kind),
      widget.copy.stepTitle(item.nextStep),
    ].any((value) => value.toLowerCase().contains(query));
  }

  _QueueAction _actionFor(Revision3VoiceProductionQueueItem item) {
    final label = widget.copy.actionLabel(item.nextStep);
    final icon = switch (item.nextStep) {
      Revision3VoiceProductionNextStep.addLanguage => Icons.translate,
      Revision3VoiceProductionNextStep.addRecording => Icons.mic_none,
      Revision3VoiceProductionNextStep.reviewAndApprove =>
        Icons.rate_review_outlined,
      Revision3VoiceProductionNextStep.selectOrRepair =>
        Icons.library_music_outlined,
      Revision3VoiceProductionNextStep.resolveTarget => Icons.link,
      Revision3VoiceProductionNextStep.productionDecisionsComplete =>
        Icons.fact_check_outlined,
    };
    FutureOr<void> Function()? invoke;
    switch (item) {
      case Revision3VoiceMissingLanguageQueueItem():
        final callback = widget.onAddLanguage;
        if (item.nextStep == Revision3VoiceProductionNextStep.addLanguage &&
            callback != null) {
          invoke = () => callback(item.choiceStableKey, item.locale, item);
        }
      case Revision3VoiceSlotQueueItem():
        final callback = switch (item.nextStep) {
          Revision3VoiceProductionNextStep.addRecording =>
            widget.onAddRecording,
          Revision3VoiceProductionNextStep.reviewAndApprove =>
            widget.onReviewAndApprove,
          Revision3VoiceProductionNextStep.selectOrRepair =>
            widget.onSelectOrRepair,
          Revision3VoiceProductionNextStep.resolveTarget =>
            widget.onResolveTarget,
          Revision3VoiceProductionNextStep.productionDecisionsComplete =>
            widget.onReviewChecks,
          Revision3VoiceProductionNextStep.addLanguage => null,
        };
        if (callback != null) {
          invoke = () => callback(item.lineId, item.locale, item);
        }
    }
    return _QueueAction(label: label, icon: icon, invoke: invoke);
  }

  String? _disabledReason(Revision3VoiceProductionQueueItem item) {
    if (_busy) return widget.copy.busyDisabledReason;
    final supplied = widget.disabledReasonFor?.call(item);
    if (supplied != null && supplied.trim().isNotEmpty) return supplied;
    if (_actionFor(item).invoke == null) {
      return widget.copy.actionUnavailableReason;
    }
    return null;
  }

  Future<void> _runAction(FutureOr<void> Function() invoke) async {
    if (!mounted || _busy) return;
    setState(() {
      _invoking = true;
      _actionFailure = null;
    });
    try {
      await Future<void>.sync(invoke);
    } catch (_) {
      if (mounted) _actionFailure = widget.copy.actionFailedMessage;
    } finally {
      if (mounted) setState(() => _invoking = false);
    }
  }
}

final class _QueueHeader extends StatelessWidget {
  const _QueueHeader({
    required this.copy,
    required this.totalCount,
    required this.actionCount,
    required this.completeCount,
    required this.compact,
  });

  final Revision3VoiceProductionQueueCopy copy;
  final int totalCount;
  final int actionCount;
  final int completeCount;
  final bool compact;

  @override
  Widget build(BuildContext context) => Padding(
    padding: EdgeInsets.fromLTRB(16, compact ? 12 : 18, 16, 8),
    child: Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Row(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Icon(
              Icons.view_list_outlined,
              color: Theme.of(context).colorScheme.primary,
            ),
            const SizedBox(width: 10),
            Expanded(
              child: Text(
                copy.title,
                style: Theme.of(context).textTheme.titleLarge,
              ),
            ),
          ],
        ),
        const SizedBox(height: 4),
        Text(copy.description),
        const SizedBox(height: 10),
        Wrap(
          spacing: 7,
          runSpacing: 7,
          children: [
            _SummaryChip(label: copy.itemCount(totalCount)),
            _SummaryChip(label: copy.needsActionCount(actionCount)),
            _SummaryChip(label: copy.completeCount(completeCount)),
          ],
        ),
      ],
    ),
  );
}

final class _QueueControls extends StatelessWidget {
  const _QueueControls({
    required this.search,
    required this.filter,
    required this.queue,
    required this.copy,
    required this.selectFilter,
  });

  final TextEditingController search;
  final Revision3VoiceProductionQueueFilter filter;
  final Revision3VoiceProductionQueue queue;
  final Revision3VoiceProductionQueueCopy copy;
  final ValueChanged<Revision3VoiceProductionQueueFilter> selectFilter;

  @override
  Widget build(BuildContext context) => Padding(
    padding: const EdgeInsets.fromLTRB(12, 4, 12, 6),
    child: Column(
      children: [
        TextField(
          key: const Key('revision3-voice-production-queue-search'),
          controller: search,
          textInputAction: TextInputAction.search,
          decoration: InputDecoration(
            labelText: copy.searchLabel,
            prefixIcon: const Icon(Icons.search),
            suffixIcon: search.text.isEmpty
                ? null
                : IconButton(
                    key: const Key(
                      'revision3-voice-production-queue-clear-search',
                    ),
                    tooltip: copy.clearSearchTooltip,
                    onPressed: search.clear,
                    icon: const Icon(Icons.clear),
                  ),
            border: const OutlineInputBorder(),
            isDense: true,
          ),
        ),
        const SizedBox(height: 6),
        Align(
          alignment: Alignment.centerLeft,
          child: SingleChildScrollView(
            key: const Key('revision3-voice-production-queue-filters'),
            scrollDirection: Axis.horizontal,
            child: Row(
              children: [
                for (final candidate
                    in Revision3VoiceProductionQueueFilter.values) ...[
                  if (candidate != Revision3VoiceProductionQueueFilter.all)
                    const SizedBox(width: 6),
                  FilterChip(
                    key: Key(
                      'revision3-voice-production-queue-filter-${candidate.name}',
                    ),
                    selected: candidate == filter,
                    onSelected: (_) => selectFilter(candidate),
                    label: Text(
                      '${copy.filterLabel(candidate)} '
                      '(${_filterCount(queue, candidate)})',
                    ),
                  ),
                ],
              ],
            ),
          ),
        ),
      ],
    ),
  );
}

final class _QueueStatus extends StatelessWidget {
  const _QueueStatus({
    required this.copy,
    required this.busy,
    required this.visibleCount,
    required this.totalCount,
    required this.actionFailure,
  });

  final Revision3VoiceProductionQueueCopy copy;
  final bool busy;
  final int visibleCount;
  final int totalCount;
  final String? actionFailure;

  @override
  Widget build(BuildContext context) {
    final failure = actionFailure;
    return Padding(
      padding: const EdgeInsets.fromLTRB(14, 2, 14, 6),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          Semantics(
            key: const Key('revision3-voice-production-queue-status'),
            liveRegion: true,
            child: Row(
              children: [
                if (busy) ...[
                  const Icon(Icons.hourglass_top, size: 16),
                  const SizedBox(width: 8),
                ],
                Expanded(
                  child: Text(
                    busy
                        ? copy.busyLabel
                        : copy.showing(
                            visible: visibleCount,
                            total: totalCount,
                          ),
                    style: Theme.of(context).textTheme.bodySmall,
                  ),
                ),
              ],
            ),
          ),
          if (failure != null) ...[
            const SizedBox(height: 6),
            Semantics(
              key: const Key('revision3-voice-production-queue-action-failure'),
              liveRegion: true,
              child: Material(
                color: Theme.of(context).colorScheme.errorContainer,
                borderRadius: BorderRadius.circular(8),
                child: Padding(
                  padding: const EdgeInsets.all(9),
                  child: Text(failure),
                ),
              ),
            ),
          ],
        ],
      ),
    );
  }
}

final class _QueuePartialNotice extends StatelessWidget {
  const _QueuePartialNotice({required this.copy, required this.omittedCount});

  final Revision3VoiceProductionQueueCopy copy;
  final int omittedCount;

  @override
  Widget build(BuildContext context) => Padding(
    padding: const EdgeInsets.fromLTRB(12, 0, 12, 8),
    child: Semantics(
      key: const Key('revision3-voice-production-queue-partial'),
      container: true,
      child: Material(
        color: Theme.of(context).colorScheme.tertiaryContainer,
        borderRadius: BorderRadius.circular(10),
        child: Padding(
          padding: const EdgeInsets.all(10),
          child: Row(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              const Icon(Icons.info_outline, size: 20),
              const SizedBox(width: 8),
              Expanded(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Text(
                      copy.partialTitle,
                      style: Theme.of(context).textTheme.labelLarge,
                    ),
                    Text(copy.partialDescription(omittedCount)),
                  ],
                ),
              ),
            ],
          ),
        ),
      ),
    ),
  );
}

final class _QueueVoiceUnavailableNotice extends StatelessWidget {
  const _QueueVoiceUnavailableNotice({required this.copy});

  final Revision3VoiceProductionQueueCopy copy;

  @override
  Widget build(BuildContext context) => Padding(
    padding: const EdgeInsets.fromLTRB(12, 0, 12, 8),
    child: Semantics(
      key: const Key('revision3-voice-production-queue-voice-unavailable'),
      container: true,
      liveRegion: true,
      child: Material(
        color: Theme.of(context).colorScheme.errorContainer,
        borderRadius: BorderRadius.circular(10),
        child: Padding(
          padding: const EdgeInsets.all(10),
          child: Row(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              const Icon(Icons.sync_problem_outlined, size: 20),
              const SizedBox(width: 8),
              Expanded(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Text(
                      copy.voiceUnavailableTitle,
                      style: Theme.of(context).textTheme.labelLarge,
                    ),
                    Text(copy.voiceUnavailableDescription),
                  ],
                ),
              ),
            ],
          ),
        ),
      ),
    ),
  );
}

final class _QueueItemCard extends StatelessWidget {
  const _QueueItemCard({
    required this.item,
    required this.copy,
    required this.compact,
    required this.action,
    required this.disabledReason,
    required this.runAction,
  });

  final Revision3VoiceProductionQueueItem item;
  final Revision3VoiceProductionQueueCopy copy;
  final bool compact;
  final _QueueAction action;
  final String? disabledReason;
  final Future<void> Function(FutureOr<void> Function()) runAction;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final enabled = disabledReason == null && action.invoke != null;
    final button = FilledButton(
      key: ValueKey('revision3-voice-production-queue-action-${item.key}'),
      onPressed: enabled ? () => unawaited(runAction(action.invoke!)) : null,
      child: Padding(
        padding: const EdgeInsets.symmetric(vertical: 2),
        child: Row(
          mainAxisSize: compact ? MainAxisSize.max : MainAxisSize.min,
          mainAxisAlignment: MainAxisAlignment.center,
          children: [
            Icon(action.icon, size: 18),
            const SizedBox(width: 7),
            if (compact)
              Expanded(child: Text(action.label, textAlign: TextAlign.center))
            else
              Flexible(child: Text(action.label, textAlign: TextAlign.center)),
          ],
        ),
      ),
    );
    return Semantics(
      key: ValueKey('revision3-voice-production-queue-item-${item.key}'),
      container: true,
      label:
          '${copy.kindLabel(item.kind)}. ${item.displayLabel}. '
          '${copy.language(item.locale)}. ${copy.nextStepLabel}: '
          '${copy.stepTitle(item.nextStep)}.',
      child: Card(
        margin: EdgeInsets.zero,
        child: Padding(
          padding: EdgeInsets.all(compact ? 12 : 14),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: [
              Row(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  CircleAvatar(
                    radius: 18,
                    child: Icon(
                      item.kind ==
                              Revision3VoiceProductionQueueItemKind
                                  .missingLanguage
                          ? Icons.translate
                          : Icons.record_voice_over_outlined,
                      size: 20,
                    ),
                  ),
                  const SizedBox(width: 10),
                  Expanded(
                    child: Column(
                      crossAxisAlignment: CrossAxisAlignment.start,
                      children: [
                        Text(
                          item.displayLabel,
                          style: theme.textTheme.titleMedium,
                        ),
                        const SizedBox(height: 5),
                        Wrap(
                          spacing: 6,
                          runSpacing: 5,
                          children: [
                            _DetailChip(label: copy.kindLabel(item.kind)),
                            _DetailChip(label: copy.language(item.locale)),
                            ..._voiceDetailLabels(
                              item,
                              copy,
                            ).map((label) => _DetailChip(label: label)),
                          ],
                        ),
                      ],
                    ),
                  ),
                ],
              ),
              const SizedBox(height: 12),
              Material(
                color: item.isActionable
                    ? theme.colorScheme.primaryContainer
                    : theme.colorScheme.secondaryContainer,
                borderRadius: BorderRadius.circular(10),
                child: Padding(
                  padding: const EdgeInsets.all(11),
                  child: Row(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      Icon(
                        item.isActionable
                            ? Icons.arrow_forward
                            : Icons.check_circle_outline,
                        size: 20,
                      ),
                      const SizedBox(width: 9),
                      Expanded(
                        child: Column(
                          crossAxisAlignment: CrossAxisAlignment.start,
                          children: [
                            Text(
                              copy.nextStepLabel,
                              style: theme.textTheme.labelMedium,
                            ),
                            Text(
                              copy.stepTitle(item.nextStep),
                              style: theme.textTheme.titleSmall,
                            ),
                            const SizedBox(height: 2),
                            Text(copy.stepDescription(item.nextStep)),
                          ],
                        ),
                      ),
                    ],
                  ),
                ),
              ),
              if (item is Revision3VoiceSlotQueueItem &&
                  (item as Revision3VoiceSlotQueueItem)
                      .hasUnreviewedAlternatives) ...[
                const SizedBox(height: 7),
                Text(
                  copy.unreviewedAlternativesLabel,
                  style: theme.textTheme.bodySmall?.copyWith(
                    color: theme.colorScheme.onSurfaceVariant,
                  ),
                ),
              ],
              const SizedBox(height: 10),
              if (compact)
                Tooltip(message: disabledReason ?? action.label, child: button)
              else
                Align(
                  alignment: Alignment.centerRight,
                  child: ConstrainedBox(
                    constraints: const BoxConstraints(maxWidth: 420),
                    child: Tooltip(
                      message: disabledReason ?? action.label,
                      child: button,
                    ),
                  ),
                ),
              if (disabledReason != null) ...[
                const SizedBox(height: 6),
                Text(
                  disabledReason!,
                  key: ValueKey(
                    'revision3-voice-production-queue-disabled-${item.key}',
                  ),
                  textAlign: compact ? TextAlign.start : TextAlign.end,
                  style: theme.textTheme.bodySmall?.copyWith(
                    color: theme.colorScheme.onSurfaceVariant,
                  ),
                ),
              ],
            ],
          ),
        ),
      ),
    );
  }
}

final class _QueueEmpty extends StatelessWidget {
  const _QueueEmpty({
    required this.icon,
    required this.title,
    required this.description,
  });

  final IconData icon;
  final String title;
  final String description;

  @override
  Widget build(BuildContext context) => Center(
    child: SingleChildScrollView(
      padding: const EdgeInsets.all(24),
      child: ConstrainedBox(
        constraints: const BoxConstraints(maxWidth: 520),
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            Icon(icon, size: 42),
            const SizedBox(height: 10),
            Text(
              title,
              textAlign: TextAlign.center,
              style: Theme.of(context).textTheme.titleMedium,
            ),
            const SizedBox(height: 5),
            Text(description, textAlign: TextAlign.center),
          ],
        ),
      ),
    ),
  );
}

final class _SummaryChip extends StatelessWidget {
  const _SummaryChip({required this.label});

  final String label;

  @override
  Widget build(BuildContext context) => DecoratedBox(
    decoration: BoxDecoration(
      color: Theme.of(context).colorScheme.surfaceContainerHighest,
      borderRadius: BorderRadius.circular(999),
    ),
    child: Padding(
      padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 5),
      child: Text(label, style: Theme.of(context).textTheme.labelMedium),
    ),
  );
}

final class _DetailChip extends StatelessWidget {
  const _DetailChip({required this.label});

  final String label;

  @override
  Widget build(BuildContext context) => DecoratedBox(
    decoration: BoxDecoration(
      border: Border.all(color: Theme.of(context).colorScheme.outlineVariant),
      borderRadius: BorderRadius.circular(999),
    ),
    child: Padding(
      padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 3),
      child: Text(label, style: Theme.of(context).textTheme.labelSmall),
    ),
  );
}

final class _QueueAction {
  const _QueueAction({
    required this.label,
    required this.icon,
    required this.invoke,
  });

  final String label;
  final IconData icon;
  final FutureOr<void> Function()? invoke;
}

int _actionCount(Revision3VoiceProductionQueue queue) =>
    Revision3VoiceProductionNextStep.values
        .where(
          (step) =>
              step !=
              Revision3VoiceProductionNextStep.productionDecisionsComplete,
        )
        .fold(0, (count, step) => count + queue.countFor(step));

int _filterCount(
  Revision3VoiceProductionQueue queue,
  Revision3VoiceProductionQueueFilter filter,
) => switch (filter) {
  Revision3VoiceProductionQueueFilter.all => queue.totalItemCount,
  Revision3VoiceProductionQueueFilter.needsAction => _actionCount(queue),
  Revision3VoiceProductionQueueFilter.missingLanguages => queue.countFor(
    Revision3VoiceProductionNextStep.addLanguage,
  ),
  Revision3VoiceProductionQueueFilter.voice =>
    queue.totalItemCount -
        queue.countFor(Revision3VoiceProductionNextStep.addLanguage),
  Revision3VoiceProductionQueueFilter.complete => queue.countFor(
    Revision3VoiceProductionNextStep.productionDecisionsComplete,
  ),
};

Iterable<String> _voiceDetailLabels(
  Revision3VoiceProductionQueueItem item,
  Revision3VoiceProductionQueueCopy copy,
) sync* {
  if (item is! Revision3VoiceSlotQueueItem) return;
  yield copy.recordingCount(item.candidateCount);
  yield copy.approvedCount(item.approvedCount);
  yield switch (item.selectionState) {
    Revision3VoiceProductionSelectionState.none => copy.noSelectionLabel,
    Revision3VoiceProductionSelectionState.selectedNotApproved =>
      copy.selectionNeedsAttentionLabel,
    Revision3VoiceProductionSelectionState.selectedApproved =>
      copy.approvedSelectionLabel,
  };
  yield switch (item.targetResolution) {
    Revision3ContentVoiceTargetResolution.unresolved =>
      copy.targetUnresolvedLabel,
    Revision3ContentVoiceTargetResolution.ambiguous =>
      copy.targetAmbiguousLabel,
    Revision3ContentVoiceTargetResolution.resolved => copy.targetResolvedLabel,
  };
}
