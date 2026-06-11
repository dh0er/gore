import 'package:flutter_test/flutter_test.dart';
import 'package:goresave/features/app/ui/about_dialog.dart';
import 'package:package_info_plus/package_info_plus.dart';

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
}
