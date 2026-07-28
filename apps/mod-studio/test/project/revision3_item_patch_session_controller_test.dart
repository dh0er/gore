import 'dart:collection';
import 'dart:convert';
import 'dart:io';

import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/core/mod_ffi.dart';
import 'package:gore_mod/project/current_project_controller.dart';
import 'package:gore_mod/project/managed_project_session.dart';
import 'package:gore_mod/project/revision3_content_index.dart';
import 'package:gore_mod/project/revision3_item_patch_authoring.dart';
import 'package:path/path.dart' as p;

import '../support/revision3_item_patch_fixture.dart';

const _projectId = '11111111111111111111111111111111';
const _patchedEntityId = '22222222222222222222222222222222';
const _otherEntityId = '33333333333333333333333333333333';
const _vanillaClass = 'ItFo_Apple';
const _catalogLayer = 'base-game.items.g1r.bundled.v1';
const _targetBytes = 171698176;
const _targetSha =
    'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa';

void main() {
  late Directory fixture;

  setUp(() async {
    fixture = await Directory.systemTemp.createTemp(
      'gore_revision3_item_wiring_',
    );
  });

  tearDown(() async {
    if (await fixture.exists()) await fixture.delete(recursive: true);
  });

  test(
    'managed session re-reads native Item schema then publishes through full reopen CAS',
    () async {
      final root = Directory(p.join(fixture.path, 'managed-items'));
      await root.create();
      final store = _FakeItemPatchStore();
      final projectJson = _projectJson(revision: 7);
      final session = await ManagedRevision3AuthoringProjectSession.create(
        root: root,
        store: store,
        projectJson: projectJson,
      );
      addTearDown(session.close);
      final basisHead = session.head;

      final catalog = await session.readItemCatalogV1();
      final plan = await _createPlan(
        head: basisHead,
        projectJson: projectJson,
        catalog: catalog,
      );
      final publication = await session.prepareAndPublishItemPatchV1(
        plan: plan,
      );

      expect(store.catalogReadCalls, 2);
      expect(store.prepareItemPatchCalls, 1);
      expect(
        store.itemPatchRequests.single.expectedHead.canonicalJson,
        basisHead.canonicalJson,
      );
      expect(publication.projectId, _projectId);
      expect(publication.projectRevision, 8);
      expect(publication.entityId, plan.entityId);
      expect(publication.entityRevision, 0);
      expect(publication.change, AuthoringRevision3ItemPatchChange.created);
      expect(publication.vanillaClass, _vanillaClass);
      expect(session.projectRevision, 8);
      expect(session.head.canonicalJson, isNot(basisHead.canonicalJson));
      expect(
        await File(p.join(root.path, 'gore-project.json')).readAsString(),
        session.head.canonicalJson,
      );
      expect(session.requiresReopen, isFalse);
    },
  );

  test(
    'managed session rejects a changed native Item schema before prepare without poison',
    () async {
      final root = Directory(p.join(fixture.path, 'stale-items'));
      await root.create();
      final store = _FakeItemPatchStore();
      final projectJson = _projectJson(revision: 7);
      final session = await ManagedRevision3AuthoringProjectSession.create(
        root: root,
        store: store,
        projectJson: projectJson,
      );
      addTearDown(session.close);
      final catalog = await session.readItemCatalogV1();
      final plan = await _createPlan(
        head: session.head,
        projectJson: projectJson,
        catalog: catalog,
      );
      store.catalogSealDigit = 'e';

      await expectLater(
        session.prepareAndPublishItemPatchV1(plan: plan),
        throwsA(isA<Revision3ItemPatchStaleCheckpointException>()),
      );

      expect(store.catalogReadCalls, 2);
      expect(store.prepareItemPatchCalls, 0);
      expect(session.projectRevision, 7);
      expect(session.requiresReopen, isFalse);
      await session.verifyCurrentHead();
    },
  );

  test('managed session poisons a wrong-target Item catalog read', () async {
    final root = Directory(p.join(fixture.path, 'wrong-target-read-items'));
    await root.create();
    final store = _FakeItemPatchStore()..wrongCatalogTarget = true;
    final session = await ManagedRevision3AuthoringProjectSession.create(
      root: root,
      store: store,
      projectJson: _projectJson(revision: 7),
    );
    addTearDown(session.close);

    await expectLater(
      session.readItemCatalogV1(),
      throwsA(isA<ManagedProjectVerificationException>()),
    );

    expect(store.catalogReadCalls, 1);
    expect(store.prepareItemPatchCalls, 0);
    expect(session.requiresReopen, isTrue);
  });

  test(
    'managed session reports unsupported current Item provenance without poison',
    () async {
      final root = Directory(p.join(fixture.path, 'unsupported-items'));
      await root.create();
      final store = _FakeItemPatchStore()
        ..nextCatalogError = const ModFfiException(
          command: 'authoring_store_read_revision3_item_catalog_v1',
          code: 'AUTHORING_REVISION3_ITEM_PATCH_PROVENANCE_CONFLICT',
          message: 'injected retired ItemPatch provenance',
        );
      final session = await ManagedRevision3AuthoringProjectSession.create(
        root: root,
        store: store,
        projectJson: _projectJson(revision: 7, patched: true),
      );
      addTearDown(session.close);

      await expectLater(
        session.readItemCatalogV1(),
        throwsA(isA<Revision3ItemPatchUnsupportedSchemaException>()),
      );

      expect(store.catalogReadCalls, 1);
      expect(store.prepareItemPatchCalls, 0);
      expect(session.projectRevision, 7);
      expect(session.requiresReopen, isFalse);
      await session.verifyCurrentHead();
    },
  );

  test(
    'managed session poisons a wrong-target Item catalog refresh before prepare',
    () async {
      final root = Directory(
        p.join(fixture.path, 'wrong-target-refresh-items'),
      );
      await root.create();
      final store = _FakeItemPatchStore();
      final projectJson = _projectJson(revision: 7);
      final session = await ManagedRevision3AuthoringProjectSession.create(
        root: root,
        store: store,
        projectJson: projectJson,
      );
      addTearDown(session.close);
      final catalog = await session.readItemCatalogV1();
      final plan = await _createPlan(
        head: session.head,
        projectJson: projectJson,
        catalog: catalog,
      );
      store.wrongCatalogTarget = true;

      await expectLater(
        session.prepareAndPublishItemPatchV1(plan: plan),
        throwsA(isA<ManagedProjectVerificationException>()),
      );

      expect(store.catalogReadCalls, 2);
      expect(store.prepareItemPatchCalls, 0);
      expect(session.projectRevision, 7);
      expect(session.requiresReopen, isTrue);
    },
  );

  test('managed session removes only an exact current ItemPatch', () async {
    final root = Directory(p.join(fixture.path, 'revert-hotfix-items'));
    await root.create();
    final store = _FakeItemPatchStore();
    final projectJson = _projectJson(revision: 7, patched: true);
    final session = await ManagedRevision3AuthoringProjectSession.create(
      root: root,
      store: store,
      projectJson: projectJson,
    );
    addTearDown(session.close);
    final catalog = await session.readItemCatalogV1();
    final plan = await _removePlan(
      head: session.head,
      projectJson: projectJson,
      catalog: catalog,
    );

    final publication = await session.prepareAndPublishItemPatchV1(plan: plan);

    expect(store.catalogReadCalls, 2);
    expect(store.prepareItemPatchCalls, 1);
    final request = store.itemPatchRequests.single;
    expect(request.action, AuthoringRevision3ItemPatchAction.remove);
    expect(request.catalogLayer, _catalogLayer);
    expect(request.sourceSeal.sha256, _digitSeal('d'));
    expect(request.expectedCatalogSeal.sha256, _digitSeal('c'));
    expect(publication.change, AuthoringRevision3ItemPatchChange.removed);
    expect(publication.entityId, _patchedEntityId);
    expect(publication.entityRevision, isNull);
    expect(session.projectRevision, 8);
    expect(session.requiresReopen, isFalse);
  });

  test(
    'managed session rejects changed provenance for remove before native prepare',
    () async {
      final root = Directory(p.join(fixture.path, 'stale-remove-items'));
      await root.create();
      final store = _FakeItemPatchStore();
      final projectJson = _projectJson(revision: 7, patched: true);
      final session = await ManagedRevision3AuthoringProjectSession.create(
        root: root,
        store: store,
        projectJson: projectJson,
      );
      addTearDown(session.close);
      final catalog = await session.readItemCatalogV1();
      final plan = await _removePlan(
        head: session.head,
        projectJson: projectJson,
        catalog: catalog,
      );
      store.catalogSourceDigit = 'e';

      await expectLater(
        session.prepareAndPublishItemPatchV1(plan: plan),
        throwsA(isA<Revision3ItemPatchStaleCheckpointException>()),
      );

      expect(store.catalogReadCalls, 2);
      expect(store.prepareItemPatchCalls, 0);
      expect(session.projectRevision, 7);
      expect(session.requiresReopen, isFalse);
    },
  );

  test('managed session poisons an ItemPatch head conflict', () async {
    final root = Directory(p.join(fixture.path, 'conflicting-items'));
    await root.create();
    final store = _FakeItemPatchStore();
    final projectJson = _projectJson(revision: 7);
    final session = await ManagedRevision3AuthoringProjectSession.create(
      root: root,
      store: store,
      projectJson: projectJson,
    );
    addTearDown(session.close);
    final catalog = await session.readItemCatalogV1();
    final plan = await _createPlan(
      head: session.head,
      projectJson: projectJson,
      catalog: catalog,
    );
    store.nextPrepareError = const ModFfiException(
      command: 'authoring_store_prepare_revision3_item_patch_v1',
      code: 'AUTHORING_REVISION3_ITEM_PATCH_HEAD_CONFLICT',
      message: 'injected head conflict',
    );

    await expectLater(
      session.prepareAndPublishItemPatchV1(plan: plan),
      throwsA(isA<ManagedProjectHeadConflictException>()),
    );

    expect(store.prepareItemPatchCalls, 1);
    expect(session.requiresReopen, isTrue);
  });

  test(
    'managed session keeps an explicit retryable ItemPatch prepare error non-poisoning',
    () async {
      final root = Directory(p.join(fixture.path, 'retryable-items'));
      await root.create();
      final store = _FakeItemPatchStore();
      final projectJson = _projectJson(revision: 7);
      final session = await ManagedRevision3AuthoringProjectSession.create(
        root: root,
        store: store,
        projectJson: projectJson,
      );
      addTearDown(session.close);
      final catalog = await session.readItemCatalogV1();
      final plan = await _createPlan(
        head: session.head,
        projectJson: projectJson,
        catalog: catalog,
      );
      store.nextPrepareError = const ModFfiException(
        command: 'authoring_store_prepare_revision3_item_patch_v1',
        code: 'AUTHORING_REVISION3_ITEM_PATCH_INPUT_LIMIT',
        message: 'injected retryable input limit',
      );

      await expectLater(
        session.prepareAndPublishItemPatchV1(plan: plan),
        throwsA(
          isA<ModFfiException>().having(
            (error) => error.code,
            'code',
            'AUTHORING_REVISION3_ITEM_PATCH_INPUT_LIMIT',
          ),
        ),
      );

      expect(store.catalogReadCalls, 2);
      expect(store.prepareItemPatchCalls, 1);
      expect(session.projectRevision, 7);
      expect(session.requiresReopen, isFalse);
      await session.verifyCurrentHead();
    },
  );

  test('managed session poisons an unknown ItemPatch prepare error', () async {
    final root = Directory(p.join(fixture.path, 'unknown-prepare-items'));
    await root.create();
    final store = _FakeItemPatchStore();
    final projectJson = _projectJson(revision: 7);
    final session = await ManagedRevision3AuthoringProjectSession.create(
      root: root,
      store: store,
      projectJson: projectJson,
    );
    addTearDown(session.close);
    final catalog = await session.readItemCatalogV1();
    final plan = await _createPlan(
      head: session.head,
      projectJson: projectJson,
      catalog: catalog,
    );
    store.nextPrepareError = const ModFfiException(
      command: 'authoring_store_prepare_revision3_item_patch_v1',
      code: 'AUTHORING_REVISION3_ITEM_PATCH_UNKNOWN_RECEIPT',
      message: 'injected unknown prepare receipt',
    );

    await expectLater(
      session.prepareAndPublishItemPatchV1(plan: plan),
      throwsA(isA<ManagedProjectVerificationException>()),
    );

    expect(store.catalogReadCalls, 2);
    expect(store.prepareItemPatchCalls, 1);
    expect(session.projectRevision, 7);
    expect(session.requiresReopen, isTrue);
  });

  test(
    'queued ItemPatch publication from the old checkpoint becomes stale before native prepare',
    () async {
      final root = Directory(p.join(fixture.path, 'queued-stale-items'));
      await root.create();
      final store = _FakeItemPatchStore();
      final projectJson = _projectJson(revision: 7);
      final session = await ManagedRevision3AuthoringProjectSession.create(
        root: root,
        store: store,
        projectJson: projectJson,
      );
      addTearDown(session.close);
      final catalog = await session.readItemCatalogV1();
      final plan = await _createPlan(
        head: session.head,
        projectJson: projectJson,
        catalog: catalog,
      );

      final first = session.prepareAndPublishItemPatchV1(plan: plan);
      final queued = session.prepareAndPublishItemPatchV1(plan: plan);

      final publication = await first;
      await expectLater(
        queued,
        throwsA(isA<Revision3ItemPatchStaleCheckpointException>()),
      );
      expect(publication.projectRevision, 8);
      expect(store.catalogReadCalls, 2);
      expect(store.prepareItemPatchCalls, 1);
      expect(session.projectRevision, 8);
      expect(session.requiresReopen, isFalse);
    },
  );

  test(
    'checkpoint-only managed session does not claim Item authority',
    () async {
      final root = Directory(p.join(fixture.path, 'checkpoint-only-items'));
      await root.create();
      final delegate = _FakeItemPatchStore();
      final session = await ManagedRevision3AuthoringProjectSession.create(
        root: root,
        store: _CheckpointOnlyItemStore(delegate),
        projectJson: _projectJson(revision: 7),
      );
      addTearDown(session.close);

      expect(session.supportsItemPatch, isFalse);
      await expectLater(
        session.readItemCatalogV1(),
        throwsA(isA<UnsupportedError>()),
      );
      expect(delegate.catalogReadCalls, 0);
      expect(session.requiresReopen, isFalse);
      await session.verifyCurrentHead();
    },
  );

  test(
    'current-project coordinator binds Item catalog and publication to the visible checkpoint',
    () async {
      final projectJson = _projectJson(revision: 7);
      final basisHead = _head(7);
      final catalog = _catalogResult(head: basisHead, projectJson: projectJson);
      final plan = await _createPlan(
        head: basisHead,
        projectJson: projectJson,
        catalog: catalog,
      );
      final lease = _FakeItemPatchLease(
        root: Directory(p.join(fixture.path, 'coordinator-items')),
        projectJson: projectJson,
        head: basisHead,
        catalog: catalog,
      );
      final coordinator = CurrentProjectCoordinator(
        openManagedRevision3: (_) async => lease,
      );
      addTearDown(() async {
        await coordinator.shutdown();
        coordinator.dispose();
      });
      final visible = await coordinator.openManagedRevision3(lease.root);

      expect(
        await coordinator.readCurrentRevision3ItemCatalogV1(
          expectedRoot: visible.root.path,
          expectedProjectId: visible.projectId,
          expectedProjectRevision: visible.projectRevision,
          expectedHead: visible.head,
        ),
        same(catalog),
      );
      await expectLater(
        coordinator.prepareAndPublishCurrentRevision3ItemPatchV1(
          expectedRoot: 'wrong-root',
          expectedProjectId: visible.projectId,
          expectedProjectRevision: visible.projectRevision,
          expectedHead: visible.head,
          plan: plan,
        ),
        throwsA(isA<Revision3ItemPatchStaleCheckpointException>()),
      );
      expect(lease.publishCalls, 0);

      final publication = await coordinator
          .prepareAndPublishCurrentRevision3ItemPatchV1(
            expectedRoot: visible.root.path,
            expectedProjectId: visible.projectId,
            expectedProjectRevision: visible.projectRevision,
            expectedHead: visible.head,
            plan: plan,
          );

      expect(publication.projectRevision, 8);
      expect(publication.change, AuthoringRevision3ItemPatchChange.created);
      expect(lease.catalogReadCalls, 1);
      expect(lease.publishCalls, 1);
      expect(lease.requiresReopen, isFalse);
      final refreshed =
          coordinator.state as ManagedRevision3CurrentProjectState;
      expect(refreshed.projectRevision, 8);
      expect(refreshed.head.canonicalJson, lease.head.canonicalJson);
    },
  );

  test(
    'current-project coordinator honors an unavailable Item capability',
    () async {
      final projectJson = _projectJson(revision: 7);
      final basisHead = _head(7);
      final lease = _FakeItemPatchLease(
        root: Directory(p.join(fixture.path, 'unavailable-items')),
        projectJson: projectJson,
        head: basisHead,
        catalog: _catalogResult(head: basisHead, projectJson: projectJson),
        supportsItemPatchValue: false,
      );
      final coordinator = CurrentProjectCoordinator(
        openManagedRevision3: (_) async => lease,
      );
      addTearDown(() async {
        await coordinator.shutdown();
        coordinator.dispose();
      });
      final visible = await coordinator.openManagedRevision3(lease.root);

      await expectLater(
        coordinator.readCurrentRevision3ItemCatalogV1(
          expectedRoot: visible.root.path,
          expectedProjectId: visible.projectId,
          expectedProjectRevision: visible.projectRevision,
          expectedHead: visible.head,
        ),
        throwsA(isA<CurrentProjectOperationUnsupportedException>()),
      );

      expect(lease.catalogReadCalls, 0);
      expect(lease.itemUncertaintyLatchCalls, 0);
      expect(lease.requiresReopen, isFalse);
    },
  );

  test(
    'current-project coordinator preserves unsupported Item schema without uncertainty',
    () async {
      final projectJson = _projectJson(revision: 7, patched: true);
      final basisHead = _head(7);
      final lease = _FakeItemPatchLease(
        root: Directory(p.join(fixture.path, 'unsupported-catalog-items')),
        projectJson: projectJson,
        head: basisHead,
        catalog: _catalogResult(head: basisHead, projectJson: projectJson),
        nextCatalogError: const Revision3ItemPatchUnsupportedSchemaException(),
      );
      final coordinator = CurrentProjectCoordinator(
        openManagedRevision3: (_) async => lease,
      );
      addTearDown(() async {
        await coordinator.shutdown();
        coordinator.dispose();
      });
      final visible = await coordinator.openManagedRevision3(lease.root);

      await expectLater(
        coordinator.readCurrentRevision3ItemCatalogV1(
          expectedRoot: visible.root.path,
          expectedProjectId: visible.projectId,
          expectedProjectRevision: visible.projectRevision,
          expectedHead: visible.head,
        ),
        throwsA(isA<Revision3ItemPatchUnsupportedSchemaException>()),
      );

      expect(lease.catalogReadCalls, 1);
      expect(lease.itemUncertaintyLatchCalls, 0);
      expect(lease.requiresReopen, isFalse);
    },
  );

  test(
    'current-project coordinator latches a mismatched Item receipt',
    () async {
      final projectJson = _projectJson(revision: 7);
      final basisHead = _head(7);
      final catalog = _catalogResult(head: basisHead, projectJson: projectJson);
      final plan = await _createPlan(
        head: basisHead,
        projectJson: projectJson,
        catalog: catalog,
      );
      final lease = _FakeItemPatchLease(
        root: Directory(p.join(fixture.path, 'bad-receipt-items')),
        projectJson: projectJson,
        head: basisHead,
        catalog: catalog,
        mismatchReceipt: true,
      );
      final coordinator = CurrentProjectCoordinator(
        openManagedRevision3: (_) async => lease,
      );
      addTearDown(() async {
        await coordinator.shutdown();
        coordinator.dispose();
      });
      final visible = await coordinator.openManagedRevision3(lease.root);

      await expectLater(
        coordinator.prepareAndPublishCurrentRevision3ItemPatchV1(
          expectedRoot: visible.root.path,
          expectedProjectId: visible.projectId,
          expectedProjectRevision: visible.projectRevision,
          expectedHead: visible.head,
          plan: plan,
        ),
        throwsA(isA<Revision3ItemPatchRequiresReopenException>()),
      );

      expect(lease.itemUncertaintyLatchCalls, 1);
      expect(lease.requiresReopen, isTrue);
      expect(
        (coordinator.state as ManagedRevision3CurrentProjectState)
            .requiresReopen,
        isTrue,
      );
    },
  );

  test(
    'current-project coordinator poisons an unknown Item catalog read error',
    () async {
      final projectJson = _projectJson(revision: 7);
      final basisHead = _head(7);
      final lease = _FakeItemPatchLease(
        root: Directory(p.join(fixture.path, 'unknown-read-items')),
        projectJson: projectJson,
        head: basisHead,
        catalog: _catalogResult(head: basisHead, projectJson: projectJson),
        nextCatalogError: const ModFfiException(
          command: 'authoring_store_read_revision3_item_catalog_v1',
          code: 'AUTHORING_REVISION3_ITEM_PATCH_UNKNOWN_RECEIPT',
          message: 'injected unknown read receipt',
        ),
      );
      final coordinator = CurrentProjectCoordinator(
        openManagedRevision3: (_) async => lease,
      );
      addTearDown(() async {
        await coordinator.shutdown();
        coordinator.dispose();
      });
      final visible = await coordinator.openManagedRevision3(lease.root);

      await expectLater(
        coordinator.readCurrentRevision3ItemCatalogV1(
          expectedRoot: visible.root.path,
          expectedProjectId: visible.projectId,
          expectedProjectRevision: visible.projectRevision,
          expectedHead: visible.head,
        ),
        throwsA(isA<Revision3ItemPatchRequiresReopenException>()),
      );

      expect(lease.catalogReadCalls, 1);
      expect(lease.itemUncertaintyLatchCalls, 1);
      expect(lease.requiresReopen, isTrue);
      expect(
        (coordinator.state as ManagedRevision3CurrentProjectState)
            .requiresReopen,
        isTrue,
      );
    },
  );

  test(
    'current-project coordinator poisons a mismatched Item catalog read receipt',
    () async {
      final projectJson = _projectJson(revision: 7);
      final basisHead = _head(7);
      final lease = _FakeItemPatchLease(
        root: Directory(p.join(fixture.path, 'mismatched-read-items')),
        projectJson: projectJson,
        head: basisHead,
        catalog: _catalogResult(
          head: basisHead,
          projectJson: projectJson,
          responseProjectRevision: 8,
        ),
      );
      final coordinator = CurrentProjectCoordinator(
        openManagedRevision3: (_) async => lease,
      );
      addTearDown(() async {
        await coordinator.shutdown();
        coordinator.dispose();
      });
      final visible = await coordinator.openManagedRevision3(lease.root);

      await expectLater(
        coordinator.readCurrentRevision3ItemCatalogV1(
          expectedRoot: visible.root.path,
          expectedProjectId: visible.projectId,
          expectedProjectRevision: visible.projectRevision,
          expectedHead: visible.head,
        ),
        throwsA(isA<Revision3ItemPatchRequiresReopenException>()),
      );

      expect(lease.catalogReadCalls, 1);
      expect(lease.itemUncertaintyLatchCalls, 1);
      expect(lease.requiresReopen, isTrue);
      expect(
        (coordinator.state as ManagedRevision3CurrentProjectState)
            .requiresReopen,
        isTrue,
      );
    },
  );
}

