import 'dart:convert';
import 'dart:io';

import 'package:flutter/material.dart';
import 'package:path/path.dart' as p;

import '../core/mod_ffi.dart';
import '../l10n/app_localizations.dart';
import 'current_project_controller.dart';

typedef Revision3ExactProjectExporter =
    Future<AuthoringRevision3ExactSnapshotExportResult> Function(String output);
typedef Revision3ProjectExportParentDirectoryPicker =
    Future<String?> Function();

/// Writes one immutable, exact-checkpoint project copy to a brand-new archive.
///
/// This is deliberately not presented as Build, Deploy, Backup, or Restore:
/// the current managed project remains authoritative and unchanged.
class Revision3ProjectExportDialog extends StatefulWidget {
  const Revision3ProjectExportDialog({
    required this.projectRevision,
    required this.export,
    required this.pickExistingParentDirectory,
    super.key,
  });

  final int projectRevision;
  final Revision3ExactProjectExporter export;
  final Revision3ProjectExportParentDirectoryPicker pickExistingParentDirectory;

  @override
  State<Revision3ProjectExportDialog> createState() =>
      _Revision3ProjectExportDialogState();
}

class _Revision3ProjectExportDialogState
    extends State<Revision3ProjectExportDialog> {
  final _formKey = GlobalKey<FormState>();
  late final TextEditingController _fileName;

  String? _parentDirectory;
  String? _error;
  bool _choosingParent = false;
  bool _exporting = false;
  bool _terminal = false;
  bool _destinationCorrectionRequired = false;
  String? _rejectedNormalizedOutput;
  String? _destinationCorrectionError;
  AuthoringRevision3ExactSnapshotExportResult? _result;

  bool get _busy => _choosingParent || _exporting;

  String? _normalizedOutputCandidate() {
    final parent = _parentDirectory;
    if (parent == null || _fileName.text.isEmpty) return null;
    return p.normalize(p.join(parent, _fileName.text));
  }

  void _refreshDestinationCorrectionGate() {
    final candidate = _normalizedOutputCandidate();
    final rejected = _rejectedNormalizedOutput;
    _destinationCorrectionRequired =
        candidate != null && rejected != null && p.equals(candidate, rejected);
    _error = _destinationCorrectionRequired
        ? _destinationCorrectionError
        : null;
  }

  void _rejectDestination(String output, String message) {
    _rejectedNormalizedOutput = p.normalize(output);
    _destinationCorrectionError = message;
    _destinationCorrectionRequired = true;
    _error = message;
  }

  String? _outputPreview(AppLocalizations l10n) {
    if (_parentDirectory == null ||
        validateRevision3ProjectExportFileName(_fileName.text, l10n) != null) {
      return null;
    }
    return _normalizedOutputCandidate();
  }

  @override
  void initState() {
    super.initState();
    _fileName = TextEditingController(
      text: 'project-copy-r${widget.projectRevision}.goremod',
    );
  }

  @override
  void dispose() {
    _fileName.dispose();
    super.dispose();
  }

  Future<void> _chooseParent() async {
    if (_busy || _terminal || _result != null) return;
    final l10n = AppLocalizations.of(context);
    setState(() {
      _choosingParent = true;
      if (!_destinationCorrectionRequired) _error = null;
    });
    try {
      final selected = await widget.pickExistingParentDirectory();
      if (!mounted || selected == null) return;
      final validation = validateRevision3ProjectExportParent(selected, l10n);
      if (validation != null) {
        setState(() => _error = validation);
        return;
      }
      setState(() {
        _parentDirectory = p.normalize(selected);
        _refreshDestinationCorrectionGate();
      });
    } catch (_) {
      if (!mounted) return;
      setState(() => _error = l10n.projectExportParentInspectFailed);
    } finally {
      if (mounted) setState(() => _choosingParent = false);
    }
  }

  Future<void> _export() async {
    if (_busy || _terminal || _result != null) return;
    if (!(_formKey.currentState?.validate() ?? false)) return;
    final l10n = AppLocalizations.of(context);
    final parent = _parentDirectory;
    if (parent == null) {
      setState(() => _error = l10n.projectExportParentRequired);
      return;
    }

    setState(() {
      _exporting = true;
      _error = null;
    });
    String? attemptedOutput;
    var exportBoundaryEntered = false;
    try {
      final parentError = validateRevision3ProjectExportParent(parent, l10n);
      if (parentError != null) {
        if (mounted) setState(() => _error = parentError);
        return;
      }
      final output = p.normalize(p.join(parent, _fileName.text));
      attemptedOutput = output;
      final outputType = FileSystemEntity.typeSync(output, followLinks: false);
      if (outputType != FileSystemEntityType.notFound) {
        if (!mounted) return;
        setState(() {
          final message = outputType == FileSystemEntityType.link
              ? l10n.projectExportOutputLink
              : l10n.projectExportOutputExists;
          _rejectDestination(output, message);
        });
        return;
      }

      exportBoundaryEntered = true;
      final result = await widget.export(output);
      if (!mounted) return;
      if (result.projectRevision != widget.projectRevision ||
          result.output != output) {
        setState(() {
          _terminal = true;
          _error = l10n.projectExportResultMismatch(output);
        });
        return;
      }
      setState(() {
        _result = result;
        _terminal = result.publicationIsUncertain;
      });
    } on Revision3ProjectExportStaleCheckpointException {
      if (!mounted) return;
      setState(() {
        _terminal = true;
        _error = l10n.projectExportStale;
      });
    } on Revision3ProjectExportRequiresReopenException catch (error) {
      if (!mounted) return;
      setState(() {
        _terminal = true;
        _error = error.publicationMayExist && exportBoundaryEntered
            ? l10n.projectExportMayExist(attemptedOutput ?? '')
            : l10n.projectExportRequiresReopen;
      });
    } on Revision3ProjectExportUnsupportedException {
      if (!mounted) return;
      setState(() {
        _terminal = true;
        _error = l10n.projectExportUnsupported;
      });
    } on Revision3ProjectExportFailedException catch (error) {
      if (!mounted) return;
      setState(() {
        if (error.publicationMayExist) {
          _terminal = true;
          _error = l10n.projectExportMayExist(attemptedOutput ?? '');
        } else if (error.retryWithNewDestination) {
          _terminal = false;
          final message =
              error.code == 'AUTHORING_REVISION3_EXPORT_OUTPUT_EXISTS'
              ? l10n.projectExportOutputExists
              : l10n.projectExportOutputRejected;
          _rejectDestination(attemptedOutput!, message);
        } else {
          _terminal = true;
          _error = l10n.projectExportPrepublicationFailed;
        }
      });
    } catch (_) {
      if (!mounted) return;
      setState(() {
        _terminal = exportBoundaryEntered;
        _error = exportBoundaryEntered
            ? l10n.projectExportMayExist(attemptedOutput ?? '')
            : l10n.projectExportFailedBeforeStart;
      });
    } finally {
      if (mounted) setState(() => _exporting = false);
    }
  }

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context);
    final result = _result;
    final outputPreview = _outputPreview(l10n);
    return PopScope(
      canPop: !_busy,
      child: AlertDialog(
        key: const Key('revision3-project-export-dialog'),
        title: Text(l10n.projectExportDialogTitle),
        content: SizedBox(
          width: 640,
          child: SingleChildScrollView(
            child: Column(
              mainAxisSize: MainAxisSize.min,
              crossAxisAlignment: CrossAxisAlignment.stretch,
              children: [
                _ProjectExportNotice(copy: l10n),
                const SizedBox(height: 16),
                Form(
                  key: _formKey,
                  child: TextFormField(
                    key: const Key('revision3-project-export-file-name'),
                    controller: _fileName,
                    enabled: !_busy && !_terminal && result == null,
                    autovalidateMode: AutovalidateMode.onUserInteraction,
                    decoration: InputDecoration(
                      labelText: l10n.projectExportFileNameLabel,
                      helperText: l10n.projectExportFileNameHelper,
                      border: const OutlineInputBorder(),
                    ),
                    validator: (value) =>
                        validateRevision3ProjectExportFileName(value, l10n),
                    onChanged: (_) =>
                        setState(_refreshDestinationCorrectionGate),
                  ),
                ),
                const SizedBox(height: 12),
                Row(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    OutlinedButton.icon(
                      key: const Key('revision3-project-export-choose-parent'),
                      onPressed: _busy || _terminal || result != null
                          ? null
                          : _chooseParent,
                      icon: _choosingParent
                          ? const SizedBox.square(
                              key: Key(
                                'revision3-project-export-pick-progress',
                              ),
                              dimension: 16,
                              child: CircularProgressIndicator(strokeWidth: 2),
                            )
                          : const Icon(Icons.folder_open_outlined),
                      label: Text(l10n.projectExportChooseDestination),
                    ),
                    const SizedBox(width: 12),
                    Expanded(
                      child: Text(
                        _parentDirectory ?? l10n.projectExportNoDestination,
                        key: const Key('revision3-project-export-parent'),
                      ),
                    ),
                  ],
                ),
                if (outputPreview != null) ...[
                  const SizedBox(height: 12),
                  _ProjectExportFact(
                    label: l10n.projectExportNewFile,
                    value: outputPreview,
                    valueKey: const Key(
                      'revision3-project-export-output-preview',
                    ),
                  ),
                ],
                if (_error case final error?) ...[
                  const SizedBox(height: 14),
                  _ProjectExportMessage(
                    key: const Key('revision3-project-export-error'),
                    message: error,
                    error: true,
                  ),
                ],
                if (result != null) ...[
                  const Divider(height: 32),
                  _ProjectExportResult(result: result, copy: l10n),
                ],
              ],
            ),
          ),
        ),
        actions: [
          TextButton(
            key: const Key('revision3-project-export-close'),
            onPressed: _busy ? null : () => Navigator.of(context).pop(result),
            child: Text(
              result == null && !_terminal
                  ? l10n.projectExportCancel
                  : l10n.projectExportClose,
            ),
          ),
          if (result == null)
            FilledButton.icon(
              key: const Key('revision3-project-export-submit'),
              onPressed:
                  !_busy &&
                      !_terminal &&
                      !_destinationCorrectionRequired &&
                      _parentDirectory != null &&
                      validateRevision3ProjectExportFileName(
                            _fileName.text,
                            l10n,
                          ) ==
                          null
                  ? _export
                  : null,
              icon: _exporting
                  ? const SizedBox.square(
                      key: Key('revision3-project-export-progress'),
                      dimension: 16,
                      child: CircularProgressIndicator(strokeWidth: 2),
                    )
                  : const Icon(Icons.archive_outlined),
              label: Text(
                _exporting
                    ? l10n.projectExportExporting
                    : l10n.projectExportSubmit,
              ),
            ),
        ],
      ),
    );
  }
}

