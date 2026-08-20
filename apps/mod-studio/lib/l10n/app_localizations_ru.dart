// ignore: unused_import
import 'package:intl/intl.dart' as intl;
import 'app_localizations.dart';

// ignore_for_file: type=lint

/// The translations for Russian (`ru`).
class AppLocalizationsRu extends AppLocalizations {
  AppLocalizationsRu([String locale = 'ru']) : super(locale);

  @override
  String get tabItems => 'Items';

  @override
  String get tabOverrides => 'Changes';

  @override
  String get tabSettings => 'Settings';

  @override
  String get tabDialogs => 'Диалоги';

  @override
  String get tabAudio => 'Аудио';

  @override
  String get tabTextures => 'Текстуры';

  @override
  String get tabScripts => 'Скрипты';

  @override
  String get changesAll => 'Все';

  @override
  String get sectionItemValues => 'Значения предметов';

  @override
  String get sectionLocalizedText => 'Локализованные тексты';

  @override
  String get audioCatCreatures => 'Существа';

  @override
  String get audioCatObjects => 'Объекты';

  @override
  String get audioCatMagic => 'Магия';

  @override
  String get audioCatMovement => 'Движение';

  @override
  String get audioCatWorld => 'Мир';

  @override
  String get audioCatAction => 'Действия';

  @override
  String get audioCatCombat => 'Бой';

  @override
  String get audioCatPhysics => 'Физика';

  @override
  String get audioCatItems => 'Предметы';

  @override
  String get audioCatUi => 'Интерфейс';

  @override
  String get audioCatFoley => 'Фоли';

  @override
  String get audioCatUnderwater => 'Под водой';

  @override
  String get audioCatVision => 'Видения';

  @override
  String get audioCatDialog => 'Диалог';

  @override
  String get audioCatOther => 'Прочее';

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
  String get extractLocalizedText => 'Извлечь локализованные тексты';

  @override
  String get lightMode => 'Светлая тема';

  @override
  String get darkMode => 'Тёмная тема';

  @override
  String get language => 'Язык';

  @override
  String get exportMod => 'Экспорт мода';

  @override
  String exportModWithCount(int count) {
    return 'Экспорт мода ($count)';
  }

  @override
  String get selectAnItemToEdit => 'Выберите предмет, чтобы изменить его поля.';

  @override
  String gameDataActiveTooltip(String name) {
    return 'Данные игры: $name';
  }

  @override
  String get gameDataBundledTooltip => 'Данные игры: встроенные';

  @override
  String get loadGameDataDump => 'Загрузить дамп данных игры…';

  @override
  String get loadGameDataDumpSubtitle =>
      'gore_game_data.json из мода gore-dump';

  @override
  String get useBundledData => 'Использовать встроенные данные';

  @override
  String get alreadyBundled => 'уже встроены';

  @override
  String get gameDataFileGroupLabel => 'данные игры';

  @override
  String get minimize => 'Свернуть';

  @override
  String get restore => 'Восстановить';

  @override
  String get maximize => 'Развернуть';

  @override
  String get close => 'Закрыть';

  @override
  String get about => 'О программе';

  @override
  String aboutVersion(String version, String sha) {
    return 'Version $version ($sha)';
  }

  @override
  String get aboutCopyright => '© 2026 Daniel Hoer';

  @override
  String get aboutLicense => 'Распространяется по лицензии MIT.';

  @override
  String get categoryMeleeWeapons => 'Оружие ближнего боя';

  @override
  String get categoryRangedWeapons => 'Оружие дальнего боя';

  @override
  String get categoryAmmunition => 'Боеприпасы';

  @override
  String get categoryRunes => 'Руны';

  @override
  String get categorySpellScrolls => 'Свитки заклинаний';

  @override
  String get categoryFoodAndPotions => 'Еда и зелья';

  @override
  String get categoryMiscellaneous => 'Разное';

  @override
  String get categoryAmulets => 'Амулеты';

  @override
  String get categoryRings => 'Кольца';

  @override
  String get categoryAnimalTrophies => 'Трофеи животных';

  @override
  String get categoryWritings => 'Письмена';

  @override
  String get categoryMissionItems => 'Квестовые предметы';

  @override
  String get categoryKeys => 'Ключи';

  @override
  String get categoryOther => 'Прочее';

  @override
  String categoryWithCount(String label, int count) {
    return '$label ($count)';
  }

  @override
  String get searchItems => 'Поиск предметов';

  @override
  String get noItemsMatch => 'Нет подходящих предметов';

  @override
  String failedToLoadCatalog(String error) {
    return 'Не удалось загрузить каталог: $error';
  }

  @override
  String pendingOverridesWithCount(int count) {
    return 'Ожидающие изменения ($count)';
  }

  @override
  String get clearAll => 'Очистить всё';

  @override
  String get noPendingOverrides =>
      'Нет ожидающих изменений.\nИзмените поля предметов, чтобы добавить.';

  @override
  String get removeOverride => 'Удалить изменение';

  @override
  String get searchChanges => 'Поиск изменений';

  @override
  String get noChangesMatch => 'Нет подходящих изменений';

  @override
  String get clearSection => 'Очистить эту группу';

  @override
  String get modName => 'Название мода';

  @override
  String get loadDelayLabel => 'Задержка загрузки (мс, 0 = мгновенно)';

  @override
  String get noFolderSelected => 'Папка не выбрана';

  @override
  String get chooseFolder => 'Выбрать папку';

  @override
  String get packageAsZip => 'Упаковать в .zip';

  @override
  String get cancel => 'Отмена';

  @override
  String get export => 'Экспорт';

  @override
  String get exportHere => 'Экспортировать сюда';

  @override
  String get mustBeNonNegativeInteger =>
      'Должно быть неотрицательным целым числом';

  @override
  String get extractingLocalizedText =>
      'Извлечение локализованных текстов игры…';

  @override
  String get localizedTextExtractionCancelled =>
      'Извлечение локализованных текстов отменено.';

  @override
  String get localizedTextExtracted => 'Локализованные тексты извлечены.';

  @override
  String get extractionFailed => 'Не удалось выполнить извлечение.';

  @override
  String get localizationCacheFileGroupLabel => 'кэш локализации';

  @override
  String get extractLocalizedTextQuestion =>
      'Извлечь локализованные тексты игры?';

  @override
  String get extractLocalizedTextBody =>
      'Локализованные тексты игры ещё не извлечены. Извлечь их сейчас из установленной игры? (необязательно)';

  @override
  String get notNow => 'Не сейчас';

  @override
  String get extract => 'Извлечь';

  @override
  String get validationRequired => 'Обязательно';

  @override
  String get validationMustBeWholeNumber => 'Должно быть целым числом';

  @override
  String get validationMustBeNumber => 'Должно быть числом';

  @override
  String get validationMustBeFinite => 'Должно быть конечным числом';

  @override
  String validationMustBeAtLeast(String min) {
    return 'Должно быть ≥ $min';
  }

  @override
  String validationMustBeAtMost(String max) {
    return 'Должно быть ≤ $max';
  }

  @override
  String get validationMustBeBool => 'Должно быть true или false';

  @override
  String validationMustBeOneOf(String options) {
    return 'Должно быть одним из: $options';
  }

  @override
  String get modNameRequired => 'Обязательно';

