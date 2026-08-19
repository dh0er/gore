import 'package:flutter/material.dart';
import 'package:flutter_localizations/flutter_localizations.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gore_manager/app/domain/ui_settings.dart';
import 'package:gore_manager/conflicts/ui/conflict_panel.dart';
import 'package:gore_manager/core/core_service.dart';
import 'package:gore_manager/core/providers.dart';
import 'package:gore_manager/l10n/app_localizations.dart';
import 'package:gore_manager/library/ui/detail_panel.dart';
import 'package:gore_manager/library/ui/mod_list.dart';

Map<String, Object?> _libraryList() => {
  'ok': true,
  'mods': [
    {
      'id': 'External.Mixed',
      'kind': 'foreign_mixed',
      'name': 'External Mixed Pack',
      'components': [
        {
          'type': 'loc_patch',
          'rel': 'loc/dialog',
          'targets': ['NPC_Diego_01|german'],
          'coverage': 'exact',
        },
        {
          'type': 'raw_file',
          'rel': 'raw/SFX.bank',
          'target_file': {
            'bank': {'name': 'SFX'},
          },
          'coverage': 'exact',
        },
        {
          'type': 'ue4ss_lua',
          'name': 'Runtime',
          'rel': 'ue4ss/Runtime',
          'targets': ['BP_Player.Health'],
          'opaque': true,
          'coverage': 'partial',
        },
        {
          'type': 'triplet',
          'rel_base': 'containers/Community',
          'targets': ['/Game/Community/Observed'],
          'coverage': 'advisory',
        },
        {
          'type': 'loose_pak',
          'rel': 'paks/Unreadable_P.pak',
          'targets': <String>[],
          'coverage': 'opaque',
        },
      ],
    },
  ],
  'loadout': {
    'format': 1,
    'entries': [
      {'id': 'External.Mixed', 'enabled': true},
    ],
  },
};

FakeGoreCoreFfiService _core({List<Object?> conflicts = const <Object?>[]}) =>
    FakeGoreCoreFfiService(
      responses: {
        'mgr_library_list': _libraryList(),
        'mgr_analyze': {'ok': true, 'conflicts': conflicts},
      },
    );

Widget _app(Widget child, {FakeGoreCoreFfiService? core}) => ProviderScope(
  overrides: [coreServiceProvider.overrideWithValue(core ?? _core())],
  child: MaterialApp(
    localizationsDelegates: const [
      AppLocalizations.delegate,
      GlobalMaterialLocalizations.delegate,
      GlobalWidgetsLocalizations.delegate,
      GlobalCupertinoLocalizations.delegate,
    ],
    supportedLocales: AppLocalizations.supportedLocales,
    builder: (context, built) => MediaQuery(
      data: MediaQuery.of(
        context,
      ).copyWith(textScaler: const TextScaler.linear(2)),
      child: built!,
    ),
    home: Scaffold(body: child),
  ),
);

void _compact200Percent(WidgetTester tester) {
  tester.view.physicalSize = const Size(700, 460);
  tester.view.devicePixelRatio = 1;
  addTearDown(tester.view.resetPhysicalSize);
  addTearDown(tester.view.resetDevicePixelRatio);
}

