import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/project/project_model.dart';

void main() {
  test('format-1 model JSON still round-trips', () {
    final project = ModProject(
      name: 'Format fixture',
      version: '1.2.3',
      author: 'tester',
      delayMs: 25,
      locEdits: const {
        'info_fixture': {'german_new': 'Hallo'},
      },
    );

    final json = project.toJson();
    final reopened = ModProject.fromJson(json);

    expect(json['format'], 1);
    expect(reopened.toJson(), json);
  });

  test('missing format fails before any project fields are decoded', () {
    expect(
      () => ModProject.fromJson({'mod': 42}),
      throwsA(
        isA<FormatException>().having(
          (error) => error.message,
          'message',
          'missing project format; expected integer 1',
        ),
      ),
    );
  });

  test('non-integer formats fail before any project fields are decoded', () {
    for (final format in <Object?>['1', 1.0, true, null]) {
      expect(
        () => ModProject.fromJson({'format': format, 'mod': 42}),
        throwsA(
          isA<FormatException>().having(
            (error) => error.message,
            'message',
            'project format must be the integer 1',
          ),
        ),
        reason: 'format $format should be rejected',
      );
    }
  });

  test(
    'unknown integer format fails before any project fields are decoded',
    () {
      expect(
        () => ModProject.fromJson({'format': 2, 'mod': 42}),
        throwsA(
          isA<FormatException>().having(
            (error) => error.message,
            'message',
            'unsupported project format 2; expected 1',
          ),
        ),
      );
    },
  );
}
