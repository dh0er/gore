// ignore: unused_import
import 'package:intl/intl.dart' as intl;
import 'app_localizations.dart';

// ignore_for_file: type=lint

/// The translations for Russian (`ru`).
class AppLocalizationsRu extends AppLocalizations {
  AppLocalizationsRu([String locale = 'ru']) : super(locale);

  @override
  String get coreBlockedTitle => 'Mod Manager не может запуститься';

  @override
  String get coreDllMissingMessage =>
      'Отсутствует нужный файл программы (gore_ffi.dll).';

  @override
  String get coreDllLoadFailedMessage =>
      'Не удалось загрузить нужный файл программы.';

  @override
  String get coreVerificationFailedMessage =>
      'Не удалось проверить нужный файл программы.';

  @override
  String get coreManagerTooOldMessage =>
      'Файлы программы новее, чем Mod Manager. Обновите Mod Manager.';

  @override
  String get coreNativeTooOldMessage =>
      'Файлы программы старее, чем Mod Manager. Переустановите Mod Manager.';

  @override
  String get coreCommandsMissingMessage =>
      'В файлах программы нет функций, которые нужны этому Mod Manager.';

  @override
  String get coreBlockedRepairHint =>
      'Переустановите или восстановите Mod Manager, затем запустите его снова.';

  @override
  String get coreTechnicalDetails => 'Технические сведения';

  @override
  String get coreCopyTechnicalDetails => 'Копировать технические сведения';

  @override
  String get coreTechnicalDetailsCopied => 'Технические сведения скопированы';

  @override
  String get coreTechnicalDetailsCopyFailed =>
      'Не удалось скопировать технические сведения. Повторите попытку.';

  @override
  String get preflightAttention =>
      'Прежде чем менять моды, нужно кое-что решить.';

  @override
  String get preflightGameRunning =>
      'Gothic всё ещё запущен. Закройте игру, прежде чем изменять моды.';

  @override
  String get managerOperationFailed => 'Операция не удалась.';

  @override
  String get libraryOperationFailed => 'Не удалось загрузить список модов.';

  @override
  String get conflictsUnavailable => 'Не удалось проверить конфликты.';

  @override
  String applyReportAppliedWithWarnings(int applied, int warnings) {
    return 'Применено: $applied. Предупреждения: $warnings.';
  }

  @override
  String get modDetailKind => 'Тип';

  @override
  String get modDetailVersion => 'Версия';

  @override
  String get modDetailAuthor => 'Автор';

  @override
  String get modDetailSource => 'Источник';

  @override
  String get modDetailImported => 'Импортирован';

  @override
  String get componentLocalization => 'Тексты';

  @override
  String get componentAudio => 'Звук';

  @override
  String get componentAngelScript => 'Скрипты';

  @override
  String get componentTexture => 'Текстуры';

  @override
  String get componentGameFiles => 'Игровые файлы';

  @override
  String get componentVoice => 'Озвучка';

  @override
  String get componentKindLocalizationPatch => 'Изменения текстов';

  @override
  String get componentKindAudioPatch => 'Изменения звука';

  @override
  String get componentKindAngelScriptPatch => 'Изменения скриптов';

  @override
  String get componentKindTexturePatch => 'Изменения текстур';

  @override
  String get componentKindLoosePak => 'Файл PAK';

  @override
  String get componentKindTriplet => 'Контейнер IoStore';

  @override
  String get componentKindUe4ssLua => 'Скрипт UE4SS';

  @override
  String get componentKindRawFile => 'Файл';

  @override
  String get componentKindFilePatch => 'Заменённый игровой файл';

  @override
  String get componentKindPakFilePatch => 'Игровой файл из PAK в ~mods';

  @override
  String get componentKindVoiceArchivePatch => 'Озвучка';

  @override
  String get rawTargetGameText => 'Все игровые тексты';

  @override
  String get rawTargetGameScripts => 'Все игровые скрипты';

  @override
  String get rawTargetSoundBank => 'Банк звуков';

  @override
  String rawTargetSoundBankNamed(String name) {
    return 'Банк звуков: $name';
  }

  @override
  String get conflictKindLocalization => 'Тексты';

  @override
  String get conflictKindAudio => 'Звук';

  @override
  String get conflictKindAsset => 'Игровые данные';

  @override
  String get conflictKindCdo => 'Значения объектов';

  @override
  String get conflictKindUe4ssUnknown => 'UE4SS (неясно)';

  @override
  String get conflictKindScriptModule => 'Игровой скрипт';

  @override
  String get conflictKindVoiceArchive => 'Озвучка';

  @override
  String get conflictKindRawFile => 'Файл';

  @override
  String get conflictKindLooseFile => 'Игровой файл';

  @override
  String get preflightUnavailable => 'Не удалось проверить установку игры.';

