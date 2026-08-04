part of '../core/mod_ffi.dart';

const _maxAuthoringRevision3VoiceBatchItems = 256;
const _maxAuthoringRevision3VoiceBatchLineLabelBytes = 256;
const _maxAuthoringRevision3VoiceBatchSpeakerBytes = 256;

enum AuthoringRevision3VoiceBatchPlanStatus { ready, blocked, noChanges }

enum AuthoringRevision3VoiceBatchItemStatus {
  ready,
  alreadyPresent,
  unmatched,
  ambiguous,
  sourceMissing,
  sourceUnavailable,
  sourceUnsafe,
  sourceLimit,
  sourceInvalid,
  sourceChanged,
  targetBlocked,
  duplicateTarget,
  caseCollision,
}

extension on AuthoringRevision3VoiceBatchItemStatus {
  bool get isReady => this == AuthoringRevision3VoiceBatchItemStatus.ready;

  bool get isAlreadyPresent =>
      this == AuthoringRevision3VoiceBatchItemStatus.alreadyPresent;

  bool get isBlocked => !isReady && !isAlreadyPresent;

  bool get requiresCompleteTarget => isReady || isAlreadyPresent;

  bool get forbidsTarget =>
      this == AuthoringRevision3VoiceBatchItemStatus.unmatched ||
      this == AuthoringRevision3VoiceBatchItemStatus.ambiguous ||
      this == AuthoringRevision3VoiceBatchItemStatus.caseCollision;
}

/// One strictly parsed row from a native, read-only folder scan.
///
/// Stable identities and seals remain backend facts. [sourceName] is the LocID
/// filename and must stay behind the managed presentation adapter together with
/// entity IDs and hashes. Friendly line/speaker facts are independently rebound
/// to the exact current content index before display.
final class AuthoringRevision3VoiceBatchPlanItem {
  const AuthoringRevision3VoiceBatchPlanItem._({
    required this.sourceName,
    required this.status,
    required this.lineDisplayName,
    required this.speaker,
    required this.lineId,
    required this.localizationId,
    required this.locId,
    required this.slotId,
    required this.takeId,
    required this.slotCreated,
    required this.voiceRequest,
    required this.asset,
    required this.ogg,
  });

  final String sourceName;
  final AuthoringRevision3VoiceBatchItemStatus status;
  final String? lineDisplayName;
  final String? speaker;
  final String? lineId;
  final String? localizationId;
  final String? locId;
  final String? slotId;
  final String? takeId;
  final bool? slotCreated;
  final AuthoringRevision3VoiceTakeRequestV1? voiceRequest;
  final AuthoringRevision3VoiceAsset? asset;
  final AuthoringRevision3VoiceOggMetadata? ogg;

  bool get isReady => status.isReady;
  bool get isAlreadyPresent => status.isAlreadyPresent;
  bool get isBlocked => status.isBlocked;

