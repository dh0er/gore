import 'dart:async';
import 'dart:convert';
import 'dart:io';
import 'dart:typed_data';

import 'package:path/path.dart' as p;

import '../core/mod_ffi.dart';
import 'managed_project_lock.dart';
import 'project_atomic_io.dart';

const int _maxManagedHeadBytes = 64 * 1024;

class ManagedProjectSessionException implements Exception {
  const ManagedProjectSessionException(this.message);

  final String message;

  @override
  String toString() => 'ManagedProjectSessionException: $message';
}

class ManagedProjectAlreadyInitializedException
    extends ManagedProjectSessionException {
  const ManagedProjectAlreadyInitializedException(super.message);

  @override
  String toString() => 'ManagedProjectAlreadyInitializedException: $message';
}

class ManagedProjectHeadConflictException
    extends ManagedProjectSessionException {
  const ManagedProjectHeadConflictException(super.message);

  @override
  String toString() => 'ManagedProjectHeadConflictException: $message';
}

class ManagedProjectVerificationException
    extends ManagedProjectSessionException {
  const ManagedProjectVerificationException(super.message);

  @override
  String toString() => 'ManagedProjectVerificationException: $message';
}

class ManagedProjectSessionClosedException
    extends ManagedProjectSessionException {
  const ManagedProjectSessionClosedException(super.message);

  @override
  String toString() => 'ManagedProjectSessionClosedException: $message';
}

/// Narrow seam over the native managed-store API.
///
/// The interface keeps session durability and ordering independently testable;
/// production callers normally use [ModFfiManagedAuthoringStore].
abstract interface class ManagedAuthoringStore {
  Future<AuthoringStoreOpenedResult> open({
    required String root,
    required AuthoringAssetVerification verification,
    required AuthoringValidationProfile profile,
  });

  Future<AuthoringCheckpointPreparation> prepareCheckpoint({
    required String root,
    required AuthoringWorkingHead? expectedHead,
    required String projectJson,
    required AuthoringValidationProfile profile,
  });

  Future<AuthoringStoreOpenedResult> openHeadBytes({
    required String root,
    required AuthoringWorkingHead head,
    required AuthoringAssetVerification verification,
    required AuthoringValidationProfile profile,
  });
}

class ModFfiManagedAuthoringStore implements ManagedAuthoringStore {
  const ModFfiManagedAuthoringStore(this.ffi);

  final ModFfi ffi;

  @override
  Future<AuthoringStoreOpenedResult> open({
    required String root,
    required AuthoringAssetVerification verification,
    required AuthoringValidationProfile profile,
  }) => ffi.authoringStoreOpen(
    root: root,
    verification: verification,
    profile: profile,
  );

  @override
  Future<AuthoringCheckpointPreparation> prepareCheckpoint({
    required String root,
    required AuthoringWorkingHead? expectedHead,
    required String projectJson,
    required AuthoringValidationProfile profile,
  }) => ffi.authoringStorePrepareCheckpoint(
    root: root,
    expectedHead: expectedHead,
    projectJson: projectJson,
    profile: profile,
  );

  @override
  Future<AuthoringStoreOpenedResult> openHeadBytes({
    required String root,
    required AuthoringWorkingHead head,
    required AuthoringAssetVerification verification,
    required AuthoringValidationProfile profile,
  }) => ffi.authoringStoreOpenHeadBytes(
    root: root,
    head: head,
    verification: verification,
    profile: profile,
  );
}

/// Exclusive, crash-recoverable editing session for one format-2 working tree.
///
/// Immutable objects are prepared by the native store. The only Dart-owned
/// mutation is publication of the fixed `gore-project.json` head. Publication
/// is an exact byte-for-byte CAS and every candidate, repaired generation, and
/// published generation is reopened using full asset verification.
class ManagedAuthoringProjectSession {
  ManagedAuthoringProjectSession._({
    required this.root,
    required this._store,
    required this._lock,
    required this._replacement,
    required this._profile,
    required this._opened,
  });

  final Directory root;
  final ManagedAuthoringStore _store;
  final ManagedProjectSessionLock _lock;
  final AtomicByteReplacement _replacement;
  final AuthoringValidationProfile _profile;

