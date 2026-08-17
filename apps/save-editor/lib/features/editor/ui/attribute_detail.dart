import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:goresave/features/app/domain/ui_settings.dart';
import 'package:goresave/features/editor/domain/actor.dart';
import 'package:goresave/features/editor/domain/editor_models.dart';
import 'package:goresave/features/editor/domain/npc_attributes.dart';
import 'package:goresave/features/editor/domain/pending_edits.dart';
import 'package:goresave/features/editor/ui/actor_detail_header.dart';
import 'package:goresave/features/editor/ui/hero_stats_card.dart';
import 'package:goresave/features/editor/ui/npc_attributes_panel.dart';
import 'package:goresave/features/editor/ui/skills_panel.dart';
import 'package:goresave/l10n/app_localizations.dart';
import 'package:goresave/loc/attribute_loc.dart';
import 'package:goresave/loc/loc_catalog_provider.dart';
import 'package:goresave/providers/data_providers.dart';

import '../domain/editor_notifier.dart';
import '../domain/hero_attributes.dart'
    show AttributeLabelResolver, TypedValueEdit, heroAttributeHidden;

/// Reverse a stored per-NPC attribute registry entry back into the panel's
/// [NpcTypedEdit] drafts so [NpcAttributesPanel] can resume from them on a
/// revisit. Inverse of the `private.typed.setValue` JSON the onPendingChanged
/// handler writes. Tolerant of unexpected shapes (skips what it can't parse).
List<NpcTypedEdit> _npcAttributeDraftsFromPending(PendingSaveEdit? pending) {
  if (pending == null) return const [];
  final drafts = <NpcTypedEdit>[];
  for (final edit in pending.edits) {
    if (edit['path'] != 'private.typed.setValue') continue;
    final value = edit['value'];
    if (value is! Map) continue;
    final path = value['path'];
    final raw = value['value'];
    if (path is! List) continue;
    final segments = path.whereType<String>().toList();
    if (segments.length != path.length) continue;
    final number = raw is num ? raw.toDouble() : null;
    if (number == null) continue;
    drafts.add(NpcTypedEdit(path: segments, value: number));
  }
  return drafts;
}

/// Reconstruct the player's queued attribute drafts from the 'heroStats' pending
/// entry so [HeroStatsCard] can resume from them when the user returns to the
/// Player after selecting an NPC. Mirror of [_npcAttributeDraftsFromPending]
/// for the player's [TypedValueEdit] type (same `private.typed.setValue` JSON).
List<TypedValueEdit> _heroDraftsFromPending(PendingSaveEdit? pending) {
  if (pending == null) return const [];
  final drafts = <TypedValueEdit>[];
  for (final edit in pending.edits) {
    if (edit['path'] != 'private.typed.setValue') continue;
    final value = edit['value'];
    if (value is! Map) continue;
    final path = value['path'];
    final raw = value['value'];
    if (path is! List) continue;
    final segments = path.whereType<String>().toList();
    if (segments.length != path.length) continue;
    final number = raw is num ? raw.toDouble() : null;
    if (number == null) continue;
    drafts.add(TypedValueEdit(path: segments, value: number));
  }
  return drafts;
}

/// The "Attribute" DETAIL body (everything to the right of the shared character
/// master list). When the Player is selected the existing player attribute view
/// ([_PrivatePanel]) is shown unchanged; when an NPC is selected its attributes
/// are loaded and rendered in a flat [NpcAttributesPanel]. The selection is
/// passed in via [actor] (the shared editor state) so it stays in sync with the
/// other character sub-tabs. Extracted verbatim from the old `_AttributePanel`.
class AttributeDetail extends ConsumerWidget {
  const AttributeDetail({
    super.key,
    required this.inspection,
    required this.notifier,
    required this.editable,
    required this.actor,
    this.showActorHeader = true,
  });

  final SaveInspection inspection;
  final EditorNotifier notifier;
  final bool editable;

  /// The selected actor (player or NPC). Orphans are guarded out by the caller,
  /// so this is only ever the player or a spawned NPC.
  final Actor actor;

