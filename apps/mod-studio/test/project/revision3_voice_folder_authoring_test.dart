import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/project/revision3_voice_folder_authoring.dart';

void main() {
  test('plan derives exact Ogg, blocked, and ignored scan counts', () {
    final plan = _blockedPlan();

    expect(plan.locale, 'de');
    expect(plan.counts.scanned, 7);
    expect(plan.counts.ogg, 5);
    expect(plan.counts.ready, 1);
    expect(plan.counts.alreadyPresent, 1);
    expect(plan.counts.unmatched, 1);
    expect(plan.counts.ambiguous, 1);
    expect(plan.counts.invalid, 1);
    expect(plan.counts.blocked, 3);
    expect(plan.counts.ignored, 2);
    expect(plan.hasBlockingRows, isTrue);
    expect(plan.canApply, isFalse);
    expect(plan.readyRows, hasLength(1));
    expect(plan.readyRows.single.beforeTakeCount, 2);
    expect(plan.readyRows.single.afterTakeCount, 3);
    expect(plan.readyRows.single.selectionUnchanged, isTrue);
    expect(plan.readyRows.single.targetUnchanged, isTrue);
    expect(plan.readyRows.single.importedTakeWillBeSelected, isFalse);
    expect(plan.readyRows.single.changesDialogText, isFalse);
    expect(plan.changesSelection, isFalse);
    expect(plan.changesDialogText, isFalse);
    expect(() => plan.rows.add(_row(ordinal: 5)), throwsUnsupportedError);
  });

  test('plan rejects unstable, unbounded, or presentation-unsafe rows', () {
    expect(() => _plan(rows: [_row(ordinal: 1)]), throwsFormatException);
    expect(
      () => _plan(
        rows: [
          _row(ordinal: 0),
          _row(ordinal: 1, rowToken: 'row-0'),
        ],
      ),
      throwsFormatException,
    );
    expect(() => _row(ordinal: 256), throwsFormatException);
    expect(
      () => _plan(
        rows: [_row(ordinal: 0)],
        scannedEntryCount: 1,
        ignoredEntryCount: 1,
      ),
      throwsFormatException,
    );
    expect(
      () => _plan(
        rows: [_row(ordinal: 0)],
        scannedEntryCount: 3,
        ignoredEntryCount: 1,
      ),
      throwsFormatException,
    );
    expect(
      () => Revision3VoiceFolderReviewRow(
        ordinal: 0,
        rowToken: 'row',
        status: Revision3VoiceFolderRowStatus.ready,
        codec: Revision3VoiceFolderCodec.vorbis,
        byteLength: 1024,
        lineLabel: r'C:\secret\asghan.ogg',
        speakerLabel: 'Asghan',
        takeDisplayName: 'Asghan take',
        beforeTakeCount: 0,
        afterTakeCount: 1,
        targetState: Revision3VoiceFolderTargetState.resolved,
      ),
      throwsFormatException,
    );
  });

  test('service rejects a plan from any other project authority', () async {
    final service = Revision3VoiceFolderAuthoringService(
      planFolder: (_) async => _cleanPlan(projectRevision: 8),
      applyPlan: _unexpectedApply,
    );

    await expectLater(
      service.plan(_request()),
      throwsA(isA<Revision3VoiceFolderRequiresReopenException>()),
    );
  });

  test('service performs one exact all-or-none apply', () async {
    final plan = _cleanPlan();
    var applyCalls = 0;
    Revision3VoiceFolderImportPlan? capturedPlan;
    final service = Revision3VoiceFolderAuthoringService(
      planFolder: (_) async => plan,
      applyPlan: ({required plan}) async {
        applyCalls++;
        capturedPlan = plan;
        return _publication(plan);
      },
    );

    final reviewed = await service.plan(_request());
    final publication = await service.apply(plan: reviewed);

    expect(reviewed.canApply, isTrue);
    expect(reviewed.counts.alreadyPresent, 1);
    expect(applyCalls, 1);
    expect(capturedPlan, same(reviewed));
    expect(publication.projectRevision, reviewed.projectRevision + 1);
    expect(publication.importedCount, reviewed.counts.ready);
    expect(publication.planToken, reviewed.planToken);
  });

  test('service never calls apply for a partially blocked plan', () async {
    final plan = _blockedPlan();
    var applyCalls = 0;
    final service = Revision3VoiceFolderAuthoringService(
      planFolder: (_) async => plan,
      applyPlan: ({required plan}) async {
        applyCalls++;
        return _publication(plan);
      },
    );

    await expectLater(service.apply(plan: plan), throwsFormatException);
    expect(applyCalls, 0);
  });

  test('service fails closed on a malformed publication', () async {
    final plan = _cleanPlan(rows: [_row(ordinal: 0)]);
    final service = Revision3VoiceFolderAuthoringService(
      planFolder: (_) async => plan,
      applyPlan: ({required plan}) async =>
          Revision3VoiceFolderImportPublication(
            projectId: plan.projectId,
            projectRevision: plan.projectRevision + 2,
            projectHead: 'changed-head',
            checkpointToken: 'changed-checkpoint',
            planToken: plan.planToken,
            importedCount: plan.counts.ready,
          ),
    );

    await expectLater(
      service.apply(plan: plan),
      throwsA(isA<Revision3VoiceFolderRequiresReopenException>()),
    );
  });
}

