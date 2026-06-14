import 'package:flutter/material.dart';
import 'package:goresave/features/editor/domain/editor_models.dart';
import 'package:goresave/features/editor/domain/editor_notifier.dart';
import 'package:path/path.dart' as p;

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
/// the selected save's stored difficulty. Writes through
/// [EditorNotifier.writeDifficulty] with optional propagation to the profile
/// and to all of the profile's saves (always with a backup).
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

  /// The effective profile (`EditorState.activeProfile`). Null when no profile
  /// is resolved — the propagation checkboxes are then disabled because there
  /// is no profile to write to.
  final ProfileSummary? profile;

  /// Whether the codec host is compress-ready. Difficulty lives in the private
  /// payload of each targeted save, so writing it recompresses the payload and
  /// requires a verified/trusted codec — same gate as the other private edits.
  final bool canCompress;

  @override
  State<DifficultyCard> createState() => _DifficultyCardState();
}

class _DifficultyCardState extends State<DifficultyCard> {
  // Collapsed by default so the Overview panel opens to the same compact
  // layout as before — the editing form (and its own Reset/Save buttons)
  // appears only when the user expands the card.
  bool _expanded = false;

  // Draft state — label strings ('Novice'/'Gothic'/'Hard'/'Custom').
  late String _preset;
  late String _combat;
  late String _resources;
  late String _progression;
  late bool _flow;
  late bool _perma;

  // Propagation checkboxes (default OFF).
  bool _alsoProfile = false;
  bool _allSaves = false;

  // Identity of the save the draft was seeded from (the inspection path), so we
  // re-seed only when a DIFFERENT save lands — not on every incidental
  // re-inspect of the same save. A null path (no save) is its own identity.
  Object? _seededFromPath;

  bool _saving = false;

  @override
  void initState() {
    super.initState();
    _seed();
  }

  @override
  void didUpdateWidget(covariant DifficultyCard oldWidget) {
    super.didUpdateWidget(oldWidget);
    // Re-seed only when a DIFFERENT save lands (path changed), when the current
    // draft has no unsaved work, OR when the notifier reports the difficulty is
    // no longer dirty. The notifier is the source of truth for "keep this
    // draft": it preserves difficultyDirty across an incidental re-inspect of
    // the SAME save (e.g. a toolbar Save of hero edits) so the draft survives,
    // but clears it on a genuine save switch, after a successful difficulty
    // write, and when the user confirms a discard-and-rescan — in which case we
    // re-seed to the stored value. We must NOT call setDifficultyDirty from
    // here (it would mutate the provider during build/didUpdateWidget).
    final samePath = widget.inspection.path == _seededFromPath;
    final notifierDirty = widget.notifier.state.difficultyDirty;
    if (!samePath || !_hasWork || !notifierDirty) {
      _seed();
      // If the re-seed lands on a draft with no work but the notifier flag is
      // still set, the flag is now stale (the card shows no "Unsaved" state),
      // and it would wedge the profile-switch / rescan guards until restart.
      // Clear it safely AFTER this frame — mutating the provider during
      // build/didUpdateWidget is not allowed. Only clear when there is genuinely
      // no work, so a still-dirty same-save draft is never silently dropped.
      if (!_hasWork && widget.notifier.state.difficultyDirty) {
        WidgetsBinding.instance.addPostFrameCallback((_) {
          if (mounted && !_hasWork) widget.notifier.setDifficultyDirty(false);
        });
      }
    }
  }

  /// Seed (or reset) the draft from the stored difficulty. Falls back to a
  /// Gothic baseline when the save has no difficulty block at all.
  void _seed() {
    final d = widget.inspection.difficulty;
    final preset = _normalizePreset(d?.presetLabel);
    setState(() {
      _seededFromPath = widget.inspection.path;
      _preset = preset;
      _combat = _normalizeLevel(d?.combatLabel, preset);
      _resources = _normalizeLevel(d?.resourcesLabel, preset);
      _progression = _normalizeLevel(d?.progressionLabel, preset);
      _flow = d?.flowHelper ?? false;
      // Novice forces permadeath off; otherwise honour the stored value.
      _perma = preset == 'Novice' ? false : (d?.permadeath ?? false);
      _alsoProfile = false;
      _allSaves = false;
    });
  }

  String _normalizePreset(String? label) {
    return _presets.contains(label) ? label! : 'Gothic';
  }

  String _normalizeLevel(String? label, String preset) {
    if (_levels.contains(label)) return label!;
    // No usable stored sub-level: derive from the preset so locked pickers show
    // a sensible value.
    return preset == 'Custom' ? 'Gothic' : _impliedLevelForPreset(preset);
  }

