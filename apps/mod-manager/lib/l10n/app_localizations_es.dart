// ignore: unused_import
import 'package:intl/intl.dart' as intl;
import 'app_localizations.dart';

// ignore_for_file: type=lint

/// The translations for Spanish Castilian (`es`).
class AppLocalizationsEs extends AppLocalizations {
  AppLocalizationsEs([String locale = 'es']) : super(locale);

  @override
  String get coreBlockedTitle => 'El Mod Manager no puede iniciarse';

  @override
  String get coreDllMissingMessage =>
      'Falta un archivo necesario del programa (gore_ffi.dll).';

  @override
  String get coreDllLoadFailedMessage =>
      'No se pudo cargar un archivo necesario del programa.';

  @override
  String get coreVerificationFailedMessage =>
      'No se pudo verificar un archivo necesario del programa.';

  @override
  String get coreManagerTooOldMessage =>
      'Los archivos del programa son más nuevos que el Mod Manager. Actualiza el Mod Manager.';

  @override
  String get coreNativeTooOldMessage =>
      'Los archivos del programa son más antiguos que el Mod Manager. Reinstala el Mod Manager.';

  @override
  String get coreCommandsMissingMessage =>
      'A los archivos del programa les faltan funciones que este Mod Manager necesita.';

  @override
  String get coreBlockedRepairHint =>
      'Reinstala o repara el Mod Manager y vuelve a iniciarlo.';

  @override
  String get coreTechnicalDetails => 'Detalles técnicos';

  @override
  String get coreCopyTechnicalDetails => 'Copiar detalles técnicos';

  @override
  String get coreTechnicalDetailsCopied => 'Detalles técnicos copiados';

  @override
  String get coreTechnicalDetailsCopyFailed =>
      'No se pudieron copiar los detalles técnicos. Inténtalo de nuevo.';

  @override
  String get preflightAttention =>
      'Hay algo que resolver antes de poder cambiar mods.';

  @override
  String get preflightGameRunning =>
      'Gothic sigue abierto. Cierra el juego antes de cambiar los mods.';

  @override
  String get managerOperationFailed => 'La operación falló.';

  @override
  String get libraryOperationFailed => 'No se pudo cargar la lista de mods.';

  @override
  String get conflictsUnavailable => 'No se pudieron comprobar los conflictos.';

  @override
  String applyReportAppliedWithWarnings(int applied, int warnings) {
    return 'Aplicados: $applied. Advertencias: $warnings.';
  }

  @override
  String get modDetailKind => 'Tipo';

  @override
  String get modDetailVersion => 'Versión';

  @override
  String get modDetailAuthor => 'Autor';

  @override
  String get modDetailSource => 'Origen';

  @override
  String get modDetailImported => 'Importado';

  @override
  String get componentLocalization => 'Textos';

  @override
  String get componentAudio => 'Sonido';

  @override
  String get componentAngelScript => 'Scripts';

  @override
  String get componentTexture => 'Texturas';

  @override
  String get componentGameFiles => 'Archivos del juego';

  @override
  String get componentVoice => 'Voces';

  @override
  String get componentKindLocalizationPatch => 'Cambios de texto';

  @override
  String get componentKindAudioPatch => 'Cambios de sonido';

  @override
  String get componentKindAngelScriptPatch => 'Cambios de script';

  @override
  String get componentKindTexturePatch => 'Cambios de texturas';

  @override
  String get componentKindLoosePak => 'Archivo PAK';

  @override
  String get componentKindTriplet => 'Contenedor IoStore';

  @override
  String get componentKindUe4ssLua => 'Script de UE4SS';

  @override
  String get componentKindRawFile => 'Archivo';

  @override
  String get componentKindFilePatch => 'Archivo del juego sustituido';

  @override
  String get componentKindPakFilePatch =>
      'Archivo del juego desde un PAK de ~mods';

  @override
  String get componentKindVoiceArchivePatch => 'Voces';

  @override
  String get rawTargetGameText => 'Todos los textos del juego';

  @override
  String get rawTargetGameScripts => 'Todos los scripts del juego';

