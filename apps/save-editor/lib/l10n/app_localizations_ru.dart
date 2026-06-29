// ignore: unused_import
import 'package:intl/intl.dart' as intl;
import 'app_localizations.dart';

// ignore_for_file: type=lint

/// The translations for Russian (`ru`).
class AppLocalizationsRu extends AppLocalizations {
  AppLocalizationsRu([String locale = 'ru']) : super(locale);

  @override
  String get appTitle => 'Редактор сохранений Gothic Remake';

  @override
  String get appLogoSemanticLabel => 'Логотип goresave';

  @override
  String get zoomTooltip => 'Нажмите Ctrl +/- для увеличения или уменьшения';

  @override
  String get switchToLightMode => 'Переключить на светлую тему';

  @override
  String get switchToDarkMode => 'Переключить на тёмную тему';

  @override
  String get about => 'О программе';

  @override
  String get tabOverview => 'Обзор';

  @override
  String get tabPlayer => 'Персонаж';

  @override
  String get tabInventory => 'Инвентарь';

  @override
  String get tabProgression => 'Прогресс';

  @override
  String get tabAllData => 'Все данные';

  @override
  String get tabBackups => 'Резервные копии';

  @override
  String get tabSettings => 'Настройки';

  @override
  String get reset => 'Сбросить';

  @override
  String get save => 'Сохранить';

  @override
  String saveWithCount(int count) {
    return 'Сохранить ($count)';
  }

  @override
  String get ok => 'ОК';

  @override
  String get cancel => 'Отмена';

  @override
  String get confirm => 'Подтвердить';

  @override
  String get close => 'Закрыть';

  @override
  String get add => 'Добавить';

  @override
  String get equippedBadge => 'Надето';

  @override
  String get browse => 'Обзор';

  @override
  String get noSavFilesFound => 'Файлы .sav не найдены';

  @override
  String get profile => 'Профиль';

  @override
  String profileWithSaves(String name, int count) {
    return '$name (сохранений: $count)';
  }

  @override
  String get switchProfile => 'Сменить профиль';

  @override
  String get rescanSaveFolder => 'Пересканировать папку сохранений';

  @override
  String get discardUnsavedChangesTitle => 'Отменить несохранённые изменения?';

