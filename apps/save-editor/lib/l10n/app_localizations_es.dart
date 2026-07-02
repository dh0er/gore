// ignore: unused_import
import 'package:intl/intl.dart' as intl;
import 'app_localizations.dart';

// ignore_for_file: type=lint

/// The translations for Spanish Castilian (`es`).
class AppLocalizationsEs extends AppLocalizations {
  AppLocalizationsEs([String locale = 'es']) : super(locale);

  @override
  String get appTitle => 'Editor de partidas guardadas de Gothic Remake';

  @override
  String get appLogoSemanticLabel => 'Logotipo de goresave';

  @override
  String get zoomTooltip => 'Pulsa Ctrl +/- para acercar o alejar';

  @override
  String get switchToLightMode => 'Cambiar al modo claro';

  @override
  String get switchToDarkMode => 'Cambiar al modo oscuro';

  @override
  String get about => 'Acerca de';

  @override
  String get tabOverview => 'Resumen';

  @override
  String get tabPlayer => 'Personaje';

  @override
  String get tabAttribute => 'Atributos';

  @override
  String get tabInventory => 'Inventario';

  @override
  String get tabProgression => 'Progreso';

  @override
  String get tabCharacters => 'Personajes';

  @override
  String get characterNoActorBody =>
      'Este personaje no tiene un actor en el mundo, por lo que no tiene atributos, inventario ni eventos.';

  @override
  String get tabAllData => 'Todos los datos';

  @override
  String get tabBackups => 'Copias de seguridad';

  @override
  String get tabSettings => 'Ajustes';

  @override
  String get reset => 'Restablecer';

  @override
  String get save => 'Guardar';

  @override
  String saveWithCount(int count) {
    return 'Guardar ($count)';
  }

  @override
  String get ok => 'Aceptar';

  @override
  String get cancel => 'Cancelar';

  @override
  String get confirm => 'Confirmar';

  @override
  String get close => 'Cerrar';

  @override
  String get add => 'Añadir';

  @override
  String get equippedBadge => 'Equipado';

  @override
  String get armorUpgradesLabel => 'Mejoras';

  @override
  String get browse => 'Examinar';

  @override
  String get noSavFilesFound => 'No se encontraron archivos .sav';

  @override
  String get profile => 'Perfil';

  @override
  String profileWithSaves(String name, int count) {
    return '$name ($count partidas)';
  }

  @override
  String get switchProfile => 'Cambiar de perfil';

  @override
  String get rescanSaveFolder => 'Volver a examinar la carpeta de guardado';

  @override
  String get discardUnsavedChangesTitle =>
      '¿Descartar los cambios sin guardar?';

