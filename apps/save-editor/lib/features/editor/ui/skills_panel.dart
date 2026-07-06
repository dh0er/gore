import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:goresave/features/editor/domain/pending_edits.dart';
import 'package:goresave/features/editor/domain/skills_models.dart';
import 'package:goresave/l10n/app_localizations.dart';

import '../domain/editor_notifier.dart';

/// An actor's learned skills, rendered inside the "Talente" group of the
/// attribute tab in the SAME row style as attributes: a fixed-width label and a
/// tier dropdown. Skills are the actor's GameplayEffects.
///
/// For the hero ([showRoster] true) every catalogued skill is listed — learned
/// ones with their tier, the rest as roster entries to learn. For an NPC
/// ([showRoster] false) only its learned skills are shown (the full roster would
/// be noise).
///
/// Self-contained: loads via [EditorNotifier.loadSkills] for [actor] and manages
/// its own [pendingKey] registry entry (declarative `private.skills.set` intents
/// keyed by skill base, each carrying [actor]), so the shared Save button writes
/// all changed skills. [reloadKey] is the current inspection identity; a change
/// (save/refresh/switch) clears local drafts and reloads — the registry is
/// cleared centrally.
class SkillsSection extends ConsumerStatefulWidget {
  const SkillsSection({
    super.key,
    required this.notifier,
    required this.editable,
    required this.reloadKey,
    this.actor = 'Hero',
    this.pendingKey = 'skills',
    this.showRoster = true,
  });

  final EditorNotifier notifier;
  final bool editable;
  final Object reloadKey;

  /// The actor whose skills these are: `'Hero'` for the player, else an NPC's
  /// GlobalId. Passed to `loadSkills` and each `private.skills.set` edit.
  final String actor;

  /// The pending-registry key these edits live under. Per-actor (`skills` for
  /// the hero, `skills:<npcId>` for an NPC) so different actors' skill edits do
  /// not collide.
  final String pendingKey;

  /// Whether to show the full learnable roster (hero) or only learned skills
  /// (NPC).
  final bool showRoster;

  @override
  ConsumerState<SkillsSection> createState() => _SkillsSectionState();
}

