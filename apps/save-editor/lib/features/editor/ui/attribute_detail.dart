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
    show AttributeLabelResolver, TypedValueEdit;

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
  });

  final SaveInspection inspection;
  final EditorNotifier notifier;
  final bool editable;

  /// The selected actor (player or NPC). Orphans are guarded out by the caller,
  /// so this is only ever the player or a spawned NPC.
  final Actor actor;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final l10n = AppLocalizations.of(context);
    final state = ref.watch(editorProvider);
    final selected = actor;
    final lang = ref.watch(currentGameLangProvider);
    final locCatalog = ref.watch(locCatalogProvider).value ?? const {};
    final showObjectIds = ref.watch(showObjectIdsProvider);
    String attributeLabel(String id, String? setClass) =>
        localizedAttributeName(locCatalog, lang, id, setClass: setClass);

    if (selected.isPlayer) {
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
          Expanded(
            child: _PrivatePanel(
              icon: Icons.person_outline,
              title: l10n.tabPlayer,
              isPlayer: true,
              inspection: inspection,
              notifier: notifier,
              editable: editable,
              lockedBody: l10n.playerLockedBody,
              attributeLabel: attributeLabel,
            ),
          ),
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
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        ActorDetailHeader(
          actor: selected,
          locCatalog: locCatalog,
          lang: lang,
          showObjectIds: showObjectIds,
        ),
        Expanded(
          // Shared sub-tab layout (see CharactersTab): outer 20/top 8 →
          // one Card → inner 16 around the whole attribute editor.
          child: Padding(
            padding: const EdgeInsets.fromLTRB(20, 8, 20, 20),
            child: Card(
              child: Padding(
                padding: const EdgeInsets.all(16),
                child: NpcAttributesPanel(
                  // Reload when the inspected save OR the selected NPC
                  // changes, so the list re-seeds from this NPC's saved
                  // values.
                  reloadKey: (inspection, npcId),
                  load: () => notifier.loadNpcAttributes(npcId),
                  editable: editable,
                  // Status row lives at the top of the core ("Hauptwerte") group.
                  status: statusConfig,
                  // This NPC's learned skills, in a "Talente" group. Own
                  // per-NPC pending key so they never collide with the hero's
                  // or another NPC's skill edits; roster hidden (only the NPC's
                  // actual skills matter).
                  skillsSection: SkillsSection(
                    notifier: notifier,
                    editable: editable,
                    reloadKey: (inspection, npcId),
                    actor: npcId,
                    pendingKey: 'skills:$npcId',
                    showRoster: false,
                  ),
                  attributeLabel: attributeLabel,
                  // Resume from this NPC's queued attribute drafts on revisit:
                  // reverse the stored typed.setValue edits back into the panel's
                  // NpcTypedEdit drafts. Without this, returning to a previously
                  // edited NPC and editing another attribute would replace the
                  // stored entry with only the newly-dirty field, dropping the rest.
                  initialPending: () => _npcAttributeDraftsFromPending(
                    notifier.pendingEditFor(pendingKey),
                  ),
                  onPendingChanged: (edits, validationError) {
                    if (validationError != null) {
                      // Transient invalid/empty input in one field must NOT discard
                      // the NPC's already-stored valid drafts (switching actors
                      // disposes this panel and loses its local _pending). Keep the
                      // stored drafts but BLOCK global Save while invalid, so the
                      // now-stale stored value is never written behind the bad field.
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
                                'value': {
                                  'path': edit.path,
                                  'value': edit.value,
                                },
                              },
                          ],
                        ),
                      );
                    }
                  },
                ),
              ),
            ),
          ),
        ),
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
  });

  final IconData icon;
  final String title;
  final bool isPlayer;
  final SaveInspection inspection;
  final EditorNotifier notifier;
  final bool editable;
  final String lockedBody;
  final AttributeLabelResolver attributeLabel;

  /// The legacy attributes editor, flattened: the whole tab body sits inside
  /// ONE main card now, so the section renders bare (no inner Card).
  Widget _legacyAttributesSection() {
    return _PrivatePlayerAttributesEditor(
      player: inspection.privatePlayer,
      notifier: notifier,
      editable: editable,
      reloadKey: inspection,
      attributeLabel: attributeLabel,
    );
  }

  /// The transform editor, flattened (see [_legacyAttributesSection]).
  Widget? _transformSection() {
    if (inspection.privatePlayer.transform == null) return null;
    return _PrivatePlayerTransformEditor(
      transform: inspection.privatePlayer.transform!,
      editable:
          editable &&
          inspection.privatePlayer.writable.contains(
            'private.player.setTransform',
          ),
      notifier: notifier,
      reloadKey: inspection,
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
            transformCard: _transformSection(),
            // The hero's learned skills live in the "Talente" group, rendered
            // in the same row style and wired to the shared Save button via its
            // own 'skills' pending-registry entry.
            skillsSection: SkillsSection(
              notifier: notifier,
              editable: editable,
              reloadKey: inspection,
            ),
            attributeLabel: attributeLabel,
          ),
        );
      }
      // Legacy / non-typed path: stacked layout in a ListView inside the same
      // single main card.
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
              if (inspection.privatePlayer.transform != null) ...[
                _transformSection()!,
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

class _PrivatePlayerTransformEditor extends StatefulWidget {
  const _PrivatePlayerTransformEditor({
    required this.transform,
    required this.editable,
    required this.notifier,
    this.reloadKey,
  });

  final PrivatePlayerTransform transform;
  final bool editable;
  final EditorNotifier notifier;
  // When provided, a change in identity triggers a field reseed even if the
  // transform values haven't changed (e.g. Reset followed by re-inspect that
  // returns the same values).
  final Object? reloadKey;

  @override
  State<_PrivatePlayerTransformEditor> createState() =>
      _PrivatePlayerTransformEditorState();
}

class _PrivatePlayerTransformEditorState
    extends State<_PrivatePlayerTransformEditor> {
  late final TextEditingController _locationXController;
  late final TextEditingController _locationYController;
  late final TextEditingController _locationZController;
  late final TextEditingController _rotationPitchController;
  late final TextEditingController _rotationYawController;
  late final TextEditingController _rotationRollController;
  PrivatePlayerTransform? _lastTransform;
  // Track the inspection (widget parent) identity so that a Reset/refresh that
  // produces a new inspection instance triggers a reseed even when the
  // transform values themselves haven't changed.
  Object? _inspectionIdentity;
  String? _error;

  @override
  void initState() {
    super.initState();
    _locationXController = TextEditingController();
    _locationYController = TextEditingController();
    _locationZController = TextEditingController();
    _rotationPitchController = TextEditingController();
    _rotationYawController = TextEditingController();
    _rotationRollController = TextEditingController();
    _sync();
  }

  @override
  void didUpdateWidget(covariant _PrivatePlayerTransformEditor oldWidget) {
    super.didUpdateWidget(oldWidget);
    _sync();
  }

  @override
  void dispose() {
    _locationXController.dispose();
    _locationYController.dispose();
    _locationZController.dispose();
    _rotationPitchController.dispose();
    _rotationYawController.dispose();
    _rotationRollController.dispose();
    super.dispose();
  }

  void _sync() {
    final transform = widget.transform;
    final last = _lastTransform;
    // Re-seed on reloadKey identity change (e.g. after Reset/refresh that
    // produces a new SaveInspection) even when the transform values themselves
    // are unchanged, so the fields visually revert after a Reset.
    final newKey = widget.reloadKey;
    final sameKey = newKey == null || identical(newKey, _inspectionIdentity);
    if (!sameKey) {
      _inspectionIdentity = newKey;
    }
    if (sameKey &&
        last != null &&
        last.location.x == transform.location.x &&
        last.location.y == transform.location.y &&
        last.location.z == transform.location.z &&
        last.rotation.pitch == transform.rotation.pitch &&
        last.rotation.yaw == transform.rotation.yaw &&
        last.rotation.roll == transform.rotation.roll) {
      return;
    }
    _lastTransform = transform;
    _locationXController.text = _formatAttributeValue(transform.location.x);
    _locationYController.text = _formatAttributeValue(transform.location.y);
    _locationZController.text = _formatAttributeValue(transform.location.z);
    _rotationPitchController.text = _formatAttributeValue(
      transform.rotation.pitch,
    );
    _rotationYawController.text = _formatAttributeValue(transform.rotation.yaw);
    _rotationRollController.text = _formatAttributeValue(
      transform.rotation.roll,
    );
    _error = null;
  }

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context);
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Row(
          children: [
            const Icon(Icons.explore_outlined),
            const SizedBox(width: 8),
            Expanded(
              child: Text(
                l10n.heroTransform,
                style: Theme.of(context).textTheme.titleSmall,
              ),
            ),
          ],
        ),
        if (_error != null) ...[
          const SizedBox(height: 6),
          Text(
            _error!,
            style: TextStyle(color: Theme.of(context).colorScheme.error),
          ),
        ],
        const SizedBox(height: 10),
        LayoutBuilder(
          builder: (context, constraints) {
            final compact = constraints.maxWidth < 700;
            final fields = [
              _TransformNumberField(
                controller: _locationXController,
                label: l10n.locationX,
                enabled: widget.editable,
                onChanged: (_) => _updatePending(),
              ),
              _TransformNumberField(
                controller: _locationYController,
                label: l10n.locationY,
                enabled: widget.editable,
                onChanged: (_) => _updatePending(),
              ),
              _TransformNumberField(
                controller: _locationZController,
                label: l10n.locationZ,
                enabled: widget.editable,
                onChanged: (_) => _updatePending(),
              ),
              _TransformNumberField(
                controller: _rotationPitchController,
                label: l10n.rotationPitch,
                enabled: widget.editable,
                onChanged: (_) => _updatePending(),
              ),
              _TransformNumberField(
                controller: _rotationYawController,
                label: l10n.rotationYaw,
                enabled: widget.editable,
                onChanged: (_) => _updatePending(),
              ),
              _TransformNumberField(
                controller: _rotationRollController,
                label: l10n.rotationRoll,
                enabled: widget.editable,
                onChanged: (_) => _updatePending(),
              ),
            ];
            if (compact) {
              return Column(
                children: [
                  for (final field in fields) ...[
                    field,
                    if (field != fields.last) const SizedBox(height: 8),
                  ],
                ],
              );
            }
            return Column(
              children: [
                Row(
                  children: [
                    for (final field in fields.take(3)) ...[
                      Expanded(child: field),
                      if (field != fields[2]) const SizedBox(width: 8),
                    ],
                  ],
                ),
                const SizedBox(height: 8),
                Row(
                  children: [
                    for (final field in fields.skip(3)) ...[
                      Expanded(child: field),
                      if (field != fields.last) const SizedBox(width: 8),
                    ],
                  ],
                ),
              ],
            );
          },
        ),
      ],
    );
  }

  void _updatePending() {
    if (!widget.editable) return;
    final locationX = double.tryParse(_locationXController.text.trim());
    final locationY = double.tryParse(_locationYController.text.trim());
    final locationZ = double.tryParse(_locationZController.text.trim());
    final rotationPitch = double.tryParse(_rotationPitchController.text.trim());
    final rotationYaw = double.tryParse(_rotationYawController.text.trim());
    final rotationRoll = double.tryParse(_rotationRollController.text.trim());
    if (locationX == null ||
        locationY == null ||
        locationZ == null ||
        rotationPitch == null ||
        rotationYaw == null ||
        rotationRoll == null) {
      final invalid = AppLocalizations.of(context).invalid;
      setState(() => _error = invalid);
      widget.notifier.clearPendingEdit('transform');
      return;
    }
    setState(() => _error = null);
    final orig = widget.transform;
    if (locationX == orig.location.x &&
        locationY == orig.location.y &&
        locationZ == orig.location.z &&
        rotationPitch == orig.rotation.pitch &&
        rotationYaw == orig.rotation.yaw &&
        rotationRoll == orig.rotation.roll) {
      widget.notifier.clearPendingEdit('transform');
      return;
    }
    widget.notifier.setPendingEdit(
      'transform',
      PendingSaveEdit(
        edits: [
          {
            'path': 'private.player.setTransform',
            'value': {
              'location': {'x': locationX, 'y': locationY, 'z': locationZ},
              'rotation': {
                'pitch': rotationPitch,
                'yaw': rotationYaw,
                'roll': rotationRoll,
              },
            },
          },
        ],
      ),
    );
  }
}

