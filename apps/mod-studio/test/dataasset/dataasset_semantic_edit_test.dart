import 'dart:convert';

import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/dataasset/domain/dataasset_inspection.dart';
import 'package:gore_mod/dataasset/domain/dataasset_semantic_edit.dart';

import 'dataasset_test_fixtures.dart';

void main() {
  test('intent binding matches the frozen Rust cross-language golden', () {
    final intent = _editor()
        .previewScalar(
          extractReceiptPath: r'C:\proof\gore-asset-extract.json',
          expectedTargetPath: '/Game/Data/DA_Test',
          value: '2',
        )
        .intent;

    expect(
      intent.intentBindingSha256,
      '36cdca91d4bf92aca2ea684cad82f6fd998755ea7987da7e35c8aa8cd68294e5',
    );
  });

  test(
    'int32 preview is friendly and emits only typed selector-bound intent',
    () {
      final editor = _editor();
      final preview = editor.previewScalar(
        extractReceiptPath: r'C:\proof\gore-asset-extract.json',
        expectedTargetPath: '/Game/Data/DA_Test',
        value: '42',
      );

      expect(preview.pathLabel, 'Health');
      expect(preview.typeLabel, 'Whole number');
      expect(preview.previousValue, '1');
      expect(preview.replacementValue, '42');
      final fields = preview.intent.toNativeFields();
      expect(
        fields['extract_receipt_path'],
        r'C:\proof\gore-asset-extract.json',
      );
      expect(fields['replacement'], <String, Object>{
        'kind': 'int32',
        'decimal': '42',
      });
      expect(
        jsonEncode(fields['selector']),
        jsonEncode(dataAssetSelector(validDataAssetInspectionResponse())),
      );
      final wire = jsonEncode(fields);
      expect(wire, isNot(contains('replacement_hex')));
      expect(wire, isNot(contains('absolute_offset')));
      expect(wire, isNot(contains('output_path')));
    },
  );

  test(
    'no-op, noncanonical, out-of-range, and absent provenance are rejected',
    () {
      final editor = _editor();
      for (final value in ['1', '01', '2147483648', '-2147483649']) {
        expect(
          () => editor.previewScalar(
            extractReceiptPath: r'C:\proof\gore-asset-extract.json',
            expectedTargetPath: '/Game/Data/DA_Test',
            value: value,
          ),
          throwsA(isA<DataAssetSemanticEditException>()),
          reason: value,
        );
      }
      expect(
        () => editor.previewScalar(
          extractReceiptPath: '',
          expectedTargetPath: '/Game/Data/DA_Test',
          value: '2',
        ),
        throwsA(isA<DataAssetSemanticEditException>()),
      );
    },
  );

  test('receipt path limit is measured in UTF-8 bytes like native wire', () {
    final editor = _editor();
    expect(
      editor
          .previewScalar(
            extractReceiptPath: 'é' * 16384,
            expectedTargetPath: '/Game/Data/DA_Test',
            value: '2',
          )
          .intent
          .extractReceiptPath,
      hasLength(16384),
    );
    expect(
      () => editor.previewScalar(
        extractReceiptPath: 'é' * 16385,
        expectedTargetPath: '/Game/Data/DA_Test',
        value: '2',
      ),
      throwsA(isA<DataAssetSemanticEditException>()),
    );
  });

  test('game asset paths mirror native segment and device-name limits', () {
    final editor = _editor();
    final thirtyTwoSegments = List<String>.filled(32, 'Safe').join('/');
    expect(
      editor
          .previewScalar(
            extractReceiptPath: r'C:\proof\gore-asset-extract.json',
            expectedTargetPath: '/Game/$thirtyTwoSegments',
            value: '2',
          )
          .intent
          .expectedTargetPath,
      '/Game/$thirtyTwoSegments',
    );

    final invalid = <String>[
      '/Game/${List<String>.filled(33, 'TooDeep').join('/')}',
      for (final reserved in const <String>[
        'CON',
        'prn',
        'AUX',
        'nul',
        'COM1',
        'com9',
        'LPT1',
        'lpt9',
      ])
        '/Game/Data/$reserved',
    ];
    for (final target in invalid) {
      expect(
        () => editor.previewScalar(
          extractReceiptPath: r'C:\proof\gore-asset-extract.json',
          expectedTargetPath: target,
          value: '2',
        ),
        throwsA(isA<DataAssetSemanticEditException>()),
        reason: target,
      );
      expect(
        () => DataAssetExtractReceiptSummary.fromJson(
          validDataAssetExtractReceiptSummaryResponse(targetPath: target),
        ),
        throwsFormatException,
        reason: 'receipt DTO must reject $target too',
      );
    }

    for (final allowed in const <String>['COM0', 'COM10', 'LPT0', 'CONSOLE']) {
      expect(
        editor
            .previewScalar(
              extractReceiptPath: r'C:\proof\gore-asset-extract.json',
              expectedTargetPath: '/Game/Data/$allowed',
              value: '2',
            )
            .intent
            .expectedTargetPath,
        '/Game/Data/$allowed',
      );
    }
  });

  test('bool values use a semantic bool and detect exact no-op', () {
    final editor = _editor(kind: 'bool', expectedHex: '00', wireType: 'bool');
    expect(editor.isBoolean, isTrue);
    expect(editor.initialScalarValue, 'Off');
    expect(
      () => editor.previewBool(
        extractReceiptPath: r'C:\proof\gore-asset-extract.json',
        expectedTargetPath: '/Game/Data/DA_Test',
        value: false,
      ),
      throwsA(isA<DataAssetSemanticEditException>()),
    );
    final preview = editor.previewBool(
      extractReceiptPath: r'C:\proof\gore-asset-extract.json',
      expectedTargetPath: '/Game/Data/DA_Test',
      value: true,
    );
    expect(preview.replacementValue, 'On');
    expect(preview.intent.replacement.toJson(), <String, Object>{
      'kind': 'bool',
      'value': true,
    });
  });

  test('unsigned 64-bit values remain exact decimal strings', () {
    final editor = _editor(
      kind: 'uint64',
      expectedHex: '0000000000000000',
      wireType: 'uint64',
    );
    final preview = editor.previewScalar(
      extractReceiptPath: r'C:\proof\gore-asset-extract.json',
      expectedTargetPath: '/Game/Data/DA_Test',
      value: '18446744073709551615',
    );
    expect(preview.replacementValue, '18446744073709551615');
    expect(preview.intent.replacement.toJson(), <String, Object>{
      'kind': 'uint64',
      'decimal': '18446744073709551615',
    });
  });

  test(
    'float32 preview rounds exactly as native wire and rejects underflow',
    () {
      final editor = _editor(
        kind: 'float32',
        expectedHex: '00000000',
        wireType: 'float',
      );
      final preview = editor.previewScalar(
        extractReceiptPath: r'C:\proof\gore-asset-extract.json',
        expectedTargetPath: '/Game/Data/DA_Test',
        value: '1.25',
      );
      expect(preview.replacementValue, '1.25');
      expect(preview.intent.replacement.toJson(), <String, Object>{
        'kind': 'float32',
        'decimal': '1.25',
      });
      expect(
        () => editor.previewScalar(
          extractReceiptPath: r'C:\proof\gore-asset-extract.json',
          expectedTargetPath: '/Game/Data/DA_Test',
          value: '1e-100',
        ),
        throwsA(isA<DataAssetSemanticEditException>()),
      );
    },
  );

  test('linear color preview names four components without raw bytes', () {
    final editor = _editor(
      kind: 'linear_color_f32x4',
      expectedHex: '0000000000000000000000000000803f',
      wireType: 'struct',
      wireTypeName: 'LinearColor',
    );
    expect(editor.componentLabels, ['Red', 'Green', 'Blue', 'Alpha']);
    final preview = editor.previewComponents(
      extractReceiptPath: r'C:\proof\gore-asset-extract.json',
      expectedTargetPath: '/Game/Data/DA_Test',
      values: const ['1', '0.5', '0.25', '1'],
    );
    expect(preview.previousValue, contains('Alpha 1.0'));
    expect(preview.replacementValue, contains('Green 0.5'));
    expect(preview.intent.replacement.toJson(), <String, Object>{
      'kind': 'linear_color_f32x4',
      'r': '1.0',
      'g': '0.5',
      'b': '0.25',
      'a': '1.0',
    });
  });

  test('inspection-only leaves cannot be promoted into semantic edits', () {
    final response = validDataAssetInspectionResponse();
    dataAssetLeaf(response)['editable'] = false;
    (response['summary'] as Map<String, Object?>)['editable_leaves'] = 0;
    final leaf = DataAssetInspection.fromJson(
      response,
    ).exports.single.leaves.single;
    expect(
      () => DataAssetSemanticValueEditor.fromLeaf(leaf),
      throwsA(isA<DataAssetSemanticEditException>()),
    );
  });

  test(
    'selector parse and canonical toJson preserve nested native wire exactly',
    () {
      final response = cloneDataAssetResponse(
        validDataAssetInspectionResponse(),
      );
      final selectorWire = dataAssetSelector(response);
      final path = selectorWire['path'] as List<Object?>;
      final property = (path.single as Map).cast<String, Object?>();
      property['declaring_module_path'] = null;
      property['property_type'] = <String, Object?>{
        'type': 'optional',
        'inner': <String, Object?>{
          'type': 'enum',
          'inner': <String, Object?>{'type': 'uint16'},
          'name': '/Script/Test.EState',
        },
      };
      path.addAll(<Object?>[
        <String, Object?>{
          'step': 'map',
          'key_type': <String, Object?>{
            'type': 'enum',
            'inner': <String, Object?>{'type': 'int'},
            'name': '/Script/Test.EKey',
          },
          'value_type': <String, Object?>{
            'type': 'array',
            'inner': <String, Object?>{
              'type': 'optional',
              'inner': <String, Object?>{'type': 'double'},
            },
          },
        },
        <String, Object?>{
          'step': 'map_entry_value',
          'key': <String, Object?>{
            'kind': 'int32',
            'byte_length': 4,
            'sha256': 'e' * 64,
          },
        },
      ]);

      final parsed = DataAssetInspection.fromJson(
        response,
      ).exports.single.leaves.single.selector;
      expect(jsonEncode(parsed.toJson()), jsonEncode(selectorWire));
    },
  );
}

DataAssetSemanticValueEditor _editor({
  String kind = 'int32',
  String expectedHex = '01000000',
  String wireType = 'int',
  String? wireTypeName,
}) {
  final response = cloneDataAssetResponse(validDataAssetInspectionResponse());
  final selector = dataAssetSelector(response);
  selector['kind'] = kind;
  selector['expected_hex'] = expectedHex;
  final path = selector['path'] as List<Object?>;
  final property = (path.single as Map).cast<String, Object?>();
  property['property_type'] = <String, Object?>{
    'type': wireType,
    'name': ?wireTypeName,
  };
  final inspection = DataAssetInspection.fromJson(response);
  return DataAssetSemanticValueEditor.fromLeaf(
    inspection.exports.single.leaves.single,
  );
}
