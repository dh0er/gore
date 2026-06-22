import 'package:flutter/material.dart';
import 'package:goresave/features/editor/domain/editor_models.dart';
import 'package:goresave/features/editor/domain/editor_notifier.dart';
import 'package:goresave/l10n/app_localizations.dart';

/// Maps a logical preset value to its localized display label.
String _presetLabel(AppLocalizations l10n, String p) => switch (p) {
  'Novice' => l10n.presetNovice,
  'Gothic' => l10n.presetGothic,
  'Hard' => l10n.presetHard,
  'Custom' => l10n.presetCustom,
  _ => p,
};

/// Maps a logical sub-level value to its localized display label.
String _levelLabel(AppLocalizations l10n, String lvl) => switch (lvl) {
  'Novice' => l10n.presetNovice,
  'Gothic' => l10n.presetGothic,
  'Hard' => l10n.presetHard,
  _ => lvl,
};

/// The four difficulty presets, in the order the in-game screen lists them.
/// These are the exact label strings the core's `write_difficulty` maps back to
/// its class names (Novice→_Easy, Gothic→_Standard, Hard→_Hard, Custom→_Custom).
const _presets = ['Novice', 'Gothic', 'Hard', 'Custom'];

/// Sentinel for a stored preset that is missing or not one of the four known UI
/// presets. Never treated as a concrete preset: the selector shows nothing
/// selected and Save stays disabled until the user picks a known preset.
const _unknownPreset = '';

/// The three sub-level options shown for Combat / Resources / Progression.
const _levels = ['Novice', 'Gothic', 'Hard'];

/// The sub-level implied (and shown locked) when the preset is not Custom.
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

String _normalizePreset(String? label) =>
    _presets.contains(label) ? label! : _unknownPreset;

String _normalizeLevel(String? label, String preset) {
  if (_levels.contains(label)) return label!;
  return preset == 'Custom' ? 'Gothic' : _impliedLevelForPreset(preset);
}

/// Preset → accent color, mirroring the in-game tone (calm → fierce). Used to
/// tint the chip so the difficulty reads at a glance.
Color _presetColor(String preset) {
  switch (preset) {
    case 'Novice':
      return const Color(0xFF3F9D54); // green
    case 'Gothic':
      return const Color(0xFF3F77C2); // blue
    case 'Hard':
      return const Color(0xFFD8662F); // orange
    case 'Custom':
      return const Color(0xFF8A5CC4); // purple
    default:
      return const Color(0xFF6B7280); // grey (unknown)
  }
}

/// A prominent, tappable difficulty chip shown in the profile header. Displays
/// the active profile's authoritative difficulty and opens [DifficultyDialog]
/// on tap. Replaces the old per-save difficulty badges — difficulty is a
/// profile-wide value, so it lives with the profile, not the save.
class ProfileDifficultyChip extends StatelessWidget {
  const ProfileDifficultyChip({
    super.key,
    required this.profile,
    required this.notifier,
    required this.isLoading,
  });

