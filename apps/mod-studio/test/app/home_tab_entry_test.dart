import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/app/domain/asset_entry_tracker.dart';
import 'package:gore_mod/app/ui/tab_entry_listener.dart';
import 'package:gore_mod/core/mod_ffi.dart';
import 'package:gore_mod/editor/ui/changes_tab.dart';
import 'package:gore_mod/home_page.dart';
import 'package:gore_mod/scripts/domain/script_modules_provider.dart';

/// [HomePage]'s legacy-tab order after removing the obsolete texture route:
/// Items(0), Dialogs(1), Audio(2), Scripts(3), Changes(4), Settings(5),
/// DataAsset Lab(6).
const _itemsTab = 0;
const _scriptsTab = 3;
const _changesTab = 4;

/// Exposes [ChangeNotifier.notifyListeners] so tests can simulate the
/// repeated same-settled-index notifications a TabBarView swipe produces.
class _RefiringTabController extends TabController {
  _RefiringTabController({required super.length, required super.vsync});

  void refire() => notifyListeners();
}

class _Harness {
  _Harness(this.controller);

  final _RefiringTabController controller;
  late final ProviderContainer container;
  int scriptBuilds = 0;
}

Future<_Harness> _pumpHarness(WidgetTester tester) async {
  final harness = _Harness(
    _RefiringTabController(length: 7, vsync: const TestVSync()),
  );
  addTearDown(harness.controller.dispose);

  await tester.pumpWidget(
    ProviderScope(
      overrides: [
        scriptModulesProvider.overrideWith((ref) {
          harness.scriptBuilds++;
          return Future.value(const <ScriptModuleInfo>[]);
        }),
      ],
      child: MaterialApp(
        home: Consumer(
          builder: (context, ref, _) => TabEntryListener(
            controller: harness.controller,
            onTabEntered: (index) => handleMainTabEntered(ref, index),
            child: const SizedBox.shrink(),
          ),
        ),
      ),
    ),
  );

  harness.container = ProviderScope.containerOf(
    tester.element(find.byType(TabEntryListener)),
  );
  // Stand in for the kept-alive standalone/Changes script views.
  harness.container.listen(scriptModulesProvider, (_, _) {});
  await tester.pump();
  expect(harness.scriptBuilds, 1);
  return harness;
}

void main() {
  testWidgets(
    'Changes main-tab entry refreshes only its embedded Scripts provider',
    (tester) async {
      final harness = await _pumpHarness(tester);
      final tracker = harness.container.read(assetEntryTrackerProvider);

      Future<void> goTo(int index) async {
        harness.controller.index = index;
        await tester.pumpAndSettle();
      }

      // Changes defaults to All, so entering and re-entering it does nothing.
      await goTo(_changesTab);
      await goTo(_itemsTab);
      await goTo(_changesTab);
      expect(harness.scriptBuilds, 1);

      // Simulate opening the embedded Scripts section. Its first display is
      // already fresh and therefore does not invalidate.
      harness.container.read(changesAssetSectionProvider.notifier).state =
          ChangesAssetSection.scripts;
      expect(tracker.shouldInvalidateOnEntry(AssetKind.scriptModules), isFalse);

      // Re-entering Changes while parked on Scripts refetches exactly once.
      await goTo(_itemsTab);
      await goTo(_changesTab);
      expect(harness.scriptBuilds, 2);

      // The standalone Scripts route shares the same freshness tracker.
      await goTo(_scriptsTab);
      expect(harness.scriptBuilds, 3);
      await goTo(_itemsTab);
      await goTo(_scriptsTab);
      expect(harness.scriptBuilds, 4);
    },
  );

  testWidgets(
    'first Scripts display avoids a double-fetch and settles are debounced',
    (tester) async {
      final harness = await _pumpHarness(tester);

      harness.controller.index = _scriptsTab;
      await tester.pumpAndSettle();
      expect(harness.scriptBuilds, 1);

      harness.controller.refire();
      harness.controller.refire();
      await tester.pumpAndSettle();
      expect(harness.scriptBuilds, 1);

      harness.controller.index = _itemsTab;
      await tester.pumpAndSettle();
      harness.controller.index = _scriptsTab;
      await tester.pumpAndSettle();
      expect(harness.scriptBuilds, 2);

      harness.controller.refire();
      await tester.pumpAndSettle();
      expect(harness.scriptBuilds, 2);
    },
  );
}
