import 'dart:convert';

const _maxInstallStateDisplayPathBytes = 32 * 1024;
const _maxInstallStateIssueMessageBytes = 64 * 1024;
const _maxInstallStateArtifacts = 7;
const _maxInstallStateIssues = 8;

enum ScriptCompileInstallDisposition {
  safeToCompile,
  gameProcessRunning,
  recoveryArtifactsPresent,
  inspectionFailed,
}

enum ScriptCompileGameProcessState { notRunning, running, inspectionFailed }

enum ScriptCompileInstallArtifactKind {
  installMutationLock,
  compileLock,
  recoveryJournal,
  shippingCacheBackup,
  jittedCodeBackup,
  ue4ssProxyBackup,
  deployRecoveryRecord,
}

enum ScriptCompileInstallIssueKind {
  gameProcessEnumeration,
  artifactMetadata,
  deployRecoveryInspection,
}

final class ScriptCompileInstallArtifact {
  const ScriptCompileInstallArtifact({
    required this.kind,
    required this.displayPath,
    required this.pathTruncated,
  });

  final ScriptCompileInstallArtifactKind kind;
  final String displayPath;
  final bool pathTruncated;
}

final class ScriptCompileInstallIssue {
  const ScriptCompileInstallIssue({
    required this.kind,
    required this.displayPath,
    required this.message,
    required this.pathTruncated,
    required this.messageTruncated,
  });

  final ScriptCompileInstallIssueKind kind;
  final String? displayPath;
  final String message;
  final bool pathTruncated;
  final bool messageTruncated;
}

/// Closed, bounded read-only snapshot of whether a game installation can enter
/// a compiler/deploy mutation window.
final class ScriptCompileInstallState {
  const ScriptCompileInstallState._({
    required this.disposition,
    required this.safeToCompile,
    required this.gameProcess,
    required this.artifacts,
    required this.issues,
  });

  final ScriptCompileInstallDisposition disposition;
  final bool safeToCompile;
  final ScriptCompileGameProcessState gameProcess;
  final List<ScriptCompileInstallArtifact> artifacts;
  final List<ScriptCompileInstallIssue> issues;

