import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/core/mod_ffi.dart';
import 'package:gore_mod/project/revision3_quest_logic_preview.dart';
import 'package:gore_mod/project/revision3_quest_transitions_authoring.dart';

void main() {
  test(
    'sequential template cascades and final success completes the parent',
    () {
      final plan = Revision3QuestTransitionsAuthoringService.sequentialTemplate(
        AuthoringRevision3QuestTransitionPlanV1.legacySeed(3),
      );
      final preview = Revision3QuestLogicPreview(plan);
      const root = AuthoringRevision3QuestTransitionNodeV1.root();
      final first = AuthoringRevision3QuestTransitionNodeV1.objective(1);
      final second = AuthoringRevision3QuestTransitionNodeV1.objective(2);
      final third = AuthoringRevision3QuestTransitionNodeV1.objective(3);

      expect(
        preview
            .triggerExternal(
              root,
              AuthoringRevision3QuestTransitionEdgeV1.start,
            )
            .status,
        Revision3QuestLogicPreviewActionStatus.applied,
      );
      expect(preview.stateOf(root).running, isTrue);
      expect(preview.stateOf(first).running, isTrue);

      preview.triggerExternal(
        first,
        AuthoringRevision3QuestTransitionEdgeV1.success,
      );
      expect(preview.stateOf(first).succeeded, isTrue);
      expect(preview.stateOf(second).running, isTrue);

      preview.triggerExternal(
        second,
        AuthoringRevision3QuestTransitionEdgeV1.success,
      );
      expect(preview.stateOf(third).running, isTrue);
      preview.triggerExternal(
        third,
        AuthoringRevision3QuestTransitionEdgeV1.success,
      );

      expect(preview.stateOf(third).completed, isTrue);
      expect(preview.stateOf(root).succeeded, isTrue);
      expect(
        preview.trace.map((entry) => entry.kind),
        contains(Revision3QuestLogicPreviewTraceKind.parentSuccess),
      );

      preview.reset();
      expect(preview.stateOf(root).started, isFalse);
      expect(preview.stateOf(first).available, isFalse);
      expect(
        preview.trace.single.kind,
        Revision3QuestLogicPreviewTraceKind.reset,
      );
    },
  );

  test(
    'predicates and negation settle to a fixed point without external access',
    () {
      var plan = AuthoringRevision3QuestTransitionPlanV1.legacySeed(2);
      const root = AuthoringRevision3QuestTransitionNodeV1.root();
      final first = AuthoringRevision3QuestTransitionNodeV1.objective(1);
      final second = AuthoringRevision3QuestTransitionNodeV1.objective(2);
      plan = Revision3QuestTransitionsAuthoringService.setTransition(
        plan,
        AuthoringRevision3QuestTransitionV1(
          node: root,
          edge: AuthoringRevision3QuestTransitionEdgeV1.availability,
          externalAllowed: false,
          predicate: Revision3QuestTransitionsAuthoringService.predicate([
            [
              AuthoringRevision3QuestTransitionConditionAtomV1(
                node: first,
                test: AuthoringRevision3QuestTransitionStateTestV1.started,
                negated: true,
              ),
            ],
          ]),
        ),
      );
      plan = Revision3QuestTransitionsAuthoringService.setTransition(
        plan,
        AuthoringRevision3QuestTransitionV1(
          node: second,
          edge: AuthoringRevision3QuestTransitionEdgeV1.availability,
          externalAllowed: false,
          predicate: Revision3QuestTransitionsAuthoringService.predicate([
            [
              const AuthoringRevision3QuestTransitionConditionAtomV1(
                node: root,
                test: AuthoringRevision3QuestTransitionStateTestV1.available,
                negated: false,
              ),
            ],
          ]),
        ),
      );

      final preview = Revision3QuestLogicPreview(plan);

      expect(preview.stateOf(root).available, isTrue);
      expect(preview.stateOf(second).available, isTrue);
      expect(
        preview.externalTriggers.any(
          (trigger) =>
              trigger.node == root &&
              trigger.edge ==
                  AuthoringRevision3QuestTransitionEdgeV1.availability,
        ),
        isFalse,
      );
      expect(
        preview
            .triggerExternal(
              root,
              AuthoringRevision3QuestTransitionEdgeV1.availability,
            )
            .status,
        Revision3QuestLogicPreviewActionStatus.refused,
      );
    },
  );

  test('marks conjunctions outside the five exclusive preview phases', () {
    var plan = AuthoringRevision3QuestTransitionPlanV1.legacySeed(1);
    const root = AuthoringRevision3QuestTransitionNodeV1.root();
    plan = Revision3QuestTransitionsAuthoringService.setTransition(
      plan,
      AuthoringRevision3QuestTransitionV1(
        node: root,
        edge: AuthoringRevision3QuestTransitionEdgeV1.availability,
        externalAllowed: true,
        predicate: Revision3QuestTransitionsAuthoringService.predicate([
          [
            const AuthoringRevision3QuestTransitionConditionAtomV1(
              node: root,
              test: AuthoringRevision3QuestTransitionStateTestV1.available,
              negated: false,
            ),
            const AuthoringRevision3QuestTransitionConditionAtomV1(
              node: root,
              test: AuthoringRevision3QuestTransitionStateTestV1.running,
              negated: false,
            ),
          ],
        ]),
      ),
    );

    final preview = Revision3QuestLogicPreview(plan);

    expect(preview.predicateConjunctionsOutsideExclusiveModel, 1);
    expect(
      preview.stateOf(root).phase,
      Revision3QuestLogicPreviewPhase.unavailable,
    );
  });

  test('follow-up actions use generated start and terminal guards', () {
    var plan = AuthoringRevision3QuestTransitionPlanV1.legacySeed(3);
    const root = AuthoringRevision3QuestTransitionNodeV1.root();
    final first = AuthoringRevision3QuestTransitionNodeV1.objective(1);
    final second = AuthoringRevision3QuestTransitionNodeV1.objective(2);
    plan = Revision3QuestTransitionsAuthoringService.setTransition(
      plan,
      AuthoringRevision3QuestTransitionV1(
        node: root,
        edge: AuthoringRevision3QuestTransitionEdgeV1.start,
        externalAllowed: true,
        effects: Revision3QuestTransitionsAuthoringService.canonicalEffects([
          AuthoringRevision3QuestTransitionEffectV1(
            target: first,
            effect: AuthoringRevision3QuestTransitionEffectKindV1.start,
          ),
          AuthoringRevision3QuestTransitionEffectV1(
            target: second,
            effect: AuthoringRevision3QuestTransitionEffectKindV1.succeed,
          ),
        ]),
      ),
    );
    final preview = Revision3QuestLogicPreview(plan);

    preview.triggerExternal(
      first,
      AuthoringRevision3QuestTransitionEdgeV1.start,
    );
    preview.triggerExternal(
      root,
      AuthoringRevision3QuestTransitionEdgeV1.start,
    );

    expect(preview.stateOf(first).running, isTrue);
    expect(preview.stateOf(second).started, isFalse);
    final ignored = preview.trace
        .where(
          (entry) => entry.kind == Revision3QuestLogicPreviewTraceKind.ignored,
        )
        .map((entry) => entry.detail);
    expect(ignored, contains('Start skipped: target has already started.'));
    expect(ignored, contains('Succeed skipped: target is not running.'));

    preview.reset();
    preview.triggerExternal(
      second,
      AuthoringRevision3QuestTransitionEdgeV1.start,
    );
    preview.triggerExternal(
      root,
      AuthoringRevision3QuestTransitionEdgeV1.start,
    );
    expect(preview.stateOf(first).running, isTrue);
    expect(preview.stateOf(second).succeeded, isTrue);
  });

  test('bounded refusal restores state and the complete trimmed trace', () {
    final plan = Revision3QuestTransitionsAuthoringService.sequentialTemplate(
      AuthoringRevision3QuestTransitionPlanV1.legacySeed(2),
    );
    final preview = Revision3QuestLogicPreview(
      plan,
      maxCascadeOperations: 1,
      maxTraceEntries: 2,
    );
    const root = AuthoringRevision3QuestTransitionNodeV1.root();
    final first = AuthoringRevision3QuestTransitionNodeV1.objective(1);
    preview.triggerExternal(
      first,
      AuthoringRevision3QuestTransitionEdgeV1.availability,
    );
    final traceBefore = preview.trace;

    final result = preview.triggerExternal(
      root,
      AuthoringRevision3QuestTransitionEdgeV1.start,
    );

    expect(result.status, Revision3QuestLogicPreviewActionStatus.refused);
    expect(preview.stateOf(root).started, isFalse);
    expect(preview.stateOf(first).available, isTrue);
    expect(preview.trace.length, 2);
    expect(preview.trace.first.sequence, traceBefore.last.sequence);
    expect(
      preview.trace.last.kind,
      Revision3QuestLogicPreviewTraceKind.refused,
    );
    expect(preview.traceWasTrimmed, isTrue);
  });

  test('refused reset restores the exact previously visible preview', () {
    var plan = AuthoringRevision3QuestTransitionPlanV1.legacySeed(2);
    const root = AuthoringRevision3QuestTransitionNodeV1.root();
    final first = AuthoringRevision3QuestTransitionNodeV1.objective(1);
    final second = AuthoringRevision3QuestTransitionNodeV1.objective(2);
    plan = Revision3QuestTransitionsAuthoringService.setTransition(
      plan,
      AuthoringRevision3QuestTransitionV1(
        node: root,
        edge: AuthoringRevision3QuestTransitionEdgeV1.availability,
        externalAllowed: false,
        predicate: Revision3QuestTransitionsAuthoringService.predicate([
          [
            AuthoringRevision3QuestTransitionConditionAtomV1(
              node: first,
              test: AuthoringRevision3QuestTransitionStateTestV1.started,
              negated: true,
            ),
          ],
        ]),
      ),
    );
    plan = Revision3QuestTransitionsAuthoringService.setTransition(
      plan,
      AuthoringRevision3QuestTransitionV1(
        node: second,
        edge: AuthoringRevision3QuestTransitionEdgeV1.availability,
        externalAllowed: false,
        predicate: Revision3QuestTransitionsAuthoringService.predicate([
          [
            const AuthoringRevision3QuestTransitionConditionAtomV1(
              node: root,
              test: AuthoringRevision3QuestTransitionStateTestV1.available,
              negated: false,
            ),
          ],
        ]),
      ),
    );
    final preview = Revision3QuestLogicPreview(
      plan,
      maxCascadeOperations: 1,
      maxTraceEntries: 2,
    );

    expect(preview.trace, isEmpty);
    expect(
      preview
          .triggerExternal(root, AuthoringRevision3QuestTransitionEdgeV1.start)
          .status,
      Revision3QuestLogicPreviewActionStatus.applied,
    );
    preview.triggerExternal(
      root,
      AuthoringRevision3QuestTransitionEdgeV1.start,
    );
    preview.triggerExternal(
      root,
      AuthoringRevision3QuestTransitionEdgeV1.start,
    );
    expect(preview.traceWasTrimmed, isTrue);
    final phasesBefore =
        <
          AuthoringRevision3QuestTransitionNodeV1,
          Revision3QuestLogicPreviewPhase
        >{
          for (final node in [root, first, second])
            node: preview.stateOf(node).phase,
        };
    final traceBefore = preview.trace.map(_traceSignature).toList();
    final trimmedBefore = preview.traceWasTrimmed;

    final result = preview.reset();

    expect(result.status, Revision3QuestLogicPreviewActionStatus.refused);
    expect(<
      AuthoringRevision3QuestTransitionNodeV1,
      Revision3QuestLogicPreviewPhase
    >{
      for (final node in [root, first, second])
        node: preview.stateOf(node).phase,
    }, phasesBefore);
    expect(preview.trace.map(_traceSignature), orderedEquals(traceBefore));
    expect(preview.traceWasTrimmed, trimmedBefore);
  });
}

String _traceSignature(Revision3QuestLogicPreviewTraceEntry entry) =>
    '${entry.sequence}|${entry.kind.name}|${entry.node.stableKey}|'
    '${entry.edge?.wireName}|${entry.source?.stableKey}|${entry.detail}';
