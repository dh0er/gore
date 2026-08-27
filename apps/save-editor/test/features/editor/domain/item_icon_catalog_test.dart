import 'dart:async';
import 'dart:convert';
import 'dart:io';

import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:goresave/features/editor/domain/core_service.dart';
import 'package:goresave/features/editor/domain/item_icon_catalog.dart';
import 'package:goresave/providers/data_providers.dart';
import 'package:goresave/utils/shared_config.dart';
import 'package:path/path.dart' as p;

void main() {
  test('manifest resolves relative paths and item ids case-insensitively', () {
    final root = Directory.systemTemp.createTempSync(
      'gore_item_icons_manifest',
    );
    addTearDown(() => root.deleteSync(recursive: true));
    final manifest = p.join(root.path, 'manifest.json');
    final catalog = ItemIconCatalog.fromManifestJson(
      manifestPath: manifest,
      json: jsonEncode({
        'schema': 1,
        'buildId': 'game-generation',
        'itemCount': 2,
        'items': {
          'ItMi_Orenugget': 'images/ItMi_Orenugget.png',
          'ItFo_Apple': 'images/ItFo_Apple.png',
        },
      }),
    );

    expect(
      catalog.pathFor(itemId: 'ITMI_ORENUGGET'),
      p.join(root.path, 'images', 'ItMi_Orenugget.png'),
    );
    expect(
      catalog.pathFor(itemId: '', itemPath: '/Script/Angelscript.ItFo_Apple'),
      p.join(root.path, 'images', 'ItFo_Apple.png'),
    );
  });

  test('manifest rejects incomplete or escaping item maps', () {
    final root = Directory.systemTemp.createTempSync('gore_item_icons_invalid');
    addTearDown(() => root.deleteSync(recursive: true));
    final manifest = p.join(root.path, 'manifest.json');

    expect(
      () => ItemIconCatalog.fromManifestJson(
        manifestPath: manifest,
        json: jsonEncode({
          'buildId': 'generation',
          'itemCount': 1,
          'items': {'ItFo_Apple': '../outside.png'},
        }),
      ),
      throwsFormatException,
    );
    expect(
      () => ItemIconCatalog.fromManifestJson(
        manifestPath: manifest,
        json: jsonEncode({
          'schema': 1,
          'buildId': 'generation',
          'itemCount': 1,
          'items': {'ItFo_Apple': p.join(root.path, 'absolute.png')},
        }),
      ),
      throwsFormatException,
    );
    expect(
      () => ItemIconCatalog.fromManifestJson(
        manifestPath: manifest,
        json: jsonEncode({
          'buildId': 'generation',
          'itemCount': 2,
          'items': {'ItFo_Apple': 'apple.png'},
        }),
      ),
      throwsFormatException,
    );
  });

  test('provider prepares the cache with the configured game path', () async {
    final root = Directory.systemTemp.createTempSync(
      'gore_item_icons_provider',
    );
    addTearDown(() => root.deleteSync(recursive: true));
    final manifest = File(p.join(root.path, 'manifest.json'))
      ..writeAsStringSync(
        jsonEncode({
          'schema': 1,
          'buildId': 'generation',
          'itemCount': 1,
          'items': {'ItFo_Apple': 'ItFo_Apple.png'},
        }),
      );
    final config = SharedConfig(File(p.join(root.path, 'config.json')))
      ..setGamePath('D:/Games/Gothic Remake');
    final core = _ItemIconCore(manifest.path);
    final container = ProviderContainer(
      overrides: [
        itemIconCoreServiceProvider.overrideWithValue(core),
        sharedConfigProvider.overrideWithValue(config),
      ],
    );
    addTearDown(container.dispose);

    final catalog = await container.read(itemIconCatalogProvider.future);

    expect(core.commands, ['item_icons_prepare']);
    expect(core.payloads, [
      {'gamePath': 'D:/Games/Gothic Remake'},
    ]);
    expect(catalog.pathFor(itemId: 'ItFo_Apple'), isNotNull);
  });

  test('failed reload keeps the previous generation available', () async {
    final root = Directory.systemTemp.createTempSync('gore_item_icons_reload');
    addTearDown(() => root.deleteSync(recursive: true));
    final manifest = File(p.join(root.path, 'manifest.json'))
      ..writeAsStringSync(
        jsonEncode({
          'schema': 1,
          'buildId': 'generation',
          'itemCount': 1,
          'items': {'ItFo_Apple': 'ItFo_Apple.png'},
        }),
      );
    final secondResponse = Completer<Map<String, Object?>>();
    final core = _ReloadItemIconCore(manifest.path, secondResponse);
    final container = ProviderContainer(
      overrides: [itemIconCoreServiceProvider.overrideWithValue(core)],
    );
    addTearDown(container.dispose);
    final sub = container.listen(itemIconCatalogProvider, (_, _) {});
    addTearDown(sub.close);

    final first = await container.read(itemIconCatalogProvider.future);
    expect(first.pathFor(itemId: 'ItFo_Apple'), isNotNull);

    container.read(itemIconCatalogReloadProvider.notifier).state++;
    await Future<void>.delayed(Duration.zero);
    final mid = container.read(itemIconCatalogProvider);
    expect(mid.isLoading, isTrue);
    expect(mid.value?.pathFor(itemId: 'ItFo_Apple'), isNotNull);

    secondResponse.complete({
      'ok': false,
      'error': {'message': 'transient native failure'},
    });
    final afterFailure = await container.read(itemIconCatalogProvider.future);
    expect(afterFailure.pathFor(itemId: 'ItFo_Apple'), isNotNull);
  });

  test('successful replacement releases the previous generation', () async {
    final root = Directory.systemTemp.createTempSync('gore_item_icons_replace');
    addTearDown(() => root.deleteSync(recursive: true));
    File manifest(String generation) =>
        File(p.join(root.path, generation, 'manifest.json'))
          ..createSync(recursive: true)
          ..writeAsStringSync(
            jsonEncode({
              'schema': 1,
              'buildId': generation,
              'itemCount': 1,
              'items': {'ItFo_Apple': 'ItFo_Apple.png'},
            }),
          );
    final firstManifest = manifest('generation-a');
    final secondManifest = manifest('generation-b');
    final core = _ReplacingItemIconCore([
      firstManifest.path,
      secondManifest.path,
    ]);
    final container = ProviderContainer(
      overrides: [itemIconCoreServiceProvider.overrideWithValue(core)],
    );
    addTearDown(container.dispose);
    final sub = container.listen(itemIconCatalogProvider, (_, _) {});
    addTearDown(sub.close);

    final first = await container.read(itemIconCatalogProvider.future);
    expect(first.buildId, 'generation-a');

    container.read(itemIconCatalogReloadProvider.notifier).state++;
    final second = await container.read(itemIconCatalogProvider.future);
    await Future<void>.delayed(Duration.zero);
    expect(second.buildId, 'generation-b');
    expect(core.commands, [
      'item_icons_prepare',
      'item_icons_prepare',
      'item_icons_release',
    ]);
    expect(core.payloads.last, {'manifestPath': firstManifest.path});
  });
}