  AuthoringStoreOpenedResult _opened;
  Future<void> _tail = Future<void>.value();
  Future<void>? _closeFuture;
  bool _closeRequested = false;
  bool _closed = false;
  bool _requiresReopen = false;

  String get projectJson => _opened.projectJson;
  AuthoringWorkingHead get head => _opened.head;
  List<AuthoringDiagnostic> get diagnostics => _opened.diagnostics;
  bool get blocksBuild => _opened.blocksBuild;
  bool get isClosed => _closed;

  /// True after an I/O or verification failure leaves publication state
  /// uncertain. Close and reopen before attempting another edit.
  bool get requiresReopen => _requiresReopen;

  File get headFile => File(p.join(root.path, 'gore-project.json'));

  static Future<ManagedAuthoringProjectSession> create({
    required Directory root,
    required ManagedAuthoringStore store,
    required String projectJson,
    required AuthoringValidationProfile profile,
    AtomicByteReplacement? replacement,
  }) async {
    final lock = await ManagedProjectSessionLock.acquire(root);
    final normalizedRoot = Directory(lock.projectRoot);
    final byteReplacement = replacement ?? AtomicByteReplacement();
    try {
      final operations = _ManagedSessionOperations(
        root: normalizedRoot,
        store: store,
        replacement: byteReplacement,
        profile: profile,
      );
      final headType = await FileSystemEntity.type(
        operations.headFile.path,
        followLinks: false,
      );
      final journalType = await FileSystemEntity.type(
        AtomicByteReplacement.journalPathFor(operations.headFile),
        followLinks: false,
      );
      if (headType != FileSystemEntityType.notFound ||
          journalType != FileSystemEntityType.notFound) {
        throw ManagedProjectAlreadyInitializedException(
          'managed project already has a head or pending recovery journal: '
          '${operations.headFile.path}',
        );
      }
      // A create operation must never select or publish a generation from a
      // pre-existing fixed journal. With both fixed artifacts absent, repair
      // can only discard journal staging that predates any content mutation.
      await operations.repairHead();

      final prepared = await store.prepareCheckpoint(
        root: normalizedRoot.path,
        expectedHead: null,
        projectJson: projectJson,
        profile: profile,
      );
      await operations.verifyPreparedCheckpoint(
        prepared.head,
        expectedProjectJson: projectJson,
      );
      await operations.publish(prepared.head, expectedHead: null);
      final opened = await operations.openPublished(
        expectedHead: prepared.head,
        expectedProjectJson: projectJson,
      );
      return ManagedAuthoringProjectSession._(
        root: normalizedRoot,
        store: store,
        lock: lock,
        replacement: byteReplacement,
        profile: profile,
        opened: opened,
      );
    } catch (error, stackTrace) {
      try {
        await lock.release();
      } catch (_) {}
      Error.throwWithStackTrace(error, stackTrace);
    }
  }

  static Future<ManagedAuthoringProjectSession> open({
    required Directory root,
    required ManagedAuthoringStore store,
    required AuthoringValidationProfile profile,
    AtomicByteReplacement? replacement,
  }) async {
    final lock = await ManagedProjectSessionLock.acquire(root);
    final normalizedRoot = Directory(lock.projectRoot);
    final byteReplacement = replacement ?? AtomicByteReplacement();
    try {
      final operations = _ManagedSessionOperations(
        root: normalizedRoot,
        store: store,
        replacement: byteReplacement,
        profile: profile,
      );
      await operations.repairHead();
      final opened = await operations.openPublished();
      return ManagedAuthoringProjectSession._(
        root: normalizedRoot,
        store: store,
        lock: lock,
        replacement: byteReplacement,
        profile: profile,
        opened: opened,
      );
    } catch (error, stackTrace) {
      try {
        await lock.release();
      } catch (_) {}
      Error.throwWithStackTrace(error, stackTrace);
    }
  }

