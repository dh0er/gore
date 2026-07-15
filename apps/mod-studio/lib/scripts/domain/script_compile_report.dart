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
  });

  final ScriptCompileOutcome outcome;
  final String? miniPath;
  final String? module;
  final ScriptCompileFailure? failure;
  final ScriptCompilerDiagnostics? diagnostics;
  final ScriptCompileInstallRestore installRestore;
  final bool recoveryRequired;

  bool get compiled => outcome == ScriptCompileOutcome.compiled;

  factory ScriptCompileReport.fromJson(Map<String, Object?> json) {
    const fields = {
      'ok',
      'outcome',
      'mini_path',
      'module',
      'compile_error',
      'compiler_diagnostics',
      'install_restore',
      'recovery_required',
    };
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
          installRestore != ScriptCompileInstallRestore.restoredExact ||
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
