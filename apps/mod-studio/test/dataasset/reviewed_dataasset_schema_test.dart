import 'dart:convert';

import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/dataasset/domain/dataasset_inspection.dart';
import 'package:gore_mod/dataasset/domain/reviewed_dataasset_schema.dart';

void main() {
  test('closed footstep schema exposes exact targets and friendly names', () {
    expect(footstepPresetReviewedSchema.id, 'g1r.tracking.footstep-preset');
    expect(footstepPresetReviewedSchema.revision, 1);
    expect(footstepPresetReviewedSchema.friendlyName, 'Footstep preset');
    expect(footstepPresetReviewedSchema.fields.single.id, 'feet_texture_size');
    expect(footstepPresetReviewedSchema.fields.single.componentNames, <String>[
      'Width',
      'Height',
    ]);
    expect(
      footstepPresetReviewedSchema.targets
          .map(
            (target) => <String>[
              target.packagePath,
              target.assetName,
              target.friendlyName,
            ],
          )
          .toList(),
      <List<String>>[
        <String>[
          '/Game/Blueprints/TrackingSystem/FootstepsPresets/DA_HumanFootsteps',
          'DA_HumanFootsteps',
          'Human footsteps',
        ],
        <String>[
          '/Game/Blueprints/TrackingSystem/FootstepsPresets/DA_ScavengerFootsteps',
          'DA_ScavengerFootsteps',
          'Scavenger footsteps',
        ],
        <String>[
          '/Game/Blueprints/TrackingSystem/FootstepsPresets/DA_WolfFootsteps',
          'DA_WolfFootsteps',
          'Wolf footsteps',
        ],
      ],
    );
  });

  test('only complete exact installed package paths match reviewed targets', () {
    const wolf =
        '/Game/Blueprints/TrackingSystem/FootstepsPresets/DA_WolfFootsteps';
    expect(
      footstepPresetReviewedSchema.matchInstalledTarget(wolf)?.assetName,
      'DA_WolfFootsteps',
    );

    for (final nearMatch in <String>[
      'DA_WolfFootsteps',
      '$wolf.uasset',
      '${wolf}Copy',
      wolf.toLowerCase(),
      '/Game/Other/DA_WolfFootsteps',
      '/Game/Blueprints/TrackingSystem/FootstepsPresets/DA_WolfFootsteps.DA_WolfFootsteps',
      '/Game/Blueprints/TrackingSystem/FootstepsPresets/DA_ViperFootsteps',
    ]) {
      expect(
        footstepPresetReviewedSchema.matchInstalledTarget(nearMatch),
        isNull,
        reason: '$nearMatch must use the generic/read-only fallback',
      );
    }
  });

  test('real Wolf and Human vectors match exact reviewed evidence', () {
    const cases = <(String, String, String, List<String>)>[
      (
        '/Game/Blueprints/TrackingSystem/FootstepsPresets/DA_WolfFootsteps',
        'DA_WolfFootsteps',
        '000000000000244000000000000024400000000000000000000000000000f03f',
        <String>['10.0', '10.0', '0.0', '1.0'],
      ),
      (
        '/Game/Blueprints/TrackingSystem/FootstepsPresets/DA_HumanFootsteps',
        'DA_HumanFootsteps',
        '000000000000304000000000000030400000000000000000000000000000f03f',
        <String>['16.0', '16.0', '0.0', '1.0'],
      ),
    ];

    for (final (packagePath, assetName, expectedHex, components) in cases) {
      final inspection = _footstepInspection(
        assetName: assetName,
        expectedHex: expectedHex,
      );
      final match = ReviewedFootstepPresetInspection.tryMatch(
        packagePath: packagePath,
        inspection: inspection,
      );

      expect(match, isNotNull, reason: assetName);
      expect(match!.target.assetName, assetName);
      expect(match.leaf.editable, isTrue);
      expect(match.currentComponents, components);
    }
  });

  test('every reviewed selector fact rejects a valid near-match', () {
    final mutations = <String, void Function(Map<String, Object?>)>{
      'object name': (json) {
        _export(json)['object_name'] = 'DA_WolfFootstepsCopy';
        _selector(json)['object_name'] = 'DA_WolfFootstepsCopy';
      },
      'class path': (json) {
        _export(json)['class_path'] = '/Script/G1R.FootstepTagChild';
        _selector(json)['class_path'] = '/Script/G1R.FootstepTagChild';
      },
      'role': (json) {
        _leaf(json)['editable'] = false;
        _summary(json)['editable_leaves'] = 0;
        _selector(json)['role'] = 'map_key';
      },
      'kind': (json) {
        _selector(json)['kind'] = 'float64';
        _selector(json)['expected_hex'] = '0000000000002440';
      },
      'BoneData schema index': (json) => _pathStep(json, 0)['schema_index'] = 1,
      'BoneData property': (json) =>
          _pathStep(json, 0)['property_name'] = 'BoneDataCopy',
      'BoneData array index': (json) {
        _pathStep(json, 0)['array_dimension'] = 2;
        _pathStep(json, 0)['array_index'] = 1;
      },
      'BoneData array dimension': (json) =>
          _pathStep(json, 0)['array_dimension'] = 2,
      'BoneData declaring schema': (json) =>
          _pathStep(json, 0)['declaring_schema_name'] = 'FootstepTagChild',
      'BoneData module': (json) =>
          _pathStep(json, 0)['declaring_module_path'] = '/Script/G1R2',
      'BoneData type': (json) => _pathStep(json, 0)['property_type'] =
          <String, Object?>{'type': 'struct', 'name': 'BoneFeetDataChild'},
      'nested name': (json) => _pathStep(json, 1)['name'] = 'BoneFeetDataChild',
      'nested schema': (json) =>
          _pathStep(json, 1)['schema_name'] = '/Script/G1R.OtherFeetData',
      'FeetTextureSize schema index': (json) =>
          _pathStep(json, 2)['schema_index'] = 1,
      'FeetTextureSize property': (json) =>
          _pathStep(json, 2)['property_name'] = 'FeetTextureSizeCopy',
      'FeetTextureSize array index': (json) {
        _pathStep(json, 2)['array_dimension'] = 2;
        _pathStep(json, 2)['array_index'] = 1;
      },
      'FeetTextureSize array dimension': (json) =>
          _pathStep(json, 2)['array_dimension'] = 2,
      'FeetTextureSize declaring schema': (json) =>
          _pathStep(json, 2)['declaring_schema_name'] = 'BoneFeetDataChild',
      'FeetTextureSize module': (json) =>
          _pathStep(json, 2)['declaring_module_path'] = '/Script/G1R2',
      'FeetTextureSize type': (json) => _pathStep(json, 2)['property_type'] =
          <String, Object?>{'type': 'struct', 'name': 'Vector4f'},
      'path length': (json) =>
          (_selector(json)['path'] as List<Object?>).add(<String, Object?>{
            'step': 'struct',
            'name': 'Extra',
            'schema_name': '/Script/G1R.Extra',
          }),
    };

    for (final entry in mutations.entries) {
      final json = _footstepInspectionJson();
      entry.value(json);
      final inspection = DataAssetInspection.fromJson(json);
      expect(
        ReviewedFootstepPresetInspection.tryMatch(
          packagePath:
              '/Game/Blueprints/TrackingSystem/FootstepsPresets/DA_WolfFootsteps',
          inspection: inspection,
        ),
        isNull,
        reason: entry.key,
      );
    }

    final invalidComponent = _footstepInspectionJson();
    _export(invalidComponent)['component'] = 'uasset';
    _selector(invalidComponent)['component'] = 'uasset';
    expect(
      () => DataAssetInspection.fromJson(invalidComponent),
      throwsFormatException,
      reason: 'the strict inspection domain excludes non-uexp components',
    );
  });

  test('unknown target, duplicate leaf, and non-finite lane stay generic', () {
    final exactInspection = _footstepInspection();
    expect(
      ReviewedFootstepPresetInspection.tryMatch(
        packagePath:
            '/Game/Blueprints/TrackingSystem/FootstepsPresets/DA_WolfFootstepsCopy',
        inspection: exactInspection,
      ),
      isNull,
    );

    final duplicate = _footstepInspectionJson();
    final leaves = _leaves(duplicate);
    final second = Map<String, Object?>.from(
      jsonDecode(jsonEncode(leaves.single)) as Map,
    )..['index'] = 1;
    second['editable'] = false;
    leaves.add(second);
    final duplicateInspection = DataAssetInspection.fromJson(duplicate);
    expect(
      ReviewedFootstepPresetInspection.tryMatch(
        packagePath:
            '/Game/Blueprints/TrackingSystem/FootstepsPresets/DA_WolfFootsteps',
        inspection: duplicateInspection,
      ),
      isNull,
      reason: 'a non-editable duplicate still makes the semantic field unsafe',
    );

    const finiteLanes = <String>[
      '0000000000002440',
      '0000000000002440',
      '0000000000000000',
      '000000000000f03f',
    ];
    for (var lane = 0; lane < 4; lane++) {
      final lanes = finiteLanes.toList()..[lane] = '000000000000f07f';
      final nonFinite = _footstepInspection(expectedHex: lanes.join());
      expect(
        ReviewedFootstepPresetInspection.tryMatch(
          packagePath:
              '/Game/Blueprints/TrackingSystem/FootstepsPresets/DA_WolfFootsteps',
          inspection: nonFinite,
        ),
        isNull,
        reason: 'non-finite lane $lane',
      );
    }
  });

  test('additional generic editable leaf does not hide reviewed match', () {
    final json = _footstepInspectionJson();
    final leaves = _leaves(json);
    final genericLeaf = Map<String, Object?>.from(
      jsonDecode(jsonEncode(leaves.single)) as Map,
    )..['index'] = 1;
    final genericSelector = (genericLeaf['selector'] as Map)
        .cast<String, Object?>();
    genericSelector['kind'] = 'bool';
    genericSelector['expected_hex'] = '01';
    genericSelector['path'] = <Object?>[
      <String, Object?>{
        'step': 'property',
        'schema_index': 0,
        'property_name': 'InvertX',
        'array_index': 0,
        'array_dimension': 1,
        'declaring_schema_name': 'BoneTrackedData',
        'declaring_module_path': '/Script/G1R',
        'property_type': <String, Object?>{'type': 'bool'},
      },
    ];
    leaves.add(genericLeaf);
    _summary(json)['editable_leaves'] = 2;

    final match = ReviewedFootstepPresetInspection.tryMatch(
      packagePath:
          '/Game/Blueprints/TrackingSystem/FootstepsPresets/DA_WolfFootsteps',
      inspection: DataAssetInspection.fromJson(json),
    );
    expect(match, isNotNull);
    expect(match!.leaf.index, 0);
    expect(match.currentComponents, <String>['10.0', '10.0', '0.0', '1.0']);
  });

  test('request wire is canonical, semantic-only, and round-trips', () {
    final request = ReviewedDataAssetEditRequest.feetTextureSize(
      x: '12.5000',
      y: '8.0',
    );
    const expected =
        '{"format":1,"schema_id":"g1r.tracking.footstep-preset","schema_revision":1,"field_id":"feet_texture_size","value":{"x":"12.5","y":"8"}}';

    expect(request.x, '12.5');
    expect(request.y, '8');
    expect(request.canonicalJson, expected);
    expect(
      ReviewedDataAssetEditRequest.fromCanonicalJson(expected).canonicalJson,
      expected,
    );
    expect(
      ReviewedDataAssetEditRequest.fromJson(
        Map<String, Object?>.from(jsonDecode(expected) as Map),
      ).canonicalJson,
      expected,
    );

    expect(request.toJson().keys, <String>[
      'format',
      'schema_id',
      'schema_revision',
      'field_id',
      'value',
    ]);
    expect((request.toJson()['value'] as Map).keys, <String>['x', 'y']);
    final wire = request.canonicalJson;
    for (final forbidden in <String>[
      'path',
      'target',
      'selector',
      'offset',
      'bytes',
      'hex',
      'binding',
      'sha256',
    ]) {
      expect(wire, isNot(contains(forbidden)), reason: forbidden);
    }
  });

  test('positive decimals are finite, ASCII, and locale-independent', () {
    expect(
      ReviewedDataAssetEditRequest.feetTextureSize(x: '1.5', y: '0.25').x,
      '1.5',
    );

    for (final invalid in <String>[
      '',
      '0',
      '0.0',
      '-1',
      '+1',
      '01',
      '.5',
      '1.',
      '1e2',
      'NaN',
      'Infinity',
      ' 1',
      '1 ',
      '1,5',
      '١.٥',
      '99999999999999999999999999999999999999999999999999999999999999999',
    ]) {
      expect(
        () => ReviewedDataAssetEditRequest.feetTextureSize(x: invalid, y: '1'),
        throwsFormatException,
        reason: invalid,
      );
    }
  });

  test('strict parser rejects drift, extra authority, and noncanonical JSON', () {
    const canonical =
        '{"format":1,"schema_id":"g1r.tracking.footstep-preset","schema_revision":1,"field_id":"feet_texture_size","value":{"x":"1","y":"2"}}';
    final decoded = Map<String, Object?>.from(jsonDecode(canonical) as Map);

    final invalidMaps = <Map<String, Object?>>[
      <String, Object?>{...decoded, 'format': 'wrong'},
      <String, Object?>{
        ...decoded,
        'schema_id': 'g1r.tracking.footstep-preset ',
      },
      <String, Object?>{...decoded, 'schema_revision': 2},
      <String, Object?>{...decoded, 'field_id': 'feet_texture_size_x'},
      <String, Object?>{...decoded, 'target_path': '/Game/Injected'},
      <String, Object?>{
        ...decoded,
        'value': <String, Object>{'x': 1, 'y': '2'},
      },
      <String, Object?>{
        ...decoded,
        'value': <String, Object>{'x': '1', 'y': '2', 'selector': 'injected'},
      },
      <String, Object?>{
        ...decoded,
        'value': <String, Object>{'x': '1.0', 'y': '2'},
      },
    ];
    for (final invalid in invalidMaps) {
      expect(
        () => ReviewedDataAssetEditRequest.fromJson(invalid),
        throwsFormatException,
      );
    }

    for (final invalid in <String>[
      ' $canonical',
      '$canonical\n',
      canonical.replaceFirst('"format":', '"schema_id":"wrong","format":'),
      canonical.replaceFirst('"x":"1"', '"x":"1.0"'),
      canonical.replaceFirst(
        '"format":1,"schema_id":"g1r.tracking.footstep-preset"',
        '"schema_id":"g1r.tracking.footstep-preset","format":1',
      ),
      '[]',
      '{',
    ]) {
      expect(
        () => ReviewedDataAssetEditRequest.fromCanonicalJson(invalid),
        throwsFormatException,
        reason: invalid,
      );
    }
  });
}