  /// Save a captured canonical format-2 snapshot in invocation order.
  Future<void> save(String projectJson) {
    if (_closeRequested) {
      return Future<void>.error(
        const ManagedProjectSessionClosedException(
          'managed project session is closing or closed',
        ),
      );
    }
    final capturedProjectJson = projectJson;
    return _enqueue(() async {
      if (_requiresReopen) {
        throw const ManagedProjectVerificationException(
          'managed project must be reopened after an uncertain publication',
        );
      }
      final oldHead = _opened.head;
      final operations = _ManagedSessionOperations(
        root: root,
        store: _store,
        replacement: _replacement,
        profile: _profile,
      );
      try {
        await operations.requirePublishedHead(oldHead);
      } on ManagedProjectSessionException {
        _requiresReopen = true;
        rethrow;
      }
      final AuthoringCheckpointPreparation prepared;
      try {
        prepared = await _store.prepareCheckpoint(
          root: root.path,
          expectedHead: oldHead,
          projectJson: capturedProjectJson,
          profile: _profile,
        );
      } on ModFfiException catch (error) {
        if (error.code == 'AUTHORING_STORE_HEAD_CONFLICT') {
          _requiresReopen = true;
          throw ManagedProjectHeadConflictException(error.message);
        }
        if (_prepareErrorRequiresReopen(error.code)) {
          _requiresReopen = true;
          throw ManagedProjectVerificationException(error.message);
        }
        rethrow;
      } on ManagedProjectHeadConflictException {
        _requiresReopen = true;
        rethrow;
      }
      await operations.verifyPreparedCheckpoint(
        prepared.head,
        expectedProjectJson: capturedProjectJson,
      );

      try {
        await operations.publish(prepared.head, expectedHead: oldHead);
      } on AtomicSwapConflictException catch (error) {
        _requiresReopen = true;
        throw ManagedProjectHeadConflictException(error.message);
      } on AtomicSwapException {
        _requiresReopen = true;
        rethrow;
      }

      try {
        _opened = await operations.openPublished(
          expectedHead: prepared.head,
          expectedProjectJson: capturedProjectJson,
        );
      } catch (_) {
        _requiresReopen = true;
        rethrow;
      }
    });
  }

  /// Wait for earlier saves, release the OS lock once, and reject new saves.
  Future<void> close() {
    final existing = _closeFuture;
    if (existing != null) return existing;
    _closeRequested = true;
    final result = _enqueue(() async {
      try {
        await _lock.release();
      } finally {
        _closed = true;
      }
    }, permitClosing: true);
    _closeFuture = result;
    return result;
  }

  Future<T> _enqueue<T>(
    Future<T> Function() operation, {
    bool permitClosing = false,
  }) {
    if (_closed || (_closeRequested && !permitClosing)) {
      return Future<T>.error(
        const ManagedProjectSessionClosedException(
          'managed project session is closing or closed',
        ),
      );
    }
    final result = _tail.then((_) => operation());
    _tail = result.then<void>((_) {}, onError: (Object _, StackTrace _) {});
    return result;
  }
}

class _ManagedSessionOperations {
  const _ManagedSessionOperations({
    required this.root,
    required this.store,
    required this.replacement,
    required this.profile,
  });

  final Directory root;
  final ManagedAuthoringStore store;
  final AtomicByteReplacement replacement;
  final AuthoringValidationProfile profile;

  File get headFile => File(p.join(root.path, 'gore-project.json'));

  Future<AtomicRepairOutcome> repairHead() =>
      replacement.repair(target: headFile, validate: _validateHeadCandidate);

  Future<void> verifyPreparedCheckpoint(
    AuthoringWorkingHead head, {
    required String expectedProjectJson,
  }) async {
    final opened = await store.openHeadBytes(
      root: root.path,
      head: head,
      verification: AuthoringAssetVerification.full,
      profile: profile,
    );
    _requireExactOpened(
      opened,
      expectedHead: head,
      expectedProjectJson: expectedProjectJson,
      context: 'prepared checkpoint',
    );
  }

  Future<void> requirePublishedHead(AuthoringWorkingHead expectedHead) async {
    final actualHead = await _readCanonicalHead(headFile);
    if (actualHead.canonicalJson != expectedHead.canonicalJson) {
      throw const ManagedProjectHeadConflictException(
        'managed project head changed since the session opened it',
      );
    }
  }

