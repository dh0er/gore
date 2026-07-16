import 'package:goresave/loc/game_lang.dart';
import 'package:goresave/loc/loc_catalog_provider.dart';

import 'glossary_npc_catalog.dart';
import 'glossary_segment_text_catalog.dart';

/// A conservative, evidence-backed link from a technical story id to an NPC
/// glossary segment. It is intentionally not called a translation: the game
/// does not ship a one-to-one localized label catalog for story properties.
class StoryGlossaryLink {
  const StoryGlossaryLink({
    required this.npcCatalogId,
    required this.uniqueName,
    required this.segmentId,
    required this.segmentLabel,
    required this.segmentClass,
    required this.textIds,
  });

  final String npcCatalogId;
  final String uniqueName;
  final String segmentId;
  final String segmentLabel;
  final String segmentClass;
  final List<String> textIds;

  String npcName(Map<String, Map<String, String>> catalog, GameLang lang) {
    return localizedGameName(catalog, lang, uniqueName) ??
        localizedGameName(catalog, lang, npcCatalogId) ??
        humanizeStoryId(npcCatalogId.split('_').last);
  }

  List<String> localizedParagraphs(
    Map<String, Map<String, String>> catalog,
    GameLang lang,
  ) {
    final seen = <String>{};
    final paragraphs = <String>[];
    for (final textId in textIds) {
      final text = resolveGameText(catalog, textId, lang)?.trim();
      if (text != null && text.isNotEmpty && seen.add(text)) {
        paragraphs.add(text);
      }
    }
    return paragraphs;
  }
}

/// Match only an exact technical suffix (`..._Stone_OreArmor`). Loose token or
/// NPC-name matching would manufacture relationships for hundreds of unrelated
/// variables and repeat the heuristic-category problem this view avoids.
StoryGlossaryLink? findStoryGlossaryLink(
  String storyId,
  List<NpcGlossaryCatalogEntry> entries,
  GlossarySegmentTextCatalog textCatalog,
) {
  final suffix = '_${storyId.toLowerCase()}';
  StoryGlossaryLink? match;
  for (final entry in entries) {
    for (final segment in entry.segments) {
      if (!segment.segmentClass.toLowerCase().endsWith(suffix)) continue;
      // More than one exact match would be ambiguous, so expose neither.
      if (match != null) return null;
      match = StoryGlossaryLink(
        npcCatalogId: entry.id,
        uniqueName: entry.uniqueName,
        segmentId: segment.id,
        segmentLabel: segment.label,
        segmentClass: segment.segmentClass,
        textIds: textCatalog[segment.segmentClass.toLowerCase()] ?? const [],
      );
    }
  }
  return match;
}

String humanizeStoryId(String value) {
  final withSpaces = value
      .replaceAll('_', ' ')
      .replaceAllMapped(RegExp(r'(?<=[a-z0-9])(?=[A-Z])'), (_) => ' ')
      .replaceAll(RegExp(r'\s+'), ' ')
      .trim();
  if (withSpaces.isEmpty) return value;
  return '${withSpaces[0].toUpperCase()}${withSpaces.substring(1)}';
}
