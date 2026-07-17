import 'package:flutter/material.dart';

import '../domain/reviewed_dataasset_schema.dart';
import 'dataasset_semantic_edit_panel.dart';
import 'installed_dataasset_semantic_edit_dialog.dart';

typedef ReviewedDataAssetStagePublisher =
    Future<DataAssetSemanticStagePublication> Function(
      ReviewedDataAssetEditRequest request,
    );

/// Guided editor for the reviewed `FeetTextureSize` field.
///
/// The callback receives only the closed semantic request. Installed target,
/// selector, source evidence, replacement bytes, and project authority remain
/// outside this widget and must be resolved again by the native transaction.
class ReviewedFootstepPresetDialog extends StatefulWidget {
  const ReviewedFootstepPresetDialog({
    required this.evidence,
    required this.publish,
    super.key,
  });

  final ReviewedFootstepPresetInspection evidence;
  final ReviewedDataAssetStagePublisher publish;

  @override
  State<ReviewedFootstepPresetDialog> createState() =>
      _ReviewedFootstepPresetDialogState();
}

class _ReviewedFootstepPresetDialogState
    extends State<ReviewedFootstepPresetDialog> {
  late final TextEditingController _xController;
  late final TextEditingController _yController;
  final _scrollController = ScrollController();
  late String _statusMessage;
  late bool _statusIsProblem;
  ReviewedDataAssetEditRequest? _preview;
  int? _selectedPreset;
  var _busy = false;
  var _epoch = 0;

  @override
  void initState() {
    super.initState();
    _xController = TextEditingController(text: widget.evidence.currentX);
    _yController = TextEditingController(text: widget.evidence.currentY);
    final evaluation = _evaluateDraft();
    _statusMessage = evaluation.message;
    _statusIsProblem = evaluation.isProblem;
  }

  @override
  void dispose() {
    _epoch++;
    _xController.dispose();
    _yController.dispose();
    _scrollController.dispose();
    super.dispose();
  }

  _DraftEvaluation _evaluateDraft() {
    final ReviewedDataAssetEditRequest request;
    try {
      request = ReviewedDataAssetEditRequest.feetTextureSize(
        x: _xController.text.trim(),
        y: _yController.text.trim(),
      );
    } on FormatException {
      return const _DraftEvaluation.problem(
        'Enter positive finite numbers for both X size and Y size.',
      );
    }

    final currentX = double.parse(widget.evidence.currentX);
    final currentY = double.parse(widget.evidence.currentY);
    if (double.parse(request.x) == currentX &&
        double.parse(request.y) == currentY) {
      return const _DraftEvaluation.problem(
        'Choose a size different from the current X and Y values.',
      );
    }
    return _DraftEvaluation.ready(request);
  }

  void _draftChanged({int? presetPercent}) {
    if (_busy) return;
    final evaluation = _evaluateDraft();
    setState(() {
      _selectedPreset = presetPercent;
      _preview = null;
      _statusMessage = evaluation.message;
      _statusIsProblem = evaluation.isProblem;
    });
  }

  void _applyPreset(int percent) {
    if (_busy) return;
    _setControllerText(
      _xController,
      _scaledDecimal(widget.evidence.currentX, percent),
    );
    _setControllerText(
      _yController,
      _scaledDecimal(widget.evidence.currentY, percent),
    );
    _draftChanged(presetPercent: percent);
  }

  void _previewDraft() {
    if (_busy) return;
    final evaluation = _evaluateDraft();
    setState(() {
      _preview = evaluation.request;
      _statusIsProblem = evaluation.isProblem;
      _statusMessage = evaluation.request == null
          ? evaluation.message
          : 'Preview ready. Review the before and after values before staging.';
    });
  }

  Future<void> _stage() async {
    final request = _preview;
    if (_busy || request == null) return;
    final epoch = ++_epoch;
    if (_scrollController.hasClients) _scrollController.jumpTo(0);
    setState(() => _busy = true);
    try {
      final publication = await widget.publish(request);
      if (!mounted || epoch != _epoch) return;
      Navigator.of(
        context,
      ).pop(InstalledDataAssetSemanticEditResult.published(publication));
    } on DataAssetSemanticStageUnavailableException catch (error) {
      if (!mounted || epoch != _epoch) return;
      Navigator.of(
        context,
      ).pop(InstalledDataAssetSemanticEditResult.unavailable(error));
    } catch (_) {
      if (!mounted || epoch != _epoch) return;
      Navigator.of(context).pop(
        const InstalledDataAssetSemanticEditResult.unavailable(
          DataAssetSemanticStageUnavailableException.unknownOutcome(),
        ),
      );
    }
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final preview = _preview;
    return PopScope<void>(
      canPop: !_busy,
      child: AlertDialog(
        key: const Key('reviewed-footstep-preset-dialog'),
        title: const Text('Edit footstep preset'),
        content: ConstrainedBox(
          constraints: const BoxConstraints(maxWidth: 680),
          child: SingleChildScrollView(
            controller: _scrollController,
            child: Column(
              mainAxisSize: MainAxisSize.min,
              crossAxisAlignment: CrossAxisAlignment.stretch,
              children: [
                if (_busy) ...[
                  Semantics(
                    key: const Key('reviewed-footstep-busy-status'),
                    container: true,
                    liveRegion: true,
                    label:
                        'Rechecking the installed asset and staging the reviewed edit',
                    child: const ExcludeSemantics(
                      child: Column(
                        crossAxisAlignment: CrossAxisAlignment.stretch,
                        children: [
                          LinearProgressIndicator(
                            key: Key('reviewed-footstep-progress'),
                            semanticsLabel: 'Staging reviewed DataAsset edit',
                          ),
                          SizedBox(height: 6),
                          Text(
                            'Rechecking the installed asset and staging the reviewed edit…',
                          ),
                        ],
                      ),
                    ),
                  ),
                  const SizedBox(height: 12),
                ],
                Card(
                  color: theme.colorScheme.secondaryContainer,
                  child: ListTile(
                    leading: const Icon(Icons.pets_outlined),
                    title: Text(widget.evidence.target.friendlyName),
                    subtitle: const Text('Reviewed footstep preset'),
                  ),
                ),
                const SizedBox(height: 10),
                const Wrap(
                  spacing: 8,
                  runSpacing: 6,
                  children: [
                    Chip(
                      key: Key('reviewed-footstep-badge-structure'),
                      avatar: Icon(Icons.verified_outlined, size: 18),
                      label: Text('Reviewed structure'),
                    ),
                    Chip(
                      key: Key('reviewed-footstep-badge-build'),
                      avatar: Icon(Icons.inventory_2_outlined, size: 18),
                      label: Text('Offline build available after saving'),
                    ),
                    Chip(
                      key: Key('reviewed-footstep-badge-runtime'),
                      avatar: Icon(Icons.science_outlined, size: 18),
                      label: Text('Gameplay/runtime unverified'),
                    ),
                    Chip(
                      key: Key('reviewed-footstep-badge-deployment'),
                      avatar: Icon(Icons.do_not_disturb_alt_outlined, size: 18),
                      label: Text('Deployment unverified'),
                    ),
                  ],
                ),
                const SizedBox(height: 16),
                Text(
                  'Footprint texture size',
                  style: theme.textTheme.titleMedium,
                ),
                const SizedBox(height: 3),
                const Text(
                  'raw asset units — gameplay meaning not yet qualified',
                  key: Key('reviewed-footstep-unit-note'),
                ),
                const SizedBox(height: 12),
                if (MediaQuery.sizeOf(context).width < 600)
                  Column(
                    children: [
                      _SizeField(
                        fieldKey: const Key('reviewed-footstep-x'),
                        label: 'X size',
                        controller: _xController,
                        enabled: !_busy,
                        onChanged: (_) => _draftChanged(),
                      ),
                      const SizedBox(height: 10),
                      _SizeField(
                        fieldKey: const Key('reviewed-footstep-y'),
                        label: 'Y size',
                        controller: _yController,
                        enabled: !_busy,
                        onChanged: (_) => _draftChanged(),
                      ),
                    ],
                  )
                else
                  Row(
                    children: [
                      Expanded(
                        child: _SizeField(
                          fieldKey: const Key('reviewed-footstep-x'),
                          label: 'X size',
                          controller: _xController,
                          enabled: !_busy,
                          onChanged: (_) => _draftChanged(),
                        ),
                      ),
                      const SizedBox(width: 12),
                      Expanded(
                        child: _SizeField(
                          fieldKey: const Key('reviewed-footstep-y'),
                          label: 'Y size',
                          controller: _yController,
                          enabled: !_busy,
                          onChanged: (_) => _draftChanged(),
                        ),
                      ),
                    ],
                  ),
                const SizedBox(height: 10),
                Semantics(
                  key: const Key('reviewed-footstep-live-status'),
                  container: true,
                  liveRegion: true,
                  label: _statusMessage,
                  child: ExcludeSemantics(
                    child: Text(
                      _statusMessage,
                      key: const Key('reviewed-footstep-status'),
                      style: TextStyle(
                        color: _statusIsProblem
                            ? theme.colorScheme.error
                            : theme.colorScheme.primary,
                      ),
                    ),
                  ),
                ),
                const SizedBox(height: 12),
                Text(
                  'Scale from current size',
                  style: theme.textTheme.labelLarge,
                ),
                const SizedBox(height: 6),
                Wrap(
                  spacing: 8,
                  runSpacing: 6,
                  children: [
                    for (final percent in const <int>[50, 100, 150, 200])
                      ChoiceChip(
                        key: Key('reviewed-footstep-preset-$percent'),
                        label: Text('$percent%'),
                        selected: _selectedPreset == percent,
                        onSelected: _busy ? null : (_) => _applyPreset(percent),
                      ),
                  ],
                ),
                const SizedBox(height: 12),
                Card(
                  child: Padding(
                    padding: const EdgeInsets.all(12),
                    child: Text(
                      'Preserved technical components: '
                      'Z ${widget.evidence.currentZ} · W ${widget.evidence.currentW}. '
                      'They are carried forward unchanged and are not editable here.',
                      key: const Key('reviewed-footstep-preserved-components'),
                    ),
                  ),
                ),
                if (preview != null) ...[
                  const SizedBox(height: 12),
                  Card(
                    key: const Key('reviewed-footstep-before-after'),
                    child: ListTile(
                      leading: const Icon(Icons.compare_arrows_outlined),
                      title: const Text('Before and after'),
                      subtitle: Text(
                        'X ${widget.evidence.currentX} → ${preview.x}\n'
                        'Y ${widget.evidence.currentY} → ${preview.y}\n'
                        'Z ${widget.evidence.currentZ} and W ${widget.evidence.currentW} stay unchanged.',
                      ),
                    ),
                  ),
                ],
              ],
            ),
          ),
        ),
        actions: [
          TextButton(
            onPressed: _busy ? null : () => Navigator.of(context).pop(),
            child: const Text('Cancel'),
          ),
          OutlinedButton(
            key: const Key('reviewed-footstep-preview'),
            onPressed: _busy ? null : _previewDraft,
            child: const Text('Preview'),
          ),
          FilledButton.icon(
            key: const Key('reviewed-footstep-stage'),
            onPressed: _busy || preview == null ? null : _stage,
            icon: const Icon(Icons.add_task_outlined),
            label: const Text('Stage edit'),
          ),
        ],
      ),
    );
  }
}

