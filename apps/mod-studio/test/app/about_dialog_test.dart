import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/app/ui/about_dialog.dart';
import 'package:gore_mod/l10n/app_localizations.dart';
import 'package:package_info_plus/package_info_plus.dart';

void main() {
  test('aboutVersionLabel shows version and git sha, no build number', () {
    final info = PackageInfo(
      appName: 'gore_mod',
      packageName: 'gore_mod',
      version: '1.2.3',
      buildNumber: '7',
    );
    // Default GIT_SHA dart-define is 'dev' in tests.
    expect(aboutVersionLabel(info), 'Version 1.2.3 (dev)');
  });

  test('aboutVersionLabel is empty while package info loads', () {
    expect(aboutVersionLabel(null), '');
  });

  testWidgets('about dialog shows copyright and license notice', (tester) async {
    await tester.pumpWidget(
      MaterialApp(
        localizationsDelegates: AppLocalizations.localizationsDelegates,
        supportedLocales: AppLocalizations.supportedLocales,
        home: const Scaffold(body: GoreStudioAboutDialog()),
      ),
    );
    await tester.pump();

    expect(find.text(aboutCopyrightNotice), findsOneWidget);
    expect(find.text(aboutLicenseNotice), findsOneWidget);
  });
}