Revision3VoiceFolderPlanRequest _request() => Revision3VoiceFolderPlanRequest(
  folderPath: r'C:\recordings\Voice Batch',
  locale: 'de',
  expectedProjectId: 'project',
  expectedProjectRevision: 7,
  expectedProjectHead: 'head-7',
  expectedCheckpointToken: 'checkpoint-7',
);

Revision3VoiceFolderImportPlan _cleanPlan({
  int projectRevision = 7,
  List<Revision3VoiceFolderReviewRow>? rows,
}) => _plan(
  projectRevision: projectRevision,
  rows:
      rows ??
      [
        _row(ordinal: 0),
        _row(ordinal: 1, status: Revision3VoiceFolderRowStatus.alreadyPresent),
      ],
);

Revision3VoiceFolderImportPlan _blockedPlan() => _plan(
  scannedEntryCount: 7,
  ignoredEntryCount: 2,
  rows: [
    _row(ordinal: 0),
    _row(ordinal: 1, status: Revision3VoiceFolderRowStatus.alreadyPresent),
    _row(ordinal: 2, status: Revision3VoiceFolderRowStatus.unmatched),
    _row(ordinal: 3, status: Revision3VoiceFolderRowStatus.ambiguous),
    _row(ordinal: 4, status: Revision3VoiceFolderRowStatus.invalid),
  ],
);

Revision3VoiceFolderImportPlan _plan({
  int projectRevision = 7,
  required List<Revision3VoiceFolderReviewRow> rows,
  int? scannedEntryCount,
  int ignoredEntryCount = 0,
}) => Revision3VoiceFolderImportPlan(
  projectId: 'project',
  projectRevision: projectRevision,
  projectHead: 'head-7',
  checkpointToken: 'checkpoint-7',
  planToken: 'plan-token',
  folderLabel: 'Voice Batch',
  locale: 'de',
  scannedEntryCount: scannedEntryCount ?? rows.length + ignoredEntryCount,
  ignoredEntryCount: ignoredEntryCount,
  rows: rows,
);

Revision3VoiceFolderReviewRow _row({
  required int ordinal,
  String? rowToken,
  Revision3VoiceFolderRowStatus status = Revision3VoiceFolderRowStatus.ready,
}) {
  final mapped =
      status == Revision3VoiceFolderRowStatus.ready ||
      status == Revision3VoiceFolderRowStatus.alreadyPresent;
  final ready = status == Revision3VoiceFolderRowStatus.ready;
  final alreadyPresent = status == Revision3VoiceFolderRowStatus.alreadyPresent;
  return Revision3VoiceFolderReviewRow(
    ordinal: ordinal,
    rowToken: rowToken ?? 'row-$ordinal',
    status: status,
    codec: status == Revision3VoiceFolderRowStatus.invalid
        ? null
        : Revision3VoiceFolderCodec.vorbis,
    byteLength: status == Revision3VoiceFolderRowStatus.invalid
        ? null
        : 4096 + ordinal,
    lineLabel: mapped ? 'Asghan — Mine entrance' : null,
    speakerLabel: mapped ? 'Asghan' : null,
    takeDisplayName: ready ? 'Asghan folder take' : null,
    beforeTakeCount: mapped ? 2 : null,
    afterTakeCount: ready ? 3 : (alreadyPresent ? 2 : null),
    targetState: mapped ? Revision3VoiceFolderTargetState.resolved : null,
  );
}

Revision3VoiceFolderImportPublication _publication(
  Revision3VoiceFolderImportPlan plan,
) => Revision3VoiceFolderImportPublication(
  projectId: plan.projectId,
  projectRevision: plan.projectRevision + 1,
  projectHead: 'head-8',
  checkpointToken: 'checkpoint-8',
  planToken: plan.planToken,
  importedCount: plan.counts.ready,
);

Future<Revision3VoiceFolderImportPublication> _unexpectedApply({
  required Revision3VoiceFolderImportPlan plan,
}) => throw StateError('apply was not expected');