class _TransformNumberField extends StatelessWidget {
  const _TransformNumberField({
    required this.controller,
    required this.label,
    required this.enabled,
    this.onChanged,
  });

  final TextEditingController controller;
  final String label;
  final bool enabled;
  final ValueChanged<String>? onChanged;

  @override
  Widget build(BuildContext context) {
    return TextField(
      controller: controller,
      enabled: enabled,
      onChanged: onChanged,
      keyboardType: const TextInputType.numberWithOptions(
        decimal: true,
        signed: true,
      ),
      decoration: InputDecoration(labelText: label),
    );
  }
}

class _PrivatePlayerAttributesEditor extends StatelessWidget {
  const _PrivatePlayerAttributesEditor({
    required this.player,
    required this.notifier,
    required this.attributeLabel,
    this.editable = true,
    this.reloadKey,
  });

  final PrivatePlayerSummary player;
  final EditorNotifier notifier;
  final AttributeLabelResolver attributeLabel;
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
                  .map(
                    (attribute) => _PrivatePlayerAttributeRow(
                      attribute: attribute,
                      label: attributeLabel(attribute.id, null),
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
    required this.notifier,
    required this.editable,
    required this.compact,
    this.reloadKey,
  });

  final PrivatePlayerAttribute attribute;
  final String label;
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
    final label = SizedBox(
      width: 116,
      child: Text(name, style: Theme.of(context).textTheme.labelLarge),
    );
    if (widget.compact) {
      return Padding(
        padding: const EdgeInsets.symmetric(vertical: 6),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            Text(name, style: Theme.of(context).textTheme.labelLarge),
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
