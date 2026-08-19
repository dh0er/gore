// ignore: unused_import
import 'package:intl/intl.dart' as intl;
import 'app_localizations.dart';

// ignore_for_file: type=lint

/// The translations for Chinese (`zh`).
class AppLocalizationsZh extends AppLocalizations {
  AppLocalizationsZh([String locale = 'zh']) : super(locale);

  @override
  String get coreBlockedTitle => 'Mod Manager 无法启动';

  @override
  String get coreDllMissingMessage => '缺少必需的程序文件 (gore_ffi.dll)。';

  @override
  String get coreDllLoadFailedMessage => '无法加载必需的程序文件。';

  @override
  String get coreVerificationFailedMessage => '无法校验必需的程序文件。';

  @override
  String get coreManagerTooOldMessage =>
      '程序文件比 Mod Manager 更新。请更新 Mod Manager。';

  @override
  String get coreNativeTooOldMessage =>
      '程序文件比 Mod Manager 更旧。请重新安装 Mod Manager。';

  @override
  String get coreCommandsMissingMessage => '程序文件缺少此 Mod Manager 需要的功能。';

  @override
  String get coreBlockedRepairHint => '请重新安装或修复 Mod Manager，然后再次启动。';

  @override
  String get coreTechnicalDetails => '技术详细信息';

  @override
  String get coreCopyTechnicalDetails => '复制技术详细信息';

  @override
  String get coreTechnicalDetailsCopied => '已复制技术详细信息';

  @override
  String get coreTechnicalDetailsCopyFailed => '无法复制技术详细信息。请重试。';

  @override
  String get preflightAttention => '更改模组前，还有一件事需要处理。';

  @override
  String get preflightGameRunning => 'Gothic 仍在运行。请先关闭游戏，再更改模组。';

  @override
  String get managerOperationFailed => '操作失败。';

  @override
  String get libraryOperationFailed => '无法加载模组列表。';

  @override
  String get conflictsUnavailable => '无法检查冲突。';

  @override
  String applyReportAppliedWithWarnings(int applied, int warnings) {
    return '已应用：$applied。警告：$warnings。';
  }

  @override
  String get modDetailKind => '类型';

  @override
  String get modDetailVersion => '版本';

  @override
  String get modDetailAuthor => '作者';

  @override
  String get modDetailSource => '来源';

  @override
  String get modDetailImported => '导入时间';

  @override
  String get componentLocalization => '文本';

  @override
  String get componentAudio => '音效';

  @override
  String get componentAngelScript => '脚本';

  @override
  String get componentTexture => '贴图';

  @override
  String get componentGameFiles => '游戏文件';

  @override
  String get componentVoice => '配音';

  @override
  String get componentKindLocalizationPatch => '文本改动';

  @override
  String get componentKindAudioPatch => '音效改动';

  @override
  String get componentKindAngelScriptPatch => '脚本改动';

  @override
  String get componentKindTexturePatch => '贴图改动';

  @override
  String get componentKindLoosePak => 'PAK 文件';

  @override
  String get componentKindTriplet => 'IoStore 容器';

  @override
  String get componentKindUe4ssLua => 'UE4SS 脚本';

  @override
  String get componentKindRawFile => '文件';

  @override
  String get componentKindFilePatch => '被替换的游戏文件';

  @override
  String get componentKindPakFilePatch => '来自 ~mods PAK 的游戏文件';

  @override
  String get componentKindVoiceArchivePatch => '配音';

  @override
  String get rawTargetGameText => '全部游戏文本';

  @override
  String get rawTargetGameScripts => '全部游戏脚本';

  @override
  String get rawTargetSoundBank => '音效库';

  @override
  String rawTargetSoundBankNamed(String name) {
    return '音效库：$name';
  }

  @override
  String get conflictKindLocalization => '文本';

  @override
  String get conflictKindAudio => '音效';

  @override
  String get conflictKindAsset => '游戏数据';

  @override
  String get conflictKindCdo => '对象数值';

