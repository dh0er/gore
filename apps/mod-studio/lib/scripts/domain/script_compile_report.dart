import 'dart:convert';

const _maxScriptCompileDiagnostics = 4096;
const _maxScriptCompileDiagnosticFileBytes = 32 * 1024;
const _maxScriptCompileDiagnosticMessageBytes = 64 * 1024;
const _maxScriptCompileDiagnosticTextBytes = 4 * 1024 * 1024;
const _maxScriptCompileErrorMessageBytes = 64 * 1024;
const _maxScriptCompilePathBytes = 32 * 1024;
const _maxScriptCompileModuleBytes = 4 * 1024;
final _scriptCompileErrorCodePattern = RegExp(
  r'^[A-Z][A-Z0-9]*(?:_[A-Z0-9]+)*$',
);
const _preexistingRecoveryFailureCodes = <String>{
  'COMPILE_BASE_RECOVERY_REQUIRED',
  'COMPILE_INSTALL_RECOVERY_REQUIRED',
  'COMPILE_INSTALL_GUARD_RELEASE_FAILED',
};

enum ScriptCompileOutcome { compiled, failed }

enum ScriptCompileCaptureDisposition {
  captured,
  captureInvalid,
  unavailableFallback,
  unavailableWithoutFallback,
  processExitUnconfirmed,
  disabled,
}

enum ScriptCompileInstallRestore {
  notStarted,
  restoredExact,
  recoveryRequiredProcessExitUnconfirmed,
  recoveryRequiredRestoreFailed,
}

enum ScriptCompilerBackendMode {
  game('game', 'Game compiler'),
  standalone('standalone', 'Standalone compiler'),
  standaloneThenGame('standalone_then_game', 'Standalone, then game fallback');

  const ScriptCompilerBackendMode(this.wireName, this.label);

  final String wireName;
  final String label;

  /// Normal product policy: use the qualified standalone compiler first and
  /// start the game compiler only after a visible, structured fallback.
  static const productDefault = ScriptCompilerBackendMode.standaloneThenGame;
}

enum ScriptCompilerBackendName { game, standalone }

enum ScriptCompilerBackendFailureKind {
  unavailable,
  preflight,
  unsupported,
  rejected,
  invalidOutput,
  internal,
  recoveryRequired,
}

final class ScriptCompilerBackendFallback {
  const ScriptCompilerBackendFallback({
    required this.failedBackend,
    required this.failureKind,
    required this.detail,
  });

  final ScriptCompilerBackendName failedBackend;
  final ScriptCompilerBackendFailureKind failureKind;
  final String detail;
}

final class ScriptCompilerQualifiedTarget {
  const ScriptCompilerQualifiedTarget({
    required this.steamAppId,
    required this.steamBuildId,
    required this.depotId,
    required this.depotManifestGid,
    required this.platform,
    required this.architecture,
    required this.buildConfiguration,
    required this.codeViewGuid,
    required this.codeViewAge,
  });

  final int steamAppId;
  final int steamBuildId;
  final int depotId;
  final int depotManifestGid;
  final String platform;
  final String architecture;
  final String buildConfiguration;
  final String codeViewGuid;
  final int codeViewAge;
}

final class ScriptCompilerQualifiedPackage {
  const ScriptCompilerQualifiedPackage({
    required this.catalogSha256,
    required this.sidecarByteLen,
    required this.sidecarSha256,
    required this.requestVersion,
    required this.responseVersion,
    required this.manifestByteLen,
    required this.manifestSha256,
    required this.profileSha256,
    required this.target,
  });

  final String catalogSha256;
  final int sidecarByteLen;
  final String sidecarSha256;
  final int requestVersion;
  final int responseVersion;
  final int manifestByteLen;
  final String manifestSha256;
  final String profileSha256;
  final ScriptCompilerQualifiedTarget target;
}

final class ScriptCompilerBackendEvidence {
  const ScriptCompilerBackendEvidence._({
    required this.requestedMode,
    required this.resultBackend,
    required this.standaloneAttempted,
    required this.gameAttempted,
    required this.qualifiedPackage,
    required this.fallbackReason,
  });

  final ScriptCompilerBackendMode requestedMode;
  final ScriptCompilerBackendName? resultBackend;
  final bool standaloneAttempted;
  final bool gameAttempted;
  final ScriptCompilerQualifiedPackage? qualifiedPackage;
  final ScriptCompilerBackendFallback? fallbackReason;

