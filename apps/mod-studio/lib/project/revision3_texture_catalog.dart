import 'package:flutter/foundation.dart';

typedef Revision3TextureCatalogLoader =
    Future<Revision3TextureCatalogSnapshot> Function({
      required String gameRoot,
    });

typedef Revision3TexturePreviewLoader =
    Future<Revision3TexturePreviewResult> Function({
      required String gameRoot,
      required Revision3TextureSourceFingerprint expectedSourceFingerprint,
      required Revision3TextureCatalogEntry texture,
    });

/// Signals that an installed-game generation changed after its catalog snapshot was loaded.
/// The UI must reload the catalog and must never retry against the stale fingerprint.
final class Revision3TextureSourceChangedException implements Exception {
  const Revision3TextureSourceChangedException(this.nativeCode);

  final String nativeCode;

  @override
  String toString() => 'installed texture source changed ($nativeCode)';
}

/// Opaque native build fingerprint returned atomically with a texture index.
@immutable
final class Revision3TextureSourceFingerprint {
  const Revision3TextureSourceFingerprint._(this.value);

  factory Revision3TextureSourceFingerprint.nativeBuildId(String value) {
    if (value.isEmpty ||
        value != value.trim() ||
        value.length > maximumCodeUnits ||
        _containsControlCodeUnit(value)) {
      throw ArgumentError.value(
        value,
        'value',
        'expected one bounded non-empty native build fingerprint',
      );
    }
    return Revision3TextureSourceFingerprint._(value);
  }

  static const maximumCodeUnits = 512;

  final String value;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is Revision3TextureSourceFingerprint && other.value == value;

  @override
  int get hashCode => value.hashCode;

  @override
  String toString() => value;
}

/// Exact unsigned package id emitted by the installed texture index.
@immutable
final class Revision3TexturePackageId {
  const Revision3TexturePackageId._(this.value);

  factory Revision3TexturePackageId.fromDecimal(String value) {
    if (!_decimalPattern.hasMatch(value)) {
      throw ArgumentError.value(
        value,
        'value',
        'expected one canonical unsigned 64-bit decimal package id',
      );
    }
    final parsed = BigInt.parse(value);
    if (parsed > maximumValue) {
      throw ArgumentError.value(
        value,
        'value',
        'package id exceeds unsigned 64-bit range',
      );
    }
    return Revision3TexturePackageId._(parsed);
  }

  static final RegExp _decimalPattern = RegExp(r'^(0|[1-9][0-9]{0,19})$');
  static final BigInt maximumValue = (BigInt.one << 64) - BigInt.one;

  final BigInt value;
  String get decimal => value.toString();

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is Revision3TexturePackageId && other.value == value;

  @override
  int get hashCode => value.hashCode;

  @override
  String toString() => decimal;
}

/// One texture identity from an exact installed-game index.
///
/// This is discovery metadata only. It grants no project, edit, build,
/// deployment, runtime, game-installation, or save authority.
@immutable
final class Revision3TextureCatalogEntry {
  Revision3TextureCatalogEntry({
    required this.assetPath,
    required this.packageId,
  }) {
    validateAssetPath(assetPath);
  }

  static const maximumAssetPathCodeUnits = 1024;
  static final RegExp _canonicalLongPackageName = RegExp(
    r'^/[A-Za-z0-9_]+(?:/[A-Za-z0-9_+\-]+)+$',
  );

  final String assetPath;
  final Revision3TexturePackageId packageId;

  static void validateAssetPath(String assetPath) {
    if (assetPath.length > maximumAssetPathCodeUnits ||
        !_canonicalLongPackageName.hasMatch(assetPath)) {
      throw ArgumentError.value(
        assetPath,
        'assetPath',
        'expected one canonical Unreal long package name',
      );
    }
  }

  String get displayName {
    final separator = assetPath.lastIndexOf('/');
    return separator < 0 ? assetPath : assetPath.substring(separator + 1);
  }
}

/// Immutable texture rows and their atomically observed native build.
@immutable
final class Revision3TextureCatalogSnapshot {
  Revision3TextureCatalogSnapshot({
    required Revision3TextureSourceFingerprint sourceFingerprint,
    required Iterable<Revision3TextureCatalogEntry> textures,
  }) : this._validated(sourceFingerprint, _validatedTextures(textures));

