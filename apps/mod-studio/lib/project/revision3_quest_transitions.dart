part of '../core/mod_ffi.dart';

const _maxRevision3QuestTransitionGroups = 8;
const _maxRevision3QuestTransitionAtoms = 8;
const _maxRevision3QuestTransitionEffects = 8;
const _maxRevision3QuestTransitionObjectiveSlot = 0xffff;
const _maxRevision3QuestTransitionsRequestJsonBytes = 512 * 1024;
const _revision3QuestTransitionPlanSealDomain =
    'gore-authoring.revision3-quest-transition-plan-v1\u0000';

enum AuthoringRevision3QuestTransitionNodeKind { root, objective }

final class AuthoringRevision3QuestTransitionObjectiveV1 {
  const AuthoringRevision3QuestTransitionObjectiveV1({
    required this.slot,
    required this.title,
  });

  final int slot;
  final String title;
}

/// Exact-current private seed for the managed transition editor.
final class AuthoringRevision3QuestTransitionsSeed {
  const AuthoringRevision3QuestTransitionsSeed._({
    required this.projectId,
    required this.projectRevision,
    required this.targetCanonicalJson,
    required this.questId,
    required this.questRevision,
    required this.moduleId,
    required this.moduleRevision,
    required this.generatorVersion,
    required this.objectives,
    required this.transitionPlan,
    required this.transitionPlanSeal,
    required this.legacySynthetic,
  });

  factory AuthoringRevision3QuestTransitionsSeed.forProject({
    required String currentProjectJson,
    required String questId,
    required int expectedQuestRevision,
    required String expectedModuleId,
    required int expectedModuleRevision,
  }) {
    final current = _authoringRequireCanonicalRevision3ProjectJson(
      currentProjectJson,
    );
    final basis = _questTransitionsRequireBasis(
      current.project,
      projectId: current.projectId,
      questId: questId,
    );
    if (basis.questRevision != expectedQuestRevision ||
        basis.moduleId != expectedModuleId ||
        basis.moduleRevision != expectedModuleRevision) {
      throw const FormatException(
        'revision-3 Quest transition seed does not match the selected exact-current Quest',
      );
    }
    return AuthoringRevision3QuestTransitionsSeed._(
      projectId: current.projectId,
      projectRevision: current.revision,
      targetCanonicalJson: jsonEncode(current.project['target']),
      questId: questId,
      questRevision: basis.questRevision,
      moduleId: basis.moduleId,
      moduleRevision: basis.moduleRevision,
      generatorVersion: basis.generatorVersion,
      objectives: basis.objectives,
      transitionPlan: basis.transitionPlan,
      transitionPlanSeal: basis.transitionPlan.contentSeal,
      legacySynthetic: basis.legacySynthetic,
    );
  }

  final String projectId;
  final int projectRevision;
  final String targetCanonicalJson;
  final String questId;
  final int questRevision;
  final String moduleId;
  final int moduleRevision;
  final int generatorVersion;
  final List<AuthoringRevision3QuestTransitionObjectiveV1> objectives;
  final AuthoringRevision3QuestTransitionPlanV1 transitionPlan;
  final AuthoringDraftContentSeal transitionPlanSeal;
  final bool legacySynthetic;
}

/// Exact-head, exact-entity-CAS-bound intent for one semantic transition edit.
final class AuthoringRevision3QuestTransitionsEditRequestV1 {
  const AuthoringRevision3QuestTransitionsEditRequestV1._({
    required this.canonicalJson,
    required this.expectedHead,
    required this.expectedProjectId,
    required this.expectedRevision,
    required this.expectedTargetCanonicalJson,
    required this.questId,
    required this.expectedQuestRevision,
    required this.expectedTransitionPlanSeal,
    required this.transitionPlan,
    required this.moduleId,
    required this.expectedModuleRevision,
    required this.previousGeneratorVersion,
    required this.upgradesLegacy,
  });

  factory AuthoringRevision3QuestTransitionsEditRequestV1.forProject({
    required AuthoringWorkingHead expectedHead,
    required String currentProjectJson,
    required String questId,
    required int expectedQuestRevision,
    required AuthoringRevision3QuestTransitionPlanV1 transitionPlan,
  }) {
    final current = _authoringRequireCanonicalRevision3ProjectJson(
      currentProjectJson,
    );
    final basis = _questTransitionsRequireBasis(
      current.project,
      projectId: current.projectId,
      questId: questId,
    );
    return AuthoringRevision3QuestTransitionsEditRequestV1.fromCanonicalJson(
      jsonEncode(<String, Object?>{
        'expected_head': jsonDecode(expectedHead.canonicalJson),
        'expected_project_id': current.projectId,
        'expected_revision': current.revision,
        'expected_target': current.project['target'],
        'quest_id': questId,
        'expected_quest_revision': expectedQuestRevision,
        'expected_transition_plan_seal': _questTransitionsSealJson(
          basis.transitionPlan.contentSeal,
        ),
        'transition_plan': transitionPlan.toJson(),
      }),
      currentProjectJson: currentProjectJson,
    );
  }

  factory AuthoringRevision3QuestTransitionsEditRequestV1.fromCanonicalJson(
    String value, {
    required String currentProjectJson,
  }) {
    try {
      _authoringRevision3RequestString(
        value,
        'questTransitionsRequestJson',
        _maxRevision3QuestTransitionsRequestJsonBytes,
      );
    } on ArgumentError {
      throw const FormatException(
        'authoring revision-3 Quest transitions request is not bounded UTF-8',
      );
    }
    final request = _authoringDecodeDuplicateSafeObject(
      value,
      'revision-3 Quest transitions request',
    );
    const fields = <String>[
      'expected_head',
      'expected_project_id',
      'expected_revision',
      'expected_target',
      'quest_id',
      'expected_quest_revision',
      'expected_transition_plan_seal',
      'transition_plan',
    ];
    _authoringExactFields(
      request,
      fields.toSet(),
      'revision-3 Quest transitions request',
    );
    if (!_questTransitionsSameStrings(
          request.keys.toList(growable: false),
          fields,
        ) ||
        jsonEncode(request) != value) {
      throw const FormatException(
        'authoring revision-3 Quest transitions request is not canonical',
      );
    }
    final parsed = AuthoringRevision3QuestTransitionsEditRequestV1._(
      canonicalJson: value,
      expectedHead: AuthoringWorkingHead.fromCanonicalJson(
        jsonEncode(
          _authoringRequiredObject(
            request['expected_head'],
            'revision-3 Quest transitions expected head',
          ),
        ),
      ),
      expectedProjectId: _questTransitionsEntityId(
        request,
        'expected_project_id',
      ),
      expectedRevision: _authoringRequiredInt(
        request,
        'expected_revision',
        max: _maxAuthoringStoryBaseRevision,
      ),
      expectedTargetCanonicalJson: jsonEncode(
        _questOutlineGeneration(
          request['expected_target'],
          'transition request target',
        ).json,
      ),
      questId: _questTransitionsEntityId(request, 'quest_id'),
      expectedQuestRevision: _authoringRequiredInt(
        request,
        'expected_quest_revision',
        max: _maxAuthoringStoryBaseRevision,
      ),
      expectedTransitionPlanSeal: AuthoringDraftContentSeal.fromJson(
        _authoringRequiredObject(
          request['expected_transition_plan_seal'],
          'revision-3 Quest transitions expected plan seal',
        ),
      ),
      transitionPlan: AuthoringRevision3QuestTransitionPlanV1.fromJson(
        request['transition_plan'],
      ),
      moduleId: '',
      expectedModuleRevision: 0,
      previousGeneratorVersion: 0,
      upgradesLegacy: false,
    );
    final current = _authoringRequireCanonicalRevision3ProjectJson(
      currentProjectJson,
    );
    final basis = _questTransitionsRequireBasis(
      current.project,
      projectId: current.projectId,
      questId: parsed.questId,
    );
    final bound = AuthoringRevision3QuestTransitionsEditRequestV1._(
      canonicalJson: parsed.canonicalJson,
      expectedHead: parsed.expectedHead,
      expectedProjectId: parsed.expectedProjectId,
      expectedRevision: parsed.expectedRevision,
      expectedTargetCanonicalJson: parsed.expectedTargetCanonicalJson,
      questId: parsed.questId,
      expectedQuestRevision: parsed.expectedQuestRevision,
      expectedTransitionPlanSeal: parsed.expectedTransitionPlanSeal,
      transitionPlan: parsed.transitionPlan,
      moduleId: basis.moduleId,
      expectedModuleRevision: basis.moduleRevision,
      previousGeneratorVersion: basis.generatorVersion,
      upgradesLegacy: basis.legacySynthetic,
    );
    bound._requireExactProjectBinding(current, basis);
    return bound;
  }

