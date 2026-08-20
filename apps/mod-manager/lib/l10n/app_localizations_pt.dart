// ignore: unused_import
import 'package:intl/intl.dart' as intl;
import 'app_localizations.dart';

// ignore_for_file: type=lint

/// The translations for Portuguese (`pt`).
class AppLocalizationsPt extends AppLocalizations {
  AppLocalizationsPt([String locale = 'pt']) : super(locale);

  @override
  String get coreBlockedTitle => 'O Mod Manager não consegue iniciar';

  @override
  String get coreDllMissingMessage =>
      'Falta um ficheiro necessário do programa (gore_ffi.dll).';

  @override
  String get coreDllLoadFailedMessage =>
      'Não foi possível carregar um ficheiro necessário do programa.';

  @override
  String get coreVerificationFailedMessage =>
      'Não foi possível verificar um ficheiro necessário do programa.';

  @override
  String get coreManagerTooOldMessage =>
      'Os ficheiros do programa são mais recentes do que o Mod Manager. Atualiza o Mod Manager.';

  @override
  String get coreNativeTooOldMessage =>
      'Os ficheiros do programa são mais antigos do que o Mod Manager. Reinstala o Mod Manager.';

  @override
  String get coreCommandsMissingMessage =>
      'Faltam aos ficheiros do programa funções de que este Mod Manager precisa.';

  @override
  String get coreBlockedRepairHint =>
      'Reinstala ou repara o Mod Manager e inicia-o de novo.';

  @override
  String get coreTechnicalDetails => 'Detalhes técnicos';

  @override
  String get coreCopyTechnicalDetails => 'Copiar detalhes técnicos';

  @override
  String get coreTechnicalDetailsCopied => 'Detalhes técnicos copiados';

  @override
  String get coreTechnicalDetailsCopyFailed =>
      'Não foi possível copiar os detalhes técnicos. Tente novamente.';

  @override
  String get preflightAttention =>
      'Há algo a resolver antes de poderes mudar mods.';

  @override
  String get preflightGameRunning =>
      'O Gothic ainda está aberto. Feche o jogo antes de alterar os mods.';

  @override
  String get managerOperationFailed => 'A operação falhou.';

  @override
  String get libraryOperationFailed =>
      'Não foi possível carregar a lista de mods.';

  @override
  String get conflictsUnavailable => 'Não foi possível verificar os conflitos.';

  @override
  String applyReportAppliedWithWarnings(int applied, int warnings) {
    return 'Aplicados: $applied. Avisos: $warnings.';
  }

  @override
  String get modDetailKind => 'Tipo';

  @override
  String get modDetailVersion => 'Versão';

  @override
  String get modDetailAuthor => 'Autor';

  @override
  String get modDetailSource => 'Origem';

  @override
  String get modDetailImported => 'Importado';

  @override
  String get componentLocalization => 'Textos';

  @override
  String get componentAudio => 'Som';

  @override
  String get componentAngelScript => 'Scripts';

  @override
  String get componentTexture => 'Texturas';

  @override
  String get componentGameFiles => 'Ficheiros do jogo';

  @override
  String get componentVoice => 'Vozes';

  @override
  String get componentKindLocalizationPatch => 'Alterações de texto';

  @override
  String get componentKindAudioPatch => 'Alterações de som';

  @override
  String get componentKindAngelScriptPatch => 'Alterações de scripts';

  @override
  String get componentKindTexturePatch => 'Alterações de texturas';

  @override
  String get componentKindLoosePak => 'Ficheiro PAK';

  @override
  String get componentKindTriplet => 'Contentor IoStore';

  @override
  String get componentKindUe4ssLua => 'Script UE4SS';

  @override
  String get componentKindRawFile => 'Ficheiro';

  @override
  String get componentKindFilePatch => 'Ficheiro do jogo substituído';

  @override
  String get componentKindPakFilePatch =>
      'Ficheiro do jogo a partir de um PAK em ~mods';

  @override
  String get componentKindVoiceArchivePatch => 'Vozes';

  @override
  String get rawTargetGameText => 'Todos os textos do jogo';

  @override
  String get rawTargetGameScripts => 'Todos os scripts do jogo';

  @override
  String get rawTargetSoundBank => 'Banco de som';

  @override
  String rawTargetSoundBankNamed(String name) {
    return 'Banco de som: $name';
  }

  @override
  String get conflictKindLocalization => 'Textos';

  @override
  String get conflictKindAudio => 'Som';

  @override
  String get conflictKindAsset => 'Dados do jogo';

  @override
  String get conflictKindCdo => 'Valores de objetos';

  @override
  String get conflictKindUe4ssUnknown => 'UE4SS (pouco claro)';

  @override
  String get conflictKindScriptModule => 'Script do jogo';

  @override
  String get conflictKindVoiceArchive => 'Vozes';

  @override
  String get conflictKindRawFile => 'Ficheiro';

  @override
  String get conflictKindLooseFile => 'Ficheiro do jogo';

  @override
  String get preflightUnavailable =>
      'Não foi possível verificar a instalação do jogo.';

  @override
  String get preflightRetry => 'Verificar novamente';

