/// Domain types for the NPC position editor (`private.npc.position`).
///
/// The attribute editor's [NpcTypedEdit] carries a single `double` and so can
/// only address a scalar leaf. A saved pose is a NATIVE STRUCT leaf — the whole
/// `CharacterLocation` / `CharacterRotation` triplet is written in one go — so
/// these types carry a `Map<String, double>` value instead
/// ([NpcStructEdit]). The write itself still travels through the existing
/// `private.typed.setValue` command; only the value shape differs.
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

/// Result of loading one NPC's pose. Carries an inline [error] instead of
/// throwing, mirroring [NpcAttributesResult].
class NpcPoseResult {
  const NpcPoseResult({this.pose, this.error});

  factory NpcPoseResult.fromJson(Map<String, Object?> json) {
    final raw = json['pose'];
    if (raw is! Map) return const NpcPoseResult();
    return NpcPoseResult(pose: NpcPose.fromJson(raw.cast<String, Object?>()));
  }

  final NpcPose? pose;
  final String? error;
}
