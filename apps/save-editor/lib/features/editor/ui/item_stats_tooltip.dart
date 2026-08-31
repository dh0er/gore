import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:goresave/features/editor/domain/item_tooltip.dart';
import 'package:goresave/features/editor/ui/game_icon.dart';
import 'package:goresave/l10n/app_localizations.dart';
import 'package:goresave/loc/loc_catalog_provider.dart';
import 'package:goresave/providers/data_providers.dart';

const _cardWidth = 320.0;
const _cardGap = 12.0;
const _screenMargin = 8.0;
const _highlightFade = Duration(milliseconds: 90);

/// Marks a row as hovered and shows the game's own item card next to it.
///
/// The card appears the moment the pointer arrives, the way the game's does —
/// no dwell delay. Deliberately not a Material `Tooltip`: the game draws a
/// framed panel with a centred name, the item type below it, a divider,
/// right-aligned numbers and boxed requirement blocks, and a plain text bubble
/// cannot carry that — or the glyphs the rows use.
///
/// A row whose id the bundled item stats do not know keeps its plain content
/// and still highlights, so the list reacts the same everywhere.
class ItemStatsTooltip extends ConsumerStatefulWidget {
  const ItemStatsTooltip({
    super.key,
    required this.itemId,
    required this.title,
    required this.child,
    this.highlightOnHover = true,
  });

  final String itemId;

  /// The localized item name, already resolved by the caller (it is the same
  /// name the row prints).
  final String title;
  final Widget child;

  /// Whether to tint the row under the pointer. Off for rows that already react
  /// on their own (a tappable list tile brings its own hover colour).
  final bool highlightOnHover;

  @override
  ConsumerState<ItemStatsTooltip> createState() => _ItemStatsTooltipState();
}

class _ItemStatsTooltipState extends ConsumerState<ItemStatsTooltip> {
  final _portal = OverlayPortalController();

  /// Where the row sits, in the overlay's own coordinates. A notifier rather
  /// than a field: a wheel scroll moves the row without the pointer leaving it,
  /// so the card has to follow without a rebuild of this widget.
  final _anchor = ValueNotifier<Rect>(Rect.zero);

  /// The list this row scrolls in, while the card is up.
  ScrollPosition? _scroll;
  bool _hovering = false;

  /// Whether the last build put an [OverlayPortal] in the tree. Showing a
  /// detached controller would make the card appear on its own the moment the
  /// item stats finish loading, without the pointer being anywhere near.
  bool _hasCard = false;

  void _enter() {
    if (!_hovering) setState(() => _hovering = true);
    if (!_hasCard) return;
    _showCard();
  }

  void _showCard() {
    if (!_measure()) return;
    // Follow the row while the list scrolls under a pointer that never moves —
    // otherwise the card kept the position it was opened at and ended up
    // beside a different row.
    _scroll = Scrollable.maybeOf(context)?.position?..addListener(_followRow);
    _portal.show();
  }

  void _followRow() {
    if (!mounted || !_portal.isShowing) return;
    _measure();
  }

  bool _measure() {
    final box = context.findRenderObject() as RenderBox?;
    if (box == null || !box.hasSize) return false;
    // Measure against the overlay the card is placed in, NOT the screen. The
    // whole UI sits inside a scale transform, so screen coordinates are the
    // scaled ones while the overlay lays out in unscaled space — anchoring on
    // them slid the card further off the row the further the UI scale moved
    // from 100%.
    final overlay =
        Overlay.of(context, rootOverlay: true).context.findRenderObject()
            as RenderBox?;
    _anchor.value =
        box.localToGlobal(Offset.zero, ancestor: overlay) & box.size;
    return true;
  }

  void _exit() {
    if (_hovering) setState(() => _hovering = false);
    _scroll?.removeListener(_followRow);
    _scroll = null;
    if (_portal.isShowing) _portal.hide();
  }

