import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:goresave/features/app/domain/ui_settings.dart';
import 'package:goresave/l10n/app_localizations.dart';
import 'package:goresave/loc/game_lang.dart';
import 'package:goresave/loc/loc_catalog_provider.dart';
import 'package:goresave/ui/design/app_theme.dart';

import '../domain/editor_models.dart';
import '../domain/editor_notifier.dart';
import '../domain/game_time.dart';
import '../domain/glossary_npc_catalog.dart';
import '../domain/glossary_segment_text_catalog.dart';
import '../domain/story_state_models.dart';
import '../domain/story_state_presentation.dart';
import '../domain/story_state_semantics.dart';

enum _StoryFilter { all, integer, timeMarker, chapter, unknown }

enum _StoryPresence { stored, unset, all }

typedef StoryStatePageLoader =
    Future<StoryStatePage> Function({
      required int offset,
      required int limit,
      required String? path,
    });

/// Source-aware catalog and transactional editor for the sparse
/// `StoryPropertyValues` map.
///
/// Every value keeps a raw signed-int32 escape hatch. Convenience controls are
/// enabled only where the shipped script cache gives stronger evidence (for
/// example an exact `FInGameTime` declaration or 0/1-only script writes).
class StoryStateDetail extends ConsumerStatefulWidget {
  const StoryStateDetail({
    super.key,
    required this.notifier,
    required this.reloadKey,
    required this.theme,
    this.editable = false,
    this.storyLoader,
    this.npcCatalogLoader,
    this.segmentTextCatalogLoader,
  });

  final EditorNotifier notifier;
  final SaveInspection reloadKey;
  final ThemeData theme;
  final bool editable;
  final StoryStatePageLoader? storyLoader;
  final Future<List<NpcGlossaryCatalogEntry>> Function()? npcCatalogLoader;
  final Future<GlossarySegmentTextCatalog> Function()? segmentTextCatalogLoader;

  @override
  ConsumerState<StoryStateDetail> createState() => _StoryStateDetailState();
}

class _StoryStateDetailState extends ConsumerState<StoryStateDetail> {
  final TextEditingController _search = TextEditingController();
  StoryStatePage _page = const StoryStatePage();
  List<NpcGlossaryCatalogEntry> _npcCatalog = const [];
  GlossarySegmentTextCatalog _segmentTextCatalog = const {};
  _StoryFilter _filter = _StoryFilter.all;
  _StoryPresence _presence = _StoryPresence.stored;
  String _query = '';
  bool _showInfo = false;
  bool _loading = false;
  int _loadEpoch = 0;

  @override
  void initState() {
    super.initState();
    _load();
  }

  @override
  void didUpdateWidget(covariant StoryStateDetail oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (widget.reloadKey != oldWidget.reloadKey ||
        !identical(widget.notifier, oldWidget.notifier)) {
      final sameSave =
          identical(widget.notifier, oldWidget.notifier) &&
          widget.reloadKey.path != null &&
          widget.reloadKey.path == oldWidget.reloadKey.path;
      if (!sameSave) {
        _search.clear();
        _query = '';
        _filter = _StoryFilter.all;
        _presence = _StoryPresence.stored;
        _showInfo = false;
        _page = const StoryStatePage();
      }
      _load();
    }
  }

  @override
  void dispose() {
    _search.dispose();
    super.dispose();
  }

  Future<void> _load() async {
    final epoch = ++_loadEpoch;
    setState(() => _loading = true);
    final page = await _loadCompleteStoryState();
    final npcCatalog = await _loadNpcCatalogSafely();
    final textCatalog = await _loadSegmentTextCatalogSafely();
    if (!mounted || epoch != _loadEpoch) return;
    setState(() {
      _page = page;
      _npcCatalog = npcCatalog;
      _segmentTextCatalog = textCatalog;
      _loading = false;
    });
  }

