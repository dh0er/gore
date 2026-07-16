import 'dart:convert';

import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/core/core_service.dart';
import 'package:gore_mod/core/mod_ffi.dart';

import '../support/revision3_voice_fixture.dart';

const _targetPath = '/Game/Blueprints/Items/FootstepPreset';
const _packName = 'ReviewedFootsteps';
const _output = r'C:\Builds\ReviewedFootsteps';
const _root = r'C:\Projects\Reviewed.goreproj';
const _gameRoot = r'C:\Games\Gothic Remake';
const _receiptFormat =
    'gore.authoring.managed-revision3-reviewed-dataasset-build-receipt.v1';
const _receiptName = 'gore-authoring-dataasset-build.json';

String _repeat(String value, int count) =>
    List<String>.filled(count, value).join();

String _headJson(String byte) => jsonEncode(<String, Object?>{
  'store_format': 1,
  'snapshot': <String, Object?>{'byte_len': 321, 'sha256': _repeat(byte, 64)},
});

Map<String, Object?> _fileSeal(String relativeName, String shaByte) =>
    <String, Object?>{
      'relative_name': relativeName,
      'byte_len': 1234,
      'sha256': _repeat(shaByte, 64),
    };

Map<String, Object?> _response({
  String outcome = 'built',
  String? artifactPublicationStatus,
  Object? warning = _defaultWarning,
}) {
  final status =
      artifactPublicationStatus ??
      switch (outcome) {
        'built' => 'published',
        'built_with_cleanup_warning' => 'published_with_cleanup_warning',
        'publication_uncertain' => 'publication_uncertain',
        _ => 'published',
      };
  final resolvedWarning = identical(warning, _defaultWarning)
      ? switch (outcome) {
          'built' => null,
          'built_with_cleanup_warning' => <String, Object?>{
            'code': 'AUTHORING_REVISION3_DATAASSET_BUILD_CLEANUP_WARNING',
            'message':
                'the verified build was published, but private staging cleanup was incomplete',
          },
          'publication_uncertain' => <String, Object?>{
            'code': 'AUTHORING_REVISION3_DATAASSET_BUILD_PUBLICATION_UNCERTAIN',
            'message':
                'publication may have completed; do not retry automatically',
          },
          _ => null,
        }
      : warning;
  return <String, Object?>{
    'ok': true,
    'outcome': outcome,
    'basis_head_json': _headJson('b'),
    'project_id': revision3VoiceFixtureProjectId,
    'project_revision': 7,
    'target_path': _targetPath,
    'pack_name': _packName,
    'output': _output,
    'files': <Object?>[
      _fileSeal('$_packName.pak', 'c'),
      _fileSeal('$_packName.ucas', 'd'),
      _fileSeal('$_packName.utoc', 'e'),
    ],
    'receipt': <String, Object?>{
      'format': _receiptFormat,
      'relative_name': _receiptName,
      'byte_len': 4567,
      'sha256': _repeat('f', 64),
    },
    'build_authority': 'reviewed_fixed_leaf_single_package_triplet',
    'artifact_publication_status': status,
    'deployment_status': 'not_performed',
    'runtime_status': 'runtime_unqualified',
    'retry_safe': false,
    'warning': resolvedWarning,
  };
}

const _defaultWarning = Object();

Map<String, Object?> _copy(Map<String, Object?> value) =>
    (jsonDecode(jsonEncode(value)) as Map).cast<String, Object?>();

AuthoringRevision3ReviewedDataAssetBuildResult _parse(
  Map<String, Object?> response, {
  AuthoringWorkingHead? expectedHead,
  String? expectedProjectJson,
  String expectedTargetPath = _targetPath,
  String expectedPackName = _packName,
  String expectedOutput = _output,
}) => AuthoringRevision3ReviewedDataAssetBuildResult.fromJson(
  response,
  expectedHead:
      expectedHead ?? AuthoringWorkingHead.fromCanonicalJson(_headJson('b')),
  expectedProjectJson:
      expectedProjectJson ?? revision3VoiceFixtureProjectJson(),
  expectedTargetPath: expectedTargetPath,
  expectedPackName: expectedPackName,
  expectedOutput: expectedOutput,
);

