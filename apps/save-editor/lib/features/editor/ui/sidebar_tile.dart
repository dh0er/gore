import 'package:flutter/material.dart';

import 'package:goresave/features/editor/domain/item_categories.dart';
import 'package:goresave/features/editor/ui/game_icon.dart';
import 'package:goresave/l10n/app_localizations.dart';
import 'package:goresave/ui/design/app_theme.dart';

/// Pieces of [AppLocalizations.categoryWithCount]: the catalog name, and the
/// interface wrapper around it (` (3)`, `（3）`).
class CatalogCountParts {
  const CatalogCountParts(this.full, this.run, this.lead, this.tail);

  final String full;
  final String run;
  final String lead;
  final String tail;

  bool get splits => full == '$lead$run$tail' && run.isNotEmpty;
}

CatalogCountParts catalogCountParts(
  AppLocalizations l10n,
  String name,
  int count,
) {
  const mark = '\uE000';
  final full = l10n.categoryWithCount(name, count);
  if (name.contains(mark)) return CatalogCountParts(full, name, '', '');
  final probe = l10n.categoryWithCount(mark, count);
  final at = probe.indexOf(mark);
  if (at < 0) return CatalogCountParts(full, name, '', '');
  final lead = probe.substring(0, at);
  final tail = probe.substring(at + mark.length);
  if (full != '$lead$name$tail') return CatalogCountParts(full, name, '', '');
  return CatalogCountParts(full, name, lead, tail);
}

/// A selectable left-sidebar row, matching the Player/Progression tab style.
class SidebarTile extends StatelessWidget {
  const SidebarTile({
    super.key,
    required this.icon,
    required this.label,
    required this.selected,
    required this.onTap,
    this.gameIcon,
    this.gameTextLocale,
    this.catalogRun,
    this.catalogLead = '',
    this.catalogTail = '',
  });

  final IconData icon;

  /// Shared game glyph shown instead of [icon] when the user's install has been
  /// read. Null (or a glyph this game build lacks) keeps [icon].
  final String? gameIcon;
  final String label;
  final bool selected;
  final VoidCallback onTap;

  /// When set, [label] is game text and uses that script's face.
  ///
  /// [catalogRun] is the catalog slice inside a mixed [label]. [catalogLead]
  /// and [catalogTail] stay on the interface face.
  final Locale? gameTextLocale;
  final String? catalogRun;
  final String catalogLead;
  final String catalogTail;

  bool get _mixedCatalog =>
      catalogRun != null &&
      catalogRun!.isNotEmpty &&
      label == '$catalogLead$catalogRun$catalogTail';

  /// The label, ellipsized to one line, wrapped in a [Tooltip] ONLY when it
  /// actually does not fit. A tooltip that repeats text the user can already
  /// read in full is noise, so the row is measured against its own width first
  /// (`TextPainter.didExceedMaxLines`) with the same style, scale and direction
  /// the `Text` will use — otherwise the measurement and the render disagree.
  Widget _label(BuildContext context, TextStyle? uiStyle) {
    final mixed = _mixedCatalog;
    final run = catalogRun ?? '';
    final gameStyle = !mixed || gameTextLocale == null
        ? null
        : gameScriptTextStyle(context, gameTextLocale!, style: uiStyle);
    final span = !mixed
        ? TextSpan(text: label, style: uiStyle)
        : TextSpan(
            children: [
              if (catalogLead.isNotEmpty)
                TextSpan(text: catalogLead, style: uiStyle),
              TextSpan(text: run, style: gameStyle ?? uiStyle),
              if (catalogTail.isNotEmpty)
                TextSpan(text: catalogTail, style: uiStyle),
            ],
          );
    return LayoutBuilder(
      builder: (context, constraints) {
        final text = Text.rich(
          span,
          maxLines: 1,
          overflow: TextOverflow.ellipsis,
        );
        final painter = TextPainter(
          text: span,
          maxLines: 1,
          textDirection: Directionality.of(context),
          textScaler: MediaQuery.textScalerOf(context),
        )..layout(maxWidth: constraints.maxWidth);
        if (!painter.didExceedMaxLines) return text;
        if (!mixed || gameTextLocale == null) {
          return Tooltip(message: label, child: text);
        }
        final font = gameScriptTextStyle(context, gameTextLocale!);
        if (font == null) return Tooltip(message: label, child: text);
        return Tooltip(
          richMessage: TextSpan(
            children: [
              if (catalogLead.isNotEmpty) TextSpan(text: catalogLead),
              TextSpan(text: run, style: font),
              if (catalogTail.isNotEmpty) TextSpan(text: catalogTail),
            ],
          ),
          child: text,
        );
      },
    );
  }

