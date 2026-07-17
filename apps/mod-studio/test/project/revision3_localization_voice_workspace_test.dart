import 'dart:async';
import 'dart:convert';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/core/mod_ffi.dart';
import 'package:gore_mod/project/revision3_content_index.dart';
import 'package:gore_mod/project/revision3_dialog_localization_authoring.dart';
import 'package:gore_mod/project/revision3_localization_voice_workspace.dart';
import 'package:gore_mod/project/revision3_voice_authoring.dart';

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
      final index = _contentIndex(
        displayName: 'Mine warning',
        locales: const <String>['de', 'en'],
        duplicateLine: true,
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
        loadVoiceCatalog: () async =>
            Revision3VoiceCatalog.fromContentIndex(index),
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
      final add = find.byKey(const Key('revision3-voice-production-add'));
      await _scrollEditorUntilVisible(tester, add);
      await tester.tap(add);
      await tester.pump();
      expect(calls, <String>['add:$_secondLineId:en']);
      expect(
        find.byKey(const Key('revision3-voice-production-manage')),
        findsNothing,
      );

      final firstLine = find.byKey(
        const ValueKey('revision3-localization-voice-line-$_lineId'),
      );
      await _scrollEditorUntilVisible(tester, firstLine, reverse: true);
      await tester.tap(firstLine);
      await tester.pump();
      await tester.tap(
        find.byKey(const ValueKey('revision3-localization-voice-locale-de')),
      );
      await tester.pump();
      final manage = find.byKey(const Key('revision3-voice-production-manage'));
      await _scrollEditorUntilVisible(tester, manage);
      await tester.tap(manage);
      await tester.pump();
      final resolve = find.byKey(
        const Key('revision3-voice-production-resolve'),
      );
      await _scrollEditorUntilVisible(tester, resolve);
      await tester.tap(resolve);
      await tester.pump();
      expect(calls, <String>[
        'add:$_secondLineId:en',
        'manage:$_lineId:de',
        'resolve:$_lineId:de',
      ]);
    },
  );

  testWidgets(
    'context actions fail closed when the exact Voice catalog rejects a seeded slot',
    (tester) async {
      await _setSurface(tester, width: 1200);
      final index = _contentIndex(
        displayName: 'Mine warning',
        locales: const <String>['de', 'en'],
        existingDeSlot: false,
      );
      var calls = 0;
      await _pumpWorkspace(
        tester,
        service: _serviceForIndex(index),
        loadVoiceCatalog: () async =>
            Revision3VoiceCatalog.fromContentIndex(index),
        onAddVoiceTakeFor: ({required initialLineId, required initialLocale}) {
          calls++;
        },
        onManageVoiceTakesFor:
            ({required initialLineId, required initialLocale}) {
              calls++;
            },
        onResolveVoiceTargetFor:
            ({required initialLineId, required initialLocale}) {
              calls++;
            },
      );

      expect(
        find.byKey(const Key('revision3-voice-production-unsafe')),
        findsOneWidget,
      );
      expect(
        find.byKey(const Key('revision3-voice-production-actions')),
        findsNothing,
      );
      expect(find.textContaining(_lineId), findsNothing);
      expect(calls, 0);
    },
  );

  testWidgets(
    'selected context is unsafe when the Voice projection rejects its whole line',
    (tester) async {
      await _setSurface(tester, width: 1200);
      final index = _contentIndex(
        displayName: 'Mine warning',
        locales: const <String>['de', 'en'],
        duplicateLine: true,
        rejectPrimaryVoiceLine: true,
        secondDisplayName: 'Zeta fallback',
      );
      final voiceCatalog = Revision3VoiceCatalog.fromContentIndex(index);
      expect(voiceCatalog.line(_lineId), isNull);
      expect(voiceCatalog.line(_secondLineId), isNotNull);
      var calls = 0;
      await _pumpWorkspace(
        tester,
        service: _serviceForIndex(index),
        loadVoiceCatalog: () async => voiceCatalog,
        onAddVoiceTakeFor: ({required initialLineId, required initialLocale}) {
          calls++;
        },
        onManageVoiceTakesFor:
            ({required initialLineId, required initialLocale}) {
              calls++;
            },
        onResolveVoiceTargetFor:
            ({required initialLineId, required initialLocale}) {
              calls++;
            },
      );

      await tester.tap(
        find.byKey(
          const ValueKey('revision3-localization-voice-line-$_lineId'),
        ),
      );
      await tester.pump();

      expect(
        find.byKey(const Key('revision3-voice-production-unsafe')),
        findsOneWidget,
      );
      expect(
        find.byKey(const Key('revision3-voice-production-unavailable')),
        findsNothing,
      );
      expect(
        find.byKey(const Key('revision3-voice-production-actions')),
        findsNothing,
      );
      expect(calls, 0);
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
      expect(find.text('Add take for any line'), findsOneWidget);
      expect(find.text('Manage takes for any line'), findsOneWidget);
      expect(find.text('Resolve target for any line'), findsOneWidget);
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
    'speaker search explains a Project text result and keeps locales visible',
    (tester) async {
      await _setSurface(tester, width: 360, height: 900);
      final index = _contentIndex(
        displayName: _locId,
        locales: const <String>['de', 'en'],
        duplicateLine: true,
        secondDisplayName: _secondLocId,
        duplicateLineDisplayName: 'Tunnel greeting',
        duplicateLineSpeaker: 'Viper',
      );
      final catalog = Revision3DialogLocalizationEditCatalog.fromContentIndex(
        index,
      );
      final matched = catalog.choices.singleWhere(
        (choice) => choice.matches('Viper'),
      );
      expect(matched.displayLabel, 'Project text (1)');
      await _pumpWorkspace(tester, service: _serviceForIndex(index));

      await tester.enterText(
        find.byKey(const Key('revision3-localization-search')),
        'Viper',
      );
      await tester.pump();

      expect(find.widgetWithText(ListTile, 'Project text (1)'), findsOneWidget);
      expect(find.text('Project text (2)'), findsNothing);
      final context = tester.widget<Text>(
        find.byKey(
          ValueKey(
            'revision3-localization-choice-context-${matched.stableKey}',
          ),
        ),
      );
      final locales = tester.widget<Text>(
        find.byKey(
          ValueKey(
            'revision3-localization-choice-locales-${matched.stableKey}',
          ),
        ),
      );
      expect(context.data, 'Viper · Tunnel greeting');
      expect(context.maxLines, 1);
      expect(context.overflow, TextOverflow.ellipsis);
      expect(locales.data, matched.locales.join('  ·  '));
      expect(locales.maxLines, 1);
      expect(locales.overflow, TextOverflow.ellipsis);
      for (final technicalValue in <String>[
        _projectId,
        _localizationId,
        _secondLocalizationId,
        _lineId,
        _secondLineId,
        _locId,
        _secondLocId,
      ]) {
        expect(find.textContaining(technicalValue), findsNothing);
      }
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
    'same-revision checkpoint rebind reloads text seed and non-null Voice loader',
    (tester) async {
      await _setSurface(tester, width: 1200);
      final initialIndex = _contentIndex(
        displayName: 'Head A warning',
        locales: const <String>['de', 'en'],
      );
      var initialVoiceLoads = 0;
      await _pumpWorkspace(
        tester,
        service: _serviceForIndex(initialIndex),
        projectCheckpointIdentity: 'head-a',
        loadVoiceCatalog: () async {
          initialVoiceLoads++;
          return Revision3VoiceCatalog.fromContentIndex(initialIndex);
        },
      );
      expect(initialVoiceLoads, 1);
      expect(
        find.byKey(const Key('revision3-voice-production-intact')),
        findsOneWidget,
      );

      final reboundIndex = _contentIndex(
        displayName: 'Head B warning',
        locales: const <String>['de', 'en'],
        existingDeSlot: false,
      );
      var catalogLoads = 0;
      var seedLoads = 0;
      var voiceLoads = 0;
      final reboundService = Revision3DialogLocalizationEditAuthoringService(
        loadContentIndex: () async {
          catalogLoads++;
          return reboundIndex;
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
                  'de': 'Frischer Head-B-Text',
                  'en': 'Fresh head-B text',
                },
                voiceSlots: const <String>{},
                backlinks: _backlinks(voiceSlotLocales: const <String>[]),
              );
            },
        publishTechnicalPlan: _unexpectedPublication,
      );
      await _pumpWorkspace(
        tester,
        service: reboundService,
        projectCheckpointIdentity: 'head-b',
        loadVoiceCatalog: () async {
          voiceLoads++;
          return Revision3VoiceCatalog.fromContentIndex(reboundIndex);
        },
      );

      expect(catalogLoads, 1);
      expect(seedLoads, 1);
      expect(voiceLoads, 1);
      expect(find.widgetWithText(ListTile, 'Head B warning'), findsOneWidget);
      expect(_textField(tester, 'en').controller!.text, 'Fresh head-B text');
      expect(
        find.byKey(const Key('revision3-voice-production-no-slot')),
        findsOneWidget,
      );
      expect(
        find.byKey(const Key('revision3-voice-production-intact')),
        findsNothing,
      );
    },
  );

  testWidgets(
    'dirty same-revision checkpoint rebind preserves text and invalidates Voice authority',
    (tester) async {
      await _setSurface(tester, width: 1200);
      final initialIndex = _contentIndex(
        displayName: 'Head A warning',
        locales: const <String>['de', 'en'],
      );
      var contextActions = 0;
      await _pumpWorkspace(
        tester,
        service: _serviceForIndex(initialIndex),
        projectCheckpointIdentity: 'head-a',
        loadVoiceCatalog: () async =>
            Revision3VoiceCatalog.fromContentIndex(initialIndex),
        onManageVoiceTakesFor:
            ({required initialLineId, required initialLocale}) {
              contextActions++;
            },
      );
      final staleManage = tester
          .widget<OutlinedButton>(
            find.byKey(const Key('revision3-voice-production-manage')),
          )
          .onPressed!;
      await tester.enterText(
        find.byKey(const Key('revision3-localization-text-en')),
        'Keep this dirty head-A draft',
      );
      await tester.pump();

      final reboundIndex = _contentIndex(
        displayName: 'Head B warning',
        locales: const <String>['de', 'en'],
        existingDeSlot: false,
      );
      var catalogLoads = 0;
      var seedLoads = 0;
      var voiceLoads = 0;
      final reboundService = Revision3DialogLocalizationEditAuthoringService(
        loadContentIndex: () async {
          catalogLoads++;
          return reboundIndex;
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
                  'de': 'Frischer Head-B-Text',
                  'en': 'Fresh head-B text',
                },
                voiceSlots: const <String>{},
                backlinks: _backlinks(voiceSlotLocales: const <String>[]),
              );
            },
        publishTechnicalPlan: _unexpectedPublication,
      );
      await _pumpWorkspace(
        tester,
        service: reboundService,
        projectCheckpointIdentity: 'head-b',
        loadVoiceCatalog: () async {
          voiceLoads++;
          return Revision3VoiceCatalog.fromContentIndex(reboundIndex);
        },
        onManageVoiceTakesFor:
            ({required initialLineId, required initialLocale}) {
              contextActions++;
            },
      );

      expect(catalogLoads, 0);
      expect(seedLoads, 0);
      expect(voiceLoads, 0);
      expect(
        _textField(tester, 'en').controller!.text,
        'Keep this dirty head-A draft',
      );
      expect(find.text(_copy.staleMessage), findsOneWidget);
      expect(
        find.byKey(const Key('revision3-voice-production-error')),
        findsOneWidget,
      );
      expect(
        find.byKey(const Key('revision3-voice-production-actions')),
        findsNothing,
      );
      expect(
        tester
            .widget<FilledButton>(
              find.byKey(const Key('revision3-localization-save')),
            )
            .onPressed,
        isNull,
      );

      staleManage();
      await tester.pump();
      expect(contextActions, 0);
      expect(find.text(_copy.voiceUnsavedTitle), findsNothing);

      await tester.tap(
        find.byKey(const Key('revision3-localization-refresh-changed-project')),
      );
      await tester.pumpAndSettle();
      await tester.tap(find.text(_copy.discardLabel));
      await tester.pumpAndSettle();

      expect(catalogLoads, 1);
      expect(seedLoads, 1);
      expect(voiceLoads, 1);
      expect(_textField(tester, 'en').controller!.text, 'Fresh head-B text');
      expect(
        find.byKey(const Key('revision3-voice-production-no-slot')),
        findsOneWidget,
      );
      staleManage();
      await tester.pump();
      expect(contextActions, 0);
    },
  );

  testWidgets(
    'context action revalidates a head rebind after discard confirmation',
    (tester) async {
      await _setSurface(tester, width: 1200);
      final initialIndex = _contentIndex(
        displayName: 'Head A warning',
        locales: const <String>['de', 'en'],
      );
      var contextActions = 0;
      await _pumpWorkspace(
        tester,
        service: _serviceForIndex(initialIndex),
        projectCheckpointIdentity: 'head-a',
        loadVoiceCatalog: () async =>
            Revision3VoiceCatalog.fromContentIndex(initialIndex),
        onManageVoiceTakesFor:
            ({required initialLineId, required initialLocale}) {
              contextActions++;
            },
      );
      const draft = 'Keep this draft across the open discard dialog';
      await tester.enterText(
        find.byKey(const Key('revision3-localization-text-en')),
        draft,
      );
      await tester.pump();
      final manage = find.byKey(const Key('revision3-voice-production-manage'));
      await _scrollEditorUntilVisible(tester, manage);
      await tester.tap(manage);
      await tester.pumpAndSettle();
      expect(find.text(_copy.voiceUnsavedTitle), findsOneWidget);

      final reboundIndex = _contentIndex(
        displayName: 'Head B warning',
        locales: const <String>['de', 'en'],
        existingDeSlot: false,
      );
      var reboundVoiceLoads = 0;
      await _pumpWorkspace(
        tester,
        service: _serviceForIndex(reboundIndex),
        projectCheckpointIdentity: 'head-b',
        loadVoiceCatalog: () async {
          reboundVoiceLoads++;
          return Revision3VoiceCatalog.fromContentIndex(reboundIndex);
        },
        onManageVoiceTakesFor:
            ({required initialLineId, required initialLocale}) {
              contextActions++;
            },
      );
      expect(find.text(_copy.voiceUnsavedTitle), findsOneWidget);
      expect(_textField(tester, 'en').controller!.text, draft);

      await tester.tap(find.text(_copy.discardAndContinueLabel));
      await tester.pumpAndSettle();

      expect(contextActions, 0);
      expect(reboundVoiceLoads, 0);
      expect(_textField(tester, 'en').controller!.text, draft);
      expect(find.text(_copy.staleMessage), findsWidgets);
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
    'failed fresh seed load keeps a discarded stale checkpoint fail closed',
    (tester) async {
      await _setSurface(tester, width: 1200);
      final initialIndex = _contentIndex(
        displayName: 'Head A warning',
        locales: const <String>['de', 'en'],
      );
      await _pumpWorkspace(
        tester,
        service: _serviceForIndex(initialIndex),
        projectCheckpointIdentity: 'head-a',
      );
      await tester.enterText(
        find.byKey(const Key('revision3-localization-text-en')),
        'Discard this stale draft before refreshing',
      );
      await tester.pump();

      final reboundIndex = _contentIndex(
        displayName: 'Head B warning',
        locales: const <String>['de', 'en'],
      );
      var catalogLoads = 0;
      var seedLoads = 0;
      final failingSeedService =
          Revision3DialogLocalizationEditAuthoringService(
            loadContentIndex: () async {
              catalogLoads++;
              return reboundIndex;
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
                  throw StateError('fresh seed unavailable');
                },
            publishTechnicalPlan: _unexpectedPublication,
          );
      await _pumpWorkspace(
        tester,
        service: failingSeedService,
        projectCheckpointIdentity: 'head-b',
      );
      expect(catalogLoads, 0);
      expect(seedLoads, 0);

      await tester.tap(
        find.byKey(const Key('revision3-localization-refresh-changed-project')),
      );
      await tester.pumpAndSettle();
      await tester.tap(find.text(_copy.discardLabel));
      await tester.pumpAndSettle();

      expect(catalogLoads, 1);
      expect(seedLoads, 1);
      expect(find.text(_copy.staleMessage), findsOneWidget);
      expect(
        find.byKey(const Key('revision3-localization-text-editor')),
        findsNothing,
      );
      expect(
        find.byKey(const Key('revision3-localization-save')),
        findsNothing,
      );
    },
  );

  testWidgets(
    'failed global Voice action cannot clear stale checkpoint authority',
    (tester) async {
      await _setSurface(tester, width: 1200);
      final initialIndex = _contentIndex(
        displayName: 'Head A warning',
        locales: const <String>['de', 'en'],
      );
      var actionCalls = 0;
      Future<void> failVoiceAction() async {
        actionCalls++;
        throw StateError('Voice action failed');
      }

      await _pumpWorkspace(
        tester,
        service: _serviceForIndex(initialIndex),
        projectCheckpointIdentity: 'head-a',
        onManageVoiceTakes: failVoiceAction,
      );
      await tester.enterText(
        find.byKey(const Key('revision3-localization-text-en')),
        'Discard before the failing Voice action',
      );
      await tester.pump();

      final reboundIndex = _contentIndex(
        displayName: 'Head B warning',
        locales: const <String>['de', 'en'],
      );
      var reboundCatalogLoads = 0;
      final reboundService = Revision3DialogLocalizationEditAuthoringService(
        loadContentIndex: () async {
          reboundCatalogLoads++;
          return reboundIndex;
        },
        loadExactSeed: _unexpectedSeedLoad,
        publishTechnicalPlan: _unexpectedPublication,
      );
      await _pumpWorkspace(
        tester,
        service: reboundService,
        projectCheckpointIdentity: 'head-b',
        onManageVoiceTakes: failVoiceAction,
      );
      expect(reboundCatalogLoads, 0);

      await tester.tap(
        find.byKey(const Key('revision3-localization-manage-voice')),
      );
      await tester.pumpAndSettle();
      await tester.tap(find.text(_copy.discardAndContinueLabel));
      await tester.pumpAndSettle();

      expect(actionCalls, 1);
      expect(reboundCatalogLoads, 0);
      expect(find.text(_copy.staleMessage), findsOneWidget);
      expect(
        find.byKey(const Key('revision3-localization-refresh-changed-project')),
        findsOneWidget,
      );
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

  testWidgets(
    'external actions offer discard and are visibly disabled while saving',
    (tester) async {
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
      await tester.tap(
        find.byKey(const Key('revision3-localization-new-line')),
      );
      await tester.pumpAndSettle();
      expect(find.text(_copy.voiceUnsavedTitle), findsOneWidget);
      await tester.tap(find.text(_copy.keepEditingLabel));
      await tester.pumpAndSettle();
      expect(actionCalls, 0);

      await tester.tap(
        find.byKey(const Key('revision3-localization-new-line')),
      );
      await tester.pumpAndSettle();
      await tester.tap(find.text(_copy.discardAndContinueLabel));
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
      expect(
        tester
            .widget<FilledButton>(
              find.byKey(const Key('revision3-localization-new-line')),
            )
            .onPressed,
        isNull,
      );
      for (final key in const <Key>[
        Key('revision3-localization-add-voice'),
        Key('revision3-localization-manage-voice'),
        Key('revision3-localization-resolve-voice'),
      ]) {
        final action = find.descendant(
          of: find.byKey(key),
          matching: find.byType(OutlinedButton),
        );
        expect(action, findsOneWidget);
        expect(tester.widget<OutlinedButton>(action).onPressed, isNull);
      }
      expect(actionCalls, 1);
      expect(find.text(_copy.voiceUnsavedTitle), findsNothing);

      pending.complete(
        _publication(
          projectId: _projectId,
          projectRevision: 8,
          localizationId: _localizationId,
          localizationRevision: 5,
        ),
      );
      await tester.pumpAndSettle();
    },
  );

  testWidgets(
    'compact header keeps every pending-save action visible and disabled',
    (tester) async {
      await _setSurface(tester, width: 360, height: 900);
      final pending = Completer<Revision3DialogLocalizationEditPublication>();
      var technicalCalls = 0;
      var actionCalls = 0;
      final index = _contentIndex(
        displayName: 'Mine warning',
        locales: const <String>['de', 'en'],
      );
      await _pumpWorkspace(
        tester,
        service: _serviceForIndex(
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
        ),
        onCreateDialogLine: () => actionCalls++,
        onAddVoiceTake: () => actionCalls++,
        onManageVoiceTakes: () => actionCalls++,
        onResolveVoiceTarget: () => actionCalls++,
      );
      await tester.tap(find.text('Mine warning'));
      await tester.pumpAndSettle();
      _textField(tester, 'de').controller!.text = 'Compact pending save';
      await tester.pump();
      final save = tester.widget<FilledButton>(
        find.byKey(const Key('revision3-localization-save')),
      );
      expect(save.onPressed, isNotNull);
      save.onPressed!();
      await _pumpUntil(tester, () => technicalCalls == 1);
      await tester.pump();

      expect(find.text(_copy.savingLabel), findsOneWidget);
      expect(
        tester
            .widget<FilledButton>(
              find.byKey(const Key('revision3-localization-new-line')),
            )
            .onPressed,
        isNull,
      );
      final moreActions = find.byKey(
        const Key('revision3-localization-more-actions'),
      );
      expect(moreActions, findsOneWidget);
      await tester.tap(moreActions);
      await tester.pump();
      await tester.pump(const Duration(milliseconds: 500));
      for (final key in const <Key>[
        Key('revision3-localization-add-voice'),
        Key('revision3-localization-manage-voice'),
        Key('revision3-localization-resolve-voice'),
      ]) {
        final item = find.byKey(key);
        expect(item, findsOneWidget);
        expect(tester.widget<PopupMenuItem<Object?>>(item).enabled, isFalse);
      }
      expect(actionCalls, 0);

      Navigator.of(
        tester.element(
          find.byKey(const Key('revision3-localization-add-voice')),
        ),
      ).pop();
      await tester.pump();
      await tester.pump(const Duration(milliseconds: 500));
      pending.complete(
        _publication(
          projectId: _projectId,
          projectRevision: 8,
          localizationId: _localizationId,
          localizationRevision: 5,
        ),
      );
      await tester.pumpAndSettle();
      expect(actionCalls, 0);
      expect(tester.takeException(), isNull);
    },
  );

  testWidgets(
    'save and continue waits for post-reload frame before resolving callback',
    (tester) async {
      await _setSurface(tester, width: 1200);
      final initialIndex = _contentIndex(
        displayName: 'Mine warning',
        locales: const <String>['de', 'en'],
      );
      const changedText = 'Save this transcript before opening Voice';
      final reboundIndex = _contentIndex(
        revision: 8,
        localizationRevision: 5,
        displayName: 'Mine warning',
        locales: const <String>['de', 'en'],
      );
      var publications = 0;
      var oldActions = 0;
      var currentActions = 0;
      String? openedLineId;
      String? openedLocale;
      void oldCallback({
        required String initialLineId,
        required String initialLocale,
      }) {
        oldActions++;
      }

      void currentCallback({
        required String initialLineId,
        required String initialLocale,
      }) {
        currentActions++;
        openedLineId = initialLineId;
        openedLocale = initialLocale;
      }

      final initialService = _serviceForIndex(
        initialIndex,
        publish:
            ({
              required expectedProjectId,
              required expectedProjectRevision,
              required technicalPlan,
            }) async {
              publications++;
              return _publication(
                projectId: expectedProjectId,
                projectRevision: expectedProjectRevision + 1,
                localizationId: technicalPlan.localizationId,
                localizationRevision:
                    technicalPlan.expectedLocalizationRevision + 1,
              );
            },
      );
      final reboundService = _serviceForIndex(
        reboundIndex,
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
              texts: const <String, String>{
                'de': 'Bleib stehen!',
                'en': changedText,
              },
            ),
      );
      late StateSetter setHostState;
      var projectRevision = 7;
      Object checkpointIdentity = 'head-a';
      var service = initialService;
      var contentIndex = initialIndex;
      var rebound = false;
      var callbackGateLoaded = false;
      var callbackGateUpdateScheduled = false;
      var callbackNullBuilds = 0;
      var callbackGateUpdates = 0;
      await tester.pumpWidget(
        MaterialApp(
          home: Scaffold(
            body: StatefulBuilder(
              builder: (context, setState) {
                setHostState = setState;
                if (rebound && !callbackGateLoaded) callbackNullBuilds++;
                return Revision3LocalizationVoiceWorkspace(
                  projectId: _projectId,
                  projectRevision: projectRevision,
                  projectCheckpointIdentity: checkpointIdentity,
                  service: service,
                  copy: _copy,
                  loadVoiceCatalog: () async {
                    final catalog = Revision3VoiceCatalog.fromContentIndex(
                      contentIndex,
                    );
                    if (rebound &&
                        !callbackGateLoaded &&
                        !callbackGateUpdateScheduled) {
                      callbackGateUpdateScheduled = true;
                      scheduleMicrotask(() {
                        if (callbackGateLoaded) return;
                        setHostState(() {
                          callbackGateLoaded = true;
                          callbackGateUpdates++;
                        });
                      });
                    }
                    return catalog;
                  },
                  onPublished: (publication) {
                    setHostState(() {
                      projectRevision = publication.projectRevision;
                      checkpointIdentity = 'head-b';
                      service = reboundService;
                      contentIndex = reboundIndex;
                      rebound = true;
                    });
                  },
                  onManageVoiceTakesFor: !rebound
                      ? oldCallback
                      : callbackGateLoaded
                      ? currentCallback
                      : null,
                );
              },
            ),
          ),
        ),
      );
      await tester.pumpAndSettle();

      await tester.enterText(
        find.byKey(const Key('revision3-localization-text-en')),
        changedText,
      );
      await tester.pump();
      final manage = find.byKey(const Key('revision3-voice-production-manage'));
      await _scrollEditorUntilVisible(tester, manage);
      await tester.tap(manage);
      await tester.pumpAndSettle();

      expect(find.text(_copy.voiceUnsavedTitle), findsOneWidget);
      expect(find.text(_copy.saveAndContinueLabel), findsOneWidget);
      expect(find.text(_copy.discardAndContinueLabel), findsOneWidget);
      await tester.tap(find.text(_copy.saveAndContinueLabel));
      await tester.pumpAndSettle();

      expect(publications, 1);
      expect(oldActions, 0);
      expect(currentActions, 1);
      expect(callbackNullBuilds, greaterThan(0));
      expect(callbackGateUpdates, 1);
      expect(openedLineId, _lineId);
      expect(openedLocale, 'de');
      expect(_textField(tester, 'en').controller!.text, changedText);
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
    'save and continue accepts exact publication rebind inside publish await',
    (tester) async {
      await _setSurface(tester, width: 1200);
      const changedText = 'Save across the coordinator publication rebind';
      final pending = Completer<Revision3DialogLocalizationEditPublication>();
      final initialIndex = _contentIndex(
        displayName: 'Mine warning',
        locales: const <String>['de', 'en'],
      );
      final reboundIndex = _contentIndex(
        revision: 8,
        localizationRevision: 5,
        displayName: 'Mine warning',
        locales: const <String>['de', 'en'],
      );
      var publishCalls = 0;
      var reboundSeedLoads = 0;
      var reboundVoiceLoads = 0;
      var oldPublished = 0;
      var currentPublished = 0;
      var oldActions = 0;
      var currentActions = 0;
      final initialService = _serviceForIndex(
        initialIndex,
        publish:
            ({
              required expectedProjectId,
              required expectedProjectRevision,
              required technicalPlan,
            }) {
              publishCalls++;
              return pending.future;
            },
      );
      final reboundService = _serviceForIndex(
        reboundIndex,
        seedFor:
            ({
              required projectId,
              required projectRevision,
              required localizationId,
              required localizationRevision,
              required locId,
            }) {
              reboundSeedLoads++;
              return _exactSeed(
                projectId: projectId,
                projectRevision: projectRevision,
                localizationId: localizationId,
                localizationRevision: localizationRevision,
                locId: locId,
                texts: const <String, String>{
                  'de': 'Bleib stehen!',
                  'en': changedText,
                },
              );
            },
      );
      void oldCallback({
        required String initialLineId,
        required String initialLocale,
      }) {
        oldActions++;
      }

      void currentCallback({
        required String initialLineId,
        required String initialLocale,
      }) {
        currentActions++;
      }

      late StateSetter setHostState;
      var projectRevision = 7;
      Object checkpointIdentity = 'head-a';
      var service = initialService;
      var contentIndex = initialIndex;
      var rebound = false;
      await tester.pumpWidget(
        MaterialApp(
          home: Scaffold(
            body: StatefulBuilder(
              builder: (context, setState) {
                setHostState = setState;
                return Revision3LocalizationVoiceWorkspace(
                  projectId: _projectId,
                  projectRevision: projectRevision,
                  projectCheckpointIdentity: checkpointIdentity,
                  service: service,
                  copy: _copy,
                  loadVoiceCatalog: () async {
                    if (rebound) reboundVoiceLoads++;
                    return Revision3VoiceCatalog.fromContentIndex(contentIndex);
                  },
                  onPublished: rebound
                      ? (_) => currentPublished++
                      : (_) => oldPublished++,
                  onManageVoiceTakesFor: rebound
                      ? currentCallback
                      : oldCallback,
                );
              },
            ),
          ),
        ),
      );
      await tester.pumpAndSettle();
      await tester.enterText(
        find.byKey(const Key('revision3-localization-text-en')),
        changedText,
      );
      await tester.pump();
      final manage = find.byKey(const Key('revision3-voice-production-manage'));
      await _scrollEditorUntilVisible(tester, manage);
      await tester.tap(manage);
      await tester.pumpAndSettle();
      await tester.tap(find.text(_copy.saveAndContinueLabel));
      await _pumpUntil(tester, () => publishCalls == 1);

      setHostState(() {
        projectRevision = 8;
        checkpointIdentity = 'head-b';
        service = reboundService;
        contentIndex = reboundIndex;
        rebound = true;
      });
      await tester.pump();
      expect(find.text(_copy.savingLabel), findsOneWidget);
      expect(_textField(tester, 'en').controller!.text, changedText);
      expect(reboundSeedLoads, 0);
      expect(reboundVoiceLoads, 0);

      pending.complete(
        _publication(
          projectId: _projectId,
          projectRevision: 8,
          localizationId: _localizationId,
          localizationRevision: 5,
        ),
      );
      await tester.pumpAndSettle();

      expect(oldPublished, 0);
      expect(currentPublished, 0);
      expect(oldActions, 0);
      expect(currentActions, 1);
      expect(reboundSeedLoads, 1);
      expect(reboundVoiceLoads, 1);
      expect(_textField(tester, 'en').controller!.text, changedText);
      expect(find.text(_copy.staleMessage), findsNothing);
    },
  );

  testWidgets(
    'save and continue rejects same-revision checkpoint drift during reload',
    (tester) async {
      await _setSurface(tester, width: 1200);
      const changedText = 'Keep the first publication checkpoint pinned';
      final publication =
          Completer<Revision3DialogLocalizationEditPublication>();
      final checkpointBCatalog = Completer<Revision3ContentIndex>();
      final initialIndex = _contentIndex(
        displayName: 'Mine warning',
        locales: const <String>['de', 'en'],
      );
      final checkpointBIndex = _contentIndex(
        revision: 8,
        localizationRevision: 5,
        displayName: 'Mine warning',
        locales: const <String>['de', 'en'],
      );
      final checkpointCIndex = _contentIndex(
        revision: 8,
        localizationRevision: 5,
        displayName: 'Mine warning',
        locales: const <String>['de', 'en'],
      );
      var publishCalls = 0;
      var checkpointBCatalogLoads = 0;
      var checkpointCCatalogLoads = 0;
      var actions = 0;
      final initialService = _serviceForIndex(
        initialIndex,
        publish:
            ({
              required expectedProjectId,
              required expectedProjectRevision,
              required technicalPlan,
            }) {
              publishCalls++;
              return publication.future;
            },
      );
      final checkpointBService = _serviceForIndex(
        checkpointBIndex,
        loadIndex: () {
          checkpointBCatalogLoads++;
          return checkpointBCatalog.future;
        },
      );
      final checkpointCService = _serviceForIndex(
        checkpointCIndex,
        loadIndex: () async {
          checkpointCCatalogLoads++;
          return checkpointCIndex;
        },
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
              texts: const <String, String>{
                'de': 'Bleib stehen!',
                'en': changedText,
              },
            ),
      );

      late StateSetter setHostState;
      var projectRevision = 7;
      Object checkpointIdentity = 'head-a';
      var service = initialService;
      var contentIndex = initialIndex;
      await tester.pumpWidget(
        MaterialApp(
          home: Scaffold(
            body: StatefulBuilder(
              builder: (context, setState) {
                setHostState = setState;
                return Revision3LocalizationVoiceWorkspace(
                  projectId: _projectId,
                  projectRevision: projectRevision,
                  projectCheckpointIdentity: checkpointIdentity,
                  service: service,
                  copy: _copy,
                  loadVoiceCatalog: () async =>
                      Revision3VoiceCatalog.fromContentIndex(contentIndex),
                  onManageVoiceTakesFor:
                      ({required initialLineId, required initialLocale}) {
                        actions++;
                      },
                );
              },
            ),
          ),
        ),
      );
      await tester.pumpAndSettle();
      await tester.enterText(
        find.byKey(const Key('revision3-localization-text-en')),
        changedText,
      );
      await tester.pump();
      final manage = find.byKey(const Key('revision3-voice-production-manage'));
      await _scrollEditorUntilVisible(tester, manage);
      await tester.tap(manage);
      await tester.pumpAndSettle();
      await tester.tap(find.text(_copy.saveAndContinueLabel));
      await _pumpUntil(tester, () => publishCalls == 1);

      setHostState(() {
        projectRevision = 8;
        checkpointIdentity = 'head-b';
        service = checkpointBService;
        contentIndex = checkpointBIndex;
      });
      await tester.pump();
      publication.complete(
        _publication(
          projectId: _projectId,
          projectRevision: 8,
          localizationId: _localizationId,
          localizationRevision: 5,
        ),
      );
      await _pumpUntil(tester, () => checkpointBCatalogLoads == 1);

      setHostState(() {
        checkpointIdentity = 'head-c';
        service = checkpointCService;
        contentIndex = checkpointCIndex;
      });
      await tester.pump();
      await _pumpUntil(tester, () => checkpointCCatalogLoads == 1);
      checkpointBCatalog.complete(checkpointBIndex);
      await tester.pumpAndSettle();

      expect(actions, 0);
      expect(
        find.text('${_copy.savedLabel}. ${_copy.staleMessage}'),
        findsOneWidget,
      );
      expect(_textField(tester, 'en').controller!.text, changedText);
    },
  );

  testWidgets(
    'save and continue interlocks wide authority while Voice reload is pending',
    (tester) async {
      const changedText = 'Do not open another workflow during Voice reload';
      final fixture = await _pumpPendingVoiceReload(
        tester,
        width: 1200,
        changedText: changedText,
        secondDisplayName: 'Beta greeting',
      );

      final newLine = find.byKey(const Key('revision3-localization-new-line'));
      expect(tester.widget<FilledButton>(newLine).onPressed, isNull);
      final refresh = find.byKey(
        const Key('revision3-localization-browser-refresh'),
      );
      expect(tester.widget<IconButton>(refresh).onPressed, isNull);
      final betaChoice = find.widgetWithText(ListTile, 'Beta greeting');
      expect(tester.widget<ListTile>(betaChoice).onTap, isNull);
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
      final voiceLine = find.byKey(
        const ValueKey('revision3-localization-voice-line-$_lineId'),
      );
      expect(tester.widget<ListTile>(voiceLine).onTap, isNull);
      final enVoiceLocale = find.byKey(
        const ValueKey('revision3-localization-voice-locale-en'),
      );
      expect(tester.widget<ChoiceChip>(enVoiceLocale).onSelected, isNull);

      final search = find.byKey(const Key('revision3-localization-search'));
      expect(tester.widget<TextField>(search).enabled, isNot(false));
      await tester.enterText(search, 'Beta');
      await tester.pump();
      expect(find.text('Beta greeting'), findsOneWidget);
      await tester.tap(newLine);
      await tester.tap(refresh);
      await tester.tap(betaChoice);
      await tester.pump();
      expect(fixture.globalActions, 0);
      expect(fixture.reboundCatalogLoads, 1);
      expect(fixture.reboundSeedLoads, 1);

      fixture.completeVoiceReload();
      await tester.pumpAndSettle();
      expect(fixture.manageActions, 1);
      expect(fixture.globalActions, 0);
      expect(tester.widget<FilledButton>(newLine).onPressed, isNotNull);
    },
  );

  testWidgets(
    'compact save and continue locks back and context during Voice reload',
    (tester) async {
      const changedText = 'Keep compact authority pinned during Voice reload';
      final fixture = await _pumpPendingVoiceReload(
        tester,
        width: 360,
        changedText: changedText,
      );

      final back = find.byKey(const Key('revision3-localization-editor-back'));
      expect(tester.widget<IconButton>(back).onPressed, isNull);
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
            .widget<ListTile>(
              find.byKey(
                const ValueKey('revision3-localization-voice-line-$_lineId'),
              ),
            )
            .onTap,
        isNull,
      );
      expect(
        tester
            .widget<ChoiceChip>(
              find.byKey(
                const ValueKey('revision3-localization-voice-locale-en'),
              ),
            )
            .onSelected,
        isNull,
      );
      await tester.tap(back);
      await tester.pump();
      expect(
        find.byKey(const Key('revision3-localization-text-editor')),
        findsOneWidget,
      );

      fixture.completeVoiceReload();
      await tester.pumpAndSettle();
      expect(fixture.manageActions, 1);
      expect(tester.widget<IconButton>(back).onPressed, isNotNull);
    },
  );

  testWidgets(
    'retry stays visibly disabled and New dialog failure uses selected-action copy',
    (tester) async {
      await _setSurface(tester, width: 360, height: 900);
      final pendingAction = Completer<void>();
      var catalogLoads = 0;
      var actionCalls = 0;
      final index = _contentIndex(
        displayName: 'Mine warning',
        locales: const <String>['de', 'en'],
      );
      final service = _serviceForIndex(
        index,
        loadIndex: () async {
          catalogLoads++;
          throw StateError('fixture catalog failure');
        },
      );
      await _pumpWorkspace(
        tester,
        service: service,
        onCreateDialogLine: () async {
          actionCalls++;
          if (actionCalls == 1) {
            await pendingAction.future;
            return;
          }
          throw StateError('fixture selected-action failure');
        },
      );

      final retry = find.byKey(
        const Key('revision3-localization-catalog-retry'),
      );
      expect(retry, findsOneWidget);
      expect(tester.widget<FilledButton>(retry).onPressed, isNotNull);
      final newLine = find.byKey(const Key('revision3-localization-new-line'));
      await tester.tap(newLine);
      await _pumpUntil(tester, () => actionCalls == 1);
      await tester.pump();

      expect(tester.widget<FilledButton>(retry).onPressed, isNull);
      await tester.tap(retry);
      await tester.pump();
      expect(catalogLoads, 1);

      pendingAction.complete();
      await tester.pumpAndSettle();
      expect(tester.widget<FilledButton>(retry).onPressed, isNotNull);

      await tester.tap(newLine);
      await tester.pumpAndSettle();
      expect(actionCalls, 2);
      expect(
        find.textContaining('The selected action did not finish cleanly.'),
        findsOneWidget,
      );
      expect(find.textContaining('The Voice action'), findsNothing);
    },
  );

  testWidgets(
    'global discard action aborts when project changes under confirmation',
    (tester) async {
      await _setSurface(tester, width: 1200);
      var projectAActions = 0;
      var projectBActions = 0;
      final projectAIndex = _contentIndex(
        displayName: 'Project A warning',
        locales: const <String>['de', 'en'],
      );
      await _pumpWorkspace(
        tester,
        service: _serviceForIndex(projectAIndex),
        projectCheckpointIdentity: 'project-a-head',
        onCreateDialogLine: () => projectAActions++,
      );
      await tester.enterText(
        find.byKey(const Key('revision3-localization-text-en')),
        'Do not carry this project-A intent into project B',
      );
      await tester.pump();
      await tester.tap(
        find.byKey(const Key('revision3-localization-new-line')),
      );
      await tester.pumpAndSettle();
      expect(find.text(_copy.voiceUnsavedTitle), findsOneWidget);

      const projectBId = 'bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb';
      final projectBIndex = _contentIndex(
        projectId: projectBId,
        revision: 3,
        displayName: 'Project B warning',
        locales: const <String>['de', 'en'],
      );
      await _pumpWorkspace(
        tester,
        projectId: projectBId,
        projectRevision: 3,
        projectCheckpointIdentity: 'project-b-head',
        service: _serviceForIndex(projectBIndex),
        onCreateDialogLine: () => projectBActions++,
      );
      expect(find.text(_copy.voiceUnsavedTitle), findsOneWidget);
      await tester.tap(find.text(_copy.discardAndContinueLabel));
      await tester.pumpAndSettle();

      expect(projectAActions, 0);
      expect(projectBActions, 0);
      expect(find.text(_copy.staleMessage), findsOneWidget);
    },
  );

  testWidgets(
    'project switch replaces a pending external-action owner without a late unlock',
    (tester) async {
      await _setSurface(tester, width: 1200);
      final projectAPublication =
          Completer<Revision3DialogLocalizationEditPublication>();
      final projectAReboundCatalog = Completer<Revision3ContentIndex>();
      final projectBAction = Completer<void>();
      var projectAPublishCalls = 0;
      var projectAReboundCatalogLoads = 0;
      var projectBCatalogLoads = 0;
      var projectAActions = 0;
      var projectBActions = 0;
      final projectAIndex = _contentIndex(
        displayName: 'Project A warning',
        locales: const <String>['de', 'en'],
      );
      final projectAReboundIndex = _contentIndex(
        revision: 8,
        localizationRevision: 5,
        displayName: 'Project A warning',
        locales: const <String>['de', 'en'],
      );
      final projectAService = _serviceForIndex(
        projectAIndex,
        publish:
            ({
              required expectedProjectId,
              required expectedProjectRevision,
              required technicalPlan,
            }) {
              projectAPublishCalls++;
              return projectAPublication.future;
            },
      );
      final projectAReboundService = _serviceForIndex(
        projectAReboundIndex,
        loadIndex: () {
          projectAReboundCatalogLoads++;
          return projectAReboundCatalog.future;
        },
      );
      const projectBId = 'bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb';
      final projectBIndex = _contentIndex(
        projectId: projectBId,
        revision: 3,
        displayName: 'Project B warning',
        locales: const <String>['de', 'en'],
      );
      final projectBService = _serviceForIndex(
        projectBIndex,
        loadIndex: () async {
          projectBCatalogLoads++;
          return projectBIndex;
        },
      );

      late StateSetter setHostState;
      var projectId = _projectId;
      var projectRevision = 7;
      Object checkpointIdentity = 'project-a-head';
      var service = projectAService;
      await tester.pumpWidget(
        MaterialApp(
          home: Scaffold(
            body: StatefulBuilder(
              builder: (context, setState) {
                setHostState = setState;
                final projectB = projectId == projectBId;
                return Revision3LocalizationVoiceWorkspace(
                  projectId: projectId,
                  projectRevision: projectRevision,
                  projectCheckpointIdentity: checkpointIdentity,
                  service: service,
                  copy: _copy,
                  onCreateDialogLine: projectB
                      ? () {
                          projectBActions++;
                          return projectBAction.future;
                        }
                      : () => projectAActions++,
                  onPublished: projectB
                      ? null
                      : (_) {
                          setHostState(() {
                            projectRevision = 8;
                            checkpointIdentity = 'project-a-published-head';
                            service = projectAReboundService;
                          });
                        },
                );
              },
            ),
          ),
        ),
      );
      await tester.pumpAndSettle();
      await tester.enterText(
        find.byKey(const Key('revision3-localization-text-en')),
        'Pending project-A external save',
      );
      await tester.pump();
      await tester.tap(
        find.byKey(const Key('revision3-localization-new-line')),
      );
      await tester.pumpAndSettle();
      await tester.tap(find.text(_copy.saveAndContinueLabel));
      await _pumpUntil(tester, () => projectAPublishCalls == 1);

      projectAPublication.complete(
        _publication(
          projectId: _projectId,
          projectRevision: 8,
          localizationId: _localizationId,
          localizationRevision: 5,
        ),
      );
      await _pumpUntil(tester, () => projectAReboundCatalogLoads == 1);

      setHostState(() {
        projectId = projectBId;
        projectRevision = 3;
        checkpointIdentity = 'project-b-head';
        service = projectBService;
      });
      await tester.pumpAndSettle();
      expect(projectBCatalogLoads, 1);
      final newLine = find.byKey(const Key('revision3-localization-new-line'));
      expect(tester.widget<FilledButton>(newLine).onPressed, isNotNull);

      await tester.tap(newLine);
      await _pumpUntil(tester, () => projectBActions == 1);
      await tester.pump();
      expect(tester.widget<FilledButton>(newLine).onPressed, isNull);

      projectAReboundCatalog.complete(projectAReboundIndex);
      await tester.pump();
      await tester.pump(const Duration(milliseconds: 10));

      expect(projectAActions, 0);
      expect(projectBActions, 1);
      expect(projectBCatalogLoads, 1);
      expect(find.textContaining(_copy.staleMessage), findsNothing);
      expect(find.text(_copy.voiceActionFailedMessage), findsNothing);
      expect(
        tester.widget<FilledButton>(newLine).onPressed,
        isNull,
        reason: 'project A must not release project B\'s action owner',
      );

      projectBAction.complete();
      await tester.pumpAndSettle();
      expect(tester.widget<FilledButton>(newLine).onPressed, isNotNull);
    },
  );

  testWidgets(
    'old discarded action cannot reload a same-revision replacement project',
    (tester) async {
      await _setSurface(tester, width: 1200);
      final projectAAction = Completer<void>();
      var projectAActions = 0;
      var projectBCatalogLoads = 0;
      final projectAIndex = _contentIndex(
        displayName: 'Project A warning',
        locales: const <String>['de', 'en'],
      );
      final projectAService = _serviceForIndex(projectAIndex);
      const projectBId = 'bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb';
      final projectBIndex = _contentIndex(
        projectId: projectBId,
        displayName: 'Project B warning',
        locales: const <String>['de', 'en'],
      );
      final projectBService = _serviceForIndex(
        projectBIndex,
        loadIndex: () async {
          projectBCatalogLoads++;
          return projectBIndex;
        },
      );

      late StateSetter setHostState;
      var projectId = _projectId;
      Object checkpointIdentity = 'project-a-head';
      var service = projectAService;
      await tester.pumpWidget(
        MaterialApp(
          home: Scaffold(
            body: StatefulBuilder(
              builder: (context, setState) {
                setHostState = setState;
                return Revision3LocalizationVoiceWorkspace(
                  projectId: projectId,
                  projectRevision: 7,
                  projectCheckpointIdentity: checkpointIdentity,
                  service: service,
                  copy: _copy,
                  onCreateDialogLine: projectId == projectBId
                      ? () {}
                      : () {
                          projectAActions++;
                          return projectAAction.future;
                        },
                );
              },
            ),
          ),
        ),
      );
      await tester.pumpAndSettle();
      await tester.enterText(
        find.byKey(const Key('revision3-localization-text-en')),
        'Discard this stale project-A draft',
      );
      await tester.pump();
      setHostState(() => checkpointIdentity = 'project-a-drifted-head');
      await tester.pump();

      await tester.tap(
        find.byKey(const Key('revision3-localization-new-line')),
      );
      await tester.pumpAndSettle();
      expect(find.text(_copy.voiceUnsavedTitle), findsOneWidget);
      await tester.tap(find.text(_copy.discardAndContinueLabel));
      await _pumpUntil(tester, () => projectAActions == 1);

      setHostState(() {
        projectId = projectBId;
        checkpointIdentity = 'project-b-head';
        service = projectBService;
      });
      await tester.pumpAndSettle();
      expect(projectBCatalogLoads, 1);

      projectAAction.complete();
      await tester.pump();
      await tester.pump(const Duration(milliseconds: 10));

      expect(projectBCatalogLoads, 1);
      expect(find.text(_copy.voiceActionFailedMessage), findsNothing);
      expect(find.text('Project B warning'), findsWidgets);
    },
  );

  testWidgets(
    'old failed external action cannot message a replacement project',
    (tester) async {
      await _setSurface(tester, width: 1200);
      final projectAAction = Completer<void>();
      var projectAActions = 0;
      final projectAIndex = _contentIndex(
        displayName: 'Project A warning',
        locales: const <String>['de', 'en'],
      );
      await _pumpWorkspace(
        tester,
        service: _serviceForIndex(projectAIndex),
        projectCheckpointIdentity: 'project-a-head',
        onCreateDialogLine: () {
          projectAActions++;
          return projectAAction.future;
        },
      );
      await tester.tap(
        find.byKey(const Key('revision3-localization-new-line')),
      );
      await _pumpUntil(tester, () => projectAActions == 1);

      const projectBId = 'bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb';
      final projectBIndex = _contentIndex(
        projectId: projectBId,
        revision: 3,
        displayName: 'Project B warning',
        locales: const <String>['de', 'en'],
      );
      await _pumpWorkspace(
        tester,
        projectId: projectBId,
        projectRevision: 3,
        projectCheckpointIdentity: 'project-b-head',
        service: _serviceForIndex(projectBIndex),
        onCreateDialogLine: () {},
      );

      projectAAction.completeError(StateError('late project-A failure'));
      await tester.pumpAndSettle();

      expect(find.text(_copy.voiceActionFailedMessage), findsNothing);
      expect(find.text('Project B warning'), findsWidgets);
      expect(
        tester
            .widget<FilledButton>(
              find.byKey(const Key('revision3-localization-new-line')),
            )
            .onPressed,
        isNotNull,
      );
    },
  );

  testWidgets(
    'old save completion cannot mutate a replacement project or its callback',
    (tester) async {
      await _setSurface(tester, width: 1200);
      final pending = Completer<Revision3DialogLocalizationEditPublication>();
      var publishCalls = 0;
      var oldPublished = 0;
      var replacementPublished = 0;
      final initialIndex = _contentIndex(
        displayName: 'Mine warning',
        locales: const <String>['de', 'en'],
      );
      final initialService = _serviceForIndex(
        initialIndex,
        publish:
            ({
              required expectedProjectId,
              required expectedProjectRevision,
              required technicalPlan,
            }) {
              publishCalls++;
              return pending.future;
            },
      );
      await _pumpWorkspace(
        tester,
        service: initialService,
        projectCheckpointIdentity: 'project-a-head',
        onPublished: (_) => oldPublished++,
      );
      await tester.enterText(
        find.byKey(const Key('revision3-localization-text-en')),
        'Pending project-A save',
      );
      await tester.pump();
      await tester.tap(find.byKey(const Key('revision3-localization-save')));
      await _pumpUntil(tester, () => publishCalls == 1);

      const replacementProjectId = 'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa';
      final replacementIndex = _contentIndex(
        projectId: replacementProjectId,
        revision: 3,
        displayName: 'Replacement project warning',
        locales: const <String>['de', 'en'],
      );
      await _pumpWorkspace(
        tester,
        projectId: replacementProjectId,
        projectRevision: 3,
        projectCheckpointIdentity: 'project-b-head',
        service: _serviceForIndex(replacementIndex),
        onPublished: (_) => replacementPublished++,
      );
      expect(_textField(tester, 'en').controller!.text, 'Stop right there!');

      pending.complete(
        _publication(
          projectId: _projectId,
          projectRevision: 8,
          localizationId: _localizationId,
          localizationRevision: 5,
        ),
      );
      await tester.pumpAndSettle();

      expect(oldPublished, 0);
      expect(replacementPublished, 0);
      expect(_textField(tester, 'en').controller!.text, 'Stop right there!');
      expect(find.text(_copy.savedLabel), findsNothing);
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
    'old save completion preserves dirty text across same-project head rebind',
    (tester) async {
      await _setSurface(tester, width: 1200);
      final pending = Completer<Revision3DialogLocalizationEditPublication>();
      var publishCalls = 0;
      var oldPublished = 0;
      var reboundPublished = 0;
      final index = _contentIndex(
        displayName: 'Mine warning',
        locales: const <String>['de', 'en'],
      );
      final pendingService = _serviceForIndex(
        index,
        publish:
            ({
              required expectedProjectId,
              required expectedProjectRevision,
              required technicalPlan,
            }) {
              publishCalls++;
              return pending.future;
            },
      );
      await _pumpWorkspace(
        tester,
        service: pendingService,
        projectCheckpointIdentity: 'head-a',
        onPublished: (_) => oldPublished++,
      );
      const draft = 'Keep this draft after the head rebind';
      await tester.enterText(
        find.byKey(const Key('revision3-localization-text-en')),
        draft,
      );
      await tester.pump();
      await tester.tap(find.byKey(const Key('revision3-localization-save')));
      await _pumpUntil(tester, () => publishCalls == 1);

      await _pumpWorkspace(
        tester,
        service: _serviceForIndex(index),
        projectCheckpointIdentity: 'head-b',
        onPublished: (_) => reboundPublished++,
        settle: false,
      );
      expect(_textField(tester, 'en').controller!.text, draft);
      expect(find.text(_copy.savingLabel), findsOneWidget);
      expect(find.text(_copy.staleMessage), findsNothing);

      pending.complete(
        _publication(
          projectId: _projectId,
          projectRevision: 8,
          localizationId: _localizationId,
          localizationRevision: 5,
        ),
      );
      await tester.pumpAndSettle();

      expect(oldPublished, 0);
      expect(reboundPublished, 0);
      expect(_textField(tester, 'en').controller!.text, draft);
      expect(find.text(_copy.savedLabel), findsNothing);
      expect(find.text(_copy.staleMessage), findsWidgets);
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
    'save and continue rejects context missing from freshly rebound catalogs',
    (tester) async {
      await _setSurface(tester, width: 1200);
      final initialIndex = _contentIndex(
        displayName: 'Mine warning',
        locales: const <String>['de', 'en'],
      );
      final reboundIndex = _contentIndex(
        revision: 8,
        localizationRevision: 5,
        displayName: 'Mine warning',
        locales: const <String>['de', 'en'],
      );
      final initialService = _serviceForIndex(initialIndex);
      final reboundService = _serviceForIndex(
        reboundIndex,
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
              texts: const <String, String>{
                'de': 'Bleib stehen!',
                'en': 'Saved without the old line context',
              },
              backlinks: const <Map<String, Object?>>[],
            ),
      );
      var actions = 0;
      late StateSetter setHostState;
      var projectRevision = 7;
      Object checkpointIdentity = 'head-a';
      var service = initialService;
      var contentIndex = initialIndex;
      await tester.pumpWidget(
        MaterialApp(
          home: Scaffold(
            body: StatefulBuilder(
              builder: (context, setState) {
                setHostState = setState;
                return Revision3LocalizationVoiceWorkspace(
                  projectId: _projectId,
                  projectRevision: projectRevision,
                  projectCheckpointIdentity: checkpointIdentity,
                  service: service,
                  copy: _copy,
                  loadVoiceCatalog: () async =>
                      Revision3VoiceCatalog.fromContentIndex(contentIndex),
                  onPublished: (publication) {
                    setHostState(() {
                      projectRevision = publication.projectRevision;
                      checkpointIdentity = 'head-b';
                      service = reboundService;
                      contentIndex = reboundIndex;
                    });
                  },
                  onCreateDialogLine: () {},
                  onManageVoiceTakesFor:
                      ({required initialLineId, required initialLocale}) =>
                          actions++,
                );
              },
            ),
          ),
        ),
      );
      await tester.pumpAndSettle();
      await tester.enterText(
        find.byKey(const Key('revision3-localization-text-en')),
        'Saved without the old line context',
      );
      await tester.pump();
      final manage = find.byKey(const Key('revision3-voice-production-manage'));
      await _scrollEditorUntilVisible(tester, manage);
      await tester.tap(manage);
      await tester.pumpAndSettle();
      await tester.tap(find.text(_copy.saveAndContinueLabel));
      await tester.pumpAndSettle();

      expect(actions, 0);
      expect(find.textContaining(_copy.savedLabel), findsOneWidget);
      expect(find.textContaining(_copy.staleMessage), findsOneWidget);
      expect(
        tester
            .widget<FilledButton>(
              find.byKey(const Key('revision3-localization-new-line')),
            )
            .onPressed,
        isNotNull,
      );
    },
  );

  testWidgets(
    'save and continue without a publication rebind fails closed after one frame',
    (tester) async {
      await _setSurface(tester, width: 1200);
      final index = _contentIndex(
        displayName: 'Mine warning',
        locales: const <String>['de', 'en'],
      );
      var actions = 0;
      await _pumpWorkspace(
        tester,
        service: _serviceForIndex(index),
        loadVoiceCatalog: () async =>
            Revision3VoiceCatalog.fromContentIndex(index),
        onManageVoiceTakesFor:
            ({required initialLineId, required initialLocale}) => actions++,
      );
      await tester.enterText(
        find.byKey(const Key('revision3-localization-text-en')),
        'Save succeeds but this static host never rebinds',
      );
      await tester.pump();
      final manage = find.byKey(const Key('revision3-voice-production-manage'));
      await _scrollEditorUntilVisible(tester, manage);
      await tester.tap(manage);
      await tester.pumpAndSettle();
      await tester.tap(find.text(_copy.saveAndContinueLabel));
      await tester.pumpAndSettle();

      expect(actions, 0);
      expect(find.textContaining(_copy.savedLabel), findsOneWidget);
      expect(find.textContaining(_copy.staleMessage), findsOneWidget);
      expect(
        tester
            .widget<OutlinedButton>(
              find.byKey(const Key('revision3-voice-production-manage')),
            )
            .onPressed,
        isNotNull,
      );
    },
  );

  testWidgets('failed save never continues to the contextual Voice action', (
    tester,
  ) async {
    await _setSurface(tester, width: 1200);
    final index = _contentIndex(
      displayName: 'Mine warning',
      locales: const <String>['de', 'en'],
    );
    var publications = 0;
    var actions = 0;
    final service = _serviceForIndex(
      index,
      publish:
          ({
            required expectedProjectId,
            required expectedProjectRevision,
            required technicalPlan,
          }) async {
            publications++;
            throw StateError('fixture save failure');
          },
    );
    await _pumpWorkspace(
      tester,
      service: service,
      loadVoiceCatalog: () async =>
          Revision3VoiceCatalog.fromContentIndex(index),
      onManageVoiceTakesFor:
          ({required initialLineId, required initialLocale}) => actions++,
    );

    const draft = 'Keep this draft after the failed save';
    await tester.enterText(
      find.byKey(const Key('revision3-localization-text-en')),
      draft,
    );
    await tester.pump();
    final manage = find.byKey(const Key('revision3-voice-production-manage'));
    await _scrollEditorUntilVisible(tester, manage);
    await tester.tap(manage);
    await tester.pumpAndSettle();
    await tester.tap(find.text(_copy.saveAndContinueLabel));
    await tester.pumpAndSettle();

    expect(publications, 1);
    expect(actions, 0);
    expect(_textField(tester, 'en').controller!.text, draft);
    expect(find.text(_copy.genericFailureMessage), findsOneWidget);
    expect(
      tester
          .widget<FilledButton>(
            find.byKey(const Key('revision3-localization-save')),
          )
          .onPressed,
      isNotNull,
    );
  });

  testWidgets('external Voice actions are single-flight and report failures', (
    tester,
  ) async {
    await _setSurface(tester, width: 1200);
    final pending = Completer<void>();
    var calls = 0;
    await _pumpWorkspace(
      tester,
      service: _successfulService(),
      onAddVoiceTake: () async {
        calls++;
        if (calls == 1) {
          await pending.future;
          return;
        }
        throw StateError('fixture Voice action failure');
      },
    );

    final addVoice = find.descendant(
      of: find.byKey(const Key('revision3-localization-add-voice')),
      matching: find.byType(OutlinedButton),
    );
    final manageVoice = find.descendant(
      of: find.byKey(const Key('revision3-localization-manage-voice')),
      matching: find.byType(OutlinedButton),
    );
    await tester.tap(addVoice);
    await tester.pump();

    expect(calls, 1);
    expect(tester.widget<OutlinedButton>(addVoice).onPressed, isNull);
    expect(tester.widget<OutlinedButton>(manageVoice).onPressed, isNull);
    expect(
      tester
          .widget<FilledButton>(
            find.byKey(const Key('revision3-localization-new-line')),
          )
          .onPressed,
      isNull,
    );
    await tester.tap(addVoice);
    await tester.pump();
    expect(calls, 1, reason: 'a pending external action must be single-flight');

    pending.complete();
    await tester.pumpAndSettle();
    expect(tester.widget<OutlinedButton>(addVoice).onPressed, isNotNull);

    await tester.tap(addVoice);
    await tester.pumpAndSettle();
    expect(calls, 2);
    expect(find.text(_copy.voiceActionFailedMessage), findsOneWidget);
    expect(tester.widget<OutlinedButton>(addVoice).onPressed, isNotNull);
  });
}

final class _PendingVoiceReloadFixture {
  _PendingVoiceReloadFixture({
    required this.reboundIndex,
    required this.voiceReload,
  });

  final Revision3ContentIndex reboundIndex;
  final Completer<Revision3VoiceCatalog> voiceReload;
  int publishCalls = 0;
  int reboundCatalogLoads = 0;
  int reboundSeedLoads = 0;
  int reboundVoiceLoads = 0;
  int manageActions = 0;
  int globalActions = 0;

  void completeVoiceReload() => voiceReload.complete(
    Revision3VoiceCatalog.fromContentIndex(reboundIndex),
  );
}

Future<_PendingVoiceReloadFixture> _pumpPendingVoiceReload(
  WidgetTester tester, {
  required double width,
  required String changedText,
  String? secondDisplayName,
}) async {
  await _setSurface(tester, width: width, height: width < 900 ? 900 : 1000);
  final publication = Completer<Revision3DialogLocalizationEditPublication>();
  final voiceReload = Completer<Revision3VoiceCatalog>();
  final primaryLabel = secondDisplayName == null
      ? 'Mine warning'
      : 'Alpha warning';
  final initialIndex = _contentIndex(
    displayName: primaryLabel,
    locales: const <String>['de', 'en'],
    secondDisplayName: secondDisplayName,
  );
  final reboundIndex = _contentIndex(
    revision: 8,
    localizationRevision: 5,
    displayName: primaryLabel,
    locales: const <String>['de', 'en'],
    secondDisplayName: secondDisplayName,
  );
  final fixture = _PendingVoiceReloadFixture(
    reboundIndex: reboundIndex,
    voiceReload: voiceReload,
  );
  final initialService = _serviceForIndex(
    initialIndex,
    publish:
        ({
          required expectedProjectId,
          required expectedProjectRevision,
          required technicalPlan,
        }) {
          fixture.publishCalls++;
          return publication.future;
        },
  );
  final reboundService = _serviceForIndex(
    reboundIndex,
    loadIndex: () async {
      fixture.reboundCatalogLoads++;
      return reboundIndex;
    },
    seedFor:
        ({
          required projectId,
          required projectRevision,
          required localizationId,
          required localizationRevision,
          required locId,
        }) {
          fixture.reboundSeedLoads++;
          return _exactSeed(
            projectId: projectId,
            projectRevision: projectRevision,
            localizationId: localizationId,
            localizationRevision: localizationRevision,
            locId: locId,
            texts: <String, String>{'de': 'Bleib stehen!', 'en': changedText},
          );
        },
  );

  late StateSetter setHostState;
  var projectRevision = 7;
  Object checkpointIdentity = 'head-a';
  var service = initialService;
  var contentIndex = initialIndex;
  var rebound = false;
  await tester.pumpWidget(
    MaterialApp(
      home: Scaffold(
        body: StatefulBuilder(
          builder: (context, setState) {
            setHostState = setState;
            return Revision3LocalizationVoiceWorkspace(
              projectId: _projectId,
              projectRevision: projectRevision,
              projectCheckpointIdentity: checkpointIdentity,
              service: service,
              copy: _copy,
              loadVoiceCatalog: () {
                if (rebound) {
                  fixture.reboundVoiceLoads++;
                  return voiceReload.future;
                }
                return Future<Revision3VoiceCatalog>.value(
                  Revision3VoiceCatalog.fromContentIndex(contentIndex),
                );
              },
              onCreateDialogLine: () => fixture.globalActions++,
              onManageVoiceTakesFor:
                  ({required initialLineId, required initialLocale}) {
                    fixture.manageActions++;
                  },
            );
          },
        ),
      ),
    ),
  );
  await tester.pumpAndSettle();
  if (width < 900) {
    await tester.tap(find.text(primaryLabel));
    await tester.pumpAndSettle();
  }
  await tester.enterText(
    find.byKey(const Key('revision3-localization-text-en')),
    changedText,
  );
  await tester.pump();
  final manage = find.byKey(const Key('revision3-voice-production-manage'));
  await _scrollEditorUntilVisible(tester, manage);
  await tester.tap(manage);
  await tester.pumpAndSettle();
  await tester.tap(find.text(_copy.saveAndContinueLabel));
  await _pumpUntil(tester, () => fixture.publishCalls == 1);

  setHostState(() {
    projectRevision = 8;
    checkpointIdentity = 'head-b';
    service = reboundService;
    contentIndex = reboundIndex;
    rebound = true;
  });
  await tester.pump();
  publication.complete(
    _publication(
      projectId: _projectId,
      projectRevision: 8,
      localizationId: _localizationId,
      localizationRevision: 5,
    ),
  );
  await _pumpUntil(tester, () {
    final field = find.byKey(const Key('revision3-localization-text-en'));
    return fixture.reboundVoiceLoads == 1 &&
        field.evaluate().isNotEmpty &&
        tester.widget<TextField>(field).controller!.text == changedText;
  });
  return fixture;
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
  Future<Revision3ContentIndex> Function()? loadIndex,
}) => Revision3DialogLocalizationEditAuthoringService(
  loadContentIndex: loadIndex ?? () async => index,
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
  Object? projectCheckpointIdentity,
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
  Revision3LocalizationVoiceCatalogLoader? loadVoiceCatalog,
}) async {
  await tester.pumpWidget(
    MaterialApp(
      home: Scaffold(
        body: Revision3LocalizationVoiceWorkspace(
          projectId: projectId,
          projectRevision: projectRevision,
          projectCheckpointIdentity:
              projectCheckpointIdentity ?? projectRevision,
          service: service,
          copy: _copy,
          loadVoiceCatalog: loadVoiceCatalog,
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

Future<void> _scrollEditorUntilVisible(
  WidgetTester tester,
  Finder target, {
  bool reverse = false,
}) async {
  final scrollable = find
      .descendant(
        of: find.byKey(const Key('revision3-localization-editor-scroll')),
        matching: find.byType(Scrollable),
      )
      .first;
  final viewportHeight =
      tester.view.physicalSize.height / tester.view.devicePixelRatio;
  for (var attempt = 0; attempt < 20; attempt++) {
    final center = tester.getCenter(target, warnIfMissed: false);
    if (center.dy >= 0 &&
        center.dy <= viewportHeight &&
        target.hitTestable().evaluate().isNotEmpty) {
      return;
    }
    await tester.drag(scrollable, Offset(0, reverse ? 220 : -220));
    await tester.pump();
  }
  fail('editor target did not become visible');
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
  String projectId = _projectId,
  int revision = 7,
  int localizationRevision = 4,
  required String displayName,
  required List<String> locales,
  bool existingDeSlot = true,
  bool duplicateLine = false,
  bool rejectPrimaryVoiceLine = false,
  String? secondDisplayName,
  String? duplicateLineDisplayName,
  String? duplicateLineSpeaker,
}) {
  var json = revision3VoiceContentIndexJsonFixture(
    revision: revision,
    existingDeSlot: existingDeSlot,
    duplicateLine: duplicateLine,
  );
  if (projectId != _projectId) {
    json = (_replaceProjectId(json, projectId)! as Map).cast<String, Object?>();
  }
  final entities = (json['entities']! as List<Object?>)
      .map(
        (value) =>
            (jsonDecode(jsonEncode(value)) as Map).cast<String, Object?>(),
      )
      .toList();
  if (duplicateLine &&
      (duplicateLineDisplayName != null || duplicateLineSpeaker != null)) {
    final duplicate = entities.singleWhere(
      (entity) => entity['id'] == _secondLineId,
    );
    if (duplicateLineDisplayName != null) {
      duplicate['display_name'] = duplicateLineDisplayName;
    }
    if (duplicateLineSpeaker != null) {
      final duplicateSummary = (duplicate['summary']! as Map)
          .cast<String, Object?>();
      final duplicateData = (duplicateSummary['data']! as Map)
          .cast<String, Object?>();
      duplicateData['speaker_hint'] = duplicateLineSpeaker;
    }
  }
  final localization = entities.singleWhere(
    (entity) => entity['id'] == _localizationId,
  );
  final primaryLocId = rejectPrimaryVoiceLine ? 'CON' : _locId;
  localization['display_name'] = displayName;
  localization['revision'] = localizationRevision;
  final summary = (localization['summary']! as Map).cast<String, Object?>();
  final data = (summary['data']! as Map).cast<String, Object?>();
  data['loc_id'] = primaryLocId;
  data['locales'] = <Object?>[...locales];
  summary['data'] = data;
  localization['summary'] = summary;
  final origin = (localization['origin']! as Map).cast<String, Object?>();
  origin['authored_runtime_id'] = primaryLocId;
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
  if (rejectPrimaryVoiceLine) {
    if (!duplicateLine || secondDisplayName == null) {
      throw ArgumentError(
        'a rejected primary Voice line needs one healthy fallback line',
      );
    }
    final fallbackLine = entities.singleWhere(
      (entity) => entity['id'] == _secondLineId,
    );
    final references = (fallbackLine['references']! as List<Object?>)
        .map((value) => (value! as Map).cast<String, Object?>())
        .toList(growable: false);
    final localizationReference = references.singleWhere(
      (reference) => reference['role'] == 'dialog_localization',
    );
    final target = (localizationReference['target']! as Map)
        .cast<String, Object?>();
    target['entity_id'] = _secondLocalizationId;
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

Object? _replaceProjectId(Object? value, String projectId) => switch (value) {
  String() when value == _projectId => projectId,
  List<Object?>() => <Object?>[
    for (final entry in value) _replaceProjectId(entry, projectId),
  ],
  Map() => <String, Object?>{
    for (final entry in value.entries)
      entry.key as String: _replaceProjectId(entry.value, projectId),
  },
  _ => value,
};
