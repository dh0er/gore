import 'dart:async';
import 'dart:convert';
import 'dart:io';
import 'dart:math';
import 'dart:typed_data';

import 'package:path/path.dart' as p;

/// Validates one complete candidate file.
///
/// Returning `false` (or throwing) marks that candidate invalid. The callback
/// is run on the flushed temporary file before promotion and on the promoted
/// target before old generations are removed.
typedef AtomicFileValidator = FutureOr<bool> Function(File candidate);

/// Observable durability boundaries. Primarily useful for deterministic
/// crash-injection tests; production callers normally leave the hook unset.
enum AtomicSwapPhase {
  journalFlushed,
  journalCommitted,
  tempFlushed,
  tempValidated,
  targetBackedUp,
  tempPromoted,
  targetValidated,
  cleanupComplete,
}

typedef AtomicSwapPhaseHook = FutureOr<void> Function(AtomicSwapPhase phase);

enum AtomicRepairOutcome {
  /// No interrupted target mutation was recorded.
  clean,

  /// The journal's validated temporary generation was promoted.
  promotedTemp,

  /// The target was already valid; owned swap residue was finalized.
  keptTarget,

  /// The new generation was unusable, so the validated backup was restored.
  restoredBackup,
}

class AtomicSwapException implements Exception {
  const AtomicSwapException(this.message);

  final String message;

  @override
  String toString() => 'AtomicSwapException: $message';
}

/// Repair cannot choose a validated generation without destroying evidence.
/// All journal-owned files are deliberately left in place for inspection.
class AtomicSwapRecoveryException extends AtomicSwapException {
  const AtomicSwapRecoveryException(super.message);

  @override
  String toString() => 'AtomicSwapRecoveryException: $message';
}

/// The target no longer contains the generation the caller based its edit on.
///
/// No replacement journal or content generation is created for this failure.
/// Callers can safely offer reload/compare instead of overwriting external
/// changes.
class AtomicSwapConflictException extends AtomicSwapException {
  const AtomicSwapConflictException(super.message);

  @override
  String toString() => 'AtomicSwapConflictException: $message';
}

/// Crash-recoverable replacement of complete byte files.
///
/// Operations for the same normalized target are serialized process-wide,
/// including operations issued by different [AtomicByteReplacement] instances.
/// This is deliberately not an inter-process file lock: callers must ensure a
/// target is owned by one process at a time.
/// A small bounded journal names only rigorously checked sibling artifacts; an
/// interrupted operation is repaired before the next replacement or by an
/// explicit [repair] call.
///
/// `flush: true` is used for temporary content and the journal. Dart exposes no
/// portable parent-directory fsync, however, so this helper can close process
/// crash windows and preserve a validated generation at every mutation step,
/// but cannot promise metadata durability across sudden power loss on every
/// filesystem.
class AtomicByteReplacement {
  AtomicByteReplacement({
    this.onPhase,
    this.directoryScanLimit = defaultDirectoryScanLimit,
    String Function()? operationIdFactory,
  }) : _operationIdFactory = operationIdFactory ?? _randomOperationId {
    if (directoryScanLimit <= 0) {
      throw ArgumentError.value(
        directoryScanLimit,
        'directoryScanLimit',
        'must be positive',
      );
    }
  }

  static const int journalFormat = 1;
  static const int maxJournalBytes = 4096;
  static const int maxPendingJournals = 32;
  static const int defaultDirectoryScanLimit = 16384;
  static final Map<String, Future<void>> _targetTails = {};
  static final Random _secureRandom = Random.secure();

  final AtomicSwapPhaseHook? onPhase;
  final int directoryScanLimit;
  final String Function() _operationIdFactory;

  /// The fixed, target-specific journal path. Exposed for startup discovery and
  /// diagnostics; callers must not modify the file themselves.
  static String journalPathFor(File target) {
    final absolute = _normalizedAbsolute(target.path);
    return '$absolute.gore-swap.json';
  }

