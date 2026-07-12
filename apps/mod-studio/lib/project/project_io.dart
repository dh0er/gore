import 'dart:convert';
import 'dart:io';
import 'dart:typed_data';

import 'package:archive/archive_io.dart' hide ZLibDecoder;
import 'package:path/path.dart' as p;

import '../audio/domain/audio_replacements_notifier.dart';
import '../scripts/domain/script_mods_notifier.dart';
import '../textures/domain/texture_replacements_notifier.dart';
import '../voice/domain/voice_edits_notifier.dart';
import 'project_atomic_io.dart';
import 'project_model.dart';

const String kProjectExtension = '.goremod';

// Format 1 is a portable interchange archive, not the eventual large-project
// working store. These finite limits keep an untrusted project from turning an
// Open action into unbounded allocation or filesystem work. Large projects
// will use the sharded V2 working format and export a bounded interchange file.
const int _maxProjectArchiveBytes = 256 * 1024 * 1024;
const int _maxProjectEntries = 16384;
const int _maxCentralDirectoryBytes = 64 * 1024 * 1024;
const int _maxEntryBytes = 256 * 1024 * 1024;
const int _maxTotalUncompressedBytes = 512 * 1024 * 1024;
const int _maxProjectJsonBytes = 16 * 1024 * 1024;
const int _maxVoiceOggBytes = 64 * 1024 * 1024;
const int _maxTotalVoiceBytes = 256 * 1024 * 1024;
const int _maxCompressionRatio = 1000;
const int _maxArchivePathBytes = 1024;

final AtomicByteReplacement _projectReplacement = AtomicByteReplacement();

/// One successfully decoded project plus ownership of any extracted sources.
class LoadedProject {
  const LoadedProject({required this.project, required this.workspace});

  final ModProject project;
  final ProjectWorkspaceLease? workspace;
}

/// Owns the unique extraction root backing one loaded format-1 project.
///
/// The session swaps leases only after a candidate project has been fully
/// applied. [release] is idempotent and never follows a replaced root link.
class ProjectWorkspaceLease {
  ProjectWorkspaceLease._(this._root);

  final Directory _root;
  Future<void>? _releaseFuture;

  String get path => _root.path;

  Future<void> release() => _releaseFuture ??= _releaseOnce();

  Future<void> _releaseOnce() async {
    final systemTemp = p.normalize(p.absolute(Directory.systemTemp.path));
    final root = p.normalize(p.absolute(_root.path));
    if (!_samePath(p.dirname(root), systemTemp) ||
        !p.basename(root).startsWith('goremod_loaded_')) {
      throw StateError(
        'refusing to release an unowned project workspace: $root',
      );
    }
    final type = await FileSystemEntity.type(root, followLinks: false);
    if (type == FileSystemEntityType.notFound) return;
    if (type != FileSystemEntityType.directory) {
      throw StateError(
        'project workspace root is no longer a directory; preserving it: $root',
      );
    }
    await Directory(root).delete(recursive: true);
  }
}

