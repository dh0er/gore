import 'package:flutter/material.dart';

import '../../core/mod_ffi.dart';
import 'dataasset_semantic_edit_panel.dart';

typedef InstalledDataAssetSemanticStagePublisher =
    Future<DataAssetSemanticStagePublication> Function(
      DataAssetInstalledSemanticEditIntent intent,
    );

final class InstalledDataAssetSemanticEditResult {
  const InstalledDataAssetSemanticEditResult.published(this.publication)
    : unavailable = null;

  const InstalledDataAssetSemanticEditResult.unavailable(this.unavailable)
    : publication = null;

  final DataAssetSemanticStagePublication? publication;
  final DataAssetSemanticStageUnavailableException? unavailable;
}

/// Typed editor for one value whose selector and source facts came from the
/// exact installed-package inspection currently on screen.
class InstalledDataAssetSemanticEditDialog extends StatefulWidget {
  const InstalledDataAssetSemanticEditDialog({
    required this.snapshot,
    required this.candidate,
    required this.inspection,
    required this.leaf,
    required this.publish,
    super.key,
  });

  final AuthoringRevision3DataAssetPackageIndexResult snapshot;
  final AuthoringRevision3DataAssetPackageCandidate candidate;
  final AuthoringRevision3InstalledDataAssetInspectionResult inspection;
  final DataAssetLeafReport leaf;
  final InstalledDataAssetSemanticStagePublisher publish;

  @override
  State<InstalledDataAssetSemanticEditDialog> createState() =>
      _InstalledDataAssetSemanticEditDialogState();
}

