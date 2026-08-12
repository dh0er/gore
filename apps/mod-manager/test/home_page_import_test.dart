import 'dart:async';
import 'dart:io';

import 'package:file_selector/file_selector.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_localizations/flutter_localizations.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gore_manager/app/domain/shared_config.dart';
import 'package:gore_manager/app/domain/ui_settings.dart';
import 'package:gore_manager/core/core_service.dart';
import 'package:gore_manager/core/mgr_ffi.dart';
import 'package:gore_manager/core/providers.dart';
import 'package:gore_manager/home_page.dart';
import 'package:gore_manager/l10n/app_localizations.dart';
import 'package:gore_manager/library/domain/library_notifier.dart';
import 'package:gore_manager/library/domain/models.dart';
import 'package:gore_manager/library/ui/import_feedback.dart';
import 'package:gore_manager/library/ui/import_source_picker.dart';
import 'package:gore_manager/library/ui/mod_list.dart';
import 'package:path/path.dart' as p;

class _SettingsStore implements UiSettingsStore {
  @override
  UiSettings read() => const UiSettings();

  @override
  void write(UiSettings settings) {}
}

SharedConfig _config() {
  final directory = Directory.systemTemp.createTempSync('gm_import_home');
  addTearDown(() {
    if (directory.existsSync()) directory.deleteSync(recursive: true);
  });
  return SharedConfig(File(p.join(directory.path, 'config.json')));
}

Map<String, Object?> _entry(String id, String name) => {
  'id': id,
  'kind': 'foreign_pak',
  'name': name,
  'components': const <Object?>[],
};

typedef _ImportScenario = ({
  Map<String, Object?> response,
  Map<String, Object?>? publishedEntry,
});

class _ImportCore implements GoreCoreFfiService {
  _ImportCore({
    List<Map<String, Object?>>? initialMods,
    Map<String, _ImportScenario>? scenarios,
  }) : mods = [...?initialMods],
       scenarios = scenarios ?? {};

  final List<Map<String, Object?>> mods;
  final Map<String, _ImportScenario> scenarios;
  final List<({String command, Map<String, Object?> payload})> calls = [];
  final Completer<void> importStarted = Completer<void>();
  Completer<void>? importRelease;
  bool failReloadAfterImport = false;
  String reloadFailureMessage = 'authoritative reload failed';
  bool _importAttempted = false;

  @override
  bool get isAvailable => true;

  @override
  String get description => 'home-import-test';

  @override
  Future<Map<String, Object?>> execute(
    String command, {
    Map<String, Object?> payload = const {},
  }) async {
    calls.add((command: command, payload: payload));
    switch (command) {
      case 'mgr_library_list' when _importAttempted && failReloadAfterImport:
        return {
          'ok': false,
          'error': {'code': 'IO', 'message': reloadFailureMessage},
        };
      case 'mgr_library_list':
        return {
          'ok': true,
          'mods': [for (final mod in mods) Map<String, Object?>.from(mod)],
          'loadout': {
            'format': 1,
            'entries': [
              for (final mod in mods) {'id': mod['id'], 'enabled': false},
            ],
          },
        };
      case 'mgr_import':
        _importAttempted = true;
        if (!importStarted.isCompleted) importStarted.complete();
        if (importRelease case final release?) await release.future;
        final path = payload['path'];
        final scenario = scenarios[path];
        if (scenario == null) {
          return {
            'ok': false,
            'error': {'code': 'IMPORT_FAILED', 'message': 'missing fixture'},
          };
        }
        if (scenario.publishedEntry case final published?) {
          final id = published['id'];
          mods.removeWhere((mod) => mod['id'] == id);
          mods.add(Map<String, Object?>.from(published));
        }
        return scenario.response;
      case 'mgr_analyze':
        return {'ok': true, 'conflicts': const <Object?>[]};
      case 'mgr_status':
        return {
          'ok': true,
          'status': {'state': 'nothing_deployed'},
        };
      case 'mgr_preflight_v1':
        return fakeHealthyManagerPreflightResponse();
      default:
        return {'ok': true};
    }
  }
}

