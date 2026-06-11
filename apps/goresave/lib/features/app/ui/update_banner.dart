import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:goresave/features/app/domain/update_notifier.dart';

/// Wraps the app content and shows a slim banner above it once an update has
/// been downloaded and staged.
class UpdateBannerHost extends ConsumerWidget {
  const UpdateBannerHost({required this.child, super.key});

  final Widget child;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final state = ref.watch(updateProvider);
    if (state is! UpdateReady) {
      return child;
    }
    final theme = Theme.of(context);
    final onContainer = theme.colorScheme.onPrimaryContainer;
    return Column(
      children: [
        Material(
          color: theme.colorScheme.primaryContainer,
          child: Padding(
            padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 4),
            child: Row(
              children: [
                Icon(Icons.system_update_alt, size: 16, color: onContainer),
                const SizedBox(width: 8),
                Expanded(
                  child: Text(
                    'Update ${state.version} ready',
                    style: theme.textTheme.bodySmall?.copyWith(
                      color: onContainer,
                    ),
                  ),
                ),
                TextButton(
                  onPressed: () =>
                      ref.read(updateProvider.notifier).applyAndRestart(),
                  child: const Text('Restart to update'),
                ),
              ],
            ),
          ),
        ),
        Expanded(child: child),
      ],
    );
  }
}
