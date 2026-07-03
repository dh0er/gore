// ignore: unused_import
import 'package:intl/intl.dart' as intl;
import 'app_localizations.dart';

// ignore_for_file: type=lint

/// The translations for Chinese (`zh`).
class AppLocalizationsZh extends AppLocalizations {
  AppLocalizationsZh([String locale = 'zh']) : super(locale);

  @override
  String get appTitle => 'gore-manager';

  @override
  String get tabMods => '模组';

  @override
  String get tabSettings => '设置';

  @override
  String get settingsGameExe => '游戏可执行文件';

  @override
  String get settingsGameExePick => '选择…';

  @override
  String get settingsLanguage => '语言';

  @override
  String get statusInSync => '已同步';

  @override
  String get statusChangesPending => '有待应用的更改';

  @override
  String get statusGameUpdated => '游戏已更新';

  @override
  String get statusStudioDeploy => 'Studio 部署已激活';

  @override
  String get statusNothingDeployed => '尚未部署任何内容';

  @override
  String get actionImport => '导入';

  @override
  String get actionApply => '应用';

  @override
  String get actionUndeployAll => '撤销全部部署';

  @override
  String get commonCancel => '取消';

  @override
  String get commonOk => '确定';
}

/// The translations for Chinese, using the Han script (`zh_Hans`).
class AppLocalizationsZhHans extends AppLocalizationsZh {
  AppLocalizationsZhHans() : super('zh_Hans');

  @override
  String get appTitle => 'gore-manager';

  @override
  String get tabMods => '模组';

  @override
  String get tabSettings => '设置';

  @override
  String get settingsGameExe => '游戏可执行文件';

  @override
  String get settingsGameExePick => '选择…';

  @override
  String get settingsLanguage => '语言';

  @override
  String get statusInSync => '已同步';

  @override
  String get statusChangesPending => '有待应用的更改';

  @override
  String get statusGameUpdated => '游戏已更新';

  @override
  String get statusStudioDeploy => 'Studio 部署已激活';

  @override
  String get statusNothingDeployed => '尚未部署任何内容';

  @override
  String get actionImport => '导入';

  @override
  String get actionApply => '应用';

  @override
  String get actionUndeployAll => '撤销全部部署';

  @override
  String get commonCancel => '取消';

  @override
  String get commonOk => '确定';
}
