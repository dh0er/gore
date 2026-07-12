import 'dart:convert';
import 'dart:io';
import 'dart:typed_data';

import 'package:archive/archive_io.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/audio/domain/audio_replacements_notifier.dart';
import 'package:gore_mod/editor/domain/override_entry.dart';
import 'package:gore_mod/project/dialog_topics_notifier.dart';
import 'package:gore_mod/project/project_io.dart';
import 'package:gore_mod/project/project_model.dart';
import 'package:gore_mod/scripts/domain/script_mods_notifier.dart';
import 'package:gore_mod/textures/domain/texture_replacements_notifier.dart';
import 'package:gore_mod/voice/domain/voice_edits_notifier.dart';
import 'package:path/path.dart' as p;

const _voiceObservation = VoiceArchiveObservation(
  archiveSize: 4096,
  archiveSha256:
      'abababababababababababababababababababababababababababababababab',
  memberProof: VoiceMemberProof.present(uncompressedSize: 64, crc32: 42),
);

void main() {
  test('all embedded domains use canonical paths and round-trip', () async {
    final sourceRoot = await Directory.systemTemp.createTemp(
      'goremod_archive_sources_',
    );
    addTearDown(() async {
      if (await sourceRoot.exists()) await sourceRoot.delete(recursive: true);
    });

    final wavBytes = <int>[...ascii.encode('RIFF'), ...List<int>.filled(28, 1)];
    final pngBytes = <int>[
      0x89,
      0x50,
      0x4e,
      0x47,
      0x0d,
      0x0a,
      0x1a,
      0x0a,
      ...List<int>.filled(24, 2),
    ];
    final scriptBytes = utf8.encode('void Fixture() {\n  return;\n}\n');
    final miniBytes = <int>[0x47, 0x4f, 0x52, 0x45, 3, 4, 5];
    final oggBytes = <int>[...ascii.encode('OggS'), ...List<int>.filled(28, 6)];

    final wav = await _writeSource(sourceRoot, 'fixture.wav', wavBytes);
    final png = await _writeSource(sourceRoot, 'fixture.png', pngBytes);
    final script = await _writeSource(sourceRoot, 'Fixture.as', scriptBytes);
    final mini = await _writeSource(sourceRoot, 'Fixture.mini', miniBytes);
    final ogg = await _writeSource(sourceRoot, 'fixture.ogg', oggBytes);
    final output = p.join(sourceRoot.path, 'all-domains.goremod');

    await saveProject(
      ModProject(
        name: 'AllDomains',
        version: '1.2.3',
        author: 'fixture',
        delayMs: 25,
        overrides: const [
          OverrideEntry(
            classId: 'ItFo_Apple',
            field: 'm_Value',
            oldValue: 0,
            newValue: 25,
          ),
        ],
        locEdits: const {
          'INFO_VIPER_FIXTURE': {'german_new': 'Testzeile'},
        },
        audio: [
          AudioReplacement(
            bank: 'SFX.bank',
            sample: 'SFX_TEST',
            wavPath: wav.path,
          ),
        ],
        textures: [
          TextureReplacement(asset: '/Game/UI/T_Fixture', imagePath: png.path),
        ],
        scripts: [
          ScriptMod(
            op: ScriptOp.add,
            moduleName: 'Fixture',
            relPath: 'Mods/Fixture.as',
            asPath: script.path,
            miniPath: mini.path,
          ),
        ],
        dialogTopics: const [
          DialogTopicDefinition(
            id: 'fixture',
            participantName: 'om_viper_001',
            topicClass: '/Script/Angelscript.ChoiceFixture',
            sentinelClass: '/Script/Angelscript.ChoiceVanilla',
          ),
        ],
        voice: [
          VoiceArchiveEdit(
            locId: 'INFO_VIPER_FIXTURE',
            locale: 'de',
            archive: 'german_new.zip',
            operation: VoicePatchOperation.replace,
            archivePath: 'NPC/Viper/info_viper_fixture.ogg',
            oggPath: ogg.path,
            observation: _voiceObservation,
          ),
        ],
      ),
      output,
    );

    final archive = ZipDecoder().decodeBytes(await File(output).readAsBytes());
    expect(archive.files.map((file) => file.name).toSet(), {
      'assets/audio/00000000.wav',
      'assets/textures/00000000.png',
      'assets/scripts/00000000.as',
      'assets/scripts_cache/00000000.mini',
      'assets/voice/00000000.ogg',
      'project.json',
    });
    final projectJson =
        jsonDecode(
              utf8.decode(
                archive.findFile('project.json')!.content as List<int>,
              ),
            )
            as Map<String, Object?>;
    expect(
      (projectJson['audio'] as List).single['wav_path'],
      'assets/audio/00000000.wav',
    );
    expect(
      (projectJson['textures'] as List).single['image_path'],
      'assets/textures/00000000.png',
    );
    expect(
      (projectJson['scripts'] as List).single['as_path'],
      'assets/scripts/00000000.as',
    );
    expect(
      (projectJson['scripts'] as List).single['mini_path'],
      'assets/scripts_cache/00000000.mini',
    );
    expect(
      (projectJson['voice'] as List).single['ogg_path'],
      'assets/voice/00000000.ogg',
    );

    final loaded = await loadProject(output);
    _registerWorkspaceCleanup(loaded);
    final opened = loaded.project;
    final extractionRoot = _extractionRoot(opened.audio.single.wavPath);
    expect(p.basename(extractionRoot.path), startsWith('goremod_loaded_'));
    for (final assetPath in [
      opened.audio.single.wavPath,
      opened.textures.single.imagePath,
      opened.scripts.single.asPath,
      opened.scripts.single.miniPath,
      opened.voice.single.oggPath,
    ]) {
      expect(p.isWithin(extractionRoot.path, assetPath), isTrue);
    }
    expect(await File(opened.audio.single.wavPath).readAsBytes(), wavBytes);
    expect(
      await File(opened.textures.single.imagePath).readAsBytes(),
      pngBytes,
    );
    expect(await File(opened.scripts.single.asPath).readAsBytes(), scriptBytes);
    expect(await File(opened.scripts.single.miniPath).readAsBytes(), miniBytes);
    expect(await File(opened.voice.single.oggPath).readAsBytes(), oggBytes);
    expect(opened.overrides.single.newValue, 25);
    expect(opened.dialogTopics.single.id, 'fixture');
  });

  test('legacy Unicode basenames load into canonical local paths', () async {
    final fixture = await _fixtureDirectory('goremod_legacy_names_');
    const audioPath = 'assets/audio/0_Stimme_ä.wav';
    const sourcePath = 'assets/scripts/0_Änderung.as';
    const miniPath = 'assets/scripts_cache/0_Änderung.cache';
    final audioBytes = <int>[...ascii.encode('RIFF'), 1, 2, 3, 4];
    final sourceBytes = utf8.encode('void Legacy() {}');
    final miniBytes = <int>[7, 8, 9];
    final project = ModProject(
      name: 'LegacyNames',
      audio: const [
        AudioReplacement(
          bank: 'SFX.bank',
          sample: 'SFX_LEGACY',
          wavPath: audioPath,
        ),
      ],
      scripts: const [
        ScriptMod(
          op: ScriptOp.add,
          moduleName: 'Legacy',
          relPath: 'Mods/Legacy.as',
          asPath: sourcePath,
          miniPath: miniPath,
        ),
      ],
    );
    final path = p.join(fixture.path, 'legacy.goremod');
    await _writeZip(path, [
      _stored('project.json', _projectJsonBytes(project)),
      _stored(audioPath, audioBytes),
      _stored(sourcePath, sourceBytes),
      _stored(miniPath, miniBytes),
    ]);

    final loaded = await loadProject(path);
    _registerWorkspaceCleanup(loaded);
    final opened = loaded.project;
    final root = _extractionRoot(opened.audio.single.wavPath);
    expect(
      p.relative(opened.audio.single.wavPath, from: root.path),
      p.join('assets', 'audio', '00000000.wav'),
    );
    expect(
      p.relative(opened.scripts.single.asPath, from: root.path),
      p.join('assets', 'scripts', '00000000.as'),
    );
    expect(
      p.relative(opened.scripts.single.miniPath, from: root.path),
      p.join('assets', 'scripts_cache', '00000000.mini'),
    );
    expect(await File(opened.audio.single.wavPath).readAsBytes(), audioBytes);
    expect(await File(opened.scripts.single.asPath).readAsBytes(), sourceBytes);
    expect(await File(opened.scripts.single.miniPath).readAsBytes(), miniBytes);
  });

  test('rejects case-insensitive duplicate ZIP entry names', () async {
    final fixture = await _fixtureDirectory('goremod_zip_duplicate_');
    final projectBytes = _projectJsonBytes(ModProject(name: 'Duplicate'));
    final path = p.join(fixture.path, 'duplicate.goremod');
    await _writeZip(path, [
      _stored('project.json', projectBytes),
      _stored('PROJECT.JSON', projectBytes),
    ]);

    await expectLater(
      loadProject(path),
      throwsA(
        isA<FormatException>().having(
          (error) => error.message,
          'message',
          contains('duplicate case-insensitive ZIP entry'),
        ),
      ),
    );
  });

  test('rejects entries not referenced by project.json', () async {
    final fixture = await _fixtureDirectory('goremod_zip_unreferenced_');
    final path = p.join(fixture.path, 'unreferenced.goremod');
    await _writeZip(path, [
      _stored('project.json', _projectJsonBytes(ModProject(name: 'Clean'))),
      _stored('assets/unused.bin', <int>[1, 2, 3]),
    ]);

    await expectLater(
      loadProject(path),
      throwsA(
        isA<FormatException>().having(
          (error) => error.message,
          'message',
          contains('unreferenced entries'),
        ),
      ),
    );
  });

  test('rejects CRC corruption and a truncated terminal EOCD', () async {
    final fixture = await _fixtureDirectory('goremod_zip_corrupt_');
    final projectBytes = _projectJsonBytes(ModProject(name: 'Intact'));
    final valid = _encodeZip([_stored('project.json', projectBytes)]);

    final corrupt = Uint8List.fromList(valid);
    final payloadOffset = _indexOf(corrupt, projectBytes);
    expect(
      payloadOffset,
      isNonNegative,
      reason: 'stored JSON payload not found',
    );
    corrupt[payloadOffset] ^= 0x01;
    final corruptPath = p.join(fixture.path, 'crc.goremod');
    await File(corruptPath).writeAsBytes(corrupt);
    await expectLater(
      loadProject(corruptPath),
      throwsA(
        isA<FormatException>().having(
          (error) => error.message,
          'message',
          contains('CRC mismatch'),
        ),
      ),
    );

    final truncatedPath = p.join(fixture.path, 'truncated.goremod');
    await File(truncatedPath).writeAsBytes(valid.sublist(0, valid.length - 1));
    await expectLater(
      loadProject(truncatedPath),
      throwsA(
        isA<FormatException>().having(
          (error) => error.message,
          'message',
          contains('no exact terminal EOCD'),
        ),
      ),
    );
  });

  test('bounds forged DEFLATE output before trusting declared size', () async {
    final fixture = await _fixtureDirectory('goremod_zip_expansion_');
    final largeJson = _projectJsonBytes(
      ModProject(name: List<String>.filled(8192, 'A').join()),
    );
    final forged = Uint8List.fromList(
      _encodeZip([ArchiveFile('project.json', largeJson.length, largeJson)]),
    );
    final local = _findSignature(forged, 0x04034b50);
    final central = _findSignature(forged, 0x02014b50);
    expect(local, isNonNegative);
    expect(central, isNonNegative);
    expect(_readUint16(forged, local + 8), 8, reason: 'fixture is DEFLATE');
    _writeUint32(forged, local + 22, 1);
    _writeUint32(forged, central + 24, 1);
    final path = p.join(fixture.path, 'forged-size.goremod');
    await File(path).writeAsBytes(forged, flush: true);

    await expectLater(
      loadProject(path),
      throwsA(
        isA<FormatException>().having(
          (error) => error.message,
          'message',
          contains('expands beyond its declared bounded size'),
        ),
      ),
    );
  });

  test('rejects encrypted and non-regular ZIP entries', () async {
    final fixture = await _fixtureDirectory('goremod_zip_types_');
    final valid = _encodeZip([
      _stored('project.json', _projectJsonBytes(ModProject(name: 'Types'))),
    ]);

    final encrypted = Uint8List.fromList(valid);
    final encryptedLocal = _findSignature(encrypted, 0x04034b50);
    final encryptedCentral = _findSignature(encrypted, 0x02014b50);
    _writeUint16(
      encrypted,
      encryptedLocal + 6,
      _readUint16(encrypted, encryptedLocal + 6) | 1,
    );
    _writeUint16(
      encrypted,
      encryptedCentral + 8,
      _readUint16(encrypted, encryptedCentral + 8) | 1,
    );
    final encryptedPath = p.join(fixture.path, 'encrypted.goremod');
    await File(encryptedPath).writeAsBytes(encrypted, flush: true);
    await expectLater(
      loadProject(encryptedPath),
      throwsA(
        isA<FormatException>().having(
          (error) => error.message,
          'message',
          contains('unsupported flags'),
        ),
      ),
    );

    final symlink = Uint8List.fromList(valid);
    final symlinkCentral = _findSignature(symlink, 0x02014b50);
    _writeUint16(symlink, symlinkCentral + 4, (3 << 8) | 20);
    _writeUint32(symlink, symlinkCentral + 38, 0xa000 << 16);
    final symlinkPath = p.join(fixture.path, 'symlink.goremod');
    await File(symlinkPath).writeAsBytes(symlink, flush: true);
    await expectLater(
      loadProject(symlinkPath),
      throwsA(
        isA<FormatException>().having(
          (error) => error.message,
          'message',
          contains('not a regular file'),
        ),
      ),
    );
  });

  test('rejects unsafe and missing embedded asset references', () async {
    final fixture = await _fixtureDirectory('goremod_zip_asset_refs_');
    final cases = {
      'unsafe': 'assets/audio/../escape.wav',
      'missing': 'assets/audio/missing.wav',
    };

    for (final fixtureCase in cases.entries) {
      final project = ModProject(
        name: fixtureCase.key,
        audio: [
          AudioReplacement(
            bank: 'SFX.bank',
            sample: fixtureCase.key,
            wavPath: fixtureCase.value,
          ),
        ],
      );
      final path = p.join(fixture.path, '${fixtureCase.key}.goremod');
      await _writeZip(path, [
        _stored('project.json', _projectJsonBytes(project)),
      ]);
      await expectLater(
        loadProject(path),
        throwsA(isA<FormatException>()),
        reason: fixtureCase.key,
      );
    }
  });

  test('rejects repeated references to one embedded payload', () async {
    final fixture = await _fixtureDirectory('goremod_zip_repeated_ref_');
    const shared = 'assets/audio/shared.wav';
    final project = ModProject(
      name: 'RepeatedReference',
      audio: const [
        AudioReplacement(
          bank: 'SFX.bank',
          sample: 'SFX_FIRST',
          wavPath: shared,
        ),
        AudioReplacement(
          bank: 'SFX.bank',
          sample: 'SFX_SECOND',
          wavPath: shared,
        ),
      ],
    );
    final path = p.join(fixture.path, 'repeated.goremod');
    await _writeZip(path, [
      _stored('project.json', _projectJsonBytes(project)),
      _stored(shared, <int>[...ascii.encode('RIFF'), 1, 2, 3, 4]),
    ]);

    await expectLater(
      loadProject(path),
      throwsA(
        isA<FormatException>().having(
          (error) => error.message,
          'message',
          contains('duplicate embedded asset reference'),
        ),
      ),
    );
  });

  test('a failed second load preserves the first extraction root', () async {
    final fixture = await _fixtureDirectory('goremod_load_isolation_');
    final sourceBytes = <int>[
      ...ascii.encode('RIFF'),
      ...List<int>.filled(12, 9),
    ];
    final source = await _writeSource(fixture, 'first.wav', sourceBytes);
    final goodPath = p.join(fixture.path, 'first.goremod');
    await saveProject(
      ModProject(
        name: 'First',
        audio: [
          AudioReplacement(
            bank: 'SFX.bank',
            sample: 'SFX_FIRST',
            wavPath: source.path,
          ),
        ],
      ),
      goodPath,
    );

    final first = await loadProject(goodPath);
    _registerWorkspaceCleanup(first);
    final firstAsset = File(first.project.audio.single.wavPath);
    final firstRoot = _extractionRoot(firstAsset.path);
    expect(p.basename(firstRoot.path), startsWith('goremod_loaded_'));
    expect(await firstAsset.readAsBytes(), sourceBytes);

    final badPath = p.join(fixture.path, 'second.goremod');
    final bad = ModProject(
      name: 'Second',
      audio: const [
        AudioReplacement(
          bank: 'SFX.bank',
          sample: 'SFX_SECOND',
          wavPath: 'assets/audio/missing.wav',
        ),
      ],
    );
    await _writeZip(badPath, [_stored('project.json', _projectJsonBytes(bad))]);

    await expectLater(loadProject(badPath), throwsFormatException);
    expect(await firstRoot.exists(), isTrue);
    expect(await firstAsset.readAsBytes(), sourceBytes);
  });

  test('loaded workspaces are isolated and release is idempotent', () async {
    final fixture = await _fixtureDirectory('goremod_workspace_lease_');
    final sourceBytes = <int>[
      ...ascii.encode('RIFF'),
      ...List<int>.filled(12, 4),
    ];
    final source = await _writeSource(fixture, 'source.wav', sourceBytes);
    final projectPath = p.join(fixture.path, 'leased.goremod');
    await saveProject(
      ModProject(
        name: 'Leased',
        audio: [
          AudioReplacement(
            bank: 'SFX.bank',
            sample: 'SFX_LEASED',
            wavPath: source.path,
          ),
        ],
      ),
      projectPath,
    );

    final first = await loadProject(projectPath);
    final second = await loadProject(projectPath);
    _registerWorkspaceCleanup(first);
    _registerWorkspaceCleanup(second);
    final firstRoot = Directory(first.workspace!.path);
    final secondRoot = Directory(second.workspace!.path);
    expect(firstRoot.path, isNot(secondRoot.path));
    expect(await firstRoot.exists(), isTrue);
    expect(await secondRoot.exists(), isTrue);

    await first.workspace!.release();
    await first.workspace!.release();
    expect(await firstRoot.exists(), isFalse);
    expect(await secondRoot.exists(), isTrue);
    expect(
      await File(second.project.audio.single.wavPath).readAsBytes(),
      sourceBytes,
    );

    await second.workspace!.release();
    expect(await secondRoot.exists(), isFalse);
  });

  test('successful overwrite leaves no swap or fixed temp residue', () async {
    final fixture = await _fixtureDirectory('goremod_swap_cleanup_');
    final output = p.join(fixture.path, 'project.goremod');

    await saveProject(ModProject(name: 'First'), output);
    await saveProject(ModProject(name: 'Second'), output);

    final loaded = await loadProject(output);
    _registerWorkspaceCleanup(loaded);
    expect(loaded.project.name, 'Second');
    final residue = <String>[];
    await for (final entity in fixture.list(followLinks: false)) {
      final name = p.basename(entity.path);
      if (name.endsWith('.tmp') ||
          name.endsWith('.bak') ||
          name.contains('.gore-swap')) {
        residue.add(name);
      }
    }
    expect(residue, isEmpty);
    expect(await File('$output.tmp').exists(), isFalse);
    expect(await File('$output.bak').exists(), isFalse);
  });

  test('equivalent saves produce byte-identical portable archives', () async {
    final fixture = await _fixtureDirectory('goremod_deterministic_');
    final source = await _writeSource(
      fixture,
      'voice with a noncanonical source name.ogg',
      <int>[...ascii.encode('OggS'), ...List<int>.filled(32, 4)],
    );
    final project = ModProject(
      name: 'Deterministic',
      voice: [
        VoiceArchiveEdit(
          locId: 'INFO_VIPER_FIXTURE',
          locale: 'de',
          archive: 'german_new.zip',
          operation: VoicePatchOperation.replace,
          archivePath: 'NPC/Viper/info_viper_fixture.ogg',
          oggPath: source.path,
          observation: _voiceObservation,
        ),
      ],
    );
    final first = p.join(fixture.path, 'first.goremod');
    final second = p.join(fixture.path, 'second.goremod');

    await saveProject(project, first);
    await Future<void>.delayed(const Duration(seconds: 2));
    await saveProject(project, second);

    expect(await File(second).readAsBytes(), await File(first).readAsBytes());
  });
}

