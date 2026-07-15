import 'dart:async';

import 'package:flutter/material.dart';

import 'revision3_global_content_search.dart';

typedef Revision3GlobalContentIdentityCallback = void Function(String value);

/// Exact action callbacks are split by source and identity type. The View
/// never translates a visible-row ordinal into authoring authority.
final class Revision3GlobalContentSearchCallbacks {
  const Revision3GlobalContentSearchCallbacks({
    required this.openThisModEntity,
    required this.openThisModAsset,
    required this.createBaseNpcDraft,
    required this.createBaseQuestDraft,
    required this.inspectInstalledDataAsset,
  });

  final Revision3GlobalContentIdentityCallback openThisModEntity;
  final Revision3GlobalContentIdentityCallback openThisModAsset;
  final Revision3GlobalContentIdentityCallback createBaseNpcDraft;
  final Revision3GlobalContentIdentityCallback createBaseQuestDraft;
  final Revision3GlobalContentIdentityCallback inspectInstalledDataAsset;

  void invoke(Revision3GlobalContentAction action) {
    switch (action.kind) {
      case Revision3GlobalContentActionKind.openThisModEntity:
        openThisModEntity(action.identity);
      case Revision3GlobalContentActionKind.openThisModAsset:
        openThisModAsset(action.identity);
      case Revision3GlobalContentActionKind.createBaseNpcDraft:
        createBaseNpcDraft(action.identity);
      case Revision3GlobalContentActionKind.createBaseQuestDraft:
        createBaseQuestDraft(action.identity);
      case Revision3GlobalContentActionKind.inspectInstalledDataAsset:
        inspectInstalledDataAsset(action.identity);
    }
  }
}

/// Localized copy injected by the host. No generated localization surface is
/// required by this standalone slice.
final class Revision3GlobalContentSearchCopy {
  Revision3GlobalContentSearchCopy({
    required this.title,
    required this.searchLabel,
    required this.searchAction,
    required this.clearAction,
    required this.emptyPrompt,
    required this.noResults,
    required this.loading,
    required this.loadFailed,
    required this.retry,
    required this.partial,
    required this.complete,
    required this.truncated,
    required this.openAction,
    required this.createDraftAction,
    required this.inspectAction,
    required Map<Revision3GlobalContentSource, String> sourceLabels,
    required Map<Revision3GlobalContentKind, String> kindLabels,
    required Map<Revision3GlobalContentReadiness, String> readinessLabels,
  }) : sourceLabels = _closedLabels(
         sourceLabels,
         Revision3GlobalContentSource.values,
         'source',
       ),
       kindLabels = _closedLabels(
         kindLabels,
         Revision3GlobalContentKind.values,
         'kind',
       ),
       readinessLabels = _closedLabels(
         readinessLabels,
         Revision3GlobalContentReadiness.values,
         'readiness',
       );

  final String title;
  final String searchLabel;
  final String searchAction;
  final String clearAction;
  final String emptyPrompt;
  final String noResults;
  final String loading;
  final String loadFailed;
  final String retry;
  final String partial;
  final String complete;
  final String truncated;
  final String openAction;
  final String createDraftAction;
  final String inspectAction;
  final Map<Revision3GlobalContentSource, String> sourceLabels;
  final Map<Revision3GlobalContentKind, String> kindLabels;
  final Map<Revision3GlobalContentReadiness, String> readinessLabels;

  String source(Revision3GlobalContentSource value) => sourceLabels[value]!;
  String kind(Revision3GlobalContentKind value) => kindLabels[value]!;
  String readiness(Revision3GlobalContentReadiness value) =>
      readinessLabels[value]!;
}

Map<T, String> _closedLabels<T>(
  Map<T, String> labels,
  Iterable<T> expected,
  String context,
) {
  final values = expected.toList(growable: false);
  if (labels.length != values.length ||
      values.any((value) => !labels.containsKey(value))) {
    throw ArgumentError(
      'Global content search $context labels are incomplete.',
    );
  }
  return Map<T, String>.unmodifiable(labels);
}

/// Responsive, source-grouped presentation for the bounded global search.
final class Revision3GlobalContentSearchView extends StatefulWidget {
  const Revision3GlobalContentSearchView({
    required this.controller,
    required this.copy,
    required this.callbacks,
    super.key,
  });

