import 'dart:convert';
import 'dart:typed_data';

import 'package:crypto/crypto.dart' as crypto;

import 'dataasset_inspection.dart';

/// One user-facing, typed replacement for an already verified fixed leaf.
///
/// The public wire contains semantic values only. Raw offsets and replacement
/// bytes never cross the UI boundary; native code performs the authoritative
/// little-endian encoding and checks it against the selector kind and width.
final class DataAssetSemanticReplacement {
  DataAssetSemanticReplacement._({
    required this.kind,
    required this.displayValue,
    required Map<String, Object> wire,
    required Uint8List comparisonBytes,
  }) : _wire = Map.unmodifiable(wire),
       _comparisonBytes = Uint8List.fromList(comparisonBytes);

  final FixedWireKind kind;
  final String displayValue;
  final Map<String, Object> _wire;
  final Uint8List _comparisonBytes;

  Map<String, Object> toJson() => Map<String, Object>.from(_wire);

  /// Compute the native edit binding without exposing raw replacement bytes
  /// to either source-authority wire format.
  String intentBindingSha256For({
    required String expectedTargetPath,
    required FixedLeafSelector selector,
  }) {
    final bytes = BytesBuilder(copy: false)
      ..add(
        utf8.encode('gore.authoring.r3-dataasset-edit.intent-binding.v1\u0000'),
      );
    _addLengthPrefixed(bytes, utf8.encode(expectedTargetPath));
    _addLengthPrefixed(bytes, utf8.encode(jsonEncode(selector.toJson())));
    _addLengthPrefixed(bytes, _comparisonBytes);
    return crypto.sha256.convert(bytes.takeBytes()).toString();
  }
}

final class DataAssetSemanticEditIntent {
  DataAssetSemanticEditIntent._({
    required this.extractReceiptPath,
    required this.expectedTargetPath,
    required this.selector,
    required this.replacement,
  });

  final String extractReceiptPath;
  final String expectedTargetPath;
  final FixedLeafSelector selector;
  final DataAssetSemanticReplacement replacement;

  Map<String, Object?> toNativeFields() => <String, Object?>{
    'extract_receipt_path': extractReceiptPath,
    'expected_target_path': expectedTargetPath,
    'selector': selector.toJson(),
    'replacement': replacement.toJson(),
  };

  /// Domain-separated exact request binding checked against the native
  /// prepare-only response. It covers the confirmed target, the complete
  /// offset-free selector, and the exact native replacement bytes.
  String get intentBindingSha256 => replacement.intentBindingSha256For(
    expectedTargetPath: expectedTargetPath,
    selector: selector,
  );
}

/// Native-verified, read-only identity of one exact ExtractReceipt-v2.
///
/// The target is deliberately surfaced before authoring. Package and USMAP
/// facts must match the inspection, while the target path may still differ for
/// byte-identical cooked packages and therefore requires explicit confirmation.
final class DataAssetExtractReceiptSummary {
  DataAssetExtractReceiptSummary._({
    required this.targetPath,
    required this.uassetSha256,
    required this.uexpSha256,
    required this.usmapSha256,
    required this.uassetLength,
    required this.uexpLength,
    required this.usmapLength,
  });

  final String targetPath;
  final String uassetSha256;
  final String uexpSha256;
  final String usmapSha256;
  final int uassetLength;
  final int uexpLength;
  final int usmapLength;

