import 'dart:async';

import 'package:flutter/material.dart';
import 'package:goresave/features/editor/domain/actor.dart';
import 'package:goresave/features/editor/domain/character_index.dart';
import 'package:goresave/features/editor/ui/actor_selector.dart' show localizedNpcName;
import 'package:goresave/l10n/app_localizations.dart';
import 'package:goresave/loc/game_lang.dart';

/// A single [CharacterRow] paired with its precomputed, lowercased search string
/// and resolved display name. Mirrors `_SearchableNpc` in [ActorSelector]: names
/// resolve ONCE when the list loads so each keystroke is a cheap substring scan
/// over the cached strings rather than re-resolving every name.
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
/// addition to the raw id. Cloned from [ActorSelector]'s proven structure for a
/// different row type ([CharacterRow] instead of `NpcActor`).
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
  /// display names the SAME way [ActorSelector] does.
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

  /// Fetch the ENTIRE character index once and cache it, split into actors
  /// (`!isOrphan`) and orphans (`isOrphan`).
  Future<void> _loadAll() async {
    final epoch = ++_epoch;
    setState(() => _loading = true);
    final page = await widget.load();
    if (!mounted || epoch != _epoch) return;
    setState(() {
      _loading = false;
      _error = page.error;
      _actors = _decorate(
        page.characters.where((r) => !r.isOrphan).toList(growable: false),
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

    // Filter (id|name) then paginate the ACTOR list, both client-side.
    final filtered = _filtered;
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
        // list, and its own Material keeps `ListTile.selectedTileColor` clipped
        // to the list bounds (see ActorSelector for the rationale).
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
                        // Orphan group: rendered ONLY when non-empty.
                        if (_orphans.isNotEmpty) ...[
                          const Padding(
                            padding: EdgeInsets.fromLTRB(16, 12, 16, 4),
                            // Plain string for now — a later task localizes it.
                            child: Text('Weitere'),
                          ),
                          for (final entry in _orphans) ...[
                            _orphanTile(entry, scheme),
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
  Widget _orphanTile(_SearchableRow entry, ColorScheme scheme) {
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
          ? Icon(Icons.menu_book_outlined, size: 18, color: scheme.onSurfaceVariant)
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
          message: l10n.sectionKnowledge,
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

/// Compact prev/next pagination row for the actor list. Mirrors
/// [ActorSelector]'s `_NpcPaginationBar` page math but stays self-contained.
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
