import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/core/mod_ffi.dart';
import 'package:gore_mod/project/revision3_content_index.dart';
import 'package:gore_mod/project/revision3_quest_transcript_authoring.dart';
import 'package:gore_mod/project/revision3_quest_transcript_panel.dart';

import '../support/revision3_quest_outline_fixture.dart';

const _projectId = 'ffffffffffffffffffffffffffffffff';
const _localizationId = '11111111111111111111111111111111';
const _lineId = '22222222222222222222222222222222';
const _slotId = '33333333333333333333333333333333';
const _takeId = '44444444444444444444444444444444';
const _questId = '55555555555555555555555555555555';
const _moduleId = '66666666666666666666666666666666';
const _collisionSha =
    'eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee';
const _collisionMediaType =
    'application/vnd.gore.quest-collision-capability+json;version=2';
const _secondLocalizationId = '12121212121212121212121212121212';
const _secondLineId = '23232323232323232323232323232323';
const _targetSha =
    'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa';

void main() {
  testWidgets(
    'shows friendly ordered coverage, lazy German preview and exact handoff without technical identity',
    (tester) async {
      await _setSurfaceSize(tester, const Size(1200, 800));
      final head = manifestHead(4096, 'b');
      final index = Revision3ContentIndex.fromJsonObject(_contentIndexJson());
      var reads = 0;
      Revision3QuestTranscriptProjection? openedProjection;
      Revision3QuestTranscriptRow? openedRow;
      String? openedLocale;
      final service = _service(
        index: index,
        head: head,
        read:
            ({
              required expectedProjectId,
              required expectedProjectRevision,
              required expectedHead,
              required localizationId,
              required expectedLocalizationRevision,
              required expectedLocId,
            }) async {
              reads++;
              return _readResult(
                head: expectedHead,
                projectId: expectedProjectId,
                projectRevision: expectedProjectRevision,
                localizationId: localizationId,
                localizationRevision: expectedLocalizationRevision,
                locId: expectedLocId,
                locale: 'de',
                text: 'Das Tor ist gesichert.',
              );
            },
      );

      await _pumpPanel(
        tester,
        head: head,
        service: service,
        copy: Revision3QuestTranscriptPanelCopy.german,
        onOpenTextVoice:
            ({required projection, required row, required locale}) async {
              openedProjection = projection;
              openedRow = row;
              openedLocale = locale;
              return true;
            },
      );
      await tester.pumpAndSettle();

      expect(
        find.byKey(const Key('revision3-quest-transcript-wide')),
        findsOneWidget,
      );
      expect(find.text('Inspect the gate'), findsOneWidget);
      expect(find.text('Dialog line'), findsWidgets);
      expect(find.text('Sprachen: de'), findsOneWidget);
      expect(find.text('Text 1/1'), findsOneWidget);
      expect(find.text('Voice 1/1 \u00b7 1 Aufnahmen'), findsOneWidget);
      expect(find.text('Das Tor ist gesichert.'), findsOneWidget);
      expect(find.textContaining('pr\u00c3\u00bcfen'), findsNothing);
      expect(find.textContaining('pr\u00c3'), findsNothing);
      expect(find.textContaining(_lineId), findsNothing);
      expect(find.textContaining(_localizationId), findsNothing);
      expect(find.textContaining('DIA_GATE_WARNING'), findsNothing);
      expect(find.textContaining('PROJECT/QUESTS'), findsNothing);
      expect(reads, 1, reason: 'only the selected row is read lazily');

      final open = find.byKey(
        const Key('revision3-quest-transcript-open-text-voice'),
      );
      await tester.ensureVisible(open);
      await tester.tap(open);
      await tester.pumpAndSettle();

      expect(openedProjection?.checkpointIdentity, head.canonicalJson);
      expect(openedRow?.lineId, _lineId);
      expect(openedLocale, 'de');
      expect(find.text('Text & Voice \u00f6ffnen'), findsOneWidget);
      expect(tester.takeException(), isNull);
    },
  );

  testWidgets(
    'compact layout tolerates long German copy and service churn while mutation gate stays fail closed',
    (tester) async {
      await _setSurfaceSize(tester, const Size(360, 640));
      final head = manifestHead(4096, 'b');
      final index = Revision3ContentIndex.fromJsonObject(_contentIndexJson());
      var loads = 0;
      var openCalls = 0;
      var mutationsEnabled = true;
      late StateSetter rebuild;

      await tester.pumpWidget(
        MaterialApp(
          home: StatefulBuilder(
            builder: (context, setState) {
              rebuild = setState;
              final service = _service(
                index: index,
                head: head,
                load: () async {
                  loads++;
                  return index;
                },
              );
              return Scaffold(
                body: SingleChildScrollView(
                  padding: const EdgeInsets.all(8),
                  child: Revision3QuestTranscriptPanel(
                    projectId: _projectId,
                    projectRevision: 7,
                    projectCheckpointIdentity: head.canonicalJson,
                    questId: _questId,
                    questRevision: 4,
                    service: service,
                    selectedLineId: null,
                    onSelectedLineChanged: (_) {},
                    onCreateLine:
                        ({
                          required projection,
                          required insertionIndex,
                          required objectiveSlot,
                          required publishTechnicalPlan,
                        }) async => false,
                    onOpenTextVoice:
                        ({
                          required projection,
                          required row,
                          required locale,
                        }) async {
                          openCalls++;
                          return true;
                        },
                    mutationsEnabled: mutationsEnabled,
                    mutationDisabledReason: mutationsEnabled
                        ? null
                        : 'Speichere oder verwirf die au\u00dfergew\u00f6hnlich umfangreichen offenen Text\u00e4nderungen, bevor du dieses Quest-Transkript bearbeitest.',
                    copy: Revision3QuestTranscriptPanelCopy.german,
                  ),
                ),
              );
            },
          ),
        ),
      );
      await tester.pumpAndSettle();
      expect(loads, 1);
      expect(
        find.byKey(const Key('revision3-quest-transcript-compact')),
        findsOneWidget,
      );

      rebuild(() => mutationsEnabled = false);
      await tester.pumpAndSettle();

      expect(
        loads,
        1,
        reason: 'a declaratively rebuilt service is not a reload key',
      );
      expect(
        tester
            .widget<FilledButton>(
              find.byKey(const Key('revision3-quest-transcript-new-line')),
            )
            .onPressed,
        isNull,
      );
      expect(
        tester
            .widget<OutlinedButton>(
              find.byKey(const Key('revision3-quest-transcript-edit')),
            )
            .onPressed,
        isNull,
      );
      expect(
        tester
            .widget<FilledButton>(
              find.byKey(
                const Key('revision3-quest-transcript-open-text-voice'),
              ),
            )
            .onPressed,
        isNotNull,
        reason: 'read-only exact handoff remains available',
      );
      final compactOpen = find.byKey(
        const Key('revision3-quest-transcript-open-text-voice'),
      );
      await tester.ensureVisible(compactOpen);
      await tester.tap(compactOpen);
      await tester.pumpAndSettle();
      expect(openCalls, 1);
      expect(find.textContaining('au\u00dfergew\u00f6hnlich'), findsOneWidget);
      expect(tester.takeException(), isNull);

      tester.view.physicalSize = const Size(640, 420);
      await tester.pumpAndSettle();
      expect(
        find.byKey(const Key('revision3-quest-transcript-compact')),
        findsOneWidget,
      );
      expect(tester.takeException(), isNull);
    },
  );

  testWidgets('late preview from the previous selected row is ignored', (
    tester,
  ) async {
    await _setSurfaceSize(tester, const Size(1000, 700));
    final head = manifestHead(4096, 'b');
    final index = Revision3ContentIndex.fromJsonObject(
      _contentIndexJson(includeSecondLine: true, bindSecondLine: true),
    );
    final first = Completer<AuthoringRevision3DialogLocalizationReadResult>();
    final second = Completer<AuthoringRevision3DialogLocalizationReadResult>();
    final service = _service(
      index: index,
      head: head,
      read:
          ({
            required expectedProjectId,
            required expectedProjectRevision,
            required expectedHead,
            required localizationId,
            required expectedLocalizationRevision,
            required expectedLocId,
          }) =>
              localizationId == _localizationId ? first.future : second.future,
    );
    await _pumpPanel(tester, head: head, service: service);
    await tester.pump();

    final secondRow = find.byKey(const Key('revision3-quest-transcript-row-1'));
    await tester.ensureVisible(secondRow);
    await tester.pump();
    await tester.tap(secondRow);
    await tester.pump();
    first.complete(
      _readResult(
        head: head,
        projectId: _projectId,
        projectRevision: 7,
        localizationId: _localizationId,
        localizationRevision: 3,
        locId: 'DIA_GATE_WARNING',
        locale: 'de',
        text: 'STALE PREVIEW MUST NOT APPEAR',
      ),
    );
    await tester.pump();
    expect(find.text('STALE PREVIEW MUST NOT APPEAR'), findsNothing);

    second.complete(
      _readResult(
        head: head,
        projectId: _projectId,
        projectRevision: 7,
        localizationId: _secondLocalizationId,
        localizationRevision: 1,
        locId: 'DIA_SECOND_WARNING',
        locale: 'en',
        text: 'Current second preview.',
      ),
    );
    await tester.pumpAndSettle();
    expect(find.text('Current second preview.'), findsOneWidget);
  });

  testWidgets('New line is single-flight and inherits exact insertion context', (
    tester,
  ) async {
    await _setSurfaceSize(tester, const Size(1000, 700));
    final head = manifestHead(4096, 'b');
    final index = Revision3ContentIndex.fromJsonObject(_contentIndexJson());
    final service = _service(index: index, head: head);
    final pending = Completer<bool>();
    var calls = 0;
    int? capturedInsertionIndex;
    int? capturedObjectiveSlot;

    await _pumpPanel(
      tester,
      head: head,
      service: service,
      onCreateLine:
          ({
            required projection,
            required int insertionIndex,
            required int objectiveSlot,
            required publishTechnicalPlan,
          }) {
            calls++;
            // Capture only friendly placement intent. The publisher remains an
            // opaque exact-bound capability and is never shown by the panel.
            capturedInsertionIndex = insertionIndex;
            capturedObjectiveSlot = objectiveSlot;
            return pending.future;
          },
    );
    await tester.pumpAndSettle();

    final create = find.byKey(const Key('revision3-quest-transcript-new-line'));
    await tester.tap(create);
    await tester.pump();
    await tester.tap(create);
    await tester.pump();
    expect(calls, 1);
    expect(capturedInsertionIndex, 1);
    expect(capturedObjectiveSlot, 2);

    pending.complete(false);
    await tester.pumpAndSettle();
    expect(calls, 1);
  });

  testWidgets(
    'review attaches, reorders, groups and detaches before exactly one replace',
    (tester) async {
      await _setSurfaceSize(tester, const Size(1000, 760));
      final head = manifestHead(4096, 'b');
      final index = Revision3ContentIndex.fromJsonObject(
        _contentIndexJson(includeSecondLine: true),
      );
      var replaceCalls = 0;
      var publishedCalls = 0;
      final service = _service(
        index: index,
        head: head,
        replace:
            ({
              required expectedProjectId,
              required expectedProjectRevision,
              required expectedHead,
              required plan,
            }) async {
              replaceCalls++;
              return Revision3QuestTranscriptPublication(
                projectId: _projectId,
                projectRevision: 8,
                questId: _questId,
                questRevision: 5,
                moduleId: _moduleId,
                moduleRevision: 5,
                mode: AuthoringRevision3QuestTranscriptMode.replace,
                transcriptCount: plan.bindings.length,
                createdLineId: null,
                createdLocalizationId: null,
                createdVoiceSlotId: null,
                localizationAction: null,
              );
            },
      );
      await _pumpPanel(
        tester,
        head: head,
        service: service,
        onPublished: (_) async => publishedCalls++,
      );
      await tester.pumpAndSettle();

      await tester.tap(
        find.byKey(const Key('revision3-quest-transcript-edit')),
      );
      await tester.pumpAndSettle();
      expect(
        find.byKey(const Key('revision3-quest-transcript-review-dialog')),
        findsOneWidget,
      );

      await tester.tap(
        find.byKey(
          const ValueKey<String>('revision3-quest-transcript-attach-1'),
        ),
      );
      await tester.pumpAndSettle();
      await tester.tap(find.text('Second warning').last);
      await tester.pumpAndSettle();

      await tester.tap(
        find.byKey(const Key('revision3-quest-transcript-review-up-1')),
      );
      await tester.pumpAndSettle();
      await tester.tap(
        find.byKey(const Key('revision3-quest-transcript-review-objective-0')),
      );
      await tester.pumpAndSettle();
      await tester.tap(find.text('Question Asghan').last);
      await tester.pumpAndSettle();
      await tester.tap(
        find.byKey(const Key('revision3-quest-transcript-review-detach-1')),
      );
      await tester.pumpAndSettle();

      expect(find.textContaining('Delete'), findsNothing);
      expect(find.textContaining('delete'), findsNothing);
      await tester.tap(
        find.byKey(const Key('revision3-quest-transcript-review-save')),
      );
      await tester.pumpAndSettle();

      expect(replaceCalls, 1);
      expect(publishedCalls, 1);
      expect(
        find.byKey(const Key('revision3-quest-transcript-review-dialog')),
        findsNothing,
      );
      expect(
        find.text(Revision3QuestTranscriptPanelCopy.english.waitingForRefresh),
        findsOneWidget,
      );
    },
  );

  testWidgets(
    'dirty review guards close and names detach as transcript removal',
    (tester) async {
      await _setSurfaceSize(tester, const Size(800, 640));
      final head = manifestHead(4096, 'b');
      final index = Revision3ContentIndex.fromJsonObject(_contentIndexJson());
      await _pumpPanel(
        tester,
        head: head,
        service: _service(index: index, head: head),
      );
      await tester.pumpAndSettle();

      await tester.tap(
        find.byKey(const Key('revision3-quest-transcript-edit')),
      );
      await tester.pumpAndSettle();
      final detach = find.byKey(
        const Key('revision3-quest-transcript-review-detach-0'),
      );
      expect(
        tester.widget<IconButton>(detach).tooltip,
        'Remove from transcript',
      );
      await tester.tap(detach);
      await tester.tap(
        find.byKey(const Key('revision3-quest-transcript-review-cancel')),
      );
      await tester.pumpAndSettle();
      expect(
        find.byKey(const Key('revision3-quest-transcript-discard-dialog')),
        findsOneWidget,
      );

      await tester.tap(
        find.byKey(const Key('revision3-quest-transcript-keep-editing')),
      );
      await tester.pumpAndSettle();
      expect(
        find.byKey(const Key('revision3-quest-transcript-review-dialog')),
        findsOneWidget,
      );
      await tester.tap(
        find.byKey(const Key('revision3-quest-transcript-review-cancel')),
      );
      await tester.pumpAndSettle();
      await tester.tap(
        find.byKey(const Key('revision3-quest-transcript-discard')),
      );
      await tester.pumpAndSettle();
      expect(
        find.byKey(const Key('revision3-quest-transcript-review-dialog')),
        findsNothing,
      );
    },
  );
}

