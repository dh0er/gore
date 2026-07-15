part of '../core/mod_ffi.dart';

const _maxAuthoringRevision3DialogLineRequestBytes = 1024 * 1024;
const _maxAuthoringRevision3DialogDisplayNameBytes = 256;
const _maxAuthoringRevision3DialogIdentityBytes = 256;
const _maxAuthoringRevision3DialogTextBytes = 64 * 1024;
const _maxAuthoringRevision3DialogTextsBytes = 512 * 1024;
const _maxAuthoringRevision3DialogTexts = 1000;
const _maxAuthoringRevision3DialogLocalizationReadLocIdBytes = 1020;
const _maxAuthoringRevision3DialogLocalizationReadLocales = 1000;
const _maxAuthoringRevision3DialogLocalizationPreviewBytes = 512;

enum AuthoringRevision3DialogLocalizationAction { created, reusedExact }

enum AuthoringRevision3DialogBuildStatus { blocked }

enum AuthoringRevision3DialogRuntimeStatus { runtimeUnqualified }

enum AuthoringRevision3DialogTopicAuthority { notGranted }

enum AuthoringRevision3DialogPublicationStatus { notSupported }

enum AuthoringRevision3DialogLocalizationReadContentAuthority {
  readOnlyExactCurrentLocalization,
}

enum AuthoringRevision3DialogLocalizationReadBuildStatus { notEvaluated }

enum AuthoringRevision3DialogLocalizationReadRuntimeStatus {
  runtimeUnqualified,
}

enum AuthoringRevision3DialogLocalizationReadPublicationStatus { notApplicable }

final class AuthoringRevision3DialogLocalizationReadRequestV1 {
  AuthoringRevision3DialogLocalizationReadRequestV1({
    required this.expectedHead,
    required String localizationId,
    required int expectedLocalizationRevision,
    required String expectedLocId,
  }) : localizationId = _dialogEntityId(
         localizationId,
         'localization read entity ID',
       ),
       expectedLocalizationRevision = _dialogRevision(
         expectedLocalizationRevision,
         'localization read revision',
       ),
       expectedLocId = _dialogLocalizationReadLocId(expectedLocId);

  final AuthoringWorkingHead expectedHead;
  final String localizationId;
  final int expectedLocalizationRevision;
  final String expectedLocId;

  Map<String, Object?> _payload(String root) => <String, Object?>{
    'root': root,
    'expected_head_json': expectedHead.canonicalJson,
    'localization_id': localizationId,
    'expected_localization_revision': expectedLocalizationRevision,
    'expected_loc_id': expectedLocId,
  };
}

final class AuthoringRevision3DialogLocalizationLocalePreview {
  const AuthoringRevision3DialogLocalizationLocalePreview._({
    required this.locale,
    required this.preview,
    required this.truncated,
    required this.hasNonemptyText,
  });

  final String locale;
  final String preview;
  final bool truncated;
  final bool hasNonemptyText;

  factory AuthoringRevision3DialogLocalizationLocalePreview._fromJson(
    Object? value,
  ) {
    final json = _authoringRequiredObject(
      value,
      'revision-3 dialog localization locale preview',
    );
    _authoringExactFields(json, const <String>{
      'locale',
      'preview',
      'truncated',
      'has_nonempty_text',
    }, 'revision-3 dialog localization locale preview');
    final locale = _authoringRevision3VoiceLocale(
      _dialogLocalizationReadString(
        json['locale'],
        'localization preview locale',
        maxBytes: 35,
      ),
    );
    final preview = _dialogLocalizationReadString(
      json['preview'],
      'localization preview text',
      maxBytes: _maxAuthoringRevision3DialogLocalizationPreviewBytes,
      allowEmpty: true,
    );
    final truncated = json['truncated'];
    final hasNonemptyText = json['has_nonempty_text'];
    if (truncated is! bool || hasNonemptyText is! bool) {
      throw const FormatException(
        'revision-3 dialog localization preview flags are invalid',
      );
    }
    final previewHasNonWhitespace = _dialogLocalizationReadHasNonWhitespace(
      preview,
    );
    if ((!truncated && hasNonemptyText != previewHasNonWhitespace) ||
        (truncated && previewHasNonWhitespace && !hasNonemptyText)) {
      throw const FormatException(
        'revision-3 dialog localization preview text flags disagree',
      );
    }
    return AuthoringRevision3DialogLocalizationLocalePreview._(
      locale: locale,
      preview: preview,
      truncated: truncated,
      hasNonemptyText: hasNonemptyText,
    );
  }
}

final class AuthoringRevision3DialogLocalizationReadResult {
  const AuthoringRevision3DialogLocalizationReadResult._({
    required this.head,
    required this.projectId,
    required this.projectRevision,
    required this.localizationId,
    required this.localizationRevision,
    required this.locId,
    required this.locales,
    required this.contentAuthority,
    required this.buildStatus,
    required this.runtimeStatus,
    required this.publicationStatus,
  });

