import 'dart:async';

import 'package:flutter/material.dart';
import 'package:goresave/features/editor/domain/game_icons.dart';
import 'package:goresave/features/editor/ui/game_icon.dart';
import 'package:goresave/features/editor/ui/glossary_portrait.dart';
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

/// Entity-first master list of the characters in the save: the Player (pinned on
/// top) and every spawned actor, searchable and paginated. Selecting an entry
/// calls [onSelect]; the parent owns which actor is [selected] and re-passes it.
///
/// Characters the save knows of but never spawned — knowledge-only entries with
/// no GlobalId — are left out entirely: there is no actor to edit behind them.
///
/// Actors sharing a display name collapse into one expandable row carrying the
/// count, so the fifty guards and the forty scavengers a save spawns cost one
/// line each instead of ninety.
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
    this.showObjectIds = false,
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

  /// Whether raw GlobalIds / orphan knowledge keys are rendered as row
  /// subtitles. Search continues to match identifiers while they are hidden.
  final bool showObjectIds;

  @override
  State<CharacterMasterList> createState() => _CharacterMasterListState();
}

class _CharacterMasterListState extends State<CharacterMasterList> {
  static const _pageSize = 100;

  /// How many actors must share a name before they fold into one row. Hiding
  /// two rows behind a click buys nothing; hiding forty scavengers does.
  static const _groupThreshold = 3;

  final TextEditingController _search = TextEditingController();
  // Debounce live search. No core call is made per keystroke (the full list is
  // cached) — this only throttles the cheap client-side re-filter so a fast
  // typist doesn't rebuild the list on every character.
  Timer? _debounce;

  // The full, cached rows with precomputed search strings + display names,
  // split into spawned actors and knowledge-only orphans. Fetched once.
  List<_SearchableRow> _actors = const [];

  /// Display names whose group the user opened. Keyed by name so the state
  /// survives paging and re-filtering.
  final Set<String> _expanded = <String>{};
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
      // The names the groups are keyed by just changed under them.
      _expanded.clear();
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
      _expanded.clear();
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
  /// no GlobalId). The result is sorted alphabetically by the RESOLVED display
  /// name (case-insensitive), so the list order follows the localized names the
  /// user sees rather than the core's emission order; the key is the tie-break
  /// for a stable order among same-named rows. Re-run on a locale/catalog change
  /// (see [didUpdateWidget]) so the order tracks the active language.
  List<_SearchableRow> _decorate(List<CharacterRow> rows) {
    final decorated = [
      for (final row in rows)
        () {
          final key = row.globalId ?? row.uniqueName;
          final name = localizedNpcName(widget.locCatalog, widget.lang, key);
          return _SearchableRow(row, name, '$key\n$name'.toLowerCase());
        }(),
    ];
    decorated.sort((a, b) {
      final byName = a.name.toLowerCase().compareTo(b.name.toLowerCase());
      if (byName != 0) return byName;
      final aKey = a.row.globalId ?? a.row.uniqueName;
      final bKey = b.row.globalId ?? b.row.uniqueName;
      return aKey.compareTo(bKey);
    });
    return decorated;
  }

  /// True for the save's own "Hero" ACTOR row (the player's avatar). The core
  /// emits it like any other actor (it IS real data), but the pinned Player row
  /// already represents it — so the NPC section excludes it, or the player
  /// would be listed twice.
  static bool _isHeroRow(CharacterRow row) =>
      row.globalId != null && row.uniqueName.toLowerCase() == 'hero';

