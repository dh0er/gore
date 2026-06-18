import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/editor/domain/override_entry.dart';
import 'package:gore_mod/editor/domain/overrides_notifier.dart';

void main() {
  late OverridesNotifier notifier;

  setUp(() => notifier = OverridesNotifier());

  const apple500 = OverrideEntry(
    classId: 'ItFo_Apple', field: 'm_Value', oldValue: 4, newValue: 500,
  );
  const appleMass = OverrideEntry(
    classId: 'ItFo_Apple', field: 'm_Mass', oldValue: 0.1, newValue: 0.5,
  );
  const sword = OverrideEntry(
    classId: 'ItMw_1H_Sword_01', field: 'm_Value', oldValue: 50, newValue: 200,
  );

  test('starts empty', () {
    expect(notifier.state.count, 0);
  });

  test('setOverride adds an entry', () {
    notifier.setOverride(apple500);
    expect(notifier.state.count, 1);
    expect(notifier.state.overrides['ItFo_Apple.m_Value']?.newValue, 500);
  });

  test('setOverride updates existing entry', () {
    notifier.setOverride(apple500);
    notifier.setOverride(apple500.copyWith(newValue: 999));
    expect(notifier.state.count, 1);
    expect(notifier.state.overrides['ItFo_Apple.m_Value']?.newValue, 999);
  });

  test('removeOverride removes a specific entry', () {
    notifier.setOverride(apple500);
    notifier.setOverride(appleMass);
    notifier.removeOverride('ItFo_Apple.m_Value');
    expect(notifier.state.count, 1);
    expect(notifier.state.overrides.containsKey('ItFo_Apple.m_Value'), isFalse);
  });

  test('clearOverridesForClass removes all entries for a class', () {
    notifier.setOverride(apple500);
    notifier.setOverride(appleMass);
    notifier.setOverride(sword);
    notifier.clearOverridesForClass('ItFo_Apple');
    expect(notifier.state.count, 1);
    expect(notifier.state.overrides.containsKey('ItMw_1H_Sword_01.m_Value'), isTrue);
  });

  test('clearAll empties the map', () {
    notifier.setOverride(apple500);
    notifier.setOverride(sword);
    notifier.clearAll();
    expect(notifier.state.count, 0);
  });

  test('entries are sorted by classId then field', () {
    notifier.setOverride(sword);
    notifier.setOverride(appleMass);
    notifier.setOverride(apple500);
    final entries = notifier.state.entries;
    expect(entries[0].classId, 'ItFo_Apple');
    expect(entries[0].field,   'm_Mass');   // m_Mass < m_Value
    expect(entries[1].field,   'm_Value');
    expect(entries[2].classId, 'ItMw_1H_Sword_01');
  });

  test('OverrideEntry.toJson shape matches overrides.toml schema', () {
    final json = apple500.toJson();
    expect(json['class'], 'ItFo_Apple');
    expect(json['field'], 'm_Value');
    expect(json['value'], 500);
  });

  test('OverrideEntry.toFfiJson uses gore_core typed value keys', () {
    final intJson = const OverrideEntry(
      classId: 'C', field: 'f', oldValue: 0, newValue: 7,
    ).toFfiJson();
    expect(intJson['value_int'], 7);
    expect(intJson.containsKey('value'), isFalse);

    final floatJson = const OverrideEntry(
      classId: 'C', field: 'f', oldValue: 0.0, newValue: 1.5,
    ).toFfiJson();
    expect(floatJson['value_float'], 1.5);

    final boolJson = const OverrideEntry(
      classId: 'C', field: 'f', oldValue: false, newValue: true,
    ).toFfiJson();
    expect(boolJson['value_bool'], true);

    final strJson = const OverrideEntry(
      classId: 'C', field: 'f', oldValue: '', newValue: 'x',
    ).toFfiJson();
    expect(strJson['value_str'], 'x');
  });
}
