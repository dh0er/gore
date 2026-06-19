// ignore: unused_import
import 'package:intl/intl.dart' as intl;
import 'app_localizations.dart';

// ignore_for_file: type=lint

/// The translations for Italian (`it`).
class AppLocalizationsIt extends AppLocalizations {
  AppLocalizationsIt([String locale = 'it']) : super(locale);

  @override
  String get extractLocalizedText => 'Estrai testi localizzati';

  @override
  String get lightMode => 'Modalità chiara';

  @override
  String get darkMode => 'Modalità scura';

  @override
  String get language => 'Lingua';

  @override
  String get exportMod => 'Esporta mod';

  @override
  String exportModWithCount(int count) {
    return 'Esporta mod ($count)';
  }

  @override
  String get selectAnItemToEdit =>
      'Seleziona un oggetto per modificarne i campi.';

  @override
  String gameDataActiveTooltip(String name) {
    return 'Dati di gioco: $name';
  }

  @override
  String get gameDataBundledTooltip => 'Dati di gioco: inclusi';

  @override
  String get loadGameDataDump => 'Carica dump dei dati di gioco…';

  @override
  String get loadGameDataDumpSubtitle =>
      'gore_game_data.json dalla mod gore-dump';

  @override
  String get useBundledData => 'Usa i dati inclusi';

  @override
  String get alreadyBundled => 'già inclusi';

  @override
  String get gameDataFileGroupLabel => 'dati di gioco';

  @override
  String get minimize => 'Riduci a icona';

  @override
  String get restore => 'Ripristina';

  @override
  String get maximize => 'Ingrandisci';

  @override
  String get close => 'Chiudi';

  @override
  String get categoryMeleeWeapons => 'Armi da mischia';

  @override
  String get categoryRangedWeapons => 'Armi a distanza';

  @override
  String get categoryAmmunition => 'Munizioni';

  @override
  String get categoryRunes => 'Rune';

  @override
  String get categorySpellScrolls => 'Pergamene magiche';

  @override
  String get categoryFoodAndPotions => 'Cibo e pozioni';

  @override
  String get categoryMiscellaneous => 'Varie';

  @override
  String get categoryAmulets => 'Amuleti';

  @override
  String get categoryRings => 'Anelli';

  @override
  String get categoryAnimalTrophies => 'Trofei di animali';

  @override
  String get categoryWritings => 'Scritti';

  @override
  String get categoryMissionItems => 'Oggetti della missione';

  @override
  String get categoryKeys => 'Chiavi';

  @override
  String get categoryOther => 'Altro';

  @override
  String categoryWithCount(String label, int count) {
    return '$label ($count)';
  }

  @override
  String get searchItems => 'Cerca oggetti';

  @override
  String get noItemsMatch => 'Nessun oggetto corrispondente';

  @override
  String failedToLoadCatalog(String error) {
    return 'Impossibile caricare il catalogo: $error';
  }

  @override
  String pendingOverridesWithCount(int count) {
    return 'Modifiche in sospeso ($count)';
  }

  @override
  String get clearAll => 'Cancella tutto';

  @override
  String get noPendingOverrides =>
      'Nessuna modifica in sospeso.\nModifica i campi degli oggetti per aggiungerne.';

  @override
  String get removeOverride => 'Rimuovi modifica';

  @override
  String get modName => 'Nome della mod';

  @override
  String get loadDelayLabel => 'Ritardo di caricamento (ms, 0 = immediato)';

  @override
  String get noFolderSelected => 'Nessuna cartella selezionata';

  @override
  String get chooseFolder => 'Scegli cartella';

  @override
  String get packageAsZip => 'Crea pacchetto .zip';

  @override
  String get cancel => 'Annulla';

  @override
  String get export => 'Esporta';

  @override
  String get exportHere => 'Esporta qui';

  @override
  String get mustBeNonNegativeInteger => 'Deve essere un intero non negativo';

  @override
  String get extractingLocalizedText =>
      'Estrazione dei testi localizzati del gioco…';

  @override
  String get localizedTextExtractionCancelled =>
      'Estrazione dei testi localizzati annullata.';

  @override
  String get localizedTextExtracted => 'Testi localizzati estratti.';

  @override
  String get extractionFailed => 'Estrazione non riuscita.';

  @override
  String get localizationCacheFileGroupLabel => 'cache di localizzazione';

  @override
  String get extractLocalizedTextQuestion =>
      'Estrarre i testi localizzati del gioco?';

  @override
  String get extractLocalizedTextBody =>
      'I testi localizzati del gioco non sono ancora stati estratti. Estrarli ora dalla tua installazione del gioco? (facoltativo)';

  @override
  String get notNow => 'Non ora';

  @override
  String get extract => 'Estrai';

  @override
  String get validationRequired => 'Obbligatorio';

  @override
  String get validationMustBeWholeNumber => 'Deve essere un numero intero';

  @override
  String get validationMustBeNumber => 'Deve essere un numero';

  @override
  String get validationMustBeFinite => 'Deve essere un numero finito';

  @override
  String validationMustBeAtLeast(String min) {
    return 'Deve essere ≥ $min';
  }

  @override
  String validationMustBeAtMost(String max) {
    return 'Deve essere ≤ $max';
  }

  @override
  String get validationMustBeBool => 'Deve essere true o false';

  @override
  String validationMustBeOneOf(String options) {
    return 'Deve essere uno tra: $options';
  }

  @override
  String get modNameRequired => 'Obbligatorio';

  @override
  String get modNameControlCharacters =>
      'Non deve contenere caratteri di controllo';

  @override
  String get modNamePathSeparators =>
      'Non deve contenere separatori di percorso';

  @override
  String get modNameNotAFolderName => 'Nome cartella non valido';
}
