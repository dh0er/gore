import 'package:file_selector/file_selector.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import '../../l10n/app_localizations.dart';
import '../../l10n/l10n_errors.dart';
import '../../editor/domain/overrides_notifier.dart';
import '../domain/export_notifier.dart';
import '../domain/export_request.dart';
import '../domain/mod_name.dart';

class ExportDialog extends ConsumerStatefulWidget {
  const ExportDialog({super.key});

  @override
  ConsumerState<ExportDialog> createState() => _ExportDialogState();
}

class _ExportDialogState extends ConsumerState<ExportDialog> {
  final _nameController  = TextEditingController(text: 'MyBalanceMod');
  final _delayController = TextEditingController(text: '0');
  String? _targetDir;
  bool _packageAsZip = false;
  String? _nameError;
  String? _delayError;

  @override
  void dispose() {
    _nameController.dispose();
    _delayController.dispose();
    super.dispose();
  }

  bool get _isValid =>
      _nameError == null &&
      _delayError == null &&
      _nameController.text.trim().isNotEmpty &&
      _targetDir != null;

  void _validateName(String v) {
    final l10n = AppLocalizations.of(context);
    final error = validateModName(v);
    setState(() {
      _nameError = error == null ? null : modNameErrorText(l10n, error);
    });
  }

  void _validateDelay(String v) {
    final l10n = AppLocalizations.of(context);
    final n = int.tryParse(v.trim());
    setState(() {
      _delayError =
          (n == null || n < 0) ? l10n.mustBeNonNegativeInteger : null;
    });
  }

  Future<void> _pickTargetDir() async {
    final dir = await getDirectoryPath(
      confirmButtonText: AppLocalizations.of(context).exportHere,
    );
    if (dir != null) setState(() => _targetDir = dir);
  }

  Future<void> _confirm() async {
    if (!_isValid) return;
    final overrides = ref.read(overridesProvider).entries;
    await ref.read(exportProvider.notifier).export(
      request: ExportRequest(
        modName:      _nameController.text.trim(),
        targetDir:    _targetDir!,
        delayMs:      int.tryParse(_delayController.text.trim()) ?? 0,
        packageAsZip: _packageAsZip,
      ),
      overrides: overrides,
    );
    // Result is surfaced by the ExportState watcher below.
  }

  @override
  Widget build(BuildContext context) {
    final exportState = ref.watch(exportProvider);
    final l10n = AppLocalizations.of(context);

    // Close on success
    ref.listen<ExportState>(exportProvider, (_, next) {
      if (next.result?.success == true && mounted) {
        Navigator.of(context).pop(next.result!.outputPath);
      }
    });

    return AlertDialog(
      title: Text(l10n.exportMod),
      content: SizedBox(
        width: 480,
        child: Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            TextField(
              controller: _nameController,
              decoration: InputDecoration(
                labelText: l10n.modName,
                errorText: _nameError,
              ),
              onChanged: _validateName,
            ),
            const SizedBox(height: 12),
            TextField(
              controller: _delayController,
              decoration: InputDecoration(
                labelText: l10n.loadDelayLabel,
                errorText: _delayError,
              ),
              keyboardType: TextInputType.number,
              inputFormatters: [FilteringTextInputFormatter.digitsOnly],
              onChanged: _validateDelay,
            ),
            const SizedBox(height: 12),
            Row(
              children: [
                Expanded(
                  child: Text(
                    _targetDir ?? l10n.noFolderSelected,
                    maxLines: 1,
                    overflow: TextOverflow.ellipsis,
                  ),
                ),
                const SizedBox(width: 8),
                OutlinedButton(
                  onPressed: _pickTargetDir,
                  child: Text(l10n.chooseFolder),
                ),
              ],
            ),
            const SizedBox(height: 12),
            CheckboxListTile(
              title: Text(l10n.packageAsZip),
              value: _packageAsZip,
              contentPadding: EdgeInsets.zero,
              onChanged: (v) => setState(() => _packageAsZip = v ?? false),
            ),
            if (exportState.validationErrors.isNotEmpty) ...[
              const SizedBox(height: 8),
              DecoratedBox(
                decoration: BoxDecoration(
                  color: Theme.of(context).colorScheme.errorContainer,
                  borderRadius: BorderRadius.circular(8),
                ),
                child: Padding(
                  padding: const EdgeInsets.all(8),
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      for (final err in exportState.validationErrors)
                        Text(err, style: TextStyle(color: Theme.of(context).colorScheme.onErrorContainer)),
                    ],
                  ),
                ),
              ),
            ],
            if (exportState.result?.error != null) ...[
              const SizedBox(height: 8),
              Text(
                exportState.result!.error!,
                style: TextStyle(color: Theme.of(context).colorScheme.error),
              ),
            ],
          ],
        ),
      ),
      actions: [
        TextButton(
          onPressed: exportState.isExporting
              ? null
              : () => Navigator.of(context).pop(null),
          child: Text(l10n.cancel),
        ),
        FilledButton(
          onPressed: (!_isValid || exportState.isExporting) ? null : _confirm,
          child: exportState.isExporting
              ? const SizedBox(
                  width: 16,
                  height: 16,
                  child: CircularProgressIndicator(strokeWidth: 2),
                )
              : Text(l10n.export),
        ),
      ],
    );
  }
}
