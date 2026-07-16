part of '../core/mod_ffi.dart';

const _maxAuthoringRevision3VoiceBuildCount = 1024;
const _maxAuthoringRevision3VoiceModelSlotCount = 100000;
const _maxAuthoringRevision3VoiceBuildBlockers =
    _maxAuthoringRevision3VoiceBuildCount * 2;
const _maxAuthoringRevision3VoiceBuildLineLabelBytes = 256;
const _maxAuthoringRevision3VoiceBuildBytes = 0x7fffffffffffffff;
const _maxAuthoringRevision3VoiceBuildSelectedPayloadBytes = 256 * 1024 * 1024;
const _maxAuthoringRevision3VoiceBuildTargetArchiveBytes =
    16 * 1024 * 1024 * 1024;
const _maxAuthoringRevision3VoiceBuildTargetMemberBytes = 256 * 1024 * 1024;

/// Exact cross-language presentation-label contract shared with the native
/// Voice planner. Keep the code-point tables explicit: runtime `trim` and
/// Unicode-control helpers are not a stable Rust/Dart wire contract.
bool _authoringRevision3VoiceBuildLineLabelIsSafe(String value) {
  if (value.isEmpty ||
      utf8.encode(value).length >
          _maxAuthoringRevision3VoiceBuildLineLabelBytes ||
      value.runes.any(_authoringRevision3VoiceControl)) {
    return false;
  }
  final runes = value.runes;
  return !_authoringRevision3VoiceBuildLineLabelBoundaryWhitespace(
        runes.first,
      ) &&
      !_authoringRevision3VoiceBuildLineLabelBoundaryWhitespace(runes.last);
}

/// Unicode White_Space plus the legacy zero-width no-break-space/BOM used by
/// common trim implementations. Rust enumerates this identical code-point set.
bool _authoringRevision3VoiceBuildLineLabelBoundaryWhitespace(int rune) =>
    (rune >= 0x0009 && rune <= 0x000d) ||
    rune == 0x0020 ||
    rune == 0x0085 ||
    rune == 0x00a0 ||
    rune == 0x1680 ||
    (rune >= 0x2000 && rune <= 0x200a) ||
    rune == 0x2028 ||
    rune == 0x2029 ||
    rune == 0x202f ||
    rune == 0x205f ||
    rune == 0x3000 ||
    rune == 0xfeff;

enum AuthoringRevision3VoiceBuildOutcome { blocked, built }

enum AuthoringRevision3VoiceBuildBlockReason {
  noVoiceSlots,
  voicePayloadBudgetExceeded,
  unresolvedTarget,
  ambiguousTarget,
  unqualifiedAdd,
  missingSelectedTake,
  selectedTakeNotApproved,
  selectedTakeCodecUnqualified,
  voiceSlotLimitExceeded,
}

final class AuthoringRevision3VoiceBuildBlocker {
  const AuthoringRevision3VoiceBuildBlocker({
    required this.slotId,
    required this.lineId,
    required this.lineLabel,
    required this.locId,
    required this.locale,
    required this.reason,
  });

  final String? slotId;
  final String? lineId;
  final String? lineLabel;
  final String? locId;
  final String? locale;
  final AuthoringRevision3VoiceBuildBlockReason reason;

  bool get isGlobal => slotId == null;

  factory AuthoringRevision3VoiceBuildBlocker._fromJson(Object? value) {
    final json = _authoringRequiredObject(
      value,
      'revision-3 Voice build blocker',
    );
    final reason = switch (_authoringRequiredString(
      json,
      'reason',
      maxBytes: 64,
    )) {
      'no_voice_slots' => AuthoringRevision3VoiceBuildBlockReason.noVoiceSlots,
      'voice_payload_budget_exceeded' =>
        AuthoringRevision3VoiceBuildBlockReason.voicePayloadBudgetExceeded,
      'unresolved_target' =>
        AuthoringRevision3VoiceBuildBlockReason.unresolvedTarget,
      'ambiguous_target' =>
        AuthoringRevision3VoiceBuildBlockReason.ambiguousTarget,
      'unqualified_add' =>
        AuthoringRevision3VoiceBuildBlockReason.unqualifiedAdd,
      'missing_selected_take' =>
        AuthoringRevision3VoiceBuildBlockReason.missingSelectedTake,
      'selected_take_not_approved' =>
        AuthoringRevision3VoiceBuildBlockReason.selectedTakeNotApproved,
      'selected_take_codec_unqualified' =>
        AuthoringRevision3VoiceBuildBlockReason.selectedTakeCodecUnqualified,
      'voice_slot_limit_exceeded' =>
        AuthoringRevision3VoiceBuildBlockReason.voiceSlotLimitExceeded,
      _ => throw const FormatException(
        'revision-3 Voice build blocker has an unknown reason',
      ),
    };
    final global =
        reason == AuthoringRevision3VoiceBuildBlockReason.noVoiceSlots ||
        reason ==
            AuthoringRevision3VoiceBuildBlockReason
                .voicePayloadBudgetExceeded ||
        reason ==
            AuthoringRevision3VoiceBuildBlockReason.voiceSlotLimitExceeded;
    _authoringExactFields(
      json,
      global
          ? const <String>{'reason'}
          : const <String>{
              'slot_id',
              'line_id',
              'line_label',
              'loc_id',
              'locale',
              'reason',
            },
      'revision-3 Voice build blocker',
    );
    if (global) {
      return AuthoringRevision3VoiceBuildBlocker(
        slotId: null,
        lineId: null,
        lineLabel: null,
        locId: null,
        locale: null,
        reason: reason,
      );
    }
    final slotId = _authoringEntityId(
      _authoringRequiredString(json, 'slot_id', maxBytes: 32),
      'slot_id',
    );
    final lineId = _authoringEntityId(
      _authoringRequiredString(json, 'line_id', maxBytes: 32),
      'line_id',
    );
    final lineLabel = _authoringRequiredString(
      json,
      'line_label',
      maxBytes: _maxAuthoringRevision3VoiceBuildLineLabelBytes,
    );
    final locId = _authoringRequiredString(
      json,
      'loc_id',
      maxBytes: _maxAuthoringRevision3VoiceTargetLocIdBytes,
    );
    final locale = _authoringRevision3VoiceLocale(
      _authoringRequiredString(json, 'locale', maxBytes: 35),
    );
    if (lineId == slotId ||
        !_authoringRevision3VoiceBuildLineLabelIsSafe(lineLabel) ||
        !authoringRevision3VoiceArchiveBasenameStemIsSafe(locId)) {
      throw const FormatException(
        'revision-3 Voice build blocker has invalid line facts',
      );
    }
    return AuthoringRevision3VoiceBuildBlocker(
      slotId: slotId,
      lineId: lineId,
      lineLabel: lineLabel,
      locId: locId,
      locale: locale,
      reason: reason,
    );
  }
}

