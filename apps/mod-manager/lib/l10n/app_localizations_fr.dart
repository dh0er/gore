// ignore: unused_import
import 'package:intl/intl.dart' as intl;
import 'app_localizations.dart';

// ignore_for_file: type=lint

/// The translations for French (`fr`).
class AppLocalizationsFr extends AppLocalizations {
  AppLocalizationsFr([String locale = 'fr']) : super(locale);

  @override
  String get coreBlockedTitle => 'Le Mod Manager ne peut pas démarrer';

  @override
  String get coreDllMissingMessage =>
      'Un fichier requis du programme est manquant (gore_ffi.dll).';

  @override
  String get coreDllLoadFailedMessage =>
      'Un fichier requis du programme n\'a pas pu être chargé.';

  @override
  String get coreVerificationFailedMessage =>
      'Un fichier requis du programme n\'a pas pu être vérifié.';

  @override
  String get coreManagerTooOldMessage =>
      'Les fichiers du programme sont plus récents que le Mod Manager. Mettez à jour le Mod Manager.';

  @override
  String get coreNativeTooOldMessage =>
      'Les fichiers du programme sont plus anciens que le Mod Manager. Réinstallez le Mod Manager.';

  @override
  String get coreCommandsMissingMessage =>
      'Il manque aux fichiers du programme des fonctions dont ce Mod Manager a besoin.';

  @override
  String get coreBlockedRepairHint =>
      'Réinstallez ou réparez le Mod Manager, puis relancez-le.';

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
      'Quelque chose doit être réglé avant de pouvoir changer les mods.';

  @override
  String get preflightGameRunning =>
      'Gothic est toujours ouvert. Fermez le jeu avant de modifier les mods.';

  @override
  String get managerOperationFailed => 'L\'opération a échoué.';

  @override
  String get libraryOperationFailed =>
      'La liste des mods n\'a pas pu être chargée.';

  @override
  String get conflictsUnavailable => 'Les conflits n’ont pas pu être vérifiés.';

  @override
  String applyReportAppliedWithWarnings(int applied, int warnings) {
    return 'Appliqués : $applied. Avertissements : $warnings.';
  }

  @override
  String get modDetailKind => 'Type';

  @override
  String get modDetailVersion => 'Version';

  @override
  String get modDetailAuthor => 'Auteur';

  @override
  String get modDetailSource => 'Source';

  @override
  String get modDetailImported => 'Importé';

  @override
  String get componentLocalization => 'Textes';

  @override
  String get componentAudio => 'Sons';

  @override
  String get componentAngelScript => 'Scripts';

  @override
  String get componentTexture => 'Textures';

  @override
  String get componentGameFiles => 'Fichiers du jeu';

  @override
  String get componentVoice => 'Voix';

  @override
  String get componentKindLocalizationPatch => 'Modifications de texte';

  @override
  String get componentKindAudioPatch => 'Modifications sonores';

  @override
  String get componentKindAngelScriptPatch => 'Modifications de scripts';

  @override
  String get componentKindTexturePatch => 'Modifications de textures';

  @override
  String get componentKindLoosePak => 'Fichier PAK';

  @override
  String get componentKindTriplet => 'Conteneur IoStore';

  @override
  String get componentKindUe4ssLua => 'Script UE4SS';

  @override
  String get componentKindRawFile => 'Fichier';

  @override
  String get componentKindFilePatch => 'Fichier du jeu remplacé';

  @override
  String get componentKindPakFilePatch => 'Fichier du jeu depuis un PAK ~mods';

  @override
  String get componentKindVoiceArchivePatch => 'Voix';

  @override
  String get rawTargetGameText => 'Tous les textes du jeu';

  @override
  String get rawTargetGameScripts => 'Tous les scripts du jeu';

  @override
  String get rawTargetSoundBank => 'Banque de sons';

  @override
  String rawTargetSoundBankNamed(String name) {
    return 'Banque de sons : $name';
  }

  @override
  String get conflictKindLocalization => 'Textes';

  @override
  String get conflictKindAudio => 'Sons';

  @override
  String get conflictKindAsset => 'Données du jeu';

  @override
  String get conflictKindCdo => 'Valeurs d\'objets';

  @override
  String get conflictKindUe4ssUnknown => 'UE4SS (incertain)';

  @override
  String get conflictKindScriptModule => 'Script du jeu';

  @override
  String get conflictKindVoiceArchive => 'Voix';

  @override
  String get conflictKindRawFile => 'Fichier';

  @override
  String get conflictKindLooseFile => 'Fichier du jeu';

  @override
  String get preflightUnavailable =>
      'L\'installation du jeu n\'a pas pu être vérifiée.';

  @override
  String get preflightRetry => 'Vérifier à nouveau';

  @override
  String get preflightReviewStatus => 'Afficher l\'état';

