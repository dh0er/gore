import 'dart:convert';

import 'package:crypto/crypto.dart' as crypto;

import '../../core/core_service.dart';
import '../../core/mod_ffi.dart';
import '../../project/managed_project_session.dart';
import 'story_draft_requests.dart';

const int _maxStoryProjectBytes = 16 * 1024 * 1024;
const int _maxStoryEntities = 100000;
const int _maxStorySourceBytes = 1024 * 1024;
const int _maxStoryDisplayNameBytes = 256;
const int _maxStoryIdentifierBytes = 1024;
const int _maxSignedRevision = 0x7fffffffffffffff;

final RegExp _entityIdPattern = RegExp(r'^[0-9a-f]{32}$');
final RegExp _sha256Pattern = RegExp(r'^[0-9a-f]{64}$');

/// One source-bearing story Draft decoded from a canonical revision-2 project.
final class StoryDraftState {
  const StoryDraftState({
    required this.draftId,
    required this.kind,
    required this.displayName,
    required this.runtimeId,
    required this.scriptModuleId,
    required this.moduleNamespace,
    required this.source,
  });

  final String draftId;
  final AuthoringStoryDraftKind kind;
  final String displayName;
  final String runtimeId;
  final String scriptModuleId;
  final String moduleNamespace;
  final String source;
}

/// Strict, bounded read model for the Story workspace.
///
/// The native document store remains the schema validator. This decoder only
/// extracts the small Story projection needed by the Studio and verifies every
/// relationship used to expose generated source.
final class StoryWorkspaceState {
  const StoryWorkspaceState._({
    required this.projectId,
    required this.revision,
    required this.drafts,
    required this.diagnostics,
    required this.blocksBuild,
  });

  final String projectId;
  final int revision;
  final List<StoryDraftState> drafts;
  final List<AuthoringDiagnostic> diagnostics;
  final bool blocksBuild;

  StoryDraftState? draftById(String id) {
    for (final draft in drafts) {
      if (draft.draftId == id) return draft;
    }
    return null;
  }