Future<Revision3ItemPatchTechnicalPlan> _createPlan({
  required AuthoringWorkingHead head,
  required String projectJson,
  required AuthoringRevision3ItemCatalogReadResult catalog,
}) async {
  final project = (jsonDecode(projectJson) as Map).cast<String, Object?>();
  late Revision3ItemPatchTechnicalPlan captured;
  final service = Revision3ItemPatchAuthoringService(
    projectScopeIdentity: 'test-project-root',
    projectId: project['project_id']! as String,
    projectRevision: project['revision']! as int,
    expectedHead: head,
    loadContentIndex: () async => _emptyContentIndex(projectJson),
    loadNativeCatalog: () async => catalog,
    publishTechnicalPlan: (plan) async {
      captured = plan;
      return Revision3ItemPatchPublication(
        projectId: plan.expectedProjectId,
        projectRevision: plan.expectedProjectRevision + 1,
        entityId: plan.entityId,
        entityRevision: 0,
        change: AuthoringRevision3ItemPatchChange.created,
        vanillaClass: plan.vanillaClass,
      );
    },
  );
  final chooser = await service.loadCatalog();
  await service.save(
    choice: chooser.choices.single,
    desiredOverrides: <String, AuthoringRevision3ItemScalarValue>{
      'm_Value': AuthoringRevision3ItemScalarValue.integer(9),
      'm_Weight': AuthoringRevision3ItemScalarValue.float(0.5),
    },
  );
  return captured;
}

