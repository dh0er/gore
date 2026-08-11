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
  String get conflictWinner => '優先';

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
