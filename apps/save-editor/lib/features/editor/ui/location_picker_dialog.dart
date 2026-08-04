import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:goresave/features/editor/domain/location_catalog.dart';
import 'package:goresave/features/editor/ui/sidebar_tile.dart';
import 'package:goresave/l10n/app_localizations.dart';
import 'package:goresave/loc/game_lang.dart';
import 'package:goresave/loc/loc_catalog_provider.dart';

/// The result of [showLocationPickerDialog]: the chosen [spot] plus whether the
/// user asked for its orientation to be applied as well.
///
/// Deliberately WRITE-AGNOSTIC — it carries no command, no path and no pending
/// key. Its one caller today, the player transform editor, turns it into a
/// `private.player.setTransform` edit; the dialog knows nothing about that.
/// (It once served the NPC panel too. That panel is read-only now: the game
/// restores an NPC's placement from the level and discards the saved pose — see
/// `NpcPositionPanel`.)
class LocationPick {
  const LocationPick({required this.spot, required this.applyRotation});

  final LocationSpot spot;

  /// True when the caller should also write the spot's heading. The catalog
  /// carries yaw ONLY, so a caller honouring this writes `pitch: 0`,
  /// `yaw: spot.yaw`, `roll: 0` — never an invented pitch or roll.
  final bool applyRotation;
}

/// Opens the shared "choose a location" picker over the bundled location
/// catalog. Returns the [LocationPick] the user confirmed, or null when the
/// dialog was dismissed.
///
/// [catalogOverride] replaces the bundled asset in widget tests; production
/// callers leave it null and get [LocationCatalog.loadBundled].
Future<LocationPick?> showLocationPickerDialog(
  BuildContext context, {
  LocationCatalog? catalogOverride,
}) {
  return showDialog<LocationPick>(
    context: context,
    builder: (_) => _LocationPickerDialog(catalogOverride: catalogOverride),
  );
}

/// Our own name for an area the game itself does not name, or null when the
/// area is not one of them.
///
/// The catalog gives every area an English `label` and gives 18 of the 26 a
/// `locId` into the game's own strings. The other eight have no clean label
/// anywhere in the game's 43,851 ids — only quest titles, item names and
/// dialogue lines mention them — so their names are the editor's own and live
/// in the ARB like any other piece of UI text.
///
/// An explicit `switch` rather than "whatever the ARB happens to contain": an
/// area added to the catalog without a translation lands on `_` and is named by
/// `location_picker_dialog_test`, instead of silently rendering English inside
/// an otherwise German sidebar — which is the bug this table exists to close.
@visibleForTesting
String? appAreaLabel(String areaId, AppLocalizations l10n) => switch (areaId) {
  'CV' => l10n.locationAreaCavalornValley,
  'EF' => l10n.locationAreaEastForest,
  'FT' => l10n.locationAreaFogTower,
  'HC' => l10n.locationAreaTundra,
  'IWM' => l10n.locationAreaIllegalWeedMixers,
  'OA' => l10n.locationAreaOrcArena,
  'OG' => l10n.locationAreaOrcGraveyard,
  'SW' => l10n.locationAreaShipwreck,
  _ => null,
};

class _LocationPickerDialog extends ConsumerStatefulWidget {
  const _LocationPickerDialog({this.catalogOverride});

  final LocationCatalog? catalogOverride;

  @override
  ConsumerState<_LocationPickerDialog> createState() =>
      _LocationPickerDialogState();
}

/// One sidebar bucket: an area id (`''` for the spots the catalog's spatial
/// pass could not assign) and the spots filed under it.
typedef _AreaGroup = ({String areaId, List<LocationSpot> spots});

class _LocationPickerDialogState extends ConsumerState<_LocationPickerDialog> {
  /// Rendered rows per page. The catalog holds >10,000 spots — an uncapped
  /// `ListView` over the whole set would build a scroll extent for all of them
  /// on every keystroke. Same cap and the same prev/next idiom as
  /// [CharacterMasterList].
  static const _pageSize = 100;