  final AuthoringWorkingHead head;
  final String projectId;
  final int projectRevision;
  final String localizationId;
  final int localizationRevision;
  final String locId;
  final List<AuthoringRevision3DialogLocalizationLocalePreview> locales;
  final AuthoringRevision3DialogLocalizationReadContentAuthority
  contentAuthority;
  final AuthoringRevision3DialogLocalizationReadBuildStatus buildStatus;
  final AuthoringRevision3DialogLocalizationReadRuntimeStatus runtimeStatus;
  final AuthoringRevision3DialogLocalizationReadPublicationStatus
  publicationStatus;

  factory AuthoringRevision3DialogLocalizationReadResult.fromJson(
    Map<String, Object?> json, {
    required AuthoringRevision3DialogLocalizationReadRequestV1 request,
  }) {
    _authoringExactFields(json, const <String>{
      'ok',
      'outcome',
      'head_json',
      'project_id',
      'project_revision',
      'localization_id',
      'localization_revision',
      'loc_id',
      'locales',
      'content_authority',
      'build_status',
      'runtime_status',
      'publication_status',
    }, 'revision-3 dialog localization read response');
    if (json['ok'] != true || json['outcome'] != 'read_only') {
      throw const FormatException(
        'revision-3 dialog localization response is not read-only',
      );
    }
    final head = AuthoringWorkingHead.fromCanonicalJson(
      _dialogLocalizationReadString(
        json['head_json'],
        'localization response head',
        maxBytes: _maxAuthoringHeadJsonBytes,
      ),
    );
    final projectId = _dialogEntityId(
      _dialogLocalizationReadString(
        json['project_id'],
        'localization response project ID',
        maxBytes: 32,
      ),
      'localization response project ID',
    );
    final projectRevision = _dialogRevision(
      _authoringRequiredInt(
        json,
        'project_revision',
        max: _maxAuthoringRevision3VoiceBasisRevision,
      ),
      'localization response project revision',
    );
    final localizationId = _dialogEntityId(
      _dialogLocalizationReadString(
        json['localization_id'],
        'localization response entity ID',
        maxBytes: 32,
      ),
      'localization response entity ID',
    );
    final localizationRevision = _dialogRevision(
      _authoringRequiredInt(
        json,
        'localization_revision',
        max: _maxAuthoringRevision3VoiceBasisRevision,
      ),
      'localization response entity revision',
    );
    final locId = _dialogLocalizationReadLocId(
      _dialogLocalizationReadString(
        json['loc_id'],
        'localization response identity',
        maxBytes: _maxAuthoringRevision3DialogLocalizationReadLocIdBytes,
      ),
    );
    final rawLocales = json['locales'];
    if (rawLocales is! List ||
        rawLocales.length >
            _maxAuthoringRevision3DialogLocalizationReadLocales) {
      throw const FormatException(
        'revision-3 dialog localization locale previews are not bounded',
      );
    }
    final locales = <AuthoringRevision3DialogLocalizationLocalePreview>[];
    String? previousLocale;
    for (final rawLocale in rawLocales) {
      final locale =
          AuthoringRevision3DialogLocalizationLocalePreview._fromJson(
            rawLocale,
          );
      if (previousLocale != null &&
          previousLocale.compareTo(locale.locale) >= 0) {
        throw const FormatException(
          'revision-3 dialog localization locales are not canonical order',
        );
      }
      previousLocale = locale.locale;
      locales.add(locale);
    }
    if (head.canonicalJson != request.expectedHead.canonicalJson ||
        localizationId != request.localizationId ||
        localizationRevision != request.expectedLocalizationRevision ||
        locId != request.expectedLocId) {
      throw const FormatException(
        'revision-3 dialog localization response disagrees with its exact request',
      );
    }
    return AuthoringRevision3DialogLocalizationReadResult._(
      head: head,
      projectId: projectId,
      projectRevision: projectRevision,
      localizationId: localizationId,
      localizationRevision: localizationRevision,
      locId: locId,
      locales: List.unmodifiable(locales),
      contentAuthority: switch (json['content_authority']) {
        'read_only_exact_current_localization' =>
          AuthoringRevision3DialogLocalizationReadContentAuthority
              .readOnlyExactCurrentLocalization,
        _ => throw const FormatException(
          'revision-3 dialog localization content authority is invalid',
        ),
      },
      buildStatus: switch (json['build_status']) {
        'not_evaluated' =>
          AuthoringRevision3DialogLocalizationReadBuildStatus.notEvaluated,
        _ => throw const FormatException(
          'revision-3 dialog localization build status is invalid',
        ),
      },
      runtimeStatus: switch (json['runtime_status']) {
        'runtime_unqualified' =>
          AuthoringRevision3DialogLocalizationReadRuntimeStatus
              .runtimeUnqualified,
        _ => throw const FormatException(
          'revision-3 dialog localization runtime status is invalid',
        ),
      },
      publicationStatus: switch (json['publication_status']) {
        'not_applicable' =>
          AuthoringRevision3DialogLocalizationReadPublicationStatus
              .notApplicable,
        _ => throw const FormatException(
          'revision-3 dialog localization publication status is invalid',
        ),
      },
    );
  }
}