  /// Standalone detail views may keep their own actor label. CharactersTab
  /// disables it because its persistent header now sits above the sub-tab bar.
  final bool showActorHeader;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final l10n = AppLocalizations.of(context);
    final state = ref.watch(editorProvider);
    final selected = actor;
    final lang = ref.watch(currentGameLangProvider);
    final locCatalog = ref.watch(locCatalogProvider).value ?? const {};
    final showObjectIds = ref.watch(showObjectIdsProvider);
    String attributeLabel(String id, String? setClass) =>
        localizedAttributeName(
          locCatalog,
          lang,
          id,
          setClass: setClass,
          l10n: l10n,
        );
    String attributeTooltipFor(String id, String? setClass) =>
        attributeTooltip(id, setClass: setClass, l10n: l10n);

    if (selected.isPlayer) {
      final body = _PrivatePanel(
        icon: Icons.person_outline,
        title: l10n.tabPlayer,
        isPlayer: true,
        inspection: inspection,
        notifier: notifier,
        editable: editable,
        lockedBody: l10n.playerLockedBody,
        attributeLabel: attributeLabel,
        attributeTooltip: attributeTooltipFor,
      );
      if (!showActorHeader) return body;
      // Player → a shared header ("Player", no GlobalId) above the EXISTING
      // player attribute view, unchanged below.
      return Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          ActorDetailHeader(
            actor: selected,
            locCatalog: locCatalog,
            lang: lang,
            showObjectIds: showObjectIds,
          ),
          Expanded(child: body),
        ],
      );
    }
    // NPC → flat attribute editor wired to a PER-NPC pending-edit key so it
    // never collides with the player's 'heroStats' contribution AND each NPC's
    // edits accumulate independently. A shared 'npc.attributes' key would let
    // a switch from NPC-A to NPC-B leave A's edit lingering under the same key
    // (the panel reload only clears its local field state, not the registry),
    // so A's edit would silently apply on the next Save while the UI shows B.
    // Keying by the NPC's GlobalId means A stays under 'npc.attributes:A' and
    // B under 'npc.attributes:B'; the save flow batches every key and the
    // distinct per-NPC GlobalId typed paths never conflict.
    final npcId = selected.id!;
    final pendingKey = 'npc.attributes:$npcId';
    // Status row: `Status: <lebend|tot>` + a Revive action, rendered as the
    // FIRST entry of the core ("Hauptwerte") group detail (NPC-only). Revive
    // drives a STANDALONE structural edit the core won't batch with peers, so
    // it REGISTERS a per-NPC pending edit (`npc.revive:$id`) — the global Save
    // button applies it (saveAllPending splits each splicing edit into its own
    // write_save). No file write happens on tap. The pending flag is read from
    // the registry so a queued revive reflects optimistically before Save; the
    // reloadKey matches the panel's so a Save's trailing refresh re-seeds the
    // status line/HP from disk.
    final revivePending = state.pendingEdits.containsKey('npc.revive:$npcId');
    final statusConfig = NpcStatusConfig(
      npcId: npcId,
      editable: editable,
      // Carry the dead state known from the sidebar selection as a fallback
      // for when the async summary reload fails / the id is missing.
      knownDead: selected.isDead,
      reloadKey: (inspection, npcId),
      // Use the cached FULL NPC list (shared across master-list selections,
      // one decompress) and let the row pick the exact-id match. A substring
      // query with the default 100-row page could miss this NPC when many ids
      // match, leaving the action disabled and isDead wrongly defaulting.
      load: () => notifier.loadAllNpcActors(),
      onRevive: () => notifier.setPendingNpcRevive(npcId),
      revivePending: revivePending,
    );
    final body = Padding(
      padding: const EdgeInsets.fromLTRB(20, 8, 20, 20),
      child: Card(
        child: Padding(
          padding: const EdgeInsets.all(16),
          child: NpcAttributesPanel(
            // Reload when the inspected save OR the selected NPC changes, so
            // the list re-seeds from this NPC's saved values.
            reloadKey: (inspection, npcId),
            load: () => notifier.loadNpcAttributes(npcId),
            editable: editable,
            // Status row lives at the top of the core ("Hauptwerte") group.
            status: statusConfig,
            skillsSection: SkillsSection(
              notifier: notifier,
              editable: editable,
              reloadKey: (inspection, npcId),
              actor: npcId,
              pendingKey: 'skills:${foldEditTargetPart(npcId)}',
              showRoster: false,
            ),
            attributeLabel: attributeLabel,
            attributeTooltip: attributeTooltipFor,
            initialPending: () => _npcAttributeDraftsFromPending(
              notifier.pendingEditFor(pendingKey),
            ),
            onPendingChanged: (edits, validationError) {
              if (validationError != null) {
                notifier.setNpcEditInvalid(pendingKey);
                return;
              }
              notifier.setNpcEditInvalid(null);
              if (edits.isEmpty) {
                notifier.clearPendingEdit(pendingKey);
              } else {
                notifier.setPendingEdit(
                  pendingKey,
                  PendingSaveEdit(
                    edits: [
                      for (final edit in edits)
                        {
                          'path': 'private.typed.setValue',
                          'value': {'path': edit.path, 'value': edit.value},
                        },
                    ],
                  ),
                );
              }
            },
          ),
        ),
      ),
    );
    if (!showActorHeader) return body;
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        ActorDetailHeader(
          actor: selected,
          locCatalog: locCatalog,
          lang: lang,
          showObjectIds: showObjectIds,
        ),
        Expanded(child: body),
      ],
    );
  }
}

