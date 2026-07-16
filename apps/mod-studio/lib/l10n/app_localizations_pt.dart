// ignore: unused_import
import 'package:intl/intl.dart' as intl;
import 'app_localizations.dart';

// ignore_for_file: type=lint

/// The translations for Portuguese (`pt`).
class AppLocalizationsPt extends AppLocalizations {
  AppLocalizationsPt([String locale = 'pt']) : super(locale);

  @override
  String get tabItems => 'Items';

  @override
  String get tabOverrides => 'Changes';

  @override
  String get tabSettings => 'Settings';

  @override
  String get tabDialogs => 'Diálogos';

  @override
  String get tabAudio => 'Áudio';

  @override
  String get tabTextures => 'Texturas';

  @override
  String get tabScripts => 'Scripts';

  @override
  String get changesAll => 'Todos';

  @override
  String get sectionItemValues => 'Valores dos itens';

  @override
  String get sectionLocalizedText => 'Textos localizados';

  @override
  String get audioCatCreatures => 'Criaturas';

  @override
  String get audioCatObjects => 'Objetos';

  @override
  String get audioCatMagic => 'Magia';

  @override
  String get audioCatMovement => 'Movimento';

  @override
  String get audioCatWorld => 'Mundo';

  @override
  String get audioCatAction => 'Ações';

  @override
  String get audioCatCombat => 'Combate';

  @override
  String get audioCatPhysics => 'Física';

  @override
  String get audioCatItems => 'Itens';

  @override
  String get audioCatUi => 'Interface';

  @override
  String get audioCatFoley => 'Foley';

  @override
  String get audioCatUnderwater => 'Subaquático';

  @override
  String get audioCatVision => 'Visões';

  @override
  String get audioCatDialog => 'Diálogo';

  @override
  String get audioCatOther => 'Outros';

  @override
  String get gameExecutable => 'Game executable';

  @override
  String get gameExecutableSubtitle =>
      'Path to the game\'s .exe. Used to auto-detect localized text and the game install.';

  @override
  String get gameExecutableNotSet => 'Not set';

  @override
  String get chooseGameExecutable => 'Choose…';

  @override
  String get settingsDataSourceSection => 'Game data';

  @override
  String get settingsLocalizationSection => 'Localized text';

  @override
  String get extractLocalizedText => 'Extrair textos localizados';

  @override
  String get lightMode => 'Modo claro';

  @override
  String get darkMode => 'Modo escuro';

  @override
  String get language => 'Idioma';

  @override
  String get exportMod => 'Exportar mod';

  @override
  String exportModWithCount(int count) {
    return 'Exportar mod ($count)';
  }

  @override
  String get selectAnItemToEdit => 'Selecione um item para editar seus campos.';

  @override
  String gameDataActiveTooltip(String name) {
    return 'Dados do jogo: $name';
  }

  @override
  String get gameDataBundledTooltip => 'Dados do jogo: incluídos';

  @override
  String get loadGameDataDump => 'Carregar dump de dados do jogo…';

  @override
  String get loadGameDataDumpSubtitle => 'gore_game_data.json do mod gore-dump';

  @override
  String get useBundledData => 'Usar dados incluídos';

  @override
  String get alreadyBundled => 'já incluídos';

  @override
  String get gameDataFileGroupLabel => 'dados do jogo';

  @override
  String get minimize => 'Minimizar';

  @override
  String get restore => 'Restaurar';

  @override
  String get maximize => 'Maximizar';

  @override
  String get close => 'Fechar';

  @override
  String get about => 'Sobre';

  @override
  String aboutVersion(String version, String sha) {
    return 'Version $version ($sha)';
  }

  @override
  String get aboutCopyright => '© 2026 colaboradores do GORE';

  @override
  String get aboutLicense => 'Licenciado sob a Licença MIT.';

  @override
  String get categoryMeleeWeapons => 'Armas corpo a corpo';

  @override
  String get categoryRangedWeapons => 'Armas à distância';

  @override
  String get categoryAmmunition => 'Munição';

  @override
  String get categoryRunes => 'Runas';

  @override
  String get categorySpellScrolls => 'Pergaminhos de magia';

  @override
  String get categoryFoodAndPotions => 'Comida e poções';

  @override
  String get categoryMiscellaneous => 'Diversos';

  @override
  String get categoryAmulets => 'Amuletos';

  @override
  String get categoryRings => 'Anéis';

  @override
  String get categoryAnimalTrophies => 'Troféus de animais';

  @override
  String get categoryWritings => 'Escritos';

  @override
  String get categoryMissionItems => 'Itens de missão';

  @override
  String get categoryKeys => 'Chaves';

  @override
  String get categoryOther => 'Outros';

  @override
  String categoryWithCount(String label, int count) {
    return '$label ($count)';
  }

  @override
  String get searchItems => 'Pesquisar itens';

  @override
  String get noItemsMatch => 'Nenhum item corresponde';

  @override
  String failedToLoadCatalog(String error) {
    return 'Falha ao carregar o catálogo: $error';
  }

  @override
  String pendingOverridesWithCount(int count) {
    return 'Alterações pendentes ($count)';
  }

  @override
  String get clearAll => 'Limpar tudo';

  @override
  String get noPendingOverrides =>
      'Nenhuma alteração pendente.\nEdite os campos dos itens para adicionar algumas.';

  @override
  String get removeOverride => 'Remover alteração';

  @override
  String get searchChanges => 'Pesquisar alterações';

  @override
  String get noChangesMatch => 'Nenhuma alteração corresponde';

  @override
  String get clearSection => 'Limpar este grupo';

  @override
  String get modName => 'Nome do mod';

  @override
  String get loadDelayLabel => 'Atraso de carregamento (ms, 0 = imediato)';

  @override
  String get noFolderSelected => 'Nenhuma pasta selecionada';

  @override
  String get chooseFolder => 'Escolher pasta';

  @override
  String get packageAsZip => 'Empacotar como .zip';

  @override
  String get cancel => 'Cancelar';

  @override
  String get export => 'Exportar';

  @override
  String get exportHere => 'Exportar aqui';

  @override
  String get mustBeNonNegativeInteger => 'Deve ser um inteiro não negativo';

  @override
  String get extractingLocalizedText => 'Extraindo textos localizados do jogo…';

  @override
  String get localizedTextExtractionCancelled =>
      'Extração de textos localizados cancelada.';

  @override
  String get localizedTextExtracted => 'Textos localizados extraídos.';

  @override
  String get extractionFailed => 'Falha na extração.';

  @override
  String get localizationCacheFileGroupLabel => 'cache de localização';

  @override
  String get extractLocalizedTextQuestion =>
      'Extrair os textos localizados do jogo?';

  @override
  String get extractLocalizedTextBody =>
      'Os textos localizados do jogo ainda não foram extraídos. Extraí-los agora da sua instalação do jogo? (opcional)';

  @override
  String get notNow => 'Agora não';

  @override
  String get extract => 'Extrair';

  @override
  String get validationRequired => 'Obrigatório';

  @override
  String get validationMustBeWholeNumber => 'Deve ser um número inteiro';

  @override
  String get validationMustBeNumber => 'Deve ser um número';

  @override
  String get validationMustBeFinite => 'Deve ser um número finito';

  @override
  String validationMustBeAtLeast(String min) {
    return 'Deve ser ≥ $min';
  }

  @override
  String validationMustBeAtMost(String max) {
    return 'Deve ser ≤ $max';
  }

  @override
  String get validationMustBeBool => 'Deve ser true ou false';

  @override
  String validationMustBeOneOf(String options) {
    return 'Deve ser um de: $options';
  }

  @override
  String get modNameRequired => 'Obrigatório';

  @override
  String get modNameControlCharacters =>
      'Não deve conter caracteres de controle';

  @override
  String get modNamePathSeparators => 'Não deve conter separadores de caminho';

  @override
  String get modNameNotAFolderName => 'Nome de pasta inválido';

  @override
  String localizedTextExtractedCount(int idCount, int languageCount) {
    return '$idCount IDs extraídos em $languageCount idiomas';
  }

  @override
  String get managerDeployActive =>
      'Um loadout do mod-manager está ativo. Faça primeiro o undeploy no gore-manager.';

  @override
  String get projectOpenLegacy => 'Open legacy project…';

  @override
  String get projectOpenManagedRevision3 => 'Open managed revision-3 project…';

  @override
  String get projectVerifyCurrentHead => 'Verify current head';

  @override
  String get projectManagedRevision3Title => 'Managed revision-3 project';

  @override
  String get projectClose => 'Close project';

  @override
  String projectCloseFailed(String error) {
    return 'Project could not be closed: $error';
  }

  @override
  String get projectManagedRevision3IdentityOnly =>
      'This shell currently exposes verified project identity only. Ctrl+S reopens and verifies the exact current head; legacy editors, Build/Deploy, and Save As are unavailable.';

  @override
  String get projectRoot => 'Root';

  @override
  String get projectId => 'Project ID';

  @override
  String get projectRevision => 'Project revision';

  @override
  String get projectHeadSha256 => 'Head SHA-256';

  @override
  String get projectSnapshotBytes => 'Snapshot bytes';

  @override
  String get projectNoCurrent => 'No current project';

  @override
  String projectManagedRevision3Opened(String projectId) {
    return 'Opened managed revision-3 project $projectId';
  }

  @override
  String projectManagedRevision3OpenFailed(String error) {
    return 'Managed revision-3 project open failed: $error';
  }

  @override
  String projectManagedRevision3Verified(String headSha256) {
    return 'Verified revision-3 head $headSha256';
  }

  @override
  String projectManagedRevision3VerifyFailed(String error) {
    return 'Revision-3 head verification failed: $error';
  }

  @override
  String get projectManagedRevision3RequiresReopen =>
      'Exact-head verification could not complete safely. This session now requires recovery and further verification is blocked. Close Mod Studio, then reopen this project before continuing.';

  @override
  String get projectManagedRevision3VerifyBlocked =>
      'Verification is blocked until the managed project is reopened.';

  @override
  String get projectTransitionCleanupWarning =>
      'O novo projeto está aberto, mas não foi possível limpar completamente a sessão do projeto anterior. A limpeza não será repetida. Reinicie o Mod Studio antes de voltar a abrir o projeto anterior.';

  @override
  String get projectNewManagedRevision3 => 'Novo projeto de mod gerido…';

  @override
  String get projectNewLegacy => 'Novo projeto legado';

  @override
  String get projectCreateGamePathRequired =>
      'Defina o caminho do Gothic 1 Remake nas Definições antes de criar um projeto de mod.';

  @override
  String get projectCreateDirectoryPickerTitle =>
      'Criar aqui o projeto de mod gerido';

  @override
  String projectManagedRevision3Created(String projectId) {
    return 'Projeto de mod gerido $projectId criado';
  }

  @override
  String projectManagedRevision3CreateFailed(String error) {
    return 'Falha ao criar o projeto de mod gerido: $error';
  }

  @override
  String get projectCreateDialogTitle => 'Criar um projeto de mod';

  @override
  String get projectCreateNameLabel => 'Nome do projeto';

  @override
  String get projectCreateNameHelper => 'O nome apresentado no Mod Studio.';

  @override
  String get projectCreateVersionLabel => 'Versão';

  @override
  String get projectCreateVersionHelper => 'Uma versão inicial, como 0.1.0.';

  @override
  String get projectCreateAuthorLabel => 'Autor';

  @override
  String get projectCreateAuthorHelper =>
      'O seu nome ou o nome da equipa de modding.';

  @override
  String get projectCreateLocalesLabel => 'Idiomas de edição';

  @override
  String get projectCreateLocalesHelper =>
      'Etiquetas canónicas separadas por vírgulas, por exemplo: en, de, en-US.';

  @override
  String get projectCreateBoundary =>
      'Isto cria um projeto offline gerido e vazio. Não compila, implementa nem executa um mod e não altera ficheiros do jogo ou gravações.';

  @override
  String get projectCreateSubmit => 'Criar projeto';

  @override
  String projectCreateMetadataRequired(String label) {
    return '$label é obrigatório.';
  }

  @override
  String projectCreateMetadataNoOuterWhitespace(String label) {
    return '$label não pode começar nem terminar com espaços.';
  }

