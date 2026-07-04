import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:goresave/features/editor/domain/editor_models.dart';
import 'package:goresave/features/editor/domain/pending_edits.dart';
import 'package:goresave/features/editor/domain/skills_models.dart';
import 'package:goresave/l10n/app_localizations.dart';

import '../domain/editor_notifier.dart';

/// The hero's learned skills, rendered inside the "Talente" group of the
/// attribute tab in the SAME row style as attributes: a fixed-width label and a
/// tier dropdown. Skills are the hero's GameplayEffects; every catalogued skill
/// is listed (learned ones with their tier, the rest as roster entries the hero
/// can learn), grouped by category.
///
/// Self-contained: loads via [EditorNotifier.loadSkills] and manages its own
/// `skills` pending-registry entry (declarative `private.skills.set` intents
/// keyed by skill base), so the shared Save button writes all changed skills.
/// [reloadKey] is the current [SaveInspection]; a change (save/refresh/switch)
/// clears local drafts and reloads — the registry is cleared centrally.
class HeroSkillsSection extends ConsumerStatefulWidget {
  const HeroSkillsSection({
    super.key,
    required this.notifier,
    required this.editable,
    required this.reloadKey,
  });

  final EditorNotifier notifier;
  final bool editable;
  final Object reloadKey;

  @override
  ConsumerState<HeroSkillsSection> createState() => _HeroSkillsSectionState();
}

class _HeroSkillsSectionState extends ConsumerState<HeroSkillsSection> {
  SkillsResult? _result;
  bool _loading = false;
  int _epoch = 0;
  // Pending tier changes keyed by skill base (target option value). Absent
  // means unchanged.
  final Map<String, String> _pending = {};

  @override
  void initState() {
    super.initState();
    // Re-seed drafts from the registry's 'skills' entry so switching away (e.g.
    // to an NPC) and back to the Player on the SAME inspection resumes from the
    // queued skill edits the shared Save button will write — instead of showing
    // on-disk values while the registry still holds (and applies) them.
    _seedFromPending();
    _load();
  }

  /// Reconstruct [_pending] (base -> target tier) from the registry's queued
  /// `private.skills.set` edits. Inverse of [_pushPending].
  void _seedFromPending() {
    final pending = widget.notifier.pendingEditFor('skills');
    if (pending == null) return;
    for (final edit in pending.edits) {
      if (edit['path'] != 'private.skills.set') continue;
      final value = edit['value'];
      if (value is! Map) continue;
      final base = value['base'];
      final tier = value['tier'];
      if (base is String && tier is String) _pending[base] = tier;
    }
  }

  @override
  void didUpdateWidget(covariant HeroSkillsSection oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (widget.reloadKey != oldWidget.reloadKey) {
      // The registry's 'skills' entry is cleared centrally on save/refresh;
      // just drop the local drafts and reload the on-disk state.
      _pending.clear();
      _load();
    }
  }

  Future<void> _load() async {
    final epoch = ++_epoch;
    // Drop the previous result too, not just flip _loading: build only shows the
    // spinner while `result == null`, so keeping stale skills here would let the
    // user queue tier changes against outdated skill.current values during a
    // reloadKey-triggered reload (e.g. after a save/refresh).
    setState(() {
      _loading = true;
      _result = null;
    });
    final result = await widget.notifier.loadSkills();
    if (!mounted || epoch != _epoch) return;
    setState(() {
      _result = result;
      _loading = false;
    });
  }

  void _pushPending() {
    if (_pending.isEmpty) {
      widget.notifier.clearPendingEdit('skills');
      return;
    }
    widget.notifier.setPendingEdit(
      'skills',
      PendingSaveEdit(
        edits: _pending.entries
            .map((e) => SkillSetEdit(base: e.key, tier: e.value).toEditJson())
            .toList(),
      ),
    );
  }

  void _setSkill(Skill skill, String? value) {
    if (value == null) return;
    setState(() {
      if (value == skill.current) {
        _pending.remove(skill.base);
      } else {
        _pending[skill.base] = value;
      }
    });
    _pushPending();
  }