class _Picker implements ImportSourcePicker {
  _Picker(
    Iterable<String?> folderPaths, {
    Iterable<String?> filePaths = const [],
  }) : _folderPaths = List<String?>.from(folderPaths),
       _filePaths = List<String?>.from(filePaths);

  final List<String?> _folderPaths;
  final List<String?> _filePaths;
  int folderCalls = 0;
  int fileCalls = 0;

  @override
  Future<String?> pickFolder() async {
    folderCalls++;
    return _folderPaths.isEmpty ? null : _folderPaths.removeAt(0);
  }

  @override
  Future<String?> pickFile({required String dialogLabel}) async {
    fileCalls++;
    return _filePaths.isEmpty ? null : _filePaths.removeAt(0);
  }
}

Widget _home(
  _ImportCore core,
  ImportSourcePicker picker, {
  TextScaler textScaler = TextScaler.noScaling,
}) => ProviderScope(
  overrides: [
    coreServiceProvider.overrideWithValue(core),
    uiSettingsStoreProvider.overrideWithValue(_SettingsStore()),
    sharedConfigProvider.overrideWithValue(_config()),
  ],
  child: MaterialApp(
    locale: const Locale('en'),
    localizationsDelegates: const [
      AppLocalizations.delegate,
      GlobalMaterialLocalizations.delegate,
      GlobalWidgetsLocalizations.delegate,
      GlobalCupertinoLocalizations.delegate,
    ],
    supportedLocales: AppLocalizations.supportedLocales,
    builder: (context, child) => MediaQuery(
      data: MediaQuery.of(context).copyWith(textScaler: textScaler),
      child: child!,
    ),
    home: HomePage(importSourcePicker: picker),
  ),
);

Widget _feedbackHarness(Locale locale) => MaterialApp(
  locale: locale,
  localizationsDelegates: const [
    AppLocalizations.delegate,
    GlobalMaterialLocalizations.delegate,
    GlobalWidgetsLocalizations.delegate,
    GlobalCupertinoLocalizations.delegate,
  ],
  supportedLocales: AppLocalizations.supportedLocales,
  builder: (context, child) => MediaQuery(
    data: MediaQuery.of(
      context,
    ).copyWith(textScaler: const TextScaler.linear(2)),
    child: child!,
  ),
  home: Scaffold(
    body: Builder(
      builder: (context) => TextButton(
        key: const ValueKey('show-import-failure'),
        onPressed: () => showImportFailureFeedback(
          context,
          MgrFfiException('opaque native detail', code: 'IMPORT_FAILED'),
          const LibraryState(authoritative: true),
        ),
        child: const Text('Show'),
      ),
    ),
  ),
);

Future<void> _chooseFolder(WidgetTester tester, {bool settle = true}) async {
  await tester.tap(find.byKey(const ValueKey('import-mod-action')));
  await tester.pumpAndSettle();
  await tester.tap(find.byKey(const ValueKey('import-folder-action')));
  if (settle) {
    await tester.pumpAndSettle();
  } else {
    await tester.pump();
  }
}

Future<void> _chooseFile(WidgetTester tester) async {
  await tester.tap(find.byKey(const ValueKey('import-mod-action')));
  await tester.pumpAndSettle();
  await tester.tap(find.byKey(const ValueKey('import-file-action')));
  await tester.pumpAndSettle();
}

Finder _snackLiveRegions() => find.descendant(
  of: find.byType(SnackBar),
  matching: find.byWidgetPredicate(
    (widget) => widget is Semantics && widget.properties.liveRegion == true,
    description: 'SnackBar live region',
  ),
);

void _expectFeedbackDoesNotOwnFocus() {
  final context = FocusManager.instance.primaryFocus?.context;
  if (context == null) return;
  final focused = find.byElementPredicate(
    (element) => identical(element, context),
  );
  expect(
    find.descendant(of: find.byType(SnackBar), matching: focused),
    findsNothing,
  );
}

Map<String, Object?> _successWire({
  required String id,
  required String disposition,
  required String matchedBy,
}) => {
  'ok': true,
  'entry': _entry(id, 'Untrusted wire name'),
  'disposition': disposition,
  'matched_by': matchedBy,
};

