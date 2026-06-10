import 'package:flutter/material.dart';

import '../domain/hero_attributes.dart';

/// Describes one entry in the player-tab sidebar. Entries appear in enum
/// declaration order in the sidebar: core, combat, resistances, thieving,
/// transform (when present), advanced.
enum _SidebarEntry {
  core,
  combat,
  resistances,
  thieving,
  transform,
  advanced,
}

/// Grouped editors for every hero gameplay attribute. Data arrives through
/// [load] (typed property search) and leaves through [onPendingChanged]
/// (callback fired with the current pending typed edits + any validation
/// error). [reloadKey] identifies the inspected save: when it changes,
/// pending edits are dropped and the card reloads.
///
/// Renders a master-detail layout: a slim left sidebar for navigation and a
/// right detail area showing the selected group's attribute rows. Pass
/// [transformCard] to inject the hero-transform editor as the first sidebar
/// entry.
///
/// Fallback behaviour (typed parse failed or no attributes): renders [fallback]
/// (and [transformCard] when provided) in the legacy stacked layout.
class HeroStatsCard extends StatefulWidget {
  const HeroStatsCard({
    super.key,
    required this.load,
    required this.onPendingChanged,
    required this.editable,
    required this.reloadKey,
    this.fallback,
    this.transformCard,
  });

  final Future<HeroAttributesResult> Function() load;

  /// Called whenever the set of pending edits changes.
  /// [edits] is the full list of TypedValueEdit objects to write (empty when
  /// there are no dirty or valid pending edits).
  /// [validationError] is non-null when any field is invalid or empty — in
  /// that case the whole card's edits are suppressed until corrected.
  final void Function(List<TypedValueEdit> edits, String? validationError)
  onPendingChanged;

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
  // True only when the last load itself failed; validation errors set
  // _error without this, so they never swap the editors for the fallback.
  bool _loadFailed = false;
  bool _loading = false;
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
    // Do NOT call widget.onPendingChanged here: the notifier centrally clears
    // the 'heroStats' pending entry in refresh() (event-handler context).
    // Calling onPendingChanged → clearPendingEdit from initState/didUpdateWidget
    // mutates the provider during build and throws with flutter_riverpod.
    final result = await widget.load();
    // Discard results from superseded reload calls (e.g. rapid reloadKey
    // changes) to avoid applying stale data over a more recent load.
    if (!mounted || epoch != _reloadEpoch) return;
    setState(() {
      _loading = false;
      _error = result.error;
      _loadFailed = result.error != null;
      _attributes = result.attributes;
      // Keep the user's pane across save-triggered reloads (every save makes
      // a new inspection and lands here); only pick the default when nothing
      // is selected yet. effectiveSelected guards against entries that
      // disappeared.
      _selected ??= _defaultSelection(result.attributes);
    });
  }

  /// Choose the default sidebar entry: prefer 'Main stats' when present, else
  /// the first available entry in sidebar (enum declaration) order.
  _SidebarEntry? _defaultSelection(List<HeroAttribute> attributes) {
    final byGroup = _byGroup(attributes);
    // Prefer Main stats.
    if (byGroup[HeroAttributeGroup.core]?.isNotEmpty == true) {
      return _SidebarEntry.core;
    }
    // Fall back to first available entry in sidebar order.
    for (final entry in _SidebarEntry.values) {
      if (entry == _SidebarEntry.transform) {
        if (widget.transformCard != null) return entry;
      } else {
        final group = _entryToGroup(entry);
        if (group != null && byGroup[group]?.isNotEmpty == true) return entry;
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
    _recomputePending();
  }

  /// Recompute the pending edits from all dirty fields and notify the parent.
  /// On any validation error, notifies with empty edits + the error message
  /// so the parent can clear the 'heroStats' pending contribution.
  void _recomputePending() {
    final edits = <TypedValueEdit>[];
    for (final attribute in _attributes) {
      for (final (path, original) in [
        (attribute.basePath, attribute.baseValue),
        (attribute.currentPath, attribute.currentValue),
      ]) {
        if (path == null) continue;
        final text = _pending[_pathKey(path)];
        // Untouched fields have no pending entry and are no-ops.
        if (text == null) continue;
        // A cleared field is almost certainly an accident.
        if (text.trim().isEmpty) {
          final errMsg =
              '${attribute.id} is empty — enter a value or restore the '
              'original before saving.';
          setState(() => _error = errMsg);
          widget.onPendingChanged(const [], errMsg);
          return;
        }
        final value = double.tryParse(text.trim());
        if (value == null) {
          final errMsg = 'Invalid number for ${attribute.id}: "$text"';
          setState(() => _error = errMsg);
          widget.onPendingChanged(const [], errMsg);
          return;
        }
        if (value == original) continue;
        edits.add(TypedValueEdit(path: path, value: value));
      }
    }
    // All fields valid. Clear any prior validation error.
    if (_error != null && !_loadFailed) setState(() => _error = null);
    widget.onPendingChanged(edits, null);
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);

    if (_loading) {
      return Card(
        child: const Padding(
          padding: EdgeInsets.all(24),
          child: Center(child: CircularProgressIndicator()),
        ),
      );
    }

    if ((_loadFailed || _attributes.isEmpty) && widget.fallback != null) {
      // The fallback editor has its own save affordances. Keep the error text
      // so the user sees why the typed editors are gone.
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
          // Same order as the legacy stacked path: attributes first.
          widget.fallback!,
          if (widget.transformCard != null) ...[
            const SizedBox(height: 16),
            widget.transformCard!,
          ],
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

    // Build sidebar entries in display order (enum declaration order).
    // transform is included between thieving and advanced when provided.
    final sidebarEntries = <_SidebarEntry>[];
    for (final entry in _SidebarEntry.values) {
      if (entry == _SidebarEntry.transform) {
        if (widget.transformCard != null) sidebarEntries.add(entry);
      } else {
        final group = _entryToGroup(entry);
        if (group != null && byGroup[group]?.isNotEmpty == true) {
          sidebarEntries.add(entry);
        }
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

    // Optional inline validation-error row shown above the detail content.
    final errorRow = _error != null
        ? Padding(
            padding: const EdgeInsets.only(bottom: 8),
            child: Text(
              _error!,
              style: TextStyle(color: theme.colorScheme.error),
            ),
          )
        : null;

    // Build the detail content for the selected entry.
    Widget detailContent;
    if (effectiveSelected == _SidebarEntry.transform) {
      detailContent = Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          ?errorRow,
          widget.transformCard!,
        ],
      );
    } else {
      final group = _entryToGroup(effectiveSelected)!;
      final attributes = byGroup[group] ?? const [];
      detailContent = Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          ?errorRow,
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

    // CrossAxisAlignment.stretch makes both children fill the available height
    // so the sidebar background extends to the bottom regardless of content
    // length, and the right side can use SingleChildScrollView.
    return Row(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        // Left sidebar: ~200px, fixed — never scrolls away with content.
        SizedBox(
          width: 200,
          child: DecoratedBox(
            decoration: BoxDecoration(
              color: theme.colorScheme.surfaceContainerLow,
              borderRadius: BorderRadius.circular(12),
            ),
            // SingleChildScrollView so the sidebar itself can scroll on very
            // small viewports while remaining pinned relative to the detail.
            child: SingleChildScrollView(
              padding: const EdgeInsets.symmetric(vertical: 6),
              child: Column(
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
        ),
        const SizedBox(width: 16),
        // Right detail area: scrolls independently while the sidebar stays put.
        Expanded(
          child: SingleChildScrollView(child: detailContent),
        ),
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
