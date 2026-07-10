import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:goresave/features/editor/domain/knowledge_catalog.dart';
import 'package:goresave/features/editor/ui/add_knowledge_entry_dialog.dart';

import 'support/l10n_test_app.dart';

void main() {
  final catalog = KnowledgeCatalog.fromJsonString(
    '[{"id":"Topic_Diego_209799","category":"topic"},'
    '{"id":"ChoiceDiegoGamestart","category":"choice"},'
    '{"id":"Info_FMORGAreyouok","category":"info"}]',
  );

  testWidgets('lists entries, excludes existing, returns selection',
      (tester) async {
    String? picked;
    await tester.pumpWidget(wrapWithL10n(
      Scaffold(
        body: Builder(builder: (context) {
          return ElevatedButton(
            onPressed: () async {
              picked = await showAddKnowledgeEntryDialog(
                context, catalog: catalog, exclude: {'choicediegogamestart'});
            },
            child: const Text('open'),
          );
        }),
      ),
    ));
    await tester.tap(find.text('open'));
    await tester.pumpAndSettle();

    expect(find.text('Topic_Diego_209799'), findsOneWidget);
    expect(find.text('ChoiceDiegoGamestart'), findsNothing);
    await tester.tap(find.text('Topic_Diego_209799'));
    await tester.pumpAndSettle();
    expect(picked, 'Topic_Diego_209799');
  });
}
