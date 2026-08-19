import 'package:flutter/material.dart';
import 'package:flutter_localizations/flutter_localizations.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gore_manager/app/domain/ui_settings.dart';
import 'package:gore_manager/core/core_service.dart';
import 'package:gore_manager/core/diagnostic_text.dart';
import 'package:gore_manager/core/providers.dart';
import 'package:gore_manager/l10n/app_localizations.dart';
import 'package:gore_manager/library/ui/detail_panel.dart';
import 'package:gore_manager/library/ui/mod_list.dart';

/// The Windows extended-length marker Native canonicalizes paths with.
const _marker = '\\\\?\\';

Map<String, Object?> _library() => {
  'ok': true,
  'mods': [
    {
      'id': 'Old.Sound',
      'kind': 'foreign_rawfile',
      'name': 'Old Level Up Sound',
      'source': '${_marker}C:\\Mods\\Old Level Up Sound.zip',
      'imported_at': '2026-08-13T13:53:55.883181Z',
      'components': [
        {'type': 'raw_file', 'rel': 'raw/SFX.bank', 'target_file': 'lcache'},
      ],
    },
  ],
  'loadout': {
    'format': 1,
    'entries': [
      {'id': 'Old.Sound', 'enabled': false},
    ],
  },
};

Widget _app(Widget child) => ProviderScope(
  overrides: [
    coreServiceProvider.overrideWithValue(
      FakeGoreCoreFfiService(
        responses: {
          'mgr_library_list': _library(),
          'mgr_analyze': {'ok': true, 'conflicts': <Object?>[]},
        },
      ),
    ),
  ],
  child: MaterialApp(
    localizationsDelegates: const [
      AppLocalizations.delegate,
      GlobalMaterialLocalizations.delegate,
      GlobalWidgetsLocalizations.delegate,
      GlobalCupertinoLocalizations.delegate,
    ],
    supportedLocales: AppLocalizations.supportedLocales,
    home: Scaffold(body: child),
  ),
);

Future<MaterialLocalizations> _material(WidgetTester tester) async {
  late MaterialLocalizations material;
  await tester.pumpWidget(
    _app(
      Builder(
        builder: (context) {
          material = MaterialLocalizations.of(context);
          return const SizedBox.shrink();
        },
      ),
    ),
  );
  return material;
}

void main() {
  group('displayPath', () {
    test('drops the extended-length marker and keeps the path', () {
      expect(displayPath('${_marker}C:\\Games\\Gothic'), 'C:\\Games\\Gothic');
      // A UNC path keeps its leading pair; only the marker goes.
      expect(
        displayPath('\\\\?\\UNC\\server\\share\\file'),
        '\\\\server\\share\\file',
      );
    });

    test('leaves plain paths alone', () {
      expect(displayPath('C:\\Games\\Gothic'), 'C:\\Games\\Gothic');
      expect(displayPath('/usr/share/gothic'), '/usr/share/gothic');
      expect(displayPath(''), '');
    });
  });

  group('formatImportedAt', () {
    testWidgets('turns a machine timestamp into a readable date', (
      tester,
    ) async {
      final material = await _material(tester);
      final shown = formatImportedAt(
        material,
        '2026-08-13T13:53:55.883181Z',
        alwaysUse24HourFormat: true,
      );
      // The day is what a reader wants; the microseconds are machine noise.
      expect(shown, contains('2026'));
      expect(shown, isNot(contains('883181')));
      expect(shown, isNot(contains('T')));
    });

    testWidgets('passes an unparsable value through untouched', (tester) async {
      final material = await _material(tester);
      expect(
        formatImportedAt(material, 'someday', alwaysUse24HourFormat: true),
        'someday',
      );
    });
  });

  testWidgets('the detail pane shows a readable date and a clean source', (
    tester,
  ) async {
    await tester.pumpWidget(_app(const DetailPanel()));
    await tester.pumpAndSettle();

    final container = ProviderScope.containerOf(
      tester.element(find.byType(DetailPanel)),
    );
    container.read(advancedDetailsProvider.notifier).set(true);
    container.read(selectedModProvider.notifier).state = 'Old.Sound';
    await tester.pumpAndSettle();

    expect(find.textContaining('883181'), findsNothing);
    expect(find.textContaining(_marker), findsNothing);
    expect(find.textContaining('Old Level Up Sound.zip'), findsOneWidget);
  });

  testWidgets('a disabled mod is not also labelled disabled', (tester) async {
    await tester.pumpWidget(_app(const ModList()));
    await tester.pumpAndSettle();

    final l10n = await AppLocalizations.delegate.load(const Locale('en'));
    expect(find.text('Old Level Up Sound'), findsOneWidget);
    // The unticked checkbox beside it already says so.
    expect(tester.widget<Checkbox>(find.byType(Checkbox)).value, isFalse);
    expect(find.text(l10n.modDisabledHint), findsNothing);
  });
}
