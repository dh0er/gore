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
  String get conflictWinner => 'vincitore';

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
