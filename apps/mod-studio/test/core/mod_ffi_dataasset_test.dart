import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/core/core_service.dart';
import 'package:gore_mod/core/mod_ffi.dart';

import '../dataasset/dataasset_test_fixtures.dart';

void main() {
  group('dataAssetFixedInspectV1 request', () {
    test(
      'sends the exact root-only payload and parses immutable facts',
      () async {
        final core = FakeGoreCoreFfiService(
          responses: {
            'dataasset_fixed_inspect_v1': validDataAssetInspectionResponse(),
          },
        );

        final result = await ModFfi(core).dataAssetFixedInspectV1(
          uassetPath: r'C:\Cooked\DA_Test.uasset',
          usmapPath: r'C:\Mappings\G1R.usmap',
        );

        expect(core.calls.single.command, 'dataasset_fixed_inspect_v1');
        expect(core.calls.single.payload, <String, Object?>{
          'uasset_path': r'C:\Cooked\DA_Test.uasset',
          'usmap_path': r'C:\Mappings\G1R.usmap',
        });
        expect(result.status, DataAssetInspectionStatus.walked);
        expect(result.exports.single.objectName, 'DA_Test');
        expect(
          result.exports.single.leaves.single.selector.pathLabel,
          'Health',
        );
        expect(() => result.exports.clear(), throwsUnsupportedError);
        expect(
          () => result.exports.single.leaves.clear(),
          throwsUnsupportedError,
        );
      },
    );

    test('includes export_index only when explicitly selected', () async {
      final core = FakeGoreCoreFfiService(
        responses: {
          'dataasset_fixed_inspect_v1': validDataAssetInspectionResponse(
            exportIndex: 5,
            packageExports: 7,
          ),
        },
      );

      final result = await ModFfi(core).dataAssetFixedInspectV1(
        uassetPath: 'selected.uasset',
        usmapPath: 'schema.usmap',
        exportIndex: 5,
      );

      expect(result.selection.exportIndex, 5);
      expect(core.calls.single.payload, <String, Object?>{
        'uasset_path': 'selected.uasset',
        'usmap_path': 'schema.usmap',
        'export_index': 5,
      });
    });

    test('rejects path and index bounds before core execution', () async {
      final core = FakeGoreCoreFfiService(
        responses: {
          'dataasset_fixed_inspect_v1': validDataAssetInspectionResponse(),
        },
      );
      final ffi = ModFfi(core);

      for (final paths in <(String, String)>[
        ('', 'schema.usmap'),
        ('bad\u0000path.uasset', 'schema.usmap'),
        ('asset.uasset', String.fromCharCode(0xd800)),
        ('x' * (32 * 1024 + 1), 'schema.usmap'),
        ('asset.uasset', '\u20ac' * 10923),
      ]) {
        await expectLater(
          ffi.dataAssetFixedInspectV1(
            uassetPath: paths.$1,
            usmapPath: paths.$2,
          ),
          throwsArgumentError,
        );
      }
      for (final index in <int>[-1, 0x80000000]) {
        await expectLater(
          ffi.dataAssetFixedInspectV1(
            uassetPath: 'asset.uasset',
            usmapPath: 'schema.usmap',
            exportIndex: index,
          ),
          throwsArgumentError,
        );
      }
      expect(core.calls, isEmpty);
    });

    test('accepts the worst-case escaped native envelope boundary', () async {
      final core = FakeGoreCoreFfiService(
        responses: {
          'dataasset_fixed_inspect_v1': validDataAssetInspectionResponse(),
        },
      );
      final path = '\u0001' * (32 * 1024);

      await ModFfi(
        core,
      ).dataAssetFixedInspectV1(uassetPath: path, usmapPath: path);

      expect(core.calls.single.payload['uasset_path'], path);
      expect(core.calls.single.payload['usmap_path'], path);
    });
  });

  group('DataAsset inspection DTO', () {
    test('accepts nested offset-free map selector facts', () {
      final response = validDataAssetInspectionResponse();
      final selector = dataAssetSelector(response);
      selector['path'] = <Object?>[
        <String, Object?>{
          'step': 'property',
          'schema_index': 0,
          'property_name': 'Stats',
          'array_index': 0,
          'array_dimension': 1,
          'declaring_schema_name': '/Script/Test.TestRecord',
          'declaring_module_path': null,
          'property_type': <String, Object?>{
            'type': 'map',
            'key': <String, Object?>{
              'type': 'enum',
              'inner': <String, Object?>{'type': 'byte'},
              'name': 'EStat',
            },
            'value': <String, Object?>{
              'type': 'optional',
              'inner': <String, Object?>{'type': 'int'},
            },
          },
        },
        <String, Object?>{
          'step': 'map',
          'key_type': <String, Object?>{'type': 'byte'},
          'value_type': <String, Object?>{'type': 'int'},
        },
        <String, Object?>{
          'step': 'map_entry_value',
          'key': <String, Object?>{
            'kind': 'byte',
            'byte_length': 1,
            'sha256': 'e' * 64,
          },
        },
      ];

      final result = DataAssetInspection.fromJson(response);

      expect(result.exports.single.leaves.single.selector.path.length, 3);
      expect(
        result.exports.single.leaves.single.selector.path.last.key?.byteLength,
        1,
      );
    });

    test('accepts a large outward string within the 8 MiB response budget', () {
      final response = validDataAssetInspectionResponse();
      final largeObjectName = 'x' * (64 * 1024 + 1);
      dataAssetExport(response)['object_name'] = largeObjectName;
      dataAssetSelector(response)['object_name'] = largeObjectName;

      final result = DataAssetInspection.fromJson(response);

      expect(result.exports.single.objectName.length, largeObjectName.length);
    });

    test('rejects non-exact and cross-bound malicious responses', () {
      final mutations = <void Function(Map<String, Object?>)>[
        (response) => response['extra'] = true,
        (response) => response.remove('binding'),
        (response) => response['status'] = 'runtime_ready',
        (response) =>
            (response['summary'] as Map<String, Object?>)['walked_exports'] = 0,
        (response) =>
            (response['summary'] as Map<String, Object?>)['editable_leaves'] =
                2,
        (response) =>
            (response['input'] as Map<String, Object?>)['uexp_length'] = 63,
        (response) => dataAssetExport(response)['extra'] = true,
        (response) => dataAssetExport(response)['component'] = 'uasset',
        (response) => dataAssetExport(response)['status'] = 'partial',
        (response) => dataAssetExport(response)['property_bytes'] = 49,
        (response) => dataAssetLeaf(response)['index'] = 1,
        (response) => dataAssetLeaf(response)['editable'] = 'yes',
        (response) => dataAssetSelector(response)['usmap_sha256'] = 'f' * 64,
        (response) => dataAssetSelector(response)['expected_hex'] = '01',
        (response) => dataAssetSelector(response)['expected_hex'] = 'A1000000',
        (response) => dataAssetSelector(response)['component'] = 'uasset',
        (response) => (dataAssetSelector(response)['path'] as List).clear(),
        (response) =>
            ((dataAssetSelector(response)['path'] as List).single
                    as Map)['extra'] =
                true,
        (response) =>
            ((dataAssetSelector(response)['path'] as List).single
                    as Map)['array_index'] =
                1,
        (response) =>
            ((dataAssetSelector(response)['path'] as List).single
                as Map)['property_type'] = _nestedWireType(
              65,
            ),
        (response) =>
            dataAssetExport(response)['object_name'] =
                'x' * (8 * 1024 * 1024 + 1),
      ];

      for (final mutate in mutations) {
        final response = cloneDataAssetResponse(
          validDataAssetInspectionResponse(),
        );
        mutate(response);
        expect(
          () => DataAssetInspection.fromJson(response),
          throwsFormatException,
        );
      }
    });

    test('rejects unsupported exports that pretend to carry typed facts', () {
      final response = validDataAssetInspectionResponse();
      response['status'] = 'unsupported';
      final summary = response['summary'] as Map<String, Object?>;
      summary['walked_exports'] = 0;
      summary['editable_leaves'] = 0;
      final export = dataAssetExport(response);
      export['status'] = 'unsupported';
      export['failure'] = <String, Object?>{
        'stage': 'schema',
        'code': 'schema_unsupported',
      };
      export['schema'] = null;
      export['property_bytes'] = null;
      export['native_suffix_bytes'] = null;
      export['leaves'] = <Object?>[];

      expect(
        DataAssetInspection.fromJson(response).exports.single.failure,
        isNotNull,
      );

      export['schema'] = 'invented';
      expect(
        () => DataAssetInspection.fromJson(response),
        throwsFormatException,
      );
    });
  });
}

Map<String, Object?> _nestedWireType(int depth) {
  Map<String, Object?> value = <String, Object?>{'type': 'int'};
  for (var index = 0; index < depth; index++) {
    value = <String, Object?>{'type': 'array', 'inner': value};
  }
  return value;
}
