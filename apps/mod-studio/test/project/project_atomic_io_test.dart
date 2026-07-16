import 'dart:async';
import 'dart:convert';
import 'dart:io';

import 'package:crypto/crypto.dart' as crypto;
import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/project/project_atomic_io.dart';

const _oldBytes = <int>[0x47, 1];
const _newBytes = <int>[0x47, 2];
const _newerBytes = <int>[0x47, 3];

Future<bool> _validGeneration(File file) async {
  final bytes = await file.readAsBytes();
  return bytes.length == 2 && bytes.first == 0x47;
}

bool _sameBytes(List<int> first, List<int> second) {
  if (first.length != second.length) return false;
  for (var index = 0; index < first.length; index++) {
    if (first[index] != second[index]) return false;
  }
  return true;
}

class _SimulatedCrash implements Exception {
  const _SimulatedCrash(this.phase);

  final AtomicSwapPhase phase;
}

void main() {
  late Directory directory;
  late File target;

  setUp(() async {
    directory = await Directory.systemTemp.createTemp('gore_atomic_io_');
    target = File('${directory.path}${Platform.pathSeparator}project.bin');
  });

  tearDown(() async {
    if (await directory.exists()) {
      await directory.delete(recursive: true);
    }
  });

  test(
    'fresh and replacement writes validate both sides and clean residue',
    () async {
      final validated = <String>[];
      final helper = AtomicByteReplacement(
        operationIdFactory: () => '00000000000000000000000000000001',
      );

      await helper.replace(
        target: target,
        bytes: _oldBytes,
        validate: (candidate) async {
          validated.add(candidate.path);
          return _validGeneration(candidate);
        },
      );
      expect(await target.readAsBytes(), _oldBytes);
      expect(validated, hasLength(2));

      final second = AtomicByteReplacement(
        operationIdFactory: () => '00000000000000000000000000000002',
      );
      await second.replace(
        target: target,
        bytes: _newBytes,
        validate: _validGeneration,
      );

      expect(await target.readAsBytes(), _newBytes);
      expect(await directory.list().map((entry) => entry.path).toList(), [
        target.path,
      ]);
    },
  );

  test('compare-and-replace accepts exact existing and absent bases', () async {
    final first = AtomicByteReplacement(
      operationIdFactory: () => '01000000000000000000000000000001',
    );
    await first.replaceIfUnchanged(
      target: target,
      bytes: _oldBytes,
      expectedBytes: null,
      validate: _validGeneration,
    );
    expect(await target.readAsBytes(), _oldBytes);

    final second = AtomicByteReplacement(
      operationIdFactory: () => '01000000000000000000000000000002',
    );
    await second.replaceIfUnchanged(
      target: target,
      bytes: _newBytes,
      expectedBytes: _oldBytes,
      validate: _validGeneration,
    );
    expect(await target.readAsBytes(), _newBytes);
    expect(await directory.list().map((entry) => entry.path).toList(), [
      target.path,
    ]);
  });

  test('compare-and-replace rejects drift without creating residue', () async {
    await target.writeAsBytes(_newBytes, flush: true);

    await expectLater(
      AtomicByteReplacement().replaceIfUnchanged(
        target: target,
        bytes: _newerBytes,
        expectedBytes: _oldBytes,
        validate: _validGeneration,
      ),
      throwsA(isA<AtomicSwapConflictException>()),
    );

    expect(await target.readAsBytes(), _newBytes);
    expect(await directory.list().map((entry) => entry.path).toList(), [
      target.path,
    ]);
  });

  test(
    'crash after journal commit but before temp creation keeps the target',
    () async {
      await target.writeAsBytes(_oldBytes);
      final helper = AtomicByteReplacement(
        operationIdFactory: () => '10000000000000000000000000000001',
        onPhase: (phase) {
          if (phase == AtomicSwapPhase.journalCommitted) {
            throw _SimulatedCrash(phase);
          }
        },
      );

      await expectLater(
        helper.replace(
          target: target,
          bytes: _newBytes,
          validate: _validGeneration,
        ),
        throwsA(isA<_SimulatedCrash>()),
      );

      expect(await target.readAsBytes(), _oldBytes);
      expect(
        await AtomicByteReplacement().repair(
          target: target,
          validate: _validGeneration,
        ),
        AtomicRepairOutcome.keptTarget,
      );
      expect(await target.readAsBytes(), _oldBytes);
    },
  );

  test('format-2 journal canonically binds exact old and new bytes', () async {
    await target.writeAsBytes(_oldBytes, flush: true);
    const operationId = '10100000000000000000000000000001';
    final helper = AtomicByteReplacement(
      operationIdFactory: () => operationId,
      onPhase: (phase) {
        if (phase == AtomicSwapPhase.journalCommitted) {
          throw _SimulatedCrash(phase);
        }
      },
    );

    await expectLater(
      helper.replace(
        target: target,
        bytes: _newBytes,
        validate: _validGeneration,
      ),
      throwsA(isA<_SimulatedCrash>()),
    );

    final journal = File(AtomicByteReplacement.journalPathFor(target));
    final text = await journal.readAsString();
    final envelope = (jsonDecode(text) as Map).cast<String, Object?>();
    final record = (envelope['record'] as Map).cast<String, Object?>();
    expect(envelope.keys, ['format', 'record', 'record_sha256']);
    expect(envelope['format'], AtomicByteReplacement.journalFormat);
    expect(record.keys, [
      'operation_id',
      'target_name',
      'temp_name',
      'backup_name',
      'old_generation',
      'new_generation',
    ]);
    expect(record['old_generation'], {
      'byte_len': _oldBytes.length,
      'sha256': crypto.sha256.convert(_oldBytes).toString(),
    });
    expect(record['new_generation'], {
      'byte_len': _newBytes.length,
      'sha256': crypto.sha256.convert(_newBytes).toString(),
    });
    expect(
      envelope['record_sha256'],
      crypto.sha256.convert(utf8.encode(jsonEncode(record))).toString(),
    );
    expect(text, '${jsonEncode(envelope)}\n');

    expect(
      await AtomicByteReplacement().repair(
        target: target,
        validate: _validGeneration,
      ),
      AtomicRepairOutcome.keptTarget,
    );
  });

  test('crash while staging the journal leaves no untracked content', () async {
    await target.writeAsBytes(_oldBytes);
    final helper = AtomicByteReplacement(
      operationIdFactory: () => '11000000000000000000000000000001',
      onPhase: (phase) {
        if (phase == AtomicSwapPhase.journalFlushed) {
          throw _SimulatedCrash(phase);
        }
      },
    );

    await expectLater(
      helper.replace(
        target: target,
        bytes: _newBytes,
        validate: _validGeneration,
      ),
      throwsA(isA<_SimulatedCrash>()),
    );
    expect(await target.readAsBytes(), _oldBytes);
    expect(
      await AtomicByteReplacement().repair(
        target: target,
        validate: _validGeneration,
      ),
      AtomicRepairOutcome.clean,
    );
    expect(await directory.list().map((entry) => entry.path).toList(), [
      target.path,
    ]);
  });

  test(
    'fresh-target journal-only crash repairs to the prior empty state',
    () async {
      final helper = AtomicByteReplacement(
        operationIdFactory: () => '12000000000000000000000000000001',
        onPhase: (phase) {
          if (phase == AtomicSwapPhase.journalCommitted) {
            throw _SimulatedCrash(phase);
          }
        },
      );

      await expectLater(
        helper.replace(
          target: target,
          bytes: _newBytes,
          validate: _validGeneration,
        ),
        throwsA(isA<_SimulatedCrash>()),
      );
      expect(
        await AtomicByteReplacement().repair(
          target: target,
          validate: _validGeneration,
        ),
        AtomicRepairOutcome.clean,
      );
      expect(await directory.list().isEmpty, isTrue);
    },
  );

  for (final phase in [
    AtomicSwapPhase.tempFlushed,
    AtomicSwapPhase.tempValidated,
    AtomicSwapPhase.targetBackedUp,
  ]) {
    test('repair promotes the intended bytes after crash at $phase', () async {
      await target.writeAsBytes(_oldBytes);
      final helper = AtomicByteReplacement(
        operationIdFactory: () => '20000000000000000000000000000001',
        onPhase: (seen) {
          if (seen == phase) throw _SimulatedCrash(seen);
        },
      );

      await expectLater(
        helper.replace(
          target: target,
          bytes: _newBytes,
          validate: _validGeneration,
        ),
        throwsA(isA<_SimulatedCrash>()),
      );

      expect(
        await AtomicByteReplacement().repair(
          target: target,
          validate: _validGeneration,
        ),
        AtomicRepairOutcome.promotedTemp,
      );
      expect(await target.readAsBytes(), _newBytes);
      expect(
        File(AtomicByteReplacement.journalPathFor(target)).existsSync(),
        isFalse,
      );
    });
  }

  for (final phase in [
    AtomicSwapPhase.tempPromoted,
    AtomicSwapPhase.targetValidated,
  ]) {
    test('repair keeps the promoted bytes after crash at $phase', () async {
      await target.writeAsBytes(_oldBytes);
      final helper = AtomicByteReplacement(
        operationIdFactory: () => '30000000000000000000000000000001',
        onPhase: (seen) {
          if (seen == phase) throw _SimulatedCrash(seen);
        },
      );

      await expectLater(
        helper.replace(
          target: target,
          bytes: _newBytes,
          validate: _validGeneration,
        ),
        throwsA(isA<_SimulatedCrash>()),
      );

      expect(
        await AtomicByteReplacement().repair(
          target: target,
          validate: _validGeneration,
        ),
        AtomicRepairOutcome.keptTarget,
      );
      expect(await target.readAsBytes(), _newBytes);
      expect(await directory.list().map((entry) => entry.path).toList(), [
        target.path,
      ]);
    });
  }

  test(
    'repair rejects a semantically valid target outside the exact binding',
    () async {
      await target.writeAsBytes(_oldBytes, flush: true);
      const operationId = '31000000000000000000000000000001';
      final helper = AtomicByteReplacement(
        operationIdFactory: () => operationId,
        onPhase: (phase) {
          if (phase == AtomicSwapPhase.journalCommitted) {
            throw _SimulatedCrash(phase);
          }
        },
      );
      await expectLater(
        helper.replace(
          target: target,
          bytes: _newBytes,
          validate: _validGeneration,
        ),
        throwsA(isA<_SimulatedCrash>()),
      );
      await target.writeAsBytes(_newerBytes, flush: true);
      final journal = File(AtomicByteReplacement.journalPathFor(target));

      await expectLater(
        AtomicByteReplacement().repair(
          target: target,
          validate: _validGeneration,
        ),
        throwsA(isA<AtomicSwapRecoveryException>()),
      );

      expect(await target.readAsBytes(), _newerBytes);
      expect(await journal.exists(), isTrue);
    },
  );

  test(
    'repair rejects a semantically valid temporary outside the exact binding',
    () async {
      await target.writeAsBytes(_oldBytes, flush: true);
      const operationId = '32000000000000000000000000000001';
      final helper = AtomicByteReplacement(
        operationIdFactory: () => operationId,
        onPhase: (phase) {
          if (phase == AtomicSwapPhase.tempFlushed) {
            throw _SimulatedCrash(phase);
          }
        },
      );
      await expectLater(
        helper.replace(
          target: target,
          bytes: _newBytes,
          validate: _validGeneration,
        ),
        throwsA(isA<_SimulatedCrash>()),
      );
      final temp = File('${target.path}.gore-swap-$operationId.tmp');
      await temp.writeAsBytes(_newerBytes, flush: true);
      final journal = File(AtomicByteReplacement.journalPathFor(target));

      await expectLater(
        AtomicByteReplacement().repair(
          target: target,
          validate: _validGeneration,
        ),
        throwsA(isA<AtomicSwapRecoveryException>()),
      );

      expect(await target.readAsBytes(), _oldBytes);
      expect(await temp.readAsBytes(), _newerBytes);
      expect(await journal.exists(), isTrue);
    },
  );

  test(
    'repair rejects a semantically valid backup outside the exact binding',
    () async {
      await target.writeAsBytes(_oldBytes, flush: true);
      const operationId = '33000000000000000000000000000001';
      final helper = AtomicByteReplacement(
        operationIdFactory: () => operationId,
        onPhase: (phase) {
          if (phase == AtomicSwapPhase.targetBackedUp) {
            throw _SimulatedCrash(phase);
          }
        },
      );
      await expectLater(
        helper.replace(
          target: target,
          bytes: _newBytes,
          validate: _validGeneration,
        ),
        throwsA(isA<_SimulatedCrash>()),
      );
      final temp = File('${target.path}.gore-swap-$operationId.tmp');
      final backup = File('${target.path}.gore-swap-$operationId.bak');
      await backup.writeAsBytes(_newerBytes, flush: true);
      final journal = File(AtomicByteReplacement.journalPathFor(target));

      await expectLater(
        AtomicByteReplacement().repair(
          target: target,
          validate: _validGeneration,
        ),
        throwsA(isA<AtomicSwapRecoveryException>()),
      );

      expect(await target.exists(), isFalse);
      expect(await temp.readAsBytes(), _newBytes);
      expect(await backup.readAsBytes(), _newerBytes);
      expect(await journal.exists(), isTrue);
    },
  );

  test(
    'failed post-promotion validation restores the validated backup',
    () async {
      await target.writeAsBytes(_oldBytes);
      final helper = AtomicByteReplacement(
        operationIdFactory: () => '40000000000000000000000000000001',
      );

      await expectLater(
        helper.replace(
          target: target,
          bytes: _newBytes,
          validate: (candidate) async {
            final bytes = await candidate.readAsBytes();
            if (_sameBytes(bytes, _oldBytes)) return true;
            return candidate.path.endsWith('.tmp') &&
                _sameBytes(bytes, _newBytes);
          },
        ),
        throwsA(isA<AtomicSwapException>()),
      );

      expect(
        await AtomicByteReplacement().repair(
          target: target,
          validate: (candidate) async =>
              _sameBytes(await candidate.readAsBytes(), _oldBytes),
        ),
        AtomicRepairOutcome.restoredBackup,
      );
      expect(await target.readAsBytes(), _oldBytes);
      final quarantines = await directory
          .list()
          .where((entry) => entry.path.contains('.invalid-'))
          .toList();
      expect(quarantines, hasLength(1));
      expect(await File(quarantines.single.path).readAsBytes(), _newBytes);
    },
  );

  test(
    'no valid generation preserves every owned artifact and journal',
    () async {
      await target.writeAsBytes(_oldBytes);
      final helper = AtomicByteReplacement(
        operationIdFactory: () => '50000000000000000000000000000001',
        onPhase: (phase) {
          if (phase == AtomicSwapPhase.targetBackedUp) {
            throw _SimulatedCrash(phase);
          }
        },
      );
      await expectLater(
        helper.replace(
          target: target,
          bytes: _newBytes,
          validate: _validGeneration,
        ),
        throwsA(isA<_SimulatedCrash>()),
      );

      final before = await directory
          .list()
          .where((entry) => entry is File)
          .toList();
      for (final entity in before) {
        final file = File(entity.path);
        if (!file.path.endsWith('.json')) {
          await file.writeAsBytes(const [0], flush: true);
        }
      }
      final pathsBefore =
          (await directory.list().map((entry) => entry.path).toList())..sort();

      await expectLater(
        AtomicByteReplacement().repair(
          target: target,
          validate: _validGeneration,
        ),
        throwsA(isA<AtomicSwapRecoveryException>()),
      );
      final pathsAfter =
          (await directory.list().map((entry) => entry.path).toList())..sort();
      expect(pathsAfter, pathsBefore);
    },
  );

  test(
    'journal path traversal is rejected before touching any named file',
    () async {
      final victim = File(
        '${directory.parent.path}${Platform.pathSeparator}victim.bin',
      );
      addTearDown(() async {
        if (await victim.exists()) await victim.delete();
      });
      await victim.writeAsBytes(_oldBytes);
      await target.writeAsBytes(_oldBytes);
      final journal = File(AtomicByteReplacement.journalPathFor(target));
      await journal.writeAsString(
        jsonEncode({
          'format': 1,
          'operation_id': '60000000000000000000000000000001',
          'target_name': 'project.bin',
          'temp_name': '..${Platform.pathSeparator}victim.bin',
          'backup_name':
              'project.bin.gore-swap-60000000000000000000000000000001.bak',
          'target_existed': true,
        }),
        flush: true,
      );

      await expectLater(
        AtomicByteReplacement().repair(
          target: target,
          validate: _validGeneration,
        ),
        throwsA(isA<AtomicSwapRecoveryException>()),
      );
      expect(await victim.readAsBytes(), _oldBytes);
      expect(await target.readAsBytes(), _oldBytes);
      expect(await journal.exists(), isTrue);
    },
  );

  test(
    'legacy journal preserves two different valid generations as ambiguous',
    () async {
      await target.writeAsBytes(_oldBytes, flush: true);
      const operationId = '61000000000000000000000000000001';
      final temp = File('${target.path}.gore-swap-$operationId.tmp');
      final journal = File(AtomicByteReplacement.journalPathFor(target));
      await temp.writeAsBytes(_newBytes, flush: true);
      await journal.writeAsString(
        jsonEncode({
          'format': AtomicByteReplacement.legacyJournalFormat,
          'operation_id': operationId,
          'target_name': 'project.bin',
          'temp_name': 'project.bin.gore-swap-$operationId.tmp',
          'backup_name': 'project.bin.gore-swap-$operationId.bak',
          'target_existed': true,
        }),
        flush: true,
      );

      await expectLater(
        AtomicByteReplacement().repair(
          target: target,
          validate: _validGeneration,
        ),
        throwsA(isA<AtomicSwapRecoveryException>()),
      );
      expect(await target.readAsBytes(), _oldBytes);
      expect(await temp.readAsBytes(), _newBytes);
      expect(await journal.exists(), isTrue);

      // A legacy journal has no byte binding. Once only one valid generation
      // remains, conservative backward recovery can retain it.
      await temp.delete();
      expect(
        await AtomicByteReplacement().repair(
          target: target,
          validate: _validGeneration,
        ),
        AtomicRepairOutcome.keptTarget,
      );
      expect(await journal.exists(), isFalse);
    },
  );

  test('format-2 checksum and canonical JSON corruption fail closed', () async {
    await target.writeAsBytes(_oldBytes, flush: true);
    const operationId = '62000000000000000000000000000001';
    final helper = AtomicByteReplacement(
      operationIdFactory: () => operationId,
      onPhase: (phase) {
        if (phase == AtomicSwapPhase.journalCommitted) {
          throw _SimulatedCrash(phase);
        }
      },
    );
    await expectLater(
      helper.replace(
        target: target,
        bytes: _newBytes,
        validate: _validGeneration,
      ),
      throwsA(isA<_SimulatedCrash>()),
    );
    final journal = File(AtomicByteReplacement.journalPathFor(target));
    final canonical = await journal.readAsString();
    final envelope = (jsonDecode(canonical) as Map).cast<String, Object?>();
    envelope['record_sha256'] = List.filled(64, '0').join();
    await journal.writeAsString('${jsonEncode(envelope)}\n', flush: true);

    await expectLater(
      AtomicByteReplacement().repair(
        target: target,
        validate: _validGeneration,
      ),
      throwsA(isA<AtomicSwapRecoveryException>()),
    );
    expect(await target.readAsBytes(), _oldBytes);
    expect(await journal.exists(), isTrue);

    final originalEnvelope = jsonDecode(canonical);
    await journal.writeAsString(
      const JsonEncoder.withIndent('  ').convert(originalEnvelope),
      flush: true,
    );
    await expectLater(
      AtomicByteReplacement().repair(
        target: target,
        validate: _validGeneration,
      ),
      throwsA(isA<AtomicSwapRecoveryException>()),
    );
    expect(await journal.exists(), isTrue);
  });

  test(
    'oversized or unknown-version journal is preserved and rejected',
    () async {
      await target.writeAsBytes(_oldBytes);
      final journal = File(AtomicByteReplacement.journalPathFor(target));
      await journal.writeAsBytes(
        List<int>.filled(AtomicByteReplacement.maxJournalBytes + 1, 0x20),
        flush: true,
      );

      await expectLater(
        AtomicByteReplacement().repair(
          target: target,
          validate: _validGeneration,
        ),
        throwsA(isA<AtomicSwapRecoveryException>()),
      );
      expect(await journal.length(), AtomicByteReplacement.maxJournalBytes + 1);

      await journal.writeAsString(
        jsonEncode({
          'format': 3,
          'operation_id': '70000000000000000000000000000001',
          'target_name': 'project.bin',
          'temp_name':
              'project.bin.gore-swap-70000000000000000000000000000001.tmp',
          'backup_name':
              'project.bin.gore-swap-70000000000000000000000000000001.bak',
          'target_existed': true,
        }),
        flush: true,
      );
      await expectLater(
        AtomicByteReplacement().repair(
          target: target,
          validate: _validGeneration,
        ),
        throwsA(isA<AtomicSwapRecoveryException>()),
      );
      expect(await target.readAsBytes(), _oldBytes);
      expect(await journal.exists(), isTrue);
    },
  );

  test(
    'directory scan cap fails before deleting pending journal evidence',
    () async {
      await target.writeAsBytes(_oldBytes);
      final pending = File(
        '${target.path}.gore-swap-71000000000000000000000000000001.journal.tmp',
      );
      await pending.writeAsString('journal evidence', flush: true);
      final unrelated = File(
        '${directory.path}${Platform.pathSeparator}unrelated.bin',
      );
      await unrelated.writeAsBytes(const [9], flush: true);

      await expectLater(
        AtomicByteReplacement(
          directoryScanLimit: 2,
        ).repair(target: target, validate: _validGeneration),
        throwsA(isA<AtomicSwapRecoveryException>()),
      );

      expect(await target.readAsBytes(), _oldBytes);
      expect(await pending.readAsString(), 'journal evidence');
      expect(await unrelated.readAsBytes(), const [9]);
    },
  );

  test('non-regular target is rejected without changing it', () async {
    await Directory(target.path).create();

    await expectLater(
      AtomicByteReplacement().repair(
        target: target,
        validate: _validGeneration,
      ),
      throwsA(isA<AtomicSwapRecoveryException>()),
    );

    expect(await Directory(target.path).exists(), isTrue);
    expect(await Directory(target.path).list().isEmpty, isTrue);
  });

  test('same-target writes serialize across helper instances', () async {
    await target.writeAsBytes(_oldBytes);
    final firstBlocked = Completer<void>();
    final releaseFirst = Completer<void>();
    var secondStarted = false;
    final first = AtomicByteReplacement(
      operationIdFactory: () => '80000000000000000000000000000001',
      onPhase: (phase) async {
        if (phase == AtomicSwapPhase.journalCommitted &&
            !firstBlocked.isCompleted) {
          firstBlocked.complete();
          await releaseFirst.future;
        }
      },
    );
    final second = AtomicByteReplacement(
      operationIdFactory: () => '80000000000000000000000000000002',
      onPhase: (phase) {
        if (phase == AtomicSwapPhase.tempFlushed) secondStarted = true;
      },
    );

    final firstWrite = first.replace(
      target: target,
      bytes: _newBytes,
      validate: _validGeneration,
    );
    await firstBlocked.future;
    final secondWrite = second.replace(
      target: target,
      bytes: _newerBytes,
      validate: _validGeneration,
    );
    await Future<void>.delayed(const Duration(milliseconds: 25));
    expect(secondStarted, isFalse);

    releaseFirst.complete();
    await Future.wait([firstWrite, secondWrite]);
    expect(secondStarted, isTrue);
    expect(await target.readAsBytes(), _newerBytes);
  });

  test(
    'queued compare-and-replace checks its base inside the target lane',
    () async {
      await target.writeAsBytes(_oldBytes);
      final firstBlocked = Completer<void>();
      final releaseFirst = Completer<void>();
      final first = AtomicByteReplacement(
        operationIdFactory: () => '81000000000000000000000000000001',
        onPhase: (phase) async {
          if (phase == AtomicSwapPhase.journalCommitted &&
              !firstBlocked.isCompleted) {
            firstBlocked.complete();
            await releaseFirst.future;
          }
        },
      );

      final firstWrite = first.replace(
        target: target,
        bytes: _newBytes,
        validate: _validGeneration,
      );
      await firstBlocked.future;
      final staleWrite = AtomicByteReplacement().replaceIfUnchanged(
        target: target,
        bytes: _newerBytes,
        expectedBytes: _oldBytes,
        validate: _validGeneration,
      );

      releaseFirst.complete();
      await firstWrite;
      await expectLater(
        staleWrite,
        throwsA(isA<AtomicSwapConflictException>()),
      );
      expect(await target.readAsBytes(), _newBytes);
    },
  );
}
