import 'dart:async';

import 'package:flutter/material.dart';
import 'package:goresave/features/editor/domain/actor.dart';
import 'package:goresave/features/editor/domain/character_index.dart';
import 'package:goresave/l10n/app_localizations.dart';
import 'package:goresave/loc/game_lang.dart';
import 'package:goresave/loc/loc_catalog_provider.dart';

/// Localized NPC display name for a GlobalId. The loc catalog is keyed by the
/// character key — the GlobalId prefix before the first `-`, lowercased (the
/// resolver lowercases). When the catalog has no entry (~42% of NPCs carry
/// generic class-name ids), fall back to a PRETTIFIED key rather than the raw
/// id: classification prefixes are stripped and the remainder is humanized.
///
/// Exported so the shared actor detail header resolves an NPC name the SAME way
/// the list tiles do (loc catalog + prettify fallback).
String localizedNpcName(
  Map<String, Map<String, String>> catalog,
  GameLang lang,
  String id,
) {
  final charKey = id.split('-').first;
  final localized = localizedGameName(catalog, lang, charKey);
  if (localized != null && localized.trim().isNotEmpty) return localized;
  return _prettifyNpcKey(charKey);
}

/// Leading classification prefixes that carry no display value (NPC archetype /
/// faction tags). Stripped before humanizing a fallback name.
const _npcKeyPrefixes = <String>[
  'OC_',
  'OM_',
  'NPC_',
  'Creature_',
  'AM_',
  'PC_',
  'BL_',
  'VLK_',
  'KDF_',
  'STT_',
  'SLD_',
  'GRD_',
  'MIL_',
  'EBR_',
  'BAU_',
  'TPL_',
  'NOV_',
  'DJG_',
  'SFB_',
  'GUR_',
  'OUT_',
  'SUM_',
];

/// Turn a generic character key (e.g. `OC_VLK_Guard_01`, `Creature_Meatbug`)
/// into a readable label: strip leading classification prefixes + trailing
/// numeric ids, then humanize (`_` → space, Title Case). Robust to keys with no
/// recognizable prefix (returns a humanized form of the whole key).
String _prettifyNpcKey(String key) {
  var rest = key;
  // Strip recognized leading prefixes, repeatedly (ids often stack two/three).
  var stripped = true;
  while (stripped) {
    stripped = false;
    for (final prefix in _npcKeyPrefixes) {
      if (rest.length > prefix.length &&
          rest.toUpperCase().startsWith(prefix.toUpperCase())) {
        rest = rest.substring(prefix.length);
        stripped = true;
        break;
      }
    }
  }
  // Drop a trailing numeric id segment (e.g. `_01`, `_1234`).
  rest = rest.replaceFirst(RegExp(r'[_-]?\d+$'), '');
  if (rest.isEmpty) rest = key;
  // Humanize: split on separators + camelCase boundaries, Title Case words.
  final words = rest
      .replaceAllMapped(RegExp('([a-z])([A-Z])'), (m) => '${m[1]} ${m[2]}')
      .split(RegExp(r'[_\s-]+'))
      .where((w) => w.isNotEmpty)
      .map((w) => w[0].toUpperCase() + w.substring(1).toLowerCase())
      .toList();
  final pretty = words.join(' ').trim();
  return pretty.isEmpty ? key : pretty;
}

/// A single [CharacterRow] paired with its precomputed, lowercased search string
/// and resolved display name. Names resolve ONCE when the list loads so each
/// keystroke is a cheap substring scan over the cached strings rather than
/// re-resolving every name.
class _SearchableRow {
  const _SearchableRow(this.row, this.name, this.search);

  final CharacterRow row;
  final String name;
  final String search;
}

