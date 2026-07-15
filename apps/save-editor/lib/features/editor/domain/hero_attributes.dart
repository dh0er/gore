import 'editor_models.dart';

/// Presentation-only resolver shared by the hero, NPC, and legacy attribute
/// editors. [setClass] is null for legacy summaries that do not expose the
/// owning AttributeSet class.
typedef AttributeLabelResolver = String Function(String id, String? setClass);

/// One pending `private.typed.setValue` edit.
class TypedValueEdit {
  const TypedValueEdit({required this.path, required this.value});

  final List<String> path;
  final Object value;
}

/// Result of loading the hero attribute subtree.
class HeroAttributesResult {
  const HeroAttributesResult({this.attributes = const [], this.error});

  final List<HeroAttribute> attributes;
  final String? error;
}

/// One hero gameplay attribute: the BaseValue/CurrentValue pair found at
/// `.../AttributesByGlobalId/{Hero}/AttributeSetsByClass/{setClass}/Attributes/{id}/...`
/// in the typed property tree. Paths are `private.typed.setValue` addressable.
class HeroAttribute {
  const HeroAttribute({
    required this.id,
    required this.setClass,
    this.basePath,
    this.currentPath,
    this.baseValue,
    this.currentValue,
  });

  final String id;
  final String setClass;
  final List<String>? basePath;
  final List<String>? currentPath;
  final double? baseValue;
  final double? currentValue;
}

enum HeroAttributeGroup { core, combat, resistances, thieving, advanced }

const heroCoreAttributeOrder = [
  'Health',
  'MaxHealth',
  'Mana',
  'MaxMana',
  'Strength',
  'Dexterity',
  'Level',
  'Experience',
  'SkillPoints',
  'MagicianLevel',
];

// The per-weapon critical-hit values used to have their own "Kampffertigkeiten"
// group; they are now hidden from the curated attribute view entirely (see
// [heroHiddenAttributeIds]) — still editable via the All-data browser. This
// list stays empty so the combat group machinery keeps compiling but never
// surfaces.
const heroCombatAttributes = <String>[];

/// Attribute ids hidden from the curated hero/NPC attribute view (the game
/// derives these from the learned skills, so editing them by hand is
/// misleading). They remain reachable in the All-data property browser.
const heroHiddenAttributeIds = <String>{
  'Critical_Fists',
  'Critical_OneHand',
  'Critical_TwoHand',
  'Critical_Orc',
};

const heroResistanceAttributes = [
  'Resistance_Blunt',
  'Resistance_Edge',
  'Resistance_Point',
  'Resistance_Fire',
  'Resistance_Energy',
  'Resistance_Ice',
  'Resistance_Wind',
  'Resistance_Falling',
];

const heroThievingAttributes = [
  'LockpickDurability',
  'LockpickPrecision',
  'PickPocketing',
];

HeroAttributeGroup heroAttributeGroup(String id) {
  if (heroCoreAttributeOrder.contains(id)) return HeroAttributeGroup.core;
  if (heroCombatAttributes.contains(id)) return HeroAttributeGroup.combat;
  if (heroResistanceAttributes.contains(id)) {
    return HeroAttributeGroup.resistances;
  }
  if (heroThievingAttributes.contains(id)) return HeroAttributeGroup.thieving;
  return HeroAttributeGroup.advanced;
}

/// Display label for an attribute id. SkillPoints are Gothic's learn points,
/// which is what players actually look for.
String heroAttributeLabel(String id) {
  if (id == 'SkillPoints') return 'Skill points (LP)';
  return id;
}

/// Stable ordering rank for an attribute id within its group, then across
/// groups. Shared by the player's [parseHeroAttributes] sort and the NPC
/// attribute panel so NPC rows order identically to the player's within a
/// group (and unlisted/advanced ids fall to the end). Exposes [_groupRank].
int heroAttributeRank(String id) => _groupRank(id);

int _groupRank(String id) {
  final group = heroAttributeGroup(id);
  final order = switch (group) {
    HeroAttributeGroup.core => heroCoreAttributeOrder,
    HeroAttributeGroup.combat => heroCombatAttributes,
    HeroAttributeGroup.resistances => heroResistanceAttributes,
    HeroAttributeGroup.thieving => heroThievingAttributes,
    HeroAttributeGroup.advanced => null,
  };
  if (order == null) return 1 << 20;
  return (group.index << 12) + order.indexOf(id);
}

/// Fold typed search hits into hero attributes. Only editable FloatProperty
/// leaves named BaseValue/CurrentValue under `AttributesByGlobalId/{Hero}`
/// count; everything else in the result page is ignored. Attributes with the
/// same id in different attribute sets stay separate entries.
List<HeroAttribute> parseHeroAttributes(List<TypedPropertyHit> hits) {
  final byPrefix = <String, _HeroAttributeBuilder>{};
  for (final hit in hits) {
    if (!hit.editable || hit.type != 'FloatProperty') continue;
    final path = hit.path;
    if (path.length < 4) continue;
    final leaf = path.last;
    if (leaf != 'BaseValue' && leaf != 'CurrentValue') continue;
    final heroIndex = path.indexOf('{Hero}');
    if (heroIndex < 1 || path[heroIndex - 1] != 'AttributesByGlobalId') {
      continue;
    }
    final idSegment = path[path.length - 2];
    if (!idSegment.startsWith('{') || !idSegment.endsWith('}')) continue;
    final id = idSegment.substring(1, idSegment.length - 1);
    if (heroHiddenAttributeIds.contains(id)) continue;
    final setIndex = path.indexOf('AttributeSetsByClass');
    var setClass = '';
    if (setIndex >= 0 && setIndex + 1 < path.length) {
      final seg = path[setIndex + 1];
      if (seg.startsWith('{') && seg.endsWith('}')) {
        setClass = seg.substring(1, seg.length - 1);
      }
    }
    final prefix = path.sublist(0, path.length - 1).join(' ');
    final builder = byPrefix.putIfAbsent(
      prefix,
      () => _HeroAttributeBuilder(id: id, setClass: setClass),
    );
    final value = double.tryParse(hit.value);
    if (leaf == 'BaseValue') {
      builder.basePath = path;
      builder.baseValue = value;
    } else {
      builder.currentPath = path;
      builder.currentValue = value;
    }
  }
  final attributes = byPrefix.values.map((b) => b.build()).toList()
    ..sort((a, b) {
      final rank = _groupRank(a.id).compareTo(_groupRank(b.id));
      if (rank != 0) return rank;
      final byId = a.id.compareTo(b.id);
      if (byId != 0) return byId;
      return a.setClass.compareTo(b.setClass);
    });
  return attributes;
}

class _HeroAttributeBuilder {
  _HeroAttributeBuilder({required this.id, required this.setClass});

  final String id;
  final String setClass;
  List<String>? basePath;
  List<String>? currentPath;
  double? baseValue;
  double? currentValue;

  HeroAttribute build() {
    return HeroAttribute(
      id: id,
      setClass: setClass,
      basePath: basePath,
      currentPath: currentPath,
      baseValue: baseValue,
      currentValue: currentValue,
    );
  }
}
