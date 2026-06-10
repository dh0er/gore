import 'package:flutter/material.dart';

import '../domain/hero_attributes.dart';

/// Grouped editors for every hero gameplay attribute. Data arrives through
/// [load] (typed property search) and leaves through [save] (one batched
/// private.typed.setValue write). [reloadKey] identifies the inspected save:
/// when it changes, pending edits are dropped and the card reloads.
///
/// Renders one [Card] per non-empty attribute group, plus a slim save-control
/// row (save button + error text) above the group cards. No outer "Hero stats"
/// wrapper card.
class HeroStatsCard extends StatefulWidget {
  const HeroStatsCard({
    super.key,
    required this.load,
    required this.save,
    required this.editable,
    required this.reloadKey,
    this.fallback,
  });

  final Future<HeroAttributesResult> Function() load;
  final Future<bool> Function(List<TypedValueEdit> edits) save;
  final bool editable;
  final Object reloadKey;

  /// Rendered instead of the group editors when loading finished with an error
  /// or zero attributes, so callers can keep a legacy editing surface available.
  final Widget? fallback;

  @override
  State<HeroStatsCard> createState() => _HeroStatsCardState();
}

class _HeroStatsCardState extends State<HeroStatsCard> {
  List<HeroAttribute> _attributes = const [];
  String? _error;
  // True only when the last load itself failed; save-validation errors set
  // _error without this, so they never swap the editors for the fallback.
  bool _loadFailed = false;
  bool _loading = false;
  bool _saving = false;
  // Pending field texts keyed by the typed path (joined). Cleared on reload.
  final Map<String, String> _pending = {};
  bool _advancedExpanded = false;
  // Epoch counter used to discard results from superseded reload calls.
  int _reloadEpoch = 0;

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
    final epoch = ++_reloadEpoch;
    setState(() {
      _loading = true;
      _pending.clear();
    });
    final result = await widget.load();
    // Discard results from superseded reload calls (e.g. rapid reloadKey
    // changes) to avoid applying stale data over a more recent load.
    if (!mounted || epoch != _reloadEpoch) return;
    setState(() {
      _loading = false;
      _error = result.error;
      _loadFailed = result.error != null;
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
        // Treat missing or whitespace-only text as "no change".
        if (text == null || text.trim().isEmpty) continue;
        final value = double.tryParse(text.trim());
        if (value == null) {
          setState(
            () => _error = 'Invalid number for ${attribute.id}: "$text"',
          );
          return;
        }
        if (value == original) continue;
        edits.add(TypedValueEdit(path: path, value: value));
      }
    }
    if (edits.isEmpty) return;
    setState(() {
      _error = null;
      _saving = true;
    });
    try {
      await widget.save(edits);
      // The save triggers a re-inspect upstream; reloadKey changes and this
      // card reloads with fresh values.
    } finally {
      if (mounted) setState(() => _saving = false);
    }
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final canSave =
        widget.editable &&
        !_loading &&
        !_saving &&
        _attributes.isNotEmpty;

    // Slim save-control row: right-aligned save button + optional error text.
    final saveControlRow = Padding(
      padding: const EdgeInsets.only(bottom: 4),
      child: Row(
        children: [
          if (_error != null)
            Expanded(
              child: Text(
                _error!,
                style: TextStyle(color: theme.colorScheme.error),
              ),
            )
          else
            const Spacer(),
          Tooltip(
            message: 'Save hero stats',
            child: IconButton.filledTonal(
              icon: const Icon(Icons.save_outlined),
              onPressed: canSave ? _save : null,
            ),
          ),
        ],
      ),
    );

