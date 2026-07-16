import 'dart:convert';

import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/core/core_service.dart';
import 'package:gore_mod/core/mod_ffi.dart';

const _projectId = '11111111111111111111111111111111';
const _draftId = '22222222222222222222222222222222';
const _moduleId = '33333333333333333333333333333333';
const _sentinelId = '44444444444444444444444444444444';

Map<String, Object?> _ref(String id, String kind) => <String, Object?>{
  'project_id': _projectId,
  'id': id,
  'expected_kind': kind,
};

Map<String, Object?> _entity({
  required String id,
  required String displayName,
  required int revision,
  required Map<String, Object?> origin,
  required String kind,
  required Map<String, Object?> data,
}) => <String, Object?>{
  'id': id,
  'display_name': displayName,
  'origin': origin,
  'revision': revision,
  'payload': <String, Object?>{'kind': kind, 'data': data},
};

Map<String, Object?> _basisProject(AuthoringStoryDraftKind kind) =>
    <String, Object?>{
      'format': 2,
      'schema_revision': 3,
      'project_id': _projectId,
      'revision': 7,
      'meta': <String, Object?>{
        'name': 'Removal fixture',
        'version': '1.0.0',
        'author': 'tests',
      },
      'target': <String, Object?>{
        'executable': <String, Object?>{'byte_len': 1, 'sha256': 'a' * 64},
      },
      'authoring_locales': <Object?>['en'],
      'entities': <String, Object?>{
        _draftId: _entity(
          id: _draftId,
          displayName: kind == AuthoringStoryDraftKind.npcDraft
              ? 'Test NPC'
              : 'Test Quest',
          revision: 3,
          origin: <String, Object?>{'type': 'authored'},
          kind: kind.wireName,
          data: <String, Object?>{
            'generator_id': 'test.story-generator',
            'generator_version': 1,
            'script_module': _ref(_moduleId, 'script_module'),
          },
        ),
        _moduleId: _entity(
          id: _moduleId,
          displayName: 'Generated module',
          revision: 4,
          origin: <String, Object?>{
            'type': 'generated',
            'generator_id': 'test.story-generator',
            'generator_version': 1,
            'owner': _ref(_draftId, kind.wireName),
          },
          kind: 'script_module',
          data: <String, Object?>{
            'generator_id': 'test.story-generator',
            'generator_version': 1,
            'owner': _ref(_draftId, kind.wireName),
            'source_sha256': 'b' * 64,
            'status': <String, Object?>{
              'authoring': 'offline_draft',
              'runtime': 'runtime_unqualified',
            },
          },
        ),
        _sentinelId: _entity(
          id: _sentinelId,
          displayName: 'Retained entity',
          revision: 2,
          origin: <String, Object?>{'type': 'authored'},
          kind: 'dialog_graph',
          data: <String, Object?>{'nodes': <Object?>[]},
        ),
      },
      'asset_store': <String, Object?>{'assets': <String, Object?>{}},
    };

String _headJson(String digit, int bytes) => jsonEncode(<String, Object?>{
  'store_format': 1,
  'snapshot': <String, Object?>{'byte_len': bytes, 'sha256': digit * 64},
});

Map<String, Object?> _fixture(AuthoringStoryDraftKind kind) {
  final basis = _basisProject(kind);
  final candidate = jsonDecode(jsonEncode(basis)) as Map<String, Object?>;
  candidate['revision'] = 8;
  final entities = (candidate['entities']! as Map).cast<String, Object?>();
  entities.remove(_draftId);
  entities.remove(_moduleId);
  return <String, Object?>{
    'basis': basis,
    'candidate': candidate,
    'response': <String, Object?>{
      'ok': true,
      'outcome': 'prepared_remove_unpublished',
      'basis_head_json': _headJson('c', 101),
      'head_json': _headJson('d', 102),
      'project_json': jsonEncode(candidate),
      'project_id': _projectId,
      'revision': 8,
      'removed': <String, Object?>{
        'draft': <String, Object?>{
          'id': _draftId,
          'kind': kind.wireName,
          'revision': 3,
        },
        'script_module': <String, Object?>{
          'id': _moduleId,
          'kind': 'script_module',
          'revision': 4,
        },
      },
      'build_status': 'blocked',
      'runtime_status': 'runtime_unqualified',
      'artifact_authority': 'not_granted',
      'publication_status': 'not_supported',
    },
  };
}

AuthoringRevision3StoryDraftRemovalRequestV1 _request(
  Map<String, Object?> fixture,
  AuthoringStoryDraftKind kind,
) => AuthoringRevision3StoryDraftRemovalRequestV1.forProject(
  currentProjectJson: jsonEncode(fixture['basis']),
  expectedHead: AuthoringWorkingHead.fromCanonicalJson(_headJson('c', 101)),
  draftId: _draftId,
  draftKind: kind,
  expectedDraftRevision: 3,
  scriptModuleId: _moduleId,
  expectedScriptModuleRevision: 4,
);

Map<String, Object?> _copyResponse(Map<String, Object?> fixture) =>
    (jsonDecode(jsonEncode(fixture['response'])) as Map)
        .cast<String, Object?>();

