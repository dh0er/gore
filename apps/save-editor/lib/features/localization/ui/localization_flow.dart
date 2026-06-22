import 'package:file_selector/file_selector.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:goresave/features/localization/domain/localization_controller.dart';
import 'package:goresave/l10n/app_localizations.dart';
import 'package:goresave/loc/loc_catalog_provider.dart';

/// Runs the shared "extract localized game text" flow used by both the
/// first-run confirmation and the manual Settings button.
///
/// 1. Attempts auto-detect extraction (no hint).
/// 2. On a not-found auto-detect failure, opens a `.lcache` file picker and
///    retries with the picked path; if the user cancels, aborts gracefully.
/// 3. Reports success/failure via a SnackBar.
///
/// [context] must come from a widget that is still mounted; the function checks
/// `context.mounted` after each await before touching the UI.
Future<void> runLocalizationExtractFlow(
  BuildContext context,
  WidgetRef ref,
) async {
  final controller = ref.read(localizationControllerProvider.notifier);

  var result = await controller.extract();

  if (result.notFound) {
    if (!context.mounted) return;
    // Auto-detect failed: let the user point us at the .lcache directly.
    final typeGroup = XTypeGroup(
      label: AppLocalizations.of(context).localizationCacheFileType,
      extensions: const ['lcache'],
    );
    final file = await openFile(acceptedTypeGroups: [typeGroup]);
    if (file == null) {
      // User cancelled the picker; abort without an error SnackBar.
      return;
    }
    result = await controller.extract(lcacheHint: file.path);
  }

  // The widget may have been disposed during the extraction await(s); don't
  // touch ref/context if so.
  if (!context.mounted) return;
  // Reload the cached catalog after any extract attempt: the catalog file can
  // be (re)written even when extraction reports an error (e.g. the catalog was
  // written but the meta write then failed), so the new names should still show.
  ref.read(locCatalogReloadProvider.notifier).state++;

  final messenger = ScaffoldMessenger.of(context);
  final l10n = AppLocalizations.of(context);
  final String message;
  if (result.success) {
    final ids = result.idCount;
    final langs = result.languageCount;
    message = (ids != null && langs != null)
        ? l10n.localizedTextExtractedCount(ids, langs)
        : l10n.extractionComplete;
  } else {
    message = result.message ?? l10n.extractionFailed;
  }
  messenger.showSnackBar(SnackBar(content: Text(message)));
}
