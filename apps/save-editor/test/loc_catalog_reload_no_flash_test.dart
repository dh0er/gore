import 'dart:convert';
import 'dart:io';

import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:goresave/features/editor/domain/core_service.dart';
import 'package:goresave/loc/loc_catalog_provider.dart';
import 'package:goresave/providers/data_providers.dart';

/// Minimal core that reports a present catalog at [catalogPath].
class _LocCore implements GoresaveCoreService {
  _LocCore(this.catalogPath);

  final String catalogPath;

  @override
  bool get isAvailable => true;

  @override
  String get description => 'loc-fake';

  @override
  Future<Map<String, Object?>> execute(
    String command, {
    Map<String, Object?> payload = const {},
  }) async {
    if (command == 'loc_status') {
      return {
        'ok': true,
        'data': {'present': true, 'catalogPath': catalogPath},
      };
    }
    return {'ok': false};
  }
}

void main() {
  test(
    'reload keeps the previous catalog visible mid-flight (no flash to IDs)',
    () async {
      final dir = await Directory.systemTemp.createTemp('loc_catalog_test');
      addTearDown(() => dir.delete(recursive: true));
      final file = File('${dir.path}/loc_catalog.json');
      await file.writeAsString(
        jsonEncode({
          'ItFo_Cheese': {'default': 'Käse'},
        }),
      );

      final container = ProviderContainer(
        overrides: [coreServiceProvider.overrideWithValue(_LocCore(file.path))],
      );
      addTearDown(container.dispose);

      // Keep the provider alive across the reload (mirrors a mounted consumer),
      // otherwise it would be disposed between reads and lose its previous value.
      final sub = container.listen(locCatalogProvider, (_, _) {});
      addTearDown(sub.close);

      // Initial load resolves the localized name.
      final first = await container.read(locCatalogProvider.future);
      expect(first['itfo_cheese']?['default'], 'Käse');

      // A window resume bumps the reload counter, recomputing the FutureProvider.
      container.read(locCatalogReloadProvider.notifier).state++;

      // Mid-reload the provider is loading and `.asData` is null — exactly why
      // the old `.asData?.value ?? {}` consumers flashed raw IDs. `.value` (which
      // retains the previous value during loading) must still expose the catalog
      // so names hold steady.
      final mid = container.read(locCatalogProvider);
      expect(mid.isLoading, isTrue);
      expect(mid.asData, isNull);
      expect(mid.value?['itfo_cheese']?['default'], 'Käse');

      // Reload settles back to data.
      final second = await container.read(locCatalogProvider.future);
      expect(second['itfo_cheese']?['default'], 'Käse');
    },
  );
}
