// ignore: unused_import
import 'package:intl/intl.dart' as intl;
import 'app_localizations.dart';

// ignore_for_file: type=lint

/// The translations for English (`en`).
class AppLocalizationsEn extends AppLocalizations {
  AppLocalizationsEn([String locale = 'en']) : super(locale);

  @override
  String get appTitle => 'gore-manager';

  @override
  String get tabMods => 'Mods';

  @override
  String get tabSettings => 'Settings';

  @override
  String get settingsGameExe => 'Game executable';

  @override
  String get settingsGameExePick => 'Choose…';

  @override
  String get settingsLanguage => 'Language';

  @override
  String get statusInSync => 'In sync';

  @override
  String get statusChangesPending => 'Changes pending';

  @override
  String get statusGameUpdated => 'Game updated';

  @override
  String get statusStudioDeploy => 'Studio deployment active';

  @override
  String get statusNothingDeployed => 'Nothing deployed';

  @override
  String get actionImport => 'Import';

  @override
  String get actionApply => 'Apply';

  @override
  String get actionUndeployAll => 'Undeploy all';

  @override
  String get commonCancel => 'Cancel';

  @override
  String get commonOk => 'OK';
}
