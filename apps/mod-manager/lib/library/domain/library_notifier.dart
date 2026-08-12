import 'package:flutter_riverpod/legacy.dart';

import '../../core/mgr_ffi.dart';
import '../../core/providers.dart';
import 'models.dart';

/// The mod library plus its loadout, as shown in the Mods tab.
///
/// Immutable; every mutation returns a fresh instance via [copyWith]. The
/// Native returns [mods] and [loadout] from one locked, reconciled store
/// snapshot, so the UI can trust that every library mod has exactly one
/// loadout entry and vice versa.
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

  /// True only after Native returned one locked, reconciled library/loadout
  /// snapshot. Mutations and Apply stay fail-closed otherwise.
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

  /// Reload the authoritative library and loadout snapshot. Reconciliation and
  /// any necessary persistence happen inside Native under the store lock.
  Future<void> refresh() async {
    await _runRefresh();
  }

  /// Import a mod from [path], then reload Native's authoritative Store
  /// snapshot. A native outcome is returned only when that reload succeeds and
  /// contains the same entry id. Failures are rethrown only after the reload
  /// attempt and state publication have settled, so UI callers can present the
  /// typed FFI error without racing stale library data.
  Future<MgrImportOutcome?> import(String path) async {
    if (state.busy || !state.authoritative) return null;
    state = state.copyWith(busy: true, clearError: true);

    MgrImportOutcome? nativeOutcome;
    Object? operationError;
    StackTrace? operationStack;
    try {
      nativeOutcome = await _mgr.import(path);
    } catch (error, stack) {
      operationError = error;
      operationStack = stack;
    }

    Object? reloadError;
    StackTrace? reloadStack;
    try {
      await _refreshInline();
    } catch (error, stack) {
      reloadError = error;
      reloadStack = stack;
      _markUnknown();
    }

    // A failed reload dominates any operation result because the UI cannot
    // safely classify or select anything until authoritative state returns.
    // When reload succeeds, keep operation failures out of LibraryState: the
    // caller owns their localized import feedback and no raw native banner may
    // race it into the page.
    Object? visibleError = reloadError ?? operationError;
    StackTrace? visibleStack = reloadError != null
        ? reloadStack
        : operationStack;
    MgrImportOutcome? authoritativeOutcome;
    if (visibleError == null && nativeOutcome != null) {
      final authoritativeEntry = state.modById(nativeOutcome.entry.id);
      if (authoritativeEntry == null) {
        visibleError = MgrFfiException(
          'mgr_import: imported entry is absent from the authoritative snapshot',
          code: 'IMPORT_INVALID_RESPONSE',
        );
        visibleStack = StackTrace.current;
      } else {
        authoritativeOutcome = nativeOutcome.withEntry(authoritativeEntry);
      }
    }

    if (reloadError != null) {
      state = state.copyWith(error: _errorMessage(reloadError));
    }
    state = state.copyWith(busy: false);

    if (visibleError != null) {
      Error.throwWithStackTrace(
        visibleError,
        visibleStack ?? StackTrace.current,
      );
    }
    return authoritativeOutcome;
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
  /// This read lane never writes a loadout back. Native owns the locked
  /// reconciliation transaction; a Dart read-then-write would race CLI or
  /// another Manager process.
  Future<void> _refreshInline() async {
    final (mods, loadout) = await _mgr.libraryList();
    state = state.copyWith(mods: mods, loadout: loadout, authoritative: true);
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
}

/// The library + loadout, kicked off with an initial refresh.
final libraryProvider = StateNotifierProvider<LibraryNotifier, LibraryState>((
  ref,
) {
  return LibraryNotifier(ref.watch(mgrFfiProvider))..refresh();
});
