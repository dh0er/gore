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

  @override
  String get importFolder => 'Importuj folder…';

  @override
  String get importFile => 'Importuj plik…';

  @override
  String get applyTooltip => 'Zastosuj zestaw modów do gry';

  @override
  String get undeployAllAction => 'Wycofaj wszystko';

  @override
  String get undeployAllConfirm =>
      'Usunąć z gry wszystko, co wdrożył menedżer?';

  @override
  String get takeOverTitle => 'Wdrożenie Studio aktywne';

  @override
  String get takeOverBody =>
      'mod-studio wdrożyło mod do gry. Przejąć kontrolę, aby menedżer mógł zastosować ten zestaw?';

  @override
  String get takeOverAction => 'Przejmij';

  @override
  String get refreshAction => 'Odśwież';

  @override
  String conflictsTitle(int count) {
    return 'Konflikty ($count)';
  }

  @override
  String get conflictWinner => 'zwycięzca';

  @override
  String get componentsTitle => 'Składniki';

  @override
  String targetsMore(int count) {
    return '+$count więcej';
  }

  @override
  String get removeModAction => 'Usuń';

  @override
  String removeModConfirm(String name) {
    return 'Usunąć „$name” z biblioteki?';
  }

  @override
  String get errorSetGamePath => 'Najpierw ustaw ścieżkę gry w Ustawieniach.';

  @override
  String applyReportApplied(int count) {
    return 'Zastosowano $count modów.';
  }

  @override
  String get warningsTitle => 'Ostrzeżenia';

  @override
  String get modDisabledHint => 'Wyłączony';

  @override
  String get kindGoremod => 'goremod';

  @override
  String get kindTriplet => 'triplet';

  @override
  String get kindPak => 'pak';

  @override
  String get kindUe4ss => 'UE4SS';

  @override
  String get kindRawfile => 'surowy plik';

  @override
  String get kindMixed => 'mieszany';

  @override
  String get sevHard => 'poważny';

  @override
  String get sevSoft => 'łagodny';

  @override
  String get sevInfo => 'info';

  @override
  String aboutVersion(String version, String sha) {
    return 'Version $version ($sha)';
  }

  @override
  String get about => 'O programie';

  @override
  String get aboutSubtitle => 'Menedżer modów Gothic 1 Remake';

  @override
  String get aboutCopyright => '© 2026 współtwórcy goresave';

  @override
  String get aboutLicense => 'Udostępniane na licencji MIT.';

  @override
  String get appearanceTitle => 'Wygląd';

  @override
  String get theme => 'Motyw';

  @override
  String get themeLight => 'Jasny';

  @override
  String get themeDark => 'Ciemny';

  @override
  String get themeSystem => 'Systemowy';

  @override
  String get uiScale => 'Skala interfejsu';

  @override
  String get resetZoomTooltip => 'Resetuj powiększenie (Ctrl+0)';

  @override
  String get zoomTip =>
      'Wskazówka: Ctrl + / Ctrl - zmienia powiększenie w dowolnym miejscu aplikacji.';

  @override
  String get lightMode => 'Tryb jasny';

  @override
  String get darkMode => 'Tryb ciemny';

  @override
  String get minimize => 'Minimalizuj';

  @override
  String get restore => 'Przywróć';

  @override
  String get maximize => 'Maksymalizuj';

  @override
  String get close => 'Zamknij';
}
