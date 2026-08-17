// ignore: unused_import
import 'package:intl/intl.dart' as intl;
import 'app_localizations.dart';

// ignore_for_file: type=lint

/// The translations for Japanese (`ja`).
class AppLocalizationsJa extends AppLocalizations {
  AppLocalizationsJa([String locale = 'ja']) : super(locale);

  @override
  String get coreBlockedTitle => 'Mod Manager を利用できません';

  @override
  String get coreDllMissingMessage => '必要な gore_ffi.dll が見つかりません。';

  @override
  String get coreDllLoadFailedMessage => 'GORE Core のネイティブライブラリを読み込めませんでした。';

  @override
  String get coreVerificationFailedMessage =>
      'GORE Core のネイティブライブラリを検証できませんでした。';

  @override
  String get coreManagerTooOldMessage =>
      'この GORE Core は Mod Manager より新しいバージョンです。Mod Manager を更新してください。';

  @override
  String get coreNativeTooOldMessage =>
      'この GORE Core は Mod Manager より古いバージョンです。Mod Manager のインストール全体を更新または修復してください。';

  @override
  String get coreCommandsMissingMessage =>
      'この GORE Core ライブラリには、Mod Manager に必要なコマンドがすべて含まれていません。';

  @override
  String get coreBlockedRepairHint =>
      'Mod Manager のパッケージ全体を更新または修復してから、アプリを再起動してください。';

  @override
  String get coreTechnicalDetails => '技術情報';

  @override
  String get coreCopyTechnicalDetails => '技術情報をコピー';

  @override
  String get coreTechnicalDetailsCopied => '技術情報をコピーしました';

  @override
  String get coreTechnicalDetailsCopyFailed => '技術情報をコピーできませんでした。もう一度お試しください。';

  @override
  String get preflightAttention => 'セットアップの確認が必要です。';

  @override
  String get preflightUnavailable => 'セットアップ診断を利用できません。';

  @override
  String get preflightRetry => '再確認';

  @override
  String get preflightReviewStatus => '状態を確認';

  @override
  String get preflightReviewRecovery => '復旧方法';

  @override
  String get installRecoveryTitle => 'インストールの復旧';

  @override
  String get installRecoveryBody =>
      'GORE は、インストールまたはスクリプトビルドに関する復旧データを検出しました。関連する処理がまだ実行中か、処理の終了後にデータだけが残っている可能性があります。GORE はこれを安全に自動修復できません。';

  @override
  String get installRecoverySteps =>
      '関連する処理がまだ実行中の場合は、完了するまで待ってください。その処理を停止したり、ロックファイルを削除したりしないでください。関連する処理が実行されていないことを確実に確認してから、下に表示された復旧フォルダー内の README.txt に従ってください。フォルダーが表示されない場合や確信が持てない場合は、復旧データを変更せず、サポートを求めてください。その後、もう一度確認してください。';

  @override
  String get installRecoveryEvidence => '検出された復旧データ';

  @override
  String get managerRecoveryTitle => '中断されたマネージャー操作を復旧';

  @override
  String get managerRecoveryConfirm =>
      'GORE は明確に中断されたマネージャー操作を検出しました。記録された操作を確認し、インストールを既知の状態に戻す場合にのみ続行してください。セーブデータは変更されません。';

  @override
  String get managerRecoveryAlreadyClean =>
      '中断された操作はすでに解決されていました。インストールを再確認しました。';

  @override
  String get managerRecoveryBusy => '操作が再び実行中です。変更は行われていません。完了を待ってから再確認してください。';

  @override
  String get managerRecoveryLockCleared =>
      '中断された操作はまだインストールを変更していませんでした。古いロックを安全に削除しました。';

  @override
  String get managerRecoveryRestoredPristine =>
      '中断された変更を取り消し、記録されていたインストールの基準状態を復元しました。';

  @override
  String get managerRecoveryApplyPreserved =>
      '適用はすでに完了していました。記録された状態を保持し、ステータスを再確認しました。';

  @override
  String get managerRecoveryUndeployConfirmed =>
      '削除は完了していました。残っていたトランザクションデータを整理し、ステータスを再確認しました。';

  @override
  String get managerRecoveryCompileRequired =>
      'これはスクリプトビルドの復旧に関するものです。マネージャーは変更していません。復旧ヘルプを確認してください。';

  @override
  String get managerRecoveryInspectionFailed =>
      'GORE は中断された操作を安全に確認できませんでした。変更は行われていません。現在の復旧情報を確認してください。';

  @override
  String get managerRecoveryFailed =>
      '復旧を完了できませんでした。GORE はインストールの再確認を試みましたが、現在の状態は不明な可能性があります。再試行する前に状態を確認してください。';

  @override
  String get statusUnknown => '不明';

  @override
  String statusDetailsTitle(String status) {
    return 'デプロイ: $status';
  }

  @override
  String statusDetailsOpen(String status) {
    return 'デプロイの詳細を表示: $status';
  }

  @override
  String get statusDetailsNoRoot => 'デプロイ状態を確認するには、設定でゲームのインストール先を選択してください。';

