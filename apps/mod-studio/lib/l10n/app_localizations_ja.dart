// ignore: unused_import
import 'package:intl/intl.dart' as intl;
import 'app_localizations.dart';

// ignore_for_file: type=lint

/// The translations for Japanese (`ja`).
class AppLocalizationsJa extends AppLocalizations {
  AppLocalizationsJa([String locale = 'ja']) : super(locale);

  @override
  String get tabItems => 'Items';

  @override
  String get tabOverrides => 'Changes';

  @override
  String get tabSettings => 'Settings';

  @override
  String get tabDialogs => '会話';

  @override
  String get tabAudio => 'オーディオ';

  @override
  String get tabTextures => 'テクスチャ';

  @override
  String get tabScripts => 'スクリプト';

  @override
  String get changesAll => 'すべて';

  @override
  String get sectionItemValues => 'アイテムの値';

  @override
  String get sectionLocalizedText => 'ローカライズテキスト';

  @override
  String get audioCatCreatures => 'クリーチャー';

  @override
  String get audioCatObjects => 'オブジェクト';

  @override
  String get audioCatMagic => '魔法';

  @override
  String get audioCatMovement => '移動';

  @override
  String get audioCatWorld => '世界';

  @override
  String get audioCatAction => 'アクション';

  @override
  String get audioCatCombat => '戦闘';

  @override
  String get audioCatPhysics => '物理';

  @override
  String get audioCatItems => 'アイテム';

  @override
  String get audioCatUi => 'UI';

  @override
  String get audioCatFoley => 'フォーリー';

  @override
  String get audioCatUnderwater => '水中';

  @override
  String get audioCatVision => 'ビジョン';

  @override
  String get audioCatDialog => '会話';

  @override
  String get audioCatOther => 'その他';

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
  String get extractLocalizedText => 'ローカライズされたテキストを抽出';

  @override
  String get lightMode => 'ライトモード';

  @override
  String get darkMode => 'ダークモード';

  @override
  String get language => '言語';

  @override
  String get exportMod => 'Modをエクスポート';

  @override
  String exportModWithCount(int count) {
    return 'Modをエクスポート（$count）';
  }

  @override
  String get selectAnItemToEdit => '編集するアイテムを選択してください。';

  @override
  String gameDataActiveTooltip(String name) {
    return 'ゲームデータ：$name';
  }

  @override
  String get gameDataBundledTooltip => 'ゲームデータ：同梱';

  @override
  String get loadGameDataDump => 'ゲームデータのダンプを読み込む…';

  @override
  String get loadGameDataDumpSubtitle => 'gore-dump Modの gore_game_data.json';

  @override
  String get useBundledData => '同梱データを使用';

  @override
  String get alreadyBundled => 'すでに同梱';

  @override
  String get gameDataFileGroupLabel => 'ゲームデータ';

  @override
  String get minimize => '最小化';

  @override
  String get restore => '元のサイズに戻す';

  @override
  String get maximize => '最大化';

  @override
  String get close => '閉じる';

  @override
  String get about => 'このアプリについて';

  @override
  String aboutVersion(String version, String sha) {
    return 'Version $version ($sha)';
  }

  @override
  String get aboutCopyright => '© 2026 GORE コントリビューター';

  @override
  String get aboutLicense => 'MIT ライセンスの下で提供されています。';

  @override
  String get categoryMeleeWeapons => '近接武器';

  @override
  String get categoryRangedWeapons => '遠距離武器';

  @override
  String get categoryAmmunition => '弾薬';

  @override
  String get categoryRunes => 'ルーン';

  @override
  String get categorySpellScrolls => '呪文の巻物';

  @override
  String get categoryFoodAndPotions => '食料・ポーション';

  @override
  String get categoryMiscellaneous => 'その他雑貨';

  @override
  String get categoryAmulets => 'アミュレット';

  @override
  String get categoryRings => '指輪';

  @override
  String get categoryAnimalTrophies => '動物のトロフィー';

  @override
  String get categoryWritings => '書物';

  @override
  String get categoryMissionItems => 'クエストアイテム';

  @override
  String get categoryKeys => '鍵';

  @override
  String get categoryOther => 'その他';

  @override
  String categoryWithCount(String label, int count) {
    return '$label（$count）';
  }

  @override
  String get searchItems => 'アイテムを検索';

  @override
  String get noItemsMatch => '一致するアイテムがありません';

  @override
  String failedToLoadCatalog(String error) {
    return 'カタログの読み込みに失敗しました：$error';
  }

  @override
  String pendingOverridesWithCount(int count) {
    return '保留中の変更（$count）';
  }

  @override
  String get clearAll => 'すべてクリア';

  @override
  String get noPendingOverrides => '保留中の変更はありません。\nアイテムのフィールドを編集して追加してください。';

  @override
  String get removeOverride => '変更を削除';

  @override
  String get searchChanges => '変更を検索';

  @override
  String get noChangesMatch => '一致する変更がありません';

  @override
  String get clearSection => 'このグループをクリア';

  @override
  String get modName => 'Mod名';

  @override
  String get loadDelayLabel => '読み込み遅延（ミリ秒、0 = 即時）';

  @override
  String get noFolderSelected => 'フォルダが選択されていません';

  @override
  String get chooseFolder => 'フォルダを選択';

