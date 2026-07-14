import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:goresave/features/app/domain/ui_settings.dart';
import 'package:goresave/features/editor/domain/knowledge_catalog.dart';
import 'package:goresave/features/editor/ui/add_knowledge_entry_dialog.dart';
import 'package:goresave/loc/loc_catalog_provider.dart';

import 'support/l10n_test_app.dart';
import 'support/ui_settings_test_store.dart';

void main() {
  final catalog = KnowledgeCatalog.fromJsonString(
    '[{"id":"Topic_Diego_209799","category":"topic",'
    '"loc_key":"INFO_DIEGO_OTHERCAMPS_15_00"},'
    '{"id":"ChoiceDiegoGamestart","category":"choice"},'
    '{"id":"Info_FMORGAreyouok","category":"info"}]',
  );

  testWidgets('lists entries, excludes existing, returns selection', (
    tester,
  ) async {
    String? picked;
    await tester.pumpWidget(
      wrapWithL10n(
        Scaffold(
          body: Builder(
            builder: (context) {
              return ElevatedButton(
                onPressed: () async {
                  picked = await showAddKnowledgeEntryDialog(
                    context,
                    catalog: catalog,
                    exclude: {'choicediegogamestart'},
                  );
                },
                child: const Text('open'),
              );
            },
          ),
        ),
      ),
    );
    await tester.tap(find.text('open'));
    await tester.pumpAndSettle();

    expect(find.text('Dialog topic'), findsOneWidget);
    expect(find.text('Topic_Diego_209799'), findsNothing);
    expect(find.text('ChoiceDiegoGamestart'), findsNothing);
    await tester.tap(find.text('Dialog topic'));
    await tester.pumpAndSettle();
    expect(picked, 'Topic_Diego_209799');
  });

  testWidgets('uses cache-derived caption key for generated numeric ids', (
    tester,
  ) async {
    await tester.pumpWidget(
      ProviderScope(
        overrides: [
          locCatalogProvider.overrideWith(
            (ref) async => {
              'info_diego_othercamps_15_00': {
                'english': 'Tell me about the other camps.',
              },
            },
          ),
        ],
        child: wrapWithL10n(
          Scaffold(
            body: Builder(
              builder: (context) => ElevatedButton(
                onPressed: () => showAddKnowledgeEntryDialog(
                  context,
                  catalog: catalog,
                  exclude: const {},
                ),
                child: const Text('open localized'),
              ),
            ),
          ),
        ),
      ),
    );
    await tester.tap(find.text('open localized'));
    await tester.pumpAndSettle();

    expect(find.text('Tell me about the other camps.'), findsOneWidget);
    expect(find.text('Topic_Diego_209799'), findsNothing);
  });

  testWidgets('shows the raw knowledge id only when enabled', (tester) async {
    await tester.pumpWidget(
      ProviderScope(
        overrides: [
          uiSettingsStoreProvider.overrideWithValue(
            TestUiSettingsStore(showObjectIds: true),
          ),
        ],
        child: wrapWithL10n(
          Scaffold(
            body: Builder(
              builder: (context) => ElevatedButton(
                onPressed: () => showAddKnowledgeEntryDialog(
                  context,
                  catalog: catalog,
                  exclude: const {},
                ),
                child: const Text('open ids'),
              ),
            ),
          ),
        ),
      ),
    );
    await tester.tap(find.text('open ids'));
    await tester.pumpAndSettle();

    expect(find.text('Dialog topic'), findsOneWidget);
    expect(find.text('Topic_Diego_209799'), findsOneWidget);
  });
}
