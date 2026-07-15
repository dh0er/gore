import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/scripts/domain/script_compile_install_state_provider.dart';
import 'package:gore_mod/scripts/domain/script_compile_report.dart';
import 'package:gore_mod/scripts/ui/script_compile_install_state_banner.dart';

void main() {
  testWidgets('shows managed recovery without offering a legacy report', (
    tester,
  ) async {
    final controller = ScriptCompileInstallSafetyController(
      (_) async => throw UnimplementedError(),
      gameRoot: r'C:\Game',
      autoRefresh: false,
    );
    addTearDown(controller.dispose);
    controller.recordManagedRecovery(
      gameRoot: r'C:\Game',
      code: 'INSTALL_RESTORE_FAILED',
      message: 'exact restore could not be proven',
      installRestore: ScriptCompileInstallRestore.recoveryRequiredRestoreFailed,
    );

    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: ScriptCompileInstallStateBanner(
            state: controller.current,
            onRecheck: () {},
            onViewRecoveryReport: () {},
          ),
        ),
      ),
    );

    expect(find.text('Game installation recovery required'), findsOneWidget);
    expect(find.textContaining('INSTALL_RESTORE_FAILED'), findsOneWidget);
    expect(
      find.textContaining('exact restore could not be proven'),
      findsOneWidget,
    );
    expect(
      find.byKey(const Key('script-compile-install-state-view-report')),
      findsNothing,
    );
  });
}
