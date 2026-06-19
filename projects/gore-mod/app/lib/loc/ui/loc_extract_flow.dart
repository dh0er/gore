import 'package:file_selector/file_selector.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import '../../l10n/app_localizations.dart';
import '../domain/loc_catalog_provider.dart';
import '../domain/loc_notifier.dart';

/// Run the shared extraction flow used by both the first-run confirmation and
/// the manual AppBar action. Tries auto-detect first; on LCACHE_NOT_FOUND it
/// opens a file picker for the .lcache and retries with that hint. Surfaces
/// progress and the success/failure result via SnackBars. On success the loaded
/// localization catalog is invalidated so item names refresh.
Future<void> runLocExtractFlow(BuildContext context, WidgetRef ref) async {
  final messenger = ScaffoldMessenger.of(context);
  final l10n = AppLocalizations.of(context);
  final notifier = ref.read(locProvider.notifier);

  messenger.showSnackBar(
    SnackBar(content: Text(l10n.extractingLocalizedText)),
  );

  var outcome = await notifier.extract();

  if (outcome.needsManualFile) {
    // The first extract() already awaited; bail if the page is gone before
    // touching the (captured) messenger.
    if (!context.mounted) return;
    messenger.hideCurrentSnackBar();
    final group = XTypeGroup(
      label: l10n.localizationCacheFileGroupLabel,
      extensions: const ['lcache'],
    );
    final file = await openFile(acceptedTypeGroups: [group]);
    if (!context.mounted) return;
    if (file == null) {
      // User cancelled the picker — abort gracefully, no error.
      messenger.showSnackBar(
        SnackBar(content: Text(l10n.localizedTextExtractionCancelled)),
      );
      return;
    }
    messenger.showSnackBar(
      SnackBar(content: Text(l10n.extractingLocalizedText)),
    );
    outcome = await notifier.extract(lcacheHint: file.path);
  }

  // The widget may have been disposed during the extraction await(s); don't
  // touch ref/messenger if so.
  if (!context.mounted) return;
  messenger.hideCurrentSnackBar();
  // Reload the catalog after any extract attempt: the catalog file can be
  // (re)written even when extraction reports an error (e.g. the catalog was
  // written but the meta write then failed), so the new names should still show.
  ref.invalidate(locCatalogProvider);
  if (outcome.success) {
    final ids = outcome.idCount;
    final langs = outcome.languageCount;
    messenger.showSnackBar(
      SnackBar(
        content: Text(
          ids != null && langs != null
              ? l10n.localizedTextExtractedCount(ids, langs)
              : l10n.localizedTextExtracted,
        ),
      ),
    );
  } else if (!outcome.needsManualFile) {
    messenger.showSnackBar(
      SnackBar(
        content: Text(outcome.message ?? l10n.extractionFailed),
      ),
    );
  }
}

/// Show the optional first-run confirmation dialog. Returns true if the user
/// chose to extract now.
Future<bool> showLocFirstRunDialog(BuildContext context) async {
  final result = await showDialog<bool>(
    context: context,
    builder: (context) {
      final l10n = AppLocalizations.of(context);
      return AlertDialog(
        title: Text(l10n.extractLocalizedTextQuestion),
        content: Text(l10n.extractLocalizedTextBody),
        actions: [
          TextButton(
            onPressed: () => Navigator.of(context).pop(false),
            child: Text(l10n.notNow),
          ),
          FilledButton(
            onPressed: () => Navigator.of(context).pop(true),
            child: Text(l10n.extract),
          ),
        ],
      );
    },
  );
  return result ?? false;
}
