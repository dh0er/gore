import 'dart:io';

import 'package:flutter_test/flutter_test.dart';
import 'package:goresave/features/editor/domain/glossary_images.dart';
import 'package:goresave/features/editor/domain/glossary_npc_catalog.dart';
import 'package:path/path.dart' as p;

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();

  group('the bundled portrait map', () {
    late GlossaryImageCatalog catalog;

    setUpAll(() async {
      catalog = await GlossaryImageCatalog.loadBundled();
    });

    test('covers every character the glossary catalog lists', () async {
      final entries = await loadGlossaryNpcCatalog();
      expect(entries, isNotEmpty);
      final without = entries
          .where(
            (entry) =>
                catalog.pathFor(
                  documentClass: entry.documentClass,
                  size: GlossaryImageSize.thumbnail,
                  gamePath: r'C:\game',
                ) ==
                null,
          )
          .map((entry) => entry.uniqueName)
          .toList();
      expect(without, isEmpty, reason: 'these entries have no portrait');
    });

    test('builds the path the installation actually uses', () {
      final path = catalog.pathFor(
        documentClass: '/Script/Angelscript.Document_Glossary_OC_STT_DIEGO',
        size: GlossaryImageSize.banner,
        gamePath: r'C:\game',
      );
      expect(path, isNotNull);
      expect(p.split(path!), containsAllInOrder(<String>['G1R', 'Story']));
      expect(path, endsWith('_M.png'));
      expect(path, contains('Glossary'));
      expect(path, contains('Characters'));
    });

    test('the thumbnail and the banner differ only by their suffix', () {
      String? of(GlossaryImageSize size) => catalog.pathFor(
        documentClass: 'Document_Glossary_Biter',
        size: size,
        gamePath: r'C:\game',
      );
      final small = of(GlossaryImageSize.thumbnail)!;
      final medium = of(GlossaryImageSize.banner)!;
      expect(small.replaceAll('_S.png', ''), medium.replaceAll('_M.png', ''));
      expect(small, contains('Creatures'));
    });

    test('a monster variant finds the species the glossary draws', () {
      // Only about half the monsters in a real save name their species
      // exactly; the rest are variants of one sketch.
      const expected = {
        'Scavenger_Adult': 'Scavenger',
        'ScavengerYoung': 'Scavenger',
        'Scavenger_Adult_Rideable': 'Scavenger',
        'Goblin_Black': 'Goblin',
        'GoblinWarrior': 'Goblin',
        'Minecrawler Nymph': 'Minecrawler',
        'MinecrawlerTemple': 'Minecrawler',
        'Minecrawler_Queen': 'Minecrawler',
        'Juvenile Troll': 'Troll',
        'TC_TrollAdult_01': 'Troll',
        'Biter_OrcGraveyard': 'Biter',
        'Lurker_Homer': 'Lurker',
        'SwampsharkNamed': 'Swampshark',
        'Skeleton Scout': 'Skeleton',
        'SH_Zombie_01': 'Zombie',
        'MotherMolerat': 'Molerat',
        'Tundra Wolf': 'Wolf',
        'Viran_Bloodfly': 'Bloodfly',
        // Respelled species: the sketch is `Shadowbeast`, the save spawns
        // `ShadowBeastCave`.
        'ShadowBeastCave': 'Shadowbeast',
        'ShadowBeastFrost': 'Shadowbeast',
      };
      expected.forEach((variant, species) {
        expect(
          catalog.creatureDocumentFor(variant),
          'Document_Glossary_$species',
          reason: variant,
        );
      });
    });

    test('a species the game spelled backwards is not mistaken for a shorter '
        'one it contains', () {
      // `LizardFire` contains `Lizard`, which the glossary also draws — the
      // full two-word match has to win.
      expect(
        catalog.creatureDocumentFor('LizardFire'),
        'Document_Glossary_FireLizard',
      );
      expect(
        catalog.creatureDocumentFor('OA_Lizard_Fire_01'),
        'Document_Glossary_FireLizard',
      );
      expect(catalog.creatureDocumentFor('Lizard'), 'Document_Glossary_Lizard');
      expect(
        catalog.creatureDocumentFor('Golem_Stone_Bridge'),
        'Document_Glossary_StoneGolem',
      );
      expect(
        catalog.creatureDocumentFor('SkeletonWarrior'),
        'Document_Glossary_Skeleton',
      );
    });

    test('a character finds the sketch no glossary document names', () {
      // The installation draws about thirty pictures no document points at.
      const expected = {
        'NC_SLD_Orik_701': 'Orik',
        'OC_STT_Balam_324': 'Balam',
        'NC_SLD_Blade_704': 'Blade',
        'NC_ORG_Bruce_828': 'Bruce',
        'OC_STT_Omid_325': 'Omid',
        'UL_ORG_Blackmailer_888': 'Blackmailer',
        'ST_GUR_MadCorKalom_1212': 'MadCorKalom',
        // Creature variants with artwork of their very own, more specific than
        // the species sketch.
        'Skeleton Scout': 'SkeletonScout',
        'SkeletonWarrior': 'SkeletonWarrior',
        'ScavengerYoung': 'ScavengerYoung',
        'SH_Zombie_01': 'Zombie',
      };
      expected.forEach((id, name) {
        expect(catalog.artworkFor(id)?.name, name, reason: id);
      });
    });

    test('only a camp code is dropped from the front of an id', () {
      // `Golem_Stone` must not become the man named Stone, nor `Lurker_Homer`
      // the man named Homer: neither `Golem` nor `Lurker` is a camp code.
      expect(catalog.artworkFor('Golem_Stone'), isNull);
      expect(catalog.artworkFor('Lurker_Homer'), isNull);
      expect(catalog.artworkFor('Scavenger_Adult_Rideable_Cohsel'), isNull);
      // And those still find their species by the other route.
      expect(
        catalog.creatureDocumentFor('Golem_Stone'),
        'Document_Glossary_StoneGolem',
      );
      expect(
        catalog.creatureDocumentFor('Lurker_Homer'),
        'Document_Glossary_Lurker',
      );
    });

    test('a character never takes a creature sketch', () {
      // There is a man named Wolf.
      expect(catalog.artworkFor('NC_ORG_Wolf_855')?.kind, 'Creatures');
      expect(
        catalog.artworkFor('NC_ORG_Wolf_855', charactersOnly: true),
        isNull,
      );
    });

    test('a person is never claimed by a species', () {
      for (final id in const [
        'OC_VLK_Digger01_501',
        'SC_NOV_Novice01_1306',
        'OW_OWR_OrcWarriorM01_2001',
        'BC_BAN_Bandit_01',
      ]) {
        expect(catalog.creatureDocumentFor(id), isNull, reason: id);
      }
    });

    test('only creature entries answer the species probe', () {
      // Diego is a glossary entry too, but he is not a species.
      expect(catalog.creatureDocumentFor('Diego'), isNull);
      expect(catalog.isCreatureDocument('Document_Glossary_Biter'), isTrue);
      expect(
        catalog.isCreatureDocument('Document_Glossary_OC_STT_DIEGO'),
        isFalse,
      );
    });

    test('no game path and unknown entries resolve to nothing', () {
      expect(
        catalog.pathFor(
          documentClass: 'Document_Glossary_Biter',
          size: GlossaryImageSize.thumbnail,
          gamePath: null,
        ),
        isNull,
      );
      expect(
        catalog.pathFor(
          documentClass: 'Document_Glossary_NotAThing',
          size: GlossaryImageSize.thumbnail,
          gamePath: r'C:\game',
        ),
        isNull,
      );
    });
  });

  test('a name that could climb out of the artwork folder is refused', () {
    final catalog = GlossaryImageCatalog.fromJsonString('''
{"schema": 1, "images": {
  "Document_Glossary_Ok": {"kind": "Characters", "name": "Diego"},
  "Document_Glossary_Climb": {"kind": "Characters", "name": "../../secret"},
  "Document_Glossary_Kind": {"kind": "../..", "name": "Diego"}
}}
''');
    expect(catalog.byDocumentClass.keys, ['Document_Glossary_Ok']);
  });

  test('a malformed document degrades to an empty catalog', () {
    expect(GlossaryImageCatalog.fromJsonString('[]').byDocumentClass, isEmpty);
    expect(GlossaryImageCatalog.fromJsonString('{}').byDocumentClass, isEmpty);
  });

  group('against a real installation', () {
    final game =
        Platform.environment['GORE_REAL_GAME'] ??
        r'C:\Program Files (x86)\Steam\steamapps\common\Gothic 1 Remake';

    test('every referenced portrait file is actually there', () async {
      final root = Directory(game);
      if (!root.existsSync()) {
        markTestSkipped('no game installation at $game');
        return;
      }
      final catalog = await GlossaryImageCatalog.loadBundled();
      final missing = <String>[];
      for (final documentClass in catalog.byDocumentClass.keys) {
        for (final size in GlossaryImageSize.values) {
          final path = catalog.pathFor(
            documentClass: documentClass,
            size: size,
            gamePath: game,
          );
          if (path == null || !File(path).existsSync()) missing.add('$path');
        }
      }
      // Every indexed artwork file, document or not, must have BOTH cuts: the
      // list shows the thumbnail, the detail view the banner.
      for (final image in catalog.byArtworkName.values) {
        for (final size in GlossaryImageSize.values) {
          final path = catalog.pathForArtwork(
            image: image,
            size: size,
            gamePath: game,
          );
          if (path == null || !File(path).existsSync()) missing.add('$path');
        }
      }
      expect(missing, isEmpty);
      expect(catalog.byArtworkName, isNotEmpty);
    });
  });
}
