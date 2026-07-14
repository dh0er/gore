import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/core/mod_ffi.dart';
import 'package:gore_mod/project/revision3_content_index.dart';
import 'package:gore_mod/project/revision3_voice_authoring.dart';
import 'package:gore_mod/project/revision3_voice_wizard.dart';

import '../support/revision3_voice_content_fixture.dart';

const _lineLabel =
    'Asghan — Mine entrance question · GRD_263_ASGHAN_OPEN_INFO_06_02';

void main() {
  testWidgets(
    'requires an explicit friendly line choice and publishes an approved take',
    (tester) async {
      await _useLargeSurface(tester);
      var loads = 0;
      Revision3VoiceTakeTechnicalPlan? captured;
      final service = Revision3VoiceAuthoringService(
        loadContentIndex: () async {
          loads++;
          return revision3VoiceContentIndexFixture();
        },
        publishTechnicalPlan:
            ({
              required expectedProjectId,
              required expectedProjectRevision,
              required plan,
            }) async {
              captured = plan;
              return _publication(
                projectId: expectedProjectId,
                revision: expectedProjectRevision + 1,
                plan: plan,
              );
            },
      );
      await _openWizard(
        tester,
        service: service,
        picker: () async => r'C:\Voice\asghan_final.ogg',
      );

      expect(find.text(_lineLabel), findsNothing);
      expect(find.text(revision3VoiceContentLineId), findsNothing);
      expect(find.text(revision3VoiceContentSlotId), findsNothing);
      expect(find.text('Saved to project only'), findsOneWidget);
      expect(find.text('Not yet usable in game'), findsOneWidget);
      expect(
        find.byKey(const Key('revision3-voice-localization-preserved')),
        findsOneWidget,
      );
      expect(find.byKey(const Key('revision3-voice-edit-text')), findsNothing);
      expect(
        find.byKey(const Key('revision3-voice-dialog-text')),
        findsNothing,
      );
      expect(_submitButton(tester).onPressed, isNull);

      await _selectLine(tester, query: 'GRD_263_ASGHAN', label: _lineLabel);
      expect(_submitButton(tester).onPressed, isNotNull);
      expect(
        tester
            .widget<ChoiceChip>(
              find.byKey(const Key('revision3-voice-locale-de')),
            )
            .selected,
        isTrue,
      );

      await tester.tap(find.byKey(const Key('revision3-voice-browse')));
      await tester.pumpAndSettle();
      expect(
        tester
            .widget<TextFormField>(
              find.byKey(const Key('revision3-voice-take-name')),
            )
            .controller!
            .text,
        'asghan_final',
      );

      await _markApprovedAndSelect(tester);
      expect(
        find.byKey(const Key('revision3-voice-replacement-warning')),
        findsNothing,
      );
      await _submit(tester);

      expect(find.byKey(const Key('revision3-voice-wizard')), findsNothing);
      expect(loads, 2, reason: 'submit refreshes the exact content index');
      expect(captured, isNotNull);
      expect(captured!.lineId, revision3VoiceContentLineId);
      expect(captured!.slotId, revision3VoiceContentSlotId);
      expect(captured!.logicalName, 'asghan_final.ogg');
      expect(captured!.selectTake, isTrue);
      expect(captured!.text, isNull);
    },
  );

  testWidgets('default Voice workflow preserves localization exactly', (
    tester,
  ) async {
    await _useLargeSurface(tester);
    Revision3VoiceTakeTechnicalPlan? captured;
    final index = revision3VoiceContentIndexFixture();
    final service = Revision3VoiceAuthoringService(
      loadContentIndex: () async => index,
      publishTechnicalPlan:
          ({
            required expectedProjectId,
            required expectedProjectRevision,
            required plan,
          }) async {
            captured = plan;
            return _publication(
              projectId: expectedProjectId,
              revision: expectedProjectRevision + 1,
              plan: plan,
            );
          },
    );
    await _openWizard(tester, service: service);
    await _selectLine(tester, query: 'Asghan', label: _lineLabel);
    await _fillSourceAndName(tester);
    await _submit(tester);

    expect(captured, isNotNull);
    expect(captured!.text, isNull);
    expect(captured!.selectTake, isFalse);
  });

  testWidgets('duplicate results hide IDs and target changes reset selection', (
    tester,
  ) async {
    await _useLargeSurface(tester);
    final index = revision3VoiceContentIndexFixture(
      duplicateLine: true,
      existingSlotCandidateCount: 2,
      existingSlotHasSelectedTake: true,
    );
    final service = Revision3VoiceAuthoringService(
      loadContentIndex: () async => index,
      publishTechnicalPlan:
          ({
            required expectedProjectId,
            required expectedProjectRevision,
            required plan,
          }) async => _publication(
            projectId: expectedProjectId,
            revision: expectedProjectRevision + 1,
            plan: plan,
          ),
    );
    await _openWizard(tester, service: service);

    const first = '$_lineLabel · 1 of 2';
    const second = '$_lineLabel · 2 of 2';
    await tester.enterText(
      find.byKey(const Key('revision3-voice-line-search')),
      'GRD_263_ASGHAN',
    );
    await tester.pumpAndSettle();
    expect(find.text(first), findsOneWidget);
    expect(find.text(second), findsOneWidget);
    expect(find.text(revision3VoiceContentLineId), findsNothing);
    expect(find.text(revision3VoiceContentDuplicateLineId), findsNothing);
    await tester.tap(find.text(first));
    await tester.pumpAndSettle();

    await tester.enterText(
      find.byKey(const Key('revision3-voice-locale')),
      ' de ',
    );
    await _fillSourceAndName(tester);
    await _markApprovedAndSelect(tester);
    expect(
      find.byKey(const Key('revision3-voice-replacement-warning')),
      findsOneWidget,
      reason: 'trimmed canonical locale must still detect the existing slot',
    );
    expect(_submitButton(tester).onPressed, isNull);
    await tester.ensureVisible(
      find.byKey(const Key('revision3-voice-confirm-replacement')),
    );
    await tester.tap(
      find.byKey(const Key('revision3-voice-confirm-replacement')),
    );
    await tester.pump();
    expect(_submitButton(tester).onPressed, isNotNull);

    await tester.enterText(
      find.byKey(const Key('revision3-voice-locale')),
      'en',
    );
    await tester.pump();
    expect(_selectedTakeCheckbox(tester).value, isFalse);
    expect(
      find.byKey(const Key('revision3-voice-replacement-warning')),
      findsNothing,
    );

    await _selectLine(tester, query: '2 of 2', label: second);
    expect(_selectedTakeCheckbox(tester).value, isFalse);
    expect(
      find.byKey(const Key('revision3-voice-replacement-warning')),
      findsNothing,
    );
  });

  testWidgets('take name follows re-picks only until the user edits it', (
    tester,
  ) async {
    await _useLargeSurface(tester);
    final paths = <String>[
      r'C:\Voice\first.ogg',
      r'C:\Voice\second.ogg',
      r'C:\Voice\third.ogg',
    ];
    var pick = 0;
    final service = Revision3VoiceAuthoringService(
      loadContentIndex: () async => revision3VoiceContentIndexFixture(),
      publishTechnicalPlan:
          ({
            required expectedProjectId,
            required expectedProjectRevision,
            required plan,
          }) async => _publication(
            projectId: expectedProjectId,
            revision: expectedProjectRevision + 1,
            plan: plan,
          ),
    );
    await _openWizard(
      tester,
      service: service,
      picker: () async => paths[pick++],
    );
    final browse = find.byKey(const Key('revision3-voice-browse'));
    final takeName = find.byKey(const Key('revision3-voice-take-name'));

    await tester.tap(browse);
    await tester.pumpAndSettle();
    expect(_text(tester, takeName), 'first');
    await tester.tap(browse);
    await tester.pumpAndSettle();
    expect(_text(tester, takeName), 'second');
    await tester.enterText(takeName, 'My deliberate name');
    await tester.tap(browse);
    await tester.pumpAndSettle();
    expect(_text(tester, takeName), 'My deliberate name');
  });

  testWidgets('unsafe existing locale is explained and cannot be submitted', (
    tester,
  ) async {
    await _useLargeSurface(tester);
    var publishes = 0;
    final service = Revision3VoiceAuthoringService(
      loadContentIndex: () async {
        final json = revision3VoiceContentIndexJsonFixture(
          existingSlotCandidateCount: 1,
          existingSlotTargetResolution: 'resolved',
        );
        final entities = (json['entities']! as List).cast<Object?>();
        final take = (entities.cast<Map<String, Object?>>().singleWhere(
          (entity) => entity['kind'] == 'voice_take',
        ));
        final summary = (take['summary']! as Map).cast<String, Object?>();
        final data = (summary['data']! as Map).cast<String, Object?>();
        data['locale'] = 'en';
        return Revision3ContentIndex.fromJsonObject(json);
      },
      publishTechnicalPlan:
          ({
            required expectedProjectId,
            required expectedProjectRevision,
            required plan,
          }) async {
            publishes++;
            return _publication(
              projectId: expectedProjectId,
              revision: expectedProjectRevision + 1,
              plan: plan,
            );
          },
    );
    await _openWizard(tester, service: service);
    await _selectLine(tester, query: 'Asghan', label: _lineLabel);

    expect(
      find.textContaining('project graph is not safe to extend'),
      findsOneWidget,
    );
    expect(find.textContaining('No Voice slot exists'), findsNothing);
    expect(_submitButton(tester).onPressed, isNull);

    await tester.enterText(
      find.byKey(const Key('revision3-voice-locale')),
      'en',
    );
    await tester.pump();
    expect(find.textContaining('No Voice slot exists'), findsOneWidget);
    expect(_submitButton(tester).onPressed, isNotNull);
    expect(publishes, 0);
  });

  testWidgets('locale validation uses the exact canonical authoring rule', (
    tester,
  ) async {
    await _useLargeSurface(tester);
    final service = Revision3VoiceAuthoringService(
      loadContentIndex: () async => revision3VoiceContentIndexFixture(),
      publishTechnicalPlan:
          ({
            required expectedProjectId,
            required expectedProjectRevision,
            required plan,
          }) async => _publication(
            projectId: expectedProjectId,
            revision: expectedProjectRevision + 1,
            plan: plan,
          ),
    );
    await _openWizard(tester, service: service);
    await _selectLine(tester, query: 'Asghan', label: _lineLabel);
    await _fillSourceAndName(tester);
    await tester.enterText(
      find.byKey(const Key('revision3-voice-locale')),
      'en-us',
    );
    await _submit(tester);

    expect(
      find.text('Use a language code such as de or en-US'),
      findsOneWidget,
    );
  });

  testWidgets('retryable native source failures get safe actionable messages', (
    tester,
  ) async {
    await _useLargeSurface(tester);
    const cases = <(String, String)>[
      (
        'AUTHORING_REVISION3_VOICE_GAME_ROOT_UNAVAILABLE',
        'The configured Gothic 1 Remake installation is unavailable. Check it in Settings, then try again.',
      ),
      (
        'AUTHORING_REVISION3_VOICE_STORE_GAME_ALIAS',
        'This project folder overlaps the configured game installation. Move the project outside the game folder before adding a Voice take.',
      ),
      (
        'AUTHORING_REVISION3_VOICE_INPUT_MISSING',
        'The selected Ogg file no longer exists. Choose the recording again.',
      ),
      (
        'AUTHORING_REVISION3_VOICE_INPUT_UNAVAILABLE',
        'The selected Ogg file could not be read. Close any app that is holding it, then try again.',
      ),
      (
        'AUTHORING_REVISION3_VOICE_INPUT_UNSAFE',
        'The selected source could not be opened safely. Choose a regular local .ogg file.',
      ),
      (
        'AUTHORING_REVISION3_VOICE_INPUT_LIMIT',
        'The selected Ogg file is larger than the supported import limit.',
      ),
      (
        'AUTHORING_REVISION3_VOICE_OGG_INVALID',
        'The selected file is not a supported, valid Vorbis or Opus Ogg recording.',
      ),
      (
        'AUTHORING_REVISION3_VOICE_INPUT_CHANGED',
        'The Ogg file changed while it was being verified. Wait for the recording to finish, then choose it again.',
      ),
    ];
    var attempt = 0;
    final service = Revision3VoiceAuthoringService(
      loadContentIndex: () async => revision3VoiceContentIndexFixture(),
      publishTechnicalPlan:
          ({
            required expectedProjectId,
            required expectedProjectRevision,
            required plan,
          }) async {
            final code = cases[attempt++].$1;
            throw ModFfiException(
              command: 'authoring_store_prepare_revision3_voice_take_v1',
              code: code,
              message: 'private native detail must not be shown',
            );
          },
    );
    await _openWizard(tester, service: service);
    await _selectLine(tester, query: 'Asghan', label: _lineLabel);
    await _fillSourceAndName(tester);

    for (final testCase in cases) {
      await _submit(tester);
      expect(find.text(testCase.$2), findsOneWidget, reason: testCase.$1);
      expect(find.textContaining('private native detail'), findsNothing);
      expect(_submitButton(tester).onPressed, isNotNull);
    }
    expect(attempt, cases.length);
  });

  testWidgets('stale checkpoint blocks publication and asks for a fresh wizard', (
    tester,
  ) async {
    await _useLargeSurface(tester);
    var loads = 0;
    var publishes = 0;
    final service = Revision3VoiceAuthoringService(
      loadContentIndex: () async =>
          revision3VoiceContentIndexFixture(revision: loads++ == 0 ? 7 : 8),
      publishTechnicalPlan:
          ({
            required expectedProjectId,
            required expectedProjectRevision,
            required plan,
          }) async {
            publishes++;
            return _publication(
              projectId: expectedProjectId,
              revision: expectedProjectRevision + 1,
              plan: plan,
            );
          },
    );
    await _openWizard(tester, service: service);
    await _selectLine(tester, query: 'Asghan', label: _lineLabel);
    await _fillSourceAndName(tester);
    await _submit(tester);

    expect(publishes, 0);
    expect(
      find.text(
        'The managed project changed while this window was open. Close it and add the take again from the current project.',
      ),
      findsOneWidget,
    );
    expect(_submitButton(tester).onPressed, isNull);
  });
}

