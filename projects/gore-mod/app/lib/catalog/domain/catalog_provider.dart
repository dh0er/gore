import 'dart:convert';
import 'dart:io';
import 'package:flutter/services.dart' show rootBundle;
import 'package:flutter_riverpod/flutter_riverpod.dart';
import '../../app/domain/ui_settings.dart';
import 'item_entry.dart';
import 'field_schema.dart';

/// Loads the item allow-list (bundled item_catalog.json) + the field/value
/// model and builds the [CatalogItem] list. The model is the bundled
/// model.json unless the user has loaded a fresh game-data dump
/// ([dumpPathProvider]), in which case that file is read instead — that's how a
/// post-release dump refreshes the editor without a rebuild. When a class is
/// absent from the model, [kDefaultItemFields] is used.
final catalogProvider = FutureProvider<List<CatalogItem>>((ref) async {
  final catalogJson = await rootBundle.loadString('assets/item_catalog.json');

  final dumpPath = ref.watch(dumpPathProvider);
  final String modelJson;
  if (dumpPath != null) {
    modelJson = await File(dumpPath).readAsString();
  } else {
    modelJson = await rootBundle.loadString('assets/model.json');
  }

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

/// Editable fields for a catalog class, taken from the model. A class absent
/// from the model (or with no listed fields) gets NO fields and is therefore
/// not editable — the model is the single source of truth for the schema, and
/// guessing a default field set would let skipped or version-mismatched items
/// export a stale/wrong schema. Both gui-model and sync emit per-class fields,
/// and the bundled model covers every catalog id, so this is not a fallback
/// path in practice — it only guards genuinely-missing classes.
List<FieldSchema> _fieldsFor(
  String classId,
  Map<String, Object?> modelClasses,
) {
  final classData = modelClasses[classId] as Map<String, Object?>?;
  final rawFields = classData?['fields'] as List?;
  if (rawFields == null || rawFields.isEmpty) return const [];
  final parsed = editableFields(
    rawFields
        .whereType<Map<String, Object?>>()
        .map(FieldSchema.fromJson)
        .toList(),
  );
  return parsed.isEmpty ? const [] : mergeDefaultBounds(parsed);
}

/// Drop fields the editor cannot present a working control for. An enum field
/// with no choices (the gore-cli gui-model currently records only name/type
/// for enum fields, omitting the members) would render an empty dropdown that
/// validateField rejects for every value — i.e. visible but impossible to
/// edit or export. Skip those rather than show a broken control.
List<FieldSchema> editableFields(List<FieldSchema> fields) => [
      for (final f in fields)
        if (f.type != FieldType.enum_ || f.enumValues.isNotEmpty) f,
    ];

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
          enumBackingValues: f.enumBackingValues,
          defaultValue: f.defaultValue,
        )
      else
        f,
  ];
}
