// ignore: unused_import
import 'package:intl/intl.dart' as intl;
import 'app_localizations.dart';

// ignore_for_file: type=lint

/// The translations for Portuguese (`pt`).
class AppLocalizationsPt extends AppLocalizations {
  AppLocalizationsPt([String locale = 'pt']) : super(locale);

  @override
  String get appTitle => 'GORE Mod Manager';

  @override
  String get tabMods => 'Mods';

  @override
  String get tabSettings => 'Definições';

  @override
  String get settingsGameExe => 'Executável do jogo';

  @override
  String get settingsGameExePick => 'Escolher…';

  @override
  String get settingsLanguage => 'Idioma';

  @override
  String get statusInSync => 'Sincronizado';

  @override
  String get statusChangesPending => 'Alterações pendentes';

  @override
  String get statusGameUpdated => 'Jogo atualizado';

  @override
  String get statusStudioDeploy => 'Implementação do Studio ativa';

  @override
  String get statusNothingDeployed => 'Nada implementado';

  @override
  String get actionImport => 'Importar';

  @override
  String get actionApply => 'Aplicar';

  @override
  String get actionUndeployAll => 'Remover tudo';

  @override
  String get commonCancel => 'Cancelar';

  @override
  String get commonOk => 'OK';

  @override
  String get importFolder => 'Importar pasta…';

  @override
  String get importFile => 'Importar ficheiro…';

  @override
  String get applyTooltip => 'Aplicar a configuração ao jogo';

  @override
  String get undeployAllAction => 'Remover tudo';

  @override
  String get undeployAllConfirm =>
      'Remover do jogo tudo o que o gestor implementou?';

  @override
  String get takeOverTitle => 'Implementação do Studio ativa';

  @override
  String get takeOverBody =>
      'O mod-studio implementou um mod no jogo. Assumir o controlo para que o gestor possa aplicar esta configuração?';

  @override
  String get takeOverAction => 'Assumir';

  @override
  String get refreshAction => 'Atualizar';

  @override
  String conflictsTitle(int count) {
    return 'Conflitos ($count)';
  }

  @override
  String get conflictWinner => 'vencedor';

  @override
  String get noConflicts => 'Sem conflitos.';

  @override
  String get componentsTitle => 'Componentes';

  @override
  String targetsMore(int count) {
    return '+$count mais';
  }

  @override
  String get removeModAction => 'Remover';

  @override
  String removeModConfirm(String name) {
    return 'Remover «$name» da biblioteca?';
  }

  @override
  String get errorSetGamePath =>
      'Defina primeiro o caminho do jogo nas Definições.';

  @override
  String applyReportApplied(int count) {
    return '$count mods aplicados.';
  }

  @override
  String get warningsTitle => 'Avisos';

  @override
  String get modDisabledHint => 'Desativado';

  @override
  String get kindGoremod => 'goremod';

  @override
  String get kindTriplet => 'triplet';

  @override
  String get kindPak => 'pak';

  @override
  String get kindUe4ss => 'UE4SS';

  @override
  String get kindRawfile => 'ficheiro em bruto';

  @override
  String get kindMixed => 'misto';

  @override
  String get sevHard => 'grave';

  @override
  String get sevSoft => 'ligeiro';

  @override
  String get sevInfo => 'info';

  @override
  String aboutVersion(String version, String sha) {
    return 'Version $version ($sha)';
  }

  @override
  String get about => 'Sobre';

  @override
  String get aboutCopyright => '© 2026 colaboradores do GORE';

  @override
  String get aboutLicense => 'Licenciado sob a Licença MIT.';

  @override
  String get appearanceTitle => 'Aparência';

  @override
  String get theme => 'Tema';

  @override
  String get themeLight => 'Claro';

  @override
  String get themeDark => 'Escuro';

  @override
  String get themeSystem => 'Sistema';

  @override
  String get uiScale => 'Escala da interface';

  @override
  String get resetZoomTooltip => 'Redefinir zoom (Ctrl+0)';

  @override
  String get zoomTip =>
      'Dica: Ctrl + / Ctrl - altera o zoom em qualquer parte do app.';

  @override
  String get lightMode => 'Modo claro';

  @override
  String get darkMode => 'Modo escuro';