  @override
  String projectCreateMetadataControlCharacters(String label) {
    return '$label não pode conter caracteres de controlo.';
  }

  @override
  String projectCreateMetadataMalformed(String label) {
    return '$label contém texto inválido.';
  }

  @override
  String projectCreateMetadataTooLong(String label, int maxBytes) {
    return '$label excede o limite UTF-8 de $maxBytes bytes.';
  }

  @override
  String get projectCreateLocalesRequired =>
      'Introduza pelo menos um idioma de edição.';

  @override
  String get projectCreateLocalesEmptyEntry =>
      'Remova a entrada de idioma vazia.';

  @override
  String projectCreateLocalesTooMany(int maxLocales) {
    return 'Utilize no máximo $maxLocales idiomas de edição.';
  }

  @override
  String projectCreateLocaleBoundedAscii(String locale) {
    return 'A região «$locale» tem de ser ASCII e ter comprimento limitado.';
  }

  @override
  String projectCreateLocaleLanguage(String locale) {
    return 'A região «$locale» necessita de um idioma em minúsculas com 2 a 8 letras.';
  }

  @override
  String projectCreateLocaleInvalidSegment(String locale) {
    return 'A região «$locale» contém um segmento inválido.';
  }

  @override
  String projectCreateLocaleNotCanonical(String locale, String canonical) {
    return 'A região «$locale» não é canónica; utilize «$canonical».';
  }

  @override
  String get managedWorkspaceOverviewLabel => 'Visão geral';

  @override
  String get managedWorkspaceContentLabel => 'Conteúdo';

  @override
  String get managedWorkspaceDataAssetsLabel => 'DataAssets';

  @override
  String get managedContentWorkspaceLibraryLabel => 'Este mod';

  @override
  String get managedWorkspaceHomeLabel => 'Início';

  @override
  String get managedWorkspaceStoryLabel => 'História';

  @override
  String get managedWorkspaceWorldLabel => 'Mundo';

  @override
  String get managedWorkspaceLocalizationVoiceLabel => 'Localização e vozes';

  @override
  String get managedWorkspaceValidateTestLabel => 'Validar e testar';

  @override
  String get managedWorkspaceBuildReleaseLabel => 'Compilar e publicar';

  @override
  String get managedWorkspaceSettingsExpertLabel =>
      'Definições e modo especialista';

  @override
  String get managedSectionStoryDescription => 'NPCs, missões e diálogos.';

  @override
  String get managedStoryWorkspaceLoading =>
      'Opening the current Story drafts…';

  @override
  String get managedStoryWorkspaceAuthorityNotice =>
      'Project-only NPC and Quest drafts. Build readiness has not been evaluated; runtime behavior remains unqualified.';

  @override
  String get managedStoryWorkspaceSearchHint =>
      'Search NPC and Quest names, objectives, speakers, or IDs';

  @override
  String get managedStoryWorkspaceCreatingNpc => 'Creating NPC draft…';

  @override
  String get managedStoryWorkspaceCreatingQuest => 'Creating Quest draft…';

  @override
  String get managedStoryWorkspaceEmpty => 'No NPC or Quest drafts yet';

  @override
  String get managedStoryWorkspaceNoMatches =>
      'No NPC or Quest drafts match this search';

  @override
  String get managedStoryWorkspaceSelectDraft =>
      'Select an NPC or Quest draft to continue';

  @override
  String get managedStoryWorkspaceLoadErrorTitle =>
      'Story drafts could not be opened';

  @override
  String get managedStoryWorkspaceCheckpointMismatch =>
      'The project changed while Story was loading. Refresh the exact current checkpoint and try again.';

  @override
  String get managedStoryWorkspacePublishedSelectionStale =>
      'The saved Story draft could not be selected at its exact project revision. Check the current Story list before continuing.';

  @override
  String managedStoryWorkspaceCheckpointSummary(int count, int revision) {
    return 'NPC and Quest drafts: $count · project revision $revision';
  }

  @override
  String managedStoryWorkspaceLoadErrorDetails(String error) {
    return 'The exact current Story view could not be read: $error';
  }

  @override
  String managedStoryWorkspaceCreateErrorDetails(String error) {
    return 'The Story draft could not be created: $error';
  }

  @override
  String managedStoryWorkspaceDetailsSheetLabel(String entityName) {
    return '$entityName Story details';
  }

  @override
  String get managedStoryWorkspaceRemovePairUnavailable =>
      'This draft is not an exact removable draft and generated-script pair.';

  @override
  String get managedStoryWorkspaceRemoveBusy =>
      'Another Story action is still in progress.';

  @override
  String get managedStoryWorkspaceRemoveRequiresReopen =>
      'Reopen this managed project before removing a draft.';