final class AuthoringRevision3VoiceBuildReport {
  AuthoringRevision3VoiceBuildReport._({
    required this.projectId,
    required this.projectRevision,
    required this.totalSlots,
    required this.readySlots,
    required List<AuthoringRevision3VoiceBuildBlocker> blockers,
  }) : blockers = List.unmodifiable(blockers);

  final String projectId;
  final int projectRevision;
  final int totalSlots;
  final int readySlots;
  final List<AuthoringRevision3VoiceBuildBlocker> blockers;

  factory AuthoringRevision3VoiceBuildReport._fromJson(
    Object? value, {
    required _AuthoringRevision3VoiceBuildExpectation expectation,
  }) {
    final json = _authoringRequiredObject(
      value,
      'revision-3 Voice build report',
    );
    _authoringExactFields(json, const <String>{
      'project_id',
      'project_revision',
      'total_slots',
      'ready_slots',
      'blockers',
    }, 'revision-3 Voice build report');
    final projectId = _authoringEntityId(
      _authoringRequiredString(json, 'project_id', maxBytes: 32),
      'project_id',
    );
    final projectRevision = _authoringRequiredInt(
      json,
      'project_revision',
      max: _maxAuthoringRevision3VoiceAppliedRevision,
    );
    final totalSlots = _authoringRequiredInt(
      json,
      'total_slots',
      max: _maxAuthoringRevision3VoiceModelSlotCount,
    );
    final readySlots = _authoringRequiredInt(
      json,
      'ready_slots',
      max: totalSlots,
    );
    final rawBlockers = json['blockers'];
    if (rawBlockers is! List ||
        rawBlockers.isEmpty ||
        rawBlockers.length > _maxAuthoringRevision3VoiceBuildBlockers) {
      throw const FormatException(
        'revision-3 Voice build report has an invalid blocker list',
      );
    }
    final blockers = rawBlockers
        .map(AuthoringRevision3VoiceBuildBlocker._fromJson)
        .toList(growable: false);
    _requireExactBuildBlockerReport(
      totalSlots: totalSlots,
      readySlots: readySlots,
      blockers: blockers,
      expectation: expectation,
    );
    return AuthoringRevision3VoiceBuildReport._(
      projectId: projectId,
      projectRevision: projectRevision,
      totalSlots: totalSlots,
      readySlots: readySlots,
      blockers: blockers,
    );
  }
}

/// Result of evaluating the exact current managed Voice graph without creating
/// an output path, reading a game installation, or granting build authority.
enum AuthoringRevision3VoiceBuildPlanOutcome { ready, blocked }

/// Strict, basis-bound projection of the native read-only Voice build plan.
///
/// The native response is accepted only when every count and blocker agrees
/// with an independent bounded derivation from [expectedProjectJson]. A ready
/// result still grants no build or deployment authority; the write-capable
/// build command remains a separate explicit operation.
final class AuthoringRevision3VoiceBuildPlanResult {
  AuthoringRevision3VoiceBuildPlanResult._({
    required this.outcome,
    required this.basisHead,
    required this.projectId,
    required this.projectRevision,
    required this.totalSlots,
    required this.readySlots,
    required List<AuthoringRevision3VoiceBuildBlocker> blockers,
  }) : blockers = List.unmodifiable(blockers);

  final AuthoringRevision3VoiceBuildPlanOutcome outcome;
  final AuthoringWorkingHead basisHead;
  final String projectId;
  final int projectRevision;
  final int totalSlots;
  final int readySlots;
  final List<AuthoringRevision3VoiceBuildBlocker> blockers;

  bool get isReady => outcome == AuthoringRevision3VoiceBuildPlanOutcome.ready;

