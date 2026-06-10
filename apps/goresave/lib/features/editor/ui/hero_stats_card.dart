import 'package:flutter/material.dart';

import '../domain/hero_attributes.dart';

/// Grouped editors for every hero gameplay attribute. Data arrives through
/// [load] (typed property search) and leaves through [save] (one batched
/// private.typed.setValue write). [reloadKey] identifies the inspected save:
/// when it changes, pending edits are dropped and the card reloads.
class HeroStatsCard extends StatefulWidget {
  const HeroStatsCard({
    super.key,
    required this.load,
    required this.save,
    required this.editable,
    required this.reloadKey,
  });

  final Future<HeroAttributesResult> Function() load;
  final Future<bool> Function(List<TypedValueEdit> edits) save;
  final bool editable;
  final Object reloadKey;

  @override
  State<HeroStatsCard> createState() => _HeroStatsCardState();
}

class _HeroStatsCardState extends State<HeroStatsCard> {
  List<HeroAttribute> _attributes = const [];
  String? _error;
  bool _loading = false;
  // Pending field texts keyed by the typed path (joined). Cleared on reload.
  final Map<String, String> _pending = {};
  bool _advancedExpanded = false;

  static const _groupTitles = {
    HeroAttributeGroup.core: 'Main stats',
    HeroAttributeGroup.combat: 'Combat skills',
    HeroAttributeGroup.resistances: 'Resistances',
    HeroAttributeGroup.thieving: 'Thieving',
    HeroAttributeGroup.advanced: 'Advanced',
  };

  @override
  void initState() {
    super.initState();
    _reload();
  }

  @override
  void didUpdateWidget(covariant HeroStatsCard oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (widget.reloadKey != oldWidget.reloadKey) _reload();
  }

  Future<void> _reload() async {
    setState(() {
      _loading = true;
      _pending.clear();
    });
    final result = await widget.load();
    if (!mounted) return;
    setState(() {
      _loading = false;
      _error = result.error;
      _attributes = result.attributes;
    });
  }

  String _pathKey(List<String> path) => path.join(' ');

  void _onFieldChanged(List<String>? path, String text) {
    if (path == null) return;
    _pending[_pathKey(path)] = text;
  }

  Future<void> _save() async {
    final edits = <TypedValueEdit>[];
    for (final attribute in _attributes) {
      for (final (path, original) in [
        (attribute.basePath, attribute.baseValue),
        (attribute.currentPath, attribute.currentValue),
      ]) {
        if (path == null) continue;
        final text = _pending[_pathKey(path)];
        if (text == null) continue;
        final value = double.tryParse(text.trim());
        if (value == null) {
          setState(() => _error = 'Invalid number: "$text"');
          return;
        }
        if (value == original) continue;
        edits.add(TypedValueEdit(path: path, value: value));
      }
    }
    if (edits.isEmpty) return;
    setState(() => _error = null);
    await widget.save(edits);
    // The save triggers a re-inspect upstream; reloadKey changes and this
    // card reloads with fresh values.
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Card(
      child: Padding(
        padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 12),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Row(
              children: [
                const Icon(Icons.monitor_heart_outlined),
                const SizedBox(width: 8),
                Expanded(
                  child: Text(
                    'Hero stats',
                    style: theme.textTheme.titleMedium,
                  ),
                ),
                Tooltip(
                  message: 'Save hero stats',
                  child: IconButton.filledTonal(
                    icon: const Icon(Icons.save_outlined),
                    onPressed:
                        widget.editable && !_loading ? _save : null,
                  ),
                ),
              ],
            ),
            if (_error != null)
              Padding(
                padding: const EdgeInsets.only(top: 8),
                child: Text(
                  _error!,
                  style: TextStyle(color: theme.colorScheme.error),
                ),
              ),
            if (_loading)
              const Padding(
                padding: EdgeInsets.all(16),
                child: Center(child: CircularProgressIndicator()),
              )
            else
              ..._buildGroups(context),
          ],
        ),
      ),
    );
  }

  List<Widget> _buildGroups(BuildContext context) {
    final theme = Theme.of(context);
    final byGroup = <HeroAttributeGroup, List<HeroAttribute>>{};
    for (final attribute in _attributes) {
      byGroup
          .putIfAbsent(heroAttributeGroup(attribute.id), () => [])
          .add(attribute);
    }
    final widgets = <Widget>[];
    for (final group in HeroAttributeGroup.values) {
      final attributes = byGroup[group];
      if (attributes == null || attributes.isEmpty) continue;
      if (group == HeroAttributeGroup.advanced) {
        widgets.add(
          ExpansionTile(
            tilePadding: EdgeInsets.zero,
            title: Text(
              _groupTitles[group]!,
              style: theme.textTheme.titleSmall,
            ),
            initiallyExpanded: _advancedExpanded,
            onExpansionChanged: (open) => _advancedExpanded = open,
            children: [for (final a in attributes) _row(a)],
          ),
        );
        continue;
      }
      widgets
        ..add(const SizedBox(height: 12))
        ..add(Text(_groupTitles[group]!, style: theme.textTheme.titleSmall))
        ..add(const SizedBox(height: 4))
        ..addAll([for (final a in attributes) _row(a)]);
    }
    return widgets;
  }

  Widget _row(HeroAttribute attribute) {
    final duplicate =
        _attributes.where((a) => a.id == attribute.id).length > 1;
    return _HeroAttributeRow(
      // Keyed by save identity and full path so a different save (or set)
      // never reuses stale field state.
      key: ValueKey(
        '${widget.reloadKey}-${attribute.setClass}-${attribute.id}',
      ),
      attribute: attribute,
      duplicate: duplicate,
      editable: widget.editable,
      onBaseChanged: (text) => _onFieldChanged(attribute.basePath, text),
      onCurrentChanged: (text) =>
          _onFieldChanged(attribute.currentPath, text),
    );
  }
}

