import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/export/ui/build_deploy_dialog.dart';
import 'package:gore_mod/project/dialog_topics_notifier.dart';

void main() {
  testWidgets('a runtime topic counts as buildable dialog content', (
    tester,
  ) async {
    tester.view.physicalSize = const Size(1000, 900);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.reset);

    final container = ProviderContainer();
    addTearDown(container.dispose);
    container
        .read(dialogTopicsProvider.notifier)
        .setTopic(
          const DialogTopicDefinition(
            id: 'viper_fixture',
            participantName: 'om_stt_viper_302',
            topicClass: '/Script/Angelscript.ChoiceGoreViperFixture',
            sentinelClass: '/Script/Angelscript.ChoiceStt302ViperExit',
          ),
        );

    await tester.pumpWidget(
      UncontrolledProviderScope(
        container: container,
        child: const MaterialApp(home: Scaffold(body: BuildDeployDialog())),
      ),
    );
    await tester.pumpAndSettle();

    expect(find.textContaining('1 runtime dialog topic'), findsOneWidget);
    final buildButton = tester.widget<OutlinedButton>(
      find.byType(OutlinedButton),
    );
    expect(buildButton.onPressed, isNotNull);
  });
}