  factory ScriptCompilerBackendEvidence.fromJson(
    Object? value, {
    required ScriptCompilerBackendMode expectedMode,
  }) {
    if (value is! Map ||
        value.length != 6 ||
        !value.containsKey('requested_mode') ||
        !value.containsKey('result_backend') ||
        !value.containsKey('standalone_attempted') ||
        !value.containsKey('game_attempted') ||
        !value.containsKey('qualified_package') ||
        !value.containsKey('fallback_reason')) {
      throw const FormatException('compiler backend evidence fields');
    }
    final requestedMode = switch (value['requested_mode']) {
      'game' => ScriptCompilerBackendMode.game,
      'standalone' => ScriptCompilerBackendMode.standalone,
      'standalone_then_game' => ScriptCompilerBackendMode.standaloneThenGame,
      _ => throw const FormatException('compiler backend requested mode'),
    };
    if (requestedMode != expectedMode) {
      throw const FormatException('compiler backend requested mode mismatch');
    }
    final resultBackend = switch (value['result_backend']) {
      null => null,
      'game' => ScriptCompilerBackendName.game,
      'standalone' => ScriptCompilerBackendName.standalone,
      _ => throw const FormatException('compiler backend result'),
    };
    final standaloneAttempted = value['standalone_attempted'];
    final gameAttempted = value['game_attempted'];
    if (standaloneAttempted is! bool || gameAttempted is! bool) {
      throw const FormatException('compiler backend attempt evidence');
    }
    final qualifiedPackage = _parseCompilerQualifiedPackage(
      value['qualified_package'],
    );
    final fallbackReason = _parseCompilerBackendFallback(
      value['fallback_reason'],
    );
    if ((resultBackend == ScriptCompilerBackendName.game && !gameAttempted) ||
        (resultBackend == ScriptCompilerBackendName.standalone &&
            !standaloneAttempted) ||
        (requestedMode == ScriptCompilerBackendMode.game &&
            (standaloneAttempted ||
                qualifiedPackage != null ||
                fallbackReason != null)) ||
        (requestedMode == ScriptCompilerBackendMode.standalone &&
            (gameAttempted || fallbackReason != null)) ||
        (requestedMode == ScriptCompilerBackendMode.standaloneThenGame &&
            resultBackend != ScriptCompilerBackendName.standalone &&
            fallbackReason == null &&
            (resultBackend != null || standaloneAttempted || gameAttempted)) ||
        (resultBackend == ScriptCompilerBackendName.standalone &&
            (gameAttempted || fallbackReason != null)) ||
        (standaloneAttempted && qualifiedPackage == null) ||
        (fallbackReason != null &&
            (requestedMode != ScriptCompilerBackendMode.standaloneThenGame ||
                fallbackReason.failedBackend !=
                    ScriptCompilerBackendName.standalone))) {
      throw const FormatException('compiler backend evidence invariant');
    }
    return ScriptCompilerBackendEvidence._(
      requestedMode: requestedMode,
      resultBackend: resultBackend,
      standaloneAttempted: standaloneAttempted,
      gameAttempted: gameAttempted,
      qualifiedPackage: qualifiedPackage,
      fallbackReason: fallbackReason,
    );
  }
}

