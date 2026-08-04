import 'dart:async';
import 'dart:convert';
import 'dart:io';

import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/core/mod_ffi.dart';
import 'package:gore_mod/project/current_project_controller.dart';
import 'package:gore_mod/project/revision3_dialog_line_authoring.dart';
import 'package:gore_mod/project/revision3_quest_authoring.dart';
import 'package:gore_mod/project/revision3_quest_opening_recipe.dart';
import 'package:gore_mod/project/revision3_quest_transcript_authoring.dart';

const _projectId = '11111111111111111111111111111111';
const _questId = '22222222222222222222222222222222';
const _moduleId = '33333333333333333333333333333333';
const _lineId = '44444444444444444444444444444444';
const _localizationId = '55555555555555555555555555555555';
const _slotId = '66666666666666666666666666666666';

void main() {
  test(
    'publishes two exact checkpoints and hands the Quest head to step two',
    () async {
      final recipe = Revision3QuestOpeningRecipe();
      final opening = _checkpoint(revision: 7, head: 7);
      final questCheckpoint = _checkpoint(revision: 8, head: 8);
      final finalCheckpoint = _checkpoint(revision: 9, head: 9);
      var current = opening;
      var questCalls = 0;
      var lineCalls = 0;

      final outcome = await recipe.run(
        openingCheckpoint: opening,
        readCurrentCheckpoint: () async => current,
        createQuest: ({required expectedCheckpoint}) async {
          questCalls++;
          expect(expectedCheckpoint, same(opening));
          current = questCheckpoint;
          return Revision3QuestOpeningRecipeQuestStep(
            publication: _questPublication(revision: 8),
            checkpoint: questCheckpoint,
          );
        },
        createOpeningLine: ({required handoff}) async {
          lineCalls++;
          expect(handoff.openingCheckpoint, same(opening));
          expect(handoff.questCheckpoint, same(questCheckpoint));
          expect(
            handoff.questCheckpoint.head.canonicalJson,
            _head(8).canonicalJson,
          );
          expect(handoff.questPublication.questId, _questId);
          current = finalCheckpoint;
          return Revision3QuestOpeningRecipeLineStep(
            publication: _linePublication(revision: 9),
            checkpoint: finalCheckpoint,
          );
        },
      );

      expect(outcome, isA<Revision3QuestOpeningRecipeCompletedOutcome>());
      final completed = outcome as Revision3QuestOpeningRecipeCompletedOutcome;
      expect(completed.questStep.checkpoint, same(questCheckpoint));
      expect(completed.lineStep.checkpoint, same(finalCheckpoint));
      expect(completed.lineStep.publication.createdLineId, _lineId);
      expect(questCalls, 1);
      expect(lineCalls, 1);
      expect(recipe.isRunning, isFalse);
    },
  );

  test(
    'step-two cancellation keeps the exact Quest as a resumable outcome',
    () async {
      final opening = _checkpoint(revision: 3, head: 3);
      final questCheckpoint = _checkpoint(revision: 4, head: 4);
      var current = opening;

      final outcome = await Revision3QuestOpeningRecipe().run(
        openingCheckpoint: opening,
        readCurrentCheckpoint: () async => current,
        createQuest: ({required expectedCheckpoint}) async {
          current = questCheckpoint;
          return Revision3QuestOpeningRecipeQuestStep(
            publication: _questPublication(revision: 4),
            checkpoint: questCheckpoint,
          );
        },
        createOpeningLine: ({required handoff}) async => null,
      );

      expect(outcome, isA<Revision3QuestOpeningRecipeQuestOnlyOutcome>());
      final partial = outcome as Revision3QuestOpeningRecipeQuestOnlyOutcome;
      expect(
        partial.reason,
        Revision3QuestOpeningRecipeQuestOnlyReason.openingLineCancelled,
      );
      expect(partial.questStep.checkpoint, same(questCheckpoint));
    },
  );

  test(
    'safe step-two failure keeps the exact Quest without retrying',
    () async {
      final opening = _checkpoint(revision: 10, head: 10);
      final questCheckpoint = _checkpoint(revision: 11, head: 11);
      var current = opening;
      var lineCalls = 0;

      final outcome = await Revision3QuestOpeningRecipe().run(
        openingCheckpoint: opening,
        readCurrentCheckpoint: () async => current,
        createQuest: ({required expectedCheckpoint}) async {
          current = questCheckpoint;
          return Revision3QuestOpeningRecipeQuestStep(
            publication: _questPublication(revision: 11),
            checkpoint: questCheckpoint,
          );
        },
        createOpeningLine: ({required handoff}) async {
          lineCalls++;
          throw const FormatException('known prepublication failure');
        },
      );

      expect(lineCalls, 1);
      expect(outcome, isA<Revision3QuestOpeningRecipeQuestOnlyOutcome>());
      expect(
        (outcome as Revision3QuestOpeningRecipeQuestOnlyOutcome).reason,
        Revision3QuestOpeningRecipeQuestOnlyReason.openingLineFailed,
      );
    },
  );

  test(
    'uncertain step-two publication requires reopen even before state refresh',
    () async {
      final opening = _checkpoint(revision: 20, head: 20);
      final questCheckpoint = _checkpoint(revision: 21, head: 21);
      var current = opening;

      final outcome = await Revision3QuestOpeningRecipe().run(
        openingCheckpoint: opening,
        readCurrentCheckpoint: () async => current,
        createQuest: ({required expectedCheckpoint}) async {
          current = questCheckpoint;
          return Revision3QuestOpeningRecipeQuestStep(
            publication: _questPublication(revision: 21),
            checkpoint: questCheckpoint,
          );
        },
        createOpeningLine: ({required handoff}) async {
          throw const Revision3QuestTranscriptRequiresReopenException();
        },
      );

      expect(outcome, isA<Revision3QuestOpeningRecipeRequiresReopenOutcome>());
      expect(
        (outcome as Revision3QuestOpeningRecipeRequiresReopenOutcome).reason,
        Revision3QuestOpeningRecipeRequiresReopenReason.openingLineStep,
      );
    },
  );

  test(
    'stale step-two authority locks instead of claiming Quest-only failure',
    () async {
      final opening = _checkpoint(revision: 30, head: 30);
      final questCheckpoint = _checkpoint(revision: 31, head: 31);
      var current = opening;

      final outcome = await Revision3QuestOpeningRecipe().run(
        openingCheckpoint: opening,
        readCurrentCheckpoint: () async => current,
        createQuest: ({required expectedCheckpoint}) async {
          current = questCheckpoint;
          return Revision3QuestOpeningRecipeQuestStep(
            publication: _questPublication(revision: 31),
            checkpoint: questCheckpoint,
          );
        },
        createOpeningLine: ({required handoff}) async {
          throw const Revision3DialogLineEntryStaleCheckpointException();
        },
      );

      expect(outcome, isA<Revision3QuestOpeningRecipeLockedOutcome>());
      expect(
        (outcome as Revision3QuestOpeningRecipeLockedOutcome).reason,
        Revision3QuestOpeningRecipeLockReason.openingLineStepStale,
      );
    },
  );

  test(
    'same-revision Quest-head drift blocks the opening-line handoff',
    () async {
      final opening = _checkpoint(revision: 40, head: 40);
      final publishedQuest = _checkpoint(revision: 41, head: 41);
      final divergentQuest = _checkpoint(revision: 41, head: 141);
      var current = opening;
      var lineCalls = 0;

      final outcome = await Revision3QuestOpeningRecipe().run(
        openingCheckpoint: opening,
        readCurrentCheckpoint: () async => current,
        createQuest: ({required expectedCheckpoint}) async {
          current = divergentQuest;
          return Revision3QuestOpeningRecipeQuestStep(
            publication: _questPublication(revision: 41),
            checkpoint: publishedQuest,
          );
        },
        createOpeningLine: ({required handoff}) async {
          lineCalls++;
          return null;
        },
      );

      expect(lineCalls, 0);
      expect(outcome, isA<Revision3QuestOpeningRecipeLockedOutcome>());
      expect(
        (outcome as Revision3QuestOpeningRecipeLockedOutcome).reason,
        Revision3QuestOpeningRecipeLockReason.questCheckpointDrift,
      );
    },
  );

  test('mismatched opening-line receipt never becomes completed', () async {
    final opening = _checkpoint(revision: 50, head: 50);
    final questCheckpoint = _checkpoint(revision: 51, head: 51);
    final finalCheckpoint = _checkpoint(revision: 52, head: 52);
    var current = opening;

    final outcome = await Revision3QuestOpeningRecipe().run(
      openingCheckpoint: opening,
      readCurrentCheckpoint: () async => current,
      createQuest: ({required expectedCheckpoint}) async {
        current = questCheckpoint;
        return Revision3QuestOpeningRecipeQuestStep(
          publication: _questPublication(revision: 51),
          checkpoint: questCheckpoint,
        );
      },
      createOpeningLine: ({required handoff}) async {
        current = finalCheckpoint;
        return Revision3QuestOpeningRecipeLineStep(
          publication: _linePublication(
            revision: 52,
            questId: '77777777777777777777777777777777',
          ),
          checkpoint: finalCheckpoint,
        );
      },
    );

    expect(outcome, isA<Revision3QuestOpeningRecipeLockedOutcome>());
    expect(
      (outcome as Revision3QuestOpeningRecipeLockedOutcome).reason,
      Revision3QuestOpeningRecipeLockReason.openingLinePublicationMismatch,
    );
  });

  test(
    'requires-reopen final checkpoint takes priority over receipt validation',
    () async {
      final opening = _checkpoint(revision: 53, head: 53);
      final questCheckpoint = _checkpoint(revision: 54, head: 54);
      final poisonedFinal = _checkpoint(
        revision: 55,
        head: 55,
        requiresReopen: true,
      );
      var current = opening;

      final outcome = await Revision3QuestOpeningRecipe().run(
        openingCheckpoint: opening,
        readCurrentCheckpoint: () async => current,
        createQuest: ({required expectedCheckpoint}) async {
          current = questCheckpoint;
          return Revision3QuestOpeningRecipeQuestStep(
            publication: _questPublication(revision: 54),
            checkpoint: questCheckpoint,
          );
        },
        createOpeningLine: ({required handoff}) async {
          current = poisonedFinal;
          return Revision3QuestOpeningRecipeLineStep(
            publication: _linePublication(
              revision: 55,
              questId: '77777777777777777777777777777777',
            ),
            checkpoint: poisonedFinal,
          );
        },
      );

      expect(outcome, isA<Revision3QuestOpeningRecipeRequiresReopenOutcome>());
      expect(
        (outcome as Revision3QuestOpeningRecipeRequiresReopenOutcome).reason,
        Revision3QuestOpeningRecipeRequiresReopenReason.finalCheckpoint,
      );
    },
  );

  test('same-revision final-head drift never becomes completed', () async {
    final opening = _checkpoint(revision: 56, head: 56);
    final questCheckpoint = _checkpoint(revision: 57, head: 57);
    final publishedFinal = _checkpoint(revision: 58, head: 58);
    final divergentFinal = _checkpoint(revision: 58, head: 158);
    var current = opening;

    final outcome = await Revision3QuestOpeningRecipe().run(
      openingCheckpoint: opening,
      readCurrentCheckpoint: () async => current,
      createQuest: ({required expectedCheckpoint}) async {
        current = questCheckpoint;
        return Revision3QuestOpeningRecipeQuestStep(
          publication: _questPublication(revision: 57),
          checkpoint: questCheckpoint,
        );
      },
      createOpeningLine: ({required handoff}) async {
        current = divergentFinal;
        return Revision3QuestOpeningRecipeLineStep(
          publication: _linePublication(revision: 58),
          checkpoint: publishedFinal,
        );
      },
    );

    expect(outcome, isA<Revision3QuestOpeningRecipeLockedOutcome>());
    expect(
      (outcome as Revision3QuestOpeningRecipeLockedOutcome).reason,
      Revision3QuestOpeningRecipeLockReason.finalCheckpointDrift,
    );
  });

  test('initial head drift prevents both mutation steps', () async {
    final opening = _checkpoint(revision: 60, head: 60);
    final drifted = _checkpoint(revision: 60, head: 160);
    var questCalls = 0;
    var lineCalls = 0;

    final outcome = await Revision3QuestOpeningRecipe().run(
      openingCheckpoint: opening,
      readCurrentCheckpoint: () async => drifted,
      createQuest: ({required expectedCheckpoint}) async {
        questCalls++;
        return null;
      },
      createOpeningLine: ({required handoff}) async {
        lineCalls++;
        return null;
      },
    );

    expect(questCalls, 0);
    expect(lineCalls, 0);
    expect(outcome, isA<Revision3QuestOpeningRecipeLockedOutcome>());
    expect(
      (outcome as Revision3QuestOpeningRecipeLockedOutcome).reason,
      Revision3QuestOpeningRecipeLockReason.openingCheckpointDrift,
    );
  });

  test('duplicate activation shares one in-flight attempt', () async {
    final recipe = Revision3QuestOpeningRecipe();
    final opening = _checkpoint(revision: 70, head: 70);
    final questCheckpoint = _checkpoint(revision: 71, head: 71);
    final gate = Completer<void>();
    var current = opening;
    var questCalls = 0;
    var lineCalls = 0;

    Future<Revision3QuestOpeningRecipeQuestStep?> createQuest({
      required ManagedRevision3CurrentProjectState expectedCheckpoint,
    }) async {
      questCalls++;
      await gate.future;
      current = questCheckpoint;
      return Revision3QuestOpeningRecipeQuestStep(
        publication: _questPublication(revision: 71),
        checkpoint: questCheckpoint,
      );
    }

    Future<Revision3QuestOpeningRecipeLineStep?> createLine({
      required Revision3QuestOpeningRecipeHandoff handoff,
    }) async {
      lineCalls++;
      return null;
    }

    final first = recipe.run(
      openingCheckpoint: opening,
      readCurrentCheckpoint: () async => current,
      createQuest: createQuest,
      createOpeningLine: createLine,
    );
    final second = recipe.run(
      openingCheckpoint: opening,
      readCurrentCheckpoint: () async => current,
      createQuest: createQuest,
      createOpeningLine: createLine,
    );

    expect(identical(first, second), isTrue);
    expect(recipe.isRunning, isTrue);
    gate.complete();
    final outcomes = await Future.wait([first, second]);

    expect(outcomes[0], same(outcomes[1]));
    expect(outcomes.first, isA<Revision3QuestOpeningRecipeQuestOnlyOutcome>());
    expect(questCalls, 1);
    expect(lineCalls, 1);
    expect(recipe.isRunning, isFalse);
  });
}