  @override
  String managedStoryWorkspaceRemoveBlocked(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count incoming project references must be removed first.',
      one: '1 incoming project reference must be removed first.',
    );
    return '$_temp0';
  }

  @override
  String get managedStoryWorkspaceRemoveDialogTitle =>
      'Remove draft from project?';

  @override
  String managedStoryWorkspaceRemoveDialogSummary(
    String draftName,
    String scriptName,
  ) {
    return 'This removes the draft \'$draftName\' together with its uniquely owned generated script \'$scriptName\'.';
  }

  @override
  String get managedStoryWorkspaceRemoveNoUndo =>
      'This removal cannot be undone in version 1.';

  @override
  String get managedStoryWorkspaceRemoveBoundary =>
      'Only the current project registry is changed. The game installation and save games stay unchanged.';

  @override
  String get managedStoryWorkspaceRemoveCancel => 'Cancel';

  @override
  String get managedStoryWorkspaceRemoveConfirm => 'Remove draft';

  @override
  String get managedStoryWorkspaceRemoveBlockedTitle =>
      'Draft is still referenced';

  @override
  String get managedStoryWorkspaceRemoveBlockedDescription =>
      'Open every source below and remove its project reference before trying again.';

  @override
  String managedStoryWorkspaceRemoveBlockerLabel(
    String sourceName,
    String role,
  ) {
    return '$sourceName · $role';
  }

  @override
  String get managedStoryWorkspaceRemoveOpenBlocker =>
      'Open referencing source';

  @override
  String get managedStoryWorkspaceRemoveBlockedClose => 'Close';

  @override
  String managedStoryWorkspaceRemoveSucceeded(String draftName) {
    return 'Removed \'$draftName\' and its generated script from the project. Game files and save games were not changed.';
  }

  @override
  String managedStoryWorkspaceRemoveError(String error) {
    return 'The draft was not removed. The Story view was refreshed without retrying automatically: $error';
  }

  @override
  String get managedSectionWorldDescription =>
      'A colocação no mundo e os fluxos associados estão planeados.';

  @override
  String get managedSectionLocalizationVoiceDescription =>
      'Escreva e traduza os diálogos do projeto num só lugar e continue depois com as vozes.';

  @override
  String get managedLocalizationProjectTextsLabel => 'Project texts';

  @override
  String get managedLocalizationSearchLabel => 'Search project texts';

  @override
  String get managedLocalizationRefresh => 'Refresh';

  @override
  String get managedLocalizationEmptyTitle => 'No project text yet';

  @override
  String get managedLocalizationEmptyDescription =>
      'Create a dialog line to start writing and translating text.';

  @override
  String get managedLocalizationLoadFailed =>
      'Project texts could not be opened';

  @override
  String get managedLocalizationSelectText => 'Select a project text to edit';

  @override
  String get managedLocalizationLanguagesLabel => 'Languages';

  @override
  String get managedLocalizationUsedByLines => 'Used by dialog lines';

  @override
  String get managedLocalizationNoLine => 'Not used by a dialog line yet';

  @override
  String get managedLocalizationSpeakerLabel => 'Speaker label';

  @override
  String get managedLocalizationAddLanguage => 'Add language';

  @override
  String get managedLocalizationRemoveLanguage => 'Remove language';

  @override
  String get managedLocalizationLanguageHint => 'For example de, en, or pt-BR';

  @override
  String get managedLocalizationLanguageExists =>
      'This language is already present.';

  @override
  String get managedLocalizationAdd => 'Add';

  @override
  String get managedLocalizationSaved => 'Project text saved';

  @override
  String get managedLocalizationVoiceLocked =>
      'This text has recorded voice takes, so its transcript is locked in this editor.';

  @override
  String get managedLocalizationVoiceSlotRemovalLocked =>
      'This language is connected to a Voice slot and cannot be removed here.';

  @override
  String get managedLocalizationMinimumLanguageLocked =>
      'Keep at least one language for this project text.';

  @override
  String get managedLocalizationSharedNotice =>
      'This project text is shared. Saving changes updates every listed dialog line.';

  @override
  String get managedLocalizationOfflineNotice =>
      'Changes are saved only to this managed project. Build and in-game behavior remain separate.';

  @override
  String get managedLocalizationUnsavedTitle => 'Discard unsaved changes?';

  @override
  String get managedLocalizationUnsavedDescription =>
      'You changed this project text. Switching now would discard those edits.';

  @override
  String get managedLocalizationDiscard => 'Discard changes';

  @override
  String get managedLocalizationKeepEditing => 'Keep editing';

  @override
  String get managedLocalizationStale =>
      'The project changed while this text was open. Refresh and try again.';

  @override
  String get managedLocalizationReopen =>
      'The project must be reopened before text editing can continue.';

  @override
  String get managedLocalizationInvalid =>
      'Check that every language and dialog text is valid and not empty.';

  @override
  String get managedLocalizationSaveFailed =>
      'The project text could not be saved.';

  @override
  String get managedSectionValidateTestDescription =>
      'Verifica a integridade exata do projeto e os pontos de controlo; não afirma um teste em execução.';

  @override
  String get managedSectionBuildReleaseDescription =>
      'Os pacotes de vozes estão disponíveis; as compilações jogáveis completas e a implementação não estão.';

  @override
  String get managedSectionSettingsExpertDescription =>
      'As definições estão disponíveis; as ferramentas especializadas ainda não estão integradas.';

  @override
  String get managedSectionStatusHeading => 'Estado';

  @override
  String get managedSectionActionsHeading => 'Ações';

  @override
  String get managedCapabilityAvailable => 'Disponível';

  @override
  String get managedCapabilityPartial => 'Parcial';

  @override
  String get managedCapabilityPlanned => 'Planeado';

  @override
  String get managedCapabilityUnavailable => 'Indisponível';

  @override
  String get managedProjectSubtitle =>
      'Área de criação offline correspondente exatamente à versão atual';

  @override
  String get managedProjectLandingTitle => 'Área de trabalho do projeto gerido';

  @override
  String get managedProjectLandingDescription =>
      'Utilize o novo fluxo Início, Conteúdo, História, Voz, validação e lançamento num único projeto gerido.';

  @override
  String get legacyCompatibilityToolsTitle =>
      'Ferramentas de compatibilidade antigas';

  @override
  String get legacyCompatibilityToolsDescription =>
      'Os separadores abaixo contêm ferramentas antigas de substituição direta. Continuam disponíveis enquanto a área de trabalho do projeto gerido evolui.';

  @override
  String get managedProjectTechnicalDetails => 'Detalhes técnicos do projeto';

  @override
  String get managedProjectRecoveryContentLocked =>
      'Volte a abrir o projeto gerido antes de ler o respetivo conteúdo.';

  @override
  String get managedProjectRecoveryDescription =>
      'Mod Studio will safely reopen this project while keeping its lock. This does not change the game or any save.';

  @override
  String get managedProjectRecoveryTry => 'Try recovery';

  @override
  String get managedProjectRecoveryTrying => 'Trying recovery…';

  @override
  String get managedProjectRecoveryAlternative =>
      'If recovery does not work, close and open the project again.';

  @override
  String get managedProjectRecoverySucceeded =>
      'Project recovery completed. You can continue working.';

  @override
  String get managedProjectRecoveryFailed =>
      'Recovery did not complete. Try again, or close and open the project again.';

  @override
  String get managedProjectRecoveryUnavailable =>
      'Recovery is not available for this project. Close and open the project again.';

  @override
  String get managedDashboardUntitledProject => 'Projeto sem título';

  @override
  String get managedDashboardDraftStatus => 'Rascunho';

  @override
  String get managedDashboardProjectVersion => 'Versão';

  @override
  String get managedDashboardProjectAuthor => 'Autor';

  @override
  String get managedDashboardNotProvided => 'Não indicado';

  @override
  String get managedDashboardContentCounts => 'Conteúdo do projeto';

  @override
  String get managedDashboardNpcDrafts => 'Rascunhos de NPC';

  @override
  String get managedDashboardQuestDrafts => 'Rascunhos de missões';

  @override
  String get managedDashboardDialogLines => 'Linhas de diálogo';

  @override
  String get managedDashboardVoiceTakes => 'Gravações de voz';

  @override
  String get managedDashboardAssets => 'Recursos';

  @override
  String get managedDashboardUnresolvedReferences => 'Referências por resolver';

  @override
  String get managedDashboardReadiness => 'O que funciona agora';

  @override
  String get managedDashboardOfflineAuthoringTitle =>
      'Criação offline disponível';

  @override
  String get managedDashboardOfflineAuthoringDescription =>
      'Crie e edite conteúdos de projeto suportados sem alterar a instalação do jogo nem os ficheiros guardados.';

  @override
  String get managedDashboardGeneralBuildBlockedTitle =>
      'Compilação geral do mod indisponível';

  @override
  String get managedDashboardGeneralBuildBlockedDescription =>
      'Apenas podem ser compilados pacotes Voice offline selados; ainda não é possível compilar um mod completo e jogável.';

  @override
  String get managedDashboardRuntimeUnqualifiedTitle =>
      'Execução ainda não verificada';

  @override
  String get managedDashboardRuntimeUnqualifiedDescription =>
      'O Mod Studio ainda não comprovou este conteúdo do projeto no jogo em execução.';

  @override
  String get managedDashboardReferenceIntegrityTitle =>
      'Integridade das referências';

  @override
  String get managedDashboardReferenceIntegrityDescription =>
      'Esta contagem verifica apenas referências do projeto; não indica que esteja pronto para compilação ou execução.';

  @override
  String get managedDashboardMissingGameTitle =>
      'Configuração do jogo necessária';

  @override
  String get managedDashboardMissingGameDescription =>
      'Configure a instalação de Gothic 1 Remake nas Definições antes de usar ações que necessitem de dados verificados do jogo instalado.';

  @override
  String get managedDashboardCreateHeading => 'Criar';

  @override
  String get managedDashboardToolsHeading => 'Ferramentas do projeto';

  @override
  String get managedDashboardLoading => 'A carregar a visão geral do projeto';

  @override
  String get managedDashboardLoadError => 'Visão geral do projeto indisponível';

  @override
  String get managedDashboardLoadErrorDescription =>
      'Não foi possível carregar a visão geral verificada do projeto. O conteúdo do projeto não foi alterado.';

  @override
  String get managedDashboardRetry => 'Tentar novamente';

  @override
  String get managedActionNewNpcTitle => 'Novo NPC';

  @override
  String get managedActionNewNpcDescription =>
      'Crie um rascunho de NPC offline e limitado a partir de dados verificados do jogo instalado.';

  @override
  String get managedActionNewQuestTitle => 'Nova missão';

  @override
  String get managedActionNewQuestDescription =>
      'Crie um rascunho de missão offline com objetivos e identidades principais verificadas.';

  @override
  String get managedActionNewDialogLineTitle => 'Adicionar linha de diálogo';

  @override
  String get managedActionNewDialogLineDescription =>
      'Escreva texto localizado do projeto ou associe um texto ainda não usado deste projeto. Isto não cria um tópico de diálogo jogável.';

  @override
  String managedActionNewDialogLineSaved(int projectRevision) {
    return 'Linha de diálogo guardada na revisão $projectRevision do projeto. O jogo e os ficheiros guardados não foram alterados.';
  }

  @override
  String get managedDialogLineIntroduction =>
      'Escreva uma nova linha de diálogo localizada ou associe texto que já pertence a este projeto.';

  @override
  String get managedDialogLineBoundary =>
      'Apenas os ficheiros do projeto são alterados. Isto não cria um tópico AngelScript nem um diálogo jogável e nunca altera a instalação do jogo ou os ficheiros guardados. O campo do interlocutor é apenas uma etiqueta; não associa nenhum NPC.';

  @override
  String get managedDialogLineCreateMode => 'Escrever novo texto';

  @override
  String get managedDialogLineReuseMode => 'Usar texto do projeto';

  @override
  String get managedDialogLineNameLabel => 'Nome da linha';

  @override
  String get managedDialogLineNameHint => 'Saudação à entrada da mina';

  @override
  String get managedDialogLineSpeakerLabel =>
      'Etiqueta do interlocutor (opcional)';

  @override
  String get managedDialogLineSpeakerHint => 'Por exemplo, Viper';

  @override
  String get managedDialogLineLocaleLabel => 'Idioma';

  @override
  String get managedDialogLineTextLabel => 'Texto do diálogo';

  @override
  String get managedDialogLineReuseSearch =>
      'Procurar texto do projeto não usado';

  @override
  String get managedDialogLineNoReusableText =>
      'Não existe texto de projeto não usado e estruturalmente válido que possa ser associado. Escreva antes um novo texto.';

  @override
  String get managedDialogLineCreateSlotLabel =>
      'Preparar este idioma para Voice';

  @override
  String get managedDialogLineCreateSlotHelp =>
      'Cria um espaço Voice vazio e não resolvido no projeto. Não adiciona nem implementa uma gravação.';

  @override
  String get managedDialogLineCancel => 'Cancelar';

  @override
  String get managedDialogLineSave => 'Guardar no projeto';

  @override
  String get managedDialogLineSaving => 'A guardar…';

  @override
  String get managedDialogLineLoading => 'A ler o conteúdo exato do projeto…';

  @override
  String get managedDialogLineLoadFailed =>
      'Não foi possível ler o conteúdo atual exato do projeto. Nada foi alterado.';

  @override
  String get managedDialogLineRetry => 'Tentar novamente';

  @override
  String get managedDialogLineStale =>
      'O projeto foi alterado enquanto esta janela estava aberta. Feche-a e tente novamente a partir do projeto atual.';

  @override
  String get managedDialogLineRequiresReopen =>
      'Já não é possível verificar o projeto atual em segurança. Feche esta janela e volte a abrir o projeto gerido.';

  @override
  String get managedDialogLineInvalidInput =>
      'Verifique a entrada do projeto realçada e escolha uma opção atual exata.';

  @override
  String get managedDialogLineSaveFailed =>
      'Não foi possível guardar a linha de diálogo em segurança. O jogo e os ficheiros guardados não foram alterados.';

  @override
  String get managedDialogLineDone => 'Concluído';

  @override
  String get managedDialogLineAddRecording => 'Adicionar gravação';

  @override
  String get managedActionAddVoiceTakeTitle => 'Adicionar gravação de voz';

  @override
  String get managedActionAddVoiceTakeDescription =>
      'Importe uma gravação Ogg Vorbis para este projeto sem a distribuir.';

  @override
  String get managedActionAddVoiceTakeRequiresDialogLine =>
      'Create or repair a dialog line with one valid localization entry before using Voice tools.';

  @override
  String get managedActionManageVoiceTakesTitle => 'Gerir gravações de voz';

  @override
  String get managedActionManageVoiceTakesDescription =>
      'Reveja as gravações e selecione as aprovadas para os espaços Voice.';

  @override
  String get managedActionResolveVoiceTargetTitle => 'Resolver destino Voice';

  @override
  String get managedActionResolveVoiceTargetDescription =>
      'Associe os espaços Voice do projeto aos membros exatos dos arquivos instalados sem alterar o jogo.';

  @override
  String get managedActionBuildVoiceBundleTitle => 'Compilar pacote Voice';

  @override
  String get managedActionBuildVoiceBundleDescription =>
      'Compile um pacote offline selado a partir de membros existentes; não é efetuada qualquer distribuição.';

  @override
  String get managedActionDataAssetsTitle => 'Edições de DataAssets';

  @override
  String get managedActionDataAssetsDescription =>
      'Inspecione os pacotes instalados e prepare no projeto alterações verificadas a valores de largura fixa.';

  @override
  String get managedActionBrowseProjectContentDescription =>
      'Explore o conteúdo exato do projeto e as respetivas referências resolvidas ou não resolvidas.';

  @override
  String get managedActionSettingsTitle => 'Definições';

  @override
  String get managedActionSettingsDescription =>
      'Configure a instalação de Gothic 1 Remake e as preferências do Mod Studio.';

  @override
  String projectStarterSetupOpenFailed(String projectId) {
    return 'O projeto $projectId foi criado em segurança, mas a configuração inicial não abriu. O projeto vazio válido continua ativo.';
  }

  @override
  String projectStarterOutcomeUnverified(String projectId) {
    return 'O projeto $projectId foi criado, mas o Mod Studio não consegue verificar o resultado inicial. Reabra o projeto gerido antes de continuar; o jogo e as gravações não foram alterados.';
  }

  @override
  String projectStarterNpcCancelled(String projectId) {
    return 'O projeto $projectId foi criado. O início de NPC não foi adicionado, por isso o projeto vazio válido continua ativo.';
  }

  @override
  String projectStarterNpcSaved(int projectRevision) {
    return 'Início de NPC guardado na revisão $projectRevision. Continua bloqueado para compilação, não qualificado em execução e não é gerado.';
  }

  @override
  String projectStarterQuestCancelled(String projectId) {
    return 'O projeto $projectId foi criado. O início de missão não foi adicionado, por isso o projeto vazio válido continua ativo.';
  }

  @override
  String projectStarterQuestSaved(int projectRevision) {
    return 'Início de missão guardado na revisão $projectRevision. Continua bloqueado para compilação e não qualificado em execução.';
  }

  @override
  String get projectStarterSemanticsLabel => 'Início do projeto';

  @override
  String get projectStarterPrompt => 'Como pretende começar?';

  @override
  String get projectStarterWriteBoundary =>
      'Escolher um início não escreve nada. O projeto só é criado depois de enviar este formulário e escolher uma pasta vazia.';

  @override
  String get projectStarterEmptyTitle => 'Projeto vazio';

  @override
  String get projectStarterEmptyDescription =>
      'Crie apenas o projeto gerido. Adicione conteúdo quando quiser.';

  @override
  String get projectStarterNpcDraftTitle => 'Rascunho de NPC';

  @override
  String get projectStarterNpcDraftDescription =>
      'Crie primeiro o projeto vazio e depois abra a configuração guiada do rascunho de NPC.';

  @override
  String get projectStarterQuestDraftTitle => 'Rascunho de missão';

  @override
  String get projectStarterQuestDraftDescription =>
      'Crie primeiro o projeto vazio e depois abra a configuração guiada do rascunho de missão.';

  @override
  String get projectStarterPartialOutcome =>
      'Se cancelar a configuração guiada de NPC ou missão, ou se o rascunho falhar, permanece um projeto vazio válido. A escolha não escreve no jogo nem num ficheiro guardado.';

  @override
  String get managedContentWorkspaceBrowseLabel => 'Explorar';

  @override
  String get managedContentWorkspaceVerifiedEditsLabel =>
      'Alterações verificadas';

  @override
  String get managedContentScopeBaseGameLabel => 'Jogo base';

  @override
  String get managedContentScopeInstalledLabel => 'Instalado';

  @override
  String get managedBaseGameBrowserTitle =>
      'Pontos de partida suportados do jogo base';

  @override
  String get managedBaseGameBrowserDescription =>
      'Explore provas exatas do jogo instalado que o Mod Studio pode inspecionar ou usar como ponto de partida seguro para um rascunho. Não é um catálogo completo do conteúdo original.';

  @override
  String get managedBaseGameBrowserLoading =>
      'A ler provas exatas do jogo base…';

  @override
  String get managedBaseGameBrowserRefresh => 'Ler um novo catálogo exato';

  @override
  String get managedBaseGameBrowserSearchLabel =>
      'Pesquisar conteúdo suportado do jogo base';

  @override
  String get managedBaseGameBrowserFilterNpcs => 'NPCs';

  @override
  String get managedBaseGameBrowserFilterQuests => 'Missões';

  @override
  String get managedBaseGameBrowserNpcSectionTitle =>
      'Pontos de partida de NPC';

  @override
  String get managedBaseGameBrowserQuestSectionTitle =>
      'Pontos de partida de missão';

  @override
  String get managedBaseGameBrowserExperimentalNpcSectionTitle =>
      'Arquétipos de NPC apenas para inspeção';

  @override
  String get managedBaseGameBrowserSearchForExperimental =>
      'Pesquise para incluir mais provas de NPC com ligação estática. Essas linhas não permitem criar um rascunho.';

  @override
  String get managedBaseGameBrowserEmpty =>
      'Nenhum resultado suportado do jogo base corresponde à pesquisa.';

  @override
  String get managedBaseGameBrowserLoadErrorTitle =>
      'Provas do jogo base indisponíveis';

  @override
  String get managedBaseGameBrowserLoadErrorDescription =>
      'Não foi possível ler o catálogo exato suportado. Nenhum ficheiro de projeto, jogo ou gravação foi alterado.';

  @override
  String get managedBaseGameBrowserOfflineDraftBadge =>
      'Rascunho offline suportado';

  @override
  String get managedBaseGameBrowserInspectOnlyBadge => 'Apenas inspeção';

  @override
  String get managedBaseGameBrowserCreateNpcDraft => 'Usar como início de NPC';

  @override
  String get managedBaseGameBrowserCreateQuestDraft =>
      'Usar como início de missão';

  @override
  String get managedBaseGameBrowserSpawnClass => 'Definição de geração';

  @override
  String get managedBaseGameBrowserActorBlueprint => 'Blueprint de ator';

  @override
  String get managedBaseGameBrowserExperimentalResultsCapped =>
      'São mostrados os primeiros 100 resultados apenas para inspeção. Refine a pesquisa para obter resultados mais específicos.';

  @override
  String get managedInstalledBrowserLoading =>
      'A ler o inventário exato de pacotes instalados…';

  @override
  String managedInstalledBrowserCompleteSummary(int count) {
    return '$count pacotes instalados candidatos';
  }

  @override
  String managedInstalledBrowserPartialSummary(int count) {
    return '$count pacotes instalados candidatos — resultado parcial';
  }

  @override
  String get managedInstalledBrowserCompleteDescription =>
      'Os metadados da pasta foram lidos e o instantâneo instalado permaneceu exato.';

  @override
  String get managedInstalledBrowserPartialDescription =>
      'Alguns metadados de pacotes estavam em falta ou não eram canónicos; os resultados ajudam na descoberta, mas não estão completos.';

  @override
  String get managedInstalledBrowserAuthorityNotice =>
      'Este âmbito mostra apenas metadados de pacotes DataAsset instalados. Inspecionar ou copiar um caminho não concede autoridade de compilação, distribuição, execução ou escrita no jogo.';

  @override
  String get managedInstalledBrowserRefresh => 'Ler um novo instantâneo exato';

  @override
  String get managedInstalledBrowserSearchLabel =>
      'Pesquisar DataAssets instalados';

  @override
  String get managedInstalledBrowserSearchHint =>
      'Nome do recurso ou caminho /Game';

  @override
  String get managedInstalledBrowserSearchPrompt =>
      'Introduza um nome de recurso ou caminho /Game para pesquisar.';

  @override
  String get managedInstalledBrowserNoMatchesTitle =>
      'Nenhum DataAsset instalado correspondente';

  @override
  String get managedInstalledBrowserNoMatchesDescription =>
      'Experimente outro nome de recurso ou um caminho /Game mais abrangente.';

  @override
  String get managedInstalledBrowserResultLimitDescription =>
      'São mostrados os primeiros 100 resultados. Refine a pesquisa para restringir o instantâneo exato.';

  @override
  String get managedInstalledBrowserKindBadge => 'Pacote DataAsset';

  @override
  String get managedInstalledBrowserMetadataOnlyBadge => 'Apenas metadados';

  @override
  String get managedInstalledBrowserOpenInspector => 'Inspecionar pacote exato';

  @override
  String get managedInstalledBrowserErrorTitle =>
      'Inventário de pacotes instalados indisponível';

  @override
  String get managedInstalledBrowserErrorDescription =>
      'Não foi possível ler o instantâneo instalado exato. Nenhum ficheiro de projeto, jogo ou gravação foi alterado.';

  @override
  String get managedGlobalSearchScopeLabel => 'Pesquisar tudo';

  @override
  String get managedGlobalSearchTitle => 'Pesquisar todo o conteúdo';

  @override
  String get managedGlobalSearchLabel =>
      'NPC, missão, fala, recurso, ID ou caminho /Game';

  @override
  String get managedGlobalSearchAction => 'Pesquisar';

  @override
  String get managedGlobalSearchClear => 'Limpar';

  @override
  String get managedGlobalSearchPrompt =>
      'Introduza uma pesquisa para consultar as três fontes de forma independente.';

  @override
  String get managedGlobalSearchNoResults =>
      'Sem correspondências nesta fonte.';

  @override
  String get managedGlobalSearchLoading => 'A ler a fonte exata…';

  @override
  String get managedGlobalSearchFailed => 'Não foi possível ler esta fonte.';

  @override
  String get managedGlobalSearchComplete => 'Completo';

  @override
  String get managedGlobalSearchPartial => 'Parcial';

  @override
  String get managedGlobalSearchTruncated =>
      'A mostrar as primeiras 100 correspondências. Refine a pesquisa.';

  @override
  String get managedGlobalSearchOpen => 'Abrir';

  @override
  String get managedGlobalSearchCreateDraft => 'Criar rascunho';

  @override
  String get managedGlobalSearchInspect => 'Inspecionar';

  @override
  String get managedGlobalSearchKindModEntity => 'Conteúdo do mod';

  @override
  String get managedGlobalSearchKindModAsset => 'Recurso do mod';

  @override
  String get managedGlobalSearchKindBaseNpc => 'Ponto de partida de NPC';

  @override
  String get managedGlobalSearchKindBaseQuest => 'Ponto de partida de missão';

  @override
  String get managedGlobalSearchKindExperimentalNpc => 'Evidência de NPC';

  @override
  String get managedGlobalSearchReadinessExact => 'Projeto atual exato';

  @override
  String get managedGlobalSearchReadinessProblems => 'Exato, com problemas';

  @override
  String get managedGlobalSearchResultStale =>
      'Este resultado já não está no projeto atual. Pesquise novamente.';

  @override
  String get managedStoryWorkbenchDraftBadge => 'Apenas rascunho';

  @override
  String get managedStoryWorkbenchBuildBlockedBadge => 'Compilação bloqueada';

  @override
  String get managedStoryWorkbenchRuntimeUnqualifiedBadge =>
      'Execução não verificada';

  @override
  String get managedStoryWorkbenchOverviewTab => 'Visão geral';

  @override
  String get managedStoryWorkbenchProfileTab => 'Perfil';

  @override
  String get managedStoryWorkbenchStoryTab => 'História';

  @override
  String get managedStoryWorkbenchLogicTab => 'Lógica';

  @override
  String get managedStoryWorkbenchRoutineTab => 'Rotina';

  @override
  String get managedStoryWorkbenchInventoryTab => 'Inventário';

  @override
  String get managedStoryWorkbenchDialogVoiceTab => 'Diálogo e voz';

  @override
  String get managedStoryWorkbenchReferencesTab => 'Referências';

  @override
  String get managedStoryWorkbenchProblemsChecksTab =>
      'Problemas e verificações';

  @override
  String get managedStoryWorkbenchEditOverview => 'Editar nome e objetivos';

  @override
  String get managedStoryWorkbenchEditStory => 'Editar descrição e ligações';

  @override
  String get managedStoryWorkbenchEditLogic => 'Editar estados e transições';

  @override
  String get managedStoryWorkbenchInspectQuest =>
      'Abrir código-fonte e verificações do compilador';

  @override
  String get managedStoryWorkbenchInspectNpc =>
      'Abrir perfil e verificações do compilador';

  @override
  String get managedStoryWorkbenchMoreActions => 'More actions';

  @override
  String get managedStoryWorkbenchRemoveDraft => 'Remove draft…';

  @override
  String get managedStoryWorkbenchRemovingDraft => 'Removing draft…';

  @override
  String get managedStoryWorkbenchReviewRemovalBlockers =>
      'Review removal blockers';

  @override
  String get managedStoryWorkbenchCapabilityUnavailable => 'Ainda não modelado';

  @override
  String get managedStoryWorkbenchNpcStoryUnavailable =>
      'As relações com missões e história ainda não estão modeladas para rascunhos de NPC.';

  @override
  String get managedStoryWorkbenchNpcRoutineUnavailable =>
      'A rotina e a colocação no mundo ainda não estão modeladas.';

  @override
  String get managedStoryWorkbenchNpcInventoryUnavailable =>
      'O inventário, o equipamento e o comércio ainda não estão modelados.';

  @override
  String get managedStoryWorkbenchNpcDialogVoiceUnavailable =>
      'As relações de diálogo, localização e voz ainda não estão modeladas para rascunhos de NPC.';

  @override
  String get managedStoryWorkbenchQuestDialogVoiceUnavailable =>
      'As relações de diálogo, localização e voz ainda não estão modeladas para rascunhos de missão.';

  @override
  String get managedStoryWorkbenchNoReferenceProblems =>
      'Não existem referências de projeto por resolver';

  @override
  String managedStoryWorkbenchReferenceProblemCount(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count referências de projeto por resolver',
      one: '1 referência de projeto por resolver',
    );
    return '$_temp0';
  }

  @override
  String get managedStoryWorkbenchReferenceScopeNotice =>
      'Apenas indica o estado das referências; não confirma a prontidão para compilação ou execução.';

  @override
  String get managedStoryWorkbenchTechnicalDetails => 'Detalhes técnicos';

  @override
  String get managedStoryWorkbenchQuestKindLabel => 'Rascunho de missão';

  @override
  String get managedStoryWorkbenchNpcKindLabel => 'Rascunho de NPC';

  @override
  String get managedStoryWorkbenchQuestTitleLabel => 'Título da missão';

  @override
  String get managedStoryWorkbenchTechnicalIdLabel => 'ID técnico';

  @override
  String get managedStoryWorkbenchObjectivesLabel => 'Objetivos';

  @override
  String get managedStoryWorkbenchUniqueNameLabel => 'Nome único';

  @override
  String get managedStoryWorkbenchModuleNamespaceLabel =>
      'Espaço de nomes do módulo';

  @override
  String get managedStoryWorkbenchQuestGiverLabel =>
      'Personagem que atribui a missão';

  @override
  String get managedStoryWorkbenchRuntimeParentLabel =>
      'Classe-base em tempo de execução';

  @override
  String get managedStoryWorkbenchLogicDescription =>
      'Os estados do ciclo de vida da missão, os acionadores, as condições e os efeitos são editados como uma única operação atómica sobre o estado atual exato.';

  @override
  String get managedStoryWorkbenchOutgoingHeading => 'Saída';

  @override
  String get managedStoryWorkbenchNoOutgoingReferences =>
      'Sem referências projetadas';

  @override
  String get managedStoryWorkbenchIncomingHeading => 'Entrada';

  @override
  String get managedStoryWorkbenchNoIncomingReferences =>
      'Sem referências de projeto recebidas';

  @override
  String get managedStoryWorkbenchSemanticIdentityLabel =>
      'Identidade semântica';

  @override
  String get managedStoryWorkbenchOriginLabel => 'Origem';

  @override
  String get managedStoryWorkbenchEntityRevisionLabel => 'Revisão da entidade';

  @override
  String get managedStoryWorkbenchStableIdLabel => 'ID estável';

  @override
  String get managedStoryWorkbenchReferenceResolvedLabel =>
      'Referência resolvida';

  @override
  String get managedStoryWorkbenchReferenceUnresolvedLabel =>
      'Referência por resolver';

  @override
  String get managedProblemsTitle => 'Problems & readiness';

  @override
  String get managedProblemsDescription =>
      'See what needs attention and open the exact affected project content.';

  @override
  String get managedProblemsScopeNotice =>
      'Every status covers only its named scope. A clear reference check does not mean the mod can be built or tested in-game.';

  @override
  String get managedProblemsRefresh => 'Refresh problems';

  @override
  String get managedProblemsPartialTitle => 'Some checks are unavailable';

  @override
  String get managedProblemsDataAssetsUnavailable =>
      'DataAsset edits could not be checked. Other exact project findings are still shown.';

  @override
  String get managedProblemsOverviewHeading => 'Readiness by area';

  @override
  String get managedProblemsSearchLabel => 'Search problems';

  @override
  String get managedProblemsClearSearch => 'Clear problem search';

  @override
  String get managedProblemsListHeading => 'Problems';

  @override
  String get managedProblemsEmptyTitle =>
      'No modeled structural problems found';

  @override
  String get managedProblemsEmptyDescription =>
      'The exact checks currently modeled by Mod Studio found nothing to repair.';

  @override
  String get managedProblemsEmptyBoundary =>
      'Compiler evidence was not evaluated, the full managed build is unavailable, and runtime behavior remains unqualified.';

  @override
  String get managedProblemsFilteredEmptyTitle => 'No matching problems';

  @override
  String get managedProblemsFilteredEmptyDescription =>
      'Change the search or category filter to see other findings.';

  @override
  String get managedProblemsSelectTitle => 'Select a problem';

  @override
  String get managedProblemsSelectDescription =>
      'Choose a finding to see what it means and the safest available next action.';

  @override
  String get managedProblemsDetailHeading => 'Problem details';

  @override
  String get managedProblemsCloseDetail => 'Close problem details';

  @override
  String get managedProblemsCategoryLabel => 'Area';

  @override
  String get managedProblemsSeverityLabel => 'Attention';

  @override
  String get managedProblemsSourceLabel => 'Evidence';

  @override
  String get managedProblemsOpenSourceEntity => 'Open source content';

  @override
  String get managedProblemsOpenReferencedAsset => 'Open referenced asset';

  @override
  String get managedProblemsOpenDataAssetEdits => 'Open DataAsset edits';

  @override
  String get managedProblemsActionFailed =>
      'The exact target could not be opened. Refresh the project problems and try again.';

  @override
  String get managedProblemsActionProgress =>
      'Opening the exact project target';

  @override
  String get managedProblemsCategoryReferences => 'References';

  @override
  String get managedProblemsCategorySetup => 'Setup';

  @override
  String get managedProblemsCategoryDataAssets => 'DataAssets';

  @override
  String get managedProblemsSeverityInformation => 'Information';

  @override
  String get managedProblemsSeverityWarning => 'Needs attention';

  @override
  String get managedProblemsSeverityBlocking => 'Blocks this scope';

  @override
  String get managedProblemsScopeReferencesTitle => 'Reference integrity';

  @override
  String get managedProblemsScopeReferencesDescription =>
      'Checks exact links between current project content and assets.';

  @override
  String get managedProblemsScopeDataAssetsTitle => 'DataAsset edit registry';

  @override
  String get managedProblemsScopeDataAssetsDescription =>
      'Checks whether the exact current list of saved DataAsset edits could be read.';

  @override
  String get managedProblemsScopeGameTitle => 'Game setup';

  @override
  String get managedProblemsScopeGameDescription =>
      'Shows whether a game installation is configured for bounded read-only tools.';

  @override
  String get managedProblemsScopeCompilerTitle => 'Source & compiler evidence';

  @override
  String get managedProblemsScopeCompilerDescription =>
      'Compiler checks run only when you explicitly open and start them for one exact entity.';

  @override
  String get managedProblemsScopeBuildTitle => 'Managed project build';

  @override
  String get managedProblemsScopeBuildDescription =>
      'A complete build path for managed NPC, Quest, dialog, and DataAsset edits is not available yet.';

  @override
  String get managedProblemsScopeRuntimeTitle => 'In-game behavior';

  @override
  String get managedProblemsScopeRuntimeDescription =>
      'No general runtime, save, deployment, or cleanup qualification is claimed.';

  @override
  String get managedProblemsReadinessClear => 'Checked within this scope';

  @override
  String get managedProblemsReadinessIssues => 'Needs attention';

  @override
  String get managedProblemsReadinessUnavailable => 'Check unavailable';

  @override
  String get managedProblemsReadinessNotEvaluated => 'Not evaluated';

  @override
  String get managedProblemsReadinessBlocked => 'Build path unavailable';

  @override
  String get managedProblemsReadinessUnqualified => 'Runtime unqualified';

  @override
  String get managedProblemsEvidenceContent => 'Exact current project content';

  @override
  String get managedProblemsEvidenceDataAssets =>
      'Exact current DataAsset registry';

  @override
  String get managedProblemsEvidenceConfiguration =>
      'Current app configuration';

  @override
  String get managedProblemsEvidenceUnavailable =>
      'Evidence source unavailable';

  @override
  String get managedProblemsEvidenceBoundary => 'Known capability boundary';

  @override
  String get managedProblemsForeignReferenceTitle =>
      'Reference points to another project';

  @override
  String get managedProblemsMissingEntityTitle =>
      'Linked project content is missing';

  @override
  String get managedProblemsEntityKindTitle =>
      'Linked project content has the wrong type';

  @override
  String get managedProblemsMissingAssetTitle =>
      'Linked project file is missing';

  @override
  String get managedProblemsAssetLengthTitle =>
      'Linked project file has an unexpected size';

  @override
  String get managedProblemsAssetTypeTitle =>
      'Linked project file has an unexpected type';

  @override
  String get managedProblemsGameSetupTitle =>
      'Game installation is not configured';

  @override
  String get managedProblemsDataAssetRegistryTitle =>
      'DataAsset edits could not be checked';

  @override
  String get managedProblemsDataAssetOfflineTitle =>
      'DataAsset edit is draft-only';

  @override
  String managedProblemsEntityReferenceDescription(String source) {
    return 'Open $source and repair this exact project-content link.';
  }

  @override
  String managedProblemsAssetReferenceDescription(String source) {
    return 'Open $source and repair this exact project-file link.';
  }

  @override
  String get managedProblemsDataAssetRegistryDescription =>
      'Refresh the exact current project. No conclusion is drawn about saved DataAsset edits until this source is available.';

  @override
  String managedProblemsDataAssetOfflineDescription(String targetPath) {
    return 'The saved edit for $targetPath can be reviewed in DataAsset edits, but it cannot be emitted by a managed project build or claimed as working in-game yet.';
  }

  @override
  String get projectExportActionTitle => 'Export project copy…';

  @override
  String get projectExportActionDescription =>
      'Write an exact portable copy of the current saved project checkpoint.';

  @override
  String get projectExportActionDirtyBlocked =>
      'Save or discard the open localization edits before exporting a project copy.';

  @override
  String get projectExportDialogTitle => 'Export project copy';

  @override
  String get projectExportPortableCopyTitle => 'Portable project copy';

  @override
  String get projectExportPortableCopyDescription =>
      'This writes the exact current saved project checkpoint to a new .goremod file. The open project stays current and unchanged.';

  @override
  String get projectExportCapabilityBoundary =>
      'This copy is not a playable mod, build, deployment, or restorable backup. It does not read or change the game or any save.';

  @override
  String get projectExportKeepOriginal =>
      'Importing this managed copy is not available yet. Keep the original project folder.';

  @override
  String get projectExportFileNameLabel => 'New project-copy file';

  @override
  String get projectExportFileNameHelper =>
      'Use a new portable file name ending in .goremod.';

  @override
  String get projectExportChooseDestination => 'Choose destination folder';

  @override
  String get projectExportNoDestination => 'No destination folder selected';

  @override
  String get projectExportNewFile => 'New file';

  @override
  String get projectExportCancel => 'Cancel';

  @override
  String get projectExportClose => 'Close';

  @override
  String get projectExportSubmit => 'Export copy';

  @override
  String get projectExportExporting => 'Exporting…';

  @override
  String get projectExportParentRequired =>
      'Choose an existing destination folder.';

  @override
  String get projectExportParentAbsolute =>
      'Choose an absolute existing destination folder.';

  @override
  String get projectExportParentLink =>
      'The selected destination is a link. Choose a real existing folder.';

  @override
  String get projectExportParentInspectFailed =>
      'The destination folder could not be inspected safely. Nothing was created.';

  @override
  String get projectExportFileNameRequired =>
      'Enter a new project-copy file name.';

  @override
  String get projectExportFileNameTooLong =>
      'The file name must be at most 128 ASCII characters.';

  @override
  String get projectExportFileNameInvalid =>
      'Start with a letter or digit, use only ASCII letters, digits, dots, underscores, or hyphens, and end with .goremod.';

  @override
  String get projectExportFileNameReserved =>
      'That file name is reserved by Windows.';

  @override
  String get projectExportOutputExists =>
      'That file already exists. Choose a new file name; existing files are never overwritten.';

  @override
  String get projectExportOutputLink =>
      'The new file path is a link. Choose a different file name.';

  @override
  String get projectExportOutputRejected =>
      'The destination was rejected before the new local file was created. Nothing was created. Choose a different file name or destination folder.';

  @override
  String get projectExportStale =>
      'The project changed before export started. No output was created. Close this window and open Export project copy again.';

  @override
  String get projectExportRequiresReopen =>
      'This project can no longer be verified as current. No output was created. Close this window and recover or reopen the project.';

  @override
  String get projectExportUnsupported =>
      'This managed project session cannot export exact portable copies. Nothing was created.';

  @override
  String get projectExportFailedBeforeStart =>
      'The project copy could not be prepared exactly. Nothing was created.';

  @override
  String get projectExportPrepublicationFailed =>
      'Export stopped safely before the new local file was created. Nothing was created. Close this window and check the project and destination before trying again.';

  @override
  String projectExportMayExist(String output) {
    return 'The export did not return a verified receipt. Do not retry. Close this window and check the destination: $output';
  }

  @override
  String projectExportResultMismatch(String output) {
    return 'The completed export does not match this checkpoint or destination. Do not retry; inspect the destination: $output';
  }

  @override
  String get projectExportPublished =>
      'The exact portable project copy was created as a new local file.';

  @override
  String get projectExportPublishedCleanupWarning =>
      'The exact project copy was created as a local file, but internal temporary-file cleanup was incomplete. The created file is valid; do not retry.';

  @override
  String projectExportPublicationUncertain(String output) {
    return 'The local file may have been created. Do not retry. Check whether this destination exists: $output';
  }

  @override
  String get projectExportArchiveBytes => 'Archive bytes';

  @override
  String get projectExportArchiveSha256 => 'Archive SHA-256';

  @override
  String get projectExportCurrentProjectUnchanged =>
      'The current project remains open and unchanged. The game and saves were not touched.';
}

