import 'dart:convert';

import 'package:crypto/crypto.dart' as crypto;
import 'package:gore_mod/core/mod_ffi.dart';
import 'package:gore_mod/project/revision3_content_index.dart';

import 'revision3_quest_fixture.dart';

const revision3QuestOutlineProjectId = '11111111111111111111111111111111';
const revision3QuestOutlineQuestId = '22222222222222222222222222222222';
const revision3QuestOutlineModuleId = '33333333333333333333333333333333';
const revision3QuestOutlineArtifactSha =
    'eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee';
const revision3QuestOutlineTargetSha =
    'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa';
const revision3QuestContextParentCatalogId =
    'g1r:quest-parent:swampcamp_scchapter3';
const revision3QuestContextGiverCatalogId = 'g1r:npc:om_grd_viper_270';
const revision3QuestContextParentRuntimeClass = 'UQuest_SwampCamp_SCChapter3';
const revision3QuestContextGiverRuntimeUniqueName = 'OM_GRD_Viper_270';

final class Revision3QuestOutlineFixture {
  Revision3QuestOutlineFixture({
    this.projectRevision = 7,
    this.questRevision = 4,
    this.moduleRevision = 5,
    this.displayName = 'Find Homer',
    this.title = 'Find Homer',
    this.includeTranscript = false,
    this.objectiveTitles = const <String>[
      'Ask Asghan about Homer',
      'Inspect the old gate',
      'Report the secured gate',
    ],
  });

  final int projectRevision;
  final int questRevision;
  final int moduleRevision;
  final String displayName;
  final String title;
  final bool includeTranscript;
  final List<String> objectiveTitles;

  String get projectJson => jsonEncode(projectObject());

  // A real revision-3 WorkingHead seals the sharded SnapshotManifest, not the
  // monolithic project JSON returned by open. Keep the fixture intentionally
  // different so Dart cannot accidentally conflate those two byte domains.
  AuthoringWorkingHead get head => manifestHead(4096, 'b');

  Map<String, Object?> projectObject() {
    final input = _questInput(title: title, objectiveTitles: objectiveTitles);
    final entities = <String, Object?>{
      revision3QuestOutlineQuestId: _questEntity(
        input: input,
        revision: questRevision,
        displayName: displayName,
      ),
      revision3QuestOutlineModuleId: _moduleEntity(
        input: input,
        revision: moduleRevision,
        displayName: '$displayName Script',
      ),
    };
    return <String, Object?>{
      'format': 2,
      'schema_revision': 3,
      'project_id': revision3QuestOutlineProjectId,
      'revision': projectRevision,
      'meta': <String, Object?>{
        'name': 'Quest outline fixture',
        'version': '1.0.0',
        'author': 'tests',
      },
      'target': _target(),
      'authoring_locales': <Object?>[],
      'entities': entities,
      'asset_store': <String, Object?>{
        'assets': <String, Object?>{
          revision3QuestOutlineArtifactSha: <String, Object?>{
            'byte_len': 123,
            'media_type':
                'application/vnd.gore.quest-collision-capability+json;version=2',
          },
        },
      },
    };
  }

  AuthoringDraftContentSeal get storyCatalogSeal =>
      AuthoringDraftContentSeal.fromJson(_seal(2048, '9'));

  AuthoringRevision3QuestContextEditRequestV1 contextRequest({
    String description = 'Find Homer and report back safely.',
    String parentCatalogId = revision3QuestContextParentCatalogId,
    String giverCatalogId = revision3QuestContextGiverCatalogId,
  }) => AuthoringRevision3QuestContextEditRequestV1.forProject(
    expectedHead: head,
    currentProjectJson: projectJson,
    expectedStoryCatalogSeal: storyCatalogSeal,
    questId: revision3QuestOutlineQuestId,
    expectedQuestRevision: questRevision,
    description: description,
    parentCatalogId: parentCatalogId,
    giverCatalogId: giverCatalogId,
    expectedParentRuntimeClass: revision3QuestContextParentRuntimeClass,
    expectedParentCatalogLayer: 'base-game.quest-parent.v1',
    expectedParentAuthoringSelector: 'SwampCamp_SCChapter3',
    expectedParentSourceSeal: AuthoringDraftContentSeal.fromJson(
      _seal(11, '1'),
    ),
    expectedGiverRuntimeUniqueName: revision3QuestContextGiverRuntimeUniqueName,
    expectedGiverCatalogLayer: 'base-game.npc.v1',
    expectedGiverAuthoringSelector: revision3QuestContextGiverRuntimeUniqueName,
    expectedGiverSourceSeal: AuthoringDraftContentSeal.fromJson(_seal(12, '2')),
  );

