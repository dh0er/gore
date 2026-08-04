import 'dart:convert';
import 'dart:typed_data';

import 'dataasset_inspection.dart';

const reviewedDataAssetEditRequestFormat = 1;
const footstepPresetSchemaId = 'g1r.tracking.footstep-preset';
const footstepPresetSchemaRevision = 1;
const feetTextureSizeFieldId = 'feet_texture_size';

/// One installed asset reviewed for a semantic schema.
///
/// Matching is deliberately based on the complete, case-sensitive package
/// path. A familiar basename in another directory must remain generic.
final class ReviewedDataAssetTarget {
  const ReviewedDataAssetTarget({
    required this.packagePath,
    required this.assetName,
    required this.friendlyName,
  });

  final String packagePath;
  final String assetName;
  final String friendlyName;
}

/// User-facing metadata for a reviewed semantic field.
final class ReviewedDataAssetField {
  const ReviewedDataAssetField({
    required this.id,
    required this.friendlyName,
    required this.componentNames,
  });

  final String id;
  final String friendlyName;
  final List<String> componentNames;
}

/// Closed registry entry for the first reviewed installed DataAsset schema.
final class ReviewedDataAssetSchema {
  const ReviewedDataAssetSchema({
    required this.id,
    required this.revision,
    required this.friendlyName,
    required this.fields,
    required this.targets,
  });

  final String id;
  final int revision;
  final String friendlyName;
  final List<ReviewedDataAssetField> fields;
  final List<ReviewedDataAssetTarget> targets;

  /// Returns `null` for every unknown or near-match target.
  ///
  /// Callers must keep that asset on the generic/read-only path.
  ReviewedDataAssetTarget? matchInstalledTarget(String packagePath) {
    for (final target in targets) {
      if (target.packagePath == packagePath) {
        return target;
      }
    }
    return null;
  }
}

const footstepPresetReviewedSchema = ReviewedDataAssetSchema(
  id: footstepPresetSchemaId,
  revision: footstepPresetSchemaRevision,
  friendlyName: 'Footstep preset',
  fields: <ReviewedDataAssetField>[
    ReviewedDataAssetField(
      id: feetTextureSizeFieldId,
      friendlyName: 'Feet texture size',
      componentNames: <String>['Width', 'Height'],
    ),
  ],
  targets: <ReviewedDataAssetTarget>[
    ReviewedDataAssetTarget(
      packagePath:
          '/Game/Blueprints/TrackingSystem/FootstepsPresets/DA_HumanFootsteps',
      assetName: 'DA_HumanFootsteps',
      friendlyName: 'Human footsteps',
    ),
    ReviewedDataAssetTarget(
      packagePath:
          '/Game/Blueprints/TrackingSystem/FootstepsPresets/DA_ScavengerFootsteps',
      assetName: 'DA_ScavengerFootsteps',
      friendlyName: 'Scavenger footsteps',
    ),
    ReviewedDataAssetTarget(
      packagePath:
          '/Game/Blueprints/TrackingSystem/FootstepsPresets/DA_WolfFootsteps',
      assetName: 'DA_WolfFootsteps',
      friendlyName: 'Wolf footsteps',
    ),
  ],
);

/// Exact reviewed view over an inspector-proven footstep preset leaf.
///
/// This matcher grants no authority from a familiar basename or approximate
/// selector. Unknown, ambiguous, duplicated, or non-finite evidence stays on
/// the generic/read-only path.
final class ReviewedFootstepPresetInspection {
  const ReviewedFootstepPresetInspection._({
    required this.target,
    required this.leaf,
    required this.currentX,
    required this.currentY,
    required this.currentZ,
    required this.currentW,
  });