  Future<void> publish(
    AuthoringWorkingHead head, {
    required AuthoringWorkingHead? expectedHead,
  }) => replacement.replaceIfUnchanged(
    target: headFile,
    bytes: utf8.encode(head.canonicalJson),
    expectedBytes: expectedHead == null
        ? null
        : utf8.encode(expectedHead.canonicalJson),
    validate: _validateHeadCandidate,
  );

  Future<AuthoringStoreOpenedResult> openPublished({
    AuthoringWorkingHead? expectedHead,
    String? expectedProjectJson,
  }) async {
    final exactDiskHead = await _readCanonicalHead(headFile);
    final opened = await store.open(
      root: root.path,
      verification: AuthoringAssetVerification.full,
      profile: profile,
    );
    _requireExactOpened(
      opened,
      expectedHead: expectedHead ?? exactDiskHead,
      expectedProjectJson: expectedProjectJson,
      context: 'published checkpoint',
    );
    if (opened.head.canonicalJson != exactDiskHead.canonicalJson) {
      throw const ManagedProjectVerificationException(
        'native open did not return the exact published head bytes',
      );
    }
    return opened;
  }

  Future<bool> _validateHeadCandidate(File candidate) async {
    try {
      final head = await _readCanonicalHead(candidate);
      final opened = await store.openHeadBytes(
        root: root.path,
        head: head,
        verification: AuthoringAssetVerification.full,
        profile: profile,
      );
      _requireExactOpened(
        opened,
        expectedHead: head,
        context: 'head candidate',
      );
      return true;
    } catch (_) {
      return false;
    }
  }
}

bool _prepareErrorRequiresReopen(String code) => const {
  'AUTHORING_STORE_HEAD_INVALID',
  'AUTHORING_STORE_HEAD_NONCANONICAL',
  'AUTHORING_STORE_HEAD_LIMIT',
  'AUTHORING_STORE_HEAD_MISSING',
  'AUTHORING_STORE_JSON_INVALID',
  'AUTHORING_STORE_JSON_NONCANONICAL',
  'AUTHORING_STORE_PATH_UNSAFE',
  'AUTHORING_STORE_ROOT_MISSING',
}.contains(code);

Future<AuthoringWorkingHead> _readCanonicalHead(File file) async {
  final type = await FileSystemEntity.type(file.path, followLinks: false);
  if (type != FileSystemEntityType.file) {
    throw ManagedProjectVerificationException(
      'managed project head is not a regular file: ${file.path}',
    );
  }
  final RandomAccessFile handle;
  try {
    handle = await file.open();
  } on FileSystemException {
    throw ManagedProjectVerificationException(
      'managed project head could not be opened: ${file.path}',
    );
  }
  final Uint8List bytes;
  try {
    final builder = BytesBuilder(copy: false);
    while (builder.length <= _maxManagedHeadBytes) {
      final remaining = _maxManagedHeadBytes + 1 - builder.length;
      final chunk = await handle.read(remaining < 8192 ? remaining : 8192);
      if (chunk.isEmpty) break;
      builder.add(chunk);
    }
    bytes = builder.takeBytes();
  } finally {
    await handle.close();
  }
  if (bytes.isEmpty || bytes.length > _maxManagedHeadBytes) {
    throw ManagedProjectVerificationException(
      'managed project head exceeds its size limit: ${file.path}',
    );
  }
  final String text;
  try {
    text = utf8.decode(bytes, allowMalformed: false);
  } on FormatException {
    throw ManagedProjectVerificationException(
      'managed project head is not valid UTF-8: ${file.path}',
    );
  }
  try {
    return AuthoringWorkingHead.fromCanonicalJson(text);
  } on FormatException {
    throw ManagedProjectVerificationException(
      'managed project head is not canonical: ${file.path}',
    );
  }
}

void _requireExactOpened(
  AuthoringStoreOpenedResult opened, {
  required AuthoringWorkingHead expectedHead,
  String? expectedProjectJson,
  required String context,
}) {
  if (opened.head.canonicalJson != expectedHead.canonicalJson) {
    throw ManagedProjectVerificationException(
      '$context returned a different head than requested',
    );
  }
  if (expectedProjectJson != null &&
      opened.projectJson != expectedProjectJson) {
    throw ManagedProjectVerificationException(
      '$context did not reproduce the exact captured project JSON',
    );
  }
}