  @override
  String get preflightReviewStatus => 'Ver estado';

  @override
  String get preflightReviewRecovery => 'Ver ajuda';

  @override
  String get installRecoveryTitle => 'Instalação interrompida';

  @override
  String get installRecoveryBody =>
      'O GORE encontrou restos de uma instalação ou de uma compilação de scripts. Esse trabalho pode ainda estar a decorrer, ou terminou e deixou isto para trás. O GORE não consegue limpar isto sozinho em segurança.';

  @override
  String get installRecoverySteps =>
      'Se o trabalho ainda estiver a decorrer, espera que termine — não o pares nem apagues ficheiros. Quando tiveres a certeza de que nada está a correr, segue o README.txt na pasta abaixo e verifica de novo. Se não houver pasta indicada ou tiveres dúvidas, deixa tudo como está e pede ajuda.';

  @override
  String get installRecoveryEvidence => 'O que o GORE encontrou';

  @override
  String get managerRecoveryTitle => 'Reparar a alteração interrompida';

  @override
  String get managerRecoveryConfirm =>
      'O GORE encontrou uma alteração interrompida e pode repor o jogo num estado conhecido. Os teus saves nunca são tocados.';

  @override
  String get managerRecoveryAlreadyClean =>
      'Não havia nada para reparar. O estado foi verificado de novo.';

  @override
  String get managerRecoveryBusy =>
      'O trabalho está a decorrer de novo. Nada foi alterado — espera que termine.';

  @override
  String get managerRecoveryLockCleared =>
      'O trabalho interrompido ainda não tinha alterado nada. Foi limpo.';

  @override
  String get managerRecoveryRestoredPristine =>
      'A alteração foi revertida. O jogo voltou ao estado anterior.';

  @override
  String get managerRecoveryApplyPreserved =>
      'A aplicação já tinha terminado. Nada se perdeu.';

  @override
  String get managerRecoveryUndeployConfirmed =>
      'A remoção já tinha terminado. Os restos foram limpos.';

  @override
  String get managerRecoveryCompileRequired =>
      'Isto pertence a uma compilação de scripts, por isso nada foi alterado. Abre a ajuda de reparação.';

  @override
  String get managerRecoveryInspectionFailed =>
      'O GORE não conseguiu verificar o trabalho interrompido em segurança. Nada foi alterado.';

  @override
  String get managerRecoveryFailed =>
      'Não foi possível concluir a reparação. Verifica o estado antes de tentares de novo.';

  @override
  String get statusUnknown => 'Desconhecido';

  @override
  String statusDetailsTitle(String status) {
    return 'Estado: $status';
  }

  @override
  String statusDetailsOpen(String status) {
    return 'Ver detalhes: $status';
  }

  @override
  String get statusDetailsNoRoot =>
      'Escolhe primeiro a tua instalação do Gothic nas definições.';

  @override
  String get statusDetailsNoDeployment =>
      'De momento não há mods instalados no jogo.';

  @override
  String get statusDetailsInSyncDescription =>
      'O jogo tem exatamente os mods que marcaste aqui.';

  @override
  String get statusDetailsDeployedLoadout => 'Mods no jogo';

  @override
  String get statusDetailsChangesDescription =>
      'A tua seleção difere do que está no jogo.';

  @override
  String get statusDetailsCurrentlyDeployed => 'Agora no jogo';

  @override
  String get statusDetailsAfterApply => 'Depois de aplicar';

  @override
  String get statusDetailsGameUpdatedDescription =>
      'O jogo foi atualizado e substituiu ficheiros de mods. Aplica de novo para os repor.';

  @override
  String get statusDetailsDriftedFiles => 'Ficheiros afetados';

  @override
  String get statusDetailsStudioDescription =>
      'O Mod Studio tem mods neste jogo. Assume o jogo antes de o Manager aplicar os teus.';

  @override
  String statusDetailsStudioMod(String name) {
    return 'Mod do Studio: $name';
  }

  @override
  String get statusDetailsStudioNameUnknown =>
      'O Mod Studio não indicou um nome.';

  @override
  String get statusDetailsRecoveryDescription =>
      'Uma alteração foi interrompida. Repara-a antes de mudar mods.';

  @override
  String get statusDetailsUnknownDescription =>
      'Não foi possível ler o estado. Atualiza primeiro.';

  @override
  String get statusDetailsUnavailable => 'Sem detalhes disponíveis.';

  @override
  String get statusDetailsEmptyLoadout => 'Sem mods.';

  @override
  String get statusDetailsLastError => 'Último erro';

  @override
  String get statusDetailsLastApply => 'Última aplicação';

  @override
  String get statusDetailsAppliedMods => 'Mods aplicados';

  @override
  String get statusDetailsWarnings => 'Avisos';

  @override
  String get statusDetailsReapply => 'Aplicar novamente';

  @override
  String get statusDetailsOpenSettings => 'Abrir Definições';

  @override
  String get recoveryAction => 'Reparar';

  @override
  String get recoveryRequiredConfirm =>
      'Reparar a alteração interrompida e remover ficheiros meio instalados?';

