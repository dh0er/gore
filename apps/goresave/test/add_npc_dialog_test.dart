import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:goresave/features/editor/domain/npc_catalog.dart';
import 'package:goresave/features/editor/ui/add_npc_dialog.dart';

void main() {
  final catalog = NpcCatalog.fromJsonString(
    '[{"id":"OC_STT_Diego","class":"c1","category":"human"},'
    '{"id":"OC_GRD_Orry_254","class":"c2","category":"human"},'
    '{"id":"Creature_Biter","class":"c3","category":"creature"}]',
  );

  testWidgets('lists catalog NPCs, excludes existing, returns selection',
      (tester) async {
    String? picked;
    await tester.pumpWidget(MaterialApp(
      home: Scaffold(
        body: Builder(builder: (context) {
          return ElevatedButton(
            onPressed: () async {
              picked = await showAddNpcDialog(
                context, catalog: catalog, exclude: {'oc_grd_orry_254'});
            },
            child: const Text('open'),
          );
        }),
      ),
    ));
    await tester.tap(find.text('open'));
    await tester.pumpAndSettle();

    expect(find.text('OC_STT_Diego'), findsOneWidget);
    expect(find.text('OC_GRD_Orry_254'), findsNothing); // excluded
    await tester.tap(find.text('OC_STT_Diego'));
    await tester.pumpAndSettle();
    expect(picked, 'OC_STT_Diego');
  });
}