  /// Replace [target] with an immutable copy of [bytes].
  ///
  /// A prior interrupted swap is repaired first. A failed operation deliberately
  /// leaves its journal and generations intact so a later call can recover them.
  Future<void> replace({
    required File target,
    required List<int> bytes,
    required AtomicFileValidator validate,
  }) {
    final normalizedTarget = File(_normalizedAbsolute(target.path));
    final snapshot = Uint8List.fromList(bytes);
    return _serialized(normalizedTarget, () async {
      await _repairUnlocked(normalizedTarget, validate);
      await _replaceUnlocked(normalizedTarget, snapshot, validate);
    });
  }

  /// Replace [target] only if its complete current bytes still equal
  /// [expectedBytes]. A null expectation means the target must still be absent.
  ///
  /// The comparison runs after interrupted-swap repair and inside the same
  /// process-wide target lane as publication. This prevents a queued writer
  /// from checking stale state before an earlier writer finishes. A separate
  /// project/session lock is still required to exclude other processes.
  Future<void> replaceIfUnchanged({
    required File target,
    required List<int> bytes,
    required List<int>? expectedBytes,
    required AtomicFileValidator validate,
  }) {
    final normalizedTarget = File(_normalizedAbsolute(target.path));
    final snapshot = Uint8List.fromList(bytes);
    final expectedSnapshot = expectedBytes == null
        ? null
        : Uint8List.fromList(expectedBytes);
    return _serialized(normalizedTarget, () async {
      await _repairUnlocked(normalizedTarget, validate);
      if (!await _hasExpectedBytes(normalizedTarget, expectedSnapshot)) {
        throw AtomicSwapConflictException(
          'target changed since the caller captured its base generation: '
          '${normalizedTarget.path}',
        );
      }
      await _replaceUnlocked(normalizedTarget, snapshot, validate);
    });
  }

  /// Repair an interrupted swap recorded for [target].
  Future<AtomicRepairOutcome> repair({
    required File target,
    required AtomicFileValidator validate,
  }) {
    final normalizedTarget = File(_normalizedAbsolute(target.path));
    return _serialized(
      normalizedTarget,
      () => _repairUnlocked(normalizedTarget, validate),
    );
  }

  Future<void> _replaceUnlocked(
    File target,
    Uint8List bytes,
    AtomicFileValidator validate,
  ) async {
    final targetExisted =
        await _safeType(target, label: 'target') == FileSystemEntityType.file;
    await target.parent.create(recursive: true);

    final operationId = _operationIdFactory();
    if (!_operationIdPattern.hasMatch(operationId)) {
      throw AtomicSwapException(
        'operation ID must be exactly 32 lowercase hexadecimal characters',
      );
    }

    final names = _SwapNames.forTarget(target, operationId);
    _validateDerivedNames(target, names);
    final temp = File(p.join(target.parent.path, names.tempName));
    final backup = File(p.join(target.parent.path, names.backupName));
    final journal = File(journalPathFor(target));
    final pendingJournal = File(
      p.join(target.parent.path, names.pendingJournalName),
    );

    for (final artifact in [temp, backup, pendingJournal]) {
      await _requireMissing(artifact, label: 'fresh swap artifact');
    }
    await _requireMissing(journal, label: 'swap journal after repair');

    // Commit the bounded ownership record before creating even the temporary
    // content generation. From this point onward every content artifact is
    // named by the journal, so a crash or failed validation is repairable.
    final record = _SwapJournal(
      operationId: operationId,
      targetName: p.basename(target.path),
      tempName: names.tempName,
      backupName: names.backupName,
      targetExisted: targetExisted,
    );
    final journalBytes = utf8.encode('${jsonEncode(record.toJson())}\n');
    if (journalBytes.length > maxJournalBytes) {
      throw const AtomicSwapException('generated swap journal is too large');
    }
    await pendingJournal.writeAsBytes(journalBytes, flush: true);
    await _phase(AtomicSwapPhase.journalFlushed);
    await pendingJournal.rename(journal.path);
    await _phase(AtomicSwapPhase.journalCommitted);

    await temp.writeAsBytes(bytes, flush: true);
    await _phase(AtomicSwapPhase.tempFlushed);
    if (!await _isValidRegularFile(temp, validate)) {
      throw AtomicSwapException(
        'temporary generation failed validation: ${temp.path}',
      );
    }
    await _phase(AtomicSwapPhase.tempValidated);

    final targetType = await _safeType(target, label: 'target');
    if (targetType == FileSystemEntityType.file) {
      await target.rename(backup.path);
      await _phase(AtomicSwapPhase.targetBackedUp);
    }

    await temp.rename(target.path);
    await _phase(AtomicSwapPhase.tempPromoted);
    if (!await _isValidRegularFile(target, validate)) {
      throw AtomicSwapException(
        'promoted target failed validation: ${target.path}',
      );
    }
    await _phase(AtomicSwapPhase.targetValidated);

    await _deleteOwnedFileIfPresent(backup, label: 'validated backup');
    await _deleteOwnedFileIfPresent(journal, label: 'completed swap journal');
    await _phase(AtomicSwapPhase.cleanupComplete);
  }