  factory AuthoringRevision3VoiceBatchPlanItem._fromJson(
    Object? value, {
    required AuthoringWorkingHead expectedHead,
    required String currentProjectJson,
    required String expectedLocale,
  }) {
    final json = _authoringRequiredObject(
      value,
      'revision-3 Voice batch plan item',
    );
    const fields = <String>[
      'source_name',
      'status',
      'line_display_name',
      'speaker',
      'line_id',
      'localization_id',
      'loc_id',
      'slot_id',
      'take_id',
      'slot_created',
      'voice_request_json',
      'asset',
      'ogg',
    ];
    _authoringExactFields(
      json,
      fields.toSet(),
      'revision-3 Voice batch plan item',
    );

    final sourceName = _authoringRevision3VoiceString(
      json,
      'source_name',
      maxBytes: _maxAuthoringRevision3VoiceLogicalNameBytes,
    );
    if (!_authoringRevision3VoiceLogicalNameIsSafe(sourceName)) {
      throw const FormatException(
        'revision-3 Voice batch source name is not one safe Ogg leaf',
      );
    }
    final status = _authoringRevision3VoiceBatchItemStatus(json['status']);
    final lineDisplayName = _authoringRevision3VoiceBatchNullableText(
      json['line_display_name'],
      context: 'line display name',
      maxBytes: _maxAuthoringRevision3VoiceBatchLineLabelBytes,
    );
    final speaker = _authoringRevision3VoiceBatchNullableText(
      json['speaker'],
      context: 'speaker',
      maxBytes: _maxAuthoringRevision3VoiceBatchSpeakerBytes,
    );
    final lineId = _authoringRevision3VoiceBatchNullableEntityId(
      json['line_id'],
      'line_id',
    );
    final localizationId = _authoringRevision3VoiceBatchNullableEntityId(
      json['localization_id'],
      'localization_id',
    );
    final locId = _authoringRevision3VoiceBatchNullableLocId(json['loc_id']);
    final slotId = _authoringRevision3VoiceBatchNullableEntityId(
      json['slot_id'],
      'slot_id',
    );
    final takeId = _authoringRevision3VoiceBatchNullableEntityId(
      json['take_id'],
      'take_id',
    );
    final slotCreated = switch (json['slot_created']) {
      null => null,
      bool value => value,
      _ => throw const FormatException(
        'revision-3 Voice batch slot-created fact is invalid',
      ),
    };
    final request = switch (json['voice_request_json']) {
      null => null,
      String value => AuthoringRevision3VoiceTakeRequestV1.fromCanonicalJson(
        value,
      ),
      _ => throw const FormatException(
        'revision-3 Voice batch request is not canonical JSON',
      ),
    };
    final asset = json['asset'] == null
        ? null
        : _authoringRevision3VoiceAsset(json['asset'], logicalName: sourceName);
    final ogg = json['ogg'] == null
        ? null
        : _authoringRevision3VoiceOgg(json['ogg']);

    final targetFacts = <Object?>[
      lineDisplayName,
      lineId,
      localizationId,
      locId,
    ];
    final hasTarget = targetFacts.every((fact) => fact != null);
    final hasNoTarget = targetFacts.every((fact) => fact == null);
    if ((!hasTarget && !hasNoTarget) || (speaker != null && !hasTarget)) {
      throw const FormatException(
        'revision-3 Voice batch target presentation facts are partial',
      );
    }
    if ((asset == null) != (ogg == null)) {
      throw const FormatException(
        'revision-3 Voice batch source validation facts are partial',
      );
    }
    if (status.requiresCompleteTarget && !hasTarget) {
      throw const FormatException(
        'revision-3 Voice batch actionable item has no exact target',
      );
    }
    if (status.forbidsTarget && !hasNoTarget) {
      throw const FormatException(
        'revision-3 Voice batch unmapped item invents a target',
      );
    }
    if (status.requiresCompleteTarget &&
        (slotId == null || takeId == null || slotCreated == null)) {
      throw const FormatException(
        'revision-3 Voice batch actionable item has partial transaction facts',
      );
    }
    if (!status.requiresCompleteTarget &&
        (slotId != null ||
            takeId != null ||
            slotCreated != null ||
            request != null)) {
      throw const FormatException(
        'revision-3 Voice batch blocked item grants transaction facts',
      );
    }
    if (status.isReady) {
      if (request == null || asset == null || ogg == null) {
        throw const FormatException(
          'revision-3 Voice batch ready item is not fully sealed',
        );
      }
      final current = _authoringRequireCanonicalRevision3ProjectJson(
        currentProjectJson,
      );
      request._requireExactProjectBinding(current);
      if (request.expectedHead.canonicalJson != expectedHead.canonicalJson ||
          request.lineId != lineId ||
          request.slotId != slotId ||
          request.takeId != takeId ||
          request.locale != expectedLocale ||
          request.logicalName != sourceName ||
          request.text != null ||
          request.status != AuthoringRevision3VoiceTakeStatus.recorded ||
          request.selectTake) {
        throw const FormatException(
          'revision-3 Voice batch ready item disagrees with its exact request',
        );
      }
    } else if (request != null) {
      throw const FormatException(
        'revision-3 Voice batch non-ready item carries mutation authority',
      );
    }
    if (status.isAlreadyPresent &&
        (slotCreated != false || asset == null || ogg == null)) {
      throw const FormatException(
        'revision-3 Voice batch existing item facts are invalid',
      );
    }
    final semanticIds = <String?>[
      lineId,
      localizationId,
      slotId,
      takeId,
    ].nonNulls.toList(growable: false);
    if (semanticIds.toSet().length != semanticIds.length) {
      throw const FormatException(
        'revision-3 Voice batch item aliases distinct entity roles',
      );
    }
    return AuthoringRevision3VoiceBatchPlanItem._(
      sourceName: sourceName,
      status: status,
      lineDisplayName: lineDisplayName,
      speaker: speaker,
      lineId: lineId,
      localizationId: localizationId,
      locId: locId,
      slotId: slotId,
      takeId: takeId,
      slotCreated: slotCreated,
      voiceRequest: request,
      asset: asset,
      ogg: ogg,
    );
  }
}

/// Exact, read-only folder scan and semantic plan. It grants no project,
/// build, runtime, deployment, game, or save write authority.
final class AuthoringRevision3VoiceBatchPlanResult {
  AuthoringRevision3VoiceBatchPlanResult._({
    required this.basisHead,
    required this.projectId,
    required this.revision,
    required this.locale,
    required this.sourceManifestSha256,
    required this.planSha256,
    required this.status,
    required this.scannedEntryCount,
    required this.oggFileCount,
    required this.readyCount,
    required this.alreadyPresentCount,
    required this.blockedCount,
    required this.ignoredEntryCount,
    required List<AuthoringRevision3VoiceBatchPlanItem> items,
  }) : items = List.unmodifiable(items);

  final AuthoringWorkingHead basisHead;
  final String projectId;
  final int revision;
  final String locale;
  final String sourceManifestSha256;
  final String planSha256;
  final AuthoringRevision3VoiceBatchPlanStatus status;
  final int scannedEntryCount;
  final int oggFileCount;
  final int readyCount;
  final int alreadyPresentCount;
  final int blockedCount;
  final int ignoredEntryCount;
  final List<AuthoringRevision3VoiceBatchPlanItem> items;

  bool get canPrepare => status == AuthoringRevision3VoiceBatchPlanStatus.ready;