  @override
  String get modNameControlCharacters =>
      'Не должно содержать управляющих символов';

  @override
  String get modNamePathSeparators => 'Не должно содержать разделителей пути';

  @override
  String get modNameNotAFolderName => 'Недопустимое имя папки';

  @override
  String localizedTextExtractedCount(int idCount, int languageCount) {
    return 'Извлечено $idCount идентификаторов на $languageCount языках';
  }

  @override
  String get managerDeployActive =>
      'Активен loadout mod-manager. Сначала выполните undeploy в gore-manager.';

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
      'Новый проект открыт, но не удалось полностью очистить сеанс предыдущего проекта. Повторная очистка выполняться не будет. Перезапустите Mod Studio, прежде чем снова открывать предыдущий проект.';

  @override
  String get projectNewManagedRevision3 => 'Новый проект мода…';

  @override
  String get projectCreateGamePathRequired =>
      'Перед созданием проекта мода укажите путь к Gothic 1 Remake в настройках.';

  @override
  String get projectCreateDirectoryPickerTitle =>
      'Создать здесь управляемый проект мода';

  @override
  String projectManagedRevision3Created(String projectId) {
    return 'Проект мода $projectId создан';
  }

  @override
  String projectManagedRevision3CreateFailed(String error) {
    return 'Не удалось создать проект мода: $error';
  }

  @override
  String get projectCreateDialogTitle => 'Создать проект мода';

  @override
  String get projectCreateNameLabel => 'Название проекта';

  @override
  String get projectCreateNameHelper => 'Название, отображаемое в Mod Studio.';

  @override
  String get projectCreateVersionLabel => 'Версия';

  @override
  String get projectCreateVersionHelper => 'Начальная версия, например 0.1.0.';

  @override
  String get projectCreateAuthorLabel => 'Автор';

  @override
  String get projectCreateAuthorHelper =>
      'Ваше имя или название команды моддеров.';

  @override
  String get projectCreateLocalesLabel => 'Языки редактирования';

  @override
  String get projectCreateLocalesHelper =>
      'Канонические теги через запятую, например: en, de, en-US.';

  @override
  String get projectCreateBoundary =>
      'Будет создан пустой управляемый офлайн-проект. Мод не компилируется, не устанавливается и не запускается; файлы игры и сохранений не изменяются.';

  @override
  String get projectCreateSubmit => 'Создать проект';

  @override
  String projectCreateMetadataRequired(String label) {
    return 'Поле «$label» обязательно.';
  }

  @override
  String projectCreateMetadataNoOuterWhitespace(String label) {
    return 'Поле «$label» не может начинаться или заканчиваться пробелом.';
  }

  @override
  String projectCreateMetadataControlCharacters(String label) {
    return 'Поле «$label» не может содержать управляющие символы.';
  }

  @override
  String projectCreateMetadataMalformed(String label) {
    return 'Поле «$label» содержит некорректный текст.';
  }

  @override
  String projectCreateMetadataTooLong(String label, int maxBytes) {
    return 'Поле «$label» превышает ограничение UTF-8 в $maxBytes байт.';
  }

  @override
  String get projectCreateLocalesRequired =>
      'Укажите хотя бы один язык редактирования.';

  @override
  String get projectCreateLocalesEmptyEntry => 'Удалите пустую запись языка.';

  @override
  String projectCreateLocalesTooMany(int maxLocales) {
    return 'Используйте не более $maxLocales языков редактирования.';
  }

  @override
  String projectCreateLocaleBoundedAscii(String locale) {
    return 'Локаль «$locale» должна быть ограниченной строкой ASCII.';
  }

  @override
  String projectCreateLocaleLanguage(String locale) {
    return 'В локали «$locale» язык должен состоять из 2–8 строчных букв.';
  }

  @override
  String projectCreateLocaleInvalidSegment(String locale) {
    return 'Локаль «$locale» содержит недопустимый сегмент.';
  }

  @override
  String projectCreateLocaleNotCanonical(String locale, String canonical) {
    return 'Локаль «$locale» не каноническая; используйте «$canonical».';
  }

  @override
  String get managedWorkspaceOverviewLabel => 'Обзор';

  @override
  String get managedWorkspaceContentLabel => 'Содержимое';

  @override
  String get managedWorkspaceDataAssetsLabel => 'DataAssets';

  @override
  String get managedContentWorkspaceLibraryLabel => 'Этот мод';

  @override
  String get managedWorkspaceHomeLabel => 'Главная';

  @override
  String get managedWorkspaceStoryLabel => 'Сюжет';

  @override
  String get managedWorkspaceSettingsExpertLabel =>
      'Настройки и экспертный режим';

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
  String get managedSectionStoryDescription => 'NPC, задания и диалоги.';

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
      'This dialog has no local rollback. After removal, Project History or global Undo can restore an earlier version while it remains available.';

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
      'Пишите и переводите диалоги проекта в одном месте, а затем переходите к озвучке.';

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
      'This project has unsaved edits. Switching now would discard them.';

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
      'Доступны настройки и DataAsset Lab в режиме только для чтения.';

  @override
  String get managedSettingsExpertDataAssetLabLabel => 'DataAsset Lab';

  @override
  String get managedSectionStatusHeading => 'Состояние';

  @override
  String get managedSectionActionsHeading => 'Действия';

  @override
  String get managedCapabilityAvailable => 'Доступно';

  @override
  String get managedCapabilityPartial => 'Частично';

  @override
  String get managedCapabilityPlanned => 'Запланировано';

  @override
  String get managedCapabilityUnavailable => 'Недоступно';

  @override
  String get managedProjectSubtitle =>
      'Рабочая область для автономного редактирования, точно соответствующая текущей версии';

  @override
  String get managedProjectLandingTitle => 'Начать проект мода';

  @override
  String get managedProjectLandingDescription =>
      'Создайте проект, откройте существующую папку проекта или восстановите резервную копию.';

  @override
  String get managedProjectTechnicalDetails => 'Технические сведения о проекте';

  @override
  String get managedProjectRecoveryContentLocked =>
      'Снова откройте управляемый проект, прежде чем читать его содержимое.';

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
  String get managedDashboardUntitledProject => 'Проект без названия';

  @override
  String get managedDashboardDraftStatus => 'Черновик';

  @override
  String get managedDashboardProjectVersion => 'Версия';

  @override
  String get managedDashboardProjectAuthor => 'Автор';

  @override
  String get managedDashboardNotProvided => 'Не указано';

  @override
  String get managedDashboardContentCounts => 'Содержимое проекта';

  @override
  String get managedDashboardChangesDescription =>
      'Everything currently saved in this exact project, grouped by what you can work on. Generated helpers stay attached only when their relationship is proven.';

  @override
  String get managedDashboardNpcDrafts => 'Черновики NPC';

  @override
  String get managedDashboardQuestDrafts => 'Черновики заданий';

  @override
  String get managedDashboardDialogLines => 'Реплики диалогов';

  @override
  String get managedDashboardVoiceTakes => 'Записи озвучки';

  @override
  String get managedDashboardAssets => 'Ресурсы';

  @override
  String get managedDashboardItemPatches => 'Items';

  @override
  String get managedDashboardLocalizationEntries => 'Project text';

  @override
  String get managedDashboardVoiceSlots => 'Voice target';

  @override
  String get managedDashboardGeneratedScripts => 'Generated script';

