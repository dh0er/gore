import 'dart:convert';
import 'dart:typed_data';

import 'package:crypto/crypto.dart' as crypto;
import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/core/core_service.dart';
import 'package:gore_mod/core/mod_ffi.dart';

import '../support/revision3_dataasset_fixture.dart';

const _wolfTarget =
    '/Game/Blueprints/TrackingSystem/FootstepsPresets/DA_WolfFootsteps';
const _wolfPackageId = '01e173a19ea374c9';
const _wolfVectorHex =
    '000000000000244000000000000024400000000000000000000000000000f03f';
const _realWolfUassetSha256 =
    '50fe60ade85a393f383bf2f44caee31f6553860c145f3f18462b85a6a9aad2fc';
const _realWolfUexpSha256 =
    '7ae97155c68748470ddef015c17371608561b88c9bba374df3432e8dcf3190fe';
const _realWolfUsmapSha256 =
    '73558c36895cd1b0f0fd1b3cb44305b240f8dbb93730ad03c88d7b8478b7ffca';
const _realWolfExportSha256 =
    '51e80a3a218b04f00f4016c780bd5cea0c7bae2512e40fd89476b72650125e08';

void main() {
  test('reviewed intent accepts Wolf plus a generic editable leaf', () {
    final fixture = _fixture();
    final request = ReviewedDataAssetEditRequest.feetTextureSize(
      x: '12.5',
      y: '8',
    );
    final intent = ReviewedInstalledDataAssetEditIntent.fromInspection(
      snapshot: fixture.snapshot,
      candidate: fixture.candidate,
      inspection: fixture.inspection,
      request: request,
    );

    expect(
      fixture.inspection.inspection.exports.single.leaves.where(
        (leaf) => leaf.editable,
      ),
      hasLength(2),
    );
    expect(intent.snapshot, same(fixture.snapshot));
    expect(intent.candidate, same(fixture.candidate));
    expect(intent.inspection, same(fixture.inspection));
    expect(intent.request, same(request));
    expect(intent.expectedTargetPath, _wolfTarget);
    expect(intent.evidence.target.assetName, 'DA_WolfFootsteps');
    expect(intent.evidence.leaf.index, 0);
    expect(intent.evidence.currentComponents, <String>[
      '10.0',
      '10.0',
      '0.0',
      '1.0',
    ]);
  });

  test('snapshot, candidate, and inspection identity drift is rejected', () {
    final basis = _fixture();
    final request = ReviewedDataAssetEditRequest.feetTextureSize(
      x: '12',
      y: '8',
    );
    ReviewedInstalledDataAssetEditIntent create({
      AuthoringRevision3DataAssetPackageCandidate? candidate,
      AuthoringRevision3InstalledDataAssetInspectionResult? inspection,
    }) => ReviewedInstalledDataAssetEditIntent.fromInspection(
      snapshot: basis.snapshot,
      candidate: candidate ?? basis.candidate,
      inspection: inspection ?? basis.inspection,
      request: request,
    );

    final foreignCandidate = _fixture().candidate;
    final driftedInspections = <String, _ReviewedFixture>{
      'head': _fixture(headDigit: '2'),
      'project': _fixture(projectId: '2' * 32),
      'revision': _fixture(projectRevision: 8),
      'package id': _fixture(packageId: '11e173a19ea374c9'),
      'package index seal': _fixture(physicalChunkCount: 2),
      'source snapshot seal': _fixture(sourceSnapshotDigit: 'd'),
    };

    expect(
      () => create(candidate: foreignCandidate),
      throwsArgumentError,
      reason: 'value-equal foreign candidate must not acquire authority',
    );
    for (final entry in driftedInspections.entries) {
      expect(
        () => create(inspection: entry.value.inspection),
        throwsArgumentError,
        reason: entry.key,
      );
    }
  });

  test('native wire is the exact narrow reviewed authority surface', () {
    final fixture = _fixture();
    final request = ReviewedDataAssetEditRequest.feetTextureSize(
      x: '12.5',
      y: '8',
    );
    final intent = ReviewedInstalledDataAssetEditIntent.fromInspection(
      snapshot: fixture.snapshot,
      candidate: fixture.candidate,
      inspection: fixture.inspection,
      request: request,
    );

    final fields = intent.toNativeFields();
    expect(fields, <String, Object?>{
      'candidate_ordinal': 0,
      'expected_package_index_seal': <String, Object?>{
        'byte_len': fixture.snapshot.packageIndexSeal.byteLength,
        'sha256': fixture.snapshot.packageIndexSeal.sha256,
      },
      'expected_source_snapshot_seal': <String, Object?>{
        'byte_len': fixture.snapshot.sourceSnapshotSeal.byteLength,
        'sha256': fixture.snapshot.sourceSnapshotSeal.sha256,
      },
      'reviewed_edit': request.toJson(),
    });
    expect(fields.keys, <String>[
      'candidate_ordinal',
      'expected_package_index_seal',
      'expected_source_snapshot_seal',
      'reviewed_edit',
    ]);

    final wire = jsonEncode(fields);
    for (final forbidden in <String>[
      'target_path',
      'package_id',
      'usmap',
      'inspection',
      'binding',
      'selector',
      'replacement',
      'expected_hex',
      'uasset',
      'uexp',
      'bytes',
    ]) {
      expect(wire, isNot(contains(forbidden)), reason: forbidden);
    }
  });

  test(
    'reviewed FFI sends semantic-only authority and rejects response drift',
    () async {
      final fixture = _fixture();
      final request = ReviewedDataAssetEditRequest.feetTextureSize(
        x: '12.5',
        y: '8',
      );
      final intent = ReviewedInstalledDataAssetEditIntent.fromInspection(
        snapshot: fixture.snapshot,
        candidate: fixture.candidate,
        inspection: fixture.inspection,
        request: request,
      );
      final response = _reviewedPreparationResponse(intent);
      final core = FakeGoreCoreFfiService(
        responses: <String, Map<String, Object?>>{
          'authoring_store_prepare_revision3_reviewed_installed_dataasset_edit_v1':
              response,
        },
      );

      final prepared = await ModFfi(core)
          .authoringStorePrepareRevision3ReviewedInstalledDataAssetEditV1(
            root: r'C:\Projects\ReviewedFootsteps.goreproj',
            gameRoot: r'C:\Games\Gothic 1 Remake',
            expectedHead: fixture.snapshot.head,
            intent: intent,
          );

      expect(core.calls.single.payload, <String, Object?>{
        'expected_head_json': fixture.snapshot.head.canonicalJson,
        ...intent.toNativeFields(),
        'game_root': r'C:\Games\Gothic 1 Remake',
        'root': r'C:\Projects\ReviewedFootsteps.goreproj',
      });
      final payloadWire = jsonEncode(core.calls.single.payload);
      for (final forbidden in <String>[
        'target_path',
        'package_id',
        'usmap',
        'inspection',
        'binding',
        'selector',
        'replacement',
        'expected_hex',
        'uasset',
        'uexp',
        'bytes',
      ]) {
        expect(payloadWire, isNot(contains(forbidden)), reason: forbidden);
      }
      expect(
        prepared.publicationStatus,
        AuthoringRevision3DataAssetNativePublicationStatus.notSupported,
      );
      expect(prepared.installedSource?.candidateOrdinal, 0);
      expect(
        prepared.reviewedEdit?.targetId,
        'g1r:dataasset:footstep-preset:wolf',
      );
      expect(prepared.reviewedEdit?.before, (x: 10.0, y: 10.0, z: 0.0, w: 1.0));
      expect(prepared.reviewedEdit?.after, (x: 12.5, y: 8.0, z: 0.0, w: 1.0));
      expect(
        prepared.reviewedEdit?.intentBindingSha256,
        intent.expectedReviewedIntentBindingSha256,
      );

      final mutations = <void Function(Map<String, Object?>)>[
        (value) =>
            (value['reviewed_edit']! as Map<String, Object?>)['target_id'] =
                'g1r:dataasset:footstep-preset:human',
        (value) =>
            (value['reviewed_edit']! as Map<String, Object?>)['schema_id'] =
                'g1r.tracking.forged',
        (value) =>
            (value['reviewed_after']! as Map<String, Object?>)['x'] = '13',
        (value) =>
            (value['reviewed_after']! as Map<String, Object?>)['z'] = '1',
        (value) =>
            (value['reviewed_after']! as Map<String, Object?>)['w'] = '2',
        (value) =>
            (value['reviewed_before']! as Map<String, Object?>)['z'] = '-0',
        (value) =>
            (value['reviewed_after']! as Map<String, Object?>)['x'] = '12.50',
        (value) => value['reviewed_intent_binding_sha256'] = 'b' * 63,
        (value) {
          final expected = intent.expectedReviewedIntentBindingSha256;
          value['reviewed_intent_binding_sha256'] =
              '${expected.startsWith('0') ? '1' : '0'}${expected.substring(1)}';
        },
        (value) => value['intent_binding_sha256'] = 'f' * 64,
        (value) =>
            (value['installed_source']!
                    as Map<String, Object?>)['candidate_ordinal'] =
                1,
      ];
      for (final mutate in mutations) {
        final malformed = (jsonDecode(jsonEncode(response)) as Map)
            .cast<String, Object?>();
        mutate(malformed);
        final malformedCore = FakeGoreCoreFfiService(
          responses: <String, Map<String, Object?>>{
            'authoring_store_prepare_revision3_reviewed_installed_dataasset_edit_v1':
                malformed,
          },
        );
        await expectLater(
          ModFfi(
            malformedCore,
          ).authoringStorePrepareRevision3ReviewedInstalledDataAssetEditV1(
            root: r'C:\Projects\ReviewedFootsteps.goreproj',
            gameRoot: r'C:\Games\Gothic 1 Remake',
            expectedHead: fixture.snapshot.head,
            intent: intent,
          ),
          throwsA(
            isA<ModFfiException>().having(
              (error) => error.code,
              'code',
              ModFfiException.malformedNativeResponseCode,
            ),
          ),
        );
      }
    },
  );

  test(
    'reviewed FFI rejects a self-consistent alternative native stage',
    () async {
      final fixture = _fixture();
      final request = ReviewedDataAssetEditRequest.feetTextureSize(
        x: '12.5',
        y: '8',
      );
      final intent = ReviewedInstalledDataAssetEditIntent.fromInspection(
        snapshot: fixture.snapshot,
        candidate: fixture.candidate,
        inspection: fixture.inspection,
        request: request,
      );
      final expected = _reviewedPreparationResponse(intent);
      final alternative = _reviewedPreparationResponse(
        intent,
        stagedComponents: const <String>['13', '8', '0', '1'],
      );
      final expectedStage = (expected['stage']! as Map).cast<String, Object?>();
      final alternativeStage = (alternative['stage']! as Map)
          .cast<String, Object?>();
      final expectedManifest = (expectedStage['manifest']! as Map)
          .cast<String, Object?>();
      final alternativeManifest = (alternativeStage['manifest']! as Map)
          .cast<String, Object?>();

      expect(
        alternativeManifest['replacement_hex'],
        isNot(expectedManifest['replacement_hex']),
      );
      expect(
        alternativeStage['manifest_asset'],
        isNot(expectedStage['manifest_asset']),
      );
      expect(alternative['project_json'], isNot(expected['project_json']));
      expect(alternative['head_json'], isNot(expected['head_json']));
      expect(
        alternative['intent_binding_sha256'],
        isNot(expected['intent_binding_sha256']),
      );
      expect(alternative['reviewed_after'], expected['reviewed_after']);

      final core = FakeGoreCoreFfiService(
        responses: <String, Map<String, Object?>>{
          'authoring_store_prepare_revision3_reviewed_installed_dataasset_edit_v1':
              alternative,
        },
      );
      await expectLater(
        ModFfi(
          core,
        ).authoringStorePrepareRevision3ReviewedInstalledDataAssetEditV1(
          root: r'C:\Projects\ReviewedFootsteps.goreproj',
          gameRoot: r'C:\Games\Gothic 1 Remake',
          expectedHead: fixture.snapshot.head,
          intent: intent,
        ),
        throwsA(
          isA<ModFfiException>().having(
            (error) => error.code,
            'code',
            ModFfiException.malformedNativeResponseCode,
          ),
        ),
      );
    },
  );

  test('reviewed intent binding matches the frozen Rust Wolf golden', () {
    final fixture = _fixture(realWolfSelector: true);
    final intent = ReviewedInstalledDataAssetEditIntent.fromInspection(
      snapshot: fixture.snapshot,
      candidate: fixture.candidate,
      inspection: fixture.inspection,
      request: ReviewedDataAssetEditRequest.feetTextureSize(x: '11', y: '12'),
    );

    expect(
      intent.expectedReviewedIntentBindingSha256,
      '4a0fcacfecbf87011bd435388fef54b05300f5e634809630104d0f9c0da705e4',
    );
  });

  test(
    'proof binding changes with exact evidence absent from request wire',
    () {
      final before = _fixture(uexpDigit: '8');
      final after = _fixture(uexpDigit: 'a');
      final request = ReviewedDataAssetEditRequest.feetTextureSize(
        x: '12.5',
        y: '8',
      );
      ReviewedInstalledDataAssetEditIntent intent(_ReviewedFixture fixture) =>
          ReviewedInstalledDataAssetEditIntent.fromInspection(
            snapshot: fixture.snapshot,
            candidate: fixture.candidate,
            inspection: fixture.inspection,
            request: request,
          );

      final beforeIntent = intent(before);
      final afterIntent = intent(after);
      expect(beforeIntent.toNativeFields(), afterIntent.toNativeFields());
      expect(
        beforeIntent.installedProofBindingSha256,
        isNot(afterIntent.installedProofBindingSha256),
      );
      expect(beforeIntent.installedProofBindingSha256, hasLength(64));
      expect(afterIntent.installedProofBindingSha256, hasLength(64));
    },
  );
}

