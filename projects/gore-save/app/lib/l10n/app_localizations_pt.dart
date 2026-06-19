// ignore: unused_import
import 'package:intl/intl.dart' as intl;
import 'app_localizations.dart';

// ignore_for_file: type=lint

/// The translations for Portuguese (`pt`).
class AppLocalizationsPt extends AppLocalizations {
  AppLocalizationsPt([String locale = 'pt']) : super(locale);

  @override
  String get appTitle => 'Editor de Saves do Gothic Remake';

  @override
  String get appLogoSemanticLabel => 'logotipo do goresave';

  @override
  String get zoomTooltip => 'Pressione Ctrl +/- para ampliar/reduzir';

  @override
  String get switchToLightMode => 'Mudar para o modo claro';

  @override
  String get switchToDarkMode => 'Mudar para o modo escuro';

  @override
  String get about => 'Sobre';

  @override
  String get tabOverview => 'Visão geral';

  @override
  String get tabPlayer => 'Jogador';

  @override
  String get tabInventory => 'Inventário';

  @override
  String get tabProgression => 'Progresso';

  @override
  String get tabAllData => 'Todos os dados';

  @override
  String get tabBackups => 'Backups';

  @override
  String get tabSettings => 'Configurações';

  @override
  String get reset => 'Redefinir';

  @override
  String get save => 'Salvar';

  @override
  String saveWithCount(int count) {
    return 'Salvar ($count)';
  }

  @override
  String get ok => 'OK';

  @override
  String get cancel => 'Cancelar';

  @override
  String get confirm => 'Confirmar';

  @override
  String get close => 'Fechar';

  @override
  String get add => 'Adicionar';

  @override
  String get browse => 'Procurar';

  @override
  String get noSavFilesFound => 'Nenhum arquivo .sav encontrado';

  @override
  String get profile => 'Perfil';

  @override
  String profileWithSaves(String name, int count) {
    return '$name ($count saves)';
  }

  @override
  String get switchProfile => 'Trocar de perfil';

  @override
  String get rescanSaveFolder => 'Reverificar a pasta de saves';

  @override
  String get discardUnsavedChangesTitle =>
      'Descartar as alterações não salvas?';

