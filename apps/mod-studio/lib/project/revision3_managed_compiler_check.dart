part of '../core/mod_ffi.dart';

const _maxRevision3ManagedCompilerDiagnostics = 4096;
const _maxRevision3ManagedCompilerDiagnosticFileBytes = 32 * 1024;
const _maxRevision3ManagedCompilerDiagnosticMessageBytes = 64 * 1024;
const _maxRevision3ManagedCompilerDiagnosticTextBytes = 4 * 1024 * 1024;
const _maxRevision3ManagedCompilerFailureMessageBytes = 64 * 1024;
const _maxRevision3ManagedCompilerModuleNamespaceBytes = 255;
const _maxRevision3ManagedCompilerModulePathBytes = 258;
const _maxRevision3ManagedCompilerCoordinate = 0xffffffff;
const _maxRevision3ManagedCompilerOmitted = 0xffffffff;
const _revision3ManagedCompilerZeroId = '00000000000000000000000000000000';
const _revision3ManagedCompilerPreexistingRecoveryCodes = <String>{
  'COMPILE_BASE_RECOVERY_REQUIRED',
  'COMPILE_INSTALL_RECOVERY_REQUIRED',
  'COMPILE_INSTALL_GUARD_RELEASE_FAILED',
};

enum AuthoringRevision3ManagedCompilerEntityKind {
  questDraft('quest_draft'),
  npcDraft('npc_draft');

  const AuthoringRevision3ManagedCompilerEntityKind(this.wireName);
  final String wireName;
}

enum AuthoringRevision3ManagedCompilerOutcome { compiledEvidenceOnly, failed }

enum AuthoringRevision3ManagedCompilerScope { compilerCheckOnly }

enum AuthoringRevision3ManagedCompilerBuildStatus { blocked }

enum AuthoringRevision3ManagedCompilerDeployStatus { notSupported }

enum AuthoringRevision3ManagedCompilerRuntimeQualification {
  runtimeUnqualified,
}

enum AuthoringRevision3ManagedCompilerPublicationStatus { notSupported }

/// Exact Store snapshot that supplied the source checked by the compiler.
final class AuthoringRevision3ManagedCompilerProjectEvidence {
  const AuthoringRevision3ManagedCompilerProjectEvidence._({
    required this.id,
    required this.revision,
    required this.seal,
  });

  final String id;
  final int revision;
  final AuthoringDraftContentSeal seal;

  factory AuthoringRevision3ManagedCompilerProjectEvidence._fromJson(
    Object? value,
  ) {
    final json = _authoringRequiredObject(
      value,
      'revision-3 managed compiler project evidence',
    );
    _authoringExactFields(json, const <String>{
      'id',
      'revision',
      'seal',
    }, 'revision-3 managed compiler project evidence');
    final seal = AuthoringDraftContentSeal.fromJson(
      _authoringRequiredObject(
        json['seal'],
        'revision-3 managed compiler project seal',
      ),
    );
    if (seal.byteLength > _maxAuthoringProjectJsonBytes) {
      throw const FormatException(
        'authoring revision-3 managed compiler project seal is too large',
      );
    }
    return AuthoringRevision3ManagedCompilerProjectEvidence._(
      id: _authoringRevision3ManagedCompilerEntityId(json['id'], 'project.id'),
      revision: _authoringRevision3ManagedCompilerRevision(
        json['revision'],
        'project.revision',
      ),
      seal: seal,
    );
  }
}

/// Exact selected Quest/NPC identity and entity revision checked by native.
final class AuthoringRevision3ManagedCompilerEntityEvidence {
  const AuthoringRevision3ManagedCompilerEntityEvidence._({
    required this.kind,
    required this.id,
    required this.revision,
  });

  final AuthoringRevision3ManagedCompilerEntityKind kind;
  final String id;
  final int revision;

