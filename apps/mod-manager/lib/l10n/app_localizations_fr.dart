// ignore: unused_import
import 'package:intl/intl.dart' as intl;
import 'app_localizations.dart';

// ignore_for_file: type=lint

/// The translations for French (`fr`).
class AppLocalizationsFr extends AppLocalizations {
  AppLocalizationsFr([String locale = 'fr']) : super(locale);

  @override
  String get recoveryAction => 'Récupérer';

  @override
  String get recoveryRequiredConfirm =>
      'Récupérer le déploiement interrompu et supprimer les fichiers partiellement déployés ?';

  @override
  String get statusRecoveryRequired => 'Récupération requise';

  @override
  String get appTitle => 'GORE Mod Manager';

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

  @override
  String get importFolder => 'Importer un dossier…';

  @override
  String get importFile => 'Importer un fichier…';

  @override
  String get applyTooltip => 'Appliquer la configuration au jeu';

  @override
  String get undeployAllAction => 'Tout retirer';

  @override
  String get undeployAllConfirm =>
      'Retirer du jeu tout ce que le gestionnaire a déployé ?';

  @override
  String get takeOverTitle => 'Déploiement Studio actif';

  @override
  String get takeOverBody =>
      'mod-studio a déployé un mod dans le jeu. Prendre le relais pour que le gestionnaire applique cette configuration ?';

  @override
  String get takeOverAction => 'Prendre le relais';

  @override
  String get refreshAction => 'Actualiser';

  @override
  String conflictsTitle(int count) {
    return 'Conflits ($count)';
  }

  @override
  String get conflictWinner => 'gagnant';

  @override
  String get noConflicts => 'Aucun conflit.';

  @override
  String get conflictsUnverified =>
      'Les conflits ne sont pas vérifiés tant que l’état de la bibliothèque n’est pas actualisé.';

  @override
  String get componentsTitle => 'Composants';

  @override
  String targetsMore(int count) {
    return '+$count de plus';
  }

  @override
  String get removeModDeploymentHint =>
      'Le retrait de la bibliothèque ne modifie pas immédiatement un déploiement existant. Si le mod est déjà déployé, sélectionnez ensuite Appliquer pour mettre à jour l\'installation du jeu.';

  @override
  String removeModSuccess(String name) {
    return '« $name » a été retiré de la bibliothèque.';
  }

  @override
  String removeModFailed(String name, String error) {
    return 'Impossible de retirer « $name » : $error';
  }

  @override
  String removeModPartialFailure(String name, String error) {
    return '« $name » a été retiré, mais le traitement suivant a signalé une erreur. L’état de la bibliothèque a été rechargé : $error';
  }

  @override
  String removeModOutcomeUnknown(String name, String error) {
    return 'Impossible de vérifier si « $name » a été retiré : $error — Actualisez pour vérifier l’état de la bibliothèque.';
  }

  @override
  String get libraryStateUnknown =>
      'L’état de la bibliothèque n’a pas pu être vérifié. Sélectionnez Actualiser avant de modifier ou d’appliquer des mods.';

  @override
  String get removeModAction => 'Retirer';

  @override
  String removeModConfirm(String name) {
    return 'Retirer « $name » de la bibliothèque ?';
  }

  @override
  String get errorSetGamePath =>
      'Définissez d’abord le chemin du jeu dans les Paramètres.';

  @override
  String applyReportApplied(int count) {
    return '$count mods appliqués.';
  }

  @override
  String get warningsTitle => 'Avertissements';

  @override
  String get modDisabledHint => 'Désactivé';

  @override
  String get kindGoremod => 'goremod';

  @override
  String get kindTriplet => 'triplet';

  @override
  String get kindPak => 'pak';

  @override
  String get kindUe4ss => 'UE4SS';

  @override
  String get kindRawfile => 'fichier brut';

  @override
  String get kindMixed => 'mixte';

  @override
  String get sevHard => 'fort';

  @override
  String get sevSoft => 'faible';

  @override
  String get sevInfo => 'info';

  @override
  String aboutVersion(String version, String sha) {
    return 'Version $version ($sha)';
  }

  @override
  String get about => 'À propos';

  @override
  String get aboutCopyright => '© 2026 contributeurs de GORE';

  @override
  String get aboutLicense => 'Sous licence MIT.';

  @override
  String get appearanceTitle => 'Apparence';

  @override
  String get theme => 'Thème';

  @override
  String get themeLight => 'Clair';

  @override
  String get themeDark => 'Sombre';

  @override
  String get themeSystem => 'Système';

  @override
  String get uiScale => 'Échelle de l\'interface';

  @override
  String get resetZoomTooltip => 'Réinitialiser le zoom (Ctrl+0)';

  @override
  String get zoomTip =>
      'Astuce : Ctrl + / Ctrl - modifie le zoom partout dans l\'application.';

  @override
  String get lightMode => 'Mode clair';

  @override
  String get darkMode => 'Mode sombre';

  @override
  String get minimize => 'Réduire';

  @override
  String get restore => 'Restaurer';

  @override
  String get maximize => 'Agrandir';

  @override
  String get close => 'Fermer';
}