  factory AuthoringRevision3VoiceBatchPlanResult.fromJson(
    Map<String, Object?> json, {
    required AuthoringWorkingHead expectedHead,
    required String currentProjectJson,
    required String expectedLocale,
  }) {
    const fields = <String>[
      'ok',
      'outcome',
      'basis_head_json',
      'project_id',
      'revision',
      'locale',
      'source_manifest_sha256',
      'plan_sha256',
      'status',
      'scanned_entry_count',
      'ogg_file_count',
      'ready_count',
      'already_present_count',
      'blocked_count',
      'ignored_entry_count',
      'items',
      'build_status',
      'runtime_status',
      'target_authority',
      'publication_status',
    ];
    _authoringExactFields(
      json,
      fields.toSet(),
      'revision-3 Voice batch plan response',
    );
    if (json['ok'] != true || json['outcome'] != 'planned') {
      throw const FormatException(
        'revision-3 Voice batch response is not a plan',
      );
    }
    final current = _authoringRequireCanonicalRevision3ProjectJson(
      currentProjectJson,
    );
    final basisHead = AuthoringWorkingHead.fromCanonicalJson(
      _authoringRevision3VoiceString(
        json,
        'basis_head_json',
        maxBytes: _maxAuthoringHeadJsonBytes,
      ),
    );
    final projectId = _authoringRevision3VoiceEntityId(json, 'project_id');
    final revision = _authoringRequiredInt(
      json,
      'revision',
      max: _maxAuthoringRevision3VoiceBasisRevision,
    );
    final locale = _authoringRevision3VoiceLocale(
      _authoringRevision3VoiceString(json, 'locale', maxBytes: 35),
    );
    final sourceManifestSha256 = _authoringRevision3VoiceBatchSha(
      json,
      'source_manifest_sha256',
    );
    final planSha256 = _authoringRevision3VoiceBatchSha(json, 'plan_sha256');
    final status = switch (json['status']) {
      'ready' => AuthoringRevision3VoiceBatchPlanStatus.ready,
      'blocked' => AuthoringRevision3VoiceBatchPlanStatus.blocked,
      'no_changes' => AuthoringRevision3VoiceBatchPlanStatus.noChanges,
      _ => throw const FormatException(
        'revision-3 Voice batch plan status is invalid',
      ),
    };
    final scannedEntryCount = _authoringRequiredInt(
      json,
      'scanned_entry_count',
      max: 0x7fffffff,
    );
    final oggFileCount = _authoringRequiredInt(
      json,
      'ogg_file_count',
      max: _maxAuthoringRevision3VoiceBatchItems,
    );
    final readyCount = _authoringRequiredInt(
      json,
      'ready_count',
      max: oggFileCount,
    );
    final alreadyPresentCount = _authoringRequiredInt(
      json,
      'already_present_count',
      max: oggFileCount,
    );
    final blockedCount = _authoringRequiredInt(
      json,
      'blocked_count',
      max: oggFileCount,
    );
    final ignoredEntryCount = _authoringRequiredInt(
      json,
      'ignored_entry_count',
      max: scannedEntryCount,
    );
    final rawItems = json['items'];
    if (rawItems is! List || rawItems.length != oggFileCount) {
      throw const FormatException(
        'revision-3 Voice batch item count is invalid',
      );
    }
    final items = rawItems
        .map(
          (item) => AuthoringRevision3VoiceBatchPlanItem._fromJson(
            item,
            expectedHead: expectedHead,
            currentProjectJson: currentProjectJson,
            expectedLocale: expectedLocale,
          ),
        )
        .toList(growable: false);
    _authoringRevision3VoiceBatchRequireSortedUniqueItems(items);
    _authoringRevision3VoiceBatchRequireExactPlanTargets(
      current.project,
      projectId: projectId,
      locale: locale,
      items: items,
    );
    final actualReady = items.where((item) => item.isReady).length;
    final actualExisting = items.where((item) => item.isAlreadyPresent).length;
    final actualBlocked = items.where((item) => item.isBlocked).length;
    final targetKeys = <String>{};
    for (final item in items.where(
      (item) => item.isReady || item.isAlreadyPresent,
    )) {
      if (!targetKeys.add('${item.lineId}\u0000$locale')) {
        throw const FormatException(
          'revision-3 Voice batch plan duplicates a mutation target',
        );
      }
    }
    final statusIsExact = switch (status) {
      AuthoringRevision3VoiceBatchPlanStatus.ready =>
        blockedCount == 0 && readyCount > 0,
      AuthoringRevision3VoiceBatchPlanStatus.blocked => blockedCount > 0,
      AuthoringRevision3VoiceBatchPlanStatus.noChanges =>
        blockedCount == 0 && readyCount == 0,
    };
    if (basisHead.canonicalJson != expectedHead.canonicalJson ||
        projectId != current.projectId ||
        revision != current.revision ||
        locale != expectedLocale ||
        readyCount != actualReady ||
        alreadyPresentCount != actualExisting ||
        blockedCount != actualBlocked ||
        readyCount + alreadyPresentCount + blockedCount != oggFileCount ||
        scannedEntryCount != oggFileCount + ignoredEntryCount ||
        !statusIsExact ||
        json['build_status'] != 'blocked' ||
        json['runtime_status'] != 'runtime_unqualified' ||
        json['target_authority'] != 'not_granted' ||
        json['publication_status'] != 'not_supported') {
      throw const FormatException(
        'revision-3 Voice batch plan disagrees with its exact basis, counts, or authority boundary',
      );
    }
    return AuthoringRevision3VoiceBatchPlanResult._(
      basisHead: basisHead,
      projectId: projectId,
      revision: revision,
      locale: locale,
      sourceManifestSha256: sourceManifestSha256,
      planSha256: planSha256,
      status: status,
      scannedEntryCount: scannedEntryCount,
      oggFileCount: oggFileCount,
      readyCount: readyCount,
      alreadyPresentCount: alreadyPresentCount,
      blockedCount: blockedCount,
      ignoredEntryCount: ignoredEntryCount,
      items: items,
    );
  }
}

final class AuthoringRevision3VoiceBatchPreparationItem {
  const AuthoringRevision3VoiceBatchPreparationItem._({
    required this.sourceName,
    required this.lineId,
    required this.localizationId,
    required this.slotId,
    required this.takeId,
    required this.slotCreated,
    required this.asset,
    required this.ogg,
    required this.assetDeduplicated,
  });

