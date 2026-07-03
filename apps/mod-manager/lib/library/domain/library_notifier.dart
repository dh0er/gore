import 'package:flutter_riverpod/legacy.dart';

import '../../core/mgr_ffi.dart';
import '../../core/providers.dart';
import 'models.dart';

/// The mod library plus its loadout, as shown in the Mods tab.
///
/// Immutable; every mutation returns a fresh instance via [copyWith]. The
/// [loadout] is always reconciled against [mods] (see
/// [LibraryNotifier.refresh]) so the UI can trust that every library mod has
/// exactly one loadout entry and vice versa.
class LibraryState {
  const LibraryState({
    this.mods = const [],
    this.loadout = const LoadoutView(),
    this.busy = false,
    this.error,
  });

  final List<ModEntryMetaView> mods;
  final LoadoutView loadout;

  /// True while an FFI call is in flight.
  final bool busy;

  /// Message of the last failed FFI call, or null. Kept until the next call
  /// starts.
  final String? error;

  LibraryState copyWith({
    List<ModEntryMetaView>? mods,
    LoadoutView? loadout,
    bool? busy,
    String? error,
    bool clearError = false,
  }) {
    return LibraryState(
      mods: mods ?? this.mods,
      loadout: loadout ?? this.loadout,
      busy: busy ?? this.busy,
      error: clearError ? null : error ?? this.error,
    );
  }

  /// Look up a mod by id, or null if it isn't in the library.
  ModEntryMetaView? modById(String id) {
    for (final m in mods) {
      if (m.id == id) return m;
    }
    return null;
  }
}

/// Owns the library + loadout and mediates every mutation through [MgrFfi].
///
/// All mutating methods set [LibraryState.busy] for their duration, clear any
/// prior error on entry, and refresh from the source of truth
/// (`mgr_library_list`) afterwards. An [MgrFfiException] is caught and its
/// message parked in [LibraryState.error]; the state is otherwise left intact.
class LibraryNotifier extends StateNotifier<LibraryState> {
  LibraryNotifier(this._mgr) : super(const LibraryState());

  final MgrFfi _mgr;

  /// Reload the library and loadout, reconciling the two so that:
  ///  * every library mod id appears exactly once in the loadout,
  ///  * ids present in the loadout but no longer in the library are dropped,
  ///  * library ids missing from the loadout are appended at the end,
  ///    disabled, preserving the stored order for the rest.
  Future<void> refresh() async {
    await _run(() async {
      final (mods, loadout) = await _mgr.libraryList();
      state = state.copyWith(
        mods: mods,
        loadout: _reconcile(mods, loadout),
      );
    });
  }

  /// Import a mod from [path] into the library, then refresh.
  Future<void> import(String path) async {
    await _run(() async {
      await _mgr.import(path);
      await _refreshInline();
    });
  }

  /// Remove the mod [id] from the library, then refresh.
  Future<void> remove(String id) async {
    await _run(() async {
      await _mgr.remove(id);
      await _refreshInline();
    });
  }

  /// Flip the enabled flag of the loadout entry for [id], persist the whole
  /// loadout, then refresh. No-op (still refreshes) if [id] isn't present.
  Future<void> toggle(String id) async {
    await _run(() async {
      final entries = [
        for (final e in state.loadout.entries)
          if (e.id == id)
            LoadoutEntryView(id: e.id, enabled: !e.enabled)
          else
            e,
      ];
      await _mgr.setLoadout(
        LoadoutView(format: state.loadout.format, entries: entries),
      );
      await _refreshInline();
    });
  }

  /// Move the loadout entry at [oldIndex] to [newIndex], persist, then
  /// refresh. Indices follow [ReorderableListView] semantics (the item is
  /// removed first, so a downward move lands one slot earlier).
  Future<void> reorder(int oldIndex, int newIndex) async {
    await _run(() async {
      final entries = [...state.loadout.entries];
      if (oldIndex < 0 || oldIndex >= entries.length) return;
      var target = newIndex;
      if (target > oldIndex) target -= 1;
      if (target < 0) target = 0;
      if (target > entries.length - 1) target = entries.length - 1;
      final moved = entries.removeAt(oldIndex);
      entries.insert(target, moved);
      await _mgr.setLoadout(
        LoadoutView(format: state.loadout.format, entries: entries),
      );
      await _refreshInline();
    });
  }

  /// Refresh without its own busy/error framing — for use inside another
  /// [_run] block so the whole operation is one busy span.
  Future<void> _refreshInline() async {
    final (mods, loadout) = await _mgr.libraryList();
    state = state.copyWith(mods: mods, loadout: _reconcile(mods, loadout));
  }

  /// Run [body] with the busy flag set and errors funneled into the state.
  Future<void> _run(Future<void> Function() body) async {
    state = state.copyWith(busy: true, clearError: true);
    try {
      await body();
    } on MgrFfiException catch (e) {
      state = state.copyWith(error: e.message);
    } finally {
      state = state.copyWith(busy: false);
    }
  }

  /// See [refresh] for the reconciliation contract.
  static LoadoutView _reconcile(
    List<ModEntryMetaView> mods,
    LoadoutView loadout,
  ) {
    final known = {for (final m in mods) m.id};
    final seen = <String>{};
    final kept = <LoadoutEntryView>[
      for (final e in loadout.entries)
        if (known.contains(e.id) && seen.add(e.id)) e,
    ];
    // Append any library mods that had no loadout entry, disabled, in library
    // order.
    for (final m in mods) {
      if (!seen.contains(m.id)) {
        kept.add(LoadoutEntryView(id: m.id, enabled: false));
        seen.add(m.id);
      }
    }
    return LoadoutView(format: loadout.format, entries: kept);
  }
}

/// The library + loadout, kicked off with an initial refresh.
final libraryProvider =
    StateNotifierProvider<LibraryNotifier, LibraryState>((ref) {
  return LibraryNotifier(ref.watch(mgrFfiProvider))..refresh();
});
