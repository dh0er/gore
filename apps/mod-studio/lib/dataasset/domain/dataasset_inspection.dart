const _format = 'gore.dataasset.fixed-inspect.v1';
const _selectorFormat = 1;
const _selectorProfile = 'g1r_ue5_4';
const _maxExports = 4096;
const _maxPackageExports = 4096;
const _maxLeavesPerExport = 10000;
const _maxTotalLeaves = 20000;
const _maxSelectorStepsPerLeaf = 128;
const _maxTotalSelectorSteps = 500000;
const _maxWireTypeDepth = 64;
const _maxTotalWireTypeNodes = 1000000;
const _maxTotalTextBytes = 8 * 1024 * 1024;
// Native bounds the complete compact response to 8 MiB, but intentionally has
// no narrower per-string wire cap (a resolved class path may exceed one legacy
// name). Keep the individual ceiling aligned with that closed aggregate.
const _maxTextBytes = _maxTotalTextBytes;
const _maxUassetBytes = 64 * 1024 * 1024;
const _maxUexpBytes = 256 * 1024 * 1024;
const _maxPackageBytes = 320 * 1024 * 1024;
const _maxUsmapBytes = 128 * 1024 * 1024;
const _maxJsonIndex = 0x7fffffff;
final _sha256Pattern = RegExp(r'^[0-9a-f]{64}$');
final _hexPattern = RegExp(r'^[0-9a-f]+$');

/// Immutable, strictly validated evidence returned by the offline DataAsset
/// inspector. This model intentionally exposes no mutation or deployment API.
final class DataAssetInspection {
  const DataAssetInspection._({
    required this.format,
    required this.status,
    required this.summary,
    required this.selectorFormat,
    required this.binding,
    required this.input,
    required this.selection,
    required this.exports,
  });

  final String format;
  final DataAssetInspectionStatus status;
  final DataAssetInspectionSummary summary;
  final DataAssetSelectorFormat selectorFormat;
  final DataAssetBinding binding;
  final DataAssetInputFacts input;
  final DataAssetSelection selection;
  final List<DataAssetExportReport> exports;

  factory DataAssetInspection.fromJson(Map<String, Object?> json) {
    final budget = _ParseBudget();
    _exact(json, const {
      'ok',
      'format',
      'status',
      'summary',
      'selector_format',
      'binding',
      'input',
      'selection',
      'exports',
    }, 'inspection response');
    if (json['ok'] != true || json['format'] != _format) {
      throw const FormatException('invalid DataAsset inspection envelope');
    }

    final status = DataAssetInspectionStatus.parse(json['status']);
    final summary = DataAssetInspectionSummary._parse(
      _object(json['summary'], 'summary'),
    );
    final selectorFormat = DataAssetSelectorFormat._parse(
      _object(json['selector_format'], 'selector format'),
    );
    final binding = DataAssetBinding._parse(
      _object(json['binding'], 'binding'),
    );
    final input = DataAssetInputFacts._parse(
      _object(json['input'], 'input facts'),
    );
    final selection = DataAssetSelection._parse(
      _object(json['selection'], 'selection'),
    );
    final rawExports = _list(json['exports'], 'exports');
    if (rawExports.length > _maxExports ||
        rawExports.length != summary.reportedExports) {
      throw const FormatException('invalid reported DataAsset export count');
    }
    if (selection.exportIndex == null) {
      if (summary.packageExports != summary.reportedExports) {
        throw const FormatException(
          'unfiltered DataAsset inspection omitted package exports',
        );
      }
    } else if (summary.reportedExports != 1 ||
        selection.exportIndex! >= summary.packageExports) {
      throw const FormatException('invalid DataAsset export selection');
    }

    final exports = <DataAssetExportReport>[];
    var walked = 0;
    var editable = 0;
    for (var position = 0; position < rawExports.length; position++) {
      final report = DataAssetExportReport._parse(
        _object(rawExports[position], 'export'),
        budget: budget,
        binding: binding,
        input: input,
      );
      final expectedIndex = selection.exportIndex ?? position;
      if (report.index != expectedIndex) {
        throw const FormatException('DataAsset export index is inconsistent');
      }
      if (report.status == DataAssetInspectionStatus.walked) walked++;
      editable += report.leaves.where((leaf) => leaf.editable).length;
      exports.add(report);
    }
    if (walked != summary.walkedExports || editable != summary.editableLeaves) {
      throw const FormatException('DataAsset summary does not match exports');
    }
    final expectedStatus = walked == 0
        ? DataAssetInspectionStatus.unsupported
        : walked == exports.length
        ? DataAssetInspectionStatus.walked
        : DataAssetInspectionStatus.partial;
    if (status != expectedStatus) {
      throw const FormatException('DataAsset aggregate status is inconsistent');
    }

    return DataAssetInspection._(
      format: _format,
      status: status,
      summary: summary,
      selectorFormat: selectorFormat,
      binding: binding,
      input: input,
      selection: selection,
      exports: List.unmodifiable(exports),
    );
  }
}

