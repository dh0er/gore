import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/project/revision3_content_workspace.dart';
import 'package:gore_mod/project/revision3_project_workspace.dart';

void main() {
  testWidgets('keeps secondary tools lazy and retains them for one project', (
    tester,
  ) async {
    await tester.pumpWidget(const _Harness());

    await tester.tap(find.text('OPEN CONTENT'));
    await tester.pump();

    expect(find.text('LIBRARY BODY'), findsOneWidget);
    expect(find.text('ITEMS BODY'), findsNothing);
    expect(find.text('TEXTURES BODY'), findsNothing);
    expect(find.text('DATAASSET BODY'), findsNothing);

    await tester.tap(find.text('Items'));
    await tester.pump();

    expect(find.text('LIBRARY BODY'), findsNothing);
    expect(find.text('ITEMS BODY'), findsOneWidget);
    expect(find.text('TEXTURES BODY'), findsNothing);
    expect(find.text('DATAASSET BODY'), findsNothing);

    await tester.tap(find.text('Textures'));
    await tester.pump();

    expect(find.text('LIBRARY BODY'), findsNothing);
    expect(find.text('ITEMS BODY'), findsNothing);
    expect(find.text('TEXTURES BODY'), findsOneWidget);
    expect(find.text('DATAASSET BODY'), findsNothing);

    await tester.ensureVisible(find.text('Verified edits'));
    await tester.tap(find.text('Verified edits'));
    await tester.pump();

    expect(find.text('LIBRARY BODY'), findsNothing);
    expect(find.text('ITEMS BODY'), findsNothing);
    expect(find.text('DATAASSET BODY'), findsOneWidget);
    expect(
      find.byKey(
        const Key('revision3-content-workspace-page-data-assets'),
        skipOffstage: false,
      ),
      findsOneWidget,
    );

    await tester.ensureVisible(find.text('My mod'));
    await tester.tap(find.text('My mod'));
    await tester.pump();

    expect(find.text('LIBRARY BODY'), findsOneWidget);
    expect(find.text('ITEMS BODY', skipOffstage: false), findsOneWidget);
    expect(find.text('TEXTURES BODY', skipOffstage: false), findsOneWidget);
    expect(find.text('DATAASSET BODY', skipOffstage: false), findsOneWidget);
  });

  testWidgets('accepts an explicit DataAssets route before first build', (
    tester,
  ) async {
    await tester.pumpWidget(const _Harness(initialSecondary: 'data-assets'));

    await tester.tap(find.text('OPEN CONTENT'));
    await tester.pump();

    expect(find.text('DATAASSET BODY'), findsOneWidget);
    expect(find.text('LIBRARY BODY'), findsNothing);
    expect(find.text('LIBRARY BODY', skipOffstage: false), findsNothing);
    expect(find.text('ITEMS BODY', skipOffstage: false), findsNothing);
    expect(find.text('TEXTURES BODY', skipOffstage: false), findsNothing);
  });

  testWidgets('accepts an explicit Items route before first build', (
    tester,
  ) async {
    await tester.pumpWidget(const _Harness(initialSecondary: 'items'));

    await tester.tap(find.text('OPEN CONTENT'));
    await tester.pump();

    expect(find.text('ITEMS BODY'), findsOneWidget);
    expect(find.text('LIBRARY BODY', skipOffstage: false), findsNothing);
    expect(find.text('TEXTURES BODY', skipOffstage: false), findsNothing);
    expect(find.text('DATAASSET BODY', skipOffstage: false), findsNothing);
  });

  testWidgets('accepts an explicit Textures route before first build', (
    tester,
  ) async {
    await tester.pumpWidget(const _Harness(initialSecondary: 'textures'));

    await tester.tap(find.text('OPEN CONTENT'));
    await tester.pump();

    expect(find.text('TEXTURES BODY'), findsOneWidget);
    expect(find.text('LIBRARY BODY', skipOffstage: false), findsNothing);
    expect(find.text('ITEMS BODY', skipOffstage: false), findsNothing);
    expect(find.text('DATAASSET BODY', skipOffstage: false), findsNothing);
  });

  testWidgets('horizontal secondary navigation survives narrow width', (
    tester,
  ) async {
    tester.view.physicalSize = const Size(280, 300);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.resetPhysicalSize);
    addTearDown(tester.view.resetDevicePixelRatio);

    await tester.pumpWidget(const _Harness());
    await tester.tap(find.text('OPEN CONTENT'));
    await tester.pump();

    expect(tester.takeException(), isNull);
    expect(
      find.byKey(const Key('revision3-content-workspace-navigation-scroll')),
      findsOneWidget,
    );
  });

  testWidgets(
    'retains state for the same project and resets on identity change',
    (tester) async {
      await tester.pumpWidget(const _IdentityHarness());

      await tester.tap(find.byKey(const Key('ITEM COUNTER')));
      await tester.pump();
      expect(find.text('ITEM COUNT 1'), findsOneWidget);

      await tester.tap(find.byKey(const Key('REBUILD SAME')));
      await tester.pump();
      expect(find.text('ITEM COUNT 1'), findsOneWidget);

      await tester.tap(find.byKey(const Key('CHANGE PROJECT')));
      await tester.pump();
      expect(find.text('ITEM COUNT 0'), findsOneWidget);
    },
  );
}

class _Harness extends StatelessWidget {
  const _Harness({this.initialSecondary});

  final String? initialSecondary;