  Future<StoryStatePage> _loadCompleteStoryState() async {
    const pageSize = 1000;
    final pinnedPath = widget.reloadKey.path;
    final loader =
        widget.storyLoader ??
        ({required int offset, required int limit, required String? path}) =>
            widget.notifier.loadStoryState(
              includeUnset: true,
              offset: offset,
              limit: limit,
              path: path,
            );
    final first = await loader(offset: 0, limit: pageSize, path: pinnedPath);
    if (first.error != null || first.values.length >= first.total) return first;

    final values = [...first.values];
    var expectedTotal = first.total;
    String? error;
    while (values.length < expectedTotal) {
      final next = await loader(
        offset: values.length,
        limit: pageSize,
        path: pinnedPath,
      );
      if (next.error != null) {
        error = next.error;
        break;
      }
      if (next.values.isEmpty) {
        error =
            'Story-state pagination ended before all $expectedTotal values were returned.';
        break;
      }
      values.addAll(next.values);
      expectedTotal = next.total;
    }

    return StoryStatePage(
      values: List.unmodifiable(values),
      kindCounts: first.kindCounts,
      total: expectedTotal,
      storedTotal: first.storedTotal,
      catalogTotal: first.catalogTotal,
      unsetTotal: first.unsetTotal,
      unknownStoredTotal: first.unknownStoredTotal,
      offset: 0,
      limit: values.length,
      currentGameTimeSeconds: first.currentGameTimeSeconds,
      writable: first.writable,
      error: error,
    );
  }

  Future<List<NpcGlossaryCatalogEntry>> _loadNpcCatalogSafely() async {
    try {
      return await (widget.npcCatalogLoader ?? loadGlossaryNpcCatalog)();
    } catch (_) {
      return const [];
    }
  }

  Future<GlossarySegmentTextCatalog> _loadSegmentTextCatalogSafely() async {
    try {
      return await (widget.segmentTextCatalogLoader ??
          loadGlossarySegmentTextCatalog)();
    } catch (_) {
      return const {};
    }
  }

  List<StoryStateValue> _visibleValues(
    Map<String, Map<String, String>> locCatalog,
    GameLang lang,
  ) {
    return _page.values
        .where((value) {
          final presenceMatches = _matchesPresence(value, _presence);
          final typeMatches = switch (_filter) {
            _StoryFilter.all => true,
            _StoryFilter.integer =>
              value.semanticType == StorySemanticType.integer,
            _StoryFilter.timeMarker =>
              value.semanticType == StorySemanticType.timeMarker,
            _StoryFilter.chapter =>
              value.semanticType == StorySemanticType.chapter,
            _StoryFilter.unknown =>
              value.semanticType == StorySemanticType.unknown,
          };
          if (!presenceMatches || !typeMatches || _query.isEmpty) {
            return presenceMatches && typeMatches;
          }
          final link = _glossaryLink(value);
          final text = [
            value.id,
            if (value.value != null) value.value.toString(),
            value.declaredType,
            humanizeStoryId(value.id),
            if (link != null) ...[
              link.npcName(locCatalog, lang),
              link.segmentLabel,
              ...link.localizedParagraphs(locCatalog, lang),
            ],
          ].join(' ').toLowerCase();
          return text.contains(_query);
        })
        .toList(growable: false);
  }

