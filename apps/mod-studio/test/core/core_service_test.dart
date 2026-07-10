import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/core/core_service.dart';

void main() {
  test('FakeGoreCoreFfiService records calls and returns canned response', () async {
    final fake = FakeGoreCoreFfiService(responses: {
      'validate_override': {'ok': true, 'data': {}},
    });
    final result = await fake.execute(
      'validate_override',
      payload: {'class': 'ItFo_Apple', 'field': 'm_Value', 'value': 500},
    );
    expect(result['ok'], isTrue);
    expect(fake.calls, hasLength(1));
    expect(fake.calls.first.command, 'validate_override');
    expect(fake.calls.first.payload['class'], 'ItFo_Apple');
  });

  test('MissingGoreCoreFfiService returns CORE_UNAVAILABLE', () async {
    final svc = MissingGoreCoreFfiService();
    final result = await svc.execute('validate_override');
    expect(result['ok'], isFalse);
    final err = result['error'] as Map<String, Object?>;
    expect(err['code'], 'CORE_UNAVAILABLE');
  });
}
