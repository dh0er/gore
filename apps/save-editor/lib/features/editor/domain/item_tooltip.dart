import 'package:flutter/foundation.dart';
import 'package:goresave/features/editor/domain/game_icons.dart';
import 'package:goresave/features/editor/domain/item_stats.dart';
import 'package:goresave/l10n/app_localizations.dart';
import 'package:goresave/loc/attribute_loc.dart';
import 'package:goresave/loc/game_lang.dart';
import 'package:goresave/loc/loc_catalog_provider.dart';

/// The stat block the game shows when the player hovers an item, rebuilt from
/// the item's own class defaults in the shipped script cache.
///
/// Sectioned the way the game's own card is: name, item type, the numbers, a
/// boxed protection block, a boxed requirement block, then the flavour text.
///
/// Every label is the game's own string where the user's install has been
/// extracted (`item_damage_*`, `ui_protection_protection`,
/// `ui_inventory_requirements`, `ui_stat_*`, `attributeset_armor_*`); trade
/// value, which the game never puts on this card, uses the editor's own.
class ItemTooltip {
  const ItemTooltip({
    this.title = '',
    this.subtitle = '',
    this.stats = const [],
    this.protection = const [],
    this.protectionLabel = '',
    this.recipe = const [],
    this.recipeLabel = '',
    this.ingredientFor = const [],
    this.ingredientForLabel = '',
    this.writing = const [],
    this.requirements = const [],
    this.requirementsLabel = '',
    this.protectionLabelFromCatalog = false,
    this.requirementsLabelFromCatalog = false,
    this.titleFromCatalog = true,
    this.recipeProduct = '',
    this.description = '',
  });

  /// Localized item name.
  final String title;

  /// False when [title] is an interface fallback rather than a catalog name.
  final bool titleFromCatalog;

  /// The item type as the game names it — "One-Handed Sword", "Scroll".
  final String subtitle;

  /// Damage, mana and trade value: the numbers under the title.
  final List<ItemTooltipRow> stats;

  /// What wearing the item protects against, under [protectionLabel].
  final List<ItemTooltipRow> protection;
  final String protectionLabel;

  /// True when [protectionLabel] came from the game catalog.
  final bool protectionLabelFromCatalog;

  /// The crafting chain a blueprint teaches, one row per step: what it takes on
  /// the left, what it yields on the right.
  final List<ItemTooltipRow> recipe;
  final String recipeLabel;

  /// Catalog item name inside [recipeLabel]. The words around it are interface
  /// text.
  final String recipeProduct;

  /// What can be made from this item, under [ingredientForLabel].
  final List<ItemTooltipRow> ingredientFor;
  final String ingredientForLabel;

  /// A writing's own text, paragraph by paragraph, headings marked as such.
  final List<ItemTooltipParagraph> writing;

  /// What the hero needs to use the item, under [requirementsLabel].
  final List<ItemTooltipRow> requirements;
  final String requirementsLabel;

  /// True when [requirementsLabel] came from the game catalog.
  final bool requirementsLabelFromCatalog;

  /// The item's flavour text.
  final String description;

  bool get isEmpty =>
      // The item type alone is a card worth showing — a key or a guard's
      // second armour has nothing else, and the game names its kind too.
      subtitle.isEmpty &&
      stats.isEmpty &&
      protection.isEmpty &&
      recipe.isEmpty &&
      ingredientFor.isEmpty &&
      writing.isEmpty &&
      requirements.isEmpty &&
      description.isEmpty;
}

/// One passage of a writing: a chapter heading, or a paragraph of its text.
@immutable
class ItemTooltipParagraph {
  const ItemTooltipParagraph(this.text, {this.isHeading = false});

  final String text;
  final bool isHeading;
}

/// One line of the card: an optional game glyph, a label, and the value the
/// game right-aligns against it.
class ItemTooltipRow {
  const ItemTooltipRow(
    this.label,
    this.value, {
    this.iconName,
    this.catalogLabel = true,
    this.interfaceValueRun,
    this.labelRuns = const [],
  });

  final String label;
  final String value;

  /// Shared game glyph in front of the label, or null for the game's ◆ bullet.
  final String? iconName;

  /// False when [label] is an interface string. Those keep the interface face
  /// when the card paints catalog text in the game-text face.
  final bool catalogLabel;

  /// A slice of [value] that came from interface text, such as the localized
  /// duration suffix. The rest of the value keeps the game-text face.
  final String? interfaceValueRun;

