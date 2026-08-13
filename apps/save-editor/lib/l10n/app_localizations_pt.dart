// ignore: unused_import
import 'package:intl/intl.dart' as intl;
import 'app_localizations.dart';

// ignore_for_file: type=lint

/// The translations for Portuguese (`pt`).
class AppLocalizationsPt extends AppLocalizations {
  AppLocalizationsPt([String locale = 'pt']) : super(locale);

  @override
  String get debugSectionTitle => 'Avançado (depuração)';

  @override
  String get debugSectionSubtitle =>
      'Diagnóstico e dados brutos para relatórios de bugs';

  @override
  String get showObjectIdsTitle => 'Mostrar IDs técnicos adicionais';

  @override
  String get showObjectIdsSubtitle =>
      'Mostra IDs técnicos de itens, conhecimento de diálogo, missões e atores órfãos. IDs de NPCs são sempre exibidos.';

  @override
  String get storyStateSidebar => 'Estado da história';

  @override
  String get storyStateDescription =>
      'Catálogo de referência dos estados persistentes declarados pelos scripts fornecidos com o jogo. As entradas guardadas mostram o valor bruto; os campos do catálogo ausentes deste jogo guardado são marcados como não definidos. Os marcadores temporais declarados no código são apresentados como tempo de jogo; os restantes inteiros podem ser booleanos, contadores ou estados de vários níveis.';

  @override
  String get storyStateReadOnly =>
      'Só de leitura até serem conhecidos o significado dos valores nos scripts e uma escrita segura do mapa. O texto de glossário relacionado fornece contexto; não é uma tradução direta do ID técnico.';

  @override
  String get storyStateStructureReadOnly =>
      'Não foi possível identificar de forma inequívoca e segura a estrutura StoryPropertyValues deste jogo guardado. Os valores da história permanecem só de leitura para este jogo guardado.';

  @override
  String get storyStateSearch => 'Pesquisar estado da história';

  @override
  String storyStateValuesCount(int shown, int total) {
    return '$shown de $total valores da história';
  }

  @override
  String get storyStateInteger => 'Inteiro';

  @override
  String get storyStateTimeMarker => 'Marcador temporal';

  @override
  String get storyStateChapter => 'Capítulo';

  @override
  String get storyStateUnknown => 'Tipo de origem desconhecido';

  @override
  String get storyStateUnknownDetail =>
      'Este ID guardado não consta do catálogo de scripts atual (por exemplo, por um mod ou uma versão mais recente do jogo). O valor serializado é int32, mas o seu significado não é inferido.';

  @override
  String get storyStateStored => 'Guardado';

  @override
  String get storyStateUnset => 'Não definido';

  @override
  String get storyStateUnsetDetail =>
      'Este campo do catálogo não está serializado neste jogo guardado; o jogo usa por isso o estado não definido ou predefinido.';

  @override
  String get storyStateRawValue => 'Valor bruto';

  @override
  String storyStateElapsed(String duration) {
    return 'Tempo decorrido ao guardar: $duration';
  }

  @override
  String storyStateAhead(String duration) {
    return 'No futuro ao guardar: $duration';
  }

  @override
  String storyStateDurationDays(int days, String time) {
    String _temp0 = intl.Intl.pluralLogic(
      days,
      locale: localeName,
      other: '$days dias',
      one: '1 dia',
    );
    return '$_temp0 $time';
  }

  @override
  String get storyStateRelatedGlossary => 'Entrada de glossário relacionada';

  @override
  String get storyStateTechnicalPath => 'Caminho técnico';

  @override
  String get storyStateEditingGuidance =>
      'Todas as entradas podem ser editadas em todo o intervalo int32 com sinal. Os indicadores e as sugestões de valores baseados nos scripts servem apenas de orientação; a introdução do valor bruto está sempre disponível. As alterações ao estado da história podem ignorar transições de diálogos, missões ou do mundo, por isso guarda-as com cuidado. É criada automaticamente uma cópia de segurança.';

  @override
  String get storyStatePending => 'Pendente';

  @override
  String storyStatePendingValue(String value) {
    return 'Será guardado como $value';
  }

  @override
  String get storyStatePendingRemoval => 'Será removido do jogo guardado';

  @override
  String get storyStateEditValue => 'Editar valor';

  @override
  String get storyStateSetValue => 'Definir valor';

  @override
  String get storyStateRemoveValue => 'Remover do jogo guardado';

  @override
  String get storyStateUndoChange => 'Anular alteração da história';

  @override
  String get storyStateResetChanges => 'Repor alterações da história';

  @override
  String storyStateDialogTitle(String id) {
    return 'Editar $id';
  }

  @override
  String get storyStateRawInput => 'Valor int32 com sinal';

  @override
  String get storyStateInvalidInt32 =>
      'Introduz um número inteiro entre -2147483648 e 2147483647.';

  @override
  String get storyStateQueueChange => 'Adicionar alteração à fila';

  @override
  String storyStateSuggestedValues(String values) {
    return 'Valores confirmados nos scripts fornecidos: $values';
  }

  @override
  String get storyStateSuggestionsNotLimits =>
      'As sugestões não são limites de validação; o código nativo, os mods ou versões posteriores do jogo podem usar outros valores.';

  @override
  String get storyStateUseCurrentTime => 'Usar a hora atual do jogo guardado';

  @override
  String get storyStateStructuredTime => 'Dia / hora';

  @override
  String get storyStateRawMode => 'int32 bruto';

  @override
  String get storyStateChapterWarning =>
      'Alterar apenas o capítulo não sincroniza missões, NPC, inventário nem o estado do mundo.';

  @override
  String get storyStateDormantWarning =>
      'Não foi encontrada qualquer leitura ou escrita ativa deste campo na cache dos scripts fornecidos. Pode ser antigo, controlado por código nativo ou reservado.';

  @override
  String get storyStateReadOnlySourceWarning =>
      'Os scripts fornecidos leem este campo, mas não contêm qualquer escrita por script. O código nativo pode ainda ser responsável por ele.';

  @override
  String get storyStateUnknownEditWarning =>
      'Este ID de um mod ou de uma versão posterior não tem semântica de código-fonte incluída. Edita apenas o respetivo valor int32 bruto.';

  @override
  String storyStateIntegerKind(String kind) {
    String _temp0 = intl.Intl.selectLogic(kind, {
      'binaryFlag': 'Indicador binário',
      'finiteState': 'Valor com vários estados',
      'counterOrScore': 'Contador / pontuação',
      'calendarDay': 'Dia do calendário',
      'derivedOrOpaqueInteger': 'Inteiro derivado / opaco',
      'readOnlyInSourceInteger': 'Só de leitura nos scripts fornecidos',
      'dormantOrLegacyInteger': 'Não utilizado nos scripts fornecidos',
      'other': 'Inteiro',
    });
    return '$_temp0';
  }

  @override
  String get storyStateZeroVsUnset =>
      'Um 0 guardado e uma entrada ausente do mapa são estados de ficheiro distintos. «Remover do jogo guardado» repõe o estado do construtor ou o estado predefinido.';

  @override
  String get appTitle => 'GORE Save Editor';

  @override
  String get appLogoSemanticLabel => 'Logotipo do GORE Save Editor';

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
  String get tabAttribute => 'Atributos';

  @override
  String get heroGroupSkills => 'Aptidões';

  @override
  String get skillsNoneBody =>
      'Não foram encontradas aptidões para esta personagem.';

  @override
  String get skillsUnavailableBody =>
      'As perícias não podem ser editadas neste save — o herói não tem dados de efeito para modificar.';

  @override
  String get skillNotLearned => 'Não aprendida';

  @override
  String get skillLearn => 'Aprender';

  @override
  String get skillActionLearn => 'aprender';

  @override
  String get skillActionUnlearn => 'desaprender';

  @override
  String get skillTierUntrained => 'Sem Treinamento';

  @override
  String get skillTierBeginner => 'Principiante';

  @override
  String get skillTierTrained => 'Treinado';

  @override
  String get skillTierMaster => 'Mestre';

  @override
  String get skillTierNovice => 'Principiante';

  @override
  String get skillTierAmateur => 'Amador (Círculo 0)';

  @override
  String get skillTierLearned => 'Aprendida';

  @override
  String skillTierCircle(int n) {
    return 'Círculo $n';
  }

  @override
  String get skillHintBlacksmith1H => 'Armas de uma mão';

  @override
  String get skillHintBlacksmith2H => 'Armas de duas mãos';

  @override
  String get skillCategoryCombat => 'Combate';

  @override
  String get skillCategoryCrafting => 'Artesanato';

  @override
  String get skillCategoryHunting => 'Caça';

  @override
  String get skillCategoryLanguage => 'Idioma';

  @override
  String get skillCategoryMagic => 'Magia';

  @override
  String get skillCategoryMovement => 'Movimento';

  @override
  String get skillCategoryThievery => 'Roubo';

  @override
  String get skillNameOneHanded => 'Uma Mão';

  @override
  String get skillNameTwoHanded => 'Duas Mãos';

  @override
  String get skillNameFists => 'Punhos';

  @override
  String get skillNameBow => 'Arco';

  @override
  String get skillNameCrossbow => 'Besta';

  @override
  String get skillNameLockpicking => 'Abrir Fechaduras';

  @override
  String get skillNamePickpocketing => 'Mão-Leve';

  @override
  String get skillNameTakeOrgans => 'Extrair Órgão';

  @override
  String get skillNameBreakTeeth => 'Extrair Dentes';

  @override
  String get skillNameTakeClaws => 'Extrair Garra';

  @override
  String get skillNameSkinFur => 'Pegar Pelo';

  @override
  String get skillNameSkin => 'Pegar Pele';

  @override
  String get skillNameTakeFins => 'Pegar Barbatanas';

  @override
  String get skillNameTakeStingers => 'Extrair Ferrões';

  @override
  String get skillNameTakeSecretion => 'Extrair Secreção';

  @override
  String get skillNameTakeSkullPlates => 'Pegar Carapaça de Osso';

  @override
  String get skillNameSkinSwampshark => 'Pegar Pele de Tubarão';

  @override
  String get skillNameTakeMinecrawlerPlates => 'Pegar Carapaças';

  @override
  String get skillNameTakeScutes => 'Pegar Escamas';

  @override
  String get skillNameTakeUluMulu => 'Pegar Ulu-Mulu';

  @override
  String get skillNameOrcWeapons => 'Armas orcas';

  @override
  String get skillNameMining => 'Mineração';

  @override
  String get skillNameDiving => 'Mergulhar';

  @override
  String get skillNameTakeMinecrawlerMandibles => 'Extrair Mandíbulas';

  @override
  String get skillNameTakeShadowbeastHorn => 'Pegar Chifre (Shadowbeast)';

  @override
  String get skillNameTakeSpines => 'Extrair Espinha';

  @override
  String get skillNameBreakSwampsharkTeeth => 'Extrair Dentes de Tubarão';

  @override
  String get skillNameTakeFireTongue => 'Pegar Língua-de-Fogo';

  @override
  String get skillNameTakeTrollHorn => 'Pegar Chifre (Troll)';

  @override
  String get skillNameAcrobatics => 'Acrobacia';

  @override
  String get skillNameWallClimbing => 'Escalar';

  @override
  String get skillNameRiding => 'Andar em Catador';

  @override
  String get skillNameSneaking => 'Furtividade';

  @override
  String get skillNameAlchemy => 'Alquimia';

  @override
  String get skillNameRuneInscription => 'Inscrição';

  @override
  String get skillNameBlacksmithing => 'Ferraria';

  @override
  String get skillNameMagicCircle => 'Círculo de Magia';

  @override
  String get skillNameOrcish => 'Língua dos Orcs';

  @override
  String get tabInventory => 'Inventário';

  @override
  String get tabTrade => 'Comércio';

  @override
  String get traderNotAMerchant => 'Esta personagem não comercia.';

  @override
  String get traderRetry => 'Tentar novamente';

  @override
  String get traderAmbiguousName =>
      'Mais do que um registo de mercador tem este nome, por isso não é possível saber que loja pertence a esta personagem. A edição está desativada em vez de arriscar mudar a errada.';

  @override
  String get traderOre => 'Minério (poder de compra)';

  @override
  String get traderNoOre => 'sem minério';

  @override
  String get traderStockCurrent => 'Estoque';

  @override
  String get traderStockBase => 'Base de reposição';

  @override
  String get traderStockBaseHint =>
      'Aquilo que o mercador repõe. Cresce com o progresso da história, por isso não é um estado original.';

  @override
  String get traderOreHint =>
      'O valor no jogo difere: ao carregar, o jogo soma o que se acumulou desde a última troca — ele vende excedentes e repõe com isso. Este número é o ponto de partida, não o que o ecrã de comércio mostra.';

  @override
  String get traderPriceWarning =>
      'Os preços reagem ao que o mercador tem em estoque e ao minério que possui, por isso mudar estes números também pode alterar o que ele cobra.';

  @override
  String get traderAddItem => 'Adicionar item';

  @override
  String get traderRemoveItem => 'Remover linha';

  @override
  String get traderReadOnlyCore =>
      'Esta versão do núcleo só consegue ler os dados dos mercadores.';

  @override
  String get traderDifficultyStockUnsupported =>
      'Este mercador tem existências por dificuldade, que o editor não modela. A edição está desativada aqui, porque uma alteração pareceria bem-sucedida deixando essas existências intactas.';

  @override
  String get traderRecordIncomplete =>
      'As listas de existências deste mercador não existem, ou têm uma forma que o editor não suporta nem consegue gravar. A edição está desativada aqui para que uma alteração não falhe ao gravar.';

  @override
  String get traderEmptyStock => 'Nada em estoque.';

  @override
  String get traderUnknownItem => 'não está no catálogo de itens';

  @override
  String editorTradersLoadFailed(String details) {
    return 'Falha ao carregar mercadores: $details';
  }

  @override
  String traderStockLineCount(int count) {
    return '$count linhas';
  }

  @override
  String get tabWorld => 'Mundo';

  @override
  String get tabCharacters => 'Personagens';

  @override
  String get characterNoActorBody =>
      'Este personagem não tem um ator no mundo, portanto não tem atributos, inventário ou eventos.';

  @override
  String get characterNoEventsBody => 'Sem eventos para este personagem.';

  @override
  String get characterOrphanGroup => 'Outros';

  @override
  String get tabAllData => 'Todos os dados';

  @override
  String get tabBackups => 'Cópias';

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
  String get equippedBadge => 'Equipado';

  @override
  String get armorUpgradesLabel => 'Melhorias';

  @override
  String get browse => 'Procurar';

  @override
  String get noSavFilesFound => 'Nenhum arquivo .sav encontrado';

  @override
  String get profile => 'Perfil';

  @override
  String get otherSaves => 'Outros jogos guardados';

  @override
  String profileWithSaves(String name, int count) {
    return '$name ($count jogos salvos)';
  }

  @override
  String get switchProfile => 'Trocar de perfil';

  @override
  String get openSaveFile => 'Abrir arquivo';

  @override
  String get externalSave => 'Jogo salvo aberto externamente';

  @override
  String get saveProfileTitle => 'Perfil do jogo salvo';

  @override
  String get saveProfileDescription =>
      'Atribua este jogo salvo a outro perfil do jogo. O jogo salvo e o índice de perfis serão copiados juntos.';

  @override
  String get saveProfileExternalHint =>
      'Selecione um perfil para importar este arquivo para a pasta de jogos salvos e registrá-lo. O arquivo original permanece inalterado.';

  @override
  String get saveProfileNoProfiles =>
      'Nenhum perfil de jogo editável foi encontrado em PersistentDataList.sav.';

