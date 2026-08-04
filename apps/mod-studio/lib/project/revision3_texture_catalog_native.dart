import 'dart:convert';
import 'dart:isolate';
import 'dart:typed_data';

import 'package:crypto/crypto.dart' as crypto;

import '../core/mod_ffi.dart';
import 'revision3_texture_catalog.dart';

/// Native/file adapter for the Managed inspect-only texture catalog.
///
/// Every preview is bound to the build fingerprint returned atomically with
/// its index. Preview bytes stay behind a bounded native capability and are
/// streamed without exposing a temporary filesystem path.
final class Revision3TextureCatalogNativeAdapter {
  const Revision3TextureCatalogNativeAdapter(this._ffi);

  final ModFfi _ffi;

  Future<Revision3TextureCatalogSnapshot> loadCatalog({
    required String gameRoot,
  }) async {
    final native = await _ffi.textureIndex(gameRoot);
    return Revision3TextureCatalogSnapshot.fromInstalledIndex(
      sourceFingerprint: Revision3TextureSourceFingerprint.nativeBuildId(
        native.buildId,
      ),
      index: native.entries,
    );
  }

  Future<Revision3TexturePreviewResult> loadPreview({
    required String gameRoot,
    required Revision3TextureSourceFingerprint expectedSourceFingerprint,
    required Revision3TextureCatalogEntry texture,
  }) async {
    final Map<String, Object?> response;
    try {
      response = await _ffi.textureExtract(
        gameRoot,
        expectedBuildId: expectedSourceFingerprint.value,
        asset: texture.assetPath,
        packageId: texture.packageId.decimal,
      );
    } on ModFfiException catch (error) {
      if (error.code == 'STALE_TEXTURE_INDEX' ||
          error.code == 'GENERATION_CHANGED' ||
          error.code == 'INDEX_REQUIRED' ||
          error.code == 'SOURCE_FINGERPRINT') {
        throw Revision3TextureSourceChangedException(error.code);
      }
      rethrow;
    }
    final previewToken = _requiredPreviewToken(response, 'preview_token');
    var releaseConfirmed = false;
    var expectedByteLength = 0;
    var expectedSha256 = '';
    Revision3TexturePreviewResult? result;
    Object? primaryError;
    StackTrace? primaryStackTrace;
    try {
      _requireExactFields(response, const {
        'ok',
        'build_id',
        'preview_token',
        'png_byte_len',
        'png_sha256',
        'width',
        'height',
        'format',
        'replaceable',
        'is_virtual',
        'vt_layers',
        'mipmapped',
      }, 'texture_extract');
      if (_requiredString(response, 'build_id') !=
          expectedSourceFingerprint.value) {
        throw const FormatException(
          'texture_extract returned a different native build ID',
        );
      }
      expectedByteLength = _requiredExactInt(response, 'png_byte_len');
      Revision3TexturePreview.validatePngByteLength(expectedByteLength);
      expectedSha256 = _requiredSha256(response, 'png_sha256');
      final width = _requiredExactInt(response, 'width');
      final height = _requiredExactInt(response, 'height');
      final pixelFormat = _requiredString(response, 'format', allowEmpty: true);
      final isVirtual = _requiredBool(response, 'is_virtual');
      final virtualLayers = _requiredExactInt(response, 'vt_layers');
      if (virtualLayers < 0 ||
          virtualLayers > Revision3TexturePreview.maximumVirtualLayers ||
          (isVirtual ? virtualLayers == 0 : virtualLayers != 0)) {
        throw const FormatException(
          'texture_extract returned inconsistent virtual texture facts',
        );
      }
      final mipmapped = _requiredBool(response, 'mipmapped');
      final replaceability = _requiredBool(response, 'replaceable')
          ? Revision3TextureReplaceability.supported
          : Revision3TextureReplaceability.unsupported;
      final bytes = await _readNativePreview(
        previewToken: previewToken,
        expectedByteLength: expectedByteLength,
      );
      final released = await _ffi.texturePreviewRelease(
        previewToken: previewToken,
      );
      _validateRelease(
        released,
        previewToken: previewToken,
        expectedByteLength: expectedByteLength,
        expectedSha256: expectedSha256,
        requireFullyRead: true,
      );
      releaseConfirmed = true;
      final preview = await Isolate.run(
        () => _validateAndBuildPreview(
          bytes: bytes,
          expectedSha256: expectedSha256,
          width: width,
          height: height,
          pixelFormat: pixelFormat,
          isVirtual: isVirtual,
          virtualLayers: virtualLayers,
          mipmapped: mipmapped,
          replaceability: replaceability,
        ),
      );
      result = Revision3TexturePreviewResult(
        sourceFingerprint: expectedSourceFingerprint,
        preview: preview,
      );
    } catch (error, stackTrace) {
      primaryError = error;
      primaryStackTrace = stackTrace;
    }
    if (!releaseConfirmed) {
      try {
        final released = await _ffi.texturePreviewRelease(
          previewToken: previewToken,
        );
        _validateRelease(
          released,
          previewToken: previewToken,
          expectedByteLength: expectedByteLength,
          expectedSha256: expectedSha256,
          requireFullyRead: false,
        );
      } catch (cleanupError, cleanupStackTrace) {
        primaryError ??= cleanupError;
        primaryStackTrace ??= cleanupStackTrace;
      }
    }
    if (primaryError != null) {
      Error.throwWithStackTrace(primaryError, primaryStackTrace!);
    }
    return result!;
  }

