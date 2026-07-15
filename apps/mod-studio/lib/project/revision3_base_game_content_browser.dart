import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../core/mod_ffi.dart';
import '../core/providers.dart';
import '../story/domain/story_catalog_adapter.dart';
import '../story/domain/story_npc_archetype_index.dart';
import 'revision3_npc_authoring.dart';
import 'revision3_quest_authoring.dart';

const _maxExperimentalRows = 100;

/// One immutable, generation-consistent projection for the Base game scope.
///
/// Both child catalogs originate from the same native Story selection and NPC
/// archetype evidence. Holding them together prevents the UI from presenting a
/// Quest parent from one game generation beside NPC choices from another.
final class Revision3BaseGameContentCatalog {
  const Revision3BaseGameContentCatalog({
    required this.npcs,
    required this.quests,
  });

  final Revision3NpcCatalog npcs;
  final Revision3QuestCatalog quests;
}

typedef Revision3BaseGameContentCatalogLoader =
    Future<Revision3BaseGameContentCatalog> Function(String gameRoot);

/// Rebuilds one Base game snapshot without writing the installation or a save.
///
/// The two native reads start together. Exactly one adapter performs the
/// generation/catalog-seal join, and both visible catalogs are projected from
/// that adapter and the same Story seals.
final class Revision3BaseGameContentCatalogService {
  const Revision3BaseGameContentCatalogService(this._ffi);

  final ModFfi _ffi;

  Future<Revision3BaseGameContentCatalog> load(String gameRoot) async {
    if (gameRoot.isEmpty) {
      throw const FormatException(
        'A configured game installation is required.',
      );
    }

    final storyFuture = _ffi.authoringStoryCatalogV1BuildAndReadForGameRoot(
      gameRoot: gameRoot,
    );
    final archetypesFuture = _ffi
        .authoringNpcArchetypeCatalogV1BuildForGameRoot(gameRoot: gameRoot);
    final evidence = await Future.wait<Object>([
      storyFuture,
      archetypesFuture,
    ], eagerError: false);
    final story = evidence[0] as AuthoringStoryCatalogSelections;
    final archetypes = evidence[1] as AuthoringNpcArchetypeCatalogBuildResult;
    final adapter = StoryCatalogAdapter.fromSelectionsAndArchetypes(
      story,
      archetypes,
    );

    return Revision3BaseGameContentCatalog(
      npcs: Revision3NpcCatalog.fromStoryCatalog(adapter),
      quests: Revision3QuestCatalog.fromStoryCatalog(
        adapter,
        catalogSeal: story.catalogSeal,
        generationExecutableSeal: story.generation.executable,
      ),
    );
  }
}

final revision3BaseGameContentCatalogLoaderProvider =
    Provider<Revision3BaseGameContentCatalogLoader>(
      (ref) => Revision3BaseGameContentCatalogService(
        ModFfi(ref.read(coreServiceProvider)),
      ).load,
    );

/// Localized presentation copy supplied by the managed-project host.
final class Revision3BaseGameContentBrowserCopy {
  const Revision3BaseGameContentBrowserCopy({
    required this.title,
    required this.description,
    required this.missingGameTitle,
    required this.missingGameDescription,
    required this.configureGame,
    required this.loading,
    required this.refresh,
    required this.searchLabel,
    required this.filterAll,
    required this.filterNpcs,
    required this.filterQuests,
    required this.npcSectionTitle,
    required this.questSectionTitle,
    required this.experimentalNpcSectionTitle,
    required this.searchForExperimental,
    required this.empty,
    required this.loadErrorTitle,
    required this.loadErrorDescription,
    required this.retry,
    required this.baseGameSourceBadge,
    required this.offlineDraftBadge,
    required this.runtimeUnqualifiedBadge,
    required this.inspectOnlyBadge,
    required this.createNpcDraft,
    required this.createQuestDraft,
    required this.spawnClass,
    required this.actorBlueprint,
    required this.experimentalResultsCapped,
  });

  final String title;
  final String description;
  final String missingGameTitle;
  final String missingGameDescription;
  final String configureGame;
  final String loading;
  final String refresh;
  final String searchLabel;
  final String filterAll;
  final String filterNpcs;
  final String filterQuests;
  final String npcSectionTitle;
  final String questSectionTitle;
  final String experimentalNpcSectionTitle;
  final String searchForExperimental;
  final String empty;
  final String loadErrorTitle;
  final String loadErrorDescription;
  final String retry;
  final String baseGameSourceBadge;
  final String offlineDraftBadge;
  final String runtimeUnqualifiedBadge;
  final String inspectOnlyBadge;
  final String createNpcDraft;
  final String createQuestDraft;
  final String spawnClass;
  final String actorBlueprint;
  final String experimentalResultsCapped;
}

