import 'package:flutter/material.dart';
import 'package:goresave/l10n/app_localizations.dart';

import '../domain/hero_attributes.dart';
import 'grouped_attribute_sidebar.dart';

/// Describes one entry in the player-tab sidebar. Entries appear in enum
/// declaration order in the sidebar: core, combat, resistances, thieving,
/// transform (when present), advanced.
enum _SidebarEntry { core, combat, resistances, thieving, transform, advanced }

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
    this.initialPending,
    this.fallback,
    this.transformCard,
    this.skillsSection,
  });

  final Future<HeroAttributesResult> Function() load;

  /// Drafts already queued for the player (the 'heroStats' pending entry),
  /// reconstructed by the parent. Seeds the local fields on (re)load so that
  /// returning to the Player after selecting an NPC resumes from the unsaved
  /// edits instead of showing on-disk values (which the next edit would
  /// otherwise overwrite, dropping the earlier ones).
  final List<TypedValueEdit> Function()? initialPending;

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

  /// When provided, the learned-skills editor is appended to the "Talente"
  /// (thieving) group's detail — the hero's GameplayEffect skills rendered in
  /// the same row style as attributes. Makes the Talente entry appear even when
  /// the save has no thieving attribute rows.
  final Widget? skillsSection;

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
      // Rehydrate any queued player drafts so a revisit resumes from them. Only
      // local fields are seeded here; the registry entry already exists, so we
      // must NOT call onPendingChanged from this build-context reload.
      for (final draft
          in widget.initialPending?.call() ?? const <TypedValueEdit>[]) {
        final v = draft.value;
        _pending[_pathKey(draft.path)] = v is num
            ? formatHeroValue(v.toDouble())
            : '$v';
      }
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
    final l10n = AppLocalizations.of(context);
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
          final errMsg = l10n.attributeEmpty(attribute.id);
          setState(() => _error = errMsg);
          widget.onPendingChanged(const [], errMsg);
          return;
        }
        final value = double.tryParse(text.trim());
        if (value == null) {
          final errMsg = l10n.attributeInvalidNumber(attribute.id, text);
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
      // The caller supplies the surrounding card (single-card tab layout).
      return const Padding(
        padding: EdgeInsets.all(24),
        child: Center(child: CircularProgressIndicator()),
      );
    }

    if ((_loadFailed || _attributes.isEmpty) && widget.fallback != null) {
      // The fallback editor has its own save affordances. Keep the error text
      // so the user sees why the typed editors are gone. The caller gives
      // this widget the full pane height (no outer ListView), so the stacked
      // fallback must scroll on its own or tall content overflows the tab.
      return SingleChildScrollView(
        child: Column(
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
            // Skills load independently of the typed attribute search, so keep
            // them visible even when attributes fell back to the legacy editor.
            if (widget.skillsSection != null) ...[
              const SizedBox(height: 16),
              widget.skillsSection!,
            ],
          ],
        ),
      );
    }

    if ((_loadFailed || _attributes.isEmpty) &&
        (widget.transformCard != null || widget.skillsSection != null)) {
      // No stats and no fallback — just show transform and/or skills (which
      // load independently of the typed attribute search).
      return SingleChildScrollView(
        child: Column(
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
            if (widget.transformCard != null) widget.transformCard!,
            if (widget.skillsSection != null) ...[
              if (widget.transformCard != null) const SizedBox(height: 16),
              widget.skillsSection!,
            ],
          ],
        ),
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
      } else if (entry == _SidebarEntry.thieving) {
        // "Talente": show whenever there are thieving attributes OR a skills
        // editor to host, so a hero with no thieving attribute rows still gets
        // the skills entry.
        if (byGroup[HeroAttributeGroup.thieving]?.isNotEmpty == true ||
            widget.skillsSection != null) {
          sidebarEntries.add(entry);
        }
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

    // Build the detail content for one entry.
    Widget detailFor(_SidebarEntry entry) {
      if (entry == _SidebarEntry.transform) {
        return Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [?errorRow, widget.transformCard!],
        );
      }
      // The rows render bare — the tab body already provides the single main
      // card, and the selected sidebar tile already names the group (no inner
      // card, no duplicate group title).
      final group = _entryToGroup(entry)!;
      final attributes = byGroup[group] ?? const [];
      // The "Talente" (thieving) pane also hosts the learned-skills editor,
      // appended below any thieving attribute rows.
      final skills =
          entry == _SidebarEntry.thieving ? widget.skillsSection : null;
      return Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          ?errorRow,
          for (final a in attributes) _row(a),
          if (skills != null) ...[
            if (attributes.isNotEmpty) const Divider(height: 24),
            skills,
          ],
        ],
      );
    }

    // Delegate the master-detail shell to the shared GroupedAttributeSidebar.
    // Each sidebar entry becomes a pane keyed by the _SidebarEntry enum value;
    // panes stay mounted (Offstage) inside the shell so the transform editor's
    // unsaved field drafts — which back a registered pending edit — survive
    // switching entries.
    return GroupedAttributeSidebar(
      selected: effectiveSelected,
      onSelect: (id) {
        final entry = id as _SidebarEntry;
        if (_selected != entry) setState(() => _selected = entry);
      },
      panes: [
        for (final entry in sidebarEntries)
          SidebarPane(
            id: entry,
            label: _entryLabel(AppLocalizations.of(context), entry),
            icon: _entryIcon(entry),
            detail: detailFor(entry),
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

  String _entryLabel(AppLocalizations l10n, _SidebarEntry entry) {
    return switch (entry) {
      _SidebarEntry.transform => l10n.heroEntryHeroTransform,
      _SidebarEntry.core => l10n.heroGroupMainStats,
      _SidebarEntry.combat => l10n.heroGroupCombatSkills,
      _SidebarEntry.resistances => l10n.heroGroupResistances,
      _SidebarEntry.thieving => l10n.heroGroupSkills,
      _SidebarEntry.advanced => l10n.heroGroupAdvanced,
    };
  }

  IconData _entryIcon(_SidebarEntry entry) {
    return switch (entry) {
      _SidebarEntry.transform => Icons.explore_outlined,
      _SidebarEntry.core => Icons.favorite_border,
      _SidebarEntry.combat => Icons.shield_outlined,
      _SidebarEntry.resistances => Icons.security_outlined,
      _SidebarEntry.thieving => Icons.military_tech_outlined,
      _SidebarEntry.advanced => Icons.tune,
    };
  }

  Widget _row(HeroAttribute attribute) {
    final duplicate = _attributes.where((a) => a.id == attribute.id).length > 1;
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
      onCurrentChanged: (text) => _onFieldChanged(attribute.currentPath, text),
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
      text:
          widget.initialBaseText ?? formatHeroValue(widget.attribute.baseValue),
    );
    _currentController = TextEditingController(
      text:
          widget.initialCurrentText ??
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
            decoration: InputDecoration(
              labelText: AppLocalizations.of(context).attributeBase(_label),
            ),
          );
          final currentField = TextField(
            controller: _currentController,
            enabled: widget.editable && widget.attribute.currentPath != null,
            onChanged: widget.onCurrentChanged,
            keyboardType: const TextInputType.numberWithOptions(
              decimal: true,
              signed: true,
            ),
            decoration: InputDecoration(
              labelText: AppLocalizations.of(context).attributeCurrent(_label),
            ),
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
  final rounded = value.toStringAsFixed(2).replaceFirst(RegExp(r'\.?0+$'), '');
  // An editable field's text becomes the saved value, so never seed it with
  // a lossy rounding (0.125 must not display — and then save — as 0.13).
  return double.tryParse(rounded) == value ? rounded : value.toString();
}