  /// The active profile. Null when no profile is resolved — the chip is then
  /// shown disabled (there is no profile difficulty to edit).
  final ProfileSummary? profile;
  final EditorNotifier notifier;
  final bool isLoading;

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context);
    final theme = Theme.of(context);
    final hasProfile = profile != null;
    // A profile carries editable difficulty when it has ANY difficulty field —
    // even if the stored preset class is unrecognised (e.g. a new preset after a
    // game update): the dialog can still repair it. Only a profile synthesized
    // without difficulty data (no `m_Profiles` entry to patch) is non-editable,
    // where a write would dead-end (fails, or targetsWritten: 0).
    final hasDifficulty = hasProfile && profile!.difficulty.hasAnyValue;
    final preset = hasProfile
        ? _normalizePreset(profile!.difficulty.presetLabel)
        : _unknownPreset;
    final known = preset != _unknownPreset;
    final color = known ? _presetColor(preset) : _presetColor(_unknownPreset);
    final label = known
        ? _presetLabel(l10n, preset)
        : (hasDifficulty
              ? l10n.difficultyLabel
              : (hasProfile
                    ? l10n.difficultyNoDifficulty
                    : l10n.difficultyNoProfile));
    final enabled = hasDifficulty && !isLoading;

    final chip = AnimatedContainer(
      duration: const Duration(milliseconds: 120),
      padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 7),
      decoration: BoxDecoration(
        color: enabled
            ? color.withValues(alpha: 0.16)
            : theme.colorScheme.surfaceContainerHighest,
        borderRadius: BorderRadius.circular(999),
        border: Border.all(
          color: enabled
              ? color.withValues(alpha: 0.55)
              : theme.colorScheme.outlineVariant,
          width: 1,
        ),
      ),
      child: Row(
        mainAxisSize: MainAxisSize.min,
        children: [
          Icon(
            Icons.local_fire_department,
            size: 18,
            color: enabled ? color : theme.colorScheme.onSurfaceVariant,
          ),
          const SizedBox(width: 6),
          Flexible(
            child: Text(
              label,
              maxLines: 1,
              overflow: TextOverflow.ellipsis,
              softWrap: false,
              style: theme.textTheme.labelLarge?.copyWith(
                fontWeight: FontWeight.w600,
                color: enabled
                    ? (known
                          ? color.withValues(alpha: 0.95)
                          : theme.colorScheme.onSurface)
                    : theme.colorScheme.onSurfaceVariant,
              ),
            ),
          ),
          if (enabled) ...[
            const SizedBox(width: 4),
            Icon(Icons.edit, size: 14, color: color.withValues(alpha: 0.8)),
          ],
        ],
      ),
    );

    return Tooltip(
      message: !hasProfile
          ? l10n.difficultyTooltipNoProfile
          : (hasDifficulty
                ? l10n.difficultyTooltipEdit
                : l10n.difficultyTooltipNoEditable),
      child: Material(
        color: Colors.transparent,
        child: InkWell(
          borderRadius: BorderRadius.circular(999),
          onTap: enabled ? () => _open(context) : null,
          child: chip,
        ),
      ),
    );
  }

  Future<void> _open(BuildContext context) async {
    await showDialog<void>(
      context: context,
      builder: (_) => DifficultyDialog(profile: profile!, notifier: notifier),
    );
  }
}

/// Modal profile-difficulty editor. Self-contained: its own draft, Save, and
/// Cancel. Save writes the profile via [EditorNotifier.writeProfileDifficulty]
/// (a mandatory backup is taken by the core) and closes. The change applies to
/// every save in the profile — stated in the hint, so no extra confirmation.
class DifficultyDialog extends StatefulWidget {
  const DifficultyDialog({
    super.key,
    required this.profile,
    required this.notifier,
  });

  final ProfileSummary profile;
  final EditorNotifier notifier;

  @override
  State<DifficultyDialog> createState() => _DifficultyDialogState();
}

class _DifficultyDialogState extends State<DifficultyDialog> {
  late String _preset;
  late bool _flow;
  late bool _perma;
  late String _combat;
  late String _resources;
  late String _progression;
  // Whether each Custom sub-level has a concrete value to write. False when the
  // stored class is unrecognised and the user has not picked one — then it is
  // omitted from the save so the stored (unknown) sub-setting is preserved.
  late bool _combatSet;
  late bool _resourcesSet;
  late bool _progressionSet;
  bool _saving = false;

  /// The last write error, surfaced INSIDE the dialog. The workspace error
  /// banner sits behind the modal, so without this the user gets no feedback on
  /// a failed Save.
  String? _error;

  DifficultySettings get _stored => widget.profile.difficulty;

  @override
  void initState() {
    super.initState();
    final stored = _stored;
    _preset = _normalizePreset(stored.presetLabel);
    _flow = stored.flowHelper ?? false;
    _perma = _preset == 'Novice' ? false : (stored.permadeath ?? false);
    _combat = _normalizeLevel(stored.combatLabel, _preset);
    _resources = _normalizeLevel(stored.resourcesLabel, _preset);
    _progression = _normalizeLevel(stored.progressionLabel, _preset);
    // A sub-level is "set" only when the stored class maps to a known level;
    // an unrecognised class stays unset (omitted on save) until the user picks.
    _combatSet = _levels.contains(stored.combatLabel);
    _resourcesSet = _levels.contains(stored.resourcesLabel);
    _progressionSet = _levels.contains(stored.progressionLabel);
  }

