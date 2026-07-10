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
}