class _PrivatePanel extends StatelessWidget {
  const _PrivatePanel({
    required this.icon,
    required this.title,
    required this.isPlayer,
    required this.inspection,
    required this.notifier,
    required this.editable,
    required this.lockedBody,
    required this.attributeLabel,
    required this.attributeTooltip,
  });

  final IconData icon;
  final String title;
  final bool isPlayer;
  final SaveInspection inspection;
  final EditorNotifier notifier;
  final bool editable;
  final String lockedBody;
  final AttributeLabelResolver attributeLabel;
  final AttributeLabelResolver attributeTooltip;

  /// The legacy attributes editor, flattened: the whole tab body sits inside
  /// ONE main card now, so the section renders bare (no inner Card).
  Widget _legacyAttributesSection() {
    return _PrivatePlayerAttributesEditor(
      player: inspection.privatePlayer,
      notifier: notifier,
      editable: editable,
      reloadKey: inspection,
      attributeLabel: attributeLabel,
      attributeTooltip: attributeTooltip,
    );
  }

  /// Shared sub-tab layout (see CharactersTab): outer 20/top 8 → one Card →
  /// inner 16 around the whole attribute editor.
  Widget _mainCard(Widget content) {
    return Padding(
      padding: const EdgeInsets.fromLTRB(20, 8, 20, 20),
      child: Card(
        child: Padding(padding: const EdgeInsets.all(16), child: content),
      ),
    );
  }

  @override
  Widget build(BuildContext context) {
    if (inspection.privateDecoded) {
      // Typed path: HeroStatsCard manages its own internal scroll for the
      // detail area and pins the sidebar. Give it the full pane via the main
      // card's Padding (not a ListView) so it has a finite height to work
      // with.
      if (isPlayer && inspection.privateTypedVerified) {
        return _mainCard(
          HeroStatsCard(
            // New SaveInspection instance after every write/refresh —
            // changing identity drops pending edits and reloads.
            reloadKey: inspection,
            load: notifier.loadHeroAttributes,
            // Resume from any unsaved player drafts after returning from an NPC.
            initialPending: () =>
                _heroDraftsFromPending(notifier.pendingEditFor('heroStats')),
            onPendingChanged: (edits, validationError) {
              if (edits.isEmpty || validationError != null) {
                notifier.clearPendingEdit('heroStats');
              } else {
                notifier.setPendingEdit(
                  'heroStats',
                  PendingSaveEdit(
                    edits: [
                      for (final edit in edits)
                        {
                          'path': 'private.typed.setValue',
                          'value': {'path': edit.path, 'value': edit.value},
                        },
                    ],
                  ),
                );
              }
            },
            editable: editable,
            // Spec: if the typed search errors out or finds nothing on a
            // typed-OK save, the heuristic editor stays available.
            fallback: inspection.privatePlayer.attributes.isNotEmpty
                ? _legacyAttributesSection()
                : null,
            // The hero's learned skills live in the "Talente" group, rendered
            // in the same row style and wired to the shared Save button via its
            // own 'skills' pending-registry entry.
            skillsSection: SkillsSection(
              notifier: notifier,
              editable: editable,
              reloadKey: inspection,
            ),
            attributeLabel: attributeLabel,
            attributeTooltip: attributeTooltip,
          ),
        );
      }
      // Legacy / non-typed path: stacked layout in a ListView inside the same
      // single main card. The player transform is NOT part of this list any
      // more — it lives in the Position sub-tab (PositionDetail), which renders
      // it regardless of privateTypedVerified so this path stays covered.
      return _mainCard(
        ListView(
          children: [
            if (isPlayer) ...[
              // Typed parse failed or not verified: stacked legacy layout —
              // no sidebar, no typed load call.
              if (inspection.privatePlayer.attributes.isNotEmpty) ...[
                _legacyAttributesSection(),
                const SizedBox(height: 16),
              ],
            ],
          ],
        ),
      );
    }
    return _MessagePane(icon: icon, title: title, body: lockedBody);
  }
}

