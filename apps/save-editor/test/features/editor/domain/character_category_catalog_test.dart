import 'package:flutter_test/flutter_test.dart';
import 'package:goresave/features/editor/domain/character_category_catalog.dart';

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();

  group('the bundled catalog', () {
    late CharacterCategoryCatalog catalog;

    setUpAll(() async {
      catalog = await loadCharacterCategoryCatalog();
    });

    test('a creature save id resolves without its definition prefix', () {
      // The save carries `Lizard-WP_…`; the definition is `Creature_Lizard`.
      for (final id in const [
        'Lizard-WP_ST_LIZARD_SPAWN_01-1',
        'Scavenger_Adult-WP_OW_PATH_SCAVENGER15_SPAWN01_CH2',
        'Molerat-WP_OC_SURROUNDINGS_MOLERAT_SPAWN_01-1',
      ]) {
        expect(
          catalog.categoryFor(id),
          CharacterCategory.creature,
          reason: '$id must read as a creature',
        );
      }
    });

    test('a creature save id that also dropped its underscores resolves', () {
      // `LizardFire-WP_…` for the definition `Creature_Lizard_Fire`.
      expect(
        catalog.categoryFor('LizardFire-WP_SW_LIZARDFIRE_SPAWN_01-1'),
        CharacterCategory.creature,
      );
    });

    test('humans stay human and are not swept up by the creature probe', () {
      for (final id in const [
        'OM_STT_Alberto_300-WorldPointActor_Alberto',
        'OC_STT_Diego-WorldPointActor_Diego',
      ]) {
        expect(catalog.categoryFor(id), CharacterCategory.human, reason: id);
        expect(catalog.isHuman(id), isTrue);
      }
    });

    test('an id nothing knows stays unresolved', () {
      expect(catalog.categoryFor('NotAnyone-WP_X'), isNull);
      expect(catalog.categoryFor(null), isNull);
    });

    test('a save id that dropped its kind prefix still resolves', () {
      // The save writes `Bloodfly-WP_…` and `OW_OPS_OrcPeasantM01_2068-…`; the
      // definitions are `Creature_Bloodfly` and `Orc_OW_OPS_OrcPeasantM01_2068`.
      expect(
        catalog.categoryFor('Bloodfly-WP_OW_BLOODFLY_SPAWN_01-1'),
        CharacterCategory.creature,
      );
      expect(
        catalog.categoryFor(
          'OW_OPS_OrcPeasantM01_2068-WorldPointActor_OrcPeasant',
        ),
        isNotNull,
      );
      expect(
        catalog.categoryFor('FM_OSL_Tarrok_2001-WorldPointActor_Tarrok'),
        isNotNull,
      );
    });

    test('a save id that spells itself out with spaces resolves', () {
      // The save writes `Minecrawler Nymph-WP_…`; the definition has no space.
      for (final id in const [
        'Minecrawler Nymph-WP_OM_MINECRAWLER_SPAWN_01-1',
        'Juvenile Troll-WP_OW_TROLL_SPAWN_01-1',
      ]) {
        expect(catalog.categoryFor(id), CharacterCategory.creature, reason: id);
      }
    });

    test('a save id that renumbered the definition still resolves', () {
      // The save spawns the Old Camp's `OC_GRD_Guard18_238` into the swamp as
      // `FM_GRD_Guard18_300N` — same guard, different world, new number.
      for (final id in const [
        'FM_GRD_Guard18_300N-WorldPointActor_Guard',
        'FM_GRD_Guard27_309N-WorldPointActor_Guard',
      ]) {
        expect(catalog.categoryFor(id), CharacterCategory.human, reason: id);
      }
    });

    test('the same species resolves whatever shape its id has', () async {
      // Every rule reads a particular id SHAPE, so whether a character
      // resolved at all came down to which shape it had. In one real save that
      // made the same animal a creature 57 times and an unknown 9 times, and
      // the list drew it as two species — "Wolf (9)" beside "Wolf (57)", and
      // the same for the blood flies, the meatbugs, the diggers and the
      // guards.
      for (final id in const [
        'Wolf-WP_OW_WOLF_SPAWN_01-1',
        'Wolf-OW_PATH_075_GUARD9_WP-1',
        'Meatbug-OC_MEATBUG_SPAWN_02-1',
        'Bloodfly-OW_BLOODFLY_SPAWN_01-1',
      ]) {
        expect(catalog.categoryFor(id), CharacterCategory.creature, reason: id);
      }
      // And the mercenary of the same name is still a man: the rule that names
      // him runs before the one that would read his waypoint.
      expect(
        catalog.categoryFor('NC_ORG_Wolf_855-WorldPointActor_wolf'),
        CharacterCategory.human,
      );
    });

    test('an ambiguous stripped id is left unresolved, not guessed', () {
      // Two definitions of different kinds sharing a stripped key must not
      // decide the species between them.
      final ambiguous = CharacterCategoryCatalog(const {
        'creature_shared': CharacterCategory.creature,
        'human_shared': CharacterCategory.human,
        'creature_only': CharacterCategory.creature,
      }, const {});
      expect(ambiguous.categoryFor('Shared-WP_X'), isNull);
      expect(ambiguous.categoryFor('Only-WP_X'), CharacterCategory.creature);
    });
  });
}