  @override
  String get preflightReviewRecovery => 'Afficher l\'aide';

  @override
  String get installRecoveryTitle => 'Installation interrompue';

  @override
  String get installRecoveryBody =>
      'GORE a trouvé des restes d\'une installation ou d\'une compilation de scripts. Ce travail est peut-être encore en cours, ou il s\'est terminé en laissant ceci. GORE ne peut pas nettoyer cela seul en toute sécurité.';

  @override
  String get installRecoverySteps =>
      'Si le travail est encore en cours, attendez qu\'il se termine : ne l\'arrêtez pas et ne supprimez aucun fichier. Quand vous êtes sûr que rien ne tourne, suivez le README.txt du dossier ci-dessous puis vérifiez à nouveau. Si aucun dossier n\'est indiqué ou en cas de doute, ne touchez à rien et demandez de l\'aide.';

  @override
  String get installRecoveryEvidence => 'Ce que GORE a trouvé';

  @override
  String get managerRecoveryTitle => 'Réparer la modification interrompue';

  @override
  String get managerRecoveryConfirm =>
      'GORE a trouvé une modification interrompue et peut remettre le jeu dans un état connu. Vos sauvegardes ne sont jamais touchées.';

  @override
  String get managerRecoveryAlreadyClean =>
      'Plus rien à réparer. L\'état a été revérifié.';

  @override
  String get managerRecoveryBusy =>
      'Le travail est de nouveau en cours. Rien n\'a été modifié — attendez la fin.';

  @override
  String get managerRecoveryLockCleared =>
      'Le travail interrompu n\'avait encore rien modifié. Il a été nettoyé.';

  @override
  String get managerRecoveryRestoredPristine =>
      'La modification a été annulée. Le jeu est revenu à son état précédent.';

  @override
  String get managerRecoveryApplyPreserved =>
      'L\'application était déjà terminée. Rien n\'a été perdu.';

  @override
  String get managerRecoveryUndeployConfirmed =>
      'La suppression était déjà terminée. Les restes ont été nettoyés.';

  @override
  String get managerRecoveryCompileRequired =>
      'Cela relève d\'une compilation de scripts ; rien n\'a été modifié. Ouvrez l\'aide de réparation.';

  @override
  String get managerRecoveryInspectionFailed =>
      'GORE n\'a pas pu vérifier le travail interrompu en toute sécurité. Rien n\'a été modifié.';

  @override
  String get managerRecoveryFailed =>
      'La réparation n\'a pas pu aboutir. Vérifiez l\'état avant de réessayer.';

  @override
  String get statusUnknown => 'Inconnu';

  @override
  String statusDetailsTitle(String status) {
    return 'État : $status';
  }

  @override
  String statusDetailsOpen(String status) {
    return 'Afficher les détails : $status';
  }

  @override
  String get statusDetailsNoRoot =>
      'Choisissez d\'abord votre installation de Gothic dans les réglages.';

  @override
  String get statusDetailsNoDeployment =>
      'Aucun mod n\'est installé dans le jeu pour le moment.';

  @override
  String get statusDetailsInSyncDescription =>
      'Le jeu contient exactement les mods cochés ici.';

  @override
  String get statusDetailsDeployedLoadout => 'Mods dans le jeu';

  @override
  String get statusDetailsChangesDescription =>
      'Votre sélection diffère de ce qui est dans le jeu.';

  @override
  String get statusDetailsCurrentlyDeployed => 'Actuellement dans le jeu';

  @override
  String get statusDetailsAfterApply => 'Après application';

  @override
  String get statusDetailsGameUpdatedDescription =>
      'Le jeu a été mis à jour et a écrasé des fichiers de mods. Appliquez de nouveau pour les rétablir.';

  @override
  String get statusDetailsDriftedFiles => 'Fichiers concernés';

  @override
  String get statusDetailsStudioDescription =>
      'Mod Studio a actuellement des mods dans ce jeu. Reprenez le jeu en main avant que le Manager applique les vôtres.';

  @override
  String statusDetailsStudioMod(String name) {
    return 'Mod Studio : $name';
  }

  @override
  String get statusDetailsStudioNameUnknown =>
      'Mod Studio n\'a indiqué aucun nom.';

  @override
  String get statusDetailsRecoveryDescription =>
      'Une modification a été interrompue. Réparez-la avant de changer des mods.';

  @override
  String get statusDetailsUnknownDescription =>
      'L\'état n\'a pas pu être lu. Actualisez d\'abord.';

  @override
  String get statusDetailsUnavailable => 'Aucun détail disponible.';

  @override
  String get statusDetailsEmptyLoadout => 'Aucun mod.';

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
  String get recoveryAction => 'Réparer';

  @override
  String get recoveryRequiredConfirm =>
      'Réparer la modification interrompue et supprimer les fichiers à moitié installés ?';

