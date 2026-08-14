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

enum HeroAttributeGroup {
  core,
  combat,
  resistances,
  thieving,
  diving,
  sleep,
  intoxication,
  advanced,
}

/// The key an attribute is addressed by throughout the curated view: its plain
/// id, or `<AttributeSet>_<id>` when that id exists in more than one set.
///
/// The separator is an underscore, not a dot, because these keys are also the
/// arm names of the ICU `select` messages that carry the labels and tooltips,
/// and ICU rejects a dot there.
///
/// Four ids are shared between sets — `FillRatio`, `FillRatioPeriod` and
/// `MaxThresholdIndex` across Hunger/Thirst/Fatigue, and
/// `RecoveryRatePerHourOfSleep` across Health/Mana/Fatigue. They mean something
/// different in each, so grouping and labelling both need the set. Everything
/// else stays keyed by the bare id, which keeps the label table and the group
/// lists readable.
String heroAttributeKey(String id, [String? setClass]) {
  if (!_setQualifiedIds.contains(id)) return id;
  final set = setClass?.split('.').last ?? '';
  if (!set.startsWith('AttributeSet_')) return id;
  return '${set.substring('AttributeSet_'.length)}_$id';
}

const _setQualifiedIds = <String>{
  'FillRatio',
  'FillRatioPeriod',
  'MaxThresholdIndex',
  'RecoveryRatePerHourOfSleep',
};

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
];

/// Combat and movement. The per-weapon critical-hit values that used to live
/// here are hidden now (see [heroHiddenAttributeIds]); what remains is the
/// poise system plus the two global factors.
///
/// `SuperArmor` is the stagger pool: a hit subtracts its super-armour damage
/// and only staggers the hero once the pool is empty, so a higher value means
/// fewer interruptions. Its maximum is `20 + 3 x Level` plus whatever the worn
/// armour adds, which is why base and current differ in a real save.
const heroCombatAttributes = [
  'SuperArmor',
  'MaxSuperArmor',
  'DamageMultiplier',
  'SpeedModifier',
];

/// Breath and diving. `Oxygen` is literally seconds of air: the swim ability
/// subtracts `OxygenDepletionRate` (always 1) every second under water and
/// kills the hero at zero, and the Diving skill raises the capacity from 45 to
/// 150 while tripling the surface recovery.
const heroDivingAttributes = [
  'Oxygen',
  'MaxOxygen',
  'OxygenDepletionRate',
  'OxygenRecoveryRate',
  'CriticalLevelPercent',
];

/// Sleeping in a bed. `SleepTime` is the budget of restful hours behind the
/// game's "Sleep for:" slider — hours beyond it are the ones the game marks
/// "No resting bonus" — and it refills by `SleepTimeRecoveryAmount` every
/// `SleepTimeRecoveryPeriod`. The three per-hour recovery rates say what an
/// hour of sleep restores, which is why they belong here rather than with the
/// pools they act on.
const heroSleepAttributes = [
  'SleepTime',
  'MaxSleepTime',
  'SleepTimeRecoveryAmount',
  'SleepTimeRecoveryPeriod',
  'MaxRestTime',
  'Health_RecoveryRatePerHourOfSleep',
  'Mana_RecoveryRatePerHourOfSleep',
];

/// Booze and swampweed. Both run the same machine: a consumable adds points,
/// the level falls into one of three tiers that trade attributes against each
/// other, and the value decays by its depletion rate until sober.
const heroIntoxicationAttributes = [
  'Alcohol',
  'MaxAlcohol',
  'AlcoholDepletionRate',
  'Swampweed',
  'MaxSwampweed',
  'SwampweedDepletionRate',
];

/// Attribute ids hidden from the curated hero/NPC attribute view (the game
/// derives these from the learned skills, so editing them by hand is
/// misleading). They remain reachable in the All-data property browser.
///
/// Each one is the attribute a `GE_Skill_*` class raises, and the game
/// re-derives it from that class when the savegame is loaded: a save edited so
/// that only the Magic Circle CLASS said circle 6 — while MagicianLevel still
/// said -1 — let the hero use a circle 4 rune in game, and rune usability is
/// stated against MagicianLevel. So the value written here never survives the
/// load, and the skill's own control (Talente) is the only one that works.
const heroHiddenAttributeIds = <String>{
  'Critical_Fists',
  'Critical_OneHand',
  'Critical_TwoHand',
  'Critical_Orc',
  // Magic Circle. Its label collided with the Talente row's, so the Attribute
  // tab showed two identical "Magischer Kreis" controls, only one of which did
  // anything.
  'MagicianLevel',
  ..._heroUnusedAttributeIds,
};