  @override
  String get preflightRetry => 'Проверить снова';

  @override
  String get preflightReviewStatus => 'Показать состояние';

  @override
  String get preflightReviewRecovery => 'Показать справку';

  @override
  String get installRecoveryTitle => 'Прерванная установка';

  @override
  String get installRecoveryBody =>
      'GORE нашёл остатки от установки или сборки скриптов. Эта задача может ещё выполняться, а может уже завершилась и оставила это после себя. GORE не может безопасно убрать это сам.';

  @override
  String get installRecoverySteps =>
      'Если задача ещё выполняется, дождитесь её завершения — не останавливайте её и не удаляйте файлы. Убедившись, что ничего не работает, следуйте README.txt в папке ниже и проверьте снова. Если папка не указана или вы не уверены, ничего не трогайте и обратитесь за помощью.';

  @override
  String get installRecoveryEvidence => 'Что нашёл GORE';

  @override
  String get managerRecoveryTitle => 'Восстановить прерванное изменение';

  @override
  String get managerRecoveryConfirm =>
      'GORE нашёл прерванное изменение и может вернуть игру в известное состояние. Ваши сохранения никогда не затрагиваются.';

  @override
  String get managerRecoveryAlreadyClean =>
      'Восстанавливать было нечего. Состояние проверено заново.';

  @override
  String get managerRecoveryBusy =>
      'Задача снова выполняется. Ничего не изменено — дождитесь завершения.';

  @override
  String get managerRecoveryLockCleared =>
      'Прерванная задача ещё ничего не изменила. Всё убрано.';

  @override
  String get managerRecoveryRestoredPristine =>
      'Изменение отменено. Игра вернулась в прежнее состояние.';

  @override
  String get managerRecoveryApplyPreserved =>
      'Применение уже завершилось. Ничего не потеряно.';

  @override
  String get managerRecoveryUndeployConfirmed =>
      'Удаление уже завершилось. Остатки убраны.';

  @override
  String get managerRecoveryCompileRequired =>
      'Это относится к сборке скриптов, поэтому ничего не изменено. Откройте справку по восстановлению.';

  @override
  String get managerRecoveryInspectionFailed =>
      'GORE не смог безопасно проверить прерванную задачу. Ничего не изменено.';

  @override
  String get managerRecoveryFailed =>
      'Восстановление не удалось завершить. Проверьте состояние, прежде чем пробовать снова.';

  @override
  String get statusUnknown => 'Неизвестно';

  @override
  String statusDetailsTitle(String status) {
    return 'Состояние: $status';
  }

  @override
  String statusDetailsOpen(String status) {
    return 'Показать подробности: $status';
  }

  @override
  String get statusDetailsNoRoot =>
      'Сначала выберите установку Gothic в настройках.';

  @override
  String get statusDetailsNoDeployment => 'Сейчас в игре нет модов.';

  @override
  String get statusDetailsInSyncDescription =>
      'В игре ровно те моды, которые отмечены здесь.';

  @override
  String get statusDetailsDeployedLoadout => 'Моды в игре';

  @override
  String get statusDetailsChangesDescription =>
      'Ваш выбор отличается от того, что стоит в игре.';

  @override
  String get statusDetailsCurrentlyDeployed => 'Сейчас в игре';

  @override
  String get statusDetailsAfterApply => 'После применения';

  @override
  String get statusDetailsGameUpdatedDescription =>
      'Игра обновилась и перезаписала файлы модов. Примените ещё раз, чтобы вернуть их.';

  @override
  String get statusDetailsDriftedFiles => 'Затронутые файлы';

  @override
  String get statusDetailsStudioDescription =>
      'Сейчас моды в этой игре ставит Mod Studio. Перехватите игру, прежде чем Manager применит ваши.';

  @override
  String statusDetailsStudioMod(String name) {
    return 'Мод Studio: $name';
  }

  @override
  String get statusDetailsStudioNameUnknown =>
      'Mod Studio не сообщила название.';

  @override
  String get statusDetailsRecoveryDescription =>
      'Изменение прервалось. Восстановите его, прежде чем менять моды.';

  @override
  String get statusDetailsUnknownDescription =>
      'Не удалось прочитать состояние. Сначала обновите.';

  @override
  String get statusDetailsUnavailable => 'Подробностей нет.';

  @override
  String get statusDetailsEmptyLoadout => 'Модов нет.';

  @override
  String get statusDetailsLastError => 'Последняя ошибка';

  @override
  String get statusDetailsLastApply => 'Последнее применение';

  @override
  String get statusDetailsAppliedMods => 'Применённые моды';

  @override
  String get statusDetailsWarnings => 'Предупреждения';

  @override
  String get statusDetailsReapply => 'Применить повторно';

  @override
  String get statusDetailsOpenSettings => 'Открыть настройки';

