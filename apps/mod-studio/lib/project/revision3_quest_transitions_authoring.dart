import '../core/mod_ffi.dart';
import 'revision3_content_index.dart';

typedef Revision3QuestTransitionsSeedLoader =
    Future<AuthoringRevision3QuestTransitionsSeed> Function({
      required String questId,
      required int expectedQuestRevision,
      required String expectedModuleId,
      required int expectedModuleRevision,
    });

typedef Revision3QuestTransitionsTechnicalPublisher =
    Future<Revision3QuestTransitionsEditPublication> Function({
      required Revision3QuestTransitionsEditTechnicalPlan plan,
    });

/// Exact visible content checkpoint plus the native-validated transition seed.
final class Revision3QuestTransitionsEditCheckpoint {
  const Revision3QuestTransitionsEditCheckpoint._({
    required this.index,
    required this.quest,
    required this.seed,
  });

  final Revision3ContentIndex index;
  final Revision3ContentEntity quest;
  final AuthoringRevision3QuestTransitionsSeed seed;

  AuthoringRevision3QuestTransitionPlanV1 get transitionPlan =>
      seed.transitionPlan;

  List<AuthoringRevision3QuestTransitionObjectiveV1> get objectives =>
      seed.objectives;

  String objectiveTitle(int slot) => objectives
      .firstWhere(
        (objective) => objective.slot == slot,
        orElse: () => throw const FormatException(
          'The transition plan references an unknown Quest objective.',
        ),
      )
      .title;
}

/// Exact CAS-bound intent. The dialog never needs to handle these identities.
final class Revision3QuestTransitionsEditTechnicalPlan {
  const Revision3QuestTransitionsEditTechnicalPlan._({
    required this.questId,
    required this.expectedQuestRevision,
    required this.moduleId,
    required this.expectedModuleRevision,
    required this.expectedTransitionPlanSeal,
    required this.transitionPlan,
  });

  final String questId;
  final int expectedQuestRevision;
  final String moduleId;
  final int expectedModuleRevision;
  final AuthoringDraftContentSeal expectedTransitionPlanSeal;
  final AuthoringRevision3QuestTransitionPlanV1 transitionPlan;
}

/// The new immutable project checkpoint returned by the integration boundary.
final class Revision3QuestTransitionsEditPublication {
  const Revision3QuestTransitionsEditPublication({
    required this.projectId,
    required this.projectRevision,
    required this.questId,
    required this.moduleId,
    required this.questRevision,
    required this.moduleRevision,
    required this.transitionPlanSeal,
  });

  final String projectId;
  final int projectRevision;
  final String questId;
  final String moduleId;
  final int questRevision;
  final int moduleRevision;
  final AuthoringDraftContentSeal transitionPlanSeal;
}

final class Revision3QuestTransitionsRequiresReopenException
    implements Exception {
  const Revision3QuestTransitionsRequiresReopenException();
}

final class Revision3QuestTransitionsStaleCheckpointException
    implements Exception {
  const Revision3QuestTransitionsStaleCheckpointException();
}

/// Checkpoint-safe orchestration and canonical helpers for the visual editor.
final class Revision3QuestTransitionsAuthoringService {
  const Revision3QuestTransitionsAuthoringService({
    required this.loadSeed,
    required this.publishTechnicalPlan,
  });

  final Revision3QuestTransitionsSeedLoader loadSeed;
  final Revision3QuestTransitionsTechnicalPublisher publishTechnicalPlan;

  Future<Revision3QuestTransitionsEditCheckpoint> load({
    required Revision3ContentIndex index,
    required Revision3ContentEntity quest,
  }) async {
    _requireVisibleQuest(index, quest);
    final module = _requireVisibleQuestModule(index, quest);
    final seed = await loadSeed(
      questId: quest.id,
      expectedQuestRevision: quest.revision,
      expectedModuleId: module.id,
      expectedModuleRevision: module.revision,
    );
    _requireSeedBinding(index, quest, module, seed);
    return Revision3QuestTransitionsEditCheckpoint._(
      index: index,
      quest: quest,
      seed: seed,
    );
  }