Map<String, Object?> _reviewedPreparationResponse(
  ReviewedInstalledDataAssetEditIntent intent, {
  List<String>? stagedComponents,
}) {
  final effectiveComponents =
      stagedComponents ??
      <String>[
        intent.request.x,
        intent.request.y,
        intent.evidence.currentZ,
        intent.evidence.currentW,
      ];
  final replacementHex = _float64VectorHex(effectiveComponents);
  final basisProjectJson = jsonEncode(<String, Object?>{
    'format': 2,
    'schema_revision': 3,
    'project_id': intent.snapshot.projectId,
    'revision': intent.snapshot.projectRevision,
    'meta': <String, Object?>{
      'name': 'Reviewed footstep fixture',
      'version': '1.0.0',
      'author': 'tests',
    },
    'target': <String, Object?>{
      'executable': _sealFrom(intent.snapshot.targetExecutableSeal),
    },
    'authoring_locales': <Object?>[],
    'entities': <String, Object?>{},
    'asset_store': <String, Object?>{'assets': <String, Object?>{}},
  });
  final stageFixture = Revision3DataAssetFixture.fromBasis(
    basisHead: intent.snapshot.head,
    basisProjectJson: basisProjectJson,
    targetPath: intent.expectedTargetPath,
    selector: intent.evidence.leaf.selector.toJson(),
    replacementHex: replacementHex,
  );
  final semanticChange = DataAssetSemanticValueEditor.fromLeaf(
    intent.evidence.leaf,
  ).changeComponents(values: effectiveComponents);
  final stageBinding = semanticChange.replacement.intentBindingSha256For(
    expectedTargetPath: intent.expectedTargetPath,
    selector: intent.evidence.leaf.selector,
  );
  final inspected = intent.inspection.inspection;
  return stageFixture.prepareResponse()
    ..['intent_binding_sha256'] = stageBinding
    ..['installed_proof_binding_sha256'] = intent.installedProofBindingSha256
    ..['installed_source'] = <String, Object?>{
      'candidate_ordinal': intent.candidate.ordinal,
      'format': 'gore.authoring.revision3-installed-dataasset-source.v1',
      'inspection_binding': <String, Object?>{
        'uasset': <String, Object?>{
          'byte_len': inspected.input.uassetLength,
          'sha256': inspected.binding.packageSeal.uassetSha256,
        },
        'uexp': <String, Object?>{
          'byte_len': inspected.input.uexpLength,
          'sha256': inspected.binding.packageSeal.uexpSha256,
        },
        'usmap': <String, Object?>{
          'byte_len': inspected.input.usmapLength,
          'sha256': inspected.binding.usmapSha256,
        },
      },
      'package_index_seal': _sealFrom(intent.snapshot.packageIndexSeal),
      'source_snapshot_seal': _sealFrom(intent.snapshot.sourceSnapshotSeal),
      'usmap_content_seal': _sealFrom(intent.inspection.usmapContentSeal),
      'usmap_inventory_seal': _sealFrom(intent.inspection.usmapInventorySeal),
    }
    ..['reviewed_edit'] = <String, Object?>{
      'format': reviewedDataAssetEditRequestFormat,
      'schema_id': footstepPresetSchemaId,
      'schema_revision': footstepPresetSchemaRevision,
      'field_id': feetTextureSizeFieldId,
      'target_id': 'g1r:dataasset:footstep-preset:wolf',
    }
    ..['reviewed_before'] = <String, Object?>{
      'x': '10',
      'y': '10',
      'z': '0',
      'w': '1',
    }
    ..['reviewed_after'] = <String, Object?>{
      'x': intent.request.x,
      'y': intent.request.y,
      'z': '0',
      'w': '1',
    }
    ..['reviewed_intent_binding_sha256'] =
        intent.expectedReviewedIntentBindingSha256;
}

