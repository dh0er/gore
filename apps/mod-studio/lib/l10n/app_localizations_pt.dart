// ignore: unused_import
import 'package:intl/intl.dart' as intl;
import 'app_localizations.dart';

// ignore_for_file: type=lint

/// The translations for Portuguese (`pt`).
class AppLocalizationsPt extends AppLocalizations {
  AppLocalizationsPt([String locale = 'pt']) : super(locale);

  @override
  String get tabItems => 'Items';

  @override
  String get tabOverrides => 'Changes';

  @override
  String get tabSettings => 'Settings';

  @override
  String get tabDialogs => 'Diálogos';

  @override
  String get tabAudio => 'Áudio';

  @override
  String get tabTextures => 'Texturas';

  @override
  String get tabScripts => 'Scripts';

  @override
  String get changesAll => 'Todos';

  @override
  String get sectionItemValues => 'Valores dos itens';

  @override
  String get sectionLocalizedText => 'Textos localizados';

  @override
  String get audioCatCreatures => 'Criaturas';

  @override
  String get audioCatObjects => 'Objetos';

  @override
  String get audioCatMagic => 'Magia';

  @override
  String get audioCatMovement => 'Movimento';

  @override
  String get audioCatWorld => 'Mundo';

  @override
  String get audioCatAction => 'Ações';

  @override
  String get audioCatCombat => 'Combate';

  @override
  String get audioCatPhysics => 'Física';

  @override
  String get audioCatItems => 'Itens';

  @override
  String get audioCatUi => 'Interface';

  @override
  String get audioCatFoley => 'Foley';

  @override
  String get audioCatUnderwater => 'Subaquático';

  @override
  String get audioCatVision => 'Visões';

  @override
  String get audioCatDialog => 'Diálogo';

  @override
  String get audioCatOther => 'Outros';

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
  String get extractLocalizedText => 'Extrair textos localizados';

  @override
  String get lightMode => 'Modo claro';

  @override
  String get darkMode => 'Modo escuro';

  @override
  String get language => 'Idioma';

  @override
  String get exportMod => 'Exportar mod';

  @override
  String exportModWithCount(int count) {
    return 'Exportar mod ($count)';
  }

  @override
  String get selectAnItemToEdit => 'Selecione um item para editar seus campos.';

  @override
  String gameDataActiveTooltip(String name) {
    return 'Dados do jogo: $name';
  }

  @override
  String get gameDataBundledTooltip => 'Dados do jogo: incluídos';

  @override
  String get loadGameDataDump => 'Carregar dump de dados do jogo…';

  @override
  String get loadGameDataDumpSubtitle => 'gore_game_data.json do mod gore-dump';

  @override
  String get useBundledData => 'Usar dados incluídos';

  @override
  String get alreadyBundled => 'já incluídos';

  @override
  String get gameDataFileGroupLabel => 'dados do jogo';

  @override
  String get minimize => 'Minimizar';

  @override
  String get restore => 'Restaurar';

  @override
  String get maximize => 'Maximizar';

  @override
  String get close => 'Fechar';

  @override
  String get about => 'Sobre';

  @override
  String aboutVersion(String version, String sha) {
    return 'Version $version ($sha)';
  }

  @override
  String get aboutCopyright => '© 2026 colaboradores do GORE';

  @override
  String get aboutLicense => 'Licenciado sob a Licença MIT.';

  @override
  String get categoryMeleeWeapons => 'Armas corpo a corpo';

  @override
  String get categoryRangedWeapons => 'Armas à distância';

  @override
  String get categoryAmmunition => 'Munição';

  @override
  String get categoryRunes => 'Runas';

  @override
  String get categorySpellScrolls => 'Pergaminhos de magia';

  @override
  String get categoryFoodAndPotions => 'Comida e poções';

  @override
  String get categoryMiscellaneous => 'Diversos';

  @override
  String get categoryAmulets => 'Amuletos';

  @override
  String get categoryRings => 'Anéis';

  @override
  String get categoryAnimalTrophies => 'Troféus de animais';

  @override
  String get categoryWritings => 'Escritos';

  @override
  String get categoryMissionItems => 'Itens de missão';

  @override
  String get categoryKeys => 'Chaves';

  @override
  String get categoryOther => 'Outros';

  @override
  String categoryWithCount(String label, int count) {
    return '$label ($count)';
  }

  @override
  String get searchItems => 'Pesquisar itens';

  @override
  String get noItemsMatch => 'Nenhum item corresponde';

  @override
  String failedToLoadCatalog(String error) {
    return 'Falha ao carregar o catálogo: $error';
  }

  @override
  String pendingOverridesWithCount(int count) {
    return 'Alterações pendentes ($count)';
  }

  @override
  String get clearAll => 'Limpar tudo';

  @override
  String get noPendingOverrides =>
      'Nenhuma alteração pendente.\nEdite os campos dos itens para adicionar algumas.';

  @override
  String get removeOverride => 'Remover alteração';

  @override
  String get searchChanges => 'Pesquisar alterações';

  @override
  String get noChangesMatch => 'Nenhuma alteração corresponde';