  factory AuthoringRevision3ManagedCompilerEntityEvidence._fromJson(
    Object? value, {
    required AuthoringRevision3ManagedCompilerEntityKind expectedKind,
    required String requestedEntityId,
  }) {
    final json = _authoringRequiredObject(
      value,
      'revision-3 managed compiler entity evidence',
    );
    _authoringExactFields(json, const <String>{
      'kind',
      'id',
      'revision',
    }, 'revision-3 managed compiler entity evidence');
    if (json['kind'] != expectedKind.wireName) {
      throw const FormatException(
        'authoring revision-3 managed compiler returned another entity kind',
      );
    }
    final id = _authoringRevision3ManagedCompilerEntityId(
      json['id'],
      'entity.id',
    );
    if (id != requestedEntityId) {
      throw const FormatException(
        'authoring revision-3 managed compiler returned another entity',
      );
    }
    return AuthoringRevision3ManagedCompilerEntityEvidence._(
      kind: expectedKind,
      id: id,
      revision: _authoringRevision3ManagedCompilerRevision(
        json['revision'],
        'entity.revision',
      ),
    );
  }
}

/// Native-derived ScriptModule identity. No source or compiler artifact crosses
/// this evidence-only boundary.
final class AuthoringRevision3ManagedCompilerModuleEvidence {
  const AuthoringRevision3ManagedCompilerModuleEvidence._({
    required this.id,
    required this.revision,
    required this.namespace,
    required this.relativePath,
    required this.sourceSha256,
  });

  final String id;
  final int revision;
  final String namespace;
  final String relativePath;
  final String sourceSha256;

  factory AuthoringRevision3ManagedCompilerModuleEvidence._fromJson(
    Object? value, {
    required String selectedEntityId,
  }) {
    final json = _authoringRequiredObject(
      value,
      'revision-3 managed compiler module evidence',
    );
    _authoringExactFields(json, const <String>{
      'id',
      'revision',
      'namespace',
      'relative_path',
      'source_sha256',
    }, 'revision-3 managed compiler module evidence');
    final id = _authoringRevision3ManagedCompilerEntityId(
      json['id'],
      'module.id',
    );
    if (id == selectedEntityId) {
      throw const FormatException(
        'authoring revision-3 managed compiler module aliases its owner',
      );
    }
    final namespace = _authoringRevision3ManagedCompilerString(
      json['namespace'],
      'module.namespace',
      maxBytes: _maxRevision3ManagedCompilerModuleNamespaceBytes,
    );
    _authoringDraftValidateModuleNamespace(namespace);
    final relativePath = _authoringRevision3ManagedCompilerString(
      json['relative_path'],
      'module.relative_path',
      maxBytes: _maxRevision3ManagedCompilerModulePathBytes,
    );
    if (relativePath != '${namespace.replaceAll('.', '/')}.as') {
      throw const FormatException(
        'authoring revision-3 managed compiler module path does not match its namespace',
      );
    }
    final sourceSha256 = _authoringRevision3ManagedCompilerString(
      json['source_sha256'],
      'module.source_sha256',
      maxBytes: 64,
    );
    if (!_authoringSha256Pattern.hasMatch(sourceSha256)) {
      throw const FormatException(
        'authoring revision-3 managed compiler source seal is not canonical',
      );
    }
    return AuthoringRevision3ManagedCompilerModuleEvidence._(
      id: id,
      revision: _authoringRevision3ManagedCompilerRevision(
        json['revision'],
        'module.revision',
      ),
      namespace: namespace,
      relativePath: relativePath,
      sourceSha256: sourceSha256,
    );
  }
}

/// Bounded evidence from one transactional game-compiler attempt.
///
/// Even a compiled outcome is deliberately not an artifact: native discarded
/// the mini-cache before returning this report.
final class AuthoringRevision3ManagedCompilerEvidence {
  const AuthoringRevision3ManagedCompilerEvidence._({
    required this.outcome,
    required this.failure,
    required this.diagnostics,
    required this.installRestore,
    required this.recoveryRequired,
    required this.outputDiscarded,
    required this.backend,
  });

  final AuthoringRevision3ManagedCompilerOutcome outcome;
  final ScriptCompileFailure? failure;
  final ScriptCompilerDiagnostics? diagnostics;
  final ScriptCompileInstallRestore installRestore;
  final bool recoveryRequired;
  final bool outputDiscarded;
  final ScriptCompilerBackendEvidence? backend;

