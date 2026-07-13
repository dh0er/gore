import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:goresave/features/editor/domain/character_index.dart';
import 'package:goresave/features/editor/domain/core_service.dart';
import 'package:goresave/features/editor/domain/editor_models.dart';
import 'package:goresave/features/editor/domain/editor_notifier.dart';
import 'package:goresave/features/editor/domain/glossary_models.dart';
import 'package:goresave/features/editor/domain/glossary_npc_catalog.dart';
import 'package:goresave/features/editor/domain/npc_actors_page.dart';
import 'package:goresave/features/editor/domain/npc_attributes.dart';
import 'package:goresave/features/editor/domain/progression_models.dart';
import 'package:goresave/features/editor/ui/glossary_panel.dart';
import 'package:goresave/features/editor/ui/npc_attributes_panel.dart';
import 'package:goresave/features/editor/ui/npc_relationship_editor.dart';
import 'package:goresave/features/editor/ui/progression_panel.dart'
    show EventsDetail;
import 'package:goresave/loc/loc_catalog_provider.dart';

import 'support/l10n_test_app.dart';

void main() {
  test('NPC relationship summary parses only a stored permanent override', () {
    final actor = NpcActor.fromJson({
      'id': 'OC_GRD_ASGHAN_253-Instance',
      'isDead': false,
      'personalRelationship': 'Friend',
    });

    expect(actor.personalRelationship, NpcRelationship.friend);
    // Missing is deliberately unknown/game-computed, never fabricated Neutral.
    expect(
      NpcActor.fromJson(const {
        'id': 'OldMock',
        'isDead': false,
      }).personalRelationship,
      isNull,
    );
  });

  test('relationship target is queued and rehydrated per NPC', () async {
    final notifier = EditorNotifier(_NoopCore(), saveDir: r'C:\tmp\saves');
    await pumpEventQueue();

    notifier.setPendingNpcRelationship('Asghan-1', NpcRelationship.friend);

    final pending = notifier.state.pendingEdits['npc.relationship:Asghan-1'];
    expect(pending, isNotNull);
    expect(pending!.edits.single, {
      'path': 'private.npc.setRelationship',
      'value': {'id': 'Asghan-1', 'relationship': 'Friend'},
    });
    expect(notifier.pendingNpcRelationship('Asghan-1'), NpcRelationship.friend);

    notifier.clearPendingNpcRelationship('Asghan-1');
    expect(notifier.pendingNpcRelationship('Asghan-1'), isNull);
  });

  testWidgets('relationship row shows stored override and edits separately', (
    tester,
  ) async {
    final notifier = _RelationshipEditorNotifier(
      const NpcActor(
        id: 'Asghan-1',
        isDead: true,
        personalRelationship: NpcRelationship.friend,
      ),
    );
    await tester.pumpWidget(
      wrapWithL10n(
        Scaffold(
          body: SizedBox(
            width: 900,
            height: 600,
            child: NpcRelationshipEditor(
              npcId: 'Asghan-1',
              notifier: notifier,
              editable: true,
              reloadKey: _relationshipInspection(supported: true),
            ),
          ),
        ),
      ),
    );
    await tester.pumpAndSettle();

    expect(find.text('Relationship'), findsOneWidget);
    expect(find.text('Relationship override'), findsNothing);
    expect(
      find.text(
        'Stored as a permanent NPC-to-player override. '
        'Guild, story, area, and crime rules can still change the effective '
        'status in game.',
      ),
      findsOneWidget,
    );

    final dropdown = tester.widget<DropdownButtonFormField<NpcRelationship>>(
      find.byType(DropdownButtonFormField<NpcRelationship>),
    );
    expect(dropdown.initialValue, NpcRelationship.friend);
    // A synthetic null cannot remove an override that already exists in the
    // save; only choosing that stored value clears a different pending draft.
    dropdown.onChanged!(null);
    expect(notifier.pendingNpcRelationship('Asghan-1'), isNull);
    dropdown.onChanged!(NpcRelationship.enemy);
    expect(notifier.pendingNpcRelationship('Asghan-1'), NpcRelationship.enemy);
    dropdown.onChanged!(NpcRelationship.friend);
    expect(notifier.pendingNpcRelationship('Asghan-1'), isNull);
  });

  testWidgets(
    'relationship control is disabled when the save lacks the capability',
    (tester) async {
      final notifier = _RelationshipEditorNotifier(
        const NpcActor(id: 'Unsupported-1', isDead: false),
      );
      await tester.pumpWidget(
        wrapWithL10n(
          Scaffold(
            body: SizedBox(
              width: 900,
              height: 600,
              child: NpcRelationshipEditor(
                npcId: 'Unsupported-1',
                notifier: notifier,
                editable: true,
                // Deliberately unsupported: inspect_save did not advertise
                // private.npc.setRelationship for this save.
                reloadKey: _relationshipInspection(supported: false),
              ),
            ),
          ),
        ),
      );
      await tester.pumpAndSettle();

      final dropdown = tester.widget<DropdownButtonFormField<NpcRelationship>>(
        find.byType(DropdownButtonFormField<NpcRelationship>),
      );
      expect(dropdown.onChanged, isNull);
      expect(find.text('Computed by game'), findsOneWidget);
    },
  );

  testWidgets('missing relationship override is shown as game-computed', (
    tester,
  ) async {
    final notifier = _RelationshipEditorNotifier(
      const NpcActor(id: 'Buster-1', isDead: false),
    );
    await tester.pumpWidget(
      wrapWithL10n(
        Scaffold(
          body: SizedBox(
            width: 900,
            height: 600,
            child: NpcRelationshipEditor(
              npcId: 'Buster-1',
              notifier: notifier,
              editable: true,
              reloadKey: _relationshipInspection(supported: true),
            ),
          ),
        ),
      ),
    );
    await tester.pumpAndSettle();

    final dropdown = tester.widget<DropdownButtonFormField<NpcRelationship>>(
      find.byType(DropdownButtonFormField<NpcRelationship>),
    );
    expect(dropdown.initialValue, isNull);
    expect(find.text('Computed by game'), findsOneWidget);
    expect(
      find.textContaining('Guild, story, area, and crime rules'),
      findsOneWidget,
    );
    expect(find.text('Neutral'), findsNothing);
  });

  testWidgets('pending relationship is shown optimistically', (tester) async {
    final notifier = _RelationshipEditorNotifier(
      const NpcActor(id: 'Asghan-1', isDead: false),
    );
    await tester.pumpWidget(
      wrapWithL10n(
        Scaffold(
          body: SizedBox(
            width: 900,
            height: 600,
            child: NpcRelationshipEditor(
              npcId: 'Asghan-1',
              notifier: notifier,
              editable: true,
              reloadKey: _relationshipInspection(supported: true),
            ),
          ),
        ),
      ),
    );
    await tester.pumpAndSettle();
    notifier.setPendingNpcRelationship('Asghan-1', NpcRelationship.friend);
    await tester.pump();

    final dropdown = tester.widget<DropdownButtonFormField<NpcRelationship>>(
      find.byType(DropdownButtonFormField<NpcRelationship>),
    );
    expect(dropdown.initialValue, NpcRelationship.friend);
    expect(find.text('Will be Friend on save'), findsOneWidget);

    // Returning to the saved null state clears the queued target. It does not
    // request deletion of an existing stored override, where Automatic is
    // deliberately absent.
    await tester.tap(find.byType(DropdownButtonFormField<NpcRelationship>));
    await tester.pumpAndSettle();
    expect(find.text('Computed by game'), findsOneWidget);
    await tester.tap(find.text('Computed by game'));
    await tester.pumpAndSettle();
    expect(notifier.pendingNpcRelationship('Asghan-1'), isNull);
  });

  testWidgets('relationship editor is absent from NPC attributes', (
    tester,
  ) async {
    await tester.pumpWidget(
      wrapWithL10n(
        Scaffold(
          body: SizedBox(
            width: 900,
            height: 600,
            child: NpcAttributesPanel(
              load: () async => const NpcAttributesResult(attributes: []),
              onPendingChanged: (_, _) {},
              editable: true,
              reloadKey: 'Asghan-1',
              status: NpcStatusConfig(
                npcId: 'Asghan-1',
                editable: true,
                reloadKey: 'Asghan-1',
                knownDead: true,
                load: () async => const NpcActorsPage(
                  npcs: [
                    NpcActor(
                      id: 'Asghan-1',
                      isDead: true,
                      personalRelationship: NpcRelationship.friend,
                    ),
                  ],
                ),
                onRevive: () {},
              ),
            ),
          ),
        ),
      ),
    );
    await tester.pumpAndSettle();

    expect(find.text('Status'), findsOneWidget);
    expect(find.byKey(const Key('npc-relationship-editor')), findsNothing);
    expect(find.byType(DropdownButtonFormField<NpcRelationship>), findsNothing);
  });

  testWidgets('NPC events show Relationship directly above pagination', (
    tester,
  ) async {
    final notifier = _RelationshipEventsNotifier(
      const NpcActor(
        id: 'Asghan-1',
        isDead: false,
        personalRelationship: NpcRelationship.friend,
      ),
    );
    await tester.pumpWidget(
      wrapWithL10n(
        Scaffold(
          body: SizedBox(
            width: 900,
            height: 600,
            child: EventsDetail(
              globalId: 'Asghan-1',
              notifier: notifier,
              editable: true,
              reloadKey: _relationshipInspection(supported: true),
              theme: ThemeData.light(),
              relationshipNpcId: 'Asghan-1',
              relationshipEditable: true,
            ),
          ),
        ),
      ),
    );
    await tester.pumpAndSettle();

    final relationship = find.byKey(const Key('npc-relationship-editor'));
    final pagination = find.text('Page 1 / 1');
    expect(relationship, findsOneWidget);
    expect(find.text('Relationship'), findsOneWidget);
    expect(pagination, findsOneWidget);
    expect(
      tester.getTopLeft(relationship).dy,
      lessThan(tester.getTopLeft(pagination).dy),
    );

    // Player events use the same detail but never opt into the NPC-only editor.
    await tester.pumpWidget(
      wrapWithL10n(
        Scaffold(
          body: SizedBox(
            width: 900,
            height: 600,
            child: EventsDetail(
              globalId: 'Hero-1',
              notifier: notifier,
              editable: true,
              reloadKey: _relationshipInspection(supported: true),
              theme: ThemeData.light(),
            ),
          ),
        ),
      ),
    );
    await tester.pumpAndSettle();
    expect(find.byKey(const Key('npc-relationship-editor')), findsNothing);
  });

  testWidgets(
    'mounted glossary reacts live to pending relationship changes and clears',
    (tester) async {
      await tester.binding.setSurfaceSize(const Size(1200, 700));
      addTearDown(() => tester.binding.setSurfaceSize(null));
      final notifier = _GlossaryRelationshipNotifier();
      await tester.pumpWidget(
        wrapWithL10n(
          Scaffold(
            body: SizedBox(
              width: 1200,
              height: 700,
              child: GlossaryDetail(
                notifier: notifier,
                editable: true,
                reloadKey: const SaveInspection(
                  format: 'G1R',
                  path: r'C:\tmp\saves\G1R-001.sav',
                  size: 1,
                  sha1: 'test',
                  raw: {},
                ),
                theme: ThemeData.light(),
                segmentTextCatalogLoader: () async =>
                    const <String, List<String>>{},
                npcCatalogLoader: () async => const [
                  NpcGlossaryCatalogEntry(
                    id: 'OM_GRD_ASGHAN',
                    uniqueName: 'OM_GRD_ASGHAN_263',
                    documentClass:
                        '/Script/Angelscript.Document_Glossary_OM_GRD_ASGHAN',
                    camp: NpcGlossaryCamp.oldCamp,
                    segments: [
                      NpcGlossaryCatalogSegment(
                        id: 'Introduction',
                        segmentClass:
                            '/Script/Angelscript.DocumentSegment_Glossary_OM_GRD_ASGHAN_Introduction',
                        label: 'Introduction',
                        roles: {NpcGlossaryRole.portrait},
                      ),
                    ],
                  ),
                ],
              ),
            ),
          ),
        ),
      );
      await tester.pump();
      await tester.runAsync(
        () => Future<void>.delayed(const Duration(milliseconds: 250)),
      );
      await tester.pump();

      expect(find.text('ASGHAN'), findsOneWidget);
      await _selectGlossaryFilter(tester, 'Hostile');
      expect(find.text('ASGHAN'), findsNothing);

      // This simulates a later edit made in the already-mounted Attribute tab.
      notifier.setPendingNpcRelationship('Asghan-1', NpcRelationship.enemy);
      await tester.pump();
      expect(find.text('ASGHAN'), findsOneWidget);

      notifier.clearPendingNpcRelationship('Asghan-1');
      await tester.pump();
      expect(find.text('ASGHAN'), findsNothing);
    },
  );

  testWidgets(
    'Dead glossary filter follows the Dead entry rather than State.Dead',
    (tester) async {
      await tester.binding.setSurfaceSize(const Size(1200, 700));
      addTearDown(() => tester.binding.setSurfaceSize(null));
      final notifier = _DeadGlossaryNotifier();
      await tester.pumpWidget(
        wrapWithL10n(
          Scaffold(
            body: SizedBox(
              width: 1200,
              height: 700,
              child: GlossaryDetail(
                notifier: notifier,
                editable: true,
                reloadKey: const SaveInspection(
                  format: 'G1R',
                  path: r'C:\tmp\saves\G1R-001.sav',
                  size: 1,
                  sha1: 'test',
                  raw: {},
                ),
                theme: ThemeData.light(),
                segmentTextCatalogLoader: () async =>
                    const <String, List<String>>{},
                npcCatalogLoader: () async => const [
                  NpcGlossaryCatalogEntry(
                    id: 'OM_GRD_ASGHAN',
                    uniqueName: 'OM_GRD_ASGHAN_263',
                    documentClass:
                        '/Script/Angelscript.Document_Glossary_OM_GRD_ASGHAN',
                    camp: NpcGlossaryCamp.oldCamp,
                    segments: [
                      NpcGlossaryCatalogSegment(
                        id: 'Introduction',
                        segmentClass:
                            '/Script/Angelscript.DocumentSegment_Glossary_OM_GRD_ASGHAN_Introduction',
                        label: 'Introduction',
                        roles: {NpcGlossaryRole.portrait},
                      ),
                      NpcGlossaryCatalogSegment(
                        id: 'Dead',
                        segmentClass:
                            '/Script/Angelscript.DocumentSegment_Glossary_OM_GRD_ASGHAN_Dead',
                        label: 'Dead',
                        roles: {NpcGlossaryRole.dead},
                      ),
                    ],
                  ),
                ],
              ),
            ),
          ),
        ),
      );
      await tester.pump();
      await tester.runAsync(
        () => Future<void>.delayed(const Duration(milliseconds: 250)),
      );
      await tester.pump();

      // The actor is authoritatively dead, but its independent glossary Dead
      // segment is still locked, so the game's segment-name filter excludes it.
      await _selectGlossaryFilter(tester, 'Dead');
      expect(find.text('ASGHAN'), findsNothing);

      await _selectGlossaryFilter(tester, 'All');
      await tester.tap(find.text('ASGHAN'));
      await tester.pump();
      final deadSwitch = tester.widget<SwitchListTile>(
        find.widgetWithText(SwitchListTile, 'Dead'),
      );
      deadSwitch.onChanged!(true);
      await tester.pump();

      await _selectGlossaryFilter(tester, 'Dead');
      expect(find.text('ASGHAN'), findsOneWidget);
    },
  );

  testWidgets(
    'same-save glossary reload keeps the saved switch state while loading',
    (tester) async {
      await tester.binding.setSurfaceSize(const Size(1200, 700));
      addTearDown(() => tester.binding.setSurfaceSize(null));
      final notifier = _ReloadingGlossaryNotifier();

      Widget buildGlossary(String sha1) => wrapWithL10n(
        Scaffold(
          body: SizedBox(
            width: 1200,
            height: 700,
            child: GlossaryDetail(
              notifier: notifier,
              editable: true,
              reloadKey: SaveInspection(
                format: 'G1R',
                path: r'C:\tmp\saves\G1R-001.sav',
                size: 1,
                sha1: sha1,
                raw: const {},
              ),
              theme: ThemeData.light(),
              segmentTextCatalogLoader: () async =>
                  const <String, List<String>>{},
              npcCatalogLoader: () async => const [
                NpcGlossaryCatalogEntry(
                  id: 'OM_GRD_ASGHAN',
                  uniqueName: 'OM_GRD_ASGHAN_263',
                  documentClass:
                      '/Script/Angelscript.Document_Glossary_OM_GRD_ASGHAN',
                  camp: NpcGlossaryCamp.oldCamp,
                  segments: [
                    NpcGlossaryCatalogSegment(
                      id: 'Introduction',
                      segmentClass:
                          '/Script/Angelscript.DocumentSegment_Glossary_OM_GRD_ASGHAN_Introduction',
                      label: 'Introduction',
                      roles: {NpcGlossaryRole.portrait},
                    ),
                    NpcGlossaryCatalogSegment(
                      id: 'Dead',
                      segmentClass:
                          '/Script/Angelscript.DocumentSegment_Glossary_OM_GRD_ASGHAN_Dead',
                      label: 'Dead',
                      roles: {NpcGlossaryRole.dead},
                    ),
                  ],
                ),
              ],
            ),
          ),
        ),
      );

      await tester.pumpWidget(buildGlossary('before-save'));
      await tester.pump();
      await tester.runAsync(
        () => Future<void>.delayed(const Duration(milliseconds: 250)),
      );
      await tester.pumpAndSettle();

      await tester.tap(find.text('ASGHAN'));
      await tester.pump();
      var deadSwitch = tester.widget<SwitchListTile>(
        find.widgetWithText(SwitchListTile, 'Dead'),
      );
      deadSwitch.onChanged!(true);
      await tester.pump();
      expect(find.text('2 of 2 entries'), findsOneWidget);
      expect(find.text('Unsaved change'), findsOneWidget);

      // saveAllPending's fresh inspection clears the central pending registry
      // before GlossaryDetail's slower full-save reload has completed.
      notifier.clearAllPendingEdits();
      final reload = Completer<GlossaryPage>();
      notifier.nextGlossary = reload.future;
      await tester.pumpWidget(buildGlossary('after-save'));
      await tester.pump();

      // The old on-disk snapshot still says Dead=false, but the just-saved
      // optimistic value and the user's selection remain stable meanwhile.
      deadSwitch = tester.widget<SwitchListTile>(
        find.widgetWithText(SwitchListTile, 'Dead'),
      );
      expect(deadSwitch.value, isTrue);
      expect(deadSwitch.onChanged, isNull);
      expect(
        tester.widget<ExcludeFocus>(find.byType(ExcludeFocus).first).excluding,
        isTrue,
      );
      expect(find.text('2 of 2 entries'), findsOneWidget);
      expect(find.text('Unsaved change'), findsNothing);

      reload.complete(_ReloadingGlossaryNotifier.savedGlossary);
      await tester.pumpAndSettle();

      deadSwitch = tester.widget<SwitchListTile>(
        find.widgetWithText(SwitchListTile, 'Dead'),
      );
      expect(deadSwitch.value, isTrue);
      expect(deadSwitch.onChanged, isNotNull);
      expect(find.text('2 of 2 entries'), findsOneWidget);
      expect(find.text('Unsaved change'), findsNothing);
    },
  );

  testWidgets(
    'glossary add action is disabled when hidden entries are not writable',
    (tester) async {
      await tester.binding.setSurfaceSize(const Size(1200, 700));
      addTearDown(() => tester.binding.setSurfaceSize(null));
      final notifier = _ReadOnlyGlossaryNotifier();
      await tester.pumpWidget(
        wrapWithL10n(
          Scaffold(
            body: SizedBox(
              width: 1200,
              height: 700,
              child: GlossaryDetail(
                notifier: notifier,
                editable: true,
                reloadKey: const SaveInspection(
                  format: 'G1R',
                  path: r'C:\tmp\saves\G1R-001.sav',
                  size: 1,
                  sha1: 'test',
                  raw: {},
                ),
                theme: ThemeData.light(),
                segmentTextCatalogLoader: () async =>
                    const <String, List<String>>{},
                npcCatalogLoader: () async => const [
                  NpcGlossaryCatalogEntry(
                    id: 'OM_GRD_ASGHAN',
                    uniqueName: 'OM_GRD_ASGHAN_263',
                    documentClass:
                        '/Script/Angelscript.Document_Glossary_OM_GRD_ASGHAN',
                    camp: NpcGlossaryCamp.oldCamp,
                    segments: [
                      NpcGlossaryCatalogSegment(
                        id: 'Introduction',
                        segmentClass:
                            '/Script/Angelscript.DocumentSegment_Glossary_OM_GRD_ASGHAN_Introduction',
                        label: 'Introduction',
                        roles: {NpcGlossaryRole.portrait},
                      ),
                    ],
                  ),
                ],
              ),
            ),
          ),
        ),
      );
      await tester.pump();
      await tester.runAsync(
        () => Future<void>.delayed(const Duration(milliseconds: 250)),
      );
      await tester.pump();

      final addButton = tester.widget<IconButton>(
        find.widgetWithIcon(IconButton, Icons.add_circle_outline),
      );
      expect(addButton.onPressed, isNull);
    },
  );

  testWidgets('glossary previews localized segment text and opens the full text', (
    tester,
  ) async {
    final semantics = tester.ensureSemantics();
    await tester.binding.setSurfaceSize(const Size(1200, 700));
    addTearDown(() => tester.binding.setSurfaceSize(null));
    const segmentClass =
        '/Script/Angelscript.DocumentSegment_Glossary_OM_GRD_ASGHAN_Introduction';
    const textId = 'TEXT_TEST_GLOSSARY_ASGHAN';
    const fullText =
        'Asghan guards the old mine and knows why the sealed tunnel must remain closed.';
    final notifier = _GlossaryRelationshipNotifier();

    await tester.pumpWidget(
      ProviderScope(
        overrides: [
          locCatalogProvider.overrideWith(
            (ref) async => const {
              'text_test_glossary_asghan': {'english': fullText},
            },
          ),
        ],
        child: wrapWithL10n(
          Scaffold(
            body: SizedBox(
              width: 1200,
              height: 700,
              child: GlossaryDetail(
                notifier: notifier,
                editable: true,
                reloadKey: const SaveInspection(
                  format: 'G1R',
                  path: r'C:\tmp\saves\G1R-001.sav',
                  size: 1,
                  sha1: 'test',
                  raw: {},
                ),
                theme: ThemeData.light(),
                segmentTextCatalogLoader: () async => const {
                  '/script/angelscript.documentsegment_glossary_om_grd_asghan_introduction':
                      [textId],
                },
                npcCatalogLoader: () async => const [
                  NpcGlossaryCatalogEntry(
                    id: 'OM_GRD_ASGHAN',
                    uniqueName: 'OM_GRD_ASGHAN_263',
                    documentClass:
                        '/Script/Angelscript.Document_Glossary_OM_GRD_ASGHAN',
                    camp: NpcGlossaryCamp.oldCamp,
                    segments: [
                      NpcGlossaryCatalogSegment(
                        id: 'Introduction',
                        segmentClass: segmentClass,
                        label: 'Introduction',
                        roles: {NpcGlossaryRole.portrait},
                      ),
                    ],
                  ),
                ],
              ),
            ),
          ),
        ),
      ),
    );
    await tester.pump();
    await tester.runAsync(
      () => Future<void>.delayed(const Duration(milliseconds: 250)),
    );
    await tester.pumpAndSettle();

    final oldCampTile = find.widgetWithText(ListTile, 'Old Camp');
    expect(tester.getSize(oldCampTile).width, 220);
    await tester.tap(find.text('ASGHAN'));
    await tester.pumpAndSettle();

    final preview = tester.widget<Text>(find.text(fullText));
    expect(preview.maxLines, 2);
    expect(preview.overflow, TextOverflow.ellipsis);
    expect(find.text('Introduction / portrait'), findsNothing);

    await tester.tap(find.widgetWithIcon(IconButton, Icons.open_in_full));
    await tester.pumpAndSettle();
    expect(find.byType(AlertDialog), findsOneWidget);
    expect(find.byType(SelectionArea), findsOneWidget);
    expect(find.text(fullText), findsNWidgets(2));
    semantics.dispose();
  });

  testWidgets('narrow glossary drills from the entry list into its detail', (
    tester,
  ) async {
    await tester.binding.setSurfaceSize(const Size(306, 700));
    addTearDown(() => tester.binding.setSurfaceSize(null));
    final notifier = _GlossaryRelationshipNotifier();
    await tester.pumpWidget(
      wrapWithL10n(
        Scaffold(
          body: SizedBox(
            width: 306,
            height: 700,
            child: GlossaryDetail(
              notifier: notifier,
              editable: true,
              reloadKey: const SaveInspection(
                format: 'G1R',
                path: r'C:\tmp\saves\G1R-001.sav',
                size: 1,
                sha1: 'test',
                raw: {},
              ),
              theme: ThemeData.light(),
              segmentTextCatalogLoader: () async =>
                  const <String, List<String>>{},
              npcCatalogLoader: () async => const [
                NpcGlossaryCatalogEntry(
                  id: 'OM_GRD_ASGHAN',
                  uniqueName: 'OM_GRD_ASGHAN_263',
                  documentClass:
                      '/Script/Angelscript.Document_Glossary_OM_GRD_ASGHAN',
                  camp: NpcGlossaryCamp.oldCamp,
                  segments: [
                    NpcGlossaryCatalogSegment(
                      id: 'Introduction',
                      segmentClass:
                          '/Script/Angelscript.DocumentSegment_Glossary_OM_GRD_ASGHAN_Introduction',
                      label: 'Introduction',
                      roles: {NpcGlossaryRole.portrait},
                    ),
                  ],
                ),
              ],
            ),
          ),
        ),
      ),
    );
    await tester.pump();
    await tester.runAsync(
      () => Future<void>.delayed(const Duration(milliseconds: 250)),
    );
    await tester.pumpAndSettle();

    expect(find.byKey(const Key('glossary-category-dropdown')), findsOneWidget);
    expect(find.byKey(const Key('glossary-detail-back')), findsNothing);
    await tester.tap(find.text('ASGHAN'));
    await tester.pumpAndSettle();

    expect(find.byKey(const Key('glossary-category-dropdown')), findsNothing);
    expect(find.byKey(const Key('glossary-detail-back')), findsOneWidget);
    expect(find.byType(SwitchListTile), findsOneWidget);
    expect(tester.takeException(), isNull);

    await tester.tap(find.byKey(const Key('glossary-detail-back')));
    await tester.pumpAndSettle();
    expect(find.byKey(const Key('glossary-category-dropdown')), findsOneWidget);
    expect(find.byKey(const Key('glossary-detail-back')), findsNothing);
  });

  testWidgets('compact glossary uses category and filter dropdowns', (
    tester,
  ) async {
    await tester.binding.setSurfaceSize(const Size(720, 700));
    addTearDown(() => tester.binding.setSurfaceSize(null));
    final notifier = _GlossaryRelationshipNotifier();
    await tester.pumpWidget(
      wrapWithL10n(
        Scaffold(
          body: SizedBox(
            width: 720,
            height: 700,
            child: GlossaryDetail(
              notifier: notifier,
              editable: true,
              reloadKey: const SaveInspection(
                format: 'G1R',
                path: r'C:\tmp\saves\G1R-001.sav',
                size: 1,
                sha1: 'test',
                raw: {},
              ),
              theme: ThemeData.light(),
              segmentTextCatalogLoader: () async =>
                  const <String, List<String>>{},
              npcCatalogLoader: () async => const [
                NpcGlossaryCatalogEntry(
                  id: 'OM_GRD_ASGHAN',
                  uniqueName: 'OM_GRD_ASGHAN_263',
                  documentClass:
                      '/Script/Angelscript.Document_Glossary_OM_GRD_ASGHAN',
                  camp: NpcGlossaryCamp.oldCamp,
                  segments: [
                    NpcGlossaryCatalogSegment(
                      id: 'Introduction',
                      segmentClass:
                          '/Script/Angelscript.DocumentSegment_Glossary_OM_GRD_ASGHAN_Introduction',
                      label: 'Introduction',
                      roles: {NpcGlossaryRole.portrait},
                    ),
                  ],
                ),
              ],
            ),
          ),
        ),
      ),
    );
    await tester.pump();
    await tester.runAsync(
      () => Future<void>.delayed(const Duration(milliseconds: 250)),
    );
    await tester.pumpAndSettle();

    expect(find.byKey(const Key('glossary-category-dropdown')), findsOneWidget);
    expect(
      find.byKey(const Key('glossary-npc-filter-dropdown')),
      findsOneWidget,
    );
    expect(tester.takeException(), isNull);
  });
}