  factory AuthoringRevision3VoiceBuildPlanResult.fromJson(
    Map<String, Object?> json, {
    required AuthoringWorkingHead expectedHead,
    required String expectedProjectJson,
  }) {
    final expectation =
        _AuthoringRevision3VoiceBuildExpectation.fromCanonicalProjectJson(
          expectedProjectJson,
        );
    _authoringExactFields(json, const <String>{
      'ok',
      'outcome',
      'basis_head_json',
      'project_id',
      'project_revision',
      'total_slots',
      'ready_slots',
      'blockers',
      'plan_authority',
      'build_authority',
      'deployment_status',
    }, 'revision-3 Voice build plan response');
    if (json['ok'] != true) {
      throw const FormatException(
        'revision-3 Voice build plan response is not successful',
      );
    }
    final outcome = switch (json['outcome']) {
      'ready' => AuthoringRevision3VoiceBuildPlanOutcome.ready,
      'blocked' => AuthoringRevision3VoiceBuildPlanOutcome.blocked,
      _ => throw const FormatException(
        'revision-3 Voice build plan response has an unknown outcome',
      ),
    };
    final basisHead = AuthoringWorkingHead.fromCanonicalJson(
      _authoringRequiredString(
        json,
        'basis_head_json',
        maxBytes: _maxAuthoringHeadJsonBytes,
      ),
    );
    final projectId = _authoringEntityId(
      _authoringRequiredString(json, 'project_id', maxBytes: 32),
      'project_id',
    );
    final projectRevision = _authoringRequiredInt(
      json,
      'project_revision',
      max: _maxAuthoringRevision3VoiceAppliedRevision,
    );
    final totalSlots = _authoringRequiredInt(
      json,
      'total_slots',
      max: _maxAuthoringRevision3VoiceModelSlotCount,
    );
    final readySlots = _authoringRequiredInt(
      json,
      'ready_slots',
      max: totalSlots,
    );
    final rawBlockers = json['blockers'];
    if (rawBlockers is! List ||
        rawBlockers.length > _maxAuthoringRevision3VoiceBuildBlockers) {
      throw const FormatException(
        'revision-3 Voice build plan response has an invalid blocker list',
      );
    }
    final blockers = rawBlockers
        .map(AuthoringRevision3VoiceBuildBlocker._fromJson)
        .toList(growable: false);

    if (basisHead.canonicalJson != expectedHead.canonicalJson ||
        projectId != expectation.projectId ||
        projectRevision != expectation.projectRevision ||
        json['plan_authority'] != 'read_only_voice_build_plan_v1' ||
        json['build_authority'] != 'not_granted' ||
        json['deployment_status'] != 'not_performed') {
      throw const FormatException(
        'revision-3 Voice build plan response disagrees with its exact project basis or authority boundary',
      );
    }
    _requireExactBuildBlockerReport(
      totalSlots: totalSlots,
      readySlots: readySlots,
      blockers: blockers,
      expectation: expectation,
    );
    if (_authoringRevision3VoiceBuildPlanOutcomeIsReady(outcome) !=
            expectation.isReady ||
        (outcome == AuthoringRevision3VoiceBuildPlanOutcome.ready &&
            blockers.isNotEmpty) ||
        (outcome == AuthoringRevision3VoiceBuildPlanOutcome.blocked &&
            blockers.isEmpty)) {
      throw const FormatException(
        'revision-3 Voice build plan outcome disagrees with the exact project readiness',
      );
    }
    return AuthoringRevision3VoiceBuildPlanResult._(
      outcome: outcome,
      basisHead: basisHead,
      projectId: projectId,
      projectRevision: projectRevision,
      totalSlots: totalSlots,
      readySlots: readySlots,
      blockers: blockers,
    );
  }
}

bool _authoringRevision3VoiceBuildPlanOutcomeIsReady(
  AuthoringRevision3VoiceBuildPlanOutcome outcome,
) => outcome == AuthoringRevision3VoiceBuildPlanOutcome.ready;

void _requireExactBuildBlockerReport({
  required int totalSlots,
  required int readySlots,
  required List<AuthoringRevision3VoiceBuildBlocker> blockers,
  required _AuthoringRevision3VoiceBuildExpectation expectation,
}) {
  if (totalSlots != expectation.totalSlots ||
      readySlots != expectation.readySlots) {
    throw const FormatException(
      'revision-3 Voice build report disagrees with the exact project readiness counts',
    );
  }
  final actual = <String, int>{};
  for (final blocker in blockers) {
    actual.update(
      _authoringRevision3VoiceBuildBlockerKey(blocker),
      (count) => count + 1,
      ifAbsent: () => 1,
    );
  }
  final expected = <String, int>{};
  for (final blocker in expectation.blockers) {
    expected.update(
      _authoringRevision3VoiceBuildBlockerKey(blocker),
      (count) => count + 1,
      ifAbsent: () => 1,
    );
  }
  if (blockers.length != expectation.blockers.length ||
      !_authoringRevision3VoiceBuildStringIntMapsEqual(actual, expected)) {
    throw const FormatException(
      'revision-3 Voice build report disagrees with the exact project blocker multiset',
    );
  }
}

String _authoringRevision3VoiceBuildBlockerKey(
  AuthoringRevision3VoiceBuildBlocker blocker,
) =>
    '${blocker.slotId ?? ''}\u0000${blocker.lineId ?? ''}\u0000'
    '${blocker.lineLabel ?? ''}\u0000${blocker.locId ?? ''}\u0000'
    '${blocker.locale ?? ''}\u0000${blocker.reason.name}';

bool _authoringRevision3VoiceBuildStringIntMapsEqual(
  Map<String, int> left,
  Map<String, int> right,
) =>
    left.length == right.length &&
    left.entries.every((entry) => right[entry.key] == entry.value);

final class AuthoringRevision3VoiceBuildResult {
  const AuthoringRevision3VoiceBuildResult._({
    required this.outcome,
    required this.basisHead,
    required this.projectId,
    required this.projectRevision,
    required this.report,
    required this.output,
    required this.editCount,
    required this.fileCount,
    required this.bundleBytes,
    required this.bundleSha256,
  });