/// Project-local localization intent. It carries no base-game catalog or
/// runtime authority and is bound to an exact managed checkpoint only when the
/// enclosing request is constructed inside the session lane.
sealed class AuthoringRevision3DialogLocalizationIntentV1 {
  const AuthoringRevision3DialogLocalizationIntentV1();

  String get localizationId;

  Map<String, Object?> _toJson();
}

final class AuthoringRevision3DialogLocalizationCreateIntentV1
    extends AuthoringRevision3DialogLocalizationIntentV1 {
  AuthoringRevision3DialogLocalizationCreateIntentV1({
    required String localizationId,
    required String displayName,
    required String locId,
    required Map<String, String> texts,
  }) : localizationId = _dialogEntityId(localizationId, 'localization_id'),
       displayName = _dialogDisplayName(
         displayName,
         'localization display name',
       ),
       locId = _dialogLocId(locId),
       texts = _dialogTexts(texts);

  @override
  final String localizationId;
  final String displayName;
  final String locId;
  final Map<String, String> texts;

  @override
  Map<String, Object?> _toJson() => <String, Object?>{
    'mode': 'create',
    'localization_id': localizationId,
    'display_name': displayName,
    'loc_id': locId,
    'texts': texts,
  };
}

final class AuthoringRevision3DialogLocalizationReuseExactIntentV1
    extends AuthoringRevision3DialogLocalizationIntentV1 {
  AuthoringRevision3DialogLocalizationReuseExactIntentV1({
    required String localizationId,
    required int expectedLocalizationRevision,
    required String expectedLocId,
  }) : localizationId = _dialogEntityId(localizationId, 'localization_id'),
       expectedLocalizationRevision = _dialogRevision(
         expectedLocalizationRevision,
         'expected localization revision',
       ),
       expectedLocId = _dialogLocId(expectedLocId);

  @override
  final String localizationId;
  final int expectedLocalizationRevision;
  final String expectedLocId;

  @override
  Map<String, Object?> _toJson() => <String, Object?>{
    'mode': 'reuse_exact',
    'localization_id': localizationId,
    'expected_localization_revision': expectedLocalizationRevision,
    'expected_loc_id': expectedLocId,
  };
}

final class AuthoringRevision3DialogEmptyVoiceSlotIntentV1 {
  AuthoringRevision3DialogEmptyVoiceSlotIntentV1({
    required String slotId,
    required String locale,
    required String displayName,
  }) : slotId = _dialogEntityId(slotId, 'voice slot ID'),
       locale = _authoringRevision3VoiceLocale(locale),
       displayName = _dialogDisplayName(displayName, 'Voice slot display name');

  final String slotId;
  final String locale;
  final String displayName;

  Map<String, Object?> _toJson() => <String, Object?>{
    'slot_id': slotId,
    'locale': locale,
    'display_name': displayName,
  };
}

/// Exact canonical request for the native prepare-only dialog transaction.
final class AuthoringRevision3DialogLineEntryRequestV1 {
  const AuthoringRevision3DialogLineEntryRequestV1._({
    required this.canonicalJson,
    required this.expectedHead,
    required this.expectedProjectId,
    required this.expectedRevision,
    required this.expectedTargetCanonicalJson,
    required this.lineId,
    required this.lineDisplayName,
    required this.lineAuthoredIdentity,
    required this.speakerHint,
    required this.localization,
    required this.voiceSlot,
  });

  factory AuthoringRevision3DialogLineEntryRequestV1.forProject({
    required AuthoringWorkingHead expectedHead,
    required String currentProjectJson,
    required String lineId,
    required String lineDisplayName,
    required String lineAuthoredIdentity,
    required String? speakerHint,
    required AuthoringRevision3DialogLocalizationIntentV1 localization,
    AuthoringRevision3DialogEmptyVoiceSlotIntentV1? voiceSlot,
  }) {
    final current = _authoringRequireCanonicalRevision3ProjectJson(
      currentProjectJson,
    );
    final json = <String, Object?>{
      'expected_head': jsonDecode(expectedHead.canonicalJson),
      'expected_project_id': current.projectId,
      'expected_revision': current.revision,
      'expected_target': current.project['target'],
      'line_id': lineId,
      'line_display_name': lineDisplayName,
      'line_authored_identity': lineAuthoredIdentity,
      'speaker_hint': ?speakerHint,
      'localization': localization._toJson(),
      if (voiceSlot != null) 'voice_slot': voiceSlot._toJson(),
    };
    return AuthoringRevision3DialogLineEntryRequestV1.fromCanonicalJson(
      jsonEncode(json),
      currentProjectJson: currentProjectJson,
    );
  }

  final String canonicalJson;
  final AuthoringWorkingHead expectedHead;
  final String expectedProjectId;
  final int expectedRevision;
  final String expectedTargetCanonicalJson;
  final String lineId;
  final String lineDisplayName;
  final String lineAuthoredIdentity;
  final String? speakerHint;
  final AuthoringRevision3DialogLocalizationIntentV1 localization;
  final AuthoringRevision3DialogEmptyVoiceSlotIntentV1? voiceSlot;

