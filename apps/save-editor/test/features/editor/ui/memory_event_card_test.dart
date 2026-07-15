import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:goresave/features/editor/domain/memory_event_presentation.dart';
import 'package:goresave/features/editor/domain/progression_models.dart';
import 'package:goresave/features/editor/ui/memory_event_card.dart';
import 'package:goresave/l10n/app_localizations.dart';
import 'package:goresave/l10n/app_localizations_en.dart';

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
}
