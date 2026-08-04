import 'dart:convert';
import 'dart:io';

import 'package:flutter/material.dart';
import 'package:path/path.dart' as p;

import '../l10n/app_localizations.dart';
import 'revision3_project_import.dart';

typedef Revision3ProjectImportParentDirectoryPicker =
    Future<String?> Function();

/// The only successful value that may leave the restore dialog.
///
/// Publication uncertainty, cancellation, and every failure return no result.
/// The cleanup bit is retained separately so the shell can report it without
/// weakening or reconstructing the native receipt.
final class Revision3ProjectImportDialogResult {
  const Revision3ProjectImportDialogResult({
    required this.receipt,
    required this.hasCleanupWarning,
  });

  final Revision3ProjectImportedReceipt receipt;
  final bool hasCleanupWarning;
}

Future<Revision3ProjectImportDialogResult?> showRevision3ProjectImportDialog({
  required BuildContext context,
  required Revision3ProjectImportSourcePicker pickSource,
  required Revision3ProjectImportNativeInspector inspect,
  required Revision3ProjectImportParentDirectoryPicker
  pickExistingParentDirectory,
  required Revision3ProjectImportNativeDestinationImporter importProject,
}) => showDialog<Revision3ProjectImportDialogResult>(
  context: context,
  barrierDismissible: false,
  builder: (_) => Revision3ProjectImportDialog(
    pickSource: pickSource,
    inspect: inspect,
    pickExistingParentDirectory: pickExistingParentDirectory,
    importProject: importProject,
  ),
);

/// Verifies one restorable V2 backup and materializes it into one new folder.
///
/// The dialog deliberately has no session opener or adoption callback. It
/// returns only a natively confirmed receipt; the app-wide current-project
/// coordinator owns the later receipt-bound candidate open and adoption.
class Revision3ProjectImportDialog extends StatefulWidget {
  const Revision3ProjectImportDialog({
    required this.pickSource,
    required this.inspect,
    required this.pickExistingParentDirectory,
    required this.importProject,
    super.key,
  });

  final Revision3ProjectImportSourcePicker pickSource;
  final Revision3ProjectImportNativeInspector inspect;
  final Revision3ProjectImportParentDirectoryPicker pickExistingParentDirectory;
  final Revision3ProjectImportNativeDestinationImporter importProject;

  @override
  State<Revision3ProjectImportDialog> createState() =>
      _Revision3ProjectImportDialogState();
}