class _HeroAttributeRow extends StatefulWidget {
  const _HeroAttributeRow({
    super.key,
    required this.attribute,
    required this.duplicate,
    required this.editable,
    required this.onBaseChanged,
    required this.onCurrentChanged,
  });

  final HeroAttribute attribute;
  final bool duplicate;
  final bool editable;
  final ValueChanged<String> onBaseChanged;
  final ValueChanged<String> onCurrentChanged;

  @override
  State<_HeroAttributeRow> createState() => _HeroAttributeRowState();
}

class _HeroAttributeRowState extends State<_HeroAttributeRow> {
  late final TextEditingController _baseController;
  late final TextEditingController _currentController;

  @override
  void initState() {
    super.initState();
    _baseController = TextEditingController(
      text: formatHeroValue(widget.attribute.baseValue),
    );
    _currentController = TextEditingController(
      text: formatHeroValue(widget.attribute.currentValue),
    );
  }

  @override
  void dispose() {
    _baseController.dispose();
    _currentController.dispose();
    super.dispose();
  }

  String get _label {
    final label = heroAttributeLabel(widget.attribute.id);
    if (!widget.duplicate) return label;
    final setName = widget.attribute.setClass.split('.').last;
    return '$label ($setName)';
  }

  @override
  Widget build(BuildContext context) {
    final name = widget.attribute.id;
    return Padding(
      padding: const EdgeInsets.symmetric(vertical: 5),
      child: LayoutBuilder(
        builder: (context, constraints) {
          final compact = constraints.maxWidth < 620;
          final baseField = TextField(
            controller: _baseController,
            enabled: widget.editable && widget.attribute.basePath != null,
            onChanged: widget.onBaseChanged,
            keyboardType: const TextInputType.numberWithOptions(
              decimal: true,
              signed: true,
            ),
            decoration: InputDecoration(labelText: '$name base'),
          );
          final currentField = TextField(
            controller: _currentController,
            enabled:
                widget.editable && widget.attribute.currentPath != null,
            onChanged: widget.onCurrentChanged,
            keyboardType: const TextInputType.numberWithOptions(
              decimal: true,
              signed: true,
            ),
            decoration: InputDecoration(labelText: '$name current'),
          );
          final label = Text(
            _label,
            style: Theme.of(context).textTheme.labelLarge,
          );
          if (compact) {
            return Column(
              crossAxisAlignment: CrossAxisAlignment.stretch,
              children: [
                label,
                const SizedBox(height: 6),
                baseField,
                const SizedBox(height: 6),
                currentField,
              ],
            );
          }
          return Row(
            crossAxisAlignment: CrossAxisAlignment.center,
            children: [
              SizedBox(width: 170, child: label),
              Expanded(child: baseField),
              const SizedBox(width: 8),
              Expanded(child: currentField),
            ],
          );
        },
      ),
    );
  }
}

/// Integers render without a decimal point; everything else keeps up to two
/// decimals (mirrors the attribute formatting used elsewhere in the editor).
String formatHeroValue(double? value) {
  if (value == null) return '';
  if (value == value.roundToDouble()) return value.toInt().toString();
  return value.toStringAsFixed(2).replaceFirst(RegExp(r'\.?0+$'), '');
}
