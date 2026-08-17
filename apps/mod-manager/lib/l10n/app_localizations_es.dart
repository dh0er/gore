// ignore: unused_import
import 'package:intl/intl.dart' as intl;
import 'app_localizations.dart';

// ignore_for_file: type=lint

/// The translations for Spanish Castilian (`es`).
class AppLocalizationsEs extends AppLocalizations {
  AppLocalizationsEs([String locale = 'es']) : super(locale);

  @override
  String get coreBlockedTitle => 'Mod Manager no disponible';

  @override
  String get coreDllMissingMessage =>
      'No se encontró el archivo gore_ffi.dll necesario.';

  @override
  String get coreDllLoadFailedMessage =>
      'No se pudo cargar la biblioteca nativa de GORE Core.';

  @override
  String get coreVerificationFailedMessage =>
      'No se pudo verificar la biblioteca nativa de GORE Core.';

  @override
  String get coreManagerTooOldMessage =>
      'Esta versión de GORE Core es más reciente que Mod Manager. Actualiza Mod Manager.';

  @override
  String get coreNativeTooOldMessage =>
      'Esta versión de GORE Core es más antigua que Mod Manager. Actualiza o repara la instalación completa de Mod Manager.';

  @override
  String get coreCommandsMissingMessage =>
      'La biblioteca de GORE Core no proporciona todos los comandos que necesita este Mod Manager.';

  @override
  String get coreBlockedRepairHint =>
      'Actualiza o repara el paquete completo de Mod Manager y reinicia la aplicación.';

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
  String get preflightAttention => 'La configuración necesita atención.';

  @override
  String get preflightUnavailable =>
      'El diagnóstico de configuración no está disponible.';

  @override
  String get preflightRetry => 'Comprobar de nuevo';

  @override
  String get preflightReviewStatus => 'Revisar estado';

  @override
  String get preflightReviewRecovery => 'Ayuda';

  @override
  String get installRecoveryTitle => 'Recuperación de la instalación';

  @override
  String get installRecoveryBody =>
      'GORE encontró datos de recuperación de una instalación o compilación de scripts interrumpida. La reparación automática no es segura porque no se pueden confirmar el proceso anterior ni el estado original de los archivos.';

  @override
  String get installRecoverySteps =>
      'Cierra Gothic, Mod Studio y las demás tareas de GORE. Sigue el archivo README.txt de la carpeta de recuperación indicada abajo. Si no aparece ninguna carpeta, deja sin cambios los datos de recuperación indicados y pide ayuda en vez de borrar nada. No elimines ningún bloqueo mientras haya una tarea en ejecución. Después, vuelve a comprobarlo.';

  @override
  String get installRecoveryEvidence => 'Datos de recuperación detectados';

  @override
  String get statusUnknown => 'Desconocido';

  @override
  String statusDetailsTitle(String status) {
    return 'Despliegue: $status';
  }

  @override
  String statusDetailsOpen(String status) {
    return 'Mostrar detalles del despliegue: $status';
  }

  @override
  String get statusDetailsNoRoot =>
      'Elige una instalación del juego en Ajustes para consultar su estado de despliegue.';

  @override
  String get statusDetailsNoDeployment =>
      'No hay ningún despliegue del gestor instalado para este juego.';

  @override
  String get statusDetailsInSyncDescription =>
      'Los mods desplegados coinciden con la configuración actual.';

  @override
  String get statusDetailsDeployedLoadout => 'Orden de carga desplegado';

  @override
  String get statusDetailsChangesDescription =>
      'El despliegue actual difiere de lo que instalará Aplicar.';

  @override
  String get statusDetailsCurrentlyDeployed => 'Desplegado actualmente';

  @override
  String get statusDetailsAfterApply => 'Después de Aplicar';

  @override
  String get statusDetailsGameUpdatedDescription =>
      'Los archivos del juego cambiaron después del último despliegue. Vuelve a aplicar la configuración para restaurar los archivos del gestor.';

  @override
  String get statusDetailsDriftedFiles => 'Archivos modificados';

  @override
  String get statusDetailsStudioDescription =>
      'Mod Studio controla actualmente esta instalación del juego. Toma el control antes de aplicar una configuración del gestor.';

  @override
  String statusDetailsStudioMod(String name) {
    return 'Mod de Studio: $name';
  }

  @override
  String get statusDetailsStudioNameUnknown =>
      'Studio no indicó el nombre del mod.';

  @override
  String get statusDetailsRecoveryDescription =>
      'Se interrumpió un despliegue. Recupéralo antes de aplicar o eliminar mods del gestor.';

