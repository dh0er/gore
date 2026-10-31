import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gore_manager/core/mgr_ffi.dart';
import 'package:gore_manager/status/ui/script_rebuild_confirmation_dialog.dart';

import '../support/l10n_test_app.dart';

MgrScriptModuleUpdateWarning _warning(int index, {bool newModule = false}) =>
    MgrScriptModuleUpdateWarning(
      modId: 'mod-$index',
      modName: 'Mod $index',
      module: 'Module.$index',
      reason: newModule ? 'added_module_now_exists' : 'vanilla_module_changed',
      originalSha256: List.filled(64, newModule ? '0' : 'a').join(),
      currentSha256: List.filled(64, 'b').join(),
    );

Widget _host(MgrScriptRebuildConfirmation confirmation) => wrapWithL10n(
  Scaffold(
    body: Builder(
      builder: (context) => TextButton(
        onPressed: () => showDialog<bool>(
          context: context,
          builder: (_) =>
              ScriptRebuildConfirmationDialog(confirmation: confirmation),
        ),
        child: const Text('Review'),
      ),
    ),
  ),
);

void main() {
  testWidgets('new vanilla name collision gets a human explanation', (
    tester,
  ) async {
    await tester.pumpWidget(
      _host(
        MgrScriptRebuildConfirmation(
          token: 'opaque-token',
          warnings: [_warning(1, newModule: true)],
        ),
      ),
    );
    await tester.tap(find.text('Review'));
    await tester.pumpAndSettle();
    expect(find.text('Mod 1: Module.1'), findsOneWidget);
    expect(
      find.text(
        'The game now includes a module with the same name as this mod’s new module.',
      ),
      findsOneWidget,
    );
    expect(find.text('The original game module changed.'), findsNothing);
    await tester.tap(find.byKey(const ValueKey('script-rebuild-cancel')));
    await tester.pumpAndSettle();
  });

  testWidgets(
    'long warning list scrolls while Cancel and Confirm stay reachable',
    (tester) async {
      await tester.binding.setSurfaceSize(const Size(700, 460));
      addTearDown(() => tester.binding.setSurfaceSize(null));
      await tester.pumpWidget(
        _host(
          MgrScriptRebuildConfirmation(
            token: 'opaque-token',
            warnings: List.generate(40, (index) => _warning(index)),
          ),
        ),
      );
      await tester.tap(find.text('Review'));
      await tester.pumpAndSettle();
      await tester.scrollUntilVisible(
        find.text('Mod 39: Module.39'),
        250,
        scrollable: find.byType(Scrollable),
      );
      expect(find.text('Mod 39: Module.39').hitTestable(), findsOneWidget);
      expect(
        find.byKey(const ValueKey('script-rebuild-confirm')).hitTestable(),
        findsOneWidget,
      );
      expect(
        find.byKey(const ValueKey('script-rebuild-cancel')).hitTestable(),
        findsOneWidget,
      );
      await tester.tap(find.byKey(const ValueKey('script-rebuild-cancel')));
      await tester.pumpAndSettle();
      expect(tester.takeException(), isNull);
    },
  );
}
