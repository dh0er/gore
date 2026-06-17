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
    void Function(String)? onCleared,
  }) {
    return MaterialApp(
      home: Scaffold(
        body: FieldEditor(
          item: apple,
          pendingOverrides: pending,
          onOverrideChanged: onChanged ?? (_) {},
          onOverrideCleared: onCleared ?? (_) {},
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
}
