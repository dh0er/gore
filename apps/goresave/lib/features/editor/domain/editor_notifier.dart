import 'package:file_selector/file_selector.dart';
import 'package:goresave/features/editor/domain/core_service.dart';
import 'package:goresave/features/editor/domain/editor_models.dart';
import 'package:goresave/features/editor/domain/editor_settings_store.dart';
import 'package:goresave/utils/default_paths.dart';
import 'package:path/path.dart' as p;
import 'package:state_notifier/state_notifier.dart';

const _unchanged = Object();

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
    this.error,
    this.lastWriteMessage,
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
  final String? error;
  final String? lastWriteMessage;

  SaveSlot? get selectedSave {
    for (final save in saves) {
      if (save.path == selectedPath) return save;
    }
    return null;
  }

  ProfileSummary? get activeProfile {
    if (profiles.isEmpty) return null;
    for (final profile in profiles) {
      if (profile.profileId == activeProfileId) return profile;
    }
    return profiles.first;
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
    String? error,
    String? lastWriteMessage,
    bool clearInspection = false,
    bool clearBackups = false,
    bool clearError = false,
    bool clearWriteMessage = false,
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
      codecStatus: codecStatus ?? this.codecStatus,
      error: clearError ? null : error ?? this.error,
      lastWriteMessage: clearWriteMessage
          ? null
          : lastWriteMessage ?? this.lastWriteMessage,
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

  bool get coreAvailable => _core.isAvailable;
  String get coreDescription => _core.description;

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
    state = state.copyWith(isLoading: true, clearError: true);
    final response = await _core.execute(
      'scan_save_dir',
      payload: {'path': state.saveDir, ..._codecPayload()},
    );
    if (seq != _loadSeq) return;
    if (response['ok'] != true) {
      state = state.copyWith(isLoading: false, error: _errorMessage(response));
      return;
    }
    final data = (response['data'] as Map?)?.cast<String, Object?>();
    final rawSaves = (data?['saves'] as List?) ?? const [];
    final saves = rawSaves
        .whereType<Map>()
        .map((m) => SaveSlot.fromJson(m.cast<String, Object?>()))
        .toList();
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
      // Keep loading while we hand off to _inspect; it owns the terminal state.
      isLoading: selectedPath != null,
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
  }

  Future<void> inspect(String path) async {
    await _inspect(path, clearWriteMessage: true);
  }

  Future<void> _inspect(String path, {bool clearWriteMessage = false}) async {
    final seq = ++_loadSeq;
    state = state.copyWith(
      selectedPath: path,
      isLoading: true,
      clearError: true,
      clearWriteMessage: clearWriteMessage,
    );
    final payload = <String, Object?>{
      'path': path,
      'includePrivate': true,
      ..._codecPayload(),
    };
    final response = await _core.execute('inspect_save', payload: payload);
    if (seq != _loadSeq) return;
    if (response['ok'] != true) {
      state = state.copyWith(
        isLoading: false,
        error: _errorMessage(response),
        clearInspection: true,
        clearBackups: true,
      );
      return;
    }
    final data = (response['data'] as Map).cast<String, Object?>();
    final backupSnapshot = await _loadBackups(path, seq);
    if (backupSnapshot == null) return;
    state = state.copyWith(
      isLoading: false,
      inspection: SaveInspection.fromJson(data),
      backups: backupSnapshot.backups,
      companionBackups: backupSnapshot.companionBackups,
    );
  }

  Future<void> refreshBackups() async {
    final path = state.selectedPath;
    if (path == null) return;
    final seq = ++_loadSeq;
    state = state.copyWith(isLoading: true, clearError: true);
    final backupSnapshot = await _loadBackups(path, seq);
    if (backupSnapshot == null) return;
    state = state.copyWith(
      isLoading: false,
      backups: backupSnapshot.backups,
      companionBackups: backupSnapshot.companionBackups,
    );
  }

  Future<void> restoreBackup(String backupPath) async {
    final path = state.selectedPath;
    if (path == null) return;
    state = state.copyWith(isLoading: true, clearError: true);
    final response = await _core.execute(
      'restore_backup',
      payload: {'path': path, 'backupPath': backupPath},
    );
    if (response['ok'] != true) {
      state = state.copyWith(isLoading: false, error: _errorMessage(response));
      return;
    }
    state = state.copyWith(lastWriteMessage: 'Restored backup: $backupPath');
    await _inspect(path);
  }

  Future<void> checkCodec() async {
    final response = await _core.execute(
      'check_codec',
      payload: _codecPayload(),
    );
    if (response['ok'] != true) {
      state = state.copyWith(error: _errorMessage(response));
      return;
    }
    final data = (response['data'] as Map).cast<String, Object?>();
    final status = CodecStatus.fromJson(data);
    state = state.copyWith(codecStatus: status);
    if (status.available && state.selectedPath != null) {
      await inspect(state.selectedPath!);
    }
  }

  Future<void> validateSelected() async {
    final path = state.selectedPath;
    if (path == null) return;
    state = state.copyWith(isLoading: true, clearError: true);
    final response = await _core.execute(
      'validate_roundtrip',
      payload: {'path': path},
    );
    if (response['ok'] != true) {
      state = state.copyWith(isLoading: false, error: _errorMessage(response));
      return;
    }
    final data = (response['data'] as Map).cast<String, Object?>();
    state = state.copyWith(
      isLoading: false,
      lastWriteMessage: data['identical'] == true
          ? 'Roundtrip validation passed'
          : 'Roundtrip validation changed bytes',
    );
  }

  Future<void> validateCodecRoundtrip() async {
    final path = state.selectedPath;
    if (path == null) return;
    state = state.copyWith(isLoading: true, clearError: true);
    final response = await _core.execute(
      'validate_codec_roundtrip',
      payload: {'path': path, ..._codecPayload()},
    );
    if (response['ok'] != true) {
      state = state.copyWith(isLoading: false, error: _errorMessage(response));
      return;
    }
    final data = (response['data'] as Map).cast<String, Object?>();
    state = state.copyWith(
      isLoading: false,
      lastWriteMessage:
          'Codec roundtrip passed: chunk ${data['chunkIndex']} recompressed to ${data['recompressedSize']} bytes',
    );
  }

  Future<void> writePlayerSaveName(String value) async {
    final path = state.selectedPath;
    if (path == null) return;
    state = state.copyWith(isLoading: true, clearError: true);
    final response = await _core.execute(
      'write_save',
      payload: {
        'path': path,
        'backup': true,
        'syncPersistentDataList': true,
        'edits': [
          {'path': 'public.m_PlayerSaveName', 'value': value},
        ],
      },
    );
    if (response['ok'] != true) {
      state = state.copyWith(isLoading: false, error: _errorMessage(response));
      return;
    }
    final data = (response['data'] as Map).cast<String, Object?>();
    state = state.copyWith(
      isLoading: false,
      lastWriteMessage: _backupMessage('Saved with backup', data),
    );
    await refresh();
  }

  Future<void> writePrivateFString({
    required String oldValue,
    required String newValue,
  }) async {
    final path = state.selectedPath;
    if (path == null) return;
    state = state.copyWith(isLoading: true, clearError: true);
    final response = await _core.execute(
      'write_save',
      payload: {
        'path': path,
        'backup': true,
        'edits': [
          {
            'path': 'private.replaceFString',
            'value': {'oldValue': oldValue, 'newValue': newValue},
          },
        ],
        ..._codecPayload(),
      },
    );
    if (response['ok'] != true) {
      state = state.copyWith(isLoading: false, error: _errorMessage(response));
      return;
    }
    final data = (response['data'] as Map).cast<String, Object?>();
    state = state.copyWith(
      isLoading: false,
      lastWriteMessage: _backupMessage(
        'Private payload saved with backup',
        data,
      ),
    );
    await refresh();
  }

  Future<void> writePrivatePlayerName(String value) async {
    final path = state.selectedPath;
    if (path == null) return;
    state = state.copyWith(isLoading: true, clearError: true);
    final response = await _core.execute(
      'write_save',
      payload: {
        'path': path,
        'backup': true,
        'edits': [
          {'path': 'private.player.setPlayerName', 'value': value},
        ],
        ..._codecPayload(),
      },
    );
    if (response['ok'] != true) {
      state = state.copyWith(isLoading: false, error: _errorMessage(response));
      return;
    }
    final data = (response['data'] as Map).cast<String, Object?>();
    state = state.copyWith(
      isLoading: false,
      lastWriteMessage: _backupMessage(
        'Private player name saved with backup',
        data,
      ),
    );
    await refresh();
  }

  Future<void> writePrivateProfileName(String value) async {
    final path = state.selectedPath;
    if (path == null) return;
    state = state.copyWith(isLoading: true, clearError: true);
    final response = await _core.execute(
      'write_save',
      payload: {
        'path': path,
        'backup': true,
        'edits': [
          {'path': 'private.profile.setProfileName', 'value': value},
        ],
        ..._codecPayload(),
      },
    );
    if (response['ok'] != true) {
      state = state.copyWith(isLoading: false, error: _errorMessage(response));
      return;
    }
    final data = (response['data'] as Map).cast<String, Object?>();
    state = state.copyWith(
      isLoading: false,
      lastWriteMessage: _backupMessage(
        'Private profile name saved with backup',
        data,
      ),
    );
    await refresh();
  }

  Future<void> writePlayerAttribute({
    required String id,
    required double baseValue,
    required double currentValue,
  }) async {
    final path = state.selectedPath;
    if (path == null) return;
    state = state.copyWith(isLoading: true, clearError: true);
    final response = await _core.execute(
      'write_save',
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
    );
    if (response['ok'] != true) {
      state = state.copyWith(isLoading: false, error: _errorMessage(response));
      return;
    }
    final data = (response['data'] as Map).cast<String, Object?>();
    state = state.copyWith(
      isLoading: false,
      lastWriteMessage: _backupMessage(
        'Private player attribute saved with backup',
        data,
      ),
    );
    await refresh();
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
    state = state.copyWith(isLoading: true, clearError: true);
    final response = await _core.execute(
      'write_save',
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
    );
    if (response['ok'] != true) {
      state = state.copyWith(isLoading: false, error: _errorMessage(response));
      return;
    }
    final data = (response['data'] as Map).cast<String, Object?>();
    state = state.copyWith(
      isLoading: false,
      lastWriteMessage: _backupMessage(
        'Private player transform saved with backup',
        data,
      ),
    );
    await refresh();
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
    state = state.copyWith(isLoading: true, clearError: true);
    final response = await _core.execute(
      'write_save',
      payload: {
        'path': savePath,
        'backup': true,
        'edits': changes.map((change) => change.toEditJson()).toList(),
        ..._codecPayload(),
      },
    );
    if (response['ok'] != true) {
      state = state.copyWith(isLoading: false, error: _errorMessage(response));
      return;
    }
    final data = (response['data'] as Map).cast<String, Object?>();
    state = state.copyWith(
      isLoading: false,
      lastWriteMessage: changes.length == 1
          ? _backupMessage('Inventory count saved with backup', data)
          : _backupMessage(
              '${changes.length} inventory counts saved with backup',
              data,
            ),
    );
    await refresh();
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
    final response = await _core.execute(
      'list_backups',
      payload: {'path': path},
    );
    // A newer load superseded this one; let it own loading/error state instead
    // of clobbering it (which could also leave the overlay spinning).
    if (seq != _loadSeq) return null;
    if (response['ok'] != true) {
      state = state.copyWith(isLoading: false, error: _errorMessage(response));
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