/// Entity-first master list of all characters in the save: the Player (pinned on
/// top), every spawned actor (searchable + paginated), and a trailing group of
/// knowledge-only "orphans" (characters with no GlobalId). Selecting an entry
/// calls [onSelect]; the parent owns which actor is [selected] and re-passes it.
///
/// The full list is fetched ONCE (one save decompress) and cached; search and
/// pagination then run entirely CLIENT-SIDE. This makes search instant and lets
/// it match the RESOLVED display name (loc catalog / prettify fallback) in
/// addition to the raw id. Cloned from the retired `ActorSelector`'s proven
/// structure for a different row type ([CharacterRow] instead of `NpcActor`).
class CharacterMasterList extends StatefulWidget {
  const CharacterMasterList({
    super.key,
    required this.selected,
    required this.onSelect,
    required this.load,
    required this.reloadKey,
    required this.locCatalog,
    required this.lang,
  });

  /// The currently selected actor (player, NPC, or orphan). Owned by the parent
  /// so the highlight stays in sync with the shared editor state.
  final Actor selected;

  /// Called when the user taps the Player row, an NPC row, or an orphan row.
  final void Function(Actor) onSelect;

  /// Loads the full unified character index (called once). Returns EVERY
  /// character in a single unpaginated response (e.g.
  /// `EditorNotifier.loadAllCharacters`), which this list then filters +
  /// paginates client-side. Injected so tests can supply a fake.
  final Future<CharacterIndexPage> Function() load;

  /// Identifies the inspected save (e.g. the inspection identity or save path).
  /// When it changes — switching save or refreshing — the cached list is stale
  /// (it belongs to the previous file), so the list resets and re-fetches.
  /// [load] is a stable method reference, so it alone can't signal a save
  /// change; this key does.
  final Object reloadKey;

  /// Loaded localization catalog (`id -> {set -> text}`) used to resolve NPC
  /// display names via [localizedNpcName].
  final Map<String, Map<String, String>> locCatalog;

  /// The current game language, driving which loc set the name resolves from.
  final GameLang lang;

  @override
  State<CharacterMasterList> createState() => _CharacterMasterListState();
}

class _CharacterMasterListState extends State<CharacterMasterList> {
  static const _pageSize = 100;

  final TextEditingController _search = TextEditingController();
  // Debounce live search. No core call is made per keystroke (the full list is
  // cached) — this only throttles the cheap client-side re-filter so a fast
  // typist doesn't rebuild the list on every character.
  Timer? _debounce;

  // The full, cached rows with precomputed search strings + display names,
  // split into spawned actors and knowledge-only orphans. Fetched once.
  List<_SearchableRow> _actors = const [];
  List<_SearchableRow> _orphans = const [];
  String? _error;
  String _query = '';
  // Client-side page cursor over the FILTERED actor list (an item offset).
  int _offset = 0;
  bool _loading = false;
  // Monotonic epoch so a stale load never overwrites a newer one.
  int _epoch = 0;

  @override
  void initState() {
    super.initState();
    _loadAll();
  }

  @override
  void didUpdateWidget(covariant CharacterMasterList oldWidget) {
    super.didUpdateWidget(oldWidget);
    // Re-resolve cached names if the catalog or language changes (the loaded
    // rows are unchanged, so no refetch — just recompute search strings).
    if (oldWidget.locCatalog != widget.locCatalog ||
        oldWidget.lang != widget.lang) {
      _actors = _decorate(_actors.map((e) => e.row).toList(growable: false));
      _orphans = _decorate(_orphans.map((e) => e.row).toList(growable: false));
    }
    // A new loader OR a new reloadKey (a different save/inspection) means the
    // cached list is stale: reset search/cursor/lists and re-fetch against the
    // new save.
    if (oldWidget.load != widget.load ||
        oldWidget.reloadKey != widget.reloadKey) {
      _search.clear();
      _query = '';
      _offset = 0;
      _actors = const [];
      _orphans = const [];
      _error = null;
      _loadAll();
    }
  }

  @override
  void dispose() {
    _debounce?.cancel();
    _search.dispose();
    super.dispose();
  }

