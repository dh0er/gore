import 'package:flutter_test/flutter_test.dart';
import 'package:goresave/features/editor/domain/editor_models.dart';

void main() {
  test('CodecStatus parses the in-process codec shape', () {
    final s = CodecStatus.fromJson(const {
      'backend': 'ooz_kraken',
      'available': true,
      'canDecompress': true,
      'canCompress': true,
      'status': 'ready',
      'details': {'adapter': 'ooz_kraken'},
    });
    expect(s.backend, 'ooz_kraken');
    expect(s.available, isTrue);
    expect(s.canDecompress, isTrue);
    expect(s.canCompress, isTrue);
    expect(s.status, 'ready');
    expect(s.adapter, 'ooz_kraken');
  });

  test('CodecStatus tolerates a decode_only / missing-details payload', () {
    final s = CodecStatus.fromJson(const {
      'backend': 'ooz_kraken',
      'available': true,
      'canDecompress': true,
      'canCompress': false,
      'status': 'decode_only',
    });
    expect(s.status, 'decode_only');
    expect(s.canDecompress, isTrue);
    expect(s.canCompress, isFalse);
    expect(s.adapter, isNull);
  });

  test('CodecStatus defaults fields when keys are absent', () {
    final s = CodecStatus.fromJson(const {});
    expect(s.backend, 'unknown');
    expect(s.available, isFalse);
    expect(s.status, 'unknown');
    expect(s.canDecompress, isFalse);
    expect(s.canCompress, isFalse);
    expect(s.adapter, isNull);
  });
}