  // Created ONCE: a future rebuilt inside build() resets the FutureBuilder and
  // reflashes the spinner on every setState — i.e. on every keystroke. Same
  // trap add_inventory_item_dialog.dart documents.
  late final Future<LocationCatalog> _catalogFuture =
      widget.catalogOverride == null
      ? LocationCatalog.loadBundled()
      : Future<LocationCatalog>.value(widget.catalogOverride);

  final TextEditingController _searchController = TextEditingController();
  // Debounced so a fast typist does not re-filter 10,000 rows per character.
  // No I/O is involved — this only throttles the client-side scan.
  Timer? _debounce;

  /// The applied query: trimmed and LOWERCASED, so it compares directly against
  /// [LocationSpot.search] (precomputed at parse time).
  String _query = '';

  /// Selected sidebar area id; null means "All". `''` is a real bucket id (the
  /// unassigned spots), so it cannot double as the All sentinel.
  String? _selectedArea;

  /// Item offset into the current filtered list.
  int _offset = 0;

  /// Opt-in, and off by default: a spot's yaw is the heading of someone USING
  /// that spot (facing the anvil, the ladder), not a sensible facing for
  /// someone merely relocated there.
  bool _applyRotation = false;

  // Grouping is O(spots); cache it against the catalog instance so it runs once
  // per load rather than once per keystroke. Labels are resolved at render time
  // (they depend on the language), so a locale change needs no regrouping.
  LocationCatalog? _groupedFor;
  List<_AreaGroup> _groups = const [];

  @override
  void dispose() {
    _debounce?.cancel();
    _searchController.dispose();
    super.dispose();
  }

  /// Buckets sorted by spot count DESCENDING (the areas a save actually uses
  /// float to the top), with the unassigned `''` bucket pinned last whatever
  /// its size. Ties break on the area id so the order is stable.
  List<_AreaGroup> _groupsOf(LocationCatalog catalog) {
    if (identical(_groupedFor, catalog)) return _groups;
    final byArea = <String, List<LocationSpot>>{};
    for (final spot in catalog.spots) {
      byArea.putIfAbsent(spot.area, () => []).add(spot);
    }
    final groups = [
      for (final entry in byArea.entries)
        (areaId: entry.key, spots: entry.value),
    ]..sort((a, b) {
      // The unlabelled bucket is last, never sorted by size.
      if (a.areaId.isEmpty != b.areaId.isEmpty) return a.areaId.isEmpty ? 1 : -1;
      final byCount = b.spots.length.compareTo(a.spots.length);
      return byCount != 0 ? byCount : a.areaId.compareTo(b.areaId);
    });
    _groupedFor = catalog;
    _groups = groups;
    return groups;
  }

  /// Localized area name, in this order: the game's own notification string
  /// when the catalog carries a loc id, then our [appAreaLabel] for the areas
  /// the game does not name, and only then the generated English
  /// [LocationArea.label] — a safety net for an area added to the catalog
  /// before anyone translated it, never the normal outcome.
  ///
  /// NOTE on German: for this `area_*` family the real string sits in the
  /// `german` set and `german_new` is NULL — inverted versus the rest of the
  /// game's text, where `german_new` wins. No special casing is needed because
  /// [resolveGameText] walks `lang.locSets` in order and SKIPS empty/missing
  /// sets, so `german_new` being absent falls through to `german` on its own.
  String _areaLabel(
    String areaId,
    LocationCatalog catalog,
    Map<String, Map<String, String>> locCatalog,
    GameLang lang,
    AppLocalizations l10n,
  ) {
    if (areaId.isEmpty) return l10n.locationAreaOther;
    final area = catalog.areaById(areaId);
    if (area == null) return areaId;
    final locId = area.locId;
    if (locId != null && locId.isNotEmpty) {
      final localized = resolveGameText(locCatalog, locId, lang);
      if (localized != null && localized.trim().isNotEmpty) return localized;
    }
    return appAreaLabel(areaId, l10n) ?? area.label;
  }

