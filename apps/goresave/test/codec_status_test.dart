import 'package:flutter_test/flutter_test.dart';
import 'package:goresave/features/editor/domain/editor_models.dart';

void main() {
  test('CodecStatus parses user-facing fields', () {
    final s = CodecStatus.fromJson(const {
      'available': false,
      'status': 'unsupported',
      'message': 'techy',
      'userSeverity': 'error',
      'userTitle': "This game version can't be opened yet",
      'userMessage': 'Looks like a new game update...',
      'userHint': 'Check for an editor update...',
    });
    expect(s.userTitle, "This game version can't be opened yet");
    expect(s.userSeverity, 'error');
    expect(s.userHint, isNotEmpty);
  });

  test('CodecStatus user-facing fields default to null when absent', () {
    final s = CodecStatus.fromJson(const {
      'available': true,
      'status': 'supported',
      'message': 'ok',
    });
    expect(s.userTitle, isNull);
    expect(s.userSeverity, isNull);
  });
}