void main() {
  final head = AuthoringWorkingHead.fromCanonicalJson(_headJson('b'));

  test('required command handshake includes reviewed R3 DataAsset build', () {
    expect(
      requiredStudioCoreCommands,
      contains('authoring_store_build_revision3_reviewed_dataasset_v1'),
    );
    expect(
      requiredStudioCoreCommands,
      orderedEquals(<String>[...requiredStudioCoreCommands]..sort()),
    );
  });

  test('wrapper sends exactly the canonical seven-field request', () async {
    final projectJson = revision3VoiceFixtureProjectJson();
    final core = FakeGoreCoreFfiService(
      responses: <String, Map<String, Object?>>{
        'authoring_store_build_revision3_reviewed_dataasset_v1': _response(),
      },
    );

    final result = await ModFfi(core)
        .authoringStoreBuildRevision3ReviewedDataAssetV1(
          root: _root,
          gameRoot: _gameRoot,
          currentProjectJson: projectJson,
          expectedHead: head,
          targetPath: _targetPath,
          packName: _packName,
          output: _output,
        );

    expect(
      result.outcome,
      AuthoringRevision3ReviewedDataAssetBuildOutcome.published,
    );
    expect(
      core.calls.single.command,
      'authoring_store_build_revision3_reviewed_dataasset_v1',
    );
    expect(core.calls.single.payload.keys, <String>[
      'current_project_json',
      'expected_head_json',
      'game_root',
      'output',
      'pack_name',
      'root',
      'target_path',
    ]);
    expect(core.calls.single.payload, <String, Object?>{
      'current_project_json': projectJson,
      'expected_head_json': head.canonicalJson,
      'game_root': _gameRoot,
      'output': _output,
      'pack_name': _packName,
      'root': _root,
      'target_path': _targetPath,
    });
  });

  test('strict DTO accepts and distinguishes all terminal publications', () {
    final published = _parse(_response());
    expect(
      published.outcome,
      AuthoringRevision3ReviewedDataAssetBuildOutcome.published,
    );
    expect(published.hasCleanupWarning, isFalse);
    expect(published.publicationIsUncertain, isFalse);
    expect(published.warning, isNull);
    expect(published.basisHead.canonicalJson, head.canonicalJson);
    expect(published.projectId, revision3VoiceFixtureProjectId);
    expect(published.projectRevision, 7);
    expect(published.targetPath, _targetPath);
    expect(published.packName, _packName);
    expect(published.output, _output);
    expect(published.files.map((seal) => seal.relativeName), <String>[
      '$_packName.pak',
      '$_packName.ucas',
      '$_packName.utoc',
    ]);
    expect(published.files.first.byteLength, 1234);
    expect(published.files.first.sha256, _repeat('c', 64));
    expect(published.receipt.format, _receiptFormat);
    expect(published.receipt.relativeName, _receiptName);
    expect(published.receipt.byteLength, 4567);
    expect(published.receipt.sha256, _repeat('f', 64));
    expect(
      () => published.files.add(published.files.first),
      throwsUnsupportedError,
    );

    final cleanup = _parse(_response(outcome: 'built_with_cleanup_warning'));
    expect(
      cleanup.outcome,
      AuthoringRevision3ReviewedDataAssetBuildOutcome
          .publishedWithCleanupWarning,
    );
    expect(cleanup.hasCleanupWarning, isTrue);
    expect(cleanup.publicationIsUncertain, isFalse);
    expect(
      cleanup.warning!.code,
      'AUTHORING_REVISION3_DATAASSET_BUILD_CLEANUP_WARNING',
    );

    final uncertain = _parse(_response(outcome: 'publication_uncertain'));
    expect(
      uncertain.outcome,
      AuthoringRevision3ReviewedDataAssetBuildOutcome.publicationUncertain,
    );
    expect(uncertain.hasCleanupWarning, isFalse);
    expect(uncertain.publicationIsUncertain, isTrue);
    expect(
      uncertain.warning!.message,
      'publication may have completed; do not retry automatically',
    );
  });

  test('strict DTO binds head, project, target, pack, and output exactly', () {
    final mutations = <Map<String, Object?> Function()>[
      () => _response()..['basis_head_json'] = _headJson('a'),
      () => _response()..['project_id'] = _repeat('1', 32),
      () => _response()..['project_revision'] = 8,
      () => _response()..['target_path'] = '/Game/Elsewhere/Preset',
      () => _response()..['pack_name'] = 'OtherPack',
      () => _response()..['output'] = r'C:\Builds\Elsewhere',
    ];
    for (final mutate in mutations) {
      expect(() => _parse(mutate()), throwsFormatException);
    }
    expect(
      () => _parse(
        _response(),
        expectedProjectJson: revision3VoiceFixtureProjectJson(revision: 8),
      ),
      throwsFormatException,
    );
  });

  test('strict DTO rejects unknown, missing, and duplicate wire fields', () {
    expect(
      () => _parse(_response()..['future_authority'] = true),
      throwsFormatException,
    );
    final missing = _response()..remove('receipt');
    expect(() => _parse(missing), throwsFormatException);
    expect(
      () => _parse(_response(outcome: 'future_terminal')),
      throwsFormatException,
    );

    final canonical = jsonEncode(_response());
    final duplicate = canonical.replaceFirst(
      '"ok":true',
      '"ok":true,"ok":true',
    );
    expect(
      () => decodeCanonicalGoreCoreResponse(duplicate),
      throwsFormatException,
    );
  });

  test('strict DTO requires the ordered path-free triplet exactly once', () {
    final tooFew = _response();
    (tooFew['files']! as List).removeLast();
    final tooMany = _response();
    (tooMany['files']! as List).add(_fileSeal('$_packName.extra', 'a'));
    final reordered = _response();
    final reorderedFiles = reordered['files']! as List;
    final first = reorderedFiles.removeAt(0);
    reorderedFiles.insert(1, first);
    final duplicated = _response();
    final duplicatedFiles = duplicated['files']! as List;
    duplicatedFiles[1] = _copy(
      (duplicatedFiles[0] as Map).cast<String, Object?>(),
    );
    final pathBearing = _response();
    ((pathBearing['files']! as List).first as Map)['relative_name'] =
        'nested\\$_packName.pak';
    final extraSealField = _response();
    ((extraSealField['files']! as List).first as Map)['path'] = _output;
    final emptySeal = _response();
    ((emptySeal['files']! as List).first as Map)['byte_len'] = 0;
    final oversizedSeal = _response();
    ((oversizedSeal['files']! as List).first as Map)['byte_len'] =
        2 * 1024 * 1024 * 1024 + 1;
    final malformedSha = _response();
    ((malformedSha['files']! as List).first as Map)['sha256'] = _repeat(
      'A',
      64,
    );

    for (final malformed in <Map<String, Object?>>[
      tooFew,
      tooMany,
      reordered,
      duplicated,
      pathBearing,
      extraSealField,
      emptySeal,
      oversizedSeal,
      malformedSha,
    ]) {
      expect(() => _parse(malformed), throwsFormatException);
    }
  });

  test('strict DTO requires the canonical bounded receipt seal', () {
    final mutations = <void Function(Map<String, Object?>)>[
      (receipt) => receipt['format'] = 'future.receipt.v2',
      (receipt) => receipt['relative_name'] = 'receipt.json',
      (receipt) => receipt['relative_name'] = r'nested\receipt.json',
      (receipt) => receipt['byte_len'] = 0,
      (receipt) => receipt['byte_len'] = 8 * 1024 * 1024 + 1,
      (receipt) => receipt['sha256'] = _repeat('F', 64),
      (receipt) => receipt['path'] = _output,
    ];
    for (final mutate in mutations) {
      final response = _response();
      mutate((response['receipt']! as Map).cast<String, Object?>());
      expect(() => _parse(response), throwsFormatException);
    }
  });

  test('strict DTO rejects widened authority and terminal disagreement', () {
    final mutations = <Map<String, Object?> Function()>[
      () => _response()..['build_authority'] = 'deploy',
      () => _response()..['deployment_status'] = 'performed',
      () => _response()..['runtime_status'] = 'runtime_qualified',
      () => _response()..['retry_safe'] = true,
      () => _response()..['retry_safe'] = 0,
      () => _response()..['artifact_publication_status'] = 'uncertain',
      () => _response()
        ..['warning'] = <String, Object?>{
          'code': 'AUTHORING_REVISION3_DATAASSET_BUILD_CLEANUP_WARNING',
          'message':
              'the verified build was published, but private staging cleanup was incomplete',
        },
      () => _response(outcome: 'built_with_cleanup_warning', warning: null),
      () => _response(
        outcome: 'built_with_cleanup_warning',
        warning: <String, Object?>{
          'code': 'AUTHORING_REVISION3_DATAASSET_BUILD_CLEANUP_WARNING',
          'message': 'changed',
        },
      ),
      () => _response(
        outcome: 'publication_uncertain',
        warning: <String, Object?>{
          'code': 'AUTHORING_REVISION3_DATAASSET_BUILD_PUBLICATION_UNCERTAIN',
          'message':
              'publication may have completed; do not retry automatically',
          'retry': true,
        },
      ),
    ];
    for (final mutate in mutations) {
      expect(() => _parse(mutate()), throwsFormatException);
    }
  });

  test(
    'wrapper rejects malformed native success as a native response error',
    () async {
      final malformed = _response()..['runtime_status'] = 'supported';
      final core = FakeGoreCoreFfiService(
        responses: <String, Map<String, Object?>>{
          'authoring_store_build_revision3_reviewed_dataasset_v1': malformed,
        },
      );

      await expectLater(
        ModFfi(core).authoringStoreBuildRevision3ReviewedDataAssetV1(
          root: _root,
          gameRoot: _gameRoot,
          currentProjectJson: revision3VoiceFixtureProjectJson(),
          expectedHead: head,
          targetPath: _targetPath,
          packName: _packName,
          output: _output,
        ),
        throwsA(
          isA<ModFfiException>()
              .having(
                (error) => error.command,
                'command',
                'authoring_store_build_revision3_reviewed_dataasset_v1',
              )
              .having(
                (error) => error.code,
                'code',
                ModFfiException.malformedNativeResponseCode,
              ),
        ),
      );
    },
  );

  test('wrapper rejects unsafe pack names before calling native', () async {
    for (final packName in <String>[
      '',
      '-starts-with-dash',
      'bad.name',
      'NUL',
      'COM1',
      'nøn_ascii',
      _repeat('x', 97),
    ]) {
      final core = FakeGoreCoreFfiService(responses: const {});
      await expectLater(
        ModFfi(core).authoringStoreBuildRevision3ReviewedDataAssetV1(
          root: _root,
          gameRoot: _gameRoot,
          currentProjectJson: revision3VoiceFixtureProjectJson(),
          expectedHead: head,
          targetPath: _targetPath,
          packName: packName,
          output: _output,
        ),
        throwsArgumentError,
      );
      expect(core.calls, isEmpty);
    }
  });
}
