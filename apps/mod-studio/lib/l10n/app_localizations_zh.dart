// ignore: unused_import
import 'package:intl/intl.dart' as intl;
import 'app_localizations.dart';

// ignore_for_file: type=lint

/// The translations for Chinese (`zh`).
class AppLocalizationsZh extends AppLocalizations {
  AppLocalizationsZh([String locale = 'zh']) : super(locale);

  @override
  String get tabItems => 'Items';

  @override
  String get tabOverrides => 'Changes';

  @override
  String get tabSettings => 'Settings';

  @override
  String get tabDialogs => '对话';

  @override
  String get tabAudio => '音频';

  @override
  String get tabTextures => '纹理';

  @override
  String get tabScripts => '脚本';

  @override
  String get changesAll => '全部';

  @override
  String get sectionItemValues => '物品数值';

  @override
  String get sectionLocalizedText => '本地化文本';

  @override
  String get audioCatCreatures => '生物';

  @override
  String get audioCatObjects => '物体';

  @override
  String get audioCatMagic => '魔法';

  @override
  String get audioCatMovement => '移动';

  @override
  String get audioCatWorld => '世界';

  @override
  String get audioCatAction => '动作';

  @override
  String get audioCatCombat => '战斗';

  @override
  String get audioCatPhysics => '物理';

  @override
  String get audioCatItems => '物品';

  @override
  String get audioCatUi => '界面';

  @override
  String get audioCatFoley => '拟音';

  @override
  String get audioCatUnderwater => '水下';

  @override
  String get audioCatVision => '幻象';

  @override
  String get audioCatDialog => '对话';

  @override
  String get audioCatOther => '其他';

  @override
  String get gameExecutable => 'Game executable';

  @override
  String get gameExecutableSubtitle =>
      'Path to the game\'s .exe. Used to auto-detect localized text and the game install.';

  @override
  String get gameExecutableNotSet => 'Not set';

  @override
  String get chooseGameExecutable => 'Choose…';

  @override
  String get settingsDataSourceSection => 'Game data';

  @override
  String get settingsLocalizationSection => 'Localized text';

  @override
  String get extractLocalizedText => '提取本地化文本';

  @override
  String get lightMode => '浅色模式';

  @override
  String get darkMode => '深色模式';

  @override
  String get language => '语言';

  @override
  String get exportMod => '导出 Mod';

  @override
  String exportModWithCount(int count) {
    return '导出 Mod（$count）';
  }

  @override
  String get selectAnItemToEdit => '选择一个物品以编辑其字段。';

  @override
  String gameDataActiveTooltip(String name) {
    return '游戏数据：$name';
  }

  @override
  String get gameDataBundledTooltip => '游戏数据：内置';

  @override
  String get loadGameDataDump => '加载游戏数据转储…';

  @override
  String get loadGameDataDumpSubtitle =>
      '来自 gore-dump mod 的 gore_game_data.json';

  @override
  String get useBundledData => '使用内置数据';

  @override
  String get alreadyBundled => '已内置';

  @override
  String get gameDataFileGroupLabel => '游戏数据';

  @override
  String get minimize => '最小化';

  @override
  String get restore => '还原';

  @override
  String get maximize => '最大化';

  @override
  String get close => '关闭';

  @override
  String get about => '关于';

  @override
  String aboutVersion(String version, String sha) {
    return 'Version $version ($sha)';
  }

  @override
  String get aboutCopyright => '© 2026 GORE 贡献者';

  @override
  String get aboutLicense => '基于 MIT 许可证授权。';

  @override
  String get categoryMeleeWeapons => '近战武器';

  @override
  String get categoryRangedWeapons => '远程武器';

  @override
  String get categoryAmmunition => '弹药';

  @override
  String get categoryRunes => '符文';

  @override
  String get categorySpellScrolls => '法术卷轴';

  @override
  String get categoryFoodAndPotions => '食物与药水';

  @override
  String get categoryMiscellaneous => '杂项';

  @override
  String get categoryAmulets => '护身符';

  @override
  String get categoryRings => '戒指';