  bool get compiledEvidenceOnly =>
      outcome == AuthoringRevision3ManagedCompilerOutcome.compiledEvidenceOnly;

  bool get gameInstallRecoveryRequired =>
      scriptCompileRequiresGameInstallRecovery(
        recoveryRequired: recoveryRequired,
        installRestore: installRestore,
        failure: failure,
      );

  factory AuthoringRevision3ManagedCompilerEvidence._fromJson(
    Object? value, {
    ScriptCompilerBackendMode? expectedBackend,
  }) {
    final json = _authoringRequiredObject(
      value,
      'revision-3 managed compiler evidence',
    );
    final fields = <String>{
      'outcome',
      'compile_error',
      'compiler_diagnostics',
      'install_restore',
      'recovery_required',
      'output_discarded',
    };
    if (expectedBackend != null) fields.add('compiler_backend');
    _authoringExactFields(json, fields, 'revision-3 managed compiler evidence');
    final outcome = switch (json['outcome']) {
      'compiled_evidence_only' =>
        AuthoringRevision3ManagedCompilerOutcome.compiledEvidenceOnly,
      'failed' => AuthoringRevision3ManagedCompilerOutcome.failed,
      _ => throw const FormatException(
        'authoring revision-3 managed compiler outcome is unsupported',
      ),
    };
    final failure = _authoringRevision3ManagedCompilerFailure(
      json['compile_error'],
    );
    final diagnostics = _authoringRevision3ManagedCompilerDiagnostics(
      json['compiler_diagnostics'],
    );
    final installRestore = switch (json['install_restore']) {
      'not_started' => ScriptCompileInstallRestore.notStarted,
      'restored_exact' => ScriptCompileInstallRestore.restoredExact,
      'recovery_required_process_exit_unconfirmed' =>
        ScriptCompileInstallRestore.recoveryRequiredProcessExitUnconfirmed,
      'recovery_required_restore_failed' =>
        ScriptCompileInstallRestore.recoveryRequiredRestoreFailed,
      _ => throw const FormatException(
        'authoring revision-3 managed compiler restore disposition is unsupported',
      ),
    };
    final recoveryRequired = json['recovery_required'];
    final outputDiscarded = json['output_discarded'];
    if (recoveryRequired is! bool || outputDiscarded is! bool) {
      throw const FormatException(
        'authoring revision-3 managed compiler output disposal or recovery state is invalid',
      );
    }
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
        outcome == AuthoringRevision3ManagedCompilerOutcome.failed &&
        failure != null &&
        _revision3ManagedCompilerPreexistingRecoveryCodes.contains(
          failure.code,
        );
    if (recoveryRequired != (restoreRequiresRecovery || preexistingRecovery)) {
      throw const FormatException(
        'authoring revision-3 managed compiler recovery evidence disagrees',
      );
    }
    if (outcome ==
        AuthoringRevision3ManagedCompilerOutcome.compiledEvidenceOnly) {
      if (failure != null ||
          diagnostics == null ||
          diagnostics.messages.any(
            (message) =>
                message.severity == ScriptCompilerDiagnosticSeverity.error,
          ) ||
          (diagnostics.capture != ScriptCompileCaptureDisposition.captured &&
              diagnostics.capture !=
                  ScriptCompileCaptureDisposition.unavailableFallback) ||
          (installRestore != ScriptCompileInstallRestore.restoredExact &&
              !(installRestore == ScriptCompileInstallRestore.notStarted &&
                  backend?.resultBackend ==
                      ScriptCompilerBackendName.standalone)) ||
          recoveryRequired ||
          !outputDiscarded) {
        throw const FormatException(
          'authoring revision-3 managed compiler compiled evidence is unsafe',
        );
      }
    } else if (failure == null) {
      throw const FormatException(
        'authoring revision-3 managed compiler failure has no reason',
      );
    }
    if (diagnostics?.capture ==
            ScriptCompileCaptureDisposition.processExitUnconfirmed &&
        !recoveryRequired) {
      throw const FormatException(
        'authoring revision-3 managed compiler lost recovery dominance',
      );
    }
    return AuthoringRevision3ManagedCompilerEvidence._(
      outcome: outcome,
      failure: failure,
      diagnostics: diagnostics,
      installRestore: installRestore,
      recoveryRequired: recoveryRequired,
      outputDiscarded: outputDiscarded,
      backend: backend,
    );
  }
}