  final String canonicalJson;
  final AuthoringWorkingHead expectedHead;
  final String expectedProjectId;
  final int expectedRevision;
  final String expectedTargetCanonicalJson;
  final String questId;
  final int expectedQuestRevision;
  final AuthoringDraftContentSeal expectedTransitionPlanSeal;
  final AuthoringRevision3QuestTransitionPlanV1 transitionPlan;
  final String moduleId;
  final int expectedModuleRevision;
  final int previousGeneratorVersion;
  final bool upgradesLegacy;

  void _requireExactCurrent(
    ({Map<String, Object?> project, String projectId, int revision}) current,
  ) {
    final basis = _questTransitionsRequireBasis(
      current.project,
      projectId: current.projectId,
      questId: questId,
    );
    _requireExactProjectBinding(current, basis);
  }

  void _requireExactProjectBinding(
    ({Map<String, Object?> project, String projectId, int revision}) current,
    ({
      int questRevision,
      String moduleId,
      int moduleRevision,
      int generatorVersion,
      List<AuthoringRevision3QuestTransitionObjectiveV1> objectives,
      AuthoringRevision3QuestTransitionPlanV1 transitionPlan,
      bool legacySynthetic,
    })
    basis,
  ) {
    if (expectedProjectId != current.projectId ||
        expectedRevision != current.revision ||
        expectedTargetCanonicalJson != jsonEncode(current.project['target']) ||
        expectedQuestRevision != basis.questRevision ||
        moduleId != basis.moduleId ||
        expectedModuleRevision != basis.moduleRevision ||
        previousGeneratorVersion != basis.generatorVersion ||
        upgradesLegacy != basis.legacySynthetic ||
        !_questTransitionsSameInts(
          transitionPlan.objectiveSlots,
          basis.transitionPlan.objectiveSlots,
        ) ||
        transitionPlan.nextSlotOrdinal < basis.transitionPlan.nextSlotOrdinal ||
        !_questTransitionsSameSeal(
          expectedTransitionPlanSeal,
          basis.transitionPlan.contentSeal,
        )) {
      throw const FormatException(
        'authoring revision-3 Quest transitions request does not bind the exact current Quest',
      );
    }
    if (!basis.legacySynthetic &&
        transitionPlan.canonicalJson == basis.transitionPlan.canonicalJson) {
      throw const FormatException(
        'authoring revision-3 Quest transitions request does not change the plan',
      );
    }
  }
}

enum AuthoringRevision3QuestTransitionsBuildStatus { blocked }

enum AuthoringRevision3QuestTransitionsRuntimeStatus { runtimeUnqualified }

enum AuthoringRevision3QuestTransitionsPublicationStatus { notSupported }

/// Strict unpublished transition-plan candidate prepared by native code.
final class AuthoringRevision3QuestTransitionsEditPreparation {
  const AuthoringRevision3QuestTransitionsEditPreparation._({
    required this.basisHead,
    required this.head,
    required this.projectJson,
    required this.projectId,
    required this.revision,
    required this.questId,
    required this.moduleId,
    required this.questRevision,
    required this.moduleRevision,
    required this.previousGeneratorVersion,
    required this.upgradedFromLegacy,
    required this.transitionPlanSeal,
    required this.buildStatus,
    required this.runtimeStatus,
    required this.publicationStatus,
  });

  final AuthoringWorkingHead basisHead;
  final AuthoringWorkingHead head;
  final String projectJson;
  final String projectId;
  final int revision;
  final String questId;
  final String moduleId;
  final int questRevision;
  final int moduleRevision;
  final int previousGeneratorVersion;
  final bool upgradedFromLegacy;
  final AuthoringDraftContentSeal transitionPlanSeal;
  final AuthoringRevision3QuestTransitionsBuildStatus buildStatus;
  final AuthoringRevision3QuestTransitionsRuntimeStatus runtimeStatus;
  final AuthoringRevision3QuestTransitionsPublicationStatus publicationStatus;

  factory AuthoringRevision3QuestTransitionsEditPreparation.fromJson(
    Map<String, Object?> json, {
    required String currentProjectJson,
    required AuthoringRevision3QuestTransitionsEditRequestV1 request,
  }) {
    final base = _authoringRequireCanonicalRevision3ProjectJson(
      currentProjectJson,
    );
    request._requireExactCurrent(base);
    _authoringExactFields(json, const <String>{
      'ok',
      'outcome',
      'basis_head_json',
      'head_json',
      'project_json',
      'project_id',
      'revision',
      'quest_id',
      'module_id',
      'quest_revision',
      'module_revision',
      'previous_generator_version',
      'upgraded_from_legacy',
      'transition_plan_seal',
      'build_status',
      'runtime_status',
      'publication_status',
    }, 'revision-3 Quest transitions preparation response');
    if (json['ok'] != true || json['outcome'] != 'prepared_unpublished') {
      throw const FormatException(
        'authoring revision-3 Quest transitions response is not an unpublished preparation',
      );
    }
    final basisHead = AuthoringWorkingHead.fromCanonicalJson(
      _authoringRevision3ResponseString(
        json,
        'basis_head_json',
        maxBytes: _maxAuthoringHeadJsonBytes,
      ),
    );
    if (basisHead.canonicalJson != request.expectedHead.canonicalJson) {
      throw const FormatException(
        'authoring revision-3 Quest transitions response changed its basis head',
      );
    }
    final head = AuthoringWorkingHead.fromCanonicalJson(
      _authoringRevision3ResponseString(
        json,
        'head_json',
        maxBytes: _maxAuthoringHeadJsonBytes,
      ),
    );
    if (head.canonicalJson == basisHead.canonicalJson) {
      throw const FormatException(
        'authoring revision-3 Quest transitions candidate did not advance its head',
      );
    }
    final projectJson = _authoringRevision3ResponseString(
      json,
      'project_json',
      maxBytes: _maxAuthoringProjectJsonBytes,
    );
    final candidate = _authoringRequireCanonicalRevision3ProjectJson(
      projectJson,
    );
    final projectId = _questTransitionsEntityId(json, 'project_id');
    final revision = _authoringRequiredInt(
      json,
      'revision',
      min: 1,
      max: _maxAuthoringStoryAppliedRevision,
    );
    final questId = _questTransitionsEntityId(json, 'quest_id');
    final moduleId = _questTransitionsEntityId(json, 'module_id');
    final questRevision = _authoringRequiredInt(
      json,
      'quest_revision',
      min: 1,
      max: _maxAuthoringStoryAppliedRevision,
    );
    final moduleRevision = _authoringRequiredInt(
      json,
      'module_revision',
      min: 1,
      max: _maxAuthoringStoryAppliedRevision,
    );
    final previousGeneratorVersion = _authoringRequiredInt(
      json,
      'previous_generator_version',
      min: _authoringRevision3QuestGeneratorVersion,
      max: _authoringRevision3SemanticQuestGeneratorVersion,
    );
    final upgradedFromLegacy = json['upgraded_from_legacy'];
    if (upgradedFromLegacy is! bool) {
      throw const FormatException(
        'authoring revision-3 Quest transitions response has an invalid upgrade flag',
      );
    }
    final transitionPlanSeal = AuthoringDraftContentSeal.fromJson(
      _authoringRequiredObject(
        json['transition_plan_seal'],
        'revision-3 Quest transitions response plan seal',
      ),
    );
    if (projectId != base.projectId ||
        projectId != candidate.projectId ||
        revision != base.revision + 1 ||
        revision != candidate.revision ||
        questId != request.questId ||
        moduleId != request.moduleId ||
        questRevision != request.expectedQuestRevision + 1 ||
        moduleRevision != request.expectedModuleRevision + 1 ||
        previousGeneratorVersion != request.previousGeneratorVersion ||
        upgradedFromLegacy != request.upgradesLegacy ||
        !_questTransitionsSameSeal(
          transitionPlanSeal,
          request.transitionPlan.contentSeal,
        )) {
      throw const FormatException(
        'authoring revision-3 Quest transitions response identity, revisions, or plan seal disagree',
      );
    }
    _questTransitionsRequireExactDelta(
      base.project,
      candidate.project,
      request: request,
      questRevision: questRevision,
      moduleRevision: moduleRevision,
    );
    return AuthoringRevision3QuestTransitionsEditPreparation._(
      basisHead: basisHead,
      head: head,
      projectJson: projectJson,
      projectId: projectId,
      revision: revision,
      questId: questId,
      moduleId: moduleId,
      questRevision: questRevision,
      moduleRevision: moduleRevision,
      previousGeneratorVersion: previousGeneratorVersion,
      upgradedFromLegacy: upgradedFromLegacy,
      transitionPlanSeal: transitionPlanSeal,
      buildStatus: switch (json['build_status']) {
        'blocked' => AuthoringRevision3QuestTransitionsBuildStatus.blocked,
        _ => throw const FormatException(
          'authoring revision-3 Quest transitions response grants unsupported build authority',
        ),
      },
      runtimeStatus: switch (json['runtime_status']) {
        'runtime_unqualified' =>
          AuthoringRevision3QuestTransitionsRuntimeStatus.runtimeUnqualified,
        _ => throw const FormatException(
          'authoring revision-3 Quest transitions response grants unsupported runtime authority',
        ),
      },
      publicationStatus: switch (json['publication_status']) {
        'not_supported' =>
          AuthoringRevision3QuestTransitionsPublicationStatus.notSupported,
        _ => throw const FormatException(
          'authoring revision-3 Quest transitions response grants unsupported publication authority',
        ),
      },
    );
  }
}

