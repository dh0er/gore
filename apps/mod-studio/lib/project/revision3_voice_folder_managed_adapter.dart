import 'dart:convert';

import 'package:crypto/crypto.dart' as crypto;
import 'package:path/path.dart' as p;

import '../core/mod_ffi.dart';
import 'managed_project_session.dart';
import 'revision3_content_index.dart';
import 'revision3_voice_folder_authoring.dart';

typedef Revision3VoiceFolderNativePlanner =
    Future<AuthoringRevision3VoiceBatchPlanResult> Function({
      required String sourceFolder,
      required String locale,
    });

typedef Revision3VoiceFolderContentIndexLoader =
    Future<Revision3ContentIndex> Function();

typedef Revision3VoiceFolderNativePublisher =
    Future<ManagedRevision3VoiceBatchCheckpoint> Function({
      required String sourceFolder,
      required AuthoringRevision3VoiceBatchPlanResult plan,
    });

/// Exact-current bridge between the native atomic batch contract and the
/// presentation-safe Voice-folder review model.
///
/// Absolute paths, entity IDs, LocIDs, seals, and native request authority stay
/// behind this boundary. The dialog receives only friendly leaf labels and an
/// opaque plan token. At most one reviewed plan remains applicable.
final class Revision3VoiceFolderManagedAdapter {
  factory Revision3VoiceFolderManagedAdapter({
    required String expectedProjectId,
    required int expectedProjectRevision,
    required String expectedProjectHead,
    required Revision3VoiceFolderContentIndexLoader loadContentIndex,
    required Revision3VoiceFolderNativePlanner planNative,
    required Revision3VoiceFolderNativePublisher publishNative,
  }) => Revision3VoiceFolderManagedAdapter._(
    expectedProjectId: expectedProjectId,
    expectedProjectRevision: expectedProjectRevision,
    expectedProjectHead: expectedProjectHead,
    expectedCheckpointToken: checkpointTokenForHead(expectedProjectHead),
    loadContentIndex: loadContentIndex,
    planNative: planNative,
    publishNative: publishNative,
  );

  Revision3VoiceFolderManagedAdapter._({
    required this.expectedProjectId,
    required this.expectedProjectRevision,
    required this.expectedProjectHead,
    required this.expectedCheckpointToken,
    required this._loadContentIndex,
    required this._planNative,
    required this._publishNative,
  });

  final String expectedProjectId;
  final int expectedProjectRevision;
  final String expectedProjectHead;
  final String expectedCheckpointToken;
  final Revision3VoiceFolderContentIndexLoader _loadContentIndex;
  final Revision3VoiceFolderNativePlanner _planNative;
  final Revision3VoiceFolderNativePublisher _publishNative;

  _Revision3VoiceFolderPlanAuthority? _authority;
  int _planEpoch = 0;
  bool _publicationInFlight = false;

  Revision3VoiceFolderAuthoringService get service =>
      Revision3VoiceFolderAuthoringService(
        planFolder: _plan,
        applyPlan: _apply,
      );

  static String checkpointTokenForHead(String canonicalHead) =>
      crypto.sha256.convert(utf8.encode(canonicalHead)).toString();

