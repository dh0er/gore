// ignore: unused_import
import 'package:intl/intl.dart' as intl;
import 'app_localizations.dart';

// ignore_for_file: type=lint

/// The translations for Japanese (`ja`).
class AppLocalizationsJa extends AppLocalizations {
  AppLocalizationsJa([String locale = 'ja']) : super(locale);

  @override
  String get appTitle => 'gore-manager';

  @override
  String get tabMods => 'Mod';

  @override
  String get tabSettings => '設定';

  @override
  String get settingsGameExe => 'ゲーム実行ファイル';

  @override
  String get settingsGameExePick => '選択…';

  @override
  String get settingsLanguage => '言語';

  @override
  String get statusInSync => '同期済み';

  @override
  String get statusChangesPending => '変更が保留中';

  @override
  String get statusGameUpdated => 'ゲームが更新されました';

  @override
  String get statusStudioDeploy => 'Studio のデプロイが有効';

  @override
  String get statusNothingDeployed => '何もデプロイされていません';

  @override
  String get actionImport => 'インポート';

  @override
  String get actionApply => '適用';

  @override
  String get actionUndeployAll => 'すべて解除';

  @override
  String get commonCancel => 'キャンセル';

  @override
  String get commonOk => 'OK';
}
