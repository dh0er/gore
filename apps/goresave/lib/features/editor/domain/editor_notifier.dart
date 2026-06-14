import 'dart:io';

import 'package:file_selector/file_selector.dart';
import 'package:goresave/features/editor/domain/core_service.dart';
import 'package:goresave/features/editor/domain/editor_models.dart';
import 'package:goresave/features/editor/domain/editor_settings_store.dart';
import 'package:goresave/features/editor/domain/hero_attributes.dart';
import 'package:goresave/features/editor/domain/pending_edits.dart';
import 'package:goresave/features/editor/domain/progression_models.dart';
import 'package:goresave/utils/default_paths.dart';
import 'package:path/path.dart' as p;
import 'package:state_notifier/state_notifier.dart';

const _unchanged = Object();

/// Sorts saves by in-game playtime (highest first). Slots with null playtime
/// sink to the bottom. Equal or both-null playtime falls back to file
/// last-modified descending so the order is stable on files that lack
/// persistent metadata — saves whose file can't be stat'd sink to the very
/// bottom rather than throwing, so a transient FS error never breaks the scan.
void _sortByPlaytimeDesc(List<SaveSlot> saves) {
  final mtime = <String, DateTime>{};
  for (final save in saves) {
    try {
      mtime[save.path] = File(save.path).lastModifiedSync();
    } catch (_) {
      // Leave unset; treated as oldest in the mtime tie-break below.
    }
  }
  saves.sort((a, b) {
    final pa = a.timePlayedSeconds;
    final pb = b.timePlayedSeconds;
    // Primary key: playtime descending; nulls sink to the bottom.
    if (pa != null && pb != null) {
      final cmp = pb.compareTo(pa);
      if (cmp != 0) return cmp;
    } else if (pa == null && pb != null) {
      return 1;
    } else if (pa != null && pb == null) {
      return -1;
    }
    // Secondary key: last-modified descending (tie-break / no-metadata path).
    final ma = mtime[a.path];
    final mb = mtime[b.path];
    if (ma == null && mb == null) return 0;
    if (ma == null) return 1;
    if (mb == null) return -1;
    return mb.compareTo(ma);
  });
}

class EditorState {
  const EditorState({
    required this.saveDir,
    required this.codecHostPath,
    required this.gameExePath,
    this.isLoading = false,
    this.saves = const [],
    this.profiles = const [],
    this.activeProfileId,
    this.selectedProfileId,
    this.backups = const [],
    this.companionBackups = const [],
    this.selectedPath,
    this.inspection,
    this.codecStatus,
    this.codecVerified = false,
    this.error,
    this.codecError,
    this.lastWriteMessage,
    this.pendingEdits = const {},
    this.pendingDifficulty,
  });

  final String saveDir;
  final String codecHostPath;
  final String gameExePath;
  final bool isLoading;
  final List<SaveSlot> saves;
  final List<ProfileSummary> profiles;
  final int? activeProfileId;

  /// Explicitly selected profile id. Null means no explicit selection — use
  /// [effectiveProfileId] for the resolved value.
  final int? selectedProfileId;

  final List<BackupEntry> backups;
  final List<BackupEntry> companionBackups;
  final String? selectedPath;
  final SaveInspection? inspection;
  final CodecStatus? codecStatus;

  /// Pending (unsaved) savegame edits, keyed by editor surface
  /// (e.g. 'publicName', 'heroStats', 'transform', 'attr:Health',
  /// 'inventory', 'typed:&lt;joined path&gt;'). Cleared on save, refresh to a
  /// different save, or selection change.
  final Map<String, PendingSaveEdit> pendingEdits;

  /// The pending (unsaved) difficulty edit, or null when difficulty is at the
  /// selected save's stored value with no propagation requested. It is held as
  /// a typed field rather than in [pendingEdits] because a difficulty write is
  /// its own multi-target `write_difficulty` command (not a `write_save` edit
  /// object). It participates in [hasUnsavedEdits], [pendingEditCount] (so the
  /// global "Unsaved (N)" badge counts it and Save enables), the global
  /// [saveAllPending], and the global Reset. Cleared on a save switch, refresh,
  /// and whenever the user reverts the controls back to the stored value.
  final PendingDifficulty? pendingDifficulty;

  /// True when there are any unsaved edits anywhere — pending registry edits or
  /// a pending difficulty edit. The profile-switch guard blocks on this so a
  /// difficulty edit is protected exactly like a pending field edit.
  bool get hasUnsavedEdits =>
      pendingEdits.isNotEmpty || pendingDifficulty != null;

  /// True once the user ran a successful codec round-trip verification this
  /// session for an executable the probe could not auto-trust (a pattern-
  /// resolved / unknown game build where canCompress is false). Known-profile
  /// builds report canCompress directly and never need this.
  final bool codecVerified;
  final String? error;

  /// Compression-dependent private writes are safe when the probe already
  /// trusts the codec, or the user verified it this session.
  bool get codecCompressReady =>
      (codecStatus?.canCompress ?? false) || codecVerified;

  /// The codec is usable but not yet trusted for compression — a manual
  /// verification round-trip would unlock writes.
  bool get codecNeedsVerification =>
      (codecStatus?.available ?? false) &&
      !(codecStatus?.canCompress ?? false) &&
      !codecVerified;

  /// Error from the most recent codec check. Kept separate from [error] so a
  /// save-directory refresh does not wipe a standing codec configuration error.
  final String? codecError;
  final String? lastWriteMessage;

  /// Total number of edit objects across all pending keys, plus the pending
  /// difficulty edit (counted as one) so the global "Unsaved (N)" badge and the
  /// Save/Reset buttons reflect a pending difficulty just like any field edit.
  int get pendingEditCount =>
      pendingEdits.values.fold(0, (n, e) => n + e.edits.length) +
      (pendingDifficulty != null ? 1 : 0);

  SaveSlot? get selectedSave {
    for (final save in saves) {
      if (save.path == selectedPath) return save;
    }
    return null;
  }

  /// The profile id to use for filtering: the explicitly selected profile, or
  /// fall back to the scan's active profile id.
  /// One resolution shared by the header and the save-list filter, so they
  /// can never disagree: explicit switcher choice first, then the selected
  /// save's own profile, then the scan's active profile id.
  int? get effectiveProfileId =>
      selectedProfileId ?? selectedSave?.persistentProfileId ?? activeProfileId;