  @override
  String rescanDiscardBody(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: 'alterações não salvas',
      one: 'alteração não salva',
    );
    return 'A reverificação recarrega todos os saves e descarta suas $count $_temp0.';
  }

  @override
  String get discardAndRescan => 'Descartar e reverificar';

  @override
  String chapterLabel(Object id) {
    return 'Capítulo $id';
  }

  @override
  String get quickSave => 'Save rápido';

  @override
  String get autoSave => 'Save automático';

  @override
  String get manualSave => 'Save manual';

  @override
  String get errorTitle => 'Erro';

  @override
  String get selectASaveTitle => 'Selecione um save';

  @override
  String get selectASaveBody => 'Os detalhes do save aparecerão aqui.';

  @override
  String get diagnosticsTitle => 'Diagnóstico e detalhes';

  @override
  String get diagnosticsSubtitle => 'Inspeção de formato somente leitura';

  @override
  String get metricFormat => 'Formato';

  @override
  String get metricSlot => 'Slot';

  @override
  String get metricChapter => 'Capítulo';

  @override
  String get metricTimePlayed => 'Tempo de jogo';

  @override
  String get metricSaveKind => 'Tipo de save';

  @override
  String get metricFileSize => 'Tamanho do arquivo';

  @override
  String get metricCompression => 'Compressão';

  @override
  String get metricChunks => 'Blocos';

  @override
  String get metricUncompressed => 'Não comprimido';

  @override
  String get metricPrivate => 'Privado';

  @override
  String get metricSlotName => 'Nome do slot';

  @override
  String get metricTrailer => 'Trailer';

  @override
  String get metricDecodedPrivate => 'Privado decodificado';

  @override
  String get metricPrivateStrings => 'Cadeias privadas';

  @override
  String get metricSha1 => 'SHA-1';

  @override
  String bytesValue(String count) {
    return '$count bytes';
  }

  @override
  String get inspectionJsonTitle => 'JSON de inspeção';

  @override
  String get inspectionJsonSubtitle => 'Dados brutos de inspeção do save';

  @override
  String get copy => 'Copiar';

  @override
  String get savegameFallbackTitle => 'Save';

  @override
  String screenshotForSlot(String slot) {
    return 'Captura de tela do $slot';
  }

  @override
  String get publicSaveName => 'Nome público do save';

  @override
  String get required => 'Obrigatório';

  @override
  String get playerLockedBody =>
      'Edições privadas do jogador exigem um codec capaz de comprimir.';

  @override
  String get heroTransform => 'Transformação do herói';

  @override
  String get locationX => 'Posição X';

  @override
  String get locationY => 'Posição Y';

  @override
  String get locationZ => 'Posição Z';

  @override
  String get rotationPitch => 'Rotação (pitch)';

  @override
  String get rotationYaw => 'Rotação (yaw)';

  @override
  String get rotationRoll => 'Rotação (roll)';

  @override
  String get invalid => 'Inválido';

  @override
  String get heroAttributes => 'Atributos do herói';

  @override
  String attributeBase(String name) {
    return '$name base';
  }

  @override
  String attributeCurrent(String name) {
    return '$name atual';
  }

  @override
  String get inventoryTitle => 'Inventário';

  @override
  String get inventoryNeedsDecoded =>
      'A edição do inventário exige os dados privados decodificados pelo codec.';

  @override
  String get inventoryNoStacks =>
      'Nenhuma pilha de itens encontrada nos dados privados decodificados.';

  @override
  String get resetInventoryChanges => 'Redefinir alterações do inventário';

  @override
  String get addItemTooltipPendingAdd =>
      'Salve primeiro as alterações pendentes — um novo item por vez ao salvar';

  @override
  String get addItemTooltipPendingRemove =>
      'Salve primeiro a remoção pendente — uma alteração estrutural por vez ao salvar';

  @override
  String get addItemTooltipPendingCount =>
      'Salve ou redefina primeiro as alterações de quantidade pendentes — uma edição estrutural precisa ser salva sozinha';

  @override
  String get addItemTooltipDefault => 'Adicionar item ao inventário';

  @override
  String get addItemButton => 'Adicionar item';

  @override
  String pendingAddSubtitle(int count) {
    return '×$count — adição pendente (ainda não salva)';
  }

  @override
  String get cancelPendingAdd => 'Cancelar adição pendente';

  @override
  String get pendingRemovalSubtitle => 'remoção pendente (ainda não salva)';

  @override
  String get cancelPendingRemoval => 'Cancelar remoção pendente';

  @override
  String get filterItems => 'Filtrar itens';

  @override
  String noItemsMatchQuery(String query) {
    return 'Nenhum item corresponde a \"$query\".';
  }

  @override
  String get pendingRemovalHidesAll =>
      'A remoção pendente oculta todos os itens — salve para aplicá-la.';

  @override
  String categoryWithCount(String label, int count) {
    return '$label ($count)';
  }

  @override
  String get count => 'Quantidade';

  @override
  String get min1 => 'Mín. 1';

  @override
  String countTimes(String count) {
    return '×$count';
  }

  @override
  String get deleteEquippedTooltip =>
      'Não é possível excluir: este item provavelmente está equipado ou atribuído a um slot de atalho';

  @override
  String get removeBlockedTooltip =>
      'Salve ou redefina primeiro as alterações pendentes do inventário — uma adição ou remoção precisa ser salva sozinha';

  @override
  String get removeItemFromInventory => 'Remover item do inventário';

  @override
  String get progressionLockedBody =>
      'Os dados de progresso exigem os dados privados decodificados pelo codec.';

  @override
  String get progressionNeedsTyped =>
      'Os dados estruturados de progresso exigem um save totalmente decodificado com análise tipada verificada.';

  @override
  String get sectionQuests => 'Missões';

  @override
  String get sectionKnowledge => 'Conhecimento';

  @override
  String get sectionEvents => 'Eventos';

  @override
  String get firstPage => 'Primeira página';

  @override
  String get previousPage => 'Página anterior';

  @override
  String get nextPage => 'Próxima página';

  @override
  String get lastPage => 'Última página';

  @override
  String pageOfPages(int page, int total) {
    return 'Página $page / $total';
  }

  @override
  String rangeOfTotal(int first, int last, int total) {
    return '$first–$last de $total';
  }

  @override
  String get perPage => 'Por página:';

  @override
  String get resetQuestChanges => 'Redefinir alterações de missões';

  @override
  String get searchQuests => 'Pesquisar missões';

  @override
  String get allGroups => 'Todos os grupos';

  @override
  String groupWithCount(String group, Object count) {
    return '$group ($count)';
  }

  @override
  String stateLabelWithCount(String label, int count) {
    return '$label $count';
  }

  @override
  String get questStateNone => 'Nenhum';

  @override
  String get questStateAvailable => 'Disponível';

  @override
  String get questStateRunning => 'Em andamento';

  @override
  String get questStateSucceeded => 'Concluída';

  @override
  String get questStateFailed => 'Fracassada';

  @override
  String get questStateUnknown => 'desconhecido';

  @override
  String get dialogKnowledge => 'Conhecimento de diálogo';

  @override
  String get resetKnowledgeChanges => 'Redefinir alterações de conhecimento';

  @override
  String get addNpc => 'Adicionar NPC';

  @override
  String get searchNpcs => 'Pesquisar NPCs';

  @override
  String entriesForCharacter(String name) {
    return 'Entradas — $name';
  }

  @override
  String get selectNpcToSeeEntries => 'Selecione um NPC para ver as entradas';

  @override
  String get addKnowledgeEntry => 'Adicionar entrada de conhecimento';

  @override
  String get browseCatalog => 'Navegar pelo catálogo';

  @override
  String get alreadyExistsForCharacter => 'Já existe para este personagem.';

  @override
  String get alreadyInPendingChanges => 'Já está nas alterações pendentes.';

  @override
  String duplicateCheckFailed(String error) {
    return 'A verificação de duplicatas falhou — tente novamente: $error';
  }

  @override
  String pendingAddsCount(int count) {
    return 'Adições pendentes ($count)';
  }

  @override
  String get undoAdd => 'Desfazer adição';

  @override
  String get undoRemove => 'Desfazer remoção';

  @override
  String get removeEntry => 'Remover entrada';

  @override
  String get selectNpcFromList => 'Selecione um NPC na lista';

  @override
  String characterWithCount(String name, int count) {
    return '$name ($count)';
  }

  @override
  String get memoryEvents => 'Eventos de memória';

  @override
  String get searchCharacters => 'Pesquisar personagens';

  @override
  String eventsForCharacter(String name) {
    return 'Eventos — $name';
  }

  @override
  String get selectCharacterToSeeEvents =>
      'Selecione um personagem para ver os eventos';

  @override
  String get noTags => '(sem tags)';

  @override
  String eventSubtitle(String time, String affected) {
    return 't=${time}s  $affected';
  }

  @override
  String get removeEvent => 'Remover evento';

  @override
  String get removeMemoryEventTitle => 'Remover evento de memória?';

  @override
  String get removeMemoryEventBody =>
      'Remover este evento de memória? Um backup é gravado antes.';

  @override
  String get duplicateEvent => 'Duplicar evento';

  @override
  String get duplicateMemoryEventTitle => 'Duplicar evento de memória?';

  @override
  String get duplicateMemoryEventBody =>
      'Duplicar este evento de memória? Um backup é gravado antes.';

  @override
  String get selectCharacterFromList => 'Selecione um personagem na lista';

  @override
  String get allDataLockedBody =>
      'O navegador completo de propriedades exige os dados privados decodificados pelo codec.';

  @override
  String get allDataDescription =>
      'Pesquise todas as propriedades tipadas por nome ou caminho. Escalares, cadeias, enums e caminhos de objeto são editáveis; structs são exibidos como somente leitura por enquanto.';

  @override
  String get searchPropertiesLabel =>
      'Pesquisar propriedades (vazio = listar tudo) — ex.: Health, GameTime';

  @override
  String get decodingSaveTitle => 'Decodificando o save…';

  @override
  String get decodingSaveBody =>
      'Decodificando todo o conteúdo privado para a primeira pesquisa. Isso é feito uma vez por save; depois, as pesquisas são instantâneas.';

  @override
  String get searchTheSaveTitle => 'Pesquisar no save';

  @override
  String get searchTheSaveBody =>
      'Digite o nome de uma propriedade e pressione Enter. Deixe vazio para listar tudo.';

  @override
  String get searchFailedTitle => 'A pesquisa falhou';

  @override
  String get noMatchesTitle => 'Nenhuma correspondência';

  @override
  String get noMatchesBody =>
      'Nenhum caminho de propriedade continha todos esses termos.';

  @override
  String get value => 'Valor';

  @override
  String get backupsTitle => 'Backups';

  @override
  String get refreshBackups => 'Atualizar backups';

  @override
  String get noBackupsTitle => 'Nenhum backup';

  @override
  String get noBackupsBody =>
      'Saves editados criam arquivos de backup ao lado do slot selecionado.';

  @override
  String get slotBackups => 'Backups do slot';

  @override
  String get profileBackups => 'Backups do perfil';

  @override
  String get backupFactName => 'Nome';

  @override
  String get backupFactSlot => 'Slot';

  @override
  String get backupFactCreated => 'Criado em';

  @override
  String get backupFactSize => 'Tamanho';

  @override
  String get backupFactStatus => 'Status';

  @override
  String get backupFactSha1 => 'SHA-1';

  @override
  String restoreBackupTooltip(String fileName) {
    return 'Restaurar $fileName';
  }

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
  String get language => 'Idioma';

  @override
  String get updatesTitle => 'Atualizações';

  @override
  String get checkForUpdatesAutomatically =>
      'Verificar atualizações automaticamente';

  @override
  String get checkForUpdatesNow => 'Verificar atualizações agora';

  @override
  String get updatesPortableNotice =>
      'As atualizações estão disponíveis apenas para versões instaladas. A versão portátil precisa ser atualizada manualmente.';

  @override
  String get gameTextTitle => 'Texto do jogo';

  @override
  String gameTextExtractedWithCounts(int ids, int languages) {
    return 'Extraído: $ids ids em $languages idiomas.';
  }

  @override
  String get gameTextExtracted => 'O texto localizado do jogo foi extraído.';

  @override
  String get gameTextNotExtracted =>
      'O texto localizado do jogo ainda não foi extraído.';

  @override
  String get extracting => 'Extraindo…';

  @override
  String get extractRefreshLocalizedText =>
      'Extrair / atualizar texto localizado';

  @override
  String get extractLocalizedTextTitle => 'Extrair o texto localizado do jogo?';

  @override
  String get extractLocalizedTextBody =>
      'O texto localizado do jogo ainda não foi extraído. Extraí-lo agora da sua instalação do jogo? (opcional)';

  @override
  String get notNow => 'Agora não';

  @override
  String get extract => 'Extrair';

  @override
  String get extractionComplete => 'Extração concluída';

  @override
  String get extractionFailed => 'A extração falhou';

  @override
  String get localizationCacheFileType => 'Cache de localização';

  @override
  String get savegameDirectoryTitle => 'Diretório de saves';

  @override
  String get folder => 'Pasta';

  @override
  String get codecTitle => 'Codec';

  @override
  String get check => 'Verificar';

  @override
  String get roundtrip => 'Ida e volta';

  @override
  String get noCodecStatus => 'Sem status do codec';

  @override
  String get codecReady => 'Codec pronto';

  @override
  String get codecReadOnly => 'Codec somente leitura';

  @override
  String get codecUnavailable => 'Codec indisponível';

  @override
  String get details => 'Detalhes';

  @override
  String codecStatusLine(String status) {
    return 'Status: $status';
  }

  @override
  String codecCapabilityLine(String decompress, String compress) {
    return 'Descompressão: $decompress | Compressão: $compress';
  }

  @override
  String codecBackendLine(String backend) {
    return 'Backend: $backend';
  }

  @override
  String get yes => 'sim';

  @override
  String get no => 'não';

  @override
  String get aboutSubtitle => 'Editor de Saves do Gothic Remake';

  @override
  String aboutVersion(String version, String sha) {
    return 'Versão $version ($sha)';
  }

  @override
  String get aboutCopyright => '© 2026 colaboradores do goresave';

  @override
  String get aboutLicense => 'Licenciado sob a Licença MIT.';

  @override
  String difficultyTitle(String profile) {
    return 'Dificuldade — $profile';
  }

  @override
  String get difficultyNoProfile => 'Nenhum perfil';

  @override
  String get difficultyNoDifficulty => 'Sem dificuldade';

  @override
  String get difficultyLabel => 'Dificuldade';

  @override
  String get difficultyTooltipNoProfile => 'Nenhum perfil selecionado';

  @override
  String get difficultyTooltipEdit => 'Editar a dificuldade deste perfil';

  @override
  String get difficultyTooltipNoEditable =>
      'Este perfil não tem dificuldade editável';

  @override
  String get preset => 'Predefinição';

  @override
  String get presetNovice => 'Iniciante';

  @override
  String get presetGothic => 'Gothic';

  @override
  String get presetHard => 'Difícil';

  @override
  String get presetCustom => 'Personalizado';

  @override
  String unrecognisedPreset(Object preset) {
    return 'A predefinição armazenada não é reconhecida ($preset). Você ainda pode salvar alterações de Assistente de Fluência / Permadeath, ou escolher uma predefinição acima para sobrescrevê-la.';
  }

  @override
  String get closeCombatFlowHelper =>
      'Assistente de Fluência em Combate Corpo a Corpo';

  @override
  String get permadeath => 'Permadeath';

  @override
  String get notAvailableOnNovice => 'Indisponível no Iniciante';

  @override
  String get levelCombat => 'Combate';

  @override
  String get levelResources => 'Recursos';

  @override
  String get levelProgression => 'Progresso';

  @override
  String get difficultyAppliesToAllSaves =>
      'A dificuldade se aplica a todos os saves deste perfil.';

  @override
  String get savingDifficultyFailed => 'Falha ao salvar a dificuldade.';

  @override
  String get addItemDialogTitle => 'Adicionar item';

  @override
  String get searchItems => 'Pesquisar itens';

  @override
  String failedToLoadCatalog(String error) {
    return 'Falha ao carregar o catálogo: $error';
  }

  @override
  String get noItemsAvailableToAdd => 'Nenhum item disponível para adicionar';

  @override
  String get noItemsMatch => 'Nenhum item corresponde';

  @override
  String get countMustBeAtLeast1 => 'Deve ser ≥ 1';

  @override
  String countMustBeAtMost(int max) {
    return 'Deve ser ≤ $max';
  }

  @override
  String get addNpcDialogTitle => 'Adicionar NPC';

  @override
  String get noNpcsAvailableToAdd => 'Nenhum NPC disponível para adicionar';

  @override
  String get noNpcsMatch => 'Nenhum NPC corresponde';

  @override
  String get categoryAll => 'Todos';

  @override
  String allWithCount(int count) {
    return 'Todos ($count)';
  }

  @override
  String get addKnowledgeEntryDialogTitle =>
      'Adicionar entrada de conhecimento';

  @override
  String get searchEntries => 'Pesquisar entradas';

  @override
  String get noKnowledgeEntriesAvailableToAdd =>
      'Nenhuma entrada de conhecimento disponível para adicionar';

  @override
  String get noEntriesMatch => 'Nenhuma entrada corresponde';

  @override
  String get heroGroupMainStats => 'Atributos principais';

  @override
  String get heroGroupCombatSkills => 'Habilidades de combate';

  @override
  String get heroGroupResistances => 'Resistências';

  @override
  String get heroGroupThieving => 'Furto';

  @override
  String get heroGroupAdvanced => 'Avançado';

  @override
  String get heroEntryHeroTransform => 'Transformação do herói';

  @override
  String attributeEmpty(String name) {
    return '$name está vazio — insira um valor ou restaure o original antes de salvar.';
  }

  @override
  String attributeInvalidNumber(String name, String text) {
    return 'Número inválido para $name: \"$text\"';
  }

  @override
  String get loadingEditorData => 'Carregando os dados do editor';
}

