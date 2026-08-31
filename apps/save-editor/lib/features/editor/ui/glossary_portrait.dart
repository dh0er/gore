import 'dart:io';

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:goresave/features/editor/domain/character_category_catalog.dart';
import 'package:goresave/features/editor/domain/game_icons.dart';
import 'package:goresave/features/editor/domain/glossary_images.dart';
import 'package:goresave/features/editor/domain/item_icon_catalog.dart';
import 'package:goresave/features/editor/ui/game_icon.dart';
import 'package:goresave/providers/data_providers.dart';

/// The footprint every character mark in a list takes, so names line up whether
/// the row shows a portrait, a stand-in glyph or the death mark.
const glossaryPortraitWidth = 40.0;
const glossaryPortraitHeight = 34.0;

/// The footprint of the wide cut, as a detail view shows it. The artwork is
/// 500x264; this is that shape at a size a header can carry.
const glossaryBannerWidth = 120.0;
const glossaryBannerHeight = 64.0;

/// The paper the game draws its glossary sketches on. The artwork is near-black
/// line work whose shading lives in the alpha, so recolouring it to the theme
/// flattens the drawing — it needs a light ground of its own instead, on both
/// themes. Only a real portrait gets it: a stand-in glyph is a normal editor
/// icon and sits on the row like every other one.
const _paper = Color(0xFFEDE6D8);

/// Ink for a stand-in glyph drawn on that paper.
const _paperInk = Color(0xFFB4A88F);

/// The pencil portrait the game shows for a glossary entry, read straight from
/// the user's installation.
///
/// Falls back to a stand-in glyph — the game's character mark for a person it
/// draws no portrait of (the hero, every generic worker, bandit and guard), its
/// creature mark for a monster, its death mark for a killed one.
class GlossaryPortrait extends StatelessWidget {
  const GlossaryPortrait({
    super.key,
    this.documentClass,
    this.npcUniqueName,
    this.size = GlossaryImageSize.thumbnail,
    this.width = glossaryPortraitWidth,
    this.height = glossaryPortraitHeight,
    this.fallbackGameIcon,
    this.undrawnGameIcon,
    this.fallbackIcon = Icons.person_outline,
    this.color,
    this.standInOnPaper = false,
  });

  /// The entry's document class, e.g. `/Script/Angelscript.Document_Glossary_X`.
  /// Null means "no artwork to look up" and goes straight to the stand-in.
  final String? documentClass;

  /// A character's own id instead — either the bare unique name
  /// (`BC_BAN_Arlin_852`) or a save GlobalId that starts with it. Resolved to a
  /// glossary document; a character the glossary does not cover falls back to
  /// the mark for its kind.
  final String? npcUniqueName;
  final GlossaryImageSize size;
  final double width;
  final double height;

  /// Glyph to stand in with, overriding the mark chosen for the character's
  /// kind. A creature or location entry wants its own section glyph here.
  final String? fallbackGameIcon;

  /// Glyph for an entry the reader HAS unlocked but the game draws no picture
  /// of — the tutorials, above all. Without it those wore the silhouette that
  /// means "still locked".
  final String? undrawnGameIcon;
  final IconData fallbackIcon;

  /// Ink for the stand-in glyph. Defaults to the theme's own icon colour.
  final Color? color;

  /// Whether the stand-in also sits on the sketch paper. The glossary wants it:
  /// a locked entry there shows the game's own silhouette on the same sheet an
  /// unlocked one shows its portrait on. A character LIST does not — there the
  /// stand-in is an ordinary row icon.
  final bool standInOnPaper;

  @override
  Widget build(BuildContext context) {
    // Panels are pumped without a ProviderScope in widget tests, and the
    // artwork is an enhancement — fall through to the stand-in rather than make
    // a scope a requirement of every list that shows a character.
    final scoped =
        context.findAncestorWidgetOfExactType<UncontrolledProviderScope>() !=
        null;
    if (!scoped) return _frame(context, null, null);
    return Consumer(builder: (context, ref, _) => _resolve(context, ref));
  }

