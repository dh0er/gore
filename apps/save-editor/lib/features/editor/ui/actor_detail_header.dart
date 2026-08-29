import 'package:flutter/material.dart';
import 'package:goresave/features/editor/domain/actor.dart';
import 'package:goresave/features/editor/domain/game_icons.dart';
import 'package:goresave/features/editor/ui/character_master_list.dart'
    show localizedNpcName;
import 'package:goresave/features/editor/domain/glossary_images.dart';
import 'package:goresave/features/editor/ui/glossary_portrait.dart';
import 'package:goresave/features/editor/ui/npc_role_badges.dart';
import 'package:goresave/l10n/app_localizations.dart';
import 'package:goresave/loc/game_lang.dart';
import 'package:goresave/ui/design/app_theme.dart';

/// Compact header identifying WHICH actor a detail pane is showing. Shared by
/// the Attribute and Inventory tab detail areas so both tabs make the selection
/// obvious above the sidebar/content.
///
/// - NPC selected → the localized display name (prominent) and the FULL
///   GlobalId. The id is always selectable and wraps so the user can copy/read
///   the whole value — never ellipsized and independent of [showObjectIds].
/// - Player selected → the localized "Player" label, no GlobalId (the player
///   has none).
/// - Orphan selected (knowledge-only, `orphan:<uniqueName>` id sentinel) → the
///   name resolved from its uniqueName (same loc-catalog key the list's orphan
///   tiles use). Optional technical display uses the real uniqueName, never the
///   synthetic sentinel.
///
/// The NPC name is resolved with the SAME [localizedNpcName] (loc catalog +
/// prettify fallback) the actor list tiles use, so the header and the list
/// always agree on the name.
class ActorDetailHeader extends StatelessWidget {
  const ActorDetailHeader({
    super.key,
    required this.actor,
    required this.locCatalog,
    required this.lang,
    this.showObjectIds = false,
  });

  /// The currently selected actor (player or NPC).
  final Actor actor;

  /// Loaded localization catalog (`id -> {set -> text}`) used to resolve the NPC
  /// display name.
  final Map<String, Map<String, String>> locCatalog;

  /// The current game language, driving which loc set the name resolves from.
  final GameLang lang;

  /// Whether a knowledge-only orphan's technical key is rendered below its
  /// player-facing name. Regular NPC GlobalIds are always shown. Orphans expose
  /// their real [Actor.uniqueName], never the synthetic `orphan:` selection
  /// sentinel.
  final bool showObjectIds;

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context);
    final theme = Theme.of(context);
    final scheme = theme.colorScheme;

    final isPlayer = actor.isPlayer;
    final isOrphan = actor.isOrphan;
    final id = actor.id;
    final technicalId = isOrphan ? actor.uniqueName : id;
    final showTechnicalId =
        !isPlayer &&
        technicalId != null &&
        technicalId.isNotEmpty &&
        (!isOrphan || showObjectIds);
    // Orphans resolve by uniqueName (their loc-catalog key — the `orphan:` id
    // sentinel would prettify into nonsense); NPCs resolve by GlobalId.
    final name = isPlayer
        ? l10n.tabPlayer
        : localizedNpcName(
            locCatalog,
            lang,
            isOrphan ? actor.uniqueName : (id ?? ''),
          );

    return Padding(
      padding: const EdgeInsets.fromLTRB(20, 16, 20, 0),
      child: Row(
        // The name — with its id under it where there is one — sits level with
        // the middle of the picture, not hung from its top edge.
        crossAxisAlignment: CrossAxisAlignment.center,
        children: [
          // The same picture the list row carries, so the detail pane and the
          // list agree on who is selected — here in the wide cut the game draws
          // for a detail view, the way the glossary shows it. A killed
          // character keeps his own face: death is a badge on the status row,
          // not an identity.
          GlossaryPortrait(
            npcUniqueName: isPlayer || isOrphan ? null : id,
            fallbackGameIcon: isOrphan || !isPlayer ? null : gameIconCharacter,
            fallbackIcon: isOrphan ? Icons.help_outline : Icons.person_outline,
            color: scheme.primary,
            // The banner artwork is 500x264, shown at the size the glossary's
            // own detail view shows it, so a character looks the same wherever
            // the editor puts him.
            size: GlossaryImageSize.banner,
            width: glossaryBannerWidth,
            height: glossaryBannerHeight,
          ),
          const SizedBox(width: 10),
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              mainAxisSize: MainAxisSize.min,
              children: [
                Text(
                  name,
                  style: theme.textTheme.titleMedium,
                  maxLines: 1,
                  overflow: TextOverflow.ellipsis,
                ),
                // NPC GlobalIds are always visible and wrap in full. Optional
                // orphan ids expose the real knowledge key, not the synthetic
                // `orphan:` selection sentinel.
                if (showTechnicalId)
                  Padding(
                    padding: const EdgeInsets.only(top: 2),
                    child: SelectableText(
                      technicalId,
                      style: theme.textTheme.bodySmall?.copyWith(
                        color: scheme.onSurfaceVariant,
                        fontFamily: uiAwareMonospaceFontFamily(context),
                      ),
                    ),
                  ),
              ],
            ),
          ),
          // What the glossary files this character as — the shops he runs, the
          // skills he teaches, the armour he makes. The list rows have no room
          // to name them; the space beside the name does.
          if (!isPlayer && !isOrphan && id != null)
            Padding(
              padding: const EdgeInsets.only(left: 12, top: 2),
              child: NpcRoleBadges(npcUniqueName: id, isDead: actor.isDead),
            ),
        ],
      ),
    );
  }
}
