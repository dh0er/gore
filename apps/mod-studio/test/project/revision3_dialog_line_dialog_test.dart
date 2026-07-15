import 'dart:async';
import 'dart:convert';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/core/mod_ffi.dart';
import 'package:gore_mod/project/revision3_content_index.dart';
import 'package:gore_mod/project/revision3_dialog_line_authoring.dart';
import 'package:gore_mod/project/revision3_dialog_line_dialog.dart';

import '../support/revision3_voice_content_fixture.dart';

const _unusedLocalizationId = '77777777777777777777777777777777';
const _unusedLocalizationLocId = 'GORE_UNUSED_GREETING';
const _secondUnusedLocalizationId = '88888888888888888888888888888888';
const _secondUnusedLocalizationLocId = 'GORE_UNUSED_GREETING_SECOND';

const _copy = Revision3DialogLineEntryDialogCopy(
  title: 'Create a Voice line',
  introduction: 'Create the project data needed before adding a recording.',
  projectOnlyBoundary:
      'Project only. This does not access the game or a save, create a topic or AngelScript, or make playable dialog.',
  createMode: 'Create new text',
  reuseMode: 'Use unused project text',
  lineNameLabel: 'Line title',
  lineNameHint: 'Mine entrance greeting',
  speakerLabel: 'Speaker (optional)',
  speakerHint: 'Asghan',
  localeLabel: 'Language',
  textLabel: 'Dialog text',
  reuseSearchLabel: 'Search unused project text',
  noReusableText: 'No unused project text is available.',
  createVoiceSlotLabel: 'Prepare an empty Voice slot',
  createVoiceSlotHelp: 'A recording can be added next.',
  cancel: 'Cancel',
  save: 'Save to project',
  saving: 'Saving to project',
  loading: 'Loading project text',
  loadFailed: 'Project text could not be loaded.',
  retry: 'Retry',
  stale: 'The project changed. Close this window and start again.',
  requiresReopen: 'Close this window and reopen the managed project.',
  invalidInput: 'Review the line details and project text.',
  saveFailed: 'The line could not be saved safely. Nothing touched the game.',
  saved: _savedCopy,
  done: 'Done',
  addRecording: 'Add a recording next',
);

String _savedCopy(int revision) => 'Saved in project revision $revision.';

Revision3DialogLineEntryDialogCopy _copyWithInvalidInput(String invalidInput) =>
    Revision3DialogLineEntryDialogCopy(
      title: _copy.title,
      introduction: _copy.introduction,
      projectOnlyBoundary: _copy.projectOnlyBoundary,
      createMode: _copy.createMode,
      reuseMode: _copy.reuseMode,
      lineNameLabel: _copy.lineNameLabel,
      lineNameHint: _copy.lineNameHint,
      speakerLabel: _copy.speakerLabel,
      speakerHint: _copy.speakerHint,
      localeLabel: _copy.localeLabel,
      textLabel: _copy.textLabel,
      reuseSearchLabel: _copy.reuseSearchLabel,
      noReusableText: _copy.noReusableText,
      createVoiceSlotLabel: _copy.createVoiceSlotLabel,
      createVoiceSlotHelp: _copy.createVoiceSlotHelp,
      cancel: _copy.cancel,
      save: _copy.save,
      saving: _copy.saving,
      loading: _copy.loading,
      loadFailed: _copy.loadFailed,
      retry: _copy.retry,
      stale: _copy.stale,
      requiresReopen: _copy.requiresReopen,
      invalidInput: invalidInput,
      saveFailed: _copy.saveFailed,
      saved: _copy.saved,
      done: _copy.done,
      addRecording: _copy.addRecording,
    );

