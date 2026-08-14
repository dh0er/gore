// ignore: unused_import
import 'package:intl/intl.dart' as intl;
import 'app_localizations.dart';

// ignore_for_file: type=lint

/// The translations for Spanish Castilian (`es`).
class AppLocalizationsEs extends AppLocalizations {
  AppLocalizationsEs([String locale = 'es']) : super(locale);

  @override
  String get debugSectionTitle => 'Avanzado (depuración)';

  @override
  String get debugSectionSubtitle =>
      'Diagnóstico y datos sin procesar para informes de errores';

  @override
  String get showObjectIdsTitle => 'Mostrar ID técnicos adicionales';

  @override
  String get showObjectIdsSubtitle =>
      'Muestra los ID técnicos de objetos, conocimientos de diálogo, misiones y actores huérfanos. Los ID de NPC se muestran siempre.';

  @override
  String get storyStateSidebar => 'Estado de la historia';

  @override
  String get storyStateDescription =>
      'Catálogo autoritativo de estados persistentes declarados por los scripts distribuidos con el juego. Las entradas guardadas muestran su valor bruto; los campos del catálogo ausentes de esta partida se marcan como no establecidos. Las marcas de tiempo declaradas en el código se muestran como tiempo de juego; los demás enteros pueden ser booleanos, contadores o estados de varios niveles.';

  @override
  String get storyStateReadOnly =>
      'Solo lectura hasta conocer el significado de los valores en los scripts y disponer de escrituras seguras del mapa. El texto de glosario relacionado aporta contexto; no es una traducción directa del ID técnico.';

  @override
  String get storyStateStructureReadOnly =>
      'No se pudo identificar de forma inequívoca y segura la estructura StoryPropertyValues de esta partida guardada. Los valores de historia seguirán siendo de solo lectura para esta partida.';

  @override
  String get storyStateSearch => 'Buscar estados de historia';

  @override
  String storyStateValuesCount(int shown, int total) {
    return '$shown de $total valores de historia';
  }

  @override
  String get storyStateInteger => 'Entero';

  @override
  String get storyStateTimeMarker => 'Marca de tiempo';

  @override
  String get storyStateChapter => 'Capítulo';

  @override
  String get storyStateUnknown => 'Tipo de origen desconocido';

  @override
  String get storyStateUnknownDetail =>
      'Este ID guardado no figura en el catálogo de scripts actual (por ejemplo, por un mod o una versión más reciente del juego). El valor almacenado es int32, pero no se infiere su significado.';

  @override
  String get storyStateStored => 'Guardado';

  @override
  String get storyStateUnset => 'No establecido';

  @override
  String get storyStateUnsetDetail =>
      'Este campo del catálogo no está serializado en esta partida; por tanto, el juego usa su estado no establecido o predeterminado.';

  @override
  String get storyStateRawValue => 'Valor bruto';

  @override
  String storyStateElapsed(String duration) {
    return 'Tiempo transcurrido al guardar: $duration';
  }

  @override
  String storyStateAhead(String duration) {
    return 'Tiempo futuro al guardar: $duration';
  }

  @override
  String storyStateDurationDays(int days, String time) {
    String _temp0 = intl.Intl.pluralLogic(
      days,
      locale: localeName,
      other: '$days días',
      one: '1 día',
    );
    return '$_temp0 $time';
  }

  @override
  String get storyStateRelatedGlossary => 'Entrada de glosario relacionada';

  @override
  String get storyStateTechnicalPath => 'Ruta técnica';

  @override
  String get storyStateEditingGuidance =>
      'Todas las entradas se pueden editar en todo el intervalo de int32 con signo. Los indicadores y las sugerencias de valores respaldados por los scripts son solo orientativos; la entrada sin procesar siempre está disponible. Los cambios en el estado de la historia pueden omitir transiciones de diálogos, misiones o del mundo, así que guárdalos con cuidado; se crea una copia de seguridad automáticamente.';

  @override
  String get storyStatePending => 'Pendiente';

  @override
  String storyStatePendingValue(String value) {
    return 'Se guardará como $value';
  }

  @override
  String get storyStatePendingRemoval => 'Se eliminará de la partida guardada';

  @override
  String get storyStateEditValue => 'Editar valor';

  @override
  String get storyStateSetValue => 'Establecer valor';

  @override
  String get storyStateRemoveValue => 'Eliminar de la partida guardada';

  @override
  String get storyStateUndoChange => 'Deshacer cambio de historia';

  @override
  String get storyStateResetChanges => 'Restablecer cambios de historia';

  @override
  String storyStateDialogTitle(String id) {
    return 'Editar $id';
  }

  @override
  String get storyStateRawInput => 'Valor int32 con signo';

  @override
  String get storyStateInvalidInt32 =>
      'Introduce un número entero entre -2147483648 y 2147483647.';

  @override
  String get storyStateQueueChange => 'Añadir cambio a la cola';

  @override
  String storyStateSuggestedValues(String values) {
    return 'Valores constatados en los scripts distribuidos: $values';
  }

  @override
  String get storyStateSuggestionsNotLimits =>
      'Las sugerencias no son límites de validación; el código nativo, los mods o las versiones posteriores del juego pueden usar otros valores.';

  @override
  String get storyStateUseCurrentTime => 'Usar la hora actual de la partida';

  @override
  String get storyStateStructuredTime => 'Día / hora';

  @override
  String get storyStateRawMode => 'int32 sin procesar';

  @override
  String get storyStateChapterWarning =>
      'Cambiar solo el capítulo no sincroniza las misiones, los PNJ, el inventario ni el estado del mundo.';

  @override
  String get storyStateDormantWarning =>
      'No se encontró ninguna lectura ni escritura activa de este campo en la caché de scripts distribuida. Puede ser heredado, estar controlado por código nativo o estar reservado.';

  @override
  String get storyStateReadOnlySourceWarning =>
      'Los scripts distribuidos leen este campo, pero no contienen ninguna escritura mediante scripts. Es posible que el código nativo siga controlándolo.';

  @override
  String get storyStateUnknownEditWarning =>
      'Este ID de un mod o de una versión posterior no tiene semántica de código fuente incluida. Edita únicamente su valor int32 sin procesar.';

  @override
  String storyStateIntegerKind(String kind) {
    String _temp0 = intl.Intl.selectLogic(kind, {
      'binaryFlag': 'Indicador binario',
      'finiteState': 'Valor multiestado',
      'counterOrScore': 'Contador / puntuación',
      'calendarDay': 'Día natural',
      'derivedOrOpaqueInteger': 'Entero derivado / opaco',
      'readOnlyInSourceInteger': 'Solo lectura en los scripts distribuidos',
      'dormantOrLegacyInteger': 'Sin uso en los scripts distribuidos',
      'other': 'Entero',
    });
    return '$_temp0';
  }

  @override
  String get storyStateZeroVsUnset =>
      'Un 0 guardado y una entrada ausente del mapa son estados de archivo distintos. «Eliminar de la partida guardada» restaura el estado del constructor o predeterminado.';

  @override
  String get appTitle => 'GORE Save Editor';

  @override
  String get appLogoSemanticLabel => 'Logotipo de GORE Save Editor';

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
  String get heroGroupSkills => 'Habilidades';

  @override
  String get skillsNoneBody =>
      'No se encontraron habilidades para este personaje.';

