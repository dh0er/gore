import 'dart:io';

import 'package:file_selector/file_selector.dart';
import 'package:goresave/features/editor/domain/actor.dart';
import 'package:goresave/features/editor/domain/character_index.dart';
import 'package:goresave/features/editor/domain/core_service.dart';
import 'package:goresave/features/editor/domain/editor_models.dart';
import 'package:goresave/features/editor/domain/editor_settings_store.dart';
import 'package:goresave/features/editor/domain/game_time.dart';
import 'package:goresave/features/editor/domain/hero_attributes.dart';
import 'package:goresave/features/editor/domain/npc_actors_page.dart';
import 'package:goresave/features/editor/domain/npc_attributes.dart';
import 'package:goresave/features/editor/domain/pending_edits.dart';
import 'package:goresave/features/editor/domain/progression_models.dart';
import 'package:goresave/features/editor/domain/skills_models.dart';
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
    this.error,
    this.codecError,
    this.lastWriteMessage,
    this.pendingEdits = const {},
    this.selectedActor = const Actor.player(),
    this.invalidNpcEditKey,
    this.heroGlobalId,
    this.heroGlobalIdSettled = false,
    this.saveProgress,
  });

  final String saveDir;
  final bool isLoading;

  /// Progress of an in-flight multi-step save: `done` sub-writes committed out
  /// of `total`. Non-null only while [saveAllPending] runs its sequential
  /// write_save worklist (a structural inventory add/remove gets its own write),
  /// so the overlay can show a determinate bar instead of an indeterminate
  /// spinner. Null at rest and during single ordinary loads.
  final ({int done, int total})? saveProgress;
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

  /// The actor (player or a specific NPC) the actor-aware editor tabs operate
  /// on. Shared so the attribute and inventory tabs stay in sync. Defaults to
  /// the player so existing behavior — player shown first — is unchanged.
  final Actor selectedActor;

  /// True when there are any unsaved edits. The profile-switch guard blocks on
  /// this. Difficulty is edited separately (a profile-level dialog that writes
  /// immediately) and is never part of the pending set.
  bool get hasUnsavedEdits => pendingEdits.isNotEmpty;

  /// Pending-edit key of the NPC whose attribute panel currently has an invalid
  /// (empty/non-numeric) field, or null. Its stored draft is KEPT (so switching
  /// actors does not lose earlier valid edits) but Save is blocked while set, so
  /// the now-stale stored value is never written behind an invalid field.
  final String? invalidNpcEditKey;

  /// True while an NPC attribute field is invalid — global Save is disabled.
  bool get hasInvalidNpcEdit => invalidNpcEditKey != null;

  /// GlobalId of the save's own "Hero" ACTOR row (the player's avatar),
  /// stashed when the character index loads (see
  /// [EditorNotifier.loadAllCharacters]). The pinned Player row in the
  /// Charaktere master list represents this actor; its GlobalId keys the
  /// player's memory events. Null until the index has loaded.
  final String? heroGlobalId;

  /// True once the character-index load for the CURRENT save has completed at
  /// least once — success or failure — so [heroGlobalId] is as resolved as
  /// it's going to get. The player's Ereignisse pane keys its spinner off
  /// this: null id + not settled = index load in flight (spinner); null id +
  /// settled = no hero row is coming (empty state, never an eternal spinner).
  /// Reset to false on a slot switch alongside [heroGlobalId].
  final bool heroGlobalIdSettled;

  final String? error;

  /// Compression-dependent private writes are safe when the in-process codec
  /// reports it can compress. The always-on codec reports this directly, so
  /// there is no longer a manual per-session verification step.
  bool get codecCompressReady => codecStatus?.canCompress ?? false;

  /// Error from the most recent codec check. Kept separate from [error] so a
  /// save-directory refresh does not wipe a standing codec configuration error.
  final String? codecError;
  final String? lastWriteMessage;

  /// Total number of edit objects across all pending keys, driving the global
  /// "Unsaved (N)" badge and the Save/Reset buttons.
  int get pendingEditCount =>
      pendingEdits.values.fold(0, (n, e) => n + e.edits.length);

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


  EditorState copyWith({
    String? saveDir,
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
    String? error,
    String? codecError,
    String? lastWriteMessage,
    Map<String, PendingSaveEdit>? pendingEdits,
    Actor? selectedActor,
    Object? invalidNpcEditKey = _unchanged,
    Object? heroGlobalId = _unchanged,
    bool? heroGlobalIdSettled,
    Object? saveProgress = _unchanged,
    bool clearSaveProgress = false,
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
      error: clearError ? null : error ?? this.error,
      codecError: clearCodecError ? null : codecError ?? this.codecError,
      lastWriteMessage: clearWriteMessage
          ? null
          : lastWriteMessage ?? this.lastWriteMessage,
      pendingEdits: clearPendingEdits
          ? const {}
          : pendingEdits ?? this.pendingEdits,
      selectedActor: selectedActor ?? this.selectedActor,
      // A fresh inspection re-seed (clearPendingEdits) also drops any standing
      // NPC validation block — the invalid in-progress field is gone with it.
      invalidNpcEditKey: clearPendingEdits
          ? null
          : identical(invalidNpcEditKey, _unchanged)
          ? this.invalidNpcEditKey
          : invalidNpcEditKey as String?,
      heroGlobalId: identical(heroGlobalId, _unchanged)
          ? this.heroGlobalId
          : heroGlobalId as String?,
      heroGlobalIdSettled: heroGlobalIdSettled ?? this.heroGlobalIdSettled,
      saveProgress: clearSaveProgress
          ? null
          : identical(saveProgress, _unchanged)
          ? this.saveProgress
          : saveProgress as ({int done, int total})?,
    );
  }
}

