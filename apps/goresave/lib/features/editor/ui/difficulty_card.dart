import 'package:flutter/material.dart';
import 'package:goresave/features/editor/domain/editor_models.dart';
import 'package:goresave/features/editor/domain/editor_notifier.dart';
import 'package:goresave/features/editor/domain/pending_edits.dart';

/// The four difficulty presets, in the order the in-game screen lists them.
/// These are the exact label strings the core maps back to its class names
/// (Novice→_Easy, Gothic→_Standard, Hard→_Hard, Custom→_Custom).
const _presets = ['Novice', 'Gothic', 'Hard', 'Custom'];

/// The three sub-level options shown for Combat / Resources / Progression.
const _levels = ['Novice', 'Gothic', 'Hard'];

/// Maps a preset to the implied sub-level shown when the pickers are locked
/// (i.e. preset != Custom). Custom has no implied level — the pickers stay at
/// whatever the user/draft holds.
String _impliedLevelForPreset(String preset) {
  switch (preset) {
    case 'Novice':
      return 'Novice';
    case 'Hard':
      return 'Hard';
    case 'Gothic':
    default:
      return 'Gothic';
  }
}

/// Editable difficulty form mirroring the in-game difficulty screen, bound to
/// the selected save's stored difficulty.
///
/// Difficulty is a normal **pending edit**: changing a control registers (or
/// updates) a [PendingDifficulty] on the notifier; reverting to the stored
/// value with no propagation box ticked clears it. It is saved by the GLOBAL
/// toolbar Save (`saveAllPending` dispatches `write_difficulty`) and discarded
/// by the GLOBAL Reset — exactly like hero/inventory/metadata edits. The card
/// therefore has no own Save/Reset buttons.
///
/// The displayed control values come from `pendingDifficulty ?? stored`, so a
/// global Reset (which clears `pendingDifficulty`) makes the card show the
/// saved values again, and switching saves re-seeds correctly.
///
/// Difficulty is stored redundantly: in each save (gameplay-relevant), and in
/// the profile's `PersistentDataList.sav` (the new-game menu default). Editing
/// here changes only the current save unless the propagation checkboxes are
/// ticked.
class DifficultyCard extends StatefulWidget {
  const DifficultyCard({
    super.key,
    required this.inspection,
    required this.notifier,
    required this.profile,
    required this.canCompress,
  });

  final SaveInspection inspection;
  final EditorNotifier notifier;

  /// The profile of the SAVE under edit (resolved from the selected save's
  /// `persistentProfileId`). Null when the edited save has no resolvable profile
  /// — the propagation checkboxes are then disabled because there is no profile
  /// to write to. NOTE: this is the edited save's profile, NOT the sidebar's
  /// effective filter profile.
  final ProfileSummary? profile;

  /// Whether the codec host is compress-ready. Difficulty lives in the private
  /// payload of each targeted save, so writing it recompresses the payload and
  /// requires a verified/trusted codec — same gate as the other private edits.
  /// The card no longer blocks on this (the global Save surfaces the codec
  /// error); it is kept for a non-blocking hint.
  final bool canCompress;

  @override
  State<DifficultyCard> createState() => _DifficultyCardState();
}

class _DifficultyCardState extends State<DifficultyCard> {
  // Expanded by default so the editing form is visible on load.
  bool _expanded = true;

  EditorState get _state => widget.notifier.state;

  PendingDifficulty? get _pending => _state.pendingDifficulty;

  // --- Stored (saved) values, normalised to UI labels --------------------

  String get _storedPreset => _normalizePreset(widget.inspection.difficulty?.presetLabel);

  bool get _storedFlow => widget.inspection.difficulty?.flowHelper ?? false;

  bool get _storedPerma {
    return _storedPreset == 'Novice'
        ? false
        : (widget.inspection.difficulty?.permadeath ?? false);
  }

  // --- Displayed values: pending edit when present, else stored ----------

  String get _preset {
    final d = _pending?.difficulty;
    if (d == null) return _storedPreset;
    // The card stores 'preset' as a UI label ('Novice'/'Gothic'/'Hard'/
    // 'Custom') — exactly what the core's write_difficulty expects.
    return _normalizePreset(d['preset'] as String?);
  }

  bool get _flow {
    final d = _pending?.difficulty;
    if (d == null) return _storedFlow;
    return (d['flowHelper'] as bool?) ?? false;
  }

  bool get _perma {
    final d = _pending?.difficulty;
    if (d == null) return _storedPerma;
    if (_preset == 'Novice') return false;
    return (d['permadeath'] as bool?) ?? false;
  }

  /// Sub-level for a Custom picker. When a pending Custom edit exists, read its
  /// stored level; otherwise fall back to the saved value normalised against
  /// the preset.
  String _customLevel(String field) {
    final d = _pending?.difficulty;
    if (d != null && d[field] is String) {
      return _normalizeLevel(d[field] as String, 'Custom');
    }
    final stored = widget.inspection.difficulty;
    final label = switch (field) {
      'combat' => stored?.combatLabel,
      'resources' => stored?.resourcesLabel,
      _ => stored?.progressionLabel,
    };
    return _normalizeLevel(label, _storedPreset);
  }

