import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/core/mod_ffi.dart';
import 'package:gore_mod/project/revision3_content_index.dart';
import 'package:gore_mod/project/revision3_quest_journey_panel.dart';
import 'package:gore_mod/project/revision3_quest_journey_service.dart';
import 'package:gore_mod/project/revision3_quest_journey_view.dart';
import 'package:gore_mod/project/revision3_quest_transcript_authoring.dart';
import 'package:gore_mod/project/revision3_quest_transitions_authoring.dart';

import '../support/revision3_quest_outline_fixture.dart';

const _lineId = '40000000000000000000000000000001';
const _localizationId = '50000000000000000000000000000001';
const _collisionMediaType =
    'application/vnd.gore.quest-collision-capability+json;version=2';

void main() {
  testWidgets('loads once and forwards every panel callback', (tester) async {
    await _setSurfaceSize(tester, const Size(1200, 1000));
    final outline = Revision3QuestOutlineFixture();
    final index = _v4Index();
    final harness = _harness(outline: outline, transcriptIndex: index);
    var nameEdits = 0;
    var connectionEdits = 0;
    var transitionEdits = 0;
    var dialogVoiceOpens = 0;
    Revision3QuestTranscriptRow? opened;

    Widget app(Revision3QuestJourneyService service) => _app(
      _view(
        outline: outline,
        index: index,
        service: service,
        giverDisplayName: 'Guard Asghan',
        parentStoryDisplayName: 'Old Camp story',
        onEditNameObjectives: () => nameEdits++,
        onEditDescriptionConnections: () => connectionEdits++,
        onEditStatesTransitions: () => transitionEdits++,
        onOpenDialogVoice: () => dialogVoiceOpens++,
        onOpenDialogLine: (row) => opened = row,
      ),
    );

    await tester.pumpWidget(app(harness.service));
    await tester.pumpAndSettle();

    expect(find.byKey(const Key('revision3-quest-journey-panel')), findsOne);
    expect(find.text('Find Homer'), findsOne);
    expect(
      find.text('Quest giver: Guard Asghan', findRichText: true),
      findsOne,
    );
    expect(find.text('Part of: Old Camp story', findRichText: true), findsOne);
    expect(harness.calls.seedReads, 1);
    expect(harness.calls.transcriptReads, 1);

    await tester.tap(
      find.byKey(const Key('revision3-quest-journey-edit-name-objectives')),
    );
    await tester.pumpAndSettle();
    await tester.tap(
      find.byKey(
        const Key('revision3-quest-journey-edit-description-connections'),
      ),
    );
    await tester.pumpAndSettle();
    await tester.tap(
      find.byKey(const Key('revision3-quest-journey-edit-states-transitions')),
    );
    await tester.pumpAndSettle();
    final setupNext = find.byKey(
      const Key('revision3-quest-draft-setup-recommended-dialog-voice'),
    );
    await tester.ensureVisible(setupNext);
    await tester.tap(setupNext);
    await tester.pumpAndSettle();
    final line = find.byKey(const Key('revision3-quest-journey-dialog-line-0'));
    await tester.ensureVisible(line);
    await tester.tap(line);
    await tester.pumpAndSettle();

    expect(nameEdits, 1);
    expect(connectionEdits, 1);
    expect(transitionEdits, 1);
    expect(dialogVoiceOpens, 1);
    expect(opened?.lineId, _lineId);
    harness.calls.expectNoForbiddenCalls();

    final equivalent = _harness(outline: outline, transcriptIndex: index);
    await tester.pumpWidget(app(equivalent.service));
    await tester.pumpAndSettle();
    expect(harness.calls.seedReads, 1);
    expect(harness.calls.transcriptReads, 1);
    expect(equivalent.calls.seedReads, 0);
    expect(equivalent.calls.transcriptReads, 0);
    expect(find.byKey(const Key('revision3-quest-journey-panel')), findsOne);
  });

  testWidgets('forwards a per-action blocker without disabling sibling edits', (
    tester,
  ) async {
    await _setSurfaceSize(tester, const Size(900, 800));
    final outline = Revision3QuestOutlineFixture();
    final index = _v4Index();
    final harness = _harness(outline: outline, transcriptIndex: index);
    const reason = 'Configure the Gothic game folder first.';
    var nameEdits = 0;

    await tester.pumpWidget(
      _app(
        _view(
          outline: outline,
          index: index,
          service: harness.service,
          onEditNameObjectives: () => nameEdits++,
          editDescriptionConnectionsDisabledReason: reason,
        ),
      ),
    );
    await tester.pumpAndSettle();

    final name = find.byKey(
      const Key('revision3-quest-journey-edit-name-objectives'),
    );
    final connections = find.byKey(
      const Key('revision3-quest-journey-edit-description-connections'),
    );
    expect(tester.widget<OutlinedButton>(name).onPressed, isNotNull);
    expect(tester.widget<OutlinedButton>(connections).onPressed, isNull);
    expect(find.text(reason), findsOne);
    await tester.tap(name);
    await tester.pumpAndSettle();
    expect(nameEdits, 1);
    harness.calls.expectNoForbiddenCalls();
    expect(tester.takeException(), isNull);
  });

  testWidgets('retryable load failure exposes Retry and reloads exactly once', (
    tester,
  ) async {
    final outline = Revision3QuestOutlineFixture();
    final index = _v4Index();
    var attempts = 0;
    var nameEdits = 0;
    final harness = _harness(
      outline: outline,
      transcriptIndex: index,
      loadSeed:
          ({
            required questId,
            required expectedQuestRevision,
            required expectedModuleId,
            required expectedModuleRevision,
          }) async {
            attempts++;
            if (attempts == 1) {
              throw const Revision3QuestTransitionsStaleCheckpointException();
            }
            return _seed(
              outline,
              questId: questId,
              questRevision: expectedQuestRevision,
              moduleId: expectedModuleId,
              moduleRevision: expectedModuleRevision,
            );
          },
    );

    await tester.pumpWidget(
      _app(
        _view(
          outline: outline,
          index: index,
          service: harness.service,
          onEditNameObjectives: () => nameEdits++,
        ),
      ),
    );
    await tester.pumpAndSettle();

    expect(
      find.byKey(const Key('revision3-quest-journey-unavailable')),
      findsOne,
    );
    expect(find.byKey(const Key('revision3-quest-journey-retry')), findsOne);
    final edit = find.byKey(
      const Key('revision3-quest-journey-edit-name-objectives'),
    );
    expect(edit, findsOne);
    await tester.tap(edit);
    await tester.pumpAndSettle();
    expect(nameEdits, 1);

    await tester.tap(find.byKey(const Key('revision3-quest-journey-retry')));
    await tester.pumpAndSettle();

    expect(find.byKey(const Key('revision3-quest-journey-panel')), findsOne);
    expect(attempts, 2);
    expect(harness.calls.seedReads, 2);
    expect(harness.calls.transcriptReads, 2);
    harness.calls.expectNoForbiddenCalls();
  });

  testWidgets('requires-reopen failure is terminal and offers no Retry', (
    tester,
  ) async {
    final outline = Revision3QuestOutlineFixture();
    final index = _v4Index();
    final harness = _harness(
      outline: outline,
      transcriptIndex: index,
      loadSeed:
          ({
            required questId,
            required expectedQuestRevision,
            required expectedModuleId,
            required expectedModuleRevision,
          }) async =>
              throw const Revision3QuestTransitionsRequiresReopenException(),
    );

    await tester.pumpWidget(
      _app(_view(outline: outline, index: index, service: harness.service)),
    );
    await tester.pumpAndSettle();

    expect(
      find.byKey(const Key('revision3-quest-journey-unavailable')),
      findsOne,
    );
    expect(
      find.byKey(const Key('revision3-quest-journey-retry')),
      findsNothing,
    );
    expect(
      find.byKey(const Key('revision3-quest-journey-edit-name-objectives')),
      findsNothing,
    );
    expect(harness.calls.seedReads, 1);
    expect(harness.calls.transcriptReads, 1);
    harness.calls.expectNoForbiddenCalls();
  });

  testWidgets(
    'terminal reopen keeps intended edits visible but safely disabled',
    (tester) async {
      await _setSurfaceSize(tester, const Size(360, 480));
      final outline = Revision3QuestOutlineFixture();
      final index = _v4Index();
      var edits = 0;
      final harness = _harness(
        outline: outline,
        transcriptIndex: index,
        loadSeed:
            ({
              required questId,
              required expectedQuestRevision,
              required expectedModuleId,
              required expectedModuleRevision,
            }) async =>
                throw const Revision3QuestTransitionsRequiresReopenException(),
      );

      await tester.pumpWidget(
        _app(
          _view(
            outline: outline,
            index: index,
            service: harness.service,
            onEditNameObjectives: () => edits++,
          ),
        ),
      );
      await tester.pumpAndSettle();

      final name = find.byKey(
        const Key('revision3-quest-journey-edit-name-objectives'),
      );
      expect(name, findsOne);
      expect(
        find.byKey(
          const Key('revision3-quest-journey-edit-description-connections'),
        ),
        findsOne,
      );
      expect(
        find.byKey(
          const Key('revision3-quest-journey-edit-states-transitions'),
        ),
        findsOne,
      );
      expect(tester.widget<OutlinedButton>(name).onPressed, isNull);
      expect(
        find.textContaining('exact project checkpoint could not be verified'),
        findsOne,
      );
      expect(
        find.byKey(const Key('revision3-quest-journey-edit-disabled-reason')),
        findsNothing,
      );
      expect(
        tester
            .widgetList<Tooltip>(find.byType(Tooltip))
            .where(
              (tooltip) =>
                  tooltip.message ==
                  const Revision3QuestJourneyPanelCopy.english()
                      .unavailableBody,
            ),
        hasLength(3),
      );
      await tester.tap(name, warnIfMissed: false);
      await tester.pump();
      expect(edits, 0);
      expect(
        find.byKey(const Key('revision3-quest-journey-retry')),
        findsNothing,
      );
      expect(tester.takeException(), isNull);
    },
  );

  testWidgets(
    'authority recovery reloads a terminal same-checkpoint view exactly once',
    (tester) async {
      final outline = Revision3QuestOutlineFixture();
      final index = _v4Index();
      var attempts = 0;
      final harness = _harness(
        outline: outline,
        transcriptIndex: index,
        loadSeed:
            ({
              required questId,
              required expectedQuestRevision,
              required expectedModuleId,
              required expectedModuleRevision,
            }) async {
              attempts++;
              if (attempts == 1) {
                throw const Revision3QuestTransitionsRequiresReopenException();
              }
              return _seed(
                outline,
                questId: questId,
                questRevision: expectedQuestRevision,
                moduleId: expectedModuleId,
                moduleRevision: expectedModuleRevision,
              );
            },
      );

      Widget app(int authorityEpoch) => _app(
        _view(
          outline: outline,
          index: index,
          service: harness.service,
          authorityEpoch: authorityEpoch,
        ),
      );

      await tester.pumpWidget(app(0));
      await tester.pumpAndSettle();

      expect(
        find.byKey(const Key('revision3-quest-journey-unavailable')),
        findsOne,
      );
      expect(
        find.byKey(const Key('revision3-quest-journey-retry')),
        findsNothing,
      );
      expect(attempts, 1);
      expect(harness.calls.seedReads, 1);
      expect(harness.calls.transcriptReads, 1);

      await tester.pumpWidget(app(1));
      await tester.pumpAndSettle();

      expect(find.byKey(const Key('revision3-quest-journey-panel')), findsOne);
      expect(attempts, 2);
      expect(harness.calls.seedReads, 2);
      expect(harness.calls.transcriptReads, 2);
      harness.calls.expectNoForbiddenCalls();
    },
  );

  testWidgets(
    'project switch discards a late result and keeps loading bounded',
    (tester) async {
      final oldOutline = Revision3QuestOutlineFixture();
      final oldIndex = _v4Index();
      final oldSeed = Completer<AuthoringRevision3QuestTransitionsSeed>();
      final oldHarness = _harness(
        outline: oldOutline,
        transcriptIndex: oldIndex,
        loadSeed:
            ({
              required questId,
              required expectedQuestRevision,
              required expectedModuleId,
              required expectedModuleRevision,
            }) => oldSeed.future,
      );
      final newOutline = Revision3QuestOutlineFixture(
        projectRevision: 8,
        displayName: 'Find Homer later',
        title: 'Find Homer later',
      );
      final newIndex = _v4Index(projectRevision: 8, title: 'Find Homer later');
      final newHarness = _harness(
        outline: newOutline,
        transcriptIndex: newIndex,
      );

      await tester.pumpWidget(
        _app(
          _view(
            outline: oldOutline,
            index: oldIndex,
            service: oldHarness.service,
          ),
        ),
      );
      await tester.pump();

      final loading = find.byKey(const Key('revision3-quest-journey-loading'));
      expect(loading, findsOne);
      expect(tester.getSize(loading).height, 240);
      expect(tester.getSize(loading).height.isFinite, isTrue);

      await tester.pumpWidget(
        _app(
          _view(
            outline: newOutline,
            index: newIndex,
            service: newHarness.service,
          ),
        ),
      );
      await tester.pumpAndSettle();

      expect(find.text('Find Homer later'), findsOne);
      expect(find.text('Find Homer'), findsNothing);
      expect(newHarness.calls.seedReads, 1);

      oldSeed.complete(
        _seed(
          oldOutline,
          questId: revision3QuestOutlineQuestId,
          questRevision: oldOutline.questRevision,
          moduleId: revision3QuestOutlineModuleId,
          moduleRevision: oldOutline.moduleRevision,
        ),
      );
      await tester.pump();
      await tester.pump();

      expect(find.text('Find Homer later'), findsOne);
      expect(find.text('Find Homer'), findsNothing);
      expect(find.byKey(const Key('revision3-quest-journey-panel')), findsOne);
      expect(oldHarness.calls.seedReads, 1);
      oldHarness.calls.expectNoForbiddenCalls();
      newHarness.calls.expectNoForbiddenCalls();
    },
  );
}