  @override
  String get packageAsZip => '.zip形式でパッケージ化';

  @override
  String get cancel => 'キャンセル';

  @override
  String get export => 'エクスポート';

  @override
  String get exportHere => 'ここにエクスポート';

  @override
  String get mustBeNonNegativeInteger => '0以上の整数を入力してください';

  @override
  String get extractingLocalizedText => 'ローカライズされたゲームテキストを抽出中…';

  @override
  String get localizedTextExtractionCancelled => 'ローカライズテキストの抽出をキャンセルしました。';

  @override
  String get localizedTextExtracted => 'ローカライズテキストを抽出しました。';

  @override
  String get extractionFailed => '抽出に失敗しました。';

  @override
  String get localizationCacheFileGroupLabel => 'ローカライズキャッシュ';

  @override
  String get extractLocalizedTextQuestion => 'ローカライズされたゲームテキストを抽出しますか？';

  @override
  String get extractLocalizedTextBody =>
      'ローカライズされたゲームテキストはまだ抽出されていません。インストール済みのゲームから今すぐ抽出しますか？（任意）';

  @override
  String get notNow => '後で';

  @override
  String get extract => '抽出';

  @override
  String get validationRequired => '必須です';

  @override
  String get validationMustBeWholeNumber => '整数で入力してください';

  @override
  String get validationMustBeNumber => '数値で入力してください';

  @override
  String get validationMustBeFinite => '有限の数値で入力してください';

  @override
  String validationMustBeAtLeast(String min) {
    return '$min 以上である必要があります';
  }

  @override
  String validationMustBeAtMost(String max) {
    return '$max 以下である必要があります';
  }

  @override
  String get validationMustBeBool => 'true または false である必要があります';

  @override
  String validationMustBeOneOf(String options) {
    return '次のいずれかである必要があります：$options';
  }

  @override
  String get modNameRequired => '必須です';

  @override
  String get modNameControlCharacters => '制御文字を含めることはできません';

  @override
  String get modNamePathSeparators => 'パス区切り文字を含めることはできません';

  @override
  String get modNameNotAFolderName => '有効なフォルダ名ではありません';

  @override
  String localizedTextExtractedCount(int idCount, int languageCount) {
    return '$languageCount言語で$idCount個のIDを抽出しました';
  }

  @override
  String get managerDeployActive =>
      'mod-manager のロードアウトが有効です。先に gore-manager で undeploy してください。';

  @override
  String get projectOpenLegacy => 'Open legacy project…';

  @override
  String get projectOpenManagedRevision3 => 'Open Mod Studio project…';

  @override
  String get projectVerifyCurrentHead => 'Verify current head';

  @override
  String get projectManagedRevision3Title => 'Mod Studio project';

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
  String get projectManagedRevision3Opened => 'Mod Studio project opened.';

  @override
  String projectManagedRevision3OpenFailed(String error) {
    return 'Mod Studio project could not be opened: $error';
  }

  @override
  String get projectManagedRevision3Verified => 'Project checkpoint verified.';

  @override
  String projectManagedRevision3VerifyFailed(String error) {
    return 'Project checkpoint could not be verified: $error';
  }

  @override
  String get projectManagedRevision3RequiresReopen =>
      'Exact-head verification could not complete safely. This session now requires recovery and further verification is blocked. Close Mod Studio, then reopen this project before continuing.';

  @override
  String get projectManagedRevision3VerifyBlocked =>
      'Verification is blocked until the managed project is reopened.';

  @override
  String get projectTransitionCleanupWarning =>
      '新しいプロジェクトは開いていますが、以前のプロジェクトセッションを完全にクリーンアップできませんでした。クリーンアップは再試行されません。以前のプロジェクトを再度開く前に Mod Studio を再起動してください。';

  @override
  String get projectNewManagedRevision3 => '新しい管理対象 Mod プロジェクト…';

  @override
  String get projectNewLegacy => '新しいレガシープロジェクト';

  @override
  String get projectCreateGamePathRequired =>
      'Mod プロジェクトを作成する前に、設定で Gothic 1 Remake のパスを指定してください。';

  @override
  String get projectCreateDirectoryPickerTitle => 'ここに管理対象 Mod プロジェクトを作成';

  @override
  String projectManagedRevision3Created(String projectId) {
    return '管理対象 Mod プロジェクト $projectId を作成しました';
  }

  @override
  String projectManagedRevision3CreateFailed(String error) {
    return '管理対象 Mod プロジェクトを作成できませんでした: $error';
  }

  @override
  String get projectCreateDialogTitle => 'Mod プロジェクトを作成';

  @override
  String get projectCreateNameLabel => 'プロジェクト名';

  @override
  String get projectCreateNameHelper => 'Mod Studio に表示される名前です。';

  @override
  String get projectCreateVersionLabel => 'バージョン';

  @override
  String get projectCreateVersionHelper => '0.1.0 などの初期バージョンです。';

  @override
  String get projectCreateAuthorLabel => '作者';

  @override
  String get projectCreateAuthorHelper => 'あなた、または Mod チームの名前です。';

  @override
  String get projectCreateLocalesLabel => '編集言語';

  @override
  String get projectCreateLocalesHelper =>
      '正規化されたタグをカンマで区切ります。例: en, de, en-US。';

