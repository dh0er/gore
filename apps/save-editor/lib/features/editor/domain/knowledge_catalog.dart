import 'dart:convert';

import 'package:flutter/services.dart' show rootBundle;

class KnowledgeCatalogEntry {
  const KnowledgeCatalogEntry({
    required this.id,
    required this.category,
    this.caption,
    this.locKey,
    this.module,
  });

  final String id;
  final String category;
  final String? caption;
  final String? locKey;
  final String? module;
}

class KnowledgeCatalog {
  KnowledgeCatalog(List<KnowledgeCatalogEntry> entries)
    : entries = List.unmodifiable(entries),
      _byLowercaseId = {
        for (final entry in entries) entry.id.toLowerCase(): entry,
      };

  final List<KnowledgeCatalogEntry> entries;
  final Map<String, KnowledgeCatalogEntry> _byLowercaseId;

  /// Case-insensitive lookup for cache-derived caption metadata.
  KnowledgeCatalogEntry? entryById(String id) =>
      _byLowercaseId[id.toLowerCase()];

  static KnowledgeCatalog fromJsonString(String json) {
    final list =
        (jsonDecode(json) as List)
            .whereType<Map<String, Object?>>()
            .map(
              (e) => KnowledgeCatalogEntry(
                id: e['id'] as String? ?? '',
                category: e['category'] as String? ?? 'topic',
                caption: e['caption'] as String?,
                locKey: e['loc_key'] as String?,
                module: e['module'] as String?,
              ),
            )
            .where((e) => e.id.isNotEmpty)
            .toList()
          ..sort((a, b) => a.id.compareTo(b.id));
    return KnowledgeCatalog(list);
  }

  static Future<KnowledgeCatalog> loadBundled() async => fromJsonString(
    await rootBundle.loadString('assets/knowledge_catalog.json'),
  );
}