/// Stable technical identity of the Quest root or one objective.
///
/// Objective slots survive presentation reordering and are never inferred from
/// a visible row index.
final class AuthoringRevision3QuestTransitionNodeV1 {
  const AuthoringRevision3QuestTransitionNodeV1._({
    required this.kind,
    this.slot,
  });

  const AuthoringRevision3QuestTransitionNodeV1.root()
    : this._(kind: AuthoringRevision3QuestTransitionNodeKind.root);

  factory AuthoringRevision3QuestTransitionNodeV1.objective(int slot) {
    _questTransitionsRequireObjectiveSlot(slot, 'objective node');
    return AuthoringRevision3QuestTransitionNodeV1._(
      kind: AuthoringRevision3QuestTransitionNodeKind.objective,
      slot: slot,
    );
  }

  factory AuthoringRevision3QuestTransitionNodeV1.fromJson(
    Object? value, {
    required String context,
  }) {
    final json = _authoringRequiredObject(value, context);
    final kind = json['kind'];
    switch (kind) {
      case 'root':
        _authoringExactFields(json, const {'kind'}, context);
        return const AuthoringRevision3QuestTransitionNodeV1.root();
      case 'objective':
        _authoringExactFields(json, const {'kind', 'slot'}, context);
        final slot = _authoringRequiredInt(
          json,
          'slot',
          min: 1,
          max: _maxRevision3QuestTransitionObjectiveSlot,
        );
        return AuthoringRevision3QuestTransitionNodeV1.objective(slot);
      default:
        throw FormatException('$context has an unsupported node kind');
    }
  }

  final AuthoringRevision3QuestTransitionNodeKind kind;
  final int? slot;

  Map<String, Object?> toJson() => switch (kind) {
    AuthoringRevision3QuestTransitionNodeKind.root => <String, Object?>{
      'kind': 'root',
    },
    AuthoringRevision3QuestTransitionNodeKind.objective => <String, Object?>{
      'kind': 'objective',
      'slot': slot!,
    },
  };

  String get stableKey => switch (kind) {
    AuthoringRevision3QuestTransitionNodeKind.root => 'root',
    AuthoringRevision3QuestTransitionNodeKind.objective => 'objective:$slot',
  };

  @override
  bool operator ==(Object other) =>
      other is AuthoringRevision3QuestTransitionNodeV1 &&
      other.kind == kind &&
      other.slot == slot;

  @override
  int get hashCode => Object.hash(kind, slot);
}

enum AuthoringRevision3QuestTransitionEdgeV1 {
  availability('availability'),
  start('start'),
  success('success'),
  failure('failure');

  const AuthoringRevision3QuestTransitionEdgeV1(this.wireName);
  final String wireName;

  static AuthoringRevision3QuestTransitionEdgeV1 fromWire(
    Object? value,
    String context,
  ) => switch (value) {
    'availability' => availability,
    'start' => start,
    'success' => success,
    'failure' => failure,
    _ => throw FormatException('$context has an unsupported lifecycle edge'),
  };
}

enum AuthoringRevision3QuestTransitionStateTestV1 {
  available('available'),
  running('running'),
  started('started'),
  succeeded('succeeded'),
  failed('failed'),
  completed('completed');

  const AuthoringRevision3QuestTransitionStateTestV1(this.wireName);
  final String wireName;

  static AuthoringRevision3QuestTransitionStateTestV1 fromWire(
    Object? value,
    String context,
  ) => switch (value) {
    'available' => available,
    'running' => running,
    'started' => started,
    'succeeded' => succeeded,
    'failed' => failed,
    'completed' => completed,
    _ => throw FormatException('$context has an unsupported state test'),
  };
}

final class AuthoringRevision3QuestTransitionConditionAtomV1 {
  const AuthoringRevision3QuestTransitionConditionAtomV1({
    required this.node,
    required this.test,
    required this.negated,
  });

  factory AuthoringRevision3QuestTransitionConditionAtomV1.fromJson(
    Object? value, {
    required String context,
  }) {
    final json = _authoringRequiredObject(value, context);
    _authoringExactFields(json, const {'node', 'test', 'negated'}, context);
    final negated = json['negated'];
    if (negated is! bool) {
      throw FormatException('$context negation must be a boolean');
    }
    return AuthoringRevision3QuestTransitionConditionAtomV1(
      node: AuthoringRevision3QuestTransitionNodeV1.fromJson(
        json['node'],
        context: '$context node',
      ),
      test: AuthoringRevision3QuestTransitionStateTestV1.fromWire(
        json['test'],
        '$context test',
      ),
      negated: negated,
    );
  }

  final AuthoringRevision3QuestTransitionNodeV1 node;
  final AuthoringRevision3QuestTransitionStateTestV1 test;
  final bool negated;

  Map<String, Object?> toJson() => <String, Object?>{
    'node': node.toJson(),
    'test': test.wireName,
    'negated': negated,
  };

  String get stableKey => '${node.stableKey}:${test.wireName}:$negated';
}

final class AuthoringRevision3QuestTransitionConditionGroupV1 {
  AuthoringRevision3QuestTransitionConditionGroupV1({
    required List<AuthoringRevision3QuestTransitionConditionAtomV1> allOf,
  }) : allOf = List.unmodifiable(allOf) {
    if (allOf.isEmpty || allOf.length > _maxRevision3QuestTransitionAtoms) {
      throw const FormatException(
        'a Quest transition condition group must contain 1 to 8 tests',
      );
    }
    _questTransitionsRequireUnique(
      allOf.map((atom) => atom.stableKey),
      'Quest transition condition group contains duplicate tests',
    );
    _questTransitionsRequireStrictlySorted(
      allOf.map(_questTransitionsAtomSortKey),
      'Quest transition condition tests are not in canonical order',
    );
    final polarities = <String, bool>{};
    for (final atom in allOf) {
      final key = '${atom.node.stableKey}:${atom.test.wireName}';
      final previous = polarities[key];
      if (previous != null && previous != atom.negated) {
        throw const FormatException(
          'Quest transition condition contains a direct contradiction',
        );
      }
      polarities[key] = atom.negated;
    }
    if (_questTransitionsHasLifecycleStateContradiction(
      polarities,
      allOf.map((atom) => atom.node),
    )) {
      throw const FormatException(
        'Quest transition condition contains incompatible lifecycle states',
      );
    }
  }