  @override
  String get conflictKindUe4ssUnknown => 'UE4SS（不明）';

  @override
  String get conflictKindScriptModule => '游戏脚本';

  @override
  String get conflictKindVoiceArchive => '配音';

  @override
  String get conflictKindRawFile => '文件';

  @override
  String get conflictKindLooseFile => '游戏文件';

  @override
  String get preflightUnavailable => '无法检查游戏安装。';

  @override
  String get preflightRetry => '重新检查';

  @override
  String get preflightReviewStatus => '查看状态';

  @override
  String get preflightReviewRecovery => '查看帮助';

  @override
  String get installRecoveryTitle => '被中断的安装';

  @override
  String get installRecoveryBody =>
      'GORE 发现了一次安装或脚本编译留下的数据。该任务可能仍在运行，也可能已经结束并留下了这些数据。GORE 无法安全地自行清理。';

  @override
  String get installRecoverySteps =>
      '如果任务仍在运行，请等它结束——不要终止它，也不要删除任何文件。确认没有任务在运行后，按下面文件夹中的 README.txt 操作，然后重新检查。如果没有列出文件夹或你不确定，请保持原样并寻求帮助。';

  @override
  String get installRecoveryEvidence => 'GORE 发现的内容';

  @override
  String get managerRecoveryTitle => '修复被中断的更改';

  @override
  String get managerRecoveryConfirm =>
      'GORE 发现了一次被中断的更改，可以把游戏恢复到已知状态。你的存档绝不会被改动。';

  @override
  String get managerRecoveryAlreadyClean => '没有需要修复的内容。已重新检查状态。';

  @override
  String get managerRecoveryBusy => '任务又在运行了。未做任何更改，请等它结束。';

  @override
  String get managerRecoveryLockCleared => '被中断的任务尚未改动任何内容，已清理完毕。';

  @override
  String get managerRecoveryRestoredPristine => '更改已回滚，游戏已恢复到之前的状态。';

  @override
  String get managerRecoveryApplyPreserved => '应用已经完成，没有丢失任何内容。';

  @override
  String get managerRecoveryUndeployConfirmed => '移除已经完成，残留内容已清理。';

  @override
  String get managerRecoveryCompileRequired => '这属于一次脚本编译，因此未做任何更改。请打开修复帮助。';

  @override
  String get managerRecoveryInspectionFailed => 'GORE 无法安全检查被中断的任务，未做任何更改。';

  @override
  String get managerRecoveryFailed => '修复未能完成。请先查看状态再重试。';

  @override
  String get statusUnknown => '未知';

  @override
  String statusDetailsTitle(String status) {
    return '状态：$status';
  }

  @override
  String statusDetailsOpen(String status) {
    return '显示详情：$status';
  }

  @override
  String get statusDetailsNoRoot => '请先在设置中选择你的 Gothic 安装目录。';

  @override
  String get statusDetailsNoDeployment => '游戏中当前没有安装任何模组。';

  @override
  String get statusDetailsInSyncDescription => '游戏中的模组与此处勾选的完全一致。';

  @override
  String get statusDetailsDeployedLoadout => '游戏中的模组';

  @override
  String get statusDetailsChangesDescription => '你的选择与游戏中的内容不一致。';

  @override
  String get statusDetailsCurrentlyDeployed => '当前游戏中';

  @override
  String get statusDetailsAfterApply => '应用之后';

  @override
  String get statusDetailsGameUpdatedDescription => '游戏更新覆盖了模组文件。请再次应用以恢复。';

  @override
  String get statusDetailsDriftedFiles => '受影响的文件';

  @override
  String get statusDetailsStudioDescription =>
      'Mod Studio 目前在这个游戏里放了模组。请先接管游戏，再让管理器应用你的模组。';

  @override
  String statusDetailsStudioMod(String name) {
    return 'Studio 模组：$name';
  }

  @override
  String get statusDetailsStudioNameUnknown => 'Mod Studio 未提供名称。';

  @override
  String get statusDetailsRecoveryDescription => '有一次更改被中断。请先修复，再更改模组。';