/// Evidence-only compiler check for one native-derived module at one exact
/// revision-3 Store head. This result cannot be used as a build/deploy input.
final class AuthoringRevision3ManagedCompilerCheckResult {
  const AuthoringRevision3ManagedCompilerCheckResult._({
    required this.exactCurrent,
    required this.head,
    required this.project,
    required this.entity,
    required this.module,
    required this.compiler,
    required this.scope,
    required this.buildStatus,
    required this.deployStatus,
    required this.runtimeQualification,
    required this.publicationStatus,
    required this.backendMode,
  });

  final bool exactCurrent;
  final AuthoringWorkingHead head;
  final AuthoringRevision3ManagedCompilerProjectEvidence project;
  final AuthoringRevision3ManagedCompilerEntityEvidence entity;
  final AuthoringRevision3ManagedCompilerModuleEvidence module;
  final AuthoringRevision3ManagedCompilerEvidence compiler;
  final AuthoringRevision3ManagedCompilerScope scope;
  final AuthoringRevision3ManagedCompilerBuildStatus buildStatus;
  final AuthoringRevision3ManagedCompilerDeployStatus deployStatus;
  final AuthoringRevision3ManagedCompilerRuntimeQualification
  runtimeQualification;
  final AuthoringRevision3ManagedCompilerPublicationStatus publicationStatus;
  final ScriptCompilerBackendMode? backendMode;

  /// True only when compiler acceptance is still bound to the requested head.
  bool get acceptedAtExactCurrent =>
      exactCurrent && compiler.compiledEvidenceOnly;

  bool get recoveryRequired => compiler.recoveryRequired;

  bool get gameInstallRecoveryRequired => compiler.gameInstallRecoveryRequired;

  String get projectId => project.id;
  int get projectRevision => project.revision;
  AuthoringDraftContentSeal get projectSeal => project.seal;
  String get entityId => entity.id;
  int get entityRevision => entity.revision;
  String get moduleId => module.id;
  int get moduleRevision => module.revision;