  static ReviewedFootstepPresetInspection? tryMatch({
    required String packagePath,
    required DataAssetInspection inspection,
  }) {
    final target = footstepPresetReviewedSchema.matchInstalledTarget(
      packagePath,
    );
    if (target == null) return null;

    final leaves = inspection.exports
        .expand((export) => export.leaves)
        .toList(growable: false);
    final matchingLeaves = leaves
        .where((leaf) => _matchesFootstepSelector(leaf.selector, target))
        .toList(growable: false);
    if (matchingLeaves.length != 1 || !matchingLeaves.single.editable) {
      return null;
    }

    final leaf = matchingLeaves.single;
    final values = _decodeFiniteVector4(leaf.selector.expectedHex);
    if (values == null) return null;
    return ReviewedFootstepPresetInspection._(
      target: target,
      leaf: leaf,
      currentX: values[0].toString(),
      currentY: values[1].toString(),
      currentZ: values[2].toString(),
      currentW: values[3].toString(),
    );
  }

  final ReviewedDataAssetTarget target;
  final DataAssetLeafReport leaf;
  final String currentX;
  final String currentY;
  final String currentZ;
  final String currentW;

  List<String> get currentComponents => <String>[
    currentX,
    currentY,
    currentZ,
    currentW,
  ];
}

bool _matchesFootstepSelector(
  FixedLeafSelector selector,
  ReviewedDataAssetTarget target,
) =>
    selector.format == 1 &&
    selector.profile == 'g1r_ue5_4' &&
    selector.exportIndex == 0 &&
    selector.objectName == target.assetName &&
    selector.classPath == '/Script/G1R.FootstepTag' &&
    selector.component == 'uexp' &&
    selector.role == FixedLeafRole.propertyValue &&
    selector.kind == FixedWireKind.vector4F64x4 &&
    _matchesFootstepPath(selector.path);

bool _matchesFootstepPath(List<FixedLeafSelectorStep> path) {
  if (path.length != 3) return false;
  final boneData = path[0];
  final nested = path[1];
  final textureSize = path[2];
  return _matchesPropertyStep(
        boneData,
        propertyName: 'BoneData',
        declaringSchemaName: 'FootstepTag',
        declaringModulePath: '/Script/G1R',
        structName: 'BoneFeetData',
      ) &&
      nested.kind == FixedLeafStepKind.structure &&
      nested.name == 'BoneFeetData' &&
      nested.schemaName == '/Script/G1R.BoneFeetData' &&
      _matchesPropertyStep(
        textureSize,
        propertyName: 'FeetTextureSize',
        declaringSchemaName: 'BoneFeetData',
        declaringModulePath: '/Script/G1R',
        structName: 'Vector4',
      );
}

bool _matchesPropertyStep(
  FixedLeafSelectorStep step, {
  required String propertyName,
  required String declaringSchemaName,
  required String declaringModulePath,
  required String structName,
}) =>
    step.kind == FixedLeafStepKind.property &&
    step.schemaIndex == 0 &&
    step.propertyName == propertyName &&
    step.arrayIndex == 0 &&
    step.arrayDimension == 1 &&
    step.declaringSchemaName == declaringSchemaName &&
    step.declaringModulePath == declaringModulePath &&
    step.propertyType?.kind == FixedLeafWireTypeKind.structure &&
    step.propertyType?.name == structName;

List<double>? _decodeFiniteVector4(String expectedHex) {
  if (expectedHex.length != 64) return null;
  final bytes = Uint8List(32);
  for (var index = 0; index < bytes.length; index++) {
    final byte = int.tryParse(
      expectedHex.substring(index * 2, index * 2 + 2),
      radix: 16,
    );
    if (byte == null) return null;
    bytes[index] = byte;
  }
  final data = ByteData.sublistView(bytes);
  final values = List<double>.generate(
    4,
    (index) => data.getFloat64(index * 8, Endian.little),
    growable: false,
  );
  return values.every((value) => value.isFinite) ? values : null;
}

/// Offset-free semantic request for the reviewed footstep field.
///
/// Target paths, selectors, replacement bytes, and intent bindings are not
/// part of this DTO. Native code remains authoritative for resolving those
/// facts and for binding the final prepared edit.
final class ReviewedDataAssetEditRequest {
  ReviewedDataAssetEditRequest._({required this.x, required this.y});