  /// Fetch the ENTIRE character index once and cache it, keeping the spawned
  /// actors (minus the [_isHeroRow] the pinned Player row represents). A
  /// character with no GlobalId never spawned; there is nothing to select.
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
    });
  }

  /// The current filtered ACTOR list (case-insensitive substring of id OR the
  /// resolved display name). Empty query → the full actor list.
  List<_SearchableRow> get _filtered {
    final q = _query;
    if (q.isEmpty) return _actors;
    return _actors.where((e) => e.search.contains(q)).toList(growable: false);
  }

  /// The filtered list folded into one entry per display name.
  ///
  /// [_decorate] sorts by resolved name, so rows sharing one are already
  /// adjacent — a single pass groups them. Fewer than [_groupThreshold] of a
  /// name stay separate rows. Pagination then counts a group as the one line it
  /// occupies while collapsed.
  List<_CharacterGroup> _groups(List<_SearchableRow> rows) {
    final groups = <_CharacterGroup>[];
    for (var start = 0; start < rows.length;) {
      var end = start + 1;
      while (end < rows.length && rows[end].name == rows[start].name) {
        end++;
      }
      final members = rows.sublist(start, end);
      if (members.length < _groupThreshold) {
        groups.addAll(members.map((row) => _CharacterGroup([row])));
      } else {
        groups.add(_CharacterGroup(members));
      }
      start = end;
    }
    return groups;
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

    // Filter (id|name), fold same-named actors into one group, then paginate —
    // all client-side. Paging counts groups, so the numbers match the lines the
    // user sees; an opened group then adds its members beyond the page size.
    final groups = _groups(_filtered);
    final total = groups.length;
    // Clamp the cursor: a shrinking filtered set may leave it past the end.
    final offset = total == 0
        ? 0
        : (_offset >= total ? ((total - 1) ~/ _pageSize) * _pageSize : _offset);
    final pageItems = groups.skip(offset).take(_pageSize).toList();

    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        // Pinned Player row FIRST (the player is not an NPC, so it sits above
        // the "search NPCs" field rather than under it).
        ListTile(
          dense: true,
          // Match the two-line NPC height only when optional ids are visible.
          // Default one-line mode keeps every row equally compact.
          contentPadding: EdgeInsets.symmetric(
            horizontal: 16,
            vertical: widget.showObjectIds ? 12 : 6,
          ),
          // The game draws no portrait of the nameless hero, so the player
          // takes the same silhouette it uses for a character it has none for.
          leading: const GlossaryPortrait(),
          // The player row carries no id, so it is a one-line tile with a
          // leading taller than its text — without this the name hangs from the
          // top of the picture instead of sitting level with it.
          titleAlignment: ListTileTitleAlignment.center,
          title: Text(
            l10n.tabPlayer,
            style: const TextStyle(fontWeight: FontWeight.bold),
          ),
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
              child: _loading && _actors.isEmpty
                  ? const Center(child: CircularProgressIndicator())
                  : ListView(
                      children: [
                        for (final group in pageItems)
                          ...(group.isSingle
                              ? [
                                  _npcTile(group.first, scheme, l10n),
                                  const Divider(height: 1),
                                ]
                              : _groupTiles(group, scheme, l10n)),
                      ],
                    ),
            ),
          ),
        ),
      ],
    );
  }

  /// A spawned-actor row: the character's picture, its display name, the
  /// GlobalId when ids are shown, and compact aspect badges as the trailing.
  ///
  /// [indent] pushes a row that sits inside an opened group in from the group
  /// header above it.
  Widget _npcTile(
    _SearchableRow entry,
    ColorScheme scheme,
    AppLocalizations l10n, {
    bool indent = false,
  }) {
    final row = entry.row;
    final name = entry.name;
    final isSelected =
        !widget.selected.isPlayer && widget.selected.id == row.globalId;
    return ListTile(
      dense: true,
      contentPadding: EdgeInsets.only(left: indent ? 32 : 16, right: 16),
      // Everyone shows the pencil portrait the glossary holds for them, or the
      // mark for their kind when there is none: a person for every generic
      // worker, bandit and guard, a creature for every monster. Death is a
      // trailing badge — it says something ABOUT the character, it is not who
      // the character is.
      leading: GlossaryPortrait(npcUniqueName: row.globalId),
      // A one-line row would otherwise hang its text from the top of a leading
      // taller than itself.
      titleAlignment: ListTileTitleAlignment.center,
      title: Text(name, maxLines: 1, overflow: TextOverflow.ellipsis),
      subtitle: widget.showObjectIds
          ? Text(
              row.globalId ?? '',
              maxLines: 1,
              overflow: TextOverflow.ellipsis,
              style: const TextStyle(fontSize: 11),
            )
          : null,
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

  /// One expandable row standing for every actor sharing a display name, and —
  /// while it is open — the rows themselves.
  List<Widget> _groupTiles(
    _CharacterGroup group,
    ColorScheme scheme,
    AppLocalizations l10n,
  ) {
    final open = _expanded.contains(group.name);
    final holdsSelection =
        !widget.selected.isPlayer &&
        group.members.any((e) => e.row.globalId == widget.selected.id);
    return [
      ListTile(
        dense: true,
        // The first member's picture stands for the group: they are the same
        // character over and over, so any of them draws the same one.
        leading: GlossaryPortrait(npcUniqueName: group.first.row.globalId),
        titleAlignment: ListTileTitleAlignment.center,
        title: Text(
          '${group.name} (${group.members.length})',
          maxLines: 1,
          overflow: TextOverflow.ellipsis,
        ),
        trailing: Icon(open ? Icons.expand_less : Icons.expand_more, size: 20),
        // Highlighted while it holds the selection, so a collapsed group still
        // shows where the user is.
        selected: holdsSelection && !open,
        selectedTileColor: scheme.primaryContainer,
        selectedColor: scheme.primary,
        onTap: () => setState(() {
          if (!_expanded.remove(group.name)) _expanded.add(group.name);
        }),
      ),
      const Divider(height: 1),
      if (open)
        for (final entry in group.members) ...[
          _npcTile(entry, scheme, l10n, indent: true),
          const Divider(height: 1),
        ],
    ];
  }

  /// Compact trailing badges for an actor row: what the row says ABOUT a
  /// character, in the game's own glyphs — captured dialogue knowledge, the
  /// shop he runs, and last the death mark for a killed one. Null when it says
  /// nothing, so a row without badges gives its whole width to the id.
  ///
  /// Deliberately short: the roles the glossary files a character under belong
  /// on the detail page beside his name, where there is room to name them. No
  /// inventory badge and no event badge either — nearly every actor has both,
  /// so neither told the reader anything.
  Widget? _aspectBadges(
    CharacterRow row,
    ColorScheme scheme,
    AppLocalizations l10n,
  ) {
    Widget badge(
      String message,
      String gameIcon,
      IconData fallback, [
      Color? color,
    ]) {
      return Tooltip(
        message: message,
        child: GameIcon(
          name: gameIcon,
          fallbackIcon: fallback,
          size: 18,
          color: color ?? scheme.onSurfaceVariant,
        ),
      );
    }

    final badges = <Widget>[
      if (row.hasKnowledge)
        badge(
          l10n.dialogKnowledge,
          gameIconKnowledge,
          Icons.menu_book_outlined,
        ),
      // The shop comes from the trader array itself, not from the glossary:
      // that is the record the editor can actually change.
      if (row.isTrader)
        badge(l10n.tabTrade, gameIconTrade, Icons.storefront_outlined),
      if (row.isDead)
        badge(l10n.npcStatusDead, gameIconDead, Icons.dangerous, scheme.error),
    ];
    if (badges.isEmpty) return null;
    return Wrap(spacing: 4, children: badges);
  }
}

/// Every actor sharing one resolved display name. A group of one renders as a
/// plain row; a bigger one folds into an expandable header.
class _CharacterGroup {
  _CharacterGroup(this.members);

  final List<_SearchableRow> members;

  _SearchableRow get first => members.first;
  String get name => first.name;

  /// Whether this renders as a plain row rather than an expandable group.
  bool get isSingle => members.length == 1;
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
