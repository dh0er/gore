// ignore: unused_import
import 'package:intl/intl.dart' as intl;
import 'app_localizations.dart';

// ignore_for_file: type=lint

/// The translations for Spanish Castilian (`es`).
class AppLocalizationsEs extends AppLocalizations {
  AppLocalizationsEs([String locale = 'es']) : super(locale);

  @override
  String get tabItems => 'Items';

  @override
  String get tabOverrides => 'Changes';

  @override
  String get tabSettings => 'Settings';

  @override
  String get tabDialogs => 'Diálogos';

  @override
  String get tabAudio => 'Audio';

  @override
  String get tabTextures => 'Texturas';

  @override
  String get tabScripts => 'Scripts';

  @override
  String get changesAll => 'Todos';

  @override
  String get sectionItemValues => 'Valores de objetos';

  @override
  String get sectionLocalizedText => 'Textos localizados';

  @override
  String get audioCatCreatures => 'Criaturas';

  @override
  String get audioCatObjects => 'Objetos';

  @override
  String get audioCatMagic => 'Magia';

  @override
  String get audioCatMovement => 'Movimiento';

  @override
  String get audioCatWorld => 'Mundo';

  @override
  String get audioCatAction => 'Acciones';

  @override
  String get audioCatCombat => 'Combate';

  @override
  String get audioCatPhysics => 'Física';

  @override
  String get audioCatItems => 'Ítems';

  @override
  String get audioCatUi => 'Interfaz';

  @override
  String get audioCatFoley => 'Foley';

  @override
  String get audioCatUnderwater => 'Bajo el agua';

  @override
  String get audioCatVision => 'Visiones';

  @override
  String get audioCatDialog => 'Diálogo';

  @override
  String get audioCatOther => 'Otros';

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
  String get extractLocalizedText => 'Extraer textos localizados';

  @override
  String get lightMode => 'Modo claro';

  @override
  String get darkMode => 'Modo oscuro';

  @override
  String get language => 'Idioma';

  @override
  String get exportMod => 'Exportar mod';

  @override
  String exportModWithCount(int count) {
    return 'Exportar mod ($count)';
  }

  @override
  String get selectAnItemToEdit =>
      'Selecciona un objeto para editar sus campos.';

  @override
  String gameDataActiveTooltip(String name) {
    return 'Datos del juego: $name';
  }

  @override
  String get gameDataBundledTooltip => 'Datos del juego: incluidos';

  @override
  String get loadGameDataDump => 'Cargar volcado de datos del juego…';

  @override
  String get loadGameDataDumpSubtitle =>
      'gore_game_data.json del mod gore-dump';

  @override
  String get useBundledData => 'Usar los datos incluidos';

  @override
  String get alreadyBundled => 'ya incluidos';

  @override
  String get gameDataFileGroupLabel => 'datos del juego';

  @override
  String get minimize => 'Minimizar';

  @override
  String get restore => 'Restaurar';

  @override
  String get maximize => 'Maximizar';

  @override
  String get close => 'Cerrar';

  @override
  String get about => 'Acerca de';

  @override
  String aboutVersion(String version, String sha) {
    return 'Version $version ($sha)';
  }

  @override
  String get aboutCopyright => '© 2026 colaboradores de GORE';

  @override
  String get aboutLicense => 'Distribuido bajo la licencia MIT.';

  @override
  String get categoryMeleeWeapons => 'Armas cuerpo a cuerpo';

  @override
  String get categoryRangedWeapons => 'Armas a distancia';

  @override
  String get categoryAmmunition => 'Munición';

  @override
  String get categoryRunes => 'Runas';

  @override
  String get categorySpellScrolls => 'Pergaminos de hechizo';

  @override
  String get categoryFoodAndPotions => 'Comida y pociones';

  @override
  String get categoryMiscellaneous => 'Varios';

  @override
  String get categoryAmulets => 'Amuletos';

  @override
  String get categoryRings => 'Anillos';

  @override
  String get categoryAnimalTrophies => 'Trofeos de animales';

  @override
  String get categoryWritings => 'Escritos';

  @override
  String get categoryMissionItems => 'Objetos de misión';

  @override
  String get categoryKeys => 'Llaves';

  @override
  String get categoryOther => 'Otros';

  @override
  String categoryWithCount(String label, int count) {
    return '$label ($count)';
  }