  @override
  String get categoryAnimalTrophies => '动物战利品';

  @override
  String get categoryWritings => '文书';

  @override
  String get categoryMissionItems => '任务物品';

  @override
  String get categoryKeys => '钥匙';

  @override
  String get categoryOther => '其他';

  @override
  String categoryWithCount(String label, int count) {
    return '$label（$count）';
  }

  @override
  String get searchItems => '搜索物品';

  @override
  String get noItemsMatch => '没有匹配的物品';

  @override
  String failedToLoadCatalog(String error) {
    return '加载目录失败：$error';
  }

  @override
  String pendingOverridesWithCount(int count) {
    return '待应用的修改（$count）';
  }

  @override
  String get clearAll => '全部清除';

  @override
  String get noPendingOverrides => '没有待应用的修改。\n编辑物品字段以添加修改。';

  @override
  String get removeOverride => '移除修改';

  @override
  String get searchChanges => '搜索修改';

  @override
  String get noChangesMatch => '没有匹配的修改';

  @override
  String get clearSection => '清除此分组';

  @override
  String get modName => 'Mod 名称';

  @override
  String get loadDelayLabel => '加载延迟（毫秒，0 = 立即）';

  @override
  String get noFolderSelected => '未选择文件夹';

  @override
  String get chooseFolder => '选择文件夹';

  @override
  String get packageAsZip => '打包为 .zip';

  @override
  String get cancel => '取消';

  @override
  String get export => '导出';

  @override
  String get exportHere => '导出到此处';

  @override
  String get mustBeNonNegativeInteger => '必须为非负整数';

  @override
  String get extractingLocalizedText => '正在提取本地化游戏文本…';

  @override
  String get localizedTextExtractionCancelled => '已取消本地化文本提取。';

  @override
  String get localizedTextExtracted => '已提取本地化文本。';

  @override
  String get extractionFailed => '提取失败。';

  @override
  String get localizationCacheFileGroupLabel => '本地化缓存';

  @override
  String get extractLocalizedTextQuestion => '提取本地化游戏文本？';

  @override
  String get extractLocalizedTextBody => '尚未提取本地化游戏文本。现在从你的游戏安装目录中提取吗？（可选）';

  @override
  String get notNow => '暂不';

  @override
  String get extract => '提取';

  @override
  String get validationRequired => '必填';

  @override
  String get validationMustBeWholeNumber => '必须为整数';

  @override
  String get validationMustBeNumber => '必须为数字';

  @override
  String get validationMustBeFinite => '必须为有限数字';

  @override
  String validationMustBeAtLeast(String min) {
    return '必须 ≥ $min';
  }

  @override
  String validationMustBeAtMost(String max) {
    return '必须 ≤ $max';
  }

  @override
  String get validationMustBeBool => '必须为 true 或 false';

  @override
  String validationMustBeOneOf(String options) {
    return '必须为以下之一：$options';
  }

  @override
  String get modNameRequired => '必填';

  @override
  String get modNameControlCharacters => '不得包含控制字符';

  @override
  String get modNamePathSeparators => '不得包含路径分隔符';

  @override
  String get modNameNotAFolderName => '不是有效的文件夹名称';

  @override
  String localizedTextExtractedCount(int idCount, int languageCount) {
    return '已提取 $idCount 个 ID，涵盖 $languageCount 种语言';
  }

  @override
  String get managerDeployActive =>
      'mod-manager 的 loadout 已启用。请先在 gore-manager 中执行 undeploy。';

  @override
  String get projectOpenLegacy => 'Open legacy project…';

  @override
  String get projectOpenManagedRevision3 => 'Open managed revision-3 project…';

  @override
  String get projectVerifyCurrentHead => 'Verify current head';

  @override
  String get projectManagedRevision3Title => 'Managed revision-3 project';

  @override
  String get projectManagedRevision3IdentityOnly =>
      'This shell currently exposes verified project identity only. Ctrl+S reopens and verifies the exact current head; legacy editors, Build/Deploy, and Save As are unavailable.';

  @override
  String get projectRoot => 'Root';

