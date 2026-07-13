import 'dart:convert';

import 'package:flutter/services.dart';

enum NpcGlossaryCamp { oldCamp, newCamp, swampCamp, outsiders }

enum NpcGlossaryRole { portrait, trader, teacher, armorer, dead, hostile }

class NpcGlossaryCatalogEntry {
  const NpcGlossaryCatalogEntry({
    required this.id,
    required this.uniqueName,
    required this.documentClass,
    required this.camp,
    required this.segments,
  });

  factory NpcGlossaryCatalogEntry.fromJson(Map<String, Object?> json) {
    return NpcGlossaryCatalogEntry(
      id: json['id'] as String? ?? '',
      uniqueName: json['uniqueName'] as String? ?? '',
      documentClass: json['documentClass'] as String? ?? '',
      camp: NpcGlossaryCamp.values.firstWhere(
        (camp) => camp.name == json['camp'],
        orElse: () => NpcGlossaryCamp.outsiders,
      ),
      segments:
          (json['segments'] as List?)
              ?.whereType<Map>()
              .map(
                (value) => NpcGlossaryCatalogSegment.fromJson(
                  value.cast<String, Object?>(),
                ),
              )
              .toList(growable: false) ??
          const [],
    );
  }

  final String id;
  final String uniqueName;
  final String documentClass;
  final NpcGlossaryCamp camp;
  final List<NpcGlossaryCatalogSegment> segments;

  NpcGlossaryCatalogSegment? get portraitSegment {
    // A single NPC (Herek) has both Introduction and Introduction_2. The
    // unsuffixed segment is the canonical first-meeting entry; keep the
    // alternate as a fallback for saves/documents that only define a variant.
    for (final segment in segments) {
      if (segment.id == 'Introduction' &&
          segment.roles.contains(NpcGlossaryRole.portrait)) {
        return segment;
      }
    }
    for (final segment in segments) {
      if (segment.roles.contains(NpcGlossaryRole.portrait)) return segment;
    }
    return null;
  }
}

class NpcGlossaryCatalogSegment {
  const NpcGlossaryCatalogSegment({
    required this.id,
    required this.segmentClass,
    required this.label,
    this.roles = const {},
  });

  factory NpcGlossaryCatalogSegment.fromJson(Map<String, Object?> json) {
    final roles = <NpcGlossaryRole>{};
    for (final name
        in (json['roles'] as List?)?.whereType<String>() ?? const []) {
      for (final role in NpcGlossaryRole.values) {
        if (role.name == name) roles.add(role);
      }
    }
    return NpcGlossaryCatalogSegment(
      id: json['id'] as String? ?? '',
      segmentClass: json['class'] as String? ?? '',
      label: json['label'] as String? ?? '',
      roles: Set.unmodifiable(roles),
    );
  }

  final String id;
  final String segmentClass;
  final String label;
  final Set<NpcGlossaryRole> roles;
}

const glossaryNpcCatalogAsset = 'assets/glossary_npc_catalog.json';

Future<List<NpcGlossaryCatalogEntry>> loadGlossaryNpcCatalog({
  AssetBundle? bundle,
}) async {
  final text = await (bundle ?? rootBundle).loadString(glossaryNpcCatalogAsset);
  final decoded = jsonDecode(text);
  if (decoded is! List) {
    throw const FormatException('NPC glossary catalog root must be a list');
  }
  return decoded
      .whereType<Map>()
      .map(
        (value) =>
            NpcGlossaryCatalogEntry.fromJson(value.cast<String, Object?>()),
      )
      .where(
        (entry) =>
            entry.documentClass.isNotEmpty &&
            entry.uniqueName.isNotEmpty &&
            entry.segments.isNotEmpty,
      )
      .toList(growable: false);
}