  @override
  String get statusDetailsUnknownDescription =>
      'No se pudo verificar el estado del despliegue. Actualízalo antes de aplicar mods.';

  @override
  String get statusDetailsUnavailable =>
      'El núcleo instalado no proporcionó estos detalles.';

  @override
  String get statusDetailsEmptyLoadout => 'No hay mods en esta configuración.';

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
  String get recoveryAction => 'Recuperar';

  @override
  String get recoveryRequiredConfirm =>
      '¿Recuperar el despliegue interrumpido y eliminar los archivos desplegados parcialmente?';

  @override
  String get statusRecoveryRequired => 'Recuperación necesaria';

  @override
  String get statusDetailsOwnershipTitle => 'Evidencia de propiedad registrada';

  @override
  String get statusDetailsOwnershipDescription =>
      'Rutas registradas en el registro de despliegue del gestor. No demuestran que esas rutas sigan existiendo.';

  @override
  String get statusDetailsOwnershipLive => 'Archivos del juego reemplazados';

  @override
  String get statusDetailsOwnershipBackups => 'Copias de seguridad originales';

  @override
  String get statusDetailsOwnershipAdditive =>
      'Archivos pak y contenedores añadidos';

  @override
  String get statusDetailsOwnershipUe4ss => 'Directorios de mods UE4SS';

  @override
  String get statusDetailsOwnershipRecovery =>
      'Archivos y ubicaciones de recuperación';

  @override
  String get statusDetailsOwnershipEmpty =>
      'No hay rutas registradas en este grupo.';

  @override
  String statusDetailsOwnershipShown(int shown, int total) {
    return 'Se muestran $shown de $total rutas registradas.';
  }

  @override
  String get appTitle => 'GORE Mod Manager';

  @override
  String get tabMods => 'Mods';

  @override
  String get tabSettings => 'Ajustes';

  @override
  String get settingsGameExe => 'Ejecutable del juego';

  @override
  String get settingsGameExePick => 'Elegir…';

  @override
  String get settingsLanguage => 'Idioma';

  @override
  String get statusInSync => 'Sincronizado';

  @override
  String get statusChangesPending => 'Cambios pendientes';

  @override
  String get statusGameUpdated => 'Juego actualizado';

  @override
  String get statusStudioDeploy => 'Despliegue de Studio activo';

  @override
  String get statusNothingDeployed => 'Nada desplegado';

  @override
  String get actionImport => 'Importar';

  @override
  String get actionApply => 'Aplicar';