  @override
  String get projectId => 'Project ID';

  @override
  String get projectRevision => 'Project revision';

  @override
  String get projectHeadSha256 => 'Head SHA-256';

  @override
  String get projectSnapshotBytes => 'Snapshot bytes';

  @override
  String get projectNoCurrent => 'No current project';

  @override
  String projectManagedRevision3Opened(String projectId) {
    return 'Opened managed revision-3 project $projectId';
  }

  @override
  String projectManagedRevision3OpenFailed(String error) {
    return 'Managed revision-3 project open failed: $error';
  }

  @override
  String projectManagedRevision3Verified(String headSha256) {
    return 'Verified revision-3 head $headSha256';
  }

  @override
  String projectManagedRevision3VerifyFailed(String error) {
    return 'Revision-3 head verification failed: $error';
  }

  @override
  String get projectManagedRevision3RequiresReopen =>
      'Exact-head verification could not complete safely. This session now requires recovery and further verification is blocked. Close Mod Studio, then reopen this project before continuing.';

  @override
  String get projectManagedRevision3VerifyBlocked =>
      'Verification is blocked until the managed project is reopened.';

  @override
  String get projectTransitionCleanupWarning =>
      '新项目已打开，但无法完全清理上一个项目的会话。不会再次尝试清理。重新打开上一个项目前，请重启 Mod Studio。';

  @override
  String get projectNewManagedRevision3 => '新建托管模组项目…';

  @override
  String get projectNewLegacy => '新建旧版项目';

  @override
  String get projectCreateGamePathRequired =>
      '创建模组项目前，请先在设置中指定 Gothic 1 Remake 路径。';

  @override
  String get projectCreateDirectoryPickerTitle => '在此创建托管模组项目';

  @override
  String projectManagedRevision3Created(String projectId) {
    return '已创建托管模组项目 $projectId';
  }

  @override
  String projectManagedRevision3CreateFailed(String error) {
    return '无法创建托管模组项目：$error';
  }

  @override
  String get projectCreateDialogTitle => '创建模组项目';

  @override
  String get projectCreateNameLabel => '项目名称';

  @override
  String get projectCreateNameHelper => '在 Mod Studio 中显示的名称。';

  @override
  String get projectCreateVersionLabel => '版本';

  @override
  String get projectCreateVersionHelper => '初始版本，例如 0.1.0。';

  @override
  String get projectCreateAuthorLabel => '作者';

  @override
  String get projectCreateAuthorHelper => '你的姓名或模组团队名称。';

  @override
  String get projectCreateLocalesLabel => '创作语言';

  @override
  String get projectCreateLocalesHelper => '使用逗号分隔的规范标签，例如：en, de, en-US。';

  @override
  String get projectCreateBoundary =>
      '这将创建一个空的托管离线项目。不会构建、部署或运行模组，也不会修改游戏文件或存档。';

  @override
  String get projectCreateSubmit => '创建项目';

  @override
  String projectCreateMetadataRequired(String label) {
    return '必须填写$label。';
  }

  @override
  String projectCreateMetadataNoOuterWhitespace(String label) {
    return '$label的开头或结尾不能有空白字符。';
  }

  @override
  String projectCreateMetadataControlCharacters(String label) {
    return '$label不能包含控制字符。';
  }

  @override
  String projectCreateMetadataMalformed(String label) {
    return '$label包含格式错误的文本。';
  }

  @override
  String projectCreateMetadataTooLong(String label, int maxBytes) {
    return '$label超过 $maxBytes 字节的 UTF-8 限制。';
  }

  @override
  String get projectCreateLocalesRequired => '请至少输入一种创作语言。';

  @override
  String get projectCreateLocalesEmptyEntry => '请删除空的语言项。';

  @override
  String projectCreateLocalesTooMany(int maxLocales) {
    return '最多可使用 $maxLocales 种创作语言。';
  }

  @override
  String projectCreateLocaleBoundedAscii(String locale) {
    return '区域设置“$locale”必须是长度受限的 ASCII。';
  }

