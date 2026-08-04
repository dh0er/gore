import 'dart:convert';
import 'dart:typed_data';

import 'package:crypto/crypto.dart' as crypto;
import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/core/core_service.dart';
import 'package:gore_mod/core/mod_ffi.dart';
import 'package:gore_mod/project/revision3_texture_catalog.dart';
import 'package:gore_mod/project/revision3_texture_catalog_native.dart';

void main() {
  test(
    'loads one atomic catalog and a multi-chunk capability preview',
    () async {
      // Deliberately cross the native 512 KiB chunk boundary.
      final pngBytes = _pngIhdr(
        width: 1,
        height: 1,
        ancillaryByteLength: 600 * 1024,
      );
      final token = 'a' * 64;
      final sha256 = crypto.sha256.convert(pngBytes).toString();
      final core = FakeGoreCoreFfiService(
        responses: {
          'texture_index': {
            'ok': true,
            'build_id': 'build-24169431',
            'count': 1,
            'entries': {'/Game/Items/T_Apple': '42'},
          },
          'texture_extract': {
            'ok': true,
            'build_id': 'build-24169431',
            'preview_token': token,
            'png_byte_len': pngBytes.length,
            'png_sha256': sha256,
            'width': 1,
            'height': 1,
            'format': 'PF_B8G8R8A8',
            'replaceable': false,
            'is_virtual': true,
            'vt_layers': 2,
            'mipmapped': true,
          },
          'texture_preview_release': _previewReleaseResponse(
            token,
            pngBytes,
            sha256,
          ),
        },
        handlers: {
          'texture_preview_read': (payload) {
            final offset = payload['offset']! as int;
            final candidateEnd = offset + 512 * 1024;
            final end = candidateEnd < pngBytes.length
                ? candidateEnd
                : pngBytes.length;
            return _previewReadResponse(
              token,
              pngBytes,
              offset: offset,
              end: end,
            );
          },
        },
      );
      final adapter = Revision3TextureCatalogNativeAdapter(ModFfi(core));

      final catalog = await adapter.loadCatalog(gameRoot: 'C:/Game');
      final result = await adapter.loadPreview(
        gameRoot: 'C:/Game',
        expectedSourceFingerprint: catalog.sourceFingerprint,
        texture: catalog.textures.single,
      );

      expect(result.sourceFingerprint, catalog.sourceFingerprint);
      expect(result.preview.width, 1);
      expect(result.preview.height, 1);
      expect(result.preview.virtualLayers, 2);
      expect(
        result.preview.replaceability,
        Revision3TextureReplaceability.unsupported,
      );
      expect(core.calls, hasLength(5));
      expect(core.calls[1].payload, {
        'game': 'C:/Game',
        'expected_build_id': 'build-24169431',
        'asset': '/Game/Items/T_Apple',
        'package_id': '42',
      });
      expect(core.calls[2].payload, {'preview_token': token, 'offset': 0});
      expect(core.calls[3].payload, {
        'preview_token': token,
        'offset': 512 * 1024,
      });
      expect(core.calls[4].payload, {'preview_token': token});
    },
  );

  test(
    'mismatched native preview build fails closed and releases capability',
    () async {
      final pngBytes = _pngIhdr(width: 1, height: 1);
      final token = 'b' * 64;
      final sha256 = crypto.sha256.convert(pngBytes).toString();
      final core = FakeGoreCoreFfiService(
        responses: {
          'texture_extract': {
            'ok': true,
            'build_id': 'new-build',
            'preview_token': token,
            'png_byte_len': pngBytes.length,
            'png_sha256': sha256,
            'width': 1,
            'height': 1,
            'format': 'PF_B8G8R8A8',
            'replaceable': true,
            'is_virtual': false,
            'vt_layers': 0,
            'mipmapped': false,
          },
          'texture_preview_release': _previewReleaseResponse(
            token,
            pngBytes,
            sha256,
            fullyRead: false,
          ),
        },
      );
      final adapter = Revision3TextureCatalogNativeAdapter(ModFfi(core));
      final fingerprint = Revision3TextureSourceFingerprint.nativeBuildId(
        'old-build',
      );
      final texture = Revision3TextureCatalogEntry(
        assetPath: '/Game/Items/T_Apple',
        packageId: Revision3TexturePackageId.fromDecimal('42'),
      );

      await expectLater(
        adapter.loadPreview(
          gameRoot: 'C:/Game',
          expectedSourceFingerprint: fingerprint,
          texture: texture,
        ),
        throwsFormatException,
      );
      expect(core.calls.map((call) => call.command), [
        'texture_extract',
        'texture_preview_release',
      ]);
    },
  );

  test(
    'release transport failure retries cleanup without returning bytes',
    () async {
      final pngBytes = _pngIhdr(width: 1, height: 1);
      final token = 'd' * 64;
      final sha256 = crypto.sha256.convert(pngBytes).toString();
      var releases = 0;
      final core = FakeGoreCoreFfiService(
        responses: {
          'texture_extract': {
            'ok': true,
            'build_id': 'build-a',
            'preview_token': token,
            'png_byte_len': pngBytes.length,
            'png_sha256': sha256,
            'width': 1,
            'height': 1,
            'format': 'PF_B8G8R8A8',
            'replaceable': true,
            'is_virtual': false,
            'vt_layers': 0,
            'mipmapped': false,
          },
          'texture_preview_read': _previewReadResponse(token, pngBytes),
        },
        handlers: {
          'texture_preview_release': (_) {
            releases++;
            if (releases == 1) throw StateError('release transport failed');
            return _previewReleaseResponse(token, pngBytes, sha256);
          },
        },
      );
      final adapter = Revision3TextureCatalogNativeAdapter(ModFfi(core));

      await expectLater(
        adapter.loadPreview(
          gameRoot: 'C:/Game',
          expectedSourceFingerprint:
              Revision3TextureSourceFingerprint.nativeBuildId('build-a'),
          texture: Revision3TextureCatalogEntry(
            assetPath: '/Game/Items/T_Apple',
            packageId: Revision3TexturePackageId.fromDecimal('42'),
          ),
        ),
        throwsStateError,
      );
      expect(releases, 2);
    },
  );

  test('cleanup failure never masks the original response error', () async {
    final pngBytes = _pngIhdr(width: 1, height: 1);
    final token = 'e' * 64;
    final sha256 = crypto.sha256.convert(pngBytes).toString();
    final core = FakeGoreCoreFfiService(
      responses: {
        'texture_extract': {
          'ok': true,
          'build_id': 'wrong-build',
          'preview_token': token,
          'png_byte_len': pngBytes.length,
          'png_sha256': sha256,
          'width': 1,
          'height': 1,
          'format': 'PF_B8G8R8A8',
          'replaceable': true,
          'is_virtual': false,
          'vt_layers': 0,
          'mipmapped': false,
        },
      },
      handlers: {
        'texture_preview_release': (_) =>
            throw StateError('cleanup transport failed'),
      },
    );
    final adapter = Revision3TextureCatalogNativeAdapter(ModFfi(core));

    await expectLater(
      adapter.loadPreview(
        gameRoot: 'C:/Game',
        expectedSourceFingerprint:
            Revision3TextureSourceFingerprint.nativeBuildId('build-a'),
        texture: Revision3TextureCatalogEntry(
          assetPath: '/Game/Items/T_Apple',
          packageId: Revision3TexturePackageId.fromDecimal('42'),
        ),
      ),
      throwsFormatException,
    );
  });

  test(
    'preview rejects a temp payload outside its native content seal',
    () async {
      final pngBytes = _pngIhdr(width: 1, height: 1);
      final token = 'c' * 64;
      final core = FakeGoreCoreFfiService(
        responses: {
          'texture_extract': {
            'ok': true,
            'build_id': 'build-a',
            'preview_token': token,
            'png_byte_len': pngBytes.length,
            'png_sha256': '0' * 64,
            'width': 1,
            'height': 1,
            'format': 'PF_B8G8R8A8',
            'replaceable': true,
            'is_virtual': false,
            'vt_layers': 0,
            'mipmapped': false,
          },
          'texture_preview_read': _previewReadResponse(token, pngBytes),
          'texture_preview_release': _previewReleaseResponse(
            token,
            pngBytes,
            '0' * 64,
          ),
        },
      );
      final adapter = Revision3TextureCatalogNativeAdapter(ModFfi(core));

      await expectLater(
        adapter.loadPreview(
          gameRoot: 'C:/Game',
          expectedSourceFingerprint:
              Revision3TextureSourceFingerprint.nativeBuildId('build-a'),
          texture: Revision3TextureCatalogEntry(
            assetPath: '/Game/Items/T_Apple',
            packageId: Revision3TexturePackageId.fromDecimal('42'),
          ),
        ),
        throwsFormatException,
      );
      expect(core.calls.last.command, 'texture_preview_release');
    },
  );

  test('native generation errors become a catalog-reload signal', () async {
    final core = FakeGoreCoreFfiService(
      responses: {
        'texture_extract': {
          'ok': false,
          'error': {'code': 'STALE_TEXTURE_INDEX', 'message': 'source changed'},
        },
      },
    );
    final adapter = Revision3TextureCatalogNativeAdapter(ModFfi(core));

    await expectLater(
      adapter.loadPreview(
        gameRoot: 'C:/Game',
        expectedSourceFingerprint:
            Revision3TextureSourceFingerprint.nativeBuildId('build-a'),
        texture: Revision3TextureCatalogEntry(
          assetPath: '/Game/Items/T_Apple',
          packageId: Revision3TexturePackageId.fromDecimal('42'),
        ),
      ),
      throwsA(isA<Revision3TextureSourceChangedException>()),
    );
  });
}