  final String sourceName;
  final String lineId;
  final String localizationId;
  final String slotId;
  final String takeId;
  final bool slotCreated;
  final AuthoringRevision3VoiceAsset asset;
  final AuthoringRevision3VoiceOggMetadata ogg;
  final bool assetDeduplicated;

  factory AuthoringRevision3VoiceBatchPreparationItem._fromJson(
    Object? value, {
    required AuthoringRevision3VoiceBatchPlanItem planItem,
  }) {
    final json = _authoringRequiredObject(
      value,
      'revision-3 Voice batch preparation item',
    );
    const fields = <String>[
      'source_name',
      'line_id',
      'localization_id',
      'slot_id',
      'take_id',
      'take_status',
      'slot_created',
      'selected',
      'asset',
      'ogg',
      'asset_deduplicated',
    ];
    _authoringExactFields(
      json,
      fields.toSet(),
      'revision-3 Voice batch preparation item',
    );
    final sourceName = _authoringRevision3VoiceString(
      json,
      'source_name',
      maxBytes: _maxAuthoringRevision3VoiceLogicalNameBytes,
    );
    final lineId = _authoringRevision3VoiceEntityId(json, 'line_id');
    final localizationId = _authoringRevision3VoiceEntityId(
      json,
      'localization_id',
    );
    final slotId = _authoringRevision3VoiceEntityId(json, 'slot_id');
    final takeId = _authoringRevision3VoiceEntityId(json, 'take_id');
    final slotCreated = _authoringRequiredBool(json, 'slot_created');
    final asset = _authoringRevision3VoiceAsset(
      json['asset'],
      logicalName: sourceName,
    );
    final ogg = _authoringRevision3VoiceOgg(json['ogg']);
    final assetDeduplicated = _authoringRequiredBool(
      json,
      'asset_deduplicated',
    );
    if (!planItem.isReady ||
        sourceName != planItem.sourceName ||
        lineId != planItem.lineId ||
        localizationId != planItem.localizationId ||
        slotId != planItem.slotId ||
        takeId != planItem.takeId ||
        slotCreated != planItem.slotCreated ||
        json['take_status'] != 'recorded' ||
        json['selected'] != false ||
        !_authoringRevision3VoiceBatchSameAsset(asset, planItem.asset!) ||
        !_authoringRevision3VoiceBatchSameOgg(ogg, planItem.ogg!)) {
      throw const FormatException(
        'revision-3 Voice batch preparation item disagrees with its sealed plan',
      );
    }
    return AuthoringRevision3VoiceBatchPreparationItem._(
      sourceName: sourceName,
      lineId: lineId,
      localizationId: localizationId,
      slotId: slotId,
      takeId: takeId,
      slotCreated: slotCreated,
      asset: asset,
      ogg: ogg,
      assetDeduplicated: assetDeduplicated,
    );
  }
}

/// One complete unpublished project candidate for every ready row in [plan].
/// Managed publication must still reopen and win the exact fixed-head CAS.
final class AuthoringRevision3VoiceBatchPreparation {
  AuthoringRevision3VoiceBatchPreparation._({
    required this.basisHead,
    required this.head,
    required this.projectJson,
    required this.projectId,
    required this.revision,
    required this.locale,
    required this.sourceManifestSha256,
    required this.planSha256,
    required this.importedCount,
    required this.alreadyPresentCount,
    required List<AuthoringRevision3VoiceBatchPreparationItem> items,
  }) : items = List.unmodifiable(items);

  final AuthoringWorkingHead basisHead;
  final AuthoringWorkingHead head;
  final String projectJson;
  final String projectId;
  final int revision;
  final String locale;
  final String sourceManifestSha256;
  final String planSha256;
  final int importedCount;
  final int alreadyPresentCount;
  final List<AuthoringRevision3VoiceBatchPreparationItem> items;