  @override
  String get recoveryAction => 'Восстановить';

  @override
  String get recoveryRequiredConfirm =>
      'Восстановить прерванное изменение и убрать наполовину установленные файлы?';

  @override
  String get statusRecoveryRequired => 'Нужно восстановление';

  @override
  String get statusDetailsOwnershipTitle => 'Файлы под управлением GORE';

  @override
  String get statusDetailsOwnershipDescription =>
      'Записано при применении модов — это не проверка того, что файлы ещё на месте.';

  @override
  String get statusDetailsOwnershipLive => 'Заменённые файлы игры';

  @override
  String get statusDetailsOwnershipBackups => 'Резервные копии оригиналов';

  @override
  String get statusDetailsOwnershipAdditive => 'Добавленные файлы модов';

  @override
  String get statusDetailsOwnershipUe4ss => 'Каталоги модов UE4SS';

  @override
  String get statusDetailsOwnershipRecovery => 'Файлы восстановления';

  @override
  String get statusDetailsOwnershipEmpty => 'Здесь ничего не записано.';

  @override
  String statusDetailsOwnershipShown(int shown, int total) {
    return 'Показано $shown из $total путей.';
  }

  @override
  String get appTitle => 'GORE Mod Manager';

  @override
  String get tabMods => 'Моды';

  @override
  String get tabSettings => 'Настройки';

  @override
  String get settingsGameExe => 'Установка Gothic';

  @override
  String get settingsGameExePick => 'Выбрать…';

  @override
  String get settingsLanguage => 'Язык';

  @override
  String get libraryEmptyTitle => 'Модов пока нет';

  @override
  String get libraryEmptyBody =>
      'Импортируйте папку или файл мода, чтобы начать.';

  @override
  String get detailEmptyHint => 'Выберите мод, чтобы увидеть, что он меняет.';

  @override
  String get settingsAdvanced => 'Подробности для продвинутых';

  @override
  String get settingsAdvancedHint =>
      'Показывает техническую сторону: затронутые записи, надёжность проверки конфликтов и файлы под управлением GORE.';

  @override
  String get updatesTitle => 'Обновления';

  @override
  String get checkForUpdatesAutomatically =>
      'Автоматически проверять обновления';

  @override
  String get checkForUpdatesNow => 'Проверить обновления сейчас';

  @override
  String get updatesPortableNotice =>
      'Портативная версия открывает страницу загрузки в браузере. Замените имеющиеся файлы новой загрузкой.';

  @override
  String get updateCheckFailed =>
      'Не удалось проверить обновления. Повторите попытку позже.';

  @override
  String get updateUpToDate => 'У вас последняя версия.';

  @override
  String get updateAvailableTitle => 'Доступно обновление';

  @override
  String updateAvailableMessage(String version, String current) {
    return 'Доступна версия $version. У вас $current.';
  }

  @override
  String get updateLater => 'Позже';

  @override
  String get updateDownload => 'Скачать';

  @override
  String updateOpenFailed(String url) {
    return 'Не удалось открыть страницу загрузки. Она доступна по адресу $url';
  }

  @override
  String get statusInSync => 'Всё актуально';

  @override
  String get statusChangesPending => 'Не применено';

  @override
  String get statusGameUpdated => 'Игра обновилась';

  @override
  String get statusStudioDeploy => 'Активна Mod Studio';

  @override
  String get statusNothingDeployed => 'В игре нет модов';

  @override
  String get actionImport => 'Импортировать';

  @override
  String get actionApply => 'Применить';

  @override
  String get actionStartGame => 'Запустить игру';

  @override
  String get startGameTooltip =>
      'Запустить Gothic с модами, которые сейчас в игре';

  @override
  String get startGameFailed =>
      'Не удалось запустить Gothic. Проверьте установку игры в настройках.';

  @override
  String get commonCancel => 'Отмена';

  @override
  String get commonOk => 'OK';

  @override
  String get importFolder => 'Импортировать папку…';

  @override
  String get importFile => 'Импортировать файл…';

  @override
  String importOutcomeCreated(String name) {
    return '«$name» добавлен.';
  }

  @override
  String importOutcomeUpdated(String name) {
    return '«$name» обновлён.';
  }

  @override
  String importOutcomeUnchanged(String name) {
    return '«$name» уже есть в вашем списке.';
  }

  @override
  String importOutcomeMatchedBy(String method) {
    String _temp0 = intl.Intl.selectLogic(method, {
      'none': 'Совпадений с имеющимися модами нет.',
      'source': 'Совпадение по тому же источнику импорта.',
      'content': 'Совпадение по подтверждённо одинаковому содержимому.',
      'entry_id': 'Совпадение по ID мода.',
      'other': 'Подробностей о совпадении нет.',
    });
    return '$_temp0';
  }

