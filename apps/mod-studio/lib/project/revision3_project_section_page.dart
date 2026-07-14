import 'package:flutter/material.dart';

/// Visual emphasis for a presentation-only project-section status.
///
/// The value selects a Material color role only. It does not imply that the
/// status was verified or that any authoring, build, or runtime capability is
/// available.
enum Revision3ProjectSectionStatusSeverity {
  neutral,
  success,
  warning,
  blocked,
}

/// Immutable presentation data for one status card.
@immutable
final class Revision3ProjectSectionStatusCard {
  Revision3ProjectSectionStatusCard({
    required String id,
    required this.icon,
    required this.title,
    required this.description,
    this.valueText,
    this.severity = Revision3ProjectSectionStatusSeverity.neutral,
  }) : assert(_isKebabSafeId(id), 'id must be a non-empty kebab-safe ID'),
       id = _requireKebabSafeId(id, 'id');

  final String id;
  final IconData icon;
  final String title;
  final String description;
  final String? valueText;
  final Revision3ProjectSectionStatusSeverity severity;
}

/// Immutable presentation data for one discoverable section action.
///
/// A null [onPressed] intentionally leaves the action visible while exposing
/// it as disabled to assistive technologies.
@immutable
final class Revision3ProjectSectionActionCard {
  Revision3ProjectSectionActionCard({
    required String id,
    required this.icon,
    required this.title,
    required this.description,
    this.onPressed,
    this.badge,
  }) : assert(_isKebabSafeId(id), 'id must be a non-empty kebab-safe ID'),
       id = _requireKebabSafeId(id, 'id');

  final String id;
  final IconData icon;
  final String title;
  final String description;
  final VoidCallback? onPressed;
  final String? badge;
}

/// Reusable presentation-only landing page for a canonical project section.
///
/// All author-facing copy is supplied by the caller. This widget owns no
/// loading, navigation, mutation, validation, build, or runtime authority.
@immutable
final class Revision3ProjectSectionPage extends StatelessWidget {
  Revision3ProjectSectionPage({
    required String sectionId,
    required this.icon,
    required this.title,
    required this.description,
    this.notice,
    this.statusHeading,
    this.actionHeading,
    List<Revision3ProjectSectionStatusCard> statusCards = const [],
    List<Revision3ProjectSectionActionCard> actionCards = const [],
    super.key,
  }) : assert(
         _isKebabSafeId(sectionId),
         'sectionId must be a non-empty kebab-safe ID',
       ),
       assert(
         _statusIdsAreUnique(statusCards),
         'status card IDs must be non-empty, kebab-safe, and unique',
       ),
       assert(
         _actionIdsAreUnique(actionCards),
         'action card IDs must be non-empty, kebab-safe, and unique',
       ),
       sectionId = _requireKebabSafeId(sectionId, 'sectionId'),
       statusCards = _validatedStatusCards(statusCards),
       actionCards = _validatedActionCards(actionCards);

  final String sectionId;
  final IconData icon;
  final String title;
  final String description;
  final String? notice;
  final String? statusHeading;
  final String? actionHeading;
  final List<Revision3ProjectSectionStatusCard> statusCards;
  final List<Revision3ProjectSectionActionCard> actionCards;

