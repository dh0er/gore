import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/audio/domain/audio_replacements_notifier.dart';
import 'package:gore_mod/editor/domain/override_entry.dart';
import 'package:gore_mod/editor/domain/overrides_notifier.dart';
import 'package:gore_mod/editor/ui/overrides_panel.dart';
import 'package:gore_mod/l10n/app_localizations.dart';
import 'package:gore_mod/loc/domain/loc_edits_notifier.dart';
import 'package:gore_mod/project/dialog_topics_notifier.dart';
import 'package:gore_mod/scripts/domain/script_mods_notifier.dart';
import 'package:gore_mod/textures/domain/texture_replacements_notifier.dart';

void main() {
  const apple500 = OverrideEntry(
    classId: 'ItFo_Apple',
    field: 'm_Value',
    oldValue: 4,
    newValue: 500,
  );
  const sword = OverrideEntry(
    classId: 'ItMw_1H_Sword_01',
    field: 'm_Value',
    oldValue: 50,
    newValue: 200,
  );
  const boarGrunt = AudioReplacement(
    bank: 'creatures_bank',
    sample: 'boar_grunt_01',
    wavPath: r'C:\snd\grunt.wav',
  );
  const appleTexture = TextureReplacement(
    asset: 'Game/Textures/T_Apple_D',
    imagePath: r'C:\mods\apple_diffuse.png',
  );
  const fooScript = ScriptMod(
    op: ScriptOp.add,
    moduleName: 'FooMod',
    relPath: 'AI/FooMod.as',
    asPath: r'C:\mods\FooMod.as',
  );
  const viperTopic = DialogTopicDefinition(
    id: 'viper_fixture',
    participantName: 'om_stt_viper_302',
    topicClass: '/Script/Angelscript.ChoiceGoreViperFixture',
    sentinelClass: '/Script/Angelscript.ChoiceStt302ViperExit',
  );

  /// One change in every domain plus a second item override — all seeded via
  /// the pure notifiers (no FFI, no files: the script mod is uncompiled).
  ProviderContainer makeFullContainer() {
    final container = ProviderContainer();
    container.read(overridesProvider.notifier).setOverride(apple500);
    container.read(overridesProvider.notifier).setOverride(sword);
    container
        .read(locEditsProvider.notifier)
        .setEdit('info_aaron_001', 'de_A', 'Servus');
    container
        .read(audioReplacementsProvider.notifier)
        .setReplacement(boarGrunt);
    container
        .read(textureReplacementsProvider.notifier)
        .setReplacement(appleTexture);
    container.read(scriptModsProvider.notifier).setMod(fooScript);
    container.read(dialogTopicsProvider.notifier).setTopic(viperTopic);
    return container;
  }

  Future<void> pumpPanel(
    WidgetTester tester,
    ProviderContainer container,
  ) async {
    tester.view.physicalSize = const Size(1200, 1000);
    tester.view.devicePixelRatio = 1.0;
    addTearDown(tester.view.reset);
    await tester.pumpWidget(
      UncontrolledProviderScope(
        container: container,
        child: MaterialApp(
          localizationsDelegates: AppLocalizations.localizationsDelegates,
          supportedLocales: AppLocalizations.supportedLocales,
          home: const Scaffold(body: OverridesPanel()),
        ),
      ),
    );
    await tester.pumpAndSettle();
  }

  testWidgets('shows empty state message when no changes', (tester) async {
    final container = ProviderContainer();
    addTearDown(container.dispose);
    await pumpPanel(tester, container);
    expect(
      find.text('No pending overrides.\nEdit item fields to add some.'),
      findsOneWidget,
    );
  });

  testWidgets('shows override rows when overrides present', (tester) async {
    final container = ProviderContainer();
    addTearDown(container.dispose);
    container.read(overridesProvider.notifier).setOverride(apple500);
    container.read(overridesProvider.notifier).setOverride(sword);
    await pumpPanel(tester, container);
    expect(find.text('ItFo_Apple.m_Value'), findsOneWidget);
    expect(find.text('ItMw_1H_Sword_01.m_Value'), findsOneWidget);
    expect(find.text('4 → 500'), findsOneWidget);
    expect(find.text('50 → 200'), findsOneWidget);
  });

  testWidgets('remove button removes an override', (tester) async {
    final container = ProviderContainer();
    addTearDown(container.dispose);
    container.read(overridesProvider.notifier).setOverride(apple500);
    await pumpPanel(tester, container);
    await tester.tap(find.byIcon(Icons.remove_circle_outline));
    await tester.pumpAndSettle();
    expect(find.text('ItFo_Apple.m_Value'), findsNothing);
  });

  testWidgets('Clear all button removes changes in every domain', (
    tester,
  ) async {
    final container = makeFullContainer();
    addTearDown(container.dispose);
    await pumpPanel(tester, container);
    await tester.tap(find.text('Clear all'));
    await tester.pumpAndSettle();
    expect(
      find.text('No pending overrides.\nEdit item fields to add some.'),
      findsOneWidget,
    );
    expect(container.read(overridesProvider).count, 0);
    expect(container.read(locEditsProvider).entryCount, 0);
    expect(container.read(audioReplacementsProvider).count, 0);
    expect(container.read(textureReplacementsProvider).count, 0);
    expect(container.read(scriptModsProvider).count, 0);
    expect(container.read(dialogTopicsProvider).count, 0);
  });

  testWidgets('header shows search field instead of a Changes title', (
    tester,
  ) async {
    final container = makeFullContainer();
    addTearDown(container.dispose);
    await pumpPanel(tester, container);
    expect(find.textContaining('Changes ('), findsNothing);
    expect(find.text('Search changes'), findsOneWidget);
    expect(find.text('Clear all'), findsOneWidget);
  });

  testWidgets('search filters across all sections and hides empty ones', (
    tester,
  ) async {
    final container = makeFullContainer();
    addTearDown(container.dispose);
    await pumpPanel(tester, container);

    // All six section headers visible without a query.
    expect(find.text('Item values'), findsOneWidget);
    expect(find.text('Localized text'), findsOneWidget);
    expect(find.text('Audio'), findsOneWidget);
    expect(find.text('Textures'), findsOneWidget);
    expect(find.text('Scripts'), findsOneWidget);
    expect(find.text('Runtime dialog topics'), findsOneWidget);

    // "apple" matches the item override and the texture asset — nothing else.
    await tester.enterText(find.byType(TextField), 'apple');
    await tester.pumpAndSettle();
    expect(find.text('ItFo_Apple.m_Value'), findsOneWidget);
    expect(find.text('Game/Textures/T_Apple_D'), findsOneWidget);
    expect(find.text('ItMw_1H_Sword_01.m_Value'), findsNothing);
    expect(find.text('Item values'), findsOneWidget);
    expect(find.text('Textures'), findsOneWidget);
    // Sections with no matches disappear together with their headers.
    expect(find.text('Localized text'), findsNothing);
    expect(find.text('Audio'), findsNothing);
    expect(find.text('Scripts'), findsNothing);
    expect(find.text('Runtime dialog topics'), findsNothing);

    // Loc edits match on the staged text too (case-insensitive).
    await tester.enterText(find.byType(TextField), 'SERVUS');
    await tester.pumpAndSettle();
    expect(find.text('Localized text'), findsOneWidget);
    expect(find.text('info_aaron_001  ·  de_A'), findsOneWidget);
    expect(find.text('Item values'), findsNothing);
    expect(find.text('Textures'), findsNothing);

    // Runtime topics match every explicitly authored field.
    await tester.enterText(find.byType(TextField), 'choicestt302viperexit');
    await tester.pumpAndSettle();
    expect(find.text('Runtime dialog topics'), findsOneWidget);
    expect(find.textContaining('viper_fixture'), findsOneWidget);
    expect(find.text('Localized text'), findsNothing);

    // Scripts match on the game-relative path.
    await tester.enterText(find.byType(TextField), 'ai/foomod');
    await tester.pumpAndSettle();
    expect(find.text('Scripts'), findsOneWidget);
    expect(find.text('Localized text'), findsNothing);
  });

  testWidgets('no matches at all shows the no-changes-match message', (
    tester,
  ) async {
    final container = makeFullContainer();
    addTearDown(container.dispose);
    await pumpPanel(tester, container);
    await tester.enterText(find.byType(TextField), 'zzz_no_such_change');
    await tester.pumpAndSettle();
    expect(find.text('No changes match'), findsOneWidget);
    expect(find.text('Item values'), findsNothing);
    expect(find.text('Scripts'), findsNothing);
    // The underlying changes are untouched — only the view is filtered.
    expect(container.read(overridesProvider).count, 2);
  });

  testWidgets('section clear button clears only its own group', (tester) async {
    final container = makeFullContainer();
    addTearDown(container.dispose);
    await pumpPanel(tester, container);

    await tester.tap(find.byKey(const ValueKey('clear-section-audio')));
    await tester.pumpAndSettle();

    expect(container.read(audioReplacementsProvider).count, 0);
    expect(find.text('Audio'), findsNothing);
    // Every other group is untouched.
    expect(container.read(overridesProvider).count, 2);
    expect(container.read(locEditsProvider).entryCount, 1);
    expect(container.read(textureReplacementsProvider).count, 1);
    expect(container.read(scriptModsProvider).count, 1);
    expect(container.read(dialogTopicsProvider).count, 1);
    expect(find.text('Item values'), findsOneWidget);
    expect(find.text('Textures'), findsOneWidget);
    expect(find.text('Scripts'), findsOneWidget);
    expect(find.text('Runtime dialog topics'), findsOneWidget);
  });

  testWidgets(
    'section clear ignores the search filter (clears hidden rows too)',
    (tester) async {
      final container = makeFullContainer();
      addTearDown(container.dispose);
      await pumpPanel(tester, container);

      // Filter down to the apple override; the sword override is hidden.
      await tester.enterText(find.byType(TextField), 'apple');
      await tester.pumpAndSettle();
      expect(find.text('ItMw_1H_Sword_01.m_Value'), findsNothing);

      await tester.tap(find.byKey(const ValueKey('clear-section-items')));
      await tester.pumpAndSettle();

      // BOTH item overrides are gone, not just the visible one.
      expect(container.read(overridesProvider).count, 0);
      expect(find.text('Item values'), findsNothing);
      // Other groups keep their changes.
      expect(container.read(locEditsProvider).entryCount, 1);
      expect(container.read(audioReplacementsProvider).count, 1);
    },
  );

  testWidgets('runtime dialog topic row is individually removable', (
    tester,
  ) async {
    final container = ProviderContainer();
    addTearDown(container.dispose);
    container.read(dialogTopicsProvider.notifier).setTopic(viperTopic);
    await pumpPanel(tester, container);

    expect(find.text('Runtime dialog topics'), findsOneWidget);
    expect(
      find.textContaining('viper_fixture  ·  om_stt_viper_302'),
      findsOneWidget,
    );
    expect(
      find.textContaining('/Script/Angelscript.ChoiceGoreViperFixture'),
      findsOneWidget,
    );
    expect(
      find.textContaining('/Script/Angelscript.ChoiceStt302ViperExit'),
      findsOneWidget,
    );

    await tester.tap(
      find.byKey(const ValueKey('remove-dialog-topic-viper_fixture')),
    );
    await tester.pumpAndSettle();

    expect(container.read(dialogTopicsProvider).count, 0);
    expect(find.text('Runtime dialog topics'), findsNothing);
  });

  testWidgets('runtime topic section clear removes hidden topic rows too', (
    tester,
  ) async {
    final container = makeFullContainer();
    addTearDown(container.dispose);
    container
        .read(dialogTopicsProvider.notifier)
        .setTopic(
          const DialogTopicDefinition(
            id: 'asghan_fixture',
            participantName: 'om_test_asghan_001',
            topicClass: '/Script/Angelscript.ChoiceGoreAsghanFixture',
            sentinelClass: '/Script/Angelscript.ChoiceAsghanVanilla',
          ),
        );
    await pumpPanel(tester, container);

    await tester.enterText(find.byType(TextField), 'viper');
    await tester.pumpAndSettle();
    expect(find.textContaining('viper_fixture'), findsOneWidget);
    expect(find.textContaining('asghan_fixture'), findsNothing);

    await tester.tap(find.byKey(const ValueKey('clear-section-dialog-topics')));
    await tester.pumpAndSettle();

    expect(container.read(dialogTopicsProvider).count, 0);
    // Section clear is domain-specific; localization and every other domain remain.
    expect(container.read(locEditsProvider).entryCount, 1);
    expect(container.read(overridesProvider).count, 2);
  });
}