  StoryGlossaryLink? _glossaryLink(StoryStateValue value) =>
      findStoryGlossaryLink(value.id, _npcCatalog, _segmentTextCatalog);

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context);
    final locCatalog = ref.watch(locCatalogProvider).value ?? const {};
    final lang = ref.watch(currentGameLangProvider);
    final showObjectIds = ref.watch(showObjectIdsProvider);
    final values = _visibleValues(locCatalog, lang);
    final pendingById = {
      for (final edit in widget.notifier.allStoryStateEdits())
        edit.normalizedId: edit,
    };
    // A same-save refresh intentionally keeps the old rows visible while the
    // new inspection loads. Do not let those rows create a draft with the old
    // compare-and-swap snapshot; editing resumes only after the fresh page
    // arrives.
    final effectiveEditable = widget.editable && _page.writable && !_loading;
    final showCodecReadOnly = !widget.editable && _page.error == null;
    final showStructureReadOnly =
        widget.editable && !_loading && !_page.writable && _page.error == null;
    final presenceValues = _page.values
        .where((value) => _matchesPresence(value, _presence))
        .toList(growable: false);
    final scheme = widget.theme.colorScheme;
    return Card(
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            if (_showInfo)
              DecoratedBox(
                key: const Key('story-state-info-box'),
                decoration: BoxDecoration(
                  color: scheme.secondaryContainer.withValues(alpha: 0.45),
                  borderRadius: BorderRadius.circular(10),
                ),
                child: Padding(
                  padding: const EdgeInsets.all(12),
                  child: Row(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      Icon(Icons.edit_note_outlined, color: scheme.secondary),
                      const SizedBox(width: 10),
                      Expanded(
                        child: Column(
                          crossAxisAlignment: CrossAxisAlignment.start,
                          children: [
                            Text(
                              l10n.storyStateDescription,
                              style: widget.theme.textTheme.bodyMedium,
                            ),
                            if (effectiveEditable) ...[
                              const SizedBox(height: 4),
                              Tooltip(
                                message: l10n.storyStateEditingGuidance,
                                child: Text(
                                  l10n.storyStateEditingGuidance,
                                  maxLines: 2,
                                  overflow: TextOverflow.ellipsis,
                                  style: widget.theme.textTheme.bodySmall
                                      ?.copyWith(
                                        color: scheme.onSurfaceVariant,
                                      ),
                                ),
                              ),
                            ],
                          ],
                        ),
                      ),
                    ],
                  ),
                ),
              ),
            if (_loading) ...[
              const SizedBox(height: 6),
              const LinearProgressIndicator(),
            ],
            if (showCodecReadOnly) ...[
              const SizedBox(height: 6),
              Text(
                l10n.codecReadOnly,
                style: widget.theme.textTheme.bodySmall?.copyWith(
                  color: scheme.onSurfaceVariant,
                ),
              ),
            ],
            if (showStructureReadOnly) ...[
              const SizedBox(height: 6),
              Text(
                l10n.storyStateStructureReadOnly,
                style: widget.theme.textTheme.bodySmall?.copyWith(
                  color: scheme.onSurfaceVariant,
                ),
              ),
            ],
            const SizedBox(height: 12),
            Row(
              children: [
                Expanded(
                  child: TextField(
                    key: const Key('story-state-search'),
                    controller: _search,
                    decoration: InputDecoration(
                      labelText: l10n.storyStateSearch,
                      prefixIcon: const Icon(Icons.search),
                      suffixIcon: _query.isEmpty
                          ? null
                          : IconButton(
                              icon: const Icon(Icons.clear),
                              onPressed: () {
                                _search.clear();
                                setState(() => _query = '');
                              },
                            ),
                    ),
                    onChanged: (value) =>
                        setState(() => _query = value.trim().toLowerCase()),
                  ),
                ),
                const SizedBox(width: 8),
                IconButton.outlined(
                  key: const Key('story-state-info'),
                  tooltip: l10n.details,
                  isSelected: _showInfo,
                  selectedIcon: const Icon(Icons.info),
                  onPressed: () => setState(() => _showInfo = !_showInfo),
                  icon: const Icon(Icons.info_outline),
                ),
                if (pendingById.isNotEmpty) ...[
                  const SizedBox(width: 4),
                  Tooltip(
                    message: l10n.storyStateResetChanges,
                    child: IconButton.outlined(
                      key: const Key('story-state-reset'),
                      onPressed: widget.editable
                          ? () {
                              widget.notifier.clearAllStoryStateEdits();
                              setState(() {});
                            }
                          : null,
                      icon: const Icon(Icons.undo),
                    ),
                  ),
                ],
              ],
            ),
            const SizedBox(height: 8),
            Wrap(
              spacing: 7,
              runSpacing: 6,
              children: [
                for (final presence in _StoryPresence.values)
                  ChoiceChip(
                    label: Text(_presenceLabel(l10n, presence, _page)),
                    selected: _presence == presence,
                    onSelected: (_) => setState(() => _presence = presence),
                  ),
              ],
            ),
            const SizedBox(height: 6),
            Wrap(
              spacing: 7,
              runSpacing: 6,
              crossAxisAlignment: WrapCrossAlignment.center,
              children: [
                for (final filter in _StoryFilter.values)
                  ChoiceChip(
                    label: Text(_filterLabel(l10n, filter, presenceValues)),
                    selected: _filter == filter,
                    onSelected: (_) => setState(() => _filter = filter),
                  ),
                const SizedBox(width: 4),
                Text(
                  l10n.storyStateValuesCount(values.length, _page.total),
                  style: widget.theme.textTheme.bodySmall?.copyWith(
                    color: scheme.onSurfaceVariant,
                  ),
                ),
              ],
            ),
            if (_page.error != null) ...[
              const SizedBox(height: 8),
              Text(_page.error!, style: TextStyle(color: scheme.error)),
            ],
            const SizedBox(height: 6),
            Expanded(
              child: _loading && _page.values.isEmpty
                  ? const Center(child: CircularProgressIndicator())
                  : values.isEmpty
                  ? Center(child: Text(l10n.noEntriesMatch))
                  : ListView.separated(
                      itemCount: values.length,
                      separatorBuilder: (_, _) => const Divider(height: 1),
                      itemBuilder: (context, index) => _StoryValueTile(
                        value: values[index],
                        link: _glossaryLink(values[index]),
                        currentGameTimeSeconds: _page.currentGameTimeSeconds,
                        pending:
                            pendingById[normalizeStoryStateId(
                              values[index].id,
                            )],
                        notifier: widget.notifier,
                        editable: effectiveEditable,
                        onChanged: () => setState(() {}),
                        locCatalog: locCatalog,
                        lang: lang,
                        showObjectIds: showObjectIds,
                        theme: widget.theme,
                      ),
                    ),
            ),
          ],
        ),
      ),
    );
  }
}

