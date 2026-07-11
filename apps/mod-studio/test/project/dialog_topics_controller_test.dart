import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/project/dialog_topics_notifier.dart';
import 'package:gore_mod/project/project_controller.dart';
import 'package:gore_mod/project/project_model.dart';

const first = DialogTopicDefinition(
  id: 'first',
  participantName: 'om_first_001',
  topicClass: '/Script/Angelscript.ChoiceFirst',
  sentinelClass: '/Script/Angelscript.ChoiceFirstVanilla',
);

const second = DialogTopicDefinition(
  id: 'second',
  participantName: 'om_second_001',
  topicClass: '/Script/Angelscript.ChoiceSecond',
  sentinelClass: '/Script/Angelscript.ChoiceSecondVanilla',
);

void main() {
  testWidgets(
    'dialog topics participate in dirty, gather, apply, saved baseline, and new',
    (tester) async {
      late WidgetRef ref;
      await tester.pumpWidget(
        ProviderScope(
          child: MaterialApp(
            home: Consumer(
              builder: (context, widgetRef, child) {
                ref = widgetRef;
                return Text('dirty=${projectIsDirty(widgetRef)}');
              },
            ),
          ),
        ),
      );

      expect(find.text('dirty=false'), findsOneWidget);
      expect(hasUnsavedChanges(ref), isFalse);

      ref.read(dialogTopicsProvider.notifier).setTopic(first);
      await tester.pump();
      expect(find.text('dirty=true'), findsOneWidget);
      expect(hasUnsavedChanges(ref), isTrue);
      expect(gatherProject(ref).dialogTopics.single.id, 'first');
      expect(gatherProject(ref).toBuildSpec()['dialog_topics'], [
        first.toJson(),
      ]);

      markProjectSaved(ref);
      expect(hasUnsavedChanges(ref), isFalse);
      ref.read(dialogTopicsProvider.notifier).setTopic(second);
      expect(hasUnsavedChanges(ref), isTrue);

      applyProject(
        ref,
        ModProject(name: 'Loaded', dialogTopics: const [second, first]),
      );
      expect(ref.read(dialogTopicsProvider).entries.map((topic) => topic.id), [
        'second',
        'first',
      ]);
      expect(gatherProject(ref).name, 'Loaded');
      markProjectSaved(ref);
      expect(hasUnsavedChanges(ref), isFalse);

      newProject(ref);
      await tester.pump();
      expect(ref.read(dialogTopicsProvider).count, 0);
      expect(find.text('dirty=false'), findsOneWidget);
      expect(hasUnsavedChanges(ref), isFalse);
    },
  );
}
