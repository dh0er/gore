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
      'The new project is open, but the previous project session could not be cleaned up completely. No cleanup retry will be attempted. Restart Mod Studio before reopening the retired project.';
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
}
