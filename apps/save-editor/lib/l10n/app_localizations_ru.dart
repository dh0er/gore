// ignore: unused_import
import 'package:intl/intl.dart' as intl;
import 'app_localizations.dart';

// ignore_for_file: type=lint

/// The translations for Russian (`ru`).
class AppLocalizationsRu extends AppLocalizations {
  AppLocalizationsRu([String locale = 'ru']) : super(locale);

  @override
  String get debugSectionTitle => 'Дополнительно (отладка)';

  @override
  String get debugSectionSubtitle =>
      'Диагностика и необработанные данные для отчётов об ошибках';

  @override
  String get showObjectIdsTitle => 'Показывать дополнительные технические ID';

  @override
  String get showObjectIdsSubtitle =>
      'Показывает технические ID предметов, знаний диалогов, заданий и потерянных персонажей. ID NPC отображаются всегда.';

  @override
  String get storyStateSidebar => 'Состояние сюжета';

  @override
  String get storyStateDescription =>
      'Авторитетный каталог постоянных состояний сюжета, объявленных в поставляемых с игрой скриптах. Сохранённые записи показывают исходное значение; отсутствующие в сохранении поля каталога отмечены как незаданные. Метки времени, объявленные в коде, показываются как игровое время; остальные целые числа могут быть логическими значениями, счётчиками или многоуровневыми состояниями.';

  @override
  String get storyStateReadOnly =>
      'Только для чтения, пока не подтверждены смысл значений в скриптах и безопасная запись карты. Связанный текст глоссария даёт контекст, а не прямой перевод технического ID.';

  @override
  String get storyStateStructureReadOnly =>
      'Не удалось однозначно и безопасно определить структуру StoryPropertyValues в этом сохранении. Значения сюжета останутся доступными только для чтения в этом сохранении.';

  @override
  String get storyStateSearch => 'Поиск по состоянию сюжета';

  @override
  String storyStateValuesCount(int shown, int total) {
    return '$shown из $total сюжетных значений';
  }

  @override
  String get storyStateInteger => 'Целое число';

  @override
  String get storyStateTimeMarker => 'Метка времени';

  @override
  String get storyStateChapter => 'Глава';

  @override
  String get storyStateUnknown => 'Неизвестный исходный тип';

  @override
  String get storyStateUnknownDetail =>
      'Этого сохранённого ID нет в текущем каталоге скриптов (например, он добавлен модом или новой версией игры). В сохранении значение имеет формат int32, но его смысл не угадывается.';

  @override
  String get storyStateStored => 'Сохранено';

  @override
  String get storyStateUnset => 'Не задано';

  @override
  String get storyStateUnsetDetail =>
      'Это поле каталога не сериализовано в сохранении, поэтому игра использует незаданное состояние или значение по умолчанию.';

  @override
  String get storyStateRawValue => 'Исходное значение';

  @override
  String storyStateElapsed(String duration) {
    return 'Прошло на момент сохранения: $duration';
  }

  @override
  String storyStateAhead(String duration) {
    return 'В будущем на момент сохранения: $duration';
  }

  @override
  String storyStateDurationDays(int days, String time) {
    String _temp0 = intl.Intl.pluralLogic(
      days,
      locale: localeName,
      other: '$days дня',
      many: '$days дней',
      few: '$days дня',
      one: '1 день',
    );
    return '$_temp0 $time';
  }

  @override
  String get storyStateRelatedGlossary => 'Связанная запись глоссария';

  @override
  String get storyStateTechnicalPath => 'Технический путь';

  @override
  String get storyStateEditingGuidance =>
      'Каждую запись можно редактировать во всём диапазоне знакового int32. Флаги и предлагаемые значения, основанные на скриптах, служат лишь подсказками; исходное значение всегда можно ввести вручную. Изменение состояния сюжета может пропустить переходы диалогов, заданий или мира, поэтому сохраняйте такие правки осознанно. Резервная копия создаётся автоматически.';

  @override
  String get storyStatePending => 'Ожидает';

  @override
  String storyStatePendingValue(String value) {
    return 'Будет сохранено как $value';
  }

  @override
  String get storyStatePendingRemoval => 'Будет удалено из сохранения';

  @override
  String get storyStateEditValue => 'Изменить значение';

  @override
  String get storyStateSetValue => 'Задать значение';

  @override
  String get storyStateRemoveValue => 'Удалить из сохранения';

  @override
  String get storyStateUndoChange => 'Отменить изменение сюжета';

  @override
  String get storyStateResetChanges => 'Сбросить изменения сюжета';

  @override
  String storyStateDialogTitle(String id) {
    return 'Изменить $id';
  }

  @override
  String get storyStateRawInput => 'Знаковое значение int32';

  @override
  String get storyStateInvalidInt32 =>
      'Введите целое число от -2147483648 до 2147483647.';

  @override
  String get storyStateQueueChange => 'Добавить изменение в очередь';

  @override
  String storyStateSuggestedValues(String values) {
    return 'Значения, подтверждённые поставляемыми скриптами: $values';
  }

  @override
  String get storyStateSuggestionsNotLimits =>
      'Предлагаемые значения не ограничивают проверку; нативный код, моды или более поздние версии игры могут использовать другие значения.';

  @override
  String get storyStateUseCurrentTime =>
      'Использовать текущее время сохранения';

  @override
  String get storyStateStructuredTime => 'День / время';

  @override
  String get storyStateRawMode => 'Исходный int32';

  @override
  String get storyStateChapterWarning =>
      'Изменение одного лишь номера главы не синхронизирует задания, NPC, инвентарь и состояние мира.';

  @override
  String get storyStateDormantWarning =>
      'В кэше поставляемых скриптов не найдено активного чтения или записи этого поля. Оно может быть устаревшим, управляться нативным кодом или быть зарезервированным.';

  @override
  String get storyStateReadOnlySourceWarning =>
      'Поставляемые скрипты читают это поле, но не записывают его. Оно по-прежнему может управляться нативным кодом.';

  @override
  String get storyStateUnknownEditWarning =>
      'У этого ID из мода или более новой версии нет встроенного описания семантики исходного кода. Изменяйте только его исходное значение int32.';

  @override
  String storyStateIntegerKind(String kind) {
    String _temp0 = intl.Intl.selectLogic(kind, {
      'binaryFlag': 'Двоичный флаг',
      'finiteState': 'Многоуровневое значение',
      'counterOrScore': 'Счётчик / результат',
      'calendarDay': 'Календарный день',
      'derivedOrOpaqueInteger': 'Производное / непрозрачное целое число',
      'readOnlyInSourceInteger': 'Только чтение в поставляемых скриптах',
      'dormantOrLegacyInteger': 'Не используется в поставляемых скриптах',
      'other': 'Целое число',
    });
    return '$_temp0';
  }

  @override
  String get storyStateZeroVsUnset =>
      'Сохранённый 0 и отсутствие записи в карте — разные состояния файла. Команда «Удалить из сохранения» восстанавливает состояние конструктора или состояние по умолчанию.';

  @override
  String get appTitle => 'GORE Save Editor';

  @override
  String get appLogoSemanticLabel => 'Логотип GORE Save Editor';

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
  String get tabAttribute => 'Атрибуты';

  @override
  String get heroGroupSkills => 'Навыки';

  @override
  String get skillsNoneBody => 'Для этого персонажа навыки не найдены.';

  @override
  String get skillsUnavailableBody =>
      'Навыки нельзя изменить в этом сохранении — у героя нет данных эффектов для изменения.';

  @override
  String get skillNotLearned => 'Не изучен';

  @override
  String get skillLearn => 'Изучить';

  @override
  String get skillActionLearn => 'изучить';

