// ignore: unused_import
import 'package:intl/intl.dart' as intl;
import 'app_localizations.dart';

// ignore_for_file: type=lint

/// The translations for Portuguese (`pt`).
class AppLocalizationsPt extends AppLocalizations {
  AppLocalizationsPt([String locale = 'pt']) : super(locale);

  @override
  String get coreBlockedTitle => 'Mod Manager indisponível';

  @override
  String get coreDllMissingMessage =>
      'Não foi encontrado o ficheiro gore_ffi.dll necessário.';

  @override
  String get coreDllLoadFailedMessage =>
      'Não foi possível carregar a biblioteca nativa do GORE Core.';

  @override
  String get coreVerificationFailedMessage =>
      'Não foi possível verificar a biblioteca nativa do GORE Core.';

  @override
  String get coreManagerTooOldMessage =>
      'Esta versão do GORE Core é mais recente do que o Mod Manager. Atualize o Mod Manager.';

  @override
  String get coreNativeTooOldMessage =>
      'Esta versão do GORE Core é mais antiga do que o Mod Manager. Atualize ou repare a instalação completa do Mod Manager.';

  @override
  String get coreCommandsMissingMessage =>
      'A biblioteca do GORE Core não fornece todos os comandos exigidos por este Mod Manager.';

  @override
  String get coreBlockedRepairHint =>
      'Atualize ou repare o pacote completo do Mod Manager e reinicie a aplicação.';

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
  String get preflightAttention => 'A configuração requer atenção.';

  @override
  String get preflightUnavailable =>
      'O diagnóstico da configuração não está disponível.';

  @override
  String get preflightRetry => 'Verificar novamente';

  @override
  String get preflightReviewStatus => 'Verificar estado';

  @override
  String get preflightReviewRecovery => 'Ajuda';

  @override
  String get installRecoveryTitle => 'Recuperação da instalação';

  @override
  String get installRecoveryBody =>
      'O GORE encontrou dados de recuperação associados a uma instalação ou a uma compilação de scripts. A operação associada pode ainda estar em execução, ou estes dados podem ser restos de uma operação já terminada. O GORE não pode efetuar uma reparação automática em segurança.';

  @override
  String get installRecoverySteps =>
      'Se a operação associada ainda estiver em execução, aguarde que termine. Não a interrompa nem elimine nenhum ficheiro de bloqueio. Siga o ficheiro README.txt na pasta de recuperação indicada abaixo apenas quando tiver a certeza de que nenhuma operação associada está em execução. Se não for indicada nenhuma pasta ou se tiver dúvidas, deixe os dados de recuperação inalterados e peça ajuda. Depois, verifique novamente.';

  @override
  String get installRecoveryEvidence => 'Dados de recuperação detetados';

  @override
  String get managerRecoveryTitle =>
      'Recuperar operação interrompida do Gestor';

  @override
  String get managerRecoveryConfirm =>
      'O GORE detetou uma operação do Gestor claramente interrompida. Continue apenas se pretender que o GORE verifique a operação registada e devolva a instalação a um estado conhecido. Os jogos guardados nunca são alterados.';

  @override
  String get managerRecoveryAlreadyClean =>
      'A operação interrompida já estava resolvida. A instalação foi verificada novamente.';

  @override
  String get managerRecoveryBusy =>
      'A operação está novamente ativa. Nada foi alterado; aguarde que termine e verifique novamente.';

  @override
  String get managerRecoveryLockCleared =>
      'A operação interrompida ainda não tinha alterado a instalação. O bloqueio obsoleto foi removido em segurança.';

  @override
  String get managerRecoveryRestoredPristine =>
      'A alteração interrompida foi anulada e o estado de base registado da instalação foi restaurado.';

  @override
  String get managerRecoveryApplyPreserved =>
      'A aplicação já tinha terminado. O estado registado foi preservado e o estado foi verificado novamente.';