Widget _app(Widget child) => MaterialApp(home: child);

Revision3QuestJourneyView _view({
  required Revision3QuestOutlineFixture outline,
  required Revision3ContentIndex index,
  required Revision3QuestJourneyService service,
  String? giverDisplayName,
  String? parentStoryDisplayName,
  Revision3QuestJourneyAction? onEditNameObjectives,
  Revision3QuestJourneyAction? onEditDescriptionConnections,
  Revision3QuestJourneyAction? onEditStatesTransitions,
  Revision3QuestJourneyAction? onOpenDialogVoice,
  String? editDisabledReason,
  String? editNameObjectivesDisabledReason,
  String? editDescriptionConnectionsDisabledReason,
  String? editStatesTransitionsDisabledReason,
  String? openDialogVoiceDisabledReason,
  Revision3QuestJourneyOpenDialogLine? onOpenDialogLine,
  int authorityEpoch = 0,
}) => Revision3QuestJourneyView(
  projectId: index.projectId,
  projectRevision: index.projectRevision,
  checkpointIdentity: outline.head.canonicalJson,
  index: index,
  quest: index.entityById(revision3QuestOutlineQuestId)!,
  service: service,
  authorityEpoch: authorityEpoch,
  giverDisplayName: giverDisplayName,
  parentStoryDisplayName: parentStoryDisplayName,
  onEditNameObjectives: onEditNameObjectives,
  onEditDescriptionConnections: onEditDescriptionConnections,
  onEditStatesTransitions: onEditStatesTransitions,
  onOpenDialogVoice: onOpenDialogVoice,
  editDisabledReason: editDisabledReason,
  editNameObjectivesDisabledReason: editNameObjectivesDisabledReason,
  editDescriptionConnectionsDisabledReason:
      editDescriptionConnectionsDisabledReason,
  editStatesTransitionsDisabledReason: editStatesTransitionsDisabledReason,
  openDialogVoiceDisabledReason: openDialogVoiceDisabledReason,
  onOpenDialogLine: onOpenDialogLine,
  copy: const Revision3QuestJourneyPanelCopy.english(),
);

