import 'dart:async';
import 'dart:convert';
import 'dart:typed_data';

import 'package:crypto/crypto.dart' as crypto;
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/core/core_service.dart';
import 'package:gore_mod/core/mod_ffi.dart';
import 'package:gore_mod/story/domain/story_catalog_adapter.dart';
import 'package:gore_mod/story/domain/story_draft_requests.dart';
import 'package:gore_mod/story/domain/story_workspace_controller.dart';
import 'package:gore_mod/story/ui/story_workspace_view.dart';

const _catalogJson = '{"format":"story_catalog"}';
const _projectId = '01010101010101010101010101010101';
const _draftId = '10101010101010101010101010101010';
const _moduleId = '11111111111111111111111111111111';
const _asghanId = 'g1r:npc:om_grd_asghan_263';
const _viperId = 'g1r:npc:om_stt_viper_302';
const _generatedSource = 'class UGoreMineGuardArko : UCharacterDefinition {}\n';

void main() {
  late StoryCatalogAdapter catalog;

  setUp(() async {
    catalog = StoryCatalogAdapter.fromSelections(await _catalogSelections());
  });

  testWidgets(
    'normal NPC flow maps the chosen catalog row and derives technical IDs',
    (tester) async {
      StoryNpcDraftInput? submitted;
      final state = _workspace();
      await _pumpWorkspace(
        tester,
        state: state,
        catalog: catalog,
        createNpc: (input) async {
          submitted = input;
          return StoryDraftCreateRejected(
            state: state,
            diagnostics: const <AuthoringDiagnostic>[],
          );
        },
      );

      expect(find.text('Draft mode only'), findsOneWidget);
      expect(
        find.bySemanticsLabel('Draft-only Story workspace warning'),
        findsOneWidget,
      );
      expect(
        find.byKey(const Key('story-module-namespace-field')),
        findsNothing,
      );
      expect(find.byKey(const Key('story-unique-name-field')), findsNothing);
      expect(
        _textField(tester, 'story-display-name-field').controller!.text,
        'Asghan Copy',
      );

      await tester.tap(find.byType(DropdownButtonFormField<String>).first);
      await tester.pumpAndSettle();
      await tester.tap(find.text('Viper').last);
      await tester.pumpAndSettle();
      expect(
        _textField(tester, 'story-display-name-field').controller!.text,
        'Viper Copy',
      );

      await tester.enterText(
        find.byKey(const Key('story-display-name-field')),
        'Custom Viper',
      );
      await tester.tap(find.byKey(const Key('story-create-npc-button')));
      await tester.pumpAndSettle();

      expect(submitted, isNotNull);
      expect(submitted!.displayName, 'Custom Viper');
      expect(
        submitted!.moduleNamespace,
        matches(r'^GoreMods\.Npcs\.CustomViper_[A-F0-9]{32}$'),
      );
      expect(submitted!.uniqueName, matches(r'^GoreCustomViper_[A-F0-9]{32}$'));
      expect(
        submitted!.parentCharacterDefinition.canonicalJson,
        contains('UCharacterDefinition_Human_OM_STT_Viper_302'),
      );
    },
  );

  testWidgets(
    'Advanced technical fields are optional, editable, and resettable',
    (tester) async {
      StoryNpcDraftInput? submitted;
      final state = _workspace();
      await _pumpWorkspace(
        tester,
        state: state,
        catalog: catalog,
        createNpc: (input) async {
          submitted = input;
          return StoryDraftCreateRejected(
            state: state,
            diagnostics: const <AuthoringDiagnostic>[],
          );
        },
      );

      expect(find.text('Module namespace'), findsNothing);
      await tester.tap(find.byKey(const Key('story-technical-advanced')));
      await tester.pumpAndSettle();
      expect(find.text('Module namespace'), findsOneWidget);
      expect(find.text('Unique name'), findsOneWidget);
      await tester.enterText(
        find.byKey(const Key('story-module-namespace-field')),
        'MyMod.Npcs.Arko',
      );
      await tester.enterText(
        find.byKey(const Key('story-unique-name-field')),
        'MyArko',
      );
      await tester.enterText(
        find.byKey(const Key('story-display-name-field')),
        'Arko Changed',
      );
      expect(
        _textField(tester, 'story-module-namespace-field').controller!.text,
        'MyMod.Npcs.Arko',
      );
      await tester.tap(find.byKey(const Key('story-create-npc-button')));
      await tester.pumpAndSettle();
      expect(submitted, isNotNull);
      expect(submitted!.moduleNamespace, 'MyMod.Npcs.Arko');
      expect(submitted!.uniqueName, 'MyArko');
      expect(
        _textField(tester, 'story-module-namespace-field').controller!.text,
        'MyMod.Npcs.Arko',
      );

      await tester.tap(find.text('Reset to automatic values'));
      await tester.pump();
      expect(
        _textField(tester, 'story-module-namespace-field').controller!.text,
        matches(r'^GoreMods\.Npcs\.ArkoChanged_[A-F0-9]{32}$'),
      );
    },
  );

  testWidgets('applied result updates the persisted draft list', (
    tester,
  ) async {
    final initial = _workspace();
    final appliedState = _workspace(withDraft: true);
    final appliedDraft = appliedState.draftById(_draftId)!;
    await _pumpWorkspace(
      tester,
      state: initial,
      catalog: catalog,
      createNpc: (input) async => StoryDraftCreateApplied(
        state: appliedState,
        draft: appliedDraft,
        diagnostics: <AuthoringDiagnostic>[_combinedGate()],
      ),
    );

    await tester.enterText(
      find.byKey(const Key('story-display-name-field')),
      'Mine Guard Arko',
    );
    await tester.tap(find.byKey(const Key('story-create-npc-button')));
    await tester.pumpAndSettle();

    expect(find.byKey(const Key('story-success-message')), findsOneWidget);
    expect(find.byKey(const Key('story-draft-$_draftId')), findsOneWidget);
    expect(find.text('Mine Guard Arko'), findsWidgets);
    expect(
      find.textContaining('REVISION2_COMBINED_VALIDATION_UNAVAILABLE'),
      findsNothing,
    );
  });

  testWidgets('semantic rejection keeps state and shows friendly diagnostics', (
    tester,
  ) async {
    final state = _workspace();
    final rejection = _diagnostic(
      code: 'NPC_DUPLICATE_UNIQUE_NAME',
      message: 'That NPC identity is already used in this project.',
    );
    await _pumpWorkspace(
      tester,
      state: state,
      catalog: catalog,
      createNpc: (input) async => StoryDraftCreateRejected(
        state: state,
        diagnostics: <AuthoringDiagnostic>[rejection],
      ),
    );

    await tester.tap(find.byKey(const Key('story-create-npc-button')));
    await tester.pumpAndSettle();

    expect(find.byKey(const Key('story-error-message')), findsOneWidget);
    expect(
      find.textContaining('That NPC identity is already used in this project.'),
      findsOneWidget,
    );
    expect(find.text('No Story drafts yet.'), findsOneWidget);
  });

  testWidgets(
    'rejection adopts authoritative state and re-salts automatic collisions',
    (tester) async {
      final initial = _workspace();
      final occupiedSuffix = _autoSuffix(
        catalogId: _asghanId,
        displayName: 'Asghan Copy',
        revision: 8,
        draftCount: 1,
        salt: 0,
      );
      final occupiedNamespace = 'GoreMods.Npcs.AsghanCopy_$occupiedSuffix';
      final occupiedRuntimeId = 'GoreAsghanCopy_$occupiedSuffix';
      final authoritative = _workspace(
        withDraft: true,
        draftDisplayName: 'Existing Collision',
        moduleNamespace: occupiedNamespace,
        runtimeId: occupiedRuntimeId,
      );
      await _pumpWorkspace(
        tester,
        state: initial,
        catalog: catalog,
        createNpc: (_) async => StoryDraftCreateRejected(
          state: authoritative,
          diagnostics: <AuthoringDiagnostic>[
            _diagnostic(
              code: 'PROJECT_REVISION_CONFLICT',
              message: 'The project changed. Automatic values were refreshed.',
            ),
          ],
        ),
      );

      await tester.tap(find.byKey(const Key('story-technical-advanced')));
      await tester.pumpAndSettle();
      await tester.tap(find.byKey(const Key('story-create-npc-button')));
      await tester.pumpAndSettle();

      final expectedSuffix = _autoSuffix(
        catalogId: _asghanId,
        displayName: 'Asghan Copy',
        revision: 8,
        draftCount: 1,
        salt: 1,
      );
      expect(
        _textField(tester, 'story-module-namespace-field').controller!.text,
        'GoreMods.Npcs.AsghanCopy_$expectedSuffix',
      );
      expect(
        _textField(tester, 'story-unique-name-field').controller!.text,
        'GoreAsghanCopy_$expectedSuffix',
      );
      expect(
        _textField(tester, 'story-module-namespace-field').controller!.text,
        isNot(occupiedNamespace),
      );
      expect(find.text('Existing Collision'), findsWidgets);
      expect(find.text('Selected draft: Existing Collision'), findsOneWidget);
    },
  );

  testWidgets(
    'unexpected save errors stay actionable without leaking details',
    (tester) async {
      final state = _workspace();
      await _pumpWorkspace(
        tester,
        state: state,
        catalog: catalog,
        createNpc: (_) => throw StateError(r'C:\private\game\path failed'),
      );

      await tester.tap(find.byKey(const Key('story-create-npc-button')));
      await tester.pumpAndSettle();

      expect(find.byKey(const Key('story-error-message')), findsOneWidget);
      expect(
        find.text(
          'Something went wrong while saving the draft. Please try again.',
        ),
        findsOneWidget,
      );
      expect(find.textContaining(r'C:\private'), findsNothing);
    },
  );

  testWidgets(
    'busy state disables submission and late completion is mounted-safe',
    (tester) async {
      final state = _workspace();
      final pending = Completer<StoryDraftCreateResult>();
      var calls = 0;
      await _pumpWorkspace(
        tester,
        state: state,
        catalog: catalog,
        createNpc: (input) {
          calls++;
          return pending.future;
        },
      );

      await tester.tap(find.byKey(const Key('story-create-npc-button')));
      await tester.pump();
      expect(find.text('Creating draft...'), findsOneWidget);
      expect(
        tester
            .widget<FilledButton>(
              find.byKey(const Key('story-create-npc-button')),
            )
            .onPressed,
        isNull,
      );
      expect(_textField(tester, 'story-display-name-field').enabled, isFalse);
      await tester.tap(find.byKey(const Key('story-create-npc-button')));
      expect(calls, 1);

      await tester.pumpWidget(const SizedBox.shrink());
      pending.complete(
        StoryDraftCreateRejected(
          state: state,
          diagnostics: const <AuthoringDiagnostic>[],
        ),
      );
      await tester.pump();
      expect(tester.takeException(), isNull);
    },
  );

  testWidgets(
    'saved source is hidden until selected Advanced expansion opens',
    (tester) async {
      final state = _workspace(withDraft: true);
      await _pumpWorkspace(
        tester,
        state: state,
        catalog: catalog,
        createNpc: (_) async => StoryDraftCreateRejected(
          state: state,
          diagnostics: const <AuthoringDiagnostic>[],
        ),
      );

      expect(find.byKey(const Key('story-draft-$_draftId')), findsOneWidget);
      expect(find.text(_generatedSource), findsNothing);
      expect(find.textContaining('GoreMods.Npcs'), findsNothing);
      await tester.ensureVisible(find.text('Advanced: generated source'));
      await tester.tap(find.text('Advanced: generated source'));
      await tester.pumpAndSettle();
      expect(find.byKey(const Key('story-generated-source')), findsOneWidget);
      expect(find.text(_generatedSource), findsOneWidget);
    },
  );

  testWidgets('Quest choices are visible but creation is clearly disabled', (
    tester,
  ) async {
    final state = _workspace();
    await _pumpWorkspace(
      tester,
      state: state,
      catalog: catalog,
      createNpc: (_) async => StoryDraftCreateRejected(
        state: state,
        diagnostics: const <AuthoringDiagnostic>[],
      ),
    );

    await tester.ensureVisible(
      find.byKey(const Key('story-quest-disabled-reason')),
    );
    await tester.pumpAndSettle();
    expect(find.text('Not available yet'), findsOneWidget);
    expect(find.textContaining('collision inventory'), findsOneWidget);
    expect(find.text('Swamp Camp Chapter 2'), findsOneWidget);
    expect(find.text('Asghan'), findsWidgets);
    expect(find.text('Viper'), findsWidgets);
    expect(find.widgetWithText(FilledButton, 'Create Quest'), findsNothing);
    expect(find.text('Build'), findsNothing);
    expect(find.text('Deploy'), findsNothing);
    expect(find.textContaining('UQuest_'), findsNothing);
    expect(find.textContaining('sha256'), findsNothing);
  });
}

