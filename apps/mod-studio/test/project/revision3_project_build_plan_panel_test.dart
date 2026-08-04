import 'dart:async';
import 'dart:convert';

import 'package:crypto/crypto.dart' as crypto;
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/core/mod_ffi.dart';
import 'package:gore_mod/project/revision3_project_build_plan_panel.dart';

import '../support/revision3_voice_fixture.dart';

const _otherProjectId = '11111111111111111111111111111111';

void main() {
  testWidgets('loads empty preview with eight fixed domains and no authority', (
    tester,
  ) async {
    final fixture = _emptyFixture();
    final completion = Completer<AuthoringRevision3ProjectBuildPlanResult>();
    var calls = 0;

    await _pumpPanel(
      tester,
      fixture: fixture,
      load: () {
        calls++;
        return calls == 1 ? completion.future : Future.value(fixture.result);
      },
    );

    expect(
      find.byKey(const Key('revision3-project-build-plan-loading')),
      findsOneWidget,
    );
    expect(find.text('Preview only'), findsOneWidget);
    expect(find.textContaining('No files were created'), findsOneWidget);

    completion.complete(fixture.result);
    await tester.pumpAndSettle();

    expect(find.text('No production content yet'), findsOneWidget);
    expect(find.text('No production records'), findsOneWidget);
    expect(find.text('Not present'), findsNWidgets(8));
    for (final domain in AuthoringRevision3ProjectBuildDomain.values) {
      expect(
        find.byKey(
          ValueKey('revision3-project-build-plan-domain-${domain.name}'),
        ),
        findsOneWidget,
      );
    }
    expect(find.textContaining(revision3VoiceFixtureProjectId), findsNothing);
    expect(
      find.textContaining(fixture.result.plan.inputSeal.sha256),
      findsNothing,
    );

    await tester.tap(
      find.byKey(const Key('revision3-project-build-plan-refresh')),
    );
    await tester.pumpAndSettle();
    expect(calls, 2);
  });

  testWidgets('groups author blockers separately from toolkit gaps', (
    tester,
  ) async {
    final fixture = _blockedFixture();
    await _pumpPanel(
      tester,
      fixture: fixture,
      load: () async => fixture.result,
    );
    await tester.pumpAndSettle();

    expect(find.text('Preparation required'), findsOneWidget);
    expect(find.text('2 production records'), findsOneWidget);
    expect(
      find.byKey(const Key('revision3-project-build-plan-author-blockers')),
      findsOneWidget,
    );
    expect(
      find.byKey(const Key('revision3-project-build-plan-toolkit-blockers')),
      findsOneWidget,
    );
    expect(find.text('A Voice target is unresolved.'), findsOneWidget);
    expect(
      find.text('Localization output is not implemented yet.'),
      findsOneWidget,
    );
    expect(
      find.text('0 ready \u00b7 1 blocked \u00b7 1 total'),
      findsNWidgets(2),
    );
    expect(find.textContaining(revision3VoiceFixtureLineId), findsNothing);
    expect(find.textContaining('GRD_263_ASGHAN'), findsNothing);
  });

  test('only exact per-line Voice reasons offer the drill-down', () {
    const expected = <AuthoringRevision3ProjectBuildBlockReason>{
      AuthoringRevision3ProjectBuildBlockReason.voiceTargetUnresolved,
      AuthoringRevision3ProjectBuildBlockReason.voiceTargetAmbiguous,
      AuthoringRevision3ProjectBuildBlockReason.voiceAddUnqualified,
      AuthoringRevision3ProjectBuildBlockReason.voiceSelectedTakeMissing,
      AuthoringRevision3ProjectBuildBlockReason.voiceSelectedTakeNotApproved,
      AuthoringRevision3ProjectBuildBlockReason
          .voiceSelectedTakeCodecUnqualified,
    };

    for (final reason in AuthoringRevision3ProjectBuildBlockReason.values) {
      expect(
        revision3ProjectBuildPlanHasExactVoiceDetails(reason),
        expected.contains(reason),
        reason: reason.name,
      );
    }
  });

  testWidgets('opens exact Voice details with local busy and failure states', (
    tester,
  ) async {
    final fixture = _blockedFixture();
    final completion = Completer<void>();
    var calls = 0;
    await _pumpPanel(
      tester,
      fixture: fixture,
      load: () async => fixture.result,
      openVoiceDetails: () {
        calls++;
        return completion.future;
      },
    );
    await tester.pumpAndSettle();

    final action = find.byKey(
      const ValueKey(
        'revision3-project-build-plan-voice-details-voiceTargetUnresolved',
      ),
    );
    expect(action, findsOneWidget);
    expect(
      find.byKey(
        const ValueKey(
          'revision3-project-build-plan-voice-details-localizationLoweringUnavailable',
        ),
      ),
      findsNothing,
    );

    await tester.ensureVisible(action);
    await tester.tap(action);
    await tester.pump();
    expect(calls, 1);
    expect(tester.widget<TextButton>(action).onPressed, isNull);
    expect(find.byType(CircularProgressIndicator), findsOneWidget);

    completion.complete();
    await tester.pumpAndSettle();
    expect(find.text('Show exact Voice problems'), findsOneWidget);
    expect(find.textContaining('Create playable'), findsNothing);
    expect(find.textContaining('Install'), findsNothing);

    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: SingleChildScrollView(
            child: Revision3ProjectBuildPlanPanel(
              checkpoint: fixture.checkpoint,
              load: () async => fixture.result,
              openVoiceDetails: () => throw StateError(r'C:\private\voice'),
            ),
          ),
        ),
      ),
    );
    await tester.pumpAndSettle();
    await tester.ensureVisible(action);
    await tester.tap(action);
    await tester.pumpAndSettle();
    expect(
      find.byKey(const Key('revision3-project-build-plan-action-error')),
      findsOneWidget,
    );
    expect(find.textContaining(r'C:\private'), findsNothing);
  });

  testWidgets('offers exact Voice details for the qualified toolkit gap', (
    tester,
  ) async {
    final fixture = _unqualifiedAddFixture();
    await _pumpPanel(
      tester,
      fixture: fixture,
      load: () async => fixture.result,
      openVoiceDetails: () {},
    );
    await tester.pumpAndSettle();

    expect(
      find.byKey(const Key('revision3-project-build-plan-toolkit-blockers')),
      findsOneWidget,
    );
    expect(
      find.byKey(
        const ValueKey(
          'revision3-project-build-plan-voice-details-voiceAddUnqualified',
        ),
      ),
      findsOneWidget,
    );
  });

  testWidgets('drops a pending Voice action after checkpoint replacement', (
    tester,
  ) async {
    final first = _blockedFixture();
    final second = _blockedFixture(
      projectId: _otherProjectId,
      projectRevision: 8,
    );
    final completion = Completer<void>();
    var current = first;
    late StateSetter updateHost;

    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: StatefulBuilder(
            builder: (context, setState) {
              updateHost = setState;
              final rendered = current;
              return SingleChildScrollView(
                child: Revision3ProjectBuildPlanPanel(
                  checkpoint: rendered.checkpoint,
                  load: () async => rendered.result,
                  openVoiceDetails: () => completion.future,
                ),
              );
            },
          ),
        ),
      ),
    );
    await tester.pumpAndSettle();
    final action = find.byKey(
      const ValueKey(
        'revision3-project-build-plan-voice-details-voiceTargetUnresolved',
      ),
    );
    await tester.ensureVisible(action);
    await tester.tap(action);
    await tester.pump();

    updateHost(() => current = second);
    await tester.pumpAndSettle();
    completion.complete();
    await tester.pumpAndSettle();

    expect(find.text('Exact project revision 8'), findsOneWidget);
    expect(
      find.byKey(const Key('revision3-project-build-plan-action-error')),
      findsNothing,
    );
    expect(tester.takeException(), isNull);
  });

  testWidgets('explains an unsupported Voice line label distinctly', (
    tester,
  ) async {
    final fixture = _lineLabelFixture();
    await _pumpPanel(
      tester,
      fixture: fixture,
      load: () async => fixture.result,
    );
    await tester.pumpAndSettle();

    expect(
      find.text('A dialog-line name is not supported for Voice output.'),
      findsOneWidget,
    );
    expect(
      find.text('The project name is not supported for Voice output.'),
      findsNothing,
    );
  });

  testWidgets('shows complete semantic coverage without implying a build', (
    tester,
  ) async {
    final fixture = _coverageFixture(slotCount: 2);
    await _pumpPanel(
      tester,
      fixture: fixture,
      load: () async => fixture.result,
    );
    await tester.pumpAndSettle();

    expect(find.text('Content coverage complete'), findsOneWidget);
    expect(find.text('2 production records'), findsOneWidget);
    expect(
      find.text('2 ready \u00b7 0 blocked \u00b7 2 total'),
      findsOneWidget,
    );
    expect(
      find.byKey(const Key('revision3-project-build-plan-author-blockers')),
      findsNothing,
    );
    expect(
      find.byKey(const Key('revision3-project-build-plan-toolkit-blockers')),
      findsNothing,
    );
    expect(
      find.textContaining('no build or install authority'),
      findsOneWidget,
    );
    expect(find.byType(FilledButton), findsNothing);
  });

  testWidgets('hides loader failures and retries safely', (tester) async {
    final fixture = _emptyFixture();
    var calls = 0;
    await _pumpPanel(
      tester,
      fixture: fixture,
      load: () async {
        calls++;
        if (calls == 1) {
          throw StateError(r'C:\private\mods\secret-project.json');
        }
        return fixture.result;
      },
    );
    await tester.pumpAndSettle();

    expect(
      find.byKey(const Key('revision3-project-build-plan-error')),
      findsOneWidget,
    );
    expect(find.text('Preview unavailable'), findsOneWidget);
    expect(find.textContaining(r'C:\private'), findsNothing);

    await tester.tap(
      find.byKey(const Key('revision3-project-build-plan-retry')),
    );
    await tester.pumpAndSettle();

    expect(calls, 2);
    expect(find.text('No production content yet'), findsOneWidget);
  });

  testWidgets('checkpoint changes reject stale async completion', (
    tester,
  ) async {
    final first = _coverageFixture(slotCount: 1);
    final second = _coverageFixture(
      slotCount: 2,
      projectId: _otherProjectId,
      revision: 8,
    );
    final firstCompletion =
        Completer<AuthoringRevision3ProjectBuildPlanResult>();
    final secondCompletion =
        Completer<AuthoringRevision3ProjectBuildPlanResult>();
    var calls = 0;
    var current = first;
    late StateSetter updateHost;

    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: StatefulBuilder(
            builder: (context, setState) {
              updateHost = setState;
              return SingleChildScrollView(
                child: Revision3ProjectBuildPlanPanel(
                  checkpoint: current.checkpoint,
                  load: () {
                    calls++;
                    return calls == 1
                        ? firstCompletion.future
                        : secondCompletion.future;
                  },
                ),
              );
            },
          ),
        ),
      ),
    );
    await tester.pump();

    updateHost(() => current = second);
    await tester.pump();
    expect(calls, 2);

    firstCompletion.complete(first.result);
    await tester.pump();
    expect(
      find.byKey(const Key('revision3-project-build-plan-loading')),
      findsOneWidget,
    );
    expect(find.text('Exact project revision 7'), findsNothing);

    secondCompletion.complete(second.result);
    await tester.pumpAndSettle();
    expect(find.text('Exact project revision 8'), findsOneWidget);
    expect(find.text('2 production records'), findsOneWidget);
  });

  testWidgets('rejects a result for another exact checkpoint', (tester) async {
    final fixture = _emptyFixture();
    final wrongCheckpoint = Revision3ProjectBuildPlanCheckpoint(
      projectId: _otherProjectId,
      projectRevision: 8,
      checkpointIdentity: fixture.head.canonicalJson,
    );
    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: Revision3ProjectBuildPlanPanel(
            checkpoint: wrongCheckpoint,
            load: () async => fixture.result,
          ),
        ),
      ),
    );
    await tester.pumpAndSettle();

    expect(
      find.byKey(const Key('revision3-project-build-plan-error')),
      findsOneWidget,
    );
    expect(find.text('No production content yet'), findsNothing);
  });

  testWidgets('compact German panel is overflow-safe at 200 percent text', (
    tester,
  ) async {
    tester.view.physicalSize = const Size(360, 900);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.resetPhysicalSize);
    addTearDown(tester.view.resetDevicePixelRatio);
    final fixture = _blockedFixture();

    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: MediaQuery(
            data: const MediaQueryData(textScaler: TextScaler.linear(2)),
            child: SingleChildScrollView(
              child: Revision3ProjectBuildPlanPanel(
                checkpoint: fixture.checkpoint,
                load: () async => fixture.result,
                copy: const Revision3ProjectBuildPlanCopy.german(),
              ),
            ),
          ),
        ),
      ),
    );
    await tester.pumpAndSettle();

    expect(tester.takeException(), isNull);
    expect(find.text('Nur Vorschau'), findsOneWidget);
    expect(find.text('Vorbereitung erforderlich'), findsOneWidget);
    expect(
      find.text('0 bereit \u00b7 1 blockiert \u00b7 1 gesamt'),
      findsNWidgets(2),
    );

    final technical = find.byKey(
      const Key('revision3-project-build-plan-technical'),
    );
    await tester.ensureVisible(technical);
    await tester.tap(technical);
    await tester.pumpAndSettle();
    expect(tester.takeException(), isNull);
    expect(
      find.textContaining(fixture.result.plan.inputSeal.sha256),
      findsOneWidget,
    );
  });
}

