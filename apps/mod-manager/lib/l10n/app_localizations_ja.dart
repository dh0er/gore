// ignore: unused_import
import 'package:intl/intl.dart' as intl;
import 'app_localizations.dart';

// ignore_for_file: type=lint

/// The translations for Japanese (`ja`).
class AppLocalizationsJa extends AppLocalizations {
  AppLocalizationsJa([String locale = 'ja']) : super(locale);

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
    return '競合 ($count)';
  }

  @override
  String get conflictWinner => '優先';

  @override
  String get noConflicts => '競合はありません。';

  @override
  String get componentsTitle => 'コンポーネント';

  @override
  String targetsMore(int count) {
    return '他 $count 件';
  }

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
