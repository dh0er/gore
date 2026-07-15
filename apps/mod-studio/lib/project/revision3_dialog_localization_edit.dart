part of '../core/mod_ffi.dart';

const _maxAuthoringRevision3DialogLocalizationEditRequestBytes = 1024 * 1024;
const _maxAuthoringRevision3DialogLocalizationEditTexts = 1000;
const _maxAuthoringRevision3DialogLocalizationEditTextBytes = 64 * 1024;
const _maxAuthoringRevision3DialogLocalizationEditTextsBytes = 512 * 1024;
const _maxAuthoringRevision3DialogLocalizationEditBacklinks = 1000;
const _maxAuthoringRevision3DialogLocalizationEditDisplayNameBytes = 256;

enum AuthoringRevision3DialogLocalizationEditContentAuthority {
  readOnlyExactCurrentLocalizationEditSeed,
}

enum AuthoringRevision3DialogLocalizationEditSeedBuildStatus { notEvaluated }

enum AuthoringRevision3DialogLocalizationEditBuildStatus { blocked }

enum AuthoringRevision3DialogLocalizationEditRuntimeStatus {
  runtimeUnqualified,
}

enum AuthoringRevision3DialogLocalizationEditTopicAuthority { notGranted }

enum AuthoringRevision3DialogLocalizationEditSeedPublicationStatus {
  notApplicable,
}

enum AuthoringRevision3DialogLocalizationEditPublicationStatus { notSupported }