  @override
  String get searchItems => 'Buscar objetos';

  @override
  String get noItemsMatch => 'Ningún objeto coincide';

  @override
  String failedToLoadCatalog(String error) {
    return 'No se pudo cargar el catálogo: $error';
  }

  @override
  String pendingOverridesWithCount(int count) {
    return 'Cambios pendientes ($count)';
  }

  @override
  String get clearAll => 'Borrar todo';

  @override
  String get noPendingOverrides =>
      'No hay cambios pendientes.\nEdita los campos de los objetos para añadir alguno.';

  @override
  String get removeOverride => 'Quitar cambio';

  @override
  String get searchChanges => 'Buscar cambios';

  @override
  String get noChangesMatch => 'Ningún cambio coincide';

  @override
  String get clearSection => 'Borrar este grupo';

  @override
  String get modName => 'Nombre del mod';

  @override
  String get loadDelayLabel => 'Retardo de carga (ms, 0 = inmediato)';

  @override
  String get noFolderSelected => 'Ninguna carpeta seleccionada';

  @override
  String get chooseFolder => 'Elegir carpeta';

  @override
  String get packageAsZip => 'Empaquetar como .zip';

  @override
  String get cancel => 'Cancelar';

  @override
  String get export => 'Exportar';

  @override
  String get exportHere => 'Exportar aquí';

  @override
  String get mustBeNonNegativeInteger => 'Debe ser un entero no negativo';

  @override
  String get extractingLocalizedText =>
      'Extrayendo los textos localizados del juego…';

  @override
  String get localizedTextExtractionCancelled =>
      'Extracción de textos localizados cancelada.';

  @override
  String get localizedTextExtracted => 'Textos localizados extraídos.';

  @override
  String get extractionFailed => 'Error en la extracción.';

  @override
  String get localizationCacheFileGroupLabel => 'caché de localización';

  @override
  String get extractLocalizedTextQuestion =>
      '¿Extraer los textos localizados del juego?';

  @override
  String get extractLocalizedTextBody =>
      'Los textos localizados del juego aún no se han extraído. ¿Extraerlos ahora desde tu instalación del juego? (opcional)';

  @override
  String get notNow => 'Ahora no';

  @override
  String get extract => 'Extraer';

  @override
  String get validationRequired => 'Obligatorio';

  @override
  String get validationMustBeWholeNumber => 'Debe ser un número entero';

  @override
  String get validationMustBeNumber => 'Debe ser un número';

  @override
  String get validationMustBeFinite => 'Debe ser un número finito';

  @override
  String validationMustBeAtLeast(String min) {
    return 'Debe ser ≥ $min';
  }

  @override
  String validationMustBeAtMost(String max) {
    return 'Debe ser ≤ $max';
  }

  @override
  String get validationMustBeBool => 'Debe ser true o false';

  @override
  String validationMustBeOneOf(String options) {
    return 'Debe ser uno de: $options';
  }

  @override
  String get modNameRequired => 'Obligatorio';

  @override
  String get modNameControlCharacters =>
      'No debe contener caracteres de control';

  @override
  String get modNamePathSeparators => 'No debe contener separadores de ruta';

  @override
  String get modNameNotAFolderName => 'Nombre de carpeta no válido';

  @override
  String localizedTextExtractedCount(int idCount, int languageCount) {
    return '$idCount ID extraídos en $languageCount idiomas';
  }

  @override
  String get managerDeployActive =>
      'Hay un loadout del mod-manager activo. Haz primero el undeploy en gore-manager.';

  @override
  String get projectOpenLegacy => 'Open legacy project…';

  @override
  String get projectOpenManagedRevision3 => 'Open managed revision-3 project…';

  @override
  String get projectVerifyCurrentHead => 'Verify current head';

  @override
  String get projectManagedRevision3Title => 'Managed revision-3 project';

  @override
  String get projectManagedRevision3IdentityOnly =>
      'This shell currently exposes verified project identity only. Ctrl+S reopens and verifies the exact current head; legacy editors, Build/Deploy, and Save As are unavailable.';

  @override
  String get projectRoot => 'Root';

  @override
  String get projectId => 'Project ID';

  @override
  String get projectRevision => 'Project revision';

  @override
  String get projectHeadSha256 => 'Head SHA-256';

  @override
  String get projectSnapshotBytes => 'Snapshot bytes';

  @override
  String get projectNoCurrent => 'No current project';

