// ignore: unused_import
import 'package:intl/intl.dart' as intl;
import 'app_localizations.dart';

// ignore_for_file: type=lint

/// The translations for Japanese (`ja`).
class AppLocalizationsJa extends AppLocalizations {
  AppLocalizationsJa([String locale = 'ja']) : super(locale);

  @override
  String get debugSectionTitle => '詳細（デバッグ）';

  @override
  String get debugSectionSubtitle => 'バグ報告用の診断と生データ';

  @override
  String get showObjectIdsTitle => '追加の技術 ID を表示';

  @override
  String get showObjectIdsSubtitle =>
      'アイテム、会話知識、クエスト、孤立アクターの技術 ID を表示します。NPC ID は常に表示されます。';

  @override
  String get storyStateSidebar => 'ストーリー状態';

  @override
  String get storyStateDescription =>
      '製品版ゲームスクリプトで宣言された永続ストーリー状態の正式なカタログです。保存済み項目には生の値を表示し、このセーブにないカタログ項目は未設定として示します。ソースで宣言された時刻はゲーム内時刻として表示します。その他の整数は真偽値、カウンター、多段階状態などの場合があります。';

  @override
  String get storyStateReadOnly =>
      'スクリプト上の意味と安全なマップ書き込みが確認できるまでは読み取り専用です。関連する用語集テキストは文脈であり、技術 ID の直接翻訳ではありません。';

  @override
  String get storyStateStructureReadOnly =>
      'このセーブデータの StoryPropertyValues 構造を一意かつ安全に特定できませんでした。このセーブデータではストーリー値は読み取り専用のままです。';

  @override
  String get storyStateSearch => 'ストーリー状態を検索';

  @override
  String storyStateValuesCount(int shown, int total) {
    return 'ストーリー値 $total 件中 $shown 件';
  }

  @override
  String get storyStateInteger => '整数';

  @override
  String get storyStateTimeMarker => '時刻マーカー';

  @override
  String get storyStateChapter => 'チャプター';

  @override
  String get storyStateUnknown => '不明なソース型';

  @override
  String get storyStateUnknownDetail =>
      'この保存済み ID は現在のスクリプトカタログに存在しません（Mod または新しいゲーム版など）。保存形式の値は int32 ですが、その意味は推測しません。';

  @override
  String get storyStateStored => '保存済み';

  @override
  String get storyStateUnset => '未設定';

  @override
  String get storyStateUnsetDetail =>
      'このカタログ項目はセーブにシリアライズされていないため、ゲームは未設定または既定の状態を使用します。';

  @override
  String get storyStateRawValue => '生の値';

  @override
  String storyStateElapsed(String duration) {
    return 'セーブ時の経過時間: $duration';
  }

  @override
  String storyStateAhead(String duration) {
    return 'セーブ時点から先: $duration';
  }

  @override
  String storyStateDurationDays(int days, String time) {
    return '$days日 $time';
  }

  @override
  String get storyStateRelatedGlossary => '関連する用語集項目';

  @override
  String get storyStateTechnicalPath => '技術パス';

  @override
  String get storyStateEditingGuidance =>
      'すべての項目は、符号付き int32 の全範囲で編集できます。スクリプトに基づくフラグや値の候補は参考情報であり、生の値はいつでも入力できます。ストーリー状態を変更すると、会話、クエスト、ワールドの遷移が飛ばされる場合があります。内容を確認して保存してください。バックアップは自動的に作成されます。';

  @override
  String get storyStatePending => '保留中';

  @override
  String storyStatePendingValue(String value) {
    return '$value として保存されます';
  }

  @override
  String get storyStatePendingRemoval => 'セーブデータから削除されます';

  @override
  String get storyStateEditValue => '値を編集';

  @override
  String get storyStateSetValue => '値を設定';

  @override
  String get storyStateRemoveValue => 'セーブデータから削除';

  @override
  String get storyStateUndoChange => 'ストーリー変更を元に戻す';

  @override
  String get storyStateResetChanges => 'ストーリー変更をリセット';

  @override
  String storyStateDialogTitle(String id) {
    return '$id を編集';
  }

  @override
  String get storyStateRawInput => '符号付き int32 値';

  @override
  String get storyStateInvalidInt32 =>
      '-2147483648 から 2147483647 までの整数を入力してください。';

  @override
  String get storyStateQueueChange => '変更を保留';

  @override
  String storyStateSuggestedValues(String values) {
    return '製品版スクリプトで確認された値: $values';
  }

  @override
  String get storyStateSuggestionsNotLimits =>
      '候補は検証上の制限ではありません。ネイティブコード、MOD、または新しいゲームバージョンでは別の値が使われる場合があります。';

  @override
  String get storyStateUseCurrentTime => '現在のセーブ時刻を使用';

  @override
  String get storyStateStructuredTime => '日 / 時刻';

  @override
  String get storyStateRawMode => '生の int32';

  @override
  String get storyStateChapterWarning =>
      'チャプターだけを変更しても、クエスト、NPC、インベントリ、ワールド状態は同期されません。';

  @override
  String get storyStateDormantWarning =>
      '製品版のスクリプトキャッシュでは、このフィールドを実際に読み書きする箇所が見つかりませんでした。旧仕様、ネイティブコードによる制御、または予約済みのフィールドである可能性があります。';

  @override
  String get storyStateReadOnlySourceWarning =>
      '製品版スクリプトはこのフィールドを読み取りますが、スクリプトから書き込む箇所はありません。ネイティブコードが管理している可能性があります。';

  @override
  String get storyStateUnknownEditWarning =>
      'この ID は MOD または新しいバージョン由来であり、同梱ソースから意味を判定できません。生の int32 値だけを編集してください。';

  @override
  String storyStateIntegerKind(String kind) {
    String _temp0 = intl.Intl.selectLogic(kind, {
      'binaryFlag': 'バイナリフラグ',
      'finiteState': '多段階の値',
      'counterOrScore': 'カウンター / スコア',
      'calendarDay': '暦日',
      'derivedOrOpaqueInteger': '派生 / 不透明な整数',
      'readOnlyInSourceInteger': '製品版スクリプトでは読み取り専用',
      'dormantOrLegacyInteger': '製品版スクリプトでは未使用',
      'other': '整数',
    });
    return '$_temp0';
  }

  @override
  String get storyStateZeroVsUnset =>
      '保存された 0 とマップ項目が存在しない状態は、ファイル上では異なります。「セーブデータから削除」を選ぶと、コンストラクターまたは既定の状態に戻ります。';

  @override
  String get appTitle => 'GORE Save Editor';

  @override
  String get appLogoSemanticLabel => 'GORE Save Editor のロゴ';

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
  String get tabAttribute => '属性';

  @override
  String get heroGroupSkills => 'スキル';

  @override
  String get skillsNoneBody => 'このキャラクターのスキルは見つかりませんでした。';

  @override
  String get skillsUnavailableBody =>
      'このセーブデータではスキルを編集できません。ヒーローに変更できるエフェクトデータがありません。';

  @override
  String get skillNotLearned => '未習得';

  @override
  String get skillLearn => '習得する';

  @override
  String get skillActionLearn => '習得';

  @override
  String get skillActionUnlearn => '習得解除';

  @override
  String get skillTierUntrained => '未訓練';

  @override
  String get skillTierBeginner => '初心者';

