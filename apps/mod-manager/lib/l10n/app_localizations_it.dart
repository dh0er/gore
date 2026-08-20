// ignore: unused_import
import 'package:intl/intl.dart' as intl;
import 'app_localizations.dart';

// ignore_for_file: type=lint

/// The translations for Italian (`it`).
class AppLocalizationsIt extends AppLocalizations {
  AppLocalizationsIt([String locale = 'it']) : super(locale);

  @override
  String get coreBlockedTitle => 'Il Mod Manager non può avviarsi';

  @override
  String get coreDllMissingMessage =>
      'Manca un file necessario del programma (gore_ffi.dll).';

  @override
  String get coreDllLoadFailedMessage =>
      'Non è stato possibile caricare un file necessario del programma.';

  @override
  String get coreVerificationFailedMessage =>
      'Non è stato possibile verificare un file necessario del programma.';

  @override
  String get coreManagerTooOldMessage =>
      'I file del programma sono più recenti del Mod Manager. Aggiorna il Mod Manager.';

  @override
  String get coreNativeTooOldMessage =>
      'I file del programma sono più vecchi del Mod Manager. Reinstalla il Mod Manager.';

  @override
  String get coreCommandsMissingMessage =>
      'Ai file del programma mancano funzioni che questo Mod Manager richiede.';

  @override
  String get coreBlockedRepairHint =>
      'Reinstalla o ripara il Mod Manager, poi riavvialo.';

  @override
  String get coreTechnicalDetails => 'Dettagli tecnici';

  @override
  String get coreCopyTechnicalDetails => 'Copia dettagli tecnici';

  @override
  String get coreTechnicalDetailsCopied => 'Dettagli tecnici copiati';

  @override
  String get coreTechnicalDetailsCopyFailed =>
      'Impossibile copiare i dettagli tecnici. Riprova.';

  @override
  String get preflightAttention =>
      'C\'è qualcosa da sistemare prima di poter cambiare le mod.';

  @override
  String get preflightGameRunning =>
      'Gothic è ancora aperto. Chiudi il gioco prima di modificare le mod.';

  @override
  String get managerOperationFailed => 'L\'operazione non è riuscita.';

  @override
  String get libraryOperationFailed =>
      'Non è stato possibile caricare l\'elenco delle mod.';

  @override
  String get conflictsUnavailable =>
      'Non è stato possibile verificare i conflitti.';

  @override
  String applyReportAppliedWithWarnings(int applied, int warnings) {
    return 'Applicati: $applied. Avvisi: $warnings.';
  }

  @override
  String get modDetailKind => 'Tipo';

  @override
  String get modDetailVersion => 'Versione';

  @override
  String get modDetailAuthor => 'Autore';

  @override
  String get modDetailSource => 'Origine';

  @override
  String get modDetailImported => 'Importato';

  @override
  String get componentLocalization => 'Testi';

  @override
  String get componentAudio => 'Audio';

  @override
  String get componentAngelScript => 'Script';

  @override
  String get componentTexture => 'Texture';

  @override
  String get componentGameFiles => 'File di gioco';

  @override
  String get componentVoice => 'Voci';

  @override
  String get componentKindLocalizationPatch => 'Modifiche ai testi';

  @override
  String get componentKindAudioPatch => 'Modifiche audio';

  @override
  String get componentKindAngelScriptPatch => 'Modifiche agli script';

  @override
  String get componentKindTexturePatch => 'Modifiche alle texture';

  @override
  String get componentKindLoosePak => 'File PAK';

  @override
  String get componentKindTriplet => 'Contenitore IoStore';

  @override
  String get componentKindUe4ssLua => 'Script UE4SS';

  @override
  String get componentKindRawFile => 'File';

  @override
  String get componentKindFilePatch => 'File di gioco sostituito';

  @override
  String get componentKindPakFilePatch => 'File di gioco da un PAK in ~mods';

  @override
  String get componentKindVoiceArchivePatch => 'Voci';

  @override
  String get rawTargetGameText => 'Tutti i testi di gioco';

