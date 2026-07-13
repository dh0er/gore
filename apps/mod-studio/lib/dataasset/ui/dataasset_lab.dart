import 'dart:async';

import 'package:file_selector/file_selector.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../core/mod_ffi.dart';
import '../../core/providers.dart';

typedef DataAssetInspector =
    Future<DataAssetInspection> Function({
      required String uassetPath,
      required String usmapPath,
      int? exportIndex,
    });
typedef DataAssetFilePicker = Future<String?> Function();

/// Bounded, read-only UI over `dataasset_fixed_inspect_v1`.
///
/// Pickers and the inspector are injectable so cancellation, races, and large
/// result sets can be tested without native dialogs or a DLL.
class DataAssetLab extends ConsumerStatefulWidget {
  const DataAssetLab({
    super.key,
    this.inspector,
    this.uassetPicker,
    this.usmapPicker,
  });

  final DataAssetInspector? inspector;
  final DataAssetFilePicker? uassetPicker;
  final DataAssetFilePicker? usmapPicker;

  @override
  ConsumerState<DataAssetLab> createState() => _DataAssetLabState();
}

class _DataAssetLabState extends ConsumerState<DataAssetLab> {
  final _exportIndexController = TextEditingController();
  final _searchController = TextEditingController();
  String? _uassetPath;
  String? _usmapPath;
  DataAssetInspection? _inspection;
  Object? _error;
  bool _busy = false;
  int _requestEpoch = 0;
  int _pickerEpoch = 0;

  @override
  void dispose() {
    _requestEpoch++;
    _pickerEpoch++;
    _exportIndexController.dispose();
    _searchController.dispose();
    super.dispose();
  }

  int? get _exportIndex {
    final raw = _exportIndexController.text.trim();
    if (raw.isEmpty) return null;
    return int.tryParse(raw);
  }

  String? get _exportIndexError {
    final raw = _exportIndexController.text.trim();
    if (raw.isEmpty) return null;
    final parsed = int.tryParse(raw);
    if (parsed == null || parsed < 0 || parsed > 0x7fffffff) {
      return 'Enter an index from 0 to 2147483647.';
    }
    return null;
  }

  bool get _canInspect =>
      !_busy &&
      _uassetPath != null &&
      _usmapPath != null &&
      _exportIndexError == null;

  void _inputsChanged() {
    _requestEpoch++;
    _inspection = null;
    _error = null;
    _busy = false;
  }

  Future<void> _pickUasset() => _pick(
    picker: widget.uassetPicker ?? _pickUassetFile,
    apply: (path) => _uassetPath = path,
  );

  Future<void> _pickUsmap() => _pick(
    picker: widget.usmapPicker ?? _pickUsmapFile,
    apply: (path) => _usmapPath = path,
  );

  Future<void> _pick({
    required DataAssetFilePicker picker,
    required void Function(String path) apply,
  }) async {
    final pickerEpoch = ++_pickerEpoch;
    final path = await picker();
    if (!mounted || pickerEpoch != _pickerEpoch || path == null) return;
    setState(() {
      apply(path);
      _inputsChanged();
    });
  }

  Future<void> _inspect() async {
    if (!_canInspect) return;
    final uassetPath = _uassetPath!;
    final usmapPath = _usmapPath!;
    final exportIndex = _exportIndex;
    final requestEpoch = ++_requestEpoch;
    setState(() {
      _busy = true;
      _inspection = null;
      _error = null;
    });
    try {
      final inspector = widget.inspector;
      final inspection = inspector == null
          ? await ModFfi(ref.read(coreServiceProvider)).dataAssetFixedInspectV1(
              uassetPath: uassetPath,
              usmapPath: usmapPath,
              exportIndex: exportIndex,
            )
          : await inspector(
              uassetPath: uassetPath,
              usmapPath: usmapPath,
              exportIndex: exportIndex,
            );
      if (!mounted || requestEpoch != _requestEpoch) return;
      setState(() {
        _inspection = inspection;
        _busy = false;
      });
    } catch (error) {
      if (!mounted || requestEpoch != _requestEpoch) return;
      setState(() {
        _error = error;
        _busy = false;
      });
    }
  }