  Revision3TextureCatalogSnapshot._validated(
    this.sourceFingerprint,
    this.textures,
  ) : assetPaths = List<String>.unmodifiable(
        textures.map((texture) => texture.assetPath),
      ),
      byAssetPath = Map<String, Revision3TextureCatalogEntry>.unmodifiable({
        for (final texture in textures) texture.assetPath: texture,
      });

  static const maximumTextureCount = 65536;

  factory Revision3TextureCatalogSnapshot.fromInstalledIndex({
    required Revision3TextureSourceFingerprint sourceFingerprint,
    required Map<String, String> index,
  }) {
    validateTextureCount(index.length);
    final textures =
        <Revision3TextureCatalogEntry>[
          for (final entry in index.entries)
            Revision3TextureCatalogEntry(
              assetPath: entry.key,
              packageId: Revision3TexturePackageId.fromDecimal(entry.value),
            ),
        ]..sort((left, right) {
          final folded = left.assetPath.toLowerCase().compareTo(
            right.assetPath.toLowerCase(),
          );
          return folded != 0
              ? folded
              : left.assetPath.compareTo(right.assetPath);
        });
    return Revision3TextureCatalogSnapshot(
      sourceFingerprint: sourceFingerprint,
      textures: textures,
    );
  }

  final Revision3TextureSourceFingerprint sourceFingerprint;
  final List<Revision3TextureCatalogEntry> textures;
  final List<String> assetPaths;
  final Map<String, Revision3TextureCatalogEntry> byAssetPath;

  static List<Revision3TextureCatalogEntry> _validatedTextures(
    Iterable<Revision3TextureCatalogEntry> textures,
  ) {
    final validated = <Revision3TextureCatalogEntry>[];
    final foldedPaths = <String>{};
    for (final texture in textures) {
      if (validated.length == maximumTextureCount) {
        throw ArgumentError.value(
          validated.length + 1,
          'textures',
          'texture count exceeds $maximumTextureCount',
        );
      }
      Revision3TextureCatalogEntry.validateAssetPath(texture.assetPath);
      if (!foldedPaths.add(texture.assetPath.toLowerCase())) {
        throw ArgumentError.value(
          texture.assetPath,
          'textures',
          'duplicate ASCII-case-insensitive texture path',
        );
      }
      validated.add(texture);
    }
    return List<Revision3TextureCatalogEntry>.unmodifiable(validated);
  }

  static void validateTextureCount(int count) {
    if (count < 0 || count > maximumTextureCount) {
      throw ArgumentError.value(
        count,
        'count',
        'expected 0..$maximumTextureCount textures',
      );
    }
  }
}

enum Revision3TextureReplaceability { supported, unsupported, unknown }

/// Decoded original-game PNG and inspect-only source facts.
///
/// Bytes keep native capability access outside the reusable view. The owner
/// must reject oversized native streams before reading. This DTO defensively
/// copies the accepted byte buffer.
@immutable
final class Revision3TexturePreview {
  Revision3TexturePreview({
    required Uint8List pngBytes,
    required this.width,
    required this.height,
    required this.pixelFormat,
    required this.isVirtual,
    required this.virtualLayers,
    required this.mipmapped,
    required this.replaceability,
  }) : pngBytes = _boundedDefensiveCopy(pngBytes) {
    final ihdr = _parseIhdr(this.pngBytes);
    if (ihdr.width != width || ihdr.height != height) {
      throw ArgumentError.value(
        '$width x $height',
        'dimensions',
        'metadata does not match PNG IHDR ${ihdr.width} x ${ihdr.height}',
      );
    }
    validateDimensions(width: width, height: height);
    if (pixelFormat != pixelFormat.trim() ||
        pixelFormat.length > maximumPixelFormatCodeUnits ||
        _containsControlCodeUnit(pixelFormat)) {
      throw ArgumentError.value(
        pixelFormat,
        'pixelFormat',
        'expected empty (unknown) or one bounded source format',
      );
    }
    if (virtualLayers < 0 ||
        virtualLayers > maximumVirtualLayers ||
        (isVirtual ? virtualLayers == 0 : virtualLayers != 0)) {
      throw ArgumentError.value(
        virtualLayers,
        'virtualLayers',
        'expected 1..$maximumVirtualLayers for a virtual texture and 0 otherwise',
      );
    }
  }