  /// The level a picker should DISPLAY: the draft value when Custom (pickers
  /// editable), otherwise the level implied by the preset (pickers locked).
  String _displayedLevel(String draftLevel) {
    return _preset == 'Custom' ? draftLevel : _impliedLevelForPreset(_preset);
  }

  /// Effective permadeath for dirty-comparison and save: forced off on Novice.
  bool get _effectivePerma => _preset == 'Novice' ? false : _perma;

  /// Whether the draft differs from the stored value (drives Save/Reset).
  bool get _dirty {
    final d = widget.inspection.difficulty;
    final storedPreset = _normalizePreset(d?.presetLabel);
    if (_preset != storedPreset) return true;
    if (_flow != (d?.flowHelper ?? false)) return true;
    final storedPerma = storedPreset == 'Novice'
        ? false
        : (d?.permadeath ?? false);
    if (_effectivePerma != storedPerma) return true;
    // Sub-levels only matter when Custom — for the other presets the level is
    // fully implied by the preset, so a stored sub-level mismatch is not a
    // user-visible difference.
    if (_preset == 'Custom') {
      if (_combat != _normalizeLevel(d?.combatLabel, storedPreset)) return true;
      if (_resources != _normalizeLevel(d?.resourcesLabel, storedPreset)) {
        return true;
      }
      if (_progression != _normalizeLevel(d?.progressionLabel, storedPreset)) {
        return true;
      }
    }
    return false;
  }

  /// True when there is work to save: a changed field OR a ticked propagation
  /// box (the user may want to push the current, unchanged difficulty to the
  /// profile / other saves). Drives Save enablement and the unsaved-edits guard.
  bool get _hasWork => _dirty || _alsoProfile || _allSaves;

  void _syncDirty() {
    // Report "has work" (not just field-dirty) so the profile-switch /
    // rescan guard also protects a propagation-only intent.
    widget.notifier.setDifficultyDirty(_hasWork);
  }

  void _onPresetChanged(String preset) {
    setState(() {
      _preset = preset;
      if (preset == 'Novice') _perma = false;
      // Leaving Custom: snap the displayed sub-levels to the preset so the
      // locked pickers reflect it. Entering Custom keeps the current values as
      // the editable starting point.
      if (preset != 'Custom') {
        final implied = _impliedLevelForPreset(preset);
        _combat = implied;
        _resources = implied;
        _progression = implied;
      }
    });
    _syncDirty();
  }

  void _reset() {
    _seed();
    widget.notifier.setDifficultyDirty(false);
  }

  Future<void> _save() async {
    final path = widget.inspection.path;
    if (path == null) return;
    final profile = widget.profile;

    final savePaths = <String>[path];
    if (_allSaves && profile != null) {
      savePaths
        ..clear()
        ..addAll(
          widget.notifier.state.saves
              .where((s) => s.persistentProfileId == profile.profileId)
              .map((s) => s.path),
        );
      // Ensure the current save is included even if its persistentProfileId is
      // null or mismatched.
      if (!savePaths.contains(path)) savePaths.add(path);
    }

    final difficulty = <String, Object?>{
      'preset': _preset,
      if (_preset == 'Custom') 'combat': _combat,
      if (_preset == 'Custom') 'resources': _resources,
      if (_preset == 'Custom') 'progression': _progression,
      'flowHelper': _flow,
      'permadeath': _effectivePerma,
    };

    final ({String path, int profileId})? target =
        (_alsoProfile && profile != null)
        ? (path: _persistentDataListPath(path), profileId: profile.profileId)
        : null;

    setState(() => _saving = true);
    try {
      await widget.notifier.writeDifficulty(
        difficulty: difficulty,
        savePaths: savePaths,
        profile: target,
      );
    } finally {
      if (mounted) setState(() => _saving = false);
    }
  }

