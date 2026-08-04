part of '../core/mod_ffi.dart';

const _maxAuthoringRevision3ProjectHistoryEntries = 256;

/// One native-proven checkpoint retained by the exact-current sealed history
/// manifest. [head] is an opaque capability and must not be displayed.
final class AuthoringRevision3ProjectHistoryEntry {
  const AuthoringRevision3ProjectHistoryEntry._({
    required this.head,
    required this.projectId,
    required this.projectRevision,
    required this.current,
  });

  final AuthoringWorkingHead head;
  final String projectId;
  final int projectRevision;
  final bool current;

  factory AuthoringRevision3ProjectHistoryEntry._fromJson(Object? value) {
    final json = _authoringRequiredObject(
      value,
      'revision-3 project history entry',
    );
    _authoringExactFields(json, const <String>{
      'head_json',
      'project_id',
      'project_revision',
      'current',
    }, 'revision-3 project history entry');
    return AuthoringRevision3ProjectHistoryEntry._(
      head: AuthoringWorkingHead.fromCanonicalJson(
        _authoringRevision3ResponseString(
          json,
          'head_json',
          maxBytes: _maxAuthoringHeadJsonBytes,
        ),
      ),
      projectId: _authoringEntityId(
        _authoringRequiredString(json, 'project_id', maxBytes: 32),
        'project_id',
      ),
      projectRevision: _authoringRequiredInt(
        json,
        'project_revision',
        max: _maxAuthoringSignedJsonInteger,
      ),
      current: _authoringRequiredBool(json, 'current'),
    );
  }
}

/// Strict read-only response for one bounded exact-current lineage.
final class AuthoringRevision3ProjectHistoryResult {
  const AuthoringRevision3ProjectHistoryResult._({
    required this.basisHead,
    required this.projectId,
    required this.projectRevision,
    required this.entries,
    required this.historyTruncated,
  });

  final AuthoringWorkingHead basisHead;
  final String projectId;
  final int projectRevision;
  final List<AuthoringRevision3ProjectHistoryEntry> entries;
  final bool historyTruncated;

  factory AuthoringRevision3ProjectHistoryResult.fromJson(
    Map<String, Object?> json, {
    required AuthoringWorkingHead expectedHead,
  }) {
    _authoringExactFields(json, const <String>{
      'ok',
      'outcome',
      'basis_head_json',
      'project_id',
      'project_revision',
      'entries',
      'history_truncated',
      'history_authority',
      'project_mutation',
      'game_mutation',
      'save_mutation',
      'build_status',
      'deployment_status',
      'runtime_status',
      'publication_status',
    }, 'revision-3 project history response');
    if (json['ok'] != true ||
        json['outcome'] != 'listed_exact_current' ||
        json['history_authority'] != 'authenticated_bounded_history' ||
        json['project_mutation'] != 'not_performed' ||
        json['game_mutation'] != 'not_performed' ||
        json['save_mutation'] != 'not_performed' ||
        json['build_status'] != 'not_performed' ||
        json['deployment_status'] != 'not_performed' ||
        json['runtime_status'] != 'runtime_unqualified' ||
        json['publication_status'] != 'not_applicable') {
      throw const FormatException(
        'revision-3 project history response widens its read-only authority',
      );
    }
    final basisHead = AuthoringWorkingHead.fromCanonicalJson(
      _authoringRevision3ResponseString(
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
      max: _maxAuthoringSignedJsonInteger,
    );
    final rawEntries = json['entries'];
    if (rawEntries is! List ||
        rawEntries.isEmpty ||
        rawEntries.length > _maxAuthoringRevision3ProjectHistoryEntries) {
      throw const FormatException(
        'revision-3 project history response has an invalid entry count',
      );
    }
    final entries = rawEntries
        .map(AuthoringRevision3ProjectHistoryEntry._fromJson)
        .toList(growable: false);
    final seenHeads = <String>{};
    for (var index = 0; index < entries.length; index++) {
      final entry = entries[index];
      if (entry.projectId != projectId ||
          entry.current != (index == 0) ||
          entry.projectRevision != projectRevision - index ||
          !seenHeads.add(entry.head.canonicalJson)) {
        throw const FormatException(
          'revision-3 project history response is not one contiguous unique lineage',
        );
      }
    }
    if (basisHead.canonicalJson != expectedHead.canonicalJson ||
        entries.first.head.canonicalJson != basisHead.canonicalJson ||
        entries.first.projectRevision != projectRevision) {
      throw const FormatException(
        'revision-3 project history response disagrees with its exact current basis',
      );
    }
    return AuthoringRevision3ProjectHistoryResult._(
      basisHead: basisHead,
      projectId: projectId,
      projectRevision: projectRevision,
      entries: List<AuthoringRevision3ProjectHistoryEntry>.unmodifiable(
        entries,
      ),
      historyTruncated: _authoringRequiredBool(json, 'history_truncated'),
    );
  }
}

/// Native prepare-only result for copying an authenticated ancestor into one
/// new current+1 project checkpoint.
final class AuthoringRevision3ProjectHistoryRestorePreparation {
  const AuthoringRevision3ProjectHistoryRestorePreparation._({
    required this.basisHead,
    required this.directParentHead,
    required this.restoredFromHead,
    required this.head,
    required this.projectJson,
    required this.projectId,
    required this.previousProjectRevision,
    required this.revision,
    required this.restoredFromRevision,
  });

