import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:goresave/features/editor/domain/editor_models.dart';
import 'package:goresave/features/editor/domain/item_catalog.dart';
import 'package:goresave/features/editor/ui/add_inventory_item_dialog.dart';

import '../../../support/l10n_test_app.dart';

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Fake catalog with entries across 3 categories plus one entry that is
/// already in the inventory (to verify exclusion).
const _fakeCatalogJson = '''
[
  {"id": "ItMi_Orenugget",    "path": "/Script/Angelscript.ItMi_Orenugget",    "category": "misc"},
  {"id": "ItMi_Sulfur",       "path": "/Script/Angelscript.ItMi_Sulfur",       "category": "misc"},
  {"id": "ItFo_Bread",        "path": "/Script/Angelscript.ItFo_Bread",        "category": "food"},
  {"id": "ItMw_Sword_01",     "path": "/Script/Angelscript.ItMw_Sword_01",     "category": "meleweapon"},
  {"id": "ItMi_AlreadyOwned", "path": "/Script/Angelscript.ItMi_AlreadyOwned", "category": "misc"}
]
''';

ItemCatalog _fakeCatalog() => ItemCatalog.fromJsonString(_fakeCatalogJson);

/// Path of an item the player already owns in the MainContainer, used to
/// verify it is excluded from the picker list.
const _ownedPath = '/Script/Angelscript.ItMi_AlreadyOwned';

/// Wraps the dialog in a host that captures the pop result, so tests can
/// assert on the returned [InventoryItemAdd].
class _DialogHost extends StatefulWidget {
  const _DialogHost({
    required this.excludePaths,
    required this.onResult,
  });

  final Set<String> excludePaths;
  final void Function(InventoryItemAdd?) onResult;

  @override
  State<_DialogHost> createState() => _DialogHostState();
}

class _DialogHostState extends State<_DialogHost> {
  @override
  void initState() {
    super.initState();
    WidgetsBinding.instance.addPostFrameCallback((_) {
      showDialog<InventoryItemAdd>(
        context: context,
        builder: (_) => AddInventoryItemDialog(
          excludePaths: widget.excludePaths,
          catalogOverride: Future.value(_fakeCatalog()),
        ),
      ).then(widget.onResult);
    });
  }

  @override
  Widget build(BuildContext context) => const SizedBox.expand();
}

