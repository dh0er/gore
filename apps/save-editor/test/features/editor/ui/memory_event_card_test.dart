import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:goresave/features/app/domain/ui_settings.dart';
import 'package:goresave/features/editor/domain/memory_event_presentation.dart';
import 'package:goresave/features/editor/domain/progression_models.dart';
import 'package:goresave/features/editor/ui/memory_event_card.dart';
import 'package:goresave/l10n/app_localizations.dart';
import 'package:goresave/l10n/app_localizations_en.dart';
import 'package:goresave/ui/design/app_theme.dart';

import '../../../support/l10n_test_app.dart';

void main() {
  testWidgets('tapping the event title expands the card', (tester) async {
    const title = 'Quest started';
    const event = MemoryEvent(index: 7, tags: ['Memory.Quest.Started']);
    const presentation = MemoryEventPresentation(
      kind: MemoryEventKind.questStarted,
      category: MemoryEventCategory.quest,
      categoryLabel: 'Quest',
      title: title,
      facts: [
        MemoryEventFact(
          kind: MemoryEventFactKind.time,
          label: 'Time',
          value: '12:00',
        ),
      ],
      tags: ['Memory.Quest.Started'],
    );

    await tester.pumpWidget(
      MaterialApp(
        locale: const Locale('en'),
        localizationsDelegates: testLocalizationsDelegates,
        supportedLocales: AppLocalizations.supportedLocales,
        home: const Scaffold(
          body: SizedBox(
            width: 800,
            child: MemoryEventCard(
              event: event,
              presentation: presentation,
              editable: false,
              showObjectIds: false,
              pendingRemoval: false,
            ),
          ),
        ),
      ),
    );

    final details = find.text(AppLocalizationsEn().memoryEventDetails);
    expect(find.widgetWithText(SelectableText, title), findsNothing);
    expect(details, findsNothing);

    await tester.tap(find.text(title));
    await tester.pumpAndSettle();

    expect(details, findsOneWidget);
  });

  testWidgets('what the card reveals starts at the left edge', (tester) async {
    // An ExpansionTile centres its children. Every block here is read left to
    // right, so each one floated in the middle of its own row instead.
    const event = MemoryEvent(
      index: 7,
      tags: ['Memory.Guild.Joined'],
      position: MemoryEventPosition(x: 43794.79, y: -118013.47, z: -8883),
    );
    const presentation = MemoryEventPresentation(
      kind: MemoryEventKind.guildJoined,
      category: MemoryEventCategory.guild,
      categoryLabel: 'Guild',
      title: 'Joined guild: Swamp Camp',
      facts: [
        MemoryEventFact(
          kind: MemoryEventFactKind.time,
          label: 'Time',
          value: 'Day 0, 12:00:00',
        ),
      ],
      tags: ['Memory.Guild.Joined'],
    );

    await tester.pumpWidget(
      MaterialApp(
        locale: const Locale('en'),
        localizationsDelegates: testLocalizationsDelegates,
        supportedLocales: AppLocalizations.supportedLocales,
        home: const Scaffold(
          body: SizedBox(
            width: 800,
            child: MemoryEventCard(
              event: event,
              presentation: presentation,
              editable: false,
              showObjectIds: false,
              pendingRemoval: false,
            ),
          ),
        ),
      ),
    );
    await tester.tap(find.text('Joined guild: Swamp Camp'));
    await tester.pumpAndSettle();

    // The section headings mark where the content column begins; the facts and
    // the coordinates have to line up with them, not sit halfway across.
    // Measured against the CARD's own edge, not against a heading — a heading
    // drifts to the middle with everything else and would hide the very thing
    // this checks. The card is 800 wide, so centred content lands past 300.
    final cardLeft = tester.getTopLeft(find.byType(MemoryEventCard)).dx;
    for (final entry in <String, Finder>{
      'the heading': find.text(AppLocalizationsEn().memoryEventDetails),
      'the fact': find.textContaining('Day 0, 12:00:00').last,
      'the coordinates': find.textContaining('X 43794'),
      'the tag': find.text('Memory.Guild.Joined').last,
    }.entries) {
      expect(
        tester.getTopLeft(entry.value).dx,
        lessThan(cardLeft + 60),
        reason: '${entry.key} floats away from the left edge',
      );
    }
  });

  testWidgets(
    'a repeated catalog name in the segment stays on the interface face',
    (tester) async {
      const zh = Locale.fromSubtags(languageCode: 'zh', scriptCode: 'Hans');
      const title = 'Entry discovered: Diego — Diego Notes';
      const event = MemoryEvent(
        index: 31,
        tags: ['Memory.Document.SegmentUnlocked'],
      );
      const presentation = MemoryEventPresentation(
        kind: MemoryEventKind.documentSegmentUnlocked,
        category: MemoryEventCategory.document,
        categoryLabel: 'Document',
        title: title,
        subject: 'Diego — Diego Notes',
        catalogTitleLead: 'Entry discovered: ',
        catalogTitleRun: 'Diego',
        catalogTitleTail: ' — Diego Notes',
        facts: [],
        tags: ['Memory.Document.SegmentUnlocked'],
      );

      await tester.pumpWidget(
        MaterialApp(
          theme: buildGoresaveTheme(
            uiFontFamily: UiFontFamily.notoSerif,
            locale: const Locale('ja'),
            gameTextLocale: zh,
          ),
          locale: const Locale('ja'),
          localizationsDelegates: testLocalizationsDelegates,
          supportedLocales: AppLocalizations.supportedLocales,
          home: Scaffold(
            body: GameTextScript(
              locale: zh,
              child: const SizedBox(
                width: 800,
                child: MemoryEventCard(
                  event: event,
                  presentation: presentation,
                  editable: false,
                  showObjectIds: false,
                  pendingRemoval: false,
                ),
              ),
            ),
          ),
        ),
      );

      final rich = tester.widget<RichText>(
        find.byWidgetPredicate(
          (widget) => widget is RichText && widget.text.toPlainText() == title,
        ),
      );
      final runs = <TextSpan>[];
      void walk(InlineSpan span) {
        if (span is! TextSpan) return;
        if (span.text != null && span.text!.isNotEmpty) runs.add(span);
        span.children?.forEach(walk);
      }

      walk(rich.text);
      expect(runs.map((span) => span.text), [
        'Entry discovered: ',
        'Diego',
        ' — Diego Notes',
      ]);
      expect(runs[0].style?.fontFamily, notoSerifJpFontFamily);
      expect(runs[1].style?.fontFamily, notoSerifScFontFamily);
      expect(runs[2].style?.fontFamily, notoSerifJpFontFamily);
    },
  );
}
