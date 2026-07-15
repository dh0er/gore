import 'dart:async';

import 'package:flutter/material.dart';

import '../core/mod_ffi.dart';
import 'revision3_content_index.dart';
import 'revision3_project_problems.dart';

typedef Revision3ProblemsViewContentLoader =
    Future<Revision3ContentIndex> Function();
typedef Revision3ProblemsViewDataAssetStageLoader =
    Future<List<AuthoringRevision3DataAssetStage>> Function();
typedef Revision3ProblemsViewOpenEntity =
    FutureOr<void> Function(String entityId);
typedef Revision3ProblemsViewOpenAsset =
    FutureOr<void> Function(String assetSha256);
typedef Revision3ProblemsViewOpenDataAssetStage =
    FutureOr<void> Function(String targetPath);
typedef Revision3ProblemsViewAction = FutureOr<void> Function();
typedef Revision3ProblemsViewProblemCopy =
    String Function(Revision3ProjectProblem problem);
typedef Revision3ProblemsViewCategoryCopy =
    String Function(Revision3ProjectProblemCategory category);
typedef Revision3ProblemsViewSeverityCopy =
    String Function(Revision3ProjectProblemSeverity severity);
typedef Revision3ProblemsViewScopeCopy =
    String Function(Revision3ProjectProblemScope scope);
typedef Revision3ProblemsViewReadinessCopy =
    String Function(Revision3ProjectProblemReadiness readiness);
typedef Revision3ProblemsViewEvidenceCopy =
    String Function(Revision3ProjectProblemEvidence evidence);

/// Read-only navigation supplied to the project Problems surface.
///
/// Every target-bearing callback receives the exact stable identifier from a
/// validated report. The view owns no project mutation, build, deployment, or
/// runtime authority.
@immutable
final class Revision3ProjectProblemsActions {
  const Revision3ProjectProblemsActions({
    this.openEntity,
    this.openAsset,
    this.openDataAssetStage,
    this.openSettings,
    this.verifyCurrentProject,
  });

  final Revision3ProblemsViewOpenEntity? openEntity;
  final Revision3ProblemsViewOpenAsset? openAsset;
  final Revision3ProblemsViewOpenDataAssetStage? openDataAssetStage;
  final Revision3ProblemsViewAction? openSettings;
  final Revision3ProblemsViewAction? verifyCurrentProject;
}

/// All author-facing copy rendered by [Revision3ProjectProblemsView].
///
/// The root workspace supplies this object from its localization layer. No
/// domain diagnostic title, technical enum name, exception, path, or hash is
/// treated as localized presentation copy by the view.
@immutable
final class Revision3ProjectProblemsCopy {
  const Revision3ProjectProblemsCopy({
    required this.title,
    required this.description,
    required this.scopeNotice,
    required this.refreshTooltip,
    required this.loadingSemanticsLabel,
    required this.loadErrorSemanticsLabel,
    required this.loadErrorTitle,
    required this.loadErrorDescription,
    required this.retryLabel,
    required this.partialTitle,
    required this.dataAssetsUnavailableDescription,
    required this.overviewHeading,
    required this.scopeTitle,
    required this.scopeDescription,
    required this.readinessName,
    required this.evidenceName,
    required this.problemTitle,
    required this.problemDescription,
    required this.categoryName,
    required this.severityName,
    required this.searchLabel,
    required this.clearSearchTooltip,
    required this.filterAllLabel,
    required this.listHeading,
    required this.emptyTitle,
    required this.emptyDescription,
    required this.emptyBoundaryDescription,
    required this.filteredEmptyTitle,
    required this.filteredEmptyDescription,
    required this.selectProblemTitle,
    required this.selectProblemDescription,
    required this.detailHeading,
    required this.closeDetailTooltip,
    required this.categoryLabel,
    required this.severityLabel,
    required this.sourceLabel,
    required this.openEntityLabel,
    required this.openAssetLabel,
    required this.openDataAssetStageLabel,
    required this.openSettingsLabel,
    required this.verifyCurrentProjectLabel,
    required this.actionFailedMessage,
    required this.actionInProgressSemanticsLabel,
  });

  final String title;
  final String description;
  final String scopeNotice;
  final String refreshTooltip;
  final String loadingSemanticsLabel;
  final String loadErrorSemanticsLabel;
  final String loadErrorTitle;
  final String loadErrorDescription;
  final String retryLabel;
  final String partialTitle;
  final String dataAssetsUnavailableDescription;
  final String overviewHeading;
  final Revision3ProblemsViewScopeCopy scopeTitle;
  final Revision3ProblemsViewScopeCopy scopeDescription;
  final Revision3ProblemsViewReadinessCopy readinessName;
  final Revision3ProblemsViewEvidenceCopy evidenceName;
  final Revision3ProblemsViewProblemCopy problemTitle;
  final Revision3ProblemsViewProblemCopy problemDescription;
  final Revision3ProblemsViewCategoryCopy categoryName;
  final Revision3ProblemsViewSeverityCopy severityName;
  final String searchLabel;
  final String clearSearchTooltip;
  final String filterAllLabel;
  final String listHeading;
  final String emptyTitle;
  final String emptyDescription;
  final String emptyBoundaryDescription;
  final String filteredEmptyTitle;
  final String filteredEmptyDescription;
  final String selectProblemTitle;
  final String selectProblemDescription;
  final String detailHeading;
  final String closeDetailTooltip;
  final String categoryLabel;
  final String severityLabel;
  final String sourceLabel;
  final String openEntityLabel;
  final String openAssetLabel;
  final String openDataAssetStageLabel;
  final String openSettingsLabel;
  final String verifyCurrentProjectLabel;
  final String actionFailedMessage;
  final String actionInProgressSemanticsLabel;
}

