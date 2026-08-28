import 'dart:async';

import 'package:flutter/material.dart';
import 'package:goresave/features/editor/domain/character_category_catalog.dart';
import 'package:goresave/features/editor/domain/character_index.dart';
import 'package:goresave/features/editor/domain/editor_models.dart';
import 'package:goresave/features/editor/domain/editor_notifier.dart';
import 'package:goresave/features/editor/domain/game_time.dart';
import 'package:goresave/features/editor/domain/hero_attributes.dart';
import 'package:goresave/features/editor/domain/progression_models.dart';
import 'package:goresave/features/editor/domain/skills_models.dart';
import 'package:goresave/l10n/app_localizations.dart';
import 'package:intl/intl.dart';

/// Read-only, save-backed statistics for the Overview tab.
///
/// Inspection-backed values render immediately. Slower typed/indexed sources
/// fill their own tiles independently, so one full memory-event scan never
/// holds the complete section behind a spinner.
class OverviewStatisticsSection extends StatefulWidget {
  const OverviewStatisticsSection({
    super.key,
    required this.inspection,
    required this.notifier,
  });

  final SaveInspection inspection;
  final EditorNotifier notifier;

  @override
  State<OverviewStatisticsSection> createState() =>
      _OverviewStatisticsSectionState();
}

class _OverviewStatisticsSectionState extends State<OverviewStatisticsSection> {
  GameTime? _gameTime;
  bool _gameTimeLoaded = false;
  List<HeroAttribute>? _heroAttributes;
  SkillsResult? _skills;
  bool _skillsLoaded = false;
  CharacterIndexPage? _characters;
  bool _charactersLoaded = false;
  CharacterCategoryCatalog? _characterCatalog;
  _EventStatistics _events = const _EventStatistics();
  int _loadEpoch = 0;

  @override
  void initState() {
    super.initState();
    _load();
  }

  @override
  void didUpdateWidget(covariant OverviewStatisticsSection oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (!identical(widget.inspection, oldWidget.inspection)) _load();
  }

  void _load() {
    final epoch = ++_loadEpoch;
    setState(() {
      _gameTime = null;
      _gameTimeLoaded = false;
      _heroAttributes = null;
      _skills = null;
      _skillsLoaded = false;
      _characters = null;
      _charactersLoaded = false;
      _characterCatalog = null;
      _events = const _EventStatistics();
    });

    unawaited(_loadGameTime(epoch));
    unawaited(_loadHeroAttributes(epoch));
    unawaited(_loadSkills(epoch));
    unawaited(_loadCharactersAndEvents(epoch));
  }

  Future<void> _loadGameTime(int epoch) async {
    final value = await widget.notifier.loadGameTime();
    if (!mounted || epoch != _loadEpoch) return;
    setState(() {
      _gameTime = value;
      _gameTimeLoaded = true;
    });
  }

  Future<void> _loadHeroAttributes(int epoch) async {
    final result = await widget.notifier.loadHeroAttributes();
    if (!mounted || epoch != _loadEpoch) return;
    setState(
      () =>
          _heroAttributes = result.error == null ? result.attributes : const [],
    );
  }

  Future<void> _loadSkills(int epoch) async {
    final result = await widget.notifier.loadSkills();
    if (!mounted || epoch != _loadEpoch) return;
    setState(() {
      _skills = result.error == null ? result : null;
      _skillsLoaded = true;
    });
  }

  Future<void> _loadCharactersAndEvents(int epoch) async {
    final catalogFuture = loadCharacterCategoryCatalog()
        .then<CharacterCategoryCatalog?>((catalog) => catalog)
        .catchError((Object _) => null);
    // Queue the save-backed index immediately. Asset I/O must not delay it
    // behind the background prefetch's much longer sequence.
    final page = await widget.notifier.loadAllCharacters();
    // Classification-dependent counts stay unavailable when the catalog
    // cannot load; never guess whether an unknown actor is human or monster.
    final catalog = await catalogFuture;
    if (!mounted || epoch != _loadEpoch) return;
    final characters = page.error == null ? page : null;
    setState(() {
      _characters = characters;
      _charactersLoaded = true;
      _characterCatalog = catalog;
    });

    final heroId = characters?.characters
        .where(
          (row) =>
              row.globalId != null && row.uniqueName.toLowerCase() == 'hero',
        )
        .map((row) => row.globalId!)
        .firstOrNull;
    if (heroId == null || catalog == null) {
      setState(() => _events = const _EventStatistics(loaded: true));
      return;
    }
    final events = await _loadEventStatistics(heroId, epoch, catalog);
    if (!mounted || epoch != _loadEpoch) return;
    setState(() => _events = events);
  }

