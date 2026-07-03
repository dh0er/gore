import 'package:flutter_riverpod/flutter_riverpod.dart';

/// The install-bound asset data shared by two display surfaces each: the
/// standalone Textures/Scripts main tab and the Changes tab's embedded
/// Textures/Scripts section, both watching the same `autoDispose` provider
/// (`textureIndexProvider` / `scriptModulesProvider`).
enum AssetKind { textureIndex, scriptModules }

/// Session-wide record of which shared asset kinds have been displayed on
/// ANY surface, deciding whether entering a surface must refresh the backing
/// provider.
///
/// Both surfaces keep their subtree alive after the user leaves
/// (`KeepAliveTab` main tabs; the kept-alive Changes tab keeps its embedded
/// section mounted), which keeps the shared `autoDispose` provider alive
/// too. A per-surface "visited" set therefore gets first entries wrong: a
/// surface's FIRST entry is not necessarily a fresh provider build — the
/// OTHER surface may have created the provider long ago, and its value can
/// predate a deploy, undeploy, or game patch. The correct invariant is
/// session-wide: only the very first display of a kind anywhere builds
/// fresh; every later entry must refetch.
class AssetEntryTracker {
  final Set<AssetKind> _seen = {};

  /// Whether a surface entering a view of [kind] should invalidate the
  /// backing provider.
  ///
  /// The first call per kind returns false — that entry's own build creates
  /// the provider fresh, so invalidating would double-fetch — and marks the
  /// kind as seen. Every later call (same surface or the other one) returns
  /// true: the provider may have stayed alive the whole time, so its value
  /// can be stale.
  bool shouldInvalidateOnEntry(AssetKind kind) => !_seen.add(kind);
}

/// App-scoped [AssetEntryTracker]: one instance per [ProviderContainer]
/// lifetime (= app session), shared by home_page's main-tab entry handler
/// and the Changes tab's section selection so both consult the same
/// seen-state.
final assetEntryTrackerProvider = Provider<AssetEntryTracker>(
  (ref) => AssetEntryTracker(),
);
