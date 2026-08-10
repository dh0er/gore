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
    this.authoritative = false,
  });

  final List<ModEntryMetaView> mods;
  final LoadoutView loadout;

  /// True while an FFI call is in flight.
  final bool busy;

  /// Message of the last failed FFI call, or null. Kept until the next call
  /// starts.
  final String? error;

  /// True only after the native library and persisted loadout were read and
  /// reconciled successfully. Mutations and Apply stay fail-closed otherwise.
  final bool authoritative;

  LibraryState copyWith({
    List<ModEntryMetaView>? mods,
    LoadoutView? loadout,
    bool? busy,
    String? error,
    bool? authoritative,
    bool clearError = false,
  }) {
    return LibraryState(
      mods: mods ?? this.mods,
      loadout: loadout ?? this.loadout,
      busy: busy ?? this.busy,
      error: clearError ? null : error ?? this.error,
      authoritative: authoritative ?? this.authoritative,
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
/// prior error on entry, and reload from the source of truth afterwards even
/// when the native operation reports an error. A failed reload clears stale
/// data and marks the state non-authoritative until [refresh] succeeds.
class LibraryNotifier extends StateNotifier<LibraryState> {
  LibraryNotifier(this._mgr) : super(const LibraryState());

  final MgrFfi _mgr;

  /// Reload the library and loadout, reconciling the two so that:
  ///  * every library mod id appears exactly once in the loadout,
  ///  * ids present in the loadout but no longer in the library are dropped,
  ///  * library ids missing from the loadout are appended at the end,
  ///    disabled, preserving the stored order for the rest.
  Future<void> refresh() async {
    await _runRefresh();
  }

  /// Import a mod from [path] into the library, then refresh.
  Future<void> import(String path) async {
    await _runMutation(() async {
      await _mgr.import(path);
    });
  }

  /// Remove the mod [id] from the library, then refresh.
  Future<void> remove(String id) async {
    await _runMutation(() async {
      await _mgr.remove(id);
    });
  }

  /// Flip the enabled flag of the loadout entry for [id], persist the whole
  /// loadout, then refresh. No-op (still refreshes) if [id] isn't present.
  Future<void> toggle(String id) async {
    await _runMutation(() async {
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
    });
  }

  /// Move the loadout entry at [oldIndex] to [newIndex], persist, then
  /// refresh. Indices follow [ReorderableListView] semantics (the item is
  /// removed first, so a downward move lands one slot earlier).
  Future<void> reorder(int oldIndex, int newIndex) async {
    await _runMutation(() async {
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
    });
  }

  /// Refresh without its own busy/error framing — for use inside another
  /// mutation/refresh lane so the whole operation is one busy span.
  ///
  /// Reconciling only fixes the *in-memory* loadout; the on-disk loadout that
  /// `mgr_apply`/`mgr_status` read is untouched. So when reconciliation
  /// actually changed something (a stale entry dropped, a new library mod
  /// appended), persist the result via `mgr_set_loadout` so the on-disk
  /// loadout matches what the UI shows. The delta check makes this idempotent:
  /// a loadout that already agrees with the library persists nothing, so the
  /// follow-up refresh after a persist can't loop.
  Future<void> _refreshInline() async {
    final (mods, loadout) = await _mgr.libraryList();
    final reconciled = _reconcile(mods, loadout);
    if (!_sameEntries(loadout.entries, reconciled.entries)) {
      await _mgr.setLoadout(reconciled);
    }
    state = state.copyWith(
      mods: mods,
      loadout: reconciled,
      authoritative: true,
    );
  }

  /// Explicit read lane. It remains available while state is unknown because
  /// a successful refresh is what restores authority.
  Future<void> _runRefresh() async {
    if (state.busy) return;
    state = state.copyWith(busy: true, clearError: true);
    try {
      await _refreshInline();
    } catch (error) {
      _markUnknown();
      state = state.copyWith(error: _errorMessage(error));
    } finally {
      state = state.copyWith(busy: false);
    }
  }

  /// Single-flight mutation lane. Every operation is followed by an
  /// authoritative reload even when the operation throws: native commands can
  /// mutate the library/loadout and only then report a follow-up failure.
  Future<void> _runMutation(Future<void> Function() operation) async {
    if (state.busy || !state.authoritative) return;
    state = state.copyWith(busy: true, clearError: true);

    Object? operationError;
    try {
      await operation();
    } catch (error) {
      operationError = error;
    }

    Object? reloadError;
    try {
      await _refreshInline();
    } catch (error) {
      reloadError = error;
      _markUnknown();
    }

    final visibleError = operationError ?? reloadError;
    if (visibleError != null) {
      state = state.copyWith(error: _errorMessage(visibleError));
    }
    state = state.copyWith(busy: false);
  }

  void _markUnknown() {
    state = state.copyWith(
      mods: const [],
      loadout: const LoadoutView(),
      authoritative: false,
    );
  }

  static String _errorMessage(Object error) => switch (error) {
    MgrFfiException() => error.message,
    _ => error.toString(),
  };

  /// True when two loadout entry lists carry the same ids, enabled flags, and
  /// order — the delta gate for persisting a reconciled loadout.
  static bool _sameEntries(List<LoadoutEntryView> a, List<LoadoutEntryView> b) {
    if (a.length != b.length) return false;
    for (var i = 0; i < a.length; i++) {
      if (a[i].id != b[i].id || a[i].enabled != b[i].enabled) return false;
    }
    return true;
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
final libraryProvider = StateNotifierProvider<LibraryNotifier, LibraryState>((
  ref,
) {
  return LibraryNotifier(ref.watch(mgrFfiProvider))..refresh();
});