  final Revision3GlobalContentSearchController controller;
  final Revision3GlobalContentSearchCopy copy;
  final Revision3GlobalContentSearchCallbacks callbacks;

  @override
  State<Revision3GlobalContentSearchView> createState() =>
      _Revision3GlobalContentSearchViewState();
}

final class _Revision3GlobalContentSearchViewState
    extends State<Revision3GlobalContentSearchView> {
  late final TextEditingController _queryController;
  final Set<Revision3GlobalContentSource> _visibleSources =
      Revision3GlobalContentSource.values.toSet();

  @override
  void initState() {
    super.initState();
    _queryController = TextEditingController(
      text: widget.controller.snapshot.query,
    );
    widget.controller.addListener(_changed);
  }

  @override
  void didUpdateWidget(Revision3GlobalContentSearchView oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.controller == widget.controller) return;
    oldWidget.controller.removeListener(_changed);
    widget.controller.addListener(_changed);
    _queryController.text = widget.controller.snapshot.query;
  }

  @override
  void dispose() {
    widget.controller.removeListener(_changed);
    _queryController.dispose();
    super.dispose();
  }

  void _changed() {
    if (mounted) setState(() {});
  }

  void _search() {
    if (!widget.controller.isLoading) {
      unawaited(widget.controller.search(_queryController.text));
    }
  }

  void _clear() {
    _queryController.clear();
    widget.controller.clear();
  }

  @override
  Widget build(BuildContext context) => Semantics(
    key: const Key('revision3-global-content-search'),
    container: true,
    explicitChildNodes: true,
    label: widget.copy.title,
    child: CustomScrollView(
      slivers: <Widget>[
        SliverToBoxAdapter(child: _header(context)),
        SliverPadding(
          padding: const EdgeInsets.fromLTRB(12, 0, 12, 24),
          sliver: SliverList.list(children: _sourceGroups(context)),
        ),
      ],
    ),
  );

  Widget _header(BuildContext context) => Padding(
    padding: const EdgeInsets.all(16),
    child: Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: <Widget>[
        Text(
          widget.copy.title,
          style: Theme.of(context).textTheme.headlineSmall,
        ),
        const SizedBox(height: 12),
        LayoutBuilder(
          builder: (context, constraints) {
            final field = TextField(
              key: const Key('revision3-global-content-search-field'),
              controller: _queryController,
              textInputAction: TextInputAction.search,
              onSubmitted: widget.controller.isLoading
                  ? null
                  : (_) => _search(),
              decoration: InputDecoration(
                labelText: widget.copy.searchLabel,
                prefixIcon: const Icon(Icons.search),
                border: const OutlineInputBorder(),
                isDense: true,
              ),
            );
            final search = FilledButton.icon(
              key: const Key('revision3-global-content-search-submit'),
              onPressed: widget.controller.isLoading ? null : _search,
              icon: const Icon(Icons.search),
              label: Text(widget.copy.searchAction),
            );
            final clear = OutlinedButton.icon(
              key: const Key('revision3-global-content-search-clear'),
              onPressed: _clear,
              icon: const Icon(Icons.clear),
              label: Text(widget.copy.clearAction),
            );
            if (constraints.maxWidth < 520) {
              return Column(
                crossAxisAlignment: CrossAxisAlignment.stretch,
                children: <Widget>[
                  field,
                  const SizedBox(height: 8),
                  search,
                  const SizedBox(height: 8),
                  clear,
                ],
              );
            }
            return Row(
              crossAxisAlignment: CrossAxisAlignment.end,
              children: <Widget>[
                Expanded(child: field),
                const SizedBox(width: 8),
                search,
                const SizedBox(width: 8),
                clear,
              ],
            );
          },
        ),
        const SizedBox(height: 8),
        Wrap(
          key: const Key('revision3-global-content-search-filters'),
          spacing: 8,
          runSpacing: 4,
          children: <Widget>[
            for (final source in Revision3GlobalContentSource.values)
              FilterChip(
                key: ValueKey<Object>(('global-search-filter', source)),
                label: Text(widget.copy.source(source)),
                selected: _visibleSources.contains(source),
                onSelected: (selected) => setState(() {
                  if (selected) {
                    _visibleSources.add(source);
                  } else {
                    _visibleSources.remove(source);
                  }
                }),
              ),
          ],
        ),
      ],
    ),
  );

  List<Widget> _sourceGroups(BuildContext context) {
    final snapshot = widget.controller.snapshot;
    if (snapshot.query.isEmpty) {
      return <Widget>[
        _MessageCard(
          key: const Key('revision3-global-content-search-empty-prompt'),
          icon: Icons.manage_search,
          message: widget.copy.emptyPrompt,
        ),
      ];
    }
    return <Widget>[
      for (final source in Revision3GlobalContentSource.values)
        if (_visibleSources.contains(source))
          _sourceGroup(context, source, snapshot.stateFor(source)),
    ];
  }

  Widget _sourceGroup(
    BuildContext context,
    Revision3GlobalContentSource source,
    Revision3GlobalContentSourceState state,
  ) => Semantics(
    key: ValueKey<Object>(('revision3-global-content-source', source)),
    container: true,
    explicitChildNodes: true,
    label: widget.copy.source(source),
    child: Padding(
      padding: const EdgeInsets.only(bottom: 12),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: <Widget>[
          Row(
            children: <Widget>[
              Expanded(
                child: Text(
                  widget.copy.source(source),
                  style: Theme.of(context).textTheme.titleMedium,
                ),
              ),
              _phaseBadge(state),
            ],
          ),
          const SizedBox(height: 6),
          switch (state.phase) {
            Revision3GlobalContentSourcePhase.idle => _MessageCard(
              key: ValueKey<Object>(('global-search-idle', source)),
              icon: Icons.refresh,
              message: widget.copy.emptyPrompt,
            ),
            Revision3GlobalContentSourcePhase.loading =>
              LinearProgressIndicator(
                key: ValueKey<Object>(('global-search-loading', source)),
                semanticsLabel: widget.copy.loading,
              ),
            Revision3GlobalContentSourcePhase.error => _errorCard(source),
            Revision3GlobalContentSourcePhase.complete ||
            Revision3GlobalContentSourcePhase.partial =>
              state.results.isEmpty
                  ? _MessageCard(
                      key: ValueKey<Object>((
                        'global-search-no-results',
                        source,
                      )),
                      icon: Icons.search_off,
                      message: widget.copy.noResults,
                    )
                  : Column(
                      children: <Widget>[
                        for (
                          var index = 0;
                          index < state.results.length;
                          index++
                        )
                          _resultCard(source, index, state.results[index]),
                        if (state.truncated)
                          _MessageCard(
                            key: ValueKey<Object>((
                              'global-search-truncated',
                              source,
                            )),
                            icon: Icons.info_outline,
                            message: widget.copy.truncated,
                          ),
                      ],
                    ),
          },
        ],
      ),
    ),
  );

  Widget _phaseBadge(Revision3GlobalContentSourceState state) {
    final label = switch (state.phase) {
      Revision3GlobalContentSourcePhase.idle => '',
      Revision3GlobalContentSourcePhase.loading => widget.copy.loading,
      Revision3GlobalContentSourcePhase.complete => widget.copy.complete,
      Revision3GlobalContentSourcePhase.partial => widget.copy.partial,
      Revision3GlobalContentSourcePhase.error => widget.copy.loadFailed,
    };
    return label.isEmpty ? const SizedBox.shrink() : _Badge(label);
  }

  Widget _errorCard(Revision3GlobalContentSource source) => Card(
    key: ValueKey<Object>(('global-search-error', source)),
    child: Padding(
      padding: const EdgeInsets.all(12),
      child: Row(
        children: <Widget>[
          const Icon(Icons.error_outline),
          const SizedBox(width: 8),
          Expanded(child: Text(widget.copy.loadFailed)),
          TextButton(
            key: ValueKey<Object>(('global-search-retry', source)),
            onPressed: widget.controller.isLoading
                ? null
                : () => unawaited(widget.controller.retrySource(source)),
            child: Text(widget.copy.retry),
          ),
        ],
      ),
    ),
  );

  Widget _resultCard(
    Revision3GlobalContentSource source,
    int index,
    Revision3GlobalContentResult result,
  ) => Card(
    key: ValueKey<Object>(('global-search-result', source, index)),
    child: Padding(
      padding: const EdgeInsets.all(12),
      child: LayoutBuilder(
        builder: (context, constraints) {
          final details = Row(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: <Widget>[
              Icon(_icon(result.kind)),
              const SizedBox(width: 10),
              Expanded(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: <Widget>[
                    Text(
                      result.title,
                      style: Theme.of(context).textTheme.titleSmall,
                    ),
                    const SizedBox(height: 2),
                    Text(result.subtitle),
                    const SizedBox(height: 8),
                    Wrap(
                      spacing: 6,
                      runSpacing: 4,
                      children: <Widget>[
                        _Badge(widget.copy.source(result.source)),
                        _Badge(widget.copy.kind(result.kind)),
                        _Badge(widget.copy.readiness(result.readiness)),
                      ],
                    ),
                  ],
                ),
              ),
            ],
          );
          final action = result.action == null
              ? null
              : FilledButton.tonalIcon(
                  key: ValueKey<Object>((
                    'global-search-action',
                    result.action!.kind,
                    result.action!.identity,
                  )),
                  onPressed: () => widget.callbacks.invoke(result.action!),
                  icon: Icon(_actionIcon(result.action!.kind)),
                  label: Text(_actionLabel(result.action!.kind)),
                );
          if (action == null) return details;
          if (constraints.maxWidth < 560) {
            return Column(
              crossAxisAlignment: CrossAxisAlignment.stretch,
              children: <Widget>[details, const SizedBox(height: 10), action],
            );
          }
          return Row(
            crossAxisAlignment: CrossAxisAlignment.center,
            children: <Widget>[
              Expanded(child: details),
              const SizedBox(width: 12),
              action,
            ],
          );
        },
      ),
    ),
  );

  String _actionLabel(Revision3GlobalContentActionKind kind) => switch (kind) {
    Revision3GlobalContentActionKind.openThisModEntity ||
    Revision3GlobalContentActionKind.openThisModAsset => widget.copy.openAction,
    Revision3GlobalContentActionKind.createBaseNpcDraft ||
    Revision3GlobalContentActionKind.createBaseQuestDraft =>
      widget.copy.createDraftAction,
    Revision3GlobalContentActionKind.inspectInstalledDataAsset =>
      widget.copy.inspectAction,
  };
}

