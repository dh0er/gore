import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/audio/domain/audio_replacements_notifier.dart';
import 'package:gore_mod/editor/domain/override_entry.dart';
import 'package:gore_mod/project/dialog_topics_notifier.dart';
import 'package:gore_mod/project/project_model.dart';
import 'package:gore_mod/scripts/domain/script_mods_notifier.dart';
import 'package:gore_mod/textures/domain/texture_replacements_notifier.dart';
import 'package:gore_mod/voice/domain/voice_edits_notifier.dart';

void main() {
  group('ModProject.validateUniqueTargets', () {
    test('rejects case-folded override class and field collisions', () {
      final project = ModProject(
        name: 'duplicates',
        overrides: const [
          OverrideEntry(
            classId: 'ItFo_Apple',
            field: 'm_Value',
            oldValue: 0,
            newValue: 1,
          ),
          OverrideEntry(
            classId: 'itfo_apple',
            field: 'M_VALUE',
            oldValue: 0,
            newValue: 2,
          ),
        ],
      );

      expect(
        project.validateUniqueTargets,
        throwsFormatExceptionContaining('duplicate override target'),
      );
    });

    test('rejects case-folded audio deployment collisions', () {
      final project = ModProject(
        name: 'duplicates',
        audio: const [
          AudioReplacement(
            bank: 'SFX.bank',
            sample: 'SFX_UI_Click',
            wavPath: 'first.wav',
          ),
          AudioReplacement(
            bank: 'sfx.BANK',
            sample: 'SFX_UI_Click',
            wavPath: 'second.wav',
          ),
        ],
      );

      expect(
        project.validateUniqueTargets,
        throwsFormatExceptionContaining('duplicate audio target'),
      );
    });

    test('retains case-sensitive FSB5 sample identities', () {
      final project = ModProject(
        name: 'distinct samples',
        audio: const [
          AudioReplacement(
            bank: 'SFX.bank',
            sample: 'SFX_UI_Click',
            wavPath: 'first.wav',
          ),
          AudioReplacement(
            bank: 'sfx.BANK',
            sample: 'sfx_ui_click',
            wavPath: 'second.wav',
          ),
        ],
      );

      expect(project.validateUniqueTargets, returnsNormally);
    });

    test('rejects case-folded texture asset collisions', () {
      final project = ModProject(
        name: 'duplicates',
        textures: const [
          TextureReplacement(asset: '/Game/UI/T_Menu', imagePath: 'first.png'),
          TextureReplacement(asset: '/game/ui/t_menu', imagePath: 'second.png'),
        ],
      );

      expect(
        project.validateUniqueTargets,
        throwsFormatExceptionContaining('duplicate texture target'),
      );
    });

    test('normalizes script separators, dot segments, and case', () {
      final project = ModProject(
        name: 'duplicates',
        scripts: const [
          ScriptMod(
            op: ScriptOp.add,
            moduleName: 'FixtureOne',
            relPath: 'Mods/AI/../Fixture.as',
            asPath: 'first.as',
          ),
          ScriptMod(
            op: ScriptOp.add,
            moduleName: 'FixtureTwo',
            relPath: r'mods\.\fixture.AS',
            asPath: 'second.as',
          ),
        ],
      );

      expect(
        project.validateUniqueTargets,
        throwsFormatExceptionContaining('duplicate script target'),
      );
    });

    test('rejects case-folded dialog topic IDs', () {
      final project = ModProject(
        name: 'duplicates',
        dialogTopics: const [
          DialogTopicDefinition(
            id: 'Asghan_Greeting',
            participantName: 'asghan',
            topicClass: '/Script/Angelscript.TopicOne',
            sentinelClass: '/Script/Angelscript.Sentinel',
          ),
          DialogTopicDefinition(
            id: 'asghan_greeting',
            participantName: 'asghan',
            topicClass: '/Script/Angelscript.TopicTwo',
            sentinelClass: '/Script/Angelscript.Sentinel',
          ),
        ],
      );

      expect(
        project.validateUniqueTargets,
        throwsFormatExceptionContaining('duplicate dialog topic id'),
      );
    });

    test('delegates semantic and deployment validation for voice edits', () {
      final project = ModProject(
        name: 'duplicates',
        voice: [
          _voiceEdit(locale: 'de', oggPath: 'de.ogg'),
          _voiceEdit(locale: 'en', oggPath: 'en.ogg'),
        ],
      );

      expect(
        project.validateUniqueTargets,
        throwsFormatExceptionContaining('duplicate voice deployment target'),
      );
    });
  });

  test('serialization and build lowering preflight duplicate targets', () {
    final project = ModProject(
      name: 'duplicates',
      textures: const [
        TextureReplacement(asset: '/Game/T_Fixture', imagePath: 'one.png'),
        TextureReplacement(asset: '/game/t_fixture', imagePath: 'two.png'),
      ],
    );

    expect(
      project.toJson,
      throwsFormatExceptionContaining('duplicate texture target'),
    );
    expect(
      project.toBuildSpec,
      throwsFormatExceptionContaining('duplicate texture target'),
    );
  });

  test(
    'JSON decoding rejects duplicates before provider maps can collapse them',
    () {
      expect(
        () => ModProject.fromJson({
          'format': 1,
          'overrides': const [
            {'class': 'Npc_Fixture', 'field': 'm_Name', 'new': 'first'},
            {'class': 'npc_fixture', 'field': 'M_NAME', 'new': 'second'},
          ],
        }),
        throwsFormatExceptionContaining('duplicate override target'),
      );
    },
  );
}

Matcher throwsFormatExceptionContaining(String text) => throwsA(
  isA<FormatException>().having(
    (error) => error.message,
    'message',
    contains(text),
  ),
);

VoiceArchiveEdit _voiceEdit({
  required String locale,
  required String oggPath,
}) => VoiceArchiveEdit(
  locId: 'INFO_FIXTURE',
  locale: locale,
  archive: 'dialog.zip',
  operation: VoicePatchOperation.add,
  archivePath: 'Dialog/INFO_FIXTURE.ogg',
  oggPath: oggPath,
  observation: const VoiceArchiveObservation(
    archiveSize: 1024,
    archiveSha256:
        '0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef',
    memberProof: VoiceMemberProof.absent(),
  ),
);
