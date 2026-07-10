import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/audio/domain/audio_replacements_notifier.dart';

void main() {
  test('set/remove/clear replacements', () {
    final c = ProviderContainer();
    addTearDown(c.dispose);
    final n = c.read(audioReplacementsProvider.notifier);

    n.setReplacement(const AudioReplacement(
        bank: 'SFX.bank', sample: 'SFX_UI_X', wavPath: r'C:\a.wav'));
    expect(c.read(audioReplacementsProvider).count, 1);

    // same key overwrites
    n.setReplacement(const AudioReplacement(
        bank: 'SFX.bank', sample: 'SFX_UI_X', wavPath: r'C:\b.wav'));
    expect(c.read(audioReplacementsProvider).count, 1);
    expect(c.read(audioReplacementsProvider).items['SFX.bank/SFX_UI_X']!.wavPath,
        r'C:\b.wav');

    n.remove('SFX.bank/SFX_UI_X');
    expect(c.read(audioReplacementsProvider).count, 0);
  });

  test('loadAll replaces and toJson/fromJson round-trips', () {
    final c = ProviderContainer();
    addTearDown(c.dispose);
    final n = c.read(audioReplacementsProvider.notifier);
    const r = AudioReplacement(bank: 'Music.bank', sample: 'm1', wavPath: r'C:\m.wav');
    n.loadAll([r]);
    expect(c.read(audioReplacementsProvider).count, 1);
    final back = AudioReplacement.fromJson(r.toJson());
    expect(back.bank, 'Music.bank');
    expect(back.sample, 'm1');
    expect(back.wavPath, r'C:\m.wav');
  });
}
