import 'dart:collection';
import 'dart:convert';
import 'dart:io';

import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/core/core_service.dart';
import 'package:gore_mod/core/mod_ffi.dart';

import '../support/revision3_item_patch_fixture.dart';

const _catalogCommand = 'authoring_store_read_revision3_item_catalog_v1';
const _prepareCommand = 'authoring_store_prepare_revision3_item_patch_v1';
const _projectId = '11111111111111111111111111111111';
const _entityId = '22222222222222222222222222222222';
const _vanillaClass = 'ItFo_Apple';
const _catalogLayer = 'base-game.items.g1r.bundled.v1';
const _signedMax = 0x7fffffffffffffff;

AuthoringWorkingHead _head(String digit) =>
    AuthoringWorkingHead.fromCanonicalJson(
      jsonEncode(<String, Object?>{
        'store_format': 1,
        'snapshot': <String, Object?>{'byte_len': 321, 'sha256': digit * 64},
      }),
    );

Map<String, Object?> _seal(String digit, int bytes) => <String, Object?>{
  'byte_len': bytes,
  'sha256': digit * 64,
};

Map<String, Object?> _target() => <String, Object?>{
  'executable': _seal('a', 171698176),
};

String _projectJson({int revision = 7, Map<String, Object?>? entities}) =>
    jsonEncode(<String, Object?>{
      'format': 2,
      'schema_revision': 3,
      'project_id': _projectId,
      'revision': revision,
      'meta': <String, Object?>{
        'name': 'Managed items',
        'version': '1.0.0',
        'author': 'tests',
      },
      'target': _target(),
      'authoring_locales': <Object?>[],
      'entities': SplayTreeMap<String, Object?>.from(
        entities ?? const <String, Object?>{},
      ),
      'asset_store': <String, Object?>{'assets': <String, Object?>{}},
    });

String _catalogJson({
  String scalarType = 'integer',
  Object? defaultValue = const _Absent(),
  String? sourceSealSha,
}) => jsonEncode(<String, Object?>{
  'catalog_layer': _catalogLayer,
  'catalog_seal': _seal('c', 9000),
  'entries': <Object?>[
    <String, Object?>{
      'category': 'food',
      'fields': <Object?>[
        revision3ItemNumericField(
          name: 'm_Value',
          scalarType: scalarType,
          defaultValue: defaultValue is _Absent ? null : defaultValue,
        ),
        revision3ItemNumericField(
          name: 'm_Weight',
          scalarType: 'float',
          defaultValue: <String, Object?>{'type': 'float', 'data': 0.25},
        ),
      ],
      'runtime_path': '/Script/Angelscript.$_vanillaClass',
      'source_seal': <String, Object?>{
        'byte_len': 500,
        'sha256': sourceSealSha ?? 'd' * 64,
      },
      'vanilla_class': _vanillaClass,
    },
  ],
  'schema_revision': 1,
  'target': _target(),
});

String _arrowCatalogJson() => jsonEncode(<String, Object?>{
  'catalog_layer': _catalogLayer,
  'catalog_seal': _seal('c', 9000),
  'entries': <Object?>[
    <String, Object?>{
      'category': 'ammunition',
      'fields': <Object?>[
        for (final name in const <String>[
          'm_ArcParam',
          'm_Buoyancy',
          'm_Mass',
          'm_Weight',
        ])
          revision3ItemNumericField(name: name, scalarType: 'float'),
      ],
      'runtime_path': '/Script/Angelscript.ItAm_Arrow',
      'source_seal': _seal('d', 500),
      'vanilla_class': 'ItAm_Arrow',
    },
  ],
  'schema_revision': 1,
  'target': _target(),
});

String _floatFixture(String name) =>
    File('test/fixtures/$name').readAsStringSync().trimRight();

Map<String, Object?> _catalogResponse({
  String? catalogJson,
  String catalogAuthority = 'native_embedded_schema_exact_current_project',
}) => <String, Object?>{
  'ok': true,
  'head_json': _head('b').canonicalJson,
  'project_id': _projectId,
  'project_revision': 7,
  'catalog_json': catalogJson ?? _catalogJson(),
  'catalog_seal': _seal('c', 9000),
  'catalog_authority': catalogAuthority,
  'build_status': 'not_evaluated',
  'runtime_status': 'runtime_unqualified',
  'publication_status': 'not_applicable',
};