  final AuthoringRevision3VoiceBuildOutcome outcome;
  final AuthoringWorkingHead basisHead;
  final String projectId;
  final int projectRevision;
  final AuthoringRevision3VoiceBuildReport? report;
  final String? output;
  final int? editCount;
  final int? fileCount;
  final int? bundleBytes;
  final String? bundleSha256;

  bool get isBuilt => outcome == AuthoringRevision3VoiceBuildOutcome.built;

  factory AuthoringRevision3VoiceBuildResult.fromJson(
    Map<String, Object?> json, {
    required AuthoringWorkingHead expectedHead,
    required String expectedProjectJson,
    required String expectedOutput,
  }) {
    final expectation =
        _AuthoringRevision3VoiceBuildExpectation.fromCanonicalProjectJson(
          expectedProjectJson,
        );
    if (json['ok'] != true) {
      throw const FormatException(
        'revision-3 Voice build response is not successful',
      );
    }
    final outcome = switch (json['outcome']) {
      'blocked' => AuthoringRevision3VoiceBuildOutcome.blocked,
      'built' => AuthoringRevision3VoiceBuildOutcome.built,
      _ => throw const FormatException(
        'revision-3 Voice build response has an unknown outcome',
      ),
    };
    final built = outcome == AuthoringRevision3VoiceBuildOutcome.built;
    final responseFields = <String>{
      'ok',
      'outcome',
      'basis_head_json',
      'project_id',
      'project_revision',
      'build_authority',
      'deployment_status',
    };
    if (built) {
      responseFields.addAll(const <String>{
        'output',
        'edit_count',
        'file_count',
        'bundle_bytes',
        'bundle_sha256',
      });
    } else {
      responseFields.add('report');
    }
    _authoringExactFields(
      json,
      responseFields,
      'revision-3 Voice build response',
    );
    final basisHead = AuthoringWorkingHead.fromCanonicalJson(
      _authoringRequiredString(
        json,
        'basis_head_json',
        maxBytes: _maxAuthoringHeadJsonBytes,
      ),
    );
    final projectId = _authoringEntityId(
      _authoringRequiredString(json, 'project_id', maxBytes: 32),
      'project_id',
    );
    final projectRevision = _authoringRequiredInt(
      json,
      'project_revision',
      max: _maxAuthoringRevision3VoiceAppliedRevision,
    );
    if (basisHead.canonicalJson != expectedHead.canonicalJson ||
        projectId != expectation.projectId ||
        projectRevision != expectation.projectRevision ||
        json['deployment_status'] != 'not_performed') {
      throw const FormatException(
        'revision-3 Voice build response disagrees with its exact project basis',
      );
    }

    if (!built) {
      if (json['build_authority'] != 'not_granted') {
        throw const FormatException(
          'blocked revision-3 Voice build unexpectedly grants build authority',
        );
      }
      final report = AuthoringRevision3VoiceBuildReport._fromJson(
        json['report'],
        expectation: expectation,
      );
      if (report.projectId != projectId ||
          report.projectRevision != projectRevision) {
        throw const FormatException(
          'revision-3 Voice build report disagrees with its project basis',
        );
      }
      return AuthoringRevision3VoiceBuildResult._(
        outcome: outcome,
        basisHead: basisHead,
        projectId: projectId,
        projectRevision: projectRevision,
        report: report,
        output: null,
        editCount: null,
        fileCount: null,
        bundleBytes: null,
        bundleSha256: null,
      );
    }

    if (json['build_authority'] !=
        'generation_sealed_existing_member_bundle_v1') {
      throw const FormatException(
        'built revision-3 Voice response has invalid build authority',
      );
    }
    final output = _authoringRequiredString(
      json,
      'output',
      maxBytes: _maxAuthoringStorePathBytes,
    );
    final editCount = _authoringRequiredInt(
      json,
      'edit_count',
      min: 1,
      max: _maxAuthoringRevision3VoiceBuildCount,
    );
    final fileCount = _authoringRequiredInt(
      json,
      'file_count',
      min: 3,
      max: _maxAuthoringRevision3VoiceBuildCount + 2,
    );
    final bundleBytes = _authoringRequiredInt(
      json,
      'bundle_bytes',
      min: 1,
      max: _maxAuthoringRevision3VoiceBuildBytes,
    );
    final bundleSha256 = _authoringRequiredString(
      json,
      'bundle_sha256',
      maxBytes: 64,
    );
    if (output != expectedOutput ||
        !expectation.isReady ||
        editCount != expectation.totalSlots ||
        fileCount != editCount + 2 ||
        !_authoringSha256Pattern.hasMatch(bundleSha256)) {
      throw const FormatException(
        'built revision-3 Voice response has an invalid bundle receipt',
      );
    }
    return AuthoringRevision3VoiceBuildResult._(
      outcome: outcome,
      basisHead: basisHead,
      projectId: projectId,
      projectRevision: projectRevision,
      report: null,
      output: output,
      editCount: editCount,
      fileCount: fileCount,
      bundleBytes: bundleBytes,
      bundleSha256: bundleSha256,
    );
  }
}

final class _AuthoringRevision3VoiceBuildSlotFacts {
  const _AuthoringRevision3VoiceBuildSlotFacts({
    required this.slotId,
    required this.lineId,
    required this.lineLabel,
    required this.locId,
    required this.locale,
  });

  final String slotId;
  final String lineId;
  final String lineLabel;
  final String locId;
  final String locale;
}

