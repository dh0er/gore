import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:goresave/features/editor/domain/actor.dart';
import 'package:goresave/features/editor/domain/editor_models.dart';
import 'package:goresave/features/editor/domain/npc_position.dart';
import 'package:goresave/features/editor/ui/hero_stats_card.dart'
    show formatHeroValue;
import 'package:goresave/features/editor/ui/transform_editor.dart';
import 'package:goresave/l10n/app_localizations.dart';
import 'package:goresave/providers/data_providers.dart';

import '../domain/editor_notifier.dart';

/// The "Position" DETAIL body (fifth Charaktere sub-tab). Actor-aware exactly
/// like [AttributeDetail]: the player gets the editable
/// [PlayerTransformEditor] (its ONLY home now — see hero_stats_card.dart), an
/// NPC gets the READ-ONLY [NpcPositionPanel].
///
/// The asymmetry is not a gap waiting to be filled — see [NpcPositionPanel] for
/// the evidence that an NPC's position cannot be changed through the savegame
/// at all.
class PositionDetail extends ConsumerWidget {
  const PositionDetail({
    super.key,
    required this.inspection,
    required this.notifier,
    required this.editable,
    required this.actor,
  });

  final SaveInspection inspection;
  final EditorNotifier notifier;
  final bool editable;

  /// The selected actor (player or NPC). Orphans are guarded out by the caller.
  final Actor actor;

  /// Shared sub-tab layout (see CharactersTab): outer 20/top 8 → one Card →
  /// inner 16 around the content.
  Widget _mainCard(Widget content) {
    return Padding(
      padding: const EdgeInsets.fromLTRB(20, 8, 20, 20),
      child: Card(
        child: Padding(padding: const EdgeInsets.all(16), child: content),
      ),
    );
  }

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final l10n = AppLocalizations.of(context);
    // Rebuild when the editor state changes so a refreshed inspection reaches
    // both branches.
    ref.watch(editorProvider);

    if (actor.isPlayer) {
      final transform = inspection.privatePlayer.transform;
      if (transform == null) {
        // Either the private payload is locked or the save carries no player
        // transform: nothing to edit, and no fields that read as editable.
        return _mainCard(
          Text(
            inspection.privateDecoded
                ? l10n.positionNotReadable
                : l10n.playerLockedBody,
          ),
        );
      }
      // Rendered whenever a transform exists — deliberately NOT gated on
      // privateTypedVerified, so the legacy (non-typed) path keeps its editor.
      // The PLAYER's saved position IS applied on load; only the NPC records
      // are discarded (see [NpcPositionPanel]).
      return _mainCard(
        PlayerTransformEditor(
          transform: transform,
          editable:
              editable &&
              inspection.privatePlayer.writable.contains(
                'private.player.setTransform',
              ),
          notifier: notifier,
          reloadKey: inspection,
        ),
      );
    }

    final npcId = actor.id!;
    return _mainCard(
      NpcPositionPanel(
        // Reload when the inspected save OR the selected NPC changes.
        reloadKey: (inspection, npcId),
        load: () => notifier.loadNpcPosition(npcId),
      ),
    );
  }
}

/// One NPC's saved pose, shown READ-ONLY: the character location/rotation and
/// the spawn location/rotation, as plain text.
///
/// **Why there is nothing to edit here.** The per-NPC pose in the save is a
/// snapshot written at save time and DISCARDED on load — placement authority is
/// the level's WorldPointActor named in the NPC's GlobalId. A UE4SS runtime
/// probe settled it: after loading a byte-verified save in which
/// `CharacterLocation`, `SpawnLocation` and `DailyRoutineClass` had all been
/// rewritten for two NPCs, every live value read back was the ORIGINAL pre-edit
/// one. One of the two NPCs was streamed out at the time and the other was not,
/// so "the snap only fires while simulated" does not explain it either. NPC
/// *attributes* live in the same `{CharacterStates}` blob and do apply, so the
/// blob is read — these particular records are simply never used.
///
/// Hence: no controllers, no location picker, no pending edit, no validation.
/// Data still arrives through [load] (the core `private.npc.position` command),
/// which stays useful for reading, the CLI and diagnostics.
class NpcPositionPanel extends StatefulWidget {
  const NpcPositionPanel({
    super.key,
    required this.load,
    required this.reloadKey,
  });

  final Future<NpcPoseResult> Function() load;
  final Object reloadKey;

  @override
  State<NpcPositionPanel> createState() => _NpcPositionPanelState();
}

