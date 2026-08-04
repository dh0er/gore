import 'dart:async';
import 'dart:convert';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/core/mod_ffi.dart';
import 'package:gore_mod/project/revision3_content_index.dart';
import 'package:gore_mod/project/revision3_npc_authoring.dart';
import 'package:gore_mod/project/revision3_npc_profile_edit_authoring.dart';
import 'package:gore_mod/project/revision3_npc_profile_edit_dialog.dart';

import '../support/revision3_npc_fixture.dart';

const _gameRoot = r'C:\Games\Gothic Remake';
const _currentCatalogId = 'g1r:npc:om_grd_asghan_263';
const _alternateCatalogId = 'g1r:npc:om_stt_viper_302';
const _copy = Revision3NpcProfileEditDialogCopy.english();

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();

  late _NpcProfileFixture fixture;

  setUp(() {
    fixture = _NpcProfileFixture.create();
  });

  testWidgets(
    'loads the friendly current profile without exposing technical identities',
    (tester) async {
      final catalog = fixture.catalog();
      await _openDialog(
        tester,
        fixture: fixture,
        service: fixture.service(catalogs: [catalog]),
      );

      expect(
        tester
            .widget<TextField>(
              find.byKey(const Key('revision3-npc-profile-edit-name')),
            )
            .controller!
            .text,
        'Inspection Guard',
      );
      final archetype = tester.widget<DropdownButtonFormField<String>>(
        find.byType(DropdownButtonFormField<String>),
      );
      expect(archetype.initialValue, _currentCatalogId);
      expect(find.text('Asghan guard'), findsOneWidget);
      expect(
        find.byKey(const Key('revision3-npc-profile-edit-boundary')),
        findsOneWidget,
      );

      for (final technicalIdentity in <String>[
        fixture.npc.id,
        fixture.module.id,
        fixture.seed.uniqueName,
        fixture.seed.moduleNamespace,
        fixture.seed.parentCharacterDefinition.runtimeClass,
        fixture.seed.parentAiAgentConfig.runtimeClass,
        fixture.seed.parentSpawnDefinition.runtimeClass,
        _currentCatalogId,
      ]) {
        expect(
          find.textContaining(technicalIdentity),
          findsNothing,
          reason: 'normal editing must hide $technicalIdentity',
        );
      }
    },
  );

  testWidgets('keeps no-op save disabled', (tester) async {
    await _openDialog(
      tester,
      fixture: fixture,
      service: fixture.service(catalogs: [fixture.catalog()]),
    );

    expect(_saveButton(tester).onPressed, isNull);
  });

  testWidgets('ignores a whitespace-only name difference', (tester) async {
    final plans = <Revision3NpcProfileEditTechnicalPlan>[];
    await _openDialog(
      tester,
      fixture: fixture,
      service: fixture.service(catalogs: [fixture.catalog()], plans: plans),
    );

    await tester.enterText(
      find.byKey(const Key('revision3-npc-profile-edit-name')),
      '  Inspection Guard  ',
    );
    await tester.pump();

    expect(_saveButton(tester).onPressed, isNull);
    expect(plans, isEmpty);
  });

  testWidgets('publishes a name-only edit with exact change flags', (
    tester,
  ) async {
    final catalog = fixture.catalog();
    final plans = <Revision3NpcProfileEditTechnicalPlan>[];
    Revision3NpcProfileEditPublication? result;
    await _openDialog(
      tester,
      fixture: fixture,
      service: fixture.service(catalogs: [catalog, catalog], plans: plans),
      onResult: (value) => result = value,
    );

    await tester.enterText(
      find.byKey(const Key('revision3-npc-profile-edit-name')),
      'North Gate Guard',
    );
    await tester.pump();
    expect(_saveButton(tester).onPressed, isNotNull);
    await tester.tap(find.byKey(const Key('revision3-npc-profile-edit-save')));
    await tester.pumpAndSettle();

    expect(plans, hasLength(1));
    expect(plans.single.displayName, 'North Gate Guard');
    expect(plans.single.parentCatalogId, _currentCatalogId);
    expect(plans.single.nameChanged, isTrue);
    expect(plans.single.archetypeChanged, isFalse);
    expect(plans.single.moduleRegenerated, isFalse);
    expect(result?.displayName, 'North Gate Guard');
    expect(result?.nameChanged, isTrue);
    expect(result?.archetypeChanged, isFalse);
  });

  testWidgets('publishes an archetype edit and regenerates its module', (
    tester,
  ) async {
    final catalog = fixture.catalog();
    final plans = <Revision3NpcProfileEditTechnicalPlan>[];
    await _openDialog(
      tester,
      fixture: fixture,
      service: fixture.service(catalogs: [catalog, catalog], plans: plans),
    );

    await _chooseArchetype(tester, 'Viper scout');
    await tester.tap(find.byKey(const Key('revision3-npc-profile-edit-save')));
    await tester.pumpAndSettle();

    expect(plans, hasLength(1));
    expect(plans.single.displayName, 'Inspection Guard');
    expect(plans.single.parentCatalogId, _alternateCatalogId);
    expect(plans.single.nameChanged, isFalse);
    expect(plans.single.archetypeChanged, isTrue);
    expect(plans.single.moduleRegenerated, isTrue);
  });

  testWidgets('catalog drift requires an explicit fresh re-review', (
    tester,
  ) async {
    final initial = fixture.catalog(storyDigit: '1', npcDigit: '2');
    final fresh = fixture.catalog(storyDigit: '3', npcDigit: '4');
    final plans = <Revision3NpcProfileEditTechnicalPlan>[];
    await _openDialog(
      tester,
      fixture: fixture,
      service: fixture.service(catalogs: [initial, fresh, fresh], plans: plans),
    );

    await tester.enterText(
      find.byKey(const Key('revision3-npc-profile-edit-name')),
      'Reviewed Guard',
    );
    await tester.pump();
    await tester.tap(find.byKey(const Key('revision3-npc-profile-edit-save')));
    await tester.pumpAndSettle();

    expect(plans, isEmpty);
    expect(find.text(_copy.catalogChanged), findsOneWidget);
    expect(_saveButton(tester).onPressed, isNull);
    final refreshedDropdown = tester.widget<DropdownButtonFormField<String>>(
      find.byType(DropdownButtonFormField<String>),
    );
    expect(refreshedDropdown.initialValue, isNull);
    expect(
      find.byKey(
        ValueKey(
          'revision3-npc-profile-edit-archetype-${fresh.npcCatalogSeal!.sha256}',
        ),
      ),
      findsOneWidget,
    );

    await _chooseArchetype(tester, 'Viper scout');
    expect(find.text(_copy.catalogChanged), findsNothing);
    expect(_saveButton(tester).onPressed, isNotNull);
    await tester.tap(find.byKey(const Key('revision3-npc-profile-edit-save')));
    await tester.pumpAndSettle();

    expect(plans, hasLength(1));
    expect(plans.single.displayName, 'Reviewed Guard');
    expect(plans.single.parentCatalogId, _alternateCatalogId);
    expect(plans.single.nameChanged, isTrue);
    expect(plans.single.archetypeChanged, isTrue);
  });

  for (final lockCase in <({String label, Object error, String message})>[
    (
      label: 'unavailable',
      error: const Revision3NpcProfileEditUnavailableException(),
      message: _copy.currentArchetypeUnavailable,
    ),
    (
      label: 'stale',
      error: const Revision3NpcProfileEditStaleCheckpointException(),
      message: _copy.stale,
    ),
    (
      label: 'requires reopen',
      error: const Revision3NpcProfileEditRequiresReopenException(),
      message: _copy.requiresReopen,
    ),
  ]) {
    testWidgets('${lockCase.label} load failure locks the editor', (
      tester,
    ) async {
      final service = Revision3NpcProfileEditAuthoringService(
        loadSeed:
            ({
              required npcId,
              required expectedNpcRevision,
              required expectedScriptModuleId,
              required expectedScriptModuleRevision,
              required expectedUniqueName,
              required expectedModuleNamespace,
              required expectedParentCharacterDefinition,
              required expectedParentAiAgentConfig,
              required expectedParentSpawnDefinition,
            }) async => throw lockCase.error,
        loadCatalog: (_) async => fixture.catalog(),
        publishTechnicalPlan: ({required gameRoot, required plan}) async =>
            throw StateError('publisher must stay unreachable'),
      );
      await _openDialog(tester, fixture: fixture, service: service);

      expect(find.text(lockCase.message), findsOneWidget);
      expect(
        find.byKey(const Key('revision3-npc-profile-edit-name')),
        findsNothing,
      );
      expect(
        find.byKey(const Key('revision3-npc-profile-edit-retry')),
        findsNothing,
      );
      expect(_saveButton(tester).onPressed, isNull);
      expect(
        tester
            .widget<TextButton>(
              find.byKey(const Key('revision3-npc-profile-edit-cancel')),
            )
            .child,
        isA<Text>().having((text) => text.data, 'label', _copy.close),
      );
    });
  }

  testWidgets('stale and reopen publication failures lock retrying', (
    tester,
  ) async {
    for (final lockCase in <({Object error, String message})>[
      (
        error: const Revision3NpcProfileEditStaleCheckpointException(),
        message: _copy.stale,
      ),
      (
        error: const Revision3NpcProfileEditRequiresReopenException(),
        message: _copy.requiresReopen,
      ),
    ]) {
      final catalog = fixture.catalog();
      final service = fixture.service(
        catalogs: [catalog, catalog],
        publishError: lockCase.error,
      );
      await _openDialog(tester, fixture: fixture, service: service);
      await tester.enterText(
        find.byKey(const Key('revision3-npc-profile-edit-name')),
        'Changed Guard',
      );
      await tester.pump();
      await tester.tap(
        find.byKey(const Key('revision3-npc-profile-edit-save')),
      );
      await tester.pumpAndSettle();

      expect(find.text(lockCase.message), findsOneWidget);
      expect(_saveButton(tester).onPressed, isNull);
      expect(find.widgetWithText(TextButton, _copy.close), findsOneWidget);

      await tester.pumpWidget(const SizedBox.shrink());
      await tester.pumpAndSettle();
    }
  });

  testWidgets('validates required, UTF-8 byte length, and controls', (
    tester,
  ) async {
    final catalog = fixture.catalog();
    final plans = <Revision3NpcProfileEditTechnicalPlan>[];
    await _openDialog(
      tester,
      fixture: fixture,
      service: fixture.service(catalogs: [catalog], plans: plans),
    );
    final name = find.byKey(const Key('revision3-npc-profile-edit-name'));
    final save = find.byKey(const Key('revision3-npc-profile-edit-save'));

    await tester.enterText(name, '   ');
    await tester.pump();
    await tester.tap(save);
    await tester.pump();
    expect(find.text(_copy.nameRequired), findsOneWidget);

    await tester.enterText(name, List<String>.filled(129, 'é').join());
    await tester.pump();
    await tester.tap(save);
    await tester.pump();
    expect(find.text(_copy.nameTooLong), findsOneWidget);

    await tester.enterText(name, 'Guard\u007f');
    await tester.pump();
    await tester.tap(save);
    await tester.pump();
    expect(find.text(_copy.nameControl), findsOneWidget);
    expect(plans, isEmpty);
  });

  testWidgets('busy save disables input and suppresses double submission', (
    tester,
  ) async {
    final catalog = fixture.catalog();
    final pending = Completer<Revision3NpcProfileEditPublication>();
    Revision3NpcProfileEditTechnicalPlan? pendingPlan;
    var publishCalls = 0;
    final service = fixture.service(
      catalogs: [catalog, catalog],
      publish: (plan) {
        publishCalls++;
        pendingPlan = plan;
        return pending.future;
      },
    );
    await _openDialog(tester, fixture: fixture, service: service);
    await tester.enterText(
      find.byKey(const Key('revision3-npc-profile-edit-name')),
      'Patient Guard',
    );
    await tester.pump();

    await tester.tap(find.byKey(const Key('revision3-npc-profile-edit-save')));
    await tester.pump();

    expect(publishCalls, 1);
    expect(find.text(_copy.saving), findsOneWidget);
    expect(_saveButton(tester).onPressed, isNull);
    expect(
      tester
          .widget<TextButton>(
            find.byKey(const Key('revision3-npc-profile-edit-cancel')),
          )
          .onPressed,
      isNull,
    );
    expect(
      tester
          .widget<TextField>(
            find.byKey(const Key('revision3-npc-profile-edit-name')),
          )
          .enabled,
      isFalse,
    );
    expect(
      tester
          .widget<DropdownButtonFormField<String>>(
            find.byType(DropdownButtonFormField<String>),
          )
          .onChanged,
      isNull,
    );

    await tester.tap(
      find.byKey(const Key('revision3-npc-profile-edit-save')),
      warnIfMissed: false,
    );
    await tester.pump();
    expect(publishCalls, 1);

    pending.complete(fixture.publication(plan: pendingPlan!));
    await tester.pumpAndSettle();
    expect(
      find.byKey(const Key('revision3-npc-profile-edit-dialog')),
      findsNothing,
    );
  });

  testWidgets('dirty cancel and back require explicit discard', (tester) async {
    Revision3NpcProfileEditPublication? result;
    await _openDialog(
      tester,
      fixture: fixture,
      service: fixture.service(catalogs: [fixture.catalog()]),
      onResult: (value) => result = value,
    );
    await tester.enterText(
      find.byKey(const Key('revision3-npc-profile-edit-name')),
      'Unsaved Guard',
    );
    await tester.pump();

    await tester.tap(
      find.byKey(const Key('revision3-npc-profile-edit-cancel')),
    );
    await tester.pumpAndSettle();
    expect(
      find.byKey(const Key('revision3-npc-profile-edit-discard-dialog')),
      findsOneWidget,
    );
    await tester.tap(
      find.byKey(const Key('revision3-npc-profile-edit-keep-editing')),
    );
    await tester.pumpAndSettle();
    expect(
      find.byKey(const Key('revision3-npc-profile-edit-dialog')),
      findsOneWidget,
    );

    await tester.binding.handlePopRoute();
    await tester.pumpAndSettle();
    expect(
      find.byKey(const Key('revision3-npc-profile-edit-discard-dialog')),
      findsOneWidget,
    );
    await tester.tap(
      find.byKey(const Key('revision3-npc-profile-edit-discard')),
    );
    await tester.pumpAndSettle();

    expect(
      find.byKey(const Key('revision3-npc-profile-edit-dialog')),
      findsNothing,
    );
    expect(result, isNull);
  });
}

