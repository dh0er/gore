import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gore_manager/core/core_service.dart';
import 'package:gore_manager/core/providers.dart';
import 'package:gore_manager/gore_manager_app.dart';

void main() {
  testWidgets('app boots with a fake core service', (tester) async {
    // The skeleton home makes no FFI calls yet; the empty fake just proves
    // the app boots against an injected service.
    final fake = FakeGoreCoreFfiService(responses: const {});
    await tester.pumpWidget(
      ProviderScope(
        overrides: [coreServiceProvider.overrideWithValue(fake)],
        child: const GoreManagerApp(),
      ),
    );
    await tester.pumpAndSettle();

    expect(find.text('GORE Mod Manager'), findsOneWidget);
    expect(tester.takeException(), isNull);
  });
}