  Future<_EventStatistics> _loadEventStatistics(
    String heroId,
    int epoch,
    CharacterCategoryCatalog catalog,
  ) async {
    const pageSize = EditorPageSize.statistics;
    var offset = 0;
    var killedMonsters = 0;
    var defeatedNpcs = 0;
    var killedNpcs = 0;
    MemoryEvent? latestGuildEvent;
    while (true) {
      final page = await widget.notifier.loadMemoryEvents(
        heroId,
        offset: offset,
        limit: pageSize,
      );
      if (!mounted || epoch != _loadEpoch) return const _EventStatistics();
      if (page.error != null) {
        return const _EventStatistics(loaded: true);
      }
      for (final event in page.events) {
        final tags = event.tags.map((tag) => tag.toLowerCase()).toSet();
        final killed =
            tags.contains('memory.character.defeated.kill') ||
            tags.contains('memory.execution');
        final defeated =
            tags.contains('memory.character.defeated') ||
            tags.contains('memory.wasdefeated') ||
            tags.contains('memory.combat.wasdefeated') ||
            tags.contains('memory.saveandload.defeated');
        final targetCategory = _targetCategory(event, catalog);
        if (killed && targetCategory == _TargetCategory.monster) {
          killedMonsters++;
        } else if (killed && targetCategory == _TargetCategory.npc) {
          killedNpcs++;
        } else if (defeated && targetCategory == _TargetCategory.npc) {
          defeatedNpcs++;
        }
        if (tags.contains('memory.guild.joined')) {
          final currentTime = event.timeSeconds ?? event.index.toDouble();
          final previousTime =
              latestGuildEvent?.timeSeconds ??
              latestGuildEvent?.index.toDouble() ??
              double.negativeInfinity;
          if (currentTime >= previousTime) latestGuildEvent = event;
        }
      }
      offset += page.events.length;
      if (offset >= page.total || page.events.isEmpty) break;
    }
    return _EventStatistics(
      loaded: true,
      available: true,
      killedMonsters: killedMonsters,
      defeatedNpcs: defeatedNpcs,
      killedNpcs: killedNpcs,
      guildTag: _guildTag(latestGuildEvent),
    );
  }

  static _TargetCategory? _targetCategory(
    MemoryEvent event,
    CharacterCategoryCatalog catalog,
  ) {
    if (event.tags.any(
      (tag) => tag.toLowerCase().startsWith('species.creature.'),
    )) {
      return _TargetCategory.monster;
    }
    return switch (catalog.categoryFor(event.affected)) {
      CharacterCategory.human => _TargetCategory.npc,
      CharacterCategory.creature ||
      CharacterCategory.other => _TargetCategory.monster,
      null => null,
    };
  }