({Revision3QuestJourneyService service, _JourneyViewCalls calls}) _harness({
  required Revision3QuestOutlineFixture outline,
  required Revision3ContentIndex transcriptIndex,
  Revision3QuestTransitionsSeedLoader? loadSeed,
}) {
  final calls = _JourneyViewCalls();
  final transitions = Revision3QuestTransitionsAuthoringService(
    loadSeed:
        ({
          required questId,
          required expectedQuestRevision,
          required expectedModuleId,
          required expectedModuleRevision,
        }) async {
          calls.seedReads++;
          final custom = loadSeed;
          if (custom != null) {
            return custom(
              questId: questId,
              expectedQuestRevision: expectedQuestRevision,
              expectedModuleId: expectedModuleId,
              expectedModuleRevision: expectedModuleRevision,
            );
          }
          return _seed(
            outline,
            questId: questId,
            questRevision: expectedQuestRevision,
            moduleId: expectedModuleId,
            moduleRevision: expectedModuleRevision,
          );
        },
    publishTechnicalPlan: ({required plan}) async {
      calls.transitionPublications++;
      throw StateError('Journey view must never publish transitions.');
    },
  );
  final transcript = Revision3QuestTranscriptAuthoringService(
    expectedHead: outline.head,
    loadContentIndex: () async {
      calls.transcriptReads++;
      return transcriptIndex;
    },
    readExactLocalization:
        ({
          required expectedProjectId,
          required expectedProjectRevision,
          required expectedHead,
          required localizationId,
          required expectedLocalizationRevision,
          required expectedLocId,
        }) async {
          calls.localizationReads++;
          throw StateError('Journey view must never read localization text.');
        },
    publishReplace:
        ({
          required expectedProjectId,
          required expectedProjectRevision,
          required expectedHead,
          required plan,
        }) async {
          calls.transcriptReplacements++;
          throw StateError('Journey view must never replace a transcript.');
        },
    publishCreate:
        ({
          required expectedProjectId,
          required expectedProjectRevision,
          required expectedHead,
          required plan,
        }) async {
          calls.transcriptCreations++;
          throw StateError('Journey view must never create transcript rows.');
        },
  );
  return (
    service: Revision3QuestJourneyService(
      transitions: transitions,
      transcript: transcript,
    ),
    calls: calls,
  );
}

