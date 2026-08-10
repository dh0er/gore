import 'dart:io';

import 'package:file_selector/file_selector.dart';
import 'package:flutter/foundation.dart' show visibleForTesting;
import 'package:goresave/features/editor/domain/actor.dart';
import 'package:goresave/features/editor/domain/character_index.dart';
import 'package:goresave/features/editor/domain/core_service.dart';
import 'package:goresave/features/editor/domain/editor_models.dart';
import 'package:goresave/features/editor/domain/editor_settings_store.dart';
import 'package:goresave/features/editor/domain/game_time.dart';
import 'package:goresave/features/editor/domain/glossary_models.dart';
import 'package:goresave/features/editor/domain/hero_attributes.dart';
import 'package:goresave/features/editor/domain/npc_actors_page.dart';
import 'package:goresave/features/editor/domain/npc_attributes.dart';
import 'package:goresave/features/editor/domain/npc_position.dart';
import 'package:goresave/features/editor/domain/pending_edits.dart';
import 'package:goresave/features/editor/domain/progression_models.dart';
import 'package:goresave/features/editor/domain/skills_models.dart';
import 'package:goresave/features/editor/domain/story_state_models.dart';
import 'package:goresave/l10n/app_localizations.dart';
import 'package:goresave/l10n/app_localizations_en.dart';
import 'package:goresave/utils/default_paths.dart';
import 'package:path/path.dart' as p;
import 'package:state_notifier/state_notifier.dart';

const _unchanged = Object();

AppLocalizations _defaultEnglishLocalizations() => AppLocalizationsEn();

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
    // Orphaned profile references are useful cleanup rows, not playable saves.
    // Keep them below every real file regardless of retained PDL playtime so
    // refresh never appears to prefer a missing slot.
    if (a.isMissing != b.isMissing) return a.isMissing ? 1 : -1;
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

bool _sameSavePath(String a, String b) {
  final windowsStyle =
      a.contains('\\') ||
      b.contains('\\') ||
      RegExp(r'^[A-Za-z]:').hasMatch(a) ||
      RegExp(r'^[A-Za-z]:').hasMatch(b) ||
      a.startsWith('//') ||
      b.startsWith('//');
  final context = windowsStyle ? p.windows : p.posix;
  final normalizedA = context.normalize(a);
  final normalizedB = context.normalize(b);
  return windowsStyle
      ? normalizedA.toLowerCase() == normalizedB.toLowerCase()
      : normalizedA == normalizedB;
}

List<String> _addSavePath(List<String> paths, String path) {
  if (paths.any((candidate) => _sameSavePath(candidate, path))) return paths;
  return List.unmodifiable([...paths, path]);
}

List<String> _removeSavePath(List<String> paths, String path) =>
    List.unmodifiable(
      paths.where((candidate) => !_sameSavePath(candidate, path)),
    );

bool _sameSavePathList(List<String> a, List<String> b) {
  if (a.length != b.length) return false;
  for (var i = 0; i < a.length; i++) {
    if (!_sameSavePath(a[i], b[i])) return false;
  }
  return true;
}

class EditorState {
  EditorState({
    required this.saveDir,
    this.isLoading = false,
    this.saves = const [],
    this.profiles = const [],
    this.activeProfileId,
    this.selectedProfileId,
    this.externalSavePaths = const [],
    this.hiddenOtherSavePaths = const [],
    this.otherSavesSelected = false,
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
    Set<String> invalidEditKeys = const {},
    String? invalidNpcEditKey,
    this.heroGlobalId,
    this.heroGlobalIdSettled = false,
    this.saveProgress,
  }) : invalidEditKeys = invalidNpcEditKey == null
           ? invalidEditKeys
           : <String>{...invalidEditKeys, invalidNpcEditKey};

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

  /// Persistent paths opened outside the configured save folder.
  final List<String> externalSavePaths;

  /// Profileless scanned paths explicitly removed from the Other saves list.
  /// Tombstones are required so a rescan does not immediately re-add them.
  final List<String> hiddenOtherSavePaths;

  /// Whether the save sidebar is showing [otherSaves] instead of a profile.
  final bool otherSavesSelected;

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
  bool get hasUnsavedEdits =>
      pendingEdits.isNotEmpty || invalidEditKeys.isNotEmpty;

  /// Keys of editor surfaces with invalid local text. Their last valid pending
  /// values remain registered, but global Save is blocked until every field is
  /// valid or the edits are reset.
  final Set<String> invalidEditKeys;

  bool get hasInvalidEdits => invalidEditKeys.isNotEmpty;

  /// Compatibility view for the NPC attribute editor. New surfaces should use
  /// [invalidEditKeys]/[hasInvalidEdits] instead.
  String? get invalidNpcEditKey {
    String? legacyFallback;
    for (final key in invalidEditKeys) {
      if (key.startsWith('npc.attributes:')) return key;
      // Older callers were allowed to use an arbitrary pending key. Preserve
      // that round-trip while excluding validation keys owned by surfaces that
      // key themselves through `setEditInvalid`: returning one here would let
      // `setNpcEditInvalid`'s `..remove(invalidNpcEditKey)` clear another
      // surface's block as a side effect.
      if (key != storyStatePendingKey && !key.startsWith('npc.position:')) {
        legacyFallback ??= key;
      }
    }
    return legacyFallback;
  }

  /// True while an NPC attribute field is invalid — global Save is disabled.
  bool get hasInvalidNpcEdit => hasInvalidEdits;

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

  /// User-visible changes across all pending keys, driving the global
  /// "Unsaved (N)" badge and the Save/Reset buttons. An invalid-only draft
  /// contributes one so Reset stays reachable.
  int get pendingEditCount =>
      pendingEdits.values.fold(0, (n, e) => n + e.pendingCount) +
      invalidEditKeys.where((key) => !pendingEdits.containsKey(key)).length;

  SaveSlot? get selectedSave {
    for (final save in saves) {
      if (selectedPath != null && _sameSavePath(save.path, selectedPath!)) {
        return save;
      }
    }
    return null;
  }

  /// Resolve the authoritative profile association. Current core scans include
  /// `persistentProfileId`; the slot arrays are also consulted for older scan
  /// payloads and lightweight test doubles that only expose the association on
  /// [ProfileSummary.savedSlots].
  int? profileIdForSave(SaveSlot save) {
    // An arbitrary external file can share a conventional slot basename with a
    // local profile save. Slot-name coincidence is never profile membership.
    if (save.isExternal) return null;
    final direct = save.persistentProfileId;
    if (direct != null) return direct;
    for (final profile in profiles) {
      if (profile.savedSlots.contains(save.slot)) return profile.profileId;
    }
    return null;
  }

  /// Existing, profileless saves in the dedicated Other view. Missing profile
  /// references stay with their profile; explicitly hidden scanned saves are
  /// filtered through [hiddenOtherSavePaths].
  List<SaveSlot> get otherSaves => saves
      .where(
        (save) =>
            !save.isMissing &&
            profileIdForSave(save) == null &&
            !hiddenOtherSavePaths.any((path) => _sameSavePath(path, save.path)),
      )
      .toList(growable: false);

  /// The profile id to use for filtering: the explicitly selected profile, or
  /// fall back to the scan's active profile id.
  /// One resolution shared by the header and the save-list filter, so they
  /// can never disagree: explicit switcher choice first, then the selected
  /// save's own profile, then the scan's active profile id.
  int? get effectiveProfileId {
    if (otherSavesSelected) return null;
    final save = selectedSave;
    return selectedProfileId ??
        (save == null ? null : profileIdForSave(save)) ??
        activeProfileId;
  }

  /// Saves to show in the sidebar. A profile list contains only saves whose
  /// [SaveSlot.persistentProfileId] matches [effectiveProfileId]. Unassigned
  /// saves never leak into one or every profile list; they are reachable only
  /// through the dedicated [otherSaves] view.
  List<SaveSlot> get visibleSaves {
    if (otherSavesSelected) return otherSaves;
    final eid = effectiveProfileId;
    if (eid == null) {
      return saves.where((save) => profileIdForSave(save) != null).toList();
    }
    return saves.where((save) => profileIdForSave(save) == eid).toList();
  }

