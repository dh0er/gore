import 'dart:async';
import 'dart:convert';

import 'package:flutter/material.dart';
import 'package:flutter_localizations/flutter_localizations.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/core/mod_ffi.dart';
import 'package:gore_mod/l10n/app_localizations.dart';
import 'package:gore_mod/project/revision3_content_index.dart';
import 'package:gore_mod/project/revision3_item_catalog.dart';
import 'package:gore_mod/project/revision3_item_patch_authoring.dart';
import 'package:gore_mod/project/revision3_items_view.dart';

import '../support/revision3_item_patch_fixture.dart';

void main() {
  testWidgets('browses bundled item facts without exposing mutation actions', (
    tester,
  ) async {
    tester.view.physicalSize = const Size(1200, 760);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.resetPhysicalSize);
    addTearDown(tester.view.resetDevicePixelRatio);

    await tester.pumpWidget(_app());
    await tester.pumpAndSettle();

    expect(
      find.byKey(const Key('revision3-items-wide-layout')),
      findsOneWidget,
    );
    expect(find.text('Apple'), findsWidgets);
    expect(find.text('ItFo_Apple'), findsWidgets);
    expect(find.text('Bundled reference'), findsOneWidget);
    expect(
      find.textContaining('not been refreshed or generation-qualified'),
      findsOneWidget,
    );
    expect(find.text('m_Value'), findsOneWidget);
    expect(find.text('int'), findsWidgets);
    expect(find.text('= 4'), findsOneWidget);
    expect(find.text('= 0'), findsOneWidget);
    expect(find.text('\u2265 0'), findsOneWidget);
    expect(find.text('\u2265 1'), findsNothing);
    expect(find.text('Edit'), findsNothing);
    expect(find.text('Create'), findsNothing);
    expect(find.text('Save'), findsNothing);
  });

  testWidgets('filters by search and category without inventing fields', (
    tester,
  ) async {
    tester.view.physicalSize = const Size(1200, 760);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.resetPhysicalSize);
    addTearDown(tester.view.resetDevicePixelRatio);

    await tester.pumpWidget(_app());
    await tester.pumpAndSettle();

    await tester.enterText(
      find.byKey(const Key('revision3-items-search')),
      'sword',
    );
    await tester.pump();
    expect(
      find.byKey(const Key('revision3-items-details-ItMw_Sword')),
      findsOneWidget,
    );
    expect(find.text('m_Weight'), findsOneWidget);

    await tester.tap(find.byKey(const Key('revision3-items-clear-search')));
    await tester.pump();
    await tester.tap(
      find.byKey(const Key('revision3-items-category-meleeWeapon')),
    );
    await tester.pump();
    expect(
      find.byKey(const Key('revision3-items-result-ItMw_Sword')),
      findsOneWidget,
    );
    expect(
      find.byKey(const Key('revision3-items-result-ItFo_Apple')),
      findsNothing,
    );

    await tester.enterText(
      find.byKey(const Key('revision3-items-search')),
      'unknown',
    );
    await tester.pump();
    expect(find.text('No items match'), findsOneWidget);

    await tester.tap(find.byKey(const Key('revision3-items-category-all')));
    await tester.pump();
    expect(
      find.byKey(const Key('revision3-items-details-ItMi_Unknown')),
      findsOneWidget,
    );
    expect(
      find.text('No modeled scalar fields are available for this item.'),
      findsOneWidget,
    );

    await tester.enterText(
      find.byKey(const Key('revision3-items-search')),
      'worldsplitter',
    );
    await tester.pump();
    expect(
      find.byKey(const Key('revision3-items-details-ItIg_Worldsplitter')),
      findsOneWidget,
    );
    expect(find.text('Special'), findsWidgets);
  });

  testWidgets('uses a compact drill-in and back pattern', (tester) async {
    tester.view.physicalSize = const Size(360, 640);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.resetPhysicalSize);
    addTearDown(tester.view.resetDevicePixelRatio);

    await tester.pumpWidget(_app());
    await tester.pumpAndSettle();

    expect(find.byKey(const Key('revision3-items-browser')), findsOneWidget);
    expect(find.byKey(const Key('revision3-items-detail-name')), findsNothing);

    await tester.tap(
      find.byKey(const Key('revision3-items-result-ItFo_Apple')),
    );
    await tester.pump();
    expect(
      find.byKey(const Key('revision3-items-detail-name')),
      findsOneWidget,
    );
    expect(find.byKey(const Key('revision3-items-browser')), findsNothing);

    await tester.tap(find.byKey(const Key('revision3-items-back')));
    await tester.pump();
    expect(find.byKey(const Key('revision3-items-browser')), findsOneWidget);
  });

  testWidgets('does not overflow at 320x180 with 200 percent text', (
    tester,
  ) async {
    tester.view.physicalSize = const Size(320, 180);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.resetPhysicalSize);
    addTearDown(tester.view.resetDevicePixelRatio);

    await tester.pumpWidget(_app(textScale: 2));
    await tester.pumpAndSettle();

    expect(find.byKey(const Key('revision3-items-browser')), findsOneWidget);
    expect(tester.takeException(), isNull);
  });

  testWidgets('reports a load failure and retries the stable loader', (
    tester,
  ) async {
    var attempts = 0;
    Future<Revision3ItemCatalog> load() async {
      attempts++;
      if (attempts == 1) throw const FormatException('bad bundled catalog');
      return _catalog();
    }

    await tester.pumpWidget(_app(load: load));
    await tester.pumpAndSettle();
    expect(find.byKey(const Key('revision3-items-load-error')), findsOneWidget);

    await tester.tap(find.byKey(const Key('revision3-items-retry')));
    await tester.pumpAndSettle();
    expect(find.byKey(const Key('revision3-items-browser')), findsOneWidget);
    expect(attempts, 2);
  });

  testWidgets('load failure scrolls at 320x180 with 200 percent text', (
    tester,
  ) async {
    tester.view.physicalSize = const Size(320, 180);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.resetPhysicalSize);
    addTearDown(tester.view.resetDevicePixelRatio);

    Future<Revision3ItemCatalog> fail() async =>
        throw const FormatException('bad bundled catalog');

    await tester.pumpWidget(_app(textScale: 2, load: fail));
    await tester.pumpAndSettle();

    expect(
      find.byKey(const Key('revision3-items-load-error-scroll')),
      findsOneWidget,
    );
    expect(find.byKey(const Key('revision3-items-retry')), findsOneWidget);
    expect(tester.takeException(), isNull);
  });

  testWidgets('shows exact managed item authoring controls and boundary', (
    tester,
  ) async {
    tester.view.physicalSize = const Size(1200, 760);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.resetPhysicalSize);
    addTearDown(tester.view.resetDevicePixelRatio);

    final service = _authoringService(
      loadContent: () async => _authoringContent(),
      publish: (_) async => throw StateError('must not publish'),
    );
    await tester.pumpWidget(_app(authoring: service));
    await tester.pumpAndSettle();

    expect(find.text('Exact project schema'), findsOneWidget);
    expect(find.text('Managed edit'), findsOneWidget);
    expect(
      find.byKey(const Key('revision3-items-authoring-boundary')),
      findsOneWidget,
    );
    expect(
      find.textContaining('saved only to this managed project'),
      findsOneWidget,
    );
    expect(
      find.textContaining('does not write to the game or a save'),
      findsOneWidget,
    );
    expect(
      find.byKey(const Key('revision3-items-add-m_Value')),
      findsOneWidget,
    );
    expect(
      find.byKey(const Key('revision3-items-add-m_Weight')),
      findsOneWidget,
    );
    expect(
      tester
          .widget<FilledButton>(find.byKey(const Key('revision3-items-save')))
          .onPressed,
      isNull,
    );
  });

  testWidgets('adds numeric override and publishes a create plan', (
    tester,
  ) async {
    tester.view.physicalSize = const Size(1200, 760);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.resetPhysicalSize);
    addTearDown(tester.view.resetDevicePixelRatio);

    late Revision3ItemPatchTechnicalPlan captured;
    var publications = 0;
    final service = _authoringService(
      loadContent: () async => _authoringContent(),
      publish: (plan) async {
        publications++;
        captured = plan;
        return Revision3ItemPatchPublication(
          projectId: _authoringProjectId,
          projectRevision: 8,
          entityId: plan.entityId,
          entityRevision: 0,
          change: AuthoringRevision3ItemPatchChange.created,
          vanillaClass: _authoringClass,
        );
      },
    );
    await tester.pumpWidget(_app(authoring: service));
    await tester.pumpAndSettle();

    await tester.tap(find.byKey(const Key('revision3-items-add-m_Value')));
    await tester.pump();
    await tester.enterText(
      find.byKey(const Key('revision3-items-value-m_Value')),
      '9',
    );
    await tester.pump();

    final save = find.byKey(const Key('revision3-items-save'));
    expect(tester.widget<FilledButton>(save).onPressed, isNotNull);
    await tester.ensureVisible(save);
    await tester.tap(save);
    await tester.pumpAndSettle();

    expect(publications, 1);
    expect(captured.action, AuthoringRevision3ItemPatchAction.upsert);
    expect(captured.expectedEntityRevision, isNull);
    expect(captured.vanillaClass, _authoringClass);
    expect(captured.fields['m_Value']!.integerValue, 9);
    expect(
      find.text('Item changes saved in project revision 8.'),
      findsOneWidget,
    );
  });

  testWidgets(
    'freezes the submitted draft and locks every field while save is pending',
    (tester) async {
      tester.view.physicalSize = const Size(1200, 760);
      tester.view.devicePixelRatio = 1;
      addTearDown(tester.view.resetPhysicalSize);
      addTearDown(tester.view.resetDevicePixelRatio);

      final publishStarted = Completer<void>();
      final releasePublish = Completer<Revision3ItemPatchPublication>();
      late Revision3ItemPatchTechnicalPlan captured;
      final service = _authoringService(
        loadContent: () async => _authoringContent(),
        publish: (plan) {
          captured = plan;
          publishStarted.complete();
          return releasePublish.future;
        },
      );
      await tester.pumpWidget(_app(authoring: service));
      await tester.pumpAndSettle();
      await tester.tap(find.byKey(const Key('revision3-items-add-m_Value')));
      await tester.pump();
      final valueField = find.byKey(const Key('revision3-items-value-m_Value'));
      await tester.enterText(valueField, '9');
      await tester.pump();
      final staleOnChanged = tester.widget<TextField>(valueField).onChanged!;
      final remove = find.byKey(const Key('revision3-items-remove-m_Value'));
      final staleRemove = tester.widget<IconButton>(remove).onPressed!;

      final save = find.byKey(const Key('revision3-items-save'));
      await tester.ensureVisible(save);
      await tester.tap(save);
      await tester.pump();
      await publishStarted.future;
      await tester.pump();

      expect(tester.widget<TextField>(valueField).enabled, isFalse);
      expect(tester.widget<IconButton>(remove).onPressed, isNull);
      expect(
        tester
            .widget<TextButton>(
              find.byKey(const Key('revision3-items-clear-all')),
            )
            .onPressed,
        isNull,
      );
      staleOnChanged('99');
      staleRemove();
      releasePublish.complete(
        Revision3ItemPatchPublication(
          projectId: _authoringProjectId,
          projectRevision: 8,
          entityId: captured.entityId,
          entityRevision: 0,
          change: AuthoringRevision3ItemPatchChange.created,
          vanillaClass: _authoringClass,
        ),
      );
      await tester.pumpAndSettle();

      expect(captured.fields.keys, <String>['m_Value']);
      expect(captured.fields['m_Value']!.integerValue, 9);
    },
  );

  testWidgets('reports managed item draft dirtiness until it is reverted', (
    tester,
  ) async {
    tester.view.physicalSize = const Size(1200, 760);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.resetPhysicalSize);
    addTearDown(tester.view.resetDevicePixelRatio);

    final reports = <bool>[];
    final service = _authoringService(
      loadContent: () async => _authoringContent(),
      publish: (_) async => throw StateError('must not publish'),
    );
    await tester.pumpWidget(
      _app(authoring: service, onDirtyChanged: reports.add),
    );
    await tester.pumpAndSettle();
    await tester.tap(find.byKey(const Key('revision3-items-add-m_Value')));
    await tester.pump();
    expect(reports, <bool>[true]);

    await tester.tap(find.byKey(const Key('revision3-items-remove-m_Value')));
    await tester.pump();
    expect(reports, <bool>[true, false]);
  });

  testWidgets('maps catalog failures to safe localized recovery actions', (
    tester,
  ) async {
    tester.view.physicalSize = const Size(1200, 760);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.resetPhysicalSize);
    addTearDown(tester.view.resetDevicePixelRatio);

    var loads = 0;
    final stale = _authoringService(
      loadContent: () async {
        loads++;
        throw const Revision3ItemPatchStaleCheckpointException();
      },
      publish: (_) async => throw StateError('must not publish'),
    );
    await tester.pumpWidget(_app(authoring: stale));
    await tester.pumpAndSettle();
    expect(find.text('Items are unavailable'), findsOneWidget);
    expect(
      find.text(
        'The project or exact item catalog changed before the item data could be loaded. Nothing was changed.',
      ),
      findsOneWidget,
    );
    expect(find.textContaining('Instance of'), findsNothing);
    expect(find.byKey(const Key('revision3-items-retry')), findsOneWidget);
    await tester.tap(find.byKey(const Key('revision3-items-retry')));
    await tester.pumpAndSettle();
    expect(loads, 2);

    await tester.pumpWidget(const SizedBox.shrink());
    await tester.pump();
    final unsupported = _authoringService(
      loadContent: () async =>
          throw const Revision3ItemPatchUnsupportedSchemaException(),
      publish: (_) async => throw StateError('must not publish'),
    );
    await tester.pumpWidget(_app(authoring: unsupported));
    await tester.pumpAndSettle();
    expect(
      find.text(
        'This project contains item data that the current exact game schema cannot edit safely. Nothing was changed.',
      ),
      findsOneWidget,
    );
    expect(find.textContaining('Instance of'), findsNothing);
    expect(find.byKey(const Key('revision3-items-retry')), findsNothing);
  });

  testWidgets('requires-reopen Item state offers project recovery only', (
    tester,
  ) async {
    var recoveryCalls = 0;
    await tester.pumpWidget(
      _app(
        authoringRequiresReopen: true,
        onRecoverAuthoring: () => recoveryCalls++,
      ),
    );
    await tester.pump();

    expect(
      find.text(
        'The exact project checkpoint can no longer be verified safely. Recover the project, or close and reopen it, before editing items.',
      ),
      findsOneWidget,
    );
    expect(find.byKey(const Key('revision3-items-recover')), findsOneWidget);
    expect(find.byKey(const Key('revision3-items-retry')), findsNothing);
    await tester.tap(find.byKey(const Key('revision3-items-recover')));
    expect(recoveryCalls, 1);
  });

  testWidgets('invalid numeric override disables save and never publishes', (
    tester,
  ) async {
    tester.view.physicalSize = const Size(1200, 760);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.resetPhysicalSize);
    addTearDown(tester.view.resetDevicePixelRatio);

    var publications = 0;
    final service = _authoringService(
      loadContent: () async => _authoringContent(),
      publish: (_) async {
        publications++;
        throw StateError('must not publish');
      },
    );
    await tester.pumpWidget(_app(authoring: service));
    await tester.pumpAndSettle();

    await tester.tap(find.byKey(const Key('revision3-items-add-m_Value')));
    await tester.pump();
    await tester.enterText(
      find.byKey(const Key('revision3-items-value-m_Value')),
      'not-a-number',
    );
    await tester.pump();

    expect(find.text('Enter a valid number.'), findsOneWidget);
    expect(
      tester
          .widget<FilledButton>(find.byKey(const Key('revision3-items-save')))
          .onPressed,
      isNull,
    );
    expect(publications, 0);
  });

  testWidgets('native-domain error is friendly and disables publication', (
    tester,
  ) async {
    tester.view.physicalSize = const Size(1200, 760);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.resetPhysicalSize);
    addTearDown(tester.view.resetDevicePixelRatio);

    var publications = 0;
    final service = _authoringService(
      loadContent: () async => _authoringContent(),
      publish: (_) async {
        publications++;
        throw StateError('must not publish');
      },
    );
    await tester.pumpWidget(_app(authoring: service));
    await tester.pumpAndSettle();

    await tester.tap(find.byKey(const Key('revision3-items-add-m_Value')));
    await tester.pump();
    await tester.enterText(
      find.byKey(const Key('revision3-items-value-m_Value')),
      '2147483648',
    );
    await tester.pump();

    expect(
      find.text('Enter a value from -2147483648 to 2147483647.'),
      findsOneWidget,
    );
    expect(
      tester
          .widget<FilledButton>(find.byKey(const Key('revision3-items-save')))
          .onPressed,
      isNull,
    );

    await tester.enterText(
      find.byKey(const Key('revision3-items-value-m_Value')),
      '2147483647',
    );
    await tester.pump();
    expect(
      find.text('Enter a value from -2147483648 to 2147483647.'),
      findsNothing,
    );

    await tester.tap(find.byKey(const Key('revision3-items-add-m_Weight')));
    await tester.pump();
    await tester.enterText(
      find.byKey(const Key('revision3-items-value-m_Weight')),
      '1e39',
    );
    await tester.pump();
    expect(
      find.text(
        'Enter a value from -3.4028234663852886e+38 to 3.4028234663852886e+38.',
      ),
      findsOneWidget,
    );
    expect(
      tester
          .widget<FilledButton>(find.byKey(const Key('revision3-items-save')))
          .onPressed,
      isNull,
    );

    await tester.enterText(
      find.byKey(const Key('revision3-items-value-m_Weight')),
      '3.4028234663852886e+38',
    );
    await tester.pump();
    expect(
      find.text(
        'Enter a value from -3.4028234663852886e+38 to 3.4028234663852886e+38.',
      ),
      findsNothing,
    );
    expect(publications, 0);
  });

  testWidgets('maps expected save failures to friendly recovery guidance', (
    tester,
  ) async {
    tester.view.physicalSize = const Size(1200, 760);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.resetPhysicalSize);
    addTearDown(tester.view.resetDevicePixelRatio);

    final cases = <(Object, String, bool)>[
      (
        const Revision3ItemPatchStaleCheckpointException(),
        'The project or item catalog changed. Nothing was saved. Reload the current item data before editing again.',
        true,
      ),
      (
        const Revision3ItemPatchRequiresReopenException(),
        'The project checkpoint can no longer be verified safely. Nothing was saved. Use project recovery, or close and reopen the project.',
        false,
      ),
      (
        const Revision3ItemPatchNoChangesException(),
        'There is no current item change to save. Reload the item data to continue.',
        true,
      ),
      (
        const Revision3ItemPatchUnsupportedSchemaException(),
        'This change no longer fits the current safe item schema. Nothing was saved. Reload the item data before continuing.',
        true,
      ),
      (
        StateError('technical detail must stay hidden'),
        'Item changes could not be saved safely. Nothing was changed. Reopen the project and try again.',
        false,
      ),
    ];
    var loads = 0;
    for (final (error, message, canReload) in cases) {
      await tester.pumpWidget(const SizedBox.shrink());
      await tester.pump();
      final service = _authoringService(
        loadContent: () async {
          loads++;
          return _authoringContent();
        },
        publish: (_) async => throw error,
      );
      await tester.pumpWidget(_app(authoring: service));
      await tester.pumpAndSettle();
      await tester.tap(find.byKey(const Key('revision3-items-add-m_Value')));
      await tester.pump();
      await tester.enterText(
        find.byKey(const Key('revision3-items-value-m_Value')),
        '9',
      );
      await tester.pump();
      final save = find.byKey(const Key('revision3-items-save'));
      await tester.ensureVisible(save);
      await tester.tap(save);
      await tester.pumpAndSettle();

      expect(find.text(message), findsOneWidget);
      expect(find.textContaining('Instance of'), findsNothing);
      expect(find.textContaining('technical detail'), findsNothing);
      expect(
        find.byKey(const Key('revision3-items-reload-after-error')),
        canReload ? findsOneWidget : findsNothing,
      );
    }
    expect(loads, cases.length * 2);
  });

  testWidgets('stale recovery reload discards the old item draft explicitly', (
    tester,
  ) async {
    tester.view.physicalSize = const Size(1200, 760);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.resetPhysicalSize);
    addTearDown(tester.view.resetDevicePixelRatio);

    var loads = 0;
    final service = _authoringService(
      loadContent: () async {
        loads++;
        return _authoringContent();
      },
      publish: (_) async =>
          throw const Revision3ItemPatchStaleCheckpointException(),
    );
    await tester.pumpWidget(_app(authoring: service));
    await tester.pumpAndSettle();
    await tester.tap(find.byKey(const Key('revision3-items-add-m_Value')));
    await tester.pump();
    await tester.enterText(
      find.byKey(const Key('revision3-items-value-m_Value')),
      '9',
    );
    await tester.pump();
    final save = find.byKey(const Key('revision3-items-save'));
    await tester.ensureVisible(save);
    await tester.tap(save);
    await tester.pumpAndSettle();

    final reload = find.byKey(const Key('revision3-items-reload-after-error'));
    await tester.drag(
      find.byKey(const Key('revision3-items-details-$_authoringClass')),
      const Offset(0, -160),
    );
    await tester.pumpAndSettle();
    await tester.tap(reload);
    await tester.pumpAndSettle();

    expect(loads, 3);
    expect(
      find.byKey(const Key('revision3-items-value-m_Value')),
      findsNothing,
    );
    expect(
      find.byKey(const Key('revision3-items-add-m_Value')),
      findsOneWidget,
    );
  });

  testWidgets('keeps unsaved item changes while browsing and filtering', (
    tester,
  ) async {
    tester.view.physicalSize = const Size(1200, 760);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.resetPhysicalSize);
    addTearDown(tester.view.resetDevicePixelRatio);

    final service = _authoringService(
      loadContent: () async => _authoringContent(),
      publish: (_) async => throw StateError('must not publish'),
    );
    await tester.pumpWidget(_app(authoring: service));
    await tester.pumpAndSettle();

    await tester.tap(find.byKey(const Key('revision3-items-add-m_Value')));
    await tester.pump();
    await tester.enterText(
      find.byKey(const Key('revision3-items-value-m_Value')),
      '9',
    );
    await tester.enterText(
      find.byKey(const Key('revision3-items-search')),
      'no match',
    );
    await tester.pump();
    expect(find.byKey(const Key('revision3-items-empty')), findsOneWidget);

    await tester.tap(find.byKey(const Key('revision3-items-clear-search')));
    await tester.pump();

    expect(
      tester
          .widget<TextField>(
            find.byKey(const Key('revision3-items-value-m_Value')),
          )
          .controller!
          .text,
      '9',
    );
    expect(
      tester
          .widget<FilledButton>(find.byKey(const Key('revision3-items-save')))
          .onPressed,
      isNotNull,
    );
  });

  testWidgets(
    'same-project rebind keeps the sibling draft and browser context',
    (tester) async {
      tester.view.physicalSize = const Size(1200, 760);
      tester.view.devicePixelRatio = 1;
      addTearDown(tester.view.resetPhysicalSize);
      addTearDown(tester.view.resetDevicePixelRatio);

      var projectRevision = 7;
      var headDigit = 'b';
      var firstPatchPublished = false;
      late StateSetter rebind;
      final firstPublishStarted = Completer<void>();
      final releaseFirstPublish = Completer<void>();
      final plans = <Revision3ItemPatchTechnicalPlan>[];
      final dirtyReports = <bool>[];
      final savingReports = <bool>[];
      final ValueChanged<bool> reportDirty = dirtyReports.add;
      final ValueChanged<bool> reportSaving = savingReports.add;

      await tester.pumpWidget(
        StatefulBuilder(
          builder: (context, setState) {
            rebind = setState;
            final boundRevision = projectRevision;
            final boundHeadDigit = headDigit;
            final authoring = _authoringService(
              projectRevision: boundRevision,
              headDigit: boundHeadDigit,
              includeSecond: true,
              loadContent: () async => _authoringContent(
                projectRevision: boundRevision,
                patched: firstPatchPublished,
                patchedValue: 9,
                patchEntityRevision: 0,
              ),
              publish: (plan) async {
                plans.add(plan);
                if (boundRevision == 7) {
                  firstPublishStarted.complete();
                  rebind(() {
                    firstPatchPublished = true;
                    projectRevision = 8;
                    headDigit = 'e';
                  });
                  await releaseFirstPublish.future;
                  return Revision3ItemPatchPublication(
                    projectId: _authoringProjectId,
                    projectRevision: 8,
                    entityId: plan.entityId,
                    entityRevision: 0,
                    change: AuthoringRevision3ItemPatchChange.created,
                    vanillaClass: _authoringClass,
                  );
                }
                return Revision3ItemPatchPublication(
                  projectId: _authoringProjectId,
                  projectRevision: 9,
                  entityId: plan.entityId,
                  entityRevision: 0,
                  change: AuthoringRevision3ItemPatchChange.created,
                  vanillaClass: _authoringSecondClass,
                );
              },
            );
            return _app(
              authoring: authoring,
              onDirtyChanged: reportDirty,
              onSavingChanged: reportSaving,
            );
          },
        ),
      );
      await tester.pumpAndSettle();

      await tester.tap(find.byKey(const Key('revision3-items-add-m_Value')));
      await tester.pump();
      await tester.enterText(
        find.byKey(const Key('revision3-items-value-m_Value')),
        '9',
      );
      await tester.tap(
        find.byKey(const Key('revision3-items-result-$_authoringSecondClass')),
      );
      await tester.pump();
      await tester.tap(find.byKey(const Key('revision3-items-add-m_Value')));
      await tester.pump();
      await tester.enterText(
        find.byKey(const Key('revision3-items-value-m_Value')),
        '13',
      );
      await tester.enterText(
        find.byKey(const Key('revision3-items-search')),
        'ItFo',
      );
      await tester.tap(find.byKey(const Key('revision3-items-category-food')));
      await tester.pump();
      await tester.tap(
        find.byKey(const Key('revision3-items-result-$_authoringClass')),
      );
      await tester.pump();

      final firstSave = find.byKey(const Key('revision3-items-save'));
      await tester.ensureVisible(firstSave);
      await tester.tap(firstSave);
      await tester.pump();
      await firstPublishStarted.future;
      await tester.pumpAndSettle();

      expect(savingReports, <bool>[true]);
      expect(
        tester
            .widget<TextField>(find.byKey(const Key('revision3-items-search')))
            .controller!
            .text,
        'ItFo',
      );
      expect(
        tester
            .widget<ChoiceChip>(
              find.byKey(const Key('revision3-items-category-food')),
            )
            .selected,
        isTrue,
      );
      expect(
        tester
            .widget<ListTile>(
              find.byKey(const Key('revision3-items-result-$_authoringClass')),
            )
            .selected,
        isTrue,
      );

      releaseFirstPublish.complete();
      await tester.pumpAndSettle();

      expect(savingReports, <bool>[true, false]);
      expect(dirtyReports, <bool>[true]);
      await tester.tap(
        find.byKey(const Key('revision3-items-result-$_authoringSecondClass')),
      );
      await tester.pump();
      expect(
        tester
            .widget<TextField>(
              find.byKey(const Key('revision3-items-value-m_Value')),
            )
            .controller!
            .text,
        '13',
      );
      final secondSave = find.byKey(const Key('revision3-items-save'));
      expect(tester.widget<FilledButton>(secondSave).onPressed, isNotNull);
      await tester.ensureVisible(secondSave);
      await tester.tap(secondSave);
      await tester.pumpAndSettle();

      expect(plans, hasLength(2));
      expect(plans[0].expectedProjectRevision, 7);
      expect(plans[0].vanillaClass, _authoringClass);
      expect(plans[1].expectedProjectRevision, 8);
      expect(
        plans[1].expectedHead.canonicalJson,
        _authoringHead('e').canonicalJson,
      );
      expect(plans[1].vanillaClass, _authoringSecondClass);
      expect(plans[1].fields['m_Value']!.integerValue, 13);
      expect(dirtyReports, <bool>[true, false]);
    },
  );

  testWidgets('drops cached catalog and drafts when the project root changes', (
    tester,
  ) async {
    tester.view.physicalSize = const Size(1200, 760);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.resetPhysicalSize);
    addTearDown(tester.view.resetDevicePixelRatio);

    var rootALoads = 0;
    var rootBLoads = 0;
    final rootA = _authoringService(
      projectScopeIdentity: r'C:\mods\root-a',
      loadContent: () async {
        rootALoads++;
        return _authoringContent();
      },
      publish: (_) async => throw StateError('must not publish'),
    );
    final rootB = _authoringService(
      projectScopeIdentity: r'C:\mods\root-b',
      loadContent: () async {
        rootBLoads++;
        return _authoringContent();
      },
      publish: (_) async => throw StateError('must not publish'),
    );
    await tester.pumpWidget(_app(authoring: rootA));
    await tester.pumpAndSettle();
    await tester.tap(find.byKey(const Key('revision3-items-add-m_Value')));
    await tester.pump();
    await tester.enterText(
      find.byKey(const Key('revision3-items-value-m_Value')),
      '9',
    );
    await tester.pump();

    await tester.pumpWidget(_app(authoring: rootB));
    await tester.pumpAndSettle();

    expect(rootALoads, 1);
    expect(rootBLoads, 1);
    expect(
      find.byKey(const Key('revision3-items-value-m_Value')),
      findsNothing,
    );
    expect(
      find.byKey(const Key('revision3-items-add-m_Value')),
      findsOneWidget,
    );
    expect(
      tester
          .widget<FilledButton>(find.byKey(const Key('revision3-items-save')))
          .onPressed,
      isNull,
    );
  });

  testWidgets('managed editor does not overflow at 320x180 and 200 percent', (
    tester,
  ) async {
    tester.view.physicalSize = const Size(320, 180);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.resetPhysicalSize);
    addTearDown(tester.view.resetDevicePixelRatio);

    final service = _authoringService(
      loadContent: () async => _authoringContent(patched: true),
      publish: (_) async => throw StateError('must not publish'),
    );
    await tester.pumpWidget(_app(textScale: 2, authoring: service));
    await tester.pumpAndSettle();
    final item = find.byKey(
      const Key('revision3-items-result-$_authoringClass'),
    );
    await tester.drag(find.byType(CustomScrollView), const Offset(0, -160));
    await tester.pumpAndSettle();
    await tester.tap(item);
    await tester.pump();

    final details = find.byKey(
      const Key('revision3-items-details-$_authoringClass'),
    );
    expect(details, findsOneWidget);
    final boundary = find.byKey(
      const Key('revision3-items-authoring-boundary'),
    );
    await tester.dragUntilVisible(boundary, details, const Offset(0, -80));
    expect(boundary, findsOneWidget);
    final clearAll = find.byKey(const Key('revision3-items-clear-all'));
    await tester.dragUntilVisible(clearAll, details, const Offset(0, -80));
    expect(clearAll, findsOneWidget);
    expect(tester.takeException(), isNull);
  });

  testWidgets('reverts an existing patch with its stored provenance', (
    tester,
  ) async {
    tester.view.physicalSize = const Size(1200, 760);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.resetPhysicalSize);
    addTearDown(tester.view.resetDevicePixelRatio);

    late Revision3ItemPatchTechnicalPlan captured;
    final service = _authoringService(
      loadContent: () async => _authoringContent(patched: true),
      publish: (plan) async {
        captured = plan;
        return Revision3ItemPatchPublication(
          projectId: _authoringProjectId,
          projectRevision: 8,
          entityId: _authoringEntityId,
          entityRevision: null,
          change: AuthoringRevision3ItemPatchChange.removed,
          vanillaClass: _authoringClass,
        );
      },
    );
    await tester.pumpWidget(_app(authoring: service));
    await tester.pumpAndSettle();

    expect(find.text('1 changed field'), findsWidgets);
    await tester.tap(find.byKey(const Key('revision3-items-clear-all')));
    await tester.pump();
    expect(find.text('Revert item to game defaults'), findsOneWidget);

    final save = find.byKey(const Key('revision3-items-save'));
    await tester.ensureVisible(save);
    await tester.pumpAndSettle();
    await tester.tap(save);
    await tester.pumpAndSettle();

    expect(captured.action, AuthoringRevision3ItemPatchAction.remove);
    expect(captured.entityId, _authoringEntityId);
    expect(captured.expectedEntityRevision, 2);
    expect(captured.expectedCatalogLayer, _authoringCatalogLayer);
    expect(captured.expectedSourceSeal.sha256, 'd' * 64);
    expect(captured.expectedCatalogSeal.sha256, 'c' * 64);
  });
}