  /// Ordered pieces of [label]. Empty means the whole label follows
  /// [catalogLabel]. A recipe line uses this so a fallback id and the
  /// count or arrow stay on the interface face while a catalog name does not.
  final List<ItemTooltipLabelRun> labelRuns;
}

/// One piece of an [ItemTooltipRow] label.
class ItemTooltipLabelRun {
  const ItemTooltipLabelRun(this.text, {required this.fromCatalog});

  final String text;
  final bool fromCatalog;
}

/// Build the hover card for one item. Returns an empty tooltip when the
/// bundled stats know nothing about it, in which case the caller shows none.
/// How many products a material's "ingredient for" list names before it gives
/// up and counts the rest.
const _maxIngredientRows = 6;

ItemTooltip buildItemTooltip({
  required String title,
  String itemId = '',
  required ItemStats? stats,
  required Map<String, Map<String, String>> catalog,
  required GameLang lang,
  required AppLocalizations l10n,
  bool titleFromCatalog = true,
}) {
  if (stats == null || stats.isEmpty) return const ItemTooltip();
  String? game(String key) => resolveGameText(catalog, key, lang);

  final rows = <ItemTooltipRow>[];

  // Weapon damage. A rune or scroll has one figure per spell level instead,
  // which the game lists as "70/90/110/150".
  if (stats.spellLevels.isNotEmpty) {
    final tags = <String>{for (final level in stats.spellLevels) ...level.keys};
    for (final tag in tags) {
      final values = stats.spellLevels
          .map((level) => _number(level[tag]))
          .where((value) => value.isNotEmpty);
      if (values.isEmpty) continue;
      final damage = _damageLabel(game, tag);
      rows.add(
        ItemTooltipRow(
          damage.label,
          values.join('/'),
          catalogLabel: damage.fromCatalog,
        ),
      );
    }
  } else {
    for (final entry in stats.damage.entries) {
      final damage = _damageLabel(game, entry.key);
      rows.add(
        ItemTooltipRow(
          damage.label,
          _number(entry.value),
          catalogLabel: damage.fromCatalog,
        ),
      );
    }
  }

  // What casting it costs. Independent of the damage above: Light, Heal and
  // every teleport rune cost mana and deal none, and used to show nothing at
  // all because their cost hung off the damage block.
  if (stats.spellMana.isNotEmpty) {
    final initial = stats.spellMana
        .map((level) => _number(level['initialMana']))
        .where((value) => value.isNotEmpty)
        .join('/');
    final upkeep = stats.spellMana
        .map((level) => _number(level['manaPerSecond']))
        .where((value) => value.isNotEmpty)
        .join('/');
    if (initial.isNotEmpty) {
      final mana = game(
        upkeep.isEmpty
            ? 'ui_stat_manacost_text'
            : 'ui_stat_initialmanacost_text',
      );
      rows.add(
        ItemTooltipRow(
          mana ?? l10n.itemTooltipManaCost,
          initial,
          iconName: 'T_Icon_Mana',
          catalogLabel: mana != null,
        ),
      );
    }
    if (upkeep.isNotEmpty) {
      final perSecond = game('ui_stat_duration_measurement') ?? '/s';
      final upkeepLabel = game('ui_stat_manaupkeep_text');
      rows.add(
        ItemTooltipRow(
          upkeepLabel ?? l10n.itemTooltipManaUpkeep,
          '$upkeep$perSecond',
          iconName: 'T_Icon_Mana',
          catalogLabel: upkeepLabel != null,
        ),
      );
    }
  }

  // What eating or drinking it does. The game applies an over-time effect at a
  // fixed rate for a fixed span, so it reads "+1/Sek. · 3 s" rather than as one
  // figure; an instant one is just the amount.
  final perSecond = game('ui_stat_duration_measurement') ?? '/s';
  for (final effect in stats.onConsume) {
    final duration = effect.seconds;
    final secondsLabel = duration == null
        ? null
        : l10n.memoryEventSecondsValue(_number(duration));
    String withDuration(String amount) =>
        secondsLabel == null ? amount : '$amount · $secondsLabel';
    void add(String attribute, String amount, {String? setClass}) {
      rows.add(
        _attributeRow(
          catalog,
          lang,
          l10n,
          attribute,
          withDuration(amount),
          setClass: setClass,
          interfaceValueRun: secondsLabel,
        ),
      );
    }

    for (final entry in effect.magnitudes.entries) {
      final amount = _signed(entry.value);
      add(
        _consumeAttribute(entry.key),
        effect.isOverTime ? '$amount$perSecond' : amount,
      );
    }
    for (final entry in effect.percent.entries) {
      add(
        entry.key,
        '${_signed(entry.value * 100)}%',
        setClass: entry.key.startsWith('Resistance_')
            ? 'AttributeSet_Armor'
            : null,
      );
    }
  }

  // What wearing it grants. Protection gets the game's own shields and its own
  // heading; anything else (a ring's strength bonus) stays with the numbers.
  final protection = <ItemTooltipRow>[];
  for (final entry in stats.onEquip.entries) {
    final isProtection = entry.key.startsWith('Resistance_');
    final setClass = isProtection ? 'AttributeSet_Armor' : null;
    final row = _attributeRow(
      catalog,
      lang,
      l10n,
      entry.key,
      _signed(entry.value),
      setClass: setClass,
    );
    (isProtection ? protection : rows).add(row);
  }

  // Trade value is the one number here the game never puts on this card, but a
  // save editor is where an item's worth is actually useful. Stack size is
  // deliberately absent: the game has no string for it and never shows it, the
  // editor does not enforce it — a count of 999 on a 99-stack item loads fine —
  // and more than half the catalog declares 1 or nothing at all.
  if (stats.value != null) {
    rows.add(
      ItemTooltipRow(
        l10n.itemTooltipValue,
        _number(stats.value),
        catalogLabel: false,
      ),
    );
  }

  final requirements = [
    for (final entry in stats.requires.entries)
      _attributeRow(catalog, lang, l10n, entry.key, _number(entry.value)),
  ];

  ({String name, bool fromCatalog}) named(String id) {
    final name = localizedGameName(catalog, lang, id);
    if (name == null || name.trim().isEmpty) {
      return (name: id, fromCatalog: false);
    }
    return (name: name, fromCatalog: true);
  }

  List<ItemTooltipLabelRun> pieceRuns(Map<String, int> items) {
    final runs = <ItemTooltipLabelRun>[];
    for (final entry in items.entries) {
      if (runs.isNotEmpty) {
        runs.add(const ItemTooltipLabelRun(' + ', fromCatalog: false));
      }
      if (entry.value != 1) {
        runs.add(ItemTooltipLabelRun('${entry.value}× ', fromCatalog: false));
      }
      final item = named(entry.key);
      runs.add(ItemTooltipLabelRun(item.name, fromCatalog: item.fromCatalog));
    }
    return runs;
  }

  // A blueprint has no numbers of its own; what is worth knowing is the chain
  // it unlocks. Each step reads as one line — what goes in, what comes out —
  // because ingredients in one column and yields in another said nothing about
  // which belonged to which.
  final recipe = <ItemTooltipRow>[];
  for (final step in stats.teaches) {
    final needs = pieceRuns(step.needs);
    final makes = pieceRuns(step.makes);
    final runs = <ItemTooltipLabelRun>[
      ...needs,
      if (needs.isNotEmpty)
        const ItemTooltipLabelRun('  →  ', fromCatalog: false),
      ...makes,
    ];
    recipe.add(
      ItemTooltipRow(
        runs.map((run) => run.text).join(),
        '',
        iconName: _stationIcon(step.station),
        catalogLabel: false,
        labelRuns: runs,
      ),
    );
  }
  // The last step yields what the blueprint is actually for; the ones before it
  // are the parts on the way there. A fallback id in that yield is not a
  // catalog run, so the heading keeps the interface face.
  final products = stats.teaches.isEmpty
      ? const <({String name, bool fromCatalog})>[]
      : [for (final id in stats.teaches.last.makes.keys) named(id)];
  final product = products.map((item) => item.name).join(' + ');
  final productFromCatalog =
      products.isNotEmpty && products.every((item) => item.fromCatalog);

  // What a raw material is for. Long lists are cut: the point is what it makes,
  // not an inventory of every recipe in the game.
  final ingredientFor = [for (final id in stats.ingredientFor) named(id)]
    ..sort((a, b) => a.name.compareTo(b.name));
  final ingredientRows = <ItemTooltipRow>[
    for (final item in ingredientFor.take(_maxIngredientRows))
      ItemTooltipRow(item.name, '', catalogLabel: item.fromCatalog),
    if (ingredientFor.length > _maxIngredientRows)
      ItemTooltipRow(
        '+ ${ingredientFor.length - _maxIngredientRows}',
        '',
        catalogLabel: false,
      ),
  ];

  // A writing's own text, where it has one. Most carry no description at all —
  // the text IS the item.
  final written = <ItemTooltipParagraph>[];
  for (final part in stats.writing) {
    final text = game(part.id.toLowerCase());
    if (text == null || text.trim().isEmpty) continue;
    written.add(ItemTooltipParagraph(text.trim(), isHeading: part.isHeading));
  }

  final protectionName = protection.isEmpty
      ? null
      : game('ui_protection_protection');
  final requirementsName = requirements.isEmpty
      ? null
      : game('ui_inventory_requirements');
  return ItemTooltip(
    title: title,
    titleFromCatalog: titleFromCatalog,
    subtitle: _itemTypeName(game, stats.itemType) ?? '',
    stats: rows,
    protection: protection,
    protectionLabel: protection.isEmpty
        ? ''
        : (protectionName ?? l10n.itemTooltipProtection),
    protectionLabelFromCatalog: protectionName != null,
    recipe: recipe,
    recipeLabel: recipe.isEmpty ? '' : l10n.itemTooltipTeaches(product),
    recipeProduct: productFromCatalog ? product : '',
    ingredientFor: ingredientRows,
    ingredientForLabel: ingredientRows.isEmpty
        ? ''
        : l10n.itemTooltipIngredientFor,
    requirements: requirements,
    requirementsLabel: requirements.isEmpty
        ? ''
        : (requirementsName ?? l10n.itemTooltipRequirements),
    requirementsLabelFromCatalog: requirementsName != null,
    // Some items name no description class-side although the game ships one
    // under their own id — the permanent potions, above all.
    description:
        game(
          (stats.descriptionKey.isEmpty
                  ? '${itemId}_description'
                  : stats.descriptionKey)
              .toLowerCase(),
        ) ??
        '',
    writing: written,
  );
}