ScriptCompilerQualifiedPackage? _parseCompilerQualifiedPackage(Object? value) {
  if (value == null) return null;
  if (value is! Map ||
      value.length != 9 ||
      !value.containsKey('catalog_sha256') ||
      !value.containsKey('sidecar_byte_len') ||
      !value.containsKey('sidecar_sha256') ||
      !value.containsKey('request_version') ||
      !value.containsKey('response_version') ||
      !value.containsKey('manifest_byte_len') ||
      !value.containsKey('manifest_sha256') ||
      !value.containsKey('profile_sha256') ||
      !value.containsKey('target')) {
    throw const FormatException('compiler backend package fields');
  }
  final catalogSha256 = _qualifiedSha256(
    value['catalog_sha256'],
    'catalog digest',
  );
  final sidecarSha256 = _qualifiedSha256(
    value['sidecar_sha256'],
    'sidecar digest',
  );
  final manifestSha256 = _qualifiedSha256(
    value['manifest_sha256'],
    'manifest digest',
  );
  final profileSha256 = _qualifiedSha256(
    value['profile_sha256'],
    'profile digest',
  );
  final sidecarByteLen = _positiveBoundedInt(
    value['sidecar_byte_len'],
    512 * 1024 * 1024,
    'sidecar length',
  );
  final manifestByteLen = _positiveBoundedInt(
    value['manifest_byte_len'],
    4 * 1024 * 1024,
    'manifest length',
  );
  final requestVersion = value['request_version'];
  final responseVersion = value['response_version'];
  if (requestVersion is! int ||
      (requestVersion != 1 && requestVersion != 2) ||
      responseVersion != 1) {
    throw const FormatException('compiler backend package protocol');
  }
  return ScriptCompilerQualifiedPackage(
    catalogSha256: catalogSha256,
    sidecarByteLen: sidecarByteLen,
    sidecarSha256: sidecarSha256,
    requestVersion: requestVersion,
    responseVersion: responseVersion as int,
    manifestByteLen: manifestByteLen,
    manifestSha256: manifestSha256,
    profileSha256: profileSha256,
    target: _parseCompilerQualifiedTarget(value['target']),
  );
}

ScriptCompilerQualifiedTarget _parseCompilerQualifiedTarget(Object? value) {
  if (value is! Map ||
      value.length != 2 ||
      !value.containsKey('target') ||
      !value.containsKey('pe_codeview')) {
    throw const FormatException('compiler backend package target fields');
  }
  final target = value['target'];
  final codeView = value['pe_codeview'];
  if (target is! Map ||
      target.length != 7 ||
      !target.containsKey('steam_app_id') ||
      !target.containsKey('steam_build_id') ||
      !target.containsKey('depot_id') ||
      !target.containsKey('depot_manifest_gid') ||
      !target.containsKey('platform') ||
      !target.containsKey('architecture') ||
      !target.containsKey('build_configuration') ||
      codeView is! Map ||
      codeView.length != 2 ||
      !codeView.containsKey('guid') ||
      !codeView.containsKey('age')) {
    throw const FormatException('compiler backend package target shape');
  }
  final guid = codeView['guid'];
  final guidPattern = RegExp(
    r'^[0-9A-Fa-f]{8}-[0-9A-Fa-f]{4}-[0-9A-Fa-f]{4}-[0-9A-Fa-f]{4}-[0-9A-Fa-f]{12}$',
  );
  if (target['platform'] != 'windows' ||
      target['architecture'] != 'x86_64' ||
      target['build_configuration'] != 'shipping' ||
      guid is! String ||
      !guidPattern.hasMatch(guid)) {
    throw const FormatException('compiler backend package target identity');
  }
  return ScriptCompilerQualifiedTarget(
    steamAppId: _positiveBoundedInt(
      target['steam_app_id'],
      0xffffffff,
      'Steam app id',
    ),
    steamBuildId: _positiveBoundedInt(
      target['steam_build_id'],
      0x7fffffffffffffff,
      'Steam build id',
    ),
    depotId: _positiveBoundedInt(
      target['depot_id'],
      0xffffffff,
      'Steam depot id',
    ),
    depotManifestGid: _positiveBoundedInt(
      target['depot_manifest_gid'],
      0x7fffffffffffffff,
      'Steam depot manifest',
    ),
    platform: target['platform'] as String,
    architecture: target['architecture'] as String,
    buildConfiguration: target['build_configuration'] as String,
    codeViewGuid: guid,
    codeViewAge: _positiveBoundedInt(
      codeView['age'],
      0xffffffff,
      'CodeView age',
    ),
  );
}

String _qualifiedSha256(Object? value, String label) {
  if (value is! String ||
      !RegExp(r'^[0-9a-f]{64}$').hasMatch(value) ||
      RegExp(r'^0{64}$').hasMatch(value)) {
    throw FormatException('compiler backend package $label');
  }
  return value;
}

int _positiveBoundedInt(Object? value, int max, String label) {
  if (value is! int || value <= 0 || value > max) {
    throw FormatException('compiler backend package $label');
  }
  return value;
}

