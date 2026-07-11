import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/app/domain/ui_settings.dart';
import 'package:gore_mod/core/core_service.dart';
import 'package:gore_mod/core/providers.dart';
import 'package:gore_mod/export/ui/build_deploy_dialog.dart';
import 'package:gore_mod/gore_mod_app.dart';
import 'package:gore_mod/project/dialog_topics_notifier.dart';

void main() {
  testWidgets(
    'dialog-topic-only project enables the HomePage Build/Deploy gate without a game',
    (tester) async {
      tester.view.physicalSize = const Size(1600, 900);
      tester.view.devicePixelRatio = 1;
      addTearDown(tester.view.resetPhysicalSize);
      addTearDown(tester.view.resetDevicePixelRatio);

      final core = FakeGoreCoreFfiService(
        responses: const {
          // Suppress the optional first-run extraction prompt.
          'loc_status': {'ok': true, 'present': true},
          // Keep this test's configured-game branch definitively false.
          'find_game': {'ok': true, 'found': false},
        },
      );
      final container = ProviderContainer(
        overrides: [coreServiceProvider.overrideWithValue(core)],
      );
      addTearDown(container.dispose);

      await tester.pumpWidget(
        UncontrolledProviderScope(
          container: container,
          child: const GoreModApp(),
        ),
      );
      // Some inactive HomePage tabs own indeterminate progress animations;
      // fixed pumps let startup callbacks finish without waiting for those
      // intentionally non-settling animations.
      await tester.pump();
      await tester.pump(const Duration(milliseconds: 100));

      expect(container.read(gameExePathProvider), isNull);
      final buttonFinder = find.widgetWithText(FilledButton, 'Build / Deploy');
      expect(buttonFinder, findsOneWidget);
      expect(tester.widget<FilledButton>(buttonFinder).onPressed, isNull);

      container
          .read(dialogTopicsProvider.notifier)
          .setTopic(
            const DialogTopicDefinition(
              id: 'viper_fixture',
              participantName: 'om_viper_001',
              topicClass: '/Script/Angelscript.ChoiceGoreViperFixture',
              sentinelClass: '/Script/Angelscript.ChoiceViperVanilla',
            ),
          );
      await tester.pump();

      expect(container.read(gameExePathProvider), isNull);
      expect(tester.widget<FilledButton>(buttonFinder).onPressed, isNotNull);

      await tester.tap(buttonFinder);
      await tester.pump();
      await tester.pump(const Duration(milliseconds: 300));
      expect(find.byType(BuildDeployDialog), findsOneWidget);
    },
  );
}
