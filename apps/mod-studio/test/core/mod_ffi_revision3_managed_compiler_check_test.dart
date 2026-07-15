import 'dart:convert';

import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/core/core_service.dart';
import 'package:gore_mod/core/mod_ffi.dart';
import 'package:gore_mod/scripts/domain/script_compile_report.dart';

const _root = r'C:\Projects\CompilerCheck.goreproj';
const _gameRoot = r'C:\Games\Gothic 1 Remake';
const _projectId = '00000000000000000000000000000031';
const _questId = '00000000000000000000000000000071';
const _npcId = '00000000000000000000000000000081';
const _moduleId = '00000000000000000000000000000091';
const _projectBytes = 4096;
const _projectSha =
    'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa';

Map<String, Object?> _seal(int byteLength, String sha256) => <String, Object?>{
  'byte_len': byteLength,
  'sha256': sha256,
};

String _headJson({String sha256 = _projectSha}) => jsonEncode(<String, Object?>{
  'store_format': 1,
  'snapshot': _seal(_projectBytes, sha256),
});

Map<String, Object?> _diagnostics({
  String capture = 'captured',
  String severity = 'warning',
}) => <String, Object?>{
  'capture': capture,
  'messages': <Object?>[
    <String, Object?>{
      'file': 'GoreMods/Quests/CompilerCheck.as',
      'line': 12,
      'column': 7,
      'severity': severity,
      'message': 'bounded diagnostic',
    },
  ],
  'omitted': 0,
};

Map<String, Object?> _compiledEvidence() => <String, Object?>{
  'outcome': 'compiled_evidence_only',
  'compile_error': null,
  'compiler_diagnostics': _diagnostics(),
  'install_restore': 'restored_exact',
  'recovery_required': false,
  'output_discarded': true,
};

Map<String, Object?> _failedEvidence({
  String code = 'COMPILER_REGEN_FAILED',
  String installRestore = 'restored_exact',
  bool recoveryRequired = false,
  Object? diagnostics,
}) => <String, Object?>{
  'outcome': 'failed',
  'compile_error': <String, Object?>{
    'code': code,
    'message': 'compiler rejected the sealed source',
  },
  'compiler_diagnostics': diagnostics,
  'install_restore': installRestore,
  'recovery_required': recoveryRequired,
  'output_discarded': true,
};

Map<String, Object?> _response({
  String entityKind = 'quest_draft',
  String entityId = _questId,
  bool exactCurrent = true,
  Map<String, Object?>? compiler,
}) => <String, Object?>{
  'ok': true,
  'outcome': 'compiler_check_only',
  'exact_current': exactCurrent,
  'head_json': _headJson(),
  'project': <String, Object?>{
    'id': _projectId,
    'revision': 14,
    'seal': _seal(_projectBytes, _projectSha),
  },
  'entity': <String, Object?>{
    'kind': entityKind,
    'id': entityId,
    'revision': 8,
  },
  'module': <String, Object?>{
    'id': _moduleId,
    'revision': 9,
    'namespace': 'GoreMods.Quests.CompilerCheck',
    'relative_path': 'GoreMods/Quests/CompilerCheck.as',
    'source_sha256':
        'bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb',
  },
  'compiler': compiler ?? _compiledEvidence(),
  'scope': 'compiler_check_only',
  'build_status': 'blocked',
  'deploy_status': 'not_supported',
  'runtime_qualification': 'runtime_unqualified',
  'publication_status': 'not_supported',
};

Map<String, Object?> _copy(Map<String, Object?> value) =>
    (jsonDecode(jsonEncode(value)) as Map).cast<String, Object?>();

Future<Object?> _questCall(
  Map<String, Object?> response, {
  String? expectedHeadJson,
}) {
  final core = FakeGoreCoreFfiService(
    responses: <String, Map<String, Object?>>{
      'authoring_store_check_revision3_quest_compiler_v1': response,
    },
  );
  return ModFfi(core).authoringStoreCheckRevision3QuestCompilerV1(
    root: _root,
    gameRoot: _gameRoot,
    expectedHead: AuthoringWorkingHead.fromCanonicalJson(
      expectedHeadJson ?? _headJson(),
    ),
    questId: _questId,
  );
}