  void _onSearchChanged(String _) {
    _debounce?.cancel();
    _debounce = Timer(const Duration(milliseconds: 200), _applyQuery);
  }

  void _applyQuery() {
    if (!mounted) return;
    setState(() {
      _query = _searchController.text.trim().toLowerCase();
      _offset = 0; // A new query starts at the first page.
    });
  }

  void _selectArea(String? areaId) {
    _debounce?.cancel();
    setState(() {
      _selectedArea = areaId;
      _query = '';
      _searchController.clear();
      _offset = 0;
    });
  }

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context);
    return AlertDialog(
      title: Text(l10n.pickLocationDialogTitle),
      contentPadding: const EdgeInsets.fromLTRB(24, 16, 24, 0),
      content: SizedBox(
        width: 720,
        height: 520,
        child: FutureBuilder<LocationCatalog>(
          future: _catalogFuture,
          builder: (context, snapshot) {
            if (snapshot.connectionState != ConnectionState.done) {
              return const Center(child: CircularProgressIndicator());
            }
            final catalog = snapshot.data;
            if (snapshot.hasError || catalog == null || catalog.spots.isEmpty) {
              return Center(child: Text(l10n.locationCatalogUnavailable));
            }
            return _body(catalog, l10n);
          },
        ),
      ),
      actions: [
        TextButton(
          onPressed: () => Navigator.of(context).pop(),
          child: Text(l10n.cancel),
        ),
      ],
    );
  }

  Widget _body(LocationCatalog catalog, AppLocalizations l10n) {
    final theme = Theme.of(context);
    final lang = ref.watch(currentGameLangProvider);
    // `.value` and never `.asData`: during a catalog RELOAD the async value
    // still carries the previous data, while `.asData` is null and every name
    // would flash back to its raw id.
    final locCatalog = ref.watch(locCatalogProvider).value ?? const {};

    final groups = _groupsOf(catalog);
    final searching = _query.isNotEmpty;

    // A non-empty query searches the WHOLE catalog, ignoring the selected area.
    // The area assignment comes from a spatial pass and is not authoritative:
    // a mis-filed spot must stay reachable by name.
    final List<LocationSpot> shown;
    if (searching) {
      shown = catalog.spots
          .where((s) => s.search.contains(_query))
          .toList(growable: false);
    } else if (_selectedArea == null) {
      shown = catalog.spots;
    } else {
      shown =
          groups.where((g) => g.areaId == _selectedArea).firstOrNull?.spots ??
          const [];
    }

    final total = shown.length;
    // Clamp: a shrinking result set can leave the cursor past the end.
    final offset = total == 0
        ? 0
        : (_offset >= total ? ((total - 1) ~/ _pageSize) * _pageSize : _offset);
    final page = shown.skip(offset).take(_pageSize).toList(growable: false);

    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        TextField(
          controller: _searchController,
          decoration: InputDecoration(
            labelText: l10n.searchEntries,
            prefixIcon: const Icon(Icons.search),
            isDense: true,
          ),
          onChanged: _onSearchChanged,
          // Enter applies immediately, skipping the debounce.
          onSubmitted: (_) {
            _debounce?.cancel();
            _applyQuery();
          },
        ),
        const SizedBox(height: 8),
        Expanded(
          child: Row(
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: [
              SizedBox(
                // Wide enough for the longest area label plus its spot count
                // without ellipsis ("Illegal Weed Mixers (109)"); the German
                // and Russian labels run longer still.
                width: 260,
                child: DecoratedBox(
                  decoration: BoxDecoration(
                    color: theme.colorScheme.surfaceContainerLow,
                    borderRadius: BorderRadius.circular(12),
                  ),
                  child: SingleChildScrollView(
                    padding: const EdgeInsets.symmetric(vertical: 6),
                    child: Column(
                      children: [
                        SidebarTile(
                          icon: Icons.list_outlined,
                          label: l10n.allWithCount(catalog.spots.length),
                          selected: !searching && _selectedArea == null,
                          onTap: () => _selectArea(null),
                        ),
                        for (final group in groups)
                          SidebarTile(
                            icon: group.areaId.isEmpty
                                ? Icons.help_outline
                                : Icons.place_outlined,
                            label: l10n.categoryWithCount(
                              _areaLabel(
                                group.areaId,
                                catalog,
                                locCatalog,
                                lang,
                                l10n,
                              ),
                              group.spots.length,
                            ),
                            selected:
                                !searching && _selectedArea == group.areaId,
                            onTap: () => _selectArea(group.areaId),
                          ),
                      ],
                    ),
                  ),
                ),
              ),
              const SizedBox(width: 16),
              Expanded(
                child: total == 0
                    ? Center(child: Text(l10n.noEntriesMatch))
                    : Column(
                        crossAxisAlignment: CrossAxisAlignment.stretch,
                        children: [
                          if (total > _pageSize)
                            _paginationBar(
                              l10n,
                              theme,
                              first: offset + 1,
                              last: offset + page.length,
                              total: total,
                              hasPrevious: offset > 0,
                              hasNext: offset + _pageSize < total,
                            ),
                          Expanded(
                            child: ListView.builder(
                              itemCount: page.length,
                              itemBuilder: (context, index) => _spotTile(
                                page[index],
                                catalog,
                                locCatalog,
                                lang,
                                l10n,
                              ),
                            ),
                          ),
                        ],
                      ),
              ),
            ],
          ),
        ),
        CheckboxListTile(
          dense: true,
          contentPadding: EdgeInsets.zero,
          controlAffinity: ListTileControlAffinity.leading,
          value: _applyRotation,
          onChanged: (value) =>
              setState(() => _applyRotation = value ?? false),
          title: Text(l10n.applySpotRotation),
        ),
      ],
    );
  }

  Widget _paginationBar(
    AppLocalizations l10n,
    ThemeData theme, {
    required int first,
    required int last,
    required int total,
    required bool hasPrevious,
    required bool hasNext,
  }) {
    final muted = theme.textTheme.bodySmall?.copyWith(
      color: theme.colorScheme.onSurfaceVariant,
    );
    return Padding(
      padding: const EdgeInsets.only(bottom: 4),
      child: Wrap(
        crossAxisAlignment: WrapCrossAlignment.center,
        children: [
          IconButton(
            tooltip: l10n.previousPage,
            visualDensity: VisualDensity.compact,
            icon: const Icon(Icons.chevron_left),
            onPressed: hasPrevious
                ? () => setState(() => _offset = first - 1 - _pageSize)
                : null,
          ),
          IconButton(
            tooltip: l10n.nextPage,
            visualDensity: VisualDensity.compact,
            icon: const Icon(Icons.chevron_right),
            onPressed: hasNext
                ? () => setState(() => _offset = first - 1 + _pageSize)
                : null,
          ),
          const SizedBox(width: 4),
          Text(l10n.rangeOfTotal(first, last, total), style: muted),
        ],
      ),
    );
  }

  Widget _spotTile(
    LocationSpot spot,
    LocationCatalog catalog,
    Map<String, Map<String, String>> locCatalog,
    GameLang lang,
    AppLocalizations l10n,
  ) {
    return ListTile(
      dense: true,
      leading: const Icon(Icons.place_outlined),
      title: Text(spot.name, maxLines: 1, overflow: TextOverflow.ellipsis),
      // The area is shown on every row, not only while searching: a query
      // spanning the whole catalog otherwise gives no clue where a hit lies.
      subtitle: Text(
        _areaLabel(spot.area, catalog, locCatalog, lang, l10n),
        maxLines: 1,
        overflow: TextOverflow.ellipsis,
      ),
      onTap: () => Navigator.of(
        context,
      ).pop(LocationPick(spot: spot, applyRotation: _applyRotation)),
    );
  }
}