class _PrivatePlayerAttributesEditor extends StatelessWidget {
  const _PrivatePlayerAttributesEditor({
    required this.player,
    required this.notifier,
    required this.attributeLabel,
    required this.attributeTooltip,
    this.editable = true,
    this.reloadKey,
  });

  final PrivatePlayerSummary player;
  final EditorNotifier notifier;
  final AttributeLabelResolver attributeLabel;
  final AttributeLabelResolver attributeTooltip;
  final bool editable;
  final Object? reloadKey;

  @override
  Widget build(BuildContext context) {
    final editable =
        this.editable &&
        player.writable.contains('private.player.setAttribute');
    final l10n = AppLocalizations.of(context);
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Row(
          children: [
            const Icon(Icons.monitor_heart_outlined),
            const SizedBox(width: 8),
            Text(
              l10n.heroAttributes,
              style: Theme.of(context).textTheme.titleSmall,
            ),
          ],
        ),
        const SizedBox(height: 10),
        LayoutBuilder(
          builder: (context, constraints) {
            final compact = constraints.maxWidth < 620;
            return Column(
              children: player.attributes
                  // The typed view hides the attributes the game re-derives
                  // from a learned skill; this fallback shows the same hero, so
                  // it must hide them too. The core's summary carries
                  // MagicianLevel, which would otherwise reappear here as a
                  // second, ineffective "Magic Circle" beside the skill's own.
                  .where((a) => !heroAttributeHidden(a.id))
                  .map(
                    (attribute) => _PrivatePlayerAttributeRow(
                      attribute: attribute,
                      label: attributeLabel(attribute.id, null),
                      tooltip: attributeTooltip(attribute.id, null),
                      notifier: notifier,
                      editable: editable,
                      compact: compact,
                      reloadKey: reloadKey,
                    ),
                  )
                  .toList(),
            );
          },
        ),
      ],
    );
  }
}

class _PrivatePlayerAttributeRow extends StatefulWidget {
  const _PrivatePlayerAttributeRow({
    required this.attribute,
    required this.label,
    this.tooltip = '',
    required this.notifier,
    required this.editable,
    required this.compact,
    this.reloadKey,
  });

  final PrivatePlayerAttribute attribute;
  final String label;

  /// One sentence on what this value does in the game. Empty = no tooltip.
  final String tooltip;
  final EditorNotifier notifier;
  final bool editable;
  final bool compact;
  // When provided, a change in identity triggers a field reseed even if the
  // attribute values haven't changed (e.g. after a Reset that reverts to the
  // same canonical value).
  final Object? reloadKey;

  @override
  State<_PrivatePlayerAttributeRow> createState() =>
      _PrivatePlayerAttributeRowState();
}