enum Revision3BaseGameContentFilter { all, npc, quest }

/// Read-only Base game catalog browser with explicitly bounded Draft actions.
///
/// Curated NPC and Quest rows emit only their catalog IDs. The broad native NPC
/// inventory appears only after a non-empty search and never exposes an action.
class Revision3BaseGameContentBrowser extends StatefulWidget {
  const Revision3BaseGameContentBrowser({
    required this.gameRoot,
    required this.sourceIdentity,
    required this.loader,
    required this.copy,
    required this.createNpcDraft,
    required this.createQuestDraft,
    this.openSettings,
    super.key,
  });

  final String? gameRoot;
  final Object sourceIdentity;
  final Revision3BaseGameContentCatalogLoader loader;
  final Revision3BaseGameContentBrowserCopy copy;
  final VoidCallback? openSettings;
  final ValueChanged<String> createNpcDraft;
  final ValueChanged<String> createQuestDraft;

  @override
  State<Revision3BaseGameContentBrowser> createState() =>
      _Revision3BaseGameContentBrowserState();
}

class _Revision3BaseGameContentBrowserState
    extends State<Revision3BaseGameContentBrowser> {
  final TextEditingController _searchController = TextEditingController();
  Revision3BaseGameContentCatalog? _catalog;
  Object? _error;
  String _query = '';
  Revision3BaseGameContentFilter _filter = Revision3BaseGameContentFilter.all;
  bool _loading = false;
  int _loadEpoch = 0;

  String? get _usableGameRoot {
    final value = widget.gameRoot;
    if (value == null || value.trim().isEmpty) return null;
    return value;
  }

  @override
  void initState() {
    super.initState();
    _beginLoad(notify: false);
  }

  @override
  void didUpdateWidget(covariant Revision3BaseGameContentBrowser oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.gameRoot == widget.gameRoot &&
        oldWidget.sourceIdentity == widget.sourceIdentity) {
      return;
    }
    _searchController.clear();
    _query = '';
    _filter = Revision3BaseGameContentFilter.all;
    _beginLoad();
  }

  @override
  void dispose() {
    _loadEpoch++;
    _searchController.dispose();
    super.dispose();
  }

  void _beginLoad({bool notify = true}) {
    final epoch = ++_loadEpoch;
    final gameRoot = _usableGameRoot;
    void reset() {
      _catalog = null;
      _error = null;
      _loading = gameRoot != null;
    }

    if (notify) {
      setState(reset);
    } else {
      reset();
    }
    if (gameRoot == null) return;

    Future<Revision3BaseGameContentCatalog>.sync(
      () => widget.loader(gameRoot),
    ).then(
      (catalog) {
        if (!mounted || epoch != _loadEpoch) return;
        setState(() {
          _catalog = catalog;
          _error = null;
          _loading = false;
        });
      },
      onError: (Object error, StackTrace _) {
        if (!mounted || epoch != _loadEpoch) return;
        setState(() {
          _catalog = null;
          _error = error;
          _loading = false;
        });
      },
    );
  }

  @override
  Widget build(BuildContext context) => Semantics(
    key: const Key('revision3-base-game-content-browser'),
    container: true,
    explicitChildNodes: true,
    child: _buildState(context),
  );

  Widget _buildState(BuildContext context) {
    if (_usableGameRoot == null) return _buildMissingGame(context);
    if (_loading) return _buildLoading(context);
    if (_error != null) return _buildError(context);
    final catalog = _catalog;
    if (catalog == null) return _buildError(context);
    return _buildCatalog(context, catalog);
  }

  Widget _buildMissingGame(BuildContext context) => _StateScroller(
    key: const Key('revision3-base-game-content-browser-missing-game'),
    icon: Icons.sports_esports_outlined,
    title: widget.copy.missingGameTitle,
    description: widget.copy.missingGameDescription,
    action: widget.openSettings == null
        ? null
        : FilledButton.icon(
            key: const Key('revision3-base-game-content-browser-open-settings'),
            onPressed: widget.openSettings,
            icon: const Icon(Icons.settings_outlined),
            label: Text(widget.copy.configureGame),
          ),
  );

  Widget _buildLoading(BuildContext context) => Center(
    key: const Key('revision3-base-game-content-browser-loading'),
    child: SingleChildScrollView(
      padding: const EdgeInsets.all(16),
      child: Column(
        mainAxisSize: MainAxisSize.min,
        children: [
          const CircularProgressIndicator(),
          const SizedBox(height: 12),
          Text(widget.copy.loading, textAlign: TextAlign.center),
        ],
      ),
    ),
  );

  Widget _buildError(BuildContext context) => _StateScroller(
    key: const Key('revision3-base-game-content-browser-error'),
    icon: Icons.error_outline,
    title: widget.copy.loadErrorTitle,
    description: widget.copy.loadErrorDescription,
    action: FilledButton.icon(
      key: const Key('revision3-base-game-content-browser-retry'),
      onPressed: _beginLoad,
      icon: const Icon(Icons.refresh),
      label: Text(widget.copy.retry),
    ),
  );

  Widget _buildCatalog(
    BuildContext context,
    Revision3BaseGameContentCatalog catalog,
  ) {
    final entries = _entries(catalog);
    return CustomScrollView(
      key: const Key('revision3-base-game-content-browser-results'),
      slivers: [
        SliverToBoxAdapter(child: _buildHeader(context)),
        SliverPadding(
          padding: const EdgeInsets.fromLTRB(12, 0, 12, 20),
          sliver: SliverList.builder(
            itemCount: entries.length,
            itemBuilder: (context, index) => _buildEntry(entries[index]),
          ),
        ),
      ],
    );
  }

  Widget _buildHeader(BuildContext context) => Padding(
    padding: const EdgeInsets.fromLTRB(16, 16, 16, 12),
    child: Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        Row(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Expanded(
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Text(
                    widget.copy.title,
                    key: const Key('revision3-base-game-content-browser-title'),
                    style: Theme.of(context).textTheme.headlineSmall,
                  ),
                  const SizedBox(height: 4),
                  Text(widget.copy.description),
                ],
              ),
            ),
            IconButton(
              key: const Key('revision3-base-game-content-browser-refresh'),
              tooltip: widget.copy.refresh,
              onPressed: _beginLoad,
              icon: const Icon(Icons.refresh),
            ),
          ],
        ),
        const SizedBox(height: 12),
        TextField(
          key: const Key('revision3-base-game-content-browser-search'),
          controller: _searchController,
          textInputAction: TextInputAction.search,
          decoration: InputDecoration(
            labelText: widget.copy.searchLabel,
            prefixIcon: const Icon(Icons.search),
            border: const OutlineInputBorder(),
            isDense: true,
          ),
          onChanged: (value) => setState(() => _query = value),
        ),
        const SizedBox(height: 8),
        Wrap(
          key: const Key('revision3-base-game-content-browser-filters'),
          spacing: 8,
          runSpacing: 4,
          children: [
            _filterChip(
              Revision3BaseGameContentFilter.all,
              widget.copy.filterAll,
              'all',
            ),
            _filterChip(
              Revision3BaseGameContentFilter.npc,
              widget.copy.filterNpcs,
              'npc',
            ),
            _filterChip(
              Revision3BaseGameContentFilter.quest,
              widget.copy.filterQuests,
              'quest',
            ),
          ],
        ),
      ],
    ),
  );

  Widget _filterChip(
    Revision3BaseGameContentFilter value,
    String label,
    String keySuffix,
  ) => ChoiceChip(
    key: Key('revision3-base-game-content-browser-filter-$keySuffix'),
    label: Text(label),
    selected: _filter == value,
    onSelected: (_) => setState(() => _filter = value),
  );

  List<_BrowserEntry> _entries(Revision3BaseGameContentCatalog catalog) {
    final query = _query.trim().toLowerCase();
    final entries = <_BrowserEntry>[];
    var contentRows = 0;

    if (_filter != Revision3BaseGameContentFilter.quest) {
      final npcs = catalog.npcs.choices
          .where(
            (choice) =>
                query.isEmpty ||
                choice.displayName.toLowerCase().contains(query),
          )
          .toList(growable: false);
      if (npcs.isNotEmpty) {
        entries.add(_SectionEntry('curated-npcs', widget.copy.npcSectionTitle));
        entries.addAll(npcs.map(_NpcEntry.new));
        contentRows += npcs.length;
      }
    }

    if (_filter != Revision3BaseGameContentFilter.npc) {
      final quests = catalog.quests.parents
          .where(
            (choice) =>
                query.isEmpty ||
                choice.displayLabel.toLowerCase().contains(query),
          )
          .toList(growable: false);
      if (quests.isNotEmpty) {
        entries.add(
          _SectionEntry('quest-parents', widget.copy.questSectionTitle),
        );
        entries.addAll(quests.map(_QuestEntry.new));
        contentRows += quests.length;
      }
    }

    if (_filter != Revision3BaseGameContentFilter.quest) {
      if (query.isEmpty) {
        entries.add(
          _MessageEntry(
            'experimental-search-hint',
            widget.copy.searchForExperimental,
          ),
        );
      } else {
        final index = catalog.npcs.archetypeIndex;
        final allExperimental =
            index?.searchExperimental(query, limit: _maxExperimentalRows + 1) ??
            const <StoryNpcArchetypeRow>[];
        if (allExperimental.isNotEmpty) {
          entries.add(
            _SectionEntry(
              'experimental-npcs',
              widget.copy.experimentalNpcSectionTitle,
            ),
          );
          entries.addAll(
            allExperimental
                .take(_maxExperimentalRows)
                .map(_ExperimentalNpcEntry.new),
          );
          contentRows += allExperimental.length.clamp(0, _maxExperimentalRows);
          if (allExperimental.length > _maxExperimentalRows) {
            entries.add(
              _MessageEntry(
                'experimental-results-capped',
                widget.copy.experimentalResultsCapped,
              ),
            );
          }
        }
      }
    }

    if (contentRows == 0) {
      entries.insert(0, _MessageEntry('empty', widget.copy.empty));
    }
    return entries;
  }

  Widget _buildEntry(_BrowserEntry entry) => switch (entry) {
    _SectionEntry() => Padding(
      key: ValueKey(('revision3-base-game-section', entry.id)),
      padding: const EdgeInsets.fromLTRB(4, 12, 4, 4),
      child: Text(entry.title, style: Theme.of(context).textTheme.titleMedium),
    ),
    _MessageEntry() => Padding(
      key: ValueKey(('revision3-base-game-message', entry.id)),
      padding: const EdgeInsets.symmetric(horizontal: 4, vertical: 12),
      child: Text(entry.message, textAlign: TextAlign.center),
    ),
    _NpcEntry() => _CuratedCatalogCard(
      key: ValueKey(('revision3-base-game-npc', entry.choice.catalogId)),
      icon: Icons.person_outline,
      title: entry.choice.displayName,
      copy: widget.copy,
      actionKey: ValueKey((
        'revision3-base-game-create-npc',
        entry.choice.catalogId,
      )),
      actionLabel: widget.copy.createNpcDraft,
      onAction: () => widget.createNpcDraft(entry.choice.catalogId),
    ),
    _QuestEntry() => _CuratedCatalogCard(
      key: ValueKey(('revision3-base-game-quest', entry.choice.catalogId)),
      icon: Icons.assignment_outlined,
      title: entry.choice.displayLabel,
      copy: widget.copy,
      actionKey: ValueKey((
        'revision3-base-game-create-quest',
        entry.choice.catalogId,
      )),
      actionLabel: widget.copy.createQuestDraft,
      onAction: () => widget.createQuestDraft(entry.choice.catalogId),
    ),
    _ExperimentalNpcEntry() => _ExperimentalNpcCard(
      key: ValueKey((
        'revision3-base-game-experimental-npc',
        entry.row.spawnClass,
      )),
      row: entry.row,
      copy: widget.copy,
    ),
  };
}

