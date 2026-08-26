import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:goresave/features/editor/domain/core_service.dart';
import 'package:goresave/features/editor/domain/editor_notifier.dart';
import 'package:goresave/features/editor/ui/skills_panel.dart';
import 'package:goresave/l10n/app_localizations.dart';

import '../../../support/l10n_test_app.dart';

/// Minimal fake core: enough for an [EditorNotifier] to settle on a save, plus
/// a canned `private.skills.list` answer.
class _SkillsCore implements GoresaveCoreService {
  _SkillsCore({required this.skills});

  final List<Map<String, Object?>> skills;

  @override
  String get description => 'skills-fake';

  @override
  bool get isAvailable => true;

  @override
  Future<Map<String, Object?>> execute(
    String command, {
    Map<String, Object?> payload = const {},
  }) async {
    switch (command) {
      case 'scan_save_dir':
        return {
          'ok': true,
          'data': {
            'saveRoot': payload['path'],
            'saves': [
              {
                'path': r'C:\tmp\saves\G1R-001.sav',
                'slot': 'G1R-001',
                'format': 'GSAV',
                'fileSize': 100,
                'sha1': 'a',
                'status': 'ok',
                'playerSaveName': 'Save A',
              },
            ],
            'profiles': <Object?>[],
          },
        };
      case 'inspect_save':
        return {
          'ok': true,
          'data': {
            'format': 'GSAV',
            'path': payload['path'],
            'slot': 'G1R-001',
            'size': 100,
            'sha1': 'a',
            'private': {
              'status': 'decoded',
              'progression': {'status': 'ok'},
              'typedParse': {'status': 'ok'},
            },
          },
        };
      case 'list_backups':
        return {
          'ok': true,
          'data': {
            'path': payload['path'],
            'backups': <Object?>[],
            'companionBackups': <Object?>[],
          },
        };
      case 'check_codec':
        return {
          'ok': true,
          'data': {
            'backend': 'kraken',
            'available': true,
            'canDecompress': true,
            'canCompress': true,
            'status': 'ready',
          },
        };
      case 'private.skills.list':
        return {
          'ok': true,
          'data': {'actor': 'Hero', 'found': true, 'skills': skills},
        };
      default:
        return {
          'ok': true,
          'data': {
            'total': 0,
            'offset': 0,
            'limit': 50,
            'characters': <Object?>[],
            'entries': <Object?>[],
          },
        };
    }
  }
}

Map<String, Object?> _skill(
  String base,
  String label,
  String kind,
  bool learned,
  String current,
  List<String> options, {
  String category = 'Hunting',
}) => {
  'base': base,
  'label': label,
  'category': category,
  'kind': kind,
  'learned': learned,
  'current': current,
  'hasUntrained': false,
  'options': [
    for (final option in options) {'value': option},
  ],
};

Future<EditorNotifier> _notifier(WidgetTester tester, _SkillsCore core) async {
  final notifier = EditorNotifier(core, saveDir: r'C:\tmp\saves');
  // inspect() schedules real timers the widget-test clock will not advance.
  await tester.runAsync(() async {
    await notifier.inspect(r'C:\tmp\saves\G1R-001.sav');
  });
  return notifier;
}

Widget _wrap(Widget child) => ProviderScope(
  child: MaterialApp(
    locale: const Locale('en'),
    localizationsDelegates: testLocalizationsDelegates,
    supportedLocales: AppLocalizations.supportedLocales,
    home: Scaffold(body: SizedBox(width: 1000, height: 700, child: child)),
  ),
);

/// The panel loads through the notifier's core queue — a real Future the
/// widget-test clock does not advance on its own.
Future<void> _settle(WidgetTester tester) async {
  for (var i = 0; i < 12; i++) {
    await tester.runAsync(
      () => Future<void>.delayed(const Duration(milliseconds: 10)),
    );
    await tester.pump();
  }
}

void main() {
  // Take Scutes is ranked: Cavalorn's second lesson is the Master rung, and it
  // has to be reachable — the shape this skill used to have could only ever
  // write Trained, which costs a hero the razor plates.
  testWidgets('the scutes ladder can be raised to Master', (tester) async {
    final core = _SkillsCore(
      skills: [
        _skill('Hunting_Scutes', 'Take Scutes', 'ladder', true, 'Trained', [
          'Untrained',
          'Trained',
          'Master',
        ]),
      ],
    );
    final notifier = await _notifier(tester, core);
    await tester.pumpWidget(
      _wrap(SkillsSection(notifier: notifier, editable: true, reloadKey: 'k')),
    );
    await _settle(tester);

    expect(find.text('Take Scutes'), findsOneWidget);
    await tester.tap(find.byType(DropdownButton<String>).first);
    await tester.pumpAndSettle();
    // Both rungs are named after what they yield: the game gives them none.
    await tester.tap(find.text('Master (+ razor plates)').last);
    await tester.pumpAndSettle();

    final pending = notifier.pendingEditFor('skills');
    expect(pending, isNotNull);
    expect(pending!.edits.single, {
      'path': 'private.skills.set',
      'value': {'actor': 'Hero', 'base': 'Hunting_Scutes', 'tier': 'Master'},
    });
  });

  // Whatever a save carries has to be removable, including a class the core
  // does not catalogue — a skill the game never grants, a console `addskill`,
  // or one from a newer game version. It arrives as an "Other" row.
  testWidgets('an uncatalogued skill can be dropped', (tester) async {
    final core = _SkillsCore(
      skills: [
        _skill(
          'Hunting_MandibleMineCrawler',
          'Hunting MandibleMineCrawler',
          'ladder',
          true,
          'Trained',
          ['Trained', 'Untrained'],
          category: 'Other',
        ),
      ],
    );
    final notifier = await _notifier(tester, core);
    await tester.pumpWidget(
      _wrap(SkillsSection(notifier: notifier, editable: true, reloadKey: 'k')),
    );
    await _settle(tester);

    expect(find.text('Other'), findsOneWidget);
    await tester.tap(find.byType(DropdownButton<String>).first);
    await tester.pumpAndSettle();
    await tester.tap(find.text('Untrained').last);
    await tester.pumpAndSettle();

    final pending = notifier.pendingEditFor('skills');
    expect(pending, isNotNull);
    expect(pending!.edits.single, {
      'path': 'private.skills.set',
      'value': {
        'actor': 'Hero',
        'base': 'Hunting_MandibleMineCrawler',
        'tier': 'Untrained',
      },
    });
  });
}