  @override
  String projectCreateLocaleLanguage(String locale) {
    return '区域设置“$locale”的语言必须是 2–8 个小写字母。';
  }

  @override
  String projectCreateLocaleInvalidSegment(String locale) {
    return '区域设置“$locale”包含无效片段。';
  }

  @override
  String projectCreateLocaleNotCanonical(String locale, String canonical) {
    return '区域设置“$locale”不是规范形式；请使用“$canonical”。';
  }

  @override
  String get managedWorkspaceOverviewLabel => '概览';

  @override
  String get managedWorkspaceContentLabel => '内容';

  @override
  String get managedWorkspaceDataAssetsLabel => 'DataAssets';

  @override
  String get managedProjectSubtitle => '与当前确切版本匹配的离线创作工作区';

  @override
  String get managedProjectTechnicalDetails => '项目技术详情';

  @override
  String get managedProjectRecoveryContentLocked => '请先重新打开托管项目，再读取其内容。';

  @override
  String get managedDashboardUntitledProject => '未命名项目';

  @override
  String get managedDashboardDraftStatus => '草稿';

  @override
  String get managedDashboardProjectVersion => '版本';

  @override
  String get managedDashboardProjectAuthor => '作者';

  @override
  String get managedDashboardNotProvided => '未提供';

  @override
  String get managedDashboardContentCounts => '项目内容';

  @override
  String get managedDashboardNpcDrafts => 'NPC 草稿';

  @override
  String get managedDashboardQuestDrafts => '任务草稿';

  @override
  String get managedDashboardDialogLines => '对话行';

  @override
  String get managedDashboardVoiceTakes => '语音录音';

  @override
  String get managedDashboardAssets => '资源';

  @override
  String get managedDashboardUnresolvedReferences => '未解析的引用';

  @override
  String get managedDashboardReadiness => '当前可用功能';

  @override
  String get managedDashboardOfflineAuthoringTitle => '离线创作可用';

  @override
  String get managedDashboardOfflineAuthoringDescription =>
      '无需改动游戏安装或存档文件，即可创建和编辑受支持的项目内容。';

  @override
  String get managedDashboardGeneralBuildBlockedTitle => '暂不支持通用模组构建';

  @override
  String get managedDashboardGeneralBuildBlockedDescription =>
      '目前只能构建封装好的离线 Voice 包；尚不能构建完整可玩的模组。';

  @override
  String get managedDashboardRuntimeUnqualifiedTitle => '尚未通过运行时验证';

  @override
  String get managedDashboardRuntimeUnqualifiedDescription =>
      'Mod Studio 尚未验证此项目内容可在运行中的游戏内正常工作。';

  @override
  String get managedDashboardReferenceIntegrityTitle => '引用完整性';

  @override
  String get managedDashboardReferenceIntegrityDescription =>
      '此计数只检查项目引用，并不表示项目已经可以构建或运行。';

  @override
  String get managedDashboardMissingGameTitle => '需要设置游戏';

  @override
  String get managedDashboardMissingGameDescription =>
      '请先在设置中配置 Gothic 1 Remake 安装位置，再使用需要已安装游戏验证信息的操作。';

  @override
  String get managedDashboardCreateHeading => '创建';

  @override
  String get managedDashboardToolsHeading => '项目工具';

  @override
  String get managedDashboardLoading => '正在加载项目概览';

  @override
  String get managedDashboardLoadError => '无法获取项目概览';

  @override
  String get managedDashboardLoadErrorDescription => '无法加载经过验证的项目概览。项目内容未被更改。';

  @override
  String get managedDashboardRetry => '重试';

  @override
  String get managedActionNewNpcTitle => '新建 NPC';

  @override
  String get managedActionNewNpcDescription => '根据已安装游戏的验证信息创建范围受限的离线 NPC 草稿。';

  @override
  String get managedActionNewQuestTitle => '新建任务';

  @override
  String get managedActionNewQuestDescription => '创建带有目标和已验证父级标识的离线任务草稿。';

  @override
  String get managedActionAddVoiceTakeTitle => '添加语音录音';