    if (_loading) {
      return Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          saveControlRow,
          Card(
            child: const Padding(
              padding: EdgeInsets.all(24),
              child: Center(child: CircularProgressIndicator()),
            ),
          ),
        ],
      );
    }

    if ((_loadFailed || _attributes.isEmpty) && widget.fallback != null) {
      // The fallback editor has its own save affordances; a permanently
      // disabled hero-stats save button above it would only confuse. Keep
      // the error text so the user sees why the typed editors are gone.
      return Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          if (_error != null)
            Padding(
              padding: const EdgeInsets.only(bottom: 4),
              child: Text(
                _error!,
                style: TextStyle(color: theme.colorScheme.error),
              ),
            ),
          widget.fallback!,
        ],
      );
    }

    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        saveControlRow,
        ..._buildGroupCards(context),
      ],
    );
  }

  List<Widget> _buildGroupCards(BuildContext context) {
    final theme = Theme.of(context);
    final byGroup = <HeroAttributeGroup, List<HeroAttribute>>{};
    for (final attribute in _attributes) {
      byGroup
          .putIfAbsent(heroAttributeGroup(attribute.id), () => [])
          .add(attribute);
    }
    final cards = <Widget>[];
    for (final group in HeroAttributeGroup.values) {
      final attributes = byGroup[group];
      if (attributes == null || attributes.isEmpty) continue;
      // Same 16px rhythm as the sibling top-level cards in the Player tab.
      if (cards.isNotEmpty) cards.add(const SizedBox(height: 16));
      if (group == HeroAttributeGroup.advanced) {
        cards.add(
          Card(
            child: Padding(
              padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 12),
              child: ExpansionTile(
                tilePadding: EdgeInsets.zero,
                title: Text(
                  _groupTitles[group]!,
                  style: theme.textTheme.titleSmall,
                ),
                initiallyExpanded: _advancedExpanded,
                onExpansionChanged: (open) => _advancedExpanded = open,
                // Keep collapsed rows alive so their text controllers stay in sync
                // with _pending; without this, collapsing and re-expanding would
                // reset the fields while _pending still held the dirty values.
                maintainState: true,
                children: [for (final a in attributes) _row(a)],
              ),
            ),
          ),
        );
      } else {
        cards.add(
          Card(
            child: Padding(
              padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 12),
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Text(
                    _groupTitles[group]!,
                    style: theme.textTheme.titleSmall,
                  ),
                  const SizedBox(height: 4),
                  for (final a in attributes) _row(a),
                ],
              ),
            ),
          ),
        );
      }
    }
    return cards;
  }

  Widget _row(HeroAttribute attribute) {
    final duplicate =
        _attributes.where((a) => a.id == attribute.id).length > 1;
    return _HeroAttributeRow(
      // Record key compares reloadKey by its own equality (identity for
      // SaveInspection, which has no == override), not by toString(), so a
      // fresh SaveInspection instance always causes a new row to be built
      // rather than reusing stale field state from the previous load.
      key: ValueKey((widget.reloadKey, attribute.setClass, attribute.id)),
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
            decoration: InputDecoration(labelText: '$_label base'),
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
            decoration: InputDecoration(labelText: '$_label current'),
          );
          final rowLabel = Text(
            _label,
            style: Theme.of(context).textTheme.labelLarge,
          );
          if (compact) {
            return Column(
              crossAxisAlignment: CrossAxisAlignment.stretch,
              children: [
                rowLabel,
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
              SizedBox(width: 170, child: rowLabel),
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

/// Integers render without a decimal point; non-integers keep up to two
/// decimals (mirrors the attribute formatting used elsewhere in the editor).
/// An editable field's text becomes the saved value, so never seed it with
/// a lossy rounding: if the shortened form does not parse back to the same
/// value (e.g. 0.125 rounds to 0.13), fall back to the full toString().
String formatHeroValue(double? value) {
  if (value == null) return '';
  if (value == value.roundToDouble()) return value.toInt().toString();
  final rounded =
      value.toStringAsFixed(2).replaceFirst(RegExp(r'\.?0+$'), '');
  // An editable field's text becomes the saved value, so never seed it with
  // a lossy rounding (0.125 must not display — and then save — as 0.13).
  return double.tryParse(rounded) == value ? rounded : value.toString();
}