/// The translations for Portuguese, as used in Brazil (`pt_BR`).
class AppLocalizationsPtBr extends AppLocalizationsPt {
  AppLocalizationsPtBr() : super('pt_BR');

  @override
  String get tabDialogs => 'Diálogos';

  @override
  String get tabAudio => 'Áudio';

  @override
  String get tabTextures => 'Texturas';

  @override
  String get tabScripts => 'Scripts';

  @override
  String get changesAll => 'Todos';

  @override
  String get sectionItemValues => 'Valores dos itens';

  @override
  String get sectionLocalizedText => 'Textos localizados';

  @override
  String get audioCatCreatures => 'Criaturas';

  @override
  String get audioCatObjects => 'Objetos';

  @override
  String get audioCatMagic => 'Magia';

  @override
  String get audioCatMovement => 'Movimento';

  @override
  String get audioCatWorld => 'Mundo';

  @override
  String get audioCatAction => 'Ações';

  @override
  String get audioCatCombat => 'Combate';

  @override
  String get audioCatPhysics => 'Física';

  @override
  String get audioCatItems => 'Itens';

  @override
  String get audioCatUi => 'Interface';

  @override
  String get audioCatFoley => 'Foley';

  @override
  String get audioCatUnderwater => 'Subaquático';

  @override
  String get audioCatVision => 'Visões';