  /// Saves to show in the sidebar. When there are fewer than 2 profiles, or
  /// no effective profile id, all saves are shown. Otherwise only saves whose
  /// [SaveSlot.persistentProfileId] matches [effectiveProfileId] are shown
  /// (saves with a null persistentProfileId stay visible in every profile —
  /// they cannot be attributed). The currently selected save is always kept
  /// visible so it is never silently removed from the list mid-session.
  List<SaveSlot> get visibleSaves {
    final eid = effectiveProfileId;
    if (eid == null || profiles.length < 2) return saves;
    return saves
        .where(
          (s) =>
              s.persistentProfileId == eid ||
              s.persistentProfileId == null ||
              s.path == selectedPath,
        )
        .toList();
  }

  ProfileSummary? get activeProfile {
    // Same resolution as the save-list filter (effectiveProfileId), so the
    // header always describes the profile whose saves are listed.
    final targetProfileId = effectiveProfileId;
    for (final profile in profiles) {
      if (profile.profileId == targetProfileId) return profile;
    }
    // No profile matches: report none rather than guessing `profiles.first`,
    // which would show another profile's name and counts.
    return null;
  }

  /// The profile of the SAVE currently under edit (the inspected/selected save),
  /// resolved from its `persistentProfileId`. This is what difficulty
  /// propagation must bind to — distinct from [activeProfile], which describes
  /// the sidebar's effective filter profile and can differ from the edited
  /// save's own profile. Null when the selected save has no profile id or no
  /// matching profile exists (propagation is then disabled).
  ProfileSummary? get editedSaveProfile {
    final profileId = selectedSave?.persistentProfileId;
    if (profileId == null) return null;
    for (final profile in profiles) {
      if (profile.profileId == profileId) return profile;
    }
    return null;
  }

  EditorState copyWith({
    String? saveDir,
    String? codecHostPath,
    String? gameExePath,
    bool? isLoading,
    List<SaveSlot>? saves,
    List<ProfileSummary>? profiles,
    Object? activeProfileId = _unchanged,
    Object? selectedProfileId = _unchanged,
    List<BackupEntry>? backups,
    List<BackupEntry>? companionBackups,
    Object? selectedPath = _unchanged,
    SaveInspection? inspection,
    CodecStatus? codecStatus,
    bool? codecVerified,
    String? error,
    String? codecError,
    String? lastWriteMessage,
    Map<String, PendingSaveEdit>? pendingEdits,
    Object? pendingDifficulty = _unchanged,
    bool clearInspection = false,
    bool clearBackups = false,
    bool clearError = false,
    bool clearCodecError = false,
    bool clearCodecStatus = false,
    bool clearWriteMessage = false,
    bool clearPendingEdits = false,
  }) {
    return EditorState(
      saveDir: saveDir ?? this.saveDir,
      codecHostPath: codecHostPath ?? this.codecHostPath,
      gameExePath: gameExePath ?? this.gameExePath,
      isLoading: isLoading ?? this.isLoading,
      saves: saves ?? this.saves,
      profiles: profiles ?? this.profiles,
      activeProfileId: identical(activeProfileId, _unchanged)
          ? this.activeProfileId
          : activeProfileId as int?,
      selectedProfileId: identical(selectedProfileId, _unchanged)
          ? this.selectedProfileId
          : selectedProfileId as int?,
      backups: clearBackups ? const [] : backups ?? this.backups,
      companionBackups: clearBackups
          ? const []
          : companionBackups ?? this.companionBackups,
      selectedPath: identical(selectedPath, _unchanged)
          ? this.selectedPath
          : selectedPath as String?,
      inspection: clearInspection ? null : inspection ?? this.inspection,
      codecStatus: clearCodecStatus ? null : codecStatus ?? this.codecStatus,
      codecVerified: codecVerified ?? this.codecVerified,
      error: clearError ? null : error ?? this.error,
      codecError: clearCodecError ? null : codecError ?? this.codecError,
      lastWriteMessage: clearWriteMessage
          ? null
          : lastWriteMessage ?? this.lastWriteMessage,
      pendingEdits: clearPendingEdits
          ? const {}
          : pendingEdits ?? this.pendingEdits,
      // A pending difficulty edit is cleared on the same events that clear the
      // pending-edit registry (slot switch, refresh, fresh inspection landing),
      // so honour clearPendingEdits for it too. Otherwise apply the explicit
      // value (passing null clears it; the _unchanged sentinel keeps it).
      pendingDifficulty: clearPendingEdits
          ? null
          : (identical(pendingDifficulty, _unchanged)
                ? this.pendingDifficulty
                : pendingDifficulty as PendingDifficulty?),
    );
  }
}

class EditorNotifier extends StateNotifier<EditorState> {
  EditorNotifier(
    this._core, {
    String? saveDir,
    String? codecHostPath,
    String? gameExePath,
    EditorSettingsStore? settingsStore,
  }) : _settingsStore = settingsStore ?? const NoopEditorSettingsStore(),
       super(
         _initialState(
           saveDir: saveDir,
           codecHostPath: codecHostPath,
           gameExePath: gameExePath,
           settingsStore: settingsStore ?? const NoopEditorSettingsStore(),
         ),
       ) {
    refresh();
    checkCodec();
  }

  final GoresaveCoreService _core;
  final EditorSettingsStore _settingsStore;

  /// Monotonic token identifying the latest in-flight load. Only the op holding
  /// the current token may write loading/result state; superseded ops bail
  /// without touching it, so the most recent op always clears `isLoading`.
  int _loadSeq = 0;

  /// Number of in-flight loads (inspect / backup refresh). The overlay shows
  /// while this is > 0; it is cleared only when the last load finishes, so an
  /// older load completing after a newer one can neither clear the spinner
  /// early nor turn it back on.
  int _activeLoads = 0;

  void _loadStarted() {
    _activeLoads++;
  }

  void _loadFinished() {
    if (_activeLoads > 0) _activeLoads--;
    if (_activeLoads == 0) {
      state = state.copyWith(isLoading: false);
    }
  }

  /// Run a mutating action (write/validate/restore) as a tracked load: show the
  /// overlay, clear prior errors, and always clear loading afterwards — even if
  /// the core call throws — so the spinner can't get stuck. Counting also lets
  /// checkCodec see that a load is in flight and not race it with an inspect.
  Future<void> _withLoading(Future<void> Function() body) async {
    _loadStarted();
    state = state.copyWith(isLoading: true, clearError: true);
    try {
      await body();
    } catch (error) {
      // A thrown core call (e.g. bad JSON / null native response) must surface
      // as an error rather than propagate and leave the UI wedged.
      state = state.copyWith(error: 'Unexpected error: $error');
    } finally {
      _loadFinished();
    }
  }