  @override
  Widget build(BuildContext context) => LayoutBuilder(
    builder: (context, constraints) {
      final viewportWidth = constraints.maxWidth.isFinite
          ? constraints.maxWidth
          : _maximumContentWidth;
      final horizontalPadding = switch (viewportWidth) {
        < 480 => 12.0,
        < 900 => 20.0,
        _ => 32.0,
      };
      final verticalPadding = viewportWidth < 480 ? 16.0 : 28.0;

      return SingleChildScrollView(
        key: Key('revision3-project-section-$sectionId-scroll'),
        padding: EdgeInsets.symmetric(
          horizontal: horizontalPadding,
          vertical: verticalPadding,
        ),
        child: Align(
          alignment: Alignment.topCenter,
          child: ConstrainedBox(
            constraints: const BoxConstraints(maxWidth: _maximumContentWidth),
            child: SizedBox(
              width: double.infinity,
              child: Semantics(
                key: Key('revision3-project-section-$sectionId-page'),
                container: true,
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.stretch,
                  children: [
                    _SectionHeader(
                      sectionId: sectionId,
                      icon: icon,
                      title: title,
                      description: description,
                    ),
                    if (notice != null) ...[
                      const SizedBox(height: 16),
                      _SectionNotice(sectionId: sectionId, copy: notice!),
                    ],
                    if (statusCards.isNotEmpty) ...[
                      const SizedBox(height: 28),
                      if (statusHeading != null) ...[
                        _SectionHeading(
                          key: Key(
                            'revision3-project-section-$sectionId-status-heading',
                          ),
                          copy: statusHeading!,
                        ),
                        const SizedBox(height: 12),
                      ],
                      _ResponsiveCardGrid(
                        key: Key(
                          'revision3-project-section-$sectionId-statuses',
                        ),
                        children: [
                          for (final status in statusCards)
                            _StatusCard(sectionId: sectionId, status: status),
                        ],
                      ),
                    ],
                    if (actionCards.isNotEmpty) ...[
                      const SizedBox(height: 28),
                      if (actionHeading != null) ...[
                        _SectionHeading(
                          key: Key(
                            'revision3-project-section-$sectionId-action-heading',
                          ),
                          copy: actionHeading!,
                        ),
                        const SizedBox(height: 12),
                      ],
                      _ResponsiveCardGrid(
                        key: Key(
                          'revision3-project-section-$sectionId-actions',
                        ),
                        children: [
                          for (final action in actionCards)
                            _ActionCard(sectionId: sectionId, action: action),
                        ],
                      ),
                    ],
                  ],
                ),
              ),
            ),
          ),
        ),
      );
    },
  );
}

class _SectionHeader extends StatelessWidget {
  const _SectionHeader({
    required this.sectionId,
    required this.icon,
    required this.title,
    required this.description,
  });

  final String sectionId;
  final IconData icon;
  final String title;
  final String description;

  @override
  Widget build(BuildContext context) {
    final scheme = Theme.of(context).colorScheme;
    final iconPanel = DecoratedBox(
      decoration: BoxDecoration(
        color: scheme.primaryContainer,
        borderRadius: BorderRadius.circular(16),
      ),
      child: SizedBox.square(
        dimension: 56,
        child: Icon(icon, color: scheme.onPrimaryContainer, size: 30),
      ),
    );
    final copy = Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Semantics(
          container: true,
          header: true,
          child: Text(title, style: Theme.of(context).textTheme.headlineSmall),
        ),
        const SizedBox(height: 8),
        Text(
          description,
          style: Theme.of(context).textTheme.bodyLarge?.copyWith(
            color: scheme.onSurfaceVariant,
            height: 1.4,
          ),
        ),
      ],
    );

    return Material(
      key: Key('revision3-project-section-$sectionId-header'),
      color: scheme.surfaceContainerLow,
      shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(20)),
      clipBehavior: Clip.antiAlias,
      child: Semantics(
        container: true,
        child: Padding(
          padding: const EdgeInsets.all(22),
          child: LayoutBuilder(
            builder: (context, constraints) {
              if (constraints.maxWidth < 420) {
                return Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [iconPanel, const SizedBox(height: 16), copy],
                );
              }
              return Row(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  iconPanel,
                  const SizedBox(width: 18),
                  Expanded(child: copy),
                ],
              );
            },
          ),
        ),
      ),
    );
  }
}

class _SectionNotice extends StatelessWidget {
  const _SectionNotice({required this.sectionId, required this.copy});

  final String sectionId;
  final String copy;

  @override
  Widget build(BuildContext context) {
    final scheme = Theme.of(context).colorScheme;
    return Material(
      key: Key('revision3-project-section-$sectionId-notice'),
      color: scheme.secondaryContainer,
      shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(14)),
      child: Semantics(
        container: true,
        child: Padding(
          padding: const EdgeInsets.all(16),
          child: Row(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Icon(Icons.info_outline, color: scheme.onSecondaryContainer),
              const SizedBox(width: 12),
              Expanded(
                child: Text(
                  copy,
                  style: TextStyle(
                    color: scheme.onSecondaryContainer,
                    height: 1.35,
                  ),
                ),
              ),
            ],
          ),
        ),
      ),
    );
  }
}

