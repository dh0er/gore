part of '../core/mod_ffi.dart';

const _maxRevision3ProjectCompilerModules = 1024;

enum AuthoringRevision3ProjectCompilerOutcome {
  compiledEvidenceOnly,
  failed,
  notNeededEmpty,
}

enum AuthoringRevision3ProjectCompilerOutputDisposition {
  discarded,
  notCreated,
  recoveryRetained,
}

enum AuthoringRevision3ProjectCompilerClosingAuditStatus {
  exact,
  drift,
  inspectionFailed,
  notRun,
}

enum AuthoringRevision3ProjectCompilerScope { projectCompilerCheckOnly }

enum AuthoringRevision3ProjectCompilerBuildStatus { blocked }

enum AuthoringRevision3ProjectCompilerDeployStatus { notSupported }

enum AuthoringRevision3ProjectCompilerRuntimeQualification {
  runtimeUnqualified,
}

enum AuthoringRevision3ProjectCompilerPublicationStatus { notSupported }

final class AuthoringRevision3ProjectCompilerProjectEvidence {
  const AuthoringRevision3ProjectCompilerProjectEvidence._({
    required this.id,
    required this.revision,
    required this.seal,
  });

  final String id;
  final int revision;
  final AuthoringDraftContentSeal seal;

  factory AuthoringRevision3ProjectCompilerProjectEvidence._fromJson(
    Object? value,
  ) {
    final json = _authoringRequiredObject(
      value,
      'revision-3 project compiler project evidence',
    );
    _authoringExactFields(json, const <String>{
      'id',
      'revision',
      'seal',
    }, 'revision-3 project compiler project evidence');
    final seal = _revision3ProjectCompilerSeal(
      json['seal'],
      'project.seal',
      maxBytes: _maxAuthoringProjectJsonBytes,
    );
    return AuthoringRevision3ProjectCompilerProjectEvidence._(
      id: _authoringRevision3ManagedCompilerEntityId(json['id'], 'project.id'),
      revision: _authoringRevision3ManagedCompilerRevision(
        json['revision'],
        'project.revision',
      ),
      seal: seal,
    );
  }
}

final class AuthoringRevision3ProjectCompilerGameInputs {
  const AuthoringRevision3ProjectCompilerGameInputs._({
    required this.executable,
    required this.shippingCache,
    required this.bindsCache,
    required this.storyCatalog,
  });

  final AuthoringDraftContentSeal executable;
  final AuthoringDraftContentSeal shippingCache;
  final AuthoringDraftContentSeal bindsCache;
  final AuthoringDraftContentSeal storyCatalog;

  factory AuthoringRevision3ProjectCompilerGameInputs._fromJson(Object? value) {
    final json = _authoringRequiredObject(
      value,
      'revision-3 project compiler game inputs',
    );
    _authoringExactFields(json, const <String>{
      'executable',
      'shipping_cache',
      'binds_cache',
      'story_catalog',
    }, 'revision-3 project compiler game inputs');
    return AuthoringRevision3ProjectCompilerGameInputs._(
      executable: _revision3ProjectCompilerSeal(
        json['executable'],
        'game_inputs.executable',
      ),
      shippingCache: _revision3ProjectCompilerSeal(
        json['shipping_cache'],
        'game_inputs.shipping_cache',
      ),
      bindsCache: _revision3ProjectCompilerSeal(
        json['binds_cache'],
        'game_inputs.binds_cache',
      ),
      storyCatalog: _revision3ProjectCompilerSeal(
        json['story_catalog'],
        'game_inputs.story_catalog',
        maxBytes: _maxAuthoringStoryCatalogJsonBytes,
      ),
    );
  }
}

final class AuthoringRevision3ProjectCompilerCoverage {
  const AuthoringRevision3ProjectCompilerCoverage._({
    required this.scriptModuleCount,
    required this.questModuleCount,
    required this.npcModuleCount,
    required this.moduleManifest,
  });

  final int scriptModuleCount;
  final int questModuleCount;
  final int npcModuleCount;
  final AuthoringDraftContentSeal moduleManifest;

  bool get isEmpty => scriptModuleCount == 0;