/// Save [project] to a self-contained `.goremod` archive.
///
/// The complete archive is encoded and reopened through the same strict reader
/// used by Open before a crash-recoverable sibling swap publishes it. A failed
/// source read, validation, or swap therefore cannot truncate the last good
/// project generation.
Future<void> saveProject(ModProject project, String path) async {
  final snapshot = _snapshotProject(project);
  snapshot.validateUniqueTargets();
  final archive = Archive();
  final budget = _SaveAssetBudget();

  final embeddedAudio = <AudioReplacement>[];
  for (var index = 0; index < snapshot.audio.length; index++) {
    final replacement = snapshot.audio[index];
    final bytes = await _readSourceAsset(
      replacement.wavPath,
      label: 'audio replacement WAV',
      maxBytes: _maxEntryBytes,
      budget: budget,
    );
    final relative = 'assets/audio/${_assetIndex(index)}.wav';
    archive.addFile(ArchiveFile(relative, bytes.length, bytes));
    embeddedAudio.add(replacement.withWavPath(relative));
  }

  final embeddedTextures = <TextureReplacement>[];
  for (var index = 0; index < snapshot.textures.length; index++) {
    final replacement = snapshot.textures[index];
    final bytes = await _readSourceAsset(
      replacement.imagePath,
      label: 'texture replacement image',
      maxBytes: _maxEntryBytes,
      budget: budget,
    );
    final relative = 'assets/textures/${_assetIndex(index)}.png';
    archive.addFile(ArchiveFile(relative, bytes.length, bytes));
    embeddedTextures.add(replacement.withImagePath(relative));
  }

  final embeddedScripts = <ScriptMod>[];
  for (var index = 0; index < snapshot.scripts.length; index++) {
    final mod = snapshot.scripts[index];
    _validateScriptRelativePath(mod.relPath);
    final sourceBytes = await _readSourceAsset(
      mod.asPath,
      label: 'AngelScript source',
      maxBytes: _maxEntryBytes,
      budget: budget,
    );
    final sourceRelative = 'assets/scripts/${_assetIndex(index)}.as';
    archive.addFile(
      ArchiveFile(sourceRelative, sourceBytes.length, sourceBytes),
    );
    var embedded = mod.withAsPath(sourceRelative);
    if (mod.miniPath.isNotEmpty) {
      final miniBytes = await _readSourceAsset(
        mod.miniPath,
        label: 'compiled AngelScript mini-cache',
        maxBytes: _maxEntryBytes,
        budget: budget,
      );
      final miniRelative = 'assets/scripts_cache/${_assetIndex(index)}.mini';
      archive.addFile(ArchiveFile(miniRelative, miniBytes.length, miniBytes));
      embedded = embedded.withMiniPath(miniRelative);
    }
    embeddedScripts.add(embedded);
  }

  final embeddedVoice = <VoiceArchiveEdit>[];
  for (var index = 0; index < snapshot.voice.length; index++) {
    final edit = snapshot.voice[index];
    validateVoiceArchiveEdit(edit);
    final bytes = await _readSourceAsset(
      edit.oggPath,
      label: 'dialog voice Ogg',
      maxBytes: _maxVoiceOggBytes,
      budget: budget,
      voice: true,
    );
    if (bytes.isEmpty) {
      throw const FormatException('dialog voice Ogg must not be empty');
    }
    final relative = 'assets/voice/${_assetIndex(index)}.ogg';
    archive.addFile(ArchiveFile(relative, bytes.length, bytes));
    embeddedVoice.add(edit.withOggPath(relative));
  }

  final embedded = snapshot.copyWith(
    audio: embeddedAudio,
    textures: embeddedTextures,
    scripts: embeddedScripts,
    voice: embeddedVoice,
  );
  final projectJson = utf8.encode(
    const JsonEncoder.withIndent('  ').convert(embedded.toJson()),
  );
  if (projectJson.isEmpty || projectJson.length > _maxProjectJsonBytes) {
    throw const FormatException(
      'project.json size is outside the supported format-1 limit',
    );
  }
  archive.addFile(ArchiveFile('project.json', projectJson.length, projectJson));

  final encoded = ZipEncoder().encode(
    archive,
    modified: DateTime.utc(1980, 1, 1),
  );
  if (encoded == null) {
    throw const FormatException('failed to encode the project archive');
  }

  // Validate before starting a swap so a bug in our own encoder leaves no
  // recovery journal. The atomic helper validates the flushed temp and promoted
  // target again to detect disk corruption or a partial write.
  final prepared = _prepareProjectArchive(encoded);
  _verifyPreparedAssets(prepared);
  await _projectReplacement.replace(
    target: File(path),
    bytes: encoded,
    validate: _validateProjectArchiveFile,
  );
}

ModProject _snapshotProject(ModProject project) => ModProject(
  name: project.name,
  version: project.version,
  author: project.author,
  delayMs: project.delayMs,
  overrides: List.unmodifiable(project.overrides),
  locEdits: Map<String, Map<String, String>>.unmodifiable({
    for (final entry in project.locEdits.entries)
      entry.key: Map<String, String>.unmodifiable(entry.value),
  }),
  audio: List.unmodifiable(project.audio),
  textures: List.unmodifiable(project.textures),
  scripts: List.unmodifiable(project.scripts),
  dialogTopics: List.unmodifiable(project.dialogTopics),
  voice: List.unmodifiable(project.voice),
);

