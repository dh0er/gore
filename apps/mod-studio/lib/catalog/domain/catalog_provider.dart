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
/// absent from the model, it remains non-editable; no bounds or fields are
/// guessed from its name.
final catalogProvider = FutureProvider<List<CatalogItem>>((ref) async {
  final catalogJson = await rootBundle.loadString('assets/item_catalog.json');

  final catalogList = (jsonDecode(catalogJson) as List)
      .whereType<Map<String, Object?>>()
      .toList();

  // model.json shape:
  // { "classes": { "ItFo_Apple": { "fields": [ { "name", "type", "default" } ] } } }
  //
  // The bundled assets/model.json is the COMPLETE authority for each
  // `(catalog class, field name, raw type)` triple. A loaded game-data dump is
  // untrusted and may refresh only scalar defaults for exact bundled triples;
  // it can neither add a field to another class nor replace schema metadata.
  // A missing, unreadable, partial, or empty dump therefore leaves the bundled
  // schema and defaults intact.
  final bundled = await rootBundle.loadString('assets/model.json');
  final modelClasses = <String, Object?>{
    ...?(jsonDecode(bundled) as Map<String, Object?>?)?['classes']
        as Map<String, Object?>?,
  };

  final dumpPath = ref.watch(dumpPathProvider);
  if (dumpPath != null) {
    try {
      final dumpStr = await File(dumpPath).readAsString();
      final dumpClasses =
          (jsonDecode(dumpStr) as Map<String, Object?>?)?['classes']
              as Map<String, Object?>?;
      if (dumpClasses != null) {
        _overlayExactDumpDefaults(modelClasses, dumpClasses);
      }
    } catch (_) {
      // unreadable/invalid dump -> keep the bundled base
    }
  }

  return [
    for (final entry in catalogList)
      if ((entry['id'] as String?)?.isNotEmpty == true)
        CatalogItem.fromCatalogEntry(
          entry,
          fields: _fieldsFor(entry['id'] as String, modelClasses),
        ),
  ]..sort((a, b) => a.id.compareTo(b.id));
});

/// Apply only unambiguous, type-valid defaults from a user-selected dump.
///
/// The bundled per-class field list remains the schema authority. In
/// particular, a globally known specialized field cannot be attached to a
/// different catalog class by a corrupt dump. Duplicate dump entries are
/// ambiguous and therefore ignored for that exact pair.
void _overlayExactDumpDefaults(
  Map<String, Object?> bundledClasses,
  Map<String, Object?> dumpClasses,
) {
  for (final dumpClass in dumpClasses.entries) {
    final bundledClassValue = bundledClasses[dumpClass.key];
    final dumpedClassValue = dumpClass.value;
    if (bundledClassValue is! Map<String, Object?> ||
        dumpedClassValue is! Map<String, Object?>) {
      continue;
    }
    final bundledFieldsValue = bundledClassValue['fields'];
    final dumpedFieldsValue = dumpedClassValue['fields'];
    if (bundledFieldsValue is! List ||
        dumpedFieldsValue is! List ||
        dumpedFieldsValue.isEmpty) {
      continue;
    }
    final bundledClass = bundledClassValue;
    final bundledFields = bundledFieldsValue;
    final dumpedFields = dumpedFieldsValue;

    final defaults = <(String, String), Object?>{};
    final ambiguous = <(String, String)>{};
    for (final dumpedField in dumpedFields.whereType<Map<String, Object?>>()) {
      final name = dumpedField['name'];
      final rawType = dumpedField['type'];
      if (name is! String ||
          rawType is! String ||
          !dumpedField.containsKey('default')) {
        continue;
      }
      final key = (name, rawType);
      if (ambiguous.contains(key) || defaults.containsKey(key)) {
        defaults.remove(key);
        ambiguous.add(key);
        continue;
      }
      defaults[key] = dumpedField['default'];
    }

    var changed = false;
    final mergedFields = <Object?>[];
    for (final bundledFieldValue in bundledFields) {
      if (bundledFieldValue is! Map<String, Object?>) {
        mergedFields.add(bundledFieldValue);
        continue;
      }
      final name = bundledFieldValue['name'];
      final rawType = bundledFieldValue['type'];
      if (name is! String || rawType is! String) {
        mergedFields.add(bundledFieldValue);
        continue;
      }
      final key = (name, rawType);
      if (ambiguous.contains(key) || !defaults.containsKey(key)) {
        mergedFields.add(bundledFieldValue);
        continue;
      }

      final candidate = <String, Object?>{
        ...bundledFieldValue,
        'default': defaults[key],
      };
      try {
        final parsed = FieldSchema.fromItemModelJson(candidate);
        if (parsed.defaultValue == null) {
          mergedFields.add(bundledFieldValue);
          continue;
        }
      } on FormatException {
        mergedFields.add(bundledFieldValue);
        continue;
      }
      mergedFields.add(candidate);
      changed = true;
    }

    if (changed) {
      bundledClasses[dumpClass.key] = <String, Object?>{
        ...bundledClass,
        'fields': mergedFields,
      };
    }
  }
}

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
        .where(isProvenItemModelField)
        .map(FieldSchema.fromItemModelJson)
        .toList(),
  );
  return parsed;
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
