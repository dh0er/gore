// ignore: unused_import
import 'package:intl/intl.dart' as intl;
import 'app_localizations.dart';

// ignore_for_file: type=lint

/// The translations for Ukrainian (`uk`).
class AppLocalizationsUk extends AppLocalizations {
  AppLocalizationsUk([String locale = 'uk']) : super(locale);

  @override
  String get debugSectionTitle => 'Додатково (налагодження)';

  @override
  String get debugSectionSubtitle =>
      'Діагностика та сирі дані для звітів про помилки';

  @override
  String get showObjectIdsTitle => 'Показати додаткові технічні ID';

  @override
  String get showObjectIdsSubtitle =>
      'Показує технічні ID предметів, знань діалогів, квестів і осиротілих акторів у редакторі. ID NPC завжди видимі.';

  @override
  String get storyStateSidebar => 'Стан сюжету';

  @override
  String get storyStateDescription =>
      'Тут гра відстежує прогрес квестів, діалогів і подій. «Збережено» показує значення з твого збереження; «Не задано» — інші відомі записи. Залежно від запису число може означати так/ні, лічильник або етап прогресу. Часові мітки показують день і час у грі.';

  @override
  String get storyStateReadOnly =>
      'Лише для читання, доки не буде підтверджено значення в скриптах і безпечний запис у мапу. Пов’язаний текст глосарію — це контекст, а не прямий переклад технічного ID.';

  @override
  String get storyStateStructureReadOnly =>
      'Структуру StoryPropertyValues у цьому збереженні не вдалося однозначно й безпечно визначити. Значення сюжету залишаються лише для читання в цьому збереженні.';

  @override
  String get storyStateSearch => 'Пошук у стані сюжету';

  @override
  String storyStateValuesCount(int shown, int total) {
    return '$shown з $total значень сюжету';
  }

  @override
  String get storyStateInteger => 'Ціле число';

  @override
  String get storyStateTimeMarker => 'Часова мітка';

  @override
  String get storyStateChapter => 'Розділ';

  @override
  String get storyStateUnknown => 'Невідомий тип джерела';

  @override
  String storyStateShowDormant(int count) {
    return 'Показати невикористані ($count)';
  }

  @override
  String get storyStateUnknownDetail =>
      'Цього збереженого ID немає в поточному каталозі скриптів (наприклад, з мода або новішої версії гри). У збереженні значення має формат int32, але його зміст не визначається.';

  @override
  String get storyStateStored => 'Збережено';

  @override
  String get storyStateUnset => 'Не задано';

  @override
  String get storyStateUnsetDetail =>
      'Це поле каталогу не серіалізоване в цьому збереженні; тому гра використовує незаданий або типовий стан.';

  @override
  String get storyStateRawValue => 'Сире значення';

  @override
  String storyStateElapsed(String duration) {
    return 'Минуло на момент збереження: $duration';
  }

  @override
  String storyStateAhead(String duration) {
    return 'Попереду від часу збереження: $duration';
  }

  @override
  String storyStateDurationDays(int days, String time) {
    String _temp0 = intl.Intl.pluralLogic(
      days,
      locale: localeName,
      other: '$days дня',
      many: '$days днів',
      few: '$days дні',
      one: '1 день',
    );
    return '$_temp0 $time';
  }

  @override
  String get storyStateRelatedGlossary => 'Пов’язаний запис глосарію';

  @override
  String get storyStateTechnicalPath => 'Технічний шлях';

  @override
  String get storyStateEditingGuidance =>
      'Обери запис, щоб змінити його значення. Зміни застосуються після збереження. Змінюй лише те, наслідки чого розумієш: інакше квести чи діалоги можуть перестати працювати як очікується. Під час збереження автоматично створюється резервна копія.';

  @override
  String get storyStatePending => 'Очікує';

  @override
  String storyStatePendingValue(String value) {
    return 'Буде збережено як $value';
  }

  @override
  String get storyStatePendingRemoval => 'Буде видалено зі збереження';

  @override
  String get storyStateEditValue => 'Редагувати значення';

  @override
  String get storyStateSetValue => 'Задати значення';

  @override
  String get storyStateRemoveValue => 'Видалити зі збереження';

  @override
  String get storyStateUndoChange => 'Скасувати зміну сюжету';

  @override
  String get storyStateResetChanges => 'Скинути зміни сюжету';

  @override
  String storyStateDialogTitle(String id) {
    return 'Редагувати $id';
  }

  @override
  String get storyStateRawInput => 'Знакове значення int32';

  @override
  String get storyStateInvalidInt32 =>
      'Введи ціле число від -2147483648 до 2147483647.';

  @override
  String get storyStateQueueChange => 'Додати зміну в чергу';

  @override
  String storyStateSuggestedValues(String values) {
    return 'Значення, підтверджені в офіційних скриптах: $values';
  }

  @override
  String get storyStateSuggestionsNotLimits =>
      'Пропозиції — не межі перевірки; нативний код, моди або новіші версії гри можуть використовувати інші значення.';

  @override
  String get storyStateUseCurrentTime => 'Використати поточний час збереження';

  @override
  String get storyStateStructuredTime => 'День / час';

  @override
  String get storyStateRawMode => 'Сире int32';

  @override
  String get storyStateChapterWarning =>
      'Зміна лише розділу не синхронізує квести, NPC, інвентар чи стан світу.';

  @override
  String get storyStateDormantWarning =>
      'У кеші офіційних скриптів не знайдено активного читання чи запису цього поля. Воно може бути застарілим, керованим нативним кодом або зарезервованим.';

  @override
  String get storyStateReadOnlySourceWarning =>
      'Офіційні скрипти читають це поле, але не містять скриптового запису. Його все одно може контролювати нативний код.';

  @override
  String get storyStateUnknownEditWarning =>
      'У цього ID з мода або новішої версії немає вбудованої семантики джерела. Редагуй лише сире значення int32.';

  @override
  String storyStateIntegerKind(String kind) {
    String _temp0 = intl.Intl.selectLogic(kind, {
      'binaryFlag': 'Двійковий прапорець',
      'finiteState': 'Багатостадійне значення',
      'counterOrScore': 'Лічильник / очки',
      'calendarDay': 'Календарний день',
      'derivedOrOpaqueInteger': 'Похідне / непрозоре ціле',
      'readOnlyInSourceInteger': 'Лише читання в офіційних скриптах',
      'dormantOrLegacyInteger': 'Невикористане в офіційних скриптах',
      'other': 'Ціле число',
    });
    return '$_temp0';
  }

  @override
  String get storyStateZeroVsUnset =>
      'Збережений 0 і відсутній запис у мапі — різні стани файлу. «Видалити зі збереження» відновлює стан конструктора або типовий стан.';

  @override
  String get appTitle => 'GORE Save Editor';

  @override
  String get appLogoSemanticLabel => 'Логотип GORE Save Editor';

  @override
  String get zoomTooltip => 'Натисни Ctrl +/-, щоб збільшити або зменшити';

  @override
  String get switchToLightMode => 'Увімкнути світлу тему';

  @override
  String get switchToDarkMode => 'Увімкнути темну тему';

  @override
  String get about => 'Про програму';

  @override
  String get tabOverview => 'Огляд';

  @override
  String get tabPlayer => 'Гравець';

  @override
  String get tabAttribute => 'Атрибути';

  @override
  String get heroGroupSkills => 'Навички';

  @override
  String get skillsNoneBody => 'Для цієї постаті навичок не знайдено.';

  @override
  String get skillsUnavailableBody =>
      'На цьому збереженні навички не редагуються — у цієї постаті немає даних ефектів, які можна змінити.';

  @override
  String get skillNotLearned => 'Не вивчено';

  @override
  String get skillLearn => 'Вивчити';

  @override
  String get skillActionLearn => 'вивчити';

  @override
  String get skillActionUnlearn => 'забути';

  @override
  String get skillTierUntrained => 'Нетренований';

  @override
  String get skillTierBeginner => 'Початківець';

  @override
  String get skillTierTrained => 'Тренований';

  @override
  String get skillTierMaster => 'Майстер';

  @override
  String get skillTierNovice => 'Новачок';

  @override
  String get skillTierAmateur => 'Аматор (Коло 0)';

  @override
  String get skillTierLearned => 'Вивчено';

  @override
  String skillTierCircle(int n) {
    return 'Коло $n';
  }

  @override
  String get skillHintBlacksmith1H => '1H-зброя';

  @override
  String get skillHintBlacksmith2H => '2H-зброя';

  @override
  String get skillScutesTrained => 'Тренований (кістяні лусочки)';

  @override
  String get skillScutesMaster => 'Майстер (+ бритвені пластини)';

  @override
  String get skillCategoryCombat => 'Бій';

  @override
  String get skillCategoryCrafting => 'Ремесло';

  @override
  String get skillCategoryHunting => 'Полювання';

  @override
  String get skillCategoryLanguage => 'Мова';

  @override
  String get skillCategoryMagic => 'Магія';

  @override
  String get skillCategoryMovement => 'Рух';

  @override
  String get skillCategoryThievery => 'Крадіж';

  @override
  String get skillCategoryOther => 'Інше';

  @override
  String get skillNameOneHanded => 'Одноручна зброя';

  @override
  String get skillNameTwoHanded => 'Дворучна зброя';

  @override
  String get skillNameFists => 'Кулаки';

  @override
  String get skillNameBow => 'Лук';

  @override
  String get skillNameCrossbow => 'Арбалет';

  @override
  String get skillNameLockpicking => 'Злам замків';

  @override
  String get skillNamePickpocketing => 'Кишенькова крадіж';

  @override
  String get skillNameTakeOrgans => 'Витягнути орган';

  @override
  String get skillNameBreakTeeth => 'Витягнути ікла';

  @override
  String get skillNameTakeClaws => 'Витягнути кігті';

  @override
  String get skillNameSkinFur => 'Зняти хутро';

  @override
  String get skillNameSkin => 'Зняти шкіру';

  @override
  String get skillNameTakeFins => 'Зняти плавці';

  @override
  String get skillNameTakeStingers => 'Витягнути жала';

  @override
  String get skillNameTakeSecretion => 'Витягнути секрет';

  @override
  String get skillNameTakeSkullPlates => 'Зняти черепний панцир';

  @override
  String get skillNameSkinSwampshark => 'Зняти шкіру болотного акули';

  @override
  String get skillNameTakeMinecrawlerPlates => 'Зняти пластини';

  @override
  String get skillNameTakeScutes => 'Зняти лусочки';

  @override
  String get skillNameTakeUluMulu => 'Зняти Ulu-Mulu';

  @override
  String get skillNameOrcWeapons => 'Зброя орків';

  @override
  String get skillNameMining => 'Гірництво';

  @override
  String get skillNameDiving => 'Ниряння';

  @override
  String get skillNameTakeMinecrawlerMandibles => 'Витягнути жвали';

  @override
  String get skillNameTakeShadowbeastHorn => 'Зняти ріг (Shadowbeast)';

  @override
  String get skillNameTakeSpines => 'Витягнути хребет';

  @override
  String get skillNameBreakSwampsharkTeeth => 'Витягнути ікла болотного акули';

  @override
  String get skillNameTakeFireTongue => 'Зняти язик вогню';

  @override
  String get skillNameTakeTrollHorn => 'Зняти ріг (Troll)';

  @override
  String get skillNameAcrobatics => 'Акробатика';

  @override
  String get skillNameWallClimbing => 'Лазіння';

  @override
  String get skillNameRiding => 'Верхова їзда на падальниках';

