import 'dart:async';
import 'dart:convert';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_localizations/flutter_localizations.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gore_manager/app/domain/ui_settings.dart';
import 'package:gore_manager/core/core_service.dart';
import 'package:gore_manager/core/providers.dart';
import 'package:gore_manager/core/ui/core_unavailable_page.dart';
import 'package:gore_manager/gore_manager_app.dart';
import 'package:gore_manager/home_page.dart';
import 'package:gore_manager/l10n/app_localizations.dart';
import 'package:package_info_plus/package_info_plus.dart';

class _StaticSettingsStore implements UiSettingsStore {
  _StaticSettingsStore(this.settings);

  UiSettings settings;

  @override
  UiSettings read() => settings;

  @override
  void write(UiSettings settings) => this.settings = settings;
}

class _BlockedRecordingCore
    implements GoreCoreFfiService, CoreBootstrapStateProvider {
  _BlockedRecordingCore(this.failure);

  final CoreBootstrapFailure failure;
  final List<String> calls = [];

  @override
  CoreBootstrapState get bootstrapState => CoreBootstrapBlocked(failure);

  @override
  String get description => 'blocked-test-core';

  @override
  bool get isAvailable => false;

  @override
  Future<Map<String, Object?>> execute(
    String command, {
    Map<String, Object?> payload = const {},
  }) async {
    calls.add(command);
    return {
      'ok': false,
      'error': {'code': 'TEST_MUST_NOT_CALL_CORE'},
    };
  }
}

Widget _app(
  GoreCoreFfiService core, {
  UiSettings settings = const UiSettings(appLocale: 'en'),
}) => ProviderScope(
  overrides: [
    coreServiceProvider.overrideWithValue(core),
    uiSettingsStoreProvider.overrideWithValue(_StaticSettingsStore(settings)),
  ],
  child: const GoreManagerApp(),
);

Widget _localizedPage(Locale locale, CoreBootstrapFailure failure) =>
    MaterialApp(
      locale: locale,
      localizationsDelegates: const [
        AppLocalizations.delegate,
        GlobalMaterialLocalizations.delegate,
        GlobalWidgetsLocalizations.delegate,
        GlobalCupertinoLocalizations.delegate,
      ],
      supportedLocales: AppLocalizations.supportedLocales,
      home: CoreUnavailablePage(failure: failure),
    );