enum DataAssetInspectionStatus {
  walked('walked'),
  partial('partial'),
  unsupported('unsupported');

  const DataAssetInspectionStatus(this.wireName);
  final String wireName;

  static DataAssetInspectionStatus parse(Object? value) => switch (value) {
    'walked' => walked,
    'partial' => partial,
    'unsupported' => unsupported,
    _ => throw const FormatException('unknown DataAsset inspection status'),
  };
}

final class DataAssetInspectionSummary {
  const DataAssetInspectionSummary._({
    required this.packageExports,
    required this.reportedExports,
    required this.walkedExports,
    required this.editableLeaves,
  });

  final int packageExports;
  final int reportedExports;
  final int walkedExports;
  final int editableLeaves;

  factory DataAssetInspectionSummary._parse(Map<String, Object?> json) {
    _exact(json, const {
      'package_exports',
      'reported_exports',
      'walked_exports',
      'editable_leaves',
    }, 'summary');
    final packageExports = _integer(
      json,
      'package_exports',
      max: _maxPackageExports,
    );
    final reportedExports = _integer(
      json,
      'reported_exports',
      max: _maxExports,
    );
    final walkedExports = _integer(
      json,
      'walked_exports',
      max: reportedExports,
    );
    final editableLeaves = _integer(
      json,
      'editable_leaves',
      max: _maxTotalLeaves,
    );
    if (reportedExports > packageExports) {
      throw const FormatException('reported exports exceed package exports');
    }
    return DataAssetInspectionSummary._(
      packageExports: packageExports,
      reportedExports: reportedExports,
      walkedExports: walkedExports,
      editableLeaves: editableLeaves,
    );
  }
}

final class DataAssetSelectorFormat {
  const DataAssetSelectorFormat._({
    required this.format,
    required this.profile,
  });

  final int format;
  final String profile;

  factory DataAssetSelectorFormat._parse(Map<String, Object?> json) {
    _exact(json, const {'format', 'profile'}, 'selector format');
    if (json['format'] != _selectorFormat ||
        json['profile'] != _selectorProfile) {
      throw const FormatException('unsupported DataAsset selector format');
    }
    return const DataAssetSelectorFormat._(
      format: _selectorFormat,
      profile: _selectorProfile,
    );
  }
}

final class DataAssetPackageSeal {
  const DataAssetPackageSeal._({
    required this.uassetSha256,
    required this.uexpSha256,
  });

  final String uassetSha256;
  final String uexpSha256;

  Map<String, Object> toJson() => <String, Object>{
    'uasset_sha256': uassetSha256,
    'uexp_sha256': uexpSha256,
  };

  factory DataAssetPackageSeal._parse(Map<String, Object?> json) {
    _exact(json, const {'uasset_sha256', 'uexp_sha256'}, 'package seal');
    return DataAssetPackageSeal._(
      uassetSha256: _sha256(json, 'uasset_sha256'),
      uexpSha256: _sha256(json, 'uexp_sha256'),
    );
  }
}

final class DataAssetBinding {
  const DataAssetBinding._({
    required this.packageSeal,
    required this.usmapSha256,
  });

  final DataAssetPackageSeal packageSeal;
  final String usmapSha256;

  factory DataAssetBinding._parse(Map<String, Object?> json) {
    _exact(json, const {'package_seal', 'usmap_sha256'}, 'binding');
    return DataAssetBinding._(
      packageSeal: DataAssetPackageSeal._parse(
        _object(json['package_seal'], 'package seal'),
      ),
      usmapSha256: _sha256(json, 'usmap_sha256'),
    );
  }
}

final class DataAssetInputFacts {
  const DataAssetInputFacts._({
    required this.uassetLength,
    required this.uexpLength,
    required this.usmapLength,
  });

  final int uassetLength;
  final int uexpLength;
  final int usmapLength;