  /// The level a picker should DISPLAY, or null to show no selection: the draft
  /// value on Custom (pickers editable), otherwise the level the preset implies
  /// (pickers locked). Null only when Custom and the field is unset (the stored
  /// class is unrecognised and the user has not picked one yet).
  String? _displayedLevel(String field) {
    if (_preset != 'Custom') return _impliedLevelForPreset(_preset);
    final (value, isSet) = switch (field) {
      'combat' => (_combat, _combatSet),
      'resources' => (_resources, _resourcesSet),
      _ => (_progression, _progressionSet),
    };
    return isSet ? value : null;
  }

  void _selectPreset(String preset) {
    setState(() {
      _preset = preset;
      // Novice locks permadeath off; leaving Custom snaps the sub-levels to the
      // implied level so a later return to Custom starts coherent.
      if (preset == 'Novice') _perma = false;
      if (preset != 'Custom') {
        final implied = _impliedLevelForPreset(preset);
        _combat = implied;
        _resources = implied;
        _progression = implied;
        // The sub-levels are now concrete (the preset's implied level), so a
        // later switch back to Custom starts from known values.
        _combatSet = true;
        _resourcesSet = true;
        _progressionSet = true;
      }
    });
  }

  Map<String, Object?> _buildDifficulty() {
    final known = _preset != _unknownPreset;
    return {
      // Omit the preset when none is selected (the stored class is unrecognised
      // and the user did not pick one): the core then leaves the preset and
      // sub-settings as stored and writes only the bool toggles below.
      if (known) 'preset': _preset,
      // Only send a Custom sub-level that is actually set; omitting one leaves
      // the stored (possibly unrecognised) sub-setting untouched.
      if (_preset == 'Custom' && _combatSet) 'combat': _combat,
      if (_preset == 'Custom' && _resourcesSet) 'resources': _resources,
      if (_preset == 'Custom' && _progressionSet) 'progression': _progression,
      'flowHelper': _flow,
      'permadeath': _perma,
    };
  }

