import 'dart:convert';

import 'package:flutter/material.dart';
import 'package:goresave/features/editor/domain/memory_event_presentation.dart';
import 'package:goresave/features/editor/domain/progression_models.dart';
import 'package:goresave/l10n/app_localizations.dart';
import 'package:goresave/ui/design/app_theme.dart';

/// Compact semantic event card. The collapsed row answers "what happened and
/// when?"; expanding it exposes every parsed fact and the raw gameplay tags.
/// Keeping the action buttons in the header preserves the fast multi-delete
/// workflow while making the rest of the row a generous details target.
class MemoryEventCard extends StatelessWidget {
  const MemoryEventCard({
    super.key,
    required this.event,
    required this.presentation,
    required this.editable,
    required this.showObjectIds,
    required this.pendingRemoval,
    this.onUndo,
    this.onRemove,
    this.onDuplicate,
  });

  final MemoryEvent event;
  final MemoryEventPresentation presentation;
  final bool editable;
  final bool showObjectIds;
  final bool pendingRemoval;
  final VoidCallback? onUndo;
  final VoidCallback? onRemove;
  final VoidCallback? onDuplicate;

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context);
    final theme = Theme.of(context);
    final scheme = theme.colorScheme;
    final accent = _categoryColor(presentation.category, scheme);
    final previewFacts = presentation.facts
        .where(
          (fact) =>
              fact.kind == MemoryEventFactKind.time ||
              fact.kind == MemoryEventFactKind.affected ||
              fact.kind == MemoryEventFactKind.instigator ||
              fact.kind == MemoryEventFactKind.amount,
        )
        .take(3)
        .toList(growable: false);
    final titleStyle = theme.textTheme.titleSmall?.copyWith(
      color: pendingRemoval ? scheme.onSurfaceVariant : null,
      decoration: pendingRemoval ? TextDecoration.lineThrough : null,
      fontWeight: FontWeight.w600,
    );

    return Card.outlined(
      margin: EdgeInsets.zero,
      color: pendingRemoval
          ? scheme.errorContainer.withValues(alpha: 0.2)
          : scheme.surfaceContainerLow,
      clipBehavior: Clip.antiAlias,
      child: Theme(
        data: theme.copyWith(dividerColor: Colors.transparent),
        child: ExpansionTile(
          key: ValueKey('memory-event-${event.index}'),
          tilePadding: const EdgeInsets.fromLTRB(12, 4, 8, 4),
          childrenPadding: const EdgeInsets.fromLTRB(16, 0, 16, 14),
          // An ExpansionTile centres what it reveals. Everything here is
          // read left to right — the headings, the facts, the coordinates,
          // the tags — so it starts at the left edge like the rest of the
          // editor, instead of each block floating in the middle of its row.
          expandedAlignment: AlignmentDirectional.centerStart,
          expandedCrossAxisAlignment: CrossAxisAlignment.start,
          // An ExpansionTile centres what it reveals. Everything here is
          // read left to right — the headings, the facts, the coordinates,
          // the tags — so it starts at the left edge like the rest of the
          // editor, instead of each block floating in the middle of its row.
          leading: Container(
            width: 40,
            height: 40,
            decoration: BoxDecoration(
              color: accent.withValues(alpha: 0.14),
              borderRadius: BorderRadius.circular(12),
            ),
            child: Icon(
              _categoryIcon(presentation.category),
              color: accent,
              size: 21,
            ),
          ),
          title: Row(
            crossAxisAlignment: CrossAxisAlignment.center,
            children: [
              Expanded(
                child: Text(
                  presentation.title,
                  maxLines: 2,
                  overflow: TextOverflow.ellipsis,
                  style: titleStyle,
                ),
              ),
              if (editable) ...[
                const SizedBox(width: 4),
                if (onUndo != null)
                  IconButton(
                    visualDensity: VisualDensity.compact,
                    icon: const Icon(Icons.undo, size: 20),
                    tooltip: l10n.cancel,
                    onPressed: onUndo,
                  )
                else ...[
                  IconButton(
                    visualDensity: VisualDensity.compact,
                    icon: const Icon(Icons.delete_outline, size: 20),
                    tooltip: l10n.removeEvent,
                    onPressed: onRemove,
                  ),
                  IconButton(
                    visualDensity: VisualDensity.compact,
                    icon: const Icon(Icons.copy_outlined, size: 20),
                    tooltip: l10n.duplicateEvent,
                    onPressed: onDuplicate,
                  ),
                ],
              ],
            ],
          ),
          subtitle: Padding(
            padding: const EdgeInsets.only(top: 5),
            child: Wrap(
              spacing: 6,
              runSpacing: 5,
              crossAxisAlignment: WrapCrossAlignment.center,
              children: [
                _Pill(
                  label: presentation.categoryLabel,
                  icon: _categoryIcon(presentation.category),
                  color: accent,
                ),
                for (final fact in previewFacts)
                  _Pill(
                    label: fact.value,
                    icon: _factIcon(fact.kind),
                    color: scheme.onSurfaceVariant,
                  ),
              ],
            ),
          ),
          children: [
            Divider(height: 16, color: scheme.outlineVariant),
            _SectionLabel(label: l10n.memoryEventDetails),
            const SizedBox(height: 8),
            LayoutBuilder(
              builder: (context, constraints) {
                final twoColumns = constraints.maxWidth >= 620;
                final width = twoColumns
                    ? (constraints.maxWidth - 12) / 2
                    : constraints.maxWidth;
                return Wrap(
                  spacing: 12,
                  runSpacing: 8,
                  children: [
                    for (final fact in presentation.facts)
                      SizedBox(
                        width: width,
                        child: _FactRow(fact: fact),
                      ),
                  ],
                );
              },
            ),
            if (event.position case final position?) ...[
              const SizedBox(height: 14),
              _SectionLabel(label: l10n.memoryEventPosition),
              const SizedBox(height: 6),
              SelectableText(
                'X ${_compactNumber(position.x)}   '
                'Y ${_compactNumber(position.y)}   '
                'Z ${_compactNumber(position.z)}',
                style: theme.textTheme.bodyMedium,
              ),
            ],
            if (event.payload case final payload?) ...[
              const SizedBox(height: 14),
              _SectionLabel(
                label: payload.type == null
                    ? l10n.memoryEventPayload
                    : '${l10n.memoryEventPayload} · ${payload.type}',
              ),
              const SizedBox(height: 6),
              for (final field in payload.fields)
                _PayloadFieldRow(field: field),
              if (payload.truncated)
                Padding(
                  padding: const EdgeInsets.only(top: 4),
                  child: Text(
                    '…',
                    style: theme.textTheme.titleMedium?.copyWith(
                      color: scheme.onSurfaceVariant,
                    ),
                  ),
                ),
            ],
            if (presentation.tags.isNotEmpty) ...[
              const SizedBox(height: 14),
              _SectionLabel(label: l10n.memoryEventTags),
              const SizedBox(height: 6),
              Wrap(
                spacing: 6,
                runSpacing: 6,
                children: [
                  for (final tag in presentation.tags)
                    Container(
                      padding: const EdgeInsets.symmetric(
                        horizontal: 8,
                        vertical: 4,
                      ),
                      decoration: BoxDecoration(
                        color: scheme.surfaceContainerHighest,
                        borderRadius: BorderRadius.circular(8),
                      ),
                      child: SelectableText(
                        tag,
                        style: theme.textTheme.bodySmall,
                      ),
                    ),
                ],
              ),
            ],
            if (showObjectIds) ...[
              const SizedBox(height: 14),
              _SectionLabel(label: l10n.memoryEventTechnicalData),
              const SizedBox(height: 6),
              _TechnicalRow(
                label: l10n.memoryEventIndex,
                value: event.index.toString(),
              ),
              if (presentation.subjectId case final subjectId?)
                _TechnicalRow(label: l10n.memoryEventSubject, value: subjectId),
              for (final fact in presentation.facts)
                if (fact.technicalValue case final technicalValue?)
                  _TechnicalRow(label: fact.label, value: technicalValue),
            ],
          ],
        ),
      ),
    );
  }
}