/// Read-only Problems surface for one exact managed revision-3 checkpoint.
///
/// The two sources are loaded independently so one unavailable projection does
/// not erase diagnostics from the other. Project identity changes, manual
/// refreshes, and disposal invalidate every older asynchronous completion.
final class Revision3ProjectProblemsView extends StatefulWidget {
  const Revision3ProjectProblemsView({
    required this.projectId,
    required this.projectRevision,
    required this.loadContent,
    required this.loadDataAssetStages,
    required this.gameConfigured,
    required this.copy,
    this.actions = const Revision3ProjectProblemsActions(),
    super.key,
  }) : assert(projectId != ''),
       assert(projectRevision >= 0);

  final String projectId;
  final int projectRevision;
  final Revision3ProblemsViewContentLoader loadContent;
  final Revision3ProblemsViewDataAssetStageLoader loadDataAssetStages;
  final bool gameConfigured;
  final Revision3ProjectProblemsCopy copy;
  final Revision3ProjectProblemsActions actions;

  @override
  State<Revision3ProjectProblemsView> createState() =>
      _Revision3ProjectProblemsViewState();
}

class _Revision3ProjectProblemsViewState
    extends State<Revision3ProjectProblemsView> {
  final TextEditingController _search = TextEditingController();
  _Revision3ProblemsSources? _sources;
  Revision3ProjectProblemCategory? _category;
  String? _selectedProblemId;
  bool _loading = true;
  bool _failed = false;
  int _loadEpoch = 0;

  @override
  void initState() {
    super.initState();
    _search.addListener(_searchChanged);
    _startLoad(notify: false);
  }

  @override
  void didUpdateWidget(covariant Revision3ProjectProblemsView oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.projectId != widget.projectId ||
        oldWidget.projectRevision != widget.projectRevision) {
      _search.clear();
      _category = null;
      _selectedProblemId = null;
      _startLoad(notify: false);
    }
  }

  @override
  void dispose() {
    _loadEpoch++;
    _search
      ..removeListener(_searchChanged)
      ..dispose();
    super.dispose();
  }

  void _searchChanged() => setState(() {});

  void _startLoad({required bool notify}) {
    final epoch = ++_loadEpoch;
    final expectedProjectId = widget.projectId;
    final expectedProjectRevision = widget.projectRevision;
    final contentLoader = widget.loadContent;
    final stageLoader = widget.loadDataAssetStages;

    void markLoading() {
      _sources = null;
      _loading = true;
      _failed = false;
    }

    if (notify) {
      setState(markLoading);
    } else {
      markLoading();
    }

    unawaited(
      _finishLoad(
        epoch: epoch,
        expectedProjectId: expectedProjectId,
        expectedProjectRevision: expectedProjectRevision,
        contentLoader: contentLoader,
        stageLoader: stageLoader,
      ),
    );
  }

  Future<void> _finishLoad({
    required int epoch,
    required String expectedProjectId,
    required int expectedProjectRevision,
    required Revision3ProblemsViewContentLoader contentLoader,
    required Revision3ProblemsViewDataAssetStageLoader stageLoader,
  }) async {
    final results = await Future.wait<Object>([
      _loadContent(
        contentLoader,
        expectedProjectId: expectedProjectId,
        expectedProjectRevision: expectedProjectRevision,
      ),
      _loadStages(
        stageLoader,
        expectedProjectId: expectedProjectId,
        expectedProjectRevision: expectedProjectRevision,
      ),
    ]);
    if (!mounted || epoch != _loadEpoch) return;

    final content = results[0] as _Revision3ContentLoad;
    final stages = results[1] as _Revision3StagesLoad;
    if (content.value == null) {
      setState(() {
        _sources = null;
        _loading = false;
        _failed = true;
      });
      return;
    }

    var exactStages = stages.value;
    if (exactStages != null) {
      try {
        Revision3ProjectProblemBuilder.build(
          content.value!,
          dataAssetStages: exactStages,
          gameConfigured: true,
        );
      } catch (_) {
        exactStages = null;
      }
    }

    setState(() {
      _sources = _Revision3ProblemsSources(
        content: content.value!,
        stages: exactStages,
      );
      _loading = false;
      _failed = false;
    });
  }

  @override
  Widget build(BuildContext context) {
    if (_loading) {
      return Center(
        key: const Key('revision3-project-problems-loading'),
        child: Semantics(
          liveRegion: true,
          label: widget.copy.loadingSemanticsLabel,
          child: const CircularProgressIndicator(),
        ),
      );
    }
    if (_failed || _sources == null) {
      return _Revision3ProblemsLoadError(
        copy: widget.copy,
        retry: () => _startLoad(notify: true),
      );
    }

    final sources = _sources!;
    final report = Revision3ProjectProblemBuilder.build(
      sources.content,
      dataAssetStages: sources.stages,
      gameConfigured: widget.gameConfigured,
    );
    final visible = _visibleProblems(report);
    final selected = _selectedProblem(visible);
    return _Revision3ProblemsLoaded(
      report: report,
      visibleProblems: visible,
      selectedProblem: selected,
      search: _search,
      selectedCategory: _category,
      copy: widget.copy,
      actions: widget.actions,
      refresh: () => _startLoad(notify: true),
      selectCategory: (category) => setState(() {
        _category = category;
        _selectedProblemId = null;
      }),
      selectProblem: (problem) => setState(() {
        _selectedProblemId = problem.id;
      }),
    );
  }

  List<Revision3ProjectProblem> _visibleProblems(
    Revision3ProjectProblemReport report,
  ) {
    final query = _search.text.trim().toLowerCase();
    return report.problems
        .where((problem) {
          if (_category != null && problem.category != _category) return false;
          if (query.isEmpty) return true;
          return <String>[
            widget.copy.problemTitle(problem),
            widget.copy.problemDescription(problem),
            widget.copy.categoryName(problem.category),
            widget.copy.severityName(problem.severity),
            ...problem.searchTerms,
          ].any((term) => term.toLowerCase().contains(query));
        })
        .toList(growable: false);
  }

  Revision3ProjectProblem? _selectedProblem(
    List<Revision3ProjectProblem> visible,
  ) {
    final selectedId = _selectedProblemId;
    if (selectedId != null) {
      for (final problem in visible) {
        if (problem.id == selectedId) return problem;
      }
    }
    return visible.isEmpty ? null : visible.first;
  }
}