  @override
  String get skillTierTrained => '訓練済み';

  @override
  String get skillTierMaster => 'マスター';

  @override
  String get skillTierNovice => '見習い';

  @override
  String get skillTierAmateur => '素人（第0サークル）';

  @override
  String get skillTierLearned => '習得済み';

  @override
  String skillTierCircle(int n) {
    return '第$nサークル';
  }

  @override
  String get skillHintBlacksmith1H => '片手武器';

  @override
  String get skillHintBlacksmith2H => '両手武器';

  @override
  String get skillCategoryCombat => '戦闘';

  @override
  String get skillCategoryCrafting => '製作';

  @override
  String get skillCategoryHunting => '狩猟';

  @override
  String get skillCategoryLanguage => '言語';

  @override
  String get skillCategoryMagic => '魔法';

  @override
  String get skillCategoryMovement => '移動';

  @override
  String get skillCategoryThievery => '盗み';

  @override
  String get skillNameOneHanded => '片手持ち';

  @override
  String get skillNameTwoHanded => '両手持ち';

  @override
  String get skillNameFists => '裸の拳';

  @override
  String get skillNameBow => '弓';

  @override
  String get skillNameCrossbow => 'クロスボウ';

  @override
  String get skillNameLockpicking => 'ロックピッキング';

  @override
  String get skillNamePickpocketing => 'スリ';

  @override
  String get skillNameTakeOrgans => '内臓を取る';

  @override
  String get skillNameBreakTeeth => '歯を取る';

  @override
  String get skillNameTakeClaws => '爪を取る';

  @override
  String get skillNameSkinFur => '毛皮を取る';

  @override
  String get skillNameSkin => '皮を取る';

  @override
  String get skillNameTakeFins => 'ひれを取る';

  @override
  String get skillNameTakeStingers => '針を取る';

  @override
  String get skillNameTakeSecretion => '分泌液を取る';

  @override
  String get skillNameTakeSkullPlates => 'スカル・アーマーを取る';

  @override
  String get skillNameSkinSwampshark => 'サメの皮を取る';

  @override
  String get skillNameTakeMinecrawlerPlates => 'プレートを取る';

  @override
  String get skillNameTakeScutes => '骨板を取る';

  @override
  String get skillNameTakeUluMulu => 'ウルムルを取る';

  @override
  String get skillNameOrcWeapons => 'オーク武器';

  @override
  String get skillNameMining => '採掘';

  @override
  String get skillNameDiving => 'ダイビング';

  @override
  String get skillNameTakeMinecrawlerMandibles => 'あごを取る';

  @override
  String get skillNameTakeShadowbeastHorn => '角を取る (Shadowbeast)';

  @override
  String get skillNameTakeSpines => '背骨を取る';

  @override
  String get skillNameBreakSwampsharkTeeth => 'サメの歯を取る';

  @override
  String get skillNameTakeFireTongue => '炎の舌を取る';

  @override
  String get skillNameTakeTrollHorn => '角を取る (Troll)';

  @override
  String get skillNameAcrobatics => 'アクロバティック';

  @override
  String get skillNameWallClimbing => 'クライミング';

  @override
  String get skillNameRiding => 'スカベンジャー乗り';

  @override
  String get skillNameSneaking => 'スニーク';

  @override
  String get skillNameAlchemy => 'アルケミー';

  @override
  String get skillNameRuneInscription => 'インスクリプション';

  @override
  String get skillNameBlacksmithing => '鍛冶';

  @override
  String get skillNameMagicCircle => 'マジック・サークル';

  @override
  String get skillNameOrcish => 'オーク語';

  @override
  String get tabInventory => 'インベントリ';

  @override
  String get tabTrade => '取引';

  @override
  String get traderNotAMerchant => 'このキャラクターは取引をしません。';

  @override
  String get traderRetry => '再試行';

  @override
  String get traderAmbiguousName =>
      '同じ名前の商人レコードが複数あるため、どの店がこのキャラクターのものか判別できません。誤って別の店を変更しないよう、編集は無効です。';

  @override
  String get traderOre => '鉱石（購買力）';

  @override
  String get traderNoOre => '鉱石なし';

  @override
  String get traderStockCurrent => '在庫';

  @override
  String get traderStockBase => '補充の基準';

  @override
  String get traderStockBaseHint => '商人が補充する基準。ストーリーの進行とともに増えるため、初期状態ではありません。';

  @override
  String get traderOreHint =>
      'ゲーム内の数値は異なります。読み込み時に、前回の取引以降に生じた分が加算されます（余剰品を売り、その分で補充します）。この数値は開始値であり、取引画面に表示される額ではありません。';

  @override
  String get traderPriceWarning =>
      '価格は商人の在庫量と保有鉱石に反応します。これらの数値を変えると、提示価格も動くことがあります。';

  @override
  String get traderAddItem => 'アイテムを追加';

  @override
  String get traderRemoveItem => '行を削除';

  @override
  String get traderReadOnlyCore => 'このコアは商人データの読み取りのみ可能です。';

  @override
  String get traderDifficultyStockUnsupported =>
      'この商人は難易度ごとの在庫を持っており、エディタはそれを扱えません。変更は成功したように見えても、その追加在庫はそのまま残るため、ここでの編集は無効です。';

  @override
  String get traderRecordIncomplete =>
      'この商人のレコードには在庫リストの一方が欠けており、エディタでは作成できません。保存時に失敗しないよう、ここでの編集は無効です。';

  @override
  String get traderEmptyStock => '在庫がありません。';

  @override
  String get traderUnknownItem => 'アイテムカタログにありません';

  @override
  String editorTradersLoadFailed(String details) {
    return '商人データの読み込みに失敗しました: $details';
  }

  @override
  String traderStockLineCount(int count) {
    return '$count 行';
  }

  @override
  String get tabWorld => 'ワールド';

  @override
  String get tabCharacters => 'キャラクター';

  @override
  String get characterNoActorBody =>
      'このキャラクターはワールド内のアクターを持たないため、属性、インベントリ、イベントはありません。';

  @override
  String get characterNoEventsBody => 'このキャラクターにはイベントがありません。';

  @override
  String get characterOrphanGroup => 'その他';

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
  String get profile => 'プロフィール';

  @override
  String get otherSaves => 'その他のセーブデータ';

  @override
  String profileWithSaves(String name, int count) {
    return '$name（セーブ $count 件）';
  }

  @override
  String get switchProfile => 'プロフィールを切り替え';

  @override
  String get openSaveFile => 'ファイルを開く';

  @override
  String get externalSave => '外部から開いたセーブデータ';

  @override
  String get saveProfileTitle => 'セーブプロフィール';

  @override
  String get saveProfileDescription =>
      'このセーブデータを別のゲームプロフィールに割り当てます。セーブデータとプロフィールインデックスは一緒にバックアップされます。';

  @override
  String get saveProfileExternalHint =>
      'プロフィールを選択し、このファイルをゲームのセーブフォルダーへインポートして登録します。元のファイルは変更されません。';

  @override
  String get saveProfileNoProfiles =>
      'PersistentDataList.sav に編集可能なゲームプロフィールが見つかりません。';

  @override
  String get saveProfileSelect => 'プロフィールを選択';

  @override
  String get rescanSaveFolder => 'セーブフォルダーを再スキャン';

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
  String bytesValue(String count) {
    return '$count バイト';
  }