Future<void> _selectGlossaryFilter(WidgetTester tester, String label) async {
  await tester.tap(find.byKey(const Key('glossary-npc-filter-dropdown')));
  await tester.pumpAndSettle();
  await tester.tap(find.text(label).last);
  await tester.pumpAndSettle();
}

SaveInspection _relationshipInspection({required bool supported}) =>
    SaveInspection(
      format: 'G1R',
      path: r'C:\tmp\saves\G1R-001.sav',
      size: 1,
      sha1: supported ? 'supported' : 'unsupported',
      raw: const {},
      privateNpc: PrivateNpcSummary(
        writable: supported ? const ['private.npc.setRelationship'] : const [],
      ),
    );

class _RelationshipEditorNotifier extends EditorNotifier {
  _RelationshipEditorNotifier(this.actor)
    : super(_NoopCore(), saveDir: r'C:\tmp\saves');

  final NpcActor actor;

  @override
  Future<NpcActorsPage> loadAllNpcActors({
    String query = '',
    int offset = 0,
    int limit = 100,
  }) async => NpcActorsPage(total: 1, npcs: [actor]);
}

class _RelationshipEventsNotifier extends _RelationshipEditorNotifier {
  _RelationshipEventsNotifier(super.actor);

  @override
  Future<MemoryEventsPage> loadMemoryEvents(
    String character, {
    String query = '',
    int offset = 0,
    int limit = 100,
  }) async => MemoryEventsPage(
    character: character,
    total: 1,
    offset: offset,
    limit: limit,
    events: const [
      MemoryEvent(index: 0, tags: ['Event.Test']),
    ],
  );
}