  Future<Uint8List> _readNativePreview({
    required String previewToken,
    required int expectedByteLength,
  }) async {
    const maximumChunkBytes = 512 * 1024;
    const maximumEncodedChunkCodeUnits = 699052;
    final decoder = await _PreviewDecodeWorker.start(expectedByteLength);
    var offset = 0;
    try {
      while (offset < expectedByteLength) {
        final response = await _ffi.texturePreviewRead(
          previewToken: previewToken,
          offset: offset,
        );
        _requireExactFields(response, const {
          'ok',
          'preview_token',
          'offset',
          'chunk_byte_len',
          'chunk_base64',
          'next_offset',
          'total_byte_len',
          'eof',
        }, 'texture_preview_read');
        if (_requiredPreviewToken(response, 'preview_token') != previewToken ||
            _requiredExactInt(response, 'offset') != offset ||
            _requiredExactInt(response, 'total_byte_len') !=
                expectedByteLength) {
          throw const FormatException(
            'texture_preview_read returned a different preview identity',
          );
        }
        final chunkByteLength = _requiredExactInt(response, 'chunk_byte_len');
        final nextOffset = _requiredExactInt(response, 'next_offset');
        final encoded = _requiredBase64Chunk(response, 'chunk_base64');
        if (chunkByteLength < 1 ||
            chunkByteLength > maximumChunkBytes ||
            encoded.length > maximumEncodedChunkCodeUnits ||
            nextOffset != offset + chunkByteLength ||
            nextOffset > expectedByteLength) {
          throw const FormatException(
            'texture_preview_read returned an invalid bounded chunk',
          );
        }
        final eof = _requiredBool(response, 'eof');
        if (eof != (nextOffset == expectedByteLength)) {
          throw const FormatException(
            'texture_preview_read returned an invalid terminal marker',
          );
        }
        await decoder.addChunk(encoded, chunkByteLength);
        offset = nextOffset;
      }
      return await decoder.finish();
    } finally {
      decoder.dispose();
    }
  }
}

Uint8List _decodeCanonicalBase64Chunk(String encoded, int expectedByteLength) {
  final Uint8List bytes;
  try {
    bytes = base64Decode(encoded);
  } on FormatException {
    throw const FormatException('texture_preview_read returned invalid base64');
  }
  if (bytes.length != expectedByteLength || base64Encode(bytes) != encoded) {
    throw const FormatException(
      'texture_preview_read returned noncanonical chunk bytes',
    );
  }
  return bytes;
}

final class _PreviewDecodeWorker {
  _PreviewDecodeWorker._(this._isolate, this._commands);

  final Isolate _isolate;
  final SendPort _commands;
  var _finished = false;

  static Future<_PreviewDecodeWorker> start(int expectedByteLength) async {
    final ready = ReceivePort();
    final isolate = await Isolate.spawn(_previewDecodeWorkerMain, (
      ready.sendPort,
      expectedByteLength,
    ));
    try {
      final commands = await ready.first;
      if (commands is! SendPort) {
        throw const FormatException('texture preview decoder failed to start');
      }
      return _PreviewDecodeWorker._(isolate, commands);
    } catch (_) {
      isolate.kill(priority: Isolate.immediate);
      rethrow;
    } finally {
      ready.close();
    }
  }

  Future<void> addChunk(String encoded, int expectedByteLength) async {
    if (_finished) {
      throw StateError('texture preview decoder is already finished');
    }
    final reply = ReceivePort();
    try {
      _commands.send(('chunk', encoded, expectedByteLength, reply.sendPort));
      final result = await reply.first;
      if (result is String) throw FormatException(result);
      if (result != null) {
        throw const FormatException(
          'texture preview decoder returned an invalid response',
        );
      }
    } finally {
      reply.close();
    }
  }

  Future<Uint8List> finish() async {
    if (_finished) {
      throw StateError('texture preview decoder is already finished');
    }
    _finished = true;
    final reply = ReceivePort();
    try {
      _commands.send(('finish', reply.sendPort));
      final result = await reply.first;
      if (result is String) throw FormatException(result);
      if (result is! TransferableTypedData) {
        throw const FormatException(
          'texture preview decoder returned an invalid terminal response',
        );
      }
      return result.materialize().asUint8List();
    } finally {
      reply.close();
    }
  }

  void dispose() => _isolate.kill(priority: Isolate.immediate);
}