  @override
  String projectManagedRevision3Opened(String projectId) {
    return 'Opened managed revision-3 project $projectId';
  }

  @override
  String projectManagedRevision3OpenFailed(String error) {
    return 'Managed revision-3 project open failed: $error';
  }

  @override
  String projectManagedRevision3Verified(String headSha256) {
    return 'Verified revision-3 head $headSha256';
  }

  @override
  String projectManagedRevision3VerifyFailed(String error) {
    return 'Revision-3 head verification failed: $error';
  }

  @override
  String get projectManagedRevision3RequiresReopen =>
      'Exact-head verification could not complete safely. This session now requires recovery and further verification is blocked. Close Mod Studio, then reopen this project before continuing.';

  @override
  String get projectManagedRevision3VerifyBlocked =>
      'Verification is blocked until the managed project is reopened.';

  @override
  String get projectTransitionCleanupWarning =>
      'El nuevo proyecto está abierto, pero no se pudo limpiar por completo la sesión del proyecto anterior. No se volverá a intentar la limpieza. Reinicia Mod Studio antes de volver a abrir el proyecto anterior.';

  @override
  String get projectNewManagedRevision3 => 'Nuevo proyecto de mod gestionado…';

  @override
  String get projectNewLegacy => 'Nuevo proyecto legacy';

  @override
  String get projectCreateGamePathRequired =>
      'Configura la ruta de Gothic 1 Remake en Ajustes antes de crear un proyecto de mod.';

  @override
  String get projectCreateDirectoryPickerTitle =>
      'Crear aquí el proyecto de mod gestionado';

  @override
  String projectManagedRevision3Created(String projectId) {
    return 'Proyecto de mod gestionado $projectId creado';
  }

  @override
  String projectManagedRevision3CreateFailed(String error) {
    return 'No se pudo crear el proyecto de mod gestionado: $error';
  }

  @override
  String get projectCreateDialogTitle => 'Crear un proyecto de mod';

  @override
  String get projectCreateNameLabel => 'Nombre del proyecto';

  @override
  String get projectCreateNameHelper =>
      'El nombre que se muestra en Mod Studio.';

  @override
  String get projectCreateVersionLabel => 'Versión';

  @override
  String get projectCreateVersionHelper => 'Una versión inicial, como 0.1.0.';

  @override
  String get projectCreateAuthorLabel => 'Autor';

  @override
  String get projectCreateAuthorHelper =>
      'Tu nombre o el de tu equipo de modding.';

  @override
  String get projectCreateLocalesLabel => 'Idiomas de edición';

  @override
  String get projectCreateLocalesHelper =>
      'Etiquetas canónicas separadas por comas, por ejemplo: en, de, en-US.';

  @override
  String get projectCreateBoundary =>
      'Esto crea un proyecto offline gestionado y vacío. No compila, instala ni ejecuta un mod, y no modifica los archivos del juego ni las partidas guardadas.';

  @override
  String get projectCreateSubmit => 'Crear proyecto';

  @override
  String projectCreateMetadataRequired(String label) {
    return '$label es obligatorio.';
  }

  @override
  String projectCreateMetadataNoOuterWhitespace(String label) {
    return '$label no puede empezar ni terminar con espacios.';
  }

  @override
  String projectCreateMetadataControlCharacters(String label) {
    return '$label no puede contener caracteres de control.';
  }

  @override
  String projectCreateMetadataMalformed(String label) {
    return '$label contiene texto no válido.';
  }

  @override
  String projectCreateMetadataTooLong(String label, int maxBytes) {
    return '$label supera el límite UTF-8 de $maxBytes bytes.';
  }

  @override
  String get projectCreateLocalesRequired =>
      'Introduce al menos un idioma de edición.';

  @override
  String get projectCreateLocalesEmptyEntry =>
      'Elimina la entrada vacía de idioma.';

  @override
  String projectCreateLocalesTooMany(int maxLocales) {
    return 'Usa como máximo $maxLocales idiomas de edición.';
  }

  @override
  String projectCreateLocaleBoundedAscii(String locale) {
    return 'La configuración regional «$locale» debe ser ASCII y tener una longitud limitada.';
  }

  @override
  String projectCreateLocaleLanguage(String locale) {
    return 'La configuración regional «$locale» necesita un idioma en minúsculas de 2 a 8 letras.';
  }