class _Revision3ProjectImportDialogState
    extends State<Revision3ProjectImportDialog> {
  final _formKey = GlobalKey<FormState>();
  final _folderName = TextEditingController();
  final Object _lifecycleOwner = Object();

  late final Revision3ProjectImportInspectionCoordinator _inspectionCoordinator;
  late final Revision3ProjectImportDestinationCoordinator
  _destinationCoordinator;

  Revision3ProjectImportInspectionPlan? _plan;
  String? _parentDirectory;
  String? _pendingDestination;
  String? _attemptedDestinationLabel;
  String? _message;
  bool _messageIsError = false;
  bool _active = true;
  bool _inspecting = false;
  bool _choosingParent = false;
  bool _materializing = false;
  bool _terminal = false;
  int _generation = 0;

  bool get _busy => _inspecting || _choosingParent || _materializing;

  Revision3ProjectImportLifecycle? _readLifecycle() => _active
      ? Revision3ProjectImportLifecycle(
          owner: _lifecycleOwner,
          generation: _generation,
        )
      : null;

  @override
  void initState() {
    super.initState();
    _inspectionCoordinator = Revision3ProjectImportInspectionCoordinator(
      readLifecycle: _readLifecycle,
      pickSource: widget.pickSource,
      inspect: widget.inspect,
    );
    _destinationCoordinator = Revision3ProjectImportDestinationCoordinator(
      readLifecycle: _readLifecycle,
      pickDestination: () async => _pendingDestination,
      importProject: widget.importProject,
    );
  }

  @override
  void dispose() {
    _active = false;
    _generation++;
    _inspectionCoordinator.dispose();
    _destinationCoordinator.dispose();
    _folderName.dispose();
    super.dispose();
  }

  Future<void> _chooseAndInspectBackup() async {
    if (_busy || _terminal) return;
    setState(() {
      _generation++;
      _plan = null;
      _parentDirectory = null;
      _pendingDestination = null;
      _attemptedDestinationLabel = null;
      _folderName.clear();
      _message = null;
      _messageIsError = false;
      _inspecting = true;
    });

    final result = await _inspectionCoordinator.plan();
    if (!mounted) return;
    final l10n = AppLocalizations.of(context);
    setState(() {
      _inspecting = false;
      switch (result.outcome) {
        case Revision3ProjectImportPlanningOutcome.inspected:
          final plan = result.plan!;
          _plan = plan;
          _folderName.text =
              'restored-project-r${plan.inspection.projectRevision}';
          _message = l10n.projectRestoreVerified;
          _messageIsError = false;
        case Revision3ProjectImportPlanningOutcome.cancelled:
          _message = null;
        case Revision3ProjectImportPlanningOutcome.invalidSource:
          _message = l10n.projectRestoreInvalidSource;
          _messageIsError = true;
        case Revision3ProjectImportPlanningOutcome.inspectionFailed:
          _message = l10n.projectRestoreInspectionFailed;
          _messageIsError = true;
        case Revision3ProjectImportPlanningOutcome.unavailable:
          _message = l10n.projectRestoreUnavailable;
          _messageIsError = true;
        case Revision3ProjectImportPlanningOutcome.stale:
        case Revision3ProjectImportPlanningOutcome.superseded:
          _message = l10n.projectRestoreStale;
          _messageIsError = true;
        case Revision3ProjectImportPlanningOutcome.busy:
          _message = l10n.projectRestoreInspectionFailed;
          _messageIsError = true;
      }
    });
  }

  Future<void> _chooseParent() async {
    if (_busy || _terminal || _plan == null) return;
    setState(() {
      _choosingParent = true;
      _message = null;
      _messageIsError = false;
    });
    try {
      final selected = await widget.pickExistingParentDirectory();
      if (!mounted || selected == null) return;
      final l10n = AppLocalizations.of(context);
      final validation = validateRevision3ProjectImportParent(selected, l10n);
      if (validation != null) {
        setState(() {
          _message = validation;
          _messageIsError = true;
        });
        return;
      }
      setState(() {
        _parentDirectory = p.normalize(selected);
        _message = null;
        _messageIsError = false;
      });
    } catch (_) {
      if (!mounted) return;
      setState(() {
        _message = AppLocalizations.of(
          context,
        ).projectExportParentInspectFailed;
        _messageIsError = true;
      });
    } finally {
      if (mounted) setState(() => _choosingParent = false);
    }
  }

  String? _destinationPreview(AppLocalizations l10n) {
    final parent = _parentDirectory;
    if (parent == null ||
        validateRevision3ProjectImportFolderName(_folderName.text, l10n) !=
            null) {
      return null;
    }
    return p.normalize(p.join(parent, _folderName.text));
  }

  Future<void> _materialize() async {
    if (_busy || _terminal || _plan == null) return;
    if (!(_formKey.currentState?.validate() ?? false)) return;
    final l10n = AppLocalizations.of(context);
    final parent = _parentDirectory;
    if (parent == null) {
      setState(() {
        _message = l10n.projectExportParentRequired;
        _messageIsError = true;
      });
      return;
    }
    final parentError = validateRevision3ProjectImportParent(parent, l10n);
    if (parentError != null) {
      setState(() {
        _message = parentError;
        _messageIsError = true;
      });
      return;
    }

    final destination = p.normalize(p.join(parent, _folderName.text));
    final FileSystemEntityType destinationType;
    try {
      destinationType = FileSystemEntity.typeSync(
        destination,
        followLinks: false,
      );
    } catch (_) {
      setState(() {
        _message = l10n.projectRestoreDestinationInvalid;
        _messageIsError = true;
      });
      return;
    }
    if (destinationType != FileSystemEntityType.notFound) {
      setState(() {
        _message = destinationType == FileSystemEntityType.link
            ? l10n.projectRestoreDestinationLink
            : l10n.projectRestoreDestinationExists;
        _messageIsError = true;
      });
      return;
    }

    setState(() {
      _pendingDestination = destination;
      _attemptedDestinationLabel = _folderName.text;
      _materializing = true;
      _message = null;
      _messageIsError = false;
    });
    final result = await _destinationCoordinator.materialize(_plan!);
    if (!mounted) return;
    setState(() => _materializing = false);

    switch (result.outcome) {
      case Revision3ProjectImportDestinationExecutionOutcome.imported:
      case Revision3ProjectImportDestinationExecutionOutcome
          .importedWithCleanupWarning:
        final receipt = result.receipt;
        if (receipt == null) {
          setState(() {
            _terminal = true;
            _message = l10n.projectRestoreMaterializationFailed;
            _messageIsError = true;
          });
          return;
        }
        if (!mounted) return;
        Navigator.of(context).pop(
          Revision3ProjectImportDialogResult(
            receipt: receipt,
            hasCleanupWarning:
                result.outcome ==
                Revision3ProjectImportDestinationExecutionOutcome
                    .importedWithCleanupWarning,
          ),
        );
      case Revision3ProjectImportDestinationExecutionOutcome
          .publicationUncertain:
        setState(() {
          _terminal = true;
          _message = l10n.projectRestorePublicationUncertain(
            _attemptedDestinationLabel ?? _folderName.text,
          );
          _messageIsError = true;
        });
      case Revision3ProjectImportDestinationExecutionOutcome.invalidDestination:
        _setTerminalError(l10n.projectRestoreDestinationInvalid);
      case Revision3ProjectImportDestinationExecutionOutcome.inspectionExpired:
        _setTerminalError(l10n.projectRestoreInspectionExpired);
      case Revision3ProjectImportDestinationExecutionOutcome.importFailed:
        _setTerminalError(l10n.projectRestoreMaterializationFailed);
      case Revision3ProjectImportDestinationExecutionOutcome.stale:
      case Revision3ProjectImportDestinationExecutionOutcome.superseded:
        _setTerminalError(l10n.projectRestoreStale);
      case Revision3ProjectImportDestinationExecutionOutcome.cancelled:
      case Revision3ProjectImportDestinationExecutionOutcome.busy:
      case Revision3ProjectImportDestinationExecutionOutcome.unavailable:
        _setTerminalError(
          result.outcome ==
                  Revision3ProjectImportDestinationExecutionOutcome.unavailable
              ? l10n.projectRestoreUnavailable
              : l10n.projectRestoreMaterializationFailed,
        );
    }
  }

  void _setTerminalError(String message) {
    if (!mounted) return;
    setState(() {
      _terminal = true;
      _message = message;
      _messageIsError = true;
    });
  }

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context);
    final plan = _plan;
    final destinationPreview = _destinationPreview(l10n);
    return PopScope(
      canPop: !_materializing,
      child: AlertDialog(
        key: const Key('revision3-project-import-dialog'),
        title: Text(l10n.projectRestoreDialogTitle),
        content: SizedBox(
          width: 680,
          child: SingleChildScrollView(
            child: Column(
              mainAxisSize: MainAxisSize.min,
              crossAxisAlignment: CrossAxisAlignment.stretch,
              children: [
                _ProjectRestoreNotice(copy: l10n),
                const SizedBox(height: 16),
                OutlinedButton.icon(
                  key: const Key('revision3-project-import-choose-source'),
                  onPressed: _busy || _terminal
                      ? null
                      : _chooseAndInspectBackup,
                  icon: _inspecting
                      ? const SizedBox.square(
                          key: Key('revision3-project-import-inspect-progress'),
                          dimension: 16,
                          child: CircularProgressIndicator(strokeWidth: 2),
                        )
                      : const Icon(Icons.inventory_2_outlined),
                  label: Text(
                    _inspecting
                        ? l10n.projectRestoreInspecting
                        : l10n.projectRestoreChooseBackup,
                  ),
                ),
                const SizedBox(height: 10),
                if (plan == null)
                  Text(
                    l10n.projectRestoreNoBackup,
                    key: const Key('revision3-project-import-no-source'),
                  )
                else ...[
                  _ProjectRestoreFact(
                    label: l10n.projectRestoreSource,
                    value: plan.sourceLabel,
                    valueKey: const Key('revision3-project-import-source'),
                  ),
                  _ProjectRestoreFact(
                    label: l10n.projectRestoreProjectRevision,
                    value: '${plan.inspection.projectRevision}',
                  ),
                  _ProjectRestoreFact(
                    label: l10n.projectRestoreArchiveBytes,
                    value: '${plan.inspection.archive.byteLength}',
                  ),
                  _ProjectRestoreFact(
                    label: l10n.projectRestoreStoreObjects,
                    value: '${plan.inspection.closure.storeObjects}',
                  ),
                  const Divider(height: 28),
                  Form(
                    key: _formKey,
                    child: TextFormField(
                      key: const Key('revision3-project-import-folder-name'),
                      controller: _folderName,
                      enabled: !_busy && !_terminal,
                      autovalidateMode: AutovalidateMode.onUserInteraction,
                      decoration: InputDecoration(
                        labelText: l10n.projectRestoreFolderNameLabel,
                        helperText: l10n.projectRestoreFolderNameHelper,
                        border: const OutlineInputBorder(),
                      ),
                      validator: (value) =>
                          validateRevision3ProjectImportFolderName(value, l10n),
                      onChanged: (_) => setState(() {
                        _message = null;
                        _messageIsError = false;
                      }),
                    ),
                  ),
                  const SizedBox(height: 12),
                  Row(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      OutlinedButton.icon(
                        key: const Key(
                          'revision3-project-import-choose-parent',
                        ),
                        onPressed: _busy || _terminal ? null : _chooseParent,
                        icon: _choosingParent
                            ? const SizedBox.square(
                                key: Key(
                                  'revision3-project-import-parent-progress',
                                ),
                                dimension: 16,
                                child: CircularProgressIndicator(
                                  strokeWidth: 2,
                                ),
                              )
                            : const Icon(Icons.create_new_folder_outlined),
                        label: Text(l10n.projectRestoreChooseDestinationParent),
                      ),
                      const SizedBox(width: 12),
                      Expanded(
                        child: Text(
                          _parentDirectory ??
                              l10n.projectRestoreNoDestinationParent,
                          key: const Key('revision3-project-import-parent'),
                        ),
                      ),
                    ],
                  ),
                  if (destinationPreview != null) ...[
                    const SizedBox(height: 12),
                    _ProjectRestoreFact(
                      label: l10n.projectRestoreNewFolder,
                      value: destinationPreview,
                      valueKey: const Key(
                        'revision3-project-import-destination-preview',
                      ),
                    ),
                  ],
                ],
                if (_message case final message?) ...[
                  const SizedBox(height: 14),
                  _ProjectRestoreMessage(
                    key: const Key('revision3-project-import-message'),
                    message: message,
                    error: _messageIsError,
                  ),
                ],
              ],
            ),
          ),
        ),
        actions: [
          TextButton(
            key: const Key('revision3-project-import-close'),
            onPressed: _materializing
                ? null
                : () => Navigator.of(context).pop(),
            child: Text(
              _terminal ? l10n.projectRestoreClose : l10n.projectRestoreCancel,
            ),
          ),
          if (!_terminal)
            FilledButton.icon(
              key: const Key('revision3-project-import-submit'),
              onPressed:
                  !_busy &&
                      plan != null &&
                      _parentDirectory != null &&
                      validateRevision3ProjectImportFolderName(
                            _folderName.text,
                            l10n,
                          ) ==
                          null
                  ? _materialize
                  : null,
              icon: _materializing
                  ? const SizedBox.square(
                      key: Key('revision3-project-import-materialize-progress'),
                      dimension: 16,
                      child: CircularProgressIndicator(strokeWidth: 2),
                    )
                  : const Icon(Icons.settings_backup_restore_outlined),
              label: Text(
                _materializing
                    ? l10n.projectRestoreRestoring
                    : l10n.projectRestoreSubmit,
              ),
            ),
        ],
      ),
    );
  }
}

