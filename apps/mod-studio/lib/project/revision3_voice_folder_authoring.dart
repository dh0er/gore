import 'dart:convert';

import 'revision3_voice_authoring.dart';

const _maxVoiceFolderPathBytes = 32 * 1024;
const _maxVoiceFolderTokenBytes = 2 * 1024 * 1024;
const _maxVoiceFolderFriendlyTextBytes = 1024;
const _maxVoiceFolderRows = 256;
const _maxVoiceFolderTakeCount = 0x7fffffff;
const _maxVoiceFolderByteLength = 0x1fffffffffffff;

typedef Revision3VoiceFolderPlanner =
    Future<Revision3VoiceFolderImportPlan> Function(
      Revision3VoiceFolderPlanRequest request,
    );

typedef Revision3VoiceFolderApplier =
    Future<Revision3VoiceFolderImportPublication> Function({
      required Revision3VoiceFolderImportPlan plan,
    });

/// One directory selected by the author and one explicit canonical locale.
///
/// The path is authority input only. Presentation must use the bounded friendly
/// folder/file labels returned by [Revision3VoiceFolderImportPlan].
final class Revision3VoiceFolderPlanRequest {
  Revision3VoiceFolderPlanRequest({
    required String folderPath,
    required String locale,
    required String expectedProjectId,
    required int expectedProjectRevision,
    required String expectedProjectHead,
    required String expectedCheckpointToken,
  }) : folderPath = _sourcePath(folderPath),
       locale = _canonicalLocale(locale),
       expectedProjectId = _token(expectedProjectId, 'project identity'),
       expectedProjectRevision = _revision(
         expectedProjectRevision,
         'project revision',
       ),
       expectedProjectHead = _token(expectedProjectHead, 'project head'),
       expectedCheckpointToken = _token(
         expectedCheckpointToken,
         'checkpoint token',
       );

  final String folderPath;
  final String locale;
  final String expectedProjectId;
  final int expectedProjectRevision;
  final String expectedProjectHead;
  final String expectedCheckpointToken;
}

enum Revision3VoiceFolderRowStatus {
  ready,
  alreadyPresent,
  unmatched,
  ambiguous,
  invalid,
}

enum Revision3VoiceFolderCodec { vorbis, opus }

enum Revision3VoiceFolderTargetState { unresolved, ambiguous, resolved }

/// One deterministic, presentation-safe review row.
///
/// [rowToken] is opaque mutation authority and is deliberately not suitable for
/// display. Source filenames are deliberately absent: the V1 filename is the
/// LocID and therefore technical identity, not presentation. Optional rendered
/// strings are bounded friendly labels only.
final class Revision3VoiceFolderReviewRow {
  Revision3VoiceFolderReviewRow({
    required this.ordinal,
    required String rowToken,
    required this.status,
    required this.codec,
    required this.byteLength,
    required String? lineLabel,
    required String? speakerLabel,
    required String? takeDisplayName,
    required this.beforeTakeCount,
    required this.afterTakeCount,
    required this.targetState,
    this.selectionUnchanged = true,
    this.targetUnchanged = true,
  }) : rowToken = _token(rowToken, 'row token'),
       lineLabel = _optionalFriendlyText(lineLabel, 'dialog-line label'),
       speakerLabel = _optionalFriendlyText(speakerLabel, 'speaker label'),
       takeDisplayName = _optionalFriendlyText(
         takeDisplayName,
         'take display name',
       ) {
    if (ordinal < 0 || ordinal >= _maxVoiceFolderRows) {
      throw const FormatException('Voice folder row ordinal is invalid.');
    }
    final bytes = byteLength;
    if (bytes != null && (bytes <= 0 || bytes > _maxVoiceFolderByteLength)) {
      throw const FormatException('Voice folder row byte length is invalid.');
    }
    final before = beforeTakeCount;
    final after = afterTakeCount;
    if ((before == null) != (after == null) ||
        (before != null &&
            (before < 0 || after! < 0 || after > _maxVoiceFolderTakeCount))) {
      throw const FormatException('Voice folder take counts are invalid.');
    }
    if (!selectionUnchanged || !targetUnchanged) {
      throw const FormatException(
        'Voice folder V1 may not change selection or target evidence.',
      );
    }
    switch (status) {
      case Revision3VoiceFolderRowStatus.ready:
        if (codec == null ||
            bytes == null ||
            before == null ||
            after != before + 1 ||
            targetState == null) {
          throw const FormatException('Ready Voice folder row is incomplete.');
        }
      case Revision3VoiceFolderRowStatus.alreadyPresent:
        if (codec == null ||
            bytes == null ||
            before == null ||
            after != before ||
            targetState == null) {
          throw const FormatException(
            'Existing Voice folder row is incomplete.',
          );
        }
      case Revision3VoiceFolderRowStatus.unmatched ||
          Revision3VoiceFolderRowStatus.ambiguous:
        if (codec == null || bytes == null || before != null) {
          throw const FormatException(
            'Unmapped Voice folder row facts disagree.',
          );
        }
      case Revision3VoiceFolderRowStatus.invalid:
        if (before != null) {
          throw const FormatException(
            'Invalid Voice folder row may not claim a semantic change.',
          );
        }
    }
  }