void main() {
  testWidgets(
    'Create publishes, shows success, and returns the Voice handoff',
    (tester) async {
      await _setSurface(tester, width: 1100);
      var loads = 0;
      var exactReads = 0;
      Revision3DialogLineEntryTechnicalPlan? publishedPlan;
      Revision3DialogLineEntryDialogResult? result;
      final service = Revision3DialogLineEntryAuthoringService(
        loadContentIndex: () async {
          loads++;
          return _contentIndex();
        },
        readExactLocalization:
            ({
              required expectedProjectId,
              required expectedProjectRevision,
              required localizationId,
              required expectedLocalizationRevision,
              required expectedLocId,
            }) async {
              exactReads++;
              return _successfulLocalizationRead(
                expectedProjectId: expectedProjectId,
                expectedProjectRevision: expectedProjectRevision,
                localizationId: localizationId,
                expectedLocalizationRevision: expectedLocalizationRevision,
                expectedLocId: expectedLocId,
              );
            },
        publishTechnicalPlan:
            ({
              required expectedProjectId,
              required expectedProjectRevision,
              required plan,
            }) async {
              publishedPlan = plan;
              return _publication(
                projectId: expectedProjectId,
                revision: expectedProjectRevision + 1,
                plan: plan,
              );
            },
      );

      await _openDialog(
        tester,
        service: service,
        onResult: (value) => result = value,
      );

      expect(
        find.byKey(const Key('revision3-dialog-line-modal')),
        findsOneWidget,
      );
      expect(
        find.byKey(const Key('revision3-dialog-line-fullscreen')),
        findsNothing,
      );
      expect(find.text(_copy.projectOnlyBoundary), findsOneWidget);
      expect(find.text(_unusedLocalizationId), findsNothing);
      expect(find.text(_unusedLocalizationLocId), findsNothing);

      await _fillCreate(
        tester,
        name: 'Mine entrance greeting',
        speaker: 'Asghan',
        text: 'You should not be here.',
      );
      await _submit(tester);

      expect(loads, 2, reason: 'publish must refresh the exact content index');
      expect(exactReads, 0, reason: 'Create must not read reusable text');
      expect(
        find.byKey(const Key('revision3-dialog-line-success')),
        findsOneWidget,
      );
      expect(find.text('Saved in project revision 8.'), findsOneWidget);
      expect(
        result,
        isNull,
        reason: 'success still needs an explicit next step',
      );

      final plan = publishedPlan!;
      expect(plan.lineDisplayName, 'Mine entrance greeting');
      expect(plan.speakerHint, 'Asghan');
      expect(plan.locale, 'de');
      expect(plan.voiceSlot, isNotNull);
      final localization =
          plan.localization
              as AuthoringRevision3DialogLocalizationCreateIntentV1;
      expect(localization.texts, {'de': 'You should not be here.'});

      await tester.tap(
        find.byKey(const Key('revision3-dialog-line-open-voice')),
      );
      await tester.pumpAndSettle();

      expect(result?.publication.projectRevision, 8);
      expect(result?.publication.lineId, plan.lineId);
      expect(result?.openVoiceNext, isTrue);
      expect(
        find.byKey(const Key('revision3-dialog-line-modal')),
        findsNothing,
      );
    },
  );

  testWidgets(
    'ReuseExact offers only unused managed text and binds its locale',
    (tester) async {
      await _setSurface(tester, width: 1100);
      Revision3DialogLineEntryTechnicalPlan? publishedPlan;
      Revision3DialogLineEntryDialogResult? result;
      final service = Revision3DialogLineEntryAuthoringService(
        loadContentIndex: () async => _contentIndex(),
        readExactLocalization: _successfulLocalizationRead,
        publishTechnicalPlan:
            ({
              required expectedProjectId,
              required expectedProjectRevision,
              required plan,
            }) async {
              publishedPlan = plan;
              return _publication(
                projectId: expectedProjectId,
                revision: expectedProjectRevision + 1,
                plan: plan,
              );
            },
      );

      await _openDialog(
        tester,
        service: service,
        initialMode: Revision3DialogLineEntryMode.reuseExact,
        onResult: (value) => result = value,
      );

      expect(find.text('Unused greeting'), findsOneWidget);
      expect(find.text('Used Asghan text'), findsNothing);
      expect(find.text(_unusedLocalizationId), findsNothing);
      expect(find.text(_unusedLocalizationLocId), findsNothing);
      await tester.enterText(
        find.byKey(const Key('revision3-dialog-line-reuse-search')),
        'unused',
      );
      await tester.pump();
      await tester.tap(find.text('Unused greeting'));
      await tester.pumpAndSettle();

      expect(
        find.byKey(const Key('revision3-dialog-line-reuse-preview')),
        findsOneWidget,
      );
      expect(find.text('Willkommen in der Mine.'), findsOneWidget);
      expect(find.text(_unusedLocalizationId), findsNothing);
      expect(find.text(_unusedLocalizationLocId), findsNothing);

      await tester.enterText(
        find.byKey(const Key('revision3-dialog-line-name')),
        'Shared mine warning',
      );
      await tester.drag(
        find.byKey(const Key('revision3-dialog-line-editor')),
        const Offset(0, -260),
      );
      await tester.pumpAndSettle();
      await tester.tap(
        find.byKey(const Key('revision3-dialog-line-locale-en')),
      );
      await tester.pump();
      await _submit(tester);

      final plan = publishedPlan!;
      expect(plan.lineDisplayName, 'Shared mine warning');
      expect(plan.locale, 'en');
      expect(plan.voiceSlot?.locale, 'en');
      final reuse =
          plan.localization
              as AuthoringRevision3DialogLocalizationReuseExactIntentV1;
      expect(reuse.localizationId, _unusedLocalizationId);
      expect(reuse.expectedLocalizationRevision, 4);
      expect(reuse.expectedLocId, _unusedLocalizationLocId);

      await tester.tap(find.byKey(const Key('revision3-dialog-line-done')));
      await tester.pumpAndSettle();
      expect(
        result?.publication.localizationAction,
        AuthoringRevision3DialogLocalizationAction.reusedExact,
      );
      expect(result?.openVoiceNext, isFalse);
    },
  );

  testWidgets('Reuse search clears a selection hidden by the new filter', (
    tester,
  ) async {
    await _setSurface(tester, width: 1100);
    await _openDialog(
      tester,
      service: _successfulService(),
      initialMode: Revision3DialogLineEntryMode.reuseExact,
    );

    await tester.tap(find.text('Unused greeting'));
    await tester.pump();
    expect(_submitButton(tester).onPressed, isNotNull);

    await tester.enterText(
      find.byKey(const Key('revision3-dialog-line-reuse-search')),
      'does not match this text',
    );
    await tester.pump();
    expect(find.text('Unused greeting'), findsNothing);
    expect(_submitButton(tester).onPressed, isNull);

    await tester.enterText(
      find.byKey(const Key('revision3-dialog-line-reuse-search')),
      'unused',
    );
    await tester.pump();
    expect(find.text('Unused greeting'), findsOneWidget);
    expect(
      _submitButton(tester).onPressed,
      isNull,
      reason: 'a hidden choice must not silently reselect itself',
    );
  });

  testWidgets('whitespace-only exact text offers no authorable locale', (
    tester,
  ) async {
    await _setSurface(tester, width: 1100);
    final service = Revision3DialogLineEntryAuthoringService(
      loadContentIndex: () async => _contentIndex(),
      readExactLocalization:
          ({
            required expectedProjectId,
            required expectedProjectRevision,
            required localizationId,
            required expectedLocalizationRevision,
            required expectedLocId,
          }) async => _localizationReadResult(
            expectedProjectId: expectedProjectId,
            expectedProjectRevision: expectedProjectRevision,
            localizationId: localizationId,
            expectedLocalizationRevision: expectedLocalizationRevision,
            expectedLocId: expectedLocId,
            locales: <Map<String, Object?>>[
              <String, Object?>{
                'locale': 'de',
                'preview': '   ',
                'truncated': false,
                'has_nonempty_text': false,
              },
              <String, Object?>{
                'locale': 'en',
                'preview': '',
                'truncated': false,
                'has_nonempty_text': false,
              },
            ],
          ),
      publishTechnicalPlan:
          ({
            required expectedProjectId,
            required expectedProjectRevision,
            required plan,
          }) async => throw StateError('empty text must not publish'),
    );
    await _openDialog(
      tester,
      service: service,
      initialMode: Revision3DialogLineEntryMode.reuseExact,
    );

    await tester.tap(find.text('Unused greeting'));
    await tester.pumpAndSettle();

    expect(find.text(_copy.noReusableText), findsOneWidget);
    expect(
      find.byKey(const Key('revision3-dialog-line-locale-de')),
      findsNothing,
    );
    expect(
      find.byKey(const Key('revision3-dialog-line-locale-en')),
      findsNothing,
    );
    expect(_submitButton(tester).onPressed, isNull);
  });

  testWidgets(
    'empty ReuseExact restores a valid locale when returning to Create',
    (tester) async {
      await _setSurface(tester, width: 1100);
      Revision3DialogLineEntryTechnicalPlan? publishedPlan;
      final service = Revision3DialogLineEntryAuthoringService(
        loadContentIndex: () async => _contentIndex(),
        readExactLocalization:
            ({
              required expectedProjectId,
              required expectedProjectRevision,
              required localizationId,
              required expectedLocalizationRevision,
              required expectedLocId,
            }) async => _localizationReadResult(
              expectedProjectId: expectedProjectId,
              expectedProjectRevision: expectedProjectRevision,
              localizationId: localizationId,
              expectedLocalizationRevision: expectedLocalizationRevision,
              expectedLocId: expectedLocId,
              locales: <Map<String, Object?>>[
                <String, Object?>{
                  'locale': 'de',
                  'preview': ' ',
                  'truncated': false,
                  'has_nonempty_text': false,
                },
                <String, Object?>{
                  'locale': 'en',
                  'preview': '',
                  'truncated': false,
                  'has_nonempty_text': false,
                },
              ],
            ),
        publishTechnicalPlan:
            ({
              required expectedProjectId,
              required expectedProjectRevision,
              required plan,
            }) async {
              publishedPlan = plan;
              return _publication(
                projectId: expectedProjectId,
                revision: expectedProjectRevision + 1,
                plan: plan,
              );
            },
      );
      await _openDialog(
        tester,
        service: service,
        initialMode: Revision3DialogLineEntryMode.reuseExact,
      );

      await tester.tap(find.text('Unused greeting'));
      await tester.pumpAndSettle();
      expect(
        tester
            .widget<TextFormField>(
              find.byKey(const Key('revision3-dialog-line-locale')),
            )
            .controller!
            .text,
        isEmpty,
      );

      await tester.tap(find.text(_copy.createMode));
      await tester.pump();
      expect(
        tester
            .widget<TextFormField>(
              find.byKey(const Key('revision3-dialog-line-locale')),
            )
            .controller!
            .text,
        'de',
      );
      await _fillCreate(tester, name: 'Recovered create line');
      await _submit(tester);

      expect(publishedPlan?.locale, 'de');
      expect(
        publishedPlan?.localization,
        isA<AuthoringRevision3DialogLocalizationCreateIntentV1>(),
      );
      expect(
        find.byKey(const Key('revision3-dialog-line-success')),
        findsOneWidget,
      );
    },
  );

  testWidgets('mode change ignores a pending ReuseExact preview', (
    tester,
  ) async {
    await _setSurface(tester, width: 1100);
    final pending = Completer<AuthoringRevision3DialogLocalizationReadResult>();
    final service = Revision3DialogLineEntryAuthoringService(
      loadContentIndex: () async => _contentIndex(),
      readExactLocalization:
          ({
            required expectedProjectId,
            required expectedProjectRevision,
            required localizationId,
            required expectedLocalizationRevision,
            required expectedLocId,
          }) => pending.future,
      publishTechnicalPlan:
          ({
            required expectedProjectId,
            required expectedProjectRevision,
            required plan,
          }) async => throw StateError('race test must not publish'),
    );
    await _openDialog(
      tester,
      service: service,
      initialMode: Revision3DialogLineEntryMode.reuseExact,
    );

    await tester.tap(find.text('Unused greeting'));
    await tester.pump();
    expect(
      find.byKey(const Key('revision3-dialog-line-preview-loading')),
      findsOneWidget,
    );
    await tester.tap(find.text(_copy.createMode));
    await tester.pump();
    pending.complete(
      _localizationReadResult(
        expectedProjectId: revision3VoiceContentProjectId,
        expectedProjectRevision: 7,
        localizationId: _unusedLocalizationId,
        expectedLocalizationRevision: 4,
        expectedLocId: _unusedLocalizationLocId,
      ),
    );
    await tester.pumpAndSettle();

    expect(
      find.byKey(const Key('revision3-dialog-line-reuse-preview')),
      findsNothing,
    );
    expect(find.byKey(const Key('revision3-dialog-line-text')), findsOneWidget);
  });

  testWidgets('search change ignores a pending hidden selection preview', (
    tester,
  ) async {
    await _setSurface(tester, width: 1100);
    final pending = Completer<AuthoringRevision3DialogLocalizationReadResult>();
    final service = Revision3DialogLineEntryAuthoringService(
      loadContentIndex: () async => _contentIndex(),
      readExactLocalization:
          ({
            required expectedProjectId,
            required expectedProjectRevision,
            required localizationId,
            required expectedLocalizationRevision,
            required expectedLocId,
          }) => pending.future,
      publishTechnicalPlan:
          ({
            required expectedProjectId,
            required expectedProjectRevision,
            required plan,
          }) async => throw StateError('race test must not publish'),
    );
    await _openDialog(
      tester,
      service: service,
      initialMode: Revision3DialogLineEntryMode.reuseExact,
    );

    await tester.tap(find.text('Unused greeting'));
    await tester.pump();
    await tester.enterText(
      find.byKey(const Key('revision3-dialog-line-reuse-search')),
      'no matching project text',
    );
    await tester.pump();
    pending.complete(
      _localizationReadResult(
        expectedProjectId: revision3VoiceContentProjectId,
        expectedProjectRevision: 7,
        localizationId: _unusedLocalizationId,
        expectedLocalizationRevision: 4,
        expectedLocId: _unusedLocalizationLocId,
      ),
    );
    await tester.pumpAndSettle();

    expect(find.text('Unused greeting'), findsNothing);
    expect(
      find.byKey(const Key('revision3-dialog-line-reuse-preview')),
      findsNothing,
    );
    expect(_submitButton(tester).onPressed, isNull);
  });

  testWidgets('requires-reopen during preview locks the dialog', (
    tester,
  ) async {
    await _setSurface(tester, width: 1100);
    final service = Revision3DialogLineEntryAuthoringService(
      loadContentIndex: () async => _contentIndex(),
      readExactLocalization:
          ({
            required expectedProjectId,
            required expectedProjectRevision,
            required localizationId,
            required expectedLocalizationRevision,
            required expectedLocId,
          }) async =>
              throw const Revision3DialogLineEntryRequiresReopenException(),
      publishTechnicalPlan:
          ({
            required expectedProjectId,
            required expectedProjectRevision,
            required plan,
          }) async => throw StateError('reopen test must not publish'),
    );
    await _openDialog(
      tester,
      service: service,
      initialMode: Revision3DialogLineEntryMode.reuseExact,
    );

    await tester.tap(find.text('Unused greeting'));
    await tester.pumpAndSettle();

    expect(find.text(_copy.requiresReopen), findsOneWidget);
    expect(_submitButton(tester).onPressed, isNull);
    expect(
      find.byKey(const Key('revision3-dialog-line-reuse-preview')),
      findsNothing,
    );
  });

  testWidgets('non-ASCII byte overflow shows only localized input copy', (
    tester,
  ) async {
    await _setSurface(tester, width: 1100);
    var publications = 0;
    const localizedInput = 'Bitte prüfe die Angaben zur Dialogzeile.';
    final service = Revision3DialogLineEntryAuthoringService(
      loadContentIndex: () async => _contentIndex(),
      readExactLocalization: _successfulLocalizationRead,
      publishTechnicalPlan:
          ({
            required expectedProjectId,
            required expectedProjectRevision,
            required plan,
          }) async {
            publications++;
            throw StateError('invalid input must not publish');
          },
    );
    await _openDialog(
      tester,
      service: service,
      copy: _copyWithInvalidInput(localizedInput),
    );
    await _fillCreate(tester, name: 'ä' * 97);

    await _submit(tester);

    expect(publications, 0);
    expect(find.text(localizedInput), findsOneWidget);
    expect(find.textContaining('valid line name'), findsNothing);
  });

  testWidgets('malformed preview uses load-failed copy', (tester) async {
    await _setSurface(tester, width: 1100);
    final service = Revision3DialogLineEntryAuthoringService(
      loadContentIndex: () async => _contentIndex(),
      readExactLocalization:
          ({
            required expectedProjectId,
            required expectedProjectRevision,
            required localizationId,
            required expectedLocalizationRevision,
            required expectedLocId,
          }) async => throw const FormatException('technical wire detail'),
      publishTechnicalPlan:
          ({
            required expectedProjectId,
            required expectedProjectRevision,
            required plan,
          }) async => throw StateError('malformed preview must not publish'),
    );
    await _openDialog(
      tester,
      service: service,
      initialMode: Revision3DialogLineEntryMode.reuseExact,
    );

    await tester.tap(find.text('Unused greeting'));
    await tester.pumpAndSettle();

    expect(find.text(_copy.loadFailed), findsOneWidget);
    expect(find.textContaining('technical wire'), findsNothing);
    expect(_submitButton(tester).onPressed, isNull);
  });

  testWidgets('publish re-reads selected locale and blocks late empty text', (
    tester,
  ) async {
    await _setSurface(tester, width: 1100);
    var reads = 0;
    var publications = 0;
    final service = Revision3DialogLineEntryAuthoringService(
      loadContentIndex: () async => _contentIndex(),
      readExactLocalization:
          ({
            required expectedProjectId,
            required expectedProjectRevision,
            required localizationId,
            required expectedLocalizationRevision,
            required expectedLocId,
          }) async {
            reads++;
            return _localizationReadResult(
              expectedProjectId: expectedProjectId,
              expectedProjectRevision: expectedProjectRevision,
              localizationId: localizationId,
              expectedLocalizationRevision: expectedLocalizationRevision,
              expectedLocId: expectedLocId,
              locales: reads == 1
                  ? null
                  : <Map<String, Object?>>[
                      <String, Object?>{
                        'locale': 'de',
                        'preview': '   ',
                        'truncated': false,
                        'has_nonempty_text': false,
                      },
                      <String, Object?>{
                        'locale': 'en',
                        'preview': 'Still English',
                        'truncated': false,
                        'has_nonempty_text': true,
                      },
                    ],
            );
          },
      publishTechnicalPlan:
          ({
            required expectedProjectId,
            required expectedProjectRevision,
            required plan,
          }) async {
            publications++;
            throw StateError('late empty text must not publish');
          },
    );
    await _openDialog(
      tester,
      service: service,
      initialMode: Revision3DialogLineEntryMode.reuseExact,
    );
    await tester.tap(find.text('Unused greeting'));
    await tester.pumpAndSettle();
    await tester.enterText(
      find.byKey(const Key('revision3-dialog-line-name')),
      'Late empty German line',
    );

    await _submit(tester);

    expect(reads, 2);
    expect(publications, 0);
    expect(find.text(_copy.noReusableText), findsOneWidget);
    expect(
      find.byKey(const Key('revision3-dialog-line-success')),
      findsNothing,
    );
  });

  testWidgets('rapid duplicate selection ignores the stale preview', (
    tester,
  ) async {
    await _setSurface(tester, width: 1100);
    final first = Completer<AuthoringRevision3DialogLocalizationReadResult>();
    final second = Completer<AuthoringRevision3DialogLocalizationReadResult>();
    final service = Revision3DialogLineEntryAuthoringService(
      loadContentIndex: () async => _contentIndex(includeDuplicate: true),
      readExactLocalization:
          ({
            required expectedProjectId,
            required expectedProjectRevision,
            required localizationId,
            required expectedLocalizationRevision,
            required expectedLocId,
          }) {
            return localizationId == _unusedLocalizationId
                ? first.future
                : second.future;
          },
      publishTechnicalPlan:
          ({
            required expectedProjectId,
            required expectedProjectRevision,
            required plan,
          }) async => throw StateError('selection test must not publish'),
    );
    await _openDialog(
      tester,
      service: service,
      initialMode: Revision3DialogLineEntryMode.reuseExact,
    );

    expect(find.text('Unused greeting (1)'), findsOneWidget);
    expect(find.text('Unused greeting (2)'), findsOneWidget);
    expect(find.text(_unusedLocalizationId), findsNothing);
    expect(find.text(_secondUnusedLocalizationId), findsNothing);
    expect(find.text(_unusedLocalizationLocId), findsNothing);
    expect(find.text(_secondUnusedLocalizationLocId), findsNothing);

    await tester.tap(find.text('Unused greeting (1)'));
    await tester.pump();
    expect(
      find.byKey(const Key('revision3-dialog-line-preview-loading')),
      findsOneWidget,
    );
    await tester.tap(find.text('Unused greeting (2)'));
    await tester.pump();

    second.complete(
      _localizationReadResult(
        expectedProjectId: revision3VoiceContentProjectId,
        expectedProjectRevision: 7,
        localizationId: _secondUnusedLocalizationId,
        expectedLocalizationRevision: 4,
        expectedLocId: _secondUnusedLocalizationLocId,
        locales: <Map<String, Object?>>[
          <String, Object?>{
            'locale': 'de',
            'preview': 'Second wins',
            'truncated': true,
            'has_nonempty_text': true,
          },
          <String, Object?>{
            'locale': 'en',
            'preview': 'Second English',
            'truncated': false,
            'has_nonempty_text': true,
          },
        ],
      ),
    );
    await tester.pumpAndSettle();
    expect(find.text('Second wins\u2026'), findsOneWidget);

    first.complete(
      _localizationReadResult(
        expectedProjectId: revision3VoiceContentProjectId,
        expectedProjectRevision: 7,
        localizationId: _unusedLocalizationId,
        expectedLocalizationRevision: 4,
        expectedLocId: _unusedLocalizationLocId,
        locales: <Map<String, Object?>>[
          <String, Object?>{
            'locale': 'de',
            'preview': 'Stale first result',
            'truncated': false,
            'has_nonempty_text': true,
          },
          <String, Object?>{
            'locale': 'en',
            'preview': 'Stale first English',
            'truncated': false,
            'has_nonempty_text': true,
          },
        ],
      ),
    );
    await tester.pumpAndSettle();
    expect(find.text('Second wins\u2026'), findsOneWidget);
    expect(find.text('Stale first result'), findsNothing);
  });

  testWidgets('manual locale input updates the suggested ChoiceChips', (
    tester,
  ) async {
    await _setSurface(tester, width: 1100);
    await _openDialog(tester, service: _successfulService());

    ChoiceChip chip(String locale) => tester.widget<ChoiceChip>(
      find.byKey(Key('revision3-dialog-line-locale-$locale')),
    );

    expect(chip('de').selected, isTrue);
    expect(chip('en').selected, isFalse);

    await tester.enterText(
      find.byKey(const Key('revision3-dialog-line-locale')),
      'en',
    );
    await tester.pump();

    expect(chip('de').selected, isFalse);
    expect(chip('en').selected, isTrue);
  });

  testWidgets('uses fullscreen below 700 pixels and a modal on wide screens', (
    tester,
  ) async {
    await _setSurface(tester, width: 640);
    final service = _successfulService();

    await _openDialog(tester, service: service, allowOpenVoiceNext: false);

    expect(
      find.byKey(const Key('revision3-dialog-line-fullscreen')),
      findsOneWidget,
    );
    expect(find.byKey(const Key('revision3-dialog-line-modal')), findsNothing);
    expect(
      find.byKey(const Key('revision3-dialog-line-editor')),
      findsOneWidget,
    );
    expect(
      find.byKey(const Key('revision3-dialog-line-cancel')),
      findsOneWidget,
    );
    expect(
      find.byKey(const Key('revision3-dialog-line-submit')),
      findsOneWidget,
    );

    await tester.tap(find.byKey(const Key('revision3-dialog-line-cancel')));
    await tester.pumpAndSettle();
    expect(
      find.byKey(const Key('revision3-dialog-line-fullscreen')),
      findsNothing,
    );
  });

  testWidgets('stale checkpoint fails closed before technical publication', (
    tester,
  ) async {
    await _setSurface(tester, width: 1100);
    var loads = 0;
    var publishCalls = 0;
    final service = Revision3DialogLineEntryAuthoringService(
      loadContentIndex: () async {
        loads++;
        return _contentIndex(revision: loads == 1 ? 7 : 8);
      },
      readExactLocalization: _successfulLocalizationRead,
      publishTechnicalPlan:
          ({
            required expectedProjectId,
            required expectedProjectRevision,
            required plan,
          }) async {
            publishCalls++;
            return _publication(
              projectId: expectedProjectId,
              revision: expectedProjectRevision + 1,
              plan: plan,
            );
          },
    );
    await _openDialog(tester, service: service);
    await _fillCreate(tester);
    await _submit(tester);

    expect(loads, 2);
    expect(publishCalls, 0);
    expect(find.text(_copy.stale), findsOneWidget);
    expect(
      find.byKey(const Key('revision3-dialog-line-success')),
      findsNothing,
    );
  });

  testWidgets('requires-reopen error locks further publication', (
    tester,
  ) async {
    await _setSurface(tester, width: 1100);
    final service = Revision3DialogLineEntryAuthoringService(
      loadContentIndex: () async => _contentIndex(),
      readExactLocalization: _successfulLocalizationRead,
      publishTechnicalPlan:
          ({
            required expectedProjectId,
            required expectedProjectRevision,
            required plan,
          }) async =>
              throw const Revision3DialogLineEntryRequiresReopenException(),
    );
    await _openDialog(tester, service: service);
    await _fillCreate(tester);
    await _submit(tester);

    expect(find.text(_copy.requiresReopen), findsOneWidget);
    expect(_submitButton(tester).onPressed, isNull);
    expect(
      find.byKey(const Key('revision3-dialog-line-success')),
      findsNothing,
    );
  });

  testWidgets('maps native semantic errors to author-facing copy', (
    tester,
  ) async {
    await _setSurface(tester, width: 1100);
    final service = Revision3DialogLineEntryAuthoringService(
      loadContentIndex: () async => _contentIndex(),
      readExactLocalization: _successfulLocalizationRead,
      publishTechnicalPlan:
          ({
            required expectedProjectId,
            required expectedProjectRevision,
            required plan,
          }) async => throw const ModFfiException(
            command: 'authoring_store_prepare_revision3_dialog_line_v1',
            code: 'AUTHORING_REVISION3_DIALOG_LOCALIZATION_CONFLICT',
            message: 'technical localization conflict',
          ),
    );
    await _openDialog(tester, service: service);
    await _fillCreate(tester);
    await _submit(tester);

    expect(find.text(_copy.invalidInput), findsOneWidget);
    expect(find.textContaining('AUTHORING_REVISION3_DIALOG'), findsNothing);
    expect(find.textContaining('technical localization'), findsNothing);
    expect(
      find.byKey(const Key('revision3-dialog-line-success')),
      findsNothing,
    );
  });

  testWidgets('back, close, and cancel stay blocked during publication', (
    tester,
  ) async {
    await _setSurface(tester, width: 1100);
    final pending = Completer<Revision3DialogLineEntryPublication>();
    Revision3DialogLineEntryTechnicalPlan? publishedPlan;
    final service = Revision3DialogLineEntryAuthoringService(
      loadContentIndex: () async => _contentIndex(),
      readExactLocalization: _successfulLocalizationRead,
      publishTechnicalPlan:
          ({
            required expectedProjectId,
            required expectedProjectRevision,
            required plan,
          }) {
            publishedPlan = plan;
            return pending.future;
          },
    );
    await _openDialog(tester, service: service);
    await _fillCreate(tester);
    await tester.tap(find.byKey(const Key('revision3-dialog-line-submit')));
    await tester.pump();
    await tester.pump();

    expect(find.text(_copy.saving), findsOneWidget);
    expect(
      tester
          .widget<IconButton>(
            find.byKey(const Key('revision3-dialog-line-close')),
          )
          .onPressed,
      isNull,
    );
    expect(
      tester
          .widget<TextButton>(
            find.byKey(const Key('revision3-dialog-line-cancel')),
          )
          .onPressed,
      isNull,
    );

    await tester.binding.handlePopRoute();
    await tester.pump();
    expect(
      find.byKey(const Key('revision3-dialog-line-modal')),
      findsOneWidget,
    );

    final plan = publishedPlan!;
    pending.complete(
      _publication(
        projectId: revision3VoiceContentProjectId,
        revision: 8,
        plan: plan,
      ),
    );
    await tester.pumpAndSettle();
    expect(
      find.byKey(const Key('revision3-dialog-line-success')),
      findsOneWidget,
    );
    await tester.tap(find.byKey(const Key('revision3-dialog-line-done')));
    await tester.pumpAndSettle();
  });
}