  String contextCandidateProjectJson({
    String description = 'Find Homer and report back safely.',
    String parentRuntimeClass = revision3QuestContextParentRuntimeClass,
    String giverRuntimeUniqueName = revision3QuestContextGiverRuntimeUniqueName,
    String parentCatalogLayer = 'base-game.quest-parent.v1',
    String? parentAuthoringSelector,
    Map<String, Object?>? parentSourceSeal,
    String giverCatalogLayer = 'base-game.npc.v1',
    String? giverAuthoringSelector,
    Map<String, Object?>? giverSourceSeal,
  }) {
    final project = projectObject();
    project['revision'] = projectRevision + 1;
    final entities = (project['entities']! as Map).cast<String, Object?>();
    final input = _questInput(
      title: title,
      objectiveTitles: objectiveTitles,
      description: description,
      parentRuntimeClass: parentRuntimeClass,
      giverRuntimeUniqueName: giverRuntimeUniqueName,
      parentCatalogLayer: parentCatalogLayer,
      parentAuthoringSelector: parentAuthoringSelector,
      parentSourceSeal: parentSourceSeal,
      giverCatalogLayer: giverCatalogLayer,
      giverAuthoringSelector: giverAuthoringSelector,
      giverSourceSeal: giverSourceSeal,
    );
    entities[revision3QuestOutlineQuestId] = _questEntity(
      input: input,
      revision: questRevision + 1,
      displayName: displayName,
    );
    entities[revision3QuestOutlineModuleId] = _moduleEntity(
      input: input,
      revision: moduleRevision + 1,
      displayName: '$displayName Script',
    );
    return jsonEncode(project);
  }

  Map<String, Object?> contextResponse({
    String description = 'Find Homer and report back safely.',
    String parentCatalogId = revision3QuestContextParentCatalogId,
    String giverCatalogId = revision3QuestContextGiverCatalogId,
    String parentRuntimeClass = revision3QuestContextParentRuntimeClass,
    String giverRuntimeUniqueName = revision3QuestContextGiverRuntimeUniqueName,
    String parentCatalogLayer = 'base-game.quest-parent.v1',
    String? parentAuthoringSelector,
    Map<String, Object?>? parentSourceSeal,
    String giverCatalogLayer = 'base-game.npc.v1',
    String? giverAuthoringSelector,
    Map<String, Object?>? giverSourceSeal,
  }) => <String, Object?>{
    'ok': true,
    'outcome': 'prepared_unpublished',
    'basis_head_json': head.canonicalJson,
    'head_json': manifestHead(4101, 'd').canonicalJson,
    'project_json': contextCandidateProjectJson(
      description: description,
      parentRuntimeClass: parentRuntimeClass,
      giverRuntimeUniqueName: giverRuntimeUniqueName,
      parentCatalogLayer: parentCatalogLayer,
      parentAuthoringSelector: parentAuthoringSelector,
      parentSourceSeal: parentSourceSeal,
      giverCatalogLayer: giverCatalogLayer,
      giverAuthoringSelector: giverAuthoringSelector,
      giverSourceSeal: giverSourceSeal,
    ),
    'project_id': revision3QuestOutlineProjectId,
    'revision': projectRevision + 1,
    'quest_id': revision3QuestOutlineQuestId,
    'module_id': revision3QuestOutlineModuleId,
    'quest_revision': questRevision + 1,
    'module_revision': moduleRevision + 1,
    'story_catalog_seal': _seal(2048, '9'),
    'parent_catalog_id': parentCatalogId,
    'giver_catalog_id': giverCatalogId,
    'build_status': 'blocked',
    'runtime_status': 'runtime_unqualified',
    'publication_status': 'not_supported',
  };

