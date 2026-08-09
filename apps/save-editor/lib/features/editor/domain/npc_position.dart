/// Domain types for the NPC position editor (`private.npc.position`).
///
/// The attribute editor's [NpcTypedEdit] carries a single `double` and so can
/// only address a scalar leaf. A saved pose is a NATIVE STRUCT leaf — the whole
/// `CharacterLocation` / `CharacterRotation` triplet is written in one go — so
/// these types carry a `Map<String, double>` value instead
/// ([NpcStructEdit]). The write itself still travels through the existing
/// `private.typed.setValue` command; only the value shape differs.
///
/// Whether the game applies the written pose on load is unsettled — see
/// `NpcPositionPanel` for what the earlier in-game tests did and did not rule
/// out, and why the write path exists again.
library;

/// A location triplet (`{x, y, z}`), as the core reports it for
/// `CharacterLocation` / `SpawnLocation`.
class Vec3 {
  const Vec3({required this.x, required this.y, required this.z});

  /// Parses `{x, y, z}`; returns null when the shape is not three finite nums
  /// (an absent leaf comes back as JSON `null`).
  static Vec3? fromJson(Object? value) {
    if (value is! Map) return null;
    final x = (value['x'] as num?)?.toDouble();
    final y = (value['y'] as num?)?.toDouble();
    final z = (value['z'] as num?)?.toDouble();
    if (x == null || y == null || z == null) return null;
    return Vec3(x: x, y: y, z: z);
  }

  final double x;
  final double y;
  final double z;

  /// The `private.typed.setValue` value payload for a Vector descriptor.
  Map<String, double> toValue() => {'x': x, 'y': y, 'z': z};

  /// True when every axis is exactly zero — the marker for "this actor was
  /// never placed in the world" (see [NpcPose.neverPlaced]).
  bool get isZero => x == 0 && y == 0 && z == 0;

  @override
  bool operator ==(Object other) =>
      other is Vec3 && other.x == x && other.y == y && other.z == z;

  @override
  int get hashCode => Object.hash(x, y, z);

  @override
  String toString() => 'Vec3($x, $y, $z)';
}

/// A rotation triplet (`{pitch, yaw, roll}`). The core renames the Rotator's
/// x/y/z members for this command specifically — see `NpcPose` in
/// `crates/gore-save/src/npc.rs` for why that rename lives there and nowhere
/// else.
class Rot3 {
  const Rot3({required this.pitch, required this.yaw, required this.roll});

  static Rot3? fromJson(Object? value) {
    if (value is! Map) return null;
    final pitch = (value['pitch'] as num?)?.toDouble();
    final yaw = (value['yaw'] as num?)?.toDouble();
    final roll = (value['roll'] as num?)?.toDouble();
    if (pitch == null || yaw == null || roll == null) return null;
    return Rot3(pitch: pitch, yaw: yaw, roll: roll);
  }

  final double pitch;
  final double yaw;
  final double roll;

  /// The `private.typed.setValue` value payload for a Rotator descriptor — the
  /// core accepts pitch/yaw/roll there and maps them back onto the struct's
  /// memory order.
  Map<String, double> toValue() => {'pitch': pitch, 'yaw': yaw, 'roll': roll};

  @override
  bool operator ==(Object other) =>
      other is Rot3 &&
      other.pitch == pitch &&
      other.yaw == yaw &&
      other.roll == roll;

  @override
  int get hashCode => Object.hash(pitch, yaw, roll);

  @override
  String toString() => 'Rot3($pitch, $yaw, $roll)';
}

/// One pending `private.typed.setValue` edit whose value is a STRUCT (a whole
/// location or rotation triplet), not a scalar. The Map-valued sibling of
/// [NpcTypedEdit] in npc_attributes.dart.
class NpcStructEdit {
  const NpcStructEdit({required this.path, required this.value});

  /// Full typed path to the struct leaf, as returned by the core.
  final List<String> path;

  /// `{x, y, z}` for a location, `{pitch, yaw, roll}` for a rotation.
  final Map<String, double> value;
}

/// One NPC's saved pose as returned by `private.npc.position`.
///
/// Each leaf is nullable (the member may be absent, or not a triplet) while its
/// typed path is always reported, so a caller can tell "absent leaf" from
/// "absent NPC" (an absent NPC is an error, not an empty pose).
class NpcPose {
  const NpcPose({
    this.location,
    this.rotation,
    this.spawnLocation,
    this.spawnRotation,
    this.locationPath = const [],
    this.rotationPath = const [],
    this.spawnLocationPath = const [],
    this.spawnRotationPath = const [],
  });

  factory NpcPose.fromJson(Map<String, Object?> json) {
    return NpcPose(
      location: Vec3.fromJson(json['location']),
      rotation: Rot3.fromJson(json['rotation']),
      spawnLocation: Vec3.fromJson(json['spawnLocation']),
      spawnRotation: Rot3.fromJson(json['spawnRotation']),
      locationPath: _stringList(json['locationPath']),
      rotationPath: _stringList(json['rotationPath']),
      spawnLocationPath: _stringList(json['spawnLocationPath']),
      spawnRotationPath: _stringList(json['spawnRotationPath']),
    );
  }

  final Vec3? location;
  final Rot3? rotation;
  final Vec3? spawnLocation;
  final Rot3? spawnRotation;

  final List<String> locationPath;
  final List<String> rotationPath;
  final List<String> spawnLocationPath;
  final List<String> spawnRotationPath;