  factory DataAssetInputFacts._parse(Map<String, Object?> json) {
    _exact(json, const {
      'uasset_length',
      'uexp_length',
      'usmap_length',
    }, 'input facts');
    final uassetLength = _integer(
      json,
      'uasset_length',
      min: 1,
      max: _maxUassetBytes,
    );
    final uexpLength = _integer(
      json,
      'uexp_length',
      min: 1,
      max: _maxUexpBytes,
    );
    if (uassetLength + uexpLength > _maxPackageBytes) {
      throw const FormatException('DataAsset package size exceeds limit');
    }
    return DataAssetInputFacts._(
      uassetLength: uassetLength,
      uexpLength: uexpLength,
      usmapLength: _integer(json, 'usmap_length', min: 1, max: _maxUsmapBytes),
    );
  }
}

final class DataAssetSelection {
  const DataAssetSelection._({required this.exportIndex});
  final int? exportIndex;

  factory DataAssetSelection._parse(Map<String, Object?> json) {
    _exact(json, const {'export_index'}, 'selection');
    final value = json['export_index'];
    if (value != null &&
        (value is! int || value < 0 || value > _maxJsonIndex)) {
      throw const FormatException('invalid selected DataAsset export index');
    }
    return DataAssetSelection._(exportIndex: value as int?);
  }
}

final class DataAssetExportFailure {
  const DataAssetExportFailure._({required this.stage, required this.code});
  final String stage;
  final String code;

  factory DataAssetExportFailure._parse(Map<String, Object?> json) {
    _exact(json, const {'stage', 'code'}, 'export failure');
    final stage = json['stage'];
    final code = json['code'];
    final valid = switch ((stage, code)) {
      ('schema', 'schema_unsupported') => true,
      ('walk', 'property_stream_unsupported') => true,
      ('selector', 'selector_receipt_unsupported') => true,
      _ => false,
    };
    if (!valid) {
      throw const FormatException('unknown DataAsset export failure');
    }
    return DataAssetExportFailure._(
      stage: stage as String,
      code: code as String,
    );
  }
}

final class DataAssetExportReport {
  const DataAssetExportReport._({
    required this.index,
    required this.objectName,
    required this.classPath,
    required this.component,
    required this.length,
    required this.status,
    required this.failure,
    required this.schema,
    required this.propertyBytes,
    required this.nativeSuffixBytes,
    required this.leaves,
  });

  final int index;
  final String objectName;
  final String classPath;
  final String component;
  final int length;
  final DataAssetInspectionStatus status;
  final DataAssetExportFailure? failure;
  final String? schema;
  final int? propertyBytes;
  final int? nativeSuffixBytes;
  final List<DataAssetLeafReport> leaves;

  factory DataAssetExportReport._parse(
    Map<String, Object?> json, {
    required _ParseBudget budget,
    required DataAssetBinding binding,
    required DataAssetInputFacts input,
  }) {
    _exact(json, const {
      'index',
      'object_name',
      'class_path',
      'component',
      'length',
      'status',
      'failure',
      'schema',
      'property_bytes',
      'native_suffix_bytes',
      'leaves',
    }, 'export');
    final index = _integer(json, 'index', max: _maxJsonIndex);
    final objectName = budget.text(json['object_name'], 'object name');
    final classPath = budget.text(json['class_path'], 'class path');
    if (objectName.isEmpty ||
        classPath.isEmpty ||
        json['component'] != 'uexp') {
      throw const FormatException('invalid DataAsset export identity');
    }
    final length = _integer(json, 'length', max: input.uexpLength);
    final status = DataAssetInspectionStatus.parse(json['status']);
    if (status == DataAssetInspectionStatus.partial) {
      throw const FormatException('an individual export cannot be partial');
    }
    final rawFailure = json['failure'];
    final failure = rawFailure == null
        ? null
        : DataAssetExportFailure._parse(_object(rawFailure, 'export failure'));
    final schema = json['schema'] == null
        ? null
        : budget.text(json['schema'], 'schema');
    final propertyBytes = _nullableInteger(json, 'property_bytes', max: length);
    final nativeSuffixBytes = _nullableInteger(
      json,
      'native_suffix_bytes',
      max: length,
    );
    final rawLeaves = _list(json['leaves'], 'leaves');
    if (rawLeaves.length > _maxLeavesPerExport) {
      throw const FormatException('too many DataAsset leaves in export');
    }
    budget.addLeaves(rawLeaves.length);

    if (status == DataAssetInspectionStatus.walked) {
      if (failure != null ||
          schema == null ||
          schema.isEmpty ||
          propertyBytes == null ||
          nativeSuffixBytes == null ||
          propertyBytes + nativeSuffixBytes != length) {
        throw const FormatException('invalid walked DataAsset export facts');
      }
    } else if (failure == null ||
        schema != null ||
        propertyBytes != null ||
        nativeSuffixBytes != null ||
        rawLeaves.isNotEmpty) {
      throw const FormatException('invalid unsupported DataAsset export facts');
    }

    final leaves = <DataAssetLeafReport>[];
    for (var position = 0; position < rawLeaves.length; position++) {
      final leaf = DataAssetLeafReport._parse(
        _object(rawLeaves[position], 'leaf'),
        budget: budget,
        binding: binding,
        exportIndex: index,
        objectName: objectName,
        classPath: classPath,
      );
      if (leaf.index != position) {
        throw const FormatException('DataAsset leaf index is inconsistent');
      }
      leaves.add(leaf);
    }
    return DataAssetExportReport._(
      index: index,
      objectName: objectName,
      classPath: classPath,
      component: 'uexp',
      length: length,
      status: status,
      failure: failure,
      schema: schema,
      propertyBytes: propertyBytes,
      nativeSuffixBytes: nativeSuffixBytes,
      leaves: List.unmodifiable(leaves),
    );
  }
}