  @override
  String get projectCreateBoundary =>
      '空の管理対象オフラインプロジェクトを作成します。Mod のビルド、配置、実行は行わず、ゲームファイルやセーブファイルも変更しません。';

  @override
  String get projectCreateSubmit => 'プロジェクトを作成';

  @override
  String projectCreateMetadataRequired(String label) {
    return '$label は必須です。';
  }

  @override
  String projectCreateMetadataNoOuterWhitespace(String label) {
    return '$label の先頭と末尾に空白は使用できません。';
  }

  @override
  String projectCreateMetadataControlCharacters(String label) {
    return '$label に制御文字は使用できません。';
  }

  @override
  String projectCreateMetadataMalformed(String label) {
    return '$label に不正なテキストが含まれています。';
  }

  @override
  String projectCreateMetadataTooLong(String label, int maxBytes) {
    return '$label が UTF-8 の上限 $maxBytes バイトを超えています。';
  }

  @override
  String get projectCreateLocalesRequired => '編集言語を1つ以上入力してください。';

  @override
  String get projectCreateLocalesEmptyEntry => '空の言語エントリを削除してください。';

  @override
  String projectCreateLocalesTooMany(int maxLocales) {
    return '編集言語は最大 $maxLocales 個です。';
  }

  @override
  String projectCreateLocaleBoundedAscii(String locale) {
    return 'ロケール「$locale」は長さ制限内の ASCII である必要があります。';
  }

  @override
  String projectCreateLocaleLanguage(String locale) {
    return 'ロケール「$locale」の言語は2～8文字の小文字で指定してください。';
  }

  @override
  String projectCreateLocaleInvalidSegment(String locale) {
    return 'ロケール「$locale」に無効なセグメントがあります。';
  }

  @override
  String projectCreateLocaleNotCanonical(String locale, String canonical) {
    return 'ロケール「$locale」は正規形ではありません。「$canonical」を使用してください。';
  }

  @override
  String get managedWorkspaceOverviewLabel => '概要';

  @override
  String get managedWorkspaceContentLabel => 'コンテンツ';

  @override
  String get managedWorkspaceDataAssetsLabel => 'DataAssets';

  @override
  String get managedContentWorkspaceLibraryLabel => 'このMOD';

  @override
  String get managedWorkspaceHomeLabel => 'ホーム';

  @override
  String get managedWorkspaceStoryLabel => 'ストーリー';

  @override
  String get managedWorkspaceWorldLabel => 'ワールド';

  @override
  String get managedWorkspaceLocalizationVoiceLabel => 'ローカライズと音声';

  @override
  String get managedWorkspaceValidateTestLabel => '検証とテスト';

  @override
  String get managedWorkspaceBuildReleaseLabel => 'ビルドとリリース';

  @override
  String get managedWorkspaceHistoryLabel => 'History';

  @override
  String get managedWorkspaceSettingsExpertLabel => '設定とエキスパート';

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
  String get managedSectionStoryDescription => 'NPC、クエスト、会話。';

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
  String get managedSectionWorldDescription => 'ワールドへの配置と関連ワークフローは今後対応予定です。';

  @override
  String get managedSectionLocalizationVoiceDescription =>
      'プロジェクトの台詞の作成と翻訳を一か所で行い、そのまま音声作業へ進めます。';

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
      'You changed this project text. Switching now would discard those edits.';

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
  String get managedSectionValidateTestDescription =>
      'プロジェクトの完全性とチェックポイントを厳密に検証します。実行時テストを保証するものではありません。';

  @override
  String get managedSectionBuildReleaseDescription =>
      '音声バンドルは利用できますが、完全にプレイ可能なビルドとデプロイは利用できません。';

  @override
  String get managedSectionSettingsExpertDescription =>
      '設定は利用できますが、エキスパートツールはまだ統合されていません。';

  @override
  String get managedSectionStatusHeading => '状態';

  @override
  String get managedSectionActionsHeading => '操作';

  @override
  String get managedCapabilityAvailable => '利用可能';

  @override
  String get managedCapabilityPartial => '一部利用可能';

  @override
  String get managedCapabilityPlanned => '対応予定';

  @override
  String get managedCapabilityUnavailable => '利用不可';

  @override
  String get managedProjectSubtitle => '現在の正確なバージョンに対応するオフライン制作ワークスペース';

  @override
  String get managedProjectLandingTitle => '管理プロジェクトのワークスペース';

  @override
  String get managedProjectLandingDescription =>
      'ホーム、コンテンツ、ストーリー、音声、検証、リリースの新しい制作フローを、1つの管理プロジェクトで利用できます。';

  @override
  String get legacyCompatibilityToolsTitle => '従来版の互換ツール';

  @override
  String get legacyCompatibilityToolsDescription =>
      '下のタブは、以前からある直接置換用のツールです。管理プロジェクトのワークスペースを拡充している間も引き続き利用できます。';

  @override
  String get managedProjectTechnicalDetails => 'プロジェクトの技術的な詳細';

  @override
  String get managedProjectRecoveryContentLocked =>
      '内容を読み取る前に、管理対象プロジェクトを開き直してください。';

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
  String get managedDashboardUntitledProject => '無題のプロジェクト';

  @override
  String get managedDashboardDraftStatus => '下書き';