Future<void> _expectMalformed(Map<String, Object?> response) => expectLater(
  _questCall(response),
  throwsA(
    isA<ModFfiException>().having(
      (error) => error.code,
      'code',
      ModFfiException.malformedNativeResponseCode,
    ),
  ),
);

void main() {
  test('Studio handshake requires both sorted managed compiler commands', () {
    expect(
      requiredStudioCoreCommands,
      containsAll(<String>[
        'authoring_store_check_revision3_npc_compiler_v1',
        'authoring_store_check_revision3_quest_compiler_v1',
      ]),
    );
    expect(
      requiredStudioCoreCommands,
      orderedEquals(<String>[...requiredStudioCoreCommands]..sort()),
    );
  });

  test(
    'Quest wrapper sends only authority-free inputs and parses evidence',
    () async {
      final core = FakeGoreCoreFfiService(
        responses: <String, Map<String, Object?>>{
          'authoring_store_check_revision3_quest_compiler_v1': _response(),
        },
      );

      final result = await ModFfi(core)
          .authoringStoreCheckRevision3QuestCompilerV1(
            root: _root,
            gameRoot: _gameRoot,
            expectedHead: AuthoringWorkingHead.fromCanonicalJson(_headJson()),
            questId: _questId,
          );

      expect(
        core.calls.single.command,
        'authoring_store_check_revision3_quest_compiler_v1',
      );
      expect(core.calls.single.payload, <String, Object?>{
        'root': _root,
        'game_root': _gameRoot,
        'expected_head_json': _headJson(),
        'quest_id': _questId,
      });
      expect(result.head.canonicalJson, _headJson());
      expect(result.exactCurrent, isTrue);
      expect(result.project.id, _projectId);
      expect(result.project.revision, 14);
      expect(result.project.seal.byteLength, _projectBytes);
      expect(
        result.entity.kind,
        AuthoringRevision3ManagedCompilerEntityKind.questDraft,
      );
      expect(result.entity.id, _questId);
      expect(result.entity.revision, 8);
      expect(result.module.id, _moduleId);
      expect(result.module.revision, 9);
      expect(result.module.namespace, 'GoreMods.Quests.CompilerCheck');
      expect(result.module.relativePath, 'GoreMods/Quests/CompilerCheck.as');
      expect(result.compiler.compiledEvidenceOnly, isTrue);
      expect(result.compiler.outputDiscarded, isTrue);
      expect(
        result.compiler.installRestore,
        ScriptCompileInstallRestore.restoredExact,
      );
      expect(result.compiler.diagnostics!.messages.single.line, 12);
      expect(result.acceptedAtExactCurrent, isTrue);
      expect(result.recoveryRequired, isFalse);
      expect(
        result.scope,
        AuthoringRevision3ManagedCompilerScope.compilerCheckOnly,
      );
      expect(
        result.buildStatus,
        AuthoringRevision3ManagedCompilerBuildStatus.blocked,
      );
      expect(
        result.runtimeQualification,
        AuthoringRevision3ManagedCompilerRuntimeQualification
            .runtimeUnqualified,
      );
    },
  );

  test('NPC wrapper binds the other command, ID, and kind', () async {
    final core = FakeGoreCoreFfiService(
      responses: <String, Map<String, Object?>>{
        'authoring_store_check_revision3_npc_compiler_v1': _response(
          entityKind: 'npc_draft',
          entityId: _npcId,
        ),
      },
    );

    final result = await ModFfi(core).authoringStoreCheckRevision3NpcCompilerV1(
      root: _root,
      gameRoot: _gameRoot,
      expectedHead: AuthoringWorkingHead.fromCanonicalJson(_headJson()),
      npcId: _npcId,
    );

    expect(
      core.calls.single.command,
      'authoring_store_check_revision3_npc_compiler_v1',
    );
    expect(core.calls.single.payload, <String, Object?>{
      'root': _root,
      'game_root': _gameRoot,
      'expected_head_json': _headJson(),
      'npc_id': _npcId,
    });
    expect(
      result.entity.kind,
      AuthoringRevision3ManagedCompilerEntityKind.npcDraft,
    );
    expect(result.entity.id, _npcId);
  });

  test(
    'compiler rejection remains structured evidence, not an exception',
    () async {
      final result =
          await _questCall(
                _response(
                  compiler: _failedEvidence(
                    diagnostics: _diagnostics(severity: 'error'),
                  ),
                ),
              )
              as AuthoringRevision3ManagedCompilerCheckResult;

      expect(
        result.compiler.outcome,
        AuthoringRevision3ManagedCompilerOutcome.failed,
      );
      expect(result.compiler.failure!.code, 'COMPILER_REGEN_FAILED');
      expect(
        result.compiler.diagnostics!.messages.single.severity,
        ScriptCompilerDiagnosticSeverity.error,
      );
      expect(result.acceptedAtExactCurrent, isFalse);
    },
  );

  test(
    'persistent install recovery dominates otherwise exact bindings',
    () async {
      final result =
          await _questCall(
                _response(
                  exactCurrent: false,
                  compiler: _failedEvidence(
                    code: 'COMPILE_INSTALL_RECOVERY_REQUIRED',
                    installRestore: 'not_started',
                    recoveryRequired: true,
                  ),
                ),
              )
              as AuthoringRevision3ManagedCompilerCheckResult;

      expect(result.exactCurrent, isFalse);
      expect(result.recoveryRequired, isTrue);
      expect(result.acceptedAtExactCurrent, isFalse);

      await _expectMalformed(
        _response(
          compiler: _failedEvidence(
            code: 'COMPILE_INSTALL_RECOVERY_REQUIRED',
            installRestore: 'not_started',
            recoveryRequired: true,
          ),
        ),
      );
    },
  );

  test(
    'post-attempt head drift preserves evidence but revokes acceptance',
    () async {
      final result =
          await _questCall(_response(exactCurrent: false))
              as AuthoringRevision3ManagedCompilerCheckResult;

      expect(result.exactCurrent, isFalse);
      expect(result.compiler.compiledEvidenceOnly, isTrue);
      expect(result.acceptedAtExactCurrent, isFalse);

      final recovery =
          await _questCall(
                _response(
                  exactCurrent: false,
                  compiler: _failedEvidence(
                    code: 'COMPILE_INSTALL_RECOVERY_REQUIRED',
                    installRestore: 'not_started',
                    recoveryRequired: true,
                  ),
                ),
              )
              as AuthoringRevision3ManagedCompilerCheckResult;
      expect(recovery.exactCurrent, isFalse);
      expect(recovery.recoveryRequired, isTrue);
      expect(recovery.acceptedAtExactCurrent, isFalse);
    },
  );

  test(
    'foreign head, entity, kind, and head/project seal are rejected',
    () async {
      final foreignHead = _copy(_response());
      foreignHead['head_json'] = _headJson(
        sha256:
            'cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc',
      );
      await _expectMalformed(foreignHead);

      await _expectMalformed(_response(entityId: _npcId));
      await _expectMalformed(_response(entityKind: 'npc_draft'));

      final foreignProject = _copy(_response());
      (foreignProject['project']! as Map<String, Object?>)['seal'] = _seal(
        _projectBytes,
        'dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd',
      );
      await _expectMalformed(foreignProject);
    },
  );

  test('module identity, canonical path, and source seal are strict', () async {
    final alias = _copy(_response());
    (alias['module']! as Map<String, Object?>)['id'] = _questId;
    await _expectMalformed(alias);

    final path = _copy(_response());
    (path['module']! as Map<String, Object?>)['relative_path'] =
        '../CompilerCheck.as';
    await _expectMalformed(path);

    final hash = _copy(_response());
    (hash['module']! as Map<String, Object?>)['source_sha256'] =
        'BBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBB';
    await _expectMalformed(hash);
  });

  test(
    'compiled evidence requires diagnostics, usable capture, and no errors',
    () async {
      final missing = _copy(_response());
      (missing['compiler']! as Map<String, Object?>)['compiler_diagnostics'] =
          null;
      await _expectMalformed(missing);

      final disabled = _copy(_response());
      ((disabled['compiler']! as Map<String, Object?>)['compiler_diagnostics']!
              as Map<String, Object?>)['capture'] =
          'disabled';
      await _expectMalformed(disabled);

      final error = _copy(_response());
      final errorMessages =
          (((error['compiler']!
                      as Map<String, Object?>)['compiler_diagnostics']!
                  as Map<String, Object?>)['messages']!
              as List<Object?>);
      (errorMessages.single! as Map<String, Object?>)['severity'] = 'error';
      await _expectMalformed(error);
    },
  );

  test('failure and recovery invariants fail closed', () async {
    final noFailure = _copy(_response(compiler: _failedEvidence()));
    (noFailure['compiler']! as Map<String, Object?>)['compile_error'] = null;
    await _expectMalformed(noFailure);

    final falseRecovery = _copy(_response(compiler: _failedEvidence()));
    final falseRecoveryCompiler =
        falseRecovery['compiler']! as Map<String, Object?>;
    falseRecoveryCompiler['install_restore'] =
        'recovery_required_restore_failed';
    falseRecoveryCompiler['recovery_required'] = false;
    await _expectMalformed(falseRecovery);

    final forgedRecovery = _copy(
      _response(compiler: _failedEvidence(recoveryRequired: true)),
    );
    await _expectMalformed(forgedRecovery);
  });

  test(
    'artifact/path fields and unknown fields are not part of the contract',
    () async {
      final mini = _copy(_response());
      (mini['compiler']! as Map<String, Object?>)['mini_path'] =
          r'C:\Temp\module.cache';
      await _expectMalformed(mini);

      final moduleArtifact = _copy(_response());
      (moduleArtifact['compiler']! as Map<String, Object?>)['module'] =
          'GoreMods.Quests.CompilerCheck';
      await _expectMalformed(moduleArtifact);

      final top = _copy(_response());
      top['build_authority'] = true;
      await _expectMalformed(top);
    },
  );

  test('numeric and diagnostic bounds are fail closed', () async {
    final decimal = _copy(_response());
    (decimal['entity']! as Map<String, Object?>)['revision'] = 1.5;
    await _expectMalformed(decimal);

    final coordinate = _copy(_response());
    final messages =
        (((coordinate['compiler']!
                    as Map<String, Object?>)['compiler_diagnostics']!
                as Map<String, Object?>)['messages']!
            as List<Object?>);
    (messages.single! as Map<String, Object?>)['line'] = 0x100000000;
    await _expectMalformed(coordinate);
  });

  test(
    'undiscarded output can only be failed, non-authoritative evidence',
    () async {
      final impossibleCompiled = _copy(_response());
      (impossibleCompiled['compiler']!
              as Map<String, Object?>)['output_discarded'] =
          false;
      await _expectMalformed(impossibleCompiled);

      final failed = _failedEvidence(code: 'COMPILE_OUTPUT_UNSAFE');
      failed['output_discarded'] = false;
      final result =
          await _questCall(_response(compiler: failed))
              as AuthoringRevision3ManagedCompilerCheckResult;
      expect(result.compiler.outputDiscarded, isFalse);
      expect(result.compiler.recoveryRequired, isFalse);
      expect(result.acceptedAtExactCurrent, isFalse);
    },
  );

  test(
    'canonical transport rejects duplicate response fields before DTO parsing',
    () {
      expect(
        () => decodeCanonicalGoreCoreResponse(
          '{"ok":true,"ok":true,"outcome":"compiler_check_only"}',
        ),
        throwsFormatException,
      );
    },
  );

  test('invalid IDs and paths are rejected before native execution', () async {
    final core = FakeGoreCoreFfiService(
      responses: <String, Map<String, Object?>>{
        'authoring_store_check_revision3_quest_compiler_v1': _response(),
      },
    );
    final ffi = ModFfi(core);
    final head = AuthoringWorkingHead.fromCanonicalJson(_headJson());

    await expectLater(
      ffi.authoringStoreCheckRevision3QuestCompilerV1(
        root: _root,
        gameRoot: _gameRoot,
        expectedHead: head,
        questId: '00000000000000000000000000000000',
      ),
      throwsFormatException,
    );
    await expectLater(
      ffi.authoringStoreCheckRevision3QuestCompilerV1(
        root: 'bad\u0000path',
        gameRoot: _gameRoot,
        expectedHead: head,
        questId: _questId,
      ),
      throwsArgumentError,
    );
    await expectLater(
      ffi.authoringStoreCheckRevision3QuestCompilerV1(
        root: _root,
        gameRoot: 'bad\u0000path',
        expectedHead: head,
        questId: _questId,
      ),
      throwsArgumentError,
    );
    expect(core.calls, isEmpty);
  });
}
