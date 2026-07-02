import 'package:flutter_test/flutter_test.dart';
import 'package:goresave/features/editor/domain/knowledge_catalog.dart';

void main() {
  test('KnowledgeCatalog parses, filters empties, sorts by id', () {
    final c = KnowledgeCatalog.fromJsonString(
      '[{"id":"Topic_Diego_209799","category":"topic"},'
      '{"id":"","category":"choice"},'
      '{"id":"ChoiceDiegoGamestart","category":"choice"}]',
    );
    expect(c.entries.map((e) => e.id), ['ChoiceDiegoGamestart', 'Topic_Diego_209799']);
  });
}
