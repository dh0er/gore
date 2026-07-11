import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/project/dialog_topics_notifier.dart';

DialogTopicDefinition topic(String id, {String? participant}) =>
    DialogTopicDefinition(
      id: id,
      participantName: participant ?? 'om_${id.toLowerCase()}_001',
      topicClass: '/Script/Angelscript.Choice$id',
      sentinelClass: '/Script/Angelscript.Choice${id}Vanilla',
    );

void main() {
  test('staging preserves insertion order across updates and ID edits', () {
    final container = ProviderContainer();
    addTearDown(container.dispose);
    final notifier = container.read(dialogTopicsProvider.notifier);

    notifier.setTopic(topic('Alpha'));
    notifier.setTopic(topic('Beta'));
    notifier.setTopic(topic('ALPHA', participant: 'om_changed_001'));

    var entries = container.read(dialogTopicsProvider).entries;
    expect(entries.map((entry) => entry.id), ['ALPHA', 'Beta']);
    expect(entries.first.participantName, 'om_changed_001');

    notifier.replaceTopic('alpha', topic('Gamma'));
    entries = container.read(dialogTopicsProvider).entries;
    expect(entries.map((entry) => entry.id), ['Gamma', 'Beta']);

    notifier.remove('GAMMA');
    expect(
      container.read(dialogTopicsProvider).entries.map((entry) => entry.id),
      ['Beta'],
    );
  });

  test('loadAll and clearAll retain authored list order without inference', () {
    final container = ProviderContainer();
    addTearDown(container.dispose);
    final notifier = container.read(dialogTopicsProvider.notifier);
    final second = topic('Second', participant: 'EXACT_Participant_42');

    notifier.loadAll([topic('First'), second, topic('Third')]);

    final state = container.read(dialogTopicsProvider);
    expect(state.count, 3);
    expect(state.entries.map((entry) => entry.id), [
      'First',
      'Second',
      'Third',
    ]);
    expect(state.entries[1].participantName, 'EXACT_Participant_42');

    notifier.clearAll();
    expect(container.read(dialogTopicsProvider).count, 0);
  });

  test('rename and project load reject duplicate IDs without data loss', () {
    final container = ProviderContainer();
    addTearDown(container.dispose);
    final notifier = container.read(dialogTopicsProvider.notifier);
    notifier.setTopic(topic('First'));
    notifier.setTopic(topic('Second'));

    expect(
      () => notifier.replaceTopic('First', topic('SECOND')),
      throwsArgumentError,
    );
    expect(
      container.read(dialogTopicsProvider).entries.map((entry) => entry.id),
      ['First', 'Second'],
    );

    expect(
      () => notifier.loadAll([topic('Known'), topic('KNOWN')]),
      throwsFormatException,
    );
    expect(
      container.read(dialogTopicsProvider).entries.map((entry) => entry.id),
      ['First', 'Second'],
    );
  });
}