  @override
  String get statusRecoveryRequired => 'Reparação necessária';

  @override
  String get statusDetailsOwnershipTitle => 'Ficheiros geridos pelo GORE';

  @override
  String get statusDetailsOwnershipDescription =>
      'Registado ao aplicar os mods — não verifica se os ficheiros ainda existem.';

  @override
  String get statusDetailsOwnershipLive => 'Ficheiros do jogo substituídos';

  @override
  String get statusDetailsOwnershipBackups => 'Cópias dos originais';

  @override
  String get statusDetailsOwnershipAdditive => 'Ficheiros de mods adicionados';

  @override
  String get statusDetailsOwnershipUe4ss => 'Diretórios de mods UE4SS';

  @override
  String get statusDetailsOwnershipRecovery => 'Ficheiros de reparação';

  @override
  String get statusDetailsOwnershipEmpty => 'Nada registado aqui.';

  @override
  String statusDetailsOwnershipShown(int shown, int total) {
    return 'A mostrar $shown de $total caminhos.';
  }

  @override
  String get appTitle => 'GORE Mod Manager';

  @override
  String get tabMods => 'Mods';

  @override
  String get tabSettings => 'Definições';

  @override
  String get settingsGameExe => 'Instalação do Gothic';

  @override
  String get settingsGameExePick => 'Escolher…';

  @override
  String get settingsLanguage => 'Idioma';

  @override
  String get libraryEmptyTitle => 'Ainda sem mods';

  @override
  String get libraryEmptyBody =>
      'Importa uma pasta ou um ficheiro de mod para começar.';

  @override
  String get detailEmptyHint => 'Escolhe um mod para veres o que altera.';

  @override
  String get settingsAdvanced => 'Detalhes avançados';

  @override
  String get settingsAdvancedHint =>
      'Mostra o lado técnico: entradas afetadas, quão fiável é a verificação de conflitos e os ficheiros geridos pelo GORE.';

  @override
  String get updatesTitle => 'Atualizações';

  @override
  String get checkForUpdatesAutomatically =>
      'Procurar atualizações automaticamente';

  @override
  String get checkForUpdatesNow => 'Procurar atualizações agora';

  @override
  String get updatesPortableNotice =>
      'A versão portátil abre a página de transferência no navegador. Substitui os teus ficheiros atuais pela nova transferência.';

  @override
  String get updateCheckFailed =>
      'Não foi possível procurar atualizações. Tenta mais tarde.';

  @override
  String get updateUpToDate => 'Estás a usar a versão mais recente.';

  @override
  String get updateAvailableTitle => 'Atualização disponível';

  @override
  String updateAvailableMessage(String version, String current) {
    return 'A versão $version está disponível. Tens a $current.';
  }

  @override
  String get updateLater => 'Mais tarde';

  @override
  String get updateDownload => 'Transferir';

  @override
  String updateOpenFailed(String url) {
    return 'Não foi possível abrir a página de transferência. Podes aceder a ela em $url';
  }

  @override
  String get statusInSync => 'Atualizado';

  @override
  String get statusChangesPending => 'Não aplicado';

  @override
  String get statusGameUpdated => 'O jogo foi atualizado';

  @override
  String get statusStudioDeploy => 'Mod Studio ativo';

  @override
  String get statusNothingDeployed => 'Sem mods no jogo';

  @override
  String get actionImport => 'Importar';

  @override
  String get actionApply => 'Aplicar';

  @override
  String get actionStartGame => 'Iniciar o jogo';

  @override
  String get startGameTooltip =>
      'Iniciar o Gothic com os mods que estão agora no jogo';

  @override
  String get startGameFailed =>
      'Não foi possível iniciar o Gothic. Verifica a instalação do jogo nas definições.';

  @override
  String get commonCancel => 'Cancelar';

  @override
  String get commonOk => 'OK';

  @override
  String get importFolder => 'Importar pasta…';

  @override
  String get importFile => 'Importar ficheiro…';

  @override
  String importOutcomeCreated(String name) {
    return '«$name» adicionado.';
  }

  @override
  String importOutcomeUpdated(String name) {
    return '«$name» atualizado.';
  }

  @override
  String importOutcomeUnchanged(String name) {
    return '«$name» já está na tua lista.';
  }

  @override
  String importOutcomeMatchedBy(String method) {
    String _temp0 = intl.Intl.selectLogic(method, {
      'none': 'Nenhum mod existente correspondeu.',
      'source': 'Correspondência pela mesma origem de importação.',
      'content': 'Correspondência por conteúdo idêntico verificado.',
      'entry_id': 'Correspondência pelo ID do mod.',
      'other': 'Detalhes da correspondência indisponíveis.',
    });
    return '$_temp0';
  }

  @override
  String get importRefusalDuplicateAmbiguous =>
      'Isto corresponde a mais do que um mod que já tens. Remove os duplicados e tenta de novo.';

  @override
  String get importRefusalIdentityConflict =>
      'A origem e o conteúdo correspondem a dois mods diferentes que já tens. Resolve isso e tenta de novo.';