  @override
  String get managedDashboardProjectVersion => 'バージョン';

  @override
  String get managedDashboardProjectAuthor => '作成者';

  @override
  String get managedDashboardNotProvided => '未指定';

  @override
  String get managedDashboardContentCounts => 'プロジェクトの内容';

  @override
  String get managedDashboardNpcDrafts => 'NPCの下書き';

  @override
  String get managedDashboardQuestDrafts => 'クエストの下書き';

  @override
  String get managedDashboardDialogLines => 'ダイアログ行';

  @override
  String get managedDashboardVoiceTakes => 'ボイステイク';

  @override
  String get managedDashboardAssets => 'アセット';

  @override
  String get managedDashboardUnresolvedReferences => '未解決の参照';

  @override
  String get managedDashboardReadiness => '現在利用できる機能';

  @override
  String get managedDashboardOfflineAuthoringTitle => 'オフライン制作が利用可能';

  @override
  String get managedDashboardOfflineAuthoringDescription =>
      'ゲームのインストールやセーブファイルを変更せずに、対応しているプロジェクト内容を作成・編集できます。';

  @override
  String get managedDashboardGeneralBuildBlockedTitle => '一般的なModビルドは利用不可';

  @override
  String get managedDashboardGeneralBuildBlockedDescription =>
      'ビルドできるのは封印済みのオフラインVoiceバンドルのみです。完全にプレイ可能なModはまだビルドできません。';

  @override
  String get managedDashboardRuntimeUnqualifiedTitle => '実行時の検証は未完了';

  @override
  String get managedDashboardRuntimeUnqualifiedDescription =>
      'Mod Studioでは、このプロジェクト内容が実行中のゲーム内で動作することをまだ確認していません。';

  @override
  String get managedDashboardReferenceIntegrityTitle => '参照整合性';

  @override
  String get managedDashboardReferenceIntegrityDescription =>
      'この件数で確認するのはプロジェクト内の参照だけで、ビルドや実行の準備状況を示すものではありません。';

  @override
  String get managedDashboardMissingGameTitle => 'ゲームの設定が必要';

  @override
  String get managedDashboardMissingGameDescription =>
      'インストール済みゲームの検証済み情報が必要な操作を行う前に、設定でGothic 1 Remakeのインストール先を指定してください。';

  @override
  String get managedDashboardCreateHeading => '作成';

  @override
  String get managedDashboardToolsHeading => 'プロジェクトツール';

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
  String get managedHomeBuildTitle => 'Create output';

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
  String get managedDashboardLoading => 'プロジェクト概要を読み込み中';

  @override
  String get managedDashboardLoadError => 'プロジェクト概要を利用できません';

  @override
  String get managedDashboardLoadErrorDescription =>
      '検証済みのプロジェクト概要を読み込めませんでした。プロジェクト内容は変更されていません。';

  @override
  String get managedDashboardRetry => '再試行';

  @override
  String get managedActionNewNpcTitle => '新しいNPC';

  @override
  String get managedActionNewNpcDescription =>
      'インストール済みゲームの検証済み情報から、範囲を限定したオフラインNPC下書きを作成します。';

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
  String get managedActionNewQuestTitle => '新しいクエスト';

  @override
  String get managedActionNewQuestDescription =>
      '目標と検証済みの親IDを含むオフラインのクエスト下書きを作成します。';

  @override
  String get managedQuestOpeningRecipeTitle => 'クエスト＋冒頭の会話行';

  @override
  String get managedQuestOpeningRecipeDescription =>
      '推奨：クエストの下書きを作成し、最初のローカライズ済み会話行を記述して挿入します。このフローではプロジェクトのチェックポイントを2つ使用しますが、プレイ可能な会話は作成されません。';

  @override
  String get managedQuestOpeningRecipeIntroduction =>
      'このガイドフローでは、まずクエストを保存し、次にその最初の会話行を開きます。手順1の後で中止しても、クエストは保存されたままです。プレイ可能な会話は作成されず、ゲームやセーブファイルも変更されません。';

  @override
  String get managedQuestOpeningRecipeStart => 'ガイド付きクエストを開始';

  @override
  String get managedQuestOpeningLineTitle => '手順2/2：冒頭の会話行';

  @override
  String get managedQuestOpeningLineIntroduction =>
      'このクエストの最初のローカライズ済み会話行を記述します。保存すると行とそのテキストが作成され、クエストのトランスクリプトの先頭に挿入されます。';

  @override
  String managedQuestOpeningRecipePreparing(int projectRevision) {
    return 'クエストをプロジェクトリビジョン $projectRevision に保存しました。冒頭の会話行を準備しています...';
  }

  @override
  String managedQuestOpeningRecipePartial(int projectRevision) {
    return 'クエストをプロジェクトリビジョン $projectRevision に保存しましたが、冒頭の会話行は追加されていません。ストーリー > 会話と音声 から続行してください。';
  }

  @override
  String get managedQuestOpeningRecipeFailed =>
      'ガイド付きクエストを開始できませんでした。プロジェクトの変更は公開されていません。';

  @override
  String get managedQuestOpeningRecipeStopped =>
      '正確な現在のプロジェクト状態が変化したため、ガイドフローを停止しました。これ以降の手順は自動実行されません。ストーリーを確認し、手動で続行してください。';