Future<void> _pumpPanel(
  WidgetTester tester, {
  required _BuildPlanFixture fixture,
  required Revision3ProjectBuildPlanLoader load,
  Revision3ProjectBuildPlanOpenVoiceDetails? openVoiceDetails,
}) => tester.pumpWidget(
  MaterialApp(
    home: Scaffold(
      body: SingleChildScrollView(
        child: Revision3ProjectBuildPlanPanel(
          checkpoint: fixture.checkpoint,
          load: load,
          openVoiceDetails: openVoiceDetails,
        ),
      ),
    ),
  ),
);

final class _BuildPlanFixture {
  const _BuildPlanFixture({
    required this.projectJson,
    required this.head,
    required this.result,
  });

  final String projectJson;
  final AuthoringWorkingHead head;
  final AuthoringRevision3ProjectBuildPlanResult result;

  Revision3ProjectBuildPlanCheckpoint get checkpoint {
    final project = (jsonDecode(projectJson) as Map).cast<String, Object?>();
    return Revision3ProjectBuildPlanCheckpoint(
      projectId: project['project_id']! as String,
      projectRevision: project['revision']! as int,
      checkpointIdentity: head.canonicalJson,
    );
  }
}

_BuildPlanFixture _emptyFixture() {
  final projectJson = revision3VoiceFixtureProjectJson();
  return _fixture(
    projectJson: projectJson,
    domainCounts: const <String, ({int ready, int blocked})>{},
    blockers: const <Map<String, Object?>>[],
  );
}

