import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/catalog/domain/field_schema.dart';
import 'package:gore_mod/catalog/domain/item_entry.dart';
import 'package:gore_mod/editor/domain/override_entry.dart';
import 'package:gore_mod/editor/domain/overrides_notifier.dart';
import 'package:gore_mod/editor/ui/field_editor.dart';
import 'package:gore_mod/l10n/app_localizations.dart';
import 'package:gore_mod/loc/domain/loc_edits_notifier.dart';

/// Wraps [child] in a localized MaterialApp so AppLocalizations.of works.
Widget _localizedApp(Widget child) => MaterialApp(
      localizationsDelegates: AppLocalizations.localizationsDelegates,
      supportedLocales: AppLocalizations.supportedLocales,
      home: Scaffold(body: child),
    );

void main() {
  const apple = CatalogItem(
    id: 'ItFo_Apple',
    displayName: 'Apple',
    fields: [
      FieldSchema(name: 'm_Value',    type: FieldType.int_,   minValue: 0),
      FieldSchema(name: 'm_Weight',   type: FieldType.float_, minValue: 0),
      FieldSchema(name: 'm_MaxStack', type: FieldType.int_,   minValue: 1),
    ],
  );

  Widget buildEditor({
    Map<String, OverrideEntry> pending = const {},
    void Function(OverrideEntry)? onChanged,
  }) {
    return _localizedApp(
      FieldEditor(
        item: apple,
        pendingOverrides: pending,
        onOverrideChanged: onChanged ?? (_) {},
      ),
    );
  }

  /// FieldEditor wired to the real providers, mirroring the items_tab wiring:
  /// pendingOverrides watched from [overridesProvider], delete taps remove the
  /// entry from the notifier. Needs a ProviderScope (onlyEdited also watches
  /// [locEditsProvider] for the name section).
  Widget providerApp({bool onlyEdited = false}) {
    return ProviderScope(
      child: MaterialApp(
        localizationsDelegates: AppLocalizations.localizationsDelegates,
        supportedLocales: AppLocalizations.supportedLocales,
        home: Scaffold(
          body: Consumer(
            builder: (context, ref, _) {
              final state = ref.watch(overridesProvider);
              return FieldEditor(
                item: apple,
                onlyEdited: onlyEdited,
                pendingOverrides: {
                  for (final e
                      in state.entries.where((e) => e.classId == apple.id))
                    e.field: e,
                },
                onOverrideChanged: (e) =>
                    ref.read(overridesProvider.notifier).setOverride(e),
                onOverrideRemoved: (e) =>
                    ref.read(overridesProvider.notifier).removeOverride(e.key),
              );
            },
          ),
        ),
      ),
    );
  }

  ProviderContainer containerOf(WidgetTester tester) =>
      ProviderScope.containerOf(
        tester.element(find.byType(FieldEditor)),
        listen: false,
      );

  const valueOverride = OverrideEntry(
    classId: 'ItFo_Apple', field: 'm_Value', oldValue: 0, newValue: 500,
  );

  testWidgets('renders a row for each field', (tester) async {
    await tester.pumpWidget(buildEditor());
    // Three fields → three TextFields (int/int/float all render TextField)
    expect(find.byType(TextField), findsNWidgets(3));
    // The localized-name section is always present in normal mode.
    expect(find.text('Name (all languages)'), findsOneWidget);
  });

  testWidgets('valid integer input calls onOverrideChanged', (tester) async {
    OverrideEntry? received;
    await tester.pumpWidget(buildEditor(onChanged: (e) => received = e));
    final fields = find.byType(TextField);
    await tester.enterText(fields.first, '500');
    await tester.testTextInput.receiveAction(TextInputAction.done);
    await tester.pump();
    expect(received?.field, 'm_Value');
    expect(received?.newValue, 500);
  });

  testWidgets('invalid integer shows error and does not call onOverrideChanged', (tester) async {
    int callCount = 0;
    await tester.pumpWidget(buildEditor(onChanged: (_) => callCount++));
    final fields = find.byType(TextField);
    await tester.enterText(fields.first, 'abc');
    await tester.pump();
    expect(callCount, 0);
    expect(find.text('Must be a whole number'), findsOneWidget);
  });

  testWidgets('displays item display name as header', (tester) async {
    await tester.pumpWidget(buildEditor());
    expect(find.text('Apple'), findsOneWidget);
  });

  testWidgets('pending field shows a delete button instead of the pencil',
      (tester) async {
    final pending = {'m_Value': valueOverride};
    await tester.pumpWidget(buildEditor(pending: pending));
    expect(find.byIcon(Icons.delete_outline), findsOneWidget);
    expect(find.byTooltip('Remove change'), findsOneWidget);
    expect(find.byIcon(Icons.edit), findsNothing);
  });

  testWidgets('delete tap removes the override; field resyncs to catalog value',
      (tester) async {
    await tester.pumpWidget(providerApp());
    containerOf(tester).read(overridesProvider.notifier)
        .setOverride(valueOverride);
    await tester.pump();
    // Normal mode: all fields stay visible; the overridden one shows the
    // pending value and a delete button.
    expect(find.byType(TextField), findsNWidgets(3));
    final field = find.byType(TextField).first;
    expect(tester.widget<TextField>(field).controller!.text, '500');

    await tester.tap(find.byTooltip('Remove change'));
    await tester.pump();
    expect(containerOf(tester).read(overridesProvider).count, 0);
    // Field remains, back at the catalog (placeholder) value, no delete button.
    expect(find.byType(TextField), findsNWidgets(3));
    expect(tester.widget<TextField>(field).controller!.text, '0');
    expect(find.byIcon(Icons.delete_outline), findsNothing);
  });

  testWidgets('onlyEdited renders only fields with a pending override',
      (tester) async {
    await tester.pumpWidget(providerApp(onlyEdited: true));
    // No overrides yet → no field rows at all.
    expect(find.byType(TextField), findsNothing);
    expect(find.text('m_Value'), findsNothing);

    containerOf(tester).read(overridesProvider.notifier)
        .setOverride(valueOverride);
    await tester.pump();
    expect(find.byType(TextField), findsOneWidget);
    expect(find.text('m_Value'), findsOneWidget);
    expect(find.text('m_Weight'), findsNothing);
    expect(find.text('m_MaxStack'), findsNothing);
  });

  testWidgets('onlyEdited: deleting the last override leaves no field rows',
      (tester) async {
    await tester.pumpWidget(providerApp(onlyEdited: true));
    containerOf(tester).read(overridesProvider.notifier)
        .setOverride(valueOverride);
    await tester.pump();
    expect(find.byType(TextField), findsOneWidget);

    await tester.tap(find.byTooltip('Remove change'));
    await tester.pump();
    expect(containerOf(tester).read(overridesProvider).count, 0);
    expect(find.byType(TextField), findsNothing);
    expect(find.byIcon(Icons.delete_outline), findsNothing);
  });

  testWidgets('onlyEdited shows the name section only with a staged name edit',
      (tester) async {
    await tester.pumpWidget(providerApp(onlyEdited: true));
    expect(find.text('Name (all languages)'), findsNothing);

    // Stage a name edit for this item's loc id → section appears.
    containerOf(tester).read(locEditsProvider.notifier)
        .setEdit('itfo_apple', 'english', 'Golden Apple');
    await tester.pump();
    expect(find.text('Name (all languages)'), findsOneWidget);

    // Reverting the last name edit hides it again.
    containerOf(tester).read(locEditsProvider.notifier)
        .removeEdit('itfo_apple', 'english');
    await tester.pump();
    expect(find.text('Name (all languages)'), findsNothing);
  });

  testWidgets('entering 0 emits an override (not treated as a clear)', (tester) async {
    // Start from a pending non-zero so typing 0 is a real change.
    final pending = {
      'm_Value': const OverrideEntry(
        classId: 'ItFo_Apple', field: 'm_Value', oldValue: 0, newValue: 500,
      ),
    };
    OverrideEntry? changed;
    await tester.pumpWidget(buildEditor(pending: pending, onChanged: (e) => changed = e));
    // 0 is a valid value (m_Value minValue is 0); it must be exportable, not
    // swallowed as a "revert to default".
    await tester.enterText(find.byType(TextField).first, '0');
    await tester.testTextInput.receiveAction(TextInputAction.done);
    await tester.pump();
    expect(changed?.field, 'm_Value');
    expect(changed?.newValue, 0);
  });

  testWidgets('does not reformat in-progress numeric input on rebuild', (tester) async {
    // m_Weight is a float (second TextField). Typing "3" then letting the
    // parent echo the override back (3.0) must NOT rewrite the field to "3.0"
    // — that would drop a half-typed ".5" and re-select the text.
    await tester.pumpWidget(buildEditor());
    final weightField = find.byType(TextField).at(1);
    await tester.enterText(weightField, '3');
    await tester.pump();
    await tester.pumpWidget(buildEditor(pending: {
      'm_Weight': const OverrideEntry(
        classId: 'ItFo_Apple', field: 'm_Weight', oldValue: 0.0, newValue: 3.0,
      ),
    }));
    expect(tester.widget<TextField>(weightField).controller!.text, '3');
  });

  testWidgets('resyncs field text when a pending override is removed externally', (tester) async {
    final pending = {
      'm_Value': const OverrideEntry(
        classId: 'ItFo_Apple', field: 'm_Value', oldValue: 0, newValue: 500,
      ),
    };
    await tester.pumpWidget(buildEditor(pending: pending));
    final field = find.byType(TextField).first;
    expect(tester.widget<TextField>(field).controller!.text, '500');

    // Parent rebuilds with the override removed (OverridesPanel Clear all).
    await tester.pumpWidget(buildEditor(pending: const {}));
    expect(tester.widget<TextField>(field).controller!.text, '0');
  });

  testWidgets('shows the real default value and carries it as oldValue', (tester) async {
    const item = CatalogItem(
      id: 'ItFo_Apple',
      displayName: 'Apple',
      fields: [
        FieldSchema(name: 'm_Value', type: FieldType.int_, minValue: 0, defaultValue: 4),
      ],
    );
    OverrideEntry? changed;
    await tester.pumpWidget(_localizedApp(
      FieldEditor(
        item: item,
        pendingOverrides: const {},
        onOverrideChanged: (e) => changed = e,
      ),
    ));
    // Field starts at its real default (4), not the placeholder 0.
    expect(tester.widget<TextField>(find.byType(TextField).first).controller!.text, '4');
    await tester.enterText(find.byType(TextField).first, '7');
    await tester.testTextInput.receiveAction(TextInputAction.done);
    await tester.pump();
    expect(changed?.newValue, 7);
    expect(changed?.oldValue, 4); // diff reads 4 -> 7
  });

  testWidgets('rebuilds controllers when the field set changes for the same id', (tester) async {
    // A loaded dump can add fields to the already-selected item (same id). The
    // editor must rebuild controllers, not crash force-unwrapping a missing one.
    const before = CatalogItem(id: 'X', displayName: 'X', fields: [
      FieldSchema(name: 'm_A', type: FieldType.int_),
    ]);
    const after = CatalogItem(id: 'X', displayName: 'X', fields: [
      FieldSchema(name: 'm_A', type: FieldType.int_),
      FieldSchema(name: 'm_B', type: FieldType.int_),
    ]);
    Widget wrap(CatalogItem item) => _localizedApp(
          FieldEditor(
            item: item,
            pendingOverrides: const {},
            onOverrideChanged: (_) {},
          ),
        );
    await tester.pumpWidget(wrap(before));
    expect(find.byType(TextField), findsOneWidget);
    await tester.pumpWidget(wrap(after));
    expect(tester.takeException(), isNull);
    expect(find.byType(TextField), findsNWidgets(2));
  });

  testWidgets('enum override stored as backing int displays the member name', (tester) async {
    const enumItem = CatalogItem(
      id: 'ItFo_Apple',
      displayName: 'Apple',
      fields: [
        FieldSchema(
          name: 'm_Quality',
          type: FieldType.enum_,
          enumValues: ['Low', 'Medium', 'High'],
        ),
      ],
    );
    // Pending override holds the backing int (2 == 'High'), as parsedValue
    // produces — the dropdown must show 'High', not '2'.
    final pending = {
      'm_Quality': const OverrideEntry(
        classId: 'ItFo_Apple', field: 'm_Quality', oldValue: 0, newValue: 2,
      ),
    };
    await tester.pumpWidget(_localizedApp(
      FieldEditor(
        item: enumItem,
        pendingOverrides: pending,
        onOverrideChanged: (_) {},
      ),
    ));
    expect(find.text('High'), findsOneWidget);
  });

  testWidgets('non-contiguous enum: stored backing value shows the right member', (tester) async {
    const enumItem = CatalogItem(
      id: 'ItFo_Apple',
      displayName: 'Apple',
      fields: [
        FieldSchema(
          name: 'm_Quality',
          type: FieldType.enum_,
          enumValues: ['Low', 'Mid', 'High'],
          enumBackingValues: [0, 5, 9],
        ),
      ],
    );
    // newValue 5 is the backing value of 'Mid' (index 1) — must show 'Mid',
    // not the member at index 5.
    final pending = {
      'm_Quality': const OverrideEntry(
        classId: 'ItFo_Apple', field: 'm_Quality', oldValue: 0, newValue: 5,
      ),
    };
    await tester.pumpWidget(_localizedApp(
      FieldEditor(
        item: enumItem,
        pendingOverrides: pending,
        onOverrideChanged: (_) {},
      ),
    ));
    expect(find.text('Mid'), findsOneWidget);
  });
}