  @override
  String rescanDiscardBody(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: 'cambios sin guardar',
      one: 'cambio sin guardar',
    );
    return 'Al volver a examinar se recargan todas las partidas y se descartan tus $count $_temp0.';
  }

  @override
  String get discardAndRescan => 'Descartar y volver a examinar';

  @override
  String chapterLabel(Object id) {
    return 'Capítulo $id';
  }

  @override
  String get quickSave => 'Guardado rápido';

  @override
  String get autoSave => 'Guardado automático';

  @override
  String get manualSave => 'Guardado manual';

  @override
  String get errorTitle => 'Error';

  @override
  String get selectASaveTitle => 'Selecciona una partida';

  @override
  String get selectASaveBody => 'Los detalles de la partida aparecerán aquí.';

  @override
  String get diagnosticsTitle => 'Diagnóstico y detalles';

  @override
  String get diagnosticsSubtitle => 'Inspección de formato (solo lectura)';

  @override
  String get metricFormat => 'Formato';

  @override
  String get metricSlot => 'Ranura';

  @override
  String get metricChapter => 'Capítulo';

  @override
  String get metricTimePlayed => 'Tiempo jugado';

  @override
  String get metricSaveKind => 'Tipo de guardado';

  @override
  String get metricFileSize => 'Tamaño del archivo';

  @override
  String get metricCompression => 'Compresión';

  @override
  String get metricChunks => 'Fragmentos';

  @override
  String get metricUncompressed => 'Sin comprimir';

  @override
  String get metricPrivate => 'Privado';

  @override
  String get metricSlotName => 'Nombre de la ranura';

  @override
  String get metricTrailer => 'Trailer';

  @override
  String get metricDecodedPrivate => 'Privado decodificado';

  @override
  String get metricPrivateStrings => 'Cadenas privadas';

  @override
  String get metricSha1 => 'SHA-1';

  @override
  String bytesValue(String count) {
    return '$count bytes';
  }

  @override
  String get inspectionJsonTitle => 'JSON de inspección';

  @override
  String get inspectionJsonSubtitle =>
      'Datos de inspección sin procesar de la partida';

  @override
  String get copy => 'Copiar';

  @override
  String get savegameFallbackTitle => 'Partida guardada';

  @override
  String screenshotForSlot(String slot) {
    return 'Captura de $slot';
  }

  @override
  String get publicSaveName => 'Nombre público de la partida';

  @override
  String get gameTimeTitle => 'Game time';

  @override
  String get gameTimeDay => 'Day';

  @override
  String get gameTimeHours => 'Hours';

  @override
  String get gameTimeMinutes => 'Minutes';

  @override
  String get gameTimeSeconds => 'Seconds';

  @override
  String gameTimeTotal(int seconds) {
    return '= $seconds s total';
  }

  @override
  String get gameTimeInvalid =>
      'Enter whole numbers — day ≥ 0, hours 0–23, minutes and seconds 0–59.';

  @override
  String get required => 'Obligatorio';

  @override
  String get playerLockedBody =>
      'La edición de datos privados del personaje requiere un códec capaz de comprimir.';

  @override
  String get heroTransform => 'Posición del héroe';

  @override
  String get locationX => 'Posición X';

  @override
  String get locationY => 'Posición Y';

  @override
  String get locationZ => 'Posición Z';

  @override
  String get rotationPitch => 'Cabeceo';

  @override
  String get rotationYaw => 'Guiñada';

  @override
  String get rotationRoll => 'Alabeo';

  @override
  String get invalid => 'No válido';

  @override
  String get heroAttributes => 'Atributos del héroe';

  @override
  String attributeBase(String name) {
    return '$name base';
  }

  @override
  String attributeCurrent(String name) {
    return '$name actual';
  }

  @override
  String get inventoryTitle => 'Inventario';

  @override
  String get inventoryEmpty => 'Este inventario está vacío.';

  @override
  String get inventoryNeedsDecoded =>
      'Para editar el inventario se necesitan los datos privados decodificados por el códec.';

  @override
  String get inventoryNoStacks =>
      'No se encontraron pilas de objetos en los datos privados decodificados.';

  @override
  String get resetInventoryChanges => 'Restablecer cambios del inventario';

  @override
  String get addItemTooltipPendingAdd =>
      'Guarda primero los cambios pendientes: un objeto nuevo por guardado';

  @override
  String get addItemTooltipPendingRemove =>
      'Guarda primero la eliminación pendiente: un cambio estructural por guardado';

  @override
  String get addItemTooltipPendingCount =>
      'Guarda o restablece primero los cambios de cantidad pendientes: una edición estructural debe guardarse por separado';

  @override
  String get addItemTooltipDefault => 'Añadir objeto al inventario';

  @override
  String get addItemButton => 'Añadir objeto';

  @override
  String pendingAddSubtitle(int count) {
    return '×$count — adición pendiente (aún sin guardar)';
  }

  @override
  String get cancelPendingAdd => 'Cancelar adición pendiente';

  @override
  String get pendingRemovalSubtitle =>
      'eliminación pendiente (aún sin guardar)';

  @override
  String get cancelPendingRemoval => 'Cancelar eliminación pendiente';

  @override
  String get filterItems => 'Filtrar objetos';

  @override
  String noItemsMatchQuery(String query) {
    return 'Ningún objeto coincide con «$query».';
  }

  @override
  String get pendingRemovalHidesAll =>
      'La eliminación pendiente oculta todos los objetos: guarda para aplicarla.';

  @override
  String categoryWithCount(String label, int count) {
    return '$label ($count)';
  }

  @override
  String get itemCategoryMeleeWeapon => 'Armas cuerpo a cuerpo';

  @override
  String get itemCategoryRangedWeapon => 'Armas a distancia';

  @override
  String get itemCategoryAmmunition => 'Munición';

  @override
  String get itemCategoryArmor => 'Armaduras';

  @override
  String get itemCategoryRune => 'Runas';

  @override
  String get itemCategoryScroll => 'Pergaminos de hechizo';

  @override
  String get itemCategoryFood => 'Comida y pociones';

  @override
  String get itemCategoryMisc => 'Misceláneos';

  @override
  String get itemCategoryAmulet => 'Amuletos';

  @override
  String get itemCategoryRing => 'Anillos';

  @override
  String get itemCategoryTrophy => 'Trofeos de animales';

  @override
  String get itemCategoryWriting => 'Escritos';

  @override
  String get itemCategoryMission => 'Objetos de misión';

  @override
  String get itemCategoryKey => 'Llaves';

  @override
  String get itemCategoryOther => 'Otros';

  @override
  String get count => 'Cantidad';

  @override
  String get min1 => 'Mín. 1';

  @override
  String countTimes(String count) {
    return '×$count';
  }

  @override
  String get deleteEquippedTooltip =>
      'No se puede eliminar: es probable que este objeto esté equipado o asignado a un acceso rápido';

  @override
  String get removeBlockedTooltip =>
      'Guarda o restablece primero los cambios pendientes del inventario: una adición o eliminación debe guardarse por separado';

  @override
  String get removeItemFromInventory => 'Quitar objeto del inventario';

  @override
  String get progressionLockedBody =>
      'Los datos de progreso necesitan los datos privados decodificados por el códec.';

  @override
  String get progressionNeedsTyped =>
      'Los datos de progreso estructurados requieren una partida totalmente decodificada con un análisis tipado verificado.';

  @override
  String get sectionQuests => 'Misiones';

  @override
  String get sectionKnowledge => 'Conocimientos';

  @override
  String get sectionEvents => 'Eventos';

  @override
  String get firstPage => 'Primera página';

  @override
  String get previousPage => 'Página anterior';

  @override
  String get nextPage => 'Página siguiente';

  @override
  String get lastPage => 'Última página';

  @override
  String pageOfPages(int page, int total) {
    return 'Página $page / $total';
  }

  @override
  String rangeOfTotal(int first, int last, int total) {
    return '$first–$last de $total';
  }

  @override
  String get perPage => 'Por página:';

  @override
  String get resetQuestChanges => 'Restablecer cambios de misiones';

  @override
  String get searchQuests => 'Buscar misiones';

  @override
  String get allGroups => 'Todos los grupos';

  @override
  String groupWithCount(String group, Object count) {
    return '$group ($count)';
  }

  @override
  String stateLabelWithCount(String label, int count) {
    return '$label $count';
  }

  @override
  String get questStateNone => 'Ninguno';

  @override
  String get questStateAvailable => 'Disponible';

  @override
  String get questStateRunning => 'En curso';

  @override
  String get questStateSucceeded => 'Completada';

  @override
  String get questStateFailed => 'Fallida';

  @override
  String get questStateUnknown => 'desconocido';

  @override
  String get dialogKnowledge => 'Conocimiento de diálogos';

  @override
  String get resetKnowledgeChanges => 'Restablecer cambios de conocimientos';

  @override
  String get addNpc => 'Añadir NPC';

  @override
  String get searchNpcs => 'Buscar NPC';

  @override
  String get npcStatusRowLabel => 'Estado';

  @override
  String get npcStatusAlive => 'vivo';

  @override
  String get npcStatusDead => 'muerto';

  @override
  String npcStateHp(String hp, String maxHp) {
    return 'PV $hp / $maxHp';
  }

  @override
  String get npcReviveButton => 'Revivir';

  @override
  String get npcReviveQueued => 'Se revivirá al guardar';

  @override
  String entriesForCharacter(String name) {
    return 'Entradas — $name';
  }

  @override
  String get selectNpcToSeeEntries => 'Selecciona un NPC para ver sus entradas';

  @override
  String get addKnowledgeEntry => 'Añadir entrada de conocimiento';

  @override
  String get browseCatalog => 'Examinar catálogo';

  @override
  String get alreadyExistsForCharacter => 'Ya existe para este personaje.';

  @override
  String get alreadyInPendingChanges => 'Ya está en los cambios pendientes.';

  @override
  String duplicateCheckFailed(String error) {
    return 'Falló la comprobación de duplicados; inténtalo de nuevo: $error';
  }

  @override
  String pendingAddsCount(int count) {
    return 'Adiciones pendientes ($count)';
  }

  @override
  String get undoAdd => 'Deshacer adición';

  @override
  String get undoRemove => 'Deshacer eliminación';

  @override
  String get removeEntry => 'Quitar entrada';

  @override
  String get selectNpcFromList => 'Selecciona un NPC de la lista';

  @override
  String characterWithCount(String name, int count) {
    return '$name ($count)';
  }

  @override
  String get memoryEvents => 'Eventos de memoria';

  @override
  String get searchCharacters => 'Buscar personajes';

  @override
  String eventsForCharacter(String name) {
    return 'Eventos — $name';
  }

  @override
  String get selectCharacterToSeeEvents =>
      'Selecciona un personaje para ver sus eventos';

  @override
  String get noTags => '(sin etiquetas)';

  @override
  String eventSubtitle(String time, String affected) {
    return 't=${time}s  $affected';
  }

  @override
  String get removeEvent => 'Quitar evento';

  @override
  String get removeMemoryEventTitle => '¿Quitar el evento de memoria?';

  @override
  String get removeMemoryEventBody =>
      '¿Quieres quitar este evento de memoria? Primero se crea una copia de seguridad.';

  @override
  String get duplicateEvent => 'Duplicar evento';

  @override
  String get duplicateMemoryEventTitle => '¿Duplicar el evento de memoria?';

  @override
  String get duplicateMemoryEventBody =>
      '¿Quieres duplicar este evento de memoria? Primero se crea una copia de seguridad.';

  @override
  String get selectCharacterFromList => 'Selecciona un personaje de la lista';

  @override
  String get factionsSidebar => 'Facciones';

  @override
  String get factionsForgiveButton => 'Perdonar';

  @override
  String get factionHostile => 'Hostil';

  @override
  String get factionFriendly => 'Amistoso';

  @override
  String crimeMurder(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count asesinatos',
      one: '$count asesinato',
    );
    return '$_temp0';
  }

  @override
  String crimeAssault(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count agresiones',
      one: '$count agresión',
    );
    return '$_temp0';
  }

  @override
  String crimeTheft(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count robos',
      one: '$count robo',
    );
    return '$_temp0';
  }

  @override
  String crimeTrespassing(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count allanamientos',
      one: '$count allanamiento',
    );
    return '$_temp0';
  }

  @override
  String crimeThreat(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count amenazas',
      one: '$count amenaza',
    );
    return '$_temp0';
  }

  @override
  String crimeOther(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count otros delitos',
      one: '$count otro delito',
    );
    return '$_temp0';
  }

  @override
  String get factionsForgiveQueued => 'perdonando…';

  @override
  String get factionsEmpty => 'No hay delitos pendientes contra facciones.';

  @override
  String get factionGuildOldCamp => 'Campamento Viejo';

  @override
  String get factionGuildNewCamp => 'Campamento Nuevo';

  @override
  String get factionGuildSwampCamp => 'Campamento del Pantano';

  @override
  String get factionGuildOther => 'Otros/individuos';

  @override
  String get allDataLockedBody =>
      'El explorador completo de propiedades necesita los datos privados decodificados por el códec.';

  @override
  String get allDataDescription =>
      'Busca cualquier propiedad tipada por nombre o ruta. Los valores numéricos, las cadenas, las enumeraciones y las rutas de objeto son editables; las estructuras se muestran de solo lectura por ahora.';

  @override
  String get searchPropertiesLabel =>
      'Buscar propiedades (vacío = mostrar todo); p. ej. Health, GameTime';

  @override
  String get decodingSaveTitle => 'Decodificando la partida…';

  @override
  String get decodingSaveBody =>
      'Se están decodificando todos los datos privados para la primera búsqueda. Esto se hace una vez por partida y luego las búsquedas son instantáneas.';

  @override
  String get searchTheSaveTitle => 'Buscar en la partida';

  @override
  String get searchTheSaveBody =>
      'Escribe el nombre de una propiedad y pulsa Intro. Déjalo vacío para mostrarlo todo.';

  @override
  String get searchFailedTitle => 'La búsqueda falló';

  @override
  String get noMatchesTitle => 'Sin coincidencias';

  @override
  String get noMatchesBody =>
      'Ninguna ruta de propiedad contenía todos esos términos.';

  @override
  String get value => 'Valor';

  @override
  String get backupsTitle => 'Copias de seguridad';

  @override
  String get refreshBackups => 'Actualizar copias de seguridad';

  @override
  String get noBackupsTitle => 'Sin copias de seguridad';

  @override
  String get noBackupsBody =>
      'Las partidas editadas crean archivos de copia de seguridad junto a la ranura seleccionada.';

  @override
  String get slotBackups => 'Copias de la ranura';

  @override
  String get profileBackups => 'Copias del perfil';

  @override
  String get backupFactName => 'Nombre';

  @override
  String get backupFactSlot => 'Ranura';

  @override
  String get backupFactCreated => 'Creada';

  @override
  String get backupFactSize => 'Tamaño';

  @override
  String get backupFactStatus => 'Estado';

  @override
  String get backupFactSha1 => 'SHA-1';

  @override
  String restoreBackupTooltip(String fileName) {
    return 'Restaurar $fileName';
  }

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
  String get language => 'Idioma';

  @override
  String get updatesTitle => 'Actualizaciones';

  @override
  String get checkForUpdatesAutomatically =>
      'Buscar actualizaciones automáticamente';

  @override
  String get checkForUpdatesNow => 'Buscar actualizaciones ahora';

  @override
  String get updatesPortableNotice =>
      'La versión portátil abre la página de descarga en tu navegador. Reemplaza tus archivos actuales con la nueva descarga.';

  @override
  String get updateAvailableTitle => 'Actualización disponible';

  @override
  String updateAvailableMessage(Object version, Object current) {
    return 'La versión $version está disponible. Tienes la $current.';
  }

  @override
  String get updateDownload => 'Descargar';

  @override
  String get updateLater => 'Más tarde';

  @override
  String get updateUpToDate => 'Estás usando la última versión.';

  @override
  String get updateCheckFailed =>
      'No se pudo buscar actualizaciones. Inténtalo de nuevo más tarde.';

  @override
  String get gameTextTitle => 'Texto del juego';

  @override
  String gameTextExtractedWithCounts(int ids, int languages) {
    return 'Extraído: $ids identificadores en $languages idiomas.';
  }

  @override
  String get gameTextExtracted =>
      'El texto localizado del juego está extraído.';

  @override
  String get gameTextNotExtracted =>
      'El texto localizado del juego aún no se ha extraído.';

  @override
  String get extracting => 'Extrayendo…';

  @override
  String get extractRefreshLocalizedText =>
      'Extraer / actualizar texto localizado';

  @override
  String get extractLocalizedTextTitle =>
      '¿Extraer el texto localizado del juego?';

  @override
  String get extractLocalizedTextBody =>
      'El texto localizado del juego aún no se ha extraído. ¿Extraerlo ahora desde tu instalación del juego? (opcional)';

  @override
  String get notNow => 'Ahora no';

  @override
  String get extract => 'Extraer';

  @override
  String get extractionComplete => 'Extracción completada';

  @override
  String get extractionFailed => 'La extracción falló';

  @override
  String get localizationCacheFileType => 'Caché de localización';

  @override
  String get savegameDirectoryTitle => 'Carpeta de partidas guardadas';

  @override
  String get folder => 'Carpeta';

  @override
  String get codecTitle => 'Códec';

  @override
  String get check => 'Comprobar';

  @override
  String get roundtrip => 'Ida y vuelta';

  @override
  String get noCodecStatus => 'Sin estado del códec';

  @override
  String get codecReady => 'Códec listo';

  @override
  String get codecReadOnly => 'Códec de solo lectura';

  @override
  String get codecUnavailable => 'Códec no disponible';

  @override
  String get details => 'Detalles';

  @override
  String codecStatusLine(String status) {
    return 'Estado: $status';
  }

  @override
  String codecCapabilityLine(String decompress, String compress) {
    return 'Descompresión: $decompress | Compresión: $compress';
  }

  @override
  String codecBackendLine(String backend) {
    return 'Backend: $backend';
  }

  @override
  String get yes => 'sí';

  @override
  String get no => 'no';

  @override
  String get aboutSubtitle => 'Editor de partidas guardadas de Gothic Remake';

  @override
  String aboutVersion(String version, String sha) {
    return 'Versión $version ($sha)';
  }

  @override
  String get aboutCopyright => '© 2026 colaboradores de goresave';

  @override
  String get aboutLicense => 'Distribuido bajo la licencia MIT.';

  @override
  String difficultyTitle(String profile) {
    return 'Dificultad — $profile';
  }

  @override
  String get difficultyNoProfile => 'Sin perfil';

  @override
  String get difficultyNoDifficulty => 'Sin dificultad';

  @override
  String get difficultyLabel => 'Dificultad';

  @override
  String get difficultyTooltipNoProfile => 'Ningún perfil seleccionado';

  @override
  String get difficultyTooltipEdit => 'Editar la dificultad de este perfil';

  @override
  String get difficultyTooltipNoEditable =>
      'Este perfil no tiene una dificultad editable';

  @override
  String get preset => 'Ajuste predefinido';

  @override
  String get presetNovice => 'Principiante';

  @override
  String get presetGothic => 'Gothic';

  @override
  String get presetHard => 'Difícil';

  @override
  String get presetCustom => 'Personalizado';

  @override
  String unrecognisedPreset(Object preset) {
    return 'El ajuste predefinido guardado no se reconoce ($preset). Aún puedes guardar los cambios de Asistente de combate / Muerte permanente, o elegir un ajuste arriba para sobrescribirlo.';
  }

  @override
  String get closeCombatFlowHelper => 'Asistente de combate cuerpo a cuerpo';

  @override
  String get permadeath => 'Muerte permanente';

  @override
  String get notAvailableOnNovice => 'No disponible en Principiante';

  @override
  String get levelCombat => 'Combate';

  @override
  String get levelResources => 'Recursos';

  @override
  String get levelProgression => 'Progreso';

  @override
  String get difficultyAppliesToAllSaves =>
      'La dificultad se aplica a todas las partidas de este perfil.';

  @override
  String get savingDifficultyFailed => 'No se pudo guardar la dificultad.';

  @override
  String get addItemDialogTitle => 'Añadir objeto';

  @override
  String get searchItems => 'Buscar objetos';

  @override
  String failedToLoadCatalog(String error) {
    return 'No se pudo cargar el catálogo: $error';
  }

  @override
  String get noItemsAvailableToAdd => 'No hay objetos disponibles para añadir';

  @override
  String get noItemsMatch => 'Ningún objeto coincide';

  @override
  String get countMustBeAtLeast1 => 'Debe ser ≥ 1';

  @override
  String countMustBeAtMost(int max) {
    return 'Debe ser ≤ $max';
  }

  @override
  String get addNpcDialogTitle => 'Añadir NPC';

  @override
  String get noNpcsAvailableToAdd => 'No hay NPC disponibles para añadir';

  @override
  String get noNpcsMatch => 'Ningún NPC coincide';

  @override
  String get categoryAll => 'Todos';

  @override
  String allWithCount(int count) {
    return 'Todos ($count)';
  }

  @override
  String get addKnowledgeEntryDialogTitle => 'Añadir entrada de conocimiento';

  @override
  String get searchEntries => 'Buscar entradas';

  @override
  String get noKnowledgeEntriesAvailableToAdd =>
      'No hay entradas de conocimiento disponibles para añadir';

  @override
  String get noEntriesMatch => 'Ninguna entrada coincide';

  @override
  String get heroGroupMainStats => 'Estadísticas principales';

  @override
  String get heroGroupCombatSkills => 'Habilidades de combate';

  @override
  String get heroGroupResistances => 'Resistencias';

  @override
  String get heroGroupThieving => 'Robo';

  @override
  String get heroGroupAdvanced => 'Avanzado';

  @override
  String get heroEntryHeroTransform => 'Posición del héroe';

  @override
  String attributeEmpty(String name) {
    return '$name está vacío: introduce un valor o restaura el original antes de guardar.';
  }

  @override
  String attributeInvalidNumber(String name, String text) {
    return 'Número no válido para $name: «$text»';
  }

  @override
  String get loadingEditorData => 'Cargando los datos del editor';

  @override
  String localizedTextExtractedCount(int idCount, int languageCount) {
    return '$idCount ID extraídos en $languageCount idiomas';
  }
}