  @override
  String get skillActionUnlearn => 'забыть';

  @override
  String get skillTierUntrained => 'Не обучен';

  @override
  String get skillTierBeginner => 'Новичок';

  @override
  String get skillTierTrained => 'Обучен';

  @override
  String get skillTierMaster => 'Мастер';

  @override
  String get skillTierNovice => 'Новичок';

  @override
  String get skillTierAmateur => 'Любитель (Круг 0)';

  @override
  String get skillTierLearned => 'Изучен';

  @override
  String skillTierCircle(int n) {
    return 'Круг $n';
  }

  @override
  String get skillHintBlacksmith1H => 'Одноручное оружие';

  @override
  String get skillHintBlacksmith2H => 'Двуручное оружие';

  @override
  String get skillCategoryCombat => 'Бой';

  @override
  String get skillCategoryCrafting => 'Ремесло';

  @override
  String get skillCategoryHunting => 'Охота';

  @override
  String get skillCategoryLanguage => 'Язык';

  @override
  String get skillCategoryMagic => 'Магия';

  @override
  String get skillCategoryMovement => 'Передвижение';

  @override
  String get skillCategoryThievery => 'Воровство';

  @override
  String get skillNameOneHanded => 'Одноручное оружие';

  @override
  String get skillNameTwoHanded => 'Двуручное оружие';

  @override
  String get skillNameFists => 'Кулаки';

  @override
  String get skillNameBow => 'Луки';

  @override
  String get skillNameCrossbow => 'Арбалеты';

  @override
  String get skillNameLockpicking => 'Взлом замков';

  @override
  String get skillNamePickpocketing => 'Карманные кражи';

  @override
  String get skillNameTakeOrgans => 'Извлечение органов';

  @override
  String get skillNameBreakTeeth => 'Извлечение зубов';

  @override
  String get skillNameTakeClaws => 'Извлечение когтей';

  @override
  String get skillNameSkinFur => 'Добыча меха';

  @override
  String get skillNameSkin => 'Снятие шкуры';

  @override
  String get skillNameTakeFins => 'Извлечение плавников';

  @override
  String get skillNameTakeStingers => 'Извлечение жала';

  @override
  String get skillNameTakeSecretion => 'Извлечение жвал';

  @override
  String get skillNameTakeSkullPlates => 'Извлечение черепной пластины';

  @override
  String get skillNameSkinSwampshark => 'Снятие шкуры болотожора';

  @override
  String get skillNameTakeMinecrawlerPlates => 'Извлечение пластин';

  @override
  String get skillNameTakeScutes => 'Снятие пластин';

  @override
  String get skillNameTakeUluMulu => 'Получение Улу-Мулу';

  @override
  String get skillNameOrcWeapons => 'Оружие орков';

  @override
  String get skillNameMining => 'Добыча руды';

  @override
  String get skillNameDiving => 'Ныряние';

  @override
  String get skillNameTakeMinecrawlerMandibles => 'Извлечение жвал';

  @override
  String get skillNameTakeShadowbeastHorn => 'Извлечение рога (Shadowbeast)';

  @override
  String get skillNameTakeSpines => 'Извлечение хребта';

  @override
  String get skillNameBreakSwampsharkTeeth => 'Извлечение зубов болотожора';

  @override
  String get skillNameTakeFireTongue => 'Извлечение огненного языка';

  @override
  String get skillNameTakeTrollHorn => 'Извлечение рога (Troll)';

  @override
  String get skillNameAcrobatics => 'Акробатика';

  @override
  String get skillNameWallClimbing => 'Лазание';

  @override
  String get skillNameRiding => 'Езда на падальщике';

  @override
  String get skillNameSneaking => 'Подкрадывание';

  @override
  String get skillNameAlchemy => 'Алхимия';

  @override
  String get skillNameRuneInscription => 'Создание заклинаний';

  @override
  String get skillNameBlacksmithing => 'Кузнечное дело';

  @override
  String get skillNameMagicCircle => 'Круг магии';

  @override
  String get skillNameOrcish => 'Орочий язык';

  @override
  String get tabInventory => 'Инвентарь';

  @override
  String get tabWorld => 'Мир';

  @override
  String get tabCharacters => 'Персонажи';

  @override
  String get characterNoActorBody =>
      'У этого персонажа нет актёра в мире, поэтому нет атрибутов, инвентаря или событий.';

  @override
  String get characterNoEventsBody => 'Для этого персонажа нет событий.';

  @override
  String get characterOrphanGroup => 'Прочие';

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
  String get armorUpgradesLabel => 'Улучшения';

  @override
  String get browse => 'Обзор';

  @override
  String get noSavFilesFound => 'Файлы .sav не найдены';

  @override
  String get profile => 'Профиль';

  @override
  String get otherSaves => 'Другие сохранения';

  @override
  String profileWithSaves(String name, int count) {
    return '$name (сохранений: $count)';
  }

  @override
  String get switchProfile => 'Сменить профиль';

  @override
  String get openSaveFile => 'Открыть файл';

  @override
  String get externalSave => 'Сохранение открыто извне';

  @override
  String get saveProfileTitle => 'Профиль сохранения';

  @override
  String get saveProfileDescription =>
      'Назначьте это сохранение другому игровому профилю. Резервные копии сохранения и индекса профилей создаются вместе.';

  @override
  String get saveProfileExternalHint =>
      'Выберите профиль, чтобы импортировать этот файл в папку сохранений игры и зарегистрировать его там. Исходный файл не изменится.';

  @override
  String get saveProfileNoProfiles =>
      'В PersistentDataList.sav не найдены редактируемые игровые профили.';

  @override
  String get saveProfileSelect => 'Выберите профиль';

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
  String bytesValue(String count) {
    return '$count байт';
  }

  @override
  String get inspectionJsonTitle => 'JSON проверки';

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
  String get gameTimeTitle => 'Время игры';

  @override
  String get gameTimeDay => 'День';

  @override
  String get gameTimeHours => 'Часы';

  @override
  String get gameTimeMinutes => 'Минуты';

  @override
  String get gameTimeSeconds => 'Секунды';

  @override
  String gameTimeTotal(int seconds) {
    return '= всего $seconds с';
  }

  @override
  String get gameTimeInvalid =>
      'Введите целые числа: день ≥ 0, часы 0–23, минуты и секунды 0–59.';

  @override
  String get required => 'Обязательно';

  @override
  String get playerLockedBody =>
      'Для редактирования приватных данных персонажа нужен кодек с поддержкой сжатия.';

  @override
  String get heroTransform => 'Позиция';

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
  String get spawnPositionSection => 'Точка появления (справочно)';

  @override
  String get resetToSpawnPosition => 'Вернуть к точке появления';

  @override
  String get positionOutOfRange =>
      'Значение должно быть от −10 000 000 до 10 000 000';

  @override
  String get positionNotEditable =>
      'Сохранённую позицию этого персонажа не удалось прочитать, поэтому её нельзя изменить.';

  @override
  String get positionNeverPlaced =>
      'Этот персонаж ни разу не был размещён в мире (позиция 0, 0, 0) — игра может игнорировать сохранённую позицию.';

  @override
  String get npcStayInPlace => 'Отключить его распорядок дня';

  @override
  String get npcStayInPlaceHint => 'Тогда он останется на месте.';

  @override
  String get npcStayInPlaceLocked =>
      'Его исходный распорядок дня не записан, поэтому отменить это больше нельзя.';

  @override
  String get npcUndoPlacement => 'Отменить перемещение';

  @override
  String get npcUndoPlacementStale =>
      'В сохранении больше нет того, что записало это перемещение, поэтому восстановление отбросило бы всё, что произошло с тех пор.';