  static const maximumDimension = 32768;
  static const maximumPngByteLength = 64 * 1024 * 1024;
  static const maximumDecodedRgbaByteLength = 128 * 1024 * 1024;
  static const maximumPixelFormatCodeUnits = 128;
  static const maximumVirtualLayers = 64;

  final Uint8List pngBytes;
  final int width;
  final int height;

  /// Empty is the explicit inspect-only "unknown source format" state.
  final String pixelFormat;
  final bool isVirtual;
  final int virtualLayers;
  final bool mipmapped;
  final Revision3TextureReplaceability replaceability;

  static void validateDimensions({required int width, required int height}) {
    final decodedRgbaBytes = width * height * 4;
    if (width <= 0 ||
        height <= 0 ||
        width > maximumDimension ||
        height > maximumDimension ||
        decodedRgbaBytes > maximumDecodedRgbaByteLength) {
      throw ArgumentError.value(
        '$width x $height',
        'dimensions',
        'expected positive dimensions up to $maximumDimension and '
            '$maximumDecodedRgbaByteLength decoded RGBA bytes',
      );
    }
  }

  /// Lets an injected adapter reject a large native file before reading it.
  static void validatePngByteLength(int byteLength) {
    if (byteLength < _minimumPngByteLength ||
        byteLength > maximumPngByteLength) {
      throw ArgumentError.value(
        byteLength,
        'byteLength',
        'expected a $_minimumPngByteLength..$maximumPngByteLength byte PNG',
      );
    }
  }

  static const _minimumPngByteLength = 57;

  static Uint8List _boundedDefensiveCopy(Uint8List bytes) {
    validatePngByteLength(bytes.length);
    return Uint8List.fromList(bytes).asUnmodifiableView();
  }

  static _PngIhdr _parseIhdr(Uint8List bytes) {
    const signature = <int>[137, 80, 78, 71, 13, 10, 26, 10];
    for (var index = 0; index < signature.length; index++) {
      if (bytes[index] != signature[index]) {
        throw ArgumentError.value(
          bytes.length,
          'pngBytes',
          'bad PNG signature',
        );
      }
    }
    const ihdrType = 0x49484452;
    const plteType = 0x504c5445;
    const idatType = 0x49444154;
    const iendType = 0x49454e44;
    var offset = signature.length;
    var seenIhdr = false;
    var seenPlte = false;
    var seenIdat = false;
    var idatFinished = false;
    var seenIend = false;
    var idatByteLength = 0;
    var width = 0;
    var height = 0;
    var bitDepth = 0;
    var colorType = -1;
    while (offset < bytes.length) {
      if (bytes.length - offset < 12) {
        throw ArgumentError.value(
          bytes.length,
          'pngBytes',
          'truncated PNG chunk',
        );
      }
      final chunkLength = _readUint32(bytes, offset);
      final typeOffset = offset + 4;
      final dataOffset = typeOffset + 4;
      final dataEnd = dataOffset + chunkLength;
      final chunkEnd = dataEnd + 4;
      if (dataEnd < dataOffset ||
          chunkEnd < dataEnd ||
          chunkEnd > bytes.length) {
        throw ArgumentError.value(
          bytes.length,
          'pngBytes',
          'PNG chunk length exceeds payload',
        );
      }
      for (var index = typeOffset; index < dataOffset; index++) {
        final byte = bytes[index];
        if (!_isAsciiLetter(byte) ||
            (index == typeOffset + 2 && !_isAsciiUpper(byte))) {
          throw ArgumentError.value(
            bytes.length,
            'pngBytes',
            'invalid PNG chunk type',
          );
        }
      }
      final chunkType = _readUint32(bytes, typeOffset);
      final expectedCrc = _readUint32(bytes, dataEnd);
      final actualCrc = _crc32(bytes, typeOffset, dataEnd);
      if (actualCrc != expectedCrc) {
        throw ArgumentError.value(
          bytes.length,
          'pngBytes',
          'invalid PNG chunk CRC',
        );
      }
      if (seenIdat && chunkType != idatType) idatFinished = true;

      switch (chunkType) {
        case ihdrType:
          if (seenIhdr || offset != signature.length || chunkLength != 13) {
            throw ArgumentError.value(
              bytes.length,
              'pngBytes',
              'PNG must begin with exactly one 13-byte IHDR chunk',
            );
          }
          width = _readUint32(bytes, dataOffset);
          height = _readUint32(bytes, dataOffset + 4);
          bitDepth = bytes[dataOffset + 8];
          colorType = bytes[dataOffset + 9];
          final compression = bytes[dataOffset + 10];
          final filter = bytes[dataOffset + 11];
          final interlace = bytes[dataOffset + 12];
          final validDepth = switch (colorType) {
            0 => const {1, 2, 4, 8, 16}.contains(bitDepth),
            2 => const {8, 16}.contains(bitDepth),
            3 => const {1, 2, 4, 8}.contains(bitDepth),
            4 || 6 => const {8, 16}.contains(bitDepth),
            _ => false,
          };
          if (width == 0 ||
              height == 0 ||
              !validDepth ||
              compression != 0 ||
              filter != 0 ||
              (interlace != 0 && interlace != 1)) {
            throw ArgumentError.value(
              bytes.length,
              'pngBytes',
              'invalid PNG IHDR fields',
            );
          }
          seenIhdr = true;
        case plteType:
          final maximumEntries = colorType == 3 ? 1 << bitDepth : 256;
          if (!seenIhdr ||
              seenPlte ||
              seenIdat ||
              colorType == 0 ||
              colorType == 4 ||
              chunkLength == 0 ||
              chunkLength % 3 != 0 ||
              chunkLength ~/ 3 > maximumEntries) {
            throw ArgumentError.value(
              bytes.length,
              'pngBytes',
              'invalid PNG PLTE chunk',
            );
          }
          seenPlte = true;
        case idatType:
          if (!seenIhdr || seenIend || idatFinished) {
            throw ArgumentError.value(
              bytes.length,
              'pngBytes',
              'invalid PNG IDAT order',
            );
          }
          seenIdat = true;
          idatByteLength += chunkLength;
        case iendType:
          if (!seenIhdr ||
              !seenIdat ||
              idatByteLength == 0 ||
              seenIend ||
              chunkLength != 0 ||
              chunkEnd != bytes.length) {
            throw ArgumentError.value(
              bytes.length,
              'pngBytes',
              'invalid or non-terminal PNG IEND chunk',
            );
          }
          seenIend = true;
        default:
          if (!seenIhdr || seenIend || _isAsciiUpper(bytes[typeOffset])) {
            throw ArgumentError.value(
              bytes.length,
              'pngBytes',
              'unsupported critical or misplaced PNG chunk',
            );
          }
      }
      offset = chunkEnd;
    }
    if (!seenIend || (colorType == 3 && !seenPlte)) {
      throw ArgumentError.value(
        bytes.length,
        'pngBytes',
        'PNG is missing required image or terminal chunks',
      );
    }
    return _PngIhdr(width: width, height: height);
  }

