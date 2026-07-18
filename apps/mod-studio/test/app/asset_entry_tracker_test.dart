import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/app/domain/asset_entry_tracker.dart';

void main() {
  test('first Scripts entry is fresh, every later entry invalidates', () {
    final tracker = AssetEntryTracker();

    expect(tracker.shouldInvalidateOnEntry(AssetKind.scriptModules), isFalse);
    expect(tracker.shouldInvalidateOnEntry(AssetKind.scriptModules), isTrue);
    expect(tracker.shouldInvalidateOnEntry(AssetKind.scriptModules), isTrue);
  });

  test('provider hands out one session-scoped instance', () {
    final container = ProviderContainer();
    addTearDown(container.dispose);

    container
        .read(assetEntryTrackerProvider)
        .shouldInvalidateOnEntry(AssetKind.scriptModules);
    expect(
      container
          .read(assetEntryTrackerProvider)
          .shouldInvalidateOnEntry(AssetKind.scriptModules),
      isTrue,
    );
  });
}