  @override
  String get audioCatDialog => 'Diálogo';

  @override
  String get audioCatOther => 'Outros';

  @override
  String get extractLocalizedText => 'Extrair textos localizados';

  @override
  String get lightMode => 'Modo claro';

  @override
  String get darkMode => 'Modo escuro';

  @override
  String get language => 'Idioma';

  @override
  String get exportMod => 'Exportar mod';

  @override
  String exportModWithCount(int count) {
    return 'Exportar mod ($count)';
  }

  @override
  String get selectAnItemToEdit => 'Selecione um item para editar seus campos.';

  @override
  String gameDataActiveTooltip(String name) {
    return 'Dados do jogo: $name';
  }

  @override
  String get gameDataBundledTooltip => 'Dados do jogo: incluídos';

  @override
  String get loadGameDataDump => 'Carregar dump de dados do jogo…';

  @override
  String get loadGameDataDumpSubtitle => 'gore_game_data.json do mod gore-dump';

  @override
  String get useBundledData => 'Usar dados incluídos';

  @override
  String get alreadyBundled => 'já incluídos';

  @override
  String get gameDataFileGroupLabel => 'dados do jogo';

  @override
  String get minimize => 'Minimizar';

  @override
  String get restore => 'Restaurar';

  @override
  String get maximize => 'Maximizar';