  @override
  String get saveProfileSelect => 'Selecionar perfil';

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
  String bytesValue(String count) {
    return '$count bytes';
  }

  @override
  String get inspectionJsonTitle => 'JSON de inspeção';

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
  String get gameTimeTitle => 'Tempo de jogo';

  @override
  String get gameTimeDay => 'Dia';

  @override
  String get gameTimeHours => 'Horas';

  @override
  String get gameTimeMinutes => 'Minutos';

  @override
  String get gameTimeSeconds => 'Segundos';

  @override
  String gameTimeTotal(int seconds) {
    return '= $seconds s no total';
  }

  @override
  String get gameTimeInvalid =>
      'Insira números inteiros: dia ≥ 0, horas 0–23, minutos e segundos 0–59.';

  @override
  String get required => 'Obrigatório';

  @override
  String get playerLockedBody =>
      'Edições privadas do jogador exigem um codec capaz de comprimir.';

  @override
  String get heroTransform => 'Posição';

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
  String get spawnPositionSection => 'Posição de nascimento (referência)';

  @override
  String get resetToSpawnPosition => 'Repor na posição de nascimento';

  @override
  String get positionOutOfRange =>
      'O valor tem de estar entre −10 000 000 e 10 000 000';

  @override
  String get positionNotEditable =>
      'Não foi possível ler a posição guardada desta personagem, pelo que não pode ser editada.';

  @override
  String get positionNeverPlaced =>
      'Esta personagem nunca foi colocada no mundo (posição 0, 0, 0) — o jogo pode ignorar a posição guardada.';

  @override
  String get npcStayInPlace => 'Desativar a rotina diária dele';

  @override
  String get npcStayInPlaceHint => 'Fica então onde está.';

  @override
  String get npcStayInPlaceLocked =>
      'A rotina diária original dele não está registada, por isso já não é possível anular.';

  @override
  String get npcUndoPlacement => 'Anular a deslocação';

  @override
  String get npcUndoPlacementStale =>
      'O jogo guardado já não contém o que essa deslocação escreveu, por isso restaurá-la descartaria o que aconteceu entretanto.';

  @override
  String get positionNotReadable =>
      'Não foi possível ler a posição guardada desta personagem.';

  @override
  String get npcPositionReadOnly =>
      'O jogo restaura a posição de um NPC a partir do nível e não do jogo guardado, pelo que estes valores podem ser lidos, mas não alterados.';

  @override
  String get pickLocation => 'Escolher local…';

  @override
  String get pickLocationDialogTitle => 'Escolher um local';

  @override
  String get applySpotRotation => 'Aplicar também a orientação do local';

  @override
  String get locationAreaOther => 'Outros';

  @override
  String get locationAreaCavalornValley => 'Vale de Cavalorn';

  @override
  String get locationAreaEastForest => 'Floresta do Leste';

  @override
  String get locationAreaFogTower => 'Torre da Névoa';

  @override
  String get locationAreaIllegalWeedMixers => 'Misturadores ilegais de erva';

  @override
  String get locationAreaOrcArena => 'Arena dos Orcs';

  @override
  String get locationAreaOrcGraveyard => 'Cemitério dos Orcs';

  @override
  String get locationAreaShipwreck => 'Naufrágio';

  @override
  String get locationAreaTundra => 'Tundra';

  @override
  String get locationCatalogUnavailable =>
      'Não foi possível carregar o catálogo de locais.';

  @override
  String get invalid => 'Inválido';

  @override
  String get heroAttributes => 'Atributos do herói';

  @override
  String attributeBase(String name) {
    return 'Valor base de $name';
  }

  @override
  String attributeCurrent(String name) {
    return '$name atual';
  }

  @override
  String get attributeBaseValue => 'Valor base';

  @override
  String get attributeCurrentValue => 'Valor atual';

  @override
  String get inventoryTitle => 'Inventário';

  @override
  String get inventoryEmpty => 'Este inventário está vazio.';

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
  String get resetInventoryButton => 'Redefinir inventário';

  @override
  String get resetInventoryTooltipDefault =>
      'Substituir este inventário pelo inventário do início do jogo';

  @override
  String get resetInventoryTooltipBlocked =>
      'Salve ou cancele primeiro as alterações de inventário pendentes';

  @override
  String get pendingResetTitle =>
      'Redefinir para o inventário do início do jogo';

  @override
  String pendingResetSubtitle(String level) {
    return 'Nível de recursos: $level';
  }

  @override
  String get cancelPendingReset => 'Cancelar redefinição';

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
  String get itemCategoryMeleeWeapon => 'Armas corpo a corpo';

  @override
  String get itemCategoryRangedWeapon => 'Armas à distância';

  @override
  String get itemCategoryAmmunition => 'Munição';

  @override
  String get itemCategoryArmor => 'Armaduras';

  @override
  String get itemCategoryRune => 'Runas';

  @override
  String get itemCategoryScroll => 'Pergaminhos de magia';

  @override
  String get itemCategoryFood => 'Comida e poções';

  @override
  String get itemCategoryMisc => 'Diversos';

  @override
  String get itemCategoryAmulet => 'Amuletos';

  @override
  String get itemCategoryRing => 'Anéis';

  @override
  String get itemCategoryTrophy => 'Troféus de animais';

  @override
  String get itemCategoryWriting => 'Escritos';

  @override
  String get itemCategoryMission => 'Itens de missão';

  @override
  String get itemCategoryKey => 'Chaves';

  @override
  String get itemCategoryOther => 'Outros';

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
  String get npcStatusRowLabel => 'Estado';

  @override
  String get npcStatusAlive => 'vivo';

  @override
  String get npcStatusDead => 'morto';

  @override
  String get npcRelationshipRowLabel => 'Relação';

  @override
  String get npcRelationshipUnavailable => 'Estado da relação indisponível';

  @override
  String get npcRelationshipAutomatic => 'Calculada pelo jogo';

  @override
  String get npcRelationshipAutomaticHint =>
      'Nenhuma substituição permanente está armazenada. As regras de guilda, história, área e crimes são avaliadas no jogo.';

  @override
  String get npcRelationshipStoredHint =>
      'Armazenada como uma substituição permanente do NPC em relação ao jogador. As regras de guilda, história, área e crimes ainda podem alterar a relação efetiva no jogo.';

  @override
  String get npcRelationshipFriend => 'Amigo';

  @override
  String get npcRelationshipNeutral => 'Neutro';

  @override
  String get npcRelationshipEnemy => 'Inimigo';

  @override
  String npcRelationshipPending(String relationship) {
    return 'Será $relationship ao salvar';
  }

  @override
  String npcStateHp(String hp, String maxHp) {
    return 'PV $hp / $maxHp';
  }

  @override
  String get npcReviveButton => 'Reviver';

  @override
  String get npcReviveQueued => 'Será revivido ao guardar';

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
  String get memoryEventRemovalQueued =>
      'Remoção do evento na fila — pressione Salvar para aplicar.';

  @override
  String get duplicateEvent => 'Duplicar evento';

  @override
  String get duplicateMemoryEventTitle => 'Duplicar evento de memória?';

  @override
  String get duplicateMemoryEventBody =>
      'Duplicar este evento de memória? Um backup é gravado antes.';

  @override
  String get memoryEventDuplicationQueued =>
      'Duplicação do evento na fila — pressione Salvar para aplicar.';

  @override
  String get selectCharacterFromList => 'Selecione um personagem na lista';

  @override
  String get factionsSidebar => 'Facções';

  @override
  String get factionsForgiveButton => 'Perdoar';

  @override
  String get factionHostile => 'Hostil';

  @override
  String get factionFriendly => 'Amigável';

