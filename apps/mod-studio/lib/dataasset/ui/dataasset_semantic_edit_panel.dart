import 'package:file_selector/file_selector.dart';
import 'package:flutter/material.dart';

import '../domain/dataasset_inspection.dart';
import '../domain/dataasset_semantic_edit.dart';

typedef DataAssetExtractReceiptPicker = Future<String?> Function();
typedef DataAssetExtractReceiptInspector =
    Future<DataAssetExtractReceiptSummary> Function(String path);
typedef DataAssetSemanticStagePublisher =
    Future<DataAssetSemanticStagePublication> Function(
      DataAssetSemanticEditIntent intent,
    );

enum DataAssetSemanticStageUnavailableReason { staleCheckpoint, requiresReopen }

/// A managed-project failure that cannot be corrected inside the currently
/// open value wizard. Receipt/selector mismatches deliberately do not use this
/// type and remain retryable after the user reinspects or chooses another
/// proof.
final class DataAssetSemanticStageUnavailableException implements Exception {
  const DataAssetSemanticStageUnavailableException.staleCheckpoint()
    : reason = DataAssetSemanticStageUnavailableReason.staleCheckpoint,
      message =
          'The project changed. Reopen the value editor from the refreshed DataAsset list.';

  const DataAssetSemanticStageUnavailableException.requiresReopen()
    : reason = DataAssetSemanticStageUnavailableReason.requiresReopen,
      message =
          'Reopen the managed project before creating another DataAsset edit.';

  final DataAssetSemanticStageUnavailableReason reason;
  final String message;

  @override
  String toString() => message;
}

final class DataAssetSemanticStagePublication {
  const DataAssetSemanticStagePublication({
    required this.targetPath,
    required this.revision,
  });

  final String targetPath;
  final int revision;
}

/// Guided value-only editor for leaves proven by `dataasset_fixed_inspect_v1`.
///
/// It previews a friendly semantic diff and delegates one ExtractReceipt-bound
/// exact-head R3 stage transaction. The panel never exposes offsets, raw wire
/// values, package output paths, build, deployment, or runtime controls.
class DataAssetSemanticEditPanel extends StatefulWidget {
  const DataAssetSemanticEditPanel({
    super.key,
    required this.inspection,
    required this.publish,
    required this.extractReceiptInspector,
    this.extractReceiptPicker,
    this.initialExtractReceiptPath,
    this.onPublished,
    this.onUnavailable,
  });

  final DataAssetInspection inspection;
  final DataAssetSemanticStagePublisher publish;
  final DataAssetExtractReceiptInspector extractReceiptInspector;
  final DataAssetExtractReceiptPicker? extractReceiptPicker;
  final String? initialExtractReceiptPath;
  final ValueChanged<DataAssetSemanticStagePublication>? onPublished;
  final ValueChanged<DataAssetSemanticStageUnavailableException>? onUnavailable;

  @override
  State<DataAssetSemanticEditPanel> createState() =>
      _DataAssetSemanticEditPanelState();
}