  Future<Revision3VoiceFolderImportPlan> _plan(
    Revision3VoiceFolderPlanRequest request,
  ) async {
    _requireExactRequest(request);
    if (_publicationInFlight) {
      throw const Revision3VoiceFolderStaleCheckpointException();
    }
    final invocationEpoch = ++_planEpoch;
    _authority = null;
    try {
      final native = await _planNative(
        sourceFolder: request.folderPath,
        locale: request.locale,
      );
      _requireCurrentPlanInvocation(invocationEpoch);
      if (native.projectId != expectedProjectId ||
          native.revision != expectedProjectRevision ||
          native.basisHead.canonicalJson != expectedProjectHead ||
          native.locale != request.locale) {
        throw const Revision3VoiceFolderRequiresReopenException();
      }
      final index = await _loadContentIndex();
      _requireCurrentPlanInvocation(invocationEpoch);
      if (index.projectId != expectedProjectId ||
          index.projectRevision != expectedProjectRevision) {
        throw const Revision3VoiceFolderStaleCheckpointException();
      }
      final rows = <Revision3VoiceFolderReviewRow>[];
      for (var ordinal = 0; ordinal < native.items.length; ordinal++) {
        rows.add(_presentationRow(native, index, ordinal));
      }
      final presentation = Revision3VoiceFolderImportPlan(
        projectId: expectedProjectId,
        projectRevision: expectedProjectRevision,
        projectHead: expectedProjectHead,
        checkpointToken: expectedCheckpointToken,
        planToken: native.planSha256,
        folderLabel: _friendlyFolderLabel(request.folderPath, native),
        locale: native.locale,
        scannedEntryCount: native.scannedEntryCount,
        ignoredEntryCount: native.ignoredEntryCount,
        rows: rows,
      );
      if (presentation.counts.ready != native.readyCount ||
          presentation.counts.alreadyPresent != native.alreadyPresentCount ||
          presentation.counts.blocked != native.blockedCount ||
          presentation.canApply != native.canPrepare) {
        throw const Revision3VoiceFolderRequiresReopenException();
      }
      _requireCurrentPlanInvocation(invocationEpoch);
      _authority = _Revision3VoiceFolderPlanAuthority(
        sourceFolder: request.folderPath,
        native: native,
        presentation: presentation,
      );
      return presentation;
    } catch (error, stackTrace) {
      _translateFailure(error, stackTrace, applying: false);
    }
  }

  Future<Revision3VoiceFolderImportPublication> _apply({
    required Revision3VoiceFolderImportPlan plan,
  }) async {
    final authority = _authority;
    if (_publicationInFlight ||
        authority == null ||
        !identical(authority.presentation, plan) ||
        plan.projectId != expectedProjectId ||
        plan.projectRevision != expectedProjectRevision ||
        plan.projectHead != expectedProjectHead ||
        plan.checkpointToken != expectedCheckpointToken ||
        plan.planToken != authority.native.planSha256 ||
        !plan.canApply ||
        !authority.native.canPrepare) {
      throw const Revision3VoiceFolderStaleCheckpointException();
    }
    _publicationInFlight = true;
    ++_planEpoch;
    _authority = null;
    try {
      final checkpoint = await _publishNative(
        sourceFolder: authority.sourceFolder,
        plan: authority.native,
      );
      if (checkpoint.projectId != expectedProjectId ||
          checkpoint.projectRevision != expectedProjectRevision + 1 ||
          checkpoint.locale != authority.native.locale ||
          checkpoint.sourceManifestSha256 !=
              authority.native.sourceManifestSha256 ||
          checkpoint.planSha256 != authority.native.planSha256 ||
          checkpoint.importedCount != authority.native.readyCount ||
          checkpoint.alreadyPresentCount !=
              authority.native.alreadyPresentCount ||
          checkpoint.head.canonicalJson == expectedProjectHead) {
        throw const Revision3VoiceFolderRequiresReopenException();
      }
      return Revision3VoiceFolderImportPublication(
        projectId: checkpoint.projectId,
        projectRevision: checkpoint.projectRevision,
        projectHead: checkpoint.head.canonicalJson,
        checkpointToken: checkpointTokenForHead(checkpoint.head.canonicalJson),
        planToken: checkpoint.planSha256,
        importedCount: checkpoint.importedCount,
      );
    } catch (error, stackTrace) {
      _translateFailure(error, stackTrace, applying: true);
    } finally {
      _publicationInFlight = false;
    }
  }

  void _requireCurrentPlanInvocation(int invocationEpoch) {
    if (_publicationInFlight || invocationEpoch != _planEpoch) {
      throw const Revision3VoiceFolderStaleCheckpointException();
    }
  }

  void _requireExactRequest(Revision3VoiceFolderPlanRequest request) {
    if (request.expectedProjectId != expectedProjectId ||
        request.expectedProjectRevision != expectedProjectRevision ||
        request.expectedProjectHead != expectedProjectHead ||
        request.expectedCheckpointToken != expectedCheckpointToken) {
      throw const Revision3VoiceFolderStaleCheckpointException();
    }
  }
}