/// Load a `.goremod` project from [path].
///
/// Every referenced asset is preflighted and decoded before a unique candidate
/// workspace is created. Extraction is all-or-nothing: malformed audio,
/// textures, scripts, or voice data fail the Open operation instead of being
/// silently omitted. A previously opened project's workspace is never deleted
/// by a later load attempt.
Future<LoadedProject> loadProject(String path) async {
  final source = File(path);
  await _projectReplacement.repair(
    target: source,
    validate: _validateProjectArchiveFile,
  );
  final length = await source.length();
  if (length <= 0 || length > _maxProjectArchiveBytes) {
    throw FormatException(
      'project archive size is outside 1..$_maxProjectArchiveBytes bytes',
    );
  }
  final parsed = _prepareProjectArchive(await source.readAsBytes());
  final project = parsed.project;
  if (!_hasEmbeddedAssets(project)) {
    return LoadedProject(project: project, workspace: null);
  }

  final workspace = await Directory.systemTemp.createTemp('goremod_loaded_');
  try {
    final audio = <AudioReplacement>[];
    for (var index = 0; index < project.audio.length; index++) {
      final replacement = project.audio[index];
      final out = await _extractPreparedAsset(
        workspace,
        'assets/audio/${_assetIndex(index)}.wav',
        parsed,
        replacement.wavPath,
      );
      audio.add(replacement.withWavPath(out));
    }

    final textures = <TextureReplacement>[];
    for (var index = 0; index < project.textures.length; index++) {
      final replacement = project.textures[index];
      final out = await _extractPreparedAsset(
        workspace,
        'assets/textures/${_assetIndex(index)}.png',
        parsed,
        replacement.imagePath,
      );
      textures.add(replacement.withImagePath(out));
    }

    final scripts = <ScriptMod>[];
    for (var index = 0; index < project.scripts.length; index++) {
      final mod = project.scripts[index];
      final sourceOut = await _extractPreparedAsset(
        workspace,
        'assets/scripts/${_assetIndex(index)}.as',
        parsed,
        mod.asPath,
      );
      var extracted = mod.withAsPath(sourceOut);
      if (mod.miniPath.isNotEmpty) {
        final miniOut = await _extractPreparedAsset(
          workspace,
          'assets/scripts_cache/${_assetIndex(index)}.mini',
          parsed,
          mod.miniPath,
        );
        extracted = extracted.withMiniPath(miniOut);
      }
      scripts.add(extracted);
    }

    final voice = <VoiceArchiveEdit>[];
    for (var index = 0; index < project.voice.length; index++) {
      final edit = project.voice[index];
      final out = await _extractPreparedAsset(
        workspace,
        'assets/voice/${_assetIndex(index)}.ogg',
        parsed,
        edit.oggPath,
      );
      voice.add(edit.withOggPath(out));
    }

    return LoadedProject(
      project: project.copyWith(
        audio: audio,
        textures: textures,
        scripts: scripts,
        voice: voice,
      ),
      workspace: ProjectWorkspaceLease._(workspace),
    );
  } catch (_) {
    await ProjectWorkspaceLease._(workspace).release();
    rethrow;
  }
}

Future<bool> _validateProjectArchiveFile(File candidate) async {
  try {
    final length = await candidate.length();
    if (length <= 0 || length > _maxProjectArchiveBytes) return false;
    final prepared = _prepareProjectArchive(await candidate.readAsBytes());
    _verifyPreparedAssets(prepared);
    return true;
  } catch (_) {
    return false;
  }
}