String _float64VectorHex(List<String> components) {
  final bytes = ByteData(32);
  for (var index = 0; index < components.length; index++) {
    bytes.setFloat64(index * 8, double.parse(components[index]), Endian.little);
  }
  return bytes.buffer.asUint8List().map((byte) {
    return byte.toRadixString(16).padLeft(2, '0');
  }).join();
}

typedef _ReviewedFixture = ({
  AuthoringRevision3DataAssetPackageIndexResult snapshot,
  AuthoringRevision3DataAssetPackageCandidate candidate,
  AuthoringRevision3InstalledDataAssetInspectionResult inspection,
});

_ReviewedFixture _fixture({
  String headDigit = '1',
  String projectId = '11111111111111111111111111111111',
  int projectRevision = 7,
  String packageId = _wolfPackageId,
  int physicalChunkCount = 1,
  String sourceSnapshotDigit = 'c',
  String uexpDigit = '8',
  bool realWolfSelector = false,
}) {
  final head = AuthoringWorkingHead.fromCanonicalJson(
    jsonEncode(<String, Object?>{
      'store_format': 1,
      'snapshot': <String, Object?>{'byte_len': 99, 'sha256': headDigit * 64},
    }),
  );
  final snapshot = _snapshot(
    head: head,
    projectId: projectId,
    projectRevision: projectRevision,
    packageId: packageId,
    physicalChunkCount: physicalChunkCount,
    sourceSnapshotDigit: sourceSnapshotDigit,
  );
  final candidate = snapshot.index.candidates.single;
  return (
    snapshot: snapshot,
    candidate: candidate,
    inspection: _inspection(
      snapshot: snapshot,
      candidate: candidate,
      uexpDigit: uexpDigit,
      realWolfSelector: realWolfSelector,
    ),
  );
}

