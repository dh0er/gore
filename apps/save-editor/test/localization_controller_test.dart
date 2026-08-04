import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:goresave/features/app/domain/ui_settings.dart';
import 'package:goresave/features/editor/domain/core_service.dart';
import 'package:goresave/features/localization/domain/localization_controller.dart';
import 'package:goresave/l10n/app_localizations.dart';
import 'package:goresave/l10n/app_localizations_de.dart';
import 'package:goresave/l10n/app_localizations_en.dart';
import 'package:goresave/providers/data_providers.dart';

void main() {
  test('direct controller construction defaults errors to English', () async {
    final controller = LocalizationController(_ErrorCoreService());

    final present = await controller.status();

    expect(present, isNull);
    expect(
      controller.state.message,
      AppLocalizationsEn().localizationStatusFailed('boom'),
    );
  });

  test('controller reads injected localization at message time', () async {
    AppLocalizations strings = AppLocalizationsEn();
    final controller = LocalizationController(
      _ErrorCoreService(),
      localizations: () => strings,
    );

    strings = AppLocalizationsDe();
    final result = await controller.extract(lcacheHint: 'missing.lcache');

    expect(
      result.message,
      AppLocalizationsDe().localizationExtractionFailed('boom'),
    );
    expect(controller.state.message, result.message);
  });

  test(
    'provider controller survives locale changes and uses new locale',
    () async {
      final container = ProviderContainer(
        overrides: [
          coreServiceProvider.overrideWithValue(_ErrorCoreService()),
          uiSettingsStoreProvider.overrideWithValue(
            const NoopUiSettingsStore(),
          ),
        ],
      );
      addTearDown(container.dispose);
      final controller = container.read(
        localizationControllerProvider.notifier,
      );

      container.read(localeProvider.notifier).setLocale('de');

      expect(
        container.read(localizationControllerProvider.notifier),
        same(controller),
      );
      await controller.status();
      expect(
        controller.state.message,
        AppLocalizationsDe().localizationStatusFailed('boom'),
      );
    },
  );
}

class _ErrorCoreService implements GoresaveCoreService {
  @override
  String get description => 'error core';

  @override
  bool get isAvailable => true;

  @override
  Future<Map<String, Object?>> execute(
    String command, {
    Map<String, Object?> payload = const {},
  }) async {
    return {
      'ok': false,
      'error': {'code': 'BROKEN', 'message': 'boom'},
    };
  }
}