  static String? _guildTag(MemoryEvent? event) {
    if (event == null) return null;
    for (final tag in event.tags) {
      final lower = tag.toLowerCase();
      if (lower == 'memory.guild.joined') continue;
      if (lower.contains('guild')) return tag;
    }
    return event.optionalClass1 ?? event.optionalClass2;
  }

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context);
    final theme = Theme.of(context);
    return Align(
      alignment: Alignment.topLeft,
      child: ConstrainedBox(
        constraints: const BoxConstraints(maxWidth: 1280),
        child: Card(
          key: const ValueKey('overview-statistics-section'),
          child: Padding(
            padding: const EdgeInsets.all(16),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Row(
                  children: [
                    Icon(
                      Icons.analytics_outlined,
                      color: theme.colorScheme.primary,
                    ),
                    const SizedBox(width: 10),
                    Text(
                      l10n.statisticsTitle,
                      style: theme.textTheme.titleMedium,
                    ),
                  ],
                ),
                const SizedBox(height: 18),
                _StatisticsGrid(
                  sections: [
                    _progressSection(context),
                    _characterSection(context),
                    _questSection(context),
                    _encountersSection(context),
                    _inventorySection(context),
                  ],
                ),
              ],
            ),
          ),
        ),
      ),
    );
  }

  _StatisticsSection _progressSection(BuildContext context) {
    final l10n = AppLocalizations.of(context);
    final parts = _gameTime == null
        ? null
        : GameTimeParts.fromTotalSeconds(_gameTime!.totalSeconds);
    return _StatisticsSection(
      key: const ValueKey('statistics-card-time'),
      icon: Icons.explore_outlined,
      title: l10n.statisticsCardTitle('progress', 'Progress'),
      metrics: [
        _Metric(
          label: l10n.statisticsMetric('chapter', 'Chapter'),
          value:
              widget.inspection.chapterId?.toString() ?? l10n.statisticsUnknown,
        ),
        _Metric(
          label: l10n.statisticsMetric('timePlayed', 'Played'),
          value: _playedTime(l10n, widget.inspection.timePlayedSeconds),
        ),
        _Metric(
          label: l10n.statisticsMetric('worldTime', 'World time'),
          value: !_gameTimeLoaded
              ? _loadingValue
              : parts == null
              ? l10n.statisticsUnknown
              : l10n.memoryEventGameTime(parts.day, _clock(parts)),
        ),
      ],
    );
  }

  _StatisticsSection _characterSection(BuildContext context) {
    final l10n = AppLocalizations.of(context);
    final attributes = _attributeValues();
    final attributesLoaded = _heroAttributes != null;
    String attribute(String id) => attributes.containsKey(id)
        ? _number(context, attributes[id])
        : attributesLoaded
        ? l10n.statisticsUnknown
        : _loadingValue;
    String pool(String current, String maximum) =>
        attributes.containsKey(current) || attributes.containsKey(maximum)
        ? _pool(attributes[current], attributes[maximum], context)
        : attributesLoaded
        ? l10n.statisticsUnknown
        : _loadingValue;
    return _StatisticsSection(
      key: const ValueKey('statistics-card-character'),
      icon: Icons.person_outline,
      title: l10n.statisticsCardTitle('character', 'Character'),
      metrics: [
        _Metric(
          label: l10n.statisticsMetric('level', 'Level'),
          value: attribute('Level'),
        ),
        _Metric(
          label: l10n.statisticsMetric('experience', 'Experience'),
          value: attribute('Experience'),
        ),
        _Metric(
          label: l10n.statisticsMetric('learningPoints', 'Learning points'),
          value: attribute('SkillPoints'),
        ),
        _Metric(
          label: l10n.statisticsMetric('guild', 'Guild'),
          value: !_events.loaded
              ? _loadingValue
              : !_events.available
              ? l10n.statisticsUnknown
              : _guildLabel(l10n, _events.guildTag),
        ),
        _Metric(
          label: l10n.statisticsMetric('health', 'Health'),
          value: pool('Health', 'MaxHealth'),
        ),
        _Metric(
          label: l10n.statisticsMetric('mana', 'Mana'),
          value: pool('Mana', 'MaxMana'),
        ),
      ],
    );
  }

  _StatisticsSection _questSection(BuildContext context) {
    final l10n = AppLocalizations.of(context);
    final counts = widget.inspection.privateProgression.questStates;
    final succeeded = _questCount(counts, 'Succeeded');
    final failed = _questCount(counts, 'Failed');
    final running = _questCount(counts, 'Running');
    final available = _questCount(counts, 'Available');
    return _StatisticsSection(
      key: const ValueKey('statistics-card-quests'),
      icon: Icons.assignment_outlined,
      title: l10n.statisticsCardTitle('quests', 'Quests'),
      metrics: [
        _Metric(label: l10n.questStateSucceeded, value: '$succeeded'),
        _Metric(label: l10n.questStateFailed, value: '$failed'),
        _Metric(label: l10n.questStateRunning, value: '$running'),
        _Metric(label: l10n.questStateAvailable, value: '$available'),
      ],
      footer: _QuestBar(
        succeeded: succeeded,
        failed: failed,
        running: running,
        available: available,
      ),
    );
  }

  _StatisticsSection _encountersSection(BuildContext context) {
    final l10n = AppLocalizations.of(context);
    final catalog = _characterCatalog;
    final humanNpcs = catalog == null || _characters == null
        ? null
        : _characters!.characters.where(
            (row) =>
                row.globalId != null &&
                row.uniqueName.toLowerCase() != 'hero' &&
                catalog.isHuman(row.uniqueName),
          );
    String eventValue(int value) => !_events.loaded
        ? _loadingValue
        : !_events.available
        ? l10n.statisticsUnknown
        : '$value';
    return _StatisticsSection(
      key: const ValueKey('statistics-card-progress'),
      icon: Icons.sports_martial_arts_outlined,
      title: l10n.statisticsCardTitle('encounters', 'Combat & contacts'),
      metrics: [
        _Metric(
          label: l10n.statisticsMetric('killedMonsters', 'Killed monsters'),
          value: eventValue(_events.killedMonsters),
        ),
        _Metric(
          label: l10n.statisticsMetric('defeatedNpcs', 'Defeated NPCs'),
          value: eventValue(_events.defeatedNpcs),
        ),
        _Metric(
          label: l10n.statisticsMetric('killedNpcs', 'Killed NPCs'),
          value: eventValue(_events.killedNpcs),
        ),
        _Metric(
          label: l10n.statisticsMetric('knownNpcs', 'Known NPCs'),
          value: !_charactersLoaded
              ? _loadingValue
              : humanNpcs?.length.toString() ?? l10n.statisticsUnknown,
        ),
        _Metric(
          label: l10n.statisticsMetric('traders', 'Known traders'),
          value: !_charactersLoaded
              ? _loadingValue
              : humanNpcs?.where((row) => row.isTrader).length.toString() ??
                    l10n.statisticsUnknown,
        ),
        _Metric(
          label: l10n.statisticsMetric('knownTeachers', 'Known teachers'),
          value: !_charactersLoaded
              ? _loadingValue
              : humanNpcs
                        ?.where((row) => catalog!.isTeacher(row.uniqueName))
                        .length
                        .toString() ??
                    l10n.statisticsUnknown,
        ),
        _Metric(
          label: l10n.statisticsMetric('openCrimes', 'Open crimes'),
          value:
              widget.inspection.privateFactions?.guilds
                  .fold<int>(0, (sum, guild) => sum + guild.unforgiven)
                  .toString() ??
              l10n.statisticsUnknown,
        ),
      ],
    );
  }

  _StatisticsSection _inventorySection(BuildContext context) {
    final l10n = AppLocalizations.of(context);
    final inventory = widget.inspection.privateInventory;
    final totalItems = inventory.items.fold<int>(
      0,
      (sum, item) => sum + (item.count ?? 1),
    );
    final ore = inventory.items
        .where(
          (item) =>
              item.id.toLowerCase().contains('orenugget') ||
              item.path.toLowerCase().contains('orenugget'),
        )
        .fold<int>(0, (sum, item) => sum + (item.count ?? 1));
    return _StatisticsSection(
      key: const ValueKey('statistics-section-inventory'),
      icon: Icons.backpack_outlined,
      title: l10n.statisticsCardTitle('inventory', 'Skills & inventory'),
      metrics: [
        _Metric(
          label: l10n.statisticsMetric('learnedSkills', 'Learned skills'),
          value: !_skillsLoaded
              ? _loadingValue
              : _skills == null
              ? l10n.statisticsUnknown
              : '${_skills!.skills.where((skill) => skill.learned).length}',
        ),
        _Metric(
          label: l10n.statisticsMetric('inventoryItems', 'Items'),
          value: '$totalItems',
        ),
        _Metric(label: l10n.statisticsMetric('ore', 'Ore'), value: '$ore'),
      ],
    );
  }

  Map<String, double> _attributeValues() {
    final values = <String, double>{};
    for (final attribute in widget.inspection.privatePlayer.attributes) {
      final value = attribute.currentValue ?? attribute.baseValue;
      if (value != null) values[attribute.id] = value;
    }
    for (final attribute in _heroAttributes ?? const <HeroAttribute>[]) {
      final value = attribute.currentValue ?? attribute.baseValue;
      if (value != null) values[attribute.id] = value;
    }
    return values;
  }

  String _number(BuildContext context, num? value) {
    if (value == null) return AppLocalizations.of(context).statisticsUnknown;
    return NumberFormat.decimalPattern(
      Localizations.localeOf(context).toLanguageTag(),
    ).format(value.round());
  }

  String _pool(num? current, num? maximum, BuildContext context) {
    if (current == null && maximum == null) {
      return AppLocalizations.of(context).statisticsUnknown;
    }
    if (maximum == null) return _number(context, current);
    return '${_number(context, current)} / ${_number(context, maximum)}';
  }

  static int _questCount(Map<String, int> counts, String state) {
    final suffix = state.toLowerCase();
    return counts.entries
        .where((entry) => entry.key.toLowerCase().endsWith(suffix))
        .fold(0, (sum, entry) => sum + entry.value);
  }

  static String _clock(GameTimeParts parts) => [
    parts.hour,
    parts.minute,
    parts.second,
  ].map((value) => value.toString().padLeft(2, '0')).join(':');

  static String _playedTime(AppLocalizations l10n, double? seconds) {
    if (seconds == null || !seconds.isFinite) return l10n.statisticsUnknown;
    final minutes = ((seconds < 0 ? 0 : seconds) / 60).floor();
    final hours = minutes ~/ 60;
    final remainder = minutes % 60;
    if (hours == 0) return l10n.durationMinutes(remainder);
    if (remainder == 0) return l10n.durationHours(hours);
    return l10n.durationHoursMinutes(hours, remainder);
  }

  static String _guildLabel(AppLocalizations l10n, String? guild) {
    if (guild == null || guild.isEmpty) return l10n.statisticsUnknown;
    final normalized = guild.toLowerCase().replaceAll('_', '.');
    final rank = switch (normalized) {
      final value when value.contains('oldcamp.firemage') => 'oldCampFireMage',
      final value when value.contains('oldcamp.guard') => 'oldCampGuard',
      final value when value.contains('oldcamp') => 'oldCampShadow',
      final value when value.contains('newcamp.watermage') =>
        'newCampWaterMage',
      final value when value.contains('newcamp.mercenary') =>
        'newCampMercenary',
      final value when value.contains('newcamp.rogue') => 'newCampRogue',
      final value when value.contains('newcamp') => 'newCampRogue',
      final value when value.contains('swampcamp.templar') =>
        'swampCampTemplar',
      final value when value.contains('swampcamp.novice') => 'swampCampNovice',
      final value when value.contains('swampcamp') => 'swampCampNovice',
      _ => null,
    };
    if (rank == null) return guild.split(RegExp(r'[._]')).last;
    return l10n.statisticsGuildRank(rank, guild).replaceFirst(' · ', '\n');
  }
}

