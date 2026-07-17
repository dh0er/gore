import 'dart:convert';
import 'dart:io';

import 'package:crypto/crypto.dart' as crypto;
import 'package:gore_mod/core/mod_ffi.dart';
import 'package:gore_mod/project/managed_project_session.dart';

import '../support/revision3_voice_fixture.dart';

const voiceBatchTestLocale = 'de';
const voiceBatchTestGameRoot = r'C:\Games\Gothic 1 Remake';
const voiceBatchTestSourceFolder = r'C:\Recordings\German';
const voiceBatchTestSourceName = 'GRD_263_ASGHAN_OPEN_INFO_06_02.ogg';
const voiceBatchTestManifestSha =
    'bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb';
const voiceBatchTestPlanSha =
    'cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc';
const voiceBatchTestExistingSlotId = '00000000000000000000000000100000';

AuthoringWorkingHead voiceBatchHeadFor(String projectJson) =>
    AuthoringWorkingHead.fromCanonicalJson(
      jsonEncode(<String, Object?>{
        'store_format': 1,
        'snapshot': <String, Object?>{
          'byte_len': utf8.encode(projectJson).length,
          'sha256': crypto.sha256.convert(utf8.encode(projectJson)).toString(),
        },
      }),
    );

AuthoringWorkingHead
voiceBatchDifferentHead() => AuthoringWorkingHead.fromCanonicalJson(
  jsonEncode(<String, Object?>{
    'store_format': 1,
    'snapshot': <String, Object?>{
      'byte_len': 1,
      'sha256':
          'dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd',
    },
  }),
);

final class Revision3VoiceBatchTestFixture {
  Revision3VoiceBatchTestFixture._({
    required this.projectJson,
    required this.basisHead,
    required this.request,
    required this.candidateProjectJson,
    required this.candidateHead,
    required this.slotCreated,
  });

  factory Revision3VoiceBatchTestFixture.create({
    String? projectJson,
    AuthoringWorkingHead? basisHead,
    bool existingSlot = false,
  }) {
    final basisProject =
        projectJson ??
        (existingSlot
            ? revision3VoiceFixtureProjectWithVoiceSlotCountJson(1)
            : revision3VoiceFixtureProjectJson());
    final head = basisHead ?? voiceBatchHeadFor(basisProject);
    final slotId = existingSlot
        ? voiceBatchTestExistingSlotId
        : revision3VoiceFixtureSlotId;
    final request = AuthoringRevision3VoiceTakeRequestV1.forProject(
      expectedHead: head,
      currentProjectJson: basisProject,
      lineId: revision3VoiceFixtureLineId,
      slotId: slotId,
      takeId: revision3VoiceFixtureTakeId,
      locale: voiceBatchTestLocale,
      takeDisplayName: 'Asghan German recording',
      logicalName: voiceBatchTestSourceName,
      status: AuthoringRevision3VoiceTakeStatus.recorded,
    );
    final candidateProjectJson = _voiceBatchCandidateProjectJson(
      basisProject,
      request,
    );
    return Revision3VoiceBatchTestFixture._(
      projectJson: basisProject,
      basisHead: head,
      request: request,
      candidateProjectJson: candidateProjectJson,
      candidateHead: voiceBatchHeadFor(candidateProjectJson),
      slotCreated: !existingSlot,
    );
  }

  final String projectJson;
  final AuthoringWorkingHead basisHead;
  final AuthoringRevision3VoiceTakeRequestV1 request;
  final String candidateProjectJson;
  final AuthoringWorkingHead candidateHead;
  final bool slotCreated;

  Map<String, Object?> readyItemResponse() => <String, Object?>{
    'source_name': voiceBatchTestSourceName,
    'status': 'ready',
    'line_display_name': 'Asghan greeting',
    'speaker': 'Asghan',
    'line_id': revision3VoiceFixtureLineId,
    'localization_id': revision3VoiceFixtureLocalizationId,
    'loc_id': 'GRD_263_ASGHAN_OPEN_INFO_06_02',
    'slot_id': request.slotId,
    'take_id': revision3VoiceFixtureTakeId,
    'slot_created': slotCreated,
    'voice_request_json': request.canonicalJson,
    'asset': <String, Object?>{
      'sha256': revision3VoiceFixtureAssetSha256,
      'byte_len': 100,
      'logical_name': voiceBatchTestSourceName,
    },
    'ogg': <String, Object?>{
      'codec': 'vorbis',
      'channels': 1,
      'sample_rate': 48000,
      'pages': 3,
      'logical_streams': 1,
    },
  };