  @override
  String get skillsUnavailableBody =>
      'Las habilidades no se pueden editar en esta partida: el héroe no tiene datos de efectos que modificar.';

  @override
  String get skillNotLearned => 'No aprendida';

  @override
  String get skillLearn => 'Aprender';

  @override
  String get skillActionLearn => 'aprender';

  @override
  String get skillActionUnlearn => 'olvidar';

  @override
  String get skillTierUntrained => 'No entrenado';

  @override
  String get skillTierBeginner => 'Principiante';

  @override
  String get skillTierTrained => 'Entrenado';

  @override
  String get skillTierMaster => 'Maestro';

  @override
  String get skillTierNovice => 'Novato';

  @override
  String get skillTierAmateur => 'Aprendiz (Círculo 0)';

  @override
  String get skillTierLearned => 'Aprendida';

  @override
  String skillTierCircle(int n) {
    return 'Círculo $n';
  }

  @override
  String get skillHintBlacksmith1H => 'armas a una mano';

  @override
  String get skillHintBlacksmith2H => 'armas a dos manos';

  @override
  String get skillCategoryCombat => 'Combate';

  @override
  String get skillCategoryCrafting => 'Artesanía';

  @override
  String get skillCategoryHunting => 'Caza';

  @override
  String get skillCategoryLanguage => 'Idioma';

  @override
  String get skillCategoryMagic => 'Magia';

  @override
  String get skillCategoryMovement => 'Movimiento';

  @override
  String get skillCategoryThievery => 'Hurto';

  @override
  String get skillNameOneHanded => 'Una mano';

  @override
  String get skillNameTwoHanded => 'Dos manos';

  @override
  String get skillNameFists => 'Puños';

  @override
  String get skillNameBow => 'Arco';

  @override
  String get skillNameCrossbow => 'Ballesta';

  @override
  String get skillNameLockpicking => 'Abrir cerraduras';

  @override
  String get skillNamePickpocketing => 'Hurto';

  @override
  String get skillNameTakeOrgans => 'Extraer órgano';

  @override
  String get skillNameBreakTeeth => 'Extraer dientes';

  @override
  String get skillNameTakeClaws => 'Extraer garra';

  @override
  String get skillNameSkinFur => 'Coger pelaje';

  @override
  String get skillNameSkin => 'Coger piel';

  @override
  String get skillNameTakeFins => 'Coger aletas';

  @override
  String get skillNameTakeStingers => 'Extraer aguijones';

  @override
  String get skillNameTakeSecretion => 'Extraer secreción';

  @override
  String get skillNameTakeSkullPlates => 'Coger armadura craneal';

  @override
  String get skillNameSkinSwampshark => 'Coger piel de tiburón';

  @override
  String get skillNameTakeMinecrawlerPlates => 'Coger placas';

  @override
  String get skillNameTakeScutes => 'Coger placas dérmicas';

  @override
  String get skillNameTakeUluMulu => 'Coger Ulu-Mulu';

  @override
  String get skillNameOrcWeapons => 'Armas orcas';

  @override
  String get skillNameMining => 'Minería';

  @override
  String get skillNameDiving => 'Buceo';

  @override
  String get skillNameTakeMinecrawlerMandibles => 'Extraer mandíbulas';

  @override
  String get skillNameTakeShadowbeastHorn => 'Coger cuerno (Shadowbeast)';

  @override
  String get skillNameTakeSpines => 'Extraer espina';

  @override
  String get skillNameBreakSwampsharkTeeth => 'Extraer dientes de tiburón';

  @override
  String get skillNameTakeFireTongue => 'Coger lengua de fuego';

  @override
  String get skillNameTakeTrollHorn => 'Coger cuerno (Troll)';

  @override
  String get skillNameAcrobatics => 'Acrobacias';

  @override
  String get skillNameWallClimbing => 'Trepar';

  @override
  String get skillNameRiding => 'Montar carroñeros';

  @override
  String get skillNameSneaking => 'Sigilo';

  @override
  String get skillNameAlchemy => 'Alquimia';

  @override
  String get skillNameRuneInscription => 'Inscripción';

  @override
  String get skillNameBlacksmithing => 'Herrería';

  @override
  String get skillNameMagicCircle => 'Círculo mágico';

  @override
  String get skillNameOrcish => 'Idioma orco';

  @override
  String get tabInventory => 'Inventario';

  @override
  String get tabWorld => 'Mundo';

  @override
  String get tabCharacters => 'Personajes';

  @override
  String get characterNoActorBody =>
      'Este personaje no tiene un actor en el mundo, por lo que no tiene atributos, inventario ni eventos.';

  @override
  String get characterNoEventsBody => 'No hay eventos para este personaje.';

  @override
  String get characterOrphanGroup => 'Otros';

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
  String get otherSaves => 'Otras partidas';

  @override
  String profileWithSaves(String name, int count) {
    return '$name ($count partidas)';
  }

  @override
  String get switchProfile => 'Cambiar de perfil';

  @override
  String get openSaveFile => 'Abrir archivo';

  @override
  String get externalSave => 'Partida abierta externamente';

  @override
  String get saveProfileTitle => 'Perfil de la partida';

  @override
  String get saveProfileDescription =>
      'Asigna esta partida a otro perfil del juego. Se crearán copias de seguridad conjuntas de la partida y del índice de perfiles.';

  @override
  String get saveProfileExternalHint =>
      'Selecciona un perfil para importar este archivo a la carpeta de partidas del juego y registrarlo allí. El archivo original no cambiará.';

  @override
  String get saveProfileNoProfiles =>
      'No se encontraron perfiles editables en PersistentDataList.sav.';

  @override
  String get saveProfileSelect => 'Seleccionar perfil';

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
  String bytesValue(String count) {
    return '$count bytes';
  }

  @override
  String get inspectionJsonTitle => 'JSON de inspección';

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
  String get gameTimeTitle => 'Tiempo de juego';

  @override
  String get gameTimeDay => 'Día';

  @override
  String get gameTimeHours => 'Horas';

  @override
  String get gameTimeMinutes => 'Minutos';

  @override
  String get gameTimeSeconds => 'Segundos';

  @override
  String gameTimeTotal(int seconds) {
    return '= $seconds s en total';
  }

  @override
  String get gameTimeInvalid =>
      'Introduce números enteros: día ≥ 0, horas 0–23, minutos y segundos 0–59.';

  @override
  String get required => 'Obligatorio';

  @override
  String get playerLockedBody =>
      'La edición de datos privados del personaje requiere un códec capaz de comprimir.';

  @override
  String get heroTransform => 'Posición';

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
  String get spawnPositionSection => 'Posición de aparición (referencia)';

  @override
  String get resetToSpawnPosition => 'Restablecer a la posición de aparición';

  @override
  String get positionOutOfRange =>
      'El valor debe estar entre −10.000.000 y 10.000.000';

  @override
  String get positionNotEditable =>
      'No se pudo leer la posición guardada de este personaje, por lo que no se puede editar.';

  @override
  String get positionNeverPlaced =>
      'Este personaje nunca ha sido colocado en el mundo (posición 0, 0, 0): es posible que el juego ignore la posición guardada.';

  @override
  String get npcStayInPlace => 'Desactivar su rutina diaria';

  @override
  String get npcStayInPlaceHint => 'Entonces se queda donde está.';

  @override
  String get npcStayInPlaceLocked =>
      'Su rutina diaria original no está registrada, así que esto ya no se puede deshacer.';

