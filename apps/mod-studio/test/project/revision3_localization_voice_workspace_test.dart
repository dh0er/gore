import 'dart:async';
import 'dart:convert';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/core/mod_ffi.dart';
import 'package:gore_mod/project/revision3_content_index.dart';
import 'package:gore_mod/project/revision3_dialog_localization_authoring.dart';
import 'package:gore_mod/project/revision3_localization_voice_workspace.dart';

import '../support/revision3_voice_content_fixture.dart';

const _projectId = revision3VoiceContentProjectId;
const _localizationId = revision3VoiceContentLocalizationId;
const _secondLocalizationId = '77777777777777777777777777777777';
const _locId = 'GORE_ASGHAN_WARNING';
const _secondLocId = 'GORE_VIPER_GREETING';
const _lineId = revision3VoiceContentLineId;
const _secondLineId = revision3VoiceContentDuplicateLineId;
const _copy = Revision3LocalizationVoiceWorkspaceCopy.english();

void main() {
  testWidgets(
    'wide workspace loads exact full text, hides identities, and publishes edits',
    (tester) async {
      await _setSurface(tester, width: 1200);
      final exactText = List.filled(220, 'Übergrößenträger 🐉').join(' ');
      var catalogLoads = 0;
      var seedLoads = 0;
      var publishCalls = 0;
      Revision3DialogLocalizationEditTechnicalPlan? publishedPlan;
      Revision3DialogLocalizationEditPublication? published;
      final index = _contentIndex(
        displayName: 'Mine warning',
        locales: const <String>['de', 'en'],
      );
      final service = Revision3DialogLocalizationEditAuthoringService(
        loadContentIndex: () async {
          catalogLoads++;
          return index;
        },
        loadExactSeed:
            ({
              required expectedProjectId,
              required expectedProjectRevision,
              required localizationId,
              required expectedLocalizationRevision,
              required expectedLocId,
            }) async {
              seedLoads++;
              return _exactSeed(
                projectId: expectedProjectId,
                projectRevision: expectedProjectRevision,
                localizationId: localizationId,
                localizationRevision: expectedLocalizationRevision,
                locId: expectedLocId,
                texts: <String, String>{
                  'de': exactText,
                  'en': 'Stop right there.',
                },
              );
            },
        publishTechnicalPlan:
            ({
              required expectedProjectId,
              required expectedProjectRevision,
              required plan,
            }) async {
              publishCalls++;
              publishedPlan = plan;
              return _publication(
                projectId: expectedProjectId,
                projectRevision: expectedProjectRevision + 1,
                localizationId: plan.localizationId,
                localizationRevision: plan.expectedLocalizationRevision + 1,
              );
            },
      );

      await _pumpWorkspace(
        tester,
        service: service,
        onPublished: (value) => published = value,
      );

      expect(
        find.byKey(const Key('revision3-localization-text-browser')),
        findsOneWidget,
      );
      expect(
        find.byKey(const Key('revision3-localization-text-editor')),
        findsOneWidget,
      );
      expect(catalogLoads, 1);
      expect(seedLoads, 1);
      expect(_textField(tester, 'de').controller!.text, exactText);
      expect(exactText.length, greaterThan(512));
      expect(find.text(_localizationId), findsNothing);
      expect(find.text(_locId), findsNothing);
      expect(find.textContaining(_localizationId), findsNothing);
      expect(find.textContaining(_locId), findsNothing);
      expect(find.text('Mine entrance question'), findsOneWidget);
      expect(find.text('${_copy.speakerLabel}: Asghan'), findsOneWidget);
      expect(
        tester
            .widget<FilledButton>(
              find.byKey(const Key('revision3-localization-save')),
            )
            .onPressed,
        isNull,
      );
      await tester.tap(find.byKey(const Key('revision3-localization-save')));
      await tester.pump();
      expect(publishCalls, 0, reason: 'a disabled save must be a no-op');

      final replacement = '$exactText\nGENAU GESPEICHERT';
      await tester.enterText(
        find.byKey(const Key('revision3-localization-text-de')),
        replacement,
      );
      await tester.pump();
      await tester.tap(find.byKey(const Key('revision3-localization-save')));
      await tester.pump();
      await tester.pump();

      expect(catalogLoads, 2, reason: 'save must reopen the exact catalog');
      expect(seedLoads, 2, reason: 'save must reopen the exact full seed');
      expect(publishCalls, 1);
      expect(publishedPlan!.texts['de'], replacement);
      expect(published?.projectRevision, 8);
      expect(find.text(_copy.savedLabel), findsOneWidget);
      expect(
        tester
            .widget<FilledButton>(
              find.byKey(const Key('revision3-localization-save')),
            )
            .onPressed,
        isNull,
      );
    },
  );

  testWidgets(
    'context actions carry the explicitly selected line and language',
    (tester) async {
      await _setSurface(tester, width: 1200);
      final calls = <String>[];
      final service = _serviceForIndex(
        _contentIndex(
          displayName: 'Mine warning',
          locales: const <String>['de', 'en'],
        ),
        seedFor:
            ({
              required projectId,
              required projectRevision,
              required localizationId,
              required localizationRevision,
              required locId,
            }) => _exactSeed(
              projectId: projectId,
              projectRevision: projectRevision,
              localizationId: localizationId,
              localizationRevision: localizationRevision,
              locId: locId,
              backlinks: _backlinks(shared: true, identicalVisibleLines: true),
            ),
      );
      Future<void> record(
        String action, {
        required String initialLineId,
        required String initialLocale,
      }) async => calls.add('$action:$initialLineId:$initialLocale');

      await _pumpWorkspace(
        tester,
        service: service,
        onAddVoiceTakeFor: ({required initialLineId, required initialLocale}) =>
            record(
              'add',
              initialLineId: initialLineId,
              initialLocale: initialLocale,
            ),
        onManageVoiceTakesFor:
            ({required initialLineId, required initialLocale}) => record(
              'manage',
              initialLineId: initialLineId,
              initialLocale: initialLocale,
            ),
        onResolveVoiceTargetFor:
            ({required initialLineId, required initialLocale}) => record(
              'resolve',
              initialLineId: initialLineId,
              initialLocale: initialLocale,
            ),
      );

      expect(
        find.byKey(const Key('revision3-localization-voice-select-line-hint')),
        findsOneWidget,
      );
      expect(find.text('Mine entrance question · 1 of 2'), findsOneWidget);
      expect(find.text('Mine entrance question · 2 of 2'), findsOneWidget);
      expect(find.textContaining(_lineId), findsNothing);
      expect(find.textContaining(_secondLineId), findsNothing);
      await tester.tap(
        find.byKey(
          const ValueKey('revision3-localization-voice-line-$_secondLineId'),
        ),
      );
      await tester.pump();
      await tester.tap(
        find.byKey(const ValueKey('revision3-localization-voice-locale-en')),
      );
      await tester.pump();
      await tester.tap(
        find.byKey(const Key('revision3-localization-context-add-voice')),
      );
      await tester.pump();
      expect(calls, <String>['add:$_secondLineId:en']);
      expect(
        tester
            .widget<OutlinedButton>(
              find.byKey(
                const Key('revision3-localization-context-manage-voice'),
              ),
            )
            .onPressed,
        isNull,
      );

      await tester.tap(
        find.byKey(
          const ValueKey('revision3-localization-voice-line-$_lineId'),
        ),
      );
      await tester.pump();
      await tester.tap(
        find.byKey(const ValueKey('revision3-localization-voice-locale-de')),
      );
      await tester.pump();
      await tester.tap(
        find.byKey(const Key('revision3-localization-context-manage-voice')),
      );
      await tester.pump();
      await tester.tap(
        find.byKey(const Key('revision3-localization-context-resolve-voice')),
      );
      await tester.pump();
      expect(calls, <String>[
        'add:$_secondLineId:en',
        'manage:$_lineId:de',
        'resolve:$_lineId:de',
      ]);
    },
  );

  testWidgets('compact browser opens the first item and discard resets it', (
    tester,
  ) async {
    await _setSurface(tester, width: 360, height: 900);
    final service = _successfulService();
    await _pumpWorkspace(tester, service: service);

    expect(
      find.byKey(const Key('revision3-localization-text-browser')),
      findsOneWidget,
    );
    expect(
      find.byKey(const Key('revision3-localization-text-editor')),
      findsNothing,
    );

    await tester.tap(find.text('Mine warning'));
    await tester.pumpAndSettle();
    expect(
      find.byKey(const Key('revision3-localization-text-editor')),
      findsOneWidget,
    );
    expect(_textField(tester, 'de').controller!.text, 'Bleib stehen!');

    await tester.enterText(
      find.byKey(const Key('revision3-localization-text-de')),
      'Ungespeicherter Test',
    );
    await tester.pump();
    await tester.tap(
      find.byKey(const Key('revision3-localization-editor-back')),
    );
    await tester.pumpAndSettle();
    expect(find.text(_copy.unsavedTitle), findsOneWidget);

    await tester.tap(find.text(_copy.keepEditingLabel));
    await tester.pumpAndSettle();
    expect(
      find.byKey(const Key('revision3-localization-text-editor')),
      findsOneWidget,
    );
    expect(_textField(tester, 'de').controller!.text, 'Ungespeicherter Test');

    await tester.tap(
      find.byKey(const Key('revision3-localization-editor-back')),
    );
    await tester.pumpAndSettle();
    await tester.tap(find.text(_copy.discardLabel));
    await tester.pumpAndSettle();
    expect(
      find.byKey(const Key('revision3-localization-text-browser')),
      findsOneWidget,
    );

    await tester.tap(find.text('Mine warning'));
    await tester.pumpAndSettle();
    expect(_textField(tester, 'de').controller!.text, 'Bleib stehen!');
    expect(
      tester
          .widget<FilledButton>(
            find.byKey(const Key('revision3-localization-save')),
          )
          .onPressed,
      isNull,
    );
  });

  testWidgets(
    'compact header keeps every configured action visibly reachable',
    (tester) async {
      await _setSurface(tester, width: 360, height: 900);
      var newLineCalls = 0;
      var addVoiceCalls = 0;
      var manageVoiceCalls = 0;
      var resolveVoiceCalls = 0;
      await _pumpWorkspace(
        tester,
        service: _successfulService(),
        onCreateDialogLine: () => newLineCalls++,
        onAddVoiceTake: () => addVoiceCalls++,
        onManageVoiceTakes: () => manageVoiceCalls++,
        onResolveVoiceTarget: () => resolveVoiceCalls++,
      );

      final newLine = find
          .byKey(const Key('revision3-localization-new-line'))
          .hitTestable();
      final moreActions = find
          .byKey(const Key('revision3-localization-more-actions'))
          .hitTestable();
      expect(newLine, findsOneWidget);
      expect(moreActions, findsOneWidget);
      expect(tester.takeException(), isNull);

      await tester.tap(newLine);
      await tester.pump();
      expect(newLineCalls, 1);

      await tester.tap(moreActions);
      await tester.pumpAndSettle();
      expect(
        find.byKey(const Key('revision3-localization-add-voice')),
        findsOneWidget,
      );
      expect(
        find.byKey(const Key('revision3-localization-manage-voice')),
        findsOneWidget,
      );
      expect(
        find.byKey(const Key('revision3-localization-resolve-voice')),
        findsOneWidget,
      );
      expect(tester.takeException(), isNull);

      await tester.tap(
        find.byKey(const Key('revision3-localization-add-voice')),
      );
      await tester.pumpAndSettle();
      expect(addVoiceCalls, 1);

      await tester.tap(moreActions);
      await tester.pumpAndSettle();
      await tester.tap(
        find.byKey(const Key('revision3-localization-manage-voice')),
      );
      await tester.pumpAndSettle();
      expect(manageVoiceCalls, 1);

      await tester.tap(moreActions);
      await tester.pumpAndSettle();
      await tester.tap(
        find.byKey(const Key('revision3-localization-resolve-voice')),
      );
      await tester.pumpAndSettle();
      expect(resolveVoiceCalls, 1);
      expect(tester.takeException(), isNull);
    },
  );

  testWidgets(
    'dirty notifications follow edits, reverts, saves, and disposal',
    (tester) async {
      await _setSurface(tester, width: 1200);
      final dirtyChanges = <bool>[];
      await _pumpWorkspace(
        tester,
        service: _successfulService(),
        onDirtyChanged: dirtyChanges.add,
      );

      await tester.enterText(
        find.byKey(const Key('revision3-localization-text-de')),
        'Ungespeichert',
      );
      await tester.pump();
      expect(dirtyChanges, const <bool>[true]);

      await tester.enterText(
        find.byKey(const Key('revision3-localization-text-de')),
        'Bleib stehen!',
      );
      await tester.pump();
      expect(dirtyChanges, const <bool>[true, false]);

      await tester.enterText(
        find.byKey(const Key('revision3-localization-text-de')),
        'Wird gespeichert',
      );
      await tester.pump();
      expect(dirtyChanges, const <bool>[true, false, true]);
      await tester.tap(find.byKey(const Key('revision3-localization-save')));
      await tester.pumpAndSettle();
      expect(dirtyChanges, const <bool>[true, false, true, false]);

      await tester.enterText(
        find.byKey(const Key('revision3-localization-text-de')),
        'Vor dem Schließen',
      );
      await tester.pump();
      expect(dirtyChanges, const <bool>[true, false, true, false, true]);

      await tester.pumpWidget(const SizedBox.shrink());
      await tester.pump();
      expect(dirtyChanges, const <bool>[true, false, true, false, true, false]);
    },
  );

  testWidgets('selection keeps or discards dirty text before changing seed', (
    tester,
  ) async {
    await _setSurface(tester, width: 1200);
    final index = _contentIndex(
      displayName: 'Alpha warning',
      locales: const <String>['de', 'en'],
      secondDisplayName: 'Beta greeting',
    );
    final service = _serviceForIndex(
      index,
      seedFor:
          ({
            required localizationId,
            required projectId,
            required projectRevision,
            required localizationRevision,
            required locId,
          }) => _exactSeed(
            projectId: projectId,
            projectRevision: projectRevision,
            localizationId: localizationId,
            localizationRevision: localizationRevision,
            locId: locId,
            texts: localizationId == _localizationId
                ? const <String, String>{
                    'de': 'Alpha Deutsch',
                    'en': 'Alpha English',
                  }
                : const <String, String>{
                    'de': 'Beta Deutsch',
                    'en': 'Beta English',
                  },
            voiceSlots: localizationId == _localizationId
                ? const <String>{'de'}
                : const <String>{},
            backlinks: localizationId == _localizationId
                ? _backlinks()
                : const <Map<String, Object?>>[],
          ),
    );
    await _pumpWorkspace(tester, service: service);

    await tester.enterText(
      find.byKey(const Key('revision3-localization-text-de')),
      'Alpha dirty',
    );
    await tester.pump();
    await tester.tap(find.text('Beta greeting'));
    await tester.pumpAndSettle();
    await tester.tap(find.text(_copy.keepEditingLabel));
    await tester.pumpAndSettle();
    expect(_textField(tester, 'de').controller!.text, 'Alpha dirty');
    expect(
      tester
          .widget<ListTile>(find.widgetWithText(ListTile, 'Alpha warning'))
          .selected,
      isTrue,
    );

    await tester.tap(find.text('Beta greeting'));
    await tester.pumpAndSettle();
    await tester.tap(find.text(_copy.discardLabel));
    await tester.pumpAndSettle();
    expect(_textField(tester, 'de').controller!.text, 'Beta Deutsch');
    expect(find.text('Alpha dirty'), findsNothing);
  });

  testWidgets(
    'adds canonical locale, rejects duplicate, removes and publishes',
    (tester) async {
      await _setSurface(tester, width: 1200);
      Revision3DialogLocalizationEditTechnicalPlan? plan;
      final index = _contentIndex(
        displayName: 'Mine warning',
        locales: const <String>['de', 'en'],
        existingDeSlot: false,
      );
      final service = _serviceForIndex(
        index,
        seedFor:
            ({
              required localizationId,
              required projectId,
              required projectRevision,
              required localizationRevision,
              required locId,
            }) => _exactSeed(
              projectId: projectId,
              projectRevision: projectRevision,
              localizationId: localizationId,
              localizationRevision: localizationRevision,
              locId: locId,
              texts: const <String, String>{
                'de': 'Bleib stehen!',
                'en': 'Stop right there!',
              },
              voiceSlots: const <String>{},
              backlinks: _backlinks(voiceSlotLocales: const <String>[]),
            ),
        publish:
            ({
              required expectedProjectId,
              required expectedProjectRevision,
              required technicalPlan,
            }) async {
              plan = technicalPlan;
              return _publication(
                projectId: expectedProjectId,
                projectRevision: expectedProjectRevision + 1,
                localizationId: technicalPlan.localizationId,
                localizationRevision:
                    technicalPlan.expectedLocalizationRevision + 1,
                addedLocales: const <String>['pt-BR'],
                removedLocales: const <String>['en'],
              );
            },
      );
      await _pumpWorkspace(tester, service: service);

      await tester.tap(
        find.byKey(const Key('revision3-localization-add-language')),
      );
      await tester.pumpAndSettle();
      await tester.enterText(
        find.byKey(const Key('revision3-localization-new-locale-code')),
        'DE',
      );
      await tester.enterText(
        find.byKey(const Key('revision3-localization-new-locale-text')),
        'Nicht kanonisch',
      );
      await tester.pump();
      expect(_dialogAddButton(tester).onPressed, isNull);

      await tester.enterText(
        find.byKey(const Key('revision3-localization-new-locale-code')),
        'de',
      );
      await tester.pump();
      expect(find.text(_copy.languageExistsMessage), findsOneWidget);
      expect(_dialogAddButton(tester).onPressed, isNull);

      await tester.enterText(
        find.byKey(const Key('revision3-localization-new-locale-code')),
        'pt-br',
      );
      await tester.pump();
      expect(_dialogAddButton(tester).onPressed, isNull);
      await tester.enterText(
        find.byKey(const Key('revision3-localization-new-locale-code')),
        'pt-BR',
      );
      await tester.enterText(
        find.byKey(const Key('revision3-localization-new-locale-text')),
        'Pare agora!',
      );
      await tester.pump();
      expect(_dialogAddButton(tester).onPressed, isNotNull);
      await tester.tap(find.text(_copy.addLabel));
      await tester.pumpAndSettle();
      expect(
        find.byKey(const Key('revision3-localization-text-pt-BR')),
        findsOneWidget,
      );

      await tester.ensureVisible(
        find.byKey(const ValueKey('revision3-localization-locale-en')),
      );
      await tester.tap(_removeButton('en'));
      await tester.pump();
      expect(
        find.byKey(const Key('revision3-localization-text-en')),
        findsNothing,
      );

      await tester.tap(find.byKey(const Key('revision3-localization-save')));
      await tester.pump();
      await tester.pump();
      expect(plan!.texts, <String, String>{
        'de': 'Bleib stehen!',
        'pt-BR': 'Pare agora!',
      });
    },
  );

  testWidgets(
    'candidate transcript is locked, VoiceSlot removal is blocked, and sharing is clear',
    (tester) async {
      await _setSurface(tester, width: 1200);
      final index = _contentIndex(
        displayName: 'Shared warning',
        locales: const <String>['de', 'en'],
        existingDeSlot: true,
        duplicateLine: true,
      );
      final service = _serviceForIndex(
        index,
        seedFor:
            ({
              required localizationId,
              required projectId,
              required projectRevision,
              required localizationRevision,
              required locId,
            }) => _exactSeed(
              projectId: projectId,
              projectRevision: projectRevision,
              localizationId: localizationId,
              localizationRevision: localizationRevision,
              locId: locId,
              texts: const <String, String>{
                'de': 'Aufgenommener Text',
                'en': 'Empty-slot transcript',
              },
              voiceSlots: const <String>{'de', 'en'},
              candidateLocales: const <String>{'de'},
              backlinks: _backlinks(
                shared: true,
                voiceSlotLocales: const <String>['de', 'en'],
              ),
            ),
      );
      await _pumpWorkspace(tester, service: service);

      expect(find.text(_copy.sharedTextNotice), findsOneWidget);
      expect(find.text('Mine entrance question'), findsOneWidget);
      expect(find.text('Mine entrance question (copy)'), findsOneWidget);
      expect(_textField(tester, 'de').readOnly, isTrue);
      expect(_textField(tester, 'en').readOnly, isFalse);
      expect(_localeRemoveIcon(tester, 'de').onPressed, isNull);
      expect(_localeRemoveIcon(tester, 'en').onPressed, isNull);
      expect(
        _localeRemoveIcon(tester, 'en').tooltip,
        _copy.voiceSlotRemovalLockedLabel,
      );
      expect(find.text(_copy.voiceLockedLabel), findsOneWidget);
      expect(find.text(_copy.voiceSlotRemovalLockedLabel), findsOneWidget);
    },
  );

  testWidgets(
    'the last language uses its honest minimum-language lock reason',
    (tester) async {
      await _setSurface(tester, width: 1100);
      final index = _contentIndex(
        displayName: 'Only German',
        locales: const <String>['de'],
        existingDeSlot: false,
      );
      final service = _serviceForIndex(
        index,
        seedFor:
            ({
              required projectId,
              required projectRevision,
              required localizationId,
              required localizationRevision,
              required locId,
            }) => _exactSeed(
              projectId: projectId,
              projectRevision: projectRevision,
              localizationId: localizationId,
              localizationRevision: localizationRevision,
              locId: locId,
              texts: const <String, String>{'de': 'Nur ein Text'},
              voiceSlots: const <String>{},
              backlinks: _backlinks(voiceSlotLocales: const <String>[]),
            ),
      );
      await _pumpWorkspace(tester, service: service);

      final remove = _localeRemoveIcon(tester, 'de');
      expect(remove.onPressed, isNull);
      expect(remove.tooltip, _copy.minimumLanguageLockedLabel);
      expect(remove.tooltip, isNot(_copy.voiceSlotRemovalLockedLabel));
    },
  );

  testWidgets('empty catalog shows a useful creation state', (tester) async {
    await _setSurface(tester, width: 1100);
    var seedLoads = 0;
    final service = Revision3DialogLocalizationEditAuthoringService(
      loadContentIndex: () async => _contentIndex(
        displayName: 'Hidden empty text',
        locales: const <String>[],
      ),
      loadExactSeed:
          ({
            required expectedProjectId,
            required expectedProjectRevision,
            required localizationId,
            required expectedLocalizationRevision,
            required expectedLocId,
          }) async {
            seedLoads++;
            throw StateError('empty catalog must not load a seed');
          },
      publishTechnicalPlan:
          ({
            required expectedProjectId,
            required expectedProjectRevision,
            required plan,
          }) async => throw StateError('empty catalog must not publish'),
    );
    await _pumpWorkspace(tester, service: service);

    expect(find.text(_copy.emptyTitle), findsOneWidget);
    expect(find.text(_copy.emptyDescription), findsOneWidget);
    expect(seedLoads, 0);
  });

  testWidgets('typed reopen catalog failure stays author-facing', (
    tester,
  ) async {
    await _setSurface(tester, width: 1100);
    final reopen = Revision3DialogLocalizationEditAuthoringService(
      loadContentIndex: () async =>
          throw const Revision3ContentRequiresReopenException(),
      loadExactSeed: _unexpectedSeedLoad,
      publishTechnicalPlan: _unexpectedPublication,
    );
    await _pumpWorkspace(tester, service: reopen);
    expect(find.text(_copy.reopenMessage), findsOneWidget);
    expect(find.text(_copy.retryLabel), findsNothing);
    expect(find.textContaining('RequiresReopenException'), findsNothing);
  });

  testWidgets('typed stale seed failure stays author-facing', (tester) async {
    await _setSurface(tester, width: 1100);
    final stale = Revision3DialogLocalizationEditAuthoringService(
      loadContentIndex: () async => _contentIndex(
        displayName: 'Mine warning',
        locales: const <String>['de', 'en'],
      ),
      loadExactSeed:
          ({
            required expectedProjectId,
            required expectedProjectRevision,
            required localizationId,
            required expectedLocalizationRevision,
            required expectedLocId,
          }) async =>
              throw const Revision3DialogLocalizationEditStaleCheckpointException(),
      publishTechnicalPlan: _unexpectedPublication,
    );
    await _pumpWorkspace(tester, service: stale);
    expect(find.text(_copy.staleMessage), findsOneWidget);
    expect(find.text(_copy.refreshLabel), findsOneWidget);
    expect(find.text(_copy.retryLabel), findsNothing);
    expect(find.textContaining('CheckpointException'), findsNothing);
  });

  testWidgets('stale seed refresh reopens the catalog before reading again', (
    tester,
  ) async {
    await _setSurface(tester, width: 1100);
    var catalogLoads = 0;
    var seedLoads = 0;
    final index = _contentIndex(
      displayName: 'Mine warning',
      locales: const <String>['de', 'en'],
    );
    final service = Revision3DialogLocalizationEditAuthoringService(
      loadContentIndex: () async {
        catalogLoads++;
        return index;
      },
      loadExactSeed:
          ({
            required expectedProjectId,
            required expectedProjectRevision,
            required localizationId,
            required expectedLocalizationRevision,
            required expectedLocId,
          }) async {
            seedLoads++;
            if (seedLoads == 1) {
              throw const Revision3DialogLocalizationEditStaleCheckpointException();
            }
            return _exactSeed(
              projectId: expectedProjectId,
              projectRevision: expectedProjectRevision,
              localizationId: localizationId,
              localizationRevision: expectedLocalizationRevision,
              locId: expectedLocId,
            );
          },
      publishTechnicalPlan: _unexpectedPublication,
    );
    await _pumpWorkspace(tester, service: service);

    expect(catalogLoads, 1);
    expect(seedLoads, 1);
    expect(find.text(_copy.staleMessage), findsOneWidget);
    expect(
      find.widgetWithText(FilledButton, _copy.refreshLabel),
      findsOneWidget,
    );
    expect(find.text(_copy.retryLabel), findsNothing);

    await tester.tap(find.widgetWithText(FilledButton, _copy.refreshLabel));
    await tester.pumpAndSettle();

    expect(catalogLoads, 2, reason: 'stale retry must reopen the catalog');
    expect(seedLoads, 2);
    expect(
      find.byKey(const Key('revision3-localization-text-editor')),
      findsOneWidget,
    );
    expect(_textField(tester, 'de').controller!.text, 'Bleib stehen!');
  });

  testWidgets('compact seed failure can return to the text catalog', (
    tester,
  ) async {
    await _setSurface(tester, width: 480, height: 900);
    final service = Revision3DialogLocalizationEditAuthoringService(
      loadContentIndex: () async => _contentIndex(
        displayName: 'Mine warning',
        locales: const <String>['de', 'en'],
      ),
      loadExactSeed:
          ({
            required expectedProjectId,
            required expectedProjectRevision,
            required localizationId,
            required expectedLocalizationRevision,
            required expectedLocId,
          }) async =>
              throw const Revision3DialogLocalizationEditStaleCheckpointException(),
      publishTechnicalPlan: _unexpectedPublication,
    );
    await _pumpWorkspace(tester, service: service);
    expect(
      find.byKey(const Key('revision3-localization-text-browser')),
      findsOneWidget,
    );

    await tester.tap(find.text('Mine warning'));
    await tester.pumpAndSettle();
    expect(find.text(_copy.staleMessage), findsOneWidget);
    expect(
      find.byKey(const Key('revision3-localization-editor-back')),
      findsOneWidget,
    );
    await tester.tap(
      find.byKey(const Key('revision3-localization-editor-back')),
    );
    await tester.pumpAndSettle();
    expect(
      find.byKey(const Key('revision3-localization-text-browser')),
      findsOneWidget,
    );
    expect(find.text('Mine warning'), findsOneWidget);
  });

  testWidgets('typed stale save failure does not claim success', (
    tester,
  ) async {
    await _setSurface(tester, width: 1200);
    var technicalCalls = 0;
    final index = _contentIndex(
      displayName: 'Mine warning',
      locales: const <String>['de', 'en'],
    );
    final service = _serviceForIndex(
      index,
      publish:
          ({
            required expectedProjectId,
            required expectedProjectRevision,
            required technicalPlan,
          }) async {
            technicalCalls++;
            throw const Revision3DialogLocalizationEditStaleCheckpointException();
          },
    );
    await _pumpWorkspace(tester, service: service);

    await tester.enterText(
      find.byKey(const Key('revision3-localization-text-en')),
      'First failed edit',
    );
    await tester.pump();
    await tester.tap(find.byKey(const Key('revision3-localization-save')));
    await _pumpUntil(tester, () => technicalCalls == 1);
    await tester.pump();
    expect(find.text(_copy.staleMessage), findsOneWidget);
    expect(find.text(_copy.savedLabel), findsNothing);
    expect(technicalCalls, 1);
  });

  testWidgets('typed reopen save failure does not claim success', (
    tester,
  ) async {
    await _setSurface(tester, width: 1200);
    var technicalCalls = 0;
    final index = _contentIndex(
      displayName: 'Mine warning',
      locales: const <String>['de', 'en'],
    );
    final service = _serviceForIndex(
      index,
      publish:
          ({
            required expectedProjectId,
            required expectedProjectRevision,
            required technicalPlan,
          }) async {
            technicalCalls++;
            throw const Revision3DialogLocalizationEditRequiresReopenException();
          },
    );
    await _pumpWorkspace(tester, service: service);
    await tester.enterText(
      find.byKey(const Key('revision3-localization-text-en')),
      'Failed reopen edit',
    );
    await tester.pump();
    await tester.tap(find.byKey(const Key('revision3-localization-save')));
    await _pumpUntil(tester, () => technicalCalls == 1);
    await tester.pump();
    expect(find.text(_copy.reopenMessage), findsOneWidget);
    expect(find.text(_copy.savedLabel), findsNothing);
    expect(technicalCalls, 1);
  });

  testWidgets(
    'older catalog completion cannot replace a newer project revision',
    (tester) async {
      await _setSurface(tester, width: 1200);
      final oldCatalog = Completer<Revision3ContentIndex>();
      final oldService = Revision3DialogLocalizationEditAuthoringService(
        loadContentIndex: () => oldCatalog.future,
        loadExactSeed: _unexpectedSeedLoad,
        publishTechnicalPlan: _unexpectedPublication,
      );
      await _pumpWorkspace(tester, service: oldService, settle: false);

      final newIndex = _contentIndex(
        revision: 8,
        displayName: 'New revision text',
        locales: const <String>['de', 'en'],
      );
      final newService = _serviceForIndex(newIndex);
      await _pumpWorkspace(tester, service: newService, projectRevision: 8);
      expect(
        find.widgetWithText(ListTile, 'New revision text'),
        findsOneWidget,
      );
      expect(_textField(tester, 'de').controller!.text, 'Bleib stehen!');

      oldCatalog.complete(
        _contentIndex(
          displayName: 'Stale old text',
          locales: const <String>['de', 'en'],
        ),
      );
      await tester.pumpAndSettle();
      expect(
        find.widgetWithText(ListTile, 'New revision text'),
        findsOneWidget,
      );
      expect(find.text('Stale old text'), findsNothing);
    },
  );

  testWidgets(
    'newer checkpoint preserves dirty text until explicit refresh and discard',
    (tester) async {
      await _setSurface(tester, width: 1200);
      final oldIndex = _contentIndex(
        displayName: 'Mine warning',
        locales: const <String>['de', 'en'],
      );
      await _pumpWorkspace(tester, service: _serviceForIndex(oldIndex));
      await tester.enterText(
        find.byKey(const Key('revision3-localization-text-en')),
        'Keep this unsaved revision-seven draft',
      );
      await tester.pump();

      var newCatalogLoads = 0;
      var newSeedLoads = 0;
      var publications = 0;
      final newIndex = _contentIndex(
        revision: 8,
        displayName: 'Mine warning from revision eight',
        locales: const <String>['de', 'en'],
      );
      final newService = Revision3DialogLocalizationEditAuthoringService(
        loadContentIndex: () async {
          newCatalogLoads++;
          return newIndex;
        },
        loadExactSeed:
            ({
              required expectedProjectId,
              required expectedProjectRevision,
              required localizationId,
              required expectedLocalizationRevision,
              required expectedLocId,
            }) async {
              newSeedLoads++;
              return _exactSeed(
                projectId: expectedProjectId,
                projectRevision: expectedProjectRevision,
                localizationId: localizationId,
                localizationRevision: expectedLocalizationRevision,
                locId: expectedLocId,
                texts: const <String, String>{
                  'de': 'Neuer Projekttext',
                  'en': 'New project text',
                },
              );
            },
        publishTechnicalPlan:
            ({
              required expectedProjectId,
              required expectedProjectRevision,
              required plan,
            }) async {
              publications++;
              throw StateError('stale draft must not publish');
            },
      );
      await _pumpWorkspace(tester, service: newService, projectRevision: 8);

      expect(newCatalogLoads, 0);
      expect(newSeedLoads, 0);
      expect(
        _textField(tester, 'en').controller!.text,
        'Keep this unsaved revision-seven draft',
      );
      expect(
        find.byKey(const Key('revision3-localization-refresh-changed-project')),
        findsOneWidget,
      );
      expect(find.text(_copy.staleMessage), findsOneWidget);
      expect(
        tester
            .widget<FilledButton>(
              find.byKey(const Key('revision3-localization-save')),
            )
            .onPressed,
        isNull,
      );
      await tester.tap(find.byKey(const Key('revision3-localization-save')));
      await tester.pump();
      expect(publications, 0);

      await tester.tap(
        find.byKey(const Key('revision3-localization-refresh-changed-project')),
      );
      await tester.pumpAndSettle();
      expect(find.text(_copy.unsavedTitle), findsOneWidget);
      await tester.tap(find.text(_copy.keepEditingLabel));
      await tester.pumpAndSettle();
      expect(newCatalogLoads, 0);
      expect(newSeedLoads, 0);
      expect(
        _textField(tester, 'en').controller!.text,
        'Keep this unsaved revision-seven draft',
      );

      await tester.tap(
        find.byKey(const Key('revision3-localization-refresh-changed-project')),
      );
      await tester.pumpAndSettle();
      await tester.tap(find.text(_copy.discardLabel));
      await tester.pumpAndSettle();

      expect(newCatalogLoads, 1);
      expect(newSeedLoads, 1);
      expect(_textField(tester, 'de').controller!.text, 'Neuer Projekttext');
      expect(_textField(tester, 'en').controller!.text, 'New project text');
      expect(
        find.byKey(const Key('revision3-localization-refresh-changed-project')),
        findsNothing,
      );
      expect(publications, 0);
    },
  );

  testWidgets(
    'pending manual catalog refresh keeps the clean editor visible and interlocked',
    (tester) async {
      await _setSurface(tester, width: 1200);
      final pendingCatalog = Completer<Revision3ContentIndex>();
      var catalogLoads = 0;
      var seedLoads = 0;
      var externalActions = 0;
      final initialIndex = _contentIndex(
        displayName: 'Mine warning',
        locales: const <String>['de', 'en'],
      );
      final refreshedIndex = _contentIndex(
        displayName: 'Refreshed mine warning',
        locales: const <String>['de', 'en'],
      );
      final service = Revision3DialogLocalizationEditAuthoringService(
        loadContentIndex: () {
          catalogLoads++;
          return catalogLoads == 1
              ? Future<Revision3ContentIndex>.value(initialIndex)
              : pendingCatalog.future;
        },
        loadExactSeed:
            ({
              required expectedProjectId,
              required expectedProjectRevision,
              required localizationId,
              required expectedLocalizationRevision,
              required expectedLocId,
            }) async {
              seedLoads++;
              return _exactSeed(
                projectId: expectedProjectId,
                projectRevision: expectedProjectRevision,
                localizationId: localizationId,
                localizationRevision: expectedLocalizationRevision,
                locId: expectedLocId,
                texts: seedLoads == 1
                    ? const <String, String>{
                        'de': 'Bleib stehen!',
                        'en': 'Stop right there!',
                      }
                    : const <String, String>{
                        'de': 'Frisch geladen',
                        'en': 'Freshly loaded',
                      },
              );
            },
        publishTechnicalPlan: _unexpectedPublication,
      );
      void action() => externalActions++;
      await _pumpWorkspace(
        tester,
        service: service,
        onCreateDialogLine: action,
        onAddVoiceTake: action,
        onManageVoiceTakes: action,
        onResolveVoiceTarget: action,
      );

      await tester.tap(find.byTooltip(_copy.refreshLabel));
      await tester.pump();
      expect(catalogLoads, 2);
      expect(seedLoads, 1);
      await _expectPendingCatalogInterlock(
        tester,
        expectedText: 'Stop right there!',
        choiceLabel: 'Mine warning',
      );
      expect(externalActions, 0);

      pendingCatalog.complete(refreshedIndex);
      await tester.pumpAndSettle();

      expect(seedLoads, 2);
      expect(_textField(tester, 'de').controller!.text, 'Frisch geladen');
      expect(_textField(tester, 'en').controller!.text, 'Freshly loaded');
      expect(_textField(tester, 'en').readOnly, isFalse);
      expect(
        tester
            .widget<OutlinedButton>(
              find.byKey(const Key('revision3-localization-add-language')),
            )
            .onPressed,
        isNotNull,
      );
      expect(
        find.widgetWithText(ListTile, 'Refreshed mine warning'),
        findsOneWidget,
      );
      expect(externalActions, 0);
    },
  );

  testWidgets(
    'pending automatic revision refresh keeps the old clean editor interlocked',
    (tester) async {
      await _setSurface(tester, width: 1200);
      final oldIndex = _contentIndex(
        displayName: 'Mine warning',
        locales: const <String>['de', 'en'],
      );
      await _pumpWorkspace(tester, service: _serviceForIndex(oldIndex));

      final pendingCatalog = Completer<Revision3ContentIndex>();
      var catalogLoads = 0;
      var seedLoads = 0;
      var externalActions = 0;
      final newService = Revision3DialogLocalizationEditAuthoringService(
        loadContentIndex: () {
          catalogLoads++;
          return pendingCatalog.future;
        },
        loadExactSeed:
            ({
              required expectedProjectId,
              required expectedProjectRevision,
              required localizationId,
              required expectedLocalizationRevision,
              required expectedLocId,
            }) async {
              seedLoads++;
              return _exactSeed(
                projectId: expectedProjectId,
                projectRevision: expectedProjectRevision,
                localizationId: localizationId,
                localizationRevision: expectedLocalizationRevision,
                locId: expectedLocId,
                texts: const <String, String>{
                  'de': 'Revision acht',
                  'en': 'Revision eight',
                },
              );
            },
        publishTechnicalPlan: _unexpectedPublication,
      );
      void action() => externalActions++;
      await _pumpWorkspace(
        tester,
        service: newService,
        projectRevision: 8,
        settle: false,
        onCreateDialogLine: action,
        onAddVoiceTake: action,
        onManageVoiceTakes: action,
        onResolveVoiceTarget: action,
      );

      expect(catalogLoads, 1);
      expect(seedLoads, 0);
      await _expectPendingCatalogInterlock(
        tester,
        expectedText: 'Stop right there!',
        choiceLabel: 'Mine warning',
      );
      expect(externalActions, 0);

      pendingCatalog.complete(
        _contentIndex(
          revision: 8,
          displayName: 'Revision-eight warning',
          locales: const <String>['de', 'en'],
        ),
      );
      await tester.pumpAndSettle();

      expect(seedLoads, 1);
      expect(_textField(tester, 'de').controller!.text, 'Revision acht');
      expect(_textField(tester, 'en').controller!.text, 'Revision eight');
      expect(_textField(tester, 'en').readOnly, isFalse);
      expect(
        find.widgetWithText(ListTile, 'Revision-eight warning'),
        findsOneWidget,
      );
      expect(externalActions, 0);
    },
  );

  testWidgets(
    'add-language modal result is rejected after a parent revision reload starts',
    (tester) async {
      await _setSurface(tester, width: 1200);
      final oldIndex = _contentIndex(
        displayName: 'Mine warning',
        locales: const <String>['de', 'en'],
      );
      await _pumpWorkspace(tester, service: _serviceForIndex(oldIndex));
      await tester.tap(
        find.byKey(const Key('revision3-localization-add-language')),
      );
      await tester.pumpAndSettle();
      await tester.enterText(
        find.byKey(const Key('revision3-localization-new-locale-code')),
        'fr',
      );
      await tester.enterText(
        find.byKey(const Key('revision3-localization-new-locale-text')),
        'Ce texte appartient à lancienne révision.',
      );
      await tester.pump();
      expect(_dialogAddButton(tester).onPressed, isNotNull);

      final pendingCatalog = Completer<Revision3ContentIndex>();
      var seedLoads = 0;
      final newService = Revision3DialogLocalizationEditAuthoringService(
        loadContentIndex: () => pendingCatalog.future,
        loadExactSeed:
            ({
              required expectedProjectId,
              required expectedProjectRevision,
              required localizationId,
              required expectedLocalizationRevision,
              required expectedLocId,
            }) async {
              seedLoads++;
              return _exactSeed(
                projectId: expectedProjectId,
                projectRevision: expectedProjectRevision,
                localizationId: localizationId,
                localizationRevision: expectedLocalizationRevision,
                locId: expectedLocId,
                texts: const <String, String>{
                  'de': 'Revision acht',
                  'en': 'Revision eight',
                },
              );
            },
        publishTechnicalPlan: _unexpectedPublication,
      );
      await _pumpWorkspace(
        tester,
        service: newService,
        projectRevision: 8,
        settle: false,
      );
      expect(_dialogAddButton(tester).onPressed, isNotNull);

      await tester.tap(find.widgetWithText(FilledButton, _copy.addLabel));
      await tester.pump();
      await tester.pump();

      expect(find.text(_copy.staleMessage), findsOneWidget);
      expect(
        find.byKey(const Key('revision3-localization-text-fr')),
        findsNothing,
      );
      expect(seedLoads, 0);

      pendingCatalog.complete(
        _contentIndex(
          revision: 8,
          displayName: 'Revision-eight warning',
          locales: const <String>['de', 'en'],
        ),
      );
      await tester.pumpAndSettle();

      expect(seedLoads, 1);
      expect(
        find.byKey(const Key('revision3-localization-text-fr')),
        findsNothing,
      );
      expect(_textField(tester, 'de').controller!.text, 'Revision acht');
      expect(_textField(tester, 'en').controller!.text, 'Revision eight');
    },
  );

  testWidgets(
    'same checkpoint parent rebuild keeps dirty text across a new service instance',
    (tester) async {
      await _setSurface(tester, width: 1200);
      final index = _contentIndex(
        displayName: 'Mine warning',
        locales: const <String>['de', 'en'],
      );
      await _pumpWorkspace(tester, service: _serviceForIndex(index));
      await tester.enterText(
        find.byKey(const Key('revision3-localization-text-en')),
        'Do not discard this parent-rebuild draft',
      );
      await tester.pump();

      var replacementServiceLoads = 0;
      final replacementService =
          Revision3DialogLocalizationEditAuthoringService(
            loadContentIndex: () async {
              replacementServiceLoads++;
              return index;
            },
            loadExactSeed: _unexpectedSeedLoad,
            publishTechnicalPlan: _unexpectedPublication,
          );
      await _pumpWorkspace(tester, service: replacementService);

      expect(replacementServiceLoads, 0);
      expect(
        _textField(tester, 'en').controller!.text,
        'Do not discard this parent-rebuild draft',
      );
      expect(
        tester
            .widget<FilledButton>(
              find.byKey(const Key('revision3-localization-save')),
            )
            .onPressed,
        isNotNull,
      );
    },
  );

  testWidgets('older seed completion cannot replace a newer selection', (
    tester,
  ) async {
    await _setSurface(tester, width: 1200);
    final alpha = Completer<AuthoringRevision3DialogLocalizationEditSeed>();
    final beta = Completer<AuthoringRevision3DialogLocalizationEditSeed>();
    final index = _contentIndex(
      displayName: 'Alpha warning',
      locales: const <String>['de', 'en'],
      secondDisplayName: 'Beta greeting',
    );
    final service = Revision3DialogLocalizationEditAuthoringService(
      loadContentIndex: () async => index,
      loadExactSeed:
          ({
            required expectedProjectId,
            required expectedProjectRevision,
            required localizationId,
            required expectedLocalizationRevision,
            required expectedLocId,
          }) => localizationId == _localizationId ? alpha.future : beta.future,
      publishTechnicalPlan: _unexpectedPublication,
    );
    await _pumpWorkspace(tester, service: service, settle: false);
    await tester.pump();
    expect(find.text('Beta greeting'), findsOneWidget);
    await tester.tap(find.text('Beta greeting'));
    await tester.pump();

    beta.complete(
      _exactSeed(
        localizationId: _secondLocalizationId,
        localizationRevision: 4,
        locId: _secondLocId,
        texts: const <String, String>{'de': 'Beta gewinnt', 'en': 'Beta wins'},
        voiceSlots: const <String>{},
        backlinks: const <Map<String, Object?>>[],
      ),
    );
    await tester.pumpAndSettle();
    expect(_textField(tester, 'de').controller!.text, 'Beta gewinnt');

    alpha.complete(
      _exactSeed(
        texts: const <String, String>{
          'de': 'Verspätetes Alpha',
          'en': 'Late alpha',
        },
      ),
    );
    await tester.pumpAndSettle();
    expect(_textField(tester, 'de').controller!.text, 'Beta gewinnt');
    expect(find.text('Verspätetes Alpha'), findsNothing);
  });

  testWidgets('external actions require discard and are no-ops while saving', (
    tester,
  ) async {
    await _setSurface(tester, width: 1200);
    var actionCalls = 0;
    var technicalCalls = 0;
    final pending = Completer<Revision3DialogLocalizationEditPublication>();
    final index = _contentIndex(
      displayName: 'Mine warning',
      locales: const <String>['de', 'en'],
    );
    final service = _serviceForIndex(
      index,
      publish:
          ({
            required expectedProjectId,
            required expectedProjectRevision,
            required technicalPlan,
          }) {
            technicalCalls++;
            return pending.future;
          },
    );
    await _pumpWorkspace(
      tester,
      service: service,
      onCreateDialogLine: () => actionCalls++,
    );

    await tester.enterText(
      find.byKey(const Key('revision3-localization-text-en')),
      'Dirty external guard',
    );
    await tester.tap(find.byKey(const Key('revision3-localization-new-line')));
    await tester.pumpAndSettle();
    expect(find.text(_copy.unsavedTitle), findsOneWidget);
    await tester.tap(find.text(_copy.keepEditingLabel));
    await tester.pumpAndSettle();
    expect(actionCalls, 0);

    await tester.tap(find.byKey(const Key('revision3-localization-new-line')));
    await tester.pumpAndSettle();
    await tester.tap(find.text(_copy.discardLabel));
    await tester.pumpAndSettle();
    expect(actionCalls, 1);
    expect(_textField(tester, 'en').controller!.text, 'Stop right there!');

    await tester.enterText(
      find.byKey(const Key('revision3-localization-text-en')),
      'Publishing now',
    );
    await tester.pump();
    await tester.tap(find.byKey(const Key('revision3-localization-save')));
    await _pumpUntil(tester, () => technicalCalls == 1);
    await tester.pump();
    expect(find.text(_copy.savingLabel), findsOneWidget);
    await tester.tap(find.byKey(const Key('revision3-localization-new-line')));
    await tester.pump();
    expect(actionCalls, 1);
    expect(find.text(_copy.unsavedTitle), findsNothing);

    pending.complete(
      _publication(
        projectId: _projectId,
        projectRevision: 8,
        localizationId: _localizationId,
        localizationRevision: 5,
      ),
    );
    await tester.pumpAndSettle();
  });
}