  Future<AtomicRepairOutcome> _repairUnlocked(
    File target,
    AtomicFileValidator validate,
  ) async {
    await _requireRegularOrMissing(target, label: 'target');
    await _cleanPendingJournalStaging(target);
    final journal = File(journalPathFor(target));
    final journalType = await _safeType(journal, label: 'swap journal');
    if (journalType == FileSystemEntityType.notFound) {
      return AtomicRepairOutcome.clean;
    }

    final record = await _readJournal(journal);
    final names = _SwapNames.forTarget(target, record.operationId);
    if (record.targetName != p.basename(target.path) ||
        record.tempName != names.tempName ||
        record.backupName != names.backupName) {
      throw AtomicSwapRecoveryException(
        'swap journal contains non-owned or non-sibling artifact names: '
        '${journal.path}',
      );
    }
    _validateDerivedNames(target, names);

    final temp = File(p.join(target.parent.path, record.tempName));
    final backup = File(p.join(target.parent.path, record.backupName));
    await _requireRegularOrMissing(temp, label: 'journal temporary');
    await _requireRegularOrMissing(backup, label: 'journal backup');

    var targetValid = await _isValidRegularFile(target, validate);
    final tempValid = await _isValidRegularFile(temp, validate);
    final backupValid = await _isValidRegularFile(backup, validate);

    if (!targetValid && !tempValid && !backupValid) {
      final tempType = await _safeType(temp, label: 'journal temporary');
      final backupType = await _safeType(backup, label: 'journal backup');
      if (!record.targetExisted &&
          tempType == FileSystemEntityType.notFound &&
          backupType == FileSystemEntityType.notFound) {
        // The journal was committed for a brand-new target, but the process
        // stopped before creating its content temp. No generation ever existed
        // and there is no content evidence to preserve.
        await _deleteOwnedFileIfPresent(
          journal,
          label: 'empty fresh-target swap journal',
        );
        return AtomicRepairOutcome.clean;
      }
      throw AtomicSwapRecoveryException(
        'no valid target, temporary, or backup generation exists; preserving '
        'all swap evidence for ${target.path}',
      );
    }

    if (tempValid) {
      final targetType = await _safeType(target, label: 'target');
      final backupType = await _safeType(backup, label: 'journal backup');
      if (targetType == FileSystemEntityType.file) {
        if (backupType != FileSystemEntityType.notFound) {
          throw AtomicSwapRecoveryException(
            'ambiguous recovery has target, temporary, and backup generations; '
            'preserving all swap evidence for ${target.path}',
          );
        }
        await target.rename(backup.path);
      }
      await temp.rename(target.path);
      if (!await _isValidRegularFile(target, validate)) {
        throw AtomicSwapRecoveryException(
          'promoted temporary generation failed post-promotion validation; '
          'preserving swap evidence for ${target.path}',
        );
      }
      targetValid = true;
      await _deleteOwnedFileIfPresent(backup, label: 'repaired backup');
      await _deleteOwnedFileIfPresent(journal, label: 'repaired swap journal');
      return AtomicRepairOutcome.promotedTemp;
    }

    if (targetValid) {
      await _deleteOwnedFileIfPresent(temp, label: 'invalid temporary');
      await _deleteOwnedFileIfPresent(backup, label: 'superseded backup');
      await _deleteOwnedFileIfPresent(journal, label: 'repaired swap journal');
      return AtomicRepairOutcome.keptTarget;
    }

    // The backup is now the only validated generation. Preserve any invalid
    // target as a uniquely named sibling before restoring it.
    final targetType = await _safeType(target, label: 'invalid target');
    if (targetType == FileSystemEntityType.file) {
      final quarantine = File('${target.path}.invalid-${record.operationId}');
      _validateQuarantinePath(target, quarantine, record.operationId);
      await _requireMissing(quarantine, label: 'invalid target quarantine');
      await target.rename(quarantine.path);
    }
    await backup.rename(target.path);
    if (!await _isValidRegularFile(target, validate)) {
      throw AtomicSwapRecoveryException(
        'restored backup failed post-restore validation; preserving swap '
        'evidence for ${target.path}',
      );
    }
    await _deleteOwnedFileIfPresent(temp, label: 'invalid temporary');
    await _deleteOwnedFileIfPresent(journal, label: 'repaired swap journal');
    return AtomicRepairOutcome.restoredBackup;
  }

