import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:goresave/features/app/domain/update_notifier.dart';
import 'package:goresave/features/app/ui/update_banner.dart';
import 'package:goresave/features/editor/domain/core_service.dart';

class _FakeCoreService implements GoresaveCoreService {
  _FakeCoreService(this.checkData);

  final Map<String, Object?> checkData;
  final List<String> commands = [];

  @override
  bool get isAvailable => true;

  @override
  String get description => 'fake';

  @override
  Future<Map<String, Object?>> execute(
    String command, {
    Map<String, Object?> payload = const {},
  }) async {
    commands.add(command);
    return switch (command) {
      'update_check' => {'ok': true, 'data': checkData},
      _ => {'ok': true, 'data': const <String, Object?>{}},
    };
  }
}

Widget _host(GoresaveCoreService core) {
  return ProviderScope(
    overrides: [
      updateProvider.overrideWith((ref) => UpdateNotifier(core)),
    ],
    child: const MaterialApp(
      home: UpdateBannerHost(child: Text('content')),
    ),
  );
}

void main() {
  testWidgets('no banner when idle', (tester) async {
    final core = _FakeCoreService({'status': 'upToDate'});
    await tester.pumpWidget(_host(core));
    await tester.pumpAndSettle();
    expect(find.text('content'), findsOneWidget);
    expect(find.textContaining('Update'), findsNothing);
  });

  testWidgets('banner shown when ready; restart triggers apply', (tester) async {
    final core = _FakeCoreService({
      'status': 'updateAvailable',
      'version': '0.2.0',
    });
    await tester.pumpWidget(_host(core));
    await tester.pumpAndSettle();
    expect(find.text('Update 0.2.0 ready'), findsOneWidget);
    expect(find.text('content'), findsOneWidget);

    await tester.tap(find.text('Restart to update'));
    await tester.pumpAndSettle();
    expect(core.commands, contains('update_apply_restart'));
  });
}