final class _Revision3ProblemsSources {
  const _Revision3ProblemsSources({
    required this.content,
    required this.stages,
  });

  final Revision3ContentIndex content;
  final List<AuthoringRevision3DataAssetStage>? stages;
}

class _Revision3ProblemsLoaded extends StatelessWidget {
  const _Revision3ProblemsLoaded({
    required this.report,
    required this.visibleProblems,
    required this.selectedProblem,
    required this.search,
    required this.selectedCategory,
    required this.copy,
    required this.actions,
    required this.refresh,
    required this.selectCategory,
    required this.selectProblem,
  });

  final Revision3ProjectProblemReport report;
  final List<Revision3ProjectProblem> visibleProblems;
  final Revision3ProjectProblem? selectedProblem;
  final TextEditingController search;
  final Revision3ProjectProblemCategory? selectedCategory;
  final Revision3ProjectProblemsCopy copy;
  final Revision3ProjectProblemsActions actions;
  final VoidCallback refresh;
  final ValueChanged<Revision3ProjectProblemCategory?> selectCategory;
  final ValueChanged<Revision3ProjectProblem> selectProblem;

  @override
  Widget build(BuildContext context) => LayoutBuilder(
    builder: (context, constraints) {
      final compact = constraints.maxHeight < 560;
      final split = constraints.maxWidth >= 960 && !compact;
      return Semantics(
        key: const Key('revision3-project-problems-view'),
        container: true,
        explicitChildNodes: true,
        child: Padding(
          padding: EdgeInsets.all(compact ? 8 : 16),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: [
              _ProblemsHeader(
                copy: copy,
                actions: actions,
                refresh: refresh,
                compact: compact,
              ),
              if (!report.dataAssetRegistryAvailable) ...[
                SizedBox(height: compact ? 6 : 10),
                _ProblemsPartialNotice(copy: copy),
              ],
              SizedBox(height: compact ? 6 : 12),
              if (!compact) ...[
                Text(
                  copy.overviewHeading,
                  style: Theme.of(context).textTheme.titleMedium,
                ),
                const SizedBox(height: 8),
              ],
              _ProblemsAssessmentStrip(
                assessments: report.assessments,
                copy: copy,
                actions: actions,
                compact: compact,
              ),
              SizedBox(height: compact ? 6 : 12),
              _ProblemsControls(
                report: report,
                search: search,
                selectedCategory: selectedCategory,
                copy: copy,
                selectCategory: selectCategory,
                compact: compact,
              ),
              SizedBox(height: compact ? 6 : 10),
              Expanded(
                child: _ProblemsMasterDetail(
                  allProblemsEmpty: report.problems.isEmpty,
                  problems: visibleProblems,
                  selectedProblem: selectedProblem,
                  copy: copy,
                  actions: actions,
                  split: split,
                  selectProblem: selectProblem,
                ),
              ),
            ],
          ),
        ),
      );
    },
  );
}

class _ProblemsHeader extends StatelessWidget {
  const _ProblemsHeader({
    required this.copy,
    required this.actions,
    required this.refresh,
    required this.compact,
  });

  final Revision3ProjectProblemsCopy copy;
  final Revision3ProjectProblemsActions actions;
  final VoidCallback refresh;
  final bool compact;