final class DataAssetLeafReport {
  const DataAssetLeafReport._({
    required this.index,
    required this.editable,
    required this.selector,
  });

  final int index;
  final bool editable;
  final FixedLeafSelector selector;

  factory DataAssetLeafReport._parse(
    Map<String, Object?> json, {
    required _ParseBudget budget,
    required DataAssetBinding binding,
    required int exportIndex,
    required String objectName,
    required String classPath,
  }) {
    _exact(json, const {'index', 'editable', 'selector'}, 'leaf');
    final editable = json['editable'];
    if (editable is! bool) {
      throw const FormatException('DataAsset leaf editable is not a bool');
    }
    final selector = FixedLeafSelector._parse(
      _object(json['selector'], 'fixed-leaf selector'),
      budget: budget,
    );
    if (selector.exportIndex != exportIndex ||
        selector.objectName != objectName ||
        selector.classPath != classPath ||
        selector.packageSeal.uassetSha256 != binding.packageSeal.uassetSha256 ||
        selector.packageSeal.uexpSha256 != binding.packageSeal.uexpSha256 ||
        selector.usmapSha256 != binding.usmapSha256) {
      throw const FormatException('DataAsset selector binding is inconsistent');
    }
    if (editable &&
        (selector.role != FixedLeafRole.propertyValue ||
            !selector.kind.isEditable ||
            selector.path.any(
              (step) =>
                  step.kind == FixedLeafStepKind.mapEntryValue &&
                  step.key?.kind == null,
            ))) {
      throw const FormatException('DataAsset leaf editability is inconsistent');
    }
    return DataAssetLeafReport._(
      index: _integer(json, 'index', max: _maxLeavesPerExport - 1),
      editable: editable,
      selector: selector,
    );
  }
}

enum FixedLeafRole {
  propertyValue('property_value'),
  mapKey('map_key'),
  removedMapKey('removed_map_key');

  const FixedLeafRole(this.wireName);
  final String wireName;

  static FixedLeafRole parse(Object? value) => switch (value) {
    'property_value' => propertyValue,
    'map_key' => mapKey,
    'removed_map_key' => removedMapKey,
    _ => throw const FormatException('unknown fixed-leaf role'),
  };
}

enum FixedWireKind {
  byte('byte', 1, true),
  boolean('bool', 1, true),
  int32('int32', 4, true),
  float32('float32', 4, true),
  packageIndex('package_index', 4, false),
  fname('fname', 8, false),
  float64('float64', 8, true),
  uint64('uint64', 8, true),
  uint32('uint32', 4, true),
  uint16('uint16', 2, true),
  int64('int64', 8, true),
  int16('int16', 2, true),
  int8('int8', 1, true),
  linearColorF32x4('linear_color_f32x4', 16, true),
  vector4F64x4('vector4_f64x4', 32, true);

  const FixedWireKind(this.wireName, this.width, this.isEditable);
  final String wireName;
  final int width;
  final bool isEditable;

  static FixedWireKind parse(Object? value) {
    for (final kind in values) {
      if (kind.wireName == value) return kind;
    }
    throw const FormatException('unknown fixed wire kind');
  }
}

final class FixedLeafSelector {
  const FixedLeafSelector._({
    required this.format,
    required this.profile,
    required this.packageSeal,
    required this.usmapSha256,
    required this.exportIndex,
    required this.objectName,
    required this.classPath,
    required this.component,
    required this.exportSha256,
    required this.role,
    required this.kind,
    required this.path,
    required this.expectedHex,
  });