Future<Revision3ItemPatchTechnicalPlan> _removePlan({
  required AuthoringWorkingHead head,
  required String projectJson,
  required AuthoringRevision3ItemCatalogReadResult catalog,
}) async {
  final project = (jsonDecode(projectJson) as Map).cast<String, Object?>();
  late Revision3ItemPatchTechnicalPlan captured;
  final service = Revision3ItemPatchAuthoringService(
    projectScopeIdentity: 'test-project-root',
    projectId: project['project_id']! as String,
    projectRevision: project['revision']! as int,
    expectedHead: head,
    loadContentIndex: () async => _patchedContentIndex(projectJson),
    loadNativeCatalog: () async => catalog,
    publishTechnicalPlan: (plan) async {
      captured = plan;
      return Revision3ItemPatchPublication(
        projectId: plan.expectedProjectId,
        projectRevision: plan.expectedProjectRevision + 1,
        entityId: plan.entityId,
        entityRevision: null,
        change: AuthoringRevision3ItemPatchChange.removed,
        vanillaClass: plan.vanillaClass,
      );
    },
  );
  final chooser = await service.loadCatalog();
  await service.save(
    choice: chooser.choices.single,
    desiredOverrides: const <String, AuthoringRevision3ItemScalarValue>{},
  );
  return captured;
}