  @override
  String get managedDashboardSelectedVoiceTake => 'Selected take';

  @override
  String get managedDashboardTechnicalContent => 'Technical content';

  @override
  String get managedDashboardTechnicalContentDescription =>
      'Generated or problematic helpers that cannot be safely assigned to an author-facing change.';

  @override
  String get managedDashboardEmptyChangesTitle => 'No changes yet';

  @override
  String get managedDashboardEmptyChangesDescription =>
      'Use Create, Content, or Story to add the first project change. Nothing has been written to the game or a save.';

  @override
  String get managedDashboardOpenChange => 'Open this exact project change';

  @override
  String get managedDashboardChangeActionFailed =>
      'This project change is no longer current. Reload the project overview and try again.';

  @override
  String get managedDashboardUnresolvedReferences => 'Неразрешённые ссылки';

  @override
  String get managedDashboardReadiness => 'Что уже работает';

  @override
  String get managedDashboardOfflineAuthoringTitle =>
      'Автономное редактирование доступно';

  @override
  String get managedDashboardOfflineAuthoringDescription =>
      'Создавайте и редактируйте поддерживаемое содержимое проекта, не изменяя установку игры и файлы сохранений.';

  @override
  String get managedDashboardGeneralBuildBlockedTitle =>
      'Общая сборка мода недоступна';

  @override
  String get managedDashboardGeneralBuildBlockedDescription =>
      'Можно собирать только запечатанные автономные пакеты Voice; полную играбельную модификацию пока собрать нельзя.';

  @override
  String get managedDashboardRuntimeUnqualifiedTitle =>
      'Работа в игре ещё не проверена';

  @override
  String get managedDashboardRuntimeUnqualifiedDescription =>
      'Mod Studio пока не подтвердил работу этого содержимого проекта в запущенной игре.';

  @override
  String get managedDashboardReferenceIntegrityTitle => 'Целостность ссылок';

  @override
  String get managedDashboardReferenceIntegrityDescription =>
      'Это число проверяет только ссылки проекта; оно не означает готовность к сборке или запуску.';

  @override
  String get managedDashboardMissingGameTitle => 'Требуется настроить игру';

  @override
  String get managedDashboardMissingGameDescription =>
      'Укажите установку Gothic 1 Remake в настройках перед использованием действий, которым нужны подтверждённые данные установленной игры.';

  @override
  String get managedDashboardCreateHeading => 'Создать';

  @override
  String get managedDashboardToolsHeading => 'Инструменты проекта';

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
  String get managedDashboardLoading => 'Загрузка обзора проекта';

  @override
  String get managedDashboardLoadError => 'Обзор проекта недоступен';

  @override
  String get managedDashboardLoadErrorDescription =>
      'Не удалось загрузить проверенный обзор проекта. Содержимое проекта не изменено.';

  @override
  String get managedDashboardRetry => 'Повторить';

  @override
  String get managedActionNewNpcTitle => 'Новый NPC';

  @override
  String get managedActionNewNpcDescription =>
      'Создать ограниченный автономный черновик NPC на основе подтверждённых данных установленной игры.';

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
  String get managedActionNewQuestTitle => 'Новое задание';

  @override
  String get managedActionNewQuestDescription =>
      'Создать автономный черновик задания с целями и проверенными родительскими идентификаторами.';

  @override
  String get managedQuestOpeningRecipeTitle => 'Задание + первая реплика';

  @override
  String get managedQuestOpeningRecipeDescription =>
      'Рекомендуется: создайте черновик задания, затем напишите и вставьте первую локализованную реплику. Этот процесс использует две контрольные точки проекта и не создаёт доступный в игре диалог.';

  @override
  String get managedQuestOpeningRecipeIntroduction =>
      'Этот пошаговый процесс сначала сохраняет задание, а затем открывает его первую реплику. Если остановиться после шага 1, задание останется сохранённым. Процесс не создаёт доступный в игре диалог и не изменяет игру или файлы сохранений.';

  @override
  String get managedQuestOpeningRecipeStart =>
      'Начать пошаговое создание задания';

  @override
  String get managedQuestOpeningLineTitle =>
      'Шаг 2 из 2: первая реплика диалога';

  @override
  String get managedQuestOpeningLineIntroduction =>
      'Напишите первую локализованную реплику этого задания. При сохранении будут созданы реплика и её текст, а затем реплика будет вставлена в начало расшифровки задания.';

  @override
  String managedQuestOpeningRecipePreparing(int projectRevision) {
    return 'Задание сохранено в ревизии проекта $projectRevision. Подготовка первой реплики...';
  }

  @override
  String managedQuestOpeningRecipePartial(int projectRevision) {
    return 'Задание сохранено в ревизии проекта $projectRevision; первая реплика не добавлена. Продолжите в разделе «Сюжет > Диалоги и озвучка».';
  }

  @override
  String get managedQuestOpeningRecipeFailed =>
      'Не удалось начать пошаговое создание задания. Изменения проекта не были опубликованы.';

  @override
  String get managedQuestOpeningRecipeStopped =>
      'Пошаговый процесс остановлен, потому что точное текущее состояние проекта изменилось. Дальнейшие шаги не будут выполнены автоматически; проверьте раздел «Сюжет» и продолжите вручную.';

  @override
  String get managedQuestOpeningRecipeRequiresReopen =>
      'Не удалось безопасно продолжить пошаговый процесс. Снова откройте этот проект и проверьте раздел «Сюжет», прежде чем повторить попытку или продолжить вручную.';

  @override
  String managedQuestOpeningRecipeComplete(int projectRevision) {
    return 'Задание и первая реплика сохранены в ревизии проекта $projectRevision. Только черновик: доступный в игре диалог не создан, игра и файлы сохранений не изменены.';
  }

  @override
  String get managedActionNewDialogLineTitle => 'Добавить реплику диалога';

  @override
  String get managedActionNewDialogLineDescription =>
      'Напишите локализованный текст проекта или привяжите неиспользуемый текст из этого проекта. Это не создаёт доступный в игре диалог.';

  @override
  String managedActionNewDialogLineSaved(int projectRevision) {
    return 'Реплика диалога сохранена в ревизии проекта $projectRevision. Игра и файлы сохранений не изменены.';
  }

  @override
  String get managedDialogLineIntroduction =>
      'Напишите новую локализованную реплику диалога или привяжите текст, уже принадлежащий этому проекту.';

  @override
  String get managedDialogLineBoundary =>
      'Изменяются только файлы проекта. Это не создаёт тему AngelScript или доступный в игре диалог и никогда не изменяет установку игры или файлы сохранений. Поле говорящего — только метка; оно не связывает реплику с NPC.';

  @override
  String get managedDialogLineCreateMode => 'Написать новый текст';

  @override
  String get managedDialogLineReuseMode => 'Использовать текст проекта';

  @override
  String get managedDialogLineNameLabel => 'Название реплики';

  @override
  String get managedDialogLineNameHint => 'Приветствие у входа в шахту';

  @override
  String get managedDialogLineSpeakerLabel =>
      'Метка говорящего (необязательно)';

  @override
  String get managedDialogLineSpeakerHint => 'Например, Viper';

  @override
  String get managedDialogLineLocaleLabel => 'Язык';

  @override
  String get managedDialogLineTextLabel => 'Текст диалога';