  @override
  String get inspectionJsonTitle => '検査 JSON';

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
  String get gameTimeTitle => 'プレイ時間';

  @override
  String get gameTimeDay => '日';

  @override
  String get gameTimeHours => '時間';

  @override
  String get gameTimeMinutes => '分';

  @override
  String get gameTimeSeconds => '秒';

  @override
  String gameTimeTotal(int seconds) {
    return '= 合計 $seconds 秒';
  }

  @override
  String get gameTimeInvalid => '整数を入力してください：日 ≥ 0、時間 0～23、分と秒 0～59。';

  @override
  String get required => '必須';

  @override
  String get playerLockedBody => 'プライベートプレイヤーの編集には圧縮対応のコーデックが必要です。';

  @override
  String get heroTransform => '位置';

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
  String get spawnPositionSection => 'スポーン位置（参考）';

  @override
  String get resetToSpawnPosition => 'スポーン位置に戻す';

  @override
  String get positionOutOfRange => '値は −10,000,000 から 10,000,000 の間で指定してください';

  @override
  String get positionNotEditable => 'このキャラクターの保存された位置を読み取れなかったため、編集できません。';

  @override
  String get positionNeverPlaced =>
      'このキャラクターはワールドに配置されたことがありません（位置 0, 0, 0）。ゲームは保存された位置を無視する場合があります。';

  @override
  String get npcStayInPlace => '日課を無効にする';

  @override
  String get npcStayInPlaceHint => 'その場にとどまります。';

  @override
  String get npcStayInPlaceLocked => '元の日課が記録されていないため、これはもう元に戻せません。';

  @override
  String get npcUndoPlacement => '移動を取り消す';

  @override
  String get npcUndoPlacementStale =>
      'セーブデータにはこの移動が書き込んだ内容がもう残っていません。元に戻すと、その後の変更が失われます。';

  @override
  String get positionNotReadable => 'このキャラクターの保存された位置を読み取れませんでした。';

  @override
  String get npcPositionReadOnly =>
      'ゲームは NPC の位置をセーブデータではなくレベルから復元します。そのため、これらの値は読み取れますが変更できません。';

  @override
  String get pickLocation => '場所を選択…';

  @override
  String get pickLocationDialogTitle => '場所を選択';

  @override
  String get applySpotRotation => '地点の向きも適用する';

  @override
  String get locationAreaOther => 'その他';

  @override
  String get locationAreaCavalornValley => 'カヴァロンの谷';

  @override
  String get locationAreaEastForest => '東の森';

  @override
  String get locationAreaFogTower => '霧の塔';

  @override
  String get locationAreaIllegalWeedMixers => '違法スワンプウィード調合師';

  @override
  String get locationAreaOrcArena => 'オークのアリーナ';

  @override
  String get locationAreaOrcGraveyard => 'オークの墓地';

  @override
  String get locationAreaShipwreck => '難破船';

  @override
  String get locationAreaTundra => '凍原';

  @override
  String get locationCatalogUnavailable => '場所カタログを読み込めませんでした。';

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
  String get attributeBaseValue => '基本値';

  @override
  String get attributeCurrentValue => '現在値';

  @override
  String get inventoryTitle => 'インベントリ';

  @override
  String get inventoryEmpty => 'このインベントリは空です。';

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
  String get resetInventoryButton => 'インベントリをリセット';

  @override
  String get resetInventoryTooltipDefault => 'このインベントリをゲーム開始時のものに置き換えます';

  @override
  String get resetInventoryTooltipBlocked => '先に保留中のインベントリ変更を保存またはキャンセルしてください';

  @override
  String get pendingResetTitle => 'ゲーム開始時のインベントリにリセット';

  @override
  String pendingResetSubtitle(String level) {
    return 'リソースレベル：$level';
  }

  @override
  String get cancelPendingReset => 'リセットをキャンセル';

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
  String get npcStatusRowLabel => '状態';

  @override
  String get npcStatusAlive => '生存';

  @override
  String get npcStatusDead => '死亡';

  @override
  String get npcRelationshipRowLabel => '関係';

  @override
  String get npcRelationshipUnavailable => '関係ステータスを利用できません';

  @override
  String get npcRelationshipAutomatic => 'ゲームが計算';

  @override
  String get npcRelationshipAutomaticHint =>
      '永続的な上書きは保存されていません。ゲーム内でギルド、ストーリー、地域、犯罪のルールが評価されます。';

  @override
  String get npcRelationshipStoredHint =>
      'NPC からプレイヤーへの永続的な上書きとして保存されています。ゲーム内のギルド、ストーリー、地域、犯罪のルールによって実際の関係が変わる場合があります。';

  @override
  String get npcRelationshipFriend => '友好';

  @override
  String get npcRelationshipNeutral => '中立';

  @override
  String get npcRelationshipEnemy => '敵';

  @override
  String npcRelationshipPending(String relationship) {
    return '保存後の関係：$relationship';
  }

  @override
  String npcStateHp(String hp, String maxHp) {
    return 'HP $hp / $maxHp';
  }

  @override
  String get npcReviveButton => '蘇生';

  @override
  String get npcReviveQueued => '保存時に蘇生されます';

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
  String get memoryEventRemovalQueued => 'イベントの削除を保留しました。保存すると適用されます。';

  @override
  String get duplicateEvent => 'イベントを複製';

  @override
  String get duplicateMemoryEventTitle => 'メモリイベントを複製しますか？';

  @override
  String get duplicateMemoryEventBody => 'このメモリイベントを複製しますか？ 事前にバックアップが作成されます。';

  @override
  String get memoryEventDuplicationQueued => 'イベントの複製を保留しました。保存すると適用されます。';

  @override
  String get selectCharacterFromList => 'リストからキャラクターを選択してください';

  @override
  String get factionsSidebar => '派閥';

  @override
  String get factionsForgiveButton => '許す';

  @override
  String get factionHostile => '敵対的';

  @override
  String get factionFriendly => '友好的';

