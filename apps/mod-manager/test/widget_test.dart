import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gore_manager/core/core_service.dart';
import 'package:gore_manager/core/providers.dart';
import 'package:gore_manager/gore_manager_app.dart';

void main() {
  testWidgets('app boots with a fake core service', (tester) async {
    final fake = FakeGoreCoreFfiService(
      responses: {
        'mgr_library_list': {
          'ok': true,
          'mods': <Object?>[],
          'loadout': {'format': 1, 'entries': <Object?>[]},
        },
      },
    );
    await tester.pumpWidget(
      ProviderScope(
        overrides: [coreServiceProvider.overrideWithValue(fake)],
        child: const GoreManagerApp(),
      ),
    );
    await tester.pumpAndSettle();

    expect(find.text('gore-manager'), findsOneWidget);
    expect(tester.takeException(), isNull);
  });
}