  @override
  String get skillNameSneaking => 'Крадіжливе пересування';

  @override
  String get skillNameAlchemy => 'Алхімія';

  @override
  String get skillNameRuneInscription => 'Напис рун';

  @override
  String get skillNameBlacksmithing => 'Ковальство';

  @override
  String get skillNameMagicCircle => 'Магічне коло';

  @override
  String get skillNameOrcish => 'Оркська мова';

  @override
  String get tabInventory => 'Інвентар';

  @override
  String get tabTrade => 'Торгівля';

  @override
  String get traderNotAMerchant => 'Ця постать не торгує.';

  @override
  String get traderRetry => 'Спробувати знову';

  @override
  String get traderAmbiguousName =>
      'Кілька записів торговця мають це ім’я, тож редактор не може визначити, який магазин належить цій постаті. Редагування вимкнено, щоб не змінити не той.';

  @override
  String get traderOre => 'Руда (купівельна спроможність)';

  @override
  String get traderNoOre => 'немає руди';

  @override
  String get traderStockCurrent => 'Запас';

  @override
  String get traderStockCurrentTooltip =>
      'Що цей торговець зараз продає. Додані предмети можуть зникнути, коли гра оновить торговця.';

  @override
  String get traderStockBase => 'Базовий запас';

  @override
  String get traderStockBaseTooltip =>
      'У збереженні є цей список, щоб гра поповнювала запас торговця. Гра може перерахувати його за своїми правилами, тож зміни тут не збережуться.';

  @override
  String get traderStockBaseHint =>
      'Лише читання: гра використовує цей список при поповненні, але може його перерахувати. Предмети, додані тут, не залишаться назавжди.';

  @override
  String get traderCurrentStockWarning =>
      'Зміни в інвентарі торговця діють лише до наступного поповнення.';

  @override
  String get traderRestockTitle => 'Таймер поповнення';

  @override
  String get traderRestockTitleTooltip =>
      'Оцінка на основі останньої активності торговця, поточного ігрового часу та складності Resources.';

  @override
  String get traderRestockPending => 'очікує';

  @override
  String get traderRestockRevertTooltip => 'Скасувати очікувану зміну часу';

  @override
  String get traderRestockNever => 'Ніколи';

  @override
  String get traderRestockUnavailable => 'Недоступно';

  @override
  String get traderRestockIntervalUnknown =>
      'Невідомий час очікування поповнення';

  @override
  String get traderRestockNeverStatus =>
      'Активність торговця ще не зафіксована.';

  @override
  String get traderRestockClockAhead =>
      'Збережений час торговця попереду поточного ігрового часу.';

  @override
  String traderRestockNotDueYet(String time) {
    return 'Не раніше ніж $time.';
  }

  @override
  String get traderRestockPossiblyDue =>
      'Торговець, можливо, уже готовий до поповнення.';

  @override
  String get traderRestockEligible =>
      'Торговець уже має бути готовий до поповнення.';

  @override
  String get traderRestockNoWorldTime =>
      'Поточний ігровий час недоступний, тож редактор не може визначити, чи настав час поповнення.';

  @override
  String get traderRestockLastActivity => 'Остання активність торговця';

  @override
  String get traderRestockLastActivityTooltip =>
      'Останній збережений час для цього торговця. Він може походити від торгівлі або іншого оновлення, тож це не обов’язково останнє поповнення.';

  @override
  String get traderRestockForecastWindow => 'Очікуване поповнення';

  @override
  String get traderRestockForecastWindowTooltip =>
      'Точний час не зберігається у збереженні. Тому редактор показує діапазон від найранішого до найпізнішого очікуваного часу.';

  @override
  String get traderRestockIntervalLabel => 'Очікування поповнення';

  @override
  String traderRestockInterval(int days, String level) {
    return '$days дн. · $level';
  }

  @override
  String get traderRestockIntervalTooltip =>
      'Час очікування залежно від складності Resources: Novice 2, Gothic 3, Hard 5 ігрових днів.';

  @override
  String get traderRestockAutomationLabel => 'Автоматичне поповнення';

  @override
  String get traderRestockAutomationValue => 'Не можна вимкнути в збереженні';

  @override
  String get traderRestockAutomationTooltip =>
      'Редактор збережень не може надійно зупинити автоматичне поповнення. Для цього потрібен мод гри.';

  @override
  String get traderRestockSetNow => 'Встановити на ігровий час';

  @override
  String get traderRestockSetNowTooltip =>
      'Використати поточний ігровий час як останню активність торговця. Це відкладе наступне очікуване поповнення.';

  @override
  String get traderRestockMakeDue => 'Зробити готовим зараз';

  @override
  String get traderRestockMakeDueTooltip =>
      'Перемістити останню активність торговця достатньо далеко назад, щоб поповнення мало бути вже зараз.';

  @override
  String get traderRestockCustom => 'Власний час…';

  @override
  String get traderRestockCustomTooltip =>
      'Обери ігровий день і час останньої активності торговця.';

  @override
  String get traderRestockEditTitle => 'Змінити останню активність торговця';

  @override
  String get traderOreHint =>
      'У грі цифра інша: під час завантаження гра додає те, що накопичилось з останньої торгівлі — продає надлишки й поповнює запас. Це число — відправна точка, а не сума на екрані торгівлі.';

  @override
  String get traderOreHintShort =>
      'Початкове значення — сума на екрані торгівлі може відрізнятися.';

  @override
  String get traderRestockStatusLabel => 'Статус';

  @override
  String get traderRestockStatusNever => 'Без активності';

  @override
  String get traderRestockStatusWaiting => 'Очікування поповнення';

  @override
  String get traderRestockStatusReady => 'Готовий до поповнення';

  @override
  String get traderRestockStatusPossiblyReady => 'Можливо готовий';

  @override
  String get traderRestockStatusCheckTime => 'Перевірити збережений час';

  @override
  String get traderRestockStatusUnknown => 'Невідомо';

  @override
  String get traderPriceWarning =>
      'Ціни залежать від запасу торговця та кількості руди, тож зміна цих чисел може змінити й його ціни.';

  @override
  String get traderAddItem => 'Додати предмет';

  @override
  String get traderRemoveItem => 'Видалити рядок';

  @override
  String get traderReadOnlyCore =>
      'Ця базова збірка може лише читати дані торговців.';

  @override
  String get traderDifficultyStockUnsupported =>
      'У цього торговця запас залежить від складності, що редактор не відображає. Редагування тут вимкнено, бо зміна виглядала б успішною, але додатковий запас залишився б без змін.';

  @override
  String get traderRecordIncomplete =>
      'Списки запасу цього торговця відсутні або мають форму, яку редактор не підтримує і не може записати. Редагування вимкнено, щоб зміна не зірвалась під час збереження.';

  @override
  String get traderEmptyStock => 'Нічого в запасі.';

  @override
  String get traderUnknownItem => 'немає в каталозі предметів';

  @override
  String editorTradersLoadFailed(String details) {
    return 'Не вдалося завантажити торговців: $details';
  }

  @override
  String traderStockLineCount(int count) {
    return '$count рядків';
  }

  @override
  String get tabWorld => 'Світ';

  @override
  String get tabCharacters => 'Персонажі';

  @override
  String get characterNoActorBody =>
      'У цієї постаті немає актора у світі, тож немає атрибутів, інвентаря чи подій.';

  @override
  String get characterNoEventsBody => 'Немає подій для цієї постаті.';

  @override
  String get characterOrphanGroup => 'Інше';

  @override
  String get tabAllData => 'Усі дані';

  @override
  String get tabBackups => 'Резервні копії';

  @override
  String get tabSettings => 'Налаштування';

  @override
  String get reset => 'Скинути';

  @override
  String get save => 'Зберегти';

  @override
  String saveWithCount(int count) {
    return 'Зберегти ($count)';
  }

  @override
  String get ok => 'OK';

  @override
  String get cancel => 'Скасувати';

  @override
  String get confirm => 'Підтвердити';

  @override
  String get close => 'Закрити';

  @override
  String get add => 'Додати';

  @override
  String get equippedBadge => 'Екіповано';

  @override
  String get armorUpgradesLabel => 'Покращення';

  @override
  String get browse => 'Огляд';

  @override
  String get noSavFilesFound => 'Файлів .sav не знайдено';

  @override
  String get profile => 'Профіль';

  @override
  String get otherSaves => 'Інші збереження';

  @override
  String profileWithSaves(String name, int count) {
    return '$name ($count збережень)';
  }

  @override
  String get switchProfile => 'Змінити профіль';

  @override
  String get openSaveFile => 'Відкрити файл';

  @override
  String get externalSave => 'Збереження, відкрите ззовні';

  @override
  String get saveProfileTitle => 'Профіль збереження';

  @override
  String get saveProfileDescription =>
      'Признач це збереження іншому ігровому профілю. Збереження та індекс профілю резервуються разом.';

  @override
  String get saveProfileExternalHint =>
      'Обери профіль, щоб імпортувати цей файл у папку збережень гри та зареєструвати його там. Оригінальний файл залишиться без змін.';

  @override
  String get saveProfileNoProfiles =>
      'У PersistentDataList.sav не знайдено профілів гри, які можна редагувати.';

  @override
  String get saveProfileSelect => 'Обрати профіль';

  @override
  String get rescanSaveFolder => 'Пересканувати папку збережень';

  @override
  String get discardUnsavedChangesTitle => 'Скасувати незбережені зміни?';