Map<String, Object?> _itemEntity({
  required int revision,
  required Map<String, Object?> fields,
  String catalogLayer = _catalogLayer,
  String sourceSealDigit = 'd',
}) => <String, Object?>{
  'id': _entityId,
  'display_name': 'Apple',
  'origin': <String, Object?>{
    'type': 'vanilla',
    'generation': _target(),
    'catalog_layer': catalogLayer,
    'canonical_selector': _vanillaClass,
    'source_seal': _seal(sourceSealDigit, 500),
  },
  'revision': revision,
  'payload': <String, Object?>{
    'kind': 'item_patch',
    'data': <String, Object?>{
      'vanilla_class': _vanillaClass,
      'fields': SplayTreeMap<String, Object?>.from(fields),
    },
  },
};

String _createdCandidateJson() => _projectJson(
  revision: 8,
  entities: <String, Object?>{
    _entityId: _itemEntity(
      revision: 0,
      fields: <String, Object?>{
        'm_Value': <String, Object?>{'type': 'integer', 'data': 9},
        'm_Weight': <String, Object?>{'type': 'float', 'data': 0.5},
      },
    ),
  },
);

Map<String, Object?> _prepareResponse({String? projectJson}) =>
    <String, Object?>{
      'ok': true,
      'outcome': 'prepared_unpublished',
      'basis_head_json': _head('b').canonicalJson,
      'head_json': _head('e').canonicalJson,
      'project_json': projectJson ?? _createdCandidateJson(),
      'project_id': _projectId,
      'revision': 8,
      'entity_id': _entityId,
      'entity_revision': 0,
      'change': 'created',
      'catalog_layer': _catalogLayer,
      'vanilla_class': _vanillaClass,
      'source_seal': _seal('d', 500),
      'catalog_seal': _seal('c', 9000),
      'build_status': 'blocked',
      'runtime_status': 'runtime_unqualified',
      'publication_status': 'not_supported',
    };

Map<String, Object?> _clone(Map<String, Object?> value) =>
    (jsonDecode(jsonEncode(value)) as Map).cast<String, Object?>();

Matcher get _throwsMalformed => throwsA(
  isA<ModFfiException>().having(
    (error) => error.code,
    'code',
    ModFfiException.malformedNativeResponseCode,
  ),
);

