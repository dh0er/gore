import 'package:goresave/features/app/domain/ui_settings.dart';

/// In-memory UI-settings store for widget tests that need an explicit advanced
/// option without touching the user's real settings file.
class TestUiSettingsStore implements UiSettingsStore {
  TestUiSettingsStore({bool showObjectIds = false})
    : _settings = UiSettings(appLocale: 'en', showObjectIds: showObjectIds);

  UiSettings _settings;

  @override
  UiSettings read() => _settings;

  @override
  void write(UiSettings settings) => _settings = settings;
}
