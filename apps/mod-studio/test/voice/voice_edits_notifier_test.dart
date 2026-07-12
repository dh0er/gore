import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/voice/domain/voice_edits_notifier.dart';

const replaceObservation = VoiceArchiveObservation(
  archiveSize: 1000,
  archiveSha256:
      'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa',
  memberProof: VoiceMemberProof.present(uncompressedSize: 100, crc32: 7),
);

VoiceArchiveEdit replacement(
  String locId,
  String locale, {
  String? archivePath,
  String? oggPath,
}) => VoiceArchiveEdit(
  locId: locId,
  locale: locale,
  archive: 'german_new.zip',
  operation: VoicePatchOperation.replace,
  archivePath: archivePath ?? 'NPC/$locId.ogg',
  oggPath: oggPath ?? '$locId.ogg',
  observation: replaceObservation,
);

void main() {
  test('set/update/remove/clear use semantic line and locale identity', () {
    final container = ProviderContainer();
    addTearDown(container.dispose);
    final notifier = container.read(voiceEditsProvider.notifier);

    notifier.setEdit(replacement('LINE_ONE', 'de'));
    notifier.setEdit(replacement('LINE_TWO', 'de'));
    notifier.setEdit(replacement('line_one', 'de', oggPath: 'new.ogg'));

    var entries = container.read(voiceEditsProvider).entries;
    expect(entries.map((entry) => entry.locId), ['line_one', 'LINE_TWO']);
    expect(entries.first.oggPath, 'new.ogg');

    notifier.remove('LINE_ONE', 'de');
    entries = container.read(voiceEditsProvider).entries;
    expect(entries.map((entry) => entry.locId), ['LINE_TWO']);

    notifier.clearAll();
    expect(container.read(voiceEditsProvider).count, 0);
  });

  test('load rejects duplicate semantic slots without changing state', () {
    final notifier = VoiceEditsNotifier()..setEdit(replacement('KNOWN', 'de'));

    expect(
      () => notifier.loadAll([
        replacement('DUPLICATE', 'de'),
        replacement('duplicate', 'de', archivePath: 'Other/DUPLICATE.ogg'),
      ]),
      throwsFormatException,
    );
    expect(notifier.state.entries.single.locId, 'KNOWN');
  });

  test('case-folded deployment collisions are never later-wins', () {
    final notifier = VoiceEditsNotifier()
      ..setEdit(replacement('FIRST', 'de', archivePath: 'NPC/FIRST.ogg'));

    expect(
      () => notifier.setEdit(
        replacement('first', 'en', archivePath: 'npc/first.OGG'),
      ),
      throwsFormatException,
    );
    expect(notifier.state.entries.single.locId, 'FIRST');
  });

  test('published state cannot be mutated outside the notifier', () {
    final notifier = VoiceEditsNotifier()
      ..setEdit(replacement('IMMUTABLE', 'de'));

    expect(() => notifier.state.items.clear(), throwsUnsupportedError);
    expect(notifier.state.entries.single.locId, 'IMMUTABLE');
  });
}