  factory ReviewedDataAssetEditRequest.feetTextureSize({
    required String x,
    required String y,
  }) => ReviewedDataAssetEditRequest._(
    x: _canonicalPositiveDecimal(x, component: 'x'),
    y: _canonicalPositiveDecimal(y, component: 'y'),
  );

  factory ReviewedDataAssetEditRequest.fromJson(Map<String, Object?> json) {
    const fields = <String>{
      'format',
      'schema_id',
      'schema_revision',
      'field_id',
      'value',
    };
    if (!_hasExactFields(json, fields) ||
        json['format'] != reviewedDataAssetEditRequestFormat ||
        json['schema_id'] != footstepPresetSchemaId ||
        json['schema_revision'] != footstepPresetSchemaRevision ||
        json['field_id'] != feetTextureSizeFieldId) {
      throw const FormatException('invalid reviewed DataAsset edit request');
    }

    final value = json['value'];
    if (value is! Map) {
      throw const FormatException('invalid feet texture size value');
    }
    final typedValue = <String, Object?>{};
    for (final entry in value.entries) {
      final key = entry.key;
      if (key is! String) {
        throw const FormatException('invalid feet texture size value');
      }
      typedValue[key] = entry.value;
    }
    if (!_hasExactFields(typedValue, const <String>{'x', 'y'})) {
      throw const FormatException('invalid feet texture size value');
    }
    final x = typedValue['x'];
    final y = typedValue['y'];
    if (x is! String || y is! String) {
      throw const FormatException('invalid feet texture size value');
    }

    final request = ReviewedDataAssetEditRequest.feetTextureSize(x: x, y: y);
    if (request.x != x || request.y != y) {
      throw const FormatException('noncanonical feet texture size value');
    }
    return request;
  }

  factory ReviewedDataAssetEditRequest.fromCanonicalJson(String source) {
    if (source.isEmpty || utf8.encode(source).length > 2048) {
      throw const FormatException('invalid reviewed DataAsset request size');
    }

    final Object? decoded;
    try {
      decoded = jsonDecode(source);
    } on FormatException {
      throw const FormatException('invalid reviewed DataAsset request JSON');
    }
    if (decoded is! Map) {
      throw const FormatException('invalid reviewed DataAsset request JSON');
    }
    final typedJson = <String, Object?>{};
    for (final entry in decoded.entries) {
      final key = entry.key;
      if (key is! String) {
        throw const FormatException('invalid reviewed DataAsset request JSON');
      }
      typedJson[key] = entry.value;
    }

    final request = ReviewedDataAssetEditRequest.fromJson(typedJson);
    if (request.canonicalJson != source) {
      throw const FormatException('noncanonical reviewed DataAsset request');
    }
    return request;
  }

  final String x;
  final String y;

  Map<String, Object> toJson() => <String, Object>{
    'format': reviewedDataAssetEditRequestFormat,
    'schema_id': footstepPresetSchemaId,
    'schema_revision': footstepPresetSchemaRevision,
    'field_id': feetTextureSizeFieldId,
    'value': <String, String>{'x': x, 'y': y},
  };

  String get canonicalJson => jsonEncode(toJson());
}

final _positiveDecimal = RegExp(r'^(?:0|[1-9][0-9]*)(?:\.[0-9]+)?$');

String _canonicalPositiveDecimal(String source, {required String component}) {
  if (source.isEmpty ||
      source.length > 64 ||
      !_positiveDecimal.hasMatch(source)) {
    throw FormatException('invalid positive decimal for $component');
  }
  final parsed = double.tryParse(source);
  if (parsed == null || !parsed.isFinite || parsed <= 0) {
    throw FormatException('invalid positive decimal for $component');
  }

  final dot = source.indexOf('.');
  if (dot < 0) {
    return source;
  }
  final integer = source.substring(0, dot);
  var fraction = source.substring(dot + 1);
  while (fraction.endsWith('0')) {
    fraction = fraction.substring(0, fraction.length - 1);
  }
  return fraction.isEmpty ? integer : '$integer.$fraction';
}

bool _hasExactFields(Map<String, Object?> json, Set<String> fields) =>
    json.length == fields.length && json.keys.every(fields.contains);