  List<DataAssetExportReport> _visibleExports(DataAssetInspection result) {
    final query = _searchController.text.trim().toLowerCase();
    if (query.isEmpty) return result.exports;
    return result.exports
        .where((report) {
          if (report.objectName.toLowerCase().contains(query) ||
              report.classPath.toLowerCase().contains(query) ||
              (report.schema?.toLowerCase().contains(query) ?? false)) {
            return true;
          }
          return report.leaves.any((leaf) {
            final selector = leaf.selector;
            return selector.kind.wireName.contains(query) ||
                selector.role.wireName.contains(query) ||
                selector.pathLabel.toLowerCase().contains(query);
          });
        })
        .toList(growable: false);
  }

  @override
  Widget build(BuildContext context) {
    final result = _inspection;
    final visibleExports = result == null
        ? const <DataAssetExportReport>[]
        : _visibleExports(result);
    return Padding(
      padding: const EdgeInsets.all(16),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          const _EvidenceNotice(),
          const SizedBox(height: 12),
          _InputPanel(
            uassetPath: _uassetPath,
            usmapPath: _usmapPath,
            exportIndexController: _exportIndexController,
            exportIndexError: _exportIndexError,
            canInspect: _canInspect,
            onPickUasset: _pickUasset,
            onPickUsmap: _pickUsmap,
            onExportIndexChanged: (_) => setState(_inputsChanged),
            onInspect: _inspect,
          ),
          if (_busy) ...[
            const SizedBox(height: 12),
            const LinearProgressIndicator(key: Key('dataasset-progress')),
          ],
          if (_error != null) ...[
            const SizedBox(height: 12),
            _InspectionError(error: _error!),
          ],
          if (result != null) ...[
            const SizedBox(height: 12),
            _Summary(result: result),
            const SizedBox(height: 8),
            TextField(
              key: const Key('dataasset-search'),
              controller: _searchController,
              decoration: const InputDecoration(
                labelText: 'Filter proven facts',
                prefixIcon: Icon(Icons.search),
                border: OutlineInputBorder(),
              ),
              onChanged: (_) => setState(() {}),
            ),
            const SizedBox(height: 8),
            Expanded(
              child: visibleExports.isEmpty
                  ? const Center(child: Text('No matching export facts.'))
                  : ListView.builder(
                      key: const Key('dataasset-export-list'),
                      itemCount: visibleExports.length,
                      itemBuilder: (context, index) => _ExportCard(
                        key: ValueKey(
                          'dataasset-export-${visibleExports[index].index}',
                        ),
                        report: visibleExports[index],
                      ),
                    ),
            ),
          ] else
            const Spacer(),
        ],
      ),
    );
  }
}

class _EvidenceNotice extends StatelessWidget {
  const _EvidenceNotice();

  @override
  Widget build(BuildContext context) => Card(
    color: Theme.of(context).colorScheme.secondaryContainer,
    child: const ListTile(
      leading: Icon(Icons.policy_outlined),
      title: Text('DataAsset Lab — offline evidence'),
      subtitle: Text(
        'Read-only inspection of selected package and USMAP snapshots. '
        'No game or project files are changed.',
      ),
    ),
  );
}

class _InputPanel extends StatelessWidget {
  const _InputPanel({
    required this.uassetPath,
    required this.usmapPath,
    required this.exportIndexController,
    required this.exportIndexError,
    required this.canInspect,
    required this.onPickUasset,
    required this.onPickUsmap,
    required this.onExportIndexChanged,
    required this.onInspect,
  });

  final String? uassetPath;
  final String? usmapPath;
  final TextEditingController exportIndexController;
  final String? exportIndexError;
  final bool canInspect;
  final VoidCallback onPickUasset;
  final VoidCallback onPickUsmap;
  final ValueChanged<String> onExportIndexChanged;
  final VoidCallback onInspect;

