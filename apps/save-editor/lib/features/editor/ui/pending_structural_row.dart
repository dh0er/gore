import 'package:flutter/material.dart';

/// Whether a queued structural change adds something or takes it away. Only the
/// colour scheme differs; the two read as opposites at a glance.
enum PendingTone { add, remove }

/// A queued structural change, shown OUTSIDE the list it will affect.
///
/// An addition has no row to mark up — the thing does not exist on disk yet — so
/// showing it as if it were already there would claim a state the save does not
/// have. A tinted banner beside the list says the same thing honestly: this is
/// coming, and here is how to take it back.
///
/// Shared by the inventory and the trade panel so a queued change looks the same
/// wherever it is made.
class PendingStructuralRow extends StatelessWidget {
  const PendingStructuralRow({
    super.key,
    required this.icon,
    required this.title,
    required this.subtitle,
    required this.onCancel,
    required this.cancelTooltip,
    required this.tone,
    this.technicalId,
  });

  final IconData icon;
  final String title;
  final String subtitle;
  final VoidCallback onCancel;
  final String cancelTooltip;
  final PendingTone tone;

  /// Class path or id, shown monospaced under the subtitle when the user has
  /// technical ids switched on.
  final String? technicalId;

  @override
  Widget build(BuildContext context) {
    final scheme = Theme.of(context).colorScheme;
    final isAdd = tone == PendingTone.add;
    final bg = isAdd ? scheme.primaryContainer : scheme.errorContainer;
    final fg = isAdd ? scheme.onPrimaryContainer : scheme.onErrorContainer;
    final accent = isAdd ? scheme.primary : scheme.error;
    return DecoratedBox(
      decoration: BoxDecoration(
        color: bg,
        borderRadius: BorderRadius.circular(8),
        border: Border.all(color: accent.withValues(alpha: 0.4)),
      ),
      child: ListTile(
        dense: true,
        leading: Icon(icon, color: accent),
        title: Text(
          title,
          maxLines: 1,
          overflow: TextOverflow.ellipsis,
          style: TextStyle(color: fg),
        ),
        subtitle: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          mainAxisSize: MainAxisSize.min,
          children: [
            Text(subtitle, style: TextStyle(color: fg.withValues(alpha: 0.8))),
            if (technicalId?.trim().isNotEmpty == true)
              Text(
                technicalId!,
                maxLines: 2,
                overflow: TextOverflow.ellipsis,
                style: TextStyle(
                  color: fg.withValues(alpha: 0.72),
                  fontFamily: 'Consolas',
                  fontSize: 11,
                ),
              ),
          ],
        ),
        trailing: IconButton(
          icon: const Icon(Icons.close),
          tooltip: cancelTooltip,
          onPressed: onCancel,
        ),
      ),
    );
  }
}
