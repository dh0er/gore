import 'dart:convert';
import 'dart:io';

import 'package:archive/archive_io.dart';
import 'package:path/path.dart' as p;

import '../audio/domain/audio_replacements_notifier.dart';
import 'project_model.dart';

const String kProjectExtension = '.goremod';

/// Save [project] to a `.goremod` zip at [path], embedding each replacement WAV under
/// `assets/audio/` so the project is self-contained and portable.
Future<void> saveProject(ModProject project, String path) async {
  final archive = Archive();

  final embeddedAudio = <AudioReplacement>[];
  var idx = 0;
  for (final a in project.audio) {
    final bytes = await File(a.wavPath).readAsBytes();
    final rel = 'assets/audio/${idx}_${p.basename(a.wavPath)}';
    idx++;
    archive.addFile(ArchiveFile(rel, bytes.length, bytes));
    embeddedAudio.add(a.withWavPath(rel));
  }

  final embedded = project.copyWith(audio: embeddedAudio);
  final json = utf8.encode(const JsonEncoder.withIndent('  ').convert(embedded.toJson()));
  archive.addFile(ArchiveFile('project.json', json.length, json));

  final zip = ZipEncoder().encode(archive) ?? <int>[];
  await File(path).writeAsBytes(zip);
}

/// Load a `.goremod` project from [path]. Embedded WAVs are extracted to a temp dir and the
/// audio replacements' paths rewritten to point at them.
Future<ModProject> loadProject(String path) async {
  final bytes = await File(path).readAsBytes();
  final archive = ZipDecoder().decodeBytes(bytes);

  final pj = archive.findFile('project.json');
  if (pj == null) {
    throw const FormatException('not a gore-mod project (no project.json)');
  }
  final project = ModProject.fromJson(
      jsonDecode(utf8.decode(pj.content as List<int>)) as Map<String, Object?>);

  final tmp = await Directory.systemTemp.createTemp('goremod_');
  final extractedAudio = <AudioReplacement>[];
  for (final a in project.audio) {
    if (a.wavPath.startsWith('assets/')) {
      final f = archive.findFile(a.wavPath);
      if (f != null) {
        final out = p.join(tmp.path, p.basename(a.wavPath));
        await File(out).writeAsBytes(f.content as List<int>);
        extractedAudio.add(a.withWavPath(out));
        continue;
      }
    }
    extractedAudio.add(a); // external path (shouldn't happen for saved projects)
  }

  return project.copyWith(audio: extractedAudio);
}
