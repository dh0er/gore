// ignore: unused_import
import 'package:intl/intl.dart' as intl;
import 'app_localizations.dart';

// ignore_for_file: type=lint

/// The translations for Japanese (`ja`).
class AppLocalizationsJa extends AppLocalizations {
  AppLocalizationsJa([String locale = 'ja']) : super(locale);

  @override
  String get coreBlockedTitle => 'Mod Manager を起動できません';

  @override
  String get coreDllMissingMessage => '必要なプログラムファイルがありません (gore_ffi.dll)。';

  @override
  String get coreDllLoadFailedMessage => '必要なプログラムファイルを読み込めませんでした。';

  @override
  String get coreVerificationFailedMessage => '必要なプログラムファイルを検証できませんでした。';

  @override
  String get coreManagerTooOldMessage =>
      'プログラムファイルが Mod Manager より新しいです。Mod Manager を更新してください。';

  @override
  String get coreNativeTooOldMessage =>
      'プログラムファイルが Mod Manager より古いです。Mod Manager を再インストールしてください。';

  @override
  String get coreCommandsMissingMessage =>
      'プログラムファイルに、この Mod Manager が必要とする機能がありません。';

  @override
  String get coreBlockedRepairHint =>
      'Mod Manager を再インストールまたは修復してから、もう一度起動してください。';

  @override
  String get coreTechnicalDetails => '技術情報';

  @override
  String get coreCopyTechnicalDetails => '技術情報をコピー';

  @override
  String get coreTechnicalDetailsCopied => '技術情報をコピーしました';

  @override
  String get coreTechnicalDetailsCopyFailed => '技術情報をコピーできませんでした。もう一度お試しください。';

  @override
  String get preflightAttention => 'MOD を変更する前に対応が必要なことがあります。';

  @override
  String get preflightGameRunning =>
      'Gothic がまだ起動しています。MOD を変更する前にゲームを終了してください。';

  @override
  String get managerOperationFailed => '処理に失敗しました。';

  @override
  String get libraryOperationFailed => 'MOD の一覧を読み込めませんでした。';

  @override
  String get conflictsUnavailable => '競合を確認できませんでした。';

  @override
  String applyReportAppliedWithWarnings(int applied, int warnings) {
    return '適用済み: $applied。警告: $warnings。';
  }

  @override
  String get modDetailKind => '種類';

  @override
  String get modDetailVersion => 'バージョン';

  @override
  String get modDetailAuthor => '作者';

  @override
  String get modDetailSource => '入手元';

  @override
  String get modDetailImported => 'インポート日時';

  @override
  String get componentLocalization => 'テキスト';

  @override
  String get componentAudio => 'サウンド';

  @override
  String get componentAngelScript => 'スクリプト';

  @override
  String get componentTexture => 'テクスチャ';

  @override
  String get componentGameFiles => 'ゲームファイル';

  @override
  String get componentVoice => 'ボイス';

  @override
  String get componentKindLocalizationPatch => 'テキストの変更';

  @override
  String get componentKindAudioPatch => 'サウンドの変更';

  @override
  String get componentKindAngelScriptPatch => 'スクリプトの変更';

  @override
  String get componentKindTexturePatch => 'テクスチャの変更';

  @override
  String get componentKindLoosePak => 'PAK ファイル';

  @override
  String get componentKindTriplet => 'IoStore コンテナ';

  @override
  String get componentKindUe4ssLua => 'UE4SS スクリプト';

  @override
  String get componentKindRawFile => 'ファイル';

  @override
  String get componentKindFilePatch => '置き換えたゲームファイル';

  @override
  String get componentKindPakFilePatch => '~mods の PAK から差し込むゲームファイル';

  @override
  String get componentKindVoiceArchivePatch => 'ボイス';

  @override
  String get rawTargetGameText => 'ゲームテキスト全体';

  @override
  String get rawTargetGameScripts => 'ゲームスクリプト全体';

  @override
  String get rawTargetSoundBank => 'サウンドバンク';