Future<void> _pumpWorkspace(
  WidgetTester tester, {
  required StoryWorkspaceState state,
  required StoryCatalogAdapter catalog,
  required StoryNpcDraftCreator createNpc,
}) async {
  await tester.binding.setSurfaceSize(const Size(1200, 1000));
  addTearDown(() => tester.binding.setSurfaceSize(null));
  await tester.pumpWidget(
    MaterialApp(
      theme: ThemeData(useMaterial3: true),
      home: Scaffold(
        body: StoryWorkspaceView(
          initialState: state,
          catalog: catalog,
          createNpc: createNpc,
        ),
      ),
    ),
  );
  await tester.pump();
}

TextFormField _textField(WidgetTester tester, String key) =>
    tester.widget<TextFormField>(find.byKey(Key(key)));

StoryWorkspaceState _workspace({
  bool withDraft = false,
  String draftDisplayName = 'Mine Guard Arko',
  String moduleNamespace = 'GoreMods.Npcs.MineGuardArko_A1B2C3D4',
  String runtimeId = 'GoreMineGuardArko_A1B2C3D4',
}) {
  final entities = <String, Object?>{};
  if (withDraft) {
    final owner = <String, Object?>{
      'project_id': _projectId,
      'id': _draftId,
      'expected_kind': 'npc_draft',
    };
    entities[_draftId] = <String, Object?>{
      'id': _draftId,
      'display_name': draftDisplayName,
      'origin': <String, Object?>{
        'type': 'new',
        'authored_runtime_id': runtimeId,
      },
      'revision': 0,
      'payload': <String, Object?>{
        'kind': 'npc_draft',
        'data': <String, Object?>{
          'generator_id': 'gore-authoring.logical-npc-clone-draft',
          'generator_version': 1,
          'input': <String, Object?>{},
          'script_module': <String, Object?>{
            'project_id': _projectId,
            'id': _moduleId,
            'expected_kind': 'script_module',
          },
        },
      },
    };
    entities[_moduleId] = <String, Object?>{
      'id': _moduleId,
      'display_name': moduleNamespace,
      'origin': <String, Object?>{
        'type': 'generated',
        'generator_id': 'gore-authoring.logical-npc-clone-draft',
        'generator_version': 1,
        'owner': owner,
      },
      'revision': 0,
      'payload': <String, Object?>{
        'kind': 'script_module',
        'data': <String, Object?>{
          'generator_id': 'gore-authoring.logical-npc-clone-draft',
          'generator_version': 1,
          'owner': owner,
          'module_namespace': moduleNamespace,
          'module_relative_path': '${moduleNamespace.replaceAll('.', '/')}.as',
          'source': _generatedSource,
          'source_sha256': crypto.sha256
              .convert(utf8.encode(_generatedSource))
              .toString(),
          'input_fingerprint': List<String>.filled(64, 'a').join(),
          'status': <String, Object?>{
            'authoring': 'offline_draft',
            'runtime': 'runtime_unqualified',
          },
        },
      },
    };
  }
  final projectJson = jsonEncode(<String, Object?>{
    'format': 2,
    'schema_revision': 2,
    'project_id': _projectId,
    'revision': withDraft ? 8 : 7,
    'meta': <String, Object?>{},
    'target': <String, Object?>{},
    'authoring_locales': <Object?>[],
    'entities': entities,
    'asset_store': <String, Object?>{},
  });
  return StoryWorkspaceState.fromCanonicalProjectJson(
    projectJson,
    blocksBuild: true,
    diagnostics: <AuthoringDiagnostic>[_combinedGate()],
  );
}

