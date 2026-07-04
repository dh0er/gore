// ignore: unused_import
import 'package:intl/intl.dart' as intl;
import 'app_localizations.dart';

// ignore_for_file: type=lint

/// The translations for Russian (`ru`).
class AppLocalizationsRu extends AppLocalizations {
  AppLocalizationsRu([String locale = 'ru']) : super(locale);

  @override
  String get appTitle => 'gore-manager';

  @override
  String get tabMods => 'Моды';

  @override
  String get tabSettings => 'Настройки';

  @override
  String get settingsGameExe => 'Исполняемый файл игры';

  @override
  String get settingsGameExePick => 'Выбрать…';

  @override
  String get settingsLanguage => 'Язык';

  @override
  String get statusInSync => 'Синхронизировано';

  @override
  String get statusChangesPending => 'Есть неприменённые изменения';

  @override
  String get statusGameUpdated => 'Игра обновлена';

  @override
  String get statusStudioDeploy => 'Активно развёртывание Studio';

  @override
  String get statusNothingDeployed => 'Ничего не развёрнуто';

  @override
  String get actionImport => 'Импортировать';

  @override
  String get actionApply => 'Применить';

  @override
  String get actionUndeployAll => 'Убрать всё из игры';

  @override
  String get commonCancel => 'Отмена';

  @override
  String get commonOk => 'OK';

  @override
  String get importFolder => 'Импортировать папку…';

  @override
  String get importFile => 'Импортировать файл…';

  @override
  String get applyTooltip => 'Применить набор модов к игре';

  @override
  String get undeployAllAction => 'Убрать всё из игры';

  @override
  String get undeployAllConfirm =>
      'Удалить из игры всё, что развернул менеджер?';

  @override
  String get takeOverTitle => 'Активно развёртывание Studio';

  @override
  String get takeOverBody =>
      'mod-studio развернуло мод в игре. Перехватить управление, чтобы менеджер применил этот набор?';

  @override
  String get takeOverAction => 'Перехватить';

  @override
  String get refreshAction => 'Обновить';

  @override
  String conflictsTitle(int count) {
    return 'Конфликты ($count)';
  }

  @override
  String get conflictWinner => 'приоритет';

  @override
  String get componentsTitle => 'Компоненты';

  @override
  String targetsMore(int count) {
    return '+ещё $count';
  }

  @override
  String get removeModAction => 'Удалить';

  @override
  String removeModConfirm(String name) {
    return 'Удалить «$name» из библиотеки?';
  }

  @override
  String get errorSetGamePath => 'Сначала укажите путь к игре в настройках.';

  @override
  String applyReportApplied(int count) {
    return 'Применено модов: $count.';
  }

  @override
  String get warningsTitle => 'Предупреждения';

  @override
  String get modDisabledHint => 'Отключён';

  @override
  String get kindGoremod => 'goremod';

  @override
  String get kindTriplet => 'triplet';

  @override
  String get kindPak => 'pak';

  @override
  String get kindUe4ss => 'UE4SS';

  @override
  String get kindRawfile => 'сырой файл';

  @override
  String get kindMixed => 'смешанный';

  @override
  String get sevHard => 'серьёзный';

  @override
  String get sevSoft => 'лёгкий';

  @override
  String get sevInfo => 'инфо';

  @override
  String aboutVersion(String version, String sha) {
    return 'Version $version ($sha)';
  }

  @override
  String get about => 'О программе';

  @override
  String get aboutSubtitle => 'Менеджер модов Gothic 1 Remake';

  @override
  String get aboutCopyright => '© 2026 участники проекта goresave';

  @override
  String get aboutLicense => 'Распространяется по лицензии MIT.';

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
  String get lightMode => 'Светлая тема';

  @override
  String get darkMode => 'Тёмная тема';

  @override
  String get minimize => 'Свернуть';

  @override
  String get restore => 'Восстановить';

  @override
  String get maximize => 'Развернуть';

  @override
  String get close => 'Закрыть';
}
