import 'dart:convert';

import 'package:flutter/services.dart';

import '../catalog/domain/field_schema.dart';

/// Exact closed category vocabulary carried by the bundled item catalog.
enum Revision3ItemCategory {
  meleeWeapon('melee_weapon'),
  rangedWeapon('ranged_weapon'),
  ammunition('ammunition'),
  rune('rune'),
  scroll('scroll'),
  food('food'),
  misc('misc'),
  amulet('amulet'),
  ring('ring'),
  trophy('trophy'),
  writing('writing'),
  mission('mission'),
  key('key'),
  special('special');

  const Revision3ItemCategory(this.wireName);

  final String wireName;

  static Revision3ItemCategory parse(Object? value, String context) {
    for (final category in values) {
      if (category.wireName == value) return category;
    }
    throw FormatException('$context has an invalid category');
  }
}

/// One presentation-only item and the exact facts bundled for it.
final class Revision3ItemCatalogEntry {
  Revision3ItemCatalogEntry({
    required this.id,
    required this.displayName,
    required this.category,
    required List<FieldSchema> fields,
  }) : fields = List<FieldSchema>.unmodifiable(fields);

  final String id;
  final String displayName;
  final Revision3ItemCategory category;
  final List<FieldSchema> fields;
}

/// Read-only projection of the bundled base-game item catalog.
///
/// This catalog is presentation evidence only. It does not grant project
/// mutation, build, deployment, or runtime authority.
final class Revision3ItemCatalog {
  Revision3ItemCatalog._(List<Revision3ItemCatalogEntry> items)
    : items = List<Revision3ItemCatalogEntry>.unmodifiable(items);

  final List<Revision3ItemCatalogEntry> items;

  factory Revision3ItemCatalog.fromJson({
    required String itemCatalogJson,
    required String modelJson,
  }) {
    final catalogRoot = _decodeJson(itemCatalogJson, 'item catalog');
    if (catalogRoot is! List<Object?>) {
      throw const FormatException('item catalog must be a JSON array');
    }

    final modelRoot = _decodeJson(modelJson, 'item field model');
    if (modelRoot is! Map<String, Object?>) {
      throw const FormatException('item field model must be a JSON object');
    }
    final modelClasses = modelRoot['classes'];
    if (modelClasses is! Map<String, Object?>) {
      throw const FormatException(
        'item field model must contain a classes object',
      );
    }

    final ids = <String>{};
    final items = <Revision3ItemCatalogEntry>[];
    for (var index = 0; index < catalogRoot.length; index++) {
      final entry = catalogRoot[index];
      if (entry is! Map<String, Object?>) {
        throw FormatException('item catalog entry $index must be an object');
      }
      final id = entry['id'];
      if (id is! String || id.isEmpty || id.trim() != id) {
        throw FormatException('item catalog entry $index has an invalid id');
      }
      if (!ids.add(id)) {
        throw FormatException('item catalog contains duplicate id $id');
      }
      final category = Revision3ItemCategory.parse(
        entry['category'],
        'item catalog entry $index',
      );

      items.add(
        Revision3ItemCatalogEntry(
          id: id,
          displayName: _displayName(id),
          category: category,
          fields: _fieldsFor(id, modelClasses),
        ),
      );
    }
    items.sort((left, right) => left.id.compareTo(right.id));
    return Revision3ItemCatalog._(items);
  }
}

typedef Revision3ItemCatalogLoader = Future<Revision3ItemCatalog> Function();

/// Load the two immutable assets shipped with Mod Studio.
///
/// No user dump, legacy provider, project file, or fallback reader participates
/// in this managed read-only route.
Future<Revision3ItemCatalog> loadRevision3BundledItemCatalog({
  AssetBundle? bundle,
}) async {
  final source = bundle ?? rootBundle;
  final documents = await Future.wait<String>([
    source.loadString('assets/item_catalog.json'),
    source.loadString('assets/model.json'),
  ]);
  return Revision3ItemCatalog.fromJson(
    itemCatalogJson: documents[0],
    modelJson: documents[1],
  );
}

Object? _decodeJson(String source, String name) {
  try {
    return jsonDecode(source);
  } on FormatException catch (error) {
    throw FormatException('$name is not valid JSON: ${error.message}');
  }
}

List<FieldSchema> _fieldsFor(
  String classId,
  Map<String, Object?> modelClasses,
) {
  final classData = modelClasses[classId];
  if (classData == null) return const <FieldSchema>[];
  if (classData is! Map<String, Object?>) {
    throw FormatException('item field model class $classId must be an object');
  }
  final rawFields = classData['fields'];
  if (rawFields == null) return const <FieldSchema>[];
  if (rawFields is! List<Object?>) {
    throw FormatException(
      'item field model class $classId fields must be an array',
    );
  }

  final names = <String>{};
  final fields = <FieldSchema>[];
  for (var index = 0; index < rawFields.length; index++) {
    final raw = rawFields[index];
    if (raw is! Map<String, Object?>) {
      throw FormatException('$classId field $index must be an object');
    }
    final field = _parseField(raw, '$classId field $index');
    if (!names.add(field.name)) {
      throw FormatException('$classId contains duplicate field ${field.name}');
    }
    fields.add(field);
  }
  return List<FieldSchema>.unmodifiable(fields);
}

FieldSchema _parseField(Map<String, Object?> raw, String context) {
  final name = raw['name'];
  if (name is! String || name.isEmpty || name.trim() != name) {
    throw FormatException('$context has an invalid name');
  }
  final type = raw['type'];
  if (type is! String ||
      !const <String>{
        'int',
        'float',
        'double',
        'bool',
        'string',
        'enum',
      }.contains(type)) {
    throw FormatException('$context has an unsupported type');
  }
  for (final key in const <String>['min', 'max']) {
    final value = raw[key];
    if (value != null && (value is! num || !_finite(value))) {
      throw FormatException('$context has an invalid $key bound');
    }
  }
  final minimum = raw['min'] as num?;
  final maximum = raw['max'] as num?;
  if (minimum != null && maximum != null && minimum > maximum) {
    throw FormatException('$context has inverted bounds');
  }

  final enumValues = raw['enum_values'];
  if (enumValues != null &&
      (enumValues is! List<Object?> ||
          enumValues.any((value) => value is! String))) {
    throw FormatException('$context has invalid enum values');
  }
  final enumBackingValues = raw['enum_value_ints'];
  if (enumBackingValues != null &&
      (enumBackingValues is! List<Object?> ||
          enumBackingValues.any((value) => value is! int))) {
    throw FormatException('$context has invalid enum backing values');
  }
  if (enumBackingValues is List<Object?> &&
      enumBackingValues.isNotEmpty &&
      enumValues is List<Object?> &&
      enumBackingValues.length != enumValues.length) {
    throw FormatException(
      '$context enum values disagree with their backing values',
    );
  }

  final defaultValue = raw['default'];
  final validDefault = switch (type) {
    'int' || 'enum' => defaultValue == null || defaultValue is int,
    'float' || 'double' =>
      defaultValue == null || (defaultValue is num && _finite(defaultValue)),
    'bool' => defaultValue == null || defaultValue is bool,
    'string' => defaultValue == null || defaultValue is String,
    _ => false,
  };
  if (!validDefault) {
    throw FormatException('$context has a default with the wrong type');
  }
  return FieldSchema.fromJson(raw);
}

bool _finite(num value) => value is! double || value.isFinite;

String _displayName(String id) {
  const prefixes = <String>[
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
