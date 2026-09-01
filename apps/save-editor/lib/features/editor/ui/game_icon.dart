import 'dart:io';

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:goresave/features/editor/domain/game_icons.dart';
import 'package:goresave/features/editor/domain/item_icon_catalog.dart';

/// A shared UI glyph extracted from the user's own game — the icon the game
/// itself draws in front of that label.
///
/// Resolution order, so a label is never left without a mark:
///   1. the glyph [name] asks for, when this generation carries it;
///   2. [fallbackIcon], the editor's own Material icon for that row;
///   3. the game's generic ◆ bullet, for rows that have no icon of their own;
///   4. a neutral Material bullet, when no game images are available at all
///      (no game installed, extraction not finished, unreadable cache).
class GameIcon extends StatelessWidget {
  const GameIcon({
    super.key,
    this.name,
    this.fallbackIcon,
    this.color,
    this.size = 18,
    this.tinted,
  });

  /// Texture name in `/Game/UI/Textures/Common/Icons`, e.g. `T_Icon_Mana`.
  /// Null means "this label has no glyph of its own".
  final String? name;

  /// Icon shown when the game glyph is unavailable. Null falls through to the
  /// game's ◆ bullet.
  final IconData? fallbackIcon;
  final Color? color;
  final double size;

  /// Whether to recolour the glyph. Null — the normal case — decides from the
  /// artwork itself: white line work is tinted so it reads on either theme, a
  /// finished picture is left alone (see [gameIconsWithOwnColours]).
  final bool? tinted;

  @override
  Widget build(BuildContext context) {
    // Panels are pumped without a ProviderScope in widget tests, and the game
    // images are an enhancement — so fall through to the icon rather than make
    // a scope a requirement of every panel that shows a label.
    final scoped =
        context.findAncestorWidgetOfExactType<UncontrolledProviderScope>() !=
        null;
    if (!scoped) return _icon(context);
    return Consumer(
      builder: (context, ref, _) =>
          _paint(context, ref.watch(itemIconCatalogProvider).value),
    );
  }

  Widget _paint(BuildContext context, ItemIconCatalog? catalog) {
    final glyph = name == null ? null : catalog?.uiPathFor(name!);
    final path =
        glyph ??
        (fallbackIcon == null
            ? catalog?.uiPathFor(gameIconGenericBullet)
            : null);
    if (path == null) return _icon(context);

    final pixels = (size * MediaQuery.devicePixelRatioOf(context)).ceil();
    return SizedBox.square(
      dimension: size,
      child: Image.file(
        File(path),
        fit: BoxFit.contain,
        filterQuality: FilterQuality.medium,
        gaplessPlayback: true,
        cacheWidth: pixels,
        cacheHeight: pixels,
        // The glyphs ship as white artwork on transparency, so they have to be
        // tinted to stay readable in the light theme. Taking the colour from
        // the surrounding IconTheme is what makes a glyph follow its context —
        // a selected tab tints its icon exactly like its label.
        color: (tinted ?? !gameIconsWithOwnColours.contains(name))
            ? _ink(context)
            : null,
        excludeFromSemantics: true,
        errorBuilder: (context, _, _) => _icon(context),
      ),
    );
  }

  Color _ink(BuildContext context) =>
      color ??
      IconTheme.of(context).color ??
      Theme.of(context).colorScheme.onSurfaceVariant;

  Widget _icon(BuildContext context) {
    return Icon(
      fallbackIcon ?? Icons.circle_outlined,
      // The neutral bullet stands in for a glyph, not for a category icon, so
      // it stays visually quieter than a real icon would be.
      size: fallbackIcon == null ? size * 0.7 : size,
      color: _ink(context),
    );
  }
}

/// A game glyph in front of a label, with the spacing the editor's rows use.
class GameIconLabel extends StatelessWidget {
  const GameIconLabel({
    super.key,
    required this.label,
    this.iconName,
    this.fallbackIcon,
    this.style,
    this.iconSize = 18,
    this.maxLines,
    this.overflow,
  });

  final String label;
  final String? iconName;
  final IconData? fallbackIcon;
  final TextStyle? style;
  final double iconSize;
  final int? maxLines;
  final TextOverflow? overflow;

  @override
  Widget build(BuildContext context) {
    return Row(
      mainAxisSize: MainAxisSize.min,
      crossAxisAlignment: CrossAxisAlignment.center,
      children: [
        GameIcon(
          name: iconName,
          fallbackIcon: fallbackIcon,
          size: iconSize,
          color: style?.color,
        ),
        const SizedBox(width: 8),
        Flexible(
          child: Text(
            label,
            style: style,
            maxLines: maxLines,
            overflow: overflow,
          ),
        ),
      ],
    );
  }
}