  final int ordinal;
  final String rowToken;
  final Revision3VoiceFolderRowStatus status;
  final Revision3VoiceFolderCodec? codec;
  final int? byteLength;
  final String? lineLabel;
  final String? speakerLabel;
  final String? takeDisplayName;
  final int? beforeTakeCount;
  final int? afterTakeCount;
  final Revision3VoiceFolderTargetState? targetState;
  final bool selectionUnchanged;
  final bool targetUnchanged;

  bool get isReady => status == Revision3VoiceFolderRowStatus.ready;
  bool get isAlreadyPresent =>
      status == Revision3VoiceFolderRowStatus.alreadyPresent;
  bool get isBlocked => !isReady && !isAlreadyPresent;

  /// V1 always imports as Recorded without selecting the new take.
  bool get importedTakeWillBeSelected => false;

  /// V1 never changes localization text while importing a folder.
  bool get changesDialogText => false;
}

final class Revision3VoiceFolderPlanCounts {
  const Revision3VoiceFolderPlanCounts._({
    required this.scanned,
    required this.ogg,
    required this.ready,
    required this.alreadyPresent,
    required this.unmatched,
    required this.ambiguous,
    required this.invalid,
    required this.ignored,
  });

  factory Revision3VoiceFolderPlanCounts.fromRows(
    List<Revision3VoiceFolderReviewRow> rows, {
    required int scannedEntryCount,
    required int ignoredEntryCount,
  }) {
    if (rows.length > _maxVoiceFolderRows ||
        scannedEntryCount < 0 ||
        ignoredEntryCount < 0 ||
        scannedEntryCount != rows.length + ignoredEntryCount ||
        scannedEntryCount > 0x7fffffff) {
      throw const FormatException('Voice folder scan counts are invalid.');
    }
    var ready = 0;
    var alreadyPresent = 0;
    var unmatched = 0;
    var ambiguous = 0;
    var invalid = 0;
    for (final row in rows) {
      switch (row.status) {
        case Revision3VoiceFolderRowStatus.ready:
          ready++;
        case Revision3VoiceFolderRowStatus.alreadyPresent:
          alreadyPresent++;
        case Revision3VoiceFolderRowStatus.unmatched:
          unmatched++;
        case Revision3VoiceFolderRowStatus.ambiguous:
          ambiguous++;
        case Revision3VoiceFolderRowStatus.invalid:
          invalid++;
      }
    }
    return Revision3VoiceFolderPlanCounts._(
      scanned: scannedEntryCount,
      ogg: rows.length,
      ready: ready,
      alreadyPresent: alreadyPresent,
      unmatched: unmatched,
      ambiguous: ambiguous,
      invalid: invalid,
      ignored: ignoredEntryCount,
    );
  }

  final int scanned;
  final int ogg;
  final int ready;
  final int alreadyPresent;
  final int unmatched;
  final int ambiguous;
  final int invalid;
  final int ignored;

