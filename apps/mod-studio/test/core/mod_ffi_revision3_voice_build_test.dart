import 'dart:convert';

import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/core/core_service.dart';
import 'package:gore_mod/core/mod_ffi.dart';

import '../support/revision3_voice_fixture.dart';

String _headJson(String byte) => jsonEncode(<String, Object?>{
  'store_format': 1,
  'snapshot': <String, Object?>{
    'byte_len': 321,
    'sha256': List<String>.filled(64, byte).join(),
  },
});

Map<String, Object?> _slotBlocker({
  String slotId = '00000000000000000000000000100000',
  String lineId = revision3VoiceFixtureLineId,
  String lineLabel = 'Asghan greeting',
  String locId = 'GRD_263_ASGHAN_OPEN_INFO_06_02',
  String locale = 'de',
  String reason = 'unresolved_target',
}) => <String, Object?>{
  'slot_id': slotId,
  'line_id': lineId,
  'line_label': lineLabel,
  'loc_id': locId,
  'locale': locale,
  'reason': reason,
};

Map<String, Object?> _blockedResponseWithReport({
  required int totalSlots,
  required int readySlots,
  required List<Object?> blockers,
}) {
  final response = _blockedResponse();
  response['report'] = <String, Object?>{
    'project_id': revision3VoiceFixtureProjectId,
    'project_revision': 7,
    'total_slots': totalSlots,
    'ready_slots': readySlots,
    'blockers': blockers,
  };
  return response;
}

Map<String, Object?> _blockedResponse() => <String, Object?>{
  'ok': true,
  'outcome': 'blocked',
  'basis_head_json': _headJson('b'),
  'project_id': revision3VoiceFixtureProjectId,
  'project_revision': 7,
  'report': <String, Object?>{
    'project_id': revision3VoiceFixtureProjectId,
    'project_revision': 7,
    'total_slots': 0,
    'ready_slots': 0,
    'blockers': <Object?>[
      <String, Object?>{'reason': 'no_voice_slots'},
    ],
  },
  'build_authority': 'not_granted',
  'deployment_status': 'not_performed',
};

Map<String, Object?> _builtResponse(String output, {int editCount = 1}) =>
    <String, Object?>{
      'ok': true,
      'outcome': 'built',
      'basis_head_json': _headJson('b'),
      'project_id': revision3VoiceFixtureProjectId,
      'project_revision': 7,
      'output': output,
      'edit_count': editCount,
      'file_count': editCount + 2,
      'bundle_bytes': 12345,
      'bundle_sha256': List<String>.filled(64, 'c').join(),
      'build_authority': 'generation_sealed_existing_member_bundle_v1',
      'deployment_status': 'not_performed',
    };

Map<String, Object?> _planResponse({
  required String outcome,
  required int totalSlots,
  required int readySlots,
  required List<Object?> blockers,
}) => <String, Object?>{
  'ok': true,
  'outcome': outcome,
  'basis_head_json': _headJson('b'),
  'project_id': revision3VoiceFixtureProjectId,
  'project_revision': 7,
  'total_slots': totalSlots,
  'ready_slots': readySlots,
  'blockers': blockers,
  'plan_authority': 'read_only_voice_build_plan_v1',
  'build_authority': 'not_granted',
  'deployment_status': 'not_performed',
};

List<Object?> _unreadySlotBlockers(int slotCount) => <Object?>[
  for (var index = 0; index < slotCount; index++) ...<Object?>[
    _slotBlocker(
      slotId: (0x100000 + index).toRadixString(16).padLeft(32, '0'),
      locale: index == 0 ? 'de' : 'de-x$index',
    ),
    _slotBlocker(
      slotId: (0x100000 + index).toRadixString(16).padLeft(32, '0'),
      locale: index == 0 ? 'de' : 'de-x$index',
      reason: 'missing_selected_take',
    ),
  ],
];

String _projectWithVoiceLineLabel(String lineLabel, {int slotCount = 1}) {
  final project =
      (jsonDecode(revision3VoiceFixtureProjectWithVoiceSlotCountJson(slotCount))
              as Map)
          .cast<String, Object?>();
  final entities = (project['entities']! as Map).cast<String, Object?>();
  final line = (entities[revision3VoiceFixtureLineId]! as Map)
      .cast<String, Object?>();
  line['display_name'] = lineLabel;
  return jsonEncode(project);
}