Revision3ContentIndex _emptyContentIndex(String projectJson) {
  final project = (jsonDecode(projectJson) as Map).cast<String, Object?>();
  final meta = (project['meta']! as Map).cast<String, Object?>();
  return Revision3ContentIndex.fromJsonObject(<String, Object?>{
    'schema_revision': 1,
    'project_id': project['project_id'],
    'project_revision': project['revision'],
    'project_name': meta['name'],
    'project_version': meta['version'],
    'project_author': meta['author'],
    'target': project['target'],
    'authoring_locales': project['authoring_locales'],
    'entity_counts': <String, Object?>{},
    'entities': <Object?>[],
    'assets': <Object?>[],
  });
}

Revision3ContentIndex _patchedContentIndex(String projectJson) {
  final project = (jsonDecode(projectJson) as Map).cast<String, Object?>();
  final meta = (project['meta']! as Map).cast<String, Object?>();
  final target = (project['target']! as Map).cast<String, Object?>();
  final projectEntities = (project['entities']! as Map).cast<String, Object?>();
  final projectEntity = (projectEntities[_patchedEntityId]! as Map)
      .cast<String, Object?>();
  final payload = (projectEntity['payload']! as Map).cast<String, Object?>();
  final data = (payload['data']! as Map).cast<String, Object?>();
  final fields = (data['fields']! as Map).cast<String, Object?>();
  final fieldTypes = <String, Object?>{
    for (final field in fields.entries)
      field.key: ((field.value as Map)['type']! as String),
  };
  return Revision3ContentIndex.fromJsonObject(<String, Object?>{
    'schema_revision': 1,
    'project_id': project['project_id'],
    'project_revision': project['revision'],
    'project_name': meta['name'],
    'project_version': meta['version'],
    'project_author': meta['author'],
    'target': target,
    'authoring_locales': project['authoring_locales'],
    'entity_counts': <String, Object?>{'item_patch': 1},
    'entities': <Object?>[
      <String, Object?>{
        'id': _patchedEntityId,
        'kind': 'item_patch',
        'display_name': projectEntity['display_name'],
        'revision': projectEntity['revision'],
        'origin': projectEntity['origin'],
        'summary': <String, Object?>{
          'kind': 'item_patch',
          'data': <String, Object?>{
            'vanilla_class': data['vanilla_class'],
            'field_count': fields.length,
            'field_types': fieldTypes,
            'fields': fields,
          },
        },
        'references': <Object?>[],
        'asset_references': <Object?>[],
      },
    ],
    'assets': <Object?>[],
  });
}