  factory StoryWorkspaceState.fromCanonicalProjectJson(
    String projectJson, {
    required bool blocksBuild,
    required List<AuthoringDiagnostic> diagnostics,
  }) {
    _requireExactRevision2BuildGate(blocksBuild, diagnostics);
    _boundedUtf8Length(
      projectJson,
      _maxStoryProjectBytes,
      'Story project JSON',
      requireNonEmpty: true,
    );
    final project = decodeCanonicalGoreCoreResponse(projectJson);
    _requireExactFields(project, const <String>{
      'format',
      'schema_revision',
      'project_id',
      'revision',
      'meta',
      'target',
      'authoring_locales',
      'entities',
      'asset_store',
    }, 'Story project');
    if (project['format'] != 2 || project['schema_revision'] != 2) {
      throw const FormatException(
        'Story workspace requires a format-2 schema-revision-2 project',
      );
    }
    final projectId = _requireEntityId(project['project_id'], 'project_id');
    final revision = _requireInt(
      project['revision'],
      'revision',
      min: 0,
      max: _maxSignedRevision,
    );
    _requireObject(project['meta'], 'meta');
    _requireObject(project['target'], 'target');
    final locales = project['authoring_locales'];
    if (locales is! List || locales.length > _maxStoryEntities) {
      throw const FormatException('authoring_locales is not bounded');
    }
    _requireObject(project['asset_store'], 'asset_store');

    final rawEntities = _requireObject(project['entities'], 'entities');
    if (rawEntities.length > _maxStoryEntities) {
      throw const FormatException('Story entity count exceeds its limit');
    }
    final entities = <String, _StoryEntity>{};
    String? previousId;
    for (final entry in rawEntities.entries) {
      final id = _requireEntityId(entry.key, 'entity key');
      if (previousId != null && previousId.compareTo(id) >= 0) {
        throw const FormatException(
          'Story project entity keys are not canonical and unique',
        );
      }
      previousId = id;
      final entity = _requireObject(entry.value, 'entity $id');
      _requireExactFields(entity, const <String>{
        'id',
        'display_name',
        'origin',
        'revision',
        'payload',
      }, 'entity $id');
      if (_requireEntityId(entity['id'], 'entity $id id') != id) {
        throw FormatException('entity $id disagrees with its map key');
      }
      final payload = _requireObject(entity['payload'], 'entity $id payload');
      _requireExactFields(payload, const <String>{
        'kind',
        'data',
      }, 'entity $id payload');
      final kind = _requireBoundedString(
        payload['kind'],
        'entity $id kind',
        64,
      );
      final data = _requireObject(payload['data'], 'entity $id payload data');
      entities[id] = _StoryEntity(
        id: id,
        displayName: _requireBoundedString(
          entity['display_name'],
          'entity $id display_name',
          _maxStoryDisplayNameBytes,
        ),
        origin: _requireObject(entity['origin'], 'entity $id origin'),
        revision: _requireInt(
          entity['revision'],
          'entity $id revision',
          min: 0,
          max: _maxSignedRevision,
        ),
        kind: kind,
        data: data,
      );
    }

    final drafts = <StoryDraftState>[];
    final claimedModules = <String>{};
    for (final entity in entities.values) {
      final AuthoringStoryDraftKind kind;
      switch (entity.kind) {
        case 'npc_draft':
          kind = AuthoringStoryDraftKind.npcDraft;
        case 'quest_draft':
          kind = AuthoringStoryDraftKind.questDraft;
        default:
          continue;
      }
      if (entity.revision != 0) {
        throw FormatException(
          'Story Draft ${entity.id} must start at entity revision zero',
        );
      }
      _requireExactFields(entity.origin, const <String>{
        'type',
        'authored_runtime_id',
      }, 'Story Draft ${entity.id} origin');
      if (entity.origin['type'] != 'new') {
        throw FormatException('Story Draft ${entity.id} is not new');
      }
      final runtimeId = _requireBoundedString(
        entity.origin['authored_runtime_id'],
        'Story Draft ${entity.id} runtime ID',
        _maxStoryIdentifierBytes,
      );
      _requireExactFields(entity.data, const <String>{
        'generator_id',
        'generator_version',
        'input',
        'script_module',
      }, 'Story Draft ${entity.id} data');
      final generatorId = _requireBoundedString(
        entity.data['generator_id'],
        'Story Draft ${entity.id} generator_id',
        _maxStoryIdentifierBytes,
      );
      final generatorVersion = _requireInt(
        entity.data['generator_version'],
        'Story Draft ${entity.id} generator_version',
        min: 1,
        max: 0x7fffffff,
      );
      _requireObject(entity.data['input'], 'Story Draft ${entity.id} input');
      final moduleRef = _requireTypedRef(
        entity.data['script_module'],
        context: 'Story Draft ${entity.id} script_module',
      );
      if (moduleRef.projectId != projectId ||
          moduleRef.expectedKind != 'script_module') {
        throw FormatException(
          'Story Draft ${entity.id} has a foreign or mistyped module ref',
        );
      }
      if (!claimedModules.add(moduleRef.id)) {
        throw FormatException(
          'generated ScriptModule ${moduleRef.id} has multiple Draft owners',
        );
      }
      final module = entities[moduleRef.id];
      if (module == null || module.kind != 'script_module') {
        throw FormatException(
          'Story Draft ${entity.id} generated ScriptModule is missing',
        );
      }
      if (module.revision != 0) {
        throw FormatException(
          'generated ScriptModule ${module.id} must start at revision zero',
        );
      }
      _requireExactFields(module.origin, const <String>{
        'type',
        'generator_id',
        'generator_version',
        'owner',
      }, 'generated ScriptModule ${module.id} origin');
      if (module.origin['type'] != 'generated' ||
          module.origin['generator_id'] != generatorId ||
          module.origin['generator_version'] != generatorVersion) {
        throw FormatException(
          'generated ScriptModule ${module.id} origin disagrees with its Draft',
        );
      }
      final expectedOwnerKind = kind.wireName;
      final originOwner = _requireTypedRef(
        module.origin['owner'],
        context: 'generated ScriptModule ${module.id} origin owner',
      );
      _requireExactOwner(
        originOwner,
        projectId: projectId,
        draftId: entity.id,
        draftKind: expectedOwnerKind,
      );

      _requireExactFields(module.data, const <String>{
        'generator_id',
        'generator_version',
        'owner',
        'module_namespace',
        'module_relative_path',
        'source',
        'source_sha256',
        'input_fingerprint',
        'status',
      }, 'generated ScriptModule ${module.id} data');
      if (module.data['generator_id'] != generatorId ||
          module.data['generator_version'] != generatorVersion) {
        throw FormatException(
          'generated ScriptModule ${module.id} contract disagrees with its Draft',
        );
      }
      final payloadOwner = _requireTypedRef(
        module.data['owner'],
        context: 'generated ScriptModule ${module.id} payload owner',
      );
      _requireExactOwner(
        payloadOwner,
        projectId: projectId,
        draftId: entity.id,
        draftKind: expectedOwnerKind,
      );
      final moduleNamespace = _requireBoundedString(
        module.data['module_namespace'],
        'generated ScriptModule ${module.id} namespace',
        _maxStoryIdentifierBytes,
      );
      if (module.displayName != moduleNamespace) {
        throw FormatException(
          'generated ScriptModule ${module.id} display name is not its namespace',
        );
      }
      final relativePath = _requireBoundedString(
        module.data['module_relative_path'],
        'generated ScriptModule ${module.id} relative path',
        _maxStoryIdentifierBytes * 2,
      );
      if (relativePath != '${moduleNamespace.replaceAll('.', '/')}.as') {
        throw FormatException(
          'generated ScriptModule ${module.id} path is not derived from its namespace',
        );
      }
      final source = _requireBoundedString(
        module.data['source'],
        'generated ScriptModule ${module.id} source',
        _maxStorySourceBytes,
      );
      final sourceSha256 = _requireSha256(
        module.data['source_sha256'],
        'generated ScriptModule ${module.id} source_sha256',
      );
      if (crypto.sha256.convert(utf8.encode(source)).toString() !=
          sourceSha256) {
        throw FormatException(
          'generated ScriptModule ${module.id} source digest does not match',
        );
      }
      _requireSha256(
        module.data['input_fingerprint'],
        'generated ScriptModule ${module.id} input_fingerprint',
      );
      final status = _requireObject(
        module.data['status'],
        'generated ScriptModule ${module.id} status',
      );
      _requireExactFields(status, const <String>{
        'authoring',
        'runtime',
      }, 'generated ScriptModule ${module.id} status');
      if (status['authoring'] != 'offline_draft' ||
          status['runtime'] != 'runtime_unqualified') {
        throw FormatException(
          'generated ScriptModule ${module.id} has an unsupported status',
        );
      }
      drafts.add(
        StoryDraftState(
          draftId: entity.id,
          kind: kind,
          displayName: entity.displayName,
          runtimeId: runtimeId,
          scriptModuleId: module.id,
          moduleNamespace: moduleNamespace,
          source: source,
        ),
      );
    }
    drafts.sort((left, right) => left.draftId.compareTo(right.draftId));
    return StoryWorkspaceState._(
      projectId: projectId,
      revision: revision,
      drafts: List<StoryDraftState>.unmodifiable(drafts),
      diagnostics: List<AuthoringDiagnostic>.unmodifiable(diagnostics),
      blocksBuild: blocksBuild,
    );
  }
}