sealed class _BrowserEntry {}

final class _SectionEntry extends _BrowserEntry {
  _SectionEntry(this.id, this.title);
  final String id;
  final String title;
}

final class _MessageEntry extends _BrowserEntry {
  _MessageEntry(this.id, this.message);
  final String id;
  final String message;
}

final class _NpcEntry extends _BrowserEntry {
  _NpcEntry(this.choice);
  final Revision3NpcCatalogChoice choice;
}

final class _QuestEntry extends _BrowserEntry {
  _QuestEntry(this.choice);
  final Revision3QuestParentChoice choice;
}

final class _ExperimentalNpcEntry extends _BrowserEntry {
  _ExperimentalNpcEntry(this.row);
  final StoryNpcArchetypeRow row;
}

class _CuratedCatalogCard extends StatelessWidget {
  const _CuratedCatalogCard({
    required this.icon,
    required this.title,
    required this.copy,
    required this.actionKey,
    required this.actionLabel,
    required this.onAction,
    super.key,
  });

  final IconData icon;
  final String title;
  final Revision3BaseGameContentBrowserCopy copy;
  final Key actionKey;
  final String actionLabel;
  final VoidCallback onAction;

  @override
  Widget build(BuildContext context) => Card(
    child: Padding(
      padding: const EdgeInsets.all(12),
      child: LayoutBuilder(
        builder: (context, constraints) {
          final details = Row(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Icon(icon),
              const SizedBox(width: 10),
              Expanded(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Text(title, style: Theme.of(context).textTheme.titleMedium),
                    const SizedBox(height: 8),
                    Wrap(
                      spacing: 6,
                      runSpacing: 4,
                      children: [
                        _StatusChip(copy.baseGameSourceBadge),
                        _StatusChip(copy.offlineDraftBadge),
                        _StatusChip(copy.runtimeUnqualifiedBadge),
                      ],
                    ),
                  ],
                ),
              ),
            ],
          );
          final action = FilledButton.tonalIcon(
            key: actionKey,
            onPressed: onAction,
            icon: const Icon(Icons.add),
            label: Text(actionLabel),
          );
          if (constraints.maxWidth < 520) {
            return Column(
              crossAxisAlignment: CrossAxisAlignment.stretch,
              children: [details, const SizedBox(height: 12), action],
            );
          }
          return Row(
            children: [
              Expanded(child: details),
              const SizedBox(width: 12),
              action,
            ],
          );
        },
      ),
    ),
  );
}

