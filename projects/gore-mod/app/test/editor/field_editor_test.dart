import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/catalog/domain/field_schema.dart';
import 'package:gore_mod/catalog/domain/item_entry.dart';
import 'package:gore_mod/editor/domain/override_entry.dart';
import 'package:gore_mod/editor/ui/field_editor.dart';

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
    return MaterialApp(
      home: Scaffold(
        body: FieldEditor(
          item: apple,
          pendingOverrides: pending,
          onOverrideChanged: onChanged ?? (_) {},
        ),
      ),
    );
  }

  testWidgets('renders a row for each field', (tester) async {
    await tester.pumpWidget(buildEditor());
    // Three fields → three TextFields (int/int/float all render TextField)
    expect(find.byType(TextField), findsNWidgets(3));
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

  testWidgets('pending field shows edit icon in suffix', (tester) async {
    final pending = {
      'm_Value': const OverrideEntry(
        classId: 'ItFo_Apple', field: 'm_Value', oldValue: 0, newValue: 500,
      ),
    };
    await tester.pumpWidget(buildEditor(pending: pending));
    expect(find.byIcon(Icons.edit), findsOneWidget);
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
    await tester.pumpWidget(MaterialApp(
      home: Scaffold(
        body: FieldEditor(
          item: enumItem,
          pendingOverrides: pending,
          onOverrideChanged: (_) {},
        ),
      ),
    ));
    expect(find.text('High'), findsOneWidget);
  });
}
