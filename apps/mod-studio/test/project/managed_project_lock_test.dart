import 'dart:convert';
import 'dart:io';

import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/project/managed_project_lock.dart';
import 'package:path/path.dart' as p;

void main() {
  late Directory root;

  setUp(() async {
    root = await Directory.systemTemp.createTemp('gore_managed_lock_');
  });

  tearDown(() async {
    if (await root.exists()) await root.delete(recursive: true);
  });

  test('acquire writes a bounded record and release permits reopen', () async {
    final opened = DateTime.utc(2026, 7, 12, 12, 34, 56);
    final first = await ManagedProjectSessionLock.acquire(
      root,
      ownerToken: '00000000000000000000000000000001',
      openedAtUtc: opened,
    );
    final lockFile = File(p.join(root.path, '.gore', 'session.lock'));
    expect(first.projectRoot, p.normalize(p.absolute(root.path)));
    expect(first.ownerToken, '00000000000000000000000000000001');

    await first.release();
    await first.release();
    final firstRecord = jsonDecode(await lockFile.readAsString()) as Map;
    expect(firstRecord['format'], 1);
    expect(firstRecord['owner_token'], first.ownerToken);
    expect(firstRecord['pid'], pid);
    expect(firstRecord['opened_at_utc'], '2026-07-12T12:34:56.000Z');
    expect(await lockFile.exists(), isTrue);

    final second = await ManagedProjectSessionLock.acquire(
      root,
      ownerToken: '00000000000000000000000000000002',
      openedAtUtc: opened.add(const Duration(minutes: 1)),
    );
    await second.release();
    final secondRecord = jsonDecode(await lockFile.readAsString()) as Map;
    expect(secondRecord['owner_token'], second.ownerToken);
    expect(secondRecord['opened_at_utc'], '2026-07-12T12:35:56.000Z');
  });

  test(
    'a second in-process owner is rejected without rewriting record',
    () async {
      final first = await ManagedProjectSessionLock.acquire(
        root,
        ownerToken: '10000000000000000000000000000001',
        openedAtUtc: DateTime.utc(2026),
      );
      final lockFile = File(p.join(root.path, '.gore', 'session.lock'));

      await expectLater(
        ManagedProjectSessionLock.acquire(
          root,
          ownerToken: '10000000000000000000000000000002',
        ),
        throwsA(isA<ManagedProjectAlreadyOpenException>()),
      );
      await first.release();
      final record = jsonDecode(await lockFile.readAsString()) as Map;
      expect(record['owner_token'], first.ownerToken);
    },
  );

  test('the Windows OS lock excludes a distinct file handle', () async {
    if (!Platform.isWindows) return;
    final owner = await ManagedProjectSessionLock.acquire(
      root,
      ownerToken: '11000000000000000000000000000001',
    );
    final contender = await File(
      owner.lockPath,
    ).open(mode: FileMode.writeOnlyAppend);
    try {
      await expectLater(
        contender.lock(FileLock.exclusive),
        throwsA(isA<FileSystemException>()),
      );
    } finally {
      await contender.close();
      await owner.release();
    }
  });

  test('invalid owner token releases the OS and in-process claims', () async {
    await expectLater(
      ManagedProjectSessionLock.acquire(root, ownerToken: 'not-a-token'),
      throwsArgumentError,
    );

    final recovered = await ManagedProjectSessionLock.acquire(
      root,
      ownerToken: '20000000000000000000000000000001',
    );
    await recovered.release();
  });

  test('non-directory control path is rejected and preserved', () async {
    final control = File(p.join(root.path, '.gore'));
    await control.writeAsString('do not replace');

    await expectLater(
      ManagedProjectSessionLock.acquire(root),
      throwsA(isA<ManagedProjectLockException>()),
    );
    expect(await control.readAsString(), 'do not replace');
  });

  test('missing or non-directory project root is rejected', () async {
    final missing = Directory(p.join(root.path, 'missing'));
    await expectLater(
      ManagedProjectSessionLock.acquire(missing),
      throwsA(isA<ManagedProjectLockException>()),
    );
    expect(await missing.exists(), isFalse);
    expect(await Directory(p.join(missing.path, '.gore')).exists(), isFalse);

    final fileRoot = File(p.join(root.path, 'file-root'));
    await fileRoot.writeAsString('preserve');
    await expectLater(
      ManagedProjectSessionLock.acquire(Directory(fileRoot.path)),
      throwsA(isA<ManagedProjectLockException>()),
    );
    expect(await fileRoot.readAsString(), 'preserve');
  });

  test('a linked directory prefix is rejected before lock artifacts', () async {
    final realParent = Directory(p.join(root.path, 'real-parent'));
    final realProject = Directory(p.join(realParent.path, 'project'));
    await realProject.create(recursive: true);
    final aliasPath = p.join(root.path, 'linked-prefix');

    if (Platform.isWindows) {
      final result = await Process.run('cmd.exe', [
        '/c',
        'mklink',
        '/J',
        aliasPath,
        realParent.path,
      ]);
      expect(
        result.exitCode,
        0,
        reason: 'could not create Windows junction: ${result.stderr}',
      );
    } else {
      await Link(aliasPath).create(realParent.path);
    }
    addTearDown(() async {
      final type = await FileSystemEntity.type(aliasPath, followLinks: false);
      if (type == FileSystemEntityType.link) {
        await Link(aliasPath).delete();
      } else if (type == FileSystemEntityType.directory) {
        await Directory(aliasPath).delete();
      }
    });

    expect(
      await FileSystemEntity.type(aliasPath, followLinks: false),
      FileSystemEntityType.link,
    );
    await expectLater(
      ManagedProjectSessionLock.acquire(
        Directory(p.join(aliasPath, 'project')),
      ),
      throwsA(isA<ManagedProjectLockException>()),
    );
    expect(
      await Directory(p.join(realProject.path, '.gore')).exists(),
      isFalse,
    );
  });
}