  @override
  String get statusDetailsUnknownDescription => '无法读取状态。请先刷新。';

  @override
  String get statusDetailsUnavailable => '没有可用的详情。';

  @override
  String get statusDetailsEmptyLoadout => '没有模组。';

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
  String get recoveryAction => '修复';

  @override
  String get recoveryRequiredConfirm => '修复被中断的更改并删除安装了一半的文件？';

  @override
  String get statusRecoveryRequired => '需要修复';

  @override
  String get statusDetailsOwnershipTitle => 'GORE 管理的文件';

  @override
  String get statusDetailsOwnershipDescription => '在应用模组时记录，并不代表这些文件现在仍然存在。';

  @override
  String get statusDetailsOwnershipLive => '已替换的游戏文件';

  @override
  String get statusDetailsOwnershipBackups => '原始文件的备份';

  @override
  String get statusDetailsOwnershipAdditive => '新增的模组文件';

  @override
  String get statusDetailsOwnershipUe4ss => 'UE4SS 模组目录';

  @override
  String get statusDetailsOwnershipRecovery => '修复文件';

  @override
  String get statusDetailsOwnershipEmpty => '这里没有记录。';

  @override
  String statusDetailsOwnershipShown(int shown, int total) {
    return '显示 $total 条路径中的 $shown 条。';
  }

  @override
  String get appTitle => 'GORE Mod Manager';

  @override
  String get tabMods => '模组';

  @override
  String get tabSettings => '设置';

  @override
  String get settingsGameExe => 'Gothic 安装目录';

  @override
  String get settingsGameExePick => '选择…';

  @override
  String get settingsLanguage => '语言';

  @override
  String get libraryEmptyTitle => '还没有模组';

  @override
  String get libraryEmptyBody => '导入一个文件夹或模组文件即可开始。';

  @override
  String get detailEmptyHint => '选择一个模组，查看它会改动什么。';

  @override
  String get settingsAdvanced => '高级详情';

  @override
  String get settingsAdvancedHint => '显示技术信息：受影响的条目、冲突检查的可靠程度，以及 GORE 管理的文件。';

  @override
  String get updatesTitle => '更新';

  @override
  String get checkForUpdatesAutomatically => '自动检查更新';

  @override
  String get checkForUpdatesNow => '立即检查更新';

  @override
  String get updatesPortableNotice => '便携版会在浏览器中打开下载页面。请用新下载的文件替换现有文件。';

  @override
  String get updateCheckFailed => '无法检查更新，请稍后再试。';

  @override
  String get updateUpToDate => '你正在使用最新版本。';

  @override
  String get updateAvailableTitle => '有可用更新';

  @override
  String updateAvailableMessage(String version, String current) {
    return '版本 $version 可用，你当前是 $current。';
  }

  @override
  String get updateLater => '稍后';

  @override
  String get updateDownload => '下载';

  @override
  String get statusInSync => '已是最新';

  @override
  String get statusChangesPending => '尚未应用';

  @override
  String get statusGameUpdated => '游戏已更新';

  @override
  String get statusStudioDeploy => 'Mod Studio 正在使用';

  @override
  String get statusNothingDeployed => '游戏中没有模组';

  @override
  String get actionImport => '导入';

  @override
  String get actionApply => '应用';

  @override
  String get actionStartGame => '启动游戏';

  @override
  String get startGameTooltip => '用当前游戏中的模组启动 Gothic';

