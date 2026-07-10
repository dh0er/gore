import 'dart:io';

import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_riverpod/legacy.dart';
import 'package:goresave/features/app/ui/router.dart';
import 'package:goresave/features/editor/domain/core_service.dart';
import 'package:goresave/features/editor/domain/editor_notifier.dart';
import 'package:goresave/features/editor/domain/editor_settings_store.dart';
import 'package:goresave/utils/shared_config.dart';
import 'package:path/path.dart' as p;

final coreServiceProvider = Provider<GoresaveCoreService>((ref) {
  return NativeGoresaveCoreService.tryCreate() ?? MissingGoresaveCoreService();
});

final editorSettingsStoreProvider = Provider<EditorSettingsStore>((ref) {
  return JsonFileEditorSettingsStore.defaultForPlatform();
});

/// The shared cross-tool `config.json` (currently just `game_path`). Widget
/// tests get an isolated, almost-certainly-absent file under the temp dir
/// instead of the real per-user config, matching [uiSettingsStoreProvider]'s
/// FLUTTER_TEST guard.
final sharedConfigProvider = Provider<SharedConfig>((ref) {
  if (Platform.environment.containsKey('FLUTTER_TEST')) {
    // Unique temp file per container so tests never leak persisted game-path
    // state into one another via a shared fixed path.
    final dir = Directory.systemTemp.createTempSync('gore_test_cfg');
    return SharedConfig(File(p.join(dir.path, 'config.json')));
  }
  return SharedConfig.defaultForPlatform();
});

final editorProvider = StateNotifierProvider<EditorNotifier, EditorState>((
  ref,
) {
  return EditorNotifier(
    ref.watch(coreServiceProvider),
    settingsStore: ref.watch(editorSettingsStoreProvider),
  );
});

final routerProvider = Provider<GoresaveRouter>((ref) => GoresaveRouter());