  /// Pair each row with its resolved display name + a lowercased search string,
  /// computed once so per-keystroke filtering is a cheap scan. The name/search
  /// key is the GlobalId for actors and the uniqueName for orphans (which have
  /// no GlobalId).
  List<_SearchableRow> _decorate(List<CharacterRow> rows) {
    return [
      for (final row in rows)
        () {
          final key = row.globalId ?? row.uniqueName;
          final name = localizedNpcName(widget.locCatalog, widget.lang, key);
          return _SearchableRow(row, name, '$key\n$name'.toLowerCase());
        }(),
    ];
  }

  /// True for the save's own "Hero" ACTOR row (the player's avatar). The core
  /// emits it like any other actor (it IS real data), but the pinned Player row
  /// already represents it — so the NPC section excludes it, or the player
  /// would be listed twice.
  static bool _isHeroRow(CharacterRow row) =>
      row.globalId != null && row.uniqueName.toLowerCase() == 'hero';

  /// Fetch the ENTIRE character index once and cache it, split into actors
  /// (`!isOrphan`, minus the [_isHeroRow] the pinned Player row represents) and
  /// orphans (`isOrphan`).
  Future<void> _loadAll() async {
    final epoch = ++_epoch;
    setState(() => _loading = true);
    final page = await widget.load();
    if (!mounted || epoch != _epoch) return;
    setState(() {
      _loading = false;
      _error = page.error;
      _actors = _decorate(
        page.characters
            .where((r) => !r.isOrphan && !_isHeroRow(r))
            .toList(growable: false),
      );
      _orphans = _decorate(
        page.characters.where((r) => r.isOrphan).toList(growable: false),
      );
    });
  }

  /// The current filtered ACTOR list (case-insensitive substring of id OR the
  /// resolved display name). Empty query → the full actor list.
  List<_SearchableRow> get _filtered {
    final q = _query;
    if (q.isEmpty) return _actors;
    return _actors.where((e) => e.search.contains(q)).toList(growable: false);
  }

  /// The current filtered ORPHAN list — the SAME id|name predicate as
  /// [_filtered], applied to the trailing orphan group so a search query
  /// narrows the whole list, not just the spawned actors. Empty query → the
  /// full orphan list. (Orphans stay unpaginated; only the filter applies.)
  List<_SearchableRow> get _filteredOrphans {
    final q = _query;
    if (q.isEmpty) return _orphans;
    return _orphans.where((e) => e.search.contains(q)).toList(growable: false);
  }

  void _onSearchChanged(String _) {
    _debounce?.cancel();
    _debounce = Timer(const Duration(milliseconds: 200), _applyQuery);
  }