const _loadingValue = '…';

enum _TargetCategory { monster, npc }

class _EventStatistics {
  const _EventStatistics({
    this.loaded = false,
    this.available = false,
    this.killedMonsters = 0,
    this.defeatedNpcs = 0,
    this.killedNpcs = 0,
    this.guildTag,
  });

  final bool loaded;
  final bool available;
  final int killedMonsters;
  final int defeatedNpcs;
  final int killedNpcs;
  final String? guildTag;
}

class _Metric {
  const _Metric({required this.label, required this.value});

  final String label;
  final String value;
}

const _statisticsCardSpacing = 14.0;
const _statisticsCardMinWidth = 300.0;
const _statisticsCardMaxColumns = 3;
const _statisticsHeaderHeight = 40.0;
const _statisticsMetricHeight = 80.0;
const _statisticsMetricSpacing = 8.0;
const _statisticsMetricMinWidth = 130.0;

class _StatisticsGrid extends StatelessWidget {
  const _StatisticsGrid({required this.sections});

  final List<_StatisticsSection> sections;

  @override
  Widget build(BuildContext context) {
    return LayoutBuilder(
      builder: (context, constraints) {
        final columnCount = _cardColumnCount(constraints.maxWidth);
        final rows = <Widget>[];
        for (var start = 0; start < sections.length; start += columnCount) {
          final end = start + columnCount < sections.length
              ? start + columnCount
              : sections.length;
          final rowSections = sections.sublist(start, end);
          final rowColumnCount = rowSections.length;
          final cardWidth =
              (constraints.maxWidth -
                  _statisticsCardSpacing * (rowColumnCount - 1)) /
              rowColumnCount;
          final metricColumns = _metricColumnCount(cardWidth - 28);
          final rowHeight = rowSections
              .map((section) => section.heightFor(metricColumns))
              .reduce(
                (height, candidate) => height > candidate ? height : candidate,
              );
          if (rows.isNotEmpty) {
            rows.add(const SizedBox(height: _statisticsCardSpacing));
          }
          rows.add(
            SizedBox(
              height: rowHeight,
              child: Row(
                crossAxisAlignment: CrossAxisAlignment.stretch,
                children: [
                  for (var index = 0; index < rowSections.length; index++) ...[
                    if (index > 0)
                      const SizedBox(width: _statisticsCardSpacing),
                    Expanded(child: rowSections[index]),
                  ],
                ],
              ),
            ),
          );
        }
        return Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: rows,
        );
      },
    );
  }

  static int _cardColumnCount(double width) {
    for (var count = _statisticsCardMaxColumns; count > 1; count--) {
      final requiredWidth =
          _statisticsCardMinWidth * count +
          _statisticsCardSpacing * (count - 1);
      if (width >= requiredWidth) return count;
    }
    return 1;
  }
}

