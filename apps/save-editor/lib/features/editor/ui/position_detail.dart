import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:goresave/features/editor/domain/actor.dart';
import 'package:goresave/features/editor/domain/editor_models.dart';
import 'package:goresave/features/editor/domain/npc_position.dart';
import 'package:goresave/features/editor/domain/pending_edits.dart';
import 'package:goresave/features/editor/ui/hero_stats_card.dart'
    show formatHeroValue;
import 'package:goresave/features/editor/ui/location_picker_dialog.dart';
import 'package:goresave/features/editor/ui/transform_editor.dart';
import 'package:goresave/l10n/app_localizations.dart';
import 'package:goresave/providers/data_providers.dart';

import '../domain/editor_notifier.dart';

/// Largest coordinate the editor accepts, matching the core's
/// `validate_private_f64` guard. The typed write path itself only checks
/// `is_finite`, so this client-side range IS the guard against a fat-fingered
/// `1e30` silently teleporting an NPC out of the world.
const double kPositionLimit = 10000000;

/// Reverse a stored per-NPC position registry entry back into the panel's
/// [NpcStructEdit] drafts so [NpcPositionPanel] can resume from them on a
/// revisit. The Map-valued sibling of `_npcAttributeDraftsFromPending` in
/// attribute_detail.dart — inverse of the `private.typed.setValue` JSON the
/// onPendingChanged handler writes. Tolerant of unexpected shapes (skips what
/// it can't parse).
List<NpcStructEdit> npcPositionDraftsFromPending(PendingSaveEdit? pending) {
  if (pending == null) return const [];
  final drafts = <NpcStructEdit>[];
  for (final edit in pending.edits) {
    if (edit['path'] != 'private.typed.setValue') continue;
    final value = edit['value'];
    if (value is! Map) continue;
    final path = value['path'];
    final raw = value['value'];
    if (path is! List) continue;
    final segments = path.whereType<String>().toList();
    if (segments.length != path.length) continue;
    // Only the STRUCT-valued edits belong to this panel; a scalar value is an
    // attribute draft that happens to share the command name.
    if (raw is! Map) continue;
    final members = <String, double>{};
    var ok = true;
    for (final entry in raw.entries) {
      final key = entry.key;
      final number = entry.value;
      if (key is! String || number is! num || !number.toDouble().isFinite) {
        ok = false;
        break;
      }
      members[key] = number.toDouble();
    }
    if (!ok || members.isEmpty) continue;
    drafts.add(NpcStructEdit(path: segments, value: members));
  }
  return drafts;
}

/// The routine class queued for this NPC, or null when none is.
///
/// The sibling of [npcPositionDraftsFromPending] for the one edit that is NOT a
/// struct: without it a revisit would read the checkbox off the SAVE while a
/// routine swap sat queued, and the next keystroke would rebuild the entry
/// without it — saving the move as position-only, which is exactly the move that
/// does not stick.
String? npcPositionRoutineFromPending(PendingSaveEdit? pending) {
  if (pending == null) return null;
  for (final edit in pending.edits) {
    if (edit['path'] != 'private.typed.setValue') continue;
    final value = edit['value'];
    if (value is! Map) continue;
    final path = value['path'];
    if (path is! List || path.isEmpty) continue;
    if (path.last != 'DailyRoutineClass') continue;
    final queued = value['value'];
    if (queued is String && queued.isNotEmpty) return queued;
  }
  return null;
}