  factory AuthoringRevision3ProjectCompilerCoverage._fromJson(Object? value) {
    final json = _authoringRequiredObject(
      value,
      'revision-3 project compiler coverage',
    );
    _authoringExactFields(json, const <String>{
      'script_module_count',
      'quest_module_count',
      'npc_module_count',
      'module_manifest',
    }, 'revision-3 project compiler coverage');
    final scriptModuleCount = _revision3ProjectCompilerCount(
      json['script_module_count'],
      'coverage.script_module_count',
    );
    final questModuleCount = _revision3ProjectCompilerCount(
      json['quest_module_count'],
      'coverage.quest_module_count',
    );
    final npcModuleCount = _revision3ProjectCompilerCount(
      json['npc_module_count'],
      'coverage.npc_module_count',
    );
    if (questModuleCount + npcModuleCount != scriptModuleCount) {
      throw const FormatException(
        'authoring revision-3 project compiler coverage counts disagree',
      );
    }
    return AuthoringRevision3ProjectCompilerCoverage._(
      scriptModuleCount: scriptModuleCount,
      questModuleCount: questModuleCount,
      npcModuleCount: npcModuleCount,
      moduleManifest: _revision3ProjectCompilerSeal(
        json['module_manifest'],
        'coverage.module_manifest',
        maxBytes: _maxAuthoringProjectJsonBytes,
      ),
    );
  }
}

final class AuthoringRevision3ProjectCompilerEvidence {
  const AuthoringRevision3ProjectCompilerEvidence._({
    required this.outcome,
    required this.runCount,
    required this.failure,
    required this.diagnostics,
    required this.installRestore,
    required this.recoveryRequired,
    required this.outputDisposition,
    required this.backend,
  });

  final AuthoringRevision3ProjectCompilerOutcome outcome;
  final int runCount;
  final ScriptCompileFailure? failure;
  final ScriptCompilerDiagnostics? diagnostics;
  final ScriptCompileInstallRestore installRestore;
  final bool recoveryRequired;
  final AuthoringRevision3ProjectCompilerOutputDisposition outputDisposition;
  final ScriptCompilerBackendEvidence? backend;

  bool get compiledEvidenceOnly =>
      outcome == AuthoringRevision3ProjectCompilerOutcome.compiledEvidenceOnly;

  bool get notNeededEmpty =>
      outcome == AuthoringRevision3ProjectCompilerOutcome.notNeededEmpty;