  @override
  String get importFailed =>
      'Não foi possível importar isto. São suportados pastas, arquivos ZIP e ficheiros de mod avulsos (*_P.pak, .utoc/.ucas, .lcache, .bank, PrecompiledScript*.Cache). Extrai primeiro os .7z ou .rar e depois importa a pasta. O mod pode ter sido adicionado ou atualizado mesmo assim — atualiza a lista antes de tentares de novo.';

  @override
  String get importPickerFailed =>
      'Não foi possível abrir o seletor de ficheiros. Nada foi importado.';

  @override
  String get importOutcomeUnknown =>
      'O resultado não é claro. Atualiza para verificar a tua lista de mods.';

  @override
  String get applyTooltip => 'Instalar no jogo os mods marcados';

  @override
  String get undeployAllAction => 'Remover tudo do jogo';

  @override
  String get undeployAllConfirm =>
      'Remover do jogo todos os mods instalados pelo Manager?';

  @override
  String get takeOverTitle => 'O Mod Studio está ativo';

  @override
  String get takeOverBody =>
      'O Mod Studio tem um mod no jogo. Assumir para o Manager aplicar a tua seleção?';

  @override
  String get takeOverAction => 'Assumir';

  @override
  String get refreshAction => 'Atualizar';

  @override
  String conflictsTitle(int count) {
    return 'Conflitos ($count)';
  }

  @override
  String get conflictWinner => 'prevalece';

  @override
  String get noConflicts => 'Não foram encontrados conflitos.';

  @override
  String get conflictCoverageIncomplete =>
      'Alguns mods não podem ser verificados por completo, por isso pode haver mais conflitos.';

  @override
  String get loadOrderDirection =>
      'Os mods mais abaixo na lista substituem os de cima.';

  @override
  String get footprintCoverageScope =>
      'Só são listados os alvos de conflito conhecidos. Não garante o que acontece no jogo.';

  @override
  String get footprintTargetsExact => 'Entradas afetadas — a lista completa:';

  @override
  String get footprintTargetsPartial => 'Entradas afetadas — pode haver mais:';

  @override
  String get footprintTargetsAdvisory =>
      'Entradas provavelmente afetadas — indícios, não prova:';

  @override
  String get footprintTargetsOpaque =>
      'O GORE não consegue saber o que isto altera.';

  @override
  String get conflictsUnverified =>
      'Conflitos desconhecidos — atualiza primeiro.';

  @override
  String get componentsTitle => 'O que este mod altera';

  @override
  String targetsMore(int count) {
    return '+$count mais';
  }

  @override
  String get removeModDeploymentHint =>
      'Isto só o remove da tua lista. Se estiver instalado no jogo, escolhe Aplicar depois.';

  @override
  String removeModSuccess(String name) {
    return '«$name» removido.';
  }

  @override
  String removeModFailed(String name) {
    return 'Não foi possível remover «$name».';
  }

  @override
  String removeModPartialFailure(String name) {
    return '«$name» removido, mas a lista não pôde ser totalmente atualizada.';
  }

  @override
  String removeModOutcomeUnknown(String name) {
    return 'Não foi possível confirmar se «$name» foi removido.';
  }

  @override
  String get libraryStateUnknown =>
      'A lista de mods não está atualizada. Atualiza antes de mudar ou aplicar mods.';

  @override
  String get removeModAction => 'Remover';

  @override
  String removeModConfirm(String name) {
    return 'Remover «$name» da tua lista?';
  }

  @override
  String get errorSetGamePath =>
      'Escolhe primeiro a tua instalação do Gothic nas definições.';

  @override
  String applyReportApplied(int count) {
    return '$count mods aplicados.';
  }

  @override
  String get modDisabledHint => 'Desativado';

  @override
  String get kindGoremod => 'Pacote GORE';

  @override
  String get kindTriplet => 'Mod IoStore';

  @override
  String get kindPak => 'Mod PAK';

  @override
  String get kindUe4ss => 'UE4SS';

  @override
  String get kindRawfile => 'Substituição de ficheiros inteiros';

  @override
  String get kindMixed => 'Misto';

  @override
  String get sevHard => 'Conflito';

  @override
  String get sevSoft => 'Aviso';

  @override
  String get sevInfo => 'Nota';

  @override
  String aboutVersion(String version, String sha) {
    return 'Version $version ($sha)';
  }

  @override
  String get about => 'Sobre';

  @override
  String get aboutCopyright => '© 2026 Daniel Hoer';

  @override
  String get aboutLicense => 'Licenciado sob a Licença MIT.';

  @override
  String get appearanceTitle => 'Aparência';

  @override
  String get theme => 'Tema';

  @override
  String get themeLight => 'Claro';

  @override
  String get themeDark => 'Escuro';

  @override
  String get themeSystem => 'Sistema';

  @override
  String get uiScale => 'Tamanho de exibição';

  @override
  String get resetZoomTooltip => 'Redefinir zoom (Ctrl+0)';

  @override
  String get zoomTip =>
      'Dica: Ctrl + / Ctrl - altera o zoom em qualquer parte do app.';

  @override
  String get lightMode => 'Modo claro';

  @override
  String get darkMode => 'Modo escuro';

  @override
  String get minimize => 'Minimizar';