/// The "Position" DETAIL body (fifth Charaktere sub-tab). Actor-aware exactly
/// like [AttributeDetail]: the player gets the existing
/// [PlayerTransformEditor] (its ONLY home now — see hero_stats_card.dart), an
/// NPC gets the [NpcPositionPanel] wired to a PER-NPC pending key.
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
    // Rebuild when the pending registry changes so a revisit rehydrates from
    // the current entry.
    ref.watch(editorProvider);

    if (actor.isPlayer) {
      final transform = inspection.privatePlayer.transform;
      if (transform == null) {
        // Either the private payload is locked or the save carries no player
        // transform: nothing to edit, and no fields that read as editable.
        return _mainCard(
          Text(
            inspection.privateDecoded
                ? l10n.positionNotEditable
                : l10n.playerLockedBody,
          ),
        );
      }
      // Rendered whenever a transform exists — deliberately NOT gated on
      // privateTypedVerified, so the legacy (non-typed) path keeps its editor.
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

    // NPC → per-NPC pending key. A SHARED key would let a switch from NPC-A to
    // NPC-B apply A's edit while the UI shows B (the bug documented at
    // attribute_detail.dart's NPC branch); the save flow batches every key and
    // the distinct per-NPC GlobalId typed paths never conflict.
    final npcId = actor.id!;
    final pendingKey = 'npc.position:$npcId';
    return _mainCard(
      NpcPositionPanel(
        npcId: npcId,
        // Reload when the inspected save OR the selected NPC changes.
        reloadKey: (inspection, npcId),
        load: () => notifier.loadNpcPosition(npcId),
        editable: editable,
        initialPending: () =>
            npcPositionDraftsFromPending(notifier.pendingEditFor(pendingKey)),
        initialPendingRoutine: () =>
            npcPositionRoutineFromPending(notifier.pendingEditFor(pendingKey)),
        onPendingChanged: (draft, validationError) {
          if (validationError != null) {
            // Blocks the global Save button while a field is unusable — unlike
            // the player editor, which silently drops its pending edit.
            //
            // Deliberately `setEditInvalid` and not `setNpcEditInvalid`: the
            // latter clears `invalidNpcEditKey` plus every `npc.attributes:`
            // key on every call, so this panel going valid would unblock Save
            // while the Attribute sub-tab still holds a bad field (both are
            // `_KeepAliveTab`s, so both stay live). Keying ourselves keeps the
            // two sub-tabs' validation independent.
            notifier.setEditInvalid(pendingKey, invalid: true);
            return;
          }
          notifier.setEditInvalid(pendingKey, invalid: false);
          if (draft.isEmpty) {
            notifier.clearPendingEdit(pendingKey);
          } else {
            notifier.setPendingEdit(
              pendingKey,
              PendingSaveEdit(
                edits: [
                  for (final edit in draft.edits)
                    {
                      'path': 'private.typed.setValue',
                      'value': {'path': edit.path, 'value': edit.value},
                    },
                  // The routine class rides the same key and the same command:
                  // moving an NPC and letting him stay is one action, so it must
                  // be one Save, never a position write that lands without it.
                  if (draft.routineClass != null &&
                      draft.routineClassPath.isNotEmpty)
                    {
                      'path': 'private.typed.setValue',
                      'value': {
                        'path': draft.routineClassPath,
                        'value': draft.routineClass,
                      },
                    },
                ],
                placementNotes: [if (draft.note != null) draft.note!],
                clearPlacementNotes: [if (draft.clearNote) npcId],
              ),
            );
          }
        },
      ),
    );
  }
}

/// Editable pose for a single NPC: the six current-position fields, the saved
/// spawn pose as a read-only reference, and a "reset to spawn" action.
///
/// Data arrives through [load] (the core `private.npc.position` command) and
/// pending edits leave through [onPendingChanged] — the same load + pending
/// contract [NpcAttributesPanel] uses, except each edit carries a STRUCT value
/// (a whole triplet) instead of a scalar.
///
/// **Moving an NPC works, and it takes TWO writes — verified in-game.**
/// `CharacterLocation` alone places him there on load and his daily routine
/// walks him back within seconds. Add
/// `DailyRoutineByGlobalId{id} › DailyRoutineClass =
/// /Script/Angelscript.DailyRoutine_Empty` and he stays. Proven by a controlled
/// pair in one save: the NPC with both writes stayed on his freepoint, the one
/// with only the position left, as in every earlier run. The cost is that he
/// loses his schedule for good.
///
/// Never write `DailyRoutine_Empty_StayAtSpawn` for this: its
/// `Activate_Implementation` calls
/// `SetCharacterLocationAndDirection(GetPreferredLocation().StartingPosition)` —
/// the level actor, never the save. Reaching for it is what made this panel
/// read-only for a week on a conclusion that was exactly backwards.
class NpcPositionPanel extends StatefulWidget {
  const NpcPositionPanel({
    super.key,
    required this.npcId,
    required this.load,
    required this.onPendingChanged,
    required this.editable,
    required this.reloadKey,
    this.initialPending,
    this.initialPendingRoutine,
  });