  @override
  String get rawTargetSoundBank => 'Banco de sonido';

  @override
  String rawTargetSoundBankNamed(String name) {
    return 'Banco de sonido: $name';
  }

  @override
  String get conflictKindLocalization => 'Textos';

  @override
  String get conflictKindAudio => 'Sonido';

  @override
  String get conflictKindAsset => 'Datos del juego';

  @override
  String get conflictKindCdo => 'Valores de objetos';

  @override
  String get conflictKindUe4ssUnknown => 'UE4SS (poco claro)';

  @override
  String get conflictKindScriptModule => 'Script del juego';

  @override
  String get conflictKindVoiceArchive => 'Voces';

  @override
  String get conflictKindRawFile => 'Archivo';

  @override
  String get conflictKindLooseFile => 'Archivo del juego';

  @override
  String get preflightUnavailable =>
      'No se pudo comprobar la instalación del juego.';

  @override
  String get preflightRetry => 'Comprobar de nuevo';

  @override
  String get preflightReviewStatus => 'Ver estado';

  @override
  String get preflightReviewRecovery => 'Ver ayuda';

  @override
  String get installRecoveryTitle => 'Instalación interrumpida';

  @override
  String get installRecoveryBody =>
      'GORE encontró restos de una instalación o de una compilación de scripts. Puede que ese proceso siga en marcha o que ya terminara y dejara esto atrás. GORE no puede limpiarlo por su cuenta de forma segura.';

  @override
  String get installRecoverySteps =>
      'Si el proceso sigue en marcha, espera a que termine: no lo detengas ni borres archivos. Cuando estés seguro de que no hay nada en marcha, sigue el README.txt de la carpeta de abajo y comprueba de nuevo. Si no aparece ninguna carpeta o tienes dudas, déjalo todo como está y pide ayuda.';

  @override
  String get installRecoveryEvidence => 'Lo que encontró GORE';

  @override
  String get managerRecoveryTitle => 'Reparar el cambio interrumpido';

  @override
  String get managerRecoveryConfirm =>
      'GORE encontró un cambio interrumpido y puede devolver el juego a un estado conocido. Tus partidas guardadas nunca se tocan.';

  @override
  String get managerRecoveryAlreadyClean =>
      'No quedaba nada que reparar. Se comprobó el estado de nuevo.';

  @override
  String get managerRecoveryBusy =>
      'El proceso está en marcha otra vez. No se cambió nada: espera a que termine.';

  @override
  String get managerRecoveryLockCleared =>
      'El proceso interrumpido aún no había cambiado nada. Se limpió.';

  @override
  String get managerRecoveryRestoredPristine =>
      'El cambio se deshizo. El juego volvió a su estado anterior.';

  @override
  String get managerRecoveryApplyPreserved =>
      'La aplicación ya había terminado. No se perdió nada.';

  @override
  String get managerRecoveryUndeployConfirmed =>
      'La eliminación ya había terminado. Se limpiaron los restos.';

  @override
  String get managerRecoveryCompileRequired =>
      'Esto pertenece a una compilación de scripts, así que no se cambió nada. Abre la ayuda de reparación.';

  @override
  String get managerRecoveryInspectionFailed =>
      'GORE no pudo comprobar con seguridad el proceso interrumpido. No se cambió nada.';

  @override
  String get managerRecoveryFailed =>
      'No se pudo terminar la reparación. Comprueba el estado antes de volver a intentarlo.';

  @override
  String get statusUnknown => 'Desconocido';

  @override
  String statusDetailsTitle(String status) {
    return 'Estado: $status';
  }

  @override
  String statusDetailsOpen(String status) {
    return 'Ver detalles: $status';
  }

  @override
  String get statusDetailsNoRoot =>
      'Elige primero tu instalación de Gothic en Ajustes.';

  @override
  String get statusDetailsNoDeployment =>
      'Ahora mismo no hay mods instalados en el juego.';

  @override
  String get statusDetailsInSyncDescription =>
      'El juego tiene exactamente los mods que has marcado aquí.';

  @override
  String get statusDetailsDeployedLoadout => 'Mods en el juego';

