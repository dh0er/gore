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

  test('same-generation reload releases its redundant lease claim', () async {
    final root = Directory.systemTemp.createTempSync('gore_item_icons_repeat');
    addTearDown(() => root.deleteSync(recursive: true));
    final manifest = File(p.join(root.path, 'manifest.json'))
      ..writeAsStringSync(
        jsonEncode({
          'schema': 1,
          'buildId': 'generation-a',
          'itemCount': 1,
          'items': {'ItFo_Apple': 'ItFo_Apple.png'},
        }),
      );
    final core = _ReplacingItemIconCore([manifest.path, manifest.path]);
    final container = ProviderContainer(
      overrides: [itemIconCoreServiceProvider.overrideWithValue(core)],
    );
    addTearDown(container.dispose);
    final sub = container.listen(itemIconCatalogProvider, (_, _) {});
    addTearDown(sub.close);

    await container.read(itemIconCatalogProvider.future);
    container.read(itemIconCatalogReloadProvider.notifier).state++;
    await container.read(itemIconCatalogProvider.future);
    await Future<void>.delayed(Duration.zero);

    expect(core.commands.last, 'item_icons_release');
    expect(core.payloads.last, {'manifestPath': manifest.path});
  });

  test('an obsolete load cannot replace the latest retained catalog', () async {
    final root = Directory.systemTemp.createTempSync('gore_item_icons_overlap');
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
    final core = _OverlappingItemIconCore();
    final container = ProviderContainer(
      overrides: [itemIconCoreServiceProvider.overrideWithValue(core)],
    );
    addTearDown(container.dispose);
    final sub = container.listen(itemIconCatalogProvider, (_, _) {});
    addTearDown(sub.close);

    final firstLoad = container.read(itemIconCatalogProvider.future);
    await Future<void>.delayed(Duration.zero);
    container.read(itemIconCatalogReloadProvider.notifier).state++;
    await Future<void>.delayed(Duration.zero);

    core.prepares[1].complete({
      'ok': true,
      'data': {'manifestPath': secondManifest.path},
    });
    expect(
      (await container.read(itemIconCatalogProvider.future)).buildId,
      'generation-b',
    );
    core.prepares[0].complete({
      'ok': true,
      'data': {'manifestPath': firstManifest.path},
    });
    await firstLoad;

    container.read(itemIconCatalogReloadProvider.notifier).state++;
    await Future<void>.delayed(Duration.zero);
    core.prepares[2].complete({
      'ok': false,
      'error': {'message': 'transient native failure'},
    });
    expect(
      (await container.read(itemIconCatalogProvider.future)).buildId,
      'generation-b',
    );
  });

  test('cheap source changes trigger one production cache reload', () async {
    final root = Directory.systemTemp.createTempSync('gore_item_icons_source');
    addTearDown(() => root.deleteSync(recursive: true));
    final manifest = File(p.join(root.path, 'manifest.json'))
      ..writeAsStringSync(
        jsonEncode({
          'schema': 1,
          'buildId': 'generation-a',
          'itemCount': 1,
          'items': {'ItFo_Apple': 'ItFo_Apple.png'},
        }),
      );
    final core = _SourceChangeItemIconCore(
      manifest.path,
      ['source-a', 'source-b'],
      ['source-a', 'source-b'],
    );
    final config = SharedConfig(File(p.join(root.path, 'config.json')))
      ..setGamePath('C:/stale-configured-game');
    final container = ProviderContainer(
      overrides: [
        itemIconCoreServiceProvider.overrideWithValue(core),
        sharedConfigProvider.overrideWithValue(config),
      ],
    );
    addTearDown(container.dispose);
    final sub = container.listen(itemIconCatalogProvider, (_, _) {});
    addTearDown(sub.close);

    await container.read(itemIconCatalogProvider.future);
    final refresh = container.read(itemIconCatalogRefreshProvider);
    await refresh.refreshIfSourceChanged();
    expect(container.read(itemIconCatalogReloadProvider), 0);

    await refresh.refreshIfSourceChanged();
    expect(container.read(itemIconCatalogReloadProvider), 1);
    await container.read(itemIconCatalogProvider.future);
    expect(core.prepares, 2);
    expect(core.identityPayloads, [
      {'gamePath': 'C:/stale-configured-game'},
      {'gamePath': 'C:/stale-configured-game'},
    ]);
  });

  test(
    'failed preparation backs off before retrying unchanged source',
    () async {
      final root = Directory.systemTemp.createTempSync(
        'gore_item_icons_failed',
      );
      addTearDown(() => root.deleteSync(recursive: true));
      final manifest = File(p.join(root.path, 'manifest.json'))
        ..writeAsStringSync(
          jsonEncode({
            'schema': 1,
            'buildId': 'generation-a',
            'itemCount': 1,
            'items': {'ItFo_Apple': 'ItFo_Apple.png'},
          }),
        );
      var now = DateTime.utc(2026, 1, 1);
      final core = _SourceChangeItemIconCore(
        manifest.path,
        ['source-a', 'source-b', 'source-b'],
        ['source-b', 'source-b', 'source-b'],
        {1},
      );
      final container = ProviderContainer(
        overrides: [
          itemIconCoreServiceProvider.overrideWithValue(core),
          itemIconCatalogNowProvider.overrideWithValue(() => now),
        ],
      );
      addTearDown(container.dispose);
      final sub = container.listen(itemIconCatalogProvider, (_, _) {});
      addTearDown(sub.close);

      await container.read(itemIconCatalogProvider.future);
      final refresh = container.read(itemIconCatalogRefreshProvider);
      await refresh.refreshIfSourceChanged();
      await container.read(itemIconCatalogProvider.future);
      await refresh.refreshIfSourceChanged();

      expect(container.read(itemIconCatalogReloadProvider), 1);
      expect(core.prepares, 2);

      now = now.add(const Duration(minutes: 1, seconds: 1));
      await refresh.refreshIfSourceChanged();
      await container.read(itemIconCatalogProvider.future);
      expect(container.read(itemIconCatalogReloadProvider), 2);
      expect(core.prepares, 3);
    },
  );

  test('configured game path changes trigger one bounded reload', () async {
    final root = Directory.systemTemp.createTempSync(
      'gore_item_icons_config_change',
    );
    addTearDown(() => root.deleteSync(recursive: true));
    final manifest = File(p.join(root.path, 'manifest.json'))
      ..writeAsStringSync(
        jsonEncode({
          'schema': 1,
          'buildId': 'generation-a',
          'itemCount': 1,
          'items': {'ItFo_Apple': 'ItFo_Apple.png'},
        }),
      );
    final config = SharedConfig(File(p.join(root.path, 'config.json')))
      ..setGamePath('C:/configured-a');
    final core = _SourceChangeItemIconCore(
      manifest.path,
      ['source-a', 'source-b'],
      ['source-a', 'source-a'],
      {1},
    );
    final container = ProviderContainer(
      overrides: [
        itemIconCoreServiceProvider.overrideWithValue(core),
        sharedConfigProvider.overrideWithValue(config),
      ],
    );
    addTearDown(container.dispose);
    final sub = container.listen(itemIconCatalogProvider, (_, _) {});
    addTearDown(sub.close);

    await container.read(itemIconCatalogProvider.future);
    config.setGamePath('E:/configured-b');
    final refresh = container.read(itemIconCatalogRefreshProvider);
    await refresh.refreshIfSourceChanged();
    await container.read(itemIconCatalogProvider.future);
    await refresh.refreshIfSourceChanged();

    expect(container.read(itemIconCatalogReloadProvider), 1);
    expect(core.prepares, 2);
    expect(core.identityPayloads, [
      {'gamePath': 'E:/configured-b'},
      {'gamePath': 'E:/configured-b'},
    ]);
  });

  test('overlapping source checks coalesce one cache reload', () async {
    final root = Directory.systemTemp.createTempSync(
      'gore_item_icons_coalesce',
    );
    addTearDown(() => root.deleteSync(recursive: true));
    final manifest = File(p.join(root.path, 'manifest.json'))
      ..writeAsStringSync(
        jsonEncode({
          'schema': 1,
          'buildId': 'generation-a',
          'itemCount': 1,
          'items': {'ItFo_Apple': 'ItFo_Apple.png'},
        }),
      );
    final core = _SourceChangeItemIconCore(
      manifest.path,
      ['source-a', 'source-b'],
      ['source-b'],
    );
    final container = ProviderContainer(
      overrides: [itemIconCoreServiceProvider.overrideWithValue(core)],
    );
    addTearDown(container.dispose);
    final sub = container.listen(itemIconCatalogProvider, (_, _) {});
    addTearDown(sub.close);

    await container.read(itemIconCatalogProvider.future);
    final refresh = container.read(itemIconCatalogRefreshProvider);
    await Future.wait([
      refresh.refreshIfSourceChanged(),
      refresh.refreshIfSourceChanged(),
    ]);
    await container.read(itemIconCatalogProvider.future);

    expect(container.read(itemIconCatalogReloadProvider), 1);
    expect(core.prepares, 2);
    expect(core.identityReads, 1);
  });
  test('a new game path drops the root even when it cannot be read', () async {
    // The configured path is a file another tool rewrites; nothing about that
    // reaches Riverpod. A new installation that is momentarily unreadable
    // returns no identity and the resume path bails out — leaving the cached
    // root naming the OLD installation, which the portraits then read from.
    final root = Directory.systemTemp.createTempSync('gore_item_icons_root');
    addTearDown(() => root.deleteSync(recursive: true));
    final manifest = File(p.join(root.path, 'manifest.json'))
      ..writeAsStringSync(
        jsonEncode({
          'schema': 1,
          'buildId': 'generation-a',
          'itemCount': 1,
          'items': {'ItFo_Apple': 'ItFo_Apple.png'},
        }),
      );
    final core = _UnreadableAfterPrepareCore(manifest.path, 'D:/installed-a');
    final config = SharedConfig(File(p.join(root.path, 'config.json')))
      ..setGamePath('D:/installed-a');
    final container = ProviderContainer(
      overrides: [
        itemIconCoreServiceProvider.overrideWithValue(core),
        sharedConfigProvider.overrideWithValue(config),
      ],
    );
    addTearDown(container.dispose);
    final sub = container.listen(itemIconCatalogProvider, (_, _) {});
    addTearDown(sub.close);

    await container.read(itemIconCatalogProvider.future);
    expect(container.read(gameRootProvider), 'D:/installed-a');

    final refresh = container.read(itemIconCatalogRefreshProvider);
    config.setGamePath('E:/installed-b');
    await refresh.refreshIfSourceChanged();

    expect(core.prepares, 1, reason: 'the unreadable install cannot prepare');
    expect(container.read(gameRootProvider), 'E:/installed-b');

    // A second switch, now that the resolved root has already gone null:
    // dropping THAT provider again recomputes null for null, which notifies
    // nobody, so the root went on naming the first fallback.
    config.setGamePath('F:/installed-c');
    await refresh.refreshIfSourceChanged();
    expect(container.read(gameRootProvider), 'F:/installed-c');

    // And back to where it started. Measured against the last PREPARED path
    // this looks like no change at all, while the root cached in between is a
    // different installation entirely.
    config.setGamePath('D:/installed-a');
    await refresh.refreshIfSourceChanged();
    expect(container.read(gameRootProvider), 'D:/installed-a');
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
    if (command == 'item_icons_source_identity') {
      return {
        'ok': true,
        'data': {'sourceIdentity': 'source-a'},
      };
    }
    return {
      'ok': true,
      'data': {'manifestPath': manifestPath, 'sourceIdentity': 'source-a'},
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
    if (command == 'item_icons_source_identity') {
      return Future.value({
        'ok': true,
        'data': {'sourceIdentity': 'source-a'},
      });
    }
    calls++;
    if (calls == 1) {
      return Future.value({
        'ok': true,
        'data': {'manifestPath': manifestPath, 'sourceIdentity': 'source-a'},
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
        'data': {
          'manifestPath': manifestPaths[prepares++],
          'sourceIdentity': 'source-a',
        },
      };
    }
    if (command == 'item_icons_source_identity') {
      return {
        'ok': true,
        'data': {'sourceIdentity': 'source-a'},
      };
    }
    return {
      'ok': true,
      'data': {'released': true},
    };
  }
}

class _OverlappingItemIconCore implements GoresaveCoreService {
  final List<Completer<Map<String, Object?>>> prepares = [];

  @override
  String get description => 'overlapping-item-icon-test-core';

  @override
  bool get isAvailable => true;

  @override
  Future<Map<String, Object?>> execute(
    String command, {
    Map<String, Object?> payload = const {},
  }) {
    if (command == 'item_icons_release') {
      return Future.value({
        'ok': true,
        'data': {'released': true},
      });
    }
    if (command == 'item_icons_source_identity') {
      return Future.value({
        'ok': true,
        'data': {'sourceIdentity': 'source-a'},
      });
    }
    final response = Completer<Map<String, Object?>>();
    prepares.add(response);
    return response.future;
  }
}

class _SourceChangeItemIconCore implements GoresaveCoreService {
  _SourceChangeItemIconCore(
    this.manifestPath,
    this.prepareIdentities,
    this.identities, [
    this.failPrepareIndices = const {},
  ]);

  final String manifestPath;
  final List<String> prepareIdentities;
  final List<String> identities;
  final Set<int> failPrepareIndices;
  final String sourceGamePath = 'D:/Steam/Gothic 1 Remake';
  final List<Map<String, Object?>> identityPayloads = [];
  int prepares = 0;
  int identityReads = 0;

  @override
  String get description => 'source-change-item-icon-test-core';

  @override
  bool get isAvailable => true;

  @override
  Future<Map<String, Object?>> execute(
    String command, {
    Map<String, Object?> payload = const {},
  }) async {
    if (command == 'item_icons_prepare') {
      final prepareIndex = prepares++;
      if (failPrepareIndices.contains(prepareIndex)) {
        return {
          'ok': false,
          'error': {'message': 'transient native failure'},
        };
      }
      final sourceIdentity = prepareIdentities[prepareIndex];
      return {
        'ok': true,
        'data': {
          'manifestPath': manifestPath,
          'sourceIdentity': sourceIdentity,
          'sourceGamePath': sourceGamePath,
        },
      };
    }
    if (command == 'item_icons_source_identity') {
      identityPayloads.add(payload);
      return {
        'ok': true,
        'data': {'sourceIdentity': identities[identityReads++]},
      };
    }
    return {
      'ok': true,
      'data': {'released': true},
    };
  }
}

/// A core that prepares once and then refuses every identity read, the way a
/// newly selected installation does while it is unreachable.
class _UnreadableAfterPrepareCore implements GoresaveCoreService {
  _UnreadableAfterPrepareCore(this.manifestPath, this.sourceGamePath);

  final String manifestPath;
  final String sourceGamePath;
  int prepares = 0;

  @override
  String get description => 'unreadable-after-prepare-test-core';

  @override
  bool get isAvailable => true;

  @override
  Future<Map<String, Object?>> execute(
    String command, {
    Map<String, Object?> payload = const {},
  }) async {
    if (command == 'item_icons_prepare') {
      prepares++;
      return {
        'ok': true,
        'data': {
          'manifestPath': manifestPath,
          'sourceIdentity': 'source-a',
          'sourceGamePath': sourceGamePath,
        },
      };
    }
    if (command == 'item_icons_source_identity') {
      return {
        'ok': false,
        'error': {'message': 'installation unavailable'},
      };
    }
    return {
      'ok': true,
      'data': {'released': true},
    };
  }
}