sealed class StoryDraftCreateResult {
  const StoryDraftCreateResult();

  StoryWorkspaceState get state;
  List<AuthoringDiagnostic> get diagnostics;
}

final class StoryDraftCreateApplied extends StoryDraftCreateResult {
  StoryDraftCreateApplied({
    required this.state,
    required this.draft,
    required List<AuthoringDiagnostic> diagnostics,
  }) : diagnostics = List<AuthoringDiagnostic>.unmodifiable(diagnostics);

  @override
  final StoryWorkspaceState state;
  final StoryDraftState draft;
  @override
  final List<AuthoringDiagnostic> diagnostics;
}

final class StoryDraftCreateRejected extends StoryDraftCreateResult {
  StoryDraftCreateRejected({
    required this.state,
    required List<AuthoringDiagnostic> diagnostics,
  }) : diagnostics = List<AuthoringDiagnostic>.unmodifiable(diagnostics);

  @override
  final StoryWorkspaceState state;
  @override
  final List<AuthoringDiagnostic> diagnostics;
}

sealed class StoryBuildReadinessCheckResult {
  const StoryBuildReadinessCheckResult();
}

/// Read-only inspection of one exact managed project revision.
final class StoryBuildReadinessChecked extends StoryBuildReadinessCheckResult {
  const StoryBuildReadinessChecked({
    required this.projectRevision,
    required this.moduleCount,
    required this.diagnosticCount,
    required this.blockingDiagnosticCount,
  });