  factory DataAssetExtractReceiptSummary.fromJson(Map<String, Object?> json) {
    const fields = <String>{
      'ok',
      'format',
      'target_path',
      'package_seal',
      'usmap_sha256',
      'input',
    };
    if (json.keys.toSet().length != fields.length ||
        !json.keys.toSet().containsAll(fields) ||
        json['ok'] != true ||
        json['format'] != 'gore.dataasset.extract-receipt-summary.v1') {
      throw const FormatException('invalid ExtractReceipt-v2 summary');
    }
    final targetPath = json['target_path'];
    final packageSeal = json['package_seal'];
    final input = json['input'];
    final usmapSha256 = json['usmap_sha256'];
    if (targetPath is! String ||
        !_isCanonicalGameAssetPath(targetPath) ||
        packageSeal is! Map<String, Object?> ||
        input is! Map<String, Object?> ||
        usmapSha256 is! String ||
        !_semanticSha256.hasMatch(usmapSha256)) {
      throw const FormatException('invalid ExtractReceipt-v2 identity');
    }
    const sealFields = <String>{'uasset_sha256', 'uexp_sha256'};
    const inputFields = <String>{
      'uasset_length',
      'uexp_length',
      'usmap_length',
    };
    if (packageSeal.keys.toSet().length != sealFields.length ||
        !packageSeal.keys.toSet().containsAll(sealFields) ||
        input.keys.toSet().length != inputFields.length ||
        !input.keys.toSet().containsAll(inputFields)) {
      throw const FormatException('invalid ExtractReceipt-v2 facts');
    }
    final uassetSha256 = packageSeal['uasset_sha256'];
    final uexpSha256 = packageSeal['uexp_sha256'];
    final uassetLength = input['uasset_length'];
    final uexpLength = input['uexp_length'];
    final usmapLength = input['usmap_length'];
    if (uassetSha256 is! String ||
        !_semanticSha256.hasMatch(uassetSha256) ||
        uexpSha256 is! String ||
        !_semanticSha256.hasMatch(uexpSha256) ||
        uassetLength is! int ||
        uassetLength <= 0 ||
        uassetLength > 64 * 1024 * 1024 ||
        uexpLength is! int ||
        uexpLength <= 0 ||
        uexpLength > 256 * 1024 * 1024 ||
        uassetLength + uexpLength > 320 * 1024 * 1024 ||
        usmapLength is! int ||
        usmapLength <= 0 ||
        usmapLength > 128 * 1024 * 1024) {
      throw const FormatException('invalid ExtractReceipt-v2 content facts');
    }
    return DataAssetExtractReceiptSummary._(
      targetPath: targetPath,
      uassetSha256: uassetSha256,
      uexpSha256: uexpSha256,
      usmapSha256: usmapSha256,
      uassetLength: uassetLength,
      uexpLength: uexpLength,
      usmapLength: usmapLength,
    );
  }

  bool matchesInspection(DataAssetInspection inspection) =>
      uassetSha256 == inspection.binding.packageSeal.uassetSha256 &&
      uexpSha256 == inspection.binding.packageSeal.uexpSha256 &&
      usmapSha256 == inspection.binding.usmapSha256 &&
      uassetLength == inspection.input.uassetLength &&
      uexpLength == inspection.input.uexpLength &&
      usmapLength == inspection.input.usmapLength;
}

final _semanticSha256 = RegExp(r'^[0-9a-f]{64}$');
final _gameAssetSegment = RegExp(r'^[A-Za-z0-9_]+$');
const _maxGameAssetSegments = 32;

bool _isCanonicalGameAssetPath(String value) {
  if (!value.startsWith('/Game/') || value.length > 512) return false;
  final segments = value.substring('/Game/'.length).split('/');
  return segments.isNotEmpty &&
      segments.length <= _maxGameAssetSegments &&
      segments.every(
        (segment) =>
            segment.isNotEmpty &&
            _gameAssetSegment.hasMatch(segment) &&
            !_isWindowsReservedGameAssetSegment(segment),
      );
}

bool _isWindowsReservedGameAssetSegment(String value) {
  final upper = value.toUpperCase();
  if (const {'CON', 'PRN', 'AUX', 'NUL'}.contains(upper)) return true;
  return upper.length == 4 &&
      (upper.startsWith('COM') || upper.startsWith('LPT')) &&
      upper.codeUnitAt(3) >= 0x31 &&
      upper.codeUnitAt(3) <= 0x39;
}

void _addLengthPrefixed(BytesBuilder output, List<int> value) {
  final length = ByteData(8)..setUint64(0, value.length, Endian.little);
  output
    ..add(length.buffer.asUint8List())
    ..add(value);
}

final class DataAssetSemanticEditPreview {
  const DataAssetSemanticEditPreview._({
    required this.pathLabel,
    required this.typeLabel,
    required this.previousValue,
    required this.replacementValue,
    required this.intent,
  });

