import 'dart:typed_data';

import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/project/revision3_texture_catalog.dart';

void main() {
  test('native source fingerprint is bounded opaque generation evidence', () {
    final first = Revision3TextureSourceFingerprint.nativeBuildId(
      'G1R.usmap|utoc:23891898:1784041556',
    );
    final equivalent = Revision3TextureSourceFingerprint.nativeBuildId(
      'G1R.usmap|utoc:23891898:1784041556',
    );

    expect(first, equivalent);
    expect(first.hashCode, equivalent.hashCode);
    for (final invalid in <String>[
      '',
      ' source',
      'source\n',
      'x' * (Revision3TextureSourceFingerprint.maximumCodeUnits + 1),
    ]) {
      expect(
        () => Revision3TextureSourceFingerprint.nativeBuildId(invalid),
        throwsArgumentError,
      );
    }
  });

  test('package id is canonical decimal unsigned u64', () {
    expect(Revision3TexturePackageId.fromDecimal('0').decimal, '0');
    expect(
      Revision3TexturePackageId.fromDecimal('18446744073709551615').value,
      Revision3TexturePackageId.maximumValue,
    );
    for (final invalid in <String>[
      '',
      ' 1',
      '+1',
      '-1',
      '01',
      '0x1',
      '18446744073709551616',
    ]) {
      expect(
        () => Revision3TexturePackageId.fromDecimal(invalid),
        throwsArgumentError,
        reason: invalid,
      );
    }
  });

  test('installed index atomically binds sorted rows to its fingerprint', () {
    final snapshot = Revision3TextureCatalogSnapshot.fromInstalledIndex(
      sourceFingerprint: _fingerprintA,
      index: const {'/Game/Z/T_Z': '2', '/Engine/A/T_A': '1'},
    );

    expect(snapshot.sourceFingerprint, _fingerprintA);
    expect(snapshot.assetPaths, const ['/Engine/A/T_A', '/Game/Z/T_Z']);
    expect(snapshot.byAssetPath['/Engine/A/T_A']!.packageId.decimal, '1');
    expect(snapshot.byAssetPath['/Engine/A/T_A']!.displayName, 'T_A');
    expect(
      () => snapshot.assetPaths.add('/Game/New/T_New'),
      throwsUnsupportedError,
    );
    expect(
      () => snapshot.byAssetPath['/Game/New/T_New'] = _entry('/Game/New/T_New'),
      throwsUnsupportedError,
    );
  });

  test('catalog grammar covers exact observed Unreal index shapes', () {
    for (final path in <String>[
      '/Game/Assets/Textures/Pre-Atlas/T_Boots-01_D',
      '/Engine/EditorResources/S_Actor',
      '/DatasmithContent/Materials/T_Water_N',
      '/Game/Weather/StaticClouds_Alpha+Up',
    ]) {
      expect(() => _entry(path), returnsNormally, reason: path);
    }

    for (final path in <String>[
      '',
      ' /Game/UI/T_Icon',
      'Game/UI/T_Icon',
      '/Game/',
      '/Game/UI/',
      '/Game//T_Icon',
      r'/Game/UI\T_Icon',
      '/Game/./T_Icon',
      '/Game/UI/T.Icon',
      '/Game/UI/T Icon',
      '/Game/UI/T_Icon\n',
      '/Game/${'x' * 1024}',
    ]) {
      expect(() => _entry(path), throwsArgumentError, reason: path);
    }
  });

  test('catalog rejects case-insensitive path collisions', () {
    expect(
      () => Revision3TextureCatalogSnapshot(
        sourceFingerprint: _fingerprintA,
        textures: [_entry('/Game/UI/T_Icon'), _entry('/Game/ui/t_icon')],
      ),
      throwsArgumentError,
    );
  });

  test('catalog enforces a bounded lazy row count', () {
    final oversized = Iterable<Revision3TextureCatalogEntry>.generate(
      Revision3TextureCatalogSnapshot.maximumTextureCount + 1,
      (index) => _entry('/Game/Generated/T_$index', packageId: '$index'),
    );

    expect(
      () => Revision3TextureCatalogSnapshot(
        sourceFingerprint: _fingerprintA,
        textures: oversized,
      ),
      throwsArgumentError,
    );
  });

  test(
    'preview strictly binds IHDR dimensions and defensively copies bytes',
    () {
      final source = _pngIhdr(width: 8192, height: 4096);
      final preview = Revision3TexturePreview(
        pngBytes: source,
        width: 8192,
        height: 4096,
        pixelFormat: '',
        isVirtual: true,
        virtualLayers: 1,
        mipmapped: true,
        replaceability: Revision3TextureReplaceability.unknown,
      );
      final originalFirstByte = preview.pngBytes.first;
      source[0] = 0;

      expect(preview.pixelFormat, isEmpty);
      expect(preview.virtualLayers, 1);
      expect(preview.pngBytes.first, originalFirstByte);
      expect(() => preview.pngBytes[0] = 0, throwsUnsupportedError);
    },
  );

  test('preview rejects mismatched or malformed IHDR', () {
    Revision3TexturePreview build(
      Uint8List bytes, {
      int width = 1,
      int height = 1,
      String format = 'PF_BC7',
    }) => Revision3TexturePreview(
      pngBytes: bytes,
      width: width,
      height: height,
      pixelFormat: format,
      isVirtual: false,
      virtualLayers: 0,
      mipmapped: false,
      replaceability: Revision3TextureReplaceability.supported,
    );

    expect(() => build(_pngIhdr(width: 2, height: 1)), throwsArgumentError);
    final badCrc = _pngIhdr(width: 1, height: 1)..[29] ^= 1;
    expect(() => build(badCrc), throwsArgumentError);
    expect(
      () => build(_pngIhdr(width: 1, height: 1, bitDepth: 4)),
      throwsArgumentError,
    );
    final badChunkLength = _pngIhdr(width: 1, height: 1)..[11] = 12;
    expect(() => build(badChunkLength), throwsArgumentError);
    expect(
      () => build(Uint8List.sublistView(_pngIhdr(width: 1, height: 1), 0, 33)),
      throwsArgumentError,
      reason: 'an IHDR alone is not a complete PNG image',
    );
    final badIendCrc = _pngIhdr(width: 1, height: 1)..last ^= 1;
    expect(() => build(badIendCrc), throwsArgumentError);
    expect(
      () => build(Uint8List.fromList(List<int>.filled(33, 0))),
      throwsArgumentError,
    );
  });

  test('preview bounds decoded RGBA, format, and source file bytes', () {
    expect(
      () => Revision3TexturePreview(
        pngBytes: _pngIhdr(width: 8192, height: 8192),
        width: 8192,
        height: 8192,
        pixelFormat: 'PF_BC7',
        isVirtual: false,
        virtualLayers: 0,
        mipmapped: false,
        replaceability: Revision3TextureReplaceability.supported,
      ),
      throwsArgumentError,
    );
    expect(
      () => Revision3TexturePreview(
        pngBytes: _pngIhdr(width: 1, height: 1),
        width: 1,
        height: 1,
        pixelFormat: ' PF_BC7',
        isVirtual: false,
        virtualLayers: 0,
        mipmapped: false,
        replaceability: Revision3TextureReplaceability.supported,
      ),
      throwsArgumentError,
    );
    expect(
      () => Revision3TexturePreview.validatePngByteLength(
        Revision3TexturePreview.maximumPngByteLength + 1,
      ),
      throwsArgumentError,
    );
    expect(
      () => Revision3TexturePreview(
        pngBytes: _pngIhdr(width: 1, height: 1),
        width: 1,
        height: 1,
        pixelFormat: 'PF_BC7',
        isVirtual: true,
        virtualLayers: 0,
        mipmapped: false,
        replaceability: Revision3TextureReplaceability.unsupported,
      ),
      throwsArgumentError,
    );
    expect(
      () => Revision3TexturePreview(
        pngBytes: _pngIhdr(width: 1, height: 1),
        width: 1,
        height: 1,
        pixelFormat: 'PF_BC7',
        isVirtual: false,
        virtualLayers: 1,
        mipmapped: false,
        replaceability: Revision3TextureReplaceability.unsupported,
      ),
      throwsArgumentError,
    );
  });
}

