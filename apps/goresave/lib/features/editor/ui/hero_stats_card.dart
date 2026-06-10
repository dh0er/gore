import 'package:flutter/material.dart';

import '../domain/hero_attributes.dart';

/// Describes one entry in the player-tab sidebar. The transform entry comes
/// first (when present); then one entry per non-empty attribute group.
enum _SidebarEntry {
  transform,
  core,
  combat,
  resistances,
  thieving,
  advanced,
}

/// Grouped editors for every hero gameplay attribute. Data arrives through
/// [load] (typed property search) and leaves through [save] (one batched
/// private.typed.setValue write). [reloadKey] identifies the inspected save:
/// when it changes, pending edits are dropped and the card reloads.
///
/// Renders a master-detail layout: a slim left sidebar for navigation and a
/// right detail area showing the selected group's attribute rows plus the
/// global save control row. Pass [transformCard] to inject the hero-transform
/// editor as the first sidebar entry.
///
/// Fallback behaviour (typed parse failed or no attributes): renders [fallback]
/// (and [transformCard] when provided) in the legacy stacked layout.
class HeroStatsCard extends StatefulWidget {
  const HeroStatsCard({
    super.key,
    required this.load,
    required this.save,
    required this.editable,
    required this.reloadKey,
    this.fallback,
    this.transformCard,
  });

  final Future<HeroAttributesResult> Function() load;
  final Future<bool> Function(List<TypedValueEdit> edits) save;
  final bool editable;
  final Object reloadKey;

  /// Rendered instead of the group editors when loading finished with an error
  /// or zero attributes, so callers can keep a legacy editing surface available.
  final Widget? fallback;

  /// When provided, a "Hero transform" entry is prepended to the sidebar and
  /// this widget is shown in the detail area for that entry.
  final Widget? transformCard;

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
  // Currently selected sidebar entry.
  _SidebarEntry? _selected;
  // Epoch counter used to discard results from superseded reload calls.
  int _reloadEpoch = 0;

  static const _groupTitles = {
    HeroAttributeGroup.core: 'Main stats',
    HeroAttributeGroup.combat: 'Combat skills',
    HeroAttributeGroup.resistances: 'Resistances',
    HeroAttributeGroup.thieving: 'Thieving',
    HeroAttributeGroup.advanced: 'Advanced',
  };