  @override
  String get positionNotReadable =>
      'Сохранённую позицию этого персонажа не удалось прочитать.';

  @override
  String get npcPositionReadOnly =>
      'Игра восстанавливает позицию NPC из уровня, а не из сохранения, поэтому эти значения можно прочитать, но не изменить.';

  @override
  String get pickLocation => 'Выбрать место…';

  @override
  String get pickLocationDialogTitle => 'Выбор места';

  @override
  String get applySpotRotation => 'Также применить ориентацию точки';

  @override
  String get locationAreaOther => 'Прочее';

  @override
  String get locationAreaCavalornValley => 'Долина Кавалорна';

  @override
  String get locationAreaEastForest => 'Восточный лес';

  @override
  String get locationAreaFogTower => 'Туманная башня';

  @override
  String get locationAreaIllegalWeedMixers => 'Подпольные травники';

  @override
  String get locationAreaOrcArena => 'Орочья арена';

  @override
  String get locationAreaOrcGraveyard => 'Орочий склеп';

  @override
  String get locationAreaShipwreck => 'Место кораблекрушения';

  @override
  String get locationAreaTundra => 'Тундра';

  @override
  String get locationCatalogUnavailable => 'Не удалось загрузить каталог мест.';

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
  String get attributeBaseValue => 'Базовое значение';

  @override
  String get attributeCurrentValue => 'Текущее значение';

  @override
  String get inventoryTitle => 'Инвентарь';

  @override
  String get inventoryEmpty => 'Этот инвентарь пуст.';

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
  String get resetInventoryButton => 'Сбросить инвентарь';

  @override
  String get resetInventoryTooltipDefault =>
      'Заменить этот инвентарь начальным инвентарём';

  @override
  String get resetInventoryTooltipBlocked =>
      'Сначала сохраните или отмените ожидающие изменения инвентаря';

  @override
  String get pendingResetTitle => 'Восстановить начальный инвентарь';

  @override
  String pendingResetSubtitle(String level) {
    return 'Уровень ресурсов: $level';
  }

  @override
  String get cancelPendingReset => 'Отменить сброс';

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
  String get npcStatusRowLabel => 'Состояние';

  @override
  String get npcStatusAlive => 'жив';

  @override
  String get npcStatusDead => 'мёртв';

  @override
  String get npcRelationshipRowLabel => 'Отношение';

  @override
  String get npcRelationshipUnavailable => 'Состояние отношения недоступно';

  @override
  String get npcRelationshipAutomatic => 'Вычисляется игрой';

  @override
  String get npcRelationshipAutomaticHint =>
      'Постоянное переопределение не сохранено. Игра учитывает правила гильдий, сюжета, областей и преступлений.';

  @override
  String get npcRelationshipStoredHint =>
      'Сохранено как постоянное отношение NPC к игроку. Правила гильдий, сюжета, областей и преступлений всё ещё могут изменить фактическое отношение в игре.';

  @override
  String get npcRelationshipFriend => 'Друг';

  @override
  String get npcRelationshipNeutral => 'Нейтральный';

  @override
  String get npcRelationshipEnemy => 'Враг';

  @override
  String npcRelationshipPending(String relationship) {
    return 'После сохранения: $relationship';
  }

  @override
  String npcStateHp(String hp, String maxHp) {
    return 'ОЗ $hp / $maxHp';
  }

  @override
  String get npcReviveButton => 'Воскресить';

  @override
  String get npcReviveQueued => 'Будет воскрешён при сохранении';

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
  String get memoryEventRemovalQueued =>
      'Удаление события поставлено в очередь — нажмите «Сохранить», чтобы применить.';

  @override
  String get duplicateEvent => 'Дублировать событие';

  @override
  String get duplicateMemoryEventTitle => 'Дублировать событие памяти?';

  @override
  String get duplicateMemoryEventBody =>
      'Дублировать это событие памяти? Сначала будет создана резервная копия.';

  @override
  String get memoryEventDuplicationQueued =>
      'Дублирование события поставлено в очередь — нажмите «Сохранить», чтобы применить.';

  @override
  String get selectCharacterFromList => 'Выберите персонажа из списка';

  @override
  String get factionsSidebar => 'Фракции';

  @override
  String get factionsForgiveButton => 'Простить';

  @override
  String get factionHostile => 'Враждебно';

  @override
  String get factionFriendly => 'Дружелюбно';