  final int format;
  final String profile;
  final DataAssetPackageSeal packageSeal;
  final String usmapSha256;
  final int exportIndex;
  final String objectName;
  final String classPath;
  final String component;
  final String exportSha256;
  final FixedLeafRole role;
  final FixedWireKind kind;
  final List<FixedLeafSelectorStep> path;
  final String expectedHex;

  String get pathLabel => path.map((step) => step.label).join(' / ');

  /// Re-emits the exact closed native selector schema after strict parsing.
  /// Semantic authoring must use this canonical projection instead of
  /// duplicating the selector wire in individual editors.
  Map<String, Object?> toJson() => <String, Object?>{
    'format': format,
    'profile': profile,
    'package_seal': packageSeal.toJson(),
    'usmap_sha256': usmapSha256,
    'export_index': exportIndex,
    'object_name': objectName,
    'class_path': classPath,
    'component': component,
    'export_sha256': exportSha256,
    'role': role.wireName,
    'kind': kind.wireName,
    'path': path.map((step) => step.toJson()).toList(growable: false),
    'expected_hex': expectedHex,
  };

  factory FixedLeafSelector._parse(
    Map<String, Object?> json, {
    required _ParseBudget budget,
  }) {
    _exact(json, const {
      'format',
      'profile',
      'package_seal',
      'usmap_sha256',
      'export_index',
      'object_name',
      'class_path',
      'component',
      'export_sha256',
      'role',
      'kind',
      'path',
      'expected_hex',
    }, 'fixed-leaf selector');
    if (json['format'] != _selectorFormat ||
        json['profile'] != _selectorProfile ||
        json['component'] != 'uexp') {
      throw const FormatException('unsupported fixed-leaf selector identity');
    }
    final kind = FixedWireKind.parse(json['kind']);
    final expectedHex = json['expected_hex'];
    if (expectedHex is! String ||
        expectedHex.length != kind.width * 2 ||
        !_hexPattern.hasMatch(expectedHex)) {
      throw const FormatException('invalid fixed-leaf expected bytes');
    }
    if (kind == FixedWireKind.boolean &&
        expectedHex != '00' &&
        expectedHex != '01') {
      throw const FormatException('invalid fixed-leaf bool byte');
    }
    final rawPath = _list(json['path'], 'fixed-leaf path');
    if (rawPath.isEmpty || rawPath.length > _maxSelectorStepsPerLeaf) {
      throw const FormatException('invalid fixed-leaf selector path length');
    }
    budget.addSelectorSteps(rawPath.length);
    final path = rawPath
        .map(
          (step) => FixedLeafSelectorStep._parse(
            _object(step, 'fixed-leaf selector step'),
            budget: budget,
          ),
        )
        .toList(growable: false);
    return FixedLeafSelector._(
      format: _selectorFormat,
      profile: _selectorProfile,
      packageSeal: DataAssetPackageSeal._parse(
        _object(json['package_seal'], 'selector package seal'),
      ),
      usmapSha256: _sha256(json, 'usmap_sha256'),
      exportIndex: _integer(json, 'export_index', max: _maxJsonIndex),
      objectName: budget.nonEmptyText(json['object_name'], 'selector object'),
      classPath: budget.nonEmptyText(json['class_path'], 'selector class'),
      component: 'uexp',
      exportSha256: _sha256(json, 'export_sha256'),
      role: FixedLeafRole.parse(json['role']),
      kind: kind,
      path: List.unmodifiable(path),
      expectedHex: expectedHex,
    );
  }
}

enum FixedLeafStepKind {
  property,
  structure,
  map,
  mapEntryValue,
  mapEntryKey,
  removedMapKey,
}

final class FixedLeafSelectorStep {
  const FixedLeafSelectorStep._({
    required this.kind,
    this.schemaIndex,
    this.propertyName,
    this.arrayIndex,
    this.arrayDimension,
    this.declaringSchemaName,
    this.declaringModulePath,
    this.propertyType,
    this.name,
    this.schemaName,
    this.keyType,
    this.valueType,
    this.key,
  });

  final FixedLeafStepKind kind;
  final int? schemaIndex;
  final String? propertyName;
  final int? arrayIndex;
  final int? arrayDimension;
  final String? declaringSchemaName;
  final String? declaringModulePath;
  final FixedLeafWireType? propertyType;
  final String? name;
  final String? schemaName;
  final FixedLeafWireType? keyType;
  final FixedLeafWireType? valueType;
  final FixedLeafMapKeyIdentity? key;