/// Bounded exact projection of the current project facts that native build
/// receipts are allowed to report.
///
/// This deliberately derives from the caller-bound canonical project rather
/// than trusting self-consistent counts or labels returned by native code.
final class _AuthoringRevision3VoiceBuildExpectation {
  _AuthoringRevision3VoiceBuildExpectation._({
    required this.projectId,
    required this.projectRevision,
    required this.totalSlots,
    required Map<String, _AuthoringRevision3VoiceBuildSlotFacts> factsBySlot,
    required this.readySlots,
    required List<AuthoringRevision3VoiceBuildBlocker> blockers,
  }) : factsBySlot = Map.unmodifiable(factsBySlot),
       blockers = List.unmodifiable(blockers);

  final String projectId;
  final int projectRevision;
  final int totalSlots;
  final Map<String, _AuthoringRevision3VoiceBuildSlotFacts> factsBySlot;
  final int readySlots;
  final List<AuthoringRevision3VoiceBuildBlocker> blockers;

  bool get isReady =>
      totalSlots > 0 && blockers.isEmpty && readySlots == totalSlots;

  factory _AuthoringRevision3VoiceBuildExpectation.fromCanonicalProjectJson(
    String projectJson,
  ) {
    if (utf8.encode(projectJson).length > _maxAuthoringProjectJsonBytes) {
      throw const FormatException(
        'revision-3 Voice build project exceeds the bounded JSON limit',
      );
    }
    final current = _authoringRequireCanonicalRevision3ProjectJson(projectJson);
    final entities = _authoringRequiredObject(
      current.project['entities'],
      'revision-3 Voice build project entities',
    );
    if (entities.length > _maxAuthoringRevision3VoiceModelSlotCount) {
      throw const FormatException(
        'revision-3 Voice build project exceeds the bounded entity limit',
      );
    }

    final voiceSlots = <String, _AuthoringRevision3VoiceBuildSlotModel>{};
    final dialogLineIds = <String>[];
    for (final entry in entities.entries) {
      final entityId = _authoringRevision3VoiceBuildEntityId(
        entry.key,
        'project entity ID',
      );
      final entity = _authoringRequiredObject(
        entry.value,
        'revision-3 Voice build project entity',
      );
      final payload = _authoringRequiredObject(
        entity['payload'],
        'revision-3 Voice build project entity payload',
      );
      final kind = _authoringRequiredString(payload, 'kind', maxBytes: 64);
      if (kind == 'dialog_line') {
        dialogLineIds.add(entityId);
        continue;
      }
      if (kind != 'voice_slot') continue;

      final slot = _authoringRevision3VoiceEntity(
        entities,
        entityId,
        'voice_slot',
        'build expectation slot',
      );
      _authoringRevision3VoiceExactOptionalFields(
        slot.data,
        const {'locale', 'target_resolution', 'candidates'},
        const {'selected'},
        'build expectation VoiceSlot data',
      );
      final locale = _authoringRevision3VoiceLocale(
        _authoringRequiredString(slot.data, 'locale', maxBytes: 35),
      );
      voiceSlots[entityId] = _AuthoringRevision3VoiceBuildSlotModel(
        id: entityId,
        locale: locale,
        data: slot.data,
      );
    }

    // Native validates the closed model, counts VoiceSlots, and applies the
    // hard slot cap before deriving any presentation or build facts. Mirror
    // that order exactly: an over-cap project always gets the one bounded
    // global blocker, even if a line label would be invalid for a buildable
    // project. This also avoids an unnecessary O(lines + ownership) pass.
    if (voiceSlots.length > _maxAuthoringRevision3VoiceBuildCount) {
      return _AuthoringRevision3VoiceBuildExpectation._(
        projectId: current.projectId,
        projectRevision: current.revision,
        totalSlots: voiceSlots.length,
        factsBySlot: const <String, _AuthoringRevision3VoiceBuildSlotFacts>{},
        readySlots: 0,
        blockers: <AuthoringRevision3VoiceBuildBlocker>[
          _authoringRevision3VoiceBuildGlobalBlocker(
            AuthoringRevision3VoiceBuildBlockReason.voiceSlotLimitExceeded,
          ),
        ],
      );
    }

    final factsBySlot = <String, _AuthoringRevision3VoiceBuildSlotFacts>{};
    for (final lineId in dialogLineIds) {
      final line = _authoringRevision3VoiceEntity(
        entities,
        lineId,
        'dialog_line',
        'build expectation line',
      );
      _authoringRevision3VoiceExactOptionalFields(
        line.data,
        const {'localization', 'voice_slots'},
        const {'speaker_hint'},
        'build expectation DialogLine data',
      );
      final slots = _authoringRequiredObject(
        line.data['voice_slots'],
        'revision-3 Voice build expectation line slots',
      );
      if (slots.isEmpty) continue;

      final lineLabel = _authoringRequiredString(
        line.entity,
        'display_name',
        maxBytes: _maxAuthoringRevision3VoiceBuildLineLabelBytes,
      );
      if (!_authoringRevision3VoiceBuildLineLabelIsSafe(lineLabel)) {
        throw const FormatException(
          'revision-3 Voice build project has an invalid line label',
        );
      }
      final localizationRef = _authoringRevision3VoiceTypedRef(
        line.data['localization'],
        projectId: current.projectId,
        kind: 'localization_entry',
        context: 'build expectation line localization',
      );
      final localization = _authoringRevision3VoiceEntity(
        entities,
        localizationRef.id,
        'localization_entry',
        'build expectation localization',
      );
      _authoringExactFields(localization.data, const {
        'loc_id',
        'texts',
      }, 'revision-3 Voice build expectation localization data');
      final locId = _authoringRequiredString(
        localization.data,
        'loc_id',
        maxBytes: _maxAuthoringRevision3VoiceTargetLocIdBytes,
      );
      if (!authoringRevision3VoiceArchiveBasenameStemIsSafe(locId)) {
        throw const FormatException(
          'revision-3 Voice build project has an invalid Voice LocID stem',
        );
      }

      for (final owned in slots.entries) {
        final locale = _authoringRevision3VoiceLocale(owned.key);
        final slotRef = _authoringRevision3VoiceTypedRef(
          owned.value,
          projectId: current.projectId,
          kind: 'voice_slot',
          context: 'build expectation line slot',
        );
        if (voiceSlots[slotRef.id]?.locale != locale ||
            factsBySlot.containsKey(slotRef.id)) {
          throw const FormatException(
            'revision-3 Voice build project has invalid VoiceSlot ownership',
          );
        }
        factsBySlot[slotRef.id] = _AuthoringRevision3VoiceBuildSlotFacts(
          slotId: slotRef.id,
          lineId: lineId,
          lineLabel: lineLabel,
          locId: locId,
          locale: locale,
        );
      }
    }
    if (factsBySlot.length != voiceSlots.length ||
        !voiceSlots.keys.every(factsBySlot.containsKey)) {
      throw const FormatException(
        'revision-3 Voice build project has an unowned VoiceSlot',
      );
    }

    final assetStore = _authoringRequiredObject(
      current.project['asset_store'],
      'revision-3 Voice build project asset Store',
    );
    _authoringExactFields(assetStore, const {
      'assets',
    }, 'revision-3 Voice build project asset Store');
    final assets = _authoringRequiredObject(
      assetStore['assets'],
      'revision-3 Voice build project assets',
    );
    if (assets.length > _maxAuthoringRevision3VoiceModelSlotCount) {
      throw const FormatException(
        'revision-3 Voice build project exceeds the bounded asset limit',
      );
    }

    final takeFacts = <String, _AuthoringRevision3VoiceBuildTakeFacts>{};
    final blockers = <AuthoringRevision3VoiceBuildBlocker>[];
    final deploymentTargets = <String>{};
    var readySlots = 0;
    var selectedPayloadBytes = 0;
    for (final slotEntry in voiceSlots.entries) {
      final slot = slotEntry.value;
      final facts = factsBySlot[slot.id]!;
      final candidates = slot.data['candidates'];
      if (candidates is! List ||
          candidates.length > _maxAuthoringRevision3VoiceSlotCandidates) {
        throw const FormatException(
          'revision-3 Voice build slot has an invalid candidate list',
        );
      }
      final candidateIds = <String>{};
      for (var index = 0; index < candidates.length; index++) {
        final candidate = _authoringRevision3VoiceTypedRef(
          candidates[index],
          projectId: current.projectId,
          kind: 'voice_take',
          context: 'build expectation slot candidate $index',
        );
        if (!candidateIds.add(candidate.id)) {
          throw const FormatException(
            'revision-3 Voice build slot contains duplicate candidates',
          );
        }
        final take = takeFacts.putIfAbsent(
          candidate.id,
          () => _authoringRevision3VoiceBuildTakeFacts(
            entities,
            assets,
            candidate.id,
          ),
        );
        if (take.locale != slot.locale) {
          throw const FormatException(
            'revision-3 Voice build candidate locale disagrees with its slot',
          );
        }
      }

      final resolution = _authoringRevision3VoiceBuildTargetResolution(
        slot.data['target_resolution'],
      );
      final AuthoringRevision3VoiceBuildBlockReason? targetReason =
          switch (resolution.state) {
            _AuthoringRevision3VoiceBuildTargetState.unresolved =>
              AuthoringRevision3VoiceBuildBlockReason.unresolvedTarget,
            _AuthoringRevision3VoiceBuildTargetState.ambiguous =>
              AuthoringRevision3VoiceBuildBlockReason.ambiguousTarget,
            _AuthoringRevision3VoiceBuildTargetState.resolved
                when !resolution.qualified =>
              AuthoringRevision3VoiceBuildBlockReason.unqualifiedAdd,
            _ => null,
          };
      if (targetReason != null) {
        blockers.add(
          _authoringRevision3VoiceBuildSlotBlocker(facts, targetReason),
        );
      }

      String? selectedId;
      final selected = slot.data['selected'];
      if (selected == null) {
        blockers.add(
          _authoringRevision3VoiceBuildSlotBlocker(
            facts,
            AuthoringRevision3VoiceBuildBlockReason.missingSelectedTake,
          ),
        );
      } else {
        selectedId = _authoringRevision3VoiceTypedRef(
          selected,
          projectId: current.projectId,
          kind: 'voice_take',
          context: 'build expectation selected take',
        ).id;
        if (!candidateIds.contains(selectedId)) {
          throw const FormatException(
            'revision-3 Voice build selected take is not a slot candidate',
          );
        }
      }

      if (targetReason != null || selectedId == null) continue;
      final selectedTake = takeFacts[selectedId]!;
      if (selectedTake.status != AuthoringRevision3VoiceTakeStatus.approved) {
        blockers.add(
          _authoringRevision3VoiceBuildSlotBlocker(
            facts,
            AuthoringRevision3VoiceBuildBlockReason.selectedTakeNotApproved,
          ),
        );
        continue;
      }
      if (selectedTake.codec != AuthoringRevision3VoiceOggCodec.vorbis) {
        blockers.add(
          _authoringRevision3VoiceBuildSlotBlocker(
            facts,
            AuthoringRevision3VoiceBuildBlockReason
                .selectedTakeCodecUnqualified,
          ),
        );
        continue;
      }
      final deploymentKey = resolution.deploymentKey!;
      if (!deploymentTargets.add(deploymentKey)) {
        throw const FormatException(
          'revision-3 Voice build project repeats a deployment target',
        );
      }
      final nextPayloadBytes = selectedPayloadBytes + selectedTake.assetBytes;
      if (nextPayloadBytes >
          _maxAuthoringRevision3VoiceBuildSelectedPayloadBytes) {
        return _AuthoringRevision3VoiceBuildExpectation._(
          projectId: current.projectId,
          projectRevision: current.revision,
          totalSlots: voiceSlots.length,
          factsBySlot: factsBySlot,
          readySlots: 0,
          blockers: <AuthoringRevision3VoiceBuildBlocker>[
            _authoringRevision3VoiceBuildGlobalBlocker(
              AuthoringRevision3VoiceBuildBlockReason
                  .voicePayloadBudgetExceeded,
            ),
          ],
        );
      }
      selectedPayloadBytes = nextPayloadBytes;
      readySlots++;
    }

    if (voiceSlots.isEmpty) {
      blockers.add(
        _authoringRevision3VoiceBuildGlobalBlocker(
          AuthoringRevision3VoiceBuildBlockReason.noVoiceSlots,
        ),
      );
    }
    return _AuthoringRevision3VoiceBuildExpectation._(
      projectId: current.projectId,
      projectRevision: current.revision,
      totalSlots: voiceSlots.length,
      factsBySlot: factsBySlot,
      readySlots: readySlots,
      blockers: blockers,
    );
  }
}