  final String pathLabel;
  final String typeLabel;
  final String previousValue;
  final String replacementValue;
  final DataAssetSemanticEditIntent intent;
}

/// One validated semantic value change before it is bound to a concrete
/// native source authority.
///
/// Receipt-backed and installed-snapshot-backed authoring deliberately share
/// this value-only step. The source proof is attached afterwards, so neither
/// route can accidentally serialize the other route's authority fields.
final class DataAssetSemanticValueChange {
  const DataAssetSemanticValueChange._({
    required this.pathLabel,
    required this.typeLabel,
    required this.previousValue,
    required this.replacementValue,
    required this.selector,
    required this.replacement,
  });

  final String pathLabel;
  final String typeLabel;
  final String previousValue;
  final String replacementValue;
  final FixedLeafSelector selector;
  final DataAssetSemanticReplacement replacement;

  DataAssetSemanticEditPreview bindExtractReceipt({
    required String extractReceiptPath,
    required String expectedTargetPath,
  }) {
    final path = extractReceiptPath.trim();
    if (path.isEmpty ||
        path.length > 32768 ||
        utf8.encode(path).length > 32768 ||
        path.contains('\u0000')) {
      throw const DataAssetSemanticEditException(
        'Choose the exact ExtractReceipt-v2 used for this inspected package.',
      );
    }
    final targetPath = expectedTargetPath.trim();
    if (targetPath != expectedTargetPath ||
        !_isCanonicalGameAssetPath(targetPath)) {
      throw const DataAssetSemanticEditException(
        'Verify and confirm the exact ExtractReceipt-v2 target first.',
      );
    }
    return DataAssetSemanticEditPreview._(
      pathLabel: pathLabel,
      typeLabel: typeLabel,
      previousValue: previousValue,
      replacementValue: replacementValue,
      intent: DataAssetSemanticEditIntent._(
        extractReceiptPath: path,
        expectedTargetPath: targetPath,
        selector: selector,
        replacement: replacement,
      ),
    );
  }
}

final class DataAssetSemanticValueEditor {
  DataAssetSemanticValueEditor._({
    required this.leaf,
    required this._expectedBytes,
  });

  factory DataAssetSemanticValueEditor.fromLeaf(DataAssetLeafReport leaf) {
    final selector = leaf.selector;
    if (!leaf.editable ||
        selector.role != FixedLeafRole.propertyValue ||
        !selector.kind.isEditable ||
        selector.kind == FixedWireKind.packageIndex ||
        selector.kind == FixedWireKind.fname) {
      throw const DataAssetSemanticEditException(
        'This fixed leaf is verified for inspection, but not for value editing.',
      );
    }
    return DataAssetSemanticValueEditor._(
      leaf: leaf,
      expectedBytes: _decodeHex(selector.expectedHex, selector.kind.width),
    );
  }

  final DataAssetLeafReport leaf;
  final Uint8List _expectedBytes;

  FixedLeafSelector get selector => leaf.selector;
  FixedWireKind get kind => selector.kind;
  bool get isBoolean => kind == FixedWireKind.boolean;
  bool get isComposite =>
      kind == FixedWireKind.linearColorF32x4 ||
      kind == FixedWireKind.vector4F64x4;

  String get typeLabel => switch (kind) {
    FixedWireKind.boolean => 'On / off',
    FixedWireKind.byte => 'Whole number (0–255)',
    FixedWireKind.int8 => 'Whole number (-128–127)',
    FixedWireKind.int16 => 'Whole number (-32,768–32,767)',
    FixedWireKind.int32 => 'Whole number',
    FixedWireKind.int64 => 'Large whole number',
    FixedWireKind.uint16 => 'Whole number (0–65,535)',
    FixedWireKind.uint32 => 'Positive whole number',
    FixedWireKind.uint64 => 'Large positive whole number',
    FixedWireKind.float32 => 'Decimal number',
    FixedWireKind.float64 => 'Precise decimal number',
    FixedWireKind.linearColorF32x4 => 'Linear color',
    FixedWireKind.vector4F64x4 => '4D vector',
    FixedWireKind.packageIndex || FixedWireKind.fname => 'Reference',
  };

