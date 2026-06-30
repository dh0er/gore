import 'dart:async';

import 'package:flutter/material.dart';
import 'package:goresave/features/editor/domain/actor.dart';
import 'package:goresave/features/editor/domain/npc_actors_page.dart';
import 'package:goresave/l10n/app_localizations.dart';
import 'package:goresave/loc/game_lang.dart';
import 'package:goresave/loc/loc_catalog_provider.dart';

/// Signature for the NPC page loader the [ActorSelector] drives. Matches
/// `EditorNotifier.loadNpcActors` so the parent can pass it directly, and lets
/// tests inject a fake without touching the core.
typedef NpcActorsLoader =
    Future<NpcActorsPage> Function({String query, int offset, int limit});

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

/// A single NPC paired with its precomputed, lowercased search string
/// (`id + resolved display name`). The search string is computed once when the
/// full list loads so each keystroke only does a cheap substring scan over
/// ~1638 cached strings rather than re-resolving every name.
class _SearchableNpc {
  const _SearchableNpc(this.npc, this.name, this.search);

  final NpcActor npc;
  final String name;
  final String search;
}

/// Shared left-sidebar that lists the Player (pinned on top) and all NPCs
/// (searchable + paginated). Selecting an entry calls [onSelect]; the parent
/// owns which actor is [selected] and re-passes it.
///
/// The full NPC list is fetched ONCE (one save decompress) and cached; search
/// and pagination then run entirely CLIENT-SIDE. This makes search instant and
/// lets it match the RESOLVED display name (loc catalog / prettify fallback) in
/// addition to the raw id — server-side `query` only matched the id substring.
class ActorSelector extends StatefulWidget {
  const ActorSelector({
    super.key,
    required this.selected,
    required this.onSelect,
    required this.loadNpcs,
    required this.reloadKey,
    required this.locCatalog,
    required this.lang,
  });

  /// The currently selected actor (player or NPC). Owned by the parent so the
  /// highlight stays in sync with the shared editor state.
  final Actor selected;

  /// Called when the user taps the Player row or an NPC row.
  final void Function(Actor) onSelect;

  /// Loads the full NPC list (called once with an empty query). The loader must
  /// return EVERY NPC — paging through the core's clamped result set internally
  /// (e.g. `EditorNotifier.loadAllNpcActors`) — which the selector then filters +
  /// paginates client-side. Injected so tests can supply a fake.
  final NpcActorsLoader loadNpcs;

  /// Identifies the inspected save (e.g. the inspection identity or save path).
  /// When it changes — switching save or refreshing — the cached NPC list is
  /// stale (it belongs to the previous file), so the selector resets and
  /// re-fetches. `loadNpcs` is a stable method reference, so it alone can't
  /// signal a save change; this key does.
  final Object reloadKey;

  /// Loaded localization catalog (`id -> {set -> text}`) used to resolve NPC
  /// display names — the same input the memory-events list watches.
  final Map<String, Map<String, String>> locCatalog;

  /// The current game language, driving which loc set the name resolves from.
  final GameLang lang;

  @override
  State<ActorSelector> createState() => _ActorSelectorState();
}

class _ActorSelectorState extends State<ActorSelector> {
  static const _pageSize = 100;

  final TextEditingController _search = TextEditingController();
  // Debounce live search. No core call is made per keystroke anymore (the full
  // list is cached) — this only throttles the cheap client-side re-filter so a
  // fast typist doesn't rebuild the list on every character.
  Timer? _debounce;