  @override
  String get managedActionAddVoiceTakeDescription =>
      '将 Ogg Vorbis 录音导入此项目，但不进行部署。';

  @override
  String get managedActionManageVoiceTakesTitle => '管理语音录音';

  @override
  String get managedActionManageVoiceTakesDescription =>
      '审核录音，并为 Voice 槽位选择已批准的录音。';

  @override
  String get managedActionResolveVoiceTargetTitle => '解析 Voice 目标';

  @override
  String get managedActionResolveVoiceTargetDescription =>
      '在不改动游戏的情况下，将项目 Voice 槽位与已安装归档中的精确条目匹配。';

  @override
  String get managedActionBuildVoiceBundleTitle => '构建 Voice 包';

  @override
  String get managedActionBuildVoiceBundleDescription =>
      '使用现有条目构建封装的离线包；不进行部署。';

  @override
  String get managedActionDataAssetsTitle => 'DataAsset 编辑';

  @override
  String get managedActionDataAssetsDescription =>
      '检查已安装的包，并在项目中暂存经过验证的固定宽度值编辑。';

  @override
  String get managedActionSettingsTitle => '设置';

  @override
  String get managedActionSettingsDescription =>
      '配置 Gothic 1 Remake 安装位置和 Mod Studio 偏好设置。';
}

/// The translations for Chinese, using the Han script (`zh_Hans`).
class AppLocalizationsZhHans extends AppLocalizationsZh {
  AppLocalizationsZhHans() : super('zh_Hans');

  @override
  String get tabDialogs => '对话';

  @override
  String get tabAudio => '音频';

  @override
  String get tabTextures => '纹理';

  @override
  String get tabScripts => '脚本';

  @override
  String get changesAll => '全部';

  @override
  String get sectionItemValues => '物品数值';

  @override
  String get sectionLocalizedText => '本地化文本';

  @override
  String get audioCatCreatures => '生物';

  @override
  String get audioCatObjects => '物体';

  @override
  String get audioCatMagic => '魔法';

  @override
  String get audioCatMovement => '移动';

  @override
  String get audioCatWorld => '世界';

  @override
  String get audioCatAction => '动作';

  @override
  String get audioCatCombat => '战斗';

  @override
  String get audioCatPhysics => '物理';

  @override
  String get audioCatItems => '物品';

  @override
  String get audioCatUi => '界面';

  @override
  String get audioCatFoley => '拟音';

  @override
  String get audioCatUnderwater => '水下';

  @override
  String get audioCatVision => '幻象';

  @override
  String get audioCatDialog => '对话';

  @override
  String get audioCatOther => '其他';

  @override
  String get extractLocalizedText => '提取本地化文本';

  @override
  String get lightMode => '浅色模式';

  @override
  String get darkMode => '深色模式';

  @override
  String get language => '语言';

  @override
  String get exportMod => '导出 Mod';

  @override
  String exportModWithCount(int count) {
    return '导出 Mod（$count）';
  }

  @override
  String get selectAnItemToEdit => '选择一个物品以编辑其字段。';

  @override
  String gameDataActiveTooltip(String name) {
    return '游戏数据：$name';
  }

  @override
  String get gameDataBundledTooltip => '游戏数据：内置';

  @override
  String get loadGameDataDump => '加载游戏数据转储…';

  @override
  String get loadGameDataDumpSubtitle =>
      '来自 gore-dump mod 的 gore_game_data.json';

  @override
  String get useBundledData => '使用内置数据';

  @override
  String get alreadyBundled => '已内置';

  @override
  String get gameDataFileGroupLabel => '游戏数据';

  @override
  String get minimize => '最小化';

  @override
  String get restore => '还原';

  @override
  String get maximize => '最大化';

  @override
  String get close => '关闭';

  @override
  String get about => '关于';

  @override
  String aboutVersion(String version, String sha) {
    return 'Version $version ($sha)';
  }

  @override
  String get aboutCopyright => '© 2026 GORE 贡献者';

  @override
  String get aboutLicense => '基于 MIT 许可证授权。';

  @override
  String get categoryMeleeWeapons => '近战武器';