  @override
  String get close => 'Fechar';

  @override
  String get about => 'Sobre';

  @override
  String aboutVersion(String version, String sha) {
    return 'Version $version ($sha)';
  }

  @override
  String get aboutCopyright => '© 2026 colaboradores do GORE';

  @override
  String get aboutLicense => 'Licenciado sob a Licença MIT.';

  @override
  String get categoryMeleeWeapons => 'Armas corpo a corpo';

  @override
  String get categoryRangedWeapons => 'Armas à distância';

  @override
  String get categoryAmmunition => 'Munição';

  @override
  String get categoryRunes => 'Runas';

  @override
  String get categorySpellScrolls => 'Pergaminhos de magia';

  @override
  String get categoryFoodAndPotions => 'Comida e poções';

  @override
  String get categoryMiscellaneous => 'Diversos';

  @override
  String get categoryAmulets => 'Amuletos';

  @override
  String get categoryRings => 'Anéis';

  @override
  String get categoryAnimalTrophies => 'Troféus de animais';

  @override
  String get categoryWritings => 'Escritos';

  @override
  String get categoryMissionItems => 'Itens de missão';

  @override
  String get categoryKeys => 'Chaves';

  @override
  String get categoryOther => 'Outros';

  @override
  String categoryWithCount(String label, int count) {
    return '$label ($count)';
  }

  @override
  String get searchItems => 'Pesquisar itens';

  @override
  String get noItemsMatch => 'Nenhum item corresponde';

  @override
  String failedToLoadCatalog(String error) {
    return 'Falha ao carregar o catálogo: $error';
  }

  @override
  String pendingOverridesWithCount(int count) {
    return 'Alterações pendentes ($count)';
  }

  @override
  String get clearAll => 'Limpar tudo';

  @override
  String get noPendingOverrides =>
      'Nenhuma alteração pendente.\nEdite os campos dos itens para adicionar algumas.';

  @override
  String get removeOverride => 'Remover alteração';

  @override
  String get searchChanges => 'Pesquisar alterações';

  @override
  String get noChangesMatch => 'Nenhuma alteração corresponde';

  @override
  String get clearSection => 'Limpar este grupo';

  @override
  String get modName => 'Nome do mod';

  @override
  String get loadDelayLabel => 'Atraso de carregamento (ms, 0 = imediato)';

  @override
  String get noFolderSelected => 'Nenhuma pasta selecionada';

  @override
  String get chooseFolder => 'Escolher pasta';

  @override
  String get packageAsZip => 'Empacotar como .zip';

  @override
  String get cancel => 'Cancelar';

  @override
  String get export => 'Exportar';

  @override
  String get exportHere => 'Exportar aqui';

  @override
  String get mustBeNonNegativeInteger => 'Deve ser um inteiro não negativo';

  @override
  String get extractingLocalizedText => 'Extraindo textos localizados do jogo…';

  @override
  String get localizedTextExtractionCancelled =>
      'Extração de textos localizados cancelada.';

  @override
  String get localizedTextExtracted => 'Textos localizados extraídos.';

  @override
  String get extractionFailed => 'Falha na extração.';

  @override
  String get localizationCacheFileGroupLabel => 'cache de localização';

  @override
  String get extractLocalizedTextQuestion =>
      'Extrair os textos localizados do jogo?';

  @override
  String get extractLocalizedTextBody =>
      'Os textos localizados do jogo ainda não foram extraídos. Extraí-los agora da sua instalação do jogo? (opcional)';

  @override
  String get notNow => 'Agora não';

  @override
  String get extract => 'Extrair';

  @override
  String get validationRequired => 'Obrigatório';

  @override
  String get validationMustBeWholeNumber => 'Deve ser um número inteiro';

  @override
  String get validationMustBeNumber => 'Deve ser um número';

  @override
  String get validationMustBeFinite => 'Deve ser um número finito';

  @override
  String validationMustBeAtLeast(String min) {
    return 'Deve ser ≥ $min';
  }

  @override
  String validationMustBeAtMost(String max) {
    return 'Deve ser ≤ $max';
  }

  @override
  String get validationMustBeBool => 'Deve ser true ou false';

  @override
  String validationMustBeOneOf(String options) {
    return 'Deve ser um de: $options';
  }

  @override
  String get modNameRequired => 'Obrigatório';

  @override
  String get modNameControlCharacters =>
      'Não deve conter caracteres de controle';

  @override
  String get modNamePathSeparators => 'Não deve conter separadores de caminho';

  @override
  String get modNameNotAFolderName => 'Nome de pasta inválido';

  @override
  String localizedTextExtractedCount(int idCount, int languageCount) {
    return '$idCount IDs extraídos em $languageCount idiomas';
  }

  @override
  String get managerDeployActive =>
      'Um loadout do mod-manager está ativo. Primeiro faça o undeploy no gore-manager.';

  @override
  String get projectTransitionCleanupWarning =>
      'O novo projeto está aberto, mas não foi possível limpar completamente a sessão do projeto anterior. A limpeza não será repetida. Reinicie o Mod Studio antes de reabrir o projeto anterior.';

  @override
  String get projectNewManagedRevision3 => 'Novo projeto de mod gerenciado…';

  @override
  String get projectNewLegacy => 'Novo projeto legado';

  @override
  String get projectCreateGamePathRequired =>
      'Defina o caminho do Gothic 1 Remake nas Configurações antes de criar um projeto de mod.';

  @override
  String get projectCreateDirectoryPickerTitle =>
      'Criar aqui o projeto de mod gerenciado';

  @override
  String projectManagedRevision3Created(String projectId) {
    return 'Projeto de mod gerenciado $projectId criado';
  }

  @override
  String projectManagedRevision3CreateFailed(String error) {
    return 'Falha ao criar o projeto de mod gerenciado: $error';
  }

  @override
  String get projectCreateDialogTitle => 'Criar um projeto de mod';

  @override
  String get projectCreateNameLabel => 'Nome do projeto';

  @override
  String get projectCreateNameHelper => 'O nome exibido no Mod Studio.';

  @override
  String get projectCreateVersionLabel => 'Versão';

  @override
  String get projectCreateVersionHelper => 'Uma versão inicial, como 0.1.0.';

  @override
  String get projectCreateAuthorLabel => 'Autor';

  @override
  String get projectCreateAuthorHelper =>
      'Seu nome ou o nome da equipe de modding.';

  @override
  String get projectCreateLocalesLabel => 'Idiomas de edição';

  @override
  String get projectCreateLocalesHelper =>
      'Tags canônicas separadas por vírgulas, por exemplo: en, de, en-US.';

  @override
  String get projectCreateBoundary =>
      'Isso cria um projeto offline gerenciado e vazio. Não compila, implanta nem executa um mod e não altera arquivos do jogo ou saves.';

  @override
  String get projectCreateSubmit => 'Criar projeto';

  @override
  String projectCreateMetadataRequired(String label) {
    return '$label é obrigatório.';
  }

  @override
  String projectCreateMetadataNoOuterWhitespace(String label) {
    return '$label não pode começar nem terminar com espaços.';
  }

  @override
  String projectCreateMetadataControlCharacters(String label) {
    return '$label não pode conter caracteres de controle.';
  }

  @override
  String projectCreateMetadataMalformed(String label) {
    return '$label contém texto inválido.';
  }

  @override
  String projectCreateMetadataTooLong(String label, int maxBytes) {
    return '$label excede o limite UTF-8 de $maxBytes bytes.';
  }

  @override
  String get projectCreateLocalesRequired =>
      'Insira pelo menos um idioma de edição.';

  @override
  String get projectCreateLocalesEmptyEntry =>
      'Remova a entrada de idioma vazia.';

  @override
  String projectCreateLocalesTooMany(int maxLocales) {
    return 'Use no máximo $maxLocales idiomas de edição.';
  }

  @override
  String projectCreateLocaleBoundedAscii(String locale) {
    return 'A localidade “$locale” deve ser ASCII e ter tamanho limitado.';
  }

  @override
  String projectCreateLocaleLanguage(String locale) {
    return 'A localidade “$locale” precisa de um idioma em minúsculas com 2 a 8 letras.';
  }

  @override
  String projectCreateLocaleInvalidSegment(String locale) {
    return 'A localidade “$locale” contém um segmento inválido.';
  }

  @override
  String projectCreateLocaleNotCanonical(String locale, String canonical) {
    return 'A localidade “$locale” não é canônica; use “$canonical”.';
  }

  @override
  String get managedWorkspaceOverviewLabel => 'Visão geral';

  @override
  String get managedWorkspaceContentLabel => 'Conteúdo';

  @override
  String get managedWorkspaceDataAssetsLabel => 'DataAssets';

  @override
  String get managedContentWorkspaceLibraryLabel => 'Este mod';

  @override
  String get managedWorkspaceHomeLabel => 'Início';

  @override
  String get managedWorkspaceStoryLabel => 'História';

  @override
  String get managedWorkspaceWorldLabel => 'Mundo';

  @override
  String get managedWorkspaceLocalizationVoiceLabel => 'Localização e vozes';

  @override
  String get managedWorkspaceValidateTestLabel => 'Validar e testar';

  @override
  String get managedWorkspaceBuildReleaseLabel => 'Compilar e publicar';

  @override
  String get managedWorkspaceSettingsExpertLabel =>
      'Configurações e modo especialista';

  @override
  String get managedSectionStoryDescription => 'NPCs, missões e diálogos.';

  @override
  String get managedSectionWorldDescription =>
      'O posicionamento no mundo e os fluxos relacionados estão planejados.';

  @override
  String get managedSectionLocalizationVoiceDescription =>
      'Escreva e traduza os diálogos do projeto em um só lugar e continue depois com as vozes.';

  @override
  String get managedSectionValidateTestDescription =>
      'Verifica a integridade exata do projeto e os checkpoints; não afirma um teste em execução.';

  @override
  String get managedSectionBuildReleaseDescription =>
      'Os pacotes de vozes estão disponíveis; builds jogáveis completos e implantação não estão.';

  @override
  String get managedSectionSettingsExpertDescription =>
      'As configurações estão disponíveis; as ferramentas especializadas ainda não estão integradas.';

  @override
  String get managedSectionStatusHeading => 'Status';

  @override
  String get managedSectionActionsHeading => 'Ações';

  @override
  String get managedCapabilityAvailable => 'Disponível';

  @override
  String get managedCapabilityPartial => 'Parcial';

  @override
  String get managedCapabilityPlanned => 'Planejado';

  @override
  String get managedCapabilityUnavailable => 'Indisponível';

  @override
  String get managedProjectSubtitle =>
      'Espaço de criação offline correspondente exatamente à versão atual';

  @override
  String get managedProjectLandingTitle =>
      'Espaço de trabalho do projeto gerenciado';

  @override
  String get managedProjectLandingDescription =>
      'Use o novo fluxo de Início, Conteúdo, História, Voz, validação e lançamento em um único projeto gerenciado.';