  String get label => switch (kind) {
    FixedLeafStepKind.property => propertyName!,
    FixedLeafStepKind.structure => name!,
    FixedLeafStepKind.map => 'map',
    FixedLeafStepKind.mapEntryValue => 'map value ${key!.shortHash}',
    FixedLeafStepKind.mapEntryKey => 'map key ${key!.shortHash}',
    FixedLeafStepKind.removedMapKey => 'removed key ${key!.shortHash}',
  };

  Map<String, Object?> toJson() => switch (kind) {
    FixedLeafStepKind.property => <String, Object?>{
      'step': 'property',
      'schema_index': schemaIndex,
      'property_name': propertyName,
      'array_index': arrayIndex,
      'array_dimension': arrayDimension,
      'declaring_schema_name': declaringSchemaName,
      'declaring_module_path': declaringModulePath,
      'property_type': propertyType!.toJson(),
    },
    FixedLeafStepKind.structure => <String, Object?>{
      'step': 'struct',
      'name': name,
      'schema_name': schemaName,
    },
    FixedLeafStepKind.map => <String, Object?>{
      'step': 'map',
      'key_type': keyType!.toJson(),
      'value_type': valueType!.toJson(),
    },
    FixedLeafStepKind.mapEntryValue => <String, Object?>{
      'step': 'map_entry_value',
      'key': key!.toJson(),
    },
    FixedLeafStepKind.mapEntryKey => <String, Object?>{
      'step': 'map_entry_key',
      'key': key!.toJson(),
    },
    FixedLeafStepKind.removedMapKey => <String, Object?>{
      'step': 'removed_map_key',
      'key': key!.toJson(),
    },
  };

  factory FixedLeafSelectorStep._parse(
    Map<String, Object?> json, {
    required _ParseBudget budget,
  }) {
    switch (json['step']) {
      case 'property':
        _exact(json, const {
          'step',
          'schema_index',
          'property_name',
          'array_index',
          'array_dimension',
          'declaring_schema_name',
          'declaring_module_path',
          'property_type',
        }, 'property selector step');
        final arrayDimension = _integer(
          json,
          'array_dimension',
          min: 1,
          max: _maxJsonIndex,
        );
        final arrayIndex = _integer(json, 'array_index', max: _maxJsonIndex);
        if (arrayIndex >= arrayDimension) {
          throw const FormatException('invalid fixed-leaf array index');
        }
        final rawModule = json['declaring_module_path'];
        final module = rawModule == null
            ? null
            : budget.nonEmptyText(rawModule, 'declaring module path');
        return FixedLeafSelectorStep._(
          kind: FixedLeafStepKind.property,
          schemaIndex: _integer(json, 'schema_index', max: _maxJsonIndex),
          propertyName: budget.nonEmptyText(
            json['property_name'],
            'property name',
          ),
          arrayIndex: arrayIndex,
          arrayDimension: arrayDimension,
          declaringSchemaName: budget.nonEmptyText(
            json['declaring_schema_name'],
            'declaring schema name',
          ),
          declaringModulePath: module,
          propertyType: FixedLeafWireType._parse(
            _object(json['property_type'], 'property wire type'),
            budget: budget,
          ),
        );
      case 'struct':
        _exact(json, const {'step', 'name', 'schema_name'}, 'struct step');
        return FixedLeafSelectorStep._(
          kind: FixedLeafStepKind.structure,
          name: budget.nonEmptyText(json['name'], 'struct name'),
          schemaName: budget.nonEmptyText(json['schema_name'], 'struct schema'),
        );
      case 'map':
        _exact(json, const {'step', 'key_type', 'value_type'}, 'map step');
        return FixedLeafSelectorStep._(
          kind: FixedLeafStepKind.map,
          keyType: FixedLeafWireType._parse(
            _object(json['key_type'], 'map key wire type'),
            budget: budget,
          ),
          valueType: FixedLeafWireType._parse(
            _object(json['value_type'], 'map value wire type'),
            budget: budget,
          ),
        );
      case 'map_entry_value':
      case 'map_entry_key':
      case 'removed_map_key':
        _exact(json, const {'step', 'key'}, 'map-key selector step');
        final kind = switch (json['step']) {
          'map_entry_value' => FixedLeafStepKind.mapEntryValue,
          'map_entry_key' => FixedLeafStepKind.mapEntryKey,
          _ => FixedLeafStepKind.removedMapKey,
        };
        return FixedLeafSelectorStep._(
          kind: kind,
          key: FixedLeafMapKeyIdentity._parse(
            _object(json['key'], 'map key identity'),
          ),
        );
      default:
        throw const FormatException('unknown fixed-leaf selector step');
    }
  }
}

