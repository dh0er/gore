import 'dart:io';

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:goresave/features/editor/domain/item_icon_catalog.dart';

/// Shows the matching image extracted from the user's game, with the existing
/// Material category icon as a stable loading/error/unknown-item fallback.
class InventoryItemVisual extends ConsumerWidget {
  const InventoryItemVisual({
    super.key,
    required this.itemId,
    this.itemPath = '',
    this.fallbackIcon = Icons.category_outlined,
    this.fallbackColor,
    this.size = 40,
  });

  final String itemId;
  final String itemPath;
  final IconData fallbackIcon;
  final Color? fallbackColor;
  final double size;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final catalog = ref.watch(itemIconCatalogProvider).value;
    final path = catalog?.pathFor(itemId: itemId, itemPath: itemPath);
    final frameInset = size <= 28 ? 0.75 : 1.0;
    final imagePadding = size <= 28 ? 1.0 : 2.0;
    final imageSize = (size - (2 * frameInset) - (2 * imagePadding))
        .clamp(1.0, size)
        .toDouble();

    final content = path == null
        ? Icon(fallbackIcon, color: fallbackColor, size: imageSize * 0.6)
        : Image.file(
            File(path),
            fit: BoxFit.contain,
            filterQuality: FilterQuality.medium,
            gaplessPlayback: true,
            cacheWidth: (imageSize * MediaQuery.devicePixelRatioOf(context))
                .ceil(),
            cacheHeight: (imageSize * MediaQuery.devicePixelRatioOf(context))
                .ceil(),
            excludeFromSemantics: true,
            errorBuilder: (_, _, _) =>
                Icon(fallbackIcon, color: fallbackColor, size: imageSize * 0.6),
          );

    return SizedBox.square(
      dimension: size,
      child: Padding(
        padding: EdgeInsets.all(frameInset),
        child: Padding(padding: EdgeInsets.all(imagePadding), child: content),
      ),
    );
  }
}