final class _FakeItemPatchStore
    implements ManagedRevision3AuthoringStore, ManagedRevision3ItemPatchStore {
  final Map<String, String> _projectsByHead = <String, String>{};
  var _sequence = 0;
  var catalogSealDigit = 'c';
  var catalogSourceDigit = 'd';
  var wrongCatalogTarget = false;
  var catalogReadCalls = 0;
  var prepareItemPatchCalls = 0;
  Object? nextCatalogError;
  ModFfiException? nextPrepareError;
  final List<AuthoringRevision3ItemPatchRequestV1> itemPatchRequests =
      <AuthoringRevision3ItemPatchRequestV1>[];

  AuthoringWorkingHead _register(String projectJson) {
    _sequence++;
    final head = AuthoringWorkingHead.fromCanonicalJson(
      jsonEncode(<String, Object?>{
        'store_format': 1,
        'snapshot': <String, Object?>{
          'byte_len': utf8.encode(projectJson).length,
          'sha256': _digitSeal(_lastHexDigit(_sequence)),
        },
      }),
    );
    _projectsByHead[head.canonicalJson] = projectJson;
    return head;
  }

  @override
  Future<AuthoringRevision3StoreOpenedResult> open({
    required String root,
    required AuthoringAssetVerification verification,
  }) async {
    final rawHead = await File(
      p.join(root, 'gore-project.json'),
    ).readAsString();
    final projectJson = _projectsByHead[rawHead];
    if (projectJson == null) throw StateError('unknown fake published head');
    return AuthoringRevision3StoreOpenedResult.fromJson(<String, Object?>{
      'ok': true,
      'head_json': rawHead,
      'project_json': projectJson,
    });
  }

  @override
  Future<AuthoringRevision3StoreOpenedResult> openHeadBytes({
    required String root,
    required AuthoringWorkingHead head,
    required AuthoringAssetVerification verification,
  }) async {
    final projectJson = _projectsByHead[head.canonicalJson];
    if (projectJson == null) throw StateError('unknown fake candidate head');
    return AuthoringRevision3StoreOpenedResult.fromJson(<String, Object?>{
      'ok': true,
      'head_json': head.canonicalJson,
      'project_json': projectJson,
    });
  }

  @override
  Future<AuthoringRevision3CheckpointPreparation> prepareCheckpoint({
    required String root,
    required AuthoringWorkingHead? expectedHead,
    required String projectJson,
  }) async {
    final headFile = File(p.join(root, 'gore-project.json'));
    final actual = await headFile.exists()
        ? await headFile.readAsString()
        : null;
    if (actual != expectedHead?.canonicalJson) {
      throw const ModFfiException(
        command: 'authoring_store_prepare_revision3_checkpoint',
        code: 'AUTHORING_STORE_HEAD_CONFLICT',
        message: 'fake checkpoint conflict',
      );
    }
    final head = _register(projectJson);
    return AuthoringRevision3CheckpointPreparation.fromJson(<String, Object?>{
      'ok': true,
      'head_json': head.canonicalJson,
    });
  }

  @override
  Future<AuthoringRevision3ItemCatalogReadResult> readItemCatalogV1({
    required String root,
    required AuthoringWorkingHead expectedHead,
  }) async {
    catalogReadCalls++;
    final actual = await File(p.join(root, 'gore-project.json')).readAsString();
    if (actual != expectedHead.canonicalJson) {
      throw const ModFfiException(
        command: 'authoring_store_read_revision3_item_catalog_v1',
        code: 'AUTHORING_REVISION3_ITEM_PATCH_HEAD_CONFLICT',
        message: 'fake Item catalog conflict',
      );
    }
    final injected = nextCatalogError;
    nextCatalogError = null;
    if (injected != null) throw injected;
    final projectJson = _projectsByHead[actual]!;
    return _catalogResult(
      head: expectedHead,
      projectJson: projectJson,
      catalogSealDigit: catalogSealDigit,
      catalogSourceDigit: catalogSourceDigit,
      wrongTarget: wrongCatalogTarget,
    );
  }

  @override
  Future<AuthoringRevision3ItemPatchPreparation> prepareItemPatchV1({
    required String root,
    required String currentProjectJson,
    required AuthoringRevision3ItemPatchRequestV1 request,
  }) async {
    prepareItemPatchCalls++;
    itemPatchRequests.add(request);
    final injected = nextPrepareError;
    nextPrepareError = null;
    if (injected != null) throw injected;
    final actual = await File(p.join(root, 'gore-project.json')).readAsString();
    if (actual != request.expectedHead.canonicalJson ||
        _projectsByHead[actual] != currentProjectJson) {
      throw const ModFfiException(
        command: 'authoring_store_prepare_revision3_item_patch_v1',
        code: 'AUTHORING_REVISION3_ITEM_PATCH_HEAD_CONFLICT',
        message: 'fake ItemPatch conflict',
      );
    }
    final requestJson = (jsonDecode(request.canonicalJson) as Map)
        .cast<String, Object?>();
    final mutation = (requestJson['mutation']! as Map).cast<String, Object?>();
    final basis = (jsonDecode(currentProjectJson) as Map)
        .cast<String, Object?>();
    final candidate = Map<String, Object?>.from(basis);
    candidate['revision'] = request.expectedRevision + 1;
    final entities = SplayTreeMap<String, Object?>.from(
      (basis['entities']! as Map).cast<String, Object?>(),
    );
    final change = switch (request.action) {
      AuthoringRevision3ItemPatchAction.upsert => 'created',
      AuthoringRevision3ItemPatchAction.remove => 'removed',
    };
    final entityRevision = switch (request.action) {
      AuthoringRevision3ItemPatchAction.upsert => 0,
      AuthoringRevision3ItemPatchAction.remove => null,
    };
    switch (request.action) {
      case AuthoringRevision3ItemPatchAction.upsert:
        entities[request.entityId] = _itemEntity(
          entityId: request.entityId,
          displayName: request.displayName!,
          revision: 0,
          target: (basis['target']! as Map).cast<String, Object?>(),
          fields: (mutation['fields']! as Map).cast<String, Object?>(),
          sourceSeal: (mutation['source_seal']! as Map).cast<String, Object?>(),
        );
      case AuthoringRevision3ItemPatchAction.remove:
        entities.remove(request.entityId);
    }
    candidate['entities'] = entities;
    final candidateJson = jsonEncode(candidate);
    final candidateHead = _register(candidateJson);
    return AuthoringRevision3ItemPatchPreparation.fromJson(
      <String, Object?>{
        'ok': true,
        'outcome': 'prepared_unpublished',
        'basis_head_json': request.expectedHead.canonicalJson,
        'head_json': candidateHead.canonicalJson,
        'project_json': candidateJson,
        'project_id': request.expectedProjectId,
        'revision': request.expectedRevision + 1,
        'entity_id': request.entityId,
        'entity_revision': entityRevision,
        'change': change,
        'catalog_layer': request.catalogLayer,
        'vanilla_class': request.vanillaClass,
        'source_seal': <String, Object?>{
          'byte_len': request.sourceSeal.byteLength,
          'sha256': request.sourceSeal.sha256,
        },
        'catalog_seal': _seal(catalogSealDigit, 9000),
        'build_status': 'blocked',
        'runtime_status': 'runtime_unqualified',
        'publication_status': 'not_supported',
      },
      currentProjectJson: currentProjectJson,
      request: request,
    );
  }

  @override
  dynamic noSuchMethod(Invocation invocation) => throw UnsupportedError(
    'unexpected fake Item Store call: ${invocation.memberName}',
  );
}

