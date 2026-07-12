import 'dart:convert';
import 'dart:io';
import 'dart:math';

import 'package:path/path.dart' as p;

const int _maxLockRecordBytes = 4096;
final RegExp _ownerTokenPattern = RegExp(r'^[0-9a-f]{32}$');

class ManagedProjectLockException implements Exception {
  const ManagedProjectLockException(this.message);

  final String message;

  @override
  String toString() => 'ManagedProjectLockException: $message';
}

/// Another session in this process or another process owns the working tree.
class ManagedProjectAlreadyOpenException extends ManagedProjectLockException {
  const ManagedProjectAlreadyOpenException(super.message);

  @override
  String toString() => 'ManagedProjectAlreadyOpenException: $message';
}

/// An exclusive operating-system lock for one managed working project.
///
/// The `.gore/session.lock` file deliberately survives release and crashes. Its
/// contents are diagnostic only; ownership is determined exclusively by the
/// live file lock. Leaving the inode in place avoids an unlock/delete/recreate
/// race between Studio processes.
class ManagedProjectSessionLock {
  ManagedProjectSessionLock._({
    required this.projectRoot,
    required this.ownerToken,
    required RandomAccessFile handle,
    required String ownershipKey,
  }) : _handle = handle,
       _ownershipKey = ownershipKey;

  static final Set<String> _ownedOrPending = <String>{};
  static final Random _secureRandom = Random.secure();

  final String projectRoot;
  final String ownerToken;
  final RandomAccessFile _handle;
  final String _ownershipKey;
  Future<void>? _releaseFuture;

  String get lockPath => p.join(projectRoot, '.gore', 'session.lock');

  static Future<ManagedProjectSessionLock> acquire(
    Directory projectRoot, {
    String? ownerToken,
    DateTime? openedAtUtc,
  }) async {
    final root = p.normalize(p.absolute(projectRoot.path));
    final key = Platform.isWindows ? root.toLowerCase() : root;
    final rootType = await FileSystemEntity.type(root, followLinks: false);
    if (rootType != FileSystemEntityType.directory) {
      throw ManagedProjectLockException(
        'managed project root must be an existing real directory: $root',
      );
    }
    if (!_ownedOrPending.add(key)) {
      throw ManagedProjectAlreadyOpenException(
        'managed project is already open in this Studio process: $root',
      );
    }

    RandomAccessFile? handle;
    var locked = false;
    try {
      final controlDirectory = Directory(p.join(root, '.gore'));
      var controlType = await FileSystemEntity.type(
        controlDirectory.path,
        followLinks: false,
      );
      if (controlType == FileSystemEntityType.notFound) {
        await controlDirectory.create();
        controlType = await FileSystemEntity.type(
          controlDirectory.path,
          followLinks: false,
        );
      }
      if (controlType != FileSystemEntityType.directory) {
        throw ManagedProjectLockException(
          'managed project control path must be a real directory: '
          '${controlDirectory.path}',
        );
      }

      final lockFile = File(p.join(controlDirectory.path, 'session.lock'));
      final lockType = await FileSystemEntity.type(
        lockFile.path,
        followLinks: false,
      );
      if (lockType != FileSystemEntityType.notFound &&
          lockType != FileSystemEntityType.file) {
        throw ManagedProjectLockException(
          'managed project lock path must be a regular file or absent: '
          '${lockFile.path}',
        );
      }

      try {
        handle = await lockFile.open(mode: FileMode.writeOnlyAppend);
      } on FileSystemException catch (error) {
        throw ManagedProjectLockException(
          'could not open managed project lock: ${error.osError?.message ?? 'I/O error'}',
        );
      }
      try {
        await handle.lock(FileLock.exclusive);
        locked = true;
      } on FileSystemException {
        throw ManagedProjectAlreadyOpenException(
          'managed project is already open in another Studio process: $root',
        );
      }

      final token = ownerToken ?? _randomOwnerToken();
      if (!_ownerTokenPattern.hasMatch(token)) {
        throw ArgumentError.value(
          token,
          'ownerToken',
          'must be exactly 32 lowercase hexadecimal characters',
        );
      }
      final opened = (openedAtUtc ?? DateTime.now().toUtc()).toUtc();
      final record = utf8.encode(
        '${jsonEncode({'format': 1, 'owner_token': token, 'pid': pid, 'opened_at_utc': opened.toIso8601String()})}\n',
      );
      if (record.length > _maxLockRecordBytes) {
        throw const ManagedProjectLockException(
          'generated managed project lock record is too large',
        );
      }
      await handle.setPosition(0);
      await handle.truncate(0);
      await handle.writeFrom(record);
      await handle.flush();

      return ManagedProjectSessionLock._(
        projectRoot: root,
        ownerToken: token,
        handle: handle,
        ownershipKey: key,
      );
    } catch (error, stackTrace) {
      if (handle != null) {
        if (locked) {
          try {
            await handle.unlock();
          } catch (_) {}
        }
        try {
          await handle.close();
        } catch (_) {}
      }
      _ownedOrPending.remove(key);
      Error.throwWithStackTrace(error, stackTrace);
    }
  }

  Future<void> release() => _releaseFuture ??= _releaseOnce();

  Future<void> _releaseOnce() async {
    Object? firstError;
    StackTrace? firstStackTrace;
    try {
      await _handle.unlock();
    } catch (error, stackTrace) {
      firstError = error;
      firstStackTrace = stackTrace;
    }
    try {
      await _handle.close();
    } catch (error, stackTrace) {
      firstError ??= error;
      firstStackTrace ??= stackTrace;
    } finally {
      _ownedOrPending.remove(_ownershipKey);
    }
    if (firstError != null) {
      Error.throwWithStackTrace(firstError, firstStackTrace!);
    }
  }

  static String _randomOwnerToken() {
    final buffer = StringBuffer();
    for (var index = 0; index < 16; index++) {
      buffer.write(
        _secureRandom.nextInt(256).toRadixString(16).padLeft(2, '0'),
      );
    }
    return buffer.toString();
  }
}