class _StoryValueTile extends StatelessWidget {
  const _StoryValueTile({
    required this.value,
    required this.link,
    required this.currentGameTimeSeconds,
    required this.pending,
    required this.notifier,
    required this.editable,
    required this.onChanged,
    required this.locCatalog,
    required this.lang,
    required this.showObjectIds,
    required this.theme,
  });

  final StoryStateValue value;
  final StoryGlossaryLink? link;
  final double? currentGameTimeSeconds;
  final StoryStateEdit? pending;
  final EditorNotifier notifier;
  final bool editable;
  final VoidCallback onChanged;
  final Map<String, Map<String, String>> locCatalog;
  final GameLang lang;
  final bool showObjectIds;
  final ThemeData theme;

  Future<void> _editValue(BuildContext context) async {
    final target = await showDialog<int>(
      context: context,
      builder: (context) => _StoryValueEditDialog(
        value: value,
        semantics: storyIntegerSemantics(value.id),
        initialValue: pending?.present == true
            ? pending!.rawValue
            : value.value,
        currentGameTimeSeconds: currentGameTimeSeconds,
      ),
    );
    if (target == null) return;
    notifier.setStoryStateEdit(
      StoryStateEdit.fromValue(value, present: true, rawValue: target),
    );
    onChanged();
  }

  void _removeValue() {
    notifier.setStoryStateEdit(StoryStateEdit.fromValue(value, present: false));
    onChanged();
  }