  List<String> get componentLabels => switch (kind) {
    FixedWireKind.linearColorF32x4 => const ['Red', 'Green', 'Blue', 'Alpha'],
    FixedWireKind.vector4F64x4 => const ['X', 'Y', 'Z', 'W'],
    _ => const [],
  };

  List<String> get initialComponentValues => switch (kind) {
    FixedWireKind.linearColorF32x4 => _decodeFloat32Components(
      _expectedBytes,
    ).map(_formatNumber).toList(growable: false),
    FixedWireKind.vector4F64x4 => _decodeFloat64Components(
      _expectedBytes,
    ).map(_formatNumber).toList(growable: false),
    _ => const [],
  };

  String get initialScalarValue => switch (kind) {
    FixedWireKind.boolean => _expectedBytes.single == 0 ? 'Off' : 'On',
    FixedWireKind.byte => _expectedBytes.single.toString(),
    FixedWireKind.int8 => ByteData.sublistView(
      _expectedBytes,
    ).getInt8(0).toString(),
    FixedWireKind.int16 => ByteData.sublistView(
      _expectedBytes,
    ).getInt16(0, Endian.little).toString(),
    FixedWireKind.int32 => ByteData.sublistView(
      _expectedBytes,
    ).getInt32(0, Endian.little).toString(),
    FixedWireKind.int64 => _decodeInteger(
      _expectedBytes,
      signed: true,
    ).toString(),
    FixedWireKind.uint16 => ByteData.sublistView(
      _expectedBytes,
    ).getUint16(0, Endian.little).toString(),
    FixedWireKind.uint32 => ByteData.sublistView(
      _expectedBytes,
    ).getUint32(0, Endian.little).toString(),
    FixedWireKind.uint64 => _decodeInteger(
      _expectedBytes,
      signed: false,
    ).toString(),
    FixedWireKind.float32 => _formatNumber(
      ByteData.sublistView(_expectedBytes).getFloat32(0, Endian.little),
    ),
    FixedWireKind.float64 => _formatNumber(
      ByteData.sublistView(_expectedBytes).getFloat64(0, Endian.little),
    ),
    FixedWireKind.linearColorF32x4 => _componentDisplay(
      componentLabels,
      initialComponentValues,
    ),
    FixedWireKind.vector4F64x4 => _componentDisplay(
      componentLabels,
      initialComponentValues,
    ),
    FixedWireKind.packageIndex || FixedWireKind.fname => 'Reference',
  };

  DataAssetSemanticEditPreview previewBool({
    required String extractReceiptPath,
    required String expectedTargetPath,
    required bool value,
  }) {
    return changeBool(value: value).bindExtractReceipt(
      extractReceiptPath: extractReceiptPath,
      expectedTargetPath: expectedTargetPath,
    );
  }

  DataAssetSemanticValueChange changeBool({required bool value}) {
    if (!isBoolean) {
      throw const DataAssetSemanticEditException(
        'The selected value is not an on/off field.',
      );
    }
    return _change(
      DataAssetSemanticReplacement._(
        kind: kind,
        displayValue: value ? 'On' : 'Off',
        wire: <String, Object>{'kind': 'bool', 'value': value},
        comparisonBytes: Uint8List.fromList([value ? 1 : 0]),
      ),
    );
  }

  DataAssetSemanticEditPreview previewScalar({
    required String extractReceiptPath,
    required String expectedTargetPath,
    required String value,
  }) {
    return changeScalar(value: value).bindExtractReceipt(
      extractReceiptPath: extractReceiptPath,
      expectedTargetPath: expectedTargetPath,
    );
  }

