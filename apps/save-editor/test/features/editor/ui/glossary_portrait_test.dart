import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:goresave/features/editor/domain/glossary_images.dart';
import 'package:goresave/features/editor/ui/game_icon.dart';
import 'package:goresave/features/editor/ui/glossary_portrait.dart';
import 'package:goresave/providers/data_providers.dart';

/// A catalog that draws Diego and nothing else — the tutorials have no artwork
/// in the shipped one either.
final _catalog = GlossaryImageCatalog(const {
  'Document_Glossary_OC_STT_DIEGO': GlossaryImageRef(
    kind: 'Characters',
    name: 'Diego',
  ),
}, const {});

Widget _portrait({String? documentClass}) => ProviderScope(
  overrides: [
    glossaryImageCatalogProvider.overrideWith((ref) async => _catalog),
  ],
  child: MaterialApp(
    home: Scaffold(
      body: Center(
        child: GlossaryPortrait(
          documentClass: documentClass,
          standInOnPaper: true,
          undrawnGameIcon: 'T_Icon_Tutorials',
          fallbackIcon: Icons.school_outlined,
        ),
      ),
    ),
  ),
);

String? _glyph(WidgetTester tester) =>
    tester.widget<GameIcon>(find.byType(GameIcon)).name;

void main() {
  testWidgets('an unlocked entry the game draws nothing of keeps its section '
      'glyph', (tester) async {
    // The shipped catalog covers Characters, Creatures and Locations only, so
    // a tutorial resolves to no artwork. Showing the "no portrait" silhouette
    // there claimed the entry was still locked.
    await tester.pumpWidget(
      _portrait(documentClass: 'Document_Glossary_Tutorial_Fighting'),
    );
    await tester.pumpAndSettle();
    expect(_glyph(tester), 'T_Icon_Tutorials');
  });

  testWidgets('a locked entry keeps the silhouette', (tester) async {
    // The panel passes no document class while the entry is locked.
    await tester.pumpWidget(_portrait());
    await tester.pumpAndSettle();
    expect(_glyph(tester), 'T_CharacterImageSmall_Missing');
  });
}