  factory AuthoringRevision3DialogLineEntryRequestV1.fromCanonicalJson(
    String value, {
    required String currentProjectJson,
  }) {
    try {
      _authoringRevision3RequestString(
        value,
        'dialogLineRequestJson',
        _maxAuthoringRevision3DialogLineRequestBytes,
      );
    } on ArgumentError {
      throw const FormatException(
        'revision-3 dialog-line request is not bounded UTF-8',
      );
    }
    final request = _authoringDecodeDuplicateSafeObject(
      value,
      'revision-3 dialog-line request',
    );
    final fieldOrder = <String>[
      'expected_head',
      'expected_project_id',
      'expected_revision',
      'expected_target',
      'line_id',
      'line_display_name',
      'line_authored_identity',
      if (request.containsKey('speaker_hint')) 'speaker_hint',
      'localization',
      if (request.containsKey('voice_slot')) 'voice_slot',
    ];
    _authoringExactFields(
      request,
      fieldOrder.toSet(),
      'revision-3 dialog-line request',
    );
    _authoringRevision3VoiceRequireFieldOrder(
      request,
      fieldOrder,
      'dialog-line request',
    );
    if (jsonEncode(request) != value) {
      throw const FormatException(
        'revision-3 dialog-line request is not canonical',
      );
    }
    final expectedHead = AuthoringWorkingHead.fromCanonicalJson(
      jsonEncode(
        _authoringRequiredObject(
          request['expected_head'],
          'revision-3 dialog-line expected head',
        ),
      ),
    );
    final projectId = _dialogEntityId(
      _dialogString(request, 'expected_project_id', maxBytes: 32),
      'expected_project_id',
    );
    final lineId = _dialogEntityId(
      _dialogString(request, 'line_id', maxBytes: 32),
      'line_id',
    );
    final displayName = _dialogDisplayName(
      _dialogString(
        request,
        'line_display_name',
        maxBytes: _maxAuthoringRevision3DialogDisplayNameBytes,
      ),
      'line display name',
    );
    final authoredIdentity = _dialogAuthoredIdentity(
      _dialogString(
        request,
        'line_authored_identity',
        maxBytes: _maxAuthoringRevision3DialogIdentityBytes,
      ),
    );
    final speaker = request.containsKey('speaker_hint')
        ? _dialogDisplayName(
            _dialogString(
              request,
              'speaker_hint',
              maxBytes: _maxAuthoringRevision3DialogDisplayNameBytes,
            ),
            'speaker hint',
          )
        : null;
    final localization = _dialogLocalizationIntent(request['localization']);
    final voiceSlot = request.containsKey('voice_slot')
        ? _dialogVoiceSlotIntent(request['voice_slot'])
        : null;
    final ids = <String>{lineId, localization.localizationId};
    if (voiceSlot != null) ids.add(voiceSlot.slotId);
    if (ids.length != 2 + (voiceSlot == null ? 0 : 1)) {
      throw const FormatException(
        'revision-3 dialog-line entity IDs must be distinct',
      );
    }
    final parsed = AuthoringRevision3DialogLineEntryRequestV1._(
      canonicalJson: value,
      expectedHead: expectedHead,
      expectedProjectId: projectId,
      expectedRevision: _dialogRevision(
        _authoringRequiredInt(
          request,
          'expected_revision',
          max: _maxAuthoringRevision3VoiceBasisRevision,
        ),
        'expected project revision',
      ),
      expectedTargetCanonicalJson: jsonEncode(
        _authoringRevision3VoiceGeneration(
          request['expected_target'],
          'dialog-line request target',
        ),
      ),
      lineId: lineId,
      lineDisplayName: displayName,
      lineAuthoredIdentity: authoredIdentity,
      speakerHint: speaker,
      localization: localization,
      voiceSlot: voiceSlot,
    );
    final current = _authoringRequireCanonicalRevision3ProjectJson(
      currentProjectJson,
    );
    parsed._requireExactProjectBinding(current);
    return parsed;
  }

  void _requireExactProjectBinding(
    ({Map<String, Object?> project, String projectId, int revision}) current,
  ) {
    if (expectedProjectId != current.projectId ||
        expectedRevision != current.revision ||
        expectedTargetCanonicalJson != jsonEncode(current.project['target'])) {
      throw const FormatException(
        'revision-3 dialog-line request does not bind the exact current project',
      );
    }
  }
}

/// Fully reopened, unpublished candidate returned by the native FFI route.
/// The managed session still owns fixed-head publication and a second full
/// reopen before this value can become current project state.
final class AuthoringRevision3DialogLineEntryPreparation {
  const AuthoringRevision3DialogLineEntryPreparation._({
    required this.basisHead,
    required this.head,
    required this.projectJson,
    required this.projectId,
    required this.revision,
    required this.lineId,
    required this.localizationId,
    required this.localizationAction,
    required this.voiceSlotId,
    required this.buildStatus,
    required this.runtimeStatus,
    required this.topicAuthority,
    required this.publicationStatus,
  });

  final AuthoringWorkingHead basisHead;
  final AuthoringWorkingHead head;
  final String projectJson;
  final String projectId;
  final int revision;
  final String lineId;
  final String localizationId;
  final AuthoringRevision3DialogLocalizationAction localizationAction;
  final String? voiceSlotId;
  final AuthoringRevision3DialogBuildStatus buildStatus;
  final AuthoringRevision3DialogRuntimeStatus runtimeStatus;
  final AuthoringRevision3DialogTopicAuthority topicAuthority;
  final AuthoringRevision3DialogPublicationStatus publicationStatus;