final class _CheckpointOnlyItemStore implements ManagedRevision3AuthoringStore {
  const _CheckpointOnlyItemStore(this.delegate);

  final _FakeItemPatchStore delegate;

  @override
  Future<AuthoringRevision3StoreOpenedResult> open({
    required String root,
    required AuthoringAssetVerification verification,
  }) => delegate.open(root: root, verification: verification);

  @override
  Future<AuthoringRevision3StoreOpenedResult> openHeadBytes({
    required String root,
    required AuthoringWorkingHead head,
    required AuthoringAssetVerification verification,
  }) => delegate.openHeadBytes(
    root: root,
    head: head,
    verification: verification,
  );

  @override
  Future<AuthoringRevision3CheckpointPreparation> prepareCheckpoint({
    required String root,
    required AuthoringWorkingHead? expectedHead,
    required String projectJson,
  }) => delegate.prepareCheckpoint(
    root: root,
    expectedHead: expectedHead,
    projectJson: projectJson,
  );

  @override
  dynamic noSuchMethod(Invocation invocation) => throw UnsupportedError(
    'unexpected checkpoint-only Item Store call: ${invocation.memberName}',
  );
}

final class _FakeItemPatchLease
    implements
        ManagedRevision3CurrentProjectLease,
        ManagedRevision3ItemPatchLease {
  _FakeItemPatchLease({
    required this.root,
    required String projectJson,
    required this.head,
    required this.catalog,
    this.mismatchReceipt = false,
    this.supportsItemPatchValue = true,
    this.nextCatalogError,
  }) : canonicalProjectJson = projectJson,
       projectRevision = ((jsonDecode(projectJson) as Map)['revision']! as int);

  @override
  final Directory root;
  @override
  final String projectId = _projectId;
  @override
  int projectRevision;
  @override
  AuthoringWorkingHead head;
  @override
  String canonicalProjectJson;
  final AuthoringRevision3ItemCatalogReadResult catalog;
  final bool mismatchReceipt;
  final bool supportsItemPatchValue;
  Object? nextCatalogError;
  var requiresReopenValue = false;
  var catalogReadCalls = 0;
  var publishCalls = 0;
  var itemUncertaintyLatchCalls = 0;
  var closeCalls = 0;

  @override
  bool get requiresReopen => requiresReopenValue;

  @override
  bool get supportsItemPatch => supportsItemPatchValue;

  @override
  Future<AuthoringRevision3ItemCatalogReadResult> readItemCatalogV1() async {
    catalogReadCalls++;
    final error = nextCatalogError;
    nextCatalogError = null;
    if (error != null) throw error;
    return catalog;
  }

  @override
  Future<Revision3ItemPatchPublication> prepareAndPublishItemPatchV1({
    required Revision3ItemPatchTechnicalPlan plan,
  }) async {
    publishCalls++;
    projectRevision++;
    head = _head(projectRevision);
    return Revision3ItemPatchPublication(
      projectId: projectId,
      projectRevision: projectRevision,
      entityId: mismatchReceipt ? _otherEntityId : plan.entityId,
      entityRevision: 0,
      change: AuthoringRevision3ItemPatchChange.created,
      vanillaClass: plan.vanillaClass,
    );
  }

  @override
  void markRequiresReopenAfterItemPatchUncertainty() {
    itemUncertaintyLatchCalls++;
    requiresReopenValue = true;
  }

  @override
  Future<void> close() async {
    closeCalls++;
  }

  @override
  dynamic noSuchMethod(Invocation invocation) => throw UnsupportedError(
    'unexpected fake Item lease call: ${invocation.memberName}',
  );
}