  Future<Revision3QuestTransitionsEditPublication> publish({
    required Revision3QuestTransitionsEditCheckpoint checkpoint,
    required AuthoringRevision3QuestTransitionPlanV1 transitionPlan,
  }) async {
    final module = _requireVisibleQuestModule(
      checkpoint.index,
      checkpoint.quest,
    );
    _requireSeedBinding(
      checkpoint.index,
      checkpoint.quest,
      module,
      checkpoint.seed,
    );
    validateEditablePlan(checkpoint, transitionPlan);
    if (transitionPlan.canonicalJson ==
        checkpoint.seed.transitionPlan.canonicalJson) {
      throw const FormatException('Change at least one Quest behavior.');
    }

    final publication = await publishTechnicalPlan(
      plan: Revision3QuestTransitionsEditTechnicalPlan._(
        questId: checkpoint.seed.questId,
        expectedQuestRevision: checkpoint.seed.questRevision,
        moduleId: checkpoint.seed.moduleId,
        expectedModuleRevision: checkpoint.seed.moduleRevision,
        expectedTransitionPlanSeal: checkpoint.seed.transitionPlanSeal,
        transitionPlan: transitionPlan,
      ),
    );
    final expectedSeal = transitionPlan.contentSeal;
    if (publication.projectId != checkpoint.seed.projectId ||
        publication.projectRevision != checkpoint.seed.projectRevision + 1 ||
        publication.questId != checkpoint.seed.questId ||
        publication.moduleId != checkpoint.seed.moduleId ||
        publication.questRevision != checkpoint.seed.questRevision + 1 ||
        publication.moduleRevision != checkpoint.seed.moduleRevision + 1 ||
        !_sameSeal(publication.transitionPlanSeal, expectedSeal)) {
      throw const Revision3QuestTransitionsRequiresReopenException();
    }
    return publication;
  }

  static void validateEditablePlan(
    Revision3QuestTransitionsEditCheckpoint checkpoint,
    AuthoringRevision3QuestTransitionPlanV1 plan,
  ) {
    if (!_sameInts(
          plan.objectiveSlots,
          checkpoint.seed.transitionPlan.objectiveSlots,
        ) ||
        plan.nextSlotOrdinal < checkpoint.seed.transitionPlan.nextSlotOrdinal) {
      throw const FormatException(
        'Quest objectives changed outside this behavior editor. Reopen the Quest.',
      );
    }
  }

  /// A predictable chain: starting the Quest starts the first objective;
  /// each success starts the next; the final success completes the parent.
  static AuthoringRevision3QuestTransitionPlanV1 sequentialTemplate(
    AuthoringRevision3QuestTransitionPlanV1 basis,
  ) {
    final first = AuthoringRevision3QuestTransitionNodeV1.objective(
      basis.objectiveOrder.first,
    );
    final transitions = <AuthoringRevision3QuestTransitionV1>[
      AuthoringRevision3QuestTransitionV1(
        node: const AuthoringRevision3QuestTransitionNodeV1.root(),
        edge: AuthoringRevision3QuestTransitionEdgeV1.availability,
        externalAllowed: true,
      ),
      AuthoringRevision3QuestTransitionV1(
        node: const AuthoringRevision3QuestTransitionNodeV1.root(),
        edge: AuthoringRevision3QuestTransitionEdgeV1.start,
        externalAllowed: true,
        effects: <AuthoringRevision3QuestTransitionEffectV1>[
          AuthoringRevision3QuestTransitionEffectV1(
            target: first,
            effect: AuthoringRevision3QuestTransitionEffectKindV1.start,
          ),
        ],
      ),
    ];
    for (final slot in basis.objectiveSlots) {
      final node = AuthoringRevision3QuestTransitionNodeV1.objective(slot);
      final orderIndex = basis.objectiveOrder.indexOf(slot);
      final hasNext = orderIndex + 1 < basis.objectiveOrder.length;
      transitions.addAll(<AuthoringRevision3QuestTransitionV1>[
        AuthoringRevision3QuestTransitionV1(
          node: node,
          edge: AuthoringRevision3QuestTransitionEdgeV1.availability,
          externalAllowed: true,
        ),
        AuthoringRevision3QuestTransitionV1(
          node: node,
          edge: AuthoringRevision3QuestTransitionEdgeV1.start,
          externalAllowed: true,
        ),
        AuthoringRevision3QuestTransitionV1(
          node: node,
          edge: AuthoringRevision3QuestTransitionEdgeV1.success,
          externalAllowed: true,
          effects: hasNext
              ? <AuthoringRevision3QuestTransitionEffectV1>[
                  AuthoringRevision3QuestTransitionEffectV1(
                    target: AuthoringRevision3QuestTransitionNodeV1.objective(
                      basis.objectiveOrder[orderIndex + 1],
                    ),
                    effect: AuthoringRevision3QuestTransitionEffectKindV1.start,
                  ),
                ]
              : const <AuthoringRevision3QuestTransitionEffectV1>[],
          succeedsParent: !hasNext,
        ),
      ]);
    }
    return _rebuild(basis, transitions: transitions);
  }