String? validateRevision3ProjectExportParent(
  String value,
  AppLocalizations l10n,
) {
  if (value.isEmpty || !p.isAbsolute(value)) {
    return l10n.projectExportParentAbsolute;
  }
  final type = FileSystemEntity.typeSync(value, followLinks: false);
  return switch (type) {
    FileSystemEntityType.directory => null,
    FileSystemEntityType.link => l10n.projectExportParentLink,
    _ => l10n.projectExportParentRequired,
  };
}

String? validateRevision3ProjectExportFileName(
  String? raw,
  AppLocalizations l10n,
) {
  final value = raw ?? '';
  if (value.isEmpty) return l10n.projectExportFileNameRequired;
  if (utf8.encode(value).length > 128) {
    return l10n.projectExportFileNameTooLong;
  }
  if (!value.runes.every((rune) => rune <= 0x7f) ||
      !RegExp(
        r'^[A-Za-z0-9][A-Za-z0-9._-]*\.goremod$',
        caseSensitive: false,
      ).hasMatch(value)) {
    return l10n.projectExportFileNameInvalid;
  }
  final stem = value.substring(0, value.length - '.goremod'.length);
  final deviceStem = stem.split('.').first.toUpperCase();
  if (const {
        'CON',
        'PRN',
        'AUX',
        'NUL',
        r'CLOCK$',
        r'CONIN$',
        r'CONOUT$',
      }.contains(deviceStem) ||
      RegExp(r'^(COM|LPT)[1-9]$').hasMatch(deviceStem)) {
    return l10n.projectExportFileNameReserved;
  }
  return null;
}