  @override
  String get rawTargetGameScripts => 'Tutti gli script di gioco';

  @override
  String get rawTargetSoundBank => 'Banco audio';

  @override
  String rawTargetSoundBankNamed(String name) {
    return 'Banco audio: $name';
  }

  @override
  String get conflictKindLocalization => 'Testi';

  @override
  String get conflictKindAudio => 'Audio';

  @override
  String get conflictKindAsset => 'Dati di gioco';

  @override
  String get conflictKindCdo => 'Valori degli oggetti';

  @override
  String get conflictKindUe4ssUnknown => 'UE4SS (non chiaro)';

  @override
  String get conflictKindScriptModule => 'Script di gioco';

  @override
  String get conflictKindVoiceArchive => 'Voci';

  @override
  String get conflictKindRawFile => 'File';

  @override
  String get conflictKindLooseFile => 'File di gioco';

  @override
  String get preflightUnavailable =>
      'Non è stato possibile controllare l\'installazione del gioco.';

  @override
  String get preflightRetry => 'Controlla di nuovo';

  @override
  String get preflightReviewStatus => 'Mostra stato';

  @override
  String get preflightReviewRecovery => 'Mostra aiuto';

  @override
  String get installRecoveryTitle => 'Installazione interrotta';

  @override
  String get installRecoveryBody =>
      'GORE ha trovato residui di un\'installazione o di una compilazione di script. Quel processo potrebbe essere ancora in corso, oppure è finito lasciando questi dati. GORE non può ripulirli da solo in sicurezza.';

  @override
  String get installRecoverySteps =>
      'Se il processo è ancora in corso, aspetta che finisca: non interromperlo e non eliminare file. Quando sei sicuro che non stia girando nulla, segui il README.txt nella cartella qui sotto e controlla di nuovo. Se non è indicata alcuna cartella o hai dubbi, lascia tutto com\'è e chiedi aiuto.';

  @override
  String get installRecoveryEvidence => 'Cosa ha trovato GORE';

  @override
  String get managerRecoveryTitle => 'Ripara la modifica interrotta';

  @override
  String get managerRecoveryConfirm =>
      'GORE ha trovato una modifica interrotta e può riportare il gioco a uno stato noto. I tuoi salvataggi non vengono mai toccati.';

  @override
  String get managerRecoveryAlreadyClean =>
      'Non c\'era più nulla da riparare. Lo stato è stato ricontrollato.';

  @override
  String get managerRecoveryBusy =>
      'Il processo è di nuovo attivo. Non è stato cambiato nulla: aspetta che finisca.';

  @override
  String get managerRecoveryLockCleared =>
      'Il processo interrotto non aveva ancora cambiato nulla. È stato ripulito.';

  @override
  String get managerRecoveryRestoredPristine =>
      'La modifica è stata annullata. Il gioco è tornato allo stato precedente.';

  @override
  String get managerRecoveryApplyPreserved =>
      'L\'applicazione era già finita. Non è andato perso nulla.';

  @override
  String get managerRecoveryUndeployConfirmed =>
      'La rimozione era già finita. I residui sono stati ripuliti.';

  @override
  String get managerRecoveryCompileRequired =>
      'Questo riguarda una compilazione di script, quindi non è stato cambiato nulla. Apri l\'aiuto per la riparazione.';

  @override
  String get managerRecoveryInspectionFailed =>
      'GORE non è riuscito a controllare in sicurezza il processo interrotto. Non è stato cambiato nulla.';

  @override
  String get managerRecoveryFailed =>
      'Non è stato possibile completare la riparazione. Controlla lo stato prima di riprovare.';

  @override
  String get statusUnknown => 'Sconosciuto';

  @override
  String statusDetailsTitle(String status) {
    return 'Stato: $status';
  }

  @override
  String statusDetailsOpen(String status) {
    return 'Mostra dettagli: $status';
  }

  @override
  String get statusDetailsNoRoot =>
      'Scegli prima la tua installazione di Gothic nelle impostazioni.';