AuthoringRevision3DataAssetPackageIndexResult _snapshot({
  required AuthoringWorkingHead head,
  required String projectId,
  required int projectRevision,
  required String packageId,
  required int physicalChunkCount,
  required String sourceSnapshotDigit,
}) {
  final indexJson = jsonEncode(<String, Object?>{
    'status': 'complete_index',
    'physical_chunk_count': physicalChunkCount,
    'winning_export_bundle_count': 1,
    'directory_indexed_export_bundle_count': 1,
    'out_of_scope_export_bundle_count': 0,
    'candidates': <Object?>[
      <String, Object?>{
        'target_path': _wolfTarget,
        'package_id_hex': packageId,
      },
    ],
    'partial_reasons': <Object?>[],
  });
  final indexBytes = utf8.encode(indexJson);
  return AuthoringRevision3DataAssetPackageIndexResult.fromJson(
    <String, Object?>{
      'authority_status': 'not_granted',
      'build_status': 'not_evaluated',
      'candidate_count': 1,
      'content_status': 'metadata_candidates_only',
      'export_bundle_payload_status': 'not_read',
      'head_json': head.canonicalJson,
      'mount_inventory_entry_count': 2,
      'mount_inventory_seal': _seal(80, 'b'),
      'mutation_status': 'not_supported',
      'ok': true,
      'outcome': 'audit_only',
      'package_index_json': indexJson,
      'package_index_seal': <String, Object?>{
        'byte_len': indexBytes.length,
        'sha256': crypto.sha256.convert(indexBytes).toString(),
      },
      'package_index_status': 'complete_index',
      'project_id': projectId,
      'project_revision': projectRevision,
      'publication_status': 'not_supported',
      'runtime_status': 'runtime_unqualified',
      'scope': 'installed_dataasset_package_candidates_only',
      'source_snapshot_seal': _seal(120, sourceSnapshotDigit),
      'target_executable_seal': _seal(171698176, 'd'),
    },
    expectedHead: head,
  );
}

