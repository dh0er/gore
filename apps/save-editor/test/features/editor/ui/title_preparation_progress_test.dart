import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter_localizations/flutter_localizations.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:goresave/features/editor/domain/core_service.dart';
import 'package:goresave/features/editor/domain/item_icon_catalog.dart';
import 'package:goresave/features/editor/ui/title_preparation_progress.dart';
import 'package:goresave/features/localization/domain/localization_controller.dart';
import 'package:goresave/l10n/app_localizations.dart';
import 'package:goresave/loc/loc_catalog_provider.dart';
import 'package:goresave/providers/data_providers.dart';

void main() {
  testWidgets('item-image activity stays visible until preparation completes', (
    tester,
  ) async {
    final completion = Completer<ItemIconCatalog>();
    final container = ProviderContainer(
      overrides: [
        itemIconCatalogProvider.overrideWith((ref) => completion.future),
        locCatalogProvider.overrideWith((ref) async => const {}),
      ],
    );
    addTearDown(container.dispose);

    await tester.pumpWidget(_testApp(container));
    await tester.pump();

    expect(
      find.byKey(const ValueKey('title-progress-item-images')),
      findsOneWidget,
    );
    expect(find.byType(LinearProgressIndicator), findsOneWidget);

    completion.complete(const ItemIconCatalog.empty());
    await tester.pumpAndSettle();
    expect(find.byType(TitlePreparationProgress), findsOneWidget);
    expect(find.byType(LinearProgressIndicator), findsNothing);
  });

  testWidgets('localization activity follows the extraction controller', (
    tester,
  ) async {
    final response = Completer<Map<String, Object?>>();
    final core = _DelayedCore(response);
    final container = ProviderContainer(
      overrides: [
        coreServiceProvider.overrideWithValue(core),
        itemIconCatalogProvider.overrideWith(
          (ref) async => const ItemIconCatalog.empty(),
        ),
        locCatalogProvider.overrideWith((ref) async => const {}),
      ],
    );
    addTearDown(container.dispose);

    await tester.pumpWidget(_testApp(container));
    await tester.pumpAndSettle();
    expect(find.byType(LinearProgressIndicator), findsNothing);

    final extraction = container
        .read(localizationControllerProvider.notifier)
        .extract();
    await tester.pump();
    expect(
      find.byKey(const ValueKey('title-progress-game-text')),
      findsOneWidget,
    );

    response.complete({
      'ok': true,
      'data': {
        'meta': {
          'id_count': 10,
          'languages': ['de', 'en'],
        },
      },
    });
    await extraction;
    await tester.pumpAndSettle();
    expect(find.byType(LinearProgressIndicator), findsNothing);
  });

  testWidgets('both jobs use two wide lanes and one compact lane', (
    tester,
  ) async {
    final localizationResponse = Completer<Map<String, Object?>>();
    final itemCompletion = Completer<ItemIconCatalog>();
    final container = ProviderContainer(
      overrides: [
        coreServiceProvider.overrideWithValue(
          _DelayedCore(localizationResponse),
        ),
        itemIconCatalogProvider.overrideWith((ref) => itemCompletion.future),
        locCatalogProvider.overrideWith((ref) async => const {}),
      ],
    );
    addTearDown(container.dispose);

    await tester.pumpWidget(_testApp(container, width: 400));
    await tester.pump();
    final extraction = container
        .read(localizationControllerProvider.notifier)
        .extract();
    await tester.pump();

    expect(
      find.byKey(const ValueKey('title-progress-game-text')),
      findsOneWidget,
    );
    expect(
      find.byKey(const ValueKey('title-progress-item-images')),
      findsOneWidget,
    );
    expect(find.byType(LinearProgressIndicator), findsNWidgets(2));
    expect(
      tester
          .widgetList<LinearProgressIndicator>(
            find.byType(LinearProgressIndicator),
          )
          .every((indicator) => indicator.value == null),
      isTrue,
    );
    expect(
      find.bySemanticsLabel('Loading texts…, Loading images…'),
      findsOneWidget,
    );

    await tester.pumpWidget(_testApp(container, width: 240));
    await tester.pump();
    expect(
      find.byKey(const ValueKey('title-progress-combined')),
      findsOneWidget,
    );
    expect(find.byType(LinearProgressIndicator), findsOneWidget);
    expect(tester.takeException(), isNull);

    localizationResponse.complete({
      'ok': true,
      'data': {
        'meta': {
          'id_count': 10,
          'languages': ['de'],
        },
      },
    });
    itemCompletion.complete(const ItemIconCatalog.empty());
    await extraction;
    await tester.pumpAndSettle();
  });

  testWidgets('background catalog reloads do not restore either progress bar', (
    tester,
  ) async {
    final localizationRefresh = Completer<Map<String, Map<String, String>>>();
    final itemRefresh = Completer<ItemIconCatalog>();
    var localizationLoads = 0;
    var itemLoads = 0;
    final container = ProviderContainer(
      overrides: [
        locCatalogProvider.overrideWith((ref) {
          ref.watch(locCatalogReloadProvider);
          localizationLoads++;
          return localizationLoads == 1
              ? Future.value(const <String, Map<String, String>>{})
              : localizationRefresh.future;
        }),
        itemIconCatalogProvider.overrideWith((ref) {
          ref.watch(itemIconCatalogReloadProvider);
          itemLoads++;
          return itemLoads == 1
              ? Future.value(const ItemIconCatalog.empty())
              : itemRefresh.future;
        }),
      ],
    );
    addTearDown(container.dispose);

    await tester.pumpWidget(_testApp(container));
    await tester.pumpAndSettle();
    expect(find.byType(LinearProgressIndicator), findsNothing);

    container.read(locCatalogReloadProvider.notifier).state++;
    container.read(itemIconCatalogReloadProvider.notifier).state++;
    await tester.pump();

    expect(container.read(locCatalogProvider).isLoading, isTrue);
    expect(container.read(locCatalogProvider).hasValue, isTrue);
    expect(container.read(itemIconCatalogProvider).isLoading, isTrue);
    expect(container.read(itemIconCatalogProvider).hasValue, isTrue);
    expect(find.byType(LinearProgressIndicator), findsNothing);

    localizationRefresh.complete(const {});
    itemRefresh.complete(const ItemIconCatalog.empty());
    await tester.pumpAndSettle();
  });
}

Widget _testApp(ProviderContainer container, {double? width}) {
  return UncontrolledProviderScope(
    container: container,
    child: MaterialApp(
      localizationsDelegates: const [
        AppLocalizations.delegate,
        GlobalMaterialLocalizations.delegate,
        GlobalWidgetsLocalizations.delegate,
        GlobalCupertinoLocalizations.delegate,
      ],
      supportedLocales: AppLocalizations.supportedLocales,
      home: Scaffold(
        body: Center(
          child: SizedBox(
            width: width,
            child: const TitlePreparationProgress(),
          ),
        ),
      ),
    ),
  );
}

class _DelayedCore implements GoresaveCoreService {
  const _DelayedCore(this.response);

  final Completer<Map<String, Object?>> response;

  @override
  String get description => 'delayed-test-core';

  @override
  bool get isAvailable => true;

  @override
  Future<Map<String, Object?>> execute(
    String command, {
    Map<String, Object?> payload = const {},
  }) {
    expect(command, 'loc_extract');
    return response.future;
  }
}