  factory AuthoringRevision3DialogLineEntryPreparation.fromJson(
    Map<String, Object?> json, {
    required String currentProjectJson,
    required AuthoringRevision3DialogLineEntryRequestV1 request,
  }) {
    final basis = _authoringRequireCanonicalRevision3ProjectJson(
      currentProjectJson,
    );
    request._requireExactProjectBinding(basis);
    _authoringExactFields(json, const <String>{
      'ok',
      'outcome',
      'basis_head_json',
      'head_json',
      'project_json',
      'project_id',
      'revision',
      'line_id',
      'localization_id',
      'localization_action',
      'voice_slot_id',
      'build_status',
      'runtime_status',
      'topic_authority',
      'publication_status',
    }, 'revision-3 dialog-line preparation response');
    if (json['ok'] != true || json['outcome'] != 'prepared_unpublished') {
      throw const FormatException(
        'revision-3 dialog-line response is not an unpublished preparation',
      );
    }
    final basisHead = AuthoringWorkingHead.fromCanonicalJson(
      _authoringRequiredString(
        json,
        'basis_head_json',
        maxBytes: _maxAuthoringHeadJsonBytes,
      ),
    );
    final head = AuthoringWorkingHead.fromCanonicalJson(
      _authoringRequiredString(
        json,
        'head_json',
        maxBytes: _maxAuthoringHeadJsonBytes,
      ),
    );
    if (basisHead.canonicalJson != request.expectedHead.canonicalJson ||
        head.canonicalJson == basisHead.canonicalJson) {
      throw const FormatException(
        'revision-3 dialog-line response has an invalid head transition',
      );
    }
    final projectJson = _authoringRequiredString(
      json,
      'project_json',
      maxBytes: _maxAuthoringProjectJsonBytes,
    );
    final candidate = _authoringRequireCanonicalRevision3ProjectJson(
      projectJson,
    );
    final projectId = _dialogEntityId(
      _authoringRequiredString(json, 'project_id', maxBytes: 32),
      'response project ID',
    );
    final revision = _authoringRequiredInt(
      json,
      'revision',
      min: 1,
      max: _maxAuthoringRevision3VoiceAppliedRevision,
    );
    final lineId = _dialogEntityId(
      _authoringRequiredString(json, 'line_id', maxBytes: 32),
      'response line ID',
    );
    final localizationId = _dialogEntityId(
      _authoringRequiredString(json, 'localization_id', maxBytes: 32),
      'response localization ID',
    );
    final action = switch (json['localization_action']) {
      'created' => AuthoringRevision3DialogLocalizationAction.created,
      'reused_exact' => AuthoringRevision3DialogLocalizationAction.reusedExact,
      _ => throw const FormatException(
        'revision-3 dialog-line response has an invalid localization action',
      ),
    };
    final rawSlotId = json['voice_slot_id'];
    final slotId = rawSlotId == null
        ? null
        : _dialogEntityId(
            _authoringRequiredString(json, 'voice_slot_id', maxBytes: 32),
            'response Voice slot ID',
          );
    final expectedAction =
        request.localization
            is AuthoringRevision3DialogLocalizationCreateIntentV1
        ? AuthoringRevision3DialogLocalizationAction.created
        : AuthoringRevision3DialogLocalizationAction.reusedExact;
    if (projectId != basis.projectId ||
        projectId != candidate.projectId ||
        revision != basis.revision + 1 ||
        revision != candidate.revision ||
        lineId != request.lineId ||
        localizationId != request.localization.localizationId ||
        action != expectedAction ||
        slotId != request.voiceSlot?.slotId) {
      throw const FormatException(
        'revision-3 dialog-line response disagrees with its exact request',
      );
    }
    _dialogRequireExactCandidate(
      basis.project,
      candidate.project,
      request: request,
    );
    return AuthoringRevision3DialogLineEntryPreparation._(
      basisHead: basisHead,
      head: head,
      projectJson: projectJson,
      projectId: projectId,
      revision: revision,
      lineId: lineId,
      localizationId: localizationId,
      localizationAction: action,
      voiceSlotId: slotId,
      buildStatus: switch (json['build_status']) {
        'blocked' => AuthoringRevision3DialogBuildStatus.blocked,
        _ => throw const FormatException(
          'revision-3 dialog-line response grants unsupported build authority',
        ),
      },
      runtimeStatus: switch (json['runtime_status']) {
        'runtime_unqualified' =>
          AuthoringRevision3DialogRuntimeStatus.runtimeUnqualified,
        _ => throw const FormatException(
          'revision-3 dialog-line response grants unsupported runtime authority',
        ),
      },
      topicAuthority: switch (json['topic_authority']) {
        'not_granted' => AuthoringRevision3DialogTopicAuthority.notGranted,
        _ => throw const FormatException(
          'revision-3 dialog-line response grants unsupported topic authority',
        ),
      },
      publicationStatus: switch (json['publication_status']) {
        'not_supported' =>
          AuthoringRevision3DialogPublicationStatus.notSupported,
        _ => throw const FormatException(
          'revision-3 dialog-line response grants unsupported native publication authority',
        ),
      },
    );
  }
}