class _SkillsSectionState extends ConsumerState<SkillsSection> {
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
    final pending = widget.notifier.pendingEditFor(widget.pendingKey);
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
  void didUpdateWidget(covariant SkillsSection oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (widget.reloadKey != oldWidget.reloadKey) {
      // Drop local drafts and re-seed from the registry: a normal save/refresh
      // clears the 'skills' entry centrally (so this seeds nothing), but a
      // PARTIAL save whose trailing skill write failed keeps that entry for
      // retry — re-seeding keeps the dropdowns in sync with the edits the next
      // Save will still apply, instead of showing on-disk values.
      _pending.clear();
      _seedFromPending();
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
    final result = await widget.notifier.loadSkills(actor: widget.actor);
    if (!mounted || epoch != _epoch) return;
    setState(() {
      _result = result;
      _loading = false;
    });
  }

  void _pushPending() {
    if (_pending.isEmpty) {
      widget.notifier.clearPendingEdit(widget.pendingKey);
      return;
    }
    widget.notifier.setPendingEdit(
      widget.pendingKey,
      PendingSaveEdit(
        edits: _pending.entries
            .map(
              (e) => SkillSetEdit(
                base: e.key,
                tier: e.value,
                actor: widget.actor,
              ).toEditJson(),
            )
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
    // `found` is false when the actor has no ActiveEffects array to edit into —
    // `private.skills.set` would then fail, so never render the editable roster
    // in that case (it lists every catalogued skill regardless of `found`).
    // Hero: a distinct message (the roster is not missing, the edit target is).
    // NPC (no roster): plain "no skills found" — the hero-specific
    // edit-target message would be wrong for a monster.
    if (result != null && !result.found) {
      return Padding(
        padding: const EdgeInsets.symmetric(vertical: 8),
        child: Text(
          widget.showRoster ? l10n.skillsUnavailableBody : l10n.skillsNoneBody,
        ),
      );
    }
    if (result == null) {
      return const SizedBox.shrink();
    }
    // The hero shows the full roster (learn anything); an NPC shows only its
    // learned skills — the whole 29-skill catalog would be noise on an NPC.
    final visible = widget.showRoster
        ? result.skills
        : result.skills.where((s) => s.learned).toList();
    if (visible.isEmpty) {
      return Padding(
        padding: const EdgeInsets.symmetric(vertical: 8),
        child: Text(l10n.skillsNoneBody),
      );
    }

    final byCategory = <String, List<Skill>>{};
    for (final s in visible) {
      byCategory.putIfAbsent(s.category, () => []).add(s);
    }
    // Sort skills within each category by their localized name (the core orders
    // by base name; the user reads them by display name). Case-insensitive.
    for (final list in byCategory.values) {
      list.sort(
        (a, b) => _skillName(l10n, a)
            .toLowerCase()
            .compareTo(_skillName(l10n, b).toLowerCase()),
      );
    }
    final categories = byCategory.entries.toList();
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        for (var i = 0; i < categories.length; i++) ...[
          // Extra breathing room between a section and the next category header
          // (the first header needs none — the group title sits above it).
          SizedBox(height: i == 0 ? 4 : 22),
          Padding(
            padding: const EdgeInsets.only(bottom: 4),
            child: Text(
              _categoryLabel(l10n, categories[i].key),
              style: theme.textTheme.labelSmall?.copyWith(
                color: theme.colorScheme.primary,
                letterSpacing: 0.6,
              ),
            ),
          ),
          for (final skill in categories[i].value)
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
            for (var i = 0; i < skill.options.length; i++)
              DropdownMenuItem<String>(
                value: skill.options[i].value,
                child: Text(_optionLabel(l10n, skill, i)),
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
                label,
                const SizedBox(height: 6),
                dropdown,
              ],
            );
          }
          return Row(
            crossAxisAlignment: CrossAxisAlignment.center,
            children: [
              SizedBox(width: 170, child: label),
              Expanded(child: dropdown),
            ],
          );
        },
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
    case 'Melee_Orc':
      return l10n.skillNameOrcWeapons;
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
    case 'Hunting_MandibleMineCrawler':
      return l10n.skillNameTakeMinecrawlerMandibles;
    case 'Hunting_ShadowbeastHorn':
      return l10n.skillNameTakeShadowbeastHorn;
    case 'Hunting_Spines':
      return l10n.skillNameTakeSpines;
    case 'Hunting_TeethSwampshark':
      return l10n.skillNameBreakSwampsharkTeeth;
    case 'Hunting_TongueOfFire':
      return l10n.skillNameTakeFireTongue;
    case 'Hunting_TrollHorn':
      return l10n.skillNameTakeTrollHorn;
    case 'Acrobatics':
      return l10n.skillNameAcrobatics;
    case 'Wallclimbing':
      return l10n.skillNameWallClimbing;
    case 'Riding':
      return l10n.skillNameRiding;
    case 'Sneak':
      return l10n.skillNameSneaking;
    case 'Diving':
      return l10n.skillNameDiving;
    case 'Crafting_Alchemy':
      return l10n.skillNameAlchemy;
    case 'Crafting_Inscription':
      return l10n.skillNameRuneInscription;
    case 'Crafting_Blacksmith':
      return l10n.skillNameBlacksmithing;
    case 'Mining':
      return l10n.skillNameMining;
    case 'Mage_Circle':
      return l10n.skillNameMagicCircle;
    case 'Orcish':
      return l10n.skillNameOrcish;
    default:
      return skill.label;
  }
}

/// Localized label for the option at [index] of [skill]'s dropdown. The scheme
/// depends on the skill's shape, keyed off the option count / kind:
/// - 3-state ladders (weapons, smithing, thievery): Beginner → Trained → Master
///   by position (a ladder's options are always Untrained + two tiers, in
///   order).
/// - Magic Circle: Not learned → Circle 0 (Amateur) → Circle 1 … → Circle 6.
/// - everything else (2-state: Orcish, hunting, on/off): Not learned → Learned,
///   keyed by VALUE because a learned on/off skill lists its options in the
///   reverse order (learned value first, Untrained second).
String _optionLabel(AppLocalizations l10n, Skill skill, int index) {
  final value = skill.options[index].value;

  // Blacksmithing's two learned tiers have bespoke in-game names
  // (skill_crafting_blacksmith_trained/master); Untrained uses the generic label.
  if (skill.base == 'Crafting_Blacksmith') {
    if (value == 'Trained') return l10n.skillSmithing1H;
    if (value == 'Master') return l10n.skillSmithing2H;
  }

  // Magic Circle uses the game's circle names (skill_mage_circle_*).
  if (skill.kind == 'circle') {
    switch (value) {
      case 'Amateur':
        return l10n.skillCircleNovice;
      case '1':
        return l10n.skillCircle1;
      case '2':
        return l10n.skillCircle2;
      case '3':
        return l10n.skillCircle3;
      case '4':
        return l10n.skillCircle4;
      case '5':
        return l10n.skillCircle5;
      case '6':
        return l10n.skillCircle6;
    }
  }

  // Generic mastery labels (the game's skillmastery_* vocabulary). The internal
  // `Skilled` tier (thievery/mining/orcish) and the binary `Learned` state both
  // display as Trained in-game — the game has no separate "Skilled" label.
  switch (value) {
    case 'Master':
      return l10n.skillTierMaster;
    case 'Trained':
    case 'Skilled':
    case 'Learned':
      return l10n.skillTierTrained;
    case 'Untrained':
    default:
      return l10n.skillTierUntrained;
  }
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