  void _applyQuery() {
    setState(() {
      _query = _search.text.trim().toLowerCase();
      _offset = 0; // New query → back to the first page.
    });
  }

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context);
    final theme = Theme.of(context);
    final scheme = theme.colorScheme;

    // Filter (id|name) then paginate the ACTOR list, both client-side. The
    // orphan group gets the same filter (no pagination — it renders in full
    // after the paginated actors).
    final filtered = _filtered;
    final orphans = _filteredOrphans;
    final total = filtered.length;
    // Clamp the cursor: a shrinking filtered set may leave it past the end.
    final offset = total == 0
        ? 0
        : (_offset >= total ? ((total - 1) ~/ _pageSize) * _pageSize : _offset);
    final pageItems = filtered.skip(offset).take(_pageSize).toList();

    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        // Pinned Player row FIRST (the player is not an NPC, so it sits above
        // the "search NPCs" field rather than under it).
        ListTile(
          dense: true,
          leading: const Icon(Icons.person_outline),
          title: Text(l10n.tabPlayer),
          selected: widget.selected.isPlayer,
          selectedTileColor: scheme.primaryContainer,
          selectedColor: scheme.primary,
          onTap: () => widget.onSelect(const Actor.player()),
        ),
        const Divider(height: 1),
        // Search field — filters the cached actor list client-side by id OR
        // resolved display name.
        Padding(
          padding: const EdgeInsets.all(8),
          child: TextField(
            controller: _search,
            decoration: InputDecoration(
              labelText: l10n.searchNpcs,
              isDense: true,
              prefixIcon: const Icon(Icons.search, size: 18),
              suffixIcon: _loading
                  ? const Padding(
                      padding: EdgeInsets.all(10),
                      child: SizedBox(
                        width: 16,
                        height: 16,
                        child: CircularProgressIndicator(strokeWidth: 2),
                      ),
                    )
                  : null,
            ),
            onChanged: _onSearchChanged,
            // Enter applies immediately (skip the debounce timer).
            onSubmitted: (_) {
              _debounce?.cancel();
              _applyQuery();
            },
          ),
        ),
        if (_error != null)
          Padding(
            padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 4),
            child: Text(
              _error!,
              style: TextStyle(color: scheme.error, fontSize: 12),
            ),
          ),
        // Actor pagination controls — client-side over the FILTERED list.
        if (total > 0)
          _CharacterPaginationBar(
            first: offset + 1,
            last: offset + pageItems.length,
            total: total,
            busy: _loading,
            hasPrevious: offset > 0,
            hasNext: offset + _pageSize < total,
            onPrevious: () => setState(() => _offset = offset - _pageSize),
            onNext: () => setState(() => _offset = offset + _pageSize),
          ),
        // Clean visual boundary between the pagination bar and the list.
        const Divider(height: 1),
        // Scrollable list — fills the remaining height. The ClipRect bounds the
        // list, but `ListTile.selectedTileColor` is painted by the NEAREST
        // enclosing Material — without one here it would draw on the ancestor
        // Scaffold Material (outside the clip) and bleed above the top edge when
        // a selected tile scrolls past it. Wrapping the list in its own Material
        // inside the ClipRect keeps that highlight clipped to the list bounds.
        Expanded(
          child: ClipRect(
            child: Material(
              type: MaterialType.transparency,
              child: _loading && _actors.isEmpty && _orphans.isEmpty
                  ? const Center(child: CircularProgressIndicator())
                  : ListView(
                      children: [
                        for (final entry in pageItems) ...[
                          _npcTile(entry, scheme, l10n),
                          const Divider(height: 1),
                        ],
                        // Orphan group: rendered ONLY when non-empty after the
                        // query filter — when a search matches no orphan the
                        // header disappears with the rows.
                        if (orphans.isNotEmpty) ...[
                          Padding(
                            padding: const EdgeInsets.fromLTRB(16, 12, 16, 4),
                            child: Text(l10n.characterOrphanGroup),
                          ),
                          for (final entry in orphans) ...[
                            _orphanTile(entry, scheme, l10n),
                            const Divider(height: 1),
                          ],
                        ],
                      ],
                    ),
            ),
          ),
        ),
      ],
    );
  }

  /// A spawned-actor row: dead/alive leading icon, display-name title, GlobalId
  /// subtitle, and compact aspect badges (knowledge / events) as the trailing.
  Widget _npcTile(
    _SearchableRow entry,
    ColorScheme scheme,
    AppLocalizations l10n,
  ) {
    final row = entry.row;
    final name = entry.name;
    final isSelected =
        !widget.selected.isPlayer && widget.selected.id == row.globalId;
    return ListTile(
      dense: true,
      // Death is encoded in the leading avatar: a deathly icon ONLY for a KILLED
      // character (isDead), the normal face otherwise. (Reviving is done on the
      // status row; this is just a glance indicator.)
      leading: Icon(
        row.isDead ? Icons.dangerous : Icons.face_outlined,
        color: row.isDead ? scheme.error : null,
      ),
      title: Text(name, maxLines: 1, overflow: TextOverflow.ellipsis),
      subtitle: Text(
        row.globalId ?? '',
        maxLines: 1,
        overflow: TextOverflow.ellipsis,
        style: const TextStyle(fontSize: 11),
      ),
      trailing: _aspectBadges(row, scheme, l10n),
      selected: isSelected,
      selectedTileColor: scheme.primaryContainer,
      selectedColor: scheme.primary,
      onTap: () => widget.onSelect(
        Actor.npc(
          id: row.globalId!,
          name: name,
          isDead: row.isDead,
          uniqueName: row.uniqueName,
        ),
      ),
    );
  }

  /// A knowledge-only orphan row: no GlobalId, so it is built with an
  /// `orphan:<uniqueName>` id sentinel (see [Actor.isOrphan]).
  Widget _orphanTile(
    _SearchableRow entry,
    ColorScheme scheme,
    AppLocalizations l10n,
  ) {
    final row = entry.row;
    // Fall back to the uniqueName if the resolved display name is blank.
    final name = entry.name.trim().isEmpty ? row.uniqueName : entry.name;
    final isSelected =
        !widget.selected.isPlayer &&
        widget.selected.id == 'orphan:${row.uniqueName}';
    return ListTile(
      dense: true,
      leading: const Icon(Icons.help_outline),
      title: Text(name, maxLines: 1, overflow: TextOverflow.ellipsis),
      trailing: row.hasKnowledge
          ? Tooltip(
              message: l10n.dialogKnowledge,
              child: Icon(
                Icons.menu_book_outlined,
                size: 18,
                color: scheme.onSurfaceVariant,
              ),
            )
          : null,
      selected: isSelected,
      selectedTileColor: scheme.primaryContainer,
      selectedColor: scheme.primary,
      onTap: () => widget.onSelect(
        Actor.npc(
          id: 'orphan:${row.uniqueName}',
          name: name,
          uniqueName: row.uniqueName,
        ),
      ),
    );
  }

  /// Compact trailing badges for an actor row: a book when it has captured
  /// knowledge, a history glyph when it has recorded events. No inventory badge.
  Widget? _aspectBadges(
    CharacterRow row,
    ColorScheme scheme,
    AppLocalizations l10n,
  ) {
    final badges = <Widget>[
      if (row.hasKnowledge)
        Tooltip(
          message: l10n.dialogKnowledge,
          child: Icon(
            Icons.menu_book_outlined,
            size: 18,
            color: scheme.onSurfaceVariant,
          ),
        ),
      if (row.hasEvents)
        Tooltip(
          message: l10n.sectionEvents,
          child: Icon(Icons.history, size: 18, color: scheme.onSurfaceVariant),
        ),
    ];
    if (badges.isEmpty) return null;
    return Wrap(spacing: 4, children: badges);
  }
}