  @override
  String get managedQuestOpeningRecipeRequiresReopen =>
      'ガイドフローを安全に続行できませんでした。このプロジェクトを開き直し、ストーリーを確認してから、再試行または手動で続行してください。';

  @override
  String managedQuestOpeningRecipeComplete(int projectRevision) {
    return 'クエストと冒頭の会話行をプロジェクトリビジョン $projectRevision に保存しました。下書きのみ：プレイ可能な会話は作成されず、ゲームやセーブファイルも変更されていません。';
  }

  @override
  String get managedActionNewDialogLineTitle => 'ダイアログ行を追加';

  @override
  String get managedActionNewDialogLineDescription =>
      'ローカライズしたプロジェクトテキストを作成するか、このプロジェクト内の未使用テキストを関連付けます。再生可能なダイアログトピックは作成されません。';

  @override
  String managedActionNewDialogLineSaved(int projectRevision) {
    return 'ダイアログ行をプロジェクトリビジョン $projectRevision に保存しました。ゲームとセーブファイルは変更されていません。';
  }

  @override
  String get managedDialogLineIntroduction =>
      '新しいローカライズ済みダイアログ行を作成するか、このプロジェクトに既に属するテキストを関連付けます。';

  @override
  String get managedDialogLineBoundary =>
      '変更されるのはプロジェクトファイルだけです。AngelScriptトピックや再生可能なダイアログは作成されず、ゲームのインストールやセーブファイルも変更されません。話者欄は単なるラベルで、NPCとは関連付けられません。';

  @override
  String get managedDialogLineCreateMode => '新しいテキストを書く';

  @override
  String get managedDialogLineReuseMode => 'プロジェクトテキストを使用';

  @override
  String get managedDialogLineNameLabel => '行の名前';

  @override
  String get managedDialogLineNameHint => '鉱山入口での挨拶';

  @override
  String get managedDialogLineSpeakerLabel => '話者ラベル（任意）';

  @override
  String get managedDialogLineSpeakerHint => '例：Viper';

  @override
  String get managedDialogLineLocaleLabel => '言語';

  @override
  String get managedDialogLineTextLabel => 'ダイアログテキスト';

  @override
  String get managedDialogLineReuseSearch => '未使用のプロジェクトテキストを検索';

  @override
  String get managedDialogLineNoReusableText =>
      '関連付け可能な、未使用で構造的に有効なプロジェクトテキストはありません。代わりに新しいテキストを書いてください。';

  @override
  String get managedDialogLineCreateSlotLabel => 'この言語のVoiceを準備';

  @override
  String get managedDialogLineCreateSlotHelp =>
      'プロジェクトに未解決の空のVoiceスロットを作成します。録音の追加や配置は行いません。';

  @override
  String get managedDialogLineCancel => 'キャンセル';

  @override
  String get managedDialogLineSave => 'プロジェクトに保存';

  @override
  String get managedDialogLineSaving => '保存中…';

  @override
  String get managedDialogLineLoading => 'プロジェクトの正確な内容を読み取り中…';

  @override
  String get managedDialogLineLoadFailed =>
      '現在のプロジェクトの正確な内容を読み取れませんでした。変更はありません。';

  @override
  String get managedDialogLineRetry => '再試行';

  @override
  String get managedDialogLineStale =>
      'このウィンドウを開いている間にプロジェクトが変更されました。閉じて、現在のプロジェクトから再試行してください。';

  @override
  String get managedDialogLineRequiresReopen =>
      '現在のプロジェクトを安全に検証できなくなりました。このウィンドウを閉じて、管理対象プロジェクトを開き直してください。';

  @override
  String get managedDialogLineInvalidInput =>
      '強調表示されたプロジェクト入力を確認し、現在の正確な項目を選択してください。';

  @override
  String get managedDialogLineSaveFailed =>
      'ダイアログ行を安全に保存できませんでした。ゲームとセーブファイルは変更されていません。';

  @override
  String get managedDialogLineDone => '完了';

  @override
  String get managedDialogLineAddRecording => '録音を追加';

  @override
  String get managedActionAddVoiceTakeTitle => 'ボイステイクを追加';

  @override
  String get managedActionAddVoiceTakeDescription =>
      '配布せずにOgg Vorbis録音をこのプロジェクトへインポートします。';

  @override
  String get managedActionAddVoiceTakeRequiresDialogLine =>
      'Create or repair a dialog line with one valid localization entry before using Voice tools.';

  @override
  String get managedActionManageVoiceTakesTitle => 'ボイステイクを管理';

  @override
  String get managedActionManageVoiceTakesDescription =>
      'テイクを確認し、Voiceスロット用に承認済みの録音を選択します。';

  @override
  String get managedActionResolveVoiceTargetTitle => 'Voiceターゲットを特定';

  @override
  String get managedActionResolveVoiceTargetDescription =>
      'ゲームを変更せずに、プロジェクトのVoiceスロットをインストール済みアーカイブの正確なメンバーと照合します。';

  @override
  String get managedActionBuildVoiceBundleTitle => 'Voiceバンドルをビルド';

  @override
  String get managedActionBuildVoiceBundleDescription =>
      '既存メンバーで構成される封印済みオフラインバンドルをビルドします。配布は行いません。';