  @override
  String crimeMurder(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count homicídios',
      one: '$count homicídio',
    );
    return '$_temp0';
  }

  @override
  String crimeAssault(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count agressões',
      one: '$count agressão',
    );
    return '$_temp0';
  }

  @override
  String crimeTheft(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count furtos',
      one: '$count furto',
    );
    return '$_temp0';
  }

  @override
  String crimeTrespassing(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count invasões',
      one: '$count invasão',
    );
    return '$_temp0';
  }

  @override
  String crimeThreat(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count ameaças',
      one: '$count ameaça',
    );
    return '$_temp0';
  }

  @override
  String crimeOther(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count outros crimes',
      one: '$count outro crime',
    );
    return '$_temp0';
  }

  @override
  String get factionsForgiveQueued => 'a perdoar…';

  @override
  String get factionsEmpty => 'Sem crimes em aberto contra facções.';

  @override
  String get factionGuildOldCamp => 'Assentamento Antigo';

  @override
  String get factionGuildNewCamp => 'Assentamento Novo';

  @override
  String get factionGuildSwampCamp => 'Assentamento do Pântano';

  @override
  String get factionGuildOther => 'Outros/indivíduos';

  @override
  String get allDataLockedBody =>
      'O explorador exaustivo de fontes está atualmente disponível para ficheiros de gravação GSAV.';

  @override
  String get allDataDescription =>
      'Explore os metadados GSAV e todos os nós tipados PUBLIC/PRIVATE. Os valores escalares e as estruturas nativas seguras são editáveis; os contentores e os bytes opacos permanecem visíveis.';

  @override
  String get allDataEditable => 'Editável';

  @override
  String get allDataReadOnly => 'Só de leitura';

  @override
  String get allDataType => 'Tipo';

  @override
  String get allDataScalars => 'Escalares';

  @override
  String get allDataStructs => 'Estruturas';

  @override
  String get allDataContainers => 'Contentores';

  @override
  String get allDataOpaque => 'Opacos';

  @override
  String get allDataNodes => 'Nós';

  @override
  String allDataChildren(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count elementos filhos',
      one: '1 elemento filho',
    );
    return '$_temp0';
  }

  @override
  String get allDataPending => 'Pendente';

  @override
  String get allDataTagInputHint =>
      'Etiquetas separadas por vírgulas ou mudanças de linha';

  @override
  String allDataTypedSource(String source) {
    return 'Fonte tipada: $source';
  }

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
  String get backupsTitle => 'Cópias de segurança';

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
      'A versão portátil abre a página de download no navegador. Substitua os ficheiros atuais pela nova transferência.';

  @override
  String get updateAvailableTitle => 'Atualização disponível';

  @override
  String updateAvailableMessage(Object version, Object current) {
    return 'A versão $version está disponível. Tem a $current.';
  }

  @override
  String get updateDownload => 'Transferir';

  @override
  String get updateLater => 'Mais tarde';

  @override
  String get updateUpToDate => 'Está a usar a versão mais recente.';

  @override
  String get updateCheckFailed =>
      'Não foi possível procurar atualizações. Tente novamente mais tarde.';

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
  String aboutVersion(String version, String sha) {
    return 'Versão $version ($sha)';
  }

  @override
  String get aboutCopyright => '© 2026 colaboradores do GORE';

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
  String get presetNovice => 'Fácil';

  @override
  String get presetGothic => 'Gothic';

  @override
  String get presetHard => 'Difícil';

  @override
  String get presetCustom => 'Personalizada';

  @override
  String unrecognisedPreset(Object preset) {
    return 'A predefinição armazenada não é reconhecida ($preset). Você ainda pode salvar alterações de Assistente de Fluência / Permadeath, ou escolher uma predefinição acima para sobrescrevê-la.';
  }

  @override
  String get closeCombatFlowHelper =>
      'Assistência de Fluidez de Combate de Curto Alcance';

  @override
  String get permadeath => 'Morte Permanente';

  @override
  String get notAvailableOnNovice => 'Indisponível no Iniciante';

  @override
  String get levelCombat => 'Combate';

  @override
  String get levelResources => 'Recursos';

  @override
  String get levelProgression => 'Progressão';

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
  String get heroEntryHeroTransform => 'Posição';

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

  @override
  String savingProgress(int done, int total) {
    return 'Salvando… $done de $total';
  }

  @override
  String localizedTextExtractedCount(int idCount, int languageCount) {
    return '$idCount IDs extraídos em $languageCount idiomas';
  }

  @override
  String get skillSmithing1H => 'Ferraria de Armas de Uma Mão';

  @override
  String get skillSmithing2H => 'Ferraria de Armas de Duas Mãos';

  @override
  String get skillCircleNovice => 'Mago Iniciado';

  @override
  String get skillCircle1 => 'Primeiro Círculo de Magia';

  @override
  String get skillCircle2 => 'Segundo Círculo de Magia';

  @override
  String get skillCircle3 => 'Terceiro Círculo de Magia';

  @override
  String get skillCircle4 => 'Quarto Círculo de Magia';

  @override
  String get skillCircle5 => 'Quinto Círculo de Magia';

  @override
  String get skillCircle6 => 'Sexto Círculo de Magia';

  @override
  String get sectionGlossary => 'Glossário';

  @override
  String get glossarySearch => 'Pesquisar no glossário';

  @override
  String get glossaryOldCamp => 'Assentamento Antigo';

  @override
  String get glossaryNewCamp => 'Assentamento Novo';

  @override
  String get glossarySwampCamp => 'Assentamento do Pântano';

  @override
  String get glossaryOutsiders => 'Forasteiros';

  @override
  String get glossaryCreatures => 'Criaturas';

  @override
  String get glossaryLocations => 'Locais';

  @override
  String get glossaryFilterLabel => 'Filtro';

  @override
  String get glossaryFilterTraders => 'Comerciantes';

  @override
  String get glossaryFilterTeachers => 'Professores';

  @override
  String get glossaryFilterArmorers => 'Armeiros';

  @override
  String get glossaryFilterHostile => 'Hostis';

  @override
  String get glossaryRelationshipFilterNote =>
      'Mostra as substituições permanentes de inimigo armazenadas no jogo salvo. As relações dinâmicas de guilda, história, área e crimes são calculadas apenas no jogo.';

  @override
  String get glossaryFilterDead => 'Mortos';

  @override
  String get glossaryAddEntry => 'Adicionar entrada ao glossário';

  @override
  String get glossaryAddTitle => 'Adicionar entrada ao glossário';

  @override
  String get glossaryResetChanges => 'Redefinir alterações do glossário';

  @override
  String get glossaryNoVisibleEntries =>
      'Nenhuma entrada visível do glossário corresponde a esta visualização.';

  @override
  String get glossaryNoHiddenEntries =>
      'Todas as entradas disponíveis já estão visíveis.';

  @override
  String get glossaryNoMatch => 'Nenhuma entrada do glossário corresponde.';

  @override
  String get glossarySelectEntry =>
      'Selecione uma entrada do glossário para editar suas seções.';

  @override
  String glossaryEntryCount(int count) {
    return '$count entradas';
  }

  @override
  String glossarySegmentsCount(int unlocked, int total) {
    return '$unlocked de $total entradas';
  }

  @override
  String get glossaryPortraitUnlocked => 'Retrato desbloqueado';

  @override
  String get glossaryPortraitSilhouette =>
      'Silhueta — retrato não desbloqueado';

  @override
  String get glossarySegments => 'Entradas';

  @override
  String get glossaryPending => 'Alteração não salva';

  @override
  String get glossaryShowFullText => 'Mostrar o texto completo da entrada';

  @override
  String get glossarySegmentIntroduction => 'Introdução / retrato';

  @override
  String get glossarySegmentUnlock => 'Descoberta';

  @override
  String glossarySegmentEntry(int number) {
    return 'Entrada $number';
  }

  @override
  String get questJournalAll => 'Todas as missões';

  @override
  String get questJournalOldCamp => 'Assentamento Antigo';

  @override
  String get questJournalNewCamp => 'Assentamento Novo';

  @override
  String get questJournalSwampCamp => 'Assentamento do Pântano';

  @override
  String get questJournalColony => 'A Colônia';

  @override
  String get questJournalCompleted => 'Concluídas';

  @override
  String get questJournalHint =>
      'Visualização do diário no jogo. Estados internos e missões ainda não iniciadas continuam disponíveis em Todos os dados.';

  @override
  String get questJournalNoEntries =>
      'Nenhuma missão do diário corresponde aos filtros atuais.';

  @override
  String get glossaryTutorials => 'Tutoriais';

  @override
  String get tutorialGateNote =>
      'Estas linhas controlam os desbloqueios de tutorial armazenados. Um desbloqueio não corresponde necessariamente a uma única página de tutorial no jogo.';

  @override
  String get tutorialResetChanges => 'Redefinir alterações dos tutoriais';

  @override
  String get tutorialNoGates =>
      'Nenhum desbloqueio de tutorial está disponível neste jogo salvo.';

  @override
  String tutorialGateUnlockCount(int unlocked, int total) {
    return '$unlocked de $total tutoriais desbloqueados';
  }

  @override
  String get tutorialGateCombatBasics => 'Noções básicas de combate';

  @override
  String get tutorialGateCrafting => 'Criação';

  @override
  String get tutorialGateCrime => 'Crimes e consequências';

  @override
  String get tutorialGateDrugs => 'Consumíveis e efeitos';

  @override
  String get tutorialGateLockpicking => 'Arrombamento';

  @override
  String get tutorialGateMagic => 'Magia';

  @override
  String get tutorialGateMap => 'Mapa';

  @override
  String get tutorialGateMeleeCombat => 'Combate corpo a corpo';

  @override
  String get tutorialGateNavigation => 'Movimento e navegação';

  @override
  String get tutorialGatePerception => 'Percepção';

  @override
  String get tutorialGatePlayerProgression => 'Progressão do personagem';

  @override
  String get tutorialGateRanged => 'Combate à distância';

  @override
  String get tutorialGateRiding => 'Montaria';

  @override
  String get tutorialGateSleep => 'Dormir';

  @override
  String get tutorialGateTrading => 'Comércio';

  @override
  String get windowMinimizeTooltip => 'Minimizar';

  @override
  String get windowMaximizeTooltip => 'Maximizar';

  @override
  String get windowRestoreTooltip => 'Restaurar';

  @override
  String get fallbackDialogEntry => 'Entrada de diálogo';

  @override
  String get fallbackDialogChoice => 'Escolha de diálogo';

  @override
  String get fallbackDialogTopic => 'Tópico de diálogo';

  @override
  String get fallbackDialogInformation => 'Informação de diálogo';

  @override
  String get fallbackQuest => 'Missão';

  @override
  String get fallbackObjective => 'Objetivo';

  @override
  String get fallbackItem => 'Item';

  @override
  String get attributeSkillPointsFallback => 'Pontos de aprendizado (PA)';

  @override
  String attributeManualFallbackLabel(String attributeId, String fallback) {
    String _temp0 = intl.Intl.selectLogic(attributeId, {
      'Alcohol': 'Álcool',
      'AlcoholDepletionRate': 'Taxa de redução do álcool',
      'MaxAlcohol': 'Nível máximo de álcool',
      'MaxSuperArmor': 'Superarmadura máxima',
      'SuperArmor': 'Superarmadura',
      'Fatigue': 'Fadiga',
      'FillRatio': 'Proporção de preenchimento',
      'FillRatioPeriod': 'Período de preenchimento',
      'MaxFatigue': 'Fadiga máxima',
      'MaxThresholdIndex': 'Índice máximo de limiar',
      'RecoveryRatePerHourOfSleep': 'Recuperação por hora de sono',
      'DamageMultiplier': 'Multiplicador de dano',
      'Toughness': 'Tenacidade',
      'ToughnessA': 'Tenacidade A',
      'ToughnessB': 'Tenacidade B',
      'ToughnessC': 'Tenacidade C',
      'XPExecutedBounty': 'Recompensa de XP por execução',
      'XPKillOrDefeatBounty': 'Recompensa de XP por morte ou derrota',
      'SpeedModifier': 'Modificador de velocidade',
      'CriticalLevelPercent': 'Nível crítico (%)',
      'MaxOxygen': 'Oxigênio máximo',
      'Oxygen': 'Oxigênio',
      'OxygenDepletionRate': 'Taxa de consumo de oxigênio',
      'OxygenRecoveryRate': 'Taxa de recuperação de oxigênio',
      'MaxRestTime': 'Tempo máximo de descanso',
      'MaxSleepTime': 'Tempo máximo de sono',
      'SleepTime': 'Tempo de sono',
      'SleepTimeRecoveryAmount': 'Quantidade recuperada durante o sono',
      'SleepTimeRecoveryPeriod': 'Intervalo de recuperação durante o sono',
      'MaxSwampweed': 'Quantidade máxima de erva do pântano',
      'Swampweed': 'Erva do pântano',
      'SwampweedDepletionRate': 'Taxa de consumo de erva do pântano',
      'other': '$fallback',
    });
    return '$_temp0';
  }

  @override
  String get knowledgeTypeVoiceLine => 'Linha de voz';

  @override
  String get knowledgeTypeOther => 'Outro';

  @override
  String get armorUpgradeUpper => 'Superior';

  @override
  String get armorUpgradeMiddle => 'Central';

  @override
  String get armorUpgradeLower => 'Inferior';

  @override
  String get knowledgeCategoryTopic => 'Tópico';

  @override
  String get knowledgeCategoryChoice => 'Escolha';

  @override
  String get knowledgeCategoryInfo => 'Informação';

  @override
  String get statusOk => 'OK';

  @override
  String get statusFailed => 'Falha';

  @override
  String get missingSaveReference => 'Arquivo ausente';

  @override
  String missingSaveReferenceDescription(String slot) {
    return '$slot.sav está ausente. O arquivo pode ter sido excluído, movido ou renomeado; o perfil ainda faz referência a ele.';
  }

  @override
  String get removeFromProfile => 'Remover do perfil';

  @override
  String get removeSaveFromProfileTitle => 'Remover jogo salvo do perfil?';

  @override
  String removeSaveFromProfileBody(String save, String profile) {
    return 'Remover $save de $profile? O próprio arquivo de jogo salvo será mantido se ainda existir.';
  }

  @override
  String get unassignedSave => 'Não atribuído a um perfil';

  @override
  String get armorUpgradeLight => 'Leve';

  @override
  String get armorUpgradeMedium => 'Média';

  @override
  String get armorUpgradeHeavy => 'Pesada';

  @override
  String get knowledgeCaptionForcedConversation => 'Conversa forçada';

  @override
  String get knowledgeCaptionFollowupTopic => 'Tópico de acompanhamento';

  @override
  String get knowledgeCaptionFallbackTopic => 'Tópico alternativo';

  @override
  String durationMinutes(int minutes) {
    return '$minutes min';
  }

  @override
  String durationHours(int hours) {
    return '$hours h';
  }

  @override
  String durationHoursMinutes(int hours, int minutes) {
    return '$hours h $minutes min';
  }

  @override
  String get backupStatusInvalidProfileStructure => 'Dados de perfil inválidos';

  @override
  String get backupStatusSlotMetadataMissing =>
      'Os metadados do jogo salvo selecionado estão ausentes';

  @override
  String defaultProfileName(int id) {
    return 'Perfil $id';
  }

  @override
  String get statusUnknown => 'Desconhecido';

  @override
  String editorUnexpectedError(String details) {
    return 'Erro inesperado: $details';
  }

  @override
  String get editorOperationInProgress =>
      'Está em curso outra operação. Tente novamente dentro de momentos.';

  @override
  String get editorUnsavedBeforeDifficulty =>
      'O jogo guardado contém alterações por guardar. Guarde-as ou reponha-as antes de alterar a dificuldade do perfil.';

  @override
  String get editorNoSaveFolderSelected =>
      'Nenhuma pasta de jogos guardados selecionada.';

  @override
  String get editorNoSaveSelected => 'Nenhum jogo guardado selecionado.';

  @override
  String get coreUnknownError => 'Erro interno desconhecido';

  @override
  String get editorUnsavedBeforeSwitchProfile =>
      'Primeiro, guarde ou reponha as alterações pendentes — mudar de perfil implica deixar o jogo guardado atual.';

  @override
  String get editorUnsavedBeforeOpenFile =>
      'Guarde ou reponha as alterações pendentes antes de abrir outro ficheiro.';

  @override
  String get editorSelectSavFile =>
      'Selecione um ficheiro .sav de jogo guardado.';

  @override
  String get editorNotGothicGsav =>
      'O ficheiro selecionado não é um jogo guardado Gothic GSAV.';

  @override
  String get editorUnsavedBeforeChangeSaveProfile =>
      'Guarde ou reponha as alterações pendentes antes de alterar o perfil do jogo guardado.';

  @override
  String get editorUnsavedBeforeRemoveProfile =>
      'Guarde ou reponha as alterações pendentes antes de remover um jogo guardado do respetivo perfil.';

  @override
  String get editorUnsavedBeforeRestoreProfile =>
      'O jogo guardado contém alterações por guardar. Guarde-as ou reponha-as antes de restaurar uma cópia de segurança do perfil.';

  @override
  String editorConflictingPropertyEdits(String path) {
    return 'As alterações pendentes de dois separadores afetam a mesma propriedade ($path). Reponha ou anule uma delas e volte a guardar.';
  }

  @override
  String editorGlossaryMemoryConflict(String path) {
    return 'Uma alteração de segmento do Glossário e outra alteração pendente em «Todos os dados» afetam a matriz Hero MemorizedEvents ($path). As alterações do Glossário adicionam ou removem entradas desta matriz, pelo que não podem ser guardadas em conjunto. Reponha ou anule uma delas e volte a guardar.';
  }

  @override
  String editorGlossaryQuestConflict(String path) {
    return 'Uma alteração de segmento do Glossário e outra alteração pendente afetam a mesma propriedade CurrentState de uma missão ($path). A própria alteração do Glossário atualiza esse estado. Reponha ou anule uma delas e volte a guardar.';
  }

  @override
  String editorRelationshipConflict(String path) {
    return 'Uma alteração de relação e outra alteração pendente em «Todos os dados» afetam a mesma entrada de relação de um NPC ($path). A alteração estruturada da relação pode substituir modificadores nessa entrada, pelo que não podem ser guardadas em conjunto. Reponha ou anule uma delas e volte a guardar.';
  }

  @override
  String editorMultipleStructuralArrayEdits(String path) {
    return 'Há mais do que uma alteração estrutural pendente para a mesma matriz ($path). Guarde ou reponha a primeira alteração antes de adicionar outra.';
  }

  @override
  String editorStructuralArrayConflict(String path) {
    return 'Uma alteração estrutural de evento e outra alteração pendente em «Todos os dados» afetam $path. Guarde ou reponha uma delas antes de continuar.';
  }

  @override
  String get editorSkillsEffectConflict =>
      'Estão pendentes uma alteração em «Aptidões» e uma alteração em «Todos os dados» no mesmo efeito da personagem (ActiveEffects › EffectSpec › Def). Não podem ser guardadas em conjunto. Reponha ou anule uma delas e volte a guardar.';

  @override
  String get editorInventoryResetConflict =>
      'Estão pendentes uma reposição do inventário e outra alteração ao mesmo inventário. A reposição substitui todo o inventário e descartaria a outra alteração. Reponha ou anule uma delas e volte a guardar.';

  @override
  String get editorUseFolder => 'Usar pasta';

  @override
  String get editorGothicSavegameFileType => 'Jogo guardado de Gothic';

  @override
  String get editorNoDifficultyChanges =>
      'Não há alterações de dificuldade para guardar';

  @override
  String get editorDifficultyWritten =>
      'Dificuldade guardada no perfil (cópia de segurança criada)';

  @override
  String editorChangesSavedWithBackup(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count alterações guardadas com uma cópia de segurança',
      one: '1 alteração guardada com uma cópia de segurança',
    );
    return '$_temp0';
  }

  @override
  String editorPlacementNoteFailed(String details) {
    return 'A deslocação foi guardada, mas não foi possível escrever a sua nota de anulação: $details';
  }

  @override
  String editorProfileNotFound(int profileId) {
    return 'O perfil $profileId não foi encontrado.';
  }

  @override
  String get editorNoFreeSaveSlot =>
      'Não há nenhum slot de gravação livre na pasta de jogos guardados (G1R-001 a G1R-999).';

  @override
  String editorSaveImportedAssigned(int profileId) {
    return 'Jogo guardado importado e atribuído ao perfil $profileId';
  }

  @override
  String editorSaveAssigned(int profileId) {
    return 'Jogo guardado atribuído ao perfil $profileId (cópias de segurança associadas criadas)';
  }

  @override
  String editorSaveSlotNotAssigned(String slot, int profileId) {
    return 'O slot de gravação $slot não está atribuído ao perfil $profileId.';
  }

  @override
  String get editorSaveRemovedFromProfile => 'Jogo guardado removido do perfil';

  @override
  String editorRestoredBackup(String path) {
    return 'Cópia de segurança restaurada: $path';
  }

  @override
  String editorRestoredBackupWithoutCompanion(String path) {
    return 'Cópia de segurança restaurada: $path (PersistentDataList.sav ficou inalterado por não existir uma cópia associada correspondente; os metadados do espaço de gravação podem diferir)';
  }

  @override
  String editorCodecRoundtripPassed(int chunkIndex, int bytes) {
    return 'Validação completa do codec concluída: o bloco $chunkIndex foi novamente comprimido para $bytes bytes';
  }

  @override
  String editorDifficultyWriteFailed(String details) {
    return 'Não foi possível guardar a dificuldade do perfil: $details';
  }

  @override
  String editorProfileAssignmentFailed(String details) {
    return 'Não foi possível atribuir o jogo guardado ao perfil: $details';
  }

  @override
  String editorProfileRemovalFailed(String details) {
    return 'Não foi possível remover o jogo guardado do perfil: $details';
  }

  @override
  String editorSaveFailed(String details) {
    return 'Não foi possível guardar as alterações: $details';
  }

  @override
  String editorScanSavesFailed(String details) {
    return 'Não foi possível analisar os jogos guardados: $details';
  }

  @override
  String editorInspectSaveFailed(String details) {
    return 'Não foi possível inspecionar o jogo guardado: $details';
  }

  @override
  String editorLoadBackupsFailed(String details) {
    return 'Não foi possível carregar as cópias de segurança: $details';
  }

  @override
  String editorRestoreFailed(String details) {
    return 'Não foi possível restaurar a cópia de segurança: $details';
  }

  @override
  String editorRestoreReloadFailed(String path, String details) {
    return 'Cópia de segurança restaurada: $path, mas não foi possível recarregar o jogo guardado: $details';
  }

  @override
  String editorCodecCheckFailed(String details) {
    return 'A verificação do codec falhou: $details';
  }

  @override
  String editorCodecValidationFailed(String details) {
    return 'A validação completa do codec falhou: $details';
  }

  @override
  String editorPropertySearchFailed(String details) {
    return 'A pesquisa de propriedades falhou: $details';
  }

  @override
  String get editorSelectionChangedWhileLoadingHeroAttributes =>
      'A seleção do jogo guardado mudou durante o carregamento dos atributos do herói.';

  @override
  String editorSkillsLoadFailed(String details) {
    return 'O carregamento das aptidões falhou: $details';
  }

  @override
  String editorProgressionQueryFailed(String details) {
    return 'A consulta de progressão falhou: $details';
  }

  @override
  String editorNpcListFailed(String details) {
    return 'O carregamento da lista de NPCs falhou: $details';
  }

  @override
  String editorCharacterListFailed(String details) {
    return 'O carregamento da lista de personagens falhou: $details';
  }

  @override
  String editorNpcAttributesFailed(String details) {
    return 'O carregamento dos atributos do NPC falhou: $details';
  }

  @override
  String editorNpcPositionFailed(String details) {
    return 'O carregamento da posição do NPC falhou: $details';
  }

  @override
  String editorNpcInventoryFailed(String details) {
    return 'O carregamento do inventário do NPC falhou: $details';
  }

  @override
  String editorFactionListFailed(String details) {
    return 'O carregamento da lista de facções falhou: $details';
  }

  @override
  String get editorNoBackupPath => 'nenhuma';

  @override
  String editorBackupMessage(String prefix, String backupPath) {
    return '$prefix: $backupPath';
  }

  @override
  String editorBackupMessageWithPersistent(
    String prefix,
    String backupPath,
    String persistentPath,
  ) {
    return '$prefix: $backupPath; cópia de segurança de PersistentDataList: $persistentPath';
  }

  @override
  String localizationStatusFailed(String details) {
    return 'Não foi possível obter o estado da localização: $details';
  }

  @override
  String localizationExtractionFailed(String details) {
    return 'A extração falhou: $details';
  }

  @override
  String glossaryLoadFailed(String details) {
    return 'O carregamento do Glossário falhou: $details';
  }

  @override
  String backupStatusError(String details) {
    return 'Erro da cópia de segurança: $details';
  }

  @override
  String memoryEventCategory(String category, String fallback) {
    String _temp0 = intl.Intl.selectLogic(category, {
      'quest': 'Missão',
      'document': 'Documento',
      'story': 'História',
      'exploration': 'Exploração',
      'combat': 'Combate',
      'social': 'Social',
      'item': 'Itens',
      'learning': 'Aprendizagem',
      'guild': 'Guilda',
      'crime': 'Crime',
      'rest': 'Descanso',
      'other': '$fallback',
    });
    return '$_temp0';
  }

  @override
  String memoryEventAction(String kind, String fallback) {
    String _temp0 = intl.Intl.selectLogic(kind, {
      'questStarted': 'Missão iniciada',
      'questSucceeded': 'Missão concluída',
      'questFailed': 'Missão falhada',
      'documentRead': 'Documento lido',
      'documentSegmentUnlocked': 'Entrada descoberta',
      'documentSegmentViewed': 'Entrada visualizada',
      'chapterCompleted': 'Capítulo concluído',
      'areaEntered': 'Entrada na área',
      'areaLeft': 'Saída da área',
      'characterKilled': 'Personagem morto',
      'characterDefeated': 'Personagem derrotado',
      'combatDodge': 'Ataque esquivado',
      'characterDebuffed': 'Penalização aplicada',
      'tradeAvailable': 'Comércio desbloqueado',
      'itemObtained': 'Item obtido',
      'itemCrafted': 'Item criado',
      'skillStateRecorded': 'Estado das habilidades registado',
      'recipeLearned': 'Receita aprendida',
      'guildJoined': 'Adesão à guilda',
      'crimeRecorded': 'Crime registado',
      'slept': 'Sono',
      'storyEvent': 'Evento da história',
      'other': '$fallback',
    });
    return '$_temp0';
  }

  @override
  String memoryEventTitleWithSubject(String action, String subject) {
    return '$action: $subject';
  }

  @override
  String memoryEventFact(String fact, String fallback) {
    String _temp0 = intl.Intl.selectLogic(fact, {
      'gameTime': 'Tempo de jogo',
      'duration': 'Duração',
      'chapter': 'Capítulo',
      'instigator': 'Iniciado por',
      'affected': 'Afetado',
      'amount': 'Quantidade',
      'primaryObject': 'Objeto',
      'secondaryObject': 'Contexto',
      'segmentText': 'Texto da entrada',
      'other': '$fallback',
    });
    return '$_temp0';
  }

  @override
  String memoryEventGameTime(int day, String time) {
    return 'Dia $day, $time';
  }

  @override
  String memoryEventSecondsValue(String value) {
    return '$value s';
  }

  @override
  String memoryEventMoreValues(String values, int count) {
    return '$values +$count';
  }

  @override
  String get memoryEventHero => 'Herói';

  @override
  String get memoryEventDetails => 'Detalhes';

  @override
  String get memoryEventTags => 'Etiquetas';

  @override
  String get memoryEventTechnicalData => 'Dados técnicos';

  @override
  String get memoryEventIndex => 'Índice';

  @override
  String get memoryEventPosition => 'Posição';

  @override
  String get memoryEventPayload => 'Conteúdo';

  @override
  String get memoryEventSubject => 'Assunto';

  @override
  String glossaryCatalogSegmentLabel(String segmentId, String fallback) {
    String _temp0 = intl.Intl.selectLogic(segmentId, {
      'Access': 'Acesso',
      'AccessDenied': 'Acesso negado',
      'AccesToTemple': 'Acesso ao templo',
      'Advice': 'Conselho',
      'AfterFight': 'Depois da luta',
      'AfterFireMages': 'Depois dos Magos do Fogo',
      'AfterNek': 'Depois de Nek',
      'AfterQuest': 'Depois da missão',
      'Alone': 'Sozinho',
      'Amulet': 'Amuleto',
      'Annoying': 'Irritante',
      'Armor': 'Armadura',
      'Avoid': 'Evitar',
      'Backstory': 'História',
      'BackStory': 'História',
      'BasicMagic': 'Magia básica',
      'Beated': 'Derrotado',
      'BecomeMercenary': 'Tornar-se mercenário',
      'Beer': 'Cerveja',
      'Bestiary': 'Bestiário',
      'Blessing': 'Bênção',
      'Boss': 'Chefe',
      'Bully': 'Valentão',
      'BullyAdvice': 'Conselho sobre o valentão',
      'Camp': 'Acampamento',
      'CampDivided': 'Acampamento dividido',
      'CareOfMessengers': 'Cuidar dos mensageiros',
      'ChangeOpinion': 'Mudar de opinião',
      'ChargeUriziel': 'Carregar Uriziel',
      'Chosen': 'Escolhido',
      'Contact': 'Contato',
      'Courier': 'Mensageiro',
      'CraftBows': 'Fabricação de arcos',
      'Crazy': 'Louco',
      'DailyMeal': 'Refeição diária',
      'DailyRation_Trader': 'Comerciante de ração diária',
      'DAM': 'Barragem',
      'Dead': 'Morto',
      'Deal': 'Acordo',
      'Dealer': 'Negociante',
      'Deceived': 'Enganado',
      'Dementia': 'Demência',
      'DenyAccess': 'Acesso negado',
      'DifferentOpinion': 'Opinião diferente',
      'Discussion': 'Discussão',
      'DontTalk': 'Não conversar',
      'Duel': 'Duelo',
      'Entrance': 'Entrada',
      'Escape': 'Fuga',
      'Extended': 'Estendido',
      'Extra': 'Extra',
      'ExtraInfo': 'Informação adicional',
      'Fanatic': 'Fanático',
      'Fight': 'Luta',
      'FindUlumulu': 'Encontrar Ulu-Mulu',
      'FireMages': 'Magos do Fogo',
      'FireMagesEscape': 'Fuga dos Magos do Fogo',
      'FiskNewDealer': 'Novo receptador para Fisk',
      'FiskNewDealerCompleted': 'Novo receptador para Fisk — concluído',
      'FogTower': 'Torre da Névoa',
      'Food': 'Comida',
      'Forgave': 'Perdoou',
      'Forgive': 'Perdoar',
      'Forgiven': 'Perdoado',
      'FourFriends': 'Quatro amigos',
      'FreeHut': 'Cabana disponível',
      'FreeMine': 'Mina Livre',
      'Fury': 'Fúria',
      'GoodTeacher': 'Bom treinador',
      'Gossip': 'Fofoca',
      'GotScavenger': 'Catador obtido',
      'GrantedAccess': 'Acesso concedido',
      'GRDArmor': 'Armadura de guarda',
      'Guide': 'Guia',
      'HateMages': 'Ódio aos magos',
      'HateMagesExplanation': 'Motivo do ódio aos magos',
      'HateRiceLord': 'Ódio ao Lorde do Arroz',
      'Heal': 'Cura',
      'Healing': 'Cura',
      'Help': 'Ajuda',
      'Helper': 'Ajudante',
      'HelpKagan': 'Ajudar Kagan',
      'HutStory': 'História da cabana',
      'Ignore': 'Ignorar',
      'Impress': 'Impressionar',
      'ImpressAlchemy': 'Impressionar — alquimia',
      'ImpressInscription': 'Impressionar — inscrições',
      'Info': 'Informação',
      'Interested': 'Interessado',
      'Introduction': 'Apresentação',
      'Introduction_2': 'Apresentação 2',
      'Introduction_Armor': 'Apresentação de armaduras',
      'Introduction_Teacher': 'Apresentação — treinador',
      'Introduction_Trader': 'Apresentação — comerciante',
      'Invocation': 'Invocação',
      'JoinSC': 'Entrada no Assentamento do Pântano',
      'Joint': 'Cigarro de maconha-do-pântano',
      'KalomCamp': 'Acampamento de Cor Kalom',
      'Leader': 'Líder',
      'Learning': 'Aprendizado',
      'LearnOrcish': 'Aprender a língua dos orcs',
      'LeftParty': 'Saiu do grupo',
      'Library': 'Biblioteca',
      'Lie': 'Mentira',
      'Lock': 'Fechadura',
      'Lockpick': 'Gazua',
      'Mad': 'Insano',
      'Mandibles': 'Mandíbulas',
      'MapMaker': 'Cartógrafo',
      'Monastery': 'Mosteiro',
      'MordragKO': 'Mordrag nocauteado',
      'Nek': 'Nek',
      'NewCamp': 'Assentamento Novo',
      'NewCamper': 'Novo integrante do acampamento',
      'NewLeader': 'Novo líder',
      'NightPatrol': 'Patrulha noturna',
      'NotInterested': 'Sem interesse',
      'OldCamp': 'Assentamento Antigo',
      'OrcEnclaveEntrance': 'Entrada do Enclave dos Orcs',
      'OrcGraveyard': 'Cemitério dos orcs',
      'OreArmor': 'Armadura de Minério',
      'Party': 'Grupo',
      'Pay': 'Pagamento',
      'PayMoney': 'Pagar',
      'Permission': 'Permissão',
      'Pet': 'Mascote',
      'PreparingInvocation': 'Preparação da invocação',
      'Quest': 'Missão',
      'RankUpFireMages': 'Promoção a Mago do Fogo',
      'RankUpGuard': 'Promoção a guarda',
      'RanUpFireMagesCompleted': 'Promoção a Mago do Fogo concluída',
      'Realocated': 'Realocado',
      'Reason': 'Motivo',
      'Respect': 'Respeito',
      'ReturnToSC': 'Retorno ao Assentamento do Pântano',
      'RicelordForeman': 'Capataz do Lorde do Arroz',
      'RideScavenger': 'Montar um catador',
      'Robe': 'Túnica',
      'Safe': 'Em segurança',
      'Scraper': 'Sucateiro',
      'SecondChance': 'Segunda chance',
      'SecretLocation': 'Local secreto',
      'SecretPassage': 'Passagem secreta',
      'SecretPath': 'Caminho secreto',
      'SleeperFollower': 'Seguidor do Adormecido',
      'SleeperTemple': 'Templo do Adormecido',
      'SmallInfo': 'Pequena informação',
      'Stonehenge': 'Círculo de Pedra',
      'StopFollowing': 'Parar de seguir',
      'SwampCamp': 'Assentamento do Pântano',
      'Talkative': 'Falante',
      'Teach': 'Treinamento',
      'TeachBow': 'Treino com arco',
      'Teacher': 'Treinador',
      'Teacher2': 'Treinador 2',
      'TeacherInscription': 'Treinador de inscrições',
      'TeacherMana': 'Treinador de mana',
      'TeachIchor': 'Treino de icor',
      'TeachMagic': 'Treino de magia',
      'TeachOrcish': 'Ensinar a língua dos orcs',
      'TeachStats': 'Treino de atributos',
      'TeachWeapon': 'Treino com armas',
      'Teleport': 'Teleporte',
      'TheMysteriousOrc': 'O orc misterioso',
      'ThroneRoom': 'Sala do trono',
      'TradeBow': 'Comércio de arcos',
      'Trader': 'Comerciante',
      'TradeSkins_Trader': 'Comerciante de peles',
      'Traitor': 'Traidor',
      'Trial': 'Provação',
      'TrollCanyon': 'Cânion do Troll',
      'Trust': 'Confiança',
      'Ulumulu': 'Ulu-Mulu',
      'Unexperienced': 'Inexperiente',
      'Uriziel': 'Uriziel',
      'UrizielRune': 'Runa de Uriziel',
      'Useful': 'Útil',
      'Velaya': 'Velaya',
      'Vibrations': 'Vibrações',
      'WaitFreeMine': 'Espera na Mina Livre',
      'WaitInTrainingArea': 'Espera na área de treino',
      'Warning': 'Aviso',
      'WarningTooLate': 'Aviso tardio',
      'WaterMessenger': 'Mensageiro dos Magos da Água',
      'Weapon': 'Arma',
      'Who': 'Quem é',
      'Women': 'Mulheres',
      'other': '$fallback',
    });
    return '$_temp0';
  }

  @override
  String get slotRepairTitle => 'Espaços de inventário danificados';

  @override
  String slotRepairBody(int count) {
    return 'Este jogo salvo tem $count espaços de inventário cujo id já não corresponde à sua posição — no jogo, largar um desses itens remove outro. O reparo apenas reescreve os ids: nenhum item é adicionado, removido ou alterado. Ao salvar, um backup é criado, como sempre.';
  }

  @override
  String get slotRepairQueued => 'Reparo na fila — salve para aplicar.';

  @override
  String get slotRepairAction => 'Reparar';

  @override
  String get slotRepairDiscard => 'Descartar';

  @override
  String get editorInventorySlotEditConflict =>
      'Uma edição direta de um espaço de inventário está na fila junto com uma operação que ocupa espaços inteiros (reparo, adição ou remoção). A segunda sobrescreveria a primeira — reverta uma delas e salve novamente.';

  @override
  String get editorTraderArrayConflict =>
      'Uma alteração de comércio está em fila junto com uma edição direta da matriz de mercadores. Essa edição renumera as linhas pelas quais uma alteração de comércio é endereçada, por isso uma das duas cairia no mercador errado — reverte uma e grava de novo.';

  @override
  String get backupFactFile => 'Arquivo';

  @override
  String get renameBackupTooltip => 'Nomear este backup';

  @override
  String get renameBackupTitle => 'Nomear backup';

  @override
  String get renameBackupLabel => 'Nome';

  @override
  String renameBackupHelp(String fileName) {
    return 'Exibido no lugar do nome do arquivo $fileName. Deixe vazio para remover o nome; o arquivo em si não é renomeado.';
  }

  @override
  String get deleteBackupTooltip => 'Excluir este backup';

  @override
  String get deleteBackupTitle => 'Excluir backup';

  @override
  String deleteBackupBody(String name, String fileName) {
    return 'Excluir “$name” ($fileName)? O arquivo é removido do disco e não pode ser recuperado.';
  }

  @override
  String get deleteBackupConfirm => 'Excluir';

  @override
  String editorDeletedBackup(String path) {
    return 'Backup excluído: $path';
  }

  @override
  String editorDeleteBackupFailed(String details) {
    return 'Não foi possível excluir o backup: $details';
  }

  @override
  String editorRenameBackupFailed(String details) {
    return 'Não foi possível nomear o backup: $details';
  }

  @override
  String get slotRepairUnavailable =>
      'No momento não é possível reparar — este jogo salvo não pode ser gravado.';

  @override
  String editorDeletedBackupWithLabelWarning(String path, String details) {
    return 'Backup excluído: $path — não foi possível remover o nome dele: $details';
  }

  @override
  String get slotRepairNotOffered =>
      'O reparo não está disponível para este jogo salvo.';
}