typedef _SeedFactory =
    AuthoringRevision3DialogLocalizationEditSeed Function({
      required String projectId,
      required int projectRevision,
      required String localizationId,
      required int localizationRevision,
      required String locId,
    });

typedef _Publisher =
    Future<Revision3DialogLocalizationEditPublication> Function({
      required String expectedProjectId,
      required int expectedProjectRevision,
      required Revision3DialogLocalizationEditTechnicalPlan technicalPlan,
    });

Revision3DialogLocalizationEditAuthoringService _successfulService() {
  final index = _contentIndex(
    displayName: 'Mine warning',
    locales: const <String>['de', 'en'],
  );
  return _serviceForIndex(index);
}

Revision3DialogLocalizationEditAuthoringService _serviceForIndex(
  Revision3ContentIndex index, {
  _SeedFactory? seedFor,
  _Publisher? publish,
}) => Revision3DialogLocalizationEditAuthoringService(
  loadContentIndex: () async => index,
  loadExactSeed:
      ({
        required expectedProjectId,
        required expectedProjectRevision,
        required localizationId,
        required expectedLocalizationRevision,
        required expectedLocId,
      }) async =>
          (seedFor ??
          ({
            required projectId,
            required projectRevision,
            required localizationId,
            required localizationRevision,
            required locId,
          }) => _exactSeed(
            projectId: projectId,
            projectRevision: projectRevision,
            localizationId: localizationId,
            localizationRevision: localizationRevision,
            locId: locId,
          ))(
            projectId: expectedProjectId,
            projectRevision: expectedProjectRevision,
            localizationId: localizationId,
            localizationRevision: expectedLocalizationRevision,
            locId: expectedLocId,
          ),
  publishTechnicalPlan:
      ({
        required expectedProjectId,
        required expectedProjectRevision,
        required plan,
      }) =>
          (publish ??
          ({
            required expectedProjectId,
            required expectedProjectRevision,
            required technicalPlan,
          }) async => _publication(
            projectId: expectedProjectId,
            projectRevision: expectedProjectRevision + 1,
            localizationId: technicalPlan.localizationId,
            localizationRevision:
                technicalPlan.expectedLocalizationRevision + 1,
          ))(
            expectedProjectId: expectedProjectId,
            expectedProjectRevision: expectedProjectRevision,
            technicalPlan: plan,
          ),
);