ItemTooltipRow _attributeRow(
  Map<String, Map<String, String>> catalog,
  GameLang lang,
  AppLocalizations l10n,
  String attribute,
  String amount, {
  String? setClass,
  String? interfaceValueRun,
}) {
  final fromCatalog = catalogAttributeName(
    catalog,
    lang,
    attribute,
    setClass: setClass,
  );
  return ItemTooltipRow(
    fromCatalog ?? readableAttributeName(attribute, l10n, setClass),
    amount,
    iconName: gameIconForAttribute(attribute, setClass),
    catalogLabel: fromCatalog != null,
    interfaceValueRun: interfaceValueRun,
  );
}

/// The game's name for an item type tag. GameplayTags are hierarchical and only
/// the coarse levels are named, so `Item_Weapon_Rune_FireBall` is looked up as
/// `item_weapon_rune` — exactly what the game's own tooltip shows.
String? _itemTypeName(String? Function(String) game, String itemType) {
  var tag = itemType.trim();
  while (tag.isNotEmpty) {
    final text = game(tag.toLowerCase());
    if (text != null && text.trim().isNotEmpty) return text;
    final cut = tag.lastIndexOf('_');
    if (cut < 0) return null;
    tag = tag.substring(0, cut);
  }
  return null;
}

({String label, bool fromCatalog}) _damageLabel(
  String? Function(String) game,
  String tag,
) {
  final text = game(tag.toLowerCase());
  if (text == null || text.trim().isEmpty) {
    return (label: tag, fromCatalog: false);
  }
  return (label: text, fromCatalog: true);
}

/// The game's own mark for the bench a recipe step is worked at.
String? _stationIcon(String station) => switch (station) {
  'forge' => 'T_Interact_Forge',
  'workbench' => 'T_Interaction_Use',
  'whetstone' => 'T_Icon_Melee',
  'inscription' => 'T_Interaction_Inscription',
  'alchemy' => 'T_Interaction_Alchemy',
  'cauldron' => 'T_Interaction_Cooking',
  _ => null,
};

/// The attribute a consume-effect parameter changes. The effects name most of
/// them exactly as the attribute set does; only the two healing ones differ.
String _consumeAttribute(String parameter) => switch (parameter) {
  'Heal' => 'Health',
  'MaxHeal' => 'MaxHealth',
  _ => parameter,
};

String _number(num? value) {
  if (value == null) return '';
  if (value == value.roundToDouble()) return value.toInt().toString();
  return value.toStringAsFixed(2).replaceFirst(RegExp(r'\.?0+$'), '');
}

String _signed(num value) {
  final text = _number(value);
  return value > 0 ? '+$text' : text;
}