  static const _entrySidebarEntry = {
    HeroAttributeGroup.core: _SidebarEntry.core,
    HeroAttributeGroup.combat: _SidebarEntry.combat,
    HeroAttributeGroup.resistances: _SidebarEntry.resistances,
    HeroAttributeGroup.thieving: _SidebarEntry.thieving,
    HeroAttributeGroup.advanced: _SidebarEntry.advanced,
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
      _selected = null;
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
      _selected = _defaultSelection(result.attributes);
    });
  }

  /// Choose the default sidebar entry: prefer 'Main stats' when present, else
  /// the first available entry (transform or first non-empty group).
  _SidebarEntry? _defaultSelection(List<HeroAttribute> attributes) {
    final byGroup = _byGroup(attributes);
    // Prefer Main stats.
    if (byGroup[HeroAttributeGroup.core]?.isNotEmpty == true) {
      return _SidebarEntry.core;
    }
    // Fall back to first available entry in sidebar order.
    if (widget.transformCard != null) return _SidebarEntry.transform;
    for (final group in HeroAttributeGroup.values) {
      if (byGroup[group]?.isNotEmpty == true) {
        return _entrySidebarEntry[group];
      }
    }
    return null;
  }

  Map<HeroAttributeGroup, List<HeroAttribute>> _byGroup(
    List<HeroAttribute> attributes,
  ) {
    final byGroup = <HeroAttributeGroup, List<HeroAttribute>>{};
    for (final attribute in attributes) {
      byGroup
          .putIfAbsent(heroAttributeGroup(attribute.id), () => [])
          .add(attribute);
    }
    return byGroup;
  }

  String _pathKey(List<String> path) => path.join(' ');

  void _onFieldChanged(List<String>? path, String text) {
    if (path == null) return;
    _pending[_pathKey(path)] = text;
  }

  Future<void> _save() async {
    // Re-entry guard: the disabled-button state only lands on the next
    // frame, so a second tap can still invoke this handler. Bail before
    // building edits or a duplicate write_save (and backup) goes out.
    if (_saving) return;
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
    if (edits.isEmpty) {
      // The input may have just been corrected back to valid-but-unchanged;
      // a validation error from the previous attempt must not stick around.
      if (_error != null && !_loadFailed) setState(() => _error = null);
      return;
    }
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
        widget.editable && !_loading && !_saving && _attributes.isNotEmpty;

    if (_loading) {
      return Card(
        child: const Padding(
          padding: EdgeInsets.all(24),
          child: Center(child: CircularProgressIndicator()),
        ),
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
          if (widget.transformCard != null) ...[
            widget.transformCard!,
            const SizedBox(height: 16),
          ],
          widget.fallback!,
        ],
      );
    }

    if ((_loadFailed || _attributes.isEmpty) && widget.transformCard != null) {
      // No stats and no fallback — just show transform.
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
          widget.transformCard!,
        ],
      );
    }

    // --- Sidebar layout ---
    final byGroup = _byGroup(_attributes);

    // Build sidebar entries in display order.
    final sidebarEntries = <_SidebarEntry>[];
    if (widget.transformCard != null) {
      sidebarEntries.add(_SidebarEntry.transform);
    }
    for (final group in HeroAttributeGroup.values) {
      if (byGroup[group]?.isNotEmpty == true) {
        sidebarEntries.add(_entrySidebarEntry[group]!);
      }
    }

    // If nothing to show at all, render error or empty.
    if (sidebarEntries.isEmpty) {
      return _error != null
          ? Text(_error!, style: TextStyle(color: theme.colorScheme.error))
          : const SizedBox.shrink();
    }

    // Ensure selected is valid — if null or stale (e.g. group disappeared after
    // a reload), pick the default.
    final effectiveSelected =
        (_selected != null && sidebarEntries.contains(_selected!))
            ? _selected!
            : sidebarEntries.first;

    // Slim save-control row: right-aligned save button + optional error text.
    final saveControlRow = Padding(
      padding: const EdgeInsets.only(bottom: 8),
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

    // Build the detail content for the selected entry.
    Widget detailContent;
    if (effectiveSelected == _SidebarEntry.transform) {
      detailContent = widget.transformCard!;
    } else {
      final group = _entryToGroup(effectiveSelected)!;
      final attributes = byGroup[group] ?? const [];
      detailContent = Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          saveControlRow,
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
        ],
      );
    }

    return Row(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        // Left sidebar: ~200px, styled to echo the save-list sidebar idiom.
        SizedBox(
          width: 200,
          child: DecoratedBox(
            decoration: BoxDecoration(
              color: theme.colorScheme.surfaceContainerLow,
              borderRadius: BorderRadius.circular(12),
            ),
            child: ListView(
              shrinkWrap: true,
              padding: const EdgeInsets.symmetric(vertical: 6),
              children: [
                for (final entry in sidebarEntries)
                  _SidebarTile(
                    label: _entryLabel(entry),
                    icon: _entryIcon(entry),
                    selected: entry == effectiveSelected,
                    onTap: () {
                      if (_selected != entry) {
                        setState(() => _selected = entry);
                      }
                    },
                  ),
              ],
            ),
          ),
        ),
        const SizedBox(width: 16),
        // Right detail area.
        Expanded(child: detailContent),
      ],
    );
  }

  HeroAttributeGroup? _entryToGroup(_SidebarEntry entry) {
    return switch (entry) {
      _SidebarEntry.core => HeroAttributeGroup.core,
      _SidebarEntry.combat => HeroAttributeGroup.combat,
      _SidebarEntry.resistances => HeroAttributeGroup.resistances,
      _SidebarEntry.thieving => HeroAttributeGroup.thieving,
      _SidebarEntry.advanced => HeroAttributeGroup.advanced,
      _SidebarEntry.transform => null,
    };
  }

  String _entryLabel(_SidebarEntry entry) {
    return switch (entry) {
      _SidebarEntry.transform => 'Hero transform',
      _SidebarEntry.core => 'Main stats',
      _SidebarEntry.combat => 'Combat skills',
      _SidebarEntry.resistances => 'Resistances',
      _SidebarEntry.thieving => 'Thieving',
      _SidebarEntry.advanced => 'Advanced',
    };
  }

  IconData _entryIcon(_SidebarEntry entry) {
    return switch (entry) {
      _SidebarEntry.transform => Icons.explore_outlined,
      _SidebarEntry.core => Icons.favorite_border,
      _SidebarEntry.combat => Icons.shield_outlined,
      _SidebarEntry.resistances => Icons.security_outlined,
      _SidebarEntry.thieving => Icons.key_outlined,
      _SidebarEntry.advanced => Icons.tune,
    };
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
      // Seed from pending text so edits made in other groups survive the
      // sidebar switch and are visible again when returning to this group.
      initialBaseText: attribute.basePath != null
          ? _pending[_pathKey(attribute.basePath!)]
          : null,
      initialCurrentText: attribute.currentPath != null
          ? _pending[_pathKey(attribute.currentPath!)]
          : null,
      onBaseChanged: (text) => _onFieldChanged(attribute.basePath, text),
      onCurrentChanged: (text) =>
          _onFieldChanged(attribute.currentPath, text),
    );
  }
}