Future<AuthoringRevision3DialogLocalizationEditSeed> _unexpectedSeedLoad({
  required String expectedProjectId,
  required int expectedProjectRevision,
  required String localizationId,
  required int expectedLocalizationRevision,
  required String expectedLocId,
}) async => throw StateError('seed load was not expected');

Future<Revision3DialogLocalizationEditPublication> _unexpectedPublication({
  required String expectedProjectId,
  required int expectedProjectRevision,
  required Revision3DialogLocalizationEditTechnicalPlan plan,
}) async => throw StateError('publication was not expected');

Future<void> _pumpWorkspace(
  WidgetTester tester, {
  required Revision3DialogLocalizationEditAuthoringService service,
  String projectId = _projectId,
  int projectRevision = 7,
  bool settle = true,
  Revision3LocalizationPublished? onPublished,
  ValueChanged<bool>? onDirtyChanged,
  Revision3LocalizationVoiceAction? onCreateDialogLine,
  Revision3LocalizationVoiceAction? onAddVoiceTake,
  Revision3LocalizationVoiceAction? onManageVoiceTakes,
  Revision3LocalizationVoiceAction? onResolveVoiceTarget,
  Revision3LocalizationVoiceContextAction? onAddVoiceTakeFor,
  Revision3LocalizationVoiceContextAction? onManageVoiceTakesFor,
  Revision3LocalizationVoiceContextAction? onResolveVoiceTargetFor,
}) async {
  await tester.pumpWidget(
    MaterialApp(
      home: Scaffold(
        body: Revision3LocalizationVoiceWorkspace(
          projectId: projectId,
          projectRevision: projectRevision,
          service: service,
          copy: _copy,
          onPublished: onPublished,
          onDirtyChanged: onDirtyChanged,
          onCreateDialogLine: onCreateDialogLine,
          onAddVoiceTake: onAddVoiceTake ?? () {},
          onManageVoiceTakes: onManageVoiceTakes ?? () {},
          onResolveVoiceTarget: onResolveVoiceTarget ?? () {},
          onAddVoiceTakeFor: onAddVoiceTakeFor,
          onManageVoiceTakesFor: onManageVoiceTakesFor,
          onResolveVoiceTargetFor: onResolveVoiceTargetFor,
        ),
      ),
    ),
  );
  if (settle) {
    await tester.pumpAndSettle();
  } else {
    await tester.pump();
  }
}