final class _Revision3VoiceFolderPlanAuthority {
  const _Revision3VoiceFolderPlanAuthority({
    required this.sourceFolder,
    required this.native,
    required this.presentation,
  });

  final String sourceFolder;
  final AuthoringRevision3VoiceBatchPlanResult native;
  final Revision3VoiceFolderImportPlan presentation;
}

Revision3VoiceFolderReviewRow _presentationRow(
  AuthoringRevision3VoiceBatchPlanResult plan,
  Revision3ContentIndex index,
  int ordinal,
) {
  final item = plan.items[ordinal];
  final status = switch (item.status) {
    AuthoringRevision3VoiceBatchItemStatus.ready =>
      Revision3VoiceFolderRowStatus.ready,
    AuthoringRevision3VoiceBatchItemStatus.alreadyPresent =>
      Revision3VoiceFolderRowStatus.alreadyPresent,
    AuthoringRevision3VoiceBatchItemStatus.unmatched =>
      Revision3VoiceFolderRowStatus.unmatched,
    AuthoringRevision3VoiceBatchItemStatus.ambiguous =>
      Revision3VoiceFolderRowStatus.ambiguous,
    _ => Revision3VoiceFolderRowStatus.invalid,
  };
  final codec = switch (item.ogg?.codec) {
    AuthoringRevision3VoiceOggCodec.vorbis => Revision3VoiceFolderCodec.vorbis,
    AuthoringRevision3VoiceOggCodec.opus => Revision3VoiceFolderCodec.opus,
    null => null,
  };

  int? beforeTakeCount;
  int? afterTakeCount;
  Revision3VoiceFolderTargetState? targetState;
  String? lineLabel;
  String? speakerLabel;
  String? takeDisplayName = _safePresentationText(
    item.voiceRequest?.takeDisplayName,
    plan,
  );
  if (item.isReady || item.isAlreadyPresent) {
    final line = index.entityById(item.lineId!);
    final lineFacts = line?.summary.dialogLine;
    if (line == null ||
        line.kind != Revision3ContentEntityKind.dialogLine ||
        lineFacts == null ||
        line.displayName != item.lineDisplayName ||
        lineFacts.speaker != item.speaker) {
      throw const FormatException(
        'Voice folder plan line is absent from the exact content index.',
      );
    }
    final localizationReferences = line.references
        .where((reference) => reference.role == 'dialog_localization')
        .toList(growable: false);
    final localeSlotReferences = line.references
        .where(
          (reference) =>
              reference.role == 'dialog_voice_slot' &&
              reference.qualifier == plan.locale,
        )
        .toList(growable: false);
    if (localizationReferences.length != 1 ||
        localizationReferences.single.target.entityId != item.localizationId ||
        (item.slotCreated!
            ? localeSlotReferences.isNotEmpty ||
                  index.entityById(item.slotId!) != null
            : localeSlotReferences.length != 1 ||
                  localeSlotReferences.single.target.entityId != item.slotId)) {
      throw const FormatException(
        'Voice folder plan target differs from the exact content index.',
      );
    }
    lineLabel = _safePresentationText(line.displayName, plan);
    speakerLabel = _safePresentationText(lineFacts.speaker, plan);
    if (item.slotCreated!) {
      beforeTakeCount = 0;
      afterTakeCount = 1;
      targetState = Revision3VoiceFolderTargetState.unresolved;
    } else {
      final slot = index.entityById(item.slotId!);
      final facts = slot?.summary.voiceSlot;
      if (slot == null ||
          slot.kind != Revision3ContentEntityKind.voiceSlot ||
          facts == null ||
          slot.summary.primaryIdentity != plan.locale) {
        throw const FormatException(
          'Voice folder plan slot is absent from the exact content index.',
        );
      }
      beforeTakeCount = facts.candidateCount;
      afterTakeCount = facts.candidateCount + (item.isReady ? 1 : 0);
      targetState = _presentationTargetState(facts.targetResolution);
    }
    if (item.isAlreadyPresent) {
      final take = index.entityById(item.takeId!);
      final slot = index.entityById(item.slotId!);
      final isCandidate = slot?.references.any(
        (reference) =>
            reference.role == 'voice_candidate' &&
            reference.target.entityId == item.takeId,
      );
      if (take == null ||
          take.kind != Revision3ContentEntityKind.voiceTake ||
          take.summary.voiceTake?.locale != plan.locale ||
          isCandidate != true) {
        throw const FormatException(
          'Voice folder no-op take is absent from the exact content index.',
        );
      }
      takeDisplayName = _safePresentationText(take.displayName, plan);
    } else if (index.entityById(item.takeId!) != null) {
      throw const FormatException(
        'Voice folder new take already exists in the exact content index.',
      );
    }
  } else if (status == Revision3VoiceFolderRowStatus.unmatched) {
    targetState = Revision3VoiceFolderTargetState.unresolved;
  } else if (status == Revision3VoiceFolderRowStatus.ambiguous) {
    targetState = Revision3VoiceFolderTargetState.ambiguous;
  }

  return Revision3VoiceFolderReviewRow(
    ordinal: ordinal,
    rowToken: crypto.sha256
        .convert(
          utf8.encode(
            '${plan.planSha256}\u0000${plan.sourceManifestSha256}\u0000$ordinal\u0000${item.sourceName}',
          ),
        )
        .toString(),
    status: status,
    codec: codec,
    byteLength: item.asset?.byteLength,
    lineLabel: lineLabel,
    speakerLabel: speakerLabel,
    takeDisplayName: takeDisplayName,
    beforeTakeCount: beforeTakeCount,
    afterTakeCount: afterTakeCount,
    targetState: targetState,
  );
}