  @override
  String get clearSection => 'Limpar este grupo';

  @override
  String get modName => 'Nome do mod';

  @override
  String get loadDelayLabel => 'Atraso de carregamento (ms, 0 = imediato)';

  @override
  String get noFolderSelected => 'Nenhuma pasta selecionada';

  @override
  String get chooseFolder => 'Escolher pasta';

  @override
  String get packageAsZip => 'Empacotar como .zip';

  @override
  String get cancel => 'Cancelar';

  @override
  String get export => 'Exportar';

  @override
  String get exportHere => 'Exportar aqui';

  @override
  String get mustBeNonNegativeInteger => 'Deve ser um inteiro não negativo';

  @override
  String get extractingLocalizedText => 'Extraindo textos localizados do jogo…';

  @override
  String get localizedTextExtractionCancelled =>
      'Extração de textos localizados cancelada.';

  @override
  String get localizedTextExtracted => 'Textos localizados extraídos.';

  @override
  String get extractionFailed => 'Falha na extração.';

  @override
  String get localizationCacheFileGroupLabel => 'cache de localização';

  @override
  String get extractLocalizedTextQuestion =>
      'Extrair os textos localizados do jogo?';

  @override
  String get extractLocalizedTextBody =>
      'Os textos localizados do jogo ainda não foram extraídos. Extraí-los agora da sua instalação do jogo? (opcional)';

  @override
  String get notNow => 'Agora não';

  @override
  String get extract => 'Extrair';

  @override
  String get validationRequired => 'Obrigatório';

  @override
  String get validationMustBeWholeNumber => 'Deve ser um número inteiro';

  @override
  String get validationMustBeNumber => 'Deve ser um número';

  @override
  String get validationMustBeFinite => 'Deve ser um número finito';

  @override
  String validationMustBeAtLeast(String min) {
    return 'Deve ser ≥ $min';
  }

  @override
  String validationMustBeAtMost(String max) {
    return 'Deve ser ≤ $max';
  }

  @override
  String get validationMustBeBool => 'Deve ser true ou false';

  @override
  String validationMustBeOneOf(String options) {
    return 'Deve ser um de: $options';
  }

  @override
  String get modNameRequired => 'Obrigatório';

  @override
  String get modNameControlCharacters =>
      'Não deve conter caracteres de controle';

  @override
  String get modNamePathSeparators => 'Não deve conter separadores de caminho';

  @override
  String get modNameNotAFolderName => 'Nome de pasta inválido';

  @override
  String localizedTextExtractedCount(int idCount, int languageCount) {
    return '$idCount IDs extraídos em $languageCount idiomas';
  }

  @override
  String get managerDeployActive =>
      'Um loadout do mod-manager está ativo. Faça primeiro o undeploy no gore-manager.';
}

/// The translations for Portuguese, as used in Brazil (`pt_BR`).
class AppLocalizationsPtBr extends AppLocalizationsPt {
  AppLocalizationsPtBr() : super('pt_BR');

  @override
  String get tabDialogs => 'Diálogos';

  @override
  String get tabAudio => 'Áudio';

  @override
  String get tabTextures => 'Texturas';

  @override
  String get tabScripts => 'Scripts';

  @override
  String get changesAll => 'Todos';

  @override
  String get sectionItemValues => 'Valores dos itens';

  @override
  String get sectionLocalizedText => 'Textos localizados';

  @override
  String get audioCatCreatures => 'Criaturas';

  @override
  String get audioCatObjects => 'Objetos';

  @override
  String get audioCatMagic => 'Magia';

  @override
  String get audioCatMovement => 'Movimento';

  @override
  String get audioCatWorld => 'Mundo';

  @override
  String get audioCatAction => 'Ações';

  @override
  String get audioCatCombat => 'Combate';

  @override
  String get audioCatPhysics => 'Física';

  @override
  String get audioCatItems => 'Itens';

  @override
  String get audioCatUi => 'Interface';

  @override
  String get audioCatFoley => 'Foley';

  @override
  String get audioCatUnderwater => 'Subaquático';

  @override
  String get audioCatVision => 'Visões';

  @override
  String get audioCatDialog => 'Diálogo';

  @override
  String get audioCatOther => 'Outros';

  @override
  String get extractLocalizedText => 'Extrair textos localizados';

  @override
  String get lightMode => 'Modo claro';

  @override
  String get darkMode => 'Modo escuro';

  @override
  String get language => 'Idioma';

  @override
  String get exportMod => 'Exportar mod';

  @override
  String exportModWithCount(int count) {
    return 'Exportar mod ($count)';
  }

  @override
  String get selectAnItemToEdit => 'Selecione um item para editar seus campos.';

  @override
  String gameDataActiveTooltip(String name) {
    return 'Dados do jogo: $name';
  }

  @override
  String get gameDataBundledTooltip => 'Dados do jogo: incluídos';

  @override
  String get loadGameDataDump => 'Carregar dump de dados do jogo…';

  @override
  String get loadGameDataDumpSubtitle => 'gore_game_data.json do mod gore-dump';

  @override
  String get useBundledData => 'Usar dados incluídos';