  Map<String, Object?> planResponse() => <String, Object?>{
    'ok': true,
    'outcome': 'planned',
    'basis_head_json': basisHead.canonicalJson,
    'project_id': revision3VoiceFixtureProjectId,
    'revision': 7,
    'locale': voiceBatchTestLocale,
    'source_manifest_sha256': voiceBatchTestManifestSha,
    'plan_sha256': voiceBatchTestPlanSha,
    'status': 'ready',
    'scanned_entry_count': 1,
    'ogg_file_count': 1,
    'ready_count': 1,
    'already_present_count': 0,
    'blocked_count': 0,
    'ignored_entry_count': 0,
    'items': <Object?>[readyItemResponse()],
    'build_status': 'blocked',
    'runtime_status': 'runtime_unqualified',
    'target_authority': 'not_granted',
    'publication_status': 'not_supported',
  };

  AuthoringRevision3VoiceBatchPlanResult plan() =>
      AuthoringRevision3VoiceBatchPlanResult.fromJson(
        planResponse(),
        expectedHead: basisHead,
        currentProjectJson: projectJson,
        expectedLocale: voiceBatchTestLocale,
      );

  Map<String, Object?> preparationItemResponse() => <String, Object?>{
    'source_name': voiceBatchTestSourceName,
    'line_id': revision3VoiceFixtureLineId,
    'localization_id': revision3VoiceFixtureLocalizationId,
    'slot_id': request.slotId,
    'take_id': revision3VoiceFixtureTakeId,
    'take_status': 'recorded',
    'slot_created': slotCreated,
    'selected': false,
    'asset': <String, Object?>{
      'sha256': revision3VoiceFixtureAssetSha256,
      'byte_len': 100,
      'logical_name': voiceBatchTestSourceName,
    },
    'ogg': <String, Object?>{
      'codec': 'vorbis',
      'channels': 1,
      'sample_rate': 48000,
      'pages': 3,
      'logical_streams': 1,
    },
    'asset_deduplicated': false,
  };

  Map<String, Object?> preparationResponse() => <String, Object?>{
    'ok': true,
    'outcome': 'prepared_unpublished',
    'basis_head_json': basisHead.canonicalJson,
    'head_json': candidateHead.canonicalJson,
    'project_json': candidateProjectJson,
    'project_id': revision3VoiceFixtureProjectId,
    'revision': 8,
    'locale': voiceBatchTestLocale,
    'source_manifest_sha256': voiceBatchTestManifestSha,
    'plan_sha256': voiceBatchTestPlanSha,
    'imported_count': 1,
    'already_present_count': 0,
    'items': <Object?>[preparationItemResponse()],
    'build_status': 'blocked',
    'runtime_status': 'runtime_unqualified',
    'target_authority': 'not_granted',
    'publication_status': 'not_supported',
  };

  AuthoringRevision3VoiceBatchPreparation preparation({
    AuthoringRevision3VoiceBatchPlanResult? forPlan,
  }) => AuthoringRevision3VoiceBatchPreparation.fromJson(
    preparationResponse(),
    currentProjectJson: projectJson,
    plan: forPlan ?? plan(),
  );
}

Map<String, Object?> voiceBatchCollisionPlanResponse({
  required Revision3VoiceBatchTestFixture fixture,
  bool reverse = false,
}) {
  Map<String, Object?> item(String sourceName) => <String, Object?>{
    'source_name': sourceName,
    'status': 'case_collision',
    'line_display_name': null,
    'speaker': null,
    'line_id': null,
    'localization_id': null,
    'loc_id': null,
    'slot_id': null,
    'take_id': null,
    'slot_created': null,
    'voice_request_json': null,
    'asset': null,
    'ogg': null,
  };

  final items = <Object?>[item('LINE.ogg'), item('line.ogg')];
  if (reverse) items.setAll(0, items.reversed.toList(growable: false));
  return <String, Object?>{
    'ok': true,
    'outcome': 'planned',
    'basis_head_json': fixture.basisHead.canonicalJson,
    'project_id': revision3VoiceFixtureProjectId,
    'revision': 7,
    'locale': voiceBatchTestLocale,
    'source_manifest_sha256': voiceBatchTestManifestSha,
    'plan_sha256': voiceBatchTestPlanSha,
    'status': 'blocked',
    'scanned_entry_count': 2,
    'ogg_file_count': 2,
    'ready_count': 0,
    'already_present_count': 0,
    'blocked_count': 2,
    'ignored_entry_count': 0,
    'items': items,
    'build_status': 'blocked',
    'runtime_status': 'runtime_unqualified',
    'target_authority': 'not_granted',
    'publication_status': 'not_supported',
  };
}