  factory AuthoringRevision3ProjectCompilerEvidence._fromJson(
    Object? value, {
    ScriptCompilerBackendMode? expectedBackend,
  }) {
    final json = _authoringRequiredObject(
      value,
      'revision-3 project compiler evidence',
    );
    final fields = <String>{
      'outcome',
      'run_count',
      'compile_error',
      'compiler_diagnostics',
      'install_restore',
      'recovery_required',
      'output_disposition',
    };
    if (expectedBackend != null) fields.add('compiler_backend');
    _authoringExactFields(json, fields, 'revision-3 project compiler evidence');
    final outcome = switch (json['outcome']) {
      'compiled_evidence_only' =>
        AuthoringRevision3ProjectCompilerOutcome.compiledEvidenceOnly,
      'failed' => AuthoringRevision3ProjectCompilerOutcome.failed,
      'not_needed_empty' =>
        AuthoringRevision3ProjectCompilerOutcome.notNeededEmpty,
      _ => throw const FormatException(
        'authoring revision-3 project compiler outcome is unsupported',
      ),
    };
    final runCount = json['run_count'];
    if (runCount is! int || runCount < 0 || runCount > 1) {
      throw const FormatException(
        'authoring revision-3 project compiler run count is invalid',
      );
    }
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
        'authoring revision-3 project compiler restore disposition is unsupported',
      ),
    };
    final recoveryRequired = json['recovery_required'];
    if (recoveryRequired is! bool) {
      throw const FormatException(
        'authoring revision-3 project compiler recovery state is invalid',
      );
    }
    final outputDisposition = switch (json['output_disposition']) {
      'discarded' =>
        AuthoringRevision3ProjectCompilerOutputDisposition.discarded,
      'not_created' =>
        AuthoringRevision3ProjectCompilerOutputDisposition.notCreated,
      'recovery_retained' =>
        AuthoringRevision3ProjectCompilerOutputDisposition.recoveryRetained,
      _ => throw const FormatException(
        'authoring revision-3 project compiler output disposition is unsupported',
      ),
    };
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
        outcome == AuthoringRevision3ProjectCompilerOutcome.failed &&
        failure != null &&
        _revision3ProjectCompilerPreexistingRecoveryCodes.contains(
          failure.code,
        );
    final outputRequiresRecovery =
        outputDisposition ==
        AuthoringRevision3ProjectCompilerOutputDisposition.recoveryRetained;
    if (recoveryRequired !=
        (restoreRequiresRecovery ||
            preexistingRecovery ||
            outputRequiresRecovery)) {
      throw const FormatException(
        'authoring revision-3 project compiler recovery evidence disagrees',
      );
    }
    if (recoveryRequired != outputRequiresRecovery) {
      throw const FormatException(
        'authoring revision-3 project compiler output lost recovery dominance',
      );
    }
    if (diagnostics?.capture ==
            ScriptCompileCaptureDisposition.processExitUnconfirmed &&
        !recoveryRequired) {
      throw const FormatException(
        'authoring revision-3 project compiler lost process-exit recovery dominance',
      );
    }
    if (runCount == 0 && diagnostics != null) {
      throw const FormatException(
        'authoring revision-3 project compiler reported diagnostics without a compiler run',
      );
    }

    switch (outcome) {
      case AuthoringRevision3ProjectCompilerOutcome.compiledEvidenceOnly:
        if (runCount != 1 ||
            failure != null ||
            !_revision3ProjectCompilerDiagnosticsAreAccepted(diagnostics) ||
            installRestore != ScriptCompileInstallRestore.restoredExact ||
            recoveryRequired ||
            outputDisposition !=
                AuthoringRevision3ProjectCompilerOutputDisposition.discarded) {
          throw const FormatException(
            'authoring revision-3 project compiler compiled evidence is unsafe',
          );
        }
      case AuthoringRevision3ProjectCompilerOutcome.notNeededEmpty:
        if (runCount != 0 ||
            failure != null ||
            diagnostics != null ||
            installRestore != ScriptCompileInstallRestore.notStarted ||
            recoveryRequired ||
            outputDisposition !=
                AuthoringRevision3ProjectCompilerOutputDisposition.notCreated) {
          throw const FormatException(
            'authoring revision-3 empty project compiler evidence is invalid',
          );
        }
      case AuthoringRevision3ProjectCompilerOutcome.failed:
        final cleanAttemptedFailure =
            installRestore == ScriptCompileInstallRestore.restoredExact &&
            (outputDisposition ==
                    AuthoringRevision3ProjectCompilerOutputDisposition
                        .discarded ||
                outputDisposition ==
                    AuthoringRevision3ProjectCompilerOutputDisposition
                        .notCreated);
        final cleanNotStartedFailure =
            installRestore == ScriptCompileInstallRestore.notStarted &&
            diagnostics == null &&
            outputDisposition ==
                AuthoringRevision3ProjectCompilerOutputDisposition.notCreated;
        if (failure == null ||
            (runCount == 0 &&
                !recoveryRequired &&
                (installRestore != ScriptCompileInstallRestore.notStarted ||
                    outputDisposition !=
                        AuthoringRevision3ProjectCompilerOutputDisposition
                            .notCreated)) ||
            (runCount == 1 &&
                !recoveryRequired &&
                !cleanAttemptedFailure &&
                !cleanNotStartedFailure)) {
          throw const FormatException(
            'authoring revision-3 project compiler failure evidence is invalid',
          );
        }
    }

    return AuthoringRevision3ProjectCompilerEvidence._(
      outcome: outcome,
      runCount: runCount,
      failure: failure,
      diagnostics: diagnostics,
      installRestore: installRestore,
      recoveryRequired: recoveryRequired,
      outputDisposition: outputDisposition,
      backend: backend,
    );
  }
}

final class AuthoringRevision3ProjectCompilerClosingAudit {
  const AuthoringRevision3ProjectCompilerClosingAudit._({
    required this.store,
    required this.game,
  });

  final AuthoringRevision3ProjectCompilerClosingAuditStatus store;
  final AuthoringRevision3ProjectCompilerClosingAuditStatus game;

  bool get bothExact =>
      store == AuthoringRevision3ProjectCompilerClosingAuditStatus.exact &&
      game == AuthoringRevision3ProjectCompilerClosingAuditStatus.exact;

  bool get storeRequiresReopen =>
      store == AuthoringRevision3ProjectCompilerClosingAuditStatus.drift ||
      store ==
          AuthoringRevision3ProjectCompilerClosingAuditStatus.inspectionFailed;