/// A slim sidebar tile echoing the save-list sidebar idiom (Material + InkWell,
/// selected highlight via primaryContainer).
class _SidebarTile extends StatelessWidget {
  const _SidebarTile({
    required this.label,
    required this.icon,
    required this.selected,
    required this.onTap,
  });

  final String label;
  final IconData icon;
  final bool selected;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    final scheme = Theme.of(context).colorScheme;
    return Padding(
      padding: const EdgeInsets.symmetric(horizontal: 6, vertical: 2),
      child: Material(
        color: selected ? scheme.primaryContainer : Colors.transparent,
        borderRadius: BorderRadius.circular(8),
        child: InkWell(
          onTap: onTap,
          borderRadius: BorderRadius.circular(8),
          child: Padding(
            padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 9),
            child: Row(
              children: [
                Icon(
                  icon,
                  size: 18,
                  color: selected ? scheme.primary : scheme.onSurfaceVariant,
                ),
                const SizedBox(width: 8),
                Expanded(
                  child: Text(
                    label,
                    maxLines: 1,
                    overflow: TextOverflow.ellipsis,
                    style: Theme.of(context).textTheme.bodyMedium?.copyWith(
                      color: selected ? scheme.primary : scheme.onSurface,
                      fontWeight: selected ? FontWeight.w600 : null,
                    ),
                  ),
                ),
              ],
            ),
          ),
        ),
      ),
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
    this.initialBaseText,
    this.initialCurrentText,
  });

  final HeroAttribute attribute;
  final bool duplicate;
  final bool editable;
  final ValueChanged<String> onBaseChanged;
  final ValueChanged<String> onCurrentChanged;
  // When present, these seed the field controllers (pending edit still visible
  // after switching away and back). Overrides the formatted attribute value.
  final String? initialBaseText;
  final String? initialCurrentText;

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
      // Prefer the pending text (surviving a sidebar switch) over the
      // formatted attribute value, so a dirty field stays dirty on return.
      text: widget.initialBaseText ?? formatHeroValue(widget.attribute.baseValue),
    );
    _currentController = TextEditingController(
      text: widget.initialCurrentText ??
          formatHeroValue(widget.attribute.currentValue),
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