  @override
  Widget build(BuildContext context) => Card(
    child: Padding(
      padding: const EdgeInsets.all(12),
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Expanded(
            child: Column(
              children: [
                _FileChoice(
                  buttonKey: const Key('dataasset-pick-uasset'),
                  label: 'Choose .uasset',
                  path: uassetPath,
                  onPressed: onPickUasset,
                ),
                const SizedBox(height: 8),
                _FileChoice(
                  buttonKey: const Key('dataasset-pick-usmap'),
                  label: 'Choose .usmap',
                  path: usmapPath,
                  onPressed: onPickUsmap,
                ),
              ],
            ),
          ),
          const SizedBox(width: 12),
          SizedBox(
            width: 220,
            child: TextField(
              key: const Key('dataasset-export-index'),
              controller: exportIndexController,
              keyboardType: TextInputType.number,
              inputFormatters: [FilteringTextInputFormatter.digitsOnly],
              decoration: InputDecoration(
                labelText: 'Export index (optional)',
                errorText: exportIndexError,
                border: const OutlineInputBorder(),
              ),
              onChanged: onExportIndexChanged,
              onSubmitted: (_) {
                if (canInspect) onInspect();
              },
            ),
          ),
          const SizedBox(width: 12),
          FilledButton.icon(
            key: const Key('dataasset-inspect'),
            onPressed: canInspect ? onInspect : null,
            icon: const Icon(Icons.manage_search),
            label: const Text('Inspect snapshot'),
          ),
        ],
      ),
    ),
  );
}

class _FileChoice extends StatelessWidget {
  const _FileChoice({
    required this.buttonKey,
    required this.label,
    required this.path,
    required this.onPressed,
  });

  final Key buttonKey;
  final String label;
  final String? path;
  final VoidCallback? onPressed;

  @override
  Widget build(BuildContext context) => Row(
    children: [
      OutlinedButton.icon(
        key: buttonKey,
        onPressed: onPressed,
        icon: const Icon(Icons.folder_open_outlined),
        label: Text(label),
      ),
      const SizedBox(width: 8),
      Expanded(
        child: Tooltip(
          message: path ?? 'Nothing selected',
          child: Text(
            path ?? 'Nothing selected',
            maxLines: 1,
            overflow: TextOverflow.ellipsis,
          ),
        ),
      ),
    ],
  );
}

class _InspectionError extends StatelessWidget {
  const _InspectionError({required this.error});
  final Object error;

  @override
  Widget build(BuildContext context) => Card(
    key: const Key('dataasset-error'),
    color: Theme.of(context).colorScheme.errorContainer,
    child: ListTile(
      leading: const Icon(Icons.error_outline),
      title: const Text('Inspection failed'),
      subtitle: Text(error.toString()),
    ),
  );
}

class _Summary extends StatelessWidget {
  const _Summary({required this.result});
  final DataAssetInspection result;

  @override
  Widget build(BuildContext context) {
    final summary = result.summary;
    return Card(
      key: const Key('dataasset-summary'),
      child: Padding(
        padding: const EdgeInsets.all(12),
        child: Wrap(
          spacing: 18,
          runSpacing: 8,
          crossAxisAlignment: WrapCrossAlignment.center,
          children: [
            _StatusChip(status: result.status),
            Text('${summary.walkedExports}/${summary.reportedExports} walked'),
            Text('${summary.packageExports} package exports'),
            Text('${summary.editableLeaves} value-only leaf classifications'),
            Text('UASSET ${_bytes(result.input.uassetLength)}'),
            Text('UEXP ${_bytes(result.input.uexpLength)}'),
            Text('USMAP ${_bytes(result.input.usmapLength)}'),
            Tooltip(
              message: result.binding.usmapSha256,
              child: Text('USMAP ${_shortHash(result.binding.usmapSha256)}'),
            ),
          ],
        ),
      ),
    );
  }
}

class _StatusChip extends StatelessWidget {
  const _StatusChip({required this.status});
  final DataAssetInspectionStatus status;

  @override
  Widget build(BuildContext context) {
    final (icon, color) = switch (status) {
      DataAssetInspectionStatus.walked => (
        Icons.verified_outlined,
        Theme.of(context).colorScheme.primary,
      ),
      DataAssetInspectionStatus.partial => (
        Icons.warning_amber_outlined,
        Theme.of(context).colorScheme.tertiary,
      ),
      DataAssetInspectionStatus.unsupported => (
        Icons.block_outlined,
        Theme.of(context).colorScheme.error,
      ),
    };
    return Chip(
      avatar: Icon(icon, size: 18, color: color),
      label: Text(status.wireName),
    );
  }
}