AuthoringRevision3DialogLocalizationIntentV1 _dialogLocalizationIntent(
  Object? value,
) {
  final json = _authoringRequiredObject(
    value,
    'revision-3 dialog-line localization intent',
  );
  switch (json['mode']) {
    case 'create':
      const fields = <String>[
        'mode',
        'localization_id',
        'display_name',
        'loc_id',
        'texts',
      ];
      _authoringExactFields(
        json,
        fields.toSet(),
        'revision-3 dialog-line create localization intent',
      );
      _authoringRevision3VoiceRequireFieldOrder(
        json,
        fields,
        'dialog-line create localization intent',
      );
      final rawTexts = _authoringRequiredObject(
        json['texts'],
        'revision-3 dialog-line localization texts',
      );
      final texts = <String, String>{};
      String? previous;
      for (final entry in rawTexts.entries) {
        final locale = _authoringRevision3VoiceLocale(entry.key);
        if (previous != null && previous.compareTo(locale) >= 0) {
          throw const FormatException(
            'revision-3 dialog-line localization texts are not sorted uniquely',
          );
        }
        previous = locale;
        if (entry.value is! String) {
          throw const FormatException(
            'revision-3 dialog-line localization text is not a string',
          );
        }
        texts[locale] = entry.value! as String;
      }
      return AuthoringRevision3DialogLocalizationCreateIntentV1(
        localizationId: _dialogString(json, 'localization_id', maxBytes: 32),
        displayName: _dialogString(
          json,
          'display_name',
          maxBytes: _maxAuthoringRevision3DialogDisplayNameBytes,
        ),
        locId: _dialogString(json, 'loc_id', maxBytes: 1024),
        texts: texts,
      );
    case 'reuse_exact':
      const fields = <String>[
        'mode',
        'localization_id',
        'expected_localization_revision',
        'expected_loc_id',
      ];
      _authoringExactFields(
        json,
        fields.toSet(),
        'revision-3 dialog-line reuse localization intent',
      );
      _authoringRevision3VoiceRequireFieldOrder(
        json,
        fields,
        'dialog-line reuse localization intent',
      );
      return AuthoringRevision3DialogLocalizationReuseExactIntentV1(
        localizationId: _dialogString(json, 'localization_id', maxBytes: 32),
        expectedLocalizationRevision: _authoringRequiredInt(
          json,
          'expected_localization_revision',
          max: _maxAuthoringRevision3VoiceBasisRevision,
        ),
        expectedLocId: _dialogString(json, 'expected_loc_id', maxBytes: 1024),
      );
    default:
      throw const FormatException(
        'revision-3 dialog-line localization intent has an invalid mode',
      );
  }
}

AuthoringRevision3DialogEmptyVoiceSlotIntentV1 _dialogVoiceSlotIntent(
  Object? value,
) {
  final json = _authoringRequiredObject(
    value,
    'revision-3 dialog-line Voice slot intent',
  );
  const fields = <String>['slot_id', 'locale', 'display_name'];
  _authoringExactFields(
    json,
    fields.toSet(),
    'revision-3 dialog-line Voice slot intent',
  );
  _authoringRevision3VoiceRequireFieldOrder(
    json,
    fields,
    'dialog-line Voice slot intent',
  );
  return AuthoringRevision3DialogEmptyVoiceSlotIntentV1(
    slotId: _dialogString(json, 'slot_id', maxBytes: 32),
    locale: _dialogString(json, 'locale', maxBytes: 35),
    displayName: _dialogString(
      json,
      'display_name',
      maxBytes: _maxAuthoringRevision3DialogDisplayNameBytes,
    ),
  );
}

String _dialogString(
  Map<String, Object?> json,
  String field, {
  required int maxBytes,
}) => _authoringRevision3VoiceString(json, field, maxBytes: maxBytes);

String _dialogEntityId(String value, String context) {
  final id = _authoringEntityId(value, context);
  if (id == '00000000000000000000000000000000') {
    throw FormatException('revision-3 dialog-line $context must not be zero');
  }
  return id;
}

int _dialogRevision(int value, String context) {
  if (value < 0 || value > _maxAuthoringRevision3VoiceBasisRevision) {
    throw FormatException('revision-3 dialog-line $context is invalid');
  }
  return value;
}

String _dialogDisplayName(String value, String context) {
  if (value.trim() != value ||
      value.isEmpty ||
      utf8.encode(value).length >
          _maxAuthoringRevision3DialogDisplayNameBytes ||
      value.runes.any(_authoringRevision3VoiceControl)) {
    throw FormatException('revision-3 dialog-line $context is invalid');
  }
  return value;
}

String _dialogAuthoredIdentity(String value) {
  final units = value.codeUnits;
  if (value.trim() != value ||
      value.isEmpty ||
      units.length > _maxAuthoringRevision3DialogIdentityBytes ||
      units.any(
        (unit) => unit < 0x21 || unit > 0x7e || unit == 0x22 || unit == 0x5c,
      )) {
    throw const FormatException(
      'revision-3 dialog-line authored identity is invalid',
    );
  }
  return value;
}

