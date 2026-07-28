import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/core/mod_ffi.dart';
import 'package:gore_mod/project/revision3_content_index.dart';
import 'package:gore_mod/project/revision3_dialog_line_authoring.dart';
import 'package:gore_mod/project/revision3_npc_dialog_voice_panel.dart';
import 'package:gore_mod/project/revision3_npc_greeting_authoring.dart';

import '../support/revision3_quest_outline_fixture.dart';

const _projectId = 'ffffffffffffffffffffffffffffffff';
const _localizationId = '11111111111111111111111111111111';
const _lineId = '22222222222222222222222222222222';
const _slotId = '33333333333333333333333333333333';
const _takeId = '44444444444444444444444444444444';
const _npcId = '55555555555555555555555555555555';
const _moduleId = '66666666666666666666666666666666';
const _secondLocalizationId = '12121212121212121212121212121212';
const _secondLineId = '23232323232323232323232323232323';
const _targetSha =
    'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa';

void main() {
  testWidgets(
    'shows friendly greeting coverage and never renders technical identity',
    (tester) async {
      await _setSurfaceSize(tester, const Size(1200, 800));
      final head = manifestHead(4096, 'b');
      final index = Revision3ContentIndex.fromJsonObject(_contentIndexJson());
      var reads = 0;

      await _pumpPanel(
        tester,
        head: head,
        service: _service(
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
                  text: 'Willkommen im Lager.',
                );
              },
        ),
        copy: Revision3NpcDialogVoicePanelCopy.german,
      );
      await tester.pumpAndSettle();

      expect(
        find.byKey(const Key('revision3-npc-greeting-wide')),
        findsOneWidget,
      );
      expect(find.text('Begr\u00fc\u00dfungszeilen'), findsOneWidget);
      expect(find.text('Gate welcome'), findsWidgets);
      expect(find.text('Sprecher: Asghan'), findsWidgets);
      expect(find.text('Sprachen: de'), findsOneWidget);
      expect(find.text('Text 1/1'), findsOneWidget);
      expect(find.text('Voice 1/1 \u00b7 1 Aufnahmen'), findsOneWidget);
      expect(find.text('1 ausgew\u00e4hlt'), findsOneWidget);
      expect(find.text('Willkommen im Lager.'), findsOneWidget);
      expect(reads, 1, reason: 'only the selected row is read lazily');

      for (final technicalValue in <String>[
        _projectId,
        _localizationId,
        _lineId,
        _slotId,
        _takeId,
        _npcId,
        _moduleId,
        'DIA_GATE_WELCOME',
        'PROJECT.NPCS.ASGHAN',
      ]) {
        expect(find.textContaining(technicalValue), findsNothing);
      }
      expect(tester.takeException(), isNull);
    },
  );

  testWidgets('selects a row and hands off its exact text and Voice target', (
    tester,
  ) async {
    await _setSurfaceSize(tester, const Size(1000, 720));
    final head = manifestHead(4096, 'b');
    final index = Revision3ContentIndex.fromJsonObject(
      _contentIndexJson(bindSecondLine: true),
    );
    Revision3NpcGreetingProjection? openedProjection;
    Revision3NpcGreetingRow? openedRow;
    String? openedLocale;
    String? selectedLine;

    await _pumpPanel(
      tester,
      head: head,
      service: _service(index: index, head: head),
      onSelectedLineChanged: (lineId) => selectedLine = lineId,
      onOpenTextVoice:
          ({required projection, required row, required locale}) async {
            openedProjection = projection;
            openedRow = row;
            openedLocale = locale;
            return true;
          },
    );
    await tester.pumpAndSettle();

    final second = find.byKey(const Key('revision3-npc-greeting-row-1'));
    await tester.ensureVisible(second);
    await tester.tap(second);
    await tester.pumpAndSettle();
    expect(selectedLine, _secondLineId);
    expect(find.text('The camp is closed.'), findsOneWidget);

    final open = find.byKey(
      const Key('revision3-npc-greeting-open-text-voice'),
    );
    await tester.ensureVisible(open);
    await tester.tap(open);
    await tester.pumpAndSettle();

    expect(openedProjection?.checkpointIdentity, head.canonicalJson);
    expect(openedProjection?.npcId, _npcId);
    expect(openedRow?.lineId, _secondLineId);
    expect(openedLocale, 'en');
    expect(tester.takeException(), isNull);
  });

  testWidgets('New greeting line receives an exact-bound create publisher', (
    tester,
  ) async {
    await _setSurfaceSize(tester, const Size(1000, 720));
    final head = manifestHead(4096, 'b');
    final index = Revision3ContentIndex.fromJsonObject(_contentIndexJson());
    Revision3NpcGreetingCreateTechnicalPlan? capturedPlan;
    int? capturedInsertionIndex;
    var wrongCheckpointRejected = false;
    var createCalls = 0;
    final service = _service(
      index: index,
      head: head,
      create:
          ({
            required expectedProjectId,
            required expectedProjectRevision,
            required expectedHead,
            required plan,
          }) async {
            createCalls++;
            capturedPlan = plan;
            return Revision3NpcGreetingPublication(
              projectId: expectedProjectId,
              projectRevision: expectedProjectRevision + 1,
              npcId: _npcId,
              npcRevision: 5,
              moduleId: _moduleId,
              moduleRevision: 5,
              mode: AuthoringRevision3NpcGreetingMode.createAndInsert,
              greetingCount: 2,
              createdLineId: plan.line.lineId,
              createdLocalizationId: plan.line.localization.localizationId,
              createdVoiceSlotId: plan.line.voiceSlot?.slotId,
              localizationAction:
                  AuthoringRevision3DialogLocalizationAction.created,
            );
          },
    );

    await _pumpPanel(
      tester,
      head: head,
      service: service,
      onCreateLine:
          ({
            required projection,
            required insertionIndex,
            required publishTechnicalPlan,
          }) async {
            capturedInsertionIndex = insertionIndex;
            final plan = Revision3DialogLineEntryTechnicalPlan.forCheckpoint(
              catalog: Revision3DialogLineEntryCatalog.fromContentIndex(index),
              input: Revision3DialogLineEntryInput.create(
                lineDisplayName: 'Fresh greeting',
                speakerHint: 'Asghan',
                locale: 'de',
                text: 'Ein neuer Gru\u00df.',
              ),
            );
            try {
              await publishTechnicalPlan(
                expectedProjectId: '00000000000000000000000000000000',
                expectedProjectRevision: 7,
                plan: plan,
              );
            } on Revision3NpcGreetingStaleCheckpointException {
              wrongCheckpointRejected = true;
            }
            await publishTechnicalPlan(
              expectedProjectId: _projectId,
              expectedProjectRevision: 7,
              plan: plan,
            );
            return true;
          },
    );
    await tester.pumpAndSettle();

    await tester.tap(find.byKey(const Key('revision3-npc-greeting-new-line')));
    await tester.pumpAndSettle();

    expect(capturedInsertionIndex, 1);
    expect(wrongCheckpointRejected, isTrue);
    expect(createCalls, 1);
    expect(capturedPlan?.npcId, _npcId);
    expect(capturedPlan?.expectedNpcRevision, 4);
    expect(capturedPlan?.expectedModuleId, _moduleId);
    expect(capturedPlan?.expectedModuleRevision, 5);
    expect(capturedPlan?.expectedGreetingCount, 1);
    expect(capturedPlan?.index, 1);
    expect(capturedPlan?.line.lineDisplayName, 'Fresh greeting');
    expect(
      find.text(Revision3NpcDialogVoicePanelCopy.english.waitingForRefresh),
      findsOneWidget,
    );
  });

  testWidgets(
    'review attaches, reorders and detaches before one exact replace',
    (tester) async {
      await _setSurfaceSize(tester, const Size(1000, 760));
      final head = manifestHead(4096, 'b');
      final index = Revision3ContentIndex.fromJsonObject(_contentIndexJson());
      List<String>? publishedBindings;
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
              publishedBindings = [
                for (final binding in plan.bindings) binding.lineId,
              ];
              return Revision3NpcGreetingPublication(
                projectId: expectedProjectId,
                projectRevision: expectedProjectRevision + 1,
                npcId: _npcId,
                npcRevision: 5,
                moduleId: _moduleId,
                moduleRevision: 5,
                mode: AuthoringRevision3NpcGreetingMode.replace,
                greetingCount: plan.bindings.length,
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

      await tester.tap(find.byKey(const Key('revision3-npc-greeting-edit')));
      await tester.pumpAndSettle();
      expect(
        find.byKey(const Key('revision3-npc-greeting-review-dialog')),
        findsOneWidget,
      );

      await tester.tap(
        find.byKey(const ValueKey<String>('revision3-npc-greeting-attach-1')),
      );
      await tester.pumpAndSettle();
      await tester.tap(find.text('Camp warning').last);
      await tester.pumpAndSettle();
      await tester.tap(
        find.byKey(const Key('revision3-npc-greeting-review-up-1')),
      );
      await tester.pumpAndSettle();
      await tester.tap(
        find.byKey(const Key('revision3-npc-greeting-review-detach-1')),
      );
      await tester.pumpAndSettle();

      expect(find.textContaining(_lineId), findsNothing);
      expect(find.textContaining(_secondLineId), findsNothing);
      await tester.tap(
        find.byKey(const Key('revision3-npc-greeting-review-save')),
      );
      await tester.pumpAndSettle();

      expect(replaceCalls, 1);
      expect(publishedCalls, 1);
      expect(publishedBindings, <String>[_secondLineId]);
      expect(
        find.byKey(const Key('revision3-npc-greeting-review-dialog')),
        findsNothing,
      );
      expect(
        find.text(Revision3NpcDialogVoicePanelCopy.english.waitingForRefresh),
        findsOneWidget,
      );
    },
  );

  testWidgets(
    'compact 200 percent German layout has no overflow and keeps mutations fail closed',
    (tester) async {
      await _setSurfaceSize(tester, const Size(360, 640));
      final head = manifestHead(4096, 'b');
      final index = Revision3ContentIndex.fromJsonObject(_contentIndexJson());
      var openCalls = 0;

      await _pumpPanel(
        tester,
        head: head,
        service: _service(index: index, head: head),
        copy: Revision3NpcDialogVoicePanelCopy.german,
        mutationsEnabled: false,
        mutationDisabledReason:
            'Speichere oder verwirf die au\u00dfergew\u00f6hnlich umfangreichen offenen Text\u00e4nderungen, bevor du diese Begr\u00fc\u00dfungen bearbeitest.',
        textScaler: const TextScaler.linear(2),
        onOpenTextVoice:
            ({required projection, required row, required locale}) async {
              openCalls++;
              return true;
            },
      );
      await tester.pumpAndSettle();

      expect(
        find.byKey(const Key('revision3-npc-greeting-compact')),
        findsOneWidget,
      );
      expect(
        tester
            .widget<FilledButton>(
              find.byKey(const Key('revision3-npc-greeting-new-line')),
            )
            .onPressed,
        isNull,
      );
      expect(
        tester
            .widget<OutlinedButton>(
              find.byKey(const Key('revision3-npc-greeting-edit')),
            )
            .onPressed,
        isNull,
      );
      final open = find.byKey(
        const Key('revision3-npc-greeting-open-text-voice'),
      );
      expect(tester.widget<FilledButton>(open).onPressed, isNotNull);
      await tester.ensureVisible(open);
      await tester.tap(open);
      await tester.pumpAndSettle();
      expect(openCalls, 1);
      final longBlock = find.textContaining('au\u00dfergew\u00f6hnlich');
      expect(longBlock, findsOneWidget);
      final blockText = tester.widget<Text>(longBlock);
      expect(blockText.maxLines, isNull);
      expect(blockText.overflow, isNull);
      expect(tester.takeException(), isNull);
    },
  );

  testWidgets('compact 200 percent review remains fully usable', (
    tester,
  ) async {
    await _setSurfaceSize(tester, const Size(360, 640));
    final head = manifestHead(4096, 'b');
    final index = Revision3ContentIndex.fromJsonObject(_contentIndexJson());

    await _pumpPanel(
      tester,
      head: head,
      service: _service(index: index, head: head),
      copy: Revision3NpcDialogVoicePanelCopy.german,
      textScaler: const TextScaler.linear(2),
    );
    await tester.pumpAndSettle();

    final edit = find.byKey(const Key('revision3-npc-greeting-edit'));
    await tester.ensureVisible(edit);
    await tester.tap(edit);
    await tester.pumpAndSettle();

    expect(
      find.byKey(const Key('revision3-npc-greeting-review-dialog')),
      findsOneWidget,
    );
    expect(
      find.byKey(const Key('revision3-npc-greeting-review-chrome-scroll')),
      findsOneWidget,
    );
    final detach = find.byKey(
      const Key('revision3-npc-greeting-review-detach-0'),
    );
    await tester.ensureVisible(detach);
    await tester.tap(detach);
    await tester.pumpAndSettle();
    expect(
      tester
          .widget<FilledButton>(
            find.byKey(const Key('revision3-npc-greeting-review-save')),
          )
          .onPressed,
      isNotNull,
    );
    expect(tester.takeException(), isNull);
  });

  testWidgets('requires-reopen load failure exposes no retry or mutation', (
    tester,
  ) async {
    await _setSurfaceSize(tester, const Size(800, 640));
    final head = manifestHead(4096, 'b');
    final index = Revision3ContentIndex.fromJsonObject(_contentIndexJson());

    await _pumpPanel(
      tester,
      head: head,
      service: _service(
        index: index,
        head: head,
        load: () async => throw const Revision3ContentRequiresReopenException(),
      ),
    );
    await tester.pumpAndSettle();

    expect(find.text('NPC greetings unavailable'), findsOneWidget);
    expect(find.byKey(const Key('revision3-npc-greeting-retry')), findsNothing);
    expect(
      tester
          .widget<FilledButton>(
            find.byKey(const Key('revision3-npc-greeting-new-line')),
          )
          .onPressed,
      isNull,
    );
    expect(
      tester
          .widget<OutlinedButton>(
            find.byKey(const Key('revision3-npc-greeting-edit')),
          )
          .onPressed,
      isNull,
    );
  });

  testWidgets('stale save keeps review open and disables every mutation', (
    tester,
  ) async {
    await _setSurfaceSize(tester, const Size(1000, 760));
    final head = manifestHead(4096, 'b');
    final index = Revision3ContentIndex.fromJsonObject(_contentIndexJson());
    final service = _service(
      index: index,
      head: head,
      replace:
          ({
            required expectedProjectId,
            required expectedProjectRevision,
            required expectedHead,
            required plan,
          }) async =>
              throw const Revision3NpcGreetingStaleCheckpointException(),
    );

    await _pumpPanel(tester, head: head, service: service);
    await tester.pumpAndSettle();
    await tester.tap(find.byKey(const Key('revision3-npc-greeting-edit')));
    await tester.pumpAndSettle();
    await tester.tap(
      find.byKey(const Key('revision3-npc-greeting-review-detach-0')),
    );
    await tester.pumpAndSettle();
    await tester.tap(
      find.byKey(const Key('revision3-npc-greeting-review-save')),
    );
    await tester.pumpAndSettle();

    expect(
      find.byKey(const Key('revision3-npc-greeting-review-dialog')),
      findsOneWidget,
    );
    expect(
      find.text(Revision3NpcDialogVoicePanelCopy.english.requiresReopen),
      findsWidgets,
    );
    expect(
      tester
          .widget<FilledButton>(
            find.byKey(const Key('revision3-npc-greeting-review-save')),
          )
          .onPressed,
      isNull,
    );
    expect(
      tester
          .widget<DropdownButtonFormField<Revision3NpcGreetingLineChoice>>(
            find.byType(
              DropdownButtonFormField<Revision3NpcGreetingLineChoice>,
            ),
          )
          .onChanged,
      isNull,
    );
  });
}

