// ignore: unused_import
import 'package:intl/intl.dart' as intl;
import 'app_localizations.dart';

// ignore_for_file: type=lint

/// The translations for Spanish Castilian (`es`).
class AppLocalizationsEs extends AppLocalizations {
  AppLocalizationsEs([String locale = 'es']) : super(locale);

  @override
  String get appTitle => 'GORE Mod Manager';

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

  @override
  String get importFolder => 'Importar carpeta…';

  @override
  String get importFile => 'Importar archivo…';

  @override
  String get applyTooltip => 'Aplicar la configuración al juego';

  @override
  String get undeployAllAction => 'Retirar todo';

  @override
  String get undeployAllConfirm =>
      '¿Quitar del juego todo lo que desplegó el gestor?';

  @override
  String get takeOverTitle => 'Despliegue de Studio activo';

  @override
  String get takeOverBody =>
      'mod-studio ha desplegado un mod en el juego. ¿Tomar el control para que el gestor aplique esta configuración?';

  @override
  String get takeOverAction => 'Tomar el control';

  @override
  String get refreshAction => 'Actualizar';

  @override
  String conflictsTitle(int count) {
    return 'Conflictos ($count)';
  }

  @override
  String get conflictWinner => 'ganador';

  @override
  String get noConflicts => 'Sin conflictos.';

  @override
  String get componentsTitle => 'Componentes';

  @override
  String targetsMore(int count) {
    return '+$count más';
  }

  @override
  String get removeModAction => 'Quitar';

  @override
  String removeModConfirm(String name) {
    return '¿Quitar «$name» de la biblioteca?';
  }

  @override
  String get errorSetGamePath => 'Primero define la ruta del juego en Ajustes.';

  @override
  String applyReportApplied(int count) {
    return '$count mods aplicados.';
  }

  @override
  String get warningsTitle => 'Advertencias';

  @override
  String get modDisabledHint => 'Desactivado';

  @override
  String get kindGoremod => 'goremod';

  @override
  String get kindTriplet => 'triplet';

  @override
  String get kindPak => 'pak';

  @override
  String get kindUe4ss => 'UE4SS';

  @override
  String get kindRawfile => 'archivo sin procesar';

  @override
  String get kindMixed => 'mixto';

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
  String get about => 'Acerca de';

  @override
  String get aboutCopyright => '© 2026 colaboradores de GORE';

  @override
  String get aboutLicense => 'Distribuido bajo la licencia MIT.';

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
  String get lightMode => 'Modo claro';

  @override
  String get darkMode => 'Modo oscuro';

  @override
  String get minimize => 'Minimizar';

  @override
  String get restore => 'Restaurar';

  @override
  String get maximize => 'Maximizar';

  @override
  String get close => 'Cerrar';
}