  @override
  void dispose() {
    _scroll?.removeListener(_followRow);
    _anchor.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final scheme = Theme.of(context).colorScheme;
    final highlighted = widget.highlightOnHover && _hovering;
    final row = AnimatedContainer(
      duration: _highlightFade,
      curve: Curves.easeOut,
      decoration: BoxDecoration(
        color: highlighted
            ? scheme.primary.withValues(alpha: 0.08)
            : Colors.transparent,
        borderRadius: BorderRadius.circular(8),
      ),
      child: widget.child,
    );

    final stats = ref
        .watch(itemStatsCatalogProvider)
        .value
        ?.statsFor(widget.itemId);
    final tooltip = stats == null
        ? const ItemTooltip()
        : buildItemTooltip(
            title: widget.title,
            itemId: widget.itemId,
            stats: stats,
            catalog: ref.watch(locCatalogProvider).value ?? const {},
            lang: ref.watch(currentGameLangProvider),
            l10n: AppLocalizations.of(context),
          );

    final hadCard = _hasCard;
    _hasCard = !tooltip.isEmpty;
    if (_hovering && _hasCard && !hadCard) {
      // The pointer was already on the row while the item stats were still
      // loading, so there was no portal to show. It exists from this build on;
      // without this the card stayed away until the row was left and entered
      // again.
      WidgetsBinding.instance.addPostFrameCallback((_) {
        if (mounted && _hovering && _hasCard) _showCard();
      });
    }
    return MouseRegion(
      onEnter: (_) => _enter(),
      onExit: (_) => _exit(),
      child: tooltip.isEmpty
          ? row
          : OverlayPortal(
              controller: _portal,
              // The card never takes the pointer: it would otherwise steal the
              // hover it is showing for and flicker itself away.
              overlayChildBuilder: (context) => Positioned.fill(
                child: IgnorePointer(
                  child: ValueListenableBuilder<Rect>(
                    valueListenable: _anchor,
                    builder: (context, anchor, card) => CustomSingleChildLayout(
                      delegate: _BesideAnchor(anchor),
                      child: card,
                    ),
                    child: ItemTooltipCard(tooltip: tooltip),
                  ),
                ),
              ),
              child: row,
            ),
    );
  }
}

/// Places the card beside [anchor], the way the game puts it next to the
/// inventory grid: to the right when there is room, otherwise to the left, and
/// always fully on screen.
class _BesideAnchor extends SingleChildLayoutDelegate {
  const _BesideAnchor(this.anchor);

  final Rect anchor;

  @override
  BoxConstraints getConstraintsForChild(BoxConstraints constraints) {
    return BoxConstraints.loose(
      Size(
        _cardWidth.clamp(0.0, constraints.maxWidth),
        (constraints.maxHeight - 2 * _screenMargin).clamp(
          0.0,
          constraints.maxHeight,
        ),
      ),
    );
  }

  @override
  Offset getPositionForChild(Size size, Size childSize) {
    final rightOf = anchor.right + _cardGap;
    final leftOf = anchor.left - _cardGap - childSize.width;
    final x = rightOf + childSize.width <= size.width - _screenMargin
        ? rightOf
        : (leftOf >= _screenMargin
              ? leftOf
              : (size.width - childSize.width - _screenMargin).clamp(
                  _screenMargin,
                  size.width,
                ));
    // Top-aligned with the row, pulled up only as far as the screen forces.
    final maxY = (size.height - childSize.height - _screenMargin).clamp(
      _screenMargin,
      size.height,
    );
    final y = anchor.top.clamp(_screenMargin, maxY);
    return Offset(x, y);
  }

  @override
  bool shouldRelayout(_BesideAnchor oldDelegate) =>
      oldDelegate.anchor != anchor;
}

/// The card itself, in the game's own arrangement. Split out from the hover
/// plumbing so it can be laid out and read in a widget test directly.
class ItemTooltipCard extends StatelessWidget {
  const ItemTooltipCard({super.key, required this.tooltip});

  final ItemTooltip tooltip;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final scheme = theme.colorScheme;
    final accent = scheme.primary;
    final muted = scheme.onSurfaceVariant;

