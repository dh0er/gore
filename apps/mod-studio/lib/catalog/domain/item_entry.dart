import 'field_schema.dart';

/// One entry in the item catalog — a class name plus its editable fields.
class CatalogItem {
  const CatalogItem({
    required this.id,
    required this.displayName,
    required this.fields,
  });

  /// Angelscript short class name (e.g. `ItFo_Apple`).
  final String id;

  /// Human-readable name derived from the class id (no localisation data).
  final String displayName;

  /// Editable CDO fields for this item, populated only from model evidence.
  /// An item without per-class evidence receives an empty list.
  final List<FieldSchema> fields;

  /// Build a [CatalogItem] from the item_catalog.json entry +
  /// the per-class field list extracted from model.json.
  factory CatalogItem.fromCatalogEntry(
    Map<String, Object?> catalogEntry, {
    required List<FieldSchema> fields,
  }) {
    final id = catalogEntry['id'] as String? ?? '';
    return CatalogItem(id: id, displayName: _displayName(id), fields: fields);
  }

  static String _displayName(String id) {
    const prefixes = [
      'ItMw_',
      'ItRw_',
      'ItAr_',
      'ItFo_',
      'ItMi_',
      'ItAt_',
      'ItWr_',
      'ItMs_',
      'ItKe_',
      'ItAm_',
    ];
    var name = id;
    for (final prefix in prefixes) {
      if (name.startsWith(prefix)) {
        name = name.substring(prefix.length);
        break;
      }
    }
    final cleaned = name.replaceAll('_', ' ').trim();
    return cleaned.isEmpty ? id : cleaned;
  }
}