final class _DraftEvaluation {
  const _DraftEvaluation.problem(this.message)
    : request = null,
      isProblem = true;

  const _DraftEvaluation.ready(this.request)
    : message = 'Values are valid. Preview the change before staging.',
      isProblem = false;

  final ReviewedDataAssetEditRequest? request;
  final String message;
  final bool isProblem;
}

class _SizeField extends StatelessWidget {
  const _SizeField({
    required this.fieldKey,
    required this.label,
    required this.controller,
    required this.enabled,
    required this.onChanged,
  });

  final Key fieldKey;
  final String label;
  final TextEditingController controller;
  final bool enabled;
  final ValueChanged<String> onChanged;

  @override
  Widget build(BuildContext context) => TextField(
    key: fieldKey,
    controller: controller,
    enabled: enabled,
    keyboardType: const TextInputType.numberWithOptions(decimal: true),
    textInputAction: TextInputAction.next,
    decoration: InputDecoration(
      labelText: label,
      border: const OutlineInputBorder(),
    ),
    onChanged: onChanged,
  );
}

void _setControllerText(TextEditingController controller, String value) {
  controller.value = TextEditingValue(
    text: value,
    selection: TextSelection.collapsed(offset: value.length),
  );
}

String _scaledDecimal(String current, int percent) {
  final scaled = double.parse(current) * percent / 100;
  if (!scaled.isFinite || scaled <= 0) return '';
  var value = scaled.toStringAsFixed(12);
  while (value.endsWith('0')) {
    value = value.substring(0, value.length - 1);
  }
  if (value.endsWith('.')) value = value.substring(0, value.length - 1);
  return value;
}
