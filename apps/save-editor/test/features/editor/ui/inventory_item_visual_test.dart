import 'dart:async';
import 'dart:convert';
import 'dart:io';

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:goresave/features/editor/domain/item_icon_catalog.dart';
import 'package:goresave/features/editor/ui/inventory_item_visual.dart';

void main() {
  for (final brightness in Brightness.values) {
    testWidgets(
      'cached game image renders unframed in ${brightness.name} mode',
      (tester) async {
        final root = Directory.systemTemp.createTempSync('gore_item_visual');
        addTearDown(() => root.deleteSync(recursive: true));
        final png = File('${root.path}${Platform.pathSeparator}apple.png')
          ..writeAsBytesSync(base64Decode(_onePixelPng));
        final colorScheme = ColorScheme.fromSeed(
          seedColor: Colors.teal,
          brightness: brightness,
        );
        await tester.pumpWidget(
          ProviderScope(
            overrides: [
              itemIconCatalogProvider.overrideWith(
                (ref) async => ItemIconCatalog(
                  buildId: 'generation',
                  manifestPath: '${root.path}/manifest.json',
                  pathByItemId: {'itfo_apple': png.path},
                ),
              ),
            ],
            child: MaterialApp(
              theme: ThemeData(colorScheme: colorScheme),
              home: const Scaffold(
                body: InventoryItemVisual(
                  key: ValueKey('inventory-item-image-test'),
                  itemId: 'ItFo_Apple',
                ),
              ),
            ),
          ),
        );
        await tester.pumpAndSettle();

        final visual = find.byKey(const ValueKey('inventory-item-image-test'));
        final image = find.descendant(of: visual, matching: find.byType(Image));

        expect(tester.getSize(visual), const Size.square(40));
        expect(tester.getSize(image), const Size.square(34));
        expect(find.byType(DecoratedBox), findsNothing);
        expect(tester.takeException(), isNull);
      },
    );
  }

  testWidgets('loading and unknown items keep the requested fallback icon', (
    tester,
  ) async {
    await tester.pumpWidget(
      ProviderScope(
        overrides: [
          itemIconCatalogProvider.overrideWith(
            (ref) => Completer<ItemIconCatalog>().future,
          ),
        ],
        child: const MaterialApp(
          home: Scaffold(
            body: InventoryItemVisual(
              itemId: 'ItFo_Apple',
              fallbackIcon: Icons.restaurant,
            ),
          ),
        ),
      ),
    );

    expect(find.byIcon(Icons.restaurant), findsOneWidget);
    expect(find.byType(Image), findsNothing);
    expect(find.byType(DecoratedBox), findsNothing);
  });

  testWidgets('adjacent item images keep their padded size and spacing', (
    tester,
  ) async {
    final root = Directory.systemTemp.createTempSync('gore_item_visual_gap');
    addTearDown(() => root.deleteSync(recursive: true));
    final png = File('${root.path}${Platform.pathSeparator}item.png')
      ..writeAsBytesSync(base64Decode(_onePixelPng));

    await tester.pumpWidget(
      ProviderScope(
        overrides: [
          itemIconCatalogProvider.overrideWith(
            (ref) async => ItemIconCatalog(
              buildId: 'generation',
              manifestPath: '${root.path}/manifest.json',
              pathByItemId: {'first': png.path, 'second': png.path},
            ),
          ),
        ],
        child: const MaterialApp(
          home: Scaffold(
            body: Column(
              mainAxisSize: MainAxisSize.min,
              children: [
                InventoryItemVisual(
                  key: ValueKey('first-item-visual'),
                  itemId: 'first',
                ),
                InventoryItemVisual(
                  key: ValueKey('second-item-visual'),
                  itemId: 'second',
                ),
              ],
            ),
          ),
        ),
      ),
    );
    await tester.pumpAndSettle();

    final firstImage = find.descendant(
      of: find.byKey(const ValueKey('first-item-visual')),
      matching: find.byType(Image),
    );
    final secondImage = find.descendant(
      of: find.byKey(const ValueKey('second-item-visual')),
      matching: find.byType(Image),
    );

    expect(tester.getSize(firstImage), const Size.square(34));
    expect(tester.getSize(secondImage), const Size.square(34));
    expect(
      tester.getRect(secondImage).top - tester.getRect(firstImage).bottom,
      6,
    );
  });

  testWidgets(
    'a completed first extraction replaces the fallback immediately',
    (tester) async {
      final root = Directory.systemTemp.createTempSync('gore_item_visual_swap');
      addTearDown(() => root.deleteSync(recursive: true));
      final png = File('${root.path}${Platform.pathSeparator}apple.png')
        ..writeAsBytesSync(base64Decode(_onePixelPng));
      final completion = Completer<ItemIconCatalog>();

      await tester.pumpWidget(
        ProviderScope(
          overrides: [
            itemIconCatalogProvider.overrideWith((ref) => completion.future),
          ],
          child: const MaterialApp(
            home: Scaffold(
              body: InventoryItemVisual(
                itemId: 'ItFo_Apple',
                fallbackIcon: Icons.restaurant,
              ),
            ),
          ),
        ),
      );
      expect(find.byIcon(Icons.restaurant), findsOneWidget);

      completion.complete(
        ItemIconCatalog(
          buildId: 'generation',
          manifestPath: '${root.path}/manifest.json',
          pathByItemId: {'itfo_apple': png.path},
        ),
      );
      await tester.pumpAndSettle();
      expect(find.byIcon(Icons.restaurant), findsNothing);
      expect(find.byType(Image), findsOneWidget);
    },
  );
}

// Valid 1x1 transparent RGBA PNG; the test exercises FileImage without
// depending on proprietary game data.
const _onePixelPng =
    'iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNk+M/wHwAF/gL+XwhuAAAAAElFTkSuQmCC';