/// The translations for Portuguese, as used in Brazil (`pt_BR`).
class AppLocalizationsPtBr extends AppLocalizationsPt {
  AppLocalizationsPtBr() : super('pt_BR');

  @override
  String get debugSectionTitle => 'Avançado (depuração)';

  @override
  String get debugSectionSubtitle =>
      'Diagnóstico e dados brutos para relatórios de bugs';

  @override
  String get showObjectIdsTitle => 'Mostrar IDs técnicos adicionais';

  @override
  String get showObjectIdsSubtitle =>
      'Mostra IDs técnicos de itens, conhecimento de diálogo, missões e atores órfãos. IDs de NPCs são sempre exibidos.';

  @override
  String get storyStateSidebar => 'Estado da história';

  @override
  String get storyStateDescription =>
      'Catálogo oficial dos estados persistentes declarados pelos scripts fornecidos com o jogo. As entradas salvas mostram o valor bruto; os campos do catálogo ausentes deste save são marcados como não definidos. Os marcadores de tempo declarados no código são exibidos como tempo de jogo; os demais inteiros podem ser booleanos, contadores ou estados com vários níveis.';

  @override
  String get storyStateReadOnly =>
      'Somente leitura até que o significado dos valores nos scripts e uma gravação segura do mapa sejam conhecidos. O texto relacionado do glossário fornece contexto; não é uma tradução direta do ID técnico.';