int _metricColumnCount(double width) {
  for (var count = 3; count > 1; count--) {
    final requiredWidth =
        _statisticsMetricMinWidth * count +
        _statisticsMetricSpacing * (count - 1);
    if (width >= requiredWidth) return count;
  }
  return 1;
}

class _StatisticsSection extends StatelessWidget {
  const _StatisticsSection({
    super.key,
    required this.icon,
    required this.title,
    required this.metrics,
    this.footer,
  });

  final IconData icon;
  final String title;
  final List<_Metric> metrics;
  final Widget? footer;

  double heightFor(int metricColumns) {
    final metricRows = (metrics.length / metricColumns).ceil();
    final metricsHeight =
        metricRows * _statisticsMetricHeight +
        (metricRows - 1) * _statisticsMetricSpacing;
    final footerHeight = footer == null ? 0 : 17;
    return 28 + _statisticsHeaderHeight + 11 + metricsHeight + footerHeight;
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return DecoratedBox(
      decoration: BoxDecoration(
        color: theme.colorScheme.surfaceContainerLow,
        border: Border.all(color: theme.colorScheme.outlineVariant),
        borderRadius: BorderRadius.circular(12),
      ),
      child: Padding(
        padding: const EdgeInsets.all(14),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            SizedBox(
              height: _statisticsHeaderHeight,
              child: Row(
                children: [
                  DecoratedBox(
                    decoration: BoxDecoration(
                      color: theme.colorScheme.primaryContainer,
                      borderRadius: BorderRadius.circular(7),
                    ),
                    child: Padding(
                      padding: const EdgeInsets.all(7),
                      child: Icon(
                        icon,
                        size: 18,
                        color: theme.colorScheme.onPrimaryContainer,
                      ),
                    ),
                  ),
                  const SizedBox(width: 10),
                  Expanded(
                    child: Text(
                      title,
                      maxLines: 2,
                      overflow: TextOverflow.ellipsis,
                      style: theme.textTheme.titleSmall?.copyWith(
                        fontWeight: FontWeight.w600,
                      ),
                    ),
                  ),
                ],
              ),
            ),
            const SizedBox(height: 11),
            LayoutBuilder(
              builder: (context, constraints) {
                final columnCount = _metricColumnCount(constraints.maxWidth);
                final width =
                    (constraints.maxWidth -
                        _statisticsMetricSpacing * (columnCount - 1)) /
                    columnCount;
                return Wrap(
                  spacing: _statisticsMetricSpacing,
                  runSpacing: _statisticsMetricSpacing,
                  children: [
                    for (final metric in metrics)
                      SizedBox(
                        width: width,
                        height: _statisticsMetricHeight,
                        child: _DetailMetric(metric: metric),
                      ),
                  ],
                );
              },
            ),
            if (footer != null) ...[const SizedBox(height: 10), footer!],
          ],
        ),
      ),
    );
  }
}

