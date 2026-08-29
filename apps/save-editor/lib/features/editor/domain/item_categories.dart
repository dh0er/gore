import 'package:goresave/features/editor/domain/editor_models.dart';
import 'package:goresave/features/editor/domain/item_stats.dart';
import 'package:goresave/l10n/app_localizations.dart';

/// The inventory categories Gothic 1 Remake itself uses.
///
/// These are the tabs on the game's own inventory rail, in its own order, taken
/// from the `UInventoryFilter_G1R_*` tables in the shipped script cache — the
/// same tables the game consults when it files an item. `other` is not a game
/// tab: it collects the few items whose type tag no filter claims, which the
/// game only ever shows under "All".
///
/// Which tab an item belongs to is decided by its `Item_*` type tag (see
/// [ItemStatsCatalog]); [itemCategoryFromId] is the fallback for ids the
/// bundled stats do not cover.
enum ItemCategory {
  meleeWeapon('Melee'),
  rangedWeapon('Ranged'),
  magic('Magic'),
  wearable('Wearables'),
  food('Food'),
  potion('Potions'),
  material('Materials'),
  document('Documents'),
  misc('Miscellaneous'),
  artefact('Artefacts'),
  other('Other');

  const ItemCategory(this.label);

  final String label;
}

/// Returns the localized display label for [category]. The English
/// [ItemCategory.label] is kept as a stable identifier / fallback; this is what
/// should be shown to the user.
///
/// The game's own wording for these tabs is in the extracted loc catalog under
/// the filter's `nameKey`, so callers that have the catalog should prefer that
/// and use this only when nothing has been extracted yet.
String localizedItemCategoryLabel(
  AppLocalizations l10n,
  ItemCategory category,
) {
  switch (category) {
    case ItemCategory.meleeWeapon:
      return l10n.itemCategoryMeleeWeapon;
    case ItemCategory.rangedWeapon:
      return l10n.itemCategoryRangedWeapon;
    case ItemCategory.magic:
      return l10n.itemCategoryMagic;
    case ItemCategory.wearable:
      return l10n.itemCategoryWearable;
    case ItemCategory.food:
      return l10n.itemCategoryFood;
    case ItemCategory.potion:
      return l10n.itemCategoryPotion;
    case ItemCategory.material:
      return l10n.itemCategoryMaterial;
    case ItemCategory.document:
      return l10n.itemCategoryDocument;
    case ItemCategory.misc:
      return l10n.itemCategoryMisc;
    case ItemCategory.artefact:
      return l10n.itemCategoryArtefact;
    case ItemCategory.other:
      return l10n.itemCategoryOther;
  }
}

/// The tab a `UInventoryFilter_G1R_*` id stands for, or null when it is not one
/// of ours.
///
/// Null covers the game's own "All" filter — which collects everything and is
/// no category — and any tab a future game build adds. Both must stay out of
/// [ItemCategory.other]: that is the editor's catch-all for items no tab
/// claims, and letting a filter land on it would rename and re-icon the group
/// after the wrong thing.
ItemCategory? itemCategoryFromFilterId(String filterId) {
  return switch (filterId) {
    'G1R_MeleeWeapons' => ItemCategory.meleeWeapon,
    'G1R_RangedWeapons' => ItemCategory.rangedWeapon,
    'G1R_Magic' => ItemCategory.magic,
    'G1R_Wereables' => ItemCategory.wearable,
    'G1R_Food' => ItemCategory.food,
    'G1R_Potions' => ItemCategory.potion,
    'G1R_Materials' => ItemCategory.material,
    'G1R_Documents' => ItemCategory.document,
    'G1R_Miscellaneous' => ItemCategory.misc,
    'G1R_Artefacts' => ItemCategory.artefact,
    _ => null,
  };
}

/// Category of an item, preferring the game's own answer.
///
/// [stats] is the bundled item-stats catalog; when it knows this item's type
/// tag the tab is exactly the one the game would open it under. Without it (an
/// id added by a newer game build, or a test) the class-name prefix decides,
/// which lands in the same tab for every shipped item family.
ItemCategory itemCategoryFor(String id, {ItemStatsCatalog? stats}) {
  final filter = stats?.filterFor(id);
  final category = filter == null ? null : itemCategoryFromFilterId(filter.id);
  return category ?? itemCategoryFromId(id);
}

/// Fallback classifier from the Angelscript class-name prefix, used when the
/// bundled item stats do not know the id. Prefix set verified against the UE4SS
/// object dump of 2026-06-12; the tabs are the game's.
ItemCategory itemCategoryFromId(String id) {
  if (_isArmorId(id)) return ItemCategory.wearable;
  if (id.startsWith('ItMw_')) return ItemCategory.meleeWeapon;
  if (id.startsWith('ItRw_')) return ItemCategory.rangedWeapon;
  // ItAm_ is ammunition (ItAm_Arrow/ItAm_Bolt); amulets live under ItAt_.
  if (id.startsWith('ItAm_')) return ItemCategory.rangedWeapon;
  if (id.startsWith('ItAr_Rune_') || id.startsWith('ItAr_Scroll_')) {
    return ItemCategory.magic;
  }
  if (id.startsWith('ItFo_Potion_') || id.startsWith('ItFo_Booze')) {
    return ItemCategory.potion;
  }
  if (id.startsWith('ItFo_')) return ItemCategory.food;
  if (id.startsWith('ItAt_Amulet_') || id.startsWith('ItAt_Ring_')) {
    return ItemCategory.wearable;
  }
  // Everything else under ItAt_ is a hunting trophy, which the game files with
  // the crafting materials rather than on its own tab.
  if (id.startsWith('ItAt_')) return ItemCategory.material;
  if (id.startsWith('ItMi_')) return ItemCategory.misc;
  if (id.startsWith('ItWr_')) return ItemCategory.document;
  if (id.startsWith('ItMs_')) return ItemCategory.artefact;
  if (id.startsWith('ItKe_') ||
      id.startsWith('ItKey') ||
      id.startsWith('ItChestKey') ||
      id.startsWith('ItDoorKey')) {
    return ItemCategory.artefact;
  }
  return ItemCategory.other;
}