final class _AuthoringRevision3VoiceBuildSlotModel {
  const _AuthoringRevision3VoiceBuildSlotModel({
    required this.id,
    required this.locale,
    required this.data,
  });

  final String id;
  final String locale;
  final Map<String, Object?> data;
}

final class _AuthoringRevision3VoiceBuildTakeFacts {
  const _AuthoringRevision3VoiceBuildTakeFacts({
    required this.locale,
    required this.status,
    required this.codec,
    required this.assetBytes,
  });

  final String locale;
  final AuthoringRevision3VoiceTakeStatus status;
  final AuthoringRevision3VoiceOggCodec codec;
  final int assetBytes;
}

_AuthoringRevision3VoiceBuildTakeFacts _authoringRevision3VoiceBuildTakeFacts(
  Map<String, Object?> entities,
  Map<String, Object?> assets,
  String takeId,
) {
  final take = _authoringRevision3VoiceEntity(
    entities,
    takeId,
    'voice_take',
    'build expectation take',
  );
  _authoringExactFields(take.data, const {
    'locale',
    'asset',
    'ogg',
    'status',
  }, 'revision-3 Voice build expectation take data');
  final locale = _authoringRevision3VoiceLocale(
    _authoringRequiredString(take.data, 'locale', maxBytes: 35),
  );
  final asset = _authoringRequiredObject(
    take.data['asset'],
    'revision-3 Voice build selected asset',
  );
  _authoringExactFields(asset, const {
    'sha256',
    'byte_len',
    'logical_name',
  }, 'revision-3 Voice build selected asset');
  final sha256 = _authoringRequiredString(asset, 'sha256', maxBytes: 64);
  final logicalName = _authoringRequiredString(
    asset,
    'logical_name',
    maxBytes: _maxAuthoringRevision3VoiceLogicalNameBytes,
  );
  if (!_authoringSha256Pattern.hasMatch(sha256) ||
      sha256 == _authoringRevision3VoiceZeroSha256 ||
      !_authoringRevision3VoiceLogicalNameIsSafe(logicalName)) {
    throw const FormatException(
      'revision-3 Voice build selected asset is invalid',
    );
  }
  final assetBytes = _authoringRequiredInt(
    asset,
    'byte_len',
    min: 1,
    max: _maxAuthoringRevision3VoiceBuildBytes,
  );
  final meta = _authoringRequiredObject(
    assets[sha256],
    'revision-3 Voice build selected asset metadata',
  );
  _authoringExactFields(meta, const {
    'byte_len',
    'media_type',
  }, 'revision-3 Voice build selected asset metadata');
  if (_authoringRequiredInt(
            meta,
            'byte_len',
            min: 1,
            max: _maxAuthoringRevision3VoiceBuildBytes,
          ) !=
          assetBytes ||
      meta['media_type'] != 'audio/ogg') {
    throw const FormatException(
      'revision-3 Voice build selected asset metadata is not exact audio/ogg',
    );
  }
  final ogg = _authoringRevision3VoiceOgg(take.data['ogg']);
  return _AuthoringRevision3VoiceBuildTakeFacts(
    locale: locale,
    status: _authoringRevision3VoiceStatus(take.data['status']),
    codec: ogg.codec,
    assetBytes: assetBytes,
  );
}

