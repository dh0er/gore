import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:goresave/features/app/domain/ui_settings.dart';
import 'package:goresave/features/app/ui/about_dialog.dart';
import 'package:goresave/ui/design/app_theme.dart';
import 'package:package_info_plus/package_info_plus.dart';

import 'support/l10n_test_app.dart';

void main() {
  test('aboutVersionLabel shows version and git sha, no build number', () {
    final info = PackageInfo(
      appName: 'goresave',
      packageName: 'goresave',
      version: '1.2.3',
      buildNumber: '7',
    );
    // Default GIT_SHA dart-define is 'dev' in tests.
    expect(aboutVersionLabel(info), 'Version 1.2.3 (dev)');
  });

  test('aboutVersionLabel is empty while package info loads', () {
    expect(aboutVersionLabel(null), '');
  });

  testWidgets('about dialog shows copyright and license notice', (
    tester,
  ) async {
    await tester.pumpWidget(
      wrapWithL10n(
        Theme(
          data: buildGoresaveTheme(
            uiFontFamily: UiFontFamily.notoSerif,
            locale: const Locale('en'),
          ),
          child: const Scaffold(body: GoresaveAboutDialog()),
        ),
      ),
    );
    await tester.pump();

    expect(find.text(aboutCopyrightNotice), findsOneWidget);
    expect(find.text(aboutLicenseNotice), findsOneWidget);
    final titleFinder = find.text('GORE Save Editor');
    final title = tester.widget<Text>(titleFinder);
    final titleContext = tester.element(titleFinder);
    expect(title.style?.fontFamily, notoSerifFontFamily);
    expect(
      title.style?.fontFamily,
      Theme.of(titleContext).textTheme.titleLarge?.fontFamily,
    );
    expect(
      title.style?.fontSize,
      Theme.of(titleContext).textTheme.titleLarge?.fontSize,
    );
    expect(title.style?.fontWeight, FontWeight.bold);
  });
}