void main() {
  test('reads a closed native item catalog on the exact head', () async {
    final core = FakeGoreCoreFfiService(
      responses: <String, Map<String, Object?>>{
        _catalogCommand: _catalogResponse(),
      },
    );
    final result = await ModFfi(core).authoringStoreReadRevision3ItemCatalogV1(
      root: r'C:\Mods\Items.goreproj',
      expectedHead: _head('b'),
    );

    expect(core.calls.single.command, _catalogCommand);
    expect(core.calls.single.payload.keys, <String>[
      'expected_head_json',
      'root',
    ]);
    expect(core.calls.single.payload, isNot(contains('game_root')));
    expect(result.projectId, _projectId);
    expect(result.projectRevision, 7);
    expect(result.catalog.entries, hasLength(1));
    final apple = result.catalog.entries.single;
    expect(apple.vanillaClass, _vanillaClass);
    expect(apple.category, AuthoringRevision3ItemCatalogCategory.food);
    expect(apple.fields.map((field) => field.name), <String>[
      'm_Value',
      'm_Weight',
    ]);
    expect(apple.fields.first.defaultValue, isNull);
    expect(apple.fields.last.defaultValue!.floatValue, 0.25);
    expect(
      apple.fields.first.numericDomain,
      AuthoringRevision3ItemNumericDomain.signedInteger32,
    );
    expect(apple.fields.first.minimumValue!.integerValue, -0x80000000);
    expect(apple.fields.first.maximumValue!.integerValue, 0x7fffffff);
    expect(
      apple.fields.last.numericDomain,
      AuthoringRevision3ItemNumericDomain.finiteFloat32,
    );
    expect(apple.fields.last.minimumValue!.floatValue, -3.4028234663852886e38);
    expect(apple.fields.last.maximumValue!.floatValue, 3.4028234663852886e38);
    expect(
      requiredStudioCoreCommands,
      containsAll(<String>[_catalogCommand, _prepareCommand]),
    );
    expect(
      requiredStudioCoreCommands,
      orderedEquals(<String>[...requiredStudioCoreCommands]..sort()),
    );
  });

  test(
    'accepts only serde_json float spelling in the native catalog',
    () async {
      final rustSpelling = _catalogJson().replaceFirst(
        '"data":0.25',
        '"data":1e-6',
      );
      final result =
          await ModFfi(
            FakeGoreCoreFfiService(
              responses: <String, Map<String, Object?>>{
                _catalogCommand: _catalogResponse(catalogJson: rustSpelling),
              },
            ),
          ).authoringStoreReadRevision3ItemCatalogV1(
            root: r'C:\Mods\Items.goreproj',
            expectedHead: _head('b'),
          );
      expect(
        result.catalog.entries.single.fields.last.defaultValue!.floatValue,
        1e-6,
      );

      final dartSpelling = rustSpelling.replaceFirst('1e-6', '0.000001');
      await expectLater(
        ModFfi(
          FakeGoreCoreFfiService(
            responses: <String, Map<String, Object?>>{
              _catalogCommand: _catalogResponse(catalogJson: dartSpelling),
            },
          ),
        ).authoringStoreReadRevision3ItemCatalogV1(
          root: r'C:\Mods\Items.goreproj',
          expectedHead: _head('b'),
        ),
        _throwsMalformed,
      );
    },
  );

  test('catalog identity, schema, seals, and authority fail closed', () async {
    Future<void> reject(Map<String, Object?> response) async {
      final core = FakeGoreCoreFfiService(
        responses: <String, Map<String, Object?>>{_catalogCommand: response},
      );
      await expectLater(
        ModFfi(core).authoringStoreReadRevision3ItemCatalogV1(
          root: r'C:\Mods\Items.goreproj',
          expectedHead: _head('b'),
        ),
        _throwsMalformed,
      );
    }

    final falseSeal = _clone(_catalogResponse());
    falseSeal['catalog_seal'] = _seal('f', 9000);
    await reject(falseSeal);

    final falseType = _catalogResponse(
      catalogJson: _catalogJson(
        scalarType: 'float',
        defaultValue: <String, Object?>{'type': 'integer', 'data': 4},
      ),
    );
    await reject(falseType);

    final falseDomain = (jsonDecode(_catalogJson()) as Map)
        .cast<String, Object?>();
    final falseDomainEntry = (falseDomain['entries'] as List).single as Map;
    final falseDomainField = (falseDomainEntry['fields'] as List).first as Map;
    falseDomainField['numeric_domain'] = 'finite_float32';
    await reject(_catalogResponse(catalogJson: jsonEncode(falseDomain)));

    final falseIntegerMaximum = (jsonDecode(_catalogJson()) as Map)
        .cast<String, Object?>();
    final falseMaximumEntry =
        (falseIntegerMaximum['entries'] as List).single as Map;
    final falseMaximumField =
        (falseMaximumEntry['fields'] as List).first as Map;
    (falseMaximumField['maximum_value'] as Map)['data'] = 0x80000000;
    await reject(
      _catalogResponse(catalogJson: jsonEncode(falseIntegerMaximum)),
    );

    await reject(_catalogResponse(catalogAuthority: 'caller_supplied'));

    final wrongHead = _clone(_catalogResponse());
    wrongHead['head_json'] = _head('f').canonicalJson;
    await reject(wrongHead);

    final oversizedCatalogSeal = (jsonDecode(_catalogJson()) as Map)
        .cast<String, Object?>();
    (oversizedCatalogSeal['catalog_seal'] as Map)['byte_len'] =
        0x8000000000000000;
    await reject(
      _catalogResponse(catalogJson: jsonEncode(oversizedCatalogSeal)),
    );

    final oversizedSourceSeal = (jsonDecode(_catalogJson()) as Map)
        .cast<String, Object?>();
    final oversizedEntry = (oversizedSourceSeal['entries'] as List).single;
    (oversizedEntry['source_seal'] as Map)['byte_len'] = 0x8000000000000000;
    await reject(
      _catalogResponse(catalogJson: jsonEncode(oversizedSourceSeal)),
    );

    final oversizedTargetSeal = (jsonDecode(_catalogJson()) as Map)
        .cast<String, Object?>();
    final oversizedTarget = oversizedTargetSeal['target'] as Map;
    (oversizedTarget['executable'] as Map)['byte_len'] = 0x8000000000000000;
    await reject(
      _catalogResponse(catalogJson: jsonEncode(oversizedTargetSeal)),
    );
  });

  test('read-only catalog accepts a signed-max project revision', () async {
    final response = _catalogResponse();
    response['project_revision'] = _signedMax;
    final result =
        await ModFfi(
          FakeGoreCoreFfiService(
            responses: <String, Map<String, Object?>>{
              _catalogCommand: response,
            },
          ),
        ).authoringStoreReadRevision3ItemCatalogV1(
          root: r'C:\Mods\Items.goreproj',
          expectedHead: _head('b'),
        );
    expect(result.projectRevision, _signedMax);
  });

  test('prepares only the exact canonical ItemPatch create delta', () async {
    final basisProjectJson = _projectJson();
    final catalogCore = FakeGoreCoreFfiService(
      responses: <String, Map<String, Object?>>{
        _catalogCommand: _catalogResponse(),
      },
    );
    final ffi = ModFfi(catalogCore);
    final catalog = await ffi.authoringStoreReadRevision3ItemCatalogV1(
      root: r'C:\Mods\Items.goreproj',
      expectedHead: _head('b'),
    );
    final request = AuthoringRevision3ItemPatchRequestV1.upsertForProject(
      expectedHead: _head('b'),
      currentProjectJson: basisProjectJson,
      catalogRead: catalog,
      catalogEntry: catalog.catalog.entries.single,
      entityId: _entityId,
      expectedEntityRevision: null,
      displayName: 'Apple',
      fields: <String, AuthoringRevision3ItemScalarValue>{
        'm_Weight': AuthoringRevision3ItemScalarValue.float(0.5),
        'm_Value': AuthoringRevision3ItemScalarValue.integer(9),
      },
    );
    final decoded = (jsonDecode(request.canonicalJson) as Map)
        .cast<String, Object?>();
    expect(decoded.keys, <String>[
      'expected_head',
      'expected_project_id',
      'expected_revision',
      'expected_target',
      'mutation',
    ]);
    final mutation = (decoded['mutation']! as Map).cast<String, Object?>();
    expect(mutation.keys, <String>[
      'action',
      'entity_id',
      'display_name',
      'catalog_layer',
      'vanilla_class',
      'source_seal',
      'fields',
    ]);
    expect((mutation['fields']! as Map).keys, <String>['m_Value', 'm_Weight']);

    final prepareCore = FakeGoreCoreFfiService(
      responses: <String, Map<String, Object?>>{
        _prepareCommand: _prepareResponse(),
      },
    );
    final prepared = await ModFfi(prepareCore)
        .authoringStorePrepareRevision3ItemPatchV1(
          root: r'C:\Mods\Items.goreproj',
          currentProjectJson: basisProjectJson,
          request: request,
        );

    expect(prepareCore.calls.single.command, _prepareCommand);
    expect(prepareCore.calls.single.payload.keys, <String>[
      'current_project_json',
      'item_patch_request_json',
      'root',
    ]);
    expect(prepareCore.calls.single.payload, isNot(contains('game_root')));
    expect(prepared.change, AuthoringRevision3ItemPatchChange.created);
    expect(prepared.entityRevision, 0);
    expect(prepared.revision, 8);
    expect(
      prepared.buildStatus,
      AuthoringRevision3ItemPatchBuildStatus.blocked,
    );
    expect(
      prepared.runtimeStatus,
      AuthoringRevision3ItemRuntimeStatus.runtimeUnqualified,
    );
  });

  test('Dart request enforces native i32 and finite-f32 boundaries', () async {
    final basisProjectJson = _projectJson();
    final catalog =
        await ModFfi(
          FakeGoreCoreFfiService(
            responses: <String, Map<String, Object?>>{
              _catalogCommand: _catalogResponse(),
            },
          ),
        ).authoringStoreReadRevision3ItemCatalogV1(
          root: r'C:\Mods\Items.goreproj',
          expectedHead: _head('b'),
        );

    AuthoringRevision3ItemPatchRequestV1 request(
      Map<String, AuthoringRevision3ItemScalarValue> fields,
    ) => AuthoringRevision3ItemPatchRequestV1.upsertForProject(
      expectedHead: _head('b'),
      currentProjectJson: basisProjectJson,
      catalogRead: catalog,
      catalogEntry: catalog.catalog.entries.single,
      entityId: _entityId,
      expectedEntityRevision: null,
      displayName: 'Apple',
      fields: fields,
    );

    expect(
      request(<String, AuthoringRevision3ItemScalarValue>{
        'm_Value': AuthoringRevision3ItemScalarValue.integer(-0x80000000),
        'm_Weight': AuthoringRevision3ItemScalarValue.float(
          3.4028234663852886e38,
        ),
      }).canonicalJson,
      contains('"data":3.4028234663852886e+38'),
    );
    expect(
      () => request(<String, AuthoringRevision3ItemScalarValue>{
        'm_Value': AuthoringRevision3ItemScalarValue.integer(-0x80000001),
      }),
      throwsFormatException,
    );
    expect(
      () => request(<String, AuthoringRevision3ItemScalarValue>{
        'm_Value': AuthoringRevision3ItemScalarValue.integer(0x80000000),
      }),
      throwsFormatException,
    );
    expect(
      () => request(<String, AuthoringRevision3ItemScalarValue>{
        'm_Weight': AuthoringRevision3ItemScalarValue.float(1e39),
      }),
      throwsFormatException,
    );
    expect(
      () => request(<String, AuthoringRevision3ItemScalarValue>{
        'm_Weight': AuthoringRevision3ItemScalarValue.float(-1e39),
      }),
      throwsFormatException,
    );
  });

  test('Dart request matches native float request and project goldens', () {
    final basis = _floatFixture('revision3_item_patch_float_basis_v1.json');
    final requestGolden = _floatFixture(
      'revision3_item_patch_float_request_v1.json',
    );
    final candidateGolden = _floatFixture(
      'revision3_item_patch_float_candidate_v1.json',
    );
    final catalog = AuthoringRevision3ItemCatalogReadResult.fromJson(
      _catalogResponse(catalogJson: _arrowCatalogJson()),
      expectedHead: _head('b'),
    );
    final request = AuthoringRevision3ItemPatchRequestV1.upsertForProject(
      expectedHead: _head('b'),
      currentProjectJson: basis,
      catalogRead: catalog,
      catalogEntry: catalog.catalog.entries.single,
      entityId: _entityId,
      expectedEntityRevision: null,
      displayName: 'Arrow physics',
      fields: <String, AuthoringRevision3ItemScalarValue>{
        'm_ArcParam': AuthoringRevision3ItemScalarValue.float(1e-6),
        'm_Buoyancy': AuthoringRevision3ItemScalarValue.float(1e20),
        'm_Mass': AuthoringRevision3ItemScalarValue.float(-0.0),
        'm_Weight': AuthoringRevision3ItemScalarValue.float(0.25),
      },
    );
    expect(request.canonicalJson, requestGolden);
    expect(request.canonicalJson, contains('"data":1e-6'));
    expect(request.canonicalJson, contains('"data":1e+20'));
    expect(request.canonicalJson, contains('"data":0.0'));
    expect(request.canonicalJson, contains('"data":0.25'));

    final prepared = AuthoringRevision3ItemPatchPreparation.fromJson(
      <String, Object?>{
        'ok': true,
        'outcome': 'prepared_unpublished',
        'basis_head_json': _head('b').canonicalJson,
        'head_json': _head('e').canonicalJson,
        'project_json': candidateGolden,
        'project_id': _projectId,
        'revision': 8,
        'entity_id': _entityId,
        'entity_revision': 0,
        'change': 'created',
        'catalog_layer': _catalogLayer,
        'vanilla_class': 'ItAm_Arrow',
        'source_seal': _seal('d', 500),
        'catalog_seal': _seal('c', 9000),
        'build_status': 'blocked',
        'runtime_status': 'runtime_unqualified',
        'publication_status': 'not_supported',
      },
      currentProjectJson: basis,
      request: request,
    );
    expect(prepared.projectJson, candidateGolden);

    final dartSpelledCandidate = candidateGolden
        .replaceFirst('"data":1e-6', '"data":0.000001')
        .replaceFirst('"data":1e+20', '"data":100000000000000000000.0');
    expect(
      () => AuthoringRevision3ItemPatchPreparation.fromJson(
        <String, Object?>{
          'ok': true,
          'outcome': 'prepared_unpublished',
          'basis_head_json': _head('b').canonicalJson,
          'head_json': _head('e').canonicalJson,
          'project_json': dartSpelledCandidate,
          'project_id': _projectId,
          'revision': 8,
          'entity_id': _entityId,
          'entity_revision': 0,
          'change': 'created',
          'catalog_layer': _catalogLayer,
          'vanilla_class': 'ItAm_Arrow',
          'source_seal': _seal('d', 500),
          'catalog_seal': _seal('c', 9000),
          'build_status': 'blocked',
          'runtime_status': 'runtime_unqualified',
          'publication_status': 'not_supported',
        },
        currentProjectJson: basis,
        request: request,
      ),
      throwsFormatException,
    );
  });

  test('prepare rejects forged candidate delta and authority', () async {
    final basisProjectJson = _projectJson();
    final catalog =
        await ModFfi(
          FakeGoreCoreFfiService(
            responses: <String, Map<String, Object?>>{
              _catalogCommand: _catalogResponse(),
            },
          ),
        ).authoringStoreReadRevision3ItemCatalogV1(
          root: r'C:\Mods\Items.goreproj',
          expectedHead: _head('b'),
        );
    final request = AuthoringRevision3ItemPatchRequestV1.upsertForProject(
      expectedHead: _head('b'),
      currentProjectJson: basisProjectJson,
      catalogRead: catalog,
      catalogEntry: catalog.catalog.entries.single,
      entityId: _entityId,
      expectedEntityRevision: null,
      displayName: 'Apple',
      fields: <String, AuthoringRevision3ItemScalarValue>{
        'm_Value': AuthoringRevision3ItemScalarValue.integer(9),
        'm_Weight': AuthoringRevision3ItemScalarValue.float(0.5),
      },
    );

    Future<void> reject(Map<String, Object?> response) async {
      final core = FakeGoreCoreFfiService(
        responses: <String, Map<String, Object?>>{_prepareCommand: response},
      );
      await expectLater(
        ModFfi(core).authoringStorePrepareRevision3ItemPatchV1(
          root: r'C:\Mods\Items.goreproj',
          currentProjectJson: basisProjectJson,
          request: request,
        ),
        _throwsMalformed,
      );
    }

    final forgedProject = (jsonDecode(_createdCandidateJson()) as Map)
        .cast<String, Object?>();
    (forgedProject['meta']! as Map<String, Object?>)['author'] = 'attacker';
    await reject(_prepareResponse(projectJson: jsonEncode(forgedProject)));

    final authority = _clone(_prepareResponse());
    authority['build_status'] = 'ready';
    await reject(authority);

    final falseCatalog = _clone(_prepareResponse());
    falseCatalog['catalog_seal'] = _seal('f', 9000);
    await reject(falseCatalog);
  });

  test(
    'remove request rejects stored provenance outside current catalog',
    () async {
      final storedEntity = _itemEntity(
        revision: 3,
        catalogLayer: 'base-game.items.g1r.older.v1',
        sourceSealDigit: '9',
        fields: <String, Object?>{
          'm_Value': <String, Object?>{'type': 'integer', 'data': 9},
        },
      );
      final basisProjectJson = _projectJson(
        entities: <String, Object?>{_entityId: storedEntity},
      );
      final catalog =
          await ModFfi(
            FakeGoreCoreFfiService(
              responses: <String, Map<String, Object?>>{
                _catalogCommand: _catalogResponse(),
              },
            ),
          ).authoringStoreReadRevision3ItemCatalogV1(
            root: r'C:\Mods\Items.goreproj',
            expectedHead: _head('b'),
          );
      expect(
        () => AuthoringRevision3ItemPatchRequestV1.removeForProject(
          expectedHead: _head('b'),
          currentProjectJson: basisProjectJson,
          catalogRead: catalog,
          currentCatalogEntry: catalog.catalog.entries.single,
          entityId: _entityId,
          expectedEntityRevision: 3,
        ),
        throwsFormatException,
      );
      expect(
        () => AuthoringRevision3ItemPatchRequestV1.upsertForProject(
          expectedHead: _head('b'),
          currentProjectJson: basisProjectJson,
          catalogRead: catalog,
          catalogEntry: catalog.catalog.entries.single,
          entityId: _entityId,
          expectedEntityRevision: 3,
          displayName: 'Apple',
          fields: <String, AuthoringRevision3ItemScalarValue>{
            'm_Value': AuthoringRevision3ItemScalarValue.integer(10),
          },
        ),
        throwsFormatException,
      );
    },
  );

  test(
    'remove accepts signed-max entity revision without incrementing it',
    () async {
      final storedEntity = _itemEntity(
        revision: _signedMax,
        fields: <String, Object?>{
          'm_Value': <String, Object?>{'type': 'integer', 'data': 9},
        },
      );
      final basisProjectJson = _projectJson(
        entities: <String, Object?>{_entityId: storedEntity},
      );
      final catalog =
          await ModFfi(
            FakeGoreCoreFfiService(
              responses: <String, Map<String, Object?>>{
                _catalogCommand: _catalogResponse(),
              },
            ),
          ).authoringStoreReadRevision3ItemCatalogV1(
            root: r'C:\Mods\Items.goreproj',
            expectedHead: _head('b'),
          );
      final request = AuthoringRevision3ItemPatchRequestV1.removeForProject(
        expectedHead: _head('b'),
        currentProjectJson: basisProjectJson,
        catalogRead: catalog,
        currentCatalogEntry: catalog.catalog.entries.single,
        entityId: _entityId,
        expectedEntityRevision: _signedMax,
      );
      final mutation =
          ((jsonDecode(request.canonicalJson) as Map)['mutation'] as Map)
              .cast<String, Object?>();
      expect(mutation['expected_entity_revision'], _signedMax);
      expect(mutation['expected_catalog_layer'], _catalogLayer);
      expect((mutation['expected_source_seal']! as Map)['sha256'], 'd' * 64);
    },
  );

  test(
    'update may produce signed-max but cannot increment signed-max',
    () async {
      final basisProjectJson = _projectJson(
        entities: <String, Object?>{
          _entityId: _itemEntity(
            revision: _signedMax - 1,
            fields: <String, Object?>{
              'm_Value': <String, Object?>{'type': 'integer', 'data': 9},
            },
          ),
        },
      );
      final catalog =
          await ModFfi(
            FakeGoreCoreFfiService(
              responses: <String, Map<String, Object?>>{
                _catalogCommand: _catalogResponse(),
              },
            ),
          ).authoringStoreReadRevision3ItemCatalogV1(
            root: r'C:\Mods\Items.goreproj',
            expectedHead: _head('b'),
          );
      final request = AuthoringRevision3ItemPatchRequestV1.upsertForProject(
        expectedHead: _head('b'),
        currentProjectJson: basisProjectJson,
        catalogRead: catalog,
        catalogEntry: catalog.catalog.entries.single,
        entityId: _entityId,
        expectedEntityRevision: _signedMax - 1,
        displayName: 'Apple',
        fields: <String, AuthoringRevision3ItemScalarValue>{
          'm_Value': AuthoringRevision3ItemScalarValue.integer(10),
        },
      );
      final candidateJson = _projectJson(
        revision: 8,
        entities: <String, Object?>{
          _entityId: _itemEntity(
            revision: _signedMax,
            fields: <String, Object?>{
              'm_Value': <String, Object?>{'type': 'integer', 'data': 10},
            },
          ),
        },
      );
      final response = _prepareResponse(projectJson: candidateJson);
      response['entity_revision'] = _signedMax;
      response['change'] = 'updated';
      final prepared = AuthoringRevision3ItemPatchPreparation.fromJson(
        response,
        currentProjectJson: basisProjectJson,
        request: request,
      );
      expect(prepared.entityRevision, _signedMax);

      final signedMaxBasis = _projectJson(
        entities: <String, Object?>{
          _entityId: _itemEntity(
            revision: _signedMax,
            fields: <String, Object?>{
              'm_Value': <String, Object?>{'type': 'integer', 'data': 10},
            },
          ),
        },
      );
      expect(
        () => AuthoringRevision3ItemPatchRequestV1.upsertForProject(
          expectedHead: _head('b'),
          currentProjectJson: signedMaxBasis,
          catalogRead: catalog,
          catalogEntry: catalog.catalog.entries.single,
          entityId: _entityId,
          expectedEntityRevision: _signedMax,
          displayName: 'Apple',
          fields: <String, AuthoringRevision3ItemScalarValue>{
            'm_Value': AuthoringRevision3ItemScalarValue.integer(11),
          },
        ),
        throwsFormatException,
      );
    },
  );
}

final class _Absent {
  const _Absent();
}
