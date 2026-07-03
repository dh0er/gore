// ignore: unused_import
import 'package:intl/intl.dart' as intl;
import 'app_localizations.dart';

// ignore_for_file: type=lint

/// The translations for French (`fr`).
class AppLocalizationsFr extends AppLocalizations {
  AppLocalizationsFr([String locale = 'fr']) : super(locale);

  @override
  String get appTitle => 'gore-manager';

  @override
  String get tabMods => 'Mods';

  @override
  String get tabSettings => 'Paramètres';

  @override
  String get settingsGameExe => 'Exécutable du jeu';

  @override
  String get settingsGameExePick => 'Choisir…';

  @override
  String get settingsLanguage => 'Langue';

  @override
  String get statusInSync => 'Synchronisé';

  @override
  String get statusChangesPending => 'Modifications en attente';

  @override
  String get statusGameUpdated => 'Jeu mis à jour';

  @override
  String get statusStudioDeploy => 'Déploiement Studio actif';

  @override
  String get statusNothingDeployed => 'Rien de déployé';

  @override
  String get actionImport => 'Importer';

  @override
  String get actionApply => 'Appliquer';

  @override
  String get actionUndeployAll => 'Tout retirer';

  @override
  String get commonCancel => 'Annuler';

  @override
  String get commonOk => 'OK';
}
