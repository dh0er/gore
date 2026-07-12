import 'dart:convert';
import 'dart:math';
import 'dart:typed_data';

import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/story/domain/story_draft_requests.dart';

const _projectId = '00000000000000000000000000000001';
const _draftId = '10000000000000000000000000000001';
const _moduleId = '20000000000000000000000000000001';

void main() {
  test('NPC factory emits only the closed transaction and input fields', () {
    final parentCharacter = _trusted(<String, Object?>{
      'selector': 'CharacterParent',
    });
    final parentAi = _trusted(<String, Object?>{'selector': 'AiParent'});
    final parentSpawn = _trusted(<String, Object?>{'selector': 'SpawnParent'});
    final json = buildNpcStoryDraftMutationJson(
      context: _context(9),
      input: StoryNpcDraftInput(
        displayName: 'Gate guard',
        moduleNamespace: 'GoreMods.Npcs.GateGuard',
        uniqueName: 'GoreGateGuard',
        parentCharacterDefinition: parentCharacter,
        parentAiAgentConfig: parentAi,
        parentSpawnDefinition: parentSpawn,
      ),
    );
    final decoded = (jsonDecode(json) as Map).cast<String, Object?>();

    expect(decoded.keys, <String>[
      'expected_project_id',
      'expected_revision',
      'draft_id',
      'script_module_id',
      'display_name',
      'draft',
    ]);
    expect(decoded['expected_project_id'], _projectId);
    expect(decoded['expected_revision'], 9);
    final draft = (decoded['draft'] as Map).cast<String, Object?>();
    expect(draft.keys, <String>['kind', 'input']);
    expect(draft['kind'], 'npc');
    final input = (draft['input'] as Map).cast<String, Object?>();
    expect(input.keys, <String>[
      'module_namespace',
      'unique_name',
      'parent_character_definition',
      'parent_ai_agent_config',
      'parent_spawn_definition',
    ]);
    expect(input['parent_character_definition'], <String, Object?>{
      'selector': 'CharacterParent',
    });
    expect(input['parent_ai_agent_config'], <String, Object?>{
      'selector': 'AiParent',
    });
    expect(input['parent_spawn_definition'], <String, Object?>{
      'selector': 'SpawnParent',
    });
  });

  test('trusted fragments must be canonical bounded JSON objects', () {
    expect(
      () => CanonicalUnverifiedStoryJsonObject.fromCanonicalJson('{ "x": 1 }'),
      throwsFormatException,
    );
    expect(
      () => CanonicalUnverifiedStoryJsonObject.fromCanonicalJson('[1,2,3]'),
      throwsFormatException,
    );
    expect(
      () =>
          CanonicalUnverifiedStoryJsonObject.fromCanonicalJson('{"x":1,"x":2}'),
      throwsFormatException,
    );
  });

  test('wire revision ceiling and non-zero distinct IDs fail closed', () {
    expect(
      () => StoryDraftMutationContext(
        projectId: _projectId,
        revision: 0x7ffffffffffffffe,
        ids: StoryDraftEntityIds(draftId: _draftId, scriptModuleId: _moduleId),
      ),
      returnsNormally,
    );
    expect(
      () => StoryDraftMutationContext(
        projectId: _projectId,
        revision: 0x7fffffffffffffff,
        ids: StoryDraftEntityIds(draftId: _draftId, scriptModuleId: _moduleId),
      ),
      throwsFormatException,
    );
    for (final ids in <({String draft, String module})>[
      (draft: '00000000000000000000000000000000', module: _moduleId),
      (draft: _draftId, module: '00000000000000000000000000000000'),
      (draft: _draftId, module: _draftId),
    ]) {
      expect(
        () =>
            StoryDraftEntityIds(draftId: ids.draft, scriptModuleId: ids.module),
        throwsFormatException,
      );
    }
  });

  test('component and aggregate bounds reject before mutation assembly', () {
    final small = _trusted(<String, Object?>{'selector': 'Parent'});
    expect(
      () => buildNpcStoryDraftMutationJson(
        context: _context(1),
        input: StoryNpcDraftInput(
          displayName: List<String>.filled(257, 'x').join(),
          moduleNamespace: 'GoreMods.Npcs.Bounded',
          uniqueName: 'GoreBounded',
          parentCharacterDefinition: small,
          parentAiAgentConfig: small,
          parentSpawnDefinition: small,
        ),
      ),
      throwsFormatException,
    );

    final bytes = Uint8List(7 * 1024 * 1024)
      ..fillRange(0, 7 * 1024 * 1024, 0x61);
    final large = CanonicalUnverifiedStoryJsonObject.fromCanonicalJson(
      jsonEncode(<String, Object?>{'blob': String.fromCharCodes(bytes)}),
    );
    expect(
      () => buildNpcStoryDraftMutationJson(
        context: _context(1),
        input: StoryNpcDraftInput(
          displayName: 'Aggregate bound',
          moduleNamespace: 'GoreMods.Npcs.AggregateBound',
          uniqueName: 'GoreAggregateBound',
          parentCharacterDefinition: large,
          parentAiAgentConfig: large,
          parentSpawnDefinition: large,
        ),
      ),
      throwsFormatException,
    );
  });

  test('malformed surrogates fail before JSON encoding', () {
    final small = _trusted(<String, Object?>{'selector': 'Parent'});
    expect(
      () => buildNpcStoryDraftMutationJson(
        context: _context(1),
        input: StoryNpcDraftInput(
          displayName: String.fromCharCode(0xd800),
          moduleNamespace: 'GoreMods.Npcs.Malformed',
          uniqueName: 'GoreMalformed',
          parentCharacterDefinition: small,
          parentAiAgentConfig: small,
          parentSpawnDefinition: small,
        ),
      ),
      throwsFormatException,
    );
    expect(
      () => CanonicalUnverifiedStoryJsonObject.fromCanonicalJson(
        '{"value":"\\ud800"}',
      ),
      throwsFormatException,
    );
  });

  test('secure ID generation has bounded zero and collision retries', () {
    expect(
      () => SecureStoryDraftIdSource(random: _ConstantRandom(0)).next(),
      throwsStateError,
    );
    expect(
      () => SecureStoryDraftIdSource(random: _ConstantRandom(1)).next(),
      throwsStateError,
    );
  });
}

StoryDraftMutationContext _context(int revision) => StoryDraftMutationContext(
  projectId: _projectId,
  revision: revision,
  ids: StoryDraftEntityIds(draftId: _draftId, scriptModuleId: _moduleId),
);

CanonicalUnverifiedStoryJsonObject _trusted(Map<String, Object?> value) =>
    CanonicalUnverifiedStoryJsonObject.fromCanonicalJson(jsonEncode(value));

final class _ConstantRandom implements Random {
  const _ConstantRandom(this.value);

  final int value;

  @override
  bool nextBool() => value.isOdd;

  @override
  double nextDouble() => value == 0 ? 0 : 0.5;

  @override
  int nextInt(int max) => value % max;
}