class _Pill extends StatelessWidget {
  const _Pill({required this.label, required this.icon, required this.color});

  final String label;
  final IconData icon;
  final Color color;

  @override
  Widget build(BuildContext context) => Container(
    padding: const EdgeInsets.symmetric(horizontal: 7, vertical: 3),
    decoration: BoxDecoration(
      color: color.withValues(alpha: 0.09),
      borderRadius: BorderRadius.circular(999),
      border: Border.all(color: color.withValues(alpha: 0.22)),
    ),
    child: Row(
      mainAxisSize: MainAxisSize.min,
      children: [
        Icon(icon, size: 13, color: color),
        const SizedBox(width: 4),
        Text(
          label,
          style: Theme.of(context).textTheme.labelSmall?.copyWith(color: color),
        ),
      ],
    ),
  );
}

class _FactRow extends StatelessWidget {
  const _FactRow({required this.fact});

  final MemoryEventFact fact;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final scheme = theme.colorScheme;
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 8),
      decoration: BoxDecoration(
        color: scheme.surfaceContainerHighest.withValues(alpha: 0.55),
        borderRadius: BorderRadius.circular(9),
      ),
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Icon(_factIcon(fact.kind), size: 17, color: scheme.onSurfaceVariant),
          const SizedBox(width: 8),
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(
                  fact.label,
                  style: theme.textTheme.labelSmall?.copyWith(
                    color: scheme.onSurfaceVariant,
                  ),
                ),
                const SizedBox(height: 1),
                SelectableText(fact.value, style: theme.textTheme.bodyMedium),
              ],
            ),
          ),
        ],
      ),
    );
  }
}

class _SectionLabel extends StatelessWidget {
  const _SectionLabel({required this.label});

  final String label;

