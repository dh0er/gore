import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/loc/domain/loc_catalog_provider.dart';
import 'package:gore_mod/loc/domain/loc_edits_notifier.dart';
import 'package:gore_mod/loc/game_lang.dart';
import 'package:gore_mod/loc/ui/lang_fields.dart';

void main() {
  const locId = 'dia_hello';
  const catalog = <String, Map<String, String>>{
    locId: {
      'english_newer': 'Hello',
      'german_new': 'Hallo',
      'french': 'Salut',
    },
  };

  Future<ProviderContainer> pumpEditor(
    WidgetTester tester, {
    bool onlyEdited = false,
    Map<String, Map<String, String>> edits = const {},
  }) async {
    final container = ProviderContainer(overrides: [
      locCatalogProvider.overrideWith((ref) => Future.value(catalog)),
    ]);
    addTearDown(container.dispose);
    container.read(locEditsProvider.notifier).loadAll(edits);
    await tester.pumpWidget(UncontrolledProviderScope(
      container: container,
      child: MaterialApp(
        home: Scaffold(
          // All 10 languages don't fit the default test surface — scroll so
          // every field is built and hit-testable.
          body: SingleChildScrollView(
            child: LangFieldsEditor(locId: locId, onlyEdited: onlyEdited),
          ),
        ),
      ),
    ));
    await tester.pumpAndSettle();
    return container;
  }

  testWidgets('main mode: delete button replaces undo and clears the edit',
      (tester) async {
    final container = await pumpEditor(tester, edits: {
      locId: {'german_new': 'Servus'},
    });

    // All languages render regardless of edits.
    expect(find.byType(TextField), findsNWidgets(kGameLangs.length));
    // The modified field carries a delete button, not the old undo icon.
    expect(find.byIcon(Icons.undo), findsNothing);
    final deleteBtn = find.byIcon(Icons.delete_outline);
    expect(deleteBtn, findsOneWidget);
    expect(find.text('Servus'), findsOneWidget);

    await tester.ensureVisible(deleteBtn);
    await tester.tap(deleteBtn);
    await tester.pumpAndSettle();

    // The change entry is gone and the field synced back to the catalog value.
    expect(container.read(locEditsProvider).isDirty, false);
    expect(find.text('Servus'), findsNothing);
    expect(find.text('Hallo'), findsOneWidget);
    expect(find.byIcon(Icons.delete_outline), findsNothing);
    // The field itself stays in main mode.
    expect(find.byType(TextField), findsNWidgets(kGameLangs.length));
  });

  testWidgets('onlyEdited shows only the edited languages', (tester) async {
    await pumpEditor(tester, onlyEdited: true, edits: {
      locId: {'german_new': 'Servus', 'french': 'Coucou'},
    });

    expect(find.byType(TextField), findsNWidgets(2));
    expect(find.widgetWithText(TextField, 'Deutsch'), findsOneWidget);
    expect(find.widgetWithText(TextField, 'Français'), findsOneWidget);
    expect(find.widgetWithText(TextField, 'English'), findsNothing);
  });

  testWidgets(
      'onlyEdited: delete removes exactly that language and its field '
      'disappears; the last delete leaves nothing', (tester) async {
    final container = await pumpEditor(tester, onlyEdited: true, edits: {
      locId: {'german_new': 'Servus', 'french': 'Coucou'},
    });
    expect(find.byType(TextField), findsNWidgets(2));

    // Delete the German change: its field vanishes, French stays intact.
    await tester.tap(find.descendant(
      of: find.widgetWithText(TextField, 'Deutsch'),
      matching: find.byIcon(Icons.delete_outline),
    ));
    await tester.pumpAndSettle();
    expect(find.widgetWithText(TextField, 'Deutsch'), findsNothing);
    expect(find.widgetWithText(TextField, 'Français'), findsOneWidget);
    expect(find.text('Coucou'), findsOneWidget);
    final s = container.read(locEditsProvider);
    expect(s.editFor(locId, 'german_new'), isNull);
    expect(s.editFor(locId, 'french'), 'Coucou');

    // Delete the last change: the editor renders no fields at all.
    await tester.tap(find.byIcon(Icons.delete_outline));
    await tester.pumpAndSettle();
    expect(find.byType(TextField), findsNothing);
    expect(container.read(locEditsProvider).isDirty, false);
  });
}