  final Future<NpcPoseResult> Function() load;

  /// Drafts already queued for this NPC under the parent's per-NPC pending key,
  /// supplied so a revisit RESUMES from them. Evaluated on each (re)load so it
  /// reflects the current registry.
  final List<NpcStructEdit> Function()? initialPending;

  /// The routine class already queued for this NPC, if any. Read on each
  /// (re)load so the checkbox resumes from the QUEUE rather than from the save:
  /// they differ exactly while a pin is pending, which is when getting it wrong
  /// silently turns a "stay there" move into a position-only one.
  final String? Function()? initialPendingRoutine;

  /// Called whenever the set of pending edits changes. [edits] holds one entry
  /// per differing group (location and/or rotation). [validationError] is
  /// non-null when any field is empty, unparseable or out of range — the
  /// panel's edits are then suppressed AND the global Save button is blocked.
  final void Function(NpcPlacementDraft draft, String? validationError)
  onPendingChanged;

  /// The NPC's GlobalId, carried so a queued undo note can name its subject.
  final String npcId;
  final bool editable;
  final Object reloadKey;

  @override
  State<NpcPositionPanel> createState() => _NpcPositionPanelState();
}

class _NpcPositionPanelState extends State<NpcPositionPanel> {
  NpcPose? _pose;
  /// The whole load result, kept for its routine half. [_pose] stays as its own
  /// field because every coordinate path already reads it.
  NpcPoseResult? _result;
  /// Whether the move should also pin the NPC by putting him on the inert
  /// routine. On by default: a move that the routine undoes within seconds is
  /// not what anyone means by moving an NPC.
  bool _stayInPlace = true;
  /// Set by "take the move back" and cleared by any other change. While set, the
  /// routine write targets the RECORDED original class instead of the inert one.
  bool _restoring = false;
  String? _error;
  bool _loadFailed = false;
  bool _loading = false;
  // Epoch counter used to discard results from superseded reload calls.
  int _reloadEpoch = 0;

  // Rehydrated field texts keyed by `<typed path>|<member>`, exactly as
  // _NpcAttributesPanelState keys its drafts by the joined typed path. Cleared
  // on reload, then re-seeded from the parent's stored entry in the same
  // setState.
  final Map<String, String> _pending = {};

  late final TextEditingController _x;
  late final TextEditingController _y;
  late final TextEditingController _z;
  late final TextEditingController _pitch;
  late final TextEditingController _yaw;
  late final TextEditingController _roll;

  @override
  void initState() {
    super.initState();
    _x = TextEditingController();
    _y = TextEditingController();
    _z = TextEditingController();
    _pitch = TextEditingController();
    _yaw = TextEditingController();
    _roll = TextEditingController();
    _reload();
  }

  @override
  void didUpdateWidget(covariant NpcPositionPanel oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (widget.reloadKey != oldWidget.reloadKey) _reload();
  }

  @override
  void dispose() {
    _x.dispose();
    _y.dispose();
    _z.dispose();
    _pitch.dispose();
    _yaw.dispose();
    _roll.dispose();
    super.dispose();
  }

  static String _draftKey(List<String> path, String member) =>
      '${path.join(' ')}|$member';