AuthoringRevision3QuestTransitionsSeed _seed(
  Revision3QuestOutlineFixture outline, {
  required String questId,
  required int questRevision,
  required String moduleId,
  required int moduleRevision,
}) => AuthoringRevision3QuestTransitionsSeed.forProject(
  currentProjectJson: outline.projectJson,
  questId: questId,
  expectedQuestRevision: questRevision,
  expectedModuleId: moduleId,
  expectedModuleRevision: moduleRevision,
);

final class _JourneyViewCalls {
  int seedReads = 0;
  int transcriptReads = 0;
  int transitionPublications = 0;
  int localizationReads = 0;
  int transcriptReplacements = 0;
  int transcriptCreations = 0;

  void expectNoForbiddenCalls() {
    expect(transitionPublications, 0);
    expect(localizationReads, 0);
    expect(transcriptReplacements, 0);
    expect(transcriptCreations, 0);
  }
}

Future<void> _setSurfaceSize(WidgetTester tester, Size size) async {
  await tester.binding.setSurfaceSize(size);
  addTearDown(() => tester.binding.setSurfaceSize(null));
}

Revision3ContentIndex _v4Index({
  int projectRevision = 7,
  String title = 'Find Homer',
}) => Revision3ContentIndex.fromJsonObject(<String, Object?>{
  'schema_revision': 1,
  'project_id': revision3QuestOutlineProjectId,
  'project_revision': projectRevision,
  'project_name': 'Quest journey view fixture',
  'project_version': '1.0.0',
  'project_author': 'tests',
  'target': <String, Object?>{
    'executable': <String, Object?>{
      'byte_len': 171698176,
      'sha256': revision3QuestOutlineTargetSha,
    },
  },
  'authoring_locales': <Object?>['de'],
  'entity_counts': <String, Object?>{
    'localization_entry': 1,
    'dialog_line': 1,
    'quest_draft': 1,
    'script_module': 1,
  },
  'entities': <Object?>[
    _entity(
      id: revision3QuestOutlineQuestId,
      kind: 'quest_draft',
      displayName: title,
      revision: 4,
      origin: <String, Object?>{
        'type': 'new',
        'authored_runtime_id': 'GORE_FIND_HOMER',
      },
      summary: <String, Object?>{
        'technical_id': 'GORE_FIND_HOMER',
        'title': title,
        'objective_title': 'Ask Asghan about Homer',
        'additional_objective_titles': <Object?>[
          'Inspect the old gate',
          'Report the secured gate',
        ],
        'objective_slots': <Object?>[1, 2, 3],
        'transcript_count': 1,
        'module_namespace': 'PROJECT.QUESTS.FINDHOMER',
        'parent_runtime_class': 'UQuest_SwampCamp_SCChapter2',
        'giver_runtime_unique_name': 'OM_GRD_Asghan_263',
      },
      references: <Object?>[
        _reference(
          role: 'draft_script_module',
          targetId: revision3QuestOutlineModuleId,
          expectedKind: 'script_module',
        ),
        _reference(
          role: 'quest_transcript_line',
          qualifier: '1',
          targetId: _lineId,
          expectedKind: 'dialog_line',
        ),
      ],
      assetReferences: <Object?>[
        <String, Object?>{
          'role': 'quest_collision_artifact',
          'sha256': revision3QuestOutlineArtifactSha,
          'byte_len': 123,
          'logical_name': null,
          'expected_media_type': _collisionMediaType,
          'resolution': 'resolved',
        },
      ],
    ),
    _entity(
      id: revision3QuestOutlineModuleId,
      kind: 'script_module',
      displayName: '$title Script',
      revision: 5,
      origin: <String, Object?>{
        'type': 'generated',
        'generator_id': 'gore-authoring.draft-quest-skeleton',
        'generator_version': 4,
        'owner': <String, Object?>{
          'project_id': revision3QuestOutlineProjectId,
          'entity_id': revision3QuestOutlineQuestId,
          'expected_kind': 'quest_draft',
        },
      },
      summary: <String, Object?>{
        'generator_id': 'gore-authoring.draft-quest-skeleton',
        'generator_version': 4,
        'module_namespace': 'PROJECT.QUESTS.FINDHOMER',
        'module_relative_path': 'PROJECT/QUESTS/FINDHOMER.as',
        'status': <String, Object?>{
          'authoring': 'offline_draft',
          'runtime': 'runtime_unqualified',
        },
      },
      references: <Object?>[
        _reference(
          role: 'origin_owner',
          targetId: revision3QuestOutlineQuestId,
          expectedKind: 'quest_draft',
        ),
        _reference(
          role: 'script_owner',
          targetId: revision3QuestOutlineQuestId,
          expectedKind: 'quest_draft',
        ),
      ],
    ),
    _entity(
      id: _lineId,
      kind: 'dialog_line',
      displayName: 'General greeting',
      revision: 1,
      summary: <String, Object?>{
        'speaker_hint': 'Asghan',
        'voice_slot_locales': <Object?>[],
      },
      references: <Object?>[
        _reference(
          role: 'dialog_localization',
          targetId: _localizationId,
          expectedKind: 'localization_entry',
        ),
      ],
    ),
    _entity(
      id: _localizationId,
      kind: 'localization_entry',
      displayName: 'General greeting text',
      revision: 1,
      summary: <String, Object?>{
        'loc_id': 'DIA_JOURNEY_GENERAL',
        'locales': <Object?>['de'],
      },
    ),
  ],
  'assets': <Object?>[
    <String, Object?>{
      'sha256': revision3QuestOutlineArtifactSha,
      'byte_len': 123,
      'media_type': _collisionMediaType,
      'class': 'quest_collision_artifact',
    },
  ],
});

Map<String, Object?> _entity({
  required String id,
  required String kind,
  required String displayName,
  required int revision,
  required Map<String, Object?> summary,
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
      <String, Object?>{
        'type': 'new',
        'authored_runtime_id': 'AUTHORED_${kind.toUpperCase()}_$id',
      },
  'summary': <String, Object?>{'kind': kind, 'data': summary},
  'references': references,
  'asset_references': assetReferences,
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
    'project_id': revision3QuestOutlineProjectId,
    'entity_id': targetId,
    'expected_kind': expectedKind,
  },
  'resolution': 'resolved',
};