/// The translations for Portuguese, as used in Brazil (`pt_BR`).
class AppLocalizationsPtBr extends AppLocalizationsPt {
  AppLocalizationsPtBr() : super('pt_BR');

  @override
  String get appTitle => 'Editor de Saves do Gothic Remake';

  @override
  String get appLogoSemanticLabel => 'logotipo do goresave';

  @override
  String get zoomTooltip => 'Pressione Ctrl +/- para ampliar/reduzir';

  @override
  String get switchToLightMode => 'Mudar para o modo claro';

  @override
  String get switchToDarkMode => 'Mudar para o modo escuro';

  @override
  String get about => 'Sobre';

  @override
  String get tabOverview => 'Visão geral';

  @override
  String get tabPlayer => 'Jogador';

  @override
  String get tabInventory => 'Inventário';

  @override
  String get tabProgression => 'Progresso';

  @override
  String get tabAllData => 'Todos os dados';

  @override
  String get tabBackups => 'Backups';

  @override
  String get tabSettings => 'Configurações';

  @override
  String get reset => 'Redefinir';

  @override
  String get save => 'Salvar';

  @override
  String saveWithCount(int count) {
    return 'Salvar ($count)';
  }

  @override
  String get ok => 'OK';

  @override
  String get cancel => 'Cancelar';