  // The full, cached NPC list with precomputed search strings. Fetched once.
  List<_SearchableNpc> _all = const [];
  String? _error;
  String _query = '';
  // Client-side page cursor over the FILTERED list (an item offset, like the
  // server cursor it replaces).
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
  void didUpdateWidget(covariant ActorSelector oldWidget) {
    super.didUpdateWidget(oldWidget);
    // Re-resolve cached names if the catalog or language changes (the loaded
    // NPC ids are unchanged, so no refetch — just recompute search strings).
    if (oldWidget.locCatalog != widget.locCatalog ||
        oldWidget.lang != widget.lang) {
      _all = _decorate(_all.map((e) => e.npc).toList(growable: false));
    }
    // A new loader OR a new reloadKey (a different save/inspection) means the
    // cached list is stale: reset search/cursor/list and re-fetch against the
    // new save. The selector is kept alive across save switches (the tab body
    // persists), so without this the previous file's NPCs + defeated badges
    // would linger while attribute/inventory ops run against the new save.
    if (oldWidget.loadNpcs != widget.loadNpcs ||
        oldWidget.reloadKey != widget.reloadKey) {
      _search.clear();
      _query = '';
      _offset = 0;
      _all = const [];
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

  /// Pair each NPC with its resolved display name + a lowercased `id + name`
  /// search string, computed once so per-keystroke filtering is a cheap scan.
  List<_SearchableNpc> _decorate(List<NpcActor> npcs) {
    return [
      for (final npc in npcs)
        () {
          final name = localizedNpcName(widget.locCatalog, widget.lang, npc.id);
          return _SearchableNpc(npc, name, '${npc.id}\n$name'.toLowerCase());
        }(),
    ];
  }

  /// Fetch the ENTIRE NPC list once and cache it. The injected loader returns the
  /// complete set (it pages through the core's clamped result internally), so the
  /// selector's filtering + pagination are then purely client-side.
  Future<void> _loadAll() async {
    final epoch = ++_epoch;
    setState(() => _loading = true);
    final page = await widget.loadNpcs(query: '', offset: 0);
    if (!mounted || epoch != _epoch) return;
    setState(() {
      _loading = false;
      _error = page.error;
      _all = _decorate(page.npcs);
    });
  }

  /// The current filtered NPC list (case-insensitive substring of id OR the
  /// resolved display name). Empty query → the full list.
  List<_SearchableNpc> get _filtered {
    final q = _query;
    if (q.isEmpty) return _all;
    return _all.where((e) => e.search.contains(q)).toList(growable: false);
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

    // Filter (id|name) then paginate, both client-side over the cached list.
    final filtered = _filtered;
    final total = filtered.length;
    // Clamp the cursor: a shrinking filtered set may leave it past the end.
    final offset = total == 0
        ? 0
        : (_offset >= total ? ((total - 1) ~/ _pageSize) * _pageSize : _offset);
    final pageItems = filtered.skip(offset).take(_pageSize).toList();
    // A lightweight page descriptor purely to drive the existing pagination bar
    // math (first/last/total, prev/next enablement) over the FILTERED list.
    final pageInfo = NpcActorsPage(
      npcs: pageItems.map((e) => e.npc).toList(growable: false),
      total: total,
      offset: offset,
      limit: _pageSize,
    );

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
        // Search field — filters the cached list client-side by id OR resolved
        // display name. Sits below the Player tile and above the NPC list.
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
        // NPC pagination controls — client-side over the FILTERED list.
        if (pageInfo.total > 0)
          _NpcPaginationBar(
            page: pageInfo,
            busy: _loading,
            onPage: (o) => setState(() => _offset = o),
          ),
        // Clean visual boundary between the pagination bar and the list.
        const Divider(height: 1),
        // Scrollable NPC list — fills the remaining height. The ClipRect bounds
        // the list, but `ListTile.selectedTileColor` is painted by the NEAREST
        // enclosing Material — without one here it would draw on the ancestor
        // Scaffold Material (outside the clip) and bleed above the top edge when
        // a selected tile scrolls past it. Wrapping the list in its own Material
        // inside the ClipRect keeps that highlight clipped to the list bounds.
        Expanded(
          child: ClipRect(
            child: Material(
              type: MaterialType.transparency,
              child: _loading && _all.isEmpty
                  ? const Center(child: CircularProgressIndicator())
                  : ListView.separated(
                      itemCount: pageItems.length,
                      separatorBuilder: (_, _) => const Divider(height: 1),
                      itemBuilder: (context, index) {
                        final entry = pageItems[index];
                        final npc = entry.npc;
                        // Resolved display name, precomputed when the list loaded.
                        final name = entry.name;
                        final isSelected =
                            !widget.selected.isPlayer &&
                            widget.selected.id == npc.id;
                        return ListTile(
                          dense: true,
                          // Death is encoded in the leading avatar: a clearly
                          // deathly icon ONLY for a KILLED NPC (isDead), the
                          // normal face for one that is alive — including a
                          // merely defeated/knocked-out NPC. (Reviving is done
                          // on the Status row of the core stats group; this is
                          // just a glance indicator.)
                          leading: Icon(
                            npc.isDead ? Icons.dangerous : Icons.face_outlined,
                            color: npc.isDead ? scheme.error : null,
                          ),
                          title: Text(
                            name,
                            maxLines: 1,
                            overflow: TextOverflow.ellipsis,
                          ),
                          // Raw GlobalId retained as the subtitle (user request).
                          // No trailing widget reserves width anymore, so the id
                          // spans the full tile width before the ellipsis kicks in.
                          subtitle: Text(
                            npc.id,
                            maxLines: 1,
                            overflow: TextOverflow.ellipsis,
                            style: const TextStyle(fontSize: 11),
                          ),
                          selected: isSelected,
                          selectedTileColor: scheme.primaryContainer,
                          selectedColor: scheme.primary,
                          onTap: () => widget.onSelect(
                            Actor.npc(id: npc.id, name: name, isDead: npc.isDead),
                          ),
                        );
                      },
                    ),
            ),
          ),
        ),
      ],
    );
  }
}

/// Compact prev/next pagination row for the NPC list. Mirrors the page math of
/// the progression panel's shared pagination bar but stays self-contained so
/// the ActorSelector has no cross-file widget dependency.
class _NpcPaginationBar extends StatelessWidget {
  const _NpcPaginationBar({
    required this.page,
    required this.busy,
    required this.onPage,
  });

  final NpcActorsPage page;
  final bool busy;
  final void Function(int newOffset) onPage;

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context);
    final theme = Theme.of(context);
    final muted = theme.textTheme.bodySmall?.copyWith(
      color: theme.colorScheme.onSurfaceVariant,
    );
    final first = page.total == 0 ? 0 : page.offset + 1;
    final last = page.offset + page.npcs.length;
    return Padding(
      padding: const EdgeInsets.symmetric(horizontal: 4),
      child: Wrap(
        crossAxisAlignment: WrapCrossAlignment.center,
        children: [
          IconButton(
            tooltip: l10n.previousPage,
            visualDensity: VisualDensity.compact,
            icon: const Icon(Icons.chevron_left),
            onPressed: busy || !page.hasPrevious
                ? null
                : () => onPage((page.pageIndex - 1) * page.limit),
          ),
          IconButton(
            tooltip: l10n.nextPage,
            visualDensity: VisualDensity.compact,
            icon: const Icon(Icons.chevron_right),
            onPressed: busy || !page.hasNext
                ? null
                : () => onPage((page.pageIndex + 1) * page.limit),
          ),
          const SizedBox(width: 4),
          Text(l10n.rangeOfTotal(first, last, page.total), style: muted),
        ],
      ),
    );
  }
}