  @override
  String get managerRecoveryUndeployConfirmed =>
      'A remoção tinha terminado. Os dados de transação restantes foram limpos e o estado foi verificado novamente.';

  @override
  String get managerRecoveryCompileRequired =>
      'Isto pertence à recuperação da compilação de scripts. O Gestor não alterou nada; consulte a ajuda de recuperação.';

  @override
  String get managerRecoveryInspectionFailed =>
      'O GORE não conseguiu verificar a operação interrompida em segurança. Nada foi alterado; consulte os detalhes de recuperação atuais.';

  @override
  String get managerRecoveryFailed =>
      'Não foi possível concluir a recuperação. O GORE tentou verificar novamente a instalação, mas o estado atual pode ser desconhecido. Consulte o estado antes de tentar outra vez.';

  @override
  String get statusUnknown => 'Desconhecido';

  @override
  String statusDetailsTitle(String status) {
    return 'Implementação: $status';
  }

  @override
  String statusDetailsOpen(String status) {
    return 'Mostrar detalhes da implementação: $status';
  }

  @override
  String get statusDetailsNoRoot =>
      'Escolha uma instalação do jogo nas Definições para consultar o estado da implementação.';

  @override
  String get statusDetailsNoDeployment =>
      'Não existe uma implementação do gestor instalada para este jogo.';

  @override
  String get statusDetailsInSyncDescription =>
      'Os mods implementados correspondem à configuração atual.';

  @override
  String get statusDetailsDeployedLoadout =>
      'Ordem de carregamento implementada';

  @override
  String get statusDetailsChangesDescription =>
      'A implementação atual difere do que Aplicar irá instalar.';

  @override
  String get statusDetailsCurrentlyDeployed => 'Atualmente implementado';

  @override
  String get statusDetailsAfterApply => 'Depois de Aplicar';

  @override
  String get statusDetailsGameUpdatedDescription =>
      'Os ficheiros do jogo mudaram após a última implementação. Volte a aplicar a configuração para restaurar os ficheiros do gestor.';

  @override
  String get statusDetailsDriftedFiles => 'Ficheiros alterados';

  @override
  String get statusDetailsStudioDescription =>
      'O Mod Studio controla atualmente esta instalação do jogo. Assuma o controlo antes de aplicar uma configuração do gestor.';

  @override
  String statusDetailsStudioMod(String name) {
    return 'Mod do Studio: $name';
  }

  @override
  String get statusDetailsStudioNameUnknown =>
      'O Studio não indicou o nome do mod.';

  @override
  String get statusDetailsRecoveryDescription =>
      'Uma implementação foi interrompida. Recupere-a antes de aplicar ou remover mods do gestor.';

  @override
  String get statusDetailsUnknownDescription =>
      'Não foi possível verificar o estado da implementação. Atualize-o antes de aplicar mods.';

  @override
  String get statusDetailsUnavailable =>
      'O núcleo instalado não forneceu estes detalhes.';

  @override
  String get statusDetailsEmptyLoadout =>
      'Não existem mods nesta configuração.';

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
  String get recoveryAction => 'Recuperar';

  @override
  String get recoveryRequiredConfirm =>
      'Recuperar a implantação interrompida e remover os arquivos parcialmente implantados?';

  @override
  String get statusRecoveryRequired => 'Recuperação necessária';

  @override
  String get statusDetailsOwnershipTitle =>
      'Evidência de propriedade registada';

  @override
  String get statusDetailsOwnershipDescription =>
      'Caminhos registados no registo de implementação do gestor. Não provam que esses caminhos ainda existam.';

  @override
  String get statusDetailsOwnershipLive => 'Ficheiros do jogo substituídos';

  @override
  String get statusDetailsOwnershipBackups => 'Cópias de segurança originais';

  @override
  String get statusDetailsOwnershipAdditive =>
      'Ficheiros pak e contentores adicionados';