String? _safePresentationText(
  String? value,
  AuthoringRevision3VoiceBatchPlanResult plan,
) {
  if (value == null) return null;
  final candidate = value.trim();
  if (candidate.isEmpty ||
      candidate.contains('/') ||
      candidate.contains('\\') ||
      RegExp(r'[0-9a-fA-F]{32,64}').hasMatch(candidate)) {
    return null;
  }
  final folded = candidate.toLowerCase();
  final technicalTokens = <String?>[
    plan.projectId,
    plan.planSha256,
    plan.sourceManifestSha256,
    plan.basisHead.snapshotSha256,
    for (final item in plan.items) ...[
      item.sourceName,
      p.basenameWithoutExtension(item.sourceName),
      item.locId,
      item.lineId,
      item.localizationId,
      item.slotId,
      item.takeId,
      item.asset?.sha256,
    ],
  ];
  for (final token in technicalTokens) {
    if (token == null || token.isEmpty) continue;
    if (folded.contains(token.toLowerCase())) return null;
  }
  return candidate;
}

Revision3VoiceFolderTargetState _presentationTargetState(
  Revision3ContentVoiceTargetResolution state,
) => switch (state) {
  Revision3ContentVoiceTargetResolution.unresolved =>
    Revision3VoiceFolderTargetState.unresolved,
  Revision3ContentVoiceTargetResolution.ambiguous =>
    Revision3VoiceFolderTargetState.ambiguous,
  Revision3ContentVoiceTargetResolution.resolved =>
    Revision3VoiceFolderTargetState.resolved,
};

String _friendlyFolderLabel(
  String sourceFolder,
  AuthoringRevision3VoiceBatchPlanResult plan,
) {
  final label = p.basename(p.normalize(sourceFolder));
  if (label.isEmpty ||
      label == '.' ||
      label == '..' ||
      label.contains('/') ||
      label.contains('\\') ||
      label.contains(':') ||
      RegExp(r'[0-9a-fA-F]{32,64}').hasMatch(label) ||
      _folderLabelContainsNativeToken(label, plan)) {
    return 'Voice recordings';
  }
  return label;
}