  int get blocked => unmatched + ambiguous + invalid;
}

/// Exact, immutable review authority for one folder and locale.
///
/// Every row is one direct Ogg file. Apply is available only when every row is
/// ready or already present and at least one row is ready. Ignored non-Ogg
/// entries are represented only by bounded friendly counts, never row data.
final class Revision3VoiceFolderImportPlan {
  Revision3VoiceFolderImportPlan({
    required String projectId,
    required int projectRevision,
    required String projectHead,
    required String checkpointToken,
    required String planToken,
    required String folderLabel,
    required String locale,
    required int scannedEntryCount,
    required int ignoredEntryCount,
    required List<Revision3VoiceFolderReviewRow> rows,
  }) : projectId = _token(projectId, 'project identity'),
       projectRevision = _revision(projectRevision, 'project revision'),
       projectHead = _token(projectHead, 'project head'),
       checkpointToken = _token(checkpointToken, 'checkpoint token'),
       planToken = _token(planToken, 'plan token'),
       folderLabel = _friendlyLeaf(folderLabel, 'folder label'),
       locale = _canonicalLocale(locale),
       rows = List<Revision3VoiceFolderReviewRow>.unmodifiable(rows),
       counts = Revision3VoiceFolderPlanCounts.fromRows(
         rows,
         scannedEntryCount: scannedEntryCount,
         ignoredEntryCount: ignoredEntryCount,
       ) {
    if (this.rows.length > _maxVoiceFolderRows) {
      throw const FormatException('Voice folder review is not bounded.');
    }
    final rowTokens = <String>{};
    for (var index = 0; index < this.rows.length; index++) {
      final row = this.rows[index];
      if (row.ordinal != index || !rowTokens.add(row.rowToken)) {
        throw const FormatException(
          'Voice folder rows are not deterministic and unique.',
        );
      }
    }
  }

  final String projectId;
  final int projectRevision;
  final String projectHead;
  final String checkpointToken;
  final String planToken;
  final String folderLabel;
  final String locale;
  final List<Revision3VoiceFolderReviewRow> rows;
  final Revision3VoiceFolderPlanCounts counts;

  List<Revision3VoiceFolderReviewRow> get readyRows =>
      List<Revision3VoiceFolderReviewRow>.unmodifiable(
        rows.where((row) => row.isReady),
      );

  bool get hasReadyRows => counts.ready > 0;
  bool get hasBlockingRows => counts.blocked > 0;
  bool get canApply => hasReadyRows && !hasBlockingRows;

  /// V1 always imports the complete ready set as Recorded, not selected, and
  /// without changing dialog text, target evidence, game files, or saves.
  bool get importsRecordedOnly => true;
  bool get changesSelection => false;
  bool get changesDialogText => false;
}

final class Revision3VoiceFolderImportPublication {
  Revision3VoiceFolderImportPublication({
    required String projectId,
    required int projectRevision,
    required String projectHead,
    required String checkpointToken,
    required String planToken,
    required this.importedCount,
  }) : projectId = _token(projectId, 'published project identity'),
       projectRevision = _revision(
         projectRevision,
         'published project revision',
       ),
       projectHead = _token(projectHead, 'published project head'),
       checkpointToken = _token(checkpointToken, 'published checkpoint token'),
       planToken = _token(planToken, 'published plan token') {
    if (importedCount <= 0 || importedCount > _maxVoiceFolderRows) {
      throw const FormatException('Published Voice folder count is invalid.');
    }
  }

  final String projectId;
  final int projectRevision;
  final String projectHead;
  final String checkpointToken;
  final String planToken;
  final int importedCount;
}

/// Strict presentation boundary over a future native plan/apply adapter.
///
/// It rejects identity drift, incomplete all-or-none plans, malformed counts,
/// and any apply result other than one exact `revision + 1` publication.
final class Revision3VoiceFolderAuthoringService {
  const Revision3VoiceFolderAuthoringService({
    required Revision3VoiceFolderPlanner planFolder,
    required Revision3VoiceFolderApplier applyPlan,
  }) : this._(planFolder, applyPlan);

