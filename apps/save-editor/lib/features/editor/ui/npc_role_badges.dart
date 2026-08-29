import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:goresave/features/editor/domain/game_icons.dart';
import 'package:goresave/features/editor/domain/glossary_npc_catalog.dart';
import 'package:goresave/features/editor/ui/game_icon.dart';
import 'package:goresave/l10n/app_localizations.dart';
import 'package:goresave/providers/data_providers.dart';

/// What the glossary files a character under, as compact labelled chips: the
/// shop he runs, the skills he teaches, the armour he makes.
///
/// Only the ~160 named characters carry any; everyone else renders nothing at
/// all, so a header that has none loses no space to this.
class NpcRoleBadges extends StatelessWidget {
  const NpcRoleBadges({
    super.key,
    required this.npcUniqueName,
    this.isDead = false,
  });

  /// The character's own id — either the bare unique name (`NC_SLD_Orik_701`)
  /// or a save GlobalId that starts with it.
  final String npcUniqueName;

  /// Whether this character is dead in the save. Not a glossary role — the
  /// glossary's `dead` entry says the player may READ about a death, this says
  /// the actor is lying there.
  final bool isDead;

  /// The roles worth naming here, in a fixed order so a character's chips do
  /// not reshuffle between selections.
  static const _shown = [
    NpcGlossaryRole.trader,
    NpcGlossaryRole.teacher,
    NpcGlossaryRole.armorer,
  ];

  @override
  Widget build(BuildContext context) {
    // Panels are pumped without a ProviderScope in widget tests, and the roles
    // are an enhancement — render nothing rather than make a scope a
    // requirement of every header that shows a character.
    final scoped =
        context.findAncestorWidgetOfExactType<UncontrolledProviderScope>() !=
        null;
    if (!scoped) return _chips(context, const {});
    return Consumer(
      builder: (context, ref, _) => _chips(
        context,
        ref.watch(glossaryRolesByNpcProvider).value?[npcUniqueName
                .split('-')
                .first
                .trim()
                .toLowerCase()] ??
            const {},
      ),
    );
  }

  Widget _chips(BuildContext context, Set<NpcGlossaryRole> roles) {
    final shown = _shown.where(roles.contains).toList(growable: false);
    if (shown.isEmpty && !isDead) return const SizedBox.shrink();
    final l10n = AppLocalizations.of(context);
    final theme = Theme.of(context);
    final scheme = theme.colorScheme;

    Widget chip(NpcGlossaryRole role, {Color? colour}) => Chip(
      visualDensity: VisualDensity.compact,
      materialTapTargetSize: MaterialTapTargetSize.shrinkWrap,
      avatar: GameIcon(
        name: gameIconForNpcRole(role),
        fallbackIcon: _fallbackIcon(role),
        size: 16,
        color: colour,
      ),
      label: Text(_label(l10n, role)),
      labelStyle: theme.textTheme.bodySmall?.copyWith(color: colour),
    );

    return Wrap(
      spacing: 6,
      runSpacing: 6,
      children: [
        for (final role in shown) chip(role),
        // Last, and in the theme's error colour: being dead is a state, not a
        // trade, and it is the one the reader should catch at a glance.
        if (isDead) chip(NpcGlossaryRole.dead, colour: scheme.error),
      ],
    );
  }

  static IconData _fallbackIcon(NpcGlossaryRole role) => switch (role) {
    NpcGlossaryRole.trader => Icons.storefront_outlined,
    NpcGlossaryRole.teacher => Icons.school_outlined,
    NpcGlossaryRole.armorer => Icons.shield_outlined,
    NpcGlossaryRole.dead => Icons.dangerous_outlined,
    NpcGlossaryRole.hostile => Icons.gpp_bad_outlined,
    NpcGlossaryRole.portrait => Icons.portrait_outlined,
  };

  static String _label(AppLocalizations l10n, NpcGlossaryRole role) =>
      switch (role) {
        NpcGlossaryRole.trader => l10n.roleTrader,
        NpcGlossaryRole.teacher => l10n.roleTeacher,
        NpcGlossaryRole.armorer => l10n.roleArmorer,
        NpcGlossaryRole.dead => l10n.roleDead,
        NpcGlossaryRole.hostile => l10n.glossaryFilterHostile,
        NpcGlossaryRole.portrait => l10n.glossarySegmentIntroduction,
      };
}