  @override
  String get categoryRangedWeapons => '远程武器';

  @override
  String get categoryAmmunition => '弹药';

  @override
  String get categoryRunes => '符文';

  @override
  String get categorySpellScrolls => '法术卷轴';

  @override
  String get categoryFoodAndPotions => '食物与药水';

  @override
  String get categoryMiscellaneous => '杂项';

  @override
  String get categoryAmulets => '护身符';

  @override
  String get categoryRings => '戒指';

  @override
  String get categoryAnimalTrophies => '动物战利品';

  @override
  String get categoryWritings => '文书';

  @override
  String get categoryMissionItems => '任务物品';

  @override
  String get categoryKeys => '钥匙';

  @override
  String get categoryOther => '其他';

  @override
  String categoryWithCount(String label, int count) {
    return '$label（$count）';
  }

  @override
  String get searchItems => '搜索物品';

  @override
  String get noItemsMatch => '没有匹配的物品';

  @override
  String failedToLoadCatalog(String error) {
    return '加载目录失败：$error';
  }

  @override
  String pendingOverridesWithCount(int count) {
    return '待应用的修改（$count）';
  }

  @override
  String get clearAll => '全部清除';

  @override
  String get noPendingOverrides => '没有待应用的修改。\n编辑物品字段以添加修改。';

  @override
  String get removeOverride => '移除修改';

  @override
  String get searchChanges => '搜索修改';

  @override
  String get noChangesMatch => '没有匹配的修改';

  @override
  String get clearSection => '清除此分组';

  @override
  String get modName => 'Mod 名称';

  @override
  String get loadDelayLabel => '加载延迟（毫秒，0 = 立即）';

  @override
  String get noFolderSelected => '未选择文件夹';

  @override
  String get chooseFolder => '选择文件夹';

  @override
  String get packageAsZip => '打包为 .zip';

  @override
  String get cancel => '取消';

  @override
  String get export => '导出';

  @override
  String get exportHere => '导出到此处';

  @override
  String get mustBeNonNegativeInteger => '必须为非负整数';

  @override
  String get extractingLocalizedText => '正在提取本地化游戏文本…';

  @override
  String get localizedTextExtractionCancelled => '已取消本地化文本提取。';

  @override
  String get localizedTextExtracted => '已提取本地化文本。';

  @override
  String get extractionFailed => '提取失败。';

  @override
  String get localizationCacheFileGroupLabel => '本地化缓存';

  @override
  String get extractLocalizedTextQuestion => '提取本地化游戏文本？';

  @override
  String get extractLocalizedTextBody => '尚未提取本地化游戏文本。现在从你的游戏安装目录中提取吗？（可选）';

  @override
  String get notNow => '暂不';

  @override
  String get extract => '提取';

  @override
  String get validationRequired => '必填';

  @override
  String get validationMustBeWholeNumber => '必须为整数';

  @override
  String get validationMustBeNumber => '必须为数字';

  @override
  String get validationMustBeFinite => '必须为有限数字';

  @override
  String validationMustBeAtLeast(String min) {
    return '必须 ≥ $min';
  }

  @override
  String validationMustBeAtMost(String max) {
    return '必须 ≤ $max';
  }

  @override
  String get validationMustBeBool => '必须为 true 或 false';

  @override
  String validationMustBeOneOf(String options) {
    return '必须为以下之一：$options';
  }

  @override
  String get modNameRequired => '必填';

  @override
  String get modNameControlCharacters => '不得包含控制字符';

  @override
  String get modNamePathSeparators => '不得包含路径分隔符';

  @override
  String get modNameNotAFolderName => '不是有效的文件夹名称';

  @override
  String localizedTextExtractedCount(int idCount, int languageCount) {
    return '已提取 $idCount 个 ID，涵盖 $languageCount 种语言';
  }

  @override
  String get managerDeployActive =>
      'mod-manager 的 loadout 已启用。请先在 gore-manager 中执行 undeploy。';

  @override
  String get projectTransitionCleanupWarning =>
      '新项目已打开，但无法完全清理上一个项目的会话。不会再次尝试清理。重新打开上一个项目前，请重启 Mod Studio。';

