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
  String get managedSectionWorldDescription => 'ワールドへの配置と関連ワークフローは今後対応予定です。';

  @override
  String get managedSectionLocalizationVoiceDescription =>
      '音声制作ツールは利用できます。管理プロジェクトでのローカライズ編集は今後対応予定です。';

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
  String get managedActionAddVoiceTakeTitle => 'ボイステイクを追加';

  @override
  String get managedActionAddVoiceTakeDescription =>
      '配布せずにOgg Vorbis録音をこのプロジェクトへインポートします。';

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
}
