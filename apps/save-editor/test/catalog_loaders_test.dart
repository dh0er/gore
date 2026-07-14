import 'package:flutter_test/flutter_test.dart';
import 'package:goresave/features/editor/domain/knowledge_catalog.dart';

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
}