  Future<void> _reload() async {
    final epoch = ++_reloadEpoch;
    setState(() {
      _loading = true;
      _pending.clear();
      // Rehydrate this NPC's queued drafts so a revisit resumes from them
      // instead of starting from disk values (which the next edit would
      // otherwise write back, dropping the earlier group).
      for (final edit in widget.initialPending?.call() ?? const []) {
        if (edit.path.isEmpty) continue;
        for (final member in edit.value.entries) {
          _pending[_draftKey(edit.path, member.key)] = formatHeroValue(
            member.value,
          );
        }
      }
    });
    // Do NOT call onPendingChanged here. Calling it from
    // initState/didUpdateWidget would mutate the provider during build and
    // throw with flutter_riverpod — the same constraint NpcAttributesPanel
    // documents. Not notifying is correct because the parent keys pending
    // edits PER-NPC: the previous NPC's registry entry must stay intact.
    final result = await widget.load();
    if (!mounted || epoch != _reloadEpoch) return;
    setState(() {
      _loading = false;
      _error = result.error;
      _loadFailed = result.error != null || result.pose == null;
      _pose = result.pose;
      _result = result;
      // The box shows the NPC's CURRENT state, so it starts ticked only for one
      // already on the inert routine. Off for everyone else: wiping a schedule
      // is a deliberate act, not something a position edit does on the side.
      //
      // A QUEUED routine swap wins over the stored state — that is the whole
      // point of a revisit resuming from the registry — and it decides the box
      // in both directions: the inert class means a pending pin, any other class
      // means a pending restore.
      final queued = widget.initialPendingRoutine?.call();
      _stayInPlace = queued != null
          ? queued == result.inertRoutineClass
          : result.isPinned;
      _restoring = false;
      _seedControllers();
    });
  }

  /// Seed the six fields from the loaded pose, preferring any rehydrated draft.
  void _seedControllers() {
    final pose = _pose;
    if (pose == null) return;
    String seed(List<String> path, String member, double? value) =>
        _pending[_draftKey(path, member)] ?? formatHeroValue(value);
    final location = pose.location;
    _x.text = seed(pose.locationPath, 'x', location?.x);
    _y.text = seed(pose.locationPath, 'y', location?.y);
    _z.text = seed(pose.locationPath, 'z', location?.z);
    final rotation = pose.rotation;
    _pitch.text = seed(pose.rotationPath, 'pitch', rotation?.pitch);
    _yaw.text = seed(pose.rotationPath, 'yaw', rotation?.yaw);
    _roll.text = seed(pose.rotationPath, 'roll', rotation?.roll);
  }

  bool get _canEditLocation {
    final pose = _pose;
    return pose != null && pose.location != null && pose.locationWritable;
  }

  bool get _canEditRotation {
    final pose = _pose;
    return pose != null && pose.rotation != null && pose.rotationWritable;
  }

  /// Recompute the pending edits from the six fields and notify the parent.
  /// One [NpcStructEdit] per group that differs from the loaded pose.
  ///
  /// On ANY validation failure the parent is notified with an error and empty
  /// edits: it then blocks the global Save button (unlike the player editor,
  /// which silently drops its pending edit).
  void _recompute() {
    final pose = _pose;
    if (pose == null) return;
    final l10n = AppLocalizations.of(context);
    String? error;
    final edits = <NpcStructEdit>[];

    double? parse(TextEditingController controller, String label) {
      if (error != null) return null;
      final text = controller.text.trim();
      // A cleared field is almost certainly an accident.
      if (text.isEmpty) {
        error = l10n.attributeEmpty(label);
        return null;
      }
      final value = double.tryParse(text);
      if (value == null) {
        error = l10n.attributeInvalidNumber(label, text);
        return null;
      }
      // The typed write path only checks is_finite, so this range IS the guard.
      if (!value.isFinite || value.abs() > kPositionLimit) {
        error = l10n.positionOutOfRange;
        return null;
      }
      return value;
    }

    if (_canEditLocation) {
      final x = parse(_x, l10n.locationX);
      final y = parse(_y, l10n.locationY);
      final z = parse(_z, l10n.locationZ);
      if (x != null && y != null && z != null) {
        final next = Vec3(x: x, y: y, z: z);
        if (next != pose.location) {
          edits.add(
            NpcStructEdit(path: pose.locationPath, value: next.toValue()),
          );
        }
      }
    }
    if (_canEditRotation) {
      final pitch = parse(_pitch, l10n.rotationPitch);
      final yaw = parse(_yaw, l10n.rotationYaw);
      final roll = parse(_roll, l10n.rotationRoll);
      if (pitch != null && yaw != null && roll != null) {
        final next = Rot3(pitch: pitch, yaw: yaw, roll: roll);
        if (next != pose.rotation) {
          edits.add(
            NpcStructEdit(path: pose.rotationPath, value: next.toValue()),
          );
        }
      }
    }

    // Rebuild regardless: the "reset to spawn" button's enabled state is
    // derived from the live field values.
    setState(() => _error = error);
    if (error != null) {
      widget.onPendingChanged(const NpcPlacementDraft(), error);
      return;
    }
    widget.onPendingChanged(_draft(edits), null);
  }