final class _Badge extends StatelessWidget {
  const _Badge(this.label);
  final String label;

  @override
  Widget build(BuildContext context) => DecoratedBox(
    decoration: BoxDecoration(
      color: Theme.of(context).colorScheme.secondaryContainer,
      borderRadius: BorderRadius.circular(999),
    ),
    child: Padding(
      padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 3),
      child: Text(label, style: Theme.of(context).textTheme.labelSmall),
    ),
  );
}

final class _MessageCard extends StatelessWidget {
  const _MessageCard({required this.icon, required this.message, super.key});

  final IconData icon;
  final String message;

  @override
  Widget build(BuildContext context) => Card(
    child: Padding(
      padding: const EdgeInsets.all(16),
      child: Row(
        children: <Widget>[
          Icon(icon),
          const SizedBox(width: 10),
          Expanded(child: Text(message)),
        ],
      ),
    ),
  );
}

IconData _icon(Revision3GlobalContentKind kind) => switch (kind) {
  Revision3GlobalContentKind.thisModEntity => Icons.edit_note,
  Revision3GlobalContentKind.thisModAsset => Icons.inventory_2_outlined,
  Revision3GlobalContentKind.baseNpc ||
  Revision3GlobalContentKind.experimentalBaseNpc => Icons.person_outline,
  Revision3GlobalContentKind.baseQuest => Icons.assignment_outlined,
  Revision3GlobalContentKind.installedDataAsset => Icons.storage_outlined,
};

IconData _actionIcon(Revision3GlobalContentActionKind kind) => switch (kind) {
  Revision3GlobalContentActionKind.openThisModEntity ||
  Revision3GlobalContentActionKind.openThisModAsset => Icons.open_in_new,
  Revision3GlobalContentActionKind.createBaseNpcDraft ||
  Revision3GlobalContentActionKind.createBaseQuestDraft => Icons.add,
  Revision3GlobalContentActionKind.inspectInstalledDataAsset =>
    Icons.search_outlined,
};