class _GlossaryRelationshipNotifier extends EditorNotifier {
  _GlossaryRelationshipNotifier()
    : super(_NoopCore(), saveDir: r'C:\tmp\saves');

  @override
  Future<GlossaryPage> loadGlossary() async => const GlossaryPage(
    writable: ['private.glossary.setSegment'],
    segmentUnlocks: [
      GlossarySegmentUnlock(
        documentClass: '/Script/Angelscript.Document_Glossary_OM_GRD_ASGHAN',
        segmentClass:
            '/Script/Angelscript.DocumentSegment_Glossary_OM_GRD_ASGHAN_Introduction',
        unlockedEventIndices: [1],
      ),
    ],
  );

  @override
  Future<CharacterIndexPage> loadAllCharacters() async =>
      const CharacterIndexPage(
        total: 1,
        characters: [
          CharacterRow(
            globalId: 'Asghan-1',
            uniqueName: 'OM_GRD_ASGHAN_263',
            isDead: false,
            hasInventory: true,
            hasKnowledge: true,
            hasEvents: true,
          ),
        ],
      );

  @override
  Future<NpcActorsPage> loadAllNpcActors({
    String query = '',
    int offset = 0,
    int limit = 100,
  }) async => const NpcActorsPage(
    total: 1,
    npcs: [NpcActor(id: 'Asghan-1', isDead: false)],
  );
}