  /// Any change the user makes by hand abandons a queued restore: the fields no
  /// longer hold what the note said to put back, so continuing to send the
  /// routine restore with them would pair one NPC's old routine with a position
  /// nobody recorded.
  void _userEdited() {
    if (_restoring) setState(() => _restoring = false);
    _recompute();
  }

  /// Fold the coordinate edits together with the routine write and its undo
  /// note into the one draft the parent queues.
  NpcPlacementDraft _draft(List<NpcStructEdit> edits) {
    final result = _result;
    final pose = _pose;
    if (result == null || pose == null || !result.canPin) {
      return NpcPlacementDraft(edits: edits);
    }
    // The box mirrors the NPC's stored state, so only a DIFFERENCE is a write.
    if (_stayInPlace == result.isPinned) {
      return NpcPlacementDraft(edits: edits);
    }
    if (!_stayInPlace) {
      // Unticked: give the routine back. The class comes from the note, because
      // it is story state and nothing in the save still holds it. The position
      // is left alone unless the user also asked for that (the undo button,
      // which seeds the fields before this runs).
      final original = result.undo?.originalRoutineClass;
      if (original == null) return NpcPlacementDraft(edits: edits);
      return NpcPlacementDraft(
        edits: edits,
        routineClassPath: result.routineClassPath,
        routineClass: original,
        clearNote: true,
      );
    }
    final location = _currentLocation();
    final before = pose.location;
    if (location == null || before == null) {
      return NpcPlacementDraft(edits: edits);
    }
    // The facing joins the note only when the move changes it — the picker can
    // apply a spot's heading. Restoring the position while leaving the new
    // facing would strand it, because clearing the note takes the only record of
    // the old one with it.
    final rotation = _currentRotation();
    final rotationBefore = pose.rotation;
    final movesRotation =
        rotation != null && rotationBefore != null && rotation != rotationBefore;
    return NpcPlacementDraft(
      edits: edits,
      routineClassPath: result.routineClassPath,
      routineClass: result.inertRoutineClass,
      note: {
        'npc': widget.npcId,
        'note': {
          'original_location': [before.x, before.y, before.z],
          if (movesRotation)
            'original_rotation': [
              rotationBefore.pitch,
              rotationBefore.yaw,
              rotationBefore.roll,
            ],
          if (result.routineClass != null)
            'original_routine_class': result.routineClass,
          'written_location': [location.x, location.y, location.z],
          if (movesRotation)
            'written_rotation': [rotation.pitch, rotation.yaw, rotation.roll],
          'written_routine_class': result.inertRoutineClass,
        },
      },
    );
  }

  /// Put the fields back to what the recorded move replaced and queue the
  /// routine restore with them. Pure UI, like [_resetToSpawn]: it seeds the
  /// controllers and lets [_recompute] decide what to queue.
  void _undoPlacement() {
    final undo = _result?.undo;
    if (undo == null || !undo.restorable) return;
    // Taking the move back is the position AND the routine, so it unticks the
    // box too — from here the ordinary "box differs from stored state" path
    // produces the routine write.
    setState(() {
      _restoring = true;
      _stayInPlace = false;
    });
    if (_canEditLocation) {
      _x.text = formatHeroValue(undo.originalLocation.x);
      _y.text = formatHeroValue(undo.originalLocation.y);
      _z.text = formatHeroValue(undo.originalLocation.z);
    }
    final rotation = undo.originalRotation;
    if (rotation != null && _canEditRotation) {
      _pitch.text = formatHeroValue(rotation.pitch);
      _yaw.text = formatHeroValue(rotation.yaw);
      _roll.text = formatHeroValue(rotation.roll);
    }
    _recompute();
  }

