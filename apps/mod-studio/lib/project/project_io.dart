import 'dart:convert';
import 'dart:io';

import 'package:archive/archive_io.dart';
import 'package:path/path.dart' as p;

import '../audio/domain/audio_replacements_notifier.dart';
import '../scripts/domain/script_mods_notifier.dart';
import '../textures/domain/texture_replacements_notifier.dart';
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

  final embeddedTextures = <TextureReplacement>[];
  var tidx = 0;
  for (final t in project.textures) {
    final bytes = await File(t.imagePath).readAsBytes();
    final rel = 'assets/textures/${tidx}_${p.basename(t.imagePath)}';
    tidx++;
    archive.addFile(ArchiveFile(rel, bytes.length, bytes));
    embeddedTextures.add(t.withImagePath(rel));
  }

  final embeddedScripts = <ScriptMod>[];
  var sidx = 0;
  for (final s in project.scripts) {
    final asBytes = await File(s.asPath).readAsBytes();
    final asRel = 'assets/scripts/${sidx}_${p.basename(s.asPath)}';
    archive.addFile(ArchiveFile(asRel, asBytes.length, asBytes));
    var rebuilt = s.withAsPath(asRel);
    // Embed the compiled mini-cache too, if this mod has been compiled.
    if (s.miniPath.isNotEmpty) {
      final miniBytes = await File(s.miniPath).readAsBytes();
      final miniRel = 'assets/scripts_cache/${sidx}_${p.basename(s.miniPath)}';
      archive.addFile(ArchiveFile(miniRel, miniBytes.length, miniBytes));
      rebuilt = rebuilt.withMiniPath(miniRel);
    }
    sidx++;
    embeddedScripts.add(rebuilt);
  }

  final embedded =
      project.copyWith(audio: embeddedAudio, textures: embeddedTextures, scripts: embeddedScripts);
  final json = utf8.encode(const JsonEncoder.withIndent('  ').convert(embedded.toJson()));
  archive.addFile(ArchiveFile('project.json', json.length, json));

  final zip = ZipEncoder().encode(archive);
  if (zip == null) {
    throw const FormatException('failed to encode the project archive');
  }
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

  // Reuse one fixed temp dir and clear it each load, so successive opens don't accumulate
  // extracted-WAV directories under the system temp folder.
  final tmp = Directory(p.join(Directory.systemTemp.path, 'goremod_loaded'));
  if (tmp.existsSync()) {
    try {
      await tmp.delete(recursive: true);
    } catch (_) {}
  }
  await tmp.create(recursive: true);
  final extractedAudio = <AudioReplacement>[];
  for (final a in project.audio) {
    final segs = a.wavPath.split('/');
    // A .goremod is portable/shareable, so treat embedded paths as untrusted: only extract
    // entries under assets/ with no '..' traversal or absolute path, and that resolve to a
    // location strictly inside the temp dir — otherwise opening a malicious project could
    // write arbitrary files on disk.
    final safe = a.wavPath.startsWith('assets/') &&
        !p.isAbsolute(a.wavPath) &&
        !segs.contains('..');
    if (safe) {
      final f = archive.findFile(a.wavPath);
      final out = p.joinAll([tmp.path, ...segs]);
      if (f != null && p.isWithin(tmp.path, out)) {
        await Directory(p.dirname(out)).create(recursive: true);
        await File(out).writeAsBytes(f.content as List<int>);
        extractedAudio.add(a.withWavPath(out));
        continue;
      }
    }
    // Unsafe (absolute/'..'/outside assets) or missing archive entry: DROP it. A saved project is
    // self-contained with audio embedded under assets/audio/, so keeping an external/absolute path
    // would let a crafted .goremod pull arbitrary local files into a later save/bundle, or break
    // the build on a missing file. Silently skipping is safer than preserving it.
  }

  final extractedTextures = <TextureReplacement>[];
  for (final t in project.textures) {
    final segs = t.imagePath.split('/');
    // Same untrusted-path guard as audio: only extract entries under assets/ with no '..'
    // traversal or absolute path, and that resolve strictly inside the temp dir.
    final safe = t.imagePath.startsWith('assets/') &&
        !p.isAbsolute(t.imagePath) &&
        !segs.contains('..');
    if (safe) {
      final f = archive.findFile(t.imagePath);
      final out = p.joinAll([tmp.path, ...segs]);
      if (f != null && p.isWithin(tmp.path, out)) {
        await Directory(p.dirname(out)).create(recursive: true);
        await File(out).writeAsBytes(f.content as List<int>);
        extractedTextures.add(t.withImagePath(out));
        continue;
      }
    }
    // Unsafe or missing archive entry: DROP it (same rationale as audio above).
  }

  final extractedScripts = <ScriptMod>[];
  for (final s in project.scripts) {
    // Untrusted embedded paths: only extract entries under assets/ with no '..' / absolute path,
    // resolving strictly inside the temp dir. Same guard as audio/textures.
    String? extract(String rel) {
      final segs = rel.split('/');
      final safe = rel.startsWith('assets/') && !p.isAbsolute(rel) && !segs.contains('..');
      if (!safe) return null;
      final f = archive.findFile(rel);
      final out = p.joinAll([tmp.path, ...segs]);
      if (f == null || !p.isWithin(tmp.path, out)) return null;
      Directory(p.dirname(out)).createSync(recursive: true);
      File(out).writeAsBytesSync(f.content as List<int>);
      return out;
    }

    // The relPath becomes the in-game ScriptRelativeFilename at deploy, so treat it as untrusted
    // too (defense-in-depth, matching the asPath stance and the gore-as compile-side guard): drop
    // the mod if it's empty, absolute, or escapes via a '..' segment.
    final relSegs = s.relPath.split(RegExp(r'[\\/]'));
    if (s.relPath.isEmpty || p.isAbsolute(s.relPath) || relSegs.contains('..')) {
      continue; // unsafe relPath: drop the mod
    }

    final asOut = extract(s.asPath);
    if (asOut == null) continue; // unsafe/missing source: drop the mod
    var rebuilt = s.withAsPath(asOut);
    if (s.miniPath.isNotEmpty) {
      final miniOut = extract(s.miniPath);
      // A missing/unsafe mini just means "not compiled" — keep the source, clear the mini.
      rebuilt = rebuilt.withMiniPath(miniOut ?? '');
    }
    extractedScripts.add(rebuilt);
  }

  return project.copyWith(
      audio: extractedAudio, textures: extractedTextures, scripts: extractedScripts);
}