  @override
  String get statusDetailsNoDeployment =>
      'Al momento non ci sono mod installate nel gioco.';

  @override
  String get statusDetailsInSyncDescription =>
      'Nel gioco ci sono esattamente le mod selezionate qui.';

  @override
  String get statusDetailsDeployedLoadout => 'Mod nel gioco';

  @override
  String get statusDetailsChangesDescription =>
      'La tua selezione è diversa da ciò che è nel gioco.';

  @override
  String get statusDetailsCurrentlyDeployed => 'Ora nel gioco';

  @override
  String get statusDetailsAfterApply => 'Dopo l\'applicazione';

  @override
  String get statusDetailsGameUpdatedDescription =>
      'Il gioco è stato aggiornato e ha sovrascritto file delle mod. Applica di nuovo per ripristinarli.';

  @override
  String get statusDetailsDriftedFiles => 'File interessati';

  @override
  String get statusDetailsStudioDescription =>
      'Mod Studio ha attualmente delle mod in questo gioco. Prendi il controllo del gioco prima che il Manager applichi le tue.';

  @override
  String statusDetailsStudioMod(String name) {
    return 'Mod di Studio: $name';
  }

  @override
  String get statusDetailsStudioNameUnknown =>
      'Mod Studio non ha indicato un nome.';

  @override
  String get statusDetailsRecoveryDescription =>
      'Una modifica è stata interrotta. Riparala prima di cambiare le mod.';

  @override
  String get statusDetailsUnknownDescription =>
      'Impossibile leggere lo stato. Aggiorna prima.';

  @override
  String get statusDetailsUnavailable => 'Nessun dettaglio disponibile.';

  @override
  String get statusDetailsEmptyLoadout => 'Nessuna mod.';

  @override
  String get statusDetailsLastError => 'Ultimo errore';

  @override
  String get statusDetailsLastApply => 'Ultima applicazione';

  @override
  String get statusDetailsAppliedMods => 'Mod applicati';

  @override
  String get statusDetailsWarnings => 'Avvisi';

  @override
  String get statusDetailsReapply => 'Riapplica';

  @override
  String get statusDetailsOpenSettings => 'Apri Impostazioni';

  @override
  String get recoveryAction => 'Ripara';

  @override
  String get recoveryRequiredConfirm =>
      'Riparare la modifica interrotta e rimuovere i file installati a metà?';

  @override
  String get statusRecoveryRequired => 'Riparazione necessaria';

  @override
  String get statusDetailsOwnershipTitle => 'File gestiti da GORE';

  @override
  String get statusDetailsOwnershipDescription =>
      'Registrato all\'applicazione delle mod; non verifica che i file esistano ancora.';

  @override
  String get statusDetailsOwnershipLive => 'File di gioco sostituiti';

  @override
  String get statusDetailsOwnershipBackups => 'Backup degli originali';

  @override
  String get statusDetailsOwnershipAdditive => 'File delle mod aggiunti';

  @override
  String get statusDetailsOwnershipUe4ss => 'Cartelle mod UE4SS';

  @override
  String get statusDetailsOwnershipRecovery => 'File di riparazione';

  @override
  String get statusDetailsOwnershipEmpty => 'Qui non è registrato nulla.';

  @override
  String statusDetailsOwnershipShown(int shown, int total) {
    return 'Mostrati $shown di $total percorsi.';
  }

  @override
  String get appTitle => 'GORE Mod Manager';

  @override
  String get tabMods => 'Mod';

  @override
  String get tabSettings => 'Impostazioni';

  @override
  String get settingsGameExe => 'Installazione di Gothic';

  @override
  String get settingsGameExePick => 'Scegli…';

  @override
  String get settingsLanguage => 'Lingua';

  @override
  String get libraryEmptyTitle => 'Ancora nessuna mod';

  @override
  String get libraryEmptyBody =>
      'Importa una cartella o un file di mod per iniziare.';

  @override
  String get detailEmptyHint => 'Scegli una mod per vedere cosa cambia.';

  @override
  String get settingsAdvanced => 'Dettagli avanzati';