  @override
  String get managedActionDataAssetsTitle => 'DataAssetの編集';

  @override
  String get managedActionDataAssetsDescription =>
      'インストール済みパッケージを調べ、検証済みの固定幅の値編集をプロジェクトに準備します。';

  @override
  String get managedActionBrowseProjectContentDescription =>
      'プロジェクトの正確な内容と、解決済みまたは未解決の参照を確認します。';

  @override
  String get managedActionSettingsTitle => '設定';

  @override
  String get managedActionSettingsDescription =>
      'Gothic 1 Remakeのインストール先とMod Studioの環境設定を構成します。';

  @override
  String projectStarterSetupOpenFailed(String projectId) {
    return 'プロジェクト$projectIdは安全に作成されましたが、スターター設定を開けませんでした。有効な空のプロジェクトが現在のままです。';
  }

  @override
  String projectStarterOutcomeUnverified(String projectId) {
    return 'プロジェクト$projectIdは作成されましたが、Mod Studioはスターターの結果を検証できません。続行前に管理対象プロジェクトを開き直してください。ゲームとセーブデータは変更されていません。';
  }

  @override
  String projectStarterNpcCancelled(String projectId) {
    return 'プロジェクト$projectIdを作成しました。NPCスターターは追加されず、有効な空のプロジェクトが現在のままです。';
  }

  @override
  String projectStarterNpcSaved(int projectRevision) {
    return 'NPCスターターをプロジェクトリビジョン$projectRevisionに保存しました。ビルドはブロックされ、実行時未検証で、スポーンされません。';
  }

  @override
  String projectStarterQuestCancelled(String projectId) {
    return 'プロジェクト$projectIdを作成しました。クエストスターターは追加されず、有効な空のプロジェクトが現在のままです。';
  }

  @override
  String projectStarterQuestSaved(int projectRevision) {
    return 'クエストスターターをプロジェクトリビジョン$projectRevisionに保存しました。ビルドはブロックされ、実行時未検証です。';
  }

  @override
  String get projectStarterSemanticsLabel => 'プロジェクトスターター';

  @override
  String get projectStarterPrompt => 'どのように開始しますか？';

  @override
  String get projectStarterWriteBoundary =>
      'スターターを選んでも書き込みは行われません。このフォームを送信して空のフォルダーを選んだ後にのみプロジェクトが作成されます。';

  @override
  String get projectStarterEmptyTitle => '空のプロジェクト';

  @override
  String get projectStarterEmptyDescription =>
      '管理対象プロジェクトだけを作成します。準備ができたら内容を追加できます。';

  @override
  String get projectStarterNpcDraftTitle => 'NPC下書き';

  @override
  String get projectStarterNpcDraftDescription =>
      '先に空のプロジェクトを作成し、既存のNPC下書きガイド設定を開きます。';

  @override
  String get projectStarterQuestDraftTitle => 'クエスト下書き';

  @override
  String get projectStarterQuestDraftDescription =>
      '先に空のプロジェクトを作成し、既存のクエスト下書きガイド設定を開きます。';

  @override
  String get projectStarterPartialOutcome =>
      'NPCまたはクエストのガイド設定をキャンセルした場合や下書きに失敗した場合も、有効な空のプロジェクトが残ります。選択によってゲームやセーブへ書き込まれることはありません。';

  @override
  String get managedContentWorkspaceBrowseLabel => '参照';

  @override
  String get managedContentWorkspaceVerifiedEditsLabel => '検証済み編集';

  @override
  String get managedContentScopeBaseGameLabel => 'ベースゲーム';

  @override
  String get managedContentScopeInstalledLabel => 'インストール済み';

  @override
  String get managedBaseGameBrowserTitle => '対応するベースゲームの開始点';

  @override
  String get managedBaseGameBrowserDescription =>
      'Mod Studioが現在検査できる、または安全な下書きの開始点として使える、インストール済みゲームの正確な証拠を参照します。オリジナル内容の完全なカタログではありません。';

  @override
  String get managedBaseGameBrowserLoading => 'ベースゲームの正確な証拠を読み取り中…';

  @override
  String get managedBaseGameBrowserRefresh => '新しい正確なカタログを読み取る';

  @override
  String get managedBaseGameBrowserSearchLabel => '対応するベースゲーム内容を検索';

  @override
  String get managedBaseGameBrowserFilterNpcs => 'NPC';

  @override
  String get managedBaseGameBrowserFilterQuests => 'クエスト';

  @override
  String get managedBaseGameBrowserNpcSectionTitle => 'NPCの開始点';

  @override
  String get managedBaseGameBrowserQuestSectionTitle => 'クエストの開始点';

  @override
  String get managedBaseGameBrowserExperimentalNpcSectionTitle =>
      '検査専用NPCアーキタイプ';

  @override
  String get managedBaseGameBrowserSearchForExperimental =>
      '検索すると、より広い静的リンクのNPC証拠も含まれます。これらの行から下書きは作成できません。';

  @override
  String get managedBaseGameBrowserEmpty => 'この検索に一致する対応済みベースゲーム結果はありません。';

  @override
  String get managedBaseGameBrowserLoadErrorTitle => 'ベースゲームの証拠を利用できません';