Map<String, Object?> _previewReadResponse(
  String token,
  Uint8List bytes, {
  int offset = 0,
  int? end,
}) {
  final nextOffset = end ?? bytes.length;
  final chunk = Uint8List.sublistView(bytes, offset, nextOffset);
  return {
    'ok': true,
    'preview_token': token,
    'offset': offset,
    'chunk_byte_len': chunk.length,
    'chunk_base64': base64Encode(chunk),
    'next_offset': nextOffset,
    'total_byte_len': bytes.length,
    'eof': nextOffset == bytes.length,
  };
}

Map<String, Object?> _previewReleaseResponse(
  String token,
  Uint8List bytes,
  String sha256, {
  bool fullyRead = true,
}) => {
  'ok': true,
  'preview_token': token,
  'released': true,
  'fully_read': fullyRead,
  'png_byte_len': bytes.length,
  'png_sha256': sha256,
};

Uint8List _pngIhdr({
  required int width,
  required int height,
  int ancillaryByteLength = 0,
}) {
  const idat = <int>[0x78, 0x9c, 0x63, 0x00, 0x00, 0x00, 0x01, 0x00, 0x01];
  final ancillaryChunkLength = ancillaryByteLength == 0
      ? 0
      : 12 + ancillaryByteLength;
  final bytes = Uint8List(33 + ancillaryChunkLength + 12 + idat.length + 12);
  bytes.setAll(0, const [137, 80, 78, 71, 13, 10, 26, 10]);
  _writeUint32(bytes, 8, 13);
  bytes.setAll(12, const [73, 72, 68, 82]);
  _writeUint32(bytes, 16, width);
  _writeUint32(bytes, 20, height);
  bytes.setAll(24, const [8, 6, 0, 0, 0]);
  _writeUint32(bytes, 29, _crc32(bytes, 12, 29));
  var offset = 33;
  if (ancillaryByteLength > 0) {
    _writeUint32(bytes, offset, ancillaryByteLength);
    bytes.setAll(offset + 4, const [116, 69, 88, 116]); // tEXt
    bytes.fillRange(offset + 8, offset + 8 + ancillaryByteLength, 0x61);
    _writeUint32(
      bytes,
      offset + 8 + ancillaryByteLength,
      _crc32(bytes, offset + 4, offset + 8 + ancillaryByteLength),
    );
    offset += 12 + ancillaryByteLength;
  }
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
