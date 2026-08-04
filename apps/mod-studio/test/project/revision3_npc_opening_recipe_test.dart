import 'dart:async';
import 'dart:convert';
import 'dart:io';

import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/core/mod_ffi.dart';
import 'package:gore_mod/project/current_project_controller.dart';
import 'package:gore_mod/project/revision3_dialog_line_authoring.dart';
import 'package:gore_mod/project/revision3_npc_authoring.dart';
import 'package:gore_mod/project/revision3_npc_greeting_authoring.dart';
import 'package:gore_mod/project/revision3_npc_opening_recipe.dart';

const _projectId = '11111111111111111111111111111111';
const _npcId = '22222222222222222222222222222222';
const _moduleId = '33333333333333333333333333333333';
const _lineId = '44444444444444444444444444444444';
const _localizationId = '55555555555555555555555555555555';
const _slotId = '66666666666666666666666666666666';

void main() {
  test(
    'publishes N+1 and N+2 and hands the exact NPC head to Greeting authoring',
    () async {
      final recipe = Revision3NpcOpeningRecipe();
      final opening = _checkpoint(revision: 7, head: 70);
      final npcCheckpoint = _checkpoint(revision: 8, head: 80);
      final finalCheckpoint = _checkpoint(revision: 9, head: 90);
      var current = opening;
      var reads = 0;
      var npcCalls = 0;
      var greetingCalls = 0;

      final outcome = await recipe.run(
        openingCheckpoint: opening,
        readCurrentCheckpoint: () async {
          reads++;
          return current;
        },
        createNpc: ({required expectedCheckpoint}) async {
          npcCalls++;
          expect(expectedCheckpoint, same(opening));
          current = npcCheckpoint;
          return Revision3NpcOpeningRecipeNpcStep(
            publication: _npcPublication(revision: 8, head: npcCheckpoint.head),
            checkpoint: npcCheckpoint,
          );
        },
        createGreeting: ({required handoff}) async {
          greetingCalls++;
          expect(handoff.openingCheckpoint, same(opening));
          expect(handoff.npcCheckpoint, same(npcCheckpoint));
          expect(
            handoff.npcPublication.head.canonicalJson,
            npcCheckpoint.head.canonicalJson,
          );
          expect(handoff.npcPublication.npcId, _npcId);
          expect(handoff.npcPublication.scriptModuleId, _moduleId);
          current = finalCheckpoint;
          return Revision3NpcOpeningRecipeGreetingStep(
            publication: _greetingPublication(revision: 9),
            checkpoint: finalCheckpoint,
          );
        },
      );

      expect(outcome, isA<Revision3NpcOpeningRecipeCompletedOutcome>());
      final completed = outcome as Revision3NpcOpeningRecipeCompletedOutcome;
      expect(completed.npcStep.checkpoint, same(npcCheckpoint));
      expect(completed.greetingStep.checkpoint, same(finalCheckpoint));
      expect(completed.greetingStep.publication.createdLineId, _lineId);
      expect(npcCalls, 1);
      expect(greetingCalls, 1);
      expect(reads, 3);
      expect(recipe.isRunning, isFalse);
    },
  );

  test(
    'NPC cancellation reports no change at the opening checkpoint',
    () async {
      final opening = _checkpoint(revision: 10, head: 100);
      var current = opening;
      var greetingCalls = 0;

      final outcome = await Revision3NpcOpeningRecipe().run(
        openingCheckpoint: opening,
        readCurrentCheckpoint: () async => current,
        createNpc: ({required expectedCheckpoint}) async => null,
        createGreeting: ({required handoff}) async {
          greetingCalls++;
          return null;
        },
      );

      expect(outcome, isA<Revision3NpcOpeningRecipeNoChangeOutcome>());
      expect(
        (outcome as Revision3NpcOpeningRecipeNoChangeOutcome).reason,
        Revision3NpcOpeningRecipeNoChangeReason.cancelled,
      );
      expect(greetingCalls, 0);
    },
  );

  test(
    'safe NPC failure reports failed without claiming publication',
    () async {
      final opening = _checkpoint(revision: 11, head: 110);

      final outcome = await Revision3NpcOpeningRecipe().run(
        openingCheckpoint: opening,
        readCurrentCheckpoint: () async => opening,
        createNpc: ({required expectedCheckpoint}) async {
          throw const FormatException('known prepublication failure');
        },
        createGreeting: ({required handoff}) async => null,
      );

      expect(outcome, isA<Revision3NpcOpeningRecipeNoChangeOutcome>());
      expect(
        (outcome as Revision3NpcOpeningRecipeNoChangeOutcome).reason,
        Revision3NpcOpeningRecipeNoChangeReason.failed,
      );
    },
  );

  test('stale NPC authority locks fail-closed', () async {
    final opening = _checkpoint(revision: 12, head: 120);

    final outcome = await Revision3NpcOpeningRecipe().run(
      openingCheckpoint: opening,
      readCurrentCheckpoint: () async => opening,
      createNpc: ({required expectedCheckpoint}) async {
        throw const Revision3NpcDraftStaleCheckpointException();
      },
      createGreeting: ({required handoff}) async => null,
    );

    expect(outcome, isA<Revision3NpcOpeningRecipeLockedOutcome>());
    expect(
      (outcome as Revision3NpcOpeningRecipeLockedOutcome).reason,
      Revision3NpcOpeningRecipeLockReason.npcStepStale,
    );
  });

  test('uncertain NPC publication requires reopening', () async {
    final opening = _checkpoint(revision: 13, head: 130);

    final outcome = await Revision3NpcOpeningRecipe().run(
      openingCheckpoint: opening,
      readCurrentCheckpoint: () async => opening,
      createNpc: ({required expectedCheckpoint}) async {
        throw const Revision3NpcDraftRequiresReopenException();
      },
      createGreeting: ({required handoff}) async => null,
    );

    expect(outcome, isA<Revision3NpcOpeningRecipeRequiresReopenOutcome>());
    expect(
      (outcome as Revision3NpcOpeningRecipeRequiresReopenOutcome).reason,
      Revision3NpcOpeningRecipeRequiresReopenReason.npcStep,
    );
  });

  test('NPC receipt head must equal its returned checkpoint head', () async {
    final opening = _checkpoint(revision: 20, head: 200);
    final npcCheckpoint = _checkpoint(revision: 21, head: 210);
    var current = opening;
    var greetingCalls = 0;

    final outcome = await Revision3NpcOpeningRecipe().run(
      openingCheckpoint: opening,
      readCurrentCheckpoint: () async => current,
      createNpc: ({required expectedCheckpoint}) async {
        current = npcCheckpoint;
        return Revision3NpcOpeningRecipeNpcStep(
          publication: _npcPublication(revision: 21, head: _head(211)),
          checkpoint: npcCheckpoint,
        );
      },
      createGreeting: ({required handoff}) async {
        greetingCalls++;
        return null;
      },
    );

    expect(greetingCalls, 0);
    expect(outcome, isA<Revision3NpcOpeningRecipeLockedOutcome>());
    expect(
      (outcome as Revision3NpcOpeningRecipeLockedOutcome).reason,
      Revision3NpcOpeningRecipeLockReason.npcPublicationMismatch,
    );
  });

  test('NPC receipt cannot reuse the opening WorkingHead', () async {
    final opening = _checkpoint(revision: 22, head: 220);
    final npcCheckpoint = _checkpoint(revision: 23, head: 220);
    var current = opening;

    final outcome = await Revision3NpcOpeningRecipe().run(
      openingCheckpoint: opening,
      readCurrentCheckpoint: () async => current,
      createNpc: ({required expectedCheckpoint}) async {
        current = npcCheckpoint;
        return Revision3NpcOpeningRecipeNpcStep(
          publication: _npcPublication(revision: 23, head: npcCheckpoint.head),
          checkpoint: npcCheckpoint,
        );
      },
      createGreeting: ({required handoff}) async => null,
    );

    expect(outcome, isA<Revision3NpcOpeningRecipeLockedOutcome>());
    expect(
      (outcome as Revision3NpcOpeningRecipeLockedOutcome).reason,
      Revision3NpcOpeningRecipeLockReason.npcPublicationMismatch,
    );
  });

  test('NPC checkpoint must be exactly N+1', () async {
    final opening = _checkpoint(revision: 24, head: 240);
    final skippedCheckpoint = _checkpoint(revision: 26, head: 260);
    var current = opening;

    final outcome = await Revision3NpcOpeningRecipe().run(
      openingCheckpoint: opening,
      readCurrentCheckpoint: () async => current,
      createNpc: ({required expectedCheckpoint}) async {
        current = skippedCheckpoint;
        return Revision3NpcOpeningRecipeNpcStep(
          publication: _npcPublication(
            revision: 25,
            head: skippedCheckpoint.head,
          ),
          checkpoint: skippedCheckpoint,
        );
      },
      createGreeting: ({required handoff}) async => null,
    );

    expect(outcome, isA<Revision3NpcOpeningRecipeLockedOutcome>());
    expect(
      (outcome as Revision3NpcOpeningRecipeLockedOutcome).reason,
      Revision3NpcOpeningRecipeLockReason.npcCheckpointMismatch,
    );
  });

  test('same-revision NPC head drift blocks the Greeting handoff', () async {
    final opening = _checkpoint(revision: 30, head: 300);
    final publishedNpc = _checkpoint(revision: 31, head: 310);
    final divergentNpc = _checkpoint(revision: 31, head: 311);
    var current = opening;
    var greetingCalls = 0;

    final outcome = await Revision3NpcOpeningRecipe().run(
      openingCheckpoint: opening,
      readCurrentCheckpoint: () async => current,
      createNpc: ({required expectedCheckpoint}) async {
        current = divergentNpc;
        return Revision3NpcOpeningRecipeNpcStep(
          publication: _npcPublication(revision: 31, head: publishedNpc.head),
          checkpoint: publishedNpc,
        );
      },
      createGreeting: ({required handoff}) async {
        greetingCalls++;
        return null;
      },
    );

    expect(greetingCalls, 0);
    expect(outcome, isA<Revision3NpcOpeningRecipeLockedOutcome>());
    expect(
      (outcome as Revision3NpcOpeningRecipeLockedOutcome).reason,
      Revision3NpcOpeningRecipeLockReason.npcCheckpointDrift,
    );
  });

  test('Greeting cancellation keeps the exact NPC resumable', () async {
    final opening = _checkpoint(revision: 40, head: 400);
    final npcCheckpoint = _checkpoint(revision: 41, head: 410);
    var current = opening;

    final outcome = await Revision3NpcOpeningRecipe().run(
      openingCheckpoint: opening,
      readCurrentCheckpoint: () async => current,
      createNpc: ({required expectedCheckpoint}) async {
        current = npcCheckpoint;
        return _npcStep(npcCheckpoint);
      },
      createGreeting: ({required handoff}) async => null,
    );

    expect(outcome, isA<Revision3NpcOpeningRecipeNpcOnlyOutcome>());
    final partial = outcome as Revision3NpcOpeningRecipeNpcOnlyOutcome;
    expect(
      partial.reason,
      Revision3NpcOpeningRecipeNpcOnlyReason.greetingCancelled,
    );
    expect(partial.npcStep.checkpoint, same(npcCheckpoint));
  });

  test('safe Greeting failure keeps the exact NPC resumable', () async {
    final opening = _checkpoint(revision: 42, head: 420);
    final npcCheckpoint = _checkpoint(revision: 43, head: 430);
    var current = opening;

    final outcome = await Revision3NpcOpeningRecipe().run(
      openingCheckpoint: opening,
      readCurrentCheckpoint: () async => current,
      createNpc: ({required expectedCheckpoint}) async {
        current = npcCheckpoint;
        return _npcStep(npcCheckpoint);
      },
      createGreeting: ({required handoff}) async {
        throw const FormatException('known prepublication failure');
      },
    );

    expect(outcome, isA<Revision3NpcOpeningRecipeNpcOnlyOutcome>());
    expect(
      (outcome as Revision3NpcOpeningRecipeNpcOnlyOutcome).reason,
      Revision3NpcOpeningRecipeNpcOnlyReason.greetingFailed,
    );
  });

  test(
    'stale DialogLine authority in Greeting authoring locks fail-closed',
    () async {
      final opening = _checkpoint(revision: 44, head: 440);
      final npcCheckpoint = _checkpoint(revision: 45, head: 450);
      var current = opening;

      final outcome = await Revision3NpcOpeningRecipe().run(
        openingCheckpoint: opening,
        readCurrentCheckpoint: () async => current,
        createNpc: ({required expectedCheckpoint}) async {
          current = npcCheckpoint;
          return _npcStep(npcCheckpoint);
        },
        createGreeting: ({required handoff}) async {
          throw const Revision3DialogLineEntryStaleCheckpointException();
        },
      );

      expect(outcome, isA<Revision3NpcOpeningRecipeLockedOutcome>());
      expect(
        (outcome as Revision3NpcOpeningRecipeLockedOutcome).reason,
        Revision3NpcOpeningRecipeLockReason.greetingStepStale,
      );
    },
  );

  test('uncertain Greeting publication requires reopening', () async {
    final opening = _checkpoint(revision: 46, head: 460);
    final npcCheckpoint = _checkpoint(revision: 47, head: 470);
    var current = opening;

    final outcome = await Revision3NpcOpeningRecipe().run(
      openingCheckpoint: opening,
      readCurrentCheckpoint: () async => current,
      createNpc: ({required expectedCheckpoint}) async {
        current = npcCheckpoint;
        return _npcStep(npcCheckpoint);
      },
      createGreeting: ({required handoff}) async {
        throw const Revision3NpcGreetingRequiresReopenException();
      },
    );

    expect(outcome, isA<Revision3NpcOpeningRecipeRequiresReopenOutcome>());
    expect(
      (outcome as Revision3NpcOpeningRecipeRequiresReopenOutcome).reason,
      Revision3NpcOpeningRecipeRequiresReopenReason.greetingStep,
    );
  });

  final invalidGreetingReceipts =
      <({String label, Revision3NpcGreetingPublication publication})>[
        (
          label: 'foreign NPC',
          publication: _greetingPublication(
            revision: 52,
            npcId: '77777777777777777777777777777777',
          ),
        ),
        (
          label: 'foreign module',
          publication: _greetingPublication(
            revision: 52,
            moduleId: '88888888888888888888888888888888',
          ),
        ),
        (
          label: 'replace mode',
          publication: _greetingPublication(
            revision: 52,
            mode: AuthoringRevision3NpcGreetingMode.replace,
          ),
        ),
        (
          label: 'non-first count',
          publication: _greetingPublication(revision: 52, greetingCount: 2),
        ),
        (
          label: 'missing line',
          publication: _greetingPublication(revision: 52, createdLineId: null),
        ),
        (
          label: 'missing localization',
          publication: _greetingPublication(
            revision: 52,
            createdLocalizationId: null,
          ),
        ),
        (
          label: 'missing localization action',
          publication: _greetingPublication(
            revision: 52,
            localizationAction: null,
          ),
        ),
      ];
  for (final invalid in invalidGreetingReceipts) {
    test('Greeting receipt rejects ${invalid.label}', () async {
      final opening = _checkpoint(revision: 50, head: 500);
      final npcCheckpoint = _checkpoint(revision: 51, head: 510);
      final finalCheckpoint = _checkpoint(revision: 52, head: 520);
      var current = opening;

      final outcome = await Revision3NpcOpeningRecipe().run(
        openingCheckpoint: opening,
        readCurrentCheckpoint: () async => current,
        createNpc: ({required expectedCheckpoint}) async {
          current = npcCheckpoint;
          return _npcStep(npcCheckpoint);
        },
        createGreeting: ({required handoff}) async {
          current = finalCheckpoint;
          return Revision3NpcOpeningRecipeGreetingStep(
            publication: invalid.publication,
            checkpoint: finalCheckpoint,
          );
        },
      );

      expect(outcome, isA<Revision3NpcOpeningRecipeLockedOutcome>());
      expect(
        (outcome as Revision3NpcOpeningRecipeLockedOutcome).reason,
        Revision3NpcOpeningRecipeLockReason.greetingPublicationMismatch,
      );
    });
  }

  test(
    'requires-reopen final checkpoint takes priority over receipt validation',
    () async {
      final opening = _checkpoint(revision: 60, head: 600);
      final npcCheckpoint = _checkpoint(revision: 61, head: 610);
      final poisonedFinal = _checkpoint(
        revision: 62,
        head: 620,
        requiresReopen: true,
      );
      var current = opening;

      final outcome = await Revision3NpcOpeningRecipe().run(
        openingCheckpoint: opening,
        readCurrentCheckpoint: () async => current,
        createNpc: ({required expectedCheckpoint}) async {
          current = npcCheckpoint;
          return _npcStep(npcCheckpoint);
        },
        createGreeting: ({required handoff}) async {
          current = poisonedFinal;
          return Revision3NpcOpeningRecipeGreetingStep(
            publication: _greetingPublication(
              revision: 62,
              npcId: '77777777777777777777777777777777',
            ),
            checkpoint: poisonedFinal,
          );
        },
      );

      expect(outcome, isA<Revision3NpcOpeningRecipeRequiresReopenOutcome>());
      expect(
        (outcome as Revision3NpcOpeningRecipeRequiresReopenOutcome).reason,
        Revision3NpcOpeningRecipeRequiresReopenReason.finalCheckpoint,
      );
    },
  );

  test('final checkpoint must be exactly N+2 with a new head', () async {
    final opening = _checkpoint(revision: 63, head: 630);
    final npcCheckpoint = _checkpoint(revision: 64, head: 640);
    final reusedHead = _checkpoint(revision: 65, head: 640);
    var current = opening;

    final outcome = await Revision3NpcOpeningRecipe().run(
      openingCheckpoint: opening,
      readCurrentCheckpoint: () async => current,
      createNpc: ({required expectedCheckpoint}) async {
        current = npcCheckpoint;
        return _npcStep(npcCheckpoint);
      },
      createGreeting: ({required handoff}) async {
        current = reusedHead;
        return Revision3NpcOpeningRecipeGreetingStep(
          publication: _greetingPublication(revision: 65),
          checkpoint: reusedHead,
        );
      },
    );

    expect(outcome, isA<Revision3NpcOpeningRecipeLockedOutcome>());
    expect(
      (outcome as Revision3NpcOpeningRecipeLockedOutcome).reason,
      Revision3NpcOpeningRecipeLockReason.finalCheckpointMismatch,
    );
  });

  test('same-revision final-head drift never becomes completed', () async {
    final opening = _checkpoint(revision: 66, head: 660);
    final npcCheckpoint = _checkpoint(revision: 67, head: 670);
    final publishedFinal = _checkpoint(revision: 68, head: 680);
    final divergentFinal = _checkpoint(revision: 68, head: 681);
    var current = opening;

    final outcome = await Revision3NpcOpeningRecipe().run(
      openingCheckpoint: opening,
      readCurrentCheckpoint: () async => current,
      createNpc: ({required expectedCheckpoint}) async {
        current = npcCheckpoint;
        return _npcStep(npcCheckpoint);
      },
      createGreeting: ({required handoff}) async {
        current = divergentFinal;
        return Revision3NpcOpeningRecipeGreetingStep(
          publication: _greetingPublication(revision: 68),
          checkpoint: publishedFinal,
        );
      },
    );

    expect(outcome, isA<Revision3NpcOpeningRecipeLockedOutcome>());
    expect(
      (outcome as Revision3NpcOpeningRecipeLockedOutcome).reason,
      Revision3NpcOpeningRecipeLockReason.finalCheckpointDrift,
    );
  });

  test('initial head drift prevents both mutation steps', () async {
    final opening = _checkpoint(revision: 70, head: 700);
    final drifted = _checkpoint(revision: 70, head: 701);
    var npcCalls = 0;
    var greetingCalls = 0;

    final outcome = await Revision3NpcOpeningRecipe().run(
      openingCheckpoint: opening,
      readCurrentCheckpoint: () async => drifted,
      createNpc: ({required expectedCheckpoint}) async {
        npcCalls++;
        return null;
      },
      createGreeting: ({required handoff}) async {
        greetingCalls++;
        return null;
      },
    );

    expect(npcCalls, 0);
    expect(greetingCalls, 0);
    expect(outcome, isA<Revision3NpcOpeningRecipeLockedOutcome>());
    expect(
      (outcome as Revision3NpcOpeningRecipeLockedOutcome).reason,
      Revision3NpcOpeningRecipeLockReason.openingCheckpointDrift,
    );
  });

  test('checkpoint reader failure locks before mutation', () async {
    final opening = _checkpoint(revision: 72, head: 720);
    var npcCalls = 0;

    final outcome = await Revision3NpcOpeningRecipe().run(
      openingCheckpoint: opening,
      readCurrentCheckpoint: () async => throw StateError('unavailable'),
      createNpc: ({required expectedCheckpoint}) async {
        npcCalls++;
        return null;
      },
      createGreeting: ({required handoff}) async => null,
    );

    expect(npcCalls, 0);
    expect(outcome, isA<Revision3NpcOpeningRecipeLockedOutcome>());
    expect(
      (outcome as Revision3NpcOpeningRecipeLockedOutcome).reason,
      Revision3NpcOpeningRecipeLockReason.checkpointUnavailable,
    );
  });

  test('duplicate activation shares one in-flight attempt', () async {
    final recipe = Revision3NpcOpeningRecipe();
    final opening = _checkpoint(revision: 80, head: 800);
    final npcCheckpoint = _checkpoint(revision: 81, head: 810);
    final gate = Completer<void>();
    var current = opening;
    var npcCalls = 0;
    var greetingCalls = 0;

    Future<Revision3NpcOpeningRecipeNpcStep?> createNpc({
      required ManagedRevision3CurrentProjectState expectedCheckpoint,
    }) async {
      npcCalls++;
      await gate.future;
      current = npcCheckpoint;
      return _npcStep(npcCheckpoint);
    }

    Future<Revision3NpcOpeningRecipeGreetingStep?> createGreeting({
      required Revision3NpcOpeningRecipeHandoff handoff,
    }) async {
      greetingCalls++;
      return null;
    }

    final first = recipe.run(
      openingCheckpoint: opening,
      readCurrentCheckpoint: () async => current,
      createNpc: createNpc,
      createGreeting: createGreeting,
    );
    final second = recipe.run(
      openingCheckpoint: opening,
      readCurrentCheckpoint: () async => current,
      createNpc: createNpc,
      createGreeting: createGreeting,
    );

    expect(identical(first, second), isTrue);
    expect(recipe.isRunning, isTrue);
    gate.complete();
    final outcomes = await Future.wait([first, second]);

    expect(outcomes[0], same(outcomes[1]));
    expect(outcomes.first, isA<Revision3NpcOpeningRecipeNpcOnlyOutcome>());
    expect(npcCalls, 1);
    expect(greetingCalls, 1);
    expect(recipe.isRunning, isFalse);
  });
}

