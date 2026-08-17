// ignore: unused_import
import 'package:intl/intl.dart' as intl;
import 'app_localizations.dart';

// ignore_for_file: type=lint

/// The translations for Italian (`it`).
class AppLocalizationsIt extends AppLocalizations {
  AppLocalizationsIt([String locale = 'it']) : super(locale);

  @override
  String get coreBlockedTitle => 'Mod Manager non disponibile';

  @override
  String get coreDllMissingMessage =>
      'Il file gore_ffi.dll richiesto non è stato trovato.';

  @override
  String get coreDllLoadFailedMessage =>
      'Non è stato possibile caricare la libreria nativa GORE Core.';

  @override
  String get coreVerificationFailedMessage =>
      'Non è stato possibile verificare la libreria nativa GORE Core.';

  @override
  String get coreManagerTooOldMessage =>
      'Questa versione di GORE Core è più recente di Mod Manager. Aggiorna Mod Manager.';

  @override
  String get coreNativeTooOldMessage =>
      'Questa versione di GORE Core è meno recente di Mod Manager. Aggiorna o ripara l’installazione completa di Mod Manager.';

  @override
  String get coreCommandsMissingMessage =>
      'La libreria GORE Core non fornisce tutti i comandi richiesti da questo Mod Manager.';

  @override
  String get coreBlockedRepairHint =>
      'Aggiorna o ripara il pacchetto completo di Mod Manager, quindi riavvia l’app.';

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
  String get preflightAttention => 'La configurazione richiede attenzione.';

  @override
  String get preflightUnavailable =>
      'La diagnosi della configurazione non è disponibile.';

  @override
  String get preflightRetry => 'Controlla di nuovo';

  @override
  String get preflightReviewStatus => 'Controlla stato';

  @override
  String get preflightReviewRecovery => 'Guida';

  @override
  String get installRecoveryTitle => 'Ripristino dell’installazione';

  @override
  String get installRecoveryBody =>
      'GORE ha trovato dati di ripristino relativi a un’installazione o a una compilazione di script. L’operazione associata potrebbe essere ancora in corso, oppure i dati potrebbero provenire da un’operazione già terminata. GORE non può eseguire una riparazione automatica in sicurezza.';

  @override
  String get installRecoverySteps =>
      'Se l’operazione associata è ancora in corso, attendi che termini. Non interromperla e non eliminare alcun file di blocco. Segui il file README.txt nella cartella di ripristino indicata qui sotto solo quando sei certo che non sia in corso alcuna operazione associata. Se non è indicata alcuna cartella o hai dubbi, lascia invariati i dati di ripristino e chiedi assistenza. Poi controlla di nuovo.';

  @override
  String get installRecoveryEvidence => 'Dati di ripristino rilevati';

  @override
  String get managerRecoveryTitle =>
      'Ripristina operazione interrotta del gestore';

  @override
  String get managerRecoveryConfirm =>
      'GORE ha rilevato un’operazione del gestore chiaramente interrotta. Continua solo se vuoi che GORE verifichi l’operazione registrata e riporti l’installazione a uno stato noto. I salvataggi non vengono mai modificati.';

  @override
  String get managerRecoveryAlreadyClean =>
      'L’operazione interrotta era già stata risolta. L’installazione è stata verificata di nuovo.';

  @override
  String get managerRecoveryBusy =>
      'L’operazione è di nuovo attiva. Non è stato modificato nulla; attendi che termini e controlla di nuovo.';

  @override
  String get managerRecoveryLockCleared =>
      'L’operazione interrotta non aveva ancora modificato l’installazione. Il blocco obsoleto è stato rimosso in sicurezza.';

  @override
  String get managerRecoveryRestoredPristine =>
      'La modifica interrotta è stata annullata ed è stato ripristinato lo stato di base registrato dell’installazione.';

  @override
  String get managerRecoveryApplyPreserved =>
      'L’applicazione era già terminata. Lo stato registrato è stato conservato e lo stato è stato controllato di nuovo.';

  @override
  String get managerRecoveryUndeployConfirmed =>
      'La rimozione era terminata. I dati di transazione rimanenti sono stati ripuliti e lo stato è stato controllato di nuovo.';

  @override
  String get managerRecoveryCompileRequired =>
      'Questo riguarda il ripristino della compilazione degli script. Il gestore non ha modificato nulla; consulta la guida al ripristino.';

  @override
  String get managerRecoveryInspectionFailed =>
      'GORE non ha potuto verificare in sicurezza l’operazione interrotta. Non è stato modificato nulla; controlla i dettagli di ripristino attuali.';