List<Object?> _unreadyBlockersWithLineLabel(String lineLabel) => <Object?>[
  _slotBlocker(lineLabel: lineLabel),
  _slotBlocker(lineLabel: lineLabel, reason: 'missing_selected_take'),
];

void main() {
  final head = AuthoringWorkingHead.fromCanonicalJson(_headJson('b'));

  test(
    'required command handshake includes managed R3 Voice plan and build',
    () {
      expect(
        requiredStudioCoreCommands,
        contains('authoring_store_build_revision3_voice_v1'),
      );
      expect(
        requiredStudioCoreCommands,
        contains('authoring_store_plan_revision3_voice_v1'),
      );
      expect(
        requiredStudioCoreCommands,
        orderedEquals(<String>[...requiredStudioCoreCommands]..sort()),
      );
    },
  );

  test(
    'read-only plan parser accepts only the exact local ready projection',
    () {
      final projectJson = revision3VoiceFixtureBuildReadyProjectJson();
      final result = AuthoringRevision3VoiceBuildPlanResult.fromJson(
        _planResponse(
          outcome: 'ready',
          totalSlots: 1,
          readySlots: 1,
          blockers: const <Object?>[],
        ),
        expectedHead: head,
        expectedProjectJson: projectJson,
      );

      expect(result.outcome, AuthoringRevision3VoiceBuildPlanOutcome.ready);
      expect(result.isReady, isTrue);
      expect(result.basisHead.canonicalJson, head.canonicalJson);
      expect(result.projectId, revision3VoiceFixtureProjectId);
      expect(result.projectRevision, 7);
      expect(result.totalSlots, 1);
      expect(result.readySlots, 1);
      expect(result.blockers, isEmpty);
    },
  );

  test('read-only plan parser retains exact structured blockers', () {
    final projectJson = revision3VoiceFixtureProjectWithVoiceSlotCountJson(1);
    final blockers = _unreadySlotBlockers(1).reversed.toList(growable: false);
    final result = AuthoringRevision3VoiceBuildPlanResult.fromJson(
      _planResponse(
        outcome: 'blocked',
        totalSlots: 1,
        readySlots: 0,
        blockers: blockers,
      ),
      expectedHead: head,
      expectedProjectJson: projectJson,
    );

    expect(result.outcome, AuthoringRevision3VoiceBuildPlanOutcome.blocked);
    expect(result.isReady, isFalse);
    expect(result.totalSlots, 1);
    expect(result.readySlots, 0);
    expect(result.blockers, hasLength(2));
    expect(
      result.blockers.map((blocker) => blocker.reason),
      containsAll(<AuthoringRevision3VoiceBuildBlockReason>[
        AuthoringRevision3VoiceBuildBlockReason.unresolvedTarget,
        AuthoringRevision3VoiceBuildBlockReason.missingSelectedTake,
      ]),
    );
  });

  test('read-only plan applies the slot cap before projecting line labels', () {
    for (final lineLabel in <String>['Asghan greeting', ' Asghan greeting']) {
      final projectJson = _projectWithVoiceLineLabel(
        lineLabel,
        slotCount: 1025,
      );
      final result = AuthoringRevision3VoiceBuildPlanResult.fromJson(
        _planResponse(
          outcome: 'blocked',
          totalSlots: 1025,
          readySlots: 0,
          blockers: <Object?>[
            <String, Object?>{'reason': 'voice_slot_limit_exceeded'},
          ],
        ),
        expectedHead: head,
        expectedProjectJson: projectJson,
      );

      expect(result.totalSlots, 1025);
      expect(result.readySlots, 0);
      expect(result.blockers, hasLength(1));
      expect(
        result.blockers.single.reason,
        AuthoringRevision3VoiceBuildBlockReason.voiceSlotLimitExceeded,
      );
      expect(result.blockers.single.isGlobal, isTrue);
    }
  });

  test(
    'line-label contract matches native controls, boundaries, and UTF-8 cap',
    () {
      final accepted = <String>[
        List<String>.filled(128, 'é').join(),
        'Asghan\u00a0greeting',
      ];
      for (final lineLabel in accepted) {
        final result = AuthoringRevision3VoiceBuildPlanResult.fromJson(
          _planResponse(
            outcome: 'blocked',
            totalSlots: 1,
            readySlots: 0,
            blockers: _unreadyBlockersWithLineLabel(lineLabel),
          ),
          expectedHead: head,
          expectedProjectJson: _projectWithVoiceLineLabel(lineLabel),
        );
        expect(result.blockers.first.lineLabel, lineLabel);
      }

      final exactBoundary = List<String>.filled(128, 'é').join();
      final rejected = <String>[
        '',
        ' Asghan greeting',
        'Asghan greeting\u00a0',
        '\ufeffAsghan greeting',
        'Asghan\u009fgreeting',
        '${exactBoundary}x',
      ];
      for (final lineLabel in rejected) {
        expect(
          () => AuthoringRevision3VoiceBuildPlanResult.fromJson(
            _planResponse(
              outcome: 'blocked',
              totalSlots: 1,
              readySlots: 0,
              blockers: _unreadyBlockersWithLineLabel(lineLabel),
            ),
            expectedHead: head,
            expectedProjectJson: _projectWithVoiceLineLabel(lineLabel),
          ),
          throwsFormatException,
          reason: 'must reject ${jsonEncode(lineLabel)}',
        );
      }
    },
  );

  test('read-only plan rejects forged basis, readiness, or authority', () {
    final projectJson = revision3VoiceFixtureBuildReadyProjectJson();
    Map<String, Object?> ready() => _planResponse(
      outcome: 'ready',
      totalSlots: 1,
      readySlots: 1,
      blockers: const <Object?>[],
    );
    final mutations = <Map<String, Object?> Function()>[
      () => ready()..['outcome'] = 'blocked',
      () => ready()..['basis_head_json'] = _headJson('c'),
      () => ready()..['project_id'] = 'f' * 32,
      () => ready()..['project_revision'] = 8,
      () => ready()..['total_slots'] = 2,
      () => ready()..['ready_slots'] = 0,
      () => ready()
        ..['blockers'] = <Object?>[
          <String, Object?>{'reason': 'no_voice_slots'},
        ],
      () => ready()..['plan_authority'] = 'build',
      () => ready()..['build_authority'] = 'granted',
      () => ready()..['deployment_status'] = 'performed',
      () => ready()..['output'] = r'C:\MustNotExist',
    ];

    for (final mutate in mutations) {
      expect(
        () => AuthoringRevision3VoiceBuildPlanResult.fromJson(
          mutate(),
          expectedHead: head,
          expectedProjectJson: projectJson,
        ),
        throwsFormatException,
      );
    }
  });

  test('read-only wrapper sends no game or output path', () async {
    final projectJson = revision3VoiceFixtureBuildReadyProjectJson();
    final core = FakeGoreCoreFfiService(
      responses: <String, Map<String, Object?>>{
        'authoring_store_plan_revision3_voice_v1': _planResponse(
          outcome: 'ready',
          totalSlots: 1,
          readySlots: 1,
          blockers: const <Object?>[],
        ),
      },
    );

    final result = await ModFfi(core).authoringStorePlanRevision3VoiceV1(
      root: r'C:\Projects\Voice.goreproj',
      currentProjectJson: projectJson,
      expectedHead: head,
    );

    expect(result.isReady, isTrue);
    expect(
      core.calls.single.command,
      'authoring_store_plan_revision3_voice_v1',
    );
    expect(core.calls.single.payload, <String, Object?>{
      'current_project_json': projectJson,
      'expected_head_json': head.canonicalJson,
      'root': r'C:\Projects\Voice.goreproj',
    });
    expect(core.calls.single.payload, isNot(contains('game_root')));
    expect(core.calls.single.payload, isNot(contains('output')));
  });

  test('strict parser retains structured all-or-nothing blockers', () {
    final result = AuthoringRevision3VoiceBuildResult.fromJson(
      _blockedResponse(),
      expectedHead: head,
      expectedProjectJson: revision3VoiceFixtureProjectJson(),
      expectedOutput: r'C:\Builds\Voice',
    );
    expect(result.outcome, AuthoringRevision3VoiceBuildOutcome.blocked);
    expect(result.isBuilt, isFalse);
    expect(result.report!.totalSlots, 0);
    expect(
      result.report!.blockers.single.reason,
      AuthoringRevision3VoiceBuildBlockReason.noVoiceSlots,
    );
    expect(result.output, isNull);
  });

  test('blocked response is bound to the exact readiness blocker multiset', () {
    final projectJson = revision3VoiceFixtureProjectWithVoiceSlotCountJson(1);
    final reversedBlockers = _unreadySlotBlockers(1).reversed.toList();
    final result = AuthoringRevision3VoiceBuildResult.fromJson(
      _blockedResponseWithReport(
        totalSlots: 1,
        readySlots: 0,
        blockers: reversedBlockers,
      ),
      expectedHead: head,
      expectedProjectJson: projectJson,
      expectedOutput: r'C:\Builds\Voice',
    );
    expect(
      result.report!.blockers.map((blocker) => blocker.reason),
      containsAll(<AuthoringRevision3VoiceBuildBlockReason>{
        AuthoringRevision3VoiceBuildBlockReason.unresolvedTarget,
        AuthoringRevision3VoiceBuildBlockReason.missingSelectedTake,
      }),
    );

    for (final forged in <List<Object?>>[
      <Object?>[_unreadySlotBlockers(1).first],
      <Object?>[
        _slotBlocker(reason: 'ambiguous_target'),
        _slotBlocker(reason: 'missing_selected_take'),
      ],
    ]) {
      expect(
        () => AuthoringRevision3VoiceBuildResult.fromJson(
          _blockedResponseWithReport(
            totalSlots: 1,
            readySlots: 0,
            blockers: forged,
          ),
          expectedHead: head,
          expectedProjectJson: projectJson,
          expectedOutput: r'C:\Builds\Voice',
        ),
        throwsFormatException,
      );
    }
  });

  test(
    'build report accepts the exact 1024-slot cap and payload budget blocker',
    () {
      final atCap = AuthoringRevision3VoiceBuildResult.fromJson(
        _blockedResponseWithReport(
          totalSlots: 1024,
          readySlots: 0,
          blockers: _unreadySlotBlockers(1024),
        ),
        expectedHead: head,
        expectedProjectJson: revision3VoiceFixtureProjectWithVoiceSlotCountJson(
          1024,
        ),
        expectedOutput: r'C:\Builds\Voice',
      );
      expect(atCap.report!.totalSlots, 1024);
      expect(atCap.report!.blockers, hasLength(2048));
      expect(atCap.report!.blockers.first.lineLabel, 'Asghan greeting');
      expect(atCap.report!.blockers.first.locId, _slotBlocker()['loc_id']);

      final payloadBudget = AuthoringRevision3VoiceBuildResult.fromJson(
        _blockedResponseWithReport(
          totalSlots: 1,
          readySlots: 0,
          blockers: <Object?>[
            <String, Object?>{'reason': 'voice_payload_budget_exceeded'},
          ],
        ),
        expectedHead: head,
        expectedProjectJson: revision3VoiceFixtureBuildReadyProjectJson(
          assetBytes: 256 * 1024 * 1024 + 1,
        ),
        expectedOutput: r'C:\Builds\Voice',
      );
      expect(
        payloadBudget.report!.blockers.single.reason,
        AuthoringRevision3VoiceBuildBlockReason.voicePayloadBudgetExceeded,
      );

      for (final total in <int>[1025]) {
        final overCap = AuthoringRevision3VoiceBuildResult.fromJson(
          _blockedResponseWithReport(
            totalSlots: total,
            readySlots: 0,
            blockers: <Object?>[
              <String, Object?>{'reason': 'voice_slot_limit_exceeded'},
            ],
          ),
          expectedHead: head,
          expectedProjectJson:
              revision3VoiceFixtureProjectWithVoiceSlotCountJson(total),
          expectedOutput: r'C:\Builds\Voice',
        );
        expect(overCap.report!.totalSlots, total);
        expect(
          overCap.report!.blockers.single.reason,
          AuthoringRevision3VoiceBuildBlockReason.voiceSlotLimitExceeded,
        );
      }
    },
  );

  test('selected payload budget counts every reused asset occurrence', () {
    const perSlotLimitBytes = 128 * 1024 * 1024;
    final exactlyAtLimit = revision3VoiceFixtureBuildReadyProjectJson(
      slotCount: 2,
      assetBytes: perSlotLimitBytes,
    );
    final built = AuthoringRevision3VoiceBuildResult.fromJson(
      _builtResponse(r'C:\Builds\Voice', editCount: 2),
      expectedHead: head,
      expectedProjectJson: exactlyAtLimit,
      expectedOutput: r'C:\Builds\Voice',
    );
    expect(built.isBuilt, isTrue);

    final oneOccurrenceByteOver = revision3VoiceFixtureBuildReadyProjectJson(
      slotCount: 2,
      assetBytes: perSlotLimitBytes + 1,
    );
    final blocked = AuthoringRevision3VoiceBuildResult.fromJson(
      _blockedResponseWithReport(
        totalSlots: 2,
        readySlots: 0,
        blockers: <Object?>[
          <String, Object?>{'reason': 'voice_payload_budget_exceeded'},
        ],
      ),
      expectedHead: head,
      expectedProjectJson: oneOccurrenceByteOver,
      expectedOutput: r'C:\Builds\Voice',
    );
    expect(blocked.report!.readySlots, 0);
    expect(
      blocked.report!.blockers.single.reason,
      AuthoringRevision3VoiceBuildBlockReason.voicePayloadBudgetExceeded,
    );
  });

  test('deployment identity uses the closed-model Unicode case folding', () {
    final projectJson = revision3VoiceFixtureBuildReadyProjectJson(
      slotCount: 2,
      archives: const <String>['Ä.zip', 'ä.zip'],
      sharedMember: true,
    );
    expect(
      () => AuthoringRevision3VoiceBuildResult.fromJson(
        _builtResponse(r'C:\Builds\Voice', editCount: 2),
        expectedHead: head,
        expectedProjectJson: projectJson,
        expectedOutput: r'C:\Builds\Voice',
      ),
      throwsFormatException,
    );
  });

  test(
    'build report rejects malformed scope, facts, duplicates, and counts',
    () {
      final otherSlot = '33333333333333333333333333333333';
      final otherLine = '44444444444444444444444444444444';
      final malformed = <({Map<String, Object?> response, String project})>[
        (
          response: _blockedResponseWithReport(
            totalSlots: 0,
            readySlots: 0,
            blockers: <Object?>[
              <String, Object?>{
                'reason': 'no_voice_slots',
                'slot_id': otherSlot,
              },
            ],
          ),
          project: revision3VoiceFixtureProjectJson(),
        ),
        (
          response: _blockedResponseWithReport(
            totalSlots: 1,
            readySlots: 0,
            blockers: <Object?>[
              <String, Object?>{..._slotBlocker()}..remove('line_label'),
            ],
          ),
          project: revision3VoiceFixtureProjectWithVoiceSlotCountJson(1),
        ),
        (
          response: _blockedResponseWithReport(
            totalSlots: 1,
            readySlots: 0,
            blockers: <Object?>[_slotBlocker(locId: 'CON')],
          ),
          project: revision3VoiceFixtureProjectWithVoiceSlotCountJson(1),
        ),
        (
          response: _blockedResponseWithReport(
            totalSlots: 1,
            readySlots: 0,
            blockers: <Object?>[_slotBlocker(), _slotBlocker()],
          ),
          project: revision3VoiceFixtureProjectWithVoiceSlotCountJson(1),
        ),
        (
          response: _blockedResponseWithReport(
            totalSlots: 1,
            readySlots: 0,
            blockers: <Object?>[
              _slotBlocker(),
              _slotBlocker(
                reason: 'missing_selected_take',
                lineLabel: 'Forged label',
              ),
            ],
          ),
          project: revision3VoiceFixtureProjectWithVoiceSlotCountJson(1),
        ),
        (
          response: _blockedResponseWithReport(
            totalSlots: 2,
            readySlots: 0,
            blockers: <Object?>[
              _slotBlocker(),
              _slotBlocker(slotId: otherSlot, reason: 'missing_selected_take'),
            ],
          ),
          project: revision3VoiceFixtureProjectWithVoiceSlotCountJson(2),
        ),
        (
          response: _blockedResponseWithReport(
            totalSlots: 2,
            readySlots: 0,
            blockers: <Object?>[_slotBlocker()],
          ),
          project: revision3VoiceFixtureProjectWithVoiceSlotCountJson(2),
        ),
        (
          response: _blockedResponseWithReport(
            totalSlots: 1025,
            readySlots: 0,
            blockers: <Object?>[_slotBlocker()],
          ),
          project: revision3VoiceFixtureProjectWithVoiceSlotCountJson(1025),
        ),
        (
          response: _blockedResponseWithReport(
            totalSlots: 2,
            readySlots: 0,
            blockers: <Object?>[
              _slotBlocker(),
              _slotBlocker(
                slotId: otherSlot,
                lineId: otherLine,
                lineLabel: 'Other line',
                locId: 'OTHER_LINE',
                locale: 'en',
              ),
              <String, Object?>{'reason': 'voice_slot_limit_exceeded'},
            ],
          ),
          project: revision3VoiceFixtureProjectWithVoiceSlotCountJson(2),
        ),
        (
          response: _blockedResponseWithReport(
            totalSlots: 1,
            readySlots: 0,
            blockers: <Object?>[
              _slotBlocker(slotId: revision3VoiceFixtureLineId),
            ],
          ),
          project: revision3VoiceFixtureProjectWithVoiceSlotCountJson(1),
        ),
      ];
      for (final malformedCase in malformed) {
        expect(
          () => AuthoringRevision3VoiceBuildResult.fromJson(
            malformedCase.response,
            expectedHead: head,
            expectedProjectJson: malformedCase.project,
            expectedOutput: r'C:\Builds\Voice',
          ),
          throwsFormatException,
        );
      }
    },
  );

  test(
    'wrapper binds exact project/head/output and accepts sealed receipt',
    () async {
      const output = r'C:\Builds\Voice';
      final projectJson = revision3VoiceFixtureBuildReadyProjectJson();
      final core = FakeGoreCoreFfiService(
        responses: <String, Map<String, Object?>>{
          'authoring_store_build_revision3_voice_v1': _builtResponse(output),
        },
      );
      final result = await ModFfi(core).authoringStoreBuildRevision3VoiceV1(
        root: r'C:\Projects\Voice.goreproj',
        gameRoot: r'C:\Games\Gothic Remake',
        currentProjectJson: projectJson,
        expectedHead: head,
        output: output,
      );
      expect(result.isBuilt, isTrue);
      expect(result.editCount, 1);
      expect(result.fileCount, 3);
      expect(result.bundleSha256, List<String>.filled(64, 'c').join());
      expect(core.calls.single.payload, <String, Object?>{
        'current_project_json': projectJson,
        'expected_head_json': head.canonicalJson,
        'game_root': r'C:\Games\Gothic Remake',
        'output': output,
        'root': r'C:\Projects\Voice.goreproj',
      });
    },
  );

  test('response cannot forge output, authority, counts, or blockers', () {
    const output = r'C:\Builds\Voice';
    final projectJson = revision3VoiceFixtureBuildReadyProjectJson();
    expect(
      () => AuthoringRevision3VoiceBuildResult.fromJson(
        _builtResponse(output),
        expectedHead: head,
        expectedProjectJson: revision3VoiceFixtureProjectWithVoiceSlotCountJson(
          1,
        ),
        expectedOutput: output,
      ),
      throwsFormatException,
      reason: 'an unresolved slot without a selected take cannot be built',
    );
    expect(
      () => AuthoringRevision3VoiceBuildResult.fromJson(
        _builtResponse(output, editCount: 2),
        expectedHead: head,
        expectedProjectJson: revision3VoiceFixtureProjectJson(),
        expectedOutput: output,
      ),
      throwsFormatException,
      reason: 'a zero-slot project cannot accept a forged two-edit receipt',
    );
    final mutations = <Map<String, Object?> Function()>[
      () => _builtResponse(output)..['output'] = r'C:\Elsewhere',
      () => _builtResponse(output)..['build_authority'] = 'deploy',
      () => _builtResponse(output)..['file_count'] = 5,
      () => _builtResponse(output, editCount: 2),
      () => _builtResponse(output)..['bundle_sha256'] = 'BAD',
      () => _blockedResponse()
        ..['report'] = <String, Object?>{
          ...(_blockedResponse()['report']! as Map).cast<String, Object?>(),
          'blockers': <Object?>[
            <String, Object?>{
              'slot_id': List<String>.filled(32, 'd').join(),
              'reason': 'no_voice_slots',
            },
          ],
        },
    ];
    for (final mutate in mutations) {
      expect(
        () => AuthoringRevision3VoiceBuildResult.fromJson(
          mutate(),
          expectedHead: head,
          expectedProjectJson: projectJson,
          expectedOutput: output,
        ),
        throwsFormatException,
      );
    }
  });
}