  @override
  String get importRefusalDuplicateAmbiguous =>
      'Это совпадает сразу с несколькими вашими модами. Удалите дубликаты и попробуйте снова.';

  @override
  String get importRefusalIdentityConflict =>
      'Источник и содержимое совпадают с двумя разными вашими модами. Разберитесь с ними и попробуйте снова.';

  @override
  String get importFailed =>
      'Это не удалось импортировать. Поддерживаются папки, ZIP-архивы и отдельные файлы модов (*_P.pak, .utoc/.ucas, .lcache, .bank, PrecompiledScript*.Cache). Сначала распакуйте .7z или .rar, затем импортируйте папку. Мод всё же мог быть добавлен или обновлён — обновите список, прежде чем пробовать снова.';

  @override
  String get importPickerFailed =>
      'Не удалось открыть выбор файлов. Ничего не импортировано.';

  @override
  String get importOutcomeUnknown =>
      'Результат неясен. Обновите, чтобы проверить список модов.';

  @override
  String get applyTooltip => 'Установить отмеченные моды в игру';

  @override
  String get undeployAllAction => 'Убрать всё из игры';

  @override
  String get undeployAllConfirm =>
      'Убрать из игры все моды, установленные Manager?';

  @override
  String get takeOverTitle => 'Mod Studio активна';

  @override
  String get takeOverBody =>
      'Сейчас мод в игре стоит от Mod Studio. Перехватить управление, чтобы Manager применил ваш выбор?';

  @override
  String get takeOverAction => 'Перехватить';

  @override
  String get refreshAction => 'Обновить';

  @override
  String conflictsTitle(int count) {
    return 'Конфликты ($count)';
  }

  @override
  String get conflictWinner => 'побеждает';

  @override
  String get noConflicts => 'Конфликтов не найдено.';

  @override
  String get conflictCoverageIncomplete =>
      'Некоторые моды нельзя проверить полностью, поэтому конфликтов может быть больше.';

  @override
  String get loadOrderDirection =>
      'Моды ниже по списку перекрывают те, что выше.';

  @override
  String get footprintCoverageScope =>
      'Перечислены только известные цели конфликтов. Это не гарантия того, что будет в игре.';

  @override
  String get footprintTargetsExact => 'Затронутые записи — полный список:';

  @override
  String get footprintTargetsPartial =>
      'Затронутые записи — могут быть и другие:';

  @override
  String get footprintTargetsAdvisory =>
      'Вероятно затронутые записи — подсказки, а не доказательство:';

  @override
  String get footprintTargetsOpaque =>
      'GORE не может определить, что здесь меняется.';

  @override
  String get conflictsUnverified => 'Конфликты неизвестны — сначала обновите.';

  @override
  String get componentsTitle => 'Что меняет этот мод';

  @override
  String targetsMore(int count) {
    return '+ещё $count';
  }

  @override
  String get removeModDeploymentHint =>
      'Это уберёт его только из вашего списка. Если он установлен в игре, затем нажмите «Применить».';

  @override
  String removeModSuccess(String name) {
    return '«$name» удалён.';
  }

  @override
  String removeModFailed(String name) {
    return 'Не удалось удалить «$name».';
  }

  @override
  String removeModPartialFailure(String name) {
    return '«$name» удалён, но список не удалось обновить полностью.';
  }

  @override
  String removeModOutcomeUnknown(String name) {
    return 'Не удалось подтвердить, был ли удалён «$name».';
  }

  @override
  String get libraryStateUnknown =>
      'Список модов устарел. Обновите его, прежде чем менять или применять моды.';

  @override
  String get removeModAction => 'Удалить';

  @override
  String removeModConfirm(String name) {
    return 'Удалить «$name» из вашего списка?';
  }

  @override
  String get errorSetGamePath =>
      'Сначала выберите установку Gothic в настройках.';

  @override
  String applyReportApplied(int count) {
    return 'Применено модов: $count.';
  }

  @override
  String get modDisabledHint => 'Отключён';

  @override
  String get kindGoremod => 'Пакет GORE';

  @override
  String get kindTriplet => 'Мод IoStore';

  @override
  String get kindPak => 'Мод PAK';

  @override
  String get kindUe4ss => 'UE4SS';

  @override
  String get kindRawfile => 'Замена файлов целиком';

  @override
  String get kindMixed => 'Смешанный';

  @override
  String get sevHard => 'Конфликт';

  @override
  String get sevSoft => 'Предупреждение';

  @override
  String get sevInfo => 'Заметка';

  @override
  String aboutVersion(String version, String sha) {
    return 'Version $version ($sha)';
  }

  @override
  String get about => 'О программе';

  @override
  String get aboutCopyright => '© 2026 Daniel Hoer';

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
  String get uiScale => 'Размер интерфейса';

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
