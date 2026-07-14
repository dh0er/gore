import 'dart:collection';
import 'dart:convert';

import 'package:crypto/crypto.dart' as crypto;
import 'package:gore_mod/core/mod_ffi.dart';

const revision3VoiceFixtureProjectId = '00000000000000000000000000000003';
const revision3VoiceFixtureLocalizationId = '00000000000000000000000000000021';
const revision3VoiceFixtureLineId = '00000000000000000000000000000022';
const revision3VoiceFixtureSlotId = '00000000000000000000000000000023';
const revision3VoiceFixtureTakeId = '00000000000000000000000000000024';
const revision3VoiceFixtureAssetSha256 =
    'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa';

String revision3VoiceFixtureProjectJson({int revision = 7}) => jsonEncode(
  <String, Object?>{
    'format': 2,
    'schema_revision': 3,
    'project_id': revision3VoiceFixtureProjectId,
    'revision': revision,
    'meta': <String, Object?>{
      'name': 'R3 Voice fixture',
      'version': '1.0.0',
      'author': 'tests',
    },
    'target': <String, Object?>{
      'executable': <String, Object?>{
        'byte_len': 123,
        'sha256':
            '4444444444444444444444444444444444444444444444444444444444444444',
      },
    },
    'authoring_locales': <Object?>[],
    'entities': SplayTreeMap<String, Object?>.from(<String, Object?>{
      revision3VoiceFixtureLocalizationId: <String, Object?>{
        'id': revision3VoiceFixtureLocalizationId,
        'display_name': 'Asghan line text',
        'origin': _importedOrigin('2'),
        'revision': 4,
        'payload': <String, Object?>{
          'kind': 'localization_entry',
          'data': <String, Object?>{
            'loc_id': 'GRD_263_ASGHAN_OPEN_INFO_06_02',
            'texts': <String, Object?>{},
          },
        },
      },
      revision3VoiceFixtureLineId: <String, Object?>{
        'id': revision3VoiceFixtureLineId,
        'display_name': 'Asghan greeting',
        'origin': _importedOrigin('3'),
        'revision': 2,
        'payload': <String, Object?>{
          'kind': 'dialog_line',
          'data': <String, Object?>{
            'localization': <String, Object?>{
              'project_id': revision3VoiceFixtureProjectId,
              'id': revision3VoiceFixtureLocalizationId,
              'expected_kind': 'localization_entry',
            },
            'speaker_hint': 'Asghan',
            'voice_slots': <String, Object?>{},
          },
        },
      },
    }),
    'asset_store': <String, Object?>{'assets': <String, Object?>{}},
  },
);

Map<String, Object?> _importedOrigin(String digit) => <String, Object?>{
  'type': 'imported',
  'importer': 'tests',
  'source_seal': <String, Object?>{
    'byte_len': 10,
    'sha256': List<String>.filled(64, digit).join(),
  },
};

final class Revision3VoiceFixture {
  Revision3VoiceFixture._({
    required this.basisHead,
    required this.basisProjectJson,
    required this.request,
    required this.candidateHead,
    required this.candidateProjectJson,
  });