class _ProjectExportNotice extends StatelessWidget {
  const _ProjectExportNotice({required this.copy});

  final AppLocalizations copy;

  @override
  Widget build(BuildContext context) => DecoratedBox(
    decoration: BoxDecoration(
      color: Theme.of(context).colorScheme.surfaceContainerHighest,
      borderRadius: BorderRadius.circular(10),
    ),
    child: Padding(
      padding: const EdgeInsets.all(14),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Text(
            copy.projectExportPortableCopyTitle,
            style: Theme.of(context).textTheme.titleSmall,
          ),
          const SizedBox(height: 6),
          Text(copy.projectExportPortableCopyDescription),
          const SizedBox(height: 6),
          Text(copy.projectExportCapabilityBoundary),
          const SizedBox(height: 6),
          Text(copy.projectExportKeepOriginal),
        ],
      ),
    ),
  );
}

class _ProjectExportResult extends StatelessWidget {
  const _ProjectExportResult({required this.result, required this.copy});

  final AuthoringRevision3ExactSnapshotExportResult result;
  final AppLocalizations copy;

  @override
  Widget build(BuildContext context) {
    final uncertain = result.publicationIsUncertain;
    final cleanupWarning = result.hasCleanupWarning;
    return Column(
      key: const Key('revision3-project-export-result'),
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        _ProjectExportMessage(
          key: Key(
            uncertain
                ? 'revision3-project-export-uncertain'
                : cleanupWarning
                ? 'revision3-project-export-cleanup-warning'
                : 'revision3-project-export-published',
          ),
          message: uncertain
              ? copy.projectExportPublicationUncertain(result.output)
              : cleanupWarning
              ? copy.projectExportPublishedCleanupWarning
              : copy.projectExportPublished,
          error: uncertain,
        ),
        const SizedBox(height: 12),
        _ProjectExportFact(
          label: copy.projectExportNewFile,
          value: result.output,
        ),
        _ProjectExportFact(
          label: copy.projectRevision,
          value: '${result.projectRevision}',
        ),
        _ProjectExportFact(
          label: copy.projectExportArchiveBytes,
          value: '${result.archive.byteLength}',
        ),
        _ProjectExportFact(
          label: copy.projectExportArchiveSha256,
          value: result.archive.sha256,
        ),
        const SizedBox(height: 8),
        Text(copy.projectExportCurrentProjectUnchanged),
      ],
    );
  }
}

