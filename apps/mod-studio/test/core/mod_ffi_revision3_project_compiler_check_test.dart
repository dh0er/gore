import 'dart:convert';

import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/core/core_service.dart';
import 'package:gore_mod/core/mod_ffi.dart';
import 'package:gore_mod/scripts/domain/script_compile_report.dart';

const _command = 'authoring_store_check_revision3_project_compiler_v1';
const _commandV2 = 'authoring_store_check_revision3_project_compiler_v2';
const _root = r'C:\Projects\ProjectCompiler.goreproj';
const _gameRoot = r'C:\Games\Gothic 1 Remake';
const _projectId = '00000000000000000000000000000031';
const _projectBytes = 4096;
const _projectSha =
    'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa';
const _executableSha =
    'bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb';

Map<String, Object?> _seal(int bytes, String sha256) => <String, Object?>{
  'byte_len': bytes,
  'sha256': sha256,
};

Map<String, Object?> _qualifiedPackage() => <String, Object?>{
  'catalog_sha256':
      '1111111111111111111111111111111111111111111111111111111111111111',
  'sidecar_byte_len': 1966592,
  'sidecar_sha256':
      '2222222222222222222222222222222222222222222222222222222222222222',
  'request_version': 2,
  'response_version': 1,
  'manifest_byte_len': 4096,
  'manifest_sha256':
      '3333333333333333333333333333333333333333333333333333333333333333',
  'profile_sha256':
      '4444444444444444444444444444444444444444444444444444444444444444',
  'target': <String, Object?>{
    'target': <String, Object?>{
      'steam_app_id': 1297900,
      'steam_build_id': 24539464,
      'depot_id': 1297901,
      'depot_manifest_gid': 1585071322101748861,
      'platform': 'windows',
      'architecture': 'x86_64',
      'build_configuration': 'shipping',
    },
    'pe_codeview': <String, Object?>{
      'guid': 'be78fe0a-46ac-6643-9685-97e85c7e5b3f',
      'age': 1,
    },
  },
};

String _headJson() => jsonEncode(<String, Object?>{
  'store_format': 1,
  'snapshot': _seal(_projectBytes, _projectSha),
});

Map<String, Object?> _diagnostics({String severity = 'warning'}) =>
    <String, Object?>{
      'capture': 'captured',
      'messages': <Object?>[
        <String, Object?>{
          'file': 'GoreMods/Project/Quest.as',
          'line': 7,
          'column': 3,
          'severity': severity,
          'message': 'bounded project diagnostic',
        },
      ],
      'omitted': 0,
    };

Map<String, Object?> _compiledCompiler() => <String, Object?>{
  'outcome': 'compiled_evidence_only',
  'run_count': 1,
  'compile_error': null,
  'compiler_diagnostics': _diagnostics(),
  'install_restore': 'restored_exact',
  'recovery_required': false,
  'output_disposition': 'discarded',
};

Map<String, Object?> _emptyCompiler() => <String, Object?>{
  'outcome': 'not_needed_empty',
  'run_count': 0,
  'compile_error': null,
  'compiler_diagnostics': null,
  'install_restore': 'not_started',
  'recovery_required': false,
  'output_disposition': 'not_created',
};

Map<String, Object?> _failedCompiler({
  int runCount = 1,
  String code = 'COMPILER_REGEN_FAILED',
  String installRestore = 'restored_exact',
  bool recoveryRequired = false,
  String outputDisposition = 'discarded',
  Object? diagnostics,
}) => <String, Object?>{
  'outcome': 'failed',
  'run_count': runCount,
  'compile_error': <String, Object?>{
    'code': code,
    'message': 'project compiler rejected the exact source tree',
  },
  'compiler_diagnostics': diagnostics,
  'install_restore': installRestore,
  'recovery_required': recoveryRequired,
  'output_disposition': outputDisposition,
};

