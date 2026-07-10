import 'dart:convert';
import 'dart:io';

import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/app/domain/ui_settings.dart';
import 'package:gore_mod/catalog/domain/catalog_provider.dart';
import 'package:path/path.dart' as p;

class _FakeStore implements UiSettingsStore {
  _FakeStore(this._settings);
  UiSettings _settings;
  @override
  UiSettings read() => _settings;
  @override
  void write(UiSettings settings) => _settings = settings;
}

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();

  group('UiSettings.dumpPath', () {
    test('round-trips through json', () {
      const s = UiSettings(dumpPath: r'C:\dumps\game_data.json');
      final back = UiSettings.fromJson(s.toJson());
      expect(back.dumpPath, r'C:\dumps\game_data.json');
    });

    test('omitted when null', () {
      expect(const UiSettings().toJson().containsKey('dumpPath'), isFalse);
    });
  });

  group('DumpPathNotifier', () {
    test('set then clear, persisting to the store', () {
      final store = _FakeStore(const UiSettings());
      final c = ProviderContainer(
        overrides: [uiSettingsStoreProvider.overrideWithValue(store)],
      );
      addTearDown(c.dispose);

      c.read(dumpPathProvider.notifier).set('x.json');
      expect(c.read(dumpPathProvider), 'x.json');
      expect(store.read().dumpPath, 'x.json');

      c.read(dumpPathProvider.notifier).clear();
      expect(c.read(dumpPathProvider), isNull);
      expect(store.read().dumpPath, isNull);
    });
  });

  test('catalogProvider reads fields/defaults from a loaded dump', () async {
    final dir = Directory.systemTemp.createTempSync('gore_mod_dump_');
    addTearDown(() => dir.deleteSync(recursive: true));
    final dump = File(p.join(dir.path, 'game_data.json'));
    // ItAm_Arrow is a real catalog id; give it a custom default via the dump.
    dump.writeAsStringSync(jsonEncode({
      'classes': {
        'ItAm_Arrow': {
          'fields': [
            {'name': 'm_Value', 'type': 'int', 'default': 99},
          ],
        },
      },
    }));

    final store = _FakeStore(UiSettings(dumpPath: dump.path));
    final c = ProviderContainer(
      overrides: [uiSettingsStoreProvider.overrideWithValue(store)],
    );
    addTearDown(c.dispose);

    final items = await c.read(catalogProvider.future);
    final arrow = items.firstWhere((i) => i.id == 'ItAm_Arrow');
    final mValue = arrow.fields.firstWhere((f) => f.name == 'm_Value');
    expect(mValue.defaultValue, 99);

    // A catalog item NOT in this sparse dump keeps its BUNDLED fields: the dump
    // overlays only the classes it actually carries, so an incomplete dump can
    // never strip an item of its (bundled) schema and make it uneditable.
    final absent = items.firstWhere((i) => i.id != 'ItAm_Arrow');
    expect(absent.fields, isNotEmpty);
  });
}