  @override
  String get managedBaseGameBrowserLoadErrorDescription =>
      '対応する正確なカタログを読み取れませんでした。プロジェクト、ゲーム、セーブのファイルは変更されていません。';

  @override
  String get managedBaseGameBrowserOfflineDraftBadge => 'オフライン下書き対応';

  @override
  String get managedBaseGameBrowserInspectOnlyBadge => '検査のみ';

  @override
  String get managedBaseGameBrowserCreateNpcDraft => 'NPCの開始点として使用';

  @override
  String get managedBaseGameBrowserCreateQuestDraft => 'クエストの開始点として使用';

  @override
  String get managedBaseGameBrowserSpawnClass => 'スポーン定義';

  @override
  String get managedBaseGameBrowserActorBlueprint => 'アクターBlueprint';

  @override
  String get managedBaseGameBrowserExperimentalResultsCapped =>
      '検査専用の一致を先頭100件表示しています。より具体的な結果には検索を絞り込んでください。';

  @override
  String get managedInstalledBrowserLoading => 'インストール済みパッケージの正確な一覧を読み取り中…';

  @override
  String managedInstalledBrowserCompleteSummary(int count) {
    return 'インストール済みパッケージ候補：$count件';
  }

  @override
  String managedInstalledBrowserPartialSummary(int count) {
    return 'インストール済みパッケージ候補：$count件 — 部分結果';
  }

  @override
  String get managedInstalledBrowserCompleteDescription =>
      'ディレクトリのメタデータを読み取り、インストール済みスナップショットの正確性を維持しました。';

  @override
  String get managedInstalledBrowserPartialDescription =>
      '一部のパッケージメタデータが欠落または非正規形式のため、結果は探索には使えますが完全ではありません。';

  @override
  String get managedInstalledBrowserAuthorityNotice =>
      'この範囲ではインストール済みDataAssetパッケージのメタデータだけを表示します。パスの検査やコピーによって、ビルド、配布、実行、ゲームへの書き込み権限が与えられることはありません。';

  @override
  String get managedInstalledBrowserRefresh => '新しい正確なスナップショットを読み取る';

  @override
  String get managedInstalledBrowserSearchLabel => 'インストール済みDataAssetを検索';

  @override
  String get managedInstalledBrowserSearchHint => 'アセット名または/Gameパス';

  @override
  String get managedInstalledBrowserSearchPrompt =>
      '検索するアセット名または/Gameパスを入力してください。';

  @override
  String get managedInstalledBrowserNoMatchesTitle =>
      '一致するインストール済みDataAssetはありません';

  @override
  String get managedInstalledBrowserNoMatchesDescription =>
      '別のアセット名またはより広い/Gameパスを試してください。';

  @override
  String get managedInstalledBrowserResultLimitDescription =>
      '先頭100件を表示しています。検索を絞り込み、正確なスナップショットを限定してください。';

  @override
  String get managedInstalledBrowserKindBadge => 'DataAssetパッケージ';

  @override
  String get managedInstalledBrowserMetadataOnlyBadge => 'メタデータのみ';

  @override
  String get managedInstalledBrowserOpenInspector => '正確なパッケージを検査';

  @override
  String get managedInstalledBrowserErrorTitle => 'インストール済みパッケージ一覧を利用できません';

  @override
  String get managedInstalledBrowserErrorDescription =>
      'インストール済みの正確なスナップショットを読み取れませんでした。プロジェクト、ゲーム、セーブのファイルは変更されていません。';

  @override
  String get managedGlobalSearchScopeLabel => 'すべて検索';

  @override
  String get managedGlobalSearchTitle => 'すべてのコンテンツを検索';

  @override
  String get managedGlobalSearchLabel => 'NPC、クエスト、セリフ、アセット、ID、または /Game パス';

  @override
  String get managedGlobalSearchAction => '検索';

  @override
  String get managedGlobalSearchClear => 'クリア';

  @override
  String get managedGlobalSearchPrompt => '検索語を入力すると、3つのソースを個別に読み取ります。';

  @override
  String get managedGlobalSearchNoResults => 'このソースには一致する項目がありません。';

  @override
  String get managedGlobalSearchLoading => '正確なソースを読み取り中…';

  @override
  String get managedGlobalSearchFailed => 'このソースを読み取れませんでした。';

  @override
  String get managedGlobalSearchComplete => '完了';

  @override
  String get managedGlobalSearchPartial => '一部';

  @override
  String get managedGlobalSearchTruncated => '最初の100件を表示しています。検索を絞り込んでください。';

  @override
  String get managedGlobalSearchOpen => '開く';

  @override
  String get managedGlobalSearchCreateDraft => '下書きを作成';

  @override
  String get managedGlobalSearchInspect => '検査';

  @override
  String get managedGlobalSearchKindModEntity => 'Modコンテンツ';

  @override
  String get managedGlobalSearchKindModAsset => 'Modアセット';

  @override
  String get managedGlobalSearchKindBaseNpc => 'NPCの開始点';

  @override
  String get managedGlobalSearchKindBaseQuest => 'クエストの開始点';

  @override
  String get managedGlobalSearchKindExperimentalNpc => 'NPCの証拠';

  @override
  String get managedGlobalSearchReadinessExact => '正確な現在のプロジェクト';

  @override
  String get managedGlobalSearchReadinessProblems => '正確（問題あり）';

