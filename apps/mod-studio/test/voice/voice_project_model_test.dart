import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/project/project_model.dart';
import 'package:gore_mod/voice/domain/voice_edits_notifier.dart';

const observation = VoiceArchiveObservation(
  archiveSize: 4096,
  archiveSha256:
      '0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef',
  memberProof: VoiceMemberProof.present(
    uncompressedSize: 321,
    crc32: 0x12345678,
  ),
);

const edit = VoiceArchiveEdit(
  locId: 'INFO_VIPER_HELLO',
  locale: 'de',
  archive: 'german_new.zip',
  operation: VoicePatchOperation.replace,
  archivePath: 'NPC/Viper/info_viper_hello.ogg',
  oggPath: r'C:\voices\viper.ogg',
  observation: observation,
);

void main() {
  test('format-1 JSON retains semantic identity and sealed observation', () {
    final project = ModProject(name: 'VoiceFixture', voice: const [edit]);

    final json = project.toJson();
    final raw = (json['voice'] as List).single as Map<String, Object?>;
    expect(raw['loc_id'], 'INFO_VIPER_HELLO');
    expect(raw['locale'], 'de');
    expect(raw['observation'], observation.toJson());

    final reopened = ModProject.fromJson(json);
    final voice = reopened.voice.single;
    expect(voice.locId, edit.locId);
    expect(voice.locale, edit.locale);
    expect(voice.archive, edit.archive);
    expect(voice.operation, VoicePatchOperation.replace);
    expect(voice.archivePath, edit.archivePath);
    expect(voice.oggPath, edit.oggPath);
    expect(voice.observation.archiveSize, 4096);
    expect(voice.observation.memberProof.state, VoiceMemberProofState.present);
    expect(voice.observation.memberProof.uncompressedSize, 321);
    expect(voice.observation.memberProof.crc32, 0x12345678);
  });

  test('BuildSpec lowers the exact sealed observation to gore-mod', () {
    final voice =
        (ModProject(
                      name: 'VoiceFixture',
                      voice: const [edit],
                    ).toBuildSpec()['voice']
                    as List)
                .single
            as Map<String, Object?>;

    expect(voice, {
      'archive': 'german_new.zip',
      'op': 'replace',
      'archive_path': 'NPC/Viper/info_viper_hello.ogg',
      'ogg_path': r'C:\voices\viper.ogg',
      'observation': {
        'archive_size': 4096,
        'archive_sha256':
            '0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef',
        'member_proof': {
          'state': 'present',
          'uncompressed_size': 321,
          'crc32': 0x12345678,
        },
      },
    });
    expect(voice.containsKey('loc_id'), isFalse);
    expect(voice.containsKey('locale'), isFalse);
  });

  test('draft add persists but production lowering rejects it', () {
    const addObservation = VoiceArchiveObservation(
      archiveSize: 8192,
      archiveSha256:
          'abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789',
      memberProof: VoiceMemberProof.absent(),
    );
    const add = VoiceArchiveEdit(
      locId: 'INFO_VIPER_NEW',
      locale: 'de',
      archive: 'german_new.zip',
      operation: VoicePatchOperation.add,
      archivePath: 'NPC/Viper/INFO_VIPER_NEW.ogg',
      oggPath: r'C:\voices\viper-new.ogg',
      observation: addObservation,
    );
    final project = ModProject(name: 'DraftVoiceAdd', voice: const [add]);

    final json = project.toJson();
    final reopened = ModProject.fromJson(json);
    expect(reopened.voice.single.operation, VoicePatchOperation.add);
    expect(reopened.voice.single.observation.toJson(), addObservation.toJson());
    expect(
      reopened.toBuildSpec,
      throwsA(
        isA<FormatException>().having(
          (error) => error.message,
          'message',
          allOf(
            contains('voice add cannot be built'),
            contains('new-member runtime binding is not qualified'),
          ),
        ),
      ),
    );
  });

  test('older format-1 projects without voice remain loadable', () {
    final project = ModProject.fromJson({
      'format': 1,
      'mod': {'name': 'Legacy', 'version': '', 'author': ''},
    });

    expect(project.voice, isEmpty);
    expect(project.toJson().containsKey('voice'), isFalse);
    expect(project.toBuildSpec()['voice'], isEmpty);
  });

  test('voice entries require canonical identity and coherent proof', () {
    Map<String, Object?> raw({
      String locale = 'de',
      String archivePath = 'NPC/Viper/info_viper_hello.ogg',
      String operation = 'replace',
      Map<String, Object?>? proof,
      int archiveSize = 4096,
    }) => {
      'loc_id': 'INFO_VIPER_HELLO',
      'locale': locale,
      'archive': 'german_new.zip',
      'op': operation,
      'archive_path': archivePath,
      'ogg_path': 'viper.ogg',
      'observation': {
        'archive_size': archiveSize,
        'archive_sha256':
            '0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef',
        'member_proof':
            proof ?? {'state': 'present', 'uncompressed_size': 321, 'crc32': 1},
      },
    };

    expect(
      () => VoiceArchiveEdit.fromJson(raw(locale: 'de-de')),
      throwsFormatException,
    );
    expect(
      () => VoiceArchiveEdit.fromJson(raw(locale: 'de-a')),
      throwsFormatException,
    );
    expect(
      () => VoiceArchiveEdit.fromJson(raw(locale: 'de-DE-DE')),
      throwsFormatException,
    );
    expect(
      VoiceArchiveEdit.fromJson(raw(locale: 'zh-Hans-CN')).locale,
      'zh-Hans-CN',
    );
    expect(
      () => VoiceArchiveEdit.fromJson(
        raw(archivePath: 'NPC/Viper/not_the_line.ogg'),
      ),
      throwsFormatException,
    );
    expect(
      () => VoiceArchiveEdit.fromJson(raw(operation: 'add')),
      throwsFormatException,
    );
    expect(
      () => VoiceArchiveEdit.fromJson(raw(archiveSize: 0)),
      throwsFormatException,
    );
    expect(
      () => VoiceArchiveEdit.fromJson(
        raw(proof: {'state': 'present', 'uncompressed_size': 0, 'crc32': 1}),
      ),
      throwsFormatException,
    );
    expect(
      () => VoiceArchiveEdit.fromJson(
        raw(
          archivePath:
              '${List<String>.filled(1020, 'x').join()}/info_viper_hello.ogg',
        ),
      ),
      throwsFormatException,
    );

    final withoutObservation = raw()..remove('observation');
    expect(
      () => VoiceArchiveEdit.fromJson(withoutObservation),
      throwsFormatException,
    );

    final add = VoiceArchiveEdit.fromJson(
      raw(operation: 'add', proof: {'state': 'absent'}),
    );
    expect(add.operation, VoicePatchOperation.add);
    expect(add.toJson()['observation'], {
      'archive_size': 4096,
      'archive_sha256':
          '0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef',
      'member_proof': {'state': 'absent'},
    });
  });

  test('programmatic duplicate lists cannot serialize or lower', () {
    final duplicate = VoiceArchiveEdit(
      locId: edit.locId.toLowerCase(),
      locale: edit.locale,
      archive: edit.archive,
      operation: edit.operation,
      archivePath: 'Other/${edit.locId.toLowerCase()}.ogg',
      oggPath: 'duplicate.ogg',
      observation: edit.observation,
    );
    final project = ModProject(name: 'Invalid', voice: [edit, duplicate]);

    expect(project.toJson, throwsFormatException);
    expect(project.toBuildSpec, throwsFormatException);
  });
}