  String _selected(Skill skill) => _pending[skill.base] ?? skill.current;

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context);
    final theme = Theme.of(context);
    final result = _result;

    if (_loading && result == null) {
      return const Padding(
        padding: EdgeInsets.symmetric(vertical: 24),
        child: Center(child: CircularProgressIndicator()),
      );
    }
    if (result != null && result.error != null) {
      return Padding(
        padding: const EdgeInsets.symmetric(vertical: 8),
        child: Text(
          result.error!,
          style: TextStyle(color: theme.colorScheme.error),
        ),
      );
    }
    // `found` is false when the hero has no ActiveEffects array to edit into —
    // `private.skills.set` would then fail, so never render the editable roster
    // in that case (it lists every catalogued skill regardless of `found`). Use
    // a distinct message: the roster is not missing, the edit target is.
    if (result != null && !result.found) {
      return Padding(
        padding: const EdgeInsets.symmetric(vertical: 8),
        child: Text(l10n.skillsUnavailableBody),
      );
    }
    if (result == null || result.skills.isEmpty) {
      return Padding(
        padding: const EdgeInsets.symmetric(vertical: 8),
        child: Text(l10n.skillsNoneBody),
      );
    }

    final byCategory = result.byCategory;
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        for (final entry in byCategory.entries) ...[
          Padding(
            padding: const EdgeInsets.only(top: 4, bottom: 2),
            child: Text(
              _categoryLabel(l10n, entry.key),
              style: theme.textTheme.labelSmall?.copyWith(
                color: theme.colorScheme.primary,
                letterSpacing: 0.6,
              ),
            ),
          ),
          for (final skill in entry.value)
            _SkillRow(
              skill: skill,
              editable: widget.editable,
              value: _selected(skill),
              onChanged: (v) => _setSkill(skill, v),
            ),
        ],
      ],
    );
  }
}

/// One skill row in the attribute-tab style: fixed-width label + tier dropdown.
/// Mirrors `_HeroAttributeRow`'s layout (label width 170, field on the right,
/// stacked when the pane is narrow).
class _SkillRow extends StatelessWidget {
  const _SkillRow({
    required this.skill,
    required this.editable,
    required this.value,
    required this.onChanged,
  });

  final Skill skill;
  final bool editable;
  final String value;
  final ValueChanged<String?> onChanged;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final l10n = AppLocalizations.of(context);
    final label = Text(_skillName(l10n, skill), style: theme.textTheme.labelLarge);
    final notLearned = !skill.learned && value == 'Untrained';

    final dropdown = InputDecorator(
      decoration: const InputDecoration(
        isDense: true,
        contentPadding: EdgeInsets.symmetric(horizontal: 10, vertical: 6),
        border: OutlineInputBorder(),
      ),
      child: DropdownButtonHideUnderline(
        child: DropdownButton<String>(
          value: value,
          isExpanded: true,
          isDense: true,
          items: [
            for (final o in skill.options)
              DropdownMenuItem<String>(
                value: o.value,
                child: Text(_optionLabel(l10n, skill.base, o)),
              ),
          ],
          onChanged: editable ? onChanged : null,
        ),
      ),
    );

    return Padding(
      padding: const EdgeInsets.symmetric(vertical: 5),
      child: LayoutBuilder(
        builder: (context, constraints) {
          final compact = constraints.maxWidth < 620;
          if (compact) {
            return Column(
              crossAxisAlignment: CrossAxisAlignment.stretch,
              children: [
                Row(
                  children: [
                    Expanded(child: label),
                    if (notLearned) _NotLearnedChip(l10n: l10n),
                  ],
                ),
                const SizedBox(height: 6),
                dropdown,
              ],
            );
          }
          return Row(
            crossAxisAlignment: CrossAxisAlignment.center,
            children: [
              SizedBox(
                width: 170,
                child: Row(
                  children: [
                    Flexible(child: label),
                    if (notLearned) _NotLearnedChip(l10n: l10n),
                  ],
                ),
              ),
              Expanded(child: dropdown),
            ],
          );
        },
      ),
    );
  }
}

class _NotLearnedChip extends StatelessWidget {
  const _NotLearnedChip({required this.l10n});

  final AppLocalizations l10n;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Padding(
      padding: const EdgeInsets.only(left: 6),
      child: Text(
        l10n.skillNotLearned.toUpperCase(),
        style: theme.textTheme.labelSmall?.copyWith(
          color: theme.colorScheme.onSurfaceVariant,
          fontSize: 9,
        ),
      ),
    );
  }
}