class _ReadOnlyGlossaryNotifier extends _GlossaryRelationshipNotifier {
  @override
  Future<GlossaryPage> loadGlossary() async => const GlossaryPage();
}

class _DeadGlossaryNotifier extends _GlossaryRelationshipNotifier {
  @override
  Future<CharacterIndexPage> loadAllCharacters() async =>
      const CharacterIndexPage(
        total: 1,
        characters: [
          CharacterRow(
            globalId: 'Asghan-1',
            uniqueName: 'OM_GRD_ASGHAN_263',
            isDead: true,
            hasInventory: true,
            hasKnowledge: true,
            hasEvents: true,
          ),
        ],
      );

  @override
  Future<NpcActorsPage> loadAllNpcActors({
    String query = '',
    int offset = 0,
    int limit = 100,
  }) async => const NpcActorsPage(
    total: 1,
    npcs: [NpcActor(id: 'Asghan-1', isDead: true)],
  );
}

class _ReloadingGlossaryNotifier extends _DeadGlossaryNotifier {
  Future<GlossaryPage>? nextGlossary;

  static const savedGlossary = GlossaryPage(
    writable: ['private.glossary.setSegment'],
    segmentUnlocks: [
      GlossarySegmentUnlock(
        documentClass: '/Script/Angelscript.Document_Glossary_OM_GRD_ASGHAN',
        segmentClass:
            '/Script/Angelscript.DocumentSegment_Glossary_OM_GRD_ASGHAN_Introduction',
        unlockedEventIndices: [1],
      ),
      GlossarySegmentUnlock(
        documentClass: '/Script/Angelscript.Document_Glossary_OM_GRD_ASGHAN',
        segmentClass:
            '/Script/Angelscript.DocumentSegment_Glossary_OM_GRD_ASGHAN_Dead',
        unlockedEventIndices: [2],
      ),
    ],
  );

  @override
  Future<GlossaryPage> loadGlossary() => nextGlossary ?? super.loadGlossary();
}

class _NoopCore implements GoresaveCoreService {
  @override
  String get description => 'relationship-test';

  @override
  bool get isAvailable => true;

  @override
  Future<Map<String, Object?>> execute(
    String command, {
    Map<String, Object?> payload = const {},
  }) async {
    return switch (command) {
      'scan_save_dir' => {
        'ok': true,
        'data': {
          'saveRoot': payload['path'],
          'saves': <Object?>[],
          'profiles': <Object?>[],
        },
      },
      'check_codec' => {
        'ok': true,
        'data': {'canDecompress': true, 'canCompress': true},
      },
      _ => {'ok': true, 'data': <String, Object?>{}},
    };
  }
}