  @override
  String rawTargetSoundBankNamed(String name) {
    return 'サウンドバンク: $name';
  }

  @override
  String get conflictKindLocalization => 'テキスト';

  @override
  String get conflictKindAudio => 'サウンド';

  @override
  String get conflictKindAsset => 'ゲームデータ';

  @override
  String get conflictKindCdo => 'オブジェクトの値';

  @override
  String get conflictKindUe4ssUnknown => 'UE4SS (不明)';

  @override
  String get conflictKindScriptModule => 'ゲームスクリプト';

  @override
  String get conflictKindVoiceArchive => 'ボイス';

  @override
  String get conflictKindRawFile => 'ファイル';

  @override
  String get conflictKindLooseFile => 'ゲームファイル';

  @override
  String get preflightUnavailable => 'ゲームのインストール先を確認できませんでした。';

  @override
  String get preflightRetry => '再確認';

  @override
  String get preflightReviewStatus => '状態を表示';

  @override
  String get preflightReviewRecovery => 'ヘルプを表示';

  @override
  String get installRecoveryTitle => '中断されたインストール';

  @override
  String get installRecoveryBody =>
      'GORE がインストールまたはスクリプトのビルドの残りデータを見つけました。その処理はまだ実行中か、終了して残したものかもしれません。GORE が安全に自動で片付けることはできません。';

  @override
  String get installRecoverySteps =>
      '処理がまだ実行中なら、終わるまで待ってください。中断したりファイルを削除したりしないでください。何も実行されていないと確認できたら、下のフォルダーの README.txt に従い、もう一度確認してください。フォルダーが表示されない場合や不安な場合は、そのままにして助けを求めてください。';

  @override
  String get installRecoveryEvidence => 'GORE が見つけたもの';

  @override
  String get managerRecoveryTitle => '中断された変更を修復';

  @override
  String get managerRecoveryConfirm =>
      'GORE が中断された変更を見つけました。ゲームを分かっている状態に戻せます。セーブデータには一切触れません。';

  @override
  String get managerRecoveryAlreadyClean => '修復するものはありませんでした。状態を再確認しました。';

  @override
  String get managerRecoveryBusy => '処理がまた実行中です。何も変更していません。終わるまで待ってください。';

  @override
  String get managerRecoveryLockCleared => '中断された処理はまだ何も変更していませんでした。片付けました。';

  @override
  String get managerRecoveryRestoredPristine => '変更を取り消しました。ゲームは以前の状態に戻っています。';

  @override
  String get managerRecoveryApplyPreserved => '適用はすでに完了していました。失われたものはありません。';

  @override
  String get managerRecoveryUndeployConfirmed => '削除はすでに完了していました。残りを片付けました。';

  @override
  String get managerRecoveryCompileRequired =>
      'これはスクリプトのビルドに属するため、何も変更していません。修復のヘルプを開いてください。';

  @override
  String get managerRecoveryInspectionFailed =>
      'GORE は中断された処理を安全に確認できませんでした。何も変更していません。';

  @override
  String get managerRecoveryFailed => '修復を完了できませんでした。もう一度試す前に状態を確認してください。';

  @override
  String get statusUnknown => '不明';

  @override
  String statusDetailsTitle(String status) {
    return '状態: $status';
  }

  @override
  String statusDetailsOpen(String status) {
    return '詳細を表示: $status';
  }

  @override
  String get statusDetailsNoRoot => 'まず設定で Gothic のインストール先を選んでください。';

  @override
  String get statusDetailsNoDeployment => '現在、ゲームに MOD は入っていません。';

  @override
  String get statusDetailsInSyncDescription => 'ここでチェックした MOD がそのままゲームに入っています。';

  @override
  String get statusDetailsDeployedLoadout => 'ゲーム内の MOD';

  @override
  String get statusDetailsChangesDescription => '選択内容がゲーム内の状態と違います。';

  @override
  String get statusDetailsCurrentlyDeployed => '現在ゲーム内';