class _ProjectExportMessage extends StatelessWidget {
  const _ProjectExportMessage({
    required this.message,
    required this.error,
    super.key,
  });

  final String message;
  final bool error;

  @override
  Widget build(BuildContext context) {
    final colors = Theme.of(context).colorScheme;
    return DecoratedBox(
      decoration: BoxDecoration(
        color: error ? colors.errorContainer : colors.primaryContainer,
        borderRadius: BorderRadius.circular(8),
      ),
      child: Padding(
        padding: const EdgeInsets.all(12),
        child: Text(
          message,
          style: TextStyle(
            color: error ? colors.onErrorContainer : colors.onPrimaryContainer,
            fontWeight: FontWeight.w600,
          ),
        ),
      ),
    );
  }
}

class _ProjectExportFact extends StatelessWidget {
  const _ProjectExportFact({
    required this.label,
    required this.value,
    this.valueKey,
  });

  final String label;
  final String value;
  final Key? valueKey;

  @override
  Widget build(BuildContext context) => Padding(
    padding: const EdgeInsets.only(bottom: 6),
    child: Row(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        SizedBox(
          width: 130,
          child: Text(
            label,
            style: const TextStyle(fontWeight: FontWeight.w600),
          ),
        ),
        Expanded(child: SelectableText(value, key: valueKey)),
      ],
    ),
  );
}