Map<String, Object?> _response({
  bool exactCurrent = true,
  String storeAudit = 'exact',
  String gameAudit = 'exact',
  int scriptCount = 2,
  int questCount = 1,
  int npcCount = 1,
  Map<String, Object?>? compiler,
}) => <String, Object?>{
  'ok': true,
  'outcome': 'project_compiler_check_only',
  'exact_current': exactCurrent,
  'closing_audit': <String, Object?>{'store': storeAudit, 'game': gameAudit},
  'head_json': _headJson(),
  'project': <String, Object?>{
    'id': _projectId,
    'revision': 14,
    'seal': _seal(_projectBytes, _projectSha),
  },
  'game_inputs': <String, Object?>{
    'executable': _seal(171698176, _executableSha),
    'shipping_cache': _seal(
      123406626,
      'cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc',
    ),
    'binds_cache': _seal(
      123456,
      'dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd',
    ),
    'story_catalog': _seal(
      4096,
      'eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee',
    ),
  },
  'coverage': <String, Object?>{
    'script_module_count': scriptCount,
    'quest_module_count': questCount,
    'npc_module_count': npcCount,
    'module_manifest': _seal(
      512,
      'ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff',
    ),
  },
  'compiler': compiler ?? _compiledCompiler(),
  'scope': 'project_compiler_check_only',
  'build_status': 'blocked',
  'deploy_status': 'not_supported',
  'runtime_qualification': 'runtime_unqualified',
  'publication_status': 'not_supported',
};

Map<String, Object?> _copy(Map<String, Object?> value) =>
    (jsonDecode(jsonEncode(value)) as Map).cast<String, Object?>();

Future<Object?> _call(Map<String, Object?> response) {
  final core = FakeGoreCoreFfiService(
    responses: <String, Map<String, Object?>>{_command: response},
  );
  return ModFfi(core).authoringStoreCheckRevision3ProjectCompilerV1(
    root: _root,
    gameRoot: _gameRoot,
    expectedHead: AuthoringWorkingHead.fromCanonicalJson(_headJson()),
  );
}

Future<AuthoringRevision3ProjectCompilerCheckResult> _callV2(
  Map<String, Object?> response,
  ScriptCompilerBackendMode mode,
) {
  final core = FakeGoreCoreFfiService(
    responses: <String, Map<String, Object?>>{_commandV2: response},
  );
  return ModFfi(core).authoringStoreCheckRevision3ProjectCompilerV2(
    root: _root,
    gameRoot: _gameRoot,
    expectedHead: AuthoringWorkingHead.fromCanonicalJson(_headJson()),
    compilerBackend: mode,
  );
}

Future<void> _expectMalformed(Map<String, Object?> response) => expectLater(
  _call(response),
  throwsA(
    isA<ModFfiException>().having(
      (error) => error.code,
      'code',
      ModFfiException.malformedNativeResponseCode,
    ),
  ),
);