  Future<_SwapJournal> _readJournal(File journal) async {
    final length = await journal.length();
    if (length <= 0 || length > maxJournalBytes) {
      throw AtomicSwapRecoveryException(
        'swap journal size is outside 1..$maxJournalBytes bytes: '
        '${journal.path}',
      );
    }
    try {
      final decoded = jsonDecode(await journal.readAsString());
      if (decoded is! Map) {
        throw const FormatException('journal root is not an object');
      }
      return _SwapJournal.fromJson(decoded.cast<String, Object?>());
    } on AtomicSwapRecoveryException {
      rethrow;
    } catch (error) {
      throw AtomicSwapRecoveryException(
        'invalid swap journal ${journal.path}: $error',
      );
    }
  }

  Future<void> _phase(AtomicSwapPhase phase) async {
    await onPhase?.call(phase);
  }

  /// Remove journal *staging* files abandoned before the fixed journal was
  /// committed. Content temps are created only after that commit, so these
  /// owned files can never describe a target mutation or content generation.
  /// Enumeration is bounded to avoid attacker-controlled directory work.
  Future<void> _cleanPendingJournalStaging(File target) async {
    final parent = target.parent;
    if (!await parent.exists()) return;
    final base = RegExp.escape(p.basename(target.path));
    final pattern = RegExp('^$base\\.gore-swap-[0-9a-f]{32}\\.journal\\.tmp\$');
    var scanned = 0;
    final matches = <File>[];
    await for (final entity in parent.list(followLinks: false)) {
      scanned++;
      if (scanned > directoryScanLimit) {
        throw AtomicSwapRecoveryException(
          'more than $directoryScanLimit directory entries must be scanned '
          'for ${target.path}; preserving all pending journal evidence',
        );
      }
      if (!pattern.hasMatch(p.basename(entity.path))) continue;
      matches.add(File(entity.path));
      if (matches.length > maxPendingJournals) {
        throw AtomicSwapRecoveryException(
          'more than $maxPendingJournals pending journals exist for '
          '${target.path}; refusing unbounded cleanup',
        );
      }
    }
    // Do not mutate anything until the complete bounded scan has succeeded.
    // Otherwise directory order could make us delete evidence before learning
    // that the scan limit was exceeded.
    for (final file in matches) {
      if (!_samePath(
        _normalizedAbsolute(file.parent.path),
        _normalizedAbsolute(parent.path),
      )) {
        throw AtomicSwapRecoveryException(
          'pending journal is not a direct target sibling: ${file.path}',
        );
      }
      if (await _safeType(file, label: 'pending swap journal') !=
          FileSystemEntityType.file) {
        throw AtomicSwapRecoveryException(
          'pending swap journal is not a regular file: ${file.path}',
        );
      }
      await file.delete();
    }
  }