  @override
  String get storyStateStructureReadOnly =>
      'Não foi possível identificar de forma inequívoca e segura a estrutura StoryPropertyValues deste save. Os valores da história permanecerão somente para leitura neste save.';

  @override
  String get storyStateSearch => 'Pesquisar estado da história';

  @override
  String storyStateValuesCount(int shown, int total) {
    return '$shown de $total valores da história';
  }

  @override
  String get storyStateInteger => 'Inteiro';

  @override
  String get storyStateTimeMarker => 'Marcador de tempo';

  @override
  String get storyStateChapter => 'Capítulo';

  @override
  String get storyStateUnknown => 'Tipo de origem desconhecido';

  @override
  String get storyStateUnknownDetail =>
      'Este ID salvo não consta no catálogo de scripts atual (por exemplo, por um mod ou uma versão mais recente do jogo). O valor serializado é int32, mas seu significado não é inferido.';

  @override
  String get storyStateStored => 'Salvo';

  @override
  String get storyStateUnset => 'Não definido';

  @override
  String get storyStateUnsetDetail =>
      'Este campo do catálogo não está serializado neste save; por isso, o jogo usa o estado não definido ou padrão.';

  @override
  String get storyStateRawValue => 'Valor bruto';

  @override
  String storyStateElapsed(String duration) {
    return 'Tempo decorrido ao salvar: $duration';
  }

  @override
  String storyStateAhead(String duration) {
    return 'No futuro ao salvar: $duration';
  }

  @override
  String storyStateDurationDays(int days, String time) {
    String _temp0 = intl.Intl.pluralLogic(
      days,
      locale: localeName,
      other: '$days dias',
      one: '1 dia',
    );
    return '$_temp0 $time';
  }

  @override
  String get storyStateRelatedGlossary => 'Entrada relacionada do glossário';

  @override
  String get storyStateTechnicalPath => 'Caminho técnico';

  @override
  String get storyStateEditingGuidance =>
      'Todos os itens podem ser editados em todo o intervalo int32 com sinal. Os indicadores e as sugestões de valores baseados nos scripts servem apenas como orientação; a entrada do valor bruto está sempre disponível. As alterações no estado da história podem pular transições de diálogos, missões ou do mundo, portanto salve-as com cuidado. Um backup é criado automaticamente.';

  @override
  String get storyStatePending => 'Pendente';

  @override
  String storyStatePendingValue(String value) {
    return 'Será salvo como $value';
  }

  @override
  String get storyStatePendingRemoval => 'Será removido do save';

  @override
  String get storyStateEditValue => 'Editar valor';

  @override
  String get storyStateSetValue => 'Definir valor';

  @override
  String get storyStateRemoveValue => 'Remover do save';

  @override
  String get storyStateUndoChange => 'Desfazer alteração da história';

  @override
  String get storyStateResetChanges => 'Redefinir alterações da história';

  @override
  String storyStateDialogTitle(String id) {
    return 'Editar $id';
  }

  @override
  String get storyStateRawInput => 'Valor int32 com sinal';

  @override
  String get storyStateInvalidInt32 =>
      'Digite um número inteiro entre -2147483648 e 2147483647.';

  @override
  String get storyStateQueueChange => 'Colocar alteração na fila';

  @override
  String storyStateSuggestedValues(String values) {
    return 'Valores confirmados nos scripts fornecidos: $values';
  }

  @override
  String get storyStateSuggestionsNotLimits =>
      'As sugestões não são limites de validação; o código nativo, os mods ou versões posteriores do jogo podem usar outros valores.';

  @override
  String get storyStateUseCurrentTime => 'Usar o horário atual do save';

  @override
  String get storyStateStructuredTime => 'Dia / horário';

  @override
  String get storyStateRawMode => 'int32 bruto';

  @override
  String get storyStateChapterWarning =>
      'Alterar apenas o capítulo não sincroniza missões, NPCs, inventário nem o estado do mundo.';

  @override
  String get storyStateDormantWarning =>
      'Nenhuma leitura ou gravação ativa desse campo foi encontrada no cache dos scripts fornecidos. Ele pode ser legado, controlado por código nativo ou reservado.';

  @override
  String get storyStateReadOnlySourceWarning =>
      'Os scripts fornecidos leem esse campo, mas não contêm nenhuma gravação por script. O código nativo ainda pode ser responsável por ele.';

  @override
  String get storyStateUnknownEditWarning =>
      'Esse ID de um mod ou de uma versão posterior não tem semântica de código-fonte incluída. Edite apenas seu valor int32 bruto.';

  @override
  String storyStateIntegerKind(String kind) {
    String _temp0 = intl.Intl.selectLogic(kind, {
      'binaryFlag': 'Indicador binário',
      'finiteState': 'Valor com vários estados',
      'counterOrScore': 'Contador / pontuação',
      'calendarDay': 'Dia do calendário',
      'derivedOrOpaqueInteger': 'Inteiro derivado / opaco',
      'readOnlyInSourceInteger': 'Somente leitura nos scripts fornecidos',
      'dormantOrLegacyInteger': 'Não utilizado nos scripts fornecidos',
      'other': 'Inteiro',
    });
    return '$_temp0';
  }

  @override
  String get storyStateZeroVsUnset =>
      'Um 0 salvo e uma entrada ausente do mapa são estados de arquivo distintos. “Remover do save” restaura o estado do construtor ou o estado padrão.';

  @override
  String get appTitle => 'GORE Save Editor';

  @override
  String get appLogoSemanticLabel => 'Logotipo do GORE Save Editor';

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
  String get tabAttribute => 'Atributos';

  @override
  String get heroGroupSkills => 'Habilidades';

  @override
  String get skillsNoneBody =>
      'Nenhuma habilidade encontrada para este personagem.';

  @override
  String get skillsUnavailableBody =>
      'As habilidades não podem ser editadas neste save — o herói não tem dados de efeito para modificar.';

  @override
  String get skillNotLearned => 'Não aprendida';

  @override
  String get skillLearn => 'Aprender';

  @override
  String get skillActionLearn => 'aprender';

  @override
  String get skillActionUnlearn => 'desaprender';

  @override
  String get skillTierUntrained => 'Sem Treinamento';

  @override
  String get skillTierBeginner => 'Iniciante';

  @override
  String get skillTierTrained => 'Treinado';

  @override
  String get skillTierMaster => 'Mestre';

  @override
  String get skillTierNovice => 'Aprendiz';

  @override
  String get skillTierAmateur => 'Amador (Círculo 0)';

  @override
  String get skillTierLearned => 'Aprendida';

  @override
  String skillTierCircle(int n) {
    return 'Círculo $n';
  }

  @override
  String get skillHintBlacksmith1H => 'Armas de uma mão';

  @override
  String get skillHintBlacksmith2H => 'Armas de duas mãos';

  @override
  String get skillCategoryCombat => 'Combate';

  @override
  String get skillCategoryCrafting => 'Fabricação';

  @override
  String get skillCategoryHunting => 'Caça';

  @override
  String get skillCategoryLanguage => 'Idioma';

  @override
  String get skillCategoryMagic => 'Magia';

  @override
  String get skillCategoryMovement => 'Movimento';

  @override
  String get skillCategoryThievery => 'Furto';

  @override
  String get skillNameOneHanded => 'Uma Mão';

  @override
  String get skillNameTwoHanded => 'Duas Mãos';

  @override
  String get skillNameFists => 'Punhos';

  @override
  String get skillNameBow => 'Arco';

  @override
  String get skillNameCrossbow => 'Besta';

  @override
  String get skillNameLockpicking => 'Abrir Fechaduras';

  @override
  String get skillNamePickpocketing => 'Mão-Leve';

  @override
  String get skillNameTakeOrgans => 'Extrair Órgão';

  @override
  String get skillNameBreakTeeth => 'Extrair Dentes';

  @override
  String get skillNameTakeClaws => 'Extrair Garra';

  @override
  String get skillNameSkinFur => 'Pegar Pelo';

  @override
  String get skillNameSkin => 'Pegar Pele';

  @override
  String get skillNameTakeFins => 'Pegar Barbatanas';

  @override
  String get skillNameTakeStingers => 'Extrair Ferrões';

  @override
  String get skillNameTakeSecretion => 'Extrair Secreção';

  @override
  String get skillNameTakeSkullPlates => 'Pegar Carapaça de Osso';

  @override
  String get skillNameSkinSwampshark => 'Pegar Pele de Tubarão';

  @override
  String get skillNameTakeMinecrawlerPlates => 'Pegar Carapaças';

  @override
  String get skillNameTakeScutes => 'Pegar Escamas';

  @override
  String get skillNameTakeUluMulu => 'Pegar Ulu-Mulu';

  @override
  String get skillNameOrcWeapons => 'Armas orcas';

  @override
  String get skillNameMining => 'Mineração';

  @override
  String get skillNameDiving => 'Mergulhar';

  @override
  String get skillNameTakeMinecrawlerMandibles => 'Extrair Mandíbulas';

  @override
  String get skillNameTakeShadowbeastHorn => 'Pegar Chifre (Shadowbeast)';

  @override
  String get skillNameTakeSpines => 'Extrair Espinha';

  @override
  String get skillNameBreakSwampsharkTeeth => 'Extrair Dentes de Tubarão';

  @override
  String get skillNameTakeFireTongue => 'Pegar Língua-de-Fogo';

  @override
  String get skillNameTakeTrollHorn => 'Pegar Chifre (Troll)';

  @override
  String get skillNameAcrobatics => 'Acrobacia';

  @override
  String get skillNameWallClimbing => 'Escalar';

  @override
  String get skillNameRiding => 'Andar em Catador';

  @override
  String get skillNameSneaking => 'Furtividade';

  @override
  String get skillNameAlchemy => 'Alquimia';

  @override
  String get skillNameRuneInscription => 'Inscrição';

  @override
  String get skillNameBlacksmithing => 'Ferraria';

  @override
  String get skillNameMagicCircle => 'Círculo de Magia';

  @override
  String get skillNameOrcish => 'Língua dos Orcs';

  @override
  String get tabInventory => 'Inventário';

  @override
  String get tabTrade => 'Comércio';

  @override
  String get traderNotAMerchant => 'Este personagem não comercia.';

  @override
  String get traderRetry => 'Tentar novamente';

  @override
  String get traderAmbiguousName =>
      'Mais de um registro de mercador tem este nome, por isso não é possível saber qual loja pertence a este personagem. A edição está desativada em vez de arriscar mudar a errada.';

  @override
  String get traderOre => 'Minério (poder de compra)';

  @override
  String get traderNoOre => 'sem minério';

  @override
  String get traderStockCurrent => 'Estoque';

  @override
  String get traderStockBase => 'Base de reposição';

  @override
  String get traderStockBaseHint =>
      'Aquilo que o mercador repõe. Cresce com o progresso da história, por isso não é um estado original.';

  @override
  String get traderOreHint =>
      'O valor no jogo difere: ao carregar, o jogo soma o que se acumulou desde a última troca — ele vende excedentes e repõe com isso. Este número é o ponto de partida, não o que a tela de comércio mostra.';

  @override
  String get traderPriceWarning =>
      'Os preços reagem ao que o mercador tem em estoque e ao minério que possui, por isso mudar esses números também pode alterar o que ele cobra.';

  @override
  String get traderAddItem => 'Adicionar item';

  @override
  String get traderRemoveItem => 'Remover linha';

  @override
  String get traderReadOnlyCore =>
      'Esta versão do núcleo só consegue ler os dados dos mercadores.';

  @override
  String get traderDifficultyStockUnsupported =>
      'Este mercador tem estoque por dificuldade, que o editor não modela. A edição está desativada aqui, porque uma alteração pareceria bem-sucedida deixando esse estoque intacto.';

  @override
  String get traderRecordIncomplete =>
      'As listas de estoque deste mercador não existem, ou têm uma forma que o editor não suporta nem consegue gravar. A edição está desativada aqui para que uma alteração não falhe ao salvar.';

  @override
  String get traderEmptyStock => 'Nada em estoque.';

  @override
  String get traderUnknownItem => 'não está no catálogo de itens';

  @override
  String editorTradersLoadFailed(String details) {
    return 'Falha ao carregar mercadores: $details';
  }

  @override
  String traderStockLineCount(int count) {
    return '$count linhas';
  }

  @override
  String get tabWorld => 'Mundo';

  @override
  String get tabCharacters => 'Personagens';

  @override
  String get characterNoActorBody =>
      'Este personagem não tem um ator no mundo, portanto não tem atributos, inventário ou eventos.';

  @override
  String get characterNoEventsBody => 'Sem eventos para este personagem.';

  @override
  String get characterOrphanGroup => 'Outros';

  @override
  String get tabAllData => 'Todos os dados';

  @override
  String get tabBackups => 'Cópias';

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
  String get equippedBadge => 'Equipado';

  @override
  String get armorUpgradesLabel => 'Melhorias';

  @override
  String get browse => 'Procurar';

  @override
  String get noSavFilesFound => 'Nenhum arquivo .sav encontrado';

  @override
  String get profile => 'Perfil';

  @override
  String get otherSaves => 'Outros jogos salvos';

  @override
  String profileWithSaves(String name, int count) {
    return '$name ($count jogos salvos)';
  }

  @override
  String get switchProfile => 'Trocar de perfil';

  @override
  String get openSaveFile => 'Abrir arquivo';

  @override
  String get externalSave => 'Jogo salvo aberto externamente';

  @override
  String get saveProfileTitle => 'Perfil do jogo salvo';

  @override
  String get saveProfileDescription =>
      'Atribua este jogo salvo a outro perfil do jogo. O jogo salvo e o índice de perfis serão copiados juntos.';

  @override
  String get saveProfileExternalHint =>
      'Selecione um perfil para importar este arquivo para a pasta de jogos salvos e registrá-lo. O arquivo original permanece inalterado.';

  @override
  String get saveProfileNoProfiles =>
      'Nenhum perfil de jogo editável foi encontrado em PersistentDataList.sav.';