_PreparedProjectArchive _prepareProjectArchive(List<int> input) {
  if (input.isEmpty || input.length > _maxProjectArchiveBytes) {
    throw FormatException(
      'project archive size is outside 1..$_maxProjectArchiveBytes bytes',
    );
  }
  final bytes = input is Uint8List ? input : Uint8List.fromList(input);
  final inspected = _inspectProjectZip(bytes);
  final projectEntry = inspected.entries['project.json'];
  if (projectEntry == null) {
    throw const FormatException('not a gore-mod project (no project.json)');
  }
  if (projectEntry.uncompressedSize <= 0 ||
      projectEntry.uncompressedSize > _maxProjectJsonBytes) {
    throw const FormatException(
      'project.json size is outside the supported format-1 limit',
    );
  }

  final projectJsonBytes = _decodeZipEntry(
    bytes,
    projectEntry,
    maxBytes: _maxProjectJsonBytes,
  )!;
  final ModProject project;
  try {
    final decoded = jsonDecode(utf8.decode(projectJsonBytes));
    if (decoded is! Map) {
      throw const FormatException('project.json root must be an object');
    }
    project = ModProject.fromJson(decoded.cast<String, Object?>());
  } on FormatException {
    rethrow;
  } catch (error) {
    throw FormatException('invalid project.json: $error');
  }

  final expected = _expectedAssetLimits(project, inspected.entries);
  final expectedNames = <String>{'project.json', ...expected.keys};
  final extraNames = inspected.entries.keys
      .where((name) => !expectedNames.contains(name))
      .toList(growable: false);
  if (extraNames.isNotEmpty) {
    throw FormatException(
      'project archive contains unreferenced entries: ${extraNames.join(', ')}',
    );
  }

  return _PreparedProjectArchive(
    project: project,
    archive: bytes,
    entries: inspected.entries,
    assetLimits: Map.unmodifiable(expected),
  );
}

void _verifyPreparedAssets(_PreparedProjectArchive prepared) {
  for (final expected in prepared.assetLimits.entries) {
    _decodeZipEntry(
      prepared.archive,
      prepared.entries[expected.key]!,
      maxBytes: expected.value,
      collect: false,
    );
  }
}

