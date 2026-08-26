import 'package:flutter/material.dart';
import 'package:goresave/l10n/app_localizations.dart';

import '../domain/hero_attributes.dart';
import 'grouped_attribute_sidebar.dart';

/// Describes one entry in the player-tab sidebar. Entries appear in enum
/// declaration order in the sidebar: core, combat, resistances, thieving,
/// advanced.
///
/// The player's transform used to be a sixth entry here. It now lives in the
/// Charaktere → Position sub-tab (PositionDetail), its ONLY home: two mounted
/// copies would both drive the single 'transform' pending key.
enum _SidebarEntry {
  core,
  combat,
  resistances,
  thieving,
  diving,
  sleep,
  intoxication,
  advanced,
}

/// Grouped editors for every hero gameplay attribute. Data arrives through
/// [load] (typed property search) and leaves through [onPendingChanged]
/// (callback fired with the current pending typed edits + any validation
/// error). [reloadKey] identifies the inspected save: when it changes,
/// pending edits are dropped and the card reloads.
///
/// Renders a master-detail layout: a slim left sidebar for navigation and a
/// right detail area showing the selected group's attribute rows.
///
/// Fallback behaviour (typed parse failed or no attributes): renders [fallback]
/// in the legacy stacked layout.
class HeroStatsCard extends StatefulWidget {
  const HeroStatsCard({
    super.key,
    required this.load,
    required this.onPendingChanged,
    required this.editable,
    required this.reloadKey,
    this.initialPending,
    this.fallback,
    this.skillsSection,
    this.attributeLabel,
    this.attributeTooltip,
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

  /// When provided, the learned-skills editor is appended to the "Talente"
  /// (thieving) group's detail — the hero's GameplayEffect skills rendered in
  /// the same row style as attributes. Makes the Talente entry appear even when
  /// the save has no thieving attribute rows.
  final Widget? skillsSection;

  /// Resolves raw save ids to player-facing names. The editor remains usable
  /// without a localization catalog via [heroAttributeLabel].
  final AttributeLabelResolver? attributeLabel;

  /// Resolves an attribute to a one-sentence explanation for its label tooltip.
  final AttributeLabelResolver? attributeTooltip;

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
      final group = _entryToGroup(entry);
      if (group != null && byGroup[group]?.isNotEmpty == true) return entry;
    }
    return null;
  }