  static Future<T> _serialized<T>(File target, Future<T> Function() action) {
    final key = _serializationKey(target.path);
    final prior = _targetTails[key] ?? Future<void>.value();
    final result = prior.catchError((Object _) {}).then((_) => action());
    late final Future<void> tail;
    tail = result.then<void>((_) {}, onError: (Object _, StackTrace _) {});
    _targetTails[key] = tail;
    unawaited(
      tail.whenComplete(() {
        if (identical(_targetTails[key], tail)) {
          _targetTails.remove(key);
        }
      }),
    );
    return result;
  }

  static Future<bool> _isValidRegularFile(
    File file,
    AtomicFileValidator validate,
  ) async {
    if (await _safeType(file, label: 'validation candidate') !=
        FileSystemEntityType.file) {
      return false;
    }
    try {
      return await validate(file);
    } catch (_) {
      return false;
    }
  }

  static Future<bool> _hasExpectedBytes(File file, Uint8List? expected) async {
    final type = await _safeType(file, label: 'expected target generation');
    if (expected == null) return type == FileSystemEntityType.notFound;
    if (type != FileSystemEntityType.file ||
        await file.length() != expected.length) {
      return false;
    }

    final handle = await file.open(mode: FileMode.read);
    try {
      const chunkSize = 64 * 1024;
      var offset = 0;
      while (offset < expected.length) {
        final count = min(chunkSize, expected.length - offset);
        final actual = await handle.read(count);
        if (actual.length != count) return false;
        for (var index = 0; index < count; index++) {
          if (actual[index] != expected[offset + index]) return false;
        }
        offset += count;
      }
      return (await handle.read(1)).isEmpty;
    } finally {
      await handle.close();
    }
  }

  static Future<FileSystemEntityType> _safeType(
    File file, {
    required String label,
  }) async {
    final type = await FileSystemEntity.type(file.path, followLinks: false);
    if (type == FileSystemEntityType.link) {
      throw AtomicSwapRecoveryException(
        '$label must not be a symbolic link: ${file.path}',
      );
    }
    if (type != FileSystemEntityType.notFound &&
        type != FileSystemEntityType.file) {
      throw AtomicSwapRecoveryException(
        '$label must be a regular file or be absent: ${file.path}',
      );
    }
    return type;
  }

  static Future<void> _requireRegularOrMissing(
    File file, {
    required String label,
  }) async {
    await _safeType(file, label: label);
  }

  static Future<void> _requireMissing(
    File file, {
    required String label,
  }) async {
    if (await _safeType(file, label: label) != FileSystemEntityType.notFound) {
      throw AtomicSwapRecoveryException('$label already exists: ${file.path}');
    }
  }

  static Future<void> _deleteOwnedFileIfPresent(
    File file, {
    required String label,
  }) async {
    final type = await _safeType(file, label: label);
    if (type == FileSystemEntityType.file) {
      await file.delete();
    }
  }

  static void _validateDerivedNames(File target, _SwapNames names) {
    final parent = _normalizedAbsolute(target.parent.path);
    for (final name in [
      names.tempName,
      names.backupName,
      names.pendingJournalName,
    ]) {
      if (name.isEmpty ||
          name != p.basename(name) ||
          _normalizedAbsolute(p.join(parent, name)) ==
              _normalizedAbsolute(target.path) ||
          !_samePath(
            _normalizedAbsolute(p.dirname(p.join(parent, name))),
            parent,
          )) {
        throw AtomicSwapRecoveryException(
          'swap artifact is not a safe direct sibling of ${target.path}: $name',
        );
      }
    }
  }

