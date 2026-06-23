import 'package:flutter_riverpod/legacy.dart';
import 'override_entry.dart';

class OverridesState {
  const OverridesState({this.overrides = const {}});

  /// Key: `OverrideEntry.key` (`classId.field`).
  final Map<String, OverrideEntry> overrides;

  int get count => overrides.length;
  List<OverrideEntry> get entries => overrides.values.toList()
    ..sort((a, b) {
      final c = a.classId.compareTo(b.classId);
      return c != 0 ? c : a.field.compareTo(b.field);
    });

  OverridesState copyWith({Map<String, OverrideEntry>? overrides}) =>
      OverridesState(overrides: overrides ?? this.overrides);
}

class OverridesNotifier extends StateNotifier<OverridesState> {
  OverridesNotifier() : super(const OverridesState());

  void setOverride(OverrideEntry entry) {
    final updated = Map<String, OverrideEntry>.from(state.overrides);
    updated[entry.key] = entry;
    state = state.copyWith(overrides: updated);
  }

  void removeOverride(String key) {
    if (!state.overrides.containsKey(key)) return;
    final updated = Map<String, OverrideEntry>.from(state.overrides);
    updated.remove(key);
    state = state.copyWith(overrides: updated);
  }

  void clearOverridesForClass(String classId) {
    final updated = Map<String, OverrideEntry>.from(state.overrides)
      ..removeWhere((k, _) => k.startsWith('$classId.'));
    state = state.copyWith(overrides: updated);
  }

  void clearAll() {
    if (state.overrides.isEmpty) return;
    state = const OverridesState();
  }
}

final overridesProvider =
    StateNotifierProvider<OverridesNotifier, OverridesState>(
  (ref) => OverridesNotifier(),
);