  @override
  String get saveProfileSelect => 'Selecionar perfil';

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
  String bytesValue(String count) {
    return '$count bytes';
  }

  @override
  String get inspectionJsonTitle => 'JSON de inspeção';

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
  String get gameTimeTitle => 'Tempo de jogo';

  @override
  String get gameTimeDay => 'Dia';

  @override
  String get gameTimeHours => 'Horas';

  @override
  String get gameTimeMinutes => 'Minutos';

  @override
  String get gameTimeSeconds => 'Segundos';

  @override
  String gameTimeTotal(int seconds) {
    return '= $seconds s no total';
  }

  @override
  String get gameTimeInvalid =>
      'Insira números inteiros: dia ≥ 0, horas 0–23, minutos e segundos 0–59.';

  @override
  String get required => 'Obrigatório';

  @override
  String get playerLockedBody =>
      'Edições privadas do jogador exigem um codec capaz de comprimir.';

  @override
  String get heroTransform => 'Posição';

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
  String get spawnPositionSection => 'Posição de spawn (referência)';

  @override
  String get resetToSpawnPosition => 'Redefinir para a posição de spawn';

  @override
  String get positionOutOfRange =>
      'O valor deve estar entre −10.000.000 e 10.000.000';

  @override
  String get positionNotEditable =>
      'Não foi possível ler a posição salva deste personagem, portanto ela não pode ser editada.';

  @override
  String get positionNeverPlaced =>
      'Este personagem nunca foi colocado no mundo (posição 0, 0, 0) — o jogo pode ignorar a posição salva.';

  @override
  String get npcStayInPlace => 'Desativar a rotina diária dele';

  @override
  String get npcStayInPlaceHint => 'Ele então fica onde está.';

  @override
  String get npcStayInPlaceLocked =>
      'A rotina diária original dele não está registrada, então não é mais possível desfazer.';

  @override
  String get npcUndoPlacement => 'Desfazer a mudança';

  @override
  String get npcUndoPlacementStale =>
      'O jogo salvo não contém mais o que essa mudança escreveu, então restaurá-la descartaria o que aconteceu desde então.';

  @override
  String get positionNotReadable =>
      'Não foi possível ler a posição salva deste personagem.';

  @override
  String get npcPositionReadOnly =>
      'O jogo restaura a posição de um NPC a partir da fase e não do jogo salvo, portanto esses valores podem ser lidos, mas não alterados.';

  @override
  String get pickLocation => 'Escolher local…';

  @override
  String get pickLocationDialogTitle => 'Escolher um local';

  @override
  String get applySpotRotation => 'Aplicar também a orientação do ponto';

  @override
  String get locationAreaOther => 'Outros';

  @override
  String get locationAreaCavalornValley => 'Vale de Cavalorn';

  @override
  String get locationAreaEastForest => 'Floresta do Leste';

  @override
  String get locationAreaFogTower => 'Torre da Névoa';

  @override
  String get locationAreaIllegalWeedMixers => 'Misturadores ilegais de erva';

  @override
  String get locationAreaOrcArena => 'Arena dos Orcs';

  @override
  String get locationAreaOrcGraveyard => 'Cemitério dos Orcs';

  @override
  String get locationAreaShipwreck => 'Naufrágio';

  @override
  String get locationAreaTundra => 'Tundra';

  @override
  String get locationCatalogUnavailable =>
      'Não foi possível carregar o catálogo de locais.';

  @override
  String get invalid => 'Inválido';

  @override
  String get heroAttributes => 'Atributos do herói';

  @override
  String attributeBase(String name) {
    return 'Valor base de $name';
  }

  @override
  String attributeCurrent(String name) {
    return '$name atual';
  }

  @override
  String get attributeBaseValue => 'Valor base';

  @override
  String get attributeCurrentValue => 'Valor atual';

  @override
  String get inventoryTitle => 'Inventário';

  @override
  String get inventoryEmpty => 'Este inventário está vazio.';

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
  String get resetInventoryButton => 'Redefinir inventário';

  @override
  String get resetInventoryTooltipDefault =>
      'Substituir este inventário pelo inventário do início do jogo';

  @override
  String get resetInventoryTooltipBlocked =>
      'Salve ou cancele primeiro as alterações de inventário pendentes';

  @override
  String get pendingResetTitle =>
      'Redefinir para o inventário do início do jogo';

  @override
  String pendingResetSubtitle(String level) {
    return 'Nível de recursos: $level';
  }

  @override
  String get cancelPendingReset => 'Cancelar redefinição';

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
  String get itemCategoryMeleeWeapon => 'Armas corpo a corpo';

  @override
  String get itemCategoryRangedWeapon => 'Armas à distância';

  @override
  String get itemCategoryAmmunition => 'Munição';

  @override
  String get itemCategoryArmor => 'Armaduras';

  @override
  String get itemCategoryRune => 'Runas';

  @override
  String get itemCategoryScroll => 'Pergaminhos de magia';

  @override
  String get itemCategoryFood => 'Comida e poções';

  @override
  String get itemCategoryMisc => 'Diversos';

  @override
  String get itemCategoryAmulet => 'Amuletos';

  @override
  String get itemCategoryRing => 'Anéis';

  @override
  String get itemCategoryTrophy => 'Troféus de animais';

  @override
  String get itemCategoryWriting => 'Escritos';

  @override
  String get itemCategoryMission => 'Itens de missão';

  @override
  String get itemCategoryKey => 'Chaves';

  @override
  String get itemCategoryOther => 'Outros';

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
  String get npcStatusRowLabel => 'Estado';

  @override
  String get npcStatusAlive => 'vivo';

  @override
  String get npcStatusDead => 'morto';

  @override
  String get npcRelationshipRowLabel => 'Relação';

  @override
  String get npcRelationshipUnavailable => 'Estado da relação indisponível';

  @override
  String get npcRelationshipAutomatic => 'Calculada pelo jogo';

  @override
  String get npcRelationshipAutomaticHint =>
      'Nenhuma substituição permanente está armazenada. As regras de guilda, história, área e crimes são avaliadas no jogo.';

  @override
  String get npcRelationshipStoredHint =>
      'Armazenada como uma substituição permanente do NPC em relação ao jogador. As regras de guilda, história, área e crimes ainda podem alterar a relação efetiva no jogo.';

  @override
  String get npcRelationshipFriend => 'Amigo';

  @override
  String get npcRelationshipNeutral => 'Neutro';

  @override
  String get npcRelationshipEnemy => 'Inimigo';

  @override
  String npcRelationshipPending(String relationship) {
    return 'Será $relationship ao salvar';
  }

  @override
  String npcStateHp(String hp, String maxHp) {
    return 'PV $hp / $maxHp';
  }

  @override
  String get npcReviveButton => 'Reviver';

  @override
  String get npcReviveQueued => 'Será revivido ao salvar';

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
  String get memoryEventRemovalQueued =>
      'Remoção do evento na fila — pressione Salvar para aplicar.';

  @override
  String get duplicateEvent => 'Duplicar evento';

  @override
  String get duplicateMemoryEventTitle => 'Duplicar evento de memória?';

  @override
  String get duplicateMemoryEventBody =>
      'Duplicar este evento de memória? Um backup é gravado antes.';

  @override
  String get memoryEventDuplicationQueued =>
      'Duplicação do evento na fila — pressione Salvar para aplicar.';

  @override
  String get selectCharacterFromList => 'Selecione um personagem na lista';

  @override
  String get factionsSidebar => 'Facções';

  @override
  String get factionsForgiveButton => 'Perdoar';

  @override
  String get factionHostile => 'Hostil';

  @override
  String get factionFriendly => 'Amigável';

