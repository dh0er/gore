// ignore: unused_import
import 'package:intl/intl.dart' as intl;
import 'app_localizations.dart';

// ignore_for_file: type=lint

/// The translations for French (`fr`).
class AppLocalizationsFr extends AppLocalizations {
  AppLocalizationsFr([String locale = 'fr']) : super(locale);

  @override
  String get coreBlockedTitle => 'Mod Manager indisponible';

  @override
  String get coreDllMissingMessage =>
      'Le fichier gore_ffi.dll requis est introuvable.';

  @override
  String get coreDllLoadFailedMessage =>
      'Impossible de charger la bibliothèque native GORE Core.';

  @override
  String get coreVerificationFailedMessage =>
      'Impossible de vérifier la bibliothèque native GORE Core.';

  @override
  String get coreManagerTooOldMessage =>
      'Cette version de GORE Core est plus récente que le Mod Manager. Mettez le Mod Manager à jour.';

  @override
  String get coreNativeTooOldMessage =>
      'Cette version de GORE Core est plus ancienne que le Mod Manager. Mettez à jour ou réparez l’installation complète du Mod Manager.';

  @override
  String get coreCommandsMissingMessage =>
      'La bibliothèque GORE Core ne fournit pas toutes les commandes requises par ce Mod Manager.';

  @override
  String get coreBlockedRepairHint =>
      'Mettez à jour ou réparez le package complet du Mod Manager, puis redémarrez l’application.';

  @override
  String get coreTechnicalDetails => 'Détails techniques';

  @override
  String get coreCopyTechnicalDetails => 'Copier les détails techniques';

  @override
  String get coreTechnicalDetailsCopied => 'Détails techniques copiés';

  @override
  String get coreTechnicalDetailsCopyFailed =>
      'Impossible de copier les détails techniques. Réessayez.';

  @override
  String get preflightAttention =>
      'La configuration nécessite votre attention.';

  @override
  String get preflightUnavailable =>
      'Le diagnostic de configuration est indisponible.';

  @override
  String get preflightRetry => 'Vérifier à nouveau';

  @override
  String get preflightReviewStatus => 'Vérifier l’état';

  @override
  String get preflightReviewRecovery => 'Aide';

  @override
  String get installRecoveryTitle => 'Récupération de l’installation';

  @override
  String get installRecoveryBody =>
      'GORE a trouvé des données de récupération provenant d’une installation ou d’une compilation de scripts interrompue. Une réparation automatique serait risquée, car l’ancien processus et l’état d’origine des fichiers ne peuvent pas être confirmés avec certitude.';

  @override
  String get installRecoverySteps =>
      'Fermez Gothic, Mod Studio et les autres tâches GORE. Suivez le fichier README.txt dans le dossier de récupération indiqué ci-dessous. Si aucun dossier n’est indiqué, laissez les données de récupération listées inchangées et demandez de l’aide au lieu de supprimer quoi que ce soit. Ne supprimez jamais un verrou tant qu’une tâche est en cours. Vérifiez ensuite à nouveau.';

  @override
  String get installRecoveryEvidence => 'Données de récupération détectées';

  @override
  String get statusUnknown => 'Inconnu';

  @override
  String statusDetailsTitle(String status) {
    return 'Déploiement : $status';
  }

  @override
  String statusDetailsOpen(String status) {
    return 'Afficher les détails du déploiement : $status';
  }

  @override
  String get statusDetailsNoRoot =>
      'Choisissez une installation du jeu dans les paramètres pour consulter son état de déploiement.';

  @override
  String get statusDetailsNoDeployment =>
      'Aucun déploiement du gestionnaire n’est installé pour ce jeu.';

  @override
  String get statusDetailsInSyncDescription =>
      'Les mods déployés correspondent à la configuration actuelle.';

  @override
  String get statusDetailsDeployedLoadout => 'Ordre de chargement déployé';

  @override
  String get statusDetailsChangesDescription =>
      'Le déploiement actuel diffère de ce qu’Appliquer installera.';

  @override
  String get statusDetailsCurrentlyDeployed => 'Actuellement déployé';

  @override
  String get statusDetailsAfterApply => 'Après Appliquer';

  @override
  String get statusDetailsGameUpdatedDescription =>
      'Des fichiers du jeu ont changé depuis le dernier déploiement. Réappliquez la configuration pour restaurer les fichiers du gestionnaire.';

  @override
  String get statusDetailsDriftedFiles => 'Fichiers modifiés';

  @override
  String get statusDetailsStudioDescription =>
      'Mod Studio contrôle actuellement cette installation du jeu. Prenez le relais avant d’appliquer une configuration du gestionnaire.';

  @override
  String statusDetailsStudioMod(String name) {
    return 'Mod Studio : $name';
  }

  @override
  String get statusDetailsStudioNameUnknown =>
      'Studio n’a pas indiqué le nom du mod.';

  @override
  String get statusDetailsRecoveryDescription =>
      'Un déploiement a été interrompu. Récupérez-le avant d’appliquer ou de supprimer des mods du gestionnaire.';

  @override
  String get statusDetailsUnknownDescription =>
      'L’état du déploiement n’a pas pu être vérifié. Actualisez-le avant d’appliquer des mods.';

  @override
  String get statusDetailsUnavailable =>
      'Le moteur installé n’a pas fourni ces détails.';

  @override
  String get statusDetailsEmptyLoadout => 'Aucun mod dans cette configuration.';