  /// The profile's `PersistentDataList.sav` lives next to the save files, in
  /// the same directory. Derive it from the current save's directory.
  String _persistentDataListPath(String savePath) {
    return p.join(p.dirname(savePath), 'PersistentDataList.sav');
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final scheme = theme.colorScheme;
    final hasProfile = widget.profile != null;
    final hasWork = _hasWork;
    // A difficulty write does a full re-inspect that re-seeds every editor and
    // clears the pending-edit registry, which would silently discard unrelated
    // unsaved hero/inventory/metadata edits. Mirror the app's "mutually
    // exclusive edits" pattern (see structural inventory edits): block the
    // Difficulty save while other pending edits exist, with a clear hint.
    final blockingPending = widget.notifier.state.pendingEditCount > 0;
    // Difficulty is a private-payload edit on every targeted save, so writing
    // needs a compress-ready codec — disable Save with a hint otherwise,
    // matching the other private editors.
    final canWrite = widget.canCompress;
    // Save is enabled for a changed field OR a propagation-only intent (ticking
    // a box to push the current difficulty to the profile / all saves).
    final canSave = hasWork &&
        canWrite &&
        !blockingPending &&
        !_saving &&
        !widget.notifier.state.isLoading;

    final presetEnabled = !_saving;
    final permaEnabled = _preset != 'Novice' && !_saving;
    final levelsEnabled = _preset == 'Custom' && !_saving;

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
                    ? (selection) => _onPresetChanged(selection.first)
                    : null,
              ),
              const SizedBox(height: 8),
              // Toggles.
              SwitchListTile(
                contentPadding: EdgeInsets.zero,
                dense: true,
                title: const Text('Close Combat Flow Helper'),
                value: _flow,
                onChanged: _saving
                    ? null
                    : (value) {
                        setState(() => _flow = value);
                        _syncDirty();
                      },
              ),
              SwitchListTile(
                contentPadding: EdgeInsets.zero,
                dense: true,
                title: const Text('Permadeath'),
                subtitle: _preset == 'Novice'
                    ? const Text('Not available on Novice')
                    : null,
                value: _effectivePerma,
                onChanged: permaEnabled
                    ? (value) {
                        setState(() => _perma = value);
                        _syncDirty();
                      }
                    : null,
              ),
              const SizedBox(height: 8),
              // Level pickers.
              _LevelPicker(
                label: 'Combat',
                value: _displayedLevel(_combat),
                enabled: levelsEnabled,
                onChanged: (value) {
                  setState(() => _combat = value);
                  _syncDirty();
                },
              ),
              const SizedBox(height: 8),
              _LevelPicker(
                label: 'Resources',
                value: _displayedLevel(_resources),
                enabled: levelsEnabled,
                onChanged: (value) {
                  setState(() => _resources = value);
                  _syncDirty();
                },
              ),
              const SizedBox(height: 8),
              _LevelPicker(
                label: 'Progression',
                value: _displayedLevel(_progression),
                enabled: levelsEnabled,
                onChanged: (value) {
                  setState(() => _progression = value);
                  _syncDirty();
                },
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
                  'you tick the options below.',
                  style: theme.textTheme.bodySmall,
                ),
              ),
              const SizedBox(height: 8),
              // Propagation checkboxes.
              CheckboxListTile(
                contentPadding: EdgeInsets.zero,
                dense: true,
                controlAffinity: ListTileControlAffinity.leading,
                title: const Text('Also update the profile'),
                subtitle: hasProfile
                    ? null
                    : const Text('No resolved profile to update'),
                value: _alsoProfile,
                onChanged: hasProfile && !_saving
                    ? (value) {
                        setState(() => _alsoProfile = value ?? false);
                        _syncDirty();
                      }
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
                onChanged: hasProfile && !_saving
                    ? (value) {
                        setState(() => _allSaves = value ?? false);
                        _syncDirty();
                      }
                    : null,
              ),
              const SizedBox(height: 12),
              if (!canWrite)
                Padding(
                  padding: const EdgeInsets.only(bottom: 8),
                  child: Text(
                    'Saving difficulty needs a verified G1R codec host '
                    '(it rewrites the private payload of each targeted save).',
                    style: theme.textTheme.bodySmall?.copyWith(
                      color: scheme.error,
                    ),
                  ),
                ),
              if (canWrite && blockingPending)
                Padding(
                  padding: const EdgeInsets.only(bottom: 8),
                  child: Text(
                    'Save or reset your other pending changes first — a '
                    'difficulty save reloads the file and would discard them.',
                    style: theme.textTheme.bodySmall?.copyWith(
                      color: scheme.error,
                    ),
                  ),
                ),
              Row(
                mainAxisAlignment: MainAxisAlignment.end,
                children: [
                  OutlinedButton.icon(
                    icon: const Icon(Icons.undo),
                    label: const Text('Reset'),
                    onPressed: hasWork && !_saving ? _reset : null,
                  ),
                  const SizedBox(width: 8),
                  FilledButton.icon(
                    icon: const Icon(Icons.save_outlined),
                    label: const Text('Save'),
                    onPressed: canSave ? _save : null,
                  ),
                ],
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
