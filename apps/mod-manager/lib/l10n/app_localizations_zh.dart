// ignore: unused_import
import 'package:intl/intl.dart' as intl;
import 'app_localizations.dart';

// ignore_for_file: type=lint

/// The translations for Chinese (`zh`).
class AppLocalizationsZh extends AppLocalizations {
  AppLocalizationsZh([String locale = 'zh']) : super(locale);

  @override
  String get coreBlockedTitle => 'Mod Manager 不可用';

  @override
  String get coreDllMissingMessage => '未找到所需的 gore_ffi.dll 文件。';

  @override
  String get coreDllLoadFailedMessage => '无法加载 GORE Core 原生库。';

  @override
  String get coreVerificationFailedMessage => '无法验证 GORE Core 原生库。';

  @override
  String get coreManagerTooOldMessage =>
      '此 GORE Core 版本比 Mod Manager 更新。请更新 Mod Manager。';

  @override
  String get coreNativeTooOldMessage =>
      '此 GORE Core 版本比 Mod Manager 更旧。请更新或修复完整的 Mod Manager 安装。';

  @override
  String get coreCommandsMissingMessage =>
      'GORE Core 库未提供此 Mod Manager 所需的全部命令。';

  @override
  String get coreBlockedRepairHint => '请更新或修复完整的 Mod Manager 软件包，然后重新启动应用。';

  @override
  String get coreTechnicalDetails => '技术详细信息';

  @override
  String get coreCopyTechnicalDetails => '复制技术详细信息';

  @override
  String get coreTechnicalDetailsCopied => '已复制技术详细信息';

  @override
  String get coreTechnicalDetailsCopyFailed => '无法复制技术详细信息。请重试。';

  @override
  String get preflightAttention => '设置需要处理。';

  @override
  String get preflightUnavailable => '设置诊断不可用。';

  @override
  String get preflightRetry => '重新检查';

  @override
  String get preflightReviewStatus => '检查状态';

  @override
  String get statusUnknown => '未知';

  @override
  String statusDetailsTitle(String status) {
    return '部署：$status';
  }

  @override
  String statusDetailsOpen(String status) {
    return '显示部署详情：$status';
  }

  @override
  String get statusDetailsNoRoot => '请在设置中选择游戏安装位置以查看部署状态。';

  @override
  String get statusDetailsNoDeployment => '此游戏没有管理器部署。';

  @override
  String get statusDetailsInSyncDescription => '已部署模组与当前配置一致。';

  @override
  String get statusDetailsDeployedLoadout => '已部署的加载顺序';

  @override
  String get statusDetailsChangesDescription => '当前部署与应用后将安装的内容不同。';

  @override
  String get statusDetailsCurrentlyDeployed => '当前部署';

  @override
  String get statusDetailsAfterApply => '应用后';

  @override
  String get statusDetailsGameUpdatedDescription =>
      '自上次部署后，游戏文件已更改。请重新应用配置以恢复管理器拥有的文件。';

  @override
  String get statusDetailsDriftedFiles => '已更改的文件';

  @override
  String get statusDetailsStudioDescription =>
      'Mod Studio 当前控制此游戏安装。应用管理器配置前请先接管。';

  @override
  String statusDetailsStudioMod(String name) {
    return 'Studio 模组：$name';
  }

  @override
  String get statusDetailsStudioNameUnknown => 'Studio 未报告模组名称。';

  @override
  String get statusDetailsRecoveryDescription => '部署被中断。应用或移除管理器模组前请先恢复。';

  @override
  String get statusDetailsUnknownDescription => '无法验证部署状态。应用模组前请刷新。';

  @override
  String get statusDetailsUnavailable => '已安装的核心未提供这些详情。';

  @override
  String get statusDetailsEmptyLoadout => '此配置中没有模组。';

  @override
  String get statusDetailsLastError => '上次错误';

  @override
  String get statusDetailsLastApply => '上次应用';

  @override
  String get statusDetailsAppliedMods => '已应用的模组';

  @override
  String get statusDetailsWarnings => '警告';

  @override
  String get statusDetailsReapply => '重新应用';

  @override
  String get statusDetailsOpenSettings => '打开设置';

  @override
  String get recoveryAction => '恢复';

  @override
  String get recoveryRequiredConfirm => '恢复中断的部署并移除已部分部署的文件吗？';

  @override
  String get statusRecoveryRequired => '需要恢复';

  @override
  String get statusDetailsOwnershipTitle => '已记录的所有权证据';