  @override
  String get settingsAdvancedHint =>
      'Mostra il lato tecnico: voci interessate, quanto è affidabile il controllo dei conflitti e i file gestiti da GORE.';

  @override
  String get updatesTitle => 'Aggiornamenti';

  @override
  String get checkForUpdatesAutomatically =>
      'Cerca aggiornamenti automaticamente';

  @override
  String get checkForUpdatesNow => 'Cerca aggiornamenti ora';

  @override
  String get updatesPortableNotice =>
      'La versione portable apre la pagina di download nel browser. Sostituisci i file esistenti con il nuovo download.';

  @override
  String get updateCheckFailed =>
      'Impossibile cercare aggiornamenti. Riprova più tardi.';

  @override
  String get updateUpToDate => 'Stai usando la versione più recente.';

  @override
  String get updateAvailableTitle => 'Aggiornamento disponibile';

  @override
  String updateAvailableMessage(String version, String current) {
    return 'È disponibile la versione $version. Hai la $current.';
  }

  @override
  String get updateLater => 'Più tardi';

  @override
  String get updateDownload => 'Scarica';

  @override
  String updateOpenFailed(String url) {
    return 'Non è stato possibile aprire la pagina di download. Puoi raggiungerla su $url';
  }

  @override
  String get statusInSync => 'Aggiornato';

  @override
  String get statusChangesPending => 'Non applicato';

  @override
  String get statusGameUpdated => 'Il gioco è stato aggiornato';

  @override
  String get statusStudioDeploy => 'Mod Studio attivo';

  @override
  String get statusNothingDeployed => 'Nessuna mod nel gioco';

  @override
  String get actionImport => 'Importa';

  @override
  String get actionApply => 'Applica';

  @override
  String get actionStartGame => 'Avvia il gioco';

  @override
  String get startGameTooltip =>
      'Avvia Gothic con le mod attualmente nel gioco';

  @override
  String get startGameFailed =>
      'Non è stato possibile avviare Gothic. Controlla l\'installazione del gioco nelle impostazioni.';

  @override
  String get commonCancel => 'Annulla';

  @override
  String get commonOk => 'OK';

  @override
  String get importFolder => 'Importa cartella…';

  @override
  String get importFile => 'Importa file…';

  @override
  String importOutcomeCreated(String name) {
    return '«$name» aggiunta.';
  }

  @override
  String importOutcomeUpdated(String name) {
    return '«$name» aggiornata.';
  }

  @override
  String importOutcomeUnchanged(String name) {
    return '«$name» è già nel tuo elenco.';
  }

  @override
  String importOutcomeMatchedBy(String method) {
    String _temp0 = intl.Intl.selectLogic(method, {
      'none': 'Nessuna mod esistente corrisponde.',
      'source': 'Corrispondenza per la stessa origine di importazione.',
      'content': 'Corrispondenza per contenuto identico verificato.',
      'entry_id': 'Corrispondenza per ID della mod.',
      'other': 'Dettagli della corrispondenza non disponibili.',
    });
    return '$_temp0';
  }

  @override
  String get importRefusalDuplicateAmbiguous =>
      'Questo corrisponde a più di una mod che hai già. Rimuovi i duplicati e riprova.';

  @override
  String get importRefusalIdentityConflict =>
      'L\'origine e il contenuto corrispondono a due mod diverse che hai già. Sistemale e riprova.';

  @override
  String get importFailed =>
      'Non è stato possibile importarlo. Sono supportati cartelle, archivi ZIP e singoli file di mod (*_P.pak, .utoc/.ucas, .lcache, .bank, PrecompiledScript*.Cache). Estrai prima i .7z o .rar, poi importa la cartella. La mod potrebbe essere stata aggiunta o aggiornata comunque: aggiorna l\'elenco prima di riprovare.';

  @override
  String get importPickerFailed =>
      'Non è stato possibile aprire la selezione file. Non è stato importato nulla.';

  @override
  String get importOutcomeUnknown =>
      'Il risultato non è chiaro. Aggiorna per controllare il tuo elenco di mod.';