AuthoringRevision3ItemCatalogReadResult _catalogResult({
  required AuthoringWorkingHead head,
  required String projectJson,
  String catalogSealDigit = 'c',
  String catalogSourceDigit = 'd',
  bool wrongTarget = false,
  int? responseProjectRevision,
}) {
  final project = (jsonDecode(projectJson) as Map).cast<String, Object?>();
  final target = wrongTarget
      ? <String, Object?>{
          'executable': <String, Object?>{
            'byte_len': _targetBytes,
            'sha256': _digitSeal('b'),
          },
        }
      : project['target'];
  final catalogJson = jsonEncode(<String, Object?>{
    'catalog_layer': _catalogLayer,
    'catalog_seal': _seal(catalogSealDigit, 9000),
    'entries': <Object?>[
      <String, Object?>{
        'category': 'food',
        'fields': <Object?>[
          revision3ItemNumericField(name: 'm_Value', scalarType: 'integer'),
          revision3ItemNumericField(
            name: 'm_Weight',
            scalarType: 'float',
            defaultValue: <String, Object?>{'type': 'float', 'data': 0.25},
          ),
        ],
        'runtime_path': '/Script/Angelscript.$_vanillaClass',
        'source_seal': _seal(catalogSourceDigit, 500),
        'vanilla_class': _vanillaClass,
      },
    ],
    'schema_revision': 1,
    'target': target,
  });
  return AuthoringRevision3ItemCatalogReadResult.fromJson(<String, Object?>{
    'ok': true,
    'head_json': head.canonicalJson,
    'project_id': project['project_id'],
    'project_revision': responseProjectRevision ?? project['revision'],
    'catalog_json': catalogJson,
    'catalog_seal': _seal(catalogSealDigit, 9000),
    'catalog_authority': 'native_embedded_schema_exact_current_project',
    'build_status': 'not_evaluated',
    'runtime_status': 'runtime_unqualified',
    'publication_status': 'not_applicable',
  }, expectedHead: head);
}

