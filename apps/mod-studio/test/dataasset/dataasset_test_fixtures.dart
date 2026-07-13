import 'dart:convert';

Map<String, Object?> validDataAssetInspectionResponse({
  int exportIndex = 0,
  int packageExports = 1,
  String objectName = 'DA_Test',
}) => <String, Object?>{
  'ok': true,
  'format': 'gore.dataasset.fixed-inspect.v1',
  'status': 'walked',
  'summary': <String, Object?>{
    'package_exports': packageExports,
    'reported_exports': 1,
    'walked_exports': 1,
    'editable_leaves': 1,
  },
  'selector_format': <String, Object?>{'format': 1, 'profile': 'g1r_ue5_4'},
  'binding': <String, Object?>{
    'package_seal': <String, Object?>{
      'uasset_sha256': 'a' * 64,
      'uexp_sha256': 'b' * 64,
    },
    'usmap_sha256': 'c' * 64,
  },
  'input': <String, Object?>{
    'uasset_length': 128,
    'uexp_length': 64,
    'usmap_length': 256,
  },
  'selection': <String, Object?>{
    'export_index': packageExports == 1 && exportIndex == 0
        ? null
        : exportIndex,
  },
  'exports': <Object?>[
    <String, Object?>{
      'index': exportIndex,
      'object_name': objectName,
      'class_path': '/Script/Test.TestRecord',
      'component': 'uexp',
      'length': 64,
      'status': 'walked',
      'failure': null,
      'schema': '/Script/Test.TestRecord',
      'property_bytes': 48,
      'native_suffix_bytes': 16,
      'leaves': <Object?>[
        <String, Object?>{
          'index': 0,
          'editable': true,
          'selector': <String, Object?>{
            'format': 1,
            'profile': 'g1r_ue5_4',
            'package_seal': <String, Object?>{
              'uasset_sha256': 'a' * 64,
              'uexp_sha256': 'b' * 64,
            },
            'usmap_sha256': 'c' * 64,
            'export_index': exportIndex,
            'object_name': objectName,
            'class_path': '/Script/Test.TestRecord',
            'component': 'uexp',
            'export_sha256': 'd' * 64,
            'role': 'property_value',
            'kind': 'int32',
            'path': <Object?>[
              <String, Object?>{
                'step': 'property',
                'schema_index': 0,
                'property_name': 'Health',
                'array_index': 0,
                'array_dimension': 1,
                'declaring_schema_name': '/Script/Test.TestRecord',
                'declaring_module_path': '/Script/Test',
                'property_type': <String, Object?>{'type': 'int'},
              },
            ],
            'expected_hex': '01000000',
          },
        },
      ],
    },
  ],
};

Map<String, Object?> cloneDataAssetResponse(Map<String, Object?> value) =>
    (jsonDecode(jsonEncode(value)) as Map).cast<String, Object?>();

Map<String, Object?> dataAssetExport(Map<String, Object?> response) =>
    ((response['exports'] as List).single as Map).cast<String, Object?>();

Map<String, Object?> dataAssetLeaf(Map<String, Object?> response) =>
    ((dataAssetExport(response)['leaves'] as List).single as Map)
        .cast<String, Object?>();

Map<String, Object?> dataAssetSelector(Map<String, Object?> response) =>
    (dataAssetLeaf(response)['selector'] as Map).cast<String, Object?>();