Future<void> _setSurface(
  WidgetTester tester, {
  required double width,
  double height = 1000,
}) async {
  tester.view.devicePixelRatio = 1;
  tester.view.physicalSize = Size(width, height);
  addTearDown(() {
    tester.view.resetPhysicalSize();
    tester.view.resetDevicePixelRatio();
  });
}

TextField _textField(WidgetTester tester, String locale) => tester
    .widget<TextField>(find.byKey(Key('revision3-localization-text-$locale')));

FilledButton _dialogAddButton(WidgetTester tester) => tester
    .widget<FilledButton>(find.widgetWithText(FilledButton, _copy.addLabel));

Finder _removeButton(String locale) => find.descendant(
  of: find.byKey(ValueKey('revision3-localization-locale-$locale')),
  matching: find.byType(IconButton),
);

IconButton _localeRemoveIcon(WidgetTester tester, String locale) =>
    tester.widget<IconButton>(_removeButton(locale));

Future<void> _expectPendingCatalogInterlock(
  WidgetTester tester, {
  required String expectedText,
  required String choiceLabel,
}) async {
  expect(_textField(tester, 'en').controller!.text, expectedText);
  expect(_textField(tester, 'en').readOnly, isTrue);
  expect(
    tester
        .widget<OutlinedButton>(
          find.byKey(const Key('revision3-localization-add-language')),
        )
        .onPressed,
    isNull,
  );
  expect(_localeRemoveIcon(tester, 'en').onPressed, isNull);
  expect(
    tester
        .widget<FilledButton>(
          find.byKey(const Key('revision3-localization-save')),
        )
        .onPressed,
    isNull,
  );
  expect(
    tester.widget<ListTile>(find.widgetWithText(ListTile, choiceLabel)).onTap,
    isNull,
  );

  final newLine = find.byKey(const Key('revision3-localization-new-line'));
  expect(tester.widget<FilledButton>(newLine).onPressed, isNull);
  final externalButtons = <Finder>[
    newLine,
    for (final key in const <Key>[
      Key('revision3-localization-add-voice'),
      Key('revision3-localization-manage-voice'),
      Key('revision3-localization-resolve-voice'),
    ])
      find.descendant(
        of: find.byKey(key),
        matching: find.byType(OutlinedButton),
      ),
  ];
  for (final finder in externalButtons.skip(1)) {
    expect(tester.widget<OutlinedButton>(finder).onPressed, isNull);
  }
  for (final finder in externalButtons) {
    await tester.tap(finder);
  }
  await tester.pump();
  expect(_textField(tester, 'en').controller!.text, expectedText);
}