  @override
  String rescanDiscardBody(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: 'змін',
      many: 'змін',
      few: 'зміни',
      one: 'зміну',
    );
    return 'Повторне сканування перезавантажить усі збереження та скасує твої $count незбережених $_temp0.';
  }

  @override
  String get discardAndRescan => 'Скасувати і пересканувати';

  @override
  String chapterLabel(Object id) {
    return 'Розділ $id';
  }

  @override
  String get quickSave => 'Швидке збереження';

  @override
  String get autoSave => 'Автозбереження';

  @override
  String get manualSave => 'Ручне збереження';

  @override
  String get errorTitle => 'Помилка';

  @override
  String get selectASaveTitle => 'Обери збереження';

  @override
  String get selectASaveBody => 'Деталі збереження з’являться тут.';

  @override
  String bytesValue(String count) {
    return '$count байт';
  }

  @override
  String get inspectionJsonTitle => 'JSON інспекції';

  @override
  String get copy => 'Копіювати';

  @override
  String get savegameFallbackTitle => 'Збереження';

  @override
  String screenshotForSlot(String slot) {
    return 'Знімок екрана для $slot';
  }

  @override
  String get publicSaveName => 'Назва';

  @override
  String get gameTimeTitle => 'Ігровий час';

  @override
  String get gameTimeDay => 'День';

  @override
  String get gameTimeHours => 'Години';

  @override
  String get gameTimeMinutes => 'Хвилини';

  @override
  String get gameTimeSeconds => 'Секунди';

  @override
  String gameTimeTotal(int seconds) {
    return '= $seconds с загалом';
  }

  @override
  String get gameTimeInvalid =>
      'Введи цілі числа — день ≥ 0, години 0–23, хвилини та секунди 0–59.';

  @override
  String get required => 'Обов’язково';

  @override
  String get playerLockedBody =>
      'Редагування приватних даних гравця потребує кодека з підтримкою стиснення.';

  @override
  String get heroTransform => 'Позиція';

  @override
  String get locationX => 'Позиція X';

  @override
  String get locationY => 'Позиція Y';

  @override
  String get locationZ => 'Позиція Z';

  @override
  String get rotationPitch => 'Нахил (pitch)';

  @override
  String get rotationYaw => 'Рискання (yaw)';

  @override
  String get rotationRoll => 'Крен (roll)';

  @override
  String get spawnPositionSection => 'Позиція спавну (орієнтир)';

  @override
  String get resetToSpawnPosition => 'Скинути до позиції спавну';

  @override
  String get positionOutOfRange =>
      'Значення має бути між −10 000 000 і 10 000 000';

  @override
  String get positionNotEditable =>
      'Не вдалося прочитати збережену позицію цього персонажа, тому її не можна редагувати.';

  @override
  String get positionNeverPlaced =>
      'Цього персонажа ніколи не розміщували у світі (позиція 0, 0, 0) — гра може ігнорувати збережену позицію.';

  @override
  String get npcStayInPlace => 'Вимкнути його щоденний розклад';

  @override
  String get npcStayInPlaceHint => 'Тоді він залишиться там, де зараз.';

  @override
  String get npcStayInPlaceLocked =>
      'Його початковий щоденний розклад не записано, тож це вже не скасувати.';

  @override
  String get npcUndoPlacement => 'Скасувати перенесення';

  @override
  String get npcUndoPlacementStale =>
      'У збереженні вже немає того, що записало це перенесення, тож відновлення скасує все, що сталося після.';

  @override
  String get positionNotReadable =>
      'Не вдалося прочитати збережену позицію цього персонажа.';

  @override
  String get npcPositionReadOnly =>
      'Гра відновлює позицію NPC з рівня, а не зі збереження, тож ці значення можна лише переглянути, але не змінити.';

  @override
  String get pickLocation => 'Обрати локацію…';

  @override
  String get pickLocationDialogTitle => 'Обрати локацію';

  @override
  String get applySpotRotation => 'Також застосувати орієнтацію точки';

  @override
  String get locationAreaOther => 'Інше';

  @override
  String get locationAreaCavalornValley => 'Долина Кавалорна';

  @override
  String get locationAreaEastForest => 'Східний ліс';

  @override
  String get locationAreaFogTower => 'Туманна вежа';

  @override
  String get locationAreaIllegalWeedMixers => 'Нелегальні змішувачі трав';

  @override
  String get locationAreaOrcArena => 'Арена орків';

  @override
  String get locationAreaOrcGraveyard => 'Орківське кладовище';

  @override
  String get locationAreaShipwreck => 'Затонулий корабель';

  @override
  String get locationAreaTundra => 'Тундра';

  @override
  String get locationCatalogUnavailable =>
      'Не вдалося завантажити каталог локацій.';

  @override
  String get invalid => 'Недійсне';

  @override
  String get heroAttributes => 'Атрибути героя';

  @override
  String attributeBase(String name) {
    return '$name — базове';
  }

  @override
  String attributeCurrent(String name) {
    return '$name — поточне';
  }

  @override
  String get attributeBaseValue => 'Базове значення';

  @override
  String get attributeCurrentValue => 'Поточне значення';

  @override
  String get inventoryTitle => 'Інвентар';

  @override
  String get inventoryEmpty => 'Цей інвентар порожній.';

  @override
  String get inventoryNeedsDecoded =>
      'Редагування інвентаря потребує розкодованих приватних даних від кодека.';

  @override
  String get inventoryNoStacks =>
      'У розкодованих приватних даних не знайдено жодного стеку предметів.';

  @override
  String get resetInventoryChanges => 'Скинути зміни інвентаря';

  @override
  String get addItemTooltipPendingAdd =>
      'Спочатку збережи очікувані зміни — один новий предмет на збереження';

  @override
  String get addItemTooltipPendingRemove =>
      'Спочатку збережи очікуване видалення — одна структурна зміна на збереження';

  @override
  String get addItemTooltipPendingCount =>
      'Спочатку збережи або скинь очікувані зміни кількості — структурну зміну треба зберегти окремо';

  @override
  String get addItemTooltipDefault => 'Додати предмет до інвентаря';

  @override
  String get addItemButton => 'Додати предмет';

  @override
  String get resetInventoryButton => 'Скинути інвентар';

  @override
  String get resetInventoryTooltipDefault =>
      'Замінити цей інвентар інвентарем із збереження на початку гри';

  @override
  String get resetInventoryTooltipBlocked =>
      'Спочатку збережи або скасуй очікувані зміни інвентаря';

  @override
  String get pendingResetTitle => 'Скинути до інвентаря на початку гри';

  @override
  String pendingResetSubtitle(String level) {
    return 'Рівень ресурсів: $level';
  }

  @override
  String get cancelPendingReset => 'Скасувати скидання';

  @override
  String pendingAddSubtitle(int count) {
    return '×$count — очікуване додавання (ще не збережено)';
  }

  @override
  String get cancelPendingAdd => 'Скасувати очікуване додавання';

  @override
  String get pendingRemovalSubtitle => 'очікуване видалення (ще не збережено)';

  @override
  String get cancelPendingRemoval => 'Скасувати очікуване видалення';

  @override
  String get filterItems => 'Фільтрувати предмети';

  @override
  String noItemsMatchQuery(String query) {
    return 'Жоден предмет не відповідає «$query».';
  }

  @override
  String get pendingRemovalHidesAll =>
      'Очікуване видалення ховає всі предмети — збережи, щоб застосувати.';

  @override
  String categoryWithCount(String label, int count) {
    return '$label ($count)';
  }

  @override
  String get itemTooltipIngredientFor => 'Інгредієнт для';

  @override
  String itemTooltipTeaches(String item) {
    return 'Вчить: $item';
  }

  @override
  String get itemTooltipValue => 'Цінність';

  @override
  String get itemTooltipProtection => 'Захист';

  @override
  String get itemTooltipRequirements => 'Вимоги:';

  @override
  String get itemTooltipManaCost => 'Вартість мани';

  @override
  String get itemTooltipManaUpkeep => 'Вартість мани зарядки';

  @override
  String get itemCategoryAll => 'Усе';

  @override
  String get itemCategoryMeleeWeapon => 'Холодна зброя';

  @override
  String get itemCategoryRangedWeapon => 'Дистанційна зброя';

  @override
  String get itemCategoryMagic => 'Магія';

  @override
  String get itemCategoryWearable => 'Одяг і прикраси';

  @override
  String get itemCategoryFood => 'Їжа';

  @override
  String get itemCategoryPotion => 'Зілля';

  @override
  String get itemCategoryMaterial => 'Матеріали';

  @override
  String get itemCategoryDocument => 'Документи';

  @override
  String get itemCategoryMisc => 'Різне';

  @override
  String get itemCategoryArtefact => 'Артефакти';

  @override
  String get itemCategoryOther => 'Інше';

  @override
  String get count => 'Кількість';

  @override
  String get min1 => 'Мін. 1';

  @override
  String countTimes(String count) {
    return '×$count';
  }

  @override
  String get deleteEquippedTooltip =>
      'Не можна видалити: цей предмет, ймовірно, екіпований або прив’язаний до слота швидкого доступу';

  @override
  String get removeBlockedTooltip =>
      'Спочатку збережи або скинь очікувані зміни інвентаря — додавання чи видалення треба зберегти окремо';

  @override
  String get removeItemFromInventory => 'Видалити предмет з інвентаря';

  @override
  String get progressionLockedBody =>
      'Дані прогресу потребують розкодованих приватних даних від кодека.';

  @override
  String get progressionNeedsTyped =>
      'Структуровані дані прогресу потребують повністю розкодованого збереження з перевіреним типізованим розбором.';

  @override
  String get sectionQuests => 'Квести';

  @override
  String get sectionKnowledge => 'Знання';

  @override
  String get sectionEvents => 'Події';

  @override
  String get firstPage => 'Перша сторінка';

  @override
  String get previousPage => 'Попередня сторінка';

  @override
  String get nextPage => 'Наступна сторінка';

  @override
  String get lastPage => 'Остання сторінка';

  @override
  String pageOfPages(int page, int total) {
    return 'Сторінка $page / $total';
  }

  @override
  String rangeOfTotal(int first, int last, int total) {
    return '$first–$last з $total';
  }

  @override
  String get perPage => 'На сторінку:';

  @override
  String get resetQuestChanges => 'Скинути зміни квестів';

  @override
  String get searchQuests => 'Шукати квести';

  @override
  String get allGroups => 'Усі групи';

  @override
  String groupWithCount(String group, Object count) {
    return '$group ($count)';
  }

  @override
  String stateLabelWithCount(String label, int count) {
    return '$label $count';
  }

  @override
  String get questStateNone => 'Немає';

  @override
  String get questStateAvailable => 'Доступний';

  @override
  String get questStateRunning => 'Активний';

  @override
  String get questStateSucceeded => 'Виконано';

  @override
  String get questStateFailed => 'Провалено';

  @override
  String get questStateUnknown => 'невідомо';

  @override
  String get dialogKnowledge => 'Діалогові знання';

  @override
  String get resetKnowledgeChanges => 'Скинути зміни знань';

  @override
  String get addNpc => 'Додати NPC';

  @override
  String get searchNpcs => 'Шукати NPC';

  @override
  String get npcStatusRowLabel => 'Статус';

  @override
  String get npcStatusAlive => 'живий';

  @override
  String get npcStatusDead => 'мертвий';

  @override
  String get npcRelationshipRowLabel => 'Стосунки';

  @override
  String get npcRelationshipUnavailable => 'Статус стосунків недоступний';

  @override
  String get npcRelationshipAutomatic => 'Обчислює гра';

  @override
  String get npcRelationshipAutomaticHint =>
      'Постійного перевизначення не збережено. Правила гільдій, сюжету, локацій і злочинів оцінюються в грі.';

  @override
  String get npcRelationshipStoredHint =>
      'Збережено як постійне перевизначення NPC щодо гравця. Правила гільдій, сюжету, локацій і злочинів у грі все одно можуть змінити фактичний статус.';

  @override
  String get npcRelationshipFriend => 'Друг';

  @override
  String get npcRelationshipNeutral => 'Нейтральний';

  @override
  String get npcRelationshipEnemy => 'Ворог';

  @override
  String npcRelationshipPending(String relationship) {
    return 'Після збереження стане $relationship';
  }

  @override
  String npcStateHp(String hp, String maxHp) {
    return 'HP $hp / $maxHp';
  }

  @override
  String get npcReviveButton => 'Воскресити';

  @override
  String get npcReviveQueued => 'Буде воскрешено після збереження';

  @override
  String entriesForCharacter(String name) {
    return 'Записи — $name';
  }

  @override
  String get selectNpcToSeeEntries => 'Обери NPC, щоб переглянути записи';

  @override
  String get addKnowledgeEntry => 'Додати запис знань';

  @override
  String get browseCatalog => 'Оглянути каталог';

  @override
  String get alreadyExistsForCharacter => 'Уже існує для цього персонажа.';

  @override
  String get alreadyInPendingChanges => 'Уже в очікуваних змінах.';

  @override
  String duplicateCheckFailed(String error) {
    return 'Перевірку дублікатів не вдалося виконати — спробуй ще раз: $error';
  }

  @override
  String pendingAddsCount(int count) {
    return 'Очікувані додавання ($count)';
  }

  @override
  String get undoAdd => 'Скасувати додавання';

  @override
  String get undoRemove => 'Скасувати видалення';

  @override
  String get removeEntry => 'Видалити запис';

  @override
  String get selectNpcFromList => 'Обери NPC зі списку';

  @override
  String characterWithCount(String name, int count) {
    return '$name ($count)';
  }

  @override
  String get memoryEvents => 'Події пам’яті';

  @override
  String get searchCharacters => 'Шукати персонажів';

  @override
  String eventsForCharacter(String name) {
    return 'Події — $name';
  }

  @override
  String get selectCharacterToSeeEvents =>
      'Обери персонажа, щоб переглянути події';

  @override
  String get noTags => '(без тегів)';

  @override
  String eventSubtitle(String time, String affected) {
    return 't=${time}s  $affected';
  }

  @override
  String get removeEvent => 'Видалити подію';

  @override
  String get removeMemoryEventTitle => 'Видалити подію пам’яті?';

  @override
  String get removeMemoryEventBody =>
      'Поставити цю подію пам’яті в чергу на видалення? Файл збереження зміниться лише коли ти натиснеш «Зберегти».';

  @override
  String get memoryEventRemovalQueued =>
      'Видалення події в черзі — натисни «Зберегти», щоб застосувати.';

  @override
  String get duplicateEvent => 'Дублювати подію';

  @override
  String get duplicateMemoryEventTitle => 'Дублювати подію пам’яті?';

  @override
  String get duplicateMemoryEventBody =>
      'Поставити дублікат цієї події пам’яті в чергу? Файл збереження зміниться лише коли ти натиснеш «Зберегти».';

  @override
  String get memoryEventDuplicationQueued =>
      'Дублювання події в черзі — натисни «Зберегти», щоб застосувати.';

  @override
  String get selectCharacterFromList => 'Обери персонажа зі списку';

  @override
  String get factionsSidebar => 'Фракції';

  @override
  String get factionsForgiveButton => 'Пробачити';

  @override
  String get factionHostile => 'Ворожі';

  @override
  String get factionFriendly => 'Дружні';

  @override
  String crimeMurder(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count вбивства',
      many: '$count вбивств',
      few: '$count вбивства',
      one: '$count вбивство',
    );
    return '$_temp0';
  }

  @override
  String crimeAssault(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count нападу',
      many: '$count нападів',
      few: '$count напади',
      one: '$count напад',
    );
    return '$_temp0';
  }

  @override
  String crimeTheft(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count крадіжки',
      many: '$count крадіжок',
      few: '$count крадіжки',
      one: '$count крадіжка',
    );
    return '$_temp0';
  }

  @override
  String crimeTrespassing(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count незаконного проникнення',
      many: '$count незаконних проникнень',
      few: '$count незаконні проникнення',
      one: '$count незаконне проникнення',
    );
    return '$_temp0';
  }

  @override
  String crimeThreat(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count погрози',
      many: '$count погроз',
      few: '$count погрози',
      one: '$count погроза',
    );
    return '$_temp0';
  }

  @override
  String crimeOther(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count іншого злочинення',
      many: '$count інших злочинень',
      few: '$count інші злочинення',
      one: '$count інше злочинення',
    );
    return '$_temp0';
  }

  @override
  String get factionsForgiveQueued => 'прощається…';

  @override
  String get factionsEmpty => 'Немає відкритих злочинів проти фракцій.';

  @override
  String get factionGuildOldCamp => 'Старий табір';

  @override
  String get factionGuildNewCamp => 'Новий табір';

  @override
  String get factionGuildSwampCamp => 'Болотний табір';

  @override
  String get factionGuildOther => 'Інші / окремі особи';

  @override
  String get allDataLockedBody =>
      'Повний переглядач джерел зараз доступний для файлів збережень GSAV.';

  @override
  String get allDataDescription =>
      'Переглядай метадані GSAV і кожен типізований вузол PUBLIC/PRIVATE. Безпечні скалярні значення та нативні структури можна редагувати; контейнери та непрозорі байти лишаються видимими.';

  @override
  String get allDataEditable => 'Можна редагувати';

  @override
  String get allDataReadOnly => 'Лише читання';

  @override
  String get allDataType => 'Тип';

  @override
  String get allDataScalars => 'Скаляри';

  @override
  String get allDataStructs => 'Структури';

  @override
  String get allDataContainers => 'Контейнери';

  @override
  String get allDataOpaque => 'Непрозорі байти';

  @override
  String get allDataNodes => 'Вузли';

  @override
  String allDataChildren(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count дочірнього вузла',
      many: '$count дочірніх вузлів',
      few: '$count дочірні вузли',
      one: '1 дочірній вузол',
    );
    return '$_temp0';
  }

  @override
  String get allDataPending => 'Очікує збереження';

  @override
  String get allDataTagInputHint => 'Теги через кому або з нового рядка';

  @override
  String allDataTypedSource(String source) {
    return 'Типізовано: $source';
  }

  @override
  String get searchPropertiesLabel =>
      'Пошук властивостей (порожньо = показати все) — напр. Health, GameTime';

  @override
  String get decodingSaveTitle => 'Декодування збереження…';

  @override
  String get decodingSaveBody =>
      'Декодуємо повне приватне вміст для першого пошуку. Це один раз на збереження, далі пошук миттєвий.';

  @override
  String get searchTheSaveTitle => 'Пошук у збереженні';

  @override
  String get searchTheSaveBody =>
      'Введи назву властивості та натисни Enter. Залиш порожнім, щоб показати все.';

  @override
  String get searchFailedTitle => 'Пошук не вдався';

  @override
  String get noMatchesTitle => 'Немає збігів';

  @override
  String get noMatchesBody =>
      'Жоден шлях властивості не містив усіх цих термінів.';

  @override
  String get value => 'Значення';

  @override
  String get backupsTitle => 'Резервні копії';

  @override
  String get refreshBackups => 'Оновити резервні копії';

  @override
  String get noBackupsTitle => 'Немає резервних копій';

  @override
  String get noBackupsBody =>
      'Відредаговані збереження створюють файли резервних копій поруч із обраним слотом.';

  @override
  String get slotBackups => 'Копії слота';

  @override
  String get profileBackups => 'Копії профілю';

  @override
  String get backupFactName => 'Назва';

  @override
  String get backupFactSlot => 'Слот';

  @override
  String get backupFactCreated => 'Створено';

  @override
  String get backupFactSize => 'Розмір';

  @override
  String get backupFactStatus => 'Стан';

  @override
  String get backupFactSha1 => 'SHA-1';

  @override
  String restoreBackupTooltip(String fileName) {
    return 'Відновити $fileName';
  }

  @override
  String get appearanceTitle => 'Вигляд';

  @override
  String get uiFont => 'Шрифт';

  @override
  String get theme => 'Тема';

  @override
  String get themeLight => 'Світла';

  @override
  String get themeDark => 'Темна';

  @override
  String get themeSystem => 'Системна';

  @override
  String get uiScale => 'Масштаб інтерфейсу';

  @override
  String get resetZoomTooltip => 'Скинути масштаб (Ctrl+0)';

  @override
  String get zoomTip =>
      'Підказка: Ctrl + / Ctrl - змінює масштаб будь-де в застосунку.';

  @override
  String get language => 'Інтерфейс';

  @override
  String get gameTextLanguage => 'Текст гри';

  @override
  String get gameTextLanguageHint =>
      'Обравши мову інтерфейсу, ти також обираєш відповідний текст гри. Потім текст гри можна змінити окремо.';

  @override
  String get updatesTitle => 'Оновлення';

  @override
  String get checkForUpdatesAutomatically => 'Перевіряти оновлення автоматично';

  @override
  String get checkForUpdatesNow => 'Перевірити оновлення зараз';

  @override
  String get updatesPortableNotice =>
      'Портативна версія відкриває сторінку завантаження в браузері. Заміни наявні файли новим завантаженням.';

  @override
  String get updateAvailableTitle => 'Доступне оновлення';

  @override
  String updateAvailableMessage(Object version, Object current) {
    return 'Доступна версія $version. У тебе $current.';
  }

  @override
  String get updateDownload => 'Завантажити';

  @override
  String updateOpenFailed(String url) {
    return 'Не вдалося відкрити сторінку завантаження. Вона доступна за адресою $url';
  }

  @override
  String get updateLater => 'Пізніше';

  @override
  String get updateUpToDate => 'Ти користуєшся останньою версією.';

  @override
  String get updateCheckFailed =>
      'Не вдалося перевірити оновлення. Спробуй пізніше.';

  @override
  String get gameTextTitle => 'Текст гри';

  @override
  String get itemImagesTitle => 'Зображення предметів';

  @override
  String get gameDataTitle => 'Дані гри';

  @override
  String itemImagesReady(int count) {
    return 'Готово зображень предметів: $count.';
  }

  @override
  String get itemImagesUnavailable =>
      'Зображення предметів недоступні. Замість них використовуватимуться іконки категорій.';

  @override
  String get checkRefreshItemImages =>
      'Перевірити / оновити зображення предметів';

  @override
  String get gameDataSourceMissing =>
      'Текст гри не вдалося підготувати автоматично. Кеш локалізації можна обрати в налаштуваннях.';

  @override
  String get loadingTexts => 'Завантаження текстів…';

  @override
  String get loadingImages => 'Завантаження зображень…';

  @override
  String get preparing => 'Підготовка…';

  @override
  String gameTextExtractedWithCounts(int ids, int languages) {
    return 'Витягнуто: $ids ідентифікаторів у $languages мовах.';
  }

  @override
  String get gameTextExtracted => 'Локалізований текст гри витягнуто.';

  @override
  String get gameTextNotExtracted => 'Локалізований текст гри ще не витягнуто.';

  @override
  String get extracting => 'Витягнення…';

  @override
  String get extractRefreshLocalizedText =>
      'Витягнути / оновити локалізований текст';

  @override
  String get extractionComplete => 'Витягнення завершено';

  @override
  String get extractionFailed => 'Витягнення не вдалося';

  @override
  String get localizationCacheFileType => 'Кеш локалізації';

  @override
  String get savegameDirectoryTitle => 'Тека збережень';

  @override
  String get folder => 'Тека';

  @override
  String get codecTitle => 'Кодек';

  @override
  String get check => 'Перевірити';

  @override
  String get roundtrip => 'Повний цикл';

  @override
  String get noCodecStatus => 'Немає стану кодека';

  @override
  String get codecReady => 'Кодек готовий';

  @override
  String get codecReadOnly => 'Кодек лише для читання';

  @override
  String get codecUnavailable => 'Кодек недоступний';

  @override
  String get details => 'Деталі';

  @override
  String codecStatusLine(String status) {
    return 'Стан: $status';
  }

  @override
  String codecCapabilityLine(String decompress, String compress) {
    return 'Розпакування: $decompress | Стиснення: $compress';
  }

  @override
  String codecBackendLine(String backend) {
    return 'Backend: $backend';
  }

  @override
  String get yes => 'так';

  @override
  String get no => 'ні';

  @override
  String aboutVersion(String version, String sha) {
    return 'Версія $version ($sha)';
  }

  @override
  String get aboutCopyright => '© 2026 Daniel Hoer';

  @override
  String get aboutLicense => 'Ліцензовано за MIT License.';

  @override
  String difficultyTitle(String profile) {
    return 'Складність — $profile';
  }

  @override
  String get difficultyNoProfile => 'Немає профілю';

  @override
  String get difficultyNoDifficulty => 'Немає складності';

  @override
  String get difficultyLabel => 'Складність';

  @override
  String get difficultyTooltipNoProfile => 'Профіль не обрано';

  @override
  String get difficultyTooltipEdit => 'Редагувати складність для цього профілю';

  @override
  String get difficultyTooltipNoEditable =>
      'У цього профілю немає редагованої складності';

  @override
  String get preset => 'Пресет';

  @override
  String get presetNovice => 'Новачок';

  @override
  String get presetGothic => 'Gothic';

  @override
  String get presetHard => 'Важко';

  @override
  String get presetCustom => 'Власний';

  @override
  String unrecognisedPreset(Object preset) {
    return 'Збережений пресет не розпізнано ($preset). Ти все одно можеш зберегти зміни помічника плавного ближнього бою / незворотної смерті або обрати пресет вище, щоб перезаписати його.';
  }

  @override
  String get closeCombatFlowHelper => 'Помічник плавного ближнього бою';

  @override
  String get permadeath => 'Незворотна смерть';

  @override
  String get notAvailableOnNovice => 'Недоступно на «Новачок»';

  @override
  String get levelCombat => 'Бій';

  @override
  String get levelResources => 'Ресурси';

  @override
  String get levelProgression => 'Прогрес';

  @override
  String get difficultyAppliesToAllSaves =>
      'Складність застосовується до всіх збережень у цьому профілі.';

  @override
  String get savingDifficultyFailed => 'Не вдалося зберегти складність.';

  @override
  String get addItemDialogTitle => 'Додати предмет';

  @override
  String get searchItems => 'Пошук предметів';

  @override
  String failedToLoadCatalog(String error) {
    return 'Не вдалося завантажити каталог: $error';
  }

  @override
  String get noItemsAvailableToAdd => 'Немає предметів для додавання';

  @override
  String get noItemsMatch => 'Жоден предмет не підходить';

  @override
  String get countMustBeAtLeast1 => 'Має бути ≥ 1';

  @override
  String countMustBeAtMost(int max) {
    return 'Має бути ≤ $max';
  }

  @override
  String get addNpcDialogTitle => 'Додати NPC';

  @override
  String get noNpcsAvailableToAdd => 'Немає NPC для додавання';

  @override
  String get noNpcsMatch => 'Жоден NPC не підходить';

  @override
  String get categoryAll => 'Усі';

  @override
  String allWithCount(int count) {
    return 'Усі ($count)';
  }

  @override
  String get addKnowledgeEntryDialogTitle => 'Додати запис знань';

  @override
  String get searchEntries => 'Пошук записів';

  @override
  String get noKnowledgeEntriesAvailableToAdd =>
      'Немає записів знань для додавання';

  @override
  String get noEntriesMatch => 'Жоден запис не підходить';

  @override
  String get heroGroupMainStats => 'Основні характеристики';

  @override
  String get heroGroupCombatMovement => 'Бій / рух';

  @override
  String get heroGroupResistances => 'Опори';

  @override
  String get heroGroupThieving => 'Злодійство';

  @override
  String get heroGroupAdvanced => 'Додатково';

  @override
  String get heroGroupDiving => 'Ніркування';

  @override
  String get heroDivingSkillNote =>
      'Після вивчення ніркування гра при кожному завантаженні збереження скидає запас повітря та відновлення до значень навички. Витрата на секунду лишається такою, якою ти її встановиш.';

  @override
  String get heroGroupSleep => 'Сон';

  @override
  String get heroGroupIntoxication => 'Сп’яніння';

  @override
  String get heroEntryHeroTransform => 'Позиція';

  @override
  String attributeEmpty(String name) {
    return '$name порожнє — введи значення або віднови оригінал перед збереженням.';
  }

  @override
  String attributeInvalidNumber(String name, String text) {
    return 'Некоректне число для $name: «$text»';
  }

  @override
  String get loadingEditorData => 'Завантаження даних редактора';

  @override
  String savingProgress(int done, int total) {
    return 'Збереження… $done з $total';
  }

  @override
  String localizedTextExtractedCount(int idCount, int languageCount) {
    return 'Витягнуто $idCount ідентифікаторів у $languageCount мовах';
  }

  @override
  String get skillSmithing1H => 'Ковальство — одноручне';

  @override
  String get skillSmithing2H => 'Ковальство — дворучне';

  @override
  String get skillCircleNovice => 'Маг-новачок';

  @override
  String get skillCircle1 => 'Перше коло магії';

  @override
  String get skillCircle2 => 'Друге коло магії';

  @override
  String get skillCircle3 => 'Третє коло магії';

  @override
  String get skillCircle4 => 'Четверте коло магії';

  @override
  String get skillCircle5 => 'П’яте коло магії';

  @override
  String get skillCircle6 => 'Шосте коло магії';

  @override
  String get sectionGlossary => 'Глосарій';

  @override
  String get glossarySearch => 'Пошук у глосарії';

  @override
  String get glossaryOldCamp => 'Старий табір';

  @override
  String get glossaryNewCamp => 'Новий табір';

  @override
  String get glossarySwampCamp => 'Болотний табір';

  @override
  String get glossaryOutsiders => 'Чужинці';

  @override
  String get glossaryCreatures => 'Істоти';

  @override
  String get glossaryLocations => 'Локації';

  @override
  String get glossaryFilterLabel => 'Фільтр';

  @override
  String get glossaryFilterTraders => 'Торговці';

  @override
  String get glossaryFilterTeachers => 'Вчителі';

  @override
  String get roleTrader => 'Торговець';

  @override
  String get roleDead => 'Мертвий';

  @override
  String get roleTeacher => 'Вчитель';

  @override
  String get roleArmorer => 'Бронник';

  @override
  String get glossaryFilterArmorers => 'Бронники';

  @override
  String get glossaryFilterHostile => 'Ворожі';

  @override
  String get glossaryRelationshipFilterNote =>
      'Показує постійні ворожі перевизначення, збережені у збереженні. Динамічні стосунки гільдій, сюжету, зон і злочинів обчислюються лише в грі.';

  @override
  String get glossaryFilterDead => 'Мертві';

  @override
  String get glossaryAddEntry => 'Додати запис глосарію';

  @override
  String get glossaryAddTitle => 'Додати запис глосарію';

  @override
  String get glossaryResetChanges => 'Скинути зміни глосарію';

  @override
  String get glossaryNoVisibleEntries =>
      'Немає видимих записів глосарію для цього перегляду.';

  @override
  String get glossaryNoHiddenEntries => 'Усі доступні записи вже видимі.';

  @override
  String get glossaryNoMatch => 'Жоден запис глосарію не підходить.';

  @override
  String get glossarySelectEntry =>
      'Обери запис глосарію, щоб редагувати його розділи.';

  @override
  String glossaryEntryCount(int count) {
    return '$count записів';
  }

  @override
  String glossarySegmentsCount(int unlocked, int total) {
    return '$unlocked з $total записів';
  }

  @override
  String get glossaryPortraitUnlocked => 'Портрет відкрито';

  @override
  String get glossaryPortraitSilhouette => 'Силует — портрет не відкрито';

  @override
  String get glossarySegments => 'Записи';

  @override
  String get glossaryPending => 'Незбережена зміна';

  @override
  String get glossaryShowFullText => 'Показати повний текст запису';

  @override
  String get glossarySegmentIntroduction => 'Вступ / портрет';

  @override
  String get glossarySegmentUnlock => 'Відкриття';

  @override
  String glossarySegmentEntry(int number) {
    return 'Запис $number';
  }

  @override
  String get questJournalAll => 'Усі квести';

  @override
  String get questJournalOldCamp => 'Старий табір';

  @override
  String get questJournalNewCamp => 'Новий табір';

  @override
  String get questJournalSwampCamp => 'Табір Братства';

  @override
  String get questJournalColony => 'Колонія';

  @override
  String get questJournalCompleted => 'Завершені';

  @override
  String get questJournalHint =>
      'Вигляд ігрового журналу. Внутрішні та ще не розпочаті стани квестів лишаються доступними в «Усі дані».';

  @override
  String get questJournalNoEntries =>
      'Жоден квест з журналу не відповідає поточним фільтрам.';

  @override
  String get glossaryTutorials => 'Навчання';

  @override
  String get tutorialGateNote =>
      'Ці рядки керують збереженими воротами розблокування навчання. Одні ворота не обов’язково відповідають одній сторінці навчання в грі.';

  @override
  String get tutorialResetChanges => 'Скинути зміни навчання';

  @override
  String get tutorialNoGates =>
      'У цьому збереженні немає доступних воріт розблокування навчання.';

  @override
  String tutorialGateUnlockCount(int unlocked, int total) {
    return 'Розблоковано $unlocked з $total воріт навчання';
  }

  @override
  String get tutorialGateCombatBasics => 'Основи бою';

  @override
  String get tutorialGateCrafting => 'Ремесло';

  @override
  String get tutorialGateCrime => 'Злочини та наслідки';

  @override
  String get tutorialGateDrugs => 'Витратні матеріали та ефекти';

  @override
  String get tutorialGateLockpicking => 'Злам замків';

  @override
  String get tutorialGateMagic => 'Магія';

  @override
  String get tutorialGateMap => 'Мапа';

  @override
  String get tutorialGateMeleeCombat => 'Ближній бій';

  @override
  String get tutorialGateNavigation => 'Рух і орієнтування';

  @override
  String get tutorialGatePerception => 'Сприйняття';

  @override
  String get tutorialGatePlayerProgression => 'Розвиток персонажа';

  @override
  String get tutorialGateRanged => 'Дистанційний бій';

  @override
  String get tutorialGateRiding => 'Верхова їзда';

  @override
  String get tutorialGateSleep => 'Сон';

  @override
  String get tutorialGateTrading => 'Торгівля';

  @override
  String get windowMinimizeTooltip => 'Згорнути';

  @override
  String get windowMaximizeTooltip => 'Розгорнути';

  @override
  String get windowRestoreTooltip => 'Відновити';

  @override
  String get fallbackDialogEntry => 'Запис діалогу';

  @override
  String get fallbackDialogChoice => 'Варіант діалогу';

  @override
  String get fallbackDialogTopic => 'Тема діалогу';

  @override
  String get fallbackDialogInformation => 'Інформація діалогу';

  @override
  String get fallbackQuest => 'Квест';

  @override
  String get fallbackObjective => 'Ціль';

  @override
  String get fallbackItem => 'Предмет';

  @override
  String get attributeSkillPointsFallback => 'Очки навичок (LP)';

  @override
  String attributeManualFallbackLabel(String attributeId, String fallback) {
    String _temp0 = intl.Intl.selectLogic(attributeId, {
      'SuperArmor': 'Стійкість',
      'MaxSuperArmor': 'Макс. стійкість',
      'DamageMultiplier': 'Отримувана шкода',
      'SpeedModifier': 'Швидкість руху',
      'Oxygen': 'Повітря',
      'MaxOxygen': 'Макс. повітря',
      'OxygenDepletionRate': 'Витрата повітря за секунду',
      'OxygenRecoveryRate': 'Відновлення повітря за секунду',
      'CriticalLevelPercent': 'Попередження про повітря',
      'SleepTime': 'Залишок відновлювальних годин',
      'MaxSleepTime': 'Макс. відновлювальні години',
      'SleepTimeRecoveryAmount': 'Обсяг поповнення',
      'SleepTimeRecoveryPeriod': 'Інтервал поповнення',
      'MaxRestTime': 'Макс. час у ліжку',
      'Health_RecoveryRatePerHourOfSleep': 'Здоров’я за годину сну',
      'Mana_RecoveryRatePerHourOfSleep': 'Мана за годину сну',
      'Alcohol': 'Рівень алкоголю',
      'MaxAlcohol': 'Макс. алкоголь',
      'AlcoholDepletionRate': 'Швидкість трезвощення',
      'Swampweed': 'Рівень болотної трави',
      'MaxSwampweed': 'Макс. болотна трава',
      'SwampweedDepletionRate': 'Швидкість згасання ефекту',
      'XPExecutedBounty': 'Досвід за добивання',
      'XPKillOrDefeatBounty': 'Досвід за перемогу',
      'Level': 'Рівень',
      'LockpickDurability': 'Міцність відмички',
      'LockpickPrecision': 'Точність відмички',
      'PickPocketing': 'Кишенькова крадіж',
      'other': '$fallback',
    });
    return '$_temp0';
  }

  @override
  String attributeManualTooltip(String attributeId) {
    String _temp0 = intl.Intl.selectLogic(attributeId, {
      'SuperArmor':
          'Скільки ударів витримує цей персонаж, перш ніж удар виб’є його з рівноваги.',
      'MaxSuperArmor':
          'Повний запас стійкості; зростає з рівнем персонажа та з одягненою бронею.',
      'DamageMultiplier':
          'Множник шкоди, яку отримує цей персонаж — 1 це норма, більше болить сильніше.',
      'SpeedModifier': 'Множник швидкості руху цього персонажа — 1 це норма.',
      'Oxygen': 'Секунди повітря під водою; на нулі цей персонаж тоне.',
      'MaxOxygen':
          'Скільки секунд цей персонаж може пробути під водою; навичка «Ніркування» це підвищує.',
      'OxygenDepletionRate':
          'Скільки повітря витрачається щосекунди під водою.',
      'OxygenRecoveryRate':
          'Скільки повітря повертається щосекунди після спливання.',
      'CriticalLevelPercent':
          'Яка частка залишку повітря, при якій гра попереджає про утоплення.',
      'SleepTime':
          'Години сну, що ще щось відновлюють; понад цей ліміт відпочинок уже нічого не дає.',
      'MaxSleepTime':
          'Найбільший запас відновлювальних годин, який може мати цей персонаж.',
      'SleepTimeRecoveryAmount':
          'Відновлювальні години, що повертаються при кожному поповненні запасу.',
      'SleepTimeRecoveryPeriod':
          'Скільки часу минає, перш ніж запас відновлювальних годин знову поповниться.',
      'MaxRestTime': 'Найдовший разовий відпочинок у ліжку, який дозволяє гра.',
      'Health_RecoveryRatePerHourOfSleep':
          'Частка максимального здоров’я, що повертається за кожну проспану годину.',
      'Mana_RecoveryRatePerHourOfSleep':
          'Частка максимальної мани, що повертається за кожну проспану годину.',
      'Alcohol':
          'Наскільки цей персонаж п’яний; на вищих рівнях змінює спритність і ману на силу.',
      'MaxAlcohol':
          'Найвищий рівень алкоголю, якого може досягти цей персонаж.',
      'AlcoholDepletionRate': 'Як швидко рівень алкоголю падає до трезвості.',
      'Swampweed':
          'Наскільки цей персонаж під кайфом; вищі рівні перерозподіляють його атрибути.',
      'MaxSwampweed':
          'Найвищий рівень болотної трави, якого може досягти цей персонаж.',
      'SwampweedDepletionRate': 'Як швидко спадає ефект болотної трави.',
      'XPExecutedBounty':
          'Досвід за вбивство цього персонажа, коли він уже лежить переможений на землі.',
      'XPKillOrDefeatBounty':
          'Досвід за те, що цього персонажа повалено — незалежно від того, чи він загинув, чи лише втратив свідомість.',
      'Level': 'Рівень персонажа. Зростає з досвідом і дає очки навичок.',
      'LockpickDurability':
          'Залежить від навички «Злам замків»: 2 без підготовки, 4 навчений, 6 майстер.',
      'LockpickPrecision':
          'Залежить від навички «Злам замків»: 0 без підготовки, 1 навчений, 2 майстер.',
      'PickPocketing':
          'Залежить від навички «Кишенькова крадіж»: -30 без підготовки, -10 навчений, +10 майстер.',
      'other': '?',
    });
    return '$_temp0';
  }

  @override
  String get knowledgeTypeVoiceLine => 'Репліка';

  @override
  String get knowledgeTypeOther => 'Інше';

  @override
  String get armorUpgradeUpper => 'Верх';

  @override
  String get armorUpgradeMiddle => 'Середина';

  @override
  String get armorUpgradeLower => 'Низ';

  @override
  String get knowledgeCategoryTopic => 'Тема';

  @override
  String get knowledgeCategoryChoice => 'Вибір';

  @override
  String get knowledgeCategoryInfo => 'Інформація';

  @override
  String get statusOk => 'OK';

  @override
  String get statusFailed => 'Помилка';

  @override
  String get missingSaveReference => 'Файл відсутній';

  @override
  String missingSaveReferenceDescription(String slot) {
    return 'Файл $slot.sav відсутній. Його могли видалити, перенести або перейменувати; профіль досі на нього посилається.';
  }

  @override
  String get removeFromProfile => 'Прибрати з профілю';

  @override
  String get deleteSavegame => 'Видалити збереження';

  @override
  String get deleteSavegameTitle => 'Видалити збереження?';

  @override
  String deleteSavegameBody(String save, String fileName, String profile) {
    return 'Видалити $save ($fileName)? Його приберуть з $profile і з папки збережень. GORE спочатку створить резервну копію.';
  }

  @override
  String get removeSaveFromProfileTitle => 'Прибрати збереження з профілю?';

  @override
  String removeSaveFromProfileBody(String save, String profile) {
    return 'Прибрати $save з профілю $profile? Сам файл збереження залишиться, якщо він ще існує.';
  }

  @override
  String get unassignedSave => 'Не прив’язане до профілю';

  @override
  String get armorUpgradeLight => 'Легка';

  @override
  String get armorUpgradeMedium => 'Середня';

  @override
  String get armorUpgradeHeavy => 'Важка';

  @override
  String get knowledgeCaptionForcedConversation => 'Примусова розмова';

  @override
  String get knowledgeCaptionFollowupTopic => 'Додаткова тема';

  @override
  String get knowledgeCaptionFallbackTopic => 'Запасна тема';

  @override
  String durationMinutes(int minutes) {
    return '$minutes хв';
  }

  @override
  String durationHours(int hours) {
    return '$hours год';
  }

  @override
  String durationHoursMinutes(int hours, int minutes) {
    return '$hours год $minutes хв';
  }

  @override
  String get backupStatusInvalidProfileStructure => 'Некоректні дані профілю';

  @override
  String get backupStatusSlotMetadataMissing =>
      'Немає метаданих обраного збереження';

  @override
  String defaultProfileName(int id) {
    return 'Профіль $id';
  }

  @override
  String get statusUnknown => 'Невідомо';

  @override
  String editorUnexpectedError(String details) {
    return 'Несподівана помилка: $details';
  }

  @override
  String get editorOperationInProgress =>
      'Триває інша операція. Спробуй ще раз через мить.';

  @override
  String get editorUnsavedBeforeDifficulty =>
      'У збереженні є незбережені зміни. Збережи їх або скинь перед зміною складності профілю.';

  @override
  String get editorNoSaveFolderSelected => 'Папку збережень не обрано.';

  @override
  String get editorNoSaveSelected => 'Збереження не обрано.';

  @override
  String get coreUnknownError => 'Невідома помилка ядра';

  @override
  String get editorUnsavedBeforeSwitchProfile =>
      'Спочатку збережи або скинь незбережені зміни — перемикання профілю відведе тебе від поточного збереження.';

  @override
  String get editorUnsavedBeforeOpenFile =>
      'Збережи або скинь незбережені зміни перед відкриттям іншого файлу.';

  @override
  String get editorSelectSavFile => 'Обери файл збереження .sav.';

  @override
  String get editorNotGothicGsav => 'Обраний файл — не збереження Gothic GSAV.';

  @override
  String get editorUnsavedBeforeChangeSaveProfile =>
      'Збережи або скинь незбережені зміни перед зміною профілю збереження.';

  @override
  String get editorUnsavedBeforeRemoveProfile =>
      'Збережи або скинь незбережені зміни перед тим, як прибрати збереження з профілю.';

  @override
  String get editorUnsavedBeforeDeleteSave =>
      'Збережи або скинь незбережені зміни перед видаленням цього збереження.';

  @override
  String get editorUnsavedBeforeRestoreProfile =>
      'У збереженні є незбережені зміни. Збережи їх або скинь перед відновленням резервної копії профілю.';

  @override
  String editorConflictingPropertyEdits(String path) {
    return 'Незбережені зміни з двох вкладок стосуються однієї властивості ($path). Скинь або відкоти одну з них і знову збережи.';
  }

  @override
  String editorGlossaryMemoryConflict(String path) {
    return 'Зміна сегмента глосарію та інша незбережена зміна в «Усі дані» стосуються масиву Hero MemorizedEvents ($path). Зміни глосарію додають або прибирають записи в цьому масиві, тому їх не можна зберегти разом — скинь або відкоти одну з них і знову збережи.';
  }

  @override
  String editorGlossaryQuestConflict(String path) {
    return 'Зміна сегмента глосарію та інша незбережена зміна стосуються тієї самої властивості CurrentState квесту ($path). Зміна глосарію сама оновлює цей стан — скинь або відкоти одну з них і знову збережи.';
  }

  @override
  String editorRelationshipConflict(String path) {
    return 'Перевизначення стосунків та інша незбережена зміна в «Усі дані» стосуються того самого запису стосунків NPC ($path). Структурна зміна стосунків може замінити модифікатори в цьому записі, тому обидві зміни не зберегти разом — скинь або відкоти одну з них і знову збережи.';
  }

  @override
  String editorMultipleStructuralArrayEdits(String path) {
    return 'Більше однієї незбереженої структурної зміни стосуються того самого масиву ($path). Збережи або скинь першу зміну перед додаванням наступної.';
  }

  @override
  String editorStructuralArrayConflict(String path) {
    return 'Структурна зміна події та інша незбережена зміна в «Усі дані» стосуються $path. Збережи або скинь одну з них перед продовженням.';
  }

  @override
  String get editorSkillsEffectConflict =>
      'Зміна в «Навичках» і зміна в «Усі дані» для того самого ефекта персонажа (ActiveEffects › EffectSpec › Def) очікують збереження. Їх не можна зберегти разом — скинь або відкоти одну з них і знову збережи.';

  @override
  String get editorInventoryResetConflict =>
      'Скидання інвентаря та інша зміна того самого інвентаря очікують збереження. Скидання замінює весь інвентар і скасує другу зміну — скинь або відкоти одну з них і знову збережи.';

  @override
  String get editorUseFolder => 'Використати папку';

  @override
  String get editorGothicSavegameFileType => 'Збереження Gothic';

  @override
  String get editorNoDifficultyChanges => 'Немає змін складності для запису';

  @override
  String get editorDifficultyWritten =>
      'Складність записано в профіль (створено резервну копію)';

  @override
  String editorChangesSavedWithBackup(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count змін збережено, створено резервну копію',
      one: '1 зміну збережено, створено резервну копію',
    );
    return '$_temp0';
  }

  @override
  String editorPlacementNoteFailed(String details) {
    return 'Переміщення збережено, але не вдалося записати нотатку для скасування: $details';
  }

  @override
  String editorProfileNotFound(int profileId) {
    return 'Профіль $profileId не знайдено.';
  }

  @override
  String get editorNoFreeSaveSlot =>
      'У папці збережень гри немає вільного слота (G1R-001…G1R-999).';

  @override
  String editorSaveImportedAssigned(int profileId) {
    return 'Збереження імпортовано та прив’язано до профілю $profileId';
  }

  @override
  String editorSaveAssigned(int profileId) {
    return 'Збереження прив’язано до профілю $profileId (створено парні резервні копії)';
  }

  @override
  String editorSaveSlotNotAssigned(String slot, int profileId) {
    return 'Слот $slot не прив’язаний до профілю $profileId.';
  }

  @override
  String get editorSaveRemovedFromProfile => 'Збереження прибрано з профілю';

  @override
  String get editorSaveDeleted =>
      'Збереження видалено; створено резервну копію';

  @override
  String editorRestoredBackup(String path) {
    return 'Відновлено резервну копію: $path';
  }

  @override
  String editorRestoredBackupWithoutCompanion(String path) {
    return 'Відновлено резервну копію: $path (PersistentDataList.sav не змінено — немає відповідної парної копії; метадані слота можуть відрізнятися)';
  }

  @override
  String editorCodecRoundtripPassed(int chunkIndex, int bytes) {
    return 'Перевірка кодека пройдена: фрагмент $chunkIndex стиснуто знову до $bytes байт';
  }

  @override
  String editorDifficultyWriteFailed(String details) {
    return 'Не вдалося записати складність профілю: $details';
  }

  @override
  String editorProfileAssignmentFailed(String details) {
    return 'Не вдалося прив’язати збереження до профілю: $details';
  }

  @override
  String editorProfileRemovalFailed(String details) {
    return 'Не вдалося прибрати збереження з профілю: $details';
  }

  @override
  String editorDeleteSaveFailed(String details) {
    return 'Не вдалося видалити збереження: $details';
  }

  @override
  String editorSaveFailed(String details) {
    return 'Не вдалося зберегти зміни: $details';
  }

  @override
  String editorScanSavesFailed(String details) {
    return 'Не вдалося просканувати збереження: $details';
  }

  @override
  String editorInspectSaveFailed(String details) {
    return 'Не вдалося переглянути збереження: $details';
  }

  @override
  String editorLoadBackupsFailed(String details) {
    return 'Не вдалося завантажити резервні копії: $details';
  }

  @override
  String editorRestoreFailed(String details) {
    return 'Не вдалося відновити резервну копію: $details';
  }

  @override
  String editorRestoreReloadFailed(String path, String details) {
    return 'Резервну копію відновлено: $path, але перезавантажити збереження не вдалося: $details';
  }

  @override
  String editorCodecCheckFailed(String details) {
    return 'Перевірка кодека не вдалася: $details';
  }

  @override
  String editorCodecValidationFailed(String details) {
    return 'Перевірка кодека (roundtrip) не вдалася: $details';
  }

  @override
  String editorPropertySearchFailed(String details) {
    return 'Пошук властивостей не вдався: $details';
  }

  @override
  String get editorSelectionChangedWhileLoadingHeroAttributes =>
      'Обране збереження змінилося під час завантаження атрибутів героя.';

  @override
  String editorSkillsLoadFailed(String details) {
    return 'Не вдалося завантажити навички: $details';
  }

  @override
  String editorProgressionQueryFailed(String details) {
    return 'Запит прогресу не вдався: $details';
  }

  @override
  String editorNpcListFailed(String details) {
    return 'Не вдалося завантажити список NPC: $details';
  }

  @override
  String editorCharacterListFailed(String details) {
    return 'Не вдалося завантажити список персонажів: $details';
  }

  @override
  String editorNpcAttributesFailed(String details) {
    return 'Не вдалося завантажити атрибути NPC: $details';
  }

  @override
  String editorNpcPositionFailed(String details) {
    return 'Не вдалося завантажити позицію NPC: $details';
  }

  @override
  String editorNpcInventoryFailed(String details) {
    return 'Не вдалося завантажити інвентар NPC: $details';
  }

  @override
  String editorFactionListFailed(String details) {
    return 'Не вдалося завантажити список фракцій: $details';
  }

  @override
  String get editorNoBackupPath => 'немає';

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
    return '$prefix: $backupPath; резервна копія PersistentDataList: $persistentPath';
  }

  @override
  String localizationStatusFailed(String details) {
    return 'Не вдалося отримати стан локалізації: $details';
  }

  @override
  String localizationExtractionFailed(String details) {
    return 'Не вдалося витягнути дані: $details';
  }

  @override
  String glossaryLoadFailed(String details) {
    return 'Не вдалося завантажити глосарій: $details';
  }

  @override
  String backupStatusError(String details) {
    return 'Помилка резервної копії: $details';
  }

  @override
  String memoryEventCategory(String category, String fallback) {
    String _temp0 = intl.Intl.selectLogic(category, {
      'quest': 'Квест',
      'document': 'Документ',
      'story': 'Сюжет',
      'exploration': 'Дослідження',
      'combat': 'Бій',
      'social': 'Спілкування',
      'item': 'Предмети',
      'learning': 'Навчання',
      'guild': 'Гільдія',
      'crime': 'Злочин',
      'rest': 'Відпочинок',
      'other': '$fallback',
    });
    return '$_temp0';
  }

  @override
  String memoryEventAction(String kind, String fallback) {
    String _temp0 = intl.Intl.selectLogic(kind, {
      'questStarted': 'Квест розпочато',
      'questSucceeded': 'Квест завершено',
      'questFailed': 'Квест провалено',
      'documentRead': 'Документ прочитано',
      'documentSegmentUnlocked': 'Запис відкрито',
      'documentSegmentViewed': 'Запис переглянуто',
      'chapterCompleted': 'Розділ завершено',
      'areaEntered': 'Увійшов у зону',
      'areaLeft': 'Покинув зону',
      'characterKilled': 'Персонажа вбито',
      'characterDefeated': 'Персонажа переможено',
      'combatDodge': 'Атаку ухилено',
      'characterDebuffed': 'Накладено дебаф',
      'tradeAvailable': 'Торгівлю розблоковано',
      'itemObtained': 'Отримано предмет',
      'itemCrafted': 'Предмет створено',
      'skillStateRecorded': 'Стан навички записано',
      'recipeLearned': 'Рецепт вивчено',
      'guildJoined': 'Долучився до гільдії',
      'crimeRecorded': 'Злочин зафіксовано',
      'slept': 'Поспав',
      'storyEvent': 'Сюжетна подія',
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
      'gameTime': 'Час у грі',
      'duration': 'Тривалість',
      'chapter': 'Розділ',
      'instigator': 'Ініціатор',
      'affected': 'Постраждалі',
      'amount': 'Кількість',
      'primaryObject': 'Об’єкт',
      'secondaryObject': 'Контекст',
      'segmentText': 'Текст запису',
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
  String get memoryEventDetails => 'Деталі';

  @override
  String get memoryEventTags => 'Мітки';

  @override
  String get memoryEventTechnicalData => 'Технічні дані';

  @override
  String get memoryEventIndex => 'Індекс';

  @override
  String get memoryEventPosition => 'Позиція';

  @override
  String get memoryEventPayload => 'Корисне навантаження';

  @override
  String get memoryEventSubject => 'Суб’єкт';

  @override
  String glossaryCatalogSegmentLabel(String segmentId, String fallback) {
    String _temp0 = intl.Intl.selectLogic(segmentId, {
      'Access': 'Доступ',
      'AccessDenied': 'Доступ заборонено',
      'AccesToTemple': 'Доступ до храму',
      'Advice': 'Порада',
      'AfterFight': 'Після бою',
      'AfterFireMages': 'Після магів вогню',
      'AfterNek': 'Після Nek',
      'AfterQuest': 'Після квесту',
      'Alone': 'Сам',
      'Amulet': 'Амулет',
      'Annoying': 'Дратує',
      'Armor': 'Броня',
      'Avoid': 'Уникати',
      'Backstory': 'Передісторія',
      'BackStory': 'Передісторія',
      'BasicMagic': 'Основи магії',
      'Beated': 'Побитий',
      'BecomeMercenary': 'Стати найманцем',
      'Beer': 'Пиво',
      'Bestiary': 'Бестіарій',
      'Blessing': 'Благословення',
      'Boss': 'Бос',
      'Bully': 'Задира',
      'BullyAdvice': 'Порада щодо задири',
      'Camp': 'Табір',
      'CampDivided': 'Розколотий табір',
      'CareOfMessengers': 'Турбота про посланців',
      'ChangeOpinion': 'Зміна думки',
      'ChargeUriziel': 'Заряд Uriziel',
      'Chosen': 'Обраний',
      'Contact': 'Контакт',
      'Courier': 'Кур’єр',
      'CraftBows': 'Крафт луків',
      'Crazy': 'Божевільний',
      'DailyMeal': 'Щоденна їжа',
      'DailyRation_Trader': 'Торговець щоденними пайками',
      'DAM': 'Гребля',
      'Dead': 'Мертвий',
      'Deal': 'Угода',
      'Dealer': 'Дилер',
      'Deceived': 'Обманутий',
      'Dementia': 'Деменція',
      'DenyAccess': 'Відмова в доступі',
      'DifferentOpinion': 'Інша думка',
      'Discussion': 'Обговорення',
      'DontTalk': 'Не говорити',
      'Duel': 'Поєдинок',
      'Entrance': 'Вхід',
      'Escape': 'Втеча',
      'Extended': 'Розширене',
      'Extra': 'Додаткове',
      'ExtraInfo': 'Додаткова інформація',
      'Fanatic': 'Фанатик',
      'Fight': 'Бій',
      'FindUlumulu': 'Знайти Ulu-Mulu',
      'FireMages': 'Маги вогню',
      'FireMagesEscape': 'Втеча магів вогню',
      'FiskNewDealer': 'Новий скупник краденого для Fisk',
      'FiskNewDealerCompleted': 'Новий скупник краденого для Fisk — завершено',
      'FogTower': 'Туманна вежа',
      'Food': 'Їжа',
      'Forgave': 'Пробачив',
      'Forgive': 'Пробачити',
      'Forgiven': 'Пробачено',
      'FourFriends': 'Четверо друзів',
      'FreeHut': 'Вільна хата',
      'FreeMine': 'Вільна шахта',
      'Fury': 'Лютість',
      'GoodTeacher': 'Хороший учитель',
      'Gossip': 'Чутки',
      'GotScavenger': 'Отримав падальника',
      'GrantedAccess': 'Доступ надано',
      'GRDArmor': 'Броня охоронця',
      'Guide': 'Провідник',
      'HateMages': 'Ненависть до магів',
      'HateMagesExplanation': 'Пояснення ненависті до магів',
      'HateRiceLord': 'Ненависть до Rice Lord',
      'Heal': 'Лікування',
      'Healing': 'Зцілення',
      'Help': 'Допомога',
      'Helper': 'Помічник',
      'HelpKagan': 'Допомога Kagan',
      'HutStory': 'Історія хати',
      'Ignore': 'Ігнорувати',
      'Impress': 'Вразити',
      'ImpressAlchemy': 'Вразити — алхімія',
      'ImpressInscription': 'Вразити — написи',
      'Info': 'Інформація',
      'Interested': 'Зацікавлений',
      'Introduction': 'Знайомство',
      'Introduction_2': 'Знайомство 2',
      'Introduction_Armor': 'Знайомство — броня',
      'Introduction_Teacher': 'Знайомство — учитель',
      'Introduction_Trader': 'Знайомство — торговець',
      'Invocation': 'Виклик',
      'JoinSC': 'Долучитися до Табору Братства',
      'Joint': 'Косяк',
      'KalomCamp': 'Табір Kalom',
      'Leader': 'Лідер',
      'Learning': 'Навчання',
      'LearnOrcish': 'Вивчити оркську',
      'LeftParty': 'Покинув групу',
      'Library': 'Бібліотека',
      'Lie': 'Брехня',
      'Lock': 'Замок',
      'Lockpick': 'Відмичка',
      'Mad': 'Божевільний',
      'Mandibles': 'Щелепи шахтного повзуна',
      'MapMaker': 'Картограф',
      'Monastery': 'Монастир',
      'MordragKO': 'Mordrag нокаут',
      'Nek': 'Nek',
      'NewCamp': 'Новий табір',
      'NewCamper': 'Новачок у таборі',
      'NewLeader': 'Новий лідер',
      'NightPatrol': 'Нічний патруль',
      'NotInterested': 'Не цікавиться',
      'OldCamp': 'Старий табір',
      'OrcEnclaveEntrance': 'Вхід в оркську анклав',
      'OrcGraveyard': 'Оркське кладовище',
      'OreArmor': 'Рудна броня',
      'Party': 'Група',
      'Pay': 'Платити',
      'PayMoney': 'Заплатити гроші',
      'Permission': 'Дозвіл',
      'Pet': 'Улюбленець',
      'PreparingInvocation': 'Підготовка виклику',
      'Quest': 'Квест',
      'RankUpFireMages': 'Підвищення мага вогню',
      'RankUpGuard': 'Підвищення охоронця',
      'RanUpFireMagesCompleted': 'Підвищення мага вогню завершено',
      'Realocated': 'Переміщено',
      'Reason': 'Причина',
      'Respect': 'Повага',
      'ReturnToSC': 'Повернутися до Табору Братства',
      'RicelordForeman': 'Бригадир Rice Lord',
      'RideScavenger': 'Їзда на падальнику',
      'Robe': 'Мантія',
      'Safe': 'Безпечно',
      'Scraper': 'Скребок',
      'SecondChance': 'Другий шанс',
      'SecretLocation': 'Таємне місце',
      'SecretPassage': 'Таємний прохід',
      'SecretPath': 'Таємна стежка',
      'SleeperFollower': 'Послідовник Sleeper',
      'SleeperTemple': 'Храм Sleeper',
      'SmallInfo': 'Дрібна інформація',
      'Stonehenge': 'Стоунхендж',
      'StopFollowing': 'Припини слідувати',
      'SwampCamp': 'Табір Братства',
      'Talkative': 'Балакучий',
      'Teach': 'Навчати',
      'TeachBow': 'Навчити стрілецтву',
      'Teacher': 'Учитель',
      'Teacher2': 'Учитель 2',
      'TeacherInscription': 'Учитель написів',
      'TeacherMana': 'Учитель мани',
      'TeachIchor': 'Навчити добутку іхору шахтних повзунів',
      'TeachMagic': 'Навчити магії',
      'TeachOrcish': 'Навчити оркської',
      'TeachStats': 'Тренування атрибутів',
      'TeachWeapon': 'Тренування зброї',
      'Teleport': 'Телепорт',
      'TheMysteriousOrc': 'Таємничий орк',
      'ThroneRoom': 'Тронна зала',
      'TradeBow': 'Торгівля луками',
      'Trader': 'Торговець',
      'TradeSkins_Trader': 'Торговець шкірами',
      'Traitor': 'Зрадник',
      'Trial': 'Випробування',
      'TrollCanyon': 'Каньйон тролів',
      'Trust': 'Довіра',
      'Ulumulu': 'Ulu-Mulu',
      'Unexperienced': 'Недосвідчений',
      'Uriziel': 'Uriziel',
      'UrizielRune': 'Руна Uriziel',
      'Useful': 'Корисний',
      'Velaya': 'Velaya',
      'Vibrations': 'Вібрації',
      'WaitFreeMine': 'Чекати біля Вільної шахти',
      'WaitInTrainingArea': 'Чекати на тренувальному майданчику',
      'Warning': 'Попередження',
      'WarningTooLate': 'Попередження запізнилося',
      'WaterMessenger': 'Посланець магів води',
      'Weapon': 'Зброя',
      'Who': 'Хто',
      'Women': 'Жінки',
      'other': '$fallback',
    });
    return '$_temp0';
  }

  @override
  String get slotRepairTitle => 'Пошкоджені слоти інвентаря';

  @override
  String slotRepairBody(int count) {
    return 'У цьому збереженні $count слотів інвентаря, чий id більше не відповідає позиції — у грі викинути такий предмет означає прибрати інший. Ремонт лише перезаписує id: жоден предмет не додається, не прибирається і не змінюється. Під час збереження, як завжди, створюється резервна копія.';
  }

  @override
  String get slotRepairQueued => 'Ремонт у черзі — збережи, щоб застосувати.';

  @override
  String get slotRepairAction => 'Виправити';

  @override
  String get slotRepairDiscard => 'Скасувати';

  @override
  String get editorInventorySlotEditConflict =>
      'У черзі пряме редагування слота інвентаря разом із зміною, що зачіпає цілі слоти (ремонт, додавання чи видалення). Друга перезапише першу — відкоти одну з них і знову збережи.';

  @override
  String get editorTraderArrayConflict =>
      'Зміна торгівлі в черзі разом із прямим редагуванням масиву торговців. Це редагування перенумеровує рядки, на які спирається зміна торгівлі, тож одна з них потрапить не до того торговця — відкоти одну з них і знову збережи.';

  @override
  String get backupFactFile => 'Файл';

  @override
  String get renameBackupTooltip => 'Назвати цю резервну копію';

  @override
  String get renameBackupTitle => 'Назва резервної копії';

  @override
  String get renameBackupLabel => 'Назва';

  @override
  String renameBackupHelp(String fileName) {
    return 'Показується замість імені файлу $fileName. Залиш порожнім, щоб прибрати назву; сам файл не перейменовується.';
  }

  @override
  String get deleteBackupTooltip => 'Видалити цю резервну копію';

  @override
  String get deleteBackupTitle => 'Видалити резервну копію';

  @override
  String deleteBackupBody(String name, String fileName) {
    return 'Видалити «$name» ($fileName)? Файл зникне з диска без можливості відновлення.';
  }

  @override
  String get deleteBackupConfirm => 'Видалити';

  @override
  String editorDeletedBackup(String path) {
    return 'Резервну копію видалено: $path';
  }

  @override
  String editorDeleteBackupFailed(String details) {
    return 'Не вдалося видалити резервну копію: $details';
  }

  @override
  String editorRenameBackupFailed(String details) {
    return 'Не вдалося назвати резервну копію: $details';
  }

  @override
  String get slotRepairUnavailable =>
      'Зараз виправити не можна — це збереження не можна записати.';

  @override
  String editorDeletedBackupWithLabelWarning(String path, String details) {
    return 'Резервну копію видалено: $path — не вдалося прибрати її назву: $details';
  }

  @override
  String get slotRepairNotOffered => 'Ремонт недоступний для цього збереження.';

  @override
  String get statisticsTitle => 'Статистика';

  @override
  String get statisticsSubtitle =>
      'Стислий підсумок персонажа, квестів, світу та прогресу збереження.';

  @override
  String statisticsCardTitle(String card, String fallback) {
    String _temp0 = intl.Intl.selectLogic(card, {
      'timing': 'Час',
      'character': 'Персонаж',
      'quests': 'Квести',
      'progress': 'Прогрес',
      'encounters': 'Бій і контакти',
      'inventory': 'Навички та інвентар',
      'other': '$fallback',
    });
    return '$_temp0';
  }

  @override
  String statisticsMetric(String metric, String fallback) {
    String _temp0 = intl.Intl.selectLogic(metric, {
      'timePlayed': 'Час гри',
      'worldTime': 'Час світу',
      'level': 'Рівень',
      'experience': 'Досвід',
      'learningPoints': 'Очки навичок',
      'guild': 'Гільдія',
      'health': 'Здоров’я',
      'mana': 'Мана',
      'chapter': 'Розділ',
      'location': 'Локація',
      'kills': 'Вбиті NPC',
      'knownCharacters': 'Відомі персонажі',
      'killedMonsters': 'Вбиті монстри',
      'defeatedNpcs': 'Переможені NPC',
      'killedNpcs': 'Вбиті NPC',
      'knownNpcs': 'Відомі NPC',
      'knownTeachers': 'Відомі вчителі',
      'learnedSkills': 'Вивчені навички',
      'knowledge': 'Записи знань',
      'deadCharacters': 'Мертві персонажі',
      'traders': 'Відомі торговці',
      'inventoryStacks': 'Стеки предметів',
      'inventoryItems': 'Предмети',
      'ore': 'Руда',
      'equipped': 'Екіпіровано',
      'hostileFactions': 'Ворожі фракції',
      'openCrimes': 'Відкриті злочини',
      'position': 'Позиція',
      'other': '$fallback',
    });
    return '$_temp0';
  }

  @override
  String statisticsGuildRank(String rank, String fallback) {
    String _temp0 = intl.Intl.selectLogic(rank, {
      'oldCampShadow': 'Старий табір · Тінь',
      'oldCampGuard': 'Старий табір · Охоронець',
      'oldCampFireMage': 'Старий табір · Маг вогню',
      'newCampRogue': 'Новий табір · Бандит',
      'newCampMercenary': 'Новий табір · Найманець',
      'newCampWaterMage': 'Новий табір · Маг води',
      'swampCampNovice': 'Табір Братства · Новачок',
      'swampCampTemplar': 'Табір Братства · Templar',
      'other': '$fallback',
    });
    return '$_temp0';
  }

  @override
  String get statisticsUnknown => 'Недоступно';

  @override
  String get statisticsMore => 'Більше статистики';

  @override
  String statisticsSummary(
    String level,
    String guild,
    String chapter,
    int completed,
    int failed,
    String playTime,
  ) {
    return 'Рівень $level, $guild, розділ $chapter. Завершено квестів: $completed, провалено: $failed. Час гри: $playTime.';
  }

  @override
  String get locksSidebar => 'Замки';

  @override
  String editorLockListFailed(String details) {
    return 'Не вдалося завантажити список замків: $details';
  }

  @override
  String get locksSearchHint => 'Шукай замок або ключ';

  @override
  String get locksAllRegions => 'Усі регіони';

  @override
  String locksShownOfTotal(int shown, int total) {
    return '$shown з $total';
  }

  @override
  String get locksFilterChests => 'Скрині';

  @override
  String get locksFilterDoors => 'Двері';

  @override
  String get locksFilterUnlocked => 'Відчинено';

  @override
  String get locksFilterLocked => 'Замкнено';

  @override
  String locksDifficultyLevel(int bars, int level) {
    return 'Складність $bars з 4 (внутрішній рівень $level з 7)';
  }

  @override
  String get locksKeyOnly => 'Лише ключем';

  @override
  String locksKeyLabel(String keys) {
    return 'Ключ: $keys';
  }

  @override
  String get locksPermalocked => 'Запечатано назавжди';

  @override
  String get locksReadOnly =>
      'У цьому збереженні немає редагованого набору замків.';

  @override
  String get locksUnknownEntry => 'Немає в цій версії гри';

  @override
  String get locksDoorLeafHint =>
      'Повторне замикання дверей також зачиняє самі двері.';

  @override
  String get locksResetPending => 'Скасувати зміни замків у черзі';
}
