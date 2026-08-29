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

/// Reserved manifest-key prefix for the shared `Common/Icons` UI glyphs.
/// Mirrors `UI_ICON_KEY_PREFIX` in gore-save.
const uiIconKeyPrefix = 'ui:';
const _maximumManifestBytes = 8 * 1024 * 1024;
const _preparationRetryDelays = [
  Duration(minutes: 1),
  Duration(minutes: 5),
  Duration(minutes: 15),
  Duration(minutes: 30),
];

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

  /// Path of a shared game UI glyph (`T_Icon_Mana`, `T_Icon_Resistance_Fire`,
  /// …), or null when this generation does not carry it. Native publishes the
  /// glyphs under a reserved `ui:` key so they cannot collide with an item id,
  /// and it is allowed to omit one the installed build no longer ships — every
  /// caller therefore needs its own fallback icon.
  String? uiPathFor(String iconName) {
    final trimmed = iconName.trim();
    if (trimmed.isEmpty) return null;
    return pathByItemId['$uiIconKeyPrefix$trimmed'.toLowerCase()];
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

@visibleForTesting
final itemIconCatalogNowProvider = Provider<DateTime Function()>(
  (ref) => DateTime.now,
);

final _itemIconCatalogRetentionProvider = Provider(
  (ref) => _ItemIconCatalogRetention(),
);

class _ItemIconCatalogRetention {
  ItemIconCatalog? value;
  int requestSequence = 0;
  String? sourceIdentity;
  String? sourceGamePath;
  String? requestedGamePath;
  String? pendingSourceIdentity;
  String? pendingRequestedGamePath;
  String? attemptedSourceIdentity;
  String? attemptedRequestedGamePath;
  int attemptedFailures = 0;
  DateTime? attemptedRetryAfter;
  Future<String?>? sourceIdentityRead;
  String? sourceIdentityReadGamePath;
}

final itemIconCatalogRefreshProvider = Provider<ItemIconCatalogRefresh>((ref) {
  final retention = ref.read(_itemIconCatalogRetentionProvider);
  return ItemIconCatalogRefresh._(
    () => ref.read(itemIconCoreServiceProvider),
    () => ref.read(sharedConfigProvider).gamePath(),
    () => ref.read(itemIconCatalogProvider).isLoading,
    () => ref.read(itemIconCatalogNowProvider)(),
    retention,
    () => ref.read(itemIconCatalogReloadProvider.notifier).state++,
  );
});

class ItemIconCatalogRefresh {
  ItemIconCatalogRefresh._(
    this._core,
    this._gamePath,
    this._catalogIsLoading,
    this._now,
    this._retention,
    this._reload,
  );

  final GoresaveCoreService Function() _core;
  final String? Function() _gamePath;
  final bool Function() _catalogIsLoading;
  final DateTime Function() _now;
  final _ItemIconCatalogRetention _retention;
  final void Function() _reload;

  /// Check only source-file metadata on resume. Full PNG verification runs
  /// solely when this identity changed or a previously missing install appears.
  Future<void> refreshIfSourceChanged() async {
    if (_catalogIsLoading()) return;
    final core = _core();
    if (!core.isAvailable) return;
    final configuredGamePath = _gamePath();
    final selectionChanged = configuredGamePath != _retention.requestedGamePath;
    final retainedSourceDiffersFromRequest =
        _retention.sourceGamePath != _retention.requestedGamePath;
    final identity = await _readItemIconSourceIdentity(
      core,
      selectionChanged || retainedSourceDiffersFromRequest
          ? configuredGamePath
          : _retention.sourceGamePath ?? configuredGamePath,
      _retention,
    );
    if (identity == null || _catalogIsLoading()) {
      return;
    }
    final pendingMatches =
        identity == _retention.pendingSourceIdentity &&
        configuredGamePath == _retention.pendingRequestedGamePath;
    final attemptedMatches =
        identity == _retention.attemptedSourceIdentity &&
        configuredGamePath == _retention.attemptedRequestedGamePath;
    final attemptedStillCoolingDown =
        attemptedMatches &&
        (_retention.attemptedRetryAfter == null ||
            _now().isBefore(_retention.attemptedRetryAfter!));
    if ((!selectionChanged && identity == _retention.sourceIdentity) ||
        pendingMatches ||
        attemptedStillCoolingDown) {
      return;
    }
    _retention.pendingSourceIdentity = identity;
    _retention.pendingRequestedGamePath = configuredGamePath;
    _reload();
  }
}

/// Ensures the complete bundled item set is cached, then reads its small
/// manifest. PNG bytes stay on disk and are decoded at widget size by Flutter.
final itemIconCatalogProvider = FutureProvider<ItemIconCatalog>((ref) async {
  ref.watch(itemIconCatalogReloadProvider);
  final retention = ref.read(_itemIconCatalogRetentionProvider);
  final now = ref.read(itemIconCatalogNowProvider);
  final requestSequence = ++retention.requestSequence;
  final requestedSourceIdentity = retention.pendingSourceIdentity;
  final requestedGamePath = retention.pendingRequestedGamePath;
  if (requestedSourceIdentity != null) {
    final sameAttempt =
        requestedSourceIdentity == retention.attemptedSourceIdentity &&
        requestedGamePath == retention.attemptedRequestedGamePath;
    if (!sameAttempt) {
      retention.attemptedFailures = 0;
      retention.attemptedRetryAfter = null;
    }
    retention.attemptedSourceIdentity = requestedSourceIdentity;
    retention.attemptedRequestedGamePath = requestedGamePath;
  }
  retention.pendingSourceIdentity = null;
  retention.pendingRequestedGamePath = null;
  ItemIconCatalog retainedOrEmpty() =>
      retention.value ?? const ItemIconCatalog.empty();
  final core = ref.watch(itemIconCoreServiceProvider);
  if (!core.isAvailable) return retainedOrEmpty();

  String? preparedManifestPath;
  var requestSucceeded = false;
  try {
    final gamePath = ref.watch(sharedConfigProvider).gamePath();
    final response = await core.execute(
      'item_icons_prepare',
      payload: {'gamePath': ?gamePath},
    );
    if (response['ok'] != true) return retainedOrEmpty();
    final data = (response['data'] as Map?)?.cast<String, Object?>();
    final manifestPath = data?['manifestPath'] as String?;
    final sourceIdentity = data?['sourceIdentity'] as String?;
    final sourceGamePath = data?['sourceGamePath'] as String?;
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
    if (sourceIdentity != null && sourceIdentity.isNotEmpty) {
      retention.sourceIdentity = sourceIdentity;
      retention.sourceGamePath =
          sourceGamePath != null && sourceGamePath.isNotEmpty
          ? sourceGamePath
          : null;
      retention.requestedGamePath = gamePath;
    }
    retention.attemptedSourceIdentity = null;
    retention.attemptedRequestedGamePath = null;
    retention.attemptedFailures = 0;
    retention.attemptedRetryAfter = null;
    final previousManifestPath = retention.value?.manifestPath;
    retention.value = catalog;
    requestSucceeded = true;
    if (previousManifestPath != null && previousManifestPath.isNotEmpty) {
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
    retention.pendingSourceIdentity = null;
    retention.pendingRequestedGamePath = null;
    if (preparedManifestPath != null) {
      await _releaseItemIconCatalog(core, preparedManifestPath);
    }
    // Images are enhancement-only. Every caller has a category-icon fallback,
    // so a missing game, corrupt cache, or transient extraction error must not
    // make the save editor unusable or erase a previously loaded generation.
    return retainedOrEmpty();
  } finally {
    if (!requestSucceeded &&
        requestSequence == retention.requestSequence &&
        requestedSourceIdentity != null &&
        requestedSourceIdentity == retention.attemptedSourceIdentity &&
        requestedGamePath == retention.attemptedRequestedGamePath) {
      final failures = retention.attemptedFailures + 1;
      retention.attemptedFailures = failures;
      final delayIndex = failures <= _preparationRetryDelays.length
          ? failures - 1
          : _preparationRetryDelays.length - 1;
      retention.attemptedRetryAfter = now().add(
        _preparationRetryDelays[delayIndex],
      );
    }
  }
});

Future<String?> _readItemIconSourceIdentity(
  GoresaveCoreService core,
  String? gamePath,
  _ItemIconCatalogRetention retention,
) {
  final active = retention.sourceIdentityRead;
  if (active != null && retention.sourceIdentityReadGamePath == gamePath) {
    return active;
  }
  final request = () async {
    try {
      final response = await core.execute(
        'item_icons_source_identity',
        payload: {'gamePath': ?gamePath},
      );
      if (response['ok'] != true) return null;
      final data = (response['data'] as Map?)?.cast<String, Object?>();
      final identity = data?['sourceIdentity'] as String?;
      return identity == null || identity.isEmpty ? null : identity;
    } catch (_) {
      return null;
    }
  }();
  retention.sourceIdentityRead = request;
  retention.sourceIdentityReadGamePath = gamePath;
  return request.whenComplete(() {
    if (identical(retention.sourceIdentityRead, request)) {
      retention.sourceIdentityRead = null;
      retention.sourceIdentityReadGamePath = null;
    }
  });
}

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