final _fingerprintA = Revision3TextureSourceFingerprint.nativeBuildId(
  'build-a|utoc:1:1',
);

Revision3TextureCatalogEntry _entry(String path, {String packageId = '1'}) =>
    Revision3TextureCatalogEntry(
      assetPath: path,
      packageId: Revision3TexturePackageId.fromDecimal(packageId),
    );

Uint8List _pngIhdr({
  required int width,
  required int height,
  int bitDepth = 8,
  int colorType = 6,
}) {
  const idat = <int>[0x78, 0x9c, 0x63, 0x00, 0x00, 0x00, 0x01, 0x00, 0x01];
  final bytes = Uint8List(33 + 12 + idat.length + 12);
  bytes.setAll(0, const [137, 80, 78, 71, 13, 10, 26, 10]);
  _writeUint32(bytes, 8, 13);
  bytes.setAll(12, const [73, 72, 68, 82]);
  _writeUint32(bytes, 16, width);
  _writeUint32(bytes, 20, height);
  bytes[24] = bitDepth;
  bytes[25] = colorType;
  bytes[26] = 0;
  bytes[27] = 0;
  bytes[28] = 0;
  _writeUint32(bytes, 29, _crc32(bytes, 12, 29));
  var offset = 33;
  _writeUint32(bytes, offset, idat.length);
  bytes.setAll(offset + 4, const [73, 68, 65, 84]);
  bytes.setAll(offset + 8, idat);
  _writeUint32(
    bytes,
    offset + 8 + idat.length,
    _crc32(bytes, offset + 4, offset + 8 + idat.length),
  );
  offset += 12 + idat.length;
  _writeUint32(bytes, offset, 0);
  bytes.setAll(offset + 4, const [73, 69, 78, 68]);
  _writeUint32(bytes, offset + 8, _crc32(bytes, offset + 4, offset + 8));
  return bytes;
}

void _writeUint32(Uint8List bytes, int offset, int value) {
  bytes[offset] = (value >> 24) & 0xff;
  bytes[offset + 1] = (value >> 16) & 0xff;
  bytes[offset + 2] = (value >> 8) & 0xff;
  bytes[offset + 3] = value & 0xff;
}

int _crc32(Uint8List bytes, int start, int end) {
  var crc = 0xffffffff;
  for (var index = start; index < end; index++) {
    crc ^= bytes[index];
    for (var bit = 0; bit < 8; bit++) {
      crc = (crc & 1) == 1 ? (0xedb88320 ^ (crc >> 1)) : (crc >> 1);
    }
  }
  return (crc ^ 0xffffffff) & 0xffffffff;
}
