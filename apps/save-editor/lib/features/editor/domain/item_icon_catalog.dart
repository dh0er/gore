import 'dart:async';
import 'dart:convert';
import 'dart:io';

import 'package:flutter/foundation.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_riverpod/legacy.dart';
import 'package:goresave/features/editor/domain/core_service.dart';
import 'package:goresave/providers/data_providers.dart';
import 'package:path/path.dart' as p;

const _maximumItemCount = 4096;
const _maximumManifestBytes = 8 * 1024 * 1024;

/// Verified, generation-bound item images extracted from the user's own game.
class ItemIconCatalog {
  const ItemIconCatalog({
    required this.buildId,
    required this.manifestPath,
    required this.pathByItemId,
  });

  const ItemIconCatalog.empty()
    : buildId = '',
      manifestPath = '',
      pathByItemId = const {};

  final String buildId;
  final String manifestPath;
  final Map<String, String> pathByItemId;

  String? pathFor({required String itemId, String itemPath = ''}) {
    for (final candidate in <String>[itemId, _itemIdFromPath(itemPath)]) {
      final path = pathByItemId[candidate.trim().toLowerCase()];
      if (path != null) return path;
    }
    return null;
  }

  @visibleForTesting
  static ItemIconCatalog fromManifestJson({
    required String manifestPath,
    required String json,
  }) {
    final decoded = jsonDecode(json);
    if (decoded is! Map) {
      throw const FormatException('Item icon manifest is not an object');
    }
    final object = decoded.cast<String, Object?>();
    final schema = object['schema'];
    final buildId = object['buildId'] as String? ?? '';
    final itemCount = object['itemCount'];
    final rawItems = object['items'];
    if (schema is! int ||
        schema != 1 ||
        buildId.isEmpty ||
        buildId.length > 512 ||
        buildId.trim() != buildId ||
        buildId.runes.any((rune) => rune < 0x20 || rune == 0x7f) ||
        itemCount is! int ||
        itemCount < 1 ||
        itemCount > _maximumItemCount ||
        rawItems is! Map) {
      throw const FormatException('Item icon manifest is incomplete');
    }

    final manifestDir = p.dirname(p.absolute(manifestPath));
    final paths = <String, String>{};
    rawItems.forEach((id, rawPath) {
      if (id is! String || id.trim().isEmpty || rawPath is! String) return;
      final trimmed = rawPath.trim();
      if (trimmed.isEmpty) return;
      if (p.isAbsolute(trimmed) ||
          p.extension(trimmed).toLowerCase() != '.png') {
        throw const FormatException('Item icon path is not a relative PNG');
      }
      final resolved = p.normalize(p.join(manifestDir, trimmed));
      // Native manifests use relative paths below their immutable generation
      // directory. Refuse a malformed relative path that escapes that authority.
      if (!p.isWithin(manifestDir, resolved) &&
          !p.equals(manifestDir, resolved)) {
        throw const FormatException(
          'Item icon path escapes its cache directory',
        );
      }
      paths[id.trim().toLowerCase()] = resolved;
    });
    if (paths.length != itemCount) {
      throw FormatException(
        'Item icon manifest expected $itemCount entries, found ${paths.length}',
      );
    }
    return ItemIconCatalog(
      buildId: buildId,
      manifestPath: p.absolute(manifestPath),
      pathByItemId: Map.unmodifiable(paths),
    );
  }
}

String _itemIdFromPath(String path) {
  final trimmed = path.trim();
  final separator = trimmed.lastIndexOf('.') > trimmed.lastIndexOf('/')
      ? trimmed.lastIndexOf('.')
      : trimmed.lastIndexOf('/');
  return separator < 0 ? trimmed : trimmed.substring(separator + 1);
}

/// A second native worker keeps the initial image extraction from queueing save
/// inspection and edits behind hundreds of texture decodes.
final itemIconCoreServiceProvider = Provider<GoresaveCoreService>((ref) {
  final core = ref.watch(coreServiceProvider);
  if (core is NativeGoresaveCoreService) {
    return NativeGoresaveCoreService.withLibraryPath(core.description);
  }
  // Widget-test and unavailable-core substitutes should not receive a new
  // production command they do not model. Item-icon tests override this
  // provider directly with their purpose-built fake.
  return MissingGoresaveCoreService();
});

