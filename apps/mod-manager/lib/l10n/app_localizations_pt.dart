// ignore: unused_import
import 'package:intl/intl.dart' as intl;
import 'app_localizations.dart';

// ignore_for_file: type=lint

/// The translations for Portuguese (`pt`).
class AppLocalizationsPt extends AppLocalizations {
  AppLocalizationsPt([String locale = 'pt']) : super(locale);

  @override
  String get appTitle => 'gore-manager';

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
}

/// The translations for Portuguese, as used in Brazil (`pt_BR`).
class AppLocalizationsPtBr extends AppLocalizationsPt {
  AppLocalizationsPtBr() : super('pt_BR');

  @override
  String get appTitle => 'gore-manager';

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
}
