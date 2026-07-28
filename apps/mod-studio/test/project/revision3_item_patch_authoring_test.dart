import 'dart:async';
import 'dart:convert';

import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/core/mod_ffi.dart';
import 'package:gore_mod/project/revision3_content_index.dart';
import 'package:gore_mod/project/revision3_item_patch_authoring.dart';

import '../support/revision3_item_patch_fixture.dart';

const _projectId = '11111111111111111111111111111111';
const _entityId = '22222222222222222222222222222222';
const _class = 'ItFo_Apple';
const _catalogLayer = 'base-game.items.g1r.bundled.v1';

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

AuthoringRevision3ItemCatalogReadResult _nativeCatalog() {
  final catalogJson = jsonEncode(<String, Object?>{
    'catalog_layer': _catalogLayer,
    'catalog_seal': _seal('c', 9000),
    'entries': <Object?>[
      <String, Object?>{
        'category': 'food',
        'fields': <Object?>[
          revision3ItemNumericField(
            name: 'm_Value',
            scalarType: 'integer',
            defaultValue: <String, Object?>{'type': 'integer', 'data': 4},
          ),
          revision3ItemNumericField(
            name: 'm_Weight',
            scalarType: 'float',
            defaultValue: <String, Object?>{'type': 'float', 'data': 0.25},
          ),
        ],
        'runtime_path': '/Script/Angelscript.$_class',
        'source_seal': _seal('d', 500),
        'vanilla_class': _class,
      },
    ],
    'schema_revision': 1,
    'target': _target(),
  });
  return AuthoringRevision3ItemCatalogReadResult.fromJson(<String, Object?>{
    'ok': true,
    'head_json': _head('b').canonicalJson,
    'project_id': _projectId,
    'project_revision': 7,
    'catalog_json': catalogJson,
    'catalog_seal': _seal('c', 9000),
    'catalog_authority': 'native_embedded_schema_exact_current_project',
    'build_status': 'not_evaluated',
    'runtime_status': 'runtime_unqualified',
    'publication_status': 'not_applicable',
  }, expectedHead: _head('b'));
}

Revision3ContentIndex _content({
  bool patched = false,
  Object? value = 4,
  String valueType = 'integer',
  String catalogLayer = _catalogLayer,
  String sourceDigit = 'd',
}) {
  final entities = <Object?>[];
  final counts = <String, Object?>{};
  if (patched) {
    counts['item_patch'] = 1;
    entities.add(<String, Object?>{
      'id': _entityId,
      'kind': 'item_patch',
      'display_name': 'Apple',
      'revision': 2,
      'origin': <String, Object?>{
        'type': 'vanilla',
        'generation': _target(),
        'catalog_layer': catalogLayer,
        'canonical_selector': _class,
        'source_seal': _seal(sourceDigit, 500),
      },
      'summary': <String, Object?>{
        'kind': 'item_patch',
        'data': <String, Object?>{
          'vanilla_class': _class,
          'field_count': 1,
          'field_types': <String, Object?>{'m_Value': valueType},
          'fields': <String, Object?>{
            'm_Value': <String, Object?>{'type': valueType, 'data': value},
          },
        },
      },
      'references': <Object?>[],
      'asset_references': <Object?>[],
    });
  }
  return Revision3ContentIndex.fromJsonObject(<String, Object?>{
    'schema_revision': 1,
    'project_id': _projectId,
    'project_revision': 7,
    'project_name': 'Managed items',
    'project_version': '1.0.0',
    'project_author': 'tests',
    'target': _target(),
    'authoring_locales': <Object?>[],
    'entity_counts': counts,
    'entities': entities,
    'assets': <Object?>[],
  });
}

Revision3ItemPatchAuthoringService _service({
  required Future<Revision3ContentIndex> Function() loadContent,
  required Revision3ItemPatchTechnicalPublisher publish,
  String projectScopeIdentity = 'test-project-root',
}) => Revision3ItemPatchAuthoringService(
  projectScopeIdentity: projectScopeIdentity,
  projectId: _projectId,
  projectRevision: 7,
  expectedHead: _head('b'),
  loadContentIndex: loadContent,
  loadNativeCatalog: () async => _nativeCatalog(),
  publishTechnicalPlan: publish,
);