  @override
  String get restore => 'Restaurar';

  @override
  String get maximize => 'Maximizar';

  @override
  String get close => 'Fechar';
}

/// The translations for Portuguese, as used in Brazil (`pt_BR`).
class AppLocalizationsPtBr extends AppLocalizationsPt {
  AppLocalizationsPtBr() : super('pt_BR');

  @override
  String get coreBlockedTitle => 'O Mod Manager não consegue iniciar';

  @override
  String get coreDllMissingMessage =>
      'Falta um arquivo necessário do programa (gore_ffi.dll).';

  @override
  String get coreDllLoadFailedMessage =>
      'Não foi possível carregar um arquivo necessário do programa.';

  @override
  String get coreVerificationFailedMessage =>
      'Não foi possível verificar um arquivo necessário do programa.';

  @override
  String get coreManagerTooOldMessage =>
      'Os arquivos do programa são mais novos que o Mod Manager. Atualize o Mod Manager.';

  @override
  String get coreNativeTooOldMessage =>
      'Os arquivos do programa são mais antigos que o Mod Manager. Reinstale o Mod Manager.';

  @override
  String get coreCommandsMissingMessage =>
      'Faltam aos arquivos do programa recursos de que este Mod Manager precisa.';

  @override
  String get coreBlockedRepairHint =>
      'Reinstale ou repare o Mod Manager e inicie-o de novo.';

  @override
  String get coreTechnicalDetails => 'Detalhes técnicos';

  @override
  String get coreCopyTechnicalDetails => 'Copiar detalhes técnicos';

  @override
  String get coreTechnicalDetailsCopied => 'Detalhes técnicos copiados';

  @override
  String get coreTechnicalDetailsCopyFailed =>
      'Não foi possível copiar os detalhes técnicos. Tente novamente.';

  @override
  String get preflightAttention =>
      'Há algo a resolver antes de você mudar mods.';

  @override
  String get preflightGameRunning =>
      'O Gothic ainda está aberto. Feche o jogo antes de alterar os mods.';

  @override
  String get managerOperationFailed => 'A operação falhou.';

  @override
  String get libraryOperationFailed =>
      'Não foi possível carregar a lista de mods.';

  @override
  String get conflictsUnavailable => 'Não foi possível verificar os conflitos.';

  @override
  String applyReportAppliedWithWarnings(int applied, int warnings) {
    return 'Aplicados: $applied. Avisos: $warnings.';
  }

  @override
  String get modDetailKind => 'Tipo';

  @override
  String get modDetailVersion => 'Versão';

  @override
  String get modDetailAuthor => 'Autor';

  @override
  String get modDetailSource => 'Origem';

  @override
  String get modDetailImported => 'Importado';

  @override
  String get componentLocalization => 'Textos';

  @override
  String get componentAudio => 'Som';

  @override
  String get componentAngelScript => 'Scripts';

  @override
  String get componentTexture => 'Texturas';

  @override
  String get componentGameFiles => 'Arquivos do jogo';

  @override
  String get componentVoice => 'Vozes';

  @override
  String get componentKindLocalizationPatch => 'Alterações de texto';

  @override
  String get componentKindAudioPatch => 'Alterações de som';

  @override
  String get componentKindAngelScriptPatch => 'Alterações de scripts';

  @override
  String get componentKindTexturePatch => 'Alterações de texturas';

  @override
  String get componentKindLoosePak => 'Arquivo PAK';

  @override
  String get componentKindTriplet => 'Contêiner IoStore';

  @override
  String get componentKindUe4ssLua => 'Script UE4SS';

  @override
  String get componentKindRawFile => 'Arquivo';

  @override
  String get componentKindFilePatch => 'Arquivo do jogo substituído';

  @override
  String get componentKindPakFilePatch =>
      'Arquivo do jogo a partir de um PAK em ~mods';

  @override
  String get componentKindVoiceArchivePatch => 'Vozes';

  @override
  String get rawTargetGameText => 'Todos os textos do jogo';

  @override
  String get rawTargetGameScripts => 'Todos os scripts do jogo';

  @override
  String get rawTargetSoundBank => 'Banco de som';

  @override
  String rawTargetSoundBankNamed(String name) {
    return 'Banco de som: $name';
  }

  @override
  String get conflictKindLocalization => 'Textos';

  @override
  String get conflictKindAudio => 'Som';

  @override
  String get conflictKindAsset => 'Dados do jogo';

  @override
  String get conflictKindCdo => 'Valores de objetos';

  @override
  String get conflictKindUe4ssUnknown => 'UE4SS (pouco claro)';

  @override
  String get conflictKindScriptModule => 'Script do jogo';

  @override
  String get conflictKindVoiceArchive => 'Vozes';

  @override
  String get conflictKindRawFile => 'Arquivo';

  @override
  String get conflictKindLooseFile => 'Arquivo do jogo';

  @override
  String get preflightUnavailable =>
      'Não foi possível verificar a instalação do jogo.';

  @override
  String get preflightRetry => 'Verificar novamente';

  @override
  String get preflightReviewStatus => 'Ver estado';