  factory AuthoringRevision3ManagedCompilerCheckResult.fromJson(
    Map<String, Object?> json, {
    required AuthoringWorkingHead expectedHead,
    required String requestedEntityId,
    required AuthoringRevision3ManagedCompilerEntityKind expectedKind,
    ScriptCompilerBackendMode? expectedBackend,
  }) {
    _authoringExactFields(json, const <String>{
      'ok',
      'outcome',
      'exact_current',
      'head_json',
      'project',
      'entity',
      'module',
      'compiler',
      'scope',
      'build_status',
      'deploy_status',
      'runtime_qualification',
      'publication_status',
    }, 'revision-3 managed compiler response');
    _authoringRequireSignedSafeUnsignedJsonNumbers(
      json,
      'revision-3 managed compiler response',
    );
    if (json['ok'] != true ||
        json['outcome'] != 'compiler_check_only' ||
        json['scope'] != 'compiler_check_only' ||
        json['build_status'] != 'blocked' ||
        json['deploy_status'] != 'not_supported' ||
        json['runtime_qualification'] != 'runtime_unqualified' ||
        json['publication_status'] != 'not_supported') {
      throw const FormatException(
        'authoring revision-3 managed compiler response widens authority',
      );
    }
    final exactCurrent = json['exact_current'];
    if (exactCurrent is! bool) {
      throw const FormatException(
        'authoring revision-3 managed compiler exact-current flag is invalid',
      );
    }
    final head = AuthoringWorkingHead.fromCanonicalJson(
      _authoringRevision3ManagedCompilerString(
        json['head_json'],
        'head_json',
        maxBytes: _maxAuthoringHeadJsonBytes,
      ),
    );
    if (head.canonicalJson != expectedHead.canonicalJson) {
      throw const FormatException(
        'authoring revision-3 managed compiler returned another head',
      );
    }
    final project = AuthoringRevision3ManagedCompilerProjectEvidence._fromJson(
      json['project'],
    );
    if (project.seal.byteLength != head.snapshotByteLength ||
        project.seal.sha256 != head.snapshotSha256) {
      throw const FormatException(
        'authoring revision-3 managed compiler project is not bound to its head',
      );
    }
    final entity = AuthoringRevision3ManagedCompilerEntityEvidence._fromJson(
      json['entity'],
      expectedKind: expectedKind,
      requestedEntityId: requestedEntityId,
    );
    final module = AuthoringRevision3ManagedCompilerModuleEvidence._fromJson(
      json['module'],
      selectedEntityId: entity.id,
    );
    final compiler = AuthoringRevision3ManagedCompilerEvidence._fromJson(
      json['compiler'],
      expectedBackend: expectedBackend,
    );
    if (compiler.recoveryRequired && exactCurrent) {
      throw const FormatException(
        'authoring revision-3 managed compiler recovery overrides exact-current authority',
      );
    }
    return AuthoringRevision3ManagedCompilerCheckResult._(
      exactCurrent: exactCurrent,
      head: head,
      project: project,
      entity: entity,
      module: module,
      compiler: compiler,
      scope: AuthoringRevision3ManagedCompilerScope.compilerCheckOnly,
      buildStatus: AuthoringRevision3ManagedCompilerBuildStatus.blocked,
      deployStatus: AuthoringRevision3ManagedCompilerDeployStatus.notSupported,
      runtimeQualification:
          AuthoringRevision3ManagedCompilerRuntimeQualification
              .runtimeUnqualified,
      publicationStatus:
          AuthoringRevision3ManagedCompilerPublicationStatus.notSupported,
      backendMode: expectedBackend,
    );
  }
}

String _authoringRevision3ManagedCompilerEntityId(Object? value, String field) {
  final id = _authoringRevision3ManagedCompilerString(
    value,
    field,
    maxBytes: 32,
  );
  if (!_authoringEntityIdPattern.hasMatch(id) ||
      id == _revision3ManagedCompilerZeroId) {
    throw FormatException(
      'authoring revision-3 managed compiler $field is not a nonzero canonical entity ID',
    );
  }
  return id;
}

int _authoringRevision3ManagedCompilerRevision(Object? value, String field) {
  if (value is! int || value < 0 || value > _maxAuthoringSignedJsonInteger) {
    throw FormatException(
      'authoring revision-3 managed compiler $field is outside the signed wire domain',
    );
  }
  return value;
}

String _authoringRevision3ManagedCompilerString(
  Object? value,
  String field, {
  required int maxBytes,
  bool allowEmpty = false,
}) {
  if (value is! String || (!allowEmpty && value.isEmpty)) {
    throw FormatException(
      'authoring revision-3 managed compiler $field is not a string',
    );
  }
  if (value.isNotEmpty) {
    try {
      _authoringDraftRequestString(value, field, maxBytes);
    } on ArgumentError {
      throw FormatException(
        'authoring revision-3 managed compiler $field is not bounded UTF-8',
      );
    }
  }
  if (value.contains('\u0000')) {
    throw FormatException(
      'authoring revision-3 managed compiler $field contains NUL',
    );
  }
  return value;
}

ScriptCompileFailure? _authoringRevision3ManagedCompilerFailure(Object? value) {
  if (value == null) return null;
  final json = _authoringRequiredObject(
    value,
    'revision-3 managed compiler failure',
  );
  _authoringExactFields(json, const <String>{
    'code',
    'message',
  }, 'revision-3 managed compiler failure');
  final code = _authoringRevision3ManagedCompilerString(
    json['code'],
    'compile_error.code',
    maxBytes: _maxNativeErrorCodeLength,
  );
  final message = _authoringRevision3ManagedCompilerString(
    json['message'],
    'compile_error.message',
    maxBytes: _maxRevision3ManagedCompilerFailureMessageBytes,
  );
  if (!_nativeErrorCodePattern.hasMatch(code) || message.trim().isEmpty) {
    throw const FormatException(
      'authoring revision-3 managed compiler failure is invalid',
    );
  }
  return ScriptCompileFailure(code: code, message: message);
}