/// Keeps a visible, non-dismissible progress surface around the shell-owned
/// receipt-bound candidate open. This is deliberately separate from
/// [Revision3ProjectImportDialog], which never receives session authority.
Future<T> showRevision3ProjectImportOpeningProgress<T>({
  required BuildContext context,
  required Future<T> Function() open,
}) async {
  final outcome = await showDialog<_Revision3ProjectOpeningOutcome<T>>(
    context: context,
    barrierDismissible: false,
    builder: (_) => _Revision3ProjectOpeningDialog<T>(open: open),
  );
  return switch (outcome) {
    _Revision3ProjectOpeningSuccess<T>(:final value) => value,
    _Revision3ProjectOpeningFailure<T>(:final error, :final stackTrace) =>
      Error.throwWithStackTrace(error, stackTrace),
    null => throw StateError(
      'the receipt-bound project opening progress route closed without a result',
    ),
  };
}

sealed class _Revision3ProjectOpeningOutcome<T> {
  const _Revision3ProjectOpeningOutcome();
}

final class _Revision3ProjectOpeningSuccess<T>
    extends _Revision3ProjectOpeningOutcome<T> {
  const _Revision3ProjectOpeningSuccess(this.value);

  final T value;
}

final class _Revision3ProjectOpeningFailure<T>
    extends _Revision3ProjectOpeningOutcome<T> {
  const _Revision3ProjectOpeningFailure(this.error, this.stackTrace);

  final Object error;
  final StackTrace stackTrace;
}