_InspectedZip _inspectProjectZip(Uint8List bytes) {
  // Format 1 intentionally rejects ZIP comments and ZIP64. This gives the
  // end record one exact position and prevents archive 3.6.1's permissive EOCD
  // scan from accepting a signature embedded in trailing data or a comment.
  if (bytes.length < 22) {
    throw const FormatException('project archive is shorter than a ZIP EOCD');
  }
  final eocd = bytes.length - 22;
  if (_uint32(bytes, eocd) != ZipDirectory.eocdLocatorSignature) {
    throw const FormatException('project archive has no exact terminal EOCD');
  }
  final disk = _uint16(bytes, eocd + 4);
  final centralDisk = _uint16(bytes, eocd + 6);
  final entriesOnDisk = _uint16(bytes, eocd + 8);
  final entryCount = _uint16(bytes, eocd + 10);
  final centralSize = _uint32(bytes, eocd + 12);
  final centralOffset = _uint32(bytes, eocd + 16);
  final commentLength = _uint16(bytes, eocd + 20);
  if (commentLength != 0 ||
      disk != 0 ||
      centralDisk != 0 ||
      entriesOnDisk != entryCount) {
    throw const FormatException(
      'project archive must be a single-disk ZIP without a comment',
    );
  }
  if (entryCount <= 0 || entryCount > _maxProjectEntries) {
    throw FormatException(
      'project archive entry count is outside 1..$_maxProjectEntries',
    );
  }
  if (centralSize <= 0 || centralSize > _maxCentralDirectoryBytes) {
    throw const FormatException(
      'project archive central directory exceeds the supported limit',
    );
  }
  if (centralOffset < 0 ||
      centralOffset > eocd ||
      centralSize > eocd - centralOffset ||
      centralOffset + centralSize != eocd) {
    throw const FormatException(
      'project archive central directory has invalid bounds',
    );
  }

  final ZipDirectory directory;
  try {
    directory = ZipDirectory.read(InputStream(bytes));
  } catch (error) {
    throw FormatException('invalid ZIP directory: $error');
  }
  if (directory.filePosition != eocd ||
      directory.numberOfThisDisk != 0 ||
      directory.diskWithTheStartOfTheCentralDirectory != 0 ||
      directory.totalCentralDirectoryEntriesOnThisDisk != entryCount ||
      directory.totalCentralDirectoryEntries != entryCount ||
      directory.centralDirectorySize != centralSize ||
      directory.centralDirectoryOffset != centralOffset ||
      directory.fileHeaders.length != entryCount) {
    throw const FormatException(
      'project archive directory disagrees with its terminal EOCD',
    );
  }

  final entries = <String, _ZipEntry>{};
  final caseFoldedNames = <String>{};
  final intervals = <({int start, int end, String name})>[];
  var totalUncompressed = 0;
  for (final header in directory.fileHeaders) {
    final name = header.filename;
    _validateArchivePath(name, label: 'ZIP entry');
    if (!caseFoldedNames.add(name.toLowerCase())) {
      throw FormatException('duplicate case-insensitive ZIP entry: $name');
    }
    if (entries.containsKey(name)) {
      throw FormatException('duplicate ZIP entry: $name');
    }

    final compressedSize = header.compressedSize;
    final uncompressedSize = header.uncompressedSize;
    final localOffset = header.localHeaderOffset;
    final crc32 = header.crc32;
    if (compressedSize == null ||
        uncompressedSize == null ||
        localOffset == null ||
        crc32 == null ||
        compressedSize < 0 ||
        uncompressedSize < 0 ||
        uncompressedSize > _maxEntryBytes) {
      throw FormatException('ZIP entry has invalid sizes or offset: $name');
    }
    totalUncompressed += uncompressedSize;
    if (totalUncompressed > _maxTotalUncompressedBytes) {
      throw const FormatException(
        'project archive exceeds the total uncompressed-size limit',
      );
    }
    if (uncompressedSize > 0 &&
        (compressedSize == 0 ||
            uncompressedSize > compressedSize * _maxCompressionRatio)) {
      throw FormatException('ZIP entry compression ratio is too high: $name');
    }
    if (header.diskNumberStart != 0) {
      throw FormatException('multi-disk ZIP entry is unsupported: $name');
    }
    final flags = header.generalPurposeBitFlag;
    if ((flags & 0x1) != 0 || (flags & ~0x080e) != 0) {
      throw FormatException('ZIP entry uses unsupported flags: $name');
    }
    final method = header.compressionMethod;
    if (method != ZipFile.zipCompressionStore &&
        method != ZipFile.zipCompressionDeflate) {
      throw FormatException('ZIP entry uses unsupported compression: $name');
    }
    if (method == ZipFile.zipCompressionStore &&
        compressedSize != uncompressedSize) {
      throw FormatException('stored ZIP entry size mismatch: $name');
    }

    final attributes = header.externalFileAttributes ?? 0;
    if (header.versionMadeBy >> 8 == 3) {
      final fileType = (attributes >> 16) & 0xf000;
      if (fileType != 0 && fileType != 0x8000) {
        throw FormatException('ZIP entry is not a regular file: $name');
      }
    }

    if (localOffset < 0 ||
        localOffset > centralOffset - 30 ||
        _uint32(bytes, localOffset) != ZipFile.zipFileSignature) {
      throw FormatException('ZIP entry has an invalid local header: $name');
    }
    final localFlags = _uint16(bytes, localOffset + 6);
    final localMethod = _uint16(bytes, localOffset + 8);
    final localCrc = _uint32(bytes, localOffset + 14);
    final localCompressed = _uint32(bytes, localOffset + 18);
    final localUncompressed = _uint32(bytes, localOffset + 22);
    final localNameLength = _uint16(bytes, localOffset + 26);
    final localExtraLength = _uint16(bytes, localOffset + 28);
    final nameStart = localOffset + 30;
    final nameEnd = nameStart + localNameLength;
    final dataOffset = nameEnd + localExtraLength;
    final dataEnd = dataOffset + compressedSize;
    if (nameEnd > centralOffset ||
        dataOffset > centralOffset ||
        dataEnd > centralOffset) {
      throw FormatException('ZIP entry data escapes local area: $name');
    }
    final String localName;
    try {
      localName = utf8.decode(bytes.sublist(nameStart, nameEnd));
    } catch (error) {
      throw FormatException('ZIP entry has an invalid local name: $error');
    }
    if (localName != name || localFlags != flags || localMethod != method) {
      throw FormatException(
        'ZIP local and central headers disagree for entry: $name',
      );
    }
    if ((flags & 0x08) == 0 &&
        (localCrc != crc32 ||
            localCompressed != compressedSize ||
            localUncompressed != uncompressedSize)) {
      throw FormatException('ZIP local sizes disagree for entry: $name');
    }

    var localEnd = dataEnd;
    if ((flags & 0x08) != 0) {
      var descriptor = dataEnd;
      if (_uint32(bytes, descriptor) == 0x08074b50) {
        descriptor += 4;
      }
      if (descriptor > centralOffset - 12 ||
          _uint32(bytes, descriptor) != crc32 ||
          _uint32(bytes, descriptor + 4) != compressedSize ||
          _uint32(bytes, descriptor + 8) != uncompressedSize) {
        throw FormatException('ZIP data descriptor mismatch: $name');
      }
      localEnd = descriptor + 12;
    }

    intervals.add((start: localOffset, end: localEnd, name: name));
    entries[name] = _ZipEntry(
      name: name,
      dataOffset: dataOffset,
      compressedSize: compressedSize,
      uncompressedSize: uncompressedSize,
      compressionMethod: method,
      crc32: crc32,
    );
  }

  intervals.sort((first, second) => first.start.compareTo(second.start));
  for (var index = 1; index < intervals.length; index++) {
    if (intervals[index].start < intervals[index - 1].end) {
      throw FormatException(
        'overlapping ZIP local entries: '
        '${intervals[index - 1].name}, ${intervals[index].name}',
      );
    }
  }
  return _InspectedZip(entries: Map.unmodifiable(entries));
}