  void _undo() {
    notifier.clearStoryStateEdit(value.id);
    onChanged();
  }

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context);
    final paragraphs = link?.localizedParagraphs(locCatalog, lang) ?? const [];
    final title = link == null
        ? humanizeStoryId(value.id)
        : '${link!.npcName(locCatalog, lang)} — '
              '${l10n.glossaryCatalogSegmentLabel(link!.segmentId, humanizeStoryId(link!.segmentLabel))}';
    final rawValue = value.value;
    final semantics = storyIntegerSemantics(value.id);
    final timeParts =
        value.stored &&
            value.semanticType == StorySemanticType.timeMarker &&
            rawValue != null &&
            rawValue >= 0
        ? GameTimeParts.fromTotalSeconds(rawValue.toDouble())
        : null;
    return ExpansionTile(
      key: ValueKey('story-value-${value.id}'),
      tilePadding: const EdgeInsets.symmetric(horizontal: 8, vertical: 2),
      childrenPadding: const EdgeInsets.fromLTRB(16, 0, 16, 12),
      leading: Icon(_semanticIcon(value.semanticType), size: 22),
      title: Text(title),
      subtitle: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          const SizedBox(height: 4),
          if (showObjectIds) ...[
            SelectableText(
              value.id,
              style: theme.textTheme.bodySmall?.copyWith(
                color: theme.colorScheme.onSurfaceVariant,
                fontFamily: uiAwareMonospaceFontFamily(context),
              ),
            ),
            const SizedBox(height: 3),
          ],
          Wrap(
            spacing: 6,
            runSpacing: 4,
            crossAxisAlignment: WrapCrossAlignment.center,
            children: [
              _CompactBadge(
                label: value.stored
                    ? l10n.storyStateStored
                    : l10n.storyStateUnset,
              ),
              _CompactBadge(label: _semanticLabel(l10n, value.semanticType)),
              if (semantics != null)
                _CompactBadge(
                  label: l10n.storyStateIntegerKind(semantics.kind.name),
                ),
              _CompactBadge(label: value.declaredType),
              if (pending != null) _CompactBadge(label: l10n.storyStatePending),
              if (timeParts != null)
                Text(
                  l10n.memoryEventGameTime(timeParts.day, _clock(timeParts)),
                  style: theme.textTheme.bodyMedium,
                )
              else if (value.stored && rawValue != null)
                Text('${l10n.storyStateRawValue}: $rawValue'),
              if (pending != null)
                Text(
                  pending!.present
                      ? l10n.storyStatePendingValue(
                          _storyValueLabel(value, pending!.rawValue!),
                        )
                      : l10n.storyStatePendingRemoval,
                  style: theme.textTheme.bodyMedium?.copyWith(
                    color: theme.colorScheme.primary,
                    fontWeight: FontWeight.w600,
                  ),
                ),
            ],
          ),
        ],
      ),
      children: [
        Align(
          alignment: Alignment.centerLeft,
          child: SelectionArea(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                if (value.stored && rawValue != null)
                  Text(
                    '${l10n.storyStateRawValue}: $rawValue',
                    style: theme.textTheme.bodyMedium,
                  )
                else
                  Text(
                    l10n.storyStateUnsetDetail,
                    style: theme.textTheme.bodyMedium,
                  ),
                if (!value.catalogKnown) ...[
                  const SizedBox(height: 6),
                  Text(
                    l10n.storyStateUnknownDetail,
                    style: theme.textTheme.bodySmall?.copyWith(
                      color: theme.colorScheme.error,
                    ),
                  ),
                ],
                if (semantics?.kind ==
                    StoryIntegerKind.dormantOrLegacyInteger) ...[
                  const SizedBox(height: 6),
                  Text(
                    l10n.storyStateDormantWarning,
                    style: theme.textTheme.bodySmall?.copyWith(
                      color: theme.colorScheme.tertiary,
                    ),
                  ),
                ],
                if (semantics?.kind ==
                    StoryIntegerKind.readOnlyInSourceInteger) ...[
                  const SizedBox(height: 6),
                  Text(
                    l10n.storyStateReadOnlySourceWarning,
                    style: theme.textTheme.bodySmall?.copyWith(
                      color: theme.colorScheme.tertiary,
                    ),
                  ),
                ],
                if (value.semanticType == StorySemanticType.chapter) ...[
                  const SizedBox(height: 6),
                  Text(
                    l10n.storyStateChapterWarning,
                    style: theme.textTheme.bodySmall?.copyWith(
                      color: theme.colorScheme.error,
                    ),
                  ),
                ],
                if (value.semanticType == StorySemanticType.unknown) ...[
                  const SizedBox(height: 6),
                  Text(
                    l10n.storyStateUnknownEditWarning,
                    style: theme.textTheme.bodySmall?.copyWith(
                      color: theme.colorScheme.error,
                    ),
                  ),
                ],
                if (value.semanticType == StorySemanticType.integer) ...[
                  const SizedBox(height: 6),
                  Text(
                    l10n.storyStateZeroVsUnset,
                    style: theme.textTheme.bodySmall?.copyWith(
                      color: theme.colorScheme.onSurfaceVariant,
                    ),
                  ),
                ],
                if (timeParts != null && currentGameTimeSeconds != null) ...[
                  const SizedBox(height: 3),
                  Text(
                    _timeDistanceLabel(
                      l10n,
                      rawValue!,
                      currentGameTimeSeconds!,
                    ),
                    style: theme.textTheme.bodySmall,
                  ),
                ],
                if (paragraphs.isNotEmpty) ...[
                  const SizedBox(height: 10),
                  Text(
                    l10n.storyStateRelatedGlossary,
                    style: theme.textTheme.labelLarge,
                  ),
                  const SizedBox(height: 3),
                  for (final paragraph in paragraphs)
                    Padding(
                      padding: const EdgeInsets.only(bottom: 4),
                      child: Text(paragraph),
                    ),
                ],
                if (value.path.isNotEmpty) ...[
                  const SizedBox(height: 10),
                  Text(
                    l10n.storyStateTechnicalPath,
                    style: theme.textTheme.labelLarge,
                  ),
                  const SizedBox(height: 3),
                  Text(
                    value.path.join(' › '),
                    style: theme.textTheme.bodySmall?.copyWith(
                      color: theme.colorScheme.onSurfaceVariant,
                      fontFamily: uiAwareMonospaceFontFamily(context),
                    ),
                  ),
                ],
                if (editable) ...[
                  const SizedBox(height: 12),
                  Wrap(
                    spacing: 8,
                    runSpacing: 8,
                    children: [
                      FilledButton.tonalIcon(
                        key: ValueKey('story-edit-${value.id}'),
                        onPressed: () => _editValue(context),
                        icon: const Icon(Icons.edit_outlined),
                        label: Text(
                          value.stored || pending?.present == true
                              ? l10n.storyStateEditValue
                              : l10n.storyStateSetValue,
                        ),
                      ),
                      if (value.stored)
                        OutlinedButton.icon(
                          key: ValueKey('story-remove-${value.id}'),
                          onPressed: pending?.present == false
                              ? null
                              : _removeValue,
                          icon: const Icon(Icons.delete_outline),
                          label: Text(l10n.storyStateRemoveValue),
                        ),
                      if (pending != null)
                        TextButton.icon(
                          key: ValueKey('story-undo-${value.id}'),
                          onPressed: _undo,
                          icon: const Icon(Icons.undo),
                          label: Text(l10n.storyStateUndoChange),
                        ),
                    ],
                  ),
                ],
              ],
            ),
          ),
        ),
      ],
    );
  }
}

