import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:goresave/features/editor/domain/core_service.dart';
import 'package:goresave/features/editor/domain/editor_models.dart';
import 'package:goresave/features/editor/domain/editor_notifier.dart';
import 'package:goresave/features/editor/domain/glossary_npc_catalog.dart';
import 'package:goresave/features/editor/domain/story_state_models.dart';
import 'package:goresave/features/editor/ui/story_state_panel.dart';
import 'package:goresave/l10n/app_localizations.dart';
import 'package:goresave/loc/game_lang.dart';
import 'package:goresave/loc/loc_catalog_provider.dart';

import '../../../support/l10n_test_app.dart';

void main() {
  testWidgets('keeps catalog guidance hidden behind the info button', (
    tester,
  ) async {
    final notifier = EditorNotifier(_BenignCore(), saveDir: r'C:\tmp\saves');
    addTearDown(notifier.dispose);

    await _pumpEditableStoryPanel(
      tester,
      notifier: notifier,
      page: const StoryStatePage(writable: true),
    );

    final infoButton = find.byKey(const Key('story-state-info'));
    expect(infoButton, findsOneWidget);
    expect(find.byKey(const Key('story-state-info-box')), findsNothing);
    expect(tester.widget<IconButton>(infoButton).isSelected, isFalse);

    await tester.tap(infoButton);
    await tester.pumpAndSettle();
    expect(find.byKey(const Key('story-state-info-box')), findsOneWidget);
    expect(
      find.textContaining('Authoritative catalog of persisted story state'),
      findsOneWidget,
    );
    expect(tester.widget<IconButton>(infoButton).isSelected, isTrue);

    await tester.tap(infoButton);
    await tester.pumpAndSettle();
    expect(find.byKey(const Key('story-state-info-box')), findsNothing);
    expect(tester.widget<IconButton>(infoButton).isSelected, isFalse);
  });

  testWidgets('shows a verified time marker and related localized context', (
    tester,
  ) async {
    final notifier = EditorNotifier(_BenignCore(), saveDir: r'C:\tmp\saves');
    addTearDown(notifier.dispose);
    const segmentClass =
        '/Script/Angelscript.DocumentSegment_Glossary_OCR_GRD_STONE_OreArmor';
    final page = StoryStatePage.fromJson({
      'total': 2,
      'storedTotal': 1,
      'catalogTotal': 2,
      'unsetTotal': 1,
      'currentGameTimeSeconds': 1875587.9437,
      'catalogSemanticTypeCounts': {
        'integer': 1,
        'timeMarker': 1,
        'chapter': 0,
      },
      'entries': [
        {
          'id': 'Stone_OreArmor',
          'rawValue': 1767047,
          'path': [
            'm_GenericData',
            '{Story}',
            'StoryPropertyValues',
            '{Stone_OreArmor}',
          ],
          'semanticType': 'timeMarker',
          'declaredType': 'FInGameTime',
        },
        {
          'id': 'AfterCinematic_Nyras',
          'rawValue': null,
          'stored': false,
          'path': <String>[],
          'semanticType': 'integer',
          'declaredType': 'int32',
        },
      ],
    });

    await tester.pumpWidget(
      ProviderScope(
        overrides: [
          currentGameLangProvider.overrideWithValue(gameLangByCode('de')),
          locCatalogProvider.overrideWith((ref) async {
            return {
              'ocr_grd_stone_219': {'german': 'Stone'},
              'text_stone_ore_armor': {
                'german': 'Er kann meine Erzrüstung verbessern.',
              },
            };
          }),
        ],
        child: MaterialApp(
          locale: const Locale('en'),
          localizationsDelegates: testLocalizationsDelegates,
          supportedLocales: AppLocalizations.supportedLocales,
          home: Scaffold(
            body: SizedBox(
              width: 1100,
              height: 760,
              child: StoryStateDetail(
                notifier: notifier,
                reloadKey: const SaveInspection(
                  format: 'GSAV',
                  path: r'C:\tmp\saves\G1R-035.sav',
                  size: 1,
                  sha1: 'sha1',
                  raw: {},
                ),
                theme: ThemeData(),
                storyLoader:
                    ({required offset, required limit, required path}) async =>
                        page,
                npcCatalogLoader: () async => const [
                  NpcGlossaryCatalogEntry(
                    id: 'OCR_GRD_STONE',
                    uniqueName: 'OCR_GRD_STONE_219',
                    documentClass:
                        '/Script/Angelscript.Document_Glossary_OCR_GRD_STONE',
                    camp: NpcGlossaryCamp.oldCamp,
                    segments: [
                      NpcGlossaryCatalogSegment(
                        id: 'OreArmor',
                        segmentClass: segmentClass,
                        label: 'Ore Armor',
                      ),
                    ],
                  ),
                ],
                segmentTextCatalogLoader: () async => const {
                  '/script/angelscript.documentsegment_glossary_ocr_grd_stone_orearmor':
                      ['TEXT_STONE_ORE_ARMOR'],
                },
              ),
            ),
          ),
        ),
      ),
    );
    await tester.pumpAndSettle();

    expect(find.text('Stone — Ore Armor'), findsOneWidget);
    expect(find.text('Stone_OreArmor'), findsOneWidget);
    expect(find.text('Day 20, 10:50:47'), findsOneWidget);
    expect(find.text('1 of 2 story values'), findsOneWidget);

    await tester.tap(find.text('Stone — Ore Armor'));
    await tester.pumpAndSettle();
    expect(find.text('Raw value: 1767047'), findsOneWidget);
    expect(find.text('Er kann meine Erzrüstung verbessern.'), findsOneWidget);
    expect(find.text('Related glossary entry'), findsOneWidget);

    await tester.tap(find.text('Not set (1)'));
    await tester.pumpAndSettle();
    expect(find.text('AfterCinematic_Nyras'), findsOneWidget);
    expect(find.text('Stone_OreArmor'), findsNothing);
    await tester.tap(find.text('After Cinematic Nyras'));
    await tester.pumpAndSettle();
    expect(
      find.text(
        'This catalog field is not serialized in this save; the game therefore uses its unset or default state.',
      ),
      findsOneWidget,
    );
  });

  testWidgets('labels unknown IDs and preserves a future time direction', (
    tester,
  ) async {
    final notifier = EditorNotifier(_BenignCore(), saveDir: r'C:\tmp\saves');
    addTearDown(notifier.dispose);
    final page = StoryStatePage.fromJson({
      'total': 2,
      'storedTotal': 2,
      'catalogTotal': 1,
      'unsetTotal': 0,
      'unknownStoredTotal': 1,
      'currentGameTimeSeconds': 90000.0,
      'semanticTypeCounts': {
        'integer': 0,
        'timeMarker': 1,
        'chapter': 0,
        'unknown': 1,
      },
      'entries': [
        {
          'id': 'Ambient_InExtremo',
          'rawValue': 90061,
          'stored': true,
          'catalogKnown': true,
          'path': ['StoryPropertyValues', '{Ambient_InExtremo}'],
          'semanticType': 'timeMarker',
          'declaredType': 'FInGameTime',
        },
        {
          'id': 'Mod_NewStoryValue',
          'rawValue': 7,
          'stored': true,
          'catalogKnown': false,
          'path': ['StoryPropertyValues', '{Mod_NewStoryValue}'],
          'semanticType': 'unknown',
          'declaredType': 'unknown',
        },
      ],
    });

    await tester.pumpWidget(
      ProviderScope(
        overrides: [
          currentGameLangProvider.overrideWithValue(gameLangByCode('en')),
          locCatalogProvider.overrideWith((ref) async => const {}),
        ],
        child: MaterialApp(
          locale: const Locale('en'),
          localizationsDelegates: testLocalizationsDelegates,
          supportedLocales: AppLocalizations.supportedLocales,
          home: Scaffold(
            body: SizedBox(
              width: 1100,
              height: 760,
              child: StoryStateDetail(
                notifier: notifier,
                reloadKey: const SaveInspection(
                  format: 'GSAV',
                  path: r'C:\tmp\saves\modded.sav',
                  size: 1,
                  sha1: 'sha1',
                  raw: {},
                ),
                theme: ThemeData(),
                storyLoader:
                    ({required offset, required limit, required path}) async =>
                        page,
                npcCatalogLoader: () async => const [],
                segmentTextCatalogLoader: () async => const {},
              ),
            ),
          ),
        ),
      ),
    );
    await tester.pumpAndSettle();

    expect(find.text('Day 1, 01:01:01'), findsOneWidget);
    expect(find.text('Unknown source type (1)'), findsOneWidget);

    await tester.enterText(
      find.byKey(const Key('story-state-search')),
      'Mod_NewStoryValue',
    );
    await tester.pumpAndSettle();
    expect(find.text('Unknown source type'), findsOneWidget);
    await tester.tap(find.text('Mod New Story Value'));
    await tester.pumpAndSettle();
    expect(
      find.text(
        'This stored ID is absent from the current script catalog (for example, from a mod or newer game version). Its save wire value is int32, but its meaning is not inferred.',
      ),
      findsOneWidget,
    );
    await tester.tap(find.text('Mod New Story Value'));
    await tester.pumpAndSettle();

    await tester.enterText(
      find.byKey(const Key('story-state-search')),
      'Ambient_InExtremo',
    );
    await tester.pumpAndSettle();
    await tester.tap(find.text('Ambient In Extremo'));
    await tester.pumpAndSettle();
    expect(find.text('Ahead of save time: 00:01:01'), findsOneWidget);
  });

  testWidgets('loads every story page beyond the core page-size limit', (
    tester,
  ) async {
    final notifier = EditorNotifier(_BenignCore(), saveDir: r'C:\tmp\saves');
    addTearDown(notifier.dispose);
    final values = List.generate(
      1001,
      (index) => StoryStateValue(
        id: 'Mod_Value_${index.toString().padLeft(4, '0')}',
        value: index,
        stored: true,
        catalogKnown: false,
        path: ['StoryPropertyValues', '{Mod_Value_$index}'],
        semanticType: StorySemanticType.unknown,
        declaredType: 'unknown',
      ),
    );
    final offsets = <int>[];
    final paths = <String?>[];

    await tester.pumpWidget(
      ProviderScope(
        overrides: [
          currentGameLangProvider.overrideWithValue(gameLangByCode('en')),
          locCatalogProvider.overrideWith((ref) async => const {}),
        ],
        child: MaterialApp(
          locale: const Locale('en'),
          localizationsDelegates: testLocalizationsDelegates,
          supportedLocales: AppLocalizations.supportedLocales,
          home: Scaffold(
            body: SizedBox(
              width: 1100,
              height: 760,
              child: StoryStateDetail(
                notifier: notifier,
                reloadKey: const SaveInspection(
                  format: 'GSAV',
                  path: r'C:\tmp\saves\heavily-modded.sav',
                  size: 1,
                  sha1: 'sha1',
                  raw: {},
                ),
                theme: ThemeData(),
                storyLoader:
                    ({required offset, required limit, required path}) async {
                      offsets.add(offset);
                      paths.add(path);
                      final end = (offset + limit).clamp(0, values.length);
                      return StoryStatePage(
                        values: values.sublist(offset, end),
                        kindCounts: const {StorySemanticType.unknown: 1001},
                        total: values.length,
                        storedTotal: values.length,
                        catalogTotal: 470,
                        unsetTotal: 0,
                        unknownStoredTotal: 531,
                        offset: offset,
                        limit: limit,
                      );
                    },
                npcCatalogLoader: () async => const [],
                segmentTextCatalogLoader: () async => const {},
              ),
            ),
          ),
        ),
      ),
    );
    await tester.pumpAndSettle();

    expect(offsets, [0, 1000]);
    expect(paths, [
      r'C:\tmp\saves\heavily-modded.sav',
      r'C:\tmp\saves\heavily-modded.sav',
    ]);
    await tester.enterText(
      find.byKey(const Key('story-state-search')),
      'Mod_Value_1000',
    );
    await tester.pumpAndSettle();
    expect(
      find.byKey(const ValueKey('story-value-Mod_Value_1000')),
      findsOneWidget,
    );
  });

  testWidgets('queues a catalog insertion and can undo it', (tester) async {
    final notifier = EditorNotifier(_BenignCore(), saveDir: r'C:\tmp\saves');
    addTearDown(notifier.dispose);
    final page = StoryStatePage.fromJson({
      'total': 1,
      'storedTotal': 0,
      'catalogTotal': 1,
      'unsetTotal': 1,
      'writable': true,
      'entries': [
        {
          'id': 'AfterCinematic_Nyras',
          'stored': false,
          'catalogKnown': true,
          'rawValue': null,
          'path': <String>[],
          'semanticType': 'integer',
          'declaredType': 'int32',
        },
      ],
    });

    await _pumpEditableStoryPanel(tester, notifier: notifier, page: page);
    await tester.tap(find.text('Not set (1)'));
    await tester.pumpAndSettle();
    await tester.tap(find.text('After Cinematic Nyras'));
    await tester.pumpAndSettle();
    expect(
      find.text('Values evidenced in the shipped scripts: 0, 1'),
      findsNothing,
    );
    expect(
      find.text(
        'Suggestions are not validation limits; native code, mods, or later '
        'game versions may use other values.',
      ),
      findsNothing,
    );
    await _tapVisible(
      tester,
      find.byKey(const ValueKey('story-edit-AfterCinematic_Nyras')),
    );
    await tester.pumpAndSettle();
    expect(
      find.text('Values evidenced in the shipped scripts: 0, 1'),
      findsOneWidget,
    );
    expect(
      find.text(
        'Suggestions are not validation limits; native code, mods, or later '
        'game versions may use other values.',
      ),
      findsOneWidget,
    );
    await tester.tap(
      find.byKey(const ValueKey('story-suggestion-AfterCinematic_Nyras-1')),
    );
    await tester.tap(find.byKey(const Key('story-queue-change')));
    await tester.pumpAndSettle();

    final edit = notifier.storyStateEditFor('aftercinematic_nyras');
    expect(edit?.present, isTrue);
    expect(edit?.rawValue, 1);
    expect(edit?.expectedStored, isFalse);
    expect(notifier.pendingEditCount, 1);
    expect(find.text('Will be stored as 1'), findsOneWidget);

    await _tapVisible(
      tester,
      find.byKey(const ValueKey('story-undo-AfterCinematic_Nyras')),
    );
    await tester.pumpAndSettle();
    expect(notifier.storyStateEditFor('AfterCinematic_Nyras'), isNull);
    expect(notifier.pendingEditCount, 0);
  });

  testWidgets('removes a stored value and resets only story changes', (
    tester,
  ) async {
    final notifier = EditorNotifier(_BenignCore(), saveDir: r'C:\tmp\saves');
    addTearDown(notifier.dispose);
    final page = StoryStatePage.fromJson({
      'total': 1,
      'storedTotal': 1,
      'catalogTotal': 1,
      'unsetTotal': 0,
      'writable': true,
      'entries': [
        {
          'id': 'Stone_ImprovedOreArmor',
          'stored': true,
          'catalogKnown': true,
          'rawValue': 1,
          'path': [
            'm_GenericData',
            '{Story}',
            'StoryPropertyValues',
            '{Stone_ImprovedOreArmor}',
          ],
          'semanticType': 'integer',
          'declaredType': 'int32',
        },
      ],
    });

    await _pumpEditableStoryPanel(tester, notifier: notifier, page: page);
    await tester.tap(find.text('Stone Improved Ore Armor'));
    await tester.pumpAndSettle();
    await _tapVisible(
      tester,
      find.byKey(const ValueKey('story-remove-Stone_ImprovedOreArmor')),
    );
    await tester.pumpAndSettle();

    final edit = notifier.storyStateEditFor('stone_improvedorearmor');
    expect(edit?.present, isFalse);
    expect(edit?.rawValue, isNull);
    expect(edit?.expectedStored, isTrue);
    expect(edit?.expectedRawValue, 1);
    expect(find.text('Will be removed from the save'), findsOneWidget);

    await tester.tap(find.byKey(const Key('story-state-reset')));
    await tester.pumpAndSettle();
    expect(notifier.allStoryStateEdits(), isEmpty);
    expect(notifier.pendingEditCount, 0);
  });

  testWidgets('sets a time marker to the current save time', (tester) async {
    final notifier = EditorNotifier(_BenignCore(), saveDir: r'C:\tmp\saves');
    addTearDown(notifier.dispose);
    final page = StoryStatePage.fromJson({
      'total': 1,
      'storedTotal': 1,
      'catalogTotal': 1,
      'unsetTotal': 0,
      'currentGameTimeSeconds': 1875587.9437,
      'writable': true,
      'entries': [
        {
          'id': 'Stone_OreArmor',
          'stored': true,
          'catalogKnown': true,
          'rawValue': 1767047,
          'path': ['StoryPropertyValues', '{Stone_OreArmor}'],
          'semanticType': 'timeMarker',
          'declaredType': 'FInGameTime',
        },
      ],
    });

    await _pumpEditableStoryPanel(tester, notifier: notifier, page: page);
    await tester.tap(find.text('Stone Ore Armor'));
    await tester.pumpAndSettle();
    await _tapVisible(
      tester,
      find.byKey(const ValueKey('story-edit-Stone_OreArmor')),
    );
    await tester.pumpAndSettle();
    await tester.tap(find.byKey(const Key('story-use-current-time')));
    await tester.tap(find.byKey(const Key('story-queue-change')));
    await tester.pumpAndSettle();

    final edit = notifier.storyStateEditFor('Stone_OreArmor');
    expect(edit?.rawValue, 1875587);
    expect(edit?.expectedRawValue, 1767047);
    expect(
      find.text('Will be stored as 21 / 16:59:47 (1875587)'),
      findsOneWidget,
    );
  });

  testWidgets('raw editor enforces the complete signed int32 range', (
    tester,
  ) async {
    final notifier = EditorNotifier(_BenignCore(), saveDir: r'C:\tmp\saves');
    addTearDown(notifier.dispose);
    final page = StoryStatePage.fromJson({
      'total': 1,
      'storedTotal': 1,
      'catalogTotal': 0,
      'unsetTotal': 0,
      'unknownStoredTotal': 1,
      'writable': true,
      'entries': [
        {
          'id': 'Mod_NewStoryValue',
          'stored': true,
          'catalogKnown': false,
          'rawValue': 7,
          'path': ['StoryPropertyValues', '{Mod_NewStoryValue}'],
          'semanticType': 'unknown',
          'declaredType': 'unknown',
        },
      ],
    });

    await _pumpEditableStoryPanel(tester, notifier: notifier, page: page);
    await tester.tap(find.text('Mod New Story Value'));
    await tester.pumpAndSettle();
    await _tapVisible(
      tester,
      find.byKey(const ValueKey('story-edit-Mod_NewStoryValue')),
    );
    await tester.pumpAndSettle();
    await tester.enterText(
      find.byKey(const Key('story-raw-value')),
      '2147483648',
    );
    await tester.tap(find.byKey(const Key('story-queue-change')));
    await tester.pumpAndSettle();
    expect(
      find.text('Enter a whole number from -2147483648 to 2147483647.'),
      findsOneWidget,
    );

    await tester.enterText(
      find.byKey(const Key('story-raw-value')),
      '-2147483648',
    );
    await tester.tap(find.byKey(const Key('story-queue-change')));
    await tester.pumpAndSettle();
    expect(
      notifier.storyStateEditFor('Mod_NewStoryValue')?.rawValue,
      -2147483648,
    );
  });

  testWidgets('same-save refresh disables edits until the fresh CAS page', (
    tester,
  ) async {
    final notifier = EditorNotifier(_BenignCore(), saveDir: r'C:\tmp\saves');
    addTearDown(notifier.dispose);
    const firstPage = StoryStatePage(
      values: [
        StoryStateValue(
          id: 'Stone_ImprovedOreArmor',
          value: 1,
          stored: true,
          catalogKnown: true,
          path: ['StoryPropertyValues', '{Stone_ImprovedOreArmor}'],
          semanticType: StorySemanticType.integer,
          declaredType: 'int32',
        ),
      ],
      total: 1,
      storedTotal: 1,
      catalogTotal: 1,
      writable: true,
    );
    const refreshedPage = StoryStatePage(
      values: [
        StoryStateValue(
          id: 'Stone_ImprovedOreArmor',
          value: 0,
          stored: true,
          catalogKnown: true,
          path: ['StoryPropertyValues', '{Stone_ImprovedOreArmor}'],
          semanticType: StorySemanticType.integer,
          declaredType: 'int32',
        ),
      ],
      total: 1,
      storedTotal: 1,
      catalogTotal: 1,
      writable: true,
    );
    final refresh = Completer<StoryStatePage>();
    var loads = 0;

    Widget app(SaveInspection inspection) => ProviderScope(
      overrides: [
        currentGameLangProvider.overrideWithValue(gameLangByCode('en')),
        locCatalogProvider.overrideWith((ref) async => const {}),
      ],
      child: MaterialApp(
        locale: const Locale('en'),
        localizationsDelegates: testLocalizationsDelegates,
        supportedLocales: AppLocalizations.supportedLocales,
        home: Scaffold(
          body: StoryStateDetail(
            notifier: notifier,
            editable: true,
            reloadKey: inspection,
            theme: ThemeData(),
            storyLoader:
                ({required offset, required limit, required path}) async {
                  loads++;
                  return loads == 1 ? firstPage : refresh.future;
                },
            npcCatalogLoader: () async => const [],
            segmentTextCatalogLoader: () async => const {},
          ),
        ),
      ),
    );

    await tester.pumpWidget(
      app(
        const SaveInspection(
          format: 'GSAV',
          path: r'C:\tmp\saves\story.sav',
          size: 1,
          sha1: 'before',
          raw: {},
        ),
      ),
    );
    await tester.pumpAndSettle();
    await tester.tap(find.text('Stone Improved Ore Armor'));
    await tester.pumpAndSettle();
    expect(
      find.byKey(const ValueKey('story-edit-Stone_ImprovedOreArmor')),
      findsOneWidget,
    );

    await tester.pumpWidget(
      app(
        const SaveInspection(
          format: 'GSAV',
          path: r'C:\tmp\saves\story.sav',
          size: 2,
          sha1: 'after',
          raw: {},
        ),
      ),
    );
    await tester.pump();
    expect(find.byType(LinearProgressIndicator), findsOneWidget);
    expect(
      find.byKey(const ValueKey('story-edit-Stone_ImprovedOreArmor')),
      findsNothing,
    );

    refresh.complete(refreshedPage);
    await tester.pumpAndSettle();
    expect(
      find.byKey(const ValueKey('story-edit-Stone_ImprovedOreArmor')),
      findsOneWidget,
    );
    expect(find.text('Raw value: 0'), findsWidgets);
  });

  testWidgets('distinguishes an unsafe story structure from codec read-only', (
    tester,
  ) async {
    final notifier = EditorNotifier(_BenignCore(), saveDir: r'C:\tmp\saves');
    addTearDown(notifier.dispose);

    await _pumpEditableStoryPanel(
      tester,
      notifier: notifier,
      page: const StoryStatePage(catalogTotal: 470, writable: false),
    );

    expect(
      find.text(
        'The StoryPropertyValues structure in this save could not be resolved '
        'uniquely and safely. Story values remain read-only for this save.',
      ),
      findsOneWidget,
    );
    expect(find.text('Codec read-only'), findsNothing);
  });

  testWidgets('a story load error does not also blame the codec or structure', (
    tester,
  ) async {
    final notifier = EditorNotifier(_BenignCore(), saveDir: r'C:\tmp\saves');
    addTearDown(notifier.dispose);

    await _pumpEditableStoryPanel(
      tester,
      notifier: notifier,
      page: const StoryStatePage(error: 'Ambiguous StoryPropertyValues path'),
      editable: false,
    );

    expect(find.text('Ambiguous StoryPropertyValues path'), findsOneWidget);
    expect(find.text('Codec read-only'), findsNothing);
    expect(
      find.textContaining('Story values remain read-only for this save.'),
      findsNothing,
    );
  });
}

