import 'dart:convert';
import 'dart:io';

import 'package:archive/archive_io.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/project/project_io.dart';
import 'package:gore_mod/project/project_model.dart';
import 'package:gore_mod/voice/domain/voice_edits_notifier.dart';
import 'package:path/path.dart' as p;

const observation = VoiceArchiveObservation(
  archiveSize: 8192,
  archiveSha256:
      'cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc',
  memberProof: VoiceMemberProof.present(uncompressedSize: 128, crc32: 1234),
);

VoiceArchiveEdit edit(String oggPath) => VoiceArchiveEdit(
  locId: 'INFO_VIPER_IO',
  locale: 'de',
  archive: 'german_new.zip',
  operation: VoicePatchOperation.replace,
  archivePath: 'NPC/Viper/info_viper_io.ogg',
  oggPath: oggPath,
  observation: observation,
);

void registerWorkspaceCleanup(LoadedProject loaded) {
  addTearDown(() async {
    await loaded.workspace?.release();
  });
}

void main() {
  test(
    'save/load embeds exact Ogg bytes and retains authoring metadata',
    () async {
      final tmp = await Directory.systemTemp.createTemp('goremod_voice_io_');
      addTearDown(() => tmp.delete(recursive: true));
      final bytes = <int>[0x4f, 0x67, 0x67, 0x53, ...List<int>.filled(124, 7)];
      final source = File(p.join(tmp.path, 'take.ogg'));
      await source.writeAsBytes(bytes);
      final out = p.join(tmp.path, 'voice.goremod');

      await saveProject(
        ModProject(name: 'VoiceRoundTrip', voice: [edit(source.path)]),
        out,
      );

      final archive = ZipDecoder().decodeBytes(await File(out).readAsBytes());
      final projectJson =
          jsonDecode(
                utf8.decode(
                  archive.findFile('project.json')!.content as List<int>,
                ),
              )
              as Map<String, Object?>;
      final rawVoice = (projectJson['voice'] as List).single as Map;
      final embeddedPath = rawVoice['ogg_path'] as String;
      expect(embeddedPath, 'assets/voice/00000000.ogg');
      expect(archive.findFile(embeddedPath)!.content as List<int>, bytes);

      await source.delete();
      final loaded = await loadProject(out);
      registerWorkspaceCleanup(loaded);
      expect(loaded.project.voice, hasLength(1));
      expect(loaded.project.voice.single.locId, 'INFO_VIPER_IO');
      expect(loaded.project.voice.single.locale, 'de');
      expect(loaded.project.voice.single.observation.archiveSize, 8192);
      expect(
        await File(loaded.project.voice.single.oggPath).readAsBytes(),
        bytes,
      );
    },
  );

  test(
    'missing source aborts save without replacing the last good file',
    () async {
      final tmp = await Directory.systemTemp.createTemp('goremod_voice_save_');
      addTearDown(() => tmp.delete(recursive: true));
      final out = p.join(tmp.path, 'voice.goremod');
      await saveProject(ModProject(name: 'LastGood'), out);
      final before = await File(out).readAsBytes();

      await expectLater(
        saveProject(
          ModProject(
            name: 'MustFail',
            voice: [edit(p.join(tmp.path, 'missing.ogg'))],
          ),
          out,
        ),
        throwsA(isA<FileSystemException>()),
      );

      expect(await File(out).readAsBytes(), before);
      final loaded = await loadProject(out);
      registerWorkspaceCleanup(loaded);
      expect(loaded.project.name, 'LastGood');
    },
  );

  test(
    'save captures an immutable project snapshot before async I/O',
    () async {
      final tmp = await Directory.systemTemp.createTemp(
        'goremod_voice_snapshot_',
      );
      addTearDown(() => tmp.delete(recursive: true));
      final source = File(p.join(tmp.path, 'take.ogg'));
      await source.writeAsBytes(<int>[0x4f, 0x67, 0x67, 0x53, 1, 2, 3]);
      final voice = <VoiceArchiveEdit>[edit(source.path)];
      final output = p.join(tmp.path, 'snapshot.goremod');

      final saving = saveProject(
        ModProject(name: 'Captured', voice: voice),
        output,
      );
      voice.clear();
      await saving;

      final loaded = await loadProject(output);
      registerWorkspaceCleanup(loaded);
      expect(loaded.project.voice.single.locId, 'INFO_VIPER_IO');
    },
  );

  test(
    'missing embedded Ogg aborts load and preserves prior extraction',
    () async {
      final tmp = await Directory.systemTemp.createTemp('goremod_voice_load_');
      addTearDown(() => tmp.delete(recursive: true));
      final source = File(p.join(tmp.path, 'take.ogg'));
      await source.writeAsBytes(<int>[0x4f, 0x67, 0x67, 0x53, 1, 2, 3]);
      final goodPath = p.join(tmp.path, 'good.goremod');
      await saveProject(
        ModProject(name: 'Good', voice: [edit(source.path)]),
        goodPath,
      );
      final good = await loadProject(goodPath);
      registerWorkspaceCleanup(good);
      final priorExtraction = File(good.project.voice.single.oggPath);
      expect(await priorExtraction.exists(), isTrue);

      final badProject = ModProject(
        name: 'Bad',
        voice: [edit('assets/voice/0_missing.ogg')],
      );
      final projectBytes = utf8.encode(jsonEncode(badProject.toJson()));
      final badArchive = Archive()
        ..addFile(
          ArchiveFile('project.json', projectBytes.length, projectBytes),
        );
      final badPath = p.join(tmp.path, 'bad.goremod');
      await File(badPath).writeAsBytes(ZipEncoder().encode(badArchive)!);

      await expectLater(loadProject(badPath), throwsFormatException);
      expect(await priorExtraction.exists(), isTrue);
      expect(await priorExtraction.readAsBytes(), <int>[
        0x4f,
        0x67,
        0x67,
        0x53,
        1,
        2,
        3,
      ]);
    },
  );
}