class _ExperimentalNpcCard extends StatelessWidget {
  const _ExperimentalNpcCard({
    required this.row,
    required this.copy,
    super.key,
  });

  final StoryNpcArchetypeRow row;
  final Revision3BaseGameContentBrowserCopy copy;

  @override
  Widget build(BuildContext context) => Semantics(
    container: true,
    label: '${row.label}, ${copy.inspectOnlyBadge}',
    child: Card(
      child: Padding(
        padding: const EdgeInsets.all(12),
        child: Row(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            const Icon(Icons.science_outlined),
            const SizedBox(width: 10),
            Expanded(
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Text(
                    row.label,
                    style: Theme.of(context).textTheme.titleMedium,
                    maxLines: 2,
                    overflow: TextOverflow.ellipsis,
                  ),
                  const SizedBox(height: 6),
                  Text(
                    '${copy.spawnClass}: ${row.spawnClass}\n'
                    '${copy.actorBlueprint}: ${row.actorBlueprint}',
                    maxLines: 3,
                    overflow: TextOverflow.ellipsis,
                  ),
                  const SizedBox(height: 8),
                  Wrap(
                    spacing: 6,
                    runSpacing: 4,
                    children: [
                      _StatusChip(copy.baseGameSourceBadge),
                      _StatusChip(copy.inspectOnlyBadge),
                      _StatusChip(copy.runtimeUnqualifiedBadge),
                    ],
                  ),
                ],
              ),
            ),
          ],
        ),
      ),
    ),
  );
}

class _StatusChip extends StatelessWidget {
  const _StatusChip(this.label);
  final String label;

  @override
  Widget build(BuildContext context) =>
      Chip(visualDensity: VisualDensity.compact, label: Text(label));
}

class _StateScroller extends StatelessWidget {
  const _StateScroller({
    required this.icon,
    required this.title,
    required this.description,
    this.action,
    super.key,
  });

  final IconData icon;
  final String title;
  final String description;
  final Widget? action;

  @override
  Widget build(BuildContext context) => SingleChildScrollView(
    padding: const EdgeInsets.all(20),
    child: Center(
      child: ConstrainedBox(
        constraints: const BoxConstraints(maxWidth: 520),
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            Icon(icon, size: 40),
            const SizedBox(height: 12),
            Text(
              title,
              style: Theme.of(context).textTheme.titleLarge,
              textAlign: TextAlign.center,
            ),
            const SizedBox(height: 8),
            Text(description, textAlign: TextAlign.center),
            if (action != null) ...[const SizedBox(height: 16), action!],
          ],
        ),
      ),
    ),
  );
}