  static bool _isAsciiLetter(int byte) =>
      _isAsciiUpper(byte) || (byte >= 0x61 && byte <= 0x7a);

  static bool _isAsciiUpper(int byte) => byte >= 0x41 && byte <= 0x5a;

  static int _readUint32(Uint8List bytes, int offset) =>
      (bytes[offset] << 24) |
      (bytes[offset + 1] << 16) |
      (bytes[offset + 2] << 8) |
      bytes[offset + 3];

  static final List<int> _crc32Table = List<int>.generate(256, (value) {
    var crc = value;
    for (var bit = 0; bit < 8; bit++) {
      crc = (crc & 1) == 1 ? (0xedb88320 ^ (crc >> 1)) : (crc >> 1);
    }
    return crc & 0xffffffff;
  }, growable: false);

  static int _crc32(Uint8List bytes, int start, int end) {
    var crc = 0xffffffff;
    for (var index = start; index < end; index++) {
      crc = _crc32Table[(crc ^ bytes[index]) & 0xff] ^ (crc >> 8);
    }
    return (crc ^ 0xffffffff) & 0xffffffff;
  }
}

@immutable
final class Revision3TexturePreviewResult {
  const Revision3TexturePreviewResult({
    required this.sourceFingerprint,
    required this.preview,
  });

  final Revision3TextureSourceFingerprint sourceFingerprint;
  final Revision3TexturePreview preview;
}

final class _PngIhdr {
  const _PngIhdr({required this.width, required this.height});
  final int width;
  final int height;
}

bool _containsControlCodeUnit(String value) =>
    value.codeUnits.any((codeUnit) => codeUnit < 0x20 || codeUnit == 0x7f);
