import 'dart:convert';
import 'package:flutter/services.dart' show rootBundle;
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'item_entry.dart';
import 'field_schema.dart';

/// Loads item_catalog.json + model.json from assets and builds the
/// [CatalogItem] list. The model provides per-class field lists; when a class
/// is absent from the model, [kDefaultItemFields] is used.
final catalogProvider = FutureProvider<List<CatalogItem>>((ref) async {
  final catalogJson = await rootBundle.loadString('assets/item_catalog.json');
  final modelJson   = await rootBundle.loadString('assets/model.json');

  final catalogList = (jsonDecode(catalogJson) as List)
      .whereType<Map<String, Object?>>()
      .toList();

  // model.json shape:
  // { "classes": { "ItFo_Apple": { "fields": [ { "name": ..., "type": ... } ] } } }
  final modelClasses = (
    (jsonDecode(modelJson) as Map<String, Object?>?)?['classes']
        as Map<String, Object?>?
  ) ?? {};

  return [
    for (final entry in catalogList)
      if ((entry['id'] as String?)?.isNotEmpty == true)
        CatalogItem.fromCatalogEntry(
          entry,
          fields: _fieldsFor(entry['id'] as String, modelClasses),
        ),
  ]..sort((a, b) => a.id.compareTo(b.id));
});

List<FieldSchema> _fieldsFor(
  String classId,
  Map<String, Object?> modelClasses,
) {
  final classData = modelClasses[classId] as Map<String, Object?>?;
  final rawFields = classData?['fields'] as List?;
  if (rawFields == null || rawFields.isEmpty) return kDefaultItemFields;
  final parsed = rawFields
      .whereType<Map<String, Object?>>()
      .map(FieldSchema.fromJson)
      .toList();
  return parsed.isEmpty ? kDefaultItemFields : mergeDefaultBounds(parsed);
}

/// Overlay the [kDefaultItemFields] min/max bounds onto matching parsed model
/// fields. The bundled model.json carries only name/type per field, so without
/// this the well-known scalar fields (m_Value, m_MaxStack, m_Weight, m_Mass)
/// would have null bounds and validateField would accept out-of-range values
/// like m_MaxStack = 0 or a negative weight — and since export no longer runs
/// native range validation, those would reach the generated mod. A bound that
/// the parsed field already specifies is left untouched.
List<FieldSchema> mergeDefaultBounds(List<FieldSchema> parsed) {
  final defaultsByName = {for (final f in kDefaultItemFields) f.name: f};
  return [
    for (final f in parsed)
      if (defaultsByName[f.name] case final def?)
        FieldSchema(
          name: f.name,
          type: f.type,
          minValue: f.minValue ?? def.minValue,
          maxValue: f.maxValue ?? def.maxValue,
          enumValues: f.enumValues,
        )
      else
        f,
  ];
}