void main() {
  for (final kind in AuthoringStoryDraftKind.values) {
    test(
      '${kind.wireName} removal preserves the exact closed wire contract',
      () async {
        final fixture = _fixture(kind);
        final response = (fixture['response']! as Map).cast<String, Object?>();
        final core = FakeGoreCoreFfiService(
          responses: <String, Map<String, Object?>>{
            'authoring_store_prepare_remove_revision3_story_draft_v1': response,
          },
        );
        final request = _request(fixture, kind);
        final prepared = await ModFfi(core)
            .authoringStorePrepareRemoveRevision3StoryDraftV1(
              root: r'C:\Mods\Story.goreproj',
              currentProjectJson: jsonEncode(fixture['basis']),
              request: request,
            );

        expect(prepared.projectId, _projectId);
        expect(prepared.revision, 8);
        expect(prepared.removedDraft.id, _draftId);
        expect(prepared.removedDraft.kind, kind);
        expect(prepared.removedScriptModule.id, _moduleId);
        expect(
          prepared.buildStatus,
          AuthoringRevision3StoryDraftRemovalBuildStatus.blocked,
        );
        expect(
          prepared.runtimeStatus,
          AuthoringRevision3StoryDraftRemovalRuntimeStatus.runtimeUnqualified,
        );
        expect(
          prepared.artifactAuthority,
          AuthoringRevision3StoryDraftRemovalArtifactAuthority.notGranted,
        );
        expect(
          prepared.publicationStatus,
          AuthoringRevision3StoryDraftRemovalNativePublicationStatus
              .notSupported,
        );
        expect(
          core.calls.single.command,
          'authoring_store_prepare_remove_revision3_story_draft_v1',
        );
        expect(core.calls.single.payload.keys, <String>[
          'current_project_json',
          'root',
          'story_draft_removal_request_json',
        ]);
        expect(core.calls.single.payload, <String, Object?>{
          'current_project_json': jsonEncode(fixture['basis']),
          'root': r'C:\Mods\Story.goreproj',
          'story_draft_removal_request_json': request.canonicalJson,
        });
        expect(jsonDecode(request.canonicalJson), <String, Object?>{
          'expected_head': jsonDecode(_headJson('c', 101)),
          'expected_project_id': _projectId,
          'expected_revision': 7,
          'expected_target': (fixture['basis']! as Map)['target'],
          'draft_id': _draftId,
          'draft_kind': kind.wireName,
          'expected_draft_revision': 3,
          'script_module_id': _moduleId,
          'expected_script_module_revision': 4,
        });
      },
    );
  }

  test(
    'response drift and expanded authority are rejected as malformed',
    () async {
      final fixture = _fixture(AuthoringStoryDraftKind.npcDraft);
      final request = _request(fixture, AuthoringStoryDraftKind.npcDraft);
      final mutations = <void Function(Map<String, Object?>)>[
        (response) => response['build_status'] = 'ready',
        (response) => response['publication_status'] = 'published',
        (response) => response['receipt_path'] = r'C:\secret.json',
        (response) {
          final removed = (response['removed']! as Map).cast<String, Object?>();
          final draft = (removed['draft']! as Map).cast<String, Object?>();
          draft['extra'] = true;
        },
        (response) {
          final candidate =
              jsonDecode(response['project_json']! as String)
                  as Map<String, Object?>;
          final entities = (candidate['entities']! as Map)
              .cast<String, Object?>();
          final sentinel = (entities[_sentinelId]! as Map)
              .cast<String, Object?>();
          sentinel['display_name'] = 'Drifted';
          response['project_json'] = jsonEncode(candidate);
        },
      ];

      for (final mutate in mutations) {
        final response = _copyResponse(fixture);
        mutate(response);
        final core = FakeGoreCoreFfiService(
          responses: <String, Map<String, Object?>>{
            'authoring_store_prepare_remove_revision3_story_draft_v1': response,
          },
        );
        await expectLater(
          ModFfi(core).authoringStorePrepareRemoveRevision3StoryDraftV1(
            root: r'C:\Mods\Story.goreproj',
            currentProjectJson: jsonEncode(fixture['basis']),
            request: request,
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

  test('a generated module shared by another Draft is never removable', () {
    final fixture = _fixture(AuthoringStoryDraftKind.questDraft);
    final basis = (fixture['basis']! as Map).cast<String, Object?>();
    final entities = (basis['entities']! as Map).cast<String, Object?>();
    const claimantId = '55555555555555555555555555555555';
    entities[claimantId] = _entity(
      id: claimantId,
      displayName: 'Other quest',
      revision: 1,
      origin: <String, Object?>{'type': 'authored'},
      kind: 'quest_draft',
      data: <String, Object?>{
        'generator_id': 'test.story-generator',
        'generator_version': 1,
        'script_module': _ref(_moduleId, 'script_module'),
      },
    );
    final candidate = jsonDecode(jsonEncode(basis)) as Map<String, Object?>;
    candidate['revision'] = 8;
    final candidateEntities = (candidate['entities']! as Map)
        .cast<String, Object?>();
    candidateEntities.remove(_draftId);
    candidateEntities.remove(_moduleId);
    final response = _copyResponse(fixture)
      ..['project_json'] = jsonEncode(candidate);
    final request = AuthoringRevision3StoryDraftRemovalRequestV1.forProject(
      currentProjectJson: jsonEncode(basis),
      expectedHead: AuthoringWorkingHead.fromCanonicalJson(_headJson('c', 101)),
      draftId: _draftId,
      draftKind: AuthoringStoryDraftKind.questDraft,
      expectedDraftRevision: 3,
      scriptModuleId: _moduleId,
      expectedScriptModuleRevision: 4,
    );

    expect(
      () => AuthoringRevision3StoryDraftRemovalPreparation.fromJson(
        response,
        currentProjectJson: jsonEncode(basis),
        request: request,
      ),
      throwsFormatException,
    );
  });
}