  /// Live location as typed, or null when a field is unusable.
  Vec3? _currentLocation() {
    final x = double.tryParse(_x.text.trim());
    final y = double.tryParse(_y.text.trim());
    final z = double.tryParse(_z.text.trim());
    if (x == null || y == null || z == null) return null;
    return Vec3(x: x, y: y, z: z);
  }

  Rot3? _currentRotation() {
    final pitch = double.tryParse(_pitch.text.trim());
    final yaw = double.tryParse(_yaw.text.trim());
    final roll = double.tryParse(_roll.text.trim());
    if (pitch == null || yaw == null || roll == null) return null;
    return Rot3(pitch: pitch, yaw: yaw, roll: roll);
  }

  /// "Reset to spawn" is a pure-UI convenience: it fills the editable fields
  /// and lets the normal recompute decide what (if anything) to queue. Disabled
  /// when there is no spawn location, when the spawn is the origin (the NPC was
  /// never placed, so the reference is meaningless), or when the fields already
  /// hold the spawn pose.
  bool get _resetEnabled {
    final pose = _pose;
    if (pose == null || !widget.editable) return false;
    if (!_canEditLocation && !_canEditRotation) return false;
    final spawnLocation = pose.spawnLocation;
    if (spawnLocation == null || spawnLocation.isZero) return false;
    final locationMatches = _currentLocation() == spawnLocation;
    final spawnRotation = pose.spawnRotation;
    final rotationMatches =
        spawnRotation == null || _currentRotation() == spawnRotation;
    return !(locationMatches && rotationMatches);
  }

  /// Open the shared location picker and fill the fields from the chosen spot.
  /// Like [_resetToSpawn] this is pure UI: it only seeds controllers and lets
  /// the normal [_recompute] decide what (if anything) to queue.
  ///
  /// Rotation is applied ONLY when the user opted in, and then as
  /// pitch 0 / yaw / roll 0 — the catalog carries no pitch or roll, and a
  /// spot's pitch would visibly tilt a standing pawn.
  Future<void> _pickLocation() async {
    final pick = await showLocationPickerDialog(context);
    if (pick == null || !mounted) return;
    final spot = pick.spot;
    if (_canEditLocation) {
      _x.text = formatHeroValue(spot.x);
      _y.text = formatHeroValue(spot.y);
      _z.text = formatHeroValue(spot.z);
    }
    if (pick.applyRotation && _canEditRotation) {
      _pitch.text = formatHeroValue(0);
      _yaw.text = formatHeroValue(spot.yaw);
      _roll.text = formatHeroValue(0);
    }
    _userEdited();
  }

