import 'package:flutter/material.dart';
import 'package:goresave/features/editor/domain/actor.dart';
import 'package:goresave/features/editor/ui/character_master_list.dart'
    show localizedNpcName;
import 'package:goresave/l10n/app_localizations.dart';
import 'package:goresave/loc/game_lang.dart';

/// Compact header identifying WHICH actor a detail pane is showing. Shared by
/// the Attribute and Inventory tab detail areas so both tabs make the selection
/// obvious above the sidebar/content.
///
/// - NPC selected → the localized display name (prominent). When
///   [showObjectIds] is enabled, the FULL GlobalId is selectable and wraps so
///   the user can copy/read the whole id — never ellipsized.
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

  /// Whether the actor's technical save identifier is rendered below its
  /// player-facing name. Knowledge-only orphans expose their real
  /// [Actor.uniqueName], never the synthetic `orphan:` selection sentinel.
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
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Icon(
            isPlayer
                ? Icons.person_outline
                : (isOrphan ? Icons.help_outline : Icons.face_outlined),
            color: scheme.primary,
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
                // Technical ids are opt-in. NPC GlobalIds wrap in full;
                // orphans expose the real knowledge key, not the synthetic
                // `orphan:` selection sentinel.
                if (showObjectIds &&
                    !isPlayer &&
                    technicalId != null &&
                    technicalId.isNotEmpty)
                  Padding(
                    padding: const EdgeInsets.only(top: 2),
                    child: SelectableText(
                      technicalId,
                      style: theme.textTheme.bodySmall?.copyWith(
                        color: scheme.onSurfaceVariant,
                        fontFamily: 'Consolas',
                      ),
                    ),
                  ),
              ],
            ),
          ),
        ],
      ),
    );
  }
}