  factory ScriptCompileInstallState.fromJson(Map<String, Object?> json) {
    const fields = <String>{
      'ok',
      'disposition',
      'safe_to_compile',
      'game_process',
      'artifacts',
      'issues',
    };
    if (json.length != fields.length || !fields.every(json.containsKey)) {
      throw const FormatException('compile install-state fields');
    }
    if (json['ok'] != true) {
      throw const FormatException('compile install-state ok');
    }
    final disposition = switch (json['disposition']) {
      'safe_to_compile' => ScriptCompileInstallDisposition.safeToCompile,
      'game_process_running' =>
        ScriptCompileInstallDisposition.gameProcessRunning,
      'recovery_artifacts_present' =>
        ScriptCompileInstallDisposition.recoveryArtifactsPresent,
      'inspection_failed' => ScriptCompileInstallDisposition.inspectionFailed,
      _ => throw const FormatException('compile install-state disposition'),
    };
    final safeToCompile = json['safe_to_compile'];
    if (safeToCompile is! bool ||
        safeToCompile !=
            (disposition == ScriptCompileInstallDisposition.safeToCompile)) {
      throw const FormatException('compile install-state safety invariant');
    }
    final gameProcess = switch (json['game_process']) {
      'not_running' => ScriptCompileGameProcessState.notRunning,
      'running' => ScriptCompileGameProcessState.running,
      'inspection_failed' => ScriptCompileGameProcessState.inspectionFailed,
      _ => throw const FormatException('compile install-state game process'),
    };
    final rawArtifacts = json['artifacts'];
    final rawIssues = json['issues'];
    if (rawArtifacts is! List<Object?> ||
        rawArtifacts.length > _maxInstallStateArtifacts ||
        rawIssues is! List<Object?> ||
        rawIssues.length > _maxInstallStateIssues) {
      throw const FormatException('compile install-state list bounds');
    }
    final artifactKinds = <ScriptCompileInstallArtifactKind>{};
    final artifacts = <ScriptCompileInstallArtifact>[];
    for (final raw in rawArtifacts) {
      final value = _exactObject(raw, const {
        'kind',
        'display_path',
        'path_truncated',
      }, 'compile install-state artifact');
      final kind = switch (value['kind']) {
        'install_mutation_lock' =>
          ScriptCompileInstallArtifactKind.installMutationLock,
        'compile_lock' => ScriptCompileInstallArtifactKind.compileLock,
        'recovery_journal' => ScriptCompileInstallArtifactKind.recoveryJournal,
        'shipping_cache_backup' =>
          ScriptCompileInstallArtifactKind.shippingCacheBackup,
        'jitted_code_backup' =>
          ScriptCompileInstallArtifactKind.jittedCodeBackup,
        'ue4ss_proxy_backup' =>
          ScriptCompileInstallArtifactKind.ue4ssProxyBackup,
        'deploy_recovery_record' =>
          ScriptCompileInstallArtifactKind.deployRecoveryRecord,
        _ => throw const FormatException('compile install-state artifact kind'),
      };
      if (!artifactKinds.add(kind)) {
        throw const FormatException(
          'compile install-state duplicate artifact kind',
        );
      }
      artifacts.add(
        ScriptCompileInstallArtifact(
          kind: kind,
          displayPath: _boundedString(
            value['display_path'],
            _maxInstallStateDisplayPathBytes,
            'compile install-state artifact path',
          ),
          pathTruncated: _requiredBool(
            value['path_truncated'],
            'compile install-state artifact truncation',
          ),
        ),
      );
    }
    final issues = <ScriptCompileInstallIssue>[];
    for (final raw in rawIssues) {
      final value = _exactObject(raw, const {
        'kind',
        'display_path',
        'message',
        'path_truncated',
        'message_truncated',
      }, 'compile install-state issue');
      final kind = switch (value['kind']) {
        'game_process_enumeration' =>
          ScriptCompileInstallIssueKind.gameProcessEnumeration,
        'artifact_metadata' => ScriptCompileInstallIssueKind.artifactMetadata,
        'deploy_recovery_inspection' =>
          ScriptCompileInstallIssueKind.deployRecoveryInspection,
        _ => throw const FormatException('compile install-state issue kind'),
      };
      final rawPath = value['display_path'];
      final displayPath = rawPath == null
          ? null
          : _boundedString(
              rawPath,
              _maxInstallStateDisplayPathBytes,
              'compile install-state issue path',
            );
      if (kind == ScriptCompileInstallIssueKind.artifactMetadata &&
          displayPath == null) {
        throw const FormatException(
          'compile install-state artifact issue path',
        );
      }
      issues.add(
        ScriptCompileInstallIssue(
          kind: kind,
          displayPath: displayPath,
          message: _boundedString(
            value['message'],
            _maxInstallStateIssueMessageBytes,
            'compile install-state issue message',
          ),
          pathTruncated: _requiredBool(
            value['path_truncated'],
            'compile install-state issue path truncation',
          ),
          messageTruncated: _requiredBool(
            value['message_truncated'],
            'compile install-state issue message truncation',
          ),
        ),
      );
    }

    if (safeToCompile &&
        (gameProcess != ScriptCompileGameProcessState.notRunning ||
            artifacts.isNotEmpty ||
            issues.isNotEmpty)) {
      throw const FormatException('safe compile install-state is not empty');
    }
    if (disposition == ScriptCompileInstallDisposition.gameProcessRunning &&
        gameProcess != ScriptCompileGameProcessState.running) {
      throw const FormatException('compile install-state running invariant');
    }
    if (gameProcess == ScriptCompileGameProcessState.inspectionFailed &&
        disposition != ScriptCompileInstallDisposition.inspectionFailed) {
      throw const FormatException(
        'compile install-state process inspection invariant',
      );
    }
    if (issues.isNotEmpty &&
        disposition != ScriptCompileInstallDisposition.inspectionFailed) {
      throw const FormatException(
        'compile install-state issue precedence invariant',
      );
    }
    if (disposition ==
            ScriptCompileInstallDisposition.recoveryArtifactsPresent &&
        artifacts.isEmpty) {
      throw const FormatException(
        'compile install-state recovery artifact invariant',
      );
    }
    if (disposition == ScriptCompileInstallDisposition.inspectionFailed &&
        issues.isEmpty) {
      throw const FormatException('compile install-state issue invariant');
    }

    return ScriptCompileInstallState._(
      disposition: disposition,
      safeToCompile: safeToCompile,
      gameProcess: gameProcess,
      artifacts: List.unmodifiable(artifacts),
      issues: List.unmodifiable(issues),
    );
  }
}

Map<String, Object?> _exactObject(
  Object? raw,
  Set<String> fields,
  String context,
) {
  if (raw is! Map ||
      raw.keys.any((key) => key is! String) ||
      raw.length != fields.length ||
      !fields.every(raw.containsKey)) {
    throw FormatException('$context fields');
  }
  return raw.cast<String, Object?>();
}

String _boundedString(Object? raw, int maxBytes, String context) {
  if (raw is! String ||
      raw.isEmpty ||
      raw.contains('\u0000') ||
      utf8.encode(raw).length > maxBytes) {
    throw FormatException('$context value');
  }
  return raw;
}

bool _requiredBool(Object? raw, String context) {
  if (raw is! bool) throw FormatException('$context value');
  return raw;
}