ScriptCompilerBackendFallback? _parseCompilerBackendFallback(Object? value) {
  if (value == null) return null;
  if (value is! Map ||
      value.length != 3 ||
      !value.containsKey('failed_backend') ||
      !value.containsKey('failure_kind') ||
      !value.containsKey('detail')) {
    throw const FormatException('compiler backend fallback fields');
  }
  final failedBackend = switch (value['failed_backend']) {
    'game' => ScriptCompilerBackendName.game,
    'standalone' => ScriptCompilerBackendName.standalone,
    _ => throw const FormatException('compiler backend fallback backend'),
  };
  final failureKind = switch (value['failure_kind']) {
    'unavailable' => ScriptCompilerBackendFailureKind.unavailable,
    'preflight' => ScriptCompilerBackendFailureKind.preflight,
    'unsupported' => ScriptCompilerBackendFailureKind.unsupported,
    'rejected' => ScriptCompilerBackendFailureKind.rejected,
    'invalid_output' => ScriptCompilerBackendFailureKind.invalidOutput,
    'internal' => ScriptCompilerBackendFailureKind.internal,
    'recovery_required' => ScriptCompilerBackendFailureKind.recoveryRequired,
    _ => throw const FormatException('compiler backend fallback kind'),
  };
  final detail = _optionalBoundedString(
    value['detail'],
    4096,
    allowEmpty: false,
  );
  if (detail == null) {
    throw const FormatException('compiler backend fallback detail');
  }
  return ScriptCompilerBackendFallback(
    failedBackend: failedBackend,
    failureKind: failureKind,
    detail: detail,
  );
}

enum ScriptCompilerDiagnosticSeverity { error, warning, note }

class ScriptCompilerDiagnostic {
  const ScriptCompilerDiagnostic({
    required this.file,
    required this.line,
    required this.column,
    required this.severity,
    required this.message,
  });

  final String file;
  final int line;
  final int column;
  final ScriptCompilerDiagnosticSeverity severity;
  final String message;

  String get location {
    final suffix = column == 0 ? '$line' : '$line,$column';
    return file.isEmpty ? suffix : '$file($suffix)';
  }
}

class ScriptCompilerDiagnostics {
  const ScriptCompilerDiagnostics({
    required this.capture,
    required this.messages,
    required this.omitted,
  });

  final ScriptCompileCaptureDisposition capture;
  final List<ScriptCompilerDiagnostic> messages;
  final int omitted;

  bool get usedNormalFallback =>
      capture == ScriptCompileCaptureDisposition.unavailableFallback;
}

class ScriptCompileFailure {
  const ScriptCompileFailure({required this.code, required this.message});

  final String code;
  final String message;
}

/// Whether recovery evidence concerns the selected game installation.
///
/// Full-graph compiler reports also use `recoveryRequired` when a private
/// compiler output could not be removed. That output must still block adoption
/// of the compiler result, but it does not make the game installation unsafe.
bool scriptCompileRequiresGameInstallRecovery({
  required bool recoveryRequired,
  required ScriptCompileInstallRestore installRestore,
  required ScriptCompileFailure? failure,
}) {
  if (!recoveryRequired) return false;
  return switch (installRestore) {
    ScriptCompileInstallRestore.recoveryRequiredProcessExitUnconfirmed ||
    ScriptCompileInstallRestore.recoveryRequiredRestoreFailed => true,
    ScriptCompileInstallRestore.restoredExact => false,
    ScriptCompileInstallRestore.notStarted =>
      failure != null &&
          _preexistingRecoveryFailureCodes.contains(failure.code),
  };
}

/// Closed, bounded projection of one transactional game-compiler attempt.
///
/// A failed compiler attempt is data, not a transport exception. In particular, callers can show
/// syntax diagnostics and independently decide whether installation recovery is required.
class ScriptCompileReport {
  const ScriptCompileReport._({
    required this.outcome,
    required this.miniPath,
    required this.module,
    required this.failure,
    required this.diagnostics,
    required this.installRestore,
    required this.recoveryRequired,
    required this.backend,
  });

  final ScriptCompileOutcome outcome;
  final String? miniPath;
  final String? module;
  final ScriptCompileFailure? failure;
  final ScriptCompilerDiagnostics? diagnostics;
  final ScriptCompileInstallRestore installRestore;
  final bool recoveryRequired;
  final ScriptCompilerBackendEvidence? backend;

  bool get compiled => outcome == ScriptCompileOutcome.compiled;

  bool get gameInstallRecoveryRequired =>
      scriptCompileRequiresGameInstallRecovery(
        recoveryRequired: recoveryRequired,
        installRestore: installRestore,
        failure: failure,
      );