  @override
  Widget build(BuildContext context) => Text(
    label,
    style: Theme.of(context).textTheme.labelLarge?.copyWith(
      color: Theme.of(context).colorScheme.onSurfaceVariant,
    ),
  );
}

class _PayloadFieldRow extends StatelessWidget {
  const _PayloadFieldRow({required this.field});

  final MemoryEventPayloadField field;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Padding(
      padding: const EdgeInsets.only(bottom: 6),
      child: Container(
        width: double.infinity,
        padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 8),
        decoration: BoxDecoration(
          color: theme.colorScheme.surfaceContainerHighest.withValues(
            alpha: 0.45,
          ),
          borderRadius: BorderRadius.circular(9),
        ),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Row(
              children: [
                Expanded(
                  child: Text(field.name, style: theme.textTheme.labelMedium),
                ),
                if (field.type.isNotEmpty)
                  Text(
                    field.type,
                    style: theme.textTheme.labelSmall?.copyWith(
                      color: theme.colorScheme.onSurfaceVariant,
                    ),
                  ),
              ],
            ),
            const SizedBox(height: 3),
            SelectableText(
              _payloadValue(field.value),
              style: theme.textTheme.bodySmall?.copyWith(
                fontFamily: field.value is Map || field.value is List
                    ? uiAwareMonospaceFontFamily(context, fallback: 'monospace')
                    : null,
              ),
            ),
          ],
        ),
      ),
    );
  }
}

class _TechnicalRow extends StatelessWidget {
  const _TechnicalRow({required this.label, required this.value});

  final String label;
  final String value;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Padding(
      padding: const EdgeInsets.only(bottom: 4),
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          SizedBox(
            width: 120,
            child: Text(
              label,
              style: theme.textTheme.bodySmall?.copyWith(
                color: theme.colorScheme.onSurfaceVariant,
              ),
            ),
          ),
          Expanded(
            child: SelectableText(value, style: theme.textTheme.bodySmall),
          ),
        ],
      ),
    );
  }
}

String _compactNumber(double value) {
  if (value == value.roundToDouble()) return value.toInt().toString();
  final fixed = value.toStringAsFixed(3);
  return fixed
      .replaceFirst(RegExp(r'0+$'), '')
      .replaceFirst(RegExp(r'\.$'), '');
}

String _payloadValue(Object? value) {
  if (value == null) return 'null';
  if (value is double) return _compactNumber(value);
  if (value is Map || value is List) {
    try {
      return const JsonEncoder.withIndent('  ').convert(value);
    } catch (_) {
      return value.toString();
    }
  }
  return value.toString();
}

IconData _categoryIcon(MemoryEventCategory category) => switch (category) {
  MemoryEventCategory.quest => Icons.assignment_turned_in_outlined,
  MemoryEventCategory.document => Icons.menu_book_outlined,
  MemoryEventCategory.story => Icons.auto_stories_outlined,
  MemoryEventCategory.exploration => Icons.explore_outlined,
  MemoryEventCategory.combat => Icons.sports_martial_arts_outlined,
  MemoryEventCategory.social => Icons.handshake_outlined,
  MemoryEventCategory.item => Icons.inventory_2_outlined,
  MemoryEventCategory.learning => Icons.school_outlined,
  MemoryEventCategory.guild => Icons.groups_outlined,
  MemoryEventCategory.crime => Icons.gavel_outlined,
  MemoryEventCategory.rest => Icons.bedtime_outlined,
  MemoryEventCategory.other => Icons.bubble_chart_outlined,
};

IconData _factIcon(MemoryEventFactKind kind) => switch (kind) {
  MemoryEventFactKind.time => Icons.schedule_outlined,
  MemoryEventFactKind.duration => Icons.timelapse_outlined,
  MemoryEventFactKind.chapter => Icons.bookmark_outline,
  MemoryEventFactKind.instigator => Icons.person_pin_outlined,
  MemoryEventFactKind.affected => Icons.person_outline,
  MemoryEventFactKind.amount => Icons.numbers_outlined,
  MemoryEventFactKind.primaryObject => Icons.link_outlined,
  MemoryEventFactKind.secondaryObject => Icons.link_outlined,
  MemoryEventFactKind.segmentText => Icons.article_outlined,
};

Color _categoryColor(MemoryEventCategory category, ColorScheme scheme) =>
    switch (category) {
      MemoryEventCategory.quest => scheme.primary,
      MemoryEventCategory.document => scheme.tertiary,
      MemoryEventCategory.story => scheme.secondary,
      MemoryEventCategory.exploration => Colors.teal,
      MemoryEventCategory.combat => scheme.error,
      MemoryEventCategory.social => Colors.cyan.shade800,
      MemoryEventCategory.item => Colors.amber.shade800,
      MemoryEventCategory.learning => Colors.indigo,
      MemoryEventCategory.guild => Colors.deepPurple,
      MemoryEventCategory.crime => Colors.orange.shade800,
      MemoryEventCategory.rest => Colors.blueGrey,
      MemoryEventCategory.other => scheme.onSurfaceVariant,
    };