Widget _wrap({
  required Set<String> excludePaths,
  void Function(InventoryItemAdd?)? onResult,
}) {
  return wrapWithL10n(
    Scaffold(
      body: _DialogHost(
        excludePaths: excludePaths,
        onResult: onResult ?? (_) {},
      ),
    ),
  );
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

void main() {
  testWidgets('sidebar lists categories; selecting one shows its items',
      (tester) async {
    await tester.pumpWidget(
      _wrap(excludePaths: {_ownedPath}),
    );
    // Wait for dialog to open and FutureBuilder to resolve.
    await tester.pumpAndSettle();

    // Category tiles present in the sidebar (label + count from the catalog,
    // already-owned misc item excluded so misc count is 2).
    expect(find.text('Melee weapons (1)'), findsOneWidget);
    expect(find.text('Food & potions (1)'), findsOneWidget);
    expect(find.text('Miscellaneous (2)'), findsOneWidget);

    // Default selection is the first category (Melee weapons) → only its
    // item is shown in the detail pane.
    expect(find.text('Sword 01'), findsOneWidget);
    expect(find.text('Orenugget'), findsNothing);

    // Selecting Miscellaneous reveals both misc items, excluding the owned one.
    await tester.tap(find.text('Miscellaneous (2)'));
    await tester.pumpAndSettle();
    expect(find.text('Orenugget'), findsOneWidget);
    expect(find.text('Sulfur'), findsOneWidget);
    expect(find.text('AlreadyOwned'), findsNothing);
  });

  testWidgets('clearing search keeps the selection and reveals its category',
      (tester) async {
    await tester.pumpWidget(_wrap(excludePaths: const {}));
    await tester.pumpAndSettle();

    // Search for and select Sulfur (misc), which is not the default category.
    await tester.enterText(find.byType(TextField).first, 'sulfur');
    await tester.pump();
    await tester.tap(find.text('Sulfur'));
    await tester.pump();
    // Selection summary visible (search + count fields).
    expect(find.byType(TextField), findsNWidgets(2));

    // Clear the search: selection must survive, and its category becomes
    // active so the item stays on screen.
    await tester.enterText(find.byType(TextField).first, '');
    await tester.pump();

    expect(find.byType(TextField), findsNWidgets(2));
    // 'Sulfur' shows twice now: the selection summary and the revealed
    // misc-category list tile.
    expect(find.text('Sulfur'), findsNWidgets(2));
    final addButton = find.widgetWithText(FilledButton, 'Add');
    expect(
      tester.widget<FilledButton>(addButton).onPressed,
      isNotNull,
      reason: 'Selection (and thus Add) must survive clearing the search',
    );
  });

  testWidgets('search filters entries by id substring', (tester) async {
    await tester.pumpWidget(_wrap(excludePaths: const {}));
    await tester.pumpAndSettle();

    // Type 'sulfur' in the search field.
    await tester.enterText(find.byType(TextField).first, 'sulfur');
    await tester.pump();

    // Only Sulfur should be visible (case-insensitive).
    expect(find.text('Sulfur'), findsOneWidget);
    expect(find.text('Orenugget'), findsNothing);
    expect(find.text('Bread'), findsNothing);
  });

  testWidgets('selecting an entry and tapping Add pops with InventoryItemAdd',
      (tester) async {
    InventoryItemAdd? result;
    await tester.pumpWidget(
      _wrap(
        excludePaths: const {},
        onResult: (r) => result = r,
      ),
    );
    await tester.pumpAndSettle();

    // Filter the list to just 'Sulfur' so it is guaranteed to be on screen.
    await tester.enterText(find.byType(TextField).first, 'sulfur');
    await tester.pump();

    // Tap the 'Sulfur' entry to select it.
    await tester.tap(find.text('Sulfur'));
    await tester.pump();

    // The count field should appear (second TextField after the search field).
    // Default value is '1'.
    final allTextFields = find.byType(TextField);
    expect(allTextFields, findsNWidgets(2),
        reason: 'Search field + count field expected after selection');
    final countField = allTextFields.at(1);

    // Clear and enter 3.
    await tester.enterText(countField, '3');
    await tester.pump();

    // Add button should now be enabled.
    final addButton = find.widgetWithText(FilledButton, 'Add');
    expect(
      tester.widget<FilledButton>(addButton).onPressed,
      isNotNull,
      reason: 'Add button must be enabled when selection + valid count',
    );

    await tester.tap(addButton);
    await tester.pumpAndSettle();

    expect(result, isNotNull);
    expect(result!.path, '/Script/Angelscript.ItMi_Sulfur');
    expect(result!.count, 3);
  });

  testWidgets('invalid count disables the Add button', (tester) async {
    await tester.pumpWidget(_wrap(excludePaths: const {}));
    await tester.pumpAndSettle();

    // Search to bring Bread into the flat result list, then select it.
    await tester.enterText(find.byType(TextField).first, 'bread');
    await tester.pump();
    await tester.tap(find.text('Bread'));
    await tester.pump();

    // Count field is the second TextField (first is the search field).
    final countField = find.byType(TextField).at(1);

    // Enter count of 0 — invalid (FilteringTextInputFormatter allows '0').
    await tester.enterText(countField, '0');
    await tester.pump();

    final addButton = find.widgetWithText(FilledButton, 'Add');
    expect(
      tester.widget<FilledButton>(addButton).onPressed,
      isNull,
      reason: 'Add button must be disabled for count = 0',
    );

    // Clear the count — also invalid.
    await tester.enterText(countField, '');
    await tester.pump();

    expect(
      tester.widget<FilledButton>(addButton).onPressed,
      isNull,
      reason: 'Add button must be disabled for empty count',
    );
  });
}
