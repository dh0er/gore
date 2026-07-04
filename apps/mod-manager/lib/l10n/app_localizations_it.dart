// ignore: unused_import
import 'package:intl/intl.dart' as intl;
import 'app_localizations.dart';

// ignore_for_file: type=lint

/// The translations for Italian (`it`).
class AppLocalizationsIt extends AppLocalizations {
  AppLocalizationsIt([String locale = 'it']) : super(locale);

  @override
  String get appTitle => 'gore-manager';

  @override
  String get tabMods => 'Mod';

  @override
  String get tabSettings => 'Impostazioni';

  @override
  String get settingsGameExe => 'Eseguibile del gioco';

  @override
  String get settingsGameExePick => 'Scegli…';

  @override
  String get settingsLanguage => 'Lingua';

  @override
  String get statusInSync => 'Sincronizzato';

  @override
  String get statusChangesPending => 'Modifiche in sospeso';

  @override
  String get statusGameUpdated => 'Gioco aggiornato';

  @override
  String get statusStudioDeploy => 'Distribuzione Studio attiva';

  @override
  String get statusNothingDeployed => 'Niente distribuito';

  @override
  String get actionImport => 'Importa';

  @override
  String get actionApply => 'Applica';

  @override
  String get actionUndeployAll => 'Ritira tutto';

  @override
  String get commonCancel => 'Annulla';

  @override
  String get commonOk => 'OK';

  @override
  String get importFolder => 'Importa cartella…';

  @override
  String get importFile => 'Importa file…';

  @override
  String get applyTooltip => 'Applica la configurazione al gioco';

  @override
  String get undeployAllAction => 'Ritira tutto';

  @override
  String get undeployAllConfirm =>
      'Rimuovere dal gioco tutto ciò che il gestore ha distribuito?';

  @override
  String get takeOverTitle => 'Distribuzione Studio attiva';

  @override
  String get takeOverBody =>
      'mod-studio ha distribuito un mod nel gioco. Subentrare così che il gestore possa applicare questa configurazione?';

  @override
  String get takeOverAction => 'Subentra';

  @override
  String get refreshAction => 'Aggiorna';

  @override
  String conflictsTitle(int count) {
    return 'Conflitti ($count)';
  }

  @override
  String get conflictWinner => 'vincitore';

  @override
  String get componentsTitle => 'Componenti';

  @override
  String targetsMore(int count) {
    return '+$count altri';
  }

  @override
  String get removeModAction => 'Rimuovi';

  @override
  String removeModConfirm(String name) {
    return 'Rimuovere «$name» dalla libreria?';
  }

  @override
  String get errorSetGamePath =>
      'Imposta prima il percorso del gioco nelle Impostazioni.';

  @override
  String applyReportApplied(int count) {
    return '$count mod applicati.';
  }

  @override
  String get warningsTitle => 'Avvisi';

  @override
  String get modDisabledHint => 'Disattivato';

  @override
  String get kindGoremod => 'goremod';

  @override
  String get kindTriplet => 'triplet';

  @override
  String get kindPak => 'pak';

  @override
  String get kindUe4ss => 'UE4SS';

  @override
  String get kindRawfile => 'file grezzo';

  @override
  String get kindMixed => 'misto';

  @override
  String get sevHard => 'grave';

  @override
  String get sevSoft => 'lieve';

  @override
  String get sevInfo => 'info';

  @override
  String aboutVersion(String version, String sha) {
    return 'Version $version ($sha)';
  }

  @override
  String get about => 'Informazioni';

  @override
  String get aboutSubtitle => 'Gestore di mod per Gothic 1 Remake';

  @override
  String get aboutCopyright => '© 2026 collaboratori di goresave';

  @override
  String get aboutLicense => 'Concesso in licenza secondo la licenza MIT.';

  @override
  String get appearanceTitle => 'Aspetto';

  @override
  String get theme => 'Tema';

  @override
  String get themeLight => 'Chiaro';

  @override
  String get themeDark => 'Scuro';

  @override
  String get themeSystem => 'Sistema';

  @override
  String get uiScale => 'Scala dell\'interfaccia';

  @override
  String get resetZoomTooltip => 'Reimposta lo zoom (Ctrl+0)';

  @override
  String get zoomTip =>
      'Suggerimento: Ctrl + / Ctrl - cambia lo zoom ovunque nell\'app.';

  @override
  String get lightMode => 'Modalità chiara';

  @override
  String get darkMode => 'Modalità scura';

  @override
  String get minimize => 'Riduci a icona';

  @override
  String get restore => 'Ripristina';

  @override
  String get maximize => 'Ingrandisci';

  @override
  String get close => 'Chiudi';
}