  final AuthoringWorkingHead basisHead;
  final AuthoringWorkingHead directParentHead;
  final AuthoringWorkingHead restoredFromHead;
  final AuthoringWorkingHead head;
  final String projectJson;
  final String projectId;
  final int previousProjectRevision;
  final int revision;
  final int restoredFromRevision;

  factory AuthoringRevision3ProjectHistoryRestorePreparation.fromJson(
    Map<String, Object?> json, {
    required AuthoringWorkingHead expectedHead,
    required AuthoringWorkingHead targetHead,
  }) {
    _authoringExactFields(json, const <String>{
      'ok',
      'outcome',
      'basis_head_json',
      'direct_parent_head_json',
      'restored_from_head_json',
      'head_json',
      'project_json',
      'project_id',
      'previous_project_revision',
      'revision',
      'restored_from_revision',
      'history_authority',
      'project_mutation',
      'game_mutation',
      'save_mutation',
      'build_status',
      'deployment_status',
      'runtime_status',
      'publication_status',
    }, 'revision-3 project history restore response');
    if (json['ok'] != true ||
        json['outcome'] != 'prepared_restore_unpublished' ||
        json['history_authority'] != 'authenticated_bounded_history' ||
        json['project_mutation'] != 'prepared_not_published' ||
        json['game_mutation'] != 'not_performed' ||
        json['save_mutation'] != 'not_performed' ||
        json['build_status'] != 'not_performed' ||
        json['deployment_status'] != 'not_performed' ||
        json['runtime_status'] != 'runtime_unqualified' ||
        json['publication_status'] != 'not_supported') {
      throw const FormatException(
        'revision-3 project history restore response widens its prepare-only authority',
      );
    }
    final basisHead = _historyHead(json, 'basis_head_json');
    final directParentHead = _historyHead(json, 'direct_parent_head_json');
    final restoredFromHead = _historyHead(json, 'restored_from_head_json');
    final head = _historyHead(json, 'head_json');
    if (basisHead.canonicalJson != expectedHead.canonicalJson ||
        directParentHead.canonicalJson != expectedHead.canonicalJson ||
        restoredFromHead.canonicalJson != targetHead.canonicalJson ||
        head.canonicalJson == expectedHead.canonicalJson ||
        head.canonicalJson == targetHead.canonicalJson) {
      throw const FormatException(
        'revision-3 project history restore response has an invalid head transition',
      );
    }
    final projectJson = _authoringRevision3ResponseString(
      json,
      'project_json',
      maxBytes: _maxAuthoringProjectJsonBytes,
    );
    final project = _authoringRequireCanonicalRevision3ProjectJson(projectJson);
    final projectId = _authoringEntityId(
      _authoringRequiredString(json, 'project_id', maxBytes: 32),
      'project_id',
    );
    final previousProjectRevision = _authoringRequiredInt(
      json,
      'previous_project_revision',
      max: _maxAuthoringSignedJsonInteger - 1,
    );
    final revision = _authoringRequiredInt(
      json,
      'revision',
      min: 1,
      max: _maxAuthoringSignedJsonInteger,
    );
    final restoredFromRevision = _authoringRequiredInt(
      json,
      'restored_from_revision',
      max: previousProjectRevision,
    );
    if (project.projectId != projectId ||
        project.revision != revision ||
        revision != previousProjectRevision + 1 ||
        restoredFromRevision >= previousProjectRevision) {
      throw const FormatException(
        'revision-3 project history restore response has an invalid project transition',
      );
    }
    return AuthoringRevision3ProjectHistoryRestorePreparation._(
      basisHead: basisHead,
      directParentHead: directParentHead,
      restoredFromHead: restoredFromHead,
      head: head,
      projectJson: projectJson,
      projectId: projectId,
      previousProjectRevision: previousProjectRevision,
      revision: revision,
      restoredFromRevision: restoredFromRevision,
    );
  }
}

AuthoringWorkingHead _historyHead(Map<String, Object?> json, String field) =>
    AuthoringWorkingHead.fromCanonicalJson(
      _authoringRevision3ResponseString(
        json,
        field,
        maxBytes: _maxAuthoringHeadJsonBytes,
      ),
    );
