import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gore_manager/app/ui/about_dialog.dart';
import 'package:package_info_plus/package_info_plus.dart';

import '../support/l10n_test_app.dart';

void main() {
  test('aboutVersionLabel shows version and git sha, no build number', () {
    final info = PackageInfo(
      appName: 'gore_manager',
      packageName: 'gore_manager',
      version: '1.2.3',
      buildNumber: '7',
    );
    // Default GIT_SHA dart-define is 'dev' in tests.
    expect(aboutVersionLabel(info), 'Version 1.2.3 (dev)');
  });

  test('aboutVersionLabel is empty while package info loads', () {
    expect(aboutVersionLabel(null), '');
  });

  testWidgets('about dialog shows the product name, copyright and license', (
    tester,
  ) async {
    // PackageInfo.fromPlatform() needs a stubbed platform response in tests.
    PackageInfo.setMockInitialValues(
      appName: 'gore_manager',
      packageName: 'gore_manager',
      version: '1.2.3',
      buildNumber: '7',
      buildSignature: '',
    );
    await tester.pumpWidget(
      wrapWithL10n(const Scaffold(body: GoreManagerAboutDialog())),
    );
    await tester.pumpAndSettle();

    expect(find.text('GORE Mod Manager'), findsOneWidget);
    expect(find.text(aboutCopyrightNotice), findsOneWidget);
    expect(find.text(aboutLicenseNotice), findsOneWidget);
    // The version line resolves once PackageInfo returns.
    expect(find.text('Version 1.2.3 (dev)'), findsOneWidget);
  });
}