  @override
  Widget build(BuildContext context) => MaterialApp(
    home: Scaffold(
      body: Revision3ProjectWorkspace(
        projectIdentity: 'project',
        destinations: [
          Revision3ProjectWorkspaceDestination(
            section: Revision3ProjectWorkspaceSection.home,
            label: 'Home',
            icon: Icons.home_outlined,
            selectedIcon: Icons.home,
            pageBuilder: (context, _) => Center(
              child: TextButton(
                onPressed: () => Revision3ProjectWorkspace.navigate(
                  context,
                  Revision3ProjectWorkspaceLocation(
                    Revision3ProjectWorkspaceSection.content,
                    secondary: initialSecondary,
                  ),
                ),
                child: const Text('OPEN CONTENT'),
              ),
            ),
          ),
          Revision3ProjectWorkspaceDestination(
            section: Revision3ProjectWorkspaceSection.content,
            label: 'Content',
            icon: Icons.account_tree_outlined,
            selectedIcon: Icons.account_tree,
            pageBuilder: (context, location) => Revision3ContentWorkspace(
              projectIdentity: 'project',
              location: location,
              libraryLabel: 'My mod',
              itemsLabel: 'Items',
              texturesLabel: 'Textures',
              dataAssetsLabel: 'Verified edits',
              library: const Text('LIBRARY BODY'),
              items: const Text('ITEMS BODY'),
              textures: const Text('TEXTURES BODY'),
              dataAssets: const Text('DATAASSET BODY'),
            ),
          ),
          Revision3ProjectWorkspaceDestination(
            section: Revision3ProjectWorkspaceSection.story,
            label: 'Story',
            icon: Icons.menu_book_outlined,
            selectedIcon: Icons.menu_book,
            pageBuilder: (_, _) => const SizedBox(),
          ),
          Revision3ProjectWorkspaceDestination(
            section: Revision3ProjectWorkspaceSection.world,
            label: 'World',
            icon: Icons.public_outlined,
            selectedIcon: Icons.public,
            pageBuilder: (_, _) => const SizedBox(),
          ),
          Revision3ProjectWorkspaceDestination(
            section: Revision3ProjectWorkspaceSection.localizationVoice,
            label: 'Localization & Voice',
            icon: Icons.record_voice_over_outlined,
            selectedIcon: Icons.record_voice_over,
            pageBuilder: (_, _) => const SizedBox(),
          ),
          Revision3ProjectWorkspaceDestination(
            section: Revision3ProjectWorkspaceSection.validateTest,
            label: 'Validate & Test',
            icon: Icons.fact_check_outlined,
            selectedIcon: Icons.fact_check,
            pageBuilder: (_, _) => const SizedBox(),
          ),
          Revision3ProjectWorkspaceDestination(
            section: Revision3ProjectWorkspaceSection.buildRelease,
            label: 'Build & Release',
            icon: Icons.inventory_2_outlined,
            selectedIcon: Icons.inventory_2,
            pageBuilder: (_, _) => const SizedBox(),
          ),
          Revision3ProjectWorkspaceDestination(
            section: Revision3ProjectWorkspaceSection.history,
            label: 'History',
            icon: Icons.history_outlined,
            selectedIcon: Icons.history,
            pageBuilder: (_, _) => const SizedBox(),
          ),
          Revision3ProjectWorkspaceDestination(
            section: Revision3ProjectWorkspaceSection.settingsExpert,
            label: 'Settings / Expert',
            icon: Icons.settings_outlined,
            selectedIcon: Icons.settings,
            pageBuilder: (_, _) => const SizedBox(),
          ),
        ],
      ),
    ),
  );
}

class _IdentityHarness extends StatefulWidget {
  const _IdentityHarness();

  @override
  State<_IdentityHarness> createState() => _IdentityHarnessState();
}

class _IdentityHarnessState extends State<_IdentityHarness> {
  String _identity = 'project-a';

  @override
  Widget build(BuildContext context) => MaterialApp(
    home: Scaffold(
      appBar: AppBar(
        actions: [
          TextButton(
            key: const Key('REBUILD SAME'),
            onPressed: () => setState(() {}),
            child: const Text('REBUILD'),
          ),
          TextButton(
            key: const Key('CHANGE PROJECT'),
            onPressed: () => setState(() => _identity = 'project-b'),
            child: const Text('CHANGE'),
          ),
        ],
      ),
      body: Revision3ContentWorkspace(
        projectIdentity: _identity,
        location: const Revision3ProjectWorkspaceLocation(
          Revision3ProjectWorkspaceSection.content,
          secondary: 'items',
        ),
        libraryLabel: 'My mod',
        itemsLabel: 'Items',
        texturesLabel: 'Textures',
        dataAssetsLabel: 'Verified edits',
        library: const Text('LIBRARY BODY'),
        items: const _CounterBody(),
        textures: const Text('TEXTURES BODY'),
        dataAssets: const Text('DATAASSET BODY'),
      ),
    ),
  );
}

class _CounterBody extends StatefulWidget {
  const _CounterBody();

  @override
  State<_CounterBody> createState() => _CounterBodyState();
}

class _CounterBodyState extends State<_CounterBody> {
  int _count = 0;

  @override
  Widget build(BuildContext context) => TextButton(
    key: const Key('ITEM COUNTER'),
    onPressed: () => setState(() => _count++),
    child: Text('ITEM COUNT $_count'),
  );
}