  factory AuthoringRevision3QuestTransitionConditionGroupV1.fromJson(
    Object? value, {
    required String context,
  }) {
    final json = _authoringRequiredObject(value, context);
    _authoringExactFields(json, const {'all_of'}, context);
    final values = json['all_of'];
    if (values is! List<Object?> ||
        values.isEmpty ||
        values.length > _maxRevision3QuestTransitionAtoms) {
      throw FormatException('$context must contain 1 to 8 tests');
    }
    return AuthoringRevision3QuestTransitionConditionGroupV1(
      allOf: <AuthoringRevision3QuestTransitionConditionAtomV1>[
        for (var index = 0; index < values.length; index++)
          AuthoringRevision3QuestTransitionConditionAtomV1.fromJson(
            values[index],
            context: '$context test ${index + 1}',
          ),
      ],
    );
  }

  final List<AuthoringRevision3QuestTransitionConditionAtomV1> allOf;

  Map<String, Object?> toJson() => <String, Object?>{
    'all_of': allOf.map((atom) => atom.toJson()).toList(growable: false),
  };
}

final class AuthoringRevision3QuestTransitionPredicateV1 {
  AuthoringRevision3QuestTransitionPredicateV1({
    required List<AuthoringRevision3QuestTransitionConditionGroupV1> anyOf,
  }) : anyOf = List.unmodifiable(anyOf) {
    if (anyOf.isEmpty || anyOf.length > _maxRevision3QuestTransitionGroups) {
      throw const FormatException(
        'a Quest transition predicate must contain 1 to 8 alternatives',
      );
    }
    _questTransitionsRequireUnique(
      anyOf.map(_questTransitionsGroupStableKey),
      'Quest transition predicate contains duplicate alternatives',
    );
    _questTransitionsRequireStrictlySorted(
      anyOf.map(_questTransitionsGroupStableKey),
      'Quest transition alternatives are not in canonical order',
    );
  }

  factory AuthoringRevision3QuestTransitionPredicateV1.fromJson(
    Object? value, {
    required String context,
  }) {
    final json = _authoringRequiredObject(value, context);
    _authoringExactFields(json, const {'any_of'}, context);
    final values = json['any_of'];
    if (values is! List<Object?> ||
        values.isEmpty ||
        values.length > _maxRevision3QuestTransitionGroups) {
      throw FormatException('$context must contain 1 to 8 alternatives');
    }
    return AuthoringRevision3QuestTransitionPredicateV1(
      anyOf: <AuthoringRevision3QuestTransitionConditionGroupV1>[
        for (var index = 0; index < values.length; index++)
          AuthoringRevision3QuestTransitionConditionGroupV1.fromJson(
            values[index],
            context: '$context alternative ${index + 1}',
          ),
      ],
    );
  }

  final List<AuthoringRevision3QuestTransitionConditionGroupV1> anyOf;

  Map<String, Object?> toJson() => <String, Object?>{
    'any_of': anyOf.map((group) => group.toJson()).toList(growable: false),
  };
}

enum AuthoringRevision3QuestTransitionEffectKindV1 {
  start('start'),
  succeed('succeed'),
  fail('fail');

  const AuthoringRevision3QuestTransitionEffectKindV1(this.wireName);
  final String wireName;

  static AuthoringRevision3QuestTransitionEffectKindV1 fromWire(
    Object? value,
    String context,
  ) => switch (value) {
    'start' => start,
    'succeed' => succeed,
    'fail' => fail,
    _ => throw FormatException('$context has an unsupported Quest effect'),
  };
}

final class AuthoringRevision3QuestTransitionEffectV1 {
  const AuthoringRevision3QuestTransitionEffectV1({
    required this.target,
    required this.effect,
  });

  factory AuthoringRevision3QuestTransitionEffectV1.fromJson(
    Object? value, {
    required String context,
  }) {
    final json = _authoringRequiredObject(value, context);
    _authoringExactFields(json, const {'target', 'effect'}, context);
    return AuthoringRevision3QuestTransitionEffectV1(
      target: AuthoringRevision3QuestTransitionNodeV1.fromJson(
        json['target'],
        context: '$context target',
      ),
      effect: AuthoringRevision3QuestTransitionEffectKindV1.fromWire(
        json['effect'],
        '$context effect',
      ),
    );
  }

  final AuthoringRevision3QuestTransitionNodeV1 target;
  final AuthoringRevision3QuestTransitionEffectKindV1 effect;

  Map<String, Object?> toJson() => <String, Object?>{
    'target': target.toJson(),
    'effect': effect.wireName,
  };

  String get stableKey => '${target.stableKey}:${effect.wireName}';
}

final class AuthoringRevision3QuestTransitionV1 {
  AuthoringRevision3QuestTransitionV1({
    required this.node,
    required this.edge,
    required this.externalAllowed,
    this.predicate,
    List<AuthoringRevision3QuestTransitionEffectV1> effects = const [],
    this.succeedsParent = false,
  }) : effects = List.unmodifiable(effects) {
    if (effects.length > _maxRevision3QuestTransitionEffects) {
      throw const FormatException(
        'a Quest transition must contain at most 8 effects',
      );
    }
    _questTransitionsRequireUnique(
      effects.map((effect) => effect.stableKey),
      'Quest transition contains duplicate effects',
    );
    _questTransitionsRequireStrictlySorted(
      effects.map(_questTransitionsEffectSortKey),
      'Quest transition effects are not in canonical order',
    );
    if (!externalAllowed && predicate == null) {
      throw const FormatException(
        'a Quest transition needs an external or condition driver',
      );
    }
    if (edge == AuthoringRevision3QuestTransitionEdgeV1.availability &&
        effects.isNotEmpty) {
      throw const FormatException(
        'Quest availability transitions cannot carry effects',
      );
    }
    if (succeedsParent &&
        (node.kind != AuthoringRevision3QuestTransitionNodeKind.objective ||
            edge != AuthoringRevision3QuestTransitionEdgeV1.success)) {
      throw const FormatException(
        'succeeds_parent is valid only for objective success',
      );
    }
    final terminalEffects =
        <String, AuthoringRevision3QuestTransitionEffectKindV1>{
          if (succeedsParent)
            const AuthoringRevision3QuestTransitionNodeV1.root().stableKey:
                AuthoringRevision3QuestTransitionEffectKindV1.succeed,
        };
    for (final effect in effects) {
      if (effect.target == node) {
        throw const FormatException(
          'a Quest transition cannot apply an effect to its own node',
        );
      }
      if (effect.effect
          case AuthoringRevision3QuestTransitionEffectKindV1.succeed ||
              AuthoringRevision3QuestTransitionEffectKindV1.fail) {
        final previous = terminalEffects[effect.target.stableKey];
        if (previous != null) {
          if (previous == effect.effect) {
            throw const FormatException(
              'an explicit Quest action duplicates implicit parent success',
            );
          }
          throw const FormatException(
            'one Quest transition cannot both succeed and fail one target',
          );
        }
        terminalEffects[effect.target.stableKey] = effect.effect;
      }
    }
  }