  DataAssetSemanticValueChange changeScalar({required String value}) {
    if (isBoolean || isComposite) {
      throw const DataAssetSemanticEditException(
        'The selected value requires its dedicated editor.',
      );
    }
    final replacement = switch (kind) {
      FixedWireKind.byte => _integerReplacement(
        value,
        BigInt.zero,
        BigInt.from(255),
      ),
      FixedWireKind.int8 => _integerReplacement(
        value,
        BigInt.from(-128),
        BigInt.from(127),
      ),
      FixedWireKind.int16 => _integerReplacement(
        value,
        BigInt.from(-32768),
        BigInt.from(32767),
      ),
      FixedWireKind.int32 => _integerReplacement(
        value,
        BigInt.from(-2147483648),
        BigInt.from(2147483647),
      ),
      FixedWireKind.int64 => _integerReplacement(
        value,
        -(BigInt.one << 63),
        (BigInt.one << 63) - BigInt.one,
      ),
      FixedWireKind.uint16 => _integerReplacement(
        value,
        BigInt.zero,
        BigInt.from(65535),
      ),
      FixedWireKind.uint32 => _integerReplacement(
        value,
        BigInt.zero,
        BigInt.from(4294967295),
      ),
      FixedWireKind.uint64 => _integerReplacement(
        value,
        BigInt.zero,
        (BigInt.one << 64) - BigInt.one,
      ),
      FixedWireKind.float32 => _floatReplacement(value, singlePrecision: true),
      FixedWireKind.float64 => _floatReplacement(value, singlePrecision: false),
      FixedWireKind.boolean ||
      FixedWireKind.linearColorF32x4 ||
      FixedWireKind.vector4F64x4 ||
      FixedWireKind.packageIndex ||
      FixedWireKind.fname => throw const DataAssetSemanticEditException(
        'The selected fixed leaf is not a scalar value field.',
      ),
    };
    return _change(replacement);
  }

  DataAssetSemanticEditPreview previewComponents({
    required String extractReceiptPath,
    required String expectedTargetPath,
    required List<String> values,
  }) {
    return changeComponents(values: values).bindExtractReceipt(
      extractReceiptPath: extractReceiptPath,
      expectedTargetPath: expectedTargetPath,
    );
  }

  DataAssetSemanticValueChange changeComponents({
    required List<String> values,
  }) {
    if (!isComposite || values.length != 4) {
      throw const DataAssetSemanticEditException(
        'The selected value requires exactly four components.',
      );
    }
    final singlePrecision = kind == FixedWireKind.linearColorF32x4;
    final parsed = values
        .map((value) => _parseFloat(value, singlePrecision: singlePrecision))
        .toList(growable: false);
    final normalized = parsed.map(_formatNumber).toList(growable: false);
    final bytes = BytesBuilder(copy: false);
    for (final value in parsed) {
      final data = ByteData(singlePrecision ? 4 : 8);
      if (singlePrecision) {
        data.setFloat32(0, value, Endian.little);
      } else {
        data.setFloat64(0, value, Endian.little);
      }
      bytes.add(data.buffer.asUint8List());
    }
    final fields = singlePrecision
        ? <String, Object>{
            'kind': 'linear_color_f32x4',
            'r': normalized[0],
            'g': normalized[1],
            'b': normalized[2],
            'a': normalized[3],
          }
        : <String, Object>{
            'kind': 'vector4_f64x4',
            'x': normalized[0],
            'y': normalized[1],
            'z': normalized[2],
            'w': normalized[3],
          };
    return _change(
      DataAssetSemanticReplacement._(
        kind: kind,
        displayValue: _componentDisplay(componentLabels, normalized),
        wire: fields,
        comparisonBytes: bytes.takeBytes(),
      ),
    );
  }

  DataAssetSemanticReplacement _integerReplacement(
    String raw,
    BigInt minimum,
    BigInt maximum,
  ) {
    final text = raw.trim();
    final value = BigInt.tryParse(text);
    if (value == null ||
        value < minimum ||
        value > maximum ||
        value.toString() != text) {
      throw DataAssetSemanticEditException(
        'Enter a whole number from $minimum to $maximum.',
      );
    }
    return DataAssetSemanticReplacement._(
      kind: kind,
      displayValue: value.toString(),
      wire: <String, Object>{
        'kind': kind.wireName,
        'decimal': value.toString(),
      },
      comparisonBytes: _encodeInteger(value, kind.width),
    );
  }