void main() {
  test(
    'builds one friendly exact-current choice from native authority',
    () async {
      final service = _service(
        loadContent: () async => _content(),
        publish: (_) async => throw StateError('not used'),
      );

      final catalog = await service.loadCatalog();
      expect(catalog.projectId, _projectId);
      expect(catalog.choices, hasLength(1));
      final apple = catalog.choices.single;
      expect(apple.displayName, 'Apple');
      expect(apple.vanillaClass, _class);
      expect(apple.hasPatch, isFalse);
      expect(apple.canEdit, isTrue);
      expect(apple.fields.map((field) => field.name), <String>[
        'm_Value',
        'm_Weight',
      ]);
      expect(apple.fields.first.defaultValue!.integerValue, 4);
      expect(apple.currentOverrides, isEmpty);
      expect(apple.matches('weight'), isTrue);
    },
  );

  test('rejects a retained choice from another managed project root', () async {
    var publications = 0;
    final rootA = _service(
      projectScopeIdentity: r'C:\mods\root-a',
      loadContent: () async => _content(),
      publish: (_) async => throw StateError('must not publish from root A'),
    );
    final rootB = _service(
      projectScopeIdentity: r'C:\mods\root-b',
      loadContent: () async => _content(),
      publish: (_) async {
        publications++;
        throw StateError('must not publish from root B');
      },
    );
    final rootAChoice = (await rootA.loadCatalog()).choices.single;

    await expectLater(
      rootB.save(
        choice: rootAChoice,
        desiredOverrides: <String, AuthoringRevision3ItemScalarValue>{
          'm_Value': AuthoringRevision3ItemScalarValue.integer(9),
        },
      ),
      throwsA(isA<Revision3ItemPatchStaleCheckpointException>()),
    );
    expect(publications, 0);
  });

  test('creates through a checkpoint-bound hidden technical plan', () async {
    late Revision3ItemPatchTechnicalPlan captured;
    final service = _service(
      loadContent: () async => _content(),
      publish: (plan) async {
        captured = plan;
        return Revision3ItemPatchPublication(
          projectId: _projectId,
          projectRevision: 8,
          entityId: plan.entityId,
          entityRevision: 0,
          change: AuthoringRevision3ItemPatchChange.created,
          vanillaClass: _class,
        );
      },
    );
    final choice = (await service.loadCatalog()).choices.single;

    final result = await service.save(
      choice: choice,
      desiredOverrides: <String, AuthoringRevision3ItemScalarValue>{
        'm_Value': AuthoringRevision3ItemScalarValue.integer(9),
      },
    );

    expect(captured.action, AuthoringRevision3ItemPatchAction.upsert);
    expect(captured.expectedHead.canonicalJson, _head('b').canonicalJson);
    expect(captured.expectedEntityRevision, isNull);
    expect(captured.expectedCatalogSeal.sha256, 'c' * 64);
    expect(captured.expectedSourceSeal.sha256, 'd' * 64);
    expect(captured.fields['m_Value']!.integerValue, 9);
    expect(captured.entityId, hasLength(32));
    expect(result.change, AuthoringRevision3ItemPatchChange.created);
  });

  test(
    'freezes desired overrides before the asynchronous checkpoint refresh',
    () async {
      final refreshStarted = Completer<void>();
      final releaseRefresh = Completer<void>();
      var loads = 0;
      late Revision3ItemPatchTechnicalPlan captured;
      final service = _service(
        loadContent: () async {
          loads++;
          if (loads == 2) {
            refreshStarted.complete();
            await releaseRefresh.future;
          }
          return _content();
        },
        publish: (plan) async {
          captured = plan;
          return Revision3ItemPatchPublication(
            projectId: _projectId,
            projectRevision: 8,
            entityId: plan.entityId,
            entityRevision: 0,
            change: AuthoringRevision3ItemPatchChange.created,
            vanillaClass: _class,
          );
        },
      );
      final choice = (await service.loadCatalog()).choices.single;
      final mutableDesired = <String, AuthoringRevision3ItemScalarValue>{
        'm_Value': AuthoringRevision3ItemScalarValue.integer(9),
      };

      final saving = service.save(
        choice: choice,
        desiredOverrides: mutableDesired,
      );
      await refreshStarted.future;
      mutableDesired['m_Value'] = AuthoringRevision3ItemScalarValue.integer(99);
      mutableDesired['m_Weight'] = AuthoringRevision3ItemScalarValue.float(1.5);
      releaseRefresh.complete();
      await saving;

      expect(captured.fields.keys, <String>['m_Value']);
      expect(captured.fields['m_Value']!.integerValue, 9);
    },
  );

  test('rejects desired values outside the sealed native domain', () async {
    var publications = 0;
    final service = _service(
      loadContent: () async => _content(),
      publish: (_) async {
        publications++;
        throw StateError('must not publish');
      },
    );
    final choice = (await service.loadCatalog()).choices.single;
    expect(
      choice.field('m_Value')!.numericDomain,
      AuthoringRevision3ItemNumericDomain.signedInteger32,
    );
    expect(choice.field('m_Value')!.minimumValue!.integerValue, -0x80000000);
    expect(choice.field('m_Value')!.maximumValue!.integerValue, 0x7fffffff);
    expect(
      choice.field('m_Weight')!.numericDomain,
      AuthoringRevision3ItemNumericDomain.finiteFloat32,
    );

    await expectLater(
      service.save(
        choice: choice,
        desiredOverrides: <String, AuthoringRevision3ItemScalarValue>{
          'm_Value': AuthoringRevision3ItemScalarValue.integer(0x80000000),
        },
      ),
      throwsFormatException,
    );
    await expectLater(
      service.save(
        choice: choice,
        desiredOverrides: <String, AuthoringRevision3ItemScalarValue>{
          'm_Weight': AuthoringRevision3ItemScalarValue.float(1e39),
        },
      ),
      throwsFormatException,
    );
    expect(publications, 0);
  });

  test(
    'updates an exact existing patch and advances its entity revision',
    () async {
      late Revision3ItemPatchTechnicalPlan captured;
      final service = _service(
        loadContent: () async => _content(patched: true),
        publish: (plan) async {
          captured = plan;
          return Revision3ItemPatchPublication(
            projectId: _projectId,
            projectRevision: 8,
            entityId: _entityId,
            entityRevision: 3,
            change: AuthoringRevision3ItemPatchChange.updated,
            vanillaClass: _class,
          );
        },
      );
      final choice = (await service.loadCatalog()).choices.single;
      expect(choice.currentOverrides['m_Value']!.integerValue, 4);

      await service.save(
        choice: choice,
        desiredOverrides: <String, AuthoringRevision3ItemScalarValue>{
          'm_Value': AuthoringRevision3ItemScalarValue.integer(12),
        },
      );

      expect(captured.action, AuthoringRevision3ItemPatchAction.upsert);
      expect(captured.entityId, _entityId);
      expect(captured.expectedEntityRevision, 2);
    },
  );

  test(
    'revert uses only the exact current native catalog provenance',
    () async {
      late Revision3ItemPatchTechnicalPlan captured;
      final service = _service(
        loadContent: () async => _content(patched: true),
        publish: (plan) async {
          captured = plan;
          return Revision3ItemPatchPublication(
            projectId: _projectId,
            projectRevision: 8,
            entityId: _entityId,
            entityRevision: null,
            change: AuthoringRevision3ItemPatchChange.removed,
            vanillaClass: _class,
          );
        },
      );
      final choice = (await service.loadCatalog()).choices.single;
      await service.save(
        choice: choice,
        desiredOverrides: const <String, AuthoringRevision3ItemScalarValue>{},
      );

      expect(captured.action, AuthoringRevision3ItemPatchAction.remove);
      expect(captured.expectedCatalogLayer, _catalogLayer);
      expect(captured.expectedSourceSeal.sha256, 'd' * 64);
      expect(captured.expectedCatalogSeal.sha256, 'c' * 64);
    },
  );

  test(
    'rejects retired provenance before exposing a choice or remove plan',
    () async {
      var publications = 0;
      final service = _service(
        loadContent: () async => _content(
          patched: true,
          catalogLayer: 'base-game.items.g1r.older.v1',
          sourceDigit: '9',
        ),
        publish: (_) async {
          publications++;
          throw StateError('must not publish');
        },
      );

      await expectLater(
        service.loadCatalog(),
        throwsA(isA<Revision3ItemPatchUnsupportedSchemaException>()),
      );
      expect(publications, 0);
    },
  );

  test(
    'rejects stale choices and unsupported stored fields without a remove plan',
    () async {
      var loadCount = 0;
      final staleService = _service(
        loadContent: () async {
          loadCount++;
          return loadCount == 1
              ? _content()
              : _content(patched: true, value: 8);
        },
        publish: (_) async => throw StateError('must not publish'),
      );
      final staleChoice = (await staleService.loadCatalog()).choices.single;
      await expectLater(
        staleService.save(
          choice: staleChoice,
          desiredOverrides: <String, AuthoringRevision3ItemScalarValue>{
            'm_Value': AuthoringRevision3ItemScalarValue.integer(9),
          },
        ),
        throwsA(isA<Revision3ItemPatchStaleCheckpointException>()),
      );

      var publications = 0;
      final unsupportedService = _service(
        loadContent: () async =>
            _content(patched: true, valueType: 'string', value: 'custom'),
        publish: (_) async {
          publications++;
          throw StateError('must not publish');
        },
      );
      await expectLater(
        unsupportedService.loadCatalog(),
        throwsA(isA<Revision3ItemPatchUnsupportedSchemaException>()),
      );

      final outsideDomain = _service(
        loadContent: () async => _content(patched: true, value: 0x80000000),
        publish: (_) async {
          publications++;
          throw StateError('must not publish');
        },
      );
      await expectLater(
        outsideDomain.loadCatalog(),
        throwsA(isA<Revision3ItemPatchUnsupportedSchemaException>()),
      );
      expect(publications, 0);
    },
  );
}
