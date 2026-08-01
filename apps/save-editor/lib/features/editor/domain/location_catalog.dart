import 'dart:convert';

import 'package:flutter/services.dart' show rootBundle;

/// One named interaction spot of the main map: a place a character can be moved
/// to, addressed by the same name the save file uses in `UsedSpot > Spotname`.
///
/// Only [yaw] is carried. Pitch and roll are deliberately absent from both the
/// asset and this model: a spot's pitch would visibly tilt a standing pawn, so
/// it is made structurally impossible to apply rather than merely discouraged.
class LocationSpot {
  LocationSpot({
    required this.name,
    required this.x,
    required this.y,
    required this.z,
    required this.yaw,
    required this.area,
  }) : search = name.toLowerCase();

  final String name;
  final double x;
  final double y;
  final double z;

  /// Yaw in degrees. The only rotation axis this catalog carries.
  final double yaw;

  /// Area code, matching a [LocationArea.id], or `''` when the spot could not
  /// be assigned to one.
  final String area;

  /// Lowercased [name], computed ONCE at parse time. The spot list is filtered
  /// client-side, so each keystroke must be a cheap substring scan over cached
  /// strings rather than 10k fresh `toLowerCase()` calls.
  final String search;
}

/// A named region of the map. [locId] resolves to a localized name through the
/// shared localization catalog; areas without one fall back to their English
/// [label], which is why area names need no ARB keys of their own:
/// `locCatalog[area.locId]?[lang] ?? area.label`.
class LocationArea {
  const LocationArea({
    required this.id,
    required this.label,
    required this.locId,
  });

  final String id;
  final String label;
  final String? locId;
}

/// The bundled catalog of named locations, generated from the game's
/// `InteractionSpots.json` by `gore location-catalog` (the source path is
/// optional — the command resolves it from the configured game install).
/// Regenerate after a game patch: the spot set is cook-specific.
class LocationCatalog {
  LocationCatalog({required this.spots, required this.areas})
      : _areasById = {for (final a in areas) a.id: a};

  final List<LocationSpot> spots;
  final List<LocationArea> areas;
  final Map<String, LocationArea> _areasById;

  LocationArea? areaById(String id) => _areasById[id];

  static LocationCatalog fromJsonString(String json) {
    final root = jsonDecode(json) as Map<String, Object?>;

    final areas = (root['areas'] as List? ?? const [])
        .whereType<Map<String, Object?>>()
        .map((a) => LocationArea(
              id: a['id'] as String? ?? '',
              label: a['label'] as String? ?? '',
              locId: a['locId'] as String?,
            ))
        .where((a) => a.id.isNotEmpty)
        .toList()
      ..sort((a, b) => a.id.compareTo(b.id));

    final spots = (root['spots'] as List? ?? const [])
        .whereType<Map<String, Object?>>()
        .map((s) => LocationSpot(
              name: s['n'] as String? ?? '',
              x: (s['x'] as num? ?? 0).toDouble(),
              y: (s['y'] as num? ?? 0).toDouble(),
              z: (s['z'] as num? ?? 0).toDouble(),
              yaw: (s['w'] as num? ?? 0).toDouble(),
              area: s['a'] as String? ?? '',
            ))
        // An all-zero coordinate is not a place on the map: the game's waypoint
        // layer stores those as placeholders. Teleporting anyone there would
        // drop them at the world origin.
        .where((s) =>
            s.name.isNotEmpty && !(s.x == 0 && s.y == 0 && s.z == 0))
        .toList()
      ..sort((a, b) {
        final byArea = a.area.compareTo(b.area);
        return byArea != 0 ? byArea : a.name.compareTo(b.name);
      });

    return LocationCatalog(spots: spots, areas: areas);
  }

  static Future<LocationCatalog> loadBundled() async =>
      fromJsonString(await rootBundle.loadString('assets/location_catalog.json'));
}