bool _folderLabelContainsNativeToken(
  String label,
  AuthoringRevision3VoiceBatchPlanResult plan,
) {
  final folded = label.toLowerCase();
  final tokens = <String?>[
    plan.projectId,
    plan.planSha256,
    plan.sourceManifestSha256,
    for (final item in plan.items) ...[
      item.sourceName,
      p.basenameWithoutExtension(item.sourceName),
      item.locId,
      item.lineId,
      item.localizationId,
      item.slotId,
      item.takeId,
      item.asset?.sha256,
    ],
  ];
  for (final token in tokens) {
    if (token == null || token.isEmpty) continue;
    if (folded.contains(token.toLowerCase())) return true;
  }
  return false;
}

Never _translateFailure(
  Object error,
  StackTrace stackTrace, {
  required bool applying,
}) {
  if (error is Revision3VoiceFolderStaleCheckpointException ||
      error is Revision3VoiceFolderPublicationUncertainException) {
    Error.throwWithStackTrace(error, stackTrace);
  }
  if (error is Revision3VoiceFolderRequiresReopenException) {
    Error.throwWithStackTrace(
      applying
          ? const Revision3VoiceFolderPublicationUncertainException()
          : error,
      stackTrace,
    );
  }
  if (error is Revision3VoiceBatchStaleCheckpointException) {
    Error.throwWithStackTrace(
      const Revision3VoiceFolderStaleCheckpointException(),
      stackTrace,
    );
  }
  if (error is Revision3VoiceBatchRequiresReopenException ||
      error is ManagedProjectVerificationException) {
    Error.throwWithStackTrace(
      applying
          ? const Revision3VoiceFolderPublicationUncertainException()
          : const Revision3VoiceFolderRequiresReopenException(),
      stackTrace,
    );
  }
  if (error is ManagedProjectHeadConflictException) {
    Error.throwWithStackTrace(
      const Revision3VoiceFolderRequiresReopenException(),
      stackTrace,
    );
  }
  if (applying &&
      error is ModFfiException &&
      _voiceFolderSafePrepublicationCodes.contains(error.code)) {
    Error.throwWithStackTrace(
      const Revision3VoiceFolderStaleCheckpointException(),
      stackTrace,
    );
  }
  if (error is FormatException) {
    Error.throwWithStackTrace(
      applying
          ? const Revision3VoiceFolderPublicationUncertainException()
          : const Revision3VoiceFolderRequiresReopenException(),
      stackTrace,
    );
  }
  if (applying) {
    Error.throwWithStackTrace(
      const Revision3VoiceFolderPublicationUncertainException(),
      stackTrace,
    );
  }
  Error.throwWithStackTrace(error, stackTrace);
}

const _voiceFolderSafePrepublicationCodes = <String>{
  'AUTHORING_REVISION3_VOICE_BATCH_REQUEST_INVALID',
  'AUTHORING_REVISION3_VOICE_BATCH_INPUT_LIMIT',
  'AUTHORING_REVISION3_VOICE_BATCH_HEAD_INVALID',
  'AUTHORING_REVISION3_VOICE_BATCH_ROOT_OVERLAP',
  'AUTHORING_REVISION3_VOICE_BATCH_SOURCE_UNAVAILABLE',
  'AUTHORING_REVISION3_VOICE_BATCH_SOURCE_UNSAFE',
  'AUTHORING_REVISION3_VOICE_BATCH_SOURCE_LIMIT',
  'AUTHORING_REVISION3_VOICE_BATCH_SOURCE_CHANGED',
  'AUTHORING_REVISION3_VOICE_BATCH_STORE_MISSING',
  'AUTHORING_REVISION3_VOICE_BATCH_STORE_UNSAFE',
  'AUTHORING_REVISION3_VOICE_BATCH_STORE_LIMIT',
  'AUTHORING_REVISION3_VOICE_BATCH_STORE_IO',
  'AUTHORING_REVISION3_VOICE_BATCH_RESPONSE_LIMIT',
  'AUTHORING_REVISION3_VOICE_BATCH_PLAN_CHANGED',
  'AUTHORING_REVISION3_VOICE_BATCH_NOT_READY',
};