FilledButton _saveButton(WidgetTester tester) => tester.widget<FilledButton>(
  find.byKey(const Key('revision3-npc-profile-edit-save')),
);

Future<void> _chooseArchetype(WidgetTester tester, String label) async {
  await tester.tap(find.byType(DropdownButtonFormField<String>));
  await tester.pumpAndSettle();
  await tester.tap(find.text(label).last);
  await tester.pumpAndSettle();
}

Future<void> _openDialog(
  WidgetTester tester, {
  required _NpcProfileFixture fixture,
  required Revision3NpcProfileEditAuthoringService service,
  ValueChanged<Revision3NpcProfileEditPublication?>? onResult,
}) async {
  await tester.binding.setSurfaceSize(const Size(1280, 900));
  addTearDown(() => tester.binding.setSurfaceSize(null));
  await tester.pumpWidget(
    MaterialApp(
      home: Builder(
        builder: (context) => Scaffold(
          body: Center(
            child: FilledButton(
              key: const Key('open-profile-editor'),
              onPressed: () async {
                final result =
                    await showDialog<Revision3NpcProfileEditPublication>(
                      context: context,
                      builder: (_) => Revision3NpcProfileEditDialog(
                        index: fixture.index,
                        npc: fixture.npc,
                        gameRoot: _gameRoot,
                        service: service,
                      ),
                    );
                onResult?.call(result);
              },
              child: const Text('Open profile editor'),
            ),
          ),
        ),
      ),
    ),
  );
  await tester.tap(find.byKey(const Key('open-profile-editor')));
  await tester.pumpAndSettle();
}