Future<void> _pumpPanel(
  WidgetTester tester, {
  required AuthoringWorkingHead head,
  required Revision3QuestTranscriptAuthoringService service,
  Revision3QuestTranscriptPanelCopy copy =
      Revision3QuestTranscriptPanelCopy.english,
  Revision3QuestTranscriptCreateLineAction? onCreateLine,
  Revision3QuestTranscriptOpenTextVoiceAction? onOpenTextVoice,
  Revision3QuestTranscriptPublishedAction? onPublished,
}) => tester.pumpWidget(
  MaterialApp(
    home: Scaffold(
      body: SingleChildScrollView(
        padding: const EdgeInsets.all(12),
        child: Revision3QuestTranscriptPanel(
          projectId: _projectId,
          projectRevision: 7,
          projectCheckpointIdentity: head.canonicalJson,
          questId: _questId,
          questRevision: 4,
          service: service,
          selectedLineId: null,
          onSelectedLineChanged: (_) {},
          onCreateLine:
              onCreateLine ??
              ({
                required projection,
                required insertionIndex,
                required objectiveSlot,
                required publishTechnicalPlan,
              }) async => false,
          onOpenTextVoice:
              onOpenTextVoice ??
              ({required projection, required row, required locale}) async =>
                  true,
          onPublished: onPublished,
          copy: copy,
        ),
      ),
    ),
  ),
);

