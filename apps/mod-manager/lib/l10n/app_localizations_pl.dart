// ignore: unused_import
import 'package:intl/intl.dart' as intl;
import 'app_localizations.dart';

// ignore_for_file: type=lint

/// The translations for Polish (`pl`).
class AppLocalizationsPl extends AppLocalizations {
  AppLocalizationsPl([String locale = 'pl']) : super(locale);

  @override
  String get appTitle => 'gore-manager';

  @override
  String get tabMods => 'Mody';

  @override
  String get tabSettings => 'Ustawienia';

  @override
  String get settingsGameExe => 'Plik wykonywalny gry';

  @override
  String get settingsGameExePick => 'Wybierz…';

  @override
  String get settingsLanguage => 'Język';

  @override
  String get statusInSync => 'Zsynchronizowano';

  @override
  String get statusChangesPending => 'Oczekujące zmiany';

  @override
  String get statusGameUpdated => 'Gra zaktualizowana';

  @override
  String get statusStudioDeploy => 'Wdrożenie Studio aktywne';

  @override
  String get statusNothingDeployed => 'Nic nie wdrożono';

  @override
  String get actionImport => 'Importuj';

  @override
  String get actionApply => 'Zastosuj';

  @override
  String get actionUndeployAll => 'Wycofaj wszystko';

  @override
  String get commonCancel => 'Anuluj';

  @override
  String get commonOk => 'OK';
}