  Map<HeroAttributeGroup, List<HeroAttribute>> _byGroup(
    List<HeroAttribute> attributes,
  ) {
    final byGroup = <HeroAttributeGroup, List<HeroAttribute>>{};
    for (final attribute in attributes) {
      byGroup
          .putIfAbsent(
            heroAttributeGroup(attribute.id, attribute.setClass),
            () => [],
          )
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
          final errMsg = l10n.attributeEmpty(_displayLabel(attribute));
          setState(() => _error = errMsg);
          widget.onPendingChanged(const [], errMsg);
          return;
        }
        final value = double.tryParse(text.trim());
        if (value == null) {
          final errMsg = l10n.attributeInvalidNumber(
            _displayLabel(attribute),
            text,
          );
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

    if ((_loadFailed || _attributes.isEmpty) && widget.skillsSection != null) {
      // No stats and no fallback — just show the skills editor (which loads
      // independently of the typed attribute search).
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
            if (widget.skillsSection != null) widget.skillsSection!,
          ],
        ),
      );
    }

    // --- Sidebar layout ---
    final byGroup = _byGroup(_attributes);

    // Build sidebar entries in display order (enum declaration order).
    final sidebarEntries = <_SidebarEntry>[];
    for (final entry in _SidebarEntry.values) {
      if (entry == _SidebarEntry.thieving) {
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
      // The rows render bare — the tab body already provides the single main
      // card, and the selected sidebar tile already names the group (no inner
      // card, no duplicate group title).
      final group = _entryToGroup(entry)!;
      // The "Talente" (thieving) pane shows ONLY the learned-skills editor — the
      // raw thieving attribute values (LockpickDurability/Precision,
      // PickPocketing) are intentionally not surfaced here; they remain editable
      // in the All-data browser. Every other group renders its attribute rows.
      final isSkillsPane = entry == _SidebarEntry.thieving;
      final attributes = isSkillsPane ? const [] : (byGroup[group] ?? const []);
      final skills = isSkillsPane ? widget.skillsSection : null;
      // Breath is the one group where an edit can silently fail to survive: the
      // player definition carries its own oxygen values for the Diving skill's
      // tag, and the game applies those on every load. Say so where the fields
      // are, rather than let people find out two savegames later.
      final note = entry == _SidebarEntry.diving
          ? _GroupNote(text: AppLocalizations.of(context).heroDivingSkillNote)
          : null;
      return Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          ?errorRow,
          ?note,
          for (final a in attributes) _row(a),
          ?skills,
        ],
      );
    }

    // Delegate the master-detail shell to the shared GroupedAttributeSidebar.
    // Each sidebar entry becomes a pane keyed by the _SidebarEntry enum value;
    // panes stay mounted (Offstage) inside the shell so a pane's unsaved field
    // drafts — which back a registered pending edit — survive switching
    // entries.
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
      _SidebarEntry.diving => HeroAttributeGroup.diving,
      _SidebarEntry.sleep => HeroAttributeGroup.sleep,
      _SidebarEntry.intoxication => HeroAttributeGroup.intoxication,
      _SidebarEntry.advanced => HeroAttributeGroup.advanced,
    };
  }

  String _displayLabel(HeroAttribute attribute) =>
      widget.attributeLabel?.call(attribute.id, attribute.setClass) ??
      heroAttributeLabel(attribute.id);

  String _entryLabel(AppLocalizations l10n, _SidebarEntry entry) {
    return switch (entry) {
      _SidebarEntry.core => l10n.heroGroupMainStats,
      _SidebarEntry.combat => l10n.heroGroupCombatMovement,
      _SidebarEntry.resistances => l10n.heroGroupResistances,
      _SidebarEntry.thieving => l10n.heroGroupSkills,
      _SidebarEntry.diving => l10n.heroGroupDiving,
      _SidebarEntry.sleep => l10n.heroGroupSleep,
      _SidebarEntry.intoxication => l10n.heroGroupIntoxication,
      _SidebarEntry.advanced => l10n.heroGroupAdvanced,
    };
  }

  IconData _entryIcon(_SidebarEntry entry) {
    return switch (entry) {
      _SidebarEntry.core => Icons.favorite_border,
      _SidebarEntry.combat => Icons.shield_outlined,
      _SidebarEntry.resistances => Icons.security_outlined,
      _SidebarEntry.thieving => Icons.military_tech_outlined,
      _SidebarEntry.diving => Icons.scuba_diving_outlined,
      _SidebarEntry.sleep => Icons.bedtime_outlined,
      _SidebarEntry.intoxication => Icons.local_bar_outlined,
      _SidebarEntry.advanced => Icons.tune,
    };
  }

  Widget _row(HeroAttribute attribute) {
    final label = _displayLabel(attribute);
    final tooltip =
        widget.attributeTooltip?.call(attribute.id, attribute.setClass) ?? '';
    return _HeroAttributeRow(
      // Record key compares reloadKey by its own equality (identity for
      // SaveInspection, which has no == override), not by toString(), so a
      // fresh SaveInspection instance always causes a new row to be built
      // rather than reusing stale field state from the previous load.
      key: ValueKey((widget.reloadKey, attribute.setClass, attribute.id)),
      tooltip: tooltip,
      attribute: attribute,
      label: label,
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

/// A quiet line above an attribute group: something about these values the user
/// cannot see in the numbers themselves. Deliberately not a card — it explains,
/// it does not warn.
class _GroupNote extends StatelessWidget {
  const _GroupNote({required this.text});

  final String text;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Padding(
      padding: const EdgeInsets.only(bottom: 12),
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Icon(
            Icons.info_outline,
            size: 15,
            color: theme.colorScheme.onSurfaceVariant,
          ),
          const SizedBox(width: 8),
          Expanded(
            child: Text(
              text,
              style: theme.textTheme.bodySmall?.copyWith(
                color: theme.colorScheme.onSurfaceVariant,
              ),
            ),
          ),
        ],
      ),
    );
  }
}

class _HeroAttributeRow extends StatefulWidget {
  const _HeroAttributeRow({
    super.key,
    required this.attribute,
    required this.label,
    this.tooltip = '',
    required this.editable,
    required this.onBaseChanged,
    required this.onCurrentChanged,
    this.initialBaseText,
    this.initialCurrentText,
  });

  final HeroAttribute attribute;
  final String label;

  /// One sentence on what this value does in the game. Empty = no tooltip.
  final String tooltip;
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

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.symmetric(vertical: 5),
      child: LayoutBuilder(
        builder: (context, constraints) {
          final compact = constraints.maxWidth < 620;
          final baseField = TextField(
            key: ValueKey(
              'hero-attribute:${widget.attribute.setClass}:'
              '${widget.attribute.id}:base',
            ),
            controller: _baseController,
            enabled: widget.editable && widget.attribute.basePath != null,
            onChanged: widget.onBaseChanged,
            keyboardType: const TextInputType.numberWithOptions(
              decimal: true,
              signed: true,
            ),
            decoration: InputDecoration(
              labelText: AppLocalizations.of(context).attributeBaseValue,
            ),
          );
          final currentField = TextField(
            key: ValueKey(
              'hero-attribute:${widget.attribute.setClass}:'
              '${widget.attribute.id}:current',
            ),
            controller: _currentController,
            enabled: widget.editable && widget.attribute.currentPath != null,
            onChanged: widget.onCurrentChanged,
            keyboardType: const TextInputType.numberWithOptions(
              decimal: true,
              signed: true,
            ),
            decoration: InputDecoration(
              labelText: AppLocalizations.of(context).attributeCurrentValue,
            ),
          );
          // The label carries the explanation of what the value does in the
          // game; rows we have nothing to say about stay plain text.
          final Widget rowLabel = widget.tooltip.isEmpty
              ? Text(
                  widget.label,
                  style: Theme.of(context).textTheme.labelLarge,
                )
              : Tooltip(
                  message: widget.tooltip,
                  child: Text(
                    widget.label,
                    style: Theme.of(context).textTheme.labelLarge,
                  ),
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
