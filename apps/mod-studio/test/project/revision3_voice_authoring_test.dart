import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/core/mod_ffi.dart';
import 'package:gore_mod/project/revision3_content_index.dart';
import 'package:gore_mod/project/revision3_voice_authoring.dart';

import '../support/revision3_voice_content_fixture.dart';

void main() {
  test(
    'catalog exposes friendly lines and plan reuses the exact locale slot',
    () {
      final catalog = Revision3VoiceCatalog.fromContentIndex(
        revision3VoiceContentIndexFixture(),
      );
      expect(catalog.projectRevision, 7);
      expect(catalog.lines, hasLength(1));
      expect(
        catalog.lines.single.displayLabel,
        'Asghan — Mine entrance question',
      );
      expect(
        catalog.lines.single.displayLabel,
        isNot(
          anyOf(
            contains(revision3VoiceContentLineId),
            contains('GRD_263_ASGHAN_OPEN_INFO_06_02'),
          ),
        ),
      );
      expect(catalog.suggestedLocales, <String>['de', 'en']);
      expect(
        catalog.lines.single.slotSummaryForLocale('de')?.candidateCount,
        0,
      );
      expect(
        catalog.lines.single.slotSummaryForLocale('de')?.hasSelectedTake,
        isFalse,
      );

      final input = Revision3VoiceTakeAuthoringInput(
        lineId: revision3VoiceContentLineId,
        locale: 'de',
        sourcePath: r'C:\Recordings\Asghan Take 01.OGG',
        takeDisplayName: 'Asghan German take 1',
        status: AuthoringRevision3VoiceTakeStatus.recorded,
      );
      final plan = Revision3VoiceTakeTechnicalPlan.forCheckpoint(
        catalog: catalog,
        input: input,
      );
      expect(plan.slotId, revision3VoiceContentSlotId);
      expect(plan.expectsSlotCreated, isFalse);
      expect(plan.takeId, isNot(anyOf(catalog.entityIds)));
      expect(plan.logicalName, 'Asghan Take 01.OGG');
      expect(
        plan.text,
        isNull,
        reason: 'normal Voice import preserves localization',
      );

      final explicitEdit = Revision3VoiceTakeAuthoringInput(
        lineId: revision3VoiceContentLineId,
        locale: 'de',
        sourcePath: r'C:\Recordings\asghan.ogg',
        takeDisplayName: 'Asghan corrected line',
        status: AuthoringRevision3VoiceTakeStatus.approved,
        selectTake: true,
        dialogText: '  Du willst in die Mine?  ',
      );
      expect(explicitEdit.dialogText, 'Du willst in die Mine?');
    },
  );

  test('catalog and target plan share the exact archive stem contract', () {
    Map<String, Object?> withLocId(String locId) {
      final json = revision3VoiceContentIndexJsonFixture();
      final localization = _entity(json, revision3VoiceContentLocalizationId);
      final summary = (localization['summary']! as Map).cast<String, Object?>();
      final data = (summary['data']! as Map).cast<String, Object?>();
      data['loc_id'] = locId;
      return json;
    }

    final boundary = Revision3VoiceCatalog.fromContentIndex(
      Revision3ContentIndex.fromJsonObject(withLocId('A' * 1020)),
    );
    final plan = Revision3VoiceTargetTechnicalPlan.forCheckpoint(
      catalog: boundary,
      lineId: revision3VoiceContentLineId,
      locale: 'de',
    );
    expect(plan.locId, hasLength(1020));

    for (final invalid in <String>[
      'A' * 1021,
      'CON',
      r'CONOUT$',
      'LPT9.txt',
      'trailing.',
      'bad:name',
      'nön-ascii',
    ]) {
      expect(
        () => Revision3VoiceCatalog.fromContentIndex(
          Revision3ContentIndex.fromJsonObject(withLocId(invalid)),
        ),
        throwsFormatException,
        reason: invalid,
      );
    }
  });

  test(
    'duplicate visible lines are searchable and disambiguated without IDs',
    () {
      final catalog = Revision3VoiceCatalog.fromContentIndex(
        revision3VoiceContentIndexFixture(duplicateLine: true),
      );

      expect(catalog.lines, hasLength(2));
      expect(catalog.lines[0].displayLabel, endsWith('· 1 of 2'));
      expect(catalog.lines[1].displayLabel, endsWith('· 2 of 2'));
      for (final line in catalog.lines) {
        expect(
          line.displayLabel,
          isNot(contains('GRD_263_ASGHAN_OPEN_INFO_06_02')),
        );
        expect(line.displayLabel, isNot(contains(line.lineId)));
        expect(line.matches('asghan'), isTrue);
        expect(line.matches('grd_263_asghan'), isTrue);
        expect(line.matches(line.lineId), isFalse);
      }
    },
  );

  test('catalog retains structured VoiceSlot counts and selected state', () {
    for (final testCase in const <(int, bool)>[
      (0, false),
      (1, false),
      (7, true),
    ]) {
      final catalog = Revision3VoiceCatalog.fromContentIndex(
        revision3VoiceContentIndexFixture(
          existingSlotCandidateCount: testCase.$1,
          existingSlotHasSelectedTake: testCase.$2,
        ),
      );
      final summary = catalog.lines.single.slotSummaryForLocale('de')!;
      expect(summary.candidateCount, testCase.$1);
      expect(summary.hasSelectedTake, testCase.$2);
      expect(
        summary.targetResolution,
        Revision3ContentVoiceTargetResolution.unresolved,
      );
    }
  });

  test(
    'content parser binds VoiceSlot counts and selection to exact references',
    () {
      final candidateMismatch = revision3VoiceContentIndexJsonFixture(
        existingSlotCandidateCount: 1,
      );
      _slotData(candidateMismatch)['candidate_count'] = 0;
      expect(
        () => Revision3ContentIndex.fromJsonObject(candidateMismatch),
        throwsFormatException,
      );

      final unresolvedMismatch = revision3VoiceContentIndexJsonFixture(
        existingSlotCandidateCount: 1,
      );
      final unresolvedCandidate = _slotReferences(unresolvedMismatch).single;
      (unresolvedCandidate['target']! as Map<String, Object?>)['entity_id'] =
          '99999999999999999999999999999999';
      unresolvedCandidate['resolution'] = 'missing_entity';
      _slotData(unresolvedMismatch)['candidate_count'] = 0;
      expect(
        () => Revision3ContentIndex.fromJsonObject(unresolvedMismatch),
        throwsFormatException,
        reason: 'unresolved references still contribute to the exact count',
      );

      final selectedMismatch = revision3VoiceContentIndexJsonFixture(
        existingSlotCandidateCount: 1,
        existingSlotHasSelectedTake: true,
      );
      _slotData(selectedMismatch)['has_selected_take'] = false;
      expect(
        () => Revision3ContentIndex.fromJsonObject(selectedMismatch),
        throwsFormatException,
      );

      final relabeledSelection = revision3VoiceContentIndexJsonFixture(
        existingSlotCandidateCount: 1,
        existingSlotHasSelectedTake: true,
      );
      _slotReferences(relabeledSelection).singleWhere(
        (reference) => reference['role'] == 'voice_selected',
      )['role'] = 'dialog_voice_slot';
      _slotData(relabeledSelection)['has_selected_take'] = false;
      expect(
        () => Revision3ContentIndex.fromJsonObject(relabeledSelection),
        throwsFormatException,
        reason: 'a foreign role cannot hide the real selected take',
      );

      final duplicateCandidate = revision3VoiceContentIndexJsonFixture(
        existingSlotCandidateCount: 1,
      );
      final duplicateReferences = _slotReferences(duplicateCandidate);
      duplicateReferences.add(
        Map<String, Object?>.from(duplicateReferences.single),
      );
      _slotData(duplicateCandidate)['candidate_count'] = 2;
      expect(
        () => Revision3ContentIndex.fromJsonObject(duplicateCandidate),
        throwsFormatException,
      );

      final duplicateSelected = revision3VoiceContentIndexJsonFixture(
        existingSlotCandidateCount: 1,
        existingSlotHasSelectedTake: true,
      );
      final selectedReferences = _slotReferences(duplicateSelected);
      selectedReferences.add(
        Map<String, Object?>.from(
          selectedReferences.singleWhere(
            (reference) => reference['role'] == 'voice_selected',
          ),
        ),
      );
      expect(
        () => Revision3ContentIndex.fromJsonObject(duplicateSelected),
        throwsFormatException,
      );

      final selectedOutsideCandidates = revision3VoiceContentIndexJsonFixture(
        existingSlotCandidateCount: 2,
        existingSlotHasSelectedTake: true,
      );
      final outsideReferences = _slotReferences(selectedOutsideCandidates);
      final candidates = outsideReferences
          .where((reference) => reference['role'] == 'voice_candidate')
          .toList(growable: false);
      final outsideTarget = Map<String, Object?>.from(
        candidates.last['target']! as Map<String, Object?>,
      );
      outsideReferences.remove(candidates.last);
      _slotData(selectedOutsideCandidates)['candidate_count'] = 1;
      final selected = outsideReferences.singleWhere(
        (reference) => reference['role'] == 'voice_selected',
      );
      selected['target'] = outsideTarget;
      expect(
        () => Revision3ContentIndex.fromJsonObject(selectedOutsideCandidates),
        throwsFormatException,
      );

      final missingCount = revision3VoiceContentIndexJsonFixture(
        omitExistingSlotCandidateCount: true,
      );
      expect(
        () => Revision3ContentIndex.fromJsonObject(missingCount),
        throwsFormatException,
      );
    },
  );

  test('content parser cross-binds DialogLine slot locales and role shape', () {
    final summaryWithoutReference = revision3VoiceContentIndexJsonFixture();
    _lineReferences(
      summaryWithoutReference,
    ).removeWhere((reference) => reference['role'] == 'dialog_voice_slot');
    expect(
      () => Revision3ContentIndex.fromJsonObject(summaryWithoutReference),
      throwsFormatException,
      reason: 'a summary locale must never make a removed slot look new',
    );

    final referenceWithoutSummary = revision3VoiceContentIndexJsonFixture();
    _lineData(referenceWithoutSummary)['voice_slot_locales'] = <Object?>[];
    expect(
      () => Revision3ContentIndex.fromJsonObject(referenceWithoutSummary),
      throwsFormatException,
    );

    final duplicateReference = revision3VoiceContentIndexJsonFixture();
    final duplicateLineReferences = _lineReferences(duplicateReference);
    duplicateLineReferences.add(
      Map<String, Object?>.from(
        duplicateLineReferences.singleWhere(
          (reference) => reference['role'] == 'dialog_voice_slot',
        ),
      ),
    );
    expect(
      () => Revision3ContentIndex.fromJsonObject(duplicateReference),
      throwsFormatException,
    );

    final malformedQualifier = revision3VoiceContentIndexJsonFixture();
    _lineReferences(malformedQualifier).singleWhere(
      (reference) => reference['role'] == 'dialog_voice_slot',
    )['qualifier'] = null;
    expect(
      () => Revision3ContentIndex.fromJsonObject(malformedQualifier),
      throwsFormatException,
    );

    final malformedKind = revision3VoiceContentIndexJsonFixture();
    final malformedKindReference = _lineReferences(
      malformedKind,
    ).singleWhere((reference) => reference['role'] == 'dialog_voice_slot');
    (malformedKindReference['target']!
            as Map<String, Object?>)['expected_kind'] =
        'dialog_line';
    malformedKindReference['resolution'] = 'kind_mismatch';
    expect(
      () => Revision3ContentIndex.fromJsonObject(malformedKind),
      throwsFormatException,
    );

    final foreignRole = revision3VoiceContentIndexJsonFixture();
    _lineReferences(foreignRole).add(<String, Object?>{
      'role': 'voice_candidate',
      'qualifier': null,
      'target': <String, Object?>{
        'project_id': revision3VoiceContentProjectId,
        'entity_id': revision3VoiceContentSlotId,
        'expected_kind': 'voice_slot',
      },
      'resolution': 'resolved',
    });
    expect(
      () => Revision3ContentIndex.fromJsonObject(foreignRole),
      throwsFormatException,
    );
  });

  test(
    'sealed target states remain extensible while malformed slot graphs stay blocked',
    () {
      for (final resolution in const ['ambiguous', 'resolved']) {
        final catalog = Revision3VoiceCatalog.fromContentIndex(
          revision3VoiceContentIndexFixture(
            existingSlotTargetResolution: resolution,
          ),
        );
        expect(catalog.lines.single.isLocaleAuthorable('de'), isTrue);
        expect(catalog.lines.single.isLocaleAuthorable('en'), isTrue);
        final plan = Revision3VoiceTakeTechnicalPlan.forCheckpoint(
          catalog: catalog,
          input: _input(locale: 'de'),
        );
        expect(plan.expectsSlotCreated, isFalse, reason: resolution);
        expect(
          catalog.lines.single
              .slotSummaryForLocale('de')!
              .targetResolution
              .name,
          resolution,
        );
      }

      final mismatchedLocale = revision3VoiceContentIndexJsonFixture(
        existingSlotCandidateCount: 1,
      );
      _takeData(mismatchedLocale)['locale'] = 'en';
      final mismatchedCatalog = Revision3VoiceCatalog.fromContentIndex(
        Revision3ContentIndex.fromJsonObject(mismatchedLocale),
      );
      expect(mismatchedCatalog.lines.single.isLocaleAuthorable('de'), isFalse);
      final cleanLocalePlan = Revision3VoiceTakeTechnicalPlan.forCheckpoint(
        catalog: mismatchedCatalog,
        input: _input(locale: 'en'),
      );
      expect(cleanLocalePlan.expectsSlotCreated, isTrue);

      final missingCandidate = revision3VoiceContentIndexJsonFixture(
        existingSlotCandidateCount: 1,
      );
      final missingReference = _slotReferences(missingCandidate).single;
      (missingReference['target']! as Map<String, Object?>)['entity_id'] =
          '99999999999999999999999999999999';
      missingReference['resolution'] = 'missing_entity';
      final missingCatalog = Revision3VoiceCatalog.fromContentIndex(
        Revision3ContentIndex.fromJsonObject(missingCandidate),
      );
      expect(missingCatalog.lines.single.isLocaleAuthorable('de'), isFalse);
    },
  );

  test(
    'selected non-approved take stays editable and build-blocked upstream',
    () {
      final json = revision3VoiceContentIndexJsonFixture(
        existingSlotCandidateCount: 1,
        existingSlotHasSelectedTake: true,
      );
      _takeData(json)['status'] = 'reviewed';
      final catalog = Revision3VoiceCatalog.fromContentIndex(
        Revision3ContentIndex.fromJsonObject(json),
      );
      expect(catalog.lines.single.isLocaleAuthorable('de'), isTrue);
      expect(catalog.lines.single.slotSummaryForLocale('de'), isNotNull);
      final plan = Revision3VoiceTakeTechnicalPlan.forCheckpoint(
        catalog: catalog,
        input: _input(locale: 'de'),
      );
      expect(plan.expectsSlotCreated, isFalse);
    },
  );

  test(
    'full intact slot blocks another take but remains target-resolvable',
    () {
      final catalog = Revision3VoiceCatalog.fromContentIndex(
        revision3VoiceContentIndexFixture(existingSlotCandidateCount: 1024),
      );
      final line = catalog.lines.single;

      expect(line.isLocaleAuthorable('de'), isFalse);
      expect(line.isLocaleTargetable('de'), isTrue);
      expect(
        () => Revision3VoiceTakeTechnicalPlan.forCheckpoint(
          catalog: catalog,
          input: _input(locale: 'de'),
        ),
        throwsFormatException,
      );

      final target = Revision3VoiceTargetTechnicalPlan.forCheckpoint(
        catalog: catalog,
        lineId: revision3VoiceContentLineId,
        locale: 'de',
      );
      expect(target.slotId, revision3VoiceContentSlotId);
      expect(target.locId, 'GRD_263_ASGHAN_OPEN_INFO_06_02');
    },
  );

  test('a VoiceSlot shared by another line or locale is never offered', () {
    final json = revision3VoiceContentIndexJsonFixture(duplicateLine: true);
    final duplicate = _entity(json, revision3VoiceContentDuplicateLineId);
    ((duplicate['summary']! as Map<String, Object?>)['data']!
        as Map<String, Object?>)['voice_slot_locales'] = <Object?>[
      'de',
    ];
    (duplicate['references']! as List<Object?>).add(<String, Object?>{
      'role': 'dialog_voice_slot',
      'qualifier': 'de',
      'target': <String, Object?>{
        'project_id': revision3VoiceContentProjectId,
        'entity_id': revision3VoiceContentSlotId,
        'expected_kind': 'voice_slot',
      },
      'resolution': 'resolved',
    });
    final catalog = Revision3VoiceCatalog.fromContentIndex(
      Revision3ContentIndex.fromJsonObject(json),
    );

    expect(catalog.lines, hasLength(2));
    for (final line in catalog.lines) {
      expect(line.isLocaleAuthorable('de'), isFalse);
      expect(line.slotIdForLocale('de'), isNull);
      expect(line.isLocaleAuthorable('en'), isTrue);
    }
  });

  test(
    'structurally incomplete slot facts fail before catalog construction',
    () {
      expect(
        () => revision3VoiceContentIndexFixture(
          omitExistingSlotCandidateCount: true,
        ),
        throwsFormatException,
      );
    },
  );

  test('UI and authoring share one canonical locale rule', () {
    expect(revision3VoiceLocaleIsCanonical('de'), isTrue);
    expect(revision3VoiceLocaleIsCanonical('en-US'), isTrue);
    expect(revision3VoiceLocaleIsCanonical('en-us'), isFalse);
    expect(revision3VoiceLocaleIsCanonical('EN'), isFalse);
  });

  test(
    'logical Ogg leaf is derived internally and rejects unsafe Windows names',
    () {
      for (final path in <String>[
        r'C:\Voice\CON.ogg',
        r'C:\Voice\Lpt1.OGG',
        r'C:\Voice\x?.ogg',
        r'C:\Voice\ x.ogg',
        r'C:x.ogg',
        r'C:\Voice\.ogg',
      ]) {
        expect(
          () => Revision3VoiceTakeAuthoringInput(
            lineId: revision3VoiceContentLineId,
            locale: 'de',
            sourcePath: path,
            takeDisplayName: 'Take',
            status: AuthoringRevision3VoiceTakeStatus.recorded,
          ),
          throwsFormatException,
          reason: path,
        );
      }
      final safe = Revision3VoiceTakeAuthoringInput(
        lineId: revision3VoiceContentLineId,
        locale: 'de',
        sourcePath: '/recordings/Asghan final.ogg',
        takeDisplayName: 'Take',
        status: AuthoringRevision3VoiceTakeStatus.recorded,
      );
      expect(safe.logicalName, 'Asghan final.ogg');
    },
  );

  test(
    'new slot and take identities probe the exact entity set for collisions',
    () {
      final baseCatalog = Revision3VoiceCatalog.fromContentIndex(
        revision3VoiceContentIndexFixture(existingDeSlot: false),
      );
      final input = Revision3VoiceTakeAuthoringInput(
        lineId: revision3VoiceContentLineId,
        locale: 'en',
        sourcePath: r'C:\Voice\asghan_en.ogg',
        takeDisplayName: 'Asghan English take',
        status: AuthoringRevision3VoiceTakeStatus.reviewed,
      );
      final first = Revision3VoiceTakeTechnicalPlan.forCheckpoint(
        catalog: baseCatalog,
        input: input,
      );
      final occupiedCatalog = Revision3VoiceCatalog.fromContentIndex(
        revision3VoiceContentIndexFixture(
          existingDeSlot: false,
          extraEntityIds: <String>[first.slotId, first.takeId],
        ),
      );
      final probed = Revision3VoiceTakeTechnicalPlan.forCheckpoint(
        catalog: occupiedCatalog,
        input: input,
      );

      expect(probed.slotId, isNot(first.slotId));
      expect(
        probed.takeId,
        isNot(anyOf(<String>[first.takeId, probed.slotId])),
      );
      expect(occupiedCatalog.entityIds, isNot(contains(probed.slotId)));
      expect(occupiedCatalog.entityIds, isNot(contains(probed.takeId)));
    },
  );

  test(
    'service refreshes the exact index and binds the technical publisher',
    () async {
      var current = revision3VoiceContentIndexFixture();
      var loads = 0;
      var publishes = 0;
      Revision3VoiceTakeTechnicalPlan? capturedPlan;
      final service = Revision3VoiceAuthoringService(
        loadContentIndex: () async {
          loads++;
          return current;
        },
        publishTechnicalPlan:
            ({
              required expectedProjectId,
              required expectedProjectRevision,
              required plan,
            }) async {
              publishes++;
              capturedPlan = plan;
              expect(expectedProjectId, revision3VoiceContentProjectId);
              expect(expectedProjectRevision, 7);
              return _publication(plan, revision: 8);
            },
      );
      final checkpoint = await service.loadCatalog();
      final input = Revision3VoiceTakeAuthoringInput(
        lineId: revision3VoiceContentLineId,
        locale: 'de',
        sourcePath: r'C:\Voice\asghan.ogg',
        takeDisplayName: 'Asghan take',
        status: AuthoringRevision3VoiceTakeStatus.recorded,
      );
      final result = await service.publish(
        checkpoint: checkpoint,
        input: input,
      );
      expect(
        loads,
        2,
        reason: 'publication must refresh the exact content index',
      );
      expect(publishes, 1);
      expect(result.takeId, capturedPlan!.takeId);

      current = revision3VoiceContentIndexFixture(revision: 8);
      await expectLater(
        service.publish(checkpoint: checkpoint, input: input),
        throwsA(isA<Revision3VoiceTakeStaleCheckpointException>()),
      );
      expect(publishes, 1, reason: 'stale UI state must not reach publication');
    },
  );

  test(
    'service fails closed when publication facts disagree with its plan',
    () async {
      final index = revision3VoiceContentIndexFixture();
      final service = Revision3VoiceAuthoringService(
        loadContentIndex: () async => index,
        publishTechnicalPlan:
            ({
              required expectedProjectId,
              required expectedProjectRevision,
              required plan,
            }) async => Revision3VoiceTakePublication(
              projectId: expectedProjectId,
              projectRevision: expectedProjectRevision + 1,
              lineId: plan.lineId,
              slotId: plan.slotId,
              takeId: '99999999999999999999999999999999',
              slotCreated: plan.expectsSlotCreated,
              selected: plan.selectTake,
            ),
      );
      final checkpoint = await service.loadCatalog();
      final input = Revision3VoiceTakeAuthoringInput(
        lineId: revision3VoiceContentLineId,
        locale: 'de',
        sourcePath: r'C:\Voice\asghan.ogg',
        takeDisplayName: 'Asghan take',
        status: AuthoringRevision3VoiceTakeStatus.recorded,
      );
      await expectLater(
        service.publish(checkpoint: checkpoint, input: input),
        throwsA(isA<Revision3VoiceTakeRequiresReopenException>()),
      );
    },
  );

  test(
    'target service refreshes exact content and binds only line slot locale and LocID',
    () async {
      var current = revision3VoiceContentIndexFixture();
      var publishes = 0;
      final service = Revision3VoiceTargetAuthoringService(
        loadContentIndex: () async => current,
        publishTechnicalPlan:
            ({
              required expectedProjectId,
              required expectedProjectRevision,
              required plan,
            }) async {
              publishes++;
              expect(plan.lineId, revision3VoiceContentLineId);
              expect(plan.slotId, revision3VoiceContentSlotId);
              expect(plan.locale, 'de');
              expect(plan.locId, 'GRD_263_ASGHAN_OPEN_INFO_06_02');
              return Revision3VoiceTargetPublication(
                projectId: expectedProjectId,
                projectRevision: expectedProjectRevision + 1,
                lineId: plan.lineId,
                slotId: plan.slotId,
                locale: plan.locale,
                locId: plan.locId,
                resolution:
                    AuthoringRevision3VoiceTargetResolutionState.resolved,
                matchCount: 1,
              );
            },
      );
      final checkpoint = await service.loadCatalog();
      final result = await service.resolve(
        checkpoint: checkpoint,
        lineId: revision3VoiceContentLineId,
        locale: 'de',
      );
      expect(result.matchCount, 1);
      expect(publishes, 1);

      current = revision3VoiceContentIndexFixture(revision: 8);
      await expectLater(
        service.resolve(
          checkpoint: checkpoint,
          lineId: revision3VoiceContentLineId,
          locale: 'de',
        ),
        throwsA(isA<Revision3VoiceTargetStaleCheckpointException>()),
      );
      expect(publishes, 1);
    },
  );
}