  factory AuthoringRevision3ProjectCompilerClosingAudit._fromJson(
    Object? value,
  ) {
    final json = _authoringRequiredObject(
      value,
      'revision-3 project compiler closing audit',
    );
    _authoringExactFields(json, const <String>{
      'store',
      'game',
    }, 'revision-3 project compiler closing audit');
    return AuthoringRevision3ProjectCompilerClosingAudit._(
      store: _revision3ProjectCompilerClosingAuditStatus(
        json['store'],
        'store',
      ),
      game: _revision3ProjectCompilerClosingAuditStatus(json['game'], 'game'),
    );
  }
}

final class AuthoringRevision3ProjectCompilerCheckResult {
  const AuthoringRevision3ProjectCompilerCheckResult._({
    required this.exactCurrent,
    required this.head,
    required this.project,
    required this._gameInputs,
    required this.coverage,
    required this.compiler,
    required this.closingAudit,
    required this.scope,
    required this.buildStatus,
    required this.deployStatus,
    required this.runtimeQualification,
    required this.publicationStatus,
    required this.backendMode,
  });

  final bool exactCurrent;
  final AuthoringWorkingHead head;
  final AuthoringRevision3ProjectCompilerProjectEvidence project;
  final AuthoringRevision3ProjectCompilerGameInputs? _gameInputs;
  AuthoringRevision3ProjectCompilerGameInputs get gameInputs =>
      _gameInputs ??
      (throw StateError('standalone compiler did not inspect game inputs'));
  AuthoringRevision3ProjectCompilerGameInputs? get gameInputsOrNull =>
      _gameInputs;
  final AuthoringRevision3ProjectCompilerCoverage coverage;
  final AuthoringRevision3ProjectCompilerEvidence compiler;
  final AuthoringRevision3ProjectCompilerClosingAudit closingAudit;
  final AuthoringRevision3ProjectCompilerScope scope;
  final AuthoringRevision3ProjectCompilerBuildStatus buildStatus;
  final AuthoringRevision3ProjectCompilerDeployStatus deployStatus;
  final AuthoringRevision3ProjectCompilerRuntimeQualification
  runtimeQualification;
  final AuthoringRevision3ProjectCompilerPublicationStatus publicationStatus;
  final ScriptCompilerBackendMode? backendMode;

  bool get recoveryRequired => compiler.recoveryRequired;

  bool get acceptedAtExactCurrent =>
      exactCurrent &&
      !compiler.recoveryRequired &&
      (compiler.compiledEvidenceOnly || compiler.notNeededEmpty);

