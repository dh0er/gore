import 'package:flutter/material.dart';

/// Presentation-only Settings / Expert destination for managed projects.
///
/// Copy and the actual settings surface are injected by the owning shell. This
/// widget owns no settings state, dialog, project mutation, or expert-mode
/// authority.
@immutable
final class Revision3SettingsExpertPage extends StatelessWidget {
  const Revision3SettingsExpertPage({
    required this.title,
    required this.description,
    required this.expertStatusLabel,
    required this.expertStatusDescription,
    required this.settings,
    super.key,
  });

  final String title;
  final String description;
  final String expertStatusLabel;
  final String expertStatusDescription;
  final Widget settings;

  @override
  Widget build(BuildContext context) => Semantics(
    key: const Key('revision3-settings-expert-page'),
    container: true,
    explicitChildNodes: true,
    child: LayoutBuilder(
      builder: (context, constraints) {
        final proportionalHeaderHeight = constraints.maxHeight.isFinite
            ? constraints.maxHeight * 0.45
            : 240.0;
        final maximumHeaderHeight = proportionalHeaderHeight < 240
            ? proportionalHeaderHeight
            : 240.0;
        return Column(
          children: [
            ConstrainedBox(
              constraints: BoxConstraints(maxHeight: maximumHeaderHeight),
              child: Material(
                key: const Key('revision3-settings-expert-page-header'),
                color: Theme.of(context).colorScheme.surfaceContainerLow,
                child: SingleChildScrollView(
                  key: const Key(
                    'revision3-settings-expert-page-header-scroll',
                  ),
                  padding: const EdgeInsets.all(20),
                  child: LayoutBuilder(
                    builder: (context, headerConstraints) {
                      final identity = _SettingsIdentity(
                        title: title,
                        description: description,
                      );
                      final status = _ExpertStatus(
                        label: expertStatusLabel,
                        description: expertStatusDescription,
                      );
                      if (headerConstraints.maxWidth < 680) {
                        return Column(
                          crossAxisAlignment: CrossAxisAlignment.stretch,
                          children: [
                            identity,
                            const SizedBox(height: 14),
                            status,
                          ],
                        );
                      }
                      return Row(
                        crossAxisAlignment: CrossAxisAlignment.start,
                        children: [
                          Expanded(child: identity),
                          const SizedBox(width: 20),
                          Flexible(
                            child: ConstrainedBox(
                              constraints: const BoxConstraints(maxWidth: 380),
                              child: status,
                            ),
                          ),
                        ],
                      );
                    },
                  ),
                ),
              ),
            ),
            const Divider(height: 1),
            Expanded(
              child: KeyedSubtree(
                key: const Key('revision3-settings-expert-page-settings'),
                child: settings,
              ),
            ),
          ],
        );
      },
    ),
  );
}

class _SettingsIdentity extends StatelessWidget {
  const _SettingsIdentity({required this.title, required this.description});

  final String title;
  final String description;

  @override
  Widget build(BuildContext context) => Row(
    crossAxisAlignment: CrossAxisAlignment.start,
    children: [
      const Icon(Icons.settings_outlined, size: 30),
      const SizedBox(width: 12),
      Expanded(
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Semantics(
              key: const Key('revision3-settings-expert-page-title'),
              container: true,
              header: true,
              child: Text(
                title,
                style: Theme.of(context).textTheme.headlineSmall,
              ),
            ),
            const SizedBox(height: 6),
            Text(
              description,
              key: const Key('revision3-settings-expert-page-description'),
            ),
          ],
        ),
      ),
    ],
  );
}

class _ExpertStatus extends StatelessWidget {
  const _ExpertStatus({required this.label, required this.description});

  final String label;
  final String description;

  @override
  Widget build(BuildContext context) {
    final scheme = Theme.of(context).colorScheme;
    return Semantics(
      key: const Key('revision3-settings-expert-page-status'),
      container: true,
      explicitChildNodes: true,
      child: Container(
        padding: const EdgeInsets.all(14),
        decoration: BoxDecoration(
          color: scheme.secondaryContainer,
          borderRadius: BorderRadius.circular(12),
        ),
        child: Row(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Icon(Icons.science_outlined, color: scheme.onSecondaryContainer),
            const SizedBox(width: 10),
            Expanded(
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Text(
                    label,
                    key: const Key(
                      'revision3-settings-expert-page-status-label',
                    ),
                    style: Theme.of(context).textTheme.titleSmall?.copyWith(
                      color: scheme.onSecondaryContainer,
                    ),
                  ),
                  const SizedBox(height: 4),
                  Text(
                    description,
                    key: const Key(
                      'revision3-settings-expert-page-status-description',
                    ),
                    style: TextStyle(color: scheme.onSecondaryContainer),
                  ),
                ],
              ),
            ),
          ],
        ),
      ),
    );
  }
}
