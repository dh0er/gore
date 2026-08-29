import 'package:goresave/features/app/domain/ui_settings.dart';

/// In-memory UI-settings store for widget tests that need an explicit advanced
/// option without touching the user's real settings file.
class TestUiSettingsStore implements UiSettingsStore {
  TestUiSettingsStore({
    bool showObjectIds = false,
    UiFontFamily uiFontFamily = UiFontFamily.notoSerif,
    String appLocale = 'en',
    double uiScale = 1.0,
  }) : _settings = UiSettings(
         appLocale: appLocale,
         showObjectIds: showObjectIds,
         uiFontFamily: uiFontFamily,
         uiScale: uiScale,
       );

  UiSettings _settings;

  @override
  UiSettings read() => _settings;

  @override
  void write(UiSettings settings) => _settings = settings;
}
