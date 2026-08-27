import 'dart:math' as math;

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:goresave/features/editor/domain/item_icon_catalog.dart';
import 'package:goresave/features/localization/domain/localization_controller.dart';
import 'package:goresave/l10n/app_localizations.dart';
import 'package:goresave/loc/loc_catalog_provider.dart';

/// Compact, truthful activity display for the two first-run asset jobs.
///
/// The native core currently exposes each extraction as one request rather
/// than a stream of per-file progress events, so these bars are intentionally
/// indeterminate. They disappear as soon as their corresponding job settles.
class TitlePreparationProgress extends ConsumerWidget {
  const TitlePreparationProgress({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final localizationRunning = ref.watch(
      localizationControllerProvider.select((state) => state.isRunning),
    );
    final localizationCatalogLoading = ref.watch(
      locCatalogProvider.select((value) => value.isLoading && !value.hasValue),
    );
    final itemImagesLoading = ref.watch(
      itemIconCatalogProvider.select(
        (value) => value.isLoading && !value.hasValue,
      ),
    );
    final localizationActive =
        localizationRunning || localizationCatalogLoading;
    if (!localizationActive && !itemImagesLoading) {
      return const SizedBox.shrink();
    }

    final l10n = AppLocalizations.of(context);
    final tasks = <({Key key, IconData icon, String label})>[
      if (localizationActive)
        (
          key: const ValueKey('title-progress-game-text'),
          icon: Icons.translate,
          label: l10n.loadingTexts,
        ),
      if (itemImagesLoading)
        (
          key: const ValueKey('title-progress-item-images'),
          icon: Icons.image_outlined,
          label: l10n.loadingImages,
        ),
    ];
    final semanticLabel = tasks.map((task) => task.label).join(', ');

    return Semantics(
      liveRegion: true,
      container: true,
      label: semanticLabel,
      excludeSemantics: true,
      child: LayoutBuilder(
        builder: (context, constraints) {
          final maximumWidth = constraints.maxWidth.isFinite
              ? constraints.maxWidth
              : 344.0;
          if (tasks.length == 2 && maximumWidth < 344) {
            return Align(
              alignment: Alignment.center,
              child: _PreparationTask(
                key: const ValueKey('title-progress-combined'),
                icon: Icons.sync,
                label: tasks.map((task) => task.label).join(' · '),
                width: math.min(260, maximumWidth),
              ),
            );
          }
          final taskWidth = math.min(164.0, maximumWidth);
          return Row(
            mainAxisSize: MainAxisSize.min,
            mainAxisAlignment: MainAxisAlignment.center,
            children: [
              for (var index = 0; index < tasks.length; index++) ...[
                if (index > 0) const SizedBox(width: 16),
                _PreparationTask(
                  key: tasks[index].key,
                  icon: tasks[index].icon,
                  label: tasks[index].label,
                  width: taskWidth,
                ),
              ],
            ],
          );
        },
      ),
    );
  }
}

class _PreparationTask extends StatelessWidget {
  const _PreparationTask({
    super.key,
    required this.icon,
    required this.label,
    required this.width,
  });

  final IconData icon;
  final String label;
  final double width;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final scheme = theme.colorScheme;
    return Tooltip(
      message: label,
      child: SizedBox(
        width: width,
        child: Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Row(
              children: [
                Icon(icon, size: 14, color: scheme.onSurfaceVariant),
                const SizedBox(width: 5),
                Expanded(
                  child: Text(
                    label,
                    maxLines: 1,
                    overflow: TextOverflow.ellipsis,
                    style: theme.textTheme.labelSmall?.copyWith(
                      color: scheme.onSurfaceVariant,
                    ),
                  ),
                ),
              ],
            ),
            const SizedBox(height: 4),
            LinearProgressIndicator(
              minHeight: 3,
              borderRadius: BorderRadius.circular(2),
            ),
          ],
        ),
      ),
    );
  }
}