/// Exact-current, read-only selector for one authored LocalizationEntry.
final class AuthoringRevision3DialogLocalizationEditSeedRequestV1 {
  AuthoringRevision3DialogLocalizationEditSeedRequestV1({
    required this.expectedHead,
    required String localizationId,
    required int expectedLocalizationRevision,
    required String expectedLocId,
  }) : localizationId = _dialogEntityId(
         localizationId,
         'localization-edit seed localization ID',
       ),
       expectedLocalizationRevision = _dialogRevision(
         expectedLocalizationRevision,
         'localization-edit seed localization revision',
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

/// One exact locale/text row and its Voice editing constraints.
final class AuthoringRevision3DialogLocalizationEditLocaleSeed {
  const AuthoringRevision3DialogLocalizationEditLocaleSeed._({
    required this.locale,
    required this.text,
    required this.voiceSlotPresent,
    required this.candidateCount,
  });

  final String locale;
  final String text;
  final bool voiceSlotPresent;
  final int candidateCount;

  factory AuthoringRevision3DialogLocalizationEditLocaleSeed._fromJson(
    Object? value,
    String context,
  ) {
    final json = _authoringRequiredObject(value, context);
    _authoringExactFields(json, const <String>{
      'locale',
      'text',
      'voice_slot_present',
      'candidate_count',
    }, context);
    final locale = _authoringRevision3VoiceLocale(
      _authoringRequiredString(json, 'locale', maxBytes: 64),
    );
    final textValue = json['text'];
    if (textValue is! String ||
        utf8.encode(textValue).length >
            _maxAuthoringRevision3DialogLocalizationEditTextBytes) {
      throw FormatException('$context text is not a bounded string');
    }
    final text = textValue;
    if (text.contains('\u0000')) {
      throw FormatException('$context text contains a forbidden NUL');
    }
    final voiceSlotPresent = json['voice_slot_present'];
    if (voiceSlotPresent is! bool) {
      throw FormatException('$context voice_slot_present is not a boolean');
    }
    final candidateCount = _authoringRequiredInt(
      json,
      'candidate_count',
      max: _maxAuthoringSignedJsonInteger,
    );
    if (!voiceSlotPresent && candidateCount != 0) {
      throw FormatException(
        '$context reports Voice candidates without a VoiceSlot',
      );
    }
    return AuthoringRevision3DialogLocalizationEditLocaleSeed._(
      locale: locale,
      text: text,
      voiceSlotPresent: voiceSlotPresent,
      candidateCount: candidateCount,
    );
  }
}

/// Friendly DialogLine backlink facts returned by the exact native seed read.
final class AuthoringRevision3DialogLocalizationLineBacklink {
  const AuthoringRevision3DialogLocalizationLineBacklink._({
    required this.lineId,
    required this.lineRevision,
    required this.displayName,
    required this.speakerHint,
    required this.voiceSlotLocales,
  });

  final String lineId;
  final int lineRevision;
  final String displayName;
  final String? speakerHint;
  final List<String> voiceSlotLocales;

  factory AuthoringRevision3DialogLocalizationLineBacklink._fromJson(
    Object? value,
    String context,
  ) {
    final json = _authoringRequiredObject(value, context);
    _authoringExactFields(json, const <String>{
      'line_id',
      'line_revision',
      'display_name',
      'speaker_hint',
      'voice_slot_locales',
    }, context);
    final speakerValue = json['speaker_hint'];
    if (speakerValue != null && speakerValue is! String) {
      throw FormatException('$context speaker_hint is not a nullable string');
    }
    final speakerHint = speakerValue == null
        ? null
        : _dialogDisplayName(speakerValue as String, '$context speaker hint');
    final rawLocales = json['voice_slot_locales'];
    if (rawLocales is! List<Object?> ||
        rawLocales.length > _maxAuthoringRevision3DialogLocalizationEditTexts) {
      throw FormatException('$context voice_slot_locales is invalid');
    }
    final locales = <String>[];
    String? previous;
    for (final value in rawLocales) {
      if (value is! String) {
        throw FormatException('$context VoiceSlot locale is not a string');
      }
      final locale = _authoringRevision3VoiceLocale(value);
      if (previous != null && previous.compareTo(locale) >= 0) {
        throw FormatException(
          '$context VoiceSlot locales are not unique canonical order',
        );
      }
      previous = locale;
      locales.add(locale);
    }
    return AuthoringRevision3DialogLocalizationLineBacklink._(
      lineId: _dialogEntityId(
        _authoringRequiredString(json, 'line_id', maxBytes: 32),
        '$context line ID',
      ),
      lineRevision: _dialogRevision(
        _authoringRequiredInt(
          json,
          'line_revision',
          max: _maxAuthoringStoryBaseRevision,
        ),
        '$context line revision',
      ),
      displayName: _dialogDisplayName(
        _authoringRequiredString(
          json,
          'display_name',
          maxBytes:
              _maxAuthoringRevision3DialogLocalizationEditDisplayNameBytes,
        ),
        '$context display name',
      ),
      speakerHint: speakerHint,
      voiceSlotLocales: List<String>.unmodifiable(locales),
    );
  }
}

/// Fully bounded, exact-current edit seed. It grants no mutation authority.
final class AuthoringRevision3DialogLocalizationEditSeed {
  const AuthoringRevision3DialogLocalizationEditSeed._({
    required this.head,
    required this.projectId,
    required this.projectRevision,
    required this.localizationId,
    required this.localizationRevision,
    required this.locId,
    required this.locales,
    required this.lineBacklinks,
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
  final List<AuthoringRevision3DialogLocalizationEditLocaleSeed> locales;
  final List<AuthoringRevision3DialogLocalizationLineBacklink> lineBacklinks;
  final AuthoringRevision3DialogLocalizationEditContentAuthority
  contentAuthority;
  final AuthoringRevision3DialogLocalizationEditSeedBuildStatus buildStatus;
  final AuthoringRevision3DialogLocalizationEditRuntimeStatus runtimeStatus;
  final AuthoringRevision3DialogLocalizationEditSeedPublicationStatus
  publicationStatus;

  factory AuthoringRevision3DialogLocalizationEditSeed.fromJson(
    Map<String, Object?> json, {
    required AuthoringRevision3DialogLocalizationEditSeedRequestV1 request,
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
      'line_backlinks',
      'content_authority',
      'build_status',
      'runtime_status',
      'publication_status',
    }, 'revision-3 localization-edit seed response');
    if (json['ok'] != true || json['outcome'] != 'read_only') {
      throw const FormatException(
        'revision-3 localization-edit seed is not an exact read-only result',
      );
    }
    final head = AuthoringWorkingHead.fromCanonicalJson(
      _authoringRequiredString(
        json,
        'head_json',
        maxBytes: _maxAuthoringHeadJsonBytes,
      ),
    );
    final projectId = _dialogEntityId(
      _authoringRequiredString(json, 'project_id', maxBytes: 32),
      'localization-edit seed project ID',
    );
    final projectRevision = _dialogRevision(
      _authoringRequiredInt(
        json,
        'project_revision',
        max: _maxAuthoringSignedJsonInteger,
      ),
      'localization-edit seed project revision',
    );
    final localizationId = _dialogEntityId(
      _authoringRequiredString(json, 'localization_id', maxBytes: 32),
      'localization-edit seed localization ID',
    );
    final localizationRevision = _dialogRevision(
      _authoringRequiredInt(
        json,
        'localization_revision',
        max: _maxAuthoringStoryBaseRevision,
      ),
      'localization-edit seed localization revision',
    );
    final locId = _dialogLocalizationReadLocId(
      _authoringRequiredString(
        json,
        'loc_id',
        maxBytes: _maxAuthoringRevision3DialogLocalizationReadLocIdBytes,
      ),
    );
    if (head.canonicalJson != request.expectedHead.canonicalJson ||
        localizationId != request.localizationId ||
        localizationRevision != request.expectedLocalizationRevision ||
        locId != request.expectedLocId) {
      throw const FormatException(
        'revision-3 localization-edit seed disagrees with its exact request',
      );
    }

    final rawLocales = json['locales'];
    if (rawLocales is! List<Object?> ||
        rawLocales.isEmpty ||
        rawLocales.length > _maxAuthoringRevision3DialogLocalizationEditTexts) {
      throw const FormatException(
        'revision-3 localization-edit seed locales are invalid',
      );
    }
    final locales = <AuthoringRevision3DialogLocalizationEditLocaleSeed>[];
    String? previousLocale;
    var textBytes = 0;
    var hasNonblank = false;
    for (var index = 0; index < rawLocales.length; index++) {
      final locale =
          AuthoringRevision3DialogLocalizationEditLocaleSeed._fromJson(
            rawLocales[index],
            'revision-3 localization-edit seed locale $index',
          );
      if (previousLocale != null &&
          previousLocale.compareTo(locale.locale) >= 0) {
        throw const FormatException(
          'revision-3 localization-edit seed locales are not unique canonical order',
        );
      }
      previousLocale = locale.locale;
      textBytes += utf8.encode(locale.text).length;
      if (textBytes > _maxAuthoringRevision3DialogLocalizationEditTextsBytes) {
        throw const FormatException(
          'revision-3 localization-edit seed text budget is exceeded',
        );
      }
      hasNonblank |= locale.text.trim().isNotEmpty;
      locales.add(locale);
    }
    if (!hasNonblank) {
      throw const FormatException(
        'revision-3 localization-edit seed contains no nonblank text',
      );
    }

    final rawBacklinks = json['line_backlinks'];
    if (rawBacklinks is! List<Object?> ||
        rawBacklinks.length >
            _maxAuthoringRevision3DialogLocalizationEditBacklinks) {
      throw const FormatException(
        'revision-3 localization-edit seed backlinks are invalid',
      );
    }
    final backlinks = <AuthoringRevision3DialogLocalizationLineBacklink>[];
    String? previousLineId;
    final expectedVoiceLocales = <String>{
      for (final locale in locales)
        if (locale.voiceSlotPresent) locale.locale,
    };
    final backlinkVoiceLocales = <String>{};
    for (var index = 0; index < rawBacklinks.length; index++) {
      final backlink =
          AuthoringRevision3DialogLocalizationLineBacklink._fromJson(
            rawBacklinks[index],
            'revision-3 localization-edit seed backlink $index',
          );
      if (previousLineId != null &&
          previousLineId.compareTo(backlink.lineId) >= 0) {
        throw const FormatException(
          'revision-3 localization-edit seed backlinks are not unique canonical order',
        );
      }
      previousLineId = backlink.lineId;
      backlinkVoiceLocales.addAll(backlink.voiceSlotLocales);
      backlinks.add(backlink);
    }
    if (!_dialogLocalizationEditSameSet(
      expectedVoiceLocales,
      backlinkVoiceLocales,
    )) {
      throw const FormatException(
        'revision-3 localization-edit seed VoiceSlot facts disagree',
      );
    }

    return AuthoringRevision3DialogLocalizationEditSeed._(
      head: head,
      projectId: projectId,
      projectRevision: projectRevision,
      localizationId: localizationId,
      localizationRevision: localizationRevision,
      locId: locId,
      locales: List.unmodifiable(locales),
      lineBacklinks: List.unmodifiable(backlinks),
      contentAuthority: switch (json['content_authority']) {
        'read_only_exact_current_localization_edit_seed' =>
          AuthoringRevision3DialogLocalizationEditContentAuthority
              .readOnlyExactCurrentLocalizationEditSeed,
        _ => throw const FormatException(
          'revision-3 localization-edit seed grants unsupported content authority',
        ),
      },
      buildStatus: switch (json['build_status']) {
        'not_evaluated' =>
          AuthoringRevision3DialogLocalizationEditSeedBuildStatus.notEvaluated,
        _ => throw const FormatException(
          'revision-3 localization-edit seed grants unsupported build authority',
        ),
      },
      runtimeStatus: switch (json['runtime_status']) {
        'runtime_unqualified' =>
          AuthoringRevision3DialogLocalizationEditRuntimeStatus
              .runtimeUnqualified,
        _ => throw const FormatException(
          'revision-3 localization-edit seed grants unsupported runtime authority',
        ),
      },
      publicationStatus: switch (json['publication_status']) {
        'not_applicable' =>
          AuthoringRevision3DialogLocalizationEditSeedPublicationStatus
              .notApplicable,
        _ => throw const FormatException(
          'revision-3 localization-edit seed grants unsupported publication authority',
        ),
      },
    );
  }
}

/// Canonical, complete replacement request for one exact LocalizationEntry.
final class AuthoringRevision3DialogLocalizationEditRequestV1 {
  const AuthoringRevision3DialogLocalizationEditRequestV1._({
    required this.canonicalJson,
    required this.expectedHead,
    required this.expectedProjectId,
    required this.expectedRevision,
    required this.expectedTargetCanonicalJson,
    required this.localizationId,
    required this.expectedLocalizationRevision,
    required this.expectedLocId,
    required this.texts,
  });

  factory AuthoringRevision3DialogLocalizationEditRequestV1.forProject({
    required AuthoringWorkingHead expectedHead,
    required String currentProjectJson,
    required String localizationId,
    required int expectedLocalizationRevision,
    required String expectedLocId,
    required Map<String, String> texts,
  }) {
    final current = _authoringRequireCanonicalRevision3ProjectJson(
      currentProjectJson,
    );
    final canonicalTexts = _dialogLocalizationEditTexts(texts);
    return AuthoringRevision3DialogLocalizationEditRequestV1.fromCanonicalJson(
      jsonEncode(<String, Object?>{
        'expected_head': jsonDecode(expectedHead.canonicalJson),
        'expected_project_id': current.projectId,
        'expected_revision': current.revision,
        'expected_target': current.project['target'],
        'localization_id': localizationId,
        'expected_localization_revision': expectedLocalizationRevision,
        'expected_loc_id': expectedLocId,
        'texts': canonicalTexts,
      }),
      currentProjectJson: currentProjectJson,
    );
  }

  final String canonicalJson;
  final AuthoringWorkingHead expectedHead;
  final String expectedProjectId;
  final int expectedRevision;
  final String expectedTargetCanonicalJson;
  final String localizationId;
  final int expectedLocalizationRevision;
  final String expectedLocId;
  final Map<String, String> texts;

  factory AuthoringRevision3DialogLocalizationEditRequestV1.fromCanonicalJson(
    String value, {
    required String currentProjectJson,
  }) {
    try {
      _authoringRevision3RequestString(
        value,
        'dialogLocalizationEditRequestJson',
        _maxAuthoringRevision3DialogLocalizationEditRequestBytes,
      );
    } on ArgumentError {
      throw const FormatException(
        'revision-3 localization-edit request is not bounded UTF-8',
      );
    }
    final request = _authoringDecodeDuplicateSafeObject(
      value,
      'revision-3 localization-edit request',
    );
    const fields = <String>[
      'expected_head',
      'expected_project_id',
      'expected_revision',
      'expected_target',
      'localization_id',
      'expected_localization_revision',
      'expected_loc_id',
      'texts',
    ];
    _authoringExactFields(
      request,
      fields.toSet(),
      'revision-3 localization-edit request',
    );
    _authoringRevision3VoiceRequireFieldOrder(
      request,
      fields,
      'localization-edit request',
    );
    if (jsonEncode(request) != value) {
      throw const FormatException(
        'revision-3 localization-edit request is not canonical',
      );
    }
    final rawTexts = _authoringRequiredObject(
      request['texts'],
      'revision-3 localization-edit texts',
    );
    final texts = _dialogLocalizationEditTexts(
      rawTexts.map(
        (key, value) => MapEntry(
          key,
          value is String
              ? value
              : throw const FormatException(
                  'revision-3 localization-edit text is not a string',
                ),
        ),
      ),
    );
    if (rawTexts.keys.toList(growable: false).join('\u0000') !=
        texts.keys.join('\u0000')) {
      throw const FormatException(
        'revision-3 localization-edit texts are not in canonical order',
      );
    }
    final parsed = AuthoringRevision3DialogLocalizationEditRequestV1._(
      canonicalJson: value,
      expectedHead: AuthoringWorkingHead.fromCanonicalJson(
        jsonEncode(
          _authoringRequiredObject(
            request['expected_head'],
            'revision-3 localization-edit expected head',
          ),
        ),
      ),
      expectedProjectId: _dialogEntityId(
        _authoringRequiredString(request, 'expected_project_id', maxBytes: 32),
        'localization-edit project ID',
      ),
      expectedRevision: _dialogRevision(
        _authoringRequiredInt(
          request,
          'expected_revision',
          max: _maxAuthoringStoryBaseRevision,
        ),
        'localization-edit project revision',
      ),
      expectedTargetCanonicalJson: jsonEncode(
        _authoringRevision3VoiceGeneration(
          request['expected_target'],
          'localization-edit target',
        ),
      ),
      localizationId: _dialogEntityId(
        _authoringRequiredString(request, 'localization_id', maxBytes: 32),
        'localization-edit localization ID',
      ),
      expectedLocalizationRevision: _dialogRevision(
        _authoringRequiredInt(
          request,
          'expected_localization_revision',
          max: _maxAuthoringStoryBaseRevision,
        ),
        'localization-edit localization revision',
      ),
      expectedLocId: _dialogLocalizationReadLocId(
        _authoringRequiredString(
          request,
          'expected_loc_id',
          maxBytes: _maxAuthoringRevision3DialogLocalizationReadLocIdBytes,
        ),
      ),
      texts: texts,
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
        'revision-3 localization-edit request does not bind the exact current project',
      );
    }
    final currentTexts = _dialogLocalizationEditCurrentTexts(
      current.project,
      localizationId: localizationId,
      expectedRevision: expectedLocalizationRevision,
      expectedLocId: expectedLocId,
    );
    if (_dialogLocalizationEditSameMap(currentTexts, texts)) {
      throw const FormatException(
        'revision-3 localization-edit request does not change any text',
      );
    }
  }
}

/// Fully reopened unpublished localization-edit candidate.
final class AuthoringRevision3DialogLocalizationEditPreparation {
  const AuthoringRevision3DialogLocalizationEditPreparation._({
    required this.basisHead,
    required this.head,
    required this.projectJson,
    required this.projectId,
    required this.revision,
    required this.localizationId,
    required this.localizationRevision,
    required this.addedLocales,
    required this.removedLocales,
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
  final String localizationId;
  final int localizationRevision;
  final List<String> addedLocales;
  final List<String> removedLocales;
  final AuthoringRevision3DialogLocalizationEditBuildStatus buildStatus;
  final AuthoringRevision3DialogLocalizationEditRuntimeStatus runtimeStatus;
  final AuthoringRevision3DialogLocalizationEditTopicAuthority topicAuthority;
  final AuthoringRevision3DialogLocalizationEditPublicationStatus
  publicationStatus;

  factory AuthoringRevision3DialogLocalizationEditPreparation.fromJson(
    Map<String, Object?> json, {
    required String currentProjectJson,
    required AuthoringRevision3DialogLocalizationEditRequestV1 request,
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
      'localization_id',
      'localization_revision',
      'added_locales',
      'removed_locales',
      'build_status',
      'runtime_status',
      'topic_authority',
      'publication_status',
    }, 'revision-3 localization-edit preparation response');
    if (json['ok'] != true || json['outcome'] != 'prepared_unpublished') {
      throw const FormatException(
        'revision-3 localization-edit response is not an unpublished preparation',
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
        'revision-3 localization-edit response has an invalid head transition',
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
      'localization-edit response project ID',
    );
    final revision = _dialogRevision(
      _authoringRequiredInt(
        json,
        'revision',
        min: 1,
        max: _maxAuthoringSignedJsonInteger,
      ),
      'localization-edit response project revision',
    );
    final localizationId = _dialogEntityId(
      _authoringRequiredString(json, 'localization_id', maxBytes: 32),
      'localization-edit response localization ID',
    );
    final localizationRevision = _dialogRevision(
      _authoringRequiredInt(
        json,
        'localization_revision',
        min: 1,
        max: _maxAuthoringSignedJsonInteger,
      ),
      'localization-edit response localization revision',
    );
    final oldTexts = _dialogLocalizationEditCurrentTexts(
      basis.project,
      localizationId: request.localizationId,
      expectedRevision: request.expectedLocalizationRevision,
      expectedLocId: request.expectedLocId,
    );
    final expectedAdded = request.texts.keys
        .where((locale) => !oldTexts.containsKey(locale))
        .toList(growable: false);
    final expectedRemoved = oldTexts.keys
        .where((locale) => !request.texts.containsKey(locale))
        .toList(growable: false);
    final addedLocales = _dialogLocalizationEditLocaleList(
      json['added_locales'],
      'localization-edit added locales',
    );
    final removedLocales = _dialogLocalizationEditLocaleList(
      json['removed_locales'],
      'localization-edit removed locales',
    );
    if (projectId != basis.projectId ||
        projectId != candidate.projectId ||
        revision != basis.revision + 1 ||
        revision != candidate.revision ||
        localizationId != request.localizationId ||
        localizationRevision != request.expectedLocalizationRevision + 1 ||
        !_dialogLocalizationEditSameList(addedLocales, expectedAdded) ||
        !_dialogLocalizationEditSameList(removedLocales, expectedRemoved)) {
      throw const FormatException(
        'revision-3 localization-edit response disagrees with its exact request',
      );
    }
    _dialogLocalizationEditRequireExactCandidate(
      basis.project,
      candidate.project,
      request: request,
    );
    return AuthoringRevision3DialogLocalizationEditPreparation._(
      basisHead: basisHead,
      head: head,
      projectJson: projectJson,
      projectId: projectId,
      revision: revision,
      localizationId: localizationId,
      localizationRevision: localizationRevision,
      addedLocales: List.unmodifiable(addedLocales),
      removedLocales: List.unmodifiable(removedLocales),
      buildStatus: switch (json['build_status']) {
        'blocked' =>
          AuthoringRevision3DialogLocalizationEditBuildStatus.blocked,
        _ => throw const FormatException(
          'revision-3 localization-edit response grants unsupported build authority',
        ),
      },
      runtimeStatus: switch (json['runtime_status']) {
        'runtime_unqualified' =>
          AuthoringRevision3DialogLocalizationEditRuntimeStatus
              .runtimeUnqualified,
        _ => throw const FormatException(
          'revision-3 localization-edit response grants unsupported runtime authority',
        ),
      },
      topicAuthority: switch (json['topic_authority']) {
        'not_granted' =>
          AuthoringRevision3DialogLocalizationEditTopicAuthority.notGranted,
        _ => throw const FormatException(
          'revision-3 localization-edit response grants unsupported topic authority',
        ),
      },
      publicationStatus: switch (json['publication_status']) {
        'not_supported' =>
          AuthoringRevision3DialogLocalizationEditPublicationStatus
              .notSupported,
        _ => throw const FormatException(
          'revision-3 localization-edit response grants unsupported publication authority',
        ),
      },
    );
  }
}

Map<String, String> _dialogLocalizationEditTexts(Map<String, String> input) {
  if (input.isEmpty ||
      input.length > _maxAuthoringRevision3DialogLocalizationEditTexts) {
    throw const FormatException(
      'revision-3 localization-edit texts are empty or exceed their count limit',
    );
  }
  final keys = input.keys.toList(growable: false)..sort();
  final result = <String, String>{};
  var total = 0;
  var hasNonblank = false;
  for (final rawLocale in keys) {
    final locale = _authoringRevision3VoiceLocale(rawLocale);
    if (locale != rawLocale || result.containsKey(locale)) {
      throw const FormatException(
        'revision-3 localization-edit locales are not canonical and unique',
      );
    }
    final text = input[rawLocale]!;
    final bytes = utf8.encode(text).length;
    total += bytes;
    hasNonblank |= text.trim().isNotEmpty;
    if (text.contains('\u0000') ||
        bytes > _maxAuthoringRevision3DialogLocalizationEditTextBytes ||
        total > _maxAuthoringRevision3DialogLocalizationEditTextsBytes) {
      throw const FormatException(
        'revision-3 localization-edit text exceeds its safe budget',
      );
    }
    result[locale] = text;
  }
  if (!hasNonblank) {
    throw const FormatException(
      'revision-3 localization-edit texts contain no nonblank value',
    );
  }
  return Map.unmodifiable(result);
}

Map<String, String> _dialogLocalizationEditCurrentTexts(
  Map<String, Object?> project, {
  required String localizationId,
  required int expectedRevision,
  required String expectedLocId,
}) {
  final entities = _authoringRequiredObject(
    project['entities'],
    'revision-3 localization-edit project entities',
  );
  final entity = _dialogExistingEntity(
    entities,
    localizationId,
    expectedKind: 'localization_entry',
    context: 'localization-edit target',
  );
  if (_authoringRequiredInt(
        entity,
        'revision',
        max: _maxAuthoringSignedJsonInteger,
      ) !=
      expectedRevision) {
    throw const FormatException(
      'revision-3 localization-edit target revision is stale',
    );
  }
  final origin = _authoringRequiredObject(
    entity['origin'],
    'revision-3 localization-edit target origin',
  );
  if (origin['type'] != 'new') {
    throw const FormatException(
      'revision-3 localization-edit target is not an authored localization',
    );
  }
  final payload = _authoringRequiredObject(
    entity['payload'],
    'revision-3 localization-edit target payload',
  );
  final data = _authoringRequiredObject(
    payload['data'],
    'revision-3 localization-edit target data',
  );
  if (_authoringRequiredString(
        data,
        'loc_id',
        maxBytes: _maxAuthoringRevision3DialogLocalizationReadLocIdBytes,
      ) !=
      expectedLocId) {
    throw const FormatException(
      'revision-3 localization-edit target identity is stale',
    );
  }
  final rawTexts = _authoringRequiredObject(
    data['texts'],
    'revision-3 localization-edit target texts',
  );
  return _dialogLocalizationEditTexts(
    rawTexts.map(
      (key, value) => MapEntry(
        key,
        value is String
            ? value
            : throw const FormatException(
                'revision-3 localization-edit target text is not a string',
              ),
      ),
    ),
  );
}

void _dialogLocalizationEditRequireExactCandidate(
  Map<String, Object?> basis,
  Map<String, Object?> candidate, {
  required AuthoringRevision3DialogLocalizationEditRequestV1 request,
}) {
  final expected = _authoringRevision3VoiceCloneObject(
    basis,
    'revision-3 localization-edit expected candidate',
  );
  expected['revision'] = request.expectedRevision + 1;
  final authoringLocales =
      (expected['authoring_locales'] as List<Object?>)
          .map(
            (value) => value is String
                ? _authoringRevision3VoiceLocale(value)
                : throw const FormatException(
                    'revision-3 localization-edit basis locale is invalid',
                  ),
          )
          .toSet()
        ..addAll(request.texts.keys);
  final sortedAuthoringLocales = authoringLocales.toList(growable: false)
    ..sort();
  expected['authoring_locales'] = sortedAuthoringLocales;

  final entities = _authoringRequiredObject(
    expected['entities'],
    'revision-3 localization-edit expected entities',
  );
  final entity = _dialogExistingEntity(
    entities,
    request.localizationId,
    expectedKind: 'localization_entry',
    context: 'localization-edit expected target',
  );
  entity['revision'] = request.expectedLocalizationRevision + 1;
  final payload = _authoringRequiredObject(
    entity['payload'],
    'revision-3 localization-edit expected target payload',
  );
  final data = _authoringRequiredObject(
    payload['data'],
    'revision-3 localization-edit expected target data',
  );
  data['texts'] = request.texts;
  payload['data'] = data;
  entity['payload'] = payload;
  entities[request.localizationId] = entity;
  expected['entities'] = entities;
  if (jsonEncode(expected) != jsonEncode(candidate)) {
    throw const FormatException(
      'revision-3 localization-edit candidate contains an unverified semantic delta',
    );
  }
}

List<String> _dialogLocalizationEditLocaleList(Object? value, String context) {
  if (value is! List<Object?> ||
      value.length > _maxAuthoringRevision3DialogLocalizationEditTexts) {
    throw FormatException('$context is invalid');
  }
  final result = <String>[];
  String? previous;
  for (final item in value) {
    if (item is! String) {
      throw FormatException('$context contains a non-string');
    }
    final locale = _authoringRevision3VoiceLocale(item);
    if (previous != null && previous.compareTo(locale) >= 0) {
      throw FormatException('$context is not unique canonical order');
    }
    previous = locale;
    result.add(locale);
  }
  return result;
}

bool _dialogLocalizationEditSameMap(
  Map<String, String> left,
  Map<String, String> right,
) {
  if (left.length != right.length) return false;
  for (final entry in left.entries) {
    if (right[entry.key] != entry.value) return false;
  }
  return true;
}

bool _dialogLocalizationEditSameSet(Set<String> left, Set<String> right) =>
    left.length == right.length && left.containsAll(right);

bool _dialogLocalizationEditSameList(List<String> left, List<String> right) {
  if (left.length != right.length) return false;
  for (var index = 0; index < left.length; index++) {
    if (left[index] != right[index]) return false;
  }
  return true;
}