  @override
  String crimeMurder(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '殺人 $count 件',
    );
    return '$_temp0';
  }

  @override
  String crimeAssault(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '暴行 $count 件',
    );
    return '$_temp0';
  }

  @override
  String crimeTheft(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '窃盗 $count 件',
    );
    return '$_temp0';
  }

  @override
  String crimeTrespassing(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '不法侵入 $count 件',
    );
    return '$_temp0';
  }

  @override
  String crimeThreat(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '脅迫 $count 件',
    );
    return '$_temp0';
  }

  @override
  String crimeOther(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: 'その他の罪 $count 件',
    );
    return '$_temp0';
  }

  @override
  String get factionsForgiveQueued => '許し中…';

  @override
  String get factionsEmpty => '派閥に対する未解決の罪はありません。';

  @override
  String get factionGuildOldCamp => 'オールド・キャンプ';

  @override
  String get factionGuildNewCamp => 'ニュー・キャンプ';

  @override
  String get factionGuildSwampCamp => 'スワンプ・キャンプ';

  @override
  String get factionGuildOther => 'その他/個人';

  @override
  String get allDataLockedBody => 'この包括的なデータブラウザーは、現在 GSAV セーブファイルで利用できます。';

  @override
  String get allDataDescription =>
      'GSAV のメタデータと、PUBLIC/PRIVATE の型付きノードをすべて参照できます。安全に扱えるスカラー値とネイティブ構造体の値は編集可能で、コンテナーと未解析のバイト列も表示されます。';

  @override
  String get allDataEditable => '編集可能';

  @override
  String get allDataReadOnly => '読み取り専用';

  @override
  String get allDataType => '型';

  @override
  String get allDataScalars => 'スカラー';

  @override
  String get allDataStructs => '構造体';

  @override
  String get allDataContainers => 'コンテナー';

  @override
  String get allDataOpaque => '未解析データ';

  @override
  String get allDataNodes => 'ノード';

  @override
  String allDataChildren(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '子ノード $count 件',
      one: '子ノード 1 件',
    );
    return '$_temp0';
  }

  @override
  String get allDataPending => '未保存';

  @override
  String get allDataTagInputHint => 'タグをカンマまたは改行で区切って入力';

  @override
  String allDataTypedSource(String source) {
    return '$source（型付き）';
  }

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
  String get profileBackups => 'プロフィールのバックアップ';

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
  String aboutVersion(String version, String sha) {
    return 'バージョン $version（$sha）';
  }

  @override
  String get aboutCopyright => '© 2026 GORE コントリビューター';

  @override
  String get aboutLicense => 'MIT ライセンスの下で提供されています。';

  @override
  String difficultyTitle(String profile) {
    return '難易度 — $profile';
  }

  @override
  String get difficultyNoProfile => 'プロフィールなし';

  @override
  String get difficultyNoDifficulty => '難易度なし';

  @override
  String get difficultyLabel => '難易度';

  @override
  String get difficultyTooltipNoProfile => 'プロフィールが選択されていません';

  @override
  String get difficultyTooltipEdit => 'このプロフィールの難易度を編集';

  @override
  String get difficultyTooltipNoEditable => 'このプロフィールには編集可能な難易度がありません';

  @override
  String get preset => 'プリセット';

  @override
  String get presetNovice => 'イージー';

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
  String get closeCombatFlowHelper => '接近戦フロー・ヘルパー';

  @override
  String get permadeath => 'パーマデス';

  @override
  String get notAvailableOnNovice => '初心者では利用できません';

  @override
  String get levelCombat => '戦闘';

  @override
  String get levelResources => '資源';

  @override
  String get levelProgression => '進行度';

  @override
  String get difficultyAppliesToAllSaves => '難易度はこのプロフィールのすべてのセーブに適用されます。';

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
  String get heroEntryHeroTransform => '位置';

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
  String savingProgress(int done, int total) {
    return '保存中… $done / $total';
  }

  @override
  String localizedTextExtractedCount(int idCount, int languageCount) {
    return '$languageCount言語で$idCount個のIDを抽出しました';
  }

  @override
  String get skillSmithing1H => '片手鍛冶屋';

  @override
  String get skillSmithing2H => '両手鍛冶屋';

  @override
  String get skillCircleNovice => '見習い魔法使い';

  @override
  String get skillCircle1 => '第一魔法円';

  @override
  String get skillCircle2 => '第二魔法円';

  @override
  String get skillCircle3 => '第三魔法円';

  @override
  String get skillCircle4 => '第四魔法円';

  @override
  String get skillCircle5 => '第五魔法円';

  @override
  String get skillCircle6 => '第六魔法円';

  @override
  String get sectionGlossary => '用語集';

  @override
  String get glossarySearch => '用語集を検索';

  @override
  String get glossaryOldCamp => 'オールド・キャンプ';

  @override
  String get glossaryNewCamp => 'ニュー・キャンプ';

  @override
  String get glossarySwampCamp => 'スワンプ・キャンプ';

  @override
  String get glossaryOutsiders => 'よそ者';

  @override
  String get glossaryCreatures => 'クリーチャー';

  @override
  String get glossaryLocations => '場所';

  @override
  String get glossaryFilterLabel => 'フィルター';

  @override
  String get glossaryFilterTraders => '商人';

  @override
  String get glossaryFilterTeachers => '教師';

  @override
  String get glossaryFilterArmorers => '防具職人';

  @override
  String get glossaryFilterHostile => '敵対';

  @override
  String get glossaryRelationshipFilterNote =>
      'セーブデータに保存された永続的な敵対設定を表示します。ギルド、ストーリー、地域、犯罪による動的な関係はゲーム内でのみ計算されます。';

  @override
  String get glossaryFilterDead => '死亡';

  @override
  String get glossaryAddEntry => '用語集エントリを追加';

  @override
  String get glossaryAddTitle => '用語集エントリを追加';

  @override
  String get glossaryResetChanges => '用語集の変更をリセット';

  @override
  String get glossaryNoVisibleEntries => 'この表示に一致する用語集エントリがありません。';

  @override
  String get glossaryNoHiddenEntries => '利用可能なエントリはすべて表示されています。';

  @override
  String get glossaryNoMatch => '一致する用語集エントリがありません。';

  @override
  String get glossarySelectEntry => '編集する用語集エントリを選択してください。';

  @override
  String glossaryEntryCount(int count) {
    return '$count 件';
  }

  @override
  String glossarySegmentsCount(int unlocked, int total) {
    return '$total 件中 $unlocked 件';
  }

  @override
  String get glossaryPortraitUnlocked => '肖像解除済み';

  @override
  String get glossaryPortraitSilhouette => 'シルエット — 肖像未解除';

  @override
  String get glossarySegments => 'エントリ';

  @override
  String get glossaryPending => '未保存の変更';

  @override
  String get glossaryShowFullText => 'エントリ全文を表示';

  @override
  String get glossarySegmentIntroduction => '紹介 / 肖像';

  @override
  String get glossarySegmentUnlock => '発見';

  @override
  String glossarySegmentEntry(int number) {
    return 'エントリ $number';
  }

  @override
  String get questJournalAll => 'すべてのクエスト';

  @override
  String get questJournalOldCamp => 'オールド・キャンプ';

  @override
  String get questJournalNewCamp => 'ニュー・キャンプ';

  @override
  String get questJournalSwampCamp => 'スワンプ・キャンプ';

  @override
  String get questJournalColony => 'コロニー';

  @override
  String get questJournalCompleted => '完了済み';

  @override
  String get questJournalHint =>
      'ゲーム内ジャーナル表示です。内部状態および未開始のクエスト状態は「すべてのデータ」で確認できます。';

  @override
  String get questJournalNoEntries => '現在のフィルターに一致するジャーナルクエストがありません。';

  @override
  String get glossaryTutorials => 'チュートリアル';

  @override
  String get tutorialGateNote =>
      'これらの行は保存されたチュートリアル解除状態を制御します。1 つの解除状態がゲーム内の 1 ページに対応するとは限りません。';

  @override
  String get tutorialResetChanges => 'チュートリアルの変更をリセット';

  @override
  String get tutorialNoGates => 'このセーブデータには利用可能なチュートリアル解除状態がありません。';

  @override
  String tutorialGateUnlockCount(int unlocked, int total) {
    return '$total 件中 $unlocked 件のチュートリアルを解除';
  }

  @override
  String get tutorialGateCombatBasics => '戦闘の基本';

  @override
  String get tutorialGateCrafting => 'クラフト';

  @override
  String get tutorialGateCrime => '犯罪とその結果';

  @override
  String get tutorialGateDrugs => '消耗品と効果';

  @override
  String get tutorialGateLockpicking => '鍵開け';

  @override
  String get tutorialGateMagic => '魔法';

  @override
  String get tutorialGateMap => 'マップ';

  @override
  String get tutorialGateMeleeCombat => '近接戦闘';

  @override
  String get tutorialGateNavigation => '移動とナビゲーション';

  @override
  String get tutorialGatePerception => '知覚';

  @override
  String get tutorialGatePlayerProgression => 'キャラクター進行';

  @override
  String get tutorialGateRanged => '遠距離戦闘';

  @override
  String get tutorialGateRiding => '騎乗';

  @override
  String get tutorialGateSleep => '睡眠';

  @override
  String get tutorialGateTrading => '取引';

  @override
  String get windowMinimizeTooltip => '最小化';

  @override
  String get windowMaximizeTooltip => '最大化';

  @override
  String get windowRestoreTooltip => '元に戻す';

  @override
  String get fallbackDialogEntry => '会話エントリ';

  @override
  String get fallbackDialogChoice => '会話の選択肢';

  @override
  String get fallbackDialogTopic => '会話トピック';

  @override
  String get fallbackDialogInformation => '会話情報';

  @override
  String get fallbackQuest => 'クエスト';

  @override
  String get fallbackObjective => '目標';

  @override
  String get fallbackItem => 'アイテム';

  @override
  String get attributeSkillPointsFallback => 'スキルポイント（LP）';

  @override
  String attributeManualFallbackLabel(String attributeId, String fallback) {
    String _temp0 = intl.Intl.selectLogic(attributeId, {
      'Alcohol': 'アルコール',
      'AlcoholDepletionRate': 'アルコール減少速度',
      'MaxAlcohol': '最大アルコール値',
      'MaxSuperArmor': '最大スーパーアーマー',
      'SuperArmor': 'スーパーアーマー',
      'Fatigue': '疲労',
      'FillRatio': '充填率',
      'FillRatioPeriod': '充填周期',
      'MaxFatigue': '最大疲労',
      'MaxThresholdIndex': '最大しきい値インデックス',
      'RecoveryRatePerHourOfSleep': '睡眠1時間あたりの回復量',
      'DamageMultiplier': 'ダメージ倍率',
      'Toughness': '強靭度',
      'ToughnessA': '強靭度 A',
      'ToughnessB': '強靭度 B',
      'ToughnessC': '強靭度 C',
      'XPExecutedBounty': '処刑時のXP報酬',
      'XPKillOrDefeatBounty': '撃破時のXP報酬',
      'SpeedModifier': '速度補正',
      'CriticalLevelPercent': 'クリティカルレベル（%）',
      'MaxOxygen': '最大酸素量',
      'Oxygen': '酸素',
      'OxygenDepletionRate': '酸素消費速度',
      'OxygenRecoveryRate': '酸素回復速度',
      'MaxRestTime': '最大休息時間',
      'MaxSleepTime': '最大睡眠時間',
      'SleepTime': '睡眠時間',
      'SleepTimeRecoveryAmount': '睡眠回復量',
      'SleepTimeRecoveryPeriod': '睡眠回復周期',
      'MaxSwampweed': '最大沼地草量',
      'Swampweed': '沼地草',
      'SwampweedDepletionRate': '沼地草消費速度',
      'other': '$fallback',
    });
    return '$_temp0';
  }

  @override
  String get knowledgeTypeVoiceLine => 'ボイスライン';

  @override
  String get knowledgeTypeOther => 'その他';

  @override
  String get armorUpgradeUpper => '上部';

  @override
  String get armorUpgradeMiddle => '中央';

  @override
  String get armorUpgradeLower => '下部';

  @override
  String get knowledgeCategoryTopic => 'トピック';

  @override
  String get knowledgeCategoryChoice => '選択肢';

  @override
  String get knowledgeCategoryInfo => '情報';

  @override
  String get statusOk => '正常';

  @override
  String get statusFailed => '失敗';

  @override
  String get missingSaveReference => 'ファイルがありません';

  @override
  String missingSaveReferenceDescription(String slot) {
    return '$slot.sav がありません。削除、移動、または名前変更された可能性がありますが、プロフィールにはまだ参照が残っています。';
  }

  @override
  String get removeFromProfile => 'プロフィールから削除';

  @override
  String get removeSaveFromProfileTitle => 'セーブデータをプロフィールから削除しますか？';

  @override
  String removeSaveFromProfileBody(String save, String profile) {
    return '$save を $profile から削除しますか？セーブファイル自体が存在する場合は保持されます。';
  }

  @override
  String get unassignedSave => 'プロフィールに未割り当て';

  @override
  String get armorUpgradeLight => '軽量';

  @override
  String get armorUpgradeMedium => '中量';

  @override
  String get armorUpgradeHeavy => '重量';

  @override
  String get knowledgeCaptionForcedConversation => '強制会話';

  @override
  String get knowledgeCaptionFollowupTopic => 'フォローアップトピック';

  @override
  String get knowledgeCaptionFallbackTopic => '代替トピック';

  @override
  String durationMinutes(int minutes) {
    return '$minutes分';
  }

  @override
  String durationHours(int hours) {
    return '$hours時間';
  }

  @override
  String durationHoursMinutes(int hours, int minutes) {
    return '$hours時間$minutes分';
  }

  @override
  String get backupStatusInvalidProfileStructure => 'プロフィールデータが無効です';

  @override
  String get backupStatusSlotMetadataMissing => '選択したセーブデータのメタデータがありません';

  @override
  String defaultProfileName(int id) {
    return 'プロフィール $id';
  }

  @override
  String get statusUnknown => '不明';

  @override
  String editorUnexpectedError(String details) {
    return '予期しないエラー: $details';
  }

  @override
  String get editorOperationInProgress => '別の処理を実行中です。しばらくしてからもう一度お試しください。';

  @override
  String get editorUnsavedBeforeDifficulty =>
      'セーブデータに未保存の変更があります。プロフィールの難易度を変更する前に、変更を保存するかリセットしてください。';

  @override
  String get editorNoSaveFolderSelected => 'セーブフォルダーが選択されていません。';

  @override
  String get editorNoSaveSelected => 'セーブデータが選択されていません。';

  @override
  String get coreUnknownError => '不明なコアエラー';

  @override
  String get editorUnsavedBeforeSwitchProfile =>
      'まず未保存の変更を保存するかリセットしてください。プロフィールを切り替えると、現在のセーブデータから移動します。';

  @override
  String get editorUnsavedBeforeOpenFile =>
      '別のファイルを開く前に、未保存の変更を保存するかリセットしてください。';

  @override
  String get editorSelectSavFile => 'セーブデータの .sav ファイルを選択してください。';

  @override
  String get editorNotGothicGsav => '選択したファイルは Gothic GSAV セーブデータではありません。';

  @override
  String get editorUnsavedBeforeChangeSaveProfile =>
      'セーブデータのプロフィールを変更する前に、未保存の変更を保存するかリセットしてください。';

  @override
  String get editorUnsavedBeforeRemoveProfile =>
      'セーブデータをプロフィールから削除する前に、未保存の変更を保存するかリセットしてください。';

  @override
  String get editorUnsavedBeforeRestoreProfile =>
      'セーブデータに未保存の変更があります。プロフィールのバックアップを復元する前に、変更を保存するかリセットしてください。';

  @override
  String editorConflictingPropertyEdits(String path) {
    return '2 つのタブで保留中の変更が同じプロパティ ($path) を対象にしています。一方をリセットするか元に戻してから、もう一度保存してください。';
  }

  @override
  String editorGlossaryMemoryConflict(String path) {
    return '用語集のセグメント変更と全データで保留中の別の変更が、どちらも Hero MemorizedEvents 配列 ($path) を対象にしています。用語集の変更はこの配列のエントリを追加または削除するため、両方を一緒に保存できません。一方をリセットするか元に戻してから、もう一度保存してください。';
  }

  @override
  String editorGlossaryQuestConflict(String path) {
    return '用語集のセグメント変更と保留中の別の変更が、どちらもクエストの同じ CurrentState プロパティ ($path) を対象にしています。用語集の変更自体がこの状態を更新します。一方をリセットするか元に戻してから、もう一度保存してください。';
  }

  @override
  String editorRelationshipConflict(String path) {
    return '関係設定の上書きと全データで保留中の別の変更が、どちらも同じ NPC 関係エントリ ($path) を対象にしています。構造化された関係の変更によって、このエントリ内の補正値が置き換わる可能性があるため、両方を一緒に保存できません。一方をリセットするか元に戻してから、もう一度保存してください。';
  }

  @override
  String editorMultipleStructuralArrayEdits(String path) {
    return '同じ配列 ($path) を対象とする構造変更が複数保留中です。別の変更を追加する前に、最初の変更を保存するかリセットしてください。';
  }

  @override
  String editorStructuralArrayConflict(String path) {
    return 'イベントの構造変更と全データで保留中の別の変更が、どちらも $path を対象にしています。続行する前に、一方を保存するかリセットしてください。';
  }

  @override
  String get editorSkillsEffectConflict =>
      'スキルの変更と、同じキャラクターのエフェクト (ActiveEffects › EffectSpec › Def) に対する全データでの変更が両方とも保留中です。両方を一緒に保存できません。一方をリセットするか元に戻してから、もう一度保存してください。';

  @override
  String get editorInventoryResetConflict =>
      'インベントリのリセットと、同じインベントリへの別の変更が両方とも保留中です。リセットするとインベントリ全体が置き換わり、別の変更が破棄されます。一方をリセットするか元に戻してから、もう一度保存してください。';

  @override
  String get editorUseFolder => 'このフォルダーを使用';

  @override
  String get editorGothicSavegameFileType => 'Gothic セーブデータ';

  @override
  String get editorNoDifficultyChanges => '保存する難易度の変更はありません';

  @override
  String get editorDifficultyWritten => '難易度をプロフィールに保存しました（バックアップを作成しました）';

  @override
  String editorChangesSavedWithBackup(int count) {
    return '$count 件の変更を保存し、バックアップを作成しました';
  }

  @override
  String editorPlacementNoteFailed(String details) {
    return '移動は保存されましたが、取り消し用の記録を書き込めませんでした: $details';
  }

  @override
  String editorProfileNotFound(int profileId) {
    return 'プロフィール $profileId が見つかりません。';
  }

  @override
  String get editorNoFreeSaveSlot =>
      'ゲームのセーブフォルダーに空きスロットがありません（G1R-001～G1R-999）。';

  @override
  String editorSaveImportedAssigned(int profileId) {
    return 'セーブデータをインポートし、プロフィール $profileId に割り当てました';
  }

  @override
  String editorSaveAssigned(int profileId) {
    return 'セーブデータをプロフィール $profileId に割り当てました（対応するバックアップを作成しました）';
  }

  @override
  String editorSaveSlotNotAssigned(String slot, int profileId) {
    return 'セーブスロット $slot はプロフィール $profileId に割り当てられていません。';
  }

  @override
  String get editorSaveRemovedFromProfile => 'セーブデータをプロフィールから削除しました';

  @override
  String editorRestoredBackup(String path) {
    return 'バックアップを復元しました: $path';
  }

  @override
  String editorRestoredBackupWithoutCompanion(String path) {
    return 'バックアップを復元しました: $path（一致する関連バックアップがないため PersistentDataList.sav は変更されていません。スロットのメタデータが異なる可能性があります）';
  }

  @override
  String editorCodecRoundtripPassed(int chunkIndex, int bytes) {
    return 'コーデックの往復検証に成功しました: チャンク $chunkIndex を $bytes バイトに再圧縮しました';
  }

  @override
  String editorDifficultyWriteFailed(String details) {
    return 'プロフィールの難易度を保存できませんでした: $details';
  }

  @override
  String editorProfileAssignmentFailed(String details) {
    return 'セーブデータをプロフィールに割り当てられませんでした: $details';
  }

  @override
  String editorProfileRemovalFailed(String details) {
    return 'セーブデータをプロフィールから削除できませんでした: $details';
  }

  @override
  String editorSaveFailed(String details) {
    return '変更を保存できませんでした: $details';
  }

  @override
  String editorScanSavesFailed(String details) {
    return 'セーブデータのスキャンに失敗しました: $details';
  }

  @override
  String editorInspectSaveFailed(String details) {
    return 'セーブデータの検査に失敗しました: $details';
  }

  @override
  String editorLoadBackupsFailed(String details) {
    return 'バックアップの読み込みに失敗しました: $details';
  }

  @override
  String editorRestoreFailed(String details) {
    return 'バックアップを復元できませんでした: $details';
  }

  @override
  String editorRestoreReloadFailed(String path, String details) {
    return 'バックアップを復元しました: $path。ただし、セーブデータの再読み込みに失敗しました: $details';
  }

  @override
  String editorCodecCheckFailed(String details) {
    return 'コーデックの確認に失敗しました: $details';
  }

  @override
  String editorCodecValidationFailed(String details) {
    return 'コーデックの往復検証に失敗しました: $details';
  }

  @override
  String editorPropertySearchFailed(String details) {
    return 'プロパティの検索に失敗しました: $details';
  }

  @override
  String get editorSelectionChangedWhileLoadingHeroAttributes =>
      'ヒーローの属性を読み込んでいる間に、選択中のセーブデータが変更されました。';

  @override
  String editorSkillsLoadFailed(String details) {
    return 'スキルの読み込みに失敗しました: $details';
  }

  @override
  String editorProgressionQueryFailed(String details) {
    return '進行状況の照会に失敗しました: $details';
  }

  @override
  String editorNpcListFailed(String details) {
    return 'NPC リストの読み込みに失敗しました: $details';
  }

  @override
  String editorCharacterListFailed(String details) {
    return 'キャラクターリストの読み込みに失敗しました: $details';
  }

  @override
  String editorNpcAttributesFailed(String details) {
    return 'NPC の属性の読み込みに失敗しました: $details';
  }

  @override
  String editorNpcPositionFailed(String details) {
    return 'NPC の位置の読み込みに失敗しました: $details';
  }

  @override
  String editorNpcInventoryFailed(String details) {
    return 'NPC のインベントリの読み込みに失敗しました: $details';
  }

  @override
  String editorFactionListFailed(String details) {
    return '派閥リストの読み込みに失敗しました: $details';
  }

  @override
  String get editorNoBackupPath => 'なし';

  @override
  String editorBackupMessage(String prefix, String backupPath) {
    return '$prefix: $backupPath';
  }

  @override
  String editorBackupMessageWithPersistent(
    String prefix,
    String backupPath,
    String persistentPath,
  ) {
    return '$prefix: $backupPath; PersistentDataList のバックアップ: $persistentPath';
  }

  @override
  String localizationStatusFailed(String details) {
    return 'ローカライズ状況の取得に失敗しました: $details';
  }

  @override
  String localizationExtractionFailed(String details) {
    return '抽出に失敗しました: $details';
  }

  @override
  String glossaryLoadFailed(String details) {
    return '用語集の読み込みに失敗しました: $details';
  }

  @override
  String backupStatusError(String details) {
    return 'バックアップエラー: $details';
  }

  @override
  String memoryEventCategory(String category, String fallback) {
    String _temp0 = intl.Intl.selectLogic(category, {
      'quest': 'クエスト',
      'document': '文書',
      'story': 'ストーリー',
      'exploration': '探索',
      'combat': '戦闘',
      'social': '交流',
      'item': 'アイテム',
      'learning': '習得',
      'guild': 'ギルド',
      'crime': '犯罪',
      'rest': '休息',
      'other': '$fallback',
    });
    return '$_temp0';
  }

  @override
  String memoryEventAction(String kind, String fallback) {
    String _temp0 = intl.Intl.selectLogic(kind, {
      'questStarted': 'クエスト開始',
      'questSucceeded': 'クエスト完了',
      'questFailed': 'クエスト失敗',
      'documentRead': '文書を読んだ',
      'documentSegmentUnlocked': '項目を発見',
      'documentSegmentViewed': '項目を閲覧',
      'chapterCompleted': 'チャプター完了',
      'areaEntered': 'エリアに入った',
      'areaLeft': 'エリアを出た',
      'characterKilled': 'キャラクターを殺害',
      'characterDefeated': 'キャラクターを撃破',
      'combatDodge': '攻撃を回避',
      'characterDebuffed': '弱体効果を付与',
      'tradeAvailable': '取引を解禁',
      'itemObtained': 'アイテム入手',
      'itemCrafted': 'アイテム作成',
      'skillStateRecorded': 'スキル状態を記録',
      'recipeLearned': 'レシピ習得',
      'guildJoined': 'ギルド加入',
      'crimeRecorded': '犯罪を記録',
      'slept': '睡眠',
      'storyEvent': 'ストーリーイベント',
      'other': '$fallback',
    });
    return '$_temp0';
  }

  @override
  String memoryEventTitleWithSubject(String action, String subject) {
    return '$action: $subject';
  }

  @override
  String memoryEventFact(String fact, String fallback) {
    String _temp0 = intl.Intl.selectLogic(fact, {
      'gameTime': 'ゲーム時間',
      'duration': '継続時間',
      'chapter': 'チャプター',
      'instigator': '発生元',
      'affected': '影響対象',
      'amount': '数量',
      'primaryObject': 'オブジェクト',
      'secondaryObject': '関連情報',
      'segmentText': '項目テキスト',
      'other': '$fallback',
    });
    return '$_temp0';
  }

  @override
  String memoryEventGameTime(int day, String time) {
    return '$day日目、$time';
  }

  @override
  String memoryEventSecondsValue(String value) {
    return '$value秒';
  }

  @override
  String memoryEventMoreValues(String values, int count) {
    return '$values +$count';
  }

  @override
  String get memoryEventHero => '主人公';

  @override
  String get memoryEventDetails => '詳細';

  @override
  String get memoryEventTags => 'タグ';

  @override
  String get memoryEventTechnicalData => '技術情報';

  @override
  String get memoryEventIndex => 'インデックス';

  @override
  String get memoryEventPosition => '位置';

  @override
  String get memoryEventPayload => 'イベントデータ';

  @override
  String get memoryEventSubject => '関連対象';

  @override
  String glossaryCatalogSegmentLabel(String segmentId, String fallback) {
    String _temp0 = intl.Intl.selectLogic(segmentId, {
      'Access': '通行許可',
      'AccessDenied': '通行拒否',
      'AccesToTemple': '神殿への立ち入り',
      'Advice': '助言',
      'AfterFight': '戦いの後',
      'AfterFireMages': '炎の魔術師事件の後',
      'AfterNek': 'ネックの後',
      'AfterQuest': 'クエストの後',
      'Alone': '独り',
      'Amulet': 'アミュレット',
      'Annoying': 'うっとうしい',
      'Armor': '防具',
      'Avoid': '避ける',
      'Backstory': '過去',
      'BackStory': '過去',
      'BasicMagic': '魔法の基礎',
      'Beated': '倒された',
      'BecomeMercenary': '傭兵になる',
      'Beer': 'ビール',
      'Bestiary': '魔物図鑑',
      'Blessing': '祝福',
      'Boss': 'ボス',
      'Bully': 'いじめっ子',
      'BullyAdvice': 'いじめへの助言',
      'Camp': 'キャンプ',
      'CampDivided': '分裂したキャンプ',
      'CareOfMessengers': '使者の世話',
      'ChangeOpinion': '考えを変える',
      'ChargeUriziel': 'ウリジエルに力を込める',
      'Chosen': '選ばれし者',
      'Contact': '連絡',
      'Courier': '配達人',
      'CraftBows': '弓の製作',
      'Crazy': '狂気',
      'DailyMeal': '毎日の食事',
      'DailyRation_Trader': '日々の配給係',
      'DAM': 'ダム',
      'Dead': '死亡',
      'Deal': '取引',
      'Dealer': '取引人',
      'Deceived': '騙された',
      'Dementia': '認知症',
      'DenyAccess': '通行拒否',
      'DifferentOpinion': '意見の相違',
      'Discussion': '話し合い',
      'DontTalk': '話しかけるな',
      'Duel': '決闘',
      'Entrance': '入口',
      'Escape': '脱出',
      'Extended': '拡張',
      'Extra': '追加',
      'ExtraInfo': '追加情報',
      'Fanatic': '狂信者',
      'Fight': '戦い',
      'FindUlumulu': 'ウルムルを探す',
      'FireMages': '炎の魔術師',
      'FireMagesEscape': '炎の魔術師の脱出',
      'FiskNewDealer': 'フィスクのための新しい故買人',
      'FiskNewDealerCompleted': 'フィスクのための新しい故買人（完了）',
      'FogTower': '霧の塔',
      'Food': '食料',
      'Forgave': '許した',
      'Forgive': '許す',
      'Forgiven': '許された',
      'FourFriends': '4人の仲間',
      'FreeHut': '空き小屋',
      'FreeMine': 'フリー・マイン',
      'Fury': '激怒',
      'GoodTeacher': '優れた師匠',
      'Gossip': '噂話',
      'GotScavenger': 'スカベンジャーを入手',
      'GrantedAccess': '通行許可済み',
      'GRDArmor': '護衛の防具',
      'Guide': '案内役',
      'HateMages': '魔術師嫌い',
      'HateMagesExplanation': '魔術師嫌いの理由',
      'HateRiceLord': 'ライス・ロードへの憎しみ',
      'Heal': '回復',
      'Healing': '回復',
      'Help': '助け',
      'Helper': '協力者',
      'HelpKagan': 'ケイガンを助ける',
      'HutStory': '小屋の話',
      'Ignore': '無視',
      'Impress': '感心させる',
      'ImpressAlchemy': '錬金術で感心させる',
      'ImpressInscription': '刻印で感心させる',
      'Info': '情報',
      'Interested': '興味あり',
      'Introduction': '初対面',
      'Introduction_2': '2度目の紹介',
      'Introduction_Armor': '防具の紹介',
      'Introduction_Teacher': '初対面（師匠）',
      'Introduction_Trader': '初対面（商人）',
      'Invocation': '召喚',
      'JoinSC': 'スワンプ・キャンプに加入',
      'Joint': 'スワンプウィード巻き',
      'KalomCamp': 'コル・カロムの野営地',
      'Leader': '指導者',
      'Learning': '修行',
      'LearnOrcish': 'オーク語を学ぶ',
      'LeftParty': 'パーティー離脱',
      'Library': '図書館',
      'Lie': '嘘',
      'Lock': '鍵',
      'Lockpick': 'ロックピック',
      'Mad': '正気を失った',
      'Mandibles': 'マインクローラーのあご',
      'MapMaker': '地図職人',
      'Monastery': '修道院',
      'MordragKO': 'モードラッグを倒す',
      'Nek': 'ネック',
      'NewCamp': 'ニュー・キャンプ',
      'NewCamper': '新入り',
      'NewLeader': '新たな指導者',
      'NightPatrol': '夜間巡回',
      'NotInterested': '興味なし',
      'OldCamp': 'オールド・キャンプ',
      'OrcEnclaveEntrance': 'オークの飛地の入口',
      'OrcGraveyard': 'オーク墓地',
      'OreArmor': '鉱石の防具',
      'Party': 'パーティー',
      'Pay': '支払い',
      'PayMoney': '金を払う',
      'Permission': '許可',
      'Pet': 'ペット',
      'PreparingInvocation': '召喚の準備',
      'Quest': 'クエスト',
      'RankUpFireMages': '炎の魔術師への昇格',
      'RankUpGuard': '護衛への昇格',
      'RanUpFireMagesCompleted': '炎の魔術師への昇格完了',
      'Realocated': '移動済み',
      'Reason': '理由',
      'Respect': '敬意',
      'ReturnToSC': 'スワンプ・キャンプへ戻る',
      'RicelordForeman': 'ライス・ロードの監督',
      'RideScavenger': 'スカベンジャーに乗る',
      'Robe': 'ローブ',
      'Safe': '安全',
      'Scraper': 'スクレーパー',
      'SecondChance': '二度目の機会',
      'SecretLocation': '秘密の場所',
      'SecretPassage': '秘密の通路',
      'SecretPath': '秘密の道',
      'SleeperFollower': 'スリーパーの信徒',
      'SleeperTemple': 'スリーパーの神殿',
      'SmallInfo': 'ちょっとした情報',
      'Stonehenge': 'ストーンヘンジ',
      'StopFollowing': '追従をやめる',
      'SwampCamp': 'スワンプ・キャンプ',
      'Talkative': 'おしゃべり',
      'Teach': '訓練',
      'TeachBow': '弓術訓練',
      'Teacher': '師匠',
      'Teacher2': '2人目の師匠',
      'TeacherInscription': '刻印の師匠',
      'TeacherMana': 'マナの師匠',
      'TeachIchor': 'マインクローラーの膿漿の採取訓練',
      'TeachMagic': '魔法の訓練',
      'TeachOrcish': 'オーク語の訓練',
      'TeachStats': '能力値の訓練',
      'TeachWeapon': '武器の訓練',
      'Teleport': 'テレポート',
      'TheMysteriousOrc': '謎のオーク',
      'ThroneRoom': '玉座の間',
      'TradeBow': '弓の取引',
      'Trader': '商人',
      'TradeSkins_Trader': '毛皮商人',
      'Traitor': '裏切り者',
      'Trial': '試練',
      'TrollCanyon': 'トロールの峡谷',
      'Trust': '信頼',
      'Ulumulu': 'ウルムル',
      'Unexperienced': '未熟',
      'Uriziel': 'ウリジエル',
      'UrizielRune': 'ウリジエルのルーン',
      'Useful': '役に立つ',
      'Velaya': 'ベラヤ',
      'Vibrations': '振動',
      'WaitFreeMine': 'フリー・マインで待つ',
      'WaitInTrainingArea': '訓練場で待つ',
      'Warning': '警告',
      'WarningTooLate': '手遅れの警告',
      'WaterMessenger': '水の魔術師の使者',
      'Weapon': '武器',
      'Who': '正体',
      'Women': '女性たち',
      'other': '$fallback',
    });
    return '$_temp0';
  }

  @override
  String get slotRepairTitle => 'インベントリスロットの破損';

  @override
  String slotRepairBody(int count) {
    return 'このセーブデータには、ID が位置と一致しないインベントリスロットが $count 個あります。ゲーム内でそのアイテムを捨てると、別のアイテムが消えます。修復はスロット ID を書き直すだけで、アイテムの追加・削除・変更は行いません。 保存時には通常どおりバックアップが作成されます。';
  }

  @override
  String get slotRepairQueued => '修復を予約しました。保存すると適用されます。';

  @override
  String get slotRepairAction => '修復';

  @override
  String get slotRepairDiscard => '取り消す';

  @override
  String get editorInventorySlotEditConflict =>
      'インベントリスロットへの直接編集と、スロットごと扱う操作（修復・追加・削除）が同時に予約されています。後者が前者を上書きします。どちらかを取り消してから保存し直してください。';

  @override
  String get editorTraderArrayConflict =>
      '取引の変更が、商人配列への直接編集と一緒に予約されています。その編集は取引の変更が参照する行番号を振り直すため、どちらかが別の商人に当たります。片方を取り消してから保存してください。';

  @override
  String get backupFactFile => 'ファイル';

  @override
  String get renameBackupTooltip => 'このバックアップに名前を付ける';

  @override
  String get renameBackupTitle => 'バックアップ名';

  @override
  String get renameBackupLabel => '名前';

  @override
  String renameBackupHelp(String fileName) {
    return 'ファイル名 $fileName の代わりに表示されます。空にすると名前を削除します。ファイル自体の名前は変わりません。';
  }

  @override
  String get deleteBackupTooltip => 'このバックアップを削除';

  @override
  String get deleteBackupTitle => 'バックアップの削除';

  @override
  String deleteBackupBody(String name, String fileName) {
    return '「$name」（$fileName）を削除しますか？ ファイルはディスクから削除され、元に戻せません。';
  }

  @override
  String get deleteBackupConfirm => '削除';

  @override
  String editorDeletedBackup(String path) {
    return 'バックアップを削除しました: $path';
  }

  @override
  String editorDeleteBackupFailed(String details) {
    return 'バックアップを削除できませんでした: $details';
  }

  @override
  String editorRenameBackupFailed(String details) {
    return 'バックアップに名前を付けられませんでした: $details';
  }

  @override
  String get slotRepairUnavailable => '現在は修復できません。このセーブデータには書き込めません。';

  @override
  String editorDeletedBackupWithLabelWarning(String path, String details) {
    return 'バックアップを削除しました: $path — その名前は削除できませんでした: $details';
  }

  @override
  String get slotRepairNotOffered => 'このセーブデータでは修復を利用できません。';
}