  @override
  String get managedGlobalSearchResultStale =>
      'この結果は現在のプロジェクトに存在しません。もう一度検索してください。';

  @override
  String get managedStoryWorkbenchDraftBadge => '下書きのみ';

  @override
  String get managedStoryWorkbenchBuildBlockedBadge => 'ビルド不可';

  @override
  String get managedStoryWorkbenchRuntimeUnqualifiedBadge => 'ランタイム未検証';

  @override
  String get managedStoryWorkbenchOverviewTab => '概要';

  @override
  String get managedStoryWorkbenchProfileTab => 'プロフィール';

  @override
  String get managedStoryWorkbenchStoryTab => 'ストーリー';

  @override
  String get managedStoryWorkbenchLogicTab => 'ロジック';

  @override
  String get managedStoryWorkbenchRoutineTab => 'ルーチン';

  @override
  String get managedStoryWorkbenchInventoryTab => 'インベントリ';

  @override
  String get managedStoryWorkbenchDialogVoiceTab => '会話と音声';

  @override
  String get managedStoryWorkbenchReferencesTab => '参照関係';

  @override
  String get managedStoryWorkbenchProblemsChecksTab => '問題とチェック';

  @override
  String get managedStoryWorkbenchEditOverview => '名前と目標を編集';

  @override
  String get managedStoryWorkbenchEditStory => '説明とつながりを編集';

  @override
  String get managedStoryWorkbenchEditLogic => '状態と遷移を編集';

  @override
  String get managedStoryWorkbenchInspectQuest => 'ソースとコンパイラーチェックを開く';

  @override
  String get managedStoryWorkbenchInspectNpc => 'プロフィールとコンパイラーチェックを開く';

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
  String get managedStoryWorkbenchCapabilityUnavailable => 'まだモデル化されていません';

  @override
  String get managedStoryWorkbenchNpcStoryUnavailable =>
      'クエストやストーリーとの関係は、NPCの下書きではまだモデル化されていません。';

  @override
  String get managedStoryWorkbenchNpcRoutineUnavailable =>
      'ルーチンとワールド内の配置はまだモデル化されていません。';

  @override
  String get managedStoryWorkbenchNpcInventoryUnavailable =>
      'インベントリ、装備、取引はまだモデル化されていません。';

  @override
  String get managedStoryWorkbenchNpcDialogVoiceUnavailable =>
      '会話、ローカライズ、音声との関係は、NPCの下書きではまだモデル化されていません。';

  @override
  String get managedStoryWorkbenchQuestDialogVoiceUnavailable =>
      '会話、ローカライズ、音声との関係は、クエストの下書きではまだモデル化されていません。';

  @override
  String get managedStoryWorkbenchNoReferenceProblems => '未解決のプロジェクト参照はありません';

  @override
  String managedStoryWorkbenchReferenceProblemCount(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '未解決のプロジェクト参照が$count件あります',
      one: '未解決のプロジェクト参照が1件あります',
    );
    return '$_temp0';
  }

  @override
  String get managedStoryWorkbenchReferenceScopeNotice =>
      '参照状態のみを示します。ビルドや実行の準備完了を示すものではありません。';

  @override
  String get managedStoryWorkbenchTechnicalDetails => '技術情報';

  @override
  String get managedStoryWorkbenchQuestKindLabel => 'クエストの下書き';

  @override
  String get managedStoryWorkbenchNpcKindLabel => 'NPCの下書き';

  @override
  String get managedStoryWorkbenchQuestTitleLabel => 'クエスト名';

  @override
  String get managedStoryWorkbenchTechnicalIdLabel => '技術ID';

  @override
  String get managedStoryWorkbenchObjectivesLabel => '目標';

  @override
  String get managedStoryWorkbenchUniqueNameLabel => '一意の名前';

  @override
  String get managedStoryWorkbenchModuleNamespaceLabel => 'モジュール名前空間';

  @override
  String get managedStoryWorkbenchQuestGiverLabel => 'クエスト提供者';

  @override
  String get managedStoryWorkbenchRuntimeParentLabel => 'ランタイム親クラス';

  @override
  String get managedStoryWorkbenchLogicDescription =>
      'クエストのライフサイクル状態、トリガー、条件、効果を、正確な現在の状態に対する単一のアトミック操作として編集します。';

  @override
  String get managedStoryWorkbenchOutgoingHeading => '参照先';

  @override
  String get managedStoryWorkbenchNoOutgoingReferences => '予測される参照先はありません';

  @override
  String get managedStoryWorkbenchIncomingHeading => '参照元';

  @override
  String get managedStoryWorkbenchNoIncomingReferences => 'プロジェクト内に参照元はありません';

  @override
  String get managedStoryWorkbenchSemanticIdentityLabel => '意味上の識別情報';

  @override
  String get managedStoryWorkbenchOriginLabel => 'オリジン';

  @override
  String get managedStoryWorkbenchEntityRevisionLabel => 'エンティティリビジョン';

  @override
  String get managedStoryWorkbenchStableIdLabel => '安定ID';

  @override
  String get managedStoryWorkbenchReferenceResolvedLabel => '参照は解決済み';

  @override
  String get managedStoryWorkbenchReferenceUnresolvedLabel => '参照は未解決';

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
}