Map<String, int> _expectedAssetLimits(
  ModProject project,
  Map<String, _ZipEntry> entries,
) {
  VoiceEditsNotifier.validateAll(project.voice);
  final expected = <String, int>{};

  void require(
    String relative,
    String prefix,
    String label, {
    int maxBytes = _maxEntryBytes,
    String? extension,
    bool nonEmpty = false,
  }) {
    _validateArchivePath(relative, label: label);
    if (!relative.startsWith('$prefix/')) {
      throw FormatException(
        '$label must be embedded under $prefix/: $relative',
      );
    }
    if (extension != null && !relative.toLowerCase().endsWith(extension)) {
      throw FormatException(
        '$label must use the $extension extension: $relative',
      );
    }
    final entry = entries[relative];
    if (entry == null) {
      throw FormatException('missing embedded $label: $relative');
    }
    if (entry.uncompressedSize > maxBytes) {
      throw FormatException(
        'embedded $label exceeds its size limit: $relative',
      );
    }
    if (nonEmpty && entry.uncompressedSize == 0) {
      throw FormatException('embedded $label must not be empty: $relative');
    }
    // Format-1 writers have always emitted one indexed member per authored
    // record. Reject aliases instead of inflating/copying one large member an
    // attacker-controlled number of times during extraction.
    if (expected.containsKey(relative)) {
      throw FormatException('duplicate embedded asset reference: $relative');
    }
    expected[relative] = maxBytes;
  }

  for (final replacement in project.audio) {
    require(
      replacement.wavPath,
      'assets/audio',
      'audio replacement WAV',
      extension: '.wav',
      nonEmpty: true,
    );
  }
  for (final replacement in project.textures) {
    require(
      replacement.imagePath,
      'assets/textures',
      'texture replacement image',
      extension: '.png',
      nonEmpty: true,
    );
  }
  for (final mod in project.scripts) {
    _validateScriptRelativePath(mod.relPath);
    require(
      mod.asPath,
      'assets/scripts',
      'AngelScript source',
      extension: '.as',
    );
    if (mod.miniPath.isNotEmpty) {
      require(
        mod.miniPath,
        'assets/scripts_cache',
        'compiled AngelScript mini-cache',
      );
    }
  }

  var totalVoiceBytes = 0;
  for (final edit in project.voice) {
    validateVoiceArchiveEdit(edit);
    require(
      edit.oggPath,
      'assets/voice',
      'dialog voice Ogg',
      maxBytes: _maxVoiceOggBytes,
      extension: '.ogg',
      nonEmpty: true,
    );
    totalVoiceBytes += entries[edit.oggPath]!.uncompressedSize;
    if (totalVoiceBytes > _maxTotalVoiceBytes) {
      throw const FormatException(
        'embedded dialog voice exceeds the total format-1 size limit',
      );
    }
  }
  return expected;
}