  @override
  String get statusDetailsChangesDescription =>
      'Tu selección no coincide con lo que hay en el juego.';

  @override
  String get statusDetailsCurrentlyDeployed => 'Ahora en el juego';

  @override
  String get statusDetailsAfterApply => 'Tras aplicar';

  @override
  String get statusDetailsGameUpdatedDescription =>
      'El juego se actualizó y sobrescribió archivos de mods. Aplica de nuevo para restaurarlos.';

  @override
  String get statusDetailsDriftedFiles => 'Archivos afectados';

  @override
  String get statusDetailsStudioDescription =>
      'Mod Studio tiene ahora mods en este juego. Toma el control del juego antes de que el Manager aplique los tuyos.';

  @override
  String statusDetailsStudioMod(String name) {
    return 'Mod de Studio: $name';
  }

  @override
  String get statusDetailsStudioNameUnknown =>
      'Mod Studio no indicó ningún nombre.';

  @override
  String get statusDetailsRecoveryDescription =>
      'Un cambio quedó interrumpido. Repáralo antes de cambiar mods.';

  @override
  String get statusDetailsUnknownDescription =>
      'No se pudo leer el estado. Actualiza primero.';

  @override
  String get statusDetailsUnavailable => 'No hay detalles disponibles.';

  @override
  String get statusDetailsEmptyLoadout => 'Sin mods.';

  @override
  String get statusDetailsLastError => 'Último error';

  @override
  String get statusDetailsLastApply => 'Última aplicación';

  @override
  String get statusDetailsAppliedMods => 'Mods aplicados';

  @override
  String get statusDetailsWarnings => 'Advertencias';

  @override
  String get statusDetailsReapply => 'Volver a aplicar';

  @override
  String get statusDetailsOpenSettings => 'Abrir Ajustes';

  @override
  String get recoveryAction => 'Reparar';

  @override
  String get recoveryRequiredConfirm =>
      '¿Reparar el cambio interrumpido y quitar los archivos instalados a medias?';

  @override
  String get statusRecoveryRequired => 'Requiere reparación';

  @override
  String get statusDetailsOwnershipTitle => 'Archivos que gestiona GORE';

  @override
  String get statusDetailsOwnershipDescription =>
      'Registrado al aplicar los mods; no comprueba que los archivos sigan existiendo.';

  @override
  String get statusDetailsOwnershipLive => 'Archivos del juego reemplazados';

  @override
  String get statusDetailsOwnershipBackups => 'Copias de los originales';

  @override
  String get statusDetailsOwnershipAdditive => 'Archivos de mods añadidos';

  @override
  String get statusDetailsOwnershipUe4ss => 'Directorios de mods UE4SS';

  @override
  String get statusDetailsOwnershipRecovery => 'Archivos de reparación';

  @override
  String get statusDetailsOwnershipEmpty => 'Aquí no hay nada registrado.';

  @override
  String statusDetailsOwnershipShown(int shown, int total) {
    return 'Mostrando $shown de $total rutas.';
  }

  @override
  String get appTitle => 'GORE Mod Manager';

  @override
  String get tabMods => 'Mods';

  @override
  String get tabSettings => 'Ajustes';

  @override
  String get settingsGameExe => 'Instalación de Gothic';

  @override
  String get settingsGameExePick => 'Elegir…';

  @override
  String get settingsLanguage => 'Idioma';

  @override
  String get libraryEmptyTitle => 'Aún no hay mods';

  @override
  String get libraryEmptyBody =>
      'Importa una carpeta o un archivo de mod para empezar.';

  @override
  String get detailEmptyHint => 'Elige un mod para ver qué cambia.';

  @override
  String get settingsAdvanced => 'Detalles avanzados';

  @override
  String get settingsAdvancedHint =>
      'Muestra la parte técnica: entradas afectadas, qué tan fiable es la comprobación de conflictos y los archivos que gestiona GORE.';

  @override
  String get updatesTitle => 'Actualizaciones';

  @override
  String get checkForUpdatesAutomatically =>
      'Buscar actualizaciones automáticamente';

  @override
  String get checkForUpdatesNow => 'Buscar actualizaciones ahora';