class _SectionHeading extends StatelessWidget {
  const _SectionHeading({required this.copy, super.key});

  final String copy;

  @override
  Widget build(BuildContext context) => Semantics(
    header: true,
    child: Text(copy, style: Theme.of(context).textTheme.titleLarge),
  );
}

class _ResponsiveCardGrid extends StatelessWidget {
  const _ResponsiveCardGrid({required this.children, super.key});

  final List<Widget> children;

  @override
  Widget build(BuildContext context) => LayoutBuilder(
    builder: (context, constraints) {
      final availableWidth = constraints.maxWidth.isFinite
          ? constraints.maxWidth
          : _maximumContentWidth;
      final columns = switch (availableWidth) {
        >= 1040 => 3,
        >= 600 => 2,
        _ => 1,
      };
      const gap = 16.0;
      final usableWidth = availableWidth - ((columns - 1) * gap);
      final cardWidth = usableWidth > 0 ? usableWidth / columns : 0.0;

      return Wrap(
        spacing: gap,
        runSpacing: gap,
        children: [
          for (final child in children)
            SizedBox(width: cardWidth, child: child),
        ],
      );
    },
  );
}

class _StatusCard extends StatelessWidget {
  const _StatusCard({required this.sectionId, required this.status});

  final String sectionId;
  final Revision3ProjectSectionStatusCard status;

