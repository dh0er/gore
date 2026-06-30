// ignore: unused_import
import 'package:intl/intl.dart' as intl;
import 'app_localizations.dart';

// ignore_for_file: type=lint

/// The translations for Japanese (`ja`).
class AppLocalizationsJa extends AppLocalizations {
  AppLocalizationsJa([String locale = 'ja']) : super(locale);

  @override
  String get appTitle => 'Gothic Remake セーブデータエディター';

  @override
  String get appLogoSemanticLabel => 'goresave ロゴ';

  @override
  String get zoomTooltip => 'Ctrl +/- で拡大・縮小';

  @override
  String get switchToLightMode => 'ライトモードに切り替え';

  @override
  String get switchToDarkMode => 'ダークモードに切り替え';

  @override
  String get about => 'このアプリについて';

  @override
  String get tabOverview => '概要';

  @override
  String get tabPlayer => 'プレイヤー';

  @override
  String get tabInventory => 'インベントリ';

  @override
  String get tabProgression => '進行状況';

  @override
  String get tabAllData => '全データ';

  @override
  String get tabBackups => 'バックアップ';

  @override
  String get tabSettings => '設定';

  @override
  String get reset => 'リセット';

  @override
  String get save => '保存';

  @override
  String saveWithCount(int count) {
    return '保存（$count）';
  }

  @override
  String get ok => 'OK';

  @override
  String get cancel => 'キャンセル';

  @override
  String get confirm => '確定';

  @override
  String get close => '閉じる';

  @override
  String get add => '追加';

  @override
  String get equippedBadge => '装備中';

  @override
  String get armorUpgradesLabel => '強化';

  @override
  String get browse => '参照';

  @override
  String get noSavFilesFound => '.sav ファイルが見つかりません';

  @override
  String get profile => 'プロファイル';

  @override
  String profileWithSaves(String name, int count) {
    return '$name（セーブ $count 件）';
  }

  @override
  String get switchProfile => 'プロファイルを切り替え';

  @override
  String get rescanSaveFolder => 'セーブフォルダを再スキャン';

  @override
  String get discardUnsavedChangesTitle => '未保存の変更を破棄しますか？';