String _storyValueLabel(StoryStateValue value, int rawValue) {
  if (value.semanticType != StorySemanticType.timeMarker || rawValue < 0) {
    return rawValue.toString();
  }
  final parts = GameTimeParts.fromTotalSeconds(rawValue.toDouble());
  return '${parts.day} / ${_clock(parts)} ($rawValue)';
}

class _StoryValueEditDialog extends StatefulWidget {
  const _StoryValueEditDialog({
    required this.value,
    required this.semantics,
    required this.initialValue,
    required this.currentGameTimeSeconds,
  });

  final StoryStateValue value;
  final StoryIntegerSemantics? semantics;
  final int? initialValue;
  final double? currentGameTimeSeconds;

  @override
  State<_StoryValueEditDialog> createState() => _StoryValueEditDialogState();
}

class _StoryValueEditDialogState extends State<_StoryValueEditDialog> {
  static final _i32Min = BigInt.from(-2147483648);
  static final _i32Max = BigInt.from(2147483647);

  final _raw = TextEditingController();
  final _day = TextEditingController();
  final _hour = TextEditingController();
  final _minute = TextEditingController();
  final _second = TextEditingController();
  late bool _structuredTime;
  String? _error;

  bool get _isTime => widget.value.semanticType == StorySemanticType.timeMarker;

  @override
  void initState() {
    super.initState();
    _structuredTime = _isTime && (widget.initialValue ?? 0) >= 0;
    final initial = widget.initialValue;
    if (initial != null) {
      _raw.text = initial.toString();
      if (initial >= 0) _seedStructuredTime(initial);
    }
  }

  @override
  void dispose() {
    _raw.dispose();
    _day.dispose();
    _hour.dispose();
    _minute.dispose();
    _second.dispose();
    super.dispose();
  }

  void _seedStructuredTime(int total) {
    final parts = GameTimeParts.fromTotalSeconds(total.toDouble());
    _day.text = parts.day.toString();
    _hour.text = parts.hour.toString();
    _minute.text = parts.minute.toString();
    _second.text = parts.second.toString();
  }

  int? _parseI32(String text) {
    final parsed = BigInt.tryParse(text.trim());
    if (parsed == null || parsed < _i32Min || parsed > _i32Max) return null;
    return parsed.toInt();
  }

  int? _structuredTotal() {
    final day = int.tryParse(_day.text.trim());
    final hour = int.tryParse(_hour.text.trim());
    final minute = int.tryParse(_minute.text.trim());
    final second = int.tryParse(_second.text.trim());
    if (day == null ||
        hour == null ||
        minute == null ||
        second == null ||
        day < 0 ||
        hour < 0 ||
        hour > 23 ||
        minute < 0 ||
        minute > 59 ||
        second < 0 ||
        second > 59) {
      return null;
    }
    final total =
        BigInt.from(day) * BigInt.from(secondsPerDay) +
        BigInt.from(hour) * BigInt.from(secondsPerHour) +
        BigInt.from(minute) * BigInt.from(secondsPerMinute) +
        BigInt.from(second);
    if (total > _i32Max) return null;
    return total.toInt();
  }

  void _setRaw(int value) {
    setState(() {
      _raw.text = value.toString();
      if (_isTime && value >= 0) _seedStructuredTime(value);
      _error = null;
    });
  }

  void _switchTimeMode(bool structured) {
    setState(() {
      if (structured) {
        final raw = _parseI32(_raw.text);
        if (raw != null && raw >= 0) _seedStructuredTime(raw);
      } else {
        final total = _structuredTotal();
        if (total != null) _raw.text = total.toString();
      }
      _structuredTime = structured;
      _error = null;
    });
  }

  void _useCurrentTime() {
    final current = widget.currentGameTimeSeconds?.floor();
    if (current == null || current < 0 || current > 2147483647) return;
    _setRaw(current);
    setState(() => _structuredTime = true);
  }

  void _submit() {
    final l10n = AppLocalizations.of(context);
    final value = _isTime && _structuredTime
        ? _structuredTotal()
        : _parseI32(_raw.text);
    if (value == null) {
      setState(
        () => _error = _isTime && _structuredTime
            ? l10n.gameTimeInvalid
            : l10n.storyStateInvalidInt32,
      );
      return;
    }
    Navigator.of(context).pop(value);
  }