  @override
  String get legacyCompatibilityToolsTitle =>
      'Ferramentas de compatibilidade legadas';

  @override
  String get legacyCompatibilityToolsDescription =>
      'As abas abaixo contêm ferramentas antigas de substituição direta. Elas continuarão disponíveis enquanto o espaço de trabalho do projeto gerenciado evolui.';

  @override
  String get managedProjectTechnicalDetails => 'Detalhes técnicos do projeto';

  @override
  String get managedProjectRecoveryContentLocked =>
      'Reabra o projeto gerenciado antes de ler seu conteúdo.';

  @override
  String get managedDashboardUntitledProject => 'Projeto sem título';

  @override
  String get managedDashboardDraftStatus => 'Rascunho';

  @override
  String get managedDashboardProjectVersion => 'Versão';

  @override
  String get managedDashboardProjectAuthor => 'Autor';

  @override
  String get managedDashboardNotProvided => 'Não informado';

  @override
  String get managedDashboardContentCounts => 'Conteúdo do projeto';

  @override
  String get managedDashboardNpcDrafts => 'Rascunhos de NPC';

  @override
  String get managedDashboardQuestDrafts => 'Rascunhos de missões';

  @override
  String get managedDashboardDialogLines => 'Linhas de diálogo';

  @override
  String get managedDashboardVoiceTakes => 'Gravações de voz';

  @override
  String get managedDashboardAssets => 'Recursos';

  @override
  String get managedDashboardUnresolvedReferences =>
      'Referências não resolvidas';

  @override
  String get managedDashboardReadiness => 'O que funciona agora';

  @override
  String get managedDashboardOfflineAuthoringTitle =>
      'Criação offline disponível';

  @override
  String get managedDashboardOfflineAuthoringDescription =>
      'Crie e edite conteúdos de projeto compatíveis sem alterar a instalação do jogo nem os arquivos salvos.';

  @override
  String get managedDashboardGeneralBuildBlockedTitle =>
      'Compilação geral do mod indisponível';

  @override
  String get managedDashboardGeneralBuildBlockedDescription =>
      'Somente pacotes Voice offline selados podem ser compilados; ainda não é possível compilar um mod completo e jogável.';

  @override
  String get managedDashboardRuntimeUnqualifiedTitle =>
      'Execução ainda não verificada';

  @override
  String get managedDashboardRuntimeUnqualifiedDescription =>
      'O Mod Studio ainda não comprovou este conteúdo do projeto dentro do jogo em execução.';

  @override
  String get managedDashboardReferenceIntegrityTitle =>
      'Integridade das referências';

  @override
  String get managedDashboardReferenceIntegrityDescription =>
      'Esta contagem verifica apenas as referências do projeto; ela não indica que o projeto está pronto para compilação ou execução.';

  @override
  String get managedDashboardMissingGameTitle =>
      'Configuração do jogo necessária';

  @override
  String get managedDashboardMissingGameDescription =>
      'Configure a instalação do Gothic 1 Remake em Configurações antes de usar ações que precisem de dados verificados do jogo instalado.';

  @override
  String get managedDashboardCreateHeading => 'Criar';

  @override
  String get managedDashboardToolsHeading => 'Ferramentas do projeto';

  @override
  String get managedDashboardLoading => 'Carregando a visão geral do projeto';

  @override
  String get managedDashboardLoadError => 'Visão geral do projeto indisponível';

  @override
  String get managedDashboardLoadErrorDescription =>
      'Não foi possível carregar a visão geral verificada do projeto. O conteúdo do projeto não foi alterado.';

  @override
  String get managedDashboardRetry => 'Tentar novamente';

  @override
  String get managedActionNewNpcTitle => 'Novo NPC';

  @override
  String get managedActionNewNpcDescription =>
      'Crie um rascunho de NPC offline e limitado a partir de dados verificados do jogo instalado.';

  @override
  String get managedActionNewQuestTitle => 'Nova missão';

  @override
  String get managedActionNewQuestDescription =>
      'Crie um rascunho de missão offline com objetivos e identidades principais verificadas.';

  @override
  String get managedActionNewDialogLineTitle => 'Adicionar linha de diálogo';

  @override
  String get managedActionNewDialogLineDescription =>
      'Escreva um texto localizado do projeto ou vincule um texto ainda não usado deste projeto. Isso não cria um tópico de diálogo jogável.';

  @override
  String managedActionNewDialogLineSaved(int projectRevision) {
    return 'Linha de diálogo salva na revisão $projectRevision do projeto. O jogo e os arquivos salvos não foram alterados.';
  }

  @override
  String get managedDialogLineIntroduction =>
      'Escreva uma nova linha de diálogo localizada ou vincule um texto que já pertence a este projeto.';

  @override
  String get managedDialogLineBoundary =>
      'Somente os arquivos do projeto são alterados. Isso não cria um tópico AngelScript nem um diálogo jogável e nunca altera a instalação do jogo ou os arquivos salvos. O campo do falante é apenas um rótulo; ele não vincula nenhum NPC.';

  @override
  String get managedDialogLineCreateMode => 'Escrever novo texto';

  @override
  String get managedDialogLineReuseMode => 'Usar texto do projeto';

  @override
  String get managedDialogLineNameLabel => 'Nome da linha';

  @override
  String get managedDialogLineNameHint => 'Saudação na entrada da mina';

  @override
  String get managedDialogLineSpeakerLabel => 'Rótulo do falante (opcional)';

  @override
  String get managedDialogLineSpeakerHint => 'Por exemplo, Viper';

  @override
  String get managedDialogLineLocaleLabel => 'Idioma';

  @override
  String get managedDialogLineTextLabel => 'Texto do diálogo';

  @override
  String get managedDialogLineReuseSearch =>
      'Pesquisar texto do projeto não usado';

  @override
  String get managedDialogLineNoReusableText =>
      'Não há texto de projeto não usado e estruturalmente válido que possa ser vinculado. Em vez disso, escreva um novo texto.';

  @override
  String get managedDialogLineCreateSlotLabel =>
      'Preparar este idioma para Voice';

  @override
  String get managedDialogLineCreateSlotHelp =>
      'Cria um espaço Voice vazio e não resolvido no projeto. Não adiciona nem implanta uma gravação.';

  @override
  String get managedDialogLineCancel => 'Cancelar';

  @override
  String get managedDialogLineSave => 'Salvar no projeto';

  @override
  String get managedDialogLineSaving => 'Salvando…';

  @override
  String get managedDialogLineLoading => 'Lendo o conteúdo exato do projeto…';

  @override
  String get managedDialogLineLoadFailed =>
      'Não foi possível ler o conteúdo atual exato do projeto. Nada foi alterado.';

  @override
  String get managedDialogLineRetry => 'Tentar novamente';

  @override
  String get managedDialogLineStale =>
      'O projeto foi alterado enquanto esta janela estava aberta. Feche-a e tente novamente a partir do projeto atual.';

  @override
  String get managedDialogLineRequiresReopen =>
      'Não é mais possível verificar o projeto atual com segurança. Feche esta janela e reabra o projeto gerenciado.';

  @override
  String get managedDialogLineInvalidInput =>
      'Verifique a entrada do projeto destacada e escolha uma opção atual exata.';

  @override
  String get managedDialogLineSaveFailed =>
      'Não foi possível salvar a linha de diálogo com segurança. O jogo e os arquivos salvos não foram alterados.';

  @override
  String get managedDialogLineDone => 'Concluído';

  @override
  String get managedDialogLineAddRecording => 'Adicionar gravação';

  @override
  String get managedActionAddVoiceTakeTitle => 'Adicionar gravação de voz';

  @override
  String get managedActionAddVoiceTakeDescription =>
      'Importe uma gravação Ogg Vorbis para este projeto sem implantá-la.';

  @override
  String get managedActionManageVoiceTakesTitle => 'Gerenciar gravações de voz';

  @override
  String get managedActionManageVoiceTakesDescription =>
      'Revise as gravações e selecione as aprovadas para os espaços Voice.';

  @override
  String get managedActionResolveVoiceTargetTitle => 'Resolver destino Voice';

  @override
  String get managedActionResolveVoiceTargetDescription =>
      'Associe os espaços Voice do projeto aos membros exatos dos arquivos instalados sem alterar o jogo.';

  @override
  String get managedActionBuildVoiceBundleTitle => 'Compilar pacote Voice';

  @override
  String get managedActionBuildVoiceBundleDescription =>
      'Compile um pacote offline selado a partir de membros existentes; nenhuma implantação é realizada.';

  @override
  String get managedActionDataAssetsTitle => 'Edições de DataAssets';

  @override
  String get managedActionDataAssetsDescription =>
      'Inspecione os pacotes instalados e prepare no projeto edições verificadas de valores de largura fixa.';

  @override
  String get managedActionBrowseProjectContentDescription =>
      'Navegue pelo conteúdo exato do projeto e por suas referências resolvidas ou não resolvidas.';

  @override
  String get managedActionSettingsTitle => 'Configurações';

  @override
  String get managedActionSettingsDescription =>
      'Configure a instalação do Gothic 1 Remake e as preferências do Mod Studio.';

  @override
  String projectStarterSetupOpenFailed(String projectId) {
    return 'O projeto $projectId foi criado com segurança, mas a configuração inicial não abriu. O projeto vazio válido continua ativo.';
  }

  @override
  String projectStarterOutcomeUnverified(String projectId) {
    return 'O projeto $projectId foi criado, mas o Mod Studio não consegue verificar o resultado inicial. Reabra o projeto gerenciado antes de continuar; o jogo e os saves não foram alterados.';
  }

  @override
  String projectStarterNpcCancelled(String projectId) {
    return 'O projeto $projectId foi criado. O início de NPC não foi adicionado, então o projeto vazio válido continua ativo.';
  }

  @override
  String projectStarterNpcSaved(int projectRevision) {
    return 'Início de NPC salvo na revisão $projectRevision. Ele continua bloqueado para compilação, não qualificado em execução e não é gerado.';
  }

  @override
  String projectStarterQuestCancelled(String projectId) {
    return 'O projeto $projectId foi criado. O início de missão não foi adicionado, então o projeto vazio válido continua ativo.';
  }

  @override
  String projectStarterQuestSaved(int projectRevision) {
    return 'Início de missão salvo na revisão $projectRevision. Ele continua bloqueado para compilação e não qualificado em execução.';
  }

  @override
  String get projectStarterSemanticsLabel => 'Início do projeto';

  @override
  String get projectStarterPrompt => 'Como você quer começar?';

  @override
  String get projectStarterWriteBoundary =>
      'Escolher um início não grava nada. O projeto só é criado depois que você envia este formulário e escolhe uma pasta vazia.';

  @override
  String get projectStarterEmptyTitle => 'Projeto vazio';

  @override
  String get projectStarterEmptyDescription =>
      'Crie apenas o projeto gerenciado. Adicione conteúdo quando quiser.';

  @override
  String get projectStarterNpcDraftTitle => 'Rascunho de NPC';

  @override
  String get projectStarterNpcDraftDescription =>
      'Crie primeiro o projeto vazio e depois abra a configuração guiada do rascunho de NPC.';

  @override
  String get projectStarterQuestDraftTitle => 'Rascunho de missão';

  @override
  String get projectStarterQuestDraftDescription =>
      'Crie primeiro o projeto vazio e depois abra a configuração guiada do rascunho de missão.';

  @override
  String get projectStarterPartialOutcome =>
      'Se você cancelar a configuração guiada de NPC ou missão, ou se o rascunho falhar, um projeto vazio válido permanece. A escolha não grava no jogo nem em um save.';

  @override
  String get managedContentWorkspaceBrowseLabel => 'Explorar';

  @override
  String get managedContentWorkspaceVerifiedEditsLabel => 'Edições verificadas';

  @override
  String get managedContentScopeBaseGameLabel => 'Jogo base';

  @override
  String get managedContentScopeInstalledLabel => 'Instalado';

  @override
  String get managedBaseGameBrowserTitle =>
      'Pontos de partida compatíveis do jogo base';

  @override
  String get managedBaseGameBrowserDescription =>
      'Explore evidências exatas do jogo instalado que o Mod Studio pode inspecionar ou usar como ponto de partida seguro para um rascunho. Não é um catálogo completo do conteúdo original.';

