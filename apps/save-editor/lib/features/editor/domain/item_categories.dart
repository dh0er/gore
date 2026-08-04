import 'package:goresave/features/editor/domain/editor_models.dart';
import 'package:goresave/l10n/app_localizations.dart';

/// Item categories for Gothic 1 Remake inventory items, derived from the
/// Angelscript class-name prefix (e.g. `ItMi_Orenugget` -> misc).
///
/// Prefix set verified against the UE4SS object dump of 2026-06-12.
enum ItemCategory {
  meleeWeapon('Melee weapons'),
  rangedWeapon('Ranged weapons'),
  armor('Armor'),
  ammunition('Ammunition'),
  rune('Runes'),
  scroll('Spell scrolls'),
  food('Food & potions'),
  misc('Miscellaneous'),
  amulet('Amulets'),
  ring('Rings'),
  trophy('Animal trophies'),
  writing('Writings'),
  mission('Mission items'),
  key('Keys'),
  other('Other');

  const ItemCategory(this.label);

  final String label;
}

/// Returns the localized display label for [category]. The English
/// [ItemCategory.label] is kept as a stable identifier / fallback; this is what
/// should be shown to the user.
String localizedItemCategoryLabel(
  AppLocalizations l10n,
  ItemCategory category,
) {
  switch (category) {
    case ItemCategory.meleeWeapon:
      return l10n.itemCategoryMeleeWeapon;
    case ItemCategory.rangedWeapon:
      return l10n.itemCategoryRangedWeapon;
    case ItemCategory.armor:
      return l10n.itemCategoryArmor;
    case ItemCategory.ammunition:
      return l10n.itemCategoryAmmunition;
    case ItemCategory.rune:
      return l10n.itemCategoryRune;
    case ItemCategory.scroll:
      return l10n.itemCategoryScroll;
    case ItemCategory.food:
      return l10n.itemCategoryFood;
    case ItemCategory.misc:
      return l10n.itemCategoryMisc;
    case ItemCategory.amulet:
      return l10n.itemCategoryAmulet;
    case ItemCategory.ring:
      return l10n.itemCategoryRing;
    case ItemCategory.trophy:
      return l10n.itemCategoryTrophy;
    case ItemCategory.writing:
      return l10n.itemCategoryWriting;
    case ItemCategory.mission:
      return l10n.itemCategoryMission;
    case ItemCategory.key:
      return l10n.itemCategoryKey;
    case ItemCategory.other:
      return l10n.itemCategoryOther;
  }
}

ItemCategory itemCategoryFromId(String id) {
  if (_isArmorId(id)) return ItemCategory.armor;
  if (id.startsWith('ItMw_')) return ItemCategory.meleeWeapon;
  if (id.startsWith('ItRw_')) return ItemCategory.rangedWeapon;
  // ItAm_ is ammunition (ItAm_Arrow/ItAm_Bolt); amulets live under ItAt_.
  if (id.startsWith('ItAm_')) return ItemCategory.ammunition;
  if (id.startsWith('ItAr_Rune_')) return ItemCategory.rune;
  if (id.startsWith('ItAr_Scroll_')) return ItemCategory.scroll;
  if (id.startsWith('ItFo_')) return ItemCategory.food;
  if (id.startsWith('ItMi_')) return ItemCategory.misc;
  if (id.startsWith('ItAt_Amulet_')) return ItemCategory.amulet;
  if (id.startsWith('ItAt_Ring_')) return ItemCategory.ring;
  if (id.startsWith('ItAt_')) return ItemCategory.trophy;
  if (id.startsWith('ItWr_')) return ItemCategory.writing;
  if (id.startsWith('ItMs_')) return ItemCategory.mission;
  if (id.startsWith('ItKe_') ||
      id.startsWith('ItKey') ||
      id.startsWith('ItChestKey') ||
      id.startsWith('ItDoorKey')) {
    return ItemCategory.key;
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
}) {
  final byCategory = <ItemCategory, List<PrivateInventoryItem>>{};
  for (final item in items) {
    byCategory.putIfAbsent(itemCategoryFromId(item.id), () => []).add(item);
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
