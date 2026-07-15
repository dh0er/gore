import 'package:flutter_test/flutter_test.dart';
import 'package:goresave/features/editor/domain/glossary_npc_catalog.dart';
import 'package:goresave/l10n/app_localizations.dart';

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();

  test('every bundled NPC segment has a label in every app locale', () async {
    final entries = await loadGlossaryNpcCatalog();
    final segmentsById = <String, NpcGlossaryCatalogSegment>{};
    for (final entry in entries) {
      for (final segment in entry.segments) {
        final previous = segmentsById[segment.id];
        expect(
          previous == null || previous.label == segment.label,
          isTrue,
          reason: 'Conflicting labels for segment id ${segment.id}',
        );
        segmentsById[segment.id] = segment;
      }
    }

    expect(segmentsById, hasLength(187));
    expect(AppLocalizations.supportedLocales, hasLength(12));

    const sentinel = '__missing_glossary_segment_translation__';
    for (final locale in AppLocalizations.supportedLocales) {
      final l10n = lookupAppLocalizations(locale);
      for (final segment in segmentsById.values) {
        final label = l10n.glossaryCatalogSegmentLabel(segment.id, sentinel);
        expect(
          label,
          isNot(sentinel),
          reason: '${locale.toLanguageTag()}: ${segment.id}',
        );
        expect(
          label.trim(),
          isNotEmpty,
          reason: '${locale.toLanguageTag()}: ${segment.id}',
        );
      }
    }
  });

  test('unknown future segment ids retain the readable fallback', () {
    const fallback = 'Future Segment';
    for (final locale in AppLocalizations.supportedLocales) {
      expect(
        lookupAppLocalizations(
          locale,
        ).glossaryCatalogSegmentLabel('FutureSegment', fallback),
        fallback,
      );
    }
  });
}