void _previewDecodeWorkerMain((SendPort, int) initialization) {
  final commands = ReceivePort();
  final output = BytesBuilder(copy: false);
  var decodedByteLength = 0;
  initialization.$1.send(commands.sendPort);
  commands.listen((message) {
    if (message case (
      'chunk',
      final String encoded,
      final int expectedChunkByteLength,
      final SendPort reply,
    )) {
      try {
        final chunk = _decodeCanonicalBase64Chunk(
          encoded,
          expectedChunkByteLength,
        );
        if (chunk.length > initialization.$2 - decodedByteLength) {
          throw const FormatException(
            'texture preview decoder exceeded its native length seal',
          );
        }
        output.add(chunk);
        decodedByteLength += chunk.length;
        reply.send(null);
      } on FormatException catch (error) {
        reply.send(error.message);
      }
      return;
    }
    if (message case ('finish', final SendPort reply)) {
      if (decodedByteLength != initialization.$2) {
        reply.send(
          'texture preview byte stream did not match its native length seal',
        );
      } else {
        reply.send(TransferableTypedData.fromList([output.takeBytes()]));
      }
      commands.close();
    }
  });
}

Revision3TexturePreview _validateAndBuildPreview({
  required Uint8List bytes,
  required String expectedSha256,
  required int width,
  required int height,
  required String pixelFormat,
  required bool isVirtual,
  required int virtualLayers,
  required bool mipmapped,
  required Revision3TextureReplaceability replaceability,
}) {
  if (crypto.sha256.convert(bytes).toString() != expectedSha256) {
    throw const FormatException(
      'texture preview does not match its native content seal',
    );
  }
  return Revision3TexturePreview(
    pngBytes: bytes,
    width: width,
    height: height,
    pixelFormat: pixelFormat,
    isVirtual: isVirtual,
    virtualLayers: virtualLayers,
    mipmapped: mipmapped,
    replaceability: replaceability,
  );
}

void _validateRelease(
  Map<String, Object?> response, {
  required String previewToken,
  required int expectedByteLength,
  required String expectedSha256,
  required bool requireFullyRead,
}) {
  _requireExactFields(response, const {
    'ok',
    'preview_token',
    'released',
    'fully_read',
    'png_byte_len',
    'png_sha256',
  }, 'texture_preview_release');
  if (_requiredPreviewToken(response, 'preview_token') != previewToken ||
      _requiredBool(response, 'released') != true) {
    throw const FormatException(
      'texture_preview_release returned a different preview identity',
    );
  }
  if (expectedByteLength > 0 &&
      _requiredExactInt(response, 'png_byte_len') != expectedByteLength) {
    throw const FormatException(
      'texture_preview_release returned a different byte seal',
    );
  }
  if (expectedSha256.isNotEmpty &&
      _requiredSha256(response, 'png_sha256') != expectedSha256) {
    throw const FormatException(
      'texture_preview_release returned a different content seal',
    );
  }
  if (requireFullyRead && !_requiredBool(response, 'fully_read')) {
    throw const FormatException(
      'texture_preview_release did not confirm a complete native read',
    );
  }
}

void _requireExactFields(
  Map<String, Object?> response,
  Set<String> fields,
  String command,
) {
  if (response.length != fields.length || !fields.every(response.containsKey)) {
    throw FormatException('$command returned an invalid response schema');
  }
}

String _requiredString(
  Map<String, Object?> response,
  String field, {
  bool allowEmpty = false,
}) {
  final value = response[field];
  if (value is! String ||
      (!allowEmpty && value.isEmpty) ||
      value != value.trim() ||
      value.length > 32768 ||
      value.codeUnits.any((codeUnit) => codeUnit < 0x20 || codeUnit == 0x7f)) {
    throw FormatException('texture_extract returned an invalid $field');
  }
  return value;
}

int _requiredExactInt(Map<String, Object?> response, String field) {
  final value = response[field];
  if (value is! int) {
    throw FormatException('texture_extract returned an invalid $field');
  }
  return value;
}

String _requiredSha256(Map<String, Object?> response, String field) {
  final value = response[field];
  if (value is! String ||
      value.length != 64 ||
      !RegExp(r'^[0-9a-f]{64}$').hasMatch(value)) {
    throw FormatException('texture_extract returned an invalid $field');
  }
  return value;
}

String _requiredPreviewToken(Map<String, Object?> response, String field) {
  final value = response[field];
  if (value is! String || !RegExp(r'^[0-9a-f]{64}$').hasMatch(value)) {
    throw FormatException('texture preview returned an invalid $field');
  }
  return value;
}

String _requiredBase64Chunk(Map<String, Object?> response, String field) {
  const maximumEncodedChunkCodeUnits = 699052;
  final value = response[field];
  if (value is! String ||
      value.isEmpty ||
      value.length > maximumEncodedChunkCodeUnits ||
      value.length % 4 != 0) {
    throw FormatException('texture preview returned an invalid $field');
  }
  return value;
}

bool _requiredBool(Map<String, Object?> response, String field) {
  final value = response[field];
  if (value is! bool) {
    throw FormatException('texture_extract returned an invalid $field');
  }
  return value;
}
