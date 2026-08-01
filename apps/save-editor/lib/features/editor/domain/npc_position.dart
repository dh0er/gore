/// Domain types for the NPC position READER (`private.npc.position`).
///
/// Read-only by design, not by omission. The save's per-NPC pose is a snapshot
/// written at save time and discarded on load — a UE4SS runtime probe rewrote
/// `CharacterLocation`, `SpawnLocation` and `DailyRoutineClass` for two NPCs,
/// loaded the byte-verified save, and read back the ORIGINAL pre-edit values in
/// every field. Placement authority is the level's WorldPointActor named in the
/// NPC's GlobalId. So these types carry no write payload and no typed paths:
/// there is nothing here to address with `private.typed.setValue`.
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

/// One NPC's saved pose as returned by `private.npc.position`.
///
/// Each leaf is nullable: the member may be absent, or not a triplet. An absent
/// NPC is an error and never reaches this type, so "no members" means "this
/// entry stores nothing", not "no such NPC".
///
/// The core also reports a typed path per member. The editor ignores them: they
/// address a record the game discards on load, so there is no write to make.
class NpcPose {
  const NpcPose({
    this.location,
    this.rotation,
    this.spawnLocation,
    this.spawnRotation,
  });

  factory NpcPose.fromJson(Map<String, Object?> json) {
    return NpcPose(
      location: Vec3.fromJson(json['location']),
      rotation: Rot3.fromJson(json['rotation']),
      spawnLocation: Vec3.fromJson(json['spawnLocation']),
      spawnRotation: Rot3.fromJson(json['spawnRotation']),
    );
  }

  final Vec3? location;
  final Rot3? rotation;
  final Vec3? spawnLocation;
  final Rot3? spawnRotation;
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