  @override
  String get npcUndoPlacement => 'Deshacer el traslado';

  @override
  String get npcUndoPlacementStale =>
      'La partida ya no contiene lo que escribió ese traslado, así que restaurarlo descartaría lo ocurrido desde entonces.';

  @override
  String get positionNotReadable =>
      'No se pudo leer la posición guardada de este personaje.';

  @override
  String get npcPositionReadOnly =>
      'El juego restaura la posición de un PNJ desde el nivel, no desde la partida guardada, por lo que estos valores se pueden leer pero no modificar.';

  @override
  String get pickLocation => 'Elegir ubicación…';

  @override
  String get pickLocationDialogTitle => 'Elegir una ubicación';

  @override
  String get applySpotRotation => 'Aplicar también la orientación del punto';

  @override
  String get locationAreaOther => 'Otros';

  @override
  String get locationAreaCavalornValley => 'Valle de Cavalorn';

  @override
  String get locationAreaEastForest => 'Bosque del Este';

  @override
  String get locationAreaFogTower => 'Torre de la Niebla';

  @override
  String get locationAreaIllegalWeedMixers => 'Mezcladores ilegales de hierba';

  @override
  String get locationAreaOrcArena => 'Arena de los orcos';

  @override
  String get locationAreaOrcGraveyard => 'Cementerio orco';

  @override
  String get locationAreaShipwreck => 'Naufragio';

  @override
  String get locationAreaTundra => 'Tundra';

  @override
  String get locationCatalogUnavailable =>
      'No se pudo cargar el catálogo de ubicaciones.';

  @override
  String get invalid => 'No válido';

  @override
  String get heroAttributes => 'Atributos del héroe';

  @override
  String attributeBase(String name) {
    return 'Valor base de $name';
  }

  @override
  String attributeCurrent(String name) {
    return '$name actual';
  }

  @override
  String get attributeBaseValue => 'Valor base';

  @override
  String get attributeCurrentValue => 'Valor actual';

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
  String get resetInventoryButton => 'Restablecer inventario';

  @override
  String get resetInventoryTooltipDefault =>
      'Sustituir este inventario por el de la partida inicial';

  @override
  String get resetInventoryTooltipBlocked =>
      'Guarda o cancela primero los cambios de inventario pendientes';

  @override
  String get pendingResetTitle => 'Restablecer al inventario inicial';

  @override
  String pendingResetSubtitle(String level) {
    return 'Nivel de recursos: $level';
  }

  @override
  String get cancelPendingReset => 'Cancelar restablecimiento';

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
  String get npcRelationshipRowLabel => 'Relación';

  @override
  String get npcRelationshipUnavailable => 'Estado de relación no disponible';

  @override
  String get npcRelationshipAutomatic => 'Calculada por el juego';

  @override
  String get npcRelationshipAutomaticHint =>
      'No hay ninguna anulación permanente guardada. El juego evalúa las reglas de gremio, historia, zona y delitos.';

  @override
  String get npcRelationshipStoredHint =>
      'Guardada como anulación permanente entre el NPC y el jugador. Las reglas de gremio, historia, zona y delitos aún pueden cambiar la relación efectiva en el juego.';

  @override
  String get npcRelationshipFriend => 'Amigo';

  @override
  String get npcRelationshipNeutral => 'Neutral';

  @override
  String get npcRelationshipEnemy => 'Enemigo';

  @override
  String npcRelationshipPending(String relationship) {
    return 'Será $relationship al guardar';
  }

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
  String get memoryEventRemovalQueued =>
      'Eliminación del evento en cola: pulsa Guardar para aplicarla.';

  @override
  String get duplicateEvent => 'Duplicar evento';

  @override
  String get duplicateMemoryEventTitle => '¿Duplicar el evento de memoria?';

  @override
  String get duplicateMemoryEventBody =>
      '¿Quieres duplicar este evento de memoria? Primero se crea una copia de seguridad.';

  @override
  String get memoryEventDuplicationQueued =>
      'Duplicación del evento en cola: pulsa Guardar para aplicarla.';

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
      'El explorador exhaustivo de fuentes está disponible actualmente para archivos de guardado GSAV.';

  @override
  String get allDataDescription =>
      'Explora los metadatos GSAV y todos los nodos tipados PUBLIC/PRIVATE. Los valores escalares y las estructuras nativas seguras son editables; los contenedores y los bytes opacos permanecen visibles.';

  @override
  String get allDataEditable => 'Editable';

  @override
  String get allDataReadOnly => 'Solo lectura';

  @override
  String get allDataType => 'Tipo';

  @override
  String get allDataScalars => 'Escalares';

  @override
  String get allDataStructs => 'Estructuras';

  @override
  String get allDataContainers => 'Contenedores';

  @override
  String get allDataOpaque => 'Opacos';

  @override
  String get allDataNodes => 'Nodos';

