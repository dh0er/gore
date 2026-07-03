// ignore: unused_import
import 'package:intl/intl.dart' as intl;
import 'app_localizations.dart';

// ignore_for_file: type=lint

/// The translations for German (`de`).
class AppLocalizationsDe extends AppLocalizations {
  AppLocalizationsDe([String locale = 'de']) : super(locale);

  @override
  String get appTitle => 'gore-manager';

  @override
  String get tabMods => 'Mods';

  @override
  String get tabSettings => 'Einstellungen';

  @override
  String get settingsGameExe => 'Spiel-Programmdatei';

  @override
  String get settingsGameExePick => 'Auswählen…';

  @override
  String get settingsLanguage => 'Sprache';

  @override
  String get statusInSync => 'Synchron';

  @override
  String get statusChangesPending => 'Änderungen ausstehend';

  @override
  String get statusGameUpdated => 'Spiel aktualisiert';

  @override
  String get statusStudioDeploy => 'Studio-Bereitstellung aktiv';

  @override
  String get statusNothingDeployed => 'Nichts bereitgestellt';

  @override
  String get actionImport => 'Importieren';

  @override
  String get actionApply => 'Anwenden';

  @override
  String get actionUndeployAll => 'Alle Bereitstellungen aufheben';

  @override
  String get commonCancel => 'Abbrechen';

  @override
  String get commonOk => 'OK';
}