  @override
  String get statusDetailsOwnershipDescription =>
      '管理器部署记录中保存的路径。这不表示这些路径当前仍然存在。';

  @override
  String get statusDetailsOwnershipLive => '已替换的游戏文件';

  @override
  String get statusDetailsOwnershipBackups => '原始文件备份';

  @override
  String get statusDetailsOwnershipAdditive => '新增的 pak 和容器文件';

  @override
  String get statusDetailsOwnershipUe4ss => 'UE4SS 模组目录';

  @override
  String get statusDetailsOwnershipRecovery => '恢复文件和保留位置';

  @override
  String get statusDetailsOwnershipEmpty => '此组中没有记录的路径。';

  @override
  String statusDetailsOwnershipShown(int shown, int total) {
    return '显示了 $total 条已记录路径中的 $shown 条。';
  }

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
  String importOutcomeCreated(String name) {
    return '已将“$name”添加到库中。';
  }

  @override
  String importOutcomeUpdated(String name) {
    return '已更新库中的“$name”。';
  }

  @override
  String importOutcomeUnchanged(String name) {
    return '“$name”已在库中。';
  }

  @override
  String importOutcomeMatchedBy(String method) {
    String _temp0 = intl.Intl.selectLogic(method, {
      'none': '未匹配到现有库条目。',
      'source': '根据相同的导入来源匹配。',
      'content': '根据经验证相同的内容匹配。',
      'entry_id': '根据模组 ID 匹配。',
      'other': '匹配详情不可用。',
    });
    return '$_temp0';
  }

  @override
  String get importRefusalDuplicateAmbiguous => '此导入匹配多个库条目。请检查或移除重复条目，然后重试。';

  @override
  String get importRefusalIdentityConflict =>
      '导入来源及其内容匹配到不同的库条目。请检查或移除冲突条目，然后重试。';

  @override
  String get importFailed =>
      '无法完成导入。支持的来源：文件夹、ZIP、独立的 *_P.pak、完整的 .utoc/.ucas 组合（.pak 可选）、.lcache、.bank 和 PrecompiledScript*.Cache。请先解压 .7z 或 .rar，再导入文件夹。来源可能不受支持、已损坏或不完整。模组可能已被添加或更新；请刷新并检查库状态，然后重试。';

  @override
  String get importPickerFailed => '无法打开文件或文件夹选择器。导入尚未开始。请重试。';

