import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/app/domain/asset_entry_tracker.dart';

void main() {
  test('first entry per kind is fresh, every later entry invalidates', () {
    final tracker = AssetEntryTracker();

    // Very first display of a kind anywhere this session: that entry's own
    // build creates the provider fresh — no invalidate.
    expect(tracker.shouldInvalidateOnEntry(AssetKind.textureIndex), isFalse);

    // Every later entry (same surface or the other one): the provider may
    // have stayed alive the whole time — invalidate.
    expect(tracker.shouldInvalidateOnEntry(AssetKind.textureIndex), isTrue);
    expect(tracker.shouldInvalidateOnEntry(AssetKind.textureIndex), isTrue);
  });

  test('kinds are tracked independently', () {
    final tracker = AssetEntryTracker();

    expect(tracker.shouldInvalidateOnEntry(AssetKind.textureIndex), isFalse);
    // Textures having been shown must not mark scripts as seen…
    expect(tracker.shouldInvalidateOnEntry(AssetKind.scriptModules), isFalse);
    // …and each kind keeps its own later-entry answer.
    expect(tracker.shouldInvalidateOnEntry(AssetKind.scriptModules), isTrue);
    expect(tracker.shouldInvalidateOnEntry(AssetKind.textureIndex), isTrue);
  });

  test('provider hands out one session-scoped instance', () {
    final container = ProviderContainer();
    addTearDown(container.dispose);

    // A mark made through one read is visible through the next: both
    // surfaces (home_page's tab entry handler and ChangesTab's section
    // selection) share the same seen-state.
    container
        .read(assetEntryTrackerProvider)
        .shouldInvalidateOnEntry(AssetKind.textureIndex);
    expect(
      container
          .read(assetEntryTrackerProvider)
          .shouldInvalidateOnEntry(AssetKind.textureIndex),
      isTrue,
    );
  });
}