  Revision3ContentIndex
  contentIndex() => Revision3ContentIndex.fromJsonObject(<String, Object?>{
    'schema_revision': 1,
    'project_id': revision3QuestOutlineProjectId,
    'project_revision': projectRevision,
    'project_name': 'Quest outline fixture',
    'project_version': '1.0.0',
    'project_author': 'tests',
    'target': _target(),
    'authoring_locales': <Object?>[],
    'entity_counts': <String, Object?>{'quest_draft': 1, 'script_module': 1},
    'entities': <Object?>[
      <String, Object?>{
        'id': revision3QuestOutlineQuestId,
        'kind': 'quest_draft',
        'display_name': displayName,
        'revision': questRevision,
        'origin': <String, Object?>{
          'type': 'new',
          'authored_runtime_id': 'GORE_FIND_HOMER',
        },
        'summary': <String, Object?>{
          'kind': 'quest_draft',
          'data': <String, Object?>{
            'technical_id': 'GORE_FIND_HOMER',
            'title': title,
            'objective_title': objectiveTitles.first,
            if (objectiveTitles.length > 1)
              'additional_objective_titles': objectiveTitles.skip(1).toList(),
            'objective_slots': List<int>.generate(
              objectiveTitles.length,
              (index) => index + 1,
            ),
            'transcript_count': 0,
            'module_namespace': 'PROJECT.QUESTS.FINDHOMER',
            'parent_runtime_class': 'UQuest_SwampCamp_SCChapter2',
            'giver_runtime_unique_name': 'OM_GRD_Asghan_263',
          },
        },
        'references': <Object?>[
          _contentReference(
            role: 'draft_script_module',
            targetId: revision3QuestOutlineModuleId,
            expectedKind: 'script_module',
          ),
        ],
        'asset_references': <Object?>[
          <String, Object?>{
            'role': 'quest_collision_artifact',
            'sha256': revision3QuestOutlineArtifactSha,
            'byte_len': 123,
            'logical_name': null,
            'expected_media_type':
                'application/vnd.gore.quest-collision-capability+json;version=2',
            'resolution': 'resolved',
          },
        ],
      },
      <String, Object?>{
        'id': revision3QuestOutlineModuleId,
        'kind': 'script_module',
        'display_name': '$displayName Script',
        'revision': moduleRevision,
        'origin': <String, Object?>{
          'type': 'generated',
          'generator_id': 'gore-authoring.draft-quest-skeleton',
          'generator_version': 4,
          'owner': <String, Object?>{
            'project_id': revision3QuestOutlineProjectId,
            'entity_id': revision3QuestOutlineQuestId,
            'expected_kind': 'quest_draft',
          },
        },
        'summary': <String, Object?>{
          'kind': 'script_module',
          'data': <String, Object?>{
            'generator_id': 'gore-authoring.draft-quest-skeleton',
            'generator_version': 4,
            'module_namespace': 'PROJECT.QUESTS.FINDHOMER',
            'module_relative_path': 'PROJECT/QUESTS/FINDHOMER.as',
            'status': <String, Object?>{
              'authoring': 'offline_draft',
              'runtime': 'runtime_unqualified',
            },
          },
        },
        'references': <Object?>[
          _contentReference(
            role: 'origin_owner',
            targetId: revision3QuestOutlineQuestId,
            expectedKind: 'quest_draft',
          ),
          _contentReference(
            role: 'script_owner',
            targetId: revision3QuestOutlineQuestId,
            expectedKind: 'quest_draft',
          ),
        ],
        'asset_references': <Object?>[],
      },
    ],
    'assets': <Object?>[
      <String, Object?>{
        'sha256': revision3QuestOutlineArtifactSha,
        'byte_len': 123,
        'media_type':
            'application/vnd.gore.quest-collision-capability+json;version=2',
        'class': 'quest_collision_artifact',
      },
    ],
  });

  Map<String, Object?> _questInput({
    required String title,
    required List<String> objectiveTitles,
    String description =
        'Find the missing worker without changing runtime logic.',
    String parentRuntimeClass = 'UQuest_SwampCamp_SCChapter2',
    String giverRuntimeUniqueName = 'OM_GRD_Asghan_263',
    String parentCatalogLayer = 'base-game.quest-parent.v1',
    String? parentAuthoringSelector,
    Map<String, Object?>? parentSourceSeal,
    String giverCatalogLayer = 'base-game.npc.v1',
    String? giverAuthoringSelector,
    Map<String, Object?>? giverSourceSeal,
  }) => <String, Object?>{
    'target': _target(),
    'quest_id': revision3QuestOutlineQuestId,
    'module_namespace': 'PROJECT.QUESTS.FINDHOMER',
    'technical_id': 'GORE_FIND_HOMER',
    'text_helper': 'GoreFindHomerText',
    'parent_quest': <String, Object?>{
      'generation': _target(),
      'source_seal': parentSourceSeal ?? _seal(11, '1'),
      'catalog_layer': parentCatalogLayer,
      'canonical_selector':
          parentAuthoringSelector ??
          parentRuntimeClass.substring('UQuest_'.length),
      'runtime_class': parentRuntimeClass,
    },
    'giver': <String, Object?>{
      'generation': _target(),
      'source_seal': giverSourceSeal ?? _seal(12, '2'),
      'catalog_layer': giverCatalogLayer,
      'canonical_selector': giverAuthoringSelector ?? giverRuntimeUniqueName,
      'runtime_unique_name': giverRuntimeUniqueName,
    },
    'title': title,
    'description': description,
    'objective_title': objectiveTitles.first,
    if (objectiveTitles.length > 1)
      'additional_objective_titles': objectiveTitles.skip(1).toList(),
    'transition_plan':
        AuthoringRevision3QuestTransitionPlanV1.defaultForObjectives(
          objectiveTitles.length,
        ).toJson(),
    'collision_catalog': <String, Object?>{
      'generation': _target(),
      'catalog_layer':
          'base-game-plus-exact-revision3-project.story-collisions.v2',
      'artifact': _seal(123, 'e'),
      'source_seal': _seal(123, 'f'),
      'basis_snapshot': _seal(4096, 'b'),
    },
  };

