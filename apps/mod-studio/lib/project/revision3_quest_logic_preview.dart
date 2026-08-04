import '../core/mod_ffi.dart';

/// One deliberately small lifecycle model used by the offline Quest preview.
///
/// The five phases are conservative and mutually exclusive. `Started` and
/// `Completed` are derived observations. The revision-3 Quest generator emits
/// the engine state calls independently, so this preview neither represents nor
/// proves simultaneous engine-state combinations outside these five phases.
/// It is project logic only: it does not claim that the engine polls predicates
/// in this order, or that a particular game build runs the generated hooks.
enum Revision3QuestLogicPreviewPhase {
  unavailable,
  available,
  running,
  succeeded,
  failed,
}

final class Revision3QuestLogicPreviewNodeState {
  const Revision3QuestLogicPreviewNodeState(this.phase);

  final Revision3QuestLogicPreviewPhase phase;

  bool get available => phase == Revision3QuestLogicPreviewPhase.available;
  bool get running => phase == Revision3QuestLogicPreviewPhase.running;
  bool get started => switch (phase) {
    Revision3QuestLogicPreviewPhase.running ||
    Revision3QuestLogicPreviewPhase.succeeded ||
    Revision3QuestLogicPreviewPhase.failed => true,
    _ => false,
  };
  bool get succeeded => phase == Revision3QuestLogicPreviewPhase.succeeded;
  bool get failed => phase == Revision3QuestLogicPreviewPhase.failed;
  bool get completed => succeeded || failed;

  bool matches(AuthoringRevision3QuestTransitionStateTestV1 test) =>
      switch (test) {
        AuthoringRevision3QuestTransitionStateTestV1.available => available,
        AuthoringRevision3QuestTransitionStateTestV1.running => running,
        AuthoringRevision3QuestTransitionStateTestV1.started => started,
        AuthoringRevision3QuestTransitionStateTestV1.succeeded => succeeded,
        AuthoringRevision3QuestTransitionStateTestV1.failed => failed,
        AuthoringRevision3QuestTransitionStateTestV1.completed => completed,
      };
}

enum Revision3QuestLogicPreviewTraceKind {
  reset,
  external,
  predicate,
  effect,
  parentSuccess,
  ignored,
  refused,
}

final class Revision3QuestLogicPreviewTraceEntry {
  const Revision3QuestLogicPreviewTraceEntry({
    required this.sequence,
    required this.kind,
    required this.node,
    this.edge,
    this.source,
    this.detail,
  });

  final int sequence;
  final Revision3QuestLogicPreviewTraceKind kind;
  final AuthoringRevision3QuestTransitionNodeV1 node;
  final AuthoringRevision3QuestTransitionEdgeV1? edge;
  final AuthoringRevision3QuestTransitionNodeV1? source;
  final String? detail;
}

enum Revision3QuestLogicPreviewActionStatus { applied, ignored, refused }

final class Revision3QuestLogicPreviewActionResult {
  const Revision3QuestLogicPreviewActionResult({
    required this.status,
    this.message,
  });

  final Revision3QuestLogicPreviewActionStatus status;
  final String? message;
}

final class Revision3QuestLogicPreviewExternalTrigger {
  const Revision3QuestLogicPreviewExternalTrigger({
    required this.node,
    required this.edge,
    required this.enabled,
  });

  final AuthoringRevision3QuestTransitionNodeV1 node;
  final AuthoringRevision3QuestTransitionEdgeV1 edge;
  final bool enabled;
}

/// Pure, bounded simulator for the authored transition plan.
///
/// Predicate transitions are evaluated to a deterministic fixed point.
/// Follow-up actions use the same guards as the Rust emitter: Start only when
/// the target has not started; Succeed and Fail only while it is running.
/// Every public action is transactional, so a refused cascade leaves the
/// previous preview state intact.
final class Revision3QuestLogicPreview {
  Revision3QuestLogicPreview(
    this.plan, {
    this.maxCascadeOperations = 256,
    this.maxFixedPointRounds = 64,
    this.maxTraceEntries = 256,
  }) {
    if (maxCascadeOperations < 1 ||
        maxFixedPointRounds < 1 ||
        maxTraceEntries < 2) {
      throw ArgumentError('Quest preview bounds must be positive');
    }
    reset();
  }