ManagedRevision3CurrentProjectState _checkpoint({
  required int revision,
  required int head,
  bool requiresReopen = false,
  String root = r'C:\mods\quest-opening-recipe',
  String projectId = _projectId,
}) => ManagedRevision3CurrentProjectState(
  root: Directory(root),
  projectId: projectId,
  projectRevision: revision,
  head: _head(head),
  requiresReopen: requiresReopen,
);

AuthoringWorkingHead _head(int value) => AuthoringWorkingHead.fromCanonicalJson(
  jsonEncode(<String, Object?>{
    'store_format': 1,
    'snapshot': <String, Object?>{
      'byte_len': value + 1,
      'sha256': value.toRadixString(16).padLeft(64, '0'),
    },
  }),
);

Revision3QuestDraftPublication _questPublication({required int revision}) =>
    Revision3QuestDraftPublication(
      projectId: _projectId,
      projectRevision: revision,
      questId: _questId,
      scriptModuleId: _moduleId,
    );

Revision3QuestTranscriptPublication _linePublication({
  required int revision,
  String questId = _questId,
}) => Revision3QuestTranscriptPublication(
  projectId: _projectId,
  projectRevision: revision,
  questId: questId,
  questRevision: 1,
  moduleId: _moduleId,
  moduleRevision: 0,
  mode: AuthoringRevision3QuestTranscriptMode.createAndInsert,
  transcriptCount: 1,
  createdLineId: _lineId,
  createdLocalizationId: _localizationId,
  createdVoiceSlotId: _slotId,
  localizationAction: AuthoringRevision3DialogLocalizationAction.created,
);