  Map<String, Object?> _questEntity({
    required Map<String, Object?> input,
    required int revision,
    required String displayName,
  }) => <String, Object?>{
    'id': revision3QuestOutlineQuestId,
    'display_name': displayName,
    'origin': <String, Object?>{
      'type': 'new',
      'authored_runtime_id': 'GORE_FIND_HOMER',
    },
    'revision': revision,
    'payload': <String, Object?>{
      'kind': 'quest_draft',
      'data': <String, Object?>{
        'generator_id': 'gore-authoring.draft-quest-skeleton',
        'generator_version': 4,
        'input': input,
        'script_module': <String, Object?>{
          'project_id': revision3QuestOutlineProjectId,
          'id': revision3QuestOutlineModuleId,
          'expected_kind': 'script_module',
        },
        if (includeTranscript)
          'transcript': <Object?>[
            <String, Object?>{
              'line': <String, Object?>{
                'project_id': revision3QuestOutlineProjectId,
                'id': '44444444444444444444444444444444',
                'expected_kind': 'dialog_line',
              },
              'objective_slot': 1,
            },
          ],
      },
    },
  };

  Map<String, Object?> _moduleEntity({
    required Map<String, Object?> input,
    required int revision,
    required String displayName,
  }) {
    final source = revision3QuestGeneratedSource(
      technicalId: 'GORE_FIND_HOMER',
      textHelper: 'GoreFindHomerText',
      parentRuntimeClass:
          ((input['parent_quest']! as Map)['runtime_class']! as String),
      giverRuntimeUniqueName:
          ((input['giver']! as Map)['runtime_unique_name']! as String),
      title: input['title']! as String,
      description: input['description']! as String,
      objectiveTitle: input['objective_title']! as String,
      additionalObjectiveTitles:
          (input['additional_objective_titles'] as List?)?.cast<String>() ??
          const <String>[],
    );
    return <String, Object?>{
      'id': revision3QuestOutlineModuleId,
      'display_name': displayName,
      'origin': <String, Object?>{
        'type': 'generated',
        'generator_id': 'gore-authoring.draft-quest-skeleton',
        'generator_version': 4,
        'owner': <String, Object?>{
          'project_id': revision3QuestOutlineProjectId,
          'id': revision3QuestOutlineQuestId,
          'expected_kind': 'quest_draft',
        },
      },
      'revision': revision,
      'payload': <String, Object?>{
        'kind': 'script_module',
        'data': <String, Object?>{
          'generator_id': 'gore-authoring.draft-quest-skeleton',
          'generator_version': 4,
          'owner': <String, Object?>{
            'project_id': revision3QuestOutlineProjectId,
            'id': revision3QuestOutlineQuestId,
            'expected_kind': 'quest_draft',
          },
          'module_namespace': 'PROJECT.QUESTS.FINDHOMER',
          'module_relative_path': 'PROJECT/QUESTS/FINDHOMER.as',
          'source': source,
          'source_sha256': crypto.sha256
              .convert(utf8.encode(source))
              .toString(),
          'input_fingerprint': revision3QuestInputFingerprint(input),
          'status': <String, Object?>{
            'authoring': 'offline_draft',
            'runtime': 'runtime_unqualified',
          },
        },
      },
    };
  }
}

AuthoringWorkingHead headFor(String projectJson) =>
    AuthoringWorkingHead.fromCanonicalJson(
      jsonEncode(<String, Object?>{
        'store_format': 1,
        'snapshot': <String, Object?>{
          'byte_len': utf8.encode(projectJson).length,
          'sha256': crypto.sha256.convert(utf8.encode(projectJson)).toString(),
        },
      }),
    );

AuthoringWorkingHead manifestHead(int byteLength, String digit) =>
    AuthoringWorkingHead.fromCanonicalJson(
      jsonEncode(<String, Object?>{
        'store_format': 1,
        'snapshot': <String, Object?>{
          'byte_len': byteLength,
          'sha256': List<String>.filled(64, digit).join(),
        },
      }),
    );

Map<String, Object?> _target() => <String, Object?>{
  'executable': <String, Object?>{
    'byte_len': 171698176,
    'sha256': revision3QuestOutlineTargetSha,
  },
};

Map<String, Object?> _seal(int bytes, String digit) => <String, Object?>{
  'byte_len': bytes,
  'sha256': List<String>.filled(64, digit).join(),
};

Map<String, Object?> _contentReference({
  required String role,
  required String targetId,
  required String expectedKind,
}) => <String, Object?>{
  'role': role,
  'qualifier': null,
  'target': <String, Object?>{
    'project_id': revision3QuestOutlineProjectId,
    'entity_id': targetId,
    'expected_kind': expectedKind,
  },
  'resolution': 'resolved',
};