  /// Run a single write request (`write_save` by default) as a tracked load,
  /// then rescan on success. Returns true only when the core accepted the
  /// write; a rejected write sets `state.error` and returns false so callers
  /// can skip success-only follow-ups. The post-success `refresh()` rescans
  /// saves AND profiles. Used by [restoreBackup] and [applyMemoryEventEdit];
  /// the global [saveAllPending] orchestrates its writes inline so it can do a
  /// slot write_save and a write_difficulty with a single trailing refresh.
  Future<bool> _runWrite({
    required Map<String, Object?> payload,
    required String Function(Map<String, Object?> data) message,
    String command = 'write_save',
  }) async {
    var ok = false;
    await _withLoading(() async {
      final response = await _execute(command, payload: payload);
      if (response['ok'] != true) {
        state = state.copyWith(error: _errorMessage(response));
        return;
      }
      final data = (response['data'] as Map).cast<String, Object?>();
      state = state.copyWith(lastWriteMessage: message(data));
      await refresh();
      ok = true;
    });
    return ok;
  }

  /// Resolve the `write_difficulty` targets for [edit] against the current
  /// state, binding propagation to the EDITED SAVE's profile (not the sidebar's
  /// filter profile). Returns the assembled command payload, or null when there
  /// is no selected save to target.
  ///
  /// - The current save is always a target.
  /// - `allSaves`: every save whose `persistentProfileId` matches the edited
  ///   save's profile id (current save always included).
  /// - `alsoProfile`: the profile's `PersistentDataList.sav`
  ///   (`dirname(currentSavePath)/PersistentDataList.sav`) with that profile id.
  ///
  /// The `binaryHost` codec host is attached exactly as the `write_save` path
  /// does (via [_codecPayload]) — the private-payload edit on save targets
  /// requires the codec host, and the core errors without it.
  Map<String, Object?>? _buildDifficultyPayload(PendingDifficulty edit) {
    final path = state.selectedPath;
    if (path == null) return null;

    // The profile of the SAVE under edit (not state.activeProfile, which is the
    // sidebar's effective filter profile and can differ). Resolve it from the
    // selected save's persistentProfileId.
    final editedProfileId = state.selectedSave?.persistentProfileId;
    ProfileSummary? editedProfile;
    if (editedProfileId != null) {
      for (final profile in state.profiles) {
        if (profile.profileId == editedProfileId) {
          editedProfile = profile;
          break;
        }
      }
    }

    final savePaths = <String>[path];
    if (edit.allSaves && editedProfile != null) {
      savePaths
        ..clear()
        ..addAll(
          state.saves
              .where((s) => s.persistentProfileId == editedProfile!.profileId)
              .map((s) => s.path),
        );
      // The current save is always a target, even if its persistentProfileId is
      // null or mismatched.
      if (!savePaths.contains(path)) savePaths.add(path);
    }

    // The save `path`s here carry the on-disk style of the save folder, which
    // is Windows-style for these saves even when the host (e.g. Linux CI) is
    // POSIX. `package:path`'s top-level functions use the HOST style, so on a
    // POSIX host `p.dirname('C:\\...\\G1R-001.sav')` collapses to '.'. Pick a
    // Context matching the SAVE path's style so dirname/join stay correct on any
    // host. Treat a backslash or an `X:` drive prefix as Windows-style.
    final isWindowsStyle =
        path.contains('\\') || RegExp(r'^[A-Za-z]:').hasMatch(path);
    final ctx = isWindowsStyle ? p.Context(style: p.Style.windows) : p.posix;

    final targets = <String, Object?>{
      'saves': savePaths,
      if (edit.alsoProfile && editedProfile != null)
        'profile': {
          'path': ctx.join(ctx.dirname(path), 'PersistentDataList.sav'),
          'profileId': editedProfile.profileId,
        },
    };

    return {
      'difficulty': edit.difficulty,
      'targets': targets,
      'backup': true,
      ..._codecPayload(),
    };
  }

  /// Serializes all core calls. The native layer runs each command in its own
  /// isolate with no serialization, so overlapping write_save/restore_backup
  /// requests on the same file could interleave temp files and renames. Chaining
  /// through this queue guarantees one core command finishes before the next
  /// starts.
  Future<void> _coreQueue = Future<void>.value();

  bool get coreAvailable => _core.isAvailable;
  String get coreDescription => _core.description;

  /// Convenience forwarder — prefer [EditorState.pendingEditCount].
  int get pendingEditCount => state.pendingEditCount;

  /// Dismiss the current error banner.
  void dismissError() {
    if (state.error != null) state = state.copyWith(clearError: true);
  }

  /// Dismiss the current success/status banner.
  void dismissWriteMessage() {
    if (state.lastWriteMessage != null) {
      state = state.copyWith(clearWriteMessage: true);
    }
  }

  /// Switch the active profile filter. Pass null to clear the explicit
  /// selection (show all profiles).
  ///
  /// Blocked with an error when there are unsaved edits — switching profiles
  /// changes which saves are visible and would potentially move selection away
  /// from the save the edits target.
  ///
  /// If the currently selected save is not in the new visible set, the first
  /// visible save is selected (triggering [_inspect]); if there are none,
  /// the selection is cleared.
  Future<void> selectProfile(int? profileId) async {
    if (state.hasUnsavedEdits) {
      state = state.copyWith(
        error:
            'Save or reset your unsaved changes first — switching profiles '
            'would move away from the current save.',
      );
      return;
    }

    // Check whether the current selection belongs to the target profile before
    // updating state. We avoid relying on visibleSaves here so that the "keep
    // selected save visible" exemption cannot silently keep the old save in view
    // and prevent the selection from moving.
    final currentSave = state.selectedSave;
    final selectionMatchesNewProfile =
        profileId == null ||
        state.profiles.length < 2 ||
        currentSave == null ||
        currentSave.persistentProfileId == null ||
        currentSave.persistentProfileId == profileId;

    state = state.copyWith(selectedProfileId: profileId);

    if (selectionMatchesNewProfile) {
      // Current selection is compatible with the new profile — stay put.
      return;
    }

    // Current save does not belong to the new profile — move to the first
    // save that does. Prefer saves attributed to the target profile; an
    // unattributed (null persistentProfileId) save is only a fallback so it
    // cannot shadow the profile's own saves in global sort order. The
    // selectedPath exemption is intentionally absent (we have already
    // established the current save is the wrong profile).
    final attributed = state.saves.where(
      (s) => s.persistentProfileId == profileId,
    );
    final unattributed = state.saves.where(
      (s) => s.persistentProfileId == null,
    );
    final candidate = attributed.isNotEmpty
        ? attributed.first
        : (unattributed.isNotEmpty ? unattributed.first : null);

    if (candidate != null) {
      await _inspect(candidate.path);
    } else {
      state = state.copyWith(
        selectedPath: null,
        clearInspection: true,
        clearBackups: true,
        clearPendingEdits: true,
      );
    }
  }