  @override
  String get statusDetailsNoDeployment => 'このゲームにはマネージャーのデプロイがありません。';

  @override
  String get statusDetailsInSyncDescription => 'デプロイ済み Mod は現在のロードアウトと一致しています。';

  @override
  String get statusDetailsDeployedLoadout => 'デプロイ済みのロード順';

  @override
  String get statusDetailsChangesDescription =>
      '現在のデプロイは、適用後にインストールされる内容と異なります。';

  @override
  String get statusDetailsCurrentlyDeployed => '現在のデプロイ';

  @override
  String get statusDetailsAfterApply => '適用後';

  @override
  String get statusDetailsGameUpdatedDescription =>
      '前回のデプロイ後にゲームファイルが変更されました。ロードアウトを再適用してマネージャー所有のファイルを復元してください。';

  @override
  String get statusDetailsDriftedFiles => '変更されたファイル';

  @override
  String get statusDetailsStudioDescription =>
      '現在 Mod Studio がこのゲームを管理しています。マネージャーのロードアウトを適用する前に引き継いでください。';

  @override
  String statusDetailsStudioMod(String name) {
    return 'Studio Mod: $name';
  }

  @override
  String get statusDetailsStudioNameUnknown => 'Studio から Mod 名が報告されませんでした。';

  @override
  String get statusDetailsRecoveryDescription =>
      'デプロイが中断されました。マネージャーの Mod を適用または削除する前に復旧してください。';

  @override
  String get statusDetailsUnknownDescription =>
      'デプロイ状態を確認できませんでした。Mod を適用する前に更新してください。';

  @override
  String get statusDetailsUnavailable => 'インストール済みのコアから詳細が提供されませんでした。';

  @override
  String get statusDetailsEmptyLoadout => 'このロードアウトに Mod はありません。';

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
  String get recoveryAction => '復旧';

  @override
  String get recoveryRequiredConfirm => '中断されたデプロイを復旧し、部分的にデプロイされたファイルを削除しますか？';

  @override
  String get statusRecoveryRequired => '復旧が必要';

  @override
  String get statusDetailsOwnershipTitle => '記録された所有権の証拠';

  @override
  String get statusDetailsOwnershipDescription =>
      'マネージャーのデプロイ記録に保存されたパスです。現在もそのパスが存在することを示すものではありません。';

  @override
  String get statusDetailsOwnershipLive => '置換されたゲームファイル';

  @override
  String get statusDetailsOwnershipBackups => '元ファイルのバックアップ';

  @override
  String get statusDetailsOwnershipAdditive => '追加された pak とコンテナファイル';

  @override
  String get statusDetailsOwnershipUe4ss => 'UE4SS Mod ディレクトリ';

  @override
  String get statusDetailsOwnershipRecovery => '復旧ファイルと保持場所';

  @override
  String get statusDetailsOwnershipEmpty => 'このグループに記録されたパスはありません。';

  @override
  String statusDetailsOwnershipShown(int shown, int total) {
    return '記録された $total 件のパスのうち $shown 件を表示しています。';
  }

  @override
  String get appTitle => 'GORE Mod Manager';

  @override
  String get tabMods => 'Mod';

  @override
  String get tabSettings => '設定';

  @override
  String get settingsGameExe => 'ゲーム実行ファイル';

  @override
  String get settingsGameExePick => '選択…';

  @override
  String get settingsLanguage => '言語';

  @override
  String get statusInSync => '同期済み';

  @override
  String get statusChangesPending => '変更が保留中';

  @override
  String get statusGameUpdated => 'ゲームが更新されました';

  @override
  String get statusStudioDeploy => 'Studio のデプロイが有効';

  @override
  String get statusNothingDeployed => '何もデプロイされていません';

  @override
  String get actionImport => 'インポート';

  @override
  String get actionApply => '適用';