  @override
  String get projectNewManagedRevision3 => '新建托管模组项目…';

  @override
  String get projectNewLegacy => '新建旧版项目';

  @override
  String get projectCreateGamePathRequired =>
      '创建模组项目前，请先在设置中指定 Gothic 1 Remake 路径。';

  @override
  String get projectCreateDirectoryPickerTitle => '在此创建托管模组项目';

  @override
  String projectManagedRevision3Created(String projectId) {
    return '已创建托管模组项目 $projectId';
  }

  @override
  String projectManagedRevision3CreateFailed(String error) {
    return '无法创建托管模组项目：$error';
  }

  @override
  String get projectCreateDialogTitle => '创建模组项目';

  @override
  String get projectCreateNameLabel => '项目名称';

  @override
  String get projectCreateNameHelper => '在 Mod Studio 中显示的名称。';

  @override
  String get projectCreateVersionLabel => '版本';

  @override
  String get projectCreateVersionHelper => '初始版本，例如 0.1.0。';

  @override
  String get projectCreateAuthorLabel => '作者';

  @override
  String get projectCreateAuthorHelper => '你的姓名或模组团队名称。';

  @override
  String get projectCreateLocalesLabel => '创作语言';

  @override
  String get projectCreateLocalesHelper => '使用逗号分隔的规范标签，例如：en, de, en-US。';

  @override
  String get projectCreateBoundary =>
      '这将创建一个空的托管离线项目。不会构建、部署或运行模组，也不会修改游戏文件或存档。';

  @override
  String get projectCreateSubmit => '创建项目';

  @override
  String projectCreateMetadataRequired(String label) {
    return '必须填写$label。';
  }

  @override
  String projectCreateMetadataNoOuterWhitespace(String label) {
    return '$label的开头或结尾不能有空白字符。';
  }

  @override
  String projectCreateMetadataControlCharacters(String label) {
    return '$label不能包含控制字符。';
  }

  @override
  String projectCreateMetadataMalformed(String label) {
    return '$label包含格式错误的文本。';
  }

  @override
  String projectCreateMetadataTooLong(String label, int maxBytes) {
    return '$label超过 $maxBytes 字节的 UTF-8 限制。';
  }

  @override
  String get projectCreateLocalesRequired => '请至少输入一种创作语言。';

  @override
  String get projectCreateLocalesEmptyEntry => '请删除空的语言项。';

  @override
  String projectCreateLocalesTooMany(int maxLocales) {
    return '最多可使用 $maxLocales 种创作语言。';
  }

  @override
  String projectCreateLocaleBoundedAscii(String locale) {
    return '区域设置“$locale”必须是长度受限的 ASCII。';
  }

  @override
  String projectCreateLocaleLanguage(String locale) {
    return '区域设置“$locale”的语言必须是 2–8 个小写字母。';
  }

  @override
  String projectCreateLocaleInvalidSegment(String locale) {
    return '区域设置“$locale”包含无效片段。';
  }

  @override
  String projectCreateLocaleNotCanonical(String locale, String canonical) {
    return '区域设置“$locale”不是规范形式；请使用“$canonical”。';
  }

  @override
  String get managedWorkspaceOverviewLabel => '概览';

  @override
  String get managedWorkspaceContentLabel => '内容';

  @override
  String get managedWorkspaceDataAssetsLabel => 'DataAssets';

  @override
  String get managedProjectSubtitle => '与当前确切版本匹配的离线创作工作区';

  @override
  String get managedProjectTechnicalDetails => '项目技术详情';

  @override
  String get managedProjectRecoveryContentLocked => '请先重新打开托管项目，再读取其内容。';

  @override
  String get managedDashboardUntitledProject => '未命名项目';

  @override
  String get managedDashboardDraftStatus => '草稿';

  @override
  String get managedDashboardProjectVersion => '版本';

  @override
  String get managedDashboardProjectAuthor => '作者';

  @override
  String get managedDashboardNotProvided => '未提供';

