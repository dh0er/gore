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
  String get projectOpenManagedRevision3 => 'Open mod project…';

  @override
  String get projectVerifyCurrentHead => 'Check project';

  @override
  String get projectManagedRevision3Title => 'Mod project';

  @override
  String get projectClose => 'Close project';

  @override
  String projectCloseFailed(String error) {
    return 'Project could not be closed: $error';
  }

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
  String get projectManagedRevision3Opened => 'Mod project opened.';

  @override
  String projectManagedRevision3OpenFailed(String error) {
    return 'Mod project could not be opened: $error';
  }

  @override
  String get projectManagedRevision3Verified => 'Project checked.';

  @override
  String projectManagedRevision3VerifyFailed(String error) {
    return 'Project check failed: $error';
  }

  @override
  String get projectManagedRevision3RequiresReopen =>
      'The project could not be checked safely. Recover or reopen it before continuing.';

  @override
  String get projectManagedRevision3VerifyBlocked =>
      'Recover or reopen the project before checking it again.';

  @override
  String get projectTransitionCleanupWarning =>
      'O novo projeto está aberto, mas não foi possível limpar completamente a sessão do projeto anterior. A limpeza não será repetida. Reinicie o Mod Studio antes de voltar a abrir o projeto anterior.';

  @override
  String get projectNewManagedRevision3 => 'Novo projeto de mod…';

  @override
  String get projectCreateGamePathRequired =>
      'Defina o caminho do Gothic 1 Remake nas Definições antes de criar um projeto de mod.';

  @override
  String get projectCreateDirectoryPickerTitle =>
      'Criar aqui o projeto de mod gerido';

  @override
  String projectManagedRevision3Created(String projectId) {
    return 'Projeto de mod $projectId criado';
  }

  @override
  String projectManagedRevision3CreateFailed(String error) {
    return 'Falha ao criar o projeto de mod: $error';
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
  String get managedWorkspaceSettingsExpertLabel =>
      'Definições e modo especialista';

  @override
  String get managedProjectHistoryTitle => 'Project history';

  @override
  String get managedProjectHistoryDescription =>
      'Return to an earlier project version without erasing the versions that came after it.';

  @override
  String get managedProjectHistoryBoundary =>
      'History changes only this managed project. It does not modify the game installation or save files.';

  @override
  String get managedProjectHistoryRefresh => 'Refresh project history';

  @override
  String get managedProjectHistoryLoading => 'Loading project history…';

  @override
  String get managedProjectHistoryLoadFailed =>
      'Project history could not be loaded';

  @override
  String get managedProjectHistoryRetry => 'Try again';

  @override
  String get managedProjectHistoryCurrentVersion => 'Current version';

  @override
  String get managedProjectHistoryPreviousVersions => 'Previous versions';

  @override
  String get managedProjectHistoryUndo => 'Undo last change';

  @override
  String get managedProjectHistoryRestoreVersion => 'Restore this version';

  @override
  String get managedProjectHistoryRestoreTitle => 'Restore project version?';

  @override
  String managedProjectHistoryRestoreBody(int revision, int nextRevision) {
    return 'The content from revision $revision will be saved as new revision $nextRevision. The current version remains in history.';
  }

  @override
  String get managedProjectHistoryRestoreBoundary =>
      'Only the project changes. The game installation and save files remain untouched.';

  @override
  String get managedProjectHistoryCancel => 'Cancel';

  @override
  String get managedProjectHistoryRestore => 'Restore';

  @override
  String get managedProjectHistoryRestoring => 'Restoring project version…';

  @override
  String get managedProjectHistoryRestoreFailed =>
      'The project version could not be restored safely. Refresh the history before trying again.';

  @override
  String managedProjectHistoryRestoreSucceeded(int revision) {
    return 'Revision $revision was restored as a new project version.';
  }

  @override
  String get managedProjectHistoryEmpty =>
      'No previous project versions have been recorded yet.';

  @override
  String managedProjectHistoryRecordingStartsAt(int revision) {
    return 'History recording starts at revision $revision; older versions were not guessed from storage.';
  }

  @override
  String get managedProjectHistoryTruncated =>
      'Older project versions have expired from history. Every version shown here is still retained and authenticated by the current project history.';

  @override
  String managedProjectHistoryRevision(int revision) {
    return 'Revision $revision';
  }

  @override
  String get managedProjectHistoryCurrentBadge => 'Current';

  @override
  String get managedProjectHistoryDirtyBlocked =>
      'Finish or discard the open text edit before restoring another project version.';

  @override
  String get managedProjectHistoryBusy =>
      'Another project action is still in progress.';

  @override
  String get managedProjectHistoryUnavailable =>
      'This managed project session does not support authenticated history.';

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
  String get managedStoryWorkspaceCreateNpcOpening =>
      'Create Character + first greeting';

  @override
  String get managedStoryWorkspaceCreatingNpcOpening =>
      'Creating Character + first greeting…';

  @override
  String get managedStoryWorkspaceCreateQuestOpening =>
      'Create Quest + opening line';

  @override
  String get managedStoryWorkspaceCreatingQuestOpening =>
      'Creating Quest + opening line…';

  @override
  String get managedStoryWorkspaceCreateAdvanced => 'Advanced creation options';

  @override
  String get managedStoryWorkspaceCreateNpcAdvanced =>
      'Create Character draft only (advanced)';

  @override
  String get managedStoryWorkspaceCreateQuestAdvanced =>
      'Create Quest draft only (advanced)';

  @override
  String get managedStoryWorkspaceMutationRequiresReopen =>
      'Reopen this project before changing Story content.';

  @override
  String get managedStoryWorkspaceMutationDirtyBlocked =>
      'Save or discard the open localization edits before changing Story content.';

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
  String get managedLocalizationVoiceContextTitle =>
      'Voice for this dialog line';

  @override
  String get managedLocalizationVoiceSelectLine => 'Select a dialog line above';

  @override
  String get managedLocalizationVoiceSetupExists => 'setup exists';

  @override
  String get managedLocalizationVoiceSetupMissing => 'no setup yet';

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
  String get managedLocalizationVoiceUnsavedTitle =>
      'Save text before continuing?';

  @override
  String get managedLocalizationVoiceUnsavedDescription =>
      'Save these text changes and continue directly to the selected action, keep editing, or deliberately discard the text changes.';

  @override
  String get managedLocalizationDiscardAndContinue => 'Discard and continue';

  @override
  String get managedLocalizationSaveAndContinue => 'Save and continue';

  @override
  String get managedLocalizationGlobalAddVoice => 'Add take for any line';

  @override
  String get managedLocalizationGlobalManageVoice =>
      'Manage takes for any line';

  @override
  String get managedLocalizationGlobalResolveVoice =>
      'Resolve target for any line';

  @override
  String get managedVoiceFolderImportTitle => 'Import recordings folder';

  @override
  String get managedVoiceFolderImportDescription =>
      'Review a folder of named Ogg recordings, then add every ready take in one all-or-nothing project update.';

  @override
  String get managedVoiceFolderImportChooseFolder => 'Choose recordings folder';

  @override
  String get managedVoiceFolderImportDirtyBlocked =>
      'Save or discard the open localization edits before importing recordings.';

  @override
  String managedVoiceFolderImportSaved(int count, int revision) {
    return 'Imported $count recordings in project revision $revision. They are project-only Recorded takes; selection, game files, and saves were not changed.';
  }

  @override
  String managedVoiceTakeSaved(int revision) {
    return 'Voice take saved in project revision $revision. It is saved to the project only and is not yet usable in game.';
  }

  @override
  String managedVoiceSelectionCleared(int revision) {
    return 'Voice selection cleared in project revision $revision. Voice build remains a separate offline step; runtime remains unqualified.';
  }

  @override
  String managedVoiceSelectionSelected(int revision) {
    return 'Approved Voice take selected in project revision $revision. Voice build remains a separate offline step; runtime remains unqualified.';
  }

  @override
  String managedVoiceTargetUnresolvedSaved(int revision) {
    return 'No installed archive member matched. Voice target evidence saved in project revision $revision.';
  }

  @override
  String managedVoiceTargetResolvedSaved(int revision) {
    return 'One installed archive member was sealed. Voice target evidence saved in project revision $revision.';
  }

  @override
  String managedVoiceTargetAmbiguousSaved(int count, int revision) {
    return '$count installed archive members matched; nothing was chosen implicitly. Voice target evidence saved in project revision $revision.';
  }

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
  String get managedLocalizationVoiceActionFailed =>
      'The selected action did not finish cleanly. Refresh the project before trying again; the exact current project will show whether a change was published. This workspace did not change game or save files.';

  @override
  String get managedSectionSettingsExpertDescription =>
      'As definições e o DataAsset Lab só de leitura estão disponíveis.';

  @override
  String get managedSettingsExpertDataAssetLabLabel => 'DataAsset Lab';

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
  String get managedProjectLandingTitle => 'Iniciar um projeto de mod';

  @override
  String get managedProjectLandingDescription =>
      'Crie um projeto, abra uma pasta de projeto existente ou restaure uma cópia de segurança.';

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
  String get managedDashboardContinueHeading => 'Continue working';

  @override
  String get managedHomeStoryEmptyTitle => 'Create a character or Quest';

  @override
  String get managedHomeStoryContinueTitle => 'Continue Story';

  @override
  String get managedHomeStoryDescription =>
      'Create and develop NPC and Quest drafts in the complete Story workspace.';

  @override
  String get managedHomeDialogVoiceTitle => 'Dialog & Voice';

  @override
  String get managedHomeDialogVoiceDescription =>
      'Write project text, create dialog lines, and manage Voice takes in one place.';

  @override
  String get managedHomeProblemsTitle => 'Review problems';

  @override
  String get managedHomeProblemsDescription =>
      'Review exact project issues and verification without claiming a runtime test.';

  @override
  String get managedHomeContentTitle => 'Browse content';

  @override
  String get managedHomeContentDescription =>
      'Find project, base-game, installed, and verified DataAsset content.';

  @override
  String get managedHomeBuildTitle => 'Check build readiness';

  @override
  String get managedHomeBuildDescription =>
      'Open the honest build view. Voice bundles are available; a complete playable mod is still blocked.';

  @override
  String get managedContentOpenInStory => 'Open in Story';

  @override
  String get managedContentOpenInStoryDescription =>
      'Continue this Quest or NPC in the complete Story workspace.';

  @override
  String get managedContentOpenInStoryRequiresReopen =>
      'Reopen this project before opening Story.';

  @override
  String get managedContentOpenInStoryFailed =>
      'Story could not be opened. The project was not changed.';

  @override
  String get managedStoryWorkbenchActionFailed =>
      'Could not open this editor. Please try again.';

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
  String managedNpcDraftSaved(int projectRevision) {
    return 'Character draft saved in project revision $projectRevision. It remains build-blocked, runtime-unqualified, and is not spawned.';
  }

  @override
  String get managedNpcOpeningRecipeTitle => 'Character + first greeting';

  @override
  String get managedNpcOpeningRecipeDescription =>
      'Recommended: create a project-only Character draft, then write and insert its first localized greeting. This uses two project checkpoints and does not create a playable conversation or spawn.';

  @override
  String get managedNpcOpeningRecipeIntroduction =>
      'This guided flow first saves the Character draft, then opens its first greeting line. If you stop after step 1, the draft stays saved. It does not create dialog logic, runtime behavior, a spawn, or change the game or save files.';

  @override
  String get managedNpcOpeningRecipeStart => 'Start guided Character';

  @override
  String get managedNpcOpeningGreetingTitle => 'Step 2 of 2: First greeting';

  @override
  String get managedNpcOpeningGreetingIntroduction =>
      'Write the first localized greeting line for this Character draft. Saving creates the line and its text, then inserts it at the start of the draft\'s greeting list. It does not add choices, conditions, effects, or playable conversation behavior.';

  @override
  String managedNpcOpeningRecipePartial(int projectRevision) {
    return 'Character draft saved in project revision $projectRevision; no greeting was added. Continue in Story > Dialog & Voice.';
  }

  @override
  String get managedNpcOpeningRecipeFailed =>
      'The guided Character could not be started. The exact project checkpoint is unchanged; game and save files were not changed.';

  @override
  String get managedNpcOpeningRecipeStopped =>
      'The guided flow stopped because its exact project checkpoint or publication could not be verified. No further step will run automatically; inspect Story and continue manually.';

  @override
  String get managedNpcOpeningRecipeRequiresReopen =>
      'The guided flow could not safely continue. Reopen this project and inspect Story before retrying or continuing manually.';

  @override
  String managedNpcOpeningRecipeComplete(int projectRevision) {
    return 'Character draft and first greeting saved in project revision $projectRevision. Draft only: no playable conversation or spawn was created; game and save files were not changed.';
  }

  @override
  String get managedActionNewQuestTitle => 'Nova missão';

  @override
  String get managedActionNewQuestDescription =>
      'Crie um rascunho de missão offline com objetivos e identidades principais verificadas.';

  @override
  String get managedQuestOpeningRecipeTitle => 'Missão + primeira fala';

  @override
  String get managedQuestOpeningRecipeDescription =>
      'Recomendado: crie um rascunho de missão e depois escreva e insira a primeira fala localizada. Este fluxo usa dois pontos de controlo do projeto e não cria um diálogo jogável.';

  @override
  String get managedQuestOpeningRecipeIntroduction =>
      'Este fluxo guiado guarda primeiro a missão e depois abre a sua primeira fala. Se parar após o passo 1, a missão continuará guardada. Não cria um diálogo jogável nem altera o jogo ou os ficheiros guardados.';

  @override
  String get managedQuestOpeningRecipeStart => 'Iniciar missão guiada';

  @override
  String get managedQuestOpeningLineTitle => 'Passo 2 de 2: primeira fala';

  @override
  String get managedQuestOpeningLineIntroduction =>
      'Escreva a primeira fala localizada desta missão. Ao guardar, são criados a fala e o respetivo texto, que são depois inseridos no início da transcrição da missão.';

  @override
  String managedQuestOpeningRecipePreparing(int projectRevision) {
    return 'Missão guardada na revisão $projectRevision do projeto. A preparar a primeira fala…';
  }

  @override
  String managedQuestOpeningRecipePartial(int projectRevision) {
    return 'Missão guardada na revisão $projectRevision do projeto; não foi adicionada uma primeira fala. Continue em História > Diálogo e voz.';
  }

  @override
  String get managedQuestOpeningRecipeFailed =>
      'Não foi possível iniciar a missão guiada. Não foi publicada nenhuma alteração do projeto.';

  @override
  String get managedQuestOpeningRecipeStopped =>
      'O fluxo guiado parou porque o estado atual exato do projeto mudou. Não será executado automaticamente mais nenhum passo; verifique História e continue manualmente.';

  @override
  String get managedQuestOpeningRecipeRequiresReopen =>
      'O fluxo guiado não pôde continuar em segurança. Volte a abrir este projeto e verifique História antes de tentar novamente ou continuar manualmente.';

  @override
  String managedQuestOpeningRecipeComplete(int projectRevision) {
    return 'Missão e primeira fala guardadas na revisão $projectRevision do projeto. Apenas rascunho: não foi criado nenhum diálogo jogável e não foram alterados o jogo ou os ficheiros guardados.';
  }

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
  String get managedItemsBundledReferenceBadge => 'Bundled reference';

  @override
  String get managedItemsBundledReferenceBoundary =>
      'Read-only reference shipped with Mod Studio. It has not been refreshed or generation-qualified against your configured game installation.';

  @override
  String get managedItemsNoKnownFields =>
      'No modeled scalar fields are available for this item.';

  @override
  String get managedItemsCategorySpecial => 'Special';

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
  String get projectExportActionTitle => 'Create project backup…';

  @override
  String get projectExportActionDescription =>
      'Write an exact restorable backup of the current saved project checkpoint.';

  @override
  String get projectExportActionDirtyBlocked =>
      'Save or discard the open localization edits before creating a project backup.';

  @override
  String get projectExportDialogTitle => 'Create project backup';

  @override
  String get projectExportPortableCopyTitle =>
      'Restorable Mod Studio project backup';

  @override
  String get projectExportPortableCopyDescription =>
      'This writes the exact current saved project checkpoint to a new .goremod file. It can be restored into a new project folder later; the open project stays current and unchanged.';

  @override
  String get projectExportCapabilityBoundary =>
      'This backup is not a playable mod, build, deployment, or runtime qualification. Creating it does not read or change the game or any save.';

  @override
  String get projectExportKeepOriginal =>
      'A restore preserves this project\'s identity and history. Use Clone or Save As for a separate project identity when those workflows become available.';

  @override
  String get projectExportFileNameLabel => 'New project-backup file';

  @override
  String get projectExportFileNameHelper =>
      'Use a new backup file name ending in .goremod.';

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
  String get projectExportSubmit => 'Create backup';

  @override
  String get projectExportExporting => 'Creating backup…';

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
      'Enter a new project-backup file name.';

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
      'The project changed before backup creation started. No output was created. Close this window and open Create project backup again.';

  @override
  String get projectExportRequiresReopen =>
      'This project can no longer be verified as current. No output was created. Close this window and recover or reopen the project.';

  @override
  String get projectExportUnsupported =>
      'This managed project session cannot create exact restorable backups. Nothing was created.';

  @override
  String get projectExportFailedBeforeStart =>
      'The project backup could not be prepared exactly. Nothing was created.';

  @override
  String get projectExportPrepublicationFailed =>
      'Backup creation stopped safely before the new local file was created. Nothing was created. Close this window and check the project and destination before trying again.';

  @override
  String projectExportMayExist(String output) {
    return 'Backup creation did not return a verified receipt. Do not retry. Close this window and check the destination: $output';
  }

  @override
  String projectExportResultMismatch(String output) {
    return 'The completed backup does not match this checkpoint or destination. Do not retry; inspect the destination: $output';
  }

  @override
  String get projectExportPublished =>
      'The exact restorable project backup was created as a new local file.';

  @override
  String get projectExportPublishedCleanupWarning =>
      'The exact restorable project backup was created as a local file, but internal temporary-file cleanup was incomplete. The created file is valid; do not retry.';

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

  @override
  String get projectRestoreActionTitle => 'Restore project backup…';

  @override
  String get projectRestoreActionDescription =>
      'Verify an exact .goremod backup, restore it into a new folder, and open that project safely.';

  @override
  String get projectRestoreDialogTitle => 'Restore project backup';

  @override
  String get projectRestoreNoticeTitle => 'Restore into a new project folder';

  @override
  String get projectRestoreNoticeDescription =>
      'Choose a restorable Mod Studio .goremod backup. Studio verifies the complete archive before creating a new project folder and preserves the backed-up project identity and history.';

  @override
  String get projectRestoreCapabilityBoundary =>
      'Restore does not build, deploy, launch, or qualify the mod at runtime. It does not read or change the game or any save.';

  @override
  String get projectRestoreChooseBackup => 'Choose backup file';

  @override
  String get projectRestoreNoBackup => 'No verified backup selected';

  @override
  String get projectRestoreInspecting => 'Verifying backup…';

  @override
  String get projectRestoreVerified =>
      'This exact V2 project backup is complete and restorable.';

  @override
  String get projectRestoreSource => 'Backup file';

  @override
  String get projectRestoreProjectRevision => 'Project revision';

  @override
  String get projectRestoreArchiveBytes => 'Archive bytes';

  @override
  String get projectRestoreStoreObjects => 'Stored project objects';

  @override
  String get projectRestoreInvalidSource =>
      'The selected file is not a valid exact project backup. Nothing was created.';

  @override
  String get projectRestoreInspectionFailed =>
      'The backup could not be verified completely. Nothing was created.';

  @override
  String get projectRestoreUnavailable =>
      'Exact project restore is unavailable on this system. Nothing was created.';

  @override
  String get projectRestoreChooseDestinationParent => 'Choose parent folder';

  @override
  String get projectRestoreNoDestinationParent => 'No parent folder selected';

  @override
  String get projectRestoreFolderNameLabel => 'New project folder name';

  @override
  String get projectRestoreFolderNameHelper =>
      'Studio creates this new folder; it must not already exist.';

  @override
  String get projectRestoreNewFolder => 'New project folder';

  @override
  String get projectRestoreFolderNameRequired =>
      'Enter a new project folder name.';

  @override
  String get projectRestoreFolderNameTooLong => 'The folder name is too long.';

  @override
  String get projectRestoreFolderNameInvalid =>
      'Use one ordinary folder name without path separators, control characters, a trailing dot, or a trailing space.';

  @override
  String get projectRestoreFolderNameReserved =>
      'That folder name is reserved by Windows.';

  @override
  String get projectRestoreDestinationExists =>
      'That destination already exists. Choose a new folder name; existing content is never overwritten.';

  @override
  String get projectRestoreDestinationLink =>
      'The new project destination is a link. Choose a different folder name.';

  @override
  String get projectRestoreDestinationInvalid =>
      'The destination was rejected before a project receipt was created. Nothing was opened. Choose a different new folder after verifying the backup again.';

  @override
  String get projectRestoreInspectionExpired =>
      'The backup changed after verification. Nothing was opened. Verify the backup again before choosing another destination.';

  @override
  String get projectRestoreMaterializationFailed =>
      'Restore did not return a verified project receipt. Nothing was opened. Do not reuse this attempt; inspect the chosen destination before starting again.';

  @override
  String projectRestorePublicationUncertain(String destination) {
    return 'Studio cannot prove whether the project folder ‘$destination’ was published. Nothing was opened. Do not retry this restore; inspect that destination first.';
  }

  @override
  String get projectRestoreStale =>
      'This restore window is no longer current. Nothing was opened. If materialization had started, inspect the chosen destination before trying anything else.';

  @override
  String get projectRestoreCancel => 'Cancel';

  @override
  String get projectRestoreClose => 'Close';

  @override
  String get projectRestoreSubmit => 'Restore and open';

  @override
  String get projectRestoreRestoring => 'Restoring…';

  @override
  String get projectRestoreSucceeded =>
      'The exact project backup was restored into the new folder.';

  @override
  String get projectRestoreSucceededCleanupWarning =>
      'The exact project backup was restored, but private temporary cleanup was incomplete. The restored project is valid; do not repeat the restore.';

  @override
  String get projectRestoreOpened => 'Project backup restored and opened.';

  @override
  String get projectRestoreOpenedCleanupWarning =>
      'Project backup restored and opened. Private temporary cleanup was incomplete; do not repeat the restore.';

  @override
  String get projectRestoreOpening => 'Opening the restored project safely…';

  @override
  String projectRestoreOpenFailed(String destination) {
    return 'The project folder ‘$destination’ was restored, but Studio could not prove it safe to open. Any previously open project remains current; otherwise no project was opened. Do not repeat the restore; inspect or open the restored folder separately.';
  }

  @override
  String get projectRestoreCandidateCleanupWarning =>
      'No project was adopted. Studio could not completely clean up the rejected candidate session. Restart Mod Studio before opening the restored destination manually.';

  @override
  String get managedVoiceTakeRemoveAction => 'Remove from this line…';

  @override
  String get managedVoiceTakeRemoveTooltip =>
      'Remove this recording from the current dialog line and language';

  @override
  String get managedVoiceTakeRemoveDialogTitle => 'Remove Voice take?';

  @override
  String managedVoiceTakeRemoveDialogSummary(
    String take,
    String line,
    String locale,
  ) {
    return 'Remove “$take” from $line ($locale)?';
  }

  @override
  String get managedVoiceTakeRemoveScope =>
      'Only the link for this dialog line and language is removed. Other project uses remain unchanged.';

  @override
  String get managedVoiceTakeRemoveInternalRetention =>
      'The audio file remains stored internally. This action does not free project storage and has no undo yet.';

  @override
  String get managedVoiceTakeRemoveGameBoundary =>
      'The game installation and save games are not changed.';

  @override
  String get managedVoiceTakeRemoveSelectedWarning =>
      'This is the active take. Removing it also clears the selection atomically. No replacement is chosen automatically, so Voice build remains blocked until an Approved take is selected.';

  @override
  String get managedVoiceTakeRemoveCancel => 'Cancel';

  @override
  String get managedVoiceTakeRemoveConfirm => 'Remove from line';

  @override
  String get managedVoiceTakeRemoveUniqueSuccess =>
      'The take was removed from this line and from the current project graph. Its internal audio data remains retained.';

  @override
  String get managedVoiceTakeRemoveSharedSuccess =>
      'The link was removed from this line and language. The take remains available to its other project uses, and its internal audio data remains retained.';

  @override
  String get managedVoiceTakeRemoveSelectionClearedSuccess =>
      'The active selection was cleared atomically. No replacement was selected; Voice build is blocked until an Approved take is selected.';

  @override
  String get managedVoiceTakeRemoveStale =>
      'The project changed before the take could be removed. Reload the latest Voice takes and review the action again.';

  @override
  String get managedVoiceTakeRemoveRequiresReopen =>
      'The removal result could not be confirmed. Do not retry. Close this window and reopen or recover the managed project.';

  @override
  String get managedVoiceTakeRemoveSavedUnconfirmed =>
      'The removal was saved, but the latest project could not be confirmed. Do not repeat the removal. Close this window and reopen or recover the managed project.';

  @override
  String get managedVoiceTakeRemoveSavedReloadFailed =>
      'The removal was saved, but the latest Voice takes could not be loaded. Reload the takes; the removal will not be repeated.';

  @override
  String managedVoiceTakeRemoveFailed(String error) {
    return 'The take was not removed: $error';
  }

  @override
  String get managedVoiceTakeRemoveReloadConfirmed =>
      'The saved removal was confirmed from the latest project.';

  @override
  String get managedVoiceSlotRemoveAction => 'Remove empty Voice setup…';

  @override
  String get managedVoiceSlotRemoveDialogTitle => 'Remove empty Voice setup?';

  @override
  String managedVoiceSlotRemoveDialogSummary(String line, String locale) {
    return 'Remove the empty $locale Voice setup from $line?';
  }

  @override
  String get managedVoiceSlotRemoveRetention =>
      'The dialog text stays in the project. No recording, audio blob, game file, or save is deleted.';

  @override
  String get managedVoiceSlotRemoveTargetWarning =>
      'This also removes the stored installed-target evidence for this line and language. The installed archive itself remains untouched.';

  @override
  String get managedVoiceSlotRemoveRecreate =>
      'You can add a new take later; the required Voice setup will then be created again automatically.';

  @override
  String get managedVoiceSlotRemoveCancel => 'Keep setup';

  @override
  String get managedVoiceSlotRemoveConfirm => 'Remove setup';

  @override
  String get managedVoiceSlotRemoveSuccess =>
      'Empty Voice setup removed. The dialog text, audio storage, game files, and saves were not changed.';

  @override
  String get managedVoiceSlotPlanSuccess =>
      'Recording planned. An empty Voice setup was added for this line and language. No audio, game file, or save was changed; build and runtime remain unqualified.';

  @override
  String get managedVoiceSlotRemoveStale =>
      'The project changed before the empty Voice setup could be removed. Reload the latest Voice takes and try again.';

  @override
  String get managedVoiceSlotRemoveRequiresReopen =>
      'Reopen the managed project before removing this Voice setup.';

  @override
  String get managedVoiceSlotRemoveSavedUnconfirmed =>
      'The result could not be confirmed and the empty Voice setup may have been saved. Do not repeat the removal. Close this window, reopen the managed project, and inspect the line.';

  @override
  String get managedVoiceSlotRemoveSavedReloadFailed =>
      'The empty Voice setup was saved, but reloading failed. Reload to confirm it; the removal will not be repeated.';

  @override
  String managedVoiceSlotRemoveFailed(String error) {
    return 'The empty Voice setup could not be removed: $error';
  }

  @override
  String get managedVoiceSlotRemoveReloadConfirmed =>
      'Saved empty Voice setup removal confirmed from the latest project.';

  @override
  String get managedVoicePreviewTooltip => 'Preview selected local Ogg';

  @override
  String get managedVoicePreviewOpened =>
      'Opened the selected local recording for author preview. This does not approve or qualify the audio for the game.';

  @override
  String managedVoicePreviewFailed(String error) {
    return 'The local recording preview could not be opened: $error';
  }

  @override
  String get managedStoryWorkbenchEditNpcProfile => 'Edit name & archetype';

  @override
  String get managedStoryWorkbenchNpcDialogVoiceNextStepTitle =>
      'Next step: Dialog & Voice';

  @override
  String get managedStoryWorkbenchNpcDialogVoiceNextStepDescription =>
      'Draft only: continue with greeting lines, text, and voice. This only links project content; it does not create playable dialog or verify runtime behavior.';

  @override
  String get managedStoryWorkbenchContinueToNpcDialogVoice =>
      'Continue to Dialog & Voice';

  @override
  String get managedStoryWorkbenchNpcDisplayNameLabel => 'Character name';

  @override
  String get managedNpcProfileEditTitle => 'Edit name & archetype';

  @override
  String get managedNpcProfileEditDescription =>
      'Change the friendly character name or choose another verified structural starting point.';

  @override
  String get managedNpcProfileEditNameLabel => 'Character name';

  @override
  String get managedNpcProfileEditNameHint =>
      'Shown to authors in this project.';

  @override
  String get managedNpcProfileEditArchetypeLabel =>
      'Archetype / base character';

  @override
  String get managedNpcProfileEditArchetypeHelp =>
      'This does not edit appearance, stats, faction, routine, inventory, dialog, or spawn.';

  @override
  String get managedNpcProfileEditBoundary =>
      'Only the offline project draft changes. The game installation and save games remain unchanged.';

  @override
  String get managedNpcProfileEditLoading => 'Loading current NPC details…';

  @override
  String get managedNpcProfileEditCancel => 'Cancel';

  @override
  String get managedNpcProfileEditClose => 'Close';

  @override
  String get managedNpcProfileEditSave => 'Save changes';

  @override
  String get managedNpcProfileEditSaving => 'Saving…';

  @override
  String get managedNpcProfileEditRetry => 'Retry';

  @override
  String get managedNpcProfileEditLoadFailed =>
      'NPC details and verified archetypes could not be loaded. No files were changed.';

  @override
  String get managedNpcProfileEditCatalogChanged =>
      'The verified archetypes changed while this editor was open. Review and choose the archetype again before saving.';

  @override
  String get managedNpcProfileEditCurrentArchetypeUnavailable =>
      'The current NPC archetype is no longer represented exactly by this game catalog. No replacement was guessed.';

  @override
  String get managedNpcProfileEditStale =>
      'The project changed while this editor was open. Close it and reopen the NPC from the refreshed Story view.';

  @override
  String get managedNpcProfileEditRequiresReopen =>
      'The save result cannot be verified. Do not retry. Close this editor and reopen or recover the managed project.';

  @override
  String get managedNpcProfileEditSaveFailed =>
      'The NPC changes could not be saved safely. Nothing was built, deployed, or written into the game.';

  @override
  String get managedNpcProfileEditNameRequired => 'Enter a character name.';

  @override
  String get managedNpcProfileEditNameTooLong =>
      'The character name must be at most 256 UTF-8 bytes.';

  @override
  String get managedNpcProfileEditNameControl =>
      'The character name contains an unsupported control character.';

  @override
  String get managedNpcProfileEditReviewSelection =>
      'Review and choose an archetype before saving.';

  @override
  String get managedNpcProfileEditDiscardTitle => 'Discard NPC changes?';

  @override
  String get managedNpcProfileEditDiscardBody =>
      'Your unsaved name and archetype choice will be lost.';

  @override
  String get managedNpcProfileEditKeepEditing => 'Keep editing';

  @override
  String get managedNpcProfileEditDiscard => 'Discard';

  @override
  String managedNpcProfileEditSaved(String name, int revision) {
    return '$name was saved in project revision $revision. It remains an offline, build-blocked draft.';
  }

  @override
  String get managedVoiceBuildReadinessTitle => 'Voice readiness';

  @override
  String get managedVoiceBuildReadinessRefresh => 'Refresh Voice readiness';

  @override
  String get managedVoiceBuildReadinessChecking =>
      'Checking exact Voice readiness';

  @override
  String get managedVoiceBuildReadinessLoadError =>
      'Voice readiness could not be verified for the current project. No build is available from this result.';

  @override
  String get managedVoiceBuildReadinessReadyTitle => 'Voice is ready';

  @override
  String get managedVoiceBuildReadinessBlockedTitle => 'Voice needs attention';

  @override
  String managedVoiceBuildReadinessCount(int readySlots, int totalSlots) {
    return '$readySlots of $totalSlots Voice slots are ready.';
  }

  @override
  String get managedVoiceBuildReadinessBlockedBoundary =>
      'No bundle was created and deployment was not performed.';

  @override
  String get managedVoiceBuildReadinessBuildBundle => 'Build bundle';

  @override
  String get managedVoiceBuildReadinessBuildReleaseGuidance =>
      'Voice content is ready. Open Build & Release to create the offline bundle.';

  @override
  String get managedVoiceBuildReadinessConfigureGameGuidance =>
      'Voice content is ready. Configure the game installation before creating an offline bundle.';

  @override
  String get managedVoiceBuildReadinessHideBlockers => 'Hide blockers';

  @override
  String managedVoiceBuildReadinessShowBlockers(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: 'Show $count blockers',
      one: 'Show 1 blocker',
    );
    return '$_temp0';
  }

  @override
  String get managedVoiceBuildReadinessWorkflowFailed =>
      'The selected Voice workflow could not be opened. Refresh and try again.';

  @override
  String get managedVoiceBuildReadinessBuildWorkflowFailed =>
      'The Voice build workflow could not be opened.';

  @override
  String managedVoiceBuildReadinessExactRevision(int revision) {
    return 'Exact project revision $revision';
  }

  @override
  String get managedVoiceBuildReadinessResolveTarget => 'Resolve target';

  @override
  String get managedVoiceBuildReadinessManageTakes => 'Manage takes';

  @override
  String get managedVoiceBuildBlockerNoSlots =>
      'No Voice setups exist in this project.';

  @override
  String get managedVoiceBuildBlockerPayloadBudget =>
      'The selected Voice recordings exceed the safe bundle memory budget.';

  @override
  String get managedVoiceBuildBlockerUnresolvedTarget =>
      'Resolve this Voice target.';

  @override
  String get managedVoiceBuildBlockerAmbiguousTarget =>
      'This Voice target is ambiguous.';

  @override
  String get managedVoiceBuildBlockerUnqualifiedAdd =>
      'This target is not a sealed existing-member replacement.';

  @override
  String get managedVoiceBuildBlockerMissingTake =>
      'Select an approved Voice take.';

  @override
  String get managedVoiceBuildBlockerTakeNotApproved =>
      'The selected Voice take is not approved.';

  @override
  String get managedVoiceBuildBlockerCodecUnqualified =>
      'The selected Voice take uses an unsupported codec.';

  @override
  String get managedVoiceBuildBlockerSlotLimit =>
      'This project exceeds the 1024-slot Voice bundle limit.';

  @override
  String get managedVoiceBuildOfflineNotice =>
      'Offline build only. This creates a sealed existing-member Voice bundle. It does not deploy or write to the game.';

  @override
  String get managedVoiceBuildNewFolderName => 'New folder name';

  @override
  String get managedVoiceBuildNewFolderHelp =>
      'The bundle must be written to a brand-new child folder.';

  @override
  String get managedVoiceBuildChooseParent => 'Choose parent folder';

  @override
  String get managedVoiceBuildNoParentSelected => 'No parent folder selected';

  @override
  String get managedVoiceBuildNewOutput => 'New output';

  @override
  String get managedVoiceBuildOfflineBundle => 'Build offline bundle';

  @override
  String get managedVoiceBuildParentInspectFailed =>
      'The parent folder could not be inspected safely. No build or deployment was attempted.';

  @override
  String get managedVoiceBuildChooseExistingParent =>
      'Choose an existing parent folder.';

  @override
  String get managedVoiceBuildTargetSymlink =>
      'The target path is a symlink. Choose a different new folder name.';

  @override
  String get managedVoiceBuildTargetExists =>
      'The target already exists. Choose a different new folder name.';

  @override
  String get managedVoiceBuildRequiresReopen =>
      'This project can no longer be verified as current. Close this window and reopen the managed project before building another Voice bundle.';

  @override
  String get managedVoiceBuildStaleCheckpoint =>
      'The managed project changed while this window was open. Close this build window and open it again from the current project.';

  @override
  String get managedVoiceBuildFailed =>
      'The Voice bundle could not be built exactly. No deployment was attempted. Before retrying, choose a new folder name if output was created.';

  @override
  String get managedVoiceBuildPlanFailed =>
      'Voice readiness could not be verified for the exact current project. Output selection and build are unavailable until verification succeeds.';

  @override
  String get managedVoiceBuildParentAbsolute =>
      'Choose an absolute existing parent folder.';

  @override
  String get managedVoiceBuildParentSymlink =>
      'The selected parent is a symlink. Choose a real existing folder.';

  @override
  String get managedVoiceBuildFolderRequired => 'Enter a new folder name.';

  @override
  String get managedVoiceBuildFolderWhitespace =>
      'The folder name cannot start or end with whitespace.';

  @override
  String get managedVoiceBuildFolderTooLong => 'The folder name is too long.';

  @override
  String get managedVoiceBuildFolderPortable =>
      'Use one portable folder name without separators or reserved characters.';

  @override
  String get managedVoiceBuildFolderWindowsReserved =>
      'That folder name is reserved by Windows.';

  @override
  String get managedVoiceBuildExecutableUnavailable =>
      'The installed game executable could not be read. Finish any game update and check the configured installation before trying again. No deployment was attempted.';

  @override
  String get managedVoiceBuildExecutableMismatch =>
      'The installed game executable no longer matches this project generation. Re-import or retarget the managed project before building again. No deployment was attempted.';

  @override
  String get managedVoiceBuildGameUnavailable =>
      'The configured Gothic 1 Remake installation is unavailable. Check it in Settings before trying again. No deployment was attempted.';

  @override
  String get managedVoiceBuildStoreGameAlias =>
      'This project folder overlaps the configured game installation. Move the project outside the game folder before building. No deployment was attempted.';

  @override
  String get managedVoiceBuildGameOutputAlias =>
      'The bundle output overlaps a Gothic 1 Remake installation. Choose a parent folder outside every game installation. No deployment was attempted.';

  @override
  String get managedVoiceBuildStoreOutputAlias =>
      'The bundle output overlaps the managed project. Choose a parent folder outside the project. No deployment was attempted.';

  @override
  String get managedVoiceBuildOutputUnavailable =>
      'The selected output parent is unavailable or cannot be traversed safely. Choose a real existing parent folder outside the project and game.';

  @override
  String get managedVoiceBuildOutputFailed =>
      'The new bundle folder could not be written completely. Do not use any output left there; choose a different new folder name before retrying. No deployment was attempted.';

  @override
  String get managedVoiceBuildPromotionFailed =>
      'The sealed bundle could not be promoted into the requested new output folder. A conflicting output was left untouched and owned staging was removed. Choose a different new folder name before retrying. No deployment was attempted.';

  @override
  String get managedVoiceBuildCleanupFailed =>
      'The Voice bundle was not published, but its temporary staging folder could not be removed completely. Remove the reported staging folder before retrying. No deployment was attempted.';

  @override
  String get managedVoiceBuildPublicationUnconfirmed =>
      'The atomic publication may have succeeded, but its final identity or durability could not be confirmed. Do not retry, replace, or delete that exact output yet. Close this window and inspect the reported folder before deciding how to proceed. No deployment was attempted.';

  @override
  String get managedVoiceBuildStoreRootChanged =>
      'The managed project root changed while the bundle was being built. Close this window and reopen the project before building again. No deployment was attempted.';

  @override
  String get managedVoiceBuildGameRootChanged =>
      'The game installation changed while the bundle was being built. Finish the update or file operation, then retry with a new folder name. No deployment was attempted.';

  @override
  String get managedVoiceBuildOutputRootChanged =>
      'The output parent changed while the bundle was being built. Finish the file operation, verify the parent, then retry with a new folder name. No deployment was attempted.';

  @override
  String get managedVoiceBuildVerifyFailed =>
      'The written bundle could not be verified exactly. Do not use that output; choose a different new folder name before retrying. No deployment was attempted.';

  @override
  String get managedVoiceBuildBundleInvalid =>
      'The selected Voice content could not be lowered into one exact sealed bundle. Reopen the project, review its Voice slots, and try again. No deployment was attempted.';

  @override
  String get managedVoiceBuildInputInvalid =>
      'The Voice build request or output path exceeds the safe supported limits. Choose a shorter new output path and try again. No deployment was attempted.';

  @override
  String get managedVoiceBuildResponseLimit =>
      'The bundle was too large to return an exact build receipt. Do not use any unreceipted output; choose a new folder only after reducing the Voice build. No deployment was attempted.';

  @override
  String get managedVoiceBuildBuiltTitle => 'Sealed Voice bundle built';

  @override
  String get managedVoiceBuildOfflineReceipt =>
      'Offline receipt only. Deployment was not performed.';

  @override
  String get managedVoiceBuildBasisRevision => 'Basis project revision';

  @override
  String get managedVoiceBuildOutputLabel => 'Output';

  @override
  String get managedVoiceBuildArchiveEdits => 'Archive edits';

  @override
  String get managedVoiceBuildBundleFiles => 'Bundle files';

  @override
  String get managedVoiceBuildSealedBytes => 'Sealed bytes';

  @override
  String get managedVoiceBuildBundleSha256 => 'Bundle SHA-256';

  @override
  String get managedVoiceBuildParentPickerTitle => 'Choose Voice bundle parent';

  @override
  String managedVoiceBuildBuiltMessage(String output) {
    return 'Sealed Voice bundle built at $output. Deployment was not performed.';
  }

  @override
  String managedVoiceBuildBlockedMessage(int count) {
    return 'Voice build blocked by $count exact requirements. No bundle was created or deployed.';
  }

  @override
  String get managedTextureSetupTitle => 'Choose the game installation';

  @override
  String get managedTextureSetupDescription =>
      'Textures are read from the configured Gothic 1 Remake installation. Nothing is changed in the game or project.';

  @override
  String get managedTextureSetupAction => 'Open Settings';

  @override
  String get managedTextureLoading => 'Loading the installed texture catalog…';

  @override
  String get managedTextureLoadingDescription =>
      'The first exact scan can take several minutes. Mod Studio runs only one scan at a time and queues the latest refresh.';

  @override
  String managedTextureCatalogCount(int count) {
    return '$count installed textures';
  }

  @override
  String managedTextureSearchCount(int matches, int total) {
    return '$matches matches · $total total';
  }

  @override
  String get managedTextureEmptyTitle => 'No textures found';

  @override
  String get managedTextureEmptyDescription =>
      'The exact installed catalog contains no texture entries.';

  @override
  String get managedTextureErrorTitle => 'Texture catalog unavailable';

  @override
  String get managedTextureErrorDescription =>
      'The installed texture catalog could not be loaded for this exact game build.';

  @override
  String get managedTextureRetry => 'Retry';

  @override
  String get managedTextureRefreshTooltip =>
      'Refresh installed texture catalog';

  @override
  String get managedTextureSearchLabel => 'Search textures';

  @override
  String get managedTextureSearchHint => 'Name or Unreal asset path';

  @override
  String get managedTextureClearSearchTooltip => 'Clear texture search';

  @override
  String get managedTextureSelectPrompt =>
      'Select a texture to inspect its original installed image.';

  @override
  String get managedTexturePreviewLoading => 'Extracting the original texture…';

  @override
  String get managedTexturePreviewErrorTitle => 'Preview unavailable';

  @override
  String get managedTexturePreviewErrorDescription =>
      'The original texture could not be extracted from the selected game build.';

  @override
  String get managedTexturePreviewRetry => 'Retry preview';

  @override
  String get managedTextureBackToCatalog => 'Back to textures';

  @override
  String get managedTextureInspectionOnly =>
      'Installed reference · inspect only. This does not edit the project, game installation, or a save.';

  @override
  String get managedTextureInstalledBadge => 'Installed source';

  @override
  String get managedTextureRegularBadge => 'Regular texture';

  @override
  String get managedTextureVirtualBadge => 'Virtual texture';

  @override
  String managedTextureVirtualLayerCount(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count VT layers',
      one: '1 VT layer',
    );
    return '$_temp0';
  }

  @override
  String get managedTextureMipmappedBadge => 'Mipmapped';

  @override
  String get managedTextureSingleMipBadge => 'Single mip';

  @override
  String get managedTextureReplaceableBadge =>
      'Replacement supported · editing not yet available';

  @override
  String get managedTextureNotReplaceableBadge =>
      'Replacement unavailable · inspect only';

  @override
  String get managedTextureUnknownReplaceabilityBadge =>
      'Replacement not qualified · inspect only';

  @override
  String get managedTextureUnknownFormat => 'Unknown source format';

  @override
  String get managedWorkspaceTextVoiceLabel => 'Texto e vozes';

  @override
  String get managedWorkspaceTestReleaseLabel => 'Testar e publicar';

  @override
  String get managedTestReleaseTitle => 'Testar e publicar';

  @override
  String get managedTestReleaseDescription =>
      'Verifique todas as partes do mod antes de criar ficheiros jogáveis ou de os instalar.';

  @override
  String get managedTestReleaseEvidenceBoundary =>
      'Nada é considerado pronto automaticamente. Um resultado verificado aplica-se apenas a esta versão exata guardada do projeto.';

  @override
  String get managedTestReleaseChecksHeading => 'Verificações do projeto';

  @override
  String get managedTestReleaseReleaseHeading => 'Resultado jogável';

  @override
  String get managedTestReleaseStatusNotChecked => 'Não verificado';

  @override
  String get managedTestReleaseStatusChecking => 'A verificar';

  @override
  String get managedTestReleaseStatusChecked => 'Verificado';

  @override
  String get managedTestReleaseStatusNeedsAttention => 'Requer atenção';

  @override
  String get managedTestReleaseStatusBlocked => 'Bloqueado';

  @override
  String get managedTestReleaseStatusNotAvailable => 'Não disponível';

  @override
  String get managedTestReleaseStatusAvailable => 'Disponível';

  @override
  String get managedTestReleaseEvidenceLabel => 'Evidência';

  @override
  String get managedTestReleaseStaleEvidenceDescription =>
      'Este resultado pertence a outra versão do projeto. Execute novamente a verificação.';

  @override
  String get managedTestReleaseActionNotConnectedDescription =>
      'Existe evidência, mas esta ação ainda não está ligada na área de trabalho atual.';

  @override
  String get managedTestReleaseProblemsHeading => 'Problemas a resolver';

  @override
  String get managedTestReleaseVoiceHeading =>
      'Verificação da compilação de vozes';

  @override
  String get managedTestReleaseProjectStructureTitle => 'Estrutura do projeto';

  @override
  String get managedTestReleaseProjectStructureDescription =>
      'Consulte abaixo a lista ativa de problemas para verificar as referências e a estrutura do projeto gerido.';

  @override
  String get managedTestReleaseProjectStructureAction => 'Rever problemas';

  @override
  String get managedTestReleaseScriptsTitle => 'Scripts';

  @override
  String get managedTestReleaseScriptsDescription =>
      'Execute uma vez o compilador do jogo para todos os scripts desta versão exata guardada do projeto. O resultado serve apenas como evidência da verificação; a saída é eliminada.';

  @override
  String get managedTestReleaseScriptsAction => 'Executar verificação';

  @override
  String get managedProjectCompilerRetryAction => 'Repetir verificação';

  @override
  String get managedProjectCompilerReviewAction =>
      'Ver resultado / verificar novamente';

  @override
  String get managedProjectCompilerDialogTitle => 'Verificar todos os scripts';

  @override
  String get managedProjectCompilerDialogIntroduction =>
      'Feche o Gothic 1 Remake antes de começar. O Mod Studio verifica temporariamente todos os scripts do projeto com o compilador do jogo, restaura a instalação e elimina toda a saída do compilador. Este resultado não pode criar ficheiros jogáveis nem instalar o mod.';

  @override
  String get managedProjectCompilerCloseAction => 'Fechar';

  @override
  String get managedProjectCompilerNoGame =>
      'Escolha a instalação do Gothic 1 Remake nas Definições antes de executar esta verificação.';

  @override
  String get managedProjectCompilerSafetyBlocked =>
      'A instalação do jogo não está pronta para uma verificação. Feche o jogo ou resolva o aviso de recuperação e tente novamente.';

  @override
  String get managedProjectCompilerCompiled =>
      'Todos os scripts do projeto passaram nesta versão exata guardada. A saída do compilador foi eliminada.';

  @override
  String get managedProjectCompilerEmpty =>
      'Esta versão guardada não tem scripts para compilar. O resultado vazio foi verificado com exatidão.';

  @override
  String get managedProjectCompilerRejected =>
      'O compilador encontrou problemas num ou mais scripts do projeto. Corrija as mensagens abaixo e tente novamente.';

  @override
  String get managedProjectCompilerPreflightBlocked =>
      'O compilador não iniciou. Feche o jogo, verifique a instalação configurada e tente novamente.';

  @override
  String get managedProjectCompilerDrifted =>
      'O projeto ou os dados do jogo mudaram, ou a verificação final deixou de ser exata. O resultado foi eliminado; execute novamente para a versão atual.';

  @override
  String get managedProjectCompilerRequiresReopen =>
      'Este projeto tem de ser fechado e reaberto antes de outra verificação exata.';

  @override
  String get managedProjectCompilerRecoveryRequired =>
      'Não foi possível comprovar a conclusão da limpeza da saída privada do compilador ou da restauração exata da instalação do jogo. Outras verificações do compilador e a instalação permanecem bloqueadas até que uma nova verificação de segurança seja concluída com sucesso.';

  @override
  String get managedProjectCompilerFailed =>
      'Não foi possível concluir ou validar a verificação. Nenhum resultado foi mantido; tente novamente quando a instalação estiver pronta.';

  @override
  String get managedProjectCompilerFailureDetails => 'Mensagem do compilador';

  @override
  String get managedProjectCompilerDiagnosticsHeading =>
      'Mensagens do compilador';

  @override
  String get managedProjectCompilerCaptureCaptured =>
      'Foram capturadas mensagens estruturadas do compilador.';

  @override
  String get managedProjectCompilerCaptureFallback =>
      'A ligação de diagnóstico não estava disponível, por isso foi usado o compilador normal do jogo como alternativa.';

  @override
  String get managedProjectCompilerCaptureInvalid =>
      'Não foi possível validar a captura das mensagens do compilador.';

  @override
  String get managedProjectCompilerCaptureUnavailable =>
      'A ligação de diagnóstico não estava disponível após a execução; não foi necessária uma segunda execução.';

  @override
  String get managedProjectCompilerCaptureExitUnconfirmed =>
      'O processo do compilador não confirmou que terminou.';

  @override
  String get managedProjectCompilerCaptureDisabled =>
      'Não estavam disponíveis mensagens estruturadas do compilador nesta execução.';

  @override
  String get managedProjectCompilerSeverityError => 'Erro';

  @override
  String get managedProjectCompilerSeverityWarning => 'Aviso';

  @override
  String get managedProjectCompilerSeverityNote => 'Nota';

  @override
  String get managedProjectCompilerFileLabel => 'Ficheiro';

  @override
  String get managedProjectCompilerLineLabel => 'Linha';

  @override
  String get managedProjectCompilerColumnLabel => 'Coluna';

  @override
  String get managedProjectCompilerOmittedDiagnostics =>
      'mensagens adicionais do compilador omitidas';

  @override
  String get managedTestReleaseVoiceTitle => 'Texto e vozes';

  @override
  String get managedTestReleaseVoiceDescription =>
      'Utilize abaixo a verificação da compilação de vozes para a versão atualmente guardada do projeto.';

  @override
  String get managedTestReleaseVoiceAction => 'Verificar vozes';

  @override
  String get managedTestReleaseDataAssetsTitle => 'DataAssets';

  @override
  String get managedTestReleaseDataAssetsDescription =>
      'Os DataAssets preparados aparecem nos Problemas, mas ainda não existe evidência de uma compilação completa do projeto.';

  @override
  String get managedTestReleaseDataAssetsAction => 'Rever DataAssets';

  @override
  String get managedTestReleasePlayableBuildTitle => 'Ficheiros jogáveis';

  @override
  String get managedTestReleasePlayableBuildDescription =>
      'Crie uma compilação jogável verificada a partir desta versão exata guardada do projeto.';

  @override
  String get managedTestReleasePlayableBuildBlockedReason =>
      'Ainda não existe evidência exata de uma compilação completa do projeto para esta versão guardada.';

  @override
  String get managedTestReleaseCreatePlayableFilesAction =>
      'Criar ficheiros jogáveis';

  @override
  String get managedTestReleaseDeploymentTitle => 'Instalação';

  @override
  String get managedTestReleaseDeploymentDescription =>
      'Instale no jogo configurado uma compilação jogável verificada com exatidão.';

  @override
  String get managedTestReleaseDeploymentBlockedReason =>
      'Ainda não existe evidência exata de uma compilação implementável para esta versão guardada do projeto.';

  @override
  String get managedTestReleaseInstallAction => 'Instalar';

  @override
  String managedProjectCommandBarCurrentSection(String section) {
    return 'Secção atual: $section';
  }

  @override
  String managedProjectCommandBarOrientationSemantics(
    String project,
    String section,
  ) {
    return 'Projeto $project. Secção atual: $section.';
  }

  @override
  String get managedProjectCommandBarUndoLabel => 'Desfazer';

  @override
  String get managedProjectCommandBarSearchLabel => 'Pesquisar';

  @override
  String get managedProjectCommandBarCreateLabel => 'Criar';

  @override
  String get managedProjectCommandBarProblemsLabel => 'Problemas';

  @override
  String get managedProjectCommandBarHistoryLabel => 'Histórico';

  @override
  String get managedProjectCommandBarSettingsLabel => 'Definições';

  @override
  String get managedProjectCommandBarMoreActionsTooltip =>
      'Mais ações do projeto';

  @override
  String get managedProjectCommandBarBusyLabel =>
      'A concluir a ação atual do projeto…';

  @override
  String get managedProjectCommandBarBusyDisabledReason =>
      'Aguarde que a ação atual do projeto termine.';
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
  String get projectNewManagedRevision3 => 'Novo projeto de mod…';

  @override
  String get projectCreateGamePathRequired =>
      'Defina o caminho do Gothic 1 Remake nas Configurações antes de criar um projeto de mod.';

  @override
  String get projectCreateDirectoryPickerTitle =>
      'Criar aqui o projeto de mod gerenciado';

  @override
  String projectManagedRevision3Created(String projectId) {
    return 'Projeto de mod $projectId criado';
  }

  @override
  String projectManagedRevision3CreateFailed(String error) {
    return 'Falha ao criar o projeto de mod: $error';
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
  String get managedWorkspaceSettingsExpertLabel =>
      'Configurações e modo especialista';

  @override
  String get managedSectionStoryDescription => 'NPCs, missões e diálogos.';

  @override
  String get managedSectionLocalizationVoiceDescription =>
      'Escreva e traduza os diálogos do projeto em um só lugar e continue depois com as vozes.';

  @override
  String get managedSectionSettingsExpertDescription =>
      'As configurações e o DataAsset Lab somente leitura estão disponíveis.';

  @override
  String get managedSettingsExpertDataAssetLabLabel => 'DataAsset Lab';

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
  String get managedProjectLandingTitle => 'Iniciar um projeto de mod';

  @override
  String get managedProjectLandingDescription =>
      'Crie um projeto, abra uma pasta de projeto existente ou restaure um backup.';

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

  @override
  String get managedWorkspaceTextVoiceLabel => 'Texto e vozes';

  @override
  String get managedWorkspaceTestReleaseLabel => 'Testar e publicar';

  @override
  String get managedTestReleaseTitle => 'Testar e publicar';

  @override
  String get managedTestReleaseDescription =>
      'Verifique todas as partes do mod antes de criar arquivos jogáveis ou instalá-los.';

  @override
  String get managedTestReleaseEvidenceBoundary =>
      'Nada é considerado pronto automaticamente. Um resultado verificado se aplica somente a esta versão exata salva do projeto.';

  @override
  String get managedTestReleaseChecksHeading => 'Verificações do projeto';

  @override
  String get managedTestReleaseReleaseHeading => 'Saída jogável';

  @override
  String get managedTestReleaseStatusNotChecked => 'Não verificado';

  @override
  String get managedTestReleaseStatusChecking => 'Verificando';

  @override
  String get managedTestReleaseStatusChecked => 'Verificado';

  @override
  String get managedTestReleaseStatusNeedsAttention => 'Requer atenção';

  @override
  String get managedTestReleaseStatusBlocked => 'Bloqueado';

  @override
  String get managedTestReleaseStatusNotAvailable => 'Não disponível';

  @override
  String get managedTestReleaseStatusAvailable => 'Disponível';

  @override
  String get managedTestReleaseEvidenceLabel => 'Evidência';

  @override
  String get managedTestReleaseStaleEvidenceDescription =>
      'Este resultado pertence a outra versão do projeto. Execute a verificação novamente.';

  @override
  String get managedTestReleaseActionNotConnectedDescription =>
      'Há evidência, mas esta ação ainda não está conectada no espaço de trabalho atual.';

  @override
  String get managedTestReleaseProblemsHeading => 'Problemas a resolver';

  @override
  String get managedTestReleaseVoiceHeading =>
      'Verificação da compilação de vozes';

  @override
  String get managedTestReleaseProjectStructureTitle => 'Estrutura do projeto';

  @override
  String get managedTestReleaseProjectStructureDescription =>
      'Confira abaixo a lista ativa de problemas para verificar as referências e a estrutura do projeto gerenciado.';

  @override
  String get managedTestReleaseProjectStructureAction => 'Revisar problemas';

  @override
  String get managedTestReleaseScriptsTitle => 'Scripts';

  @override
  String get managedTestReleaseScriptsDescription =>
      'Execute uma vez o compilador do jogo para todos os scripts desta versão exata salva do projeto. O resultado serve apenas como evidência da verificação; a saída é descartada.';

  @override
  String get managedTestReleaseScriptsAction => 'Executar verificação';

  @override
  String get managedProjectCompilerRetryAction => 'Repetir verificação';

  @override
  String get managedProjectCompilerReviewAction =>
      'Ver resultado / verificar novamente';

  @override
  String get managedProjectCompilerDialogTitle => 'Verificar todos os scripts';

  @override
  String get managedProjectCompilerDialogIntroduction =>
      'Feche o Gothic 1 Remake antes de começar. O Mod Studio verifica temporariamente todos os scripts do projeto com o compilador do jogo, restaura a instalação e descarta toda a saída do compilador. Esse resultado não pode criar arquivos jogáveis nem instalar o mod.';

  @override
  String get managedProjectCompilerCloseAction => 'Fechar';

  @override
  String get managedProjectCompilerNoGame =>
      'Escolha a instalação do Gothic 1 Remake nas Configurações antes de executar esta verificação.';

  @override
  String get managedProjectCompilerSafetyBlocked =>
      'A instalação do jogo não está pronta para uma verificação. Feche o jogo ou resolva o aviso de recuperação e tente novamente.';

  @override
  String get managedProjectCompilerCompiled =>
      'Todos os scripts do projeto foram aprovados nesta versão exata salva. A saída do compilador foi descartada.';

  @override
  String get managedProjectCompilerEmpty =>
      'Esta versão salva não tem scripts para compilar. O resultado vazio foi verificado com exatidão.';

  @override
  String get managedProjectCompilerRejected =>
      'O compilador encontrou problemas em um ou mais scripts do projeto. Corrija as mensagens abaixo e tente novamente.';

  @override
  String get managedProjectCompilerPreflightBlocked =>
      'O compilador não foi iniciado. Feche o jogo, verifique a instalação configurada e tente novamente.';

  @override
  String get managedProjectCompilerDrifted =>
      'O projeto ou os dados do jogo mudaram, ou a verificação final deixou de ser exata. O resultado foi descartado; execute novamente para a versão atual.';

  @override
  String get managedProjectCompilerRequiresReopen =>
      'Este projeto precisa ser fechado e reaberto antes de outra verificação exata.';

  @override
  String get managedProjectCompilerRecoveryRequired =>
      'Não foi possível confirmar que a limpeza da saída privada do compilador ou a restauração exata da instalação do jogo foi concluída. Novas verificações do compilador e a instalação permanecem bloqueadas até que uma nova verificação de segurança seja bem-sucedida.';

  @override
  String get managedProjectCompilerFailed =>
      'Não foi possível concluir ou validar a verificação. Nenhum resultado foi mantido; tente novamente quando a instalação estiver pronta.';

  @override
  String get managedProjectCompilerFailureDetails => 'Mensagem do compilador';

  @override
  String get managedProjectCompilerDiagnosticsHeading =>
      'Mensagens do compilador';

  @override
  String get managedProjectCompilerCaptureCaptured =>
      'Foram capturadas mensagens estruturadas do compilador.';

  @override
  String get managedProjectCompilerCaptureFallback =>
      'A conexão de diagnóstico não estava disponível, então foi usado o compilador normal do jogo como alternativa.';

  @override
  String get managedProjectCompilerCaptureInvalid =>
      'Não foi possível validar a captura das mensagens do compilador.';

  @override
  String get managedProjectCompilerCaptureUnavailable =>
      'A conexão de diagnóstico não estava disponível após a execução; não foi necessária uma segunda execução.';

  @override
  String get managedProjectCompilerCaptureExitUnconfirmed =>
      'O processo do compilador não confirmou que terminou.';

  @override
  String get managedProjectCompilerCaptureDisabled =>
      'Não havia mensagens estruturadas do compilador disponíveis nesta execução.';

  @override
  String get managedProjectCompilerSeverityError => 'Erro';

  @override
  String get managedProjectCompilerSeverityWarning => 'Aviso';

  @override
  String get managedProjectCompilerSeverityNote => 'Nota';

  @override
  String get managedProjectCompilerFileLabel => 'Arquivo';

  @override
  String get managedProjectCompilerLineLabel => 'Linha';

  @override
  String get managedProjectCompilerColumnLabel => 'Coluna';

  @override
  String get managedProjectCompilerOmittedDiagnostics =>
      'mensagens adicionais do compilador omitidas';

  @override
  String get managedTestReleaseVoiceTitle => 'Texto e vozes';

  @override
  String get managedTestReleaseVoiceDescription =>
      'Use abaixo a verificação da compilação de vozes para a versão atualmente salva do projeto.';

  @override
  String get managedTestReleaseVoiceAction => 'Verificar vozes';

  @override
  String get managedTestReleaseDataAssetsTitle => 'DataAssets';

  @override
  String get managedTestReleaseDataAssetsDescription =>
      'Os DataAssets preparados aparecem em Problemas, mas ainda não há evidência de uma compilação completa do projeto.';

  @override
  String get managedTestReleaseDataAssetsAction => 'Revisar DataAssets';

  @override
  String get managedTestReleasePlayableBuildTitle => 'Arquivos jogáveis';

  @override
  String get managedTestReleasePlayableBuildDescription =>
      'Crie uma compilação jogável verificada a partir desta versão exata salva do projeto.';

  @override
  String get managedTestReleasePlayableBuildBlockedReason =>
      'Ainda não há evidência exata de uma compilação completa do projeto para esta versão salva.';

  @override
  String get managedTestReleaseCreatePlayableFilesAction =>
      'Criar arquivos jogáveis';

  @override
  String get managedTestReleaseDeploymentTitle => 'Instalação';

  @override
  String get managedTestReleaseDeploymentDescription =>
      'Instale no jogo configurado uma compilação jogável verificada com exatidão.';

  @override
  String get managedTestReleaseDeploymentBlockedReason =>
      'Ainda não há evidência exata de uma compilação implantável para esta versão salva do projeto.';

  @override
  String get managedTestReleaseInstallAction => 'Instalar';

  @override
  String managedProjectCommandBarCurrentSection(String section) {
    return 'Seção atual: $section';
  }

  @override
  String managedProjectCommandBarOrientationSemantics(
    String project,
    String section,
  ) {
    return 'Projeto $project. Seção atual: $section.';
  }

  @override
  String get managedProjectCommandBarUndoLabel => 'Desfazer';

  @override
  String get managedProjectCommandBarSearchLabel => 'Pesquisar';

  @override
  String get managedProjectCommandBarCreateLabel => 'Criar';

  @override
  String get managedProjectCommandBarProblemsLabel => 'Problemas';

  @override
  String get managedProjectCommandBarHistoryLabel => 'Histórico';

  @override
  String get managedProjectCommandBarSettingsLabel => 'Configurações';

  @override
  String get managedProjectCommandBarMoreActionsTooltip =>
      'Mais ações do projeto';

  @override
  String get managedProjectCommandBarBusyLabel =>
      'Finalizando a ação atual do projeto…';

  @override
  String get managedProjectCommandBarBusyDisabledReason =>
      'Aguarde a conclusão da ação atual do projeto.';
}