class _NpcPositionPanelState extends State<NpcPositionPanel> {
  NpcPose? _pose;
  String? _error;
  bool _loadFailed = false;
  bool _loading = false;
  // Epoch counter used to discard results from superseded reload calls.
  int _reloadEpoch = 0;

  @override
  void initState() {
    super.initState();
    _reload();
  }

  @override
  void didUpdateWidget(covariant NpcPositionPanel oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (widget.reloadKey != oldWidget.reloadKey) _reload();
  }

  Future<void> _reload() async {
    final epoch = ++_reloadEpoch;
    setState(() => _loading = true);
    final result = await widget.load();
    if (!mounted || epoch != _reloadEpoch) return;
    setState(() {
      _loading = false;
      _error = result.error;
      _loadFailed = result.error != null || result.pose == null;
      _pose = result.pose;
    });
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final l10n = AppLocalizations.of(context);

    if (_loading) {
      return const Padding(
        padding: EdgeInsets.all(24),
        child: Center(child: CircularProgressIndicator()),
      );
    }

    final pose = _pose;
    if (_loadFailed || pose == null) {
      // A load ERROR is shown in the error colour; a successful load with no
      // pose (no entry for this NPC) is just a fact, not a failure.
      final message = _error;
      return Padding(
        padding: const EdgeInsets.all(20),
        child: Text(
          message ?? l10n.positionNotReadable,
          style: message != null
              ? TextStyle(color: theme.colorScheme.error)
              : TextStyle(color: theme.colorScheme.onSurfaceVariant),
        ),
      );
    }

    final current = <String>[
      ..._locationValues(l10n, pose.location),
      ..._rotationValues(l10n, pose.rotation),
    ];
    final spawn = <String>[
      ..._locationValues(l10n, pose.spawnLocation),
      ..._rotationValues(l10n, pose.spawnRotation),
    ];

    return SingleChildScrollView(
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          _sectionHeader(theme, Icons.place_outlined, l10n.heroTransform),
          const SizedBox(height: 8),
          // The one thing this panel exists to say.
          Row(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Icon(
                Icons.info_outline,
                size: 18,
                color: theme.colorScheme.onSurfaceVariant,
              ),
              const SizedBox(width: 6),
              Expanded(
                child: Text(
                  l10n.npcPositionReadOnly,
                  style: TextStyle(color: theme.colorScheme.onSurfaceVariant),
                ),
              ),
            ],
          ),
          if (current.isEmpty && spawn.isEmpty) ...[
            const SizedBox(height: 12),
            Text(
              l10n.positionNotReadable,
              style: TextStyle(color: theme.colorScheme.onSurfaceVariant),
            ),
          ] else ...[
            const SizedBox(height: 12),
            _values(theme, current),
            const SizedBox(height: 20),
            _sectionHeader(theme, Icons.flag_outlined, l10n.spawnPositionSection),
            const SizedBox(height: 8),
            _values(theme, spawn),
          ],
        ],
      ),
    );
  }

  List<String> _locationValues(AppLocalizations l10n, Vec3? location) => [
    if (location != null) ...[
      '${l10n.locationX}: ${formatHeroValue(location.x)}',
      '${l10n.locationY}: ${formatHeroValue(location.y)}',
      '${l10n.locationZ}: ${formatHeroValue(location.z)}',
    ],
  ];

  List<String> _rotationValues(AppLocalizations l10n, Rot3? rotation) => [
    if (rotation != null) ...[
      '${l10n.rotationPitch}: ${formatHeroValue(rotation.pitch)}',
      '${l10n.rotationYaw}: ${formatHeroValue(rotation.yaw)}',
      '${l10n.rotationRoll}: ${formatHeroValue(rotation.roll)}',
    ],
  ];

  Widget _sectionHeader(ThemeData theme, IconData icon, String title) {
    return Row(
      children: [
        Icon(icon),
        const SizedBox(width: 8),
        Expanded(child: Text(title, style: theme.textTheme.titleSmall)),
      ],
    );
  }

  /// A pose group as plain text — the style the spawn reference always used,
  /// deliberately NOT disabled input fields, which read as "temporarily locked".
  Widget _values(ThemeData theme, List<String> values) {
    if (values.isEmpty) return const SizedBox.shrink();
    return Align(
      alignment: Alignment.centerLeft,
      child: Wrap(
        spacing: 20,
        runSpacing: 6,
        children: [
          for (final value in values)
            Text(
              value,
              style: theme.textTheme.bodyMedium?.copyWith(
                color: theme.colorScheme.onSurfaceVariant,
              ),
            ),
        ],
      ),
    );
  }
}
