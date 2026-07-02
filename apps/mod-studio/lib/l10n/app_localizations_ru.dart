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
}