class _DetailMetric extends StatelessWidget {
  const _DetailMetric({required this.metric});

  final _Metric metric;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Padding(
      padding: const EdgeInsets.symmetric(horizontal: 4),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Text(metric.label, maxLines: 2, style: theme.textTheme.bodySmall),
          const SizedBox(height: 2),
          Text(metric.value, maxLines: 2, style: theme.textTheme.labelLarge),
        ],
      ),
    );
  }
}

class _QuestBar extends StatelessWidget {
  const _QuestBar({
    required this.succeeded,
    required this.failed,
    required this.running,
    required this.available,
  });

  final int succeeded;
  final int failed;
  final int running;
  final int available;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final segments = [
      (succeeded, Colors.green),
      (failed, theme.colorScheme.error),
      (running, Colors.amber.shade700),
      (available, theme.colorScheme.outline),
    ].where((segment) => segment.$1 > 0).toList();
    if (segments.isEmpty) return const SizedBox.shrink();
    return ClipRRect(
      borderRadius: BorderRadius.circular(4),
      child: SizedBox(
        height: 7,
        child: Row(
          children: [
            for (final segment in segments)
              Expanded(
                flex: segment.$1,
                child: ColoredBox(color: segment.$2),
              ),
          ],
        ),
      ),
    );
  }
}