/// Localized skill display name. Falls back to the core's English [Skill.label]
/// for any base not (yet) given a key.
String _skillName(AppLocalizations l10n, Skill skill) {
  switch (skill.base) {
    case 'Melee_OneHanded':
      return l10n.skillNameOneHanded;
    case 'Melee_TwoHanded':
      return l10n.skillNameTwoHanded;
    case 'Melee_Fists':
      return l10n.skillNameFists;
    case 'Ranged_Bow':
      return l10n.skillNameBow;
    case 'Ranged_Crossbow':
      return l10n.skillNameCrossbow;
    case 'Picklock':
      return l10n.skillNameLockpicking;
    case 'Pickpocket':
      return l10n.skillNamePickpocketing;
    case 'Hunting_Organ':
      return l10n.skillNameTakeOrgans;
    case 'Hunting_Teeth':
      return l10n.skillNameBreakTeeth;
    case 'Hunting_Claw':
      return l10n.skillNameTakeClaws;
    case 'Hunting_Fur':
      return l10n.skillNameSkinFur;
    case 'Hunting_Skin':
      return l10n.skillNameSkin;
    case 'Hunting_Fins':
      return l10n.skillNameTakeFins;
    case 'Hunting_Stings':
      return l10n.skillNameTakeStingers;
    case 'Hunting_Secretion':
      return l10n.skillNameTakeSecretion;
    case 'Hunting_SkullArmor':
      return l10n.skillNameTakeSkullPlates;
    case 'Hunting_SkinSwampshark':
      return l10n.skillNameSkinSwampshark;
    case 'Hunting_MCPlate':
      return l10n.skillNameTakeMinecrawlerPlates;
    case 'Hunting_Scutes':
      return l10n.skillNameTakeScutes;
    case 'Hunting_UluMulu':
      return l10n.skillNameTakeUluMulu;
    case 'Acrobatics':
      return l10n.skillNameAcrobatics;
    case 'Wallclimbing':
      return l10n.skillNameWallClimbing;
    case 'Riding':
      return l10n.skillNameRiding;
    case 'Sneak':
      return l10n.skillNameSneaking;
    case 'Crafting_Alchemy':
      return l10n.skillNameAlchemy;
    case 'Crafting_Inscription':
      return l10n.skillNameRuneInscription;
    case 'Crafting_Blacksmith':
      return l10n.skillNameBlacksmithing;
    case 'Mage_Circle':
      return l10n.skillNameMagicCircle;
    case 'Orcish':
      return l10n.skillNameOrcish;
    default:
      return skill.label;
  }
}

/// Localized tier-option label composed from the option's structured pieces,
/// falling back to the core's English [SkillOption.label] where a token is not
/// mapped.
String _optionLabel(AppLocalizations l10n, String base, SkillOption o) {
  if (o.standalone == 'notLearned') return l10n.skillNotLearned;
  if (o.standalone == 'learn') return l10n.skillLearn;
  final buffer = StringBuffer(_tierToken(l10n, o.value));
  if (o.roman != null) buffer.write(' (${o.roman})');
  buffer.write(_hint(l10n, base, o.value));
  if (o.suffix == 'learn') buffer.write(' · ${l10n.skillActionLearn}');
  if (o.suffix == 'unlearn') buffer.write(' · ${l10n.skillActionUnlearn}');
  return buffer.toString();
}

/// Localized display for a tier value. Circle tiers are numeric ("Circle N").
String _tierToken(AppLocalizations l10n, String value) {
  switch (value) {
    case 'Untrained':
      return l10n.skillTierUntrained;
    case 'Trained':
      return l10n.skillTierTrained;
    case 'Master':
      return l10n.skillTierMaster;
    case 'Skilled':
      return l10n.skillTierNovice;
    case 'Amateur':
      return l10n.skillTierAmateur;
    case 'Learned':
      return l10n.skillTierLearned;
    default:
      final n = int.tryParse(value);
      return n != null ? l10n.skillTierCircle(n) : value;
  }
}

/// Blacksmithing's per-tier hint ("1H weapons" / "2H weapons"), else empty.
String _hint(AppLocalizations l10n, String base, String value) {
  if (base != 'Crafting_Blacksmith') return '';
  return switch (value) {
    'Trained' => ' — ${l10n.skillHintBlacksmith1H}',
    'Master' => ' — ${l10n.skillHintBlacksmith2H}',
    _ => '',
  };
}

/// Localized category header. Falls back to the raw category for anything not
/// yet given a key.
String _categoryLabel(AppLocalizations l10n, String category) {
  switch (category) {
    case 'Combat':
      return l10n.skillCategoryCombat;
    case 'Crafting':
      return l10n.skillCategoryCrafting;
    case 'Hunting':
      return l10n.skillCategoryHunting;
    case 'Language':
      return l10n.skillCategoryLanguage;
    case 'Magic':
      return l10n.skillCategoryMagic;
    case 'Movement':
      return l10n.skillCategoryMovement;
    case 'Thievery':
      return l10n.skillCategoryThievery;
    default:
      return category;
  }
}