  Future<Map<String, Object?>> _execute(
    String command, {
    Map<String, Object?> payload = const {},
  }) {
    final pending = _coreQueue.then(
      (_) => _core.execute(command, payload: payload),
    );
    // Keep the queue alive regardless of this command's success/failure.
    _coreQueue = pending.then((_) {}, onError: (_) {});
    return pending;
  }

  // ---------------------------------------------------------------------------
  // Pending-edit registry
  // ---------------------------------------------------------------------------

  /// Upsert a pending edit for a given editor surface key.
  void setPendingEdit(String key, PendingSaveEdit edit) {
    final updated = Map<String, PendingSaveEdit>.from(state.pendingEdits);
    updated[key] = edit;
    state = state.copyWith(pendingEdits: updated);
  }

  /// Remove the pending edit for a given editor surface key.
  void clearPendingEdit(String key) {
    if (!state.pendingEdits.containsKey(key)) return;
    final updated = Map<String, PendingSaveEdit>.from(state.pendingEdits);
    updated.remove(key);
    state = state.copyWith(pendingEdits: updated);
  }

  /// Upsert the pending difficulty edit. Called by the Difficulty card as its
  /// controls change (or a propagation box is ticked). Participates in
  /// [EditorState.hasUnsavedEdits], [EditorState.pendingEditCount] (so the
  /// global Save/Reset reflect it), and is written by [saveAllPending].
  void setPendingDifficulty(PendingDifficulty edit) {
    state = state.copyWith(pendingDifficulty: edit);
  }

  /// Clear the pending difficulty edit (e.g. the card reverts to the stored
  /// value with no propagation requested).
  void clearPendingDifficulty() {
    if (state.pendingDifficulty == null) return;
    state = state.copyWith(pendingDifficulty: null);
  }

  /// Clear all pending edits (registry and the pending difficulty edit).
  void clearAllPendingEdits() {
    if (state.pendingEdits.isEmpty && state.pendingDifficulty == null) return;
    state = state.copyWith(clearPendingEdits: true);
  }

  /// Save all pending edits: the slot edits in one `write_save` and the pending
  /// difficulty edit in one `write_difficulty`. When both exist, both core
  /// writes run sequentially against the current save's `.sav` (each re-reads
  /// the file, so order is safe — slot write_save first, then write_difficulty),
  /// then the state refreshes ONCE at the end. No-op when nothing is pending.
  /// Re-entry-safe: bails immediately if a load is already in flight.
  /// Returns true on success (or when nothing to save), false on failure.
  Future<bool> saveAllPending() async {
    final difficultyEdit = state.pendingDifficulty;
    if (state.pendingEdits.isEmpty && difficultyEdit == null) return true;
    if (state.isLoading) return false;
    final savePath = state.selectedPath;
    if (savePath == null) return false;

    // Snapshot the keys in stable (sorted) order for determinism. We clear
    // exactly these keys on success rather than using clearAllPendingEdits()
    // so that an edit typed during the in-flight write (which lives only in
    // widget-local text until onChanged fires again) isn't silently discarded
    // by a subsequent refresh-clears-all; the refresh's central clear will
    // wipe those mid-write registry entries anyway, but the snapshot-key
    // path is the explicit safety net for any failed-then-refreshed scenarios.
    final snapshotKeys = state.pendingEdits.keys.toList()..sort();
    final allEdits = <Map<String, Object?>>[];
    var syncPersistent = false;
    for (final key in snapshotKeys) {
      final entry = state.pendingEdits[key]!;
      allEdits.addAll(entry.edits);
      if (entry.syncPersistentDataList) syncPersistent = true;
    }

    // The same typed property can be edited from two surfaces at once (the
    // Player tab's hero stats and the All data browser). Batching both would
    // silently let sorted-key order pick the winner — refuse instead and let
    // the user resolve the conflict.
    final seenTypedPaths = <String>{};
    for (final edit in allEdits) {
      if (edit['path'] != 'private.typed.setValue') continue;
      final value = edit['value'];
      if (value is! Map) continue;
      final path = (value['path'] as List?)?.join(' › ') ?? '';
      if (!seenTypedPaths.add(path)) {
        state = state.copyWith(
          error:
              'Conflicting unsaved edits target the same property '
              '($path) from two tabs. Reset or revert one of them, '
              'then save again.',
        );
        return false;
      }
    }

    // A structural inventory edit (addItem/removeItem) splices the MainContainer
    // slot array and shifts every byte after the splice point. The core applies
    // a batch in one write round, so ANY other edit in the same write is unsafe:
    // a typed edit re-resolves a now-shifted array index, and an in-place
    // setItemCount patches byte offsets that the splice invalidates. Require a
    // structural inventory edit to be the only edit in the write.
    final hasStructuralInventory = allEdits.any(
      (edit) =>
          edit['path'] == 'private.inventory.addItem' ||
          edit['path'] == 'private.inventory.removeItem',
    );
    if (hasStructuralInventory && allEdits.length > 1) {
      state = state.copyWith(
        error:
            'An inventory add or remove shifts item indices, so it must be '
            'saved on its own. Save or reset your other unsaved changes first, '
            'then save the inventory change.',
      );
      return false;
    }

    // Difficulty rewrites the private payload of each target save, so it needs
    // a compress-ready codec — surface the same error the other private writes
    // use rather than silently skipping it.
    if (difficultyEdit != null && !state.codecCompressReady) {
      state = state.copyWith(
        error:
            'Saving difficulty needs a verified G1R codec host '
            '(it rewrites the private payload of each targeted save).',
      );
      return false;
    }

    final difficultyPayload = difficultyEdit == null
        ? null
        : _buildDifficultyPayload(difficultyEdit);

    final n = allEdits.length;
    // Outcome tracking. The two core commands write the same .sav independently,
    // so there is no atomic cross-command rollback: if write_save commits but
    // write_difficulty then fails, the slot bytes are ALREADY on disk. We must
    // therefore converge editor state with disk (no divergence) rather than
    // pretend nothing happened — see the convergence block after the writes.
    var slotEditsCommitted = false; // write_save succeeded (or no slot edits)
    var difficultyCommitted = false; // write_difficulty succeeded
    String? difficultyError; // set when write_difficulty failed after a slot commit
    var ok = false;
    await _withLoading(() async {
      final messages = <String>[];

      // 1) Slot edits (write_save), if any.
      if (allEdits.isNotEmpty) {
        final response = await _execute(
          'write_save',
          payload: {
            'path': savePath,
            'backup': true,
            if (syncPersistent) 'syncPersistentDataList': true,
            'edits': allEdits,
            ..._codecPayload(),
          },
        );
        if (response['ok'] != true) {
          // write_save failed: nothing committed. Keep ALL pending edits.
          state = state.copyWith(error: _errorMessage(response));
          return;
        }
        final data = (response['data'] as Map).cast<String, Object?>();
        messages.add(
          _backupMessage('$n change${n == 1 ? '' : 's'} saved with backup', data),
        );
      }
      // Either there were no slot edits, or write_save succeeded — the slot
      // edits in the snapshot are now on disk.
      slotEditsCommitted = true;

      // 2) Difficulty (write_difficulty), if pending. Each write re-reads the
      // file, so running it after the slot write is safe.
      if (difficultyPayload != null) {
        try {
          final response = await _execute(
            'write_difficulty',
            payload: difficultyPayload,
          );
          if (response['ok'] != true) {
            // The slot edits are already committed to disk, but the difficulty
            // write failed. Record the error and fall through to refresh +
            // converge: we must NOT keep the committed slot edits pending.
            difficultyError = _errorMessage(response);
          } else {
            difficultyCommitted = true;
            final data = (response['data'] as Map).cast<String, Object?>();
            final targetsList = (difficultyPayload['targets'] as Map);
            final saveTargets = (targetsList['saves'] as List?)?.length ?? 1;
            final profileTarget = targetsList.containsKey('profile') ? 1 : 0;
            final written =
                (data['targetsWritten'] as num?)?.toInt() ??
                (saveTargets + profileTarget);
            messages.add(
              written == 0
                  ? 'No difficulty changes to write'
                  : 'Difficulty written to $written '
                        'target${written == 1 ? '' : 's'} (backup created)',
            );
          }
        } catch (error) {
          // A THROWN difficulty dispatch (e.g. malformed native JSON from the
          // core) must be treated as a difficulty failure for convergence, not
          // swallowed by _withLoading's generic catch — otherwise difficultyError
          // stays null and the convergence branch would clearPendingDifficulty(),
          // silently discarding an edit that never landed on disk. Setting
          // difficultyError (with difficultyCommitted false) routes us to the
          // "keep pendingDifficulty + surface error" branch.
          difficultyError = 'Unexpected error: $error';
        }
      }

      // Report the slot/difficulty messages that did land, then refresh ONCE so
      // the re-inspect reflects exactly what is on disk. The error (if any) is
      // re-applied after refresh, since refresh clears it.
      if (messages.isNotEmpty) {
        state = state.copyWith(lastWriteMessage: messages.join('; '));
      }
      await refresh();
      // Overall success only when nothing failed.
      ok = difficultyError == null;
    });

    // Converge editor state with disk. After this returns, the pending set
    // reflects exactly what is NOT yet on disk.
    if (slotEditsCommitted) {
      // The snapshot slot edits ARE on disk now — clear them regardless of the
      // difficulty outcome. Leaving them pending is the divergence bug (the
      // editor would still think on-disk changes are unsaved, and a retry would
      // double-apply). refresh()'s _inspect already clears all pending on a
      // successful re-inspect, but a failed re-inspect keeps them — so clear the
      // committed keys explicitly here to guarantee convergence either way.
      for (final key in snapshotKeys) {
        clearPendingEdit(key);
      }
      if (difficultyCommitted) {
        // Difficulty is on disk too — nothing left pending for it.
        clearPendingDifficulty();
      } else if (difficultyError != null) {
        // Slot edits landed but difficulty did NOT. Keep ONLY the difficulty
        // edit pending so the user can retry just the difficulty, and surface a
        // clear, honest error. Re-assert the pending difficulty in case the
        // post-write refresh re-seeded/cleared it.
        if (difficultyEdit != null) {
          state = state.copyWith(pendingDifficulty: difficultyEdit);
        }
        // Only claim the slot changes were saved when there actually were any —
        // a difficulty-only save that fails has nothing committed to report.
        state = state.copyWith(
          error: allEdits.isEmpty
              ? 'Difficulty failed: $difficultyError'
              : 'Slot changes saved, but difficulty failed: $difficultyError',
        );
      } else {
        // No difficulty edit was pending — slot-only save. Nothing else to do.
        clearPendingDifficulty();
      }
    }
    return ok;
  }