  @override
  String get minimize => 'Minimizar';

  @override
  String get restore => 'Restaurar';

  @override
  String get maximize => 'Maximizar';

  @override
  String get close => 'Fechar';
}

/// The translations for Portuguese, as used in Brazil (`pt_BR`).
class AppLocalizationsPtBr extends AppLocalizationsPt {
  AppLocalizationsPtBr() : super('pt_BR');

  @override
  String get appTitle => 'GORE Mod Manager';

  @override
  String get tabMods => 'Mods';

  @override
  String get tabSettings => 'Configurações';

  @override
  String get settingsGameExe => 'Executável do jogo';

  @override
  String get settingsGameExePick => 'Escolher…';

  @override
  String get settingsLanguage => 'Idioma';

  @override
  String get statusInSync => 'Sincronizado';

  @override
  String get statusChangesPending => 'Alterações pendentes';

  @override
  String get statusGameUpdated => 'Jogo atualizado';

  @override
  String get statusStudioDeploy => 'Implantação do Studio ativa';

  @override
  String get statusNothingDeployed => 'Nada implantado';

  @override
  String get actionImport => 'Importar';

  @override
  String get actionApply => 'Aplicar';

  @override
  String get actionUndeployAll => 'Remover tudo';

  @override
  String get commonCancel => 'Cancelar';

  @override
  String get commonOk => 'OK';

  @override
  String get importFolder => 'Importar pasta…';

  @override
  String get importFile => 'Importar arquivo…';

  @override
  String get applyTooltip => 'Aplicar a configuração ao jogo';

  @override
  String get undeployAllAction => 'Remover tudo';

  @override
  String get undeployAllConfirm =>
      'Remover do jogo tudo o que o gerenciador implantou?';

  @override
  String get takeOverTitle => 'Implantação do Studio ativa';

  @override
  String get takeOverBody =>
      'O mod-studio implantou um mod no jogo. Assumir o controle para que o gerenciador possa aplicar esta configuração?';

  @override
  String get takeOverAction => 'Assumir';

  @override
  String get refreshAction => 'Atualizar';

  @override
  String conflictsTitle(int count) {
    return 'Conflitos ($count)';
  }

  @override
  String get conflictWinner => 'vencedor';

  @override
  String get noConflicts => 'Sem conflitos.';

  @override
  String get componentsTitle => 'Componentes';

  @override
  String targetsMore(int count) {
    return '+$count mais';
  }

  @override
  String get removeModAction => 'Remover';

  @override
  String removeModConfirm(String name) {
    return 'Remover “$name” da biblioteca?';
  }

  @override
  String get errorSetGamePath =>
      'Defina primeiro o caminho do jogo nas Configurações.';

  @override
  String applyReportApplied(int count) {
    return '$count mods aplicados.';
  }

  @override
  String get warningsTitle => 'Avisos';

  @override
  String get modDisabledHint => 'Desativado';

  @override
  String get kindGoremod => 'goremod';

  @override
  String get kindTriplet => 'triplet';

  @override
  String get kindPak => 'pak';

  @override
  String get kindUe4ss => 'UE4SS';

  @override
  String get kindRawfile => 'arquivo bruto';

  @override
  String get kindMixed => 'misto';

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
  String get about => 'Sobre';

  @override
  String get aboutCopyright => '© 2026 colaboradores do GORE';

  @override
  String get aboutLicense => 'Licenciado sob a Licença MIT.';

  @override
  String get appearanceTitle => 'Aparência';

  @override
  String get theme => 'Tema';

  @override
  String get themeLight => 'Claro';

  @override
  String get themeDark => 'Escuro';

  @override
  String get themeSystem => 'Sistema';

  @override
  String get uiScale => 'Escala da interface';

  @override
  String get resetZoomTooltip => 'Redefinir zoom (Ctrl+0)';

  @override
  String get zoomTip =>
      'Dica: Ctrl + / Ctrl - altera o zoom em qualquer parte do app.';

  @override
  String get lightMode => 'Modo claro';

  @override
  String get darkMode => 'Modo escuro';

  @override
  String get minimize => 'Minimizar';

  @override
  String get restore => 'Restaurar';

  @override
  String get maximize => 'Maximizar';

  @override
  String get close => 'Fechar';
}