  @override
  Widget build(BuildContext context) {
    final palette = _statusPalette(
      Theme.of(context).colorScheme,
      status.severity,
    );
    return Semantics(
      container: true,
      child: Material(
        key: Key('revision3-project-section-$sectionId-status-${status.id}'),
        color: palette.background,
        shape: RoundedRectangleBorder(
          borderRadius: BorderRadius.circular(16),
          side: BorderSide(color: palette.foreground.withValues(alpha: 0.12)),
        ),
        child: Padding(
          padding: const EdgeInsets.all(18),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Icon(status.icon, color: palette.accent, size: 26),
              const SizedBox(height: 14),
              Text(
                status.title,
                style: Theme.of(context).textTheme.titleMedium?.copyWith(
                  color: palette.foreground,
                  fontWeight: FontWeight.w600,
                ),
              ),
              const SizedBox(height: 6),
              Text(
                status.description,
                style: TextStyle(color: palette.foreground, height: 1.35),
              ),
              if (status.valueText != null) ...[
                const SizedBox(height: 14),
                Text(
                  status.valueText!,
                  style: Theme.of(context).textTheme.titleLarge?.copyWith(
                    color: palette.foreground,
                    fontWeight: FontWeight.w700,
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

class _ActionCard extends StatelessWidget {
  const _ActionCard({required this.sectionId, required this.action});

  final String sectionId;
  final Revision3ProjectSectionActionCard action;

  @override
  Widget build(BuildContext context) {
    final scheme = Theme.of(context).colorScheme;
    final enabled = action.onPressed != null;
    return Semantics(
      key: Key('revision3-project-section-$sectionId-action-${action.id}'),
      container: true,
      button: true,
      enabled: enabled,
      label: action.title,
      hint: action.description,
      value: action.badge,
      onTap: action.onPressed,
      excludeSemantics: true,
      child: Opacity(
        opacity: enabled ? 1 : 0.58,
        child: Material(
          color: scheme.surfaceContainerLow,
          shape: RoundedRectangleBorder(
            borderRadius: BorderRadius.circular(16),
            side: BorderSide(color: scheme.outlineVariant),
          ),
          clipBehavior: Clip.antiAlias,
          child: InkWell(
            onTap: action.onPressed,
            canRequestFocus: enabled,
            child: Padding(
              padding: const EdgeInsets.all(18),
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Row(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      Icon(action.icon, color: scheme.primary, size: 26),
                      if (action.badge != null) ...[
                        const SizedBox(width: 12),
                        Flexible(
                          child: Align(
                            alignment: Alignment.centerRight,
                            child: DecoratedBox(
                              decoration: BoxDecoration(
                                color: scheme.secondaryContainer,
                                borderRadius: BorderRadius.circular(999),
                              ),
                              child: Padding(
                                padding: const EdgeInsets.symmetric(
                                  horizontal: 10,
                                  vertical: 5,
                                ),
                                child: Text(
                                  action.badge!,
                                  style: Theme.of(context).textTheme.labelMedium
                                      ?.copyWith(
                                        color: scheme.onSecondaryContainer,
                                        fontWeight: FontWeight.w600,
                                      ),
                                ),
                              ),
                            ),
                          ),
                        ),
                      ],
                    ],
                  ),
                  const SizedBox(height: 14),
                  Text(
                    action.title,
                    style: Theme.of(context).textTheme.titleMedium?.copyWith(
                      fontWeight: FontWeight.w600,
                    ),
                  ),
                  const SizedBox(height: 6),
                  Text(
                    action.description,
                    style: const TextStyle(height: 1.35),
                  ),
                ],
              ),
            ),
          ),
        ),
      ),
    );
  }
}

typedef _StatusPalette = ({Color background, Color foreground, Color accent});

_StatusPalette _statusPalette(
  ColorScheme scheme,
  Revision3ProjectSectionStatusSeverity severity,
) => switch (severity) {
  Revision3ProjectSectionStatusSeverity.neutral => (
    background: scheme.surfaceContainerHighest,
    foreground: scheme.onSurface,
    accent: scheme.onSurfaceVariant,
  ),
  Revision3ProjectSectionStatusSeverity.success => (
    background: scheme.primaryContainer,
    foreground: scheme.onPrimaryContainer,
    accent: scheme.primary,
  ),
  Revision3ProjectSectionStatusSeverity.warning => (
    background: scheme.tertiaryContainer,
    foreground: scheme.onTertiaryContainer,
    accent: scheme.tertiary,
  ),
  Revision3ProjectSectionStatusSeverity.blocked => (
    background: scheme.errorContainer,
    foreground: scheme.onErrorContainer,
    accent: scheme.error,
  ),
};

const _maximumContentWidth = 1280.0;
final RegExp _kebabSafeId = RegExp(r'^[a-z0-9]+(?:-[a-z0-9]+)*$');

bool _isKebabSafeId(String value) => _kebabSafeId.hasMatch(value);

String _requireKebabSafeId(String value, String parameterName) {
  if (!_isKebabSafeId(value)) {
    throw ArgumentError.value(
      value,
      parameterName,
      'must be a non-empty kebab-safe ID',
    );
  }
  return value;
}

bool _statusIdsAreUnique(List<Revision3ProjectSectionStatusCard> cards) {
  final ids = <String>{};
  return cards.every((card) => _isKebabSafeId(card.id) && ids.add(card.id));
}

bool _actionIdsAreUnique(List<Revision3ProjectSectionActionCard> cards) {
  final ids = <String>{};
  return cards.every((card) => _isKebabSafeId(card.id) && ids.add(card.id));
}

List<Revision3ProjectSectionStatusCard> _validatedStatusCards(
  List<Revision3ProjectSectionStatusCard> cards,
) {
  if (!_statusIdsAreUnique(cards)) {
    throw ArgumentError.value(
      cards,
      'statusCards',
      'IDs must be non-empty, kebab-safe, and unique',
    );
  }
  return List<Revision3ProjectSectionStatusCard>.unmodifiable(cards);
}

List<Revision3ProjectSectionActionCard> _validatedActionCards(
  List<Revision3ProjectSectionActionCard> cards,
) {
  if (!_actionIdsAreUnique(cards)) {
    throw ArgumentError.value(
      cards,
      'actionCards',
      'IDs must be non-empty, kebab-safe, and unique',
    );
  }
  return List<Revision3ProjectSectionActionCard>.unmodifiable(cards);
}