  @override
  String get updatesPortableNotice =>
      'La versión portátil abre la página de descarga en tu navegador. Sustituye tus archivos actuales por la nueva descarga.';

  @override
  String get updateCheckFailed =>
      'No se pudo buscar actualizaciones. Inténtalo más tarde.';

  @override
  String get updateUpToDate => 'Estás usando la última versión.';

  @override
  String get updateAvailableTitle => 'Actualización disponible';

  @override
  String updateAvailableMessage(String version, String current) {
    return 'La versión $version está disponible. Tienes la $current.';
  }

  @override
  String get updateLater => 'Más tarde';

  @override
  String get updateDownload => 'Descargar';

  @override
  String get statusInSync => 'Al día';

  @override
  String get statusChangesPending => 'Sin aplicar';

  @override
  String get statusGameUpdated => 'El juego se actualizó';

  @override
  String get statusStudioDeploy => 'Mod Studio activo';

  @override
  String get statusNothingDeployed => 'Sin mods en el juego';

  @override
  String get actionImport => 'Importar';

  @override
  String get actionApply => 'Aplicar';

  @override
  String get actionStartGame => 'Iniciar el juego';

  @override
  String get startGameTooltip =>
      'Iniciar Gothic con los mods que hay ahora en el juego';

  @override
  String get startGameFailed =>
      'No se pudo iniciar Gothic. Comprueba la instalación del juego en Ajustes.';

  @override
  String get commonCancel => 'Cancelar';

  @override
  String get commonOk => 'OK';

  @override
  String get importFolder => 'Importar carpeta…';

  @override
  String get importFile => 'Importar archivo…';

  @override
  String importOutcomeCreated(String name) {
    return 'Se añadió «$name».';
  }

  @override
  String importOutcomeUpdated(String name) {
    return 'Se actualizó «$name».';
  }

  @override
  String importOutcomeUnchanged(String name) {
    return '«$name» ya está en tu lista.';
  }

  @override
  String importOutcomeMatchedBy(String method) {
    String _temp0 = intl.Intl.selectLogic(method, {
      'none': 'Ningún mod existente coincidió.',
      'source': 'Coincidencia por la misma fuente de importación.',
      'content': 'Coincidencia por contenido idéntico verificado.',
      'entry_id': 'Coincidencia por ID del mod.',
      'other': 'No hay detalles de la coincidencia.',
    });
    return '$_temp0';
  }

  @override
  String get importRefusalDuplicateAmbiguous =>
      'Esto coincide con más de un mod que ya tienes. Quita los duplicados e inténtalo de nuevo.';

  @override
  String get importRefusalIdentityConflict =>
      'El origen y el contenido coinciden con dos mods distintos que ya tienes. Resuélvelo e inténtalo de nuevo.';

  @override
  String get importFailed =>
      'No se pudo importar. Se admiten carpetas, archivos ZIP y archivos de mod sueltos (*_P.pak, .utoc/.ucas, .lcache, .bank, PrecompiledScript*.Cache). Extrae primero los .7z o .rar y luego importa la carpeta. Puede que el mod se haya añadido o actualizado igualmente: actualiza la lista antes de volver a intentarlo.';

  @override
  String get importPickerFailed =>
      'No se pudo abrir el selector de archivos. No se importó nada.';

  @override
  String get importOutcomeUnknown =>
      'El resultado no está claro. Actualiza para revisar tu lista de mods.';

  @override
  String get applyTooltip => 'Instalar en el juego los mods marcados';

  @override
  String get undeployAllAction => 'Quitar todo del juego';

  @override
  String get undeployAllConfirm =>
      '¿Quitar del juego todos los mods que instaló el Manager?';

  @override
  String get takeOverTitle => 'Mod Studio está activo';

  @override
  String get takeOverBody =>
      'Mod Studio tiene ahora un mod en el juego. ¿Tomar el control para que el Manager aplique tu selección?';

  @override
  String get takeOverAction => 'Tomar el control';

  @override
  String get refreshAction => 'Actualizar';

  @override
  String conflictsTitle(int count) {
    return 'Conflictos ($count)';
  }

