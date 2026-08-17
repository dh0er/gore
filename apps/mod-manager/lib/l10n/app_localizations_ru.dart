// ignore: unused_import
import 'package:intl/intl.dart' as intl;
import 'app_localizations.dart';

// ignore_for_file: type=lint

/// The translations for Russian (`ru`).
class AppLocalizationsRu extends AppLocalizations {
  AppLocalizationsRu([String locale = 'ru']) : super(locale);

  @override
  String get coreBlockedTitle => 'Mod Manager недоступен';

  @override
  String get coreDllMissingMessage =>
      'Не найден необходимый файл gore_ffi.dll.';

  @override
  String get coreDllLoadFailedMessage =>
      'Не удалось загрузить нативную библиотеку GORE Core.';

  @override
  String get coreVerificationFailedMessage =>
      'Не удалось проверить нативную библиотеку GORE Core.';

  @override
  String get coreManagerTooOldMessage =>
      'Эта версия GORE Core новее, чем Mod Manager. Обновите Mod Manager.';

  @override
  String get coreNativeTooOldMessage =>
      'Эта версия GORE Core старее, чем Mod Manager. Обновите или восстановите полную установку Mod Manager.';

  @override
  String get coreCommandsMissingMessage =>
      'Библиотека GORE Core не предоставляет все команды, необходимые этому Mod Manager.';

  @override
  String get coreBlockedRepairHint =>
      'Обновите или восстановите полный пакет Mod Manager, затем перезапустите приложение.';

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
  String get preflightAttention => 'Настройка требует внимания.';

  @override
  String get preflightUnavailable => 'Диагностика настройки недоступна.';

  @override
  String get preflightRetry => 'Проверить снова';

  @override
  String get preflightReviewStatus => 'Проверить состояние';

  @override
  String get preflightReviewRecovery => 'Помощь';

  @override
  String get installRecoveryTitle => 'Восстановление установки';

  @override
  String get installRecoveryBody =>
      'GORE обнаружил данные восстановления, связанные с установкой или сборкой скриптов. Соответствующая операция может всё ещё выполняться, либо эти данные могли остаться после её завершения. GORE не может безопасно исправить это автоматически.';

  @override
  String get installRecoverySteps =>
      'Если соответствующая операция ещё выполняется, дождитесь её завершения. Не останавливайте её и не удаляйте файлы блокировки. Следуйте инструкциям в файле README.txt в указанной ниже папке восстановления только после того, как убедитесь, что никакие связанные операции больше не выполняются. Если папка не указана или вы не уверены, оставьте данные восстановления без изменений и обратитесь за помощью. Затем проверьте снова.';

  @override
  String get installRecoveryEvidence => 'Обнаруженные данные восстановления';

  @override
  String get statusUnknown => 'Неизвестно';

  @override
  String statusDetailsTitle(String status) {
    return 'Развёртывание: $status';
  }

  @override
  String statusDetailsOpen(String status) {
    return 'Показать сведения о развёртывании: $status';
  }

  @override
  String get statusDetailsNoRoot =>
      'Выберите установку игры в настройках, чтобы проверить состояние развёртывания.';

  @override
  String get statusDetailsNoDeployment =>
      'Для этой игры нет развёртывания менеджера.';

  @override
  String get statusDetailsInSyncDescription =>
      'Развёрнутые моды соответствуют текущему набору.';

  @override
  String get statusDetailsDeployedLoadout => 'Развёрнутый порядок загрузки';

  @override
  String get statusDetailsChangesDescription =>
      'Текущее развёртывание отличается от того, что установит команда «Применить».';

  @override
  String get statusDetailsCurrentlyDeployed => 'Сейчас развёрнуто';

  @override
  String get statusDetailsAfterApply => 'После применения';

  @override
  String get statusDetailsGameUpdatedDescription =>
      'После последнего развёртывания файлы игры изменились. Примените набор повторно, чтобы восстановить файлы менеджера.';

  @override
  String get statusDetailsDriftedFiles => 'Изменённые файлы';

  @override
  String get statusDetailsStudioDescription =>
      'Сейчас этой установкой игры управляет Mod Studio. Перехватите управление перед применением набора менеджера.';

  @override
  String statusDetailsStudioMod(String name) {
    return 'Мод Studio: $name';
  }

  @override
  String get statusDetailsStudioNameUnknown => 'Studio не сообщило имя мода.';

  @override
  String get statusDetailsRecoveryDescription =>
      'Развёртывание было прервано. Восстановите его перед применением или удалением модов менеджера.';

  @override
  String get statusDetailsUnknownDescription =>
      'Не удалось проверить состояние развёртывания. Обновите его перед применением модов.';

  @override
  String get statusDetailsUnavailable =>
      'Установленное ядро не предоставило эти сведения.';

  @override
  String get statusDetailsEmptyLoadout => 'В этом наборе нет модов.';

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
      'Восстановить прерванное развёртывание и удалить частично развёрнутые файлы?';

  @override
  String get statusRecoveryRequired => 'Требуется восстановление';

  @override
  String get statusDetailsOwnershipTitle => 'Записанные свидетельства владения';

  @override
  String get statusDetailsOwnershipDescription =>
      'Пути из записи развёртывания Менеджера. Они не подтверждают, что эти пути существуют сейчас.';

  @override
  String get statusDetailsOwnershipLive => 'Заменённые файлы игры';

  @override
  String get statusDetailsOwnershipBackups => 'Резервные копии исходных файлов';