/// Explicit reload hook. Do not bump this on ordinary app resume: native cache
/// verification intentionally proves every PNG and is therefore not cheap.
final itemIconCatalogReloadProvider = StateProvider<int>((ref) => 0);

final _itemIconCatalogRetentionProvider = Provider(
  (ref) => _ItemIconCatalogRetention(),
);

class _ItemIconCatalogRetention {
  ItemIconCatalog? value;
  int requestSequence = 0;
}

/// Ensures the complete bundled item set is cached, then reads its small
/// manifest. PNG bytes stay on disk and are decoded at widget size by Flutter.
final itemIconCatalogProvider = FutureProvider<ItemIconCatalog>((ref) async {
  ref.watch(itemIconCatalogReloadProvider);
  final retention = ref.read(_itemIconCatalogRetentionProvider);
  final requestSequence = ++retention.requestSequence;
  ItemIconCatalog retainedOrEmpty() =>
      retention.value ?? const ItemIconCatalog.empty();
  final core = ref.watch(itemIconCoreServiceProvider);
  if (!core.isAvailable) return retainedOrEmpty();

  String? preparedManifestPath;
  try {
    final gamePath = ref.watch(sharedConfigProvider).gamePath();
    final response = await core.execute(
      'item_icons_prepare',
      payload: {'gamePath': ?gamePath},
    );
    if (response['ok'] != true) return retainedOrEmpty();
    final data = (response['data'] as Map?)?.cast<String, Object?>();
    final manifestPath = data?['manifestPath'] as String?;
    if (manifestPath == null || manifestPath.isEmpty) {
      return retainedOrEmpty();
    }
    preparedManifestPath = manifestPath;
    final file = File(manifestPath);
    if (!await file.exists()) {
      await _releaseItemIconCatalog(core, manifestPath);
      return retainedOrEmpty();
    }
    final manifestLength = await file.length();
    if (manifestLength < 1 || manifestLength > _maximumManifestBytes) {
      await _releaseItemIconCatalog(core, manifestPath);
      return retainedOrEmpty();
    }
    final catalog = ItemIconCatalog.fromManifestJson(
      manifestPath: manifestPath,
      json: await file.readAsString(),
    );
    if (requestSequence != retention.requestSequence) {
      await _releaseItemIconCatalog(core, catalog.manifestPath);
      return retainedOrEmpty();
    }
    final previousManifestPath = retention.value?.manifestPath;
    retention.value = catalog;
    if (previousManifestPath != null &&
        previousManifestPath.isNotEmpty) {
      // Publish the new catalog first. Widgets can still paint the retained
      // AsyncData for the rest of this event turn, so release its native lease
      // on the next turn rather than opening a deletion race with that paint.
      // Native leases are reference-counted, so overlapping A -> B -> A loads
      // remain safe regardless of request completion order. A same-generation
      // reload also releases the redundant claim acquired by preparation.
      unawaited(
        Future<void>.delayed(
          Duration.zero,
          () => _releaseItemIconCatalog(core, previousManifestPath),
        ),
      );
    }
    return catalog;
  } catch (_) {
    if (preparedManifestPath != null) {
      await _releaseItemIconCatalog(core, preparedManifestPath);
    }
    // Images are enhancement-only. Every caller has a category-icon fallback,
    // so a missing game, corrupt cache, or transient extraction error must not
    // make the save editor unusable or erase a previously loaded generation.
    return retainedOrEmpty();
  }
});

Future<void> _releaseItemIconCatalog(
  GoresaveCoreService core,
  String manifestPath,
) async {
  try {
    await core.execute(
      'item_icons_release',
      payload: {'manifestPath': manifestPath},
    );
  } catch (_) {
    // The OS releases the lease when the editor exits. A failed best-effort
    // release may retain one old cache generation, but must not hide images.
  }
}