Future<void> _pumpUntil(WidgetTester tester, bool Function() condition) async {
  for (var attempt = 0; attempt < 30 && !condition(); attempt++) {
    await tester.pump(const Duration(milliseconds: 10));
  }
  expect(condition(), isTrue, reason: 'asynchronous workflow did not advance');
}

AuthoringWorkingHead _head() => AuthoringWorkingHead.fromCanonicalJson(
  jsonEncode(<String, Object?>{
    'store_format': 1,
    'snapshot': <String, Object?>{'byte_len': 321, 'sha256': 'b' * 64},
  }),
);

AuthoringRevision3DialogLocalizationEditSeed _exactSeed({
  String projectId = _projectId,
  int projectRevision = 7,
  String localizationId = _localizationId,
  int localizationRevision = 4,
  String locId = _locId,
  Map<String, String> texts = const <String, String>{
    'de': 'Bleib stehen!',
    'en': 'Stop right there!',
  },
  Set<String> voiceSlots = const <String>{'de'},
  Set<String> candidateLocales = const <String>{},
  List<Map<String, Object?>>? backlinks,
}) {
  final expectedHead = _head();
  final request = AuthoringRevision3DialogLocalizationEditSeedRequestV1(
    expectedHead: expectedHead,
    localizationId: localizationId,
    expectedLocalizationRevision: localizationRevision,
    expectedLocId: locId,
  );
  final localeEntries = texts.entries.toList(growable: false)
    ..sort((left, right) => left.key.compareTo(right.key));
  return AuthoringRevision3DialogLocalizationEditSeed.fromJson(
    <String, Object?>{
      'ok': true,
      'outcome': 'read_only',
      'head_json': expectedHead.canonicalJson,
      'project_id': projectId,
      'project_revision': projectRevision,
      'localization_id': localizationId,
      'localization_revision': localizationRevision,
      'loc_id': locId,
      'locales': <Object?>[
        for (final entry in localeEntries)
          <String, Object?>{
            'locale': entry.key,
            'text': entry.value,
            'voice_slot_present': voiceSlots.contains(entry.key),
            'candidate_count': candidateLocales.contains(entry.key) ? 1 : 0,
          },
      ],
      'line_backlinks':
          backlinks ??
          _backlinks(voiceSlotLocales: voiceSlots.toList()..sort()),
      'content_authority': 'read_only_exact_current_localization_edit_seed',
      'build_status': 'not_evaluated',
      'runtime_status': 'runtime_unqualified',
      'publication_status': 'not_applicable',
    },
    request: request,
  );
}