typedef _TechnicalPublish =
    Future<Revision3NpcProfileEditPublication> Function(
      Revision3NpcProfileEditTechnicalPlan plan,
    );

final class _NpcProfileFixture {
  const _NpcProfileFixture._({
    required this.projectJson,
    required this.head,
    required this.index,
    required this.npc,
    required this.module,
    required this.seed,
  });

  factory _NpcProfileFixture.create() {
    final emptyProjectJson = revision3NpcInspectionProjectJson();
    final emptyProject = (jsonDecode(emptyProjectJson) as Map)
        .cast<String, Object?>();
    final request = AuthoringRevision3NpcDraftRequestV1.forProject(
      expectedHead: revision3NpcFixtureHead(emptyProjectJson),
      currentProjectJson: emptyProjectJson,
      npcId: revision3NpcInspectionNpcId,
      scriptModuleId: revision3NpcInspectionModuleId,
      displayName: 'Inspection Guard',
      intent: AuthoringRevision3NpcDraftIntentV1(
        moduleNamespace: revision3NpcInspectionModuleNamespace,
        uniqueName: revision3NpcInspectionUniqueName,
        parentCatalogId: _currentCatalogId,
      ),
    );
    final target = (emptyProject['target']! as Map).cast<String, Object?>();
    final input = revision3NpcFixtureInput(request: request, target: target);
    final npcEntity = revision3NpcFixtureEntity(
      projectId: revision3NpcInspectionProjectId,
      request: request,
      input: input,
    )..['revision'] = 2;
    final moduleEntity = revision3NpcFixtureModuleEntity(
      projectId: revision3NpcInspectionProjectId,
      request: request,
      input: input,
    )..['revision'] = 3;
    emptyProject['entities'] = <String, Object?>{
      revision3NpcInspectionNpcId: npcEntity,
      revision3NpcInspectionModuleId: moduleEntity,
    };
    final projectJson = jsonEncode(emptyProject);
    final head = revision3NpcFixtureHead(projectJson);
    final index = Revision3ContentIndex.fromJsonObject(<String, Object?>{
      'schema_revision': 1,
      'project_id': revision3NpcInspectionProjectId,
      'project_revision': 7,
      'project_name': 'NPC profile editor fixture',
      'project_version': '1.0.0',
      'project_author': 'tests',
      'target': target,
      'authoring_locales': <Object?>[],
      'entity_counts': <String, Object?>{'npc_draft': 1, 'script_module': 1},
      'entities': <Object?>[
        <String, Object?>{
          'id': revision3NpcInspectionNpcId,
          'kind': 'npc_draft',
          'display_name': 'Inspection Guard',
          'revision': 2,
          'origin': npcEntity['origin'],
          'summary': <String, Object?>{
            'kind': 'npc_draft',
            'data': <String, Object?>{
              'unique_name': revision3NpcInspectionUniqueName,
              'module_namespace': revision3NpcInspectionModuleNamespace,
              'parent_character_definition':
                  'UCharacterDefinition_Human_OM_GRD_Asghan_263',
              'parent_ai_agent_config':
                  'UAIAgentConfig_Human_OM_GRD_Asghan_263',
              'parent_spawn_definition':
                  'USpawnAIAgentDefinition_OM_GRD_Asghan_263',
              'greeting_count': 0,
            },
          },
          'references': <Object?>[
            _reference(
              role: 'draft_script_module',
              targetId: revision3NpcInspectionModuleId,
              expectedKind: 'script_module',
            ),
          ],
          'asset_references': <Object?>[],
        },
        <String, Object?>{
          'id': revision3NpcInspectionModuleId,
          'kind': 'script_module',
          'display_name': revision3NpcInspectionModuleNamespace,
          'revision': 3,
          'origin': <String, Object?>{
            'type': 'generated',
            'generator_id': revision3NpcFixtureGeneratorId,
            'generator_version': revision3NpcFixtureGeneratorVersion,
            'owner': <String, Object?>{
              'project_id': revision3NpcInspectionProjectId,
              'entity_id': revision3NpcInspectionNpcId,
              'expected_kind': 'npc_draft',
            },
          },
          'summary': <String, Object?>{
            'kind': 'script_module',
            'data': <String, Object?>{
              'generator_id': revision3NpcFixtureGeneratorId,
              'generator_version': revision3NpcFixtureGeneratorVersion,
              'module_namespace': revision3NpcInspectionModuleNamespace,
              'module_relative_path':
                  '${revision3NpcInspectionModuleNamespace.replaceAll('.', '/')}.as',
              'status': <String, Object?>{
                'authoring': 'offline_draft',
                'runtime': 'runtime_unqualified',
              },
            },
          },
          'references': <Object?>[
            _reference(
              role: 'origin_owner',
              targetId: revision3NpcInspectionNpcId,
              expectedKind: 'npc_draft',
            ),
            _reference(
              role: 'script_owner',
              targetId: revision3NpcInspectionNpcId,
              expectedKind: 'npc_draft',
            ),
          ],
          'asset_references': <Object?>[],
        },
      ],
      'assets': <Object?>[],
    });
    final npc = index.entityById(revision3NpcInspectionNpcId)!;
    final module = index.entityById(revision3NpcInspectionModuleId)!;
    final seed = AuthoringRevision3NpcProfileEditSeed.forProject(
      head: head,
      currentProjectJson: projectJson,
      npcId: npc.id,
      expectedNpcRevision: npc.revision,
      expectedScriptModuleId: module.id,
      expectedScriptModuleRevision: module.revision,
      expectedUniqueName: revision3NpcInspectionUniqueName,
      expectedModuleNamespace: revision3NpcInspectionModuleNamespace,
      expectedParentCharacterDefinition:
          'UCharacterDefinition_Human_OM_GRD_Asghan_263',
      expectedParentAiAgentConfig: 'UAIAgentConfig_Human_OM_GRD_Asghan_263',
      expectedParentSpawnDefinition:
          'USpawnAIAgentDefinition_OM_GRD_Asghan_263',
    );
    return _NpcProfileFixture._(
      projectJson: projectJson,
      head: head,
      index: index,
      npc: npc,
      module: module,
      seed: seed,
    );
  }