  factory AuthoringRevision3VoiceBatchPreparation.fromJson(
    Map<String, Object?> json, {
    required String currentProjectJson,
    required AuthoringRevision3VoiceBatchPlanResult plan,
  }) {
    const fields = <String>[
      'ok',
      'outcome',
      'basis_head_json',
      'head_json',
      'project_json',
      'project_id',
      'revision',
      'locale',
      'source_manifest_sha256',
      'plan_sha256',
      'imported_count',
      'already_present_count',
      'items',
      'build_status',
      'runtime_status',
      'target_authority',
      'publication_status',
    ];
    _authoringExactFields(
      json,
      fields.toSet(),
      'revision-3 Voice batch preparation response',
    );
    if (json['ok'] != true ||
        json['outcome'] != 'prepared_unpublished' ||
        !plan.canPrepare) {
      throw const FormatException(
        'revision-3 Voice batch response is not one planned preparation',
      );
    }
    final base = _authoringRequireCanonicalRevision3ProjectJson(
      currentProjectJson,
    );
    final basisHead = AuthoringWorkingHead.fromCanonicalJson(
      _authoringRevision3VoiceString(
        json,
        'basis_head_json',
        maxBytes: _maxAuthoringHeadJsonBytes,
      ),
    );
    final head = AuthoringWorkingHead.fromCanonicalJson(
      _authoringRevision3VoiceString(
        json,
        'head_json',
        maxBytes: _maxAuthoringHeadJsonBytes,
      ),
    );
    final projectJson = _authoringRevision3VoiceString(
      json,
      'project_json',
      maxBytes: _maxAuthoringProjectJsonBytes,
    );
    final candidate = _authoringRequireCanonicalRevision3ProjectJson(
      projectJson,
    );
    final projectId = _authoringRevision3VoiceEntityId(json, 'project_id');
    final revision = _authoringRequiredInt(
      json,
      'revision',
      min: 1,
      max: _maxAuthoringRevision3VoiceAppliedRevision,
    );
    final locale = _authoringRevision3VoiceLocale(
      _authoringRevision3VoiceString(json, 'locale', maxBytes: 35),
    );
    final manifestSha = _authoringRevision3VoiceBatchSha(
      json,
      'source_manifest_sha256',
    );
    final planSha = _authoringRevision3VoiceBatchSha(json, 'plan_sha256');
    final importedCount = _authoringRequiredInt(
      json,
      'imported_count',
      min: 1,
      max: _maxAuthoringRevision3VoiceBatchItems,
    );
    final alreadyPresentCount = _authoringRequiredInt(
      json,
      'already_present_count',
      max: _maxAuthoringRevision3VoiceBatchItems,
    );
    final rawItems = json['items'];
    if (rawItems is! List || rawItems.length != importedCount) {
      throw const FormatException(
        'revision-3 Voice batch preparation count is invalid',
      );
    }
    final readyByName = <String, AuthoringRevision3VoiceBatchPlanItem>{
      for (final item in plan.items.where((item) => item.isReady))
        item.sourceName: item,
    };
    final items = <AuthoringRevision3VoiceBatchPreparationItem>[];
    final seen = <String>{};
    for (final raw in rawItems) {
      final itemJson = _authoringRequiredObject(
        raw,
        'revision-3 Voice batch preparation item',
      );
      final sourceName = itemJson['source_name'];
      final planItem = sourceName is String ? readyByName[sourceName] : null;
      if (planItem == null || !seen.add(sourceName as String)) {
        throw const FormatException(
          'revision-3 Voice batch preparation item is absent or duplicated in its plan',
        );
      }
      items.add(
        AuthoringRevision3VoiceBatchPreparationItem._fromJson(
          raw,
          planItem: planItem,
        ),
      );
    }
    if (basisHead.canonicalJson != plan.basisHead.canonicalJson ||
        head.canonicalJson == basisHead.canonicalJson ||
        base.projectId != plan.projectId ||
        base.revision != plan.revision ||
        candidate.projectId != base.projectId ||
        projectId != base.projectId ||
        candidate.revision != base.revision + 1 ||
        revision != candidate.revision ||
        locale != plan.locale ||
        manifestSha != plan.sourceManifestSha256 ||
        planSha != plan.planSha256 ||
        importedCount != plan.readyCount ||
        alreadyPresentCount != plan.alreadyPresentCount ||
        json['build_status'] != 'blocked' ||
        json['runtime_status'] != 'runtime_unqualified' ||
        json['target_authority'] != 'not_granted' ||
        json['publication_status'] != 'not_supported') {
      throw const FormatException(
        'revision-3 Voice batch preparation disagrees with its exact plan, basis, or authority boundary',
      );
    }
    _authoringRevision3VoiceBatchRequireExactCandidate(
      base.project,
      candidate.project,
      plan: plan,
      items: items,
    );
    return AuthoringRevision3VoiceBatchPreparation._(
      basisHead: basisHead,
      head: head,
      projectJson: projectJson,
      projectId: projectId,
      revision: revision,
      locale: locale,
      sourceManifestSha256: manifestSha,
      planSha256: planSha,
      importedCount: importedCount,
      alreadyPresentCount: alreadyPresentCount,
      items: items,
    );
  }
}

AuthoringRevision3VoiceBatchItemStatus _authoringRevision3VoiceBatchItemStatus(
  Object? value,
) => switch (value) {
  'ready' => AuthoringRevision3VoiceBatchItemStatus.ready,
  'already_present' => AuthoringRevision3VoiceBatchItemStatus.alreadyPresent,
  'unmatched' => AuthoringRevision3VoiceBatchItemStatus.unmatched,
  'ambiguous' => AuthoringRevision3VoiceBatchItemStatus.ambiguous,
  'source_missing' => AuthoringRevision3VoiceBatchItemStatus.sourceMissing,
  'source_unavailable' =>
    AuthoringRevision3VoiceBatchItemStatus.sourceUnavailable,
  'source_unsafe' => AuthoringRevision3VoiceBatchItemStatus.sourceUnsafe,
  'source_limit' => AuthoringRevision3VoiceBatchItemStatus.sourceLimit,
  'source_invalid' => AuthoringRevision3VoiceBatchItemStatus.sourceInvalid,
  'source_changed' => AuthoringRevision3VoiceBatchItemStatus.sourceChanged,
  'target_blocked' => AuthoringRevision3VoiceBatchItemStatus.targetBlocked,
  'duplicate_target' => AuthoringRevision3VoiceBatchItemStatus.duplicateTarget,
  'case_collision' => AuthoringRevision3VoiceBatchItemStatus.caseCollision,
  _ => throw const FormatException(
    'revision-3 Voice batch item status is invalid',
  ),
};

String? _authoringRevision3VoiceBatchNullableText(
  Object? value, {
  required String context,
  required int maxBytes,
}) {
  if (value == null) return null;
  if (value is! String ||
      value.isEmpty ||
      value.trim() != value ||
      utf8.encode(value).length > maxBytes ||
      value.runes.any(_authoringRevision3VoiceControl)) {
    throw FormatException('revision-3 Voice batch $context is invalid');
  }
  return value;
}

