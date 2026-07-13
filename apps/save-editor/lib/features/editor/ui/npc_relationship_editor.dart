import 'package:flutter/material.dart';
import 'package:goresave/features/editor/domain/editor_models.dart';
import 'package:goresave/features/editor/domain/editor_notifier.dart';
import 'package:goresave/features/editor/domain/npc_actors_page.dart';
import 'package:goresave/l10n/app_localizations.dart';

/// Edits the explicit NPC-to-Hero relationship value stored in the save.
///
/// This is intentionally hosted by the character Events tab: relationship
/// modifiers are part of the same long-lived world state as the event history,
/// not one of the NPC's numeric attributes. A missing stored value is shown as
/// game-computed rather than being mistaken for Neutral.
class NpcRelationshipEditor extends StatefulWidget {
  const NpcRelationshipEditor({
    super.key,
    required this.npcId,
    required this.notifier,
    required this.editable,
    required this.reloadKey,
  });

  final String npcId;
  final EditorNotifier notifier;
  final bool editable;
  final SaveInspection reloadKey;

  @override
  State<NpcRelationshipEditor> createState() => _NpcRelationshipEditorState();
}

class _NpcRelationshipEditorState extends State<NpcRelationshipEditor> {
  void Function()? _removeNotifierListener;
  NpcActor? _actor;
  String? _error;
  bool _loading = false;
  int _loadEpoch = 0;

  @override
  void initState() {
    super.initState();
    _listenToNotifier();
    _load();
  }

  @override
  void didUpdateWidget(covariant NpcRelationshipEditor oldWidget) {
    super.didUpdateWidget(oldWidget);
    final notifierChanged = !identical(widget.notifier, oldWidget.notifier);
    if (notifierChanged) {
      _removeNotifierListener?.call();
      _listenToNotifier();
    }
    if (notifierChanged ||
        widget.npcId != oldWidget.npcId ||
        widget.reloadKey != oldWidget.reloadKey) {
      _load();
    }
  }

  @override
  void dispose() {
    _removeNotifierListener?.call();
    super.dispose();
  }

  void _listenToNotifier() {
    _removeNotifierListener = widget.notifier.addListener((_) {
      if (mounted) setState(() {});
    }, fireImmediately: false);
  }

  Future<void> _load() async {
    final epoch = ++_loadEpoch;
    setState(() {
      _loading = true;
      _actor = null;
      _error = null;
    });
    try {
      final page = await widget.notifier.loadAllNpcActors();
      if (!mounted || epoch != _loadEpoch) return;
      final wanted = widget.npcId.toLowerCase();
      NpcActor? actor;
      for (final candidate in page.npcs) {
        if (candidate.id.toLowerCase() == wanted) {
          actor = candidate;
          break;
        }
      }
      setState(() {
        _loading = false;
        _actor = actor;
        _error = page.error;
      });
    } catch (error) {
      if (!mounted || epoch != _loadEpoch) return;
      setState(() {
        _loading = false;
        _actor = null;
        _error = error.toString();
      });
    }
  }

  void _setRelationship(NpcRelationship? relationship) {
    final saved = _actor?.personalRelationship;
    // Automatic is offered only when no stored override exists. Ignore a
    // synthetic null callback for an existing value so this control never
    // pretends it can delete an override that the writer cannot remove.
    if (relationship == null && saved != null) return;
    if (relationship == saved) {
      widget.notifier.clearPendingNpcRelationship(widget.npcId);
    } else if (relationship != null) {
      widget.notifier.setPendingNpcRelationship(widget.npcId, relationship);
    } else {
      widget.notifier.clearPendingNpcRelationship(widget.npcId);
    }
  }

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context);
    final theme = Theme.of(context);
    final scheme = theme.colorScheme;
    final actor = _actor;
    final saved = actor?.personalRelationship;
    final pending = widget.notifier.pendingNpcRelationship(widget.npcId);
    final selected = pending ?? saved;

    final String? hint = pending != null
        ? l10n.npcRelationshipPending(_relationshipLabel(l10n, pending))
        : actor == null
        ? null
        : saved == null
        ? l10n.npcRelationshipAutomaticHint
        : l10n.npcRelationshipStoredHint;

    final Widget control;
    if (_loading) {
      control = const Align(
        alignment: Alignment.centerLeft,
        child: SizedBox(
          width: 16,
          height: 16,
          child: CircularProgressIndicator(strokeWidth: 2),
        ),
      );
    } else if (actor == null) {
      final unavailable = _error?.trim().isNotEmpty == true
          ? _error!
          : l10n.npcRelationshipUnavailable;
      control = Text(
        pending == null
            ? unavailable
            : '$unavailable\n${l10n.npcRelationshipPending(_relationshipLabel(l10n, pending))}',
        style: theme.textTheme.bodySmall?.copyWith(color: scheme.error),
      );
    } else {
      control = KeyedSubtree(
        key: const Key('npc-relationship-dropdown'),
        child: DropdownButtonFormField<NpcRelationship>(
          key: ValueKey((saved, pending)),
          initialValue: selected,
          isExpanded: true,
          decoration: const InputDecoration(isDense: true),
          hint: Text(l10n.npcRelationshipAutomatic),
          items: [
            if (saved == null)
              DropdownMenuItem<NpcRelationship>(
                value: null,
                child: Text(l10n.npcRelationshipAutomatic),
              ),
            for (final relationship in NpcRelationship.values)
              DropdownMenuItem(
                value: relationship,
                child: Text(_relationshipLabel(l10n, relationship)),
              ),
          ],
          onChanged:
              widget.editable && widget.reloadKey.privateNpc.canSetRelationship
              ? _setRelationship
              : null,
        ),
      );
    }

    final label = Text(
      l10n.npcRelationshipRowLabel,
      style: theme.textTheme.labelLarge,
    );
    final hintWidget = hint == null
        ? const SizedBox.shrink()
        : Text(
            hint,
            style: theme.textTheme.bodySmall?.copyWith(
              color: pending != null ? scheme.primary : scheme.onSurfaceVariant,
            ),
          );

    return Padding(
      key: const Key('npc-relationship-editor'),
      padding: const EdgeInsets.symmetric(vertical: 5),
      child: LayoutBuilder(
        builder: (context, constraints) {
          if (constraints.maxWidth < 620) {
            return Column(
              crossAxisAlignment: CrossAxisAlignment.stretch,
              children: [
                label,
                const SizedBox(height: 6),
                control,
                if (hint != null) ...[const SizedBox(height: 6), hintWidget],
              ],
            );
          }
          return Row(
            crossAxisAlignment: CrossAxisAlignment.center,
            children: [
              SizedBox(width: 170, child: label),
              Expanded(child: control),
              const SizedBox(width: 8),
              Expanded(child: hintWidget),
            ],
          );
        },
      ),
    );
  }
}

String _relationshipLabel(
  AppLocalizations l10n,
  NpcRelationship relationship,
) => switch (relationship) {
  NpcRelationship.friend => l10n.npcRelationshipFriend,
  NpcRelationship.neutral => l10n.npcRelationshipNeutral,
  NpcRelationship.enemy => l10n.npcRelationshipEnemy,
};