  final String projectJson;
  final AuthoringWorkingHead head;
  final Revision3ContentIndex index;
  final Revision3ContentEntity npc;
  final Revision3ContentEntity module;
  final AuthoringRevision3NpcProfileEditSeed seed;

  Revision3NpcCatalog catalog({
    String storyDigit = '1',
    String npcDigit = '2',
  }) => Revision3NpcCatalog(
    choices: <Revision3NpcCatalogChoice>[
      Revision3NpcCatalogChoice(
        catalogId: _currentCatalogId,
        displayName: 'Asghan guard',
        parentTriple: Revision3NpcCatalogParentTriple(
          characterDefinition: _binding(seed.parentCharacterDefinition),
          aiAgentConfig: _binding(seed.parentAiAgentConfig),
          spawnDefinition: _binding(seed.parentSpawnDefinition),
        ),
      ),
      Revision3NpcCatalogChoice(
        catalogId: _alternateCatalogId,
        displayName: 'Viper scout',
        parentTriple: Revision3NpcCatalogParentTriple(
          characterDefinition: _alternateBinding('character', '7'),
          aiAgentConfig: _alternateBinding('agent', '8'),
          spawnDefinition: _alternateBinding('spawn', '9'),
        ),
      ),
    ],
    generationExecutableSeal:
        seed.parentCharacterDefinition.generation.executable,
    storyCatalogSeal: _seal(100, storyDigit),
    npcCatalogSeal: _seal(200, npcDigit),
  );

