import 'dart:async';
import 'dart:io';

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:goresave/features/editor/domain/glossary_images.dart';
import 'package:goresave/features/editor/ui/game_icon.dart';
import 'package:goresave/features/editor/ui/glossary_portrait.dart';
import 'package:goresave/providers/data_providers.dart';
import 'package:goresave/utils/shared_config.dart';
import 'package:path/path.dart' as p;

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

/// A config that counts how often the game path is read off disk. Every read
/// opens and parses `config.json`.
class _CountingConfig extends SharedConfig {
  _CountingConfig(super.file);

  int reads = 0;

  @override
  String? gamePath() {
    reads++;
    return super.gamePath();
  }
}

Future<int> _configReadsFor(WidgetTester tester, int portraits) async {
  final config = _CountingConfig(
    File(
      p.join(
        Directory.systemTemp.createTempSync('gore_portrait_cfg').path,
        'config.json',
      ),
    ),
  );
  await tester.pumpWidget(
    ProviderScope(
      overrides: [
        glossaryImageCatalogProvider.overrideWith((ref) async => _catalog),
        sharedConfigProvider.overrideWithValue(config),
      ],
      child: MaterialApp(
        home: Scaffold(
          body: SingleChildScrollView(
            child: Column(
              children: List.generate(
                portraits,
                (_) => const GlossaryPortrait(
                  documentClass: 'Document_Glossary_OC_STT_DIEGO',
                ),
              ),
            ),
          ),
        ),
      ),
    ),
  );
  await tester.pumpAndSettle();
  return config.reads;
}

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

  testWidgets('a catalog still loading is not mistaken for one that drew '
      'nothing', (tester) async {
    // Everything is undrawn while the catalog is on its way. Answering with a
    // section glyph there flashed one over every row before the portraits
    // arrived.
    final ready = Completer<GlossaryImageCatalog>();
    await tester.pumpWidget(
      ProviderScope(
        overrides: [
          glossaryImageCatalogProvider.overrideWith((ref) => ready.future),
        ],
        child: const MaterialApp(
          home: Scaffold(
            body: Center(
              child: GlossaryPortrait(
                documentClass: 'Document_Glossary_OC_STT_DIEGO',
                standInOnPaper: true,
                undrawnGameIcon: 'T_Icon_Tutorials',
                fallbackIcon: Icons.school_outlined,
              ),
            ),
          ),
        ),
      ),
    );
    await tester.pump();
    expect(_glyph(tester), 'T_CharacterImageSmall_Missing');

    ready.complete(_catalog);
    await tester.pumpAndSettle();
    // Diego IS drawn, so nothing changes to the section glyph on arrival — no
    // game path is configured here, so the stand-in stays what it was.
    expect(_glyph(tester), 'T_CharacterImageSmall_Missing');
  });

  testWidgets('the game root is worked out once, not once per portrait', (
    tester,
  ) async {
    // Both halves of the answer touch the disk: reading the setting parses
    // `config.json`, normalizing it probes every ancestor for the story
    // folder. Doing that inside each portrait's build put hundreds of blocking
    // checks on the UI thread for a page of characters, on every rebuild.
    final one = await _configReadsFor(tester, 1);
    final many = await _configReadsFor(tester, 20);
    expect(many, one);
  });
}