  @override
  String get managerRecoveryFailed =>
      'Non è stato possibile completare il ripristino. GORE ha provato a verificare di nuovo l’installazione, ma lo stato attuale potrebbe essere sconosciuto. Controlla lo stato prima di riprovare.';

  @override
  String get statusUnknown => 'Sconosciuto';

  @override
  String statusDetailsTitle(String status) {
    return 'Distribuzione: $status';
  }

  @override
  String statusDetailsOpen(String status) {
    return 'Mostra dettagli distribuzione: $status';
  }

  @override
  String get statusDetailsNoRoot =>
      'Scegli un’installazione del gioco nelle Impostazioni per controllarne lo stato di distribuzione.';

  @override
  String get statusDetailsNoDeployment =>
      'Nessuna distribuzione del gestore è installata per questo gioco.';

  @override
  String get statusDetailsInSyncDescription =>
      'I mod distribuiti corrispondono alla configurazione attuale.';

  @override
  String get statusDetailsDeployedLoadout =>
      'Ordine di caricamento distribuito';

  @override
  String get statusDetailsChangesDescription =>
      'La distribuzione attuale è diversa da ciò che installerà Applica.';

  @override
  String get statusDetailsCurrentlyDeployed => 'Attualmente distribuito';

  @override
  String get statusDetailsAfterApply => 'Dopo Applica';

  @override
  String get statusDetailsGameUpdatedDescription =>
      'I file di gioco sono cambiati dopo l’ultima distribuzione. Riapplica la configurazione per ripristinare i file del gestore.';

  @override
  String get statusDetailsDriftedFiles => 'File modificati';

  @override
  String get statusDetailsStudioDescription =>
      'Mod Studio controlla attualmente questa installazione del gioco. Subentra prima di applicare una configurazione del gestore.';

  @override
  String statusDetailsStudioMod(String name) {
    return 'Mod di Studio: $name';
  }

  @override
  String get statusDetailsStudioNameUnknown =>
      'Studio non ha indicato il nome del mod.';

  @override
  String get statusDetailsRecoveryDescription =>
      'Una distribuzione è stata interrotta. Ripristinala prima di applicare o rimuovere mod del gestore.';

  @override
  String get statusDetailsUnknownDescription =>
      'Impossibile verificare lo stato della distribuzione. Aggiornalo prima di applicare mod.';

  @override
  String get statusDetailsUnavailable =>
      'Il core installato non ha fornito questi dettagli.';

  @override
  String get statusDetailsEmptyLoadout =>
      'Nessun mod in questa configurazione.';

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
  String get recoveryAction => 'Ripristina';

  @override
  String get recoveryRequiredConfirm =>
      'Ripristinare la distribuzione interrotta e rimuovere i file distribuiti parzialmente?';

  @override
  String get statusRecoveryRequired => 'Ripristino necessario';

  @override
  String get statusDetailsOwnershipTitle => 'Prove di proprietà registrate';

  @override
  String get statusDetailsOwnershipDescription =>
      'Percorsi registrati nel record di distribuzione del gestore. Non provano che tali percorsi esistano ancora.';

  @override
  String get statusDetailsOwnershipLive => 'File di gioco sostituiti';

  @override
  String get statusDetailsOwnershipBackups => 'Backup originali';

  @override
  String get statusDetailsOwnershipAdditive =>
      'File pak e contenitori aggiunti';

  @override
  String get statusDetailsOwnershipUe4ss => 'Cartelle mod UE4SS';

  @override
  String get statusDetailsOwnershipRecovery => 'File e posizioni di recupero';

  @override
  String get statusDetailsOwnershipEmpty =>
      'Nessun percorso registrato in questo gruppo.';

  @override
  String statusDetailsOwnershipShown(int shown, int total) {
    return 'Visualizzati $shown di $total percorsi registrati.';
  }

  @override
  String get appTitle => 'GORE Mod Manager';

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
  String importOutcomeCreated(String name) {
    return 'Il mod «$name» è stato aggiunto alla libreria.';
  }

  @override
  String importOutcomeUpdated(String name) {
    return 'Il mod «$name» è stato aggiornato nella libreria.';
  }

  @override
  String importOutcomeUnchanged(String name) {
    return 'Il mod «$name» è già nella libreria.';
  }

