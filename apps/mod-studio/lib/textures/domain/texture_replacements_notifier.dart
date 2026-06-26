import 'package:flutter_riverpod/legacy.dart';

/// One staged texture replacement: put [imagePath] (a PNG) in place of cooked [asset].
class TextureReplacement {
  const TextureReplacement({required this.asset, required this.imagePath});
  final String asset;
  final String imagePath;

  String get key => asset;

  Map<String, Object?> toJson() => {'asset': asset, 'image_path': imagePath};

  factory TextureReplacement.fromJson(Map<String, Object?> j) => TextureReplacement(
        asset: j['asset'] as String,
        imagePath: j['image_path'] as String,
      );

  TextureReplacement withImagePath(String path) =>
      TextureReplacement(asset: asset, imagePath: path);
}

class TextureReplacementsState {
  const TextureReplacementsState({this.items = const {}});
  final Map<String, TextureReplacement> items;
  int get count => items.length;
  List<TextureReplacement> get entries => items.values.toList()
    ..sort((a, b) => a.asset.compareTo(b.asset));
  TextureReplacementsState copyWith({Map<String, TextureReplacement>? items}) =>
      TextureReplacementsState(items: items ?? this.items);
}

class TextureReplacementsNotifier extends StateNotifier<TextureReplacementsState> {
  TextureReplacementsNotifier() : super(const TextureReplacementsState());
  void setReplacement(TextureReplacement r) {
    final items = Map<String, TextureReplacement>.from(state.items);
    items[r.key] = r;
    state = state.copyWith(items: items);
  }
  void remove(String key) {
    if (!state.items.containsKey(key)) return;
    final items = Map<String, TextureReplacement>.from(state.items)..remove(key);
    state = state.copyWith(items: items);
  }
  void clearAll() {
    if (state.items.isEmpty) return;
    state = const TextureReplacementsState();
  }
  void loadAll(List<TextureReplacement> list) {
    state = TextureReplacementsState(items: {for (final r in list) r.key: r});
  }
}

final textureReplacementsProvider =
    StateNotifierProvider<TextureReplacementsNotifier, TextureReplacementsState>(
        (ref) => TextureReplacementsNotifier());