  @override
  String get managedBaseGameBrowserLoading =>
      'Lendo evidências exatas do jogo base…';

  @override
  String get managedBaseGameBrowserRefresh => 'Ler um novo catálogo exato';

  @override
  String get managedBaseGameBrowserSearchLabel =>
      'Pesquisar conteúdo compatível do jogo base';

  @override
  String get managedBaseGameBrowserFilterNpcs => 'NPCs';

  @override
  String get managedBaseGameBrowserFilterQuests => 'Missões';

  @override
  String get managedBaseGameBrowserNpcSectionTitle =>
      'Pontos de partida de NPC';

  @override
  String get managedBaseGameBrowserQuestSectionTitle =>
      'Pontos de partida de missão';

  @override
  String get managedBaseGameBrowserExperimentalNpcSectionTitle =>
      'Arquétipos de NPC somente para inspeção';

  @override
  String get managedBaseGameBrowserSearchForExperimental =>
      'Pesquise para incluir mais evidências de NPC com ligação estática. Essas linhas não permitem criar um rascunho.';

  @override
  String get managedBaseGameBrowserEmpty =>
      'Nenhum resultado compatível do jogo base corresponde à pesquisa.';

  @override
  String get managedBaseGameBrowserLoadErrorTitle =>
      'Evidências do jogo base indisponíveis';

  @override
  String get managedBaseGameBrowserLoadErrorDescription =>
      'Não foi possível ler o catálogo exato compatível. Nenhum arquivo do projeto, jogo ou save foi alterado.';

  @override
  String get managedBaseGameBrowserOfflineDraftBadge =>
      'Rascunho offline compatível';

  @override
  String get managedBaseGameBrowserInspectOnlyBadge => 'Somente inspeção';

  @override
  String get managedBaseGameBrowserCreateNpcDraft => 'Usar como início de NPC';

  @override
  String get managedBaseGameBrowserCreateQuestDraft =>
      'Usar como início de missão';

  @override
  String get managedBaseGameBrowserSpawnClass => 'Definição de geração';

  @override
  String get managedBaseGameBrowserActorBlueprint => 'Blueprint de ator';

  @override
  String get managedBaseGameBrowserExperimentalResultsCapped =>
      'Os primeiros 100 resultados somente para inspeção são exibidos. Refine a pesquisa para obter resultados mais específicos.';

  @override
  String get managedInstalledBrowserLoading =>
      'Lendo o inventário exato de pacotes instalados…';

  @override
  String managedInstalledBrowserCompleteSummary(int count) {
    return '$count pacotes instalados candidatos';
  }

  @override
  String managedInstalledBrowserPartialSummary(int count) {
    return '$count pacotes instalados candidatos — resultado parcial';
  }

  @override
  String get managedInstalledBrowserCompleteDescription =>
      'Os metadados da pasta foram lidos e o instantâneo instalado permaneceu exato.';

  @override
  String get managedInstalledBrowserPartialDescription =>
      'Alguns metadados de pacotes estavam ausentes ou não eram canônicos; os resultados ajudam na descoberta, mas não estão completos.';

  @override
  String get managedInstalledBrowserAuthorityNotice =>
      'Este escopo mostra apenas metadados de pacotes DataAsset instalados. Inspecionar ou copiar um caminho não concede autoridade de compilação, implantação, execução ou gravação no jogo.';

  @override
  String get managedInstalledBrowserRefresh => 'Ler um novo instantâneo exato';

  @override
  String get managedInstalledBrowserSearchLabel =>
      'Pesquisar DataAssets instalados';

  @override
  String get managedInstalledBrowserSearchHint =>
      'Nome do recurso ou caminho /Game';

  @override
  String get managedInstalledBrowserSearchPrompt =>
      'Digite um nome de recurso ou caminho /Game para pesquisar.';

  @override
  String get managedInstalledBrowserNoMatchesTitle =>
      'Nenhum DataAsset instalado correspondente';

  @override
  String get managedInstalledBrowserNoMatchesDescription =>
      'Tente outro nome de recurso ou um caminho /Game mais amplo.';

  @override
  String get managedInstalledBrowserResultLimitDescription =>
      'Os primeiros 100 resultados são exibidos. Refine a pesquisa para restringir o instantâneo exato.';

  @override
  String get managedInstalledBrowserKindBadge => 'Pacote DataAsset';

  @override
  String get managedInstalledBrowserMetadataOnlyBadge => 'Somente metadados';

  @override
  String get managedInstalledBrowserOpenInspector => 'Inspecionar pacote exato';

  @override
  String get managedInstalledBrowserErrorTitle =>
      'Inventário de pacotes instalados indisponível';

  @override
  String get managedInstalledBrowserErrorDescription =>
      'Não foi possível ler o instantâneo instalado exato. Nenhum arquivo do projeto, jogo ou save foi alterado.';

  @override
  String get managedGlobalSearchScopeLabel => 'Pesquisar tudo';

  @override
  String get managedGlobalSearchTitle => 'Pesquisar todo o conteúdo';

  @override
  String get managedGlobalSearchLabel =>
      'NPC, missão, fala, recurso, ID ou caminho /Game';

  @override
  String get managedGlobalSearchAction => 'Pesquisar';

  @override
  String get managedGlobalSearchClear => 'Limpar';

  @override
  String get managedGlobalSearchPrompt =>
      'Digite uma pesquisa para consultar as três fontes de forma independente.';

  @override
  String get managedGlobalSearchNoResults =>
      'Nenhuma correspondência nesta fonte.';

  @override
  String get managedGlobalSearchLoading => 'Lendo a fonte exata…';

  @override
  String get managedGlobalSearchFailed => 'Não foi possível ler esta fonte.';

  @override
  String get managedGlobalSearchComplete => 'Completo';

  @override
  String get managedGlobalSearchPartial => 'Parcial';

  @override
  String get managedGlobalSearchTruncated =>
      'Mostrando as primeiras 100 correspondências. Refine a pesquisa.';

  @override
  String get managedGlobalSearchOpen => 'Abrir';

  @override
  String get managedGlobalSearchCreateDraft => 'Criar rascunho';

  @override
  String get managedGlobalSearchInspect => 'Inspecionar';

  @override
  String get managedGlobalSearchKindModEntity => 'Conteúdo do mod';

  @override
  String get managedGlobalSearchKindModAsset => 'Recurso do mod';

  @override
  String get managedGlobalSearchKindBaseNpc => 'Ponto de partida de NPC';

  @override
  String get managedGlobalSearchKindBaseQuest => 'Ponto de partida de missão';

  @override
  String get managedGlobalSearchKindExperimentalNpc => 'Evidência de NPC';

  @override
  String get managedGlobalSearchReadinessExact => 'Projeto atual exato';

  @override
  String get managedGlobalSearchReadinessProblems => 'Exato, com problemas';

  @override
  String get managedGlobalSearchResultStale =>
      'Este resultado não está mais no projeto atual. Pesquise novamente.';

  @override
  String get managedStoryWorkbenchDraftBadge => 'Somente rascunho';

  @override
  String get managedStoryWorkbenchBuildBlockedBadge => 'Compilação bloqueada';

  @override
  String get managedStoryWorkbenchRuntimeUnqualifiedBadge =>
      'Execução não verificada';

  @override
  String get managedStoryWorkbenchOverviewTab => 'Visão geral';

  @override
  String get managedStoryWorkbenchProfileTab => 'Perfil';

  @override
  String get managedStoryWorkbenchStoryTab => 'História';

  @override
  String get managedStoryWorkbenchLogicTab => 'Lógica';

  @override
  String get managedStoryWorkbenchRoutineTab => 'Rotina';

  @override
  String get managedStoryWorkbenchInventoryTab => 'Inventário';

  @override
  String get managedStoryWorkbenchDialogVoiceTab => 'Diálogo e voz';

  @override
  String get managedStoryWorkbenchReferencesTab => 'Referências';

  @override
  String get managedStoryWorkbenchProblemsChecksTab =>
      'Problemas e verificações';

  @override
  String get managedStoryWorkbenchEditOverview => 'Editar nome e objetivos';

  @override
  String get managedStoryWorkbenchEditStory => 'Editar descrição e conexões';

  @override
  String get managedStoryWorkbenchEditLogic => 'Editar estados e transições';

  @override
  String get managedStoryWorkbenchInspectQuest =>
      'Abrir código-fonte e verificações do compilador';

  @override
  String get managedStoryWorkbenchInspectNpc =>
      'Abrir perfil e verificações do compilador';

  @override
  String get managedStoryWorkbenchCapabilityUnavailable => 'Ainda não modelado';

  @override
  String get managedStoryWorkbenchNpcStoryUnavailable =>
      'As relações com missões e história ainda não estão modeladas para rascunhos de NPC.';

  @override
  String get managedStoryWorkbenchNpcRoutineUnavailable =>
      'A rotina e o posicionamento no mundo ainda não estão modelados.';

  @override
  String get managedStoryWorkbenchNpcInventoryUnavailable =>
      'O inventário, os equipamentos e o comércio ainda não estão modelados.';

  @override
  String get managedStoryWorkbenchNpcDialogVoiceUnavailable =>
      'As relações de diálogo, localização e voz ainda não estão modeladas para rascunhos de NPC.';

  @override
  String get managedStoryWorkbenchQuestDialogVoiceUnavailable =>
      'As relações de diálogo, localização e voz ainda não estão modeladas para rascunhos de missão.';

  @override
  String get managedStoryWorkbenchNoReferenceProblems =>
      'Não há referências de projeto não resolvidas';

  @override
  String managedStoryWorkbenchReferenceProblemCount(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count referências de projeto não resolvidas',
      one: '1 referência de projeto não resolvida',
    );
    return '$_temp0';
  }

  @override
  String get managedStoryWorkbenchReferenceScopeNotice =>
      'Indica apenas o status das referências; não confirma que o projeto esteja pronto para compilação ou execução.';

  @override
  String get managedStoryWorkbenchTechnicalDetails => 'Detalhes técnicos';

  @override
  String get managedStoryWorkbenchQuestKindLabel => 'Rascunho de missão';

  @override
  String get managedStoryWorkbenchNpcKindLabel => 'Rascunho de NPC';

  @override
  String get managedStoryWorkbenchQuestTitleLabel => 'Título da missão';

  @override
  String get managedStoryWorkbenchTechnicalIdLabel => 'ID técnico';

  @override
  String get managedStoryWorkbenchObjectivesLabel => 'Objetivos';

  @override
  String get managedStoryWorkbenchUniqueNameLabel => 'Nome exclusivo';

  @override
  String get managedStoryWorkbenchModuleNamespaceLabel => 'Namespace do módulo';

  @override
  String get managedStoryWorkbenchQuestGiverLabel => 'Concedente da missão';

  @override
  String get managedStoryWorkbenchRuntimeParentLabel =>
      'Classe-base em tempo de execução';

  @override
  String get managedStoryWorkbenchLogicDescription =>
      'Os estados do ciclo de vida da missão, os gatilhos, as condições e os efeitos são editados como uma única operação atômica sobre o estado atual exato.';

  @override
  String get managedStoryWorkbenchOutgoingHeading => 'Saída';

  @override
  String get managedStoryWorkbenchNoOutgoingReferences =>
      'Nenhuma referência projetada';

  @override
  String get managedStoryWorkbenchIncomingHeading => 'Entrada';

  @override
  String get managedStoryWorkbenchNoIncomingReferences =>
      'Nenhuma referência de projeto recebida';

  @override
  String get managedStoryWorkbenchSemanticIdentityLabel =>
      'Identidade semântica';

  @override
  String get managedStoryWorkbenchOriginLabel => 'Origem';

  @override
  String get managedStoryWorkbenchEntityRevisionLabel => 'Revisão da entidade';

  @override
  String get managedStoryWorkbenchStableIdLabel => 'ID estável';

  @override
  String get managedStoryWorkbenchReferenceResolvedLabel =>
      'Referência resolvida';

  @override
  String get managedStoryWorkbenchReferenceUnresolvedLabel =>
      'Referência não resolvida';
}