_BuildPlanFixture _coverageFixture({
  required int slotCount,
  String projectId = revision3VoiceFixtureProjectId,
  int revision = 7,
}) {
  final projectJson = revision3VoiceFixtureBuildReadyProjectJson(
    slotCount: slotCount,
    projectId: projectId,
    projectRevision: revision,
  );
  return _fixture(
    projectJson: projectJson,
    domainCounts: <String, ({int ready, int blocked})>{
      'voice': (ready: slotCount, blocked: 0),
    },
    blockers: const <Map<String, Object?>>[],
  );
}

_BuildPlanFixture _blockedFixture({
  String projectId = revision3VoiceFixtureProjectId,
  int projectRevision = 7,
}) {
  final project =
      (jsonDecode(
                revision3VoiceFixtureBuildReadyProjectJson(
                  projectId: projectId,
                  projectRevision: projectRevision,
                ),
              )
              as Map)
          .cast<String, Object?>();
  final entities = (project['entities']! as Map).cast<String, Object?>();
  final localization = (entities[revision3VoiceFixtureLocalizationId]! as Map)
      .cast<String, Object?>();
  localization['origin'] = <String, Object?>{
    'type': 'new',
    'authored_runtime_id': 'GRD_263_ASGHAN_OPEN_INFO_06_02',
  };
  entities[revision3VoiceFixtureLocalizationId] = localization;
  final slot =
      (entities.values.firstWhere((value) {
                final entity = (value as Map).cast<String, Object?>();
                final payload = (entity['payload']! as Map)
                    .cast<String, Object?>();
                return payload['kind'] == 'voice_slot';
              })
              as Map)
          .cast<String, Object?>();
  final slotPayload = (slot['payload']! as Map).cast<String, Object?>();
  final slotData = (slotPayload['data']! as Map).cast<String, Object?>();
  slotData['target_resolution'] = <String, Object?>{'state': 'unresolved'};
  slotPayload['data'] = slotData;
  slot['payload'] = slotPayload;
  entities[slot['id']! as String] = slot;
  project['entities'] = entities;
  final projectJson = jsonEncode(project);
  return _fixture(
    projectJson: projectJson,
    domainCounts: const <String, ({int ready, int blocked})>{
      'localization': (ready: 0, blocked: 1),
      'voice': (ready: 0, blocked: 1),
    },
    blockers: const <Map<String, Object?>>[
      <String, Object?>{
        'category': 'author_project',
        'domain': 'voice',
        'reason': 'voice_target_unresolved',
        'affected_count': 1,
      },
      <String, Object?>{
        'category': 'toolkit_support',
        'domain': 'localization',
        'reason': 'localization_lowering_unavailable',
        'affected_count': 1,
      },
    ],
  );
}

