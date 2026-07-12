import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/dialog/ui/dialoge_tab.dart';
import 'package:gore_mod/l10n/app_localizations.dart';
import 'package:gore_mod/loc/domain/loc_catalog_provider.dart';
import 'package:gore_mod/project/dialog_topics_notifier.dart';

void main() {
  testWidgets(
    'runtime topics add, edit, and delete without inferring authored fields',
    (tester) async {
      tester.view.physicalSize = const Size(1400, 1000);
      tester.view.devicePixelRatio = 1;
      addTearDown(tester.view.reset);

      final container = ProviderContainer(
        overrides: [
          locCatalogProvider.overrideWith((ref) => Future.value(const {})),
        ],
      );
      addTearDown(container.dispose);
      await tester.pumpWidget(
        UncontrolledProviderScope(
          container: container,
          child: MaterialApp(
            localizationsDelegates: AppLocalizations.localizationsDelegates,
            supportedLocales: AppLocalizations.supportedLocales,
            home: const Scaffold(body: DialogeTab()),
          ),
        ),
      );
      await tester.pumpAndSettle();

      await tester.tap(find.text('Runtime dialog topics (0)'));
      await tester.pumpAndSettle();

      Future<void> addTopic({
        required String id,
        required String participant,
        required String topicClass,
        required String sentinelClass,
        bool allowHidden = false,
      }) async {
        await tester.tap(find.text('Add runtime topic'));
        await tester.pumpAndSettle();
        await tester.enterText(find.widgetWithText(TextField, 'Topic ID'), id);
        await tester.enterText(
          find.widgetWithText(TextField, 'Participant name'),
          participant,
        );
        await tester.enterText(
          find.widgetWithText(TextField, 'Topic class'),
          topicClass,
        );
        await tester.enterText(
          find.widgetWithText(TextField, 'Sentinel class'),
          sentinelClass,
        );
        if (allowHidden) {
          await tester.tap(
            find.byKey(const ValueKey('dialog-topic-allow-hidden')),
          );
          await tester.pumpAndSettle();
        }
        await tester.tap(find.widgetWithText(FilledButton, 'Add'));
        await tester.pumpAndSettle();
      }

      await addTopic(
        id: 'first',
        participant: 'om_stt_viper_302',
        topicClass: '/Script/Angelscript.ChoiceGoreFirst',
        sentinelClass: '/Script/Angelscript.ChoiceStt302ViperExit',
        allowHidden: true,
      );
      await addTopic(
        id: 'second',
        participant: 'om_test_asghan_001',
        topicClass: '/Script/Angelscript.ChoiceGoreSecond',
        sentinelClass: '/Script/Angelscript.ChoiceAsghanVanilla',
      );

      var topics = container.read(dialogTopicsProvider).entries;
      expect(topics.map((topic) => topic.id), ['first', 'second']);
      expect(topics.first.participantName, 'om_stt_viper_302');
      expect(topics.first.topicClass, '/Script/Angelscript.ChoiceGoreFirst');
      expect(
        topics.first.sentinelClass,
        '/Script/Angelscript.ChoiceStt302ViperExit',
      );
      expect(topics.first.allowHidden, isTrue);
      expect(topics.last.allowHidden, isFalse);
      expect(
        find.byTooltip('Topic may be hidden in its current state'),
        findsOneWidget,
      );
      expect(
        tester.getTopLeft(find.text('first')).dy,
        lessThan(tester.getTopLeft(find.text('second')).dy),
      );

      await tester.tap(find.byTooltip('Edit runtime dialog topic first'));
      await tester.pumpAndSettle();
      expect(
        tester
            .widget<CheckboxListTile>(
              find.byKey(const ValueKey('dialog-topic-allow-hidden')),
            )
            .value,
        isTrue,
      );
      await tester.enterText(
        find.widgetWithText(TextField, 'Topic ID'),
        'first-renamed',
      );
      await tester.enterText(
        find.widgetWithText(TextField, 'Participant name'),
        'om_stt_viper_999',
      );
      await tester.tap(find.widgetWithText(FilledButton, 'Save'));
      await tester.pumpAndSettle();

      topics = container.read(dialogTopicsProvider).entries;
      expect(topics.map((topic) => topic.id), ['first-renamed', 'second']);
      expect(topics.first.participantName, 'om_stt_viper_999');
      expect(topics.first.topicClass, '/Script/Angelscript.ChoiceGoreFirst');
      expect(
        topics.first.sentinelClass,
        '/Script/Angelscript.ChoiceStt302ViperExit',
      );
      expect(topics.first.allowHidden, isTrue);

      await tester.tap(
        find.byTooltip('Edit runtime dialog topic first-renamed'),
      );
      await tester.pumpAndSettle();
      await tester.tap(find.byKey(const ValueKey('dialog-topic-allow-hidden')));
      await tester.pumpAndSettle();
      await tester.tap(find.widgetWithText(FilledButton, 'Save'));
      await tester.pumpAndSettle();
      topics = container.read(dialogTopicsProvider).entries;
      expect(topics.first.allowHidden, isFalse);
      expect(
        find.byTooltip('Topic may be hidden in its current state'),
        findsNothing,
      );

      await tester.tap(find.byTooltip('Delete runtime dialog topic second'));
      await tester.pumpAndSettle();
      expect(find.text('Delete runtime dialog topic?'), findsOneWidget);
      await tester.tap(find.widgetWithText(FilledButton, 'Delete'));
      await tester.pumpAndSettle();

      topics = container.read(dialogTopicsProvider).entries;
      expect(topics.map((topic) => topic.id), ['first-renamed']);
      expect(find.text('Runtime dialog topics (1)'), findsOneWidget);
      expect(find.text('second'), findsNothing);
    },
  );

  testWidgets('runtime topic IDs are unique case-insensitively', (
    tester,
  ) async {
    tester.view.physicalSize = const Size(1400, 1000);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.reset);

    final container = ProviderContainer(
      overrides: [
        locCatalogProvider.overrideWith((ref) => Future.value(const {})),
      ],
    );
    addTearDown(container.dispose);
    container
        .read(dialogTopicsProvider.notifier)
        .setTopic(
          const DialogTopicDefinition(
            id: 'Known',
            participantName: 'om_stt_viper_302',
            topicClass: '/Script/Angelscript.ChoiceKnown',
            sentinelClass: '/Script/Angelscript.ChoiceKnownSentinel',
          ),
        );

    await tester.pumpWidget(
      UncontrolledProviderScope(
        container: container,
        child: MaterialApp(
          localizationsDelegates: AppLocalizations.localizationsDelegates,
          supportedLocales: AppLocalizations.supportedLocales,
          home: const Scaffold(body: DialogeTab()),
        ),
      ),
    );
    await tester.pumpAndSettle();
    await tester.tap(find.text('Runtime dialog topics (1)'));
    await tester.pumpAndSettle();
    await tester.tap(find.text('Add runtime topic'));
    await tester.pumpAndSettle();

    await tester.enterText(find.widgetWithText(TextField, 'Topic ID'), 'KNOWN');
    await tester.enterText(
      find.widgetWithText(TextField, 'Participant name'),
      'om_test_asghan_001',
    );
    await tester.enterText(
      find.widgetWithText(TextField, 'Topic class'),
      '/Script/Angelscript.ChoiceOther',
    );
    await tester.enterText(
      find.widgetWithText(TextField, 'Sentinel class'),
      '/Script/Angelscript.ChoiceOtherSentinel',
    );
    await tester.tap(find.widgetWithText(FilledButton, 'Add'));
    await tester.pumpAndSettle();

    expect(find.text('This topic ID already exists.'), findsOneWidget);
    expect(container.read(dialogTopicsProvider).count, 1);
  });

  testWidgets('runtime topics reject values the backend cannot build', (
    tester,
  ) async {
    tester.view.physicalSize = const Size(1400, 1000);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.reset);

    final container = ProviderContainer(
      overrides: [
        locCatalogProvider.overrideWith((ref) => Future.value(const {})),
      ],
    );
    addTearDown(container.dispose);
    await tester.pumpWidget(
      UncontrolledProviderScope(
        container: container,
        child: MaterialApp(
          localizationsDelegates: AppLocalizations.localizationsDelegates,
          supportedLocales: AppLocalizations.supportedLocales,
          home: const Scaffold(body: DialogeTab()),
        ),
      ),
    );
    await tester.pumpAndSettle();
    await tester.tap(find.text('Runtime dialog topics (0)'));
    await tester.pumpAndSettle();
    await tester.tap(find.text('Add runtime topic'));
    await tester.pumpAndSettle();

    await tester.enterText(find.widgetWithText(TextField, 'Topic ID'), 'bad');
    await tester.enterText(
      find.widgetWithText(TextField, 'Participant name'),
      'Display Name',
    );
    await tester.enterText(
      find.widgetWithText(TextField, 'Topic class'),
      '/Game/Topics/BP_Invalid_C',
    );
    await tester.enterText(
      find.widgetWithText(TextField, 'Sentinel class'),
      '/Game/Topics/BP_InvalidSentinel_C',
    );
    await tester.tap(find.widgetWithText(FilledButton, 'Add'));
    await tester.pumpAndSettle();

    expect(
      find.text(
        'Participant name must use 1-128 ASCII letters, digits, or underscores.',
      ),
      findsOneWidget,
    );
    expect(container.read(dialogTopicsProvider).count, 0);
  });
}