AuthoringRevision3InstalledDataAssetInspectionResult _inspection({
  required AuthoringRevision3DataAssetPackageIndexResult snapshot,
  required AuthoringRevision3DataAssetPackageCandidate candidate,
  required String uexpDigit,
  required bool realWolfSelector,
}) => AuthoringRevision3InstalledDataAssetInspectionResult.fromJson(
  <String, Object?>{
    'authority_status': 'not_granted',
    'build_status': 'not_evaluated',
    'candidate_ordinal': candidate.ordinal,
    'head_json': snapshot.head.canonicalJson,
    'inspection': _inspectionPayload(
      uexpDigit: uexpDigit,
      realWolfSelector: realWolfSelector,
    ),
    'mutation_status': 'not_supported',
    'ok': true,
    'outcome': 'inspection_only',
    'package_id_hex': candidate.packageIdHex,
    'package_index_seal': _sealFrom(snapshot.packageIndexSeal),
    'project_id': snapshot.projectId,
    'project_revision': snapshot.projectRevision,
    'publication_status': 'not_supported',
    'runtime_status': 'runtime_unqualified',
    'scope': 'selected_installed_dataasset_fixed_leaf_inspection_only',
    'source_snapshot_seal': _sealFrom(snapshot.sourceSnapshotSeal),
    'target_path': candidate.targetPath,
    'usmap_content_seal': realWolfSelector
        ? <String, Object?>{'byte_len': 256, 'sha256': _realWolfUsmapSha256}
        : _seal(256, '3'),
    'usmap_inventory_seal': _seal(96, 'e'),
  },
  expectedSnapshot: snapshot,
  requestedOrdinal: candidate.ordinal,
);