/// Human-readable name derived from the class id; never reads game
/// localization data (legal posture: identifiers only).
String itemDisplayNameFromId(String id, {String fallback = 'Item'}) {
  const prefixes = [
    'ItMw_',
    'ItRw_',
    'ItAr_',
    'ItFo_',
    'ItMi_',
    'ItAt_',
    'ItWr_',
    'ItMs_',
    'ItKe_',
    'ItAm_',
  ];
  var name = id;
  var genericItemPrefix = false;
  for (final prefix in prefixes) {
    if (name.startsWith(prefix)) {
      name = name.substring(prefix.length);
      break;
    }
  }
  // Some game item families do not use the underscore category form
  // (`ItChestKey01`, `ItFocusStoneBridgeItem`, ...). Strip only their generic
  // `It` marker, then split camel-case and number boundaries so the normal UI
  // never has to fall back to echoing the exact technical id.
  if (identical(name, id) &&
      name.length > 2 &&
      name.startsWith('It') &&
      _isAsciiUpper(name.codeUnitAt(2))) {
    name = name.substring(2);
    genericItemPrefix = true;
  }
  final cleaned = genericItemPrefix
      ? _humanizeItemToken(name)
      : name.replaceAll('_', ' ').trim();
  return cleaned.isEmpty ? fallback : cleaned;
}

bool _isAsciiUpper(int codeUnit) => codeUnit >= 0x41 && codeUnit <= 0x5a;

String _humanizeItemToken(String value) {
  final out = StringBuffer();
  for (var i = 0; i < value.length; i++) {
    final current = value.codeUnitAt(i);
    final previous = i == 0 ? null : value.codeUnitAt(i - 1);
    final next = i + 1 < value.length ? value.codeUnitAt(i + 1) : null;
    final separator = current == 0x5f || current == 0x2d;
    if (separator) {
      if (out.isNotEmpty && !out.toString().endsWith(' ')) out.write(' ');
      continue;
    }
    final currentUpper = _isAsciiUpper(current);
    final currentDigit = current >= 0x30 && current <= 0x39;
    final previousUpper = previous != null && _isAsciiUpper(previous);
    final previousLower =
        previous != null && previous >= 0x61 && previous <= 0x7a;
    final previousDigit =
        previous != null && previous >= 0x30 && previous <= 0x39;
    final nextLower = next != null && next >= 0x61 && next <= 0x7a;
    final boundary =
        i > 0 &&
        previous != 0x5f &&
        previous != 0x2d &&
        ((currentUpper && (previousLower || previousDigit)) ||
            (currentUpper && previousUpper && nextLower) ||
            (currentDigit && !previousDigit) ||
            (!currentDigit && previousDigit));
    if (boundary && out.isNotEmpty && !out.toString().endsWith(' ')) {
      out.write(' ');
    }
    out.writeCharCode(current);
  }
  return out.toString().replaceAll(RegExp(r'\s+'), ' ').trim();
}

class InventoryItemGroup {
  const InventoryItemGroup({required this.category, required this.items});

  final ItemCategory category;
  final List<PrivateInventoryItem> items;
}

/// Groups items by category. Groups appear in [ItemCategory] declaration
/// order, empty groups are omitted, items are sorted within a group.
///
/// When [displayNameOf] is given, items sort case-insensitively by that
/// (localized) name — what the user actually reads — with the id as a stable
/// tiebreak. Without it (e.g. tests), items fall back to id order.
List<InventoryItemGroup> groupInventoryItems(
  List<PrivateInventoryItem> items, {
  String Function(PrivateInventoryItem item)? displayNameOf,
  ItemStatsCatalog? stats,
}) {
  final byCategory = <ItemCategory, List<PrivateInventoryItem>>{};
  for (final item in items) {
    byCategory
        .putIfAbsent(itemCategoryFor(item.id, stats: stats), () => [])
        .add(item);
  }
  int compare(PrivateInventoryItem a, PrivateInventoryItem b) {
    if (displayNameOf != null) {
      final byName = displayNameOf(
        a,
      ).toLowerCase().compareTo(displayNameOf(b).toLowerCase());
      if (byName != 0) return byName;
    }
    return a.id.compareTo(b.id);
  }

  return [
    for (final category in ItemCategory.values)
      if (byCategory.containsKey(category))
        InventoryItemGroup(
          category: category,
          items: byCategory[category]!..sort(compare),
        ),
  ];
}

/// True for any armor class name (base, per-NPC, or tier piece). Mirrors the
/// Rust `gore_catalog::is_armor_id` display-side classifier.
bool _isArmorId(String id) {
  if (!id.contains('Armor')) return false;
  if (id.startsWith('Armor_')) return true;
  final parts = id.split('_');
  if (parts.length < 2) return false;
  final head = parts.first;
  // The segment after the head must be exactly `Armor` (a tier suffix follows in
  // a later segment) — not merely start with `Armor`, which would also match an
  // `Armory` segment such as `NC_Armory_Door`. Mirrors the Rust classifiers.
  return head.length >= 2 &&
      head.length <= 4 &&
      RegExp(r'^[A-Za-z]+$').hasMatch(head) &&
      parts[1] == 'Armor';
}