  @override
  String allDataChildren(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count elementos hijos',
      one: '1 elemento hijo',
    );
    return '$_temp0';
  }

  @override
  String get allDataPending => 'Pendiente';

  @override
  String get allDataTagInputHint =>
      'Etiquetas separadas por comas o saltos de línea';

  @override
  String allDataTypedSource(String source) {
    return 'Fuente tipada: $source';
  }

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
  String aboutVersion(String version, String sha) {
    return 'Versión $version ($sha)';
  }

  @override
  String get aboutCopyright => '© 2026 colaboradores de GORE';

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
  String get presetNovice => 'Fácil';

  @override
  String get presetGothic => 'Gothic';

  @override
  String get presetHard => 'Difícil';

  @override
  String get presetCustom => 'Personalizada';

  @override
  String unrecognisedPreset(Object preset) {
    return 'El ajuste predefinido guardado no se reconoce ($preset). Aún puedes guardar los cambios de Asistente de combate / Muerte permanente, o elegir un ajuste arriba para sobrescribirlo.';
  }

  @override
  String get closeCombatFlowHelper => 'Ayuda de fluidez de combate';

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
  String get heroGroupCombatMovement => 'Combate / movimiento';

  @override
  String get heroGroupResistances => 'Resistencias';

  @override
  String get heroGroupThieving => 'Robo';

  @override
  String get heroGroupAdvanced => 'Avanzado';

  @override
  String get heroGroupDiving => 'Buceo';

  @override
  String get heroGroupSleep => 'Sueño y descanso';

  @override
  String get heroGroupIntoxication => 'Embriaguez';

  @override
  String get heroEntryHeroTransform => 'Posición';

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
  String savingProgress(int done, int total) {
    return 'Guardando… $done de $total';
  }

  @override
  String localizedTextExtractedCount(int idCount, int languageCount) {
    return '$idCount ID extraídos en $languageCount idiomas';
  }

  @override
  String get skillSmithing1H => 'Herrería de una mano';

  @override
  String get skillSmithing2H => 'Herrería de dos manos';

  @override
  String get skillCircleNovice => 'Mago Novato';

  @override
  String get skillCircle1 => 'Primer Círculo de Magia';

  @override
  String get skillCircle2 => 'Segundo Círculo de Magia';

  @override
  String get skillCircle3 => 'Tercer Círculo de Magia';

  @override
  String get skillCircle4 => 'Cuarto Círculo de Magia';

  @override
  String get skillCircle5 => 'Quinto Círculo de Magia';

  @override
  String get skillCircle6 => 'Sexto Círculo de Magia';

  @override
  String get sectionGlossary => 'Glosario';

  @override
  String get glossarySearch => 'Buscar en el glosario';

  @override
  String get glossaryOldCamp => 'Campamento Viejo';

  @override
  String get glossaryNewCamp => 'Campamento Nuevo';

  @override
  String get glossarySwampCamp => 'Campamento del Pantano';

  @override
  String get glossaryOutsiders => 'Forasteros';

  @override
  String get glossaryCreatures => 'Criaturas';

  @override
  String get glossaryLocations => 'Ubicaciones';

  @override
  String get glossaryFilterLabel => 'Filtro';

  @override
  String get glossaryFilterTraders => 'Comerciantes';

  @override
  String get glossaryFilterTeachers => 'Instructores';

  @override
  String get glossaryFilterArmorers => 'Armeros';

  @override
  String get glossaryFilterHostile => 'Hostiles';

  @override
  String get glossaryRelationshipFilterNote =>
      'Muestra las anulaciones permanentes de enemigo guardadas en la partida. Las relaciones dinámicas de gremio, historia, zona y delitos solo se calculan en el juego.';

  @override
  String get glossaryFilterDead => 'Muertos';

  @override
  String get glossaryAddEntry => 'Añadir entrada al glosario';

  @override
  String get glossaryAddTitle => 'Añadir entrada al glosario';

  @override
  String get glossaryResetChanges => 'Restablecer cambios del glosario';

  @override
  String get glossaryNoVisibleEntries =>
      'Ninguna entrada visible del glosario coincide con esta vista.';

  @override
  String get glossaryNoHiddenEntries =>
      'Todas las entradas disponibles ya están visibles.';

  @override
  String get glossaryNoMatch => 'Ninguna entrada del glosario coincide.';

  @override
  String get glossarySelectEntry =>
      'Selecciona una entrada del glosario para editar sus apartados.';

  @override
  String glossaryEntryCount(int count) {
    return '$count entradas';
  }

  @override
  String glossarySegmentsCount(int unlocked, int total) {
    return '$unlocked de $total entradas';
  }

  @override
  String get glossaryPortraitUnlocked => 'Retrato desbloqueado';

  @override
  String get glossaryPortraitSilhouette => 'Silueta: retrato no desbloqueado';

  @override
  String get glossarySegments => 'Entradas';

  @override
  String get glossaryPending => 'Cambio sin guardar';

  @override
  String get glossaryShowFullText => 'Mostrar el texto completo de la entrada';

  @override
  String get glossarySegmentIntroduction => 'Introducción / retrato';

  @override
  String get glossarySegmentUnlock => 'Descubrimiento';

  @override
  String glossarySegmentEntry(int number) {
    return 'Entrada $number';
  }

  @override
  String get questJournalAll => 'Todas las misiones';

  @override
  String get questJournalOldCamp => 'Campamento Viejo';

  @override
  String get questJournalNewCamp => 'Campamento Nuevo';

  @override
  String get questJournalSwampCamp => 'Campamento del Pantano';

  @override
  String get questJournalColony => 'La Colonia';

  @override
  String get questJournalCompleted => 'Completadas';

  @override
  String get questJournalHint =>
      'Vista del diario del juego. Los estados internos y de misiones aún no iniciadas siguen disponibles en Todos los datos.';

  @override
  String get questJournalNoEntries =>
      'Ninguna misión del diario coincide con los filtros actuales.';

  @override
  String get glossaryTutorials => 'Tutoriales';

  @override
  String get tutorialGateNote =>
      'Estas filas controlan los desbloqueos de tutoriales guardados. Un desbloqueo no corresponde necesariamente a una sola página de tutorial del juego.';

  @override
  String get tutorialResetChanges => 'Restablecer cambios de tutoriales';

  @override
  String get tutorialNoGates =>
      'No hay desbloqueos de tutoriales disponibles en esta partida.';

  @override
  String tutorialGateUnlockCount(int unlocked, int total) {
    return '$unlocked de $total tutoriales desbloqueados';
  }

  @override
  String get tutorialGateCombatBasics => 'Fundamentos del combate';

  @override
  String get tutorialGateCrafting => 'Fabricación';

  @override
  String get tutorialGateCrime => 'Delitos y consecuencias';

  @override
  String get tutorialGateDrugs => 'Consumibles y efectos';

  @override
  String get tutorialGateLockpicking => 'Forzar cerraduras';

  @override
  String get tutorialGateMagic => 'Magia';

  @override
  String get tutorialGateMap => 'Mapa';

  @override
  String get tutorialGateMeleeCombat => 'Combate cuerpo a cuerpo';

  @override
  String get tutorialGateNavigation => 'Movimiento y navegación';

  @override
  String get tutorialGatePerception => 'Percepción';

  @override
  String get tutorialGatePlayerProgression => 'Progreso del personaje';

  @override
  String get tutorialGateRanged => 'Combate a distancia';

  @override
  String get tutorialGateRiding => 'Montar';

  @override
  String get tutorialGateSleep => 'Dormir';

  @override
  String get tutorialGateTrading => 'Comercio';

  @override
  String get windowMinimizeTooltip => 'Minimizar';

  @override
  String get windowMaximizeTooltip => 'Maximizar';

  @override
  String get windowRestoreTooltip => 'Restaurar';

  @override
  String get fallbackDialogEntry => 'Entrada de diálogo';

  @override
  String get fallbackDialogChoice => 'Opción de diálogo';

  @override
  String get fallbackDialogTopic => 'Tema de diálogo';

  @override
  String get fallbackDialogInformation => 'Información de diálogo';

  @override
  String get fallbackQuest => 'Misión';

  @override
  String get fallbackObjective => 'Objetivo';

  @override
  String get fallbackItem => 'Objeto';

  @override
  String get attributeSkillPointsFallback => 'Puntos de aprendizaje (PA)';

  @override
  String attributeManualFallbackLabel(String attributeId, String fallback) {
    String _temp0 = intl.Intl.selectLogic(attributeId, {
      'SuperArmor': 'Aplomo',
      'MaxSuperArmor': 'Aplomo máx.',
      'DamageMultiplier': 'Daño recibido',
      'SpeedModifier': 'Velocidad de movimiento',
      'Oxygen': 'Aire',
      'MaxOxygen': 'Aire máx.',
      'OxygenDepletionRate': 'Aire gastado por segundo',
      'OxygenRecoveryRate': 'Aire recuperado por seg.',
      'CriticalLevelPercent': 'Aviso de falta de aire',
      'SleepTime': 'Horas reparadoras rest.',
      'MaxSleepTime': 'Máx. horas reparadoras',
      'SleepTimeRecoveryAmount': 'Horas que se recuperan',
      'SleepTimeRecoveryPeriod': 'Intervalo de recarga',
      'MaxRestTime': 'Máx. tiempo en la cama',
      'Health_RecoveryRatePerHourOfSleep': 'Vida por hora de sueño',
      'Mana_RecoveryRatePerHourOfSleep': 'Maná por hora de sueño',
      'Alcohol': 'Nivel de alcohol',
      'MaxAlcohol': 'Nivel de alcohol máx.',
      'AlcoholDepletionRate': 'Velocidad para despejarse',
      'Swampweed': 'Nivel de hierba de pantano',
      'MaxSwampweed': 'Máx. hierba de pantano',
      'SwampweedDepletionRate': 'Velocidad del bajón',
      'XPExecutedBounty': 'EXP por rematar en el suelo',
      'XPKillOrDefeatBounty': 'EXP por derrotar',
      'other': '$fallback',
    });
    return '$_temp0';
  }

  @override
  String attributeManualTooltip(String attributeId) {
    String _temp0 = intl.Intl.selectLogic(attributeId, {
      'SuperArmor':
          'Cuánto castigo aguanta el héroe antes de que un golpe lo haga tambalearse.',
      'MaxSuperArmor':
          'La reserva completa de aplomo; aumenta con el nivel y con la armadura que lleva puesta.',
      'DamageMultiplier':
          'Factor que se aplica al daño que recibe el héroe: 1 es lo normal, y cuanto más alto, más duele.',
      'SpeedModifier':
          'Factor sobre lo rápido que se mueve el héroe: 1 es lo normal.',
      'Oxygen':
          'Segundos de aire que quedan bajo el agua; al llegar a cero el héroe se ahoga.',
      'MaxOxygen':
          'Cuántos segundos puede aguantar el héroe bajo el agua; la habilidad Buceo lo aumenta.',
      'OxygenDepletionRate': 'Aire que se consume cada segundo bajo el agua.',
      'OxygenRecoveryRate':
          'Aire que se recupera cada segundo al salir a la superficie.',
      'CriticalLevelPercent':
          'Porcentaje de aire restante con el que el juego avisa del peligro de ahogarse.',
      'SleepTime':
          'Horas de sueño que todavía aportan algo; a partir de ahí el juego no da ninguna recuperación.',
      'MaxSleepTime':
          'El mayor número de horas reparadoras que puede acumular el héroe.',
      'SleepTimeRecoveryAmount':
          'Horas reparadoras que se devuelven cada vez que se rellena la reserva.',
      'SleepTimeRecoveryPeriod':
          'Cuánto tarda la reserva de horas reparadoras en volver a llenarse.',
      'MaxRestTime':
          'El tiempo más largo que el juego permite pasar en la cama de una sola vez.',
      'Health_RecoveryRatePerHourOfSleep':
          'Porcentaje de la vida máxima que se recupera por cada hora dormida.',
      'Mana_RecoveryRatePerHourOfSleep':
          'Porcentaje del maná máximo que se recupera por cada hora dormida.',
      'Alcohol':
          'Lo borracho que está el héroe; los niveles altos cambian destreza y maná por fuerza.',
      'MaxAlcohol': 'El nivel de alcohol más alto que puede alcanzar el héroe.',
      'AlcoholDepletionRate':
          'Con qué rapidez baja el nivel de alcohol hacia la sobriedad.',
      'Swampweed':
          'Lo colocado que está el héroe; los niveles altos le mueven los atributos.',
      'MaxSwampweed':
          'El nivel de hierba de pantano más alto que puede alcanzar el héroe.',
      'SwampweedDepletionRate':
          'Con qué rapidez se pasa el efecto de la hierba de pantano.',
      'XPExecutedBounty':
          'Experiencia por matar a este personaje cuando ya yace derrotado en el suelo.',
      'XPKillOrDefeatBounty':
          'Experiencia por derribar a este personaje, tanto si muere como si solo queda inconsciente.',
      'other': '?',
    });
    return '$_temp0';
  }

  @override
  String get knowledgeTypeVoiceLine => 'Línea de voz';

  @override
  String get knowledgeTypeOther => 'Otro';

  @override
  String get armorUpgradeUpper => 'Superior';

  @override
  String get armorUpgradeMiddle => 'Central';

  @override
  String get armorUpgradeLower => 'Inferior';

  @override
  String get knowledgeCategoryTopic => 'Tema';

  @override
  String get knowledgeCategoryChoice => 'Opción';

  @override
  String get knowledgeCategoryInfo => 'Información';

  @override
  String get statusOk => 'Correcto';

  @override
  String get statusFailed => 'Fallido';

  @override
  String get missingSaveReference => 'Falta el archivo';

  @override
  String missingSaveReferenceDescription(String slot) {
    return 'Falta $slot.sav. Puede que se haya eliminado, movido o renombrado; el perfil aún hace referencia al archivo.';
  }

  @override
  String get removeFromProfile => 'Quitar del perfil';

  @override
  String get removeSaveFromProfileTitle => '¿Quitar la partida del perfil?';

  @override
  String removeSaveFromProfileBody(String save, String profile) {
    return '¿Quitar $save de $profile? El archivo de partida se conservará si aún existe.';
  }

  @override
  String get unassignedSave => 'Sin asignar a un perfil';

  @override
  String get armorUpgradeLight => 'Ligera';

  @override
  String get armorUpgradeMedium => 'Media';

  @override
  String get armorUpgradeHeavy => 'Pesada';

  @override
  String get knowledgeCaptionForcedConversation => 'Conversación forzada';

  @override
  String get knowledgeCaptionFollowupTopic => 'Tema de seguimiento';

  @override
  String get knowledgeCaptionFallbackTopic => 'Tema alternativo';

  @override
  String durationMinutes(int minutes) {
    return '$minutes min';
  }

  @override
  String durationHours(int hours) {
    return '$hours h';
  }

  @override
  String durationHoursMinutes(int hours, int minutes) {
    return '$hours h $minutes min';
  }

  @override
  String get backupStatusInvalidProfileStructure =>
      'Datos de perfil no válidos';

  @override
  String get backupStatusSlotMetadataMissing =>
      'Faltan los metadatos de la partida seleccionada';

  @override
  String defaultProfileName(int id) {
    return 'Perfil $id';
  }

  @override
  String get statusUnknown => 'Desconocido';

  @override
  String editorUnexpectedError(String details) {
    return 'Error inesperado: $details';
  }

  @override
  String get editorOperationInProgress =>
      'Hay otra operación en curso. Inténtalo de nuevo en un momento.';

  @override
  String get editorUnsavedBeforeDifficulty =>
      'La partida contiene cambios sin guardar. Guárdalos o restablécelos antes de cambiar la dificultad del perfil.';

  @override
  String get editorNoSaveFolderSelected =>
      'No se ha seleccionado ninguna carpeta de partidas.';

  @override
  String get editorNoSaveSelected => 'No se ha seleccionado ninguna partida.';

  @override
  String get coreUnknownError => 'Error interno desconocido';

  @override
  String get editorUnsavedBeforeSwitchProfile =>
      'Guarda o restablece primero los cambios pendientes; cambiar de perfil te haría abandonar la partida actual.';

  @override
  String get editorUnsavedBeforeOpenFile =>
      'Guarda o restablece los cambios pendientes antes de abrir otro archivo.';

  @override
  String get editorSelectSavFile => 'Selecciona un archivo de partida .sav.';

  @override
  String get editorNotGothicGsav =>
      'El archivo seleccionado no es una partida Gothic GSAV.';

  @override
  String get editorUnsavedBeforeChangeSaveProfile =>
      'Guarda o restablece los cambios pendientes antes de cambiar el perfil de la partida.';

  @override
  String get editorUnsavedBeforeRemoveProfile =>
      'Guarda o restablece los cambios pendientes antes de quitar una partida de su perfil.';

  @override
  String get editorUnsavedBeforeRestoreProfile =>
      'La partida contiene cambios sin guardar. Guárdalos o restablécelos antes de restaurar una copia de seguridad del perfil.';

  @override
  String editorConflictingPropertyEdits(String path) {
    return 'Hay cambios pendientes de dos pestañas que afectan a la misma propiedad ($path). Restablece o deshaz uno de ellos y vuelve a guardar.';
  }

  @override
  String editorGlossaryMemoryConflict(String path) {
    return 'Un cambio de segmento del glosario y otro cambio pendiente de Todos los datos afectan a la matriz Hero MemorizedEvents ($path). Los cambios del glosario añaden o eliminan entradas de esa matriz, por lo que no se pueden guardar juntos. Restablece o deshaz uno de ellos y vuelve a guardar.';
  }

  @override
  String editorGlossaryQuestConflict(String path) {
    return 'Un cambio de segmento del glosario y otro cambio pendiente afectan a la misma propiedad CurrentState de una misión ($path). El propio cambio del glosario actualiza ese estado. Restablece o deshaz uno de ellos y vuelve a guardar.';
  }

  @override
  String editorRelationshipConflict(String path) {
    return 'Un cambio de relación y otro cambio pendiente de Todos los datos afectan a la misma entrada de relación de un PNJ ($path). El cambio de relación estructurado puede sustituir modificadores de esa entrada, por lo que no se pueden guardar juntos. Restablece o deshaz uno de ellos y vuelve a guardar.';
  }

  @override
  String editorMultipleStructuralArrayEdits(String path) {
    return 'Hay más de un cambio estructural pendiente para la misma matriz ($path). Guarda o restablece el primer cambio antes de añadir otro.';
  }

  @override
  String editorStructuralArrayConflict(String path) {
    return 'Un cambio estructural de evento y otro cambio pendiente de Todos los datos afectan a $path. Guarda o restablece uno de ellos antes de continuar.';
  }

  @override
  String get editorSkillsEffectConflict =>
      'Hay pendientes un cambio de Habilidades y otro de Todos los datos para el mismo efecto del personaje (ActiveEffects › EffectSpec › Def). No se pueden guardar juntos. Restablece o deshaz uno de ellos y vuelve a guardar.';

  @override
  String get editorInventoryResetConflict =>
      'Hay pendientes un restablecimiento del inventario y otro cambio en el mismo inventario. El restablecimiento sustituye todo el inventario y descartaría el otro cambio. Restablece o deshaz uno de ellos y vuelve a guardar.';

  @override
  String get editorUseFolder => 'Usar carpeta';

  @override
  String get editorGothicSavegameFileType => 'Partida de Gothic';

  @override
  String get editorNoDifficultyChanges =>
      'No hay cambios de dificultad que guardar';

  @override
  String get editorDifficultyWritten =>
      'Dificultad guardada en el perfil (se creó una copia de seguridad)';

  @override
  String editorChangesSavedWithBackup(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count cambios guardados con copia de seguridad',
      one: '1 cambio guardado con copia de seguridad',
    );
    return '$_temp0';
  }

  @override
  String editorPlacementNoteFailed(String details) {
    return 'El traslado se guardó, pero no se pudo escribir su nota para deshacerlo: $details';
  }

  @override
  String editorProfileNotFound(int profileId) {
    return 'No se encontró el perfil $profileId.';
  }

  @override
  String get editorNoFreeSaveSlot =>
      'No hay ninguna ranura libre en la carpeta de partidas del juego (de G1R-001 a G1R-999).';

  @override
  String editorSaveImportedAssigned(int profileId) {
    return 'Partida importada y asignada al perfil $profileId';
  }

  @override
  String editorSaveAssigned(int profileId) {
    return 'Partida asignada al perfil $profileId (se crearon copias de seguridad asociadas)';
  }

  @override
  String editorSaveSlotNotAssigned(String slot, int profileId) {
    return 'La ranura de partida $slot no está asignada al perfil $profileId.';
  }

  @override
  String get editorSaveRemovedFromProfile => 'Partida quitada del perfil';

  @override
  String editorRestoredBackup(String path) {
    return 'Copia de seguridad restaurada: $path';
  }

  @override
  String editorRestoredBackupWithoutCompanion(String path) {
    return 'Copia de seguridad restaurada: $path (PersistentDataList.sav no se modificó porque no había una copia asociada coincidente; los metadatos de la ranura pueden diferir)';
  }

  @override
  String editorCodecRoundtripPassed(int chunkIndex, int bytes) {
    return 'Verificación de ida y vuelta del códec superada: el bloque $chunkIndex se volvió a comprimir a $bytes bytes';
  }

  @override
  String editorDifficultyWriteFailed(String details) {
    return 'No se pudo guardar la dificultad del perfil: $details';
  }

  @override
  String editorProfileAssignmentFailed(String details) {
    return 'No se pudo asignar la partida al perfil: $details';
  }

  @override
  String editorProfileRemovalFailed(String details) {
    return 'No se pudo quitar la partida del perfil: $details';
  }

  @override
  String editorSaveFailed(String details) {
    return 'No se pudieron guardar los cambios: $details';
  }

  @override
  String editorScanSavesFailed(String details) {
    return 'No se pudieron examinar las partidas: $details';
  }

  @override
  String editorInspectSaveFailed(String details) {
    return 'No se pudo inspeccionar la partida: $details';
  }

  @override
  String editorLoadBackupsFailed(String details) {
    return 'No se pudieron cargar las copias de seguridad: $details';
  }

  @override
  String editorRestoreFailed(String details) {
    return 'No se pudo restaurar la copia de seguridad: $details';
  }

  @override
  String editorRestoreReloadFailed(String path, String details) {
    return 'Copia de seguridad restaurada: $path, pero no se pudo volver a cargar la partida: $details';
  }

  @override
  String editorCodecCheckFailed(String details) {
    return 'Error al comprobar el códec: $details';
  }

  @override
  String editorCodecValidationFailed(String details) {
    return 'Error en la verificación de ida y vuelta del códec: $details';
  }

  @override
  String editorPropertySearchFailed(String details) {
    return 'Error al buscar propiedades: $details';
  }

  @override
  String get editorSelectionChangedWhileLoadingHeroAttributes =>
      'La partida seleccionada cambió mientras se cargaban los atributos del héroe.';

  @override
  String editorSkillsLoadFailed(String details) {
    return 'No se pudieron cargar las habilidades: $details';
  }

  @override
  String editorProgressionQueryFailed(String details) {
    return 'Error en la consulta de progresión: $details';
  }

  @override
  String editorNpcListFailed(String details) {
    return 'No se pudo cargar la lista de PNJ: $details';
  }

  @override
  String editorCharacterListFailed(String details) {
    return 'No se pudo cargar la lista de personajes: $details';
  }

  @override
  String editorNpcAttributesFailed(String details) {
    return 'No se pudieron cargar los atributos del PNJ: $details';
  }

  @override
  String editorNpcPositionFailed(String details) {
    return 'No se pudo cargar la posición del PNJ: $details';
  }

  @override
  String editorNpcInventoryFailed(String details) {
    return 'No se pudo cargar el inventario del PNJ: $details';
  }

  @override
  String editorFactionListFailed(String details) {
    return 'No se pudo cargar la lista de facciones: $details';
  }

  @override
  String get editorNoBackupPath => 'ninguna';

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
    return '$prefix: $backupPath; copia de seguridad de PersistentDataList: $persistentPath';
  }

  @override
  String localizationStatusFailed(String details) {
    return 'No se pudo obtener el estado de localización: $details';
  }

  @override
  String localizationExtractionFailed(String details) {
    return 'Error de extracción: $details';
  }

  @override
  String glossaryLoadFailed(String details) {
    return 'No se pudo cargar el glosario: $details';
  }

  @override
  String backupStatusError(String details) {
    return 'Error de la copia de seguridad: $details';
  }

  @override
  String memoryEventCategory(String category, String fallback) {
    String _temp0 = intl.Intl.selectLogic(category, {
      'quest': 'Misión',
      'document': 'Documento',
      'story': 'Historia',
      'exploration': 'Exploración',
      'combat': 'Combate',
      'social': 'Social',
      'item': 'Objetos',
      'learning': 'Aprendizaje',
      'guild': 'Gremio',
      'crime': 'Delito',
      'rest': 'Descanso',
      'other': '$fallback',
    });
    return '$_temp0';
  }

  @override
  String memoryEventAction(String kind, String fallback) {
    String _temp0 = intl.Intl.selectLogic(kind, {
      'questStarted': 'Misión iniciada',
      'questSucceeded': 'Misión completada',
      'questFailed': 'Misión fallida',
      'documentRead': 'Documento leído',
      'documentSegmentUnlocked': 'Entrada descubierta',
      'documentSegmentViewed': 'Entrada vista',
      'chapterCompleted': 'Capítulo completado',
      'areaEntered': 'Entrada en zona',
      'areaLeft': 'Salida de zona',
      'characterKilled': 'Personaje eliminado',
      'characterDefeated': 'Personaje derrotado',
      'combatDodge': 'Ataque esquivado',
      'characterDebuffed': 'Efecto negativo aplicado',
      'tradeAvailable': 'Comercio desbloqueado',
      'itemObtained': 'Objeto obtenido',
      'itemCrafted': 'Objeto fabricado',
      'skillStateRecorded': 'Estado de habilidades registrado',
      'recipeLearned': 'Receta aprendida',
      'guildJoined': 'Ingreso en el gremio',
      'crimeRecorded': 'Delito registrado',
      'slept': 'Descanso',
      'storyEvent': 'Evento de historia',
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
      'gameTime': 'Tiempo de juego',
      'duration': 'Duración',
      'chapter': 'Capítulo',
      'instigator': 'Iniciado por',
      'affected': 'Afectado',
      'amount': 'Cantidad',
      'primaryObject': 'Objeto',
      'secondaryObject': 'Contexto',
      'segmentText': 'Texto de la entrada',
      'other': '$fallback',
    });
    return '$_temp0';
  }

  @override
  String memoryEventGameTime(int day, String time) {
    return 'Día $day, $time';
  }

  @override
  String memoryEventSecondsValue(String value) {
    return '$value s';
  }

  @override
  String memoryEventMoreValues(String values, int count) {
    return '$values +$count';
  }

  @override
  String get memoryEventHero => 'Héroe';

  @override
  String get memoryEventDetails => 'Detalles';

  @override
  String get memoryEventTags => 'Etiquetas';

  @override
  String get memoryEventTechnicalData => 'Datos técnicos';

  @override
  String get memoryEventIndex => 'Índice';

  @override
  String get memoryEventPosition => 'Posición';

  @override
  String get memoryEventPayload => 'Contenido';

  @override
  String get memoryEventSubject => 'Asunto';

  @override
  String glossaryCatalogSegmentLabel(String segmentId, String fallback) {
    String _temp0 = intl.Intl.selectLogic(segmentId, {
      'Access': 'Acceso',
      'AccessDenied': 'Acceso denegado',
      'AccesToTemple': 'Acceso al templo',
      'Advice': 'Consejo',
      'AfterFight': 'Después de la pelea',
      'AfterFireMages': 'Después de los magos del fuego',
      'AfterNek': 'Después de Nek',
      'AfterQuest': 'Después de la misión',
      'Alone': 'Solo',
      'Amulet': 'Amuleto',
      'Annoying': 'Molesto',
      'Armor': 'Armadura',
      'Avoid': 'Evitar',
      'Backstory': 'Historia personal',
      'BackStory': 'Historia personal',
      'BasicMagic': 'Magia básica',
      'Beated': 'Derrotado',
      'BecomeMercenary': 'Convertirse en mercenario',
      'Beer': 'Cerveza',
      'Bestiary': 'Bestiario',
      'Blessing': 'Bendición',
      'Boss': 'Jefe',
      'Bully': 'Matón',
      'BullyAdvice': 'Consejo sobre el matón',
      'Camp': 'Campamento',
      'CampDivided': 'Campamento dividido',
      'CareOfMessengers': 'Cuidar de los mensajeros',
      'ChangeOpinion': 'Cambio de opinión',
      'ChargeUriziel': 'Cargar a Uriziel',
      'Chosen': 'Elegido',
      'Contact': 'Contacto',
      'Courier': 'Mensajero',
      'CraftBows': 'Fabricar arcos',
      'Crazy': 'Loco',
      'DailyMeal': 'Comida diaria',
      'DailyRation_Trader': 'Comerciante de raciones diarias',
      'DAM': 'Presa',
      'Dead': 'Muerto',
      'Deal': 'Trato',
      'Dealer': 'Comerciante',
      'Deceived': 'Engañado',
      'Dementia': 'Demencia',
      'DenyAccess': 'Denegar el acceso',
      'DifferentOpinion': 'Opinión diferente',
      'Discussion': 'Discusión',
      'DontTalk': 'No hablar',
      'Duel': 'Duelo',
      'Entrance': 'Entrada',
      'Escape': 'Huida',
      'Extended': 'Extendido',
      'Extra': 'Adicional',
      'ExtraInfo': 'Información adicional',
      'Fanatic': 'Fanático',
      'Fight': 'Combate',
      'FindUlumulu': 'Encontrar Ulu-Mulu',
      'FireMages': 'Magos de Fuego',
      'FireMagesEscape': 'Huida de los Magos de Fuego',
      'FiskNewDealer': 'Nuevo perista para Fisk',
      'FiskNewDealerCompleted': 'Nuevo perista para Fisk — completado',
      'FogTower': 'Torre de la Niebla',
      'Food': 'Comida',
      'Forgave': 'Perdonó',
      'Forgive': 'Perdonar',
      'Forgiven': 'Perdonado',
      'FourFriends': 'Cuatro amigos',
      'FreeHut': 'Choza libre',
      'FreeMine': 'Mina Libre',
      'Fury': 'Furia',
      'GoodTeacher': 'Buen maestro',
      'Gossip': 'Chismes',
      'GotScavenger': 'Carroñero obtenido',
      'GrantedAccess': 'Acceso concedido',
      'GRDArmor': 'Armadura de guardia',
      'Guide': 'Guía',
      'HateMages': 'Odio a los magos',
      'HateMagesExplanation': 'Explicación del odio a los magos',
      'HateRiceLord': 'Odio al Señor del Arroz',
      'Heal': 'Sanación',
      'Healing': 'Curación',
      'Help': 'Ayuda',
      'Helper': 'Ayudante',
      'HelpKagan': 'Ayudar a Kagan',
      'HutStory': 'Historia de la choza',
      'Ignore': 'Ignorar',
      'Impress': 'Impresionar',
      'ImpressAlchemy': 'Impresionar con alquimia',
      'ImpressInscription': 'Impresionar con inscripciones',
      'Info': 'Información',
      'Interested': 'Interesado',
      'Introduction': 'Introducción / retrato',
      'Introduction_2': 'Introducción / retrato 2',
      'Introduction_Armor': 'Introducción: armadura',
      'Introduction_Teacher': 'Introducción: instructor',
      'Introduction_Trader': 'Introducción: comerciante',
      'Invocation': 'Invocación',
      'JoinSC': 'Unirse al Campamento del Pantano',
      'Joint': 'Porro',
      'KalomCamp': 'Campamento de Kalom',
      'Leader': 'Líder',
      'Learning': 'Aprendizaje',
      'LearnOrcish': 'Aprender el idioma orco',
      'LeftParty': 'Abandonó el grupo',
      'Library': 'Biblioteca',
      'Lie': 'Mentira',
      'Lock': 'Cerradura',
      'Lockpick': 'Ganzúa',
      'Mad': 'Loco',
      'Mandibles': 'Mandíbulas de reptador',
      'MapMaker': 'Cartógrafo',
      'Monastery': 'Monasterio',
      'MordragKO': 'Mordrag KO',
      'Nek': 'Nek',
      'NewCamp': 'Campamento Nuevo',
      'NewCamper': 'Nuevo en el campamento',
      'NewLeader': 'Nuevo líder',
      'NightPatrol': 'Patrulla nocturna',
      'NotInterested': 'No interesado',
      'OldCamp': 'Campamento Viejo',
      'OrcEnclaveEntrance': 'Entrada al enclave orco',
      'OrcGraveyard': 'Cementerio de orcos',
      'OreArmor': 'Armadura de mineral',
      'Party': 'Grupo',
      'Pay': 'Pagar',
      'PayMoney': 'Pagar dinero',
      'Permission': 'Permiso',
      'Pet': 'Mascota',
      'PreparingInvocation': 'Preparando la invocación',
      'Quest': 'Misión',
      'RankUpFireMages': 'Ascenso a Mago de Fuego',
      'RankUpGuard': 'Ascenso a guardia',
      'RanUpFireMagesCompleted': 'Ascenso a Mago de Fuego completado',
      'Realocated': 'Reubicado',
      'Reason': 'Razón',
      'Respect': 'Respeto',
      'ReturnToSC': 'Regreso al Campamento del Pantano',
      'RicelordForeman': 'Capataz del Señor del Arroz',
      'RideScavenger': 'Montar un carroñero',
      'Robe': 'Túnica',
      'Safe': 'Seguro',
      'Scraper': 'Raspador',
      'SecondChance': 'Segunda oportunidad',
      'SecretLocation': 'Ubicación secreta',
      'SecretPassage': 'Pasaje secreto',
      'SecretPath': 'Camino secreto',
      'SleeperFollower': 'Seguidor del Durmiente',
      'SleeperTemple': 'Templo del Durmiente',
      'SmallInfo': 'Información breve',
      'Stonehenge': 'Círculo de piedras',
      'StopFollowing': 'Dejar de seguir',
      'SwampCamp': 'Campamento del Pantano',
      'Talkative': 'Hablador',
      'Teach': 'Enseñar',
      'TeachBow': 'Enseñar tiro con arco',
      'Teacher': 'Instructor',
      'Teacher2': 'Instructor 2',
      'TeacherInscription': 'Instructor de inscripciones',
      'TeacherMana': 'Instructor de maná',
      'TeachIchor': 'Enseñar a extraer icor de reptadores',
      'TeachMagic': 'Enseñar magia',
      'TeachOrcish': 'Enseñar el idioma orco',
      'TeachStats': 'Enseñar atributos',
      'TeachWeapon': 'Enseñar manejo de armas',
      'Teleport': 'Teletransporte',
      'TheMysteriousOrc': 'El orco misterioso',
      'ThroneRoom': 'Salón del Trono',
      'TradeBow': 'Comercio de arcos',
      'Trader': 'Comerciante',
      'TradeSkins_Trader': 'Comerciante de pieles',
      'Traitor': 'Traidor',
      'Trial': 'Prueba',
      'TrollCanyon': 'Cañón del trol',
      'Trust': 'Confianza',
      'Ulumulu': 'Ulu-Mulu',
      'Unexperienced': 'Sin experiencia',
      'Uriziel': 'Uriziel',
      'UrizielRune': 'Runa Uriziel',
      'Useful': 'Útil',
      'Velaya': 'Velaya',
      'Vibrations': 'Vibraciones',
      'WaitFreeMine': 'Esperar en la Mina Libre',
      'WaitInTrainingArea': 'Esperar en la zona de entrenamiento',
      'Warning': 'Advertencia',
      'WarningTooLate': 'Advertencia demasiado tardía',
      'WaterMessenger': 'Mensajero de los Magos del Agua',
      'Weapon': 'Arma',
      'Who': 'Quién',
      'Women': 'Mujeres',
      'other': '$fallback',
    });
    return '$_temp0';
  }

  @override
  String get slotRepairTitle => 'Ranuras de inventario dañadas';

  @override
  String slotRepairBody(int count) {
    return 'Esta partida guardada tiene $count ranuras de inventario cuyo id ya no coincide con su posición: en el juego, soltar uno de esos objetos elimina otro distinto. La reparación solo reescribe los id: no se añade, elimina ni cambia ningún objeto. Al guardar se crea una copia de seguridad, como siempre.';
  }

  @override
  String get slotRepairQueued => 'Reparación pendiente: guarda para aplicarla.';

  @override
  String get slotRepairAction => 'Reparar';

  @override
  String get slotRepairDiscard => 'Descartar';

  @override
  String get editorInventorySlotEditConflict =>
      'Hay en cola una edición directa de una ranura de inventario junto con una operación que ocupa ranuras enteras (reparar, añadir o eliminar). La segunda sobrescribiría la primera: revierte una de las dos y vuelve a guardar.';

  @override
  String get backupFactFile => 'Archivo';

  @override
  String get renameBackupTooltip => 'Poner nombre a esta copia';

  @override
  String get renameBackupTitle => 'Nombrar copia de seguridad';

  @override
  String get renameBackupLabel => 'Nombre';

  @override
  String renameBackupHelp(String fileName) {
    return 'Se muestra en lugar del nombre de archivo $fileName. Déjalo vacío para quitar el nombre; el archivo no se renombra.';
  }

  @override
  String get deleteBackupTooltip => 'Eliminar esta copia de seguridad';

  @override
  String get deleteBackupTitle => 'Eliminar copia de seguridad';

  @override
  String deleteBackupBody(String name, String fileName) {
    return '¿Eliminar «$name» ($fileName)? El archivo se borra del disco y no se puede recuperar.';
  }

  @override
  String get deleteBackupConfirm => 'Eliminar';

  @override
  String editorDeletedBackup(String path) {
    return 'Copia de seguridad eliminada: $path';
  }

  @override
  String editorDeleteBackupFailed(String details) {
    return 'No se pudo eliminar la copia de seguridad: $details';
  }

  @override
  String editorRenameBackupFailed(String details) {
    return 'No se pudo nombrar la copia de seguridad: $details';
  }

  @override
  String get slotRepairUnavailable =>
      'Ahora mismo no se puede reparar: esta partida guardada no se puede escribir.';

  @override
  String editorDeletedBackupWithLabelWarning(String path, String details) {
    return 'Copia de seguridad eliminada: $path: no se pudo quitar su nombre: $details';
  }

  @override
  String get slotRepairNotOffered =>
      'La reparación no está disponible para esta partida guardada.';
}