  @override
  String get startGameFailed => '无法启动 Gothic。请在设置中检查游戏安装。';

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
    return '已添加“$name”。';
  }

  @override
  String importOutcomeUpdated(String name) {
    return '已更新“$name”。';
  }

  @override
  String importOutcomeUnchanged(String name) {
    return '“$name”已在你的列表中。';
  }

  @override
  String importOutcomeMatchedBy(String method) {
    String _temp0 = intl.Intl.selectLogic(method, {
      'none': '没有匹配到已有模组。',
      'source': '按相同的导入来源匹配。',
      'content': '按已验证的相同内容匹配。',
      'entry_id': '按模组 ID 匹配。',
      'other': '没有匹配详情。',
    });
    return '$_temp0';
  }

  @override
  String get importRefusalDuplicateAmbiguous => '这与你已有的多个模组重复。请删除重复项后重试。';

  @override
  String get importRefusalIdentityConflict => '来源和内容分别对应你已有的两个不同模组。请先理清后重试。';

  @override
  String get importFailed =>
      '无法导入。支持文件夹、ZIP 压缩包和单个模组文件（*_P.pak、.utoc/.ucas、.lcache、.bank、PrecompiledScript*.Cache）。请先解压 .7z 或 .rar，再导入文件夹。该模组仍可能已被添加或更新——请先刷新列表再重试。';

  @override
  String get importPickerFailed => '无法打开文件选择器，未导入任何内容。';

  @override
  String get importOutcomeUnknown => '结果不明确。请刷新以检查你的模组列表。';

  @override
  String get applyTooltip => '把勾选的模组装进游戏';

  @override
  String get undeployAllAction => '从游戏中移除全部';

  @override
  String get undeployAllConfirm => '从游戏中移除管理器安装的所有模组？';

  @override
  String get takeOverTitle => 'Mod Studio 正在使用';

  @override
  String get takeOverBody => 'Mod Studio 目前在游戏里放了一个模组。接管后由管理器应用你的选择？';

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
  String get noConflicts => '未发现冲突。';

  @override
  String get conflictCoverageIncomplete => '部分模组无法完全检查，可能还有其他冲突。';

  @override
  String get loadOrderDirection => '列表中靠下的模组会覆盖靠上的模组。';

  @override
  String get footprintCoverageScope => '仅列出已知的冲突目标，不保证游戏中的实际结果。';

  @override
  String get footprintTargetsExact => '受影响的条目 — 完整列表：';

  @override
  String get footprintTargetsPartial => '受影响的条目 — 可能还有更多：';

  @override
  String get footprintTargetsAdvisory => '可能受影响的条目 — 只是线索，并非确证：';

  @override
  String get footprintTargetsOpaque => 'GORE 无法判断这里改动了什么。';

  @override
  String get conflictsUnverified => '冲突未知，请先刷新。';

  @override
  String get componentsTitle => '这个模组会改动什么';

  @override
  String targetsMore(int count) {
    return '还有 $count 项';
  }

  @override
  String get removeModDeploymentHint => '这只会把它从你的列表中移除。如果它已装进游戏，请随后选择“应用”。';

  @override
  String removeModSuccess(String name) {
    return '已移除“$name”。';
  }

  @override
  String removeModFailed(String name) {
    return '无法移除“$name”。';
  }

  @override
  String removeModPartialFailure(String name) {
    return '已移除“$name”，但列表未能完全更新。';
  }

  @override
  String removeModOutcomeUnknown(String name) {
    return '无法确认“$name”是否已被移除。';
  }

  @override
  String get libraryStateUnknown => '模组列表已过期。请先刷新，再更改或应用模组。';

  @override
  String get removeModAction => '移除';

  @override
  String removeModConfirm(String name) {
    return '从你的列表中移除“$name”？';
  }

  @override
  String get errorSetGamePath => '请先在设置中选择你的 Gothic 安装目录。';

  @override
  String applyReportApplied(int count) {
    return '已应用 $count 个模组。';
  }

  @override
  String get modDisabledHint => '已禁用';

  @override
  String get kindGoremod => 'GORE 包';

  @override
  String get kindTriplet => 'IoStore 模组';

  @override
  String get kindPak => 'PAK 模组';

  @override
  String get kindUe4ss => 'UE4SS';

  @override
  String get kindRawfile => '整文件替换';

  @override
  String get kindMixed => '混合';

  @override
  String get sevHard => '冲突';

  @override
  String get sevSoft => '警告';

  @override
  String get sevInfo => '提示';

  @override
  String aboutVersion(String version, String sha) {
    return 'Version $version ($sha)';
  }

  @override
  String get about => '关于';

  @override
  String get aboutCopyright => '© 2026 Daniel Hoer';

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
  String get uiScale => '显示大小';

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
  String get coreBlockedTitle => 'Mod Manager 无法启动';

  @override
  String get coreDllMissingMessage => '缺少必需的程序文件 (gore_ffi.dll)。';

  @override
  String get coreDllLoadFailedMessage => '无法加载必需的程序文件。';

  @override
  String get coreVerificationFailedMessage => '无法校验必需的程序文件。';

  @override
  String get coreManagerTooOldMessage =>
      '程序文件比 Mod Manager 更新。请更新 Mod Manager。';

  @override
  String get coreNativeTooOldMessage =>
      '程序文件比 Mod Manager 更旧。请重新安装 Mod Manager。';

  @override
  String get coreCommandsMissingMessage => '程序文件缺少此 Mod Manager 需要的功能。';

  @override
  String get coreBlockedRepairHint => '请重新安装或修复 Mod Manager，然后再次启动。';

  @override
  String get coreTechnicalDetails => '技术详细信息';

  @override
  String get coreCopyTechnicalDetails => '复制技术详细信息';

  @override
  String get coreTechnicalDetailsCopied => '已复制技术详细信息';

  @override
  String get coreTechnicalDetailsCopyFailed => '无法复制技术详细信息。请重试。';

  @override
  String get preflightAttention => '更改模组前，还有一件事需要处理。';

  @override
  String get preflightGameRunning => 'Gothic 仍在运行。请先关闭游戏，再更改模组。';

  @override
  String get managerOperationFailed => '操作失败。';

  @override
  String get libraryOperationFailed => '无法加载模组列表。';

  @override
  String get conflictsUnavailable => '无法检查冲突。';

  @override
  String applyReportAppliedWithWarnings(int applied, int warnings) {
    return '已应用：$applied。警告：$warnings。';
  }

  @override
  String get modDetailKind => '类型';

  @override
  String get modDetailVersion => '版本';

  @override
  String get modDetailAuthor => '作者';

  @override
  String get modDetailSource => '来源';

  @override
  String get modDetailImported => '导入时间';

  @override
  String get componentLocalization => '文本';

  @override
  String get componentAudio => '音效';

  @override
  String get componentAngelScript => '脚本';

  @override
  String get componentTexture => '贴图';

  @override
  String get componentGameFiles => '游戏文件';

  @override
  String get componentVoice => '配音';

  @override
  String get componentKindLocalizationPatch => '文本改动';

  @override
  String get componentKindAudioPatch => '音效改动';

  @override
  String get componentKindAngelScriptPatch => '脚本改动';

  @override
  String get componentKindTexturePatch => '贴图改动';

  @override
  String get componentKindLoosePak => 'PAK 文件';

  @override
  String get componentKindTriplet => 'IoStore 容器';

  @override
  String get componentKindUe4ssLua => 'UE4SS 脚本';

  @override
  String get componentKindRawFile => '文件';

  @override
  String get componentKindFilePatch => '被替换的游戏文件';

  @override
  String get componentKindPakFilePatch => '来自 ~mods PAK 的游戏文件';

  @override
  String get componentKindVoiceArchivePatch => '配音';

  @override
  String get rawTargetGameText => '全部游戏文本';

  @override
  String get rawTargetGameScripts => '全部游戏脚本';

  @override
  String get rawTargetSoundBank => '音效库';

  @override
  String rawTargetSoundBankNamed(String name) {
    return '音效库：$name';
  }

  @override
  String get conflictKindLocalization => '文本';

  @override
  String get conflictKindAudio => '音效';

  @override
  String get conflictKindAsset => '游戏数据';

  @override
  String get conflictKindCdo => '对象数值';

  @override
  String get conflictKindUe4ssUnknown => 'UE4SS（不明）';

  @override
  String get conflictKindScriptModule => '游戏脚本';

  @override
  String get conflictKindVoiceArchive => '配音';

  @override
  String get conflictKindRawFile => '文件';

  @override
  String get conflictKindLooseFile => '游戏文件';

  @override
  String get preflightUnavailable => '无法检查游戏安装。';

  @override
  String get preflightRetry => '重新检查';

  @override
  String get preflightReviewStatus => '查看状态';

  @override
  String get preflightReviewRecovery => '查看帮助';

  @override
  String get installRecoveryTitle => '被中断的安装';

  @override
  String get installRecoveryBody =>
      'GORE 发现了一次安装或脚本编译留下的数据。该任务可能仍在运行，也可能已经结束并留下了这些数据。GORE 无法安全地自行清理。';

  @override
  String get installRecoverySteps =>
      '如果任务仍在运行，请等它结束——不要终止它，也不要删除任何文件。确认没有任务在运行后，按下面文件夹中的 README.txt 操作，然后重新检查。如果没有列出文件夹或你不确定，请保持原样并寻求帮助。';

  @override
  String get installRecoveryEvidence => 'GORE 发现的内容';

  @override
  String get managerRecoveryTitle => '修复被中断的更改';

  @override
  String get managerRecoveryConfirm =>
      'GORE 发现了一次被中断的更改，可以把游戏恢复到已知状态。你的存档绝不会被改动。';

  @override
  String get managerRecoveryAlreadyClean => '没有需要修复的内容。已重新检查状态。';

  @override
  String get managerRecoveryBusy => '任务又在运行了。未做任何更改，请等它结束。';

  @override
  String get managerRecoveryLockCleared => '被中断的任务尚未改动任何内容，已清理完毕。';

  @override
  String get managerRecoveryRestoredPristine => '更改已回滚，游戏已恢复到之前的状态。';

  @override
  String get managerRecoveryApplyPreserved => '应用已经完成，没有丢失任何内容。';

  @override
  String get managerRecoveryUndeployConfirmed => '移除已经完成，残留内容已清理。';

  @override
  String get managerRecoveryCompileRequired => '这属于一次脚本编译，因此未做任何更改。请打开修复帮助。';

  @override
  String get managerRecoveryInspectionFailed => 'GORE 无法安全检查被中断的任务，未做任何更改。';

  @override
  String get managerRecoveryFailed => '修复未能完成。请先查看状态再重试。';

  @override
  String get statusUnknown => '未知';

  @override
  String statusDetailsTitle(String status) {
    return '状态：$status';
  }

  @override
  String statusDetailsOpen(String status) {
    return '显示详情：$status';
  }

  @override
  String get statusDetailsNoRoot => '请先在设置中选择你的 Gothic 安装目录。';

  @override
  String get statusDetailsNoDeployment => '游戏中当前没有安装任何模组。';

  @override
  String get statusDetailsInSyncDescription => '游戏中的模组与此处勾选的完全一致。';

  @override
  String get statusDetailsDeployedLoadout => '游戏中的模组';

  @override
  String get statusDetailsChangesDescription => '你的选择与游戏中的内容不一致。';

  @override
  String get statusDetailsCurrentlyDeployed => '当前游戏中';

  @override
  String get statusDetailsAfterApply => '应用之后';

  @override
  String get statusDetailsGameUpdatedDescription => '游戏更新覆盖了模组文件。请再次应用以恢复。';

  @override
  String get statusDetailsDriftedFiles => '受影响的文件';

  @override
  String get statusDetailsStudioDescription =>
      'Mod Studio 目前在这个游戏里放了模组。请先接管游戏，再让管理器应用你的模组。';

  @override
  String statusDetailsStudioMod(String name) {
    return 'Studio 模组：$name';
  }

  @override
  String get statusDetailsStudioNameUnknown => 'Mod Studio 未提供名称。';

  @override
  String get statusDetailsRecoveryDescription => '有一次更改被中断。请先修复，再更改模组。';

  @override
  String get statusDetailsUnknownDescription => '无法读取状态。请先刷新。';

  @override
  String get statusDetailsUnavailable => '没有可用的详情。';

  @override
  String get statusDetailsEmptyLoadout => '没有模组。';

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
  String get recoveryAction => '修复';

  @override
  String get recoveryRequiredConfirm => '修复被中断的更改并删除安装了一半的文件？';

  @override
  String get statusRecoveryRequired => '需要修复';

  @override
  String get statusDetailsOwnershipTitle => 'GORE 管理的文件';

  @override
  String get statusDetailsOwnershipDescription => '在应用模组时记录，并不代表这些文件现在仍然存在。';

  @override
  String get statusDetailsOwnershipLive => '已替换的游戏文件';

  @override
  String get statusDetailsOwnershipBackups => '原始文件的备份';

  @override
  String get statusDetailsOwnershipAdditive => '新增的模组文件';

  @override
  String get statusDetailsOwnershipUe4ss => 'UE4SS 模组目录';

  @override
  String get statusDetailsOwnershipRecovery => '修复文件';

  @override
  String get statusDetailsOwnershipEmpty => '这里没有记录。';

  @override
  String statusDetailsOwnershipShown(int shown, int total) {
    return '显示 $total 条路径中的 $shown 条。';
  }

  @override
  String get appTitle => 'GORE Mod Manager';

  @override
  String get tabMods => '模组';

  @override
  String get tabSettings => '设置';

  @override
  String get settingsGameExe => 'Gothic 安装目录';

  @override
  String get settingsGameExePick => '选择…';

  @override
  String get settingsLanguage => '语言';

  @override
  String get libraryEmptyTitle => '还没有模组';

  @override
  String get libraryEmptyBody => '导入一个文件夹或模组文件即可开始。';

  @override
  String get detailEmptyHint => '选择一个模组，查看它会改动什么。';

  @override
  String get settingsAdvanced => '高级详情';

  @override
  String get settingsAdvancedHint => '显示技术信息：受影响的条目、冲突检查的可靠程度，以及 GORE 管理的文件。';

  @override
  String get updatesTitle => '更新';

  @override
  String get checkForUpdatesAutomatically => '自动检查更新';

  @override
  String get checkForUpdatesNow => '立即检查更新';

  @override
  String get updatesPortableNotice => '便携版会在浏览器中打开下载页面。请用新下载的文件替换现有文件。';

  @override
  String get updateCheckFailed => '无法检查更新，请稍后再试。';

  @override
  String get updateUpToDate => '你正在使用最新版本。';

  @override
  String get updateAvailableTitle => '有可用更新';

  @override
  String updateAvailableMessage(String version, String current) {
    return '版本 $version 可用，你当前是 $current。';
  }

  @override
  String get updateLater => '稍后';

  @override
  String get updateDownload => '下载';

  @override
  String get statusInSync => '已是最新';

  @override
  String get statusChangesPending => '尚未应用';

  @override
  String get statusGameUpdated => '游戏已更新';

  @override
  String get statusStudioDeploy => 'Mod Studio 正在使用';

  @override
  String get statusNothingDeployed => '游戏中没有模组';

  @override
  String get actionImport => '导入';

  @override
  String get actionApply => '应用';

  @override
  String get actionStartGame => '启动游戏';

  @override
  String get startGameTooltip => '用当前游戏中的模组启动 Gothic';

  @override
  String get startGameFailed => '无法启动 Gothic。请在设置中检查游戏安装。';

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
    return '已添加“$name”。';
  }

  @override
  String importOutcomeUpdated(String name) {
    return '已更新“$name”。';
  }

  @override
  String importOutcomeUnchanged(String name) {
    return '“$name”已在你的列表中。';
  }

  @override
  String importOutcomeMatchedBy(String method) {
    String _temp0 = intl.Intl.selectLogic(method, {
      'none': '没有匹配到已有模组。',
      'source': '按相同的导入来源匹配。',
      'content': '按已验证的相同内容匹配。',
      'entry_id': '按模组 ID 匹配。',
      'other': '没有匹配详情。',
    });
    return '$_temp0';
  }

  @override
  String get importRefusalDuplicateAmbiguous => '这与你已有的多个模组重复。请删除重复项后重试。';

  @override
  String get importRefusalIdentityConflict => '来源和内容分别对应你已有的两个不同模组。请先理清后重试。';

  @override
  String get importFailed =>
      '无法导入。支持文件夹、ZIP 压缩包和单个模组文件（*_P.pak、.utoc/.ucas、.lcache、.bank、PrecompiledScript*.Cache）。请先解压 .7z 或 .rar，再导入文件夹。该模组仍可能已被添加或更新——请先刷新列表再重试。';

  @override
  String get importPickerFailed => '无法打开文件选择器，未导入任何内容。';

  @override
  String get importOutcomeUnknown => '结果不明确。请刷新以检查你的模组列表。';

  @override
  String get applyTooltip => '把勾选的模组装进游戏';

  @override
  String get undeployAllAction => '从游戏中移除全部';

  @override
  String get undeployAllConfirm => '从游戏中移除管理器安装的所有模组？';

  @override
  String get takeOverTitle => 'Mod Studio 正在使用';

  @override
  String get takeOverBody => 'Mod Studio 目前在游戏里放了一个模组。接管后由管理器应用你的选择？';

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
  String get noConflicts => '未发现冲突。';

  @override
  String get conflictCoverageIncomplete => '部分模组无法完全检查，可能还有其他冲突。';

  @override
  String get loadOrderDirection => '列表中靠下的模组会覆盖靠上的模组。';

  @override
  String get footprintCoverageScope => '仅列出已知的冲突目标，不保证游戏中的实际结果。';

  @override
  String get footprintTargetsExact => '受影响的条目 — 完整列表：';

  @override
  String get footprintTargetsPartial => '受影响的条目 — 可能还有更多：';

  @override
  String get footprintTargetsAdvisory => '可能受影响的条目 — 只是线索，并非确证：';

  @override
  String get footprintTargetsOpaque => 'GORE 无法判断这里改动了什么。';

  @override
  String get conflictsUnverified => '冲突未知，请先刷新。';

  @override
  String get componentsTitle => '这个模组会改动什么';

  @override
  String targetsMore(int count) {
    return '还有 $count 项';
  }

  @override
  String get removeModDeploymentHint => '这只会把它从你的列表中移除。如果它已装进游戏，请随后选择“应用”。';

  @override
  String removeModSuccess(String name) {
    return '已移除“$name”。';
  }

  @override
  String removeModFailed(String name) {
    return '无法移除“$name”。';
  }

  @override
  String removeModPartialFailure(String name) {
    return '已移除“$name”，但列表未能完全更新。';
  }

  @override
  String removeModOutcomeUnknown(String name) {
    return '无法确认“$name”是否已被移除。';
  }

  @override
  String get libraryStateUnknown => '模组列表已过期。请先刷新，再更改或应用模组。';

  @override
  String get removeModAction => '移除';

  @override
  String removeModConfirm(String name) {
    return '从你的列表中移除“$name”？';
  }

  @override
  String get errorSetGamePath => '请先在设置中选择你的 Gothic 安装目录。';

  @override
  String applyReportApplied(int count) {
    return '已应用 $count 个模组。';
  }

  @override
  String get modDisabledHint => '已禁用';

  @override
  String get kindGoremod => 'GORE 包';

  @override
  String get kindTriplet => 'IoStore 模组';

  @override
  String get kindPak => 'PAK 模组';

  @override
  String get kindUe4ss => 'UE4SS';

  @override
  String get kindRawfile => '整文件替换';

  @override
  String get kindMixed => '混合';

  @override
  String get sevHard => '冲突';

  @override
  String get sevSoft => '警告';

  @override
  String get sevInfo => '提示';

  @override
  String aboutVersion(String version, String sha) {
    return 'Version $version ($sha)';
  }

  @override
  String get about => '关于';

  @override
  String get aboutCopyright => '© 2026 Daniel Hoer';

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
  String get uiScale => '显示大小';

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