String _voiceBatchCandidateProjectJson(
  String basisProjectJson,
  AuthoringRevision3VoiceTakeRequestV1 request,
) {
  final project = (jsonDecode(basisProjectJson) as Map).cast<String, Object?>();
  project['revision'] = request.expectedRevision + 1;
  project['authoring_locales'] = <Object?>[request.locale];

  final entities = (project['entities']! as Map).cast<String, Object?>();
  final line = (entities[request.lineId]! as Map).cast<String, Object?>();
  final linePayload = (line['payload']! as Map).cast<String, Object?>();
  final lineData = (linePayload['data']! as Map).cast<String, Object?>();
  final slots = (lineData['voice_slots']! as Map).cast<String, Object?>();
  final takeRef = <String, Object?>{
    'project_id': request.expectedProjectId,
    'id': request.takeId,
    'expected_kind': 'voice_take',
  };
  if (slots[request.locale] == null) {
    slots[request.locale] = <String, Object?>{
      'project_id': request.expectedProjectId,
      'id': request.slotId,
      'expected_kind': 'voice_slot',
    };
    line['revision'] = (line['revision']! as int) + 1;
    entities[request.slotId] = <String, Object?>{
      'id': request.slotId,
      'display_name': 'Voice ${request.locale}',
      'origin': <String, Object?>{
        'type': 'generated',
        'generator_id': 'gore-authoring.voice-slot',
        'generator_version': 1,
        'owner': <String, Object?>{
          'project_id': request.expectedProjectId,
          'id': request.lineId,
          'expected_kind': 'dialog_line',
        },
      },
      'revision': 0,
      'payload': <String, Object?>{
        'kind': 'voice_slot',
        'data': <String, Object?>{
          'locale': request.locale,
          'target_resolution': <String, Object?>{'state': 'unresolved'},
          'candidates': <Object?>[takeRef],
        },
      },
    };
  } else {
    final slot = (entities[request.slotId]! as Map).cast<String, Object?>();
    final slotPayload = (slot['payload']! as Map).cast<String, Object?>();
    final slotData = (slotPayload['data']! as Map).cast<String, Object?>();
    final candidates = (slotData['candidates']! as List).cast<Object?>()
      ..add(takeRef);
    slotData['candidates'] = candidates;
    slot['revision'] = (slot['revision']! as int) + 1;
  }
  entities[request.takeId] = <String, Object?>{
    'id': request.takeId,
    'display_name': request.takeDisplayName,
    'origin': <String, Object?>{
      'type': 'imported',
      'importer': 'gore-authoring.ogg-import',
      'source_seal': <String, Object?>{
        'byte_len': 100,
        'sha256': revision3VoiceFixtureAssetSha256,
      },
    },
    'revision': 0,
    'payload': <String, Object?>{
      'kind': 'voice_take',
      'data': <String, Object?>{
        'locale': request.locale,
        'asset': <String, Object?>{
          'sha256': revision3VoiceFixtureAssetSha256,
          'byte_len': 100,
          'logical_name': request.logicalName,
        },
        'ogg': <String, Object?>{
          'codec': 'vorbis',
          'channels': 1,
          'sample_rate': 48000,
          'pages': 3,
          'logical_streams': 1,
        },
        'status': 'recorded',
      },
    },
  };
  final assetStore = (project['asset_store']! as Map).cast<String, Object?>();
  final assets = (assetStore['assets']! as Map).cast<String, Object?>();
  assets[revision3VoiceFixtureAssetSha256] = <String, Object?>{
    'byte_len': 100,
    'media_type': 'audio/ogg',
  };
  return jsonEncode(project);
}