  Widget _resolve(BuildContext context, WidgetRef ref) {
    final artwork = ref.watch(glossaryImageCatalogProvider).value;
    // Which mark stands in depends on what the character IS. Resolved even when
    // a picture was found: the image may still fail to load, and its fallback
    // must not turn every monster into a person.
    var kind = npcUniqueName == null
        ? null
        : ref
              .watch(characterCategoryCatalogProvider)
              .value
              ?.categoryFor(npcUniqueName);
    var image = documentClass == null
        ? null
        : artwork?.artworkForDocument(documentClass!);
    if (image == null && documentClass == null && npcUniqueName != null) {
      final unique = npcUniqueName!.split('-').first.trim();
      final human = kind == CharacterCategory.human;
      if (unique.isNotEmpty && artwork != null) {
        // The glossary's own filing first: the document the character catalog
        // names for this person, then a document named after the character
        // itself — creatures have no dialogue and no roles, so that is the only
        // place they appear.
        final document =
            ref.watch(glossaryDocumentByNpcProvider).value?[unique
                .toLowerCase()] ??
            'Document_Glossary_$unique';
        image = artwork.artworkForDocument(document);
        // Then the artwork files themselves. Roughly thirty of them belong to
        // no document — Orik, Balam, Blade, SkeletonScout — but are drawn for
        // characters the save really spawns.
        image ??= artwork.artworkFor(unique, charactersOnly: human);
        // Last, the species: most monsters in a save are a variant of one the
        // glossary draws once, `Scavenger_Adult` of `Scavenger`. Never for
        // somebody the catalog calls human — Xardas' demon would take the demon
        // sketch, and a man named Wolf a wolf's.
        if (image == null && !human) {
          final species = artwork.creatureDocumentFor(unique);
          image = species == null ? null : artwork.artworkForDocument(species);
        }
      }
    }
    // The glossary's own filing is the better answer about what something is
    // than a catalog whose kind prefix the save dropped: a creature's sketch
    // means a creature, whatever the id looked like.
    if (kind != CharacterCategory.human &&
        image?.kind == GlossaryImageCatalog.creatureKind) {
      kind = CharacterCategory.creature;
    }
    return _frame(
      context,
      artwork?.pathForArtwork(
        image: image,
        size: size,
        // What the core resolved when it prepared the item icons — it has
        // already normalized the configured path and found a Steam install if
        // none was configured. Falls back to normalizing the setting here.
        gamePath:
            ref.watch(resolvedGameRootProvider) ??
            normalizeGameRoot(ref.watch(sharedConfigProvider).gamePath()),
      ),
      kind,
      // An entry was asked for and the catalog draws none of it: that is not
      // the same as a locked one.
      undrawn: documentClass != null && image == null,
    );
  }

  Widget _frame(
    BuildContext context,
    String? path,
    CharacterCategory? kind, {
    bool undrawn = false,
  }) {
    return SizedBox(
      width: width,
      height: height,
      child: Center(
        child: path == null && !standInOnPaper
            ? _standIn(context, kind, height, undrawn)
            : ClipRRect(
                borderRadius: BorderRadius.circular(3),
                child: ColoredBox(
                  color: _paper,
                  child: path == null
                      ? SizedBox(
                          width: width,
                          height: height,
                          child: Center(
                            child: _standIn(context, kind, height, undrawn),
                          ),
                        )
                      : Image.file(
                          File(path),
                          width: width,
                          height: height,
                          fit: BoxFit.cover,
                          filterQuality: FilterQuality.medium,
                          gaplessPlayback: true,
                          cacheWidth:
                              (width * MediaQuery.devicePixelRatioOf(context))
                                  .ceil(),
                          excludeFromSemantics: true,
                          errorBuilder: (context, _, _) =>
                              _standIn(context, kind, height, undrawn),
                        ),
                ),
              ),
      ),
    );
  }

  Widget _standIn(
    BuildContext context,
    CharacterCategory? kind,
    double height,
    bool undrawn,
  ) {
    // On the sheet, the stand-in is the game's own "no portrait" silhouette,
    // inked for paper — but only where a picture is what is missing. An
    // unlocked entry the game draws none of gets its section's glyph instead.
    final name =
        fallbackGameIcon ??
        (undrawn ? undrawnGameIcon : null) ??
        (standInOnPaper
            ? (size == GlossaryImageSize.banner
                  ? 'T_CharacterImageMedium_Missing'
                  : 'T_CharacterImageSmall_Missing')
            : (kind == CharacterCategory.creature
                  ? gameIconCreature
                  : gameIconCharacter));
    return GameIcon(
      name: name,
      fallbackIcon: fallbackIcon,
      size: height * 0.82,
      color: color ?? (standInOnPaper ? _paperInk : null),
    );
  }
}
