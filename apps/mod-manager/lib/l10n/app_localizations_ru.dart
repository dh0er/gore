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
}