  Future<void> chooseSaveDir() async {
    final selected = await getDirectoryPath(
      confirmButtonText: 'Use folder',
      initialDirectory: state.saveDir,
    );
    if (selected == null) return;
    await setSaveDir(selected);
  }

  Future<void> chooseCodecHost() async {
    final selected = await openFile(
      confirmButtonText: 'Use helper',
      initialDirectory: _existingParent(state.codecHostPath),
      acceptedTypeGroups: const [
        XTypeGroup(label: 'Windows executable', extensions: ['exe']),
      ],
    );
    if (selected == null) return;
    await setCodecHostPath(selected.path);
  }

  Future<void> chooseGameExe() async {
    final selected = await openFile(
      confirmButtonText: 'Use game EXE',
      initialDirectory: _existingParent(state.gameExePath),
      acceptedTypeGroups: const [
        XTypeGroup(label: 'Windows executable', extensions: ['exe']),
      ],
    );
    if (selected == null) return;
    await setGameExePath(selected.path);
  }

  Future<void> setSaveDir(String value) async {
    state = state.copyWith(
      saveDir: value,
      // Drop the previous folder's slots/selection up front so a failed scan
      // can't leave the sidebar showing the old folder under the new path.
      saves: const [],
      profiles: const [],
      selectedPath: null,
      activeProfileId: null,
      selectedProfileId: null,
      clearInspection: true,
      clearBackups: true,
    );
    _persistSettings();
    await refresh();
  }

  Future<void> setCodecHostPath(String value) async {
    state = state.copyWith(codecHostPath: value, clearError: true);
    _persistSettings();
    await checkCodec();
  }

  Future<void> setGameExePath(String value) async {
    state = state.copyWith(gameExePath: value, clearError: true);
    _persistSettings();
    await checkCodec();
  }

