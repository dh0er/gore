// ignore: unused_import
import 'package:intl/intl.dart' as intl;
import 'app_localizations.dart';

// ignore_for_file: type=lint

/// The translations for Spanish Castilian (`es`).
class AppLocalizationsEs extends AppLocalizations {
  AppLocalizationsEs([String locale = 'es']) : super(locale);

  @override
  String get appTitle => 'gore-manager';

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
}