_BuildPlanFixture _lineLabelFixture() {
  final project =
      (jsonDecode(revision3VoiceFixtureBuildReadyProjectJson()) as Map)
          .cast<String, Object?>();
  final entities = (project['entities']! as Map).cast<String, Object?>();
  final line = (entities[revision3VoiceFixtureLineId]! as Map)
      .cast<String, Object?>();
  line['display_name'] = ' invalid line label';
  entities[revision3VoiceFixtureLineId] = line;
  project['entities'] = entities;
  return _fixture(
    projectJson: jsonEncode(project),
    domainCounts: const <String, ({int ready, int blocked})>{
      'voice': (ready: 0, blocked: 1),
    },
    blockers: const <Map<String, Object?>>[
      <String, Object?>{
        'category': 'author_project',
        'domain': 'voice',
        'reason': 'voice_line_label_unsupported',
        'affected_count': 1,
      },
    ],
  );
}

_BuildPlanFixture _unqualifiedAddFixture() {
  final project =
      (jsonDecode(revision3VoiceFixtureBuildReadyProjectJson()) as Map)
          .cast<String, Object?>();
  final entities = (project['entities']! as Map).cast<String, Object?>();
  final slot = entities.values
      .map((value) => (value! as Map).cast<String, Object?>())
      .singleWhere(
        (entity) =>
            ((entity['payload']! as Map<String, Object?>)['kind']) ==
            'voice_slot',
      );
  final payload = (slot['payload']! as Map).cast<String, Object?>();
  final data = (payload['data']! as Map).cast<String, Object?>();
  final resolution = (data['target_resolution']! as Map)
      .cast<String, Object?>();
  final target = (resolution['target']! as Map).cast<String, Object?>();
  target['operation'] = 'add';
  return _fixture(
    projectJson: jsonEncode(project),
    domainCounts: const <String, ({int ready, int blocked})>{
      'voice': (ready: 0, blocked: 1),
    },
    blockers: const <Map<String, Object?>>[
      <String, Object?>{
        'category': 'toolkit_support',
        'domain': 'voice',
        'reason': 'voice_add_unqualified',
        'affected_count': 1,
      },
    ],
  );
}