void main() {
  test(
    'all twelve locales provide the complete import feedback surface',
    () async {
      expect(AppLocalizations.supportedLocales, hasLength(12));
      for (final locale in AppLocalizations.supportedLocales) {
        final l10n = await AppLocalizations.delegate.load(locale);
        final values = [
          l10n.importOutcomeCreated('Exact name'),
          l10n.importOutcomeUpdated('Exact name'),
          l10n.importOutcomeUnchanged('Exact name'),
          l10n.importOutcomeMatchedBy('none'),
          l10n.importOutcomeMatchedBy('source'),
          l10n.importOutcomeMatchedBy('content'),
          l10n.importOutcomeMatchedBy('entry_id'),
          l10n.importRefusalDuplicateAmbiguous,
          l10n.importRefusalIdentityConflict,
          l10n.importFailed,
          l10n.importOutcomeUnknown,
        ];
        expect(values.every((value) => value.trim().isNotEmpty), isTrue);
        for (final token in const [
          'ZIP',
          '*_P.pak',
          '.utoc',
          '.ucas',
          '.lcache',
          '.bank',
          'PrecompiledScript*.Cache',
          '.7z',
          '.rar',
        ]) {
          expect(
            l10n.importFailed,
            contains(token),
            reason: '${locale.toLanguageTag()} must explain $token',
          );
        }
      }
    },
  );

  test(
    'default file picker allows native to classify every extension',
    () async {
      late List<XTypeGroup> acceptedTypeGroups;
      final picker = FileSelectorImportSourcePicker(
        fileOpener: (groups) async {
          acceptedTypeGroups = groups;
          return XFile('D:/picked/unsupported.7z');
        },
      );

      expect(
        await picker.pickFile(dialogLabel: 'Import file'),
        'D:/picked/unsupported.7z',
      );
      expect(acceptedTypeGroups, hasLength(1));
      expect(acceptedTypeGroups.single.label, 'Import file');
      expect(acceptedTypeGroups.single.allowsAny, isTrue);
    },
  );

  testWidgets('long localized guidance remains usable at compact 200% scale', (
    tester,
  ) async {
    tester.view.physicalSize = const Size(700, 600);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.resetPhysicalSize);
    addTearDown(tester.view.resetDevicePixelRatio);
    const locale = Locale('de');
    await tester.pumpWidget(_feedbackHarness(locale));
    await tester.pumpAndSettle();

    await tester.tap(find.byKey(const ValueKey('show-import-failure')));
    await tester.pumpAndSettle();

    final l10n = await AppLocalizations.delegate.load(locale);
    expect(find.text(l10n.importFailed), findsOneWidget);
    final messageScroll = find.byKey(
      const ValueKey('import-feedback-message-scroll'),
    );
    final scrollable = find.descendant(
      of: messageScroll,
      matching: find.byType(Scrollable),
    );
    expect(scrollable, findsOneWidget);
    final position = tester.state<ScrollableState>(scrollable).position;
    expect(position.maxScrollExtent, greaterThan(0));
    expect(_snackLiveRegions(), findsOneWidget);
    _expectFeedbackDoesNotOwnFocus();
    final toggle = find.byKey(const ValueKey('import-feedback-details-toggle'));
    expect(toggle.hitTestable(), findsOneWidget);
    expect(tester.getSize(toggle).height, greaterThanOrEqualTo(44));
    expect(
      tester.getSemantics(toggle),
      matchesSemantics(
        label: l10n.coreTechnicalDetails,
        isButton: true,
        isFocusable: true,
        hasEnabledState: true,
        isEnabled: true,
        hasTapAction: true,
        hasFocusAction: true,
        hasExpandedState: true,
      ),
    );
    var snackRect = tester.getRect(find.byType(SnackBar));
    expect(snackRect.left, greaterThanOrEqualTo(0));
    expect(snackRect.right, lessThanOrEqualTo(700));
    expect(snackRect.top, greaterThanOrEqualTo(0));
    expect(snackRect.bottom, lessThanOrEqualTo(600));
    await tester.tap(toggle);
    await tester.pumpAndSettle();
    expect(
      tester.getSemantics(toggle),
      matchesSemantics(
        label: l10n.coreTechnicalDetails,
        isButton: true,
        isFocusable: true,
        hasEnabledState: true,
        isEnabled: true,
        hasTapAction: true,
        hasFocusAction: true,
        hasExpandedState: true,
        isExpanded: true,
      ),
    );
    expect(
      find.byKey(const ValueKey('import-feedback-details')),
      findsOneWidget,
    );
    expect(_snackLiveRegions(), findsOneWidget);
    snackRect = tester.getRect(find.byType(SnackBar));
    expect(snackRect.left, greaterThanOrEqualTo(0));
    expect(snackRect.right, lessThanOrEqualTo(700));
    expect(snackRect.top, greaterThanOrEqualTo(0));
    expect(snackRect.bottom, lessThanOrEqualTo(600));
    expect(tester.takeException(), isNull);
  });

  test('display names are rune-bounded and sanitize controls and bidi', () {
    final name = 'Exact\nname\u202E${'x' * 300}';
    final shown = importDisplayName(
      ModEntryMetaView(id: 'fallback', kind: 'foreign_pak', name: name),
    );
    expect(shown, startsWith('Exact name'));
    expect(shown, isNot(contains('\n')));
    expect(shown, isNot(contains('\u202E')));
    expect(shown, endsWith('…'));
    expect(shown.runes.length, lessThanOrEqualTo(161));
  });

  final outcomes =
      <({String disposition, String matchedBy, String id, String name})>[
        (
          disposition: 'created',
          matchedBy: 'none',
          id: 'created-mod',
          name: 'Authoritative Added',
        ),
        (
          disposition: 'updated',
          matchedBy: 'source',
          id: 'source-mod',
          name: 'Authoritative Source Update',
        ),
        (
          disposition: 'updated',
          matchedBy: 'content',
          id: 'content-mod',
          name: 'Authoritative Content Update',
        ),
        (
          disposition: 'unchanged',
          matchedBy: 'entry_id',
          id: 'id-mod',
          name: 'Authoritative Existing',
        ),
      ];

  for (final outcome in outcomes) {
    testWidgets(
      'shows ${outcome.disposition}/${outcome.matchedBy} with authoritative name',
      (tester) async {
        const path = 'D:/picked/mod.zip';
        final core = _ImportCore(
          initialMods: [
            if (outcome.disposition != 'created')
              _entry(outcome.id, 'Old name'),
          ],
          scenarios: {
            path: (
              response: _successWire(
                id: outcome.id,
                disposition: outcome.disposition,
                matchedBy: outcome.matchedBy,
              ),
              publishedEntry: _entry(outcome.id, outcome.name),
            ),
          },
        );
        await tester.pumpWidget(_home(core, _Picker([path])));
        await tester.pumpAndSettle();

        await _chooseFolder(tester);

        final l10n = await AppLocalizations.delegate.load(const Locale('en'));
        final dispositionText = switch (outcome.disposition) {
          'created' => l10n.importOutcomeCreated(outcome.name),
          'updated' => l10n.importOutcomeUpdated(outcome.name),
          _ => l10n.importOutcomeUnchanged(outcome.name),
        };
        expect(
          find.text(
            '$dispositionText '
            '${l10n.importOutcomeMatchedBy(outcome.matchedBy)}',
          ),
          findsOneWidget,
        );
        expect(find.textContaining('Untrusted wire name'), findsNothing);
        expect(_snackLiveRegions(), findsOneWidget);
        _expectFeedbackDoesNotOwnFocus();

        final container = ProviderScope.containerOf(
          tester.element(find.byType(HomePage)),
        );
        expect(container.read(selectedModProvider), outcome.id);
        expect(
          container.read(libraryProvider).modById(outcome.id)?.name,
          outcome.name,
        );
      },
    );
  }

  testWidgets('picker cancellation changes no Manager or selection state', (
    tester,
  ) async {
    final core = _ImportCore(initialMods: [_entry('alpha', 'Alpha')]);
    final picker = _Picker([null]);
    await tester.pumpWidget(_home(core, picker));
    await tester.pumpAndSettle();
    final container = ProviderScope.containerOf(
      tester.element(find.byType(HomePage)),
    );
    container.read(selectedModProvider.notifier).state = 'alpha';
    final importButton = tester.widget<OutlinedButton>(
      find.byKey(const ValueKey('import-mod-action')),
    );
    importButton.focusNode!.requestFocus();
    await tester.pump();
    expect(importButton.focusNode!.hasFocus, isTrue);
    final callsBefore = core.calls.length;

    await _chooseFolder(tester);

    expect(picker.folderCalls, 1);
    expect(core.calls, hasLength(callsBefore));
    expect(container.read(selectedModProvider), 'alpha');
    expect(container.read(libraryProvider).mods, hasLength(1));
    expect(find.byType(SnackBar), findsNothing);
    expect(importButton.focusNode!.hasFocus, isTrue);
  });

  for (final source in const [
    (extension: '7z', path: 'D:/picked/unsupported.7z'),
    (extension: 'rar', path: 'D:/picked/unsupported.rar'),
    (extension: 'unknown', path: 'D:/picked/unsupported.unknown'),
  ]) {
    testWidgets(
      '${source.extension} selection reaches native and shows honest guidance',
      (tester) async {
        final compact = source.extension == '7z';
        if (compact) {
          tester.view.physicalSize = const Size(700, 600);
          tester.view.devicePixelRatio = 1;
          addTearDown(tester.view.resetPhysicalSize);
          addTearDown(tester.view.resetDevicePixelRatio);
        }
        final core = _ImportCore(
          scenarios: {
            source.path: (
              response: {
                'ok': false,
                'error': {
                  'code': 'IMPORT_FAILED',
                  'message': 'native-only reason must stay collapsed',
                },
              },
              publishedEntry: null,
            ),
          },
        );
        final picker = _Picker(const [], filePaths: [source.path]);
        await tester.pumpWidget(
          _home(
            core,
            picker,
            textScaler: compact
                ? const TextScaler.linear(2)
                : TextScaler.noScaling,
          ),
        );
        await tester.pumpAndSettle();

        await _chooseFile(tester);

        final importCall = core.calls
            .where((call) => call.command == 'mgr_import')
            .single;
        expect(importCall.payload['path'], source.path);
        expect(picker.fileCalls, 1);
        final l10n = await AppLocalizations.delegate.load(const Locale('en'));
        expect(find.text(l10n.importFailed), findsOneWidget);
        expect(find.textContaining('native-only reason'), findsNothing);
        expect(_snackLiveRegions(), findsOneWidget);
        _expectFeedbackDoesNotOwnFocus();
        if (compact) {
          final snackRect = tester.getRect(find.byType(SnackBar));
          expect(snackRect.left, greaterThanOrEqualTo(0));
          expect(snackRect.right, lessThanOrEqualTo(700));
          expect(snackRect.top, greaterThanOrEqualTo(0));
          expect(snackRect.bottom, lessThanOrEqualTo(600));
          expect(
            find.byKey(const ValueKey('import-feedback-message-scroll')),
            findsOneWidget,
          );
          expect(tester.takeException(), isNull);
        }
      },
    );
  }

  testWidgets('A to B to A selection race never auto-selects the import', (
    tester,
  ) async {
    const path = 'D:/picked/race.zip';
    final core = _ImportCore(
      initialMods: [_entry('alpha', 'Alpha'), _entry('beta', 'Beta')],
      scenarios: {
        path: (
          response: _successWire(
            id: 'created-mod',
            disposition: 'created',
            matchedBy: 'none',
          ),
          publishedEntry: _entry('created-mod', 'Created During Race'),
        ),
      },
    )..importRelease = Completer<void>();
    await tester.pumpWidget(_home(core, _Picker([path])));
    await tester.pumpAndSettle();
    final container = ProviderScope.containerOf(
      tester.element(find.byType(HomePage)),
    );
    container.read(selectedModProvider.notifier).state = 'alpha';
    await tester.pump();

    await _chooseFolder(tester, settle: false);
    await core.importStarted.future;
    container.read(selectedModProvider.notifier).state = 'beta';
    await tester.pump();
    container.read(selectedModProvider.notifier).state = 'alpha';
    await tester.pump();
    core.importRelease!.complete();
    await tester.pumpAndSettle();

    expect(container.read(selectedModProvider), 'alpha');
    expect(
      container.read(libraryProvider).modById('created-mod')?.name,
      'Created During Race',
    );
    expect(
      find.byKey(const ValueKey('import-feedback-message')),
      findsOneWidget,
    );
    expect(
      tester
          .widget<Text>(find.byKey(const ValueKey('import-feedback-message')))
          .data,
      contains('Created During Race'),
    );
  });

  final refusals = <({String code, String friendly, Object details})>[
    (
      code: 'IMPORT_DUPLICATE_AMBIGUOUS',
      friendly: 'duplicate',
      details: {
        'candidate_ids': ['alpha', 'beta'],
      },
    ),
    (
      code: 'IMPORT_IDENTITY_CONFLICT',
      friendly: 'identity',
      details: {
        'candidates': [
          {
            'id': 'alpha',
            'matched_by': ['entry_id', 'source'],
          },
          {
            'id': 'beta',
            'matched_by': ['content'],
          },
        ],
      },
    ),
  ];

  for (final refusal in refusals) {
    testWidgets(
      '${refusal.friendly} refusal is friendly with bounded expandable detail',
      (tester) async {
        tester.view.physicalSize = const Size(700, 600);
        tester.view.devicePixelRatio = 1;
        addTearDown(tester.view.resetPhysicalSize);
        addTearDown(tester.view.resetDevicePixelRatio);
        const path = 'D:/picked/refused.zip';
        final core = _ImportCore(
          initialMods: [
            _entry('alpha', 'Alpha\nName\u202E'),
            _entry('beta', 'Beta'),
          ],
          scenarios: {
            path: (
              response: {
                'ok': false,
                'error': {
                  'code': refusal.code,
                  'message': 'native\nsecret\u202E${'x' * 1400}',
                  'details': refusal.details,
                },
              },
              publishedEntry: null,
            ),
          },
        );
        await tester.pumpWidget(
          _home(core, _Picker([path]), textScaler: const TextScaler.linear(2)),
        );
        await tester.pumpAndSettle();
        expect(tester.takeException(), isNull, reason: 'compact startup');
        final container = ProviderScope.containerOf(
          tester.element(find.byType(HomePage)),
        );
        container.read(selectedModProvider.notifier).state = 'alpha';

        await _chooseFolder(tester);
        expect(tester.takeException(), isNull, reason: 'collapsed feedback');

        final l10n = await AppLocalizations.delegate.load(const Locale('en'));
        final expected = refusal.code == 'IMPORT_DUPLICATE_AMBIGUOUS'
            ? l10n.importRefusalDuplicateAmbiguous
            : l10n.importRefusalIdentityConflict;
        expect(find.text(expected), findsOneWidget);
        expect(find.textContaining('native'), findsNothing);
        expect(find.textContaining('secret'), findsNothing);
        expect(container.read(selectedModProvider), 'alpha');
        expect(container.read(libraryProvider).error, isNull);
        expect(_snackLiveRegions(), findsOneWidget);
        _expectFeedbackDoesNotOwnFocus();

        final toggle = find.byKey(
          const ValueKey('import-feedback-details-toggle'),
        );
        expect(toggle.hitTestable(), findsOneWidget);
        expect(tester.getSize(toggle).height, greaterThanOrEqualTo(44));
        expect(
          tester.getSemantics(toggle),
          matchesSemantics(
            label: l10n.coreTechnicalDetails,
            isButton: true,
            isFocusable: true,
            hasEnabledState: true,
            isEnabled: true,
            hasTapAction: true,
            hasFocusAction: true,
            hasExpandedState: true,
          ),
        );
        final snackBar = tester.widget<SnackBar>(find.byType(SnackBar));
        expect(snackBar.persist, isTrue);
        await tester.ensureVisible(toggle);
        await tester.tap(toggle);
        await tester.pumpAndSettle();
        expect(
          tester.getSemantics(toggle),
          matchesSemantics(
            label: l10n.coreTechnicalDetails,
            isButton: true,
            isFocusable: true,
            hasEnabledState: true,
            isEnabled: true,
            hasTapAction: true,
            hasFocusAction: true,
            hasExpandedState: true,
            isExpanded: true,
          ),
        );
        final details = tester.widget<SelectableText>(
          find.byKey(const ValueKey('import-feedback-details')),
        );
        expect(details.data, contains('native secret'));
        expect(details.data, contains('Alpha Name'));
        expect(details.data, contains('Beta'));
        expect(details.data, isNot(contains('\u202E')));
        expect(details.data, isNot(contains('\n')));
        expect(details.data!.runes.length, lessThanOrEqualTo(1025));
        expect(_snackLiveRegions(), findsOneWidget);
        final snackRect = tester.getRect(find.byType(SnackBar));
        expect(snackRect.left, greaterThanOrEqualTo(0));
        expect(snackRect.right, lessThanOrEqualTo(700));
        expect(snackRect.top, greaterThanOrEqualTo(0));
        expect(snackRect.bottom, lessThanOrEqualTo(600));
        expect(tester.takeException(), isNull, reason: 'expanded feedback');
      },
    );
  }

  testWidgets('technical details toggle is keyboard operable', (tester) async {
    const path = 'D:/picked/keyboard-refusal.zip';
    final core = _ImportCore(
      scenarios: {
        path: (
          response: {
            'ok': false,
            'error': {
              'code': 'IMPORT_DUPLICATE_AMBIGUOUS',
              'message': 'bounded diagnostic',
              'details': {
                'candidate_ids': ['alpha', 'beta'],
              },
            },
          },
          publishedEntry: null,
        ),
      },
    );
    await tester.pumpWidget(_home(core, _Picker([path])));
    await tester.pumpAndSettle();

    await _chooseFolder(tester);
    final toggle = find.byKey(const ValueKey('import-feedback-details-toggle'));
    final button = tester.widget<TextButton>(toggle);
    button.focusNode!.requestFocus();
    await tester.pump();
    expect(button.focusNode!.hasFocus, isTrue);
    await tester.sendKeyEvent(LogicalKeyboardKey.enter);
    await tester.pumpAndSettle();

    expect(
      find.byKey(const ValueKey('import-feedback-details')),
      findsOneWidget,
    );
  });

  testWidgets('structured refusal witnesses retain both maximum-bound ids', (
    tester,
  ) async {
    const path = 'D:/picked/bounded-refusal.zip';
    final firstId = 'a' * 256;
    final secondId = 'b' * 256;
    final core = _ImportCore(
      initialMods: [
        _entry(firstId, 'First ${'x' * 300}'),
        _entry(secondId, 'Second ${'y' * 300}'),
      ],
      scenarios: {
        path: (
          response: {
            'ok': false,
            'error': {
              'code': 'IMPORT_IDENTITY_CONFLICT',
              'message': 'opaque ${'z' * 900}',
              'details': {
                'candidates': [
                  {
                    'id': firstId,
                    'matched_by': ['entry_id', 'source', 'content'],
                  },
                  {
                    'id': secondId,
                    'matched_by': ['entry_id', 'source', 'content'],
                  },
                ],
              },
            },
          },
          publishedEntry: null,
        ),
      },
    );
    await tester.pumpWidget(_home(core, _Picker([path])));
    await tester.pumpAndSettle();

    await _chooseFolder(tester);
    await tester.tap(
      find.byKey(const ValueKey('import-feedback-details-toggle')),
    );
    await tester.pumpAndSettle();

    final details = tester.widget<SelectableText>(
      find.byKey(const ValueKey('import-feedback-details')),
    );
    expect(details.data, contains(firstId));
    expect(details.data, contains(secondId));
    expect(details.data!.runes.length, lessThanOrEqualTo(1025));
  });

  testWidgets(
    'generic failure admits partial publication and refreshes truth',
    (tester) async {
      const path = 'D:/picked/partial.zip';
      final core = _ImportCore(
        initialMods: [_entry('alpha', 'Alpha')],
        scenarios: {
          path: (
            response: {
              'ok': false,
              'error': {
                'code': 'IO',
                'message': 'loadout\nfollow-up failed\u202E',
              },
            },
            publishedEntry: _entry('partial-mod', 'Published Before Error'),
          ),
        },
      );
      await tester.pumpWidget(_home(core, _Picker([path])));
      await tester.pumpAndSettle();

      await _chooseFolder(tester);

      final l10n = await AppLocalizations.delegate.load(const Locale('en'));
      expect(find.text(l10n.importFailed), findsOneWidget);
      expect(
        l10n.importFailed,
        contains('may already have been added or updated'),
      );
      expect(
        l10n.importFailed.toLowerCase(),
        isNot(contains('nothing changed')),
      );
      expect(
        find.textContaining('Added “Published Before Error”'),
        findsNothing,
      );
      final container = ProviderScope.containerOf(
        tester.element(find.byType(HomePage)),
      );
      expect(container.read(libraryProvider).authoritative, isTrue);
      expect(
        container.read(libraryProvider).modById('partial-mod')?.name,
        'Published Before Error',
      );
      expect(container.read(libraryProvider).error, isNull);
    },
  );

  testWidgets('failed authoritative reload produces no success outcome', (
    tester,
  ) async {
    const path = 'D:/picked/reload-fails.zip';
    final core = _ImportCore(
      initialMods: [_entry('alpha', 'Alpha')],
      scenarios: {
        path: (
          response: _successWire(
            id: 'created-mod',
            disposition: 'created',
            matchedBy: 'none',
          ),
          publishedEntry: _entry('created-mod', 'Must Not Toast'),
        ),
      },
    )..failReloadAfterImport = true;
    await tester.pumpWidget(_home(core, _Picker([path])));
    await tester.pumpAndSettle();
    final container = ProviderScope.containerOf(
      tester.element(find.byType(HomePage)),
    );
    container.read(selectedModProvider.notifier).state = 'alpha';

    await _chooseFolder(tester);

    final l10n = await AppLocalizations.delegate.load(const Locale('en'));
    expect(find.text(l10n.importOutcomeUnknown), findsOneWidget);
    expect(find.textContaining('Added “Must Not Toast”'), findsNothing);
    expect(container.read(libraryProvider).authoritative, isFalse);
    expect(container.read(libraryProvider).mods, isEmpty);
    expect(container.read(selectedModProvider), 'alpha');
    expect(
      container.read(libraryProvider).error,
      contains('authoritative reload failed'),
    );
  });

  testWidgets('failed reload banner sanitizes and bounds native detail', (
    tester,
  ) async {
    const path = 'D:/picked/reload-detail.zip';
    final core =
        _ImportCore(
            scenarios: {
              path: (
                response: _successWire(
                  id: 'created-mod',
                  disposition: 'created',
                  matchedBy: 'none',
                ),
                publishedEntry: _entry('created-mod', 'Must Not Toast'),
              ),
            },
          )
          ..failReloadAfterImport = true
          ..reloadFailureMessage = 'unsafe\nreload\u202E${'x' * 900}';
    await tester.pumpWidget(_home(core, _Picker([path])));
    await tester.pumpAndSettle();

    await _chooseFolder(tester);

    final banner = tester
        .widgetList<Text>(find.byType(Text))
        .map((text) => text.data)
        .whereType<String>()
        .firstWhere((text) => text.startsWith('mgr_library_list: unsafe'));
    expect(banner, startsWith('mgr_library_list: unsafe reload'));
    expect(banner, isNot(contains('\n')));
    expect(banner, isNot(contains('\u202E')));
    expect(banner, endsWith('…'));
    expect(banner.runes.length, lessThanOrEqualTo(513));
  });
}
