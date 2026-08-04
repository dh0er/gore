// ignore: unused_import
import 'package:intl/intl.dart' as intl;
import 'app_localizations.dart';

// ignore_for_file: type=lint

/// The translations for Chinese (`zh`).
class AppLocalizationsZh extends AppLocalizations {
  AppLocalizationsZh([String locale = 'zh']) : super(locale);

  @override
  String get recoveryAction => '恢复';

  @override
  String get recoveryRequiredConfirm => '恢复中断的部署并移除已部分部署的文件吗？';

  @override
  String get statusRecoveryRequired => '需要恢复';

  @override
  String get appTitle => 'GORE Mod Manager';

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

  @override
  String get importFolder => '导入文件夹…';

  @override
  String get importFile => '导入文件…';

  @override
  String get applyTooltip => '将模组配置应用到游戏';

  @override
  String get undeployAllAction => '撤销全部部署';

  @override
  String get undeployAllConfirm => '从游戏中移除管理器部署的全部内容？';

  @override
  String get takeOverTitle => 'Studio 部署已激活';

  @override
  String get takeOverBody => 'mod-studio 已向游戏部署了一个模组。是否接管以便管理器应用此配置？';

  @override
  String get takeOverAction => '接管';

  @override
  String get refreshAction => '刷新';

  @override
  String conflictsTitle(int count) {
    return '冲突 ($count)';
  }

  @override
  String get conflictWinner => '生效';

  @override
  String get noConflicts => '无冲突。';

  @override
  String get componentsTitle => '组件';

  @override
  String targetsMore(int count) {
    return '还有 $count 项';
  }

  @override
  String get removeModAction => '移除';

  @override
  String removeModConfirm(String name) {
    return '从库中移除“$name”？';
  }

  @override
  String get errorSetGamePath => '请先在设置中指定游戏路径。';

  @override
  String applyReportApplied(int count) {
    return '已应用 $count 个模组。';
  }

  @override
  String get warningsTitle => '警告';

  @override
  String get modDisabledHint => '已禁用';

  @override
  String get kindGoremod => 'goremod';

  @override
  String get kindTriplet => 'triplet';

  @override
  String get kindPak => 'pak';

  @override
  String get kindUe4ss => 'UE4SS';

  @override
  String get kindRawfile => '原始文件';

  @override
  String get kindMixed => '混合';

  @override
  String get sevHard => '严重';

  @override
  String get sevSoft => '轻微';

  @override
  String get sevInfo => '信息';

  @override
  String aboutVersion(String version, String sha) {
    return 'Version $version ($sha)';
  }

  @override
  String get about => '关于';

  @override
  String get aboutCopyright => '© 2026 GORE 贡献者';

  @override
  String get aboutLicense => '基于 MIT 许可证授权。';

  @override
  String get appearanceTitle => '外观';

  @override
  String get theme => '主题';

  @override
  String get themeLight => '浅色';

  @override
  String get themeDark => '深色';

  @override
  String get themeSystem => '跟随系统';

  @override
  String get uiScale => '界面缩放';

  @override
  String get resetZoomTooltip => '重置缩放（Ctrl+0）';

  @override
  String get zoomTip => '提示：在应用内任意位置按 Ctrl + / Ctrl - 均可调整缩放。';

  @override
  String get lightMode => '浅色模式';

  @override
  String get darkMode => '深色模式';

  @override
  String get minimize => '最小化';

  @override
  String get restore => '还原';

  @override
  String get maximize => '最大化';

  @override
  String get close => '关闭';
}

/// The translations for Chinese, using the Han script (`zh_Hans`).
class AppLocalizationsZhHans extends AppLocalizationsZh {
  AppLocalizationsZhHans() : super('zh_Hans');

  @override
  String get recoveryAction => '恢复';

  @override
  String get recoveryRequiredConfirm => '恢复中断的部署并移除已部分部署的文件吗？';

  @override
  String get statusRecoveryRequired => '需要恢复';

  @override
  String get appTitle => 'GORE Mod Manager';

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

  @override
  String get importFolder => '导入文件夹…';

  @override
  String get importFile => '导入文件…';

  @override
  String get applyTooltip => '将模组配置应用到游戏';

  @override
  String get undeployAllAction => '撤销全部部署';

  @override
  String get undeployAllConfirm => '从游戏中移除管理器部署的全部内容？';

  @override
  String get takeOverTitle => 'Studio 部署已激活';

  @override
  String get takeOverBody => 'mod-studio 已向游戏部署了一个模组。是否接管以便管理器应用此配置？';

  @override
  String get takeOverAction => '接管';

  @override
  String get refreshAction => '刷新';

  @override
  String conflictsTitle(int count) {
    return '冲突 ($count)';
  }

  @override
  String get conflictWinner => '生效';

  @override
  String get noConflicts => '无冲突。';

  @override
  String get componentsTitle => '组件';

  @override
  String targetsMore(int count) {
    return '还有 $count 项';
  }

  @override
  String get removeModAction => '移除';

  @override
  String removeModConfirm(String name) {
    return '从库中移除“$name”？';
  }

  @override
  String get errorSetGamePath => '请先在设置中指定游戏路径。';

  @override
  String applyReportApplied(int count) {
    return '已应用 $count 个模组。';
  }

  @override
  String get warningsTitle => '警告';

  @override
  String get modDisabledHint => '已禁用';

  @override
  String get kindGoremod => 'goremod';

  @override
  String get kindTriplet => 'triplet';

  @override
  String get kindPak => 'pak';

  @override
  String get kindUe4ss => 'UE4SS';

  @override
  String get kindRawfile => '原始文件';

  @override
  String get kindMixed => '混合';

  @override
  String get sevHard => '严重';

  @override
  String get sevSoft => '轻微';

  @override
  String get sevInfo => '信息';

  @override
  String aboutVersion(String version, String sha) {
    return 'Version $version ($sha)';
  }

  @override
  String get about => '关于';

  @override
  String get aboutCopyright => '© 2026 GORE 贡献者';

  @override
  String get aboutLicense => '基于 MIT 许可证授权。';

  @override
  String get appearanceTitle => '外观';

  @override
  String get theme => '主题';

  @override
  String get themeLight => '浅色';

  @override
  String get themeDark => '深色';

  @override
  String get themeSystem => '跟随系统';

  @override
  String get uiScale => '界面缩放';

  @override
  String get resetZoomTooltip => '重置缩放（Ctrl+0）';

  @override
  String get zoomTip => '提示：在应用内任意位置按 Ctrl + / Ctrl - 均可调整缩放。';

  @override
  String get lightMode => '浅色模式';

  @override
  String get darkMode => '深色模式';

  @override
  String get minimize => '最小化';

  @override
  String get restore => '还原';

  @override
  String get maximize => '最大化';

  @override
  String get close => '关闭';
}