  static void _validateQuarantinePath(
    File target,
    File quarantine,
    String operationId,
  ) {
    final expected = '${p.basename(target.path)}.invalid-$operationId';
    final parent = _normalizedAbsolute(target.parent.path);
    if (p.basename(quarantine.path) != expected ||
        !_samePath(_normalizedAbsolute(quarantine.parent.path), parent)) {
      throw AtomicSwapRecoveryException(
        'invalid quarantine is not a safe target sibling: ${quarantine.path}',
      );
    }
  }

  static String _normalizedAbsolute(String path) =>
      p.normalize(p.absolute(path));

  static String _serializationKey(String path) {
    final normalized = _normalizedAbsolute(path);
    return Platform.isWindows ? normalized.toLowerCase() : normalized;
  }

  static bool _samePath(String first, String second) =>
      _serializationKey(first) == _serializationKey(second);

  static String _randomOperationId() {
    final buffer = StringBuffer();
    for (var index = 0; index < 16; index++) {
      buffer.write(
        _secureRandom.nextInt(256).toRadixString(16).padLeft(2, '0'),
      );
    }
    return buffer.toString();
  }

  static final RegExp _operationIdPattern = RegExp(r'^[0-9a-f]{32}$');
}

class _SwapNames {
  const _SwapNames({
    required this.tempName,
    required this.backupName,
    required this.pendingJournalName,
  });

  factory _SwapNames.forTarget(File target, String operationId) {
    final base = p.basename(target.path);
    return _SwapNames(
      tempName: '$base.gore-swap-$operationId.tmp',
      backupName: '$base.gore-swap-$operationId.bak',
      pendingJournalName: '$base.gore-swap-$operationId.journal.tmp',
    );
  }

  final String tempName;
  final String backupName;
  final String pendingJournalName;
}

class _SwapJournal {
  const _SwapJournal({
    required this.operationId,
    required this.targetName,
    required this.tempName,
    required this.backupName,
    required this.targetExisted,
  });

  factory _SwapJournal.fromJson(Map<String, Object?> json) {
    const keys = {
      'format',
      'operation_id',
      'target_name',
      'temp_name',
      'backup_name',
      'target_existed',
    };
    if (json.keys.toSet().difference(keys).isNotEmpty ||
        keys.difference(json.keys.toSet()).isNotEmpty) {
      throw const AtomicSwapRecoveryException(
        'swap journal fields do not match format 1',
      );
    }
    if (json['format'] != AtomicByteReplacement.journalFormat) {
      throw AtomicSwapRecoveryException(
        'unsupported swap journal format: ${json['format']}',
      );
    }
    final operationId = json['operation_id'];
    final targetName = json['target_name'];
    final tempName = json['temp_name'];
    final backupName = json['backup_name'];
    final targetExisted = json['target_existed'];
    if (operationId is! String ||
        targetName is! String ||
        tempName is! String ||
        backupName is! String ||
        targetExisted is! bool ||
        !AtomicByteReplacement._operationIdPattern.hasMatch(operationId)) {
      throw const AtomicSwapRecoveryException(
        'swap journal has invalid field types or operation ID',
      );
    }
    return _SwapJournal(
      operationId: operationId,
      targetName: targetName,
      tempName: tempName,
      backupName: backupName,
      targetExisted: targetExisted,
    );
  }

  final String operationId;
  final String targetName;
  final String tempName;
  final String backupName;
  final bool targetExisted;

  Map<String, Object?> toJson() => {
    'format': AtomicByteReplacement.journalFormat,
    'operation_id': operationId,
    'target_name': targetName,
    'temp_name': tempName,
    'backup_name': backupName,
    'target_existed': targetExisted,
  };
}