final class FixedLeafMapKeyIdentity {
  const FixedLeafMapKeyIdentity._({
    required this.kind,
    required this.byteLength,
    required this.sha256,
  });

  final FixedWireKind? kind;
  final int byteLength;
  final String sha256;
  String get shortHash => sha256.substring(0, 8);

  Map<String, Object?> toJson() => <String, Object?>{
    'kind': kind?.wireName,
    'byte_length': byteLength,
    'sha256': sha256,
  };

  factory FixedLeafMapKeyIdentity._parse(Map<String, Object?> json) {
    _exact(json, const {'kind', 'byte_length', 'sha256'}, 'map key identity');
    final rawKind = json['kind'];
    final kind = rawKind == null ? null : FixedWireKind.parse(rawKind);
    final byteLength = _integer(json, 'byte_length', max: _maxUexpBytes);
    if (kind != null && byteLength != kind.width) {
      throw const FormatException('fixed map key width is inconsistent');
    }
    return FixedLeafMapKeyIdentity._(
      kind: kind,
      byteLength: byteLength,
      sha256: _sha256(json, 'sha256'),
    );
  }
}

enum FixedLeafWireTypeKind {
  byte('byte'),
  boolean('bool'),
  integer('int'),
  float('float'),
  object('object'),
  name('name'),
  delegate('delegate'),
  doublePrecision('double'),
  array('array'),
  structure('struct'),
  string('string'),
  text('text'),
  interface('interface'),
  multicastDelegate('multicast_delegate'),
  weakObject('weak_object'),
  lazyObject('lazy_object'),
  assetObject('asset_object'),
  softObject('soft_object'),
  uint64('uint64'),
  uint32('uint32'),
  uint16('uint16'),
  int64('int64'),
  int16('int16'),
  int8('int8'),
  map('map'),
  set('set'),
  enumeration('enum'),
  fieldPath('field_path'),
  optional('optional'),
  utf8String('utf8_string'),
  ansiString('ansi_string'),
  unknown('unknown');

  const FixedLeafWireTypeKind(this.wireName);
  final String wireName;
}

final class FixedLeafWireType {
  const FixedLeafWireType._({
    required this.kind,
    this.inner,
    this.key,
    this.value,
    this.name,
  });

  final FixedLeafWireTypeKind kind;
  final FixedLeafWireType? inner;
  final FixedLeafWireType? key;
  final FixedLeafWireType? value;
  final String? name;

  Map<String, Object?> toJson() => switch (kind) {
    FixedLeafWireTypeKind.array || FixedLeafWireTypeKind.optional =>
      <String, Object?>{'type': kind.wireName, 'inner': inner!.toJson()},
    FixedLeafWireTypeKind.structure => <String, Object?>{
      'type': kind.wireName,
      'name': name,
    },
    FixedLeafWireTypeKind.map => <String, Object?>{
      'type': kind.wireName,
      'key': key!.toJson(),
      'value': value!.toJson(),
    },
    FixedLeafWireTypeKind.set => <String, Object?>{
      'type': kind.wireName,
      'key': key!.toJson(),
    },
    FixedLeafWireTypeKind.enumeration => <String, Object?>{
      'type': kind.wireName,
      'inner': inner!.toJson(),
      'name': name,
    },
    _ => <String, Object?>{'type': kind.wireName},
  };

  factory FixedLeafWireType._parse(
    Map<String, Object?> json, {
    required _ParseBudget budget,
    int depth = 0,
  }) {
    if (depth >= _maxWireTypeDepth) {
      throw const FormatException('fixed wire type nesting is too deep');
    }
    budget.addWireTypeNode();
    final rawType = json['type'];
    FixedLeafWireType nested(Object? value, String context) =>
        FixedLeafWireType._parse(
          _object(value, context),
          budget: budget,
          depth: depth + 1,
        );
    for (final kind in FixedLeafWireTypeKind.values) {
      if (kind.wireName != rawType) continue;
      switch (kind) {
        case FixedLeafWireTypeKind.array:
        case FixedLeafWireTypeKind.optional:
          _exact(json, const {'type', 'inner'}, 'nested wire type');
          return FixedLeafWireType._(
            kind: kind,
            inner: nested(json['inner'], 'inner wire type'),
          );
        case FixedLeafWireTypeKind.structure:
          _exact(json, const {'type', 'name'}, 'struct wire type');
          return FixedLeafWireType._(
            kind: kind,
            name: budget.nonEmptyText(json['name'], 'wire struct name'),
          );
        case FixedLeafWireTypeKind.map:
          _exact(json, const {'type', 'key', 'value'}, 'map wire type');
          return FixedLeafWireType._(
            kind: kind,
            key: nested(json['key'], 'key wire type'),
            value: nested(json['value'], 'value wire type'),
          );
        case FixedLeafWireTypeKind.set:
          _exact(json, const {'type', 'key'}, 'set wire type');
          return FixedLeafWireType._(
            kind: kind,
            key: nested(json['key'], 'set key wire type'),
          );
        case FixedLeafWireTypeKind.enumeration:
          _exact(json, const {'type', 'inner', 'name'}, 'enum wire type');
          return FixedLeafWireType._(
            kind: kind,
            inner: nested(json['inner'], 'enum inner wire type'),
            name: budget.nonEmptyText(json['name'], 'wire enum name'),
          );
        default:
          _exact(json, const {'type'}, 'simple wire type');
          return FixedLeafWireType._(kind: kind);
      }
    }
    throw const FormatException('unknown fixed-leaf wire type');
  }
}