  ProfileSummary? get activeProfile {
    if (otherSavesSelected) return null;
    // A directly opened file is detached from this folder's
    // PersistentDataList. Even if its embedded numeric id happens to match a
    // local profile, that coincidence must never expose profile-wide difficulty
    // editing for the wrong profile.
    final save = selectedSave;
    if (save != null && (save.isExternal || profileIdForSave(save) == null)) {
      return null;
    }
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
    List<String>? externalSavePaths,
    List<String>? hiddenOtherSavePaths,
    bool? otherSavesSelected,
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
    Set<String>? invalidEditKeys,
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
    var resolvedInvalidEditKeys = clearPendingEdits
        ? <String>{}
        : Set<String>.from(invalidEditKeys ?? this.invalidEditKeys);
    // Backward-compatible copyWith channel used by the NPC attribute editor.
    // Replacing it must leave an invalid story-state draft intact.
    if (!identical(invalidNpcEditKey, _unchanged)) {
      final previousNpcKey = this.invalidNpcEditKey;
      if (previousNpcKey != null) {
        resolvedInvalidEditKeys.remove(previousNpcKey);
      }
      resolvedInvalidEditKeys.removeWhere(
        (key) => key.startsWith('npc.attributes:'),
      );
      final legacyKey = invalidNpcEditKey as String?;
      if (legacyKey != null) resolvedInvalidEditKeys.add(legacyKey);
    }
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
      externalSavePaths: externalSavePaths ?? this.externalSavePaths,
      hiddenOtherSavePaths: hiddenOtherSavePaths ?? this.hiddenOtherSavePaths,
      otherSavesSelected: otherSavesSelected ?? this.otherSavesSelected,
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
      // A fresh inspection re-seed (clearPendingEdits) drops all standing
      // NPC validation block — the invalid in-progress field is gone with it.
      invalidEditKeys: Set.unmodifiable(resolvedInvalidEditKeys),
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
    AppLocalizations Function()? localizations,
    bool Function(String path)? fileExists,
  }) : _settingsStore = settingsStore ?? const NoopEditorSettingsStore(),
       _localizations = localizations ?? _defaultEnglishLocalizations,
       _fileExists = fileExists ?? ((path) => File(path).existsSync()),
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
  final AppLocalizations Function() _localizations;
  final bool Function(String path) _fileExists;

  AppLocalizations get _l10n => _localizations();

  bool _saveFileExists(String path) {
    try {
      return _fileExists(path);
    } catch (_) {
      return false;
    }
  }

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
  Future<void> _withLoading(
    Future<void> Function() body, {
    String Function(String details)? failureMessage,
  }) async {
    _loadStarted();
    state = state.copyWith(isLoading: true, clearError: true);
    try {
      await body();
    } catch (error) {
      // A thrown core call (e.g. bad JSON / null native response) must surface
      // as an error rather than propagate and leave the UI wedged.
      state = state.copyWith(
        error: (failureMessage ?? _l10n.editorUnexpectedError)('$error'),
      );
    } finally {
      _loadFinished();
    }
  }

  /// Run a single write request (`write_save` by default) as a tracked load,
  /// then rescan on success. Returns true only when the core accepted the
  /// write; a rejected write sets `state.error` and returns false so callers
  /// can skip success-only follow-ups. The post-success `refresh()` rescans
  /// saves AND profiles. Used by backup/profile operations; normal editor
  /// changes go through the pending registry and [saveAllPending].
  Future<bool> _runWrite({
    required Map<String, Object?> payload,
    required String Function(Map<String, Object?> data) message,
    required String Function(String details) failureMessage,
    String command = 'write_save',
    void Function()? beforeRefresh,
  }) async {
    var ok = false;
    await _withLoading(() async {
      final response = await _execute(command, payload: payload);
      if (response['ok'] != true) {
        state = state.copyWith(error: failureMessage(_errorDetails(response)));
        return;
      }
      final data = (response['data'] as Map).cast<String, Object?>();
      state = state.copyWith(lastWriteMessage: message(data));
      beforeRefresh?.call();
      await refresh();
      ok = true;
    }, failureMessage: failureMessage);
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
    // with that work — mirrors saveAllPending. Set an
    // explicit error so the dialog explains why rather than showing a generic
    // failure.
    if (state.isLoading) {
      state = state.copyWith(error: _l10n.editorOperationInProgress);
      return Future.value(false);
    }
    // Refuse while slot edits are pending: this write runs _runWrite -> refresh,
    // and the same-save _inspect clears the pending registry — silently
    // discarding those drafts even though no write_save ran for them. Make the
    // user save or reset them first.
    if (state.hasUnsavedEdits) {
      state = state.copyWith(error: _l10n.editorUnsavedBeforeDifficulty);
      return Future.value(false);
    }
    final dir = state.saveDir;
    if (dir.isEmpty) {
      state = state.copyWith(error: _l10n.editorNoSaveFolderSelected);
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
      failureMessage: (details) => _l10n.editorDifficultyWriteFailed(details),
      message: (data) {
        final written = (data['targetsWritten'] as num?)?.toInt() ?? 0;
        return written == 0
            ? _l10n.editorNoDifficultyChanges
            : _l10n.editorDifficultyWritten;
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

  /// The effective Resources difficulty level for the INSPECTED save, normalized
  /// to 'Novice' | 'Gothic' | 'Hard' — used to pick the inventory-reset
  /// start-save. Falls back to 'Gothic' when nothing resolves.
  ///
  /// Priority: (1) the profile ACTUALLY attached to the save (its
  /// persistentProfileId); (2) the save's OWN parsed difficulty — an
  /// unattributed/imported save carries it even when the folder holds OTHER
  /// profiles, so we must NOT borrow another profile's level; (3) the scan's
  /// active profile as a directory-wide default (e.g. no save inspected yet);
  /// (4) 'Gothic'. Deliberately never the sidebar profile FILTER
  /// (`activeProfile`/`effectiveProfileId`), which is a browsing choice.
  ///
  /// Mirrors the difficulty dialog's authoritative display: a non-Custom preset
  /// (Novice/Gothic/Hard) LOCKS every sub-level to its implied tier, so a stale
  /// or disagreeing stored Resources class is ignored — a Hard profile always
  /// resets from the Hard save even if it carries an out-of-date `_Standard`
  /// resources class. Only a Custom preset — or a profile with no recognized
  /// preset to imply from — lets the stored Resources sub-level decide (else
  /// Gothic).
  String activeResourcesLevel() {
    const known = {'Novice', 'Gothic', 'Hard'};
    // A directory profile's difficulty by id, only when it carries values.
    DifficultySettings? profileDifficulty(int? id) {
      if (id == null) return null;
      for (final profile in state.profiles) {
        if (profile.profileId == id) {
          return profile.difficulty.hasAnyValue ? profile.difficulty : null;
        }
      }
      return null;
    }

    // 1. The profile ACTUALLY attached to the inspected save.
    var difficulty = profileDifficulty(state.selectedSave?.persistentProfileId);
    // 2. Else the inspected save's OWN parsed difficulty. An unattributed/imported
    //    save carries it even when the folder holds OTHER profiles — do NOT borrow
    //    the scan-active profile's level, which may belong to a different save.
    if (difficulty == null) {
      final own = state.selectedSave?.difficulty;
      if (own != null && own.hasAnyValue) difficulty = own;
    }
    // 3. Else the scan's active profile (directory-wide default; e.g. no save
    //    inspected yet).
    difficulty ??= profileDifficulty(state.activeProfileId);
    if (difficulty == null || !difficulty.hasAnyValue) return 'Gothic';
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
      state = state.copyWith(error: _l10n.editorUnsavedBeforeSwitchProfile);
      return;
    }

    // Check whether the current selection belongs to the target profile before
    // updating state. We avoid relying on visibleSaves here so that the "keep
    // selected save visible" exemption cannot silently keep the old save in view
    // and prevent the selection from moving.
    final currentSave = state.selectedSave;
    final selectionMatchesNewProfile =
        currentSave != null &&
        !currentSave.isExternal &&
        state.profileIdForSave(currentSave) != null &&
        (profileId == null || state.profileIdForSave(currentSave) == profileId);

    state = state.copyWith(
      selectedProfileId: profileId,
      otherSavesSelected: false,
    );

    if (selectionMatchesNewProfile) {
      // Current selection is compatible with the new profile — stay put.
      return;
    }

    // Current save does not belong to the new profile — move to the first
    // save that does. Unattributed saves are intentionally absent: the
    // switcher's dedicated Other saves view is their only navigation path.
    final attributed = state.saves.where(
      (s) => !s.isMissing && state.profileIdForSave(s) == profileId,
    );
    final candidate = profileId == null
        ? state.saves
              .where(
                (save) =>
                    !save.isMissing && state.profileIdForSave(save) != null,
              )
              .firstOrNull
        : attributed.firstOrNull;

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

  /// Switch the sidebar to the persistent list of profileless saves.
  Future<void> selectOtherSaves() async {
    if (state.hasUnsavedEdits) {
      state = state.copyWith(error: _l10n.editorUnsavedBeforeSwitchProfile);
      return;
    }
    if (state.isLoading) return;
    final currentPath = state.selectedPath;
    state = state.copyWith(selectedProfileId: null, otherSavesSelected: true);
    if (currentPath != null &&
        state.otherSaves.any((save) => _sameSavePath(save.path, currentPath))) {
      return;
    }
    final candidate = state.otherSaves.firstOrNull;
    if (candidate != null) {
      await _inspect(candidate.path, clearWriteMessage: true);
    } else {
      state = state.copyWith(
        selectedPath: null,
        clearInspection: true,
        clearBackups: true,
        clearPendingEdits: true,
      );
    }
  }

  /// Remove one entry from the Other saves list without deleting its file.
  /// The path receives a persistent tombstone so the next scan does not re-add
  /// it, even if an external file becomes a regular scanned file meanwhile.
  Future<bool> removeOtherSave(String path) async {
    if (state.isLoading) return false;
    if (state.hasUnsavedEdits) {
      state = state.copyWith(error: _l10n.editorUnsavedBeforeSwitchProfile);
      return false;
    }
    final save = state.otherSaves
        .where((candidate) => _sameSavePath(candidate.path, path))
        .firstOrNull;
    if (save == null) return false;

    final selectedWasRemoved = _sameSavePath(path, state.selectedPath ?? '');
    state = state.copyWith(
      saves: save.isExternal
          ? [
              for (final candidate in state.saves)
                if (!_sameSavePath(candidate.path, path)) candidate,
            ]
          : null,
      externalSavePaths: _removeSavePath(state.externalSavePaths, path),
      hiddenOtherSavePaths: _addSavePath(state.hiddenOtherSavePaths, save.path),
    );
    _persistSettings();

    if (!selectedWasRemoved) return true;
    final next = state.otherSaves.firstOrNull;
    if (next != null) {
      await _inspect(next.path, clearWriteMessage: true);
    } else {
      state = state.copyWith(
        selectedPath: null,
        clearInspection: true,
        clearBackups: true,
        clearPendingEdits: true,
      );
    }
    return true;
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
    // `npc.position:` is swept alongside `npc.attributes:` because the Position
    // sub-tab keys itself through `setEditInvalid` (see position_detail.dart);
    // without this, a stale block from the previous NPC would outlive the
    // switch and disable Save for an actor whose fields are all valid.
    final invalid = Set<String>.from(state.invalidEditKeys)
      ..remove(state.invalidNpcEditKey)
      ..removeWhere(
        (key) =>
            key.startsWith('npc.attributes:') || key.startsWith('npc.position:'),
      );
    state = state.copyWith(selectedActor: actor, invalidEditKeys: invalid);
  }

  /// Mark (`pendingKey`) or clear (`null`) the NPC attribute panel's invalid
  /// field state. While set, global Save is disabled ([EditorState.hasInvalidNpcEdit])
  /// so a now-stale stored draft is never written behind an invalid field; the
  /// stored draft itself is left intact.
  void setNpcEditInvalid(String? pendingKey) {
    if (state.invalidNpcEditKey == pendingKey) return;
    final invalid = Set<String>.from(state.invalidEditKeys)
      ..remove(state.invalidNpcEditKey)
      ..removeWhere((key) => key.startsWith('npc.attributes:'));
    if (pendingKey != null) invalid.add(pendingKey);
    state = state.copyWith(invalidEditKeys: invalid);
  }

  /// Mark or clear invalid local text for any editor surface. [key] should be
  /// the same central key as its pending edit so the global counter does not
  /// double-count a stored valid draft plus its invalid text successor.
  void setEditInvalid(String key, {required bool invalid}) {
    final normalized = key.trim();
    if (normalized.isEmpty) return;
    final updated = Set<String>.from(state.invalidEditKeys);
    final changed = invalid
        ? updated.add(normalized)
        : updated.remove(normalized);
    if (!changed) return;
    state = state.copyWith(invalidEditKeys: updated);
  }

  /// Story editor convenience wrapper for its one aggregated pending surface.
  void setStoryStateEditInvalid(bool invalid) {
    setEditInvalid(storyStatePendingKey, invalid: invalid);
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
    if (state.pendingEdits.isEmpty && state.invalidEditKeys.isEmpty) return;
    state = state.copyWith(clearPendingEdits: true);
  }

  /// All value-addressed story changes currently stored in the one atomic
  /// `private.story.apply` pending edit, sorted case-insensitively by ID.
  List<StoryStateEdit> allStoryStateEdits() {
    final pending = pendingEditFor(storyStatePendingKey);
    if (pending == null || pending.edits.isEmpty) return const [];
    // This surface deliberately owns exactly one aggregate edit. A malformed
    // registry entry is treated as no readable draft, never partially decoded.
    if (pending.edits.length != 1) return const [];
    try {
      return parseStoryStateApplyEdit(pending.edits.single);
    } on FormatException {
      return const [];
    }
  }

  /// Pending story change for [id], using the map's case-insensitive identity.
  StoryStateEdit? storyStateEditFor(String id) {
    final target = normalizeStoryStateId(id);
    if (target.isEmpty) return null;
    for (final edit in allStoryStateEdits()) {
      if (edit.normalizedId == target) return edit;
    }
    return null;
  }

  /// Upsert one story value into the aggregate. Reverting to the inspection
  /// snapshot removes it; when the last change disappears the central pending
  /// key disappears as well.
  void setStoryStateEdit(StoryStateEdit edit) {
    final normalizedId = edit.normalizedId;
    if (normalizedId.isEmpty) return;
    final byId = <String, StoryStateEdit>{
      for (final current in allStoryStateEdits()) current.normalizedId: current,
    };
    if (edit.isNoop) {
      byId.remove(normalizedId);
    } else {
      byId[normalizedId] = edit;
    }
    _setStoryStateEdits(byId.values);
  }

  /// Remove one pending story change without changing the other rows.
  void clearStoryStateEdit(String id) {
    final normalizedId = normalizeStoryStateId(id);
    if (normalizedId.isEmpty) return;
    final remaining = allStoryStateEdits()
        .where((edit) => edit.normalizedId != normalizedId)
        .toList();
    _setStoryStateEdits(remaining);
  }

  /// Remove the complete story-state aggregate and its validation block.
  void clearAllStoryStateEdits() {
    clearPendingEdit(storyStatePendingKey);
    setStoryStateEditInvalid(false);
  }

  void _setStoryStateEdits(Iterable<StoryStateEdit> edits) {
    final sorted = edits.toList()
      ..sort((a, b) => a.normalizedId.compareTo(b.normalizedId));
    if (sorted.isEmpty) {
      clearPendingEdit(storyStatePendingKey);
      return;
    }
    setPendingEdit(
      storyStatePendingKey,
      PendingSaveEdit(
        edits: [storyStateApplyEdit(sorted)],
        displayCount: sorted.length,
      ),
    );
  }

  /// Save all pending slot edits in one `write_save`, then refresh ONCE.
  /// No-op when nothing is pending. Re-entry-safe: bails immediately if a load
  /// is already in flight. Returns true on success (or when nothing to save),
  /// false on failure.
  ///
  /// Difficulty is NOT part of this path — it is a profile-level edit written
  /// directly by [writeProfileDifficulty] from the profile-header dialog.
  Future<bool> saveAllPending() async {
    if (state.hasInvalidEdits) return false;
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
    var displayEditCount = 0;
    // Placement notes belong to the SAVE, not to any one edit, so they are
    // collected across every pending key and ride the first sub-write — the same
    // one that takes the backup. The core only records them once those bytes are
    // committed.
    final placementNotes = <Map<String, Object?>>[];
    final clearPlacementNotes = <String>[];
    for (final key in snapshotKeys) {
      final entry = state.pendingEdits[key]!;
      displayEditCount += entry.pendingCount;
      for (final edit in entry.edits) {
        allEdits.add(_KeyedEdit(key, edit));
      }
      if (entry.syncPersistentDataList) syncPersistent = true;
      placementNotes.addAll(entry.placementNotes);
      clearPlacementNotes.addAll(entry.clearPlacementNotes);
    }

    // The same typed property can be edited from two surfaces at once (the
    // Player tab's hero stats and the All data browser). Batching both would
    // silently let sorted-key order pick the winner — refuse instead and let
    // the user resolve the conflict.
    final seenTypedPaths = <String>{};
    final typedPaths = <List<Object?>>[];
    for (final keyed in allEdits) {
      final edit = keyed.edit;
      if (edit['path'] != 'private.typed.setValue') continue;
      final value = edit['value'];
      if (value is! Map) continue;
      final rawPath = value['path'];
      if (rawPath is! List) continue;
      final typedPath = List<Object?>.from(rawPath);
      typedPaths.add(typedPath);
      final path = typedPath.join(' › ');
      if (!seenTypedPaths.add(path)) {
        state = state.copyWith(
          error: _l10n.editorConflictingPropertyEdits(path),
        );
        return false;
      }
    }

    // Glossary segment edits add/remove entries in the Hero's MemorizedEvents
    // array. A queued raw typed edit to that array (or one of its descendants)
    // cannot be sequenced safely with the structural glossary operation: the
    // fixed typed batch runs first, after which a removal can discard that
    // edited event, while editing OptionalClass1/2 can make the glossary lookup
    // miss its target. Refuse the ambiguous combination instead of reporting
    // success for two edits when only one intent survives.
    final hasGlossarySegmentEdit = allEdits.any(
      (keyed) => keyed.edit['path'] == 'private.glossary.setSegment',
    );
    if (hasGlossarySegmentEdit) {
      for (final keyed in allEdits) {
        final edit = keyed.edit;
        final editPath = edit['path'];
        if (editPath is! String || !editPath.startsWith('private.typed.')) {
          continue;
        }
        final value = edit['value'];
        if (value is! Map) continue;
        final rawPath = value['path'];
        if (rawPath is! List || !_addressesHeroMemorizedEvents(rawPath)) {
          continue;
        }
        final path = rawPath.join(' › ');
        state = state.copyWith(error: _l10n.editorGlossaryMemoryConflict(path));
        return false;
      }
    }

    // A glossary segment operation with a questStatePath updates that
    // CurrentState itself. Refuse a raw typed write to the exact same path;
    // sequencing the two would silently make whichever sub-write runs last win.
    for (final keyed in allEdits) {
      final edit = keyed.edit;
      if (edit['path'] != 'private.glossary.setSegment') continue;
      final value = edit['value'];
      if (value is! Map) continue;
      final rawQuestPath = value['questStatePath'];
      if (rawQuestPath is! List) continue;
      final questPath = List<Object?>.from(rawQuestPath);
      if (!typedPaths.any((path) => _sameEditorPath(path, questPath))) continue;
      final path = questPath.join(' › ');
      state = state.copyWith(error: _l10n.editorGlossaryQuestConflict(path));
      return false;
    }

    // A structured relationship edit patches or appends an object below this
    // NPC's RelationshipByGlobalId entry. A queued All-data edit below the same
    // entry can therefore be overwritten by that later structural write (or an
    // array removal can be undone when the structured write recreates the
    // modifier). Block only the same-NPC collision; edits for different NPCs
    // remain safely sequenced across their separate writes.
    final relationshipNpcIds = <String>{};
    for (final keyed in allEdits) {
      final edit = keyed.edit;
      if (edit['path'] != 'private.npc.setRelationship') continue;
      final value = edit['value'];
      if (value is! Map) continue;
      final id = value['id'];
      if (id is String && id.trim().isNotEmpty) {
        relationshipNpcIds.add(id.trim().toLowerCase());
      }
    }
    if (relationshipNpcIds.isNotEmpty) {
      for (final keyed in allEdits) {
        final edit = keyed.edit;
        final editPath = edit['path'];
        if (editPath is! String || !editPath.startsWith('private.typed.')) {
          continue;
        }
        final value = edit['value'];
        if (value is! Map) continue;
        final rawPath = value['path'];
        if (rawPath is! List ||
            !_addressesNpcRelationshipEntry(rawPath, relationshipNpcIds)) {
          continue;
        }
        final path = rawPath.join(' › ');
        state = state.copyWith(error: _l10n.editorRelationshipConflict(path));
        return false;
      }
    }

    // Structural array edits are index-addressed. Multiple REMOVES for one
    // array are safe when they target distinct original indices and run from
    // highest to lowest: a higher splice cannot shift a lower target. Keep
    // duplicate exclusive, however; insertion mixed with another structural
    // intent is rejected rather than assigning surprising index semantics.
    // Also reject a raw value edit inside a structurally edited array, where a
    // splice could retarget that descendant.
    final structuralArrayGroups = <_StructuralArrayGroup>[];
    for (final keyed in allEdits) {
      final op = keyed.edit['path'];
      if (op != 'private.typed.arrayRemove' &&
          op != 'private.typed.arrayDuplicate') {
        continue;
      }
      final value = keyed.edit['value'];
      final rawPath = value is Map ? value['path'] : null;
      if (rawPath is! List) continue;
      final path = List<Object?>.from(rawPath);
      final rawIndex = value is Map ? value['index'] : null;
      if (rawIndex is! num || rawIndex < 0 || rawIndex != rawIndex.toInt()) {
        continue;
      }
      _StructuralArrayGroup? group;
      for (final candidate in structuralArrayGroups) {
        if (_sameEditorPath(candidate.path, path)) {
          group = candidate;
          break;
        }
      }
      group ??= _StructuralArrayGroup(path);
      if (!structuralArrayGroups.contains(group)) {
        structuralArrayGroups.add(group);
      }
      final index = rawIndex.toInt();
      if (group.edits.any((candidate) => candidate.index == index)) {
        state = state.copyWith(
          error: _l10n.editorMultipleStructuralArrayEdits(path.join(' › ')),
        );
        return false;
      }
      group.edits.add(
        _IndexedStructuralEdit(
          keyed: keyed,
          index: index,
          isDuplicate: op == 'private.typed.arrayDuplicate',
        ),
      );
    }
    for (final group in structuralArrayGroups) {
      if (group.edits.length > 1 &&
          group.edits.any((edit) => edit.isDuplicate)) {
        state = state.copyWith(
          error: _l10n.editorMultipleStructuralArrayEdits(
            group.path.join(' › '),
          ),
        );
        return false;
      }
      group.edits.sort((left, right) => right.index.compareTo(left.index));
      final arrayPath = group.path;
      final conflictingValuePath = typedPaths.where(
        (path) => _editorPathIsPrefix(arrayPath, path),
      );
      if (conflictingValuePath.isEmpty) continue;
      state = state.copyWith(
        error: _l10n.editorStructuralArrayConflict(arrayPath.join(' › ')),
      );
      return false;
    }
    // Revive removes defeat/kill events across every owner's MemorizedEvents
    // array. Combining it with an index-addressed edit to one of those arrays
    // could shift the queued target before its sub-write, so require separate
    // saves for those intentions.
    final hasNpcRevive = allEdits.any(
      (keyed) => keyed.edit['path'] == 'private.npc.revive',
    );
    if (hasNpcRevive) {
      for (final group in structuralArrayGroups) {
        if (!group.path.contains('MemorizedEvents')) continue;
        state = state.copyWith(
          error: _l10n.editorMultipleStructuralArrayEdits(
            group.path.join(' › '),
          ),
        );
        return false;
      }
    }

    // Splicing structural edits (inventory, knowledge, glossary segments,
    // memory events, NPC revive/relationship) insert or remove bytes mid-payload and shift every
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
      'private.knowledge.setEntry',
      'private.typed.arrayRemove',
      'private.typed.arrayDuplicate',
      'private.glossary.setSegment',
      'private.npc.revive',
      'private.npc.setRelationship',
      storyStateApplyPath,
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
    // Reorder only the occupied positions for each array path. Other splicing
    // operations retain their stable order, while every allowed remove group
    // reaches its singleton sub-writes index-descending even if another caller
    // inserted the pending edits out of order.
    for (final group in structuralArrayGroups) {
      final positions = <int>[];
      for (var i = 0; i < splicing.length; i++) {
        if (group.edits.any(
          (entry) => identical(entry.keyed.edit, splicing[i].edit),
        )) {
          positions.add(i);
        }
      }
      for (var i = 0; i < positions.length; i++) {
        splicing[positions[i]] = group.edits[i].keyed;
      }
    }
    // Adding a segment needs an existing SegmentUnlocked event as its byte
    // template. If the same Save removes its last unlock first, a later add can
    // no longer be encoded. Stable-partition only the glossary slots so all
    // adds precede all removals while every non-glossary splice keeps its
    // original position relative to the other structural operations.
    final glossarySplices = splicing
        .where((k) => k.edit['path'] == 'private.glossary.setSegment')
        .toList();
    final orderedGlossarySplices = <_KeyedEdit>[
      ...glossarySplices.where(
        (k) => (k.edit['value'] as Map?)?['unlocked'] == true,
      ),
      ...glossarySplices.where(
        (k) => (k.edit['value'] as Map?)?['unlocked'] != true,
      ),
    ];
    var nextGlossarySplice = 0;
    final orderedSplicing = <_KeyedEdit>[
      for (final keyed in splicing)
        if (keyed.edit['path'] == 'private.glossary.setSegment')
          orderedGlossarySplices[nextGlossarySplice++]
        else
          keyed,
    ];
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
      state = state.copyWith(error: _l10n.editorSkillsEffectConflict);
      return false;
    }
    // A reset REPLACES the whole m_Inventory of its actor. Any other edit that
    // touches that SAME inventory — a structured setItemCount/addItem/removeItem
    // for the same actor, or a raw All-data private.typed.setValue stepping
    // through an m_Inventory — lands in an earlier sub-write (the fixed batch, or
    // another splice), so the reset would silently overwrite (discard) it while
    // Save still reported success for both. Refuse the combination (like the
    // conflicts above); the reset and the other inventory edit must be saved
    // separately. Structured ops are matched by the reset's actorId (null =
    // player); the raw typed case is matched broadly (its actor is not cheaply
    // recoverable from the path), so a cross-actor typed pair just gets a "save
    // separately" nudge rather than a silent overwrite.
    final resetActors = <String?>{
      for (final k in allEdits)
        if (k.edit['path'] == 'private.inventory.reset')
          (k.edit['value'] as Map?)?['actorId'] as String?,
    };
    if (resetActors.isNotEmpty &&
        allEdits.any((k) {
          final path = k.edit['path'];
          if (path == 'private.inventory.reset') return false;
          if (_isInventoryTypedEdit(k.edit)) return true;
          if (path == 'private.inventory.setItemCount' ||
              path == 'private.inventory.addItem' ||
              path == 'private.inventory.removeItem') {
            return resetActors.contains(
              (k.edit['value'] as Map?)?['actorId'] as String?,
            );
          }
          return false;
        })) {
      state = state.copyWith(error: _l10n.editorInventoryResetConflict);
      return false;
    }
    // The whole-save slot repair rewrites every misaligned m_Id. Any edit that
    // addresses a slot by the id the UI showed — an NPC removal or count edit —
    // must therefore run BEFORE it, so the repair gets its own trailing write
    // instead of leading the fixed batch.
    const repairSlotsPath = 'private.inventory.repairSlots';
    final repairEdits = allEdits
        .where((k) => k.edit['path'] == repairSlotsPath)
        .toList();
    // An add or a removal claims a whole slot — the add fills a blank one and
    // resets its payload, the removal blanks one — so ANY raw All-Data edit into
    // a slot would be silently overwritten while Save still reported success.
    // The repair is narrower: it only rewrites ids, and only after everything
    // else has run, so it collides with an edit of a slot's m_Id and with
    // nothing else. Refuse those combinations the way a queued reset does.
    const slotClaimingPaths = {
      'private.inventory.addItem',
      'private.inventory.removeItem',
    };
    final claimsSlots = allEdits.any(
      (k) => slotClaimingPaths.contains(k.edit['path']),
    );
    final conflicts = claimsSlots
        ? allEdits.any((k) => isInventorySlotTypedEdit(k.edit))
        : repairEdits.isNotEmpty &&
              allEdits.any((k) => isInventorySlotIdTypedEdit(k.edit));
    if (conflicts) {
      state = state.copyWith(error: _l10n.editorInventorySlotEditConflict);
      return false;
    }
    final fixedBatch = allEdits
        .where(
          (k) =>
              !splicingPaths.contains(k.edit['path']) &&
              k.edit['path'] != skillPath &&
              k.edit['path'] != repairSlotsPath,
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
      for (final keyed in orderedSplicing) _SubWrite(edits: [keyed.edit]),
      // All skill edits together, in their own trailing write: they batch safely
      // among themselves but must not share a write with an index-addressed peer
      // (see skillPath above).
      if (skillEdits.isNotEmpty)
        _SubWrite(edits: [for (final keyed in skillEdits) keyed.edit]),
      // Last: the slot repair, so every id-addressed edit above resolved against
      // the ids the user saw (see repairSlotsPath).
      if (repairEdits.isNotEmpty)
        _SubWrite(edits: [for (final keyed in repairEdits) keyed.edit]),
    ];
    // Hang the placement notes on whichever sub-write goes first. It is the one
    // that takes the backup, and — for a position edit, which is never a
    // splicing edit — the one that actually carries the move.
    if (worklist.isNotEmpty &&
        (placementNotes.isNotEmpty || clearPlacementNotes.isNotEmpty)) {
      final first = worklist.first;
      worklist[0] = _SubWrite(
        edits: first.edits,
        syncPersistentDataList: first.syncPersistentDataList,
        placementNotes: placementNotes,
        clearPlacementNotes: clearPlacementNotes,
      );
    }

    final n = displayEditCount;
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
    // The core writes the undo note AFTER the bytes land and reports a failure
    // beside a successful save rather than failing it. Unreported, the user
    // would be told the pin succeeded while the routine it replaced was lost
    // with nothing recording it.
    String? placementNoteWarning;
    String? failureError;
    var ok = false;
    await _withLoading(() async {
      // Seed the determinate progress bar (0 of N committed). Each sequential
      // write_save below bumps `done`, so a multi-write save (e.g. several
      // inventory adds) shows real progress instead of a stuck spinner.
      state = state.copyWith(saveProgress: (done: 0, total: worklist.length));
      try {
        for (var i = 0; i < worklist.length; i++) {
          final sub = worklist[i];
          Map<String, Object?> response;
          try {
            response = await _execute(
              'write_save',
              payload: {
                'path': savePath,
                // Backup-once: only the first sub-write snapshots the pristine file.
                'backup': i == 0,
                if (sub.syncPersistentDataList) 'syncPersistentDataList': true,
                if (sub.placementNotes.isNotEmpty)
                  'placementNotes': sub.placementNotes,
                if (sub.clearPlacementNotes.isNotEmpty)
                  'clearPlacementNotes': sub.clearPlacementNotes,
                'edits': sub.edits,
              },
            );
          } catch (error) {
            // Treat a worker/native exception exactly like a structured failed
            // sub-write. Earlier writes may already be on disk, so the shared
            // partial-failure path below must refresh the inspection and
            // rehydrate only the still-unwritten pending edits.
            failureError = _l10n.editorSaveFailed('$error');
            break;
          }
          if (response['ok'] != true) {
            // Stop on the first failure. Earlier sub-writes already committed.
            failureError = _l10n.editorSaveFailed(_errorDetails(response));
            break;
          }
          final data =
              (response['data'] as Map?)?.cast<String, Object?>() ?? const {};
          if (i == 0) firstData = data;
          final warning = data['placementNoteWarning'];
          if (warning is String && warning.isNotEmpty) {
            placementNoteWarning ??= warning;
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
          final saved = _backupMessage(
            _l10n.editorChangesSavedWithBackup(n),
            firstData,
          );
          state = state.copyWith(
            lastWriteMessage: placementNoteWarning == null
                ? saved
                : '$saved\n'
                      '${_l10n.editorPlacementNoteFailed(placementNoteWarning!)}',
          );
          // Single trailing refresh after the last successful write.
          await refresh();
          ok = true;
          return;
        }

        // Any failed sub-write requires a fresh inspection. Even when no local
        // write committed, an optimistic-concurrency failure means another writer
        // may already have changed the file. Preserve every still-unwritten draft
        // across that refresh so the user can compare/retry it against fresh disk
        // state. refresh() clears the error, so restore the write failure afterward.
        final preserved = _pendingMinusCommitted(committedEdits);
        // Restore the drafts ATOMICALLY with the new inspection — but only if we
        // land back on the same save they target. refresh() may clear/auto-switch
        // selectedPath (this save vanished, or another slot was auto-selected);
        // the preserved edits target the ORIGINAL file, so they are dropped in
        // that case rather than re-targeted at the wrong save. Restoring inside
        // the inspection re-seed means kept-alive editors rehydrate WITH them.
        await refresh(preservedEdits: preserved, preservedForPath: savePath);
        // A sub-write that COMMITTED may still have failed to write its undo
        // note, and that survives the failure of a later sub-write: the move is
        // on disk either way, so reporting only the save error would leave an
        // NPC pinned with the replaced routine recorded nowhere.
        state = state.copyWith(
          error: placementNoteWarning == null
              ? failureError
              : '$failureError\n'
                    '${_l10n.editorPlacementNoteFailed(placementNoteWarning!)}',
        );
      } finally {
        // A thrown _execute (e.g. CoreWorkerException from the persistent worker
        // isolate) skips the in-loop clear above; guarantee the determinate bar
        // is dropped so a later load shows the plain spinner, not stale counts.
        if (state.saveProgress != null) {
          state = state.copyWith(clearSaveProgress: true);
        }
      }
    }, failureMessage: (details) => _l10n.editorSaveFailed(details));

    // Converge the pending set to only the still-uncommitted edits — per EDIT, so
    // a partially-committed key keeps its unwritten edits for retry — even if the
    // refresh above never ran or threw. On success refresh() already cleared
    // everything (this is then a no-op); on a partial/failed refresh this is the
    // safety net that stops committed edits from lingering as pending.
    if (committedEdits.isNotEmpty) {
      for (final entry in Map<String, PendingSaveEdit>.from(
        state.pendingEdits,
      ).entries) {
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
              // Carried, not dropped: a retry of the still-unwritten edits must
              // still record its undo note. Re-recording one whose sub-write did
              // commit is harmless — the note is keyed by NPC and identical — but
              // losing it would leave an NPC pinned with no way back.
              placementNotes: entry.value.placementNotes,
              clearPlacementNotes: entry.value.clearPlacementNotes,
              displayCount: entry.value.displayCount,
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
      final remaining = entry.value.edits
          .where((e) => !committed.contains(e))
          .toList();
      if (remaining.isNotEmpty) {
        result[entry.key] = PendingSaveEdit(
          edits: remaining,
          syncPersistentDataList: entry.value.syncPersistentDataList,
          // See the same carry in the converge loop above: an undo note has to
          // survive a partial save, or the retry pins an NPC with no way back.
          placementNotes: entry.value.placementNotes,
          clearPlacementNotes: entry.value.clearPlacementNotes,
          displayCount: entry.value.displayCount,
        );
      }
    }
    return result;
  }

  Future<void> chooseSaveDir() async {
    final selected = await getDirectoryPath(
      confirmButtonText: _l10n.editorUseFolder,
      initialDirectory: state.saveDir,
    );
    if (selected == null) return;
    await setSaveDir(selected);
  }

  /// Open a detached Gothic save without changing the configured game save
  /// folder. The picker is kept here (rather than in the widget) so all profile
  /// menu call sites share the same file filter and loading guard.
  Future<void> openSaveFile() async {
    final file = await openFile(
      acceptedTypeGroups: [
        XTypeGroup(
          label: _l10n.editorGothicSavegameFileType,
          extensions: const ['sav'],
        ),
      ],
    );
    if (file == null) return;
    await loadExternalSave(file.path);
  }

  /// Testable/non-picker half of [openSaveFile]. The external entry is retained
  /// across rescans and remains explicitly detached from folder profiles.
  Future<void> loadExternalSave(String path) async {
    if (state.isLoading) return;
    if (state.hasUnsavedEdits) {
      state = state.copyWith(error: _l10n.editorUnsavedBeforeOpenFile);
      return;
    }
    final normalized = path.trim();
    if (normalized.isEmpty || !normalized.toLowerCase().endsWith('.sav')) {
      state = state.copyWith(error: _l10n.editorSelectSavFile);
      return;
    }

    SaveSlot? existing;
    for (final save in state.saves) {
      if (_sameSavePath(save.path, normalized)) {
        // A scanned entry is authoritative if stale state ever contains both
        // it and a detached placeholder for the same Windows path.
        if (!save.isExternal) {
          existing = save;
          break;
        }
        existing ??= save;
      }
    }
    // Picking a file that already belongs to the scanned folder is just an
    // ordinary selection; retain its authoritative profile association.
    if (existing != null && !existing.isExternal) {
      final profileId = state.profileIdForSave(existing);
      final externalSavePaths = _removeSavePath(
        state.externalSavePaths,
        existing.path,
      );
      final hiddenOtherSavePaths = profileId == null
          ? _removeSavePath(state.hiddenOtherSavePaths, existing.path)
          : state.hiddenOtherSavePaths;
      state = state.copyWith(
        selectedProfileId: profileId,
        otherSavesSelected: profileId == null,
        externalSavePaths: externalSavePaths,
        hiddenOtherSavePaths: hiddenOtherSavePaths,
      );
      _persistSettings();
      await inspect(existing.path);
      return;
    }
    final previousState = state;
    final placeholder = existing?.isExternal == true
        ? existing!
        : SaveSlot(
            path: normalized,
            slot: p.basenameWithoutExtension(normalized),
            format: 'GSAV',
            fileSize: 0,
            sha1: '',
            status: 'loading',
            isExternal: true,
          );
    // Reopening an existing detached save with different Windows casing or
    // separators must keep the path stored by its SaveSlot. EditorState's
    // selection/offer accessors intentionally use that canonical value.
    final externalPath = placeholder.path;
    final saves = <SaveSlot>[
      for (final save in state.saves)
        if (!_sameSavePath(save.path, externalPath)) save,
      placeholder,
    ];
    _sortByPlaytimeDesc(saves);
    final externalSavePaths = _addSavePath(
      state.externalSavePaths,
      externalPath,
    );
    state = state.copyWith(
      saves: saves,
      externalSavePaths: externalSavePaths,
      hiddenOtherSavePaths: _removeSavePath(
        state.hiddenOtherSavePaths,
        externalPath,
      ),
      selectedProfileId: null,
      otherSavesSelected: true,
    );
    await _inspect(externalPath, clearWriteMessage: true);

    final inspection = state.selectedPath == externalPath
        ? state.inspection
        : null;
    if (inspection == null || inspection.format != 'GSAV') {
      final openError = state.error ?? _l10n.editorNotGothicGsav;
      state = previousState.copyWith(error: openError);
    } else {
      _persistSettings();
    }
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
      Map<String, Object?> response;
      try {
        response = await _execute(
          'scan_save_dir',
          payload: {'path': state.saveDir},
        );
      } catch (error) {
        // Treat a thrown worker/native failure like a structured scan error so
        // detached files can still be restored and stale paths pruned.
        response = {
          'ok': false,
          'error': {'message': '$error'},
        };
      }
      if (seq != _loadSeq) return;
      String? scanError;
      Map<String, Object?>? data;
      late final List<ProfileSummary> profiles;
      late final List<SaveSlot> saves;
      if (response['ok'] == true) {
        data = (response['data'] as Map?)?.cast<String, Object?>();
        final rawProfiles = (data?['profiles'] as List?) ?? const [];
        profiles = rawProfiles
            .whereType<Map>()
            .map((m) => ProfileSummary.fromJson(m.cast<String, Object?>()))
            .toList();
        final profileBySavedSlot = <String, int>{
          for (final profile in profiles)
            for (final slot in profile.savedSlots) slot: profile.profileId,
        };
        final rawSaves = (data?['saves'] as List?) ?? const [];
        saves = rawSaves.whereType<Map>().map((m) {
          final json = m.cast<String, Object?>();
          final inferredProfileId = profileBySavedSlot[json['slot'] as String?];
          return SaveSlot.fromJson(
            json['persistentProfileId'] == null && inferredProfileId != null
                ? {...json, 'persistentProfileId': inferredProfileId}
                : json,
          );
        }).toList();
      } else {
        // Detached saves are independent from the configured game save folder.
        // Keep the last successful folder snapshot, but still restore/prune the
        // persisted external list when that folder is missing or unreadable.
        scanError = _l10n.editorScanSavesFailed(_errorDetails(response));
        profiles = List<ProfileSummary>.of(state.profiles);
        saves = state.saves.where((save) => !save.isExternal).toList();
      }
      bool isUnassignedInNewScan(SaveSlot save) =>
          !save.isMissing &&
          save.persistentProfileId == null &&
          !profiles.any((profile) => profile.savedSlots.contains(save.slot));

      // Restore every persisted external file as a detached SaveSlot. A file
      // that has since appeared in the configured scan becomes authoritative
      // there instead; a path that vanished from disk is pruned automatically.
      var externalSavePaths = <String>[];
      var hiddenOtherSavePaths = state.hiddenOtherSavePaths;
      for (final externalPath in state.externalSavePaths) {
        final scanned = saves
            .where(
              (save) =>
                  !save.isExternal && _sameSavePath(save.path, externalPath),
            )
            .firstOrNull;
        if (scanned != null) {
          if (isUnassignedInNewScan(scanned)) {
            // Explicitly opening a scanned, profileless file re-adds it after a
            // previous manual removal from the Other list.
            hiddenOtherSavePaths = _removeSavePath(
              hiddenOtherSavePaths,
              externalPath,
            );
          }
          continue;
        }
        if (!_saveFileExists(externalPath)) continue;
        externalSavePaths = _addSavePath(externalSavePaths, externalPath);
        if (saves.any(
          (save) => save.isExternal && _sameSavePath(save.path, externalPath),
        )) {
          continue;
        }
        final retained = state.saves
            .where(
              (save) =>
                  save.isExternal && _sameSavePath(save.path, externalPath),
            )
            .firstOrNull;
        final normalized = externalPath.replaceAll('\\', '/');
        final fileName = normalized.split('/').last;
        final dot = fileName.lastIndexOf('.');
        final slot = dot > 0 ? fileName.substring(0, dot) : fileName;
        saves.add(
          retained ??
              SaveSlot(
                path: externalPath,
                slot: slot.isEmpty ? 'external' : slot,
                format: 'GSAV',
                fileSize: 0,
                sha1: '',
                status: 'ok',
                isExternal: true,
              ),
        );
      }

      // Keep a scanned-save tombstone only while the same file still exists and
      // remains profileless. Assigned/deleted saves cannot belong to this list.
      var keptHiddenOtherSavePaths = <String>[];
      for (final hiddenPath in hiddenOtherSavePaths) {
        final scanned = saves
            .where(
              (save) =>
                  !save.isExternal && _sameSavePath(save.path, hiddenPath),
            )
            .firstOrNull;
        final keep = scanned != null
            ? isUnassignedInNewScan(scanned)
            : _saveFileExists(hiddenPath);
        if (keep) {
          keptHiddenOtherSavePaths = _addSavePath(
            keptHiddenOtherSavePaths,
            hiddenPath,
          );
        }
      }
      hiddenOtherSavePaths = keptHiddenOtherSavePaths;
      _sortByPlaytimeDesc(saves);
      final activeProfileId = scanError == null
          ? (data?['activeProfileId'] as num?)?.toInt()
          : state.activeProfileId;
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
        externalSavePaths: externalSavePaths,
        hiddenOtherSavePaths: hiddenOtherSavePaths,
        // With no profiles, Other saves is the switcher's only destination and
        // therefore the natural initial view (including its Open file button).
        otherSavesSelected: profiles.isEmpty ? true : state.otherSavesSelected,
      );
      final settingsChanged =
          !_sameSavePathList(state.externalSavePaths, externalSavePaths) ||
          !_sameSavePathList(state.hiddenOtherSavePaths, hiddenOtherSavePaths);
      // Compute visible saves with the updated state fields to find a
      // sensible first selection path when the folder or profile changed.
      final visibleAfterRefresh = newState.visibleSaves;
      final retainedSelection = visibleAfterRefresh
          .where(
            (save) =>
                !save.isMissing &&
                state.selectedPath != null &&
                _sameSavePath(save.path, state.selectedPath!),
          )
          .firstOrNull;
      final selectedPath =
          retainedSelection?.path ??
          visibleAfterRefresh
              .where((save) => !save.isMissing)
              .firstOrNull
              ?.path;
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
      state = newState;
      if (settingsChanged) _persistSettings();
      if (selectedPath == null) {
        state = state.copyWith(
          selectedPath: null,
          clearInspection: true,
          clearBackups: true,
          clearPendingEdits: true,
        );
      } else {
        await _inspect(
          selectedPath,
          // Restore the preserved partial-save edits only if we landed back on
          // the same save they target (atomic with the inspection re-seed).
          restorePendingEdits:
              (preservedForPath != null &&
                  _sameSavePath(selectedPath, preservedForPath))
              ? preservedEdits
              : null,
        );
      }
      if (scanError != null && state.error == null) {
        state = state.copyWith(error: scanError);
      }
    } catch (error) {
      // A thrown core call (e.g. invalid/null native JSON) must surface as an
      // in-app error, not just an async console error.
      if (seq == _loadSeq) {
        state = state.copyWith(error: _l10n.editorScanSavesFailed('$error'));
      }
    } finally {
      _loadFinished();
    }
  }

  Future<void> inspect(String path) async {
    // Missing profile references use the expected file path as a stable row
    // key, but no file exists to inspect. Ignore programmatic taps as well as
    // disabling the row in the widget so this invariant is enforced in-domain.
    if (state.saves.any(
      (save) => _sameSavePath(save.path, path) && save.isMissing,
    )) {
      return;
    }
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
    final switchingSlot =
        state.selectedPath == null || !_sameSavePath(state.selectedPath!, path);
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
      final payload = <String, Object?>{'path': path, 'includePrivate': true};
      final response = await _execute('inspect_save', payload: payload);
      // Only the latest load applies results. Core calls are serialized, so a
      // superseded load always finishes before the newer one; bailing here
      // prevents it from applying stale data over the fresher load.
      if (seq != _loadSeq) return;
      if (response['ok'] != true) {
        state = state.copyWith(
          error: _l10n.editorInspectSaveFailed(_errorDetails(response)),
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
      final inspection = SaveInspection.fromJson(data);
      final selectedWasExternal = state.saves.any(
        (save) => save.isExternal && _sameSavePath(save.path, path),
      );
      final refreshedSaves = selectedWasExternal
          ? <SaveSlot>[
              for (final save in state.saves)
                if (!_sameSavePath(save.path, path)) save,
              SaveSlot.fromInspection(inspection, isExternal: true),
            ]
          : state.saves;
      if (selectedWasExternal) _sortByPlaytimeDesc(refreshedSaves);
      state = state.copyWith(
        inspection: inspection,
        saves: refreshedSaves,
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
          error: _l10n.editorInspectSaveFailed('$error'),
          clearInspection: true,
          clearBackups: true,
        );
      }
    } finally {
      _loadFinished();
    }
  }

  /// Atomically assign the selected save to another game profile.
  ///
  /// Registered saves are moved between profiles in place. A detached save is
  /// imported into the configured game save folder under a free `G1R-NNN`
  /// slot first; the source file remains untouched. In both cases the core
  /// updates the save and PersistentDataList.sav as one operation.
  Future<bool> assignSelectedSaveToProfile(int profileId) async {
    if (state.isLoading) return false;
    if (state.hasUnsavedEdits) {
      state = state.copyWith(error: _l10n.editorUnsavedBeforeChangeSaveProfile);
      return false;
    }
    final save = state.selectedSave;
    if (save == null) return false;
    if (!state.profiles.any((profile) => profile.profileId == profileId)) {
      state = state.copyWith(error: _l10n.editorProfileNotFound(profileId));
      return false;
    }
    if (!save.isExternal && save.persistentProfileId == profileId) return true;

    final dir = state.saveDir;
    if (dir.trim().isEmpty) {
      state = state.copyWith(error: _l10n.editorNoSaveFolderSelected);
      return false;
    }
    final isWindowsStyle =
        dir.contains('\\') || RegExp(r'^[A-Za-z]:').hasMatch(dir);
    final ctx = isWindowsStyle ? p.Context(style: p.Style.windows) : p.posix;

    final destinationPath = save.isExternal
        ? _freeExternalImportPath(save, ctx)
        : null;
    if (save.isExternal && destinationPath == null) {
      state = state.copyWith(error: _l10n.editorNoFreeSaveSlot);
      return false;
    }

    final previousSaves = state.saves;
    final previousPath = state.selectedPath;
    final previousSelection = state.selectedProfileId;
    final previousExternalSavePaths = state.externalSavePaths;
    final previousHiddenOtherSavePaths = state.hiddenOtherSavePaths;
    final previousOtherSelection = state.otherSavesSelected;
    // Keep the freshly assigned save visible through the trailing rescan. For
    // imports, remove the detached source before refresh so refresh() does not
    // merge it back into the scanned folder list, and point selection at the
    // destination that the core is about to create.
    state = state.copyWith(
      saves: save.isExternal
          ? <SaveSlot>[
              for (final candidate in state.saves)
                if (candidate.path != save.path) candidate,
            ]
          : null,
      selectedPath: save.isExternal ? destinationPath : _unchanged,
      selectedProfileId: profileId,
      otherSavesSelected: false,
      externalSavePaths: _removeSavePath(state.externalSavePaths, save.path),
      hiddenOtherSavePaths: _removeSavePath(
        state.hiddenOtherSavePaths,
        save.path,
      ),
    );
    final ok = await _runWrite(
      command: 'assign_save_profile',
      payload: {
        'path': save.path,
        'destinationPath': ?destinationPath,
        'persistentPath': ctx.join(dir, 'PersistentDataList.sav'),
        'profileId': profileId,
        'backup': true,
      },
      failureMessage: (details) => _l10n.editorProfileAssignmentFailed(details),
      message: (data) {
        final assigned = save.isExternal
            ? _l10n.editorSaveImportedAssigned(profileId)
            : _l10n.editorSaveAssigned(profileId);
        // An import copies the save's undo notes across after the bytes land.
        // If that failed, the imported save can hold a pinned NPC with no
        // record of the routine the pin replaced.
        final noteWarning = data['placementNoteWarning'];
        return noteWarning is String && noteWarning.isNotEmpty
            ? '$assigned\n${_l10n.editorPlacementNoteFailed(noteWarning)}'
            : assigned;
      },
      beforeRefresh: _persistSettings,
    );
    if (!ok) {
      // The command did not commit. Restore the detached entry and selection
      // exactly as they were; copyWith intentionally preserves the core error
      // set by _runWrite so the UI can still explain the failure.
      state = state.copyWith(
        saves: previousSaves,
        selectedPath: previousPath,
        selectedProfileId: previousSelection,
        externalSavePaths: previousExternalSavePaths,
        hiddenOtherSavePaths: previousHiddenOtherSavePaths,
        otherSavesSelected: previousOtherSelection,
      );
    }
    return ok;
  }

  /// Remove a slot from its game profile without deleting the physical save.
  ///
  /// The core removes all profile-array references and the authoritative
  /// PersistentDataList public-data entry in one validated, backed-up write.
  /// This works for both a real save and a missing/orphaned reference. A real
  /// file remains in the scan as an unattributed save; an orphan disappears.
  Future<bool> removeSaveFromProfile({
    required String slot,
    required int profileId,
  }) async {
    if (state.isLoading) return false;
    if (state.hasUnsavedEdits) {
      state = state.copyWith(error: _l10n.editorUnsavedBeforeRemoveProfile);
      return false;
    }
    final profile = state.profiles
        .where((candidate) => candidate.profileId == profileId)
        .firstOrNull;
    if (profile == null) {
      state = state.copyWith(error: _l10n.editorProfileNotFound(profileId));
      return false;
    }
    final save = state.saves
        .where(
          (candidate) =>
              candidate.slot == slot &&
              candidate.persistentProfileId == profileId,
        )
        .firstOrNull;
    if (!profile.savedSlots.contains(slot) && save == null) {
      state = state.copyWith(
        error: _l10n.editorSaveSlotNotAssigned(slot, profileId),
      );
      return false;
    }

    final dir = state.saveDir;
    if (dir.trim().isEmpty) {
      state = state.copyWith(error: _l10n.editorNoSaveFolderSelected);
      return false;
    }
    final isWindowsStyle =
        dir.contains('\\') || RegExp(r'^[A-Za-z]:').hasMatch(dir);
    final ctx = isWindowsStyle ? p.Context(style: p.Style.windows) : p.posix;

    return _runWrite(
      command: 'remove_save_from_profile',
      payload: {
        'persistentPath': ctx.join(dir, 'PersistentDataList.sav'),
        'slot': slot,
        'profileId': profileId,
        'backup': true,
      },
      failureMessage: (details) => _l10n.editorProfileRemovalFailed(details),
      message: (data) =>
          _backupMessage(_l10n.editorSaveRemovedFromProfile, data),
      beforeRefresh: save == null || save.isMissing
          ? null
          : () {
              state = state.copyWith(
                hiddenOtherSavePaths: _removeSavePath(
                  state.hiddenOtherSavePaths,
                  save.path,
                ),
              );
              _persistSettings();
            },
    );
  }

  String? _freeExternalImportPath(SaveSlot source, p.Context ctx) {
    final occupiedSlots = state.saves
        .where((save) => !save.isExternal)
        .map((save) => save.slot.toUpperCase())
        .toSet();
    final occupiedPaths = state.saves
        .where((save) => !save.isExternal)
        .map((save) => ctx.normalize(save.path).toLowerCase())
        .toSet();

    bool available(String slot) {
      final path = ctx.join(state.saveDir, '$slot.sav');
      return !occupiedSlots.contains(slot) &&
          !occupiedPaths.contains(ctx.normalize(path).toLowerCase()) &&
          !File(path).existsSync();
    }

    // Preserve a conventional detached slot name when it is genuinely free;
    // otherwise allocate the first free game slot deterministically.
    final sourceStem = ctx.basenameWithoutExtension(source.path).toUpperCase();
    final sourceMatch = RegExp(r'^G1R-(\d{3})$').firstMatch(sourceStem);
    final sourceNumber = sourceMatch == null
        ? null
        : int.tryParse(sourceMatch.group(1)!);
    if (sourceNumber != null &&
        sourceNumber >= 1 &&
        sourceNumber <= 999 &&
        available(sourceStem)) {
      return ctx.join(state.saveDir, '$sourceStem.sav');
    }

    for (var number = 1; number <= 999; number++) {
      final slot = 'G1R-${number.toString().padLeft(3, '0')}';
      if (available(slot)) return ctx.join(state.saveDir, '$slot.sav');
    }
    return null;
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
        state = state.copyWith(error: _l10n.editorLoadBackupsFailed('$error'));
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
      state = state.copyWith(error: _l10n.editorUnsavedBeforeRestoreProfile);
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
        state = state.copyWith(
          error: _l10n.editorRestoreFailed(_errorDetails(response)),
        );
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
      final restoreMessage =
          companionPresent && !companionRestored && !targetIsPdl
          ? _l10n.editorRestoredBackupWithoutCompanion(backupPath)
          : _l10n.editorRestoredBackup(backupPath);
      // The bytes are the backup's either way; only the undo notes that describe
      // them failed to follow. Unreported, the restored save can hold a pinned
      // NPC while the sidecar says nothing about the routine that pin replaced.
      final noteWarning = data?['placementNoteWarning'];
      state = state.copyWith(
        lastWriteMessage: noteWarning is String && noteWarning.isNotEmpty
            ? '$restoreMessage\n'
                  '${_l10n.editorPlacementNoteFailed(noteWarning)}'
            : restoreMessage,
      );
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
          error: _l10n.editorRestoreReloadFailed(backupPath, state.error!),
        );
      }
    }, failureMessage: (details) => _l10n.editorRestoreFailed(details));
  }

  /// Delete one backup of the selected save (or of `targetPath`). The core only
  /// accepts a file its own backup listing produced, so this can never remove
  /// the live save or another slot's snapshot.
  Future<void> deleteBackup(String backupPath, {String? targetPath}) async {
    final path = targetPath ?? state.selectedPath;
    if (path == null) return;
    await _withLoading(() async {
      final response = await _execute(
        'delete_backup',
        payload: {'path': path, 'backupPath': backupPath},
      );
      if (response['ok'] != true) {
        state = state.copyWith(
          error: _l10n.editorDeleteBackupFailed(_errorDetails(response)),
        );
        return;
      }
      // The core deletes first and tidies the name afterwards, so a name it
      // could not drop comes back as a warning on an otherwise successful
      // response. Say so: the leftover would otherwise be inherited unannounced
      // by the next backup that lands under the same file name.
      final data = (response['data'] as Map?);
      final warning = data?['labelWarning'];
      var message = warning is String && warning.isNotEmpty
          ? _l10n.editorDeletedBackupWithLabelWarning(backupPath, warning)
          : _l10n.editorDeletedBackup(backupPath);
      // Same story for the undo notes this backup carried: a snapshot that
      // could not be dropped would be inherited by the next backup to land
      // under the same file name.
      final noteWarning = data?['placementNoteWarning'];
      if (noteWarning is String && noteWarning.isNotEmpty) {
        message =
            '$message\n${_l10n.editorPlacementNoteFailed(noteWarning)}';
      }
      state = state.copyWith(lastWriteMessage: message);
      await refreshBackups();
    }, failureMessage: (details) => _l10n.editorDeleteBackupFailed(details));
  }

  /// Label one backup of the selected save (or of `targetPath`). An empty name
  /// clears the label. The backup FILE keeps its own name either way — it
  /// encodes which save it belongs to and when it was taken.
  Future<void> renameBackup(
    String backupPath,
    String name, {
    String? targetPath,
  }) async {
    final path = targetPath ?? state.selectedPath;
    if (path == null) return;
    await _withLoading(() async {
      final response = await _execute(
        'rename_backup',
        payload: {'path': path, 'backupPath': backupPath, 'name': name},
      );
      if (response['ok'] != true) {
        state = state.copyWith(
          error: _l10n.editorRenameBackupFailed(_errorDetails(response)),
        );
        return;
      }
      await refreshBackups();
    }, failureMessage: (details) => _l10n.editorRenameBackupFailed(details));
  }

  Future<void> checkCodec() async {
    try {
      final response = await _execute('check_codec');
      if (response['ok'] != true) {
        // Use the dedicated codec error channel so a concurrent/later refresh
        // does not wipe this message, and drop the now-stale codec status so
        // the UI doesn't keep showing an earlier "ready" state.
        state = state.copyWith(
          codecError: _l10n.editorCodecCheckFailed(_errorDetails(response)),
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
        codecError: _l10n.editorCodecCheckFailed('$error'),
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
        state = state.copyWith(
          error: _l10n.editorCodecValidationFailed(_errorDetails(response)),
        );
        return;
      }
      final data = (response['data'] as Map).cast<String, Object?>();
      state = state.copyWith(
        lastWriteMessage: _l10n.editorCodecRoundtripPassed(
          (data['chunkIndex'] as num?)?.toInt() ?? 0,
          (data['recompressedSize'] as num?)?.toInt() ?? 0,
        ),
      );
    }, failureMessage: (details) => _l10n.editorCodecValidationFailed(details));
  }

  /// Search every typed property in the decoded private payload. The core
  /// caches the decoded payload, so the first search pays the decode cost and
  /// later searches are instant. Returns a result carrying an error string
  /// instead of throwing, so the browser UI can render it inline.
  Future<TypedSearchResult> searchTypedProperties(
    String query, {
    int offset = 0,
    int limit = 50,
    String source = 'private',
    bool includeNodes = false,
    String? kind,
    String? type,
    bool? editable,
  }) async {
    final path = state.selectedPath;
    if (path == null) {
      return TypedSearchResult(error: _l10n.editorNoSaveSelected);
    }
    try {
      final response = await _execute(
        'search_typed_properties',
        payload: {
          'path': path,
          'query': query,
          'offset': offset,
          'limit': limit,
          if (includeNodes) 'includeNodes': true,
          if (includeNodes) 'source': source,
          if (includeNodes && kind != null && kind != 'all') 'kind': kind,
          if (includeNodes && type != null && type.trim().isNotEmpty)
            'type': type.trim(),
          if (includeNodes && editable != null) 'editable': editable,
        },
      );
      if (response['ok'] != true) {
        return TypedSearchResult(
          error: _l10n.editorPropertySearchFailed(_errorDetails(response)),
        );
      }
      return TypedSearchResult.fromJson(
        (response['data'] as Map).cast<String, Object?>(),
      );
    } catch (error) {
      return TypedSearchResult(
        error: _l10n.editorPropertySearchFailed('$error'),
      );
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
        return HeroAttributesResult(
          error: _l10n.editorSelectionChangedWhileLoadingHeroAttributes,
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
      return SkillsResult(error: _l10n.editorNoSaveSelected);
    }
    try {
      final response = await _execute(
        'private.skills.list',
        payload: {'path': path, 'actor': actor},
      );
      if (response['ok'] != true) {
        return SkillsResult(
          error: _l10n.editorSkillsLoadFailed(_errorDetails(response)),
        );
      }
      return SkillsResult.fromJson(
        (response['data'] as Map).cast<String, Object?>(),
      );
    } catch (error) {
      return SkillsResult(error: _l10n.editorSkillsLoadFailed('$error'));
    }
  }

  /// Run one progression section query. Returns the raw data map, or null
  /// with [onError] called, so each typed loader below can build its own page
  /// object with an inline error.
  Future<Map<String, Object?>?> _queryProgression(
    Map<String, Object?> params, {
    required void Function(String message) onError,
    String? path,
  }) async {
    final resolvedPath = path ?? state.selectedPath;
    if (resolvedPath == null) {
      onError(_l10n.editorNoSaveSelected);
      return null;
    }
    try {
      final response = await _execute(
        'query_progression',
        payload: {'path': resolvedPath, ...params},
      );
      if (response['ok'] != true) {
        onError(_l10n.editorProgressionQueryFailed(_errorDetails(response)));
        return null;
      }
      return (response['data'] as Map).cast<String, Object?>();
    } catch (error) {
      onError(_l10n.editorProgressionQueryFailed('$error'));
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

  /// Load the tutorial unlock gates that the game presents from its glossary.
  ///
  /// The core deliberately exposes these separately from normal journal quests
  /// and omits the structural `Quest_Tutorials` root. The response otherwise
  /// uses the same shape as a quest page so the existing typed model and edit
  /// intent can be reused without leaking tutorials back into the quest pane.
  Future<ProgressionQuestPage> loadProgressionTutorials({
    int offset = 0,
    int limit = 100,
  }) async {
    String? error;
    final data = await _queryProgression({
      'section': 'tutorials',
      'offset': offset,
      'limit': limit,
    }, onError: (message) => error = message);
    if (data == null) return ProgressionQuestPage(error: error);
    return ProgressionQuestPage.fromJson(data);
  }

  /// Load one page of the sparse save-backed story-property map and,
  /// optionally, the source-declared catalog entries absent from that map.
  /// The core enriches serialized int32 values with their declared game-script
  /// type, allowing the UI to distinguish in-game timestamps from integers.
  /// [path] lets a multi-page caller pin every page to the save where its load
  /// began, even if the active selection changes before the last page arrives.
  Future<StoryStatePage> loadStoryState({
    String query = '',
    int offset = 0,
    int limit = 1000,
    StorySemanticType? semanticType,
    bool includeUnset = false,
    String? path,
  }) async {
    String? error;
    final data = await _queryProgression(
      {
        'section': 'story',
        'query': query,
        'offset': offset,
        'limit': limit,
        if (includeUnset) 'includeUnset': true,
        if (semanticType != null) 'semanticType': semanticType.name,
      },
      path: path,
      onError: (message) => error = message,
    );
    if (data == null) return StoryStatePage(error: error);
    return StoryStatePage.fromJson(data);
  }

  /// Load the complete save-backed glossary in one query. Creature and
  /// location documents are returned as structured quest trees; the raw Hero
  /// segment unlocks in the same response are joined to the bundled NPC
  /// catalog by [GlossaryDetail].
  Future<GlossaryPage> loadGlossary() async {
    String? error;
    final data = await _queryProgression({
      'section': 'glossary',
      // The current game catalog is comfortably below this limit. Keeping the
      // glossary client-side makes category/search filters instant.
      'offset': 0,
      'limit': 1000,
    }, onError: (message) => error = message);
    if (data == null) return GlossaryPage(error: error);
    return GlossaryPage.fromJson(data);
  }

  static String _glossaryPendingKey(
    String documentClass,
    String segmentClass,
  ) => 'glossary.segment:$documentClass::$segmentClass';

  /// Queue one atomic glossary segment toggle. Each segment deliberately owns
  /// its own pending key because the core may splice the Hero memory array;
  /// [saveAllPending] therefore writes every toggle in a separately reparsed
  /// save round.
  void setPendingGlossarySegment(GlossarySegmentEdit edit) {
    setPendingEdit(
      _glossaryPendingKey(edit.documentClass, edit.segmentClass),
      PendingSaveEdit(edits: [edit.toEditJson()]),
    );
  }

  /// Drop a queued segment toggle when the switch returns to its on-disk value.
  void clearPendingGlossarySegment(String documentClass, String segmentClass) {
    clearPendingEdit(_glossaryPendingKey(documentClass, segmentClass));
  }

  /// Return the queued target for one glossary segment. The glossary panel
  /// uses this after an inspection refresh so a structural edit left pending
  /// by a partially failed multi-write remains visible to the user.
  bool? pendingGlossarySegment(String documentClass, String segmentClass) {
    final pending = pendingEditFor(
      _glossaryPendingKey(documentClass, segmentClass),
    );
    if (pending == null) return null;
    for (final edit in pending.edits) {
      if (edit['path'] != 'private.glossary.setSegment') continue;
      final value = edit['value'];
      if (value is! Map || value['unlocked'] is! bool) continue;
      if (value['documentClass'] != documentClass ||
          value['segmentClass'] != segmentClass) {
        continue;
      }
      return value['unlocked'] as bool;
    }
    return null;
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
      return NpcActorsPage(error: _l10n.editorNoSaveSelected);
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
        return NpcActorsPage(
          error: _l10n.editorNpcListFailed(_errorDetails(response)),
        );
      }
      return NpcActorsPage.fromJson(
        (response['data'] as Map).cast<String, Object?>(),
      );
    } catch (error) {
      return NpcActorsPage(error: _l10n.editorNpcListFailed('$error'));
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
      return CharacterIndexPage(error: _l10n.editorNoSaveSelected);
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
        return CharacterIndexPage(
          error: _l10n.editorCharacterListFailed(_errorDetails(response)),
        );
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
      return CharacterIndexPage(
        error: _l10n.editorCharacterListFailed('$error'),
      );
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
  final Map<String, Future<NpcPoseResult>> _npcPositionCache = {};

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
      _npcPositionCache.clear();
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
    _npcPositionCache.clear();
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
      return Future.value(
        NpcAttributesResult(error: _l10n.editorNoSaveSelected),
      );
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
          return NpcAttributesResult(
            error: _l10n.editorNpcAttributesFailed(_errorDetails(response)),
          );
        }
        return NpcAttributesResult.fromJson(
          (response['data'] as Map).cast<String, Object?>(),
        );
      } catch (error) {
        _npcAttributesCache.remove(id);
        return NpcAttributesResult(
          error: _l10n.editorNpcAttributesFailed('$error'),
        );
      }
    }();
    _npcAttributesCache[id] = future;
    return future;
  }

  /// Load a single NPC's saved pose (by GlobalId) from the core
  /// `private.npc.position` command for the currently selected save: the
  /// character location/rotation plus the spawn location/rotation reference,
  /// each paired with the FULL typed path `private.typed.setValue` resolves —
  /// so the position editor registers its edits through the same pending
  /// mechanism the attribute editor uses (only the value is a struct, not a
  /// scalar). Rotations arrive as `{pitch, yaw, roll}`.
  ///
  /// Writing this pose is an OPEN QUESTION, deliberately re-enabled — see
  /// `NpcPositionPanel` for what the earlier in-game tests did and did not rule
  /// out.
  ///
  /// Memoized per (save, GlobalId) exactly like [loadNpcAttributes]; a failed
  /// load is NOT cached so a transient error can retry.
  Future<NpcPoseResult> loadNpcPosition(String id) {
    final path = state.selectedPath;
    if (path == null) {
      return Future.value(NpcPoseResult(error: _l10n.editorNoSaveSelected));
    }
    _guardNpcDetailCache(path);
    final cached = _npcPositionCache[id];
    if (cached != null) return cached;
    final future = () async {
      try {
        final response = await _execute(
          'private.npc.position',
          payload: {'path': path, 'id': id},
        );
        if (response['ok'] != true) {
          _npcPositionCache.remove(id);
          return NpcPoseResult(
            error: _l10n.editorNpcPositionFailed(_errorDetails(response)),
          );
        }
        return NpcPoseResult.fromJson(
          (response['data'] as Map).cast<String, Object?>(),
        );
      } catch (error) {
        _npcPositionCache.remove(id);
        return NpcPoseResult(error: _l10n.editorNpcPositionFailed('$error'));
      }
    }();
    _npcPositionCache[id] = future;
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
      return Future.value(
        NpcInventoryResult(error: _l10n.editorNoSaveSelected),
      );
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
          return NpcInventoryResult(
            error: _l10n.editorNpcInventoryFailed(_errorDetails(response)),
          );
        }
        return NpcInventoryResult.fromJson(
          (response['data'] as Map).cast<String, Object?>(),
        );
      } catch (error) {
        _npcInventoryCache.remove(id);
        return NpcInventoryResult(
          error: _l10n.editorNpcInventoryFailed('$error'),
        );
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

  static const _memoryEventPendingPrefix = 'progression.events:';

  /// Queue an index-addressed event edit for [character]. Multiple distinct
  /// removals are kept in descending original-index order; each becomes its own
  /// reparsed sub-write in [saveAllPending], so removing a higher index never
  /// shifts a lower pending target. Duplicate is intentionally exclusive with
  /// every other edit for the character because mixing insertion and removal
  /// intents makes the pending row indices ambiguous.
  ///
  /// Returns false when [edit] conflicts with an already-pending duplicate or
  /// removal. Re-queuing the same operation for the same index is idempotent.
  bool setPendingMemoryEventEdit(String character, MemoryEventEdit edit) {
    final existing = pendingMemoryEventEdits(character);
    for (final pending in existing) {
      if (pending.index != edit.index) continue;
      return pending.isRemove == edit.isRemove;
    }
    if (existing.isNotEmpty &&
        (!edit.isRemove || existing.any((pending) => !pending.isRemove))) {
      return false;
    }
    final updated = [...existing, edit]
      ..sort((left, right) => right.index.compareTo(left.index));
    setPendingEdit(
      '$_memoryEventPendingPrefix$character',
      PendingSaveEdit(
        edits: [for (final pending in updated) pending.toEditJson()],
      ),
    );
    return true;
  }

  /// Clear one pending event index, or every pending event for [character]
  /// when [index] is omitted.
  void clearPendingMemoryEventEdit(String character, {int? index}) {
    final key = '$_memoryEventPendingPrefix$character';
    if (index == null) {
      clearPendingEdit(key);
      return;
    }
    final pending = pendingEditFor(key);
    if (pending == null) return;
    final remaining = pending.edits.where((raw) {
      final parsed = MemoryEventEdit.fromEditJson(raw);
      return parsed == null || parsed.index != index;
    }).toList();
    if (remaining.length == pending.edits.length) return;
    if (remaining.isEmpty) {
      clearPendingEdit(key);
    } else {
      setPendingEdit(key, PendingSaveEdit(edits: remaining));
    }
  }

  List<MemoryEventEdit> pendingMemoryEventEdits(String character) {
    final pending = pendingEditFor('$_memoryEventPendingPrefix$character');
    if (pending == null) return const [];
    return pending.edits
        .map(MemoryEventEdit.fromEditJson)
        .whereType<MemoryEventEdit>()
        .toList(growable: false);
  }

  /// Backwards-compatible singular view used by older callers/tests.
  MemoryEventEdit? pendingMemoryEventEdit(String character) {
    final pending = pendingMemoryEventEdits(character);
    return pending.length == 1 ? pending.single : null;
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

  static String _npcRelationshipPendingKey(String id) => 'npc.relationship:$id';

  /// Register an explicit permanent NPC-to-Hero relationship override under
  /// its own structural pending key. The game otherwise derives this value at
  /// runtime from rules that are not persisted as one save field.
  void setPendingNpcRelationship(String id, NpcRelationship relationship) {
    setPendingEdit(
      _npcRelationshipPendingKey(id),
      PendingSaveEdit(
        edits: [
          {
            'path': 'private.npc.setRelationship',
            'value': {'id': id, 'relationship': relationship.wireValue},
          },
        ],
      ),
    );
  }

  void clearPendingNpcRelationship(String id) {
    clearPendingEdit(_npcRelationshipPendingKey(id));
  }

  /// Rehydrate the optimistic dropdown value from the pending registry when a
  /// user revisits this NPC before saving.
  NpcRelationship? pendingNpcRelationship(String id) {
    final pending = pendingEditFor(_npcRelationshipPendingKey(id));
    if (pending == null) return null;
    for (final edit in pending.edits) {
      if (edit['path'] != 'private.npc.setRelationship') continue;
      final value = edit['value'];
      if (value is! Map || value['relationship'] is! String) continue;
      return NpcRelationship.fromJson(value['relationship']);
    }
    return null;
  }

  /// Load the player's per-guild crime tally from the core
  /// `private.factions.list` command for the currently selected save. Returns a
  /// page carrying an inline error instead of throwing, mirroring
  /// [loadNpcAttributes].
  Future<FactionsPage> loadFactions() async {
    final path = state.selectedPath;
    if (path == null) {
      return FactionsPage(error: _l10n.editorNoSaveSelected);
    }
    try {
      final response = await _execute(
        'private.factions.list',
        payload: {'path': path},
      );
      if (response['ok'] != true) {
        return FactionsPage(
          error: _l10n.editorFactionListFailed(_errorDetails(response)),
        );
      }
      return FactionsPage.fromJson(
        (response['data'] as Map).cast<String, Object?>(),
      );
    } catch (error) {
      return FactionsPage(error: _l10n.editorFactionListFailed('$error'));
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

  String _errorDetails(Map<String, Object?> response) {
    final error = (response['error'] as Map?)?.cast<String, Object?>();
    return error?['message'] as String? ?? _l10n.coreUnknownError;
  }

  String _backupMessage(String prefix, Map<String, Object?> data) {
    final backupPath =
        data['backupPath']?.toString() ?? _l10n.editorNoBackupPath;
    final persistentBackupPath = data['persistentBackupPath'] as String?;
    if (persistentBackupPath == null || persistentBackupPath.isEmpty) {
      return _l10n.editorBackupMessage(prefix, backupPath);
    }
    return _l10n.editorBackupMessageWithPersistent(
      prefix,
      backupPath,
      persistentBackupPath,
    );
  }

  Future<_BackupSnapshot?> _loadBackups(String path, int seq) async {
    final response = await _execute('list_backups', payload: {'path': path});
    // Only the latest load applies; a superseded load must not replace the
    // fresher list with its outdated result.
    if (seq != _loadSeq) return null;
    if (response['ok'] != true) {
      // Leave isLoading to the caller's load-counter bookkeeping.
      state = state.copyWith(
        error: _l10n.editorLoadBackupsFailed(_errorDetails(response)),
      );
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
    _settingsStore.write(
      EditorSettings(
        saveDir: state.saveDir,
        externalSavePaths: state.externalSavePaths,
        hiddenOtherSavePaths: state.hiddenOtherSavePaths,
      ),
    );
  }

  static EditorState _initialState({
    required String? saveDir,
    required EditorSettingsStore settingsStore,
  }) {
    final stored = settingsStore.read();
    return EditorState(
      saveDir: saveDir ?? stored.saveDir ?? defaultSaveRoot(),
      externalSavePaths: stored.externalSavePaths,
      hiddenOtherSavePaths: stored.hiddenOtherSavePaths,
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

class _IndexedStructuralEdit {
  const _IndexedStructuralEdit({
    required this.keyed,
    required this.index,
    required this.isDuplicate,
  });

  final _KeyedEdit keyed;
  final int index;
  final bool isDuplicate;
}

class _StructuralArrayGroup {
  _StructuralArrayGroup(this.path);

  final List<Object?> path;
  final List<_IndexedStructuralEdit> edits = [];
}

bool _sameEditorPath(List<Object?> left, List<Object?> right) {
  if (left.length != right.length) return false;
  for (var i = 0; i < left.length; i++) {
    if (left[i] != right[i]) return false;
  }
  return true;
}

bool _editorPathIsPrefix(List<Object?> prefix, List<Object?> path) {
  if (prefix.length > path.length) return false;
  for (var i = 0; i < prefix.length; i++) {
    if (prefix[i] != path[i]) return false;
  }
  return true;
}

bool _addressesHeroMemorizedEvents(List<Object?> path) {
  const target = <String>[
    'LongTermMemoryByGlobalId',
    '{Hero}',
    'MemorizedEvents',
  ];
  if (path.length < target.length) return false;
  for (var start = 0; start <= path.length - target.length; start++) {
    var matches = true;
    for (var offset = 0; offset < target.length; offset++) {
      if (path[start + offset] != target[offset]) {
        matches = false;
        break;
      }
    }
    if (matches) return true;
  }
  return false;
}

/// Whether [path] targets the relationship map itself or an entry belonging to
/// one of [npcIds] (already normalized to lower case). The generic All-data
/// browser represents map keys as `{GlobalId}` path segments.
bool _addressesNpcRelationshipEntry(List<Object?> path, Set<String> npcIds) {
  for (var i = 0; i < path.length; i++) {
    if (path[i] != 'RelationshipByGlobalId') continue;
    // A hypothetical edit of the whole map collides with every structured
    // relationship write, even though current UI operations normally descend
    // to an individual entry first.
    if (i + 1 >= path.length) return true;
    final rawKey = path[i + 1];
    if (rawKey is! String) return true;
    final key = rawKey.startsWith('{') && rawKey.endsWith('}')
        ? rawKey.substring(1, rawKey.length - 1)
        : rawKey;
    return npcIds.contains(key.trim().toLowerCase());
  }
  return false;
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
/// Every raw typed operation, whether it writes a value or mutates a container.
/// All of them address their target the same way, through `value.path`.
const _typedEditPaths = {
  'private.typed.setValue',
  'private.typed.setAdd',
  'private.typed.setRemove',
  'private.typed.arrayRemove',
  'private.typed.arrayDuplicate',
};

/// A raw typed edit that reaches a slot — INTO one (its id, its count, a set or
/// array inside its payload, anything below `m_Slots/[i]`) or AT the slot array
/// itself, which an array operation addresses by ending at `m_Slots` and naming
/// its element in `value.index`.
///
/// An add or a removal claims whole slots, so either shape collides with it: the
/// add fills a blank slot the splice may then delete, and a splice of the array
/// shifts every later slot away from its id again.
///
/// Matched on the `m_Slots` step rather than on an ancestor name: only the
/// PLAYER inventory sits under an `m_Inventory` segment, while an NPC's lives
/// under `InventoryByGlobalId{id}/InventoryItems/…` (see
/// `npc::npc_inventory_path`), and both are rewritten alike.
@visibleForTesting
bool isInventorySlotTypedEdit(Map<String, Object?> edit) {
  if (!_typedEditPaths.contains(edit['path'])) return false;
  final path = (edit['value'] as Map?)?['path'];
  if (path is! List) return false;
  final segments = path.whereType<String>().toList();
  if (segments.isNotEmpty && segments.last == 'm_Slots') return true;
  for (var index = 0; index + 1 < segments.length; index++) {
    if (segments[index] != 'm_Slots') continue;
    final slot = segments[index + 1];
    if (slot.startsWith('[') && slot.endsWith(']')) return true;
  }
  return false;
}

/// A raw typed edit that writes a slot's `m_Id` — the one field the whole-save
/// repair rewrites, and therefore the only one it can collide with. Anything
/// else inside a slot survives the repair untouched.
@visibleForTesting
bool isInventorySlotIdTypedEdit(Map<String, Object?> edit) {
  if (!_typedEditPaths.contains(edit['path'])) return false;
  final path = (edit['value'] as Map?)?['path'];
  if (path is! List) return false;
  final segments = path.whereType<String>().toList();
  if (segments.length < 3 || segments.last != 'm_Id') return false;
  final slot = segments[segments.length - 2];
  return segments[segments.length - 3] == 'm_Slots' &&
      slot.startsWith('[') &&
      slot.endsWith(']');
}

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
    this.placementNotes = const [],
    this.clearPlacementNotes = const [],
  });

  final List<Map<String, Object?>> edits;
  final bool syncPersistentDataList;
  final List<Map<String, Object?>> placementNotes;
  final List<String> clearPlacementNotes;
}