  factory ScriptCompileReport.fromJson(
    Map<String, Object?> json, {
    ScriptCompilerBackendMode? expectedBackend,
  }) {
    final fields = <String>{
      'ok',
      'outcome',
      'mini_path',
      'module',
      'compile_error',
      'compiler_diagnostics',
      'install_restore',
      'recovery_required',
    };
    if (expectedBackend != null) fields.add('compiler_backend');
    if (json.length != fields.length || !fields.every(json.containsKey)) {
      throw const FormatException('compile report fields');
    }
    if (json['ok'] != true) {
      throw const FormatException('compile report ok');
    }
    final outcome = switch (json['outcome']) {
      'compiled' => ScriptCompileOutcome.compiled,
      'failed' => ScriptCompileOutcome.failed,
      _ => throw const FormatException('compile report outcome'),
    };
    final installRestore = switch (json['install_restore']) {
      'not_started' => ScriptCompileInstallRestore.notStarted,
      'restored_exact' => ScriptCompileInstallRestore.restoredExact,
      'recovery_required_process_exit_unconfirmed' =>
        ScriptCompileInstallRestore.recoveryRequiredProcessExitUnconfirmed,
      'recovery_required_restore_failed' =>
        ScriptCompileInstallRestore.recoveryRequiredRestoreFailed,
      _ => throw const FormatException('compile report restore'),
    };
    final recoveryRequired = json['recovery_required'];
    if (recoveryRequired is! bool) {
      throw const FormatException('compile report recovery');
    }
    final miniPath = _optionalBoundedString(
      json['mini_path'],
      _maxScriptCompilePathBytes,
      allowEmpty: false,
    );
    final module = _optionalBoundedString(
      json['module'],
      _maxScriptCompileModuleBytes,
      allowEmpty: false,
    );
    final failure = _parseFailure(json['compile_error']);
    final diagnostics = _parseDiagnostics(json['compiler_diagnostics']);
    final backend = expectedBackend == null
        ? null
        : ScriptCompilerBackendEvidence.fromJson(
            json['compiler_backend'],
            expectedMode: expectedBackend,
          );

    final restoreRequiresRecovery =
        installRestore ==
            ScriptCompileInstallRestore
                .recoveryRequiredProcessExitUnconfirmed ||
        installRestore ==
            ScriptCompileInstallRestore.recoveryRequiredRestoreFailed;
    final preexistingRecovery =
        installRestore == ScriptCompileInstallRestore.notStarted &&
        outcome == ScriptCompileOutcome.failed &&
        failure != null &&
        _preexistingRecoveryFailureCodes.contains(failure.code);
    if (recoveryRequired != (restoreRequiresRecovery || preexistingRecovery)) {
      throw const FormatException('compile report recovery');
    }

    if (outcome == ScriptCompileOutcome.compiled) {
      if (miniPath == null ||
          module == null ||
          failure != null ||
          diagnostics == null ||
          diagnostics.messages.any(
            (message) =>
                message.severity == ScriptCompilerDiagnosticSeverity.error,
          ) ||
          diagnostics.capture ==
              ScriptCompileCaptureDisposition.captureInvalid ||
          diagnostics.capture ==
              ScriptCompileCaptureDisposition.unavailableWithoutFallback ||
          diagnostics.capture == ScriptCompileCaptureDisposition.disabled ||
          diagnostics.capture ==
              ScriptCompileCaptureDisposition.processExitUnconfirmed ||
          (installRestore != ScriptCompileInstallRestore.restoredExact &&
              !(installRestore == ScriptCompileInstallRestore.notStarted &&
                  backend?.resultBackend ==
                      ScriptCompilerBackendName.standalone)) ||
          recoveryRequired) {
        throw const FormatException('compiled report invariant');
      }
    } else if (miniPath != null || module != null || failure == null) {
      throw const FormatException('failed report invariant');
    }
    if (diagnostics?.capture ==
            ScriptCompileCaptureDisposition.processExitUnconfirmed &&
        !recoveryRequired) {
      throw const FormatException('diagnostic recovery invariant');
    }

    return ScriptCompileReport._(
      outcome: outcome,
      miniPath: miniPath,
      module: module,
      failure: failure,
      diagnostics: diagnostics,
      installRestore: installRestore,
      recoveryRequired: recoveryRequired,
      backend: backend,
    );
  }
}