  @override
  String get applyTooltip => 'Installa nel gioco le mod selezionate';

  @override
  String get undeployAllAction => 'Rimuovi tutto dal gioco';

  @override
  String get undeployAllConfirm =>
      'Rimuovere dal gioco tutte le mod installate dal Manager?';

  @override
  String get takeOverTitle => 'Mod Studio è attivo';

  @override
  String get takeOverBody =>
      'Mod Studio ha una mod nel gioco. Prendere il controllo perché il Manager applichi la tua selezione?';

  @override
  String get takeOverAction => 'Subentra';

  @override
  String get refreshAction => 'Aggiorna';

  @override
  String conflictsTitle(int count) {
    return 'Conflitti ($count)';
  }

  @override
  String get conflictWinner => 'vince';

  @override
  String get noConflicts => 'Nessun conflitto trovato.';

  @override
  String get conflictCoverageIncomplete =>
      'Alcune mod non possono essere controllate del tutto, quindi potrebbero esserci altri conflitti.';

  @override
  String get loadOrderDirection =>
      'Le mod più in basso nell\'elenco sostituiscono quelle sopra.';

  @override
  String get footprintCoverageScope =>
      'Sono elencati solo i bersagli di conflitto noti. Non è una garanzia di ciò che accade in gioco.';

  @override
  String get footprintTargetsExact => 'Voci interessate — l\'elenco completo:';

  @override
  String get footprintTargetsPartial =>
      'Voci interessate — potrebbero essercene altre:';

  @override
  String get footprintTargetsAdvisory =>
      'Voci probabilmente interessate — indizi, non prove:';

  @override
  String get footprintTargetsOpaque =>
      'GORE non riesce a capire cosa cambia qui.';

  @override
  String get conflictsUnverified => 'Conflitti sconosciuti: aggiorna prima.';

  @override
  String get componentsTitle => 'Cosa cambia questa mod';

  @override
  String targetsMore(int count) {
    return '+$count altri';
  }

  @override
  String get removeModDeploymentHint =>
      'Questo la toglie solo dal tuo elenco. Se è installata nel gioco, scegli poi Applica.';

  @override
  String removeModSuccess(String name) {
    return '«$name» rimossa.';
  }

  @override
  String removeModFailed(String name) {
    return 'Impossibile rimuovere «$name».';
  }

  @override
  String removeModPartialFailure(String name) {
    return '«$name» rimossa, ma l\'elenco non è stato aggiornato del tutto.';
  }

  @override
  String removeModOutcomeUnknown(String name) {
    return 'Non è stato possibile confermare se «$name» è stata rimossa.';
  }

  @override
  String get libraryStateUnknown =>
      'L\'elenco delle mod non è aggiornato. Aggiorna prima di cambiare o applicare le mod.';

  @override
  String get removeModAction => 'Rimuovi';

  @override
  String removeModConfirm(String name) {
    return 'Rimuovere «$name» dal tuo elenco?';
  }

  @override
  String get errorSetGamePath =>
      'Scegli prima la tua installazione di Gothic nelle impostazioni.';

  @override
  String applyReportApplied(int count) {
    return '$count mod applicati.';
  }

  @override
  String get modDisabledHint => 'Disattivato';

  @override
  String get kindGoremod => 'Bundle GORE';

  @override
  String get kindTriplet => 'Mod IoStore';

  @override
  String get kindPak => 'Mod PAK';

  @override
  String get kindUe4ss => 'UE4SS';

  @override
  String get kindRawfile => 'Sostituzione di file interi';

  @override
  String get kindMixed => 'Misto';

  @override
  String get sevHard => 'Conflitto';

  @override
  String get sevSoft => 'Avviso';

  @override
  String get sevInfo => 'Nota';

  @override
  String aboutVersion(String version, String sha) {
    return 'Version $version ($sha)';
  }

  @override
  String get about => 'Informazioni';

  @override
  String get aboutCopyright => '© 2026 Daniel Hoer';

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
  String get uiScale => 'Dimensione';

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