String? _authoringRevision3VoiceBatchNullableEntityId(
  Object? value,
  String field,
) {
  if (value == null) return null;
  if (value is! String) {
    throw FormatException('revision-3 Voice batch $field is invalid');
  }
  return _authoringRevision3VoiceEntityId(<String, Object?>{
    field: value,
  }, field);
}

String? _authoringRevision3VoiceBatchNullableLocId(Object? value) {
  if (value == null) return null;
  if (value is! String ||
      !authoringRevision3VoiceArchiveBasenameStemIsSafe(value)) {
    throw const FormatException(
      'revision-3 Voice batch localization basename is invalid',
    );
  }
  return value;
}

String _authoringRevision3VoiceBatchSha(
  Map<String, Object?> json,
  String field,
) {
  final sha = _authoringRevision3VoiceString(json, field, maxBytes: 64);
  if (!_authoringSha256Pattern.hasMatch(sha) ||
      sha == _authoringRevision3VoiceZeroSha256) {
    throw FormatException('revision-3 Voice batch $field is invalid');
  }
  return sha;
}

void _authoringRevision3VoiceBatchRequireSortedUniqueItems(
  List<AuthoringRevision3VoiceBatchPlanItem> items,
) {
  final foldedCounts = <String, int>{};
  for (final item in items) {
    final folded = _authoringRevision3VoiceBatchAsciiFold(item.sourceName);
    foldedCounts.update(folded, (count) => count + 1, ifAbsent: () => 1);
  }
  final exactNames = <String>{};
  String? previousFolded;
  String? previousExact;
  for (final item in items) {
    final folded = _authoringRevision3VoiceBatchAsciiFold(item.sourceName);
    final exact = item.sourceName;
    final foldedCollision = foldedCounts[folded]! > 1;
    if (!exactNames.add(exact) ||
        (previousFolded != null &&
            _authoringRevision3VoiceBatchCompareUtf8(previousFolded, folded) >
                0) ||
        (previousFolded == folded &&
            _authoringRevision3VoiceBatchCompareUtf8(previousExact!, exact) >=
                0) ||
        (foldedCollision !=
            (item.status ==
                AuthoringRevision3VoiceBatchItemStatus.caseCollision))) {
      throw const FormatException(
        'revision-3 Voice batch items are not deterministically unique and sorted',
      );
    }
    previousFolded = folded;
    previousExact = exact;
  }
}

String _authoringRevision3VoiceBatchAsciiFold(String value) =>
    String.fromCharCodes(
      value.codeUnits.map(
        (unit) => unit >= 0x41 && unit <= 0x5a ? unit + 0x20 : unit,
      ),
    );

int _authoringRevision3VoiceBatchCompareUtf8(String left, String right) {
  final leftBytes = utf8.encode(left);
  final rightBytes = utf8.encode(right);
  final shared = leftBytes.length < rightBytes.length
      ? leftBytes.length
      : rightBytes.length;
  for (var index = 0; index < shared; index++) {
    final comparison = leftBytes[index].compareTo(rightBytes[index]);
    if (comparison != 0) return comparison;
  }
  return leftBytes.length.compareTo(rightBytes.length);
}

bool _authoringRevision3VoiceBatchSameAsset(
  AuthoringRevision3VoiceAsset left,
  AuthoringRevision3VoiceAsset right,
) =>
    left.sha256 == right.sha256 &&
    left.byteLength == right.byteLength &&
    left.logicalName == right.logicalName;

bool _authoringRevision3VoiceBatchSameOgg(
  AuthoringRevision3VoiceOggMetadata left,
  AuthoringRevision3VoiceOggMetadata right,
) =>
    left.codec == right.codec &&
    left.channels == right.channels &&
    left.sampleRate == right.sampleRate &&
    left.pages == right.pages &&
    left.logicalStreams == right.logicalStreams;