Future<void> _setSurfaceSize(WidgetTester tester, Size size) async {
  tester.view.devicePixelRatio = 1;
  tester.view.physicalSize = size;
  addTearDown(tester.view.reset);
}

Revision3QuestTranscriptAuthoringService _service({
  required Revision3ContentIndex index,
  required AuthoringWorkingHead head,
  Revision3QuestTranscriptContentLoader? load,
  Revision3QuestTranscriptLocalizationReader? read,
  Revision3QuestTranscriptReplacePublisher? replace,
}) => Revision3QuestTranscriptAuthoringService(
  expectedHead: head,
  loadContentIndex: load ?? () async => index,
  readExactLocalization:
      read ??
      ({
        required expectedProjectId,
        required expectedProjectRevision,
        required expectedHead,
        required localizationId,
        required expectedLocalizationRevision,
        required expectedLocId,
      }) async => _readResult(
        head: expectedHead,
        projectId: expectedProjectId,
        projectRevision: expectedProjectRevision,
        localizationId: localizationId,
        localizationRevision: expectedLocalizationRevision,
        locId: expectedLocId,
        locale: localizationId == _secondLocalizationId ? 'en' : 'de',
        text: localizationId == _secondLocalizationId
            ? 'The second warning.'
            : 'Das Tor ist gesichert.',
      ),
  publishReplace:
      replace ??
      ({
        required expectedProjectId,
        required expectedProjectRevision,
        required expectedHead,
        required plan,
      }) async => throw StateError('unexpected replace'),
  publishCreate:
      ({
        required expectedProjectId,
        required expectedProjectRevision,
        required expectedHead,
        required plan,
      }) async => throw StateError('unexpected create'),
);