  static AuthoringRevision3QuestTransitionPlanV1 reorderObjectives(
    AuthoringRevision3QuestTransitionPlanV1 basis,
    List<int> objectiveOrder,
  ) => AuthoringRevision3QuestTransitionPlanV1(
    objectiveSlots: basis.objectiveSlots,
    objectiveOrder: objectiveOrder,
    nextSlotOrdinal: basis.nextSlotOrdinal,
    transitions: basis.transitions,
  );

  static AuthoringRevision3QuestTransitionPlanV1 setTransition(
    AuthoringRevision3QuestTransitionPlanV1 basis,
    AuthoringRevision3QuestTransitionV1 transition,
  ) {
    final transitions = <AuthoringRevision3QuestTransitionV1>[
      for (final current in basis.transitions)
        if (current.stableKey != transition.stableKey) current,
      transition,
    ];
    return _rebuild(basis, transitions: transitions);
  }

  static AuthoringRevision3QuestTransitionPlanV1 removeOptionalTransition(
    AuthoringRevision3QuestTransitionPlanV1 basis, {
    required AuthoringRevision3QuestTransitionNodeV1 node,
    required AuthoringRevision3QuestTransitionEdgeV1 edge,
  }) {
    if (edge == AuthoringRevision3QuestTransitionEdgeV1.availability ||
        edge == AuthoringRevision3QuestTransitionEdgeV1.start) {
      throw const FormatException(
        'Only success or failure behavior can be removed.',
      );
    }
    final key = '${node.stableKey}:${edge.wireName}';
    return _rebuild(
      basis,
      transitions: <AuthoringRevision3QuestTransitionV1>[
        for (final transition in basis.transitions)
          if (transition.stableKey != key) transition,
      ],
    );
  }

  static AuthoringRevision3QuestTransitionConditionGroupV1 conditionGroup(
    Iterable<AuthoringRevision3QuestTransitionConditionAtomV1> atoms,
  ) {
    final sorted = atoms.toList()
      ..sort(
        (left, right) => _atomSortKey(left).compareTo(_atomSortKey(right)),
      );
    return AuthoringRevision3QuestTransitionConditionGroupV1(allOf: sorted);
  }

  static AuthoringRevision3QuestTransitionPredicateV1 predicate(
    Iterable<Iterable<AuthoringRevision3QuestTransitionConditionAtomV1>>
    alternatives,
  ) {
    final groups =
        <AuthoringRevision3QuestTransitionConditionGroupV1>[
          for (final atoms in alternatives) conditionGroup(atoms),
        ]..sort(
          (left, right) => _groupSortKey(left).compareTo(_groupSortKey(right)),
        );
    return AuthoringRevision3QuestTransitionPredicateV1(anyOf: groups);
  }

  static List<AuthoringRevision3QuestTransitionEffectV1> canonicalEffects(
    Iterable<AuthoringRevision3QuestTransitionEffectV1> effects,
  ) => List<AuthoringRevision3QuestTransitionEffectV1>.unmodifiable(
    effects.toList()..sort(
      (left, right) => _effectSortKey(left).compareTo(_effectSortKey(right)),
    ),
  );
}

