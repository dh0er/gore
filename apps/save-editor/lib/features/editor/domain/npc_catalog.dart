import 'dart:convert';

import 'package:flutter/services.dart' show rootBundle;

class NpcCatalogEntry {
  const NpcCatalogEntry({
    required this.id,
    required this.className,
    required this.category,
  });

  final String id;
  final String className;
  final String category;
}

class NpcCatalog {
  const NpcCatalog(this.entries);

  final List<NpcCatalogEntry> entries;

  static NpcCatalog fromJsonString(String json) {
    final list = (jsonDecode(json) as List)
        .whereType<Map<String, Object?>>()
        .map((e) => NpcCatalogEntry(
              id: e['id'] as String? ?? '',
              className: e['class'] as String? ?? '',
              category: e['category'] as String? ?? 'other',
            ))
        .where((e) => e.id.isNotEmpty)
        .toList()
      ..sort((a, b) => a.id.compareTo(b.id));
    return NpcCatalog(list);
  }

  static Future<NpcCatalog> loadBundled() async =>
      fromJsonString(await rootBundle.loadString('assets/npc_catalog.json'));
}