Revision3DialogLineEntryAuthoringService _successfulService() =>
    Revision3DialogLineEntryAuthoringService(
      loadContentIndex: () async => _contentIndex(),
      readExactLocalization: _successfulLocalizationRead,
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

Future<void> _openDialog(
  WidgetTester tester, {
  required Revision3DialogLineEntryAuthoringService service,
  Revision3DialogLineEntryDialogCopy copy = _copy,
  Revision3DialogLineEntryMode initialMode =
      Revision3DialogLineEntryMode.create,
  bool allowOpenVoiceNext = true,
  ValueChanged<Revision3DialogLineEntryDialogResult?>? onResult,
}) async {
  await tester.pumpWidget(
    MaterialApp(
      home: Scaffold(
        body: Builder(
          builder: (context) => Center(
            child: FilledButton(
              key: const Key('open-dialog-line-entry'),
              onPressed: () async {
                final result =
                    await showDialog<Revision3DialogLineEntryDialogResult>(
                      context: context,
                      builder: (_) => Revision3DialogLineEntryDialog(
                        service: service,
                        copy: copy,
                        initialMode: initialMode,
                        allowOpenVoiceNext: allowOpenVoiceNext,
                      ),
                    );
                onResult?.call(result);
              },
              child: const Text('Open'),
            ),
          ),
        ),
      ),
    ),
  );
  await tester.tap(find.byKey(const Key('open-dialog-line-entry')));
  await tester.pumpAndSettle();
}

Future<void> _fillCreate(
  WidgetTester tester, {
  String name = 'Mine warning',
  String speaker = '',
  String text = 'Turn around.',
}) async {
  await tester.enterText(
    find.byKey(const Key('revision3-dialog-line-name')),
    name,
  );
  if (speaker.isNotEmpty) {
    await tester.enterText(
      find.byKey(const Key('revision3-dialog-line-speaker')),
      speaker,
    );
  }
  await tester.enterText(
    find.byKey(const Key('revision3-dialog-line-text')),
    text,
  );
  await tester.pump();
}

Future<void> _submit(WidgetTester tester) async {
  await tester.tap(find.byKey(const Key('revision3-dialog-line-submit')));
  await tester.pumpAndSettle();
}

FilledButton _submitButton(WidgetTester tester) => tester.widget<FilledButton>(
  find.byKey(const Key('revision3-dialog-line-submit')),
);

Future<void> _setSurface(
  WidgetTester tester, {
  required double width,
  double height = 900,
}) async {
  tester.view.devicePixelRatio = 1;
  tester.view.physicalSize = Size(width, height);
  addTearDown(() {
    tester.view.resetPhysicalSize();
    tester.view.resetDevicePixelRatio();
  });
}

AuthoringWorkingHead _dialogReadHead() =>
    AuthoringWorkingHead.fromCanonicalJson(
      jsonEncode(<String, Object?>{
        'store_format': 1,
        'snapshot': <String, Object?>{'byte_len': 321, 'sha256': 'a' * 64},
      }),
    );

Future<AuthoringRevision3DialogLocalizationReadResult>
_successfulLocalizationRead({
  required String expectedProjectId,
  required int expectedProjectRevision,
  required String localizationId,
  required int expectedLocalizationRevision,
  required String expectedLocId,
}) async => _localizationReadResult(
  expectedProjectId: expectedProjectId,
  expectedProjectRevision: expectedProjectRevision,
  localizationId: localizationId,
  expectedLocalizationRevision: expectedLocalizationRevision,
  expectedLocId: expectedLocId,
);

AuthoringRevision3DialogLocalizationReadResult _localizationReadResult({
  required String expectedProjectId,
  required int expectedProjectRevision,
  required String localizationId,
  required int expectedLocalizationRevision,
  required String expectedLocId,
  String? actualProjectId,
  int? actualProjectRevision,
  String? actualLocalizationId,
  int? actualLocalizationRevision,
  String? actualLocId,
  List<Map<String, Object?>>? locales,
}) {
  final head = _dialogReadHead();
  final request = AuthoringRevision3DialogLocalizationReadRequestV1(
    expectedHead: head,
    localizationId: localizationId,
    expectedLocalizationRevision: expectedLocalizationRevision,
    expectedLocId: expectedLocId,
  );
  return AuthoringRevision3DialogLocalizationReadResult.fromJson(
    <String, Object?>{
      'ok': true,
      'outcome': 'read_only',
      'head_json': head.canonicalJson,
      'project_id': actualProjectId ?? expectedProjectId,
      'project_revision': actualProjectRevision ?? expectedProjectRevision,
      'localization_id': actualLocalizationId ?? localizationId,
      'localization_revision':
          actualLocalizationRevision ?? expectedLocalizationRevision,
      'loc_id': actualLocId ?? expectedLocId,
      'locales':
          locales ??
          <Object?>[
            <String, Object?>{
              'locale': 'de',
              'preview': 'Willkommen in der Mine.',
              'truncated': false,
              'has_nonempty_text': true,
            },
            <String, Object?>{
              'locale': 'en',
              'preview': 'Welcome to the mine.',
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

Revision3DialogLineEntryPublication _publication({
  required String projectId,
  required int revision,
  required Revision3DialogLineEntryTechnicalPlan plan,
}) => Revision3DialogLineEntryPublication(
  projectId: projectId,
  projectRevision: revision,
  lineId: plan.lineId,
  localizationId: plan.localization.localizationId,
  localizationAction:
      plan.localization is AuthoringRevision3DialogLocalizationCreateIntentV1
      ? AuthoringRevision3DialogLocalizationAction.created
      : AuthoringRevision3DialogLocalizationAction.reusedExact,
  voiceSlotId: plan.voiceSlot?.slotId,
  locale: plan.locale,
);

Revision3ContentIndex _contentIndex({
  int revision = 7,
  bool includeDuplicate = false,
}) {
  final json = revision3VoiceContentIndexJsonFixture(revision: revision);
  final entities = (json['entities']! as List<Object?>)
      .map((value) => Map<String, Object?>.from(value! as Map))
      .toList();
  final used = entities.singleWhere(
    (entity) => entity['id'] == revision3VoiceContentLocalizationId,
  );
  used['display_name'] = 'Used Asghan text';
  used['revision'] = 2;
  final usedSummary = Map<String, Object?>.from(used['summary']! as Map);
  final usedData = Map<String, Object?>.from(usedSummary['data']! as Map);
  usedData['locales'] = <Object?>['de'];
  usedSummary['data'] = usedData;
  used['summary'] = usedSummary;

  entities.add(<String, Object?>{
    'id': _unusedLocalizationId,
    'kind': 'localization_entry',
    'display_name': 'Unused greeting',
    'revision': 4,
    'origin': <String, Object?>{
      'type': 'new',
      'authored_runtime_id': _unusedLocalizationLocId,
    },
    'summary': <String, Object?>{
      'kind': 'localization_entry',
      'data': <String, Object?>{
        'loc_id': _unusedLocalizationLocId,
        'locales': <Object?>['de', 'en'],
      },
    },
    'references': <Object?>[],
    'asset_references': <Object?>[],
  });
  if (includeDuplicate) {
    entities.add(<String, Object?>{
      'id': _secondUnusedLocalizationId,
      'kind': 'localization_entry',
      'display_name': 'Unused greeting',
      'revision': 4,
      'origin': <String, Object?>{
        'type': 'new',
        'authored_runtime_id': _secondUnusedLocalizationLocId,
      },
      'summary': <String, Object?>{
        'kind': 'localization_entry',
        'data': <String, Object?>{
          'loc_id': _secondUnusedLocalizationLocId,
          'locales': <Object?>['de', 'en'],
        },
      },
      'references': <Object?>[],
      'asset_references': <Object?>[],
    });
  }
  entities.sort(
    (left, right) => (left['id']! as String).compareTo(right['id']! as String),
  );
  json['entities'] = entities;
  final counts = Map<String, Object?>.from(json['entity_counts']! as Map);
  counts['localization_entry'] = includeDuplicate ? 3 : 2;
  json['entity_counts'] = counts;
  return Revision3ContentIndex.fromJsonObject(json);
}