  final AuthoringRevision3QuestTransitionPlanV1 plan;
  final int maxCascadeOperations;
  final int maxFixedPointRounds;
  final int maxTraceEntries;

  final Map<
    AuthoringRevision3QuestTransitionNodeV1,
    Revision3QuestLogicPreviewPhase
  >
  _phases = {};
  final List<Revision3QuestLogicPreviewTraceEntry> _trace = [];
  var _sequence = 0;
  var _traceWasTrimmed = false;

  Map<
    AuthoringRevision3QuestTransitionNodeV1,
    Revision3QuestLogicPreviewNodeState
  >
  get states => Map.unmodifiable(<
    AuthoringRevision3QuestTransitionNodeV1,
    Revision3QuestLogicPreviewNodeState
  >{
    for (final entry in _phases.entries)
      entry.key: Revision3QuestLogicPreviewNodeState(entry.value),
  });

  Revision3QuestLogicPreviewNodeState stateOf(
    AuthoringRevision3QuestTransitionNodeV1 node,
  ) => Revision3QuestLogicPreviewNodeState(
    _phases[node] ?? Revision3QuestLogicPreviewPhase.unavailable,
  );

  List<Revision3QuestLogicPreviewTraceEntry> get trace =>
      List.unmodifiable(_trace);

  bool get traceWasTrimmed => _traceWasTrimmed;

  /// Predicate alternatives that have no satisfying assignment in the five
  /// exclusive preview phases and therefore always evaluate false here.
  ///
  /// The Rust renderer still emits their independent engine state calls. This
  /// count is a preview-model boundary, not a runtime validity judgment.
  int get predicateConjunctionsOutsideExclusiveModel {
    var count = 0;
    for (final transition in plan.transitions) {
      final predicate = transition.predicate;
      if (predicate == null) continue;
      for (final group in predicate.anyOf) {
        if (!_canRepresentPredicateGroup(group)) count++;
      }
    }
    return count;
  }

  List<Revision3QuestLogicPreviewExternalTrigger> get externalTriggers =>
      List.unmodifiable(<Revision3QuestLogicPreviewExternalTrigger>[
        for (final transition in plan.transitions)
          if (transition.externalAllowed)
            Revision3QuestLogicPreviewExternalTrigger(
              node: transition.node,
              edge: transition.edge,
              enabled: _canApplyEdge(transition.node, transition.edge),
            ),
      ]);

  Revision3QuestLogicPreviewActionResult reset() {
    final priorPhases = Map.of(_phases);
    final priorTrace = List.of(_trace);
    final priorSequence = _sequence;
    final priorTraceWasTrimmed = _traceWasTrimmed;
    _phases
      ..clear()
      ..addAll(_initialPhases());
    _trace.clear();
    _sequence = 0;
    _traceWasTrimmed = false;
    _appendTrace(
      kind: Revision3QuestLogicPreviewTraceKind.reset,
      node: const AuthoringRevision3QuestTransitionNodeV1.root(),
      detail: 'Preview reset',
    );
    try {
      _settlePredicates(_PreviewOperationBudget(maxCascadeOperations));
      return const Revision3QuestLogicPreviewActionResult(
        status: Revision3QuestLogicPreviewActionStatus.applied,
      );
    } on _PreviewRefusal catch (error) {
      // Construction has no previously visible state. Keep a complete offline
      // baseline in that one case; every later reset restores the exact visible
      // predecessor, including its timeline metadata.
      _restore(
        priorPhases.isEmpty ? _initialPhases() : priorPhases,
        priorTrace,
        priorSequence,
        priorTraceWasTrimmed,
      );
      return Revision3QuestLogicPreviewActionResult(
        status: Revision3QuestLogicPreviewActionStatus.refused,
        message: error.message,
      );
    }
  }