  @override
  String get statusDetailsAfterApply => '適用後';

  @override
  String get statusDetailsGameUpdatedDescription =>
      'ゲームが更新され、MOD のファイルが上書きされました。もう一度適用して戻してください。';

  @override
  String get statusDetailsDriftedFiles => '影響を受けたファイル';

  @override
  String get statusDetailsStudioDescription =>
      '現在 Mod Studio がこのゲームに MOD を入れています。マネージャーで適用する前にゲームを引き継いでください。';

  @override
  String statusDetailsStudioMod(String name) {
    return 'Studio Mod: $name';
  }

  @override
  String get statusDetailsStudioNameUnknown => 'Mod Studio は名前を報告しませんでした。';

  @override
  String get statusDetailsRecoveryDescription =>
      '処理が中断されました。MOD を変更する前に修復してください。';

  @override
  String get statusDetailsUnknownDescription => '状態を読み取れませんでした。先に更新してください。';

  @override
  String get statusDetailsUnavailable => '詳細はありません。';

  @override
  String get statusDetailsEmptyLoadout => 'MOD はありません。';

  @override
  String get statusDetailsLastError => '最後のエラー';

  @override
  String get statusDetailsLastApply => '前回の適用';

  @override
  String get statusDetailsAppliedMods => '適用した Mod';

  @override
  String get statusDetailsWarnings => '警告';

  @override
  String get statusDetailsReapply => '再適用';

  @override
  String get statusDetailsOpenSettings => '設定を開く';

  @override
  String get recoveryAction => '修復';

  @override
  String get recoveryRequiredConfirm => '中断された処理を修復し、中途半端に入ったファイルを削除しますか？';

  @override
  String get statusRecoveryRequired => '修復が必要';

  @override
  String get statusDetailsOwnershipTitle => 'GORE が管理するファイル';

  @override
  String get statusDetailsOwnershipDescription =>
      'MOD の適用時に記録された一覧です。ファイルが今も存在するかは確認していません。';

  @override
  String get statusDetailsOwnershipLive => '置換されたゲームファイル';

  @override
  String get statusDetailsOwnershipBackups => 'オリジナルのバックアップ';

  @override
  String get statusDetailsOwnershipAdditive => '追加された MOD ファイル';

  @override
  String get statusDetailsOwnershipUe4ss => 'UE4SS Mod ディレクトリ';

  @override
  String get statusDetailsOwnershipRecovery => '修復用ファイル';

  @override
  String get statusDetailsOwnershipEmpty => 'ここには何も記録されていません。';

  @override
  String statusDetailsOwnershipShown(int shown, int total) {
    return '$total 件中 $shown 件のパスを表示中。';
  }

  @override
  String get appTitle => 'GORE Mod Manager';

  @override
  String get tabMods => 'Mod';

  @override
  String get tabSettings => '設定';

  @override
  String get settingsGameExe => 'Gothic のインストール先';

  @override
  String get settingsGameExePick => '選択…';

  @override
  String get settingsLanguage => '言語';

  @override
  String get libraryEmptyTitle => 'MOD がまだありません';

  @override
  String get libraryEmptyBody => 'フォルダーか MOD ファイルをインポートして始めましょう。';

  @override
  String get detailEmptyHint => 'MOD を選ぶと、何を変えるか表示されます。';

  @override
  String get settingsAdvanced => '詳細情報';

  @override
  String get settingsAdvancedHint =>
      '技術的な情報を表示します: 影響を受ける項目、競合チェックの確からしさ、GORE が管理するファイル。';

  @override
  String get updatesTitle => 'アップデート';

  @override
  String get checkForUpdatesAutomatically => '自動でアップデートを確認する';

  @override
  String get checkForUpdatesNow => '今すぐアップデートを確認';

  @override
  String get updatesPortableNotice =>
      'ポータブル版はダウンロードページをブラウザーで開きます。既存のファイルを新しいダウンロードで置き換えてください。';