  DataAssetSemanticReplacement _floatReplacement(
    String raw, {
    required bool singlePrecision,
  }) {
    final parsed = _parseFloat(raw, singlePrecision: singlePrecision);
    final normalized = _formatNumber(parsed);
    final data = ByteData(singlePrecision ? 4 : 8);
    if (singlePrecision) {
      data.setFloat32(0, parsed, Endian.little);
    } else {
      data.setFloat64(0, parsed, Endian.little);
    }
    return DataAssetSemanticReplacement._(
      kind: kind,
      displayValue: normalized,
      wire: <String, Object>{'kind': kind.wireName, 'decimal': normalized},
      comparisonBytes: data.buffer.asUint8List(),
    );
  }

  DataAssetSemanticValueChange _change(
    DataAssetSemanticReplacement replacement,
  ) {
    if (_sameBytes(_expectedBytes, replacement._comparisonBytes)) {
      throw const DataAssetSemanticEditException(
        'Choose a new value; the current value would not change.',
      );
    }
    return DataAssetSemanticValueChange._(
      pathLabel: selector.pathLabel,
      typeLabel: typeLabel,
      previousValue: initialScalarValue,
      replacementValue: replacement.displayValue,
      selector: selector,
      replacement: replacement,
    );
  }
}

final class DataAssetSemanticEditException implements Exception {
  const DataAssetSemanticEditException(this.message);
  final String message;

  @override
  String toString() => message;
}

double _parseFloat(String raw, {required bool singlePrecision}) {
  final text = raw.trim();
  final wide = double.tryParse(text);
  if (wide == null || !wide.isFinite) {
    throw const DataAssetSemanticEditException(
      'Enter a finite decimal number.',
    );
  }
  if (!singlePrecision) return wide;
  final data = ByteData(4)..setFloat32(0, wide, Endian.little);
  final narrowed = data.getFloat32(0, Endian.little);
  if (!narrowed.isFinite || (wide != 0 && narrowed == 0)) {
    throw const DataAssetSemanticEditException(
      'This number is outside the supported decimal range.',
    );
  }
  return narrowed;
}

Uint8List _decodeHex(String value, int width) {
  if (value.length != width * 2) {
    throw const DataAssetSemanticEditException(
      'The verified current value has an invalid width.',
    );
  }
  final result = Uint8List(width);
  for (var index = 0; index < width; index++) {
    final byte = int.tryParse(
      value.substring(index * 2, index * 2 + 2),
      radix: 16,
    );
    if (byte == null) {
      throw const DataAssetSemanticEditException(
        'The verified current value is not valid.',
      );
    }
    result[index] = byte;
  }
  return result;
}

BigInt _decodeInteger(Uint8List bytes, {required bool signed}) {
  var value = BigInt.zero;
  for (var index = 0; index < bytes.length; index++) {
    value |= BigInt.from(bytes[index]) << (index * 8);
  }
  if (signed && (bytes.last & 0x80) != 0) {
    value -= BigInt.one << (bytes.length * 8);
  }
  return value;
}

Uint8List _encodeInteger(BigInt value, int width) {
  var encoded = value;
  if (encoded.isNegative) encoded += BigInt.one << (width * 8);
  final bytes = Uint8List(width);
  for (var index = 0; index < width; index++) {
    bytes[index] = ((encoded >> (index * 8)) & BigInt.from(0xff)).toInt();
  }
  return bytes;
}

List<double> _decodeFloat32Components(Uint8List bytes) {
  final data = ByteData.sublistView(bytes);
  return List<double>.generate(
    4,
    (index) => data.getFloat32(index * 4, Endian.little),
    growable: false,
  );
}

List<double> _decodeFloat64Components(Uint8List bytes) {
  final data = ByteData.sublistView(bytes);
  return List<double>.generate(
    4,
    (index) => data.getFloat64(index * 8, Endian.little),
    growable: false,
  );
}

String _formatNumber(double value) => value.toString();

String _componentDisplay(List<String> labels, List<String> values) =>
    List.generate(
      labels.length,
      (index) => '${labels[index]} ${values[index]}',
    ).join(', ');

bool _sameBytes(Uint8List left, Uint8List right) {
  if (left.length != right.length) return false;
  for (var index = 0; index < left.length; index++) {
    if (left[index] != right[index]) return false;
  }
  return true;
}