  @override
  String get preflightReviewRecovery => 'Ver ajuda';

  @override
  String get installRecoveryTitle => 'Instalação interrompida';

  @override
  String get installRecoveryBody =>
      'O GORE encontrou restos de uma instalação ou de uma compilação de scripts. Esse trabalho pode ainda estar em andamento, ou terminou e deixou isso para trás. O GORE não consegue limpar isso sozinho com segurança.';

  @override
  String get installRecoverySteps =>
      'Se o trabalho ainda estiver em andamento, espere terminar — não o interrompa nem apague arquivos. Quando tiver certeza de que nada está rodando, siga o README.txt na pasta abaixo e verifique de novo. Se nenhuma pasta for indicada ou você estiver em dúvida, deixe tudo como está e peça ajuda.';

  @override
  String get installRecoveryEvidence => 'O que o GORE encontrou';

  @override
  String get managerRecoveryTitle => 'Reparar a alteração interrompida';

  @override
  String get managerRecoveryConfirm =>
      'O GORE encontrou uma alteração interrompida e pode devolver o jogo a um estado conhecido. Seus saves nunca são tocados.';

  @override
  String get managerRecoveryAlreadyClean =>
      'Não havia nada para reparar. O estado foi verificado de novo.';

  @override
  String get managerRecoveryBusy =>
      'O trabalho está em andamento de novo. Nada foi alterado — espere terminar.';

  @override
  String get managerRecoveryLockCleared =>
      'O trabalho interrompido ainda não tinha alterado nada. Foi limpo.';

  @override
  String get managerRecoveryRestoredPristine =>
      'A alteração foi revertida. O jogo voltou ao estado anterior.';

  @override
  String get managerRecoveryApplyPreserved =>
      'A aplicação já tinha terminado. Nada se perdeu.';

  @override
  String get managerRecoveryUndeployConfirmed =>
      'A remoção já tinha terminado. Os restos foram limpos.';

  @override
  String get managerRecoveryCompileRequired =>
      'Isso pertence a uma compilação de scripts, então nada foi alterado. Abra a ajuda de reparo.';

  @override
  String get managerRecoveryInspectionFailed =>
      'O GORE não conseguiu verificar o trabalho interrompido com segurança. Nada foi alterado.';

  @override
  String get managerRecoveryFailed =>
      'Não foi possível concluir o reparo. Verifique o estado antes de tentar de novo.';

  @override
  String get statusUnknown => 'Desconhecido';

  @override
  String statusDetailsTitle(String status) {
    return 'Estado: $status';
  }

  @override
  String statusDetailsOpen(String status) {
    return 'Ver detalhes: $status';
  }

  @override
  String get statusDetailsNoRoot =>
      'Escolha primeiro sua instalação do Gothic nas configurações.';

  @override
  String get statusDetailsNoDeployment =>
      'No momento não há mods instalados no jogo.';

  @override
  String get statusDetailsInSyncDescription =>
      'O jogo tem exatamente os mods que você marcou aqui.';

  @override
  String get statusDetailsDeployedLoadout => 'Mods no jogo';

  @override
  String get statusDetailsChangesDescription =>
      'Sua seleção difere do que está no jogo.';

  @override
  String get statusDetailsCurrentlyDeployed => 'Agora no jogo';

  @override
  String get statusDetailsAfterApply => 'Depois de aplicar';

  @override
  String get statusDetailsGameUpdatedDescription =>
      'O jogo foi atualizado e substituiu arquivos de mods. Aplique de novo para restaurá-los.';

  @override
  String get statusDetailsDriftedFiles => 'Arquivos afetados';

  @override
  String get statusDetailsStudioDescription =>
      'O Mod Studio tem mods neste jogo. Assuma o jogo antes de o Manager aplicar os seus.';

  @override
  String statusDetailsStudioMod(String name) {
    return 'Mod do Studio: $name';
  }

  @override
  String get statusDetailsStudioNameUnknown =>
      'O Mod Studio não informou um nome.';

  @override
  String get statusDetailsRecoveryDescription =>
      'Uma alteração foi interrompida. Repare-a antes de mudar mods.';

  @override
  String get statusDetailsUnknownDescription =>
      'Não foi possível ler o estado. Atualize primeiro.';

  @override
  String get statusDetailsUnavailable => 'Sem detalhes disponíveis.';

  @override
  String get statusDetailsEmptyLoadout => 'Sem mods.';

  @override
  String get statusDetailsLastError => 'Último erro';

  @override
  String get statusDetailsLastApply => 'Última aplicação';

  @override
  String get statusDetailsAppliedMods => 'Mods aplicados';

  @override
  String get statusDetailsWarnings => 'Avisos';

  @override
  String get statusDetailsReapply => 'Aplicar novamente';

  @override
  String get statusDetailsOpenSettings => 'Abrir Configurações';

  @override
  String get recoveryAction => 'Reparar';

  @override
  String get recoveryRequiredConfirm =>
      'Reparar a alteração interrompida e remover arquivos pela metade?';

  @override
  String get statusRecoveryRequired => 'Reparo necessário';

  @override
  String get statusDetailsOwnershipTitle => 'Arquivos gerenciados pelo GORE';