  Revision3QuestLogicPreviewActionResult triggerExternal(
    AuthoringRevision3QuestTransitionNodeV1 node,
    AuthoringRevision3QuestTransitionEdgeV1 edge,
  ) {
    final transition = _findTransition(node, edge);
    if (transition == null || !transition.externalAllowed) {
      const message =
          'This edge is not externally allowed by the project plan.';
      _appendTrace(
        kind: Revision3QuestLogicPreviewTraceKind.refused,
        node: node,
        edge: edge,
        detail: message,
      );
      return const Revision3QuestLogicPreviewActionResult(
        status: Revision3QuestLogicPreviewActionStatus.refused,
        message: message,
      );
    }
    if (!_canApplyEdge(node, edge)) {
      const message = 'The lifecycle guard does not allow this edge now.';
      _appendTrace(
        kind: Revision3QuestLogicPreviewTraceKind.ignored,
        node: node,
        edge: edge,
        detail: message,
      );
      return const Revision3QuestLogicPreviewActionResult(
        status: Revision3QuestLogicPreviewActionStatus.ignored,
        message: message,
      );
    }

    final priorPhases = Map.of(_phases);
    final priorTrace = List.of(_trace);
    final priorSequence = _sequence;
    final priorTraceWasTrimmed = _traceWasTrimmed;
    try {
      final budget = _PreviewOperationBudget(maxCascadeOperations);
      final applied = _fireTransition(
        transition,
        kind: Revision3QuestLogicPreviewTraceKind.external,
        budget: budget,
      );
      _settlePredicates(budget);
      return Revision3QuestLogicPreviewActionResult(
        status: applied
            ? Revision3QuestLogicPreviewActionStatus.applied
            : Revision3QuestLogicPreviewActionStatus.ignored,
      );
    } on _PreviewRefusal catch (error) {
      _restore(priorPhases, priorTrace, priorSequence, priorTraceWasTrimmed);
      _appendTrace(
        kind: Revision3QuestLogicPreviewTraceKind.refused,
        node: node,
        edge: edge,
        detail: error.message,
      );
      return Revision3QuestLogicPreviewActionResult(
        status: Revision3QuestLogicPreviewActionStatus.refused,
        message: error.message,
      );
    }
  }

  void _settlePredicates(_PreviewOperationBudget budget) {
    final seen = <String>{};
    for (var round = 0; round < maxFixedPointRounds; round++) {
      final fingerprint = _fingerprint();
      if (!seen.add(fingerprint)) {
        throw const _PreviewRefusal(
          'Automatic conditions repeated a lifecycle state; preview refused the loop.',
        );
      }
      var changed = false;
      for (final transition in plan.transitions) {
        final predicate = transition.predicate;
        if (predicate == null ||
            !_canApplyEdge(transition.node, transition.edge) ||
            !_matchesPredicate(predicate)) {
          continue;
        }
        changed =
            _fireTransition(
              transition,
              kind: Revision3QuestLogicPreviewTraceKind.predicate,
              budget: budget,
            ) ||
            changed;
      }
      if (!changed) return;
    }
    throw const _PreviewRefusal(
      'Automatic conditions exceeded the bounded fixed-point preview.',
    );
  }

  bool _matchesPredicate(
    AuthoringRevision3QuestTransitionPredicateV1 predicate,
  ) => predicate.anyOf.any(
    (group) => group.allOf.every((atom) {
      final matches = stateOf(atom.node).matches(atom.test);
      return atom.negated ? !matches : matches;
    }),
  );

  bool _canRepresentPredicateGroup(
    AuthoringRevision3QuestTransitionConditionGroupV1 group,
  ) {
    final byNode =
        <
          AuthoringRevision3QuestTransitionNodeV1,
          List<AuthoringRevision3QuestTransitionConditionAtomV1>
        >{};
    for (final atom in group.allOf) {
      byNode.putIfAbsent(atom.node, () => []).add(atom);
    }
    for (final atoms in byNode.values) {
      final hasSatisfyingPhase = Revision3QuestLogicPreviewPhase.values.any((
        phase,
      ) {
        final state = Revision3QuestLogicPreviewNodeState(phase);
        return atoms.every((atom) {
          final matches = state.matches(atom.test);
          return atom.negated ? !matches : matches;
        });
      });
      if (!hasSatisfyingPhase) return false;
    }
    return true;
  }

