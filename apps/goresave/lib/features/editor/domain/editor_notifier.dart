import 'dart:io';

import 'package:file_selector/file_selector.dart';
import 'package:goresave/features/editor/domain/core_service.dart';
import 'package:goresave/features/editor/domain/editor_models.dart';
import 'package:goresave/features/editor/domain/editor_settings_store.dart';
import 'package:goresave/features/editor/domain/hero_attributes.dart';
import 'package:goresave/features/editor/domain/pending_edits.dart';
import 'package:goresave/utils/default_paths.dart';
import 'package:path/path.dart' as p;
import 'package:state_notifier/state_notifier.dart';

const _unchanged = Object();

/// Sorts saves by file last-modified time, newest first. Saves whose file
/// can't be stat'd (e.g. fake paths in tests) sink to the bottom rather than
/// throwing, so a transient FS error never breaks the scan.
void _sortByLastModifiedDesc(List<SaveSlot> saves) {
  final mtime = <String, DateTime>{};
  for (final save in saves) {
    try {
      mtime[save.path] = File(save.path).lastModifiedSync();
    } catch (_) {
      // Leave unset; treated as oldest below.
    }
  }
  saves.sort((a, b) {
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
  });

  final String saveDir;
  final String codecHostPath;
  final String gameExePath;
  final bool isLoading;
  final List<SaveSlot> saves;
  final List<ProfileSummary> profiles;
  final int? activeProfileId;
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

  SaveSlot? get selectedSave {
    for (final save in saves) {
      if (save.path == selectedPath) return save;
    }
    return null;
  }

  ProfileSummary? get activeProfile {
    // Prefer the selected save's own profile so a mixed-profile folder shows the
    // profile that the selected slot belongs to, falling back to the scan's
    // active profile id.
    final targetProfileId = selectedSave?.persistentProfileId ?? activeProfileId;
    for (final profile in profiles) {
      if (profile.profileId == targetProfileId) return profile;
    }
    // No profile matches: report none rather than guessing `profiles.first`,
    // which would show another profile's name and counts.
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

  /// Run a `write_save` request as a tracked load, then rescan on success.
  /// Returns true only when the core accepted the write; a rejected write sets
  /// `state.error` and returns false so callers can skip success-only follow-ups.
  Future<bool> _runWrite({
    required Map<String, Object?> payload,
    required String Function(Map<String, Object?> data) message,
  }) async {
    var ok = false;
    await _withLoading(() async {
      final response = await _execute('write_save', payload: payload);
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

  /// Serializes all core calls. The native layer runs each command in its own
  /// isolate with no serialization, so overlapping write_save/restore_backup
  /// requests on the same file could interleave temp files and renames. Chaining
  /// through this queue guarantees one core command finishes before the next
  /// starts.
  Future<void> _coreQueue = Future<void>.value();

  bool get coreAvailable => _core.isAvailable;
  String get coreDescription => _core.description;

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

  /// Clear all pending edits.
  void clearAllPendingEdits() {
    if (state.pendingEdits.isEmpty) return;
    state = state.copyWith(clearPendingEdits: true);
  }

  /// Total number of edit objects across all pending keys.
  int get pendingEditCount {
    return state.pendingEdits.values.fold(0, (sum, e) => sum + e.edits.length);
  }

  /// Save all pending edits in one write_save call. No-op when empty.
  /// Re-entry-safe: bails immediately if a load is already in flight.
  /// Returns true on success (or when nothing to save), false on failure.
  Future<bool> saveAllPending() async {
    if (state.pendingEdits.isEmpty) return true;
    if (state.isLoading) return false;
    final savePath = state.selectedPath;
    if (savePath == null) return false;

    // Build edits in stable (sorted) key order for determinism.
    final sortedKeys = state.pendingEdits.keys.toList()..sort();
    final allEdits = <Map<String, Object?>>[];
    var syncPersistent = false;
    for (final key in sortedKeys) {
      final entry = state.pendingEdits[key]!;
      allEdits.addAll(entry.edits);
      if (entry.syncPersistentDataList) syncPersistent = true;
    }

    final n = allEdits.length;
    final ok = await _runWrite(
      payload: {
        'path': savePath,
        'backup': true,
        if (syncPersistent) 'syncPersistentDataList': true,
        'edits': allEdits,
        ..._codecPayload(),
      },
      message: (data) => _backupMessage(
        '$n change${n == 1 ? '' : 's'} saved with backup',
        data,
      ),
    );
    if (ok) {
      // Clear pending edits on success (the refresh from _runWrite already
      // populated fresh inspection data).
      clearAllPendingEdits();
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
      _sortByLastModifiedDesc(saves);
      final rawProfiles = (data?['profiles'] as List?) ?? const [];
      final profiles = rawProfiles
          .whereType<Map>()
          .map((m) => ProfileSummary.fromJson(m.cast<String, Object?>()))
          .toList();
      final activeProfileId = (data?['activeProfileId'] as num?)?.toInt();
      final selectedPath = saves.any((save) => save.path == state.selectedPath)
          ? state.selectedPath
          : (saves.isNotEmpty ? saves.first.path : null);
      state = state.copyWith(
        saves: saves,
        profiles: profiles,
        activeProfileId: activeProfileId,
        selectedPath: selectedPath,
        clearInspection: selectedPath == null,
        clearBackups: selectedPath == null,
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
      // Stale pending edits from a prior slot must not be written to a
      // different file. Clear them when switching to a new slot.
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
      state = state.copyWith(inspection: SaveInspection.fromJson(data));
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
      final response = await _execute(
        'check_codec',
        payload: _codecPayload(),
      );
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

  Future<void> validateSelected() async {
    final path = state.selectedPath;
    if (path == null) return;
    await _withLoading(() async {
      final response = await _execute(
        'validate_roundtrip',
        payload: {'path': path},
      );
      if (response['ok'] != true) {
        state = state.copyWith(error: _errorMessage(response));
        return;
      }
      final data = (response['data'] as Map).cast<String, Object?>();
      state = state.copyWith(
        lastWriteMessage: data['identical'] == true
            ? 'Roundtrip validation passed'
            : 'Roundtrip validation changed bytes',
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

  Future<void> writePlayerSaveName(String value) async {
    final path = state.selectedPath;
    if (path == null) return;
    await _runWrite(
      payload: {
        'path': path,
        'backup': true,
        'syncPersistentDataList': true,
        'edits': [
          {'path': 'public.m_PlayerSaveName', 'value': value},
        ],
      },
      message: (data) => _backupMessage('Saved with backup', data),
    );
  }

  Future<void> writePlayerAttribute({
    required String id,
    required double baseValue,
    required double currentValue,
  }) async {
    final path = state.selectedPath;
    if (path == null) return;
    await _runWrite(
      payload: {
        'path': path,
        'backup': true,
        'edits': [
          {
            'path': 'private.player.setAttribute',
            'value': {
              'id': id,
              'baseValue': baseValue,
              'currentValue': currentValue,
            },
          },
        ],
        ..._codecPayload(),
      },
      message: (data) =>
          _backupMessage('Private player attribute saved with backup', data),
    );
  }

  Future<void> writePlayerTransform({
    required double locationX,
    required double locationY,
    required double locationZ,
    required double rotationPitch,
    required double rotationYaw,
    required double rotationRoll,
  }) async {
    final path = state.selectedPath;
    if (path == null) return;
    await _runWrite(
      payload: {
        'path': path,
        'backup': true,
        'edits': [
          {
            'path': 'private.player.setTransform',
            'value': {
              'location': {'x': locationX, 'y': locationY, 'z': locationZ},
              'rotation': {
                'pitch': rotationPitch,
                'yaw': rotationYaw,
                'roll': rotationRoll,
              },
            },
          },
        ],
        ..._codecPayload(),
      },
      message: (data) =>
          _backupMessage('Private player transform saved with backup', data),
    );
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

  /// Layout-verified typed edit: set a fixed-size scalar (int/float/bool/
  /// byte) or a string-valued property (Str/Name/Object/Enum and the
  /// enum-as-byte form of ByteProperty) at a typed property path. String
  /// edits may change the payload length; the core fixes up every enclosing
  /// size field. Only offered by the core when the strict typed parse of the
  /// save succeeded (`private.typed.setValue` in writable).
  ///
  /// Path segments: property name, `{mapKey}` for map entries, `[i]` for
  /// container/object-array indices.
  Future<bool> writeTypedValue({
    required List<String> propertyPath,
    required Object value,
  }) =>
      writeTypedValues([TypedValueEdit(path: propertyPath, value: value)]);

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

  /// Apply several typed edits as one write_save call: one backup, one
  /// re-inspect, all-or-nothing from the user's point of view. An empty list
  /// is a successful no-op: no write is performed and no backup is created.
  Future<bool> writeTypedValues(List<TypedValueEdit> edits) async {
    if (edits.isEmpty) return true;
    final savePath = state.selectedPath;
    if (savePath == null) return false;
    return _runWrite(
      payload: {
        'path': savePath,
        'backup': true,
        'edits': [
          for (final edit in edits)
            {
              'path': 'private.typed.setValue',
              'value': {'path': edit.path, 'value': edit.value},
            },
        ],
        ..._codecPayload(),
      },
      message: (data) => _backupMessage(
        edits.length == 1
            ? 'Typed value saved with backup'
            : '${edits.length} typed values saved with backup',
        data,
      ),
    );
  }

  Future<void> writeInventoryItemCount({
    required String id,
    required String path,
    required int count,
  }) async {
    await writeInventoryItemCounts([
      InventoryItemCountChange(id: id, path: path, count: count),
    ]);
  }

  Future<void> writeInventoryItemCounts(
    List<InventoryItemCountChange> changes,
  ) async {
    if (changes.isEmpty) return;
    final savePath = state.selectedPath;
    if (savePath == null) return;
    await _runWrite(
      payload: {
        'path': savePath,
        'backup': true,
        'edits': changes.map((change) => change.toEditJson()).toList(),
        ..._codecPayload(),
      },
      message: (data) => changes.length == 1
          ? _backupMessage('Inventory count saved with backup', data)
          : _backupMessage(
              '${changes.length} inventory counts saved with backup',
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
    final response = await _execute(
      'list_backups',
      payload: {'path': path},
    );
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