Future<void> _useLargeSurface(WidgetTester tester) async {
  await tester.binding.setSurfaceSize(const Size(1000, 1100));
  addTearDown(() => tester.binding.setSurfaceSize(null));
}

Future<void> _selectLine(
  WidgetTester tester, {
  required String query,
  required String label,
}) async {
  await tester.enterText(
    find.byKey(const Key('revision3-voice-line-search')),
    query,
  );
  await tester.pumpAndSettle();
  expect(find.text(label), findsOneWidget);
  await tester.tap(find.text(label));
  await tester.pumpAndSettle();
}

Future<void> _fillSourceAndName(WidgetTester tester) async {
  await tester.enterText(
    find.byKey(const Key('revision3-voice-source')),
    r'C:\Voice\asghan.ogg',
  );
  await tester.enterText(
    find.byKey(const Key('revision3-voice-take-name')),
    'Asghan take 1',
  );
}

Future<void> _markApprovedAndSelect(WidgetTester tester) async {
  await tester.ensureVisible(find.byKey(const Key('revision3-voice-status')));
  await tester.tap(find.byKey(const Key('revision3-voice-status')));
  await tester.pumpAndSettle();
  await tester.tap(find.text('Approved').last);
  await tester.pumpAndSettle();
  await tester.ensureVisible(find.byKey(const Key('revision3-voice-select')));
  await tester.tap(find.byKey(const Key('revision3-voice-select')));
  await tester.pump();
}

