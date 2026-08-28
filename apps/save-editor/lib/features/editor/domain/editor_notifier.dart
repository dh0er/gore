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
import 'package:goresave/features/editor/domain/trader_models.dart';
import 'package:goresave/l10n/app_localizations.dart';
import 'package:goresave/l10n/app_localizations_en.dart';
import 'package:goresave/utils/default_paths.dart';
import 'package:path/path.dart' as p;
import 'package:state_notifier/state_notifier.dart';

const _unchanged = Object();

/// The page sizes the editor's panels ask the core for.
///
/// These live here rather than in each panel because the core caches one
/// response per exact request, and [EditorNotifier.prefetchTabData] warms those
/// caches by issuing the panels' own queries ahead of time. A panel that quietly
/// chose its own size would be warmed with an answer it never asks for.
abstract final class EditorPageSize {
  /// One screen of rows: knowledge entries, memory events, the property browser.
  static const detail = 50;

  /// Overview aggregates Hero combat events in larger pages so it reaches the
  /// final totals without dozens of serialized round trips.
  static const statistics = 500;

  /// Fetched whole and then filtered/paged in the client: quests, tutorials,
  /// story state.
  static const fullList = 1000;
}

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
    this.deletedSaveRecovery,
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

  /// One-click recovery target retained after a backed-up save deletion.
  ///
  /// The deleted slot disappears from the scan immediately, so its regular
  /// Backups tab can no longer address the snapshot. Keep the exact live and
  /// backup paths until the user restores it or explicitly dismisses its
  /// dedicated recovery banner.
  final DeletedSaveRecovery? deletedSaveRecovery;

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
    Object? deletedSaveRecovery = _unchanged,
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
    bool clearDeletedSaveRecovery = false,
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
      deletedSaveRecovery: clearDeletedSaveRecovery
          ? null
          : identical(deletedSaveRecovery, _unchanged)
          ? this.deletedSaveRecovery
          : deletedSaveRecovery as DeletedSaveRecovery?,
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
    void Function(Map<String, Object?> data)? onSuccess,
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
      onSuccess?.call(data);
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

  // The Overview header, statistics section, and background prefetch are
  // mounted at nearly the same time and request the same read-only sources.
  // Share only their in-flight futures: this removes duplicate work from the
  // serialized native queue while a later refresh still performs a fresh read.
  String? _sharedReadPath;
  Future<GameTime?>? _gameTimeInFlight;
  Future<HeroAttributesResult>? _heroAttributesInFlight;
  Future<SkillsResult>? _skillsInFlight;
  Future<CharacterIndexPage>? _charactersInFlight;

  void _invalidateSharedReads() {
    _sharedReadPath = null;
    _gameTimeInFlight = null;
    _heroAttributesInFlight = null;
    _skillsInFlight = null;
    _charactersInFlight = null;
  }

  void _guardSharedReads(String? path) {
    if (_sharedReadPath == path) return;
    _invalidateSharedReads();
    _sharedReadPath = path;
  }

  /// The inspection the background prefetch warmed IN FULL, so re-entering the
  /// editor for an unchanged save does not queue the same warm-up twice.
  ///
  /// Set only once every step has run. A warm-up that was cut short — the user
  /// renamed a backup, ran a codec check — leaves this null so the next state
  /// change starts it again; the steps that did complete are answered from the
  /// core's cache, so a restart re-walks them for a few milliseconds.
  SaveInspection? _prefetchedFor;

  /// Whether a warm-up is running right now. [_prefetchedFor] cannot serve as
  /// this flag any more (it is only set at the end), and the warm-up itself
  /// changes editor state — the character index settles the hero id — so
  /// without this a state change mid-warm-up would start a second one.
  bool _prefetchRunning = false;

  /// The in-flight prefetch, exposed so a test can await the warm-up instead of
  /// racing it. Production fires and forgets.
  @visibleForTesting
  Future<void>? prefetchInFlight;

  /// Warm the core's caches for every tab of the freshly inspected save.
  ///
  /// The core answers a repeated read from a cache keyed by the save's content,
  /// so running the panels' own queries here turns the first visit to a tab from
  /// a fresh multi-hundred-millisecond traversal into a cache hit. Nothing here
  /// touches [EditorState.isLoading] or reports an error: the user is looking at
  /// the Overview tab while it runs, and a warm-up that fails simply leaves the
  /// panel to load the normal way.
  ///
  /// The queries must match what the panels ask for, argument for argument —
  /// the cache holds one response per exact request, so a warm-up with a
  /// different page size would prime an answer nobody asks for. That is why the
  /// page sizes live in [EditorPageSize] rather than in each panel.
  void prefetchTabData() {
    // The page listens for state changes to trigger this, and a change can still
    // be delivered while the provider is being torn down (a hot restart, the
    // window closing). Reading `state` then throws.
    if (!mounted) return;
    final inspection = state.inspection;
    final path = state.selectedPath;
    if (inspection == null || path == null) return;
    // The inspection lands BEFORE its load finishes — `_inspect` still has the
    // backup list to fetch — and every warm-up step bails out while a load is in
    // flight. Claiming the inspection here would therefore burn it on a warm-up
    // that does nothing, and the identity check below would refuse to try again.
    // Wait instead: clearing the loading flag is itself a state change, so the
    // page calls this again, and that call starts the warm-up for real.
    if (state.isLoading) return;
    if (_prefetchRunning) return;
    if (identical(_prefetchedFor, inspection)) return;
    _prefetchRunning = true;
    prefetchInFlight = _prefetchTabData(path, inspection, _loadSeq).whenComplete(
      () {
        _prefetchRunning = false;
        // A run that was cut short cannot simply wait for the next state
        // change: a step already in flight keeps this flag up past the moment
        // the interrupting operation clears the loading flag, so the state
        // change that would have restarted the warm-up bounces off the guard
        // above and never comes again. Re-arm here instead. This cannot spin —
        // a fresh run takes the current load sequence, so it can only be cut
        // short by a NEW interruption, and the guards decide whether it may
        // start at all.
        if (!identical(_prefetchedFor, inspection)) prefetchTabData();
      },
    );
  }

  /// Warm every tab's query for [inspection], in reachability order.
  ///
  /// Steps are skipped, never queued, while something else holds the editor:
  /// a warm-up that queued behind the user's own request would be the very
  /// stall it exists to remove. A skipped step is not lost — the inspection is
  /// then not marked warmed, so the next state change runs the sequence again
  /// and the steps that did complete come back from the core's cache.
  Future<void> _prefetchTabData(
    String path,
    SaveInspection inspection,
    int seq,
  ) async {
    // The story panel pins its pages to the path as the INSPECTION spells it;
    // the selection's spelling would warm a request the panel never makes.
    final inspectionPath = inspection.path;
    // A newer load (or a write) has taken over: continuing would only make the
    // user's request wait behind ours. A disposed notifier stops it too — the
    // editor is gone, and touching `state` after teardown throws.
    bool superseded() =>
        !mounted ||
        seq != _loadSeq ||
        state.selectedPath != path ||
        state.isLoading;

    var complete = true;
    Future<void> step(Future<Object?> Function() load) async {
      if (superseded()) {
        complete = false;
        return;
      }
      try {
        await load();
      } catch (_) {
        // A warm-up failure is not the user's problem; the panel will retry.
      }
    }

    // Ordered by how soon the user can reach the data: the Overview tab is
    // already on screen, Characters is one click away, then World, then the
    // property browser.
    await step(loadGameTime);
    // Also settles the hero GlobalId that the player's Events sub-tab needs.
    await step(loadAllCharacters);
    await step(loadHeroAttributes);
    await step(loadSkills);
    // The Overview is already visible. Warm every page its aggregate combat
    // statistics ask for before the broad tab prefetch can occupy the queue.
    final overviewHeroId = superseded() ? null : state.heroGlobalId;
    if (overviewHeroId != null) {
      await step(
        () => _prefetchAllPages(superseded, (offset) async {
          final page = await loadMemoryEvents(
            overviewHeroId,
            offset: offset,
            limit: EditorPageSize.statistics,
          );
          return (total: page.total, count: page.events.length);
        }),
      );
    }
    // Warms the CORE's cache without filling the Dart-side NPC memo. That memo
    // is pinned to one inspection by design, so pre-filling it here would hand
    // the first NPC panel a roster fetched seconds earlier; letting the panel
    // fill it on first use keeps it derived from the file as of that moment,
    // and the paging it repeats is answered from the warm core.
    await step(
      () => _fetchAllNpcActors(
        path,
        dropMemoOnError: false,
        superseded: superseded,
      ),
    );
    await step(
      () => loadKnowledgeEntries(
        const Actor.player().uniqueName,
        limit: EditorPageSize.detail,
      ),
    );
    // The player's Events pane keys on the hero id the character index above
    // settles. Without one there is nothing to warm — and nothing the pane will
    // ask for either.
    final heroId = superseded() ? null : state.heroGlobalId;
    if (heroId != null) {
      await step(() => loadMemoryEvents(heroId, limit: EditorPageSize.detail));
    }
    await step(
      () => _prefetchAllPages(superseded, (offset) async {
        final page = await loadProgressionQuests(
          offset: offset,
          limit: EditorPageSize.fullList,
          path: path,
        );
        return (total: page.total, count: page.quests.length);
      }),
    );
    await step(loadGlossary);
    await step(loadProgressionTutorials);
    await step(
      () => _prefetchAllPages(superseded, (offset) async {
        final page = await loadStoryState(
          includeUnset: true,
          offset: offset,
          limit: EditorPageSize.fullList,
          path: inspectionPath,
        );
        return (total: page.total, count: page.values.length);
      }),
    );
    await step(loadFactions);
    await step(
      () => searchTypedProperties(
        '',
        limit: EditorPageSize.detail,
        includeNodes: true,
      ),
    );

    // (see `_prefetchAllPages` for why the two full-list sections above walk
    // their pages instead of warming the first one.)

    // Last, and deliberately so. Everything reading private data shares the
    // core's single decoded payload and parsed tree, and the per-NPC panels are
    // far too numerous to warm one by one — so the tree itself has to be warmed.
    // Loading a save normally leaves the core holding it already, making this a
    // few milliseconds; the case that costs is returning to a save opened
    // earlier, where the core holds whichever save came in between. But that is
    // exactly the case where every step above is a cached answer, and this one
    // would hold the queue for a second in front of them. So warm the tabs the
    // user can click first, and rebuild the tree behind them, in time for the
    // first NPC they open.
    await step(() => _warmPrivateTree(path));

    // Only a run that warmed everything retires this inspection. Anything less
    // leaves the marker unset so the next state change picks the sequence up
    // again — otherwise a warm-up interrupted by, say, a backup rename would
    // leave the tabs it never reached loading the slow way for the rest of the
    // session.
    if (complete && mounted) _prefetchedFor = inspection;
  }

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

  /// Dismiss the one-click recovery for the most recently deleted save.
  void dismissDeletedSaveRecovery() {
    if (state.deletedSaveRecovery != null) {
      state = state.copyWith(clearDeletedSaveRecovery: true);
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
            key.startsWith('npc.attributes:') ||
            key.startsWith('npc.position:'),
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
    //
    // This has to catch every pair the core's own rule claims, or the packer
    // splits the pair into two writes and lets the later one win in silence.
    // So: any raw typed operation, not only a value write; the quest path under
    // either of the two names the core reads it from; and the paths compared the
    // way the core compares them, where an index segment is a number and [04]
    // and [4] are one and the same.
    if (hasGlossarySegmentEdit) {
      final rawTypedPaths = <List<Object?>>[];
      for (final keyed in allEdits) {
        final editPath = keyed.edit['path'];
        if (editPath is! String || !editPath.startsWith('private.typed.')) {
          continue;
        }
        final value = keyed.edit['value'];
        if (value is! Map) continue;
        final rawPath = value['path'];
        if (rawPath is List) rawTypedPaths.add(List<Object?>.from(rawPath));
      }
      for (final keyed in allEdits) {
        final edit = keyed.edit;
        if (edit['path'] != 'private.glossary.setSegment') continue;
        final value = edit['value'];
        if (value is! Map) continue;
        final rawQuestPath = value['questStatePath'] ?? value['statePath'];
        if (rawQuestPath is! List) continue;
        final questPath = List<Object?>.from(rawQuestPath);
        if (!rawTypedPaths.any((path) => _sameCorePath(path, questPath))) {
          continue;
        }
        final path = questPath.join(' › ');
        state = state.copyWith(error: _l10n.editorGlossaryQuestConflict(path));
        return false;
      }
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
      // Both splice a trader's stock map, which shifts every later byte offset
      // and renumbers the map's entry indices. private.traders.setStock is
      // deliberately absent: it overwrites a bare i32 in place, so it batches.
      'private.traders.addItem',
      'private.traders.removeItem',
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
    // A trade change and a raw array operation on the trader array cannot be
    // rescued by putting them in different writes: the trade change's row index
    // came from a list read before either ran, so whichever goes second
    // resolves it against a layout the first moved. The core refuses the pair
    // inside one write; splitting them here would slip past that and report
    // both as committed, so refuse before building the worklist.
    if (traderArrayConflict(allEdits.map((k) => k.edit).toList()) != null) {
      state = state.copyWith(error: _l10n.editorTraderArrayConflict);
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
    // Only one edit in this batch can move anything: a raw write to a slot's
    // m_Id, which renumbers the ids and positions that a count or an indexed
    // edit is addressed BY. Splitting the two would not make them safe — the
    // second write would still resolve an id the first had already moved — but
    // their order among the fixed edits is free, so let everything that moves
    // nothing go first, where it still resolves against the layout the user was
    // looking at. That is also the order the core accepts, so the pair keeps
    // sharing one write.
    final fixedMoversLast = [
      ...fixedBatch.where((k) => !_mayInvalidateOrdinals(k.edit)),
      ...fixedBatch.where((k) => _mayInvalidateOrdinals(k.edit)),
    ];

    // The edits in the exact order they must reach the core, which applies a batch
    // sequentially against one payload and re-resolves every edit's target as it
    // goes. All the ordering this method computed above is preserved by simple
    // concatenation:
    //  - the fixed batch leads. It carries syncPersistentDataList, so it is the
    //    backup-taking write; and a manual Health edit lands before a splicing
    //    npc.revive's HP restore, so the Revive action still wins as last writer.
    //  - the splices keep glossary adds ahead of removals and array removals
    //    index-descending.
    //  - skills follow, then the slot repair, so every id-addressed edit above
    //    resolved against the ids the user actually saw.
    //  - story goes last: it always needs its own write, and putting it at the end
    //    keeps it from taking the backup away from the syncPersistentDataList one.
    final ordered = <_KeyedEdit>[
      ...fixedMoversLast,
      ...orderedSplicing.where((k) => k.edit['path'] != storyStateApplyPath),
      ...skillEdits,
      ...repairEdits,
      ...orderedSplicing.where((k) => k.edit['path'] == storyStateApplyPath),
    ];

    // Pack that sequence into as few write_saves as the core will accept. It
    // refuses three combinations — an edit addressed by an index or slot id
    // placed after an edit that can change how many elements a container holds,
    // a raw typed edit sharing a write with a structured operation that rewrites
    // what it addresses, and two structured operations that rewrite one target
    // (the last two order-independent) — so a new sub-write starts exactly when
    // the next edit would hit any of them, plus one each for the two operations
    // that must stand alone. In practice a whole editing session lands in a
    // single write instead of one per splicing edit.
    //
    // A split is not a way to make a pair safe, only a way to keep the core from
    // refusing the whole write: the checks further up refuse the combinations
    // where running the two in sequence would resolve the second against a
    // layout the first moved.
    final worklist = <_SubWrite>[];
    var current = <Map<String, Object?>>[];
    var currentMayInvalidateOrdinals = false;
    // syncPersistentDataList keys off a public/fixed edit, so it belongs to the
    // first batch — which is also the one that takes the backup, so the companion
    // file is updated with a restorable snapshot beside it.
    var syncPending = syncPersistent;
    void flush() {
      if (current.isEmpty) return;
      worklist.add(
        _SubWrite(edits: current, syncPersistentDataList: syncPending),
      );
      syncPending = false;
      current = <Map<String, Object?>>[];
      currentMayInvalidateOrdinals = false;
    }

    for (final keyed in ordered) {
      if (_exclusiveEditPaths.contains(keyed.edit['path'])) {
        flush();
        worklist.add(_SubWrite(edits: [keyed.edit]));
        continue;
      }
      // Two reasons to start a new sub-write: the ordinal rule (positional —
      // only an ordinal-carrying edit AFTER an ordinal-invalidating one), and
      // the same-target rule (order-independent, so it is checked against every
      // edit already in this batch, both ways round).
      if ((currentMayInvalidateOrdinals && _carriesCallerOrdinal(keyed.edit)) ||
          current.any(
            (edit) =>
                editsRewriteSameTarget(edit, keyed.edit) ||
                structuredEditsShareATarget(edit, keyed.edit),
          )) {
        flush();
      }
      current.add(keyed.edit);
      currentMayInvalidateOrdinals =
          currentMayInvalidateOrdinals || _mayInvalidateOrdinals(keyed.edit);
    }
    flush();
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
      // A same-path re-inspection is still a new data generation. An older
      // paginated read may remain in flight across the serialized inspect;
      // never hand that mixed-generation future to the rebuilt Overview.
      _invalidateSharedReads();
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

  /// Delete a registered save file and remove all of its profile references.
  ///
  /// Native owns the paired compare-and-swap mutation and creates matching
  /// backups of the save and PersistentDataList before either live path
  /// changes. Missing references and detached files deliberately use their
  /// existing non-destructive flows instead.
  Future<bool> deleteSave({
    required String slot,
    required int profileId,
  }) async {
    if (state.isLoading) return false;
    // Keep the existing one-click recovery reachable. A second successful
    // deletion would otherwise replace its sole recovery token and strand the
    // first save's paired snapshots outside the UI.
    if (state.deletedSaveRecovery != null) return false;
    if (state.hasUnsavedEdits) {
      state = state.copyWith(error: _l10n.editorUnsavedBeforeDeleteSave);
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
              candidate.persistentProfileId == profileId &&
              !candidate.isExternal &&
              !candidate.isMissing,
        )
        .firstOrNull;
    if (save == null || !profile.savedSlots.contains(slot)) {
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
        save.path.contains('\\') || RegExp(r'^[A-Za-z]:').hasMatch(save.path);
    final ctx = isWindowsStyle ? p.Context(style: p.Style.windows) : p.posix;

    return _runWrite(
      command: 'delete_save',
      payload: {
        'path': save.path,
        'persistentPath': ctx.join(
          ctx.dirname(save.path),
          'PersistentDataList.sav',
        ),
        'slot': slot,
        'profileId': profileId,
        'backup': true,
      },
      failureMessage: (details) => _l10n.editorDeleteSaveFailed(details),
      message: (data) {
        var message = _backupMessage(_l10n.editorSaveDeleted, data);
        final noteWarning = data['placementNoteWarning'];
        if (noteWarning is String && noteWarning.isNotEmpty) {
          message = '$message\n${_l10n.editorPlacementNoteFailed(noteWarning)}';
        }
        return message;
      },
      onSuccess: (data) {
        final backupPath = data['backupPath'] as String?;
        final persistentPostDeleteSha1 =
            data['persistentPostDeleteSha1'] as String?;
        final deletedSaveSha1 = data['deletedSaveSha1'] as String?;
        final deletedPersistentSha1 = data['deletedPersistentSha1'] as String?;
        if (backupPath == null ||
            backupPath.isEmpty ||
            persistentPostDeleteSha1 == null ||
            persistentPostDeleteSha1.isEmpty ||
            deletedSaveSha1 == null ||
            deletedSaveSha1.isEmpty ||
            deletedPersistentSha1 == null ||
            deletedPersistentSha1.isEmpty) {
          return;
        }
        state = state.copyWith(
          clearWriteMessage: true,
          deletedSaveRecovery: DeletedSaveRecovery(
            targetPath: save.path,
            backupPath: backupPath,
            persistentPostDeleteSha1: persistentPostDeleteSha1,
            deletedSaveSha1: deletedSaveSha1,
            deletedPersistentSha1: deletedPersistentSha1,
            message: state.lastWriteMessage!,
          ),
        );
      },
      beforeRefresh: () {
        state = state.copyWith(
          externalSavePaths: _removeSavePath(
            state.externalSavePaths,
            save.path,
          ),
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
  Future<bool> restoreBackup(String backupPath, {String? targetPath}) async {
    return _restoreBackup('restore_backup', backupPath, targetPath: targetPath);
  }

  Future<bool> _restoreBackup(
    String command,
    String backupPath, {
    String? targetPath,
    String? expectedPersistentSha1,
    String? expectedSaveSha1,
    String? expectedPersistentBackupSha1,
  }) async {
    final path = targetPath ?? state.selectedPath;
    if (path == null) return false;
    var restored = false;
    await _withLoading(() async {
      final response = await _execute(
        command,
        payload: {
          'path': path,
          'backupPath': backupPath,
          'expectedPersistentSha1': ?expectedPersistentSha1,
          'expectedSaveSha1': ?expectedSaveSha1,
          'expectedPersistentBackupSha1': ?expectedPersistentBackupSha1,
        },
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
      restored = true;
    }, failureMessage: (details) => _l10n.editorRestoreFailed(details));
    return restored;
  }

  /// Restore the most recently deleted slot from the exact snapshot returned
  /// by `delete_save`. The core also restores its matching
  /// PersistentDataList backup when present.
  Future<void> restoreDeletedSave() async {
    final recovery = state.deletedSaveRecovery;
    if (recovery == null) return;
    if (state.hasUnsavedEdits) {
      state = state.copyWith(error: _l10n.editorUnsavedBeforeRestoreProfile);
      return;
    }
    final restored = await _restoreBackup(
      'restore_deleted_save',
      recovery.backupPath,
      targetPath: recovery.targetPath,
      expectedPersistentSha1: recovery.persistentPostDeleteSha1,
      expectedSaveSha1: recovery.deletedSaveSha1,
      expectedPersistentBackupSha1: recovery.deletedPersistentSha1,
    );
    if (restored) {
      state = state.copyWith(clearDeletedSaveRecovery: true);
    }
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
        message = '$message\n${_l10n.editorPlacementNoteFailed(noteWarning)}';
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
  Future<HeroAttributesResult> loadHeroAttributes() {
    final path = state.selectedPath;
    _guardSharedReads(path);
    final inFlight = _heroAttributesInFlight;
    if (inFlight != null) return inFlight;
    final future = _loadHeroAttributes();
    _heroAttributesInFlight = future;
    return future.whenComplete(() {
      if (identical(_heroAttributesInFlight, future)) {
        _heroAttributesInFlight = null;
      }
    });
  }

  Future<HeroAttributesResult> _loadHeroAttributes() async {
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
  Future<GameTime?> loadGameTime() {
    final path = state.selectedPath;
    _guardSharedReads(path);
    final inFlight = _gameTimeInFlight;
    if (inFlight != null) return inFlight;
    final future = _loadGameTime();
    _gameTimeInFlight = future;
    return future.whenComplete(() {
      if (identical(_gameTimeInFlight, future)) _gameTimeInFlight = null;
    });
  }

  Future<GameTime?> _loadGameTime() async {
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
  Future<SkillsResult> loadSkills({String actor = 'Hero'}) {
    // Only Hero participates in the shared Overview/prefetch load. Actor-aware
    // calls remain independent so a future non-Hero consumer cannot receive the
    // wrong roster.
    if (actor != 'Hero') return _loadSkills(actor);
    final path = state.selectedPath;
    _guardSharedReads(path);
    final inFlight = _skillsInFlight;
    if (inFlight != null) return inFlight;
    final future = _loadSkills(actor);
    _skillsInFlight = future;
    return future.whenComplete(() {
      if (identical(_skillsInFlight, future)) _skillsInFlight = null;
    });
  }

  Future<SkillsResult> _loadSkills(String actor) async {
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

  /// Load every merchant's shop record (`private.traders.list`).
  ///
  /// Returns a result carrying an inline [TradersResult.error] instead of
  /// throwing, and reports which trader commands this core build offers so the
  /// panel degrades to read-only against an older core rather than sending a
  /// command that does not exist.
  Future<TradersResult> loadTraders() async {
    final path = state.selectedPath;
    if (path == null) {
      return TradersResult(error: _l10n.editorNoSaveSelected);
    }
    try {
      final response = await _execute(
        'private.traders.list',
        payload: {'path': path},
      );
      if (response['ok'] != true) {
        return TradersResult(
          error: _l10n.editorTradersLoadFailed(_errorDetails(response)),
        );
      }
      return TradersResult.fromJson(
        (response['data'] as Map).cast<String, Object?>(),
      );
    } catch (error) {
      return TradersResult(error: _l10n.editorTradersLoadFailed('$error'));
    }
  }

  /// Load one merchant's full record by its `m_Traders` index.
  ///
  /// The index, not the name, is the address: two shipped rows are named `None`
  /// and the core refuses to guess between them.
  Future<TraderDetailResult> loadTraderDetail(int index) async {
    final path = state.selectedPath;
    if (path == null) {
      return TraderDetailResult(error: _l10n.editorNoSaveSelected);
    }
    try {
      final response = await _execute(
        'private.traders.detail',
        payload: {'path': path, 'index': index},
      );
      if (response['ok'] != true) {
        return TraderDetailResult(
          error: _l10n.editorTradersLoadFailed(_errorDetails(response)),
        );
      }
      return TraderDetailResult(
        detail: TraderDetail.fromJson(
          (response['data'] as Map).cast<String, Object?>(),
        ),
      );
    } catch (error) {
      return TraderDetailResult(error: _l10n.editorTradersLoadFailed('$error'));
    }
  }

  /// Queue one trader stock change. Re-editing the same line replaces its
  /// pending edit rather than stacking a second one.
  void setTraderStockEdit(TraderStockEdit edit) {
    setPendingEdit(edit.pendingKey, PendingSaveEdit(edits: [edit.toEdit()]));
  }

  /// Drop a queued trader change (the user reverted the field).
  void clearTraderStockEdit(TraderStockEdit edit) =>
      clearPendingEdit(edit.pendingKey);

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

  /// [path] lets a multi-page caller pin every page to the save its walk began
  /// against, so a selection change midway cannot make a later offset — derived
  /// from the previous file's total — query a different file. Defaults to the
  /// current selection, which is what the panel asks against.
  Future<ProgressionQuestPage> loadProgressionQuests({
    String query = '',
    int offset = 0,
    int limit = 100,
    String? state,
    String? group,
    String? path,
  }) async {
    String? error;
    final data = await _queryProgression(
      {
        'section': 'quests',
        'query': query,
        'offset': offset,
        'limit': limit,
        if (state != null && state.isNotEmpty) 'state': state,
        if (group != null && group.isNotEmpty) 'group': group,
      },
      path: path,
      onError: (message) => error = message,
    );
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
  ) =>
      'glossary.segment:${foldEditTargetPart(documentClass)}'
      '::${foldEditTargetPart(segmentClass)}';

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
  Future<CharacterIndexPage> loadAllCharacters() {
    final path = state.selectedPath;
    _guardSharedReads(path);
    final inFlight = _charactersInFlight;
    if (inFlight != null) return inFlight;
    final future = _loadAllCharacters();
    _charactersInFlight = future;
    return future.whenComplete(() {
      if (identical(_charactersInFlight, future)) _charactersInFlight = null;
    });
  }

  Future<CharacterIndexPage> _loadAllCharacters() async {
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
    final future = _fetchAllNpcActors(pinnedPath, dropMemoOnError: true);
    _allNpcActorsFuture = future;
    _allNpcActorsFor = inspection;
    return future;
  }

  /// Ask the core to make [path]'s decoded payload and parsed tree the ones it
  /// holds. Returns nothing: the point is the state it leaves behind, which
  /// every later private read shares.
  Future<void> _warmPrivateTree(String path) async {
    await _execute('warm_save', payload: {'path': path});
  }

  /// Warm every page a full-list panel will ask for.
  ///
  /// The quest and story panels fetch their section whole and filter it in the
  /// client, walking pages of [EditorPageSize.fullList] until they have `total`.
  /// The core clamps a page to 1000 and caches one response per exact request,
  /// so warming only the first page leaves a save that has outgrown that clamp
  /// to load its remaining pages cold on the tab's first visit — with the
  /// panel's spinner up, which is the wait this warm-up exists to remove.
  ///
  /// [page] must issue the panel's own request for an offset and report that
  /// page's `total` and item count. The offsets mirror the panels' arithmetic —
  /// items collected so far — because a different offset warms a request they
  /// never make. A save inside the clamp costs exactly one request, as before.
  ///
  /// [superseded] is checked before every page, not just before the walk: the
  /// offsets and total belong to the file the walk began against, so once
  /// something else takes over, every further page would occupy the core queue
  /// ahead of the user's own request to warm an offset nothing will ask for.
  /// Each page must also be pinned to that file for the same reason.
  Future<void> _prefetchAllPages(
    bool Function() superseded,
    Future<({int total, int count})> Function(int offset) page,
  ) async {
    var offset = 0;
    while (!superseded()) {
      final result = await page(offset);
      offset += result.count;
      // An empty page also covers the failure case, where the loader reports a
      // zero total: never loop on a stuck or erroring section.
      if (result.count == 0 || offset >= result.total) break;
    }
  }

  /// Page the full NPC roster out of the core, without touching the memo.
  ///
  /// The core clamps `private.npc.list` `limit` to 1000, but real saves hold
  /// ~1484+ NPCs — a single request would silently drop everyone past the first
  /// page. Pages are accumulated until `total` is reached and returned as one.
  /// [pinnedPath] fixes the file for the WHOLE fetch, so a save switch midway
  /// cannot merge pages from two different files into one list.
  ///
  /// [dropMemoOnError] belongs to the memoizing caller: a failed load must not
  /// stay cached, so it clears the memo slot the future was stored in. The
  /// background warm-up passes false — it has no slot to clear, and clearing the
  /// memo behind a real load in flight would be wrong.
  ///
  /// [superseded] likewise belongs to the warm-up: it abandons the walk when
  /// something else takes the editor, rather than keeping the core queue busy
  /// ahead of the user's own request. A real load passes none — a panel that
  /// asked for the roster needs all of it, not a prefix.
  Future<NpcActorsPage> _fetchAllNpcActors(
    String? pinnedPath, {
    required bool dropMemoOnError,
    bool Function()? superseded,
  }) async {
    final npcs = <NpcActor>[];
    var offset = 0;
    var total = 0;
    while (true) {
      if (superseded?.call() ?? false) break;
      final page = await loadNpcActors(
        offset: offset,
        limit: 1000,
        path: pinnedPath,
      );
      if (page.error != null) {
        if (dropMemoOnError) _invalidateNpcCache();
        return page;
      }
      npcs.addAll(page.npcs);
      total = page.total;
      offset += page.npcs.length;
      // Stop once we've collected every NPC, or the core returns an empty page
      // (defensive: never loop forever on a stuck/empty response).
      if (page.npcs.isEmpty || offset >= total) break;
    }
    return NpcActorsPage(npcs: npcs, total: total, offset: 0, limit: total);
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

  /// Folded like the core folds the id it keys the operation by, so two entries
  /// for one NPC cannot sit in the registry side by side and then collide.
  static String _npcRelationshipPendingKey(String id) =>
      'npc.relationship:${foldEditTargetPart(id)}';

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

class DeletedSaveRecovery {
  const DeletedSaveRecovery({
    required this.targetPath,
    required this.backupPath,
    required this.persistentPostDeleteSha1,
    required this.deletedSaveSha1,
    required this.deletedPersistentSha1,
    required this.message,
  });

  final String targetPath;
  final String backupPath;
  final String persistentPostDeleteSha1;
  final String deletedSaveSha1;
  final String deletedPersistentSha1;
  final String message;

  String get fileName {
    final normalized = targetPath.replaceAll('\\', '/');
    return normalized.split('/').last;
  }
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

/// What a structured edit rewrites as a whole, for the operations that resolve
/// their target from a key and then replace whatever they find there. Two edits
/// naming the same one cannot share a write.
///
/// Mirrors `structured_edit_target` in crates/gore-save/src/lib.rs, down to how
/// it folds each part of the key and how it reads an asset name out of a class
/// reference. The pending registry keys these operations by the same folded
/// target, so a pair should not get this far; if one does, two writes lose less
/// than a refusal that takes every unrelated edit in the batch down with it.
/// Whether [left] and [right] are two structured operations rewriting one
/// target — the pair the core refuses, so the packer splits it.
@visibleForTesting
bool structuredEditsShareATarget(
  Map<String, Object?> left,
  Map<String, Object?> right,
) {
  final target = _structuredEditTarget(left);
  return target != null && _structuredEditTarget(right) == target;
}

String? _structuredEditTarget(Map<String, Object?> edit) {
  final op = edit['path'];
  if (op is! String) return null;
  final value = edit['value'];
  final fields = value is Map ? value : const <Object?, Object?>{};
  String key(List<String> parts) => [op, ...parts].join('');
  switch (op) {
    case 'private.npc.setRelationship':
      return key([foldEditTargetPart(fields['id'])]);
    case 'private.skills.set':
      // Read through the same helper the other rules use, so an actor left
      // empty resolves to the hero everywhere or nowhere. It reads the raw
      // text, before any folding, so a blank of spaces stays an actor.
      return key([
        foldEditTargetPart(_skillEditActor(edit)),
        foldEditTargetPart(fields['base']),
      ]);
    case 'private.knowledge.setEntry':
      return key([
        foldEditTargetPart(fields['character']),
        foldEditTargetPart(fields['entry']),
      ]);
    case 'private.glossary.setSegment':
      return key([
        _foldAssetName(fields['documentClass']),
        _foldAssetName(fields['segmentClass']),
      ]);
    default:
      return null;
  }
}

/// One part of a target key, folded the way the core folds it — and the way the
/// pending registry has to key these operations so two entries cannot collapse
/// into one target only once the core sees them.
///
/// The core folds ASCII case only (`to_ascii_lowercase`), so this does too:
/// Dart's own `toLowerCase` also folds Ä to ä, which would put two targets the
/// core keeps apart under one key and let the second edit quietly replace the
/// first.
String foldEditTargetPart(Object? part) {
  if (part is! String) return '';
  final trimmed = part.trim();
  final folded = StringBuffer();
  for (final unit in trimmed.codeUnits) {
    const a = 0x41, z = 0x5a, toLower = 0x20;
    folded.writeCharCode(unit >= a && unit <= z ? unit + toLower : unit);
  }
  return folded.toString();
}

/// The asset name of a `/Package.Asset` class reference, folded. The core keys a
/// glossary segment by the asset names and refuses a pair whose packages differ.
String _foldAssetName(Object? part) {
  final text = part is String ? part : '';
  final dot = text.lastIndexOf('.');
  return foldEditTargetPart(dot < 0 ? text : text.substring(dot + 1));
}

/// Whether [path] addresses the `CurrentState` of a quest entry — the leaf a
/// glossary segment operation rewrites beside the hero's memory.
///
/// Mirrors `path_is_a_quest_current_state` in crates/gore-save/src/lib.rs,
/// including why it claims the shape rather than one entry: the operation can be
/// sent without a quest path at all and the core then derives the leaf from the
/// save, which nothing here can do.
bool _pathIsAQuestCurrentState(List<Object?> path) {
  if (path.length < 2) return false;
  if (path.last != 'CurrentState') return false;
  if (_mapKeySegment(path[path.length - 2]) == null) return false;
  return _pathHasName(path, 'QuestDataByClass');
}

/// Whether two raw segment lists address the same path in the sense the core
/// gives them: `parse_path` reads `{k}` as a map key and `[n]` as an index, so
/// `[03]` and `[3]` are one and the same segment to it even though the two
/// strings differ. Used where a mirror has to agree with the core exactly;
/// [_sameEditorPath] compares the segments as the editor wrote them.
bool _sameCorePath(List<Object?> left, List<Object?> right) {
  if (left.length != right.length) return false;
  for (var i = 0; i < left.length; i++) {
    if (!_sameCoreSegment(left[i], right[i])) return false;
  }
  return true;
}

bool _sameCoreSegment(Object? left, Object? right) {
  if (left == right) return true;
  final index = _indexSegment(left);
  return index != null && index == _indexSegment(right);
}

/// The number of an `[n]` index segment, or null when [segment] is not one.
int? _indexSegment(Object? segment) {
  if (segment is! String) return null;
  if (segment.length < 3 ||
      !segment.startsWith('[') ||
      !segment.endsWith(']')) {
    return null;
  }
  return int.tryParse(segment.substring(1, segment.length - 1));
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
  // The core takes the actor when it is a non-empty string and the hero
  // otherwise, so an empty one is the hero here too — a rule this shares with
  // the target key, and the two must not disagree.
  final actor = value['actor'];
  return actor is String && actor.isNotEmpty ? actor : 'Hero';
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

/// The path a raw `private.typed.*` edit addresses, or `null` when [edit] is not
/// one. Mirrors `raw_typed_path` in crates/gore-save/src/lib.rs.
List<Object?>? _rawTypedEditPath(Map<String, Object?> edit) {
  if (!_typedEditPaths.contains(edit['path'])) return null;
  final value = edit['value'];
  if (value is! Map) return null;
  final path = value['path'];
  return path is List ? path : null;
}

/// The key of a `{likeThis}` map-key segment, or `null` when [segment] is not
/// one. The generic All-data browser writes map keys wrapped in braces, and the
/// core's `parse_path` turns exactly those into `PathSeg::MapKey`.
String? _mapKeySegment(Object? segment) {
  if (segment is! String) return null;
  if (segment.length < 2 ||
      !segment.startsWith('{') ||
      !segment.endsWith('}')) {
    return null;
  }
  return segment.substring(1, segment.length - 1);
}

/// Whether [segment] is an `[i]` element segment.
bool _isIndexSegment(Object? segment) =>
    segment is String &&
    segment.length >= 2 &&
    segment.startsWith('[') &&
    segment.endsWith(']');

/// Map keys compare trimmed and case-insensitively, as the core does.
bool _sameMapKey(String left, String right) =>
    left.trim().toLowerCase() == right.trim().toLowerCase();

/// Mirrors `path_has_name` in crates/gore-save/src/lib.rs.
bool _pathHasName(List<Object?> path, String name) =>
    path.any((segment) => segment == name);

/// Mirrors `path_has_key` in crates/gore-save/src/lib.rs.
bool _pathHasKey(List<Object?> path, String key) => path.any((segment) {
  final found = _mapKeySegment(segment);
  return found != null && _sameMapKey(found, key);
});

/// Whether [path] descends into [map]'s entry for [key] — or into the map as a
/// whole, which collides with every entry in it.
///
/// Mirrors `path_enters_map_entry` in crates/gore-save/src/lib.rs, including its
/// last clause: a segment after the map that is not a `{key}` addresses it some
/// other way, which is too unclear to call safe.
bool _pathEntersMapEntry(List<Object?> path, String map, String key) {
  final at = path.indexWhere((segment) => segment == map);
  if (at < 0) return false;
  if (at + 1 >= path.length) return true;
  final found = _mapKeySegment(path[at + 1]);
  if (found == null) return true;
  return _sameMapKey(found, key);
}

/// Whether [path] reaches a slot of an inventory container — the array itself,
/// or an element of it.
///
/// Mirrors `path_reaches_inventory_slot` in crates/gore-save/src/lib.rs.
bool _pathReachesInventorySlot(List<Object?> path) {
  final at = path.indexWhere((segment) => segment == 'm_Slots');
  if (at < 0) return false;
  if (at + 1 >= path.length) return true;
  return _isIndexSegment(path[at + 1]);
}

/// Whether [path] writes a slot's `m_Id` — the field a slot is selected by, so a
/// write to it renumbers slots exactly as a repair does.
///
/// Mirrors `path_writes_a_slot_id` in crates/gore-save/src/lib.rs.
bool _pathWritesASlotId(List<Object?> path) {
  if (path.isEmpty || path.last != 'm_Id') return false;
  return _pathReachesInventorySlot(path.sublist(0, path.length - 1));
}

/// Whether a slot-reaching [path] belongs to the inventory [actorId] names. An
/// NPC's inventory hangs under that character's own map entry, so its key
/// settles it; the controlled player's hangs off `m_SavedPlayers`. A path that
/// fits neither description is treated as a conflict rather than waved through.
///
/// Mirrors `slot_edit_targets_actor` in crates/gore-save/src/lib.rs.
bool _slotEditTargetsActor(List<Object?> path, String? actorId) {
  if (!_pathReachesInventorySlot(path)) return false;
  if (actorId != null) return _pathHasKey(path, actorId);
  return _pathHasName(path, 'm_SavedPlayers') ||
      !path.any((segment) => _mapKeySegment(segment) != null);
}

/// The actor a structured inventory edit targets, or `null` for the controlled
/// player — which is also what a blank `actorId` means to the core.
String? _inventoryEditActorId(Map<String, Object?> edit) {
  final value = edit['value'];
  final actorId = value is Map ? value['actorId'] : null;
  return actorId is String && actorId.trim().isNotEmpty ? actorId : null;
}

/// Whether the raw typed edit at [typedPath] addresses something the structured
/// [structured] edit rewrites AS A WHOLE.
///
/// Mirrors `structured_edit_rewrites` in crates/gore-save/src/lib.rs. Ordering
/// cannot rescue such a pair — a structured operation resolves its own target
/// and recreates it, so whichever runs second discards the other's work — which
/// is why the core refuses the two in ONE write whichever way round they come.
/// The packer therefore has to put them in SEPARATE sub-writes; sequential
/// writes are safe, because the core re-reads the file and re-resolves every
/// symbolic target per write.
@visibleForTesting
bool structuredEditRewrites(
  Map<String, Object?> structured,
  List<Object?> typedPath,
) {
  final op = structured['path'];
  if (op is! String) return false;
  final value = structured['value'];
  final fields = value is Map ? value : const <Object?, Object?>{};
  switch (op) {
    // Patches or appends a modifier under this NPC's relationship entry.
    case 'private.npc.setRelationship':
      final id = fields['id'];
      return id is String &&
          id.trim().isNotEmpty &&
          _pathEntersMapEntry(typedPath, 'RelationshipByGlobalId', id);
    // Learning or unlearning rewrites this actor's effect elements. Broader
    // than the same-actor `EffectSpec/Def` refusal above: ANY leaf of that
    // actor's ActiveEffects is rewritten wholesale.
    case 'private.skills.set':
      final actor = _skillEditActor(structured);
      return actor != null &&
          _pathHasName(typedPath, 'ActiveEffects') &&
          _pathHasKey(typedPath, actor);
    // Adds or removes an unlock event in the hero's memory, and rewrites the
    // CurrentState of the quest leaf that stands for the same segment.
    case 'private.glossary.setSegment':
      return (_pathHasName(typedPath, 'MemorizedEvents') &&
              _pathHasKey(typedPath, 'Hero')) ||
          _pathIsAQuestCurrentState(typedPath);
    // Strips memory events, death tags and the corpse entry.
    case 'private.npc.revive':
      return _pathHasName(typedPath, 'MemorizedEvents') ||
          _pathHasName(typedPath, 'LooseTagsByGlobalId') ||
          _pathHasName(typedPath, 'm_SavedInventories');
    // Insert or update ONE character's knowledge entry. Another character's
    // entry is a different map value, and every applier re-resolves its target
    // by key, so the two do not collide.
    case 'private.knowledge.addCharacter':
      final name = fields['value'];
      return name is String &&
          _pathEntersMapEntry(
            typedPath,
            'CharacterKnowledgeByUniqueName',
            name,
          );
    case 'private.knowledge.setEntry':
      final character = fields['character'];
      return character is String &&
          _pathEntersMapEntry(
            typedPath,
            'CharacterKnowledgeByUniqueName',
            character,
          );
    // Claims a whole slot — but only in the inventory it targets; another
    // actor's slots are a different subtree.
    case 'private.inventory.addItem':
    case 'private.inventory.removeItem':
      return _slotEditTargetsActor(
        typedPath,
        _inventoryEditActorId(structured),
      );
    // Narrower still: it only rewrites ids, but it does so across every
    // container in the save, so it is not scoped to one actor.
    case 'private.inventory.repairSlots':
      return _pathWritesASlotId(typedPath);
    // A trader edit is addressed by its row's position in m_Traders, and a raw
    // array operation ON that array renumbers the rows. Splitting the two into
    // separate writes does not rescue them: the index came from a list read
    // BEFORE either ran, so whichever goes second resolves it against a layout
    // the first moved. The pair is refused whichever way round it comes.
    case 'private.traders.setStock':
    case 'private.traders.addItem':
    case 'private.traders.removeItem':
      return _pathTargetsTheTraderArray(typedPath);
    default:
      return false;
  }
}

/// The first pair of pending edits where a trade change meets a raw array
/// operation on the trader array, or null when there is none.
///
/// Separate from the packer's boundary test: this pair is not made safe by a
/// split, so it has to abort the save rather than start a new sub-write.
@visibleForTesting
(Map<String, Object?>, Map<String, Object?>)? traderArrayConflict(
  List<Map<String, Object?>> edits,
) {
  const traderOps = {
    'private.traders.setStock',
    'private.traders.addItem',
    'private.traders.removeItem',
  };
  const arrayOps = {
    'private.typed.arrayRemove',
    'private.typed.arrayDuplicate',
  };
  for (final edit in edits) {
    if (!traderOps.contains(edit['path'])) continue;
    for (final other in edits) {
      // Only an array operation ON the array renumbers its rows. An edit that
      // merely runs THROUGH it — a value under one row, or a container inside
      // one — moves nothing, and refusing those would block safe pairs.
      if (!arrayOps.contains(other['path'])) continue;
      final path = _rawTypedEditPath(other);
      if (path != null && _pathTargetsTheTraderArray(path)) {
        return (edit, other);
      }
    }
  }
  return null;
}

/// Whether a raw typed path addresses the trader ARRAY itself rather than
/// something inside one of its rows.
bool _pathTargetsTheTraderArray(List<Object?> path) =>
    path.isNotEmpty && path.last == 'm_Traders';

/// Whether [left] and [right] address the same target in the sense above, in
/// EITHER direction — the pair test the packer uses. The core's rule is
/// order-independent, so a batch may hold neither ordering of such a pair.
@visibleForTesting
bool editsRewriteSameTarget(
  Map<String, Object?> left,
  Map<String, Object?> right,
) {
  final leftPath = _rawTypedEditPath(left);
  if (leftPath != null && structuredEditRewrites(right, leftPath)) return true;
  final rightPath = _rawTypedEditPath(right);
  return rightPath != null && structuredEditRewrites(left, rightPath);
}

/// Edits that must be the only edit in their `write_save`, whatever else is
/// pending. Both are refused as peers by the core: a story batch takes its
/// compare-and-set snapshot from the payload as it enters and proves its own
/// postconditions before committing, and a reset replaces the whole inventory.
const _exclusiveEditPaths = {storyStateApplyPath, 'private.inventory.reset'};

/// Whether [edit] can change how many elements a container holds, or renumber
/// inventory slot ids — the only two things that invalidate an index or slot id a
/// later edit in the same write was addressed with.
///
/// Mirrors `may_invalidate_caller_ordinals` in crates/gore-save/src/lib.rs. Keep the
/// two in step: the core rejects the write outright when they disagree.
bool _mayInvalidateOrdinals(Map<String, Object?> edit) {
  final path = edit['path'];
  if (path is! String) return true;
  if (_typedEditPaths.contains(path) && path != 'private.typed.setValue') {
    // setAdd/setRemove change a set's cardinality, arrayRemove/arrayDuplicate an
    // array's length.
    return true;
  }
  if (path == 'private.typed.setValue') {
    // It can add or drop no container element — but writing a slot's m_Id
    // renumbers slots, which is the other half of what invalidates an ordinal a
    // later edit was addressed with.
    final typedPath = _rawTypedEditPath(edit);
    return typedPath != null && _pathWritesASlotId(typedPath);
  }
  return const {
    'private.inventory.addItem',
    'private.inventory.removeItem',
    'private.inventory.reset',
    'private.inventory.repairSlots',
    'private.knowledge.addCharacter',
    'private.knowledge.setEntry',
    'private.npc.revive',
    'private.npc.setRelationship',
    'private.glossary.setSegment',
    'private.skills.set',
    // Splice an entry into or out of a trader's stock map, which changes how
    // many entries it holds. private.traders.setStock is absent: it overwrites a
    // bare i32 in place.
    'private.traders.addItem',
    'private.traders.removeItem',
    storyStateApplyPath,
  }.contains(path);
}

/// Whether [edit] addresses its target with an index or slot id the user's view of
/// the save supplied — something an earlier length change would silently retarget.
///
/// Mirrors `carries_caller_ordinal` in crates/gore-save/src/lib.rs.
bool _carriesCallerOrdinal(Map<String, Object?> edit) {
  final path = edit['path'];
  if (path is! String) return false;
  final value = edit['value'];
  if (path == 'private.typed.arrayRemove' ||
      path == 'private.typed.arrayDuplicate') {
    return true;
  }
  if (_typedEditPaths.contains(path)) {
    final raw = value is Map ? value['path'] : null;
    return raw is List &&
        raw.whereType<String>().any(
          (segment) => segment.startsWith('[') && segment.endsWith(']'),
        );
  }
  if (path == 'private.inventory.setItemCount') {
    // A slot id is an index by invariant. The player path carries an ordinal even
    // without one: it finds the stack through a positional scan of the payload
    // rather than through the typed tree.
    return (value is Map ? value['slotId'] : null) != null ||
        (value is Map ? value['actorId'] : null) == null;
  }
  if (path == 'private.inventory.removeItem') {
    return (value is Map ? value['slotId'] : null) != null;
  }
  if (path == 'private.traders.setStock' ||
      path == 'private.traders.addItem' ||
      path == 'private.traders.removeItem') {
    // Every trader edit addresses its row by an index into the trader array that
    // the user's view supplied.
    return true;
  }
  return false;
}

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
  // Exactly the reach the core's `path_reaches_inventory_slot` describes, so
  // the two cannot drift apart. This predicate is broader than the core's rule
  // in one respect on purpose: it is not scoped to the add's/removal's actor,
  // so a queued slot edit for ANY actor is refused rather than merely split.
  final path = _rawTypedEditPath(edit);
  return path != null && _pathReachesInventorySlot(path);
}

/// A raw typed edit that writes a slot's `m_Id` — the one field the whole-save
/// repair rewrites, and therefore the only one it can collide with. Anything
/// else inside a slot survives the repair untouched.
///
/// Deliberately narrower than [structuredEditRewrites]'s repair arm, which
/// matches any `m_Id` leaf below a slot: the shapes only that one catches are
/// SPLIT across sub-writes by the packer, not refused with an error.
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