AuthoringRevision3QuestTransitionPlanV1 _rebuild(
  AuthoringRevision3QuestTransitionPlanV1 basis, {
  required List<AuthoringRevision3QuestTransitionV1> transitions,
}) {
  transitions.sort(
    (left, right) =>
        _transitionSortKey(left).compareTo(_transitionSortKey(right)),
  );
  return AuthoringRevision3QuestTransitionPlanV1(
    objectiveSlots: basis.objectiveSlots,
    objectiveOrder: basis.objectiveOrder,
    nextSlotOrdinal: basis.nextSlotOrdinal,
    transitions: transitions,
  );
}

void _requireVisibleQuest(
  Revision3ContentIndex index,
  Revision3ContentEntity quest,
) {
  if (quest.kind != Revision3ContentEntityKind.questDraft ||
      quest.summary.questDraft == null ||
      !identical(index.entityById(quest.id), quest)) {
    throw const FormatException(
      'The selected item is not the exact Quest from this project view.',
    );
  }
}

Revision3ContentEntity _requireVisibleQuestModule(
  Revision3ContentIndex index,
  Revision3ContentEntity quest,
) {
  final references = quest.references
      .where(
        (reference) =>
            reference.role == 'draft_script_module' &&
            reference.qualifier == null &&
            reference.resolution ==
                Revision3ContentReferenceResolution.resolved &&
            reference.target.projectId == index.projectId &&
            reference.target.expectedKind ==
                Revision3ContentEntityKind.scriptModule,
      )
      .toList(growable: false);
  final module = references.length == 1
      ? index.entityById(references.single.target.entityId)
      : null;
  if (module == null ||
      module.kind != Revision3ContentEntityKind.scriptModule) {
    throw const FormatException(
      'The selected Quest does not own one exact generated script.',
    );
  }
  return module;
}

void _requireSeedBinding(
  Revision3ContentIndex index,
  Revision3ContentEntity quest,
  Revision3ContentEntity module,
  AuthoringRevision3QuestTransitionsSeed seed,
) {
  final summary = quest.summary.questDraft!;
  final titles = summary.objectiveTitles;
  final objectives = seed.objectives;
  var objectivesMatch =
      objectives.length == titles.length &&
      objectives.length == seed.transitionPlan.objectiveOrder.length;
  if (objectivesMatch) {
    for (var index = 0; index < objectives.length; index++) {
      if (objectives[index].title != titles[index] ||
          objectives[index].slot != seed.transitionPlan.objectiveOrder[index]) {
        objectivesMatch = false;
        break;
      }
    }
  }
  if (seed.projectId != index.projectId ||
      seed.projectRevision != index.projectRevision ||
      seed.questId != quest.id ||
      seed.questRevision != quest.revision ||
      seed.moduleId != module.id ||
      seed.moduleRevision != module.revision ||
      !_sameSeal(seed.transitionPlanSeal, seed.transitionPlan.contentSeal) ||
      !objectivesMatch) {
    throw const Revision3QuestTransitionsStaleCheckpointException();
  }
}

bool _sameSeal(
  AuthoringDraftContentSeal left,
  AuthoringDraftContentSeal right,
) => left.byteLength == right.byteLength && left.sha256 == right.sha256;

bool _sameInts(List<int> left, List<int> right) {
  if (left.length != right.length) return false;
  for (var index = 0; index < left.length; index++) {
    if (left[index] != right[index]) return false;
  }
  return true;
}

String _nodeSortKey(AuthoringRevision3QuestTransitionNodeV1 node) =>
    switch (node.kind) {
      AuthoringRevision3QuestTransitionNodeKind.root => '00000',
      AuthoringRevision3QuestTransitionNodeKind.objective =>
        node.slot!.toString().padLeft(5, '0'),
    };

String _atomSortKey(AuthoringRevision3QuestTransitionConditionAtomV1 atom) =>
    '${_nodeSortKey(atom.node)}:${atom.test.index}:${atom.negated ? 1 : 0}';

String _groupSortKey(AuthoringRevision3QuestTransitionConditionGroupV1 group) =>
    group.allOf.map(_atomSortKey).join('|');

String _effectSortKey(AuthoringRevision3QuestTransitionEffectV1 effect) =>
    '${_nodeSortKey(effect.target)}:${effect.effect.index}';

String _transitionSortKey(AuthoringRevision3QuestTransitionV1 transition) =>
    '${_nodeSortKey(transition.node)}:${transition.edge.index}';
