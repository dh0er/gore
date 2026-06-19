import 'package:file_selector/file_selector.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import '../domain/loc_notifier.dart';

/// Run the shared extraction flow used by both the first-run confirmation and
/// the manual AppBar action. Tries auto-detect first; on LCACHE_NOT_FOUND it
/// opens a file picker for the .lcache and retries with that hint. Surfaces
/// progress and the success/failure result via SnackBars.
Future<void> runLocExtractFlow(BuildContext context, WidgetRef ref) async {
  final messenger = ScaffoldMessenger.of(context);
  final notifier = ref.read(locProvider.notifier);

  messenger.showSnackBar(
    const SnackBar(content: Text('Extracting localized game text…')),
  );

  var outcome = await notifier.extract();

  if (outcome.needsManualFile) {
    messenger.hideCurrentSnackBar();
    if (!context.mounted) return;
    const group = XTypeGroup(label: 'localization cache', extensions: ['lcache']);
    final file = await openFile(acceptedTypeGroups: [group]);
    if (file == null) {
      // User cancelled the picker — abort gracefully, no error.
      messenger.showSnackBar(
        const SnackBar(content: Text('Localized text extraction cancelled.')),
      );
      return;
    }
    messenger.showSnackBar(
      const SnackBar(content: Text('Extracting localized game text…')),
    );
    outcome = await notifier.extract(lcacheHint: file.path);
  }

  messenger.hideCurrentSnackBar();
  if (outcome.success) {
    messenger.showSnackBar(
      SnackBar(
        content: Text(
          outcome.message ?? 'Localized text extracted.',
        ),
      ),
    );
  } else if (!outcome.needsManualFile) {
    messenger.showSnackBar(
      SnackBar(
        content: Text(outcome.message ?? 'Extraction failed.'),
      ),
    );
  }
}

/// Show the optional first-run confirmation dialog. Returns true if the user
/// chose to extract now.
Future<bool> showLocFirstRunDialog(BuildContext context) async {
  final result = await showDialog<bool>(
    context: context,
    builder: (context) => AlertDialog(
      title: const Text('Extract localized game text?'),
      content: const Text(
        "Localized game text isn't extracted yet. Extract it now from your "
        'game install? (optional)',
      ),
      actions: [
        TextButton(
          onPressed: () => Navigator.of(context).pop(false),
          child: const Text('Not now'),
        ),
        FilledButton(
          onPressed: () => Navigator.of(context).pop(true),
          child: const Text('Extract'),
        ),
      ],
    ),
  );
  return result ?? false;
}