void _authoringRevision3VoiceBatchRequireExactPlanTargets(
  Map<String, Object?> project, {
  required String projectId,
  required String locale,
  required List<AuthoringRevision3VoiceBatchPlanItem> items,
}) {
  final entities = _authoringRequiredObject(
    project['entities'],
    'revision-3 Voice batch basis entities',
  );
  for (final item in items) {
    final lineId = item.lineId;
    if (lineId == null) continue;
    final line = _authoringRevision3VoiceEntity(
      entities,
      lineId,
      'dialog_line',
      'batch basis line',
    );
    _authoringRevision3VoiceExactOptionalFields(
      line.data,
      const {'localization', 'voice_slots'},
      const {'speaker_hint'},
      'batch basis DialogLine data',
    );
    final lineDisplayName = _authoringRevision3VoiceString(
      line.entity,
      'display_name',
      maxBytes: _maxAuthoringRevision3VoiceBatchLineLabelBytes,
    );
    final speaker = switch (line.data['speaker_hint']) {
      null => null,
      String value => _authoringRevision3VoiceBatchNullableText(
        value,
        context: 'basis speaker',
        maxBytes: _maxAuthoringRevision3VoiceBatchSpeakerBytes,
      ),
      _ => throw const FormatException(
        'revision-3 Voice batch basis speaker is invalid',
      ),
    };
    final localizationRef = _authoringRevision3VoiceTypedRef(
      line.data['localization'],
      projectId: projectId,
      kind: 'localization_entry',
      context: 'batch basis line localization',
    );
    final localization = _authoringRevision3VoiceEntity(
      entities,
      localizationRef.id,
      'localization_entry',
      'batch basis localization',
    );
    _authoringExactFields(localization.data, const {
      'loc_id',
      'texts',
    }, 'revision-3 Voice batch basis LocalizationEntry data');
    final locId = _authoringRevision3VoiceString(
      localization.data,
      'loc_id',
      maxBytes: 1024,
    );
    if (!authoringRevision3VoiceArchiveBasenameStemIsSafe(locId) ||
        lineDisplayName != item.lineDisplayName ||
        speaker != item.speaker ||
        localizationRef.id != item.localizationId ||
        locId != item.locId) {
      throw const FormatException(
        'revision-3 Voice batch target facts disagree with the exact basis',
      );
    }

    if (!item.status.requiresCompleteTarget) continue;
    final slots = _authoringRequiredObject(
      line.data['voice_slots'],
      'revision-3 Voice batch basis line slots',
    );
    final existingSlot = slots[locale];
    if (item.slotCreated!) {
      if (!item.isReady ||
          existingSlot != null ||
          entities.containsKey(item.slotId) ||
          entities.containsKey(item.takeId)) {
        throw const FormatException(
          'revision-3 Voice batch new-slot facts disagree with the exact basis',
        );
      }
      continue;
    }

    final slotRef = _authoringRevision3VoiceTypedRef(
      existingSlot,
      projectId: projectId,
      kind: 'voice_slot',
      context: 'batch basis existing slot',
    );
    if (slotRef.id != item.slotId) {
      throw const FormatException(
        'revision-3 Voice batch existing slot differs from the exact basis',
      );
    }
    _authoringRevision3VoiceValidateExistingSlot(
      entities,
      projectId: projectId,
      lineId: lineId,
      slotId: slotRef.id,
      locale: locale,
      locId: locId,
    );
    if (item.isReady) {
      if (entities.containsKey(item.takeId)) {
        throw const FormatException(
          'revision-3 Voice batch ready take already exists in the basis',
        );
      }
      _authoringRevision3VoiceRequireAddTakeCapacity(
        entities,
        slotId: slotRef.id,
      );
      continue;
    }

    final slot = _authoringRevision3VoiceEntity(
      entities,
      slotRef.id,
      'voice_slot',
      'batch exact existing slot',
    );
    final candidateIds =
        _authoringRevision3VoiceObjectList(
              slot.data['candidates'],
              'revision-3 Voice batch existing candidates',
            )
            .map(
              (candidate) => _authoringRevision3VoiceTypedRef(
                candidate,
                projectId: projectId,
                kind: 'voice_take',
                context: 'batch existing candidate',
              ).id,
            )
            .toSet();
    if (!candidateIds.contains(item.takeId)) {
      throw const FormatException(
        'revision-3 Voice batch existing take is not a slot candidate',
      );
    }
    final take = _authoringRevision3VoiceEntity(
      entities,
      item.takeId!,
      'voice_take',
      'batch exact existing take',
    );
    _authoringExactFields(take.data, const {
      'locale',
      'asset',
      'ogg',
      'status',
    }, 'revision-3 Voice batch exact existing take data');
    final asset = _authoringRequiredObject(
      take.data['asset'],
      'revision-3 Voice batch existing take asset',
    );
    final ogg = _authoringRequiredObject(
      take.data['ogg'],
      'revision-3 Voice batch existing take Ogg metadata',
    );
    final expectedOgg = <String, Object?>{
      'codec': switch (item.ogg!.codec) {
        AuthoringRevision3VoiceOggCodec.vorbis => 'vorbis',
        AuthoringRevision3VoiceOggCodec.opus => 'opus',
      },
      'channels': item.ogg!.channels,
      'sample_rate': item.ogg!.sampleRate,
      'pages': item.ogg!.pages,
      'logical_streams': item.ogg!.logicalStreams,
    };
    if (take.data['locale'] != locale ||
        asset['sha256'] != item.asset!.sha256 ||
        asset['byte_len'] != item.asset!.byteLength ||
        !_authoringRevision3VoiceDeepEqual(ogg, expectedOgg)) {
      throw const FormatException(
        'revision-3 Voice batch existing take differs from the exact source',
      );
    }
  }
}