void main() {
  test('Studio requires the sorted project compiler capability', () {
    expect(requiredStudioCoreCommands, contains(_command));
    expect(requiredStudioCoreCommands, contains(_commandV2));
    expect(
      requiredStudioCoreCommands,
      orderedEquals(<String>[...requiredStudioCoreCommands]..sort()),
    );
  });

  test(
    'V2 strict standalone is explicit, package-free, and game-free',
    () async {
      final response = _response(
        exactCurrent: false,
        storeAudit: 'not_run',
        gameAudit: 'not_run',
        compiler:
            _failedCompiler(
                runCount: 0,
                code: 'AUTHORING_REVISION3_PROJECT_STANDALONE_BUNDLE_ABSENT',
                installRestore: 'not_started',
                outputDisposition: 'not_created',
              )
              ..['compiler_backend'] = <String, Object?>{
                'requested_mode': 'standalone',
                'result_backend': null,
                'standalone_attempted': false,
                'game_attempted': false,
                'qualified_package': null,
                'fallback_reason': null,
              },
      )..['game_inputs'] = null;
      final core = FakeGoreCoreFfiService(
        responses: <String, Map<String, Object?>>{_commandV2: response},
      );

      final result = await ModFfi(core)
          .authoringStoreCheckRevision3ProjectCompilerV2(
            root: _root,
            gameRoot: _gameRoot,
            expectedHead: AuthoringWorkingHead.fromCanonicalJson(_headJson()),
            compilerBackend: ScriptCompilerBackendMode.standalone,
          );

      expect(core.calls.single.command, _commandV2);
      expect(core.calls.single.payload, <String, Object?>{
        'compiler_backend': 'standalone',
        'root': _root,
        'game_root': _gameRoot,
        'expected_head_json': _headJson(),
      });
      expect(result.gameInputsOrNull, isNull);
      expect(
        result.compiler.backend?.requestedMode,
        ScriptCompilerBackendMode.standalone,
      );
      expect(result.compiler.backend?.resultBackend, isNull);
      expect(result.compiler.backend?.standaloneAttempted, isFalse);
      expect(result.compiler.backend?.gameAttempted, isFalse);
      expect(result.compiler.failure?.code, contains('BUNDLE_ABSENT'));

      for (final invalid in <(ScriptCompilerBackendMode, Map<String, Object?>)>[
        (
          ScriptCompilerBackendMode.game,
          _copy(response)
            ..['compiler'] = <String, Object?>{
              ...response['compiler']! as Map<String, Object?>,
              'compiler_backend': <String, Object?>{
                'requested_mode': 'game',
                'result_backend': null,
                'standalone_attempted': false,
                'game_attempted': false,
                'qualified_package': null,
                'fallback_reason': null,
              },
            },
        ),
        (
          ScriptCompilerBackendMode.standalone,
          _copy(response)
            ..['compiler'] = <String, Object?>{
              ...response['compiler']! as Map<String, Object?>,
              'compiler_backend': <String, Object?>{
                'requested_mode': 'standalone',
                'result_backend': null,
                'standalone_attempted': true,
                'game_attempted': false,
                'qualified_package': null,
                'fallback_reason': null,
              },
            },
        ),
      ]) {
        final invalidCore = FakeGoreCoreFfiService(
          responses: <String, Map<String, Object?>>{_commandV2: invalid.$2},
        );
        await expectLater(
          ModFfi(invalidCore).authoringStoreCheckRevision3ProjectCompilerV2(
            root: _root,
            gameRoot: _gameRoot,
            expectedHead: AuthoringWorkingHead.fromCanonicalJson(_headJson()),
            compilerBackend: invalid.$1,
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

  test('V2 accepts strict standalone and two-run fallback evidence', () async {
    final strict = _compiledCompiler()
      ..['install_restore'] = 'not_started'
      ..['compiler_backend'] = <String, Object?>{
        'requested_mode': 'standalone',
        'result_backend': 'standalone',
        'standalone_attempted': true,
        'game_attempted': false,
        'qualified_package': _qualifiedPackage(),
        'fallback_reason': null,
      };
    final strictResult = await _callV2(
      _response(compiler: strict),
      ScriptCompilerBackendMode.standalone,
    );
    expect(strictResult.compiler.compiledEvidenceOnly, isTrue);
    expect(strictResult.compiler.runCount, 1);
    expect(
      strictResult.compiler.installRestore,
      ScriptCompileInstallRestore.notStarted,
    );

    final fallback = _compiledCompiler()
      ..['run_count'] = 2
      ..['compiler_backend'] = <String, Object?>{
        'requested_mode': 'standalone_then_game',
        'result_backend': 'game',
        'standalone_attempted': true,
        'game_attempted': true,
        'qualified_package': _qualifiedPackage(),
        'fallback_reason': <String, Object?>{
          'failed_backend': 'standalone',
          'failure_kind': 'rejected',
          'detail': 'standalone compiler rejected the sealed graph',
        },
      };
    final fallbackResult = await _callV2(
      _response(compiler: fallback),
      ScriptCompilerBackendMode.standaloneThenGame,
    );
    expect(fallbackResult.compiler.compiledEvidenceOnly, isTrue);
    expect(fallbackResult.compiler.runCount, 2);
    expect(
      fallbackResult.compiler.installRestore,
      ScriptCompileInstallRestore.restoredExact,
    );

    final mismatchedRunCount = _copy(_response(compiler: fallback));
    (mismatchedRunCount['compiler']! as Map<String, Object?>)['run_count'] = 1;
    await expectLater(
      _callV2(mismatchedRunCount, ScriptCompilerBackendMode.standaloneThenGame),
      throwsA(
        isA<ModFfiException>().having(
          (error) => error.code,
          'code',
          ModFfiException.malformedNativeResponseCode,
        ),
      ),
    );

    final untouchedGame = _compiledCompiler()
      ..['install_restore'] = 'not_started'
      ..['compiler_backend'] = <String, Object?>{
        'requested_mode': 'game',
        'result_backend': 'game',
        'standalone_attempted': false,
        'game_attempted': true,
        'qualified_package': null,
        'fallback_reason': null,
      };
    await expectLater(
      _callV2(
        _response(compiler: untouchedGame),
        ScriptCompilerBackendMode.game,
      ),
      throwsA(
        isA<ModFfiException>().having(
          (error) => error.code,
          'code',
          ModFfiException.malformedNativeResponseCode,
        ),
      ),
    );
  });

  test(
    'V2 empty fallback mode accepts no run and preserves unavailability',
    () async {
      final noOp = _emptyCompiler()
        ..['compiler_backend'] = <String, Object?>{
          'requested_mode': 'standalone_then_game',
          'result_backend': null,
          'standalone_attempted': false,
          'game_attempted': false,
          'qualified_package': null,
          'fallback_reason': <String, Object?>{
            'failed_backend': 'standalone',
            'failure_kind': 'unavailable',
            'detail': 'authenticated standalone bundle is unavailable',
          },
        };
      final result = await _callV2(
        _response(scriptCount: 0, questCount: 0, npcCount: 0, compiler: noOp),
        ScriptCompilerBackendMode.standaloneThenGame,
      );

      expect(result.compiler.notNeededEmpty, isTrue);
      expect(result.compiler.runCount, 0);
      expect(result.compiler.backend!.fallbackReason, isNotNull);
    },
  );

  test('wrapper sends exactly the three Store-owned inputs', () async {
    final core = FakeGoreCoreFfiService(
      responses: <String, Map<String, Object?>>{_command: _response()},
    );
    final result = await ModFfi(core)
        .authoringStoreCheckRevision3ProjectCompilerV1(
          root: _root,
          gameRoot: _gameRoot,
          expectedHead: AuthoringWorkingHead.fromCanonicalJson(_headJson()),
        );

    expect(core.calls, hasLength(1));
    expect(core.calls.single.command, _command);
    expect(core.calls.single.payload, <String, Object?>{
      'root': _root,
      'game_root': _gameRoot,
      'expected_head_json': _headJson(),
    });
    expect(result.acceptedAtExactCurrent, isTrue);
    expect(result.project.id, _projectId);
    expect(result.project.revision, 14);
    expect(result.coverage.scriptModuleCount, 2);
    expect(result.coverage.questModuleCount, 1);
    expect(result.coverage.npcModuleCount, 1);
    expect(result.gameInputs.executable.sha256, _executableSha);
    expect(result.compiler.runCount, 1);
    expect(
      result.compiler.outputDisposition,
      AuthoringRevision3ProjectCompilerOutputDisposition.discarded,
    );
    expect(
      result.compiler.installRestore,
      ScriptCompileInstallRestore.restoredExact,
    );
  });

  test(
    'compiler rejection remains bounded non-authoritative evidence',
    () async {
      final result =
          await _call(
                _response(
                  compiler: _failedCompiler(
                    diagnostics: _diagnostics(severity: 'error'),
                  ),
                ),
              )
              as AuthoringRevision3ProjectCompilerCheckResult;

      expect(result.exactCurrent, isTrue);
      expect(result.acceptedAtExactCurrent, isFalse);
      expect(result.compiler.failure!.code, 'COMPILER_REGEN_FAILED');
      expect(
        result.compiler.diagnostics!.messages.single.severity,
        ScriptCompilerDiagnosticSeverity.error,
      );
    },
  );

  test('empty-project evidence requires zero coverage and no run', () async {
    final result =
        await _call(
              _response(
                scriptCount: 0,
                questCount: 0,
                npcCount: 0,
                compiler: _emptyCompiler(),
              ),
            )
            as AuthoringRevision3ProjectCompilerCheckResult;
    expect(result.coverage.isEmpty, isTrue);
    expect(result.compiler.notNeededEmpty, isTrue);
    expect(result.acceptedAtExactCurrent, isTrue);

    await _expectMalformed(_response(compiler: _emptyCompiler()));
    await _expectMalformed(
      _response(
        scriptCount: 0,
        questCount: 0,
        npcCount: 0,
        compiler: _compiledCompiler(),
      ),
    );
  });

  test('coverage counts and head/project binding are exact', () async {
    await _expectMalformed(
      _response(scriptCount: 2, questCount: 2, npcCount: 1),
    );

    final projectSeal = _copy(_response());
    ((projectSeal['project']! as Map<String, Object?>)['seal']!
            as Map<String, Object?>)['sha256'] =
        '1111111111111111111111111111111111111111111111111111111111111111';
    await _expectMalformed(projectSeal);

    final foreignHead = _copy(_response());
    foreignHead['head_json'] = jsonEncode(<String, Object?>{
      'store_format': 1,
      'snapshot': _seal(
        _projectBytes,
        '2222222222222222222222222222222222222222222222222222222222222222',
      ),
    });
    await _expectMalformed(foreignHead);
  });

  test('every project and input evidence seal is non-empty', () async {
    for (final mutate in <void Function(Map<String, Object?>)>[
      (response) {
        final project = response['project']! as Map<String, Object?>;
        final seal = project['seal']! as Map<String, Object?>;
        seal['byte_len'] = 0;
      },
      (response) {
        final inputs = response['game_inputs']! as Map<String, Object?>;
        final seal = inputs['executable']! as Map<String, Object?>;
        seal['byte_len'] = 0;
      },
      (response) {
        final inputs = response['game_inputs']! as Map<String, Object?>;
        final seal = inputs['shipping_cache']! as Map<String, Object?>;
        seal['byte_len'] = 0;
      },
      (response) {
        final inputs = response['game_inputs']! as Map<String, Object?>;
        final seal = inputs['binds_cache']! as Map<String, Object?>;
        seal['byte_len'] = 0;
      },
      (response) {
        final inputs = response['game_inputs']! as Map<String, Object?>;
        final seal = inputs['story_catalog']! as Map<String, Object?>;
        seal['byte_len'] = 0;
      },
      (response) {
        final coverage = response['coverage']! as Map<String, Object?>;
        final seal = coverage['module_manifest']! as Map<String, Object?>;
        seal['byte_len'] = 0;
      },
    ]) {
      final response = _copy(_response());
      mutate(response);
      await _expectMalformed(response);
    }
  });

  test('compiled outcome rejects every authority or safety widening', () async {
    for (final mutate in <void Function(Map<String, Object?>)>[
      (response) => response['build_status'] = 'ready',
      (response) => response['deploy_status'] = 'supported',
      (response) => response['artifact_path'] = r'C:\Temp\project.cache',
      (response) =>
          (response['compiler']! as Map<String, Object?>)['run_count'] = 0,
      (response) =>
          (response['compiler']!
                  as Map<String, Object?>)['output_disposition'] =
              'not_created',
      (response) =>
          (response['compiler']!
                  as Map<String, Object?>)['compiler_diagnostics'] =
              null,
      (response) =>
          (((((response['compiler']!
                                  as Map<
                                    String,
                                    Object?
                                  >)['compiler_diagnostics']!
                              as Map<String, Object?>)['messages']!
                          as List<Object?>)
                      .single)
                  as Map<String, Object?>)['severity'] =
              'error',
    ]) {
      final response = _copy(_response());
      mutate(response);
      await _expectMalformed(response);
    }
  });

  test('closing audit is strict and solely defines exact-current', () async {
    final missing = _copy(_response());
    missing.remove('closing_audit');
    await _expectMalformed(missing);

    final extra = _copy(_response());
    (extra['closing_audit']! as Map<String, Object?>)['future'] = 'exact';
    await _expectMalformed(extra);

    final unsupported = _copy(_response());
    (unsupported['closing_audit']! as Map<String, Object?>)['store'] =
        'unknown';
    await _expectMalformed(unsupported);

    await _expectMalformed(_response(exactCurrent: false));
    await _expectMalformed(_response(storeAudit: 'drift', exactCurrent: true));

    final storeDrift =
        await _call(
              _response(
                exactCurrent: false,
                storeAudit: 'drift',
                compiler: _failedCompiler(
                  diagnostics: _diagnostics(severity: 'error'),
                ),
              ),
            )
            as AuthoringRevision3ProjectCompilerCheckResult;
    expect(storeDrift.exactCurrent, isFalse);
    expect(
      storeDrift.closingAudit.store,
      AuthoringRevision3ProjectCompilerClosingAuditStatus.drift,
    );
    expect(
      storeDrift.closingAudit.game,
      AuthoringRevision3ProjectCompilerClosingAuditStatus.exact,
    );

    for (final response in <Map<String, Object?>>[
      _response(exactCurrent: false, gameAudit: 'inspection_failed'),
      _response(
        exactCurrent: false,
        storeAudit: 'not_run',
        scriptCount: 0,
        questCount: 0,
        npcCount: 0,
        compiler: _emptyCompiler(),
      ),
    ]) {
      await _expectMalformed(response);
    }
  });

  test('install recovery stays separate from output disposition', () async {
    final recovery = _failedCompiler(
      runCount: 0,
      code: 'COMPILE_INSTALL_RECOVERY_REQUIRED',
      installRestore: 'not_started',
      recoveryRequired: true,
      outputDisposition: 'not_created',
    );
    final result =
        await _call(_response(compiler: recovery))
            as AuthoringRevision3ProjectCompilerCheckResult;
    expect(result.exactCurrent, isTrue);
    expect(result.recoveryRequired, isTrue);
    expect(result.acceptedAtExactCurrent, isFalse);

    final emptyRecovery =
        await _call(
              _response(
                scriptCount: 0,
                questCount: 0,
                npcCount: 0,
                compiler: recovery,
              ),
            )
            as AuthoringRevision3ProjectCompilerCheckResult;
    expect(emptyRecovery.coverage.isEmpty, isTrue);
    expect(emptyRecovery.recoveryRequired, isTrue);

    await _expectMalformed(
      _response(
        scriptCount: 0,
        questCount: 0,
        npcCount: 0,
        compiler: _failedCompiler(
          runCount: 0,
          code: 'COMPILER_PREFLIGHT_FAILED',
          installRestore: 'not_started',
          outputDisposition: 'not_created',
        ),
      ),
    );

    final inventedOutput = _copy(_response(compiler: recovery));
    (inventedOutput['compiler']!
            as Map<String, Object?>)['output_disposition'] =
        'recovery_retained';
    await _expectMalformed(inventedOutput);
  });

  test(
    'failed preflight and attempted failure have distinct output rules',
    () async {
      final preflight =
          await _call(
                _response(
                  compiler: _failedCompiler(
                    runCount: 0,
                    code: 'COMPILER_PREFLIGHT_FAILED',
                    installRestore: 'not_started',
                    outputDisposition: 'not_created',
                  ),
                ),
              )
              as AuthoringRevision3ProjectCompilerCheckResult;
      expect(preflight.compiler.runCount, 0);

      final setupFailure =
          await _call(
                _response(
                  compiler: _failedCompiler(outputDisposition: 'not_created'),
                ),
              )
              as AuthoringRevision3ProjectCompilerCheckResult;
      expect(setupFailure.compiler.runCount, 1);
      expect(
        setupFailure.compiler.outputDisposition,
        AuthoringRevision3ProjectCompilerOutputDisposition.notCreated,
      );

      final enteredRunnerPreflight =
          await _call(
                _response(
                  compiler: _failedCompiler(
                    runCount: 1,
                    code: 'COMPILER_TRANSACTION_PREFLIGHT_FAILED',
                    installRestore: 'not_started',
                    outputDisposition: 'not_created',
                  ),
                ),
              )
              as AuthoringRevision3ProjectCompilerCheckResult;
      expect(enteredRunnerPreflight.compiler.runCount, 1);
      expect(
        enteredRunnerPreflight.compiler.installRestore,
        ScriptCompileInstallRestore.notStarted,
      );

      final invalid = _copy(
        _response(
          compiler: _failedCompiler(runCount: 0, installRestore: 'not_started'),
        ),
      );
      await _expectMalformed(invalid);

      final preflightWithDiagnostics = _copy(
        _response(
          compiler: _failedCompiler(
            runCount: 0,
            code: 'COMPILER_PREFLIGHT_FAILED',
            installRestore: 'not_started',
            outputDisposition: 'not_created',
            diagnostics: _diagnostics(),
          ),
        ),
      );
      await _expectMalformed(preflightWithDiagnostics);

      final preflightWithRestore = _copy(
        _response(
          compiler: _failedCompiler(
            runCount: 0,
            code: 'COMPILER_PREFLIGHT_FAILED',
            installRestore: 'restored_exact',
            outputDisposition: 'not_created',
          ),
        ),
      );
      await _expectMalformed(preflightWithRestore);
    },
  );

  test(
    'private output recovery is retained after exact install restore',
    () async {
      final result =
          await _call(
                _response(
                  compiler: _failedCompiler(
                    installRestore: 'restored_exact',
                    recoveryRequired: true,
                    outputDisposition: 'recovery_retained',
                  ),
                ),
              )
              as AuthoringRevision3ProjectCompilerCheckResult;

      expect(result.compiler.runCount, 1);
      expect(
        result.compiler.installRestore,
        ScriptCompileInstallRestore.restoredExact,
      );
      expect(result.recoveryRequired, isTrue);
      expect(result.exactCurrent, isTrue);
      expect(result.acceptedAtExactCurrent, isFalse);

      final simultaneousCompiler =
          _failedCompiler(
              installRestore: 'recovery_required_restore_failed',
              recoveryRequired: true,
              outputDisposition: 'recovery_retained',
            )
            ..['compiler_backend'] = <String, Object?>{
              'requested_mode': 'game',
              'result_backend': 'game',
              'standalone_attempted': false,
              'game_attempted': true,
              'qualified_package': null,
              'fallback_reason': null,
            };
      final simultaneous = await _callV2(
        _response(compiler: simultaneousCompiler),
        ScriptCompilerBackendMode.game,
      );
      expect(
        simultaneous.compiler.installRestore,
        ScriptCompileInstallRestore.recoveryRequiredRestoreFailed,
      );
      expect(
        simultaneous.compiler.outputDisposition,
        AuthoringRevision3ProjectCompilerOutputDisposition.recoveryRetained,
      );
      expect(simultaneous.recoveryRequired, isTrue);
    },
  );

  test('unconfirmed process exit always requires recovery', () async {
    final diagnostics = _diagnostics()
      ..['capture'] = 'process_exit_unconfirmed';
    await _expectMalformed(
      _response(compiler: _failedCompiler(diagnostics: diagnostics)),
    );
  });

  test('invalid local paths stop before native and never fall back', () async {
    final core = FakeGoreCoreFfiService(
      responses: <String, Map<String, Object?>>{_command: _response()},
    );
    final ffi = ModFfi(core);
    await expectLater(
      ffi.authoringStoreCheckRevision3ProjectCompilerV1(
        root: 'bad\u0000root',
        gameRoot: _gameRoot,
        expectedHead: AuthoringWorkingHead.fromCanonicalJson(_headJson()),
      ),
      throwsArgumentError,
    );
    expect(core.calls, isEmpty);
  });
}