enum _AuthoringRevision3VoiceBuildTargetState {
  unresolved,
  ambiguous,
  resolved,
}

({
  _AuthoringRevision3VoiceBuildTargetState state,
  bool qualified,
  String? deploymentKey,
})
_authoringRevision3VoiceBuildTargetResolution(Object? value) {
  final resolution = _authoringRequiredObject(
    value,
    'revision-3 Voice build target resolution',
  );
  switch (resolution['state']) {
    case 'unresolved':
      _authoringExactFields(resolution, const {
        'state',
      }, 'revision-3 Voice build target resolution');
      return (
        state: _AuthoringRevision3VoiceBuildTargetState.unresolved,
        qualified: false,
        deploymentKey: null,
      );
    case 'ambiguous':
      _authoringExactFields(resolution, const {
        'state',
        'candidates',
      }, 'revision-3 Voice build target resolution');
      final raw = resolution['candidates'];
      if (raw is! List ||
          raw.length < 2 ||
          raw.length > _maxAuthoringRevision3VoiceTargetMatches) {
        throw const FormatException(
          'revision-3 Voice build ambiguous target cardinality is invalid',
        );
      }
      final keys = <String>{};
      for (var index = 0; index < raw.length; index++) {
        final target = _authoringRevision3VoiceBuildTarget(
          raw[index],
          context: 'revision-3 Voice build ambiguous target $index',
        );
        if (!target.qualified || !keys.add(target.deploymentKey)) {
          throw const FormatException(
            'revision-3 Voice build ambiguous targets are invalid or duplicated',
          );
        }
      }
      return (
        state: _AuthoringRevision3VoiceBuildTargetState.ambiguous,
        qualified: false,
        deploymentKey: null,
      );
    case 'resolved':
      _authoringExactFields(resolution, const {
        'state',
        'target',
      }, 'revision-3 Voice build target resolution');
      final target = _authoringRevision3VoiceBuildTarget(
        resolution['target'],
        context: 'revision-3 Voice build resolved target',
      );
      return (
        state: _AuthoringRevision3VoiceBuildTargetState.resolved,
        qualified: target.qualified,
        deploymentKey: target.qualified ? target.deploymentKey : null,
      );
    default:
      throw const FormatException(
        'revision-3 Voice build target resolution state is invalid',
      );
  }
}

