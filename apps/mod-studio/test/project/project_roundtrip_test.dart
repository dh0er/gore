import 'dart:io';

import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/audio/domain/audio_replacements_notifier.dart';
import 'package:gore_mod/editor/domain/override_entry.dart';
import 'package:gore_mod/project/dialog_topics_notifier.dart';
import 'package:gore_mod/project/project_io.dart';
import 'package:gore_mod/project/project_model.dart';

void main() {
  test('saveProject -> loadProject round-trips overrides, loc edits, audio', () async {
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
        OverrideEntry(classId: 'ItFo_Apple', field: 'm_Value', oldValue: 0, newValue: 500),
      ],
      locEdits: const {
        'itfo_cheese': {'german_new': 'Käse'},
      },
      audio: [
        AudioReplacement(bank: 'SFX.bank', sample: 'SFX_UI_X', wavPath: wav.path),
      ],
      dialogTopics: const [
        DialogTopicDefinition(
          id: 'viper_fixture',
          participantName: 'om_viper_001',
          topicClass: '/Script/Angelscript.ChoiceGoreViperFixture',
          sentinelClass: '/Script/Angelscript.ChoiceViperVanilla',
        ),
      ],
    );

    final out = '${tmp.path}/proj.goremod';
    await saveProject(project, out);
    expect(File(out).existsSync(), true);

    final loaded = await loadProject(out);
    expect(loaded.name, 'RoundTrip');
    expect(loaded.version, '2.0');
    expect(loaded.author, 'tester');
    expect(loaded.delayMs, 50);

    expect(loaded.overrides.length, 1);
    expect(loaded.overrides.single.classId, 'ItFo_Apple');
    expect(loaded.overrides.single.field, 'm_Value');
    expect(loaded.overrides.single.newValue, 500);

    expect(loaded.locEdits['itfo_cheese']!['german_new'], 'Käse');

    expect(loaded.audio.length, 1);
    expect(loaded.audio.single.bank, 'SFX.bank');
    expect(loaded.audio.single.sample, 'SFX_UI_X');
    // wav was embedded and extracted to a real file
    expect(File(loaded.audio.single.wavPath).existsSync(), true);
    expect(await File(loaded.audio.single.wavPath).readAsBytes(), List<int>.filled(64, 7));

    expect(loaded.dialogTopics.length, 1);
    expect(loaded.dialogTopics.single.id, 'viper_fixture');
    expect(loaded.dialogTopics.single.participantName, 'om_viper_001');
    expect(loaded.dialogTopics.single.topicClass, '/Script/Angelscript.ChoiceGoreViperFixture');
    expect(loaded.dialogTopics.single.sentinelClass, '/Script/Angelscript.ChoiceViperVanilla');

    // build spec carries all editor domains in FFI shape
    final spec = loaded.toBuildSpec();
    expect((spec['overrides'] as List).length, 1);
    expect((spec['audio'] as List).length, 1);
    expect((spec['loc_edits'] as Map).isNotEmpty, true);
    expect(spec['dialog_topics'], [
      {
        'id': 'viper_fixture',
        'participant_name': 'om_viper_001',
        'topic_class': '/Script/Angelscript.ChoiceGoreViperFixture',
        'sentinel_class': '/Script/Angelscript.ChoiceViperVanilla',
      }
    ]);
  });

  test('saveProject over an existing project replaces it and leaves no temp/backup', () async {
    final tmp = await Directory.systemTemp.createTemp('goremod_overwrite_');
    addTearDown(() => tmp.delete(recursive: true));
    final out = '${tmp.path}/proj.goremod';

    await saveProject(ModProject(name: 'First', version: '1.0'), out);
    // Overwrite the existing project with different content.
    await saveProject(ModProject(name: 'Second', version: '2.0'), out);

    final loaded = await loadProject(out);
    expect(loaded.name, 'Second');
    expect(loaded.version, '2.0');

    // The move-aside replace must clean up after itself — no leftover temp/backup siblings.
    expect(File('$out.bak').existsSync(), false, reason: 'backup left behind');
    expect(File('$out.tmp').existsSync(), false, reason: 'temp file left behind');
  });
}
