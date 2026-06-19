import 'package:file_selector/file_selector.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:goresave/features/localization/domain/localization_controller.dart';

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
    const typeGroup = XTypeGroup(label: 'Localization cache', extensions: [
      'lcache',
    ]);
    final file = await openFile(acceptedTypeGroups: const [typeGroup]);
    if (file == null) {
      // User cancelled the picker; abort without an error SnackBar.
      return;
    }
    result = await controller.extract(lcacheHint: file.path);
  }

  if (!context.mounted) return;
  final messenger = ScaffoldMessenger.of(context);
  final message =
      result.message ?? (result.success ? 'Extraction complete' : 'Extraction failed');
  messenger.showSnackBar(SnackBar(content: Text(message)));
}