  factory AuthoringRevision3QuestTransitionV1.fromJson(
    Object? value, {
    required String context,
  }) {
    final json = _authoringRequiredObject(value, context);
    _authoringExactFields(json, <String>{
      'node',
      'edge',
      'external_allowed',
      if (json.containsKey('predicate')) 'predicate',
      if (json.containsKey('effects')) 'effects',
      if (json.containsKey('succeeds_parent')) 'succeeds_parent',
    }, context);
    final externalAllowed = json['external_allowed'];
    if (externalAllowed is! bool) {
      throw FormatException('$context external trigger flag must be boolean');
    }
    final succeedsParent = json['succeeds_parent'] ?? false;
    if (succeedsParent is! bool ||
        succeedsParent == false && json.containsKey('succeeds_parent')) {
      throw FormatException(
        '$context succeeds_parent must be true or omitted canonically',
      );
    }
    final rawEffects = json['effects'];
    if (rawEffects != null &&
        (rawEffects is! List<Object?> ||
            rawEffects.isEmpty ||
            rawEffects.length > _maxRevision3QuestTransitionEffects)) {
      throw FormatException('$context effects must contain 1 to 8 items');
    }
    return AuthoringRevision3QuestTransitionV1(
      node: AuthoringRevision3QuestTransitionNodeV1.fromJson(
        json['node'],
        context: '$context node',
      ),
      edge: AuthoringRevision3QuestTransitionEdgeV1.fromWire(
        json['edge'],
        '$context edge',
      ),
      externalAllowed: externalAllowed,
      predicate: json.containsKey('predicate')
          ? AuthoringRevision3QuestTransitionPredicateV1.fromJson(
              json['predicate'],
              context: '$context predicate',
            )
          : null,
      effects: rawEffects is List<Object?>
          ? <AuthoringRevision3QuestTransitionEffectV1>[
              for (var index = 0; index < rawEffects.length; index++)
                AuthoringRevision3QuestTransitionEffectV1.fromJson(
                  rawEffects[index],
                  context: '$context effect ${index + 1}',
                ),
            ]
          : const [],
      succeedsParent: succeedsParent,
    );
  }

  final AuthoringRevision3QuestTransitionNodeV1 node;
  final AuthoringRevision3QuestTransitionEdgeV1 edge;
  final bool externalAllowed;
  final AuthoringRevision3QuestTransitionPredicateV1? predicate;
  final List<AuthoringRevision3QuestTransitionEffectV1> effects;
  final bool succeedsParent;

  Map<String, Object?> toJson() => <String, Object?>{
    'node': node.toJson(),
    'edge': edge.wireName,
    'external_allowed': externalAllowed,
    if (predicate case final predicate?) 'predicate': predicate.toJson(),
    if (effects.isNotEmpty)
      'effects': effects
          .map((effect) => effect.toJson())
          .toList(growable: false),
    if (succeedsParent) 'succeeds_parent': true,
  };

  String get stableKey => '${node.stableKey}:${edge.wireName}';
}

/// Closed, bounded semantic lifecycle plan used by Quest generator version 4.
final class AuthoringRevision3QuestTransitionPlanV1 {
  AuthoringRevision3QuestTransitionPlanV1({
    required List<int> objectiveSlots,
    required List<int> objectiveOrder,
    required this.nextSlotOrdinal,
    required List<AuthoringRevision3QuestTransitionV1> transitions,
  }) : objectiveSlots = List.unmodifiable(objectiveSlots),
       objectiveOrder = List.unmodifiable(objectiveOrder),
       transitions = List.unmodifiable(transitions) {
    _validate();
  }

  factory AuthoringRevision3QuestTransitionPlanV1.fromJson(
    Object? value, {
    String context = 'revision-3 Quest transition plan',
  }) {
    final json = _authoringRequiredObject(value, context);
    _authoringExactFields(json, const {
      'objective_slots',
      'objective_order',
      'next_slot_ordinal',
      'transitions',
    }, context);
    final slots = _questTransitionsSlotList(
      json['objective_slots'],
      '$context objective slots',
    );
    final order = _questTransitionsSlotList(
      json['objective_order'],
      '$context objective order',
    );
    final rawTransitions = json['transitions'];
    if (rawTransitions is! List<Object?> || rawTransitions.isEmpty) {
      throw FormatException('$context must contain lifecycle transitions');
    }
    return AuthoringRevision3QuestTransitionPlanV1(
      objectiveSlots: slots,
      objectiveOrder: order,
      nextSlotOrdinal: _authoringRequiredInt(
        json,
        'next_slot_ordinal',
        min: 2,
        max: _maxRevision3QuestTransitionObjectiveSlot,
      ),
      transitions: <AuthoringRevision3QuestTransitionV1>[
        for (var index = 0; index < rawTransitions.length; index++)
          AuthoringRevision3QuestTransitionV1.fromJson(
            rawTransitions[index],
            context: '$context transition ${index + 1}',
          ),
      ],
    );
  }

  /// Effective transition plan of one frozen generator-v2/v3 Quest.
  factory AuthoringRevision3QuestTransitionPlanV1.legacySeed(
    int objectiveCount,
  ) {
    if (objectiveCount < 1 ||
        objectiveCount > _maxAuthoringRevision3QuestObjectives) {
      throw const FormatException(
        'a legacy Quest transition seed requires 1 to 8 objectives',
      );
    }
    final slots = List<int>.generate(objectiveCount, (index) => index + 1);
    return AuthoringRevision3QuestTransitionPlanV1(
      objectiveSlots: slots,
      objectiveOrder: slots,
      nextSlotOrdinal: objectiveCount + 1,
      transitions: <AuthoringRevision3QuestTransitionV1>[
        AuthoringRevision3QuestTransitionV1(
          node: const AuthoringRevision3QuestTransitionNodeV1.root(),
          edge: AuthoringRevision3QuestTransitionEdgeV1.availability,
          externalAllowed: true,
        ),
        AuthoringRevision3QuestTransitionV1(
          node: const AuthoringRevision3QuestTransitionNodeV1.root(),
          edge: AuthoringRevision3QuestTransitionEdgeV1.start,
          externalAllowed: true,
        ),
        for (var index = 0; index < slots.length; index++) ...[
          AuthoringRevision3QuestTransitionV1(
            node: AuthoringRevision3QuestTransitionNodeV1.objective(
              slots[index],
            ),
            edge: AuthoringRevision3QuestTransitionEdgeV1.availability,
            externalAllowed: true,
          ),
          AuthoringRevision3QuestTransitionV1(
            node: AuthoringRevision3QuestTransitionNodeV1.objective(
              slots[index],
            ),
            edge: AuthoringRevision3QuestTransitionEdgeV1.start,
            externalAllowed: true,
          ),
          AuthoringRevision3QuestTransitionV1(
            node: AuthoringRevision3QuestTransitionNodeV1.objective(
              slots[index],
            ),
            edge: AuthoringRevision3QuestTransitionEdgeV1.success,
            externalAllowed: true,
            succeedsParent: index + 1 == slots.length,
          ),
        ],
      ],
    );
  }

  final List<int> objectiveSlots;
  final List<int> objectiveOrder;
  final int nextSlotOrdinal;
  final List<AuthoringRevision3QuestTransitionV1> transitions;

  Map<String, Object?> toJson() => <String, Object?>{
    'objective_slots': objectiveSlots,
    'objective_order': objectiveOrder,
    'next_slot_ordinal': nextSlotOrdinal,
    'transitions': transitions
        .map((transition) => transition.toJson())
        .toList(growable: false),
  };

  String get canonicalJson => jsonEncode(toJson());

  /// Domain-separated exact CAS seal shared with the native transaction.
  AuthoringDraftContentSeal get contentSeal {
    final planBytes = utf8.encode(canonicalJson);
    final length = ByteData(8)..setUint64(0, planBytes.length, Endian.big);
    final bytes = BytesBuilder(copy: false)
      ..add(utf8.encode(_revision3QuestTransitionPlanSealDomain))
      ..add(length.buffer.asUint8List())
      ..add(planBytes);
    return AuthoringDraftContentSeal.fromJson(<String, Object?>{
      'byte_len': planBytes.length,
      'sha256': crypto.sha256.convert(bytes.takeBytes()).toString(),
    });
  }