  Future<void> _save() async {
    final l10n = AppLocalizations.of(context);
    setState(() {
      _saving = true;
      _error = null;
    });
    final ok = await widget.notifier.writeProfileDifficulty(
      profileId: widget.profile.profileId,
      difficulty: _buildDifficulty(),
    );
    if (!mounted) return;
    if (ok) {
      Navigator.of(context).pop();
    } else {
      // Surface the failure INSIDE the dialog — the workspace banner is hidden
      // behind the modal. Capture the notifier's error and clear it there so it
      // is shown in one place; keep the dialog open to retry or cancel.
      final message = widget.notifier.lastError ?? l10n.savingDifficultyFailed;
      widget.notifier.dismissError();
      setState(() {
        _saving = false;
        _error = message;
      });
    }
  }

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context);
    final theme = Theme.of(context);
    final scheme = theme.colorScheme;
    final known = _preset != _unknownPreset;
    final permaEnabled = _preset != 'Novice' && !_saving;
    final levelsEnabled = _preset == 'Custom' && !_saving;

    return AlertDialog(
      title: Row(
        children: [
          Icon(Icons.local_fire_department, color: _presetColor(_preset)),
          const SizedBox(width: 8),
          Expanded(
            child: Text(l10n.difficultyTitle(widget.profile.displayName)),
          ),
        ],
      ),
      content: SizedBox(
        width: 420,
        child: SingleChildScrollView(
          child: Column(
            mainAxisSize: MainAxisSize.min,
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Text(l10n.preset, style: theme.textTheme.labelLarge),
              const SizedBox(height: 6),
              SegmentedButton<String>(
                segments: [
                  for (final preset in _presets)
                    ButtonSegment<String>(
                      value: preset,
                      label: Text(_presetLabel(l10n, preset)),
                    ),
                ],
                emptySelectionAllowed: !known,
                selected: known ? {_preset} : const <String>{},
                showSelectedIcon: false,
                onSelectionChanged: _saving
                    ? null
                    : (selection) => _selectPreset(selection.first),
              ),
              if (!known)
                Padding(
                  padding: const EdgeInsets.only(top: 6),
                  child: Text(
                    l10n.unrecognisedPreset(_stored.presetLabel),
                    style: theme.textTheme.bodySmall?.copyWith(
                      color: scheme.error,
                    ),
                  ),
                ),
              const SizedBox(height: 8),
              SwitchListTile(
                contentPadding: EdgeInsets.zero,
                dense: true,
                title: Text(l10n.closeCombatFlowHelper),
                value: _flow,
                onChanged: _saving ? null : (v) => setState(() => _flow = v),
              ),
              SwitchListTile(
                contentPadding: EdgeInsets.zero,
                dense: true,
                title: Text(l10n.permadeath),
                subtitle: _preset == 'Novice'
                    ? Text(l10n.notAvailableOnNovice)
                    : null,
                value: _perma,
                onChanged: permaEnabled
                    ? (v) => setState(() => _perma = v)
                    : null,
              ),
              const SizedBox(height: 8),
              _LevelPicker(
                label: l10n.levelCombat,
                value: _displayedLevel('combat'),
                enabled: levelsEnabled,
                onChanged: (v) => setState(() {
                  _combat = v;
                  _combatSet = true;
                }),
              ),
              const SizedBox(height: 8),
              _LevelPicker(
                label: l10n.levelResources,
                value: _displayedLevel('resources'),
                enabled: levelsEnabled,
                onChanged: (v) => setState(() {
                  _resources = v;
                  _resourcesSet = true;
                }),
              ),
              const SizedBox(height: 8),
              _LevelPicker(
                label: l10n.levelProgression,
                value: _displayedLevel('progression'),
                enabled: levelsEnabled,
                onChanged: (v) => setState(() {
                  _progression = v;
                  _progressionSet = true;
                }),
              ),
              const SizedBox(height: 16),
              Container(
                padding: const EdgeInsets.all(12),
                decoration: BoxDecoration(
                  color: scheme.surfaceContainerHighest,
                  borderRadius: BorderRadius.circular(8),
                ),
                child: Row(
                  children: [
                    Icon(
                      Icons.info_outline,
                      size: 18,
                      color: scheme.onSurfaceVariant,
                    ),
                    const SizedBox(width: 8),
                    Expanded(
                      child: Text(
                        l10n.difficultyAppliesToAllSaves,
                        style: theme.textTheme.bodySmall,
                      ),
                    ),
                  ],
                ),
              ),
              if (_error != null) ...[
                const SizedBox(height: 12),
                Container(
                  padding: const EdgeInsets.all(12),
                  decoration: BoxDecoration(
                    color: scheme.errorContainer,
                    borderRadius: BorderRadius.circular(8),
                  ),
                  child: Row(
                    children: [
                      Icon(
                        Icons.error_outline,
                        size: 18,
                        color: scheme.onErrorContainer,
                      ),
                      const SizedBox(width: 8),
                      Expanded(
                        child: Text(
                          _error!,
                          style: theme.textTheme.bodySmall?.copyWith(
                            color: scheme.onErrorContainer,
                          ),
                        ),
                      ),
                    ],
                  ),
                ),
              ],
            ],
          ),
        ),
      ),
      actions: [
        TextButton(
          onPressed: _saving ? null : () => Navigator.of(context).pop(),
          child: Text(l10n.cancel),
        ),
        FilledButton(
          // Save is allowed even with an unrecognised stored preset: an
          // unselected preset is simply omitted (bool-only edit). It is blocked
          // only while a write is in flight.
          onPressed: _saving ? null : _save,
          child: _saving
              ? const SizedBox(
                  width: 16,
                  height: 16,
                  child: CircularProgressIndicator(strokeWidth: 2),
                )
              : Text(l10n.save),
        ),
      ],
    );
  }
}

/// One labelled Novice/Gothic/Hard segmented picker. A null [value] shows no
/// selection (an unrecognised, unpicked Custom sub-level). Disabled state
/// reflects the value but ignores taps.
class _LevelPicker extends StatelessWidget {
  const _LevelPicker({
    required this.label,
    required this.value,
    required this.enabled,
    required this.onChanged,
  });

  final String label;
  final String? value;
  final bool enabled;
  final ValueChanged<String> onChanged;

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context);
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
                ButtonSegment<String>(
                  value: level,
                  label: Text(_levelLabel(l10n, level)),
                ),
            ],
            emptySelectionAllowed: value == null,
            selected: value == null ? const <String>{} : {value!},
            showSelectedIcon: false,
            onSelectionChanged:
                enabled ? (selection) => onChanged(selection.first) : null,
          ),
        ),
      ],
    );
  }
}