  @override
  String get actionUndeployAll => 'すべて解除';

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
    return '「$name」をライブラリに追加しました。';
  }

  @override
  String importOutcomeUpdated(String name) {
    return 'ライブラリ内の「$name」を更新しました。';
  }

  @override
  String importOutcomeUnchanged(String name) {
    return '「$name」はすでにライブラリにあります。';
  }

  @override
  String importOutcomeMatchedBy(String method) {
    String _temp0 = intl.Intl.selectLogic(method, {
      'none': '既存のライブラリエントリとの一致はありません。',
      'source': '同じインポート元に基づく一致です。',
      'content': '同一であることが確認されたコンテンツに基づく一致です。',
      'entry_id': 'Mod ID に基づく一致です。',
      'other': '一致方法の詳細を確認できません。',
    });
    return '$_temp0';
  }

  @override
  String get importRefusalDuplicateAmbiguous =>
      'このインポートは複数のライブラリエントリに一致します。重複を確認するか削除してから、もう一度お試しください。';

  @override
  String get importRefusalIdentityConflict =>
      'インポート元とそのコンテンツが、ライブラリ内の別々のエントリに一致しています。競合するエントリを確認するか削除してから、もう一度お試しください。';

  @override
  String get importFailed =>
      'インポートを完了できませんでした。対応するソースは、フォルダー、ZIP、単体の *_P.pak、完全な .utoc/.ucas セット（.pak は任意）、.lcache、.bank、PrecompiledScript*.Cache です。.7z または .rar は先に展開し、そのフォルダーをインポートしてください。ソースは未対応、破損、または不完全な可能性があります。Mod はすでに追加または更新されている可能性があります。ライブラリを更新して確認してから、もう一度お試しください。';

  @override
  String get importPickerFailed =>
      'ファイルまたはフォルダーの選択画面を開けませんでした。インポートは開始されていません。もう一度お試しください。';

  @override
  String get importOutcomeUnknown =>
      'インポート結果を確認できませんでした。［更新］を選択してライブラリを確認してください。';

  @override
  String get applyTooltip => 'ロードアウトをゲームに適用';

  @override
  String get undeployAllAction => 'すべて解除';

  @override
  String get undeployAllConfirm => 'マネージャーがデプロイしたものをすべてゲームから削除しますか？';

  @override
  String get takeOverTitle => 'Studio のデプロイが有効';

  @override
  String get takeOverBody =>
      'mod-studio がゲームに Mod をデプロイしています。マネージャーがこのロードアウトを適用できるように引き継ぎますか？';

  @override
  String get takeOverAction => '引き継ぐ';

  @override
  String get refreshAction => '更新';

  @override
  String conflictsTitle(int count) {
    return '検出結果 ($count)';
  }

  @override
  String get conflictWinner => '想定上の勝者';

  @override
  String get noConflicts => '認識された競合はありません。';

  @override
  String get conflictCoverageIncomplete =>
      '有効な Mod の競合情報は不完全です。ほかにも競合が存在する可能性があります。';

  @override
  String get loadOrderDirection => 'ロード順: 低い優先度が先で、後の Mod ほど意図された優先度が高くなります。';

  @override
  String get footprintCoverageScope =>
      'カバレッジは認識済みの競合ターゲットだけを示し、実行時の優先度を保証しません。';

  @override
  String get footprintCoverageExact => '完全 — コンポーネントの競合ターゲット一覧は完全です。';

  @override
  String get footprintCoveragePartial =>
      '部分的 — 表示された競合ターゲットは既知ですが、ほかにも影響する可能性があります。';

  @override
  String get footprintCoverageAdvisory =>
      '参考 — 表示されたターゲットは手掛かりであり、網羅的な証明ではありません。';

  @override
  String get footprintCoverageOpaque => '不透明 — コンポーネントの競合ターゲットは不明です。';

  @override
  String get footprintCoverageExactLabel => '完全';

  @override
  String get footprintCoveragePartialLabel => '部分的';

  @override
  String get footprintCoverageAdvisoryLabel => '参考';

  @override
  String get footprintCoverageOpaqueLabel => '不透明';

  @override
  String get conflictsUnverified => 'ライブラリの状態を更新するまで、競合は未確認です。';

  @override
  String get componentsTitle => 'コンポーネント';

  @override
  String targetsMore(int count) {
    return '他 $count 件';
  }

  @override
  String get removeModDeploymentHint =>
      'ライブラリから削除しても、既存のデプロイはすぐには変更されません。この Mod がデプロイ済みの場合は、その後に［適用］を選択してゲームのインストールを更新してください。';

  @override
  String removeModSuccess(String name) {
    return '「$name」をライブラリから削除しました。';
  }

  @override
  String removeModFailed(String name, String error) {
    return '「$name」を削除できませんでした: $error';
  }

  @override
  String removeModPartialFailure(String name, String error) {
    return '「$name」は削除されましたが、後続処理でエラーが報告されました。ライブラリの状態は再読み込みされました: $error';
  }

  @override
  String removeModOutcomeUnknown(String name, String error) {
    return '「$name」が削除されたか確認できませんでした: $error。ライブラリの状態を確認するには更新してください。';
  }

  @override
  String get libraryStateUnknown =>
      'ライブラリの状態を確認できませんでした。Mod を変更または適用する前に［更新］を選択してください。';

  @override
  String get removeModAction => '削除';

  @override
  String removeModConfirm(String name) {
    return '「$name」をライブラリから削除しますか？';
  }

  @override
  String get errorSetGamePath => '先に設定でゲームのパスを指定してください。';

  @override
  String applyReportApplied(int count) {
    return '$count 個の Mod を適用しました。';
  }

  @override
  String get warningsTitle => '警告';

  @override
  String get modDisabledHint => '無効';

  @override
  String get kindGoremod => 'goremod';

  @override
  String get kindTriplet => 'triplet';

  @override
  String get kindPak => 'pak';

  @override
  String get kindUe4ss => 'UE4SS';

  @override
  String get kindRawfile => '生ファイル';

  @override
  String get kindMixed => '混合';

  @override
  String get sevHard => '重大';

  @override
  String get sevSoft => '軽微';

  @override
  String get sevInfo => '情報';

  @override
  String aboutVersion(String version, String sha) {
    return 'Version $version ($sha)';
  }

  @override
  String get about => 'このアプリについて';

  @override
  String get aboutCopyright => '© 2026 GORE コントリビューター';

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
  String get uiScale => 'UI スケール';

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