  @override
  String get managedDashboardContentCounts => '项目内容';

  @override
  String get managedDashboardNpcDrafts => 'NPC 草稿';

  @override
  String get managedDashboardQuestDrafts => '任务草稿';

  @override
  String get managedDashboardDialogLines => '对话行';

  @override
  String get managedDashboardVoiceTakes => '语音录音';

  @override
  String get managedDashboardAssets => '资源';

  @override
  String get managedDashboardUnresolvedReferences => '未解析的引用';

  @override
  String get managedDashboardReadiness => '当前可用功能';

  @override
  String get managedDashboardOfflineAuthoringTitle => '离线创作可用';

  @override
  String get managedDashboardOfflineAuthoringDescription =>
      '无需改动游戏安装或存档文件，即可创建和编辑受支持的项目内容。';

  @override
  String get managedDashboardGeneralBuildBlockedTitle => '暂不支持通用模组构建';

  @override
  String get managedDashboardGeneralBuildBlockedDescription =>
      '目前只能构建封装好的离线 Voice 包；尚不能构建完整可玩的模组。';

  @override
  String get managedDashboardRuntimeUnqualifiedTitle => '尚未通过运行时验证';

  @override
  String get managedDashboardRuntimeUnqualifiedDescription =>
      'Mod Studio 尚未验证此项目内容可在运行中的游戏内正常工作。';

  @override
  String get managedDashboardReferenceIntegrityTitle => '引用完整性';

  @override
  String get managedDashboardReferenceIntegrityDescription =>
      '此计数只检查项目引用，并不表示项目已经可以构建或运行。';

  @override
  String get managedDashboardMissingGameTitle => '需要设置游戏';

  @override
  String get managedDashboardMissingGameDescription =>
      '请先在设置中配置 Gothic 1 Remake 安装位置，再使用需要已安装游戏验证信息的操作。';

  @override
  String get managedDashboardCreateHeading => '创建';

  @override
  String get managedDashboardToolsHeading => '项目工具';

  @override
  String get managedDashboardLoading => '正在加载项目概览';

  @override
  String get managedDashboardLoadError => '无法获取项目概览';

  @override
  String get managedDashboardLoadErrorDescription => '无法加载经过验证的项目概览。项目内容未被更改。';

  @override
  String get managedDashboardRetry => '重试';

  @override
  String get managedActionNewNpcTitle => '新建 NPC';

  @override
  String get managedActionNewNpcDescription => '根据已安装游戏的验证信息创建范围受限的离线 NPC 草稿。';

  @override
  String get managedActionNewQuestTitle => '新建任务';

  @override
  String get managedActionNewQuestDescription => '创建带有目标和已验证父级标识的离线任务草稿。';

  @override
  String get managedActionAddVoiceTakeTitle => '添加语音录音';

  @override
  String get managedActionAddVoiceTakeDescription =>
      '将 Ogg Vorbis 录音导入此项目，但不进行部署。';

  @override
  String get managedActionManageVoiceTakesTitle => '管理语音录音';

  @override
  String get managedActionManageVoiceTakesDescription =>
      '审核录音，并为 Voice 槽位选择已批准的录音。';

  @override
  String get managedActionResolveVoiceTargetTitle => '解析 Voice 目标';

  @override
  String get managedActionResolveVoiceTargetDescription =>
      '在不改动游戏的情况下，将项目 Voice 槽位与已安装归档中的精确条目匹配。';

  @override
  String get managedActionBuildVoiceBundleTitle => '构建 Voice 包';

  @override
  String get managedActionBuildVoiceBundleDescription =>
      '使用现有条目构建封装的离线包；不进行部署。';

  @override
  String get managedActionDataAssetsTitle => 'DataAsset 编辑';

  @override
  String get managedActionDataAssetsDescription =>
      '检查已安装的包，并在项目中暂存经过验证的固定宽度值编辑。';

  @override
  String get managedActionSettingsTitle => '设置';

  @override
  String get managedActionSettingsDescription =>
      '配置 Gothic 1 Remake 安装位置和 Mod Studio 偏好设置。';
}