AuthoringRevision3DialogLocalizationReadResult _readResult({
  required AuthoringWorkingHead head,
  required String projectId,
  required int projectRevision,
  required String localizationId,
  required int localizationRevision,
  required String locId,
  required String locale,
  required String text,
}) {
  final request = AuthoringRevision3DialogLocalizationReadRequestV1(
    expectedHead: head,
    localizationId: localizationId,
    expectedLocalizationRevision: localizationRevision,
    expectedLocId: locId,
  );
  return AuthoringRevision3DialogLocalizationReadResult.fromJson(
    <String, Object?>{
      'ok': true,
      'outcome': 'read_only',
      'head_json': head.canonicalJson,
      'project_id': projectId,
      'project_revision': projectRevision,
      'localization_id': localizationId,
      'localization_revision': localizationRevision,
      'loc_id': locId,
      'locales': <Object?>[
        <String, Object?>{
          'locale': locale,
          'preview': text,
          'truncated': false,
          'has_nonempty_text': true,
        },
      ],
      'content_authority': 'read_only_exact_current_localization',
      'build_status': 'not_evaluated',
      'runtime_status': 'runtime_unqualified',
      'publication_status': 'not_applicable',
    },
    request: request,
  );
}

Map<String, Object?> _contentIndexJson({
  bool includeSecondLine = false,
  bool bindSecondLine = false,
}) => <String, Object?>{
  'schema_revision': 1,
  'project_id': _projectId,
  'project_revision': 7,
  'project_name': 'Transcript fixture',
  'project_version': '1.0.0',
  'project_author': 'tests',
  'target': <String, Object?>{
    'executable': <String, Object?>{'byte_len': 99, 'sha256': _targetSha},
  },
  'authoring_locales': <Object?>['de', if (includeSecondLine) 'en'],
  'entity_counts': <String, Object?>{
    'localization_entry': includeSecondLine ? 2 : 1,
    'dialog_line': includeSecondLine ? 2 : 1,
    'voice_slot': 1,
    'voice_take': 1,
    'quest_draft': 1,
    'script_module': 1,
  },
  'entities': <Object?>[
    _entity(
      id: _localizationId,
      kind: 'localization_entry',
      displayName: 'Gate warning text',
      revision: 3,
      summaryData: <String, Object?>{
        'loc_id': 'DIA_GATE_WARNING',
        'locales': <Object?>['de'],
      },
    ),
    if (includeSecondLine)
      _entity(
        id: _secondLocalizationId,
        kind: 'localization_entry',
        displayName: 'Second warning text',
        revision: 1,
        summaryData: <String, Object?>{
          'loc_id': 'DIA_SECOND_WARNING',
          'locales': <Object?>['en'],
        },
      ),
    _entity(
      id: _lineId,
      kind: 'dialog_line',
      displayName: _lineId,
      revision: 2,
      summaryData: <String, Object?>{
        'speaker_hint': _lineId,
        'voice_slot_locales': <Object?>['de'],
      },
      references: <Object?>[
        _reference(
          role: 'dialog_localization',
          targetId: _localizationId,
          expectedKind: 'localization_entry',
        ),
        _reference(
          role: 'dialog_voice_slot',
          qualifier: 'de',
          targetId: _slotId,
          expectedKind: 'voice_slot',
        ),
      ],
    ),
    if (includeSecondLine)
      _entity(
        id: _secondLineId,
        kind: 'dialog_line',
        displayName: 'Second warning',
        revision: 1,
        summaryData: <String, Object?>{
          'speaker_hint': 'Asghan',
          'voice_slot_locales': <Object?>[],
        },
        references: <Object?>[
          _reference(
            role: 'dialog_localization',
            targetId: _secondLocalizationId,
            expectedKind: 'localization_entry',
          ),
        ],
      ),
    _entity(
      id: _slotId,
      kind: 'voice_slot',
      displayName: 'German Voice',
      revision: 1,
      origin: _generatedOrigin(
        ownerId: _lineId,
        ownerKind: 'dialog_line',
        generatorId: 'gore-authoring.dialog-voice-slot',
      ),
      summaryData: <String, Object?>{
        'locale': 'de',
        'target_resolution': 'resolved',
        'candidate_count': 1,
        'has_selected_take': true,
      },
      references: <Object?>[
        _reference(
          role: 'origin_owner',
          targetId: _lineId,
          expectedKind: 'dialog_line',
        ),
        _reference(
          role: 'voice_candidate',
          targetId: _takeId,
          expectedKind: 'voice_take',
        ),
        _reference(
          role: 'voice_selected',
          targetId: _takeId,
          expectedKind: 'voice_take',
        ),
      ],
    ),
    _entity(
      id: _takeId,
      kind: 'voice_take',
      displayName: 'Take 1',
      revision: 1,
      summaryData: <String, Object?>{
        'locale': 'de',
        'status': 'recorded',
        'codec': 'vorbis',
        'channels': 1,
        'sample_rate': 44100,
      },
    ),
    _entity(
      id: _questId,
      kind: 'quest_draft',
      displayName: 'Secure the gate',
      revision: 4,
      origin: <String, Object?>{
        'type': 'new',
        'authored_runtime_id': 'GORE_SECURE_GATE',
      },
      summaryData: <String, Object?>{
        'technical_id': 'GORE_SECURE_GATE',
        'title': 'Secure the gate',
        'objective_title': 'Question Asghan',
        'additional_objective_titles': <Object?>['Inspect the gate'],
        'objective_slots': <Object?>[1, 2],
        'transcript_count': bindSecondLine ? 2 : 1,
        'module_namespace': 'PROJECT.QUESTS.SECUREGATE',
        'parent_runtime_class': 'UQuest_SwampCamp',
        'giver_runtime_unique_name': 'OM_GRD_Asghan_263',
      },
      references: <Object?>[
        _reference(
          role: 'draft_script_module',
          targetId: _moduleId,
          expectedKind: 'script_module',
        ),
        _reference(
          role: 'quest_transcript_line',
          qualifier: '2',
          targetId: _lineId,
          expectedKind: 'dialog_line',
        ),
        if (bindSecondLine)
          _reference(
            role: 'quest_transcript_line',
            qualifier: '1',
            targetId: _secondLineId,
            expectedKind: 'dialog_line',
          ),
      ],
      assetReferences: _questCollisionAssetReferences,
    ),
    _entity(
      id: _moduleId,
      kind: 'script_module',
      displayName: 'Secure the gate script',
      revision: 5,
      origin: _generatedOrigin(
        ownerId: _questId,
        ownerKind: 'quest_draft',
        generatorId: 'gore-authoring.draft-quest-skeleton',
        generatorVersion: 4,
      ),
      summaryData: <String, Object?>{
        'generator_id': 'gore-authoring.draft-quest-skeleton',
        'generator_version': 4,
        'module_namespace': 'PROJECT.QUESTS.SECUREGATE',
        'module_relative_path': 'PROJECT/QUESTS/SECUREGATE.as',
        'status': <String, Object?>{
          'authoring': 'offline_draft',
          'runtime': 'runtime_unqualified',
        },
      },
      references: <Object?>[
        _reference(
          role: 'origin_owner',
          targetId: _questId,
          expectedKind: 'quest_draft',
        ),
        _reference(
          role: 'script_owner',
          targetId: _questId,
          expectedKind: 'quest_draft',
        ),
      ],
    ),
  ],
  'assets': <Object?>[
    <String, Object?>{
      'sha256': _collisionSha,
      'byte_len': 123,
      'media_type': _collisionMediaType,
      'class': 'quest_collision_artifact',
    },
  ],
};

