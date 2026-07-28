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
  String get projectOpenManagedRevision3 => 'Open mod project…';

  @override
  String get projectVerifyCurrentHead => 'Check project';

  @override
  String get projectManagedRevision3Title => 'Mod project';

  @override
  String get projectClose => 'Close project';

  @override
  String projectCloseFailed(String error) {
    return 'Project could not be closed: $error';
  }

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
  String get projectManagedRevision3Opened => 'Mod project opened.';

  @override
  String projectManagedRevision3OpenFailed(String error) {
    return 'Mod project could not be opened: $error';
  }

  @override
  String get projectManagedRevision3Verified => 'Project checked.';

  @override
  String projectManagedRevision3VerifyFailed(String error) {
    return 'Project check failed: $error';
  }

  @override
  String get projectManagedRevision3RequiresReopen =>
      'The project could not be checked safely. Recover or reopen it before continuing.';

  @override
  String get projectManagedRevision3VerifyBlocked =>
      'Recover or reopen the project before checking it again.';

  @override
  String get projectTransitionCleanupWarning =>
      '新项目已打开，但无法完全清理上一个项目的会话。不会再次尝试清理。重新打开上一个项目前，请重启 Mod Studio。';

  @override
  String get projectNewManagedRevision3 => '新建模组项目…';

  @override
  String get projectCreateGamePathRequired =>
      '创建模组项目前，请先在设置中指定 Gothic 1 Remake 路径。';

  @override
  String get projectCreateDirectoryPickerTitle => '在此创建托管模组项目';

  @override
  String projectManagedRevision3Created(String projectId) {
    return '已创建模组项目 $projectId';
  }

  @override
  String projectManagedRevision3CreateFailed(String error) {
    return '无法创建模组项目：$error';
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
  String get managedWorkspaceSettingsExpertLabel => '设置与专家工具';

  @override
  String get managedProjectHistoryTitle => 'Project history';

  @override
  String get managedProjectHistoryDescription =>
      'Return to an earlier project version without erasing the versions that came after it.';

  @override
  String get managedProjectHistoryBoundary =>
      'History changes only this managed project. It does not modify the game installation or save files.';

  @override
  String get managedProjectHistoryRefresh => 'Refresh project history';

  @override
  String get managedProjectHistoryLoading => 'Loading project history…';

  @override
  String get managedProjectHistoryLoadFailed =>
      'Project history could not be loaded';

  @override
  String get managedProjectHistoryRetry => 'Try again';

  @override
  String get managedProjectHistoryCurrentVersion => 'Current version';

  @override
  String get managedProjectHistoryPreviousVersions => 'Previous versions';

  @override
  String get managedProjectHistoryUndo => 'Undo last change';

  @override
  String get managedProjectHistoryRestoreVersion => 'Restore this version';

  @override
  String get managedProjectHistoryRestoreTitle => 'Restore project version?';

  @override
  String managedProjectHistoryRestoreBody(int revision, int nextRevision) {
    return 'The content from revision $revision will be saved as new revision $nextRevision. The current version remains in history.';
  }

  @override
  String get managedProjectHistoryRestoreBoundary =>
      'Only the project changes. The game installation and save files remain untouched.';

  @override
  String get managedProjectHistoryCancel => 'Cancel';

  @override
  String get managedProjectHistoryRestore => 'Restore';

  @override
  String get managedProjectHistoryRestoring => 'Restoring project version…';

  @override
  String get managedProjectHistoryRestoreFailed =>
      'The project version could not be restored safely. Refresh the history before trying again.';

  @override
  String managedProjectHistoryRestoreSucceeded(int revision) {
    return 'Revision $revision was restored as a new project version.';
  }

  @override
  String get managedProjectHistoryEmpty =>
      'No previous project versions have been recorded yet.';

  @override
  String managedProjectHistoryRecordingStartsAt(int revision) {
    return 'History recording starts at revision $revision; older versions were not guessed from storage.';
  }

  @override
  String get managedProjectHistoryTruncated =>
      'Older project versions have expired from history. Every version shown here is still retained and authenticated by the current project history.';

  @override
  String managedProjectHistoryRevision(int revision) {
    return 'Revision $revision';
  }

  @override
  String get managedProjectHistoryCurrentBadge => 'Current';

  @override
  String get managedProjectHistoryDirtyBlocked =>
      'Finish or discard the open text edit before restoring another project version.';

  @override
  String get managedProjectHistoryBusy =>
      'Another project action is still in progress.';

  @override
  String get managedProjectHistoryUnavailable =>
      'This managed project session does not support authenticated history.';

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
  String get managedStoryWorkspaceCreateNpcOpening =>
      'Create Character + first greeting';

  @override
  String get managedStoryWorkspaceCreatingNpcOpening =>
      'Creating Character + first greeting…';

  @override
  String get managedStoryWorkspaceCreateQuestOpening =>
      'Create Quest + opening line';

  @override
  String get managedStoryWorkspaceCreatingQuestOpening =>
      'Creating Quest + opening line…';

  @override
  String get managedStoryWorkspaceCreateAdvanced => 'Advanced creation options';

  @override
  String get managedStoryWorkspaceCreateNpcAdvanced =>
      'Create Character draft only (advanced)';

  @override
  String get managedStoryWorkspaceCreateQuestAdvanced =>
      'Create Quest draft only (advanced)';

  @override
  String get managedStoryWorkspaceMutationRequiresReopen =>
      'Reopen this project before changing Story content.';

  @override
  String get managedStoryWorkspaceMutationDirtyBlocked =>
      'Save or discard the open localization edits before changing Story content.';

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
  String get managedStoryWorkspaceRemovePairUnavailable =>
      'This draft is not an exact removable draft and generated-script pair.';

  @override
  String get managedStoryWorkspaceRemoveBusy =>
      'Another Story action is still in progress.';

  @override
  String get managedStoryWorkspaceRemoveRequiresReopen =>
      'Reopen this managed project before removing a draft.';

  @override
  String managedStoryWorkspaceRemoveBlocked(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count incoming project references must be removed first.',
      one: '1 incoming project reference must be removed first.',
    );
    return '$_temp0';
  }

  @override
  String get managedStoryWorkspaceRemoveDialogTitle =>
      'Remove draft from project?';

  @override
  String managedStoryWorkspaceRemoveDialogSummary(
    String draftName,
    String scriptName,
  ) {
    return 'This removes the draft \'$draftName\' together with its uniquely owned generated script \'$scriptName\'.';
  }

  @override
  String get managedStoryWorkspaceRemoveNoUndo =>
      'This removal cannot be undone in version 1.';

  @override
  String get managedStoryWorkspaceRemoveBoundary =>
      'Only the current project registry is changed. The game installation and save games stay unchanged.';

  @override
  String get managedStoryWorkspaceRemoveCancel => 'Cancel';

  @override
  String get managedStoryWorkspaceRemoveConfirm => 'Remove draft';

  @override
  String get managedStoryWorkspaceRemoveBlockedTitle =>
      'Draft is still referenced';

  @override
  String get managedStoryWorkspaceRemoveBlockedDescription =>
      'Open every source below and remove its project reference before trying again.';

  @override
  String managedStoryWorkspaceRemoveBlockerLabel(
    String sourceName,
    String role,
  ) {
    return '$sourceName · $role';
  }

  @override
  String get managedStoryWorkspaceRemoveOpenBlocker =>
      'Open referencing source';

  @override
  String get managedStoryWorkspaceRemoveBlockedClose => 'Close';

  @override
  String managedStoryWorkspaceRemoveSucceeded(String draftName) {
    return 'Removed \'$draftName\' and its generated script from the project. Game files and save games were not changed.';
  }

  @override
  String managedStoryWorkspaceRemoveError(String error) {
    return 'The draft was not removed. The Story view was refreshed without retrying automatically: $error';
  }

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
  String get managedLocalizationVoiceContextTitle =>
      'Voice for this dialog line';

  @override
  String get managedLocalizationVoiceSelectLine => 'Select a dialog line above';

  @override
  String get managedLocalizationVoiceSetupExists => 'setup exists';

  @override
  String get managedLocalizationVoiceSetupMissing => 'no setup yet';

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
      'This project has unsaved edits. Switching now would discard them.';

  @override
  String get managedLocalizationVoiceUnsavedTitle =>
      'Save text before continuing?';

  @override
  String get managedLocalizationVoiceUnsavedDescription =>
      'Save these text changes and continue directly to the selected action, keep editing, or deliberately discard the text changes.';

  @override
  String get managedLocalizationDiscardAndContinue => 'Discard and continue';

  @override
  String get managedLocalizationSaveAndContinue => 'Save and continue';

  @override
  String get managedLocalizationGlobalAddVoice => 'Add take for any line';

  @override
  String get managedLocalizationGlobalManageVoice =>
      'Manage takes for any line';

  @override
  String get managedLocalizationGlobalResolveVoice =>
      'Resolve target for any line';

  @override
  String get managedVoiceFolderImportTitle => 'Import recordings folder';

  @override
  String get managedVoiceFolderImportDescription =>
      'Review a folder of named Ogg recordings, then add every ready take in one all-or-nothing project update.';

  @override
  String get managedVoiceFolderImportChooseFolder => 'Choose recordings folder';

  @override
  String get managedVoiceFolderImportDirtyBlocked =>
      'Save or discard the open localization edits before importing recordings.';

  @override
  String managedVoiceFolderImportSaved(int count, int revision) {
    return 'Imported $count recordings in project revision $revision. They are project-only Recorded takes; selection, game files, and saves were not changed.';
  }

  @override
  String managedVoiceTakeSaved(int revision) {
    return 'Voice take saved in project revision $revision. It is saved to the project only and is not yet usable in game.';
  }

  @override
  String managedVoiceSelectionCleared(int revision) {
    return 'Voice selection cleared in project revision $revision. Voice build remains a separate offline step; runtime remains unqualified.';
  }

  @override
  String managedVoiceSelectionSelected(int revision) {
    return 'Approved Voice take selected in project revision $revision. Voice build remains a separate offline step; runtime remains unqualified.';
  }

  @override
  String managedVoiceTargetUnresolvedSaved(int revision) {
    return 'No installed archive member matched. Voice target evidence saved in project revision $revision.';
  }

  @override
  String managedVoiceTargetResolvedSaved(int revision) {
    return 'One installed archive member was sealed. Voice target evidence saved in project revision $revision.';
  }

  @override
  String managedVoiceTargetAmbiguousSaved(int count, int revision) {
    return '$count installed archive members matched; nothing was chosen implicitly. Voice target evidence saved in project revision $revision.';
  }

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
  String get managedLocalizationVoiceActionFailed =>
      'The selected action did not finish cleanly. Refresh the project before trying again; the exact current project will show whether a change was published. This workspace did not change game or save files.';

  @override
  String get managedSectionSettingsExpertDescription =>
      '设置和只读 DataAsset Lab 现已可用。';

  @override
  String get managedSettingsExpertDataAssetLabLabel => 'DataAsset Lab';

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
  String get managedProjectLandingTitle => '开始模组项目';

  @override
  String get managedProjectLandingDescription => '创建项目、打开现有项目文件夹或从备份恢复项目。';

  @override
  String get managedProjectTechnicalDetails => '项目技术详情';

  @override
  String get managedProjectRecoveryContentLocked => '请先重新打开托管项目，再读取其内容。';

  @override
  String get managedProjectRecoveryDescription =>
      'Mod Studio will safely reopen this project while keeping its lock. This does not change the game or any save.';

  @override
  String get managedProjectRecoveryTry => 'Try recovery';

  @override
  String get managedProjectRecoveryTrying => 'Trying recovery…';

  @override
  String get managedProjectRecoveryAlternative =>
      'If recovery does not work, close and open the project again.';

  @override
  String get managedProjectRecoverySucceeded =>
      'Project recovery completed. You can continue working.';

  @override
  String get managedProjectRecoveryFailed =>
      'Recovery did not complete. Try again, or close and open the project again.';

  @override
  String get managedProjectRecoveryUnavailable =>
      'Recovery is not available for this project. Close and open the project again.';

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
  String get managedDashboardChangesDescription =>
      'Everything currently saved in this exact project, grouped by what you can work on. Generated helpers stay attached only when their relationship is proven.';

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
  String get managedDashboardItemPatches => 'Items';

  @override
  String get managedDashboardLocalizationEntries => 'Project text';

  @override
  String get managedDashboardVoiceSlots => 'Voice target';

  @override
  String get managedDashboardGeneratedScripts => 'Generated script';

  @override
  String get managedDashboardSelectedVoiceTake => 'Selected take';

  @override
  String get managedDashboardTechnicalContent => 'Technical content';

  @override
  String get managedDashboardTechnicalContentDescription =>
      'Generated or problematic helpers that cannot be safely assigned to an author-facing change.';

  @override
  String get managedDashboardEmptyChangesTitle => 'No changes yet';

  @override
  String get managedDashboardEmptyChangesDescription =>
      'Use Create, Content, or Story to add the first project change. Nothing has been written to the game or a save.';

  @override
  String get managedDashboardOpenChange => 'Open this exact project change';

  @override
  String get managedDashboardChangeActionFailed =>
      'This project change is no longer current. Reload the project overview and try again.';

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
  String get managedDashboardContinueHeading => 'Continue working';

  @override
  String get managedHomeStoryEmptyTitle => 'Create a character or Quest';

  @override
  String get managedHomeStoryContinueTitle => 'Continue Story';

  @override
  String get managedHomeStoryDescription =>
      'Create and develop NPC and Quest drafts in the complete Story workspace.';

  @override
  String get managedHomeDialogVoiceTitle => 'Dialog & Voice';

  @override
  String get managedHomeDialogVoiceDescription =>
      'Write project text, create dialog lines, and manage Voice takes in one place.';

  @override
  String get managedHomeProblemsTitle => 'Review problems';

  @override
  String get managedHomeProblemsDescription =>
      'Review exact project issues and verification without claiming a runtime test.';

  @override
  String get managedHomeContentTitle => 'Browse content';

  @override
  String get managedHomeContentDescription =>
      'Find project, base-game, installed, and verified DataAsset content.';

  @override
  String get managedHomeBuildTitle => 'Check build readiness';

  @override
  String get managedHomeBuildDescription =>
      'Open the honest build view. Voice bundles are available; a complete playable mod is still blocked.';

  @override
  String get managedContentOpenInStory => 'Open in Story';

  @override
  String get managedContentOpenInStoryDescription =>
      'Continue this Quest or NPC in the complete Story workspace.';

  @override
  String get managedContentOpenInStoryRequiresReopen =>
      'Reopen this project before opening Story.';

  @override
  String get managedContentOpenInStoryFailed =>
      'Story could not be opened. The project was not changed.';

  @override
  String get managedStoryWorkbenchActionFailed =>
      'Could not open this editor. Please try again.';

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
  String managedNpcDraftSaved(int projectRevision) {
    return 'Character draft saved in project revision $projectRevision. It remains build-blocked, runtime-unqualified, and is not spawned.';
  }

  @override
  String get managedNpcOpeningRecipeTitle => 'Character + first greeting';

  @override
  String get managedNpcOpeningRecipeDescription =>
      'Recommended: create a project-only Character draft, then write and insert its first localized greeting. This uses two project checkpoints and does not create a playable conversation or spawn.';

  @override
  String get managedNpcOpeningRecipeIntroduction =>
      'This guided flow first saves the Character draft, then opens its first greeting line. If you stop after step 1, the draft stays saved. It does not create dialog logic, runtime behavior, a spawn, or change the game or save files.';

  @override
  String get managedNpcOpeningRecipeStart => 'Start guided Character';

  @override
  String get managedNpcOpeningGreetingTitle => 'Step 2 of 2: First greeting';

  @override
  String get managedNpcOpeningGreetingIntroduction =>
      'Write the first localized greeting line for this Character draft. Saving creates the line and its text, then inserts it at the start of the draft\'s greeting list. It does not add choices, conditions, effects, or playable conversation behavior.';

  @override
  String managedNpcOpeningRecipePartial(int projectRevision) {
    return 'Character draft saved in project revision $projectRevision; no greeting was added. Continue in Story > Dialog & Voice.';
  }

  @override
  String get managedNpcOpeningRecipeFailed =>
      'The guided Character could not be started. The exact project checkpoint is unchanged; game and save files were not changed.';

  @override
  String get managedNpcOpeningRecipeStopped =>
      'The guided flow stopped because its exact project checkpoint or publication could not be verified. No further step will run automatically; inspect Story and continue manually.';

  @override
  String get managedNpcOpeningRecipeRequiresReopen =>
      'The guided flow could not safely continue. Reopen this project and inspect Story before retrying or continuing manually.';

  @override
  String managedNpcOpeningRecipeComplete(int projectRevision) {
    return 'Character draft and first greeting saved in project revision $projectRevision. Draft only: no playable conversation or spawn was created; game and save files were not changed.';
  }

  @override
  String get managedActionNewQuestTitle => '新建任务';

  @override
  String get managedActionNewQuestDescription => '创建带有目标和已验证父级标识的离线任务草稿。';

  @override
  String get managedQuestOpeningRecipeTitle => '任务 + 首句对话';

  @override
  String get managedQuestOpeningRecipeDescription =>
      '推荐：创建任务草稿，然后编写并插入首条本地化对话。此流程使用两个项目检查点，且不会创建可在游戏中使用的对话。';

  @override
  String get managedQuestOpeningRecipeIntroduction =>
      '此引导流程会先保存任务，再打开其首句对话。如果在第 1 步后停止，任务仍会保留。此流程不会创建可在游戏中使用的对话，也不会更改游戏或存档文件。';

  @override
  String get managedQuestOpeningRecipeStart => '开始引导式任务创建';

  @override
  String get managedQuestOpeningLineTitle => '第 2 步（共 2 步）：首句对话';

  @override
  String get managedQuestOpeningLineIntroduction =>
      '编写此任务的首条本地化对话。保存时会创建对话行及其文本，并将其插入任务对话稿的开头。';

  @override
  String managedQuestOpeningRecipePreparing(int projectRevision) {
    return '任务已保存到项目修订版 $projectRevision。正在准备首句对话...';
  }

  @override
  String managedQuestOpeningRecipePartial(int projectRevision) {
    return '任务已保存到项目修订版 $projectRevision；未添加首句对话。请前往“剧情 > 对话与语音”继续。';
  }

  @override
  String get managedQuestOpeningRecipeFailed => '无法启动引导式任务流程。未发布任何项目更改。';

  @override
  String get managedQuestOpeningRecipeStopped =>
      '由于当前项目的精确状态已发生变化，引导流程已停止。后续步骤不会自动运行；请检查“剧情”并手动继续。';

  @override
  String get managedQuestOpeningRecipeRequiresReopen =>
      '引导流程无法安全继续。请重新打开此项目并检查“剧情”，然后再重试或手动继续。';

  @override
  String managedQuestOpeningRecipeComplete(int projectRevision) {
    return '任务和首句对话已保存到项目修订版 $projectRevision。仅为草稿：未创建可在游戏中使用的对话，也未更改游戏或存档文件。';
  }

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
  String get managedItemsBundledReferenceBadge => 'Bundled reference';

  @override
  String get managedItemsBundledReferenceBoundary =>
      'Read-only reference shipped with Mod Studio. It has not been refreshed or generation-qualified against your configured game installation.';

  @override
  String get managedItemsNoKnownFields =>
      'No modeled scalar fields are available for this item.';

  @override
  String get managedItemsCategorySpecial => 'Special';

  @override
  String get managedItemsCategoryArmor => 'Armor';

  @override
  String get managedItemsExactSchemaBadge => 'Exact project schema';

  @override
  String get managedItemsEditableBadge => 'Managed edit';

  @override
  String get managedItemsBuildPendingBadge => 'Build support pending';

  @override
  String get managedItemsInvalidNumber => 'Enter a valid number.';

  @override
  String managedItemsNumberOutsideNativeRange(String minimum, String maximum) {
    return 'Enter a value from $minimum to $maximum.';
  }

  @override
  String get managedItemsAuthoringBoundary =>
      'Changes are saved only to this managed project. This editor does not write to the game or a save. Item bundle build is not available yet.';

  @override
  String managedItemsCurrentChanges(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count changed fields',
      one: '1 changed field',
      zero: 'No item changes',
    );
    return '$_temp0';
  }

  @override
  String get managedItemsChangeField => 'Change this field';

  @override
  String get managedItemsUseGameDefault => 'Use game default';

  @override
  String get managedItemsSaveChanges => 'Save item changes';

  @override
  String get managedItemsRevertItem => 'Revert item to game defaults';

  @override
  String get managedItemsClearChanges => 'Clear all item changes';

  @override
  String get managedItemsNoUnsavedChanges => 'No unsaved changes.';

  @override
  String managedItemsSaved(int revision) {
    return 'Item changes saved in project revision $revision.';
  }

  @override
  String get managedItemsSaveStale =>
      'The project or item catalog changed. Nothing was saved. Reload the current item data before editing again.';

  @override
  String get managedItemsSaveRequiresReopen =>
      'The project checkpoint can no longer be verified safely. Nothing was saved. Use project recovery, or close and reopen the project.';

  @override
  String get managedItemsSaveNoChanges =>
      'There is no current item change to save. Reload the item data to continue.';

  @override
  String get managedItemsSaveUnsupported =>
      'This change no longer fits the current safe item schema. Nothing was saved. Reload the item data before continuing.';

  @override
  String get managedItemsSaveUnexpected =>
      'Item changes could not be saved safely. Nothing was changed. Reopen the project and try again.';

  @override
  String get managedItemsReloadDiscardDraft =>
      'Reload item data and discard this draft';

  @override
  String get managedItemsCatalogLoadTitle => 'Items are unavailable';

  @override
  String get managedItemsCatalogStale =>
      'The project or exact item catalog changed before the item data could be loaded. Nothing was changed.';

  @override
  String get managedItemsCatalogRequiresReopen =>
      'The exact project checkpoint can no longer be verified safely. Recover the project, or close and reopen it, before editing items.';

  @override
  String get managedItemsCatalogUnsupported =>
      'This project contains item data that the current exact game schema cannot edit safely. Nothing was changed.';

  @override
  String get managedItemsCatalogLoadUnexpected =>
      'The item data could not be loaded safely. Nothing was changed. Try loading it again.';

  @override
  String get managedItemsCatalogReload => 'Reload item data';

  @override
  String get managedItemsUnsupportedSchema =>
      'This item change no longer matches the current safe catalog or field schema. You can still revert the whole item.';

  @override
  String get managedItemsDefaultUnknown => 'Game default not recorded';

  @override
  String managedItemsGameDefault(String value) {
    return 'Game default: $value';
  }

  @override
  String get managedItemsModValue => 'Mod value';

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
  String get managedStoryWorkbenchMoreActions => 'More actions';

  @override
  String get managedStoryWorkbenchRemoveDraft => 'Remove draft…';

  @override
  String get managedStoryWorkbenchRemovingDraft => 'Removing draft…';

  @override
  String get managedStoryWorkbenchReviewRemovalBlockers =>
      'Review removal blockers';

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

  @override
  String get projectExportActionTitle => 'Create project backup…';

  @override
  String get projectExportActionDescription =>
      'Write an exact restorable backup of the current saved project checkpoint.';

  @override
  String get projectExportActionDirtyBlocked =>
      'Save or discard the open localization edits before creating a project backup.';

  @override
  String get projectExportDialogTitle => 'Create project backup';

  @override
  String get projectExportPortableCopyTitle =>
      'Restorable Mod Studio project backup';

  @override
  String get projectExportPortableCopyDescription =>
      'This writes the exact current saved project checkpoint to a new .goremod file. It can be restored into a new project folder later; the open project stays current and unchanged.';

  @override
  String get projectExportCapabilityBoundary =>
      'This backup is not a playable mod, build, deployment, or runtime qualification. Creating it does not read or change the game or any save.';

  @override
  String get projectExportKeepOriginal =>
      'A restore preserves this project\'s identity and history. Use Clone or Save As for a separate project identity when those workflows become available.';

  @override
  String get projectExportFileNameLabel => 'New project-backup file';

  @override
  String get projectExportFileNameHelper =>
      'Use a new backup file name ending in .goremod.';

  @override
  String get projectExportChooseDestination => 'Choose destination folder';

  @override
  String get projectExportNoDestination => 'No destination folder selected';

  @override
  String get projectExportNewFile => 'New file';

  @override
  String get projectExportCancel => 'Cancel';

  @override
  String get projectExportClose => 'Close';

  @override
  String get projectExportSubmit => 'Create backup';

  @override
  String get projectExportExporting => 'Creating backup…';

  @override
  String get projectExportParentRequired =>
      'Choose an existing destination folder.';

  @override
  String get projectExportParentAbsolute =>
      'Choose an absolute existing destination folder.';

  @override
  String get projectExportParentLink =>
      'The selected destination is a link. Choose a real existing folder.';

  @override
  String get projectExportParentInspectFailed =>
      'The destination folder could not be inspected safely. Nothing was created.';

  @override
  String get projectExportFileNameRequired =>
      'Enter a new project-backup file name.';

  @override
  String get projectExportFileNameTooLong =>
      'The file name must be at most 128 ASCII characters.';

  @override
  String get projectExportFileNameInvalid =>
      'Start with a letter or digit, use only ASCII letters, digits, dots, underscores, or hyphens, and end with .goremod.';

  @override
  String get projectExportFileNameReserved =>
      'That file name is reserved by Windows.';

  @override
  String get projectExportOutputExists =>
      'That file already exists. Choose a new file name; existing files are never overwritten.';

  @override
  String get projectExportOutputLink =>
      'The new file path is a link. Choose a different file name.';

  @override
  String get projectExportOutputRejected =>
      'The destination was rejected before the new local file was created. Nothing was created. Choose a different file name or destination folder.';

  @override
  String get projectExportStale =>
      'The project changed before backup creation started. No output was created. Close this window and open Create project backup again.';

  @override
  String get projectExportRequiresReopen =>
      'This project can no longer be verified as current. No output was created. Close this window and recover or reopen the project.';

  @override
  String get projectExportUnsupported =>
      'This managed project session cannot create exact restorable backups. Nothing was created.';

  @override
  String get projectExportFailedBeforeStart =>
      'The project backup could not be prepared exactly. Nothing was created.';

  @override
  String get projectExportPrepublicationFailed =>
      'Backup creation stopped safely before the new local file was created. Nothing was created. Close this window and check the project and destination before trying again.';

  @override
  String projectExportMayExist(String output) {
    return 'Backup creation did not return a verified receipt. Do not retry. Close this window and check the destination: $output';
  }

  @override
  String projectExportResultMismatch(String output) {
    return 'The completed backup does not match this checkpoint or destination. Do not retry; inspect the destination: $output';
  }

  @override
  String get projectExportPublished =>
      'The exact restorable project backup was created as a new local file.';

  @override
  String get projectExportPublishedCleanupWarning =>
      'The exact restorable project backup was created as a local file, but internal temporary-file cleanup was incomplete. The created file is valid; do not retry.';

  @override
  String projectExportPublicationUncertain(String output) {
    return 'The local file may have been created. Do not retry. Check whether this destination exists: $output';
  }

  @override
  String get projectExportArchiveBytes => 'Archive bytes';

  @override
  String get projectExportArchiveSha256 => 'Archive SHA-256';

  @override
  String get projectExportCurrentProjectUnchanged =>
      'The current project remains open and unchanged. The game and saves were not touched.';

  @override
  String get projectRestoreActionTitle => 'Restore project backup…';

  @override
  String get projectRestoreActionDescription =>
      'Verify an exact .goremod backup, restore it into a new folder, and open that project safely.';

  @override
  String get projectRestoreDialogTitle => 'Restore project backup';

  @override
  String get projectRestoreNoticeTitle => 'Restore into a new project folder';

  @override
  String get projectRestoreNoticeDescription =>
      'Choose a restorable Mod Studio .goremod backup. Studio verifies the complete archive before creating a new project folder and preserves the backed-up project identity and history.';

  @override
  String get projectRestoreCapabilityBoundary =>
      'Restore does not build, deploy, launch, or qualify the mod at runtime. It does not read or change the game or any save.';

  @override
  String get projectRestoreChooseBackup => 'Choose backup file';

  @override
  String get projectRestoreNoBackup => 'No verified backup selected';

  @override
  String get projectRestoreInspecting => 'Verifying backup…';

  @override
  String get projectRestoreVerified =>
      'This exact V2 project backup is complete and restorable.';

  @override
  String get projectRestoreSource => 'Backup file';

  @override
  String get projectRestoreProjectRevision => 'Project revision';

  @override
  String get projectRestoreArchiveBytes => 'Archive bytes';

  @override
  String get projectRestoreStoreObjects => 'Stored project objects';

  @override
  String get projectRestoreInvalidSource =>
      'The selected file is not a valid exact project backup. Nothing was created.';

  @override
  String get projectRestoreInspectionFailed =>
      'The backup could not be verified completely. Nothing was created.';

  @override
  String get projectRestoreUnavailable =>
      'Exact project restore is unavailable on this system. Nothing was created.';

  @override
  String get projectRestoreChooseDestinationParent => 'Choose parent folder';

  @override
  String get projectRestoreNoDestinationParent => 'No parent folder selected';

  @override
  String get projectRestoreFolderNameLabel => 'New project folder name';

  @override
  String get projectRestoreFolderNameHelper =>
      'Studio creates this new folder; it must not already exist.';

  @override
  String get projectRestoreNewFolder => 'New project folder';

  @override
  String get projectRestoreFolderNameRequired =>
      'Enter a new project folder name.';

  @override
  String get projectRestoreFolderNameTooLong => 'The folder name is too long.';

  @override
  String get projectRestoreFolderNameInvalid =>
      'Use one ordinary folder name without path separators, control characters, a trailing dot, or a trailing space.';

  @override
  String get projectRestoreFolderNameReserved =>
      'That folder name is reserved by Windows.';

  @override
  String get projectRestoreDestinationExists =>
      'That destination already exists. Choose a new folder name; existing content is never overwritten.';

  @override
  String get projectRestoreDestinationLink =>
      'The new project destination is a link. Choose a different folder name.';

  @override
  String get projectRestoreDestinationInvalid =>
      'The destination was rejected before a project receipt was created. Nothing was opened. Choose a different new folder after verifying the backup again.';

  @override
  String get projectRestoreInspectionExpired =>
      'The backup changed after verification. Nothing was opened. Verify the backup again before choosing another destination.';

  @override
  String get projectRestoreMaterializationFailed =>
      'Restore did not return a verified project receipt. Nothing was opened. Do not reuse this attempt; inspect the chosen destination before starting again.';

  @override
  String projectRestorePublicationUncertain(String destination) {
    return 'Studio cannot prove whether the project folder ‘$destination’ was published. Nothing was opened. Do not retry this restore; inspect that destination first.';
  }

  @override
  String get projectRestoreStale =>
      'This restore window is no longer current. Nothing was opened. If materialization had started, inspect the chosen destination before trying anything else.';

  @override
  String get projectRestoreCancel => 'Cancel';

  @override
  String get projectRestoreClose => 'Close';

  @override
  String get projectRestoreSubmit => 'Restore and open';

  @override
  String get projectRestoreRestoring => 'Restoring…';

  @override
  String get projectRestoreSucceeded =>
      'The exact project backup was restored into the new folder.';

  @override
  String get projectRestoreSucceededCleanupWarning =>
      'The exact project backup was restored, but private temporary cleanup was incomplete. The restored project is valid; do not repeat the restore.';

  @override
  String get projectRestoreOpened => 'Project backup restored and opened.';

  @override
  String get projectRestoreOpenedCleanupWarning =>
      'Project backup restored and opened. Private temporary cleanup was incomplete; do not repeat the restore.';

  @override
  String get projectRestoreOpening => 'Opening the restored project safely…';

  @override
  String projectRestoreOpenFailed(String destination) {
    return 'The project folder ‘$destination’ was restored, but Studio could not prove it safe to open. Any previously open project remains current; otherwise no project was opened. Do not repeat the restore; inspect or open the restored folder separately.';
  }

  @override
  String get projectRestoreCandidateCleanupWarning =>
      'No project was adopted. Studio could not completely clean up the rejected candidate session. Restart Mod Studio before opening the restored destination manually.';

  @override
  String get managedVoiceTakeRemoveAction => 'Remove from this line…';

  @override
  String get managedVoiceTakeRemoveTooltip =>
      'Remove this recording from the current dialog line and language';

  @override
  String get managedVoiceTakeRemoveDialogTitle => 'Remove Voice take?';

  @override
  String managedVoiceTakeRemoveDialogSummary(
    String take,
    String line,
    String locale,
  ) {
    return 'Remove “$take” from $line ($locale)?';
  }

  @override
  String get managedVoiceTakeRemoveScope =>
      'Only the link for this dialog line and language is removed. Other project uses remain unchanged.';

  @override
  String get managedVoiceTakeRemoveInternalRetention =>
      'The audio file remains stored internally. This action does not free project storage and has no undo yet.';

  @override
  String get managedVoiceTakeRemoveGameBoundary =>
      'The game installation and save games are not changed.';

  @override
  String get managedVoiceTakeRemoveSelectedWarning =>
      'This is the active take. Removing it also clears the selection atomically. No replacement is chosen automatically, so Voice build remains blocked until an Approved take is selected.';

  @override
  String get managedVoiceTakeRemoveCancel => 'Cancel';

  @override
  String get managedVoiceTakeRemoveConfirm => 'Remove from line';

  @override
  String get managedVoiceTakeRemoveUniqueSuccess =>
      'The take was removed from this line and from the current project graph. Its internal audio data remains retained.';

  @override
  String get managedVoiceTakeRemoveSharedSuccess =>
      'The link was removed from this line and language. The take remains available to its other project uses, and its internal audio data remains retained.';

  @override
  String get managedVoiceTakeRemoveSelectionClearedSuccess =>
      'The active selection was cleared atomically. No replacement was selected; Voice build is blocked until an Approved take is selected.';

  @override
  String get managedVoiceTakeRemoveStale =>
      'The project changed before the take could be removed. Reload the latest Voice takes and review the action again.';

  @override
  String get managedVoiceTakeRemoveRequiresReopen =>
      'The removal result could not be confirmed. Do not retry. Close this window and reopen or recover the managed project.';

  @override
  String get managedVoiceTakeRemoveSavedUnconfirmed =>
      'The removal was saved, but the latest project could not be confirmed. Do not repeat the removal. Close this window and reopen or recover the managed project.';

  @override
  String get managedVoiceTakeRemoveSavedReloadFailed =>
      'The removal was saved, but the latest Voice takes could not be loaded. Reload the takes; the removal will not be repeated.';

  @override
  String managedVoiceTakeRemoveFailed(String error) {
    return 'The take was not removed: $error';
  }

  @override
  String get managedVoiceTakeRemoveReloadConfirmed =>
      'The saved removal was confirmed from the latest project.';

  @override
  String get managedVoiceSlotRemoveAction => 'Remove empty Voice setup…';

  @override
  String get managedVoiceSlotRemoveDialogTitle => 'Remove empty Voice setup?';

  @override
  String managedVoiceSlotRemoveDialogSummary(String line, String locale) {
    return 'Remove the empty $locale Voice setup from $line?';
  }

  @override
  String get managedVoiceSlotRemoveRetention =>
      'The dialog text stays in the project. No recording, audio blob, game file, or save is deleted.';

  @override
  String get managedVoiceSlotRemoveTargetWarning =>
      'This also removes the stored installed-target evidence for this line and language. The installed archive itself remains untouched.';

  @override
  String get managedVoiceSlotRemoveRecreate =>
      'You can add a new take later; the required Voice setup will then be created again automatically.';

  @override
  String get managedVoiceSlotRemoveCancel => 'Keep setup';

  @override
  String get managedVoiceSlotRemoveConfirm => 'Remove setup';

  @override
  String get managedVoiceSlotRemoveSuccess =>
      'Empty Voice setup removed. The dialog text, audio storage, game files, and saves were not changed.';

  @override
  String get managedVoiceSlotPlanSuccess =>
      'Recording planned. An empty Voice setup was added for this line and language. No audio, game file, or save was changed; build and runtime remain unqualified.';

  @override
  String get managedVoiceSlotRemoveStale =>
      'The project changed before the empty Voice setup could be removed. Reload the latest Voice takes and try again.';

  @override
  String get managedVoiceSlotRemoveRequiresReopen =>
      'Reopen the managed project before removing this Voice setup.';

  @override
  String get managedVoiceSlotRemoveSavedUnconfirmed =>
      'The result could not be confirmed and the empty Voice setup may have been saved. Do not repeat the removal. Close this window, reopen the managed project, and inspect the line.';

  @override
  String get managedVoiceSlotRemoveSavedReloadFailed =>
      'The empty Voice setup was saved, but reloading failed. Reload to confirm it; the removal will not be repeated.';

  @override
  String managedVoiceSlotRemoveFailed(String error) {
    return 'The empty Voice setup could not be removed: $error';
  }

  @override
  String get managedVoiceSlotRemoveReloadConfirmed =>
      'Saved empty Voice setup removal confirmed from the latest project.';

  @override
  String get managedVoicePreviewTooltip => 'Preview selected local Ogg';

  @override
  String get managedVoicePreviewOpened =>
      'Opened the selected local recording for author preview. This does not approve or qualify the audio for the game.';

  @override
  String managedVoicePreviewFailed(String error) {
    return 'The local recording preview could not be opened: $error';
  }

  @override
  String get managedStoryWorkbenchEditNpcProfile => 'Edit name & archetype';

  @override
  String get managedStoryWorkbenchNpcDialogVoiceNextStepTitle =>
      'Next step: Dialog & Voice';

  @override
  String get managedStoryWorkbenchNpcDialogVoiceNextStepDescription =>
      'Draft only: continue with greeting lines, text, and voice. This only links project content; it does not create playable dialog or verify runtime behavior.';

  @override
  String get managedStoryWorkbenchContinueToNpcDialogVoice =>
      'Continue to Dialog & Voice';

  @override
  String get managedStoryWorkbenchNpcDisplayNameLabel => 'Character name';

  @override
  String get managedNpcProfileEditTitle => 'Edit name & archetype';

  @override
  String get managedNpcProfileEditDescription =>
      'Change the friendly character name or choose another verified structural starting point.';

  @override
  String get managedNpcProfileEditNameLabel => 'Character name';

  @override
  String get managedNpcProfileEditNameHint =>
      'Shown to authors in this project.';

  @override
  String get managedNpcProfileEditArchetypeLabel =>
      'Archetype / base character';

  @override
  String get managedNpcProfileEditArchetypeHelp =>
      'This does not edit appearance, stats, faction, routine, inventory, dialog, or spawn.';

  @override
  String get managedNpcProfileEditBoundary =>
      'Only the offline project draft changes. The game installation and save games remain unchanged.';

  @override
  String get managedNpcProfileEditLoading => 'Loading current NPC details…';

  @override
  String get managedNpcProfileEditCancel => 'Cancel';

  @override
  String get managedNpcProfileEditClose => 'Close';

  @override
  String get managedNpcProfileEditSave => 'Save changes';

  @override
  String get managedNpcProfileEditSaving => 'Saving…';

  @override
  String get managedNpcProfileEditRetry => 'Retry';

  @override
  String get managedNpcProfileEditLoadFailed =>
      'NPC details and verified archetypes could not be loaded. No files were changed.';

  @override
  String get managedNpcProfileEditCatalogChanged =>
      'The verified archetypes changed while this editor was open. Review and choose the archetype again before saving.';

  @override
  String get managedNpcProfileEditCurrentArchetypeUnavailable =>
      'The current NPC archetype is no longer represented exactly by this game catalog. No replacement was guessed.';

  @override
  String get managedNpcProfileEditStale =>
      'The project changed while this editor was open. Close it and reopen the NPC from the refreshed Story view.';

  @override
  String get managedNpcProfileEditRequiresReopen =>
      'The save result cannot be verified. Do not retry. Close this editor and reopen or recover the managed project.';

  @override
  String get managedNpcProfileEditSaveFailed =>
      'The NPC changes could not be saved safely. Nothing was built, deployed, or written into the game.';

  @override
  String get managedNpcProfileEditNameRequired => 'Enter a character name.';

  @override
  String get managedNpcProfileEditNameTooLong =>
      'The character name must be at most 256 UTF-8 bytes.';

  @override
  String get managedNpcProfileEditNameControl =>
      'The character name contains an unsupported control character.';

  @override
  String get managedNpcProfileEditReviewSelection =>
      'Review and choose an archetype before saving.';

  @override
  String get managedNpcProfileEditDiscardTitle => 'Discard NPC changes?';

  @override
  String get managedNpcProfileEditDiscardBody =>
      'Your unsaved name and archetype choice will be lost.';

  @override
  String get managedNpcProfileEditKeepEditing => 'Keep editing';

  @override
  String get managedNpcProfileEditDiscard => 'Discard';

  @override
  String managedNpcProfileEditSaved(String name, int revision) {
    return '$name was saved in project revision $revision. It remains an offline, build-blocked draft.';
  }

  @override
  String get managedVoiceBuildReadinessTitle => 'Voice readiness';

  @override
  String get managedVoiceBuildReadinessRefresh => 'Refresh Voice readiness';

  @override
  String get managedVoiceBuildReadinessChecking =>
      'Checking exact Voice readiness';

  @override
  String get managedVoiceBuildReadinessLoadError =>
      'Voice readiness could not be verified for the current project. No build is available from this result.';

  @override
  String get managedVoiceBuildReadinessReadyTitle => 'Voice is ready';

  @override
  String get managedVoiceBuildReadinessBlockedTitle => 'Voice needs attention';

  @override
  String managedVoiceBuildReadinessCount(int readySlots, int totalSlots) {
    return '$readySlots of $totalSlots Voice slots are ready.';
  }

  @override
  String get managedVoiceBuildReadinessBlockedBoundary =>
      'No bundle was created and deployment was not performed.';

  @override
  String get managedVoiceBuildReadinessBuildBundle => 'Build bundle';

  @override
  String get managedVoiceBuildReadinessBuildReleaseGuidance =>
      'Voice content is ready. Open Build & Release to create the offline bundle.';

  @override
  String get managedVoiceBuildReadinessConfigureGameGuidance =>
      'Voice content is ready. Configure the game installation before creating an offline bundle.';

  @override
  String get managedVoiceBuildReadinessHideBlockers => 'Hide blockers';

  @override
  String managedVoiceBuildReadinessShowBlockers(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: 'Show $count blockers',
      one: 'Show 1 blocker',
    );
    return '$_temp0';
  }

  @override
  String get managedVoiceBuildReadinessWorkflowFailed =>
      'The selected Voice workflow could not be opened. Refresh and try again.';

  @override
  String get managedVoiceBuildReadinessBuildWorkflowFailed =>
      'The Voice build workflow could not be opened.';

  @override
  String managedVoiceBuildReadinessExactRevision(int revision) {
    return 'Exact project revision $revision';
  }

  @override
  String get managedVoiceBuildReadinessResolveTarget => 'Resolve target';

  @override
  String get managedVoiceBuildReadinessManageTakes => 'Manage takes';

  @override
  String get managedVoiceBuildBlockerNoSlots =>
      'No Voice setups exist in this project.';

  @override
  String get managedVoiceBuildBlockerPayloadBudget =>
      'The selected Voice recordings exceed the safe bundle memory budget.';

  @override
  String get managedVoiceBuildBlockerUnresolvedTarget =>
      'Resolve this Voice target.';

  @override
  String get managedVoiceBuildBlockerAmbiguousTarget =>
      'This Voice target is ambiguous.';

  @override
  String get managedVoiceBuildBlockerUnqualifiedAdd =>
      'This target is not a sealed existing-member replacement.';

  @override
  String get managedVoiceBuildBlockerMissingTake =>
      'Select an approved Voice take.';

  @override
  String get managedVoiceBuildBlockerTakeNotApproved =>
      'The selected Voice take is not approved.';

  @override
  String get managedVoiceBuildBlockerCodecUnqualified =>
      'The selected Voice take uses an unsupported codec.';

  @override
  String get managedVoiceBuildBlockerSlotLimit =>
      'This project exceeds the 1024-slot Voice bundle limit.';

  @override
  String get managedVoiceBuildOfflineNotice =>
      'Offline build only. This creates a sealed existing-member Voice bundle. It does not deploy or write to the game.';

  @override
  String get managedVoiceBuildNewFolderName => 'New folder name';

  @override
  String get managedVoiceBuildNewFolderHelp =>
      'The bundle must be written to a brand-new child folder.';

  @override
  String get managedVoiceBuildChooseParent => 'Choose parent folder';

  @override
  String get managedVoiceBuildNoParentSelected => 'No parent folder selected';

  @override
  String get managedVoiceBuildNewOutput => 'New output';

  @override
  String get managedVoiceBuildOfflineBundle => 'Build offline bundle';

  @override
  String get managedVoiceBuildParentInspectFailed =>
      'The parent folder could not be inspected safely. No build or deployment was attempted.';

  @override
  String get managedVoiceBuildChooseExistingParent =>
      'Choose an existing parent folder.';

  @override
  String get managedVoiceBuildTargetSymlink =>
      'The target path is a symlink. Choose a different new folder name.';

  @override
  String get managedVoiceBuildTargetExists =>
      'The target already exists. Choose a different new folder name.';

  @override
  String get managedVoiceBuildRequiresReopen =>
      'This project can no longer be verified as current. Close this window and reopen the managed project before building another Voice bundle.';

  @override
  String get managedVoiceBuildStaleCheckpoint =>
      'The managed project changed while this window was open. Close this build window and open it again from the current project.';

  @override
  String get managedVoiceBuildFailed =>
      'The Voice bundle could not be built exactly. No deployment was attempted. Before retrying, choose a new folder name if output was created.';

  @override
  String get managedVoiceBuildPlanFailed =>
      'Voice readiness could not be verified for the exact current project. Output selection and build are unavailable until verification succeeds.';

  @override
  String get managedVoiceBuildParentAbsolute =>
      'Choose an absolute existing parent folder.';

  @override
  String get managedVoiceBuildParentSymlink =>
      'The selected parent is a symlink. Choose a real existing folder.';

  @override
  String get managedVoiceBuildFolderRequired => 'Enter a new folder name.';

  @override
  String get managedVoiceBuildFolderWhitespace =>
      'The folder name cannot start or end with whitespace.';

  @override
  String get managedVoiceBuildFolderTooLong => 'The folder name is too long.';

  @override
  String get managedVoiceBuildFolderPortable =>
      'Use one portable folder name without separators or reserved characters.';

  @override
  String get managedVoiceBuildFolderWindowsReserved =>
      'That folder name is reserved by Windows.';

  @override
  String get managedVoiceBuildExecutableUnavailable =>
      'The installed game executable could not be read. Finish any game update and check the configured installation before trying again. No deployment was attempted.';

  @override
  String get managedVoiceBuildExecutableMismatch =>
      'The installed game executable no longer matches this project generation. Re-import or retarget the managed project before building again. No deployment was attempted.';

  @override
  String get managedVoiceBuildGameUnavailable =>
      'The configured Gothic 1 Remake installation is unavailable. Check it in Settings before trying again. No deployment was attempted.';

  @override
  String get managedVoiceBuildStoreGameAlias =>
      'This project folder overlaps the configured game installation. Move the project outside the game folder before building. No deployment was attempted.';

  @override
  String get managedVoiceBuildGameOutputAlias =>
      'The bundle output overlaps a Gothic 1 Remake installation. Choose a parent folder outside every game installation. No deployment was attempted.';

  @override
  String get managedVoiceBuildStoreOutputAlias =>
      'The bundle output overlaps the managed project. Choose a parent folder outside the project. No deployment was attempted.';

  @override
  String get managedVoiceBuildOutputUnavailable =>
      'The selected output parent is unavailable or cannot be traversed safely. Choose a real existing parent folder outside the project and game.';

  @override
  String get managedVoiceBuildOutputFailed =>
      'The new bundle folder could not be written completely. Do not use any output left there; choose a different new folder name before retrying. No deployment was attempted.';

  @override
  String get managedVoiceBuildPromotionFailed =>
      'The sealed bundle could not be promoted into the requested new output folder. A conflicting output was left untouched and owned staging was removed. Choose a different new folder name before retrying. No deployment was attempted.';

  @override
  String get managedVoiceBuildCleanupFailed =>
      'The Voice bundle was not published, but its temporary staging folder could not be removed completely. Remove the reported staging folder before retrying. No deployment was attempted.';

  @override
  String get managedVoiceBuildPublicationUnconfirmed =>
      'The atomic publication may have succeeded, but its final identity or durability could not be confirmed. Do not retry, replace, or delete that exact output yet. Close this window and inspect the reported folder before deciding how to proceed. No deployment was attempted.';

  @override
  String get managedVoiceBuildStoreRootChanged =>
      'The managed project root changed while the bundle was being built. Close this window and reopen the project before building again. No deployment was attempted.';

  @override
  String get managedVoiceBuildGameRootChanged =>
      'The game installation changed while the bundle was being built. Finish the update or file operation, then retry with a new folder name. No deployment was attempted.';

  @override
  String get managedVoiceBuildOutputRootChanged =>
      'The output parent changed while the bundle was being built. Finish the file operation, verify the parent, then retry with a new folder name. No deployment was attempted.';

  @override
  String get managedVoiceBuildVerifyFailed =>
      'The written bundle could not be verified exactly. Do not use that output; choose a different new folder name before retrying. No deployment was attempted.';

  @override
  String get managedVoiceBuildBundleInvalid =>
      'The selected Voice content could not be lowered into one exact sealed bundle. Reopen the project, review its Voice slots, and try again. No deployment was attempted.';

  @override
  String get managedVoiceBuildInputInvalid =>
      'The Voice build request or output path exceeds the safe supported limits. Choose a shorter new output path and try again. No deployment was attempted.';

  @override
  String get managedVoiceBuildResponseLimit =>
      'The bundle was too large to return an exact build receipt. Do not use any unreceipted output; choose a new folder only after reducing the Voice build. No deployment was attempted.';

  @override
  String get managedVoiceBuildBuiltTitle => 'Sealed Voice bundle built';

  @override
  String get managedVoiceBuildOfflineReceipt =>
      'Offline receipt only. Deployment was not performed.';

  @override
  String get managedVoiceBuildBasisRevision => 'Basis project revision';

  @override
  String get managedVoiceBuildOutputLabel => 'Output';

  @override
  String get managedVoiceBuildArchiveEdits => 'Archive edits';

  @override
  String get managedVoiceBuildBundleFiles => 'Bundle files';

  @override
  String get managedVoiceBuildSealedBytes => 'Sealed bytes';

  @override
  String get managedVoiceBuildBundleSha256 => 'Bundle SHA-256';

  @override
  String get managedVoiceBuildParentPickerTitle => 'Choose Voice bundle parent';

  @override
  String managedVoiceBuildBuiltMessage(String output) {
    return 'Sealed Voice bundle built at $output. Deployment was not performed.';
  }

  @override
  String managedVoiceBuildBlockedMessage(int count) {
    return 'Voice build blocked by $count exact requirements. No bundle was created or deployed.';
  }

  @override
  String get managedTextureSetupTitle => 'Choose the game installation';

  @override
  String get managedTextureSetupDescription =>
      'Textures are read from the configured Gothic 1 Remake installation. Nothing is changed in the game or project.';

  @override
  String get managedTextureSetupAction => 'Open Settings';

  @override
  String get managedTextureLoading => 'Loading the installed texture catalog…';

  @override
  String get managedTextureLoadingDescription =>
      'The first exact scan can take several minutes. Mod Studio runs only one scan at a time and queues the latest refresh.';

  @override
  String managedTextureCatalogCount(int count) {
    return '$count installed textures';
  }

  @override
  String managedTextureSearchCount(int matches, int total) {
    return '$matches matches · $total total';
  }

  @override
  String get managedTextureEmptyTitle => 'No textures found';

  @override
  String get managedTextureEmptyDescription =>
      'The exact installed catalog contains no texture entries.';

  @override
  String get managedTextureErrorTitle => 'Texture catalog unavailable';

  @override
  String get managedTextureErrorDescription =>
      'The installed texture catalog could not be loaded for this exact game build.';

  @override
  String get managedTextureRetry => 'Retry';

  @override
  String get managedTextureRefreshTooltip =>
      'Refresh installed texture catalog';

  @override
  String get managedTextureSearchLabel => 'Search textures';

  @override
  String get managedTextureSearchHint => 'Name or Unreal asset path';

  @override
  String get managedTextureClearSearchTooltip => 'Clear texture search';

  @override
  String get managedTextureSelectPrompt =>
      'Select a texture to inspect its original installed image.';

  @override
  String get managedTexturePreviewLoading => 'Extracting the original texture…';

  @override
  String get managedTexturePreviewErrorTitle => 'Preview unavailable';

  @override
  String get managedTexturePreviewErrorDescription =>
      'The original texture could not be extracted from the selected game build.';

  @override
  String get managedTexturePreviewRetry => 'Retry preview';

  @override
  String get managedTextureBackToCatalog => 'Back to textures';

  @override
  String get managedTextureInspectionOnly =>
      'Installed reference · inspect only. This does not edit the project, game installation, or a save.';

  @override
  String get managedTextureInstalledBadge => 'Installed source';

  @override
  String get managedTextureRegularBadge => 'Regular texture';

  @override
  String get managedTextureVirtualBadge => 'Virtual texture';

  @override
  String managedTextureVirtualLayerCount(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count VT layers',
      one: '1 VT layer',
    );
    return '$_temp0';
  }

  @override
  String get managedTextureMipmappedBadge => 'Mipmapped';

  @override
  String get managedTextureSingleMipBadge => 'Single mip';

  @override
  String get managedTextureReplaceableBadge =>
      'Replacement supported · editing not yet available';

  @override
  String get managedTextureNotReplaceableBadge =>
      'Replacement unavailable · inspect only';

  @override
  String get managedTextureUnknownReplaceabilityBadge =>
      'Replacement not qualified · inspect only';

  @override
  String get managedTextureUnknownFormat => 'Unknown source format';

  @override
  String get managedWorkspaceTextVoiceLabel => '文本与配音';

  @override
  String get managedWorkspaceTestReleaseLabel => '测试与发布';

  @override
  String get managedTestReleaseTitle => '测试与发布';

  @override
  String get managedTestReleaseDescription => '创建或安装可游玩文件前，请检查模组的每个部分。';

  @override
  String get managedTestReleaseEvidenceBoundary =>
      '系统不会自动认定任何内容已就绪。检查结果仅适用于当前这一确切的已保存项目版本。';

  @override
  String get managedTestReleaseChecksHeading => '项目检查';

  @override
  String get managedTestReleaseReleaseHeading => '可游玩输出';

  @override
  String get managedTestReleaseStatusNotChecked => '未检查';

  @override
  String get managedTestReleaseStatusChecking => '检查中';

  @override
  String get managedTestReleaseStatusChecked => '已检查';

  @override
  String get managedTestReleaseStatusNeedsAttention => '需要注意';

  @override
  String get managedTestReleaseStatusBlocked => '已阻止';

  @override
  String get managedTestReleaseStatusNotAvailable => '不可用';

  @override
  String get managedTestReleaseStatusAvailable => '可用';

  @override
  String get managedTestReleaseEvidenceLabel => '证据';

  @override
  String get managedTestReleaseStaleEvidenceDescription =>
      '此结果属于其他项目版本。请重新运行检查。';

  @override
  String get managedTestReleaseActionNotConnectedDescription =>
      '已有证据，但此操作尚未连接到当前工作区。';

  @override
  String get managedTestReleaseProblemsHeading => '需要解决的问题';

  @override
  String get managedTestReleaseVoiceHeading => '配音构建检查';

  @override
  String get managedTestReleaseProjectStructureTitle => '项目结构';

  @override
  String get managedTestReleaseProjectStructureDescription =>
      '请在下方的当前问题列表中检查引用和托管项目结构。';

  @override
  String get managedTestReleaseProjectStructureAction => '查看问题';

  @override
  String get managedTestReleaseScriptsTitle => '脚本';

  @override
  String get managedTestReleaseScriptsDescription =>
      '对这个精确保存的项目版本中的所有脚本运行一次游戏编译器。结果仅作为检查证据；编译输出会被丢弃。';

  @override
  String get managedTestReleaseScriptsAction => '运行编译器检查';

  @override
  String get managedProjectCompilerRetryAction => '重试编译器检查';

  @override
  String get managedProjectCompilerReviewAction => '查看结果/再次检查';

  @override
  String get managedProjectCompilerDialogTitle => '检查所有项目脚本';

  @override
  String get managedProjectCompilerDialogIntroduction =>
      '开始前请关闭 Gothic 1 Remake。Mod Studio 会暂时使用游戏编译器检查所有项目脚本，恢复游戏安装，并丢弃全部编译输出。此结果不能创建可玩文件，也不能安装模组。';

  @override
  String get managedProjectCompilerCloseAction => '关闭';

  @override
  String get managedProjectCompilerNoGame =>
      '运行此检查前，请在设置中选择 Gothic 1 Remake 安装位置。';

  @override
  String get managedProjectCompilerSafetyBlocked =>
      '游戏安装尚未准备好进行编译器检查。请关闭游戏或解决恢复警告，然后重试。';

  @override
  String get managedProjectCompilerCompiled =>
      '所有项目脚本都已通过这个精确保存版本的检查。编译输出已被丢弃。';

  @override
  String get managedProjectCompilerEmpty => '这个保存的项目没有需要编译的脚本。空结果已得到精确检查。';

  @override
  String get managedProjectCompilerRejected =>
      '游戏编译器在一个或多个项目脚本中发现了问题。请修复下面的消息，然后重试。';

  @override
  String get managedProjectCompilerPreflightBlocked =>
      '编译器未启动。请关闭游戏并检查已配置的安装，然后重试。';

  @override
  String get managedProjectCompilerDrifted =>
      '项目或游戏输入发生了变化，或者最终核对不再精确。此结果已被丢弃；请为当前版本重新运行检查。';

  @override
  String get managedProjectCompilerRequiresReopen =>
      '再次进行精确编译器检查前，必须关闭并重新打开此项目。';

  @override
  String get managedProjectCompilerRecoveryRequired =>
      '无法确认私有编译器输出清理或游戏安装精确还原已完成。在新的安全检查成功之前，后续编译器检查和安装将保持阻止状态。';

  @override
  String get managedProjectCompilerFailed =>
      '无法完成或验证编译器检查。未保留任何结果；请在游戏安装准备好后重试。';

  @override
  String get managedProjectCompilerFailureDetails => '编译器消息';

  @override
  String get managedProjectCompilerDiagnosticsHeading => '编译器消息';

  @override
  String get managedProjectCompilerCaptureCaptured => '已捕获结构化编译器消息。';

  @override
  String get managedProjectCompilerCaptureFallback =>
      '诊断挂钩不可用，因此使用了普通游戏编译器作为回退。';

  @override
  String get managedProjectCompilerCaptureInvalid => '无法验证编译器消息捕获。';

  @override
  String get managedProjectCompilerCaptureUnavailable =>
      '编译器运行后诊断挂钩不可用；无需再次运行。';

  @override
  String get managedProjectCompilerCaptureExitUnconfirmed => '编译器进程未确认已经退出。';

  @override
  String get managedProjectCompilerCaptureDisabled => '本次运行没有可用的结构化编译器消息。';

  @override
  String get managedProjectCompilerSeverityError => '错误';

  @override
  String get managedProjectCompilerSeverityWarning => '警告';

  @override
  String get managedProjectCompilerSeverityNote => '备注';

  @override
  String get managedProjectCompilerFileLabel => '文件';

  @override
  String get managedProjectCompilerLineLabel => '行';

  @override
  String get managedProjectCompilerColumnLabel => '列';

  @override
  String get managedProjectCompilerOmittedDiagnostics => '条其他编译器消息已省略';

  @override
  String get managedTestReleaseVoiceTitle => '文本与配音';

  @override
  String get managedTestReleaseVoiceDescription => '请使用下方的配音构建检查来检查当前保存的项目版本。';

  @override
  String get managedTestReleaseVoiceAction => '检查配音';

  @override
  String get managedTestReleaseDataAssetsTitle => 'DataAssets';

  @override
  String get managedTestReleaseDataAssetsDescription =>
      '已准备的 DataAsset 会显示在问题列表中，但尚无完整的全项目构建证据。';

  @override
  String get managedTestReleaseDataAssetsAction => '查看 DataAsset';

  @override
  String get managedTestReleasePlayableBuildTitle => '可游玩文件';

  @override
  String get managedTestReleasePlayableBuildDescription =>
      '从当前这一确切的已保存项目版本创建经过检查的可游玩构建。';

  @override
  String get managedTestReleasePlayableBuildBlockedReason =>
      '当前已保存版本尚无确切、完整的项目构建证据。';

  @override
  String get managedTestReleaseCreatePlayableFilesAction => '创建可游玩文件';

  @override
  String get managedTestReleaseDeploymentTitle => '安装';

  @override
  String get managedTestReleaseDeploymentDescription =>
      '将经过精确检查的可游玩构建安装到已配置的游戏中。';

  @override
  String get managedTestReleaseDeploymentBlockedReason =>
      '当前已保存项目版本尚无可部署构建的确切证据。';

  @override
  String get managedTestReleaseInstallAction => '安装';

  @override
  String managedProjectCommandBarCurrentSection(String section) {
    return '当前部分：$section';
  }

  @override
  String managedProjectCommandBarOrientationSemantics(
    String project,
    String section,
  ) {
    return '项目 $project。当前部分：$section。';
  }

  @override
  String get managedProjectCommandBarUndoLabel => '撤销';

  @override
  String get managedProjectCommandBarSearchLabel => '搜索';

  @override
  String get managedProjectCommandBarCreateLabel => '创建';

  @override
  String get managedProjectCommandBarProblemsLabel => '问题';

  @override
  String get managedProjectCommandBarHistoryLabel => '历史记录';

  @override
  String get managedProjectCommandBarSettingsLabel => '设置';

  @override
  String get managedProjectCommandBarMoreActionsTooltip => '更多项目操作';

  @override
  String get managedProjectCommandBarBusyLabel => '正在完成当前项目操作…';

  @override
  String get managedProjectCommandBarBusyDisabledReason => '请等待当前项目操作完成。';
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
  String get projectNewManagedRevision3 => '新建模组项目…';

  @override
  String get projectCreateGamePathRequired =>
      '创建模组项目前，请先在设置中指定 Gothic 1 Remake 路径。';

  @override
  String get projectCreateDirectoryPickerTitle => '在此创建托管模组项目';

  @override
  String projectManagedRevision3Created(String projectId) {
    return '已创建模组项目 $projectId';
  }

  @override
  String projectManagedRevision3CreateFailed(String error) {
    return '无法创建模组项目：$error';
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
  String get managedWorkspaceSettingsExpertLabel => '设置与专家工具';

  @override
  String get managedSectionStoryDescription => 'NPC、任务与对话。';

  @override
  String get managedSectionLocalizationVoiceDescription =>
      '在同一处编写和翻译项目对话，然后继续处理语音。';

  @override
  String get managedSectionSettingsExpertDescription =>
      '设置和只读 DataAsset Lab 现已可用。';

  @override
  String get managedSettingsExpertDataAssetLabLabel => 'DataAsset Lab';

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
  String get managedProjectLandingTitle => '开始模组项目';

  @override
  String get managedProjectLandingDescription => '创建项目、打开现有项目文件夹或从备份恢复项目。';

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

  @override
  String get managedWorkspaceTextVoiceLabel => '文本与配音';

  @override
  String get managedWorkspaceTestReleaseLabel => '测试与发布';

  @override
  String get managedTestReleaseTitle => '测试与发布';

  @override
  String get managedTestReleaseDescription => '创建或安装可游玩文件前，请检查模组的每个部分。';

  @override
  String get managedTestReleaseEvidenceBoundary =>
      '系统不会自动认定任何内容已就绪。检查结果仅适用于当前这一确切的已保存项目版本。';

  @override
  String get managedTestReleaseChecksHeading => '项目检查';

  @override
  String get managedTestReleaseReleaseHeading => '可游玩输出';

  @override
  String get managedTestReleaseStatusNotChecked => '未检查';

  @override
  String get managedTestReleaseStatusChecking => '检查中';

  @override
  String get managedTestReleaseStatusChecked => '已检查';

  @override
  String get managedTestReleaseStatusNeedsAttention => '需要注意';

  @override
  String get managedTestReleaseStatusBlocked => '已阻止';

  @override
  String get managedTestReleaseStatusNotAvailable => '不可用';

  @override
  String get managedTestReleaseStatusAvailable => '可用';

  @override
  String get managedTestReleaseEvidenceLabel => '证据';

  @override
  String get managedTestReleaseStaleEvidenceDescription =>
      '此结果属于其他项目版本。请重新运行检查。';

  @override
  String get managedTestReleaseActionNotConnectedDescription =>
      '已有证据，但此操作尚未连接到当前工作区。';

  @override
  String get managedTestReleaseProblemsHeading => '需要解决的问题';

  @override
  String get managedTestReleaseVoiceHeading => '配音构建检查';

  @override
  String get managedTestReleaseProjectStructureTitle => '项目结构';

  @override
  String get managedTestReleaseProjectStructureDescription =>
      '请在下方的当前问题列表中检查引用和托管项目结构。';

  @override
  String get managedTestReleaseProjectStructureAction => '查看问题';

  @override
  String get managedTestReleaseScriptsTitle => '脚本';

  @override
  String get managedTestReleaseScriptsDescription =>
      '对这个精确保存的项目版本中的所有脚本运行一次游戏编译器。结果仅作为检查证据；编译输出会被丢弃。';

  @override
  String get managedTestReleaseScriptsAction => '运行编译器检查';

  @override
  String get managedProjectCompilerRetryAction => '重试编译器检查';

  @override
  String get managedProjectCompilerReviewAction => '查看结果/再次检查';

  @override
  String get managedProjectCompilerDialogTitle => '检查所有项目脚本';

  @override
  String get managedProjectCompilerDialogIntroduction =>
      '开始前请关闭 Gothic 1 Remake。Mod Studio 会暂时使用游戏编译器检查所有项目脚本，恢复游戏安装，并丢弃全部编译输出。此结果不能创建可玩文件，也不能安装模组。';

  @override
  String get managedProjectCompilerCloseAction => '关闭';

  @override
  String get managedProjectCompilerNoGame =>
      '运行此检查前，请在设置中选择 Gothic 1 Remake 安装位置。';

  @override
  String get managedProjectCompilerSafetyBlocked =>
      '游戏安装尚未准备好进行编译器检查。请关闭游戏或解决恢复警告，然后重试。';

  @override
  String get managedProjectCompilerCompiled =>
      '所有项目脚本都已通过这个精确保存版本的检查。编译输出已被丢弃。';

  @override
  String get managedProjectCompilerEmpty => '这个保存的项目没有需要编译的脚本。空结果已得到精确检查。';

  @override
  String get managedProjectCompilerRejected =>
      '游戏编译器在一个或多个项目脚本中发现了问题。请修复下面的消息，然后重试。';

  @override
  String get managedProjectCompilerPreflightBlocked =>
      '编译器未启动。请关闭游戏并检查已配置的安装，然后重试。';

  @override
  String get managedProjectCompilerDrifted =>
      '项目或游戏输入发生了变化，或者最终核对不再精确。此结果已被丢弃；请为当前版本重新运行检查。';

  @override
  String get managedProjectCompilerRequiresReopen =>
      '再次进行精确编译器检查前，必须关闭并重新打开此项目。';

  @override
  String get managedProjectCompilerRecoveryRequired =>
      '无法确认私有编译器输出清理或游戏安装精确还原已完成。在新的安全检查成功之前，后续编译器检查和安装将保持阻止状态。';

  @override
  String get managedProjectCompilerFailed =>
      '无法完成或验证编译器检查。未保留任何结果；请在游戏安装准备好后重试。';

  @override
  String get managedProjectCompilerFailureDetails => '编译器消息';

  @override
  String get managedProjectCompilerDiagnosticsHeading => '编译器消息';

  @override
  String get managedProjectCompilerCaptureCaptured => '已捕获结构化编译器消息。';

  @override
  String get managedProjectCompilerCaptureFallback =>
      '诊断挂钩不可用，因此使用了普通游戏编译器作为回退。';

  @override
  String get managedProjectCompilerCaptureInvalid => '无法验证编译器消息捕获。';

  @override
  String get managedProjectCompilerCaptureUnavailable =>
      '编译器运行后诊断挂钩不可用；无需再次运行。';

  @override
  String get managedProjectCompilerCaptureExitUnconfirmed => '编译器进程未确认已经退出。';

  @override
  String get managedProjectCompilerCaptureDisabled => '本次运行没有可用的结构化编译器消息。';

  @override
  String get managedProjectCompilerSeverityError => '错误';

  @override
  String get managedProjectCompilerSeverityWarning => '警告';

  @override
  String get managedProjectCompilerSeverityNote => '备注';

  @override
  String get managedProjectCompilerFileLabel => '文件';

  @override
  String get managedProjectCompilerLineLabel => '行';

  @override
  String get managedProjectCompilerColumnLabel => '列';

  @override
  String get managedProjectCompilerOmittedDiagnostics => '条其他编译器消息已省略';

  @override
  String get managedTestReleaseVoiceTitle => '文本与配音';

  @override
  String get managedTestReleaseVoiceDescription => '请使用下方的配音构建检查来检查当前保存的项目版本。';

  @override
  String get managedTestReleaseVoiceAction => '检查配音';

  @override
  String get managedTestReleaseDataAssetsTitle => 'DataAssets';

  @override
  String get managedTestReleaseDataAssetsDescription =>
      '已准备的 DataAsset 会显示在问题列表中，但尚无完整的全项目构建证据。';

  @override
  String get managedTestReleaseDataAssetsAction => '查看 DataAsset';

  @override
  String get managedTestReleasePlayableBuildTitle => '可游玩文件';

  @override
  String get managedTestReleasePlayableBuildDescription =>
      '从当前这一确切的已保存项目版本创建经过检查的可游玩构建。';

  @override
  String get managedTestReleasePlayableBuildBlockedReason =>
      '当前已保存版本尚无确切、完整的项目构建证据。';

  @override
  String get managedTestReleaseCreatePlayableFilesAction => '创建可游玩文件';

  @override
  String get managedTestReleaseDeploymentTitle => '安装';

  @override
  String get managedTestReleaseDeploymentDescription =>
      '将经过精确检查的可游玩构建安装到已配置的游戏中。';

  @override
  String get managedTestReleaseDeploymentBlockedReason =>
      '当前已保存项目版本尚无可部署构建的确切证据。';

  @override
  String get managedTestReleaseInstallAction => '安装';

  @override
  String managedProjectCommandBarCurrentSection(String section) {
    return '当前部分：$section';
  }

  @override
  String managedProjectCommandBarOrientationSemantics(
    String project,
    String section,
  ) {
    return '项目 $project。当前部分：$section。';
  }

  @override
  String get managedProjectCommandBarUndoLabel => '撤销';

  @override
  String get managedProjectCommandBarSearchLabel => '搜索';

  @override
  String get managedProjectCommandBarCreateLabel => '创建';

  @override
  String get managedProjectCommandBarProblemsLabel => '问题';

  @override
  String get managedProjectCommandBarHistoryLabel => '历史记录';

  @override
  String get managedProjectCommandBarSettingsLabel => '设置';

  @override
  String get managedProjectCommandBarMoreActionsTooltip => '更多项目操作';

  @override
  String get managedProjectCommandBarBusyLabel => '正在完成当前项目操作…';

  @override
  String get managedProjectCommandBarBusyDisabledReason => '请等待当前项目操作完成。';
}
