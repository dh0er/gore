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
}