  void _validate() {
    if (objectiveSlots.isEmpty ||
        objectiveSlots.length > _maxAuthoringRevision3QuestObjectives) {
      throw const FormatException(
        'a Quest transition plan must contain 1 to 8 objective slots',
      );
    }
    for (final slot in objectiveSlots) {
      _questTransitionsRequireObjectiveSlot(slot, 'objective slot');
    }
    for (var index = 1; index < objectiveSlots.length; index++) {
      if (objectiveSlots[index - 1] >= objectiveSlots[index]) {
        throw const FormatException(
          'Quest transition objective slots must be strictly ascending',
        );
      }
    }
    if (!objectiveSlots.contains(1)) {
      throw const FormatException(
        'Quest transition objective slot 1 must remain active',
      );
    }
    if (objectiveOrder.length != objectiveSlots.length ||
        objectiveOrder.toSet().length != objectiveOrder.length ||
        !objectiveOrder.every(objectiveSlots.contains)) {
      throw const FormatException(
        'Quest transition objective order must be a full slot permutation',
      );
    }
    if (nextSlotOrdinal <= objectiveSlots.last ||
        nextSlotOrdinal > _maxRevision3QuestTransitionObjectiveSlot) {
      throw const FormatException(
        'Quest transition next slot must be greater than every active slot',
      );
    }
    _questTransitionsRequireUnique(
      transitions.map((transition) => transition.stableKey),
      'Quest transition plan contains duplicate node edges',
    );
    _questTransitionsRequireStrictlySorted(
      transitions.map(_questTransitionsTransitionSortKey),
      'Quest transitions are not in canonical node/edge order',
    );
    final active = objectiveSlots.toSet();
    bool known(AuthoringRevision3QuestTransitionNodeV1 node) =>
        node.kind == AuthoringRevision3QuestTransitionNodeKind.root ||
        active.contains(node.slot);
    for (final transition in transitions) {
      if (!known(transition.node)) {
        throw const FormatException(
          'Quest transition references an inactive objective node',
        );
      }
      for (final group in transition.predicate?.anyOf ?? const []) {
        for (final atom in group.allOf) {
          if (!known(atom.node)) {
            throw const FormatException(
              'Quest transition condition references an inactive objective node',
            );
          }
        }
      }
      for (final effect in transition.effects) {
        if (!known(effect.target)) {
          throw const FormatException(
            'Quest transition effect references an inactive objective node',
          );
        }
      }
    }
    const root = AuthoringRevision3QuestTransitionNodeV1.root();
    final nodes = <AuthoringRevision3QuestTransitionNodeV1>[
      root,
      for (final slot in objectiveSlots)
        AuthoringRevision3QuestTransitionNodeV1.objective(slot),
    ];
    final byEdge = <String, AuthoringRevision3QuestTransitionV1>{
      for (final transition in transitions) transition.stableKey: transition,
    };
    for (final node in nodes) {
      for (final edge in const [
        AuthoringRevision3QuestTransitionEdgeV1.availability,
        AuthoringRevision3QuestTransitionEdgeV1.start,
      ]) {
        if (!byEdge.containsKey('${node.stableKey}:${edge.wireName}')) {
          throw const FormatException(
            'every Quest node requires availability and start transitions',
          );
        }
      }
      final success =
          byEdge['${node.stableKey}:${AuthoringRevision3QuestTransitionEdgeV1.success.wireName}'];
      final failure =
          byEdge['${node.stableKey}:${AuthoringRevision3QuestTransitionEdgeV1.failure.wireName}'];
      if (node.kind == AuthoringRevision3QuestTransitionNodeKind.objective &&
          success == null &&
          failure == null) {
        throw const FormatException(
          'every Quest objective requires success or failure',
        );
      }
      final successPredicate = success?.predicate;
      final failurePredicate = failure?.predicate;
      if (successPredicate != null &&
          failurePredicate != null &&
          _questTransitionsPredicatesMayOverlap(
            successPredicate,
            failurePredicate,
          )) {
        throw const FormatException(
          'Quest success and failure conditions must be provably disjoint',
        );
      }
    }
    for (final effectKind
        in AuthoringRevision3QuestTransitionEffectKindV1.values) {
      final graph = <String, Set<String>>{};
      for (final transition in transitions) {
        if (effectKind ==
                AuthoringRevision3QuestTransitionEffectKindV1.succeed &&
            transition.succeedsParent) {
          graph
              .putIfAbsent(transition.node.stableKey, () => <String>{})
              .add(root.stableKey);
        }
        for (final effect in transition.effects) {
          if (effect.effect == effectKind) {
            graph
                .putIfAbsent(transition.node.stableKey, () => <String>{})
                .add(effect.target.stableKey);
          }
        }
      }
      _questTransitionsRejectEffectCycle(
        graph,
        nodes.map((node) => node.stableKey),
      );
    }
  }
}

bool _questTransitionsPredicatesMayOverlap(
  AuthoringRevision3QuestTransitionPredicateV1 left,
  AuthoringRevision3QuestTransitionPredicateV1 right,
) {
  for (final leftGroup in left.anyOf) {
    for (final rightGroup in right.anyOf) {
      final polarities = <String, bool>{};
      final nodes = <AuthoringRevision3QuestTransitionNodeV1>{};
      var directlyContradictory = false;
      for (final atom in <AuthoringRevision3QuestTransitionConditionAtomV1>[
        ...leftGroup.allOf,
        ...rightGroup.allOf,
      ]) {
        nodes.add(atom.node);
        final key = '${atom.node.stableKey}:${atom.test.wireName}';
        final previous = polarities[key];
        if (previous != null && previous != atom.negated) {
          directlyContradictory = true;
          break;
        }
        polarities[key] = atom.negated;
      }
      if (!directlyContradictory &&
          !_questTransitionsHasLifecycleStateContradiction(polarities, nodes)) {
        return true;
      }
    }
  }
  return false;
}

bool _questTransitionsHasLifecycleStateContradiction(
  Map<String, bool> polarities,
  Iterable<AuthoringRevision3QuestTransitionNodeV1> nodes,
) {
  for (final node in nodes.toSet()) {
    bool positive(AuthoringRevision3QuestTransitionStateTestV1 test) =>
        polarities['${node.stableKey}:${test.wireName}'] == false;
    bool negative(AuthoringRevision3QuestTransitionStateTestV1 test) =>
        polarities['${node.stableKey}:${test.wireName}'] == true;
    final terminal =
        positive(AuthoringRevision3QuestTransitionStateTestV1.succeeded) ||
        positive(AuthoringRevision3QuestTransitionStateTestV1.failed) ||
        positive(AuthoringRevision3QuestTransitionStateTestV1.completed);
    if ((positive(AuthoringRevision3QuestTransitionStateTestV1.succeeded) &&
            positive(AuthoringRevision3QuestTransitionStateTestV1.failed)) ||
        (positive(AuthoringRevision3QuestTransitionStateTestV1.running) &&
            terminal) ||
        (negative(AuthoringRevision3QuestTransitionStateTestV1.started) &&
            (positive(AuthoringRevision3QuestTransitionStateTestV1.running) ||
                terminal)) ||
        (negative(AuthoringRevision3QuestTransitionStateTestV1.completed) &&
            (positive(AuthoringRevision3QuestTransitionStateTestV1.succeeded) ||
                positive(
                  AuthoringRevision3QuestTransitionStateTestV1.failed,
                ))) ||
        (positive(AuthoringRevision3QuestTransitionStateTestV1.completed) &&
            negative(AuthoringRevision3QuestTransitionStateTestV1.succeeded) &&
            negative(AuthoringRevision3QuestTransitionStateTestV1.failed))) {
      return true;
    }
  }
  return false;
}

List<int> _questTransitionsSlotList(Object? value, String context) {
  if (value is! List<Object?> ||
      value.isEmpty ||
      value.length > _maxAuthoringRevision3QuestObjectives) {
    throw FormatException('$context must contain 1 to 8 slots');
  }
  return List<int>.unmodifiable([
    for (var index = 0; index < value.length; index++)
      _questTransitionsRequireObjectiveSlot(
        value[index],
        '$context slot ${index + 1}',
      ),
  ]);
}

int _questTransitionsRequireObjectiveSlot(Object? value, String context) {
  if (value is! int ||
      value < 1 ||
      value > _maxRevision3QuestTransitionObjectiveSlot) {
    throw FormatException('$context must be an unsigned 16-bit nonzero slot');
  }
  return value;
}

void _questTransitionsRequireUnique(Iterable<String> values, String message) {
  final seen = <String>{};
  if (!values.every(seen.add)) throw FormatException(message);
}