    return Material(
      type: MaterialType.transparency,
      child: Container(
        decoration: BoxDecoration(
          color: scheme.surfaceContainerLowest,
          border: Border.all(color: accent.withValues(alpha: 0.55)),
          borderRadius: BorderRadius.circular(4),
          boxShadow: [
            BoxShadow(
              color: Colors.black.withValues(alpha: 0.35),
              blurRadius: 18,
              offset: const Offset(0, 6),
            ),
          ],
        ),
        padding: const EdgeInsets.fromLTRB(14, 12, 14, 12),
        child: SingleChildScrollView(
          child: Column(
            mainAxisSize: MainAxisSize.min,
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: [
              Text(
                tooltip.title,
                textAlign: TextAlign.center,
                style: theme.textTheme.titleSmall?.copyWith(
                  color: scheme.onSurface,
                  fontWeight: FontWeight.w600,
                  letterSpacing: 0.8,
                ),
              ),
              if (tooltip.subtitle.isNotEmpty) ...[
                const SizedBox(height: 2),
                Text(
                  tooltip.subtitle,
                  textAlign: TextAlign.center,
                  style: theme.textTheme.bodySmall?.copyWith(color: muted),
                ),
              ],
              if (tooltip.stats.isNotEmpty ||
                  tooltip.protection.isNotEmpty ||
                  tooltip.requirements.isNotEmpty ||
                  tooltip.description.isNotEmpty)
                _DiamondDivider(color: accent.withValues(alpha: 0.5)),
              for (final row in tooltip.stats) _Row(row: row, accent: accent),
              if (tooltip.protection.isNotEmpty)
                _Block(
                  label: tooltip.protectionLabel,
                  rows: tooltip.protection,
                  accent: accent,
                ),
              if (tooltip.requirements.isNotEmpty)
                _Block(
                  label: tooltip.requirementsLabel,
                  rows: tooltip.requirements,
                  accent: accent,
                ),
              if (tooltip.recipe.isNotEmpty)
                _Block(
                  label: tooltip.recipeLabel,
                  rows: tooltip.recipe,
                  accent: accent,
                ),
              if (tooltip.ingredientFor.isNotEmpty)
                _Block(
                  label: tooltip.ingredientForLabel,
                  rows: tooltip.ingredientFor,
                  accent: accent,
                ),
              // A writing's own text, set like the flavour line below it but
              // paragraph by paragraph. A chapter heading keeps its rank: the
              // asset records which passages are headings, and flattening them
              // into the body threw the document's structure away.
              for (final paragraph in tooltip.writing) ...[
                const SizedBox(height: 8),
                Text(
                  paragraph.text,
                  style: theme.textTheme.bodySmall?.copyWith(
                    color: paragraph.isHeading ? accent : muted,
                    fontStyle: paragraph.isHeading
                        ? FontStyle.normal
                        : FontStyle.italic,
                    fontWeight: paragraph.isHeading ? FontWeight.w600 : null,
                    height: 1.35,
                  ),
                ),
              ],
              if (tooltip.description.isNotEmpty) ...[
                const SizedBox(height: 8),
                Text(
                  tooltip.description,
                  style: theme.textTheme.bodySmall?.copyWith(
                    color: muted,
                    fontStyle: FontStyle.italic,
                    height: 1.35,
                  ),
                ),
              ],
            ],
          ),
        ),
      ),
    );
  }
}

/// The game's section rule: a hairline with a small diamond centred on it.
class _DiamondDivider extends StatelessWidget {
  const _DiamondDivider({required this.color});

  final Color color;

  @override
  Widget build(BuildContext context) {
    final line = Expanded(child: Container(height: 1, color: color));
    return Padding(
      padding: const EdgeInsets.symmetric(vertical: 8),
      child: Row(
        children: [
          line,
          Padding(
            padding: const EdgeInsets.symmetric(horizontal: 6),
            child: Transform.rotate(
              angle: 0.785398, // 45°
              child: Container(
                width: 5,
                height: 5,
                decoration: BoxDecoration(border: Border.all(color: color)),
              ),
            ),
          ),
          line,
        ],
      ),
    );
  }
}

/// A labelled, boxed group — the game frames its protection and requirement
/// lists this way, set off from the plain numbers above them.
class _Block extends StatelessWidget {
  const _Block({required this.label, required this.rows, required this.accent});

  final String label;
  final List<ItemTooltipRow> rows;
  final Color accent;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Container(
      margin: const EdgeInsets.only(top: 8),
      padding: const EdgeInsets.fromLTRB(8, 6, 8, 6),
      decoration: BoxDecoration(
        color: theme.colorScheme.surfaceContainerHighest.withValues(alpha: 0.6),
        borderRadius: BorderRadius.circular(3),
      ),
      child: Column(
        mainAxisSize: MainAxisSize.min,
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          if (label.isNotEmpty)
            Text(
              label,
              style: theme.textTheme.bodySmall?.copyWith(
                color: theme.colorScheme.onSurfaceVariant,
              ),
            ),
          for (final row in rows) _Row(row: row, accent: accent),
        ],
      ),
    );
  }
}

/// One line: glyph, label, and the value the game right-aligns against it.
class _Row extends StatelessWidget {
  const _Row({required this.row, required this.accent});

  final ItemTooltipRow row;
  final Color accent;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Padding(
      padding: const EdgeInsets.symmetric(vertical: 2),
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.center,
        children: [
          GameIcon(name: row.iconName, size: 14, color: accent),
          const SizedBox(width: 6),
          Expanded(
            child: Text(
              row.label,
              style: theme.textTheme.bodySmall?.copyWith(
                color: theme.colorScheme.onSurface,
              ),
            ),
          ),
          if (row.value.isNotEmpty) ...[
            const SizedBox(width: 12),
            Text(
              row.value,
              style: theme.textTheme.bodySmall?.copyWith(
                color: accent,
                fontWeight: FontWeight.w600,
                fontFeatures: const [FontFeature.tabularFigures()],
              ),
            ),
          ],
        ],
      ),
    );
  }
}