  Revision3NpcProfileEditAuthoringService service({
    required List<Revision3NpcCatalog> catalogs,
    List<Revision3NpcProfileEditTechnicalPlan>? plans,
    _TechnicalPublish? publish,
    Object? publishError,
  }) {
    var catalogIndex = 0;
    return Revision3NpcProfileEditAuthoringService(
      loadSeed:
          ({
            required npcId,
            required expectedNpcRevision,
            required expectedScriptModuleId,
            required expectedScriptModuleRevision,
            required expectedUniqueName,
            required expectedModuleNamespace,
            required expectedParentCharacterDefinition,
            required expectedParentAiAgentConfig,
            required expectedParentSpawnDefinition,
          }) async => seed,
      loadCatalog: (_) async {
        if (catalogIndex >= catalogs.length) {
          throw StateError('unexpected catalog refresh');
        }
        return catalogs[catalogIndex++];
      },
      publishTechnicalPlan: ({required gameRoot, required plan}) async {
        plans?.add(plan);
        if (publishError != null) throw publishError;
        if (publish != null) return publish(plan);
        return publication(plan: plan);
      },
    );
  }

  Revision3NpcProfileEditPublication publication({
    required Revision3NpcProfileEditTechnicalPlan plan,
  }) => Revision3NpcProfileEditPublication(
    projectId: plan.projectId,
    projectRevision: plan.projectRevision + 1,
    npcId: plan.npcId,
    npcRevision: plan.expectedNpcRevision + 1,
    scriptModuleId: plan.scriptModuleId,
    scriptModuleRevision:
        plan.expectedScriptModuleRevision + (plan.moduleRegenerated ? 1 : 0),
    displayName: plan.displayName,
    previousParentCatalogId: plan.expectedParentCatalogId,
    parentCatalogId: plan.parentCatalogId,
    nameChanged: plan.nameChanged,
    archetypeChanged: plan.archetypeChanged,
    moduleRegenerated: plan.moduleRegenerated,
  );
}