  final int projectRevision;
  final int moduleCount;
  final int diagnosticCount;
  final int blockingDiagnosticCount;
}

/// The managed head changed while the read-only inspection was suspended.
final class StoryBuildReadinessStale extends StoryBuildReadinessCheckResult {
  const StoryBuildReadinessStale();
}

/// Production Story transaction coordinator.
///
/// Derivation, native evaluation, and publication all run inside the managed
/// session lane. A semantic rejection returns diagnostics without preparing a
/// checkpoint; an applied value is returned only after the candidate has been
/// fully verified and published by the session.
final class StoryWorkspaceController {
  factory StoryWorkspaceController({
    required ManagedAuthoringProjectSession session,
    required ModFfi ffi,
    StoryDraftIdSource? idSource,
    StoryDraftMutationJsonBuilder mutationBuilder =
        const ClosedStoryDraftMutationJsonBuilder(),
  }) => StoryWorkspaceController._(
    session: session,
    ffi: ffi,
    idSource: idSource ?? SecureStoryDraftIdSource(),
    mutationBuilder: mutationBuilder,
  );

  StoryWorkspaceController._({
    required this._session,
    required this._ffi,
    required this._idSource,
    required this._mutationBuilder,
  });

  final ManagedAuthoringProjectSession _session;
  final ModFfi _ffi;
  final StoryDraftIdSource _idSource;
  final StoryDraftMutationJsonBuilder _mutationBuilder;

  StoryWorkspaceState get current =>
      StoryWorkspaceState.fromCanonicalProjectJson(
        _session.projectJson,
        blocksBuild: _session.blocksBuild,
        diagnostics: _session.diagnostics,
      );

  Future<StoryDraftCreateResult> createNpc(StoryNpcDraftInput input) {
    return _create(
      expectedKind: AuthoringStoryDraftKind.npcDraft,
      buildMutation: (context) =>
          _mutationBuilder.buildNpc(context: context, input: input),
    );
  }

  Future<StoryBuildReadinessCheckResult> checkBuildPlan() async {
    try {
      return await _session.deriveAndSave<StoryBuildReadinessCheckResult>((
        latestProjectJson,
      ) async {
        final captured = StoryWorkspaceState.fromCanonicalProjectJson(
          latestProjectJson,
          blocksBuild: _session.blocksBuild,
          diagnostics: _session.diagnostics,
        );
        final plan = await _ffi.authoringStoryBuildPlanV1Generate(
          projectJson: latestProjectJson,
          profile: _session.profile,
        );
        if (plan.project.projectId != captured.projectId ||
            plan.project.projectRevision != captured.revision) {
          throw const FormatException(
            'Story build-plan result changed its captured project identity',
          );
        }
        return ManagedProjectDerivedRejection<StoryBuildReadinessCheckResult>(
          StoryBuildReadinessChecked(
            projectRevision: captured.revision,
            moduleCount: plan.moduleCount,
            diagnosticCount: plan.diagnosticCount,
            blockingDiagnosticCount: plan.blockingDiagnosticIndexes.length,
          ),
        );
      });
    } on ManagedProjectHeadConflictException {
      return const StoryBuildReadinessStale();
    }
  }

