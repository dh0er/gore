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
  String get projectClose => 'Close project';

  @override
  String projectCloseFailed(String error) {
    return 'Project could not be closed: $error';
  }

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
  String get managedContentWorkspaceLibraryLabel => '此模组';

  @override
  String get managedWorkspaceHomeLabel => '首页';

  @override
  String get managedWorkspaceStoryLabel => '剧情';

  @override
  String get managedWorkspaceWorldLabel => '世界';

  @override
  String get managedWorkspaceLocalizationVoiceLabel => '本地化与配音';

  @override
  String get managedWorkspaceValidateTestLabel => '验证与测试';

  @override
  String get managedWorkspaceBuildReleaseLabel => '构建与发布';

  @override
  String get managedWorkspaceSettingsExpertLabel => '设置与专家工具';

  @override
  String get managedSectionStoryDescription => 'NPC、任务与对话。';

  @override
  String get managedStoryWorkspaceLoading =>
      'Opening the current Story drafts…';

  @override
  String get managedStoryWorkspaceAuthorityNotice =>
      'Project-only NPC and Quest drafts. Build readiness has not been evaluated; runtime behavior remains unqualified.';

  @override
  String get managedStoryWorkspaceSearchHint =>
      'Search NPC and Quest names, objectives, speakers, or IDs';

  @override
  String get managedStoryWorkspaceCreatingNpc => 'Creating NPC draft…';

  @override
  String get managedStoryWorkspaceCreatingQuest => 'Creating Quest draft…';

  @override
  String get managedStoryWorkspaceEmpty => 'No NPC or Quest drafts yet';

  @override
  String get managedStoryWorkspaceNoMatches =>
      'No NPC or Quest drafts match this search';

  @override
  String get managedStoryWorkspaceSelectDraft =>
      'Select an NPC or Quest draft to continue';

  @override
  String get managedStoryWorkspaceLoadErrorTitle =>
      'Story drafts could not be opened';

  @override
  String get managedStoryWorkspaceCheckpointMismatch =>
      'The project changed while Story was loading. Refresh the exact current checkpoint and try again.';

  @override
  String get managedStoryWorkspacePublishedSelectionStale =>
      'The saved Story draft could not be selected at its exact project revision. Check the current Story list before continuing.';

  @override
  String managedStoryWorkspaceCheckpointSummary(int count, int revision) {
    return 'NPC and Quest drafts: $count · project revision $revision';
  }

  @override
  String managedStoryWorkspaceLoadErrorDetails(String error) {
    return 'The exact current Story view could not be read: $error';
  }

  @override
  String managedStoryWorkspaceCreateErrorDetails(String error) {
    return 'The Story draft could not be created: $error';
  }

  @override
  String managedStoryWorkspaceDetailsSheetLabel(String entityName) {
    return '$entityName Story details';
  }

  @override
  String get managedSectionWorldDescription => '世界放置及相关工作流程尚在规划中。';

  @override
  String get managedSectionLocalizationVoiceDescription =>
      '在同一处编写和翻译项目对话，然后继续处理语音。';

  @override
  String get managedLocalizationProjectTextsLabel => 'Project texts';

  @override
  String get managedLocalizationSearchLabel => 'Search project texts';

  @override
  String get managedLocalizationRefresh => 'Refresh';

  @override
  String get managedLocalizationEmptyTitle => 'No project text yet';

  @override
  String get managedLocalizationEmptyDescription =>
      'Create a dialog line to start writing and translating text.';

  @override
  String get managedLocalizationLoadFailed =>
      'Project texts could not be opened';

  @override
  String get managedLocalizationSelectText => 'Select a project text to edit';

  @override
  String get managedLocalizationLanguagesLabel => 'Languages';

  @override
  String get managedLocalizationUsedByLines => 'Used by dialog lines';

  @override
  String get managedLocalizationNoLine => 'Not used by a dialog line yet';

  @override
  String get managedLocalizationSpeakerLabel => 'Speaker label';

  @override
  String get managedLocalizationAddLanguage => 'Add language';

  @override
  String get managedLocalizationRemoveLanguage => 'Remove language';

  @override
  String get managedLocalizationLanguageHint => 'For example de, en, or pt-BR';

  @override
  String get managedLocalizationLanguageExists =>
      'This language is already present.';

  @override
  String get managedLocalizationAdd => 'Add';

  @override
  String get managedLocalizationSaved => 'Project text saved';

  @override
  String get managedLocalizationVoiceLocked =>
      'This text has recorded voice takes, so its transcript is locked in this editor.';

  @override
  String get managedLocalizationVoiceSlotRemovalLocked =>
      'This language is connected to a Voice slot and cannot be removed here.';

  @override
  String get managedLocalizationMinimumLanguageLocked =>
      'Keep at least one language for this project text.';

  @override
  String get managedLocalizationSharedNotice =>
      'This project text is shared. Saving changes updates every listed dialog line.';

  @override
  String get managedLocalizationOfflineNotice =>
      'Changes are saved only to this managed project. Build and in-game behavior remain separate.';

  @override
  String get managedLocalizationUnsavedTitle => 'Discard unsaved changes?';

  @override
  String get managedLocalizationUnsavedDescription =>
      'You changed this project text. Switching now would discard those edits.';

  @override
  String get managedLocalizationDiscard => 'Discard changes';

  @override
  String get managedLocalizationKeepEditing => 'Keep editing';

  @override
  String get managedLocalizationStale =>
      'The project changed while this text was open. Refresh and try again.';

  @override
  String get managedLocalizationReopen =>
      'The project must be reopened before text editing can continue.';

  @override
  String get managedLocalizationInvalid =>
      'Check that every language and dialog text is valid and not empty.';

  @override
  String get managedLocalizationSaveFailed =>
      'The project text could not be saved.';

  @override
  String get managedSectionValidateTestDescription =>
      '验证项目及检查点的精确完整性；不代表已完成运行时测试。';

  @override
  String get managedSectionBuildReleaseDescription => '语音包现已可用；完整可玩构建和部署尚不可用。';

  @override
  String get managedSectionSettingsExpertDescription => '设置现已可用；专家工具尚未集成。';

  @override
  String get managedSectionStatusHeading => '状态';

  @override
  String get managedSectionActionsHeading => '操作';

  @override
  String get managedCapabilityAvailable => '可用';

  @override
  String get managedCapabilityPartial => '部分可用';

  @override
  String get managedCapabilityPlanned => '已规划';

  @override
  String get managedCapabilityUnavailable => '不可用';

  @override
  String get managedProjectSubtitle => '与当前确切版本匹配的离线创作工作区';

  @override
  String get managedProjectLandingTitle => '托管项目工作区';

  @override
  String get managedProjectLandingDescription =>
      '在一个托管项目中使用新的主页、内容、剧情、语音、验证和发布工作流程。';

  @override
  String get legacyCompatibilityToolsTitle => '旧版兼容工具';

  @override
  String get legacyCompatibilityToolsDescription =>
      '下方标签页是旧版直接替换工具。在托管项目工作区逐步完善期间，这些工具仍可使用。';

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
  String get managedActionNewDialogLineTitle => '添加对话行';

  @override
  String get managedActionNewDialogLineDescription =>
      '编写本地化项目文本，或关联此项目中尚未使用的文本。这不会创建可在游戏中使用的对话主题。';

  @override
  String managedActionNewDialogLineSaved(int projectRevision) {
    return '对话行已保存到项目修订版 $projectRevision。游戏和存档文件均未更改。';
  }

  @override
  String get managedDialogLineIntroduction => '编写新的本地化对话行，或关联已属于此项目的文本。';

  @override
  String get managedDialogLineBoundary =>
      '只会更改项目文件。这不会创建 AngelScript 主题或可在游戏中使用的对话，也绝不会更改游戏安装或存档文件。说话者字段只是标签，不会关联任何 NPC。';

  @override
  String get managedDialogLineCreateMode => '编写新文本';

  @override
  String get managedDialogLineReuseMode => '使用项目文本';

  @override
  String get managedDialogLineNameLabel => '对话行名称';

  @override
  String get managedDialogLineNameHint => '矿井入口问候';

  @override
  String get managedDialogLineSpeakerLabel => '说话者标签（可选）';

  @override
  String get managedDialogLineSpeakerHint => '例如 Viper';

  @override
  String get managedDialogLineLocaleLabel => '语言';

  @override
  String get managedDialogLineTextLabel => '对话文本';

  @override
  String get managedDialogLineReuseSearch => '搜索未使用的项目文本';

  @override
  String get managedDialogLineNoReusableText =>
      '没有可关联的、未使用且结构完整的项目文本。请改为编写新文本。';

  @override
  String get managedDialogLineCreateSlotLabel => '为此语言准备 Voice';

  @override
  String get managedDialogLineCreateSlotHelp =>
      '在项目中创建一个空的未解析 Voice 槽位。不会添加或部署录音。';

  @override
  String get managedDialogLineCancel => '取消';

  @override
  String get managedDialogLineSave => '保存到项目';

  @override
  String get managedDialogLineSaving => '正在保存…';

  @override
  String get managedDialogLineLoading => '正在读取项目的精确内容…';

  @override
  String get managedDialogLineLoadFailed => '无法读取项目当前的精确内容。未进行任何更改。';

  @override
  String get managedDialogLineRetry => '重试';

  @override
  String get managedDialogLineStale => '打开此窗口期间项目已更改。请关闭窗口，并从当前项目重试。';

  @override
  String get managedDialogLineRequiresReopen => '已无法安全验证当前项目。请关闭此窗口并重新打开托管项目。';

  @override
  String get managedDialogLineInvalidInput => '请检查突出显示的项目输入，并选择当前的精确选项。';

  @override
  String get managedDialogLineSaveFailed => '无法安全保存对话行。游戏和存档文件均未更改。';

  @override
  String get managedDialogLineDone => '完成';

  @override
  String get managedDialogLineAddRecording => '添加录音';

  @override
  String get managedActionAddVoiceTakeTitle => '添加语音录音';

  @override
  String get managedActionAddVoiceTakeDescription =>
      '将 Ogg Vorbis 录音导入此项目，但不进行部署。';

  @override
  String get managedActionAddVoiceTakeRequiresDialogLine =>
      'Create or repair a dialog line with one valid localization entry before using Voice tools.';

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
  String get managedActionBrowseProjectContentDescription =>
      '浏览项目的精确内容及其已解析或未解析的引用。';

  @override
  String get managedActionSettingsTitle => '设置';

  @override
  String get managedActionSettingsDescription =>
      '配置 Gothic 1 Remake 安装位置和 Mod Studio 偏好设置。';

  @override
  String projectStarterSetupOpenFailed(String projectId) {
    return '项目 $projectId 已安全创建，但未能打开起始设置。有效的空项目仍为当前项目。';
  }

  @override
  String projectStarterOutcomeUnverified(String projectId) {
    return '项目 $projectId 已创建，但 Mod Studio 无法验证起始设置的结果。请先重新打开托管项目再继续；游戏和存档未被更改。';
  }

  @override
  String projectStarterNpcCancelled(String projectId) {
    return '项目 $projectId 已创建。未添加 NPC 起始内容，因此有效的空项目仍为当前项目。';
  }

  @override
  String projectStarterNpcSaved(int projectRevision) {
    return 'NPC 起始内容已保存到项目修订版 $projectRevision。它仍无法构建、尚未通过运行时验证，也不会生成。';
  }

  @override
  String projectStarterQuestCancelled(String projectId) {
    return '项目 $projectId 已创建。未添加任务起始内容，因此有效的空项目仍为当前项目。';
  }

  @override
  String projectStarterQuestSaved(int projectRevision) {
    return '任务起始内容已保存到项目修订版 $projectRevision。它仍无法构建，且尚未通过运行时验证。';
  }

  @override
  String get projectStarterSemanticsLabel => '项目起始方式';

  @override
  String get projectStarterPrompt => '你想如何开始？';

  @override
  String get projectStarterWriteBoundary =>
      '选择起始方式不会写入任何内容。只有提交此表单并选择空文件夹后，项目才会创建。';

  @override
  String get projectStarterEmptyTitle => '空项目';

  @override
  String get projectStarterEmptyDescription => '仅创建托管项目，准备好后再添加内容。';

  @override
  String get projectStarterNpcDraftTitle => 'NPC 草稿';

  @override
  String get projectStarterNpcDraftDescription => '先创建空项目，然后打开现有的 NPC 草稿引导设置。';

  @override
  String get projectStarterQuestDraftTitle => '任务草稿';

  @override
  String get projectStarterQuestDraftDescription => '先创建空项目，然后打开现有的任务草稿引导设置。';

  @override
  String get projectStarterPartialOutcome =>
      '取消 NPC 或任务引导设置，或草稿失败时，仍会保留有效的空项目。选择起始方式不会写入游戏或存档。';

  @override
  String get managedContentWorkspaceBrowseLabel => '浏览';

  @override
  String get managedContentWorkspaceVerifiedEditsLabel => '已验证编辑';

  @override
  String get managedContentScopeBaseGameLabel => '基础游戏';

  @override
  String get managedContentScopeInstalledLabel => '已安装';

  @override
  String get managedBaseGameBrowserTitle => '支持的基础游戏起始内容';

  @override
  String get managedBaseGameBrowserDescription =>
      '浏览已安装游戏中的精确证据，Mod Studio 目前可检查这些内容，或将其用作安全的草稿起点。这不是完整的原版内容目录。';

  @override
  String get managedBaseGameBrowserLoading => '正在读取基础游戏的精确证据…';

  @override
  String get managedBaseGameBrowserRefresh => '读取新的精确目录';

  @override
  String get managedBaseGameBrowserSearchLabel => '搜索支持的基础游戏内容';

  @override
  String get managedBaseGameBrowserFilterNpcs => 'NPC';

  @override
  String get managedBaseGameBrowserFilterQuests => '任务';

  @override
  String get managedBaseGameBrowserNpcSectionTitle => 'NPC 起始内容';

  @override
  String get managedBaseGameBrowserQuestSectionTitle => '任务起始内容';

  @override
  String get managedBaseGameBrowserExperimentalNpcSectionTitle =>
      '仅供检查的 NPC 原型';

  @override
  String get managedBaseGameBrowserSearchForExperimental =>
      '搜索可包含更多静态链接的 NPC 证据。这些条目不能创建草稿。';

  @override
  String get managedBaseGameBrowserEmpty => '没有支持的基础游戏结果符合此搜索。';

  @override
  String get managedBaseGameBrowserLoadErrorTitle => '基础游戏证据不可用';

  @override
  String get managedBaseGameBrowserLoadErrorDescription =>
      '无法读取精确的支持目录。项目、游戏和存档文件均未更改。';

  @override
  String get managedBaseGameBrowserOfflineDraftBadge => '支持离线草稿';

  @override
  String get managedBaseGameBrowserInspectOnlyBadge => '仅检查';

  @override
  String get managedBaseGameBrowserCreateNpcDraft => '用作 NPC 起点';

  @override
  String get managedBaseGameBrowserCreateQuestDraft => '用作任务起点';

  @override
  String get managedBaseGameBrowserSpawnClass => '生成定义';

  @override
  String get managedBaseGameBrowserActorBlueprint => '角色蓝图';

  @override
  String get managedBaseGameBrowserExperimentalResultsCapped =>
      '正在显示前 100 个仅供检查的匹配项。请细化搜索以获得更具体的结果。';

  @override
  String get managedInstalledBrowserLoading => '正在读取已安装包的精确清单…';

  @override
  String managedInstalledBrowserCompleteSummary(int count) {
    return '$count 个已安装包候选项';
  }

  @override
  String managedInstalledBrowserPartialSummary(int count) {
    return '$count 个已安装包候选项 — 部分结果';
  }

  @override
  String get managedInstalledBrowserCompleteDescription =>
      '已读取目录元数据，并保持了已安装快照的精确性。';

  @override
  String get managedInstalledBrowserPartialDescription =>
      '部分包元数据缺失或不是规范格式；结果可用于发现内容，但并不完整。';

  @override
  String get managedInstalledBrowserAuthorityNotice =>
      '此范围仅显示已安装 DataAsset 包的元数据。检查或复制路径不会授予构建、部署、运行或写入游戏的权限。';

  @override
  String get managedInstalledBrowserRefresh => '读取新的精确快照';

  @override
  String get managedInstalledBrowserSearchLabel => '搜索已安装的 DataAsset';

  @override
  String get managedInstalledBrowserSearchHint => '资源名称或 /Game 路径';

  @override
  String get managedInstalledBrowserSearchPrompt => '输入资源名称或 /Game 路径进行搜索。';

  @override
  String get managedInstalledBrowserNoMatchesTitle => '没有匹配的已安装 DataAsset';

  @override
  String get managedInstalledBrowserNoMatchesDescription =>
      '请尝试其他资源名称或范围更大的 /Game 路径。';

  @override
  String get managedInstalledBrowserResultLimitDescription =>
      '正在显示前 100 个匹配项。请细化搜索以缩小精确快照的范围。';

  @override
  String get managedInstalledBrowserKindBadge => 'DataAsset 包';

  @override
  String get managedInstalledBrowserMetadataOnlyBadge => '仅元数据';

  @override
  String get managedInstalledBrowserOpenInspector => '检查精确包';

  @override
  String get managedInstalledBrowserErrorTitle => '已安装包清单不可用';

  @override
  String get managedInstalledBrowserErrorDescription =>
      '无法读取精确的已安装快照。项目、游戏和存档文件均未更改。';

  @override
  String get managedGlobalSearchScopeLabel => '搜索全部';

  @override
  String get managedGlobalSearchTitle => '搜索所有内容';

  @override
  String get managedGlobalSearchLabel => 'NPC、任务、台词、资产、ID 或 /Game 路径';

  @override
  String get managedGlobalSearchAction => '搜索';

  @override
  String get managedGlobalSearchClear => '清除';

  @override
  String get managedGlobalSearchPrompt => '输入搜索内容以分别读取三个来源。';

  @override
  String get managedGlobalSearchNoResults => '此来源中无匹配项。';

  @override
  String get managedGlobalSearchLoading => '正在读取精确来源…';

  @override
  String get managedGlobalSearchFailed => '无法读取此来源。';

  @override
  String get managedGlobalSearchComplete => '完整';

  @override
  String get managedGlobalSearchPartial => '部分';

  @override
  String get managedGlobalSearchTruncated => '仅显示前 100 个匹配项。请缩小搜索范围。';

  @override
  String get managedGlobalSearchOpen => '打开';

  @override
  String get managedGlobalSearchCreateDraft => '创建草稿';

  @override
  String get managedGlobalSearchInspect => '检查';

  @override
  String get managedGlobalSearchKindModEntity => '模组内容';

  @override
  String get managedGlobalSearchKindModAsset => '模组资产';

  @override
  String get managedGlobalSearchKindBaseNpc => 'NPC 起点';

  @override
  String get managedGlobalSearchKindBaseQuest => '任务起点';

  @override
  String get managedGlobalSearchKindExperimentalNpc => 'NPC 证据';

  @override
  String get managedGlobalSearchReadinessExact => '精确的当前项目';

  @override
  String get managedGlobalSearchReadinessProblems => '精确，但存在问题';

  @override
  String get managedGlobalSearchResultStale => '此结果已不在当前项目中。请重新搜索。';

  @override
  String get managedStoryWorkbenchDraftBadge => '仅草稿';

  @override
  String get managedStoryWorkbenchBuildBlockedBadge => '构建已阻止';

  @override
  String get managedStoryWorkbenchRuntimeUnqualifiedBadge => '运行时未验证';

  @override
  String get managedStoryWorkbenchOverviewTab => '概览';

  @override
  String get managedStoryWorkbenchProfileTab => '档案';

  @override
  String get managedStoryWorkbenchStoryTab => '故事';

  @override
  String get managedStoryWorkbenchLogicTab => '逻辑';

  @override
  String get managedStoryWorkbenchRoutineTab => '日程';

  @override
  String get managedStoryWorkbenchInventoryTab => '物品栏';

  @override
  String get managedStoryWorkbenchDialogVoiceTab => '对话与语音';

  @override
  String get managedStoryWorkbenchReferencesTab => '引用';

  @override
  String get managedStoryWorkbenchProblemsChecksTab => '问题与检查';

  @override
  String get managedStoryWorkbenchEditOverview => '编辑名称和目标';

  @override
  String get managedStoryWorkbenchEditStory => '编辑描述和关联';

  @override
  String get managedStoryWorkbenchEditLogic => '编辑状态和转换';

  @override
  String get managedStoryWorkbenchInspectQuest => '打开源码和编译器检查';

  @override
  String get managedStoryWorkbenchInspectNpc => '打开档案和编译器检查';

  @override
  String get managedStoryWorkbenchCapabilityUnavailable => '尚未建模';

  @override
  String get managedStoryWorkbenchNpcStoryUnavailable => 'NPC 草稿中的任务和故事关系尚未建模。';

  @override
  String get managedStoryWorkbenchNpcRoutineUnavailable => '日程和世界放置尚未建模。';

  @override
  String get managedStoryWorkbenchNpcInventoryUnavailable => '物品栏、装备和交易尚未建模。';

  @override
  String get managedStoryWorkbenchNpcDialogVoiceUnavailable =>
      'NPC 草稿中的对话、本地化和语音关系尚未建模。';

  @override
  String get managedStoryWorkbenchQuestDialogVoiceUnavailable =>
      '任务草稿中的对话、本地化和语音关系尚未建模。';

  @override
  String get managedStoryWorkbenchNoReferenceProblems => '没有未解决的项目引用';

  @override
  String managedStoryWorkbenchReferenceProblemCount(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count 个未解决的项目引用',
      one: '1 个未解决的项目引用',
    );
    return '$_temp0';
  }

  @override
  String get managedStoryWorkbenchReferenceScopeNotice => '仅表示引用状态；不代表已可构建或运行。';

  @override
  String get managedStoryWorkbenchTechnicalDetails => '技术详情';

  @override
  String get managedStoryWorkbenchQuestKindLabel => '任务草稿';

  @override
  String get managedStoryWorkbenchNpcKindLabel => 'NPC 草稿';

  @override
  String get managedStoryWorkbenchQuestTitleLabel => '任务标题';

  @override
  String get managedStoryWorkbenchTechnicalIdLabel => '技术 ID';

  @override
  String get managedStoryWorkbenchObjectivesLabel => '目标';

  @override
  String get managedStoryWorkbenchUniqueNameLabel => '唯一名称';

  @override
  String get managedStoryWorkbenchModuleNamespaceLabel => '模块命名空间';

  @override
  String get managedStoryWorkbenchQuestGiverLabel => '任务发布者';

  @override
  String get managedStoryWorkbenchRuntimeParentLabel => '运行时父类';

  @override
  String get managedStoryWorkbenchLogicDescription =>
      '任务生命周期状态、触发器、条件和效果将作为针对精确当前状态的单个原子操作进行编辑。';

  @override
  String get managedStoryWorkbenchOutgoingHeading => '传出';

  @override
  String get managedStoryWorkbenchNoOutgoingReferences => '没有预计的引用';

  @override
  String get managedStoryWorkbenchIncomingHeading => '传入';

  @override
  String get managedStoryWorkbenchNoIncomingReferences => '没有传入的项目引用';

  @override
  String get managedStoryWorkbenchSemanticIdentityLabel => '语义标识';

  @override
  String get managedStoryWorkbenchOriginLabel => '来源';

  @override
  String get managedStoryWorkbenchEntityRevisionLabel => '实体修订';

  @override
  String get managedStoryWorkbenchStableIdLabel => '稳定 ID';

  @override
  String get managedStoryWorkbenchReferenceResolvedLabel => '引用已解析';

  @override
  String get managedStoryWorkbenchReferenceUnresolvedLabel => '引用未解析';

  @override
  String get managedProblemsTitle => 'Problems & readiness';

  @override
  String get managedProblemsDescription =>
      'See what needs attention and open the exact affected project content.';

  @override
  String get managedProblemsScopeNotice =>
      'Every status covers only its named scope. A clear reference check does not mean the mod can be built or tested in-game.';

  @override
  String get managedProblemsRefresh => 'Refresh problems';

  @override
  String get managedProblemsPartialTitle => 'Some checks are unavailable';

  @override
  String get managedProblemsDataAssetsUnavailable =>
      'DataAsset edits could not be checked. Other exact project findings are still shown.';

  @override
  String get managedProblemsOverviewHeading => 'Readiness by area';

  @override
  String get managedProblemsSearchLabel => 'Search problems';

  @override
  String get managedProblemsClearSearch => 'Clear problem search';

  @override
  String get managedProblemsListHeading => 'Problems';

  @override
  String get managedProblemsEmptyTitle =>
      'No modeled structural problems found';

  @override
  String get managedProblemsEmptyDescription =>
      'The exact checks currently modeled by Mod Studio found nothing to repair.';

  @override
  String get managedProblemsEmptyBoundary =>
      'Compiler evidence was not evaluated, the full managed build is unavailable, and runtime behavior remains unqualified.';

  @override
  String get managedProblemsFilteredEmptyTitle => 'No matching problems';

  @override
  String get managedProblemsFilteredEmptyDescription =>
      'Change the search or category filter to see other findings.';

  @override
  String get managedProblemsSelectTitle => 'Select a problem';

  @override
  String get managedProblemsSelectDescription =>
      'Choose a finding to see what it means and the safest available next action.';

  @override
  String get managedProblemsDetailHeading => 'Problem details';

  @override
  String get managedProblemsCloseDetail => 'Close problem details';

  @override
  String get managedProblemsCategoryLabel => 'Area';

  @override
  String get managedProblemsSeverityLabel => 'Attention';

  @override
  String get managedProblemsSourceLabel => 'Evidence';

  @override
  String get managedProblemsOpenSourceEntity => 'Open source content';

  @override
  String get managedProblemsOpenReferencedAsset => 'Open referenced asset';

  @override
  String get managedProblemsOpenDataAssetEdits => 'Open DataAsset edits';

  @override
  String get managedProblemsActionFailed =>
      'The exact target could not be opened. Refresh the project problems and try again.';

  @override
  String get managedProblemsActionProgress =>
      'Opening the exact project target';

  @override
  String get managedProblemsCategoryReferences => 'References';

  @override
  String get managedProblemsCategorySetup => 'Setup';

  @override
  String get managedProblemsCategoryDataAssets => 'DataAssets';

  @override
  String get managedProblemsSeverityInformation => 'Information';

  @override
  String get managedProblemsSeverityWarning => 'Needs attention';

  @override
  String get managedProblemsSeverityBlocking => 'Blocks this scope';

  @override
  String get managedProblemsScopeReferencesTitle => 'Reference integrity';

  @override
  String get managedProblemsScopeReferencesDescription =>
      'Checks exact links between current project content and assets.';

  @override
  String get managedProblemsScopeDataAssetsTitle => 'DataAsset edit registry';

  @override
  String get managedProblemsScopeDataAssetsDescription =>
      'Checks whether the exact current list of saved DataAsset edits could be read.';

  @override
  String get managedProblemsScopeGameTitle => 'Game setup';

  @override
  String get managedProblemsScopeGameDescription =>
      'Shows whether a game installation is configured for bounded read-only tools.';

  @override
  String get managedProblemsScopeCompilerTitle => 'Source & compiler evidence';

  @override
  String get managedProblemsScopeCompilerDescription =>
      'Compiler checks run only when you explicitly open and start them for one exact entity.';

  @override
  String get managedProblemsScopeBuildTitle => 'Managed project build';

  @override
  String get managedProblemsScopeBuildDescription =>
      'A complete build path for managed NPC, Quest, dialog, and DataAsset edits is not available yet.';

  @override
  String get managedProblemsScopeRuntimeTitle => 'In-game behavior';

  @override
  String get managedProblemsScopeRuntimeDescription =>
      'No general runtime, save, deployment, or cleanup qualification is claimed.';

  @override
  String get managedProblemsReadinessClear => 'Checked within this scope';

  @override
  String get managedProblemsReadinessIssues => 'Needs attention';

  @override
  String get managedProblemsReadinessUnavailable => 'Check unavailable';

  @override
  String get managedProblemsReadinessNotEvaluated => 'Not evaluated';

  @override
  String get managedProblemsReadinessBlocked => 'Build path unavailable';

  @override
  String get managedProblemsReadinessUnqualified => 'Runtime unqualified';

  @override
  String get managedProblemsEvidenceContent => 'Exact current project content';

  @override
  String get managedProblemsEvidenceDataAssets =>
      'Exact current DataAsset registry';

  @override
  String get managedProblemsEvidenceConfiguration =>
      'Current app configuration';

  @override
  String get managedProblemsEvidenceUnavailable =>
      'Evidence source unavailable';

  @override
  String get managedProblemsEvidenceBoundary => 'Known capability boundary';

  @override
  String get managedProblemsForeignReferenceTitle =>
      'Reference points to another project';

  @override
  String get managedProblemsMissingEntityTitle =>
      'Linked project content is missing';

  @override
  String get managedProblemsEntityKindTitle =>
      'Linked project content has the wrong type';

  @override
  String get managedProblemsMissingAssetTitle =>
      'Linked project file is missing';

  @override
  String get managedProblemsAssetLengthTitle =>
      'Linked project file has an unexpected size';

  @override
  String get managedProblemsAssetTypeTitle =>
      'Linked project file has an unexpected type';

  @override
  String get managedProblemsGameSetupTitle =>
      'Game installation is not configured';

  @override
  String get managedProblemsDataAssetRegistryTitle =>
      'DataAsset edits could not be checked';

  @override
  String get managedProblemsDataAssetOfflineTitle =>
      'DataAsset edit is draft-only';

  @override
  String managedProblemsEntityReferenceDescription(String source) {
    return 'Open $source and repair this exact project-content link.';
  }

  @override
  String managedProblemsAssetReferenceDescription(String source) {
    return 'Open $source and repair this exact project-file link.';
  }

  @override
  String get managedProblemsDataAssetRegistryDescription =>
      'Refresh the exact current project. No conclusion is drawn about saved DataAsset edits until this source is available.';

  @override
  String managedProblemsDataAssetOfflineDescription(String targetPath) {
    return 'The saved edit for $targetPath can be reviewed in DataAsset edits, but it cannot be emitted by a managed project build or claimed as working in-game yet.';
  }
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
  String get managedContentWorkspaceLibraryLabel => '此模组';

  @override
  String get managedWorkspaceHomeLabel => '首页';

  @override
  String get managedWorkspaceStoryLabel => '剧情';

  @override
  String get managedWorkspaceWorldLabel => '世界';

  @override
  String get managedWorkspaceLocalizationVoiceLabel => '本地化与配音';

  @override
  String get managedWorkspaceValidateTestLabel => '验证与测试';

  @override
  String get managedWorkspaceBuildReleaseLabel => '构建与发布';

  @override
  String get managedWorkspaceSettingsExpertLabel => '设置与专家工具';

  @override
  String get managedSectionStoryDescription => 'NPC、任务与对话。';

  @override
  String get managedSectionWorldDescription => '世界放置及相关工作流程尚在规划中。';

  @override
  String get managedSectionLocalizationVoiceDescription =>
      '在同一处编写和翻译项目对话，然后继续处理语音。';

  @override
  String get managedSectionValidateTestDescription =>
      '验证项目及检查点的精确完整性；不代表已完成运行时测试。';

  @override
  String get managedSectionBuildReleaseDescription => '语音包现已可用；完整可玩构建和部署尚不可用。';

  @override
  String get managedSectionSettingsExpertDescription => '设置现已可用；专家工具尚未集成。';

  @override
  String get managedSectionStatusHeading => '状态';

  @override
  String get managedSectionActionsHeading => '操作';

  @override
  String get managedCapabilityAvailable => '可用';

  @override
  String get managedCapabilityPartial => '部分可用';

  @override
  String get managedCapabilityPlanned => '已规划';

  @override
  String get managedCapabilityUnavailable => '不可用';

  @override
  String get managedProjectSubtitle => '与当前确切版本匹配的离线创作工作区';

  @override
  String get managedProjectLandingTitle => '托管项目工作区';

  @override
  String get managedProjectLandingDescription =>
      '在一个托管项目中使用新的主页、内容、剧情、语音、验证和发布工作流程。';

  @override
  String get legacyCompatibilityToolsTitle => '旧版兼容工具';

  @override
  String get legacyCompatibilityToolsDescription =>
      '下方标签页是旧版直接替换工具。在托管项目工作区逐步完善期间，这些工具仍可使用。';

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
  String get managedActionNewDialogLineTitle => '添加对话行';

  @override
  String get managedActionNewDialogLineDescription =>
      '编写本地化项目文本，或关联此项目中尚未使用的文本。这不会创建可在游戏中使用的对话主题。';

  @override
  String managedActionNewDialogLineSaved(int projectRevision) {
    return '对话行已保存到项目修订版 $projectRevision。游戏和存档文件均未更改。';
  }

  @override
  String get managedDialogLineIntroduction => '编写新的本地化对话行，或关联已属于此项目的文本。';

  @override
  String get managedDialogLineBoundary =>
      '只会更改项目文件。这不会创建 AngelScript 主题或可在游戏中使用的对话，也绝不会更改游戏安装或存档文件。说话者字段只是标签，不会关联任何 NPC。';

  @override
  String get managedDialogLineCreateMode => '编写新文本';

  @override
  String get managedDialogLineReuseMode => '使用项目文本';

  @override
  String get managedDialogLineNameLabel => '对话行名称';

  @override
  String get managedDialogLineNameHint => '矿井入口问候';

  @override
  String get managedDialogLineSpeakerLabel => '说话者标签（可选）';

  @override
  String get managedDialogLineSpeakerHint => '例如 Viper';

  @override
  String get managedDialogLineLocaleLabel => '语言';

  @override
  String get managedDialogLineTextLabel => '对话文本';

  @override
  String get managedDialogLineReuseSearch => '搜索未使用的项目文本';

  @override
  String get managedDialogLineNoReusableText =>
      '没有可关联的、未使用且结构完整的项目文本。请改为编写新文本。';

  @override
  String get managedDialogLineCreateSlotLabel => '为此语言准备 Voice';

  @override
  String get managedDialogLineCreateSlotHelp =>
      '在项目中创建一个空的未解析 Voice 槽位。不会添加或部署录音。';

  @override
  String get managedDialogLineCancel => '取消';

  @override
  String get managedDialogLineSave => '保存到项目';

  @override
  String get managedDialogLineSaving => '正在保存…';

  @override
  String get managedDialogLineLoading => '正在读取项目的精确内容…';

  @override
  String get managedDialogLineLoadFailed => '无法读取项目当前的精确内容。未进行任何更改。';

  @override
  String get managedDialogLineRetry => '重试';

  @override
  String get managedDialogLineStale => '打开此窗口期间项目已更改。请关闭窗口，并从当前项目重试。';

  @override
  String get managedDialogLineRequiresReopen => '已无法安全验证当前项目。请关闭此窗口并重新打开托管项目。';

  @override
  String get managedDialogLineInvalidInput => '请检查突出显示的项目输入，并选择当前的精确选项。';

  @override
  String get managedDialogLineSaveFailed => '无法安全保存对话行。游戏和存档文件均未更改。';

  @override
  String get managedDialogLineDone => '完成';

  @override
  String get managedDialogLineAddRecording => '添加录音';

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
  String get managedActionBrowseProjectContentDescription =>
      '浏览项目的精确内容及其已解析或未解析的引用。';

  @override
  String get managedActionSettingsTitle => '设置';

  @override
  String get managedActionSettingsDescription =>
      '配置 Gothic 1 Remake 安装位置和 Mod Studio 偏好设置。';

  @override
  String projectStarterSetupOpenFailed(String projectId) {
    return '项目 $projectId 已安全创建，但未能打开起始设置。有效的空项目仍为当前项目。';
  }

  @override
  String projectStarterOutcomeUnverified(String projectId) {
    return '项目 $projectId 已创建，但 Mod Studio 无法验证起始设置的结果。请先重新打开托管项目再继续；游戏和存档未被更改。';
  }

  @override
  String projectStarterNpcCancelled(String projectId) {
    return '项目 $projectId 已创建。未添加 NPC 起始内容，因此有效的空项目仍为当前项目。';
  }

  @override
  String projectStarterNpcSaved(int projectRevision) {
    return 'NPC 起始内容已保存到项目修订版 $projectRevision。它仍无法构建、尚未通过运行时验证，也不会生成。';
  }

  @override
  String projectStarterQuestCancelled(String projectId) {
    return '项目 $projectId 已创建。未添加任务起始内容，因此有效的空项目仍为当前项目。';
  }

  @override
  String projectStarterQuestSaved(int projectRevision) {
    return '任务起始内容已保存到项目修订版 $projectRevision。它仍无法构建，且尚未通过运行时验证。';
  }

  @override
  String get projectStarterSemanticsLabel => '项目起始方式';

  @override
  String get projectStarterPrompt => '你想如何开始？';

  @override
  String get projectStarterWriteBoundary =>
      '选择起始方式不会写入任何内容。只有提交此表单并选择空文件夹后，项目才会创建。';

  @override
  String get projectStarterEmptyTitle => '空项目';

  @override
  String get projectStarterEmptyDescription => '仅创建托管项目，准备好后再添加内容。';

  @override
  String get projectStarterNpcDraftTitle => 'NPC 草稿';

  @override
  String get projectStarterNpcDraftDescription => '先创建空项目，然后打开现有的 NPC 草稿引导设置。';

  @override
  String get projectStarterQuestDraftTitle => '任务草稿';

  @override
  String get projectStarterQuestDraftDescription => '先创建空项目，然后打开现有的任务草稿引导设置。';

  @override
  String get projectStarterPartialOutcome =>
      '取消 NPC 或任务引导设置，或草稿失败时，仍会保留有效的空项目。选择起始方式不会写入游戏或存档。';

  @override
  String get managedContentWorkspaceBrowseLabel => '浏览';

  @override
  String get managedContentWorkspaceVerifiedEditsLabel => '已验证编辑';

  @override
  String get managedContentScopeBaseGameLabel => '基础游戏';

  @override
  String get managedContentScopeInstalledLabel => '已安装';

  @override
  String get managedBaseGameBrowserTitle => '支持的基础游戏起始内容';

  @override
  String get managedBaseGameBrowserDescription =>
      '浏览已安装游戏中的精确证据，Mod Studio 目前可检查这些内容，或将其用作安全的草稿起点。这不是完整的原版内容目录。';

  @override
  String get managedBaseGameBrowserLoading => '正在读取基础游戏的精确证据…';

  @override
  String get managedBaseGameBrowserRefresh => '读取新的精确目录';

  @override
  String get managedBaseGameBrowserSearchLabel => '搜索支持的基础游戏内容';

  @override
  String get managedBaseGameBrowserFilterNpcs => 'NPC';

  @override
  String get managedBaseGameBrowserFilterQuests => '任务';

  @override
  String get managedBaseGameBrowserNpcSectionTitle => 'NPC 起始内容';

  @override
  String get managedBaseGameBrowserQuestSectionTitle => '任务起始内容';

  @override
  String get managedBaseGameBrowserExperimentalNpcSectionTitle =>
      '仅供检查的 NPC 原型';

  @override
  String get managedBaseGameBrowserSearchForExperimental =>
      '搜索可包含更多静态链接的 NPC 证据。这些条目不能创建草稿。';

  @override
  String get managedBaseGameBrowserEmpty => '没有支持的基础游戏结果符合此搜索。';

  @override
  String get managedBaseGameBrowserLoadErrorTitle => '基础游戏证据不可用';

  @override
  String get managedBaseGameBrowserLoadErrorDescription =>
      '无法读取精确的支持目录。项目、游戏和存档文件均未更改。';

  @override
  String get managedBaseGameBrowserOfflineDraftBadge => '支持离线草稿';

  @override
  String get managedBaseGameBrowserInspectOnlyBadge => '仅检查';

  @override
  String get managedBaseGameBrowserCreateNpcDraft => '用作 NPC 起点';

  @override
  String get managedBaseGameBrowserCreateQuestDraft => '用作任务起点';

  @override
  String get managedBaseGameBrowserSpawnClass => '生成定义';

  @override
  String get managedBaseGameBrowserActorBlueprint => '角色蓝图';

  @override
  String get managedBaseGameBrowserExperimentalResultsCapped =>
      '正在显示前 100 个仅供检查的匹配项。请细化搜索以获得更具体的结果。';

  @override
  String get managedInstalledBrowserLoading => '正在读取已安装包的精确清单…';

  @override
  String managedInstalledBrowserCompleteSummary(int count) {
    return '$count 个已安装包候选项';
  }

  @override
  String managedInstalledBrowserPartialSummary(int count) {
    return '$count 个已安装包候选项 — 部分结果';
  }

  @override
  String get managedInstalledBrowserCompleteDescription =>
      '已读取目录元数据，并保持了已安装快照的精确性。';

  @override
  String get managedInstalledBrowserPartialDescription =>
      '部分包元数据缺失或不是规范格式；结果可用于发现内容，但并不完整。';

  @override
  String get managedInstalledBrowserAuthorityNotice =>
      '此范围仅显示已安装 DataAsset 包的元数据。检查或复制路径不会授予构建、部署、运行或写入游戏的权限。';

  @override
  String get managedInstalledBrowserRefresh => '读取新的精确快照';

  @override
  String get managedInstalledBrowserSearchLabel => '搜索已安装的 DataAsset';

  @override
  String get managedInstalledBrowserSearchHint => '资源名称或 /Game 路径';

  @override
  String get managedInstalledBrowserSearchPrompt => '输入资源名称或 /Game 路径进行搜索。';

  @override
  String get managedInstalledBrowserNoMatchesTitle => '没有匹配的已安装 DataAsset';

  @override
  String get managedInstalledBrowserNoMatchesDescription =>
      '请尝试其他资源名称或范围更大的 /Game 路径。';

  @override
  String get managedInstalledBrowserResultLimitDescription =>
      '正在显示前 100 个匹配项。请细化搜索以缩小精确快照的范围。';

  @override
  String get managedInstalledBrowserKindBadge => 'DataAsset 包';

  @override
  String get managedInstalledBrowserMetadataOnlyBadge => '仅元数据';

  @override
  String get managedInstalledBrowserOpenInspector => '检查精确包';

  @override
  String get managedInstalledBrowserErrorTitle => '已安装包清单不可用';

  @override
  String get managedInstalledBrowserErrorDescription =>
      '无法读取精确的已安装快照。项目、游戏和存档文件均未更改。';

  @override
  String get managedGlobalSearchScopeLabel => '搜索全部';

  @override
  String get managedGlobalSearchTitle => '搜索所有内容';

  @override
  String get managedGlobalSearchLabel => 'NPC、任务、台词、资产、ID 或 /Game 路径';

  @override
  String get managedGlobalSearchAction => '搜索';

  @override
  String get managedGlobalSearchClear => '清除';

  @override
  String get managedGlobalSearchPrompt => '输入搜索内容以分别读取三个来源。';

  @override
  String get managedGlobalSearchNoResults => '此来源中无匹配项。';

  @override
  String get managedGlobalSearchLoading => '正在读取精确来源…';

  @override
  String get managedGlobalSearchFailed => '无法读取此来源。';

  @override
  String get managedGlobalSearchComplete => '完整';

  @override
  String get managedGlobalSearchPartial => '部分';

  @override
  String get managedGlobalSearchTruncated => '仅显示前 100 个匹配项。请缩小搜索范围。';

  @override
  String get managedGlobalSearchOpen => '打开';

  @override
  String get managedGlobalSearchCreateDraft => '创建草稿';

  @override
  String get managedGlobalSearchInspect => '检查';

  @override
  String get managedGlobalSearchKindModEntity => '模组内容';

  @override
  String get managedGlobalSearchKindModAsset => '模组资产';

  @override
  String get managedGlobalSearchKindBaseNpc => 'NPC 起点';

  @override
  String get managedGlobalSearchKindBaseQuest => '任务起点';

  @override
  String get managedGlobalSearchKindExperimentalNpc => 'NPC 证据';

  @override
  String get managedGlobalSearchReadinessExact => '精确的当前项目';

  @override
  String get managedGlobalSearchReadinessProblems => '精确，但存在问题';

  @override
  String get managedGlobalSearchResultStale => '此结果已不在当前项目中。请重新搜索。';

  @override
  String get managedStoryWorkbenchDraftBadge => '仅草稿';

  @override
  String get managedStoryWorkbenchBuildBlockedBadge => '构建已阻止';

  @override
  String get managedStoryWorkbenchRuntimeUnqualifiedBadge => '运行时未验证';

  @override
  String get managedStoryWorkbenchOverviewTab => '概览';

  @override
  String get managedStoryWorkbenchProfileTab => '档案';

  @override
  String get managedStoryWorkbenchStoryTab => '故事';

  @override
  String get managedStoryWorkbenchLogicTab => '逻辑';

  @override
  String get managedStoryWorkbenchRoutineTab => '日程';

  @override
  String get managedStoryWorkbenchInventoryTab => '物品栏';

  @override
  String get managedStoryWorkbenchDialogVoiceTab => '对话与语音';

  @override
  String get managedStoryWorkbenchReferencesTab => '引用';

  @override
  String get managedStoryWorkbenchProblemsChecksTab => '问题与检查';

  @override
  String get managedStoryWorkbenchEditOverview => '编辑名称和目标';

  @override
  String get managedStoryWorkbenchEditStory => '编辑描述和关联';

  @override
  String get managedStoryWorkbenchEditLogic => '编辑状态和转换';

  @override
  String get managedStoryWorkbenchInspectQuest => '打开源码和编译器检查';

  @override
  String get managedStoryWorkbenchInspectNpc => '打开档案和编译器检查';

  @override
  String get managedStoryWorkbenchCapabilityUnavailable => '尚未建模';

  @override
  String get managedStoryWorkbenchNpcStoryUnavailable => 'NPC 草稿中的任务和故事关系尚未建模。';

  @override
  String get managedStoryWorkbenchNpcRoutineUnavailable => '日程和世界放置尚未建模。';

  @override
  String get managedStoryWorkbenchNpcInventoryUnavailable => '物品栏、装备和交易尚未建模。';

  @override
  String get managedStoryWorkbenchNpcDialogVoiceUnavailable =>
      'NPC 草稿中的对话、本地化和语音关系尚未建模。';

  @override
  String get managedStoryWorkbenchQuestDialogVoiceUnavailable =>
      '任务草稿中的对话、本地化和语音关系尚未建模。';

  @override
  String get managedStoryWorkbenchNoReferenceProblems => '没有未解决的项目引用';

  @override
  String managedStoryWorkbenchReferenceProblemCount(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count 个未解决的项目引用',
      one: '1 个未解决的项目引用',
    );
    return '$_temp0';
  }

  @override
  String get managedStoryWorkbenchReferenceScopeNotice => '仅表示引用状态；不代表已可构建或运行。';

  @override
  String get managedStoryWorkbenchTechnicalDetails => '技术详情';

  @override
  String get managedStoryWorkbenchQuestKindLabel => '任务草稿';

  @override
  String get managedStoryWorkbenchNpcKindLabel => 'NPC 草稿';

  @override
  String get managedStoryWorkbenchQuestTitleLabel => '任务标题';

  @override
  String get managedStoryWorkbenchTechnicalIdLabel => '技术 ID';

  @override
  String get managedStoryWorkbenchObjectivesLabel => '目标';

  @override
  String get managedStoryWorkbenchUniqueNameLabel => '唯一名称';

  @override
  String get managedStoryWorkbenchModuleNamespaceLabel => '模块命名空间';

  @override
  String get managedStoryWorkbenchQuestGiverLabel => '任务发布者';

  @override
  String get managedStoryWorkbenchRuntimeParentLabel => '运行时父类';

  @override
  String get managedStoryWorkbenchLogicDescription =>
      '任务生命周期状态、触发器、条件和效果将作为针对精确当前状态的单个原子操作进行编辑。';

  @override
  String get managedStoryWorkbenchOutgoingHeading => '传出';

  @override
  String get managedStoryWorkbenchNoOutgoingReferences => '没有预计的引用';

  @override
  String get managedStoryWorkbenchIncomingHeading => '传入';

  @override
  String get managedStoryWorkbenchNoIncomingReferences => '没有传入的项目引用';

  @override
  String get managedStoryWorkbenchSemanticIdentityLabel => '语义标识';

  @override
  String get managedStoryWorkbenchOriginLabel => '来源';

  @override
  String get managedStoryWorkbenchEntityRevisionLabel => '实体修订';

  @override
  String get managedStoryWorkbenchStableIdLabel => '稳定 ID';

  @override
  String get managedStoryWorkbenchReferenceResolvedLabel => '引用已解析';

  @override
  String get managedStoryWorkbenchReferenceUnresolvedLabel => '引用未解析';
}