Widget _app({
  double textScale = 1,
  Future<Revision3ItemCatalog> Function()? load,
  Revision3ItemPatchAuthoringService? authoring,
  bool authoringRequiresReopen = false,
  VoidCallback? onRecoverAuthoring,
  ValueChanged<bool>? onDirtyChanged,
  ValueChanged<bool>? onSavingChanged,
}) => MaterialApp(
  localizationsDelegates: const <LocalizationsDelegate<dynamic>>[
    AppLocalizations.delegate,
    GlobalMaterialLocalizations.delegate,
    GlobalWidgetsLocalizations.delegate,
    GlobalCupertinoLocalizations.delegate,
  ],
  supportedLocales: AppLocalizations.supportedLocales,
  builder: (context, child) => MediaQuery(
    data: MediaQuery.of(
      context,
    ).copyWith(textScaler: TextScaler.linear(textScale)),
    child: child!,
  ),
  home: Scaffold(
    body: Revision3ItemsView(
      load: load ?? _loadCatalog,
      authoring: authoring,
      authoringRequiresReopen: authoringRequiresReopen,
      onRecoverAuthoring: onRecoverAuthoring,
      onDirtyChanged: onDirtyChanged,
      onSavingChanged: onSavingChanged,
    ),
  ),
);

Future<Revision3ItemCatalog> _loadCatalog() async => _catalog();