void _authoringRevision3VoiceBatchRequireExactCandidate(
  Map<String, Object?> base,
  Map<String, Object?> candidate, {
  required AuthoringRevision3VoiceBatchPlanResult plan,
  required List<AuthoringRevision3VoiceBatchPreparationItem> items,
}) {
  final expected = _authoringRevision3VoiceCloneObject(
    base,
    'revision-3 Voice batch expected candidate',
  );
  expected['revision'] = plan.revision + 1;
  final locales = _authoringRevision3VoiceStringList(
    expected['authoring_locales'],
    'revision-3 Voice batch expected locales',
  );
  if (!locales.contains(plan.locale)) locales.add(plan.locale);
  locales.sort();
  expected['authoring_locales'] = locales;
  final entities = _authoringRequiredObject(
    expected['entities'],
    'revision-3 Voice batch expected entities',
  );
  final assetStore = _authoringRequiredObject(
    expected['asset_store'],
    'revision-3 Voice batch expected asset store',
  );
  final assets = _authoringRequiredObject(
    assetStore['assets'],
    'revision-3 Voice batch expected assets',
  );

  for (final prepared in items) {
    final planItem = plan.items.singleWhere(
      (item) => item.sourceName == prepared.sourceName && item.isReady,
    );
    final request = planItem.voiceRequest!;
    final lineRecord = _authoringRevision3VoiceEntity(
      entities,
      request.lineId,
      'dialog_line',
      'batch line',
    );
    final localizationRef = _authoringRevision3VoiceTypedRef(
      lineRecord.data['localization'],
      projectId: request.expectedProjectId,
      kind: 'localization_entry',
      context: 'batch line localization',
    );
    if (localizationRef.id != prepared.localizationId) {
      throw const FormatException(
        'revision-3 Voice batch localization identity disagrees',
      );
    }
    final slots = _authoringRequiredObject(
      lineRecord.data['voice_slots'],
      'revision-3 Voice batch line slots',
    );
    final takeRef = <String, Object?>{
      'project_id': request.expectedProjectId,
      'id': request.takeId,
      'expected_kind': 'voice_take',
    };
    if (prepared.slotCreated) {
      if (slots[request.locale] != null ||
          entities.containsKey(request.slotId)) {
        throw const FormatException(
          'revision-3 Voice batch new slot collides with its basis',
        );
      }
      slots[request.locale] = <String, Object?>{
        'project_id': request.expectedProjectId,
        'id': request.slotId,
        'expected_kind': 'voice_slot',
      };
      lineRecord.entity['revision'] = _authoringRevision3VoiceIncrementRevision(
        lineRecord.entity,
      );
      lineRecord.data['voice_slots'] = slots;
      final linePayload = _authoringRequiredObject(
        lineRecord.entity['payload'],
        'revision-3 Voice batch expected line payload',
      );
      linePayload['data'] = lineRecord.data;
      lineRecord.entity['payload'] = linePayload;
      entities[request.lineId] = lineRecord.entity;
      entities[request.slotId] = <String, Object?>{
        'id': request.slotId,
        'display_name': 'Voice ${request.locale}',
        'origin': <String, Object?>{
          'type': 'generated',
          'generator_id': _authoringRevision3VoiceSlotGeneratorId,
          'generator_version': _authoringRevision3VoiceSlotGeneratorVersion,
          'owner': <String, Object?>{
            'project_id': request.expectedProjectId,
            'id': request.lineId,
            'expected_kind': 'dialog_line',
          },
        },
        'revision': 0,
        'payload': <String, Object?>{
          'kind': 'voice_slot',
          'data': <String, Object?>{
            'locale': request.locale,
            'target_resolution': <String, Object?>{'state': 'unresolved'},
            'candidates': <Object?>[takeRef],
          },
        },
      };
    } else {
      final slotRef = _authoringRevision3VoiceTypedRef(
        slots[request.locale],
        projectId: request.expectedProjectId,
        kind: 'voice_slot',
        context: 'batch existing slot',
      );
      if (slotRef.id != request.slotId) {
        throw const FormatException(
          'revision-3 Voice batch existing slot identity disagrees',
        );
      }
      final slot = _authoringRevision3VoiceEntity(
        entities,
        request.slotId,
        'voice_slot',
        'batch existing slot',
      );
      final candidates = _authoringRevision3VoiceObjectList(
        slot.data['candidates'],
        'revision-3 Voice batch existing slot candidates',
      )..add(takeRef);
      slot.data['candidates'] = candidates;
      slot.entity['revision'] = _authoringRevision3VoiceIncrementRevision(
        slot.entity,
      );
      final slotPayload = _authoringRequiredObject(
        slot.entity['payload'],
        'revision-3 Voice batch expected slot payload',
      );
      slotPayload['data'] = slot.data;
      slot.entity['payload'] = slotPayload;
      entities[request.slotId] = slot.entity;
    }
    entities[request.takeId] = <String, Object?>{
      'id': request.takeId,
      'display_name': request.takeDisplayName,
      'origin': <String, Object?>{
        'type': 'imported',
        'importer': _authoringRevision3VoiceTakeImporterId,
        'source_seal': <String, Object?>{
          'byte_len': prepared.asset.byteLength,
          'sha256': prepared.asset.sha256,
        },
      },
      'revision': 0,
      'payload': <String, Object?>{
        'kind': 'voice_take',
        'data': <String, Object?>{
          'locale': request.locale,
          'asset': <String, Object?>{
            'sha256': prepared.asset.sha256,
            'byte_len': prepared.asset.byteLength,
            'logical_name': prepared.asset.logicalName,
          },
          'ogg': <String, Object?>{
            'codec': switch (prepared.ogg.codec) {
              AuthoringRevision3VoiceOggCodec.vorbis => 'vorbis',
              AuthoringRevision3VoiceOggCodec.opus => 'opus',
            },
            'channels': prepared.ogg.channels,
            'sample_rate': prepared.ogg.sampleRate,
            'pages': prepared.ogg.pages,
            'logical_streams': prepared.ogg.logicalStreams,
          },
          'status': 'recorded',
        },
      },
    };
    final expectedAsset = <String, Object?>{
      'byte_len': prepared.asset.byteLength,
      'media_type': 'audio/ogg',
    };
    final existingAsset = assets[prepared.asset.sha256];
    if (existingAsset != null &&
        !_authoringRevision3VoiceDeepEqual(existingAsset, expectedAsset)) {
      throw const FormatException(
        'revision-3 Voice batch asset metadata conflicts with its basis',
      );
    }
    assets[prepared.asset.sha256] = expectedAsset;
  }
  expected['entities'] = entities;
  assetStore['assets'] = assets;
  expected['asset_store'] = assetStore;
  if (!_authoringRevision3VoiceDeepEqual(expected, candidate)) {
    throw const FormatException(
      'revision-3 Voice batch candidate contains a non-exact project delta',
    );
  }
}