  List<int> get _suggestedValues {
    if (widget.value.semanticType == StorySemanticType.chapter) {
      return const [1, 2, 3, 4, 5, 6];
    }
    return widget.semantics?.knownValues ?? const [];
  }

  String _suggestionLabel(AppLocalizations l10n, int value) {
    if (widget.semantics?.kind == StoryIntegerKind.binaryFlag) {
      return '$value — ${value == 0 ? l10n.no : l10n.yes}';
    }
    return value.toString();
  }

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context);
    final theme = Theme.of(context);
    final suggestions = _suggestedValues;
    final selectedRaw = _parseI32(_raw.text);

    Widget numberField(
      String label,
      TextEditingController controller, {
      double width = 105,
    }) {
      return SizedBox(
        width: width,
        child: TextField(
          controller: controller,
          keyboardType: const TextInputType.numberWithOptions(signed: true),
          onChanged: (_) => setState(() => _error = null),
          decoration: InputDecoration(labelText: label),
        ),
      );
    }

    return AlertDialog(
      title: Text(l10n.storyStateDialogTitle(widget.value.id)),
      content: SizedBox(
        width: 620,
        child: SingleChildScrollView(
          child: Column(
            mainAxisSize: MainAxisSize.min,
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: [
              if (_isTime) ...[
                Wrap(
                  spacing: 8,
                  children: [
                    ChoiceChip(
                      key: const Key('story-time-structured'),
                      label: Text(l10n.storyStateStructuredTime),
                      selected: _structuredTime,
                      onSelected: (_) => _switchTimeMode(true),
                    ),
                    ChoiceChip(
                      key: const Key('story-time-raw'),
                      label: Text(l10n.storyStateRawMode),
                      selected: !_structuredTime,
                      onSelected: (_) => _switchTimeMode(false),
                    ),
                  ],
                ),
                const SizedBox(height: 12),
                if (_structuredTime)
                  Wrap(
                    spacing: 8,
                    runSpacing: 8,
                    children: [
                      numberField(l10n.gameTimeDay, _day),
                      numberField(l10n.gameTimeHours, _hour),
                      numberField(l10n.gameTimeMinutes, _minute),
                      numberField(l10n.gameTimeSeconds, _second),
                    ],
                  )
                else
                  TextField(
                    key: const Key('story-raw-value'),
                    controller: _raw,
                    autofocus: true,
                    keyboardType: const TextInputType.numberWithOptions(
                      signed: true,
                    ),
                    onChanged: (_) => setState(() => _error = null),
                    decoration: InputDecoration(
                      labelText: l10n.storyStateRawInput,
                    ),
                  ),
                if (widget.currentGameTimeSeconds != null) ...[
                  const SizedBox(height: 8),
                  Align(
                    alignment: Alignment.centerLeft,
                    child: TextButton.icon(
                      key: const Key('story-use-current-time'),
                      onPressed: _useCurrentTime,
                      icon: const Icon(Icons.update),
                      label: Text(l10n.storyStateUseCurrentTime),
                    ),
                  ),
                ],
              ] else ...[
                if (suggestions.isNotEmpty) ...[
                  Text(
                    l10n.storyStateSuggestedValues(suggestions.join(', ')),
                    style: theme.textTheme.bodySmall,
                  ),
                  const SizedBox(height: 6),
                  Wrap(
                    spacing: 7,
                    runSpacing: 6,
                    children: [
                      for (final suggestion in suggestions)
                        ChoiceChip(
                          key: ValueKey(
                            'story-suggestion-${widget.value.id}-$suggestion',
                          ),
                          label: Text(_suggestionLabel(l10n, suggestion)),
                          selected: selectedRaw == suggestion,
                          onSelected: (_) => _setRaw(suggestion),
                        ),
                    ],
                  ),
                  const SizedBox(height: 8),
                  Text(
                    l10n.storyStateSuggestionsNotLimits,
                    style: theme.textTheme.bodySmall?.copyWith(
                      color: theme.colorScheme.onSurfaceVariant,
                    ),
                  ),
                  const SizedBox(height: 12),
                ],
                TextField(
                  key: const Key('story-raw-value'),
                  controller: _raw,
                  autofocus: true,
                  keyboardType: const TextInputType.numberWithOptions(
                    signed: true,
                  ),
                  onChanged: (_) => setState(() => _error = null),
                  decoration: InputDecoration(
                    labelText: l10n.storyStateRawInput,
                  ),
                ),
              ],
              if (_error != null) ...[
                const SizedBox(height: 8),
                Semantics(
                  liveRegion: true,
                  child: Text(
                    _error!,
                    style: TextStyle(color: theme.colorScheme.error),
                  ),
                ),
              ],
            ],
          ),
        ),
      ),
      actions: [
        TextButton(
          onPressed: () => Navigator.of(context).pop(),
          child: Text(l10n.cancel),
        ),
        FilledButton(
          key: const Key('story-queue-change'),
          onPressed: _submit,
          child: Text(l10n.storyStateQueueChange),
        ),
      ],
    );
  }
}