class _PrivatePlayerAttributeRowState
    extends State<_PrivatePlayerAttributeRow> {
  late final TextEditingController _baseController;
  late final TextEditingController _currentController;
  String? _lastId;
  double? _lastBase;
  double? _lastCurrent;
  Object? _lastReloadKey;
  String? _error;

  @override
  void initState() {
    super.initState();
    _baseController = TextEditingController();
    _currentController = TextEditingController();
    _sync();
  }

  @override
  void didUpdateWidget(covariant _PrivatePlayerAttributeRow oldWidget) {
    super.didUpdateWidget(oldWidget);
    _sync();
  }

  @override
  void dispose() {
    _baseController.dispose();
    _currentController.dispose();
    super.dispose();
  }

  void _sync() {
    final attribute = widget.attribute;
    final newKey = widget.reloadKey;
    final sameKey = newKey == null || identical(newKey, _lastReloadKey);
    if (!sameKey) {
      _lastReloadKey = newKey;
    }
    if (sameKey &&
        _lastId == attribute.id &&
        _lastBase == attribute.baseValue &&
        _lastCurrent == attribute.currentValue) {
      return;
    }
    _lastId = attribute.id;
    _lastBase = attribute.baseValue;
    _lastCurrent = attribute.currentValue;
    _baseController.text = _formatAttributeValue(attribute.baseValue);
    _currentController.text = _formatAttributeValue(attribute.currentValue);
    _error = null;
  }

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context);
    final name = widget.label;
    // Same affordance as the typed rows: the label explains what the value does.
    Widget named() {
      final text = Text(name, style: Theme.of(context).textTheme.labelLarge);
      return widget.tooltip.isEmpty
          ? text
          : Tooltip(message: widget.tooltip, child: text);
    }

    final baseField = TextField(
      key: ValueKey('legacy-attribute:${widget.attribute.id}:base'),
      controller: _baseController,
      enabled: widget.editable,
      keyboardType: const TextInputType.numberWithOptions(decimal: true),
      onChanged: (_) => _updatePending(),
      decoration: InputDecoration(
        labelText: l10n.attributeBaseValue,
        errorText: _error,
      ),
    );
    final currentField = TextField(
      key: ValueKey('legacy-attribute:${widget.attribute.id}:current'),
      controller: _currentController,
      enabled: widget.editable,
      keyboardType: const TextInputType.numberWithOptions(decimal: true),
      onChanged: (_) => _updatePending(),
      decoration: InputDecoration(labelText: l10n.attributeCurrentValue),
    );
    final label = SizedBox(width: 116, child: named());
    if (widget.compact) {
      return Padding(
        padding: const EdgeInsets.symmetric(vertical: 6),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            named(),
            const SizedBox(height: 6),
            baseField,
            const SizedBox(height: 6),
            currentField,
          ],
        ),
      );
    }
    return Padding(
      padding: const EdgeInsets.symmetric(vertical: 5),
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          label,
          Expanded(child: baseField),
          const SizedBox(width: 8),
          Expanded(child: currentField),
        ],
      ),
    );
  }

  void _updatePending() {
    if (!widget.editable) return;
    final id = widget.attribute.id;
    final baseValue = double.tryParse(_baseController.text.trim());
    final currentValue = double.tryParse(_currentController.text.trim());
    if (baseValue == null || currentValue == null) {
      final invalid = AppLocalizations.of(context).invalid;
      setState(() => _error = invalid);
      widget.notifier.clearPendingEdit('attr:$id');
      return;
    }
    setState(() => _error = null);
    final origBase = widget.attribute.baseValue;
    final origCurrent = widget.attribute.currentValue;
    if (baseValue == origBase && currentValue == origCurrent) {
      widget.notifier.clearPendingEdit('attr:$id');
      return;
    }
    widget.notifier.setPendingEdit(
      'attr:$id',
      PendingSaveEdit(
        edits: [
          {
            'path': 'private.player.setAttribute',
            'value': {
              'id': id,
              'baseValue': baseValue,
              'currentValue': currentValue,
            },
          },
        ],
      ),
    );
  }
}

String _formatAttributeValue(double? value) {
  if (value == null) return '';
  if (value == value.roundToDouble()) return value.toInt().toString();
  final rounded = value.toStringAsFixed(2).replaceFirst(RegExp(r'\.?0+$'), '');
  // These texts seed editable fields whose parsed value gets written back —
  // a lossy rounding (0.125 → 0.13) would silently corrupt untouched axes
  // the moment any sibling field changes. Round-trip or full precision.
  return double.tryParse(rounded) == value ? rounded : value.toString();
}

/// Centered icon + title + body message pane for empty/locked states. A private
/// copy of the same widget in `editor_page.dart` / `world_tab.dart`
/// (kept per-file so these detail widgets have no cross-file dependency).
class _MessagePane extends StatelessWidget {
  const _MessagePane({
    required this.icon,
    required this.title,
    required this.body,
  });

  final IconData icon;
  final String title;
  final String body;

  @override
  Widget build(BuildContext context) {
    return Center(
      child: ConstrainedBox(
        constraints: const BoxConstraints(maxWidth: 520),
        child: Card(
          child: Padding(
            padding: const EdgeInsets.all(24),
            child: Column(
              mainAxisSize: MainAxisSize.min,
              children: [
                Icon(
                  icon,
                  size: 48,
                  color: Theme.of(context).colorScheme.primary,
                ),
                const SizedBox(height: 12),
                Text(title, style: Theme.of(context).textTheme.titleLarge),
                const SizedBox(height: 8),
                Text(body, textAlign: TextAlign.center),
              ],
            ),
          ),
        ),
      ),
    );
  }
}
