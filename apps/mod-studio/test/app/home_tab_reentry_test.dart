import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/app/ui/tab_reentry_listener.dart';
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

/// Drives the REAL [handleMainTabReentered] mapping (home_page.dart) through
/// a real [TabController] + [TabReentryListener], without pumping the
/// FFI-heavy [HomePage] itself: the Changes-tab case must refresh exactly
/// the provider of the asset section currently embedded there.
void main() {
  testWidgets(
      'Changes main-tab re-entry refreshes exactly the embedded asset '
      "section's provider", (tester) async {
    var textureBuilds = 0;
    var scriptBuilds = 0;

    final controller = TabController(length: 7, vsync: const TestVSync());
    addTearDown(controller.dispose);

    await tester.pumpWidget(
      ProviderScope(
        overrides: [
          textureIndexProvider.overrideWith((ref) {
            textureBuilds++;
            return Future.value(const <String, String>{});
          }),
          scriptModulesProvider.overrideWith((ref) {
            scriptBuilds++;
            return Future.value(const <ScriptModuleInfo>[]);
          }),
        ],
        child: MaterialApp(
          home: Consumer(
            builder: (context, ref, _) => TabReentryListener(
              controller: controller,
              // The real home_page mapping under test — not a replica.
              onTabReentered: (index) => handleMainTabReentered(ref, index),
              child: const SizedBox.shrink(),
            ),
          ),
        ),
      ),
    );

    final container = ProviderScope.containerOf(
      tester.element(find.byType(TabReentryListener)),
    );
    // Stand-in for the keep-alive main tabs, which watch both providers in
    // the real app: keeps the autoDispose providers alive across tab
    // switches, so a refetch can only come from an explicit invalidate —
    // not from autoDispose disposal/re-creation.
    container.listen(textureIndexProvider, (_, _) {});
    container.listen(scriptModulesProvider, (_, _) {});
    await tester.pump();
    expect(textureBuilds, 1);
    expect(scriptBuilds, 1);

    Future<void> goTo(int index) async {
      controller.index = index;
      await tester.pumpAndSettle();
    }

    // First entry into Changes: excluded by TabReentryListener (in the real
    // app that build creates the tab's providers fresh — no double fetch).
    await goTo(_changesTab);
    expect(textureBuilds, 1);
    expect(scriptBuilds, 1);

    // Re-entry while the Changes tab shows a non-asset section (provider
    // default null = initial "All"): nothing to refresh.
    await goTo(_itemsTab);
    await goTo(_changesTab);
    expect(textureBuilds, 1);
    expect(scriptBuilds, 1);

    // Re-entry while the embedded section is Textures: exactly the texture
    // index refetches.
    container.read(changesAssetSectionProvider.notifier).state =
        ChangesAssetSection.textures;
    await goTo(_itemsTab);
    await goTo(_changesTab);
    expect(textureBuilds, 2);
    expect(scriptBuilds, 1);

    // Same for Scripts.
    container.read(changesAssetSectionProvider.notifier).state =
        ChangesAssetSection.scripts;
    await goTo(_itemsTab);
    await goTo(_changesTab);
    expect(scriptBuilds, 2);
    expect(textureBuilds, 2);

    // Standalone-tab parity is untouched: the dedicated Textures/Scripts
    // tabs still refresh their providers on re-entry, independent of the
    // Changes-tab section state.
    await goTo(_texturesTab); // first entry: no refresh
    await goTo(_scriptsTab); // first entry: no refresh
    expect(textureBuilds, 2);
    expect(scriptBuilds, 2);
    await goTo(_texturesTab); // re-entry
    expect(textureBuilds, 3);
    expect(scriptBuilds, 2);
    await goTo(_scriptsTab); // re-entry
    expect(scriptBuilds, 3);
    expect(textureBuilds, 3);
  });
}
