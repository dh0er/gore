import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/catalog/domain/field_schema.dart';
import 'package:gore_mod/project/revision3_item_catalog.dart';

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();

  test('parses only modeled fields and preserves exact scalar facts', () {
    final catalog = Revision3ItemCatalog.fromJson(
      itemCatalogJson: '''
        [
          {"category":"food","id":"ItFo_Unknown","path":"/Script/Angelscript.ItFo_Unknown"},
          {"category":"food","id":"ItFo_Apple","path":"/Script/Angelscript.ItFo_Apple"}
        ]
      ''',
      modelJson: '''
        {
          "classes": {
            "ItFo_Apple": {
              "fields": [
                {"name":"m_Value","type":"int","default":4,"min":0},
                {"name":"m_MaxStack","type":"int","default":0},
                {"name":"m_Weight","type":"float","default":1.5,"min":0},
                {"name":"m_AutoTarget","type":"bool","default":false},
                {
                  "name":"m_Mode",
                  "type":"enum",
                  "default":5,
                  "enum_values":["Low","High"],
                  "enum_value_ints":[0,5]
                },
                {"name":"m_Note","type":"string","default":"food"}
              ]
            }
          }
        }
      ''',
    );

    expect(catalog.items.map((item) => item.id), [
      'ItFo_Apple',
      'ItFo_Unknown',
    ]);
    final apple = catalog.items.first;
    expect(apple.displayName, 'Apple');
    expect(apple.category, Revision3ItemCategory.food);
    expect(apple.fields.map((field) => field.type), [
      FieldType.int_,
      FieldType.int_,
      FieldType.float_,
      FieldType.bool_,
      FieldType.enum_,
      FieldType.string_,
    ]);
    expect(apple.fields[0].defaultValue, 4);
    expect(apple.fields[0].minValue, 0);
    expect(apple.fields[1].defaultValue, 0);
    expect(apple.fields[1].minValue, isNull);
    expect(apple.fields[2].defaultValue, 1.5);
    expect(apple.fields[2].minValue, 0);
    expect(apple.fields[4].enumValues, ['Low', 'High']);
    expect(apple.fields[4].enumBackingValues, [0, 5]);

    // An absent class remains inspectable but receives no guessed fallback
    // fields from the old editor.
    expect(catalog.items.last.fields, isEmpty);
  });

  test('rejects ambiguous or falsely typed bundled facts', () {
    Revision3ItemCatalog parse(String fields) => Revision3ItemCatalog.fromJson(
      itemCatalogJson: '[{"category":"food","id":"ItFo_Apple"}]',
      modelJson: '{"classes":{"ItFo_Apple":{"fields":[$fields]}}}',
    );

    expect(
      () => parse('{"name":"m_Value","type":"vector","default":4}'),
      throwsFormatException,
    );
    expect(
      () => parse('{"name":"m_Value","type":"int","default":4.0}'),
      throwsFormatException,
    );
    expect(
      () => parse(
        '{"name":"m_Value","type":"int","default":4},'
        '{"name":"m_Value","type":"int","default":5}',
      ),
      throwsFormatException,
    );
  });

  test('rejects duplicate item identities', () {
    expect(
      () => Revision3ItemCatalog.fromJson(
        itemCatalogJson:
            '[{"category":"food","id":"ItFo_Apple"},{"category":"food","id":"ItFo_Apple"}]',
        modelJson: '{"classes":{}}',
      ),
      throwsFormatException,
    );
  });

  test(
    'preserves special catalog category and rejects missing or unknown kinds',
    () {
      final catalog = Revision3ItemCatalog.fromJson(
        itemCatalogJson: '''
        [
          {"category":"special","id":"ItFocusStoneBridgeItem"},
          {"category":"special","id":"ItIg_Worldsplitter"}
        ]
      ''',
        modelJson: '{"classes":{}}',
      );

      expect(
        catalog.items.map((item) => item.category),
        everyElement(Revision3ItemCategory.special),
      );
      expect(
        () => Revision3ItemCatalog.fromJson(
          itemCatalogJson: '[{"id":"ItFocusStoneBridgeItem"}]',
          modelJson: '{"classes":{}}',
        ),
        throwsFormatException,
      );
      expect(
        () => Revision3ItemCatalog.fromJson(
          itemCatalogJson:
              '[{"category":"other","id":"ItFocusStoneBridgeItem"}]',
          modelJson: '{"classes":{}}',
        ),
        throwsFormatException,
      );
    },
  );

  test(
    'shipped assets reopen as a non-empty deterministic reference',
    () async {
      final catalog = await loadRevision3BundledItemCatalog();

      expect(catalog.items, isNotEmpty);
      expect(
        catalog.items.map((item) => item.id).toSet().length,
        catalog.items.length,
      );
      expect(
        catalog.items.every(
          (item) =>
              item.fields.map((field) => field.name).toSet().length ==
              item.fields.length,
        ),
        isTrue,
      );
      final special = catalog.items
          .where(
            (item) =>
                item.id == 'ItFocusStoneBridgeItem' ||
                item.id == 'ItIg_Worldsplitter',
          )
          .toList(growable: false);
      expect(special, hasLength(2));
      expect(
        special.map((item) => item.category),
        everyElement(Revision3ItemCategory.special),
      );
      for (final id in const <String>['ItMi_Oldcoin_01', 'ItMi_Orenugget']) {
        final item = catalog.items.singleWhere((entry) => entry.id == id);
        final maxStack = item.fields.singleWhere(
          (field) => field.name == 'm_MaxStack',
        );
        expect(maxStack.defaultValue, 0, reason: id);
        expect(maxStack.minValue, isNull, reason: id);
        expect(maxStack.maxValue, isNull, reason: id);
      }
    },
  );
}
