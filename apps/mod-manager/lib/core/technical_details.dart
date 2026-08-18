import 'package:flutter/material.dart';

import '../l10n/app_localizations.dart';
import 'diagnostic_text.dart';

const _technicalDetailLimit = 512;

String boundedTechnicalDetail(String raw) {
  final bounded = boundedDiagnosticText(raw, _technicalDetailLimit);
  return switch (bounded.value) {
    final value? => '$value${bounded.truncated ? '…' : ''}',
    null => '—',
  };
}

Future<void> showTechnicalDetailsDialog(
  BuildContext context,
  String raw,
) async {
  final l10n = AppLocalizations.of(context);
  final detail = boundedTechnicalDetail(raw);
  await showDialog<void>(
    context: context,
    builder: (dialogContext) => AlertDialog(
      key: const ValueKey('technical-details-dialog'),
      scrollable: true,
      title: Text(l10n.coreTechnicalDetails),
      content: ConstrainedBox(
        constraints: const BoxConstraints(maxWidth: 560),
        child: SelectableText(
          detail,
          key: const ValueKey('technical-details-content'),
        ),
      ),
      actions: [
        TextButton(
          key: const ValueKey('technical-details-close-action'),
          onPressed: () => Navigator.pop(dialogContext),
          child: Text(l10n.close),
        ),
      ],
    ),
  );
}

class TechnicalDetailsIconButton extends StatefulWidget {
  const TechnicalDetailsIconButton({super.key, required this.detail});

  final String detail;

  @override
  State<TechnicalDetailsIconButton> createState() =>
      _TechnicalDetailsIconButtonState();
}

class _TechnicalDetailsIconButtonState
    extends State<TechnicalDetailsIconButton> {
  late final FocusNode _focusNode = FocusNode(
    debugLabel: 'technical-details-action',
  );

  @override
  void initState() {
    super.initState();
    _focusNode.addListener(_handleFocusChange);
  }

  void _handleFocusChange() {
    if (mounted) setState(() {});
  }

  void _openTechnicalDetails() {
    showTechnicalDetailsDialog(context, widget.detail);
  }

  @override
  void dispose() {
    _focusNode.removeListener(_handleFocusChange);
    _focusNode.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context);
    return Semantics(
      container: true,
      label: l10n.coreTechnicalDetails,
      button: true,
      focusable: true,
      focused: _focusNode.hasFocus,
      onFocus: _focusNode.requestFocus,
      onTap: _openTechnicalDetails,
      child: ExcludeSemantics(
        child: IconButton(
          focusNode: _focusNode,
          visualDensity: VisualDensity.compact,
          tooltip: l10n.coreTechnicalDetails,
          onPressed: _openTechnicalDetails,
          icon: const Icon(Icons.info_outline),
        ),
      ),
    );
  }
}