class _DataAssetSemanticEditPanelState
    extends State<DataAssetSemanticEditPanel> {
  final _searchController = TextEditingController();
  final _scalarController = TextEditingController();
  final _componentControllers = List.generate(
    4,
    (_) => TextEditingController(),
  );
  late List<_EditableLeafChoice> _choices;
  _EditableLeafChoice? _selected;
  String? _extractReceiptPath;
  DataAssetExtractReceiptSummary? _receiptSummary;
  bool _targetConfirmed = false;
  bool _boolValue = false;
  bool _busy = false;
  int _epoch = 0;
  String? _receiptError;
  String? _error;
  DataAssetSemanticEditPreview? _preview;
  DataAssetSemanticStagePublication? _publication;

  @override
  void initState() {
    super.initState();
    _adoptInspection();
    final initialPath = widget.initialExtractReceiptPath;
    if (initialPath != null) {
      WidgetsBinding.instance.addPostFrameCallback((_) {
        if (mounted) _verifyExtractReceipt(initialPath);
      });
    }
  }

  @override
  void didUpdateWidget(covariant DataAssetSemanticEditPanel oldWidget) {
    super.didUpdateWidget(oldWidget);
    final inspectionChanged = !identical(
      oldWidget.inspection,
      widget.inspection,
    );
    if (inspectionChanged) {
      _epoch++;
      _adoptInspection();
    }
    final initialPath = widget.initialExtractReceiptPath;
    if (initialPath != null &&
        (inspectionChanged ||
            oldWidget.initialExtractReceiptPath != initialPath)) {
      WidgetsBinding.instance.addPostFrameCallback((_) {
        if (mounted) _verifyExtractReceipt(initialPath);
      });
    }
  }

  @override
  void dispose() {
    _epoch++;
    _searchController.dispose();
    _scalarController.dispose();
    for (final controller in _componentControllers) {
      controller.dispose();
    }
    super.dispose();
  }

  void _adoptInspection() {
    _choices = <_EditableLeafChoice>[
      for (final export in widget.inspection.exports)
        for (final leaf in export.leaves)
          if (leaf.editable) _EditableLeafChoice(export: export, leaf: leaf),
    ];
    _selected = _choices.isEmpty ? null : _choices.first;
    _searchController.clear();
    _loadSelectedValue();
    _preview = null;
    _publication = null;
    _extractReceiptPath = null;
    _receiptSummary = null;
    _targetConfirmed = false;
    _receiptError = null;
    _error = null;
    _busy = false;
  }

  DataAssetSemanticValueEditor? get _editor {
    final selected = _selected;
    return selected == null
        ? null
        : DataAssetSemanticValueEditor.fromLeaf(selected.leaf);
  }

  List<_EditableLeafChoice> get _visibleChoices {
    final query = _searchController.text.trim().toLowerCase();
    if (query.isEmpty) return _choices;
    return _choices
        .where(
          (choice) =>
              choice.export.objectName.toLowerCase().contains(query) ||
              choice.export.classPath.toLowerCase().contains(query) ||
              choice.leaf.selector.pathLabel.toLowerCase().contains(query) ||
              choice.leaf.selector.kind.wireName.contains(query),
        )
        .toList(growable: false);
  }

  void _loadSelectedValue() {
    final editor = _editor;
    if (editor == null) {
      _scalarController.clear();
      for (final controller in _componentControllers) {
        controller.clear();
      }
      return;
    }
    if (editor.isBoolean) {
      _boolValue = editor.initialScalarValue == 'On';
    } else if (editor.isComposite) {
      final values = editor.initialComponentValues;
      for (var index = 0; index < _componentControllers.length; index++) {
        _componentControllers[index].text = values[index];
      }
    } else {
      _scalarController.text = editor.initialScalarValue;
    }
  }

  void _select(_EditableLeafChoice choice) {
    if (_busy || identical(choice, _selected)) return;
    setState(() {
      _selected = choice;
      _loadSelectedValue();
      _invalidatePreview();
    });
  }

  void _invalidatePreview() {
    _epoch++;
    _preview = null;
    _publication = null;
    _error = null;
  }

  Future<void> _pickExtractReceipt() async {
    if (_busy) return;
    final epoch = ++_epoch;
    setState(() {
      _busy = true;
      _receiptError = null;
      _error = null;
    });
    try {
      final path =
          await (widget.extractReceiptPicker ?? _pickExtractReceiptFile)();
      if (!mounted || epoch != _epoch) return;
      if (path == null) {
        setState(() => _busy = false);
        return;
      }
      final summary = await widget.extractReceiptInspector(path);
      if (!mounted || epoch != _epoch) return;
      _adoptVerifiedReceipt(path, summary);
    } on DataAssetSemanticEditException catch (error) {
      if (!mounted || epoch != _epoch) return;
      setState(() {
        _busy = false;
        _receiptError = error.message;
      });
    } catch (_) {
      if (!mounted || epoch != _epoch) return;
      setState(() {
        _busy = false;
        _receiptError =
            'The extraction proof could not be opened or verified. Choose it again or select another ExtractReceipt-v2.';
      });
    }
  }

  Future<void> _verifyExtractReceipt(String path) async {
    if (_busy) return;
    final epoch = ++_epoch;
    setState(() {
      _busy = true;
      _receiptError = null;
      _error = null;
    });
    try {
      final summary = await widget.extractReceiptInspector(path);
      if (!mounted || epoch != _epoch) return;
      _adoptVerifiedReceipt(path, summary);
    } catch (_) {
      if (!mounted || epoch != _epoch) return;
      setState(() {
        _busy = false;
        _receiptError =
            'The initial extraction proof could not be opened or verified. Choose the exact ExtractReceipt-v2 again.';
      });
    }
  }

  void _adoptVerifiedReceipt(
    String path,
    DataAssetExtractReceiptSummary summary,
  ) {
    if (!summary.matchesInspection(widget.inspection)) {
      throw const DataAssetSemanticEditException(
        'This extraction proof does not match the inspected package or USMAP bytes. Inspect the package copied by the same ExtractReceipt-v2.',
      );
    }
    setState(() {
      _busy = false;
      _extractReceiptPath = path;
      _receiptSummary = summary;
      _targetConfirmed = false;
      _preview = null;
      _publication = null;
      _receiptError = null;
      _error = null;
    });
  }

  void _buildPreview() {
    final editor = _editor;
    final receipt = _extractReceiptPath;
    final receiptSummary = _receiptSummary;
    if (_busy || editor == null) return;
    if (receipt == null || receiptSummary == null || !_targetConfirmed) {
      setState(() {
        _preview = null;
        _publication = null;
        _error =
            'Verify the extraction proof and confirm its exact /Game target before previewing.';
      });
      return;
    }
    try {
      final preview = editor.isBoolean
          ? editor.previewBool(
              extractReceiptPath: receipt,
              expectedTargetPath: receiptSummary.targetPath,
              value: _boolValue,
            )
          : editor.isComposite
          ? editor.previewComponents(
              extractReceiptPath: receipt,
              expectedTargetPath: receiptSummary.targetPath,
              values: _componentControllers
                  .map((controller) => controller.text)
                  .toList(growable: false),
            )
          : editor.previewScalar(
              extractReceiptPath: receipt,
              expectedTargetPath: receiptSummary.targetPath,
              value: _scalarController.text,
            );
      setState(() {
        _preview = preview;
        _publication = null;
        _error = null;
      });
    } on DataAssetSemanticEditException catch (error) {
      setState(() {
        _preview = null;
        _publication = null;
        _error = error.message;
      });
    }
  }

  Future<void> _stage() async {
    final preview = _preview;
    if (_busy || preview == null || _publication != null) return;
    final epoch = ++_epoch;
    setState(() {
      _busy = true;
      _error = null;
    });
    try {
      final publication = await widget.publish(preview.intent);
      if (!mounted || epoch != _epoch) return;
      setState(() {
        _busy = false;
        _publication = publication;
      });
      widget.onPublished?.call(publication);
    } on DataAssetSemanticStageUnavailableException catch (error) {
      if (!mounted || epoch != _epoch) return;
      setState(() {
        _busy = false;
        _error = error.message;
      });
      widget.onUnavailable?.call(error);
    } catch (_) {
      if (!mounted || epoch != _epoch) return;
      setState(() {
        _busy = false;
        _error =
            'The verified edit could not be staged. Reopen the managed project '
            'if Studio reports head or integrity drift.';
      });
    }
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final editor = _editor;
    final visible = _visibleChoices;
    return ListView(
      key: const Key('dataasset-semantic-editor'),
      padding: const EdgeInsets.all(16),
      children: [
        Card(
          color: theme.colorScheme.surfaceContainerHighest,
          child: const ListTile(
            leading: Icon(Icons.verified_outlined),
            title: Text('Verified value edit'),
            subtitle: Text(
              'Choose a fixed value proven by the offline inspector. Preview '
              'the semantic change, then stage it in the managed project. '
              'Build, deployment, and runtime support remain blocked.',
            ),
          ),
        ),
        const SizedBox(height: 12),
        Row(
          children: [
            FilledButton.tonalIcon(
              key: const Key('dataasset-semantic-pick-receipt'),
              onPressed: _busy ? null : _pickExtractReceipt,
              icon: const Icon(Icons.receipt_long_outlined),
              label: const Text('Choose extraction proof'),
            ),
            const SizedBox(width: 12),
            Expanded(
              child: Text(
                _extractReceiptPath == null
                    ? 'No ExtractReceipt-v2 selected'
                    : _extractReceiptPath!,
                key: const Key('dataasset-semantic-receipt-path'),
                maxLines: 1,
                overflow: TextOverflow.ellipsis,
              ),
            ),
          ],
        ),
        if (_receiptSummary case final summary?) ...[
          const SizedBox(height: 8),
          Card(
            key: const Key('dataasset-semantic-receipt-target'),
            child: Column(
              children: [
                ListTile(
                  leading: const Icon(Icons.gps_fixed),
                  title: const Text('Exact in-game target from the proof'),
                  subtitle: Text(summary.targetPath),
                ),
                CheckboxListTile(
                  key: const Key('dataasset-semantic-confirm-target'),
                  value: _targetConfirmed,
                  onChanged: _busy
                      ? null
                      : (value) => setState(() {
                          _targetConfirmed = value ?? false;
                          _invalidatePreview();
                        }),
                  title: Text(
                    'I confirm this edit targets ${summary.targetPath}',
                  ),
                  subtitle: const Text(
                    'The package and USMAP bytes match the inspection. Byte-identical packages can still have different /Game targets.',
                  ),
                  controlAffinity: ListTileControlAffinity.leading,
                ),
              ],
            ),
          ),
        ],
        if (_receiptError != null) ...[
          const SizedBox(height: 8),
          Text(
            _receiptError!,
            key: const Key('dataasset-semantic-receipt-error'),
            style: TextStyle(color: theme.colorScheme.error),
          ),
        ],
        const SizedBox(height: 12),
        TextField(
          key: const Key('dataasset-semantic-search'),
          controller: _searchController,
          enabled: !_busy,
          decoration: const InputDecoration(
            labelText: 'Find a verified value',
            prefixIcon: Icon(Icons.search),
            border: OutlineInputBorder(),
          ),
          onChanged: (_) => setState(() {}),
        ),
        const SizedBox(height: 8),
        if (_choices.isEmpty)
          const Card(
            key: Key('dataasset-semantic-empty'),
            child: ListTile(
              title: Text('No editable fixed values were proven.'),
              subtitle: Text(
                'Try another walked export or a reviewed DataAsset schema.',
              ),
            ),
          )
        else
          SizedBox(
            height: 180,
            child: visible.isEmpty
                ? const Center(child: Text('No verified values match.'))
                : ListView.builder(
                    key: const Key('dataasset-semantic-leaf-list'),
                    itemCount: visible.length,
                    itemBuilder: (context, index) {
                      final choice = visible[index];
                      final selected = identical(choice, _selected);
                      return ListTile(
                        key: ValueKey(choice.key),
                        selected: selected,
                        enabled: !_busy,
                        leading: Icon(
                          selected
                              ? Icons.radio_button_checked
                              : Icons.radio_button_unchecked,
                        ),
                        title: Text(choice.leaf.selector.pathLabel),
                        subtitle: Text(
                          '${choice.export.objectName} · '
                          '${choice.leaf.selector.kind.wireName}',
                          maxLines: 1,
                          overflow: TextOverflow.ellipsis,
                        ),
                        onTap: _busy ? null : () => _select(choice),
                      );
                    },
                  ),
          ),
        if (editor != null) ...[
          const Divider(height: 24),
          Text(editor.selector.pathLabel, style: theme.textTheme.titleMedium),
          const SizedBox(height: 4),
          Text('${editor.typeLabel} · Current: ${editor.initialScalarValue}'),
          const SizedBox(height: 12),
          _ValueEditor(
            editor: editor,
            scalarController: _scalarController,
            componentControllers: _componentControllers,
            boolValue: _boolValue,
            enabled: !_busy,
            onBoolChanged: (value) => setState(() {
              _boolValue = value;
              _invalidatePreview();
            }),
            onChanged: () => setState(_invalidatePreview),
          ),
          const SizedBox(height: 12),
          Align(
            alignment: Alignment.centerLeft,
            child: FilledButton.tonalIcon(
              key: const Key('dataasset-semantic-preview'),
              onPressed: _busy || _receiptSummary == null || !_targetConfirmed
                  ? null
                  : _buildPreview,
              icon: const Icon(Icons.compare_arrows),
              label: const Text('Preview change'),
            ),
          ),
        ],
        if (_error != null) ...[
          const SizedBox(height: 12),
          Text(
            _error!,
            key: const Key('dataasset-semantic-error'),
            style: TextStyle(color: theme.colorScheme.error),
          ),
        ],
        if (_preview case final preview?) ...[
          const SizedBox(height: 12),
          Card(
            key: const Key('dataasset-semantic-diff'),
            child: Padding(
              padding: const EdgeInsets.all(12),
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Text('Preview', style: theme.textTheme.titleSmall),
                  const SizedBox(height: 8),
                  Text('Before: ${preview.previousValue}'),
                  Text('After: ${preview.replacementValue}'),
                  Text('Target: ${preview.intent.expectedTargetPath}'),
                  const SizedBox(height: 8),
                  const Text(
                    'The exact extraction proof, selector, live game '
                    'generation, and managed project head will be reverified.',
                  ),
                  const SizedBox(height: 12),
                  Align(
                    alignment: Alignment.centerLeft,
                    child: FilledButton.icon(
                      key: const Key('dataasset-semantic-stage'),
                      onPressed: _busy || _publication != null ? null : _stage,
                      icon: const Icon(Icons.inventory_2_outlined),
                      label: const Text('Stage verified edit'),
                    ),
                  ),
                ],
              ),
            ),
          ),
        ],
        if (_busy) ...[
          const SizedBox(height: 12),
          const LinearProgressIndicator(
            key: Key('dataasset-semantic-progress'),
          ),
        ],
        if (_publication case final publication?) ...[
          const SizedBox(height: 12),
          Card(
            key: const Key('dataasset-semantic-success'),
            color: theme.colorScheme.secondaryContainer,
            child: ListTile(
              leading: const Icon(Icons.check_circle_outline),
              title: const Text('Verified edit staged'),
              subtitle: Text(
                '${publication.targetPath} · project revision '
                '${publication.revision}. Build and runtime remain blocked.',
              ),
            ),
          ),
        ],
      ],
    );
  }
}

