import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/app/domain/asset_entry_tracker.dart';
import 'package:gore_mod/app/ui/tab_entry_listener.dart';
import 'package:gore_mod/core/mod_ffi.dart';
import 'package:gore_mod/editor/ui/changes_tab.dart';
import 'package:gore_mod/home_page.dart';
import 'package:gore_mod/scripts/domain/script_modules_provider.dart';
import 'package:gore_mod/textures/domain/texture_index_provider.dart';

/// [HomePage]'s main-tab order: Items(0), Dialogs(1), Audio(2), Textures(3),
/// Scripts(4), Changes(5), Settings(6).
const _itemsTab = 0;
const _texturesTab = 3;
const _scriptsTab = 4;
const _changesTab = 5;

/// Exposes [ChangeNotifier.notifyListeners] so tests can simulate the
/// repeated same-settled-index notifications a TabBarView swipe gesture
/// produces (indexIsChanging stays false throughout).
class _RefiringTabController extends TabController {
  _RefiringTabController({required super.length, required super.vsync});

  void refire() => notifyListeners();
}

/// Drives the REAL [handleMainTabEntered] mapping (home_page.dart) through a
/// real [TabController] + [TabEntryListener], without pumping the FFI-heavy
/// [HomePage] itself.
class _Harness {
  _Harness(this.controller);

  final _RefiringTabController controller;
  late final ProviderContainer container;
  int textureBuilds = 0;
  int scriptBuilds = 0;
}

Future<_Harness> _pumpHarness(WidgetTester tester) async {
  final h = _Harness(
    _RefiringTabController(length: 7, vsync: const TestVSync()),
  );
  addTearDown(h.controller.dispose);

  await tester.pumpWidget(
    ProviderScope(
      overrides: [
        textureIndexProvider.overrideWith((ref) {
          h.textureBuilds++;
          return Future.value(const <String, String>{});
        }),
        scriptModulesProvider.overrideWith((ref) {
          h.scriptBuilds++;
          return Future.value(const <ScriptModuleInfo>[]);
        }),
      ],
      child: MaterialApp(
        home: Consumer(
          builder: (context, ref, _) => TabEntryListener(
            controller: h.controller,
            // The real home_page mapping under test — not a replica.
            onTabEntered: (index) => handleMainTabEntered(ref, index),
            child: const SizedBox.shrink(),
          ),
        ),
      ),
    ),
  );

  h.container = ProviderScope.containerOf(
    tester.element(find.byType(TabEntryListener)),
  );
  // Stand-in for the keep-alive main tabs, which watch both providers in
  // the real app: keeps the autoDispose providers alive across tab
  // switches, so a refetch can only come from an explicit invalidate —
  // not from autoDispose disposal/re-creation.
  h.container.listen(textureIndexProvider, (_, _) {});
  h.container.listen(scriptModulesProvider, (_, _) {});
  await tester.pump();
  expect(h.textureBuilds, 1);
  expect(h.scriptBuilds, 1);
  return h;
}

