import 'dart:convert';

import 'package:flutter/services.dart';

typedef GlossarySegmentTextCatalog = Map<String, List<String>>;

const glossarySegmentTextCatalogAsset =
    'assets/glossary_segment_text_catalog.json';

/// Loads the generated mapping from a glossary segment class to the ordered
/// LocText ids used by its BuildSegment implementation.
///
/// The asset deliberately contains no translated prose. Text stays sourced
/// from the user's extracted game localization catalog and therefore follows
/// the selected game language (including its normal English fallback).
Future<GlossarySegmentTextCatalog> loadGlossarySegmentTextCatalog({
  AssetBundle? bundle,
}) async {
  final text = await (bundle ?? rootBundle).loadString(
    glossarySegmentTextCatalogAsset,
  );
  final decoded = jsonDecode(text);
  if (decoded is! Map) {
    throw const FormatException(
      'Glossary segment text catalog root must be an object',
    );
  }

  final catalog = <String, List<String>>{};
  for (final entry in decoded.entries) {
    final segmentClass = entry.key;
    final rawTextIds = entry.value;
    if (segmentClass is! String || rawTextIds is! List) continue;
    final textIds = rawTextIds
        .whereType<String>()
        .where((id) => id.trim().isNotEmpty)
        .toList(growable: false);
    if (segmentClass.isNotEmpty && textIds.isNotEmpty) {
      catalog[segmentClass.toLowerCase()] = textIds;
    }
  }
  return Map.unmodifiable(catalog);
}
