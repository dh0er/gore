import 'package:flutter_riverpod/legacy.dart';

/// Declarative localized-text edits, shared by the Dialoge tab and the Items name editor.
/// State is the full desired edit set: `{ locId: { setName: text } }`. An edit equal to the
/// catalog's current value should be dropped by the caller (so reverting clears it).
class LocEditsState {
  const LocEditsState({this.edits = const {}});

  /// `{ locId: { setName: text } }`
  final Map<String, Map<String, String>> edits;

  int get entryCount => edits.values.fold(0, (a, m) => a + m.length);
  bool get isDirty => edits.isNotEmpty;

  /// Current staged text for `(locId, set)`, or null.
  String? editFor(String locId, String set) => edits[locId]?[set];

  LocEditsState copyWith({Map<String, Map<String, String>>? edits}) =>
      LocEditsState(edits: edits ?? this.edits);
}

class LocEditsNotifier extends StateNotifier<LocEditsState> {
  LocEditsNotifier() : super(const LocEditsState());

  Map<String, Map<String, String>> _clone() => {
        for (final e in state.edits.entries) e.key: Map<String, String>.from(e.value),
      };

  void setEdit(String locId, String set, String text) {
    final edits = _clone();
    (edits[locId] ??= {})[set] = text;
    state = state.copyWith(edits: edits);
  }

  void removeEdit(String locId, String set) {
    if (!state.edits.containsKey(locId)) return;
    final edits = _clone();
    final inner = edits[locId]!..remove(set);
    if (inner.isEmpty) edits.remove(locId);
    state = state.copyWith(edits: edits);
  }

  void clearForId(String locId) {
    if (!state.edits.containsKey(locId)) return;
    final edits = _clone()..remove(locId);
    state = state.copyWith(edits: edits);
  }

  void clearAll() {
    if (state.edits.isEmpty) return;
    state = const LocEditsState();
  }

  /// Replace the entire edit set (used when loading a project).
  void loadAll(Map<String, Map<String, String>> edits) {
    state = LocEditsState(edits: {
      for (final e in edits.entries) e.key: Map<String, String>.from(e.value),
    });
  }
}

final locEditsProvider =
    StateNotifierProvider<LocEditsNotifier, LocEditsState>((ref) => LocEditsNotifier());