ScriptCompilerDiagnostics? _authoringRevision3ManagedCompilerDiagnostics(
  Object? value,
) {
  if (value == null) return null;
  final json = _authoringRequiredObject(
    value,
    'revision-3 managed compiler diagnostics',
  );
  _authoringExactFields(json, const <String>{
    'capture',
    'messages',
    'omitted',
  }, 'revision-3 managed compiler diagnostics');
  final capture = switch (json['capture']) {
    'captured' => ScriptCompileCaptureDisposition.captured,
    'capture_invalid' => ScriptCompileCaptureDisposition.captureInvalid,
    'unavailable_fallback' =>
      ScriptCompileCaptureDisposition.unavailableFallback,
    'unavailable_without_fallback' =>
      ScriptCompileCaptureDisposition.unavailableWithoutFallback,
    'process_exit_unconfirmed' =>
      ScriptCompileCaptureDisposition.processExitUnconfirmed,
    'disabled' => ScriptCompileCaptureDisposition.disabled,
    _ => throw const FormatException(
      'authoring revision-3 managed compiler diagnostics capture is unsupported',
    ),
  };
  final rawMessages = json['messages'];
  final omitted = json['omitted'];
  if (rawMessages is! List ||
      rawMessages.length > _maxRevision3ManagedCompilerDiagnostics ||
      omitted is! int ||
      omitted < 0 ||
      omitted > _maxRevision3ManagedCompilerOmitted) {
    throw const FormatException(
      'authoring revision-3 managed compiler diagnostics exceed their bounds',
    );
  }
  var textBytes = 0;
  final messages = <ScriptCompilerDiagnostic>[];
  for (var index = 0; index < rawMessages.length; index++) {
    final raw = _authoringRequiredObject(
      rawMessages[index],
      'revision-3 managed compiler diagnostic at index $index',
    );
    _authoringExactFields(raw, const <String>{
      'file',
      'line',
      'column',
      'severity',
      'message',
    }, 'revision-3 managed compiler diagnostic at index $index');
    final file = _authoringRevision3ManagedCompilerString(
      raw['file'],
      'compiler_diagnostics.messages[$index].file',
      maxBytes: _maxRevision3ManagedCompilerDiagnosticFileBytes,
      allowEmpty: true,
    );
    final message = _authoringRevision3ManagedCompilerString(
      raw['message'],
      'compiler_diagnostics.messages[$index].message',
      maxBytes: _maxRevision3ManagedCompilerDiagnosticMessageBytes,
      allowEmpty: true,
    );
    final line = raw['line'];
    final column = raw['column'];
    if (line is! int ||
        line < 0 ||
        line > _maxRevision3ManagedCompilerCoordinate ||
        column is! int ||
        column < 0 ||
        column > _maxRevision3ManagedCompilerCoordinate) {
      throw const FormatException(
        'authoring revision-3 managed compiler diagnostic coordinate is invalid',
      );
    }
    final severity = switch (raw['severity']) {
      'error' => ScriptCompilerDiagnosticSeverity.error,
      'warning' => ScriptCompilerDiagnosticSeverity.warning,
      'note' => ScriptCompilerDiagnosticSeverity.note,
      _ => throw const FormatException(
        'authoring revision-3 managed compiler diagnostic severity is unsupported',
      ),
    };
    textBytes += utf8.encode(file).length + utf8.encode(message).length;
    if (textBytes > _maxRevision3ManagedCompilerDiagnosticTextBytes) {
      throw const FormatException(
        'authoring revision-3 managed compiler diagnostic text exceeds its budget',
      );
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