void main() {
  testWidgets(
      'Changes main-tab entry refreshes exactly the embedded asset '
      "section's provider", (tester) async {
    final h = await _pumpHarness(tester);
    final tracker = h.container.read(assetEntryTrackerProvider);

    Future<void> goTo(int index) async {
      h.controller.index = index;
      await tester.pumpAndSettle();
    }

    // First entry into Changes while it shows a non-asset section (provider
    // default null = initial "All"): nothing to refresh.
    await goTo(_changesTab);
    expect(h.textureBuilds, 1);
    expect(h.scriptBuilds, 1);

    // Re-entry on a non-asset section: still nothing.
    await goTo(_itemsTab);
    await goTo(_changesTab);
    expect(h.textureBuilds, 1);
    expect(h.scriptBuilds, 1);

    // The user opens the Textures section inside Changes. The real
    // ChangesTab publishes the section AND consults the tracker in
    // _selectSection (first display of the kind anywhere: builds fresh, no
    // invalidate) — simulate both halves, since ChangesTab isn't pumped.
    h.container.read(changesAssetSectionProvider.notifier).state =
        ChangesAssetSection.textures;
    expect(tracker.shouldInvalidateOnEntry(AssetKind.textureIndex), isFalse);
    expect(h.textureBuilds, 1);

    // Re-entering Changes parked on Textures: exactly the texture index
    // refetches.
    await goTo(_itemsTab);
    await goTo(_changesTab);
    expect(h.textureBuilds, 2);
    expect(h.scriptBuilds, 1);

    // Same for Scripts.
    h.container.read(changesAssetSectionProvider.notifier).state =
        ChangesAssetSection.scripts;
    expect(tracker.shouldInvalidateOnEntry(AssetKind.scriptModules), isFalse);
    await goTo(_itemsTab);
    await goTo(_changesTab);
    expect(h.scriptBuilds, 2);
    expect(h.textureBuilds, 2);

    // Both kinds were already shown in the Changes embed, so even the FIRST
    // standalone Textures/Scripts entry refreshes (the shared provider
    // stayed alive and could predate a deploy/undeploy/game patch)…
    await goTo(_texturesTab);
    expect(h.textureBuilds, 3);
    expect(h.scriptBuilds, 2);
    await goTo(_scriptsTab);
    expect(h.scriptBuilds, 3);
    expect(h.textureBuilds, 3);

    // …and re-entries keep refreshing, as before.
    await goTo(_texturesTab);
    expect(h.textureBuilds, 4);
    await goTo(_scriptsTab);
    expect(h.scriptBuilds, 4);
  });

  testWidgets(
      'first standalone tab entry refreshes when the Changes embed already '
      'loaded the shared provider', (tester) async {
    final h = await _pumpHarness(tester);

    // The Changes tab's Textures section was opened at some point: its
    // _selectSection consulted the tracker (first display of the kind
    // anywhere — built fresh, no invalidate) and the kept-alive embed has
    // watched textureIndexProvider ever since (the harness listen).
    h.container
        .read(assetEntryTrackerProvider)
        .shouldInvalidateOnEntry(AssetKind.textureIndex);
    expect(h.textureBuilds, 1);

    // A deploy/undeploy/game patch happens here: the still-alive provider
    // now holds a stale texture index.

    // FIRST entry into the standalone Textures tab must refetch. (With the
    // per-surface visited set this entry counted as "fresh build" and the
    // refresh was skipped — the stale value stayed on screen.)
    h.controller.index = _texturesTab;
    await tester.pumpAndSettle();
    expect(h.textureBuilds, 2);
    // The scripts kind is untouched — never shown outside the initial
    // harness listen.
    expect(h.scriptBuilds, 1);
  });

  testWidgets(
      'the very first display of an asset kind does not double-fetch, and '
      'repeated settles on the same index do not re-invalidate',
      (tester) async {
    final h = await _pumpHarness(tester);

    // Very first display of textures anywhere: entering the standalone tab
    // must NOT invalidate on top of that entry's own fresh build.
    h.controller.index = _texturesTab;
    await tester.pumpAndSettle();
    expect(h.textureBuilds, 1);
    expect(h.scriptBuilds, 1);

    // Swipe-style repeat notifications on the settled index: no extra
    // invalidate (the settle debounce is preserved).
    h.controller.refire();
    h.controller.refire();
    await tester.pumpAndSettle();
    expect(h.textureBuilds, 1);

    // An actual re-entry refetches…
    h.controller.index = _itemsTab;
    await tester.pumpAndSettle();
    h.controller.index = _texturesTab;
    await tester.pumpAndSettle();
    expect(h.textureBuilds, 2);

    // …and repeat notifications after it stay debounced too.
    h.controller.refire();
    await tester.pumpAndSettle();
    expect(h.textureBuilds, 2);
  });
}