ManagedRevision3CurrentProjectState _checkpoint({
  required int revision,
  required int head,
  bool requiresReopen = false,
  String root = r'C:\mods\npc-opening-recipe',
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

Revision3NpcOpeningRecipeNpcStep _npcStep(
  ManagedRevision3CurrentProjectState checkpoint,
) => Revision3NpcOpeningRecipeNpcStep(
  publication: _npcPublication(
    revision: checkpoint.projectRevision,
    head: checkpoint.head,
  ),
  checkpoint: checkpoint,
);

Revision3NpcDraftPublication _npcPublication({
  required int revision,
  required AuthoringWorkingHead head,
}) => Revision3NpcDraftPublication(
  projectId: _projectId,
  projectRevision: revision,
  head: head,
  npcId: _npcId,
  scriptModuleId: _moduleId,
);

Revision3NpcGreetingPublication _greetingPublication({
  required int revision,
  String npcId = _npcId,
  String moduleId = _moduleId,
  AuthoringRevision3NpcGreetingMode mode =
      AuthoringRevision3NpcGreetingMode.createAndInsert,
  int greetingCount = 1,
  String? createdLineId = _lineId,
  String? createdLocalizationId = _localizationId,
  AuthoringRevision3DialogLocalizationAction? localizationAction =
      AuthoringRevision3DialogLocalizationAction.created,
}) => Revision3NpcGreetingPublication(
  projectId: _projectId,
  projectRevision: revision,
  npcId: npcId,
  npcRevision: 2,
  moduleId: moduleId,
  moduleRevision: 0,
  mode: mode,
  greetingCount: greetingCount,
  createdLineId: createdLineId,
  createdLocalizationId: createdLocalizationId,
  createdVoiceSlotId: _slotId,
  localizationAction: localizationAction,
);