AuthoringDiagnostic _combinedGate() => _diagnostic(
  code: 'REVISION2_COMBINED_VALIDATION_UNAVAILABLE',
  message: 'Story drafts are not runtime-qualified yet.',
  propertyPath: 'schema_revision',
);

AuthoringDiagnostic _diagnostic({
  required String code,
  required String message,
  String? propertyPath,
}) => AuthoringDiagnostic.fromJson(<String, Object?>{
  'code': code,
  'severity': 'error',
  'entity': null,
  'property_path': propertyPath,
  'message': message,
  'related_entities': <Object?>[],
  'blocks_build': true,
});

String _autoSuffix({
  required String catalogId,
  required String displayName,
  required int revision,
  required int draftCount,
  required int salt,
}) => crypto.sha256
    .convert(
      utf8.encode(
        '$catalogId\u0000$displayName\u0000$_projectId'
        '\u0000$revision\u0000$draftCount\u0000$salt',
      ),
    )
    .toString()
    .substring(0, 32)
    .toUpperCase();

Future<AuthoringStoryCatalogSelections> _catalogSelections() => ModFfi(
  FakeGoreCoreFfiService(
    responses: <String, Map<String, Object?>>{
      'authoring_story_catalog_v1_read': _catalogResponse(),
    },
  ),
).authoringStoryCatalogV1Read(catalogJson: _catalogJson);