({bool qualified, String deploymentKey}) _authoringRevision3VoiceBuildTarget(
  Object? value, {
  required String context,
}) {
  final target = _authoringRequiredObject(value, context);
  _authoringExactFields(target, const {
    'archive',
    'member',
    'operation',
    'archive_seal',
    'member_proof',
  }, context);
  final archive = _authoringRequiredString(
    target,
    'archive',
    maxBytes: _maxAuthoringRevision3VoiceTargetArchiveBytes,
  );
  final member = _authoringRequiredString(
    target,
    'member',
    maxBytes: _maxAuthoringRevision3VoiceTargetMemberBytes,
  );
  if (!_authoringRevision3VoiceTargetArchiveIsSafe(archive) ||
      !_authoringRevision3VoiceTargetMemberIsSafe(member)) {
    throw FormatException('$context path is invalid');
  }
  final operation = target['operation'];
  if (operation != 'add' && operation != 'replace') {
    throw FormatException('$context operation is invalid');
  }
  final archiveSeal = _authoringRequiredObject(
    target['archive_seal'],
    '$context archive seal',
  );
  _authoringExactFields(archiveSeal, const {
    'byte_len',
    'sha256',
  }, '$context archive seal');
  final archiveSha256 = _authoringRequiredString(
    archiveSeal,
    'sha256',
    maxBytes: 64,
  );
  if (_authoringRequiredInt(
            archiveSeal,
            'byte_len',
            min: 1,
            max: _maxAuthoringRevision3VoiceBuildTargetArchiveBytes,
          ) <
          1 ||
      !_authoringSha256Pattern.hasMatch(archiveSha256) ||
      archiveSha256 == _authoringRevision3VoiceZeroSha256) {
    throw FormatException('$context archive seal is invalid');
  }
  final proof = _authoringRequiredObject(
    target['member_proof'],
    '$context member proof',
  );
  var present = false;
  var uncompressedSize = 0;
  switch (proof['state']) {
    case 'absent':
      _authoringExactFields(proof, const {'state'}, '$context member proof');
    case 'present':
      _authoringExactFields(proof, const {
        'state',
        'uncompressed_size',
        'crc32',
      }, '$context member proof');
      present = true;
      uncompressedSize = _authoringRequiredInt(
        proof,
        'uncompressed_size',
        max: _maxAuthoringRevision3VoiceBuildTargetMemberBytes,
      );
      _authoringRequiredInt(proof, 'crc32', max: 0xffffffff);
    default:
      throw FormatException('$context member proof state is invalid');
  }
  return (
    qualified: operation == 'replace' && present && uncompressedSize > 0,
    deploymentKey:
        '${archive.replaceAll(r'\', '/').toLowerCase()}|'
        '${member.replaceAll(r'\', '/').toLowerCase()}',
  );
}

AuthoringRevision3VoiceBuildBlocker _authoringRevision3VoiceBuildSlotBlocker(
  _AuthoringRevision3VoiceBuildSlotFacts facts,
  AuthoringRevision3VoiceBuildBlockReason reason,
) => AuthoringRevision3VoiceBuildBlocker(
  slotId: facts.slotId,
  lineId: facts.lineId,
  lineLabel: facts.lineLabel,
  locId: facts.locId,
  locale: facts.locale,
  reason: reason,
);

AuthoringRevision3VoiceBuildBlocker _authoringRevision3VoiceBuildGlobalBlocker(
  AuthoringRevision3VoiceBuildBlockReason reason,
) => AuthoringRevision3VoiceBuildBlocker(
  slotId: null,
  lineId: null,
  lineLabel: null,
  locId: null,
  locale: null,
  reason: reason,
);

String _authoringRevision3VoiceBuildEntityId(String value, String context) {
  final id = _authoringEntityId(value, context);
  if (id == '00000000000000000000000000000000') {
    throw FormatException('revision-3 Voice build $context must not be zero');
  }
  return id;
}