void _questTransitionsRequireStrictlySorted(
  Iterable<String> values,
  String message,
) {
  String? previous;
  for (final value in values) {
    if (previous != null && previous.compareTo(value) >= 0) {
      throw FormatException(message);
    }
    previous = value;
  }
}

String _questTransitionsGroupStableKey(
  AuthoringRevision3QuestTransitionConditionGroupV1 group,
) => group.allOf.map(_questTransitionsAtomSortKey).join('|');

String _questTransitionsNodeSortKey(
  AuthoringRevision3QuestTransitionNodeV1 node,
) => switch (node.kind) {
  AuthoringRevision3QuestTransitionNodeKind.root => '00000',
  AuthoringRevision3QuestTransitionNodeKind.objective =>
    node.slot!.toString().padLeft(5, '0'),
};

String _questTransitionsAtomSortKey(
  AuthoringRevision3QuestTransitionConditionAtomV1 atom,
) {
  final test = switch (atom.test) {
    AuthoringRevision3QuestTransitionStateTestV1.available => '0',
    AuthoringRevision3QuestTransitionStateTestV1.running => '1',
    AuthoringRevision3QuestTransitionStateTestV1.started => '2',
    AuthoringRevision3QuestTransitionStateTestV1.succeeded => '3',
    AuthoringRevision3QuestTransitionStateTestV1.failed => '4',
    AuthoringRevision3QuestTransitionStateTestV1.completed => '5',
  };
  return '${_questTransitionsNodeSortKey(atom.node)}:$test:${atom.negated ? 1 : 0}';
}

String _questTransitionsEffectSortKey(
  AuthoringRevision3QuestTransitionEffectV1 effect,
) {
  final kind = switch (effect.effect) {
    AuthoringRevision3QuestTransitionEffectKindV1.start => '0',
    AuthoringRevision3QuestTransitionEffectKindV1.succeed => '1',
    AuthoringRevision3QuestTransitionEffectKindV1.fail => '2',
  };
  return '${_questTransitionsNodeSortKey(effect.target)}:$kind';
}

String _questTransitionsTransitionSortKey(
  AuthoringRevision3QuestTransitionV1 transition,
) {
  final node = _questTransitionsNodeSortKey(transition.node);
  final edge = switch (transition.edge) {
    AuthoringRevision3QuestTransitionEdgeV1.availability => '0',
    AuthoringRevision3QuestTransitionEdgeV1.start => '1',
    AuthoringRevision3QuestTransitionEdgeV1.success => '2',
    AuthoringRevision3QuestTransitionEdgeV1.failure => '3',
  };
  return '$node:$edge';
}

void _questTransitionsRejectEffectCycle(
  Map<String, Set<String>> graph,
  Iterable<String> nodes,
) {
  final visiting = <String>{};
  final visited = <String>{};

  bool visit(String node) {
    if (visited.contains(node)) return false;
    if (!visiting.add(node)) return true;
    for (final target in graph[node] ?? const <String>{}) {
      if (visit(target)) return true;
    }
    visiting.remove(node);
    visited.add(node);
    return false;
  }

  if (nodes.any(visit)) {
    throw const FormatException(
      'same-kind Quest transition effects must not form a cycle',
    );
  }
}

void _questTransitionsRequireExactDelta(
  Map<String, Object?> base,
  Map<String, Object?> candidate, {
  required AuthoringRevision3QuestTransitionsEditRequestV1 request,
  required int questRevision,
  required int moduleRevision,
}) {
  for (final field in const <String>[
    'format',
    'schema_revision',
    'project_id',
    'meta',
    'target',
    'authoring_locales',
    'asset_store',
  ]) {
    if (!_authoringJsonDeepEquals(base[field], candidate[field])) {
      throw FormatException(
        'authoring revision-3 Quest transitions candidate changed basis field $field',
      );
    }
  }
  final baseEntities = _authoringRequiredObject(
    base['entities'],
    'revision-3 Quest transitions basis entities',
  );
  final candidateEntities = _authoringRequiredObject(
    candidate['entities'],
    'revision-3 Quest transitions candidate entities',
  );
  if (baseEntities.length != candidateEntities.length ||
      !_questTransitionsSameStrings(
        baseEntities.keys.toList(growable: false),
        candidateEntities.keys.toList(growable: false),
      )) {
    throw const FormatException(
      'authoring revision-3 Quest transitions candidate changed the entity set',
    );
  }
  for (final entry in baseEntities.entries) {
    if (entry.key != request.questId &&
        entry.key != request.moduleId &&
        !_authoringJsonDeepEquals(entry.value, candidateEntities[entry.key])) {
      throw const FormatException(
        'authoring revision-3 Quest transitions candidate changed an unrelated entity',
      );
    }
  }

  final baseQuest = _questOutlineEntity(
    baseEntities,
    request.questId,
    'quest_draft',
  );
  final candidateQuest = _questOutlineEntity(
    candidateEntities,
    request.questId,
    'quest_draft',
  );
  final candidateBasis = _questTransitionsRequireBasis(
    candidate,
    projectId: request.expectedProjectId,
    questId: request.questId,
  );
  if (candidateQuest.entity['revision'] != questRevision ||
      candidateBasis.generatorVersion !=
          _authoringRevision3SemanticQuestGeneratorVersion ||
      candidateBasis.legacySynthetic ||
      candidateBasis.transitionPlan.canonicalJson !=
          request.transitionPlan.canonicalJson) {
    throw const FormatException(
      'authoring revision-3 Quest transitions candidate disagrees with the requested plan',
    );
  }
  final normalizedQuest = _questOutlineClone(candidateQuest.entity);
  final baseQuestObject = _authoringRequiredObject(
    baseEntities[request.questId],
    'revision-3 Quest transitions basis Quest',
  );
  normalizedQuest['revision'] = baseQuestObject['revision'];
  final normalizedQuestData = _questOutlineMutableObject(
    _questOutlineMutableObject(
      normalizedQuest['payload'],
      'normalized transition Quest payload',
    )['data'],
    'normalized transition Quest data',
  );
  normalizedQuestData['generator_version'] =
      baseQuest.data['generator_version'];
  final normalizedInput = _questOutlineMutableObject(
    normalizedQuestData['input'],
    'normalized transition Quest input',
  );
  final baseInput = _authoringRequiredObject(
    baseQuest.data['input'],
    'revision-3 Quest transitions basis input',
  );
  if (baseInput.containsKey('transition_plan')) {
    normalizedInput['transition_plan'] = baseInput['transition_plan'];
  } else {
    normalizedInput.remove('transition_plan');
  }
  if (!_authoringJsonDeepEquals(normalizedQuest, baseQuestObject)) {
    throw const FormatException(
      'authoring revision-3 Quest transitions candidate changed a non-plan Quest field',
    );
  }

  final baseModule = _questOutlineEntity(
    baseEntities,
    request.moduleId,
    'script_module',
  );
  final candidateModule = _questOutlineEntity(
    candidateEntities,
    request.moduleId,
    'script_module',
  );
  if (candidateModule.entity['revision'] != moduleRevision ||
      candidateModule.data['generator_id'] !=
          _authoringRevision3QuestGeneratorId ||
      candidateModule.data['generator_version'] !=
          _authoringRevision3SemanticQuestGeneratorVersion) {
    throw const FormatException(
      'authoring revision-3 Quest transitions candidate module contract is not exact',
    );
  }
  final source = _authoringRequiredString(
    candidateModule.data,
    'source',
    maxBytes: _maxAuthoringDraftSourceBytes,
  );
  final sourceSha = _authoringRequiredString(
    candidateModule.data,
    'source_sha256',
    maxBytes: 64,
  );
  final inputFingerprint = _authoringRequiredString(
    candidateModule.data,
    'input_fingerprint',
    maxBytes: 64,
  );
  final candidateInput = _authoringRequiredObject(
    candidateQuest.data['input'],
    'revision-3 Quest transitions candidate input',
  );
  if (!_authoringSha256Pattern.hasMatch(sourceSha) ||
      !_authoringSha256Pattern.hasMatch(inputFingerprint) ||
      crypto.sha256.convert(utf8.encode(source)).toString() != sourceSha ||
      _authoringRevision3QuestInputFingerprint(candidateInput) !=
          inputFingerprint) {
    throw const FormatException(
      'authoring revision-3 Quest transitions candidate module seals disagree',
    );
  }
  final normalizedModule = _questOutlineClone(candidateModule.entity);
  final baseModuleObject = _authoringRequiredObject(
    baseEntities[request.moduleId],
    'revision-3 Quest transitions basis module',
  );
  normalizedModule['revision'] = baseModuleObject['revision'];
  final normalizedOrigin = _questOutlineMutableObject(
    normalizedModule['origin'],
    'normalized transition module origin',
  );
  normalizedOrigin['generator_version'] = _authoringRequiredObject(
    baseModuleObject['origin'],
    'revision-3 Quest transitions basis module origin',
  )['generator_version'];
  final normalizedModuleData = _questOutlineMutableObject(
    _questOutlineMutableObject(
      normalizedModule['payload'],
      'normalized transition module payload',
    )['data'],
    'normalized transition module data',
  );
  normalizedModuleData['generator_version'] =
      baseModule.data['generator_version'];
  for (final field in const <String>[
    'source',
    'source_sha256',
    'input_fingerprint',
  ]) {
    normalizedModuleData[field] = baseModule.data[field];
  }
  if (!_authoringJsonDeepEquals(normalizedModule, baseModuleObject)) {
    throw const FormatException(
      'authoring revision-3 Quest transitions candidate changed a non-generated module field',
    );
  }
}