  @override
  String rescanDiscardBody(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '変更',
      one: '変更',
    );
    return '再スキャンするとすべてのセーブが読み込み直され、未保存の$count件の$_temp0が破棄されます。';
  }

  @override
  String get discardAndRescan => '破棄して再スキャン';

  @override
  String chapterLabel(Object id) {
    return '第 $id 章';
  }

  @override
  String get quickSave => 'クイックセーブ';

  @override
  String get autoSave => 'オートセーブ';

  @override
  String get manualSave => '手動セーブ';

  @override
  String get errorTitle => 'エラー';

  @override
  String get selectASaveTitle => 'セーブを選択';

  @override
  String get selectASaveBody => 'セーブの詳細がここに表示されます。';

  @override
  String get diagnosticsTitle => '診断と詳細';

  @override
  String get diagnosticsSubtitle => '読み取り専用のフォーマット検査';

  @override
  String get metricFormat => 'フォーマット';

  @override
  String get metricSlot => 'スロット';

  @override
  String get metricChapter => '章';

  @override
  String get metricTimePlayed => 'プレイ時間';

  @override
  String get metricSaveKind => 'セーブ種別';

  @override
  String get metricFileSize => 'ファイルサイズ';

  @override
  String get metricCompression => '圧縮';

  @override
  String get metricChunks => 'チャンク';

  @override
  String get metricUncompressed => '非圧縮';

  @override
  String get metricPrivate => 'プライベート';

  @override
  String get metricSlotName => 'スロット名';

  @override
  String get metricTrailer => 'トレーラー';

  @override
  String get metricDecodedPrivate => 'デコード済みプライベート';

  @override
  String get metricPrivateStrings => 'プライベート文字列';

  @override
  String get metricSha1 => 'SHA-1';

  @override
  String bytesValue(String count) {
    return '$count バイト';
  }

  @override
  String get inspectionJsonTitle => '検査 JSON';

  @override
  String get inspectionJsonSubtitle => '生のセーブ検査データ';

  @override
  String get copy => 'コピー';

  @override
  String get savegameFallbackTitle => 'セーブデータ';

  @override
  String screenshotForSlot(String slot) {
    return '$slot のスクリーンショット';
  }

  @override
  String get publicSaveName => '公開セーブ名';

  @override
  String get gameTimeTitle => 'Game time';

  @override
  String get gameTimeDay => 'Day';

  @override
  String get gameTimeHours => 'Hours';

  @override
  String get gameTimeMinutes => 'Minutes';

  @override
  String get gameTimeSeconds => 'Seconds';

  @override
  String gameTimeTotal(int seconds) {
    return '= $seconds s total';
  }

  @override
  String get gameTimeInvalid =>
      'Enter whole numbers — day ≥ 0, hours 0–23, minutes and seconds 0–59.';

  @override
  String get required => '必須';

  @override
  String get playerLockedBody => 'プライベートプレイヤーの編集には圧縮対応のコーデックが必要です。';

  @override
  String get heroTransform => 'ヒーローのトランスフォーム';

  @override
  String get locationX => '位置 X';

  @override
  String get locationY => '位置 Y';

  @override
  String get locationZ => '位置 Z';

  @override
  String get rotationPitch => '回転ピッチ';

  @override
  String get rotationYaw => '回転ヨー';

  @override
  String get rotationRoll => '回転ロール';

  @override
  String get invalid => '無効';

  @override
  String get heroAttributes => 'ヒーローの属性';

  @override
  String attributeBase(String name) {
    return '$name 基本値';
  }

  @override
  String attributeCurrent(String name) {
    return '$name 現在値';
  }

  @override
  String get inventoryTitle => 'インベントリ';

  @override
  String get inventoryNeedsDecoded =>
      'インベントリの編集には、コーデックでデコードされたプライベートペイロードデータが必要です。';

  @override
  String get inventoryNoStacks => 'デコードされたプライベートペイロードにアイテムスタックが見つかりません。';

  @override
  String get resetInventoryChanges => 'インベントリの変更をリセット';

  @override
  String get addItemTooltipPendingAdd =>
      '先に保留中の変更を保存してください — 1 回の保存につき新規アイテムは 1 つです';

  @override
  String get addItemTooltipPendingRemove =>
      '先に保留中の削除を保存してください — 1 回の保存につき構造変更は 1 つです';

  @override
  String get addItemTooltipPendingCount =>
      '先に保留中の数量変更を保存またはリセットしてください — 構造編集は単独で保存する必要があります';

  @override
  String get addItemTooltipDefault => 'インベントリにアイテムを追加';

  @override
  String get addItemButton => 'アイテムを追加';

  @override
  String pendingAddSubtitle(int count) {
    return '×$count — 追加保留中（未保存）';
  }

  @override
  String get cancelPendingAdd => '追加保留をキャンセル';

  @override
  String get pendingRemovalSubtitle => '削除保留中（未保存）';

  @override
  String get cancelPendingRemoval => '削除保留をキャンセル';

  @override
  String get filterItems => 'アイテムを絞り込む';

  @override
  String noItemsMatchQuery(String query) {
    return '「$query」に一致するアイテムはありません。';
  }

  @override
  String get pendingRemovalHidesAll =>
      '保留中の削除によりすべてのアイテムが非表示になっています — 保存して適用してください。';

  @override
  String categoryWithCount(String label, int count) {
    return '$label（$count）';
  }

  @override
  String get itemCategoryMeleeWeapon => '近接武器';

  @override
  String get itemCategoryRangedWeapon => '遠隔武器';

  @override
  String get itemCategoryAmmunition => '弾薬';

  @override
  String get itemCategoryArmor => '鎧';

  @override
  String get itemCategoryRune => 'ルーン';

  @override
  String get itemCategoryScroll => '呪文の巻物';

  @override
  String get itemCategoryFood => '食料・ポーション';

  @override
  String get itemCategoryMisc => 'その他雑貨';

  @override
  String get itemCategoryAmulet => 'アミュレット';

  @override
  String get itemCategoryRing => '指輪';

  @override
  String get itemCategoryTrophy => '動物の戦利品';

  @override
  String get itemCategoryWriting => '書物';

  @override
  String get itemCategoryMission => 'クエストアイテム';

  @override
  String get itemCategoryKey => '鍵';

  @override
  String get itemCategoryOther => 'その他';

  @override
  String get count => '数量';

  @override
  String get min1 => '最小 1';

  @override
  String countTimes(String count) {
    return '×$count';
  }

  @override
  String get deleteEquippedTooltip =>
      '削除できません: このアイテムは装備中か、ホットキースロットに割り当てられている可能性があります';

  @override
  String get removeBlockedTooltip =>
      '先に保留中のインベントリ変更を保存またはリセットしてください — 追加と削除は単独で保存する必要があります';

  @override
  String get removeItemFromInventory => 'インベントリからアイテムを削除';

  @override
  String get progressionLockedBody =>
      '進行状況データには、コーデックでデコードされたプライベートペイロードデータが必要です。';

  @override
  String get progressionNeedsTyped =>
      '構造化された進行状況データには、型付き解析が検証された完全にデコード済みのセーブが必要です。';

  @override
  String get sectionQuests => 'クエスト';

  @override
  String get sectionKnowledge => '知識';

  @override
  String get sectionEvents => 'イベント';

  @override
  String get firstPage => '最初のページ';

  @override
  String get previousPage => '前のページ';

  @override
  String get nextPage => '次のページ';

  @override
  String get lastPage => '最後のページ';

  @override
  String pageOfPages(int page, int total) {
    return 'ページ $page / $total';
  }

  @override
  String rangeOfTotal(int first, int last, int total) {
    return '$first–$last / $total';
  }

  @override
  String get perPage => 'ページあたり:';

  @override
  String get resetQuestChanges => 'クエストの変更をリセット';

  @override
  String get searchQuests => 'クエストを検索';

  @override
  String get allGroups => 'すべてのグループ';

  @override
  String groupWithCount(String group, Object count) {
    return '$group（$count）';
  }

  @override
  String stateLabelWithCount(String label, int count) {
    return '$label $count';
  }

  @override
  String get questStateNone => 'なし';

  @override
  String get questStateAvailable => '受注可能';

  @override
  String get questStateRunning => '進行中';

  @override
  String get questStateSucceeded => '成功';

  @override
  String get questStateFailed => '失敗';

  @override
  String get questStateUnknown => '不明';

  @override
  String get dialogKnowledge => '会話知識';

  @override
  String get resetKnowledgeChanges => '知識の変更をリセット';

  @override
  String get addNpc => 'NPC を追加';

  @override
  String get searchNpcs => 'NPC を検索';

  @override
  String entriesForCharacter(String name) {
    return 'エントリ — $name';
  }

  @override
  String get selectNpcToSeeEntries => 'エントリを表示する NPC を選択してください';

  @override
  String get addKnowledgeEntry => '知識エントリを追加';

  @override
  String get browseCatalog => 'カタログを参照';

  @override
  String get alreadyExistsForCharacter => 'このキャラクターには既に存在します。';

  @override
  String get alreadyInPendingChanges => '既に保留中の変更に含まれています。';

  @override
  String duplicateCheckFailed(String error) {
    return '重複チェックに失敗しました — もう一度お試しください: $error';
  }

  @override
  String pendingAddsCount(int count) {
    return '保留中の追加（$count）';
  }

  @override
  String get undoAdd => '追加を元に戻す';

  @override
  String get undoRemove => '削除を元に戻す';

  @override
  String get removeEntry => 'エントリを削除';

  @override
  String get selectNpcFromList => 'リストから NPC を選択してください';

  @override
  String characterWithCount(String name, int count) {
    return '$name（$count）';
  }

  @override
  String get memoryEvents => 'メモリイベント';

  @override
  String get searchCharacters => 'キャラクターを検索';

  @override
  String eventsForCharacter(String name) {
    return 'イベント — $name';
  }

  @override
  String get selectCharacterToSeeEvents => 'イベントを表示するキャラクターを選択してください';

  @override
  String get noTags => '（タグなし）';

  @override
  String eventSubtitle(String time, String affected) {
    return 't=${time}s  $affected';
  }

  @override
  String get removeEvent => 'イベントを削除';

  @override
  String get removeMemoryEventTitle => 'メモリイベントを削除しますか？';

  @override
  String get removeMemoryEventBody => 'このメモリイベントを削除しますか？ 事前にバックアップが作成されます。';

  @override
  String get duplicateEvent => 'イベントを複製';

  @override
  String get duplicateMemoryEventTitle => 'メモリイベントを複製しますか？';

  @override
  String get duplicateMemoryEventBody => 'このメモリイベントを複製しますか？ 事前にバックアップが作成されます。';

  @override
  String get selectCharacterFromList => 'リストからキャラクターを選択してください';

  @override
  String get allDataLockedBody =>
      '完全なプロパティブラウザには、コーデックでデコードされたプライベートペイロードデータが必要です。';

  @override
  String get allDataDescription =>
      'すべての型付きプロパティを名前またはパスで検索できます。スカラー、文字列、列挙型、オブジェクトパスは編集可能です。構造体は現時点では読み取り専用で表示されます。';

  @override
  String get searchPropertiesLabel => 'プロパティを検索（空欄ですべて表示） — 例: Health、GameTime';

  @override
  String get decodingSaveTitle => 'セーブをデコード中…';

  @override
  String get decodingSaveBody =>
      '最初の検索のためにプライベートペイロード全体をデコードしています。これはセーブごとに 1 回だけ実行され、その後の検索は瞬時に行われます。';

  @override
  String get searchTheSaveTitle => 'セーブを検索';

  @override
  String get searchTheSaveBody =>
      'プロパティ名を入力して Enter キーを押してください。空欄にするとすべて表示されます。';

  @override
  String get searchFailedTitle => '検索に失敗しました';

  @override
  String get noMatchesTitle => '一致なし';

  @override
  String get noMatchesBody => 'それらの語句をすべて含むプロパティパスはありませんでした。';

  @override
  String get value => '値';

  @override
  String get backupsTitle => 'バックアップ';

  @override
  String get refreshBackups => 'バックアップを更新';

  @override
  String get noBackupsTitle => 'バックアップなし';

  @override
  String get noBackupsBody => 'セーブを編集すると、選択したスロットの隣にバックアップファイルが作成されます。';

  @override
  String get slotBackups => 'スロットのバックアップ';

  @override
  String get profileBackups => 'プロファイルのバックアップ';

  @override
  String get backupFactName => '名前';

  @override
  String get backupFactSlot => 'スロット';

  @override
  String get backupFactCreated => '作成日時';

  @override
  String get backupFactSize => 'サイズ';

  @override
  String get backupFactStatus => 'ステータス';

  @override
  String get backupFactSha1 => 'SHA-1';

  @override
  String restoreBackupTooltip(String fileName) {
    return '$fileName を復元';
  }

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
  String get language => '言語';

  @override
  String get updatesTitle => 'アップデート';

  @override
  String get checkForUpdatesAutomatically => '自動的にアップデートを確認';

  @override
  String get checkForUpdatesNow => '今すぐアップデートを確認';

  @override
  String get updatesPortableNotice =>
      'ポータブル版はダウンロードページをブラウザで開きます。既存のファイルを新しいダウンロードで置き換えてください。';

  @override
  String get updateAvailableTitle => 'アップデートがあります';

  @override
  String updateAvailableMessage(Object version, Object current) {
    return 'バージョン $version が利用可能です。現在は $current です。';
  }

  @override
  String get updateDownload => 'ダウンロード';

  @override
  String get updateLater => '後で';

  @override
  String get updateUpToDate => '最新バージョンを使用しています。';

  @override
  String get updateCheckFailed => 'アップデートを確認できませんでした。後でもう一度お試しください。';

  @override
  String get gameTextTitle => 'ゲームテキスト';

  @override
  String gameTextExtractedWithCounts(int ids, int languages) {
    return '抽出済み: $languages 言語にわたり $ids 件の ID。';
  }

  @override
  String get gameTextExtracted => 'ローカライズされたゲームテキストが抽出されています。';

  @override
  String get gameTextNotExtracted => 'ローカライズされたゲームテキストはまだ抽出されていません。';

  @override
  String get extracting => '抽出中…';

  @override
  String get extractRefreshLocalizedText => 'ローカライズテキストを抽出 / 更新';

  @override
  String get extractLocalizedTextTitle => 'ローカライズされたゲームテキストを抽出しますか？';

  @override
  String get extractLocalizedTextBody =>
      'ローカライズされたゲームテキストはまだ抽出されていません。今すぐゲームのインストール先から抽出しますか？（任意）';

  @override
  String get notNow => '後で';

  @override
  String get extract => '抽出';

  @override
  String get extractionComplete => '抽出が完了しました';

  @override
  String get extractionFailed => '抽出に失敗しました';

  @override
  String get localizationCacheFileType => 'ローカライズキャッシュ';

  @override
  String get savegameDirectoryTitle => 'セーブデータディレクトリ';

  @override
  String get folder => 'フォルダ';

  @override
  String get codecTitle => 'コーデック';

  @override
  String get check => 'チェック';

  @override
  String get roundtrip => 'ラウンドトリップ';

  @override
  String get noCodecStatus => 'コーデックのステータスなし';

  @override
  String get codecReady => 'コーデック準備完了';

  @override
  String get codecReadOnly => 'コーデック読み取り専用';

  @override
  String get codecUnavailable => 'コーデック利用不可';

  @override
  String get details => '詳細';

  @override
  String codecStatusLine(String status) {
    return 'ステータス: $status';
  }

  @override
  String codecCapabilityLine(String decompress, String compress) {
    return '展開: $decompress | 圧縮: $compress';
  }

  @override
  String codecBackendLine(String backend) {
    return 'バックエンド: $backend';
  }

  @override
  String get yes => 'はい';

  @override
  String get no => 'いいえ';

  @override
  String get aboutSubtitle => 'Gothic Remake セーブデータエディター';

  @override
  String aboutVersion(String version, String sha) {
    return 'バージョン $version（$sha）';
  }

  @override
  String get aboutCopyright => '© 2026 goresave コントリビューター';

  @override
  String get aboutLicense => 'MIT ライセンスの下で提供されています。';

  @override
  String difficultyTitle(String profile) {
    return '難易度 — $profile';
  }

  @override
  String get difficultyNoProfile => 'プロファイルなし';

  @override
  String get difficultyNoDifficulty => '難易度なし';

  @override
  String get difficultyLabel => '難易度';

  @override
  String get difficultyTooltipNoProfile => 'プロファイルが選択されていません';

  @override
  String get difficultyTooltipEdit => 'このプロファイルの難易度を編集';

  @override
  String get difficultyTooltipNoEditable => 'このプロファイルには編集可能な難易度がありません';

  @override
  String get preset => 'プリセット';

  @override
  String get presetNovice => '初心者';

  @override
  String get presetGothic => 'Gothic';

  @override
  String get presetHard => 'ハード';

  @override
  String get presetCustom => 'カスタム';

  @override
  String unrecognisedPreset(Object preset) {
    return '保存されているプリセットは認識できません（$preset）。フロウヘルパー / パーマデスの変更は引き続き保存できます。または上記のプリセットを選択して上書きしてください。';
  }

  @override
  String get closeCombatFlowHelper => '近接戦闘フロウヘルパー';

  @override
  String get permadeath => 'パーマデス';

  @override
  String get notAvailableOnNovice => '初心者では利用できません';

  @override
  String get levelCombat => '戦闘';

  @override
  String get levelResources => 'リソース';

  @override
  String get levelProgression => '進行';

  @override
  String get difficultyAppliesToAllSaves => '難易度はこのプロファイルのすべてのセーブに適用されます。';

  @override
  String get savingDifficultyFailed => '難易度の保存に失敗しました。';

  @override
  String get addItemDialogTitle => 'アイテムを追加';

  @override
  String get searchItems => 'アイテムを検索';

  @override
  String failedToLoadCatalog(String error) {
    return 'カタログの読み込みに失敗しました: $error';
  }

  @override
  String get noItemsAvailableToAdd => '追加できるアイテムがありません';

  @override
  String get noItemsMatch => '一致するアイテムがありません';

  @override
  String get countMustBeAtLeast1 => '≥ 1 である必要があります';

  @override
  String countMustBeAtMost(int max) {
    return '≤ $max である必要があります';
  }

  @override
  String get addNpcDialogTitle => 'NPC を追加';

  @override
  String get noNpcsAvailableToAdd => '追加できる NPC がありません';

  @override
  String get noNpcsMatch => '一致する NPC がありません';

  @override
  String get categoryAll => 'すべて';

  @override
  String allWithCount(int count) {
    return 'すべて（$count）';
  }

  @override
  String get addKnowledgeEntryDialogTitle => '知識エントリを追加';

  @override
  String get searchEntries => 'エントリを検索';

  @override
  String get noKnowledgeEntriesAvailableToAdd => '追加できる知識エントリがありません';

  @override
  String get noEntriesMatch => '一致するエントリがありません';

  @override
  String get heroGroupMainStats => '主要ステータス';

  @override
  String get heroGroupCombatSkills => '戦闘スキル';

  @override
  String get heroGroupResistances => '耐性';

  @override
  String get heroGroupThieving => '盗み';

  @override
  String get heroGroupAdvanced => '詳細設定';

  @override
  String get heroEntryHeroTransform => 'ヒーローのトランスフォーム';

  @override
  String attributeEmpty(String name) {
    return '$name が空です — 値を入力するか、保存前に元の値を復元してください。';
  }

  @override
  String attributeInvalidNumber(String name, String text) {
    return '$name の数値が無効です: 「$text」';
  }

  @override
  String get loadingEditorData => 'エディターデータを読み込み中';

  @override
  String localizedTextExtractedCount(int idCount, int languageCount) {
    return '$languageCount言語で$idCount個のIDを抽出しました';
  }
}