Future<void> _pumpPanel(
  WidgetTester tester, {
  required AuthoringWorkingHead head,
  required Revision3NpcGreetingAuthoringService service,
  Revision3NpcDialogVoicePanelCopy copy =
      Revision3NpcDialogVoicePanelCopy.english,
  ValueChanged<String?>? onSelectedLineChanged,
  Revision3NpcGreetingCreateLineAction? onCreateLine,
  Revision3NpcGreetingOpenTextVoiceAction? onOpenTextVoice,
  Revision3NpcGreetingPublishedAction? onPublished,
  bool mutationsEnabled = true,
  String? mutationDisabledReason,
  TextScaler textScaler = TextScaler.noScaling,
}) => tester.pumpWidget(
  MaterialApp(
    builder: (context, child) => MediaQuery(
      data: MediaQuery.of(context).copyWith(textScaler: textScaler),
      child: child!,
    ),
    home: Scaffold(
      body: SingleChildScrollView(
        padding: const EdgeInsets.all(12),
        child: Revision3NpcDialogVoicePanel(
          projectId: _projectId,
          projectRevision: 7,
          projectCheckpointIdentity: head.canonicalJson,
          npcId: _npcId,
          npcRevision: 4,
          service: service,
          selectedLineId: null,
          onSelectedLineChanged: onSelectedLineChanged ?? (_) {},
          onCreateLine:
              onCreateLine ??
              ({
                required projection,
                required insertionIndex,
                required publishTechnicalPlan,
              }) async => false,
          onOpenTextVoice:
              onOpenTextVoice ??
              ({required projection, required row, required locale}) async =>
                  true,
          onPublished: onPublished,
          mutationsEnabled: mutationsEnabled,
          mutationDisabledReason: mutationDisabledReason,
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

Revision3NpcGreetingAuthoringService _service({
  required Revision3ContentIndex index,
  required AuthoringWorkingHead head,
  Revision3NpcGreetingContentLoader? load,
  Revision3NpcGreetingLocalizationReader? read,
  Revision3NpcGreetingReplacePublisher? replace,
  Revision3NpcGreetingCreatePublisher? create,
}) => Revision3NpcGreetingAuthoringService(
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
            ? 'The camp is closed.'
            : 'Welcome to the camp.',
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
      create ??
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

Map<String, Object?> _contentIndexJson({bool bindSecondLine = false}) =>
    <String, Object?>{
      'schema_revision': 1,
      'project_id': _projectId,
      'project_revision': 7,
      'project_name': 'NPC greeting fixture',
      'project_version': '1.0.0',
      'project_author': 'tests',
      'target': <String, Object?>{
        'executable': <String, Object?>{'byte_len': 99, 'sha256': _targetSha},
      },
      'authoring_locales': <Object?>['de', 'en'],
      'entity_counts': <String, Object?>{
        'localization_entry': 2,
        'dialog_line': 2,
        'voice_slot': 1,
        'voice_take': 1,
        'npc_draft': 1,
        'script_module': 1,
      },
      'entities': <Object?>[
        _entity(
          id: _localizationId,
          kind: 'localization_entry',
          displayName: 'Gate welcome text',
          revision: 3,
          summaryData: <String, Object?>{
            'loc_id': 'DIA_GATE_WELCOME',
            'locales': <Object?>['de'],
          },
        ),
        _entity(
          id: _secondLocalizationId,
          kind: 'localization_entry',
          displayName: 'Camp warning text',
          revision: 1,
          summaryData: <String, Object?>{
            'loc_id': 'DIA_CAMP_WARNING',
            'locales': <Object?>['en'],
          },
        ),
        _entity(
          id: _lineId,
          kind: 'dialog_line',
          displayName: 'Gate welcome',
          revision: 2,
          summaryData: <String, Object?>{
            'speaker_hint': 'Asghan',
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
        _entity(
          id: _secondLineId,
          kind: 'dialog_line',
          displayName: 'Camp warning',
          revision: 1,
          summaryData: <String, Object?>{
            'speaker_hint': 'Guard',
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
          id: _npcId,
          kind: 'npc_draft',
          displayName: 'Asghan',
          revision: 4,
          origin: <String, Object?>{
            'type': 'new',
            'authored_runtime_id': 'OM_GRD_Asghan_263',
          },
          summaryData: <String, Object?>{
            'unique_name': 'OM_GRD_Asghan_263',
            'module_namespace': 'PROJECT.NPCS.ASGHAN',
            'parent_character_definition': 'C_HUMAN',
            'parent_ai_agent_config': 'AIV_HUMAN',
            'parent_spawn_definition': 'SPAWN_HUMAN',
            'greeting_count': bindSecondLine ? 2 : 1,
          },
          references: <Object?>[
            _reference(
              role: 'draft_script_module',
              targetId: _moduleId,
              expectedKind: 'script_module',
            ),
            _reference(
              role: 'npc_greeting_line',
              targetId: _lineId,
              expectedKind: 'dialog_line',
            ),
            if (bindSecondLine)
              _reference(
                role: 'npc_greeting_line',
                targetId: _secondLineId,
                expectedKind: 'dialog_line',
              ),
          ],
        ),
        _entity(
          id: _moduleId,
          kind: 'script_module',
          displayName: 'Asghan script',
          revision: 5,
          origin: _generatedOrigin(
            ownerId: _npcId,
            ownerKind: 'npc_draft',
            generatorId: 'gore-authoring.logical-npc-clone-draft',
          ),
          summaryData: <String, Object?>{
            'generator_id': 'gore-authoring.logical-npc-clone-draft',
            'generator_version': 1,
            'module_namespace': 'PROJECT.NPCS.ASGHAN',
            'module_relative_path': 'PROJECT/NPCS/ASGHAN.as',
            'status': <String, Object?>{
              'authoring': 'offline_draft',
              'runtime': 'runtime_unqualified',
            },
          },
          references: <Object?>[
            _reference(
              role: 'origin_owner',
              targetId: _npcId,
              expectedKind: 'npc_draft',
            ),
            _reference(
              role: 'script_owner',
              targetId: _npcId,
              expectedKind: 'npc_draft',
            ),
          ],
        ),
      ],
      'assets': <Object?>[],
    };

Map<String, Object?> _entity({
  required String id,
  required String kind,
  required String displayName,
  required int revision,
  required Map<String, Object?> summaryData,
  Map<String, Object?>? origin,
  List<Object?> references = const <Object?>[],
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
  'asset_references': <Object?>[],
};

Map<String, Object?> _generatedOrigin({
  required String ownerId,
  required String ownerKind,
  required String generatorId,
}) => <String, Object?>{
  'type': 'generated',
  'generator_id': generatorId,
  'generator_version': 1,
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