  @override
  String get statusDetailsOwnershipUe4ss => 'Diretórios de mods UE4SS';

  @override
  String get statusDetailsOwnershipRecovery =>
      'Ficheiros e locais de recuperação';

  @override
  String get statusDetailsOwnershipEmpty =>
      'Nenhum caminho registado neste grupo.';

  @override
  String statusDetailsOwnershipShown(int shown, int total) {
    return 'São apresentados $shown de $total caminhos registados.';
  }

  @override
  String get appTitle => 'GORE Mod Manager';

  @override
  String get tabMods => 'Mods';

  @override
  String get tabSettings => 'Definições';

  @override
  String get settingsGameExe => 'Executável do jogo';

  @override
  String get settingsGameExePick => 'Escolher…';

  @override
  String get settingsLanguage => 'Idioma';

  @override
  String get statusInSync => 'Sincronizado';

  @override
  String get statusChangesPending => 'Alterações pendentes';

  @override
  String get statusGameUpdated => 'Jogo atualizado';

  @override
  String get statusStudioDeploy => 'Implementação do Studio ativa';

  @override
  String get statusNothingDeployed => 'Nada implementado';

  @override
  String get actionImport => 'Importar';

  @override
  String get actionApply => 'Aplicar';

  @override
  String get actionUndeployAll => 'Remover tudo';

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
    return '«$name» foi adicionado à biblioteca.';
  }

  @override
  String importOutcomeUpdated(String name) {
    return '«$name» foi atualizado na biblioteca.';
  }

  @override
  String importOutcomeUnchanged(String name) {
    return '«$name» já se encontra na biblioteca.';
  }

  @override
  String importOutcomeMatchedBy(String method) {
    String _temp0 = intl.Intl.selectLogic(method, {
      'none':
          'Não houve correspondência com nenhuma entrada existente da biblioteca.',
      'source': 'Correspondência com a mesma origem de importação.',
      'content': 'Correspondência com conteúdo idêntico verificado.',
      'entry_id': 'Correspondência com o ID do mod.',
      'other': 'Os detalhes da correspondência não estão disponíveis.',
    });
    return '$_temp0';
  }

  @override
  String get importRefusalDuplicateAmbiguous =>
      'Esta importação corresponde a mais do que uma entrada da biblioteca. Reveja ou remova as entradas duplicadas e tente novamente.';

  @override
  String get importRefusalIdentityConflict =>
      'A origem da importação e o respetivo conteúdo correspondem a entradas diferentes da biblioteca. Reveja ou remova as entradas em conflito e tente novamente.';

  @override
  String get importFailed =>
      'Não foi possível concluir a importação. Fontes suportadas: pastas, ZIP, ficheiros *_P.pak autónomos, conjuntos .utoc/.ucas completos (.pak opcional), .lcache, .bank e PrecompiledScript*.Cache. Extraia primeiro os ficheiros .7z ou .rar e depois importe a pasta. A fonte poderá não ser suportada, estar danificada ou incompleta. O mod poderá já ter sido adicionado ou atualizado; atualize e verifique a biblioteca antes de tentar novamente.';

  @override
  String get importPickerFailed =>
      'Não foi possível abrir o seletor de ficheiros ou pastas. Nenhuma importação foi iniciada. Tente novamente.';

  @override
  String get importOutcomeUnknown =>
      'Não foi possível verificar o resultado da importação. Selecione Atualizar para verificar a biblioteca.';

  @override
  String get applyTooltip => 'Aplicar a configuração ao jogo';

  @override
  String get undeployAllAction => 'Remover tudo';

  @override
  String get undeployAllConfirm =>
      'Remover do jogo tudo o que o gestor implementou?';

  @override
  String get takeOverTitle => 'Implementação do Studio ativa';

  @override
  String get takeOverBody =>
      'O mod-studio implementou um mod no jogo. Assumir o controlo para que o gestor possa aplicar esta configuração?';

  @override
  String get takeOverAction => 'Assumir';

  @override
  String get refreshAction => 'Atualizar';

  @override
  String conflictsTitle(int count) {
    return 'Resultados ($count)';
  }

  @override
  String get conflictWinner => 'vencedor previsto';

  @override
  String get noConflicts => 'Nenhum conflito reconhecido.';

  @override
  String get conflictCoverageIncomplete =>
      'O conhecimento de conflitos dos mods ativos está incompleto; podem existir outros conflitos.';

  @override
  String get loadOrderDirection =>
      'Ordem de carregamento: menor prioridade primeiro; os mods posteriores têm maior prioridade prevista.';

  @override
  String get footprintCoverageScope =>
      'A cobertura descreve apenas alvos de conflito reconhecidos; não comprova a prioridade em execução.';

  @override
  String get footprintCoverageExact =>
      'Exata — a lista de alvos de conflito do componente está completa.';

  @override
  String get footprintCoveragePartial =>
      'Parcial — os alvos listados são conhecidos, mas o componente pode afetar outros.';

  @override
  String get footprintCoverageAdvisory =>
      'Indicativa — os alvos listados são pistas, não uma prova exaustiva.';

  @override
  String get footprintCoverageOpaque =>
      'Opaca — os alvos de conflito do componente são desconhecidos.';

  @override
  String get footprintCoverageExactLabel => 'Exata';

  @override
  String get footprintCoveragePartialLabel => 'Parcial';

  @override
  String get footprintCoverageAdvisoryLabel => 'Indicativa';

  @override
  String get footprintCoverageOpaqueLabel => 'Opaca';

  @override
  String get conflictsUnverified =>
      'Os conflitos não estão verificados até o estado da biblioteca ser atualizado.';

  @override
  String get componentsTitle => 'Componentes';

  @override
  String targetsMore(int count) {
    return '+$count mais';
  }

  @override
  String get removeModDeploymentHint =>
      'Remover da biblioteca não altera imediatamente uma implantação existente. Se o mod já estiver implantado, selecione Aplicar depois para atualizar a instalação do jogo.';

  @override
  String removeModSuccess(String name) {
    return '«$name» foi removido da biblioteca.';
  }

  @override
  String removeModFailed(String name, String error) {
    return 'Não foi possível remover «$name»: $error';
  }

  @override
  String removeModPartialFailure(String name, String error) {
    return '«$name» foi removido, mas o processamento posterior comunicou um erro. O estado da biblioteca foi recarregado: $error';
  }

  @override
  String removeModOutcomeUnknown(String name, String error) {
    return 'Não foi possível verificar se «$name» foi removido: $error — Atualize para verificar o estado da biblioteca.';
  }

  @override
  String get libraryStateUnknown =>
      'Não foi possível verificar o estado da biblioteca. Selecione Atualizar antes de alterar ou aplicar mods.';

  @override
  String get removeModAction => 'Remover';

  @override
  String removeModConfirm(String name) {
    return 'Remover «$name» da biblioteca?';
  }

  @override
  String get errorSetGamePath =>
      'Defina primeiro o caminho do jogo nas Definições.';

  @override
  String applyReportApplied(int count) {
    return '$count mods aplicados.';
  }

  @override
  String get warningsTitle => 'Avisos';

  @override
  String get modDisabledHint => 'Desativado';

  @override
  String get kindGoremod => 'goremod';

  @override
  String get kindTriplet => 'triplet';

  @override
  String get kindPak => 'pak';

  @override
  String get kindUe4ss => 'UE4SS';

  @override
  String get kindRawfile => 'ficheiro em bruto';

  @override
  String get kindMixed => 'misto';

  @override
  String get sevHard => 'grave';

  @override
  String get sevSoft => 'ligeiro';

  @override
  String get sevInfo => 'info';

  @override
  String aboutVersion(String version, String sha) {
    return 'Version $version ($sha)';
  }

  @override
  String get about => 'Sobre';

  @override
  String get aboutCopyright => '© 2026 colaboradores do GORE';

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
  String get uiScale => 'Escala da interface';

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
  String get coreBlockedTitle => 'Mod Manager indisponível';

  @override
  String get coreDllMissingMessage =>
      'Não foi encontrado o arquivo gore_ffi.dll necessário.';

  @override
  String get coreDllLoadFailedMessage =>
      'Não foi possível carregar a biblioteca nativa do GORE Core.';

  @override
  String get coreVerificationFailedMessage =>
      'Não foi possível verificar a biblioteca nativa do GORE Core.';

  @override
  String get coreManagerTooOldMessage =>
      'Esta versão do GORE Core é mais recente que o Mod Manager. Atualize o Mod Manager.';

  @override
  String get coreNativeTooOldMessage =>
      'Esta versão do GORE Core é mais antiga que o Mod Manager. Atualize ou repare a instalação completa do Mod Manager.';

  @override
  String get coreCommandsMissingMessage =>
      'A biblioteca do GORE Core não fornece todos os comandos exigidos por este Mod Manager.';

  @override
  String get coreBlockedRepairHint =>
      'Atualize ou repare o pacote completo do Mod Manager e reinicie o aplicativo.';

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
  String get preflightAttention => 'A configuração requer atenção.';

  @override
  String get preflightUnavailable =>
      'O diagnóstico da configuração não está disponível.';

  @override
  String get preflightRetry => 'Verificar novamente';

  @override
  String get preflightReviewStatus => 'Verificar status';

  @override
  String get preflightReviewRecovery => 'Ajuda';

  @override
  String get installRecoveryTitle => 'Recuperação da instalação';

  @override
  String get installRecoveryBody =>
      'O GORE encontrou dados de recuperação associados a uma instalação ou a uma compilação de scripts. A operação associada talvez ainda esteja em execução, ou esses dados podem ser restos de uma operação já encerrada. O GORE não pode fazer um reparo automático com segurança.';

  @override
  String get installRecoverySteps =>
      'Se a operação associada ainda estiver em execução, espere até que termine. Não a interrompa nem exclua nenhum arquivo de bloqueio. Siga o arquivo README.txt na pasta de recuperação indicada abaixo somente quando tiver certeza de que nenhuma operação associada está em execução. Se nenhuma pasta for indicada ou se você não tiver certeza, deixe os dados de recuperação inalterados e peça ajuda. Depois, verifique novamente.';

  @override
  String get installRecoveryEvidence => 'Dados de recuperação detectados';

  @override
  String get managerRecoveryTitle =>
      'Recuperar operação interrompida do Gerenciador';

  @override
  String get managerRecoveryConfirm =>
      'O GORE detectou uma operação do Gerenciador claramente interrompida. Continue apenas se quiser que o GORE verifique a operação registrada e devolva a instalação a um estado conhecido. Os jogos salvos nunca são alterados.';

  @override
  String get managerRecoveryAlreadyClean =>
      'A operação interrompida já estava resolvida. A instalação foi verificada novamente.';

  @override
  String get managerRecoveryBusy =>
      'A operação está ativa novamente. Nada foi alterado; espere que termine e verifique outra vez.';

  @override
  String get managerRecoveryLockCleared =>
      'A operação interrompida ainda não tinha alterado a instalação. O bloqueio obsoleto foi removido com segurança.';

  @override
  String get managerRecoveryRestoredPristine =>
      'A alteração interrompida foi desfeita e o estado-base registrado da instalação foi restaurado.';

  @override
  String get managerRecoveryApplyPreserved =>
      'A aplicação já tinha terminado. O estado registrado foi preservado e o status foi verificado novamente.';

  @override
  String get managerRecoveryUndeployConfirmed =>
      'A remoção tinha terminado. Os dados de transação restantes foram limpos e o status foi verificado novamente.';

  @override
  String get managerRecoveryCompileRequired =>
      'Isso pertence à recuperação da compilação de scripts. O Gerenciador não alterou nada; consulte a ajuda de recuperação.';

  @override
  String get managerRecoveryInspectionFailed =>
      'O GORE não conseguiu verificar a operação interrompida com segurança. Nada foi alterado; consulte os detalhes de recuperação atuais.';

  @override
  String get managerRecoveryFailed =>
      'Não foi possível concluir a recuperação. O GORE tentou verificar novamente a instalação, mas o estado atual pode ser desconhecido. Confira o estado antes de tentar de novo.';

  @override
  String get statusUnknown => 'Desconhecido';

  @override
  String statusDetailsTitle(String status) {
    return 'Implantação: $status';
  }

  @override
  String statusDetailsOpen(String status) {
    return 'Mostrar detalhes da implantação: $status';
  }

  @override
  String get statusDetailsNoRoot =>
      'Escolha uma instalação do jogo nas Configurações para consultar o estado da implantação.';

  @override
  String get statusDetailsNoDeployment =>
      'Não há uma implantação do gerenciador instalada para este jogo.';

  @override
  String get statusDetailsInSyncDescription =>
      'Os mods implantados correspondem à configuração atual.';

  @override
  String get statusDetailsDeployedLoadout => 'Ordem de carregamento implantada';

  @override
  String get statusDetailsChangesDescription =>
      'A implantação atual difere do que Aplicar instalará.';

  @override
  String get statusDetailsCurrentlyDeployed => 'Implantado atualmente';

  @override
  String get statusDetailsAfterApply => 'Depois de Aplicar';

  @override
  String get statusDetailsGameUpdatedDescription =>
      'Os arquivos do jogo mudaram após a última implantação. Aplique a configuração novamente para restaurar os arquivos do gerenciador.';

  @override
  String get statusDetailsDriftedFiles => 'Arquivos alterados';

  @override
  String get statusDetailsStudioDescription =>
      'O Mod Studio controla atualmente esta instalação do jogo. Assuma o controle antes de aplicar uma configuração do gerenciador.';

  @override
  String statusDetailsStudioMod(String name) {
    return 'Mod do Studio: $name';
  }

  @override
  String get statusDetailsStudioNameUnknown =>
      'O Studio não informou o nome do mod.';

  @override
  String get statusDetailsRecoveryDescription =>
      'Uma implantação foi interrompida. Recupere-a antes de aplicar ou remover mods do gerenciador.';

  @override
  String get statusDetailsUnknownDescription =>
      'Não foi possível verificar o estado da implantação. Atualize antes de aplicar mods.';

  @override
  String get statusDetailsUnavailable =>
      'O núcleo instalado não forneceu esses detalhes.';

  @override
  String get statusDetailsEmptyLoadout => 'Não há mods nesta configuração.';

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
  String get recoveryAction => 'Recuperar';

  @override
  String get recoveryRequiredConfirm =>
      'Recuperar a implantação interrompida e remover os arquivos parcialmente implantados?';

  @override
  String get statusRecoveryRequired => 'Recuperação necessária';

  @override
  String get statusDetailsOwnershipTitle =>
      'Evidência de propriedade registrada';

  @override
  String get statusDetailsOwnershipDescription =>
      'Caminhos registrados no registro de implantação do gerenciador. Eles não comprovam que esses caminhos ainda existam.';

  @override
  String get statusDetailsOwnershipLive => 'Arquivos do jogo substituídos';

  @override
  String get statusDetailsOwnershipBackups => 'Backups originais';

  @override
  String get statusDetailsOwnershipAdditive =>
      'Arquivos pak e contêineres adicionados';

  @override
  String get statusDetailsOwnershipUe4ss => 'Diretórios de mods UE4SS';

  @override
  String get statusDetailsOwnershipRecovery =>
      'Arquivos e locais de recuperação';

  @override
  String get statusDetailsOwnershipEmpty =>
      'Nenhum caminho registrado neste grupo.';

  @override
  String statusDetailsOwnershipShown(int shown, int total) {
    return 'Exibidos $shown de $total caminhos registrados.';
  }

  @override
  String get appTitle => 'GORE Mod Manager';

  @override
  String get tabMods => 'Mods';

  @override
  String get tabSettings => 'Configurações';

  @override
  String get settingsGameExe => 'Executável do jogo';

  @override
  String get settingsGameExePick => 'Escolher…';

  @override
  String get settingsLanguage => 'Idioma';

  @override
  String get statusInSync => 'Sincronizado';

  @override
  String get statusChangesPending => 'Alterações pendentes';

  @override
  String get statusGameUpdated => 'Jogo atualizado';

  @override
  String get statusStudioDeploy => 'Implantação do Studio ativa';

  @override
  String get statusNothingDeployed => 'Nada implantado';

  @override
  String get actionImport => 'Importar';

  @override
  String get actionApply => 'Aplicar';

  @override
  String get actionUndeployAll => 'Remover tudo';

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
    return '“$name” foi adicionado à biblioteca.';
  }

  @override
  String importOutcomeUpdated(String name) {
    return '“$name” foi atualizado na biblioteca.';
  }

  @override
  String importOutcomeUnchanged(String name) {
    return '“$name” já está na biblioteca.';
  }

  @override
  String importOutcomeMatchedBy(String method) {
    String _temp0 = intl.Intl.selectLogic(method, {
      'none':
          'Não houve correspondência com nenhuma entrada existente da biblioteca.',
      'source': 'Correspondência com a mesma origem de importação.',
      'content': 'Correspondência com conteúdo idêntico verificado.',
      'entry_id': 'Correspondência com o ID do mod.',
      'other': 'Os detalhes da correspondência não estão disponíveis.',
    });
    return '$_temp0';
  }

  @override
  String get importRefusalDuplicateAmbiguous =>
      'Esta importação corresponde a mais de uma entrada da biblioteca. Revise ou remova as entradas duplicadas e tente novamente.';

  @override
  String get importRefusalIdentityConflict =>
      'A origem da importação e o conteúdo correspondem a entradas diferentes da biblioteca. Revise ou remova as entradas em conflito e tente novamente.';

  @override
  String get importFailed =>
      'Não foi possível concluir a importação. Fontes compatíveis: pastas, ZIP, arquivos *_P.pak avulsos, conjuntos .utoc/.ucas completos (.pak opcional), .lcache, .bank e PrecompiledScript*.Cache. Extraia primeiro os arquivos .7z ou .rar e depois importe a pasta. A fonte pode não ser compatível, estar corrompida ou incompleta. O mod talvez já tenha sido adicionado ou atualizado; atualize e verifique a biblioteca antes de tentar novamente.';

  @override
  String get importPickerFailed =>
      'Não foi possível abrir o seletor de arquivos ou pastas. Nenhuma importação foi iniciada. Tente novamente.';

  @override
  String get importOutcomeUnknown =>
      'Não foi possível verificar o resultado da importação. Selecione Atualizar para verificar a biblioteca.';

  @override
  String get applyTooltip => 'Aplicar a configuração ao jogo';

  @override
  String get undeployAllAction => 'Remover tudo';

  @override
  String get undeployAllConfirm =>
      'Remover do jogo tudo o que o gerenciador implantou?';

  @override
  String get takeOverTitle => 'Implantação do Studio ativa';

  @override
  String get takeOverBody =>
      'O mod-studio implantou um mod no jogo. Assumir o controle para que o gerenciador possa aplicar esta configuração?';

  @override
  String get takeOverAction => 'Assumir';

  @override
  String get refreshAction => 'Atualizar';

  @override
  String conflictsTitle(int count) {
    return 'Resultados ($count)';
  }

  @override
  String get conflictWinner => 'vencedor previsto';

  @override
  String get noConflicts => 'Nenhum conflito reconhecido.';

  @override
  String get conflictCoverageIncomplete =>
      'O conhecimento de conflitos dos mods ativos está incompleto; podem existir outros conflitos.';

  @override
  String get loadOrderDirection =>
      'Ordem de carregamento: menor prioridade primeiro; os mods posteriores têm maior prioridade pretendida.';

  @override
  String get footprintCoverageScope =>
      'A cobertura descreve apenas alvos de conflito reconhecidos; não comprova a prioridade em tempo de execução.';

  @override
  String get footprintCoverageExact =>
      'Exata — a lista de alvos de conflito do componente está completa.';

  @override
  String get footprintCoveragePartial =>
      'Parcial — os alvos listados são conhecidos, mas o componente pode afetar outros.';

  @override
  String get footprintCoverageAdvisory =>
      'Indicativa — os alvos listados são pistas, não uma prova completa.';

  @override
  String get footprintCoverageOpaque =>
      'Opaca — os alvos de conflito do componente são desconhecidos.';

  @override
  String get footprintCoverageExactLabel => 'Exata';

  @override
  String get footprintCoveragePartialLabel => 'Parcial';

  @override
  String get footprintCoverageAdvisoryLabel => 'Indicativa';

  @override
  String get footprintCoverageOpaqueLabel => 'Opaca';

  @override
  String get conflictsUnverified =>
      'Os conflitos não estão verificados até que o estado da biblioteca seja atualizado.';

  @override
  String get componentsTitle => 'Componentes';

  @override
  String targetsMore(int count) {
    return '+$count mais';
  }

  @override
  String get removeModDeploymentHint =>
      'Remover da biblioteca não altera imediatamente uma implantação existente. Se o mod já estiver implantado, selecione Aplicar depois para atualizar a instalação do jogo.';

  @override
  String removeModSuccess(String name) {
    return '“$name” foi removido da biblioteca.';
  }

  @override
  String removeModFailed(String name, String error) {
    return 'Não foi possível remover “$name”: $error';
  }

  @override
  String removeModPartialFailure(String name, String error) {
    return '“$name” foi removido, mas o processamento posterior informou um erro. O estado da biblioteca foi recarregado: $error';
  }

  @override
  String removeModOutcomeUnknown(String name, String error) {
    return 'Não foi possível verificar se “$name” foi removido: $error — Atualize para verificar o estado da biblioteca.';
  }

  @override
  String get libraryStateUnknown =>
      'Não foi possível verificar o estado da biblioteca. Selecione Atualizar antes de alterar ou aplicar mods.';

  @override
  String get removeModAction => 'Remover';

  @override
  String removeModConfirm(String name) {
    return 'Remover “$name” da biblioteca?';
  }

  @override
  String get errorSetGamePath =>
      'Defina primeiro o caminho do jogo nas Configurações.';

  @override
  String applyReportApplied(int count) {
    return '$count mods aplicados.';
  }

  @override
  String get warningsTitle => 'Avisos';

  @override
  String get modDisabledHint => 'Desativado';

  @override
  String get kindGoremod => 'goremod';

  @override
  String get kindTriplet => 'triplet';

  @override
  String get kindPak => 'pak';

  @override
  String get kindUe4ss => 'UE4SS';

  @override
  String get kindRawfile => 'arquivo bruto';

  @override
  String get kindMixed => 'misto';

  @override
  String get sevHard => 'grave';

  @override
  String get sevSoft => 'leve';

  @override
  String get sevInfo => 'info';

  @override
  String aboutVersion(String version, String sha) {
    return 'Version $version ($sha)';
  }

  @override
  String get about => 'Sobre';

  @override
  String get aboutCopyright => '© 2026 colaboradores do GORE';

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
  String get uiScale => 'Escala da interface';

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
