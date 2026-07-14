import '../core/mod_ffi.dart';

typedef Revision3DataAssetStageLoader =
    Future<List<AuthoringRevision3DataAssetStage>> Function();

typedef Revision3DataAssetStagePublisher =
    Future<Revision3DataAssetStagePublication> Function({
      required String patchReceiptPath,
    });

typedef Revision3DataAssetStageRemover =
    Future<Revision3DataAssetStageRemovalPublication> Function({
      required String targetPath,
    });

typedef Revision3DataAssetPatchReceiptPicker = Future<String?> Function();

/// Exact project checkpoint returned after one verified fixed-leaf edit was
/// imported into the managed project.
///
/// This is project-storage evidence only. It grants no build, pack, deploy, or
/// runtime authority and deliberately carries no receipt path or raw offset.
final class Revision3DataAssetStagePublication {
  Revision3DataAssetStagePublication({
    required String projectId,
    required int projectRevision,
    required this.stage,
    required this.deduplicatedBlobs,
  }) : projectId = _dataAssetProjectId(projectId),
       projectRevision = _dataAssetProjectRevision(projectRevision) {
    if (stage.projectId != this.projectId ||
        stage.stagedProjectRevision != this.projectRevision ||
        deduplicatedBlobs < 0 ||
        deduplicatedBlobs > 4 + stage.sidecars.length) {
      throw const FormatException(
        'Published DataAsset edit does not match its project checkpoint.',
      );
    }
  }

  final String projectId;
  final int projectRevision;
  final AuthoringRevision3DataAssetStage stage;
  final int deduplicatedBlobs;
}

/// Exact project checkpoint returned after a stage-registry-only removal.
///
/// Removal does not claim that immutable CAS objects were reclaimed and does
/// not touch a source receipt or game installation.
final class Revision3DataAssetStageRemovalPublication {
  Revision3DataAssetStageRemovalPublication({
    required String projectId,
    required int projectRevision,
    required this.removed,
  }) : projectId = _dataAssetProjectId(projectId),
       projectRevision = _dataAssetProjectRevision(projectRevision) {
    if (removed.projectId != this.projectId ||
        removed.stagedProjectRevision >= this.projectRevision) {
      throw const FormatException(
        'Removed DataAsset edit does not match its project checkpoint.',
      );
    }
  }

  final String projectId;
  final int projectRevision;
  final AuthoringRevision3DataAssetStage removed;
}

/// The visible panel was opened for a checkpoint that is no longer current.
final class Revision3DataAssetStaleCheckpointException implements Exception {
  const Revision3DataAssetStaleCheckpointException();

  @override
  String toString() =>
      'The project changed. Reload DataAsset edits before continuing.';
}

/// Exact verification became uncertain; the managed project must be reopened.
final class Revision3DataAssetRequiresReopenException implements Exception {
  const Revision3DataAssetRequiresReopenException();

  @override
  String toString() =>
      'Reopen the managed project before changing DataAsset edits.';
}

String revision3DataAssetFriendlyError(Object error) {
  if (error is Revision3DataAssetStaleCheckpointException ||
      error is Revision3DataAssetRequiresReopenException) {
    return error.toString();
  }
  if (error is ArgumentError) {
    return 'The selected verified edit file could not be used.';
  }
  if (error is ModFfiException) {
    return switch (error.code) {
      'AUTHORING_REVISION3_DATAASSET_INPUT_MISSING' =>
        'The selected DataAsset proof file no longer exists.',
      'AUTHORING_REVISION3_DATAASSET_INPUT_UNSAFE' ||
      'AUTHORING_REVISION3_DATAASSET_INPUT_INVALID' =>
        'The selected file is not a complete verified DataAsset proof for this game version.',
      'AUTHORING_REVISION3_DATAASSET_EDIT_INVALID' =>
        'The value or extraction proof no longer matches the inspected DataAsset. Inspect it again and retry.',
      'AUTHORING_REVISION3_DATAASSET_EXECUTABLE_MISMATCH' =>
        'This verified edit belongs to a different game version.',
      'AUTHORING_REVISION3_DATAASSET_TARGET_EXISTS' =>
        'This DataAsset already has a verified edit in the project.',
      'AUTHORING_REVISION3_DATAASSET_TARGET_MISSING' =>
        'This DataAsset edit is no longer present. Refresh the list.',
      'AUTHORING_REVISION3_DATAASSET_INPUT_LIMIT' ||
      'AUTHORING_REVISION3_DATAASSET_PROJECT_LIMIT' ||
      'AUTHORING_REVISION3_DATAASSET_RESPONSE_LIMIT' ||
      'AUTHORING_REVISION3_DATAASSET_REVISION_LIMIT' ||
      'AUTHORING_REVISION3_DATAASSET_STORE_LIMIT' =>
        'This DataAsset edit exceeds the supported project limits.',
      _ => 'The verified DataAsset edit could not be processed.',
    };
  }
  return 'The DataAsset operation could not be verified exactly.';
}

String _dataAssetProjectId(String value) {
  if (!RegExp(r'^[0-9a-f]{32}$').hasMatch(value) ||
      value == '00000000000000000000000000000000') {
    throw const FormatException(
      'DataAsset edit has no valid project identity.',
    );
  }
  return value;
}

int _dataAssetProjectRevision(int value) {
  if (value < 1 || value > 0x7fffffffffffffff) {
    throw const FormatException(
      'DataAsset edit has no valid project revision.',
    );
  }
  return value;
}