  @override
  String get alreadyBundled => 'já incluídos';

  @override
  String get gameDataFileGroupLabel => 'dados do jogo';

  @override
  String get minimize => 'Minimizar';

  @override
  String get restore => 'Restaurar';

  @override
  String get maximize => 'Maximizar';

  @override
  String get close => 'Fechar';

  @override
  String get about => 'Sobre';

  @override
  String aboutVersion(String version, String sha) {
    return 'Version $version ($sha)';
  }

  @override
  String get aboutCopyright => '© 2026 colaboradores do GORE';

  @override
  String get aboutLicense => 'Licenciado sob a Licença MIT.';

  @override
  String get categoryMeleeWeapons => 'Armas corpo a corpo';

  @override
  String get categoryRangedWeapons => 'Armas à distância';

  @override
  String get categoryAmmunition => 'Munição';

  @override
  String get categoryRunes => 'Runas';

  @override
  String get categorySpellScrolls => 'Pergaminhos de magia';

  @override
  String get categoryFoodAndPotions => 'Comida e poções';

  @override
  String get categoryMiscellaneous => 'Diversos';

  @override
  String get categoryAmulets => 'Amuletos';

  @override
  String get categoryRings => 'Anéis';

  @override
  String get categoryAnimalTrophies => 'Troféus de animais';

  @override
  String get categoryWritings => 'Escritos';

  @override
  String get categoryMissionItems => 'Itens de missão';

  @override
  String get categoryKeys => 'Chaves';

  @override
  String get categoryOther => 'Outros';

  @override
  String categoryWithCount(String label, int count) {
    return '$label ($count)';
  }

  @override
  String get searchItems => 'Pesquisar itens';

  @override
  String get noItemsMatch => 'Nenhum item corresponde';

  @override
  String failedToLoadCatalog(String error) {
    return 'Falha ao carregar o catálogo: $error';
  }

  @override
  String pendingOverridesWithCount(int count) {
    return 'Alterações pendentes ($count)';
  }

  @override
  String get clearAll => 'Limpar tudo';

  @override
  String get noPendingOverrides =>
      'Nenhuma alteração pendente.\nEdite os campos dos itens para adicionar algumas.';

  @override
  String get removeOverride => 'Remover alteração';

  @override
  String get searchChanges => 'Pesquisar alterações';

  @override
  String get noChangesMatch => 'Nenhuma alteração corresponde';

  @override
  String get clearSection => 'Limpar este grupo';

  @override
  String get modName => 'Nome do mod';

  @override
  String get loadDelayLabel => 'Atraso de carregamento (ms, 0 = imediato)';

  @override
  String get noFolderSelected => 'Nenhuma pasta selecionada';

  @override
  String get chooseFolder => 'Escolher pasta';

  @override
  String get packageAsZip => 'Empacotar como .zip';

  @override
  String get cancel => 'Cancelar';

  @override
  String get export => 'Exportar';

  @override
  String get exportHere => 'Exportar aqui';

  @override
  String get mustBeNonNegativeInteger => 'Deve ser um inteiro não negativo';

  @override
  String get extractingLocalizedText => 'Extraindo textos localizados do jogo…';

  @override
  String get localizedTextExtractionCancelled =>
      'Extração de textos localizados cancelada.';

  @override
  String get localizedTextExtracted => 'Textos localizados extraídos.';

  @override
  String get extractionFailed => 'Falha na extração.';

  @override
  String get localizationCacheFileGroupLabel => 'cache de localização';

  @override
  String get extractLocalizedTextQuestion =>
      'Extrair os textos localizados do jogo?';

  @override
  String get extractLocalizedTextBody =>
      'Os textos localizados do jogo ainda não foram extraídos. Extraí-los agora da sua instalação do jogo? (opcional)';

  @override
  String get notNow => 'Agora não';

  @override
  String get extract => 'Extrair';

  @override
  String get validationRequired => 'Obrigatório';

  @override
  String get validationMustBeWholeNumber => 'Deve ser um número inteiro';

  @override
  String get validationMustBeNumber => 'Deve ser um número';

  @override
  String get validationMustBeFinite => 'Deve ser um número finito';

  @override
  String validationMustBeAtLeast(String min) {
    return 'Deve ser ≥ $min';
  }

  @override
  String validationMustBeAtMost(String max) {
    return 'Deve ser ≤ $max';
  }

  @override
  String get validationMustBeBool => 'Deve ser true ou false';

  @override
  String validationMustBeOneOf(String options) {
    return 'Deve ser um de: $options';
  }

  @override
  String get modNameRequired => 'Obrigatório';

  @override
  String get modNameControlCharacters =>
      'Não deve conter caracteres de controle';

  @override
  String get modNamePathSeparators => 'Não deve conter separadores de caminho';

  @override
  String get modNameNotAFolderName => 'Nome de pasta inválido';

  @override
  String localizedTextExtractedCount(int idCount, int languageCount) {
    return '$idCount IDs extraídos em $languageCount idiomas';
  }

  @override
  String get managerDeployActive =>
      'Um loadout do mod-manager está ativo. Primeiro faça o undeploy no gore-manager.';
}