  bool get _alsoProfile => _pending?.alsoProfile ?? false;
  bool get _allSaves => _pending?.allSaves ?? false;

  // --- Label/normalisation helpers ---------------------------------------

  String _normalizePreset(String? label) =>
      _presets.contains(label) ? label! : 'Gothic';

  String _normalizeLevel(String? label, String preset) {
    if (_levels.contains(label)) return label!;
    return preset == 'Custom' ? 'Gothic' : _impliedLevelForPreset(preset);
  }

  /// The level a picker should DISPLAY: the draft value when Custom (pickers
  /// editable), otherwise the level implied by the preset (pickers locked).
  String _displayedLevel(String field) {
    return _preset == 'Custom'
        ? _customLevel(field)
        : _impliedLevelForPreset(_preset);
  }

  // --- Pending-edit registration -----------------------------------------

  /// Recompute the pending difficulty from a candidate set of control values
  /// and the current propagation flags, then either register it or — when it
  /// matches the stored value AND no propagation box is ticked — clear it.
  void _apply({
    String? preset,
    bool? flow,
    bool? perma,
    String? combat,
    String? resources,
    String? progression,
    bool? alsoProfile,
    bool? allSaves,
  }) {
    final nextPreset = preset ?? _preset;
    // Novice forces permadeath off; otherwise carry the explicit toggle or the
    // current displayed value.
    final nextPerma = nextPreset == 'Novice' ? false : (perma ?? _perma);
    final nextFlow = flow ?? _flow;

    // Custom sub-levels: only meaningful (and editable) on Custom. When leaving
    // Custom, snap them to the implied preset level so a later return to Custom
    // starts coherent.
    final String nextCombat;
    final String nextResources;
    final String nextProgression;
    if (nextPreset == 'Custom') {
      nextCombat = combat ?? _customLevel('combat');
      nextResources = resources ?? _customLevel('resources');
      nextProgression = progression ?? _customLevel('progression');
    } else {
      final implied = _impliedLevelForPreset(nextPreset);
      nextCombat = implied;
      nextResources = implied;
      nextProgression = implied;
    }

    final nextAlsoProfile =
        (alsoProfile ?? _alsoProfile) && widget.profile != null;
    final nextAllSaves = (allSaves ?? _allSaves) && widget.profile != null;

    // Does this match the stored value with no propagation? Then there is no
    // pending work — clear it. Sub-levels only matter on Custom.
    final matchesStored =
        nextPreset == _storedPreset &&
        nextFlow == _storedFlow &&
        nextPerma == _storedPerma &&
        (nextPreset != 'Custom' ||
            (nextCombat == _customLevel('combat') &&
                nextResources == _customLevel('resources') &&
                nextProgression == _customLevel('progression')));

    if (matchesStored && !nextAlsoProfile && !nextAllSaves) {
      widget.notifier.clearPendingDifficulty();
      setState(() {});
      return;
    }

    final difficulty = <String, Object?>{
      'preset': nextPreset,
      if (nextPreset == 'Custom') 'combat': nextCombat,
      if (nextPreset == 'Custom') 'resources': nextResources,
      if (nextPreset == 'Custom') 'progression': nextProgression,
      'flowHelper': nextFlow,
      'permadeath': nextPerma,
    };
    widget.notifier.setPendingDifficulty(
      PendingDifficulty(
        difficulty: difficulty,
        alsoProfile: nextAlsoProfile,
        allSaves: nextAllSaves,
      ),
    );
    setState(() {});
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final scheme = theme.colorScheme;
    final hasProfile = widget.profile != null;
    final hasWork = _pending != null;
    final canWrite = widget.canCompress;
    final busy = _state.isLoading;

    final presetEnabled = !busy;
    final permaEnabled = _preset != 'Novice' && !busy;
    final levelsEnabled = _preset == 'Custom' && !busy;

    return Card(
      child: Padding(
        padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 8),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            InkWell(
              onTap: () => setState(() => _expanded = !_expanded),
              borderRadius: BorderRadius.circular(8),
              child: Padding(
                padding: const EdgeInsets.symmetric(vertical: 4),
                child: Row(
                  children: [
                    const Icon(Icons.local_fire_department_outlined),
                    const SizedBox(width: 8),
                    Expanded(
                      child: Column(
                        crossAxisAlignment: CrossAxisAlignment.start,
                        mainAxisSize: MainAxisSize.min,
                        children: [
                          Text(
                            'Difficulty',
                            style: theme.textTheme.titleMedium,
                          ),
                          Text(
                            _preset,
                            style: theme.textTheme.bodySmall,
                          ),
                        ],
                      ),
                    ),
                    if (hasWork)
                      Padding(
                        padding: const EdgeInsets.only(right: 8),
                        child: Text(
                          'Unsaved',
                          style: theme.textTheme.labelSmall?.copyWith(
                            color: scheme.primary,
                          ),
                        ),
                      ),
                    Icon(_expanded ? Icons.expand_less : Icons.expand_more),
                  ],
                ),
              ),
            ),
            if (_expanded) ...[
              const SizedBox(height: 8),
              // Preset selector.
              Text('Preset', style: theme.textTheme.labelLarge),
              const SizedBox(height: 6),
              SegmentedButton<String>(
                segments: [
                  for (final preset in _presets)
                    ButtonSegment<String>(value: preset, label: Text(preset)),
                ],
                selected: {_preset},
                showSelectedIcon: false,
                onSelectionChanged: presetEnabled
                    ? (selection) => _apply(preset: selection.first)
                    : null,
              ),
              const SizedBox(height: 8),
              // Toggles.
              SwitchListTile(
                contentPadding: EdgeInsets.zero,
                dense: true,
                title: const Text('Close Combat Flow Helper'),
                value: _flow,
                onChanged: busy
                    ? null
                    : (value) => _apply(flow: value),
              ),
              SwitchListTile(
                contentPadding: EdgeInsets.zero,
                dense: true,
                title: const Text('Permadeath'),
                subtitle: _preset == 'Novice'
                    ? const Text('Not available on Novice')
                    : null,
                value: _perma,
                onChanged: permaEnabled
                    ? (value) => _apply(perma: value)
                    : null,
              ),
              const SizedBox(height: 8),
              // Level pickers.
              _LevelPicker(
                label: 'Combat',
                value: _displayedLevel('combat'),
                enabled: levelsEnabled,
                onChanged: (value) => _apply(combat: value),
              ),
              const SizedBox(height: 8),
              _LevelPicker(
                label: 'Resources',
                value: _displayedLevel('resources'),
                enabled: levelsEnabled,
                onChanged: (value) => _apply(resources: value),
              ),
              const SizedBox(height: 8),
              _LevelPicker(
                label: 'Progression',
                value: _displayedLevel('progression'),
                enabled: levelsEnabled,
                onChanged: (value) => _apply(progression: value),
              ),
              const SizedBox(height: 16),
              // Explanation.
              Container(
                padding: const EdgeInsets.all(12),
                decoration: BoxDecoration(
                  color: scheme.surfaceContainerHighest,
                  borderRadius: BorderRadius.circular(8),
                ),
                child: Text(
                  'Difficulty is stored in this save (gameplay-relevant), in the '
                  'profile (the new-game menu default), and separately in every '
                  'other save. Editing here changes only the current save unless '
                  'you tick the options below. Use the toolbar Save to write your '
                  'changes.',
                  style: theme.textTheme.bodySmall,
                ),
              ),
              const SizedBox(height: 8),
              // Propagation checkboxes (bound to the EDITED SAVE's profile).
              CheckboxListTile(
                contentPadding: EdgeInsets.zero,
                dense: true,
                controlAffinity: ListTileControlAffinity.leading,
                title: const Text('Also update the profile'),
                subtitle: hasProfile
                    ? null
                    : const Text('No resolved profile to update'),
                value: _alsoProfile,
                onChanged: hasProfile && !busy
                    ? (value) => _apply(alsoProfile: value ?? false)
                    : null,
              ),
              CheckboxListTile(
                contentPadding: EdgeInsets.zero,
                dense: true,
                controlAffinity: ListTileControlAffinity.leading,
                title: const Text('Apply to all saves of this profile'),
                subtitle: hasProfile
                    ? null
                    : const Text('No resolved profile to apply to'),
                value: _allSaves,
                onChanged: hasProfile && !busy
                    ? (value) => _apply(allSaves: value ?? false)
                    : null,
              ),
              if (hasWork && !canWrite)
                Padding(
                  padding: const EdgeInsets.only(top: 8),
                  child: Text(
                    'Saving difficulty needs a verified G1R codec host '
                    '(it rewrites the private payload of each targeted save).',
                    style: theme.textTheme.bodySmall?.copyWith(
                      color: scheme.error,
                    ),
                  ),
                ),
            ],
          ],
        ),
      ),
    );
  }
}

/// One labelled Novice/Gothic/Hard segmented picker. Disabled state reflects
/// the value but ignores taps.
class _LevelPicker extends StatelessWidget {
  const _LevelPicker({
    required this.label,
    required this.value,
    required this.enabled,
    required this.onChanged,
  });

  final String label;
  final String value;
  final bool enabled;
  final ValueChanged<String> onChanged;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Row(
      children: [
        SizedBox(
          width: 110,
          child: Text(label, style: theme.textTheme.bodyMedium),
        ),
        Expanded(
          child: SegmentedButton<String>(
            segments: [
              for (final level in _levels)
                ButtonSegment<String>(value: level, label: Text(level)),
            ],
            selected: {value},
            showSelectedIcon: false,
            onSelectionChanged:
                enabled ? (selection) => onChanged(selection.first) : null,
          ),
        ),
      ],
    );
  }
}