  @override
  String get importOutcomeUnknown => '无法验证导入结果。请选择“刷新”以检查库状态。';

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
    return '检测结果 ($count)';
  }

  @override
  String get conflictWinner => '预期生效';

  @override
  String get noConflicts => '未识别到冲突。';

  @override
  String get conflictCoverageIncomplete => '已启用模组的冲突信息不完整，可能还存在其他冲突。';

  @override
  String get loadOrderDirection => '加载顺序：低优先级在前，后面的模组具有更高的预期优先级。';

  @override
  String get footprintCoverageScope => '覆盖度仅描述已识别的冲突目标，不能证明运行时优先级。';

  @override
  String get footprintCoverageExact => '精确 — 组件的冲突目标列表完整。';

  @override
  String get footprintCoveragePartial => '部分 — 已列出的冲突目标是已知的，但组件可能影响更多目标。';

  @override
  String get footprintCoverageAdvisory => '参考 — 已列出的目标只是线索，并非完整证明。';

  @override
  String get footprintCoverageOpaque => '不透明 — 组件的冲突目标未知。';

  @override
  String get footprintCoverageExactLabel => '精确';

  @override
  String get footprintCoveragePartialLabel => '部分';

  @override
  String get footprintCoverageAdvisoryLabel => '参考';

  @override
  String get footprintCoverageOpaqueLabel => '不透明';

  @override
  String get conflictsUnverified => '在刷新库状态之前，冲突尚未验证。';

  @override
  String get componentsTitle => '组件';

  @override
  String targetsMore(int count) {
    return '还有 $count 项';
  }

  @override
  String get removeModDeploymentHint =>
      '从库中移除不会立即更改现有部署。如果该模组已部署，请随后选择“应用”以更新游戏安装。';

  @override
  String removeModSuccess(String name) {
    return '已从库中移除“$name”。';
  }

  @override
  String removeModFailed(String name, String error) {
    return '无法移除“$name”：$error';
  }

  @override
  String removeModPartialFailure(String name, String error) {
    return '已移除“$name”，但后续处理报告了错误。库状态已重新加载：$error';
  }

  @override
  String removeModOutcomeUnknown(String name, String error) {
    return '无法验证是否已移除“$name”：$error；请刷新以检查库状态。';
  }

  @override
  String get libraryStateUnknown => '无法验证库状态。请在更改或应用模组前选择“刷新”。';

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
  String get coreBlockedTitle => 'Mod Manager 不可用';

  @override
  String get coreDllMissingMessage => '未找到所需的 gore_ffi.dll 文件。';

  @override
  String get coreDllLoadFailedMessage => '无法加载 GORE Core 原生库。';

  @override
  String get coreVerificationFailedMessage => '无法验证 GORE Core 原生库。';

  @override
  String get coreManagerTooOldMessage =>
      '此 GORE Core 版本比 Mod Manager 更新。请更新 Mod Manager。';

  @override
  String get coreNativeTooOldMessage =>
      '此 GORE Core 版本比 Mod Manager 更旧。请更新或修复完整的 Mod Manager 安装。';

  @override
  String get coreCommandsMissingMessage =>
      'GORE Core 库未提供此 Mod Manager 所需的全部命令。';

  @override
  String get coreBlockedRepairHint => '请更新或修复完整的 Mod Manager 软件包，然后重新启动应用。';

  @override
  String get coreTechnicalDetails => '技术详细信息';

  @override
  String get coreCopyTechnicalDetails => '复制技术详细信息';

  @override
  String get coreTechnicalDetailsCopied => '已复制技术详细信息';

  @override
  String get coreTechnicalDetailsCopyFailed => '无法复制技术详细信息。请重试。';

  @override
  String get preflightAttention => '设置需要处理。';

  @override
  String get preflightUnavailable => '设置诊断不可用。';

  @override
  String get preflightRetry => '重新检查';

  @override
  String get preflightReviewStatus => '检查状态';

  @override
  String get statusUnknown => '未知';

  @override
  String statusDetailsTitle(String status) {
    return '部署：$status';
  }

  @override
  String statusDetailsOpen(String status) {
    return '显示部署详情：$status';
  }

  @override
  String get statusDetailsNoRoot => '请在设置中选择游戏安装位置以查看部署状态。';

  @override
  String get statusDetailsNoDeployment => '此游戏没有管理器部署。';

  @override
  String get statusDetailsInSyncDescription => '已部署模组与当前配置一致。';

  @override
  String get statusDetailsDeployedLoadout => '已部署的加载顺序';

  @override
  String get statusDetailsChangesDescription => '当前部署与应用后将安装的内容不同。';

  @override
  String get statusDetailsCurrentlyDeployed => '当前部署';

  @override
  String get statusDetailsAfterApply => '应用后';

  @override
  String get statusDetailsGameUpdatedDescription =>
      '自上次部署后，游戏文件已更改。请重新应用配置以恢复管理器拥有的文件。';

  @override
  String get statusDetailsDriftedFiles => '已更改的文件';

  @override
  String get statusDetailsStudioDescription =>
      'Mod Studio 当前控制此游戏安装。应用管理器配置前请先接管。';

  @override
  String statusDetailsStudioMod(String name) {
    return 'Studio 模组：$name';
  }

  @override
  String get statusDetailsStudioNameUnknown => 'Studio 未报告模组名称。';

  @override
  String get statusDetailsRecoveryDescription => '部署被中断。应用或移除管理器模组前请先恢复。';

  @override
  String get statusDetailsUnknownDescription => '无法验证部署状态。应用模组前请刷新。';

  @override
  String get statusDetailsUnavailable => '已安装的核心未提供这些详情。';

  @override
  String get statusDetailsEmptyLoadout => '此配置中没有模组。';

  @override
  String get statusDetailsLastError => '上次错误';

  @override
  String get statusDetailsLastApply => '上次应用';

  @override
  String get statusDetailsAppliedMods => '已应用的模组';

  @override
  String get statusDetailsWarnings => '警告';

  @override
  String get statusDetailsReapply => '重新应用';

  @override
  String get statusDetailsOpenSettings => '打开设置';

  @override
  String get recoveryAction => '恢复';

  @override
  String get recoveryRequiredConfirm => '恢复中断的部署并移除已部分部署的文件吗？';

  @override
  String get statusRecoveryRequired => '需要恢复';

  @override
  String get statusDetailsOwnershipTitle => '已记录的所有权证据';

  @override
  String get statusDetailsOwnershipDescription =>
      '管理器部署记录中保存的路径。这不表示这些路径当前仍然存在。';

  @override
  String get statusDetailsOwnershipLive => '已替换的游戏文件';

  @override
  String get statusDetailsOwnershipBackups => '原始文件备份';

  @override
  String get statusDetailsOwnershipAdditive => '新增的 pak 和容器文件';

  @override
  String get statusDetailsOwnershipUe4ss => 'UE4SS 模组目录';

  @override
  String get statusDetailsOwnershipRecovery => '恢复文件和保留位置';

  @override
  String get statusDetailsOwnershipEmpty => '此组中没有记录的路径。';

  @override
  String statusDetailsOwnershipShown(int shown, int total) {
    return '显示了 $total 条已记录路径中的 $shown 条。';
  }

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
  String importOutcomeCreated(String name) {
    return '已将“$name”添加到库中。';
  }

  @override
  String importOutcomeUpdated(String name) {
    return '已更新库中的“$name”。';
  }

  @override
  String importOutcomeUnchanged(String name) {
    return '“$name”已在库中。';
  }

  @override
  String importOutcomeMatchedBy(String method) {
    String _temp0 = intl.Intl.selectLogic(method, {
      'none': '未匹配到现有库条目。',
      'source': '根据相同的导入来源匹配。',
      'content': '根据经验证相同的内容匹配。',
      'entry_id': '根据模组 ID 匹配。',
      'other': '匹配详情不可用。',
    });
    return '$_temp0';
  }

  @override
  String get importRefusalDuplicateAmbiguous => '此导入匹配多个库条目。请检查或移除重复条目，然后重试。';

  @override
  String get importRefusalIdentityConflict =>
      '导入来源及其内容匹配到不同的库条目。请检查或移除冲突条目，然后重试。';

  @override
  String get importFailed =>
      '无法完成导入。支持的来源：文件夹、ZIP、独立的 *_P.pak、完整的 .utoc/.ucas 组合（.pak 可选）、.lcache、.bank 和 PrecompiledScript*.Cache。请先解压 .7z 或 .rar，再导入文件夹。来源可能不受支持、已损坏或不完整。模组可能已被添加或更新；请刷新并检查库状态，然后重试。';

  @override
  String get importPickerFailed => '无法打开文件或文件夹选择器。导入尚未开始。请重试。';

  @override
  String get importOutcomeUnknown => '无法验证导入结果。请选择“刷新”以检查库状态。';

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
    return '检测结果 ($count)';
  }

  @override
  String get conflictWinner => '预期生效';

  @override
  String get noConflicts => '未识别到冲突。';

  @override
  String get conflictCoverageIncomplete => '已启用模组的冲突信息不完整，可能还存在其他冲突。';

  @override
  String get loadOrderDirection => '加载顺序：低优先级在前，后面的模组具有更高的预期优先级。';

  @override
  String get footprintCoverageScope => '覆盖度仅描述已识别的冲突目标，不能证明运行时优先级。';

  @override
  String get footprintCoverageExact => '精确 — 组件的冲突目标列表完整。';

  @override
  String get footprintCoveragePartial => '部分 — 已列出的冲突目标是已知的，但组件可能影响更多目标。';

  @override
  String get footprintCoverageAdvisory => '参考 — 已列出的目标只是线索，并非完整证明。';

  @override
  String get footprintCoverageOpaque => '不透明 — 组件的冲突目标未知。';

  @override
  String get footprintCoverageExactLabel => '精确';

  @override
  String get footprintCoveragePartialLabel => '部分';

  @override
  String get footprintCoverageAdvisoryLabel => '参考';

  @override
  String get footprintCoverageOpaqueLabel => '不透明';

  @override
  String get conflictsUnverified => '在刷新库状态之前，冲突尚未验证。';

  @override
  String get componentsTitle => '组件';

  @override
  String targetsMore(int count) {
    return '还有 $count 项';
  }

  @override
  String get removeModDeploymentHint =>
      '从库中移除不会立即更改现有部署。如果该模组已部署，请随后选择“应用”以更新游戏安装。';

  @override
  String removeModSuccess(String name) {
    return '已从库中移除“$name”。';
  }

  @override
  String removeModFailed(String name, String error) {
    return '无法移除“$name”：$error';
  }

  @override
  String removeModPartialFailure(String name, String error) {
    return '已移除“$name”，但后续处理报告了错误。库状态已重新加载：$error';
  }

  @override
  String removeModOutcomeUnknown(String name, String error) {
    return '无法验证是否已移除“$name”：$error；请刷新以检查库状态。';
  }

  @override
  String get libraryStateUnknown => '无法验证库状态。请在更改或应用模组前选择“刷新”。';

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
