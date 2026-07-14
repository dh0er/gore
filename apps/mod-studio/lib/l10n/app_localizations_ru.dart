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
  String get aboutCopyright => '© 2026 участники проекта GORE';

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
  String get projectOpenLegacy => 'Open legacy project…';

  @override
  String get projectOpenManagedRevision3 => 'Open managed revision-3 project…';

  @override
  String get projectVerifyCurrentHead => 'Verify current head';

  @override
  String get projectManagedRevision3Title => 'Managed revision-3 project';

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
      'Новый проект открыт, но не удалось полностью очистить сеанс предыдущего проекта. Повторная очистка выполняться не будет. Перезапустите Mod Studio, прежде чем снова открывать предыдущий проект.';

  @override
  String get projectNewManagedRevision3 => 'Новый управляемый проект мода…';

  @override
  String get projectNewLegacy => 'Новый проект старого формата';

  @override
  String get projectCreateGamePathRequired =>
      'Перед созданием проекта мода укажите путь к Gothic 1 Remake в настройках.';

  @override
  String get projectCreateDirectoryPickerTitle =>
      'Создать здесь управляемый проект мода';

  @override
  String projectManagedRevision3Created(String projectId) {
    return 'Управляемый проект мода $projectId создан';
  }

  @override
  String projectManagedRevision3CreateFailed(String error) {
    return 'Не удалось создать управляемый проект мода: $error';
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
  String get managedProjectSubtitle =>
      'Рабочая область для автономного редактирования, точно соответствующая текущей версии';

  @override
  String get managedProjectTechnicalDetails => 'Технические сведения о проекте';

  @override
  String get managedProjectRecoveryContentLocked =>
      'Снова откройте управляемый проект, прежде чем читать его содержимое.';

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
  String get managedActionNewQuestTitle => 'Новое задание';

  @override
  String get managedActionNewQuestDescription =>
      'Создать автономный черновик задания с целями и проверенными родительскими идентификаторами.';

  @override
  String get managedActionAddVoiceTakeTitle => 'Добавить запись озвучки';

  @override
  String get managedActionAddVoiceTakeDescription =>
      'Импортировать запись Ogg Vorbis в этот проект без её развёртывания.';

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
  String get managedActionSettingsTitle => 'Настройки';

  @override
  String get managedActionSettingsDescription =>
      'Настроить установку Gothic 1 Remake и параметры Mod Studio.';
}