  factory AuthoringRevision3ProjectCompilerCheckResult.fromJson(
    Map<String, Object?> json, {
    required AuthoringWorkingHead expectedHead,
    ScriptCompilerBackendMode? expectedBackend,
  }) {
    _authoringExactFields(json, const <String>{
      'ok',
      'outcome',
      'exact_current',
      'head_json',
      'project',
      'game_inputs',
      'coverage',
      'compiler',
      'closing_audit',
      'scope',
      'build_status',
      'deploy_status',
      'runtime_qualification',
      'publication_status',
    }, 'revision-3 project compiler response');
    _authoringRequireSignedSafeUnsignedJsonNumbers(
      json,
      'revision-3 project compiler response',
    );
    if (json['ok'] != true ||
        json['outcome'] != 'project_compiler_check_only' ||
        json['scope'] != 'project_compiler_check_only' ||
        json['build_status'] != 'blocked' ||
        json['deploy_status'] != 'not_supported' ||
        json['runtime_qualification'] != 'runtime_unqualified' ||
        json['publication_status'] != 'not_supported') {
      throw const FormatException(
        'authoring revision-3 project compiler response widens authority',
      );
    }
    final exactCurrent = json['exact_current'];
    if (exactCurrent is! bool) {
      throw const FormatException(
        'authoring revision-3 project compiler exact-current state is invalid',
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
        'authoring revision-3 project compiler returned another head',
      );
    }
    final project = AuthoringRevision3ProjectCompilerProjectEvidence._fromJson(
      json['project'],
    );
    if (project.seal.byteLength != head.snapshotByteLength ||
        project.seal.sha256 != head.snapshotSha256) {
      throw const FormatException(
        'authoring revision-3 project compiler project is not bound to its head',
      );
    }
    final gameInputs = json['game_inputs'] == null
        ? null
        : AuthoringRevision3ProjectCompilerGameInputs._fromJson(
            json['game_inputs'],
          );
    if (expectedBackend == null && gameInputs == null) {
      throw const FormatException(
        'authoring revision-3 project compiler V1 omitted game inputs',
      );
    }
    final coverage = AuthoringRevision3ProjectCompilerCoverage._fromJson(
      json['coverage'],
    );
    final compiler = AuthoringRevision3ProjectCompilerEvidence._fromJson(
      json['compiler'],
      expectedBackend: expectedBackend,
    );
    final backend = compiler.backend;
    if (gameInputs == null &&
        (expectedBackend != ScriptCompilerBackendMode.standalone ||
            backend == null ||
            backend.resultBackend != null ||
            backend.standaloneAttempted ||
            backend.gameAttempted)) {
      throw const FormatException(
        'authoring revision-3 project compiler omitted game inputs outside strict standalone preflight',
      );
    }
    final closingAudit =
        AuthoringRevision3ProjectCompilerClosingAudit._fromJson(
          json['closing_audit'],
        );
    if ((compiler.notNeededEmpty && !coverage.isEmpty) ||
        (coverage.isEmpty &&
            !compiler.notNeededEmpty &&
            !compiler.recoveryRequired) ||
        exactCurrent != closingAudit.bothExact ||
        ((compiler.compiledEvidenceOnly || compiler.notNeededEmpty) &&
            (!closingAudit.bothExact || gameInputs == null))) {
      throw const FormatException(
        'authoring revision-3 project compiler result has inconsistent authority',
      );
    }
    return AuthoringRevision3ProjectCompilerCheckResult._(
      exactCurrent: exactCurrent,
      head: head,
      project: project,
      gameInputs: gameInputs,
      coverage: coverage,
      compiler: compiler,
      closingAudit: closingAudit,
      scope: AuthoringRevision3ProjectCompilerScope.projectCompilerCheckOnly,
      buildStatus: AuthoringRevision3ProjectCompilerBuildStatus.blocked,
      deployStatus: AuthoringRevision3ProjectCompilerDeployStatus.notSupported,
      runtimeQualification:
          AuthoringRevision3ProjectCompilerRuntimeQualification
              .runtimeUnqualified,
      publicationStatus:
          AuthoringRevision3ProjectCompilerPublicationStatus.notSupported,
      backendMode: expectedBackend,
    );
  }
}

AuthoringRevision3ProjectCompilerClosingAuditStatus
_revision3ProjectCompilerClosingAuditStatus(
  Object? value,
  String field,
) => switch (value) {
  'exact' => AuthoringRevision3ProjectCompilerClosingAuditStatus.exact,
  'drift' => AuthoringRevision3ProjectCompilerClosingAuditStatus.drift,
  'inspection_failed' =>
    AuthoringRevision3ProjectCompilerClosingAuditStatus.inspectionFailed,
  'not_run' => AuthoringRevision3ProjectCompilerClosingAuditStatus.notRun,
  _ => throw FormatException(
    'authoring revision-3 project compiler closing audit $field is unsupported',
  ),
};

const _revision3ProjectCompilerPreexistingRecoveryCodes = <String>{
  'COMPILE_BASE_RECOVERY_REQUIRED',
  'COMPILE_INSTALL_RECOVERY_REQUIRED',
  'COMPILE_INSTALL_GUARD_RELEASE_FAILED',
};

AuthoringDraftContentSeal _revision3ProjectCompilerSeal(
  Object? value,
  String field, {
  int maxBytes = _maxAuthoringSignedJsonInteger,
}) {
  final seal = AuthoringDraftContentSeal.fromJson(
    _authoringRequiredObject(value, 'revision-3 project compiler $field seal'),
  );
  if (seal.byteLength <= 0 || seal.byteLength > maxBytes) {
    throw FormatException(
      'authoring revision-3 project compiler $field seal is outside its bounded range',
    );
  }
  return seal;
}

int _revision3ProjectCompilerCount(Object? value, String field) {
  if (value is! int ||
      value < 0 ||
      value > _maxRevision3ProjectCompilerModules) {
    throw FormatException(
      'authoring revision-3 project compiler $field is outside its bounded range',
    );
  }
  return value;
}

bool _revision3ProjectCompilerDiagnosticsAreAccepted(
  ScriptCompilerDiagnostics? diagnostics,
) =>
    diagnostics != null &&
    (diagnostics.capture == ScriptCompileCaptureDisposition.captured ||
        diagnostics.capture ==
            ScriptCompileCaptureDisposition.unavailableFallback) &&
    !diagnostics.messages.any(
      (message) => message.severity == ScriptCompilerDiagnosticSeverity.error,
    );