  @override
  String projectCreateLocaleInvalidSegment(String locale) {
    return 'La configuración regional «$locale» contiene un segmento no válido.';
  }

  @override
  String projectCreateLocaleNotCanonical(String locale, String canonical) {
    return 'La configuración regional «$locale» no es canónica; usa «$canonical».';
  }

  @override
  String get managedWorkspaceOverviewLabel => 'Resumen';

  @override
  String get managedWorkspaceContentLabel => 'Contenido';

  @override
  String get managedWorkspaceDataAssetsLabel => 'DataAssets';

  @override
  String get managedContentWorkspaceLibraryLabel => 'Este mod';

  @override
  String get managedWorkspaceHomeLabel => 'Inicio';

  @override
  String get managedWorkspaceStoryLabel => 'Historia';

  @override
  String get managedWorkspaceWorldLabel => 'Mundo';

  @override
  String get managedWorkspaceLocalizationVoiceLabel => 'Localización y voces';

  @override
  String get managedWorkspaceValidateTestLabel => 'Validar y probar';

  @override
  String get managedWorkspaceBuildReleaseLabel => 'Compilar y publicar';

  @override
  String get managedWorkspaceSettingsExpertLabel => 'Ajustes y modo experto';

  @override
  String get managedSectionStoryDescription => 'NPC, misiones y diálogos.';

  @override
  String get managedSectionWorldDescription =>
      'La colocación en el mundo y sus flujos de trabajo están planificados.';

  @override
  String get managedSectionLocalizationVoiceDescription =>
      'Las herramientas de producción de voz están disponibles; la edición de localización en proyectos gestionados está planificada.';

  @override
  String get managedSectionValidateTestDescription =>
      'Verifica la integridad exacta del proyecto y sus puntos de control; no implica una prueba en ejecución.';

  @override
  String get managedSectionBuildReleaseDescription =>
      'Los paquetes de voces están disponibles; las compilaciones jugables completas y el despliegue no lo están.';

  @override
  String get managedSectionSettingsExpertDescription =>
      'Los ajustes están disponibles; las herramientas expertas aún no están integradas.';

  @override
  String get managedSectionStatusHeading => 'Estado';

  @override
  String get managedSectionActionsHeading => 'Acciones';

  @override
  String get managedCapabilityAvailable => 'Disponible';

  @override
  String get managedCapabilityPartial => 'Parcial';

  @override
  String get managedCapabilityPlanned => 'Planificado';

  @override
  String get managedCapabilityUnavailable => 'No disponible';

  @override
  String get managedProjectSubtitle =>
      'Espacio de autoría sin conexión con la versión actual exacta';

  @override
  String get managedProjectLandingTitle =>
      'Espacio de trabajo de proyectos gestionados';

  @override
  String get managedProjectLandingDescription =>
      'Usa el nuevo flujo de Inicio, Contenido, Historia, Voz, validación y publicación en un único proyecto gestionado.';

  @override
  String get legacyCompatibilityToolsTitle =>
      'Herramientas de compatibilidad heredadas';

  @override
  String get legacyCompatibilityToolsDescription =>
      'Las pestañas de abajo contienen herramientas antiguas de reemplazo directo. Seguirán disponibles mientras ampliamos el espacio de trabajo de proyectos gestionados.';

  @override
  String get managedProjectTechnicalDetails => 'Detalles técnicos del proyecto';

  @override
  String get managedProjectRecoveryContentLocked =>
      'Vuelve a abrir el proyecto gestionado antes de leer su contenido.';

  @override
  String get managedDashboardUntitledProject => 'Proyecto sin título';

  @override
  String get managedDashboardDraftStatus => 'Borrador';

  @override
  String get managedDashboardProjectVersion => 'Versión';

  @override
  String get managedDashboardProjectAuthor => 'Autor';

  @override
  String get managedDashboardNotProvided => 'No especificado';

  @override
  String get managedDashboardContentCounts => 'Contenido del proyecto';

  @override
  String get managedDashboardNpcDrafts => 'Borradores de PNJ';

  @override
  String get managedDashboardQuestDrafts => 'Borradores de misiones';

  @override
  String get managedDashboardDialogLines => 'Líneas de diálogo';

  @override
  String get managedDashboardVoiceTakes => 'Tomas de voz';

  @override
  String get managedDashboardAssets => 'Recursos';