Future<void> _pumpEditableStoryPanel(
  WidgetTester tester, {
  required EditorNotifier notifier,
  required StoryStatePage page,
  bool editable = true,
}) async {
  await tester.pumpWidget(
    ProviderScope(
      overrides: [
        currentGameLangProvider.overrideWithValue(gameLangByCode('en')),
        locCatalogProvider.overrideWith((ref) async => const {}),
      ],
      child: MaterialApp(
        locale: const Locale('en'),
        localizationsDelegates: testLocalizationsDelegates,
        supportedLocales: AppLocalizations.supportedLocales,
        home: Scaffold(
          body: StoryStateDetail(
            notifier: notifier,
            editable: editable,
            reloadKey: const SaveInspection(
              format: 'GSAV',
              path: r'C:\tmp\saves\story.sav',
              size: 1,
              sha1: 'sha1',
              raw: {},
            ),
            theme: ThemeData(),
            storyLoader:
                ({required offset, required limit, required path}) async =>
                    page,
            npcCatalogLoader: () async => const [],
            segmentTextCatalogLoader: () async => const {},
          ),
        ),
      ),
    ),
  );
  await tester.pumpAndSettle();
}

Future<void> _tapVisible(WidgetTester tester, Finder finder) async {
  await tester.ensureVisible(finder);
  await tester.pumpAndSettle();
  await tester.tap(finder);
  await tester.pumpAndSettle();
}

class _BenignCore implements GoresaveCoreService {
  @override
  String get description => 'story-test';

  @override
  bool get isAvailable => true;

  @override
  Future<Map<String, Object?>> execute(
    String command, {
    Map<String, Object?> payload = const {},
  }) async {
    if (command == 'scan_save_dir') {
      return {
        'ok': true,
        'data': {
          'saveRoot': payload['path'],
          'saves': <Object?>[],
          'profiles': <Object?>[],
        },
      };
    }
    if (command == 'check_codec') {
      return {
        'ok': true,
        'data': {
          'backend': 'test',
          'available': true,
          'canDecompress': true,
          'canCompress': true,
          'status': 'ready',
        },
      };
    }
    return {'ok': true, 'data': <String, Object?>{}};
  }
}