  @override
  String get conflictWinner => 'gana';

  @override
  String get noConflicts => 'No se encontraron conflictos.';

  @override
  String get conflictCoverageIncomplete =>
      'Algunos mods no se pueden comprobar del todo, así que puede haber más conflictos.';

  @override
  String get loadOrderDirection =>
      'Los mods más abajo en la lista sustituyen a los de arriba.';

  @override
  String get footprintCoverageScope =>
      'Solo se listan los objetivos de conflicto conocidos. No garantiza lo que ocurre en el juego.';

  @override
  String get footprintTargetsExact => 'Entradas afectadas: la lista completa:';

  @override
  String get footprintTargetsPartial => 'Entradas afectadas: puede haber más:';

  @override
  String get footprintTargetsAdvisory =>
      'Entradas probablemente afectadas: indicios, no pruebas:';

  @override
  String get footprintTargetsOpaque => 'GORE no puede saber qué cambia esto.';

  @override
  String get conflictsUnverified =>
      'Conflictos desconocidos: actualiza primero.';

  @override
  String get componentsTitle => 'Qué cambia este mod';

  @override
  String targetsMore(int count) {
    return '+$count más';
  }

  @override
  String get removeModDeploymentHint =>
      'Esto solo la quita de tu lista. Si está instalado en el juego, elige Aplicar después.';

  @override
  String removeModSuccess(String name) {
    return 'Se quitó «$name».';
  }

  @override
  String removeModFailed(String name) {
    return 'No se pudo quitar «$name».';
  }

  @override
  String removeModPartialFailure(String name) {
    return 'Se quitó «$name», pero la lista no se pudo actualizar del todo.';
  }

  @override
  String removeModOutcomeUnknown(String name) {
    return 'No se pudo confirmar si «$name» se quitó.';
  }

  @override
  String get libraryStateUnknown =>
      'La lista de mods no está al día. Actualiza antes de cambiar o aplicar mods.';

  @override
  String get removeModAction => 'Quitar';

  @override
  String removeModConfirm(String name) {
    return '¿Quitar «$name» de tu lista?';
  }

  @override
  String get errorSetGamePath =>
      'Elige primero tu instalación de Gothic en Ajustes.';

  @override
  String applyReportApplied(int count) {
    return '$count mods aplicados.';
  }

  @override
  String get modDisabledHint => 'Desactivado';

  @override
  String get kindGoremod => 'Paquete GORE';

  @override
  String get kindTriplet => 'Mod IoStore';

  @override
  String get kindPak => 'Mod PAK';

  @override
  String get kindUe4ss => 'UE4SS';

  @override
  String get kindRawfile => 'Sustitución de archivos completos';

  @override
  String get kindMixed => 'Mixto';

  @override
  String get sevHard => 'Conflicto';

  @override
  String get sevSoft => 'Aviso';

  @override
  String get sevInfo => 'Nota';

  @override
  String aboutVersion(String version, String sha) {
    return 'Version $version ($sha)';
  }

  @override
  String get about => 'Acerca de';

  @override
  String get aboutCopyright => '© 2026 Daniel Hoer';

  @override
  String get aboutLicense => 'Distribuido bajo la licencia MIT.';

  @override
  String get appearanceTitle => 'Apariencia';

  @override
  String get theme => 'Tema';

  @override
  String get themeLight => 'Claro';

  @override
  String get themeDark => 'Oscuro';

  @override
  String get themeSystem => 'Sistema';

  @override
  String get uiScale => 'Tamaño de pantalla';

  @override
  String get resetZoomTooltip => 'Restablecer el zoom (Ctrl+0)';

  @override
  String get zoomTip =>
      'Consejo: Ctrl + / Ctrl - cambia el zoom en cualquier parte de la aplicación.';

  @override
  String get lightMode => 'Modo claro';

  @override
  String get darkMode => 'Modo oscuro';

  @override
  String get minimize => 'Minimizar';

  @override
  String get restore => 'Restaurar';

  @override
  String get maximize => 'Maximizar';

  @override
  String get close => 'Cerrar';
}
