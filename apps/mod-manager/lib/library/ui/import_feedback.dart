import 'package:flutter/material.dart';

import '../../core/diagnostic_text.dart';
import '../../core/mgr_ffi.dart';
import '../../l10n/app_localizations.dart';
import '../domain/library_notifier.dart';
import '../domain/models.dart';

const _maxImportDisplayNameRunes = 160;
const _maxImportErrorMessageRunes = 512;
const _maxImportTechnicalRunes = 1024;
const _maxImportCandidateIdRunes = 256;

String importDisplayName(ModEntryMetaView entry) {
  final preferred = boundedDiagnosticText(
    entry.name.isEmpty ? entry.id : entry.name,
    _maxImportDisplayNameRunes,
  );
  if (preferred.value case final value?) {
    return '$value${preferred.truncated ? '…' : ''}';
  }
  final fallback = boundedDiagnosticText(entry.id, _maxImportDisplayNameRunes);
  return switch (fallback.value) {
    final value? => '$value${fallback.truncated ? '…' : ''}',
    null => '—',
  };
}

/// Confirms one import. [showMatchReason] appends why Native matched (or did
/// not match) an existing entry — true only while advanced details are on,
/// since the plain confirmation already says what happened.
void showImportSuccessFeedback(
  BuildContext context,
  MgrImportOutcome outcome, {
  bool showMatchReason = false,
}) {
  final l10n = AppLocalizations.of(context);
  final name = importDisplayName(outcome.entry);
  final disposition = switch (outcome.disposition) {
    MgrImportDisposition.created => l10n.importOutcomeCreated(name),
    MgrImportDisposition.updated => l10n.importOutcomeUpdated(name),
    MgrImportDisposition.unchanged => l10n.importOutcomeUnchanged(name),
  };
  _showImportSnackBar(
    context,
    message: showMatchReason
        ? '$disposition '
              '${l10n.importOutcomeMatchedBy(outcome.matchedBy.wireName)}'
        : disposition,
  );
}

void showImportFailureFeedback(
  BuildContext context,
  MgrFfiException error,
  LibraryState library,
) {
  final l10n = AppLocalizations.of(context);
  final message = switch (error.code) {
    // Picker failures happen before Native receives an import request, so they
    // must never imply a bad source or possible library publication.
    'IMPORT_PICKER_FAILED' => l10n.importPickerFailed,
    _ when !library.authoritative => l10n.importOutcomeUnknown,
    'IMPORT_INVALID_RESPONSE' => l10n.importOutcomeUnknown,
    'IMPORT_DUPLICATE_AMBIGUOUS' => l10n.importRefusalDuplicateAmbiguous,
    'IMPORT_IDENTITY_CONFLICT' => l10n.importRefusalIdentityConflict,
    _ => l10n.importFailed,
  };
  _showImportSnackBar(
    context,
    message: message,
    technicalDetails: _importTechnicalDetails(error, library),
  );
}

String? _importTechnicalDetails(MgrFfiException error, LibraryState library) {
  final lines = <String>[];
  // Put the bounded machine-readable witnesses first. With Native's two-item
  // candidate cap, both complete candidate ids, display names, and match roles
  // fit inside the overall budget; opaque native prose may use only what
  // remains after those facts.
  if (error.details case final MgrImportRefusalDetails details) {
    for (final candidate in details.candidates) {
      final safeId = boundedDiagnosticText(
        candidate.id,
        _maxImportCandidateIdRunes,
      );
      final id = switch (safeId.value) {
        final value? => '$value${safeId.truncated ? '…' : ''}',
        null => '—',
      };
      final entry = library.modById(candidate.id);
      final name = entry == null ? null : importDisplayName(entry);
      final roles = candidate.matchedBy.map((role) => role.wireName).join(', ');
      final identity = name == null || name == id ? id : '$name ($id)';
      lines.add(roles.isEmpty ? identity : '$identity — $roles');
    }
  }

  final diagnostic = boundedDiagnosticText(
    error.message,
    _maxImportErrorMessageRunes,
  );
  if (diagnostic.value case final value?) {
    lines.add('$value${diagnostic.truncated ? '…' : ''}');
  }

  if (lines.isEmpty) return null;
  final combined = boundedDiagnosticText(
    lines.join('\n'),
    _maxImportTechnicalRunes,
  );
  return switch (combined.value) {
    final value? => '$value${combined.truncated ? '…' : ''}',
    null => null,
  };
}

void _showImportSnackBar(
  BuildContext context, {
  required String message,
  String? technicalDetails,
}) {
  final messenger = ScaffoldMessenger.of(context);
  messenger.hideCurrentSnackBar();
  messenger.showSnackBar(
    SnackBar(
      duration: technicalDetails == null
          ? const Duration(seconds: 5)
          : const Duration(seconds: 30),
      // Technical failures contain an interactive expandable diagnostic. Keep
      // it available until explicitly dismissed, including for assistive
      // navigation; [duration] remains useful only for non-technical feedback.
      persist: technicalDetails != null,
      showCloseIcon: true,
      content: ImportFeedbackContent(
        message: message,
        technicalDetails: technicalDetails,
      ),
    ),
  );
}

class ImportFeedbackContent extends StatefulWidget {
  const ImportFeedbackContent({
    super.key,
    required this.message,
    this.technicalDetails,
  });

  final String message;
  final String? technicalDetails;

  @override
  State<ImportFeedbackContent> createState() => _ImportFeedbackContentState();
}

class _ImportFeedbackContentState extends State<ImportFeedbackContent> {
  bool _expanded = false;
  late final FocusNode _detailsFocusNode = FocusNode(
    debugLabel: 'import-feedback-technical-details',
  );

  @override
  void dispose() {
    _detailsFocusNode.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final technical = widget.technicalDetails;
    final maxContentHeight = (MediaQuery.sizeOf(context).height * 0.6)
        .clamp(0.0, 360.0)
        .toDouble();
    return ConstrainedBox(
      key: const ValueKey('import-feedback-content'),
      constraints: BoxConstraints(maxHeight: maxContentHeight),
      child: Column(
        mainAxisSize: MainAxisSize.min,
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Flexible(
            child: SingleChildScrollView(
              key: const ValueKey('import-feedback-message-scroll'),
              child: Text(
                key: const ValueKey('import-feedback-message'),
                widget.message,
              ),
            ),
          ),
          if (technical != null) ...[
            MergeSemantics(
              child: Semantics(
                expanded: _expanded,
                child: TextButton(
                  key: const ValueKey('import-feedback-details-toggle'),
                  focusNode: _detailsFocusNode,
                  onPressed: () => setState(() => _expanded = !_expanded),
                  style: TextButton.styleFrom(
                    foregroundColor: Theme.of(
                      context,
                    ).colorScheme.inversePrimary,
                    padding: const EdgeInsets.symmetric(
                      horizontal: 0,
                      vertical: 4,
                    ),
                    minimumSize: const Size(44, 44),
                  ),
                  child: Text(
                    AppLocalizations.of(context).coreTechnicalDetails,
                  ),
                ),
              ),
            ),
            if (_expanded)
              ConstrainedBox(
                constraints: const BoxConstraints(maxHeight: 72),
                child: SingleChildScrollView(
                  key: const ValueKey('import-feedback-details-scroll'),
                  child: SelectableText(
                    technical,
                    key: const ValueKey('import-feedback-details'),
                  ),
                ),
              ),
          ],
        ],
      ),
    );
  }
}