class _ItemIconCore implements GoresaveCoreService {
  _ItemIconCore(this.manifestPath);

  final String manifestPath;
  final List<String> commands = [];
  final List<Map<String, Object?>> payloads = [];

  @override
  String get description => 'item-icon-test-core';

  @override
  bool get isAvailable => true;

  @override
  Future<Map<String, Object?>> execute(
    String command, {
    Map<String, Object?> payload = const {},
  }) async {
    commands.add(command);
    payloads.add(payload);
    return {
      'ok': true,
      'data': {'manifestPath': manifestPath},
    };
  }
}

class _ReloadItemIconCore implements GoresaveCoreService {
  _ReloadItemIconCore(this.manifestPath, this.secondResponse);

  final String manifestPath;
  final Completer<Map<String, Object?>> secondResponse;
  int calls = 0;

  @override
  String get description => 'item-icon-reload-test-core';

  @override
  bool get isAvailable => true;

  @override
  Future<Map<String, Object?>> execute(
    String command, {
    Map<String, Object?> payload = const {},
  }) {
    calls++;
    if (calls == 1) {
      return Future.value({
        'ok': true,
        'data': {'manifestPath': manifestPath},
      });
    }
    return secondResponse.future;
  }
}

class _ReplacingItemIconCore implements GoresaveCoreService {
  _ReplacingItemIconCore(this.manifestPaths);

  final List<String> manifestPaths;
  final List<String> commands = [];
  final List<Map<String, Object?>> payloads = [];
  int prepares = 0;

  @override
  String get description => 'replacing-item-icon-test-core';

  @override
  bool get isAvailable => true;

  @override
  Future<Map<String, Object?>> execute(
    String command, {
    Map<String, Object?> payload = const {},
  }) async {
    commands.add(command);
    payloads.add(payload);
    if (command == 'item_icons_prepare') {
      return {
        'ok': true,
        'data': {'manifestPath': manifestPaths[prepares++]},
      };
    }
    return {
      'ok': true,
      'data': {'released': true},
    };
  }
}
