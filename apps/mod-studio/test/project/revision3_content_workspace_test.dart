import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/project/revision3_content_workspace.dart';
import 'package:gore_mod/project/revision3_project_workspace.dart';

void main() {
  testWidgets('keeps DataAssets lazy and deep-links through parent location', (
    tester,
  ) async {
    await tester.pumpWidget(const _Harness());

    await tester.tap(find.text('OPEN CONTENT'));
    await tester.pump();

    expect(find.text('LIBRARY BODY'), findsOneWidget);
    expect(find.text('DATAASSET BODY'), findsNothing);

    await tester.tap(find.text('Verified edits'));
    await tester.pump();

    expect(find.text('LIBRARY BODY'), findsNothing);
    expect(find.text('DATAASSET BODY'), findsOneWidget);
    expect(
      find.byKey(
        const Key('revision3-content-workspace-page-data-assets'),
        skipOffstage: false,
      ),
      findsOneWidget,
    );

    await tester.tap(find.text('My mod'));
    await tester.pump();

    expect(find.text('LIBRARY BODY'), findsOneWidget);
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
              location: location,
              libraryLabel: 'My mod',
              dataAssetsLabel: 'Verified edits',
              library: const Text('LIBRARY BODY'),
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