  TextStyle? _uiStyle(BuildContext context, ColorScheme scheme) {
    return Theme.of(context).textTheme.bodyMedium?.copyWith(
      color: selected ? scheme.primary : scheme.onSurface,
      fontWeight: selected ? FontWeight.w600 : null,
    );
  }

  @override
  Widget build(BuildContext context) {
    final scheme = Theme.of(context).colorScheme;
    return Padding(
      padding: const EdgeInsets.symmetric(horizontal: 6, vertical: 2),
      child: Material(
        color: selected ? scheme.primaryContainer : Colors.transparent,
        borderRadius: BorderRadius.circular(8),
        child: InkWell(
          onTap: onTap,
          borderRadius: BorderRadius.circular(8),
          child: Padding(
            padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 9),
            child: Row(
              children: [
                GameIcon(
                  name: gameIcon,
                  fallbackIcon: icon,
                  size: 18,
                  color: selected ? scheme.primary : scheme.onSurfaceVariant,
                ),
                const SizedBox(width: 8),
                Expanded(
                  child: _label(
                    context,
                    catalogRun == null
                        ? gameScriptTextStyle(
                            context,
                            gameTextLocale ?? Localizations.localeOf(context),
                            style: _uiStyle(context, scheme),
                          )
                        : _uiStyle(context, scheme),
                  ),
                ),
              ],
            ),
          ),
        ),
      ),
    );
  }
}

/// Chip label: catalog name on the game-text face, count wrapper on the UI face.
Widget catalogCountChipLabel(
  BuildContext context, {
  required AppLocalizations l10n,
  required String name,
  required int count,
  Locale? gameLocale,
}) {
  final parts = catalogCountParts(l10n, name, count);
  final font = gameLocale == null || !parts.splits
      ? null
      : gameScriptTextStyle(context, gameLocale);
  if (font == null) return Text(parts.full);
  return Text.rich(
    TextSpan(
      children: [
        if (parts.lead.isNotEmpty) TextSpan(text: parts.lead),
        TextSpan(text: parts.run, style: font),
        if (parts.tail.isNotEmpty) TextSpan(text: parts.tail),
      ],
    ),
  );
}

/// Material icon for an item category, used by inventory sidebars.
IconData iconForItemCategory(ItemCategory category) {
  switch (category) {
    case ItemCategory.meleeWeapon:
      return Icons.gavel;
    case ItemCategory.rangedWeapon:
      return Icons.gps_fixed;
    case ItemCategory.magic:
      return Icons.auto_awesome;
    case ItemCategory.wearable:
      return Icons.shield_outlined;
    case ItemCategory.food:
      return Icons.restaurant;
    case ItemCategory.potion:
      return Icons.science_outlined;
    case ItemCategory.material:
      return Icons.diamond_outlined;
    case ItemCategory.document:
      return Icons.menu_book_outlined;
    case ItemCategory.misc:
      return Icons.category_outlined;
    case ItemCategory.artefact:
      return Icons.vpn_key_outlined;
    case ItemCategory.other:
      return Icons.help_outline;
  }
}