  @override
  String get updateCheckFailed => 'アップデートを確認できませんでした。後でもう一度お試しください。';

  @override
  String get updateUpToDate => '最新バージョンを使用しています。';

  @override
  String get updateAvailableTitle => 'アップデートがあります';

  @override
  String updateAvailableMessage(String version, String current) {
    return 'バージョン $version が利用できます。現在は $current です。';
  }

  @override
  String get updateLater => '後で';

  @override
  String get updateDownload => 'ダウンロード';

  @override
  String updateOpenFailed(String url) {
    return 'ダウンロードページを開けませんでした。$url からアクセスできます。';
  }

  @override
  String get statusInSync => '最新の状態';

  @override
  String get statusChangesPending => '未適用';

  @override
  String get statusGameUpdated => 'ゲームが更新されました';

  @override
  String get statusStudioDeploy => 'Mod Studio が使用中';

  @override
  String get statusNothingDeployed => 'ゲームに MOD なし';

  @override
  String get actionImport => 'インポート';

  @override
  String get actionApply => '適用';

  @override
  String get actionStartGame => 'ゲームを起動';

  @override
  String get startGameTooltip => '現在ゲームに入っている MOD で Gothic を起動します';

  @override
  String get startGameFailed => 'Gothic を起動できませんでした。設定でゲームのインストール先を確認してください。';

  @override
  String get commonCancel => 'キャンセル';

  @override
  String get commonOk => 'OK';

  @override
  String get importFolder => 'フォルダーをインポート…';

  @override
  String get importFile => 'ファイルをインポート…';

  @override
  String importOutcomeCreated(String name) {
    return '「$name」を追加しました。';
  }

  @override
  String importOutcomeUpdated(String name) {
    return '「$name」を更新しました。';
  }

  @override
  String importOutcomeUnchanged(String name) {
    return '「$name」はすでに一覧にあります。';
  }

  @override
  String importOutcomeMatchedBy(String method) {
    String _temp0 = intl.Intl.selectLogic(method, {
      'none': '一致する既存の MOD はありません。',
      'source': '同じインポート元で一致しました。',
      'content': '内容が同一と確認できて一致しました。',
      'entry_id': 'MOD ID で一致しました。',
      'other': '一致の詳細は取得できません。',
    });
    return '$_temp0';
  }

  @override
  String get importRefusalDuplicateAmbiguous =>
      'これはすでに持っている複数の MOD と一致します。重複を削除してからもう一度試してください。';

  @override
  String get importRefusalIdentityConflict =>
      '取得元と中身が、すでに持っている別々の MOD と一致します。整理してからもう一度試してください。';

  @override
  String get importFailed =>
      'これはインポートできませんでした。対応しているのはフォルダー、ZIP アーカイブ、単体の MOD ファイル (*_P.pak、.utoc/.ucas、.lcache、.bank、PrecompiledScript*.Cache) です。.7z や .rar は先に展開してからフォルダーをインポートしてください。すでに追加または更新されている場合もあるため、もう一度試す前に一覧を更新してください。';

  @override
  String get importPickerFailed => 'ファイル選択を開けませんでした。何もインポートしていません。';

  @override
  String get importOutcomeUnknown => '結果がはっきりしません。更新して MOD の一覧を確認してください。';

  @override
  String get applyTooltip => 'チェックした MOD をゲームに入れる';

  @override
  String get undeployAllAction => 'すべてゲームから外す';

  @override
  String get undeployAllConfirm => 'マネージャーが入れた MOD をすべてゲームから外しますか？';

  @override
  String get takeOverTitle => 'Mod Studio が使用中です';

  @override
  String get takeOverBody =>
      'Mod Studio が MOD をゲームに入れています。引き継いでマネージャーで選択を適用しますか？';

  @override
  String get takeOverAction => '引き継ぐ';

  @override
  String get refreshAction => '更新';

  @override
  String conflictsTitle(int count) {
    return '競合 ($count)';
  }