/// Compact prev/next pagination row for the actor list. Mirrors the retired
/// `ActorSelector`'s `_NpcPaginationBar` page math but stays self-contained.
class _CharacterPaginationBar extends StatelessWidget {
  const _CharacterPaginationBar({
    required this.first,
    required this.last,
    required this.total,
    required this.busy,
    required this.hasPrevious,
    required this.hasNext,
    required this.onPrevious,
    required this.onNext,
  });

  final int first;
  final int last;
  final int total;
  final bool busy;
  final bool hasPrevious;
  final bool hasNext;
  final VoidCallback onPrevious;
  final VoidCallback onNext;

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context);
    final theme = Theme.of(context);
    final muted = theme.textTheme.bodySmall?.copyWith(
      color: theme.colorScheme.onSurfaceVariant,
    );
    return Padding(
      padding: const EdgeInsets.symmetric(horizontal: 4),
      child: Wrap(
        crossAxisAlignment: WrapCrossAlignment.center,
        children: [
          IconButton(
            tooltip: l10n.previousPage,
            visualDensity: VisualDensity.compact,
            icon: const Icon(Icons.chevron_left),
            onPressed: busy || !hasPrevious ? null : onPrevious,
          ),
          IconButton(
            tooltip: l10n.nextPage,
            visualDensity: VisualDensity.compact,
            icon: const Icon(Icons.chevron_right),
            onPressed: busy || !hasNext ? null : onNext,
          ),
          const SizedBox(width: 4),
          Text(l10n.rangeOfTotal(first, last, total), style: muted),
        ],
      ),
    );
  }
}
