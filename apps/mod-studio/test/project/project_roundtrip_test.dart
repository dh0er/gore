import 'dart:io';

import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/audio/domain/audio_replacements_notifier.dart';
import 'package:gore_mod/editor/domain/override_entry.dart';
import 'package:gore_mod/project/dialog_topics_notifier.dart';
import 'package:gore_mod/project/project_io.dart';
import 'package:gore_mod/project/project_model.dart';

void main() {
  test(
    'saveProject -> loadProject round-trips overrides, loc edits, audio',
    () async {
      final tmp = await Directory.systemTemp.createTemp('goremod_test_');
      addTearDown(() => tmp.delete(recursive: true));

      // a source wav to embed
      final wav = File('${tmp.path}/tone.wav');
      await wav.writeAsBytes(List<int>.filled(64, 7));

      final project = ModProject(
        name: 'RoundTrip',
        version: '2.0',
        author: 'tester',
        delayMs: 50,
        overrides: const [
          OverrideEntry(
            classId: 'ItFo_Apple',
            field: 'm_Value',
            oldValue: 0,
            newValue: 500,
          ),
        ],
        locEdits: const {
          'itfo_cheese': {'german_new': 'Käse'},
        },
        audio: [
          AudioReplacement(
            bank: 'SFX.bank',
            sample: 'SFX_UI_X',
            wavPath: wav.path,
          ),
        ],
        dialogTopics: const [
          DialogTopicDefinition(
            id: 'viper_fixture',
            participantName: 'om_viper_001',
            topicClass: '/Script/Angelscript.ChoiceGoreViperFixture',
            sentinelClass: '/Script/Angelscript.ChoiceViperVanilla',
            allowHidden: true,
          ),
        ],
      );

      final out = '${tmp.path}/proj.goremod';
      await saveProject(project, out);
      expect(File(out).existsSync(), true);

      final loaded = await loadProject(out);
      addTearDown(() async => loaded.workspace?.release());
      final opened = loaded.project;
      expect(opened.name, 'RoundTrip');
      expect(opened.version, '2.0');
      expect(opened.author, 'tester');
      expect(opened.delayMs, 50);

      expect(opened.overrides.length, 1);
      expect(opened.overrides.single.classId, 'ItFo_Apple');
      expect(opened.overrides.single.field, 'm_Value');
      expect(opened.overrides.single.newValue, 500);

      expect(opened.locEdits['itfo_cheese']!['german_new'], 'Käse');

      expect(opened.audio.length, 1);
      expect(opened.audio.single.bank, 'SFX.bank');
      expect(opened.audio.single.sample, 'SFX_UI_X');
      // wav was embedded and extracted to a real file
      expect(File(opened.audio.single.wavPath).existsSync(), true);
      expect(
        await File(opened.audio.single.wavPath).readAsBytes(),
        List<int>.filled(64, 7),
      );

      expect(opened.dialogTopics.length, 1);
      expect(opened.dialogTopics.single.id, 'viper_fixture');
      expect(opened.dialogTopics.single.participantName, 'om_viper_001');
      expect(
        opened.dialogTopics.single.topicClass,
        '/Script/Angelscript.ChoiceGoreViperFixture',
      );
      expect(
        opened.dialogTopics.single.sentinelClass,
        '/Script/Angelscript.ChoiceViperVanilla',
      );
      expect(opened.dialogTopics.single.allowHidden, isTrue);

      // build spec carries all editor domains in FFI shape
      final spec = opened.toBuildSpec();
      expect((spec['overrides'] as List).length, 1);
      expect((spec['audio'] as List).length, 1);
      expect((spec['loc_edits'] as Map).isNotEmpty, true);
      expect(spec['dialog_topics'], [
        {
          'id': 'viper_fixture',
          'participant_name': 'om_viper_001',
          'topic_class': '/Script/Angelscript.ChoiceGoreViperFixture',
          'sentinel_class': '/Script/Angelscript.ChoiceViperVanilla',
          'allow_hidden': true,
        },
      ]);
    },
  );

  test(
    'saveProject over an existing project replaces it and leaves no temp/backup',
    () async {
      final tmp = await Directory.systemTemp.createTemp('goremod_overwrite_');
      addTearDown(() => tmp.delete(recursive: true));
      final out = '${tmp.path}/proj.goremod';

      await saveProject(ModProject(name: 'First', version: '1.0'), out);
      // Overwrite the existing project with different content.
      await saveProject(ModProject(name: 'Second', version: '2.0'), out);

      final loaded = await loadProject(out);
      addTearDown(() async => loaded.workspace?.release());
      expect(loaded.project.name, 'Second');
      expect(loaded.project.version, '2.0');

      // The move-aside replace must clean up after itself — no leftover temp/backup siblings.
      expect(
        File('$out.bak').existsSync(),
        false,
        reason: 'backup left behind',
      );
      expect(
        File('$out.tmp').existsSync(),
        false,
        reason: 'temp file left behind',
      );
    },
  );
}