  @override
  String get actionUndeployAll => 'Retirar todo';

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
    return 'Se añadió «$name» a la biblioteca.';
  }

  @override
  String importOutcomeUpdated(String name) {
    return 'Se actualizó «$name» en la biblioteca.';
  }

  @override
  String importOutcomeUnchanged(String name) {
    return '«$name» ya está en la biblioteca.';
  }

  @override
  String importOutcomeMatchedBy(String method) {
    String _temp0 = intl.Intl.selectLogic(method, {
      'none': 'No hubo coincidencias con entradas existentes de la biblioteca.',
      'source': 'Coincidencia por la misma fuente de importación.',
      'content': 'Coincidencia por contenido idéntico verificado.',
      'entry_id': 'Coincidencia por el ID del mod.',
      'other': 'Los detalles de la coincidencia no están disponibles.',
    });
    return '$_temp0';
  }

  @override
  String get importRefusalDuplicateAmbiguous =>
      'Esta importación coincide con varias entradas de la biblioteca. Revisa o elimina los duplicados y vuelve a intentarlo.';

  @override
  String get importRefusalIdentityConflict =>
      'La fuente de importación y su contenido coinciden con distintas entradas de la biblioteca. Revisa o elimina las entradas en conflicto y vuelve a intentarlo.';

  @override
  String get importFailed =>
      'No se pudo completar la importación. Fuentes compatibles: carpetas, ZIP, archivos *_P.pak sueltos, conjuntos .utoc/.ucas completos (con .pak opcional), .lcache, .bank y PrecompiledScript*.Cache. Extrae primero los archivos .7z o .rar y después importa la carpeta. La fuente puede no ser compatible, estar dañada o incompleta. Es posible que el mod ya se haya añadido o actualizado; actualiza y comprueba la biblioteca antes de volver a intentarlo.';

  @override
  String get importPickerFailed =>
      'No se pudo abrir el selector de archivos o carpetas. No se inició ninguna importación. Inténtalo de nuevo.';

  @override
  String get importOutcomeUnknown =>
      'No se pudo verificar el resultado de la importación. Selecciona Actualizar para comprobar la biblioteca.';

  @override
  String get applyTooltip => 'Aplicar la configuración al juego';

  @override
  String get undeployAllAction => 'Retirar todo';

  @override
  String get undeployAllConfirm =>
      '¿Quitar del juego todo lo que desplegó el gestor?';

  @override
  String get takeOverTitle => 'Despliegue de Studio activo';

  @override
  String get takeOverBody =>
      'mod-studio ha desplegado un mod en el juego. ¿Tomar el control para que el gestor aplique esta configuración?';

  @override
  String get takeOverAction => 'Tomar el control';

  @override
  String get refreshAction => 'Actualizar';

  @override
  String conflictsTitle(int count) {
    return 'Hallazgos ($count)';
  }

  @override
  String get conflictWinner => 'ganador previsto';

  @override
  String get noConflicts => 'No se reconocieron conflictos.';

  @override
  String get conflictCoverageIncomplete =>
      'El conocimiento de conflictos de los mods activados está incompleto; puede haber más conflictos.';

  @override
  String get loadOrderDirection =>
      'Orden de carga: primero la prioridad más baja; los mods posteriores tienen mayor prioridad prevista.';

  @override
  String get footprintCoverageScope =>
      'La cobertura solo describe objetivos de conflicto reconocidos; no demuestra la prioridad en tiempo de ejecución.';

  @override
  String get footprintCoverageExact =>
      'Exacta — la lista de objetivos de conflicto del componente está completa.';

  @override
  String get footprintCoveragePartial =>
      'Parcial — se conocen los objetivos indicados, pero el componente puede afectar a más.';

  @override
  String get footprintCoverageAdvisory =>
      'Orientativa — los objetivos indicados son pistas, no una prueba exhaustiva.';

  @override
  String get footprintCoverageOpaque =>
      'Opaca — se desconocen los objetivos de conflicto del componente.';

  @override
  String get footprintCoverageExactLabel => 'Exacta';

  @override
  String get footprintCoveragePartialLabel => 'Parcial';

  @override
  String get footprintCoverageAdvisoryLabel => 'Orientativa';

  @override
  String get footprintCoverageOpaqueLabel => 'Opaca';

  @override
  String get conflictsUnverified =>
      'Los conflictos no están verificados hasta que se actualice el estado de la biblioteca.';

  @override
  String get componentsTitle => 'Componentes';

  @override
  String targetsMore(int count) {
    return '+$count más';
  }

  @override
  String get removeModDeploymentHint =>
      'Quitar el mod de la biblioteca no cambia inmediatamente un despliegue existente. Si el mod ya está desplegado, selecciona Aplicar después para actualizar la instalación del juego.';

  @override
  String removeModSuccess(String name) {
    return 'Se quitó «$name» de la biblioteca.';
  }

  @override
  String removeModFailed(String name, String error) {
    return 'No se pudo quitar «$name»: $error';
  }

  @override
  String removeModPartialFailure(String name, String error) {
    return 'Se quitó «$name», pero el procesamiento posterior notificó un error. Se volvió a cargar el estado de la biblioteca: $error';
  }

  @override
  String removeModOutcomeUnknown(String name, String error) {
    return 'No se pudo verificar si se quitó «$name»: $error — Actualiza para comprobar el estado de la biblioteca.';
  }

  @override
  String get libraryStateUnknown =>
      'No se pudo verificar el estado de la biblioteca. Selecciona Actualizar antes de cambiar o aplicar mods.';

  @override
  String get removeModAction => 'Quitar';

  @override
  String removeModConfirm(String name) {
    return '¿Quitar «$name» de la biblioteca?';
  }

  @override
  String get errorSetGamePath => 'Primero define la ruta del juego en Ajustes.';

  @override
  String applyReportApplied(int count) {
    return '$count mods aplicados.';
  }

  @override
  String get warningsTitle => 'Advertencias';

  @override
  String get modDisabledHint => 'Desactivado';

  @override
  String get kindGoremod => 'goremod';

  @override
  String get kindTriplet => 'triplet';

  @override
  String get kindPak => 'pak';

  @override
  String get kindUe4ss => 'UE4SS';

  @override
  String get kindRawfile => 'archivo sin procesar';

  @override
  String get kindMixed => 'mixto';

  @override
  String get sevHard => 'grave';

  @override
  String get sevSoft => 'leve';

  @override
  String get sevInfo => 'info';

  @override
  String aboutVersion(String version, String sha) {
    return 'Version $version ($sha)';
  }

  @override
  String get about => 'Acerca de';

  @override
  String get aboutCopyright => '© 2026 colaboradores de GORE';

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
  String get uiScale => 'Escala de la interfaz';

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