  Future<void> refresh() async {
    final seq = ++_loadSeq;
    _loadStarted();
    state = state.copyWith(isLoading: true, clearError: true);
    try {
      final response = await _execute(
        'scan_save_dir',
        payload: {'path': state.saveDir, ..._codecPayload()},
      );
      if (seq != _loadSeq) return;
      if (response['ok'] != true) {
        state = state.copyWith(error: _errorMessage(response));
        return;
      }
      final data = (response['data'] as Map?)?.cast<String, Object?>();
      final rawSaves = (data?['saves'] as List?) ?? const [];
      final saves = rawSaves
          .whereType<Map>()
          .map((m) => SaveSlot.fromJson(m.cast<String, Object?>()))
          .toList();
      _sortByPlaytimeDesc(saves);
      final rawProfiles = (data?['profiles'] as List?) ?? const [];
      final profiles = rawProfiles
          .whereType<Map>()
          .map((m) => ProfileSummary.fromJson(m.cast<String, Object?>()))
          .toList();
      final activeProfileId = (data?['activeProfileId'] as num?)?.toInt();
      // Keep the explicit profile selection if that profile still exists in
      // the new scan result, otherwise reset it to null.
      final profileIds = profiles.map((p) => p.profileId).toSet();
      final keptSelectedProfileId =
          (state.selectedProfileId != null &&
              profileIds.contains(state.selectedProfileId))
          ? state.selectedProfileId
          : null;

      // When the explicit selection was reset, fall back to any visible save;
      // otherwise restrict to the still-valid profile's visible saves.
      final newState = state.copyWith(
        saves: saves,
        profiles: profiles,
        activeProfileId: activeProfileId,
        selectedProfileId: keptSelectedProfileId,
      );
      // Compute visible saves with the updated state fields to find a
      // sensible first selection path when the folder or profile changed.
      final visibleAfterRefresh = newState.visibleSaves;
      final selectedPath =
          visibleAfterRefresh.any((s) => s.path == state.selectedPath)
          ? state.selectedPath
          : (visibleAfterRefresh.isNotEmpty
                ? visibleAfterRefresh.first.path
                : null);
      // Pending edits are cleared by _inspect once the fresh inspection
      // actually lands (so a failed re-inspect keeps them retryable); only
      // when nothing remains selected is there no inspect to do it.
      state = newState.copyWith(
        selectedPath: selectedPath,
        clearInspection: selectedPath == null,
        clearBackups: selectedPath == null,
        clearPendingEdits: selectedPath == null,
      );
      if (selectedPath != null) {
        await _inspect(selectedPath);
      }
    } catch (error) {
      // A thrown core call (e.g. invalid/null native JSON) must surface as an
      // in-app error, not just an async console error.
      if (seq == _loadSeq) {
        state = state.copyWith(error: 'Failed to scan saves: $error');
      }
    } finally {
      _loadFinished();
    }
  }

  Future<void> inspect(String path) async {
    await _inspect(path, clearWriteMessage: true);
  }

  Future<void> _inspect(String path, {bool clearWriteMessage = false}) async {
    final seq = ++_loadSeq;
    // Switching slots: drop the previous slot's inspection/backups so the panes
    // don't keep showing stale data while the new load runs.
    final switchingSlot = state.selectedPath != path;
    _loadStarted();
    state = state.copyWith(
      selectedPath: path,
      isLoading: true,
      clearError: true,
      clearWriteMessage: clearWriteMessage,
      clearInspection: switchingSlot,
      clearBackups: switchingSlot,
      // Slot switch: stale edits must never be written into a different
      // file, so drop them immediately. Same-save re-inspects clear pending
      // only once the fresh inspection lands (below) — if the inspect fails,
      // fields still show the drafts and the registry must keep matching
      // them so the user can retry the save.
      clearPendingEdits: switchingSlot,
    );
    try {
      final payload = <String, Object?>{
        'path': path,
        'includePrivate': true,
        ..._codecPayload(),
      };
      final response = await _execute('inspect_save', payload: payload);
      // Only the latest load applies results. Core calls are serialized, so a
      // superseded load always finishes before the newer one; bailing here
      // prevents it from applying stale data over the fresher load.
      if (seq != _loadSeq) return;
      if (response['ok'] != true) {
        state = state.copyWith(
          error: _errorMessage(response),
          clearInspection: true,
          clearBackups: true,
        );
        return;
      }
      final data = (response['data'] as Map).cast<String, Object?>();
      // Apply the parsed inspection immediately so a later list_backups failure
      // does not drop the save metadata/private views that already loaded.
      // The fresh inspection re-seeds every editor, so pending edits are
      // discarded in the same state change — never earlier (see above).
      state = state.copyWith(
        inspection: SaveInspection.fromJson(data),
        // The fresh inspection re-seeds every editor, so discard all pending
        // edits — including any pending difficulty edit, which clearPendingEdits
        // also clears. The card re-seeds its controls from the new inspection's
        // stored difficulty.
        clearPendingEdits: true,
      );
      final backupSnapshot = await _loadBackups(path, seq);
      if (backupSnapshot == null) return;
      state = state.copyWith(
        backups: backupSnapshot.backups,
        companionBackups: backupSnapshot.companionBackups,
      );
    } catch (error) {
      if (seq == _loadSeq) {
        state = state.copyWith(
          error: 'Failed to inspect save: $error',
          clearInspection: true,
          clearBackups: true,
        );
      }
    } finally {
      _loadFinished();
    }
  }

  Future<void> refreshBackups() async {
    final path = state.selectedPath;
    if (path == null) return;
    final seq = ++_loadSeq;
    _loadStarted();
    state = state.copyWith(isLoading: true, clearError: true);
    try {
      final backupSnapshot = await _loadBackups(path, seq);
      if (backupSnapshot == null) return;
      state = state.copyWith(
        backups: backupSnapshot.backups,
        companionBackups: backupSnapshot.companionBackups,
      );
    } catch (error) {
      if (seq == _loadSeq) {
        state = state.copyWith(error: 'Failed to load backups: $error');
      }
    } finally {
      _loadFinished();
    }
  }

  Future<void> restoreBackup(String backupPath) async {
    final path = state.selectedPath;
    if (path == null) return;
    await _withLoading(() async {
      final response = await _execute(
        'restore_backup',
        payload: {'path': path, 'backupPath': backupPath},
      );
      if (response['ok'] != true) {
        state = state.copyWith(error: _errorMessage(response));
        return;
      }
      final data = (response['data'] as Map?)?.cast<String, Object?>();
      final companionPresent = data?['persistentCompanionPresent'] == true;
      final companionRestored = data?['persistentRestoredFrom'] != null;
      final restoreMessage = companionPresent && !companionRestored
          ? 'Restored backup: $backupPath (PersistentDataList.sav left unchanged '
                '— no matching companion backup; slot metadata may differ)'
          : 'Restored backup: $backupPath';
      state = state.copyWith(lastWriteMessage: restoreMessage);
      // Rescan so the sidebar/profile summary reflect the rolled-back public
      // name and PersistentDataList metadata, not just the detail pane.
      // refresh() also centrally clears all pending edits (avoids mutating
      // the provider from widget lifecycle hooks).
      await refresh();
      // The restore itself succeeded on disk; if the follow-up rescan/inspection
      // failed, make clear the restore worked so the error is not misread as a
      // failed restore.
      if (state.error != null) {
        state = state.copyWith(
          error:
              'Restored backup: $backupPath, but reloading the save failed: ${state.error}',
        );
      }
    });
  }