void main() {
  setUpAll(() {
    PackageInfo.setMockInitialValues(
      appName: 'gore_manager',
      packageName: 'gore_manager',
      version: '1.2.3',
      buildNumber: '4',
      buildSignature: '',
    );
  });

  testWidgets('blocked startup routes away from every manager operation', (
    tester,
  ) async {
    final core = _BlockedRecordingCore(
      CoreBootstrapFailure(
        reason: CoreBootstrapFailureReason.dllMissing,
        candidatePath: r'C:\app\gore_ffi.dll',
      ),
    );

    await tester.pumpWidget(_app(core));
    await tester.pumpAndSettle();

    expect(find.byKey(const ValueKey('core-unavailable-page')), findsOneWidget);
    expect(find.byType(HomePage), findsNothing);
    expect(core.calls, isEmpty);
    expect(find.byKey(const ValueKey('import-mod-action')), findsNothing);
    expect(find.byKey(const ValueKey('manager-overflow-action')), findsNothing);
    expect(find.byKey(const ValueKey('library-refresh-action')), findsNothing);
    expect(find.text('Apply'), findsNothing);
    expect(find.text('Recover'), findsNothing);
    expect(tester.takeException(), isNull);
  });

  testWidgets('copy is focused, bounded, exact, and uses one live region', (
    tester,
  ) async {
    String? clipboardText;
    tester.binding.defaultBinaryMessenger.setMockMethodCallHandler(
      SystemChannels.platform,
      (call) async {
        if (call.method == 'Clipboard.setData') {
          clipboardText = (call.arguments as Map)['text'] as String?;
        }
        return null;
      },
    );
    addTearDown(
      () => tester.binding.defaultBinaryMessenger.setMockMethodCallHandler(
        SystemChannels.platform,
        null,
      ),
    );
    final failure = CoreBootstrapFailure(
      reason: CoreBootstrapFailureReason.protocolAbiMismatch,
      candidatePath: r'C:\app\gore_ffi.dll',
      observedTransportAbi: 2,
      observedProtocolAbi: 2,
      coreVersion: '9.0.0',
      detail:
          'unsafe\u061c\u200e\u200f\u202a\u202b\u202c\u202d\u202e'
          '\u2066\u2067\u2068\u2069details',
    );
    final semantics = tester.ensureSemantics();

    await tester.pumpWidget(_app(_BlockedRecordingCore(failure)));
    await tester.pumpAndSettle();

    final liveRegions = find.byWidgetPredicate(
      (widget) => widget is Semantics && widget.properties.liveRegion == true,
      description: 'live regions',
    );
    expect(liveRegions, findsOneWidget);
    final blockerHeading = find.byKey(
      const ValueKey('core-unavailable-heading'),
    );
    expect(blockerHeading, findsOneWidget);
    expect(tester.widget<Semantics>(blockerHeading).properties.header, isTrue);

    final copyFinder = find.byKey(const ValueKey('core-copy-details-action'));
    final copyButton = tester.widget<FilledButton>(copyFinder);
    expect(copyButton.focusNode?.hasPrimaryFocus, isTrue);

    await tester.sendKeyEvent(LogicalKeyboardKey.enter);
    await tester.pumpAndSettle();

    expect(clipboardText, failure.technicalReport(managerVersion: '1.2.3'));
    expect(utf8.encode(clipboardText!).length, lessThanOrEqualTo(8 * 1024));
    final visibleDetails = tester.widget<SelectableText>(
      find.byKey(const ValueKey('core-technical-details')),
    );
    for (final bidiControl in const [
      '\u061c',
      '\u200e',
      '\u200f',
      '\u202a',
      '\u202b',
      '\u202c',
      '\u202d',
      '\u202e',
      '\u2066',
      '\u2067',
      '\u2068',
      '\u2069',
    ]) {
      expect(visibleDetails.data, isNot(contains(bidiControl)));
      expect(clipboardText, isNot(contains(bidiControl)));
    }
    expect(
      jsonDecode(clipboardText!)['compatibility_direction'],
      'manager_too_old',
    );
    expect(
      find.byKey(const ValueKey('core-technical-details-copied')),
      findsOneWidget,
    );
    expect(liveRegions, findsOneWidget);
    expect(
      tester.widget<FilledButton>(copyFinder).focusNode?.hasPrimaryFocus,
      isTrue,
    );
    expect(tester.takeException(), isNull);
    semantics.dispose();
  });

  testWidgets('blocker remains reachable at 700x460 and 200% UI scale', (
    tester,
  ) async {
    tester.view.physicalSize = const Size(700, 460);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.resetPhysicalSize);
    addTearDown(tester.view.resetDevicePixelRatio);
    final failure = CoreBootstrapFailure(
      reason: CoreBootstrapFailureReason.dllLoadFailed,
      candidatePath: 'C:\\${List.filled(700, 'long-path').join()}',
      detail: List.filled(700, 'blocked dependency ').join(),
    );

    await tester.pumpWidget(
      _app(
        _BlockedRecordingCore(failure),
        settings: const UiSettings(appLocale: 'de', uiScale: 2),
      ),
    );
    await tester.pumpAndSettle();

    final copy = find.byKey(const ValueKey('core-copy-details-action'));
    expect(copy.hitTestable(), findsOneWidget);
    expect(tester.takeException(), isNull);

    final details = find.byKey(
      const ValueKey('core-technical-details-heading'),
    );
    await tester.scrollUntilVisible(
      details,
      80,
      scrollable: find.ancestor(of: details, matching: find.byType(Scrollable)),
    );
    expect(details.hitTestable(), findsOneWidget);
    expect(
      find.byKey(const ValueKey('core-technical-details')),
      findsOneWidget,
    );
    expect(tester.takeException(), isNull);
  });

  testWidgets('clipboard failure is accessible and retryable', (tester) async {
    var fail = true;
    String? clipboardText;
    tester.binding.defaultBinaryMessenger.setMockMethodCallHandler(
      SystemChannels.platform,
      (call) async {
        if (call.method != 'Clipboard.setData') return null;
        if (fail) {
          throw PlatformException(code: 'clipboard-busy');
        }
        clipboardText = (call.arguments as Map)['text'] as String?;
        return null;
      },
    );
    addTearDown(
      () => tester.binding.defaultBinaryMessenger.setMockMethodCallHandler(
        SystemChannels.platform,
        null,
      ),
    );
    final failure = CoreBootstrapFailure(
      reason: CoreBootstrapFailureReason.dllMissing,
    );

    await tester.pumpWidget(_app(_BlockedRecordingCore(failure)));
    await tester.pumpAndSettle();

    final copyFinder = find.byKey(const ValueKey('core-copy-details-action'));
    await tester.sendKeyEvent(LogicalKeyboardKey.enter);
    await tester.pumpAndSettle();

    expect(
      find.byKey(const ValueKey('core-technical-details-copy-failed')),
      findsOneWidget,
    );
    expect(
      find.byKey(const ValueKey('core-technical-details-copied')),
      findsNothing,
    );
    expect(
      tester.widget<FilledButton>(copyFinder).focusNode?.hasPrimaryFocus,
      isTrue,
    );

    fail = false;
    await tester.sendKeyEvent(LogicalKeyboardKey.enter);
    await tester.pumpAndSettle();

    expect(clipboardText, failure.technicalReport(managerVersion: '1.2.3'));
    expect(
      find.byKey(const ValueKey('core-technical-details-copy-failed')),
      findsNothing,
    );
    expect(
      find.byKey(const ValueKey('core-technical-details-copied')),
      findsOneWidget,
    );
    expect(
      tester.widget<FilledButton>(copyFinder).focusNode?.hasPrimaryFocus,
      isTrue,
    );
    expect(tester.takeException(), isNull);
  });

  testWidgets('copy waits for manager version metadata', (tester) async {
    final version = Completer<String?>();
    String? clipboardText;
    tester.binding.defaultBinaryMessenger.setMockMethodCallHandler(
      SystemChannels.platform,
      (call) async {
        if (call.method == 'Clipboard.setData') {
          clipboardText = (call.arguments as Map)['text'] as String?;
        }
        return null;
      },
    );
    addTearDown(
      () => tester.binding.defaultBinaryMessenger.setMockMethodCallHandler(
        SystemChannels.platform,
        null,
      ),
    );
    final failure = CoreBootstrapFailure(
      reason: CoreBootstrapFailureReason.dllMissing,
    );

    await tester.pumpWidget(
      MaterialApp(
        locale: const Locale('en'),
        localizationsDelegates: const [
          AppLocalizations.delegate,
          GlobalMaterialLocalizations.delegate,
          GlobalWidgetsLocalizations.delegate,
          GlobalCupertinoLocalizations.delegate,
        ],
        supportedLocales: AppLocalizations.supportedLocales,
        home: CoreUnavailablePage.forTesting(
          () => version.future,
          failure: failure,
        ),
      ),
    );
    await tester.pumpAndSettle();

    await tester.sendKeyEvent(LogicalKeyboardKey.enter);
    await tester.pump();
    expect(clipboardText, isNull);
    expect(
      find.byKey(const ValueKey('core-technical-details-copied')),
      findsNothing,
    );

    version.complete('7.8.9');
    await tester.pumpAndSettle();

    expect(jsonDecode(clipboardText!)['manager_version'], '7.8.9');
    expect(
      find.byKey(const ValueKey('core-technical-details-copied')),
      findsOneWidget,
    );
    expect(tester.takeException(), isNull);
  });

  testWidgets('all twelve supported locales render the blocker contract', (
    tester,
  ) async {
    final failure = CoreBootstrapFailure(
      reason: CoreBootstrapFailureReason.requiredCommandsMissing,
      missingCommands: const ['mgr_apply'],
    );

    for (final locale in AppLocalizations.supportedLocales) {
      await tester.pumpWidget(_localizedPage(locale, failure));
      await tester.pumpAndSettle();
      final l10n = await AppLocalizations.delegate.load(locale);
      expect(
        [
          l10n.coreBlockedTitle,
          l10n.coreDllMissingMessage,
          l10n.coreDllLoadFailedMessage,
          l10n.coreVerificationFailedMessage,
          l10n.coreManagerTooOldMessage,
          l10n.coreNativeTooOldMessage,
          l10n.coreCommandsMissingMessage,
          l10n.coreBlockedRepairHint,
          l10n.coreTechnicalDetails,
          l10n.coreCopyTechnicalDetails,
          l10n.coreTechnicalDetailsCopied,
          l10n.coreTechnicalDetailsCopyFailed,
        ].every((value) => value.trim().isNotEmpty),
        isTrue,
        reason: 'incomplete blocker localization for $locale',
      );
      expect(
        find.text(l10n.coreBlockedTitle),
        findsOneWidget,
        reason: 'missing blocker title for $locale',
      );
      expect(
        find.text(l10n.coreCopyTechnicalDetails),
        findsOneWidget,
        reason: 'missing copy action for $locale',
      );
      expect(tester.takeException(), isNull, reason: 'failed locale $locale');
    }
  });
}