class _InstalledDataAssetSemanticEditDialogState
    extends State<InstalledDataAssetSemanticEditDialog> {
  late final DataAssetSemanticValueEditor _editor;
  final _scalarController = TextEditingController();
  final _componentControllers = List<TextEditingController>.generate(
    4,
    (_) => TextEditingController(),
  );
  var _boolValue = false;
  DataAssetSemanticValueChange? _change;
  DataAssetInstalledSemanticEditIntent? _intent;
  String? _error;
  var _busy = false;
  var _epoch = 0;

  @override
  void initState() {
    super.initState();
    _editor = DataAssetSemanticValueEditor.fromLeaf(widget.leaf);
    if (_editor.isBoolean) {
      _boolValue = _editor.initialScalarValue == 'On';
    } else if (_editor.isComposite) {
      final values = _editor.initialComponentValues;
      for (var index = 0; index < values.length; index++) {
        _componentControllers[index].text = values[index];
      }
    } else {
      _scalarController.text = _editor.initialScalarValue;
    }
  }

  @override
  void dispose() {
    _epoch++;
    _scalarController.dispose();
    for (final controller in _componentControllers) {
      controller.dispose();
    }
    super.dispose();
  }

  void _invalidate() {
    if (_busy) return;
    setState(() {
      _change = null;
      _intent = null;
      _error = null;
    });
  }

  void _preview() {
    if (_busy) return;
    try {
      final change = _editor.isBoolean
          ? _editor.changeBool(value: _boolValue)
          : _editor.isComposite
          ? _editor.changeComponents(
              values: _componentControllers
                  .map((controller) => controller.text)
                  .toList(growable: false),
            )
          : _editor.changeScalar(value: _scalarController.text);
      final intent = DataAssetInstalledSemanticEditIntent.fromInspection(
        snapshot: widget.snapshot,
        candidate: widget.candidate,
        inspection: widget.inspection,
        change: change,
      );
      setState(() {
        _change = change;
        _intent = intent;
        _error = null;
      });
    } on DataAssetSemanticEditException catch (error) {
      setState(() {
        _change = null;
        _intent = null;
        _error = error.message;
      });
    } on ArgumentError {
      setState(() {
        _change = null;
        _intent = null;
        _error =
            'This inspection is no longer bound to the exact installed snapshot. Close it and inspect again.';
      });
    }
  }

  Future<void> _stage() async {
    final intent = _intent;
    if (_busy || intent == null) return;
    final epoch = ++_epoch;
    setState(() {
      _busy = true;
      _error = null;
    });
    try {
      final publication = await widget.publish(intent);
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
    final change = _change;
    final theme = Theme.of(context);
    return PopScope(
      canPop: !_busy,
      child: AlertDialog(
        key: const Key('installed-dataasset-semantic-edit-dialog'),
        title: const Text('Edit proven installed value'),
        content: ConstrainedBox(
          constraints: const BoxConstraints(maxWidth: 680),
          child: SingleChildScrollView(
            child: Column(
              mainAxisSize: MainAxisSize.min,
              crossAxisAlignment: CrossAxisAlignment.stretch,
              children: [
                if (_error != null) ...[
                  Semantics(
                    key: const Key('installed-dataasset-semantic-error-status'),
                    container: true,
                    liveRegion: true,
                    label: _error!,
                    child: ExcludeSemantics(
                      child: Text(
                        _error!,
                        key: const Key('installed-dataasset-semantic-error'),
                        style: TextStyle(color: theme.colorScheme.error),
                      ),
                    ),
                  ),
                  const SizedBox(height: 10),
                ],
                if (_busy) ...[
                  Semantics(
                    key: const Key('installed-dataasset-semantic-busy-status'),
                    container: true,
                    liveRegion: true,
                    label:
                        'Re-reading the exact package and preparing the managed candidate',
                    child: const ExcludeSemantics(
                      child: Column(
                        crossAxisAlignment: CrossAxisAlignment.stretch,
                        children: [
                          LinearProgressIndicator(
                            key: Key('installed-dataasset-semantic-progress'),
                            semanticsLabel:
                                'Preparing installed DataAsset edit',
                          ),
                          SizedBox(height: 6),
                          Text(
                            'Re-reading the exact package and preparing the managed candidate…',
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
                    leading: const Icon(Icons.verified_user_outlined),
                    title: Text(widget.inspection.targetPath),
                    subtitle: const Text(
                      'The source path, package, USMAP, selector, and project head are fixed by the inspection. The result is staged only in this project; build and runtime remain blocked.',
                    ),
                  ),
                ),
                const SizedBox(height: 8),
                Text(
                  _editor.selector.pathLabel,
                  style: theme.textTheme.titleSmall,
                ),
                const SizedBox(height: 2),
                Text(
                  '${_editor.typeLabel} · current ${_editor.initialScalarValue}',
                ),
                const SizedBox(height: 14),
                DataAssetSemanticValueFields(
                  editor: _editor,
                  scalarController: _scalarController,
                  componentControllers: _componentControllers,
                  boolValue: _boolValue,
                  enabled: !_busy,
                  onBoolChanged: (value) {
                    setState(() => _boolValue = value);
                    _invalidate();
                  },
                  onChanged: _invalidate,
                ),
                if (change != null) ...[
                  const SizedBox(height: 12),
                  Card(
                    key: const Key('installed-dataasset-semantic-preview'),
                    child: ListTile(
                      leading: const Icon(Icons.compare_arrows_outlined),
                      title: Text(
                        '${change.previousValue} → ${change.replacementValue}',
                      ),
                      subtitle: Text(
                        '${change.pathLabel} · ${change.typeLabel}\n'
                        'Native code will re-read and compare the installed bytes before staging.',
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
            key: const Key('installed-dataasset-semantic-preview-action'),
            onPressed: _busy ? null : _preview,
            child: const Text('Preview'),
          ),
          FilledButton.icon(
            key: const Key('installed-dataasset-semantic-stage-action'),
            onPressed: _busy || _intent == null ? null : _stage,
            icon: const Icon(Icons.add_task_outlined),
            label: const Text('Stage edit'),
          ),
        ],
      ),
    );
  }
}