  Future<void> checkCodec() async {
    try {
      final response = await _execute('check_codec', payload: _codecPayload());
      if (response['ok'] != true) {
        // Use the dedicated codec error channel so a concurrent/later refresh
        // does not wipe this message, and drop the now-stale codec status so
        // the UI doesn't keep showing an earlier "ready" state.
        state = state.copyWith(
          codecError: _errorMessage(response),
          clearCodecStatus: true,
          codecVerified: false,
        );
        return;
      }
      final data = (response['data'] as Map).cast<String, Object?>();
      final status = CodecStatus.fromJson(data);
      // A fresh probe supersedes any earlier manual verification (the host
      // path or executable may have changed), so drop the session flag and let
      // the new probe — or a new verification — decide compress readiness.
      state = state.copyWith(
        codecStatus: status,
        codecVerified: false,
        clearCodecError: true,
      );
      // Re-decode the selected save now the codec is available — but only if no
      // load is already running. An in-flight inspect is already the latest load
      // and will populate; spawning another here would just race it.
      if (status.available && state.selectedPath != null && _activeLoads == 0) {
        await inspect(state.selectedPath!);
      }
    } catch (error) {
      // checkCodec is fire-and-forget from the constructor; a thrown core call
      // must surface in UI state, not as an unhandled async error.
      state = state.copyWith(
        codecError: 'Codec check failed: $error',
        clearCodecStatus: true,
        codecVerified: false,
      );
    }
  }

  /// Verify the configured codec by round-tripping a real private chunk from
  /// the selected save through decompress → compress → decompress in the game
  /// executable. On success, compression-dependent edits unlock for the session
  /// even when the probe could not auto-trust the build (pattern-resolved /
  /// unknown executable). This is the manual bridge for non-known game builds.
  Future<void> verifyCodec() async {
    final path = state.selectedPath;
    if (path == null) return;
    await _withLoading(() async {
      final response = await _execute(
        'validate_codec_roundtrip',
        payload: {'path': path, ..._codecPayload()},
      );
      if (response['ok'] != true) {
        state = state.copyWith(
          codecVerified: false,
          codecError: _errorMessage(response),
        );
        return;
      }
      state = state.copyWith(
        codecVerified: true,
        lastWriteMessage: 'Codec verified — private edits unlocked',
        clearCodecError: true,
      );
    });
  }

  Future<void> validateCodecRoundtrip() async {
    final path = state.selectedPath;
    if (path == null) return;
    await _withLoading(() async {
      final response = await _execute(
        'validate_codec_roundtrip',
        payload: {'path': path, ..._codecPayload()},
      );
      if (response['ok'] != true) {
        state = state.copyWith(error: _errorMessage(response));
        return;
      }
      final data = (response['data'] as Map).cast<String, Object?>();
      state = state.copyWith(
        lastWriteMessage:
            'Codec roundtrip passed: chunk ${data['chunkIndex']} recompressed to ${data['recompressedSize']} bytes',
      );
    });
  }

  /// Search every typed property in the decoded private payload. The core
  /// caches the decoded payload, so the first search pays the decode cost and
  /// later searches are instant. Returns a result carrying an error string
  /// instead of throwing, so the browser UI can render it inline.
  Future<TypedSearchResult> searchTypedProperties(
    String query, {
    int offset = 0,
    int limit = 50,
  }) async {
    final path = state.selectedPath;
    if (path == null) {
      return const TypedSearchResult(error: 'No save selected.');
    }
    try {
      final response = await _execute(
        'search_typed_properties',
        payload: {
          'path': path,
          'query': query,
          'offset': offset,
          'limit': limit,
          ..._codecPayload(),
        },
      );
      if (response['ok'] != true) {
        return TypedSearchResult(error: _errorMessage(response));
      }
      return TypedSearchResult.fromJson(
        (response['data'] as Map).cast<String, Object?>(),
      );
    } catch (error) {
      return TypedSearchResult(error: 'Property search failed: $error');
    }
  }

  /// Search query that returns exactly the hero attribute leaves: both terms
  /// must appear in the display path, which only holds for entries under
  /// AttributesByGlobalId/{Hero}.
  static const heroAttributesQuery = 'AttributesByGlobalId {Hero}';

  /// Load every hero gameplay attribute from the typed property tree. The
  /// core caps each search page at 1000 hits, so page through the full match
  /// set instead of trusting one request. The decode cache is already seeded
  /// by inspect, so this does not pay a second full private-payload decode.
  Future<HeroAttributesResult> loadHeroAttributes() async {
    // Pin the save under load: searchTypedProperties always reads the
    // current selection, so a save switch mid-pagination would silently
    // merge pages from two different files into one stat list.
    final loadPath = state.selectedPath;
    final hits = <TypedPropertyHit>[];
    var offset = 0;
    while (true) {
      final result = await searchTypedProperties(
        heroAttributesQuery,
        offset: offset,
        limit: 1000,
      );
      if (state.selectedPath != loadPath) {
        return const HeroAttributesResult(
          error: 'Save selection changed while loading hero attributes.',
        );
      }
      if (result.error != null) {
        return HeroAttributesResult(error: result.error);
      }
      hits.addAll(result.results);
      offset += result.results.length;
      if (offset >= result.total || result.results.isEmpty) break;
    }
    return HeroAttributesResult(attributes: parseHeroAttributes(hits));
  }

  /// Run one progression section query. Returns the raw data map, or null
  /// with [onError] called, so each typed loader below can build its own page
  /// object with an inline error.
  Future<Map<String, Object?>?> _queryProgression(
    Map<String, Object?> params, {
    required void Function(String message) onError,
  }) async {
    final path = state.selectedPath;
    if (path == null) {
      onError('No save selected.');
      return null;
    }
    try {
      final response = await _execute(
        'query_progression',
        payload: {'path': path, ...params, ..._codecPayload()},
      );
      if (response['ok'] != true) {
        onError(_errorMessage(response));
        return null;
      }
      return (response['data'] as Map).cast<String, Object?>();
    } catch (error) {
      onError('Progression query failed: $error');
      return null;
    }
  }

  Future<ProgressionQuestPage> loadProgressionQuests({
    String query = '',
    int offset = 0,
    int limit = 100,
    String? state,
    String? group,
  }) async {
    String? error;
    final data = await _queryProgression({
      'section': 'quests',
      'query': query,
      'offset': offset,
      'limit': limit,
      if (state != null && state.isNotEmpty) 'state': state,
      if (group != null && group.isNotEmpty) 'group': group,
    }, onError: (message) => error = message);
    if (data == null) return ProgressionQuestPage(error: error);
    return ProgressionQuestPage.fromJson(data);
  }