class _ValueEditor extends StatelessWidget {
  const _ValueEditor({
    required this.editor,
    required this.scalarController,
    required this.componentControllers,
    required this.boolValue,
    required this.enabled,
    required this.onBoolChanged,
    required this.onChanged,
  });

  final DataAssetSemanticValueEditor editor;
  final TextEditingController scalarController;
  final List<TextEditingController> componentControllers;
  final bool boolValue;
  final bool enabled;
  final ValueChanged<bool> onBoolChanged;
  final VoidCallback onChanged;

  @override
  Widget build(BuildContext context) {
    if (editor.isBoolean) {
      return SwitchListTile(
        key: const Key('dataasset-semantic-bool'),
        contentPadding: EdgeInsets.zero,
        title: const Text('New value'),
        subtitle: Text(boolValue ? 'On' : 'Off'),
        value: boolValue,
        onChanged: enabled ? onBoolChanged : null,
      );
    }
    if (editor.isComposite) {
      return Wrap(
        spacing: 8,
        runSpacing: 8,
        children: List.generate(componentControllers.length, (index) {
          return SizedBox(
            width: 150,
            child: TextField(
              key: ValueKey('dataasset-semantic-component-$index'),
              controller: componentControllers[index],
              enabled: enabled,
              keyboardType: const TextInputType.numberWithOptions(
                decimal: true,
                signed: true,
              ),
              decoration: InputDecoration(
                labelText: editor.componentLabels[index],
                border: const OutlineInputBorder(),
              ),
              onChanged: (_) => onChanged(),
            ),
          );
        }),
      );
    }
    return TextField(
      key: const Key('dataasset-semantic-value'),
      controller: scalarController,
      enabled: enabled,
      keyboardType: const TextInputType.numberWithOptions(
        decimal: true,
        signed: true,
      ),
      decoration: InputDecoration(
        labelText: 'New value',
        helperText: editor.typeLabel,
        border: const OutlineInputBorder(),
      ),
      onChanged: (_) => onChanged(),
    );
  }
}

final class _EditableLeafChoice {
  const _EditableLeafChoice({required this.export, required this.leaf});

  final DataAssetExportReport export;
  final DataAssetLeafReport leaf;

  String get key =>
      'dataasset-semantic-leaf-${export.index}-${leaf.index}-${leaf.selector.exportSha256}';
}

Future<String?> _pickExtractReceiptFile() async {
  final file = await openFile(
    acceptedTypeGroups: const [
      XTypeGroup(
        label: 'GORE DataAsset extraction proof',
        extensions: ['json'],
      ),
    ],
  );
  return file?.path;
}