  @override
  String crimeMurder(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count homicídios',
      one: '$count homicídio',
    );
    return '$_temp0';
  }

  @override
  String crimeAssault(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count agressões',
      one: '$count agressão',
    );
    return '$_temp0';
  }

  @override
  String crimeTheft(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count furtos',
      one: '$count furto',
    );
    return '$_temp0';
  }

  @override
  String crimeTrespassing(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count invasões',
      one: '$count invasão',
    );
    return '$_temp0';
  }

  @override
  String crimeThreat(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count ameaças',
      one: '$count ameaça',
    );
    return '$_temp0';
  }

  @override
  String crimeOther(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count outros crimes',
      one: '$count outro crime',
    );
    return '$_temp0';
  }

  @override
  String get factionsForgiveQueued => 'perdoando…';

  @override
  String get factionsEmpty => 'Nenhum crime em aberto contra facções.';

  @override
  String get factionGuildOldCamp => 'Assentamento Antigo';

  @override
  String get factionGuildNewCamp => 'Assentamento Novo';

  @override
  String get factionGuildSwampCamp => 'Assentamento do Pântano';

  @override
  String get factionGuildOther => 'Outros/indivíduos';

  @override
  String get allDataLockedBody =>
      'O navegador completo de fontes está disponível no momento para arquivos de salvamento GSAV.';

  @override
  String get allDataDescription =>
      'Navegue pelos metadados GSAV e por todos os nós tipados PUBLIC/PRIVATE. Valores escalares e estruturas nativas seguras são editáveis; contêineres e bytes opacos continuam visíveis.';

  @override
  String get allDataEditable => 'Editável';

  @override
  String get allDataReadOnly => 'Somente leitura';

  @override
  String get allDataType => 'Tipo';

  @override
  String get allDataScalars => 'Escalares';

  @override
  String get allDataStructs => 'Estruturas';

  @override
  String get allDataContainers => 'Contêineres';

  @override
  String get allDataOpaque => 'Opacos';

  @override
  String get allDataNodes => 'Nós';

  @override
  String allDataChildren(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count nós filhos',
      one: '1 nó filho',
    );
    return '$_temp0';
  }

  @override
  String get allDataPending => 'Pendente';

  @override
  String get allDataTagInputHint =>
      'Tags separadas por vírgulas ou quebras de linha';

  @override
  String allDataTypedSource(String source) {
    return 'Fonte tipada: $source';
  }

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
  String get backupsTitle => 'Cópias de segurança';

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
      'A versão portátil abre a página de download no navegador. Substitua os arquivos atuais pelo novo download.';

  @override
  String get updateAvailableTitle => 'Atualização disponível';

  @override
  String updateAvailableMessage(Object version, Object current) {
    return 'A versão $version está disponível. Você tem a $current.';
  }

  @override
  String get updateDownload => 'Baixar';

  @override
  String get updateLater => 'Mais tarde';

  @override
  String get updateUpToDate => 'Você está usando a versão mais recente.';

  @override
  String get updateCheckFailed =>
      'Não foi possível verificar atualizações. Tente novamente mais tarde.';

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
  String aboutVersion(String version, String sha) {
    return 'Versão $version ($sha)';
  }

  @override
  String get aboutCopyright => '© 2026 colaboradores do GORE';

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
  String get presetNovice => 'Fácil';

  @override
  String get presetGothic => 'Gothic';

  @override
  String get presetHard => 'Difícil';

  @override
  String get presetCustom => 'Personalizada';

  @override
  String unrecognisedPreset(Object preset) {
    return 'A predefinição armazenada não é reconhecida ($preset). Você ainda pode salvar alterações de Assistente de Fluência / Permadeath, ou escolher uma predefinição acima para sobrescrevê-la.';
  }

  @override
  String get closeCombatFlowHelper =>
      'Assistência de Fluidez de Combate de Curto Alcance';

  @override
  String get permadeath => 'Morte Permanente';

  @override
  String get notAvailableOnNovice => 'Indisponível no Iniciante';

  @override
  String get levelCombat => 'Combate';

  @override
  String get levelResources => 'Recursos';

  @override
  String get levelProgression => 'Progressão';

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
  String get heroEntryHeroTransform => 'Posição';

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

  @override
  String savingProgress(int done, int total) {
    return 'Salvando… $done de $total';
  }

  @override
  String localizedTextExtractedCount(int idCount, int languageCount) {
    return '$idCount IDs extraídos em $languageCount idiomas';
  }

  @override
  String get skillSmithing1H => 'Ferraria de Armas de Uma Mão';

  @override
  String get skillSmithing2H => 'Ferraria de Armas de Duas Mãos';

  @override
  String get skillCircleNovice => 'Mago Iniciado';

  @override
  String get skillCircle1 => 'Primeiro Círculo de Magia';

  @override
  String get skillCircle2 => 'Segundo Círculo de Magia';

  @override
  String get skillCircle3 => 'Terceiro Círculo de Magia';

  @override
  String get skillCircle4 => 'Quarto Círculo de Magia';

  @override
  String get skillCircle5 => 'Quinto Círculo de Magia';

  @override
  String get skillCircle6 => 'Sexto Círculo de Magia';

  @override
  String get sectionGlossary => 'Glossário';

  @override
  String get glossarySearch => 'Pesquisar no glossário';

  @override
  String get glossaryOldCamp => 'Assentamento Antigo';

  @override
  String get glossaryNewCamp => 'Assentamento Novo';

  @override
  String get glossarySwampCamp => 'Assentamento do Pântano';

  @override
  String get glossaryOutsiders => 'Forasteiros';

  @override
  String get glossaryCreatures => 'Criaturas';

  @override
  String get glossaryLocations => 'Locais';

  @override
  String get glossaryFilterLabel => 'Filtro';

  @override
  String get glossaryFilterTraders => 'Comerciantes';

  @override
  String get glossaryFilterTeachers => 'Professores';

  @override
  String get glossaryFilterArmorers => 'Armeiros';

  @override
  String get glossaryFilterHostile => 'Hostis';

  @override
  String get glossaryRelationshipFilterNote =>
      'Mostra as substituições permanentes de inimigo armazenadas no jogo salvo. As relações dinâmicas de guilda, história, área e crimes são calculadas apenas no jogo.';

  @override
  String get glossaryFilterDead => 'Mortos';

  @override
  String get glossaryAddEntry => 'Adicionar entrada ao glossário';

  @override
  String get glossaryAddTitle => 'Adicionar entrada ao glossário';

  @override
  String get glossaryResetChanges => 'Redefinir alterações do glossário';

  @override
  String get glossaryNoVisibleEntries =>
      'Nenhuma entrada visível do glossário corresponde a esta visualização.';

  @override
  String get glossaryNoHiddenEntries =>
      'Todas as entradas disponíveis já estão visíveis.';

  @override
  String get glossaryNoMatch => 'Nenhuma entrada do glossário corresponde.';

  @override
  String get glossarySelectEntry =>
      'Selecione uma entrada do glossário para editar suas seções.';

  @override
  String glossaryEntryCount(int count) {
    return '$count entradas';
  }

  @override
  String glossarySegmentsCount(int unlocked, int total) {
    return '$unlocked de $total entradas';
  }

  @override
  String get glossaryPortraitUnlocked => 'Retrato desbloqueado';

  @override
  String get glossaryPortraitSilhouette =>
      'Silhueta — retrato não desbloqueado';

  @override
  String get glossarySegments => 'Entradas';

  @override
  String get glossaryPending => 'Alteração não salva';

  @override
  String get glossaryShowFullText => 'Mostrar o texto completo da entrada';

  @override
  String get glossarySegmentIntroduction => 'Introdução / retrato';

  @override
  String get glossarySegmentUnlock => 'Descoberta';

  @override
  String glossarySegmentEntry(int number) {
    return 'Entrada $number';
  }

  @override
  String get questJournalAll => 'Todas as missões';

  @override
  String get questJournalOldCamp => 'Assentamento Antigo';

  @override
  String get questJournalNewCamp => 'Assentamento Novo';

  @override
  String get questJournalSwampCamp => 'Assentamento do Pântano';

  @override
  String get questJournalColony => 'A Colônia';

  @override
  String get questJournalCompleted => 'Concluídas';

  @override
  String get questJournalHint =>
      'Visualização do diário no jogo. Estados internos e missões ainda não iniciadas continuam disponíveis em Todos os dados.';

  @override
  String get questJournalNoEntries =>
      'Nenhuma missão do diário corresponde aos filtros atuais.';

  @override
  String get glossaryTutorials => 'Tutoriais';

  @override
  String get tutorialGateNote =>
      'Estas linhas controlam os desbloqueios de tutorial armazenados. Um desbloqueio não corresponde necessariamente a uma única página de tutorial no jogo.';

  @override
  String get tutorialResetChanges => 'Redefinir alterações dos tutoriais';

  @override
  String get tutorialNoGates =>
      'Nenhum desbloqueio de tutorial está disponível neste jogo salvo.';

  @override
  String tutorialGateUnlockCount(int unlocked, int total) {
    return '$unlocked de $total tutoriais desbloqueados';
  }

  @override
  String get tutorialGateCombatBasics => 'Noções básicas de combate';

  @override
  String get tutorialGateCrafting => 'Criação';

  @override
  String get tutorialGateCrime => 'Crimes e consequências';

  @override
  String get tutorialGateDrugs => 'Consumíveis e efeitos';

  @override
  String get tutorialGateLockpicking => 'Arrombamento';

  @override
  String get tutorialGateMagic => 'Magia';

  @override
  String get tutorialGateMap => 'Mapa';

  @override
  String get tutorialGateMeleeCombat => 'Combate corpo a corpo';

  @override
  String get tutorialGateNavigation => 'Movimento e navegação';

  @override
  String get tutorialGatePerception => 'Percepção';

  @override
  String get tutorialGatePlayerProgression => 'Progressão do personagem';

  @override
  String get tutorialGateRanged => 'Combate à distância';

  @override
  String get tutorialGateRiding => 'Montaria';

  @override
  String get tutorialGateSleep => 'Dormir';

  @override
  String get tutorialGateTrading => 'Comércio';

  @override
  String get windowMinimizeTooltip => 'Minimizar';

  @override
  String get windowMaximizeTooltip => 'Maximizar';

  @override
  String get windowRestoreTooltip => 'Restaurar';

  @override
  String get fallbackDialogEntry => 'Entrada de diálogo';

  @override
  String get fallbackDialogChoice => 'Escolha de diálogo';

  @override
  String get fallbackDialogTopic => 'Tópico de diálogo';

  @override
  String get fallbackDialogInformation => 'Informação de diálogo';

  @override
  String get fallbackQuest => 'Missão';

  @override
  String get fallbackObjective => 'Objetivo';

  @override
  String get fallbackItem => 'Item';

  @override
  String get attributeSkillPointsFallback => 'Pontos de aprendizado (PA)';

  @override
  String attributeManualFallbackLabel(String attributeId, String fallback) {
    String _temp0 = intl.Intl.selectLogic(attributeId, {
      'Alcohol': 'Álcool',
      'AlcoholDepletionRate': 'Taxa de redução do álcool',
      'MaxAlcohol': 'Nível máximo de álcool',
      'MaxSuperArmor': 'Superarmadura máxima',
      'SuperArmor': 'Superarmadura',
      'Fatigue': 'Fadiga',
      'FillRatio': 'Proporção de preenchimento',
      'FillRatioPeriod': 'Período de preenchimento',
      'MaxFatigue': 'Fadiga máxima',
      'MaxThresholdIndex': 'Índice máximo de limiar',
      'RecoveryRatePerHourOfSleep': 'Recuperação por hora de sono',
      'DamageMultiplier': 'Multiplicador de dano',
      'Toughness': 'Tenacidade',
      'ToughnessA': 'Tenacidade A',
      'ToughnessB': 'Tenacidade B',
      'ToughnessC': 'Tenacidade C',
      'XPExecutedBounty': 'Recompensa de XP por execução',
      'XPKillOrDefeatBounty': 'Recompensa de XP por morte ou derrota',
      'SpeedModifier': 'Modificador de velocidade',
      'CriticalLevelPercent': 'Nível crítico (%)',
      'MaxOxygen': 'Oxigênio máximo',
      'Oxygen': 'Oxigênio',
      'OxygenDepletionRate': 'Taxa de consumo de oxigênio',
      'OxygenRecoveryRate': 'Taxa de recuperação de oxigênio',
      'MaxRestTime': 'Tempo máximo de descanso',
      'MaxSleepTime': 'Tempo máximo de sono',
      'SleepTime': 'Tempo de sono',
      'SleepTimeRecoveryAmount': 'Quantidade recuperada durante o sono',
      'SleepTimeRecoveryPeriod': 'Intervalo de recuperação durante o sono',
      'MaxSwampweed': 'Quantidade máxima de erva do pântano',
      'Swampweed': 'Erva do pântano',
      'SwampweedDepletionRate': 'Taxa de consumo de erva do pântano',
      'other': '$fallback',
    });
    return '$_temp0';
  }

  @override
  String get knowledgeTypeVoiceLine => 'Linha de voz';

  @override
  String get knowledgeTypeOther => 'Outro';

  @override
  String get armorUpgradeUpper => 'Superior';

  @override
  String get armorUpgradeMiddle => 'Central';

  @override
  String get armorUpgradeLower => 'Inferior';

  @override
  String get knowledgeCategoryTopic => 'Tópico';

  @override
  String get knowledgeCategoryChoice => 'Escolha';

  @override
  String get knowledgeCategoryInfo => 'Informação';

  @override
  String get statusOk => 'OK';

  @override
  String get statusFailed => 'Falha';

  @override
  String get missingSaveReference => 'Arquivo ausente';

  @override
  String missingSaveReferenceDescription(String slot) {
    return '$slot.sav está ausente. O arquivo pode ter sido excluído, movido ou renomeado; o perfil ainda faz referência a ele.';
  }

  @override
  String get removeFromProfile => 'Remover do perfil';

  @override
  String get removeSaveFromProfileTitle => 'Remover jogo salvo do perfil?';

  @override
  String removeSaveFromProfileBody(String save, String profile) {
    return 'Remover $save de $profile? O próprio arquivo de jogo salvo será mantido se ainda existir.';
  }

  @override
  String get unassignedSave => 'Não atribuído a um perfil';

  @override
  String get armorUpgradeLight => 'Leve';

  @override
  String get armorUpgradeMedium => 'Média';

  @override
  String get armorUpgradeHeavy => 'Pesada';

  @override
  String get knowledgeCaptionForcedConversation => 'Conversa forçada';

  @override
  String get knowledgeCaptionFollowupTopic => 'Tópico de acompanhamento';

  @override
  String get knowledgeCaptionFallbackTopic => 'Tópico alternativo';

  @override
  String durationMinutes(int minutes) {
    return '$minutes min';
  }

  @override
  String durationHours(int hours) {
    return '$hours h';
  }

  @override
  String durationHoursMinutes(int hours, int minutes) {
    return '$hours h $minutes min';
  }

  @override
  String get backupStatusInvalidProfileStructure => 'Dados de perfil inválidos';

  @override
  String get backupStatusSlotMetadataMissing =>
      'Os metadados do jogo salvo selecionado estão ausentes';

  @override
  String defaultProfileName(int id) {
    return 'Perfil $id';
  }

  @override
  String get statusUnknown => 'Desconhecido';

  @override
  String editorUnexpectedError(String details) {
    return 'Erro inesperado: $details';
  }

  @override
  String get editorOperationInProgress =>
      'Há outra operação em andamento. Tente novamente em instantes.';

  @override
  String get editorUnsavedBeforeDifficulty =>
      'O jogo salvo contém alterações não salvas. Salve-as ou redefina-as antes de mudar a dificuldade do perfil.';

  @override
  String get editorNoSaveFolderSelected =>
      'Nenhuma pasta de jogos salvos selecionada.';

  @override
  String get editorNoSaveSelected => 'Nenhum jogo salvo selecionado.';

  @override
  String get coreUnknownError => 'Erro interno desconhecido';

  @override
  String get editorUnsavedBeforeSwitchProfile =>
      'Primeiro, salve ou redefina as alterações pendentes, pois a troca de perfil faria você sair do jogo salvo atual.';

  @override
  String get editorUnsavedBeforeOpenFile =>
      'Salve ou redefina as alterações pendentes antes de abrir outro arquivo.';

  @override
  String get editorSelectSavFile => 'Selecione um arquivo .sav de jogo salvo.';

  @override
  String get editorNotGothicGsav =>
      'O arquivo selecionado não é um jogo salvo Gothic GSAV.';

  @override
  String get editorUnsavedBeforeChangeSaveProfile =>
      'Salve ou redefina as alterações pendentes antes de mudar o perfil do jogo salvo.';

  @override
  String get editorUnsavedBeforeRemoveProfile =>
      'Salve ou redefina as alterações pendentes antes de remover um jogo salvo do perfil.';

  @override
  String get editorUnsavedBeforeRestoreProfile =>
      'O jogo salvo contém alterações não salvas. Salve-as ou redefina-as antes de restaurar um backup do perfil.';

  @override
  String editorConflictingPropertyEdits(String path) {
    return 'As alterações pendentes de duas abas afetam a mesma propriedade ($path). Redefina ou desfaça uma delas e salve novamente.';
  }

  @override
  String editorGlossaryMemoryConflict(String path) {
    return 'Uma alteração de segmento do Glossário e outra alteração pendente em “Todos os dados” afetam a matriz Hero MemorizedEvents ($path). As alterações do Glossário adicionam ou removem entradas dessa matriz, portanto não podem ser salvas juntas. Redefina ou desfaça uma delas e salve novamente.';
  }

  @override
  String editorGlossaryQuestConflict(String path) {
    return 'Uma alteração de segmento do Glossário e outra alteração pendente afetam a mesma propriedade CurrentState de uma missão ($path). A própria alteração do Glossário atualiza esse estado. Redefina ou desfaça uma delas e salve novamente.';
  }

  @override
  String editorRelationshipConflict(String path) {
    return 'Uma alteração de relacionamento e outra alteração pendente em “Todos os dados” afetam a mesma entrada de relacionamento de um NPC ($path). A alteração estruturada do relacionamento pode substituir modificadores nessa entrada, portanto não podem ser salvas juntas. Redefina ou desfaça uma delas e salve novamente.';
  }

  @override
  String editorMultipleStructuralArrayEdits(String path) {
    return 'Há mais de uma alteração estrutural pendente para a mesma matriz ($path). Salve ou redefina a primeira alteração antes de adicionar outra.';
  }

  @override
  String editorStructuralArrayConflict(String path) {
    return 'Uma alteração estrutural de evento e outra alteração pendente em “Todos os dados” afetam $path. Salve ou redefina uma delas antes de continuar.';
  }

  @override
  String get editorSkillsEffectConflict =>
      'Há uma alteração em “Habilidades” e outra em “Todos os dados” para o mesmo efeito do personagem (ActiveEffects › EffectSpec › Def) pendentes. Elas não podem ser salvas juntas. Redefina ou desfaça uma delas e salve novamente.';

  @override
  String get editorInventoryResetConflict =>
      'Há uma redefinição do inventário e outra alteração no mesmo inventário pendentes. A redefinição substitui todo o inventário e descartaria a outra alteração. Redefina ou desfaça uma delas e salve novamente.';

  @override
  String get editorUseFolder => 'Usar pasta';

  @override
  String get editorGothicSavegameFileType => 'Jogo salvo de Gothic';

  @override
  String get editorNoDifficultyChanges =>
      'Não há alterações de dificuldade para salvar';

  @override
  String get editorDifficultyWritten =>
      'Dificuldade salva no perfil (backup criado)';

  @override
  String editorChangesSavedWithBackup(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count alterações salvas com backup',
      one: '1 alteração salva com backup',
    );
    return '$_temp0';
  }

  @override
  String editorPlacementNoteFailed(String details) {
    return 'A mudança foi salva, mas não foi possível escrever a nota para desfazê-la: $details';
  }

  @override
  String editorProfileNotFound(int profileId) {
    return 'O perfil $profileId não foi encontrado.';
  }

  @override
  String get editorNoFreeSaveSlot =>
      'Não há nenhum slot livre na pasta de jogos salvos (G1R-001 a G1R-999).';

  @override
  String editorSaveImportedAssigned(int profileId) {
    return 'Jogo salvo importado e atribuído ao perfil $profileId';
  }

  @override
  String editorSaveAssigned(int profileId) {
    return 'Jogo salvo atribuído ao perfil $profileId (backups correspondentes criados)';
  }

  @override
  String editorSaveSlotNotAssigned(String slot, int profileId) {
    return 'O slot de jogo salvo $slot não está atribuído ao perfil $profileId.';
  }

  @override
  String get editorSaveRemovedFromProfile => 'Jogo salvo removido do perfil';

  @override
  String editorRestoredBackup(String path) {
    return 'Backup restaurado: $path';
  }

  @override
  String editorRestoredBackupWithoutCompanion(String path) {
    return 'Backup restaurado: $path (PersistentDataList.sav não foi alterado porque não há um backup correspondente; os metadados do slot podem ser diferentes)';
  }

  @override
  String editorCodecRoundtripPassed(int chunkIndex, int bytes) {
    return 'Validação completa do codec concluída: o bloco $chunkIndex foi recomprimido para $bytes bytes';
  }

  @override
  String editorDifficultyWriteFailed(String details) {
    return 'Não foi possível salvar a dificuldade do perfil: $details';
  }

  @override
  String editorProfileAssignmentFailed(String details) {
    return 'Não foi possível atribuir o jogo salvo ao perfil: $details';
  }

  @override
  String editorProfileRemovalFailed(String details) {
    return 'Não foi possível remover o jogo salvo do perfil: $details';
  }

  @override
  String editorSaveFailed(String details) {
    return 'Não foi possível salvar as alterações: $details';
  }

  @override
  String editorScanSavesFailed(String details) {
    return 'Não foi possível examinar os jogos salvos: $details';
  }

  @override
  String editorInspectSaveFailed(String details) {
    return 'Não foi possível inspecionar o jogo salvo: $details';
  }

  @override
  String editorLoadBackupsFailed(String details) {
    return 'Não foi possível carregar os backups: $details';
  }

  @override
  String editorRestoreFailed(String details) {
    return 'Não foi possível restaurar o backup: $details';
  }

  @override
  String editorRestoreReloadFailed(String path, String details) {
    return 'Backup restaurado: $path, mas não foi possível recarregar o jogo salvo: $details';
  }

  @override
  String editorCodecCheckFailed(String details) {
    return 'A verificação do codec falhou: $details';
  }

  @override
  String editorCodecValidationFailed(String details) {
    return 'A validação completa do codec falhou: $details';
  }

  @override
  String editorPropertySearchFailed(String details) {
    return 'A busca de propriedades falhou: $details';
  }

  @override
  String get editorSelectionChangedWhileLoadingHeroAttributes =>
      'A seleção do jogo salvo mudou durante o carregamento dos atributos do herói.';

  @override
  String editorSkillsLoadFailed(String details) {
    return 'O carregamento das habilidades falhou: $details';
  }

  @override
  String editorProgressionQueryFailed(String details) {
    return 'A consulta de progressão falhou: $details';
  }

  @override
  String editorNpcListFailed(String details) {
    return 'O carregamento da lista de NPCs falhou: $details';
  }

  @override
  String editorCharacterListFailed(String details) {
    return 'O carregamento da lista de personagens falhou: $details';
  }

  @override
  String editorNpcAttributesFailed(String details) {
    return 'O carregamento dos atributos do NPC falhou: $details';
  }

  @override
  String editorNpcPositionFailed(String details) {
    return 'O carregamento da posição do NPC falhou: $details';
  }

  @override
  String editorNpcInventoryFailed(String details) {
    return 'O carregamento do inventário do NPC falhou: $details';
  }

  @override
  String editorFactionListFailed(String details) {
    return 'O carregamento da lista de facções falhou: $details';
  }

  @override
  String get editorNoBackupPath => 'nenhum';

  @override
  String editorBackupMessage(String prefix, String backupPath) {
    return '$prefix: $backupPath';
  }

  @override
  String editorBackupMessageWithPersistent(
    String prefix,
    String backupPath,
    String persistentPath,
  ) {
    return '$prefix: $backupPath; backup de PersistentDataList: $persistentPath';
  }

  @override
  String localizationStatusFailed(String details) {
    return 'Não foi possível obter o status da localização: $details';
  }

  @override
  String localizationExtractionFailed(String details) {
    return 'A extração falhou: $details';
  }

  @override
  String glossaryLoadFailed(String details) {
    return 'O carregamento do Glossário falhou: $details';
  }

  @override
  String backupStatusError(String details) {
    return 'Erro de backup: $details';
  }

  @override
  String memoryEventCategory(String category, String fallback) {
    String _temp0 = intl.Intl.selectLogic(category, {
      'quest': 'Missão',
      'document': 'Documento',
      'story': 'História',
      'exploration': 'Exploração',
      'combat': 'Combate',
      'social': 'Social',
      'item': 'Itens',
      'learning': 'Aprendizado',
      'guild': 'Guilda',
      'crime': 'Crime',
      'rest': 'Descanso',
      'other': '$fallback',
    });
    return '$_temp0';
  }

  @override
  String memoryEventAction(String kind, String fallback) {
    String _temp0 = intl.Intl.selectLogic(kind, {
      'questStarted': 'Missão iniciada',
      'questSucceeded': 'Missão concluída',
      'questFailed': 'Missão fracassada',
      'documentRead': 'Documento lido',
      'documentSegmentUnlocked': 'Entrada descoberta',
      'documentSegmentViewed': 'Entrada visualizada',
      'chapterCompleted': 'Capítulo concluído',
      'areaEntered': 'Entrada na área',
      'areaLeft': 'Saída da área',
      'characterKilled': 'Personagem morto',
      'characterDefeated': 'Personagem derrotado',
      'combatDodge': 'Ataque esquivado',
      'characterDebuffed': 'Efeito negativo aplicado',
      'tradeAvailable': 'Comércio desbloqueado',
      'itemObtained': 'Item obtido',
      'itemCrafted': 'Item criado',
      'skillStateRecorded': 'Estado das habilidades registrado',
      'recipeLearned': 'Receita aprendida',
      'guildJoined': 'Entrada na guilda',
      'crimeRecorded': 'Crime registrado',
      'slept': 'Sono',
      'storyEvent': 'Evento da história',
      'other': '$fallback',
    });
    return '$_temp0';
  }

  @override
  String memoryEventTitleWithSubject(String action, String subject) {
    return '$action: $subject';
  }

  @override
  String memoryEventFact(String fact, String fallback) {
    String _temp0 = intl.Intl.selectLogic(fact, {
      'gameTime': 'Tempo de jogo',
      'duration': 'Duração',
      'chapter': 'Capítulo',
      'instigator': 'Iniciado por',
      'affected': 'Afetado',
      'amount': 'Quantidade',
      'primaryObject': 'Objeto',
      'secondaryObject': 'Contexto',
      'segmentText': 'Texto da entrada',
      'other': '$fallback',
    });
    return '$_temp0';
  }

  @override
  String memoryEventGameTime(int day, String time) {
    return 'Dia $day, $time';
  }

  @override
  String memoryEventSecondsValue(String value) {
    return '$value s';
  }

  @override
  String memoryEventMoreValues(String values, int count) {
    return '$values +$count';
  }

  @override
  String get memoryEventHero => 'Herói';

  @override
  String get memoryEventDetails => 'Detalhes';

  @override
  String get memoryEventTags => 'Tags';

  @override
  String get memoryEventTechnicalData => 'Dados técnicos';

  @override
  String get memoryEventIndex => 'Índice';

  @override
  String get memoryEventPosition => 'Posição';

  @override
  String get memoryEventPayload => 'Conteúdo';

  @override
  String get memoryEventSubject => 'Assunto';

  @override
  String glossaryCatalogSegmentLabel(String segmentId, String fallback) {
    String _temp0 = intl.Intl.selectLogic(segmentId, {
      'Access': 'Acesso',
      'AccessDenied': 'Acesso negado',
      'AccesToTemple': 'Acesso ao templo',
      'Advice': 'Conselho',
      'AfterFight': 'Depois da luta',
      'AfterFireMages': 'Depois dos Magos do Fogo',
      'AfterNek': 'Depois de Nek',
      'AfterQuest': 'Depois da missão',
      'Alone': 'Sozinho',
      'Amulet': 'Amuleto',
      'Annoying': 'Irritante',
      'Armor': 'Armadura',
      'Avoid': 'Evitar',
      'Backstory': 'História',
      'BackStory': 'História',
      'BasicMagic': 'Magia básica',
      'Beated': 'Derrotado',
      'BecomeMercenary': 'Tornar-se mercenário',
      'Beer': 'Cerveja',
      'Bestiary': 'Bestiário',
      'Blessing': 'Bênção',
      'Boss': 'Chefe',
      'Bully': 'Valentão',
      'BullyAdvice': 'Conselho sobre o valentão',
      'Camp': 'Acampamento',
      'CampDivided': 'Acampamento dividido',
      'CareOfMessengers': 'Cuidar dos mensageiros',
      'ChangeOpinion': 'Mudar de opinião',
      'ChargeUriziel': 'Carregar Uriziel',
      'Chosen': 'Escolhido',
      'Contact': 'Contato',
      'Courier': 'Mensageiro',
      'CraftBows': 'Fabricação de arcos',
      'Crazy': 'Louco',
      'DailyMeal': 'Refeição diária',
      'DailyRation_Trader': 'Comerciante de ração diária',
      'DAM': 'Barragem',
      'Dead': 'Morto',
      'Deal': 'Acordo',
      'Dealer': 'Negociante',
      'Deceived': 'Enganado',
      'Dementia': 'Demência',
      'DenyAccess': 'Acesso negado',
      'DifferentOpinion': 'Opinião diferente',
      'Discussion': 'Discussão',
      'DontTalk': 'Não conversar',
      'Duel': 'Duelo',
      'Entrance': 'Entrada',
      'Escape': 'Fuga',
      'Extended': 'Estendido',
      'Extra': 'Extra',
      'ExtraInfo': 'Informação adicional',
      'Fanatic': 'Fanático',
      'Fight': 'Luta',
      'FindUlumulu': 'Encontrar Ulu-Mulu',
      'FireMages': 'Magos do Fogo',
      'FireMagesEscape': 'Fuga dos Magos do Fogo',
      'FiskNewDealer': 'Novo receptador para Fisk',
      'FiskNewDealerCompleted': 'Novo receptador para Fisk — concluído',
      'FogTower': 'Torre da Névoa',
      'Food': 'Comida',
      'Forgave': 'Perdoou',
      'Forgive': 'Perdoar',
      'Forgiven': 'Perdoado',
      'FourFriends': 'Quatro amigos',
      'FreeHut': 'Cabana disponível',
      'FreeMine': 'Mina Livre',
      'Fury': 'Fúria',
      'GoodTeacher': 'Bom treinador',
      'Gossip': 'Fofoca',
      'GotScavenger': 'Catador obtido',
      'GrantedAccess': 'Acesso concedido',
      'GRDArmor': 'Armadura de guarda',
      'Guide': 'Guia',
      'HateMages': 'Ódio aos magos',
      'HateMagesExplanation': 'Motivo do ódio aos magos',
      'HateRiceLord': 'Ódio ao Lorde do Arroz',
      'Heal': 'Cura',
      'Healing': 'Cura',
      'Help': 'Ajuda',
      'Helper': 'Ajudante',
      'HelpKagan': 'Ajudar Kagan',
      'HutStory': 'História da cabana',
      'Ignore': 'Ignorar',
      'Impress': 'Impressionar',
      'ImpressAlchemy': 'Impressionar — alquimia',
      'ImpressInscription': 'Impressionar — inscrições',
      'Info': 'Informação',
      'Interested': 'Interessado',
      'Introduction': 'Apresentação',
      'Introduction_2': 'Apresentação 2',
      'Introduction_Armor': 'Apresentação de armaduras',
      'Introduction_Teacher': 'Apresentação — treinador',
      'Introduction_Trader': 'Apresentação — comerciante',
      'Invocation': 'Invocação',
      'JoinSC': 'Entrada no Assentamento do Pântano',
      'Joint': 'Cigarro de maconha-do-pântano',
      'KalomCamp': 'Acampamento de Cor Kalom',
      'Leader': 'Líder',
      'Learning': 'Aprendizado',
      'LearnOrcish': 'Aprender a língua dos orcs',
      'LeftParty': 'Saiu do grupo',
      'Library': 'Biblioteca',
      'Lie': 'Mentira',
      'Lock': 'Fechadura',
      'Lockpick': 'Gazua',
      'Mad': 'Insano',
      'Mandibles': 'Mandíbulas',
      'MapMaker': 'Cartógrafo',
      'Monastery': 'Mosteiro',
      'MordragKO': 'Mordrag nocauteado',
      'Nek': 'Nek',
      'NewCamp': 'Assentamento Novo',
      'NewCamper': 'Novo integrante do acampamento',
      'NewLeader': 'Novo líder',
      'NightPatrol': 'Patrulha noturna',
      'NotInterested': 'Sem interesse',
      'OldCamp': 'Assentamento Antigo',
      'OrcEnclaveEntrance': 'Entrada do Enclave dos Orcs',
      'OrcGraveyard': 'Cemitério dos orcs',
      'OreArmor': 'Armadura de Minério',
      'Party': 'Grupo',
      'Pay': 'Pagamento',
      'PayMoney': 'Pagar',
      'Permission': 'Permissão',
      'Pet': 'Mascote',
      'PreparingInvocation': 'Preparação da invocação',
      'Quest': 'Missão',
      'RankUpFireMages': 'Promoção a Mago do Fogo',
      'RankUpGuard': 'Promoção a guarda',
      'RanUpFireMagesCompleted': 'Promoção a Mago do Fogo concluída',
      'Realocated': 'Realocado',
      'Reason': 'Motivo',
      'Respect': 'Respeito',
      'ReturnToSC': 'Retorno ao Assentamento do Pântano',
      'RicelordForeman': 'Capataz do Lorde do Arroz',
      'RideScavenger': 'Montar um catador',
      'Robe': 'Túnica',
      'Safe': 'Em segurança',
      'Scraper': 'Sucateiro',
      'SecondChance': 'Segunda chance',
      'SecretLocation': 'Local secreto',
      'SecretPassage': 'Passagem secreta',
      'SecretPath': 'Caminho secreto',
      'SleeperFollower': 'Seguidor do Adormecido',
      'SleeperTemple': 'Templo do Adormecido',
      'SmallInfo': 'Pequena informação',
      'Stonehenge': 'Círculo de Pedra',
      'StopFollowing': 'Parar de seguir',
      'SwampCamp': 'Assentamento do Pântano',
      'Talkative': 'Falante',
      'Teach': 'Treinamento',
      'TeachBow': 'Treino com arco',
      'Teacher': 'Treinador',
      'Teacher2': 'Treinador 2',
      'TeacherInscription': 'Treinador de inscrições',
      'TeacherMana': 'Treinador de mana',
      'TeachIchor': 'Treino de icor',
      'TeachMagic': 'Treino de magia',
      'TeachOrcish': 'Ensinar a língua dos orcs',
      'TeachStats': 'Treino de atributos',
      'TeachWeapon': 'Treino com armas',
      'Teleport': 'Teleporte',
      'TheMysteriousOrc': 'O orc misterioso',
      'ThroneRoom': 'Sala do trono',
      'TradeBow': 'Comércio de arcos',
      'Trader': 'Comerciante',
      'TradeSkins_Trader': 'Comerciante de peles',
      'Traitor': 'Traidor',
      'Trial': 'Provação',
      'TrollCanyon': 'Cânion do Troll',
      'Trust': 'Confiança',
      'Ulumulu': 'Ulu-Mulu',
      'Unexperienced': 'Inexperiente',
      'Uriziel': 'Uriziel',
      'UrizielRune': 'Runa de Uriziel',
      'Useful': 'Útil',
      'Velaya': 'Velaya',
      'Vibrations': 'Vibrações',
      'WaitFreeMine': 'Espera na Mina Livre',
      'WaitInTrainingArea': 'Espera na área de treino',
      'Warning': 'Aviso',
      'WarningTooLate': 'Aviso tardio',
      'WaterMessenger': 'Mensageiro dos Magos da Água',
      'Weapon': 'Arma',
      'Who': 'Quem é',
      'Women': 'Mulheres',
      'other': '$fallback',
    });
    return '$_temp0';
  }

  @override
  String get slotRepairTitle => 'Espaços de inventário danificados';

  @override
  String slotRepairBody(int count) {
    return 'Este jogo salvo tem $count espaços de inventário cujo id já não corresponde à sua posição — no jogo, largar um desses itens remove outro. O reparo apenas reescreve os ids: nenhum item é adicionado, removido ou alterado. Ao salvar, um backup é criado, como sempre.';
  }

  @override
  String get slotRepairQueued => 'Reparo na fila — salve para aplicar.';

  @override
  String get slotRepairAction => 'Reparar';

  @override
  String get slotRepairDiscard => 'Descartar';

  @override
  String get editorInventorySlotEditConflict =>
      'Uma edição direta de um espaço de inventário está na fila junto com uma operação que ocupa espaços inteiros (reparo, adição ou remoção). A segunda sobrescreveria a primeira — reverta uma delas e salve novamente.';

  @override
  String get editorTraderArrayConflict =>
      'Uma alteração de comércio está na fila junto com uma edição direta do array de mercadores. Essa edição renumera as linhas pelas quais uma alteração de comércio é endereçada, então uma das duas cairia no mercador errado — reverta uma e salve de novo.';

  @override
  String get backupFactFile => 'Arquivo';

  @override
  String get renameBackupTooltip => 'Nomear este backup';

  @override
  String get renameBackupTitle => 'Nomear backup';

  @override
  String get renameBackupLabel => 'Nome';

  @override
  String renameBackupHelp(String fileName) {
    return 'Exibido no lugar do nome do arquivo $fileName. Deixe vazio para remover o nome; o arquivo em si não é renomeado.';
  }

  @override
  String get deleteBackupTooltip => 'Excluir este backup';

  @override
  String get deleteBackupTitle => 'Excluir backup';

  @override
  String deleteBackupBody(String name, String fileName) {
    return 'Excluir “$name” ($fileName)? O arquivo é removido do disco e não pode ser recuperado.';
  }

  @override
  String get deleteBackupConfirm => 'Excluir';

  @override
  String editorDeletedBackup(String path) {
    return 'Backup excluído: $path';
  }

  @override
  String editorDeleteBackupFailed(String details) {
    return 'Não foi possível excluir o backup: $details';
  }

  @override
  String editorRenameBackupFailed(String details) {
    return 'Não foi possível nomear o backup: $details';
  }

  @override
  String get slotRepairUnavailable =>
      'No momento não é possível reparar — este jogo salvo não pode ser gravado.';

  @override
  String editorDeletedBackupWithLabelWarning(String path, String details) {
    return 'Backup excluído: $path — não foi possível remover o nome dele: $details';
  }

  @override
  String get slotRepairNotOffered =>
      'O reparo não está disponível para este jogo salvo.';
}