Uint8List? _decodeZipEntry(
  Uint8List archive,
  _ZipEntry entry, {
  required int maxBytes,
  bool collect = true,
  void Function(List<int> data)? onData,
}) {
  if (entry.uncompressedSize > maxBytes) {
    throw FormatException('ZIP entry exceeds its domain limit: ${entry.name}');
  }
  final raw = Uint8List.sublistView(
    archive,
    entry.dataOffset,
    entry.dataOffset + entry.compressedSize,
  );
  final sink = _BoundedByteSink(
    entry.uncompressedSize,
    collect: collect,
    onData: onData,
  );
  try {
    const chunkSize = 64 * 1024;
    if (entry.compressionMethod == ZipFile.zipCompressionStore) {
      for (var offset = 0; offset < raw.length; offset += chunkSize) {
        final end = offset + chunkSize < raw.length
            ? offset + chunkSize
            : raw.length;
        sink.add(Uint8List.sublistView(raw, offset, end));
      }
      sink.close();
    } else {
      final decoder = ZLibDecoder(raw: true).startChunkedConversion(sink);
      for (var offset = 0; offset < raw.length; offset += chunkSize) {
        final end = offset + chunkSize < raw.length
            ? offset + chunkSize
            : raw.length;
        decoder.add(Uint8List.sublistView(raw, offset, end));
      }
      decoder.close();
    }
  } on FormatException {
    rethrow;
  } catch (error) {
    throw FormatException('failed to inflate ${entry.name}: $error');
  }
  if (sink.length != entry.uncompressedSize) {
    throw FormatException(
      'ZIP entry decoded size disagrees with metadata: ${entry.name}',
    );
  }
  if (sink.crc32 != entry.crc32) {
    throw FormatException('ZIP entry CRC mismatch: ${entry.name}');
  }
  return sink.takeBytes();
}

Future<List<int>> _readSourceAsset(
  String path, {
  required String label,
  required int maxBytes,
  required _SaveAssetBudget budget,
  bool voice = false,
}) async {
  final file = File(path);
  final type = await FileSystemEntity.type(path, followLinks: false);
  if (type != FileSystemEntityType.file) {
    throw FileSystemException('$label must be a regular file', path);
  }
  final length = await file.length();
  if (length < 0 || length > maxBytes) {
    throw FormatException('$label exceeds its size limit: $path');
  }
  budget.add(length, voice: voice);
  final bytes = await file.readAsBytes();
  if (bytes.length != length) {
    throw FileSystemException('$label changed while it was being read', path);
  }
  return bytes;
}

Future<String> _extractPreparedAsset(
  Directory workspace,
  String outputRelative,
  _PreparedProjectArchive prepared,
  String sourceRelative,
) async {
  _validateArchivePath(outputRelative, label: 'extraction path');
  final segments = outputRelative.split('/');
  final output = p.normalize(p.joinAll([workspace.path, ...segments]));
  if (!p.isWithin(workspace.path, output)) {
    throw FormatException(
      'embedded asset escapes extraction root: $outputRelative',
    );
  }
  await Directory(p.dirname(output)).create(recursive: true);
  final file = File(output).openSync(mode: FileMode.write);
  try {
    _decodeZipEntry(
      prepared.archive,
      prepared.entries[sourceRelative]!,
      maxBytes: prepared.assetLimits[sourceRelative]!,
      collect: false,
      onData: file.writeFromSync,
    );
    file.flushSync();
  } finally {
    file.closeSync();
  }
  return output;
}

void _validateScriptRelativePath(String relative) {
  if (!_isSafePortablePath(relative) ||
      !relative.toLowerCase().endsWith('.as')) {
    throw FormatException(
      'script relative path must be a safe forward-slash .as path: $relative',
    );
  }
}

void _validateArchivePath(String value, {required String label}) {
  if (!_isSafePortablePath(value)) {
    throw FormatException('$label is not a safe portable path: $value');
  }
}