void main() {
  test('coverage copy is generated for all 12 supported locales', () async {
    expect(AppLocalizations.supportedLocales, hasLength(12));
    final english = await AppLocalizations.delegate.load(const Locale('en'));
    expect(english.conflictWinner, 'wins');
    final chinese = await AppLocalizations.delegate.load(const Locale('zh'));
    expect(chinese.conflictWinner, '生效');
    for (final locale in AppLocalizations.supportedLocales) {
      final l10n = await AppLocalizations.delegate.load(locale);
      expect(
        [
          l10n.footprintTargetsExact,
          l10n.footprintTargetsPartial,
          l10n.footprintTargetsAdvisory,
          l10n.footprintTargetsOpaque,
          l10n.footprintCoverageScope,
          l10n.loadOrderDirection,
          l10n.conflictCoverageIncomplete,
        ].every((value) => value.trim().isNotEmpty),
        isTrue,
        reason: locale.toLanguageTag(),
      );
    }
  });

  testWidgets(
    'component detail exposes all grades and raw footprint at compact 200%',
    (tester) async {
      _compact200Percent(tester);
      final semantics = tester.ensureSemantics();
      await tester.pumpWidget(_app(const DetailPanel()));
      await tester.pumpAndSettle();

      final container = ProviderScope.containerOf(
        tester.element(find.byType(DetailPanel)),
      );
      container.read(advancedDetailsProvider.notifier).set(true);
      container.read(selectedModProvider.notifier).state = 'External.Mixed';
      await tester.pumpAndSettle();

      final l10n = await AppLocalizations.delegate.load(const Locale('en'));
      // The grade heads the list it grades, so it reads as one sentence.
      expect(find.text(l10n.footprintTargetsExact), findsOneWidget);
      // It must sit above the list it introduces, or it anchors nothing. The
      // row's merged semantics carry it in that same reading order.
      expect(
        tester.getRect(find.text(l10n.footprintTargetsExact)).top,
        lessThan(tester.getRect(find.text('NPC_Diego_01|german')).top),
      );
      expect(
        tester
            .getSemantics(find.text(l10n.footprintTargetsExact))
            .label
            .replaceAll(RegExp(r'\s+'), ' '),
        contains(l10n.footprintTargetsExact),
      );
      // The raw_file below it is Exact too but has nothing to list, so it gets
      // no dangling "the full list:" heading over empty space.
      expect(
        find.byKey(const ValueKey('component-footprint-coverage-1')),
        findsNothing,
      );
      final scrollable = find.descendant(
        of: find.byType(DetailPanel),
        matching: find.byType(Scrollable),
      );
      for (final text in [
        // The destination names the row; the path is the advanced-only extra.
        '${l10n.rawTargetSoundBankNamed('SFX')} · raw/SFX.bank',
        l10n.footprintTargetsPartial,
        l10n.footprintTargetsAdvisory,
        // Nothing to list here, so this one is a statement, not a heading.
        l10n.footprintTargetsOpaque,
      ]) {
        await tester.scrollUntilVisible(
          find.text(text),
          120,
          scrollable: scrollable,
        );
        expect(find.text(text), findsOneWidget);
      }
      expect(tester.takeException(), isNull);
      semantics.dispose();
    },
  );

  testWidgets(
    'zero findings stay qualified when enabled coverage is non-exact',
    (tester) async {
      _compact200Percent(tester);
      final semantics = tester.ensureSemantics();
      await tester.pumpWidget(
        _app(const SizedBox(height: 72, child: ConflictPanel())),
      );
      await tester.pumpAndSettle();
      ProviderScope.containerOf(
        tester.element(find.byType(ConflictPanel)),
      ).read(advancedDetailsProvider.notifier).set(true);
      await tester.pumpAndSettle();

      final l10n = await AppLocalizations.delegate.load(const Locale('en'));
      expect(find.text(l10n.noConflicts), findsOneWidget);
      expect(find.text('No conflicts.'), findsNothing);
      final panelRect = tester.getRect(find.byType(ConflictPanel));
      final resultRect = tester.getRect(find.text(l10n.noConflicts));
      expect(resultRect.top, lessThan(panelRect.bottom));
      expect(resultRect.bottom, greaterThan(panelRect.top));
      final knowledgeNote = find.byKey(
        const ValueKey('conflict-knowledge-note'),
      );
      await tester.scrollUntilVisible(
        knowledgeNote,
        60,
        scrollable: find.descendant(
          of: find.byType(ConflictPanel),
          matching: find.byType(Scrollable),
        ),
      );
      expect(find.text(l10n.conflictCoverageIncomplete), findsOneWidget);
      expect(find.text(l10n.loadOrderDirection), findsOneWidget);
      expect(find.text(l10n.footprintCoverageScope), findsOneWidget);
      final label = tester.getSemantics(knowledgeNote).label;
      expect(label, contains(l10n.conflictCoverageIncomplete));
      expect(label, contains(l10n.loadOrderDirection));
      expect(label, contains(l10n.footprintCoverageScope));
      expect(tester.takeException(), isNull);
      semantics.dispose();
    },
  );

  testWidgets('compact findings show a conflict row before the coverage note', (
    tester,
  ) async {
    _compact200Percent(tester);
    final core = _core(
      conflicts: [
        {
          'kind': 'ue4ss_unknown',
          'target': '<unknown>',
          'mods': ['External.Mixed', 'External.Other'],
          'severity': 'info',
        },
      ],
    );
    await tester.pumpWidget(
      _app(const SizedBox(height: 72, child: ConflictPanel()), core: core),
    );
    await tester.pumpAndSettle();

    final panelRect = tester.getRect(find.byType(ConflictPanel));
    final rowRect = tester.getRect(find.byType(ConflictRow));
    expect(rowRect.top, lessThan(panelRect.bottom));
    expect(rowRect.bottom, greaterThan(panelRect.top));
    final knowledgeNote = find.byKey(const ValueKey('conflict-knowledge-note'));
    await tester.scrollUntilVisible(
      knowledgeNote,
      60,
      scrollable: find.descendant(
        of: find.byType(ConflictPanel),
        matching: find.byType(Scrollable),
      ),
    );
    expect(knowledgeNote, findsOneWidget);
    expect(tester.takeException(), isNull);
  });
}
