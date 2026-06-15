import 'dart:convert';

import 'package:flutter/services.dart' show rootBundle;

class KnowledgeCatalogEntry {
  const KnowledgeCatalogEntry({required this.id, required this.category});

  final String id;
  final String category;
}

class KnowledgeCatalog {
  const KnowledgeCatalog(this.entries);

  final List<KnowledgeCatalogEntry> entries;

  static KnowledgeCatalog fromJsonString(String json) {
    final list = (jsonDecode(json) as List)
        .whereType<Map<String, Object?>>()
        .map((e) => KnowledgeCatalogEntry(
              id: e['id'] as String? ?? '',
              category: e['category'] as String? ?? 'topic',
            ))
        .where((e) => e.id.isNotEmpty)
        .toList()
      ..sort((a, b) => a.id.compareTo(b.id));
    return KnowledgeCatalog(list);
  }

  static Future<KnowledgeCatalog> loadBundled() async =>
      fromJsonString(await rootBundle.loadString('assets/knowledge_catalog.json'));
}