Revision3ItemCatalog _catalog() => Revision3ItemCatalog.fromJson(
  itemCatalogJson: '''
    [
      {"category":"melee_weapon","id":"ItMw_Sword"},
      {"category":"misc","id":"ItMi_Unknown"},
      {"category":"food","id":"ItFo_Apple"},
      {"category":"special","id":"ItIg_Worldsplitter"}
    ]
  ''',
  modelJson: '''
    {
      "classes": {
        "ItFo_Apple": {
          "fields": [
            {"name":"m_Value","type":"int","default":4,"min":0},
            {"name":"m_MaxStack","type":"int","default":0}
          ]
        },
        "ItMw_Sword": {
          "fields": [
            {"name":"m_Weight","type":"float","default":2.5}
          ]
        }
      }
    }
  ''',
);

const _authoringProjectId = '11111111111111111111111111111111';
const _authoringEntityId = '22222222222222222222222222222222';
const _authoringClass = 'ItFo_Apple';
const _authoringSecondClass = 'ItFo_Bread';
const _authoringCatalogLayer = 'base-game.items.g1r.bundled.v1';

AuthoringWorkingHead _authoringHead(String digit) =>
    AuthoringWorkingHead.fromCanonicalJson(
      jsonEncode(<String, Object?>{
        'store_format': 1,
        'snapshot': <String, Object?>{'byte_len': 321, 'sha256': digit * 64},
      }),
    );

