import 'package:flutter_test/flutter_test.dart';
import 'package:goresave/features/editor/domain/knowledge_catalog.dart';
import 'package:goresave/features/editor/domain/location_catalog.dart';

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();

  test('KnowledgeCatalog parses, filters empties, sorts by id', () {
    final c = KnowledgeCatalog.fromJsonString(
      '[{"id":"Topic_Diego_209799","category":"topic",'
      '"loc_key":"INFO_DIEGO_OTHERCAMPS_15_00",'
      '"module":"Story.Conversation_Diego"},'
      '{"id":"","category":"choice"},'
      '{"id":"ChoiceDiegoGamestart","category":"choice",'
      '"caption":"[Forced Conversation]"}]',
    );
    expect(c.entries.map((e) => e.id), [
      'ChoiceDiegoGamestart',
      'Topic_Diego_209799',
    ]);
    final topic = c.entryById('topic_diego_209799');
    expect(topic?.locKey, 'INFO_DIEGO_OTHERCAMPS_15_00');
    expect(topic?.module, 'Story.Conversation_Diego');
    expect(
      c.entryById('ChoiceDiegoGamestart')?.caption,
      '[Forced Conversation]',
    );
    expect(c.entryById('missing'), isNull);
  });

  test('bundled catalog carries cache-derived dialog captions', () async {
    final catalog = await KnowledgeCatalog.loadBundled();
    expect(
      catalog.entryById('Topic_Jan_148468')?.locKey,
      'TEXT_WIP_DUZEPXD_20250131_155657_443',
    );
    expect(
      catalog.entryById('Info_Whatslife')?.locKey,
      'Info_Vlk_2_DieLage_15_00',
    );
    expect(
      catalog.entryById('ChoiceAsghan144609')?.caption,
      '[Forced Conversation]',
    );
  });

  test('LocationCatalog parses, drops origin spots, sorts by area then name',
      () {
    final c = LocationCatalog.fromJsonString(
      '{"version":1,'
      '"areas":[{"id":"OC","label":"Old Camp",'
      '"locId":"area_oldcamp_notification"},'
      '{"id":"CV","label":"Cavalorn Valley","locId":null}],'
      '"spots":['
      '{"n":"FP_OC_STAND_YARD_2","x":1.5,"y":-2.5,"z":3.5,"w":90.0,"a":"OC"},'
      '{"n":"FP_CV_ROAM_1","x":10.0,"y":20.0,"z":30.0,"w":-45.5,"a":"CV"},'
      '{"n":"FP_OC_STAND_YARD_1","x":4.0,"y":5.0,"z":6.0,"w":0.0,"a":"OC"},'
      '{"n":"WP_PLACEHOLDER","x":0.0,"y":0.0,"z":0.0,"w":0.0,"a":"OC"},'
      '{"n":"","x":7.0,"y":8.0,"z":9.0,"w":0.0,"a":"OC"}]}',
    );

    expect(c.spots.map((s) => s.name), [
      'FP_CV_ROAM_1',
      'FP_OC_STAND_YARD_1',
      'FP_OC_STAND_YARD_2',
    ]);
    final spot = c.spots.last;
    expect(spot.x, 1.5);
    expect(spot.y, -2.5);
    expect(spot.z, 3.5);
    expect(spot.yaw, 90.0);
    expect(spot.area, 'OC');
    expect(spot.search, 'fp_oc_stand_yard_2');

    expect(c.areas.map((a) => a.id), ['CV', 'OC']);
    expect(c.areaById('OC')?.label, 'Old Camp');
    expect(c.areaById('OC')?.locId, 'area_oldcamp_notification');
    expect(c.areaById('CV')?.locId, isNull);
    expect(c.areaById('ZZ'), isNull);
  });

  test('bundled location catalog carries the spots real saves reference',
      () async {
    final catalog = await LocationCatalog.loadBundled();
    expect(catalog.areas.length, 26);
    expect(catalog.spots.length, greaterThan(10000));

    final byName = {for (final s in catalog.spots) s.name: s};
    expect(byName['FP_FM_GUARD_21']?.area, 'FM');
    expect(byName['FP_NC_GUARD_31']?.area, 'NC');
    expect(byName['IO_SC_ANVIL_2']?.area, 'SC');

    final yard = byName['FP_OC_STAND_YARD_1'];
    expect(yard?.area, 'OC');
    expect(yard?.x, 110520.3);
    expect(catalog.areaById('OC')?.label, 'Old Camp');

    // The Tundra names its spots TA_ while its territory code is HC. Without
    // that alias it has no lexical anchor and its spots scatter into the Old
    // Mine, so assert the join rather than trusting it.
    expect(byName['IO_TA_Chest_01']?.area, 'HC');
    expect(catalog.areaById('HC')?.label, 'Tundra');
    expect(catalog.areas.every((a) =>
        catalog.spots.any((s) => s.area == a.id)), isTrue,
        reason: 'no area may ship without spots');
  });
}