  const Revision3VoiceFolderAuthoringService._(
    this._planFolder,
    this._applyPlan,
  );

  final Revision3VoiceFolderPlanner _planFolder;
  final Revision3VoiceFolderApplier _applyPlan;

  Future<Revision3VoiceFolderImportPlan> plan(
    Revision3VoiceFolderPlanRequest request,
  ) async {
    final plan = await _planFolder(request);
    if (plan.projectId != request.expectedProjectId ||
        plan.projectRevision != request.expectedProjectRevision ||
        plan.projectHead != request.expectedProjectHead ||
        plan.checkpointToken != request.expectedCheckpointToken ||
        plan.locale != request.locale) {
      throw const Revision3VoiceFolderRequiresReopenException();
    }
    return plan;
  }

  Future<Revision3VoiceFolderImportPublication> apply({
    required Revision3VoiceFolderImportPlan plan,
  }) async {
    if (!plan.canApply) {
      throw const FormatException(
        'Every Ogg recording must be ready or already present, with at least one new recording.',
      );
    }
    final publication = await _applyPlan(plan: plan);
    if (publication.projectId != plan.projectId ||
        publication.projectRevision != plan.projectRevision + 1 ||
        publication.planToken != plan.planToken ||
        publication.importedCount != plan.counts.ready ||
        publication.projectHead == plan.projectHead ||
        publication.checkpointToken == plan.checkpointToken) {
      throw const Revision3VoiceFolderRequiresReopenException();
    }
    return publication;
  }
}

final class Revision3VoiceFolderStaleCheckpointException implements Exception {
  const Revision3VoiceFolderStaleCheckpointException();
}

final class Revision3VoiceFolderRequiresReopenException implements Exception {
  const Revision3VoiceFolderRequiresReopenException();
}

final class Revision3VoiceFolderPublicationUncertainException
    implements Exception {
  const Revision3VoiceFolderPublicationUncertainException();
}

String _sourcePath(String value) {
  if (value.isEmpty ||
      value.trim() != value ||
      utf8.encode(value).length > _maxVoiceFolderPathBytes ||
      value.runes.any(_control)) {
    throw const FormatException('Choose one bounded Voice source folder.');
  }
  return value;
}

String _canonicalLocale(String value) {
  final normalized = value.trim();
  if (normalized != value || !revision3VoiceLocaleIsCanonical(normalized)) {
    throw const FormatException('Choose one canonical Voice locale.');
  }
  return normalized;
}

int _revision(int value, String context) {
  if (value < 0 || value > _maxVoiceFolderByteLength) {
    throw FormatException('$context is invalid.');
  }
  return value;
}

String _token(String value, String context) {
  if (value.isEmpty ||
      utf8.encode(value).length > _maxVoiceFolderTokenBytes ||
      value.contains('\u0000')) {
    throw FormatException('$context is invalid.');
  }
  return value;
}

String _friendlyLeaf(String value, String context) {
  final result = _friendlyText(value, context);
  if (result == '.' ||
      result == '..' ||
      result.contains('/') ||
      result.contains('\\') ||
      result.contains(':')) {
    throw FormatException('$context is not a friendly leaf label.');
  }
  return result;
}

String? _optionalFriendlyText(String? value, String context) =>
    value == null ? null : _friendlyPresentationText(value, context);

String _friendlyPresentationText(String value, String context) {
  final result = _friendlyText(value, context);
  if (result.contains('\\') ||
      result.startsWith('/') ||
      RegExp(r'^[A-Za-z]:[/\\]').hasMatch(result) ||
      RegExp(r'[0-9a-fA-F]{32,64}').hasMatch(result)) {
    throw FormatException('$context exposes technical identity.');
  }
  return result;
}

String _friendlyText(String value, String context) {
  if (value.isEmpty ||
      value.trim() != value ||
      utf8.encode(value).length > _maxVoiceFolderFriendlyTextBytes ||
      value.runes.any(_control)) {
    throw FormatException('$context is invalid.');
  }
  return value;
}

bool _control(int rune) => rune < 0x20 || (rune >= 0x7f && rune <= 0x9f);
