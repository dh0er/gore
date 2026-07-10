import 'package:flutter_riverpod/legacy.dart';

/// One staged audio replacement: put [wavPath]'s audio in place of [sample] in [bank].
class AudioReplacement {
  const AudioReplacement({required this.bank, required this.sample, required this.wavPath});
  final String bank;
  final String sample;
  final String wavPath;

  String get key => '$bank/$sample';

  Map<String, Object?> toJson() => {'bank': bank, 'sample': sample, 'wav_path': wavPath};

  factory AudioReplacement.fromJson(Map<String, Object?> j) => AudioReplacement(
        bank: j['bank'] as String,
        sample: j['sample'] as String,
        wavPath: j['wav_path'] as String,
      );

  AudioReplacement withWavPath(String path) =>
      AudioReplacement(bank: bank, sample: sample, wavPath: path);
}

class AudioReplacementsState {
  const AudioReplacementsState({this.items = const {}});

  /// keyed by `bank/sample`
  final Map<String, AudioReplacement> items;

  int get count => items.length;
  List<AudioReplacement> get entries => items.values.toList()
    ..sort((a, b) {
      final c = a.bank.compareTo(b.bank);
      return c != 0 ? c : a.sample.compareTo(b.sample);
    });

  AudioReplacementsState copyWith({Map<String, AudioReplacement>? items}) =>
      AudioReplacementsState(items: items ?? this.items);
}

class AudioReplacementsNotifier extends StateNotifier<AudioReplacementsState> {
  AudioReplacementsNotifier() : super(const AudioReplacementsState());

  void setReplacement(AudioReplacement r) {
    final items = Map<String, AudioReplacement>.from(state.items);
    items[r.key] = r;
    state = state.copyWith(items: items);
  }

  void remove(String key) {
    if (!state.items.containsKey(key)) return;
    final items = Map<String, AudioReplacement>.from(state.items)..remove(key);
    state = state.copyWith(items: items);
  }

  void clearAll() {
    if (state.items.isEmpty) return;
    state = const AudioReplacementsState();
  }

  void loadAll(List<AudioReplacement> list) {
    state = AudioReplacementsState(items: {for (final r in list) r.key: r});
  }
}

final audioReplacementsProvider =
    StateNotifierProvider<AudioReplacementsNotifier, AudioReplacementsState>(
        (ref) => AudioReplacementsNotifier());
