import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_riverpod/legacy.dart';
import 'package:goresave/features/app/ui/router.dart';
import 'package:goresave/features/editor/domain/core_service.dart';
import 'package:goresave/features/editor/domain/editor_notifier.dart';
import 'package:goresave/features/editor/domain/editor_settings_store.dart';

final coreServiceProvider = Provider<GoresaveCoreService>((ref) {
  return NativeGoresaveCoreService.tryCreate() ?? MissingGoresaveCoreService();
});

final editorSettingsStoreProvider = Provider<EditorSettingsStore>((ref) {
  return JsonFileEditorSettingsStore.defaultForPlatform();
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