class _Revision3ProjectOpeningDialog<T> extends StatefulWidget {
  const _Revision3ProjectOpeningDialog({required this.open});

  final Future<T> Function() open;

  @override
  State<_Revision3ProjectOpeningDialog<T>> createState() =>
      _Revision3ProjectOpeningDialogState<T>();
}

class _Revision3ProjectOpeningDialogState<T>
    extends State<_Revision3ProjectOpeningDialog<T>> {
  @override
  void initState() {
    super.initState();
    WidgetsBinding.instance.addPostFrameCallback((_) => _open());
  }

  Future<void> _open() async {
    late final _Revision3ProjectOpeningOutcome<T> outcome;
    try {
      outcome = _Revision3ProjectOpeningSuccess<T>(await widget.open());
    } catch (error, stackTrace) {
      outcome = _Revision3ProjectOpeningFailure<T>(error, stackTrace);
    }
    if (!mounted) return;
    Navigator.of(context).pop(outcome);
  }

  @override
  Widget build(BuildContext context) => PopScope(
    canPop: false,
    child: AlertDialog(
      key: const Key('revision3-project-import-opening-dialog'),
      content: Row(
        mainAxisSize: MainAxisSize.min,
        children: [
          const SizedBox.square(
            dimension: 22,
            child: CircularProgressIndicator(strokeWidth: 2.5),
          ),
          const SizedBox(width: 16),
          Flexible(
            child: Text(AppLocalizations.of(context).projectRestoreOpening),
          ),
        ],
      ),
    ),
  );
}