  @override
  Widget build(BuildContext context) => Material(
    color: Theme.of(context).colorScheme.surfaceContainerLow,
    borderRadius: BorderRadius.circular(16),
    child: Padding(
      padding: EdgeInsets.symmetric(
        horizontal: compact ? 12 : 18,
        vertical: compact ? 8 : 14,
      ),
      child: Row(
        children: [
          Icon(
            Icons.rule_folder_outlined,
            color: Theme.of(context).colorScheme.primary,
          ),
          const SizedBox(width: 12),
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              mainAxisSize: MainAxisSize.min,
              children: [
                Semantics(
                  header: true,
                  child: Text(
                    copy.title,
                    style: Theme.of(context).textTheme.titleLarge,
                  ),
                ),
                if (!compact) ...[
                  const SizedBox(height: 2),
                  Text(
                    copy.description,
                    maxLines: 2,
                    overflow: TextOverflow.ellipsis,
                    style: Theme.of(context).textTheme.bodySmall,
                  ),
                ],
                const SizedBox(height: 2),
                Text(
                  copy.scopeNotice,
                  maxLines: compact ? 1 : 2,
                  overflow: TextOverflow.ellipsis,
                  style: Theme.of(context).textTheme.bodySmall?.copyWith(
                    color: Theme.of(context).colorScheme.onSurfaceVariant,
                  ),
                ),
              ],
            ),
          ),
          if (actions.verifyCurrentProject != null)
            _ProblemsActionIcon(
              action: _ProblemsActionSpec(
                key: 'revision3-project-problems-verify-current-project',
                label: copy.verifyCurrentProjectLabel,
                icon: Icons.verified_outlined,
                invoke: actions.verifyCurrentProject!,
              ),
              actionKey: 'revision3-project-problems-verify-current-project',
              copy: copy,
            ),
          IconButton(
            key: const Key('revision3-project-problems-refresh'),
            tooltip: copy.refreshTooltip,
            onPressed: refresh,
            icon: const Icon(Icons.refresh),
          ),
        ],
      ),
    ),
  );
}

class _ProblemsPartialNotice extends StatelessWidget {
  const _ProblemsPartialNotice({required this.copy});

  final Revision3ProjectProblemsCopy copy;

  @override
  Widget build(BuildContext context) => Semantics(
    key: const Key('revision3-project-problems-partial'),
    container: true,
    liveRegion: true,
    child: Material(
      color: Theme.of(context).colorScheme.tertiaryContainer,
      borderRadius: BorderRadius.circular(12),
      child: Padding(
        padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 8),
        child: Row(
          children: [
            const Icon(Icons.warning_amber_rounded, size: 20),
            const SizedBox(width: 8),
            Expanded(
              child: Text.rich(
                TextSpan(
                  children: [
                    TextSpan(
                      text: '${copy.partialTitle} ',
                      style: const TextStyle(fontWeight: FontWeight.w600),
                    ),
                    TextSpan(text: copy.dataAssetsUnavailableDescription),
                  ],
                ),
              ),
            ),
          ],
        ),
      ),
    ),
  );
}

class _ProblemsAssessmentStrip extends StatelessWidget {
  const _ProblemsAssessmentStrip({
    required this.assessments,
    required this.copy,
    required this.actions,
    required this.compact,
  });

  final List<Revision3ProjectProblemAssessment> assessments;
  final Revision3ProjectProblemsCopy copy;
  final Revision3ProjectProblemsActions actions;
  final bool compact;

  @override
  Widget build(BuildContext context) => SizedBox(
    height: compact ? 72 : 112,
    child: ListView.separated(
      key: const Key('revision3-project-problems-assessments'),
      scrollDirection: Axis.horizontal,
      itemCount: assessments.length,
      separatorBuilder: (_, _) => const SizedBox(width: 8),
      itemBuilder: (context, index) => SizedBox(
        width: compact ? 184 : 230,
        child: _ProblemsAssessmentCard(
          assessment: assessments[index],
          copy: copy,
          actions: actions,
          compact: compact,
        ),
      ),
    ),
  );
}

class _ProblemsAssessmentCard extends StatelessWidget {
  const _ProblemsAssessmentCard({
    required this.assessment,
    required this.copy,
    required this.actions,
    required this.compact,
  });

  final Revision3ProjectProblemAssessment assessment;
  final Revision3ProjectProblemsCopy copy;
  final Revision3ProjectProblemsActions actions;
  final bool compact;