Future<Directory> _fixtureDirectory(String prefix) async {
  final directory = await Directory.systemTemp.createTemp(prefix);
  addTearDown(() async {
    if (await directory.exists()) await directory.delete(recursive: true);
  });
  return directory;
}

Future<File> _writeSource(
  Directory directory,
  String name,
  List<int> bytes,
) async {
  final file = File(p.join(directory.path, name));
  await file.writeAsBytes(bytes, flush: true);
  return file;
}

Uint8List _projectJsonBytes(ModProject project) =>
    Uint8List.fromList(utf8.encode(jsonEncode(project.toJson())));

ArchiveFile _stored(String name, List<int> bytes) =>
    ArchiveFile.noCompress(name, bytes.length, Uint8List.fromList(bytes));

List<int> _encodeZip(List<ArchiveFile> files) {
  final archive = Archive();
  for (final file in files) {
    archive.addFile(file);
  }
  return ZipEncoder().encode(archive)!;
}

Future<void> _writeZip(String path, List<ArchiveFile> files) =>
    File(path).writeAsBytes(_encodeZip(files), flush: true);

int _indexOf(List<int> haystack, List<int> needle) {
  if (needle.isEmpty) return 0;
  for (var start = 0; start <= haystack.length - needle.length; start++) {
    var matches = true;
    for (var offset = 0; offset < needle.length; offset++) {
      if (haystack[start + offset] != needle[offset]) {
        matches = false;
        break;
      }
    }
    if (matches) return start;
  }
  return -1;
}

int _findSignature(List<int> bytes, int signature) {
  final needle = <int>[
    signature & 0xff,
    (signature >> 8) & 0xff,
    (signature >> 16) & 0xff,
    (signature >> 24) & 0xff,
  ];
  return _indexOf(bytes, needle);
}

int _readUint16(List<int> bytes, int offset) =>
    bytes[offset] | (bytes[offset + 1] << 8);

void _writeUint16(List<int> bytes, int offset, int value) {
  bytes[offset] = value & 0xff;
  bytes[offset + 1] = (value >> 8) & 0xff;
}

void _writeUint32(List<int> bytes, int offset, int value) {
  bytes[offset] = value & 0xff;
  bytes[offset + 1] = (value >> 8) & 0xff;
  bytes[offset + 2] = (value >> 16) & 0xff;
  bytes[offset + 3] = (value >> 24) & 0xff;
}

Directory _extractionRoot(String assetPath) =>
    Directory(p.dirname(p.dirname(p.dirname(assetPath))));

void _registerWorkspaceCleanup(LoadedProject loaded) {
  addTearDown(() async {
    await loaded.workspace?.release();
  });
}