  void _resetToSpawn() {
    final pose = _pose;
    if (pose == null) return;
    final spawnLocation = pose.spawnLocation;
    if (spawnLocation != null && _canEditLocation) {
      _x.text = formatHeroValue(spawnLocation.x);
      _y.text = formatHeroValue(spawnLocation.y);
      _z.text = formatHeroValue(spawnLocation.z);
    }
    final spawnRotation = pose.spawnRotation;
    if (spawnRotation != null && _canEditRotation) {
      _pitch.text = formatHeroValue(spawnRotation.pitch);
      _yaw.text = formatHeroValue(spawnRotation.yaw);
      _roll.text = formatHeroValue(spawnRotation.roll);
    }
    _userEdited();
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
          message ?? l10n.positionNotEditable,
          style: message != null
              ? TextStyle(color: theme.colorScheme.error)
              : TextStyle(color: theme.colorScheme.onSurfaceVariant),
        ),
      );
    }

    final anyEditable = _canEditLocation || _canEditRotation;

    return SingleChildScrollView(
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          Row(
            children: [
              const Icon(Icons.place_outlined),
              const SizedBox(width: 8),
              Expanded(
                child: Text(
                  l10n.heroTransform,
                  style: theme.textTheme.titleSmall,
                ),
              ),
            ],
          ),
          if (_error != null) ...[
            const SizedBox(height: 6),
            Text(_error!, style: TextStyle(color: theme.colorScheme.error)),
          ],
          if (!anyEditable) ...[
            const SizedBox(height: 6),
            Text(
              l10n.positionNotEditable,
              style: TextStyle(color: theme.colorScheme.onSurfaceVariant),
            ),
          ],
          if (pose.neverPlaced) ...[
            const SizedBox(height: 6),
            Row(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Icon(
                  Icons.warning_amber_outlined,
                  size: 18,
                  color: theme.colorScheme.tertiary,
                ),
                const SizedBox(width: 6),
                Expanded(
                  child: Text(
                    l10n.positionNeverPlaced,
                    style: TextStyle(color: theme.colorScheme.tertiary),
                  ),
                ),
              ],
            ),
          ],
          // The location picker sits directly above the fields: it writes the
          // chosen spot into the same six controllers the manual fields drive
          // and then calls _recompute(), exactly like _resetToSpawn does — one
          // pending path, never a second write route.
          if (_canEditLocation) ...[
            const SizedBox(height: 12),
            Align(
              alignment: Alignment.centerLeft,
              child: OutlinedButton.icon(
                icon: const Icon(Icons.travel_explore, size: 18),
                label: Text(l10n.pickLocation),
                onPressed: widget.editable ? _pickLocation : null,
              ),
            ),
          ],
          const SizedBox(height: 10),
          _fieldGrid(l10n),
          _stayInPlaceSection(theme, l10n),
          _undoSection(theme, l10n),
          const SizedBox(height: 20),
          _spawnSection(theme, l10n, pose),
        ],
      ),
    );
  }

  /// "…and let him stay there", below the fields rather than inside the
  /// location picker.
  ///
  /// The picker is deliberately write-agnostic — it serves the hero too, and
  /// carries no command, path or pending key. This option is neither the hero's
  /// nor the spot's: the spot's own checkbox ("apply its facing") is a property
  /// of the SPOT, while this one is a property of the NPC and has to hold just
  /// as much when coordinates are typed by hand.
  ///
  /// **A state, not a modifier on the move.** The box shows whether this NPC
  /// currently HAS a daily routine, so it is always visible and independent of
  /// any pending position edit: ticking it alone freezes him where he stands,
  /// and ticking it with a move is what makes the move stick.
  ///
  /// Off by default. A move that the routine undoes within seconds is not what
  /// anyone means by moving an NPC — but wiping a schedule is a deliberate act,
  /// and a box that arrives pre-ticked would do it as a side effect of editing a
  /// coordinate.
  ///
  /// Unticking gives the routine back, which needs the recorded note: the class
  /// is story state (`..._Collapsed`, `..._WaitYard`) and is not derivable from
  /// anything in the save. Without a usable note the box is locked with the
  /// reason shown, rather than offering an undo that would have to guess.
  Widget _stayInPlaceSection(ThemeData theme, AppLocalizations l10n) {
    final result = _result;
    if (result == null || !result.canPin) return const SizedBox.shrink();
    final canUntick = result.undo?.canRestoreRoutine ?? false;
    // Gated on the STORED state, never on the tick the user just made. Ticking
    // queues an edit; unticking drops it again and writes nothing, so a fresh
    // tick is always reversible and locking it would be a lie told before the
    // save even happened. Only an NPC whose save ALREADY says "no routine", with
    // no note saying which one it was, has no way back.
    final locked = result.isPinned && !canUntick;
    return Padding(
      padding: const EdgeInsets.only(top: 6),
      child: CheckboxListTile(
        key: const ValueKey('npc-position:stay'),
        contentPadding: EdgeInsets.zero,
        controlAffinity: ListTileControlAffinity.leading,
        dense: true,
        value: _stayInPlace,
        onChanged: !widget.editable || locked || _restoring
            ? null
            : (value) {
                setState(() => _stayInPlace = value ?? false);
                _recompute();
              },
        title: Text(l10n.npcStayInPlace),
        subtitle: Text(
          locked ? l10n.npcStayInPlaceLocked : l10n.npcStayInPlaceHint,
          style: TextStyle(color: theme.colorScheme.onSurfaceVariant),
        ),
      ),
    );
  }

  /// Take a recorded move back: the position AND the routine class it replaced.
  ///
  /// Shown only while the core still vouches for the note — once the save no
  /// longer holds what that move wrote, restoring it would quietly throw away
  /// whatever happened since, so the reason is spelled out instead.
  Widget _undoSection(ThemeData theme, AppLocalizations l10n) {
    final undo = _result?.undo;
    if (undo == null) return const SizedBox.shrink();
    return Padding(
      padding: const EdgeInsets.only(top: 6),
      child: Align(
        alignment: Alignment.centerLeft,
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            OutlinedButton.icon(
              key: const ValueKey('npc-position:undo'),
              icon: const Icon(Icons.undo, size: 18),
              label: Text(l10n.npcUndoPlacement),
              onPressed: widget.editable && undo.restorable && !_restoring
                  ? _undoPlacement
                  : null,
            ),
            if (!undo.restorable) ...[
              const SizedBox(height: 4),
              Text(
                l10n.npcUndoPlacementStale,
                style: TextStyle(color: theme.colorScheme.onSurfaceVariant),
              ),
            ],
          ],
        ),
      ),
    );
  }

  /// The six editable fields in the player editor's responsive 3+3 layout
  /// (same 700px breakpoint).
  Widget _fieldGrid(AppLocalizations l10n) {
    final locationEnabled = widget.editable && _canEditLocation;
    final rotationEnabled = widget.editable && _canEditRotation;
    final fields = [
      _field('location:x', _x, l10n.locationX, locationEnabled),
      _field('location:y', _y, l10n.locationY, locationEnabled),
      _field('location:z', _z, l10n.locationZ, locationEnabled),
      _field('rotation:pitch', _pitch, l10n.rotationPitch, rotationEnabled),
      _field('rotation:yaw', _yaw, l10n.rotationYaw, rotationEnabled),
      _field('rotation:roll', _roll, l10n.rotationRoll, rotationEnabled),
    ];
    return LayoutBuilder(
      builder: (context, constraints) {
        final compact = constraints.maxWidth < 700;
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
    );
  }

  Widget _field(
    String id,
    TextEditingController controller,
    String label,
    bool enabled,
  ) {
    return TransformNumberField(
      key: ValueKey('npc-position:$id'),
      controller: controller,
      label: label,
      enabled: enabled,
      onChanged: (_) => _userEdited(),
    );
  }

  /// The spawn pose, READ-ONLY. Rendered as plain text rather than disabled
  /// fields so it never reads as "editable but locked".
  Widget _spawnSection(ThemeData theme, AppLocalizations l10n, NpcPose pose) {
    final spawnLocation = pose.spawnLocation;
    final spawnRotation = pose.spawnRotation;
    final values = <String>[
      if (spawnLocation != null) ...[
        '${l10n.locationX}: ${formatHeroValue(spawnLocation.x)}',
        '${l10n.locationY}: ${formatHeroValue(spawnLocation.y)}',
        '${l10n.locationZ}: ${formatHeroValue(spawnLocation.z)}',
      ],
      if (spawnRotation != null) ...[
        '${l10n.rotationPitch}: ${formatHeroValue(spawnRotation.pitch)}',
        '${l10n.rotationYaw}: ${formatHeroValue(spawnRotation.yaw)}',
        '${l10n.rotationRoll}: ${formatHeroValue(spawnRotation.roll)}',
      ],
    ];
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Row(
          children: [
            const Icon(Icons.flag_outlined),
            const SizedBox(width: 8),
            Expanded(
              child: Text(
                l10n.spawnPositionSection,
                style: theme.textTheme.titleSmall,
              ),
            ),
          ],
        ),
        if (values.isNotEmpty) ...[
          const SizedBox(height: 8),
          Wrap(
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
        ],
        const SizedBox(height: 12),
        Align(
          alignment: Alignment.centerLeft,
          child: FilledButton.tonalIcon(
            icon: const Icon(Icons.restart_alt, size: 18),
            label: Text(l10n.resetToSpawnPosition),
            onPressed: _resetEnabled ? _resetToSpawn : null,
          ),
        ),
      ],
    );
  }
}