Map<String, Object?> _itemEntity({
  required String entityId,
  required String displayName,
  required int revision,
  required Map<String, Object?> target,
  required Map<String, Object?> fields,
  required Map<String, Object?> sourceSeal,
  String catalogLayer = _catalogLayer,
}) => <String, Object?>{
  'id': entityId,
  'display_name': displayName,
  'origin': <String, Object?>{
    'type': 'vanilla',
    'generation': target,
    'catalog_layer': catalogLayer,
    'canonical_selector': _vanillaClass,
    'source_seal': sourceSeal,
  },
  'revision': revision,
  'payload': <String, Object?>{
    'kind': 'item_patch',
    'data': <String, Object?>{
      'vanilla_class': _vanillaClass,
      'fields': SplayTreeMap<String, Object?>.from(fields),
    },
  },
};

String _projectJson({
  required int revision,
  bool patched = false,
  String itemCatalogLayer = _catalogLayer,
  String itemSourceDigit = 'd',
}) {
  final target = <String, Object?>{
    'executable': <String, Object?>{
      'byte_len': _targetBytes,
      'sha256': _targetSha,
    },
  };
  return jsonEncode(<String, Object?>{
    'format': 2,
    'schema_revision': 3,
    'project_id': _projectId,
    'revision': revision,
    'meta': <String, Object?>{
      'name': 'Managed items',
      'version': '1.0.0',
      'author': 'tests',
    },
    'target': target,
    'authoring_locales': <Object?>[],
    'entities': <String, Object?>{
      if (patched)
        _patchedEntityId: _itemEntity(
          entityId: _patchedEntityId,
          displayName: 'Apple',
          revision: 2,
          target: target,
          fields: <String, Object?>{
            'm_Value': <String, Object?>{'type': 'integer', 'data': 4},
          },
          sourceSeal: _seal(itemSourceDigit, 500),
          catalogLayer: itemCatalogLayer,
        ),
    },
    'asset_store': <String, Object?>{'assets': <String, Object?>{}},
  });
}

AuthoringWorkingHead _head(int revision) =>
    AuthoringWorkingHead.fromCanonicalJson(
      jsonEncode(<String, Object?>{
        'store_format': 1,
        'snapshot': <String, Object?>{
          'byte_len': 1000 + revision,
          'sha256': _digitSeal(_lastHexDigit(revision)),
        },
      }),
    );

Map<String, Object?> _seal(String digit, int byteLength) => <String, Object?>{
  'byte_len': byteLength,
  'sha256': _digitSeal(digit),
};

String _digitSeal(String digit) => List<String>.filled(64, digit).join();

String _lastHexDigit(int value) {
  final encoded = value.toRadixString(16);
  return encoded.substring(encoded.length - 1);
}