  bool _fireTransition(
    AuthoringRevision3QuestTransitionV1 transition, {
    required Revision3QuestLogicPreviewTraceKind kind,
    required _PreviewOperationBudget budget,
    AuthoringRevision3QuestTransitionNodeV1? source,
  }) => _fireLifecycle(
    transition.node,
    transition.edge,
    transition: transition,
    kind: kind,
    budget: budget,
    source: source,
  );

  bool _fireLifecycle(
    AuthoringRevision3QuestTransitionNodeV1 node,
    AuthoringRevision3QuestTransitionEdgeV1 edge, {
    required AuthoringRevision3QuestTransitionV1? transition,
    required Revision3QuestLogicPreviewTraceKind kind,
    required _PreviewOperationBudget budget,
    AuthoringRevision3QuestTransitionNodeV1? source,
  }) {
    budget.consume();
    if (!_applyEdge(node, edge)) return false;
    _appendTrace(kind: kind, node: node, edge: edge, source: source);

    if (transition != null) {
      for (final effect in transition.effects) {
        final effectEdge = switch (effect.effect) {
          AuthoringRevision3QuestTransitionEffectKindV1.start =>
            AuthoringRevision3QuestTransitionEdgeV1.start,
          AuthoringRevision3QuestTransitionEffectKindV1.succeed =>
            AuthoringRevision3QuestTransitionEdgeV1.success,
          AuthoringRevision3QuestTransitionEffectKindV1.fail =>
            AuthoringRevision3QuestTransitionEdgeV1.failure,
        };
        if (!_canApplyEdge(effect.target, effectEdge)) {
          _appendTrace(
            kind: Revision3QuestLogicPreviewTraceKind.ignored,
            node: effect.target,
            edge: effectEdge,
            source: node,
            detail: switch (effect.effect) {
              AuthoringRevision3QuestTransitionEffectKindV1.start =>
                'Start skipped: target has already started.',
              AuthoringRevision3QuestTransitionEffectKindV1.succeed =>
                'Succeed skipped: target is not running.',
              AuthoringRevision3QuestTransitionEffectKindV1.fail =>
                'Fail skipped: target is not running.',
            },
          );
          continue;
        }
        _fireLifecycle(
          effect.target,
          effectEdge,
          transition: _findTransition(effect.target, effectEdge),
          kind: Revision3QuestLogicPreviewTraceKind.effect,
          budget: budget,
          source: node,
        );
      }
      if (transition.succeedsParent) {
        const root = AuthoringRevision3QuestTransitionNodeV1.root();
        if (_canApplyEdge(
          root,
          AuthoringRevision3QuestTransitionEdgeV1.success,
        )) {
          _fireLifecycle(
            root,
            AuthoringRevision3QuestTransitionEdgeV1.success,
            transition: _findTransition(
              root,
              AuthoringRevision3QuestTransitionEdgeV1.success,
            ),
            kind: Revision3QuestLogicPreviewTraceKind.parentSuccess,
            budget: budget,
            source: node,
          );
        } else {
          _appendTrace(
            kind: Revision3QuestLogicPreviewTraceKind.ignored,
            node: root,
            edge: AuthoringRevision3QuestTransitionEdgeV1.success,
            source: node,
            detail: 'Parent success skipped: Main Quest is not running.',
          );
        }
      }
    }
    return true;
  }

