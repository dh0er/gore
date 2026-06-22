import 'package:flutter_test/flutter_test.dart';
import 'package:goresave/features/editor/domain/npc_catalog.dart';
import 'package:goresave/features/editor/domain/knowledge_catalog.dart';

void main() {
  test('NpcCatalog parses, filters empties, sorts by id', () {
    final c = NpcCatalog.fromJsonString(
      '[{"id":"OC_STT_Diego","class":"CharacterDefinition_Human_OC_STT_Diego","category":"human"},'
      '{"id":"","class":"x","category":"human"},'
      '{"id":"Creature_Biter","class":"CharacterDefinition_Creature_Biter","category":"creature"}]',
    );
    expect(c.entries.map((e) => e.id), ['Creature_Biter', 'OC_STT_Diego']);
    expect(c.entries.first.category, 'creature');
  });

  test('KnowledgeCatalog parses, filters empties, sorts by id', () {
    final c = KnowledgeCatalog.fromJsonString(
      '[{"id":"Topic_Diego_209799","category":"topic"},'
      '{"id":"","category":"choice"},'
      '{"id":"ChoiceDiegoGamestart","category":"choice"}]',
    );
    expect(c.entries.map((e) => e.id), ['ChoiceDiegoGamestart', 'Topic_Diego_209799']);
  });
}
