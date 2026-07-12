import 'dart:async';
import 'dart:convert';
import 'dart:io';
import 'dart:typed_data';

import 'package:crypto/crypto.dart' as crypto;
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/core/core_service.dart';
import 'package:gore_mod/core/mod_ffi.dart';
import 'package:gore_mod/story/domain/story_catalog_adapter.dart';
import 'package:gore_mod/story/domain/story_draft_requests.dart';
import 'package:gore_mod/story/domain/story_workspace_bootstrap.dart';
import 'package:gore_mod/story/domain/story_workspace_controller.dart';
import 'package:gore_mod/story/domain/story_workspace_launcher.dart';
import 'package:gore_mod/story/ui/story_workspace_flow.dart';
import 'package:gore_mod/story/ui/story_workspace_view.dart';

const _catalogJson = '{"format":"story_catalog"}';

void main() {
  late StoryCatalogAdapter catalog;
  late StoryWorkspaceState state;

  setUpAll(() async {
    catalog = StoryCatalogAdapter.fromSelections(await _catalogSelections());
    state = _emptyState();
  });

  testWidgets('create flow preserves game path and renders draft-only view', (
    tester,
  ) async {
    final session = _FakeSession(state: state, catalog: catalog);
    final launcher = _FakeLauncher()..createResult = Future.value(session);
    var metadataPrompts = 0;
    const configured = r'D:\Exact Configured Game Root';

    await tester.pumpWidget(
      _FlowHost(
        onOpen: (context) => runStoryWorkspaceFlow(
          context: context,
          mode: StoryWorkspaceFlowMode.create,
          configuredGamePath: configured,
          launcher: launcher,
          pickDirectory: () async => r'D:\Managed Story Workspace',
          promptMetadata: (_) async {
            metadataPrompts++;
            return StoryProjectMetadata(
              name: 'Visible Story Mod',
              version: '0.1.0',
              author: 'tests',
            );
          },
        ),
      ),
    );
    await tester.tap(find.text('Open flow'));
    await tester.pumpAndSettle();

    expect(metadataPrompts, 1);
    expect(launcher.createCalls, hasLength(1));
    expect(launcher.createCalls.single.configuredGamePath, configured);
    expect(
      launcher.createCalls.single.workspaceRoot.path,
      r'D:\Managed Story Workspace',
    );
    expect(launcher.createCalls.single.metadata.name, 'Visible Story Mod');
    expect(find.byType(StoryWorkspaceView), findsOneWidget);
    expect(find.text('Story workspace (drafts)'), findsOneWidget);
    expect(find.text('Build / Deploy'), findsNothing);
    await tester.tap(find.byKey(const Key('story-check-build-plan-button')));
    await tester.pumpAndSettle();
    expect(session.checkBuildPlanCalls, 1);
    expect(find.byKey(const Key('story-build-plan-result')), findsOneWidget);
  });

  testWidgets('open flow skips metadata and awaits close before Back pops', (
    tester,
  ) async {
    final close = Completer<void>();
    final session = _FakeSession(
      state: state,
      catalog: catalog,
      closeResult: close.future,
    );
    final launcher = _FakeLauncher()..openResult = Future.value(session);
    var metadataPrompts = 0;

    await tester.pumpWidget(
      _FlowHost(
        onOpen: (context) => runStoryWorkspaceFlow(
          context: context,
          mode: StoryWorkspaceFlowMode.open,
          configuredGamePath: 'game-root',
          launcher: launcher,
          pickDirectory: () async => 'workspace-root',
          promptMetadata: (_) async {
            metadataPrompts++;
            return null;
          },
        ),
      ),
    );
    await tester.tap(find.text('Open flow'));
    await tester.pumpAndSettle();
    expect(metadataPrompts, 0);
    expect(launcher.openCalls, hasLength(1));

    await tester.tap(find.byKey(const Key('story-workspace-back')));
    await tester.pump();
    expect(session.closeCalls, 1);
    expect(find.text('Closing workspace...'), findsOneWidget);
    expect(find.text('Story workspace (drafts)'), findsOneWidget);

    close.complete();
    await tester.pumpAndSettle();
    expect(find.text('Open flow'), findsOneWidget);
    expect(find.text('Story workspace (drafts)'), findsNothing);
    expect(session.closeCalls, 1);
  });

  testWidgets('Back during launch waits for late handle cleanup before pop', (
    tester,
  ) async {
    final acquired = Completer<StoryWorkspaceFlowSession>();
    final closed = Completer<void>();
    final session = _FakeSession(
      state: state,
      catalog: catalog,
      closeResult: closed.future,
    );
    final launcher = _FakeLauncher()..openResult = acquired.future;
    var flowReturned = false;
    await tester.pumpWidget(
      _FlowHost(
        onOpen: (context) async {
          await runStoryWorkspaceFlow(
            context: context,
            mode: StoryWorkspaceFlowMode.open,
            configuredGamePath: 'game-root',
            launcher: launcher,
            pickDirectory: () async => 'workspace-root',
          );
          flowReturned = true;
        },
      ),
    );
    await tester.tap(find.text('Open flow'));
    await tester.pump();
    await tester.pump();
    expect(find.text('Opening workspace...'), findsOneWidget);

    await tester.tap(find.byKey(const Key('story-workspace-back')));
    await tester.pump();
    expect(find.text('Closing workspace...'), findsOneWidget);
    expect(flowReturned, isFalse);

    acquired.complete(session);
    await tester.pump();
    await tester.pump();
    expect(session.closeCalls, 1);
    expect(flowReturned, isFalse);
    expect(find.text('Story workspace (drafts)'), findsOneWidget);

    closed.complete();
    await tester.pumpAndSettle();
    expect(flowReturned, isTrue);
    expect(find.text('Story workspace (drafts)'), findsNothing);
    expect(session.closeCalls, 1);
  });

  testWidgets('friendly path-free error can retry successfully', (
    tester,
  ) async {
    final session = _FakeSession(state: state, catalog: catalog);
    final launcher = _FakeLauncher();
    var attempts = 0;
    launcher.openCallback = () async {
      attempts++;
      if (attempts == 1) {
        throw const StoryWorkspaceLaunchException(
          StoryWorkspaceLaunchError.catalogBuildFailed,
          r'C:\secret\game.cache: native parser internals',
        );
      }
      return session;
    };

    await tester.pumpWidget(
      MaterialApp(
        home: StoryWorkspaceFlowPage(
          mode: StoryWorkspaceFlowMode.open,
          configuredGamePath: 'game-root',
          workspaceRoot: Directory('workspace-root'),
          launcher: launcher,
        ),
      ),
    );
    await tester.pumpAndSettle();

    expect(find.text('Could not open Story workspace'), findsOneWidget);
    final message = tester.widget<Text>(
      find.byKey(const Key('story-workspace-flow-error')),
    );
    expect(message.data, contains('catalog could not be read'));
    expect(message.data, isNot(contains('secret')));
    expect(message.data, isNot(contains(r'C:\')));

    await tester.tap(find.byKey(const Key('story-workspace-retry')));
    await tester.pumpAndSettle();
    expect(attempts, 2);
    expect(find.byType(StoryWorkspaceView), findsOneWidget);
  });

  testWidgets('rapid double Retry starts exactly one new launch', (
    tester,
  ) async {
    final retried = Completer<StoryWorkspaceFlowSession>();
    final session = _FakeSession(state: state, catalog: catalog);
    final launcher = _FakeLauncher();
    var attempts = 0;
    launcher.openCallback = () {
      attempts++;
      if (attempts == 1) {
        return Future<StoryWorkspaceFlowSession>.error(StateError('first'));
      }
      return retried.future;
    };
    await tester.pumpWidget(
      MaterialApp(
        home: StoryWorkspaceFlowPage(
          mode: StoryWorkspaceFlowMode.open,
          configuredGamePath: 'game-root',
          workspaceRoot: Directory('workspace-root'),
          launcher: launcher,
        ),
      ),
    );
    await tester.pumpAndSettle();
    final retry = find.byKey(const Key('story-workspace-retry'));
    expect(retry, findsOneWidget);

    await tester.tap(retry);
    await tester.tap(retry);
    expect(attempts, 2);
    await tester.pump();
    expect(find.text('Opening workspace...'), findsOneWidget);

    retried.complete(session);
    await tester.pumpAndSettle();
    expect(attempts, 2);
    expect(session.closeCalls, 0);
    expect(find.byType(StoryWorkspaceView), findsOneWidget);
  });

  testWidgets('late session completion after disposal is closed', (
    tester,
  ) async {
    final acquired = Completer<StoryWorkspaceFlowSession>();
    final session = _FakeSession(state: state, catalog: catalog);
    final launcher = _FakeLauncher()..openResult = acquired.future;

    await tester.pumpWidget(
      MaterialApp(
        home: StoryWorkspaceFlowPage(
          mode: StoryWorkspaceFlowMode.open,
          configuredGamePath: 'game-root',
          workspaceRoot: Directory('workspace-root'),
          launcher: launcher,
        ),
      ),
    );
    await tester.pump();
    await tester.pumpWidget(const MaterialApp(home: SizedBox()));
    acquired.complete(session);
    await tester.pump();
    await tester.pump();

    expect(session.closeCalls, 1);
  });

  testWidgets('external route disposal closes an active session', (
    tester,
  ) async {
    final session = _FakeSession(state: state, catalog: catalog);
    final launcher = _FakeLauncher()..openResult = Future.value(session);
    await tester.pumpWidget(
      MaterialApp(
        home: StoryWorkspaceFlowPage(
          mode: StoryWorkspaceFlowMode.open,
          configuredGamePath: 'game-root',
          workspaceRoot: Directory('workspace-root'),
          launcher: launcher,
        ),
      ),
    );
    await tester.pumpAndSettle();
    await tester.pumpWidget(const MaterialApp(home: SizedBox()));
    await tester.pump();

    expect(session.closeCalls, 1);
  });

  testWidgets('Back remains usable when lease cleanup reports an error', (
    tester,
  ) async {
    final session = _FakeSession(
      state: state,
      catalog: catalog,
      closeError: StateError('private cleanup detail'),
    );
    final launcher = _FakeLauncher()..openResult = Future.value(session);
    await tester.pumpWidget(
      _FlowHost(
        onOpen: (context) => runStoryWorkspaceFlow(
          context: context,
          mode: StoryWorkspaceFlowMode.open,
          configuredGamePath: 'game-root',
          launcher: launcher,
          pickDirectory: () async => 'workspace-root',
        ),
      ),
    );
    await tester.tap(find.text('Open flow'));
    await tester.pumpAndSettle();
    await tester.tap(find.byKey(const Key('story-workspace-back')));
    await tester.pumpAndSettle();

    expect(session.closeCalls, 1);
    expect(find.text('Open flow'), findsOneWidget);
  });

  testWidgets('cancelled picker does not prompt or launch', (tester) async {
    final launcher = _FakeLauncher();
    var prompts = 0;
    await tester.pumpWidget(
      _FlowHost(
        onOpen: (context) => runStoryWorkspaceFlow(
          context: context,
          mode: StoryWorkspaceFlowMode.create,
          configuredGamePath: 'game-root',
          launcher: launcher,
          pickDirectory: () async => null,
          promptMetadata: (_) async {
            prompts++;
            return StoryProjectMetadata(
              name: 'unused',
              version: '',
              author: '',
            );
          },
        ),
      ),
    );
    await tester.tap(find.text('Open flow'));
    await tester.pumpAndSettle();

    expect(prompts, 0);
    expect(launcher.createCalls, isEmpty);
    expect(find.byType(StoryWorkspaceFlowPage), findsNothing);
  });

  testWidgets('metadata dialog validates and returns bounded fields', (
    tester,
  ) async {
    StoryProjectMetadata? result;
    await tester.pumpWidget(
      MaterialApp(
        home: Builder(
          builder: (context) => TextButton(
            onPressed: () async {
              result = await showStoryProjectMetadataDialog(context);
            },
            child: const Text('Metadata'),
          ),
        ),
      ),
    );
    await tester.tap(find.text('Metadata'));
    await tester.pumpAndSettle();
    await tester.tap(find.byKey(const Key('story-project-create-button')));
    await tester.pump();
    expect(find.text('Enter a mod name.'), findsOneWidget);

    await tester.enterText(
      find.byKey(const Key('story-project-name-field')),
      'My Story Mod',
    );
    await tester.enterText(
      find.byKey(const Key('story-project-author-field')),
      'Daniel',
    );
    await tester.tap(find.byKey(const Key('story-project-create-button')));
    await tester.pumpAndSettle();
    expect(result?.name, 'My Story Mod');
    expect(result?.version, '0.1.0');
    expect(result?.author, 'Daniel');
  });
}

final class _FlowHost extends StatelessWidget {
  const _FlowHost({required this.onOpen});

  final Future<void> Function(BuildContext context) onOpen;

  @override
  Widget build(BuildContext context) => MaterialApp(
    home: Builder(
      builder: (context) => Scaffold(
        body: Center(
          child: FilledButton(
            onPressed: () => unawaited(onOpen(context)),
            child: const Text('Open flow'),
          ),
        ),
      ),
    ),
  );
}

final class _FakeLauncher implements StoryWorkspaceFlowLauncher {
  Future<StoryWorkspaceFlowSession>? createResult;
  Future<StoryWorkspaceFlowSession>? openResult;
  Future<StoryWorkspaceFlowSession> Function()? createCallback;
  Future<StoryWorkspaceFlowSession> Function()? openCallback;
  final List<
    ({
      String configuredGamePath,
      Directory workspaceRoot,
      StoryProjectMetadata metadata,
    })
  >
  createCalls = [];
  final List<({String configuredGamePath, Directory workspaceRoot})> openCalls =
      [];

  @override
  Future<StoryWorkspaceFlowSession> create({
    required String configuredGamePath,
    required Directory workspaceRoot,
    required StoryProjectMetadata metadata,
  }) {
    createCalls.add((
      configuredGamePath: configuredGamePath,
      workspaceRoot: workspaceRoot,
      metadata: metadata,
    ));
    return createCallback?.call() ?? createResult!;
  }

  @override
  Future<StoryWorkspaceFlowSession> open({
    required String configuredGamePath,
    required Directory workspaceRoot,
  }) {
    openCalls.add((
      configuredGamePath: configuredGamePath,
      workspaceRoot: workspaceRoot,
    ));
    return openCallback?.call() ?? openResult!;
  }
}

final class _FakeSession implements StoryWorkspaceFlowSession {
  _FakeSession({
    required this.state,
    required this.catalog,
    this.closeResult,
    this.closeError,
  });

  @override
  final StoryWorkspaceState state;
  @override
  final StoryCatalogAdapter catalog;
  final Future<void>? closeResult;
  final Object? closeError;
  int closeCalls = 0;
  int checkBuildPlanCalls = 0;

  @override
  Future<StoryBuildReadinessCheckResult> checkBuildPlan() async {
    checkBuildPlanCalls++;
    return StoryBuildReadinessChecked(
      projectRevision: state.revision,
      moduleCount: state.drafts.length,
      diagnosticCount: 1,
      blockingDiagnosticCount: 1,
    );
  }

  @override
  Future<void> close() {
    closeCalls++;
    if (closeError != null) return Future<void>.error(closeError!);
    return closeResult ?? Future<void>.value();
  }

  @override
  Future<StoryDraftCreateResult> createNpc(StoryNpcDraftInput input) =>
      throw UnimplementedError();
}

StoryWorkspaceState _emptyState() =>
    StoryWorkspaceState.fromCanonicalProjectJson(
      jsonEncode(<String, Object?>{
        'format': 2,
        'schema_revision': 2,
        'project_id': '01010101010101010101010101010101',
        'revision': 0,
        'meta': <String, Object?>{},
        'target': <String, Object?>{},
        'authoring_locales': <Object?>[],
        'entities': <String, Object?>{},
        'asset_store': <String, Object?>{},
      }),
      blocksBuild: true,
      diagnostics: <AuthoringDiagnostic>[
        AuthoringDiagnostic.fromJson(<String, Object?>{
          'code': 'REVISION2_COMBINED_VALIDATION_UNAVAILABLE',
          'severity': 'error',
          'entity': null,
          'property_path': 'schema_revision',
          'message': 'combined validation unavailable',
          'related_entities': <Object?>[],
          'blocks_build': true,
        }),
      ],
    );

Future<AuthoringStoryCatalogSelections> _catalogSelections() => ModFfi(
  FakeGoreCoreFfiService(
    responses: <String, Map<String, Object?>>{
      'authoring_story_catalog_v1_read': <String, Object?>{
        'ok': true,
        'request_catalog_sha256': crypto.sha256
            .convert(utf8.encode(_catalogJson))
            .toString(),
        'selections': <String, Object?>{
          'schema_revision': 1,
          'generation': <String, Object?>{
            'edition': 'g1r-steam',
            'executable': _seal('1', 100),
            'shipping_cache': _seal('2', 100),
            'binds_cache': _seal('3', 100),
          },
          'catalog_seal': _seal('4', 100),
          'npcs': <Object?>[_npc(viper: false), _npc(viper: true)],
          'quest_parents': <Object?>[_questParent()],
          'quest_collision_catalog': <String, Object?>{
            'status': 'inventory_unavailable',
            'catalog_layer': 'resolved-loadout.scripts.v1',
            'source_seal': _seal('2', 100),
            'blocks_draft_creation': true,
          },
          'blocks_build': true,
        },
      },
    },
  ),
).authoringStoryCatalogV1Read(catalogJson: _catalogJson);

Map<String, Object?> _npc({required bool viper}) {
  final id = viper ? 'g1r:npc:om_stt_viper_302' : 'g1r:npc:om_grd_asghan_263';
  final runtime = viper ? 'OM_STT_Viper_302' : 'OM_GRD_Asghan_263';
  final character = _classSelection(
    id,
    'character_definition',
    'a',
    'UCharacterDefinition_Human_$runtime',
  );
  return <String, Object?>{
    'catalog_id': id,
    'display_name': viper ? 'Viper' : 'Asghan',
    'runtime_unique_name': runtime,
    'character_definition': character,
    'ai_agent_config': _classSelection(
      id,
      'ai_agent_config',
      'b',
      'UAIAgentConfig_Human_$runtime',
    ),
    'spawn_definition': _classSelection(
      id,
      'spawn_definition',
      'c',
      'USpawnAIAgentDefinition_$runtime',
    ),
    'quest_giver': <String, Object?>{
      'catalog_layer': character['catalog_layer'],
      'authoring_selector': _selectorAlias(id, 'quest_giver'),
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

Map<String, Object?> _questParent() => <String, Object?>{
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
};

Map<String, Object?> _classSelection(
  String id,
  String role,
  String sealByte,
  String runtimeClass,
) => <String, Object?>{
  'catalog_layer': 'base-game.g1r.scripts',
  'authoring_selector': _selectorAlias(id, role),
  'source_catalog_selector': 'script-class:Trusted/$runtimeClass',
  'runtime_class': runtimeClass,
  'source_seal': _seal(sealByte, 100),
};

String _selectorAlias(String id, String role) {
  final bytes = <int>[
    ...utf8.encode('gore-story-catalog.authoring-selector-v1\u0000'),
  ];
  for (final value in <String>[id, role]) {
    final encoded = utf8.encode(value);
    final length = Uint8List(8);
    ByteData.sublistView(length).setUint64(0, encoded.length, Endian.little);
    bytes
      ..addAll(length)
      ..addAll(encoded);
  }
  return 'Catalog_${crypto.sha256.convert(bytes)}';
}

Map<String, Object?> _seal(String byte, int byteLength) => <String, Object?>{
  'byte_len': byteLength,
  'sha256': List<String>.filled(64, byte).join(),
};
