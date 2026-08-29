import 'dart:io';

import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_riverpod/legacy.dart';
import 'package:goresave/features/app/domain/ui_settings.dart';
import 'package:goresave/features/app/ui/router.dart';
import 'package:goresave/features/editor/domain/core_service.dart';
import 'package:goresave/features/editor/domain/editor_notifier.dart';
import 'package:goresave/features/editor/domain/editor_settings_store.dart';
import 'package:goresave/features/editor/domain/character_category_catalog.dart';
import 'package:goresave/features/editor/domain/glossary_images.dart';
import 'package:goresave/features/editor/domain/glossary_npc_catalog.dart';
import 'package:goresave/features/editor/domain/item_stats.dart';
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

/// The bundled item-stats catalog: what the game's own script cache says about
/// every item, plus the inventory filters it groups them by. Purely
/// presentational, and never required — every caller falls back to the
/// id-prefix classifier and its own labels when this fails to load.
final itemStatsCatalogProvider = FutureProvider<ItemStatsCatalog>((ref) async {
  try {
    return await ItemStatsCatalog.loadBundled();
  } catch (_) {
    return const ItemStatsCatalog();
  }
});

/// Where each glossary entry's pencil portrait lives inside the installation.
/// Purely presentational; an unreadable asset simply means no portraits.
final glossaryImageCatalogProvider = FutureProvider<GlossaryImageCatalog>((
  ref,
) async {
  try {
    return await GlossaryImageCatalog.loadBundled();
  } catch (_) {
    return GlossaryImageCatalog();
  }
});

/// Unique NPC name (`BC_BAN_Arlin_852`, lowercased) -> its glossary document
/// class, so a character row can find the portrait the glossary holds for it.
/// Only the ~160 named characters have one; a generic worker or bandit has not.
final glossaryDocumentByNpcProvider = FutureProvider<Map<String, String>>((
  ref,
) async {
  try {
    final entries = await loadGlossaryNpcCatalog();
    return {
      for (final entry in entries)
        entry.uniqueName.toLowerCase(): entry.documentClass,
    };
  } catch (_) {
    return const {};
  }
});

/// Unique NPC name (lowercased) -> the roles the glossary files it under, so a
/// character's detail header can name the shops he runs and the skills he
/// teaches. Presentational; an unreadable asset simply means no role badges.
final glossaryRolesByNpcProvider =
    FutureProvider<Map<String, Set<NpcGlossaryRole>>>((ref) async {
      try {
        final entries = await loadGlossaryNpcCatalog();
        return {
          for (final entry in entries)
            entry.uniqueName.toLowerCase(): {
              for (final segment in entry.segments) ...segment.roles,
            },
        };
      } catch (_) {
        return const {};
      }
    });

/// Human / creature / other for a character reference, from the bundled
/// character-definition catalog. Decides which glyph stands in for a character
/// the glossary holds no portrait of.
final characterCategoryCatalogProvider =
    FutureProvider<CharacterCategoryCatalog>((ref) async {
      try {
        return await loadCharacterCategoryCatalog();
      } catch (_) {
        return CharacterCategoryCatalog(const {}, const {});
      }
    });