  @override
  String rescanDiscardBody(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: 'ваши несохранённые изменения ($count)',
      one: 'ваше $count несохранённое изменение',
    );
    return 'Повторное сканирование перезагрузит все сохранения и отменит $_temp0.';
  }

  @override
  String get discardAndRescan => 'Отменить и пересканировать';

  @override
  String chapterLabel(Object id) {
    return 'Глава $id';
  }

  @override
  String get quickSave => 'Быстрое сохранение';

  @override
  String get autoSave => 'Автосохранение';

  @override
  String get manualSave => 'Ручное сохранение';

  @override
  String get errorTitle => 'Ошибка';

  @override
  String get selectASaveTitle => 'Выберите сохранение';

  @override
  String get selectASaveBody => 'Здесь появятся сведения о сохранении.';

  @override
  String get diagnosticsTitle => 'Диагностика и сведения';

  @override
  String get diagnosticsSubtitle => 'Просмотр формата (только чтение)';

  @override
  String get metricFormat => 'Формат';

  @override
  String get metricSlot => 'Слот';

  @override
  String get metricChapter => 'Глава';

  @override
  String get metricTimePlayed => 'Время в игре';

  @override
  String get metricSaveKind => 'Тип сохранения';

  @override
  String get metricFileSize => 'Размер файла';

  @override
  String get metricCompression => 'Сжатие';

  @override
  String get metricChunks => 'Блоки';

  @override
  String get metricUncompressed => 'Без сжатия';

  @override
  String get metricPrivate => 'Приватные';

  @override
  String get metricSlotName => 'Имя слота';

  @override
  String get metricTrailer => 'Концовка';

  @override
  String get metricDecodedPrivate => 'Декодированные приватные';

  @override
  String get metricPrivateStrings => 'Приватные строки';

  @override
  String get metricSha1 => 'SHA-1';

  @override
  String bytesValue(String count) {
    return '$count байт';
  }

  @override
  String get inspectionJsonTitle => 'JSON проверки';

  @override
  String get inspectionJsonSubtitle =>
      'Необработанные данные проверки сохранения';

  @override
  String get copy => 'Копировать';

  @override
  String get savegameFallbackTitle => 'Сохранение';

  @override
  String screenshotForSlot(String slot) {
    return 'Снимок экрана для $slot';
  }

  @override
  String get publicSaveName => 'Публичное имя сохранения';

  @override
  String get required => 'Обязательно';

  @override
  String get playerLockedBody =>
      'Для редактирования приватных данных персонажа нужен кодек с поддержкой сжатия.';

  @override
  String get heroTransform => 'Положение героя';

  @override
  String get locationX => 'Координата X';

  @override
  String get locationY => 'Координата Y';

  @override
  String get locationZ => 'Координата Z';

  @override
  String get rotationPitch => 'Тангаж';

  @override
  String get rotationYaw => 'Рыскание';

  @override
  String get rotationRoll => 'Крен';

  @override
  String get invalid => 'Недопустимо';

  @override
  String get heroAttributes => 'Атрибуты героя';

  @override
  String attributeBase(String name) {
    return '$name (базовое)';
  }

  @override
  String attributeCurrent(String name) {
    return '$name (текущее)';
  }

  @override
  String get inventoryTitle => 'Инвентарь';

  @override
  String get inventoryNeedsDecoded =>
      'Для редактирования инвентаря нужны декодированные приватные данные из кодека.';

  @override
  String get inventoryNoStacks =>
      'В декодированных приватных данных не найдено стопок предметов.';

  @override
  String get resetInventoryChanges => 'Сбросить изменения инвентаря';

  @override
  String get addItemTooltipPendingAdd =>
      'Сначала сохраните ожидающие изменения — один новый предмет за одно сохранение';

  @override
  String get addItemTooltipPendingRemove =>
      'Сначала сохраните ожидающее удаление — одно структурное изменение за одно сохранение';

  @override
  String get addItemTooltipPendingCount =>
      'Сначала сохраните или сбросьте ожидающие изменения количества — структурное изменение нужно сохранять отдельно';

  @override
  String get addItemTooltipDefault => 'Добавить предмет в инвентарь';

  @override
  String get addItemButton => 'Добавить предмет';

  @override
  String pendingAddSubtitle(int count) {
    return '×$count — ожидающее добавление (ещё не сохранено)';
  }

  @override
  String get cancelPendingAdd => 'Отменить ожидающее добавление';

  @override
  String get pendingRemovalSubtitle => 'ожидающее удаление (ещё не сохранено)';

  @override
  String get cancelPendingRemoval => 'Отменить ожидающее удаление';

  @override
  String get filterItems => 'Фильтровать предметы';

  @override
  String noItemsMatchQuery(String query) {
    return 'Ни один предмет не соответствует «$query».';
  }

  @override
  String get pendingRemovalHidesAll =>
      'Ожидающее удаление скрывает все предметы — сохраните, чтобы применить его.';

  @override
  String categoryWithCount(String label, int count) {
    return '$label ($count)';
  }

  @override
  String get itemCategoryMeleeWeapon => 'Оружие ближнего боя';

  @override
  String get itemCategoryRangedWeapon => 'Дальнобойное оружие';

  @override
  String get itemCategoryAmmunition => 'Боеприпасы';

  @override
  String get itemCategoryArmor => 'Броня';

  @override
  String get itemCategoryRune => 'Руны';

  @override
  String get itemCategoryScroll => 'Свитки заклинаний';

  @override
  String get itemCategoryFood => 'Еда и зелья';

  @override
  String get itemCategoryMisc => 'Разное';

  @override
  String get itemCategoryAmulet => 'Амулеты';

  @override
  String get itemCategoryRing => 'Кольца';

  @override
  String get itemCategoryTrophy => 'Трофеи животных';

  @override
  String get itemCategoryWriting => 'Записи';

  @override
  String get itemCategoryMission => 'Квестовые предметы';

  @override
  String get itemCategoryKey => 'Ключи';

  @override
  String get itemCategoryOther => 'Прочее';

  @override
  String get count => 'Количество';

  @override
  String get min1 => 'Мин. 1';

  @override
  String countTimes(String count) {
    return '×$count';
  }

  @override
  String get deleteEquippedTooltip =>
      'Нельзя удалить: этот предмет, вероятно, надет или назначен на ячейку быстрого доступа';

  @override
  String get removeBlockedTooltip =>
      'Сначала сохраните или сбросьте ожидающие изменения инвентаря — добавление или удаление нужно сохранять отдельно';

  @override
  String get removeItemFromInventory => 'Убрать предмет из инвентаря';

  @override
  String get progressionLockedBody =>
      'Для данных прогресса нужны декодированные приватные данные из кодека.';

  @override
  String get progressionNeedsTyped =>
      'Для структурированных данных прогресса нужно полностью декодированное сохранение с подтверждённым типизированным разбором.';

  @override
  String get sectionQuests => 'Задания';

  @override
  String get sectionKnowledge => 'Знания';

  @override
  String get sectionEvents => 'События';

  @override
  String get firstPage => 'Первая страница';

  @override
  String get previousPage => 'Предыдущая страница';

  @override
  String get nextPage => 'Следующая страница';

  @override
  String get lastPage => 'Последняя страница';

  @override
  String pageOfPages(int page, int total) {
    return 'Страница $page / $total';
  }

  @override
  String rangeOfTotal(int first, int last, int total) {
    return '$first–$last из $total';
  }

  @override
  String get perPage => 'На странице:';

  @override
  String get resetQuestChanges => 'Сбросить изменения заданий';

  @override
  String get searchQuests => 'Поиск заданий';

  @override
  String get allGroups => 'Все группы';

  @override
  String groupWithCount(String group, Object count) {
    return '$group ($count)';
  }

  @override
  String stateLabelWithCount(String label, int count) {
    return '$label $count';
  }

  @override
  String get questStateNone => 'Нет';

  @override
  String get questStateAvailable => 'Доступно';

  @override
  String get questStateRunning => 'В процессе';

  @override
  String get questStateSucceeded => 'Выполнено';

  @override
  String get questStateFailed => 'Провалено';

  @override
  String get questStateUnknown => 'неизвестно';

  @override
  String get dialogKnowledge => 'Знания из диалогов';

  @override
  String get resetKnowledgeChanges => 'Сбросить изменения знаний';

  @override
  String get addNpc => 'Добавить NPC';

  @override
  String get searchNpcs => 'Поиск NPC';

  @override
  String entriesForCharacter(String name) {
    return 'Записи — $name';
  }

  @override
  String get selectNpcToSeeEntries => 'Выберите NPC, чтобы увидеть записи';

  @override
  String get addKnowledgeEntry => 'Добавить запись знаний';

  @override
  String get browseCatalog => 'Просмотреть каталог';

  @override
  String get alreadyExistsForCharacter => 'Уже существует для этого персонажа.';

  @override
  String get alreadyInPendingChanges => 'Уже есть в ожидающих изменениях.';

  @override
  String duplicateCheckFailed(String error) {
    return 'Не удалось проверить дубликаты — повторите попытку: $error';
  }

  @override
  String pendingAddsCount(int count) {
    return 'Ожидающие добавления ($count)';
  }

  @override
  String get undoAdd => 'Отменить добавление';

  @override
  String get undoRemove => 'Отменить удаление';

  @override
  String get removeEntry => 'Удалить запись';

  @override
  String get selectNpcFromList => 'Выберите NPC из списка';

  @override
  String characterWithCount(String name, int count) {
    return '$name ($count)';
  }

  @override
  String get memoryEvents => 'События памяти';

  @override
  String get searchCharacters => 'Поиск персонажей';

  @override
  String eventsForCharacter(String name) {
    return 'События — $name';
  }

  @override
  String get selectCharacterToSeeEvents =>
      'Выберите персонажа, чтобы увидеть события';

  @override
  String get noTags => '(нет тегов)';

  @override
  String eventSubtitle(String time, String affected) {
    return 't=$timeс  $affected';
  }

  @override
  String get removeEvent => 'Удалить событие';

  @override
  String get removeMemoryEventTitle => 'Удалить событие памяти?';

  @override
  String get removeMemoryEventBody =>
      'Удалить это событие памяти? Сначала будет создана резервная копия.';

  @override
  String get duplicateEvent => 'Дублировать событие';

  @override
  String get duplicateMemoryEventTitle => 'Дублировать событие памяти?';

  @override
  String get duplicateMemoryEventBody =>
      'Дублировать это событие памяти? Сначала будет создана резервная копия.';

  @override
  String get selectCharacterFromList => 'Выберите персонажа из списка';

  @override
  String get allDataLockedBody =>
      'Для полного обозревателя свойств нужны декодированные приватные данные из кодека.';

  @override
  String get allDataDescription =>
      'Ищите любое типизированное свойство по имени или пути. Числа, строки, перечисления и пути объектов доступны для редактирования; структуры пока отображаются только для чтения.';

  @override
  String get searchPropertiesLabel =>
      'Поиск свойств (пусто = показать всё) — например, Health, GameTime';

  @override
  String get decodingSaveTitle => 'Декодирование сохранения…';

  @override
  String get decodingSaveBody =>
      'Декодирование всех приватных данных для первого поиска. Это выполняется один раз для каждого сохранения, после чего поиск становится мгновенным.';

  @override
  String get searchTheSaveTitle => 'Поиск по сохранению';

  @override
  String get searchTheSaveBody =>
      'Введите имя свойства и нажмите Enter. Оставьте поле пустым, чтобы показать всё.';

  @override
  String get searchFailedTitle => 'Поиск не удался';

  @override
  String get noMatchesTitle => 'Совпадений нет';

  @override
  String get noMatchesBody =>
      'Ни один путь свойства не содержал всех этих терминов.';

  @override
  String get value => 'Значение';

  @override
  String get backupsTitle => 'Резервные копии';

  @override
  String get refreshBackups => 'Обновить резервные копии';

  @override
  String get noBackupsTitle => 'Резервных копий нет';

  @override
  String get noBackupsBody =>
      'При редактировании сохранений рядом с выбранным слотом создаются файлы резервных копий.';

  @override
  String get slotBackups => 'Копии слота';

  @override
  String get profileBackups => 'Копии профиля';

  @override
  String get backupFactName => 'Имя';

  @override
  String get backupFactSlot => 'Слот';

  @override
  String get backupFactCreated => 'Создано';

  @override
  String get backupFactSize => 'Размер';

  @override
  String get backupFactStatus => 'Состояние';

  @override
  String get backupFactSha1 => 'SHA-1';

  @override
  String restoreBackupTooltip(String fileName) {
    return 'Восстановить $fileName';
  }

  @override
  String get appearanceTitle => 'Внешний вид';

  @override
  String get theme => 'Тема';

  @override
  String get themeLight => 'Светлая';

  @override
  String get themeDark => 'Тёмная';

  @override
  String get themeSystem => 'Системная';

  @override
  String get uiScale => 'Масштаб интерфейса';

  @override
  String get resetZoomTooltip => 'Сбросить масштаб (Ctrl+0)';

  @override
  String get zoomTip =>
      'Совет: Ctrl + / Ctrl - меняет масштаб в любом месте приложения.';

  @override
  String get language => 'Язык';

  @override
  String get updatesTitle => 'Обновления';

  @override
  String get checkForUpdatesAutomatically =>
      'Проверять обновления автоматически';

  @override
  String get checkForUpdatesNow => 'Проверить обновления сейчас';

  @override
  String get updatesPortableNotice =>
      'Портативная версия открывает страницу загрузки в браузере. Замените имеющиеся файлы новым загруженным.';

  @override
  String get updateAvailableTitle => 'Доступно обновление';

  @override
  String updateAvailableMessage(Object version, Object current) {
    return 'Доступна версия $version. У вас $current.';
  }

  @override
  String get updateDownload => 'Скачать';

  @override
  String get updateLater => 'Позже';

  @override
  String get updateUpToDate => 'У вас установлена последняя версия.';

  @override
  String get updateCheckFailed =>
      'Не удалось проверить обновления. Повторите попытку позже.';

  @override
  String get gameTextTitle => 'Текст игры';

  @override
  String gameTextExtractedWithCounts(int ids, int languages) {
    return 'Извлечено: $ids идентификаторов на $languages языках.';
  }

  @override
  String get gameTextExtracted => 'Локализованный текст игры извлечён.';

  @override
  String get gameTextNotExtracted =>
      'Локализованный текст игры ещё не извлечён.';

  @override
  String get extracting => 'Извлечение…';

  @override
  String get extractRefreshLocalizedText =>
      'Извлечь / обновить локализованный текст';

  @override
  String get extractLocalizedTextTitle => 'Извлечь локализованный текст игры?';

  @override
  String get extractLocalizedTextBody =>
      'Локализованный текст игры ещё не извлечён. Извлечь его сейчас из вашей установки игры? (необязательно)';

  @override
  String get notNow => 'Не сейчас';

  @override
  String get extract => 'Извлечь';

  @override
  String get extractionComplete => 'Извлечение завершено';

  @override
  String get extractionFailed => 'Извлечение не удалось';

  @override
  String get localizationCacheFileType => 'Кеш локализации';

  @override
  String get savegameDirectoryTitle => 'Папка сохранений';

  @override
  String get folder => 'Папка';

  @override
  String get codecTitle => 'Кодек';

  @override
  String get check => 'Проверить';

  @override
  String get roundtrip => 'Полный цикл';

  @override
  String get noCodecStatus => 'Нет состояния кодека';

  @override
  String get codecReady => 'Кодек готов';

  @override
  String get codecReadOnly => 'Кодек только для чтения';

  @override
  String get codecUnavailable => 'Кодек недоступен';

  @override
  String get details => 'Подробности';

  @override
  String codecStatusLine(String status) {
    return 'Состояние: $status';
  }

  @override
  String codecCapabilityLine(String decompress, String compress) {
    return 'Распаковка: $decompress | Сжатие: $compress';
  }

  @override
  String codecBackendLine(String backend) {
    return 'Бэкенд: $backend';
  }

  @override
  String get yes => 'да';

  @override
  String get no => 'нет';

  @override
  String get aboutSubtitle => 'Редактор сохранений Gothic Remake';

  @override
  String aboutVersion(String version, String sha) {
    return 'Версия $version ($sha)';
  }

  @override
  String get aboutCopyright => '© 2026 участники проекта goresave';

  @override
  String get aboutLicense => 'Распространяется по лицензии MIT.';

  @override
  String difficultyTitle(String profile) {
    return 'Сложность — $profile';
  }

  @override
  String get difficultyNoProfile => 'Нет профиля';

  @override
  String get difficultyNoDifficulty => 'Нет сложности';

  @override
  String get difficultyLabel => 'Сложность';

  @override
  String get difficultyTooltipNoProfile => 'Профиль не выбран';

  @override
  String get difficultyTooltipEdit => 'Изменить сложность для этого профиля';

  @override
  String get difficultyTooltipNoEditable =>
      'У этого профиля нет редактируемой сложности';

  @override
  String get preset => 'Предустановка';

  @override
  String get presetNovice => 'Новичок';

  @override
  String get presetGothic => 'Gothic';

  @override
  String get presetHard => 'Сложный';

  @override
  String get presetCustom => 'Свой';

  @override
  String unrecognisedPreset(Object preset) {
    return 'Сохранённая предустановка не распознана ($preset). Вы всё ещё можете сохранить изменения Помощника боя / Перманентной смерти или выбрать предустановку выше, чтобы перезаписать её.';
  }

  @override
  String get closeCombatFlowHelper => 'Помощник ближнего боя';

  @override
  String get permadeath => 'Перманентная смерть';

  @override
  String get notAvailableOnNovice => 'Недоступно на уровне «Новичок»';

  @override
  String get levelCombat => 'Бой';

  @override
  String get levelResources => 'Ресурсы';

  @override
  String get levelProgression => 'Прогресс';

  @override
  String get difficultyAppliesToAllSaves =>
      'Сложность применяется ко всем сохранениям этого профиля.';

  @override
  String get savingDifficultyFailed => 'Не удалось сохранить сложность.';

  @override
  String get addItemDialogTitle => 'Добавить предмет';

  @override
  String get searchItems => 'Поиск предметов';

  @override
  String failedToLoadCatalog(String error) {
    return 'Не удалось загрузить каталог: $error';
  }

  @override
  String get noItemsAvailableToAdd => 'Нет предметов для добавления';

  @override
  String get noItemsMatch => 'Нет подходящих предметов';

  @override
  String get countMustBeAtLeast1 => 'Должно быть ≥ 1';

  @override
  String countMustBeAtMost(int max) {
    return 'Должно быть ≤ $max';
  }

  @override
  String get addNpcDialogTitle => 'Добавить NPC';

  @override
  String get noNpcsAvailableToAdd => 'Нет NPC для добавления';

  @override
  String get noNpcsMatch => 'Нет подходящих NPC';

  @override
  String get categoryAll => 'Все';

  @override
  String allWithCount(int count) {
    return 'Все ($count)';
  }

  @override
  String get addKnowledgeEntryDialogTitle => 'Добавить запись знаний';

  @override
  String get searchEntries => 'Поиск записей';

  @override
  String get noKnowledgeEntriesAvailableToAdd =>
      'Нет записей знаний для добавления';

  @override
  String get noEntriesMatch => 'Нет подходящих записей';

  @override
  String get heroGroupMainStats => 'Основные характеристики';

  @override
  String get heroGroupCombatSkills => 'Боевые навыки';

  @override
  String get heroGroupResistances => 'Сопротивления';

  @override
  String get heroGroupThieving => 'Воровство';

  @override
  String get heroGroupAdvanced => 'Дополнительно';

  @override
  String get heroEntryHeroTransform => 'Положение героя';

  @override
  String attributeEmpty(String name) {
    return '$name не заполнено — введите значение или восстановите исходное перед сохранением.';
  }

  @override
  String attributeInvalidNumber(String name, String text) {
    return 'Недопустимое число для $name: «$text»';
  }

  @override
  String get loadingEditorData => 'Загрузка данных редактора';

  @override
  String localizedTextExtractedCount(int idCount, int languageCount) {
    return 'Извлечено $idCount идентификаторов на $languageCount языках';
  }
}
