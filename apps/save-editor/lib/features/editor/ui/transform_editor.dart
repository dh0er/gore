import 'package:flutter/material.dart';
import 'package:goresave/features/editor/domain/editor_models.dart';
import 'package:goresave/features/editor/domain/pending_edits.dart';
import 'package:goresave/features/editor/ui/hero_stats_card.dart'
    show formatHeroValue;
import 'package:goresave/features/editor/ui/location_picker_dialog.dart';
import 'package:goresave/l10n/app_localizations.dart';

import '../domain/editor_notifier.dart';

/// The PLAYER's transform editor: six number fields (location XYZ + rotation
/// pitch/yaw/roll) driving the single `'transform'` pending-edit key via the
/// `private.player.setTransform` command.
///
/// Promoted out of `attribute_detail.dart` (where it was `_PrivatePlayerTransform
/// Editor`) so the Position sub-tab can host it. It must be mounted EXACTLY
/// ONCE: two live instances would both drive the one `'transform'` key and the
/// last writer would win.
class PlayerTransformEditor extends StatefulWidget {
  const PlayerTransformEditor({
    super.key,
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
  State<PlayerTransformEditor> createState() => _PlayerTransformEditorState();
}

class _PlayerTransformEditorState extends State<PlayerTransformEditor> {
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
  void didUpdateWidget(covariant PlayerTransformEditor oldWidget) {
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
    _locationXController.text = formatHeroValue(transform.location.x);
    _locationYController.text = formatHeroValue(transform.location.y);
    _locationZController.text = formatHeroValue(transform.location.z);
    _rotationPitchController.text = formatHeroValue(transform.rotation.pitch);
    _rotationYawController.text = formatHeroValue(transform.rotation.yaw);
    _rotationRollController.text = formatHeroValue(transform.rotation.roll);
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
        // The SAME picker the NPC panel uses. It is write-agnostic, so the
        // player keeps its own write: the fields below still drive the single
        // `'transform'` key through `private.player.setTransform`.
        const SizedBox(height: 12),
        Align(
          alignment: Alignment.centerLeft,
          child: OutlinedButton.icon(
            icon: const Icon(Icons.travel_explore, size: 18),
            label: Text(l10n.pickLocation),
            onPressed: widget.editable ? _pickLocation : null,
          ),
        ),
        const SizedBox(height: 10),
        LayoutBuilder(
          builder: (context, constraints) {
            final compact = constraints.maxWidth < 700;
            final fields = [
              TransformNumberField(
                controller: _locationXController,
                label: l10n.locationX,
                enabled: widget.editable,
                onChanged: (_) => _updatePending(),
              ),
              TransformNumberField(
                controller: _locationYController,
                label: l10n.locationY,
                enabled: widget.editable,
                onChanged: (_) => _updatePending(),
              ),
              TransformNumberField(
                controller: _locationZController,
                label: l10n.locationZ,
                enabled: widget.editable,
                onChanged: (_) => _updatePending(),
              ),
              TransformNumberField(
                controller: _rotationPitchController,
                label: l10n.rotationPitch,
                enabled: widget.editable,
                onChanged: (_) => _updatePending(),
              ),
              TransformNumberField(
                controller: _rotationYawController,
                label: l10n.rotationYaw,
                enabled: widget.editable,
                onChanged: (_) => _updatePending(),
              ),
              TransformNumberField(
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

  /// Open the shared location picker and seed the fields from the chosen spot.
  /// The picker touches CONTROLLERS ONLY — the existing [_updatePending] then
  /// registers the same `private.player.setTransform` edit a hand-typed
  /// coordinate would, so there is no second write route.
  ///
  /// Rotation is applied only on opt-in, and then as pitch 0 / yaw / roll 0:
  /// the catalog stores yaw alone, so pitch and roll are not ours to invent.
  Future<void> _pickLocation() async {
    final pick = await showLocationPickerDialog(context);
    if (pick == null || !mounted) return;
    final spot = pick.spot;
    _locationXController.text = formatHeroValue(spot.x);
    _locationYController.text = formatHeroValue(spot.y);
    _locationZController.text = formatHeroValue(spot.z);
    if (pick.applyRotation) {
      _rotationPitchController.text = formatHeroValue(0);
      _rotationYawController.text = formatHeroValue(spot.yaw);
      _rotationRollController.text = formatHeroValue(0);
    }
    _updatePending();
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

/// One signed-decimal number field of a transform/position editor. Promoted out
/// of `attribute_detail.dart` (`_TransformNumberField`) so the NPC position
/// panel renders the same field as the player's editor.
class TransformNumberField extends StatelessWidget {
  const TransformNumberField({
    super.key,
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