  @override
  String get managedDashboardUnresolvedReferences => 'Referencias sin resolver';

  @override
  String get managedDashboardReadiness => 'Qué funciona ahora';

  @override
  String get managedDashboardOfflineAuthoringTitle =>
      'Edición sin conexión disponible';

  @override
  String get managedDashboardOfflineAuthoringDescription =>
      'Crea y edita contenido de proyecto compatible sin modificar la instalación del juego ni los archivos de guardado.';

  @override
  String get managedDashboardGeneralBuildBlockedTitle =>
      'Compilación general de mods no disponible';

  @override
  String get managedDashboardGeneralBuildBlockedDescription =>
      'Solo se pueden compilar paquetes de voz sellados sin conexión; todavía no se puede compilar un mod completo y jugable.';

  @override
  String get managedDashboardRuntimeUnqualifiedTitle =>
      'Ejecución aún no verificada';

  @override
  String get managedDashboardRuntimeUnqualifiedDescription =>
      'Mod Studio aún no ha comprobado este contenido del proyecto dentro del juego en ejecución.';

  @override
  String get managedDashboardReferenceIntegrityTitle =>
      'Integridad de las referencias';

  @override
  String get managedDashboardReferenceIntegrityDescription =>
      'Este recuento solo comprueba las referencias del proyecto; no confirma que esté listo para compilarse o ejecutarse.';

  @override
  String get managedDashboardMissingGameTitle =>
      'Se requiere configurar el juego';

  @override
  String get managedDashboardMissingGameDescription =>
      'Configura la instalación de Gothic 1 Remake en Ajustes antes de usar acciones que necesiten datos verificados del juego instalado.';

  @override
  String get managedDashboardCreateHeading => 'Crear';

  @override
  String get managedDashboardToolsHeading => 'Herramientas del proyecto';

  @override
  String get managedDashboardLoading => 'Cargando el resumen del proyecto';

  @override
  String get managedDashboardLoadError => 'Resumen del proyecto no disponible';

  @override
  String get managedDashboardLoadErrorDescription =>
      'No se pudo cargar el resumen verificado del proyecto. El contenido del proyecto no se modificó.';

  @override
  String get managedDashboardRetry => 'Reintentar';

  @override
  String get managedActionNewNpcTitle => 'Nuevo PNJ';

  @override
  String get managedActionNewNpcDescription =>
      'Crea un borrador de PNJ sin conexión y de alcance limitado a partir de datos verificados del juego instalado.';

  @override
  String get managedActionNewQuestTitle => 'Nueva misión';

  @override
  String get managedActionNewQuestDescription =>
      'Crea un borrador de misión sin conexión con objetivos e identidades superiores verificadas.';

  @override
  String get managedActionAddVoiceTakeTitle => 'Añadir toma de voz';

  @override
  String get managedActionAddVoiceTakeDescription =>
      'Importa una grabación Ogg Vorbis en este proyecto sin desplegarla.';

  @override
  String get managedActionManageVoiceTakesTitle => 'Gestionar tomas de voz';

  @override
  String get managedActionManageVoiceTakesDescription =>
      'Revisa las tomas y selecciona grabaciones aprobadas para los espacios de voz.';

  @override
  String get managedActionResolveVoiceTargetTitle => 'Resolver destino de voz';

  @override
  String get managedActionResolveVoiceTargetDescription =>
      'Asocia los espacios de voz del proyecto con miembros exactos de los archivos instalados sin modificar el juego.';

  @override
  String get managedActionBuildVoiceBundleTitle => 'Compilar paquete de voz';

  @override
  String get managedActionBuildVoiceBundleDescription =>
      'Compila un paquete sellado sin conexión con miembros existentes; no se realiza ningún despliegue.';

  @override
  String get managedActionDataAssetsTitle => 'Ediciones de DataAssets';

  @override
  String get managedActionDataAssetsDescription =>
      'Inspecciona paquetes instalados y prepara en el proyecto ediciones verificadas de valores de ancho fijo.';

  @override
  String get managedActionBrowseProjectContentDescription =>
      'Explora el contenido exacto del proyecto y sus referencias resueltas o sin resolver.';

  @override
  String get managedActionSettingsTitle => 'Ajustes';

  @override
  String get managedActionSettingsDescription =>
      'Configura la instalación de Gothic 1 Remake y las preferencias de Mod Studio.';
}
