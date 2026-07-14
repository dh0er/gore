import 'package:flutter/material.dart';

import '../domain/dataasset_inspection.dart';
import 'dataasset_lab.dart';
import 'dataasset_semantic_edit_panel.dart';

/// Two-step managed DataAsset value workflow: prove a local snapshot, then
/// author one typed fixed-leaf stage against its exact ExtractReceipt-v2.
class DataAssetSemanticEditWizardDialog extends StatefulWidget {
  const DataAssetSemanticEditWizardDialog({
    super.key,
    required this.publish,
    required this.extractReceiptInspector,
    this.inspector,
    this.uassetPicker,
    this.usmapPicker,
    this.extractReceiptPicker,
  });

  final DataAssetSemanticStagePublisher publish;
  final DataAssetExtractReceiptInspector extractReceiptInspector;
  final DataAssetInspector? inspector;
  final DataAssetFilePicker? uassetPicker;
  final DataAssetFilePicker? usmapPicker;
  final DataAssetExtractReceiptPicker? extractReceiptPicker;

  @override
  State<DataAssetSemanticEditWizardDialog> createState() =>
      _DataAssetSemanticEditWizardDialogState();
}

class _DataAssetSemanticEditWizardDialogState
    extends State<DataAssetSemanticEditWizardDialog> {
  DataAssetInspection? _inspection;

  @override
  Widget build(BuildContext context) {
    final inspection = _inspection;
    return Dialog(
      key: const Key('dataasset-semantic-wizard'),
      child: SizedBox(
        width: 1050,
        height: 820,
        child: Column(
          children: [
            Padding(
              padding: const EdgeInsets.fromLTRB(20, 12, 8, 8),
              child: Row(
                children: [
                  const Icon(Icons.tune_outlined),
                  const SizedBox(width: 10),
                  Expanded(
                    child: Text(
                      inspection == null
                          ? '1 of 2 · Prove a DataAsset snapshot'
                          : '2 of 2 · Edit a verified value',
                      style: Theme.of(context).textTheme.titleLarge,
                    ),
                  ),
                  if (inspection != null)
                    TextButton.icon(
                      key: const Key('dataasset-semantic-wizard-back'),
                      onPressed: () => setState(() => _inspection = null),
                      icon: const Icon(Icons.arrow_back),
                      label: const Text('Choose another snapshot'),
                    ),
                  IconButton(
                    key: const Key('dataasset-semantic-wizard-close'),
                    tooltip: 'Close',
                    onPressed: () => Navigator.of(context).pop(),
                    icon: const Icon(Icons.close),
                  ),
                ],
              ),
            ),
            const Divider(height: 1),
            Expanded(
              child: inspection == null
                  ? DataAssetLab(
                      inspector: widget.inspector,
                      uassetPicker: widget.uassetPicker,
                      usmapPicker: widget.usmapPicker,
                      onInspectionReady: (result) {
                        if (!mounted) return;
                        setState(() => _inspection = result);
                      },
                    )
                  : DataAssetSemanticEditPanel(
                      inspection: inspection,
                      publish: widget.publish,
                      extractReceiptInspector: widget.extractReceiptInspector,
                      extractReceiptPicker: widget.extractReceiptPicker,
                      onPublished: (publication) =>
                          Navigator.of(context).pop(publication),
                      onUnavailable: (error) =>
                          Navigator.of(context).pop(error),
                    ),
            ),
          ],
        ),
      ),
    );
  }
}