  @override
  String get statusDetailsLastError => 'Dernière erreur';

  @override
  String get statusDetailsLastApply => 'Dernière application';

  @override
  String get statusDetailsAppliedMods => 'Mods appliqués';

  @override
  String get statusDetailsWarnings => 'Avertissements';

  @override
  String get statusDetailsReapply => 'Réappliquer';

  @override
  String get statusDetailsOpenSettings => 'Ouvrir les paramètres';

  @override
  String get recoveryAction => 'Récupérer';

  @override
  String get recoveryRequiredConfirm =>
      'Récupérer le déploiement interrompu et supprimer les fichiers partiellement déployés ?';

  @override
  String get statusRecoveryRequired => 'Récupération requise';

  @override
  String get statusDetailsOwnershipTitle => 'Preuves de propriété enregistrées';

  @override
  String get statusDetailsOwnershipDescription =>
      'Chemins enregistrés dans le journal de déploiement du gestionnaire. Ils ne prouvent pas que ces chemins existent encore.';

  @override
  String get statusDetailsOwnershipLive => 'Fichiers du jeu remplacés';

  @override
  String get statusDetailsOwnershipBackups => 'Sauvegardes d\'origine';

  @override
  String get statusDetailsOwnershipAdditive =>
      'Fichiers pak et conteneurs ajoutés';

  @override
  String get statusDetailsOwnershipUe4ss => 'Dossiers de mods UE4SS';

  @override
  String get statusDetailsOwnershipRecovery =>
      'Fichiers et emplacements de récupération';

  @override
  String get statusDetailsOwnershipEmpty =>
      'Aucun chemin enregistré dans ce groupe.';

  @override
  String statusDetailsOwnershipShown(int shown, int total) {
    return '$shown chemins enregistrés affichés sur $total.';
  }

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
  String importOutcomeCreated(String name) {
    return '« $name » a été ajouté à la bibliothèque.';
  }

  @override
  String importOutcomeUpdated(String name) {
    return '« $name » a été mis à jour dans la bibliothèque.';
  }

  @override
  String importOutcomeUnchanged(String name) {
    return '« $name » se trouve déjà dans la bibliothèque.';
  }

  @override
  String importOutcomeMatchedBy(String method) {
    String _temp0 = intl.Intl.selectLogic(method, {
      'none':
          'Aucune correspondance avec une entrée existante de la bibliothèque.',
      'source': 'Correspondance avec la même source d’importation.',
      'content': 'Correspondance avec un contenu identique vérifié.',
      'entry_id': 'Correspondance avec l’ID du mod.',
      'other': 'Les détails de la correspondance ne sont pas disponibles.',
    });
    return '$_temp0';
  }

  @override
  String get importRefusalDuplicateAmbiguous =>
      'Cette importation correspond à plusieurs entrées de la bibliothèque. Vérifiez ou supprimez les doublons, puis réessayez.';

  @override
  String get importRefusalIdentityConflict =>
      'La source d’importation et son contenu correspondent à des entrées différentes de la bibliothèque. Vérifiez ou supprimez les entrées en conflit, puis réessayez.';

  @override
  String get importFailed =>
      'L’importation n’a pas pu être terminée. Sources prises en charge : dossiers, ZIP, fichiers *_P.pak autonomes, ensembles .utoc/.ucas complets (.pak facultatif), .lcache, .bank et PrecompiledScript*.Cache. Extrayez d’abord les archives .7z ou .rar, puis importez le dossier. La source peut être non prise en charge, endommagée ou incomplète. Le mod a peut-être déjà été ajouté ou mis à jour ; actualisez et vérifiez la bibliothèque avant de réessayer.';

  @override
  String get importPickerFailed =>
      'Le sélecteur de fichiers ou de dossiers n’a pas pu être ouvert. Aucune importation n’a été lancée. Réessayez.';

  @override
  String get importOutcomeUnknown =>
      'Le résultat de l’importation n’a pas pu être vérifié. Sélectionnez Actualiser pour vérifier la bibliothèque.';

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
    return 'Résultats ($count)';
  }

  @override
  String get conflictWinner => 'gagnant prévu';

  @override
  String get noConflicts => 'Aucun conflit reconnu.';

  @override
  String get conflictCoverageIncomplete =>
      'La connaissance des conflits des mods activés est incomplète ; d’autres conflits peuvent exister.';

  @override
  String get loadOrderDirection =>
      'Ordre de chargement : priorité basse d’abord ; les mods placés après ont une priorité prévue plus élevée.';

  @override
  String get footprintCoverageScope =>
      'La couverture décrit uniquement les cibles de conflit reconnues ; elle ne prouve pas la priorité à l’exécution.';

  @override
  String get footprintCoverageExact =>
      'Exacte — la liste des cibles de conflit du composant est complète.';

  @override
  String get footprintCoveragePartial =>
      'Partielle — les cibles indiquées sont connues, mais le composant peut en affecter d’autres.';

  @override
  String get footprintCoverageAdvisory =>
      'Indicative — les cibles indiquées sont des indices, pas une preuve exhaustive.';

  @override
  String get footprintCoverageOpaque =>
      'Opaque — les cibles de conflit du composant sont inconnues.';

  @override
  String get footprintCoverageExactLabel => 'Exacte';

  @override
  String get footprintCoveragePartialLabel => 'Partielle';

  @override
  String get footprintCoverageAdvisoryLabel => 'Indicative';

  @override
  String get footprintCoverageOpaqueLabel => 'Opaque';

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