  @override
  String get managedDialogLineReuseSearch =>
      'Поиск неиспользуемого текста проекта';

  @override
  String get managedDialogLineNoReusableText =>
      'Нет неиспользуемого и структурно корректного текста проекта, который можно привязать. Напишите новый текст.';

  @override
  String get managedDialogLineCreateSlotLabel =>
      'Подготовить этот язык для Voice';

  @override
  String get managedDialogLineCreateSlotHelp =>
      'Создаёт в проекте пустой неразрешённый слот Voice. Запись не добавляется и не развёртывается.';

  @override
  String get managedDialogLineCancel => 'Отмена';

  @override
  String get managedDialogLineSave => 'Сохранить в проекте';

  @override
  String get managedDialogLineSaving => 'Сохранение…';

  @override
  String get managedDialogLineLoading => 'Чтение точного содержимого проекта…';

  @override
  String get managedDialogLineLoadFailed =>
      'Не удалось прочитать точное текущее содержимое проекта. Ничего не изменено.';

  @override
  String get managedDialogLineRetry => 'Повторить';

  @override
  String get managedDialogLineStale =>
      'Проект изменился, пока это окно было открыто. Закройте его и повторите попытку из текущего проекта.';

  @override
  String get managedDialogLineRequiresReopen =>
      'Текущий проект больше нельзя безопасно проверить. Закройте это окно и заново откройте управляемый проект.';

  @override
  String get managedDialogLineInvalidInput =>
      'Проверьте выделенные данные проекта и выберите точный текущий вариант.';

  @override
  String get managedDialogLineSaveFailed =>
      'Не удалось безопасно сохранить реплику диалога. Игра и файлы сохранений не изменены.';

  @override
  String get managedDialogLineDone => 'Готово';

  @override
  String get managedDialogLineAddRecording => 'Добавить запись';

  @override
  String get managedActionAddVoiceTakeTitle => 'Добавить запись озвучки';

  @override
  String get managedActionAddVoiceTakeDescription =>
      'Импортировать запись Ogg Vorbis в этот проект без её развёртывания.';

  @override
  String get managedActionAddVoiceTakeRequiresDialogLine =>
      'Create or repair a dialog line with one valid localization entry before using Voice tools.';

  @override
  String get managedActionManageVoiceTakesTitle =>
      'Управление записями озвучки';

  @override
  String get managedActionManageVoiceTakesDescription =>
      'Просмотреть записи и выбрать одобренные варианты для слотов Voice.';

  @override
  String get managedActionResolveVoiceTargetTitle => 'Определить цель Voice';

  @override
  String get managedActionResolveVoiceTargetDescription =>
      'Сопоставить слоты Voice проекта с точными элементами установленных архивов, не изменяя игру.';

  @override
  String get managedActionBuildVoiceBundleTitle => 'Собрать пакет Voice';

  @override
  String get managedActionBuildVoiceBundleDescription =>
      'Собрать запечатанный автономный пакет из существующих элементов; развёртывание не выполняется.';

  @override
  String get managedActionDataAssetsTitle => 'Изменения DataAssets';

  @override
  String get managedActionDataAssetsDescription =>
      'Проверить установленные пакеты и подготовить в проекте проверенные изменения значений фиксированной ширины.';

  @override
  String get managedActionBrowseProjectContentDescription =>
      'Просматривайте точное содержимое проекта и связанные с ним разрешённые или неразрешённые ссылки.';

  @override
  String get managedActionSettingsTitle => 'Настройки';

  @override
  String get managedActionSettingsDescription =>
      'Настроить установку Gothic 1 Remake и параметры Mod Studio.';

  @override
  String projectStarterSetupOpenFailed(String projectId) {
    return 'Проект $projectId безопасно создан, но мастер начальной настройки не открылся. Текущим остаётся корректный пустой проект.';
  }

  @override
  String projectStarterOutcomeUnverified(String projectId) {
    return 'Проект $projectId создан, но Mod Studio не может проверить результат начальной настройки. Перед продолжением заново откройте управляемый проект; игра и сохранения не изменены.';
  }

  @override
  String projectStarterNpcCancelled(String projectId) {
    return 'Проект $projectId создан. Заготовка NPC не добавлена, поэтому текущим остаётся корректный пустой проект.';
  }

  @override
  String projectStarterNpcSaved(int projectRevision) {
    return 'Заготовка NPC сохранена в ревизии $projectRevision. Сборка по-прежнему заблокирована, работа в игре не подтверждена, NPC не создаётся.';
  }

  @override
  String projectStarterQuestCancelled(String projectId) {
    return 'Проект $projectId создан. Заготовка задания не добавлена, поэтому текущим остаётся корректный пустой проект.';
  }

  @override
  String projectStarterQuestSaved(int projectRevision) {
    return 'Заготовка задания сохранена в ревизии $projectRevision. Сборка по-прежнему заблокирована, работа в игре не подтверждена.';
  }

  @override
  String get projectStarterSemanticsLabel => 'Начало проекта';

  @override
  String get projectStarterPrompt => 'С чего вы хотите начать?';

  @override
  String get projectStarterWriteBoundary =>
      'Выбор варианта ничего не записывает. Проект создаётся только после отправки формы и выбора пустой папки.';

  @override
  String get projectStarterEmptyTitle => 'Пустой проект';

  @override
  String get projectStarterEmptyDescription =>
      'Создать только управляемый проект. Содержимое можно добавить позже.';

  @override
  String get projectStarterNpcDraftTitle => 'Черновик NPC';

  @override
  String get projectStarterNpcDraftDescription =>
      'Сначала создать пустой проект, затем открыть пошаговую настройку черновика NPC.';

  @override
  String get projectStarterQuestDraftTitle => 'Черновик задания';

  @override
  String get projectStarterQuestDraftDescription =>
      'Сначала создать пустой проект, затем открыть пошаговую настройку черновика задания.';

  @override
  String get projectStarterPartialOutcome =>
      'Отмена пошаговой настройки NPC или задания либо ошибка черновика оставляет корректный пустой проект. Выбор не записывает данные в игру или сохранение.';

  @override
  String get managedContentWorkspaceBrowseLabel => 'Обзор';

  @override
  String get managedContentWorkspaceVerifiedEditsLabel =>
      'Проверенные изменения';

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
  String get managedItemsCategoryArmor => 'Armor';

  @override
  String get managedItemsExactSchemaBadge => 'Exact project schema';

  @override
  String get managedItemsEditableBadge => 'Managed edit';

  @override
  String get managedItemsBuildPendingBadge => 'Build support pending';

  @override
  String get managedItemsInvalidNumber => 'Enter a valid number.';

  @override
  String managedItemsNumberOutsideNativeRange(String minimum, String maximum) {
    return 'Enter a value from $minimum to $maximum.';
  }

  @override
  String get managedItemsAuthoringBoundary =>
      'Changes are saved only to this managed project. This editor does not write to the game or a save. Item bundle build is not available yet.';

