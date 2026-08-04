import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/scripts/domain/script_compile_install_state.dart';

Map<String, Object?> _state({
  String disposition = 'safe_to_compile',
  bool safe = true,
  String gameProcess = 'not_running',
  List<Object?> artifacts = const [],
  List<Object?> issues = const [],
}) => <String, Object?>{
  'ok': true,
  'disposition': disposition,
  'safe_to_compile': safe,
  'game_process': gameProcess,
  'artifacts': artifacts,
  'issues': issues,
};

Map<String, Object?> _artifact({
  String kind = 'recovery_journal',
  String path = r'C:\Game\.gore-compile-recovery.json',
}) => <String, Object?>{
  'kind': kind,
  'display_path': path,
  'path_truncated': false,
};

Map<String, Object?> _issue({
  String kind = 'artifact_metadata',
  Object? path = r'C:\Game\.gore-compile-recovery.json',
}) => <String, Object?>{
  'kind': kind,
  'display_path': path,
  'message': 'metadata could not be inspected',
  'path_truncated': false,
  'message_truncated': false,
};

void main() {
  test('parses the unique empty safe state', () {
    final state = ScriptCompileInstallState.fromJson(_state());

    expect(state.safeToCompile, isTrue);
    expect(state.disposition, ScriptCompileInstallDisposition.safeToCompile);
    expect(state.gameProcess, ScriptCompileGameProcessState.notRunning);
    expect(state.artifacts, isEmpty);
    expect(state.issues, isEmpty);
  });

  test('parses recovery evidence as unsafe', () {
    final state = ScriptCompileInstallState.fromJson(
      _state(
        disposition: 'recovery_artifacts_present',
        safe: false,
        artifacts: [_artifact()],
      ),
    );

    expect(state.safeToCompile, isFalse);
    expect(
      state.artifacts.single.kind,
      ScriptCompileInstallArtifactKind.recoveryJournal,
    );
  });

  test('inspection failure dominates a concurrently running process', () {
    final state = ScriptCompileInstallState.fromJson(
      _state(
        disposition: 'inspection_failed',
        safe: false,
        gameProcess: 'running',
        issues: [_issue()],
      ),
    );

    expect(state.disposition, ScriptCompileInstallDisposition.inspectionFailed);
    expect(state.gameProcess, ScriptCompileGameProcessState.running);
    expect(
      state.issues.single.kind,
      ScriptCompileInstallIssueKind.artifactMetadata,
    );
  });

  test('keeps lower-priority artifacts visible while the game is running', () {
    final running = ScriptCompileInstallState.fromJson(
      _state(
        disposition: 'game_process_running',
        safe: false,
        gameProcess: 'running',
        artifacts: [_artifact(kind: 'compile_lock')],
      ),
    );
    final deployRecovery = ScriptCompileInstallState.fromJson(
      _state(
        disposition: 'recovery_artifacts_present',
        safe: false,
        gameProcess: 'running',
        artifacts: [_artifact(kind: 'deploy_recovery_record')],
      ),
    );

    expect(
      running.disposition,
      ScriptCompileInstallDisposition.gameProcessRunning,
    );
    expect(
      deployRecovery.disposition,
      ScriptCompileInstallDisposition.recoveryArtifactsPresent,
    );
  });

  test('rejects unknown fields and contradictory safety states', () {
    final extra = _state()..['extra'] = true;
    expect(
      () => ScriptCompileInstallState.fromJson(extra),
      throwsFormatException,
    );
    expect(
      () => ScriptCompileInstallState.fromJson(_state(safe: false)),
      throwsFormatException,
    );
    expect(
      () => ScriptCompileInstallState.fromJson(
        _state(disposition: 'recovery_artifacts_present', safe: false),
      ),
      throwsFormatException,
    );
    expect(
      () => ScriptCompileInstallState.fromJson(
        _state(disposition: 'inspection_failed', safe: false),
      ),
      throwsFormatException,
    );
    expect(
      () => ScriptCompileInstallState.fromJson(
        _state(
          disposition: 'game_process_running',
          safe: false,
          gameProcess: 'running',
          issues: [_issue()],
        ),
      ),
      throwsFormatException,
    );
  });

  test('rejects duplicate artifacts and pathless metadata issues', () {
    expect(
      () => ScriptCompileInstallState.fromJson(
        _state(
          disposition: 'recovery_artifacts_present',
          safe: false,
          artifacts: [_artifact(), _artifact()],
        ),
      ),
      throwsFormatException,
    );
    expect(
      () => ScriptCompileInstallState.fromJson(
        _state(
          disposition: 'inspection_failed',
          safe: false,
          issues: [_issue(path: null)],
        ),
      ),
      throwsFormatException,
    );
  });
}