Future<void> _submit(WidgetTester tester) async {
  await tester.ensureVisible(find.byKey(const Key('revision3-voice-submit')));
  await tester.tap(find.byKey(const Key('revision3-voice-submit')));
  await tester.pumpAndSettle();
}

FilledButton _submitButton(WidgetTester tester) => tester.widget<FilledButton>(
  find.byKey(const Key('revision3-voice-submit')),
);

CheckboxListTile _selectedTakeCheckbox(WidgetTester tester) => tester
    .widget<CheckboxListTile>(find.byKey(const Key('revision3-voice-select')));

String _text(WidgetTester tester, Finder finder) =>
    tester.widget<TextFormField>(finder).controller!.text;

Future<void> _openWizard(
  WidgetTester tester, {
  required Revision3VoiceAuthoringService service,
  Revision3VoiceOggPicker? picker,
}) async {
  await tester.pumpWidget(
    MaterialApp(
      home: Scaffold(
        body: Builder(
          builder: (context) => FilledButton(
            key: const Key('open-voice-wizard'),
            onPressed: () => showDialog<Revision3VoiceTakePublication>(
              context: context,
              builder: (_) =>
                  Revision3VoiceTakeDialog(service: service, pickOgg: picker),
            ),
            child: const Text('Open'),
          ),
        ),
      ),
    ),
  );
  await tester.tap(find.byKey(const Key('open-voice-wizard')));
  await tester.pumpAndSettle();
}

Revision3VoiceTakePublication _publication({
  required String projectId,
  required int revision,
  required Revision3VoiceTakeTechnicalPlan plan,
}) => Revision3VoiceTakePublication(
  projectId: projectId,
  projectRevision: revision,
  lineId: plan.lineId,
  slotId: plan.slotId,
  takeId: plan.takeId,
  slotCreated: plan.expectsSlotCreated,
  selected: plan.selectTake,
);