  @override
  String get statusDetailsOwnershipAdditive =>
      'Добавленные pak-файлы и контейнеры';

  @override
  String get statusDetailsOwnershipUe4ss => 'Каталоги модов UE4SS';

  @override
  String get statusDetailsOwnershipRecovery =>
      'Файлы и расположения восстановления';

  @override
  String get statusDetailsOwnershipEmpty =>
      'В этой группе нет записанных путей.';

  @override
  String statusDetailsOwnershipShown(int shown, int total) {
    return 'Показано $shown из $total записанных путей.';
  }

  @override
  String get appTitle => 'GORE Mod Manager';

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
  String importOutcomeCreated(String name) {
    return 'Мод «$name» добавлен в библиотеку.';
  }

  @override
  String importOutcomeUpdated(String name) {
    return 'Мод «$name» обновлён в библиотеке.';
  }

  @override
  String importOutcomeUnchanged(String name) {
    return 'Мод «$name» уже есть в библиотеке.';
  }

  @override
  String importOutcomeMatchedBy(String method) {
    String _temp0 = intl.Intl.selectLogic(method, {
      'none': 'Совпадений с существующими записями библиотеки нет.',
      'source': 'Совпадение по тому же источнику импорта.',
      'content':
          'Совпадение по содержимому, идентичность которого подтверждена.',
      'entry_id': 'Совпадение по идентификатору мода.',
      'other': 'Сведения о совпадении недоступны.',
    });
    return '$_temp0';
  }

  @override
  String get importRefusalDuplicateAmbiguous =>
      'Этот импорт соответствует нескольким записям библиотеки. Проверьте или удалите дубликаты и повторите попытку.';

  @override
  String get importRefusalIdentityConflict =>
      'Источник импорта и его содержимое соответствуют разным записям библиотеки. Проверьте или удалите конфликтующие записи и повторите попытку.';

  @override
  String get importFailed =>
      'Не удалось завершить импорт. Поддерживаемые источники: папки, ZIP, отдельные файлы *_P.pak, полные наборы .utoc/.ucas (необязательный .pak), .lcache, .bank и PrecompiledScript*.Cache. Сначала распакуйте архив .7z или .rar, затем импортируйте папку. Источник может не поддерживаться, быть повреждённым или неполным. Возможно, мод уже добавлен или обновлён; обновите и проверьте библиотеку перед повторной попыткой.';

  @override
  String get importPickerFailed =>
      'Не удалось открыть окно выбора файла или папки. Импорт не был запущен. Повторите попытку.';

  @override
  String get importOutcomeUnknown =>
      'Не удалось проверить результат импорта. Нажмите «Обновить», чтобы проверить библиотеку.';

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
    return 'Результаты ($count)';
  }

  @override
  String get conflictWinner => 'предполагаемый победитель';

  @override
  String get noConflicts => 'Распознанные конфликты отсутствуют.';

  @override
  String get conflictCoverageIncomplete =>
      'Сведения о конфликтах включённых модов неполны; возможны дополнительные конфликты.';

  @override
  String get loadOrderDirection =>
      'Порядок загрузки: сначала низкий приоритет; более поздние моды имеют более высокий предполагаемый приоритет.';

  @override
  String get footprintCoverageScope =>
      'Покрытие описывает только распознанные цели конфликтов и не доказывает приоритет во время выполнения.';

  @override
  String get footprintCoverageExact =>
      'Точное — список целей конфликтов компонента полон.';

  @override
  String get footprintCoveragePartial =>
      'Частичное — указанные цели известны, но компонент может затрагивать и другие.';

  @override
  String get footprintCoverageAdvisory =>
      'Ориентировочное — указанные цели являются подсказками, а не исчерпывающим доказательством.';

  @override
  String get footprintCoverageOpaque =>
      'Непрозрачное — цели конфликтов компонента неизвестны.';

  @override
  String get footprintCoverageExactLabel => 'Точное';

  @override
  String get footprintCoveragePartialLabel => 'Частичное';

  @override
  String get footprintCoverageAdvisoryLabel => 'Ориентировочное';

  @override
  String get footprintCoverageOpaqueLabel => 'Непрозрачное';

  @override
  String get conflictsUnverified =>
      'Конфликты не проверены, пока состояние библиотеки не обновлено.';

  @override
  String get componentsTitle => 'Компоненты';

  @override
  String targetsMore(int count) {
    return '+ещё $count';
  }

  @override
  String get removeModDeploymentHint =>
      'Удаление из библиотеки не изменит существующее развёртывание сразу. Если мод уже развёрнут, затем нажмите «Применить», чтобы обновить установленную игру.';

  @override
  String removeModSuccess(String name) {
    return 'Мод «$name» удалён из библиотеки.';
  }

  @override
  String removeModFailed(String name, String error) {
    return 'Не удалось удалить мод «$name»: $error';
  }

  @override
  String removeModPartialFailure(String name, String error) {
    return 'Мод «$name» удалён, но последующая обработка сообщила об ошибке. Состояние библиотеки было перечитано: $error';
  }

  @override
  String removeModOutcomeUnknown(String name, String error) {
    return 'Не удалось проверить, был ли удалён мод «$name»: $error — Обновите состояние библиотеки для проверки.';
  }

  @override
  String get libraryStateUnknown =>
      'Не удалось проверить состояние библиотеки. Нажмите «Обновить» перед изменением или применением модов.';

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
  String get aboutCopyright => '© 2026 участники проекта GORE';

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
