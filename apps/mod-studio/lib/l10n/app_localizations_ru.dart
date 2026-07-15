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
  String get managedContentWorkspaceLibraryLabel => 'Этот мод';

  @override
  String get managedWorkspaceHomeLabel => 'Главная';

  @override
  String get managedWorkspaceStoryLabel => 'Сюжет';

  @override
  String get managedWorkspaceWorldLabel => 'Мир';

  @override
  String get managedWorkspaceLocalizationVoiceLabel => 'Локализация и озвучка';

  @override
  String get managedWorkspaceValidateTestLabel => 'Проверка и тестирование';

  @override
  String get managedWorkspaceBuildReleaseLabel => 'Сборка и выпуск';

  @override
  String get managedWorkspaceSettingsExpertLabel =>
      'Настройки и экспертный режим';

  @override
  String get managedSectionStoryDescription => 'NPC, задания и диалоги.';

  @override
  String get managedSectionWorldDescription =>
      'Размещение в мире и связанные рабочие процессы запланированы.';

  @override
  String get managedSectionLocalizationVoiceDescription =>
      'Инструменты производства озвучки доступны; редактирование локализации в управляемом проекте запланировано.';

  @override
  String get managedSectionValidateTestDescription =>
      'Проверяет точную целостность проекта и контрольные точки; тестирование в игре не подтверждается.';

  @override
  String get managedSectionBuildReleaseDescription =>
      'Пакеты озвучки доступны; полноценные игровые сборки и развёртывание недоступны.';

  @override
  String get managedSectionSettingsExpertDescription =>
      'Настройки доступны; экспертные инструменты ещё не интегрированы.';

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
  String get managedProjectLandingTitle =>
      'Рабочая область управляемого проекта';

  @override
  String get managedProjectLandingDescription =>
      'Используйте новый процесс работы с разделами «Главная», «Контент», «Сюжет», «Озвучка», «Проверка» и «Выпуск» в одном управляемом проекте.';

  @override
  String get legacyCompatibilityToolsTitle =>
      'Инструменты совместимости старого формата';

  @override
  String get legacyCompatibilityToolsDescription =>
      'Вкладки ниже содержат прежние инструменты прямой замены. Они останутся доступными, пока рабочая область управляемого проекта развивается.';

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
      'Это только статус ссылок; он не означает готовности к сборке или запуску.';

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
}