  @override
  String get statusDetailsOwnershipDescription =>
      'Registrado ao aplicar os mods — não verifica se os arquivos ainda existem.';

  @override
  String get statusDetailsOwnershipLive => 'Arquivos do jogo substituídos';

  @override
  String get statusDetailsOwnershipBackups => 'Cópias dos originais';

  @override
  String get statusDetailsOwnershipAdditive => 'Arquivos de mods adicionados';

  @override
  String get statusDetailsOwnershipUe4ss => 'Diretórios de mods UE4SS';

  @override
  String get statusDetailsOwnershipRecovery => 'Arquivos de reparo';

  @override
  String get statusDetailsOwnershipEmpty => 'Nada registrado aqui.';

  @override
  String statusDetailsOwnershipShown(int shown, int total) {
    return 'Mostrando $shown de $total caminhos.';
  }

  @override
  String get appTitle => 'GORE Mod Manager';

  @override
  String get tabMods => 'Mods';

  @override
  String get tabSettings => 'Configurações';

  @override
  String get settingsGameExe => 'Instalação do Gothic';

  @override
  String get settingsGameExePick => 'Escolher…';

  @override
  String get settingsLanguage => 'Idioma';

  @override
  String get libraryEmptyTitle => 'Ainda sem mods';

  @override
  String get libraryEmptyBody =>
      'Importe uma pasta ou um arquivo de mod para começar.';

  @override
  String get detailEmptyHint => 'Escolha um mod para ver o que ele altera.';

  @override
  String get settingsAdvanced => 'Detalhes avançados';

  @override
  String get settingsAdvancedHint =>
      'Mostra o lado técnico: entradas afetadas, quão confiável é a verificação de conflitos e os arquivos gerenciados pelo GORE.';

  @override
  String get updatesTitle => 'Atualizações';

  @override
  String get checkForUpdatesAutomatically =>
      'Procurar atualizações automaticamente';

  @override
  String get checkForUpdatesNow => 'Procurar atualizações agora';

  @override
  String get updatesPortableNotice =>
      'A versão portátil abre a página de download no navegador. Substitua seus arquivos atuais pelo novo download.';

  @override
  String get updateCheckFailed =>
      'Não foi possível procurar atualizações. Tente mais tarde.';

  @override
  String get updateUpToDate => 'Você está usando a versão mais recente.';

  @override
  String get updateAvailableTitle => 'Atualização disponível';

  @override
  String updateAvailableMessage(String version, String current) {
    return 'A versão $version está disponível. Você tem a $current.';
  }

  @override
  String get updateLater => 'Mais tarde';

  @override
  String get updateDownload => 'Baixar';

  @override
  String updateOpenFailed(String url) {
    return 'Não foi possível abrir a página de download. Você pode acessá-la em $url';
  }

  @override
  String get statusInSync => 'Atualizado';

  @override
  String get statusChangesPending => 'Não aplicado';

  @override
  String get statusGameUpdated => 'O jogo foi atualizado';

  @override
  String get statusStudioDeploy => 'Mod Studio ativo';

  @override
  String get statusNothingDeployed => 'Sem mods no jogo';

  @override
  String get actionImport => 'Importar';

  @override
  String get actionApply => 'Aplicar';

  @override
  String get actionStartGame => 'Iniciar o jogo';

  @override
  String get startGameTooltip =>
      'Iniciar o Gothic com os mods que estão agora no jogo';

  @override
  String get startGameFailed =>
      'Não foi possível iniciar o Gothic. Verifique a instalação do jogo nas configurações.';

  @override
  String get commonCancel => 'Cancelar';

  @override
  String get commonOk => 'OK';

  @override
  String get importFolder => 'Importar pasta…';

  @override
  String get importFile => 'Importar arquivo…';

  @override
  String importOutcomeCreated(String name) {
    return '“$name” adicionado.';
  }

  @override
  String importOutcomeUpdated(String name) {
    return '“$name” atualizado.';
  }

  @override
  String importOutcomeUnchanged(String name) {
    return '“$name” já está na sua lista.';
  }

  @override
  String importOutcomeMatchedBy(String method) {
    String _temp0 = intl.Intl.selectLogic(method, {
      'none': 'Nenhum mod existente correspondeu.',
      'source': 'Correspondência pela mesma origem de importação.',
      'content': 'Correspondência por conteúdo idêntico verificado.',
      'entry_id': 'Correspondência pelo ID do mod.',
      'other': 'Detalhes da correspondência indisponíveis.',
    });
    return '$_temp0';
  }

  @override
  String get importRefusalDuplicateAmbiguous =>
      'Isso corresponde a mais de um mod que você já tem. Remova os duplicados e tente de novo.';

  @override
  String get importRefusalIdentityConflict =>
      'A origem e o conteúdo correspondem a dois mods diferentes que você já tem. Resolva isso e tente de novo.';

  @override
  String get importFailed =>
      'Não foi possível importar isso. São suportados pastas, arquivos ZIP e arquivos de mod avulsos (*_P.pak, .utoc/.ucas, .lcache, .bank, PrecompiledScript*.Cache). Extraia primeiro os .7z ou .rar e depois importe a pasta. O mod pode ter sido adicionado ou atualizado mesmo assim — atualize a lista antes de tentar de novo.';