Revision3NpcCatalogParentBinding _binding(
  AuthoringRevision3NpcInspectionParent parent,
) => Revision3NpcCatalogParentBinding(
  catalogLayer: parent.catalogLayer,
  authoringSelector: parent.canonicalSelector,
  runtimeClass: parent.runtimeClass,
  sourceSeal: parent.sourceSeal,
);

Revision3NpcCatalogParentBinding _alternateBinding(String role, String digit) =>
    Revision3NpcCatalogParentBinding(
      catalogLayer: revision3NpcFixtureCatalogLayer,
      authoringSelector: 'Catalog_${List<String>.filled(64, digit).join()}',
      runtimeClass: 'UViper${role[0].toUpperCase()}${role.substring(1)}',
      sourceSeal: _seal(300 + role.length, digit),
    );

AuthoringDraftContentSeal _seal(int byteLength, String digit) =>
    AuthoringDraftContentSeal.fromJson(<String, Object?>{
      'byte_len': byteLength,
      'sha256': List<String>.filled(64, digit).join(),
    });

Map<String, Object?> _reference({
  required String role,
  required String targetId,
  required String expectedKind,
}) => <String, Object?>{
  'role': role,
  'qualifier': null,
  'target': <String, Object?>{
    'project_id': revision3NpcInspectionProjectId,
    'entity_id': targetId,
    'expected_kind': expectedKind,
  },
  'resolution': 'resolved',
};