Map<String, Object?> _entity({
  required String id,
  required String kind,
  required String displayName,
  required int revision,
  required Map<String, Object?> summaryData,
  Map<String, Object?>? origin,
  List<Object?> references = const <Object?>[],
  List<Object?> assetReferences = const <Object?>[],
}) => <String, Object?>{
  'id': id,
  'kind': kind,
  'display_name': displayName,
  'revision': revision,
  'origin':
      origin ??
      <String, Object?>{'type': 'new', 'authored_runtime_id': 'AUTHORED_$kind'},
  'summary': <String, Object?>{'kind': kind, 'data': summaryData},
  'references': references,
  'asset_references': assetReferences,
};

const _questCollisionAssetReferences = <Object?>[
  <String, Object?>{
    'role': 'quest_collision_artifact',
    'sha256': _collisionSha,
    'byte_len': 123,
    'logical_name': null,
    'expected_media_type': _collisionMediaType,
    'resolution': 'resolved',
  },
];

Map<String, Object?> _generatedOrigin({
  required String ownerId,
  required String ownerKind,
  required String generatorId,
  int generatorVersion = 1,
}) => <String, Object?>{
  'type': 'generated',
  'generator_id': generatorId,
  'generator_version': generatorVersion,
  'owner': <String, Object?>{
    'project_id': _projectId,
    'entity_id': ownerId,
    'expected_kind': ownerKind,
  },
};

Map<String, Object?> _reference({
  required String role,
  String? qualifier,
  required String targetId,
  required String expectedKind,
}) => <String, Object?>{
  'role': role,
  'qualifier': qualifier,
  'target': <String, Object?>{
    'project_id': _projectId,
    'entity_id': targetId,
    'expected_kind': expectedKind,
  },
  'resolution': 'resolved',
};