Map<String, Object?> _seal(String byte, int byteLength) => <String, Object?>{
  'byte_len': byteLength,
  'sha256': List<String>.filled(64, byte).join(),
};

String _selectorAlias(String catalogId, String role) {
  final bytes = <int>[
    ...utf8.encode('gore-story-catalog.authoring-selector-v1\u0000'),
  ];
  for (final value in <String>[catalogId, role]) {
    final encoded = utf8.encode(value);
    final length = Uint8List(8);
    ByteData.sublistView(length).setUint64(0, encoded.length, Endian.little);
    bytes
      ..addAll(length)
      ..addAll(encoded);
  }
  return 'Catalog_${crypto.sha256.convert(bytes)}';
}

Map<String, Object?> _classSelection(
  String catalogId,
  String role,
  String sealByte,
  String runtimeClass,
) => <String, Object?>{
  'catalog_layer': 'base-game.g1r.scripts',
  'authoring_selector': _selectorAlias(catalogId, role),
  'source_catalog_selector': 'script-class:Trusted/$runtimeClass',
  'runtime_class': runtimeClass,
  'source_seal': _seal(sealByte, 100),
};

Map<String, Object?> _npc({required bool viper}) {
  final runtime = viper ? 'OM_STT_Viper_302' : 'OM_GRD_Asghan_263';
  final catalogId = viper ? _viperId : _asghanId;
  final character = _classSelection(
    catalogId,
    'character_definition',
    viper ? 'e' : 'a',
    'UCharacterDefinition_Human_$runtime',
  );
  return <String, Object?>{
    'catalog_id': catalogId,
    'display_name': viper ? 'Viper' : 'Asghan',
    'runtime_unique_name': runtime,
    'character_definition': character,
    'ai_agent_config': _classSelection(
      catalogId,
      'ai_agent_config',
      viper ? 'd' : 'b',
      'UAIAgentConfig_Human_$runtime',
    ),
    'spawn_definition': _classSelection(
      catalogId,
      'spawn_definition',
      'c',
      'USpawnAIAgentDefinition_$runtime',
    ),
    'quest_giver': <String, Object?>{
      'catalog_layer': character['catalog_layer'],
      'authoring_selector': _selectorAlias(catalogId, 'quest_giver'),
      'source_catalog_selector': character['source_catalog_selector'],
      'runtime_unique_name': runtime,
      'source_seal': character['source_seal'],
    },
    'discovery_status': 'sealed_cache_defaults_verified',
    'authoring_qualification': 'offline_qualified',
    'runtime_qualification': 'runtime_unqualified',
    'evidence_id': viper
        ? 'npc-logical-clone-v1:viper-current-v1'
        : 'npc-logical-clone-v1',
    'blocks_build': true,
  };
}