/// Attributes the shipped game carries but never acts on — encumbrance, which
/// was designed and then left out: nothing in the script layer reads
/// `Toughness`, and carrying capacity is unlimited in play. The game still
/// SHOWS Toughness on its own character screen (`ui_attribute_toughness`), but
/// the number drives nothing, and A/B/C are the coefficients of the curve that
/// was meant to compute it — they do not reproduce the values the game actually
/// stores under any simple polynomial.
const _heroUnusedAttributeIds = <String>{
  'Toughness',
  'ToughnessA',
  'ToughnessB',
  'ToughnessC',
  // Hunger, thirst and fatigue: the game's optional Survival mode, which never
  // became reachable. Measured in game on 2026-08-13 with a UE4SS probe:
  // GetSurvivalModeState() was forced to true BEFORE the hero loaded, the six
  // need abilities are granted, the attribute sets are present, and Hunger sat
  // at 900/1000 — the harshest stage, which owes -15% Strength and 1 HP per
  // second. Strength stayed 30.0 and health stayed 71.0 for a minute. The
  // abilities never activate, so every one of these values is inert.
  'Hunger', 'MaxHunger',
  'Thirst', 'MaxThirst',
  'Fatigue', 'MaxFatigue',
  // These three exist ONLY in the Hunger/Thirst/Fatigue sets, so hiding them by
  // bare id is exact.
  'FillRatio', 'FillRatioPeriod', 'MaxThresholdIndex',
  // This one also exists on Health and Mana, where it is real — so it has to be
  // hidden by its set-qualified key, not by id.
  'Fatigue_RecoveryRatePerHourOfSleep',
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

/// The group an attribute belongs to. [setClass] disambiguates the handful of
/// ids that exist in several attribute sets; without it those fall back to
/// their bare id, which no group claims, so they land in `advanced`.
/// Whether an attribute is hidden from the curated view. [setClass] matters for
/// the ids that exist in several sets: `RecoveryRatePerHourOfSleep` is inert on
/// Fatigue but real on Health and Mana.
bool heroAttributeHidden(String id, [String? setClass]) =>
    heroHiddenAttributeIds.contains(id) ||
    heroHiddenAttributeIds.contains(heroAttributeKey(id, setClass));

HeroAttributeGroup heroAttributeGroup(String id, [String? setClass]) {
  final key = heroAttributeKey(id, setClass);
  for (final entry in _groupOrders.entries) {
    if (entry.value.contains(key)) return entry.key;
  }
  return HeroAttributeGroup.advanced;
}

/// Every group's ordered member list, in sidebar order. `advanced` is absent on
/// purpose: it is the catch-all for anything unlisted.
const _groupOrders = <HeroAttributeGroup, List<String>>{
  HeroAttributeGroup.core: heroCoreAttributeOrder,
  HeroAttributeGroup.combat: heroCombatAttributes,
  HeroAttributeGroup.resistances: heroResistanceAttributes,
  HeroAttributeGroup.thieving: heroThievingAttributes,
  HeroAttributeGroup.diving: heroDivingAttributes,
  HeroAttributeGroup.sleep: heroSleepAttributes,
  HeroAttributeGroup.intoxication: heroIntoxicationAttributes,
};

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
int heroAttributeRank(String id, [String? setClass]) =>
    _groupRank(id, setClass);

int _groupRank(String id, [String? setClass]) {
  final key = heroAttributeKey(id, setClass);
  final group = heroAttributeGroup(id, setClass);
  final order = _groupOrders[group];
  if (order == null) return 1 << 20;
  return (group.index << 12) + order.indexOf(key);
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
    final setIndex = path.indexOf('AttributeSetsByClass');
    var setClass = '';
    if (setIndex >= 0 && setIndex + 1 < path.length) {
      final seg = path[setIndex + 1];
      if (seg.startsWith('{') && seg.endsWith('}')) {
        setClass = seg.substring(1, seg.length - 1);
      }
    }
    // Needs the set: `RecoveryRatePerHourOfSleep` is inert on Fatigue but real
    // on Health and Mana, so the bare id cannot decide this.
    if (heroAttributeHidden(id, setClass)) continue;
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
      final rank = _groupRank(
        a.id,
        a.setClass,
      ).compareTo(_groupRank(b.id, b.setClass));
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
