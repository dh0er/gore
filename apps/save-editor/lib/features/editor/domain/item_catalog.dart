import 'dart:convert';

import 'package:flutter/services.dart' show rootBundle;

class ItemCatalogEntry {
  const ItemCatalogEntry({
    required this.id,
    required this.path,
    required this.category,
    required this.icon,
  });

  final String id;
  final String path;
  final String category;
  final String icon;
}

class ItemCatalog {
  const ItemCatalog(this.entries);

  final List<ItemCatalogEntry> entries;

  static ItemCatalog fromJsonString(String json) {
    final list =
        (jsonDecode(json) as List)
            .whereType<Map<String, Object?>>()
            .map(
              (e) => ItemCatalogEntry(
                id: e['id'] as String? ?? '',
                path: e['path'] as String? ?? '',
                category: e['category'] as String? ?? 'special',
                icon: e['icon'] as String? ?? '',
              ),
            )
            .where(
              (e) => e.id.isNotEmpty && e.path.isNotEmpty && e.icon.isNotEmpty,
            )
            .toList()
          ..sort((a, b) => a.id.compareTo(b.id));
    return ItemCatalog(list);
  }

  static Future<ItemCatalog> loadBundled() async =>
      fromJsonString(await rootBundle.loadString('assets/item_catalog.json'));
}
