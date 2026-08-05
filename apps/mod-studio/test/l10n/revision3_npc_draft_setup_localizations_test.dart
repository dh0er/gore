import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/l10n/app_localizations_de.dart';
import 'package:gore_mod/l10n/app_localizations_en.dart';

void main() {
  test('English NPC Draft setup copy stays explicit and count-aware', () {
    final l10n = AppLocalizationsEn();

    expect(
      l10n.managedStoryWorkbenchNpcDraftSetupTitle,
      'Write this Character',
    );
    expect(
      l10n.managedStoryWorkbenchNpcDraftSetupWriteFirstGreeting,
      'Write first greeting',
    );
    expect(
      l10n.managedStoryWorkbenchNpcDraftSetupReviewDialogVoice,
      'Review greetings in Dialog & Voice',
    );
    expect(
      l10n.managedStoryWorkbenchNpcDraftSetupGreetingLinkCount(0),
      'No authored greeting links',
    );
    expect(
      l10n.managedStoryWorkbenchNpcDraftSetupGreetingLinkCount(1),
      '1 authored greeting link',
    );
    expect(
      l10n.managedStoryWorkbenchNpcDraftSetupGreetingLinkCount(2),
      '2 authored greeting links',
    );
    expect(
      l10n.managedStoryWorkbenchNpcDraftSetupBoundary,
      allOf(
        contains('current authored project content only'),
        contains('not a playable'),
        contains('publication history'),
      ),
    );
  });

  test('German NPC Draft setup copy preserves the same honest boundary', () {
    final l10n = AppLocalizationsDe();

    expect(
      l10n.managedStoryWorkbenchNpcDraftSetupTitle,
      'Diese Figur schreiben',
    );
    expect(
      l10n.managedStoryWorkbenchNpcDraftSetupWriteFirstGreeting,
      'Erste Begrüßung schreiben',
    );
    expect(
      l10n.managedStoryWorkbenchNpcDraftSetupReviewDialogVoice,
      'Begrüßungen in Dialog & Sprachausgabe prüfen',
    );
    expect(
      l10n.managedStoryWorkbenchNpcDraftSetupGreetingLinkCount(0),
      'Keine entworfenen Begrüßungsverknüpfungen',
    );
    expect(
      l10n.managedStoryWorkbenchNpcDraftSetupGreetingLinkCount(1),
      '1 entworfene Begrüßungsverknüpfung',
    );
    expect(
      l10n.managedStoryWorkbenchNpcDraftSetupGreetingLinkCount(2),
      '2 entworfene Begrüßungsverknüpfungen',
    );
    expect(
      l10n.managedStoryWorkbenchNpcDraftSetupBoundary,
      allOf(
        contains('nur aktuelle entworfene Projektinhalte'),
        contains('kein spielbares'),
        contains('Veröffentlichungshistorie'),
      ),
    );
  });
}