  bool _applyEdge(
    AuthoringRevision3QuestTransitionNodeV1 node,
    AuthoringRevision3QuestTransitionEdgeV1 edge,
  ) {
    if (!_canApplyEdge(node, edge)) return false;
    _phases[node] = switch (edge) {
      AuthoringRevision3QuestTransitionEdgeV1.availability =>
        Revision3QuestLogicPreviewPhase.available,
      AuthoringRevision3QuestTransitionEdgeV1.start =>
        Revision3QuestLogicPreviewPhase.running,
      AuthoringRevision3QuestTransitionEdgeV1.success =>
        Revision3QuestLogicPreviewPhase.succeeded,
      AuthoringRevision3QuestTransitionEdgeV1.failure =>
        Revision3QuestLogicPreviewPhase.failed,
    };
    return true;
  }

  bool _canApplyEdge(
    AuthoringRevision3QuestTransitionNodeV1 node,
    AuthoringRevision3QuestTransitionEdgeV1 edge,
  ) {
    final state = stateOf(node);
    return switch (edge) {
      AuthoringRevision3QuestTransitionEdgeV1.availability =>
        !state.started && !state.available,
      AuthoringRevision3QuestTransitionEdgeV1.start => !state.started,
      AuthoringRevision3QuestTransitionEdgeV1.success ||
      AuthoringRevision3QuestTransitionEdgeV1.failure => state.running,
    };
  }

  AuthoringRevision3QuestTransitionV1? _findTransition(
    AuthoringRevision3QuestTransitionNodeV1 node,
    AuthoringRevision3QuestTransitionEdgeV1 edge,
  ) {
    for (final transition in plan.transitions) {
      if (transition.node == node && transition.edge == edge) {
        return transition;
      }
    }
    return null;
  }

  String _fingerprint() => [
    const AuthoringRevision3QuestTransitionNodeV1.root(),
    for (final slot in plan.objectiveSlots)
      AuthoringRevision3QuestTransitionNodeV1.objective(slot),
  ].map((node) => '${node.stableKey}:${_phases[node]!.index}').join('|');

  Map<AuthoringRevision3QuestTransitionNodeV1, Revision3QuestLogicPreviewPhase>
  _initialPhases() =>
      <
        AuthoringRevision3QuestTransitionNodeV1,
        Revision3QuestLogicPreviewPhase
      >{
        const AuthoringRevision3QuestTransitionNodeV1.root():
            Revision3QuestLogicPreviewPhase.unavailable,
        for (final slot in plan.objectiveSlots)
          AuthoringRevision3QuestTransitionNodeV1.objective(slot):
              Revision3QuestLogicPreviewPhase.unavailable,
      };

  void _restore(
    Map<
      AuthoringRevision3QuestTransitionNodeV1,
      Revision3QuestLogicPreviewPhase
    >
    phases,
    List<Revision3QuestLogicPreviewTraceEntry> trace,
    int sequence,
    bool traceWasTrimmed,
  ) {
    _phases
      ..clear()
      ..addAll(phases);
    _trace
      ..clear()
      ..addAll(trace);
    _sequence = sequence;
    _traceWasTrimmed = traceWasTrimmed;
  }

  void _appendTrace({
    required Revision3QuestLogicPreviewTraceKind kind,
    required AuthoringRevision3QuestTransitionNodeV1 node,
    AuthoringRevision3QuestTransitionEdgeV1? edge,
    AuthoringRevision3QuestTransitionNodeV1? source,
    String? detail,
  }) {
    _trace.add(
      Revision3QuestLogicPreviewTraceEntry(
        sequence: ++_sequence,
        kind: kind,
        node: node,
        edge: edge,
        source: source,
        detail: detail,
      ),
    );
    if (_trace.length > maxTraceEntries) {
      _trace.removeAt(0);
      _traceWasTrimmed = true;
    }
  }
}

final class _PreviewOperationBudget {
  _PreviewOperationBudget(this.maximum);

  final int maximum;
  var _used = 0;

  void consume() {
    if (++_used > maximum) {
      throw const _PreviewRefusal(
        'Quest logic exceeded the bounded cascade preview; no preview state was changed.',
      );
    }
  }
}

final class _PreviewRefusal implements Exception {
  const _PreviewRefusal(this.message);

  final String message;
}
