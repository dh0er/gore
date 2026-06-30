import 'package:flutter/material.dart';
import 'package:goresave/features/editor/domain/actor.dart';
import 'package:goresave/features/editor/ui/actor_selector.dart' show localizedNpcName;
import 'package:goresave/l10n/app_localizations.dart';
import 'package:goresave/loc/game_lang.dart';

/// Compact header identifying WHICH actor a detail pane is showing. Shared by
/// the Attribute and Inventory tab detail areas so both tabs make the selection
/// obvious above the sidebar/content.
///
/// - NPC selected → the localized display name (prominent) plus the FULL
///   GlobalId, fully visible (wraps to multiple lines if long, selectable) so
///   the user can copy/read the whole id — never ellipsized.
/// - Player selected → the localized "Player" label, no GlobalId (the player
///   has none).
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
  });

  /// The currently selected actor (player or NPC).
  final Actor actor;

  /// Loaded localization catalog (`id -> {set -> text}`) used to resolve the NPC
  /// display name.
  final Map<String, Map<String, String>> locCatalog;

  /// The current game language, driving which loc set the name resolves from.
  final GameLang lang;

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context);
    final theme = Theme.of(context);
    final scheme = theme.colorScheme;

    final isPlayer = actor.isPlayer;
    final id = actor.id;
    final name = isPlayer
        ? l10n.tabPlayer
        : localizedNpcName(locCatalog, lang, id ?? '');

    return Padding(
      padding: const EdgeInsets.fromLTRB(20, 16, 20, 0),
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Icon(
            isPlayer ? Icons.person_outline : Icons.face_outlined,
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
                // NPCs carry a long GlobalId; show it IN FULL (wrapping,
                // selectable) so it is always readable. The player has none.
                if (!isPlayer && id != null && id.isNotEmpty)
                  Padding(
                    padding: const EdgeInsets.only(top: 2),
                    child: SelectableText(
                      id,
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