Map<String, Object?> _inspectionPayload({
  required String uexpDigit,
  required bool realWolfSelector,
}) => <String, Object?>{
  'ok': true,
  'format': 'gore.dataasset.fixed-inspect.v1',
  'status': 'walked',
  'summary': <String, Object?>{
    'package_exports': 1,
    'reported_exports': 1,
    'walked_exports': 1,
    'editable_leaves': 2,
  },
  'selector_format': <String, Object?>{'format': 1, 'profile': 'g1r_ue5_4'},
  'binding': <String, Object?>{
    'package_seal': <String, Object?>{
      'uasset_sha256': realWolfSelector ? _realWolfUassetSha256 : '7' * 64,
      'uexp_sha256': realWolfSelector ? _realWolfUexpSha256 : uexpDigit * 64,
    },
    'usmap_sha256': realWolfSelector ? _realWolfUsmapSha256 : '3' * 64,
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
      'object_name': 'DA_WolfFootsteps',
      'class_path': '/Script/G1R.FootstepTag',
      'component': 'uexp',
      'length': 86,
      'status': 'walked',
      'failure': null,
      'schema': '/Script/G1R.FootstepTag',
      'property_bytes': 82,
      'native_suffix_bytes': 4,
      'leaves': <Object?>[
        _leaf(
          index: 0,
          uexpDigit: uexpDigit,
          kind: 'vector4_f64x4',
          path: _feetTextureSizePath(),
          expectedHex: _wolfVectorHex,
          realWolfSelector: realWolfSelector,
        ),
        _leaf(
          index: 1,
          uexpDigit: uexpDigit,
          kind: 'bool',
          path: <Object?>[
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
          ],
          expectedHex: '01',
          realWolfSelector: realWolfSelector,
        ),
      ],
    },
  ],
};

Map<String, Object?> _leaf({
  required int index,
  required String uexpDigit,
  required String kind,
  required List<Object?> path,
  required String expectedHex,
  required bool realWolfSelector,
}) => <String, Object?>{
  'index': index,
  'editable': true,
  'selector': <String, Object?>{
    'format': 1,
    'profile': 'g1r_ue5_4',
    'package_seal': <String, Object?>{
      'uasset_sha256': realWolfSelector ? _realWolfUassetSha256 : '7' * 64,
      'uexp_sha256': realWolfSelector ? _realWolfUexpSha256 : uexpDigit * 64,
    },
    'usmap_sha256': realWolfSelector ? _realWolfUsmapSha256 : '3' * 64,
    'export_index': 0,
    'object_name': 'DA_WolfFootsteps',
    'class_path': '/Script/G1R.FootstepTag',
    'component': 'uexp',
    'export_sha256': realWolfSelector ? _realWolfExportSha256 : '9' * 64,
    'role': 'property_value',
    'kind': kind,
    'path': path,
    'expected_hex': expectedHex,
  },
};

List<Object?> _feetTextureSizePath() => <Object?>[
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
    'property_type': <String, Object?>{'type': 'struct', 'name': 'Vector4'},
  },
];

Map<String, Object?> _seal(int byteLength, String digit) => <String, Object?>{
  'byte_len': byteLength,
  'sha256': digit * 64,
};

Map<String, Object?> _sealFrom(AuthoringDraftContentSeal seal) =>
    <String, Object?>{'byte_len': seal.byteLength, 'sha256': seal.sha256};