String _dialogLocId(String value) {
  if (!authoringRevision3VoiceArchiveBasenameStemIsSafe(value)) {
    throw const FormatException(
      'revision-3 dialog-line localization identity is not a safe Voice basename stem',
    );
  }
  return value;
}

String _dialogLocalizationReadLocId(String value) {
  final validated = _dialogLocalizationReadString(
    value,
    'localization identity',
    maxBytes: _maxAuthoringRevision3DialogLocalizationReadLocIdBytes,
  );
  if (validated.contains('\u0000')) {
    throw const FormatException(
      'revision-3 dialog localization identity contains NUL',
    );
  }
  return validated;
}

/// Mirrors Rust `char::is_whitespace` (Unicode White_Space). Dart's
/// `String.trim` additionally treats U+FEFF as whitespace, so it cannot be
/// used to validate the native flag contract exactly.
bool _dialogLocalizationReadRuneIsRustWhitespace(int rune) =>
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
    rune == 0x3000;

bool _dialogLocalizationReadHasNonWhitespace(String value) => value.runes.any(
  (rune) => !_dialogLocalizationReadRuneIsRustWhitespace(rune),
);

String _dialogLocalizationReadString(
  Object? value,
  String context, {
  required int maxBytes,
  bool allowEmpty = false,
}) {
  if (value is! String || (!allowEmpty && value.isEmpty)) {
    throw FormatException('revision-3 dialog $context is not bounded text');
  }
  if (value.isEmpty) return value;
  try {
    _authoringRevision3RequestString(value, context, maxBytes);
  } on ArgumentError {
    throw FormatException('revision-3 dialog $context is not bounded UTF-8');
  }
  return value;
}

Map<String, String> _dialogTexts(Map<String, String> input) {
  if (input.isEmpty || input.length > _maxAuthoringRevision3DialogTexts) {
    throw const FormatException(
      'revision-3 dialog-line localization texts are empty or too large',
    );
  }
  final keys = input.keys.toList(growable: false)..sort();
  final result = <String, String>{};
  var total = 0;
  for (final rawLocale in keys) {
    final locale = _authoringRevision3VoiceLocale(rawLocale);
    if (locale != rawLocale || result.containsKey(locale)) {
      throw const FormatException(
        'revision-3 dialog-line localization locales are not canonical and unique',
      );
    }
    final text = input[rawLocale]!;
    final bytes = utf8.encode(text).length;
    total += bytes;
    if (text.trim().isEmpty ||
        text.contains('\u0000') ||
        bytes > _maxAuthoringRevision3DialogTextBytes ||
        total > _maxAuthoringRevision3DialogTextsBytes) {
      throw const FormatException(
        'revision-3 dialog-line localization text is empty or exceeds its safe limit',
      );
    }
    result[locale] = text;
  }
  return Map<String, String>.unmodifiable(result);
}