class _CompactBadge extends StatelessWidget {
  const _CompactBadge({required this.label});

  final String label;

  @override
  Widget build(BuildContext context) {
    final scheme = Theme.of(context).colorScheme;
    return DecoratedBox(
      decoration: BoxDecoration(
        color: scheme.surfaceContainerHighest,
        borderRadius: BorderRadius.circular(999),
      ),
      child: Padding(
        padding: const EdgeInsets.symmetric(horizontal: 7, vertical: 2),
        child: Text(label, style: Theme.of(context).textTheme.labelSmall),
      ),
    );
  }
}

String _filterLabel(
  AppLocalizations l10n,
  _StoryFilter filter,
  List<StoryStateValue> values,
) {
  final (label, count) = switch (filter) {
    _StoryFilter.all => (l10n.categoryAll, values.length),
    _StoryFilter.integer => (
      l10n.storyStateInteger,
      values
          .where((value) => value.semanticType == StorySemanticType.integer)
          .length,
    ),
    _StoryFilter.timeMarker => (
      l10n.storyStateTimeMarker,
      values
          .where((value) => value.semanticType == StorySemanticType.timeMarker)
          .length,
    ),
    _StoryFilter.chapter => (
      l10n.storyStateChapter,
      values
          .where((value) => value.semanticType == StorySemanticType.chapter)
          .length,
    ),
    _StoryFilter.unknown => (
      l10n.storyStateUnknown,
      values
          .where((value) => value.semanticType == StorySemanticType.unknown)
          .length,
    ),
  };
  return '$label ($count)';
}

bool _matchesPresence(StoryStateValue value, _StoryPresence presence) =>
    switch (presence) {
      _StoryPresence.stored => value.stored,
      _StoryPresence.unset => !value.stored,
      _StoryPresence.all => true,
    };

String _presenceLabel(
  AppLocalizations l10n,
  _StoryPresence presence,
  StoryStatePage page,
) {
  final (label, count) = switch (presence) {
    _StoryPresence.stored => (l10n.storyStateStored, page.storedTotal),
    _StoryPresence.unset => (l10n.storyStateUnset, page.unsetTotal),
    _StoryPresence.all => (l10n.categoryAll, page.total),
  };
  return '$label ($count)';
}

String _semanticLabel(AppLocalizations l10n, StorySemanticType type) =>
    switch (type) {
      StorySemanticType.integer => l10n.storyStateInteger,
      StorySemanticType.timeMarker => l10n.storyStateTimeMarker,
      StorySemanticType.chapter => l10n.storyStateChapter,
      StorySemanticType.unknown => l10n.storyStateUnknown,
    };

IconData _semanticIcon(StorySemanticType type) => switch (type) {
  StorySemanticType.integer => Icons.numbers_outlined,
  StorySemanticType.timeMarker => Icons.schedule_outlined,
  StorySemanticType.chapter => Icons.auto_stories_outlined,
  StorySemanticType.unknown => Icons.help_outline,
};

String _clock(GameTimeParts parts) =>
    '${parts.hour.toString().padLeft(2, '0')}:'
    '${parts.minute.toString().padLeft(2, '0')}:'
    '${parts.second.toString().padLeft(2, '0')}';

String _timeDistanceLabel(
  AppLocalizations l10n,
  int timestamp,
  double currentTime,
) {
  final signedSeconds = currentTime.floor() - timestamp;
  final duration = _durationLabel(l10n, signedSeconds.abs());
  return signedSeconds >= 0
      ? l10n.storyStateElapsed(duration)
      : l10n.storyStateAhead(duration);
}

String _durationLabel(AppLocalizations l10n, int seconds) {
  final days = seconds ~/ secondsPerDay;
  final hours = (seconds % secondsPerDay) ~/ secondsPerHour;
  final minutes = (seconds % secondsPerHour) ~/ secondsPerMinute;
  final remainder = seconds % secondsPerMinute;
  final clock =
      '${hours.toString().padLeft(2, '0')}:'
      '${minutes.toString().padLeft(2, '0')}:'
      '${remainder.toString().padLeft(2, '0')}';
  return days == 0 ? clock : l10n.storyStateDurationDays(days, clock);
}