ScriptCompileFailure? _parseFailure(Object? value) {
  if (value == null) return null;
  if (value is! Map ||
      value.length != 2 ||
      !value.containsKey('code') ||
      !value.containsKey('message')) {
    throw const FormatException('compile failure fields');
  }
  final code = value['code'];
  final message = value['message'];
  if (code is! String ||
      code.isEmpty ||
      utf8.encode(code).length > 128 ||
      !_scriptCompileErrorCodePattern.hasMatch(code) ||
      message is! String ||
      message.trim().isEmpty ||
      utf8.encode(message).length > _maxScriptCompileErrorMessageBytes) {
    throw const FormatException('compile failure values');
  }
  return ScriptCompileFailure(code: code, message: message);
}

ScriptCompilerDiagnostics? _parseDiagnostics(Object? value) {
  if (value == null) return null;
  if (value is! Map ||
      value.length != 3 ||
      !value.containsKey('capture') ||
      !value.containsKey('messages') ||
      !value.containsKey('omitted')) {
    throw const FormatException('compiler diagnostics fields');
  }
  final capture = switch (value['capture']) {
    'captured' => ScriptCompileCaptureDisposition.captured,
    'capture_invalid' => ScriptCompileCaptureDisposition.captureInvalid,
    'unavailable_fallback' =>
      ScriptCompileCaptureDisposition.unavailableFallback,
    'unavailable_without_fallback' =>
      ScriptCompileCaptureDisposition.unavailableWithoutFallback,
    'process_exit_unconfirmed' =>
      ScriptCompileCaptureDisposition.processExitUnconfirmed,
    'disabled' => ScriptCompileCaptureDisposition.disabled,
    _ => throw const FormatException('compiler diagnostics capture'),
  };
  final rawMessages = value['messages'];
  final omitted = value['omitted'];
  if (rawMessages is! List ||
      rawMessages.length > _maxScriptCompileDiagnostics ||
      omitted is! int ||
      omitted < 0) {
    throw const FormatException('compiler diagnostics bounds');
  }
  var textBytes = 0;
  final messages = <ScriptCompilerDiagnostic>[];
  for (final raw in rawMessages) {
    if (raw is! Map ||
        raw.length != 5 ||
        !raw.containsKey('file') ||
        !raw.containsKey('line') ||
        !raw.containsKey('column') ||
        !raw.containsKey('severity') ||
        !raw.containsKey('message')) {
      throw const FormatException('compiler diagnostic fields');
    }
    final file = raw['file'];
    final line = raw['line'];
    final column = raw['column'];
    final message = raw['message'];
    final severity = switch (raw['severity']) {
      'error' => ScriptCompilerDiagnosticSeverity.error,
      'warning' => ScriptCompilerDiagnosticSeverity.warning,
      'note' => ScriptCompilerDiagnosticSeverity.note,
      _ => throw const FormatException('compiler diagnostic severity'),
    };
    if (file is! String ||
        utf8.encode(file).length > _maxScriptCompileDiagnosticFileBytes ||
        line is! int ||
        line < 0 ||
        line > 0xffffffff ||
        column is! int ||
        column < 0 ||
        column > 0xffffffff ||
        message is! String ||
        utf8.encode(message).length > _maxScriptCompileDiagnosticMessageBytes) {
      throw const FormatException('compiler diagnostic values');
    }
    textBytes += utf8.encode(file).length + utf8.encode(message).length;
    if (textBytes > _maxScriptCompileDiagnosticTextBytes) {
      throw const FormatException('compiler diagnostic text budget');
    }
    messages.add(
      ScriptCompilerDiagnostic(
        file: file,
        line: line,
        column: column,
        severity: severity,
        message: message,
      ),
    );
  }
  return ScriptCompilerDiagnostics(
    capture: capture,
    messages: List.unmodifiable(messages),
    omitted: omitted,
  );
}

String? _optionalBoundedString(
  Object? value,
  int maxBytes, {
  required bool allowEmpty,
}) {
  if (value == null) return null;
  if (value is! String ||
      (!allowEmpty && value.isEmpty) ||
      utf8.encode(value).length > maxBytes ||
      value.contains('\u0000')) {
    throw const FormatException('bounded string');
  }
  return value;
}
