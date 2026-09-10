import 'package:flutter/foundation.dart' show visibleForTesting;
import 'package:goresave/features/editor/domain/location_catalog.dart';
import 'package:goresave/l10n/app_localizations.dart';
import 'package:goresave/loc/game_lang.dart';

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

/// Localized area name, in this order: the game's own notification string when
/// the catalog carries a loc id, then our [appAreaLabel] for the areas the game
/// does not name, and only then the generated English [LocationArea.label] — a
/// safety net for an area added to the catalog before anyone translated it,
/// never the normal outcome.
///
/// Shared by the location picker and the locks panel: both group by the same
/// area codes, and a second copy of the table above is exactly the drift this
/// function exists to prevent.
///
/// NOTE on German: for this `area_*` family the real string sits in the
/// `german` set and `german_new` is NULL — inverted versus the rest of the
/// game's text, where `german_new` wins. No special casing is needed because
/// [resolveGameText] walks `lang.locSets` in order and SKIPS empty/missing
/// sets, so `german_new` being absent falls through to `german` on its own.
String localizedAreaLabel(
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