Map<String, Object?> _authoringSeal(String digit, int bytes) =>
    <String, Object?>{'byte_len': bytes, 'sha256': digit * 64};

Map<String, Object?> _authoringTarget() => <String, Object?>{
  'executable': _authoringSeal('a', 171698176),
};

AuthoringRevision3ItemCatalogReadResult _authoringNativeCatalog({
  int projectRevision = 7,
  String headDigit = 'b',
  bool includeSecond = false,
}) {
  final vanillaClasses = <String>[
    _authoringClass,
    if (includeSecond) _authoringSecondClass,
  ];
  final catalogJson = jsonEncode(<String, Object?>{
    'catalog_layer': _authoringCatalogLayer,
    'catalog_seal': _authoringSeal('c', 9000),
    'entries': <Object?>[
      for (final vanillaClass in vanillaClasses)
        <String, Object?>{
          'category': 'food',
          'fields': <Object?>[
            revision3ItemNumericField(
              name: 'm_Value',
              scalarType: 'integer',
              defaultValue: <String, Object?>{'type': 'integer', 'data': 4},
            ),
            revision3ItemNumericField(
              name: 'm_Weight',
              scalarType: 'float',
              defaultValue: <String, Object?>{'type': 'float', 'data': 0.25},
            ),
          ],
          'runtime_path': '/Script/Angelscript.$vanillaClass',
          'source_seal': _authoringSeal('d', 500),
          'vanilla_class': vanillaClass,
        },
    ],
    'schema_revision': 1,
    'target': _authoringTarget(),
  });
  return AuthoringRevision3ItemCatalogReadResult.fromJson(<String, Object?>{
    'ok': true,
    'head_json': _authoringHead(headDigit).canonicalJson,
    'project_id': _authoringProjectId,
    'project_revision': projectRevision,
    'catalog_json': catalogJson,
    'catalog_seal': _authoringSeal('c', 9000),
    'catalog_authority': 'native_embedded_schema_exact_current_project',
    'build_status': 'not_evaluated',
    'runtime_status': 'runtime_unqualified',
    'publication_status': 'not_applicable',
  }, expectedHead: _authoringHead(headDigit));
}