String? validateRevision3ProjectImportParent(
  String value,
  AppLocalizations l10n,
) {
  if (value.isEmpty || !p.isAbsolute(value)) {
    return l10n.projectExportParentAbsolute;
  }
  final FileSystemEntityType type;
  try {
    type = FileSystemEntity.typeSync(value, followLinks: false);
  } catch (_) {
    return l10n.projectExportParentInspectFailed;
  }
  return switch (type) {
    FileSystemEntityType.directory => null,
    FileSystemEntityType.link => l10n.projectExportParentLink,
    _ => l10n.projectExportParentRequired,
  };
}

String? validateRevision3ProjectImportFolderName(
  String? raw,
  AppLocalizations l10n,
) {
  final value = raw ?? '';
  if (value.isEmpty) return l10n.projectRestoreFolderNameRequired;
  if (utf8.encode(value).length > 128) {
    return l10n.projectRestoreFolderNameTooLong;
  }
  if (value == '.' ||
      value == '..' ||
      value.startsWith(' ') ||
      value.endsWith(' ') ||
      value.endsWith('.') ||
      value.contains('/') ||
      value.contains(r'\') ||
      value.contains(':') ||
      value.contains('<') ||
      value.contains('>') ||
      value.contains('"') ||
      value.contains('|') ||
      value.contains('?') ||
      value.contains('*') ||
      value.runes.any(
        (rune) => rune <= 0x1f || (rune >= 0x7f && rune <= 0x9f),
      )) {
    return l10n.projectRestoreFolderNameInvalid;
  }
  final deviceStem = value.split('.').first.toUpperCase();
  if (const {'CON', 'PRN', 'AUX', 'NUL'}.contains(deviceStem) ||
      RegExp(r'^(?:COM|LPT)[1-9¹²³]$').hasMatch(deviceStem)) {
    return l10n.projectRestoreFolderNameReserved;
  }
  return null;
}

class _ProjectRestoreNotice extends StatelessWidget {
  const _ProjectRestoreNotice({required this.copy});

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
            copy.projectRestoreNoticeTitle,
            style: Theme.of(context).textTheme.titleSmall,
          ),
          const SizedBox(height: 6),
          Text(copy.projectRestoreNoticeDescription),
          const SizedBox(height: 6),
          Text(copy.projectRestoreCapabilityBoundary),
        ],
      ),
    ),
  );
}

class _ProjectRestoreFact extends StatelessWidget {
  const _ProjectRestoreFact({
    required this.label,
    required this.value,
    this.valueKey,
  });

  final String label;
  final String value;
  final Key? valueKey;

  @override
  Widget build(BuildContext context) => Padding(
    padding: const EdgeInsets.only(bottom: 8),
    child: Row(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        SizedBox(
          width: 170,
          child: Text(label, style: Theme.of(context).textTheme.labelLarge),
        ),
        const SizedBox(width: 12),
        Expanded(child: SelectableText(value, key: valueKey)),
      ],
    ),
  );
}

class _ProjectRestoreMessage extends StatelessWidget {
  const _ProjectRestoreMessage({
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