_BuildPlanFixture _fixture({
  required String projectJson,
  required Map<String, ({int ready, int blocked})> domainCounts,
  required List<Map<String, Object?>> blockers,
}) {
  final project = (jsonDecode(projectJson) as Map).cast<String, Object?>();
  final projectBytes = utf8.encode(projectJson);
  final projectSeal = _seal(projectBytes);
  final head = AuthoringWorkingHead.fromCanonicalJson(
    jsonEncode(<String, Object?>{'store_format': 1, 'snapshot': projectSeal}),
  );
  const domainNames = <String>[
    'localization',
    'dialog',
    'voice',
    'npc',
    'quest',
    'scripts',
    'items',
    'data_assets',
  ];
  final domains = <Object?>[
    for (final domain in domainNames)
      <String, Object?>{
        'domain': domain,
        'status': switch (domainCounts[domain]) {
          null => 'not_present',
          (ready: _, blocked: 0) => 'ready',
          _ => 'blocked',
        },
        'content_count': switch (domainCounts[domain]) {
          null => 0,
          (:final ready, :final blocked) => ready + blocked,
        },
        'ready_count': domainCounts[domain]?.ready ?? 0,
        'blocked_count': domainCounts[domain]?.blocked ?? 0,
      },
  ];
  final productionContentCount = domainCounts.entries
      .where((entry) => entry.key != 'scripts')
      .fold<int>(
        0,
        (total, entry) => total + entry.value.ready + entry.value.blocked,
      );
  final outcome = productionContentCount == 0
      ? 'empty'
      : blockers.isEmpty
      ? 'coverage_complete'
      : 'blocked';
  final inputProjection = <String, Object?>{
    'format': 'gore.authoring.revision3-project-build-input.v1',
    'project': projectSeal,
    'dataasset_stage_manifests': <Object?>[],
  };
  final inputSeal = _seal(utf8.encode(jsonEncode(inputProjection)));
  final planProjection = <String, Object?>{
    'format': 'gore.authoring.revision3-project-build-plan.v1',
    'schema_revision': 1,
    'project_id': project['project_id'],
    'project_revision': project['revision'],
    'outcome': outcome,
    'production_content_count': productionContentCount,
    'input_seal': inputSeal,
    'domains': domains,
    'blockers': blockers,
    'scope': 'project_build_readiness_only',
    'build_authority': 'not_granted',
    'artifact_status': 'not_created',
    'deployment_status': 'not_performed',
    'runtime_status': 'runtime_unqualified',
    'publication_status': 'not_supported',
  };
  final planSeal = _seal(utf8.encode(jsonEncode(planProjection)));
  final response = <String, Object?>{
    'ok': true,
    'basis_head_json': head.canonicalJson,
    'plan': <String, Object?>{
      'schema_revision': 1,
      'project_id': project['project_id'],
      'project_revision': project['revision'],
      'outcome': outcome,
      'production_content_count': productionContentCount,
      'input_seal': inputSeal,
      'plan_seal': planSeal,
      'domains': domains,
      'blockers': blockers,
      'scope': 'project_build_readiness_only',
      'build_authority': 'not_granted',
      'artifact_status': 'not_created',
      'deployment_status': 'not_performed',
      'runtime_status': 'runtime_unqualified',
      'publication_status': 'not_supported',
    },
  };
  return _BuildPlanFixture(
    projectJson: projectJson,
    head: head,
    result: AuthoringRevision3ProjectBuildPlanResult.fromJson(
      response,
      expectedHead: head,
      expectedProjectJson: projectJson,
    ),
  );
}

Map<String, Object?> _seal(List<int> bytes) => <String, Object?>{
  'byte_len': bytes.length,
  'sha256': crypto.sha256.convert(bytes).toString(),
};