  @override
  String crimeMurder(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count убийства',
      many: '$count убийств',
      few: '$count убийства',
      one: '$count убийство',
    );
    return '$_temp0';
  }

  @override
  String crimeAssault(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count нападения',
      many: '$count нападений',
      few: '$count нападения',
      one: '$count нападение',
    );
    return '$_temp0';
  }

  @override
  String crimeTheft(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count кражи',
      many: '$count краж',
      few: '$count кражи',
      one: '$count кража',
    );
    return '$_temp0';
  }

  @override
  String crimeTrespassing(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count проникновения',
      many: '$count проникновений',
      few: '$count проникновения',
      one: '$count проникновение',
    );
    return '$_temp0';
  }

  @override
  String crimeThreat(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count угрозы',
      many: '$count угроз',
      few: '$count угрозы',
      one: '$count угроза',
    );
    return '$_temp0';
  }

  @override
  String crimeOther(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count иного преступления',
      many: '$count иных преступлений',
      few: '$count иных преступления',
      one: '$count иное преступление',
    );
    return '$_temp0';
  }

  @override
  String get factionsForgiveQueued => 'прощается…';

  @override
  String get factionsEmpty => 'Нет незакрытых преступлений против фракций.';

  @override
  String get factionGuildOldCamp => 'Старый лагерь';

  @override
  String get factionGuildNewCamp => 'Новый лагерь';

  @override
  String get factionGuildSwampCamp => 'Болотный лагерь';

  @override
  String get factionGuildOther => 'Прочие/отдельные лица';

  @override
  String get allDataLockedBody =>
      'Полный просмотр данных сейчас доступен для сохранений в формате GSAV.';

  @override
  String get allDataDescription =>
      'Просматривайте метаданные GSAV и все типизированные узлы разделов PUBLIC и PRIVATE. Безопасные скалярные значения и нативные структуры можно редактировать; контейнеры и необработанные байты также отображаются.';

  @override
  String get allDataEditable => 'Редактируемые';

  @override
  String get allDataReadOnly => 'Только для чтения';

  @override
  String get allDataType => 'Тип';

  @override
  String get allDataScalars => 'Скалярные значения';

  @override
  String get allDataStructs => 'Структуры';

  @override
  String get allDataContainers => 'Контейнеры';

  @override
  String get allDataOpaque => 'Необработанные байты';

  @override
  String get allDataNodes => 'Узлы';

  @override
  String allDataChildren(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count дочернего узла',
      many: '$count дочерних узлов',
      few: '$count дочерних узла',
      one: '$count дочерний узел',
    );
    return '$_temp0';
  }

  @override
  String get allDataPending => 'Ожидает сохранения';

  @override
  String get allDataTagInputHint => 'Теги через запятую или с новой строки';

  @override
  String allDataTypedSource(String source) {
    return 'Типизированные данные: $source';
  }

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
  String aboutVersion(String version, String sha) {
    return 'Версия $version ($sha)';
  }

  @override
  String get aboutCopyright => '© 2026 участники проекта GORE';

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
  String get presetNovice => 'Низкая';

  @override
  String get presetGothic => 'Готическая';

  @override
  String get presetHard => 'Высокая';

  @override
  String get presetCustom => 'Свои настройки';

  @override
  String unrecognisedPreset(Object preset) {
    return 'Сохранённая предустановка не распознана ($preset). Вы всё ещё можете сохранить изменения Помощника боя / Перманентной смерти или выбрать предустановку выше, чтобы перезаписать её.';
  }

  @override
  String get closeCombatFlowHelper => 'Помощь в ближнем бою';

  @override
  String get permadeath => 'Необратимая смерть';

  @override
  String get notAvailableOnNovice => 'Недоступно на уровне «Новичок»';

  @override
  String get levelCombat => 'Сложность боя';

  @override
  String get levelResources => 'Доступность ресурсов';

  @override
  String get levelProgression => 'Сложность прогресса';

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
  String get heroGroupCombatMovement => 'Бой и передвижение';

  @override
  String get heroGroupResistances => 'Сопротивления';

  @override
  String get heroGroupThieving => 'Воровство';

  @override
  String get heroGroupAdvanced => 'Дополнительно';

  @override
  String get heroGroupDiving => 'Ныряние';

  @override
  String get heroGroupSleep => 'Сон и отдых';

  @override
  String get heroGroupIntoxication => 'Опьянение';

  @override
  String get heroEntryHeroTransform => 'Позиция';

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
  String savingProgress(int done, int total) {
    return 'Сохранение… $done из $total';
  }

  @override
  String localizedTextExtractedCount(int idCount, int languageCount) {
    return 'Извлечено $idCount идентификаторов на $languageCount языках';
  }

  @override
  String get skillSmithing1H => 'Кузнечное дело — одноручное';

  @override
  String get skillSmithing2H => 'Кузнечное дело — двуручное';

  @override
  String get skillCircleNovice => 'Маг-послушник';

  @override
  String get skillCircle1 => 'Первый круг магии';

  @override
  String get skillCircle2 => 'Второй круг магии';

  @override
  String get skillCircle3 => 'Третий круг магии';

  @override
  String get skillCircle4 => 'Четвёртый круг магии';

  @override
  String get skillCircle5 => 'Пятый круг магии';

  @override
  String get skillCircle6 => 'Шестой круг магии';

  @override
  String get sectionGlossary => 'Глоссарий';

  @override
  String get glossarySearch => 'Поиск в глоссарии';

  @override
  String get glossaryOldCamp => 'Старый лагерь';

  @override
  String get glossaryNewCamp => 'Новый лагерь';

  @override
  String get glossarySwampCamp => 'Болотный лагерь';

  @override
  String get glossaryOutsiders => 'Чужаки';

  @override
  String get glossaryCreatures => 'Существа';

  @override
  String get glossaryLocations => 'Места';

  @override
  String get glossaryFilterLabel => 'Фильтр';

  @override
  String get glossaryFilterTraders => 'Торговцы';

  @override
  String get glossaryFilterTeachers => 'Учителя';

  @override
  String get glossaryFilterArmorers => 'Бронники';

  @override
  String get glossaryFilterHostile => 'Враждебные';

  @override
  String get glossaryRelationshipFilterNote =>
      'Показывает постоянные враждебные отношения, сохранённые в файле. Динамические отношения гильдий, сюжета, областей и преступлений вычисляются только в игре.';

  @override
  String get glossaryFilterDead => 'Мёртвые';

  @override
  String get glossaryAddEntry => 'Добавить запись в глоссарий';

  @override
  String get glossaryAddTitle => 'Добавить запись в глоссарий';

  @override
  String get glossaryResetChanges => 'Сбросить изменения глоссария';

  @override
  String get glossaryNoVisibleEntries =>
      'В этом представлении нет подходящих видимых записей глоссария.';

  @override
  String get glossaryNoHiddenEntries =>
      'Все доступные записи уже отображаются.';

  @override
  String get glossaryNoMatch => 'Подходящих записей глоссария нет.';

  @override
  String get glossarySelectEntry =>
      'Выберите запись глоссария, чтобы изменить её разделы.';

  @override
  String glossaryEntryCount(int count) {
    return 'Записей: $count';
  }

  @override
  String glossarySegmentsCount(int unlocked, int total) {
    return 'Открыто $unlocked из $total записей';
  }

  @override
  String get glossaryPortraitUnlocked => 'Портрет открыт';

  @override
  String get glossaryPortraitSilhouette => 'Силуэт — портрет не открыт';

  @override
  String get glossarySegments => 'Записи';

  @override
  String get glossaryPending => 'Несохранённое изменение';

  @override
  String get glossaryShowFullText => 'Показать полный текст записи';

  @override
  String get glossarySegmentIntroduction => 'Введение / портрет';

  @override
  String get glossarySegmentUnlock => 'Открытие';

  @override
  String glossarySegmentEntry(int number) {
    return 'Запись $number';
  }

  @override
  String get questJournalAll => 'Все задания';

  @override
  String get questJournalOldCamp => 'Старый лагерь';

  @override
  String get questJournalNewCamp => 'Новый лагерь';

  @override
  String get questJournalSwampCamp => 'Болотный лагерь';

  @override
  String get questJournalColony => 'Колония';

  @override
  String get questJournalCompleted => 'Завершённые';

  @override
  String get questJournalHint =>
      'Вид внутриигрового журнала. Внутренние состояния и ещё не начатые задания остаются доступны в разделе «Все данные».';

  @override
  String get questJournalNoEntries =>
      'Нет заданий журнала, соответствующих текущим фильтрам.';

  @override
  String get glossaryTutorials => 'Обучение';

  @override
  String get tutorialGateNote =>
      'Эти строки управляют сохранёнными разблокировками обучения. Одна разблокировка не обязательно соответствует одной странице обучения в игре.';

  @override
  String get tutorialResetChanges => 'Сбросить изменения обучения';

  @override
  String get tutorialNoGates =>
      'В этом сохранении нет доступных разблокировок обучения.';

  @override
  String tutorialGateUnlockCount(int unlocked, int total) {
    return 'Открыто $unlocked из $total обучений';
  }

  @override
  String get tutorialGateCombatBasics => 'Основы боя';

  @override
  String get tutorialGateCrafting => 'Ремесло';

  @override
  String get tutorialGateCrime => 'Преступления и последствия';

  @override
  String get tutorialGateDrugs => 'Расходуемые предметы и эффекты';

  @override
  String get tutorialGateLockpicking => 'Взлом замков';

  @override
  String get tutorialGateMagic => 'Магия';

  @override
  String get tutorialGateMap => 'Карта';

  @override
  String get tutorialGateMeleeCombat => 'Ближний бой';

  @override
  String get tutorialGateNavigation => 'Движение и навигация';

  @override
  String get tutorialGatePerception => 'Восприятие';

  @override
  String get tutorialGatePlayerProgression => 'Развитие персонажа';

  @override
  String get tutorialGateRanged => 'Дальний бой';

  @override
  String get tutorialGateRiding => 'Верховая езда';

  @override
  String get tutorialGateSleep => 'Сон';

  @override
  String get tutorialGateTrading => 'Торговля';

  @override
  String get windowMinimizeTooltip => 'Свернуть';

  @override
  String get windowMaximizeTooltip => 'Развернуть';

  @override
  String get windowRestoreTooltip => 'Восстановить';

  @override
  String get fallbackDialogEntry => 'Реплика диалога';

  @override
  String get fallbackDialogChoice => 'Вариант диалога';

  @override
  String get fallbackDialogTopic => 'Тема диалога';

  @override
  String get fallbackDialogInformation => 'Сведения диалога';

  @override
  String get fallbackQuest => 'Задание';

  @override
  String get fallbackObjective => 'Цель';

  @override
  String get fallbackItem => 'Предмет';

  @override
  String get attributeSkillPointsFallback => 'Очки обучения (LP)';

  @override
  String attributeManualFallbackLabel(String attributeId, String fallback) {
    String _temp0 = intl.Intl.selectLogic(attributeId, {
      'SuperArmor': 'Стойкость',
      'MaxSuperArmor': 'Макс. стойкость',
      'DamageMultiplier': 'Получаемый урон',
      'SpeedModifier': 'Скорость передвижения',
      'Oxygen': 'Запас воздуха',
      'MaxOxygen': 'Макс. запас воздуха',
      'OxygenDepletionRate': 'Расход воздуха в секунду',
      'OxygenRecoveryRate': 'Возврат воздуха в секунду',
      'CriticalLevelPercent': 'Порог нехватки воздуха',
      'SleepTime': 'Полезные часы сна',
      'MaxSleepTime': 'Макс. полезные часы сна',
      'SleepTimeRecoveryAmount': 'Возврат полезных часов',
      'SleepTimeRecoveryPeriod': 'Интервал восполнения',
      'MaxRestTime': 'Макс. время в кровати',
      'Health_RecoveryRatePerHourOfSleep': 'Здоровье за час сна',
      'Mana_RecoveryRatePerHourOfSleep': 'Мана за час сна',
      'Alcohol': 'Уровень опьянения',
      'MaxAlcohol': 'Макс. опьянение',
      'AlcoholDepletionRate': 'Скорость отрезвления',
      'Swampweed': 'Уровень болотника',
      'MaxSwampweed': 'Макс. уровень болотника',
      'SwampweedDepletionRate': 'Скорость выветривания',
      'XPExecutedBounty': 'Опыт за добивание',
      'XPKillOrDefeatBounty': 'Опыт за победу',
      'other': '$fallback',
    });
    return '$_temp0';
  }

  @override
  String attributeManualTooltip(String attributeId) {
    String _temp0 = intl.Intl.selectLogic(attributeId, {
      'SuperArmor': 'Сколько герой выдерживает, прежде чем удар его пошатнёт.',
      'MaxSuperArmor':
          'Полный запас стойкости; он растёт с уровнем и с надетой бронёй.',
      'DamageMultiplier':
          'Множитель урона, который получает герой: 1 — как обычно, больше — больнее.',
      'SpeedModifier':
          'Множитель того, как быстро герой двигается: 1 — как обычно.',
      'Oxygen':
          'Сколько секунд воздуха осталось под водой; на нуле герой тонет.',
      'MaxOxygen':
          'Сколько секунд герой может пробыть под водой; навык Ныряние это повышает.',
      'OxygenDepletionRate':
          'Сколько воздуха расходуется под водой каждую секунду.',
      'OxygenRecoveryRate':
          'Сколько воздуха возвращается каждую секунду после всплытия.',
      'CriticalLevelPercent':
          'Доля оставшегося воздуха, при которой игра предупреждает об угрозе утонуть.',
      'SleepTime':
          'Часы сна, которые ещё что-то дают; сверх них отдых уже ничего не восстанавливает.',
      'MaxSleepTime':
          'Наибольший запас полезных часов сна, который может держать герой.',
      'SleepTimeRecoveryAmount':
          'Сколько полезных часов сна возвращается при каждом восполнении.',
      'SleepTimeRecoveryPeriod':
          'Сколько времени проходит, прежде чем запас полезных часов сна восполнится снова.',
      'MaxRestTime':
          'Самое долгое пребывание в кровати за один раз, которое допускает игра.',
      'Health_RecoveryRatePerHourOfSleep':
          'Доля максимального здоровья, которая возвращается за каждый час сна.',
      'Mana_RecoveryRatePerHourOfSleep':
          'Доля максимальной маны, которая возвращается за каждый час сна.',
      'Alcohol':
          'Насколько герой пьян; высокие ступени меняют ловкость и ману на силу.',
      'MaxAlcohol':
          'Самый высокий уровень опьянения, которого может достичь герой.',
      'AlcoholDepletionRate':
          'Насколько быстро уровень опьянения падает обратно к трезвости.',
      'Swampweed':
          'Насколько герой одурманен; высокие ступени сдвигают его характеристики.',
      'MaxSwampweed':
          'Самый высокий уровень болотника, которого может достичь герой.',
      'SwampweedDepletionRate':
          'Насколько быстро проходит дурман от болотника.',
      'XPExecutedBounty':
          'Опыт за то, чтобы добить этого персонажа, пока он уже лежит поверженным на земле.',
      'XPKillOrDefeatBounty':
          'Опыт за то, чтобы одолеть этого персонажа: убить его или просто оставить лежать без сознания.',
      'other': '?',
    });
    return '$_temp0';
  }

  @override
  String get knowledgeTypeVoiceLine => 'Озвученная реплика';

  @override
  String get knowledgeTypeOther => 'Другое';

  @override
  String get armorUpgradeUpper => 'Верх';

  @override
  String get armorUpgradeMiddle => 'Середина';

  @override
  String get armorUpgradeLower => 'Низ';

  @override
  String get knowledgeCategoryTopic => 'Тема';

  @override
  String get knowledgeCategoryChoice => 'Вариант';

  @override
  String get knowledgeCategoryInfo => 'Сведения';

  @override
  String get statusOk => 'OK';

  @override
  String get statusFailed => 'Ошибка';

  @override
  String get missingSaveReference => 'Файл отсутствует';

  @override
  String missingSaveReferenceDescription(String slot) {
    return 'Файл $slot.sav отсутствует. Возможно, он удалён, перемещён или переименован; профиль всё ещё ссылается на него.';
  }

  @override
  String get removeFromProfile => 'Удалить из профиля';

  @override
  String get removeSaveFromProfileTitle => 'Удалить сохранение из профиля?';

  @override
  String removeSaveFromProfileBody(String save, String profile) {
    return 'Удалить $save из профиля $profile? Сам файл сохранения останется, если он ещё существует.';
  }

  @override
  String get unassignedSave => 'Не назначено профилю';

  @override
  String get armorUpgradeLight => 'Лёгкое';

  @override
  String get armorUpgradeMedium => 'Среднее';

  @override
  String get armorUpgradeHeavy => 'Тяжёлое';

  @override
  String get knowledgeCaptionForcedConversation => 'Принудительный диалог';

  @override
  String get knowledgeCaptionFollowupTopic => 'Последующая тема';

  @override
  String get knowledgeCaptionFallbackTopic => 'Резервная тема';

  @override
  String durationMinutes(int minutes) {
    return '$minutes мин';
  }

  @override
  String durationHours(int hours) {
    return '$hours ч';
  }

  @override
  String durationHoursMinutes(int hours, int minutes) {
    return '$hours ч $minutes мин';
  }

  @override
  String get backupStatusInvalidProfileStructure =>
      'Недопустимые данные профиля';

  @override
  String get backupStatusSlotMetadataMissing =>
      'Метаданные выбранного сохранения отсутствуют';

  @override
  String defaultProfileName(int id) {
    return 'Профиль $id';
  }

  @override
  String get statusUnknown => 'Неизвестно';

  @override
  String editorUnexpectedError(String details) {
    return 'Непредвиденная ошибка: $details';
  }

  @override
  String get editorOperationInProgress =>
      'Выполняется другая операция. Повторите попытку через несколько секунд.';

  @override
  String get editorUnsavedBeforeDifficulty =>
      'В сохранении есть несохранённые изменения. Сохраните или сбросьте их перед изменением сложности профиля.';

  @override
  String get editorNoSaveFolderSelected => 'Папка сохранений не выбрана.';

  @override
  String get editorNoSaveSelected => 'Сохранение не выбрано.';

  @override
  String get coreUnknownError => 'Неизвестная ошибка ядра';

  @override
  String get editorUnsavedBeforeSwitchProfile =>
      'Сначала сохраните или сбросьте несохранённые изменения — при переключении профиля текущее сохранение будет закрыто.';

  @override
  String get editorUnsavedBeforeOpenFile =>
      'Сохраните или сбросьте несохранённые изменения перед открытием другого файла.';

  @override
  String get editorSelectSavFile => 'Выберите файл сохранения .sav.';

  @override
  String get editorNotGothicGsav =>
      'Выбранный файл не является сохранением Gothic GSAV.';

  @override
  String get editorUnsavedBeforeChangeSaveProfile =>
      'Сохраните или сбросьте несохранённые изменения перед сменой профиля сохранения.';

  @override
  String get editorUnsavedBeforeRemoveProfile =>
      'Сохраните или сбросьте несохранённые изменения перед удалением сохранения из профиля.';

  @override
  String get editorUnsavedBeforeRestoreProfile =>
      'В сохранении есть несохранённые изменения. Сохраните или сбросьте их перед восстановлением резервной копии профиля.';

  @override
  String editorConflictingPropertyEdits(String path) {
    return 'Несохранённые изменения в двух вкладках затрагивают одно и то же свойство ($path). Сбросьте или отмените одно из них, затем повторите сохранение.';
  }

  @override
  String editorGlossaryMemoryConflict(String path) {
    return 'Изменение сегмента глоссария и другое несохранённое изменение на вкладке «Все данные» затрагивают массив Hero MemorizedEvents ($path). Изменения глоссария добавляют или удаляют записи в этом массиве, поэтому их нельзя сохранить вместе. Сбросьте или отмените одно из них, затем повторите сохранение.';
  }

  @override
  String editorGlossaryQuestConflict(String path) {
    return 'Изменение сегмента глоссария и другое несохранённое изменение затрагивают одно и то же свойство CurrentState задания ($path). Изменение глоссария само обновляет это состояние. Сбросьте или отмените одно из них, затем повторите сохранение.';
  }

  @override
  String editorRelationshipConflict(String path) {
    return 'Переопределение отношения и другое несохранённое изменение на вкладке «Все данные» затрагивают одну и ту же запись отношения NPC ($path). Структурированное изменение отношения может заменить модификаторы в этой записи, поэтому изменения нельзя сохранить вместе. Сбросьте или отмените одно из них, затем повторите сохранение.';
  }

  @override
  String editorMultipleStructuralArrayEdits(String path) {
    return 'Несколько несохранённых структурных изменений затрагивают один и тот же массив ($path). Сохраните или сбросьте первое изменение перед добавлением следующего.';
  }

  @override
  String editorStructuralArrayConflict(String path) {
    return 'Структурное изменение события и другое несохранённое изменение на вкладке «Все данные» затрагивают $path. Сохраните или сбросьте одно из них перед продолжением.';
  }

  @override
  String get editorSkillsEffectConflict =>
      'В очереди находятся изменение на вкладке «Навыки» и изменение на вкладке «Все данные», затрагивающие один и тот же эффект персонажа (ActiveEffects › EffectSpec › Def). Их нельзя сохранить вместе. Сбросьте или отмените одно из них, затем повторите сохранение.';

  @override
  String get editorInventoryResetConflict =>
      'В очереди находятся сброс инвентаря и другое изменение того же инвентаря. Сброс заменит весь инвентарь и удалит другое изменение. Сбросьте или отмените одно из них, затем повторите сохранение.';

  @override
  String get editorUseFolder => 'Использовать папку';

  @override
  String get editorGothicSavegameFileType => 'Сохранение Gothic';

  @override
  String get editorNoDifficultyChanges => 'Нет изменений сложности для записи';

  @override
  String get editorDifficultyWritten =>
      'Сложность записана в профиль (создана резервная копия)';

  @override
  String editorChangesSavedWithBackup(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: 'Сохранено $count изменения и создана резервная копия',
      many: 'Сохранено $count изменений и создана резервная копия',
      few: 'Сохранено $count изменения и создана резервная копия',
      one: 'Сохранено $count изменение и создана резервная копия',
    );
    return '$_temp0';
  }

  @override
  String editorPlacementNoteFailed(String details) {
    return 'Перемещение сохранено, но записать заметку для его отмены не удалось: $details';
  }

  @override
  String editorProfileNotFound(int profileId) {
    return 'Профиль $profileId не найден.';
  }

  @override
  String get editorNoFreeSaveSlot =>
      'В папке сохранений игры нет свободных слотов (G1R-001–G1R-999).';

  @override
  String editorSaveImportedAssigned(int profileId) {
    return 'Сохранение импортировано и назначено профилю $profileId';
  }

  @override
  String editorSaveAssigned(int profileId) {
    return 'Сохранение назначено профилю $profileId (созданы связанные резервные копии)';
  }

  @override
  String editorSaveSlotNotAssigned(String slot, int profileId) {
    return 'Слот сохранения $slot не назначен профилю $profileId.';
  }

  @override
  String get editorSaveRemovedFromProfile => 'Сохранение удалено из профиля';

  @override
  String editorRestoredBackup(String path) {
    return 'Резервная копия восстановлена: $path';
  }

  @override
  String editorRestoredBackupWithoutCompanion(String path) {
    return 'Резервная копия восстановлена: $path (PersistentDataList.sav не изменён, так как подходящей сопутствующей резервной копии нет; метаданные слота могут отличаться)';
  }

  @override
  String editorCodecRoundtripPassed(int chunkIndex, int bytes) {
    return 'Проверка полного цикла кодека пройдена: блок $chunkIndex повторно сжат до $bytes байт';
  }

  @override
  String editorDifficultyWriteFailed(String details) {
    return 'Не удалось записать сложность профиля: $details';
  }

  @override
  String editorProfileAssignmentFailed(String details) {
    return 'Не удалось назначить сохранение профилю: $details';
  }

  @override
  String editorProfileRemovalFailed(String details) {
    return 'Не удалось удалить сохранение из профиля: $details';
  }

  @override
  String editorSaveFailed(String details) {
    return 'Не удалось сохранить изменения: $details';
  }

  @override
  String editorScanSavesFailed(String details) {
    return 'Не удалось просканировать сохранения: $details';
  }

  @override
  String editorInspectSaveFailed(String details) {
    return 'Не удалось проверить сохранение: $details';
  }

  @override
  String editorLoadBackupsFailed(String details) {
    return 'Не удалось загрузить резервные копии: $details';
  }

  @override
  String editorRestoreFailed(String details) {
    return 'Не удалось восстановить резервную копию: $details';
  }

  @override
  String editorRestoreReloadFailed(String path, String details) {
    return 'Резервная копия восстановлена: $path, но повторно загрузить сохранение не удалось: $details';
  }

  @override
  String editorCodecCheckFailed(String details) {
    return 'Проверка кодека не пройдена: $details';
  }

  @override
  String editorCodecValidationFailed(String details) {
    return 'Проверка полного цикла кодека не пройдена: $details';
  }

  @override
  String editorPropertySearchFailed(String details) {
    return 'Поиск свойств не выполнен: $details';
  }

  @override
  String get editorSelectionChangedWhileLoadingHeroAttributes =>
      'Выбранное сохранение изменилось во время загрузки атрибутов героя.';

  @override
  String editorSkillsLoadFailed(String details) {
    return 'Не удалось загрузить навыки: $details';
  }

  @override
  String editorProgressionQueryFailed(String details) {
    return 'Не удалось выполнить запрос прогресса: $details';
  }

  @override
  String editorNpcListFailed(String details) {
    return 'Не удалось загрузить список NPC: $details';
  }

  @override
  String editorCharacterListFailed(String details) {
    return 'Не удалось загрузить список персонажей: $details';
  }

  @override
  String editorNpcAttributesFailed(String details) {
    return 'Не удалось загрузить атрибуты NPC: $details';
  }

  @override
  String editorNpcPositionFailed(String details) {
    return 'Не удалось загрузить позицию NPC: $details';
  }

  @override
  String editorNpcInventoryFailed(String details) {
    return 'Не удалось загрузить инвентарь NPC: $details';
  }

  @override
  String editorFactionListFailed(String details) {
    return 'Не удалось загрузить список фракций: $details';
  }

  @override
  String get editorNoBackupPath => 'нет';

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
    return '$prefix: $backupPath; резервная копия PersistentDataList: $persistentPath';
  }

  @override
  String localizationStatusFailed(String details) {
    return 'Не удалось получить состояние локализации: $details';
  }

  @override
  String localizationExtractionFailed(String details) {
    return 'Не удалось извлечь данные: $details';
  }

  @override
  String glossaryLoadFailed(String details) {
    return 'Не удалось загрузить глоссарий: $details';
  }

  @override
  String backupStatusError(String details) {
    return 'Ошибка резервной копии: $details';
  }

  @override
  String memoryEventCategory(String category, String fallback) {
    String _temp0 = intl.Intl.selectLogic(category, {
      'quest': 'Квест',
      'document': 'Документ',
      'story': 'Сюжет',
      'exploration': 'Исследование',
      'combat': 'Бой',
      'social': 'Общение',
      'item': 'Предметы',
      'learning': 'Обучение',
      'guild': 'Гильдия',
      'crime': 'Преступление',
      'rest': 'Отдых',
      'other': '$fallback',
    });
    return '$_temp0';
  }

  @override
  String memoryEventAction(String kind, String fallback) {
    String _temp0 = intl.Intl.selectLogic(kind, {
      'questStarted': 'Квест начат',
      'questSucceeded': 'Квест завершён',
      'questFailed': 'Квест провален',
      'documentRead': 'Документ прочитан',
      'documentSegmentUnlocked': 'Запись открыта',
      'documentSegmentViewed': 'Запись просмотрена',
      'chapterCompleted': 'Глава завершена',
      'areaEntered': 'Вход в область',
      'areaLeft': 'Выход из области',
      'characterKilled': 'Персонаж убит',
      'characterDefeated': 'Персонаж побеждён',
      'combatDodge': 'Уклонение от атаки',
      'characterDebuffed': 'Наложено ослабление',
      'tradeAvailable': 'Торговля разблокирована',
      'itemObtained': 'Предмет получен',
      'itemCrafted': 'Предмет создан',
      'skillStateRecorded': 'Состояние навыков записано',
      'recipeLearned': 'Рецепт изучен',
      'guildJoined': 'Вступление в гильдию',
      'crimeRecorded': 'Преступление записано',
      'slept': 'Сон',
      'storyEvent': 'Сюжетное событие',
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
      'gameTime': 'Игровое время',
      'duration': 'Длительность',
      'chapter': 'Глава',
      'instigator': 'Инициатор',
      'affected': 'Цель',
      'amount': 'Количество',
      'primaryObject': 'Объект',
      'secondaryObject': 'Контекст',
      'segmentText': 'Текст записи',
      'other': '$fallback',
    });
    return '$_temp0';
  }

  @override
  String memoryEventGameTime(int day, String time) {
    return 'День $day, $time';
  }

  @override
  String memoryEventSecondsValue(String value) {
    return '$value с';
  }

  @override
  String memoryEventMoreValues(String values, int count) {
    return '$values +$count';
  }

  @override
  String get memoryEventHero => 'Герой';

  @override
  String get memoryEventDetails => 'Подробности';

  @override
  String get memoryEventTags => 'Теги';

  @override
  String get memoryEventTechnicalData => 'Технические данные';

  @override
  String get memoryEventIndex => 'Индекс';

  @override
  String get memoryEventPosition => 'Позиция';

  @override
  String get memoryEventPayload => 'Данные события';

  @override
  String get memoryEventSubject => 'Объект события';

  @override
  String glossaryCatalogSegmentLabel(String segmentId, String fallback) {
    String _temp0 = intl.Intl.selectLogic(segmentId, {
      'Access': 'Доступ',
      'AccessDenied': 'Доступ запрещён',
      'AccesToTemple': 'Доступ в храм',
      'Advice': 'Совет',
      'AfterFight': 'После боя',
      'AfterFireMages': 'После магов круга огня',
      'AfterNek': 'После Нека',
      'AfterQuest': 'После задания',
      'Alone': 'Один',
      'Amulet': 'Амулет',
      'Annoying': 'Назойливый',
      'Armor': 'Доспех',
      'Avoid': 'Избегать',
      'Backstory': 'Предыстория',
      'BackStory': 'Предыстория',
      'BasicMagic': 'Основы магии',
      'Beated': 'Избит',
      'BecomeMercenary': 'Стать наёмником',
      'Beer': 'Пиво',
      'Bestiary': 'Бестиарий',
      'Blessing': 'Благословение',
      'Boss': 'Главарь',
      'Bully': 'Задира',
      'BullyAdvice': 'Совет насчёт задиры',
      'Camp': 'Лагерь',
      'CampDivided': 'Расколотый лагерь',
      'CareOfMessengers': 'Забота о посланниках',
      'ChangeOpinion': 'Изменить мнение',
      'ChargeUriziel': 'Зарядить Уризель',
      'Chosen': 'Избранный',
      'Contact': 'Связь',
      'Courier': 'Курьер',
      'CraftBows': 'Изготовление луков',
      'Crazy': 'Сумасшедший',
      'DailyMeal': 'Ежедневная еда',
      'DailyRation_Trader': 'Торговец дневными пайками',
      'DAM': 'Плотина',
      'Dead': 'Мёртв',
      'Deal': 'Сделка',
      'Dealer': 'Торговец',
      'Deceived': 'Обманут',
      'Dementia': 'Слабоумие',
      'DenyAccess': 'Отказ в доступе',
      'DifferentOpinion': 'Иное мнение',
      'Discussion': 'Обсуждение',
      'DontTalk': 'Не разговаривать',
      'Duel': 'Дуэль',
      'Entrance': 'Вход',
      'Escape': 'Побег',
      'Extended': 'Расширено',
      'Extra': 'Дополнительно',
      'ExtraInfo': 'Дополнительные сведения',
      'Fanatic': 'Фанатик',
      'Fight': 'Бой',
      'FindUlumulu': 'Найти Улу-Мулу',
      'FireMages': 'Маги круга огня',
      'FireMagesEscape': 'Побег магов круга огня',
      'FiskNewDealer': 'Новый поставщик для Фиска',
      'FiskNewDealerCompleted': 'Новый поставщик для Фиска — завершено',
      'FogTower': 'Туманная башня',
      'Food': 'Еда',
      'Forgave': 'Простил',
      'Forgive': 'Простить',
      'Forgiven': 'Прощён',
      'FourFriends': 'Четверо друзей',
      'FreeHut': 'Свободная хижина',
      'FreeMine': 'Свободная шахта',
      'Fury': 'Ярость',
      'GoodTeacher': 'Хороший учитель',
      'Gossip': 'Сплетни',
      'GotScavenger': 'Падальщик получен',
      'GrantedAccess': 'Доступ разрешён',
      'GRDArmor': 'Доспех стражника',
      'Guide': 'Проводник',
      'HateMages': 'Ненависть к магам',
      'HateMagesExplanation': 'Причина ненависти к магам',
      'HateRiceLord': 'Ненависть к Рисовому Лорду',
      'Heal': 'Исцеление',
      'Healing': 'Исцеление',
      'Help': 'Помощь',
      'Helper': 'Помощник',
      'HelpKagan': 'Помощь Кагану',
      'HutStory': 'История хижины',
      'Ignore': 'Игнорирование',
      'Impress': 'Впечатлить',
      'ImpressAlchemy': 'Впечатлить — алхимия',
      'ImpressInscription': 'Впечатлить — начертание',
      'Info': 'Сведения',
      'Interested': 'Заинтересован',
      'Introduction': 'Знакомство',
      'Introduction_2': 'Знакомство 2',
      'Introduction_Armor': 'Представление доспехов',
      'Introduction_Teacher': 'Знакомство — учитель',
      'Introduction_Trader': 'Знакомство — торговец',
      'Invocation': 'Призыв',
      'JoinSC': 'Вступление в Болотный лагерь',
      'Joint': 'Косяк болотника',
      'KalomCamp': 'Лагерь Кор Галома',
      'Leader': 'Предводитель',
      'Learning': 'Обучение',
      'LearnOrcish': 'Изучение языка орков',
      'LeftParty': 'Покинул группу',
      'Library': 'Библиотека',
      'Lie': 'Ложь',
      'Lock': 'Замок',
      'Lockpick': 'Отмычка',
      'Mad': 'Безумен',
      'Mandibles': 'Жвалы',
      'MapMaker': 'Картограф',
      'Monastery': 'Монастырь',
      'MordragKO': 'Мордраг побеждён',
      'Nek': 'Нек',
      'NewCamp': 'Новый лагерь',
      'NewCamper': 'Новый поселенец',
      'NewLeader': 'Новый предводитель',
      'NightPatrol': 'Ночной патруль',
      'NotInterested': 'Не заинтересован',
      'OldCamp': 'Старый лагерь',
      'OrcEnclaveEntrance': 'Вход в город орков',
      'OrcGraveyard': 'Орочий склеп',
      'OreArmor': 'Рудная броня',
      'Party': 'Группа',
      'Pay': 'Оплата',
      'PayMoney': 'Заплатить',
      'Permission': 'Разрешение',
      'Pet': 'Питомец',
      'PreparingInvocation': 'Подготовка призыва',
      'Quest': 'Задание',
      'RankUpFireMages': 'Повышение до мага круга огня',
      'RankUpGuard': 'Повышение до стражника',
      'RanUpFireMagesCompleted': 'Повышение до мага круга огня завершено',
      'Realocated': 'Переселён',
      'Reason': 'Причина',
      'Respect': 'Уважение',
      'ReturnToSC': 'Возвращение в Болотный лагерь',
      'RicelordForeman': 'Надсмотрщик Рисового Лорда',
      'RideScavenger': 'Езда на падальщике',
      'Robe': 'Мантия',
      'Safe': 'В безопасности',
      'Scraper': 'Скребок',
      'SecondChance': 'Второй шанс',
      'SecretLocation': 'Тайное место',
      'SecretPassage': 'Тайный проход',
      'SecretPath': 'Тайная тропа',
      'SleeperFollower': 'Последователь Спящего',
      'SleeperTemple': 'Храм Спящего',
      'SmallInfo': 'Небольшая подсказка',
      'Stonehenge': 'Круг камней',
      'StopFollowing': 'Перестать следовать',
      'SwampCamp': 'Болотный лагерь',
      'Talkative': 'Разговорчивый',
      'Teach': 'Обучение',
      'TeachBow': 'Обучение стрельбе из лука',
      'Teacher': 'Учитель',
      'Teacher2': 'Учитель 2',
      'TeacherInscription': 'Учитель создания заклинаний',
      'TeacherMana': 'Учитель маны',
      'TeachIchor': 'Обучение добыче слизи ползунов',
      'TeachMagic': 'Обучение магии',
      'TeachOrcish': 'Обучение языку орков',
      'TeachStats': 'Обучение характеристикам',
      'TeachWeapon': 'Обучение владению оружием',
      'Teleport': 'Телепортация',
      'TheMysteriousOrc': 'Таинственный орк',
      'ThroneRoom': 'Тронный зал',
      'TradeBow': 'Торговля луками',
      'Trader': 'Торговец',
      'TradeSkins_Trader': 'Торговец шкурами',
      'Traitor': 'Предатель',
      'Trial': 'Испытание',
      'TrollCanyon': 'Ущелье троллей',
      'Trust': 'Доверие',
      'Ulumulu': 'Улу-Мулу',
      'Unexperienced': 'Неопытен',
      'Uriziel': 'Уризель',
      'UrizielRune': 'Руна Уризеля',
      'Useful': 'Полезен',
      'Velaya': 'Велая',
      'Vibrations': 'Вибрации',
      'WaitFreeMine': 'Ожидание в Свободной шахте',
      'WaitInTrainingArea': 'Ожидание на тренировочной площадке',
      'Warning': 'Предупреждение',
      'WarningTooLate': 'Запоздалое предупреждение',
      'WaterMessenger': 'Посланник магов круга воды',
      'Weapon': 'Оружие',
      'Who': 'Кто это',
      'Women': 'Женщины',
      'other': '$fallback',
    });
    return '$_temp0';
  }

  @override
  String get slotRepairTitle => 'Повреждённые слоты инвентаря';

  @override
  String slotRepairBody(int count) {
    return 'В этом сохранении $count слотов инвентаря, идентификатор которых больше не соответствует их позиции: в игре при выбрасывании такого предмета исчезает другой. Восстановление переписывает только идентификаторы: ни один предмет не добавляется, не удаляется и не изменяется. При сохранении, как обычно, создаётся резервная копия.';
  }

  @override
  String get slotRepairQueued =>
      'Восстановление добавлено — сохраните, чтобы применить.';

  @override
  String get slotRepairAction => 'Восстановить';

  @override
  String get slotRepairDiscard => 'Отменить';

  @override
  String get editorInventorySlotEditConflict =>
      'В очереди одновременно прямое изменение слота инвентаря и операция, занимающая слоты целиком (восстановление, добавление или удаление). Вторая перезапишет первую — отмените одно из них и сохраните снова.';

  @override
  String get backupFactFile => 'Файл';

  @override
  String get renameBackupTooltip => 'Назвать эту копию';

  @override
  String get renameBackupTitle => 'Название копии';

  @override
  String get renameBackupLabel => 'Название';

  @override
  String renameBackupHelp(String fileName) {
    return 'Показывается вместо имени файла $fileName. Пустое поле убирает название; сам файл не переименовывается.';
  }

  @override
  String get deleteBackupTooltip => 'Удалить эту копию';

  @override
  String get deleteBackupTitle => 'Удалить копию';

  @override
  String deleteBackupBody(String name, String fileName) {
    return 'Удалить «$name» ($fileName)? Файл будет стёрт с диска, вернуть его не получится.';
  }

  @override
  String get deleteBackupConfirm => 'Удалить';

  @override
  String editorDeletedBackup(String path) {
    return 'Копия удалена: $path';
  }

  @override
  String editorDeleteBackupFailed(String details) {
    return 'Не удалось удалить копию: $details';
  }

  @override
  String editorRenameBackupFailed(String details) {
    return 'Не удалось задать название копии: $details';
  }

  @override
  String get slotRepairUnavailable =>
      'Восстановление сейчас невозможно — в это сохранение нельзя записать.';

  @override
  String editorDeletedBackupWithLabelWarning(String path, String details) {
    return 'Копия удалена: $path — её название убрать не удалось: $details';
  }

  @override
  String get slotRepairNotOffered =>
      'Для этого сохранения восстановление недоступно.';
}