  Future<StoryDraftCreateResult> _create({
    required AuthoringStoryDraftKind expectedKind,
    required String Function(StoryDraftMutationContext context) buildMutation,
  }) =>
      _session.deriveAndSave<StoryDraftCreateResult>((latestProjectJson) async {
        final latest = StoryWorkspaceState.fromCanonicalProjectJson(
          latestProjectJson,
          blocksBuild: _session.blocksBuild,
          diagnostics: _session.diagnostics,
        );
        // Allocate only after the session has verified and decoded the exact
        // published head. Closed, reentrant, stale, or malformed calls consume
        // no IDs.
        final ids = _idSource.next();
        final mutationJson = buildMutation(
          StoryDraftMutationContext(
            projectId: latest.projectId,
            revision: latest.revision,
            ids: ids,
          ),
        );
        final result = await _ffi.authoringProjectStoryDraftInsertV1(
          projectJson: latestProjectJson,
          mutationJson: mutationJson,
          profile: _session.profile,
        );
        switch (result) {
          case AuthoringStoryDraftInsertRejected rejected:
            return ManagedProjectDerivedRejection<StoryDraftCreateResult>(
              StoryDraftCreateRejected(
                state: latest,
                diagnostics: rejected.diagnostics,
              ),
            );
          case AuthoringStoryDraftInsertApplied applied:
            if (applied.draftId != ids.draftId ||
                applied.scriptModuleId != ids.scriptModuleId ||
                applied.draftKind != expectedKind) {
              throw const FormatException(
                'Story mutation builder changed its allocated identity or kind',
              );
            }
            final candidate = StoryWorkspaceState.fromCanonicalProjectJson(
              applied.projectJson,
              blocksBuild: applied.blocksBuild,
              diagnostics: applied.diagnostics,
            );
            final draft = candidate.draftById(applied.draftId);
            if (draft == null ||
                draft.scriptModuleId != applied.scriptModuleId ||
                draft.kind != applied.draftKind ||
                candidate.revision != applied.revision) {
              throw const FormatException(
                'Story candidate projection disagrees with its applied result',
              );
            }
            return ManagedProjectDerivedCandidate<StoryDraftCreateResult>(
              projectJson: applied.projectJson,
              value: StoryDraftCreateApplied(
                state: candidate,
                draft: draft,
                diagnostics: applied.diagnostics,
              ),
            );
        }
      });
}

final class _StoryEntity {
  const _StoryEntity({
    required this.id,
    required this.displayName,
    required this.origin,
    required this.revision,
    required this.kind,
    required this.data,
  });

  final String id;
  final String displayName;
  final Map<String, Object?> origin;
  final int revision;
  final String kind;
  final Map<String, Object?> data;
}

final class _StoryTypedRef {
  const _StoryTypedRef({
    required this.projectId,
    required this.id,
    required this.expectedKind,
  });

  final String projectId;
  final String id;
  final String expectedKind;
}

_StoryTypedRef _requireTypedRef(Object? value, {required String context}) {
  final ref = _requireObject(value, context);
  _requireExactFields(ref, const <String>{
    'project_id',
    'id',
    'expected_kind',
  }, context);
  return _StoryTypedRef(
    projectId: _requireEntityId(ref['project_id'], '$context project_id'),
    id: _requireEntityId(ref['id'], '$context id'),
    expectedKind: _requireBoundedString(
      ref['expected_kind'],
      '$context expected_kind',
      64,
    ),
  );
}

