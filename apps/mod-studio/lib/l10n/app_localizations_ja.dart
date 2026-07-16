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
  String get managedWorkspaceSettingsExpertLabel => '設定とエキスパート';

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
  String get managedActionNewQuestTitle => '新しいクエスト';

  @override
  String get managedActionNewQuestDescription =>
      '目標と検証済みの親IDを含むオフラインのクエスト下書きを作成します。';

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
}
