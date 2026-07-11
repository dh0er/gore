import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/project/dialog_topics_notifier.dart';
import 'package:gore_mod/project/project_model.dart';

const first = DialogTopicDefinition(
  id: 'first fixture',
  participantName: 'om_target_001',
  topicClass: '/Script/Angelscript.ChoiceAuthoredFirst',
  sentinelClass: '/Script/Angelscript.ChoiceVanillaFirst',
);

const second = DialogTopicDefinition(
  id: 'second fixture',
  participantName: 'OM_TARGET_002',
  topicClass: '/Script/Angelscript.ChoiceAuthoredSecond',
  sentinelClass: '/Script/Angelscript.ChoiceVanillaSecond',
);

void main() {
  test(
    'format-1 JSON and BuildSpec round-trip exact dialog topic fields in order',
    () {
      final project = ModProject(
        name: 'Dialogs',
        dialogTopics: const [first, second],
      );

      final json = project.toJson();
      expect(json['format'], 1);
      expect(json['dialog_topics'], [first.toJson(), second.toJson()]);

      final loaded = ModProject.fromJson(json);
      expect(loaded.dialogTopics.map((topic) => topic.id), [
        'first fixture',
        'second fixture',
      ]);
      expect(loaded.dialogTopics[1].participantName, 'OM_TARGET_002');
      expect(
        loaded.dialogTopics[1].topicClass,
        '/Script/Angelscript.ChoiceAuthoredSecond',
      );
      expect(
        loaded.dialogTopics[1].sentinelClass,
        '/Script/Angelscript.ChoiceVanillaSecond',
      );
      expect(loaded.toBuildSpec()['dialog_topics'], [
        first.toJson(),
        second.toJson(),
      ]);
    },
  );

  test('older format-1 JSON without dialog_topics remains loadable', () {
    final loaded = ModProject.fromJson({
      'format': 1,
      'mod': {'name': 'Legacy', 'version': '1.0', 'author': 'old'},
      'delay_ms': 0,
      'overrides': <Object?>[],
      'loc_edits': <String, Object?>{},
      'audio': <Object?>[],
      'textures': <Object?>[],
      'scripts': <Object?>[],
    });

    expect(loaded.name, 'Legacy');
    expect(loaded.dialogTopics, isEmpty);
    expect(loaded.toBuildSpec()['dialog_topics'], isEmpty);
  });
}