  @override
  String get confirm => 'Confirmar';

  @override
  String get close => 'Fechar';

  @override
  String get add => 'Adicionar';

  @override
  String get browse => 'Procurar';

  @override
  String get noSavFilesFound => 'Nenhum arquivo .sav encontrado';

  @override
  String get profile => 'Perfil';

  @override
  String profileWithSaves(String name, int count) {
    return '$name ($count saves)';
  }

  @override
  String get switchProfile => 'Trocar de perfil';

  @override
  String get rescanSaveFolder => 'Reverificar a pasta de saves';

  @override
  String get discardUnsavedChangesTitle =>
      'Descartar as alterações não salvas?';

  @override
  String rescanDiscardBody(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: 'alterações não salvas',
      one: 'alteração não salva',
    );
    return 'A reverificação recarrega todos os saves e descarta suas $count $_temp0.';
  }

  @override
  String get discardAndRescan => 'Descartar e reverificar';

  @override
  String chapterLabel(Object id) {
    return 'Capítulo $id';
  }

  @override
  String get quickSave => 'Save rápido';

  @override
  String get autoSave => 'Save automático';

  @override
  String get manualSave => 'Save manual';

  @override
  String get errorTitle => 'Erro';

  @override
  String get selectASaveTitle => 'Selecione um save';

  @override
  String get selectASaveBody => 'Os detalhes do save aparecerão aqui.';

  @override
  String get diagnosticsTitle => 'Diagnóstico e detalhes';

  @override
  String get diagnosticsSubtitle => 'Inspeção de formato somente leitura';

  @override
  String get metricFormat => 'Formato';

  @override
  String get metricSlot => 'Slot';

  @override
  String get metricChapter => 'Capítulo';

  @override
  String get metricTimePlayed => 'Tempo de jogo';

  @override
  String get metricSaveKind => 'Tipo de save';

  @override
  String get metricFileSize => 'Tamanho do arquivo';

  @override
  String get metricCompression => 'Compressão';

  @override
  String get metricChunks => 'Blocos';

  @override
  String get metricUncompressed => 'Não comprimido';

  @override
  String get metricPrivate => 'Privado';

  @override
  String get metricSlotName => 'Nome do slot';

  @override
  String get metricTrailer => 'Trailer';

  @override
  String get metricDecodedPrivate => 'Privado decodificado';

  @override
  String get metricPrivateStrings => 'Cadeias privadas';

  @override
  String get metricSha1 => 'SHA-1';

  @override
  String bytesValue(String count) {
    return '$count bytes';
  }

  @override
  String get inspectionJsonTitle => 'JSON de inspeção';

  @override
  String get inspectionJsonSubtitle => 'Dados brutos de inspeção do save';

  @override
  String get copy => 'Copiar';

  @override
  String get savegameFallbackTitle => 'Save';

  @override
  String screenshotForSlot(String slot) {
    return 'Captura de tela do $slot';
  }

  @override
  String get publicSaveName => 'Nome público do save';

  @override
  String get required => 'Obrigatório';

  @override
  String get playerLockedBody =>
      'Edições privadas do jogador exigem um codec capaz de comprimir.';

  @override
  String get heroTransform => 'Transformação do herói';

  @override
  String get locationX => 'Posição X';

  @override
  String get locationY => 'Posição Y';

  @override
  String get locationZ => 'Posição Z';

  @override
  String get rotationPitch => 'Rotação (pitch)';

  @override
  String get rotationYaw => 'Rotação (yaw)';

  @override
  String get rotationRoll => 'Rotação (roll)';

  @override
  String get invalid => 'Inválido';

  @override
  String get heroAttributes => 'Atributos do herói';

  @override
  String attributeBase(String name) {
    return '$name base';
  }

  @override
  String attributeCurrent(String name) {
    return '$name atual';
  }

  @override
  String get inventoryTitle => 'Inventário';

  @override
  String get inventoryNeedsDecoded =>
      'A edição do inventário exige os dados privados decodificados pelo codec.';

  @override
  String get inventoryNoStacks =>
      'Nenhuma pilha de itens encontrada nos dados privados decodificados.';

  @override
  String get resetInventoryChanges => 'Redefinir alterações do inventário';

  @override
  String get addItemTooltipPendingAdd =>
      'Salve primeiro as alterações pendentes — um novo item por vez ao salvar';

  @override
  String get addItemTooltipPendingRemove =>
      'Salve primeiro a remoção pendente — uma alteração estrutural por vez ao salvar';

  @override
  String get addItemTooltipPendingCount =>
      'Salve ou redefina primeiro as alterações de quantidade pendentes — uma edição estrutural precisa ser salva sozinha';

  @override
  String get addItemTooltipDefault => 'Adicionar item ao inventário';

  @override
  String get addItemButton => 'Adicionar item';

  @override
  String pendingAddSubtitle(int count) {
    return '×$count — adição pendente (ainda não salva)';
  }

  @override
  String get cancelPendingAdd => 'Cancelar adição pendente';

  @override
  String get pendingRemovalSubtitle => 'remoção pendente (ainda não salva)';

  @override
  String get cancelPendingRemoval => 'Cancelar remoção pendente';

  @override
  String get filterItems => 'Filtrar itens';

  @override
  String noItemsMatchQuery(String query) {
    return 'Nenhum item corresponde a \"$query\".';
  }

  @override
  String get pendingRemovalHidesAll =>
      'A remoção pendente oculta todos os itens — salve para aplicá-la.';

  @override
  String categoryWithCount(String label, int count) {
    return '$label ($count)';
  }

  @override
  String get count => 'Quantidade';

  @override
  String get min1 => 'Mín. 1';

  @override
  String countTimes(String count) {
    return '×$count';
  }

  @override
  String get deleteEquippedTooltip =>
      'Não é possível excluir: este item provavelmente está equipado ou atribuído a um slot de atalho';

  @override
  String get removeBlockedTooltip =>
      'Salve ou redefina primeiro as alterações pendentes do inventário — uma adição ou remoção precisa ser salva sozinha';

  @override
  String get removeItemFromInventory => 'Remover item do inventário';

  @override
  String get progressionLockedBody =>
      'Os dados de progresso exigem os dados privados decodificados pelo codec.';

  @override
  String get progressionNeedsTyped =>
      'Os dados estruturados de progresso exigem um save totalmente decodificado com análise tipada verificada.';

  @override
  String get sectionQuests => 'Missões';

  @override
  String get sectionKnowledge => 'Conhecimento';

  @override
  String get sectionEvents => 'Eventos';

  @override
  String get firstPage => 'Primeira página';

  @override
  String get previousPage => 'Página anterior';

  @override
  String get nextPage => 'Próxima página';

  @override
  String get lastPage => 'Última página';

  @override
  String pageOfPages(int page, int total) {
    return 'Página $page / $total';
  }

  @override
  String rangeOfTotal(int first, int last, int total) {
    return '$first–$last de $total';
  }

  @override
  String get perPage => 'Por página:';

  @override
  String get resetQuestChanges => 'Redefinir alterações de missões';

  @override
  String get searchQuests => 'Pesquisar missões';

  @override
  String get allGroups => 'Todos os grupos';

  @override
  String groupWithCount(String group, Object count) {
    return '$group ($count)';
  }

  @override
  String stateLabelWithCount(String label, int count) {
    return '$label $count';
  }

  @override
  String get questStateNone => 'Nenhum';

  @override
  String get questStateAvailable => 'Disponível';

  @override
  String get questStateRunning => 'Em andamento';

  @override
  String get questStateSucceeded => 'Concluída';

  @override
  String get questStateFailed => 'Fracassada';

  @override
  String get questStateUnknown => 'desconhecido';

  @override
  String get dialogKnowledge => 'Conhecimento de diálogo';

  @override
  String get resetKnowledgeChanges => 'Redefinir alterações de conhecimento';

  @override
  String get addNpc => 'Adicionar NPC';

  @override
  String get searchNpcs => 'Pesquisar NPCs';

  @override
  String entriesForCharacter(String name) {
    return 'Entradas — $name';
  }

  @override
  String get selectNpcToSeeEntries => 'Selecione um NPC para ver as entradas';

  @override
  String get addKnowledgeEntry => 'Adicionar entrada de conhecimento';

  @override
  String get browseCatalog => 'Navegar pelo catálogo';

  @override
  String get alreadyExistsForCharacter => 'Já existe para este personagem.';

  @override
  String get alreadyInPendingChanges => 'Já está nas alterações pendentes.';

  @override
  String duplicateCheckFailed(String error) {
    return 'A verificação de duplicatas falhou — tente novamente: $error';
  }

  @override
  String pendingAddsCount(int count) {
    return 'Adições pendentes ($count)';
  }

  @override
  String get undoAdd => 'Desfazer adição';

  @override
  String get undoRemove => 'Desfazer remoção';

  @override
  String get removeEntry => 'Remover entrada';

  @override
  String get selectNpcFromList => 'Selecione um NPC na lista';

  @override
  String characterWithCount(String name, int count) {
    return '$name ($count)';
  }

  @override
  String get memoryEvents => 'Eventos de memória';

  @override
  String get searchCharacters => 'Pesquisar personagens';

  @override
  String eventsForCharacter(String name) {
    return 'Eventos — $name';
  }

  @override
  String get selectCharacterToSeeEvents =>
      'Selecione um personagem para ver os eventos';

  @override
  String get noTags => '(sem tags)';

  @override
  String eventSubtitle(String time, String affected) {
    return 't=${time}s  $affected';
  }

  @override
  String get removeEvent => 'Remover evento';

  @override
  String get removeMemoryEventTitle => 'Remover evento de memória?';

  @override
  String get removeMemoryEventBody =>
      'Remover este evento de memória? Um backup é gravado antes.';

  @override
  String get duplicateEvent => 'Duplicar evento';

  @override
  String get duplicateMemoryEventTitle => 'Duplicar evento de memória?';

  @override
  String get duplicateMemoryEventBody =>
      'Duplicar este evento de memória? Um backup é gravado antes.';

  @override
  String get selectCharacterFromList => 'Selecione um personagem na lista';

  @override
  String get allDataLockedBody =>
      'O navegador completo de propriedades exige os dados privados decodificados pelo codec.';

  @override
  String get allDataDescription =>
      'Pesquise todas as propriedades tipadas por nome ou caminho. Escalares, cadeias, enums e caminhos de objeto são editáveis; structs são exibidos como somente leitura por enquanto.';

  @override
  String get searchPropertiesLabel =>
      'Pesquisar propriedades (vazio = listar tudo) — ex.: Health, GameTime';

  @override
  String get decodingSaveTitle => 'Decodificando o save…';

  @override
  String get decodingSaveBody =>
      'Decodificando todo o conteúdo privado para a primeira pesquisa. Isso é feito uma vez por save; depois, as pesquisas são instantâneas.';

  @override
  String get searchTheSaveTitle => 'Pesquisar no save';

  @override
  String get searchTheSaveBody =>
      'Digite o nome de uma propriedade e pressione Enter. Deixe vazio para listar tudo.';

  @override
  String get searchFailedTitle => 'A pesquisa falhou';

  @override
  String get noMatchesTitle => 'Nenhuma correspondência';

  @override
  String get noMatchesBody =>
      'Nenhum caminho de propriedade continha todos esses termos.';

  @override
  String get value => 'Valor';

  @override
  String get backupsTitle => 'Backups';

  @override
  String get refreshBackups => 'Atualizar backups';

  @override
  String get noBackupsTitle => 'Nenhum backup';

  @override
  String get noBackupsBody =>
      'Saves editados criam arquivos de backup ao lado do slot selecionado.';

  @override
  String get slotBackups => 'Backups do slot';

  @override
  String get profileBackups => 'Backups do perfil';

  @override
  String get backupFactName => 'Nome';

  @override
  String get backupFactSlot => 'Slot';

  @override
  String get backupFactCreated => 'Criado em';

  @override
  String get backupFactSize => 'Tamanho';

  @override
  String get backupFactStatus => 'Status';

  @override
  String get backupFactSha1 => 'SHA-1';

  @override
  String restoreBackupTooltip(String fileName) {
    return 'Restaurar $fileName';
  }

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
  String get language => 'Idioma';

  @override
  String get updatesTitle => 'Atualizações';

  @override
  String get checkForUpdatesAutomatically =>
      'Verificar atualizações automaticamente';

  @override
  String get checkForUpdatesNow => 'Verificar atualizações agora';

  @override
  String get updatesPortableNotice =>
      'As atualizações estão disponíveis apenas para versões instaladas. A versão portátil precisa ser atualizada manualmente.';

  @override
  String get gameTextTitle => 'Texto do jogo';

  @override
  String gameTextExtractedWithCounts(int ids, int languages) {
    return 'Extraído: $ids ids em $languages idiomas.';
  }

  @override
  String get gameTextExtracted => 'O texto localizado do jogo foi extraído.';

  @override
  String get gameTextNotExtracted =>
      'O texto localizado do jogo ainda não foi extraído.';

  @override
  String get extracting => 'Extraindo…';

  @override
  String get extractRefreshLocalizedText =>
      'Extrair / atualizar texto localizado';

  @override
  String get extractLocalizedTextTitle => 'Extrair o texto localizado do jogo?';

  @override
  String get extractLocalizedTextBody =>
      'O texto localizado do jogo ainda não foi extraído. Extraí-lo agora da sua instalação do jogo? (opcional)';

  @override
  String get notNow => 'Agora não';

  @override
  String get extract => 'Extrair';

  @override
  String get extractionComplete => 'Extração concluída';

  @override
  String get extractionFailed => 'A extração falhou';

  @override
  String get localizationCacheFileType => 'Cache de localização';

  @override
  String get savegameDirectoryTitle => 'Diretório de saves';

  @override
  String get folder => 'Pasta';

  @override
  String get codecTitle => 'Codec';

  @override
  String get check => 'Verificar';

  @override
  String get roundtrip => 'Ida e volta';

  @override
  String get noCodecStatus => 'Sem status do codec';

  @override
  String get codecReady => 'Codec pronto';

  @override
  String get codecReadOnly => 'Codec somente leitura';

  @override
  String get codecUnavailable => 'Codec indisponível';

  @override
  String get details => 'Detalhes';

  @override
  String codecStatusLine(String status) {
    return 'Status: $status';
  }

  @override
  String codecCapabilityLine(String decompress, String compress) {
    return 'Descompressão: $decompress | Compressão: $compress';
  }

  @override
  String codecBackendLine(String backend) {
    return 'Backend: $backend';
  }

  @override
  String get yes => 'sim';

  @override
  String get no => 'não';

  @override
  String get aboutSubtitle => 'Editor de Saves do Gothic Remake';

  @override
  String aboutVersion(String version, String sha) {
    return 'Versão $version ($sha)';
  }

  @override
  String get aboutCopyright => '© 2026 colaboradores do goresave';

  @override
  String get aboutLicense => 'Licenciado sob a Licença MIT.';

  @override
  String difficultyTitle(String profile) {
    return 'Dificuldade — $profile';
  }

  @override
  String get difficultyNoProfile => 'Nenhum perfil';

  @override
  String get difficultyNoDifficulty => 'Sem dificuldade';

  @override
  String get difficultyLabel => 'Dificuldade';

  @override
  String get difficultyTooltipNoProfile => 'Nenhum perfil selecionado';

  @override
  String get difficultyTooltipEdit => 'Editar a dificuldade deste perfil';

  @override
  String get difficultyTooltipNoEditable =>
      'Este perfil não tem dificuldade editável';

  @override
  String get preset => 'Predefinição';

  @override
  String get presetNovice => 'Iniciante';

  @override
  String get presetGothic => 'Gothic';

  @override
  String get presetHard => 'Difícil';

  @override
  String get presetCustom => 'Personalizado';

  @override
  String unrecognisedPreset(Object preset) {
    return 'A predefinição armazenada não é reconhecida ($preset). Você ainda pode salvar alterações de Assistente de Fluência / Permadeath, ou escolher uma predefinição acima para sobrescrevê-la.';
  }

  @override
  String get closeCombatFlowHelper =>
      'Assistente de Fluência em Combate Corpo a Corpo';

  @override
  String get permadeath => 'Permadeath';

  @override
  String get notAvailableOnNovice => 'Indisponível no Iniciante';

  @override
  String get levelCombat => 'Combate';

  @override
  String get levelResources => 'Recursos';

  @override
  String get levelProgression => 'Progresso';

  @override
  String get difficultyAppliesToAllSaves =>
      'A dificuldade se aplica a todos os saves deste perfil.';

  @override
  String get savingDifficultyFailed => 'Falha ao salvar a dificuldade.';

  @override
  String get addItemDialogTitle => 'Adicionar item';

  @override
  String get searchItems => 'Pesquisar itens';

  @override
  String failedToLoadCatalog(String error) {
    return 'Falha ao carregar o catálogo: $error';
  }

  @override
  String get noItemsAvailableToAdd => 'Nenhum item disponível para adicionar';

  @override
  String get noItemsMatch => 'Nenhum item corresponde';

  @override
  String get countMustBeAtLeast1 => 'Deve ser ≥ 1';

  @override
  String countMustBeAtMost(int max) {
    return 'Deve ser ≤ $max';
  }

  @override
  String get addNpcDialogTitle => 'Adicionar NPC';

  @override
  String get noNpcsAvailableToAdd => 'Nenhum NPC disponível para adicionar';

  @override
  String get noNpcsMatch => 'Nenhum NPC corresponde';

  @override
  String get categoryAll => 'Todos';

  @override
  String allWithCount(int count) {
    return 'Todos ($count)';
  }

  @override
  String get addKnowledgeEntryDialogTitle =>
      'Adicionar entrada de conhecimento';

  @override
  String get searchEntries => 'Pesquisar entradas';

  @override
  String get noKnowledgeEntriesAvailableToAdd =>
      'Nenhuma entrada de conhecimento disponível para adicionar';

  @override
  String get noEntriesMatch => 'Nenhuma entrada corresponde';

  @override
  String get heroGroupMainStats => 'Atributos principais';

  @override
  String get heroGroupCombatSkills => 'Habilidades de combate';

  @override
  String get heroGroupResistances => 'Resistências';

  @override
  String get heroGroupThieving => 'Furto';

  @override
  String get heroGroupAdvanced => 'Avançado';

  @override
  String get heroEntryHeroTransform => 'Transformação do herói';

  @override
  String attributeEmpty(String name) {
    return '$name está vazio — insira um valor ou restaure o original antes de salvar.';
  }

  @override
  String attributeInvalidNumber(String name, String text) {
    return 'Número inválido para $name: \"$text\"';
  }

  @override
  String get loadingEditorData => 'Carregando os dados do editor';
}