bool _isSafePortablePath(String value) {
  if (value.isEmpty ||
      utf8.encode(value).length > _maxArchivePathBytes ||
      value.startsWith('/') ||
      value.startsWith(r'\') ||
      value.contains(r'\') ||
      value.runes.any(
        (rune) => rune < 0x20 || (rune >= 0x7f && rune <= 0x9f),
      )) {
    return false;
  }
  for (final segment in value.split('/')) {
    if (segment.isEmpty ||
        segment == '.' ||
        segment == '..' ||
        segment.contains(':') ||
        RegExp(r'[<>"|?*]').hasMatch(segment) ||
        segment.endsWith(' ') ||
        segment.endsWith('.') ||
        _isWindowsReservedName(segment)) {
      return false;
    }
  }
  return true;
}

bool _isWindowsReservedName(String segment) {
  final stem = segment.split('.').first.replaceFirst(RegExp(r'[ .]+$'), '');
  final folded = stem.toUpperCase();
  if (const {
    'CON',
    'PRN',
    'AUX',
    'NUL',
    r'CLOCK$',
    r'CONIN$',
    r'CONOUT$',
  }.contains(folded)) {
    return true;
  }
  return RegExp(r'^(?:COM|LPT)[1-9]$').hasMatch(folded);
}

bool _hasEmbeddedAssets(ModProject project) =>
    project.audio.isNotEmpty ||
    project.textures.isNotEmpty ||
    project.scripts.isNotEmpty ||
    project.voice.isNotEmpty;

String _assetIndex(int index) => index.toString().padLeft(8, '0');

bool _samePath(String first, String second) {
  final normalizedFirst = p.normalize(p.absolute(first));
  final normalizedSecond = p.normalize(p.absolute(second));
  return Platform.isWindows
      ? normalizedFirst.toLowerCase() == normalizedSecond.toLowerCase()
      : normalizedFirst == normalizedSecond;
}

int _uint16(Uint8List bytes, int offset) {
  if (offset < 0 || offset > bytes.length - 2) {
    throw const FormatException('ZIP metadata read is out of bounds');
  }
  return bytes[offset] | (bytes[offset + 1] << 8);
}

int _uint32(Uint8List bytes, int offset) {
  if (offset < 0 || offset > bytes.length - 4) {
    throw const FormatException('ZIP metadata read is out of bounds');
  }
  return bytes[offset] |
      (bytes[offset + 1] << 8) |
      (bytes[offset + 2] << 16) |
      (bytes[offset + 3] << 24);
}

class _BoundedByteSink implements Sink<List<int>> {
  _BoundedByteSink(this.limit, {required bool collect, this._onData})
    : _builder = collect ? BytesBuilder(copy: true) : null;

  final int limit;
  final BytesBuilder? _builder;
  final void Function(List<int> data)? _onData;
  var _length = 0;
  var _crc32 = 0;
  var _closed = false;

  int get length => _length;
  int get crc32 => _crc32;

  @override
  void add(List<int> data) {
    if (_closed) throw StateError('bounded ZIP sink is already closed');
    if (data.length > limit - _length) {
      throw const FormatException(
        'ZIP entry expands beyond its declared bounded size',
      );
    }
    _length += data.length;
    _crc32 = getCrc32(data, _crc32);
    _builder?.add(data);
    _onData?.call(data);
  }

  @override
  void close() {
    _closed = true;
  }

  Uint8List? takeBytes() {
    if (!_closed) throw StateError('bounded ZIP sink is not closed');
    return _builder?.takeBytes();
  }
}

class _SaveAssetBudget {
  var totalBytes = 0;
  var voiceBytes = 0;

  void add(int bytes, {required bool voice}) {
    totalBytes += bytes;
    if (totalBytes > _maxTotalUncompressedBytes) {
      throw const FormatException(
        'project assets exceed the total format-1 size limit',
      );
    }
    if (voice) {
      voiceBytes += bytes;
      if (voiceBytes > _maxTotalVoiceBytes) {
        throw const FormatException(
          'dialog voice exceeds the total format-1 size limit',
        );
      }
    }
  }
}

class _ZipEntry {
  const _ZipEntry({
    required this.name,
    required this.dataOffset,
    required this.compressedSize,
    required this.uncompressedSize,
    required this.compressionMethod,
    required this.crc32,
  });

  final String name;
  final int dataOffset;
  final int compressedSize;
  final int uncompressedSize;
  final int compressionMethod;
  final int crc32;
}

class _InspectedZip {
  const _InspectedZip({required this.entries});

  final Map<String, _ZipEntry> entries;
}

class _PreparedProjectArchive {
  const _PreparedProjectArchive({
    required this.project,
    required this.archive,
    required this.entries,
    required this.assetLimits,
  });

  final ModProject project;
  final Uint8List archive;
  final Map<String, _ZipEntry> entries;
  final Map<String, int> assetLimits;
}