Map<String, Object?> _catalogResponse() => <String, Object?>{
  'ok': true,
  'request_catalog_sha256': crypto.sha256
      .convert(utf8.encode(_catalogJson))
      .toString(),
  'selections': <String, Object?>{
    'schema_revision': 1,
    'generation': <String, Object?>{
      'edition': 'g1r-steam',
      'executable': _seal('1', 171698176),
      'shipping_cache': _seal('2', 123394250),
      'binds_cache': _seal('3', 5903938),
    },
    'catalog_seal': _seal('4', 5611),
    'npcs': <Object?>[_npc(viper: false), _npc(viper: true)],
    'quest_parents': <Object?>[
      <String, Object?>{
        'catalog_id': 'g1r:quest-parent:swampcamp_scchapter2',
        'display_name': 'Swamp Camp Chapter 2',
        'quest_class': _classSelection(
          'g1r:quest-parent:swampcamp_scchapter2',
          'quest_parent',
          'f',
          'UQuest_SwampCamp_SCCHAPTER2',
        ),
        'parent_class_name': 'UQuest_SwampCamp',
        'role': 'chapter',
        'qualification': 'curated_defaults_verified',
        'transition_qualification': 'runtime_unqualified',
        'evidence_id': 'current-cache-defaults-swampcamp-chapter2-20260712',
        'blocks_build': true,
      },
    ],
    'quest_collision_catalog': <String, Object?>{
      'status': 'inventory_unavailable',
      'catalog_layer': 'resolved-loadout.scripts.v1',
      'source_seal': _seal('2', 123394250),
      'blocks_draft_creation': true,
    },
    'blocks_build': true,
  },
};