  @override
  String get conflictWinner => '優先';

  @override
  String get noConflicts => '競合は見つかりませんでした。';

  @override
  String get conflictCoverageIncomplete =>
      '一部の MOD は完全に確認できないため、他にも競合がある可能性があります。';

  @override
  String get loadOrderDirection => 'リストの下にある MOD が、上にある MOD を上書きします。';

  @override
  String get footprintCoverageScope =>
      '判明している競合対象だけを表示しています。ゲーム内の結果を保証するものではありません。';

  @override
  String get footprintTargetsExact => '影響を受ける項目 — 完全な一覧:';

  @override
  String get footprintTargetsPartial => '影響を受ける項目 — 他にもある可能性があります:';

  @override
  String get footprintTargetsAdvisory => '影響を受ける可能性のある項目 — 手がかりであり確定ではありません:';

  @override
  String get footprintTargetsOpaque => 'ここで何が変わるか、GORE には判別できません。';

  @override
  String get conflictsUnverified => '競合は不明です。先に更新してください。';

  @override
  String get componentsTitle => 'この MOD が変えるもの';

  @override
  String targetsMore(int count) {
    return '他 $count 件';
  }

  @override
  String get removeModDeploymentHint =>
      '一覧から外すだけです。ゲームに入っている場合は、そのあと「適用」を選んでください。';

  @override
  String removeModSuccess(String name) {
    return '「$name」を削除しました。';
  }

  @override
  String removeModFailed(String name) {
    return '「$name」を削除できませんでした。';
  }

  @override
  String removeModPartialFailure(String name) {
    return '「$name」を削除しましたが、一覧を完全に更新できませんでした。';
  }

  @override
  String removeModOutcomeUnknown(String name) {
    return '「$name」が削除されたか確認できませんでした。';
  }

  @override
  String get libraryStateUnknown => 'MOD の一覧が最新ではありません。変更や適用の前に更新してください。';

  @override
  String get removeModAction => '削除';

  @override
  String removeModConfirm(String name) {
    return '「$name」を一覧から削除しますか？';
  }

  @override
  String get errorSetGamePath => 'まず設定で Gothic のインストール先を選んでください。';

  @override
  String applyReportApplied(int count) {
    return '$count 個の Mod を適用しました。';
  }

  @override
  String get modDisabledHint => '無効';

  @override
  String get kindGoremod => 'GORE バンドル';

  @override
  String get kindTriplet => 'IoStore MOD';

  @override
  String get kindPak => 'PAK MOD';

  @override
  String get kindUe4ss => 'UE4SS';

  @override
  String get kindRawfile => 'ファイル丸ごと置換';

  @override
  String get kindMixed => '混在';

  @override
  String get sevHard => '競合';

  @override
  String get sevSoft => '警告';

  @override
  String get sevInfo => '情報';

  @override
  String aboutVersion(String version, String sha) {
    return 'Version $version ($sha)';
  }

  @override
  String get about => 'このアプリについて';

  @override
  String get aboutCopyright => '© 2026 Daniel Hoer';

  @override
  String get aboutLicense => 'MIT ライセンスの下で提供されています。';

  @override
  String get appearanceTitle => '外観';

  @override
  String get theme => 'テーマ';

  @override
  String get themeLight => 'ライト';

  @override
  String get themeDark => 'ダーク';

  @override
  String get themeSystem => 'システム';

  @override
  String get uiScale => '表示サイズ';

  @override
  String get resetZoomTooltip => 'ズームをリセット（Ctrl+0）';

  @override
  String get zoomTip => 'ヒント: アプリ内のどこでも Ctrl + / Ctrl - でズームを変更できます。';

  @override
  String get lightMode => 'ライトモード';

  @override
  String get darkMode => 'ダークモード';

  @override
  String get minimize => '最小化';

  @override
  String get restore => '元のサイズに戻す';

  @override
  String get maximize => '最大化';

  @override
  String get close => '閉じる';
}