void _requireExactOwner(
  _StoryTypedRef owner, {
  required String projectId,
  required String draftId,
  required String draftKind,
}) {
  if (owner.projectId != projectId ||
      owner.id != draftId ||
      owner.expectedKind != draftKind) {
    throw FormatException(
      'generated ScriptModule owner is not the exact Story Draft',
    );
  }
}

Map<String, Object?> _requireObject(Object? value, String context) {
  if (value is! Map) {
    throw FormatException('$context must be an object');
  }
  return value.cast<String, Object?>();
}

void _requireExactFields(
  Map<String, Object?> value,
  Set<String> expected,
  String context,
) {
  if (value.length != expected.length || !expected.every(value.containsKey)) {
    throw FormatException('$context has an invalid closed schema');
  }
}

String _requireEntityId(Object? value, String context) {
  if (value is! String || !_entityIdPattern.hasMatch(value)) {
    throw FormatException('$context is not a lowercase 128-bit entity ID');
  }
  return value;
}

String _requireSha256(Object? value, String context) {
  if (value is! String || !_sha256Pattern.hasMatch(value)) {
    throw FormatException('$context is not a lowercase SHA-256 digest');
  }
  return value;
}

int _requireInt(
  Object? value,
  String context, {
  required int min,
  required int max,
}) {
  if (value is! int || value < min || value > max) {
    throw FormatException('$context is not a bounded integer');
  }
  return value;
}

String _requireBoundedString(Object? value, String context, int maxBytes) {
  if (value is! String || value.isEmpty) {
    throw FormatException('$context must be a non-empty string');
  }
  _boundedUtf8Length(value, maxBytes, context);
  return value;
}

void _requireExactRevision2BuildGate(
  bool blocksBuild,
  List<AuthoringDiagnostic> diagnostics,
) {
  final diagnosticsBlock = diagnostics.any(
    (diagnostic) => diagnostic.blocksBuild,
  );
  if (blocksBuild != diagnosticsBlock) {
    throw const FormatException(
      'Story workspace blocksBuild disagrees with its diagnostics',
    );
  }
  final hasExactGate = diagnostics.any(
    (diagnostic) =>
        diagnostic.code == 'REVISION2_COMBINED_VALIDATION_UNAVAILABLE' &&
        diagnostic.severity == AuthoringDiagnosticSeverity.error &&
        diagnostic.entity == null &&
        diagnostic.propertyPath == 'schema_revision' &&
        diagnostic.blocksBuild,
  );
  if (!hasExactGate) {
    throw const FormatException(
      'Story workspace is missing its exact revision-2 combined-validation gate',
    );
  }
}

int _boundedUtf8Length(
  String value,
  int maxBytes,
  String context, {
  bool requireNonEmpty = false,
}) {
  if (requireNonEmpty && value.isEmpty) {
    throw FormatException('$context must not be empty');
  }
  var length = 0;
  for (var index = 0; index < value.length; index++) {
    final codeUnit = value.codeUnitAt(index);
    final int encodedLength;
    if (codeUnit <= 0x7f) {
      encodedLength = 1;
    } else if (codeUnit <= 0x7ff) {
      encodedLength = 2;
    } else if (codeUnit >= 0xd800 && codeUnit <= 0xdbff) {
      if (index + 1 >= value.length) {
        throw FormatException('$context contains a malformed UTF-16 surrogate');
      }
      final low = value.codeUnitAt(index + 1);
      if (low < 0xdc00 || low > 0xdfff) {
        throw FormatException('$context contains a malformed UTF-16 surrogate');
      }
      index++;
      encodedLength = 4;
    } else if (codeUnit >= 0xdc00 && codeUnit <= 0xdfff) {
      throw FormatException('$context contains a malformed UTF-16 surrogate');
    } else {
      encodedLength = 3;
    }
    length += encodedLength;
    if (length > maxBytes) {
      throw FormatException('$context exceeds its $maxBytes-byte limit');
    }
  }
  return length;
}