class EditorNotifier extends StateNotifier<EditorState> {
  EditorNotifier(
    this._core, {
    String? saveDir,
    EditorSettingsStore? settingsStore,
  }) : _settingsStore = settingsStore ?? const NoopEditorSettingsStore(),
       super(
         _initialState(
           saveDir: saveDir,
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

  /// Write difficulty into the active profile's `PersistentDataList.sav`.
  ///
  /// This is the ONLY difficulty write the app performs: the profile copy is
  /// the authoritative, profile-wide value — editing a save's own copy has no
  /// in-game effect, so the per-save write path was removed. The change applies
  /// to every save in the profile. [difficulty] is the same map shape the core's
  /// `write_difficulty` expects (`preset`, optional `combat`/`resources`/
  /// `progression`, `flowHelper`, `permadeath`). No codec host is needed —
  /// `PersistentDataList.sav` is a plain GVAS file with no compressed stream.
  /// Returns true on success; on failure sets `state.error` and returns false.
  Future<bool> writeProfileDifficulty({
    required int profileId,
    required Map<String, Object?> difficulty,
  }) {
    // Re-entry guard: bail if a load is already in flight (a rescan or another
    // write), so this write + refresh cannot interleave editor-state updates
    // with that work — mirrors saveAllPending / applyMemoryEventEdit. Set an
    // explicit error so the dialog explains why rather than showing a generic
    // failure.
    if (state.isLoading) {
      state = state.copyWith(
        error: 'Another operation is in progress. Try again in a moment.',
      );
      return Future.value(false);
    }
    // Refuse while slot edits are pending: this write runs _runWrite -> refresh,
    // and the same-save _inspect clears the pending registry — silently
    // discarding those drafts even though no write_save ran for them. Make the
    // user save or reset them first.
    if (state.hasUnsavedEdits) {
      state = state.copyWith(
        error:
            'You have unsaved save edits. Save or reset them before changing '
            'the profile difficulty.',
      );
      return Future.value(false);
    }
    final dir = state.saveDir;
    if (dir.isEmpty) {
      state = state.copyWith(error: 'No save folder selected.');
      return Future.value(false);
    }
    // `dir` carries the on-disk style of the save folder (Windows-style for
    // these saves even on a POSIX host). Pick a path Context matching that style
    // so join() stays correct on any host (a POSIX host's p.join would otherwise
    // mangle a Windows save path).
    final isWindowsStyle =
        dir.contains('\\') || RegExp(r'^[A-Za-z]:').hasMatch(dir);
    final ctx = isWindowsStyle ? p.Context(style: p.Style.windows) : p.posix;
    final payload = <String, Object?>{
      'difficulty': difficulty,
      'targets': {
        'profile': {
          'path': ctx.join(dir, 'PersistentDataList.sav'),
          'profileId': profileId,
        },
      },
      'backup': true,
    };
    return _runWrite(
      command: 'write_difficulty',
      payload: payload,
      message: (data) {
        final written = (data['targetsWritten'] as num?)?.toInt() ?? 0;
        return written == 0
            ? 'No difficulty changes to write'
            : 'Difficulty written to the profile (backup created)';
      },
    );
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

  /// The current error message, if any. Lets a modal (e.g. the difficulty
  /// dialog) read a just-failed write's error without reaching into `state`.
  String? get lastError => state.error;

  /// Whether there are unsaved edits. Lets UI guards check the live value
  /// without reaching into the protected `state`.
  bool get hasUnsavedEdits => state.hasUnsavedEdits;

  /// The currently selected save path. Lets async UI callbacks read the live
  /// value without reaching into the protected `state`.
  String? get selectedPath => state.selectedPath;

  /// The pending edit registered under [key], or null. Lets UI surfaces
  /// rehydrate their local draft from a previously-registered per-actor entry
  /// (e.g. a per-NPC inventory/attribute draft kept across an actor switch)
  /// without reaching into the protected `state`.
  PendingSaveEdit? pendingEditFor(String key) => state.pendingEdits[key];

  /// The active profile's effective Resources difficulty level, normalized to
  /// one of 'Novice' | 'Gothic' | 'Hard' — used to pick the inventory-reset
  /// start-save. Falls back to 'Gothic' (the standard preset) when there is no
  /// profile or difficulty.
  ///
  /// Mirrors the difficulty dialog's authoritative display: a non-Custom preset
  /// (Novice/Gothic/Hard) LOCKS every sub-level to its implied tier, so a stale
  /// or disagreeing stored Resources class is ignored — a Hard profile always
  /// resets from the Hard save even if it carries an out-of-date `_Standard`
  /// resources class. Only a Custom preset — or a profile with no recognized
  /// preset to imply from — lets the stored Resources sub-level decide (else
  /// Gothic). Reading `resourcesLabel` first would both mis-route Novice/Hard
  /// profiles (no explicit sub-level → Gothic) and honor a stale sub-level.
  String activeResourcesLevel() {
    const known = {'Novice', 'Gothic', 'Hard'};
    final difficulty = state.activeProfile?.difficulty;
    if (difficulty == null) return 'Gothic';
    return switch (difficulty.presetLabel) {
      'Novice' => 'Novice',
      'Gothic' => 'Gothic',
      'Hard' => 'Hard',
      // Custom, or an unrecognized/absent preset: the stored Resources sub-level
      // is authoritative (a non-Custom preset returned above and locked the
      // level to its tier).
      _ =>
        known.contains(difficulty.resourcesLabel)
            ? difficulty.resourcesLabel
            : 'Gothic',
    };
  }

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

  /// Select the actor (player or a specific NPC) the actor-aware editor tabs
  /// operate on. Updates shared state so the attribute and inventory tabs
  /// rebuild against the new selection. No-op if [actor] is already selected.
  void selectActor(Actor actor) {
    if (state.selectedActor == actor) return;
    // Switching actor abandons any in-progress invalid NPC field, so drop the
    // validation block — the previous NPC's stored (valid) draft survives.
    state = state.copyWith(selectedActor: actor, invalidNpcEditKey: null);
  }

  /// Mark (`pendingKey`) or clear (`null`) the NPC attribute panel's invalid
  /// field state. While set, global Save is disabled ([EditorState.hasInvalidNpcEdit])
  /// so a now-stale stored draft is never written behind an invalid field; the
  /// stored draft itself is left intact.
  void setNpcEditInvalid(String? pendingKey) {
    if (state.invalidNpcEditKey == pendingKey) return;
    state = state.copyWith(invalidNpcEditKey: pendingKey);
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

  /// Clear all pending edits.
  void clearAllPendingEdits() {
    if (state.pendingEdits.isEmpty) return;
    state = state.copyWith(clearPendingEdits: true);
  }

  /// Save all pending slot edits in one `write_save`, then refresh ONCE.
  /// No-op when nothing is pending. Re-entry-safe: bails immediately if a load
  /// is already in flight. Returns true on success (or when nothing to save),
  /// false on failure.
  ///
  /// Difficulty is NOT part of this path — it is a profile-level edit written
  /// directly by [writeProfileDifficulty] from the profile-header dialog.
  Future<bool> saveAllPending() async {
    if (state.pendingEdits.isEmpty) return true;
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
    // Each flattened edit remembers which snapshot key it came from, so a
    // partially-successful save can clear exactly the keys whose sub-write
    // committed (and keep the rest pending for retry).
    final allEdits = <_KeyedEdit>[];
    var syncPersistent = false;
    for (final key in snapshotKeys) {
      final entry = state.pendingEdits[key]!;
      for (final edit in entry.edits) {
        allEdits.add(_KeyedEdit(key, edit));
      }
      if (entry.syncPersistentDataList) syncPersistent = true;
    }

    // The same typed property can be edited from two surfaces at once (the
    // Player tab's hero stats and the All data browser). Batching both would
    // silently let sorted-key order pick the winner — refuse instead and let
    // the user resolve the conflict.
    final seenTypedPaths = <String>{};
    for (final keyed in allEdits) {
      final edit = keyed.edit;
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

    // Splicing structural edits (addItem/removeItem, knowledge.addCharacter,
    // npc.revive) insert or remove bytes mid-payload and shift every
    // offset/index after the splice point; the core rejects a write that mixes
    // one with ANY peer edit. Mirror the core's list and give each splicing edit
    // its OWN write_save; everything else (fixed-size, in-place) batches into a
    // single trailing write. Because the core re-reads the file fresh on every
    // write_save and re-resolves symbolic paths per edit, sequential writes
    // chain safely — even two splices on the same NPC, where the second
    // re-parses the first's already-spliced tag container.
    const splicingPaths = {
      'private.inventory.addItem',
      'private.inventory.removeItem',
      'private.inventory.reset',
      'private.knowledge.addCharacter',
      'private.npc.revive',
    };
    // A skill edit can learn/unlearn — splicing the hero's ActiveEffects array —
    // and the core rejects a write that mixes it with an index-addressed edit
    // (an All-Data edit whose path steps through `[i]`), since the splice shifts
    // that index. Skill edits DO batch safely among themselves, so give all of
    // them ONE write of their own, run LAST — after the fixed batch so any
    // indexed peer resolves against the pre-splice layout first.
    const skillPath = 'private.skills.set';
    final splicing = allEdits
        .where((k) => splicingPaths.contains(k.edit['path']))
        .toList();
    final skillEdits = allEdits
        .where((k) => k.edit['path'] == skillPath)
        .toList();
    // A raw All-Data `private.typed.setValue` on an ActiveEffects `EffectSpec/Def`
    // leaf and a Skills-panel edit for the SAME actor both target that actor's
    // effect array. They cannot be sequenced safely: a skill learn/unlearn
    // SPLICES the array, so a Def edit ordered after it re-resolves its `[i]`
    // against a shifted array and retargets the wrong effect — and ordered before
    // it changes the GE class the skill edit resolves by base. Refuse only that
    // same-actor collision (like the two-tab conflict above); a hero skill edit
    // paired with an NPC's Def edit (or vice-versa) touches different arrays and
    // is safe. With no skill edit for the Def's actor the Def edit is a normal
    // fixed-size in-place write and batches as usual.
    final skillActors = <String>{
      for (final k in skillEdits) ?_skillEditActor(k.edit),
    };
    if (allEdits.any((k) {
      final actor = _activeEffectsDefActor(k.edit);
      return actor != null && skillActors.contains(actor);
    })) {
      state = state.copyWith(
        error:
            'A Skills change and an All-data edit to the same actor’s effect '
            '(ActiveEffects › EffectSpec › Def) are both queued. They cannot be '
            'saved together — reset or revert one of them, then save again.',
      );
      return false;
    }
    // A reset REPLACES the whole m_Inventory. A raw All-data private.typed.setValue
    // targeting an m_Inventory runs in the fixed batch BEFORE the reset splice, so
    // the reset would silently overwrite (discard) it while Save still reported
    // success for both. Refuse the combination (like the conflicts above) — a reset
    // and a raw inventory edit must be saved separately. Deliberately broad (any
    // m_Inventory typed edit, not just the reset's actor): a cross-actor pair is
    // rare and the worst case here is a clear "save separately" nudge, never a
    // silent overwrite.
    if (allEdits.any((k) => k.edit['path'] == 'private.inventory.reset') &&
        allEdits.any((k) => _isInventoryTypedEdit(k.edit))) {
      state = state.copyWith(
        error:
            'An inventory reset and a raw All-data edit to an inventory are both '
            'queued. The reset replaces the entire inventory and would discard the '
            'other edit — reset or revert one of them, then save again.',
      );
      return false;
    }
    final fixedBatch = allEdits
        .where(
          (k) =>
              !splicingPaths.contains(k.edit['path']) &&
              k.edit['path'] != skillPath,
        )
        .toList();

    // Build the worklist: ONE write for the fixed-size batch (if any) FIRST,
    // then one write per splicing edit. Backup is taken on the FIRST sub-write
    // only, so a Save makes exactly one pristine snapshot regardless of
    // sub-write count.
    //
    // The fixed batch leads for two reasons:
    //  - It carries syncPersistentDataList, so making it the backup-taking write
    //    means the PersistentDataList.sav companion is updated WITH a restorable
    //    companion backup (the synced write must be the one that takes backup).
    //  - A splicing npc.revive writes HP (restore→Max). Running the fixed batch
    //    first means a conflicting manual Health edit on the SAME NPC is applied
    //    BEFORE revive, so the Revive action's HP wins (last write).
    // If there is no fixed batch, the first splice takes backup:true instead.
    final worklist = <_SubWrite>[
      if (fixedBatch.isNotEmpty)
        _SubWrite(
          edits: [for (final keyed in fixedBatch) keyed.edit],
          // syncPersistentDataList keys off a public/fixed edit, so it rides the
          // fixed-size batch.
          syncPersistentDataList: syncPersistent,
        ),
      for (final keyed in splicing) _SubWrite(edits: [keyed.edit]),
      // All skill edits together, in their own trailing write: they batch safely
      // among themselves but must not share a write with an index-addressed peer
      // (see skillPath above).
      if (skillEdits.isNotEmpty)
        _SubWrite(edits: [for (final keyed in skillEdits) keyed.edit]),
    ];

    final n = allEdits.length;
    // Edit objects that committed bytes to disk, captured BEFORE the trailing
    // refresh() so we still converge even if that refresh fails. Tracked per-EDIT,
    // not per-key: one pending key can span several sequential sub-writes (e.g.
    // multiple inventory adds), so a key may be only PARTIALLY committed — if a
    // later add fails, the earlier committed adds must not drag the whole key's
    // still-unwritten edits out of the pending set. An IDENTITY set: the exact
    // edit map objects flow from the registry into the sub-writes, and two
    // distinct adds of the same item must count as two entries, never collapse.
    final committedEdits = Set<Map<String, Object?>>.identity();
    // The first (backup-taking) sub-write's response data drives the success
    // message: its `backupPath` is the one pristine snapshot for this Save.
    Map<String, Object?> firstData = const {};
    String? failureError;
    var ok = false;
    await _withLoading(() async {
      // Seed the determinate progress bar (0 of N committed). Each sequential
      // write_save below bumps `done`, so a multi-write save (e.g. several
      // inventory adds) shows real progress instead of a stuck spinner.
      state = state.copyWith(
        saveProgress: (done: 0, total: worklist.length),
      );
      try {
        for (var i = 0; i < worklist.length; i++) {
          final sub = worklist[i];
          final response = await _execute(
            'write_save',
            payload: {
              'path': savePath,
              // Backup-once: only the first sub-write snapshots the pristine file.
              'backup': i == 0,
              if (sub.syncPersistentDataList) 'syncPersistentDataList': true,
              'edits': sub.edits,
            },
          );
          if (response['ok'] != true) {
            // Stop on the first failure. Earlier sub-writes already committed.
            failureError = _errorMessage(response);
            break;
          }
          if (i == 0) {
            firstData = (response['data'] as Map?)?.cast<String, Object?>() ??
                const {};
          }
          committedEdits.addAll(sub.edits);
          state = state.copyWith(
            saveProgress: (done: i + 1, total: worklist.length),
          );
        }
        // Writes are done — drop the bar so the trailing refresh shows the plain
        // spinner (and a failure path shows its error, not a frozen bar).
        state = state.copyWith(clearSaveProgress: true);

        if (failureError == null) {
          // All sub-writes succeeded.
          state = state.copyWith(
            lastWriteMessage: _backupMessage(
              '$n change${n == 1 ? '' : 's'} saved with backup',
              firstData,
            ),
          );
          // Single trailing refresh after the last successful write.
          await refresh();
          ok = true;
          return;
        }

        // A sub-write failed AFTER an earlier one already committed bytes to disk.
        // The panes are still seeded from the pre-save inspection, so refresh to
        // show the new on-disk state — but PRESERVE the still-unsaved (uncommitted)
        // pending edits so the user can retry them. refresh() clears every pending
        // edit AND the error, so snapshot the uncommitted ones, refresh, re-apply
        // them, then re-surface the error. With nothing committed yet, the panes
        // already match disk, so skip the refresh and just surface the error.
        if (committedEdits.isNotEmpty) {
          final preserved = _pendingMinusCommitted(committedEdits);
          // Refresh from disk and restore the still-unsaved edits ATOMICALLY with
          // the new inspection — but only if we land back on the same save they
          // target. refresh() may clear/auto-switch selectedPath (this save
          // vanished, or another slot was auto-selected); the preserved edits
          // target the ORIGINAL file, so they are dropped in that case rather than
          // re-targeted at the wrong save. Restoring inside the inspection re-seed
          // (vs. re-adding afterward) means the kept-alive editors rehydrate WITH
          // them, so a preserved inventory add/remove is shown, not just counted.
          await refresh(preservedEdits: preserved, preservedForPath: savePath);
        }
        state = state.copyWith(error: failureError);
      } finally {
        // A thrown _execute (e.g. CoreWorkerException from the persistent worker
        // isolate) skips the in-loop clear above; guarantee the determinate bar
        // is dropped so a later load shows the plain spinner, not stale counts.
        if (state.saveProgress != null) {
          state = state.copyWith(clearSaveProgress: true);
        }
      }
    });

    // Converge the pending set to only the still-uncommitted edits — per EDIT, so
    // a partially-committed key keeps its unwritten edits for retry — even if the
    // refresh above never ran or threw. On success refresh() already cleared
    // everything (this is then a no-op); on a partial/failed refresh this is the
    // safety net that stops committed edits from lingering as pending.
    if (committedEdits.isNotEmpty) {
      for (final entry
          in Map<String, PendingSaveEdit>.from(state.pendingEdits).entries) {
        final remaining = entry.value.edits
            .where((e) => !committedEdits.contains(e))
            .toList();
        if (remaining.isEmpty) {
          clearPendingEdit(entry.key);
        } else if (remaining.length != entry.value.edits.length) {
          setPendingEdit(
            entry.key,
            PendingSaveEdit(
              edits: remaining,
              syncPersistentDataList: entry.value.syncPersistentDataList,
            ),
          );
        }
      }
    }
    return ok;
  }

  /// The current pending edits minus any that already committed to disk, keyed
  /// the same way, dropping keys left with nothing. Edits are matched by identity
  /// (the same objects flow from the registry into the sub-writes), so a key
  /// whose earlier sub-write committed keeps only its still-unwritten edits.
  Map<String, PendingSaveEdit> _pendingMinusCommitted(
    Set<Map<String, Object?>> committed,
  ) {
    final result = <String, PendingSaveEdit>{};
    for (final entry in state.pendingEdits.entries) {
      final remaining =
          entry.value.edits.where((e) => !committed.contains(e)).toList();
      if (remaining.isNotEmpty) {
        result[entry.key] = PendingSaveEdit(
          edits: remaining,
          syncPersistentDataList: entry.value.syncPersistentDataList,
        );
      }
    }
    return result;
  }

  Future<void> chooseSaveDir() async {
    final selected = await getDirectoryPath(
      confirmButtonText: 'Use folder',
      initialDirectory: state.saveDir,
    );
    if (selected == null) return;
    await setSaveDir(selected);
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

  /// Re-scan the save folder and re-inspect the (possibly re-selected) save.
  ///
  /// [preservedEdits] + [preservedForPath]: a partial-save retry can carry the
  /// still-uncommitted edits across the refresh. They are restored ONLY when the
  /// post-refresh selection is still [preservedForPath] (the save they target) —
  /// and atomically with the new inspection, so the editors rehydrate with them.
  /// If the save vanished or another slot was auto-selected, they are dropped.
  Future<void> refresh({
    Map<String, PendingSaveEdit>? preservedEdits,
    String? preservedForPath,
  }) async {
    final seq = ++_loadSeq;
    _loadStarted();
    state = state.copyWith(isLoading: true, clearError: true);
    try {
      final response = await _execute(
        'scan_save_dir',
        payload: {'path': state.saveDir},
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
      //
      // Do NOT pre-set selectedPath when an inspect will follow: _inspect derives
      // `switchingSlot` from `state.selectedPath != path` and must still see the
      // PREVIOUS path, so a real slot switch (the old save disappeared / the
      // folder changed) resets the actor-aware tabs to the player. Pre-setting it
      // here made switchingSlot always false on refresh, leaking a stale NPC
      // GlobalId into the newly inspected save.
      if (selectedPath == null) {
        state = newState.copyWith(
          selectedPath: null,
          clearInspection: true,
          clearBackups: true,
          clearPendingEdits: true,
        );
      } else {
        state = newState;
        await _inspect(
          selectedPath,
          // Restore the preserved partial-save edits only if we landed back on
          // the same save they target (atomic with the inspection re-seed).
          restorePendingEdits:
              (preservedForPath != null && selectedPath == preservedForPath)
              ? preservedEdits
              : null,
        );
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

  Future<void> _inspect(
    String path, {
    bool clearWriteMessage = false,
    Map<String, PendingSaveEdit>? restorePendingEdits,
  }) async {
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
      // Slot switch: the hero GlobalId belongs to the PREVIOUS save. Drop it
      // so the player's Ereignisse sub-tab never queries the old id against
      // the new file; the master list's index load re-stashes it. Its settled
      // flag resets with it — the new save's index has not completed yet.
      heroGlobalId: switchingSlot ? null : _unchanged,
      heroGlobalIdSettled: switchingSlot ? false : null,
    );
    try {
      final payload = <String, Object?>{
        'path': path,
        'includePrivate': true,
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
      // A fresh inspection re-seeds every editor; drop the cached full NPC list
      // so the next list load re-fetches against the new save state.
      _invalidateNpcCache();
      state = state.copyWith(
        inspection: SaveInspection.fromJson(data),
        // The fresh inspection re-seeds every editor, so discard all pending
        // edits — including any pending difficulty edit, which clearPendingEdits
        // also clears. The card re-seeds its controls from the new inspection's
        // stored difficulty. EXCEPTION: a same-save partial-save refresh passes
        // the preserved uncommitted edits here so they are restored IN THE SAME
        // state-apply as the new inspection — the kept-alive editors then
        // rehydrate WITH them, instead of rehydrating empty and only counting
        // (but not showing) edits re-added after the fact.
        clearPendingEdits: restorePendingEdits == null,
        pendingEdits: restorePendingEdits,
        // On a SLOT SWITCH, reset the actor-aware tabs to the player: the
        // selected NPC's GlobalId belongs to the PREVIOUS save, so keeping it
        // would make the attribute/inventory tabs run loadNpcAttributes/
        // loadNpcInventory with a stale id against the new save. On a same-save
        // refresh (after a save/reset) the selected NPC is still valid, so keep
        // it (null = unchanged) — otherwise NPC editing jumps back to Player
        // after every save.
        selectedActor: switchingSlot ? const Actor.player() : null,
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

  /// Restore the profile's `PersistentDataList.sav` from one of its companion
  /// backups (e.g. one created by a profile difficulty write). Targets the
  /// PersistentDataList.sav that sits alongside the selected save (the same
  /// directory the slot lives in), not the selected slot itself.
  Future<void> restoreCompanionBackup(String backupPath) async {
    // Restoring the PDL runs refresh(), which clears the pending slot-edit
    // registry. Those drafts are unrelated to the profile file, so block the
    // restore while they are unsaved (mirrors the profile difficulty write)
    // rather than silently discarding them.
    if (state.hasUnsavedEdits) {
      state = state.copyWith(
        error:
            'You have unsaved save edits. Save or reset them before restoring '
            'a profile backup.',
      );
      return;
    }
    final selected = state.selectedPath;
    if (selected == null) return;
    // The save paths carry the on-disk style of the save folder (Windows-style
    // even on a POSIX host), so pick a matching path Context — p.dirname on a
    // POSIX host would otherwise collapse a `C:\...` path to '.'.
    final isWindowsStyle =
        selected.contains('\\') || RegExp(r'^[A-Za-z]:').hasMatch(selected);
    final ctx = isWindowsStyle ? p.Context(style: p.Style.windows) : p.posix;
    await restoreBackup(
      backupPath,
      targetPath: ctx.join(ctx.dirname(selected), 'PersistentDataList.sav'),
    );
  }

  /// Restore a backup. [targetPath] overrides the file to restore (used for
  /// companion `PersistentDataList.sav` backups); it defaults to the selected
  /// slot.
  Future<void> restoreBackup(String backupPath, {String? targetPath}) async {
    final path = targetPath ?? state.selectedPath;
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
      // The companion-unchanged warning is only meaningful for SLOT restores.
      // When the restore target IS PersistentDataList.sav (a companion-backup
      // restore), the core reports persistentCompanionPresent (the target file
      // exists) and no separate companion — but this restore just replaced it,
      // so the warning would be misleading. Suppress it for PDL targets.
      final targetIsPdl = path.endsWith('PersistentDataList.sav');
      final restoreMessage = companionPresent && !companionRestored && !targetIsPdl
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
      final response = await _execute('check_codec');
      if (response['ok'] != true) {
        // Use the dedicated codec error channel so a concurrent/later refresh
        // does not wipe this message, and drop the now-stale codec status so
        // the UI doesn't keep showing an earlier "ready" state.
        state = state.copyWith(
          codecError: _errorMessage(response),
          clearCodecStatus: true,
        );
        return;
      }
      final data = (response['data'] as Map).cast<String, Object?>();
      final status = CodecStatus.fromJson(data);
      state = state.copyWith(codecStatus: status, clearCodecError: true);
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
      );
    }
  }

  /// Round-trip a real private chunk from the selected save through the
  /// in-process codec (decompress → compress → decompress) and report the
  /// result. Surfaces a quick confidence check for the always-on codec.
  Future<void> validateCodecRoundtrip() async {
    final path = state.selectedPath;
    if (path == null) return;
    await _withLoading(() async {
      final response = await _execute(
        'validate_codec_roundtrip',
        payload: {'path': path},
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

  /// Search query that surfaces the single world-clock leaf. Its map key is
  /// `{GameTime}`, so a plain "GameTime" query matches only this property tree.
  static const gameTimeQuery = 'GameTime';

  /// Load the world game clock — the lone `DoubleProperty` at
  /// `m_GenericData{GameTime} › CurrentTime › TotalSeconds`. Returns null when
  /// the save has no such leaf (non-GSAV, not decoded, or absent), so the
  /// Overview card can simply hide itself. The decode cache is already seeded by
  /// inspect.
  ///
  /// Pages to the leaf rather than trusting the first result page: the core
  /// caps each page at 1000 hits, and while `GameTime` matches only this tree in
  /// practice, a save whose data happens to push the leaf past one page must
  /// still surface it. Mirrors [loadHeroAttributes]' paginated fixed-query scan,
  /// including the save-pin guard against a mid-pagination selection change.
  Future<GameTime?> loadGameTime() async {
    final loadPath = state.selectedPath;
    var offset = 0;
    while (true) {
      final result = await searchTypedProperties(
        gameTimeQuery,
        offset: offset,
        limit: 1000,
      );
      // A save switch mid-pagination would merge pages from two different files.
      if (state.selectedPath != loadPath) return null;
      if (result.error != null) return null;
      for (final hit in result.results) {
        final path = hit.path;
        if (hit.type == 'DoubleProperty' &&
            path.length >= 3 &&
            path.last == 'TotalSeconds' &&
            path[path.length - 2] == 'CurrentTime' &&
            path.contains('{GameTime}')) {
          final value = double.tryParse(hit.value);
          if (value != null) return GameTime(totalSeconds: value, path: path);
        }
      }
      offset += result.results.length;
      if (offset >= result.total || result.results.isEmpty) break;
    }
    return null;
  }

  /// Load the hero's skills (`private.skills.list`): every learned skill plus
  /// the full learnable roster, with per-skill tier options. Returns a result
  /// carrying an inline [SkillsResult.error] on failure instead of throwing.
  Future<SkillsResult> loadSkills({String actor = 'Hero'}) async {
    final path = state.selectedPath;
    if (path == null) {
      return const SkillsResult(error: 'No save selected.');
    }
    try {
      final response = await _execute(
        'private.skills.list',
        payload: {'path': path, 'actor': actor},
      );
      if (response['ok'] != true) {
        return SkillsResult(error: _errorMessage(response));
      }
      return SkillsResult.fromJson((response['data'] as Map).cast<String, Object?>());
    } catch (error) {
      return SkillsResult(error: 'Skills load failed: $error');
    }
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
        payload: {'path': path, ...params},
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

  /// Load one page of NPC actors from the core `private.npc.list` command for
  /// the currently selected save. Mirrors [loadKnowledgeEntries]: server-side
  /// pagination + optional query, returning a typed page that carries an inline
  /// error instead of throwing so the caller can render it. The full NPC set
  /// (~1484) is large, so callers MUST paginate rather than fetch it all.
  Future<NpcActorsPage> loadNpcActors({
    String query = '',
    int offset = 0,
    int limit = 100,
    String? path,
  }) async {
    // `path` lets a multi-page caller PIN the save it started against so a
    // mid-fetch save switch can't mix pages from two files (see
    // [loadAllNpcActors]); single-shot callers omit it and use the live path.
    final resolvedPath = path ?? state.selectedPath;
    if (resolvedPath == null) {
      return const NpcActorsPage(error: 'No save selected.');
    }
    try {
      final response = await _execute(
        'private.npc.list',
        payload: {
          'path': resolvedPath,
          if (query.isNotEmpty) 'query': query,
          'offset': offset,
          'limit': limit,
        },
      );
      if (response['ok'] != true) {
        return NpcActorsPage(error: _errorMessage(response));
      }
      return NpcActorsPage.fromJson(
        (response['data'] as Map).cast<String, Object?>(),
      );
    } catch (error) {
      return NpcActorsPage(error: 'NPC list failed: $error');
    }
  }

  /// Fetch the full unified character index for the selected save in ONE call
  /// (`private.characters.list` is unpaginated — it returns every actor plus
  /// knowledge-only orphans in a single response, so there is no paging loop
  /// unlike [loadAllNpcActors]). Backs the Charaktere master list. Mirrors
  /// [loadNpcActors]: reads [state.selectedPath], goes through [_execute], and
  /// returns a typed page carrying an inline [CharacterIndexPage.error] instead
  /// of throwing so the caller can render it.
  ///
  /// A successful parse also stashes [EditorState.heroGlobalId]: the save's own
  /// "Hero" ACTOR row is the player's avatar — the pinned Player row in the
  /// master list represents it, and its GlobalId keys the player's memory
  /// events. Error pages leave the id itself untouched (a stale value from the
  /// same save is still correct; the next successful load re-stashes it).
  ///
  /// EVERY completed attempt — success (with or without a hero row), error
  /// page, or thrown failure — additionally marks
  /// [EditorState.heroGlobalIdSettled] for the save it was issued against, so
  /// the player's Ereignisse pane can stop showing its "index load in flight"
  /// spinner and settle to an empty state when no id is coming.
  Future<CharacterIndexPage> loadAllCharacters() async {
    final path = state.selectedPath;
    if (path == null) {
      return const CharacterIndexPage(error: 'No save selected.');
    }
    // Marks the load settled — only for the save this request was issued
    // against: a slot switch during the (serialized, possibly slow) core call
    // must not let the PREVIOUS save's outcome settle the newly selected file.
    void settle() {
      if (state.selectedPath == path) {
        state = state.copyWith(heroGlobalIdSettled: true);
      }
    }

    try {
      final response = await _execute(
        'private.characters.list',
        payload: {'path': path},
      );
      if (response['ok'] != true) {
        settle();
        return CharacterIndexPage(error: _errorMessage(response));
      }
      final page = CharacterIndexPage.fromJson(
        (response['data'] as Map).cast<String, Object?>(),
      );
      // Same path pin as settle(): the PREVIOUS save's hero id must not land
      // on the newly selected file.
      if (state.selectedPath == path) {
        for (final row in page.characters) {
          if (row.globalId != null && row.uniqueName.toLowerCase() == 'hero') {
            state = state.copyWith(heroGlobalId: row.globalId);
            break;
          }
        }
        state = state.copyWith(heroGlobalIdSettled: true);
      }
      return page;
    } catch (error) {
      settle();
      return CharacterIndexPage(error: 'Character list failed: $error');
    }
  }

  /// Cached full NPC list, memoized per inspection. [loadAllNpcActors] fetches
  /// the ENTIRE list once (no server `query`) so its consumers (e.g. the NPC
  /// status row's exact-id lookup) reuse a single decompress instead of
  /// re-hitting the core. The cache is keyed by the inspection identity it was
  /// loaded for; a refresh / slot switch produces a fresh inspection, which
  /// invalidates it (see [_invalidateNpcCache]).
  Future<NpcActorsPage>? _allNpcActorsFuture;
  SaveInspection? _allNpcActorsFor;

  /// Per-NPC memo of [loadNpcAttributes] / [loadNpcInventory], keyed by GlobalId.
  /// Re-selecting an NPC (or toggling between its Attribute/Inventory sub-tabs)
  /// otherwise re-hits the core each time; caching the future makes a revisit
  /// free. Cleared with the rest of the NPC caches on a fresh inspection, so an
  /// edit+save (which refreshes) re-fetches the changed NPC. Errors are not
  /// cached (the entry is dropped) so a transient failure can retry. Keyed by
  /// GlobalId AND guarded by [_npcDetailCacheForPath] (below), because the same
  /// GlobalId can exist in two saves.
  final Map<String, Future<NpcAttributesResult>> _npcAttributesCache = {};
  final Map<String, Future<NpcInventoryResult>> _npcInventoryCache = {};

  /// The save path the per-NPC detail memos were populated for. `selectedPath`
  /// changes at the START of a slot switch, but [_invalidateNpcCache] only runs
  /// after a SUCCESSFUL inspect — so a detail load in that window (or after a
  /// failed inspect) would otherwise return the previous save's memoized future
  /// for a matching GlobalId. Guarding memo access on this path drops the stale
  /// entries the moment a load runs for a different file.
  String? _npcDetailCacheForPath;

  /// Drop the per-NPC detail memos if they belong to a different save than
  /// [path]. Called at the top of every detail load, before a cache hit.
  void _guardNpcDetailCache(String path) {
    if (_npcDetailCacheForPath != path) {
      _npcAttributesCache.clear();
      _npcInventoryCache.clear();
      _npcDetailCacheForPath = path;
    }
  }

  /// Drop the cached full NPC list and per-NPC detail memos. Called whenever a
  /// fresh inspection lands so the next load re-fetches against the new save
  /// state.
  void _invalidateNpcCache() {
    _allNpcActorsFuture = null;
    _allNpcActorsFor = null;
    _npcAttributesCache.clear();
    _npcInventoryCache.clear();
    _npcDetailCacheForPath = null;
  }

  /// Load (and memoize) the FULL NPC list for the current inspection.
  /// [query]/[offset]/[limit] are ignored (kept for loader-signature
  /// compatibility) — consumers filter client-side. Subsequent calls within
  /// the same inspection return the cached future (one decompress shared
  /// across all consumers). A failed load is NOT cached, so a transient error
  /// can retry.
  Future<NpcActorsPage> loadAllNpcActors({
    String query = '',
    int offset = 0,
    int limit = 100,
  }) {
    final inspection = state.inspection;
    // Pin the save path for the WHOLE multi-page fetch. If the user switches
    // saves mid-fetch, every page still comes from the file this fetch started
    // against, so pages from two different saves can never be merged into one
    // list (the stale future's cache slot is invalidated by the new inspection).
    final pinnedPath = state.selectedPath;
    final cached = _allNpcActorsFuture;
    if (cached != null && identical(_allNpcActorsFor, inspection)) {
      return cached;
    }
    final future = () async {
      // The core clamps `private.npc.list` `limit` to 1000, but real saves have
      // ~1484+ NPCs — a single request would silently drop everyone past the
      // first page. PAGE through with an increasing offset, accumulating until
      // we have `total`, then return one combined page. The decode is cached
      // per-inspection in the core, so follow-up pages are cheap.
      final npcs = <NpcActor>[];
      var offset = 0;
      var total = 0;
      while (true) {
        final page = await loadNpcActors(
          offset: offset,
          limit: 1000,
          path: pinnedPath,
        );
        // Don't cache an error result — let the next call retry.
        if (page.error != null) {
          _invalidateNpcCache();
          return page;
        }
        npcs.addAll(page.npcs);
        total = page.total;
        offset += page.npcs.length;
        // Stop once we've collected every NPC, or the core returns an empty
        // page (defensive: never loop forever on a stuck/empty response).
        if (page.npcs.isEmpty || offset >= total) break;
      }
      return NpcActorsPage(npcs: npcs, total: total, offset: 0, limit: total);
    }();
    _allNpcActorsFuture = future;
    _allNpcActorsFor = inspection;
    return future;
  }

  /// Load every attribute of a single NPC (by GlobalId) from the core
  /// `private.npc.attributes` command for the currently selected save. Real
  /// NPCs return ~46 rows. Each row carries the FULL typed Base/Current paths
  /// that `private.typed.setValue` resolves, so the NPC attribute editor can
  /// register edits via the same pending-edit mechanism the player uses.
  /// Returns a result carrying an inline error instead of throwing, mirroring
  /// [loadHeroAttributes].
  Future<NpcAttributesResult> loadNpcAttributes(String id) {
    final path = state.selectedPath;
    if (path == null) {
      return Future.value(const NpcAttributesResult(error: 'No save selected.'));
    }
    _guardNpcDetailCache(path);
    final cached = _npcAttributesCache[id];
    if (cached != null) return cached;
    final future = () async {
      try {
        final response = await _execute(
          'private.npc.attributes',
          payload: {'path': path, 'id': id},
        );
        if (response['ok'] != true) {
          _npcAttributesCache.remove(id);
          return NpcAttributesResult(error: _errorMessage(response));
        }
        return NpcAttributesResult.fromJson(
          (response['data'] as Map).cast<String, Object?>(),
        );
      } catch (error) {
        _npcAttributesCache.remove(id);
        return NpcAttributesResult(error: 'NPC attributes failed: $error');
      }
    }();
    _npcAttributesCache[id] = future;
    return future;
  }

  /// Load a single NPC's inventory (by GlobalId) from the core
  /// `private.npc.inventory` command for the currently selected save. The
  /// payload has the SAME shape as the player inventory summary
  /// ([PrivateInventorySummary]), so the inventory card renders it unchanged;
  /// queued edits carry `actorId: <id>` so they target this NPC's container.
  /// Returns a result carrying an inline error instead of throwing, mirroring
  /// [loadNpcAttributes].
  Future<NpcInventoryResult> loadNpcInventory(String id) {
    final path = state.selectedPath;
    if (path == null) {
      return Future.value(const NpcInventoryResult(error: 'No save selected.'));
    }
    _guardNpcDetailCache(path);
    final cached = _npcInventoryCache[id];
    if (cached != null) return cached;
    final future = () async {
      try {
        final response = await _execute(
          'private.npc.inventory',
          payload: {'path': path, 'id': id},
        );
        if (response['ok'] != true) {
          _npcInventoryCache.remove(id);
          return NpcInventoryResult(error: _errorMessage(response));
        }
        return NpcInventoryResult.fromJson(
          (response['data'] as Map).cast<String, Object?>(),
        );
      } catch (error) {
        _npcInventoryCache.remove(id);
        return NpcInventoryResult(error: 'NPC inventory failed: $error');
      }
    }();
    _npcInventoryCache[id] = future;
    return future;
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
      },
      message: (data) => _backupMessage(
        edit.isRemove ? 'Memory event removed' : 'Memory event duplicated',
        data,
      ),
    );
  }

  /// Insert a brand-new character into CharacterKnowledgeByUniqueName and write
  /// the file immediately (with backup), then re-inspect so the new NPC's empty
  /// Knowledge set is queryable for follow-on entry edits. Map insertions are
  /// structural and applied one-at-a-time, so this is intentionally NOT a
  /// pending edit. Returns true on success; on failure sets state.error and
  /// returns false.
  ///
  /// Mirrors [applyMemoryEventEdit]: same no-save / isLoading / hasUnsavedEdits
  /// guards (an immediate write + refresh re-seeds the editors and would discard
  /// any unsaved pending edits), and the same _runWrite-then-refresh flow.
  Future<bool> applyAddKnowledgeCharacter(String uniqueName) async {
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
    if (state.hasUnsavedEdits) {
      state = state.copyWith(
        error:
            'Save or reset your unsaved changes first — adding a character '
            'writes the file immediately and would discard them.',
      );
      return false;
    }
    return _runWrite(
      payload: {
        'path': savePath,
        'backup': true,
        'edits': [
          {
            'path': 'private.knowledge.addCharacter',
            'value': {'value': uniqueName.trim()},
          },
        ],
      },
      message: (data) => _backupMessage('Character added', data),
    );
  }

  /// Register a PENDING revive of an NPC under the per-NPC key `npc.revive:$id`.
  /// The global Save button applies it via [saveAllPending], which submits
  /// `private.npc.revive` as its own write_save (the core rejects batching this
  /// splicing edit with peers, so [saveAllPending] splits it out).
  ///
  /// Reviving clears the NPC's defeat/kill memory events AND restores HP→Max.
  /// Registering a draft only — no write fires here, mirroring every other
  /// editor surface's pending contribution. Re-invoking for the same NPC simply
  /// overwrites its key (idempotent).
  void setPendingNpcRevive(String id) {
    setPendingEdit(
      'npc.revive:$id',
      PendingSaveEdit(
        edits: [
          {
            'path': 'private.npc.revive',
            'value': {'id': id},
          },
        ],
      ),
    );
  }

  /// Load the player's per-guild crime tally from the core
  /// `private.factions.list` command for the currently selected save. Returns a
  /// page carrying an inline error instead of throwing, mirroring
  /// [loadNpcAttributes].
  Future<FactionsPage> loadFactions() async {
    final path = state.selectedPath;
    if (path == null) {
      return const FactionsPage(error: 'No save selected.');
    }
    try {
      final response = await _execute(
        'private.factions.list',
        payload: {'path': path},
      );
      if (response['ok'] != true) {
        return FactionsPage(error: _errorMessage(response));
      }
      return FactionsPage.fromJson(
        (response['data'] as Map).cast<String, Object?>(),
      );
    } catch (error) {
      return FactionsPage(error: 'Faction list failed: $error');
    }
  }

  /// Pending-edit key prefix for a queued faction forgive (`<prefix><guild>`).
  static const _factionForgivePrefix = 'factions.forgive:';

  /// Register a PENDING forgive of a guild under the per-guild key
  /// `factions.forgive:$guild`. `private.factions.forgive` is a FIXED-size edit
  /// (it only flips `bIsForgiven`/`bIsSuppressed` bools), so it is NOT in
  /// [saveAllPending]'s splicingPaths set and rides the normal fixed-size batch
  /// when the global Save runs. Registering a draft only — no write fires here,
  /// mirroring every other editor surface's pending contribution. Re-invoking
  /// for the same guild simply overwrites its key (idempotent).
  void setPendingFactionForgive(String guild) {
    setPendingEdit(
      '$_factionForgivePrefix$guild',
      PendingSaveEdit(
        edits: [
          {
            'path': 'private.factions.forgive',
            'value': {'guild': guild},
          },
        ],
      ),
    );
  }

  /// The guild tags with a queued (pending) forgive, read from the pending-edit
  /// registry. The UI derives its optimistic "being forgiven…" reflect from this
  /// so the state survives a partial-save refresh (which re-applies still-pending
  /// forgives into the registry) rather than relying on a local cache.
  Set<String> pendingForgiveGuilds() => state.pendingEdits.keys
      .where((k) => k.startsWith(_factionForgivePrefix))
      .map((k) => k.substring(_factionForgivePrefix.length))
      .toSet();

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

  void _persistSettings() {
    _settingsStore.write(EditorSettings(saveDir: state.saveDir));
  }

  static EditorState _initialState({
    required String? saveDir,
    required EditorSettingsStore settingsStore,
  }) {
    final stored = settingsStore.read();
    return EditorState(
      saveDir: saveDir ?? stored.saveDir ?? defaultSaveRoot(),
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

/// A single flattened pending edit paired with the snapshot key it came from, so
/// [EditorNotifier.saveAllPending] can clear committed keys per sub-write.
class _KeyedEdit {
  const _KeyedEdit(this.key, this.edit);

  final String key;
  final Map<String, Object?> edit;
}

/// The actor a `private.skills.set` edit targets (`Hero` or an NPC GlobalId),
/// or `null` if [edit] is not a skill edit. A skill edit that omits `actor`
/// defaults to `Hero` — the core does the same, so the same-actor conflict guard
/// must too, or a hero skill edit with no explicit actor would slip past it.
String? _skillEditActor(Map<String, Object?> edit) {
  if (edit['path'] != 'private.skills.set') return null;
  final value = edit['value'];
  if (value is! Map) return null;
  return (value['actor'] as String?) ?? 'Hero';
}

/// The actor whose ActiveEffects a raw `private.typed.setValue` on an
/// `EffectSpec/Def` leaf targets, or `null` when [edit] is not such an edit.
///
/// A Def edit's path is `ActiveEffectsByGlobalId/{actor}/ActiveEffects/[i]/
/// EffectSpec/Def`; the `{actor}` segment is returned unwrapped so it matches
/// the `actor` a `private.skills.set` carries. A skill edit and a Def edit for
/// the SAME actor collide (a splice shifts that actor's indices); different
/// actors touch independent arrays and are safe to save together.
String? _activeEffectsDefActor(Map<String, Object?> edit) {
  if (edit['path'] != 'private.typed.setValue') return null;
  final value = edit['value'];
  if (value is! Map) return null;
  final path = value['path'];
  if (path is! List) return null;
  final segs = path.whereType<String>().toList();
  final n = segs.length;
  if (n < 2 || segs[n - 1] != 'Def' || segs[n - 2] != 'EffectSpec') {
    return null;
  }
  final i = segs.indexOf('ActiveEffectsByGlobalId');
  if (i < 0 || i + 1 >= segs.length) return null;
  final key = segs[i + 1];
  return (key.startsWith('{') && key.endsWith('}'))
      ? key.substring(1, key.length - 1)
      : key;
}

/// Whether [edit] is a raw `private.typed.setValue` whose path steps through an
/// `m_Inventory`. Such an edit collides with a queued `private.inventory.reset`,
/// which replaces the whole `m_Inventory`: the reset splice runs after the fixed
/// batch and would silently discard the typed edit (see [EditorNotifier.saveAllPending]).
bool _isInventoryTypedEdit(Map<String, Object?> edit) {
  if (edit['path'] != 'private.typed.setValue') return false;
  final value = edit['value'];
  if (value is! Map) return false;
  final path = value['path'];
  if (path is! List) return false;
  return path.whereType<String>().contains('m_Inventory');
}

/// One write_save unit in [EditorNotifier.saveAllPending]'s worklist: the edits
/// to submit. Post-write convergence is done per-edit (matched by identity)
/// rather than per-key, so a sub-write no longer needs to carry its keys.
class _SubWrite {
  const _SubWrite({
    required this.edits,
    this.syncPersistentDataList = false,
  });

  final List<Map<String, Object?>> edits;
  final bool syncPersistentDataList;
}