({
  int questRevision,
  String moduleId,
  int moduleRevision,
  int generatorVersion,
  List<AuthoringRevision3QuestTransitionObjectiveV1> objectives,
  AuthoringRevision3QuestTransitionPlanV1 transitionPlan,
  bool legacySynthetic,
})
_questTransitionsRequireBasis(
  Map<String, Object?> project, {
  required String projectId,
  required String questId,
}) {
  final entities = _authoringRequiredObject(
    project['entities'],
    'revision-3 Quest transitions basis entities',
  );
  final quest = _questOutlineEntity(entities, questId, 'quest_draft');
  final hasTranscript = quest.data.containsKey('transcript');
  _authoringExactFields(quest.data, <String>{
    'generator_id',
    'generator_version',
    'input',
    'script_module',
    if (hasTranscript) 'transcript',
  }, 'revision-3 Quest transitions Quest data');
  if (quest.data['generator_id'] != _authoringRevision3QuestGeneratorId) {
    throw const FormatException(
      'revision-3 Quest transitions basis uses an unsupported generator',
    );
  }
  final generatorVersion = _authoringRequiredInt(
    quest.data,
    'generator_version',
    min: _authoringRevision3QuestGeneratorVersion,
    max: _authoringRevision3SemanticQuestGeneratorVersion,
  );
  final scriptRef = _authoringRequiredObject(
    quest.data['script_module'],
    'revision-3 Quest transitions module reference',
  );
  _authoringExactFields(scriptRef, const {
    'project_id',
    'id',
    'expected_kind',
  }, 'revision-3 Quest transitions module reference');
  final moduleId = _questTransitionsEntityId(scriptRef, 'id');
  if (scriptRef['project_id'] != projectId ||
      scriptRef['expected_kind'] != 'script_module') {
    throw const FormatException(
      'revision-3 Quest transitions basis has a foreign or mistyped module reference',
    );
  }
  final module = _questOutlineEntity(entities, moduleId, 'script_module');
  if (module.data['generator_id'] != _authoringRevision3QuestGeneratorId ||
      module.data['generator_version'] != generatorVersion) {
    throw const FormatException(
      'revision-3 Quest transitions Quest/module generator versions disagree',
    );
  }

  final input = _authoringRequiredObject(
    quest.data['input'],
    'revision-3 Quest transitions input',
  );
  final hasAdditional = input.containsKey('additional_objective_titles');
  final hasPlan = input.containsKey('transition_plan');
  _authoringExactFields(input, <String>{
    'target',
    'quest_id',
    'module_namespace',
    'technical_id',
    'text_helper',
    'parent_quest',
    'giver',
    'title',
    'description',
    'objective_title',
    if (hasAdditional) 'additional_objective_titles',
    if (hasPlan) 'transition_plan',
    'collision_catalog',
  }, 'revision-3 Quest transitions input');
  if (input['quest_id'] != questId ||
      jsonEncode(input['target']) != jsonEncode(project['target'])) {
    throw const FormatException(
      'revision-3 Quest transitions Quest identity or target is not exact',
    );
  }
  final firstTitle = _questTransitionsLiteral(input, 'objective_title');
  final additional = hasAdditional
      ? _authoringRevision3QuestObjectiveTitleList(
          input['additional_objective_titles'],
          firstTitle: firstTitle,
          requireAdditional: true,
          context: 'transition basis',
        )
      : const <String>[];
  final titles = <String>[firstTitle, ...additional];
  if (switch (generatorVersion) {
    _authoringRevision3QuestGeneratorVersion =>
      hasAdditional || hasPlan || titles.length != 1,
    _authoringRevision3MultiObjectiveQuestGeneratorVersion =>
      !hasAdditional || hasPlan || titles.length < 2,
    _authoringRevision3SemanticQuestGeneratorVersion => !hasPlan,
    _ => true,
  }) {
    throw const FormatException(
      'revision-3 Quest transitions input does not match its generator version',
    );
  }
  final legacySynthetic =
      generatorVersion != _authoringRevision3SemanticQuestGeneratorVersion;
  final plan = legacySynthetic
      ? AuthoringRevision3QuestTransitionPlanV1.legacySeed(titles.length)
      : AuthoringRevision3QuestTransitionPlanV1.fromJson(
          input['transition_plan'],
        );
  if (plan.objectiveOrder.length != titles.length) {
    throw const FormatException(
      'revision-3 Quest transition plan does not cover every objective title',
    );
  }
  return (
    questRevision: _authoringRequiredInt(
      quest.entity,
      'revision',
      max: _maxAuthoringStoryBaseRevision,
    ),
    moduleId: moduleId,
    moduleRevision: _authoringRequiredInt(
      module.entity,
      'revision',
      max: _maxAuthoringStoryBaseRevision,
    ),
    generatorVersion: generatorVersion,
    objectives:
        List<AuthoringRevision3QuestTransitionObjectiveV1>.unmodifiable([
          for (var index = 0; index < titles.length; index++)
            AuthoringRevision3QuestTransitionObjectiveV1(
              slot: plan.objectiveOrder[index],
              title: titles[index],
            ),
        ]),
    transitionPlan: plan,
    legacySynthetic: legacySynthetic,
  );
}

Map<String, Object?> _questTransitionsSealJson(
  AuthoringDraftContentSeal seal,
) => <String, Object?>{'byte_len': seal.byteLength, 'sha256': seal.sha256};

bool _questTransitionsSameSeal(
  AuthoringDraftContentSeal left,
  AuthoringDraftContentSeal right,
) => left.byteLength == right.byteLength && left.sha256 == right.sha256;

String _questTransitionsEntityId(Map<String, Object?> json, String field) {
  final value = _authoringRequiredString(json, field, maxBytes: 32);
  if (!_authoringEntityIdPattern.hasMatch(value)) {
    throw FormatException(
      'revision-3 Quest transitions $field is not an entity ID',
    );
  }
  return value;
}

String _questTransitionsLiteral(Map<String, Object?> json, String field) {
  final value = _authoringRequiredString(
    json,
    field,
    maxBytes: _maxAuthoringRevision3QuestObjectiveTitleBytes,
  );
  _authoringRevision3QuestValidateObjectiveTitle(value, 'transition $field');
  return value;
}

bool _questTransitionsSameStrings(List<String> left, List<String> right) {
  if (left.length != right.length) return false;
  for (var index = 0; index < left.length; index++) {
    if (left[index] != right[index]) return false;
  }
  return true;
}

bool _questTransitionsSameInts(List<int> left, List<int> right) {
  if (left.length != right.length) return false;
  for (var index = 0; index < left.length; index++) {
    if (left[index] != right[index]) return false;
  }
  return true;
}