List<Map<String, Object?>> _backlinks({
  bool shared = false,
  bool identicalVisibleLines = false,
  List<String> voiceSlotLocales = const <String>['de'],
}) => <Map<String, Object?>>[
  <String, Object?>{
    'line_id': _lineId,
    'line_revision': 2,
    'display_name': 'Mine entrance question',
    'speaker_hint': 'Asghan',
    'voice_slot_locales': voiceSlotLocales,
  },
  if (shared)
    <String, Object?>{
      'line_id': _secondLineId,
      'line_revision': 1,
      'display_name': identicalVisibleLines
          ? 'Mine entrance question'
          : 'Mine entrance question (copy)',
      'speaker_hint': identicalVisibleLines ? 'Asghan' : 'Viper',
      'voice_slot_locales': const <String>[],
    },
];

Revision3DialogLocalizationEditPublication _publication({
  required String projectId,
  required int projectRevision,
  required String localizationId,
  required int localizationRevision,
  List<String> addedLocales = const <String>[],
  List<String> removedLocales = const <String>[],
}) => Revision3DialogLocalizationEditPublication(
  projectId: projectId,
  projectRevision: projectRevision,
  localizationId: localizationId,
  localizationRevision: localizationRevision,
  addedLocales: addedLocales,
  removedLocales: removedLocales,
);

