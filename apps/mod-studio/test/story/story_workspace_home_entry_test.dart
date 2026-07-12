import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/core/core_service.dart';
import 'package:gore_mod/core/providers.dart';
import 'package:gore_mod/gore_mod_app.dart';

void main() {
  testWidgets('Project menu exposes explicit Story draft create/open entries', (
    tester,
  ) async {
    tester.view.physicalSize = const Size(1600, 900);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.resetPhysicalSize);
    addTearDown(tester.view.resetDevicePixelRatio);

    final core = FakeGoreCoreFfiService(
      responses: const <String, Map<String, Object?>>{
        'loc_status': <String, Object?>{'ok': true, 'present': true},
        'find_game': <String, Object?>{'ok': true, 'found': false},
      },
    );
    final container = ProviderContainer(
      overrides: [coreServiceProvider.overrideWithValue(core)],
    );
    addTearDown(container.dispose);
    await tester.pumpWidget(
      UncontrolledProviderScope(
        container: container,
        child: const GoreModApp(),
      ),
    );
    await tester.pump();
    await tester.pump(const Duration(milliseconds: 100));

    await tester.tap(find.byTooltip('Project'));
    await tester.pump();
    await tester.pump(const Duration(milliseconds: 200));

    expect(
      find.byKey(const Key('project-create-story-workspace')),
      findsOneWidget,
    );
    expect(
      find.byKey(const Key('project-open-story-workspace')),
      findsOneWidget,
    );
    expect(find.text('Create Story workspace (drafts)...'), findsOneWidget);
    expect(find.text('Open Story workspace (drafts)...'), findsOneWidget);
  });
}