  @override
  String get statusRecoveryRequired => 'Réparation nécessaire';

  @override
  String get statusDetailsOwnershipTitle => 'Fichiers gérés par GORE';

  @override
  String get statusDetailsOwnershipDescription =>
      'Enregistré lors de l\'application des mods ; ne vérifie pas que les fichiers existent encore.';

  @override
  String get statusDetailsOwnershipLive => 'Fichiers du jeu remplacés';

  @override
  String get statusDetailsOwnershipBackups => 'Sauvegardes des originaux';

  @override
  String get statusDetailsOwnershipAdditive => 'Fichiers de mods ajoutés';

  @override
  String get statusDetailsOwnershipUe4ss => 'Dossiers de mods UE4SS';

  @override
  String get statusDetailsOwnershipRecovery => 'Fichiers de réparation';

  @override
  String get statusDetailsOwnershipEmpty => 'Rien d\'enregistré ici.';

  @override
  String statusDetailsOwnershipShown(int shown, int total) {
    return '$shown chemins affichés sur $total.';
  }

  @override
  String get appTitle => 'GORE Mod Manager';

  @override
  String get tabMods => 'Mods';

  @override
  String get tabSettings => 'Paramètres';

  @override
  String get settingsGameExe => 'Installation de Gothic';

  @override
  String get settingsGameExePick => 'Choisir…';

  @override
  String get settingsLanguage => 'Langue';

  @override
  String get libraryEmptyTitle => 'Aucun mod pour l\'instant';

  @override
  String get libraryEmptyBody =>
      'Importez un dossier ou un fichier de mod pour commencer.';

  @override
  String get detailEmptyHint =>
      'Choisissez un mod pour voir ce qu\'il modifie.';

  @override
  String get settingsAdvanced => 'Détails avancés';

  @override
  String get settingsAdvancedHint =>
      'Affiche le côté technique : entrées concernées, fiabilité de la détection de conflits et fichiers gérés par GORE.';

  @override
  String get updatesTitle => 'Mises à jour';

  @override
  String get checkForUpdatesAutomatically =>
      'Rechercher les mises à jour automatiquement';

  @override
  String get checkForUpdatesNow => 'Rechercher les mises à jour maintenant';

  @override
  String get updatesPortableNotice =>
      'La version portable ouvre la page de téléchargement dans votre navigateur. Remplacez vos fichiers actuels par le nouveau téléchargement.';

  @override
  String get updateCheckFailed =>
      'Impossible de rechercher les mises à jour. Réessayez plus tard.';

  @override
  String get updateUpToDate => 'Vous utilisez la dernière version.';

  @override
  String get updateAvailableTitle => 'Mise à jour disponible';

  @override
  String updateAvailableMessage(String version, String current) {
    return 'La version $version est disponible. Vous avez la $current.';
  }

  @override
  String get updateLater => 'Plus tard';

  @override
  String get updateDownload => 'Télécharger';

  @override
  String updateOpenFailed(String url) {
    return 'Impossible d\'ouvrir la page de téléchargement. Vous pouvez y accéder à $url';
  }

  @override
  String get statusInSync => 'À jour';

  @override
  String get statusChangesPending => 'Non appliqué';

  @override
  String get statusGameUpdated => 'Le jeu a été mis à jour';

  @override
  String get statusStudioDeploy => 'Mod Studio actif';

  @override
  String get statusNothingDeployed => 'Aucun mod dans le jeu';

  @override
  String get actionImport => 'Importer';

  @override
  String get actionApply => 'Appliquer';

  @override
  String get actionStartGame => 'Lancer le jeu';

  @override
  String get startGameTooltip =>
      'Lancer Gothic avec les mods actuellement dans le jeu';