  factory Revision3VoiceFixture.fromBasis({
    required AuthoringWorkingHead basisHead,
    required String basisProjectJson,
    required AuthoringRevision3VoiceTakeRequestV1 request,
  }) {
    final project = (jsonDecode(basisProjectJson) as Map)
        .cast<String, Object?>();
    final entities = SplayTreeMap<String, Object?>.from(
      (project['entities']! as Map).cast<String, Object?>(),
    );

    final localization = (entities[revision3VoiceFixtureLocalizationId]! as Map)
        .cast<String, Object?>();
    final localizationPayload = (localization['payload']! as Map)
        .cast<String, Object?>();
    final localizationData = (localizationPayload['data']! as Map)
        .cast<String, Object?>();
    final texts = SplayTreeMap<String, Object?>.from(
      (localizationData['texts']! as Map).cast<String, Object?>(),
    );
    if (request.text != null) {
      texts[request.locale] = request.text;
      localization['revision'] = (localization['revision']! as int) + 1;
    }
    localizationData['texts'] = texts;
    localizationPayload['data'] = localizationData;
    localization['payload'] = localizationPayload;
    entities[revision3VoiceFixtureLocalizationId] = localization;

    final line = (entities[request.lineId]! as Map).cast<String, Object?>();
    final linePayload = (line['payload']! as Map).cast<String, Object?>();
    final lineData = (linePayload['data']! as Map).cast<String, Object?>();
    final slots = SplayTreeMap<String, Object?>.from(
      (lineData['voice_slots']! as Map).cast<String, Object?>(),
    )..[request.locale] = _typedRef(request.slotId, 'voice_slot');
    lineData['voice_slots'] = slots;
    linePayload['data'] = lineData;
    line['payload'] = linePayload;
    line['revision'] = (line['revision']! as int) + 1;
    entities[request.lineId] = line;

    final takeRef = _typedRef(request.takeId, 'voice_take');
    entities[request.slotId] = <String, Object?>{
      'id': request.slotId,
      'display_name': 'Voice ${request.locale}',
      'origin': <String, Object?>{
        'type': 'generated',
        'generator_id': 'gore-authoring.voice-slot',
        'generator_version': 1,
        'owner': _typedRef(request.lineId, 'dialog_line'),
      },
      'revision': 0,
      'payload': <String, Object?>{
        'kind': 'voice_slot',
        'data': <String, Object?>{
          'locale': request.locale,
          'target_resolution': <String, Object?>{'state': 'unresolved'},
          'candidates': <Object?>[takeRef],
          if (request.selectTake) 'selected': takeRef,
        },
      },
    };
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
          'status': request.status.name,
        },
      },
    };
    project['revision'] = request.expectedRevision + 1;
    project['authoring_locales'] = <Object?>[request.locale];
    project['entities'] = entities;
    project['asset_store'] = <String, Object?>{
      'assets': SplayTreeMap<String, Object?>.from(<String, Object?>{
        revision3VoiceFixtureAssetSha256: <String, Object?>{
          'byte_len': 100,
          'media_type': 'audio/ogg',
        },
      }),
    };
    final candidateProjectJson = jsonEncode(project);
    final candidateHead = _headFor(candidateProjectJson);
    return Revision3VoiceFixture._(
      basisHead: basisHead,
      basisProjectJson: basisProjectJson,
      request: request,
      candidateHead: candidateHead,
      candidateProjectJson: candidateProjectJson,
    );
  }

  final AuthoringWorkingHead basisHead;
  final String basisProjectJson;
  final AuthoringRevision3VoiceTakeRequestV1 request;
  final AuthoringWorkingHead candidateHead;
  final String candidateProjectJson;

  Map<String, Object?> response() => <String, Object?>{
    'ok': true,
    'outcome': 'prepared_unpublished',
    'basis_head_json': basisHead.canonicalJson,
    'head_json': candidateHead.canonicalJson,
    'project_json': candidateProjectJson,
    'revision': request.expectedRevision + 1,
    'line_id': request.lineId,
    'localization_id': revision3VoiceFixtureLocalizationId,
    'slot_id': request.slotId,
    'take_id': request.takeId,
    'locale': request.locale,
    'take_status': request.status.name,
    'slot_created': true,
    'selected': request.selectTake,
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
    'asset_deduplicated': false,
    'build_status': 'blocked',
    'runtime_status': 'runtime_unqualified',
    'target_authority': 'not_granted',
    'publication_status': 'not_supported',
  };
}

Map<String, Object?> _typedRef(String id, String kind) => <String, Object?>{
  'project_id': revision3VoiceFixtureProjectId,
  'id': id,
  'expected_kind': kind,
};

AuthoringWorkingHead _headFor(String projectJson) =>
    AuthoringWorkingHead.fromCanonicalJson(
      jsonEncode(<String, Object?>{
        'store_format': 1,
        'snapshot': <String, Object?>{
          'byte_len': utf8.encode(projectJson).length,
          'sha256': crypto.sha256.convert(utf8.encode(projectJson)).toString(),
        },
      }),
    );