Revision3ContentIndex _authoringContent({
  bool patched = false,
  int projectRevision = 7,
  int patchedValue = 4,
  int patchEntityRevision = 2,
  String catalogLayer = _authoringCatalogLayer,
  String sourceDigit = 'd',
}) {
  final entities = <Object?>[];
  final counts = <String, Object?>{};
  if (patched) {
    counts['item_patch'] = 1;
    entities.add(<String, Object?>{
      'id': _authoringEntityId,
      'kind': 'item_patch',
      'display_name': 'Apple',
      'revision': patchEntityRevision,
      'origin': <String, Object?>{
        'type': 'vanilla',
        'generation': _authoringTarget(),
        'catalog_layer': catalogLayer,
        'canonical_selector': _authoringClass,
        'source_seal': _authoringSeal(sourceDigit, 500),
      },
      'summary': <String, Object?>{
        'kind': 'item_patch',
        'data': <String, Object?>{
          'vanilla_class': _authoringClass,
          'field_count': 1,
          'field_types': <String, Object?>{'m_Value': 'integer'},
          'fields': <String, Object?>{
            'm_Value': <String, Object?>{
              'type': 'integer',
              'data': patchedValue,
            },
          },
        },
      },
      'references': <Object?>[],
      'asset_references': <Object?>[],
    });
  }
  return Revision3ContentIndex.fromJsonObject(<String, Object?>{
    'schema_revision': 1,
    'project_id': _authoringProjectId,
    'project_revision': projectRevision,
    'project_name': 'Managed items',
    'project_version': '1.0.0',
    'project_author': 'tests',
    'target': _authoringTarget(),
    'authoring_locales': <Object?>[],
    'entity_counts': counts,
    'entities': entities,
    'assets': <Object?>[],
  });
}

Revision3ItemPatchAuthoringService _authoringService({
  required Future<Revision3ContentIndex> Function() loadContent,
  required Revision3ItemPatchTechnicalPublisher publish,
  String projectScopeIdentity = 'test-project-root',
  int projectRevision = 7,
  String headDigit = 'b',
  bool includeSecond = false,
}) => Revision3ItemPatchAuthoringService(
  projectScopeIdentity: projectScopeIdentity,
  projectId: _authoringProjectId,
  projectRevision: projectRevision,
  expectedHead: _authoringHead(headDigit),
  loadContentIndex: loadContent,
  loadNativeCatalog: () async => _authoringNativeCatalog(
    projectRevision: projectRevision,
    headDigit: headDigit,
    includeSecond: includeSecond,
  ),
  publishTechnicalPlan: publish,
);
