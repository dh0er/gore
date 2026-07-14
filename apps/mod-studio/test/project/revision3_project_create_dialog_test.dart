import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/l10n/app_localizations.dart';
import 'package:gore_mod/project/revision3_project_create_dialog.dart';

void main() {
  test('injected prompt results are publicly constructible and immutable', () {
    final sourceLocales = <String>['de', 'en-US'];
    final result = Revision3ProjectCreateFormResult(
      name: 'Asghan Expanded',
      version: '0.1.0',
      author: 'Gore Team',
      authoringLocales: sourceLocales,
    );

    sourceLocales.add('fr');
    expect(result.authoringLocales, const <String>['de', 'en-US']);
    expect(() => result.authoringLocales.add('fr'), throwsUnsupportedError);
  });

  testWidgets(
    'shows safe offline scope, stable controls, and friendly defaults',
    (tester) async {
      final capture = _DialogCapture();
      await _openDialog(tester, capture);

      expect(
        find.byKey(const Key('revision3-project-create-dialog')),
        findsOne,
      );
      expect(
        find.byKey(const Key('revision3-project-create-boundary')),
        findsOne,
      );
      expect(find.textContaining('empty managed offline project'), findsOne);
      expect(find.textContaining('does not build'), findsOne);
      expect(find.textContaining('game files or save files'), findsOne);
      expect(_fieldText(tester, 'revision3-project-create-name'), isEmpty);
      expect(_fieldText(tester, 'revision3-project-create-version'), '0.1.0');
      expect(_fieldText(tester, 'revision3-project-create-author'), isEmpty);
      expect(_fieldText(tester, 'revision3-project-create-locales'), 'en');
      expect(
        find.byKey(const Key('revision3-project-create-cancel')),
        findsOne,
      );
      expect(
        find.byKey(const Key('revision3-project-create-submit')),
        findsOne,
      );

      await tester.tap(
        find.byKey(const Key('revision3-project-create-cancel')),
      );
      await tester.pumpAndSettle();
      expect(capture.completed, isTrue);
      expect(capture.result, isNull);
    },
  );

  testWidgets(
    'returns immutable metadata with sorted unique canonical locales',
    (tester) async {
      final capture = _DialogCapture();
      await _openDialog(tester, capture);

      await tester.enterText(
        find.byKey(const Key('revision3-project-create-name')),
        'My Story Mod',
      );
      await tester.enterText(
        find.byKey(const Key('revision3-project-create-author')),
        'Gore Team',
      );
      await tester.enterText(
        find.byKey(const Key('revision3-project-create-locales')),
        'zh-Hans, de, en-US, de, sl-rozaj-biske-1994',
      );
      await tester.tap(
        find.byKey(const Key('revision3-project-create-submit')),
      );
      await tester.pumpAndSettle();

      final result = capture.result!;
      expect(result.name, 'My Story Mod');
      expect(result.version, '0.1.0');
      expect(result.author, 'Gore Team');
      expect(result.authoringLocales, const <String>[
        'de',
        'en-US',
        'sl-rozaj-biske-1994',
        'zh-Hans',
      ]);
      expect(() => result.authoringLocales.add('fr'), throwsUnsupportedError);
    },
  );

  testWidgets('metadata validators mirror bootstrap UTF-8 and trim bounds', (
    tester,
  ) async {
    final capture = _DialogCapture();
    await _openDialog(tester, capture);

    final name = _field(tester, 'revision3-project-create-name');
    final version = _field(tester, 'revision3-project-create-version');
    final author = _field(tester, 'revision3-project-create-author');
    expect(name.validator!(''), isNotNull);
    expect(name.validator!(' Project'), contains('whitespace'));
    expect(name.validator!('Project '), contains('whitespace'));
    expect(name.validator!('bad\nname'), contains('control'));
    expect(name.validator!(_repeat(String.fromCharCode(0xe9), 128)), isNull);
    expect(
      name.validator!(_repeat(String.fromCharCode(0xe9), 129)),
      contains('256-byte'),
    );
    expect(name.validator!(String.fromCharCode(0xd800)), contains('malformed'));
    expect(version.validator!(_repeat('x', 128)), isNull);
    expect(version.validator!(_repeat('x', 129)), contains('128-byte'));
    expect(author.validator!(_repeat(String.fromCharCode(0xe9), 128)), isNull);
    expect(
      author.validator!(_repeat(String.fromCharCode(0xe9), 129)),
      contains('256-byte'),
    );

    await tester.enterText(
      find.byKey(const Key('revision3-project-create-author')),
      'Author',
    );
    await tester.tap(find.byKey(const Key('revision3-project-create-submit')));
    await tester.pump();
    expect(find.byKey(const Key('revision3-project-create-dialog')), findsOne);
    expect(find.textContaining('Project name'), findsWidgets);
    expect(capture.completed, isFalse);
  });

  testWidgets('exact UTF-8 metadata limits remain submittable', (tester) async {
    final capture = _DialogCapture();
    await _openDialog(tester, capture);

    final exactName = _repeat(String.fromCharCode(0xe9), 128);
    final exactVersion = _repeat('x', 128);
    final exactAuthor = _repeat(String.fromCharCode(0xe9), 128);
    await tester.enterText(
      find.byKey(const Key('revision3-project-create-name')),
      exactName,
    );
    await tester.enterText(
      find.byKey(const Key('revision3-project-create-version')),
      exactVersion,
    );
    await tester.enterText(
      find.byKey(const Key('revision3-project-create-author')),
      exactAuthor,
    );
    await tester.tap(find.byKey(const Key('revision3-project-create-submit')));
    await tester.pumpAndSettle();

    expect(capture.result?.name, exactName);
    expect(capture.result?.version, exactVersion);
    expect(capture.result?.author, exactAuthor);
  });

  testWidgets('locale validation matches the canonical Rust BCP-47 subset', (
    tester,
  ) async {
    final capture = _DialogCapture();
    await _openDialog(tester, capture);

    final validator = _field(
      tester,
      'revision3-project-create-locales',
    ).validator!;
    expect(validator('de, en-US, zh-Hans, sl-rozaj-biske-1994'), isNull);
    expect(validator(' de , en-US '), isNull);
    expect(validator('de, de'), isNull, reason: 'the result is unique');
    for (final value in <String>[
      '',
      'e',
      'DE',
      'en-us',
      'zh-hans',
      'de-',
      'de--x',
      'de_foo',
      'de-abcdefghi',
      'd${String.fromCharCode(0xe9)}',
      'en,',
    ]) {
      expect(validator(value), isNotNull, reason: value);
    }
    expect(
      validator(<String>[for (var i = 0; i < 65; i++) 'de-$i'].join(',')),
      contains('at most 64'),
    );

    await tester.enterText(
      find.byKey(const Key('revision3-project-create-name')),
      'Locale test',
    );
    await tester.enterText(
      find.byKey(const Key('revision3-project-create-author')),
      'Author',
    );
    await tester.enterText(
      find.byKey(const Key('revision3-project-create-locales')),
      'en-us',
    );
    await tester.tap(find.byKey(const Key('revision3-project-create-submit')));
    await tester.pump();
    expect(find.byKey(const Key('revision3-project-create-dialog')), findsOne);
    expect(find.textContaining('use "en-US"'), findsOne);
    expect(capture.completed, isFalse);
  });

  testWidgets('uses the selected app locale for copy and validation', (
    tester,
  ) async {
    final capture = _DialogCapture();
    await _openDialog(tester, capture, locale: const Locale('de'));

    expect(find.text('Mod-Projekt erstellen'), findsOneWidget);
    expect(find.text('Projektname'), findsOneWidget);
    expect(find.text('Projekt erstellen'), findsOneWidget);
    final validator = _field(
      tester,
      'revision3-project-create-locales',
    ).validator!;
    expect(validator(''), contains('mindestens eine Bearbeitungssprache'));
    expect(validator('en-us'), contains('verwende „en-US“'));
  });
}

final class _DialogCapture {
  Revision3ProjectCreateFormResult? result;
  bool completed = false;
}

Future<void> _openDialog(
  WidgetTester tester,
  _DialogCapture capture, {
  Locale locale = const Locale('en'),
}) async {
  await tester.pumpWidget(
    MaterialApp(
      locale: locale,
      localizationsDelegates: AppLocalizations.localizationsDelegates,
      supportedLocales: AppLocalizations.supportedLocales,
      home: Scaffold(
        body: Builder(
          builder: (context) => Center(
            child: FilledButton(
              key: const Key('open-project-create-dialog'),
              onPressed: () async {
                capture.result = await showRevision3ProjectCreateDialog(
                  context,
                );
                capture.completed = true;
              },
              child: const Text('Open'),
            ),
          ),
        ),
      ),
    ),
  );
  await tester.tap(find.byKey(const Key('open-project-create-dialog')));
  await tester.pumpAndSettle();
}

TextFormField _field(WidgetTester tester, String key) =>
    tester.widget<TextFormField>(find.byKey(Key(key)));

String _fieldText(WidgetTester tester, String key) =>
    _field(tester, key).controller!.text;

String _repeat(String value, int count) =>
    List<String>.filled(count, value).join();