class _ExportCard extends StatelessWidget {
  const _ExportCard({super.key, required this.report});
  final DataAssetExportReport report;

  @override
  Widget build(BuildContext context) {
    final walked = report.status == DataAssetInspectionStatus.walked;
    final failure = report.failure;
    return Card(
      margin: const EdgeInsets.only(bottom: 8),
      child: ExpansionTile(
        key: Key('dataasset-export-tile-${report.index}'),
        enabled: walked,
        leading: _StatusChip(status: report.status),
        title: Text('#${report.index}  ${report.objectName}'),
        subtitle: Text(
          walked
              ? '${report.classPath} · ${report.schema} · '
                    '${report.leaves.length} fixed leaves'
              : '${report.classPath} · ${failure!.stage}: ${failure.code}',
          maxLines: 2,
          overflow: TextOverflow.ellipsis,
        ),
        childrenPadding: const EdgeInsets.fromLTRB(16, 0, 16, 12),
        children: walked
            ? [
                Align(
                  alignment: Alignment.centerLeft,
                  child: Wrap(
                    spacing: 16,
                    runSpacing: 4,
                    children: [
                      Text(
                        '${report.component.toUpperCase()} ${_bytes(report.length)}',
                      ),
                      Text('Properties ${_bytes(report.propertyBytes!)}'),
                      Text(
                        'Native suffix ${_bytes(report.nativeSuffixBytes!)}',
                      ),
                    ],
                  ),
                ),
                const SizedBox(height: 8),
                if (report.leaves.isEmpty)
                  const Align(
                    alignment: Alignment.centerLeft,
                    child: Text('No fixed-width leaf facts were proven.'),
                  )
                else
                  SizedBox(
                    height: (report.leaves.length * 74.0).clamp(74.0, 360.0),
                    child: ListView.builder(
                      key: Key('dataasset-leaf-list-${report.index}'),
                      itemCount: report.leaves.length,
                      itemBuilder: (context, index) => _LeafFact(
                        key: ValueKey('dataasset-leaf-${report.index}-$index'),
                        leaf: report.leaves[index],
                      ),
                    ),
                  ),
              ]
            : const [],
      ),
    );
  }
}

class _LeafFact extends StatelessWidget {
  const _LeafFact({super.key, required this.leaf});
  final DataAssetLeafReport leaf;

  @override
  Widget build(BuildContext context) {
    final selector = leaf.selector;
    return ListTile(
      dense: true,
      leading: Text('#${leaf.index}'),
      title: Text(
        '${selector.pathLabel} · ${selector.kind.wireName}',
        maxLines: 1,
        overflow: TextOverflow.ellipsis,
      ),
      subtitle: Text(
        '${selector.role.wireName} · expected ${selector.expectedHex} · '
        'export ${_shortHash(selector.exportSha256)}',
        maxLines: 1,
        overflow: TextOverflow.ellipsis,
      ),
      trailing: Chip(
        label: Text(leaf.editable ? 'value-only' : 'restricted role'),
      ),
    );
  }
}

Future<String?> _pickUassetFile() async {
  final file = await openFile(
    acceptedTypeGroups: const [
      XTypeGroup(label: 'Cooked Unreal package', extensions: ['uasset']),
    ],
  );
  return file?.path;
}

Future<String?> _pickUsmapFile() async {
  final file = await openFile(
    acceptedTypeGroups: const [
      XTypeGroup(label: 'Unreal schema map', extensions: ['usmap']),
    ],
  );
  return file?.path;
}

String _shortHash(String value) => '${value.substring(0, 8)}…';

String _bytes(int value) {
  if (value < 1024) return '$value B';
  if (value < 1024 * 1024) return '${(value / 1024).toStringAsFixed(1)} KiB';
  return '${(value / (1024 * 1024)).toStringAsFixed(1)} MiB';
}