final class _ParseBudget {
  var _textBytes = 0;
  var _leaves = 0;
  var _selectorSteps = 0;
  var _wireTypeNodes = 0;

  String text(Object? value, String context) {
    if (value is! String) {
      throw FormatException('$context is not a string');
    }
    final length = _strictUtf8Length(value, _maxTextBytes);
    if (length == null || length > _maxTotalTextBytes - _textBytes) {
      throw FormatException('$context exceeds the bounded text budget');
    }
    _textBytes += length;
    return value;
  }

  String nonEmptyText(Object? value, String context) {
    final result = text(value, context);
    if (result.isEmpty) throw FormatException('$context is empty');
    return result;
  }

  void addLeaves(int count) {
    if (count > _maxTotalLeaves - _leaves) {
      throw const FormatException('too many DataAsset leaves');
    }
    _leaves += count;
  }

  void addSelectorSteps(int count) {
    if (count > _maxTotalSelectorSteps - _selectorSteps) {
      throw const FormatException('too many fixed-leaf selector steps');
    }
    _selectorSteps += count;
  }

  void addWireTypeNode() {
    if (_wireTypeNodes >= _maxTotalWireTypeNodes) {
      throw const FormatException('too many fixed wire type nodes');
    }
    _wireTypeNodes++;
  }
}

void _exact(Map<String, Object?> json, Set<String> fields, String context) {
  if (json.length != fields.length || !fields.every(json.containsKey)) {
    throw FormatException('$context has an invalid schema');
  }
}

Map<String, Object?> _object(Object? value, String context) {
  if (value is! Map) throw FormatException('$context is not an object');
  final result = <String, Object?>{};
  for (final entry in value.entries) {
    if (entry.key is! String) {
      throw FormatException('$context contains a non-string field');
    }
    result[entry.key as String] = entry.value;
  }
  return result;
}

List<Object?> _list(Object? value, String context) {
  if (value is! List) throw FormatException('$context is not an array');
  return value;
}

int _integer(
  Map<String, Object?> json,
  String field, {
  int min = 0,
  int max = _maxJsonIndex,
}) {
  final value = json[field];
  if (value is! int || value < min || value > max) {
    throw FormatException('$field is not an integer in the bounded range');
  }
  return value;
}

int? _nullableInteger(
  Map<String, Object?> json,
  String field, {
  int min = 0,
  int max = _maxJsonIndex,
}) {
  if (json[field] == null) return null;
  return _integer(json, field, min: min, max: max);
}

String _sha256(Map<String, Object?> json, String field) {
  final value = json[field];
  if (value is! String || !_sha256Pattern.hasMatch(value)) {
    throw FormatException('$field is not canonical SHA-256');
  }
  return value;
}

int? _strictUtf8Length(String value, int limit) {
  var length = 0;
  for (var index = 0; index < value.length; index++) {
    final unit = value.codeUnitAt(index);
    final int added;
    if (unit <= 0x7f) {
      added = 1;
    } else if (unit <= 0x7ff) {
      added = 2;
    } else if (unit >= 0xd800 && unit <= 0xdbff) {
      if (index + 1 >= value.length) return null;
      final low = value.codeUnitAt(++index);
      if (low < 0xdc00 || low > 0xdfff) return null;
      added = 4;
    } else if (unit >= 0xdc00 && unit <= 0xdfff) {
      return null;
    } else {
      added = 3;
    }
    if (added > limit - length) return null;
    length += added;
  }
  return length;
}