  @override
  String get startGameFailed =>
      'Impossible de lancer Gothic. Vérifiez l\'installation du jeu dans les réglages.';

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
    return '« $name » ajouté.';
  }

  @override
  String importOutcomeUpdated(String name) {
    return '« $name » mis à jour.';
  }

  @override
  String importOutcomeUnchanged(String name) {
    return '« $name » est déjà dans votre liste.';
  }

  @override
  String importOutcomeMatchedBy(String method) {
    String _temp0 = intl.Intl.selectLogic(method, {
      'none': 'Aucun mod existant ne correspond.',
      'source': 'Correspondance par la même source d\'import.',
      'content': 'Correspondance par contenu identique vérifié.',
      'entry_id': 'Correspondance par identifiant de mod.',
      'other': 'Détails de correspondance indisponibles.',
    });
    return '$_temp0';
  }

  @override
  String get importRefusalDuplicateAmbiguous =>
      'Ceci correspond à plusieurs mods que vous avez déjà. Supprimez les doublons, puis réessayez.';

  @override
  String get importRefusalIdentityConflict =>
      'La source et le contenu correspondent à deux mods différents que vous avez déjà. Réglez cela, puis réessayez.';

  @override
  String get importFailed =>
      'Impossible d\'importer ceci. Pris en charge : dossiers, archives ZIP et fichiers de mod isolés (*_P.pak, .utoc/.ucas, .lcache, .bank, PrecompiledScript*.Cache). Extrayez d\'abord les .7z ou .rar, puis importez le dossier. Le mod a peut-être quand même été ajouté ou mis à jour : actualisez la liste avant de réessayer.';

  @override
  String get importPickerFailed =>
      'Le sélecteur de fichiers n\'a pas pu s\'ouvrir. Rien n\'a été importé.';

  @override
  String get importOutcomeUnknown =>
      'Le résultat est incertain. Actualisez pour vérifier votre liste de mods.';

  @override
  String get applyTooltip => 'Installer dans le jeu les mods cochés';

  @override
  String get undeployAllAction => 'Tout retirer du jeu';

  @override
  String get undeployAllConfirm =>
      'Retirer du jeu tous les mods installés par le Manager ?';

  @override
  String get takeOverTitle => 'Mod Studio est actif';

  @override
  String get takeOverBody =>
      'Mod Studio a actuellement un mod dans le jeu. Reprendre la main pour que le Manager applique votre sélection ?';

  @override
  String get takeOverAction => 'Prendre le relais';

  @override
  String get refreshAction => 'Actualiser';

  @override
  String conflictsTitle(int count) {
    return 'Conflits ($count)';
  }

  @override
  String get conflictWinner => 'l\'emporte';

  @override
  String get noConflicts => 'Aucun conflit trouvé.';

  @override
  String get conflictCoverageIncomplete =>
      'Certains mods ne peuvent pas être vérifiés entièrement ; il peut y avoir d\'autres conflits.';

  @override
  String get loadOrderDirection =>
      'Les mods plus bas dans la liste remplacent ceux du dessus.';

  @override
  String get footprintCoverageScope =>
      'Seules les cibles de conflit connues sont listées. Cela ne garantit pas le résultat en jeu.';

  @override
  String get footprintTargetsExact => 'Entrées concernées — liste complète :';

  @override
  String get footprintTargetsPartial =>
      'Entrées concernées — il peut y en avoir d\'autres :';

  @override
  String get footprintTargetsAdvisory =>
      'Entrées probablement concernées — des indices, pas des preuves :';

  @override
  String get footprintTargetsOpaque =>
      'GORE ne peut pas déterminer ce que cela modifie.';

  @override
  String get conflictsUnverified => 'Conflits inconnus — actualisez d\'abord.';

  @override
  String get componentsTitle => 'Ce que ce mod modifie';

  @override
  String targetsMore(int count) {
    return '+$count de plus';
  }

  @override
  String get removeModDeploymentHint =>
      'Cela le retire seulement de votre liste. S\'il est installé dans le jeu, choisissez ensuite Appliquer.';

  @override
  String removeModSuccess(String name) {
    return '« $name » retiré.';
  }

  @override
  String removeModFailed(String name) {
    return 'Impossible de retirer « $name ».';
  }

  @override
  String removeModPartialFailure(String name) {
    return '« $name » retiré, mais la liste n\'a pas pu être entièrement actualisée.';
  }

  @override
  String removeModOutcomeUnknown(String name) {
    return 'Impossible de confirmer si « $name » a été retiré.';
  }

  @override
  String get libraryStateUnknown =>
      'La liste des mods n\'est pas à jour. Actualisez avant de changer ou d\'appliquer des mods.';

  @override
  String get removeModAction => 'Retirer';

  @override
  String removeModConfirm(String name) {
    return 'Retirer « $name » de votre liste ?';
  }

  @override
  String get errorSetGamePath =>
      'Choisissez d\'abord votre installation de Gothic dans les réglages.';

  @override
  String applyReportApplied(int count) {
    return '$count mods appliqués.';
  }

  @override
  String get modDisabledHint => 'Désactivé';

  @override
  String get kindGoremod => 'Bundle GORE';

  @override
  String get kindTriplet => 'Mod IoStore';

  @override
  String get kindPak => 'Mod PAK';

  @override
  String get kindUe4ss => 'UE4SS';

  @override
  String get kindRawfile => 'Remplacement de fichiers entiers';

  @override
  String get kindMixed => 'Mixte';

  @override
  String get sevHard => 'Conflit';

  @override
  String get sevSoft => 'Avertissement';

  @override
  String get sevInfo => 'Info';

  @override
  String aboutVersion(String version, String sha) {
    return 'Version $version ($sha)';
  }

  @override
  String get about => 'À propos';

  @override
  String get aboutCopyright => '© 2026 Daniel Hoer';

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
  String get uiScale => 'Taille d\'affichage';

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
