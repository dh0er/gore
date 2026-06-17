import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/core/core_service.dart';
import 'package:gore_mod/core/providers.dart';
import 'package:gore_mod/editor/domain/override_entry.dart';
import 'package:gore_mod/editor/domain/overrides_notifier.dart';
import 'package:gore_mod/export/domain/export_notifier.dart';
import 'package:gore_mod/export/domain/export_request.dart';

void main() {
  const apple500 = OverrideEntry(
    classId: 'ItFo_Apple', field: 'm_Value', oldValue: 4, newValue: 500,
  );
  const sword = OverrideEntry(
    classId: 'ItMw_1H_Sword_01', field: 'm_Value', oldValue: 50, newValue: 200,
  );

  testWidgets('ExportNotifier passes correct override list to validate_override and generate_mod', (tester) async {
    final fake = FakeGoreCoreFfiService(responses: {
      'validate_override': {'ok': true},
      'generate_mod': {
        'ok': true,
        'data': {'output_path': 'C:/mods/MyBalanceMod'},
      },
    });

    // Test via ExportNotifier directly (no full dialog pump needed for the core assertion)
    final container = ProviderContainer(
      overrides: [coreServiceProvider.overrideWithValue(fake)],
    );
    addTearDown(container.dispose);

    // Seed overrides
    final overridesNotifier = container.read(overridesProvider.notifier);
    overridesNotifier.setOverride(apple500);
    overridesNotifier.setOverride(sword);

    final exportNotifier = container.read(exportProvider.notifier);
    await exportNotifier.export(
      request: const ExportRequest(
        modName: 'MyBalanceMod',
        targetDir: 'C:/mods',
        delayMs: 0,
      ),
      overrides: container.read(overridesProvider).entries,
    );

    // validate_override called once per override
    final validateCalls = fake.calls.where((c) => c.command == 'validate_override').toList();
    expect(validateCalls, hasLength(2));

    // Check first validate_override call carries correct payload shape
    final appleCall = validateCalls.firstWhere(
      (c) => c.payload['class'] == 'ItFo_Apple',
    );
    expect(appleCall.payload['field'], 'm_Value');
    expect(appleCall.payload['value'], 500);

    // generate_mod called once with full override list
    final genCalls = fake.calls.where((c) => c.command == 'generate_mod').toList();
    expect(genCalls, hasLength(1));
    final genPayload = genCalls.first.payload;
    expect(genPayload['meta'], containsPair('name', 'MyBalanceMod'));
    expect(genPayload['meta'], containsPair('delay_ms', 0));
    final sentOverrides = genPayload['overrides'] as List;
    expect(sentOverrides, hasLength(2));
    expect(sentOverrides.map((o) => (o as Map)['class']), containsAll(['ItFo_Apple', 'ItMw_1H_Sword_01']));

    // Result
    expect(container.read(exportProvider).result?.success, isTrue);
  });

  testWidgets('ExportNotifier surfaces validation errors and does not call generate_mod', (tester) async {
    final fake = FakeGoreCoreFfiService(responses: {
      'validate_override': {
        'ok': false,
        'error': {'message': 'Unknown field'},
      },
    });

    final container = ProviderContainer(
      overrides: [coreServiceProvider.overrideWithValue(fake)],
    );
    addTearDown(container.dispose);

    await container.read(exportProvider.notifier).export(
      request: const ExportRequest(modName: 'Test', targetDir: 'C:/mods'),
      overrides: [apple500],
    );

    expect(container.read(exportProvider).validationErrors, isNotEmpty);
    expect(fake.calls.where((c) => c.command == 'generate_mod'), isEmpty);
  });
}