  @override
  String get importPickerFailed =>
      'Não foi possível abrir o seletor de arquivos. Nada foi importado.';

  @override
  String get importOutcomeUnknown =>
      'O resultado não é claro. Atualize para verificar sua lista de mods.';

  @override
  String get applyTooltip => 'Instalar no jogo os mods marcados';

  @override
  String get undeployAllAction => 'Remover tudo do jogo';

  @override
  String get undeployAllConfirm =>
      'Remover do jogo todos os mods instalados pelo Manager?';

  @override
  String get takeOverTitle => 'O Mod Studio está ativo';

  @override
  String get takeOverBody =>
      'O Mod Studio tem um mod no jogo. Assumir para o Manager aplicar sua seleção?';

  @override
  String get takeOverAction => 'Assumir';

  @override
  String get refreshAction => 'Atualizar';

  @override
  String conflictsTitle(int count) {
    return 'Conflitos ($count)';
  }

  @override
  String get conflictWinner => 'prevalece';

  @override
  String get noConflicts => 'Nenhum conflito encontrado.';

  @override
  String get conflictCoverageIncomplete =>
      'Alguns mods não podem ser verificados por completo, então pode haver mais conflitos.';

  @override
  String get loadOrderDirection =>
      'Os mods mais abaixo na lista substituem os de cima.';

  @override
  String get footprintCoverageScope =>
      'Só são listados os alvos de conflito conhecidos. Não garante o que acontece no jogo.';

  @override
  String get footprintTargetsExact => 'Entradas afetadas — a lista completa:';

  @override
  String get footprintTargetsPartial => 'Entradas afetadas — pode haver mais:';

  @override
  String get footprintTargetsAdvisory =>
      'Entradas provavelmente afetadas — indícios, não prova:';

  @override
  String get footprintTargetsOpaque =>
      'O GORE não consegue saber o que isso altera.';

  @override
  String get conflictsUnverified =>
      'Conflitos desconhecidos — atualize primeiro.';

  @override
  String get componentsTitle => 'O que este mod altera';

  @override
  String targetsMore(int count) {
    return '+$count mais';
  }

  @override
  String get removeModDeploymentHint =>
      'Isso só o remove da sua lista. Se estiver instalado no jogo, escolha Aplicar depois.';

  @override
  String removeModSuccess(String name) {
    return '“$name” removido.';
  }

  @override
  String removeModFailed(String name) {
    return 'Não foi possível remover “$name”.';
  }

  @override
  String removeModPartialFailure(String name) {
    return '“$name” removido, mas a lista não pôde ser totalmente atualizada.';
  }

  @override
  String removeModOutcomeUnknown(String name) {
    return 'Não foi possível confirmar se “$name” foi removido.';
  }

  @override
  String get libraryStateUnknown =>
      'A lista de mods não está atualizada. Atualize antes de mudar ou aplicar mods.';

  @override
  String get removeModAction => 'Remover';

  @override
  String removeModConfirm(String name) {
    return 'Remover “$name” da sua lista?';
  }

  @override
  String get errorSetGamePath =>
      'Escolha primeiro sua instalação do Gothic nas configurações.';

  @override
  String applyReportApplied(int count) {
    return '$count mods aplicados.';
  }

  @override
  String get modDisabledHint => 'Desativado';

  @override
  String get kindGoremod => 'Pacote GORE';

  @override
  String get kindTriplet => 'Mod IoStore';

  @override
  String get kindPak => 'Mod PAK';

  @override
  String get kindUe4ss => 'UE4SS';

  @override
  String get kindRawfile => 'Substituição de arquivos inteiros';

  @override
  String get kindMixed => 'Misto';

  @override
  String get sevHard => 'Conflito';

  @override
  String get sevSoft => 'Aviso';

  @override
  String get sevInfo => 'Nota';

  @override
  String aboutVersion(String version, String sha) {
    return 'Version $version ($sha)';
  }

  @override
  String get about => 'Sobre';

  @override
  String get aboutCopyright => '© 2026 Daniel Hoer';

  @override
  String get aboutLicense => 'Licenciado sob a Licença MIT.';

  @override
  String get appearanceTitle => 'Aparência';

  @override
  String get theme => 'Tema';

  @override
  String get themeLight => 'Claro';

  @override
  String get themeDark => 'Escuro';

  @override
  String get themeSystem => 'Sistema';

  @override
  String get uiScale => 'Tamanho de exibição';

  @override
  String get resetZoomTooltip => 'Redefinir zoom (Ctrl+0)';

  @override
  String get zoomTip =>
      'Dica: Ctrl + / Ctrl - altera o zoom em qualquer parte do app.';

  @override
  String get lightMode => 'Modo claro';

  @override
  String get darkMode => 'Modo escuro';

  @override
  String get minimize => 'Minimizar';

  @override
  String get restore => 'Restaurar';

  @override
  String get maximize => 'Maximizar';

  @override
  String get close => 'Fechar';
}