Revision3ContentIndex _contentIndex({
  int revision = 7,
  required String displayName,
  required List<String> locales,
  bool existingDeSlot = true,
  bool duplicateLine = false,
  String? secondDisplayName,
}) {
  final json = revision3VoiceContentIndexJsonFixture(
    revision: revision,
    existingDeSlot: existingDeSlot,
    duplicateLine: duplicateLine,
  );
  final entities = (json['entities']! as List<Object?>)
      .map(
        (value) =>
            (jsonDecode(jsonEncode(value)) as Map).cast<String, Object?>(),
      )
      .toList();
  final localization = entities.singleWhere(
    (entity) => entity['id'] == _localizationId,
  );
  localization['display_name'] = displayName;
  localization['revision'] = 4;
  final summary = (localization['summary']! as Map).cast<String, Object?>();
  final data = (summary['data']! as Map).cast<String, Object?>();
  data['loc_id'] = _locId;
  data['locales'] = <Object?>[...locales];
  summary['data'] = data;
  localization['summary'] = summary;
  final origin = (localization['origin']! as Map).cast<String, Object?>();
  origin['authored_runtime_id'] = _locId;
  localization['origin'] = origin;

  if (secondDisplayName != null) {
    entities.add(<String, Object?>{
      'id': _secondLocalizationId,
      'kind': 'localization_entry',
      'display_name': secondDisplayName,
      'revision': 4,
      'origin': <String, Object?>{
        'type': 'new',
        'authored_runtime_id': _secondLocId,
      },
      'summary': <String, Object?>{
        'kind': 'localization_entry',
        'data': <String, Object?>{
          'loc_id': _secondLocId,
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
  final counts = (json['entity_counts']! as Map).cast<String, Object?>();
  counts['localization_entry'] = secondDisplayName == null ? 1 : 2;
  json['entity_counts'] = counts;
  return Revision3ContentIndex.fromJsonObject(json);
}