  @override
  String importOutcomeMatchedBy(String method) {
    String _temp0 = intl.Intl.selectLogic(method, {
      'none': 'Nessuna corrispondenza con le voci esistenti della libreria.',
      'source': 'Corrispondenza con la stessa origine di importazione.',
      'content': 'Corrispondenza con contenuti identici verificati.',
      'entry_id': 'Corrispondenza con l’ID del mod.',
      'other': 'I dettagli della corrispondenza non sono disponibili.',
    });
    return '$_temp0';
  }

  @override
  String get importRefusalDuplicateAmbiguous =>
      'Questa importazione corrisponde a più di una voce della libreria. Controlla o rimuovi i duplicati, quindi riprova.';

  @override
  String get importRefusalIdentityConflict =>
      'L’origine dell’importazione e il suo contenuto corrispondono a voci diverse della libreria. Controlla o rimuovi le voci in conflitto, quindi riprova.';

  @override
  String get importFailed =>
      'Non è stato possibile completare l’importazione. Sorgenti supportate: cartelle, ZIP, file *_P.pak autonomi, set .utoc/.ucas completi (.pak facoltativo), .lcache, .bank e PrecompiledScript*.Cache. Estrai prima gli archivi .7z o .rar, quindi importa la cartella. La sorgente potrebbe non essere supportata, essere danneggiata o incompleta. Il mod potrebbe essere già stato aggiunto o aggiornato; aggiorna e controlla la libreria prima di riprovare.';

  @override
  String get importPickerFailed =>
      'Non è stato possibile aprire il selettore di file o cartelle. Non è stata avviata alcuna importazione. Riprova.';

  @override
  String get importOutcomeUnknown =>
      'Non è stato possibile verificare il risultato dell’importazione. Seleziona Aggiorna per controllare la libreria.';

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
    return 'Riscontri ($count)';
  }

  @override
  String get conflictWinner => 'vincitore previsto';

  @override
  String get noConflicts => 'Nessun conflitto riconosciuto.';

  @override
  String get conflictCoverageIncomplete =>
      'Le informazioni sui conflitti dei mod attivi sono incomplete; potrebbero esistere altri conflitti.';

  @override
  String get loadOrderDirection =>
      'Ordine di caricamento: prima la priorità più bassa; i mod successivi hanno una priorità prevista maggiore.';

  @override
  String get footprintCoverageScope =>
      'La copertura descrive solo gli obiettivi di conflitto riconosciuti; non dimostra la priorità in esecuzione.';

  @override
  String get footprintCoverageExact =>
      'Esatta — l’elenco degli obiettivi di conflitto del componente è completo.';

  @override
  String get footprintCoveragePartial =>
      'Parziale — gli obiettivi elencati sono noti, ma il componente può interessarne altri.';

  @override
  String get footprintCoverageAdvisory =>
      'Indicativa — gli obiettivi elencati sono indizi, non una prova esaustiva.';

  @override
  String get footprintCoverageOpaque =>
      'Opaca — gli obiettivi di conflitto del componente sono sconosciuti.';

  @override
  String get footprintCoverageExactLabel => 'Esatta';

  @override
  String get footprintCoveragePartialLabel => 'Parziale';

  @override
  String get footprintCoverageAdvisoryLabel => 'Indicativa';

  @override
  String get footprintCoverageOpaqueLabel => 'Opaca';

  @override
  String get conflictsUnverified =>
      'I conflitti non sono verificati finché lo stato della libreria non viene aggiornato.';

  @override
  String get componentsTitle => 'Componenti';

  @override
  String targetsMore(int count) {
    return '+$count altri';
  }

  @override
  String get removeModDeploymentHint =>
      'La rimozione dalla libreria non modifica immediatamente una distribuzione esistente. Se la mod è già distribuita, seleziona Applica per aggiornare l\'installazione del gioco.';

  @override
  String removeModSuccess(String name) {
    return 'Il mod «$name» è stato rimosso dalla libreria.';
  }

  @override
  String removeModFailed(String name, String error) {
    return 'Impossibile rimuovere il mod «$name»: $error';
  }

  @override
  String removeModPartialFailure(String name, String error) {
    return 'Il mod «$name» è stato rimosso, ma l’elaborazione successiva ha segnalato un errore. Lo stato della libreria è stato ricaricato: $error';
  }

  @override
  String removeModOutcomeUnknown(String name, String error) {
    return 'Non è stato possibile verificare se il mod «$name» è stato rimosso: $error — Aggiorna per controllare lo stato della libreria.';
  }

  @override
  String get libraryStateUnknown =>
      'Non è stato possibile verificare lo stato della libreria. Seleziona Aggiorna prima di modificare o applicare mod.';

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
  String get aboutCopyright => '© 2026 collaboratori di GORE';

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