  @override
  String managedItemsCurrentChanges(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count changed fields',
      one: '1 changed field',
      zero: 'No item changes',
    );
    return '$_temp0';
  }

  @override
  String get managedItemsChangeField => 'Change this field';

  @override
  String get managedItemsUseGameDefault => 'Use game default';

  @override
  String get managedItemsSaveChanges => 'Save item changes';

  @override
  String get managedItemsRevertItem => 'Revert item to game defaults';

  @override
  String get managedItemsClearChanges => 'Clear all item changes';

  @override
  String get managedItemsNoUnsavedChanges => 'No unsaved changes.';

  @override
  String managedItemsSaved(int revision) {
    return 'Item changes saved in project revision $revision.';
  }

  @override
  String get managedItemsSaveStale =>
      'The project or item catalog changed. Nothing was saved. Reload the current item data before editing again.';

  @override
  String get managedItemsSaveRequiresReopen =>
      'The project checkpoint can no longer be verified safely. Nothing was saved. Use project recovery, or close and reopen the project.';

  @override
  String get managedItemsSaveNoChanges =>
      'There is no current item change to save. Reload the item data to continue.';

  @override
  String get managedItemsSaveUnsupported =>
      'This change no longer fits the current safe item schema. Nothing was saved. Reload the item data before continuing.';

  @override
  String get managedItemsSaveUnexpected =>
      'Item changes could not be saved safely. Nothing was changed. Reopen the project and try again.';

  @override
  String get managedItemsReloadDiscardDraft =>
      'Reload item data and discard this draft';

  @override
  String get managedItemsCatalogLoadTitle => 'Items are unavailable';

  @override
  String get managedItemsCatalogStale =>
      'The project or exact item catalog changed before the item data could be loaded. Nothing was changed.';

  @override
  String get managedItemsCatalogRequiresReopen =>
      'The exact project checkpoint can no longer be verified safely. Recover the project, or close and reopen it, before editing items.';

  @override
  String get managedItemsCatalogUnsupported =>
      'This project contains item data that the current exact game schema cannot edit safely. Nothing was changed.';

  @override
  String get managedItemsCatalogLoadUnexpected =>
      'The item data could not be loaded safely. Nothing was changed. Try loading it again.';

  @override
  String get managedItemsCatalogReload => 'Reload item data';

  @override
  String get managedItemsUnsupportedSchema =>
      'This item change no longer matches the current safe catalog or field schema. You can still revert the whole item.';

  @override
  String get managedItemsDefaultUnknown => 'Game default not recorded';

  @override
  String managedItemsGameDefault(String value) {
    return 'Game default: $value';
  }

  @override
  String get managedItemsModValue => 'Mod value';

  @override
  String get managedContentScopeBaseGameLabel => 'Основная игра';

  @override
  String get managedContentScopeInstalledLabel => 'Установлено';

  @override
  String get managedBaseGameBrowserTitle =>
      'Поддерживаемые исходные объекты основной игры';

  @override
  String get managedBaseGameBrowserDescription =>
      'Просматривайте точные данные установленной игры, которые Mod Studio может проверить или использовать как безопасную основу черновика. Это не полный каталог исходного содержимого.';

  @override
  String get managedBaseGameBrowserLoading =>
      'Чтение точных данных основной игры…';

  @override
  String get managedBaseGameBrowserRefresh => 'Прочитать новый точный каталог';

  @override
  String get managedBaseGameBrowserSearchLabel =>
      'Поиск поддерживаемого содержимого основной игры';

  @override
  String get managedBaseGameBrowserFilterNpcs => 'NPC';

  @override
  String get managedBaseGameBrowserFilterQuests => 'Задания';

  @override
  String get managedBaseGameBrowserNpcSectionTitle => 'Исходные NPC';

  @override
  String get managedBaseGameBrowserQuestSectionTitle => 'Исходные задания';

  @override
  String get managedBaseGameBrowserExperimentalNpcSectionTitle =>
      'Архетипы NPC только для просмотра';

  @override
  String get managedBaseGameBrowserSearchForExperimental =>
      'Поиск включает дополнительные статически связанные данные NPC. Из этих строк нельзя создать черновик.';

  @override
  String get managedBaseGameBrowserEmpty =>
      'Поддерживаемых результатов основной игры для этого поиска нет.';

  @override
  String get managedBaseGameBrowserLoadErrorTitle =>
      'Данные основной игры недоступны';

  @override
  String get managedBaseGameBrowserLoadErrorDescription =>
      'Не удалось прочитать точный поддерживаемый каталог. Файлы проекта, игры и сохранений не изменены.';

  @override
  String get managedBaseGameBrowserOfflineDraftBadge =>
      'Черновик офлайн поддерживается';

  @override
  String get managedBaseGameBrowserInspectOnlyBadge => 'Только просмотр';

  @override
  String get managedBaseGameBrowserCreateNpcDraft =>
      'Использовать как основу NPC';

  @override
  String get managedBaseGameBrowserCreateQuestDraft =>
      'Использовать как основу задания';

  @override
  String get managedBaseGameBrowserSpawnClass => 'Определение создания';

  @override
  String get managedBaseGameBrowserActorBlueprint => 'Blueprint актора';

  @override
  String get managedBaseGameBrowserExperimentalResultsCapped =>
      'Показаны первые 100 совпадений только для просмотра. Уточните поиск для более точных результатов.';

  @override
  String get managedInstalledBrowserLoading =>
      'Чтение точного списка установленных пакетов…';

  @override
  String managedInstalledBrowserCompleteSummary(int count) {
    return 'Кандидатов среди установленных пакетов: $count';
  }

  @override
  String managedInstalledBrowserPartialSummary(int count) {
    return 'Кандидатов среди установленных пакетов: $count — частичный результат';
  }

  @override
  String get managedInstalledBrowserCompleteDescription =>
      'Метаданные каталога прочитаны, точность установленного снимка сохранена.';

  @override
  String get managedInstalledBrowserPartialDescription =>
      'Часть метаданных пакетов отсутствовала или имела неканонический вид; результаты полезны для поиска, но неполны.';

  @override
  String get managedInstalledBrowserAuthorityNotice =>
      'Здесь показаны только метаданные установленных пакетов DataAsset. Просмотр или копирование пути не даёт разрешения на сборку, развёртывание, выполнение или запись в игру.';

  @override
  String get managedInstalledBrowserRefresh => 'Прочитать новый точный снимок';

  @override
  String get managedInstalledBrowserSearchLabel =>
      'Поиск установленных DataAssets';

  @override
  String get managedInstalledBrowserSearchHint => 'Имя ресурса или путь /Game';

  @override
  String get managedInstalledBrowserSearchPrompt =>
      'Введите имя ресурса или путь /Game для поиска.';

  @override
  String get managedInstalledBrowserNoMatchesTitle =>
      'Подходящих установленных DataAsset нет';

  @override
  String get managedInstalledBrowserNoMatchesDescription =>
      'Попробуйте другое имя ресурса или более общий путь /Game.';

  @override
  String get managedInstalledBrowserResultLimitDescription =>
      'Показаны первые 100 совпадений. Уточните поиск, чтобы сузить точный снимок.';

  @override
  String get managedInstalledBrowserKindBadge => 'Пакет DataAsset';

  @override
  String get managedInstalledBrowserMetadataOnlyBadge => 'Только метаданные';

  @override
  String get managedInstalledBrowserOpenInspector => 'Проверить точный пакет';

  @override
  String get managedInstalledBrowserErrorTitle =>
      'Список установленных пакетов недоступен';

  @override
  String get managedInstalledBrowserErrorDescription =>
      'Не удалось прочитать точный снимок установленных пакетов. Файлы проекта, игры и сохранений не изменены.';

  @override
  String get managedGlobalSearchScopeLabel => 'Искать везде';

  @override
  String get managedGlobalSearchTitle => 'Поиск по всему содержимому';

  @override
  String get managedGlobalSearchLabel =>
      'NPC, задание, реплика, ресурс, ID или путь /Game';

  @override
  String get managedGlobalSearchAction => 'Искать';

  @override
  String get managedGlobalSearchClear => 'Очистить';

  @override
  String get managedGlobalSearchPrompt =>
      'Введите запрос для независимого поиска в трёх источниках.';

  @override
  String get managedGlobalSearchNoResults => 'В этом источнике совпадений нет.';

  @override
  String get managedGlobalSearchLoading => 'Чтение точного источника…';

  @override
  String get managedGlobalSearchFailed => 'Не удалось прочитать этот источник.';

  @override
  String get managedGlobalSearchComplete => 'Полностью';

  @override
  String get managedGlobalSearchPartial => 'Частично';

  @override
  String get managedGlobalSearchTruncated =>
      'Показаны первые 100 совпадений. Уточните запрос.';

  @override
  String get managedGlobalSearchOpen => 'Открыть';

  @override
  String get managedGlobalSearchCreateDraft => 'Создать черновик';

  @override
  String get managedGlobalSearchInspect => 'Проверить';

  @override
  String get managedGlobalSearchKindModEntity => 'Контент мода';

  @override
  String get managedGlobalSearchKindModAsset => 'Ресурс мода';

  @override
  String get managedGlobalSearchKindBaseNpc => 'Исходный NPC';

  @override
  String get managedGlobalSearchKindBaseQuest => 'Исходное задание';

  @override
  String get managedGlobalSearchKindExperimentalNpc => 'Данные NPC';

  @override
  String get managedGlobalSearchReadinessExact => 'Точный текущий проект';

  @override
  String get managedGlobalSearchReadinessProblems => 'Точно, но с проблемами';

  @override
  String get managedGlobalSearchResultStale =>
      'Этого результата больше нет в текущем проекте. Выполните поиск снова.';

  @override
  String get managedStoryWorkbenchDraftBadge => 'Только черновик';

  @override
  String get managedStoryWorkbenchBuildBlockedBadge => 'Сборка заблокирована';

  @override
  String get managedStoryWorkbenchRuntimeUnqualifiedBadge =>
      'Работа в игре не проверена';

  @override
  String get managedStoryWorkbenchOverviewTab => 'Обзор';

  @override
  String get managedStoryWorkbenchProfileTab => 'Профиль';

  @override
  String get managedStoryWorkbenchStoryTab => 'Сюжет';

  @override
  String get managedStoryWorkbenchLogicTab => 'Логика';

  @override
  String get managedStoryWorkbenchRoutineTab => 'Распорядок';

  @override
  String get managedStoryWorkbenchInventoryTab => 'Инвентарь';

  @override
  String get managedStoryWorkbenchDialogVoiceTab => 'Диалоги и озвучка';

  @override
  String get managedStoryWorkbenchReferencesTab => 'Ссылки';

  @override
  String get managedStoryWorkbenchProblemsChecksTab => 'Проблемы и проверки';

  @override
  String get managedStoryWorkbenchEditOverview => 'Изменить имя и цели';

  @override
  String get managedStoryWorkbenchEditStory => 'Изменить описание и связи';

  @override
  String get managedStoryWorkbenchEditLogic => 'Изменить состояния и переходы';

  @override
  String get managedStoryWorkbenchInspectQuest =>
      'Открыть исходный код и проверки компилятора';

  @override
  String get managedStoryWorkbenchInspectNpc =>
      'Открыть профиль и проверки компилятора';

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
  String get managedStoryWorkbenchCapabilityUnavailable =>
      'Ещё не смоделировано';

  @override
  String get managedStoryWorkbenchNpcStoryUnavailable =>
      'Связи с заданиями и сюжетом ещё не смоделированы для черновиков NPC.';

  @override
  String get managedStoryWorkbenchNpcRoutineUnavailable =>
      'Распорядок и размещение в мире ещё не смоделированы.';

  @override
  String get managedStoryWorkbenchNpcInventoryUnavailable =>
      'Инвентарь, экипировка и торговля ещё не смоделированы.';

  @override
  String get managedStoryWorkbenchNpcDialogVoiceUnavailable =>
      'Связи с диалогами, локализацией и озвучкой ещё не смоделированы для черновиков NPC.';

  @override
  String get managedStoryWorkbenchQuestDialogVoiceUnavailable =>
      'Связи с диалогами, локализацией и озвучкой ещё не смоделированы для черновиков заданий.';

  @override
  String get managedStoryWorkbenchNoReferenceProblems =>
      'Нет неразрешённых ссылок в проекте';

  @override
  String managedStoryWorkbenchReferenceProblemCount(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count неразрешённой ссылки в проекте',
      many: '$count неразрешённых ссылок в проекте',
      few: '$count неразрешённые ссылки в проекте',
      one: '1 неразрешённая ссылка в проекте',
    );
    return '$_temp0';
  }

  @override
  String get managedStoryWorkbenchReferenceScopeNotice =>
      'Только прямые ссылки этого черновика; проблемы дочерних материалов диалогов и озвучки не включены. Это не означает готовности к сборке или запуску.';

  @override
  String get managedStoryWorkbenchTechnicalDetails => 'Технические сведения';

  @override
  String get managedStoryWorkbenchQuestKindLabel => 'Черновик задания';

  @override
  String get managedStoryWorkbenchNpcKindLabel => 'Черновик NPC';

  @override
  String get managedStoryWorkbenchQuestTitleLabel => 'Название задания';

  @override
  String get managedStoryWorkbenchTechnicalIdLabel => 'Технический ID';

  @override
  String get managedStoryWorkbenchObjectivesLabel => 'Цели';

  @override
  String get managedStoryWorkbenchUniqueNameLabel => 'Уникальное имя';

  @override
  String get managedStoryWorkbenchModuleNamespaceLabel =>
      'Пространство имён модуля';

  @override
  String get managedStoryWorkbenchQuestGiverLabel => 'Выдающий задание';

  @override
  String get managedStoryWorkbenchRuntimeParentLabel =>
      'Родительский класс времени выполнения';

  @override
  String get managedStoryWorkbenchLogicDescription =>
      'Состояния жизненного цикла задания, триггеры, условия и эффекты редактируются как единая атомарная операция над точным текущим состоянием.';

  @override
  String get managedStoryWorkbenchOutgoingHeading => 'Исходящие';

  @override
  String get managedStoryWorkbenchNoOutgoingReferences =>
      'Нет предполагаемых ссылок';

  @override
  String get managedStoryWorkbenchIncomingHeading => 'Входящие';

  @override
  String get managedStoryWorkbenchNoIncomingReferences =>
      'Нет входящих ссылок проекта';

  @override
  String get managedStoryWorkbenchSemanticIdentityLabel =>
      'Семантическая идентичность';

  @override
  String get managedStoryWorkbenchOriginLabel => 'Источник';

  @override
  String get managedStoryWorkbenchEntityRevisionLabel => 'Ревизия сущности';

  @override
  String get managedStoryWorkbenchStableIdLabel => 'Стабильный ID';

  @override
  String get managedStoryWorkbenchReferenceResolvedLabel => 'Ссылка разрешена';

  @override
  String get managedStoryWorkbenchReferenceUnresolvedLabel =>
      'Ссылка не разрешена';

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
  String get managedStoryWorkbenchNpcDraftSetupTitle => 'Write this Character';

  @override
  String get managedStoryWorkbenchNpcDraftSetupDescription =>
      'This view tracks the exact Character details and first authored greeting as two project steps in the current revision.';

  @override
  String get managedStoryWorkbenchNpcDraftSetupCharacterDetailsTitle =>
      '1. Character details';

  @override
  String get managedStoryWorkbenchNpcDraftSetupFirstGreetingTitle =>
      '2. First greeting';

  @override
  String get managedStoryWorkbenchNpcDraftSetupCompleteStatus =>
      'Saved in project';

  @override
  String get managedStoryWorkbenchNpcDraftSetupNextStatus =>
      'Recommended next step';

  @override
  String get managedStoryWorkbenchNpcDraftSetupOpenStatus => 'Still open';

  @override
  String get managedStoryWorkbenchNpcDraftSetupCharacterDetailsComplete =>
      'The exact Character name and reviewed archetype parents are present in this project revision.';

  @override
  String get managedStoryWorkbenchNpcDraftSetupCharacterDetailsUnavailable =>
      'The exact current Character details could not be verified.';

  @override
  String get managedStoryWorkbenchNpcDraftSetupFirstGreetingPending =>
      'Link the first authored greeting in Dialog & Voice.';

  @override
  String
  get managedStoryWorkbenchNpcDraftSetupFirstGreetingDetailsUnavailable =>
      'Text and Voice coverage for the first greeting could not be verified in this exact project revision.';

  @override
  String get managedStoryWorkbenchNpcDraftSetupRecommendedNext =>
      'Recommended next step';

  @override
  String get managedStoryWorkbenchNpcDraftSetupWriteFirstGreeting =>
      'Write first greeting';

  @override
  String get managedStoryWorkbenchNpcDraftSetupReviewDialogVoice =>
      'Review greetings in Dialog & Voice';

  @override
  String get managedStoryWorkbenchNpcDraftSetupActionUnavailable =>
      'Dialog & Voice is unavailable for this exact project revision.';

  @override
  String get managedStoryWorkbenchNpcDraftSetupBoundary =>
      'Draft setup tracks current authored project content only. A greeting link is not a playable dialog topic and does not prove publication history, build, or runtime behavior.';

  @override
  String managedStoryWorkbenchNpcDraftSetupGreetingLinkCount(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count authored greeting links',
      one: '1 authored greeting link',
      zero: 'No authored greeting links',
    );
    return '$_temp0';
  }

  @override
  String managedStoryWorkbenchNpcDraftSetupTextLanguageCount(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count text languages',
      one: '1 text language',
    );
    return '$_temp0';
  }

  @override
  String managedStoryWorkbenchNpcDraftSetupVoiceTakeCount(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count Voice takes',
      one: '1 Voice take',
    );
    return '$_temp0';
  }

  @override
  String managedStoryWorkbenchNpcDraftSetupSelectedVoiceCount(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count selected Voice takes',
      one: '1 selected Voice take',
    );
    return '$_temp0';
  }

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
  String get managedVoiceBuildReadinessTitle => 'Проверка голосового пакета';

  @override
  String get managedVoiceBuildReadinessRefresh =>
      'Обновить проверку голосового пакета';

  @override
  String get managedVoiceBuildReadinessChecking =>
      'Проверка точного плана голосового пакета';

  @override
  String get managedVoiceBuildReadinessLoadError =>
      'Не удалось проверить точный план голосового пакета для текущего проекта. Этот результат не содержит свидетельств по плану пакета.';

  @override
  String get managedVoiceBuildReadinessReadyTitle =>
      'План голосового пакета проверен';

  @override
  String get managedVoiceBuildReadinessBlockedTitle =>
      'План голосового пакета требует внимания';

  @override
  String managedVoiceBuildReadinessCount(int readySlots, int totalSlots) {
    return '$readySlots из $totalSlots существующих слотов Voice соответствуют этому плану пакета.';
  }

  @override
  String get managedVoiceBuildReadinessBlockedBoundary =>
      'Пакет не создан, развёртывание не выполнялось.';

  @override
  String get managedVoiceBuildReadinessBuildBundle => 'Build bundle';

  @override
  String get managedVoiceBuildReadinessBuildReleaseGuidance =>
      'Здесь проверяется только план; создание автономного голосового пакета остаётся отдельным действием.';

  @override
  String get managedVoiceBuildReadinessConfigureGameGuidance =>
      'Точный план голосового пакета проверен. Чтобы стало доступно отдельное действие создания автономного пакета, по-прежнему требуется настроенная установка игры.';

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
  String get managedWorkspaceTextVoiceLabel => 'Текст и озвучка';

  @override
  String get managedWorkspaceTestReleaseLabel => 'Тестирование и выпуск';

  @override
  String get managedTestReleaseTitle => 'Тестирование и выпуск';

  @override
  String get managedTestReleaseDescription =>
      'Проверьте все части мода перед созданием игровых файлов или их установкой.';

  @override
  String get managedTestReleaseEvidenceBoundary =>
      'Ничто не считается готовым автоматически. Результат проверки относится только к этой точной сохранённой версии проекта.';

  @override
  String get managedTestReleaseChecksHeading => 'Проверки проекта';

  @override
  String get managedTestReleaseReleaseHeading => 'Игровые файлы';

  @override
  String get managedTestReleaseStatusNotChecked => 'Не проверено';

  @override
  String get managedTestReleaseStatusChecking => 'Идёт проверка';

  @override
  String get managedTestReleaseStatusChecked => 'Проверено';

  @override
  String get managedTestReleaseStatusNeedsAttention => 'Требует внимания';

  @override
  String get managedTestReleaseStatusBlocked => 'Заблокировано';

  @override
  String get managedTestReleaseStatusNotAvailable => 'Недоступно';

  @override
  String get managedTestReleaseStatusAvailable => 'Доступно';

  @override
  String get managedTestReleaseEvidenceLabel => 'Подтверждение';

  @override
  String get managedTestReleaseStaleEvidenceDescription =>
      'Этот результат относится к другой версии проекта. Запустите проверку снова.';

  @override
  String get managedTestReleaseActionNotConnectedDescription =>
      'Подтверждение существует, но это действие ещё не подключено в текущей рабочей области.';

  @override
  String get managedTestReleaseProblemsHeading => 'Проблемы для решения';

  @override
  String get managedTestReleaseVoiceHeading => 'Проверка голосового пакета';

  @override
  String get managedTestReleaseProjectStructureTitle => 'Структура проекта';

  @override
  String get managedTestReleaseProjectStructureDescription =>
      'Просмотрите ниже актуальный список проблем, чтобы проверить ссылки и структуру управляемого проекта.';

  @override
  String get managedTestReleaseProjectStructureAction => 'Просмотреть проблемы';

  @override
  String get managedTestReleaseScriptsTitle => 'Скрипты';

  @override
  String get managedTestReleaseScriptsDescription =>
      'Один раз запустите компилятор игры для всех скриптов этой точно сохранённой версии проекта. Результат служит только подтверждением проверки; выходные данные удаляются.';

  @override
  String get managedTestReleaseScriptsAction => 'Запустить проверку';

  @override
  String get managedProjectCompilerRetryAction => 'Повторить проверку';

  @override
  String get managedProjectCompilerReviewAction =>
      'Посмотреть результат / проверить снова';

  @override
  String get managedProjectCompilerDialogTitle =>
      'Проверить все скрипты проекта';

  @override
  String get managedProjectCompilerDialogIntroduction =>
      'Перед началом закройте Gothic 1 Remake. Mod Studio временно проверит все скрипты проекта компилятором игры, восстановит установку и удалит все выходные данные компилятора. Этот результат не позволяет создавать игровые файлы или устанавливать мод.';

  @override
  String get managedProjectCompilerCloseAction => 'Закрыть';

  @override
  String get managedProjectCompilerNoGame =>
      'Перед проверкой выберите установку Gothic 1 Remake в настройках.';

  @override
  String get managedProjectCompilerSafetyBlocked =>
      'Установка игры не готова к проверке компилятором. Закройте игру или устраните предупреждение о восстановлении и повторите попытку.';

  @override
  String get managedProjectCompilerCompiled =>
      'Все скрипты проекта прошли проверку для этой точно сохранённой версии. Выходные данные компилятора удалены.';

  @override
  String get managedProjectCompilerEmpty =>
      'В этой сохранённой версии проекта нет скриптов для компиляции. Пустой результат точно проверен.';

  @override
  String get managedProjectCompilerRejected =>
      'Компилятор обнаружил проблемы в одном или нескольких скриптах проекта. Исправьте сообщения ниже и повторите попытку.';

  @override
  String get managedProjectCompilerPreflightBlocked =>
      'Компилятор не запустился. Закройте игру, проверьте настроенную установку и повторите попытку.';

  @override
  String get managedProjectCompilerDrifted =>
      'Проект или данные игры изменились, либо итоговая сверка перестала быть точной. Результат удалён; запустите проверку снова для текущей версии.';

  @override
  String get managedProjectCompilerRequiresReopen =>
      'Перед следующей точной проверкой проект необходимо закрыть и открыть заново.';

  @override
  String get managedProjectCompilerRecoveryRequired =>
      'Не удалось подтвердить завершение очистки закрытых выходных данных компилятора или точного восстановления установки игры. Дальнейшие проверки компилятором и установка остаются заблокированными до успешного завершения новой проверки безопасности.';

  @override
  String get managedProjectCompilerFailed =>
      'Не удалось завершить или подтвердить проверку. Результат не сохранён; повторите попытку, когда установка игры будет готова.';

  @override
  String get managedProjectCompilerFailureDetails => 'Сообщение компилятора';

  @override
  String get managedProjectCompilerDiagnosticsHeading =>
      'Сообщения компилятора';

  @override
  String get managedProjectCompilerCaptureCaptured =>
      'Структурированные сообщения компилятора получены.';

  @override
  String get managedProjectCompilerCaptureFallback =>
      'Диагностический интерфейс был недоступен, поэтому использован обычный компилятор игры.';

  @override
  String get managedProjectCompilerCaptureInvalid =>
      'Не удалось подтвердить получение сообщений компилятора.';

  @override
  String get managedProjectCompilerCaptureUnavailable =>
      'Диагностический интерфейс был недоступен после запуска компилятора; повторный запуск не требовался.';

  @override
  String get managedProjectCompilerCaptureExitUnconfirmed =>
      'Процесс компилятора не подтвердил завершение.';

  @override
  String get managedProjectCompilerCaptureDisabled =>
      'Для этого запуска структурированные сообщения компилятора недоступны.';

  @override
  String get managedProjectCompilerSeverityError => 'Ошибка';

  @override
  String get managedProjectCompilerSeverityWarning => 'Предупреждение';

  @override
  String get managedProjectCompilerSeverityNote => 'Примечание';

  @override
  String get managedProjectCompilerFileLabel => 'Файл';

  @override
  String get managedProjectCompilerLineLabel => 'Строка';

  @override
  String get managedProjectCompilerColumnLabel => 'Столбец';

  @override
  String get managedProjectCompilerOmittedDiagnostics =>
      'дополнительных сообщений компилятора скрыто';

  @override
  String get managedTestReleaseVoiceTitle => 'Проверка голосового пакета';

  @override
  String get managedTestReleaseVoiceDescription =>
      'Проверяет только точный текущий план голосового пакета для существующих элементов. Не проверяет полноту текста или переводов, воспроизведение, результаты сборки, развёртывание или работу во время выполнения.';

  @override
  String get managedTestReleaseVoiceAction => 'Открыть проверку пакета';

  @override
  String get managedTestReleaseDataAssetsTitle => 'DataAssets';

  @override
  String get managedTestReleaseDataAssetsDescription =>
      'Проверяет только точный текущий домен подготовленных DataAssets, уже подтверждённый предварительным просмотром сборки проекта. Проверка не охватывает новые или структурные ресурсы, игровые файлы, установку, развёртывание, изменения игры или сохранений, поведение во время выполнения и содержимое мира.';

  @override
  String get managedTestReleaseDataAssetsAction => 'Просмотреть DataAsset';

  @override
  String get managedTestReleasePlayableBuildTitle => 'Игровые файлы';

  @override
  String get managedTestReleasePlayableBuildDescription =>
      'Создайте проверенную игровую сборку из этой точной сохранённой версии проекта.';

  @override
  String get managedTestReleasePlayableBuildBlockedReason =>
      'Для этой сохранённой версии ещё нет точного подтверждения полной сборки проекта.';

  @override
  String get managedTestReleaseCreatePlayableFilesAction =>
      'Создать игровые файлы';

  @override
  String get managedTestReleaseDeploymentTitle => 'Установка';

  @override
  String get managedTestReleaseDeploymentDescription =>
      'Установите точно проверенную игровую сборку в настроенную игру.';

  @override
  String get managedTestReleaseDeploymentBlockedReason =>
      'Для этой сохранённой версии проекта ещё нет точного подтверждения сборки, готовой к установке.';

  @override
  String get managedTestReleaseInstallAction => 'Установить';

  @override
  String managedProjectCommandBarCurrentSection(String section) {
    return 'Текущий раздел: $section';
  }

  @override
  String managedProjectCommandBarOrientationSemantics(
    String project,
    String section,
  ) {
    return 'Проект $project. Текущий раздел: $section.';
  }

  @override
  String get managedProjectCommandBarUndoLabel => 'Отменить';

  @override
  String get managedProjectCommandBarSearchLabel => 'Поиск';

  @override
  String get managedProjectCommandBarCreateLabel => 'Создать';

  @override
  String get managedProjectCommandBarProblemsLabel => 'Проблемы';

  @override
  String get managedProjectCommandBarHistoryLabel => 'История';

  @override
  String get managedProjectCommandBarSettingsLabel => 'Настройки';

  @override
  String get managedProjectCommandBarMoreActionsTooltip =>
      'Другие действия проекта';

  @override
  String get managedProjectCommandBarBusyLabel =>
      'Завершение текущего действия проекта…';

  @override
  String get managedProjectCommandBarBusyDisabledReason =>
      'Дождитесь завершения текущего действия проекта.';
}
