import 'dart:convert';

import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/core/core_service.dart';

String _coreInfoResponse({
  int abi = goreCoreAbi,
  String version = '0.1.0-test',
  List<String> commands = requiredStudioCoreCommands,
}) => jsonEncode({
  'ok': true,
  'abi': abi,
  'version': version,
  'commands': commands,
});

void main() {
  group('GoreCoreInfo', () {
    test('strictly parses a canonical bounded response', () {
      final info = GoreCoreInfo.parseResponse(_coreInfoResponse());

      expect(info.abi, goreCoreAbi);
      expect(info.version, '0.1.0-test');
      expect(info.commands, requiredStudioCoreCommands);
      expect(info.isStudioCompatible, isTrue);
      expect(info.missingRequiredCommands, isEmpty);
      expect(() => info.commands.add('validate'), throwsUnsupportedError);
    });

    test(
      'candidate decision skips legacy, wrong ABI, and incomplete cores',
      () {
        final legacy = jsonEncode({
          'ok': false,
          'error': {
            'code': 'UNKNOWN_COMMAND',
            'message': 'unknown command: core_info',
          },
        });
        final wrongAbi = _coreInfoResponse(abi: goreCoreAbi + 1);
        final missingAuthoring = _coreInfoResponse(
          commands: requiredStudioCoreCommands
              .where((command) => command != 'authoring_project_check')
              .toList(),
        );
        final missingVoice = _coreInfoResponse(
          commands: requiredStudioCoreCommands
              .where((command) => command != 'voice_archive_match_line')
              .toList(),
        );
        final current = _coreInfoResponse(version: '0.2.0-current');

        final decisions = [
          legacy,
          wrongAbi,
          missingAuthoring,
          missingVoice,
          current,
        ].map(GoreCoreInfo.tryParseCompatibleResponse).toList();
        expect(decisions.take(4), everyElement(isNull));
        expect(decisions.last?.version, '0.2.0-current');
      },
    );

    test('reports every missing Studio capability deterministically', () {
      final commands = requiredStudioCoreCommands
          .where(
            (command) =>
                command != 'authoring_project_check' &&
                command != 'voice_archive_match_line',
          )
          .toList();
      final info = GoreCoreInfo.parseResponse(
        _coreInfoResponse(commands: commands),
      );

      expect(info.isStudioCompatible, isFalse);
      expect(info.missingRequiredCommands, [
        'authoring_project_check',
        'voice_archive_match_line',
      ]);
    });

    test('rejects non-exact, non-canonical, and oversized responses', () {
      final unsortedCommands = requiredStudioCoreCommands.reversed.toList();
      final duplicateCommands = [...requiredStudioCoreCommands]
        ..insert(1, requiredStudioCoreCommands.first);
      final cases = <String>[
        '[]',
        '{',
        jsonEncode({
          'ok': true,
          'abi': goreCoreAbi,
          'version': '0.1.0',
          'commands': requiredStudioCoreCommands,
          'extra': true,
        }),
        jsonEncode({
          'ok': true,
          'abi': 1.0,
          'version': '0.1.0',
          'commands': requiredStudioCoreCommands,
        }),
        _coreInfoResponse(version: 'bad version'),
        _coreInfoResponse(commands: unsortedCommands),
        _coreInfoResponse(commands: duplicateCommands),
        _coreInfoResponse(commands: const ['Not_Canonical']),
        ' ' * (64 * 1024 + 1),
      ];

      for (final response in cases) {
        expect(
          () => GoreCoreInfo.parseResponse(response),
          throwsFormatException,
          reason: response.length < 100 ? response : 'bounded response',
        );
        expect(GoreCoreInfo.tryParseCompatibleResponse(response), isNull);
      }
    });
  });

  test(
    'FakeGoreCoreFfiService records calls and returns canned response',
    () async {
      final fake = FakeGoreCoreFfiService(
        responses: {
          'validate_override': {'ok': true, 'data': {}},
        },
      );
      final result = await fake.execute(
        'validate_override',
        payload: {'class': 'ItFo_Apple', 'field': 'm_Value', 'value': 500},
      );
      expect(result['ok'], isTrue);
      expect(fake.calls, hasLength(1));
      expect(fake.calls.first.command, 'validate_override');
      expect(fake.calls.first.payload['class'], 'ItFo_Apple');
    },
  );

  test('MissingGoreCoreFfiService returns CORE_UNAVAILABLE', () async {
    final svc = MissingGoreCoreFfiService();
    final result = await svc.execute('validate_override');
    expect(result['ok'], isFalse);
    final err = result['error'] as Map<String, Object?>;
    expect(err['code'], 'CORE_UNAVAILABLE');
  });
}