Revision3VoiceTakeAuthoringInput _input({required String locale}) =>
    Revision3VoiceTakeAuthoringInput(
      lineId: revision3VoiceContentLineId,
      locale: locale,
      sourcePath: 'C:\\Voice\\asghan_$locale.ogg',
      takeDisplayName: 'Asghan $locale take',
      status: AuthoringRevision3VoiceTakeStatus.recorded,
    );

Map<String, Object?> _entity(Map<String, Object?> json, String id) =>
    (json['entities']! as List<Object?>)
        .cast<Map<String, Object?>>()
        .singleWhere((entity) => entity['id'] == id);

Map<String, Object?> _slotData(Map<String, Object?> json) =>
    ((_entity(json, revision3VoiceContentSlotId)['summary']!
            as Map<String, Object?>)['data']!
        as Map<String, Object?>);

Map<String, Object?> _lineData(Map<String, Object?> json) =>
    ((_entity(json, revision3VoiceContentLineId)['summary']!
            as Map<String, Object?>)['data']!
        as Map<String, Object?>);

List<Map<String, Object?>> _lineReferences(Map<String, Object?> json) =>
    (_entity(json, revision3VoiceContentLineId)['references']! as List<Object?>)
        .cast<Map<String, Object?>>();

List<Map<String, Object?>> _slotReferences(Map<String, Object?> json) =>
    (_entity(json, revision3VoiceContentSlotId)['references']! as List<Object?>)
        .cast<Map<String, Object?>>();

Map<String, Object?> _takeData(Map<String, Object?> json) {
  final take = (json['entities']! as List<Object?>)
      .cast<Map<String, Object?>>()
      .firstWhere((entity) => entity['kind'] == 'voice_take');
  return (take['summary']! as Map<String, Object?>)['data']!
      as Map<String, Object?>;
}

Revision3VoiceTakePublication _publication(
  Revision3VoiceTakeTechnicalPlan plan, {
  required int revision,
}) => Revision3VoiceTakePublication(
  projectId: revision3VoiceContentProjectId,
  projectRevision: revision,
  lineId: plan.lineId,
  slotId: plan.slotId,
  takeId: plan.takeId,
  slotCreated: plan.expectsSlotCreated,
  selected: plan.selectTake,
);