  /// The one signal that this pose is writable: there is no per-NPC `writable`
  /// list, so "the core resolved a path" IS the permission. An empty path means
  /// the leaf cannot be addressed by `private.typed.setValue`.
  bool get locationWritable => locationPath.isNotEmpty;
  bool get rotationWritable => rotationPath.isNotEmpty;
  bool get editable => locationWritable || rotationWritable;

  /// True when the stored location is exactly the origin: the NPC has never
  /// been placed in the world, so the game most likely ignores this entry and
  /// spawns it from its level placement instead. Editing stays allowed — the
  /// UI only warns.
  bool get neverPlaced => location?.isZero ?? false;

  static List<String> _stringList(Object? value) {
    if (value is! List) return const [];
    return value.whereType<String>().toList(growable: false);
  }
}

/// Everything one NPC's Position panel wants written on the next save.
///
/// The coordinate groups and the routine class travel together because they are
/// one user action — "put him there and let him stay" — even though they land in
/// two different maps. [note] rides along so the routine class this replaces is
/// written down before it is gone; [clearNote] is its opposite, sent by a
/// restore that has just spent the note.
class NpcPlacementDraft {
  const NpcPlacementDraft({
    this.edits = const [],
    this.routineClassPath = const [],
    this.routineClass,
    this.note,
    this.clearNote = false,
  });

  /// One entry per coordinate group that differs from the loaded pose.
  final List<NpcStructEdit> edits;

  /// Where to write [routineClass]. Empty when the routine is not being touched.
  final List<String> routineClassPath;
  final String? routineClass;

  /// The undo note to record, in the core's `placementNotes` entry shape.
  final Map<String, Object?>? note;
  final bool clearNote;

  bool get isEmpty => edits.isEmpty && routineClass == null && !clearNote;
}

/// What an earlier "keep him here" move replaced, so it can be taken back.
///
/// [restorable] is the core's verdict, not a hint: it is false once the save no
/// longer holds what that move wrote — the player saved in game and the NPC
/// moved on, or another tool touched him — because restoring then would discard
/// whatever happened since. The UI offers the action only while it is true.
class PlacementUndo {
  const PlacementUndo({
    required this.originalLocation,
    required this.originalRotation,
    required this.originalRoutineClass,
    required this.restorable,
    required this.routineRestorable,
  });

  static PlacementUndo? fromJson(Object? value) {
    if (value is! Map) return null;
    final location = Vec3.fromJson(value['originalLocation']);
    if (location == null) return null;
    return PlacementUndo(
      originalLocation: location,
      originalRotation: Rot3.fromJson(value['originalRotation']),
      originalRoutineClass: value['originalRoutineClass'] as String?,
      restorable: value['restorable'] == true,
      routineRestorable: value['routineRestorable'] == true,
    );
  }

  final Vec3 originalLocation;

  /// The facing the move replaced, when it changed one. Null when the move left
  /// the facing alone — a restore then leaves it alone too.
  final Rot3? originalRotation;

  /// The routine the NPC was on. Null when he had no routine record at all — a
  /// restore then puts the position back and writes no routine, rather than
  /// inventing one the save never had.
  final String? originalRoutineClass;

  /// Whether the whole move — position AND routine — can be put back.
  final bool restorable;

  /// Whether the ROUTINE alone can be put back: the NPC is still on the class
  /// the move wrote, whatever has happened to his position since.
  final bool routineRestorable;

  /// Whether "give him his routine back" has both a target and a right to fire.
  bool get canRestoreRoutine =>
      routineRestorable && (originalRoutineClass?.isNotEmpty ?? false);
}

/// Result of loading one NPC's pose. Carries an inline [error] instead of
/// throwing, mirroring [NpcAttributesResult].
class NpcPoseResult {
  const NpcPoseResult({
    this.pose,
    this.error,
    this.routineClass,
    this.routineClassPath = const [],
    this.inertRoutineClass,
    this.undo,
  });

  factory NpcPoseResult.fromJson(Map<String, Object?> json) {
    final raw = json['pose'];
    if (raw is! Map) return const NpcPoseResult();
    return NpcPoseResult(
      pose: NpcPose.fromJson(raw.cast<String, Object?>()),
      routineClass: json['routineClass'] as String?,
      routineClassPath: NpcPose._stringList(json['routineClassPath']),
      inertRoutineClass: json['inertRoutineClass'] as String?,
      undo: PlacementUndo.fromJson(json['undo']),
    );
  }

  final NpcPose? pose;
  final String? error;

  /// The NPC's current daily-routine class, and the typed path that writes it.
  /// Every other routine walks or teleports a moved NPC back, so pinning him
  /// means replacing this with [inertRoutineClass].
  final String? routineClass;
  final List<String> routineClassPath;

  /// The routine class that does nothing at all, as named by the core. Read from
  /// the core rather than spelled out here so the two can never drift.
  final String? inertRoutineClass;

  final PlacementUndo? undo;

  /// Whether "keep him where I put him" can be offered: the routine leaf has to
  /// be addressable, the core has to have named an inert class to put there, and
  /// there has to be a location to write down.
  ///
  /// The last one because a pin without a recordable position has no undo: the
  /// note needs the location it replaced. Offering a checkbox that queues
  /// nothing while staying visibly ticked is worse than not offering it.
  bool get canPin =>
      routineClassPath.isNotEmpty &&
      (inertRoutineClass?.isNotEmpty ?? false) &&
      pose?.location != null;

  /// True once the NPC is already on the inert routine — he stays put as things
  /// stand, so the checkbox starts ticked and changing nothing changes nothing.
  bool get isPinned =>
      inertRoutineClass != null && routineClass == inertRoutineClass;
}