DataAssetInspection _footstepInspection({
  String assetName = 'DA_WolfFootsteps',
  String expectedHex =
      '000000000000244000000000000024400000000000000000000000000000f03f',
}) => DataAssetInspection.fromJson(
  _footstepInspectionJson(assetName: assetName, expectedHex: expectedHex),
);

Map<String, Object?> _footstepInspectionJson({
  String assetName = 'DA_WolfFootsteps',
  String expectedHex =
      '000000000000244000000000000024400000000000000000000000000000f03f',
}) => <String, Object?>{
  'ok': true,
  'format': 'gore.dataasset.fixed-inspect.v1',
  'status': 'walked',
  'summary': <String, Object?>{
    'package_exports': 1,
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
    'uexp_length': 86,
    'usmap_length': 256,
  },
  'selection': <String, Object?>{'export_index': null},
  'exports': <Object?>[
    <String, Object?>{
      'index': 0,
      'object_name': assetName,
      'class_path': '/Script/G1R.FootstepTag',
      'component': 'uexp',
      'length': 86,
      'status': 'walked',
      'failure': null,
      'schema': '/Script/G1R.FootstepTag',
      'property_bytes': 82,
      'native_suffix_bytes': 4,
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
            'export_index': 0,
            'object_name': assetName,
            'class_path': '/Script/G1R.FootstepTag',
            'component': 'uexp',
            'export_sha256': 'd' * 64,
            'role': 'property_value',
            'kind': 'vector4_f64x4',
            'path': <Object?>[
              <String, Object?>{
                'step': 'property',
                'schema_index': 0,
                'property_name': 'BoneData',
                'array_index': 0,
                'array_dimension': 1,
                'declaring_schema_name': 'FootstepTag',
                'declaring_module_path': '/Script/G1R',
                'property_type': <String, Object?>{
                  'type': 'struct',
                  'name': 'BoneFeetData',
                },
              },
              <String, Object?>{
                'step': 'struct',
                'name': 'BoneFeetData',
                'schema_name': '/Script/G1R.BoneFeetData',
              },
              <String, Object?>{
                'step': 'property',
                'schema_index': 0,
                'property_name': 'FeetTextureSize',
                'array_index': 0,
                'array_dimension': 1,
                'declaring_schema_name': 'BoneFeetData',
                'declaring_module_path': '/Script/G1R',
                'property_type': <String, Object?>{
                  'type': 'struct',
                  'name': 'Vector4',
                },
              },
            ],
            'expected_hex': expectedHex,
          },
        },
      ],
    },
  ],
};

Map<String, Object?> _summary(Map<String, Object?> json) =>
    (json['summary'] as Map).cast<String, Object?>();

Map<String, Object?> _export(Map<String, Object?> json) =>
    ((json['exports'] as List).single as Map).cast<String, Object?>();

List<Object?> _leaves(Map<String, Object?> json) =>
    _export(json)['leaves'] as List<Object?>;

Map<String, Object?> _leaf(Map<String, Object?> json) =>
    (_leaves(json).single as Map).cast<String, Object?>();

Map<String, Object?> _selector(Map<String, Object?> json) =>
    (_leaf(json)['selector'] as Map).cast<String, Object?>();

Map<String, Object?> _pathStep(Map<String, Object?> json, int index) =>
    ((_selector(json)['path'] as List)[index] as Map).cast<String, Object?>();
