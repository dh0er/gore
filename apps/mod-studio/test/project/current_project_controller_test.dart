import 'dart:async';
import 'dart:convert';
import 'dart:io';

import 'package:crypto/crypto.dart' as crypto;
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/core/mod_ffi.dart';
import 'package:gore_mod/project/current_project_controller.dart';
import 'package:gore_mod/project/managed_project_session.dart';
import 'package:gore_mod/project/project_atomic_io.dart';
import 'package:gore_mod/project/project_controller.dart';
import 'package:gore_mod/project/revision3_content_index.dart';
import 'package:gore_mod/project/revision3_dataasset_authoring.dart';
import 'package:gore_mod/project/revision3_dialog_localization_authoring.dart';
import 'package:gore_mod/project/revision3_dialog_line_authoring.dart';
import 'package:gore_mod/project/revision3_dialog_voice_slot_removal_authoring.dart';
import 'package:gore_mod/project/revision3_npc_authoring.dart';
import 'package:gore_mod/project/revision3_npc_profile_edit_authoring.dart';
import 'package:gore_mod/project/revision3_quest_authoring.dart';
import 'package:gore_mod/project/revision3_quest_context_authoring.dart';
import 'package:gore_mod/project/revision3_quest_outline_authoring.dart';
import 'package:gore_mod/project/revision3_quest_transitions_authoring.dart';
import 'package:gore_mod/project/revision3_project_history.dart';
import 'package:gore_mod/project/revision3_voice_authoring.dart';
import 'package:gore_mod/project/revision3_voice_take_media_qa_service.dart';
import 'package:gore_mod/project/revision3_voice_take_preview_authoring.dart';
import 'package:gore_mod/project/revision3_voice_take_removal_authoring.dart';
import 'package:gore_mod/project/revision3_voice_take_selection_authoring.dart';
import 'package:gore_mod/project/revision3_voice_take_status_authoring.dart';
import 'package:path/path.dart' as p;

import '../support/revision3_dataasset_fixture.dart';
import '../support/revision3_npc_fixture.dart';
import '../support/revision3_npc_profile_edit_fixture.dart';
import '../support/revision3_voice_content_fixture.dart';
import '../support/revision3_voice_fixture.dart';
import '../support/revision3_voice_preview_fixture.dart';
import '../support/revision3_quest_outline_fixture.dart';
import '../dataasset/dataasset_test_fixtures.dart';

void main() {
  test(
    'managed create adopts only a fully opened candidate and closes legacy',
    () async {
      final legacy = _FakeLegacyLease(path: 'before-create.goremod');
      final managed = _FakeManagedLease(
        root: Directory('created-managed'),
        projectIdValue: 'abababababababababababababababab',
        projectRevision: 0,
        head: _head(41),
      );
      ManagedRevision3ProjectCreateRequest? received;
      final coordinator = CurrentProjectCoordinator(
        initialLegacy: legacy,
        createManagedRevision3: (request) async {
          received = request;
          return managed;
        },
        openManagedRevision3: (_) async => throw UnimplementedError(),
      );
      addTearDown(() async {
        await coordinator.shutdown();
        coordinator.dispose();
      });
      final request = ManagedRevision3ProjectCreateRequest(
        root: Directory('chosen-empty-root'),
        gameRoot: r'C:\Games\Gothic 1 Remake',
        name: 'My first managed mod',
        version: '0.1.0',
        author: 'Author',
        authoringLocales: const <String>['de', 'en'],
      );

      final created = await coordinator.createManagedRevision3(request);

      expect(received, same(request));
      expect(created.root.path, managed.root.path);
      expect(created.projectId, managed.projectId);
      expect(created.projectRevision, 0);
      expect(legacy.closeCalls, 1);
      expect(managed.closeCalls, 0);
    },
  );

  test(
    'failed managed create preserves current lease and closes bad candidate',
    () async {
      final legacy = _FakeLegacyLease(path: 'preserved-create.goremod');
      final candidate = _FakeManagedLease(
        root: Directory('bad-created-managed'),
        projectIdValue: 'cdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcd',
        projectRevision: 0,
        head: _head(42),
        projectIdError: StateError('created project failed full reopen'),
      );
      final coordinator = CurrentProjectCoordinator(
        initialLegacy: legacy,
        createManagedRevision3: (_) async => candidate,
        openManagedRevision3: (_) async => throw UnimplementedError(),
      );
      addTearDown(() async {
        await coordinator.shutdown();
        coordinator.dispose();
      });
      final before = coordinator.state;

      await expectLater(
        coordinator.createManagedRevision3(
          ManagedRevision3ProjectCreateRequest(
            root: Directory('chosen-empty-root'),
            gameRoot: r'C:\Games\Gothic 1 Remake',
            name: 'Rejected candidate',
            version: '0.1.0',
            author: '',
            authoringLocales: const <String>['en'],
          ),
        ),
        throwsA(isA<StateError>()),
      );

      expect(coordinator.state, same(before));
      expect(legacy.closeCalls, 0);
      expect(candidate.closeCalls, 1);
    },
  );

  test(
    'failed managed open preserves the exact legacy current project',
    () async {
      final legacy = _FakeLegacyLease(
        path: 'current.goremod',
        hasUnsavedChanges: true,
      );
      final expectedError = StateError('candidate could not be opened');
      final coordinator = CurrentProjectCoordinator(
        initialLegacy: legacy,
        openManagedRevision3: (_) async => throw expectedError,
      );
      addTearDown(() async {
        await coordinator.shutdown();
        coordinator.dispose();
      });
      final before = coordinator.state;

      await expectLater(
        coordinator.openManagedRevision3(Directory('candidate')),
        throwsA(same(expectedError)),
      );

      expect(coordinator.state, same(before));
      expect(coordinator.state, isA<LegacyCurrentProjectState>());
      expect(legacy.closeCalls, 0);
      expect(legacy.saveCalls, 0);
    },
  );

  test(
    'managed adoption closes legacy and save verifies without a managed write',
    () async {
      final legacy = _FakeLegacyLease(path: 'old.goremod');
      final managed = _FakeManagedLease(
        root: Directory('managed'),
        projectIdValue: '01010101010101010101010101010101',
        projectRevision: 7,
        head: _head(7),
      );
      final coordinator = CurrentProjectCoordinator(
        initialLegacy: legacy,
        openManagedRevision3: (_) async => managed,
      );
      addTearDown(() async {
        await coordinator.shutdown();
        coordinator.dispose();
      });

      final opened = await coordinator.openManagedRevision3(
        Directory('caller-path'),
      );
      expect(opened.root.path, managed.root.path);
      expect(opened.projectId, managed.projectId);
      expect(opened.projectRevision, 7);
      expect(opened.head.canonicalJson, managed.head.canonicalJson);
      expect(legacy.closeCalls, 1);
      expect(managed.verifyCalls, 0);

      final saved = await coordinator.saveCurrent();
      expect(saved, isA<ManagedRevision3CurrentProjectState>());
      expect(managed.verifyCalls, 1);
      expect(managed.closeCalls, 0);

      final verified = await coordinator.verifyCurrent();
      expect(verified.projectRevision, 7);
      expect(managed.verifyCalls, 2);
    },
  );

  test('save and managed open share one invocation-ordered lane', () async {
    final events = <String>[];
    final saveEntered = Completer<void>();
    final releaseSave = Completer<void>();
    final legacy = _FakeLegacyLease(
      path: 'ordered.goremod',
      onSave: () async {
        events.add('save-enter');
        saveEntered.complete();
        await releaseSave.future;
        events.add('save-exit');
      },
      onClose: () => events.add('legacy-close'),
    );
    final managed = _FakeManagedLease(
      root: Directory('ordered-managed'),
      projectIdValue: '02020202020202020202020202020202',
      projectRevision: 2,
      head: _head(2),
    );
    var openCalls = 0;
    final coordinator = CurrentProjectCoordinator(
      initialLegacy: legacy,
      openManagedRevision3: (_) async {
        openCalls++;
        events.add('managed-open');
        return managed;
      },
    );
    addTearDown(() async {
      await coordinator.shutdown();
      coordinator.dispose();
    });

    final save = coordinator.saveCurrent();
    await saveEntered.future;
    final open = coordinator.openManagedRevision3(Directory('managed'));
    await Future<void>.delayed(Duration.zero);
    expect(openCalls, 0);

    releaseSave.complete();
    await save;
    await open;
    expect(events, <String>[
      'save-enter',
      'save-exit',
      'managed-open',
      'legacy-close',
    ]);
  });

  test(
    'invalid managed candidate snapshot is closed and legacy remains current',
    () async {
      final legacy = _FakeLegacyLease(path: 'preserved.goremod');
      final candidate = _FakeManagedLease(
        root: Directory('invalid'),
        projectIdValue: '03030303030303030303030303030303',
        projectRevision: 3,
        head: _head(3),
        projectIdError: StateError('invalid candidate identity'),
      );
      final coordinator = CurrentProjectCoordinator(
        initialLegacy: legacy,
        openManagedRevision3: (_) async => candidate,
      );
      addTearDown(() async {
        await coordinator.shutdown();
        coordinator.dispose();
      });

      await expectLater(
        coordinator.openManagedRevision3(Directory('invalid')),
        throwsA(isA<StateError>()),
      );

      expect(coordinator.state, isA<LegacyCurrentProjectState>());
      expect(legacy.closeCalls, 0);
      expect(candidate.closeCalls, 1);
    },
  );

  test('managed verification failure refreshes visible reopen state', () async {
    final managed = _FakeManagedLease(
      root: Directory('poisoned'),
      projectIdValue: '04040404040404040404040404040404',
      projectRevision: 4,
      head: _head(4),
      onVerify: (lease) {
        lease.requiresReopenValue = true;
        throw StateError('full reopen failed');
      },
    );
    final coordinator = CurrentProjectCoordinator(
      openManagedRevision3: (_) async => managed,
    );
    addTearDown(() async {
      await coordinator.shutdown();
      coordinator.dispose();
    });
    await coordinator.openManagedRevision3(Directory('poisoned'));

    await expectLater(coordinator.saveCurrent(), throwsA(isA<StateError>()));

    final state = coordinator.state as ManagedRevision3CurrentProjectState;
    expect(state.requiresReopen, isTrue);
    expect(managed.verifyCalls, 1);

    await expectLater(
      coordinator.saveCurrent(),
      throwsA(isA<CurrentProjectOperationUnsupportedException>()),
    );
    await expectLater(
      coordinator.verifyCurrent(),
      throwsA(isA<CurrentProjectOperationUnsupportedException>()),
    );
    expect(managed.verifyCalls, 1);
  });

  test(
    'managed recovery keeps an unchanged exact durable checkpoint',
    () async {
      const projectId = '14141414141414141414141414141414';
      final projectJson = _recoveryProjectJson(
        projectId: projectId,
        revision: 4,
      );
      final head = _recoveryHead(projectJson);
      late _FakeRecoveryManagedLease managed;
      managed = _FakeRecoveryManagedLease(
        root: Directory('recover-unchanged'),
        projectIdValue: projectId,
        projectRevision: 4,
        head: head,
        canonicalProjectJson: projectJson,
        onRecovery: (lease) {
          lease.requiresReopenValue = false;
          return ManagedRevision3RecoveryCheckpoint(
            previousHead: head,
            recoveredHead: head,
            projectId: projectId,
            previousProjectRevision: 4,
            recoveredProjectRevision: 4,
            repairOutcome: AtomicRepairOutcome.restoredBackup,
            canonicalProjectJson: projectJson,
          );
        },
      )..requiresReopenValue = true;
      final coordinator = CurrentProjectCoordinator(
        openManagedRevision3: (_) async => managed,
      );
      addTearDown(() async {
        await coordinator.shutdown();
        coordinator.dispose();
      });
      final visible = await coordinator.openManagedRevision3(managed.root);

      final recovered = await coordinator.recoverCurrentRevision3(
        expectedRoot: visible.root.path,
        expectedProjectId: visible.projectId,
        expectedProjectRevision: visible.projectRevision,
        expectedHead: visible.head,
      );

      expect(recovered.advanced, isFalse);
      expect(recovered.repairOutcome, AtomicRepairOutcome.restoredBackup);
      expect(managed.recoveryCalls, 1);
      final state = coordinator.state as ManagedRevision3CurrentProjectState;
      expect(state.projectRevision, 4);
      expect(state.head.canonicalJson, head.canonicalJson);
      expect(state.requiresReopen, isFalse);
    },
  );

  test('managed recovery publishes an exact one-revision advance', () async {
    const projectId = '15151515151515151515151515151515';
    final previousJson = _recoveryProjectJson(
      projectId: projectId,
      revision: 7,
    );
    final previousHead = _recoveryHead(previousJson);
    final recoveredJson = _recoveryProjectJson(
      projectId: projectId,
      revision: 8,
    );
    final recoveredHead = _recoveryHead(recoveredJson);
    late _FakeRecoveryManagedLease managed;
    managed = _FakeRecoveryManagedLease(
      root: Directory('recover-advanced'),
      projectIdValue: projectId,
      projectRevision: 7,
      head: previousHead,
      canonicalProjectJson: previousJson,
      onRecovery: (lease) {
        lease
          ..projectRevision = 8
          ..head = recoveredHead
          ..canonicalProjectJson = recoveredJson
          ..requiresReopenValue = false;
        return ManagedRevision3RecoveryCheckpoint(
          previousHead: previousHead,
          recoveredHead: recoveredHead,
          projectId: projectId,
          previousProjectRevision: 7,
          recoveredProjectRevision: 8,
          repairOutcome: AtomicRepairOutcome.promotedTemp,
          canonicalProjectJson: recoveredJson,
        );
      },
    )..requiresReopenValue = true;
    final coordinator = CurrentProjectCoordinator(
      openManagedRevision3: (_) async => managed,
    );
    addTearDown(() async {
      await coordinator.shutdown();
      coordinator.dispose();
    });
    final visible = await coordinator.openManagedRevision3(managed.root);

    final recovered = await coordinator.recoverCurrentRevision3(
      expectedRoot: visible.root.path,
      expectedProjectId: visible.projectId,
      expectedProjectRevision: visible.projectRevision,
      expectedHead: visible.head,
    );

    expect(recovered.advanced, isTrue);
    expect(managed.recoveryCalls, 1);
    final state = coordinator.state as ManagedRevision3CurrentProjectState;
    expect(state.projectRevision, 8);
    expect(state.head.canonicalJson, recoveredHead.canonicalJson);
    expect(state.requiresReopen, isFalse);
  });

  test(
    'forged format schema or target drift fails closed after a sealed advance',
    () async {
      const projectId = '21212121212121212121212121212121';
      const stableTarget = <String, Object?>{
        'executable': <String, Object?>{
          'byte_len': 171698176,
          'sha256':
              'f406f969d3e73b6e58ea6e7aa10df7380318d97e7974d3be6e5a01183a4524f5',
        },
        'game_generation': <String, Object?>{
          'edition': 'remake',
          'catalog_seal': 'a',
        },
      };
      final scenarios =
          <
            ({
              String name,
              int format,
              int schemaRevision,
              Map<String, Object?> target,
            })
          >[
            (
              name: 'format',
              format: 1,
              schemaRevision: 3,
              target: stableTarget,
            ),
            (
              name: 'schema',
              format: 2,
              schemaRevision: 4,
              target: stableTarget,
            ),
            (
              name: 'target',
              format: 2,
              schemaRevision: 3,
              target: const <String, Object?>{
                'executable': <String, Object?>{
                  'byte_len': 171698176,
                  'sha256':
                      'f406f969d3e73b6e58ea6e7aa10df7380318d97e7974d3be6e5a01183a4524f5',
                },
                'game_generation': <String, Object?>{
                  'edition': 'remake',
                  'catalog_seal': 'b',
                },
              },
            ),
          ];

      for (final scenario in scenarios) {
        final previousJson = _recoveryProjectJson(
          projectId: projectId,
          revision: 12,
          target: stableTarget,
        );
        final previousHead = _recoveryHead(previousJson);
        final forgedJson = _recoveryProjectJson(
          projectId: projectId,
          revision: 13,
          format: scenario.format,
          schemaRevision: scenario.schemaRevision,
          target: scenario.target,
        );
        final forgedHead = _recoveryHead(forgedJson);
        late _FakeRecoveryManagedLease managed;
        managed = _FakeRecoveryManagedLease(
          root: Directory('recover-forged-${scenario.name}'),
          projectIdValue: projectId,
          projectRevision: 12,
          head: previousHead,
          canonicalProjectJson: previousJson,
          onRecovery: (lease) {
            lease
              ..projectRevision = 13
              ..head = forgedHead
              ..canonicalProjectJson = forgedJson
              ..requiresReopenValue = false;
            return ManagedRevision3RecoveryCheckpoint(
              previousHead: previousHead,
              recoveredHead: forgedHead,
              projectId: projectId,
              previousProjectRevision: 12,
              recoveredProjectRevision: 13,
              repairOutcome: AtomicRepairOutcome.keptTarget,
              canonicalProjectJson: forgedJson,
            );
          },
        )..requiresReopenValue = true;
        final coordinator = CurrentProjectCoordinator(
          openManagedRevision3: (_) async => managed,
        );
        try {
          final visible = await coordinator.openManagedRevision3(managed.root);

          await expectLater(
            coordinator.recoverCurrentRevision3(
              expectedRoot: visible.root.path,
              expectedProjectId: visible.projectId,
              expectedProjectRevision: visible.projectRevision,
              expectedHead: visible.head,
            ),
            throwsA(isA<Revision3RecoveryFailedException>()),
            reason: scenario.name,
          );

          expect(managed.recoveryCalls, 1, reason: scenario.name);
          expect(managed.recoveryRelatchCalls, 1, reason: scenario.name);
          expect(
            (coordinator.state as ManagedRevision3CurrentProjectState)
                .requiresReopen,
            isTrue,
            reason: scenario.name,
          );
        } finally {
          await coordinator.shutdown();
          coordinator.dispose();
        }
      }
    },
  );

  test(
    'stale managed recovery tuples never reach the recovery lease',
    () async {
      const projectId = '16161616161616161616161616161616';
      final projectJson = _recoveryProjectJson(
        projectId: projectId,
        revision: 3,
      );
      final head = _recoveryHead(projectJson);
      final managed = _FakeRecoveryManagedLease(
        root: Directory('recover-stale'),
        projectIdValue: projectId,
        projectRevision: 3,
        head: head,
        canonicalProjectJson: projectJson,
        onRecovery: (_) => throw StateError('must not recover a stale tuple'),
      )..requiresReopenValue = true;
      final coordinator = CurrentProjectCoordinator(
        openManagedRevision3: (_) async => managed,
      );
      addTearDown(() async {
        await coordinator.shutdown();
        coordinator.dispose();
      });
      final visible = await coordinator.openManagedRevision3(managed.root);
      final attempts = <Future<ManagedRevision3RecoveryCheckpoint> Function()>[
        () => coordinator.recoverCurrentRevision3(
          expectedRoot: '${visible.root.path}-other',
          expectedProjectId: visible.projectId,
          expectedProjectRevision: visible.projectRevision,
          expectedHead: visible.head,
        ),
        () => coordinator.recoverCurrentRevision3(
          expectedRoot: visible.root.path,
          expectedProjectId: 'ffffffffffffffffffffffffffffffff',
          expectedProjectRevision: visible.projectRevision,
          expectedHead: visible.head,
        ),
        () => coordinator.recoverCurrentRevision3(
          expectedRoot: visible.root.path,
          expectedProjectId: visible.projectId,
          expectedProjectRevision: visible.projectRevision + 1,
          expectedHead: visible.head,
        ),
        () => coordinator.recoverCurrentRevision3(
          expectedRoot: visible.root.path,
          expectedProjectId: visible.projectId,
          expectedProjectRevision: visible.projectRevision,
          expectedHead: _head(99),
        ),
      ];

      for (final attempt in attempts) {
        await expectLater(
          attempt(),
          throwsA(isA<Revision3RecoveryStaleCheckpointException>()),
        );
      }
      expect(managed.recoveryCalls, 0);
    },
  );

  test(
    'unsupported managed recovery capability is rejected without a call',
    () async {
      const projectId = '17171717171717171717171717171717';
      final projectJson = _recoveryProjectJson(
        projectId: projectId,
        revision: 2,
      );
      final managed = _FakeManagedLease(
        root: Directory('recover-unsupported'),
        projectIdValue: projectId,
        projectRevision: 2,
        head: _recoveryHead(projectJson),
        canonicalProjectJson: projectJson,
      )..requiresReopenValue = true;
      final coordinator = CurrentProjectCoordinator(
        openManagedRevision3: (_) async => managed,
      );
      addTearDown(() async {
        await coordinator.shutdown();
        coordinator.dispose();
      });
      final visible = await coordinator.openManagedRevision3(managed.root);

      await expectLater(
        coordinator.recoverCurrentRevision3(
          expectedRoot: visible.root.path,
          expectedProjectId: visible.projectId,
          expectedProjectRevision: visible.projectRevision,
          expectedHead: visible.head,
        ),
        throwsA(isA<Revision3RecoveryNotSupportedException>()),
      );
      expect(managed.requiresReopen, isTrue);
    },
  );

  test(
    'healthy managed checkpoint rejects unnecessary recovery without a call',
    () async {
      const projectId = '18181818181818181818181818181818';
      final projectJson = _recoveryProjectJson(
        projectId: projectId,
        revision: 6,
      );
      final managed = _FakeRecoveryManagedLease(
        root: Directory('recover-not-required'),
        projectIdValue: projectId,
        projectRevision: 6,
        head: _recoveryHead(projectJson),
        canonicalProjectJson: projectJson,
        onRecovery: (_) => throw StateError('healthy lease must not recover'),
      );
      final coordinator = CurrentProjectCoordinator(
        openManagedRevision3: (_) async => managed,
      );
      addTearDown(() async {
        await coordinator.shutdown();
        coordinator.dispose();
      });
      final visible = await coordinator.openManagedRevision3(managed.root);

      await expectLater(
        coordinator.recoverCurrentRevision3(
          expectedRoot: visible.root.path,
          expectedProjectId: visible.projectId,
          expectedProjectRevision: visible.projectRevision,
          expectedHead: visible.head,
        ),
        throwsA(isA<Revision3RecoveryNotRequiredException>()),
      );
      expect(managed.recoveryCalls, 0);
    },
  );

  test(
    'recovery receipt mismatch relatches and never claims success',
    () async {
      const projectId = '19191919191919191919191919191919';
      final previousJson = _recoveryProjectJson(
        projectId: projectId,
        revision: 10,
      );
      final previousHead = _recoveryHead(previousJson);
      final claimedJson = _recoveryProjectJson(
        projectId: projectId,
        revision: 11,
      );
      final claimedHead = _recoveryHead(claimedJson);
      final managed = _FakeRecoveryManagedLease(
        root: Directory('recover-mismatch'),
        projectIdValue: projectId,
        projectRevision: 10,
        head: previousHead,
        canonicalProjectJson: previousJson,
        onRecovery: (lease) {
          lease.requiresReopenValue = false;
          return ManagedRevision3RecoveryCheckpoint(
            previousHead: previousHead,
            recoveredHead: claimedHead,
            projectId: projectId,
            previousProjectRevision: 10,
            recoveredProjectRevision: 11,
            repairOutcome: AtomicRepairOutcome.keptTarget,
            canonicalProjectJson: claimedJson,
          );
        },
      )..requiresReopenValue = true;
      final coordinator = CurrentProjectCoordinator(
        openManagedRevision3: (_) async => managed,
      );
      addTearDown(() async {
        await coordinator.shutdown();
        coordinator.dispose();
      });
      final visible = await coordinator.openManagedRevision3(managed.root);

      await expectLater(
        coordinator.recoverCurrentRevision3(
          expectedRoot: visible.root.path,
          expectedProjectId: visible.projectId,
          expectedProjectRevision: visible.projectRevision,
          expectedHead: visible.head,
        ),
        throwsA(isA<Revision3RecoveryFailedException>()),
      );

      expect(managed.recoveryCalls, 1);
      expect(managed.recoveryRelatchCalls, 1);
      final state = coordinator.state as ManagedRevision3CurrentProjectState;
      expect(state.projectRevision, 10);
      expect(state.head.canonicalJson, previousHead.canonicalJson);
      expect(state.requiresReopen, isTrue);
    },
  );

  test('normal work resumes only after a later successful recovery', () async {
    const projectId = '20202020202020202020202020202020';
    final projectJson = _recoveryProjectJson(projectId: projectId, revision: 5);
    final head = _recoveryHead(projectJson);
    late _FakeRecoveryManagedLease managed;
    managed = _FakeRecoveryManagedLease(
      root: Directory('recover-retry'),
      projectIdValue: projectId,
      projectRevision: 5,
      head: head,
      canonicalProjectJson: projectJson,
      contentIndex: _contentIndex(projectId: projectId, revision: 5),
      onRecovery: (lease) {
        if (lease.recoveryCalls == 1) {
          lease.requiresReopenValue = false;
          throw StateError('first repair did not verify');
        }
        lease.requiresReopenValue = false;
        return ManagedRevision3RecoveryCheckpoint(
          previousHead: head,
          recoveredHead: head,
          projectId: projectId,
          previousProjectRevision: 5,
          recoveredProjectRevision: 5,
          repairOutcome: AtomicRepairOutcome.clean,
          canonicalProjectJson: projectJson,
        );
      },
    )..requiresReopenValue = true;
    final coordinator = CurrentProjectCoordinator(
      openManagedRevision3: (_) async => managed,
    );
    addTearDown(() async {
      await coordinator.shutdown();
      coordinator.dispose();
    });
    final visible = await coordinator.openManagedRevision3(managed.root);

    await expectLater(
      coordinator.recoverCurrentRevision3(
        expectedRoot: visible.root.path,
        expectedProjectId: visible.projectId,
        expectedProjectRevision: visible.projectRevision,
        expectedHead: visible.head,
      ),
      throwsA(isA<Revision3RecoveryFailedException>()),
    );
    expect(
      (coordinator.state as ManagedRevision3CurrentProjectState).requiresReopen,
      isTrue,
    );
    await expectLater(
      coordinator.readCurrentRevision3ContentIndex(),
      throwsA(isA<Revision3ContentRequiresReopenException>()),
    );
    expect(managed.contentReadCalls, 0);

    await coordinator.recoverCurrentRevision3(
      expectedRoot: visible.root.path,
      expectedProjectId: visible.projectId,
      expectedProjectRevision: visible.projectRevision,
      expectedHead: visible.head,
    );
    final index = await coordinator.readCurrentRevision3ContentIndex();
    expect(index.projectRevision, 5);
    expect(managed.recoveryCalls, 2);
    expect(managed.recoveryRelatchCalls, 1);
    expect(managed.contentReadCalls, 1);
  });

  test('adopting legacy retires the managed lease exactly once', () async {
    final managed = _FakeManagedLease(
      root: Directory('before-legacy'),
      projectIdValue: '05050505050505050505050505050505',
      projectRevision: 5,
      head: _head(5),
    );
    final legacy = _FakeLegacyLease(
      path: 'adopted.goremod',
      hasUnsavedChanges: false,
    );
    final coordinator = CurrentProjectCoordinator(
      openManagedRevision3: (_) async => managed,
    );
    addTearDown(() async {
      await coordinator.shutdown();
      coordinator.dispose();
    });
    await coordinator.openManagedRevision3(Directory('before-legacy'));

    final state = await coordinator.adoptLegacy(legacy);

    expect(state.path, 'adopted.goremod');
    expect(coordinator.state, isA<LegacyCurrentProjectState>());
    expect(managed.closeCalls, 1);
    expect(legacy.closeCalls, 0);
    await coordinator.saveCurrent();
    expect(legacy.saveCalls, 1);
    await expectLater(
      coordinator.verifyCurrent(),
      throwsA(isA<CurrentProjectOperationUnsupportedException>()),
    );
  });

  test(
    'production legacy factory creates one-shot leases across a format round trip',
    () async {
      final container = ProviderContainer.test();
      final factory = container.read(legacyCurrentProjectLeaseFactoryProvider);
      final firstLegacy = factory();
      final firstManaged = _FakeManagedLease(
        root: Directory('first-managed'),
        projectIdValue: '08080808080808080808080808080808',
        projectRevision: 8,
        head: _head(8),
      );
      final secondManaged = _FakeManagedLease(
        root: Directory('second-managed'),
        projectIdValue: '09090909090909090909090909090909',
        projectRevision: 9,
        head: _head(9),
      );
      final managed = <_FakeManagedLease>[firstManaged, secondManaged];
      var nextManaged = 0;
      container.read(currentProjectPathProvider.notifier).state =
          'first.goremod';
      final coordinator = CurrentProjectCoordinator(
        initialLegacy: firstLegacy,
        openManagedRevision3: (_) async => managed[nextManaged++],
      );
      addTearDown(() async {
        await coordinator.shutdown();
        coordinator.dispose();
        container.dispose();
      });

      await coordinator.openManagedRevision3(Directory('first-managed'));
      expect(container.read(currentProjectPathProvider), isNull);

      container.read(currentProjectPathProvider.notifier).state =
          'second.goremod';
      final secondLegacy = factory();
      expect(secondLegacy, isNot(same(firstLegacy)));
      final legacyState = await coordinator.adoptLegacy(secondLegacy);
      expect(legacyState.path, 'second.goremod');
      expect(firstManaged.closeCalls, 1);

      await coordinator.openManagedRevision3(Directory('second-managed'));
      expect(container.read(currentProjectPathProvider), isNull);
      await expectLater(
        coordinator.adoptLegacy(firstLegacy),
        throwsA(isA<StateError>()),
      );
      expect(coordinator.state, isA<ManagedRevision3CurrentProjectState>());
    },
  );

  test(
    'close failure is terminal diagnostic state and is never retried',
    () async {
      final managed = _FakeManagedLease(
        root: Directory('terminal-close'),
        projectIdValue: '06060606060606060606060606060606',
        projectRevision: 6,
        head: _head(6),
        closeFailuresRemaining: 1,
      );
      final coordinator = CurrentProjectCoordinator(
        openManagedRevision3: (_) async => managed,
      );
      addTearDown(coordinator.dispose);
      await coordinator.openManagedRevision3(Directory('terminal-close'));

      await expectLater(coordinator.closeCurrent(), throwsA(isA<StateError>()));
      expect(coordinator.state, isA<NoCurrentProjectState>());
      expect(coordinator.terminalCleanupFailures, hasLength(1));
      expect(
        coordinator.terminalCleanupFailures.single.projectKind,
        CurrentProjectKind.managedRevision3,
      );
      expect(
        coordinator.terminalCleanupFailures.single.error,
        isA<StateError>(),
      );
      expect(managed.closeCalls, 1);

      await coordinator.shutdown();
      expect(managed.closeCalls, 1);
      expect(coordinator.terminalCleanupFailures, hasLength(1));
      await coordinator.shutdown();
      expect(managed.closeCalls, 1);
    },
  );

  test(
    'retired close failure does not fail adoption or retry during shutdown',
    () async {
      final legacy = _FakeLegacyLease(
        path: 'terminal-retired.goremod',
        onClose: () => throw StateError('injected retired close failure'),
      );
      final managed = _FakeManagedLease(
        root: Directory('after-terminal-retire'),
        projectIdValue: '0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a',
        projectRevision: 10,
        head: _head(10),
      );
      final coordinator = CurrentProjectCoordinator(
        initialLegacy: legacy,
        openManagedRevision3: (_) async => managed,
      );
      addTearDown(coordinator.dispose);

      final opened = await coordinator.openManagedRevision3(
        Directory('after-terminal-retire'),
      );
      expect(opened.projectRevision, 10);
      expect(legacy.closeCalls, 1);
      expect(coordinator.terminalCleanupFailures, hasLength(1));
      expect(
        coordinator.terminalCleanupFailures.single.projectKind,
        CurrentProjectKind.legacyFormat1,
      );

      await coordinator.shutdown();
      expect(legacy.closeCalls, 1);
      expect(managed.closeCalls, 1);
      expect(coordinator.terminalCleanupFailures, hasLength(1));
    },
  );

  test('shutdown drains an accepted late open and closes its result', () async {
    final openEntered = Completer<void>();
    final releaseOpen = Completer<void>();
    final legacy = _FakeLegacyLease(path: 'initial.goremod');
    final managed = _FakeManagedLease(
      root: Directory('late'),
      projectIdValue: '07070707070707070707070707070707',
      projectRevision: 7,
      head: _head(7),
    );
    final coordinator = CurrentProjectCoordinator(
      initialLegacy: legacy,
      openManagedRevision3: (_) async {
        openEntered.complete();
        await releaseOpen.future;
        return managed;
      },
    );
    addTearDown(coordinator.dispose);

    final opening = coordinator.openManagedRevision3(Directory('late'));
    await openEntered.future;
    final shutdown = coordinator.shutdown();
    await expectLater(
      coordinator.saveCurrent(),
      throwsA(isA<CurrentProjectCoordinatorClosedException>()),
    );
    releaseOpen.complete();
    await opening;
    await shutdown;

    expect(legacy.closeCalls, 1);
    expect(managed.closeCalls, 1);
    expect(coordinator.state, isA<NoCurrentProjectState>());
  });

  test(
    'dispose drains an accepted save without reading disposed state',
    () async {
      final saveEntered = Completer<void>();
      final releaseSave = Completer<void>();
      final legacy = _FakeLegacyLease(
        path: 'dispose.goremod',
        onSave: () async {
          saveEntered.complete();
          await releaseSave.future;
        },
      );
      final coordinator = CurrentProjectCoordinator(
        initialLegacy: legacy,
        openManagedRevision3: (_) async => throw UnimplementedError(),
      );

      final saving = coordinator.saveCurrent();
      await saveEntered.future;
      coordinator.dispose();
      releaseSave.complete();

      final saved = await saving;
      expect(saved, isA<LegacyCurrentProjectState>());
      await coordinator.shutdown();
      expect(legacy.closeCalls, 1);
    },
  );

  test(
    'legacy open is validated in the coordinator lane before replacing managed',
    () async {
      final candidate = _FakeLegacyLease();
      final managed = _FakeManagedLease(
        root: Directory('managed-before-legacy-open'),
        projectIdValue: '11111111111111111111111111111111',
        projectRevision: 11,
        head: _head(11),
      );
      final coordinator = CurrentProjectCoordinator(
        createLegacy: () => candidate,
        openManagedRevision3: (_) async => managed,
      );
      addTearDown(() async {
        await coordinator.shutdown();
        coordinator.dispose();
      });
      await coordinator.openManagedRevision3(managed.root);

      final opened = await coordinator.openLegacyFromPath('opened.goremod');

      expect(opened.path, 'opened.goremod');
      expect(coordinator.state, isA<LegacyCurrentProjectState>());
      expect(candidate.openFromPathCalls, 1);
      expect(candidate.closeCalls, 0);
      expect(managed.closeCalls, 1);
    },
  );

  test(
    'failed legacy candidate open preserves the exact managed current project',
    () async {
      final expectedError = StateError('legacy candidate is invalid');
      final candidate = _FakeLegacyLease(
        onOpenFromPath: (_) => throw expectedError,
      );
      final managed = _FakeManagedLease(
        root: Directory('managed-preserved'),
        projectIdValue: '12121212121212121212121212121212',
        projectRevision: 12,
        head: _head(12),
      );
      final coordinator = CurrentProjectCoordinator(
        createLegacy: () => candidate,
        openManagedRevision3: (_) async => managed,
      );
      addTearDown(() async {
        await coordinator.shutdown();
        coordinator.dispose();
      });
      await coordinator.openManagedRevision3(managed.root);
      final before = coordinator.state;

      await expectLater(
        coordinator.openLegacyFromPath('invalid.goremod'),
        throwsA(same(expectedError)),
      );

      expect(coordinator.state, same(before));
      expect(candidate.openFromPathCalls, 1);
      expect(candidate.closeCalls, 1);
      expect(managed.closeCalls, 0);
    },
  );

  test('managed current project rejects compatibility Save As', () async {
    final managed = _FakeManagedLease(
      root: Directory('managed-no-save-as'),
      projectIdValue: '13131313131313131313131313131313',
      projectRevision: 13,
      head: _head(13),
    );
    final coordinator = CurrentProjectCoordinator(
      createLegacy: () => _FakeLegacyLease(),
      openManagedRevision3: (_) async => managed,
    );
    addTearDown(() async {
      await coordinator.shutdown();
      coordinator.dispose();
    });
    await coordinator.openManagedRevision3(managed.root);

    await expectLater(
      coordinator.saveLegacyToPath('forbidden.goremod'),
      throwsA(isA<CurrentProjectOperationUnsupportedException>()),
    );

    expect(coordinator.state, isA<ManagedRevision3CurrentProjectState>());
    expect(managed.verifyCalls, 0);
  });

  test(
    'managed content read uses the current lease and refreshes poison state',
    () async {
      final projectId = '14141414141414141414141414141414';
      final index = _contentIndex(projectId: projectId, revision: 14);
      final managed = _FakeManagedLease(
        root: Directory('managed-content'),
        projectIdValue: projectId,
        projectRevision: 14,
        head: _head(14),
        contentIndex: index,
      );
      final coordinator = CurrentProjectCoordinator(
        openManagedRevision3: (_) async => managed,
      );
      addTearDown(() async {
        await coordinator.shutdown();
        coordinator.dispose();
      });
      await coordinator.openManagedRevision3(managed.root);

      expect(await coordinator.readCurrentRevision3ContentIndex(), same(index));
      expect(managed.contentReadCalls, 1);
      expect(coordinator.state, isA<ManagedRevision3CurrentProjectState>());

      managed.onContentRead = (lease) {
        lease.requiresReopenValue = true;
        throw StateError('injected content verification failure');
      };
      await expectLater(
        coordinator.readCurrentRevision3ContentIndex(),
        throwsA(isA<Revision3ContentRequiresReopenException>()),
      );
      expect(
        (coordinator.state as ManagedRevision3CurrentProjectState)
            .requiresReopen,
        isTrue,
      );
      await expectLater(
        coordinator.readCurrentRevision3ContentIndex(),
        throwsA(isA<Revision3ContentRequiresReopenException>()),
      );
      expect(managed.contentReadCalls, 2);
    },
  );

  test(
    'dialog localization read binds the visible root project revision head and candidate',
    () async {
      const projectId = '15151515151515151515151515151515';
      const localizationId = '25252525252525252525252525252525';
      const locId = 'GORE_EXISTING_TEXT';
      final managed = _FakeManagedLease(
        root: Directory('managed-dialog-localization'),
        projectIdValue: projectId,
        projectRevision: 15,
        head: _head(15),
        onDialogLocalizationRead:
            (lease, receivedId, receivedRevision, receivedLocId) {
              expect(receivedId, localizationId);
              expect(receivedRevision, 4);
              expect(receivedLocId, locId);
              return _controllerDialogLocalizationReadResult(
                head: lease.head,
                projectId: lease.projectId,
                projectRevision: lease.projectRevision,
                localizationId: receivedId,
                localizationRevision: receivedRevision,
                locId: receivedLocId,
              );
            },
      );
      final coordinator = CurrentProjectCoordinator(
        openManagedRevision3: (_) async => managed,
      );
      addTearDown(() async {
        await coordinator.shutdown();
        coordinator.dispose();
      });
      final visible = await coordinator.openManagedRevision3(managed.root);

      final staleReads =
          <Future<AuthoringRevision3DialogLocalizationReadResult> Function()>[
            () => coordinator.readCurrentRevision3DialogLocalization(
              expectedRoot: 'another-root',
              expectedProjectId: visible.projectId,
              expectedProjectRevision: visible.projectRevision,
              expectedHead: visible.head,
              localizationId: localizationId,
              expectedLocalizationRevision: 4,
              expectedLocId: locId,
            ),
            () => coordinator.readCurrentRevision3DialogLocalization(
              expectedRoot: visible.root.path,
              expectedProjectId: '16161616161616161616161616161616',
              expectedProjectRevision: visible.projectRevision,
              expectedHead: visible.head,
              localizationId: localizationId,
              expectedLocalizationRevision: 4,
              expectedLocId: locId,
            ),
            () => coordinator.readCurrentRevision3DialogLocalization(
              expectedRoot: visible.root.path,
              expectedProjectId: visible.projectId,
              expectedProjectRevision: visible.projectRevision + 1,
              expectedHead: visible.head,
              localizationId: localizationId,
              expectedLocalizationRevision: 4,
              expectedLocId: locId,
            ),
            () => coordinator.readCurrentRevision3DialogLocalization(
              expectedRoot: visible.root.path,
              expectedProjectId: visible.projectId,
              expectedProjectRevision: visible.projectRevision,
              expectedHead: _head(16),
              localizationId: localizationId,
              expectedLocalizationRevision: 4,
              expectedLocId: locId,
            ),
          ];
      for (final read in staleReads) {
        await expectLater(
          read(),
          throwsA(
            isA<Revision3DialogLocalizationReadStaleCheckpointException>(),
          ),
        );
      }
      expect(managed.dialogLocalizationReadCalls, 0);

      final result = await coordinator.readCurrentRevision3DialogLocalization(
        expectedRoot: visible.root.path,
        expectedProjectId: visible.projectId,
        expectedProjectRevision: visible.projectRevision,
        expectedHead: visible.head,
        localizationId: localizationId,
        expectedLocalizationRevision: 4,
        expectedLocId: locId,
      );

      expect(result.projectId, projectId);
      expect(result.projectRevision, 15);
      expect(result.localizationId, localizationId);
      expect(result.localizationRevision, 4);
      expect(result.locId, locId);
      expect(result.locales.single.preview, 'Bleib stehen!');
      expect(managed.dialogLocalizationReadCalls, 1);
      expect(managed.dialogLocalizationReadIds, <String>[localizationId]);
      expect(managed.dialogLocalizationReadRevisions, <int>[4]);
      expect(managed.dialogLocalizationReadLocIds, <String>[locId]);
      expect(
        (coordinator.state as ManagedRevision3CurrentProjectState)
            .requiresReopen,
        isFalse,
      );
    },
  );

  test(
    'dialog localization result and native candidate drift map to stale',
    () async {
      const projectId = '17171717171717171717171717171717';
      const localizationId = '27272727272727272727272727272727';
      const locId = 'GORE_STALE_TEXT';
      var returnMismatchedResult = true;
      final managed = _FakeManagedLease(
        root: Directory('managed-dialog-localization-stale'),
        projectIdValue: projectId,
        projectRevision: 17,
        head: _head(17),
        onDialogLocalizationRead: (lease, id, revision, identity) {
          if (returnMismatchedResult) {
            returnMismatchedResult = false;
            return _controllerDialogLocalizationReadResult(
              head: lease.head,
              projectId: lease.projectId,
              projectRevision: lease.projectRevision + 1,
              localizationId: id,
              localizationRevision: revision,
              locId: identity,
            );
          }
          throw const ModFfiException(
            command: 'authoring_store_read_revision3_dialog_localization_v1',
            code: 'AUTHORING_REVISION3_DIALOG_LOCALIZATION_REVISION_CONFLICT',
            message: 'fake stale candidate revision',
          );
        },
      );
      final coordinator = CurrentProjectCoordinator(
        openManagedRevision3: (_) async => managed,
      );
      addTearDown(() async {
        await coordinator.shutdown();
        coordinator.dispose();
      });
      final visible = await coordinator.openManagedRevision3(managed.root);
      Future<AuthoringRevision3DialogLocalizationReadResult> read() =>
          coordinator.readCurrentRevision3DialogLocalization(
            expectedRoot: visible.root.path,
            expectedProjectId: visible.projectId,
            expectedProjectRevision: visible.projectRevision,
            expectedHead: visible.head,
            localizationId: localizationId,
            expectedLocalizationRevision: 4,
            expectedLocId: locId,
          );

      await expectLater(
        read(),
        throwsA(isA<Revision3DialogLocalizationReadStaleCheckpointException>()),
      );
      await expectLater(
        read(),
        throwsA(isA<Revision3DialogLocalizationReadStaleCheckpointException>()),
      );
      expect(managed.dialogLocalizationReadCalls, 2);
      expect(
        (coordinator.state as ManagedRevision3CurrentProjectState)
            .requiresReopen,
        isFalse,
      );
    },
  );

  test(
    'dialog localization poisoned lease maps to requires-reopen and locks retry',
    () async {
      const projectId = '18181818181818181818181818181818';
      const localizationId = '28282828282828282828282828282828';
      final managed = _FakeManagedLease(
        root: Directory('managed-dialog-localization-reopen'),
        projectIdValue: projectId,
        projectRevision: 18,
        head: _head(18),
        onDialogLocalizationRead: (lease, _, _, _) {
          lease.requiresReopenValue = true;
          throw StateError('injected localization integrity failure');
        },
      );
      final coordinator = CurrentProjectCoordinator(
        openManagedRevision3: (_) async => managed,
      );
      addTearDown(() async {
        await coordinator.shutdown();
        coordinator.dispose();
      });
      final visible = await coordinator.openManagedRevision3(managed.root);
      Future<AuthoringRevision3DialogLocalizationReadResult> read() =>
          coordinator.readCurrentRevision3DialogLocalization(
            expectedRoot: visible.root.path,
            expectedProjectId: visible.projectId,
            expectedProjectRevision: visible.projectRevision,
            expectedHead: visible.head,
            localizationId: localizationId,
            expectedLocalizationRevision: 4,
            expectedLocId: 'GORE_REOPEN_TEXT',
          );

      await expectLater(
        read(),
        throwsA(isA<Revision3DialogLocalizationReadRequiresReopenException>()),
      );
      expect(managed.dialogLocalizationReadCalls, 1);
      expect(
        (coordinator.state as ManagedRevision3CurrentProjectState)
            .requiresReopen,
        isTrue,
      );
      await expectLater(
        read(),
        throwsA(isA<Revision3DialogLocalizationReadRequiresReopenException>()),
      );
      expect(managed.dialogLocalizationReadCalls, 1);
    },
  );

  test(
    'localization edit binds exact seed and publication then refreshes the visible checkpoint',
    () async {
      const projectId = '19191919191919191919191919191919';
      const localizationId = '29292929292929292929292929292929';
      const locId = 'GORE_EDITABLE_TEXT';
      final managed = _FakeManagedLease(
        root: Directory('managed-dialog-localization-edit'),
        projectIdValue: projectId,
        projectRevision: 23,
        head: _head(23),
        onDialogLocalizationEditSeed: (lease, id, revision, identity) {
          expect(id, localizationId);
          expect(revision, 4);
          expect(identity, locId);
          return _controllerDialogLocalizationEditSeed(
            head: lease.head,
            projectId: lease.projectId,
            projectRevision: lease.projectRevision,
            localizationId: id,
            localizationRevision: revision,
            locId: identity,
          );
        },
        onDialogLocalizationEditPublish: (lease, _) {
          lease.projectRevision++;
          lease.head = _head(24);
          return Revision3DialogLocalizationEditPublication(
            projectId: lease.projectId,
            projectRevision: lease.projectRevision,
            localizationId: localizationId,
            localizationRevision: 5,
            addedLocales: const <String>[],
            removedLocales: const <String>[],
          );
        },
      );
      final coordinator = CurrentProjectCoordinator(
        openManagedRevision3: (_) async => managed,
      );
      addTearDown(() async {
        await coordinator.shutdown();
        coordinator.dispose();
      });
      final visible = await coordinator.openManagedRevision3(managed.root);

      await expectLater(
        coordinator.readCurrentRevision3DialogLocalizationEditSeed(
          expectedRoot: 'another-root',
          expectedProjectId: visible.projectId,
          expectedProjectRevision: visible.projectRevision,
          expectedHead: visible.head,
          localizationId: localizationId,
          expectedLocalizationRevision: 4,
          expectedLocId: locId,
        ),
        throwsA(isA<Revision3DialogLocalizationEditStaleCheckpointException>()),
      );
      expect(managed.dialogLocalizationEditSeedCalls, 0);
      final seed = await coordinator
          .readCurrentRevision3DialogLocalizationEditSeed(
            expectedRoot: visible.root.path,
            expectedProjectId: visible.projectId,
            expectedProjectRevision: visible.projectRevision,
            expectedHead: visible.head,
            localizationId: localizationId,
            expectedLocalizationRevision: 4,
            expectedLocId: locId,
          );
      expect(seed.locales.map((locale) => locale.text), <String>[
        'Bleib stehen!',
        'Stop right there!',
      ]);
      final plan = await _controllerDialogLocalizationEditPlan(
        head: visible.head,
        projectId: projectId,
        projectRevision: 23,
        localizationId: localizationId,
        localizationRevision: 4,
        locId: locId,
      );

      final publication = await coordinator
          .prepareAndPublishCurrentRevision3DialogLocalizationEdit(
            expectedRoot: visible.root.path,
            expectedProjectId: visible.projectId,
            expectedProjectRevision: visible.projectRevision,
            expectedHead: visible.head,
            plan: plan,
          );

      expect(publication.projectRevision, 24);
      expect(publication.localizationRevision, 5);
      expect(managed.dialogLocalizationEditSeedCalls, 1);
      expect(managed.dialogLocalizationEditPublishCalls, 1);
      final current = coordinator.state as ManagedRevision3CurrentProjectState;
      expect(current.projectRevision, 24);
      expect(current.head.canonicalJson, _head(24).canonicalJson);
      expect(current.requiresReopen, isFalse);
    },
  );

  test(
    'localization edit maps Voice conflict without poisoning then locks a poisoned lease',
    () async {
      const projectId = '20202020202020202020202020202020';
      const localizationId = '30303030303030303030303030303030';
      const locId = 'GORE_LOCKED_TEXT';
      var attempts = 0;
      final managed = _FakeManagedLease(
        root: Directory('managed-dialog-localization-edit-errors'),
        projectIdValue: projectId,
        projectRevision: 25,
        head: _head(25),
        onDialogLocalizationEditPublish: (lease, _) {
          attempts++;
          if (attempts == 1) {
            throw const ModFfiException(
              command:
                  'authoring_store_prepare_revision3_dialog_localization_edit_v1',
              code:
                  'AUTHORING_REVISION3_DIALOG_LOCALIZATION_EDIT_VOICE_CONFLICT',
              message: 'fake Voice candidate protects this text',
            );
          }
          lease.requiresReopenValue = true;
          throw StateError('injected localization-edit integrity failure');
        },
      );
      final coordinator = CurrentProjectCoordinator(
        openManagedRevision3: (_) async => managed,
      );
      addTearDown(() async {
        await coordinator.shutdown();
        coordinator.dispose();
      });
      final visible = await coordinator.openManagedRevision3(managed.root);
      final plan = await _controllerDialogLocalizationEditPlan(
        head: visible.head,
        projectId: projectId,
        projectRevision: visible.projectRevision,
        localizationId: localizationId,
        localizationRevision: 4,
        locId: locId,
      );
      Future<Revision3DialogLocalizationEditPublication> publish() =>
          coordinator.prepareAndPublishCurrentRevision3DialogLocalizationEdit(
            expectedRoot: visible.root.path,
            expectedProjectId: visible.projectId,
            expectedProjectRevision: visible.projectRevision,
            expectedHead: visible.head,
            plan: plan,
          );

      await expectLater(
        publish(),
        throwsA(isA<Revision3DialogLocalizationEditLockedVoiceTextException>()),
      );
      expect(managed.dialogLocalizationEditPublishCalls, 1);
      expect(
        (coordinator.state as ManagedRevision3CurrentProjectState)
            .requiresReopen,
        isFalse,
      );
      await expectLater(
        publish(),
        throwsA(isA<Revision3DialogLocalizationEditRequiresReopenException>()),
      );
      expect(managed.dialogLocalizationEditPublishCalls, 2);
      expect(
        (coordinator.state as ManagedRevision3CurrentProjectState)
            .requiresReopen,
        isTrue,
      );
      await expectLater(
        publish(),
        throwsA(isA<Revision3DialogLocalizationEditRequiresReopenException>()),
      );
      expect(managed.dialogLocalizationEditPublishCalls, 2);
    },
  );

  test(
    'Quest source inspection binds the visible root, identity, revision, and head',
    () async {
      const projectId = '41414141414141414141414141414141';
      const questId = '71717171717171717171717171717171';
      const gameRoot = r'C:\Games\Gothic Remake';
      final managed = _FakeManagedLease(
        root: Directory('managed-quest-inspection'),
        projectIdValue: projectId,
        projectRevision: 41,
        head: _head(41),
        onQuestInspection: (lease, receivedGameRoot, receivedQuestId) {
          expect(receivedGameRoot, gameRoot);
          expect(receivedQuestId, questId);
          return _controllerQuestInspectionResult(
            head: lease.head,
            projectId: lease.projectId,
            projectRevision: lease.projectRevision,
            questId: receivedQuestId,
          );
        },
      );
      final coordinator = CurrentProjectCoordinator(
        openManagedRevision3: (_) async => managed,
      );
      addTearDown(() async {
        await coordinator.shutdown();
        coordinator.dispose();
      });
      final visible = await coordinator.openManagedRevision3(managed.root);

      final staleRequests =
          <Future<AuthoringRevision3QuestSourceInspectionResult> Function()>[
            () => coordinator.inspectCurrentRevision3QuestSource(
              expectedRoot: 'another-root',
              expectedProjectId: visible.projectId,
              expectedProjectRevision: visible.projectRevision,
              expectedHead: visible.head,
              gameRoot: gameRoot,
              questId: questId,
            ),
            () => coordinator.inspectCurrentRevision3QuestSource(
              expectedRoot: visible.root.path,
              expectedProjectId: '42424242424242424242424242424242',
              expectedProjectRevision: visible.projectRevision,
              expectedHead: visible.head,
              gameRoot: gameRoot,
              questId: questId,
            ),
            () => coordinator.inspectCurrentRevision3QuestSource(
              expectedRoot: visible.root.path,
              expectedProjectId: visible.projectId,
              expectedProjectRevision: visible.projectRevision + 1,
              expectedHead: visible.head,
              gameRoot: gameRoot,
              questId: questId,
            ),
            () => coordinator.inspectCurrentRevision3QuestSource(
              expectedRoot: visible.root.path,
              expectedProjectId: visible.projectId,
              expectedProjectRevision: visible.projectRevision,
              expectedHead: _head(42),
              gameRoot: gameRoot,
              questId: questId,
            ),
          ];
      for (final inspect in staleRequests) {
        await expectLater(
          inspect(),
          throwsA(
            isA<Revision3QuestSourceInspectionStaleCheckpointException>(),
          ),
        );
      }
      expect(managed.questInspectionCalls, 0);

      final result = await coordinator.inspectCurrentRevision3QuestSource(
        expectedRoot: visible.root.path,
        expectedProjectId: visible.projectId,
        expectedProjectRevision: visible.projectRevision,
        expectedHead: visible.head,
        gameRoot: gameRoot,
        questId: questId,
      );

      expect(result.projectId, visible.projectId);
      expect(result.projectRevision, visible.projectRevision);
      expect(result.head.canonicalJson, visible.head.canonicalJson);
      expect(result.questId, questId);
      expect(managed.questInspectionCalls, 1);
      expect(managed.questInspectionGameRoots, <String>[gameRoot]);
      expect(managed.questInspectionQuestIds, <String>[questId]);
      final after = coordinator.state as ManagedRevision3CurrentProjectState;
      expect(after.root.path, visible.root.path);
      expect(after.projectId, visible.projectId);
      expect(after.projectRevision, visible.projectRevision);
      expect(after.head.canonicalJson, visible.head.canonicalJson);
      expect(after.requiresReopen, isFalse);
    },
  );

  test('Quest source inspection maps a mismatched result to stale', () async {
    const projectId = '43434343434343434343434343434343';
    const questId = '73737373737373737373737373737373';
    final managed = _FakeManagedLease(
      root: Directory('managed-quest-inspection-result-stale'),
      projectIdValue: projectId,
      projectRevision: 43,
      head: _head(43),
      onQuestInspection: (lease, _, receivedQuestId) =>
          _controllerQuestInspectionResult(
            head: _head(44),
            projectId: lease.projectId,
            projectRevision: lease.projectRevision,
            questId: receivedQuestId,
          ),
    );
    final coordinator = CurrentProjectCoordinator(
      openManagedRevision3: (_) async => managed,
    );
    addTearDown(() async {
      await coordinator.shutdown();
      coordinator.dispose();
    });
    final visible = await coordinator.openManagedRevision3(managed.root);

    await expectLater(
      coordinator.inspectCurrentRevision3QuestSource(
        expectedRoot: visible.root.path,
        expectedProjectId: visible.projectId,
        expectedProjectRevision: visible.projectRevision,
        expectedHead: visible.head,
        gameRoot: r'C:\Games\Gothic Remake',
        questId: questId,
      ),
      throwsA(isA<Revision3QuestSourceInspectionStaleCheckpointException>()),
    );

    expect(managed.questInspectionCalls, 1);
    expect(
      (coordinator.state as ManagedRevision3CurrentProjectState).requiresReopen,
      isFalse,
    );
  });

  test(
    'Quest source inspection maps poisoned lease state to requires-reopen and locks retry',
    () async {
      const projectId = '45454545454545454545454545454545';
      const questId = '75757575757575757575757575757575';
      final managed = _FakeManagedLease(
        root: Directory('managed-quest-inspection-reopen'),
        projectIdValue: projectId,
        projectRevision: 45,
        head: _head(45),
        onQuestInspection: (lease, _, _) {
          lease.requiresReopenValue = true;
          throw StateError('injected Quest inspection integrity failure');
        },
      );
      final coordinator = CurrentProjectCoordinator(
        openManagedRevision3: (_) async => managed,
      );
      addTearDown(() async {
        await coordinator.shutdown();
        coordinator.dispose();
      });
      final visible = await coordinator.openManagedRevision3(managed.root);

      Future<AuthoringRevision3QuestSourceInspectionResult> inspect() =>
          coordinator.inspectCurrentRevision3QuestSource(
            expectedRoot: visible.root.path,
            expectedProjectId: visible.projectId,
            expectedProjectRevision: visible.projectRevision,
            expectedHead: visible.head,
            gameRoot: r'C:\Games\Gothic Remake',
            questId: questId,
          );

      await expectLater(
        inspect(),
        throwsA(isA<Revision3QuestSourceInspectionRequiresReopenException>()),
      );
      expect(managed.questInspectionCalls, 1);
      expect(
        (coordinator.state as ManagedRevision3CurrentProjectState)
            .requiresReopen,
        isTrue,
      );
      await expectLater(
        inspect(),
        throwsA(isA<Revision3QuestSourceInspectionRequiresReopenException>()),
      );
      expect(managed.questInspectionCalls, 1);
    },
  );

  test(
    'NPC source inspection binds the exact visible checkpoint without a game root',
    () async {
      const projectId = '46464646464646464646464646464646';
      const npcId = '76767676767676767676767676767676';
      final managed = _FakeManagedLease(
        root: Directory('managed-npc-inspection'),
        projectIdValue: projectId,
        projectRevision: 46,
        head: _head(46),
        onNpcInspection: (lease, receivedNpcId) {
          expect(receivedNpcId, npcId);
          return revision3NpcInspectionResult(
            head: lease.head,
            projectJson: revision3NpcInspectionProjectJson(
              projectId: lease.projectId,
              revision: lease.projectRevision,
            ),
            npcId: receivedNpcId,
          );
        },
      );
      final coordinator = CurrentProjectCoordinator(
        openManagedRevision3: (_) async => managed,
      );
      addTearDown(() async {
        await coordinator.shutdown();
        coordinator.dispose();
      });
      final visible = await coordinator.openManagedRevision3(managed.root);

      final staleRequests =
          <Future<AuthoringRevision3NpcSourceInspectionResult> Function()>[
            () => coordinator.inspectCurrentRevision3NpcSource(
              expectedRoot: 'another-root',
              expectedProjectId: visible.projectId,
              expectedProjectRevision: visible.projectRevision,
              expectedHead: visible.head,
              npcId: npcId,
            ),
            () => coordinator.inspectCurrentRevision3NpcSource(
              expectedRoot: visible.root.path,
              expectedProjectId: '47474747474747474747474747474747',
              expectedProjectRevision: visible.projectRevision,
              expectedHead: visible.head,
              npcId: npcId,
            ),
            () => coordinator.inspectCurrentRevision3NpcSource(
              expectedRoot: visible.root.path,
              expectedProjectId: visible.projectId,
              expectedProjectRevision: visible.projectRevision + 1,
              expectedHead: visible.head,
              npcId: npcId,
            ),
            () => coordinator.inspectCurrentRevision3NpcSource(
              expectedRoot: visible.root.path,
              expectedProjectId: visible.projectId,
              expectedProjectRevision: visible.projectRevision,
              expectedHead: _head(47),
              npcId: npcId,
            ),
          ];
      for (final inspect in staleRequests) {
        await expectLater(
          inspect(),
          throwsA(isA<Revision3NpcSourceInspectionStaleCheckpointException>()),
        );
      }
      expect(managed.npcInspectionCalls, 0);

      final result = await coordinator.inspectCurrentRevision3NpcSource(
        expectedRoot: visible.root.path,
        expectedProjectId: visible.projectId,
        expectedProjectRevision: visible.projectRevision,
        expectedHead: visible.head,
        npcId: npcId,
      );

      expect(result.projectId, visible.projectId);
      expect(result.projectRevision, visible.projectRevision);
      expect(result.head.canonicalJson, visible.head.canonicalJson);
      expect(result.npcId, npcId);
      expect(managed.npcInspectionCalls, 1);
      expect(managed.npcInspectionNpcIds, <String>[npcId]);
      expect(
        (coordinator.state as ManagedRevision3CurrentProjectState)
            .requiresReopen,
        isFalse,
      );
    },
  );

  test('NPC source inspection maps a mismatched result to stale', () async {
    const projectId = '48484848484848484848484848484848';
    const npcId = '78787878787878787878787878787878';
    final managed = _FakeManagedLease(
      root: Directory('managed-npc-inspection-result-stale'),
      projectIdValue: projectId,
      projectRevision: 48,
      head: _head(48),
      onNpcInspection: (lease, receivedNpcId) => revision3NpcInspectionResult(
        head: _head(49),
        projectJson: revision3NpcInspectionProjectJson(
          projectId: lease.projectId,
          revision: lease.projectRevision,
        ),
        npcId: receivedNpcId,
      ),
    );
    final coordinator = CurrentProjectCoordinator(
      openManagedRevision3: (_) async => managed,
    );
    addTearDown(() async {
      await coordinator.shutdown();
      coordinator.dispose();
    });
    final visible = await coordinator.openManagedRevision3(managed.root);

    await expectLater(
      coordinator.inspectCurrentRevision3NpcSource(
        expectedRoot: visible.root.path,
        expectedProjectId: visible.projectId,
        expectedProjectRevision: visible.projectRevision,
        expectedHead: visible.head,
        npcId: npcId,
      ),
      throwsA(isA<Revision3NpcSourceInspectionStaleCheckpointException>()),
    );

    expect(managed.npcInspectionCalls, 1);
    expect(
      (coordinator.state as ManagedRevision3CurrentProjectState).requiresReopen,
      isFalse,
    );
  });

  test(
    'NPC source inspection maps poisoned lease state to requires-reopen and locks retry',
    () async {
      const projectId = '49494949494949494949494949494949';
      const npcId = '79797979797979797979797979797979';
      final managed = _FakeManagedLease(
        root: Directory('managed-npc-inspection-reopen'),
        projectIdValue: projectId,
        projectRevision: 49,
        head: _head(49),
        onNpcInspection: (lease, _) {
          lease.requiresReopenValue = true;
          throw StateError('injected NPC inspection integrity failure');
        },
      );
      final coordinator = CurrentProjectCoordinator(
        openManagedRevision3: (_) async => managed,
      );
      addTearDown(() async {
        await coordinator.shutdown();
        coordinator.dispose();
      });
      final visible = await coordinator.openManagedRevision3(managed.root);

      Future<AuthoringRevision3NpcSourceInspectionResult> inspect() =>
          coordinator.inspectCurrentRevision3NpcSource(
            expectedRoot: visible.root.path,
            expectedProjectId: visible.projectId,
            expectedProjectRevision: visible.projectRevision,
            expectedHead: visible.head,
            npcId: npcId,
          );

      await expectLater(
        inspect(),
        throwsA(isA<Revision3NpcSourceInspectionRequiresReopenException>()),
      );
      expect(managed.npcInspectionCalls, 1);
      expect(
        (coordinator.state as ManagedRevision3CurrentProjectState)
            .requiresReopen,
        isTrue,
      );
      await expectLater(
        inspect(),
        throwsA(isA<Revision3NpcSourceInspectionRequiresReopenException>()),
      );
      expect(managed.npcInspectionCalls, 1);
    },
  );

  test(
    'managed compiler rejects stale visible and selected identities before lease callback',
    () async {
      final fixture = Revision3QuestOutlineFixture();
      final projectJson = fixture.projectJson;
      final head = headFor(projectJson);
      final managed = _FakeManagedLease(
        root: Directory('managed-compiler-stale'),
        projectIdValue: revision3QuestOutlineProjectId,
        projectRevision: fixture.projectRevision,
        head: head,
        canonicalProjectJson: projectJson,
      );
      final coordinator = CurrentProjectCoordinator(
        openManagedRevision3: (_) async => managed,
      );
      addTearDown(() async {
        await coordinator.shutdown();
        coordinator.dispose();
      });
      final visible = await coordinator.openManagedRevision3(managed.root);

      final staleRequests =
          <Future<ManagedRevision3CompilerCheckReceipt> Function()>[
            () => _checkControllerManagedCompiler(
              coordinator,
              visible,
              fixture,
              expectedRoot: 'another-root',
            ),
            () => _checkControllerManagedCompiler(
              coordinator,
              visible,
              fixture,
              expectedProjectId: 'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa',
            ),
            () => _checkControllerManagedCompiler(
              coordinator,
              visible,
              fixture,
              expectedProjectRevision: fixture.projectRevision + 1,
            ),
            () => _checkControllerManagedCompiler(
              coordinator,
              visible,
              fixture,
              expectedHead: _head(404),
            ),
            () => _checkControllerManagedCompiler(
              coordinator,
              visible,
              fixture,
              expectedEntityRevision: fixture.questRevision + 1,
            ),
            () => _checkControllerManagedCompiler(
              coordinator,
              visible,
              fixture,
              expectedModuleId: 'bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb',
            ),
            () => _checkControllerManagedCompiler(
              coordinator,
              visible,
              fixture,
              expectedModuleRevision: fixture.moduleRevision + 1,
            ),
          ];
      for (final check in staleRequests) {
        await expectLater(
          check(),
          throwsA(isA<Revision3ManagedCompilerCheckStaleCheckpointException>()),
        );
      }

      expect(managed.managedCompilerCheckCalls, 0);
      expect(
        (coordinator.state as ManagedRevision3CurrentProjectState)
            .requiresReopen,
        isFalse,
      );
    },
  );

  test(
    'managed compiler returns exact evidence and structured recovery without publication',
    () async {
      for (final recoveryRequired in <bool>[false, true]) {
        final fixture = Revision3QuestOutlineFixture();
        final projectJson = fixture.projectJson;
        final head = headFor(projectJson);
        late _FakeManagedLease managed;
        managed = _FakeManagedLease(
          root: Directory('managed-compiler-exact-$recoveryRequired'),
          projectIdValue: revision3QuestOutlineProjectId,
          projectRevision: fixture.projectRevision,
          head: head,
          canonicalProjectJson: projectJson,
          onManagedCompilerCheck:
              (
                lease,
                kind,
                gameRoot,
                entityId,
                entityRevision,
                moduleId,
                moduleRevision,
              ) {
                expect(
                  kind,
                  AuthoringRevision3ManagedCompilerEntityKind.questDraft,
                );
                expect(gameRoot, r'C:\Games\Gothic Remake');
                expect(entityId, revision3QuestOutlineQuestId);
                expect(entityRevision, fixture.questRevision);
                expect(moduleId, revision3QuestOutlineModuleId);
                expect(moduleRevision, fixture.moduleRevision);
                return ManagedRevision3CompilerCheckReceipt(
                  result: _controllerManagedCompilerCheckResult(
                    head: lease.head,
                    projectJson: lease.canonicalProjectJson,
                    recoveryRequired: recoveryRequired,
                  ),
                  storeStillExactCurrent: true,
                );
              },
        );
        final coordinator = CurrentProjectCoordinator(
          openManagedRevision3: (_) async => managed,
        );
        final visible = await coordinator.openManagedRevision3(managed.root);

        final receipt = await _checkControllerManagedCompiler(
          coordinator,
          visible,
          fixture,
        );

        expect(receipt.storeStillExactCurrent, isTrue);
        expect(receipt.recoveryRequired, recoveryRequired);
        expect(receipt.acceptedAtExactCurrent, !recoveryRequired);
        expect(managed.managedCompilerCheckCalls, 1);
        expect(managed.questPublishCalls, 0);
        expect(managed.npcPublishCalls, 0);
        expect(managed.dataAssetPublishCalls, 0);
        final after = coordinator.state as ManagedRevision3CurrentProjectState;
        expect(after.head.canonicalJson, visible.head.canonicalJson);
        expect(after.projectRevision, visible.projectRevision);
        expect(after.requiresReopen, isFalse);
        await coordinator.shutdown();
        coordinator.dispose();
      }
    },
  );

  test(
    'managed compiler preserves evidence and publishes requires-reopen after post-call drift',
    () async {
      final fixture = Revision3QuestOutlineFixture();
      final projectJson = fixture.projectJson;
      final head = headFor(projectJson);
      late _FakeManagedLease managed;
      managed = _FakeManagedLease(
        root: Directory('managed-compiler-post-drift'),
        projectIdValue: revision3QuestOutlineProjectId,
        projectRevision: fixture.projectRevision,
        head: head,
        canonicalProjectJson: projectJson,
        onManagedCompilerCheck: (lease, _, _, _, _, _, _) {
          final result = _controllerManagedCompilerCheckResult(
            head: lease.head,
            projectJson: lease.canonicalProjectJson,
          );
          lease.requiresReopenValue = true;
          return ManagedRevision3CompilerCheckReceipt(
            result: result,
            storeStillExactCurrent: false,
          );
        },
      );
      final coordinator = CurrentProjectCoordinator(
        openManagedRevision3: (_) async => managed,
      );
      addTearDown(() async {
        await coordinator.shutdown();
        coordinator.dispose();
      });
      final visible = await coordinator.openManagedRevision3(managed.root);

      final receipt = await _checkControllerManagedCompiler(
        coordinator,
        visible,
        fixture,
      );

      expect(receipt.result.compiler.compiledEvidenceOnly, isTrue);
      expect(receipt.storeStillExactCurrent, isFalse);
      expect(receipt.acceptedAtExactCurrent, isFalse);
      expect(
        (coordinator.state as ManagedRevision3CurrentProjectState)
            .requiresReopen,
        isTrue,
      );
      await expectLater(
        _checkControllerManagedCompiler(coordinator, visible, fixture),
        throwsA(isA<Revision3ManagedCompilerCheckRequiresReopenException>()),
      );
      expect(managed.managedCompilerCheckCalls, 1);
    },
  );

  test(
    'DataAsset package browsing binds the exact visible checkpoint and game root',
    () async {
      const projectId = '50505050505050505050505050505050';
      final managed = _FakeManagedLease(
        root: Directory('managed-dataasset-package-index'),
        projectIdValue: projectId,
        projectRevision: 50,
        head: _head(50),
        onDataAssetPackageIndexRead: (lease, gameRoot) {
          expect(gameRoot, r'C:\Games\Gothic Remake');
          return _controllerDataAssetPackageIndexResult(
            head: lease.head,
            projectId: lease.projectId,
            projectRevision: lease.projectRevision,
          );
        },
      );
      final coordinator = CurrentProjectCoordinator(
        openManagedRevision3: (_) async => managed,
      );
      addTearDown(() async {
        await coordinator.shutdown();
        coordinator.dispose();
      });
      final visible = await coordinator.openManagedRevision3(managed.root);

      for (final read
          in <Future<AuthoringRevision3DataAssetPackageIndexResult> Function()>[
            () => coordinator.readCurrentRevision3DataAssetPackageIndex(
              expectedRoot: 'another-root',
              expectedProjectId: visible.projectId,
              expectedProjectRevision: visible.projectRevision,
              expectedHead: visible.head,
              gameRoot: r'C:\Games\Gothic Remake',
            ),
            () => coordinator.readCurrentRevision3DataAssetPackageIndex(
              expectedRoot: visible.root.path,
              expectedProjectId: '51515151515151515151515151515151',
              expectedProjectRevision: visible.projectRevision,
              expectedHead: visible.head,
              gameRoot: r'C:\Games\Gothic Remake',
            ),
            () => coordinator.readCurrentRevision3DataAssetPackageIndex(
              expectedRoot: visible.root.path,
              expectedProjectId: visible.projectId,
              expectedProjectRevision: visible.projectRevision + 1,
              expectedHead: visible.head,
              gameRoot: r'C:\Games\Gothic Remake',
            ),
            () => coordinator.readCurrentRevision3DataAssetPackageIndex(
              expectedRoot: visible.root.path,
              expectedProjectId: visible.projectId,
              expectedProjectRevision: visible.projectRevision,
              expectedHead: _head(51),
              gameRoot: r'C:\Games\Gothic Remake',
            ),
          ]) {
        await expectLater(
          read(),
          throwsA(
            isA<Revision3DataAssetPackageIndexStaleCheckpointException>(),
          ),
        );
      }
      await expectLater(
        coordinator.readCurrentRevision3DataAssetPackageIndex(
          expectedRoot: visible.root.path,
          expectedProjectId: visible.projectId,
          expectedProjectRevision: visible.projectRevision,
          expectedHead: visible.head,
          gameRoot: '',
        ),
        throwsA(isA<CurrentProjectOperationUnsupportedException>()),
      );
      expect(managed.dataAssetPackageIndexReadCalls, 0);

      final result = await coordinator
          .readCurrentRevision3DataAssetPackageIndex(
            expectedRoot: visible.root.path,
            expectedProjectId: visible.projectId,
            expectedProjectRevision: visible.projectRevision,
            expectedHead: visible.head,
            gameRoot: r'C:\Games\Gothic Remake',
          );

      expect(result.projectId, visible.projectId);
      expect(result.projectRevision, visible.projectRevision);
      expect(
        result.index.candidates.single.targetPath,
        '/Game/Characters/DA_Asghan',
      );
      expect(managed.dataAssetPackageIndexReadCalls, 1);
      expect(managed.dataAssetPackageIndexGameRoots, <String>[
        r'C:\Games\Gothic Remake',
      ]);
      expect(
        (coordinator.state as ManagedRevision3CurrentProjectState)
            .requiresReopen,
        isFalse,
      );
    },
  );

  test('DataAsset package result mismatch maps to stale', () async {
    const projectId = '52525252525252525252525252525252';
    final managed = _FakeManagedLease(
      root: Directory('managed-dataasset-package-index-stale'),
      projectIdValue: projectId,
      projectRevision: 52,
      head: _head(52),
      onDataAssetPackageIndexRead: (lease, _) =>
          _controllerDataAssetPackageIndexResult(
            head: lease.head,
            projectId: lease.projectId,
            projectRevision: lease.projectRevision + 1,
          ),
    );
    final coordinator = CurrentProjectCoordinator(
      openManagedRevision3: (_) async => managed,
    );
    addTearDown(() async {
      await coordinator.shutdown();
      coordinator.dispose();
    });
    final visible = await coordinator.openManagedRevision3(managed.root);

    await expectLater(
      coordinator.readCurrentRevision3DataAssetPackageIndex(
        expectedRoot: visible.root.path,
        expectedProjectId: visible.projectId,
        expectedProjectRevision: visible.projectRevision,
        expectedHead: visible.head,
        gameRoot: r'C:\Games\Gothic Remake',
      ),
      throwsA(isA<Revision3DataAssetPackageIndexStaleCheckpointException>()),
    );
    expect(managed.dataAssetPackageIndexReadCalls, 1);
    expect(
      (coordinator.state as ManagedRevision3CurrentProjectState).requiresReopen,
      isFalse,
    );
  });

  test(
    'DataAsset package browsing maps poisoned lease state to requires-reopen',
    () async {
      const projectId = '53535353535353535353535353535353';
      final managed = _FakeManagedLease(
        root: Directory('managed-dataasset-package-index-reopen'),
        projectIdValue: projectId,
        projectRevision: 53,
        head: _head(53),
        onDataAssetPackageIndexRead: (lease, _) {
          lease.requiresReopenValue = true;
          throw StateError('injected DataAsset package-index failure');
        },
      );
      final coordinator = CurrentProjectCoordinator(
        openManagedRevision3: (_) async => managed,
      );
      addTearDown(() async {
        await coordinator.shutdown();
        coordinator.dispose();
      });
      final visible = await coordinator.openManagedRevision3(managed.root);

      Future<AuthoringRevision3DataAssetPackageIndexResult> read() =>
          coordinator.readCurrentRevision3DataAssetPackageIndex(
            expectedRoot: visible.root.path,
            expectedProjectId: visible.projectId,
            expectedProjectRevision: visible.projectRevision,
            expectedHead: visible.head,
            gameRoot: r'C:\Games\Gothic Remake',
          );

      await expectLater(
        read(),
        throwsA(isA<Revision3DataAssetPackageIndexRequiresReopenException>()),
      );
      expect(managed.dataAssetPackageIndexReadCalls, 1);
      expect(
        (coordinator.state as ManagedRevision3CurrentProjectState)
            .requiresReopen,
        isTrue,
      );
      await expectLater(
        read(),
        throwsA(isA<Revision3DataAssetPackageIndexRequiresReopenException>()),
      );
      expect(managed.dataAssetPackageIndexReadCalls, 1);
    },
  );

  test(
    'installed DataAsset inspection binds visible identity, snapshot, and original candidate',
    () async {
      const projectId = '54545454545454545454545454545454';
      late AuthoringRevision3DataAssetPackageIndexResult exactSnapshot;
      late AuthoringRevision3DataAssetPackageCandidate exactCandidate;
      final managed = _FakeManagedLease(
        root: Directory('managed-installed-dataasset-inspection'),
        projectIdValue: projectId,
        projectRevision: 54,
        head: _head(54),
        onDataAssetPackageIndexRead: (lease, _) =>
            _controllerDataAssetPackageIndexResult(
              head: lease.head,
              projectId: lease.projectId,
              projectRevision: lease.projectRevision,
            ),
        onInstalledDataAssetInspection: (lease, gameRoot, snapshot, candidate) {
          expect(gameRoot, r'C:\Games\Gothic Remake');
          expect(identical(snapshot, exactSnapshot), isTrue);
          expect(identical(candidate, exactCandidate), isTrue);
          return _controllerInstalledDataAssetInspectionResult(
            expectedSnapshot: snapshot,
            candidate: candidate,
          );
        },
      );
      final coordinator = CurrentProjectCoordinator(
        openManagedRevision3: (_) async => managed,
      );
      addTearDown(() async {
        await coordinator.shutdown();
        coordinator.dispose();
      });
      final visible = await coordinator.openManagedRevision3(managed.root);
      exactSnapshot = await coordinator
          .readCurrentRevision3DataAssetPackageIndex(
            expectedRoot: visible.root.path,
            expectedProjectId: visible.projectId,
            expectedProjectRevision: visible.projectRevision,
            expectedHead: visible.head,
            gameRoot: r'C:\Games\Gothic Remake',
          );
      exactCandidate = exactSnapshot.index.candidates.single;
      final otherSnapshot = _controllerDataAssetPackageIndexResult(
        head: visible.head,
        projectId: visible.projectId,
        projectRevision: visible.projectRevision,
      );

      for (final inspect
          in <
            Future<AuthoringRevision3InstalledDataAssetInspectionResult>
            Function()
          >[
            () => coordinator.inspectCurrentRevision3InstalledDataAsset(
              expectedRoot: 'another-root',
              expectedProjectId: visible.projectId,
              expectedProjectRevision: visible.projectRevision,
              expectedHead: visible.head,
              gameRoot: r'C:\Games\Gothic Remake',
              expectedSnapshot: exactSnapshot,
              candidate: exactCandidate,
            ),
            () => coordinator.inspectCurrentRevision3InstalledDataAsset(
              expectedRoot: visible.root.path,
              expectedProjectId: visible.projectId,
              expectedProjectRevision: visible.projectRevision,
              expectedHead: visible.head,
              gameRoot: r'C:\Games\Gothic Remake',
              expectedSnapshot: exactSnapshot,
              candidate: otherSnapshot.index.candidates.single,
            ),
          ]) {
        await expectLater(
          inspect(),
          throwsA(
            isA<
              Revision3InstalledDataAssetInspectionStaleCheckpointException
            >(),
          ),
        );
      }
      expect(managed.installedDataAssetInspectionCalls, 0);

      final result = await coordinator
          .inspectCurrentRevision3InstalledDataAsset(
            expectedRoot: visible.root.path,
            expectedProjectId: visible.projectId,
            expectedProjectRevision: visible.projectRevision,
            expectedHead: visible.head,
            gameRoot: r'C:\Games\Gothic Remake',
            expectedSnapshot: exactSnapshot,
            candidate: exactCandidate,
          );

      expect(result.projectId, visible.projectId);
      expect(result.projectRevision, visible.projectRevision);
      expect(result.candidateOrdinal, exactCandidate.ordinal);
      expect(managed.installedDataAssetInspectionCalls, 1);
      expect(managed.installedDataAssetInspectionGameRoots, <String>[
        r'C:\Games\Gothic Remake',
      ]);
      expect(
        identical(
          managed.installedDataAssetInspectionSnapshots.single,
          exactSnapshot,
        ),
        isTrue,
      );
      expect(
        identical(
          managed.installedDataAssetInspectionCandidates.single,
          exactCandidate,
        ),
        isTrue,
      );
    },
  );

  test('installed DataAsset result identity drift maps to stale', () async {
    const projectId = '55555555555555555555555555555555';
    final managed = _FakeManagedLease(
      root: Directory('managed-installed-dataasset-result-stale'),
      projectIdValue: projectId,
      projectRevision: 55,
      head: _head(55),
      onDataAssetPackageIndexRead: (lease, _) =>
          _controllerDataAssetPackageIndexResult(
            head: lease.head,
            projectId: lease.projectId,
            projectRevision: lease.projectRevision,
          ),
      onInstalledDataAssetInspection: (lease, _, snapshot, candidate) {
        final drifted = _controllerDataAssetPackageIndexResult(
          head: lease.head,
          projectId: lease.projectId,
          projectRevision: lease.projectRevision + 1,
        );
        return _controllerInstalledDataAssetInspectionResult(
          expectedSnapshot: drifted,
          candidate: drifted.index.candidates[candidate.ordinal],
        );
      },
    );
    final coordinator = CurrentProjectCoordinator(
      openManagedRevision3: (_) async => managed,
    );
    addTearDown(() async {
      await coordinator.shutdown();
      coordinator.dispose();
    });
    final visible = await coordinator.openManagedRevision3(managed.root);
    final snapshot = await coordinator
        .readCurrentRevision3DataAssetPackageIndex(
          expectedRoot: visible.root.path,
          expectedProjectId: visible.projectId,
          expectedProjectRevision: visible.projectRevision,
          expectedHead: visible.head,
          gameRoot: r'C:\Games\Gothic Remake',
        );

    await expectLater(
      coordinator.inspectCurrentRevision3InstalledDataAsset(
        expectedRoot: visible.root.path,
        expectedProjectId: visible.projectId,
        expectedProjectRevision: visible.projectRevision,
        expectedHead: visible.head,
        gameRoot: r'C:\Games\Gothic Remake',
        expectedSnapshot: snapshot,
        candidate: snapshot.index.candidates.single,
      ),
      throwsA(
        isA<Revision3InstalledDataAssetInspectionStaleCheckpointException>(),
      ),
    );
    expect(managed.installedDataAssetInspectionCalls, 1);
    expect(
      (coordinator.state as ManagedRevision3CurrentProjectState).requiresReopen,
      isFalse,
    );
  });

  test(
    'installed DataAsset inspection maps poisoned lease state to requires-reopen',
    () async {
      const projectId = '56565656565656565656565656565656';
      final managed = _FakeManagedLease(
        root: Directory('managed-installed-dataasset-reopen'),
        projectIdValue: projectId,
        projectRevision: 56,
        head: _head(56),
        onDataAssetPackageIndexRead: (lease, _) =>
            _controllerDataAssetPackageIndexResult(
              head: lease.head,
              projectId: lease.projectId,
              projectRevision: lease.projectRevision,
            ),
        onInstalledDataAssetInspection: (lease, _, _, _) {
          lease.requiresReopenValue = true;
          throw StateError('injected installed DataAsset inspection failure');
        },
      );
      final coordinator = CurrentProjectCoordinator(
        openManagedRevision3: (_) async => managed,
      );
      addTearDown(() async {
        await coordinator.shutdown();
        coordinator.dispose();
      });
      final visible = await coordinator.openManagedRevision3(managed.root);
      final snapshot = await coordinator
          .readCurrentRevision3DataAssetPackageIndex(
            expectedRoot: visible.root.path,
            expectedProjectId: visible.projectId,
            expectedProjectRevision: visible.projectRevision,
            expectedHead: visible.head,
            gameRoot: r'C:\Games\Gothic Remake',
          );
      final candidate = snapshot.index.candidates.single;

      Future<AuthoringRevision3InstalledDataAssetInspectionResult> inspect() =>
          coordinator.inspectCurrentRevision3InstalledDataAsset(
            expectedRoot: visible.root.path,
            expectedProjectId: visible.projectId,
            expectedProjectRevision: visible.projectRevision,
            expectedHead: visible.head,
            gameRoot: r'C:\Games\Gothic Remake',
            expectedSnapshot: snapshot,
            candidate: candidate,
          );

      await expectLater(
        inspect(),
        throwsA(
          isA<Revision3InstalledDataAssetInspectionRequiresReopenException>(),
        ),
      );
      expect(managed.installedDataAssetInspectionCalls, 1);
      expect(
        (coordinator.state as ManagedRevision3CurrentProjectState)
            .requiresReopen,
        isTrue,
      );
      await expectLater(
        inspect(),
        throwsA(
          isA<Revision3InstalledDataAssetInspectionRequiresReopenException>(),
        ),
      );
      expect(managed.installedDataAssetInspectionCalls, 1);
    },
  );

  test(
    'Quest publication is exact-basis bound and refreshes R3 state',
    () async {
      final projectId = '15151515151515151515151515151515';
      late Revision3QuestDraftAuthoringInput receivedInput;
      final managed = _FakeManagedLease(
        root: Directory('managed-quest'),
        projectIdValue: projectId,
        projectRevision: 15,
        head: _head(15),
        onQuestPublish: (lease, gameRoot, input) {
          expect(gameRoot, r'C:\Games\Gothic Remake');
          receivedInput = input;
          lease.projectRevision = 16;
          lease.head = _head(16);
          return Revision3QuestDraftPublication(
            projectId: projectId,
            projectRevision: 16,
            questId: '25252525252525252525252525252525',
            scriptModuleId: '35353535353535353535353535353535',
          );
        },
      );
      final coordinator = CurrentProjectCoordinator(
        openManagedRevision3: (_) async => managed,
      );
      addTearDown(() async {
        await coordinator.shutdown();
        coordinator.dispose();
      });
      await coordinator.openManagedRevision3(managed.root);

      final published = await coordinator.createCurrentRevision3QuestDraft(
        expectedRoot: managed.root.path,
        expectedProjectId: projectId,
        expectedProjectRevision: 15,
        expectedHead: _head(15),
        gameRoot: r'C:\Games\Gothic Remake',
        input: _questInput(),
      );

      expect(published.projectRevision, 16);
      expect(receivedInput.title, 'Find Homer');
      expect(managed.questPublishCalls, 1);
      final state = coordinator.state as ManagedRevision3CurrentProjectState;
      expect(state.projectRevision, 16);
      expect(state.head.canonicalJson, _head(16).canonicalJson);
    },
  );

  test(
    'Quest publication rejects same-id/revision different-head wizard before touching the lease',
    () async {
      final managed = _FakeManagedLease(
        root: Directory('managed-stale-quest'),
        projectIdValue: '16161616161616161616161616161616',
        projectRevision: 16,
        head: _head(16),
      );
      final coordinator = CurrentProjectCoordinator(
        openManagedRevision3: (_) async => managed,
      );
      addTearDown(() async {
        await coordinator.shutdown();
        coordinator.dispose();
      });
      await coordinator.openManagedRevision3(managed.root);

      await expectLater(
        coordinator.createCurrentRevision3QuestDraft(
          expectedRoot: managed.root.path,
          expectedProjectId: managed.projectId,
          expectedProjectRevision: 16,
          expectedHead: _head(15),
          gameRoot: r'C:\Games\Gothic Remake',
          input: _questInput(),
        ),
        throwsA(isA<Revision3QuestDraftStaleCheckpointException>()),
      );
      expect(managed.questPublishCalls, 0);
      expect(
        (coordinator.state as ManagedRevision3CurrentProjectState)
            .projectRevision,
        16,
      );
    },
  );

  test(
    'poisoned Quest failure locks retries and refreshes requiresReopen',
    () async {
      final managed = _FakeManagedLease(
        root: Directory('managed-poisoned-quest'),
        projectIdValue: '17171717171717171717171717171717',
        projectRevision: 17,
        head: _head(17),
        onQuestPublish: (lease, _, _) {
          lease.requiresReopenValue = true;
          throw StateError('injected publication verification failure');
        },
      );
      final coordinator = CurrentProjectCoordinator(
        openManagedRevision3: (_) async => managed,
      );
      addTearDown(() async {
        await coordinator.shutdown();
        coordinator.dispose();
      });
      await coordinator.openManagedRevision3(managed.root);

      Future<void> publish() async {
        await coordinator.createCurrentRevision3QuestDraft(
          expectedRoot: managed.root.path,
          expectedProjectId: managed.projectId,
          expectedProjectRevision: 17,
          expectedHead: _head(17),
          gameRoot: r'C:\Games\Gothic Remake',
          input: _questInput(),
        );
      }

      await expectLater(
        publish(),
        throwsA(isA<Revision3QuestDraftRequiresReopenException>()),
      );
      expect(managed.questPublishCalls, 1);
      expect(
        (coordinator.state as ManagedRevision3CurrentProjectState)
            .requiresReopen,
        isTrue,
      );
      await expectLater(
        publish(),
        throwsA(isA<Revision3QuestDraftRequiresReopenException>()),
      );
      expect(managed.questPublishCalls, 1);
    },
  );

  test(
    'Quest outline edit is exact-checkpoint bound and needs no game root',
    () async {
      final fixture = Revision3QuestOutlineFixture();
      final index = fixture.contentIndex();
      final input = Revision3QuestOutlineEditInput.forQuest(
        index: index,
        quest: index.entityById(revision3QuestOutlineQuestId)!,
        displayName: 'Find Homer safely',
        title: 'Find Homer safely',
        objectiveTitles: const [
          'Inspect the old gate',
          'Ask Asghan about Homer',
          'Report to Diego',
        ],
      );
      final managed = _FakeManagedLease(
        root: Directory('managed-quest-outline'),
        projectIdValue: revision3QuestOutlineProjectId,
        projectRevision: 7,
        head: _head(7),
        onQuestOutlinePublish: (lease, received) {
          expect(received, same(input));
          lease.projectRevision = 8;
          lease.head = _head(8);
          return Revision3QuestOutlineEditPublication(
            projectId: revision3QuestOutlineProjectId,
            projectRevision: 8,
            questId: revision3QuestOutlineQuestId,
            moduleId: revision3QuestOutlineModuleId,
            questRevision: 5,
            moduleRevision: 6,
          );
        },
      );
      final coordinator = CurrentProjectCoordinator(
        openManagedRevision3: (_) async => managed,
      );
      addTearDown(() async {
        await coordinator.shutdown();
        coordinator.dispose();
      });
      await coordinator.openManagedRevision3(managed.root);

      final publication = await coordinator.editCurrentRevision3QuestOutline(
        expectedRoot: managed.root.path,
        expectedProjectId: revision3QuestOutlineProjectId,
        expectedProjectRevision: 7,
        expectedHead: _head(7),
        input: input,
      );

      expect(publication.projectRevision, 8);
      expect(managed.questOutlinePublishCalls, 1);
      expect(
        (coordinator.state as ManagedRevision3CurrentProjectState)
            .projectRevision,
        8,
      );
    },
  );

  test(
    'Quest outline stale/reopen guards make zero additional lease calls',
    () async {
      final fixture = Revision3QuestOutlineFixture();
      final index = fixture.contentIndex();
      final input = Revision3QuestOutlineEditInput.forQuest(
        index: index,
        quest: index.entityById(revision3QuestOutlineQuestId)!,
        displayName: 'Find Homer safely',
        title: 'Find Homer safely',
        objectiveTitles: const [
          'Inspect the old gate',
          'Ask Asghan about Homer',
          'Report to Diego',
        ],
      );
      final managed = _FakeManagedLease(
        root: Directory('managed-quest-outline-guard'),
        projectIdValue: revision3QuestOutlineProjectId,
        projectRevision: 7,
        head: _head(7),
        onQuestOutlinePublish: (lease, _) {
          lease.requiresReopenValue = true;
          throw StateError('injected outline verification failure');
        },
      );
      final coordinator = CurrentProjectCoordinator(
        openManagedRevision3: (_) async => managed,
      );
      addTearDown(() async {
        await coordinator.shutdown();
        coordinator.dispose();
      });
      await coordinator.openManagedRevision3(managed.root);

      await expectLater(
        coordinator.editCurrentRevision3QuestOutline(
          expectedRoot: managed.root.path,
          expectedProjectId: revision3QuestOutlineProjectId,
          expectedProjectRevision: 7,
          expectedHead: _head(6),
          input: input,
        ),
        throwsA(isA<Revision3QuestOutlineStaleCheckpointException>()),
      );
      expect(managed.questOutlinePublishCalls, 0);

      await expectLater(
        coordinator.editCurrentRevision3QuestOutline(
          expectedRoot: managed.root.path,
          expectedProjectId: revision3QuestOutlineProjectId,
          expectedProjectRevision: 7,
          expectedHead: _head(7),
          input: input,
        ),
        throwsA(isA<Revision3QuestOutlineRequiresReopenException>()),
      );
      expect(managed.questOutlinePublishCalls, 1);
      await expectLater(
        coordinator.editCurrentRevision3QuestOutline(
          expectedRoot: managed.root.path,
          expectedProjectId: revision3QuestOutlineProjectId,
          expectedProjectRevision: 7,
          expectedHead: _head(7),
          input: input,
        ),
        throwsA(isA<Revision3QuestOutlineRequiresReopenException>()),
      );
      expect(managed.questOutlinePublishCalls, 1);
    },
  );

  test(
    'Quest transitions seed and publication are exact-checkpoint guarded without a game root',
    () async {
      final fixture = Revision3QuestOutlineFixture();
      final seed = _questTransitionsSeed(fixture);
      final plan = await _questTransitionsPlan(fixture);
      final managed = _FakeManagedLease(
        root: Directory('managed-quest-transitions'),
        projectIdValue: revision3QuestOutlineProjectId,
        projectRevision: fixture.projectRevision,
        head: _head(fixture.projectRevision),
        onQuestTransitionsSeed:
            (lease, questId, questRevision, moduleId, moduleRevision) {
              expect(questId, revision3QuestOutlineQuestId);
              expect(questRevision, fixture.questRevision);
              expect(moduleId, revision3QuestOutlineModuleId);
              expect(moduleRevision, fixture.moduleRevision);
              return seed;
            },
        onQuestTransitionsPublish: (lease, received) {
          expect(received, same(plan));
          lease.projectRevision = fixture.projectRevision + 1;
          lease.head = _head(fixture.projectRevision + 1);
          return Revision3QuestTransitionsEditPublication(
            projectId: revision3QuestOutlineProjectId,
            projectRevision: fixture.projectRevision + 1,
            questId: revision3QuestOutlineQuestId,
            moduleId: revision3QuestOutlineModuleId,
            questRevision: fixture.questRevision + 1,
            moduleRevision: fixture.moduleRevision + 1,
            transitionPlanSeal: plan.transitionPlan.contentSeal,
          );
        },
      );
      final coordinator = CurrentProjectCoordinator(
        openManagedRevision3: (_) async => managed,
      );
      addTearDown(() async {
        await coordinator.shutdown();
        coordinator.dispose();
      });
      await coordinator.openManagedRevision3(managed.root);

      final loaded = await coordinator.readCurrentRevision3QuestTransitionsSeed(
        expectedRoot: managed.root.path,
        expectedProjectId: revision3QuestOutlineProjectId,
        expectedProjectRevision: fixture.projectRevision,
        expectedHead: _head(fixture.projectRevision),
        questId: revision3QuestOutlineQuestId,
        expectedQuestRevision: fixture.questRevision,
        expectedModuleId: revision3QuestOutlineModuleId,
        expectedModuleRevision: fixture.moduleRevision,
      );
      expect(loaded.transitionPlanSeal.sha256, seed.transitionPlanSeal.sha256);
      expect(managed.questTransitionsSeedCalls, 1);

      final publication = await coordinator
          .editCurrentRevision3QuestTransitions(
            expectedRoot: managed.root.path,
            expectedProjectId: revision3QuestOutlineProjectId,
            expectedProjectRevision: fixture.projectRevision,
            expectedHead: _head(fixture.projectRevision),
            plan: plan,
          );
      expect(publication.projectRevision, fixture.projectRevision + 1);
      expect(
        publication.transitionPlanSeal.sha256,
        plan.transitionPlan.contentSeal.sha256,
      );
      expect(managed.questTransitionsPublishCalls, 1);
      expect(
        (coordinator.state as ManagedRevision3CurrentProjectState)
            .projectRevision,
        fixture.projectRevision + 1,
      );
    },
  );

  test(
    'Quest transitions stale and reopen guards make no extra lease calls',
    () async {
      final fixture = Revision3QuestOutlineFixture();
      final plan = await _questTransitionsPlan(fixture);
      final managed = _FakeManagedLease(
        root: Directory('managed-quest-transitions-guards'),
        projectIdValue: revision3QuestOutlineProjectId,
        projectRevision: fixture.projectRevision,
        head: _head(fixture.projectRevision),
        onQuestTransitionsPublish: (lease, _) {
          lease.requiresReopenValue = true;
          throw StateError('injected transitions verification failure');
        },
      );
      final coordinator = CurrentProjectCoordinator(
        openManagedRevision3: (_) async => managed,
      );
      addTearDown(() async {
        await coordinator.shutdown();
        coordinator.dispose();
      });
      await coordinator.openManagedRevision3(managed.root);

      await expectLater(
        coordinator.editCurrentRevision3QuestTransitions(
          expectedRoot: managed.root.path,
          expectedProjectId: revision3QuestOutlineProjectId,
          expectedProjectRevision: fixture.projectRevision,
          expectedHead: _head(fixture.projectRevision - 1),
          plan: plan,
        ),
        throwsA(isA<Revision3QuestTransitionsStaleCheckpointException>()),
      );
      expect(managed.questTransitionsPublishCalls, 0);

      await expectLater(
        coordinator.editCurrentRevision3QuestTransitions(
          expectedRoot: managed.root.path,
          expectedProjectId: revision3QuestOutlineProjectId,
          expectedProjectRevision: fixture.projectRevision,
          expectedHead: _head(fixture.projectRevision),
          plan: plan,
        ),
        throwsA(isA<Revision3QuestTransitionsRequiresReopenException>()),
      );
      expect(managed.questTransitionsPublishCalls, 1);
      await expectLater(
        coordinator.editCurrentRevision3QuestTransitions(
          expectedRoot: managed.root.path,
          expectedProjectId: revision3QuestOutlineProjectId,
          expectedProjectRevision: fixture.projectRevision,
          expectedHead: _head(fixture.projectRevision),
          plan: plan,
        ),
        throwsA(isA<Revision3QuestTransitionsRequiresReopenException>()),
      );
      expect(managed.questTransitionsPublishCalls, 1);
    },
  );

  test(
    'Quest context seed and publication are exact-checkpoint guarded',
    () async {
      final fixture = Revision3QuestOutlineFixture();
      final seed = _questContextSeed(fixture);
      final plan = await _questContextPlan(fixture);
      final managed = _FakeManagedLease(
        root: Directory('managed-quest-context'),
        projectIdValue: revision3QuestOutlineProjectId,
        projectRevision: fixture.projectRevision,
        head: _head(fixture.projectRevision),
        onQuestContextSeed:
            (
              lease,
              questId,
              questRevision,
              moduleId,
              moduleRevision,
              parentRuntime,
              giverRuntime,
            ) {
              expect(questId, revision3QuestOutlineQuestId);
              expect(questRevision, fixture.questRevision);
              expect(moduleId, revision3QuestOutlineModuleId);
              expect(moduleRevision, fixture.moduleRevision);
              expect(parentRuntime, 'UQuest_SwampCamp_SCChapter2');
              expect(giverRuntime, 'OM_GRD_Asghan_263');
              return seed;
            },
        onQuestContextPublish: (lease, gameRoot, received) {
          expect(gameRoot, r'C:\Games\Gothic Remake');
          expect(received, same(plan));
          lease.projectRevision = fixture.projectRevision + 1;
          lease.head = _head(fixture.projectRevision + 1);
          return Revision3QuestContextEditPublication(
            projectId: revision3QuestOutlineProjectId,
            projectRevision: fixture.projectRevision + 1,
            questId: revision3QuestOutlineQuestId,
            moduleId: revision3QuestOutlineModuleId,
            questRevision: fixture.questRevision + 1,
            moduleRevision: fixture.moduleRevision + 1,
          );
        },
      );
      final coordinator = CurrentProjectCoordinator(
        openManagedRevision3: (_) async => managed,
      );
      addTearDown(() async {
        await coordinator.shutdown();
        coordinator.dispose();
      });
      await coordinator.openManagedRevision3(managed.root);

      final loaded = await coordinator.readCurrentRevision3QuestContextSeed(
        expectedRoot: managed.root.path,
        expectedProjectId: revision3QuestOutlineProjectId,
        expectedProjectRevision: fixture.projectRevision,
        expectedHead: _head(fixture.projectRevision),
        questId: revision3QuestOutlineQuestId,
        expectedQuestRevision: fixture.questRevision,
        expectedModuleId: revision3QuestOutlineModuleId,
        expectedModuleRevision: fixture.moduleRevision,
        expectedParentRuntimeClass: 'UQuest_SwampCamp_SCChapter2',
        expectedGiverRuntimeUniqueName: 'OM_GRD_Asghan_263',
      );
      expect(loaded.description, seed.description);
      expect(managed.questContextSeedCalls, 1);

      final publication = await coordinator.editCurrentRevision3QuestContext(
        expectedRoot: managed.root.path,
        expectedProjectId: revision3QuestOutlineProjectId,
        expectedProjectRevision: fixture.projectRevision,
        expectedHead: _head(fixture.projectRevision),
        gameRoot: r'C:\Games\Gothic Remake',
        plan: plan,
      );
      expect(publication.projectRevision, fixture.projectRevision + 1);
      expect(managed.questContextPublishCalls, 1);
      expect(
        (coordinator.state as ManagedRevision3CurrentProjectState)
            .projectRevision,
        fixture.projectRevision + 1,
      );
    },
  );

  test(
    'Quest context stale and reopen guards make no extra lease calls',
    () async {
      final fixture = Revision3QuestOutlineFixture();
      final plan = await _questContextPlan(fixture);
      final managed = _FakeManagedLease(
        root: Directory('managed-quest-context-guards'),
        projectIdValue: revision3QuestOutlineProjectId,
        projectRevision: fixture.projectRevision,
        head: _head(fixture.projectRevision),
        onQuestContextPublish: (lease, _, _) {
          lease.requiresReopenValue = true;
          throw StateError('injected context verification failure');
        },
      );
      final coordinator = CurrentProjectCoordinator(
        openManagedRevision3: (_) async => managed,
      );
      addTearDown(() async {
        await coordinator.shutdown();
        coordinator.dispose();
      });
      await coordinator.openManagedRevision3(managed.root);

      await expectLater(
        coordinator.editCurrentRevision3QuestContext(
          expectedRoot: managed.root.path,
          expectedProjectId: revision3QuestOutlineProjectId,
          expectedProjectRevision: fixture.projectRevision,
          expectedHead: _head(fixture.projectRevision - 1),
          gameRoot: r'C:\Games\Gothic Remake',
          plan: plan,
        ),
        throwsA(isA<Revision3QuestContextStaleCheckpointException>()),
      );
      expect(managed.questContextPublishCalls, 0);

      await expectLater(
        coordinator.editCurrentRevision3QuestContext(
          expectedRoot: managed.root.path,
          expectedProjectId: revision3QuestOutlineProjectId,
          expectedProjectRevision: fixture.projectRevision,
          expectedHead: _head(fixture.projectRevision),
          gameRoot: r'C:\Games\Gothic Remake',
          plan: plan,
        ),
        throwsA(isA<Revision3QuestContextRequiresReopenException>()),
      );
      expect(managed.questContextPublishCalls, 1);
      await expectLater(
        coordinator.editCurrentRevision3QuestContext(
          expectedRoot: managed.root.path,
          expectedProjectId: revision3QuestOutlineProjectId,
          expectedProjectRevision: fixture.projectRevision,
          expectedHead: _head(fixture.projectRevision),
          gameRoot: r'C:\Games\Gothic Remake',
          plan: plan,
        ),
        throwsA(isA<Revision3QuestContextRequiresReopenException>()),
      );
      expect(managed.questContextPublishCalls, 1);
    },
  );

  test(
    'NPC publication is exact-checkpoint bound and refreshes R3 state',
    () async {
      const projectId = '18181818181818181818181818181818';
      late Revision3NpcDraftAuthoringInput receivedInput;
      final managed = _FakeManagedLease(
        root: Directory('managed-npc'),
        projectIdValue: projectId,
        projectRevision: 18,
        head: _head(18),
        onNpcPublish: (lease, gameRoot, input) {
          expect(gameRoot, r'C:\Games\Gothic Remake');
          receivedInput = input;
          lease.projectRevision = 19;
          lease.head = _head(19);
          return Revision3NpcDraftPublication(
            projectId: projectId,
            projectRevision: 19,
            head: lease.head,
            npcId: '28282828282828282828282828282828',
            scriptModuleId: '38383838383838383838383838383838',
          );
        },
      );
      final coordinator = CurrentProjectCoordinator(
        openManagedRevision3: (_) async => managed,
      );
      addTearDown(() async {
        await coordinator.shutdown();
        coordinator.dispose();
      });
      await coordinator.openManagedRevision3(managed.root);

      final published = await coordinator.createCurrentRevision3NpcDraft(
        expectedRoot: managed.root.path,
        expectedHead: _head(18),
        expectedProjectId: projectId,
        expectedProjectRevision: 18,
        gameRoot: r'C:\Games\Gothic Remake',
        input: _npcInput(),
      );

      expect(published.projectRevision, 19);
      expect(published.head.canonicalJson, _head(19).canonicalJson);
      expect(receivedInput.displayName, 'North Gate Guard');
      expect(receivedInput.parentCatalogId, 'g1r:npc:om_grd_asghan_263');
      expect(managed.npcPublishCalls, 1);
      final state = coordinator.state as ManagedRevision3CurrentProjectState;
      expect(state.projectRevision, 19);
      expect(state.head.canonicalJson, _head(19).canonicalJson);
    },
  );

  test('NPC publication rejects a reused pre-publication head', () async {
    const projectId = '18181818181818181818181818181818';
    final managed = _FakeManagedLease(
      root: Directory('managed-npc-reused-head'),
      projectIdValue: projectId,
      projectRevision: 18,
      head: _head(18),
      onNpcPublish: (lease, _, _) {
        lease.projectRevision = 19;
        return Revision3NpcDraftPublication(
          projectId: projectId,
          projectRevision: 19,
          head: lease.head,
          npcId: '28282828282828282828282828282828',
          scriptModuleId: '38383838383838383838383838383838',
        );
      },
    );
    final coordinator = CurrentProjectCoordinator(
      openManagedRevision3: (_) async => managed,
    );
    addTearDown(() async {
      await coordinator.shutdown();
      coordinator.dispose();
    });
    await coordinator.openManagedRevision3(managed.root);

    await expectLater(
      coordinator.createCurrentRevision3NpcDraft(
        expectedRoot: managed.root.path,
        expectedHead: _head(18),
        expectedProjectId: projectId,
        expectedProjectRevision: 18,
        gameRoot: r'C:\Games\Gothic Remake',
        input: _npcInput(),
      ),
      throwsA(isA<CurrentProjectCoordinatorException>()),
    );
    expect(managed.npcPublishCalls, 1);
  });

  test(
    'NPC publication rejects divergent root or head before lease access',
    () async {
      final managed = _FakeManagedLease(
        root: Directory('managed-npc-stale'),
        projectIdValue: '19191919191919191919191919191919',
        projectRevision: 19,
        head: _head(19),
      );
      final coordinator = CurrentProjectCoordinator(
        openManagedRevision3: (_) async => managed,
      );
      addTearDown(() async {
        await coordinator.shutdown();
        coordinator.dispose();
      });
      await coordinator.openManagedRevision3(managed.root);

      Future<void> publish({
        required String root,
        required AuthoringWorkingHead head,
      }) async {
        await coordinator.createCurrentRevision3NpcDraft(
          expectedRoot: root,
          expectedHead: head,
          expectedProjectId: managed.projectId,
          expectedProjectRevision: 19,
          gameRoot: r'C:\Games\Gothic Remake',
          input: _npcInput(),
        );
      }

      await expectLater(
        publish(
          root: Directory('divergent-managed-clone').path,
          head: _head(19),
        ),
        throwsA(isA<Revision3NpcDraftStaleCheckpointException>()),
      );
      await expectLater(
        publish(root: managed.root.path, head: _head(1919)),
        throwsA(isA<Revision3NpcDraftStaleCheckpointException>()),
      );
      expect(managed.npcPublishCalls, 0);
    },
  );

  test(
    'NPC publication rejects an empty game root before lease access',
    () async {
      final managed = _FakeManagedLease(
        root: Directory('managed-npc-no-game'),
        projectIdValue: '20202020202020202020202020202020',
        projectRevision: 20,
        head: _head(20),
      );
      final coordinator = CurrentProjectCoordinator(
        openManagedRevision3: (_) async => managed,
      );
      addTearDown(() async {
        await coordinator.shutdown();
        coordinator.dispose();
      });
      await coordinator.openManagedRevision3(managed.root);

      await expectLater(
        coordinator.createCurrentRevision3NpcDraft(
          expectedRoot: managed.root.path,
          expectedHead: _head(20),
          expectedProjectId: managed.projectId,
          expectedProjectRevision: 20,
          gameRoot: '',
          input: _npcInput(),
        ),
        throwsA(isA<CurrentProjectOperationUnsupportedException>()),
      );
      expect(managed.npcPublishCalls, 0);
    },
  );

  test(
    'poisoned NPC failure locks retries and refreshes requiresReopen',
    () async {
      final managed = _FakeManagedLease(
        root: Directory('managed-poisoned-npc'),
        projectIdValue: '21212121212121212121212121212121',
        projectRevision: 21,
        head: _head(21),
        onNpcPublish: (lease, _, _) {
          lease.requiresReopenValue = true;
          throw StateError('injected NPC publication verification failure');
        },
      );
      final coordinator = CurrentProjectCoordinator(
        openManagedRevision3: (_) async => managed,
      );
      addTearDown(() async {
        await coordinator.shutdown();
        coordinator.dispose();
      });
      await coordinator.openManagedRevision3(managed.root);

      Future<void> publish() async {
        await coordinator.createCurrentRevision3NpcDraft(
          expectedRoot: managed.root.path,
          expectedHead: _head(21),
          expectedProjectId: managed.projectId,
          expectedProjectRevision: 21,
          gameRoot: r'C:\Games\Gothic Remake',
          input: _npcInput(),
        );
      }

      await expectLater(
        publish(),
        throwsA(isA<Revision3NpcDraftRequiresReopenException>()),
      );
      expect(managed.npcPublishCalls, 1);
      expect(
        (coordinator.state as ManagedRevision3CurrentProjectState)
            .requiresReopen,
        isTrue,
      );
      await expectLater(
        publish(),
        throwsA(isA<Revision3NpcDraftRequiresReopenException>()),
      );
      expect(managed.npcPublishCalls, 1);
    },
  );

  test(
    'NPC profile seed and exact publication refresh the managed checkpoint',
    () async {
      final profile = Revision3NpcProfileTestFixture.create();
      final plan = await _npcProfileEditPlan(profile);
      final managed = _FakeNpcProfileManagedLease(
        root: Directory('managed-npc-profile'),
        projectIdValue: profile.seed.projectId,
        projectRevision: profile.seed.projectRevision,
        head: profile.head,
        seed: profile.seed,
        onPublish: (lease, gameRoot, received) {
          expect(gameRoot, r'C:\Games\Gothic Remake');
          expect(received, same(plan));
          lease.projectRevision = plan.projectRevision + 1;
          lease.head = _head(lease.projectRevision);
          return _npcProfilePublication(plan);
        },
      );
      final coordinator = CurrentProjectCoordinator(
        openManagedRevision3: (_) async => managed,
      );
      addTearDown(() async {
        await coordinator.shutdown();
        coordinator.dispose();
      });
      await coordinator.openManagedRevision3(managed.root);

      final seed = await coordinator.readCurrentRevision3NpcProfileEditSeed(
        expectedRoot: managed.root.path,
        expectedProjectId: profile.seed.projectId,
        expectedProjectRevision: profile.seed.projectRevision,
        expectedHead: profile.head,
        npcId: profile.seed.npcId,
        expectedNpcRevision: profile.seed.npcRevision,
        expectedScriptModuleId: profile.seed.scriptModuleId,
        expectedScriptModuleRevision: profile.seed.scriptModuleRevision,
        expectedUniqueName: profile.seed.uniqueName,
        expectedModuleNamespace: profile.seed.moduleNamespace,
        expectedParentCharacterDefinition:
            profile.seed.parentCharacterDefinition.runtimeClass,
        expectedParentAiAgentConfig:
            profile.seed.parentAiAgentConfig.runtimeClass,
        expectedParentSpawnDefinition:
            profile.seed.parentSpawnDefinition.runtimeClass,
      );
      expect(seed, same(profile.seed));
      expect(managed.seedCalls, 1);

      final publication = await coordinator.editCurrentRevision3NpcProfile(
        expectedRoot: managed.root.path,
        expectedProjectId: profile.seed.projectId,
        expectedProjectRevision: profile.seed.projectRevision,
        expectedHead: profile.head,
        gameRoot: r'C:\Games\Gothic Remake',
        plan: plan,
      );

      expect(publication.displayName, plan.displayName);
      expect(managed.publishCalls, 1);
      expect(managed.latchCalls, 0);
      final state = coordinator.state as ManagedRevision3CurrentProjectState;
      expect(state.projectRevision, profile.seed.projectRevision + 1);
      expect(state.head.canonicalJson, _head(2).canonicalJson);
    },
  );

  test(
    'NPC profile correctable failure is stale without retry or poison',
    () async {
      final profile = Revision3NpcProfileTestFixture.create();
      final plan = await _npcProfileEditPlan(profile);
      final managed = _FakeNpcProfileManagedLease(
        root: Directory('managed-npc-profile-stale'),
        projectIdValue: profile.seed.projectId,
        projectRevision: profile.seed.projectRevision,
        head: profile.head,
        seed: profile.seed,
        onPublish: (_, _, _) => throw const ModFfiException(
          command: 'authoring_store_prepare_revision3_npc_profile_edit_v1',
          code: 'AUTHORING_REVISION3_NPC_PROFILE_CATALOG_CONFLICT',
          message: 'fake fresh catalog conflict',
        ),
      );
      final coordinator = CurrentProjectCoordinator(
        openManagedRevision3: (_) async => managed,
      );
      addTearDown(() async {
        await coordinator.shutdown();
        coordinator.dispose();
      });
      await coordinator.openManagedRevision3(managed.root);

      await expectLater(
        coordinator.editCurrentRevision3NpcProfile(
          expectedRoot: managed.root.path,
          expectedProjectId: profile.seed.projectId,
          expectedProjectRevision: profile.seed.projectRevision,
          expectedHead: profile.head,
          gameRoot: r'C:\Games\Gothic Remake',
          plan: plan,
        ),
        throwsA(isA<Revision3NpcProfileEditStaleCheckpointException>()),
      );
      expect(managed.publishCalls, 1);
      expect(managed.latchCalls, 0);
      expect(managed.requiresReopen, isFalse);
    },
  );

  test(
    'NPC profile receipt mismatch latches reopen and blocks retry',
    () async {
      final profile = Revision3NpcProfileTestFixture.create();
      final plan = await _npcProfileEditPlan(profile);
      final managed = _FakeNpcProfileManagedLease(
        root: Directory('managed-npc-profile-poison'),
        projectIdValue: profile.seed.projectId,
        projectRevision: profile.seed.projectRevision,
        head: profile.head,
        seed: profile.seed,
        onPublish: (lease, _, plan) {
          lease.projectRevision = plan.projectRevision + 1;
          lease.head = _head(lease.projectRevision);
          final publication = _npcProfilePublication(plan);
          return Revision3NpcProfileEditPublication(
            projectId: publication.projectId,
            projectRevision: publication.projectRevision,
            npcId: publication.npcId,
            npcRevision: publication.npcRevision,
            scriptModuleId: publication.scriptModuleId,
            scriptModuleRevision: publication.scriptModuleRevision,
            displayName: 'Forged receipt name',
            previousParentCatalogId: publication.previousParentCatalogId,
            parentCatalogId: publication.parentCatalogId,
            nameChanged: publication.nameChanged,
            archetypeChanged: publication.archetypeChanged,
            moduleRegenerated: publication.moduleRegenerated,
          );
        },
      );
      final coordinator = CurrentProjectCoordinator(
        openManagedRevision3: (_) async => managed,
      );
      addTearDown(() async {
        await coordinator.shutdown();
        coordinator.dispose();
      });
      await coordinator.openManagedRevision3(managed.root);

      Future<void> publish() async {
        await coordinator.editCurrentRevision3NpcProfile(
          expectedRoot: managed.root.path,
          expectedProjectId: profile.seed.projectId,
          expectedProjectRevision: profile.seed.projectRevision,
          expectedHead: profile.head,
          gameRoot: r'C:\Games\Gothic Remake',
          plan: plan,
        );
      }

      await expectLater(
        publish(),
        throwsA(isA<Revision3NpcProfileEditRequiresReopenException>()),
      );
      expect(managed.publishCalls, 1);
      expect(managed.latchCalls, 1);
      expect(managed.requiresReopen, isTrue);
      await expectLater(
        publish(),
        throwsA(isA<Revision3NpcProfileEditRequiresReopenException>()),
      );
      expect(managed.publishCalls, 1);
    },
  );

  test(
    'DialogLine publication rejects stale checkpoints and refreshes managed state without a game root',
    () async {
      const projectId = '22222222222222222222222222222222';
      const projectRevision = 22;
      final plan = _dialogLinePlan(
        projectId: projectId,
        projectRevision: projectRevision,
      );
      final managed = _FakeManagedLease(
        root: Directory('managed-dialog-line'),
        projectIdValue: projectId,
        projectRevision: projectRevision,
        head: _head(projectRevision),
        onDialogLinePublish: (lease, received) {
          expect(received, same(plan));
          lease.projectRevision = projectRevision + 1;
          lease.head = _head(projectRevision + 1);
          return Revision3DialogLineEntryPublication(
            projectId: projectId,
            projectRevision: projectRevision + 1,
            lineId: received.lineId,
            localizationId: received.localization.localizationId,
            localizationAction:
                AuthoringRevision3DialogLocalizationAction.created,
            voiceSlotId: received.voiceSlot?.slotId,
            locale: received.locale,
          );
        },
      );
      final coordinator = CurrentProjectCoordinator(
        openManagedRevision3: (_) async => managed,
      );
      addTearDown(() async {
        await coordinator.shutdown();
        coordinator.dispose();
      });
      await coordinator.openManagedRevision3(managed.root);

      for (final stale
          in <
            ({
              String root,
              String projectId,
              int revision,
              AuthoringWorkingHead head,
            })
          >[
            (
              root: Directory('another-dialog-root').path,
              projectId: projectId,
              revision: projectRevision,
              head: _head(projectRevision),
            ),
            (
              root: managed.root.path,
              projectId: '23232323232323232323232323232323',
              revision: projectRevision,
              head: _head(projectRevision),
            ),
            (
              root: managed.root.path,
              projectId: projectId,
              revision: projectRevision - 1,
              head: _head(projectRevision),
            ),
            (
              root: managed.root.path,
              projectId: projectId,
              revision: projectRevision,
              head: _head(projectRevision - 1),
            ),
          ]) {
        await expectLater(
          coordinator.createCurrentRevision3DialogLine(
            expectedRoot: stale.root,
            expectedProjectId: stale.projectId,
            expectedProjectRevision: stale.revision,
            expectedHead: stale.head,
            plan: plan,
          ),
          throwsA(isA<Revision3DialogLineEntryStaleCheckpointException>()),
        );
      }
      expect(managed.dialogLinePublishCalls, 0);

      final publication = await coordinator.createCurrentRevision3DialogLine(
        expectedRoot: managed.root.path,
        expectedProjectId: projectId,
        expectedProjectRevision: projectRevision,
        expectedHead: _head(projectRevision),
        plan: plan,
      );
      expect(publication.lineId, plan.lineId);
      expect(publication.localizationId, plan.localization.localizationId);
      expect(publication.voiceSlotId, plan.voiceSlot?.slotId);
      expect(managed.dialogLinePublishCalls, 1);
      final state = coordinator.state as ManagedRevision3CurrentProjectState;
      expect(state.projectRevision, projectRevision + 1);
      expect(
        state.head.canonicalJson,
        _head(projectRevision + 1).canonicalJson,
      );
    },
  );

  test(
    'Voice publication is exact-checkpoint bound and refreshes R3 state',
    () async {
      final plan = _voicePlan();
      final managed = _FakeManagedLease(
        root: Directory('managed-voice'),
        projectIdValue: revision3VoiceContentProjectId,
        projectRevision: 7,
        head: _head(7),
        onVoicePublish: (lease, gameRoot, received) {
          expect(gameRoot, r'C:\Games\Gothic Remake');
          expect(received, same(plan));
          lease.projectRevision = 8;
          lease.head = _head(8);
          return Revision3VoiceTakePublication(
            projectId: revision3VoiceContentProjectId,
            projectRevision: 8,
            lineId: received.lineId,
            slotId: received.slotId,
            takeId: received.takeId,
            slotCreated: received.expectsSlotCreated,
            selected: received.selectTake,
          );
        },
      );
      final coordinator = CurrentProjectCoordinator(
        openManagedRevision3: (_) async => managed,
      );
      addTearDown(() async {
        await coordinator.shutdown();
        coordinator.dispose();
      });
      await coordinator.openManagedRevision3(managed.root);

      await expectLater(
        coordinator.addCurrentRevision3VoiceTake(
          expectedRoot: managed.root.path,
          expectedProjectId: managed.projectId,
          expectedProjectRevision: 7,
          expectedHead: _head(6),
          gameRoot: r'C:\Games\Gothic Remake',
          plan: plan,
        ),
        throwsA(isA<Revision3VoiceTakeStaleCheckpointException>()),
      );
      expect(managed.voicePublishCalls, 0);

      await expectLater(
        coordinator.addCurrentRevision3VoiceTake(
          expectedRoot: managed.root.path,
          expectedProjectId: managed.projectId,
          expectedProjectRevision: 7,
          expectedHead: _head(7),
          gameRoot: '',
          plan: plan,
        ),
        throwsA(isA<CurrentProjectOperationUnsupportedException>()),
      );
      expect(managed.voicePublishCalls, 0);

      final publication = await coordinator.addCurrentRevision3VoiceTake(
        expectedRoot: managed.root.path,
        expectedProjectId: managed.projectId,
        expectedProjectRevision: 7,
        expectedHead: _head(7),
        gameRoot: r'C:\Games\Gothic Remake',
        plan: plan,
      );
      expect(publication.takeId, plan.takeId);
      expect(managed.voicePublishCalls, 1);
      final state = coordinator.state as ManagedRevision3CurrentProjectState;
      expect(state.projectRevision, 8);
      expect(state.head.canonicalJson, _head(8).canonicalJson);
    },
  );

  test(
    'Voice media QA is exact-checkpoint bound and leaves state unchanged',
    () async {
      final plan = revision3VoicePreviewPlan();
      final managed = _FakeVoiceTakeMediaQaManagedLease(
        root: Directory('managed-voice-media-qa'),
        projectIdValue: revision3VoiceContentProjectId,
        projectRevision: 7,
        head: _head(7),
      );
      final coordinator = CurrentProjectCoordinator(
        openManagedRevision3: (_) async => managed,
      );
      addTearDown(() async {
        await coordinator.shutdown();
        coordinator.dispose();
      });
      final visible = await coordinator.openManagedRevision3(managed.root);

      await expectLater(
        coordinator.inspectCurrentRevision3VoiceTakeMediaQa(
          expectedRoot: visible.root.path,
          expectedProjectId: visible.projectId,
          expectedProjectRevision: visible.projectRevision,
          expectedHead: _head(6),
          plan: plan,
        ),
        throwsA(isA<Revision3VoiceTakeMediaQaStaleCheckpointException>()),
      );
      expect(managed.inspectCalls, 0);

      final result = await coordinator.inspectCurrentRevision3VoiceTakeMediaQa(
        expectedRoot: visible.root.path,
        expectedProjectId: visible.projectId,
        expectedProjectRevision: visible.projectRevision,
        expectedHead: visible.head,
        plan: plan,
      );
      expect(managed.inspectCalls, 1);
      expect(result.basisHead.canonicalJson, visible.head.canonicalJson);
      expect(result.projectRevision, 7);
      expect(result.lineId, plan.lineId);
      expect(result.locale, plan.locale);
      expect(result.takeId, plan.takeId);
      final state = coordinator.state as ManagedRevision3CurrentProjectState;
      expect(state.projectRevision, 7);
      expect(state.head.canonicalJson, visible.head.canonicalJson);
      expect(state.requiresReopen, isFalse);
    },
  );

  test('Voice media QA maps only graph-leaf conflicts to stale', () async {
    final managed = _FakeVoiceTakeMediaQaManagedLease(
      root: Directory('managed-voice-media-qa-stale'),
      projectIdValue: revision3VoiceContentProjectId,
      projectRevision: 7,
      head: _head(7),
    );
    final coordinator = CurrentProjectCoordinator(
      openManagedRevision3: (_) async => managed,
    );
    addTearDown(() async {
      await coordinator.shutdown();
      coordinator.dispose();
    });
    final visible = await coordinator.openManagedRevision3(managed.root);

    for (final code in <String>[
      'AUTHORING_REVISION3_VOICE_MEDIA_LINE_CONFLICT',
      'AUTHORING_REVISION3_VOICE_MEDIA_LOCALIZATION_CONFLICT',
      'AUTHORING_REVISION3_VOICE_MEDIA_SLOT_CONFLICT',
      'AUTHORING_REVISION3_VOICE_MEDIA_TAKE_CONFLICT',
      'AUTHORING_REVISION3_VOICE_MEDIA_ASSET_CONFLICT',
    ]) {
      managed.nextError = ModFfiException(
        command: 'authoring_store_inspect_revision3_voice_take_media_v1',
        code: code,
        message: 'fake exact Voice media graph drift',
      );
      await expectLater(
        coordinator.inspectCurrentRevision3VoiceTakeMediaQa(
          expectedRoot: visible.root.path,
          expectedProjectId: visible.projectId,
          expectedProjectRevision: visible.projectRevision,
          expectedHead: visible.head,
          plan: revision3VoicePreviewPlan(),
        ),
        throwsA(isA<Revision3VoiceTakeMediaQaStaleCheckpointException>()),
        reason: code,
      );
      expect(managed.requiresReopen, isFalse, reason: code);
      expect(managed.relatchCalls, 0, reason: code);
    }
  });

  test('Voice media QA uncertainty maps to reopen and locks retry', () async {
    final managed = _FakeVoiceTakeMediaQaManagedLease(
      root: Directory('managed-voice-media-qa-poison'),
      projectIdValue: revision3VoiceContentProjectId,
      projectRevision: 7,
      head: _head(7),
      nextError: const ModFfiException(
        command: 'authoring_store_inspect_revision3_voice_take_media_v1',
        code: 'AUTHORING_REVISION3_VOICE_MEDIA_STORE_INVARIANT',
        message: 'fake Voice media Store uncertainty',
      ),
    );
    final coordinator = CurrentProjectCoordinator(
      openManagedRevision3: (_) async => managed,
    );
    addTearDown(() async {
      await coordinator.shutdown();
      coordinator.dispose();
    });
    final visible = await coordinator.openManagedRevision3(managed.root);

    Future<void> inspect() async {
      await coordinator.inspectCurrentRevision3VoiceTakeMediaQa(
        expectedRoot: visible.root.path,
        expectedProjectId: visible.projectId,
        expectedProjectRevision: visible.projectRevision,
        expectedHead: visible.head,
        plan: revision3VoicePreviewPlan(),
      );
    }

    await expectLater(
      inspect(),
      throwsA(isA<Revision3VoiceTakeMediaQaRequiresReopenException>()),
    );
    expect(managed.inspectCalls, 1);
    expect(managed.relatchCalls, 1);
    expect(managed.requiresReopen, isTrue);
    await expectLater(
      inspect(),
      throwsA(isA<Revision3VoiceTakeMediaQaRequiresReopenException>()),
    );
    expect(managed.inspectCalls, 1);
  });

  test('Voice media QA receipt mismatch relatches the lease', () async {
    final managed = _FakeVoiceTakeMediaQaManagedLease(
      root: Directory('managed-voice-media-qa-mismatch'),
      projectIdValue: revision3VoiceContentProjectId,
      projectRevision: 7,
      head: _head(7),
      receiptMismatch: true,
    );
    final coordinator = CurrentProjectCoordinator(
      openManagedRevision3: (_) async => managed,
    );
    addTearDown(() async {
      await coordinator.shutdown();
      coordinator.dispose();
    });
    final visible = await coordinator.openManagedRevision3(managed.root);

    await expectLater(
      coordinator.inspectCurrentRevision3VoiceTakeMediaQa(
        expectedRoot: visible.root.path,
        expectedProjectId: visible.projectId,
        expectedProjectRevision: visible.projectRevision,
        expectedHead: visible.head,
        plan: revision3VoicePreviewPlan(),
      ),
      throwsA(isA<Revision3VoiceTakeMediaQaRequiresReopenException>()),
    );
    expect(managed.inspectCalls, 1);
    expect(managed.relatchCalls, 1);
    expect(managed.requiresReopen, isTrue);
  });

  test(
    'Voice media QA remains an explicit optional lease capability',
    () async {
      final managed = _FakeManagedLease(
        root: Directory('managed-voice-media-qa-unsupported'),
        projectIdValue: revision3VoiceContentProjectId,
        projectRevision: 7,
        head: _head(7),
      );
      final coordinator = CurrentProjectCoordinator(
        openManagedRevision3: (_) async => managed,
      );
      addTearDown(() async {
        await coordinator.shutdown();
        coordinator.dispose();
      });
      final visible = await coordinator.openManagedRevision3(managed.root);

      await expectLater(
        coordinator.inspectCurrentRevision3VoiceTakeMediaQa(
          expectedRoot: visible.root.path,
          expectedProjectId: visible.projectId,
          expectedProjectRevision: visible.projectRevision,
          expectedHead: visible.head,
          plan: revision3VoicePreviewPlan(),
        ),
        throwsA(isA<CurrentProjectOperationUnsupportedException>()),
      );
    },
  );

  test(
    'Voice preview is exact-checkpoint bound and leaves current state unchanged',
    () async {
      final plan = revision3VoicePreviewPlan();
      final managed = _FakeVoiceTakePreviewManagedLease(
        root: Directory('managed-voice-preview'),
        projectIdValue: revision3VoiceContentProjectId,
        projectRevision: 7,
        head: _head(7),
      );
      final coordinator = CurrentProjectCoordinator(
        openManagedRevision3: (_) async => managed,
      );
      addTearDown(() async {
        await coordinator.shutdown();
        coordinator.dispose();
      });
      final visible = await coordinator.openManagedRevision3(managed.root);

      await expectLater(
        coordinator.materializeCurrentRevision3VoiceTakePreview(
          expectedRoot: visible.root.path,
          expectedProjectId: visible.projectId,
          expectedProjectRevision: visible.projectRevision,
          expectedHead: _head(6),
          plan: plan,
        ),
        throwsA(isA<Revision3VoiceTakePreviewStaleCheckpointException>()),
      );
      expect(managed.materializeCalls, 0);

      final capability = await coordinator
          .materializeCurrentRevision3VoiceTakePreview(
            expectedRoot: visible.root.path,
            expectedProjectId: visible.projectId,
            expectedProjectRevision: visible.projectRevision,
            expectedHead: visible.head,
            plan: plan,
          );
      expect(managed.materializeCalls, 1);
      expect(capability.projectRevision, 7);
      expect(
        await File(capability.path).readAsBytes(),
        revision3VoicePreviewBytes,
      );
      final state = coordinator.state as ManagedRevision3CurrentProjectState;
      expect(state.projectRevision, 7);
      expect(state.head.canonicalJson, visible.head.canonicalJson);
      expect(state.requiresReopen, isFalse);
      await capability.close();
    },
  );

  test('Voice preview maps only exact graph-leaf conflicts to stale', () async {
    final plan = revision3VoicePreviewPlan();
    final managed = _FakeVoiceTakePreviewManagedLease(
      root: Directory('managed-voice-preview-stale'),
      projectIdValue: revision3VoiceContentProjectId,
      projectRevision: 7,
      head: _head(7),
    );
    final coordinator = CurrentProjectCoordinator(
      openManagedRevision3: (_) async => managed,
    );
    addTearDown(() async {
      await coordinator.shutdown();
      coordinator.dispose();
    });
    final visible = await coordinator.openManagedRevision3(managed.root);

    for (final code in <String>[
      'AUTHORING_REVISION3_VOICE_PREVIEW_LINE_CONFLICT',
      'AUTHORING_REVISION3_VOICE_PREVIEW_LOCALIZATION_CONFLICT',
      'AUTHORING_REVISION3_VOICE_PREVIEW_SLOT_CONFLICT',
      'AUTHORING_REVISION3_VOICE_PREVIEW_TAKE_CONFLICT',
      'AUTHORING_REVISION3_VOICE_PREVIEW_ASSET_CONFLICT',
    ]) {
      managed.nextError = ModFfiException(
        command: 'authoring_store_materialize_revision3_voice_take_preview_v1',
        code: code,
        message: 'fake semantic preview drift',
      );
      await expectLater(
        coordinator.materializeCurrentRevision3VoiceTakePreview(
          expectedRoot: visible.root.path,
          expectedProjectId: visible.projectId,
          expectedProjectRevision: visible.projectRevision,
          expectedHead: visible.head,
          plan: plan,
        ),
        throwsA(isA<Revision3VoiceTakePreviewStaleCheckpointException>()),
        reason: code,
      );
      expect(managed.requiresReopen, isFalse, reason: code);
    }
  });

  test(
    'Voice preview preserves stale classification with failed cleanup',
    () async {
      late Revision3VoiceTakePreviewMaterializationCleanupException retained;
      late String registeredRoot;
      try {
        await Revision3VoiceTakePreviewCapability.materialize(
          register: () async {
            final previewRoot =
                (await createRevision3VoicePreviewTestRoot()).path;
            registeredRoot = previewRoot;
            return AuthoringRevision3VoiceTakePreviewRegistration.fromJson(
              revision3VoicePreviewRegistrationResponse(
                previewRoot: previewRoot,
              ),
            );
          },
          materialize: (token, previewRoot) async {
            await File(
              '$previewRoot${Platform.pathSeparator}unexpected',
            ).writeAsString('lock');
            throw const ModFfiException(
              command:
                  'authoring_store_materialize_revision3_voice_take_preview_v1',
              code: 'AUTHORING_REVISION3_VOICE_PREVIEW_TAKE_CONFLICT',
              message: 'fake stale Voice take',
            );
          },
          release: (token) => _deleteFakeVoicePreviewRoot(registeredRoot),
        );
        fail('stale materialization must retain failed cleanup');
      } on Revision3VoiceTakePreviewMaterializationCleanupException catch (
        error
      ) {
        retained = error;
      }
      final managed = _FakeVoiceTakePreviewManagedLease(
        root: Directory('managed-voice-preview-stale-cleanup'),
        projectIdValue: revision3VoiceContentProjectId,
        projectRevision: 7,
        head: _head(7),
        nextError: retained,
      );
      final coordinator = CurrentProjectCoordinator(
        openManagedRevision3: (_) async => managed,
      );
      addTearDown(() async {
        await coordinator.shutdown();
        coordinator.dispose();
      });
      final visible = await coordinator.openManagedRevision3(managed.root);
      late Revision3VoiceTakePreviewStaleCheckpointException failure;

      try {
        await coordinator.materializeCurrentRevision3VoiceTakePreview(
          expectedRoot: visible.root.path,
          expectedProjectId: visible.projectId,
          expectedProjectRevision: visible.projectRevision,
          expectedHead: visible.head,
          plan: revision3VoicePreviewPlan(),
        );
        fail('graph drift must remain stale while cleanup is retained');
      } on Revision3VoiceTakePreviewStaleCheckpointException catch (error) {
        failure = error;
      }

      expect(failure.cleanupObligation, same(retained));
      expect(managed.requiresReopen, isFalse);
      expect(managed.relatchCalls, 0);
      await File(
        '${retained.diagnosticPreviewRoot}${Platform.pathSeparator}unexpected',
      ).delete();
      await failure.cleanupObligation!.retryCleanup();
      expect(retained.isCleaned, isTrue);

      final retry = await coordinator
          .materializeCurrentRevision3VoiceTakePreview(
            expectedRoot: visible.root.path,
            expectedProjectId: visible.projectId,
            expectedProjectRevision: visible.projectRevision,
            expectedHead: visible.head,
            plan: revision3VoicePreviewPlan(),
          );
      await retry.close();
      expect(managed.materializeCalls, 2);
    },
  );

  test('Voice preview temp-output failure stays local and retryable', () async {
    final plan = revision3VoicePreviewPlan();
    final managed = _FakeVoiceTakePreviewManagedLease(
      root: Directory('managed-voice-preview-local'),
      projectIdValue: revision3VoiceContentProjectId,
      projectRevision: 7,
      head: _head(7),
    );
    final coordinator = CurrentProjectCoordinator(
      openManagedRevision3: (_) async => managed,
    );
    addTearDown(() async {
      await coordinator.shutdown();
      coordinator.dispose();
    });
    final visible = await coordinator.openManagedRevision3(managed.root);

    const localCodes = <String>[
      'AUTHORING_REVISION3_VOICE_PREVIEW_PREVIEW_CAPABILITY_CONFLICT',
      'AUTHORING_REVISION3_VOICE_PREVIEW_PREVIEW_CAPABILITY_LIMIT',
      'AUTHORING_REVISION3_VOICE_PREVIEW_PREVIEW_CAPABILITY_UNAVAILABLE',
      'AUTHORING_REVISION3_VOICE_PREVIEW_CLEANUP_TOKEN_UNKNOWN',
      'AUTHORING_REVISION3_VOICE_PREVIEW_PREVIEW_IO',
    ];
    for (final code in localCodes) {
      managed.nextError = ModFfiException(
        command: 'authoring_store_materialize_revision3_voice_take_preview_v1',
        code: code,
        message: 'fake local preview capability failure',
      );
      await expectLater(
        coordinator.materializeCurrentRevision3VoiceTakePreview(
          expectedRoot: visible.root.path,
          expectedProjectId: visible.projectId,
          expectedProjectRevision: visible.projectRevision,
          expectedHead: visible.head,
          plan: plan,
        ),
        throwsA(
          isA<ModFfiException>().having((error) => error.code, 'code', code),
        ),
        reason: code,
      );
      expect(managed.requiresReopen, isFalse, reason: code);
    }

    final capability = await coordinator
        .materializeCurrentRevision3VoiceTakePreview(
          expectedRoot: visible.root.path,
          expectedProjectId: visible.projectId,
          expectedProjectRevision: visible.projectRevision,
          expectedHead: visible.head,
          plan: plan,
        );
    expect(managed.materializeCalls, localCodes.length + 1);
    await capability.close();
  });

  test(
    'Voice preview cleanup obligation crosses coordinator unchanged',
    () async {
      late Revision3VoiceTakePreviewMaterializationCleanupException retained;
      late String registeredRoot;
      try {
        await Revision3VoiceTakePreviewCapability.materialize(
          register: () async {
            final previewRoot =
                (await createRevision3VoicePreviewTestRoot()).path;
            registeredRoot = previewRoot;
            return AuthoringRevision3VoiceTakePreviewRegistration.fromJson(
              revision3VoicePreviewRegistrationResponse(
                previewRoot: previewRoot,
              ),
            );
          },
          materialize: (token, previewRoot) async {
            await File(
              '$previewRoot${Platform.pathSeparator}unexpected',
            ).writeAsString('lock');
            throw const ModFfiException(
              command:
                  'authoring_store_materialize_revision3_voice_take_preview_v1',
              code: 'AUTHORING_REVISION3_VOICE_PREVIEW_PREVIEW_IO',
              message: 'fake local preview failure',
            );
          },
          release: (token) => _deleteFakeVoicePreviewRoot(registeredRoot),
        );
        fail('nested capability must retain failed cleanup');
      } on Revision3VoiceTakePreviewMaterializationCleanupException catch (
        error
      ) {
        retained = error;
      }
      final managed = _FakeVoiceTakePreviewManagedLease(
        root: Directory('managed-voice-preview-cleanup-retainer'),
        projectIdValue: revision3VoiceContentProjectId,
        projectRevision: 7,
        head: _head(7),
        nextError: retained,
      );
      final coordinator = CurrentProjectCoordinator(
        openManagedRevision3: (_) async => managed,
      );
      addTearDown(() async {
        await coordinator.shutdown();
        coordinator.dispose();
      });
      final visible = await coordinator.openManagedRevision3(managed.root);

      await expectLater(
        coordinator.materializeCurrentRevision3VoiceTakePreview(
          expectedRoot: visible.root.path,
          expectedProjectId: visible.projectId,
          expectedProjectRevision: visible.projectRevision,
          expectedHead: visible.head,
          plan: revision3VoicePreviewPlan(),
        ),
        throwsA(same(retained)),
      );
      expect(managed.requiresReopen, isFalse);
      expect(managed.relatchCalls, 0);

      await File(
        '${retained.diagnosticPreviewRoot}${Platform.pathSeparator}unexpected',
      ).delete();
      await retained.retryCleanup();
      expect(retained.isCleaned, isTrue);
    },
  );

  test(
    'poisoned Voice preview maps to requires-reopen and locks retry',
    () async {
      final plan = revision3VoicePreviewPlan();
      final managed = _FakeVoiceTakePreviewManagedLease(
        root: Directory('managed-voice-preview-poison'),
        projectIdValue: revision3VoiceContentProjectId,
        projectRevision: 7,
        head: _head(7),
        poisonOnError: true,
        nextError: StateError('fake Store verification uncertainty'),
      );
      final coordinator = CurrentProjectCoordinator(
        openManagedRevision3: (_) async => managed,
      );
      addTearDown(() async {
        await coordinator.shutdown();
        coordinator.dispose();
      });
      final visible = await coordinator.openManagedRevision3(managed.root);

      Future<void> materialize() async {
        await coordinator.materializeCurrentRevision3VoiceTakePreview(
          expectedRoot: visible.root.path,
          expectedProjectId: visible.projectId,
          expectedProjectRevision: visible.projectRevision,
          expectedHead: visible.head,
          plan: plan,
        );
      }

      await expectLater(
        materialize(),
        throwsA(isA<Revision3VoiceTakePreviewRequiresReopenException>()),
      );
      expect(managed.materializeCalls, 1);
      expect(managed.requiresReopen, isTrue);
      await expectLater(
        materialize(),
        throwsA(isA<Revision3VoiceTakePreviewRequiresReopenException>()),
      );
      expect(managed.materializeCalls, 1);
    },
  );

  test(
    'Voice preview receipt mismatch relatches and preserves failed cleanup ownership',
    () async {
      final plan = revision3VoicePreviewPlan();
      final managed = _FakeVoiceTakePreviewManagedLease(
        root: Directory('managed-voice-preview-mismatch'),
        projectIdValue: revision3VoiceContentProjectId,
        projectRevision: 7,
        head: _head(7),
        receiptMismatch: true,
        cleanupFailure: true,
      );
      final coordinator = CurrentProjectCoordinator(
        openManagedRevision3: (_) async => managed,
      );
      addTearDown(() async {
        await coordinator.shutdown();
        coordinator.dispose();
      });
      final visible = await coordinator.openManagedRevision3(managed.root);
      late Revision3VoiceTakePreviewRequiresReopenException failure;

      try {
        await coordinator.materializeCurrentRevision3VoiceTakePreview(
          expectedRoot: visible.root.path,
          expectedProjectId: visible.projectId,
          expectedProjectRevision: visible.projectRevision,
          expectedHead: visible.head,
          plan: plan,
        );
        fail('mismatched receipt must fail closed');
      } on Revision3VoiceTakePreviewRequiresReopenException catch (error) {
        failure = error;
      }

      final capability = managed.lastCapability!;
      expect(managed.relatchCalls, 1);
      expect(managed.requiresReopen, isTrue);
      expect(capability.isClosed, isFalse);
      expect(failure.cleanupObligation, same(capability));
      await Directory(capability.path).delete();
      await failure.cleanupObligation!.retryCleanup();
      expect(capability.isClosed, isTrue);
    },
  );

  test('Voice preview remains an explicit optional lease capability', () async {
    final managed = _FakeManagedLease(
      root: Directory('managed-voice-preview-unsupported'),
      projectIdValue: revision3VoiceContentProjectId,
      projectRevision: 7,
      head: _head(7),
    );
    final coordinator = CurrentProjectCoordinator(
      openManagedRevision3: (_) async => managed,
    );
    addTearDown(() async {
      await coordinator.shutdown();
      coordinator.dispose();
    });
    final visible = await coordinator.openManagedRevision3(managed.root);

    await expectLater(
      coordinator.materializeCurrentRevision3VoiceTakePreview(
        expectedRoot: visible.root.path,
        expectedProjectId: visible.projectId,
        expectedProjectRevision: visible.projectRevision,
        expectedHead: visible.head,
        plan: revision3VoicePreviewPlan(),
      ),
      throwsA(isA<CurrentProjectOperationUnsupportedException>()),
    );

    final disabled = _FakeVoiceTakePreviewManagedLease(
      root: Directory('managed-voice-preview-disabled'),
      projectIdValue: revision3VoiceContentProjectId,
      projectRevision: 7,
      head: _head(7),
      supportsPreview: false,
    );
    final disabledCoordinator = CurrentProjectCoordinator(
      openManagedRevision3: (_) async => disabled,
    );
    addTearDown(() async {
      await disabledCoordinator.shutdown();
      disabledCoordinator.dispose();
    });
    final disabledVisible = await disabledCoordinator.openManagedRevision3(
      disabled.root,
    );
    await expectLater(
      disabledCoordinator.materializeCurrentRevision3VoiceTakePreview(
        expectedRoot: disabledVisible.root.path,
        expectedProjectId: disabledVisible.projectId,
        expectedProjectRevision: disabledVisible.projectRevision,
        expectedHead: disabledVisible.head,
        plan: revision3VoicePreviewPlan(),
      ),
      throwsA(isA<CurrentProjectOperationUnsupportedException>()),
    );
    expect(disabled.materializeCalls, 0);
  });

  test('poisoned Voice failure locks retries behind requires-reopen', () async {
    final plan = _voicePlan();
    final managed = _FakeManagedLease(
      root: Directory('managed-poisoned-voice'),
      projectIdValue: revision3VoiceContentProjectId,
      projectRevision: 7,
      head: _head(7),
      onVoicePublish: (lease, _, _) {
        lease.requiresReopenValue = true;
        throw StateError('injected Voice publication verification failure');
      },
    );
    final coordinator = CurrentProjectCoordinator(
      openManagedRevision3: (_) async => managed,
    );
    addTearDown(() async {
      await coordinator.shutdown();
      coordinator.dispose();
    });
    await coordinator.openManagedRevision3(managed.root);

    Future<void> publish() async {
      await coordinator.addCurrentRevision3VoiceTake(
        expectedRoot: managed.root.path,
        expectedProjectId: managed.projectId,
        expectedProjectRevision: 7,
        expectedHead: _head(7),
        gameRoot: r'C:\Games\Gothic Remake',
        plan: plan,
      );
    }

    await expectLater(
      publish(),
      throwsA(isA<Revision3VoiceTakeRequiresReopenException>()),
    );
    expect(managed.voicePublishCalls, 1);
    expect(
      (coordinator.state as ManagedRevision3CurrentProjectState).requiresReopen,
      isTrue,
    );
    await expectLater(
      publish(),
      throwsA(isA<Revision3VoiceTakeRequiresReopenException>()),
    );
    expect(managed.voicePublishCalls, 1);
  });

  test(
    'Voice selection is exact-checkpoint bound, needs no game root, and refreshes state',
    () async {
      final plan = _voiceSelectionPlan();
      final managed = _FakeManagedLease(
        root: Directory('managed-voice-selection'),
        projectIdValue: revision3VoiceContentProjectId,
        projectRevision: 7,
        head: _head(7),
        onVoiceSelectionPublish: (lease, received) {
          expect(received, same(plan));
          lease.projectRevision = 8;
          lease.head = _head(8);
          return Revision3VoiceTakeSelectionPublication(
            projectId: revision3VoiceContentProjectId,
            projectRevision: 8,
            lineId: received.lineId,
            slotId: received.slotId,
            slotRevision: received.expectedSlotRevision + 1,
            locale: received.locale,
            locId: received.locId,
            previousSelectedTakeId: received.expectedSelectedTakeId,
            selectedTakeId: received.selectedTakeId,
          );
        },
      );
      final coordinator = CurrentProjectCoordinator(
        openManagedRevision3: (_) async => managed,
      );
      addTearDown(() async {
        await coordinator.shutdown();
        coordinator.dispose();
      });
      await coordinator.openManagedRevision3(managed.root);

      for (final stale
          in <({String root, int revision, AuthoringWorkingHead head})>[
            (root: Directory('another-root').path, revision: 7, head: _head(7)),
            (root: managed.root.path, revision: 6, head: _head(7)),
            (root: managed.root.path, revision: 7, head: _head(6)),
          ]) {
        await expectLater(
          coordinator.selectCurrentRevision3VoiceTake(
            expectedRoot: stale.root,
            expectedProjectId: managed.projectId,
            expectedProjectRevision: stale.revision,
            expectedHead: stale.head,
            plan: plan,
          ),
          throwsA(isA<Revision3VoiceTakeSelectionStaleCheckpointException>()),
        );
      }
      expect(managed.voiceSelectionPublishCalls, 0);

      final publication = await coordinator.selectCurrentRevision3VoiceTake(
        expectedRoot: managed.root.path,
        expectedProjectId: managed.projectId,
        expectedProjectRevision: 7,
        expectedHead: _head(7),
        plan: plan,
      );
      expect(publication.selectedTakeId, isNull);
      expect(managed.voiceSelectionPublishCalls, 1);
      final state = coordinator.state as ManagedRevision3CurrentProjectState;
      expect(state.projectRevision, 8);
      expect(state.head.canonicalJson, _head(8).canonicalJson);
    },
  );

  test(
    'poisoned Voice selection maps to requires-reopen and locks retry',
    () async {
      final plan = _voiceSelectionPlan();
      final managed = _FakeManagedLease(
        root: Directory('managed-poisoned-voice-selection'),
        projectIdValue: revision3VoiceContentProjectId,
        projectRevision: 7,
        head: _head(7),
        onVoiceSelectionPublish: (lease, _) {
          lease.requiresReopenValue = true;
          throw StateError('injected Voice selection verification failure');
        },
      );
      final coordinator = CurrentProjectCoordinator(
        openManagedRevision3: (_) async => managed,
      );
      addTearDown(() async {
        await coordinator.shutdown();
        coordinator.dispose();
      });
      await coordinator.openManagedRevision3(managed.root);

      Future<void> publish() async {
        await coordinator.selectCurrentRevision3VoiceTake(
          expectedRoot: managed.root.path,
          expectedProjectId: managed.projectId,
          expectedProjectRevision: 7,
          expectedHead: _head(7),
          plan: plan,
        );
      }

      await expectLater(
        publish(),
        throwsA(isA<Revision3VoiceTakeSelectionRequiresReopenException>()),
      );
      expect(managed.voiceSelectionPublishCalls, 1);
      expect(
        (coordinator.state as ManagedRevision3CurrentProjectState)
            .requiresReopen,
        isTrue,
      );
      await expectLater(
        publish(),
        throwsA(isA<Revision3VoiceTakeSelectionRequiresReopenException>()),
      );
      expect(managed.voiceSelectionPublishCalls, 1);
    },
  );

  test(
    'Voice take removal binds exact visible checkpoint and refreshes state',
    () async {
      final plan = _voiceTakeRemovalPlan();
      final managed = _FakeVoiceTakeRemovalManagedLease(
        root: Directory('managed-voice-take-removal'),
        projectIdValue: revision3VoiceContentProjectId,
        projectRevision: 7,
        head: _head(7),
      );
      final coordinator = CurrentProjectCoordinator(
        openManagedRevision3: (_) async => managed,
      );
      addTearDown(() async {
        await coordinator.shutdown();
        coordinator.dispose();
      });
      final visible = await coordinator.openManagedRevision3(managed.root);

      await expectLater(
        coordinator.removeCurrentRevision3VoiceTake(
          expectedRoot: visible.root.path,
          expectedProjectId: visible.projectId,
          expectedProjectRevision: visible.projectRevision - 1,
          expectedHead: visible.head,
          plan: plan,
        ),
        throwsA(isA<Revision3VoiceTakeRemovalStaleCheckpointException>()),
      );
      expect(managed.voiceTakeRemovalCalls, 0);

      final publication = await coordinator.removeCurrentRevision3VoiceTake(
        expectedRoot: visible.root.path,
        expectedProjectId: visible.projectId,
        expectedProjectRevision: visible.projectRevision,
        expectedHead: visible.head,
        plan: plan,
      );

      expect(publication.takeId, plan.takeId);
      expect(publication.takeRevision, plan.expectedTakeRevision);
      expect(publication.selectionCleared, plan.expectsSelectionCleared);
      expect(publication.takeEntityRemoved, plan.expectedTakeEntityRemoved);
      expect(
        publication.remainingCandidateCount,
        plan.expectedRemainingCandidateCount,
      );
      expect(managed.voiceTakeRemovalCalls, 1);
      final state = coordinator.state as ManagedRevision3CurrentProjectState;
      expect(state.projectRevision, 8);
      expect(state.head.canonicalJson, _head(8).canonicalJson);
      expect(state.requiresReopen, isFalse);
    },
  );

  test(
    'correctable Voice take removal conflict is stale and retryable',
    () async {
      final plan = _voiceTakeRemovalPlan();
      final managed = _FakeVoiceTakeRemovalManagedLease(
        root: Directory('managed-voice-take-removal-stale'),
        projectIdValue: revision3VoiceContentProjectId,
        projectRevision: 7,
        head: _head(7),
        nextError: const ModFfiException(
          command: 'authoring_store_prepare_revision3_voice_take_removal_v1',
          code: 'AUTHORING_REVISION3_VOICE_TAKE_REMOVAL_SLOT_CONFLICT',
          message: 'injected closed conflict',
        ),
      );
      final coordinator = CurrentProjectCoordinator(
        openManagedRevision3: (_) async => managed,
      );
      addTearDown(() async {
        await coordinator.shutdown();
        coordinator.dispose();
      });
      final visible = await coordinator.openManagedRevision3(managed.root);

      await expectLater(
        coordinator.removeCurrentRevision3VoiceTake(
          expectedRoot: visible.root.path,
          expectedProjectId: visible.projectId,
          expectedProjectRevision: visible.projectRevision,
          expectedHead: visible.head,
          plan: plan,
        ),
        throwsA(isA<Revision3VoiceTakeRemovalStaleCheckpointException>()),
      );
      expect(managed.requiresReopen, isFalse);
      expect(managed.voiceTakeRemovalRelatchCalls, 0);
    },
  );

  test(
    'disabled Voice take removal capability is rejected before mutation',
    () async {
      final plan = _voiceTakeRemovalPlan();
      final managed = _FakeVoiceTakeRemovalManagedLease(
        root: Directory('managed-voice-take-removal-disabled'),
        projectIdValue: revision3VoiceContentProjectId,
        projectRevision: 7,
        head: _head(7),
        supportsRemoval: false,
      );
      final coordinator = CurrentProjectCoordinator(
        openManagedRevision3: (_) async => managed,
      );
      addTearDown(() async {
        await coordinator.shutdown();
        coordinator.dispose();
      });
      final visible = await coordinator.openManagedRevision3(managed.root);

      await expectLater(
        coordinator.removeCurrentRevision3VoiceTake(
          expectedRoot: visible.root.path,
          expectedProjectId: visible.projectId,
          expectedProjectRevision: visible.projectRevision,
          expectedHead: visible.head,
          plan: plan,
        ),
        throwsA(isA<CurrentProjectOperationUnsupportedException>()),
      );
      expect(managed.voiceTakeRemovalCalls, 0);
      expect(managed.requiresReopen, isFalse);
    },
  );

  test('Voice take removal receipt mismatch latches requires-reopen', () async {
    final plan = _voiceTakeRemovalPlan();
    final managed = _FakeVoiceTakeRemovalManagedLease(
      root: Directory('managed-voice-take-removal-mismatch'),
      projectIdValue: revision3VoiceContentProjectId,
      projectRevision: 7,
      head: _head(7),
      receiptMismatch: true,
    );
    final coordinator = CurrentProjectCoordinator(
      openManagedRevision3: (_) async => managed,
    );
    addTearDown(() async {
      await coordinator.shutdown();
      coordinator.dispose();
    });
    final visible = await coordinator.openManagedRevision3(managed.root);

    await expectLater(
      coordinator.removeCurrentRevision3VoiceTake(
        expectedRoot: visible.root.path,
        expectedProjectId: visible.projectId,
        expectedProjectRevision: visible.projectRevision,
        expectedHead: visible.head,
        plan: plan,
      ),
      throwsA(isA<Revision3VoiceTakeRemovalRequiresReopenException>()),
    );
    expect(managed.requiresReopen, isTrue);
    expect(managed.voiceTakeRemovalRelatchCalls, 1);
    expect(
      (coordinator.state as ManagedRevision3CurrentProjectState).requiresReopen,
      isTrue,
    );
  });

  test(
    'only-wrong Voice take entity-removal receipt flag requires reopen',
    () async {
      final plan = _voiceTakeRemovalPlan();
      final managed = _FakeVoiceTakeRemovalManagedLease(
        root: Directory('managed-voice-take-removal-flag-mismatch'),
        projectIdValue: revision3VoiceContentProjectId,
        projectRevision: 7,
        head: _head(7),
        takeEntityFlagMismatch: true,
      );
      final coordinator = CurrentProjectCoordinator(
        openManagedRevision3: (_) async => managed,
      );
      addTearDown(() async {
        await coordinator.shutdown();
        coordinator.dispose();
      });
      final visible = await coordinator.openManagedRevision3(managed.root);

      await expectLater(
        coordinator.removeCurrentRevision3VoiceTake(
          expectedRoot: visible.root.path,
          expectedProjectId: visible.projectId,
          expectedProjectRevision: visible.projectRevision,
          expectedHead: visible.head,
          plan: plan,
        ),
        throwsA(isA<Revision3VoiceTakeRemovalRequiresReopenException>()),
      );
      expect(managed.requiresReopen, isTrue);
      expect(managed.voiceTakeRemovalRelatchCalls, 1);
    },
  );

  test(
    'post-publish FormatException poisons Voice removal and cannot retry',
    () async {
      final plan = _voiceTakeRemovalPlan();
      final managed = _FakeVoiceTakeRemovalManagedLease(
        root: Directory('managed-voice-take-removal-post-format'),
        projectIdValue: revision3VoiceContentProjectId,
        projectRevision: 7,
        head: _head(7),
        postPublishError: const FormatException(
          'injected post-publication receipt decode failure',
        ),
      );
      final coordinator = CurrentProjectCoordinator(
        openManagedRevision3: (_) async => managed,
      );
      addTearDown(() async {
        await coordinator.shutdown();
        coordinator.dispose();
      });
      final visible = await coordinator.openManagedRevision3(managed.root);

      Future<void> remove() async {
        await coordinator.removeCurrentRevision3VoiceTake(
          expectedRoot: visible.root.path,
          expectedProjectId: visible.projectId,
          expectedProjectRevision: visible.projectRevision,
          expectedHead: visible.head,
          plan: plan,
        );
      }

      await expectLater(
        remove(),
        throwsA(isA<Revision3VoiceTakeRemovalRequiresReopenException>()),
      );
      expect(managed.projectRevision, 8);
      expect(managed.requiresReopen, isTrue);
      expect(managed.voiceTakeRemovalRelatchCalls, 1);
      expect(managed.voiceTakeRemovalCalls, 1);
      await expectLater(
        remove(),
        throwsA(isA<Revision3VoiceTakeRemovalRequiresReopenException>()),
      );
      expect(managed.voiceTakeRemovalCalls, 1);
    },
  );

  test(
    'dialog Voice slot removal binds exact checkpoint and refreshes state',
    () async {
      final plan = _dialogVoiceSlotRemovalPlan();
      final managed = _FakeDialogVoiceSlotRemovalManagedLease(
        root: Directory('managed-dialog-voice-slot-removal'),
        projectIdValue: revision3VoiceContentProjectId,
        projectRevision: 7,
        head: _head(7),
      );
      final coordinator = CurrentProjectCoordinator(
        openManagedRevision3: (_) async => managed,
      );
      addTearDown(() async {
        await coordinator.shutdown();
        coordinator.dispose();
      });
      final visible = await coordinator.openManagedRevision3(managed.root);

      await expectLater(
        coordinator.removeCurrentRevision3DialogVoiceSlot(
          expectedRoot: visible.root.path,
          expectedProjectId: visible.projectId,
          expectedProjectRevision: visible.projectRevision - 1,
          expectedHead: visible.head,
          plan: plan,
        ),
        throwsA(isA<Revision3DialogVoiceSlotRemovalStaleCheckpointException>()),
      );
      expect(managed.removalCalls, 0);

      final publication = await coordinator
          .removeCurrentRevision3DialogVoiceSlot(
            expectedRoot: visible.root.path,
            expectedProjectId: visible.projectId,
            expectedProjectRevision: visible.projectRevision,
            expectedHead: visible.head,
            plan: plan,
          );
      expect(publication.lineRevision, plan.expectedLineRevision + 1);
      expect(publication.removedSlotRevision, plan.expectedSlotRevision);
      expect(publication.removedTargetResolution, plan.targetResolution);
      expect(managed.removalCalls, 1);
      final state = coordinator.state as ManagedRevision3CurrentProjectState;
      expect(state.projectRevision, 8);
      expect(state.head.canonicalJson, _head(8).canonicalJson);
      expect(state.requiresReopen, isFalse);
    },
  );

  test(
    'correctable dialog Voice slot conflict is stale and retryable',
    () async {
      final plan = _dialogVoiceSlotRemovalPlan();
      final managed = _FakeDialogVoiceSlotRemovalManagedLease(
        root: Directory('managed-dialog-voice-slot-removal-stale'),
        projectIdValue: revision3VoiceContentProjectId,
        projectRevision: 7,
        head: _head(7),
        nextError: const ModFfiException(
          command:
              'authoring_store_prepare_revision3_dialog_voice_slot_removal_v1',
          code: 'AUTHORING_REVISION3_DIALOG_VOICE_SLOT_REMOVAL_NOT_EMPTY',
          message: 'injected closed conflict',
        ),
      );
      final coordinator = CurrentProjectCoordinator(
        openManagedRevision3: (_) async => managed,
      );
      addTearDown(() async {
        await coordinator.shutdown();
        coordinator.dispose();
      });
      final visible = await coordinator.openManagedRevision3(managed.root);

      await expectLater(
        coordinator.removeCurrentRevision3DialogVoiceSlot(
          expectedRoot: visible.root.path,
          expectedProjectId: visible.projectId,
          expectedProjectRevision: visible.projectRevision,
          expectedHead: visible.head,
          plan: plan,
        ),
        throwsA(isA<Revision3DialogVoiceSlotRemovalStaleCheckpointException>()),
      );
      expect(managed.requiresReopen, isFalse);
      expect(managed.relatchCalls, 0);
    },
  );

  test('dialog Voice slot receipt mismatch latches requires-reopen', () async {
    final plan = _dialogVoiceSlotRemovalPlan();
    final managed = _FakeDialogVoiceSlotRemovalManagedLease(
      root: Directory('managed-dialog-voice-slot-removal-mismatch'),
      projectIdValue: revision3VoiceContentProjectId,
      projectRevision: 7,
      head: _head(7),
      receiptMismatch: true,
    );
    final coordinator = CurrentProjectCoordinator(
      openManagedRevision3: (_) async => managed,
    );
    addTearDown(() async {
      await coordinator.shutdown();
      coordinator.dispose();
    });
    final visible = await coordinator.openManagedRevision3(managed.root);

    await expectLater(
      coordinator.removeCurrentRevision3DialogVoiceSlot(
        expectedRoot: visible.root.path,
        expectedProjectId: visible.projectId,
        expectedProjectRevision: visible.projectRevision,
        expectedHead: visible.head,
        plan: plan,
      ),
      throwsA(isA<Revision3DialogVoiceSlotRemovalRequiresReopenException>()),
    );
    expect(managed.requiresReopen, isTrue);
    expect(managed.relatchCalls, 1);
  });

  test(
    'Voice take status is exact-visible-tuple bound and refreshes the published checkpoint',
    () async {
      final plan = _voiceTakeStatusPlan();
      final managed = _FakeManagedLease(
        root: Directory('managed-voice-take-status'),
        projectIdValue: revision3VoiceContentProjectId,
        projectRevision: 7,
        head: _head(7),
        onVoiceTakeStatusPublish: (lease, received) {
          expect(received, same(plan));
          lease.projectRevision = 8;
          lease.head = _head(8);
          return Revision3VoiceTakeStatusPublication(
            projectId: revision3VoiceContentProjectId,
            projectRevision: 8,
            lineId: received.lineId,
            localizationId: received.localizationId,
            slotId: received.slotId,
            slotRevision: received.expectedSlotRevision,
            locale: received.locale,
            locId: received.locId,
            takeId: received.takeId,
            takeRevision: received.expectedTakeRevision + 1,
            previousStatus: received.expectedStatus,
            status: received.desiredStatus,
          );
        },
      );
      final coordinator = CurrentProjectCoordinator(
        openManagedRevision3: (_) async => managed,
      );
      addTearDown(() async {
        await coordinator.shutdown();
        coordinator.dispose();
      });
      await coordinator.openManagedRevision3(managed.root);

      for (final stale
          in <
            ({
              String root,
              String projectId,
              int revision,
              AuthoringWorkingHead head,
            })
          >[
            (
              root: Directory('another-root').path,
              projectId: managed.projectId,
              revision: 7,
              head: _head(7),
            ),
            (
              root: managed.root.path,
              projectId: '99999999999999999999999999999999',
              revision: 7,
              head: _head(7),
            ),
            (
              root: managed.root.path,
              projectId: managed.projectId,
              revision: 6,
              head: _head(7),
            ),
            (
              root: managed.root.path,
              projectId: managed.projectId,
              revision: 7,
              head: _head(6),
            ),
          ]) {
        await expectLater(
          coordinator.editCurrentRevision3VoiceTakeStatus(
            expectedRoot: stale.root,
            expectedProjectId: stale.projectId,
            expectedProjectRevision: stale.revision,
            expectedHead: stale.head,
            plan: plan,
          ),
          throwsA(isA<Revision3VoiceTakeStatusStaleCheckpointException>()),
        );
      }
      expect(managed.voiceTakeStatusPublishCalls, 0);

      final publication = await coordinator.editCurrentRevision3VoiceTakeStatus(
        expectedRoot: managed.root.path,
        expectedProjectId: managed.projectId,
        expectedProjectRevision: 7,
        expectedHead: _head(7),
        plan: plan,
      );
      expect(publication.projectRevision, 8);
      expect(publication.lineId, plan.lineId);
      expect(publication.localizationId, plan.localizationId);
      expect(publication.slotId, plan.slotId);
      expect(publication.slotRevision, plan.expectedSlotRevision);
      expect(publication.locale, plan.locale);
      expect(publication.locId, plan.locId);
      expect(publication.takeId, plan.takeId);
      expect(publication.takeRevision, plan.expectedTakeRevision + 1);
      expect(publication.previousStatus, plan.expectedStatus);
      expect(publication.status, plan.desiredStatus);
      expect(managed.voiceTakeStatusPublishCalls, 1);
      final state = coordinator.state as ManagedRevision3CurrentProjectState;
      expect(state.projectRevision, 8);
      expect(state.head.canonicalJson, _head(8).canonicalJson);
    },
  );

  test('Voice take status rejects a mismatched publication tuple', () async {
    final plan = _voiceTakeStatusPlan();
    final managed = _FakeManagedLease(
      root: Directory('managed-voice-take-status-mismatch'),
      projectIdValue: revision3VoiceContentProjectId,
      projectRevision: 7,
      head: _head(7),
      onVoiceTakeStatusPublish: (lease, received) {
        lease.projectRevision = 8;
        lease.head = _head(8);
        return Revision3VoiceTakeStatusPublication(
          projectId: revision3VoiceContentProjectId,
          projectRevision: 8,
          lineId: received.lineId,
          localizationId: received.localizationId,
          slotId: received.slotId,
          slotRevision: received.expectedSlotRevision + 1,
          locale: received.locale,
          locId: received.locId,
          takeId: received.takeId,
          takeRevision: received.expectedTakeRevision + 1,
          previousStatus: received.expectedStatus,
          status: received.desiredStatus,
        );
      },
    );
    final coordinator = CurrentProjectCoordinator(
      openManagedRevision3: (_) async => managed,
    );
    addTearDown(() async {
      await coordinator.shutdown();
      coordinator.dispose();
    });
    await coordinator.openManagedRevision3(managed.root);

    await expectLater(
      coordinator.editCurrentRevision3VoiceTakeStatus(
        expectedRoot: managed.root.path,
        expectedProjectId: managed.projectId,
        expectedProjectRevision: 7,
        expectedHead: _head(7),
        plan: plan,
      ),
      throwsA(isA<Revision3VoiceTakeStatusRequiresReopenException>()),
    );
    expect(managed.voiceTakeStatusPublishCalls, 1);
    expect(managed.requiresReopen, isTrue);
    expect(
      (coordinator.state as ManagedRevision3CurrentProjectState).requiresReopen,
      isTrue,
    );
    await expectLater(
      coordinator.editCurrentRevision3VoiceTakeStatus(
        expectedRoot: managed.root.path,
        expectedProjectId: managed.projectId,
        expectedProjectRevision: 7,
        expectedHead: _head(7),
        plan: plan,
      ),
      throwsA(isA<Revision3VoiceTakeStatusRequiresReopenException>()),
    );
    expect(managed.voiceTakeStatusPublishCalls, 1);
  });

  test(
    'poisoned Voice take status maps to requires-reopen and locks retry',
    () async {
      final plan = _voiceTakeStatusPlan();
      final managed = _FakeManagedLease(
        root: Directory('managed-poisoned-voice-take-status'),
        projectIdValue: revision3VoiceContentProjectId,
        projectRevision: 7,
        head: _head(7),
        onVoiceTakeStatusPublish: (lease, _) {
          lease.requiresReopenValue = true;
          throw StateError('injected Voice take status verification failure');
        },
      );
      final coordinator = CurrentProjectCoordinator(
        openManagedRevision3: (_) async => managed,
      );
      addTearDown(() async {
        await coordinator.shutdown();
        coordinator.dispose();
      });
      await coordinator.openManagedRevision3(managed.root);

      Future<void> publish() async {
        await coordinator.editCurrentRevision3VoiceTakeStatus(
          expectedRoot: managed.root.path,
          expectedProjectId: managed.projectId,
          expectedProjectRevision: 7,
          expectedHead: _head(7),
          plan: plan,
        );
      }

      await expectLater(
        publish(),
        throwsA(isA<Revision3VoiceTakeStatusRequiresReopenException>()),
      );
      expect(managed.voiceTakeStatusPublishCalls, 1);
      expect(
        (coordinator.state as ManagedRevision3CurrentProjectState)
            .requiresReopen,
        isTrue,
      );
      await expectLater(
        publish(),
        throwsA(isA<Revision3VoiceTakeStatusRequiresReopenException>()),
      );
      expect(managed.voiceTakeStatusPublishCalls, 1);
    },
  );

  test(
    'Voice target publication advances exactly once and Voice build preserves the checkpoint',
    () async {
      final plan = _voiceTargetPlan();
      final managed = _FakeManagedLease(
        root: Directory('managed-voice-target-build'),
        projectIdValue: revision3VoiceContentProjectId,
        projectRevision: 7,
        head: _head(7),
        onVoiceTargetPublish: (lease, gameRoot, received) {
          expect(gameRoot, r'C:\Games\Gothic Remake');
          expect(received, same(plan));
          lease.projectRevision = 8;
          lease.head = _head(8);
          return Revision3VoiceTargetPublication(
            projectId: revision3VoiceContentProjectId,
            projectRevision: 8,
            lineId: received.lineId,
            slotId: received.slotId,
            locale: received.locale,
            locId: received.locId,
            resolution: AuthoringRevision3VoiceTargetResolutionState.resolved,
            matchCount: 1,
          );
        },
        onVoiceBuild: (lease, gameRoot, output) {
          expect(gameRoot, r'C:\Games\Gothic Remake');
          return _voiceBuildResult(
            head: lease.head,
            projectId: lease.projectId,
            projectRevision: lease.projectRevision,
            output: output,
          );
        },
      );
      final coordinator = CurrentProjectCoordinator(
        openManagedRevision3: (_) async => managed,
      );
      addTearDown(() async {
        await coordinator.shutdown();
        coordinator.dispose();
      });
      await coordinator.openManagedRevision3(managed.root);

      await expectLater(
        coordinator.resolveCurrentRevision3VoiceTarget(
          expectedRoot: managed.root.path,
          expectedProjectId: managed.projectId,
          expectedProjectRevision: 6,
          expectedHead: _head(6),
          gameRoot: r'C:\Games\Gothic Remake',
          plan: plan,
        ),
        throwsA(isA<Revision3VoiceTargetStaleCheckpointException>()),
      );
      expect(managed.voiceTargetPublishCalls, 0);

      final publication = await coordinator.resolveCurrentRevision3VoiceTarget(
        expectedRoot: managed.root.path,
        expectedProjectId: managed.projectId,
        expectedProjectRevision: 7,
        expectedHead: _head(7),
        gameRoot: r'C:\Games\Gothic Remake',
        plan: plan,
      );
      expect(
        publication.resolution,
        AuthoringRevision3VoiceTargetResolutionState.resolved,
      );
      expect(managed.voiceTargetPublishCalls, 1);
      expect(
        (coordinator.state as ManagedRevision3CurrentProjectState)
            .projectRevision,
        8,
      );

      const output = r'C:\Builds\managed-voice';
      final build = await coordinator.buildCurrentRevision3Voice(
        expectedRoot: managed.root.path,
        expectedProjectId: managed.projectId,
        expectedProjectRevision: 8,
        expectedHead: _head(8),
        gameRoot: r'C:\Games\Gothic Remake',
        output: output,
      );
      expect(build.isBuilt, isTrue);
      expect(build.output, output);
      expect(managed.voiceBuildCalls, 1);
      expect(managed.projectRevision, 8);
      expect(managed.head.canonicalJson, _head(8).canonicalJson);
    },
  );

  test(
    'Voice plan is exact, read-only, and leaves the current checkpoint unchanged',
    () async {
      final managed = _FakeManagedLease(
        root: Directory('managed-voice-plan'),
        projectIdValue: revision3VoiceContentProjectId,
        projectRevision: 7,
        head: _head(7),
      );
      final coordinator = CurrentProjectCoordinator(
        openManagedRevision3: (_) async => managed,
      );
      addTearDown(() async {
        await coordinator.shutdown();
        coordinator.dispose();
      });
      await coordinator.openManagedRevision3(managed.root);

      await expectLater(
        coordinator.planCurrentRevision3Voice(
          expectedRoot: managed.root.path,
          expectedProjectId: managed.projectId,
          expectedProjectRevision: 6,
          expectedHead: _head(6),
        ),
        throwsA(isA<Revision3VoiceBuildStaleCheckpointException>()),
      );
      expect(managed.voicePlanCalls, 0);
      expect(managed.voiceBuildCalls, 0);

      final result = await coordinator.planCurrentRevision3Voice(
        expectedRoot: managed.root.path,
        expectedProjectId: managed.projectId,
        expectedProjectRevision: 7,
        expectedHead: _head(7),
      );

      expect(result.isReady, isTrue);
      expect(result.totalSlots, 1);
      expect(result.readySlots, 1);
      expect(result.blockers, isEmpty);
      expect(result.basisHead.canonicalJson, _head(7).canonicalJson);
      expect(managed.voicePlanCalls, 1);
      expect(managed.voiceBuildCalls, 0);
      expect(managed.projectRevision, 7);
      expect(managed.head.canonicalJson, _head(7).canonicalJson);
      final after = coordinator.state as ManagedRevision3CurrentProjectState;
      expect(after.projectRevision, 7);
      expect(after.head.canonicalJson, _head(7).canonicalJson);
      expect(after.requiresReopen, isFalse);
    },
  );

  test('poisoned Voice plan maps to requires-reopen and locks retry', () async {
    final managed = _FakeManagedLease(
      root: Directory('managed-poisoned-voice-plan'),
      projectIdValue: revision3VoiceContentProjectId,
      projectRevision: 7,
      head: _head(7),
      onVoicePlan: (lease) {
        lease.requiresReopenValue = true;
        return _voicePlanResult(
          head: lease.head,
          projectId: lease.projectId,
          projectRevision: lease.projectRevision,
        );
      },
    );
    final coordinator = CurrentProjectCoordinator(
      openManagedRevision3: (_) async => managed,
    );
    addTearDown(() async {
      await coordinator.shutdown();
      coordinator.dispose();
    });
    await coordinator.openManagedRevision3(managed.root);

    Future<void> plan() async {
      await coordinator.planCurrentRevision3Voice(
        expectedRoot: managed.root.path,
        expectedProjectId: managed.projectId,
        expectedProjectRevision: 7,
        expectedHead: _head(7),
      );
    }

    await expectLater(
      plan(),
      throwsA(isA<Revision3VoiceBuildRequiresReopenException>()),
    );
    expect(managed.voicePlanCalls, 1);
    expect(managed.voiceBuildCalls, 0);
    expect(
      (coordinator.state as ManagedRevision3CurrentProjectState).requiresReopen,
      isTrue,
    );
    await expectLater(
      plan(),
      throwsA(isA<Revision3VoiceBuildRequiresReopenException>()),
    );
    expect(managed.voicePlanCalls, 1);
    expect(managed.voiceBuildCalls, 0);
  });

  test('Voice plan rejects a result for a different checkpoint', () async {
    final managed = _FakeManagedLease(
      root: Directory('managed-mismatched-voice-plan'),
      projectIdValue: revision3VoiceContentProjectId,
      projectRevision: 7,
      head: _head(7),
      onVoicePlan: (_) => _voicePlanResult(
        head: _head(6),
        projectId: revision3VoiceContentProjectId,
        projectRevision: 6,
      ),
    );
    final coordinator = CurrentProjectCoordinator(
      openManagedRevision3: (_) async => managed,
    );
    addTearDown(() async {
      await coordinator.shutdown();
      coordinator.dispose();
    });
    await coordinator.openManagedRevision3(managed.root);

    await expectLater(
      coordinator.planCurrentRevision3Voice(
        expectedRoot: managed.root.path,
        expectedProjectId: managed.projectId,
        expectedProjectRevision: 7,
        expectedHead: _head(7),
      ),
      throwsA(isA<CurrentProjectCoordinatorException>()),
    );
    expect(managed.voicePlanCalls, 1);
    expect(managed.voiceBuildCalls, 0);
    expect(
      (coordinator.state as ManagedRevision3CurrentProjectState).requiresReopen,
      isFalse,
    );
  });

  test(
    'exact project export is tuple-bound, game-independent, and leaves the current checkpoint unchanged',
    () async {
      const output = r'C:\Exports\project-copy-r7.goremod';
      final managed = _FakeExportManagedLease(
        root: Directory('managed-project-export'),
        projectIdValue: revision3VoiceFixtureProjectId,
        projectRevision: 7,
        head: _head(7),
        onExport: (lease, receivedOutput) {
          expect(receivedOutput, output);
          return _projectExportResult(
            head: lease.head,
            projectId: lease.projectId,
            projectRevision: lease.projectRevision,
            output: receivedOutput,
          );
        },
      );
      final coordinator = CurrentProjectCoordinator(
        openManagedRevision3: (_) async => managed,
      );
      addTearDown(() async {
        await coordinator.shutdown();
        coordinator.dispose();
      });
      final visible = await coordinator.openManagedRevision3(managed.root);

      final result = await coordinator.exportCurrentRevision3ExactSnapshot(
        expectedRoot: visible.root.path,
        expectedProjectId: visible.projectId,
        expectedProjectRevision: visible.projectRevision,
        expectedHead: visible.head,
        output: output,
      );

      expect(result.output, output);
      expect(result.publicationIsUncertain, isFalse);
      expect(managed.exportCalls, 1);
      expect(managed.exportOutputs, [output]);
      final after = coordinator.state as ManagedRevision3CurrentProjectState;
      expect(after.root.path, visible.root.path);
      expect(after.projectId, visible.projectId);
      expect(after.projectRevision, visible.projectRevision);
      expect(after.head.canonicalJson, visible.head.canonicalJson);
      expect(after.requiresReopen, isFalse);
    },
  );

  test(
    'exact project export rejects stale and unsupported sessions before lease access',
    () async {
      final managed = _FakeExportManagedLease(
        root: Directory('managed-project-export-stale'),
        projectIdValue: revision3VoiceFixtureProjectId,
        projectRevision: 7,
        head: _head(7),
        onExport: (lease, output) => _projectExportResult(
          head: lease.head,
          projectId: lease.projectId,
          projectRevision: lease.projectRevision,
          output: output,
        ),
      );
      final coordinator = CurrentProjectCoordinator(
        openManagedRevision3: (_) async => managed,
      );
      addTearDown(() async {
        await coordinator.shutdown();
        coordinator.dispose();
      });
      await coordinator.openManagedRevision3(managed.root);

      await expectLater(
        coordinator.exportCurrentRevision3ExactSnapshot(
          expectedRoot: managed.root.path,
          expectedProjectId: managed.projectId,
          expectedProjectRevision: 6,
          expectedHead: _head(6),
          output: r'C:\Exports\stale.goremod',
        ),
        throwsA(isA<Revision3ProjectExportStaleCheckpointException>()),
      );
      expect(managed.exportCalls, 0);

      final unsupported = _FakeManagedLease(
        root: Directory('managed-project-export-unsupported'),
        projectIdValue: revision3VoiceFixtureProjectId,
        projectRevision: 7,
        head: _head(7),
      );
      final unsupportedCoordinator = CurrentProjectCoordinator(
        openManagedRevision3: (_) async => unsupported,
      );
      addTearDown(() async {
        await unsupportedCoordinator.shutdown();
        unsupportedCoordinator.dispose();
      });
      final unsupportedVisible = await unsupportedCoordinator
          .openManagedRevision3(unsupported.root);

      await expectLater(
        unsupportedCoordinator.exportCurrentRevision3ExactSnapshot(
          expectedRoot: unsupportedVisible.root.path,
          expectedProjectId: unsupportedVisible.projectId,
          expectedProjectRevision: unsupportedVisible.projectRevision,
          expectedHead: unsupportedVisible.head,
          output: r'C:\Exports\unsupported.goremod',
        ),
        throwsA(isA<Revision3ProjectExportUnsupportedException>()),
      );
      expect(unsupported.requiresReopen, isFalse);
    },
  );

  test(
    'exact project export honors a false optional capability and requires-reopen latch',
    () async {
      final managed = _FakeExportManagedLease(
        root: Directory('managed-project-export-capability'),
        projectIdValue: revision3VoiceFixtureProjectId,
        projectRevision: 7,
        head: _head(7),
        supportsExport: false,
        onExport: (lease, output) => _projectExportResult(
          head: lease.head,
          projectId: lease.projectId,
          projectRevision: lease.projectRevision,
          output: output,
        ),
      );
      final coordinator = CurrentProjectCoordinator(
        openManagedRevision3: (_) async => managed,
      );
      addTearDown(() async {
        await coordinator.shutdown();
        coordinator.dispose();
      });
      final visible = await coordinator.openManagedRevision3(managed.root);

      await expectLater(
        coordinator.exportCurrentRevision3ExactSnapshot(
          expectedRoot: visible.root.path,
          expectedProjectId: visible.projectId,
          expectedProjectRevision: visible.projectRevision,
          expectedHead: visible.head,
          output: r'C:\Exports\disabled.goremod',
        ),
        throwsA(isA<Revision3ProjectExportUnsupportedException>()),
      );
      expect(managed.exportCalls, 0);

      managed.requiresReopenValue = true;
      await expectLater(
        coordinator.exportCurrentRevision3ExactSnapshot(
          expectedRoot: visible.root.path,
          expectedProjectId: visible.projectId,
          expectedProjectRevision: visible.projectRevision,
          expectedHead: visible.head,
          output: r'C:\Exports\reopen.goremod',
        ),
        throwsA(isA<Revision3ProjectExportRequiresReopenException>()),
      );
      expect(managed.exportCalls, 0);
    },
  );

  test(
    'exact project export preserves every known safe native prepublication code',
    () async {
      const safeCodes = <String>{
        'AUTHORING_REVISION3_EXPORT_REQUEST_INVALID',
        'AUTHORING_REVISION3_EXPORT_INPUT_LIMIT',
        'AUTHORING_REVISION3_EXPORT_CLOSURE_LIMIT',
        'AUTHORING_REVISION3_EXPORT_OUTPUT_EXISTS',
        'AUTHORING_REVISION3_EXPORT_OUTPUT_INVALID',
        'AUTHORING_REVISION3_EXPORT_ARCHIVE_FAILED',
        'AUTHORING_REVISION3_EXPORT_VERIFY_FAILED',
        'AUTHORING_REVISION3_EXPORT_CLEANUP_FAILED',
        'AUTHORING_REVISION3_EXPORT_PUBLICATION_FAILED',
      };
      const destinationCodes = <String>{
        'AUTHORING_REVISION3_EXPORT_OUTPUT_EXISTS',
        'AUTHORING_REVISION3_EXPORT_OUTPUT_INVALID',
      };

      for (final code in safeCodes) {
        final nativeError = ModFfiException(
          command: 'authoring_store_export_revision3_exact_snapshot_v1',
          code: code,
          message: 'safe prepublication failure',
        );
        final managed = _FakeExportManagedLease(
          root: Directory('managed-project-export-safe-$code'),
          projectIdValue: revision3VoiceFixtureProjectId,
          projectRevision: 7,
          head: _head(7),
          onExport: (_, _) => throw nativeError,
        );
        final coordinator = CurrentProjectCoordinator(
          openManagedRevision3: (_) async => managed,
        );
        addTearDown(() async {
          await coordinator.shutdown();
          coordinator.dispose();
        });
        final visible = await coordinator.openManagedRevision3(managed.root);

        await expectLater(
          coordinator.exportCurrentRevision3ExactSnapshot(
            expectedRoot: visible.root.path,
            expectedProjectId: visible.projectId,
            expectedProjectRevision: visible.projectRevision,
            expectedHead: visible.head,
            output: 'C:\\Exports\\safe-$code.goremod',
          ),
          throwsA(
            isA<Revision3ProjectExportFailedException>()
                .having(
                  (error) => error.publicationMayExist,
                  'publicationMayExist',
                  isFalse,
                )
                .having((error) => error.code, 'code', code)
                .having(
                  (error) => error.retryWithNewDestination,
                  'retryWithNewDestination',
                  destinationCodes.contains(code),
                )
                .having((error) => error.cause, 'cause', same(nativeError)),
          ),
        );
        expect(managed.exportCalls, 1);
        expect(managed.requiresReopen, isFalse);
      }
    },
  );

  test(
    'exact project export keeps head drift and poisoned known prepublication failures output-absent',
    () async {
      late _FakeExportManagedLease headConflictLease;
      headConflictLease = _FakeExportManagedLease(
        root: Directory('managed-project-export-head-conflict'),
        projectIdValue: revision3VoiceFixtureProjectId,
        projectRevision: 7,
        head: _head(7),
        onExport: (_, _) {
          headConflictLease.requiresReopenValue = true;
          throw const ManagedProjectHeadConflictException(
            'head drift before export publication',
          );
        },
      );
      final headCoordinator = CurrentProjectCoordinator(
        openManagedRevision3: (_) async => headConflictLease,
      );
      addTearDown(() async {
        await headCoordinator.shutdown();
        headCoordinator.dispose();
      });
      final headVisible = await headCoordinator.openManagedRevision3(
        headConflictLease.root,
      );

      await expectLater(
        headCoordinator.exportCurrentRevision3ExactSnapshot(
          expectedRoot: headVisible.root.path,
          expectedProjectId: headVisible.projectId,
          expectedProjectRevision: headVisible.projectRevision,
          expectedHead: headVisible.head,
          output: r'C:\Exports\head-conflict.goremod',
        ),
        throwsA(
          isA<Revision3ProjectExportRequiresReopenException>()
              .having(
                (error) => error.publicationMayExist,
                'publicationMayExist',
                isFalse,
              )
              .having(
                (error) => error.cause,
                'cause',
                isA<ManagedProjectHeadConflictException>(),
              ),
        ),
      );
      expect(
        (headCoordinator.state as ManagedRevision3CurrentProjectState)
            .requiresReopen,
        isTrue,
      );

      const code = 'AUTHORING_REVISION3_EXPORT_STORE_CHANGED';
      late _FakeExportManagedLease prepublicationLease;
      prepublicationLease = _FakeExportManagedLease(
        root: Directory('managed-project-export-prepublication-poison'),
        projectIdValue: revision3VoiceFixtureProjectId,
        projectRevision: 7,
        head: _head(7),
        onExport: (_, _) {
          prepublicationLease.requiresReopenValue = true;
          throw const ManagedRevision3ExactSnapshotExportPrepublicationException(
            code: code,
            message: 'Store changed before publication',
          );
        },
      );
      final prepublicationCoordinator = CurrentProjectCoordinator(
        openManagedRevision3: (_) async => prepublicationLease,
      );
      addTearDown(() async {
        await prepublicationCoordinator.shutdown();
        prepublicationCoordinator.dispose();
      });
      final prepublicationVisible = await prepublicationCoordinator
          .openManagedRevision3(prepublicationLease.root);

      await expectLater(
        prepublicationCoordinator.exportCurrentRevision3ExactSnapshot(
          expectedRoot: prepublicationVisible.root.path,
          expectedProjectId: prepublicationVisible.projectId,
          expectedProjectRevision: prepublicationVisible.projectRevision,
          expectedHead: prepublicationVisible.head,
          output: r'C:\Exports\prepublication-poison.goremod',
        ),
        throwsA(
          isA<Revision3ProjectExportRequiresReopenException>()
              .having(
                (error) => error.publicationMayExist,
                'publicationMayExist',
                isFalse,
              )
              .having((error) => error.code, 'code', code),
        ),
      );
      expect(
        (prepublicationCoordinator.state as ManagedRevision3CurrentProjectState)
            .requiresReopen,
        isTrue,
      );
    },
  );

  test(
    'exact project export latches malformed post-call failures before another export',
    () async {
      final managed = _FakeExportManagedLease(
        root: Directory('managed-project-export-unknown-failure'),
        projectIdValue: revision3VoiceFixtureProjectId,
        projectRevision: 7,
        head: _head(7),
        onExport: (_, _) => throw const ModFfiException(
          command: 'authoring_store_export_revision3_exact_snapshot_v1',
          code: ModFfiException.malformedNativeResponseCode,
          message: 'malformed native response',
        ),
      );
      final coordinator = CurrentProjectCoordinator(
        openManagedRevision3: (_) async => managed,
      );
      addTearDown(() async {
        await coordinator.shutdown();
        coordinator.dispose();
      });
      final visible = await coordinator.openManagedRevision3(managed.root);

      Future<void> export(String output) => coordinator
          .exportCurrentRevision3ExactSnapshot(
            expectedRoot: visible.root.path,
            expectedProjectId: visible.projectId,
            expectedProjectRevision: visible.projectRevision,
            expectedHead: visible.head,
            output: output,
          )
          .then<void>((_) {});

      await expectLater(
        export(r'C:\Exports\malformed.goremod'),
        throwsA(
          isA<Revision3ProjectExportRequiresReopenException>()
              .having(
                (error) => error.publicationMayExist,
                'publicationMayExist',
                isTrue,
              )
              .having((error) => error.cause, 'cause', isA<ModFfiException>()),
        ),
      );
      expect(managed.requiresReopen, isTrue);
      expect(managed.publicationUncertaintyLatchCalls, 1);
      await expectLater(
        export(r'C:\Exports\blocked-after-malformed.goremod'),
        throwsA(
          isA<Revision3ProjectExportRequiresReopenException>().having(
            (error) => error.publicationMayExist,
            'publicationMayExist',
            isFalse,
          ),
        ),
      );
      expect(managed.exportCalls, 1);
    },
  );

  test(
    'exact project export latches mismatched and thrown post-call results',
    () async {
      final mismatchLease = _FakeExportManagedLease(
        root: Directory('managed-project-export-failure'),
        projectIdValue: revision3VoiceFixtureProjectId,
        projectRevision: 7,
        head: _head(7),
        onExport: (lease, output) {
          return _projectExportResult(
            head: lease.head,
            projectId: lease.projectId,
            projectRevision: lease.projectRevision + 1,
            output: output,
          );
        },
      );
      final coordinator = CurrentProjectCoordinator(
        openManagedRevision3: (_) async => mismatchLease,
      );
      addTearDown(() async {
        await coordinator.shutdown();
        coordinator.dispose();
      });
      final visible = await coordinator.openManagedRevision3(
        mismatchLease.root,
      );

      Future<void> export(String output) => coordinator
          .exportCurrentRevision3ExactSnapshot(
            expectedRoot: visible.root.path,
            expectedProjectId: visible.projectId,
            expectedProjectRevision: visible.projectRevision,
            expectedHead: visible.head,
            output: output,
          )
          .then<void>((_) {});

      await expectLater(
        export(r'C:\Exports\mismatch.goremod'),
        throwsA(
          isA<Revision3ProjectExportRequiresReopenException>().having(
            (error) => error.publicationMayExist,
            'publicationMayExist',
            isTrue,
          ),
        ),
      );
      expect(mismatchLease.requiresReopen, isTrue);
      expect(mismatchLease.publicationUncertaintyLatchCalls, 1);
      await expectLater(
        export(r'C:\Exports\blocked-after-mismatch.goremod'),
        throwsA(isA<Revision3ProjectExportRequiresReopenException>()),
      );
      expect(mismatchLease.exportCalls, 1);
      final refreshed =
          coordinator.state as ManagedRevision3CurrentProjectState;
      expect(refreshed.projectRevision, visible.projectRevision);
      expect(refreshed.head.canonicalJson, visible.head.canonicalJson);
      expect(refreshed.requiresReopen, isTrue);

      final thrownLease = _FakeExportManagedLease(
        root: Directory('managed-project-export-thrown-failure'),
        projectIdValue: revision3VoiceFixtureProjectId,
        projectRevision: 7,
        head: _head(7),
        onExport: (_, _) => throw StateError('injected export failure'),
      );
      final thrownCoordinator = CurrentProjectCoordinator(
        openManagedRevision3: (_) async => thrownLease,
      );
      addTearDown(() async {
        await thrownCoordinator.shutdown();
        thrownCoordinator.dispose();
      });
      final thrownVisible = await thrownCoordinator.openManagedRevision3(
        thrownLease.root,
      );
      await expectLater(
        thrownCoordinator.exportCurrentRevision3ExactSnapshot(
          expectedRoot: thrownVisible.root.path,
          expectedProjectId: thrownVisible.projectId,
          expectedProjectRevision: thrownVisible.projectRevision,
          expectedHead: thrownVisible.head,
          output: r'C:\Exports\failure.goremod',
        ),
        throwsA(
          isA<Revision3ProjectExportRequiresReopenException>()
              .having((error) => error.cause, 'cause', isA<StateError>())
              .having(
                (error) => error.publicationMayExist,
                'publicationMayExist',
                isTrue,
              ),
        ),
      );
      expect(thrownLease.requiresReopen, isTrue);
      expect(thrownLease.publicationUncertaintyLatchCalls, 1);
      expect(thrownLease.exportCalls, 1);
    },
  );

  test(
    'reviewed DataAsset build rejects a stale lease before access and accepts an exact uncertain receipt',
    () async {
      const targetPath = '/Game/Blueprints/Items/FootstepPreset';
      const packName = 'ReviewedFootsteps';
      const output = r'C:\Builds\ReviewedFootsteps';
      final managed = _FakeManagedLease(
        root: Directory('managed-reviewed-dataasset-build'),
        projectIdValue: revision3VoiceFixtureProjectId,
        projectRevision: 7,
        head: _head(7),
        onReviewedDataAssetBuild:
            (lease, gameRoot, receivedTarget, receivedPack, receivedOutput) {
              expect(gameRoot, r'C:\Games\Gothic Remake');
              expect(receivedTarget, targetPath);
              expect(receivedPack, packName);
              expect(receivedOutput, output);
              return _reviewedDataAssetBuildResult(
                head: lease.head,
                projectId: lease.projectId,
                projectRevision: lease.projectRevision,
                targetPath: receivedTarget,
                packName: receivedPack,
                output: receivedOutput,
                publicationUncertain: true,
              );
            },
      );
      final coordinator = CurrentProjectCoordinator(
        openManagedRevision3: (_) async => managed,
      );
      addTearDown(() async {
        await coordinator.shutdown();
        coordinator.dispose();
      });
      await coordinator.openManagedRevision3(managed.root);

      await expectLater(
        coordinator.buildCurrentRevision3ReviewedDataAsset(
          expectedRoot: managed.root.path,
          expectedProjectId: managed.projectId,
          expectedProjectRevision: 6,
          expectedHead: _head(6),
          gameRoot: r'C:\Games\Gothic Remake',
          targetPath: targetPath,
          packName: packName,
          output: output,
        ),
        throwsA(isA<Revision3DataAssetStaleCheckpointException>()),
      );
      expect(managed.reviewedDataAssetBuildCalls, 0);

      final result = await coordinator.buildCurrentRevision3ReviewedDataAsset(
        expectedRoot: managed.root.path,
        expectedProjectId: managed.projectId,
        expectedProjectRevision: 7,
        expectedHead: _head(7),
        gameRoot: r'C:\Games\Gothic Remake',
        targetPath: targetPath,
        packName: packName,
        output: output,
      );

      expect(result.publicationIsUncertain, isTrue);
      expect(
        result.receipt.relativeName,
        'gore-authoring-dataasset-build.json',
      );
      expect(result.output, output);
      expect(managed.reviewedDataAssetBuildCalls, 1);
      expect(managed.projectRevision, 7);
      expect(managed.head.canonicalJson, _head(7).canonicalJson);
    },
  );

  test(
    'reviewed DataAsset build rejects an absent lease capability without poisoning it',
    () async {
      final managed = _FakeManagedLease(
        root: Directory('managed-reviewed-dataasset-build-unsupported'),
        projectIdValue: revision3VoiceFixtureProjectId,
        projectRevision: 7,
        head: _head(7),
      );
      final coordinator = CurrentProjectCoordinator(
        openManagedRevision3: (_) async => managed,
      );
      addTearDown(() async {
        await coordinator.shutdown();
        coordinator.dispose();
      });
      await coordinator.openManagedRevision3(managed.root);

      await expectLater(
        coordinator.buildCurrentRevision3ReviewedDataAsset(
          expectedRoot: managed.root.path,
          expectedProjectId: managed.projectId,
          expectedProjectRevision: managed.projectRevision,
          expectedHead: managed.head,
          gameRoot: r'C:\Games\Gothic Remake',
          targetPath: '/Game/Blueprints/Items/FootstepPreset',
          packName: 'ReviewedFootsteps',
          output: r'C:\Builds\ReviewedFootsteps',
        ),
        throwsA(isA<CurrentProjectOperationUnsupportedException>()),
      );
      expect(managed.reviewedDataAssetBuildCalls, 0);
      expect(managed.requiresReopen, isFalse);
      expect(
        (coordinator.state as ManagedRevision3CurrentProjectState)
            .requiresReopen,
        isFalse,
      );
    },
  );

  test(
    'DataAsset list is exact-checkpoint bound before lease access',
    () async {
      final stage = _dataAssetStage();
      final managed = _FakeManagedLease(
        root: Directory('managed-dataasset-list'),
        projectIdValue: stage.projectId,
        projectRevision: 5,
        head: _head(5),
        dataAssetStages: [stage],
      );
      final coordinator = CurrentProjectCoordinator(
        openManagedRevision3: (_) async => managed,
      );
      addTearDown(() async {
        await coordinator.shutdown();
        coordinator.dispose();
      });
      await coordinator.openManagedRevision3(managed.root);

      await expectLater(
        coordinator.listCurrentRevision3DataAssetStages(
          expectedRoot: managed.root.path,
          expectedProjectId: stage.projectId,
          expectedProjectRevision: 4,
          expectedHead: _head(4),
        ),
        throwsA(isA<Revision3DataAssetStaleCheckpointException>()),
      );
      expect(managed.dataAssetListCalls, 0);

      await expectLater(
        coordinator.listCurrentRevision3DataAssetStages(
          expectedRoot: Directory('divergent-managed-clone').path,
          expectedProjectId: stage.projectId,
          expectedProjectRevision: 5,
          expectedHead: _head(5),
        ),
        throwsA(isA<Revision3DataAssetStaleCheckpointException>()),
      );
      await expectLater(
        coordinator.listCurrentRevision3DataAssetStages(
          expectedRoot: managed.root.path,
          expectedProjectId: stage.projectId,
          expectedProjectRevision: 5,
          expectedHead: _head(55),
        ),
        throwsA(isA<Revision3DataAssetStaleCheckpointException>()),
      );
      expect(managed.dataAssetListCalls, 0);

      final listed = await coordinator.listCurrentRevision3DataAssetStages(
        expectedRoot: managed.root.path,
        expectedProjectId: stage.projectId,
        expectedProjectRevision: 5,
        expectedHead: _head(5),
      );
      expect(listed, [same(stage)]);
      expect(managed.dataAssetListCalls, 1);
    },
  );

  test(
    'Voice build returns a basis receipt while publishing reopen state',
    () async {
      final managed = _FakeManagedLease(
        root: Directory('managed-voice-snapshot-build'),
        projectIdValue: revision3VoiceContentProjectId,
        projectRevision: 7,
        head: _head(7),
        onVoiceBuild: (lease, _, output) {
          final result = _voiceBuildResult(
            head: lease.head,
            projectId: lease.projectId,
            projectRevision: lease.projectRevision,
            output: output,
          );
          lease.requiresReopenValue = true;
          return result;
        },
      );
      final coordinator = CurrentProjectCoordinator(
        openManagedRevision3: (_) async => managed,
      );
      addTearDown(() async {
        await coordinator.shutdown();
        coordinator.dispose();
      });
      await coordinator.openManagedRevision3(managed.root);

      const output = r'C:\Builds\snapshot-voice';
      final result = await coordinator.buildCurrentRevision3Voice(
        expectedRoot: managed.root.path,
        expectedProjectId: managed.projectId,
        expectedProjectRevision: 7,
        expectedHead: _head(7),
        gameRoot: r'C:\Games\Gothic Remake',
        output: output,
      );

      expect(result.output, output);
      expect(
        (coordinator.state as ManagedRevision3CurrentProjectState)
            .requiresReopen,
        isTrue,
      );
    },
  );

  test(
    'DataAsset add and registry removal advance and refresh R3 state',
    () async {
      final stage = _dataAssetStage();
      late _FakeManagedLease managed;
      managed = _FakeManagedLease(
        root: Directory('managed-dataasset-write'),
        projectIdValue: stage.projectId,
        projectRevision: 4,
        head: _head(4),
        onDataAssetPublish: (lease, receiptPath) {
          expect(receiptPath, r'C:\proof\edit.gore-asset-patch.json');
          lease.projectRevision = 5;
          lease.head = _head(5);
          return Revision3DataAssetStagePublication(
            projectId: stage.projectId,
            projectRevision: 5,
            stage: stage,
            deduplicatedBlobs: 1,
          );
        },
        onDataAssetRemove: (lease, targetPath) {
          expect(targetPath, stage.targetPath);
          lease.projectRevision = 6;
          lease.head = _head(6);
          return Revision3DataAssetStageRemovalPublication(
            projectId: stage.projectId,
            projectRevision: 6,
            removed: stage,
          );
        },
      );
      final coordinator = CurrentProjectCoordinator(
        openManagedRevision3: (_) async => managed,
      );
      addTearDown(() async {
        await coordinator.shutdown();
        coordinator.dispose();
      });
      await coordinator.openManagedRevision3(managed.root);

      final added = await coordinator.addCurrentRevision3DataAssetStage(
        expectedRoot: managed.root.path,
        expectedProjectId: stage.projectId,
        expectedProjectRevision: 4,
        expectedHead: _head(4),
        patchReceiptPath: r'C:\proof\edit.gore-asset-patch.json',
      );
      expect(added.stage, same(stage));
      expect(managed.dataAssetPublishCalls, 1);
      expect(
        (coordinator.state as ManagedRevision3CurrentProjectState)
            .projectRevision,
        5,
      );

      final removed = await coordinator.removeCurrentRevision3DataAssetStage(
        expectedRoot: managed.root.path,
        expectedProjectId: stage.projectId,
        expectedProjectRevision: 5,
        expectedHead: _head(5),
        targetPath: stage.targetPath,
      );
      expect(removed.removed, same(stage));
      expect(managed.dataAssetRemoveCalls, 1);
      expect(
        (coordinator.state as ManagedRevision3CurrentProjectState)
            .projectRevision,
        6,
      );
    },
  );

  test(
    'typed DataAsset value edit is exact-checkpoint bound and refreshes state',
    () async {
      final stage = _dataAssetStage();
      final intent = _dataAssetSemanticIntent();
      late _FakeManagedLease managed;
      managed = _FakeManagedLease(
        root: Directory('managed-dataasset-semantic'),
        projectIdValue: stage.projectId,
        projectRevision: 4,
        head: _head(4),
        onDataAssetSemanticPublish: (lease, received) {
          expect(received, same(intent));
          expect(
            received.toNativeFields()['extract_receipt_path'],
            r'C:\proof\extract-receipt.v2.json',
          );
          lease.projectRevision = 5;
          lease.head = _head(5);
          return Revision3DataAssetStagePublication(
            projectId: stage.projectId,
            projectRevision: 5,
            stage: stage,
            deduplicatedBlobs: 0,
          );
        },
      );
      final coordinator = CurrentProjectCoordinator(
        openManagedRevision3: (_) async => managed,
      );
      addTearDown(() async {
        await coordinator.shutdown();
        coordinator.dispose();
      });
      await coordinator.openManagedRevision3(managed.root);

      final published = await coordinator.addCurrentRevision3DataAssetEdit(
        expectedRoot: managed.root.path,
        expectedProjectId: stage.projectId,
        expectedProjectRevision: 4,
        expectedHead: _head(4),
        intent: intent,
      );

      expect(published.stage, same(stage));
      expect(managed.dataAssetSemanticPublishCalls, 1);
      final state = coordinator.state as ManagedRevision3CurrentProjectState;
      expect(state.projectRevision, 5);
      expect(state.head.canonicalJson, _head(5).canonicalJson);

      await expectLater(
        coordinator.addCurrentRevision3DataAssetEdit(
          expectedRoot: managed.root.path,
          expectedProjectId: stage.projectId,
          expectedProjectRevision: 4,
          expectedHead: _head(4),
          intent: intent,
        ),
        throwsA(isA<Revision3DataAssetStaleCheckpointException>()),
      );
      expect(managed.dataAssetSemanticPublishCalls, 1);
    },
  );

  test(
    'installed DataAsset edit is exact-evidence bound and refreshes state',
    () async {
      final stage = _dataAssetStage();
      final initialHead = _head(4);
      final snapshot = _controllerDataAssetPackageIndexResult(
        head: initialHead,
        projectId: stage.projectId,
        projectRevision: 4,
        targetPath: stage.targetPath,
        packageIdHex: 'e54f79b8fc97323c',
      );
      final candidate = snapshot.index.candidates.single;
      final inspection = _controllerInstalledDataAssetInspectionResult(
        expectedSnapshot: snapshot,
        candidate: candidate,
      );
      final intent = DataAssetInstalledSemanticEditIntent.fromInspection(
        snapshot: snapshot,
        candidate: candidate,
        inspection: inspection,
        change: DataAssetSemanticValueEditor.fromLeaf(
          inspection.inspection.exports.single.leaves.single,
        ).changeScalar(value: '2'),
      );
      late _FakeManagedLease managed;
      managed = _FakeManagedLease(
        root: Directory('managed-installed-dataasset-edit'),
        projectIdValue: stage.projectId,
        projectRevision: 4,
        head: initialHead,
        onInstalledDataAssetEditPublish: (lease, gameRoot, received) {
          expect(gameRoot, r'D:\Games\Gothic Remake');
          expect(received, same(intent));
          expect(received.toNativeFields(), isNot(contains('target_path')));
          lease.projectRevision = 5;
          lease.head = _head(5);
          return Revision3DataAssetStagePublication(
            projectId: stage.projectId,
            projectRevision: 5,
            stage: stage,
            deduplicatedBlobs: 0,
          );
        },
      );
      final coordinator = CurrentProjectCoordinator(
        openManagedRevision3: (_) async => managed,
      );
      addTearDown(() async {
        await coordinator.shutdown();
        coordinator.dispose();
      });
      await coordinator.openManagedRevision3(managed.root);

      final published = await coordinator
          .addCurrentRevision3InstalledDataAssetEdit(
            expectedRoot: managed.root.path,
            expectedProjectId: stage.projectId,
            expectedProjectRevision: 4,
            expectedHead: initialHead,
            gameRoot: r'D:\Games\Gothic Remake',
            intent: intent,
          );

      expect(published.stage, same(stage));
      expect(managed.installedDataAssetEditPublishCalls, 1);
      expect(managed.installedDataAssetEditIntents.single, same(intent));
      final state = coordinator.state as ManagedRevision3CurrentProjectState;
      expect(state.projectRevision, 5);
      expect(state.head.canonicalJson, _head(5).canonicalJson);
    },
  );

  test(
    'reviewed installed DataAsset edit forwards the closed intent and refreshes state',
    () async {
      final stage = _dataAssetStage(targetPath: _controllerReviewedWolfTarget);
      final initialHead = _head(4);
      final intent = _controllerReviewedDataAssetIntent(
        head: initialHead,
        projectId: stage.projectId,
        projectRevision: 4,
      );
      late _FakeManagedLease managed;
      managed = _FakeManagedLease(
        root: Directory('managed-reviewed-installed-dataasset-edit'),
        projectIdValue: stage.projectId,
        projectRevision: 4,
        head: initialHead,
        onReviewedInstalledDataAssetEditPublish: (lease, gameRoot, received) {
          expect(gameRoot, r'D:\Games\Gothic Remake');
          expect(received, same(intent));
          expect(received.toNativeFields().keys, <String>[
            'candidate_ordinal',
            'expected_package_index_seal',
            'expected_source_snapshot_seal',
            'reviewed_edit',
          ]);
          lease.projectRevision = 5;
          lease.head = _head(5);
          return Revision3DataAssetStagePublication(
            projectId: stage.projectId,
            projectRevision: 5,
            stage: stage,
            deduplicatedBlobs: 0,
          );
        },
      );
      final coordinator = CurrentProjectCoordinator(
        openManagedRevision3: (_) async => managed,
      );
      addTearDown(() async {
        await coordinator.shutdown();
        coordinator.dispose();
      });
      await coordinator.openManagedRevision3(managed.root);

      final published = await coordinator
          .addCurrentRevision3ReviewedInstalledDataAssetEdit(
            expectedRoot: managed.root.path,
            expectedProjectId: stage.projectId,
            expectedProjectRevision: 4,
            expectedHead: initialHead,
            gameRoot: r'D:\Games\Gothic Remake',
            intent: intent,
          );

      expect(published.stage, same(stage));
      expect(managed.reviewedInstalledDataAssetEditPublishCalls, 1);
      expect(
        managed.reviewedInstalledDataAssetEditIntents.single,
        same(intent),
      );
      final state = coordinator.state as ManagedRevision3CurrentProjectState;
      expect(state.projectRevision, 5);
      expect(state.head.canonicalJson, _head(5).canonicalJson);
    },
  );

  test(
    'reviewed installed DataAsset native rejections distinguish stale evidence from preparation failures',
    () async {
      final scenarios = <({String code, bool staleEvidence})>[
        (
          code:
              'AUTHORING_REVISION3_REVIEWED_INSTALLED_DATAASSET_EDIT_CANDIDATE_INVALID',
          staleEvidence: true,
        ),
        (
          code:
              'AUTHORING_REVISION3_REVIEWED_INSTALLED_DATAASSET_EDIT_MATCH_INVALID',
          staleEvidence: true,
        ),
        (
          code:
              'AUTHORING_REVISION3_REVIEWED_INSTALLED_DATAASSET_EDIT_REQUEST_INVALID',
          staleEvidence: false,
        ),
        (
          code: 'AUTHORING_REVISION3_REVIEWED_INSTALLED_DATAASSET_EDIT_INVALID',
          staleEvidence: false,
        ),
      ];

      for (var index = 0; index < scenarios.length; index++) {
        final scenario = scenarios[index];
        final stage = _dataAssetStage(
          targetPath: _controllerReviewedWolfTarget,
        );
        final initialHead = _head(4);
        final intent = _controllerReviewedDataAssetIntent(
          head: initialHead,
          projectId: stage.projectId,
          projectRevision: 4,
        );
        final managed = _FakeManagedLease(
          root: Directory('managed-reviewed-installed-dataasset-error-$index'),
          projectIdValue: stage.projectId,
          projectRevision: 4,
          head: initialHead,
          onReviewedInstalledDataAssetEditPublish: (_, _, _) =>
              throw ModFfiException(
                command:
                    'authoring_store_prepare_revision3_reviewed_installed_dataasset_edit_v1',
                code: scenario.code,
                message: 'fake reviewed installed DataAsset native rejection',
              ),
        );
        final coordinator = CurrentProjectCoordinator(
          openManagedRevision3: (_) async => managed,
        );
        addTearDown(() async {
          await coordinator.shutdown();
          coordinator.dispose();
        });
        await coordinator.openManagedRevision3(managed.root);

        final operation = coordinator
            .addCurrentRevision3ReviewedInstalledDataAssetEdit(
              expectedRoot: managed.root.path,
              expectedProjectId: stage.projectId,
              expectedProjectRevision: 4,
              expectedHead: initialHead,
              gameRoot: r'D:\Games\Gothic Remake',
              intent: intent,
            );
        if (scenario.staleEvidence) {
          await expectLater(
            operation,
            throwsA(
              isA<
                Revision3InstalledDataAssetEditSourceEvidenceStaleException
              >(),
            ),
          );
        } else {
          await expectLater(
            operation,
            throwsA(
              isA<Revision3InstalledDataAssetEditRejectedException>().having(
                (error) => error.reason,
                'reason',
                Revision3InstalledDataAssetEditRejectionReason
                    .preparationFailed,
              ),
            ),
          );
        }

        expect(
          managed.reviewedInstalledDataAssetEditPublishCalls,
          1,
          reason: scenario.code,
        );
        final state = coordinator.state as ManagedRevision3CurrentProjectState;
        expect(state.projectRevision, 4, reason: scenario.code);
        expect(state.requiresReopen, isFalse, reason: scenario.code);
      }
    },
  );

  test(
    'installed DataAsset source drift closes evidence without poisoning the project',
    () async {
      final stage = _dataAssetStage();
      final initialHead = _head(4);
      final snapshot = _controllerDataAssetPackageIndexResult(
        head: initialHead,
        projectId: stage.projectId,
        projectRevision: 4,
        targetPath: stage.targetPath,
        packageIdHex: 'e54f79b8fc97323c',
      );
      final candidate = snapshot.index.candidates.single;
      final inspection = _controllerInstalledDataAssetInspectionResult(
        expectedSnapshot: snapshot,
        candidate: candidate,
      );
      final intent = DataAssetInstalledSemanticEditIntent.fromInspection(
        snapshot: snapshot,
        candidate: candidate,
        inspection: inspection,
        change: DataAssetSemanticValueEditor.fromLeaf(
          inspection.inspection.exports.single.leaves.single,
        ).changeScalar(value: '2'),
      );
      final managed = _FakeManagedLease(
        root: Directory('managed-installed-dataasset-edit-source-drift'),
        projectIdValue: stage.projectId,
        projectRevision: 4,
        head: initialHead,
        onInstalledDataAssetEditPublish: (_, _, _) => throw const ModFfiException(
          command:
              'authoring_store_prepare_revision3_installed_dataasset_edit_v1',
          code:
              'AUTHORING_REVISION3_INSTALLED_DATAASSET_EDIT_SOURCE_SNAPSHOT_MISMATCH',
          message: 'fake installed source drift',
        ),
      );
      final coordinator = CurrentProjectCoordinator(
        openManagedRevision3: (_) async => managed,
      );
      addTearDown(() async {
        await coordinator.shutdown();
        coordinator.dispose();
      });
      await coordinator.openManagedRevision3(managed.root);

      await expectLater(
        coordinator.addCurrentRevision3InstalledDataAssetEdit(
          expectedRoot: managed.root.path,
          expectedProjectId: stage.projectId,
          expectedProjectRevision: 4,
          expectedHead: initialHead,
          gameRoot: r'D:\Games\Gothic Remake',
          intent: intent,
        ),
        throwsA(
          isA<Revision3InstalledDataAssetEditSourceEvidenceStaleException>(),
        ),
      );
      expect(managed.installedDataAssetEditPublishCalls, 1);
      final state = coordinator.state as ManagedRevision3CurrentProjectState;
      expect(state.projectRevision, 4);
      expect(state.requiresReopen, isFalse);

      final stagedTargetLease = _FakeManagedLease(
        root: Directory('managed-installed-dataasset-edit-target-exists'),
        projectIdValue: stage.projectId,
        projectRevision: 4,
        head: initialHead,
        onInstalledDataAssetEditPublish: (_, _, _) => throw const ModFfiException(
          command:
              'authoring_store_prepare_revision3_installed_dataasset_edit_v1',
          code: 'AUTHORING_REVISION3_DATAASSET_TARGET_EXISTS',
          message: 'fake target already staged',
        ),
      );
      final stagedTargetCoordinator = CurrentProjectCoordinator(
        openManagedRevision3: (_) async => stagedTargetLease,
      );
      addTearDown(() async {
        await stagedTargetCoordinator.shutdown();
        stagedTargetCoordinator.dispose();
      });
      await stagedTargetCoordinator.openManagedRevision3(
        stagedTargetLease.root,
      );
      await expectLater(
        stagedTargetCoordinator.addCurrentRevision3InstalledDataAssetEdit(
          expectedRoot: stagedTargetLease.root.path,
          expectedProjectId: stage.projectId,
          expectedProjectRevision: 4,
          expectedHead: initialHead,
          gameRoot: r'D:\Games\Gothic Remake',
          intent: intent,
        ),
        throwsA(
          isA<Revision3InstalledDataAssetEditRejectedException>().having(
            (error) => error.reason,
            'reason',
            Revision3InstalledDataAssetEditRejectionReason.targetAlreadyStaged,
          ),
        ),
      );
      final stagedState =
          stagedTargetCoordinator.state as ManagedRevision3CurrentProjectState;
      expect(stagedState.projectRevision, 4);
      expect(stagedState.requiresReopen, isFalse);
    },
  );

  test(
    'installed DataAsset local preparation errors reject without poisoning the project',
    () async {
      final stage = _dataAssetStage();
      final initialHead = _head(4);
      final snapshot = _controllerDataAssetPackageIndexResult(
        head: initialHead,
        projectId: stage.projectId,
        projectRevision: 4,
        targetPath: stage.targetPath,
        packageIdHex: 'e54f79b8fc97323c',
      );
      final candidate = snapshot.index.candidates.single;
      final inspection = _controllerInstalledDataAssetInspectionResult(
        expectedSnapshot: snapshot,
        candidate: candidate,
      );
      final intent = DataAssetInstalledSemanticEditIntent.fromInspection(
        snapshot: snapshot,
        candidate: candidate,
        inspection: inspection,
        change: DataAssetSemanticValueEditor.fromLeaf(
          inspection.inspection.exports.single.leaves.single,
        ).changeScalar(value: '2'),
      );
      final errors = <Object>[
        ArgumentError('fake local request preflight rejection'),
        const FormatException('fake local response preflight rejection'),
      ];

      for (var index = 0; index < errors.length; index++) {
        final error = errors[index];
        final managed = _FakeManagedLease(
          root: Directory('managed-installed-dataasset-local-error-$index'),
          projectIdValue: stage.projectId,
          projectRevision: 4,
          head: initialHead,
          onInstalledDataAssetEditPublish: (_, _, _) => throw error,
        );
        final coordinator = CurrentProjectCoordinator(
          openManagedRevision3: (_) async => managed,
        );
        addTearDown(() async {
          await coordinator.shutdown();
          coordinator.dispose();
        });
        await coordinator.openManagedRevision3(managed.root);

        await expectLater(
          coordinator.addCurrentRevision3InstalledDataAssetEdit(
            expectedRoot: managed.root.path,
            expectedProjectId: stage.projectId,
            expectedProjectRevision: 4,
            expectedHead: initialHead,
            gameRoot: r'D:\Games\Gothic Remake',
            intent: intent,
          ),
          throwsA(
            isA<Revision3InstalledDataAssetEditRejectedException>().having(
              (error) => error.reason,
              'reason',
              Revision3InstalledDataAssetEditRejectionReason.preparationFailed,
            ),
          ),
        );
        final state = coordinator.state as ManagedRevision3CurrentProjectState;
        expect(state.projectRevision, 4);
        expect(state.requiresReopen, isFalse);
      }
    },
  );

  test(
    'DataAsset mutations reject divergent roots and heads before lease access',
    () async {
      final stage = _dataAssetStage();
      final managed = _FakeManagedLease(
        root: Directory('managed-dataasset-exact-mutation'),
        projectIdValue: stage.projectId,
        projectRevision: 5,
        head: _head(5),
      );
      final coordinator = CurrentProjectCoordinator(
        openManagedRevision3: (_) async => managed,
      );
      addTearDown(() async {
        await coordinator.shutdown();
        coordinator.dispose();
      });
      await coordinator.openManagedRevision3(managed.root);

      await expectLater(
        coordinator.addCurrentRevision3DataAssetStage(
          expectedRoot: managed.root.path,
          expectedProjectId: stage.projectId,
          expectedProjectRevision: 5,
          expectedHead: _head(55),
          patchReceiptPath: r'C:\proof\edit.gore-asset-patch.json',
        ),
        throwsA(isA<Revision3DataAssetStaleCheckpointException>()),
      );
      await expectLater(
        coordinator.removeCurrentRevision3DataAssetStage(
          expectedRoot: Directory('divergent-managed-clone').path,
          expectedProjectId: stage.projectId,
          expectedProjectRevision: 5,
          expectedHead: _head(5),
          targetPath: stage.targetPath,
        ),
        throwsA(isA<Revision3DataAssetStaleCheckpointException>()),
      );

      expect(managed.dataAssetPublishCalls, 0);
      expect(managed.dataAssetRemoveCalls, 0);
    },
  );

  test(
    'poisoned DataAsset failure locks retries and refreshes state',
    () async {
      final stage = _dataAssetStage();
      final managed = _FakeManagedLease(
        root: Directory('managed-dataasset-poisoned'),
        projectIdValue: stage.projectId,
        projectRevision: 4,
        head: _head(4),
        onDataAssetPublish: (lease, _) {
          lease.requiresReopenValue = true;
          throw StateError('injected DataAsset verification failure');
        },
      );
      final coordinator = CurrentProjectCoordinator(
        openManagedRevision3: (_) async => managed,
      );
      addTearDown(() async {
        await coordinator.shutdown();
        coordinator.dispose();
      });
      await coordinator.openManagedRevision3(managed.root);

      Future<void> add() async {
        await coordinator.addCurrentRevision3DataAssetStage(
          expectedRoot: managed.root.path,
          expectedProjectId: stage.projectId,
          expectedProjectRevision: 4,
          expectedHead: _head(4),
          patchReceiptPath: r'C:\proof\edit.gore-asset-patch.json',
        );
      }

      await expectLater(
        add(),
        throwsA(isA<Revision3DataAssetRequiresReopenException>()),
      );
      expect(managed.dataAssetPublishCalls, 1);
      expect(
        (coordinator.state as ManagedRevision3CurrentProjectState)
            .requiresReopen,
        isTrue,
      );
      await expectLater(
        add(),
        throwsA(isA<Revision3DataAssetRequiresReopenException>()),
      );
      expect(managed.dataAssetPublishCalls, 1);
    },
  );

  for (final kind in AuthoringStoryDraftKind.values) {
    test(
      '${kind.wireName} removal binds the exact visible tuple and refreshes state',
      () async {
        final managed = _FakeStoryDraftRemovalManagedLease(
          root: Directory('managed-story-remove-${kind.wireName}'),
          projectIdValue: revision3VoiceFixtureProjectId,
          projectRevision: 7,
          head: _head(7),
        );
        final coordinator = CurrentProjectCoordinator(
          openManagedRevision3: (_) async => managed,
        );
        addTearDown(() async {
          await coordinator.shutdown();
          coordinator.dispose();
        });
        final visible = await coordinator.openManagedRevision3(managed.root);
        final draftId = kind == AuthoringStoryDraftKind.npcDraft
            ? '11111111111111111111111111111111'
            : '22222222222222222222222222222222';
        final moduleId = kind == AuthoringStoryDraftKind.npcDraft
            ? '33333333333333333333333333333333'
            : '44444444444444444444444444444444';

        final publication = await coordinator.removeCurrentRevision3StoryDraft(
          expectedRoot: visible.root.path,
          expectedProjectId: visible.projectId,
          expectedProjectRevision: visible.projectRevision,
          expectedHead: visible.head,
          draftId: draftId,
          draftKind: kind,
          expectedDraftRevision: 3,
          scriptModuleId: moduleId,
          expectedScriptModuleRevision: 4,
        );

        expect(publication.head.canonicalJson, _head(8).canonicalJson);
        expect(publication.projectRevision, 8);
        expect(publication.removedDraftId, draftId);
        expect(publication.removedDraftKind, kind);
        expect(publication.removedScriptModuleId, moduleId);
        expect(managed.storyDraftRemovalCalls, 1);
        expect(managed.receivedDraftIds, <String>[draftId]);
        expect(managed.receivedDraftKinds, <AuthoringStoryDraftKind>[kind]);
        final refreshed =
            coordinator.state as ManagedRevision3CurrentProjectState;
        expect(refreshed.projectRevision, 8);
        expect(refreshed.head.canonicalJson, _head(8).canonicalJson);
        expect(refreshed.requiresReopen, isFalse);
      },
    );
  }

  test(
    'Story Draft removal rejects stale, unsupported, and disabled capabilities before mutation',
    () async {
      final managed = _FakeStoryDraftRemovalManagedLease(
        root: Directory('managed-story-remove-stale'),
        projectIdValue: revision3VoiceFixtureProjectId,
        projectRevision: 7,
        head: _head(7),
      );
      final coordinator = CurrentProjectCoordinator(
        openManagedRevision3: (_) async => managed,
      );
      addTearDown(() async {
        await coordinator.shutdown();
        coordinator.dispose();
      });
      final visible = await coordinator.openManagedRevision3(managed.root);

      await expectLater(
        coordinator.removeCurrentRevision3StoryDraft(
          expectedRoot: visible.root.path,
          expectedProjectId: visible.projectId,
          expectedProjectRevision: 6,
          expectedHead: _head(6),
          draftId: '11111111111111111111111111111111',
          draftKind: AuthoringStoryDraftKind.npcDraft,
          expectedDraftRevision: 3,
          scriptModuleId: '33333333333333333333333333333333',
          expectedScriptModuleRevision: 4,
        ),
        throwsA(isA<Revision3StoryDraftRemovalStaleCheckpointException>()),
      );
      expect(managed.storyDraftRemovalCalls, 0);

      final unsupported = _FakeManagedLease(
        root: Directory('managed-story-remove-unsupported'),
        projectIdValue: revision3VoiceFixtureProjectId,
        projectRevision: 7,
        head: _head(7),
      );
      final unsupportedCoordinator = CurrentProjectCoordinator(
        openManagedRevision3: (_) async => unsupported,
      );
      addTearDown(() async {
        await unsupportedCoordinator.shutdown();
        unsupportedCoordinator.dispose();
      });
      final unsupportedVisible = await unsupportedCoordinator
          .openManagedRevision3(unsupported.root);
      await expectLater(
        unsupportedCoordinator.removeCurrentRevision3StoryDraft(
          expectedRoot: unsupportedVisible.root.path,
          expectedProjectId: unsupportedVisible.projectId,
          expectedProjectRevision: unsupportedVisible.projectRevision,
          expectedHead: unsupportedVisible.head,
          draftId: '11111111111111111111111111111111',
          draftKind: AuthoringStoryDraftKind.npcDraft,
          expectedDraftRevision: 3,
          scriptModuleId: '33333333333333333333333333333333',
          expectedScriptModuleRevision: 4,
        ),
        throwsA(isA<Revision3StoryDraftRemovalUnsupportedException>()),
      );

      final disabled = _FakeStoryDraftRemovalManagedLease(
        root: Directory('managed-story-remove-disabled'),
        projectIdValue: revision3VoiceFixtureProjectId,
        projectRevision: 7,
        head: _head(7),
        supportsRemoval: false,
      );
      final disabledCoordinator = CurrentProjectCoordinator(
        openManagedRevision3: (_) async => disabled,
      );
      addTearDown(() async {
        await disabledCoordinator.shutdown();
        disabledCoordinator.dispose();
      });
      final disabledVisible = await disabledCoordinator.openManagedRevision3(
        disabled.root,
      );
      await expectLater(
        disabledCoordinator.removeCurrentRevision3StoryDraft(
          expectedRoot: disabledVisible.root.path,
          expectedProjectId: disabledVisible.projectId,
          expectedProjectRevision: disabledVisible.projectRevision,
          expectedHead: disabledVisible.head,
          draftId: '11111111111111111111111111111111',
          draftKind: AuthoringStoryDraftKind.npcDraft,
          expectedDraftRevision: 3,
          scriptModuleId: '33333333333333333333333333333333',
          expectedScriptModuleRevision: 4,
        ),
        throwsA(isA<Revision3StoryDraftRemovalUnsupportedException>()),
      );
      expect(disabled.storyDraftRemovalCalls, 0);
    },
  );

  test(
    'every correctable Story Draft removal code is a retryable rejection',
    () async {
      const codes = <String>{
        'AUTHORING_REVISION3_STORY_DRAFT_REMOVE_INPUT_LIMIT',
        'AUTHORING_REVISION3_STORY_DRAFT_REMOVE_PROJECT_CONFLICT',
        'AUTHORING_REVISION3_STORY_DRAFT_REMOVE_TARGET_CONFLICT',
        'AUTHORING_REVISION3_STORY_DRAFT_REMOVE_DRAFT_CONFLICT',
        'AUTHORING_REVISION3_STORY_DRAFT_REMOVE_MODULE_CONFLICT',
        'AUTHORING_REVISION3_STORY_DRAFT_REMOVE_OWNERSHIP_CONFLICT',
        'AUTHORING_REVISION3_STORY_DRAFT_REMOVE_DRAFT_REFERENCED',
        'AUTHORING_REVISION3_STORY_DRAFT_REMOVE_MODULE_REFERENCED',
        'AUTHORING_REVISION3_STORY_DRAFT_REMOVE_REFERENCE_LIMIT',
        'AUTHORING_REVISION3_STORY_DRAFT_REMOVE_REVISION_LIMIT',
        'AUTHORING_REVISION3_STORY_DRAFT_REMOVE_PROJECT_LIMIT',
        'AUTHORING_REVISION3_STORY_DRAFT_REMOVE_REQUEST_INVALID',
        'AUTHORING_REVISION3_STORY_DRAFT_REMOVE_REQUEST_LIMIT',
        'AUTHORING_REVISION3_STORY_DRAFT_REMOVE_REQUEST_REJECTED',
        'AUTHORING_REVISION3_STORY_DRAFT_REMOVE_RESPONSE_LIMIT',
        'AUTHORING_REVISION3_STORY_DRAFT_REMOVE_SIGNED_WIRE_LIMIT',
      };
      final managed = _FakeStoryDraftRemovalManagedLease(
        root: Directory('managed-story-remove-retryable'),
        projectIdValue: revision3VoiceFixtureProjectId,
        projectRevision: 7,
        head: _head(7),
      );
      final coordinator = CurrentProjectCoordinator(
        openManagedRevision3: (_) async => managed,
      );
      addTearDown(() async {
        await coordinator.shutdown();
        coordinator.dispose();
      });
      final visible = await coordinator.openManagedRevision3(managed.root);

      for (final code in codes) {
        final native = ModFfiException(
          command: 'authoring_store_prepare_remove_revision3_story_draft_v1',
          code: code,
          message: 'correctable fake rejection',
        );
        managed.nextStoryDraftRemovalError = native;
        await expectLater(
          coordinator.removeCurrentRevision3StoryDraft(
            expectedRoot: visible.root.path,
            expectedProjectId: visible.projectId,
            expectedProjectRevision: visible.projectRevision,
            expectedHead: visible.head,
            draftId: '11111111111111111111111111111111',
            draftKind: AuthoringStoryDraftKind.npcDraft,
            expectedDraftRevision: 3,
            scriptModuleId: '33333333333333333333333333333333',
            expectedScriptModuleRevision: 4,
          ),
          throwsA(
            isA<Revision3StoryDraftRemovalRejectedException>()
                .having((error) => error.code, 'code', code)
                .having((error) => error.cause, 'cause', same(native)),
          ),
        );
        expect(managed.requiresReopen, isFalse, reason: code);
        expect(managed.storyDraftRemovalRelatchCalls, 0, reason: code);
      }
    },
  );

  test(
    'uncertain Story Draft removal failures always latch requires-reopen',
    () async {
      const codes = <String>{
        'AUTHORING_REVISION3_STORY_DRAFT_REMOVE_HEAD_CONFLICT',
        'AUTHORING_REVISION3_STORY_DRAFT_REMOVE_HEAD_MISSING',
        'AUTHORING_REVISION3_STORY_DRAFT_REMOVE_PROJECT_INVALID',
        'AUTHORING_REVISION3_STORY_DRAFT_REMOVE_INVARIANT',
        'AUTHORING_REVISION3_STORY_DRAFT_REMOVE_STORE_IO',
        ModFfiException.malformedNativeResponseCode,
        'AUTHORING_REVISION3_STORY_DRAFT_REMOVE_FUTURE_UNKNOWN',
      };
      for (final code in codes) {
        final managed = _FakeStoryDraftRemovalManagedLease(
          root: Directory('managed-story-remove-poison-$code'),
          projectIdValue: revision3VoiceFixtureProjectId,
          projectRevision: 7,
          head: _head(7),
          nextStoryDraftRemovalError: ModFfiException(
            command: 'authoring_store_prepare_remove_revision3_story_draft_v1',
            code: code,
            message: 'uncertain fake failure',
          ),
        );
        final coordinator = CurrentProjectCoordinator(
          openManagedRevision3: (_) async => managed,
        );
        addTearDown(() async {
          await coordinator.shutdown();
          coordinator.dispose();
        });
        final visible = await coordinator.openManagedRevision3(managed.root);

        await expectLater(
          coordinator.removeCurrentRevision3StoryDraft(
            expectedRoot: visible.root.path,
            expectedProjectId: visible.projectId,
            expectedProjectRevision: visible.projectRevision,
            expectedHead: visible.head,
            draftId: '11111111111111111111111111111111',
            draftKind: AuthoringStoryDraftKind.npcDraft,
            expectedDraftRevision: 3,
            scriptModuleId: '33333333333333333333333333333333',
            expectedScriptModuleRevision: 4,
          ),
          throwsA(
            isA<Revision3StoryDraftRemovalRequiresReopenException>().having(
              (error) => error.cause,
              'cause',
              isA<ModFfiException>().having(
                (error) => error.code,
                'code',
                code,
              ),
            ),
          ),
        );
        expect(managed.requiresReopen, isTrue, reason: code);
        expect(managed.storyDraftRemovalRelatchCalls, 1, reason: code);
        expect(
          (coordinator.state as ManagedRevision3CurrentProjectState)
              .requiresReopen,
          isTrue,
          reason: code,
        );
      }
    },
  );

  test(
    'Story Draft removal receipt mismatch is post-publication uncertainty',
    () async {
      final managed = _FakeStoryDraftRemovalManagedLease(
        root: Directory('managed-story-remove-receipt-mismatch'),
        projectIdValue: revision3VoiceFixtureProjectId,
        projectRevision: 7,
        head: _head(7),
        receiptMismatch: true,
      );
      final coordinator = CurrentProjectCoordinator(
        openManagedRevision3: (_) async => managed,
      );
      addTearDown(() async {
        await coordinator.shutdown();
        coordinator.dispose();
      });
      final visible = await coordinator.openManagedRevision3(managed.root);

      await expectLater(
        coordinator.removeCurrentRevision3StoryDraft(
          expectedRoot: visible.root.path,
          expectedProjectId: visible.projectId,
          expectedProjectRevision: visible.projectRevision,
          expectedHead: visible.head,
          draftId: '11111111111111111111111111111111',
          draftKind: AuthoringStoryDraftKind.npcDraft,
          expectedDraftRevision: 3,
          scriptModuleId: '33333333333333333333333333333333',
          expectedScriptModuleRevision: 4,
        ),
        throwsA(isA<Revision3StoryDraftRemovalRequiresReopenException>()),
      );
      expect(managed.projectRevision, 8);
      expect(managed.requiresReopen, isTrue);
      expect(managed.storyDraftRemovalRelatchCalls, 1);
    },
  );

  test(
    'authenticated history read and restore advance the visible project',
    () async {
      const projectId = '71717171717171717171717171717171';
      final projectJson = _recoveryProjectJson(
        projectId: projectId,
        revision: 7,
      );
      final currentHead = _recoveryHead(projectJson);
      late final _FakeHistoryManagedLease managed;
      final history = Revision3ProjectHistorySnapshot(
        basisHead: currentHead,
        projectId: projectId,
        currentRevision: 7,
        entries: <Revision3ProjectHistoryEntry>[
          Revision3ProjectHistoryEntry(
            head: currentHead,
            projectId: projectId,
            projectRevision: 7,
            isCurrent: true,
          ),
          Revision3ProjectHistoryEntry(
            head: _head(6),
            projectId: projectId,
            projectRevision: 6,
            isCurrent: false,
          ),
        ],
        historyTruncated: false,
      );
      managed = _FakeHistoryManagedLease(
        root: Directory('managed-history'),
        projectIdValue: projectId,
        projectRevision: 7,
        head: currentHead,
        canonicalProjectJson: projectJson,
        history: history,
        onRestore: (lease, expected, target) {
          expect(identical(expected, history), isTrue);
          expect(target.projectRevision, 6);
          final nextJson = _recoveryProjectJson(
            projectId: projectId,
            revision: 8,
          );
          final nextHead = _recoveryHead(nextJson);
          lease
            ..projectRevision = 8
            ..canonicalProjectJson = nextJson
            ..head = nextHead;
          return ManagedRevision3ProjectHistoryRestoreCheckpoint(
            previousHead: currentHead,
            head: nextHead,
            projectJson: nextJson,
            projectId: projectId,
            previousProjectRevision: 7,
            projectRevision: 8,
            restoredFromHead: target.head,
            restoredFromRevision: target.projectRevision,
          );
        },
      );
      final coordinator = CurrentProjectCoordinator(
        openManagedRevision3: (_) async => managed,
      );
      addTearDown(() async {
        await coordinator.shutdown();
        coordinator.dispose();
      });
      final visible = await coordinator.openManagedRevision3(managed.root);

      final loaded = await coordinator.readCurrentRevision3ProjectHistory(
        expectedRoot: visible.root.path,
        expectedProjectId: visible.projectId,
        expectedProjectRevision: visible.projectRevision,
        expectedHead: visible.head,
      );
      expect(identical(loaded, history), isTrue);
      final publication = await coordinator
          .restoreCurrentRevision3ProjectHistory(
            expectedRoot: visible.root.path,
            expectedProjectId: visible.projectId,
            expectedProjectRevision: visible.projectRevision,
            expectedHead: visible.head,
            expectedHistory: loaded,
            target: loaded.entries[1],
          );

      expect(publication.projectRevision, 8);
      expect(publication.restoredFromRevision, 6);
      expect(managed.historyReadCalls, 1);
      expect(managed.historyRestoreCalls, 1);
      final refreshed =
          coordinator.state as ManagedRevision3CurrentProjectState;
      expect(refreshed.projectRevision, 8);
      expect(refreshed.head.canonicalJson, managed.head.canonicalJson);
      expect(refreshed.requiresReopen, isFalse);
    },
  );

  test('stale history tuple reaches no lease capability', () async {
    const projectId = '72727272727272727272727272727272';
    final projectJson = _recoveryProjectJson(projectId: projectId, revision: 3);
    final currentHead = _recoveryHead(projectJson);
    final history = Revision3ProjectHistorySnapshot(
      basisHead: currentHead,
      projectId: projectId,
      currentRevision: 3,
      entries: <Revision3ProjectHistoryEntry>[
        Revision3ProjectHistoryEntry(
          head: currentHead,
          projectId: projectId,
          projectRevision: 3,
          isCurrent: true,
        ),
      ],
      historyTruncated: false,
    );
    final managed = _FakeHistoryManagedLease(
      root: Directory('managed-history-stale'),
      projectIdValue: projectId,
      projectRevision: 3,
      head: currentHead,
      canonicalProjectJson: projectJson,
      history: history,
      onRestore: (_, _, _) => throw StateError('must not restore'),
    );
    final coordinator = CurrentProjectCoordinator(
      openManagedRevision3: (_) async => managed,
    );
    addTearDown(() async {
      await coordinator.shutdown();
      coordinator.dispose();
    });
    final visible = await coordinator.openManagedRevision3(managed.root);

    await expectLater(
      coordinator.readCurrentRevision3ProjectHistory(
        expectedRoot: visible.root.path,
        expectedProjectId: visible.projectId,
        expectedProjectRevision: visible.projectRevision + 1,
        expectedHead: visible.head,
      ),
      throwsA(isA<Revision3ProjectHistoryStaleCheckpointException>()),
    );
    expect(managed.historyReadCalls, 0);
    expect(managed.historyRestoreCalls, 0);
  });

  test('content read rejects absent and legacy current projects', () async {
    final empty = CurrentProjectCoordinator(
      openManagedRevision3: (_) async => throw UnimplementedError(),
    );
    addTearDown(() async {
      await empty.shutdown();
      empty.dispose();
    });
    await expectLater(
      empty.readCurrentRevision3ContentIndex(),
      throwsA(isA<NoCurrentProjectException>()),
    );

    final legacy = CurrentProjectCoordinator(
      initialLegacy: _FakeLegacyLease(path: 'legacy.goremod'),
      openManagedRevision3: (_) async => throw UnimplementedError(),
    );
    addTearDown(() async {
      await legacy.shutdown();
      legacy.dispose();
    });
    await expectLater(
      legacy.readCurrentRevision3ContentIndex(),
      throwsA(isA<CurrentProjectOperationUnsupportedException>()),
    );
  });
}

final class _FakeLegacyLease implements LegacyCurrentProjectLease {
  _FakeLegacyLease({
    this.path,
    this.hasUnsavedChanges = false,
    this.onSave,
    this.onOpenFromPath,
    this.onClose,
  });

  String? path;
  @override
  final bool hasUnsavedChanges;
  final FutureOr<void> Function()? onSave;
  final FutureOr<void> Function(String path)? onOpenFromPath;
  final FutureOr<void> Function()? onClose;
  int saveCalls = 0;
  int saveToPathCalls = 0;
  int openFromPathCalls = 0;
  int newProjectCalls = 0;
  int closeCalls = 0;

  @override
  String? get currentPath => path;

  @override
  Future<void> saveCurrent() async {
    saveCalls++;
    await onSave?.call();
  }

  @override
  Future<void> saveToPath(String path) async {
    saveToPathCalls++;
    this.path = path;
  }

  @override
  Future<void> openFromPath(String path) async {
    openFromPathCalls++;
    await onOpenFromPath?.call(path);
    this.path = path;
  }

  @override
  Future<void> newProject() async {
    newProjectCalls++;
    path = null;
  }

  @override
  Future<void> close() async {
    closeCalls++;
    await onClose?.call();
  }
}

AuthoringRevision3QuestTransitionsSeed _questTransitionsSeed(
  Revision3QuestOutlineFixture fixture,
) => AuthoringRevision3QuestTransitionsSeed.forProject(
  currentProjectJson: fixture.projectJson,
  questId: revision3QuestOutlineQuestId,
  expectedQuestRevision: fixture.questRevision,
  expectedModuleId: revision3QuestOutlineModuleId,
  expectedModuleRevision: fixture.moduleRevision,
);

Future<Revision3QuestTransitionsEditTechnicalPlan> _questTransitionsPlan(
  Revision3QuestOutlineFixture fixture,
) async {
  final index = fixture.contentIndex();
  final seed = _questTransitionsSeed(fixture);
  Revision3QuestTransitionsEditTechnicalPlan? result;
  final service = Revision3QuestTransitionsAuthoringService(
    loadSeed:
        ({
          required questId,
          required expectedQuestRevision,
          required expectedModuleId,
          required expectedModuleRevision,
        }) async => seed,
    publishTechnicalPlan: ({required plan}) async {
      result = plan;
      return Revision3QuestTransitionsEditPublication(
        projectId: revision3QuestOutlineProjectId,
        projectRevision: fixture.projectRevision + 1,
        questId: revision3QuestOutlineQuestId,
        moduleId: revision3QuestOutlineModuleId,
        questRevision: fixture.questRevision + 1,
        moduleRevision: fixture.moduleRevision + 1,
        transitionPlanSeal: plan.transitionPlan.contentSeal,
      );
    },
  );
  final checkpoint = await service.load(
    index: index,
    quest: index.entityById(revision3QuestOutlineQuestId)!,
  );
  await service.publish(
    checkpoint: checkpoint,
    transitionPlan:
        Revision3QuestTransitionsAuthoringService.sequentialTemplate(
          seed.transitionPlan,
        ),
  );
  return result!;
}

AuthoringRevision3QuestContextSeed _questContextSeed(
  Revision3QuestOutlineFixture fixture,
) => AuthoringRevision3QuestContextSeed.forProject(
  currentProjectJson: fixture.projectJson,
  questId: revision3QuestOutlineQuestId,
  expectedQuestRevision: fixture.questRevision,
  expectedModuleId: revision3QuestOutlineModuleId,
  expectedModuleRevision: fixture.moduleRevision,
  expectedParentRuntimeClass: 'UQuest_SwampCamp_SCChapter2',
  expectedGiverRuntimeUniqueName: 'OM_GRD_Asghan_263',
);

Future<Revision3QuestContextEditTechnicalPlan> _questContextPlan(
  Revision3QuestOutlineFixture fixture,
) async {
  final index = fixture.contentIndex();
  final catalog = _controllerQuestContextCatalog(fixture);
  Revision3QuestContextEditTechnicalPlan? result;
  final service = Revision3QuestContextAuthoringService(
    loadSeed:
        ({
          required questId,
          required expectedQuestRevision,
          required expectedModuleId,
          required expectedModuleRevision,
          required expectedParentRuntimeClass,
          required expectedGiverRuntimeUniqueName,
        }) async => _questContextSeed(fixture),
    loadCatalog: (_) async => catalog,
    publishTechnicalPlan: ({required gameRoot, required plan}) async {
      result = plan;
      return Revision3QuestContextEditPublication(
        projectId: revision3QuestOutlineProjectId,
        projectRevision: fixture.projectRevision + 1,
        questId: revision3QuestOutlineQuestId,
        moduleId: revision3QuestOutlineModuleId,
        questRevision: fixture.questRevision + 1,
        moduleRevision: fixture.moduleRevision + 1,
      );
    },
  );
  final checkpoint = await service.load(
    index: index,
    quest: index.entityById(revision3QuestOutlineQuestId)!,
    gameRoot: r'C:\Games\Gothic Remake',
  );
  await service.publish(
    checkpoint: checkpoint,
    gameRoot: r'C:\Games\Gothic Remake',
    description: 'Find Homer and report back safely.',
    parent: checkpoint.catalog.parent(revision3QuestContextParentCatalogId)!,
    giver: checkpoint.catalog.giver(revision3QuestContextGiverCatalogId)!,
  );
  return result!;
}

Revision3QuestCatalog _controllerQuestContextCatalog(
  Revision3QuestOutlineFixture fixture,
) => Revision3QuestCatalog(
  parents: [
    Revision3QuestParentChoice(
      catalogId: 'current-parent',
      displayName: 'Chapter Two',
      runtimeClass: 'UQuest_SwampCamp_SCChapter2',
      catalogLayer: 'base-game.quest-parent.v1',
      authoringSelector: 'SwampCamp_SCChapter2',
      sourceSeal: _controllerSeal(11, '1'),
    ),
    Revision3QuestParentChoice(
      catalogId: revision3QuestContextParentCatalogId,
      displayName: 'Chapter Three',
      runtimeClass: revision3QuestContextParentRuntimeClass,
      catalogLayer: 'base-game.quest-parent.v1',
      authoringSelector: 'SwampCamp_SCChapter3',
      sourceSeal: _controllerSeal(11, '1'),
    ),
  ],
  givers: [
    Revision3QuestGiverChoice(
      catalogId: 'current-giver',
      displayName: 'Asghan',
      runtimeUniqueName: 'OM_GRD_Asghan_263',
      catalogLayer: 'base-game.npc.v1',
      authoringSelector: 'OM_GRD_Asghan_263',
      sourceSeal: _controllerSeal(12, '2'),
    ),
    Revision3QuestGiverChoice(
      catalogId: revision3QuestContextGiverCatalogId,
      displayName: 'Viper',
      runtimeUniqueName: revision3QuestContextGiverRuntimeUniqueName,
      catalogLayer: 'base-game.npc.v1',
      authoringSelector: revision3QuestContextGiverRuntimeUniqueName,
      sourceSeal: _controllerSeal(12, '2'),
    ),
  ],
  catalogSeal: fixture.storyCatalogSeal,
  generationExecutableSeal: _controllerSeal(171698176, 'a'),
);

AuthoringDraftContentSeal _controllerSeal(int bytes, String digit) =>
    AuthoringDraftContentSeal.fromJson(<String, Object?>{
      'byte_len': bytes,
      'sha256': List<String>.filled(64, digit).join(),
    });

Future<Revision3NpcProfileEditTechnicalPlan> _npcProfileEditPlan(
  Revision3NpcProfileTestFixture fixture,
) async {
  final catalog = fixture.catalog();
  Revision3NpcProfileEditTechnicalPlan? result;
  final service = Revision3NpcProfileEditAuthoringService(
    loadSeed:
        ({
          required npcId,
          required expectedNpcRevision,
          required expectedScriptModuleId,
          required expectedScriptModuleRevision,
          required expectedUniqueName,
          required expectedModuleNamespace,
          required expectedParentCharacterDefinition,
          required expectedParentAiAgentConfig,
          required expectedParentSpawnDefinition,
        }) async => fixture.seed,
    loadCatalog: (_) async => catalog,
    publishTechnicalPlan: ({required gameRoot, required plan}) async {
      result = plan;
      return _npcProfilePublication(plan);
    },
  );
  final checkpoint = await service.load(
    index: fixture.index,
    npc: fixture.npc,
    gameRoot: r'C:\Games\Gothic Remake',
  );
  await service.publish(
    checkpoint: checkpoint,
    gameRoot: r'C:\Games\Gothic Remake',
    displayName: 'Renamed Managed Guard',
    archetype: catalog.choice(revision3NpcProfileAsghanId)!,
  );
  return result!;
}

Revision3NpcProfileEditPublication _npcProfilePublication(
  Revision3NpcProfileEditTechnicalPlan plan,
) => Revision3NpcProfileEditPublication(
  projectId: plan.projectId,
  projectRevision: plan.projectRevision + 1,
  npcId: plan.npcId,
  npcRevision: plan.expectedNpcRevision + 1,
  scriptModuleId: plan.scriptModuleId,
  scriptModuleRevision:
      plan.expectedScriptModuleRevision + (plan.moduleRegenerated ? 1 : 0),
  displayName: plan.displayName,
  previousParentCatalogId: plan.expectedParentCatalogId,
  parentCatalogId: plan.parentCatalogId,
  nameChanged: plan.nameChanged,
  archetypeChanged: plan.archetypeChanged,
  moduleRegenerated: plan.moduleRegenerated,
);

typedef _VerifyHook = FutureOr<void> Function(_FakeManagedLease lease);
typedef _ContentReadHook = FutureOr<void> Function(_FakeManagedLease lease);
typedef _QuestInspectionHook =
    FutureOr<AuthoringRevision3QuestSourceInspectionResult> Function(
      _FakeManagedLease lease,
      String gameRoot,
      String questId,
    );
typedef _NpcInspectionHook =
    FutureOr<AuthoringRevision3NpcSourceInspectionResult> Function(
      _FakeManagedLease lease,
      String npcId,
    );
typedef _QuestPublishHook =
    FutureOr<Revision3QuestDraftPublication> Function(
      _FakeManagedLease lease,
      String gameRoot,
      Revision3QuestDraftAuthoringInput input,
    );
typedef _QuestOutlinePublishHook =
    FutureOr<Revision3QuestOutlineEditPublication> Function(
      _FakeManagedLease lease,
      Revision3QuestOutlineEditInput input,
    );
typedef _QuestTransitionsSeedHook =
    FutureOr<AuthoringRevision3QuestTransitionsSeed> Function(
      _FakeManagedLease lease,
      String questId,
      int questRevision,
      String moduleId,
      int moduleRevision,
    );
typedef _QuestTransitionsPublishHook =
    FutureOr<Revision3QuestTransitionsEditPublication> Function(
      _FakeManagedLease lease,
      Revision3QuestTransitionsEditTechnicalPlan plan,
    );
typedef _QuestContextSeedHook =
    FutureOr<AuthoringRevision3QuestContextSeed> Function(
      _FakeManagedLease lease,
      String questId,
      int questRevision,
      String moduleId,
      int moduleRevision,
      String parentRuntimeClass,
      String giverRuntimeUniqueName,
    );
typedef _QuestContextPublishHook =
    FutureOr<Revision3QuestContextEditPublication> Function(
      _FakeManagedLease lease,
      String gameRoot,
      Revision3QuestContextEditTechnicalPlan plan,
    );
typedef _NpcPublishHook =
    FutureOr<Revision3NpcDraftPublication> Function(
      _FakeManagedLease lease,
      String gameRoot,
      Revision3NpcDraftAuthoringInput input,
    );
typedef _DialogLinePublishHook =
    FutureOr<Revision3DialogLineEntryPublication> Function(
      _FakeManagedLease lease,
      Revision3DialogLineEntryTechnicalPlan plan,
    );
typedef _ManagedCompilerCheckHook =
    FutureOr<ManagedRevision3CompilerCheckReceipt> Function(
      _FakeManagedLease lease,
      AuthoringRevision3ManagedCompilerEntityKind entityKind,
      String gameRoot,
      String entityId,
      int entityRevision,
      String moduleId,
      int moduleRevision,
    );
typedef _VoicePublishHook =
    FutureOr<Revision3VoiceTakePublication> Function(
      _FakeManagedLease lease,
      String gameRoot,
      Revision3VoiceTakeTechnicalPlan plan,
    );
typedef _VoiceTargetPublishHook =
    FutureOr<Revision3VoiceTargetPublication> Function(
      _FakeManagedLease lease,
      String gameRoot,
      Revision3VoiceTargetTechnicalPlan plan,
    );
typedef _VoiceBuildHook =
    FutureOr<AuthoringRevision3VoiceBuildResult> Function(
      _FakeManagedLease lease,
      String gameRoot,
      String output,
    );
typedef _VoicePlanHook =
    FutureOr<AuthoringRevision3VoiceBuildPlanResult> Function(
      _FakeManagedLease lease,
    );
typedef _ReviewedDataAssetBuildHook =
    FutureOr<AuthoringRevision3ReviewedDataAssetBuildResult> Function(
      _FakeManagedLease lease,
      String gameRoot,
      String targetPath,
      String packName,
      String output,
    );
typedef _DataAssetPublishHook =
    FutureOr<Revision3DataAssetStagePublication> Function(
      _FakeManagedLease lease,
      String patchReceiptPath,
    );
typedef _DataAssetSemanticPublishHook =
    FutureOr<Revision3DataAssetStagePublication> Function(
      _FakeManagedLease lease,
      DataAssetSemanticEditIntent intent,
    );
typedef _InstalledDataAssetEditPublishHook =
    FutureOr<Revision3DataAssetStagePublication> Function(
      _FakeManagedLease lease,
      String gameRoot,
      DataAssetInstalledSemanticEditIntent intent,
    );
typedef _ReviewedInstalledDataAssetEditPublishHook =
    FutureOr<Revision3DataAssetStagePublication> Function(
      _FakeManagedLease lease,
      String gameRoot,
      ReviewedInstalledDataAssetEditIntent intent,
    );
typedef _VoiceSelectionPublishHook =
    FutureOr<Revision3VoiceTakeSelectionPublication> Function(
      _FakeManagedLease lease,
      Revision3VoiceTakeSelectionTechnicalPlan plan,
    );
typedef _VoiceTakeStatusPublishHook =
    FutureOr<Revision3VoiceTakeStatusPublication> Function(
      _FakeManagedLease lease,
      Revision3VoiceTakeStatusTechnicalPlan plan,
    );
typedef _DataAssetRemoveHook =
    FutureOr<Revision3DataAssetStageRemovalPublication> Function(
      _FakeManagedLease lease,
      String targetPath,
    );
typedef _DataAssetPackageIndexReadHook =
    FutureOr<AuthoringRevision3DataAssetPackageIndexResult> Function(
      _FakeManagedLease lease,
      String gameRoot,
    );
typedef _InstalledDataAssetInspectionHook =
    FutureOr<AuthoringRevision3InstalledDataAssetInspectionResult> Function(
      _FakeManagedLease lease,
      String gameRoot,
      AuthoringRevision3DataAssetPackageIndexResult expectedSnapshot,
      AuthoringRevision3DataAssetPackageCandidate candidate,
    );
typedef _DialogLocalizationReadHook =
    FutureOr<AuthoringRevision3DialogLocalizationReadResult> Function(
      _FakeManagedLease lease,
      String localizationId,
      int expectedLocalizationRevision,
      String expectedLocId,
    );
typedef _DialogLocalizationEditSeedHook =
    FutureOr<AuthoringRevision3DialogLocalizationEditSeed> Function(
      _FakeManagedLease lease,
      String localizationId,
      int expectedLocalizationRevision,
      String expectedLocId,
    );
typedef _DialogLocalizationEditPublishHook =
    FutureOr<Revision3DialogLocalizationEditPublication> Function(
      _FakeManagedLease lease,
      Revision3DialogLocalizationEditTechnicalPlan plan,
    );
typedef _RecoveryHook =
    FutureOr<ManagedRevision3RecoveryCheckpoint> Function(
      _FakeRecoveryManagedLease lease,
    );
typedef _ProjectExportHook =
    FutureOr<AuthoringRevision3ExactSnapshotExportResult> Function(
      _FakeExportManagedLease lease,
      String output,
    );
typedef _HistoryRestoreHook =
    FutureOr<ManagedRevision3ProjectHistoryRestoreCheckpoint> Function(
      _FakeHistoryManagedLease lease,
      Revision3ProjectHistorySnapshot expectedHistory,
      Revision3ProjectHistoryEntry target,
    );

class _FakeManagedLease
    implements
        ManagedRevision3CurrentProjectLease,
        ManagedRevision3DialogLocalizationReadLease,
        ManagedRevision3DialogLocalizationEditLease,
        ManagedRevision3VoiceTakeStatusLease,
        ManagedRevision3ReviewedDataAssetBuildLease {
  _FakeManagedLease({
    required this.root,
    required this.projectIdValue,
    required this.projectRevision,
    required this.head,
    this.projectIdError,
    this.onVerify,
    this.onQuestInspection,
    this.onNpcInspection,
    this.onDialogLocalizationRead,
    this.onDialogLocalizationEditSeed,
    this.onDialogLocalizationEditPublish,
    this.onManagedCompilerCheck,
    this.onNpcPublish,
    this.onDialogLinePublish,
    this.onQuestPublish,
    this.onQuestOutlinePublish,
    this.onQuestTransitionsSeed,
    this.onQuestTransitionsPublish,
    this.onQuestContextSeed,
    this.onQuestContextPublish,
    this.onVoicePublish,
    this.onVoiceSelectionPublish,
    this.onVoiceTakeStatusPublish,
    this.onVoiceTargetPublish,
    this.onVoicePlan,
    this.onVoiceBuild,
    this.onReviewedDataAssetBuild,
    this.onDataAssetPublish,
    this.onDataAssetSemanticPublish,
    this.onInstalledDataAssetEditPublish,
    this.onReviewedInstalledDataAssetEditPublish,
    this.onDataAssetRemove,
    this.onDataAssetPackageIndexRead,
    this.onInstalledDataAssetInspection,
    this.dataAssetStages = const [],
    this.contentIndex,
    this.canonicalProjectJson = '{}',
    this.closeFailuresRemaining = 0,
  });

  @override
  final Directory root;
  final String projectIdValue;
  final Object? projectIdError;
  @override
  String canonicalProjectJson;
  @override
  int projectRevision;
  @override
  AuthoringWorkingHead head;
  final _VerifyHook? onVerify;
  final _QuestInspectionHook? onQuestInspection;
  final _NpcInspectionHook? onNpcInspection;
  final _DialogLocalizationReadHook? onDialogLocalizationRead;
  final _DialogLocalizationEditSeedHook? onDialogLocalizationEditSeed;
  final _DialogLocalizationEditPublishHook? onDialogLocalizationEditPublish;
  final _ManagedCompilerCheckHook? onManagedCompilerCheck;
  final _NpcPublishHook? onNpcPublish;
  final _DialogLinePublishHook? onDialogLinePublish;
  final _QuestPublishHook? onQuestPublish;
  final _QuestOutlinePublishHook? onQuestOutlinePublish;
  final _QuestTransitionsSeedHook? onQuestTransitionsSeed;
  final _QuestTransitionsPublishHook? onQuestTransitionsPublish;
  final _QuestContextSeedHook? onQuestContextSeed;
  final _QuestContextPublishHook? onQuestContextPublish;
  final _VoicePublishHook? onVoicePublish;
  final _VoiceSelectionPublishHook? onVoiceSelectionPublish;
  final _VoiceTakeStatusPublishHook? onVoiceTakeStatusPublish;
  final _VoiceTargetPublishHook? onVoiceTargetPublish;
  final _VoicePlanHook? onVoicePlan;
  final _VoiceBuildHook? onVoiceBuild;
  final _ReviewedDataAssetBuildHook? onReviewedDataAssetBuild;
  final _DataAssetPublishHook? onDataAssetPublish;
  final _DataAssetSemanticPublishHook? onDataAssetSemanticPublish;
  final _InstalledDataAssetEditPublishHook? onInstalledDataAssetEditPublish;
  final _ReviewedInstalledDataAssetEditPublishHook?
  onReviewedInstalledDataAssetEditPublish;
  final _DataAssetRemoveHook? onDataAssetRemove;
  final _DataAssetPackageIndexReadHook? onDataAssetPackageIndexRead;
  final _InstalledDataAssetInspectionHook? onInstalledDataAssetInspection;
  final List<AuthoringRevision3DataAssetStage> dataAssetStages;
  final Revision3ContentIndex? contentIndex;
  _ContentReadHook? onContentRead;
  int closeFailuresRemaining;
  bool requiresReopenValue = false;
  int verifyCalls = 0;
  int contentReadCalls = 0;
  int questInspectionCalls = 0;
  final List<String> questInspectionGameRoots = <String>[];
  final List<String> questInspectionQuestIds = <String>[];
  int npcInspectionCalls = 0;
  final List<String> npcInspectionNpcIds = <String>[];
  int dialogLocalizationReadCalls = 0;
  final List<String> dialogLocalizationReadIds = <String>[];
  final List<int> dialogLocalizationReadRevisions = <int>[];
  final List<String> dialogLocalizationReadLocIds = <String>[];
  int dialogLocalizationEditSeedCalls = 0;
  int dialogLocalizationEditPublishCalls = 0;
  int managedCompilerCheckCalls = 0;
  final List<AuthoringRevision3ManagedCompilerEntityKind>
  managedCompilerCheckKinds = <AuthoringRevision3ManagedCompilerEntityKind>[];
  final List<String> managedCompilerCheckGameRoots = <String>[];
  final List<String> managedCompilerCheckEntityIds = <String>[];
  final List<int> managedCompilerCheckEntityRevisions = <int>[];
  final List<String> managedCompilerCheckModuleIds = <String>[];
  final List<int> managedCompilerCheckModuleRevisions = <int>[];
  int npcPublishCalls = 0;
  int dialogLinePublishCalls = 0;
  int questPublishCalls = 0;
  int questOutlinePublishCalls = 0;
  int questTransitionsSeedCalls = 0;
  int questTransitionsPublishCalls = 0;
  int questContextSeedCalls = 0;
  int questContextPublishCalls = 0;
  int voicePublishCalls = 0;
  int voiceSelectionPublishCalls = 0;
  int voiceTakeStatusPublishCalls = 0;
  int voiceTargetPublishCalls = 0;
  int voicePlanCalls = 0;
  int voiceBuildCalls = 0;
  int reviewedDataAssetBuildCalls = 0;
  int dataAssetListCalls = 0;
  int dataAssetPublishCalls = 0;
  int dataAssetSemanticPublishCalls = 0;
  int installedDataAssetEditPublishCalls = 0;
  final List<String> installedDataAssetEditGameRoots = <String>[];
  final List<DataAssetInstalledSemanticEditIntent>
  installedDataAssetEditIntents = <DataAssetInstalledSemanticEditIntent>[];
  int reviewedInstalledDataAssetEditPublishCalls = 0;
  final List<String> reviewedInstalledDataAssetEditGameRoots = <String>[];
  final List<ReviewedInstalledDataAssetEditIntent>
  reviewedInstalledDataAssetEditIntents =
      <ReviewedInstalledDataAssetEditIntent>[];
  int dataAssetRemoveCalls = 0;
  int dataAssetPackageIndexReadCalls = 0;
  final List<String> dataAssetPackageIndexGameRoots = <String>[];
  int installedDataAssetInspectionCalls = 0;
  int publicationUncertaintyLatchCalls = 0;

  @override
  void markRequiresReopenAfterPublicationUncertainty() {
    publicationUncertaintyLatchCalls++;
    requiresReopenValue = true;
  }

  final List<String> installedDataAssetInspectionGameRoots = <String>[];
  final List<AuthoringRevision3DataAssetPackageIndexResult>
  installedDataAssetInspectionSnapshots =
      <AuthoringRevision3DataAssetPackageIndexResult>[];
  final List<AuthoringRevision3DataAssetPackageCandidate>
  installedDataAssetInspectionCandidates =
      <AuthoringRevision3DataAssetPackageCandidate>[];
  int closeCalls = 0;

  @override
  String get projectId {
    final error = projectIdError;
    if (error != null) throw error;
    return projectIdValue;
  }

  @override
  bool get requiresReopen => requiresReopenValue;

  @override
  bool get supportsReviewedDataAssetBuild => onReviewedDataAssetBuild != null;

  @override
  Future<void> verifyCurrentHead() async {
    verifyCalls++;
    await onVerify?.call(this);
  }

  @override
  Future<Revision3ContentIndex> readContentIndex() async {
    contentReadCalls++;
    await onContentRead?.call(this);
    return contentIndex ??
        (throw StateError('fake managed lease has no content index'));
  }

  @override
  Future<AuthoringRevision3QuestSourceInspectionResult> inspectQuestSourceV1({
    required String gameRoot,
    required String questId,
  }) async {
    questInspectionCalls++;
    questInspectionGameRoots.add(gameRoot);
    questInspectionQuestIds.add(questId);
    final inspect = onQuestInspection;
    if (inspect == null) {
      throw StateError('fake managed lease has no Quest source inspector');
    }
    return inspect(this, gameRoot, questId);
  }

  @override
  Future<AuthoringRevision3NpcSourceInspectionResult> inspectNpcSourceV1({
    required String npcId,
  }) async {
    npcInspectionCalls++;
    npcInspectionNpcIds.add(npcId);
    final inspect = onNpcInspection;
    if (inspect == null) {
      throw StateError('fake managed lease has no NPC source inspector');
    }
    return inspect(this, npcId);
  }

  @override
  Future<AuthoringRevision3DialogLocalizationReadResult>
  readDialogLocalizationV1({
    required String localizationId,
    required int expectedLocalizationRevision,
    required String expectedLocId,
  }) async {
    dialogLocalizationReadCalls++;
    dialogLocalizationReadIds.add(localizationId);
    dialogLocalizationReadRevisions.add(expectedLocalizationRevision);
    dialogLocalizationReadLocIds.add(expectedLocId);
    final read = onDialogLocalizationRead;
    if (read == null) {
      throw StateError('fake managed lease has no localization reader');
    }
    return read(
      this,
      localizationId,
      expectedLocalizationRevision,
      expectedLocId,
    );
  }

  @override
  Future<AuthoringRevision3DialogLocalizationEditSeed>
  readDialogLocalizationEditSeedV1({
    required String localizationId,
    required int expectedLocalizationRevision,
    required String expectedLocId,
  }) async {
    dialogLocalizationEditSeedCalls++;
    final read = onDialogLocalizationEditSeed;
    if (read == null) {
      throw StateError('fake managed lease has no localization-edit reader');
    }
    return read(
      this,
      localizationId,
      expectedLocalizationRevision,
      expectedLocId,
    );
  }

  @override
  Future<Revision3DialogLocalizationEditPublication>
  prepareAndPublishDialogLocalizationEditV1({
    required Revision3DialogLocalizationEditTechnicalPlan plan,
  }) async {
    dialogLocalizationEditPublishCalls++;
    final publish = onDialogLocalizationEditPublish;
    if (publish == null) {
      throw StateError('fake managed lease has no localization-edit publisher');
    }
    return publish(this, plan);
  }

  @override
  Future<ManagedRevision3CompilerCheckReceipt> checkCompilerV1({
    required AuthoringRevision3ManagedCompilerEntityKind entityKind,
    required String gameRoot,
    required String entityId,
    required int expectedEntityRevision,
    required String expectedModuleId,
    required int expectedModuleRevision,
  }) async {
    managedCompilerCheckCalls++;
    managedCompilerCheckKinds.add(entityKind);
    managedCompilerCheckGameRoots.add(gameRoot);
    managedCompilerCheckEntityIds.add(entityId);
    managedCompilerCheckEntityRevisions.add(expectedEntityRevision);
    managedCompilerCheckModuleIds.add(expectedModuleId);
    managedCompilerCheckModuleRevisions.add(expectedModuleRevision);
    final check = onManagedCompilerCheck;
    if (check == null) {
      throw StateError('fake managed lease has no compiler checker');
    }
    return check(
      this,
      entityKind,
      gameRoot,
      entityId,
      expectedEntityRevision,
      expectedModuleId,
      expectedModuleRevision,
    );
  }

  @override
  Future<AuthoringRevision3DataAssetPackageIndexResult>
  readDataAssetPackageIndexV1({required String gameRoot}) async {
    dataAssetPackageIndexReadCalls++;
    dataAssetPackageIndexGameRoots.add(gameRoot);
    final read = onDataAssetPackageIndexRead;
    if (read == null) {
      throw StateError('fake managed lease has no DataAsset package index');
    }
    return read(this, gameRoot);
  }

  @override
  Future<AuthoringRevision3InstalledDataAssetInspectionResult>
  inspectInstalledDataAssetV1({
    required String gameRoot,
    required AuthoringRevision3DataAssetPackageIndexResult expectedSnapshot,
    required AuthoringRevision3DataAssetPackageCandidate candidate,
  }) async {
    installedDataAssetInspectionCalls++;
    installedDataAssetInspectionGameRoots.add(gameRoot);
    installedDataAssetInspectionSnapshots.add(expectedSnapshot);
    installedDataAssetInspectionCandidates.add(candidate);
    final inspect = onInstalledDataAssetInspection;
    if (inspect == null) {
      throw StateError(
        'fake managed lease has no installed DataAsset inspector',
      );
    }
    return inspect(this, gameRoot, expectedSnapshot, candidate);
  }

  @override
  Future<Revision3DataAssetStagePublication>
  prepareAndPublishInstalledDataAssetEditV1({
    required String gameRoot,
    required DataAssetInstalledSemanticEditIntent intent,
  }) async {
    installedDataAssetEditPublishCalls++;
    installedDataAssetEditGameRoots.add(gameRoot);
    installedDataAssetEditIntents.add(intent);
    final publish = onInstalledDataAssetEditPublish;
    if (publish == null) {
      throw StateError(
        'fake managed lease has no installed DataAsset edit publisher',
      );
    }
    return publish(this, gameRoot, intent);
  }

  @override
  Future<Revision3DataAssetStagePublication>
  prepareAndPublishReviewedInstalledDataAssetEditV1({
    required String gameRoot,
    required ReviewedInstalledDataAssetEditIntent intent,
  }) async {
    reviewedInstalledDataAssetEditPublishCalls++;
    reviewedInstalledDataAssetEditGameRoots.add(gameRoot);
    reviewedInstalledDataAssetEditIntents.add(intent);
    final publish = onReviewedInstalledDataAssetEditPublish;
    if (publish == null) {
      throw StateError(
        'fake managed lease has no reviewed installed DataAsset edit publisher',
      );
    }
    return publish(this, gameRoot, intent);
  }

  @override
  Future<Revision3QuestDraftPublication> prepareAndPublishQuestDraftV3({
    required String gameRoot,
    required Revision3QuestDraftAuthoringInput input,
  }) async {
    questPublishCalls++;
    final publish = onQuestPublish;
    if (publish == null) {
      throw StateError('fake managed lease has no Quest publisher');
    }
    return publish(this, gameRoot, input);
  }

  @override
  Future<Revision3QuestOutlineEditPublication>
  prepareAndPublishQuestOutlineEditV1({
    required Revision3QuestOutlineEditInput input,
  }) async {
    questOutlinePublishCalls++;
    final publish = onQuestOutlinePublish;
    if (publish == null) {
      throw StateError('fake managed lease has no Quest outline publisher');
    }
    return publish(this, input);
  }

  @override
  Future<AuthoringRevision3QuestTransitionsSeed> readQuestTransitionsSeedV1({
    required String questId,
    required int expectedQuestRevision,
    required String expectedModuleId,
    required int expectedModuleRevision,
  }) async {
    questTransitionsSeedCalls++;
    final read = onQuestTransitionsSeed;
    if (read == null) {
      throw StateError('fake managed lease has no Quest transitions seed');
    }
    return read(
      this,
      questId,
      expectedQuestRevision,
      expectedModuleId,
      expectedModuleRevision,
    );
  }

  @override
  Future<Revision3QuestTransitionsEditPublication>
  prepareAndPublishQuestTransitionsEditV1({
    required Revision3QuestTransitionsEditTechnicalPlan plan,
  }) async {
    questTransitionsPublishCalls++;
    final publish = onQuestTransitionsPublish;
    if (publish == null) {
      throw StateError('fake managed lease has no Quest transitions publisher');
    }
    return publish(this, plan);
  }

  @override
  Future<AuthoringRevision3QuestContextSeed> readQuestContextSeedV1({
    required String questId,
    required int expectedQuestRevision,
    required String expectedModuleId,
    required int expectedModuleRevision,
    required String expectedParentRuntimeClass,
    required String expectedGiverRuntimeUniqueName,
  }) async {
    questContextSeedCalls++;
    final read = onQuestContextSeed;
    if (read == null) {
      throw StateError('fake managed lease has no Quest context seed');
    }
    return read(
      this,
      questId,
      expectedQuestRevision,
      expectedModuleId,
      expectedModuleRevision,
      expectedParentRuntimeClass,
      expectedGiverRuntimeUniqueName,
    );
  }

  @override
  Future<Revision3QuestContextEditPublication>
  prepareAndPublishQuestContextEditV1({
    required String gameRoot,
    required Revision3QuestContextEditTechnicalPlan plan,
  }) async {
    questContextPublishCalls++;
    final publish = onQuestContextPublish;
    if (publish == null) {
      throw StateError('fake managed lease has no Quest context publisher');
    }
    return publish(this, gameRoot, plan);
  }

  @override
  Future<Revision3NpcDraftPublication> prepareAndPublishNpcDraftV1({
    required String gameRoot,
    required Revision3NpcDraftAuthoringInput input,
  }) async {
    npcPublishCalls++;
    final publish = onNpcPublish;
    if (publish == null) {
      throw StateError('fake managed lease has no NPC publisher');
    }
    return publish(this, gameRoot, input);
  }

  @override
  Future<Revision3DialogLineEntryPublication> prepareAndPublishDialogLineV1({
    required Revision3DialogLineEntryTechnicalPlan plan,
  }) async {
    dialogLinePublishCalls++;
    final publish = onDialogLinePublish;
    if (publish == null) {
      throw StateError('fake managed lease has no dialog-line publisher');
    }
    return publish(this, plan);
  }

  @override
  Future<Revision3VoiceTakePublication> prepareAndPublishVoiceTakeV1({
    required String gameRoot,
    required Revision3VoiceTakeTechnicalPlan plan,
  }) async {
    voicePublishCalls++;
    final publish = onVoicePublish;
    if (publish == null) {
      throw StateError('fake managed lease has no Voice publisher');
    }
    return publish(this, gameRoot, plan);
  }

  @override
  Future<Revision3VoiceTakeSelectionPublication>
  prepareAndPublishVoiceTakeSelectionV1({
    required Revision3VoiceTakeSelectionTechnicalPlan plan,
  }) async {
    voiceSelectionPublishCalls++;
    final publish = onVoiceSelectionPublish;
    if (publish == null) {
      throw StateError('fake managed lease has no Voice selection publisher');
    }
    return publish(this, plan);
  }

  @override
  Future<Revision3VoiceTakeStatusPublication>
  prepareAndPublishVoiceTakeStatusV1({
    required Revision3VoiceTakeStatusTechnicalPlan plan,
  }) async {
    voiceTakeStatusPublishCalls++;
    final publish = onVoiceTakeStatusPublish;
    if (publish == null) {
      throw StateError('fake managed lease has no Voice take status publisher');
    }
    return publish(this, plan);
  }

  @override
  Future<Revision3VoiceTargetPublication> prepareAndPublishVoiceTargetV1({
    required String gameRoot,
    required Revision3VoiceTargetTechnicalPlan plan,
  }) async {
    voiceTargetPublishCalls++;
    final publish = onVoiceTargetPublish;
    if (publish == null) {
      throw StateError('fake managed lease has no Voice target publisher');
    }
    return publish(this, gameRoot, plan);
  }

  @override
  Future<AuthoringRevision3VoiceBuildPlanResult> planVoiceV1() async {
    voicePlanCalls++;
    final plan = onVoicePlan;
    if (plan != null) return plan(this);
    return _voicePlanResult(
      head: head,
      projectId: projectId,
      projectRevision: projectRevision,
    );
  }

  @override
  Future<AuthoringRevision3VoiceBuildResult> buildVoiceV1({
    required String gameRoot,
    required String output,
  }) async {
    voiceBuildCalls++;
    final build = onVoiceBuild;
    if (build == null) {
      throw StateError('fake managed lease has no Voice builder');
    }
    return build(this, gameRoot, output);
  }

  @override
  Future<AuthoringRevision3ReviewedDataAssetBuildResult>
  buildReviewedDataAssetV1({
    required String gameRoot,
    required String targetPath,
    required String packName,
    required String output,
  }) async {
    reviewedDataAssetBuildCalls++;
    final build = onReviewedDataAssetBuild;
    if (build == null) {
      throw StateError('fake managed lease has no reviewed DataAsset builder');
    }
    return build(this, gameRoot, targetPath, packName, output);
  }

  @override
  Future<List<AuthoringRevision3DataAssetStage>> listDataAssetStagesV1() async {
    dataAssetListCalls++;
    return dataAssetStages;
  }

  @override
  Future<Revision3DataAssetStagePublication> prepareAndPublishDataAssetStageV1({
    required String patchReceiptPath,
  }) async {
    dataAssetPublishCalls++;
    final publish = onDataAssetPublish;
    if (publish == null) {
      throw StateError('fake managed lease has no DataAsset publisher');
    }
    return publish(this, patchReceiptPath);
  }

  @override
  Future<Revision3DataAssetStagePublication> prepareAndPublishDataAssetEditV1({
    required DataAssetSemanticEditIntent intent,
  }) async {
    dataAssetSemanticPublishCalls++;
    final publish = onDataAssetSemanticPublish;
    if (publish == null) {
      throw StateError(
        'fake managed lease has no semantic DataAsset publisher',
      );
    }
    return publish(this, intent);
  }

  @override
  Future<Revision3DataAssetStageRemovalPublication>
  prepareAndPublishRemoveDataAssetStageV1({required String targetPath}) async {
    dataAssetRemoveCalls++;
    final remove = onDataAssetRemove;
    if (remove == null) {
      throw StateError('fake managed lease has no DataAsset remover');
    }
    return remove(this, targetPath);
  }

  @override
  Future<void> close() async {
    closeCalls++;
    if (closeFailuresRemaining > 0) {
      closeFailuresRemaining--;
      throw StateError('injected close failure');
    }
  }
}

final class _FakeNpcProfileManagedLease extends _FakeManagedLease
    implements ManagedRevision3NpcProfileEditLease {
  _FakeNpcProfileManagedLease({
    required super.root,
    required super.projectIdValue,
    required super.projectRevision,
    required super.head,
    required this.seed,
    required this.onPublish,
  });

  final AuthoringRevision3NpcProfileEditSeed seed;
  final FutureOr<Revision3NpcProfileEditPublication> Function(
    _FakeNpcProfileManagedLease lease,
    String gameRoot,
    Revision3NpcProfileEditTechnicalPlan plan,
  )
  onPublish;
  int seedCalls = 0;
  int publishCalls = 0;
  int latchCalls = 0;

  @override
  bool get supportsNpcProfileEdit => true;

  @override
  void markRequiresReopenAfterNpcProfileEditUncertainty() {
    latchCalls++;
    requiresReopenValue = true;
  }

  @override
  Future<AuthoringRevision3NpcProfileEditSeed> readNpcProfileEditSeedV1({
    required String npcId,
    required int expectedNpcRevision,
    required String expectedScriptModuleId,
    required int expectedScriptModuleRevision,
    required String expectedUniqueName,
    required String expectedModuleNamespace,
    required String expectedParentCharacterDefinition,
    required String expectedParentAiAgentConfig,
    required String expectedParentSpawnDefinition,
  }) async {
    seedCalls++;
    return seed;
  }

  @override
  Future<Revision3NpcProfileEditPublication> prepareAndPublishNpcProfileEditV1({
    required String gameRoot,
    required Revision3NpcProfileEditTechnicalPlan plan,
  }) async {
    publishCalls++;
    return onPublish(this, gameRoot, plan);
  }
}

final class _FakeExportManagedLease extends _FakeManagedLease
    implements ManagedRevision3ProjectExportLease {
  _FakeExportManagedLease({
    required super.root,
    required super.projectIdValue,
    required super.projectRevision,
    required super.head,
    required this.onExport,
    this.supportsExport = true,
  });

  final _ProjectExportHook onExport;
  final bool supportsExport;
  int exportCalls = 0;
  final List<String> exportOutputs = <String>[];

  @override
  bool get supportsExactSnapshotExport => supportsExport;

  @override
  Future<AuthoringRevision3ExactSnapshotExportResult> exportExactSnapshotV1({
    required String output,
  }) async {
    exportCalls++;
    exportOutputs.add(output);
    return onExport(this, output);
  }
}

final class _FakeStoryDraftRemovalManagedLease extends _FakeManagedLease
    implements ManagedRevision3StoryDraftRemovalLease {
  _FakeStoryDraftRemovalManagedLease({
    required super.root,
    required super.projectIdValue,
    required super.projectRevision,
    required super.head,
    this.supportsRemoval = true,
    this.receiptMismatch = false,
    this.nextStoryDraftRemovalError,
  });

  final bool supportsRemoval;
  final bool receiptMismatch;
  Object? nextStoryDraftRemovalError;
  int storyDraftRemovalCalls = 0;
  int storyDraftRemovalRelatchCalls = 0;
  final List<String> receivedDraftIds = <String>[];
  final List<AuthoringStoryDraftKind> receivedDraftKinds =
      <AuthoringStoryDraftKind>[];

  @override
  bool get supportsStoryDraftRemoval => supportsRemoval;

  @override
  void markRequiresReopenAfterStoryDraftRemovalUncertainty() {
    storyDraftRemovalRelatchCalls++;
    requiresReopenValue = true;
  }

  @override
  Future<Revision3StoryDraftRemovalPublication>
  prepareAndPublishRemoveStoryDraftV1({
    required String draftId,
    required AuthoringStoryDraftKind draftKind,
    required int expectedDraftRevision,
    required String scriptModuleId,
    required int expectedScriptModuleRevision,
  }) async {
    storyDraftRemovalCalls++;
    receivedDraftIds.add(draftId);
    receivedDraftKinds.add(draftKind);
    final injected = nextStoryDraftRemovalError;
    nextStoryDraftRemovalError = null;
    if (injected != null) throw injected;

    projectRevision++;
    head = _head(projectRevision);
    return Revision3StoryDraftRemovalPublication(
      head: receiptMismatch ? _head(99) : head,
      projectId: projectId,
      projectRevision: projectRevision,
      removedDraftId: draftId,
      removedDraftKind: draftKind,
      removedDraftRevision: expectedDraftRevision,
      removedScriptModuleId: scriptModuleId,
      removedScriptModuleRevision: expectedScriptModuleRevision,
    );
  }
}

final class _FakeVoiceTakeMediaQaManagedLease extends _FakeManagedLease
    implements ManagedRevision3VoiceTakeMediaQaLease {
  _FakeVoiceTakeMediaQaManagedLease({
    required super.root,
    required super.projectIdValue,
    required super.projectRevision,
    required super.head,
    this.receiptMismatch = false,
    this.nextError,
  });

  final bool receiptMismatch;
  Object? nextError;
  int inspectCalls = 0;
  int relatchCalls = 0;

  @override
  bool get supportsVoiceTakeMediaQa => true;

  @override
  void markRequiresReopenAfterVoiceTakeMediaQaUncertainty() {
    relatchCalls++;
    requiresReopenValue = true;
  }

  @override
  Future<AuthoringRevision3VoiceTakeMediaQaResult> inspectVoiceTakeMediaQaV1({
    required Revision3VoiceTakePreviewTechnicalPlan plan,
  }) async {
    inspectCalls++;
    final injected = nextError;
    nextError = null;
    if (injected != null) {
      throw injected;
    }
    final request = revision3VoicePreviewRequest(head: head);
    return AuthoringRevision3VoiceTakeMediaQaResult.fromJson(
      revision3VoiceMediaQaResponse(
        request: request,
        status: receiptMismatch ? 'approved' : 'recorded',
      ),
      request: request,
    );
  }
}

final class _FakeVoiceTakePreviewManagedLease extends _FakeManagedLease
    implements ManagedRevision3VoiceTakePreviewLease {
  _FakeVoiceTakePreviewManagedLease({
    required super.root,
    required super.projectIdValue,
    required super.projectRevision,
    required super.head,
    this.supportsPreview = true,
    this.receiptMismatch = false,
    this.cleanupFailure = false,
    this.poisonOnError = false,
    this.nextError,
  });

  final bool supportsPreview;
  final bool receiptMismatch;
  final bool cleanupFailure;
  final bool poisonOnError;
  Object? nextError;
  int materializeCalls = 0;
  int relatchCalls = 0;
  Revision3VoiceTakePreviewCapability? lastCapability;

  @override
  bool get supportsVoiceTakePreview => supportsPreview;

  @override
  void markRequiresReopenAfterVoiceTakePreviewUncertainty() {
    relatchCalls++;
    requiresReopenValue = true;
  }

  @override
  Future<Revision3VoiceTakePreviewCapability> materializeVoiceTakePreviewV1({
    required Revision3VoiceTakePreviewTechnicalPlan plan,
  }) async {
    materializeCalls++;
    final injected = nextError;
    nextError = null;
    if (injected != null) {
      if (poisonOnError) requiresReopenValue = true;
      throw injected;
    }
    final capability = await _voicePreviewCapability(
      head: head,
      projectId: projectId,
      projectRevision: receiptMismatch ? projectRevision + 1 : projectRevision,
      plan: plan,
    );
    if (cleanupFailure) {
      await File(capability.path).delete();
      await Directory(capability.path).create();
    }
    lastCapability = capability;
    return capability;
  }
}

final class _FakeVoiceTakeRemovalManagedLease extends _FakeManagedLease
    implements ManagedRevision3VoiceTakeRemovalLease {
  _FakeVoiceTakeRemovalManagedLease({
    required super.root,
    required super.projectIdValue,
    required super.projectRevision,
    required super.head,
    this.supportsRemoval = true,
    this.receiptMismatch = false,
    this.takeEntityFlagMismatch = false,
    this.nextError,
    this.postPublishError,
  });

  final bool supportsRemoval;
  final bool receiptMismatch;
  final bool takeEntityFlagMismatch;
  Object? nextError;
  Object? postPublishError;
  int voiceTakeRemovalCalls = 0;
  int voiceTakeRemovalRelatchCalls = 0;

  @override
  bool get supportsVoiceTakeRemoval => supportsRemoval;

  @override
  void markRequiresReopenAfterVoiceTakeRemovalUncertainty() {
    voiceTakeRemovalRelatchCalls++;
    requiresReopenValue = true;
  }

  @override
  Future<Revision3VoiceTakeRemovalPublication>
  prepareAndPublishVoiceTakeRemovalV1({
    required Revision3VoiceTakeRemovalTechnicalPlan plan,
  }) async {
    voiceTakeRemovalCalls++;
    final injected = nextError;
    nextError = null;
    if (injected != null) throw injected;
    projectRevision++;
    head = _head(projectRevision);
    final postPublish = postPublishError;
    postPublishError = null;
    if (postPublish != null) throw postPublish;
    return Revision3VoiceTakeRemovalPublication(
      projectId: projectId,
      projectRevision: receiptMismatch ? projectRevision + 1 : projectRevision,
      lineId: plan.lineId,
      localizationId: plan.localizationId,
      slotId: plan.slotId,
      slotRevision: plan.expectedSlotRevision + 1,
      locale: plan.locale,
      locId: plan.locId,
      takeId: plan.takeId,
      takeRevision: plan.expectedTakeRevision,
      previousSelectedTakeId: plan.expectedSelectedTakeId,
      selectionCleared: plan.expectsSelectionCleared,
      takeEntityRemoved: takeEntityFlagMismatch
          ? !plan.expectedTakeEntityRemoved
          : plan.expectedTakeEntityRemoved,
      remainingCandidateCount: plan.expectedRemainingCandidateCount,
    );
  }
}

final class _FakeDialogVoiceSlotRemovalManagedLease extends _FakeManagedLease
    implements ManagedRevision3DialogVoiceSlotRemovalLease {
  _FakeDialogVoiceSlotRemovalManagedLease({
    required super.root,
    required super.projectIdValue,
    required super.projectRevision,
    required super.head,
    this.receiptMismatch = false,
    this.nextError,
  });

  final bool receiptMismatch;
  Object? nextError;
  int removalCalls = 0;
  int relatchCalls = 0;

  @override
  bool get supportsDialogVoiceSlotRemoval => true;

  @override
  void markRequiresReopenAfterDialogVoiceSlotRemovalUncertainty() {
    relatchCalls++;
    requiresReopenValue = true;
  }

  @override
  Future<Revision3DialogVoiceSlotRemovalPublication>
  prepareAndPublishDialogVoiceSlotRemovalV1({
    required Revision3DialogVoiceSlotRemovalTechnicalPlan plan,
  }) async {
    removalCalls++;
    final injected = nextError;
    nextError = null;
    if (injected != null) throw injected;
    projectRevision++;
    head = _head(projectRevision);
    return Revision3DialogVoiceSlotRemovalPublication(
      projectId: projectId,
      projectRevision: receiptMismatch ? projectRevision + 1 : projectRevision,
      lineId: plan.lineId,
      lineRevision: plan.expectedLineRevision + 1,
      localizationId: plan.localizationId,
      slotId: plan.slotId,
      removedSlotRevision: plan.expectedSlotRevision,
      locale: plan.locale,
      locId: plan.locId,
      removedTargetResolution: plan.targetResolution,
    );
  }
}

final class _FakeRecoveryManagedLease extends _FakeManagedLease
    implements ManagedRevision3RecoveryLease {
  _FakeRecoveryManagedLease({
    required super.root,
    required super.projectIdValue,
    required super.projectRevision,
    required super.head,
    required super.canonicalProjectJson,
    required this.onRecovery,
    super.contentIndex,
  });

  final _RecoveryHook onRecovery;
  int recoveryCalls = 0;
  int recoveryRelatchCalls = 0;

  @override
  Future<ManagedRevision3RecoveryCheckpoint>
  recoverAfterUncertainPublication() async {
    recoveryCalls++;
    return onRecovery(this);
  }

  @override
  void markRequiresReopenAfterRecoveryUncertainty() {
    recoveryRelatchCalls++;
    requiresReopenValue = true;
  }
}

final class _FakeHistoryManagedLease extends _FakeManagedLease
    implements ManagedRevision3ProjectHistoryLease {
  _FakeHistoryManagedLease({
    required super.root,
    required super.projectIdValue,
    required super.projectRevision,
    required super.head,
    required super.canonicalProjectJson,
    required this.history,
    required this.onRestore,
  });

  final Revision3ProjectHistorySnapshot history;
  final _HistoryRestoreHook onRestore;
  int historyReadCalls = 0;
  int historyRestoreCalls = 0;
  int historyRelatchCalls = 0;

  @override
  bool get supportsProjectHistory => true;

  @override
  Future<Revision3ProjectHistorySnapshot> readProjectHistoryV1() async {
    historyReadCalls++;
    return history;
  }

  @override
  Future<ManagedRevision3ProjectHistoryRestoreCheckpoint>
  prepareAndPublishProjectHistoryRestoreV1({
    required Revision3ProjectHistorySnapshot expectedHistory,
    required Revision3ProjectHistoryEntry target,
  }) async {
    historyRestoreCalls++;
    return onRestore(this, expectedHistory, target);
  }

  @override
  void markRequiresReopenAfterHistoryUncertainty() {
    historyRelatchCalls++;
    requiresReopenValue = true;
  }
}

Revision3DialogLineEntryTechnicalPlan _dialogLinePlan({
  required String projectId,
  required int projectRevision,
}) => Revision3DialogLineEntryTechnicalPlan.forCheckpoint(
  catalog: Revision3DialogLineEntryCatalog.fromContentIndex(
    _contentIndex(projectId: projectId, revision: projectRevision),
  ),
  input: Revision3DialogLineEntryInput.create(
    lineDisplayName: 'Gate greeting',
    speakerHint: 'Asghan',
    locale: 'de',
    text: 'Halt! Wer da?',
  ),
);

Revision3VoiceTakeTechnicalPlan _voicePlan() {
  final catalog = Revision3VoiceCatalog.fromContentIndex(
    revision3VoiceContentIndexFixture(),
  );
  return Revision3VoiceTakeTechnicalPlan.forCheckpoint(
    catalog: catalog,
    input: Revision3VoiceTakeAuthoringInput(
      lineId: revision3VoiceContentLineId,
      locale: 'de',
      sourcePath: r'C:\Voice\asghan.ogg',
      takeDisplayName: 'Asghan take',
      status: AuthoringRevision3VoiceTakeStatus.recorded,
    ),
  );
}

Future<Revision3VoiceTakePreviewCapability> _voicePreviewCapability({
  required AuthoringWorkingHead head,
  required String projectId,
  required int projectRevision,
  required Revision3VoiceTakePreviewTechnicalPlan plan,
}) {
  final request = AuthoringRevision3VoiceTakePreviewRequestV1(
    expectedHead: head,
    expectedProjectId: projectId,
    expectedRevision: projectRevision,
    lineId: plan.lineId,
    expectedLineRevision: plan.expectedLineRevision,
    localizationId: plan.localizationId,
    expectedLocalizationRevision: plan.expectedLocalizationRevision,
    expectedLocId: plan.locId,
    slotId: plan.slotId,
    expectedSlotRevision: plan.expectedSlotRevision,
    locale: plan.locale,
    takeId: plan.takeId,
    expectedTakeRevision: plan.expectedTakeRevision,
    expectedAsset: AuthoringRevision3VoiceTakePreviewExpectedAsset(
      sha256: plan.assetSha256,
      byteLength: plan.assetByteLength,
      logicalName: plan.assetLogicalName,
    ),
  );
  late String registeredRoot;
  return Revision3VoiceTakePreviewCapability.materialize(
    register: () async {
      final previewRoot = (await createRevision3VoicePreviewTestRoot()).path;
      registeredRoot = previewRoot;
      return AuthoringRevision3VoiceTakePreviewRegistration.fromJson(
        revision3VoicePreviewRegistrationResponse(previewRoot: previewRoot),
      );
    },
    materialize: (token, previewRoot) async {
      await File(
        '$previewRoot${Platform.pathSeparator}preview.ogg',
      ).writeAsBytes(revision3VoicePreviewBytes, flush: true);
      return AuthoringRevision3VoiceTakePreviewMaterialization.fromJson(
        revision3VoicePreviewResponse(
          previewRoot: previewRoot,
          cleanupToken: token,
          request: request,
        ),
        previewRoot: previewRoot,
        cleanupToken: token,
        request: request,
      );
    },
    release: (token) => _deleteFakeVoicePreviewRoot(registeredRoot),
  );
}

Future<void> _deleteFakeVoicePreviewRoot(String rootPath) async {
  final root = Directory(rootPath);
  final entries = await root.list(followLinks: false).toList();
  if (entries.length > 1 ||
      (entries.isNotEmpty &&
          (p.basename(entries.single.path) != 'preview.ogg' ||
              await FileSystemEntity.type(
                    entries.single.path,
                    followLinks: false,
                  ) !=
                  FileSystemEntityType.file))) {
    throw const FileSystemException('fake retained cleanup failure');
  }
  if (entries.isNotEmpty) await entries.single.delete();
  await root.delete();
}

Revision3VoiceTargetTechnicalPlan _voiceTargetPlan() {
  final catalog = Revision3VoiceCatalog.fromContentIndex(
    revision3VoiceContentIndexFixture(),
  );
  return Revision3VoiceTargetTechnicalPlan.forCheckpoint(
    catalog: catalog,
    lineId: revision3VoiceContentLineId,
    locale: 'de',
  );
}

Revision3VoiceTakeSelectionTechnicalPlan _voiceSelectionPlan() {
  final catalog = Revision3VoiceCatalog.fromContentIndex(
    revision3VoiceContentIndexFixture(
      existingSlotCandidateCount: 1,
      existingSlotHasSelectedTake: true,
    ),
  );
  return Revision3VoiceTakeSelectionTechnicalPlan.forCheckpoint(
    catalog: catalog,
    lineId: revision3VoiceContentLineId,
    locale: 'de',
    selectedTakeId: null,
  );
}

Revision3VoiceTakeRemovalTechnicalPlan _voiceTakeRemovalPlan() {
  final catalog = Revision3VoiceCatalog.fromContentIndex(
    revision3VoiceContentIndexFixture(
      existingSlotCandidateCount: 2,
      existingSlotHasSelectedTake: true,
    ),
  );
  final takeId = catalog
      .line(revision3VoiceContentLineId)!
      .slotSummaryForLocale('de')!
      .candidates
      .first
      .id;
  return Revision3VoiceTakeRemovalTechnicalPlan.forCheckpoint(
    catalog: catalog,
    lineId: revision3VoiceContentLineId,
    locale: 'de',
    takeId: takeId,
  );
}

Revision3DialogVoiceSlotRemovalTechnicalPlan _dialogVoiceSlotRemovalPlan() {
  final catalog = Revision3VoiceCatalog.fromContentIndex(
    revision3VoiceContentIndexFixture(existingSlotGenerated: true),
  );
  return Revision3DialogVoiceSlotRemovalTechnicalPlan.forCheckpoint(
    catalog: catalog,
    lineId: revision3VoiceContentLineId,
    locale: 'de',
  );
}

Revision3VoiceTakeStatusTechnicalPlan _voiceTakeStatusPlan() {
  final catalog = Revision3VoiceCatalog.fromContentIndex(
    revision3VoiceContentIndexFixture(
      existingSlotCandidateCount: 2,
      existingSlotHasSelectedTake: true,
    ),
  );
  final line = catalog.line(revision3VoiceContentLineId)!;
  final summary = line.slotSummaryForLocale('de')!;
  final take = summary.candidates.firstWhere(
    (candidate) => candidate.id != summary.selectedTakeId,
  );
  return Revision3VoiceTakeStatusTechnicalPlan.forCheckpoint(
    catalog: catalog,
    lineId: revision3VoiceContentLineId,
    locale: 'de',
    takeId: take.id,
    desiredStatus: AuthoringRevision3VoiceTakeStatus.reviewed,
  );
}

AuthoringRevision3VoiceBuildPlanResult _voicePlanResult({
  required AuthoringWorkingHead head,
  required String projectId,
  required int projectRevision,
}) => AuthoringRevision3VoiceBuildPlanResult.fromJson(
  <String, Object?>{
    'ok': true,
    'outcome': 'ready',
    'basis_head_json': head.canonicalJson,
    'project_id': projectId,
    'project_revision': projectRevision,
    'total_slots': 1,
    'ready_slots': 1,
    'blockers': const <Object?>[],
    'plan_authority': 'read_only_voice_build_plan_v1',
    'build_authority': 'not_granted',
    'deployment_status': 'not_performed',
  },
  expectedHead: head,
  expectedProjectJson: revision3VoiceFixtureBuildReadyProjectJson(
    projectId: projectId,
    projectRevision: projectRevision,
  ),
);

AuthoringRevision3VoiceBuildResult _voiceBuildResult({
  required AuthoringWorkingHead head,
  required String projectId,
  required int projectRevision,
  required String output,
}) => AuthoringRevision3VoiceBuildResult.fromJson(
  <String, Object?>{
    'ok': true,
    'outcome': 'built',
    'basis_head_json': head.canonicalJson,
    'project_id': projectId,
    'project_revision': projectRevision,
    'output': output,
    'edit_count': 1,
    'file_count': 3,
    'bundle_bytes': 1234,
    'bundle_sha256': 'd' * 64,
    'build_authority': 'generation_sealed_existing_member_bundle_v1',
    'deployment_status': 'not_performed',
  },
  expectedHead: head,
  expectedProjectJson: revision3VoiceFixtureBuildReadyProjectJson(
    projectId: projectId,
    projectRevision: projectRevision,
  ),
  expectedOutput: output,
);

AuthoringRevision3ExactSnapshotExportResult _projectExportResult({
  required AuthoringWorkingHead head,
  required String projectId,
  required int projectRevision,
  required String output,
  AuthoringRevision3ExactSnapshotExportOutcome outcome =
      AuthoringRevision3ExactSnapshotExportOutcome.exported,
}) {
  final (outcomeName, publicationStatus, warning) = switch (outcome) {
    AuthoringRevision3ExactSnapshotExportOutcome.exported => (
      'exported',
      'published',
      null,
    ),
    AuthoringRevision3ExactSnapshotExportOutcome.exportedWithCleanupWarning => (
      'exported_with_cleanup_warning',
      'published_with_cleanup_warning',
      <String, Object?>{
        'code': 'AUTHORING_REVISION3_EXPORT_CLEANUP_WARNING',
        'message':
            'the verified snapshot was published, but private staging cleanup was incomplete',
      },
    ),
    AuthoringRevision3ExactSnapshotExportOutcome.publicationUncertain => (
      'publication_uncertain',
      'publication_uncertain',
      <String, Object?>{
        'code': 'AUTHORING_REVISION3_EXPORT_PUBLICATION_UNCERTAIN',
        'message': 'publication may have completed; do not retry automatically',
      },
    ),
  };
  return AuthoringRevision3ExactSnapshotExportResult.fromJson(
    <String, Object?>{
      'ok': true,
      'outcome': outcomeName,
      'format': 'managed_revision3_exact_snapshot_v1',
      'artifact_kind': 'portable_snapshot_review_copy',
      'restore_status': 'not_supported',
      'basis_head_json': head.canonicalJson,
      'project_id': projectId,
      'project_revision': projectRevision,
      'output': output,
      'archive': <String, Object?>{
        'byte_len': 300,
        'sha256': List<String>.filled(64, 'a').join(),
      },
      'manifest': <String, Object?>{
        'relative_name': 'gore-export.json',
        'byte_len': 100,
        'sha256': List<String>.filled(64, 'b').join(),
      },
      'closure': <String, Object?>{
        'snapshot_objects': 1,
        'entity_objects': 0,
        'asset_objects': 0,
        'archive_entries': 4,
        'uncompressed_bytes': 200,
      },
      'publication_status': publicationStatus,
      'retry_safe': false,
      'warning': warning,
      'project_mutation': 'not_performed',
      'game_mutation': 'not_performed',
      'save_mutation': 'not_performed',
      'build_status': 'not_performed',
      'deployment_status': 'not_performed',
      'runtime_status': 'runtime_unqualified',
    },
    expectedHead: head,
    expectedOutput: output,
  );
}

AuthoringRevision3ReviewedDataAssetBuildResult _reviewedDataAssetBuildResult({
  required AuthoringWorkingHead head,
  required String projectId,
  required int projectRevision,
  required String targetPath,
  required String packName,
  required String output,
  bool publicationUncertain = false,
}) => AuthoringRevision3ReviewedDataAssetBuildResult.fromJson(
  <String, Object?>{
    'ok': true,
    'outcome': publicationUncertain ? 'publication_uncertain' : 'built',
    'basis_head_json': head.canonicalJson,
    'project_id': projectId,
    'project_revision': projectRevision,
    'target_path': targetPath,
    'pack_name': packName,
    'output': output,
    'files': <Object?>[
      <String, Object?>{
        'relative_name': '$packName.pak',
        'byte_len': 101,
        'sha256': List<String>.filled(64, 'a').join(),
      },
      <String, Object?>{
        'relative_name': '$packName.ucas',
        'byte_len': 102,
        'sha256': List<String>.filled(64, 'b').join(),
      },
      <String, Object?>{
        'relative_name': '$packName.utoc',
        'byte_len': 103,
        'sha256': List<String>.filled(64, 'c').join(),
      },
    ],
    'receipt': <String, Object?>{
      'format':
          'gore.authoring.managed-revision3-reviewed-dataasset-build-receipt.v1',
      'relative_name': 'gore-authoring-dataasset-build.json',
      'byte_len': 456,
      'sha256': List<String>.filled(64, 'd').join(),
    },
    'build_authority': 'reviewed_fixed_leaf_single_package_triplet',
    'artifact_publication_status': publicationUncertain
        ? 'publication_uncertain'
        : 'published',
    'deployment_status': 'not_performed',
    'runtime_status': 'runtime_unqualified',
    'retry_safe': false,
    'warning': publicationUncertain
        ? <String, Object?>{
            'code': 'AUTHORING_REVISION3_DATAASSET_BUILD_PUBLICATION_UNCERTAIN',
            'message':
                'publication may have completed; do not retry automatically',
          }
        : null,
  },
  expectedHead: head,
  expectedProjectJson: revision3VoiceFixtureProjectJson(
    revision: projectRevision,
  ),
  expectedTargetPath: targetPath,
  expectedPackName: packName,
  expectedOutput: output,
);

Future<ManagedRevision3CompilerCheckReceipt> _checkControllerManagedCompiler(
  CurrentProjectCoordinator coordinator,
  ManagedRevision3CurrentProjectState visible,
  Revision3QuestOutlineFixture fixture, {
  String? expectedRoot,
  String? expectedProjectId,
  int? expectedProjectRevision,
  AuthoringWorkingHead? expectedHead,
  int? expectedEntityRevision,
  String? expectedModuleId,
  int? expectedModuleRevision,
}) => coordinator.checkCurrentRevision3ManagedCompiler(
  expectedRoot: expectedRoot ?? visible.root.path,
  expectedProjectId: expectedProjectId ?? visible.projectId,
  expectedProjectRevision: expectedProjectRevision ?? visible.projectRevision,
  expectedHead: expectedHead ?? visible.head,
  entityKind: AuthoringRevision3ManagedCompilerEntityKind.questDraft,
  entityId: revision3QuestOutlineQuestId,
  expectedEntityRevision: expectedEntityRevision ?? fixture.questRevision,
  expectedModuleId: expectedModuleId ?? revision3QuestOutlineModuleId,
  expectedModuleRevision: expectedModuleRevision ?? fixture.moduleRevision,
  gameRoot: r'C:\Games\Gothic Remake',
);

AuthoringRevision3ManagedCompilerCheckResult
_controllerManagedCompilerCheckResult({
  required AuthoringWorkingHead head,
  required String projectJson,
  bool recoveryRequired = false,
}) {
  final project = (jsonDecode(projectJson) as Map).cast<String, Object?>();
  final entities = (project['entities']! as Map).cast<String, Object?>();
  final entity = (entities[revision3QuestOutlineQuestId]! as Map)
      .cast<String, Object?>();
  final module = (entities[revision3QuestOutlineModuleId]! as Map)
      .cast<String, Object?>();
  final modulePayload = (module['payload']! as Map).cast<String, Object?>();
  final moduleData = (modulePayload['data']! as Map).cast<String, Object?>();
  final projectBytes = utf8.encode(projectJson);
  final compiler = recoveryRequired
      ? <String, Object?>{
          'outcome': 'failed',
          'compile_error': <String, Object?>{
            'code': 'COMPILE_INSTALL_RECOVERY_REQUIRED',
            'message': 'restore requires explicit recovery',
          },
          'compiler_diagnostics': null,
          'install_restore': 'not_started',
          'recovery_required': true,
          'output_discarded': true,
        }
      : <String, Object?>{
          'outcome': 'compiled_evidence_only',
          'compile_error': null,
          'compiler_diagnostics': <String, Object?>{
            'capture': 'captured',
            'messages': <Object?>[],
            'omitted': 0,
          },
          'install_restore': 'restored_exact',
          'recovery_required': false,
          'output_discarded': true,
        };
  return AuthoringRevision3ManagedCompilerCheckResult.fromJson(
    <String, Object?>{
      'ok': true,
      'outcome': 'compiler_check_only',
      'exact_current': !recoveryRequired,
      'head_json': head.canonicalJson,
      'project': <String, Object?>{
        'id': project['project_id'],
        'revision': project['revision'],
        'seal': <String, Object?>{
          'byte_len': projectBytes.length,
          'sha256': crypto.sha256.convert(projectBytes).toString(),
        },
      },
      'entity': <String, Object?>{
        'kind': 'quest_draft',
        'id': revision3QuestOutlineQuestId,
        'revision': entity['revision'],
      },
      'module': <String, Object?>{
        'id': revision3QuestOutlineModuleId,
        'revision': module['revision'],
        'namespace': moduleData['module_namespace'],
        'relative_path': moduleData['module_relative_path'],
        'source_sha256': moduleData['source_sha256'],
      },
      'compiler': compiler,
      'scope': 'compiler_check_only',
      'build_status': 'blocked',
      'deploy_status': 'not_supported',
      'runtime_qualification': 'runtime_unqualified',
      'publication_status': 'not_supported',
    },
    expectedHead: head,
    requestedEntityId: revision3QuestOutlineQuestId,
    expectedKind: AuthoringRevision3ManagedCompilerEntityKind.questDraft,
  );
}

String _recoveryProjectJson({
  required String projectId,
  required int revision,
  int format = 2,
  int schemaRevision = 3,
  Map<String, Object?>? target,
}) => jsonEncode(<String, Object?>{
  'format': format,
  'schema_revision': schemaRevision,
  'project_id': projectId,
  'revision': revision,
  'target':
      target ??
      <String, Object?>{
        'executable': <String, Object?>{
          'byte_len': 171698176,
          'sha256':
              'f406f969d3e73b6e58ea6e7aa10df7380318d97e7974d3be6e5a01183a4524f5',
        },
      },
  'entities': <String, Object?>{},
  'asset_store': <String, Object?>{'assets': <String, Object?>{}},
});

AuthoringWorkingHead _recoveryHead(String projectJson) {
  final bytes = utf8.encode(projectJson);
  return AuthoringWorkingHead.fromCanonicalJson(
    jsonEncode(<String, Object?>{
      'store_format': 1,
      'snapshot': <String, Object?>{
        'byte_len': bytes.length,
        'sha256': crypto.sha256.convert(bytes).toString(),
      },
    }),
  );
}

AuthoringWorkingHead _head(int value) => AuthoringWorkingHead.fromCanonicalJson(
  jsonEncode(<String, Object?>{
    'store_format': 1,
    'snapshot': <String, Object?>{
      'byte_len': value + 1,
      'sha256': value.toRadixString(16).padLeft(64, '0'),
    },
  }),
);

Revision3ContentIndex _contentIndex({
  required String projectId,
  required int revision,
}) => Revision3ContentIndex.fromJsonObject(<String, Object?>{
  'schema_revision': 1,
  'project_id': projectId,
  'project_revision': revision,
  'project_name': 'Controller content',
  'project_version': '1.0.0',
  'project_author': 'tests',
  'target': <String, Object?>{
    'executable': <String, Object?>{
      'byte_len': 1,
      'sha256': List<String>.filled(64, '5').join(),
    },
  },
  'authoring_locales': <Object?>[],
  'entity_counts': <String, Object?>{},
  'entities': <Object?>[],
  'assets': <Object?>[],
});

AuthoringRevision3DialogLocalizationReadResult
_controllerDialogLocalizationReadResult({
  required AuthoringWorkingHead head,
  required String projectId,
  required int projectRevision,
  required String localizationId,
  required int localizationRevision,
  required String locId,
}) {
  final request = AuthoringRevision3DialogLocalizationReadRequestV1(
    expectedHead: head,
    localizationId: localizationId,
    expectedLocalizationRevision: localizationRevision,
    expectedLocId: locId,
  );
  return AuthoringRevision3DialogLocalizationReadResult.fromJson(
    <String, Object?>{
      'ok': true,
      'outcome': 'read_only',
      'head_json': head.canonicalJson,
      'project_id': projectId,
      'project_revision': projectRevision,
      'localization_id': localizationId,
      'localization_revision': localizationRevision,
      'loc_id': locId,
      'locales': <Object?>[
        <String, Object?>{
          'locale': 'de',
          'preview': 'Bleib stehen!',
          'truncated': false,
          'has_nonempty_text': true,
        },
      ],
      'content_authority': 'read_only_exact_current_localization',
      'build_status': 'not_evaluated',
      'runtime_status': 'runtime_unqualified',
      'publication_status': 'not_applicable',
    },
    request: request,
  );
}

Revision3ContentIndex _controllerDialogLocalizationEditIndex({
  required String projectId,
  required int projectRevision,
  required String localizationId,
  required int localizationRevision,
  required String locId,
}) => Revision3ContentIndex.fromJsonObject(<String, Object?>{
  'schema_revision': 1,
  'project_id': projectId,
  'project_revision': projectRevision,
  'project_name': 'Controller localization edit',
  'project_version': '1.0.0',
  'project_author': 'tests',
  'target': <String, Object?>{
    'executable': <String, Object?>{
      'byte_len': 1,
      'sha256': List<String>.filled(64, '5').join(),
    },
  },
  'authoring_locales': <Object?>['de', 'en'],
  'entity_counts': <String, Object?>{'localization_entry': 1},
  'entities': <Object?>[
    <String, Object?>{
      'id': localizationId,
      'kind': 'localization_entry',
      'display_name': 'Asghan warning',
      'revision': localizationRevision,
      'origin': <String, Object?>{'type': 'new', 'authored_runtime_id': locId},
      'summary': <String, Object?>{
        'kind': 'localization_entry',
        'data': <String, Object?>{
          'loc_id': locId,
          'locales': <Object?>['de', 'en'],
        },
      },
      'references': <Object?>[],
      'asset_references': <Object?>[],
    },
  ],
  'assets': <Object?>[],
});

AuthoringRevision3DialogLocalizationEditSeed
_controllerDialogLocalizationEditSeed({
  required AuthoringWorkingHead head,
  required String projectId,
  required int projectRevision,
  required String localizationId,
  required int localizationRevision,
  required String locId,
}) {
  final request = AuthoringRevision3DialogLocalizationEditSeedRequestV1(
    expectedHead: head,
    localizationId: localizationId,
    expectedLocalizationRevision: localizationRevision,
    expectedLocId: locId,
  );
  return AuthoringRevision3DialogLocalizationEditSeed.fromJson(
    <String, Object?>{
      'ok': true,
      'outcome': 'read_only',
      'head_json': head.canonicalJson,
      'project_id': projectId,
      'project_revision': projectRevision,
      'localization_id': localizationId,
      'localization_revision': localizationRevision,
      'loc_id': locId,
      'locales': <Object?>[
        <String, Object?>{
          'locale': 'de',
          'text': 'Bleib stehen!',
          'voice_slot_present': false,
          'candidate_count': 0,
        },
        <String, Object?>{
          'locale': 'en',
          'text': 'Stop right there!',
          'voice_slot_present': false,
          'candidate_count': 0,
        },
      ],
      'line_backlinks': <Object?>[],
      'content_authority': 'read_only_exact_current_localization_edit_seed',
      'build_status': 'not_evaluated',
      'runtime_status': 'runtime_unqualified',
      'publication_status': 'not_applicable',
    },
    request: request,
  );
}

Future<Revision3DialogLocalizationEditTechnicalPlan>
_controllerDialogLocalizationEditPlan({
  required AuthoringWorkingHead head,
  required String projectId,
  required int projectRevision,
  required String localizationId,
  required int localizationRevision,
  required String locId,
}) async {
  final index = _controllerDialogLocalizationEditIndex(
    projectId: projectId,
    projectRevision: projectRevision,
    localizationId: localizationId,
    localizationRevision: localizationRevision,
    locId: locId,
  );
  final exact = _controllerDialogLocalizationEditSeed(
    head: head,
    projectId: projectId,
    projectRevision: projectRevision,
    localizationId: localizationId,
    localizationRevision: localizationRevision,
    locId: locId,
  );
  late Revision3DialogLocalizationEditTechnicalPlan captured;
  final service = Revision3DialogLocalizationEditAuthoringService(
    loadContentIndex: () async => index,
    loadExactSeed:
        ({
          required expectedProjectId,
          required expectedProjectRevision,
          required localizationId,
          required expectedLocalizationRevision,
          required expectedLocId,
        }) async => exact,
    publishTechnicalPlan:
        ({
          required expectedProjectId,
          required expectedProjectRevision,
          required plan,
        }) async {
          captured = plan;
          return Revision3DialogLocalizationEditPublication(
            projectId: expectedProjectId,
            projectRevision: expectedProjectRevision + 1,
            localizationId: localizationId,
            localizationRevision: localizationRevision + 1,
            addedLocales: const <String>[],
            removedLocales: const <String>[],
          );
        },
  );
  final catalog = await service.loadCatalog();
  final seed = await service.loadSeed(
    catalog: catalog,
    choice: catalog.choices.single,
  );
  await service.publish(
    seed: seed,
    input: Revision3DialogLocalizationEditInput(
      texts: const <String, String>{
        'de': 'Geänderter Text.',
        'en': 'Changed text.',
      },
    ),
  );
  return captured;
}

AuthoringRevision3DataAssetPackageIndexResult
_controllerDataAssetPackageIndexResult({
  required AuthoringWorkingHead head,
  required String projectId,
  required int projectRevision,
  String targetPath = '/Game/Characters/DA_Asghan',
  String packageIdHex = '0123456789abcdef',
}) {
  final indexJson = jsonEncode(<String, Object?>{
    'status': 'complete_index',
    'physical_chunk_count': 1,
    'winning_export_bundle_count': 1,
    'directory_indexed_export_bundle_count': 1,
    'out_of_scope_export_bundle_count': 0,
    'candidates': <Object?>[
      <String, Object?>{
        'target_path': targetPath,
        'package_id_hex': packageIdHex,
      },
    ],
    'partial_reasons': <Object?>[],
  });
  final indexBytes = utf8.encode(indexJson);
  Map<String, Object?> seal(int byteLength, String sha256) => <String, Object?>{
    'byte_len': byteLength,
    'sha256': sha256,
  };
  return AuthoringRevision3DataAssetPackageIndexResult.fromJson(
    <String, Object?>{
      'authority_status': 'not_granted',
      'build_status': 'not_evaluated',
      'candidate_count': 1,
      'content_status': 'metadata_candidates_only',
      'export_bundle_payload_status': 'not_read',
      'head_json': head.canonicalJson,
      'mount_inventory_entry_count': 2,
      'mount_inventory_seal': seal(80, 'b' * 64),
      'mutation_status': 'not_supported',
      'ok': true,
      'outcome': 'audit_only',
      'package_index_json': indexJson,
      'package_index_seal': seal(
        indexBytes.length,
        crypto.sha256.convert(indexBytes).toString(),
      ),
      'package_index_status': 'complete_index',
      'project_id': projectId,
      'project_revision': projectRevision,
      'publication_status': 'not_supported',
      'runtime_status': 'runtime_unqualified',
      'scope': 'installed_dataasset_package_candidates_only',
      'source_snapshot_seal': seal(120, 'c' * 64),
      'target_executable_seal': seal(171698176, 'd' * 64),
    },
    expectedHead: head,
  );
}

AuthoringRevision3InstalledDataAssetInspectionResult
_controllerInstalledDataAssetInspectionResult({
  required AuthoringRevision3DataAssetPackageIndexResult expectedSnapshot,
  required AuthoringRevision3DataAssetPackageCandidate candidate,
  bool reviewedFootstep = false,
}) => AuthoringRevision3InstalledDataAssetInspectionResult.fromJson(
  <String, Object?>{
    'authority_status': 'not_granted',
    'build_status': 'not_evaluated',
    'candidate_ordinal': candidate.ordinal,
    'head_json': expectedSnapshot.head.canonicalJson,
    'inspection': reviewedFootstep
        ? _controllerReviewedFootstepInspectionResponse()
        : validDataAssetInspectionResponse(),
    'mutation_status': 'not_supported',
    'ok': true,
    'outcome': 'inspection_only',
    'package_id_hex': candidate.packageIdHex,
    'package_index_seal': <String, Object?>{
      'byte_len': expectedSnapshot.packageIndexSeal.byteLength,
      'sha256': expectedSnapshot.packageIndexSeal.sha256,
    },
    'project_id': expectedSnapshot.projectId,
    'project_revision': expectedSnapshot.projectRevision,
    'publication_status': 'not_supported',
    'runtime_status': 'runtime_unqualified',
    'scope': 'selected_installed_dataasset_fixed_leaf_inspection_only',
    'source_snapshot_seal': <String, Object?>{
      'byte_len': expectedSnapshot.sourceSnapshotSeal.byteLength,
      'sha256': expectedSnapshot.sourceSnapshotSeal.sha256,
    },
    'target_path': candidate.targetPath,
    'usmap_content_seal': <String, Object?>{
      'byte_len': 256,
      'sha256': 'c' * 64,
    },
    'usmap_inventory_seal': <String, Object?>{
      'byte_len': 96,
      'sha256': 'e' * 64,
    },
  },
  expectedSnapshot: expectedSnapshot,
  requestedOrdinal: candidate.ordinal,
);

const _controllerReviewedWolfTarget =
    '/Game/Blueprints/TrackingSystem/FootstepsPresets/DA_WolfFootsteps';

Map<String, Object?> _controllerReviewedFootstepInspectionResponse() {
  final inspection = validDataAssetInspectionResponse(
    objectName: 'DA_WolfFootsteps',
  );
  final export = dataAssetExport(inspection);
  export
    ..['class_path'] = '/Script/G1R.FootstepTag'
    ..['schema'] = '/Script/G1R.FootstepTag';
  final selector = dataAssetSelector(inspection);
  selector
    ..['class_path'] = '/Script/G1R.FootstepTag'
    ..['kind'] = 'vector4_f64x4'
    ..['path'] = <Object?>[
      <String, Object?>{
        'step': 'property',
        'schema_index': 0,
        'property_name': 'BoneData',
        'array_index': 0,
        'array_dimension': 1,
        'declaring_schema_name': 'FootstepTag',
        'declaring_module_path': '/Script/G1R',
        'property_type': <String, Object?>{
          'type': 'struct',
          'name': 'BoneFeetData',
        },
      },
      <String, Object?>{
        'step': 'struct',
        'name': 'BoneFeetData',
        'schema_name': '/Script/G1R.BoneFeetData',
      },
      <String, Object?>{
        'step': 'property',
        'schema_index': 0,
        'property_name': 'FeetTextureSize',
        'array_index': 0,
        'array_dimension': 1,
        'declaring_schema_name': 'BoneFeetData',
        'declaring_module_path': '/Script/G1R',
        'property_type': <String, Object?>{'type': 'struct', 'name': 'Vector4'},
      },
    ]
    ..['expected_hex'] =
        '000000000000244000000000000024400000000000000000000000000000f03f';
  return inspection;
}

ReviewedInstalledDataAssetEditIntent _controllerReviewedDataAssetIntent({
  required AuthoringWorkingHead head,
  required String projectId,
  required int projectRevision,
}) {
  final snapshot = _controllerDataAssetPackageIndexResult(
    head: head,
    projectId: projectId,
    projectRevision: projectRevision,
    targetPath: _controllerReviewedWolfTarget,
    packageIdHex: '01e173a19ea374c9',
  );
  final candidate = snapshot.index.candidates.single;
  final inspection = _controllerInstalledDataAssetInspectionResult(
    expectedSnapshot: snapshot,
    candidate: candidate,
    reviewedFootstep: true,
  );
  return ReviewedInstalledDataAssetEditIntent.fromInspection(
    snapshot: snapshot,
    candidate: candidate,
    inspection: inspection,
    request: ReviewedDataAssetEditRequest.feetTextureSize(x: '12.5', y: '8'),
  );
}

AuthoringRevision3QuestSourceInspectionResult _controllerQuestInspectionResult({
  required AuthoringWorkingHead head,
  required String projectId,
  required int projectRevision,
  required String questId,
}) {
  const moduleId = '72727272727272727272727272727272';
  const source = '''class UQuest_GoreControllerInspection : UQuest
{
    void OnStart() {}
}
''';
  final sourceBytes = utf8.encode(source);
  final sourceSha = crypto.sha256.convert(sourceBytes).toString();
  final projectBytes = utf8.encode(
    'controller Quest inspection $projectId@$projectRevision',
  );
  final projectSeal = <String, Object?>{
    'byte_len': projectBytes.length,
    'sha256': crypto.sha256.convert(projectBytes).toString(),
  };
  Map<String, Object?> seal(int byteLength, String digit) => <String, Object?>{
    'byte_len': byteLength,
    'sha256': List<String>.filled(64, digit).join(),
  };
  Map<String, Object?> typedRef(String id, String kind) => <String, Object?>{
    'project_id': projectId,
    'id': id,
    'expected_kind': kind,
  };
  final planJson = jsonEncode(<String, Object?>{
    'format': 'revision3_quest_source_inspection_plan',
    'schema_revision': 3,
    'scope': 'source_inspection_only',
    'build_status': 'blocked',
    'runtime_qualification': 'runtime_unqualified',
    'publication_status': 'not_supported',
    'provenance': <String, Object?>{
      'project_id': projectId,
      'project_revision': projectRevision,
      'target_executable': seal(171698176, '2'),
      'canonical_project': projectSeal,
      'collision_basis_head': jsonDecode(head.canonicalJson),
      'collision_basis_project': seal(1024, '3'),
      'collision_nonquest_project': seal(900, '4'),
      'collision_prior_quest_count': 2,
      'collision_prior_quest_evidence': seal(300, '5'),
      'collision_artifact': seal(700, '6'),
      'collision_source': seal(700, '7'),
    },
    'module': <String, Object?>{
      'quest': typedRef(questId, 'quest_draft'),
      'script_module': typedRef(moduleId, 'script_module'),
      'draft_input': seal(420, '8'),
      'persisted_source': <String, Object?>{
        'byte_len': sourceBytes.length,
        'sha256': sourceSha,
      },
      'generated': <String, Object?>{
        'generator_id': 'gore-authoring.draft-quest-skeleton',
        'generator_version': 4,
        'owner': typedRef(questId, 'quest_draft'),
        'module_namespace': 'GoreMods.Quests.ControllerInspection',
        'module_relative_path': 'GoreMods/Quests/ControllerInspection.as',
        'source': source,
        'source_sha256': sourceSha,
        'input_fingerprint': List<String>.filled(64, '9').join(),
        'status': <String, Object?>{
          'authoring': 'offline_draft',
          'runtime': 'runtime_unqualified',
        },
      },
    },
  });
  final planBytes = utf8.encode(planJson);
  return AuthoringRevision3QuestSourceInspectionResult.fromJson(
    <String, Object?>{
      'ok': true,
      'outcome': 'inspection_only',
      'head_json': head.canonicalJson,
      'project_id': projectId,
      'project_revision': projectRevision,
      'project_seal': projectSeal,
      'quest_id': questId,
      'plan_json': planJson,
      'plan_seal': <String, Object?>{
        'byte_len': planBytes.length,
        'sha256': crypto.sha256.convert(planBytes).toString(),
      },
      'scope': 'source_inspection_only',
      'build_status': 'blocked',
      'runtime_qualification': 'runtime_unqualified',
      'publication_status': 'not_supported',
    },
    expectedHead: head,
    requestedQuestId: questId,
  );
}

AuthoringRevision3DataAssetStage _dataAssetStage({String? targetPath}) {
  final basis = revision3DataAssetNativeGoldenFixture();
  final fixture = targetPath == null
      ? basis
      : Revision3DataAssetFixture.fromBasis(
          basisHead: basis.basisHead,
          basisProjectJson: basis.basisProjectJson,
          targetPath: targetPath,
        );
  return AuthoringRevision3DataAssetStageListResult.fromJson(
    fixture.listResponse(),
    expectedHead: fixture.stagedHead,
  ).stages.single;
}

DataAssetSemanticEditIntent _dataAssetSemanticIntent() {
  final inspection = DataAssetInspection.fromJson(
    validDataAssetInspectionResponse(),
  );
  return DataAssetSemanticValueEditor.fromLeaf(
        inspection.exports.single.leaves.single,
      )
      .previewScalar(
        extractReceiptPath: r'C:\proof\extract-receipt.v2.json',
        expectedTargetPath: '/Game/TestAsset',
        value: '2',
      )
      .intent;
}

Revision3QuestDraftAuthoringInput _questInput() =>
    Revision3QuestDraftAuthoringInput(
      parentCatalogId: 'chapter-one',
      giverCatalogId: 'asghan',
      title: 'Find Homer',
      description: 'Homer vanished near the old gate.',
      objectiveTitle: 'Ask Asghan about Homer',
    );

Revision3NpcDraftAuthoringInput _npcInput() => Revision3NpcDraftAuthoringInput(
  parentCatalogId: 'g1r:npc:om_grd_asghan_263',
  displayName: 'North Gate Guard',
);
