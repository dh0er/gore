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
      'O novo projeto está aberto, mas não foi possível limpar completamente a sessão do projeto anterior. A limpeza não será repetida. Reinicie o Mod Studio antes de voltar a abrir o projeto anterior.';

  @override
  String get projectNewManagedRevision3 => 'Novo projeto de mod gerido…';

  @override
  String get projectNewLegacy => 'Novo projeto legado';

  @override
  String get projectCreateGamePathRequired =>
      'Defina o caminho do Gothic 1 Remake nas Definições antes de criar um projeto de mod.';

  @override
  String get projectCreateDirectoryPickerTitle =>
      'Criar aqui o projeto de mod gerido';

  @override
  String projectManagedRevision3Created(String projectId) {
    return 'Projeto de mod gerido $projectId criado';
  }

  @override
  String projectManagedRevision3CreateFailed(String error) {
    return 'Falha ao criar o projeto de mod gerido: $error';
  }

  @override
  String get projectCreateDialogTitle => 'Criar um projeto de mod';

  @override
  String get projectCreateNameLabel => 'Nome do projeto';

  @override
  String get projectCreateNameHelper => 'O nome apresentado no Mod Studio.';

  @override
  String get projectCreateVersionLabel => 'Versão';

  @override
  String get projectCreateVersionHelper => 'Uma versão inicial, como 0.1.0.';

  @override
  String get projectCreateAuthorLabel => 'Autor';

  @override
  String get projectCreateAuthorHelper =>
      'O seu nome ou o nome da equipa de modding.';

  @override
  String get projectCreateLocalesLabel => 'Idiomas de edição';

  @override
  String get projectCreateLocalesHelper =>
      'Etiquetas canónicas separadas por vírgulas, por exemplo: en, de, en-US.';

  @override
  String get projectCreateBoundary =>
      'Isto cria um projeto offline gerido e vazio. Não compila, implementa nem executa um mod e não altera ficheiros do jogo ou gravações.';

  @override
  String get projectCreateSubmit => 'Criar projeto';

  @override
  String projectCreateMetadataRequired(String label) {
    return '$label é obrigatório.';
  }

  @override
  String projectCreateMetadataNoOuterWhitespace(String label) {
    return '$label não pode começar nem terminar com espaços.';
  }

  @override
  String projectCreateMetadataControlCharacters(String label) {
    return '$label não pode conter caracteres de controlo.';
  }

  @override
  String projectCreateMetadataMalformed(String label) {
    return '$label contém texto inválido.';
  }

  @override
  String projectCreateMetadataTooLong(String label, int maxBytes) {
    return '$label excede o limite UTF-8 de $maxBytes bytes.';
  }

  @override
  String get projectCreateLocalesRequired =>
      'Introduza pelo menos um idioma de edição.';

  @override
  String get projectCreateLocalesEmptyEntry =>
      'Remova a entrada de idioma vazia.';

  @override
  String projectCreateLocalesTooMany(int maxLocales) {
    return 'Utilize no máximo $maxLocales idiomas de edição.';
  }

  @override
  String projectCreateLocaleBoundedAscii(String locale) {
    return 'A região «$locale» tem de ser ASCII e ter comprimento limitado.';
  }

  @override
  String projectCreateLocaleLanguage(String locale) {
    return 'A região «$locale» necessita de um idioma em minúsculas com 2 a 8 letras.';
  }

  @override
  String projectCreateLocaleInvalidSegment(String locale) {
    return 'A região «$locale» contém um segmento inválido.';
  }

  @override
  String projectCreateLocaleNotCanonical(String locale, String canonical) {
    return 'A região «$locale» não é canónica; utilize «$canonical».';
  }
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

  @override
  String get projectTransitionCleanupWarning =>
      'O novo projeto está aberto, mas não foi possível limpar completamente a sessão do projeto anterior. A limpeza não será repetida. Reinicie o Mod Studio antes de reabrir o projeto anterior.';

  @override
  String get projectNewManagedRevision3 => 'Novo projeto de mod gerenciado…';

  @override
  String get projectNewLegacy => 'Novo projeto legado';

  @override
  String get projectCreateGamePathRequired =>
      'Defina o caminho do Gothic 1 Remake nas Configurações antes de criar um projeto de mod.';

  @override
  String get projectCreateDirectoryPickerTitle =>
      'Criar aqui o projeto de mod gerenciado';

  @override
  String projectManagedRevision3Created(String projectId) {
    return 'Projeto de mod gerenciado $projectId criado';
  }

  @override
  String projectManagedRevision3CreateFailed(String error) {
    return 'Falha ao criar o projeto de mod gerenciado: $error';
  }

  @override
  String get projectCreateDialogTitle => 'Criar um projeto de mod';

  @override
  String get projectCreateNameLabel => 'Nome do projeto';

  @override
  String get projectCreateNameHelper => 'O nome exibido no Mod Studio.';

  @override
  String get projectCreateVersionLabel => 'Versão';

  @override
  String get projectCreateVersionHelper => 'Uma versão inicial, como 0.1.0.';

  @override
  String get projectCreateAuthorLabel => 'Autor';

  @override
  String get projectCreateAuthorHelper =>
      'Seu nome ou o nome da equipe de modding.';

  @override
  String get projectCreateLocalesLabel => 'Idiomas de edição';

  @override
  String get projectCreateLocalesHelper =>
      'Tags canônicas separadas por vírgulas, por exemplo: en, de, en-US.';

  @override
  String get projectCreateBoundary =>
      'Isso cria um projeto offline gerenciado e vazio. Não compila, implanta nem executa um mod e não altera arquivos do jogo ou saves.';

  @override
  String get projectCreateSubmit => 'Criar projeto';

  @override
  String projectCreateMetadataRequired(String label) {
    return '$label é obrigatório.';
  }

  @override
  String projectCreateMetadataNoOuterWhitespace(String label) {
    return '$label não pode começar nem terminar com espaços.';
  }

  @override
  String projectCreateMetadataControlCharacters(String label) {
    return '$label não pode conter caracteres de controle.';
  }

  @override
  String projectCreateMetadataMalformed(String label) {
    return '$label contém texto inválido.';
  }

  @override
  String projectCreateMetadataTooLong(String label, int maxBytes) {
    return '$label excede o limite UTF-8 de $maxBytes bytes.';
  }

  @override
  String get projectCreateLocalesRequired =>
      'Insira pelo menos um idioma de edição.';

  @override
  String get projectCreateLocalesEmptyEntry =>
      'Remova a entrada de idioma vazia.';

  @override
  String projectCreateLocalesTooMany(int maxLocales) {
    return 'Use no máximo $maxLocales idiomas de edição.';
  }

  @override
  String projectCreateLocaleBoundedAscii(String locale) {
    return 'A localidade “$locale” deve ser ASCII e ter tamanho limitado.';
  }

  @override
  String projectCreateLocaleLanguage(String locale) {
    return 'A localidade “$locale” precisa de um idioma em minúsculas com 2 a 8 letras.';
  }

  @override
  String projectCreateLocaleInvalidSegment(String locale) {
    return 'A localidade “$locale” contém um segmento inválido.';
  }

  @override
  String projectCreateLocaleNotCanonical(String locale, String canonical) {
    return 'A localidade “$locale” não é canônica; use “$canonical”.';
  }
}
