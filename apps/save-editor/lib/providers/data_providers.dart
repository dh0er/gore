import 'dart:io';

import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_riverpod/legacy.dart';
import 'package:goresave/features/app/domain/ui_settings.dart';
import 'package:goresave/features/app/ui/router.dart';
import 'package:goresave/features/editor/domain/core_service.dart';
import 'package:goresave/features/editor/domain/editor_notifier.dart';
import 'package:goresave/features/editor/domain/editor_settings_store.dart';
import 'package:goresave/l10n/app_localizations.dart';
import 'package:goresave/loc/game_lang.dart';
import 'package:goresave/utils/shared_config.dart';
import 'package:path/path.dart' as p;

final coreServiceProvider = Provider<GoresaveCoreService>((ref) {
  return NativeGoresaveCoreService.tryCreate() ?? MissingGoresaveCoreService();
});

final editorSettingsStoreProvider = Provider<EditorSettingsStore>((ref) {
  return JsonFileEditorSettingsStore.defaultForPlatform();
});

/// Current generated app strings for domain-layer messages. Callers that must
/// survive locale changes should pass a closure that reads this provider at
/// message time instead of watching it and rebuilding long-lived controllers.
final appLocalizationsProvider = Provider<AppLocalizations>((ref) {
  final locale = gameLangByCode(ref.watch(localeProvider)).locale;
  return lookupAppLocalizations(locale);
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
    localizations: () => ref.read(appLocalizationsProvider),
  );
});

final routerProvider = Provider<GoresaveRouter>((ref) => GoresaveRouter());