Map<String, Object?> voiceBatchDeepCopy(Map<String, Object?> value) =>
    (jsonDecode(jsonEncode(value)) as Map).cast<String, Object?>();

AuthoringRevision3StoreOpenedResult voiceBatchOpened(
  AuthoringWorkingHead head,
  String projectJson,
) => AuthoringRevision3StoreOpenedResult.fromJson(<String, Object?>{
  'ok': true,
  'head_json': head.canonicalJson,
  'project_json': projectJson,
});

AuthoringRevision3CheckpointPreparation voiceBatchCheckpointPreparation(
  AuthoringWorkingHead head,
) => AuthoringRevision3CheckpointPreparation.fromJson(<String, Object?>{
  'ok': true,
  'head_json': head.canonicalJson,
});

final class VoiceBatchManagedStore
    implements ManagedRevision3AuthoringStore, ManagedRevision3VoiceBatchStore {
  VoiceBatchManagedStore(this.fixture);

  final Revision3VoiceBatchTestFixture fixture;
  int planCalls = 0;
  int prepareBatchCalls = 0;
  bool throwUncertainPrepare = false;
  AuthoringRevision3VoiceBatchPlanResult? planResult;

  String? receivedPlanRoot;
  String? receivedPlanGameRoot;
  String? receivedPlanSourceFolder;
  String? receivedPlanLocale;
  String? receivedPlanProjectJson;
  AuthoringWorkingHead? receivedPlanHead;

  @override
  Future<AuthoringRevision3StoreOpenedResult> open({
    required String root,
    required AuthoringAssetVerification verification,
  }) async {
    final headJson = await File(
      '$root${Platform.pathSeparator}gore-project.json',
    ).readAsString();
    if (headJson == fixture.basisHead.canonicalJson) {
      return voiceBatchOpened(fixture.basisHead, fixture.projectJson);
    }
    if (headJson == fixture.candidateHead.canonicalJson) {
      return voiceBatchOpened(
        fixture.candidateHead,
        fixture.candidateProjectJson,
      );
    }
    throw StateError('unknown published test head');
  }

  @override
  Future<AuthoringRevision3CheckpointPreparation> prepareCheckpoint({
    required String root,
    required AuthoringWorkingHead? expectedHead,
    required String projectJson,
  }) async {
    if (expectedHead != null || projectJson != fixture.projectJson) {
      throw StateError('unexpected initial checkpoint');
    }
    return voiceBatchCheckpointPreparation(fixture.basisHead);
  }

  @override
  Future<AuthoringRevision3StoreOpenedResult> openHeadBytes({
    required String root,
    required AuthoringWorkingHead head,
    required AuthoringAssetVerification verification,
  }) async {
    if (head.canonicalJson == fixture.basisHead.canonicalJson) {
      return voiceBatchOpened(fixture.basisHead, fixture.projectJson);
    }
    if (head.canonicalJson == fixture.candidateHead.canonicalJson) {
      return voiceBatchOpened(
        fixture.candidateHead,
        fixture.candidateProjectJson,
      );
    }
    throw StateError('unknown prepared test head');
  }

  @override
  Future<AuthoringRevision3VoiceBatchPlanResult> planVoiceBatchV1({
    required String root,
    required String gameRoot,
    required String sourceFolder,
    required String locale,
    required String currentProjectJson,
    required AuthoringWorkingHead expectedHead,
  }) async {
    planCalls++;
    receivedPlanRoot = root;
    receivedPlanGameRoot = gameRoot;
    receivedPlanSourceFolder = sourceFolder;
    receivedPlanLocale = locale;
    receivedPlanProjectJson = currentProjectJson;
    receivedPlanHead = expectedHead;
    return planResult ?? fixture.plan();
  }

  @override
  Future<AuthoringRevision3VoiceBatchPreparation> prepareVoiceBatchV1({
    required String root,
    required String gameRoot,
    required String sourceFolder,
    required String currentProjectJson,
    required AuthoringRevision3VoiceBatchPlanResult plan,
  }) async {
    prepareBatchCalls++;
    if (throwUncertainPrepare) {
      throw StateError('prepare completion is uncertain');
    }
    return fixture.preparation(forPlan: plan);
  }

  @override
  dynamic noSuchMethod(Invocation invocation) => throw UnsupportedError(
    'unused managed-store member: ${invocation.memberName}',
  );
}