void _dialogRequireExactCandidate(
  Map<String, Object?> basis,
  Map<String, Object?> candidate, {
  required AuthoringRevision3DialogLineEntryRequestV1 request,
}) {
  final expected = _authoringRevision3VoiceCloneObject(
    basis,
    'revision-3 dialog-line expected candidate',
  );
  expected['revision'] = request.expectedRevision + 1;
  final entities = _authoringRequiredObject(
    expected['entities'],
    'revision-3 dialog-line expected entities',
  );
  if (entities.containsKey(request.lineId) ||
      (request.localization
              is AuthoringRevision3DialogLocalizationCreateIntentV1 &&
          entities.containsKey(request.localization.localizationId)) ||
      (request.voiceSlot != null &&
          entities.containsKey(request.voiceSlot!.slotId))) {
    throw const FormatException(
      'revision-3 dialog-line request collides with its exact basis',
    );
  }

  final locales = (expected['authoring_locales'] as List<Object?>).map((value) {
    if (value is! String) {
      throw const FormatException(
        'revision-3 dialog-line basis locales are invalid',
      );
    }
    return _authoringRevision3VoiceLocale(value);
  }).toSet();

  final localization = request.localization;
  switch (localization) {
    case AuthoringRevision3DialogLocalizationCreateIntentV1():
      for (final locale in localization.texts.keys) {
        locales.add(locale);
      }
      entities[localization.localizationId] = <String, Object?>{
        'id': localization.localizationId,
        'display_name': localization.displayName,
        'origin': <String, Object?>{
          'type': 'new',
          'authored_runtime_id': localization.locId,
        },
        'revision': 0,
        'payload': <String, Object?>{
          'kind': 'localization_entry',
          'data': <String, Object?>{
            'loc_id': localization.locId,
            'texts': localization.texts,
          },
        },
      };
    case AuthoringRevision3DialogLocalizationReuseExactIntentV1():
      final existing = _dialogExistingEntity(
        entities,
        localization.localizationId,
        expectedKind: 'localization_entry',
        context: 'reused localization',
      );
      final revision = _authoringRequiredInt(
        existing,
        'revision',
        max: _maxAuthoringRevision3VoiceBasisRevision,
      );
      final payload = _authoringRequiredObject(
        existing['payload'],
        'revision-3 dialog-line reused localization payload',
      );
      final data = _authoringRequiredObject(
        payload['data'],
        'revision-3 dialog-line reused localization data',
      );
      final locId = _authoringRequiredString(data, 'loc_id', maxBytes: 1024);
      if (revision != localization.expectedLocalizationRevision ||
          locId != localization.expectedLocId) {
        throw const FormatException(
          'revision-3 dialog-line reused localization is stale',
        );
      }
      for (final entityValue in entities.values) {
        final entity = _authoringRequiredObject(
          entityValue,
          'revision-3 dialog-line basis entity',
        );
        final entityPayload = _authoringRequiredObject(
          entity['payload'],
          'revision-3 dialog-line basis entity payload',
        );
        if (entityPayload['kind'] != 'dialog_line') continue;
        final lineData = _authoringRequiredObject(
          entityPayload['data'],
          'revision-3 dialog-line basis line data',
        );
        final refValue = lineData['localization'];
        if (refValue is! Map) continue;
        final ref = refValue.cast<String, Object?>();
        if (ref['project_id'] == request.expectedProjectId &&
            ref['id'] == localization.localizationId &&
            ref['expected_kind'] == 'localization_entry') {
          throw const FormatException(
            'revision-3 dialog-line reused localization is already owned by another line',
          );
        }
      }
      final rawTexts = _authoringRequiredObject(
        data['texts'],
        'revision-3 dialog-line reused localization texts',
      );
      final requestedSlot = request.voiceSlot;
      if (requestedSlot != null) {
        final slotText = rawTexts[requestedSlot.locale];
        if (slotText is! String ||
            !_dialogLocalizationReadHasNonWhitespace(slotText)) {
          throw const FormatException(
            'revision-3 dialog-line reused localization has no text for its Voice slot locale',
          );
        }
      }
      for (final locale in rawTexts.keys) {
        locales.add(_authoringRevision3VoiceLocale(locale));
      }
  }

  final slot = request.voiceSlot;
  final voiceSlots = <String, Object?>{};
  if (slot != null) {
    locales.add(slot.locale);
    voiceSlots[slot.locale] = _dialogTypedRef(
      request.expectedProjectId,
      slot.slotId,
      'voice_slot',
    );
    entities[slot.slotId] = <String, Object?>{
      'id': slot.slotId,
      'display_name': slot.displayName,
      'origin': <String, Object?>{
        'type': 'generated',
        'generator_id': _authoringRevision3VoiceSlotGeneratorId,
        'generator_version': _authoringRevision3VoiceSlotGeneratorVersion,
        'owner': _dialogTypedRef(
          request.expectedProjectId,
          request.lineId,
          'dialog_line',
        ),
      },
      'revision': 0,
      'payload': <String, Object?>{
        'kind': 'voice_slot',
        'data': <String, Object?>{
          'locale': slot.locale,
          'target_resolution': <String, Object?>{'state': 'unresolved'},
          'candidates': <Object?>[],
        },
      },
    };
  }
  entities[request.lineId] = <String, Object?>{
    'id': request.lineId,
    'display_name': request.lineDisplayName,
    'origin': <String, Object?>{
      'type': 'new',
      'authored_runtime_id': request.lineAuthoredIdentity,
    },
    'revision': 0,
    'payload': <String, Object?>{
      'kind': 'dialog_line',
      'data': <String, Object?>{
        'localization': _dialogTypedRef(
          request.expectedProjectId,
          localization.localizationId,
          'localization_entry',
        ),
        if (request.speakerHint != null) 'speaker_hint': request.speakerHint,
        'voice_slots': voiceSlots,
      },
    },
  };

  final sortedLocales = locales.toList(growable: false)..sort();
  expected['authoring_locales'] = sortedLocales;
  final sortedEntityIds = entities.keys.toList(growable: false)..sort();
  expected['entities'] = <String, Object?>{
    for (final id in sortedEntityIds) id: entities[id],
  };
  if (jsonEncode(expected) != jsonEncode(candidate)) {
    throw const FormatException(
      'revision-3 dialog-line candidate contains an unverified semantic delta',
    );
  }
}

Map<String, Object?> _dialogExistingEntity(
  Map<String, Object?> entities,
  String id, {
  required String expectedKind,
  required String context,
}) {
  final entity = _authoringRequiredObject(
    entities[id],
    'revision-3 dialog-line $context',
  );
  if (_authoringRequiredString(entity, 'id', maxBytes: 32) != id) {
    throw FormatException('revision-3 dialog-line $context has a false ID');
  }
  final payload = _authoringRequiredObject(
    entity['payload'],
    'revision-3 dialog-line $context payload',
  );
  if (payload['kind'] != expectedKind) {
    throw FormatException('revision-3 dialog-line $context has a wrong kind');
  }
  return entity;
}

Map<String, Object?> _dialogTypedRef(
  String projectId,
  String id,
  String expectedKind,
) => <String, Object?>{
  'project_id': projectId,
  'id': id,
  'expected_kind': expectedKind,
};