  Future<KnowledgeCharactersPage> loadKnowledgeCharacters({
    String query = '',
    int offset = 0,
    int limit = 100,
  }) async {
    String? error;
    final data = await _queryProgression({
      'section': 'knowledge',
      'query': query,
      'offset': offset,
      'limit': limit,
    }, onError: (message) => error = message);
    if (data == null) return KnowledgeCharactersPage(error: error);
    return KnowledgeCharactersPage.fromJson(data);
  }

  Future<KnowledgeEntriesPage> loadKnowledgeEntries(
    String character, {
    String query = '',
    int offset = 0,
    int limit = 200,
  }) async {
    String? error;
    final data = await _queryProgression({
      'section': 'knowledge',
      'character': character,
      'query': query,
      'offset': offset,
      'limit': limit,
    }, onError: (message) => error = message);
    if (data == null) return KnowledgeEntriesPage(error: error);
    return KnowledgeEntriesPage.fromJson(data);
  }

  Future<MemoryCharactersPage> loadMemoryCharacters({
    String query = '',
    int offset = 0,
    int limit = 100,
  }) async {
    String? error;
    final data = await _queryProgression({
      'section': 'events',
      'query': query,
      'offset': offset,
      'limit': limit,
    }, onError: (message) => error = message);
    if (data == null) return MemoryCharactersPage(error: error);
    return MemoryCharactersPage.fromJson(data);
  }

  Future<MemoryEventsPage> loadMemoryEvents(
    String character, {
    String query = '',
    int offset = 0,
    int limit = 100,
  }) async {
    String? error;
    final data = await _queryProgression({
      'section': 'events',
      'character': character,
      'query': query,
      'offset': offset,
      'limit': limit,
    }, onError: (message) => error = message);
    if (data == null) return MemoryEventsPage(error: error);
    return MemoryEventsPage.fromJson(data);
  }

  /// Apply one structural progression edit (event remove/duplicate)
  /// immediately, with backup. Index-addressed array edits must go one per
  /// write round — indices shift after every structural change — so this is
  /// intentionally not part of the pending-edit registry.
  Future<bool> applyMemoryEventEdit(MemoryEventEdit edit) async {
    final savePath = state.selectedPath;
    if (savePath == null) {
      state = state.copyWith(error: 'No save selected.');
      return false;
    }
    if (state.isLoading) {
      state = state.copyWith(
        error: 'Another operation is in progress — try again when it finishes.',
      );
      return false;
    }
    // Guard on hasUnsavedEdits (pendingEdits OR a pending difficulty), matching
    // selectProfile/refresh: removing or duplicating a memory event writes the
    // file immediately and its success path re-seeds the editors, which would
    // silently discard a pending difficulty edit just as it would pending edits.
    if (state.hasUnsavedEdits) {
      state = state.copyWith(
        error:
            'Save or reset your unsaved changes first — removing or '
            'duplicating a memory event writes the file immediately and '
            'would discard them.',
      );
      return false;
    }
    return _runWrite(
      payload: {
        'path': savePath,
        'backup': true,
        'edits': [edit.toEditJson()],
        ..._codecPayload(),
      },
      message: (data) => _backupMessage(
        edit.isRemove ? 'Memory event removed' : 'Memory event duplicated',
        data,
      ),
    );
  }

  String _errorMessage(Map<String, Object?> response) {
    final error = (response['error'] as Map?)?.cast<String, Object?>();
    return error?['message'] as String? ?? 'Unknown core error';
  }

  String _backupMessage(String prefix, Map<String, Object?> data) {
    final backupPath = data['backupPath'] ?? 'none';
    final persistentBackupPath = data['persistentBackupPath'] as String?;
    if (persistentBackupPath == null || persistentBackupPath.isEmpty) {
      return '$prefix: $backupPath';
    }
    return '$prefix: $backupPath; PersistentDataList backup: $persistentBackupPath';
  }

  Future<_BackupSnapshot?> _loadBackups(String path, int seq) async {
    final response = await _execute('list_backups', payload: {'path': path});
    // Only the latest load applies; a superseded load must not replace the
    // fresher list with its outdated result.
    if (seq != _loadSeq) return null;
    if (response['ok'] != true) {
      // Leave isLoading to the caller's load-counter bookkeeping.
      state = state.copyWith(error: _errorMessage(response));
      return null;
    }
    final data = (response['data'] as Map?)?.cast<String, Object?>();
    final rawBackups = (data?['backups'] as List?) ?? const [];
    final rawCompanionBackups =
        (data?['companionBackups'] as List?) ?? const [];
    final backups = rawBackups
        .whereType<Map>()
        .map((value) => BackupEntry.fromJson(value.cast<Object?, Object?>()))
        .toList();
    final companionBackups = rawCompanionBackups
        .whereType<Map>()
        .map((value) => BackupEntry.fromJson(value.cast<Object?, Object?>()))
        .toList();
    return _BackupSnapshot(
      backups: backups,
      companionBackups: companionBackups,
    );
  }

  Map<String, Object?> _codecPayload() {
    final helperPath = state.codecHostPath.trim();
    final exePath = state.gameExePath.trim();
    if (helperPath.isEmpty || exePath.isEmpty) return const {};
    return {
      'binaryHost': {'helperPath': helperPath, 'exePath': exePath},
    };
  }

  String? _existingParent(String path) {
    if (path.trim().isEmpty) return null;
    final parent = p.dirname(path);
    return parent == '.' ? null : parent;
  }

  void _persistSettings() {
    _settingsStore.write(
      EditorSettings(
        saveDir: state.saveDir,
        codecHostPath: state.codecHostPath,
        gameExePath: state.gameExePath,
      ),
    );
  }

  static EditorState _initialState({
    required String? saveDir,
    required String? codecHostPath,
    required String? gameExePath,
    required EditorSettingsStore settingsStore,
  }) {
    final stored = settingsStore.read();
    return EditorState(
      saveDir: saveDir ?? stored.saveDir ?? defaultSaveRoot(),
      codecHostPath:
          codecHostPath ?? stored.codecHostPath ?? defaultCodecHostPath(),
      gameExePath: gameExePath ?? stored.gameExePath ?? defaultGameExePath(),
    );
  }
}

class _BackupSnapshot {
  const _BackupSnapshot({
    required this.backups,
    required this.companionBackups,
  });

  final List<BackupEntry> backups;
  final List<BackupEntry> companionBackups;
}