  @override
  Widget build(BuildContext context) {
    final colors = _readinessColors(
      Theme.of(context).colorScheme,
      assessment.readiness,
    );
    final action = assessment.primaryTarget == null
        ? null
        : _actionForTarget(assessment.primaryTarget!, actions, copy);
    return Semantics(
      key: Key(
        'revision3-project-problems-assessment-${assessment.scope.name}',
      ),
      container: true,
      explicitChildNodes: true,
      label: copy.scopeTitle(assessment.scope),
      value: copy.readinessName(assessment.readiness),
      child: Material(
        color: colors.$1,
        borderRadius: BorderRadius.circular(12),
        child: Padding(
          padding: EdgeInsets.all(compact ? 8 : 11),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Row(
                children: [
                  Expanded(
                    child: Text(
                      copy.scopeTitle(assessment.scope),
                      maxLines: 1,
                      overflow: TextOverflow.ellipsis,
                      style: Theme.of(context).textTheme.labelLarge,
                    ),
                  ),
                  if (action != null)
                    _ProblemsActionIcon(
                      action: action,
                      actionKey:
                          'revision3-project-problems-assessment-action-${assessment.scope.name}',
                      copy: copy,
                    ),
                ],
              ),
              const SizedBox(height: 3),
              Text(
                copy.readinessName(assessment.readiness),
                maxLines: 1,
                overflow: TextOverflow.ellipsis,
                style: Theme.of(context).textTheme.labelMedium?.copyWith(
                  color: colors.$2,
                  fontWeight: FontWeight.w700,
                ),
              ),
              if (!compact) ...[
                const SizedBox(height: 4),
                Expanded(
                  child: Text(
                    copy.scopeDescription(assessment.scope),
                    maxLines: 3,
                    overflow: TextOverflow.ellipsis,
                    style: Theme.of(context).textTheme.bodySmall,
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

class _ProblemsControls extends StatelessWidget {
  const _ProblemsControls({
    required this.report,
    required this.search,
    required this.selectedCategory,
    required this.copy,
    required this.selectCategory,
    required this.compact,
  });

  final Revision3ProjectProblemReport report;
  final TextEditingController search;
  final Revision3ProjectProblemCategory? selectedCategory;
  final Revision3ProjectProblemsCopy copy;
  final ValueChanged<Revision3ProjectProblemCategory?> selectCategory;
  final bool compact;

  @override
  Widget build(BuildContext context) => SizedBox(
    height: compact ? 78 : 86,
    child: Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        SizedBox(
          height: 42,
          child: TextField(
            key: const Key('revision3-project-problems-search'),
            controller: search,
            decoration: InputDecoration(
              labelText: copy.searchLabel,
              prefixIcon: const Icon(Icons.search, size: 20),
              suffixIcon: search.text.isEmpty
                  ? null
                  : IconButton(
                      key: const Key('revision3-project-problems-clear-search'),
                      tooltip: copy.clearSearchTooltip,
                      onPressed: search.clear,
                      icon: const Icon(Icons.clear, size: 20),
                    ),
              border: const OutlineInputBorder(),
              isDense: true,
            ),
          ),
        ),
        const SizedBox(height: 5),
        Expanded(
          child: ListView(
            key: const Key('revision3-project-problems-filters'),
            scrollDirection: Axis.horizontal,
            children: [
              FilterChip(
                key: const Key('revision3-project-problems-filter-all'),
                selected: selectedCategory == null,
                onSelected: (_) => selectCategory(null),
                label: Text(
                  '${copy.filterAllLabel} (${report.problems.length})',
                ),
              ),
              for (final category
                  in Revision3ProjectProblemCategory.values) ...[
                const SizedBox(width: 6),
                FilterChip(
                  key: Key(
                    'revision3-project-problems-filter-${category.name}',
                  ),
                  selected: selectedCategory == category,
                  onSelected: (_) => selectCategory(category),
                  label: Text(
                    '${copy.categoryName(category)} '
                    '(${report.countForCategory(category)})',
                  ),
                ),
              ],
            ],
          ),
        ),
      ],
    ),
  );
}

class _ProblemsMasterDetail extends StatelessWidget {
  const _ProblemsMasterDetail({
    required this.allProblemsEmpty,
    required this.problems,
    required this.selectedProblem,
    required this.copy,
    required this.actions,
    required this.split,
    required this.selectProblem,
  });

  final bool allProblemsEmpty;
  final List<Revision3ProjectProblem> problems;
  final Revision3ProjectProblem? selectedProblem;
  final Revision3ProjectProblemsCopy copy;
  final Revision3ProjectProblemsActions actions;
  final bool split;
  final ValueChanged<Revision3ProjectProblem> selectProblem;

  @override
  Widget build(BuildContext context) {
    if (allProblemsEmpty) return _ProblemsEmpty(copy: copy);
    if (problems.isEmpty) return _ProblemsFilteredEmpty(copy: copy);

    final list = _ProblemsList(
      problems: problems,
      selectedProblemId: split ? selectedProblem?.id : null,
      copy: copy,
      select: (problem) {
        if (split) {
          selectProblem(problem);
        } else {
          _showProblemSheet(context, problem, copy, actions);
        }
      },
    );
    if (!split) return list;
    return Row(
      key: const Key('revision3-project-problems-split'),
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        Expanded(flex: 5, child: list),
        const VerticalDivider(width: 18),
        Expanded(
          flex: 6,
          child: selectedProblem == null
              ? _ProblemsSelectionPrompt(copy: copy)
              : _ProblemDetailPanel(
                  problem: selectedProblem!,
                  copy: copy,
                  actions: actions,
                ),
        ),
      ],
    );
  }
}

class _ProblemsList extends StatelessWidget {
  const _ProblemsList({
    required this.problems,
    required this.selectedProblemId,
    required this.copy,
    required this.select,
  });

  final List<Revision3ProjectProblem> problems;
  final String? selectedProblemId;
  final Revision3ProjectProblemsCopy copy;
  final ValueChanged<Revision3ProjectProblem> select;

  @override
  Widget build(BuildContext context) => Material(
    color: Theme.of(context).colorScheme.surfaceContainerLowest,
    borderRadius: BorderRadius.circular(12),
    clipBehavior: Clip.antiAlias,
    child: Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        Padding(
          padding: const EdgeInsets.fromLTRB(12, 8, 12, 4),
          child: Text(
            copy.listHeading,
            style: Theme.of(context).textTheme.labelLarge,
          ),
        ),
        Expanded(
          child: ListView.separated(
            key: const Key('revision3-project-problems-list'),
            itemCount: problems.length,
            separatorBuilder: (_, _) => const Divider(height: 1),
            itemBuilder: (context, index) {
              final problem = problems[index];
              return Semantics(
                button: true,
                selected: problem.id == selectedProblemId,
                label: copy.problemTitle(problem),
                hint: copy.problemDescription(problem),
                child: ListTile(
                  key: Key('revision3-project-problem-${problem.id}'),
                  selected: problem.id == selectedProblemId,
                  leading: Icon(
                    _severityIcon(problem.severity),
                    color: _severityColor(
                      Theme.of(context).colorScheme,
                      problem.severity,
                    ),
                  ),
                  title: Text(
                    copy.problemTitle(problem),
                    maxLines: 1,
                    overflow: TextOverflow.ellipsis,
                  ),
                  subtitle: Text(
                    copy.problemDescription(problem),
                    maxLines: 2,
                    overflow: TextOverflow.ellipsis,
                  ),
                  trailing: const Icon(Icons.chevron_right),
                  onTap: () => select(problem),
                ),
              );
            },
          ),
        ),
      ],
    ),
  );
}

class _ProblemDetailPanel extends StatefulWidget {
  const _ProblemDetailPanel({
    required this.problem,
    required this.copy,
    required this.actions,
    this.close,
  });

  final Revision3ProjectProblem problem;
  final Revision3ProjectProblemsCopy copy;
  final Revision3ProjectProblemsActions actions;
  final VoidCallback? close;

  @override
  State<_ProblemDetailPanel> createState() => _ProblemDetailPanelState();
}

class _ProblemDetailPanelState extends State<_ProblemDetailPanel> {
  @override
  Widget build(BuildContext context) {
    final problem = widget.problem;
    final copy = widget.copy;
    final targets = <Revision3ProjectProblemTarget>[
      problem.primaryTarget,
      ...problem.relatedTargets,
    ];
    final seen = <String>{};
    final availableActions = <_ProblemsActionSpec>[];
    for (final target in targets) {
      final key = '${target.kind.name}:${target.identity}';
      if (!seen.add(key)) continue;
      final action = _actionForTarget(target, widget.actions, copy);
      if (action != null) availableActions.add(action);
    }

    return Material(
      key: const Key('revision3-project-problems-detail'),
      color: Theme.of(context).colorScheme.surfaceContainerLow,
      borderRadius: BorderRadius.circular(12),
      clipBehavior: Clip.antiAlias,
      child: SingleChildScrollView(
        key: const Key('revision3-project-problems-detail-scroll'),
        padding: const EdgeInsets.all(16),
        child: Semantics(
          container: true,
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: [
              Row(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Expanded(
                    child: Text(
                      copy.detailHeading,
                      style: Theme.of(context).textTheme.labelLarge,
                    ),
                  ),
                  if (widget.close != null)
                    IconButton(
                      key: const Key('revision3-project-problems-close-detail'),
                      tooltip: copy.closeDetailTooltip,
                      onPressed: widget.close,
                      icon: const Icon(Icons.close),
                    ),
                ],
              ),
              const SizedBox(height: 8),
              Text(
                copy.problemTitle(problem),
                style: Theme.of(context).textTheme.titleLarge,
              ),
              const SizedBox(height: 8),
              Text(copy.problemDescription(problem)),
              const SizedBox(height: 14),
              Wrap(
                spacing: 8,
                runSpacing: 8,
                children: [
                  _ProblemsFactChip(
                    label: copy.categoryLabel,
                    value: copy.categoryName(problem.category),
                  ),
                  _ProblemsFactChip(
                    label: copy.severityLabel,
                    value: copy.severityName(problem.severity),
                  ),
                  _ProblemsFactChip(
                    label: copy.sourceLabel,
                    value: copy.evidenceName(problem.evidence),
                  ),
                ],
              ),
              if (availableActions.isNotEmpty) ...[
                const SizedBox(height: 18),
                Wrap(
                  spacing: 8,
                  runSpacing: 8,
                  children: [
                    for (final action in availableActions)
                      _ProblemsActionButton(
                        action: action,
                        copy: copy,
                        onSuccess: widget.close,
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

class _ProblemsFactChip extends StatelessWidget {
  const _ProblemsFactChip({required this.label, required this.value});

  final String label;
  final String value;

  @override
  Widget build(BuildContext context) =>
      InputChip(isEnabled: false, label: Text('$label: $value'));
}

class _ProblemsActionSpec {
  const _ProblemsActionSpec({
    required this.key,
    required this.label,
    required this.icon,
    required this.invoke,
  });

  final String key;
  final String label;
  final IconData icon;
  final Revision3ProblemsViewAction invoke;
}

class _ProblemsActionButton extends StatefulWidget {
  const _ProblemsActionButton({
    required this.action,
    required this.copy,
    required this.onSuccess,
  });

  final _ProblemsActionSpec action;
  final Revision3ProjectProblemsCopy copy;
  final VoidCallback? onSuccess;

  @override
  State<_ProblemsActionButton> createState() => _ProblemsActionButtonState();
}

class _ProblemsActionButtonState extends State<_ProblemsActionButton> {
  bool _busy = false;

  Future<void> _invoke() async {
    if (_busy) return;
    setState(() => _busy = true);
    try {
      await Future<void>.sync(widget.action.invoke);
      if (mounted) widget.onSuccess?.call();
    } catch (_) {
      if (mounted) {
        ScaffoldMessenger.maybeOf(context)?.showSnackBar(
          SnackBar(content: Text(widget.copy.actionFailedMessage)),
        );
      }
    } finally {
      if (mounted) setState(() => _busy = false);
    }
  }

  @override
  Widget build(BuildContext context) => Semantics(
    liveRegion: _busy,
    label: _busy ? widget.copy.actionInProgressSemanticsLabel : null,
    child: FilledButton.tonalIcon(
      key: Key(widget.action.key),
      onPressed: _busy ? null : _invoke,
      icon: _busy
          ? const SizedBox.square(
              dimension: 16,
              child: CircularProgressIndicator(strokeWidth: 2),
            )
          : Icon(widget.action.icon),
      label: Text(widget.action.label),
    ),
  );
}

class _ProblemsActionIcon extends StatefulWidget {
  const _ProblemsActionIcon({
    required this.action,
    required this.actionKey,
    required this.copy,
  });

  final _ProblemsActionSpec action;
  final String actionKey;
  final Revision3ProjectProblemsCopy copy;

  @override
  State<_ProblemsActionIcon> createState() => _ProblemsActionIconState();
}

class _ProblemsActionIconState extends State<_ProblemsActionIcon> {
  bool _busy = false;

  Future<void> _invoke() async {
    if (_busy) return;
    setState(() => _busy = true);
    try {
      await Future<void>.sync(widget.action.invoke);
    } catch (_) {
      if (mounted) {
        ScaffoldMessenger.maybeOf(context)?.showSnackBar(
          SnackBar(content: Text(widget.copy.actionFailedMessage)),
        );
      }
    } finally {
      if (mounted) setState(() => _busy = false);
    }
  }

  @override
  Widget build(BuildContext context) => IconButton(
    key: Key(widget.actionKey),
    tooltip: widget.action.label,
    visualDensity: VisualDensity.compact,
    constraints: const BoxConstraints.tightFor(width: 32, height: 32),
    padding: EdgeInsets.zero,
    onPressed: _busy ? null : _invoke,
    icon: _busy
        ? const SizedBox.square(
            dimension: 16,
            child: CircularProgressIndicator(strokeWidth: 2),
          )
        : Icon(widget.action.icon, size: 18),
  );
}

class _ProblemsEmpty extends StatelessWidget {
  const _ProblemsEmpty({required this.copy});

  final Revision3ProjectProblemsCopy copy;

  @override
  Widget build(BuildContext context) => _ProblemsCenteredMessage(
    key: const Key('revision3-project-problems-empty'),
    icon: Icons.check_circle_outline,
    title: copy.emptyTitle,
    descriptions: [copy.emptyDescription, copy.emptyBoundaryDescription],
  );
}

class _ProblemsFilteredEmpty extends StatelessWidget {
  const _ProblemsFilteredEmpty({required this.copy});

  final Revision3ProjectProblemsCopy copy;

  @override
  Widget build(BuildContext context) => _ProblemsCenteredMessage(
    key: const Key('revision3-project-problems-filtered-empty'),
    icon: Icons.search_off,
    title: copy.filteredEmptyTitle,
    descriptions: [copy.filteredEmptyDescription],
  );
}

class _ProblemsSelectionPrompt extends StatelessWidget {
  const _ProblemsSelectionPrompt({required this.copy});

  final Revision3ProjectProblemsCopy copy;

  @override
  Widget build(BuildContext context) => _ProblemsCenteredMessage(
    key: const Key('revision3-project-problems-select-prompt'),
    icon: Icons.touch_app_outlined,
    title: copy.selectProblemTitle,
    descriptions: [copy.selectProblemDescription],
  );
}

class _ProblemsCenteredMessage extends StatelessWidget {
  const _ProblemsCenteredMessage({
    required super.key,
    required this.icon,
    required this.title,
    required this.descriptions,
  });

  final IconData icon;
  final String title;
  final List<String> descriptions;

  @override
  Widget build(BuildContext context) => SingleChildScrollView(
    child: Center(
      child: Padding(
        padding: const EdgeInsets.all(20),
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            Icon(icon, size: 36),
            const SizedBox(height: 10),
            Text(
              title,
              textAlign: TextAlign.center,
              style: Theme.of(context).textTheme.titleMedium,
            ),
            for (final description in descriptions) ...[
              const SizedBox(height: 6),
              Text(description, textAlign: TextAlign.center),
            ],
          ],
        ),
      ),
    ),
  );
}

Future<void> _showProblemSheet(
  BuildContext context,
  Revision3ProjectProblem problem,
  Revision3ProjectProblemsCopy copy,
  Revision3ProjectProblemsActions actions,
) => showModalBottomSheet<void>(
  context: context,
  isScrollControlled: true,
  useSafeArea: true,
  builder: (sheetContext) => FractionallySizedBox(
    heightFactor: 0.86,
    child: _ProblemDetailPanel(
      problem: problem,
      copy: copy,
      actions: actions,
      close: () => Navigator.of(sheetContext).pop(),
    ),
  ),
);

_ProblemsActionSpec? _actionForTarget(
  Revision3ProjectProblemTarget target,
  Revision3ProjectProblemsActions actions,
  Revision3ProjectProblemsCopy copy,
) {
  final key =
      'revision3-project-problems-action-${target.kind.name}-${target.identity}';
  return switch (target.kind) {
    Revision3ProjectProblemTargetKind.entity when actions.openEntity != null =>
      _ProblemsActionSpec(
        key: key,
        label: copy.openEntityLabel,
        icon: Icons.open_in_new,
        invoke: () => actions.openEntity!(target.identity),
      ),
    Revision3ProjectProblemTargetKind.asset when actions.openAsset != null =>
      _ProblemsActionSpec(
        key: key,
        label: copy.openAssetLabel,
        icon: Icons.inventory_2_outlined,
        invoke: () => actions.openAsset!(target.identity),
      ),
    Revision3ProjectProblemTargetKind.dataAssetStage
        when actions.openDataAssetStage != null =>
      _ProblemsActionSpec(
        key: key,
        label: copy.openDataAssetStageLabel,
        icon: Icons.data_object_outlined,
        invoke: () => actions.openDataAssetStage!(target.identity),
      ),
    Revision3ProjectProblemTargetKind.settings
        when actions.openSettings != null =>
      _ProblemsActionSpec(
        key: key,
        label: copy.openSettingsLabel,
        icon: Icons.settings_outlined,
        invoke: actions.openSettings!,
      ),
    _ => null,
  };
}

(Color, Color) _readinessColors(
  ColorScheme scheme,
  Revision3ProjectProblemReadiness readiness,
) => switch (readiness) {
  Revision3ProjectProblemReadiness.clear => (
    scheme.primaryContainer,
    scheme.onPrimaryContainer,
  ),
  Revision3ProjectProblemReadiness.issues ||
  Revision3ProjectProblemReadiness.unavailable ||
  Revision3ProjectProblemReadiness.notEvaluated => (
    scheme.tertiaryContainer,
    scheme.onTertiaryContainer,
  ),
  Revision3ProjectProblemReadiness.blocked ||
  Revision3ProjectProblemReadiness.unqualified => (
    scheme.errorContainer,
    scheme.onErrorContainer,
  ),
};

IconData _severityIcon(Revision3ProjectProblemSeverity severity) =>
    switch (severity) {
      Revision3ProjectProblemSeverity.information => Icons.info_outline,
      Revision3ProjectProblemSeverity.warning => Icons.warning_amber_rounded,
      Revision3ProjectProblemSeverity.blocking => Icons.error_outline,
    };

Color _severityColor(
  ColorScheme scheme,
  Revision3ProjectProblemSeverity severity,
) => switch (severity) {
  Revision3ProjectProblemSeverity.information => scheme.primary,
  Revision3ProjectProblemSeverity.warning => scheme.tertiary,
  Revision3ProjectProblemSeverity.blocking => scheme.error,
};

final class _Revision3ContentLoad {
  const _Revision3ContentLoad(this.value);

  final Revision3ContentIndex? value;
}

final class _Revision3StagesLoad {
  const _Revision3StagesLoad(this.value);

  final List<AuthoringRevision3DataAssetStage>? value;
}

Future<_Revision3ContentLoad> _loadContent(
  Revision3ProblemsViewContentLoader loader, {
  required String expectedProjectId,
  required int expectedProjectRevision,
}) async {
  try {
    final content = await loader();
    if (content.projectId != expectedProjectId ||
        content.projectRevision != expectedProjectRevision) {
      return const _Revision3ContentLoad(null);
    }
    return _Revision3ContentLoad(content);
  } catch (_) {
    return const _Revision3ContentLoad(null);
  }
}

Future<_Revision3StagesLoad> _loadStages(
  Revision3ProblemsViewDataAssetStageLoader loader, {
  required String expectedProjectId,
  required int expectedProjectRevision,
}) async {
  try {
    final stages = await loader();
    if (stages.any(
      (stage) =>
          stage.projectId != expectedProjectId ||
          stage.stagedProjectRevision != expectedProjectRevision,
    )) {
      return const _Revision3StagesLoad(null);
    }
    return _Revision3StagesLoad(
      List<AuthoringRevision3DataAssetStage>.unmodifiable(stages),
    );
  } catch (_) {
    return const _Revision3StagesLoad(null);
  }
}

class _Revision3ProblemsLoadError extends StatelessWidget {
  const _Revision3ProblemsLoadError({required this.copy, required this.retry});

  final Revision3ProjectProblemsCopy copy;
  final VoidCallback retry;

  @override
  Widget build(BuildContext context) => LayoutBuilder(
    builder: (context, constraints) => SingleChildScrollView(
      child: ConstrainedBox(
        constraints: BoxConstraints(minHeight: constraints.maxHeight),
        child: Center(
          child: Semantics(
            container: true,
            liveRegion: true,
            label: copy.loadErrorSemanticsLabel,
            child: ConstrainedBox(
              key: const Key('revision3-project-problems-error'),
              constraints: const BoxConstraints(maxWidth: 560),
              child: Padding(
                padding: const EdgeInsets.all(24),
                child: Column(
                  mainAxisSize: MainAxisSize.min,
                  children: [
                    Icon(
                      Icons.error_outline,
                      size: 42,
                      color: Theme.of(context).colorScheme.error,
                    ),
                    const SizedBox(height: 12),
                    Text(
                      copy.loadErrorTitle,
                      textAlign: TextAlign.center,
                      style: Theme.of(context).textTheme.titleMedium,
                    ),
                    const SizedBox(height: 8),
                    Text(
                      copy.loadErrorDescription,
                      textAlign: TextAlign.center,
                    ),
                    const SizedBox(height: 16),
                    FilledButton.icon(
                      key: const Key('revision3-project-problems-retry'),
                      onPressed: retry,
                      icon: const Icon(Icons.refresh),
                      label: Text(copy.retryLabel),
                    ),
                  ],
                ),
              ),
            ),
          ),
        ),
      ),
    ),
  );
}
