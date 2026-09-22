// ignore: unused_import
import 'package:intl/intl.dart' as intl;
import 'app_localizations.dart';

// ignore_for_file: type=lint

/// The translations for Chinese (`zh`).
class AppLocalizationsZh extends AppLocalizations {
  AppLocalizationsZh([String locale = 'zh']) : super(locale);

  @override
  String get debugSectionTitle => '高级（调试）';

  @override
  String get debugSectionSubtitle => '用于错误报告的诊断和原始数据';

  @override
  String get showObjectIdsTitle => '显示其他技术 ID';

  @override
  String get showObjectIdsSubtitle => '显示物品、对话知识、任务和孤立角色的技术 ID。NPC ID 始终显示。';

  @override
  String get storyStateSidebar => '剧情状态';

  @override
  String get storyStateDescription =>
      '游戏在这里记录任务、对话和事件的进度。“已保存”显示存档中的数值，“未设置”显示其他已知条目。根据条目的不同，数字可能表示是或否、次数或进度阶段。时间标记表示游戏内的日期和时间。';

  @override
  String get storyStateReadOnly =>
      '在确认脚本含义和安全的映射写入方式前保持只读。关联的词条文本仅提供上下文，并非技术 ID 的直接翻译。';

  @override
  String get storyStateStructureReadOnly =>
      '无法唯一且安全地确定此存档中的 StoryPropertyValues 结构。此存档的剧情值将保持只读。';

  @override
  String get storyStateSearch => '搜索剧情状态';

  @override
  String storyStateValuesCount(int shown, int total) {
    return '已显示 $shown 个，共 $total 个剧情值';
  }

  @override
  String get storyStateInteger => '整数';

  @override
  String get storyStateTimeMarker => '时间标记';

  @override
  String get storyStateChapter => '章节';

  @override
  String get storyStateUnknown => '未知源码类型';

  @override
  String storyStateShowDormant(int count) {
    return '显示未使用（$count）';
  }

  @override
  String get storyStateUnknownDetail =>
      '当前脚本目录中没有此已保存 ID（例如来自模组或更新的游戏版本）。其存档线值为 int32，但不会推断其含义。';

  @override
  String get storyStateStored => '已保存';

  @override
  String get storyStateUnset => '未设置';

  @override
  String get storyStateUnsetDetail => '此目录字段未序列化到该存档中，因此游戏会使用未设置或默认状态。';

  @override
  String get storyStateRawValue => '原始值';

  @override
  String storyStateElapsed(String duration) {
    return '保存时已过去：$duration';
  }

  @override
  String storyStateAhead(String duration) {
    return '保存时尚在未来：$duration';
  }

  @override
  String storyStateDurationDays(int days, String time) {
    return '$days 天 $time';
  }

  @override
  String get storyStateRelatedGlossary => '关联词条';

  @override
  String get storyStateTechnicalPath => '技术路径';

  @override
  String get storyStateEditingGuidance =>
      '选择条目即可修改数值，修改会在保存后生效。请只修改你了解其作用的数值，否则任务或对话可能无法按预期进行。保存时会自动创建备份。';

  @override
  String get storyStatePending => '待处理';

  @override
  String storyStatePendingValue(String value) {
    return '将保存为 $value';
  }

  @override
  String get storyStatePendingRemoval => '将从存档中移除';

  @override
  String get storyStateEditValue => '编辑值';

  @override
  String get storyStateSetValue => '设置值';

  @override
  String get storyStateRemoveValue => '从存档中移除';

  @override
  String get storyStateUndoChange => '撤销剧情更改';

  @override
  String get storyStateResetChanges => '重置剧情更改';

  @override
  String storyStateDialogTitle(String id) {
    return '编辑 $id';
  }

  @override
  String get storyStateRawInput => '有符号 int32 值';

  @override
  String get storyStateInvalidInt32 => '请输入 -2147483648 到 2147483647 之间的整数。';

  @override
  String get storyStateQueueChange => '将更改加入队列';

  @override
  String storyStateSuggestedValues(String values) {
    return '随游戏提供的脚本中已确认的值：$values';
  }

  @override
  String get storyStateSuggestionsNotLimits =>
      '建议值不是验证限制；原生代码、模组或后续游戏版本可能会使用其他值。';

  @override
  String get storyStateUseCurrentTime => '使用当前存档时间';

  @override
  String get storyStateStructuredTime => '天数 / 时间';

  @override
  String get storyStateRawMode => '原始 int32';

  @override
  String get storyStateChapterWarning => '仅更改章节不会同步任务、NPC、物品栏或世界状态。';

  @override
  String get storyStateDormantWarning =>
      '在随游戏提供的脚本缓存中未找到对此字段的有效读取或写入。它可能是旧字段、由原生代码控制，或为保留字段。';

  @override
  String get storyStateReadOnlySourceWarning =>
      '随游戏提供的脚本会读取此字段，但没有通过脚本写入。它仍可能由原生代码管理。';

  @override
  String get storyStateUnknownEditWarning =>
      '这个来自模组或后续版本的 ID 没有内置的源码语义。请仅编辑其原始 int32 值。';

  @override
  String storyStateIntegerKind(String kind) {
    String _temp0 = intl.Intl.selectLogic(kind, {
      'binaryFlag': '二进制标志',
      'finiteState': '多状态值',
      'counterOrScore': '计数器 / 分数',
      'calendarDay': '日历日',
      'derivedOrOpaqueInteger': '派生 / 不透明整数',
      'readOnlyInSourceInteger': '随附脚本中只读',
      'dormantOrLegacyInteger': '随附脚本中未使用',
      'other': '整数',
    });
    return '$_temp0';
  }

  @override
  String get storyStateZeroVsUnset =>
      '已保存的 0 与映射中没有该条目是两种不同的文件状态。“从存档中移除”会恢复构造函数或默认状态。';

  @override
  String get appTitle => 'GORE Save Editor';

  @override
  String get appLogoSemanticLabel => 'GORE Save Editor 徽标';

  @override
  String get zoomTooltip => '按 Ctrl +/- 放大/缩小';

  @override
  String get switchToLightMode => '切换到浅色模式';

  @override
  String get switchToDarkMode => '切换到深色模式';

  @override
  String get about => '关于';

  @override
  String get tabOverview => '概览';

  @override
  String get tabPlayer => '玩家';

  @override
  String get tabAttribute => '属性';

  @override
  String get heroGroupSkills => '技能';

  @override
  String get skillsNoneBody => '未找到该角色的技能。';

  @override
  String get skillsUnavailableBody => '此存档无法编辑技能——主角没有可修改的效果数据。';

  @override
  String get skillNotLearned => '未习得';

  @override
  String get skillLearn => '学习';

  @override
  String get skillActionLearn => '学习';

  @override
  String get skillActionUnlearn => '遗忘';

  @override
  String get skillTierUntrained => '未受过训练';

  @override
  String get skillTierBeginner => '初学者';

  @override
  String get skillTierTrained => '训练有素';

  @override
  String get skillTierMaster => '大师级';

  @override
  String get skillTierNovice => '熟练';

  @override
  String get skillTierAmateur => '业余（第0环）';

  @override
  String get skillTierLearned => '已习得';

  @override
  String skillTierCircle(int n) {
    return '第$n环';
  }

  @override
  String get skillHintBlacksmith1H => '单手武器';

  @override
  String get skillHintBlacksmith2H => '双手武器';

  @override
  String get skillScutesTrained => '熟练（骨板）';

  @override
  String get skillScutesMaster => '大师（＋剃刀兽角板）';

  @override
  String get skillCategoryCombat => '战斗';

  @override
  String get skillCategoryCrafting => '制作';

  @override
  String get skillCategoryHunting => '狩猎';

  @override
  String get skillCategoryLanguage => '语言';

  @override
  String get skillCategoryMagic => '魔法';

  @override
  String get skillCategoryMovement => '移动';

  @override
  String get skillCategoryThievery => '盗窃';

  @override
  String get skillCategoryOther => '其他';

  @override
  String get skillNameOneHanded => '单手';

  @override
  String get skillNameTwoHanded => '双手';

  @override
  String get skillNameFists => '赤手空拳';

  @override
  String get skillNameBow => '弓';

  @override
  String get skillNameCrossbow => '弩';

  @override
  String get skillNameLockpicking => '开锁';

  @override
  String get skillNamePickpocketing => '扒窃';

  @override
  String get skillNameTakeOrgans => '摘取内脏';

  @override
  String get skillNameBreakTeeth => '摘取牙齿';

  @override
  String get skillNameTakeClaws => '摘取爪子';

  @override
  String get skillNameSkinFur => '拿取毛皮';

  @override
  String get skillNameSkin => '拿取皮肤';

  @override
  String get skillNameTakeFins => '拿取鳍';

  @override
  String get skillNameTakeStingers => '摘取刺';

  @override
  String get skillNameTakeSecretion => '摘取分泌物';

  @override
  String get skillNameTakeSkullPlates => '拿取头骨甲';

  @override
  String get skillNameSkinSwampshark => '拿取鲨鱼皮';

  @override
  String get skillNameTakeMinecrawlerPlates => '拿取护甲板';

  @override
  String get skillNameTakeScutes => '拿取鳞甲';

  @override
  String get skillNameTakeUluMulu => '拿取乌鲁木鲁';

  @override
  String get skillNameOrcWeapons => '兽人武器';

  @override
  String get skillNameMining => '采矿';

  @override
  String get skillNameDiving => '潜水';

  @override
  String get skillNameTakeMinecrawlerMandibles => '摘取下颌';

  @override
  String get skillNameTakeShadowbeastHorn => '拿取角 (Shadowbeast)';

  @override
  String get skillNameTakeSpines => '摘取脊柱';

  @override
  String get skillNameBreakSwampsharkTeeth => '摘取鲨鱼牙';

  @override
  String get skillNameTakeFireTongue => '拿取火蜥蜴的舌';

  @override
  String get skillNameTakeTrollHorn => '拿取角 (Troll)';

  @override
  String get skillNameAcrobatics => '杂技';

  @override
  String get skillNameWallClimbing => '攀登';

  @override
  String get skillNameRiding => '骑乘食尸鸟';

  @override
  String get skillNameSneaking => '潜行';

  @override
  String get skillNameAlchemy => '炼金术';

  @override
  String get skillNameRuneInscription => '铭刻';

  @override
  String get skillNameBlacksmithing => '锻造';

  @override
  String get skillNameMagicCircle => '魔法环';

  @override
  String get skillNameOrcish => '兽人语';

  @override
  String get tabInventory => '物品栏';

  @override
  String get tabTrade => '交易';

  @override
  String get traderNotAMerchant => '该角色不进行交易。';

  @override
  String get traderRetry => '重试';

  @override
  String get traderAmbiguousName => '有多条商人记录使用这个名字，无法判断哪家店属于该角色。已禁用编辑，以免改错。';

  @override
  String get traderOre => '矿石（购买力）';

  @override
  String get traderNoOre => '无矿石';

  @override
  String get traderStockCurrent => '已保存库存';

  @override
  String get traderStockCurrentTooltip => '目前为该商人保存的库存。游戏下次更新商人时，添加的物品可能会消失。';

  @override
  String get traderStockBase => '参考库存';

  @override
  String get traderStockBaseTooltip =>
      '这是已保存的库存副本，游戏可按该商人的规则更改或重新生成。这里只读显示，添加的物品不会永久保留。';

  @override
  String get traderStockBaseHint => '只读。此库存会随剧情推进而增加，也可能按商人规则被替换。它不是游戏最初的库存。';

  @override
  String get traderCurrentStockWarning => '商人库存的更改只会保留到下次补货。';

  @override
  String get traderRestockTitle => '预计补货时间';

  @override
  String get traderRestockTitleTooltip => '根据商人的上次活动、游戏时间和资源难度估算。';

  @override
  String get traderRestockPending => '待处理';

  @override
  String get traderRestockRevertTooltip => '撤销尚未保存的上次活动更改';

  @override
  String get traderRestockNever => '从未';

  @override
  String get traderRestockUnavailable => '不可用';

  @override
  String get traderRestockIntervalUnknown => '游戏天数未知';

  @override
  String get traderRestockNeverStatus => '尚未记录该商人的活动。';

  @override
  String get traderRestockClockAhead => '商人的上次活动晚于当前游戏时间。';

  @override
  String traderRestockNotDueYet(String time) {
    return '预计不会早于 $time。';
  }

  @override
  String get traderRestockPossiblyDue => '估算：库存可能已经可以更新。';

  @override
  String get traderRestockEligible => '估算：现在应该补货。';

  @override
  String get traderRestockNoWorldTime => '当前游戏时间不可用，因此无法估算补货时间。';

  @override
  String get traderRestockLastActivity => '上次商人活动';

  @override
  String get traderRestockLastActivityTooltip =>
      '此保存时间可能会在交易后或游戏更新库存时改变。它不一定就是上次补货的时间。';

  @override
  String get traderRestockForecastWindow => '预计时间';

  @override
  String get traderRestockForecastWindowTooltip =>
      '显示最早和最晚可能补货的时间。存档中没有游戏的确切规则，因此这只是估算。';

  @override
  String get traderRestockIntervalLabel => '两次补货之间的天数';

  @override
  String traderRestockInterval(int days, String level) {
    return '$days 天 · $level';
  }

  @override
  String get traderRestockIntervalTooltip =>
      '按资源难度计算：新手 2 天、Gothic 3 天、困难 5 个游戏日。';

  @override
  String get traderRestockAutomationLabel => '自动补货';

  @override
  String get traderRestockAutomationValue => '无法在存档中禁用';

  @override
  String get traderRestockAutomationTooltip => '无法在存档中关闭自动补货。只有模组能改变这项游戏规则。';

  @override
  String get traderRestockSetNow => '设为游戏时间';

  @override
  String get traderRestockSetNowTooltip =>
      '将当前游戏时间（包括尚未保存的更改）设为商人的上次活动。这会推迟预计补货时间。';

  @override
  String get traderRestockMakeDue => '准备补货';

  @override
  String get traderRestockMakeDueTooltip => '将上次活动向过去调整，使其达到预计补货时间。';

  @override
  String get traderRestockCustom => '自定义时间…';

  @override
  String get traderRestockCustomTooltip => '为商人的上次活动选择游戏内日期和时间。';

  @override
  String get traderRestockEditTitle => '商人的上次活动';

  @override
  String get traderOreHint =>
      '游戏内的数值会不同：载入时游戏会加上自他上次交易以来累积的部分——他会卖掉多余货物并以此补货。这个数字是起点，而非交易界面显示的金额。';

  @override
  String get traderOreHintShort => '初始值——可能与交易界面中的金额不同。';

  @override
  String get traderRestockStatusLabel => '状态';

  @override
  String get traderRestockStatusNever => '无活动';

  @override
  String get traderRestockStatusWaiting => '等待补货';

  @override
  String get traderRestockStatusReady => '可以补货';

  @override
  String get traderRestockStatusPossiblyReady => '可能可以补货';

  @override
  String get traderRestockStatusCheckTime => '检查保存时间';

  @override
  String get traderRestockStatusUnknown => '未知';

  @override
  String get traderPriceWarning => '价格会随商人的库存量和持有矿石而变化，因此修改这些数字也可能改变他的开价。';

  @override
  String get traderAddItem => '添加物品';

  @override
  String get traderRemoveItem => '移除条目';

  @override
  String get traderReadOnlyCore => '此核心版本只能读取商人数据。';

  @override
  String get traderDifficultyStockUnsupported =>
      '该商人拥有按难度区分的库存，编辑器并未建模。此处已禁用编辑，因为修改看似成功，却会让这部分额外库存原封不动。';

  @override
  String get traderRecordIncomplete =>
      '该商人的库存清单缺失，或其结构编辑器不支持、无法写入。此处已禁用编辑，以免修改在保存时失败。';

  @override
  String get traderEmptyStock => '没有库存。';

  @override
  String get traderUnknownItem => '不在物品目录中';

  @override
  String editorTradersLoadFailed(String details) {
    return '商人数据加载失败：$details';
  }

  @override
  String traderStockLineCount(int count) {
    return '$count 件商品';
  }

  @override
  String get tabWorld => '世界';

  @override
  String get tabCharacters => '角色';

  @override
  String get characterNoActorBody => '该角色在世界中没有对应的实体，因此没有属性、物品栏或事件。';

  @override
  String get characterNoEventsBody => '该角色没有事件。';

  @override
  String get characterOrphanGroup => '其他';

  @override
  String get tabAllData => '全部数据';

  @override
  String get tabBackups => '备份';

  @override
  String get tabSettings => '设置';

  @override
  String get reset => '重置';

  @override
  String get save => '保存';

  @override
  String saveWithCount(int count) {
    return '保存（$count）';
  }

  @override
  String get ok => '确定';

  @override
  String get cancel => '取消';

  @override
  String get confirm => '确认';

  @override
  String get close => '关闭';

  @override
  String get add => '添加';

  @override
  String get equippedBadge => '已装备';

  @override
  String get armorUpgradesLabel => '升级';

  @override
  String get browse => '浏览';

  @override
  String get noSavFilesFound => '未找到 .sav 文件';

  @override
  String get profile => '存档配置';

  @override
  String get otherSaves => '其他存檔';

  @override
  String profileWithSaves(String name, int count) {
    return '$name（$count 个存档）';
  }

  @override
  String get switchProfile => '切换存档配置';

  @override
  String get openSaveFile => '打开文件';

  @override
  String get externalSave => '从外部打开的存档';

  @override
  String get saveProfileTitle => '存档配置';

  @override
  String get saveProfileDescription => '将此存档分配给另一个游戏存档配置。存档和存档配置索引将一同备份。';

  @override
  String get saveProfileExternalHint => '选择一个存档配置，将此文件导入游戏存档文件夹并在其中登记。原文件不会更改。';

  @override
  String get saveProfileNoProfiles =>
      '在 PersistentDataList.sav 中未找到可编辑的游戏存档配置。';

  @override
  String get saveProfileSelect => '选择存档配置';

  @override
  String get rescanSaveFolder => '重新扫描存档文件夹';

  @override
  String get discardUnsavedChangesTitle => '放弃未保存的更改？';

  @override
  String rescanDiscardBody(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '更改',
      one: '更改',
    );
    return '重新扫描将重新加载每个存档，并放弃你 $count 项未保存的$_temp0。';
  }

  @override
  String get discardAndRescan => '放弃并重新扫描';

  @override
  String chapterLabel(Object id) {
    return '第 $id 章';
  }

  @override
  String get quickSave => '快速存档';

  @override
  String get autoSave => '自动存档';

  @override
  String get manualSave => '手动存档';

  @override
  String get errorTitle => '错误';

  @override
  String get selectASaveTitle => '选择存档';

  @override
  String get selectASaveBody => '存档详情将显示在此处。';

  @override
  String bytesValue(String count) {
    return '$count 字节';
  }

  @override
  String get inspectionJsonTitle => '检查 JSON';

  @override
  String get copy => '复制';

  @override
  String get savegameFallbackTitle => '存档';

  @override
  String screenshotForSlot(String slot) {
    return '$slot 的截图';
  }

  @override
  String get publicSaveName => '名称';

  @override
  String get gameTimeTitle => '游戏时间';

  @override
  String get gameTimeDay => '天';

  @override
  String get gameTimeHours => '小时';

  @override
  String get gameTimeMinutes => '分钟';

  @override
  String get gameTimeSeconds => '秒';

  @override
  String gameTimeTotal(int seconds) {
    return '= 共 $seconds 秒';
  }

  @override
  String get gameTimeInvalid => '请输入整数：天数 ≥ 0，小时 0–23，分钟和秒数 0–59。';

  @override
  String get required => '必填';

  @override
  String get playerLockedBody => '编辑私有玩家数据需要支持压缩的编解码器。';

  @override
  String get heroTransform => '位置';

  @override
  String get locationX => '位置 X';

  @override
  String get locationY => '位置 Y';

  @override
  String get locationZ => '位置 Z';

  @override
  String get rotationPitch => '旋转俯仰';

  @override
  String get rotationYaw => '旋转偏航';

  @override
  String get rotationRoll => '旋转翻滚';

  @override
  String get spawnPositionSection => '出生位置（参考）';

  @override
  String get resetToSpawnPosition => '重置为出生位置';

  @override
  String get positionOutOfRange => '数值必须介于 −10,000,000 与 10,000,000 之间';

  @override
  String get positionNotEditable => '无法读取该角色已保存的位置，因此无法编辑。';

  @override
  String get positionNeverPlaced => '该角色从未在世界中放置过（位置 0, 0, 0）——游戏可能会忽略已保存的位置。';

  @override
  String get npcStayInPlace => '停用他的日常作息';

  @override
  String get npcStayInPlaceHint => '他會留在原地。';

  @override
  String get npcStayInPlaceLocked => '他原本的日常作息沒有被記錄下來，因此無法再還原。';

  @override
  String get npcUndoPlacement => '撤銷這次移動';

  @override
  String get npcUndoPlacementStale => '存檔已不再是這次移動當時寫入的樣子，還原會丟棄此後發生的變化。';

  @override
  String get positionNotReadable => '无法读取该角色已保存的位置。';

  @override
  String get npcPositionReadOnly => '游戏会从关卡而非存档中恢复 NPC 的位置，因此这些数值可以查看，但无法修改。';

  @override
  String get pickLocation => '选择地点…';

  @override
  String get pickLocationDialogTitle => '选择地点';

  @override
  String get applySpotRotation => '同时应用该地点的朝向';

  @override
  String get locationAreaOther => '其他';

  @override
  String get locationAreaCavalornValley => '卡瓦隆山谷';

  @override
  String get locationAreaEastForest => '东部森林';

  @override
  String get locationAreaFogTower => '雾塔';

  @override
  String get locationAreaIllegalWeedMixers => '非法沼泽烟草调配者';

  @override
  String get locationAreaOrcArena => '兽人竞技场';

  @override
  String get locationAreaOrcGraveyard => '兽人墓地';

  @override
  String get locationAreaShipwreck => '沉船残骸';

  @override
  String get locationAreaTundra => '苔原';

  @override
  String get locationCatalogUnavailable => '无法加载地点目录。';

  @override
  String get invalid => '无效';

  @override
  String get heroAttributes => '主角属性';

  @override
  String attributeBase(String name) {
    return '$name 基础值';
  }

  @override
  String attributeCurrent(String name) {
    return '$name 当前值';
  }

  @override
  String get attributeBaseValue => '基础值';

  @override
  String get attributeCurrentValue => '当前值';

  @override
  String get inventoryTitle => '物品栏';

  @override
  String get inventoryEmpty => '此物品栏为空。';

  @override
  String get inventoryNeedsDecoded => '编辑物品栏需要来自编解码器的已解码私有负载数据。';

  @override
  String get inventoryNoStacks => '已解码的私有负载中未找到物品堆叠。';

  @override
  String get resetInventoryChanges => '重置物品栏更改';

  @override
  String get addItemTooltipPendingAdd => '请先保存待处理的更改 — 每次保存只能添加一件新物品';

  @override
  String get addItemTooltipPendingRemove => '请先保存待处理的移除 — 每次保存只能进行一项结构更改';

  @override
  String get addItemTooltipPendingCount => '请先保存或重置待处理的数量更改 — 结构编辑必须单独保存';

  @override
  String get addItemTooltipDefault => '向物品栏添加物品';

  @override
  String get addItemButton => '添加物品';

  @override
  String get resetInventoryButton => '重置物品栏';

  @override
  String get resetInventoryTooltipDefault => '将此物品栏替换为游戏开始时的物品栏';

  @override
  String get resetInventoryTooltipBlocked => '请先保存或取消待处理的物品栏更改';

  @override
  String get pendingResetTitle => '重置为游戏开始时的物品栏';

  @override
  String pendingResetSubtitle(String level) {
    return '资源等级：$level';
  }

  @override
  String get cancelPendingReset => '取消重置';

  @override
  String pendingAddSubtitle(int count) {
    return '×$count — 待添加（尚未保存）';
  }

  @override
  String get cancelPendingAdd => '取消待添加';

  @override
  String get pendingRemovalSubtitle => '待移除（尚未保存）';

  @override
  String get cancelPendingRemoval => '取消待移除';

  @override
  String get filterItems => '筛选物品';

  @override
  String noItemsMatchQuery(String query) {
    return '没有物品匹配“$query”。';
  }

  @override
  String get pendingRemovalHidesAll => '待处理的移除隐藏了所有物品 — 请保存以应用。';

  @override
  String categoryWithCount(String label, int count) {
    return '$label（$count）';
  }

  @override
  String get itemTooltipIngredientFor => '用于制作';

  @override
  String itemTooltipTeaches(String item) {
    return '传授: $item';
  }

  @override
  String get itemTooltipValue => '价值';

  @override
  String get itemTooltipProtection => '防御';

  @override
  String get itemTooltipRequirements => '需求：';

  @override
  String get itemTooltipManaCost => '法力消耗';

  @override
  String get itemTooltipManaUpkeep => '蓄力法力消耗';

  @override
  String get itemCategoryAll => '全部';

  @override
  String get itemCategoryMeleeWeapon => '近战武器';

  @override
  String get itemCategoryRangedWeapon => '远程武器';

  @override
  String get itemCategoryMagic => '魔法';

  @override
  String get itemCategoryWearable => '穿戴装备';

  @override
  String get itemCategoryFood => '食物';

  @override
  String get itemCategoryPotion => '药水';

  @override
  String get itemCategoryMaterial => '材料';

  @override
  String get itemCategoryDocument => '文件';

  @override
  String get itemCategoryMisc => '杂项';

  @override
  String get itemCategoryArtefact => '神器';

  @override
  String get itemCategoryOther => '其他';

  @override
  String get count => '数量';

  @override
  String get min1 => '最少 1';

  @override
  String countTimes(String count) {
    return '×$count';
  }

  @override
  String get deleteEquippedTooltip => '无法删除：该物品可能已装备或已分配到快捷键槽位';

  @override
  String get removeBlockedTooltip => '请先保存或重置待处理的物品栏更改 — 添加或移除必须单独保存';

  @override
  String get removeItemFromInventory => '从物品栏移除物品';

  @override
  String get progressionLockedBody => '进度数据需要来自编解码器的已解码私有负载数据。';

  @override
  String get progressionNeedsTyped => '结构化进度数据需要完全解码且已验证类型解析的存档。';

  @override
  String get sectionQuests => '任务';

  @override
  String get sectionKnowledge => '知识';

  @override
  String get sectionEvents => '事件';

  @override
  String get firstPage => '首页';

  @override
  String get previousPage => '上一页';

  @override
  String get nextPage => '下一页';

  @override
  String get lastPage => '末页';

  @override
  String pageOfPages(int page, int total) {
    return '第 $page / $total 页';
  }

  @override
  String rangeOfTotal(int first, int last, int total) {
    return '$first–$last，共 $total';
  }

  @override
  String get perPage => '每页：';

  @override
  String get resetQuestChanges => '重置任务更改';

  @override
  String get searchQuests => '搜索任务';

  @override
  String get allGroups => '所有分组';

  @override
  String groupWithCount(String group, Object count) {
    return '$group（$count）';
  }

  @override
  String stateLabelWithCount(String label, int count) {
    return '$label $count';
  }

  @override
  String get questStateNone => '无';

  @override
  String get questStateAvailable => '可接取';

  @override
  String get questStateRunning => '进行中';

  @override
  String get questStateSucceeded => '已完成';

  @override
  String get questStateFailed => '已失败';

  @override
  String get questStateUnknown => '未知';

  @override
  String get dialogKnowledge => '对话知识';

  @override
  String get resetKnowledgeChanges => '重置知识更改';

  @override
  String get addNpc => '添加 NPC';

  @override
  String get searchNpcs => '搜索 NPC';

  @override
  String get npcStatusRowLabel => '状态';

  @override
  String get npcStatusAlive => '存活';

  @override
  String get npcStatusDead => '已死亡';

  @override
  String get npcRelationshipRowLabel => '关系';

  @override
  String get npcRelationshipUnavailable => '关系状态不可用';

  @override
  String get npcRelationshipAutomatic => '由游戏计算';

  @override
  String get npcRelationshipAutomaticHint => '未保存永久覆盖。游戏会评估公会、剧情、区域和犯罪规则。';

  @override
  String get npcRelationshipStoredHint =>
      '已保存为 NPC 对玩家的永久覆盖。公会、剧情、区域和犯罪规则仍可能改变游戏中的实际关系。';

  @override
  String get npcRelationshipFriend => '友好';

  @override
  String get npcRelationshipNeutral => '中立';

  @override
  String get npcRelationshipEnemy => '敌人';

  @override
  String npcRelationshipPending(String relationship) {
    return '保存后将为$relationship';
  }

  @override
  String npcStateHp(String hp, String maxHp) {
    return 'HP $hp / $maxHp';
  }

  @override
  String get npcReviveButton => '复活';

  @override
  String get npcReviveQueued => '将在保存时复活';

  @override
  String entriesForCharacter(String name) {
    return '条目 — $name';
  }

  @override
  String get selectNpcToSeeEntries => '选择一个 NPC 以查看条目';

  @override
  String get addKnowledgeEntry => '添加知识条目';

  @override
  String get browseCatalog => '浏览目录';

  @override
  String get alreadyExistsForCharacter => '该角色已存在此项。';

  @override
  String get alreadyInPendingChanges => '已在待处理的更改中。';

  @override
  String duplicateCheckFailed(String error) {
    return '重复检查失败 — 请重试：$error';
  }

  @override
  String pendingAddsCount(int count) {
    return '待添加（$count）';
  }

  @override
  String get undoAdd => '撤销添加';

  @override
  String get undoRemove => '撤销移除';

  @override
  String get removeEntry => '移除条目';

  @override
  String get selectNpcFromList => '从列表中选择一个 NPC';

  @override
  String characterWithCount(String name, int count) {
    return '$name（$count）';
  }

  @override
  String get memoryEvents => '记忆事件';

  @override
  String get searchCharacters => '搜索角色';

  @override
  String eventsForCharacter(String name) {
    return '事件 — $name';
  }

  @override
  String get selectCharacterToSeeEvents => '选择一个角色以查看事件';

  @override
  String get noTags => '（无标签）';

  @override
  String eventSubtitle(String time, String affected) {
    return 't=${time}s  $affected';
  }

  @override
  String get removeEvent => '移除事件';

  @override
  String get removeMemoryEventTitle => '移除记忆事件？';

  @override
  String get removeMemoryEventBody => '移除此记忆事件？将先写入一份备份。';

  @override
  String get memoryEventRemovalQueued => '事件移除已加入队列 — 按“保存”以应用。';

  @override
  String get duplicateEvent => '复制事件';

  @override
  String get duplicateMemoryEventTitle => '复制记忆事件？';

  @override
  String get duplicateMemoryEventBody => '复制此记忆事件？将先写入一份备份。';

  @override
  String get memoryEventDuplicationQueued => '事件复制已加入队列 — 按“保存”以应用。';

  @override
  String get selectCharacterFromList => '从列表中选择一个角色';

  @override
  String get factionsSidebar => '阵营';

  @override
  String get factionsForgiveButton => '宽恕';

  @override
  String get factionHostile => '敌对';

  @override
  String get factionFriendly => '友好';

  @override
  String crimeMurder(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count 起谋杀',
    );
    return '$_temp0';
  }

  @override
  String crimeAssault(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count 起袭击',
    );
    return '$_temp0';
  }

  @override
  String crimeTheft(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count 起盗窃',
    );
    return '$_temp0';
  }

  @override
  String crimeTrespassing(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count 起擅闯',
    );
    return '$_temp0';
  }

  @override
  String crimeThreat(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count 起威胁',
    );
    return '$_temp0';
  }

  @override
  String crimeOther(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count 起其他罪行',
    );
    return '$_temp0';
  }

  @override
  String get factionsForgiveQueued => '宽恕中…';

  @override
  String get factionsEmpty => '没有针对阵营的未了罪行。';

  @override
  String get factionGuildOldCamp => '旧营地';

  @override
  String get factionGuildNewCamp => '新营地';

  @override
  String get factionGuildSwampCamp => '沼泽营地';

  @override
  String get factionGuildOther => '其他/个人';

  @override
  String get allDataLockedBody => '目前，完整的数据源浏览器可用于 GSAV 存档文件。';

  @override
  String get allDataDescription =>
      '浏览 GSAV 元数据以及 PUBLIC/PRIVATE 中的所有类型化节点。可安全修改的标量值和原生结构体值均可编辑；容器和未解析的字节数据也会显示。';

  @override
  String get allDataEditable => '可编辑';

  @override
  String get allDataReadOnly => '只读';

  @override
  String get allDataType => '类型';

  @override
  String get allDataScalars => '标量';

  @override
  String get allDataStructs => '结构体';

  @override
  String get allDataContainers => '容器';

  @override
  String get allDataOpaque => '未解析数据';

  @override
  String get allDataNodes => '节点';

  @override
  String allDataChildren(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count 个子节点',
      one: '1 个子节点',
    );
    return '$_temp0';
  }

  @override
  String get allDataPending => '待保存';

  @override
  String get allDataTagInputHint => '用逗号或换行分隔标签';

  @override
  String allDataTypedSource(String source) {
    return '$source 类型化数据';
  }

  @override
  String get searchPropertiesLabel => '搜索属性（留空 = 列出全部） — 例如 Health、GameTime';

  @override
  String get decodingSaveTitle => '正在解码存档…';

  @override
  String get decodingSaveBody => '正在为首次搜索解码完整的私有负载。此操作每个存档只运行一次，之后的搜索将立即完成。';

  @override
  String get searchTheSaveTitle => '搜索存档';

  @override
  String get searchTheSaveBody => '输入属性名称并按回车键。留空则列出全部。';

  @override
  String get searchFailedTitle => '搜索失败';

  @override
  String get noMatchesTitle => '无匹配项';

  @override
  String get noMatchesBody => '没有属性路径包含所有这些词条。';

  @override
  String get value => '值';

  @override
  String get backupsTitle => '备份';

  @override
  String get refreshBackups => '刷新备份';

  @override
  String get noBackupsTitle => '无备份';

  @override
  String get noBackupsBody => '编辑存档时会在所选槽位旁创建备份文件。';

  @override
  String get slotBackups => '槽位备份';

  @override
  String get profileBackups => '存档配置备份';

  @override
  String get backupFactName => '名称';

  @override
  String get backupFactSlot => '槽位';

  @override
  String get backupFactCreated => '创建时间';

  @override
  String get backupFactSize => '大小';

  @override
  String get backupFactStatus => '状态';

  @override
  String get backupFactSha1 => 'SHA-1';

  @override
  String restoreBackupTooltip(String fileName) {
    return '恢复 $fileName';
  }

  @override
  String get appearanceTitle => '外观';

  @override
  String get uiFont => '字体';

  @override
  String get theme => '主题';

  @override
  String get themeLight => '浅色';

  @override
  String get themeDark => '深色';

  @override
  String get themeSystem => '跟随系统';

  @override
  String get uiScale => '界面缩放';

  @override
  String get resetZoomTooltip => '重置缩放（Ctrl+0）';

  @override
  String get zoomTip => '提示：在应用内任意位置按 Ctrl + / Ctrl - 均可调整缩放。';

  @override
  String get language => '界面';

  @override
  String get gameTextLanguage => '游戏文本';

  @override
  String get gameTextLanguageHint => '选择界面语言时，也会选中对应的游戏文本。之后可以单独更改游戏文本。';

  @override
  String get updatesTitle => '更新';

  @override
  String get checkForUpdatesAutomatically => '自动检查更新';

  @override
  String get checkForUpdatesNow => '立即检查更新';

  @override
  String get updatesPortableNotice => '便携版会在浏览器中打开下载页面。请用新下载的文件替换现有文件。';

  @override
  String get updateAvailableTitle => '有可用更新';

  @override
  String updateAvailableMessage(Object version, Object current) {
    return '版本 $version 可用。您当前为 $current。';
  }

  @override
  String get updateDownload => '下载';

  @override
  String updateOpenFailed(String url) {
    return '无法打开下载页面。你可以通过 $url 访问。';
  }

  @override
  String get updateLater => '稍后';

  @override
  String get updateUpToDate => '您正在使用最新版本。';

  @override
  String get updateCheckFailed => '无法检查更新，请稍后重试。';

  @override
  String get gameTextTitle => '游戏文本';

  @override
  String get itemImagesTitle => '物品图片';

  @override
  String get gameDataTitle => '游戏数据';

  @override
  String itemImagesReady(int count) {
    return '已准备 $count 张物品图片。';
  }

  @override
  String get itemImagesUnavailable => '物品图片不可用，将改用分类图标。';

  @override
  String get checkRefreshItemImages => '检查 / 更新物品图片';

  @override
  String get gameDataSourceMissing => '无法自动准备游戏文本。你可以在设置中选择本地化缓存。';

  @override
  String get loadingTexts => '正在加载文本…';

  @override
  String get loadingImages => '正在加载图片…';

  @override
  String get preparing => '正在准备…';

  @override
  String gameTextExtractedWithCounts(int ids, int languages) {
    return '已提取：$languages 种语言共 $ids 个 ID。';
  }

  @override
  String get gameTextExtracted => '本地化游戏文本已提取。';

  @override
  String get gameTextNotExtracted => '本地化游戏文本尚未提取。';

  @override
  String get extracting => '正在提取…';

  @override
  String get extractRefreshLocalizedText => '提取 / 刷新本地化文本';

  @override
  String get extractionComplete => '提取完成';

  @override
  String get extractionFailed => '提取失败';

  @override
  String get localizationCacheFileType => '本地化缓存';

  @override
  String get savegameDirectoryTitle => '存档目录';

  @override
  String get folder => '文件夹';

  @override
  String get codecTitle => '编解码器';

  @override
  String get check => '检查';

  @override
  String get roundtrip => '往返测试';

  @override
  String get noCodecStatus => '无编解码器状态';

  @override
  String get codecReady => '编解码器就绪';

  @override
  String get codecReadOnly => '编解码器只读';

  @override
  String get codecUnavailable => '编解码器不可用';

  @override
  String get details => '详情';

  @override
  String codecStatusLine(String status) {
    return '状态：$status';
  }

  @override
  String codecCapabilityLine(String decompress, String compress) {
    return '解压：$decompress | 压缩：$compress';
  }

  @override
  String codecBackendLine(String backend) {
    return '后端：$backend';
  }

  @override
  String get yes => '是';

  @override
  String get no => '否';

  @override
  String aboutVersion(String version, String sha) {
    return '版本 $version（$sha）';
  }

  @override
  String get aboutCopyright => '© 2026 Daniel Hoer';

  @override
  String get aboutLicense => '基于 MIT 许可证授权。';

  @override
  String difficultyTitle(String profile) {
    return '难度 — $profile';
  }

  @override
  String get difficultyNoProfile => '无存档配置';

  @override
  String get difficultyNoDifficulty => '无难度';

  @override
  String get difficultyLabel => '难度';

  @override
  String get difficultyTooltipNoProfile => '未选择存档配置';

  @override
  String get difficultyTooltipEdit => '编辑此存档配置的难度';

  @override
  String get difficultyTooltipNoEditable => '此存档配置没有可编辑的难度';

  @override
  String get preset => '预设';

  @override
  String get presetNovice => '简单';

  @override
  String get presetGothic => '哥特';

  @override
  String get presetHard => '困难';

  @override
  String get presetCustom => '自定义';

  @override
  String unrecognisedPreset(Object preset) {
    return '存储的预设无法识别（$preset）。你仍可保存流畅助手 / 永久死亡的更改，或在上方选择一个预设以覆盖它。';
  }

  @override
  String get closeCombatFlowHelper => '近战流程助手';

  @override
  String get permadeath => '永久死亡';

  @override
  String get notAvailableOnNovice => '新手难度下不可用';

  @override
  String get levelCombat => '战斗';

  @override
  String get levelResources => '资源';

  @override
  String get levelProgression => '进度';

  @override
  String get difficultyAppliesToAllSaves => '难度将应用于此存档配置中的所有存档。';

  @override
  String get savingDifficultyFailed => '保存难度失败。';

  @override
  String get addItemDialogTitle => '添加物品';

  @override
  String get searchItems => '搜索物品';

  @override
  String failedToLoadCatalog(String error) {
    return '加载目录失败：$error';
  }

  @override
  String get noItemsAvailableToAdd => '没有可添加的物品';

  @override
  String get noItemsMatch => '没有匹配的物品';

  @override
  String get countMustBeAtLeast1 => '必须 ≥ 1';

  @override
  String countMustBeAtMost(int max) {
    return '必须 ≤ $max';
  }

  @override
  String get addNpcDialogTitle => '添加 NPC';

  @override
  String get noNpcsAvailableToAdd => '没有可添加的 NPC';

  @override
  String get noNpcsMatch => '没有匹配的 NPC';

  @override
  String get categoryAll => '全部';

  @override
  String allWithCount(int count) {
    return '全部（$count）';
  }

  @override
  String get addKnowledgeEntryDialogTitle => '添加知识条目';

  @override
  String get searchEntries => '搜索条目';

  @override
  String get noKnowledgeEntriesAvailableToAdd => '没有可添加的知识条目';

  @override
  String get noEntriesMatch => '没有匹配的条目';

  @override
  String get heroGroupMainStats => '主要属性';

  @override
  String get heroGroupCombatMovement => '战斗 / 移动';

  @override
  String get heroGroupResistances => '抗性';

  @override
  String get heroGroupThieving => '盗窃';

  @override
  String get heroGroupAdvanced => '高级';

  @override
  String get heroGroupDiving => '潜水';

  @override
  String get heroDivingSkillNote =>
      '学会潜水后，游戏每次读取存档都会把屏息量和恢复速度重置为技能自带的数值。每秒消耗量则保持你设定的值。';

  @override
  String get heroGroupSleep => '睡眠';

  @override
  String get heroGroupIntoxication => '醉酒';

  @override
  String get heroEntryHeroTransform => '位置';

  @override
  String attributeEmpty(String name) {
    return '$name 为空 — 请输入一个值，或在保存前恢复原始值。';
  }

  @override
  String attributeInvalidNumber(String name, String text) {
    return '$name 的数字无效：“$text”';
  }

  @override
  String get loadingEditorData => '正在加载编辑器数据';

  @override
  String savingProgress(int done, int total) {
    return '正在保存… $done/$total';
  }

  @override
  String localizedTextExtractedCount(int idCount, int languageCount) {
    return '已提取 $idCount 个 ID，涵盖 $languageCount 种语言';
  }

  @override
  String get skillSmithing1H => '单手武器锻造';

  @override
  String get skillSmithing2H => '双手武器锻造';

  @override
  String get skillCircleNovice => '新手法师';

  @override
  String get skillCircle1 => '第一魔法环阶';

  @override
  String get skillCircle2 => '第二魔法环阶';

  @override
  String get skillCircle3 => '第三魔法环阶';

  @override
  String get skillCircle4 => '第四魔法环阶';

  @override
  String get skillCircle5 => '第五魔法环阶';

  @override
  String get skillCircle6 => '第六魔法环阶';

  @override
  String get sectionGlossary => '图鉴';

  @override
  String get glossarySearch => '搜索图鉴';

  @override
  String get glossaryOldCamp => '旧营地';

  @override
  String get glossaryNewCamp => '新营地';

  @override
  String get glossarySwampCamp => '沼泽营地';

  @override
  String get glossaryOutsiders => '外来者';

  @override
  String get glossaryCreatures => '生物';

  @override
  String get glossaryLocations => '地点';

  @override
  String get glossaryFilterLabel => '筛选';

  @override
  String get glossaryFilterTraders => '商人';

  @override
  String get glossaryFilterTeachers => '导师';

  @override
  String get roleTrader => '商人';

  @override
  String get roleDead => '已死亡';

  @override
  String get roleTeacher => '导师';

  @override
  String get roleArmorer => '护甲匠';

  @override
  String get glossaryFilterArmorers => '护甲匠';

  @override
  String get glossaryFilterHostile => '敌对';

  @override
  String get glossaryRelationshipFilterNote =>
      '显示存档中保存的永久敌对覆盖。公会、剧情、区域和犯罪产生的动态关系仅在游戏中计算。';

  @override
  String get glossaryFilterDead => '已死亡';

  @override
  String get glossaryAddEntry => '添加图鉴条目';

  @override
  String get glossaryAddTitle => '添加图鉴条目';

  @override
  String get glossaryResetChanges => '重置图鉴更改';

  @override
  String get glossaryNoVisibleEntries => '此视图中没有匹配的可见图鉴条目。';

  @override
  String get glossaryNoHiddenEntries => '所有可用条目均已显示。';

  @override
  String get glossaryNoMatch => '没有匹配的图鉴条目。';

  @override
  String get glossarySelectEntry => '选择一个图鉴条目以编辑其内容。';

  @override
  String glossaryEntryCount(int count) {
    return '$count 个条目';
  }

  @override
  String glossarySegmentsCount(int unlocked, int total) {
    return '已解锁 $unlocked/$total 个条目';
  }

  @override
  String get glossaryPortraitUnlocked => '肖像已解锁';

  @override
  String get glossaryPortraitSilhouette => '剪影 — 肖像尚未解锁';

  @override
  String get glossarySegments => '条目';

  @override
  String get glossaryPending => '未保存的更改';

  @override
  String get glossaryShowFullText => '显示条目全文';

  @override
  String get glossarySegmentIntroduction => '介绍 / 肖像';

  @override
  String get glossarySegmentUnlock => '发现';

  @override
  String glossarySegmentEntry(int number) {
    return '条目 $number';
  }

  @override
  String get questJournalAll => '所有任务';

  @override
  String get questJournalOldCamp => '旧营地';

  @override
  String get questJournalNewCamp => '新营地';

  @override
  String get questJournalSwampCamp => '沼泽营地';

  @override
  String get questJournalColony => '殖民地';

  @override
  String get questJournalCompleted => '已完成';

  @override
  String get questJournalHint => '游戏内日志视图。内部状态和尚未开始的任务状态仍可在“所有数据”中查看。';

  @override
  String get questJournalNoEntries => '没有符合当前筛选条件的日志任务。';

  @override
  String get glossaryTutorials => '教程';

  @override
  String get tutorialGateNote => '这些行控制存档中的教程解锁状态。一个解锁状态不一定对应游戏中的单个教程页面。';

  @override
  String get tutorialResetChanges => '重置教程更改';

  @override
  String get tutorialNoGates => '此存档中没有可用的教程解锁状态。';

  @override
  String tutorialGateUnlockCount(int unlocked, int total) {
    return '已解锁 $unlocked/$total 个教程';
  }

  @override
  String get tutorialGateCombatBasics => '战斗基础';

  @override
  String get tutorialGateCrafting => '制作';

  @override
  String get tutorialGateCrime => '犯罪与后果';

  @override
  String get tutorialGateDrugs => '消耗品与效果';

  @override
  String get tutorialGateLockpicking => '开锁';

  @override
  String get tutorialGateMagic => '魔法';

  @override
  String get tutorialGateMap => '地图';

  @override
  String get tutorialGateMeleeCombat => '近战';

  @override
  String get tutorialGateNavigation => '移动与导航';

  @override
  String get tutorialGatePerception => '感知';

  @override
  String get tutorialGatePlayerProgression => '角色成长';

  @override
  String get tutorialGateRanged => '远程战斗';

  @override
  String get tutorialGateRiding => '骑乘';

  @override
  String get tutorialGateSleep => '睡眠';

  @override
  String get tutorialGateTrading => '交易';

  @override
  String get windowMinimizeTooltip => '最小化';

  @override
  String get windowMaximizeTooltip => '最大化';

  @override
  String get windowRestoreTooltip => '还原';

  @override
  String get fallbackDialogEntry => '对话条目';

  @override
  String get fallbackDialogChoice => '对话选项';

  @override
  String get fallbackDialogTopic => '对话主题';

  @override
  String get fallbackDialogInformation => '对话信息';

  @override
  String get fallbackQuest => '任务';

  @override
  String get fallbackObjective => '目标';

  @override
  String get fallbackItem => '物品';

  @override
  String get attributeSkillPointsFallback => '学习点数（LP）';

  @override
  String attributeManualFallbackLabel(String attributeId, String fallback) {
    String _temp0 = intl.Intl.selectLogic(attributeId, {
      'SuperArmor': '霸体值',
      'MaxSuperArmor': '最大霸体值',
      'DamageMultiplier': '受到的伤害',
      'SpeedModifier': '移动速度',
      'Oxygen': '氧气量',
      'MaxOxygen': '最大氧气量',
      'OxygenDepletionRate': '每秒氧气消耗',
      'OxygenRecoveryRate': '每秒氧气恢复',
      'CriticalLevelPercent': '缺氧警告阈值',
      'SleepTime': '剩余有效睡眠',
      'MaxSleepTime': '最大有效睡眠',
      'SleepTimeRecoveryAmount': '有效睡眠回补量',
      'SleepTimeRecoveryPeriod': '回补间隔',
      'MaxRestTime': '最长卧床时间',
      'Health_RecoveryRatePerHourOfSleep': '每小时睡眠回复生命',
      'Mana_RecoveryRatePerHourOfSleep': '每小时睡眠回复法力',
      'Alcohol': '酒精值',
      'MaxAlcohol': '最大酒精值',
      'AlcoholDepletionRate': '醒酒速度',
      'Swampweed': '沼泽草值',
      'MaxSwampweed': '最大沼泽草值',
      'SwampweedDepletionRate': '药性消退速度',
      'XPExecutedBounty': '倒地处决获得的经验',
      'XPKillOrDefeatBounty': '击败获得的经验',
      'Level': '等级',
      'LockpickDurability': '开锁器耐久',
      'LockpickPrecision': '开锁精度',
      'PickPocketing': '扒窃',
      'other': '$fallback',
    });
    return '$_temp0';
  }

  @override
  String attributeManualTooltip(String attributeId) {
    String _temp0 = intl.Intl.selectLogic(attributeId, {
      'SuperArmor': '该角色在被一击打得踉跄之前还能扛下多少打击。',
      'MaxSuperArmor': '霸体值的上限，会随着等级提升和所穿的护甲一起增长。',
      'DamageMultiplier': '作用于该角色所受伤害的系数——1 为正常，数值越高越吃痛。',
      'SpeedModifier': '该角色移动快慢的系数——1 为正常。',
      'Oxygen': '水下剩余的呼吸秒数，归零时该角色就会淹死。',
      'MaxOxygen': '该角色能在水下待多少秒，潜水技能可以提高这个上限。',
      'OxygenDepletionRate': '潜在水下时每秒消耗掉的空气量。',
      'OxygenRecoveryRate': '浮出水面后每秒回来的空气量。',
      'CriticalLevelPercent': '剩余空气低到这个比例时，游戏就会发出溺水警告。',
      'SleepTime': '还能带来恢复的睡眠小时数，超出之后再睡游戏也不会给任何恢复。',
      'MaxSleepTime': '该角色能攒下的有效睡眠时间上限。',
      'SleepTimeRecoveryAmount': '每次补充时重新加回来的有效睡眠小时数。',
      'SleepTimeRecoveryPeriod': '有效睡眠时间隔多久才会重新补满。',
      'MaxRestTime': '游戏允许一次躺在床上的最长时间。',
      'Health_RecoveryRatePerHourOfSleep': '每睡一小时能恢复的最大生命值比例。',
      'Mana_RecoveryRatePerHourOfSleep': '每睡一小时能恢复的最大法力值比例。',
      'Alcohol': '该角色醉到什么程度，较高的档位会拿敏捷和法力去换力量。',
      'MaxAlcohol': '该角色能达到的最高酒精值。',
      'AlcoholDepletionRate': '酒精值往清醒方向回落得有多快。',
      'Swampweed': '该角色嗨到什么程度，较高的档位会让其属性此消彼长。',
      'MaxSwampweed': '该角色能达到的最高沼泽草值。',
      'SwampweedDepletionRate': '沼泽草带来的迷幻劲头消退得有多快。',
      'XPExecutedBounty': '在这名角色已经被打倒在地时再将其杀死，所能拿到的经验值。',
      'XPKillOrDefeatBounty': '把这名角色打倒时所能拿到的经验值，不管对方是当场毙命还是只被打晕在地。',
      'Level': '角色等级。随经验提升，并带来学习点数。',
      'LockpickDurability': '由开锁技能决定：未学2、已学4、精通6。',
      'LockpickPrecision': '由开锁技能决定：未学0、已学1、精通2。',
      'PickPocketing': '由扒窃技能决定：未学-30、已学-10、精通+10。',
      'other': '?',
    });
    return '$_temp0';
  }

  @override
  String get knowledgeTypeVoiceLine => '语音台词';

  @override
  String get knowledgeTypeOther => '其他';

  @override
  String get armorUpgradeUpper => '上部';

  @override
  String get armorUpgradeMiddle => '中部';

  @override
  String get armorUpgradeLower => '下部';

  @override
  String get knowledgeCategoryTopic => '主题';

  @override
  String get knowledgeCategoryChoice => '选项';

  @override
  String get knowledgeCategoryInfo => '信息';

  @override
  String get statusOk => '正常';

  @override
  String get statusFailed => '失败';

  @override
  String get missingSaveReference => '文件缺失';

  @override
  String missingSaveReferenceDescription(String slot) {
    return '$slot.sav 缺失。它可能已被删除、移动或重命名；存档配置仍在引用它。';
  }

  @override
  String get removeFromProfile => '从存档配置中移除';

  @override
  String get deleteSavegame => '删除存档';

  @override
  String get deleteSavegameTitle => '删除存档？';

  @override
  String deleteSavegameBody(String save, String fileName, String profile) {
    return '要删除 $save（$fileName）吗？它将从 $profile 中移除，并从存档文件夹中删除。GORE 会先创建备份。';
  }

  @override
  String get removeSaveFromProfileTitle => '从存档配置中移除存档？';

  @override
  String removeSaveFromProfileBody(String save, String profile) {
    return '要从 $profile 中移除 $save 吗？如果存档文件仍然存在，文件本身将会保留。';
  }

  @override
  String get unassignedSave => '未分配给存档配置';

  @override
  String get armorUpgradeLight => '轻型';

  @override
  String get armorUpgradeMedium => '中型';

  @override
  String get armorUpgradeHeavy => '重型';

  @override
  String get knowledgeCaptionForcedConversation => '强制对话';

  @override
  String get knowledgeCaptionFollowupTopic => '后续话题';

  @override
  String get knowledgeCaptionFallbackTopic => '后备话题';

  @override
  String durationMinutes(int minutes) {
    return '$minutes 分钟';
  }

  @override
  String durationHours(int hours) {
    return '$hours 小时';
  }

  @override
  String durationHoursMinutes(int hours, int minutes) {
    return '$hours 小时 $minutes 分钟';
  }

  @override
  String get backupStatusInvalidProfileStructure => '存档配置数据无效';

  @override
  String get backupStatusSlotMetadataMissing => '所选存档的元数据缺失';

  @override
  String defaultProfileName(int id) {
    return '存档配置 $id';
  }

  @override
  String get statusUnknown => '未知';

  @override
  String editorUnexpectedError(String details) {
    return '意外错误：$details';
  }

  @override
  String get editorOperationInProgress => '另一项操作正在进行中。请稍后重试。';

  @override
  String get editorUnsavedBeforeDifficulty =>
      '存档中有未保存的修改。更改存档配置难度前，请先保存或重置这些修改。';

  @override
  String get editorNoSaveFolderSelected => '未选择存档文件夹。';

  @override
  String get editorNoSaveSelected => '未选择存档。';

  @override
  String get coreUnknownError => '核心组件发生未知错误';

  @override
  String get editorUnsavedBeforeSwitchProfile => '请先保存或重置未保存的修改；切换存档配置会离开当前存档。';

  @override
  String get editorUnsavedBeforeOpenFile => '打开其他文件前，请先保存或重置未保存的修改。';

  @override
  String get editorSelectSavFile => '请选择 .sav 存档文件。';

  @override
  String get editorNotGothicGsav => '所选文件不是 Gothic GSAV 存档。';

  @override
  String get editorUnsavedBeforeChangeSaveProfile =>
      '更改存档所属的存档配置前，请先保存或重置未保存的修改。';

  @override
  String get editorUnsavedBeforeRemoveProfile => '从存档配置中移除存档前，请先保存或重置未保存的修改。';

  @override
  String get editorUnsavedBeforeDeleteSave => '删除此存档前，请先保存或重置未保存的修改。';

  @override
  String get editorUnsavedBeforeRestoreProfile =>
      '存档中有未保存的修改。恢复存档配置备份前，请先保存或重置这些修改。';

  @override
  String editorConflictingPropertyEdits(String path) {
    return '两个标签页中未保存的修改针对同一属性 ($path)。请重置或撤销其中一项，然后再次保存。';
  }

  @override
  String editorGlossaryMemoryConflict(String path) {
    return '图鉴分段更改和“全部数据”中另一项未保存的修改都针对 Hero MemorizedEvents 数组 ($path)。图鉴更改会在该数组中添加或移除条目，因此两项修改无法同时保存。请重置或撤销其中一项，然后再次保存。';
  }

  @override
  String editorGlossaryQuestConflict(String path) {
    return '图鉴分段更改和另一项未保存的修改都针对同一任务的 CurrentState 属性 ($path)。图鉴更改本身会更新该状态。请重置或撤销其中一项，然后再次保存。';
  }

  @override
  String editorRelationshipConflict(String path) {
    return '关系覆盖设置和“全部数据”中另一项未保存的修改都针对同一 NPC 关系条目 ($path)。结构化的关系更改可能会替换该条目中的修正值，因此两项修改无法同时保存。请重置或撤销其中一项，然后再次保存。';
  }

  @override
  String editorMultipleStructuralArrayEdits(String path) {
    return '同一数组 ($path) 有多项未保存的结构更改。添加另一项更改前，请先保存或重置第一项更改。';
  }

  @override
  String editorStructuralArrayConflict(String path) {
    return '事件结构更改和“全部数据”中另一项未保存的修改都针对 $path。继续前，请保存或重置其中一项。';
  }

  @override
  String get editorSkillsEffectConflict =>
      '“技能”中的更改和“全部数据”中针对同一角色效果 (ActiveEffects › EffectSpec › Def) 的修改都在等待保存。两项修改无法同时保存。请重置或撤销其中一项，然后再次保存。';

  @override
  String get editorInventoryResetConflict =>
      '重置物品栏和对同一物品栏的另一项修改都在等待保存。重置会替换整个物品栏并丢弃另一项修改。请重置或撤销其中一项，然后再次保存。';

  @override
  String get editorUseFolder => '使用此文件夹';

  @override
  String get editorGothicSavegameFileType => 'Gothic 存档';

  @override
  String get editorNoDifficultyChanges => '没有需要保存的难度更改';

  @override
  String get editorDifficultyWritten => '难度已写入存档配置（已创建备份）';

  @override
  String editorChangesSavedWithBackup(int count) {
    return '已保存 $count 项更改并创建备份';
  }

  @override
  String editorPlacementNoteFailed(String details) {
    return '移動已儲存，但無法寫入用於還原的記錄：$details';
  }

  @override
  String editorProfileNotFound(int profileId) {
    return '未找到存档配置 $profileId。';
  }

  @override
  String get editorNoFreeSaveSlot => '游戏存档文件夹中没有可用的存档槽位（G1R-001 至 G1R-999）。';

  @override
  String editorSaveImportedAssigned(int profileId) {
    return '存档已导入并分配给存档配置 $profileId';
  }

  @override
  String editorSaveAssigned(int profileId) {
    return '存档已分配给存档配置 $profileId（已创建配套备份）';
  }

  @override
  String editorSaveSlotNotAssigned(String slot, int profileId) {
    return '存档槽位 $slot 未分配给存档配置 $profileId。';
  }

  @override
  String get editorSaveRemovedFromProfile => '已从存档配置中移除存档';

  @override
  String get editorSaveDeleted => '存档已删除；已创建备份';

  @override
  String editorRestoredBackup(String path) {
    return '已恢复备份：$path';
  }

  @override
  String editorRestoredBackupWithoutCompanion(String path) {
    return '已恢复备份：$path（由于没有匹配的配套备份，PersistentDataList.sav 保持不变；槽位元数据可能不同）';
  }

  @override
  String editorCodecRoundtripPassed(int chunkIndex, int bytes) {
    return '编解码器往返验证通过：区块 $chunkIndex 已重新压缩为 $bytes 字节';
  }

  @override
  String editorDifficultyWriteFailed(String details) {
    return '无法写入存档配置难度：$details';
  }

  @override
  String editorProfileAssignmentFailed(String details) {
    return '无法将存档分配给存档配置：$details';
  }

  @override
  String editorProfileRemovalFailed(String details) {
    return '无法从存档配置中移除存档：$details';
  }

  @override
  String editorDeleteSaveFailed(String details) {
    return '无法删除存档：$details';
  }

  @override
  String editorSaveFailed(String details) {
    return '无法保存修改：$details';
  }

  @override
  String editorScanSavesFailed(String details) {
    return '扫描存档失败：$details';
  }

  @override
  String editorInspectSaveFailed(String details) {
    return '检查存档失败：$details';
  }

  @override
  String editorLoadBackupsFailed(String details) {
    return '加载备份失败：$details';
  }

  @override
  String editorRestoreFailed(String details) {
    return '无法恢复备份：$details';
  }

  @override
  String editorRestoreReloadFailed(String path, String details) {
    return '已恢复备份：$path，但重新加载存档失败：$details';
  }

  @override
  String editorCodecCheckFailed(String details) {
    return '编解码器检查失败：$details';
  }

  @override
  String editorCodecValidationFailed(String details) {
    return '编解码器往返验证失败：$details';
  }

  @override
  String editorPropertySearchFailed(String details) {
    return '属性搜索失败：$details';
  }

  @override
  String get editorSelectionChangedWhileLoadingHeroAttributes =>
      '加载英雄属性时，所选存档发生了变化。';

  @override
  String editorSkillsLoadFailed(String details) {
    return '加载技能失败：$details';
  }

  @override
  String editorProgressionQueryFailed(String details) {
    return '查询进度失败：$details';
  }

  @override
  String editorNpcListFailed(String details) {
    return '加载 NPC 列表失败：$details';
  }

  @override
  String editorCharacterListFailed(String details) {
    return '加载角色列表失败：$details';
  }

  @override
  String editorNpcAttributesFailed(String details) {
    return '加载 NPC 属性失败：$details';
  }

  @override
  String editorNpcPositionFailed(String details) {
    return '加载 NPC 位置失败：$details';
  }

  @override
  String editorNpcInventoryFailed(String details) {
    return '加载 NPC 物品栏失败：$details';
  }

  @override
  String editorFactionListFailed(String details) {
    return '加载阵营列表失败：$details';
  }

  @override
  String get editorNoBackupPath => '无';

  @override
  String editorBackupMessage(String prefix, String backupPath) {
    return '$prefix：$backupPath';
  }

  @override
  String editorBackupMessageWithPersistent(
    String prefix,
    String backupPath,
    String persistentPath,
  ) {
    return '$prefix：$backupPath；PersistentDataList 备份：$persistentPath';
  }

  @override
  String localizationStatusFailed(String details) {
    return '获取本地化状态失败：$details';
  }

  @override
  String localizationExtractionFailed(String details) {
    return '提取失败：$details';
  }

  @override
  String glossaryLoadFailed(String details) {
    return '加载图鉴失败：$details';
  }

  @override
  String backupStatusError(String details) {
    return '备份错误：$details';
  }

  @override
  String memoryEventCategory(String category, String fallback) {
    String _temp0 = intl.Intl.selectLogic(category, {
      'quest': '任务',
      'document': '文档',
      'story': '剧情',
      'exploration': '探索',
      'combat': '战斗',
      'social': '社交',
      'item': '物品',
      'learning': '学习',
      'guild': '公会',
      'crime': '犯罪',
      'rest': '休息',
      'other': '$fallback',
    });
    return '$_temp0';
  }

  @override
  String memoryEventAction(String kind, String fallback) {
    String _temp0 = intl.Intl.selectLogic(kind, {
      'questStarted': '任务已开始',
      'questSucceeded': '任务已完成',
      'questFailed': '任务失败',
      'documentRead': '已阅读文档',
      'documentSegmentUnlocked': '已发现条目',
      'documentSegmentViewed': '已查看条目',
      'chapterCompleted': '章节已完成',
      'areaEntered': '进入区域',
      'areaLeft': '离开区域',
      'characterKilled': '角色已被杀死',
      'characterDefeated': '角色已被击败',
      'combatDodge': '已闪避攻击',
      'characterDebuffed': '已施加负面效果',
      'tradeAvailable': '已解锁交易',
      'itemObtained': '获得物品',
      'itemCrafted': '制作物品',
      'skillStateRecorded': '已记录技能状态',
      'recipeLearned': '学会配方',
      'guildJoined': '加入公会',
      'crimeRecorded': '犯罪已记录',
      'slept': '睡眠',
      'storyEvent': '剧情事件',
      'other': '$fallback',
    });
    return '$_temp0';
  }

  @override
  String memoryEventTitleWithSubject(String action, String subject) {
    return '$action：$subject';
  }

  @override
  String memoryEventFact(String fact, String fallback) {
    String _temp0 = intl.Intl.selectLogic(fact, {
      'gameTime': '游戏时间',
      'duration': '持续时间',
      'chapter': '章节',
      'instigator': '触发者',
      'affected': '受影响对象',
      'amount': '数量',
      'primaryObject': '对象',
      'secondaryObject': '上下文',
      'segmentText': '条目文本',
      'other': '$fallback',
    });
    return '$_temp0';
  }

  @override
  String memoryEventGameTime(int day, String time) {
    return '第 $day 天，$time';
  }

  @override
  String memoryEventSecondsValue(String value) {
    return '$value 秒';
  }

  @override
  String memoryEventMoreValues(String values, int count) {
    return '$values +$count';
  }

  @override
  String get memoryEventHero => '主角';

  @override
  String get memoryEventDetails => '详细信息';

  @override
  String get memoryEventTags => '标签';

  @override
  String get memoryEventTechnicalData => '技术信息';

  @override
  String get memoryEventIndex => '索引';

  @override
  String get memoryEventPosition => '位置';

  @override
  String get memoryEventPayload => '事件数据';

  @override
  String get memoryEventSubject => '关联对象';

  @override
  String glossaryCatalogSegmentLabel(String segmentId, String fallback) {
    String _temp0 = intl.Intl.selectLogic(segmentId, {
      'Access': '通行',
      'AccessDenied': '禁止通行',
      'AccesToTemple': '进入神殿',
      'Advice': '建议',
      'AfterFight': '战斗之后',
      'AfterFireMages': '火法师事件之后',
      'AfterNek': '尼克之后',
      'AfterQuest': '任务之后',
      'Alone': '独自一人',
      'Amulet': '护符',
      'Annoying': '烦人',
      'Armor': '护甲',
      'Avoid': '回避',
      'Backstory': '背景故事',
      'BackStory': '背景故事',
      'BasicMagic': '基础魔法',
      'Beated': '被打败',
      'BecomeMercenary': '成为佣兵',
      'Beer': '啤酒',
      'Bestiary': '怪物图鉴',
      'Blessing': '祝福',
      'Boss': '首领',
      'Bully': '恶霸',
      'BullyAdvice': '应对恶霸的建议',
      'Camp': '营地',
      'CampDivided': '分裂的营地',
      'CareOfMessengers': '照顾信使',
      'ChangeOpinion': '改变看法',
      'ChargeUriziel': '为尤里泽尔充能',
      'Chosen': '天选者',
      'Contact': '接触',
      'Courier': '信使',
      'CraftBows': '制作弓箭',
      'Crazy': '疯癫',
      'DailyMeal': '每日餐食',
      'DailyRation_Trader': '每日口粮商人',
      'DAM': '水坝',
      'Dead': '死亡',
      'Deal': '交易',
      'Dealer': '交易商',
      'Deceived': '受骗',
      'Dementia': '痴呆',
      'DenyAccess': '拒绝通行',
      'DifferentOpinion': '不同意见',
      'Discussion': '讨论',
      'DontTalk': '不许交谈',
      'Duel': '决斗',
      'Entrance': '入口',
      'Escape': '逃脱',
      'Extended': '扩展',
      'Extra': '额外',
      'ExtraInfo': '额外信息',
      'Fanatic': '狂信徒',
      'Fight': '战斗',
      'FindUlumulu': '寻找乌鲁穆鲁',
      'FireMages': '火法师',
      'FireMagesEscape': '火法师的逃脱',
      'FiskNewDealer': '菲斯克的新销赃贩子',
      'FiskNewDealerCompleted': '菲斯克的新销赃贩子——已完成',
      'FogTower': '雾塔',
      'Food': '食物',
      'Forgave': '已原谅',
      'Forgive': '原谅',
      'Forgiven': '已获原谅',
      'FourFriends': '四位好友',
      'FreeHut': '空闲小屋',
      'FreeMine': '自由矿场',
      'Fury': '狂怒',
      'GoodTeacher': '好导师',
      'Gossip': '传闻',
      'GotScavenger': '获得食尸鸟',
      'GrantedAccess': '已获准通行',
      'GRDArmor': '卫兵防具',
      'Guide': '向导',
      'HateMages': '仇恨法师',
      'HateMagesExplanation': '仇恨法师的原因',
      'HateRiceLord': '憎恨稻田主',
      'Heal': '治疗',
      'Healing': '治疗',
      'Help': '帮助',
      'Helper': '帮手',
      'HelpKagan': '帮助卡根',
      'HutStory': '小屋的故事',
      'Ignore': '无视',
      'Impress': '打动',
      'ImpressAlchemy': '用炼金术打动',
      'ImpressInscription': '用铭文打动',
      'Info': '信息',
      'Interested': '感兴趣',
      'Introduction': '初识',
      'Introduction_2': '初识 2',
      'Introduction_Armor': '护甲介绍',
      'Introduction_Teacher': '初识（导师）',
      'Introduction_Trader': '初识（商人）',
      'Invocation': '召唤仪式',
      'JoinSC': '加入沼泽营地',
      'Joint': '沼泽草烟卷',
      'KalomCamp': '科尔·卡隆的营地',
      'Leader': '领袖',
      'Learning': '学习',
      'LearnOrcish': '学习兽人语',
      'LeftParty': '离队',
      'Library': '图书馆',
      'Lie': '谎言',
      'Lock': '锁',
      'Lockpick': '开锁工具',
      'Mad': '疯狂',
      'Mandibles': '矿爬虫的下颚',
      'MapMaker': '制图师',
      'Monastery': '修道院',
      'MordragKO': '击倒莫德拉格',
      'Nek': '尼克',
      'NewCamp': '新营地',
      'NewCamper': '新营地成员',
      'NewLeader': '新领袖',
      'NightPatrol': '夜间巡逻',
      'NotInterested': '不感兴趣',
      'OldCamp': '旧营地',
      'OrcEnclaveEntrance': '兽族聚居地入口',
      'OrcGraveyard': '兽人墓地',
      'OreArmor': '矿石铠甲',
      'Party': '队伍',
      'Pay': '付款',
      'PayMoney': '付钱',
      'Permission': '许可',
      'Pet': '宠物',
      'PreparingInvocation': '准备召唤仪式',
      'Quest': '任务',
      'RankUpFireMages': '晋升为火法师',
      'RankUpGuard': '晋升为卫兵',
      'RanUpFireMagesCompleted': '晋升火法师完成',
      'Realocated': '已迁移',
      'Reason': '原因',
      'Respect': '尊重',
      'ReturnToSC': '返回沼泽营地',
      'RicelordForeman': '稻田主的监工',
      'RideScavenger': '骑乘食尸鸟',
      'Robe': '法袍',
      'Safe': '安全',
      'Scraper': '采矿工',
      'SecondChance': '第二次机会',
      'SecretLocation': '秘密地点',
      'SecretPassage': '秘密通道',
      'SecretPath': '秘密小径',
      'SleeperFollower': '沉睡者信徒',
      'SleeperTemple': '沉睡者神庙',
      'SmallInfo': '小道消息',
      'Stonehenge': '巨石阵',
      'StopFollowing': '停止跟随',
      'SwampCamp': '沼泽营地',
      'Talkative': '健谈',
      'Teach': '教学',
      'TeachBow': '弓术训练',
      'Teacher': '导师',
      'Teacher2': '导师 2',
      'TeacherInscription': '铭文导师',
      'TeacherMana': '魔力导师',
      'TeachIchor': '矿爬虫体液采集训练',
      'TeachMagic': '魔法训练',
      'TeachOrcish': '教授兽人语',
      'TeachStats': '属性训练',
      'TeachWeapon': '武器训练',
      'Teleport': '传送',
      'TheMysteriousOrc': '神秘的兽人',
      'ThroneRoom': '王座厅',
      'TradeBow': '弓箭交易',
      'Trader': '商人',
      'TradeSkins_Trader': '毛皮商人',
      'Traitor': '叛徒',
      'Trial': '试炼',
      'TrollCanyon': '巨魔谷',
      'Trust': '信任',
      'Ulumulu': '乌鲁穆鲁',
      'Unexperienced': '缺乏经验',
      'Uriziel': '尤里泽尔',
      'UrizielRune': '尤里泽尔符文',
      'Useful': '有用',
      'Velaya': '维拉雅',
      'Vibrations': '震动',
      'WaitFreeMine': '在自由矿场等待',
      'WaitInTrainingArea': '在训练场等待',
      'Warning': '警告',
      'WarningTooLate': '迟来的警告',
      'WaterMessenger': '水法师的信使',
      'Weapon': '武器',
      'Who': '身份',
      'Women': '女人们',
      'other': '$fallback',
    });
    return '$_temp0';
  }

  @override
  String get slotRepairTitle => '物品栏槽位损坏';

  @override
  String slotRepairBody(int count) {
    return '此存档中有 $count 个物品栏槽位的 ID 与其位置不再匹配 — 在游戏中丢弃这类物品时，会误删另一件物品。修复仅重写槽位 ID，不会添加、删除或更改任何物品。保存时会照常创建备份。';
  }

  @override
  String get slotRepairQueued => '修复已加入待保存列表 — 保存后生效。';

  @override
  String get slotRepairAction => '修复';

  @override
  String get slotRepairDiscard => '放弃';

  @override
  String get editorInventorySlotEditConflict =>
      '对物品栏槽位的直接编辑与占用整个槽位的操作（修复、添加或删除）同时在待保存列表中。后者会覆盖前者 — 请撤销其中一项后再保存。';

  @override
  String get editorTraderArrayConflict =>
      '一项交易修改与对商人数组的直接编辑一同排队。该编辑会重新编号交易修改所依据的行，因此两者之一会落到错误的商人身上——撤销其中一项后再保存。';

  @override
  String get backupFactFile => '文件';

  @override
  String get renameBackupTooltip => '为此备份命名';

  @override
  String get renameBackupTitle => '备份名称';

  @override
  String get renameBackupLabel => '名称';

  @override
  String renameBackupHelp(String fileName) {
    return '显示在文件名 $fileName 之外。留空则移除名称；文件本身不会被重命名。';
  }

  @override
  String get deleteBackupTooltip => '删除此备份';

  @override
  String get deleteBackupTitle => '删除备份';

  @override
  String deleteBackupBody(String name, String fileName) {
    return '删除“$name”（$fileName）？文件将从磁盘移除且无法恢复。';
  }

  @override
  String get deleteBackupConfirm => '删除';

  @override
  String editorDeletedBackup(String path) {
    return '已删除备份：$path';
  }

  @override
  String editorDeleteBackupFailed(String details) {
    return '无法删除备份：$details';
  }

  @override
  String editorRenameBackupFailed(String details) {
    return '无法为备份命名：$details';
  }

  @override
  String get slotRepairUnavailable => '目前无法修复 — 无法写入此存档。';

  @override
  String editorDeletedBackupWithLabelWarning(String path, String details) {
    return '已删除备份：$path — 但无法移除其名称：$details';
  }

  @override
  String get slotRepairNotOffered => '此存档不支持修复。';

  @override
  String get statisticsTitle => '统计';

  @override
  String get statisticsSubtitle => '角色、任务、世界和游戏进度的简要概览。';

  @override
  String statisticsCardTitle(String card, String fallback) {
    String _temp0 = intl.Intl.selectLogic(card, {
      'timing': '时间',
      'character': '角色',
      'quests': '任务',
      'progress': '进度',
      'encounters': '战斗与交往',
      'inventory': '技能与物品',
      'other': '$fallback',
    });
    return '$_temp0';
  }

  @override
  String statisticsMetric(String metric, String fallback) {
    String _temp0 = intl.Intl.selectLogic(metric, {
      'timePlayed': '游玩时间',
      'worldTime': '世界时间',
      'level': '等级',
      'experience': '经验',
      'learningPoints': '学习点数',
      'guild': '阵营',
      'health': '生命值',
      'mana': '法力',
      'chapter': '章节',
      'location': '位置',
      'kills': '击杀NPC',
      'knownCharacters': '已知角色',
      'killedMonsters': '击杀怪物',
      'defeatedNpcs': '击败NPC',
      'killedNpcs': '击杀NPC',
      'knownNpcs': '已知NPC',
      'knownTeachers': '已知导师',
      'learnedSkills': '已学技能',
      'knowledge': '知识条目',
      'deadCharacters': '死亡角色',
      'traders': '已知商人',
      'inventoryStacks': '物品堆叠',
      'inventoryItems': '物品',
      'ore': '矿石',
      'equipped': '已装备',
      'hostileFactions': '敌对阵营',
      'openCrimes': '未解决罪行',
      'position': '坐标',
      'other': '$fallback',
    });
    return '$_temp0';
  }

  @override
  String statisticsGuildRank(String rank, String fallback) {
    String _temp0 = intl.Intl.selectLogic(rank, {
      'oldCampShadow': '旧营地 · 影子',
      'oldCampGuard': '旧营地 · 守卫',
      'oldCampFireMage': '旧营地 · 火法师',
      'newCampRogue': '新营地 · 强盗',
      'newCampMercenary': '新营地 · 雇佣兵',
      'newCampWaterMage': '新营地 · 水法师',
      'swampCampNovice': '沼泽营地 · 新人',
      'swampCampTemplar': '沼泽营地 · 圣殿骑士',
      'other': '$fallback',
    });
    return '$_temp0';
  }

  @override
  String get statisticsUnknown => '不可用';

  @override
  String get statisticsMore => '更多统计';

  @override
  String statisticsSummary(
    String level,
    String guild,
    String chapter,
    int completed,
    int failed,
    String playTime,
  ) {
    return '等级$level，$guild，第$chapter章。完成任务$completed个，失败$failed个。游玩时间：$playTime。';
  }

  @override
  String get locksSidebar => '锁';

  @override
  String editorLockListFailed(String details) {
    return '无法加载锁列表：$details';
  }

  @override
  String get locksSearchHint => '搜索锁或钥匙';

  @override
  String get locksAllRegions => '所有区域';

  @override
  String locksShownOfTotal(int shown, int total) {
    return '$total 个中的 $shown 个';
  }

  @override
  String get locksFilterChests => '箱子';

  @override
  String get locksFilterDoors => '门';

  @override
  String get locksFilterUnlocked => '已开启';

  @override
  String get locksFilterLocked => '已上锁';

  @override
  String locksDifficultyLevel(int bars, int level) {
    return '难度 $bars/4（内部等级 $level/7）';
  }

  @override
  String get locksKeyOnly => '仅限钥匙';

  @override
  String locksKeyLabel(String keys) {
    return '钥匙：$keys';
  }

  @override
  String get locksPermalocked => '永久封闭';

  @override
  String get locksReadOnly => '此存档没有可编辑的锁列表。';

  @override
  String get locksUnknownEntry => '此游戏版本中不存在';

  @override
  String get locksDoorLeafHint => '重新锁上门时，门本身也会关闭。';

  @override
  String get locksResetPending => '放弃待保存的锁改动';
}

/// The translations for Chinese, using the Han script (`zh_Hans`).
class AppLocalizationsZhHans extends AppLocalizationsZh {
  AppLocalizationsZhHans() : super('zh_Hans');

  @override
  String get debugSectionTitle => '高级（调试）';

  @override
  String get debugSectionSubtitle => '用于错误报告的诊断和原始数据';

  @override
  String get showObjectIdsTitle => '显示其他技术 ID';

  @override
  String get showObjectIdsSubtitle => '显示物品、对话知识、任务和孤立角色的技术 ID。NPC ID 始终显示。';

  @override
  String get storyStateSidebar => '剧情状态';

  @override
  String get storyStateDescription =>
      '游戏在这里记录任务、对话和事件的进度。“已保存”显示存档中的数值，“未设置”显示其他已知条目。根据条目的不同，数字可能表示是或否、次数或进度阶段。时间标记表示游戏内的日期和时间。';

  @override
  String get storyStateReadOnly =>
      '在确认脚本含义和安全的映射写入方式前保持只读。关联的词条文本仅提供上下文，并非技术 ID 的直接翻译。';

  @override
  String get storyStateStructureReadOnly =>
      '无法唯一且安全地确定此存档中的 StoryPropertyValues 结构。此存档的剧情值将保持只读。';

  @override
  String get storyStateSearch => '搜索剧情状态';

  @override
  String storyStateValuesCount(int shown, int total) {
    return '已显示 $shown 个，共 $total 个剧情值';
  }

  @override
  String get storyStateInteger => '整数';

  @override
  String get storyStateTimeMarker => '时间标记';

  @override
  String get storyStateChapter => '章节';

  @override
  String get storyStateUnknown => '未知源码类型';

  @override
  String storyStateShowDormant(int count) {
    return '显示未使用（$count）';
  }

  @override
  String get storyStateUnknownDetail =>
      '当前脚本目录中没有此已保存 ID（例如来自模组或更新的游戏版本）。其存档线值为 int32，但不会推断其含义。';

  @override
  String get storyStateStored => '已保存';

  @override
  String get storyStateUnset => '未设置';

  @override
  String get storyStateUnsetDetail => '此目录字段未序列化到该存档中，因此游戏会使用未设置或默认状态。';

  @override
  String get storyStateRawValue => '原始值';

  @override
  String storyStateElapsed(String duration) {
    return '保存时已过去：$duration';
  }

  @override
  String storyStateAhead(String duration) {
    return '保存时尚在未来：$duration';
  }

  @override
  String storyStateDurationDays(int days, String time) {
    return '$days 天 $time';
  }

  @override
  String get storyStateRelatedGlossary => '关联词条';

  @override
  String get storyStateTechnicalPath => '技术路径';

  @override
  String get storyStateEditingGuidance =>
      '选择条目即可修改数值，修改会在保存后生效。请只修改你了解其作用的数值，否则任务或对话可能无法按预期进行。保存时会自动创建备份。';

  @override
  String get storyStatePending => '待处理';

  @override
  String storyStatePendingValue(String value) {
    return '将保存为 $value';
  }

  @override
  String get storyStatePendingRemoval => '将从存档中移除';

  @override
  String get storyStateEditValue => '编辑值';

  @override
  String get storyStateSetValue => '设置值';

  @override
  String get storyStateRemoveValue => '从存档中移除';

  @override
  String get storyStateUndoChange => '撤销剧情更改';

  @override
  String get storyStateResetChanges => '重置剧情更改';

  @override
  String storyStateDialogTitle(String id) {
    return '编辑 $id';
  }

  @override
  String get storyStateRawInput => '有符号 int32 值';

  @override
  String get storyStateInvalidInt32 => '请输入 -2147483648 到 2147483647 之间的整数。';

  @override
  String get storyStateQueueChange => '将更改加入队列';

  @override
  String storyStateSuggestedValues(String values) {
    return '随游戏提供的脚本中已确认的值：$values';
  }

  @override
  String get storyStateSuggestionsNotLimits =>
      '建议值不是验证限制；原生代码、模组或后续游戏版本可能会使用其他值。';

  @override
  String get storyStateUseCurrentTime => '使用当前存档时间';

  @override
  String get storyStateStructuredTime => '天数 / 时间';

  @override
  String get storyStateRawMode => '原始 int32';

  @override
  String get storyStateChapterWarning => '仅更改章节不会同步任务、NPC、物品栏或世界状态。';

  @override
  String get storyStateDormantWarning =>
      '在随游戏提供的脚本缓存中未找到对此字段的有效读取或写入。它可能是旧字段、由原生代码控制，或为保留字段。';

  @override
  String get storyStateReadOnlySourceWarning =>
      '随游戏提供的脚本会读取此字段，但没有通过脚本写入。它仍可能由原生代码管理。';

  @override
  String get storyStateUnknownEditWarning =>
      '这个来自模组或后续版本的 ID 没有内置的源码语义。请仅编辑其原始 int32 值。';

  @override
  String storyStateIntegerKind(String kind) {
    String _temp0 = intl.Intl.selectLogic(kind, {
      'binaryFlag': '二进制标志',
      'finiteState': '多状态值',
      'counterOrScore': '计数器 / 分数',
      'calendarDay': '日历日',
      'derivedOrOpaqueInteger': '派生 / 不透明整数',
      'readOnlyInSourceInteger': '随附脚本中只读',
      'dormantOrLegacyInteger': '随附脚本中未使用',
      'other': '整数',
    });
    return '$_temp0';
  }

  @override
  String get storyStateZeroVsUnset =>
      '已保存的 0 与映射中没有该条目是两种不同的文件状态。“从存档中移除”会恢复构造函数或默认状态。';

  @override
  String get appTitle => 'GORE Save Editor';

  @override
  String get appLogoSemanticLabel => 'GORE Save Editor 徽标';

  @override
  String get zoomTooltip => '按 Ctrl +/- 放大/缩小';

  @override
  String get switchToLightMode => '切换到浅色模式';

  @override
  String get switchToDarkMode => '切换到深色模式';

  @override
  String get about => '关于';

  @override
  String get tabOverview => '概览';

  @override
  String get tabPlayer => '玩家';

  @override
  String get tabAttribute => '属性';

  @override
  String get heroGroupSkills => '技能';

  @override
  String get skillsNoneBody => '未找到该角色的技能。';

  @override
  String get skillsUnavailableBody => '此存档无法编辑技能——主角没有可修改的效果数据。';

  @override
  String get skillNotLearned => '未习得';

  @override
  String get skillLearn => '学习';

  @override
  String get skillActionLearn => '学习';

  @override
  String get skillActionUnlearn => '遗忘';

  @override
  String get skillTierUntrained => '未受过训练';

  @override
  String get skillTierBeginner => '初学者';

  @override
  String get skillTierTrained => '训练有素';

  @override
  String get skillTierMaster => '大师级';

  @override
  String get skillTierNovice => '熟练';

  @override
  String get skillTierAmateur => '业余（第0环）';

  @override
  String get skillTierLearned => '已习得';

  @override
  String skillTierCircle(int n) {
    return '第$n环';
  }

  @override
  String get skillHintBlacksmith1H => '单手武器';

  @override
  String get skillHintBlacksmith2H => '双手武器';

  @override
  String get skillScutesTrained => '熟练（骨板）';

  @override
  String get skillScutesMaster => '大师（＋剃刀兽角板）';

  @override
  String get skillCategoryCombat => '战斗';

  @override
  String get skillCategoryCrafting => '制作';

  @override
  String get skillCategoryHunting => '狩猎';

  @override
  String get skillCategoryLanguage => '语言';

  @override
  String get skillCategoryMagic => '魔法';

  @override
  String get skillCategoryMovement => '移动';

  @override
  String get skillCategoryThievery => '盗窃';

  @override
  String get skillCategoryOther => '其他';

  @override
  String get skillNameOneHanded => '单手';

  @override
  String get skillNameTwoHanded => '双手';

  @override
  String get skillNameFists => '赤手空拳';

  @override
  String get skillNameBow => '弓';

  @override
  String get skillNameCrossbow => '弩';

  @override
  String get skillNameLockpicking => '开锁';

  @override
  String get skillNamePickpocketing => '扒窃';

  @override
  String get skillNameTakeOrgans => '摘取内脏';

  @override
  String get skillNameBreakTeeth => '摘取牙齿';

  @override
  String get skillNameTakeClaws => '摘取爪子';

  @override
  String get skillNameSkinFur => '拿取毛皮';

  @override
  String get skillNameSkin => '拿取皮肤';

  @override
  String get skillNameTakeFins => '拿取鳍';

  @override
  String get skillNameTakeStingers => '摘取刺';

  @override
  String get skillNameTakeSecretion => '摘取分泌物';

  @override
  String get skillNameTakeSkullPlates => '拿取头骨甲';

  @override
  String get skillNameSkinSwampshark => '拿取鲨鱼皮';

  @override
  String get skillNameTakeMinecrawlerPlates => '拿取护甲板';

  @override
  String get skillNameTakeScutes => '拿取鳞甲';

  @override
  String get skillNameTakeUluMulu => '拿取乌鲁木鲁';

  @override
  String get skillNameOrcWeapons => '兽人武器';

  @override
  String get skillNameMining => '采矿';

  @override
  String get skillNameDiving => '潜水';

  @override
  String get skillNameTakeMinecrawlerMandibles => '摘取下颌';

  @override
  String get skillNameTakeShadowbeastHorn => '拿取角 (Shadowbeast)';

  @override
  String get skillNameTakeSpines => '摘取脊柱';

  @override
  String get skillNameBreakSwampsharkTeeth => '摘取鲨鱼牙';

  @override
  String get skillNameTakeFireTongue => '拿取火蜥蜴的舌';

  @override
  String get skillNameTakeTrollHorn => '拿取角 (Troll)';

  @override
  String get skillNameAcrobatics => '杂技';

  @override
  String get skillNameWallClimbing => '攀登';

  @override
  String get skillNameRiding => '骑乘食尸鸟';

  @override
  String get skillNameSneaking => '潜行';

  @override
  String get skillNameAlchemy => '炼金术';

  @override
  String get skillNameRuneInscription => '铭刻';

  @override
  String get skillNameBlacksmithing => '锻造';

  @override
  String get skillNameMagicCircle => '魔法环';

  @override
  String get skillNameOrcish => '兽人语';

  @override
  String get tabInventory => '物品栏';

  @override
  String get tabTrade => '交易';

  @override
  String get traderNotAMerchant => '该角色不进行交易。';

  @override
  String get traderRetry => '重试';

  @override
  String get traderAmbiguousName => '有多条商人记录使用这个名字，无法判断哪家店属于该角色。已禁用编辑，以免改错。';

  @override
  String get traderOre => '矿石（购买力）';

  @override
  String get traderNoOre => '无矿石';

  @override
  String get traderStockCurrent => '已保存库存';

  @override
  String get traderStockCurrentTooltip => '目前为该商人保存的库存。游戏下次更新商人时，添加的物品可能会消失。';

  @override
  String get traderStockBase => '参考库存';

  @override
  String get traderStockBaseTooltip =>
      '这是已保存的库存副本，游戏可按该商人的规则更改或重新生成。这里只读显示，添加的物品不会永久保留。';

  @override
  String get traderStockBaseHint => '只读。此库存会随剧情推进而增加，也可能按商人规则被替换。它不是游戏最初的库存。';

  @override
  String get traderCurrentStockWarning => '商人库存的更改只会保留到下次补货。';

  @override
  String get traderRestockTitle => '预计补货时间';

  @override
  String get traderRestockTitleTooltip => '根据商人的上次活动、游戏时间和资源难度估算。';

  @override
  String get traderRestockPending => '待处理';

  @override
  String get traderRestockRevertTooltip => '撤销尚未保存的上次活动更改';

  @override
  String get traderRestockNever => '从未';

  @override
  String get traderRestockUnavailable => '不可用';

  @override
  String get traderRestockIntervalUnknown => '游戏天数未知';

  @override
  String get traderRestockNeverStatus => '尚未记录该商人的活动。';

  @override
  String get traderRestockClockAhead => '商人的上次活动晚于当前游戏时间。';

  @override
  String traderRestockNotDueYet(String time) {
    return '预计不会早于 $time。';
  }

  @override
  String get traderRestockPossiblyDue => '估算：库存可能已经可以更新。';

  @override
  String get traderRestockEligible => '估算：现在应该补货。';

  @override
  String get traderRestockNoWorldTime => '当前游戏时间不可用，因此无法估算补货时间。';

  @override
  String get traderRestockLastActivity => '上次商人活动';

  @override
  String get traderRestockLastActivityTooltip =>
      '此保存时间可能会在交易后或游戏更新库存时改变。它不一定就是上次补货的时间。';

  @override
  String get traderRestockForecastWindow => '预计时间';

  @override
  String get traderRestockForecastWindowTooltip =>
      '显示最早和最晚可能补货的时间。存档中没有游戏的确切规则，因此这只是估算。';

  @override
  String get traderRestockIntervalLabel => '两次补货之间的天数';

  @override
  String traderRestockInterval(int days, String level) {
    return '$days 天 · $level';
  }

  @override
  String get traderRestockIntervalTooltip =>
      '按资源难度计算：新手 2 天、Gothic 3 天、困难 5 个游戏日。';

  @override
  String get traderRestockAutomationLabel => '自动补货';

  @override
  String get traderRestockAutomationValue => '无法在存档中禁用';

  @override
  String get traderRestockAutomationTooltip => '无法在存档中关闭自动补货。只有模组能改变这项游戏规则。';

  @override
  String get traderRestockSetNow => '设为游戏时间';

  @override
  String get traderRestockSetNowTooltip =>
      '将当前游戏时间（包括尚未保存的更改）设为商人的上次活动。这会推迟预计补货时间。';

  @override
  String get traderRestockMakeDue => '准备补货';

  @override
  String get traderRestockMakeDueTooltip => '将上次活动向过去调整，使其达到预计补货时间。';

  @override
  String get traderRestockCustom => '自定义时间…';

  @override
  String get traderRestockCustomTooltip => '为商人的上次活动选择游戏内日期和时间。';

  @override
  String get traderRestockEditTitle => '商人的上次活动';

  @override
  String get traderOreHint =>
      '游戏内的数值会不同：载入时游戏会加上自他上次交易以来累积的部分——他会卖掉多余货物并以此补货。这个数字是起点，而非交易界面显示的金额。';

  @override
  String get traderOreHintShort => '初始值——可能与交易界面中的金额不同。';

  @override
  String get traderRestockStatusLabel => '状态';

  @override
  String get traderRestockStatusNever => '无活动';

  @override
  String get traderRestockStatusWaiting => '等待补货';

  @override
  String get traderRestockStatusReady => '可以补货';

  @override
  String get traderRestockStatusPossiblyReady => '可能可以补货';

  @override
  String get traderRestockStatusCheckTime => '检查保存时间';

  @override
  String get traderRestockStatusUnknown => '未知';

  @override
  String get traderPriceWarning => '价格会随商人的库存量和持有矿石而变化，因此修改这些数字也可能改变他的开价。';

  @override
  String get traderAddItem => '添加物品';

  @override
  String get traderRemoveItem => '移除条目';

  @override
  String get traderReadOnlyCore => '此核心版本只能读取商人数据。';

  @override
  String get traderDifficultyStockUnsupported =>
      '该商人拥有按难度区分的库存，编辑器并未建模。此处已禁用编辑，因为修改看似成功，却会让这部分额外库存原封不动。';

  @override
  String get traderRecordIncomplete =>
      '该商人的库存清单缺失，或其结构编辑器不支持、无法写入。此处已禁用编辑，以免修改在保存时失败。';

  @override
  String get traderEmptyStock => '没有库存。';

  @override
  String get traderUnknownItem => '不在物品目录中';

  @override
  String editorTradersLoadFailed(String details) {
    return '商人数据加载失败：$details';
  }

  @override
  String traderStockLineCount(int count) {
    return '$count 件商品';
  }

  @override
  String get tabWorld => '世界';

  @override
  String get tabCharacters => '角色';

  @override
  String get characterNoActorBody => '该角色在世界中没有对应的实体，因此没有属性、物品栏或事件。';

  @override
  String get characterNoEventsBody => '该角色没有事件。';

  @override
  String get characterOrphanGroup => '其他';

  @override
  String get tabAllData => '全部数据';

  @override
  String get tabBackups => '备份';

  @override
  String get tabSettings => '设置';

  @override
  String get reset => '重置';

  @override
  String get save => '保存';

  @override
  String saveWithCount(int count) {
    return '保存（$count）';
  }

  @override
  String get ok => '确定';

  @override
  String get cancel => '取消';

  @override
  String get confirm => '确认';

  @override
  String get close => '关闭';

  @override
  String get add => '添加';

  @override
  String get equippedBadge => '已装备';

  @override
  String get armorUpgradesLabel => '升级';

  @override
  String get browse => '浏览';

  @override
  String get noSavFilesFound => '未找到 .sav 文件';

  @override
  String get profile => '存档配置';

  @override
  String get otherSaves => '其他存档';

  @override
  String profileWithSaves(String name, int count) {
    return '$name（$count 个存档）';
  }

  @override
  String get switchProfile => '切换存档配置';

  @override
  String get openSaveFile => '打开文件';

  @override
  String get externalSave => '从外部打开的存档';

  @override
  String get saveProfileTitle => '存档配置';

  @override
  String get saveProfileDescription => '将此存档分配给另一个游戏存档配置。存档和存档配置索引将一同备份。';

  @override
  String get saveProfileExternalHint => '选择一个存档配置，将此文件导入游戏存档文件夹并在其中登记。原文件不会更改。';

  @override
  String get saveProfileNoProfiles =>
      '在 PersistentDataList.sav 中未找到可编辑的游戏存档配置。';

  @override
  String get saveProfileSelect => '选择存档配置';

  @override
  String get rescanSaveFolder => '重新扫描存档文件夹';

  @override
  String get discardUnsavedChangesTitle => '放弃未保存的更改？';

  @override
  String rescanDiscardBody(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '更改',
      one: '更改',
    );
    return '重新扫描将重新加载每个存档，并放弃你 $count 项未保存的$_temp0。';
  }

  @override
  String get discardAndRescan => '放弃并重新扫描';

  @override
  String chapterLabel(Object id) {
    return '第 $id 章';
  }

  @override
  String get quickSave => '快速存档';

  @override
  String get autoSave => '自动存档';

  @override
  String get manualSave => '手动存档';

  @override
  String get errorTitle => '错误';

  @override
  String get selectASaveTitle => '选择存档';

  @override
  String get selectASaveBody => '存档详情将显示在此处。';

  @override
  String bytesValue(String count) {
    return '$count 字节';
  }

  @override
  String get inspectionJsonTitle => '检查 JSON';

  @override
  String get copy => '复制';

  @override
  String get savegameFallbackTitle => '存档';

  @override
  String screenshotForSlot(String slot) {
    return '$slot 的截图';
  }

  @override
  String get publicSaveName => '名称';

  @override
  String get gameTimeTitle => '游戏时间';

  @override
  String get gameTimeDay => '天';

  @override
  String get gameTimeHours => '小时';

  @override
  String get gameTimeMinutes => '分钟';

  @override
  String get gameTimeSeconds => '秒';

  @override
  String gameTimeTotal(int seconds) {
    return '= 共 $seconds 秒';
  }

  @override
  String get gameTimeInvalid => '请输入整数：天数 ≥ 0，小时 0–23，分钟和秒数 0–59。';

  @override
  String get required => '必填';

  @override
  String get playerLockedBody => '编辑私有玩家数据需要支持压缩的编解码器。';

  @override
  String get heroTransform => '位置';

  @override
  String get locationX => '位置 X';

  @override
  String get locationY => '位置 Y';

  @override
  String get locationZ => '位置 Z';

  @override
  String get rotationPitch => '旋转俯仰';

  @override
  String get rotationYaw => '旋转偏航';

  @override
  String get rotationRoll => '旋转翻滚';

  @override
  String get spawnPositionSection => '出生位置（参考）';

  @override
  String get resetToSpawnPosition => '重置为出生位置';

  @override
  String get positionOutOfRange => '数值必须介于 −10,000,000 与 10,000,000 之间';

  @override
  String get positionNotEditable => '无法读取该角色已保存的位置，因此无法编辑。';

  @override
  String get positionNeverPlaced => '该角色从未在世界中放置过（位置 0, 0, 0）——游戏可能会忽略已保存的位置。';

  @override
  String get npcStayInPlace => '停用他的日常作息';

  @override
  String get npcStayInPlaceHint => '他会留在原地。';

  @override
  String get npcStayInPlaceLocked => '他原本的日常作息没有被记录下来，因此无法再还原。';

  @override
  String get npcUndoPlacement => '撤销这次移动';

  @override
  String get npcUndoPlacementStale => '存档已不再是这次移动当时写入的样子，还原会丢弃此后发生的变化。';

  @override
  String get positionNotReadable => '无法读取该角色已保存的位置。';

  @override
  String get npcPositionReadOnly => '游戏会从关卡而非存档中恢复 NPC 的位置，因此这些数值可以查看，但无法修改。';

  @override
  String get pickLocation => '选择地点…';

  @override
  String get pickLocationDialogTitle => '选择地点';

  @override
  String get applySpotRotation => '同时应用该地点的朝向';

  @override
  String get locationAreaOther => '其他';

  @override
  String get locationAreaCavalornValley => '卡瓦隆山谷';

  @override
  String get locationAreaEastForest => '东部森林';

  @override
  String get locationAreaFogTower => '雾塔';

  @override
  String get locationAreaIllegalWeedMixers => '非法沼泽烟草调配者';

  @override
  String get locationAreaOrcArena => '兽人竞技场';

  @override
  String get locationAreaOrcGraveyard => '兽人墓地';

  @override
  String get locationAreaShipwreck => '沉船残骸';

  @override
  String get locationAreaTundra => '苔原';

  @override
  String get locationCatalogUnavailable => '无法加载地点目录。';

  @override
  String get invalid => '无效';

  @override
  String get heroAttributes => '主角属性';

  @override
  String attributeBase(String name) {
    return '$name 基础值';
  }

  @override
  String attributeCurrent(String name) {
    return '$name 当前值';
  }

  @override
  String get attributeBaseValue => '基础值';

  @override
  String get attributeCurrentValue => '当前值';

  @override
  String get inventoryTitle => '物品栏';

  @override
  String get inventoryEmpty => '此物品栏为空。';

  @override
  String get inventoryNeedsDecoded => '编辑物品栏需要来自编解码器的已解码私有负载数据。';

  @override
  String get inventoryNoStacks => '已解码的私有负载中未找到物品堆叠。';

  @override
  String get resetInventoryChanges => '重置物品栏更改';

  @override
  String get addItemTooltipPendingAdd => '请先保存待处理的更改 — 每次保存只能添加一件新物品';

  @override
  String get addItemTooltipPendingRemove => '请先保存待处理的移除 — 每次保存只能进行一项结构更改';

  @override
  String get addItemTooltipPendingCount => '请先保存或重置待处理的数量更改 — 结构编辑必须单独保存';

  @override
  String get addItemTooltipDefault => '向物品栏添加物品';

  @override
  String get addItemButton => '添加物品';

  @override
  String get resetInventoryButton => '重置物品栏';

  @override
  String get resetInventoryTooltipDefault => '将此物品栏替换为游戏开始时的物品栏';

  @override
  String get resetInventoryTooltipBlocked => '请先保存或取消待处理的物品栏更改';

  @override
  String get pendingResetTitle => '重置为游戏开始时的物品栏';

  @override
  String pendingResetSubtitle(String level) {
    return '资源等级：$level';
  }

  @override
  String get cancelPendingReset => '取消重置';

  @override
  String pendingAddSubtitle(int count) {
    return '×$count — 待添加（尚未保存）';
  }

  @override
  String get cancelPendingAdd => '取消待添加';

  @override
  String get pendingRemovalSubtitle => '待移除（尚未保存）';

  @override
  String get cancelPendingRemoval => '取消待移除';

  @override
  String get filterItems => '筛选物品';

  @override
  String noItemsMatchQuery(String query) {
    return '没有物品匹配“$query”。';
  }

  @override
  String get pendingRemovalHidesAll => '待处理的移除隐藏了所有物品 — 请保存以应用。';

  @override
  String categoryWithCount(String label, int count) {
    return '$label（$count）';
  }

  @override
  String get itemTooltipIngredientFor => '用于制作';

  @override
  String itemTooltipTeaches(String item) {
    return '传授: $item';
  }

  @override
  String get itemTooltipValue => '价值';

  @override
  String get itemTooltipProtection => '防御';

  @override
  String get itemTooltipRequirements => '需求：';

  @override
  String get itemTooltipManaCost => '法力消耗';

  @override
  String get itemTooltipManaUpkeep => '蓄力法力消耗';

  @override
  String get itemCategoryAll => '全部';

  @override
  String get itemCategoryMeleeWeapon => '近战武器';

  @override
  String get itemCategoryRangedWeapon => '远程武器';

  @override
  String get itemCategoryMagic => '魔法';

  @override
  String get itemCategoryWearable => '穿戴装备';

  @override
  String get itemCategoryFood => '食物';

  @override
  String get itemCategoryPotion => '药水';

  @override
  String get itemCategoryMaterial => '材料';

  @override
  String get itemCategoryDocument => '文件';

  @override
  String get itemCategoryMisc => '杂项';

  @override
  String get itemCategoryArtefact => '神器';

  @override
  String get itemCategoryOther => '其他';

  @override
  String get count => '数量';

  @override
  String get min1 => '最少 1';

  @override
  String countTimes(String count) {
    return '×$count';
  }

  @override
  String get deleteEquippedTooltip => '无法删除：该物品可能已装备或已分配到快捷键槽位';

  @override
  String get removeBlockedTooltip => '请先保存或重置待处理的物品栏更改 — 添加或移除必须单独保存';

  @override
  String get removeItemFromInventory => '从物品栏移除物品';

  @override
  String get progressionLockedBody => '进度数据需要来自编解码器的已解码私有负载数据。';

  @override
  String get progressionNeedsTyped => '结构化进度数据需要完全解码且已验证类型解析的存档。';

  @override
  String get sectionQuests => '任务';

  @override
  String get sectionKnowledge => '知识';

  @override
  String get sectionEvents => '事件';

  @override
  String get firstPage => '首页';

  @override
  String get previousPage => '上一页';

  @override
  String get nextPage => '下一页';

  @override
  String get lastPage => '末页';

  @override
  String pageOfPages(int page, int total) {
    return '第 $page / $total 页';
  }

  @override
  String rangeOfTotal(int first, int last, int total) {
    return '$first–$last，共 $total';
  }

  @override
  String get perPage => '每页：';

  @override
  String get resetQuestChanges => '重置任务更改';

  @override
  String get searchQuests => '搜索任务';

  @override
  String get allGroups => '所有分组';

  @override
  String groupWithCount(String group, Object count) {
    return '$group（$count）';
  }

  @override
  String stateLabelWithCount(String label, int count) {
    return '$label $count';
  }

  @override
  String get questStateNone => '无';

  @override
  String get questStateAvailable => '可接取';

  @override
  String get questStateRunning => '进行中';

  @override
  String get questStateSucceeded => '已完成';

  @override
  String get questStateFailed => '已失败';

  @override
  String get questStateUnknown => '未知';

  @override
  String get dialogKnowledge => '对话知识';

  @override
  String get resetKnowledgeChanges => '重置知识更改';

  @override
  String get addNpc => '添加 NPC';

  @override
  String get searchNpcs => '搜索 NPC';

  @override
  String get npcStatusRowLabel => '状态';

  @override
  String get npcStatusAlive => '存活';

  @override
  String get npcStatusDead => '已死亡';

  @override
  String get npcRelationshipRowLabel => '关系';

  @override
  String get npcRelationshipUnavailable => '关系状态不可用';

  @override
  String get npcRelationshipAutomatic => '由游戏计算';

  @override
  String get npcRelationshipAutomaticHint => '未保存永久覆盖。游戏会评估公会、剧情、区域和犯罪规则。';

  @override
  String get npcRelationshipStoredHint =>
      '已保存为 NPC 对玩家的永久覆盖。公会、剧情、区域和犯罪规则仍可能改变游戏中的实际关系。';

  @override
  String get npcRelationshipFriend => '友好';

  @override
  String get npcRelationshipNeutral => '中立';

  @override
  String get npcRelationshipEnemy => '敌人';

  @override
  String npcRelationshipPending(String relationship) {
    return '保存后将为$relationship';
  }

  @override
  String npcStateHp(String hp, String maxHp) {
    return 'HP $hp / $maxHp';
  }

  @override
  String get npcReviveButton => '复活';

  @override
  String get npcReviveQueued => '将在保存时复活';

  @override
  String entriesForCharacter(String name) {
    return '条目 — $name';
  }

  @override
  String get selectNpcToSeeEntries => '选择一个 NPC 以查看条目';

  @override
  String get addKnowledgeEntry => '添加知识条目';

  @override
  String get browseCatalog => '浏览目录';

  @override
  String get alreadyExistsForCharacter => '该角色已存在此项。';

  @override
  String get alreadyInPendingChanges => '已在待处理的更改中。';

  @override
  String duplicateCheckFailed(String error) {
    return '重复检查失败 — 请重试：$error';
  }

  @override
  String pendingAddsCount(int count) {
    return '待添加（$count）';
  }

  @override
  String get undoAdd => '撤销添加';

  @override
  String get undoRemove => '撤销移除';

  @override
  String get removeEntry => '移除条目';

  @override
  String get selectNpcFromList => '从列表中选择一个 NPC';

  @override
  String characterWithCount(String name, int count) {
    return '$name（$count）';
  }

  @override
  String get memoryEvents => '记忆事件';

  @override
  String get searchCharacters => '搜索角色';

  @override
  String eventsForCharacter(String name) {
    return '事件 — $name';
  }

  @override
  String get selectCharacterToSeeEvents => '选择一个角色以查看事件';

  @override
  String get noTags => '（无标签）';

  @override
  String eventSubtitle(String time, String affected) {
    return 't=${time}s  $affected';
  }

  @override
  String get removeEvent => '移除事件';

  @override
  String get removeMemoryEventTitle => '移除记忆事件？';

  @override
  String get removeMemoryEventBody => '移除此记忆事件？将先写入一份备份。';

  @override
  String get memoryEventRemovalQueued => '事件移除已加入队列 — 按“保存”以应用。';

  @override
  String get duplicateEvent => '复制事件';

  @override
  String get duplicateMemoryEventTitle => '复制记忆事件？';

  @override
  String get duplicateMemoryEventBody => '复制此记忆事件？将先写入一份备份。';

  @override
  String get memoryEventDuplicationQueued => '事件复制已加入队列 — 按“保存”以应用。';

  @override
  String get selectCharacterFromList => '从列表中选择一个角色';

  @override
  String get factionsSidebar => '阵营';

  @override
  String get factionsForgiveButton => '宽恕';

  @override
  String get factionHostile => '敌对';

  @override
  String get factionFriendly => '友好';

  @override
  String crimeMurder(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count 起谋杀',
    );
    return '$_temp0';
  }

  @override
  String crimeAssault(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count 起袭击',
    );
    return '$_temp0';
  }

  @override
  String crimeTheft(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count 起盗窃',
    );
    return '$_temp0';
  }

  @override
  String crimeTrespassing(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count 起擅闯',
    );
    return '$_temp0';
  }

  @override
  String crimeThreat(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count 起威胁',
    );
    return '$_temp0';
  }

  @override
  String crimeOther(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count 起其他罪行',
    );
    return '$_temp0';
  }

  @override
  String get factionsForgiveQueued => '宽恕中…';

  @override
  String get factionsEmpty => '没有针对阵营的未了罪行。';

  @override
  String get factionGuildOldCamp => '旧营地';

  @override
  String get factionGuildNewCamp => '新营地';

  @override
  String get factionGuildSwampCamp => '沼泽营地';

  @override
  String get factionGuildOther => '其他/个人';

  @override
  String get allDataLockedBody => '目前，完整的数据源浏览器可用于 GSAV 存档文件。';

  @override
  String get allDataDescription =>
      '浏览 GSAV 元数据以及 PUBLIC/PRIVATE 中的所有类型化节点。可安全修改的标量值和原生结构体值均可编辑；容器和未解析的字节数据也会显示。';

  @override
  String get allDataEditable => '可编辑';

  @override
  String get allDataReadOnly => '只读';

  @override
  String get allDataType => '类型';

  @override
  String get allDataScalars => '标量';

  @override
  String get allDataStructs => '结构体';

  @override
  String get allDataContainers => '容器';

  @override
  String get allDataOpaque => '未解析数据';

  @override
  String get allDataNodes => '节点';

  @override
  String allDataChildren(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count 个子节点',
      one: '1 个子节点',
    );
    return '$_temp0';
  }

  @override
  String get allDataPending => '待保存';

  @override
  String get allDataTagInputHint => '用逗号或换行分隔标签';

  @override
  String allDataTypedSource(String source) {
    return '$source 类型化数据';
  }

  @override
  String get searchPropertiesLabel => '搜索属性（留空 = 列出全部） — 例如 Health、GameTime';

  @override
  String get decodingSaveTitle => '正在解码存档…';

  @override
  String get decodingSaveBody => '正在为首次搜索解码完整的私有负载。此操作每个存档只运行一次，之后的搜索将立即完成。';

  @override
  String get searchTheSaveTitle => '搜索存档';

  @override
  String get searchTheSaveBody => '输入属性名称并按回车键。留空则列出全部。';

  @override
  String get searchFailedTitle => '搜索失败';

  @override
  String get noMatchesTitle => '无匹配项';

  @override
  String get noMatchesBody => '没有属性路径包含所有这些词条。';

  @override
  String get value => '值';

  @override
  String get backupsTitle => '备份';

  @override
  String get refreshBackups => '刷新备份';

  @override
  String get noBackupsTitle => '无备份';

  @override
  String get noBackupsBody => '编辑存档时会在所选槽位旁创建备份文件。';

  @override
  String get slotBackups => '槽位备份';

  @override
  String get profileBackups => '存档配置备份';

  @override
  String get backupFactName => '名称';

  @override
  String get backupFactSlot => '槽位';

  @override
  String get backupFactCreated => '创建时间';

  @override
  String get backupFactSize => '大小';

  @override
  String get backupFactStatus => '状态';

  @override
  String get backupFactSha1 => 'SHA-1';

  @override
  String restoreBackupTooltip(String fileName) {
    return '恢复 $fileName';
  }

  @override
  String get appearanceTitle => '外观';

  @override
  String get uiFont => '字体';

  @override
  String get theme => '主题';

  @override
  String get themeLight => '浅色';

  @override
  String get themeDark => '深色';

  @override
  String get themeSystem => '跟随系统';

  @override
  String get uiScale => '界面缩放';

  @override
  String get resetZoomTooltip => '重置缩放（Ctrl+0）';

  @override
  String get zoomTip => '提示：在应用内任意位置按 Ctrl + / Ctrl - 均可调整缩放。';

  @override
  String get language => '界面';

  @override
  String get gameTextLanguage => '游戏文本';

  @override
  String get gameTextLanguageHint => '选择界面语言时，也会选中对应的游戏文本。之后可以单独更改游戏文本。';

  @override
  String get updatesTitle => '更新';

  @override
  String get checkForUpdatesAutomatically => '自动检查更新';

  @override
  String get checkForUpdatesNow => '立即检查更新';

  @override
  String get updatesPortableNotice => '便携版会在浏览器中打开下载页面。请用新下载的文件替换现有文件。';

  @override
  String get updateAvailableTitle => '有可用更新';

  @override
  String updateAvailableMessage(Object version, Object current) {
    return '版本 $version 可用。您当前为 $current。';
  }

  @override
  String get updateDownload => '下载';

  @override
  String updateOpenFailed(String url) {
    return '无法打开下载页面。你可以通过 $url 访问。';
  }

  @override
  String get updateLater => '稍后';

  @override
  String get updateUpToDate => '您正在使用最新版本。';

  @override
  String get updateCheckFailed => '无法检查更新，请稍后重试。';

  @override
  String get gameTextTitle => '游戏文本';

  @override
  String get itemImagesTitle => '物品图片';

  @override
  String get gameDataTitle => '游戏数据';

  @override
  String itemImagesReady(int count) {
    return '已准备 $count 张物品图片。';
  }

  @override
  String get itemImagesUnavailable => '物品图片不可用，将改用分类图标。';

  @override
  String get checkRefreshItemImages => '检查 / 更新物品图片';

  @override
  String get gameDataSourceMissing => '无法自动准备游戏文本。你可以在设置中选择本地化缓存。';

  @override
  String get loadingTexts => '正在加载文本…';

  @override
  String get loadingImages => '正在加载图片…';

  @override
  String get preparing => '正在准备…';

  @override
  String gameTextExtractedWithCounts(int ids, int languages) {
    return '已提取：$languages 种语言共 $ids 个 ID。';
  }

  @override
  String get gameTextExtracted => '本地化游戏文本已提取。';

  @override
  String get gameTextNotExtracted => '本地化游戏文本尚未提取。';

  @override
  String get extracting => '正在提取…';

  @override
  String get extractRefreshLocalizedText => '提取 / 刷新本地化文本';

  @override
  String get extractionComplete => '提取完成';

  @override
  String get extractionFailed => '提取失败';

  @override
  String get localizationCacheFileType => '本地化缓存';

  @override
  String get savegameDirectoryTitle => '存档目录';

  @override
  String get folder => '文件夹';

  @override
  String get codecTitle => '编解码器';

  @override
  String get check => '检查';

  @override
  String get roundtrip => '往返测试';

  @override
  String get noCodecStatus => '无编解码器状态';

  @override
  String get codecReady => '编解码器就绪';

  @override
  String get codecReadOnly => '编解码器只读';

  @override
  String get codecUnavailable => '编解码器不可用';

  @override
  String get details => '详情';

  @override
  String codecStatusLine(String status) {
    return '状态：$status';
  }

  @override
  String codecCapabilityLine(String decompress, String compress) {
    return '解压：$decompress | 压缩：$compress';
  }

  @override
  String codecBackendLine(String backend) {
    return '后端：$backend';
  }

  @override
  String get yes => '是';

  @override
  String get no => '否';

  @override
  String aboutVersion(String version, String sha) {
    return '版本 $version（$sha）';
  }

  @override
  String get aboutCopyright => '© 2026 Daniel Hoer';

  @override
  String get aboutLicense => '基于 MIT 许可证授权。';

  @override
  String difficultyTitle(String profile) {
    return '难度 — $profile';
  }

  @override
  String get difficultyNoProfile => '无存档配置';

  @override
  String get difficultyNoDifficulty => '无难度';

  @override
  String get difficultyLabel => '难度';

  @override
  String get difficultyTooltipNoProfile => '未选择存档配置';

  @override
  String get difficultyTooltipEdit => '编辑此存档配置的难度';

  @override
  String get difficultyTooltipNoEditable => '此存档配置没有可编辑的难度';

  @override
  String get preset => '预设';

  @override
  String get presetNovice => '简单';

  @override
  String get presetGothic => '哥特';

  @override
  String get presetHard => '困难';

  @override
  String get presetCustom => '自定义';

  @override
  String unrecognisedPreset(Object preset) {
    return '存储的预设无法识别（$preset）。你仍可保存流畅助手 / 永久死亡的更改，或在上方选择一个预设以覆盖它。';
  }

  @override
  String get closeCombatFlowHelper => '近战流程助手';

  @override
  String get permadeath => '永久死亡';

  @override
  String get notAvailableOnNovice => '新手难度下不可用';

  @override
  String get levelCombat => '战斗';

  @override
  String get levelResources => '资源';

  @override
  String get levelProgression => '进度';

  @override
  String get difficultyAppliesToAllSaves => '难度将应用于此存档配置中的所有存档。';

  @override
  String get savingDifficultyFailed => '保存难度失败。';

  @override
  String get addItemDialogTitle => '添加物品';

  @override
  String get searchItems => '搜索物品';

  @override
  String failedToLoadCatalog(String error) {
    return '加载目录失败：$error';
  }

  @override
  String get noItemsAvailableToAdd => '没有可添加的物品';

  @override
  String get noItemsMatch => '没有匹配的物品';

  @override
  String get countMustBeAtLeast1 => '必须 ≥ 1';

  @override
  String countMustBeAtMost(int max) {
    return '必须 ≤ $max';
  }

  @override
  String get addNpcDialogTitle => '添加 NPC';

  @override
  String get noNpcsAvailableToAdd => '没有可添加的 NPC';

  @override
  String get noNpcsMatch => '没有匹配的 NPC';

  @override
  String get categoryAll => '全部';

  @override
  String allWithCount(int count) {
    return '全部（$count）';
  }

  @override
  String get addKnowledgeEntryDialogTitle => '添加知识条目';

  @override
  String get searchEntries => '搜索条目';

  @override
  String get noKnowledgeEntriesAvailableToAdd => '没有可添加的知识条目';

  @override
  String get noEntriesMatch => '没有匹配的条目';

  @override
  String get heroGroupMainStats => '主要属性';

  @override
  String get heroGroupCombatMovement => '战斗 / 移动';

  @override
  String get heroGroupResistances => '抗性';

  @override
  String get heroGroupThieving => '盗窃';

  @override
  String get heroGroupAdvanced => '高级';

  @override
  String get heroGroupDiving => '潜水';

  @override
  String get heroDivingSkillNote =>
      '学会潜水后，游戏每次读取存档都会把屏息量和恢复速度重置为技能自带的数值。每秒消耗量则保持你设定的值。';

  @override
  String get heroGroupSleep => '睡眠';

  @override
  String get heroGroupIntoxication => '醉酒';

  @override
  String get heroEntryHeroTransform => '位置';

  @override
  String attributeEmpty(String name) {
    return '$name 为空 — 请输入一个值，或在保存前恢复原始值。';
  }

  @override
  String attributeInvalidNumber(String name, String text) {
    return '$name 的数字无效：“$text”';
  }

  @override
  String get loadingEditorData => '正在加载编辑器数据';

  @override
  String savingProgress(int done, int total) {
    return '正在保存… $done/$total';
  }

  @override
  String localizedTextExtractedCount(int idCount, int languageCount) {
    return '已提取 $idCount 个 ID，涵盖 $languageCount 种语言';
  }

  @override
  String get skillSmithing1H => '单手武器锻造';

  @override
  String get skillSmithing2H => '双手武器锻造';

  @override
  String get skillCircleNovice => '新手法师';

  @override
  String get skillCircle1 => '第一魔法环阶';

  @override
  String get skillCircle2 => '第二魔法环阶';

  @override
  String get skillCircle3 => '第三魔法环阶';

  @override
  String get skillCircle4 => '第四魔法环阶';

  @override
  String get skillCircle5 => '第五魔法环阶';

  @override
  String get skillCircle6 => '第六魔法环阶';

  @override
  String get sectionGlossary => '图鉴';

  @override
  String get glossarySearch => '搜索图鉴';

  @override
  String get glossaryOldCamp => '旧营地';

  @override
  String get glossaryNewCamp => '新营地';

  @override
  String get glossarySwampCamp => '沼泽营地';

  @override
  String get glossaryOutsiders => '外来者';

  @override
  String get glossaryCreatures => '生物';

  @override
  String get glossaryLocations => '地点';

  @override
  String get glossaryFilterLabel => '筛选';

  @override
  String get glossaryFilterTraders => '商人';

  @override
  String get glossaryFilterTeachers => '导师';

  @override
  String get roleTrader => '商人';

  @override
  String get roleDead => '已死亡';

  @override
  String get roleTeacher => '导师';

  @override
  String get roleArmorer => '护甲匠';

  @override
  String get glossaryFilterArmorers => '护甲匠';

  @override
  String get glossaryFilterHostile => '敌对';

  @override
  String get glossaryRelationshipFilterNote =>
      '显示存档中保存的永久敌对覆盖。公会、剧情、区域和犯罪产生的动态关系仅在游戏中计算。';

  @override
  String get glossaryFilterDead => '已死亡';

  @override
  String get glossaryAddEntry => '添加图鉴条目';

  @override
  String get glossaryAddTitle => '添加图鉴条目';

  @override
  String get glossaryResetChanges => '重置图鉴更改';

  @override
  String get glossaryNoVisibleEntries => '此视图中没有匹配的可见图鉴条目。';

  @override
  String get glossaryNoHiddenEntries => '所有可用条目均已显示。';

  @override
  String get glossaryNoMatch => '没有匹配的图鉴条目。';

  @override
  String get glossarySelectEntry => '选择一个图鉴条目以编辑其内容。';

  @override
  String glossaryEntryCount(int count) {
    return '$count 个条目';
  }

  @override
  String glossarySegmentsCount(int unlocked, int total) {
    return '已解锁 $unlocked/$total 个条目';
  }

  @override
  String get glossaryPortraitUnlocked => '肖像已解锁';

  @override
  String get glossaryPortraitSilhouette => '剪影 — 肖像尚未解锁';

  @override
  String get glossarySegments => '条目';

  @override
  String get glossaryPending => '未保存的更改';

  @override
  String get glossaryShowFullText => '显示条目全文';

  @override
  String get glossarySegmentIntroduction => '介绍 / 肖像';

  @override
  String get glossarySegmentUnlock => '发现';

  @override
  String glossarySegmentEntry(int number) {
    return '条目 $number';
  }

  @override
  String get questJournalAll => '所有任务';

  @override
  String get questJournalOldCamp => '旧营地';

  @override
  String get questJournalNewCamp => '新营地';

  @override
  String get questJournalSwampCamp => '沼泽营地';

  @override
  String get questJournalColony => '殖民地';

  @override
  String get questJournalCompleted => '已完成';

  @override
  String get questJournalHint => '游戏内日志视图。内部状态和尚未开始的任务状态仍可在“所有数据”中查看。';

  @override
  String get questJournalNoEntries => '没有符合当前筛选条件的日志任务。';

  @override
  String get glossaryTutorials => '教程';

  @override
  String get tutorialGateNote => '这些行控制存档中的教程解锁状态。一个解锁状态不一定对应游戏中的单个教程页面。';

  @override
  String get tutorialResetChanges => '重置教程更改';

  @override
  String get tutorialNoGates => '此存档中没有可用的教程解锁状态。';

  @override
  String tutorialGateUnlockCount(int unlocked, int total) {
    return '已解锁 $unlocked/$total 个教程';
  }

  @override
  String get tutorialGateCombatBasics => '战斗基础';

  @override
  String get tutorialGateCrafting => '制作';

  @override
  String get tutorialGateCrime => '犯罪与后果';

  @override
  String get tutorialGateDrugs => '消耗品与效果';

  @override
  String get tutorialGateLockpicking => '开锁';

  @override
  String get tutorialGateMagic => '魔法';

  @override
  String get tutorialGateMap => '地图';

  @override
  String get tutorialGateMeleeCombat => '近战';

  @override
  String get tutorialGateNavigation => '移动与导航';

  @override
  String get tutorialGatePerception => '感知';

  @override
  String get tutorialGatePlayerProgression => '角色成长';

  @override
  String get tutorialGateRanged => '远程战斗';

  @override
  String get tutorialGateRiding => '骑乘';

  @override
  String get tutorialGateSleep => '睡眠';

  @override
  String get tutorialGateTrading => '交易';

  @override
  String get windowMinimizeTooltip => '最小化';

  @override
  String get windowMaximizeTooltip => '最大化';

  @override
  String get windowRestoreTooltip => '还原';

  @override
  String get fallbackDialogEntry => '对话条目';

  @override
  String get fallbackDialogChoice => '对话选项';

  @override
  String get fallbackDialogTopic => '对话主题';

  @override
  String get fallbackDialogInformation => '对话信息';

  @override
  String get fallbackQuest => '任务';

  @override
  String get fallbackObjective => '目标';

  @override
  String get fallbackItem => '物品';

  @override
  String get attributeSkillPointsFallback => '学习点数（LP）';

  @override
  String attributeManualFallbackLabel(String attributeId, String fallback) {
    String _temp0 = intl.Intl.selectLogic(attributeId, {
      'SuperArmor': '霸体值',
      'MaxSuperArmor': '最大霸体值',
      'DamageMultiplier': '受到的伤害',
      'SpeedModifier': '移动速度',
      'Oxygen': '氧气量',
      'MaxOxygen': '最大氧气量',
      'OxygenDepletionRate': '每秒氧气消耗',
      'OxygenRecoveryRate': '每秒氧气恢复',
      'CriticalLevelPercent': '缺氧警告阈值',
      'SleepTime': '剩余有效睡眠',
      'MaxSleepTime': '最大有效睡眠',
      'SleepTimeRecoveryAmount': '有效睡眠回补量',
      'SleepTimeRecoveryPeriod': '回补间隔',
      'MaxRestTime': '最长卧床时间',
      'Health_RecoveryRatePerHourOfSleep': '每小时睡眠回复生命',
      'Mana_RecoveryRatePerHourOfSleep': '每小时睡眠回复法力',
      'Alcohol': '酒精值',
      'MaxAlcohol': '最大酒精值',
      'AlcoholDepletionRate': '醒酒速度',
      'Swampweed': '沼泽草值',
      'MaxSwampweed': '最大沼泽草值',
      'SwampweedDepletionRate': '药性消退速度',
      'XPExecutedBounty': '倒地处决获得的经验',
      'XPKillOrDefeatBounty': '击败获得的经验',
      'Level': '等级',
      'LockpickDurability': '开锁器耐久',
      'LockpickPrecision': '开锁精度',
      'PickPocketing': '扒窃',
      'other': '$fallback',
    });
    return '$_temp0';
  }

  @override
  String attributeManualTooltip(String attributeId) {
    String _temp0 = intl.Intl.selectLogic(attributeId, {
      'SuperArmor': '该角色在被一击打得踉跄之前还能扛下多少打击。',
      'MaxSuperArmor': '霸体值的上限，会随着等级提升和所穿的护甲一起增长。',
      'DamageMultiplier': '作用于该角色所受伤害的系数——1 为正常，数值越高越吃痛。',
      'SpeedModifier': '该角色移动快慢的系数——1 为正常。',
      'Oxygen': '水下剩余的呼吸秒数，归零时该角色就会淹死。',
      'MaxOxygen': '该角色能在水下待多少秒，潜水技能可以提高这个上限。',
      'OxygenDepletionRate': '潜在水下时每秒消耗掉的空气量。',
      'OxygenRecoveryRate': '浮出水面后每秒回来的空气量。',
      'CriticalLevelPercent': '剩余空气低到这个比例时，游戏就会发出溺水警告。',
      'SleepTime': '还能带来恢复的睡眠小时数，超出之后再睡游戏也不会给任何恢复。',
      'MaxSleepTime': '该角色能攒下的有效睡眠时间上限。',
      'SleepTimeRecoveryAmount': '每次补充时重新加回来的有效睡眠小时数。',
      'SleepTimeRecoveryPeriod': '有效睡眠时间隔多久才会重新补满。',
      'MaxRestTime': '游戏允许一次躺在床上的最长时间。',
      'Health_RecoveryRatePerHourOfSleep': '每睡一小时能恢复的最大生命值比例。',
      'Mana_RecoveryRatePerHourOfSleep': '每睡一小时能恢复的最大法力值比例。',
      'Alcohol': '该角色醉到什么程度，较高的档位会拿敏捷和法力去换力量。',
      'MaxAlcohol': '该角色能达到的最高酒精值。',
      'AlcoholDepletionRate': '酒精值往清醒方向回落得有多快。',
      'Swampweed': '该角色嗨到什么程度，较高的档位会让其属性此消彼长。',
      'MaxSwampweed': '该角色能达到的最高沼泽草值。',
      'SwampweedDepletionRate': '沼泽草带来的迷幻劲头消退得有多快。',
      'XPExecutedBounty': '在这名角色已经被打倒在地时再将其杀死，所能拿到的经验值。',
      'XPKillOrDefeatBounty': '把这名角色打倒时所能拿到的经验值，不管对方是当场毙命还是只被打晕在地。',
      'Level': '角色等级。随经验提升，并带来学习点数。',
      'LockpickDurability': '由开锁技能决定：未学2、已学4、精通6。',
      'LockpickPrecision': '由开锁技能决定：未学0、已学1、精通2。',
      'PickPocketing': '由扒窃技能决定：未学-30、已学-10、精通+10。',
      'other': '?',
    });
    return '$_temp0';
  }

  @override
  String get knowledgeTypeVoiceLine => '语音台词';

  @override
  String get knowledgeTypeOther => '其他';

  @override
  String get armorUpgradeUpper => '上部';

  @override
  String get armorUpgradeMiddle => '中部';

  @override
  String get armorUpgradeLower => '下部';

  @override
  String get knowledgeCategoryTopic => '主题';

  @override
  String get knowledgeCategoryChoice => '选项';

  @override
  String get knowledgeCategoryInfo => '信息';

  @override
  String get statusOk => '正常';

  @override
  String get statusFailed => '失败';

  @override
  String get missingSaveReference => '文件缺失';

  @override
  String missingSaveReferenceDescription(String slot) {
    return '$slot.sav 缺失。它可能已被删除、移动或重命名；存档配置仍在引用它。';
  }

  @override
  String get removeFromProfile => '从存档配置中移除';

  @override
  String get deleteSavegame => '删除存档';

  @override
  String get deleteSavegameTitle => '删除存档？';

  @override
  String deleteSavegameBody(String save, String fileName, String profile) {
    return '要删除 $save（$fileName）吗？它将从 $profile 中移除，并从存档文件夹中删除。GORE 会先创建备份。';
  }

  @override
  String get removeSaveFromProfileTitle => '从存档配置中移除存档？';

  @override
  String removeSaveFromProfileBody(String save, String profile) {
    return '要从 $profile 中移除 $save 吗？如果存档文件仍然存在，文件本身将会保留。';
  }

  @override
  String get unassignedSave => '未分配给存档配置';

  @override
  String get armorUpgradeLight => '轻型';

  @override
  String get armorUpgradeMedium => '中型';

  @override
  String get armorUpgradeHeavy => '重型';

  @override
  String get knowledgeCaptionForcedConversation => '强制对话';

  @override
  String get knowledgeCaptionFollowupTopic => '后续话题';

  @override
  String get knowledgeCaptionFallbackTopic => '后备话题';

  @override
  String durationMinutes(int minutes) {
    return '$minutes 分钟';
  }

  @override
  String durationHours(int hours) {
    return '$hours 小时';
  }

  @override
  String durationHoursMinutes(int hours, int minutes) {
    return '$hours 小时 $minutes 分钟';
  }

  @override
  String get backupStatusInvalidProfileStructure => '存档配置数据无效';

  @override
  String get backupStatusSlotMetadataMissing => '所选存档的元数据缺失';

  @override
  String defaultProfileName(int id) {
    return '存档配置 $id';
  }

  @override
  String get statusUnknown => '未知';

  @override
  String editorUnexpectedError(String details) {
    return '意外错误：$details';
  }

  @override
  String get editorOperationInProgress => '另一项操作正在进行中。请稍后重试。';

  @override
  String get editorUnsavedBeforeDifficulty =>
      '存档中有未保存的修改。更改存档配置难度前，请先保存或重置这些修改。';

  @override
  String get editorNoSaveFolderSelected => '未选择存档文件夹。';

  @override
  String get editorNoSaveSelected => '未选择存档。';

  @override
  String get coreUnknownError => '核心组件发生未知错误';

  @override
  String get editorUnsavedBeforeSwitchProfile => '请先保存或重置未保存的修改；切换存档配置会离开当前存档。';

  @override
  String get editorUnsavedBeforeOpenFile => '打开其他文件前，请先保存或重置未保存的修改。';

  @override
  String get editorSelectSavFile => '请选择 .sav 存档文件。';

  @override
  String get editorNotGothicGsav => '所选文件不是 Gothic GSAV 存档。';

  @override
  String get editorUnsavedBeforeChangeSaveProfile =>
      '更改存档所属的存档配置前，请先保存或重置未保存的修改。';

  @override
  String get editorUnsavedBeforeRemoveProfile => '从存档配置中移除存档前，请先保存或重置未保存的修改。';

  @override
  String get editorUnsavedBeforeDeleteSave => '删除此存档前，请先保存或重置未保存的修改。';

  @override
  String get editorUnsavedBeforeRestoreProfile =>
      '存档中有未保存的修改。恢复存档配置备份前，请先保存或重置这些修改。';

  @override
  String editorConflictingPropertyEdits(String path) {
    return '两个标签页中未保存的修改针对同一属性 ($path)。请重置或撤销其中一项，然后再次保存。';
  }

  @override
  String editorGlossaryMemoryConflict(String path) {
    return '图鉴分段更改和“全部数据”中另一项未保存的修改都针对 Hero MemorizedEvents 数组 ($path)。图鉴更改会在该数组中添加或移除条目，因此两项修改无法同时保存。请重置或撤销其中一项，然后再次保存。';
  }

  @override
  String editorGlossaryQuestConflict(String path) {
    return '图鉴分段更改和另一项未保存的修改都针对同一任务的 CurrentState 属性 ($path)。图鉴更改本身会更新该状态。请重置或撤销其中一项，然后再次保存。';
  }

  @override
  String editorRelationshipConflict(String path) {
    return '关系覆盖设置和“全部数据”中另一项未保存的修改都针对同一 NPC 关系条目 ($path)。结构化的关系更改可能会替换该条目中的修正值，因此两项修改无法同时保存。请重置或撤销其中一项，然后再次保存。';
  }

  @override
  String editorMultipleStructuralArrayEdits(String path) {
    return '同一数组 ($path) 有多项未保存的结构更改。添加另一项更改前，请先保存或重置第一项更改。';
  }

  @override
  String editorStructuralArrayConflict(String path) {
    return '事件结构更改和“全部数据”中另一项未保存的修改都针对 $path。继续前，请保存或重置其中一项。';
  }

  @override
  String get editorSkillsEffectConflict =>
      '“技能”中的更改和“全部数据”中针对同一角色效果 (ActiveEffects › EffectSpec › Def) 的修改都在等待保存。两项修改无法同时保存。请重置或撤销其中一项，然后再次保存。';

  @override
  String get editorInventoryResetConflict =>
      '重置物品栏和对同一物品栏的另一项修改都在等待保存。重置会替换整个物品栏并丢弃另一项修改。请重置或撤销其中一项，然后再次保存。';

  @override
  String get editorUseFolder => '使用此文件夹';

  @override
  String get editorGothicSavegameFileType => 'Gothic 存档';

  @override
  String get editorNoDifficultyChanges => '没有需要保存的难度更改';

  @override
  String get editorDifficultyWritten => '难度已写入存档配置（已创建备份）';

  @override
  String editorChangesSavedWithBackup(int count) {
    return '已保存 $count 项更改并创建备份';
  }

  @override
  String editorPlacementNoteFailed(String details) {
    return '移动已保存，但无法写入用于还原的记录：$details';
  }

  @override
  String editorProfileNotFound(int profileId) {
    return '未找到存档配置 $profileId。';
  }

  @override
  String get editorNoFreeSaveSlot => '游戏存档文件夹中没有可用的存档槽位（G1R-001 至 G1R-999）。';

  @override
  String editorSaveImportedAssigned(int profileId) {
    return '存档已导入并分配给存档配置 $profileId';
  }

  @override
  String editorSaveAssigned(int profileId) {
    return '存档已分配给存档配置 $profileId（已创建配套备份）';
  }

  @override
  String editorSaveSlotNotAssigned(String slot, int profileId) {
    return '存档槽位 $slot 未分配给存档配置 $profileId。';
  }

  @override
  String get editorSaveRemovedFromProfile => '已从存档配置中移除存档';

  @override
  String get editorSaveDeleted => '存档已删除；已创建备份';

  @override
  String editorRestoredBackup(String path) {
    return '已恢复备份：$path';
  }

  @override
  String editorRestoredBackupWithoutCompanion(String path) {
    return '已恢复备份：$path（由于没有匹配的配套备份，PersistentDataList.sav 保持不变；槽位元数据可能不同）';
  }

  @override
  String editorCodecRoundtripPassed(int chunkIndex, int bytes) {
    return '编解码器往返验证通过：区块 $chunkIndex 已重新压缩为 $bytes 字节';
  }

  @override
  String editorDifficultyWriteFailed(String details) {
    return '无法写入存档配置难度：$details';
  }

  @override
  String editorProfileAssignmentFailed(String details) {
    return '无法将存档分配给存档配置：$details';
  }

  @override
  String editorProfileRemovalFailed(String details) {
    return '无法从存档配置中移除存档：$details';
  }

  @override
  String editorDeleteSaveFailed(String details) {
    return '无法删除存档：$details';
  }

  @override
  String editorSaveFailed(String details) {
    return '无法保存修改：$details';
  }

  @override
  String editorScanSavesFailed(String details) {
    return '扫描存档失败：$details';
  }

  @override
  String editorInspectSaveFailed(String details) {
    return '检查存档失败：$details';
  }

  @override
  String editorLoadBackupsFailed(String details) {
    return '加载备份失败：$details';
  }

  @override
  String editorRestoreFailed(String details) {
    return '无法恢复备份：$details';
  }

  @override
  String editorRestoreReloadFailed(String path, String details) {
    return '已恢复备份：$path，但重新加载存档失败：$details';
  }

  @override
  String editorCodecCheckFailed(String details) {
    return '编解码器检查失败：$details';
  }

  @override
  String editorCodecValidationFailed(String details) {
    return '编解码器往返验证失败：$details';
  }

  @override
  String editorPropertySearchFailed(String details) {
    return '属性搜索失败：$details';
  }

  @override
  String get editorSelectionChangedWhileLoadingHeroAttributes =>
      '加载英雄属性时，所选存档发生了变化。';

  @override
  String editorSkillsLoadFailed(String details) {
    return '加载技能失败：$details';
  }

  @override
  String editorProgressionQueryFailed(String details) {
    return '查询进度失败：$details';
  }

  @override
  String editorNpcListFailed(String details) {
    return '加载 NPC 列表失败：$details';
  }

  @override
  String editorCharacterListFailed(String details) {
    return '加载角色列表失败：$details';
  }

  @override
  String editorNpcAttributesFailed(String details) {
    return '加载 NPC 属性失败：$details';
  }

  @override
  String editorNpcPositionFailed(String details) {
    return '加载 NPC 位置失败：$details';
  }

  @override
  String editorNpcInventoryFailed(String details) {
    return '加载 NPC 物品栏失败：$details';
  }

  @override
  String editorFactionListFailed(String details) {
    return '加载阵营列表失败：$details';
  }

  @override
  String get editorNoBackupPath => '无';

  @override
  String editorBackupMessage(String prefix, String backupPath) {
    return '$prefix：$backupPath';
  }

  @override
  String editorBackupMessageWithPersistent(
    String prefix,
    String backupPath,
    String persistentPath,
  ) {
    return '$prefix：$backupPath；PersistentDataList 备份：$persistentPath';
  }

  @override
  String localizationStatusFailed(String details) {
    return '获取本地化状态失败：$details';
  }

  @override
  String localizationExtractionFailed(String details) {
    return '提取失败：$details';
  }

  @override
  String glossaryLoadFailed(String details) {
    return '加载图鉴失败：$details';
  }

  @override
  String backupStatusError(String details) {
    return '备份错误：$details';
  }

  @override
  String memoryEventCategory(String category, String fallback) {
    String _temp0 = intl.Intl.selectLogic(category, {
      'quest': '任务',
      'document': '文档',
      'story': '剧情',
      'exploration': '探索',
      'combat': '战斗',
      'social': '社交',
      'item': '物品',
      'learning': '学习',
      'guild': '公会',
      'crime': '犯罪',
      'rest': '休息',
      'other': '$fallback',
    });
    return '$_temp0';
  }

  @override
  String memoryEventAction(String kind, String fallback) {
    String _temp0 = intl.Intl.selectLogic(kind, {
      'questStarted': '任务已开始',
      'questSucceeded': '任务已完成',
      'questFailed': '任务失败',
      'documentRead': '已阅读文档',
      'documentSegmentUnlocked': '已发现条目',
      'documentSegmentViewed': '已查看条目',
      'chapterCompleted': '章节已完成',
      'areaEntered': '进入区域',
      'areaLeft': '离开区域',
      'characterKilled': '角色已被杀死',
      'characterDefeated': '角色已被击败',
      'combatDodge': '已闪避攻击',
      'characterDebuffed': '已施加负面效果',
      'tradeAvailable': '已解锁交易',
      'itemObtained': '获得物品',
      'itemCrafted': '制作物品',
      'skillStateRecorded': '已记录技能状态',
      'recipeLearned': '学会配方',
      'guildJoined': '加入公会',
      'crimeRecorded': '犯罪已记录',
      'slept': '睡眠',
      'storyEvent': '剧情事件',
      'other': '$fallback',
    });
    return '$_temp0';
  }

  @override
  String memoryEventTitleWithSubject(String action, String subject) {
    return '$action：$subject';
  }

  @override
  String memoryEventFact(String fact, String fallback) {
    String _temp0 = intl.Intl.selectLogic(fact, {
      'gameTime': '游戏时间',
      'duration': '持续时间',
      'chapter': '章节',
      'instigator': '触发者',
      'affected': '受影响对象',
      'amount': '数量',
      'primaryObject': '对象',
      'secondaryObject': '上下文',
      'segmentText': '条目文本',
      'other': '$fallback',
    });
    return '$_temp0';
  }

  @override
  String memoryEventGameTime(int day, String time) {
    return '第 $day 天，$time';
  }

  @override
  String memoryEventSecondsValue(String value) {
    return '$value 秒';
  }

  @override
  String memoryEventMoreValues(String values, int count) {
    return '$values +$count';
  }

  @override
  String get memoryEventHero => '主角';

  @override
  String get memoryEventDetails => '详细信息';

  @override
  String get memoryEventTags => '标签';

  @override
  String get memoryEventTechnicalData => '技术信息';

  @override
  String get memoryEventIndex => '索引';

  @override
  String get memoryEventPosition => '位置';

  @override
  String get memoryEventPayload => '事件数据';

  @override
  String get memoryEventSubject => '关联对象';

  @override
  String glossaryCatalogSegmentLabel(String segmentId, String fallback) {
    String _temp0 = intl.Intl.selectLogic(segmentId, {
      'Access': '通行',
      'AccessDenied': '禁止通行',
      'AccesToTemple': '进入神殿',
      'Advice': '建议',
      'AfterFight': '战斗之后',
      'AfterFireMages': '火法师事件之后',
      'AfterNek': '尼克之后',
      'AfterQuest': '任务之后',
      'Alone': '独自一人',
      'Amulet': '护符',
      'Annoying': '烦人',
      'Armor': '护甲',
      'Avoid': '回避',
      'Backstory': '背景故事',
      'BackStory': '背景故事',
      'BasicMagic': '基础魔法',
      'Beated': '被打败',
      'BecomeMercenary': '成为佣兵',
      'Beer': '啤酒',
      'Bestiary': '怪物图鉴',
      'Blessing': '祝福',
      'Boss': '首领',
      'Bully': '恶霸',
      'BullyAdvice': '应对恶霸的建议',
      'Camp': '营地',
      'CampDivided': '分裂的营地',
      'CareOfMessengers': '照顾信使',
      'ChangeOpinion': '改变看法',
      'ChargeUriziel': '为尤里泽尔充能',
      'Chosen': '天选者',
      'Contact': '接触',
      'Courier': '信使',
      'CraftBows': '制作弓箭',
      'Crazy': '疯癫',
      'DailyMeal': '每日餐食',
      'DailyRation_Trader': '每日口粮商人',
      'DAM': '水坝',
      'Dead': '死亡',
      'Deal': '交易',
      'Dealer': '交易商',
      'Deceived': '受骗',
      'Dementia': '痴呆',
      'DenyAccess': '拒绝通行',
      'DifferentOpinion': '不同意见',
      'Discussion': '讨论',
      'DontTalk': '不许交谈',
      'Duel': '决斗',
      'Entrance': '入口',
      'Escape': '逃脱',
      'Extended': '扩展',
      'Extra': '额外',
      'ExtraInfo': '额外信息',
      'Fanatic': '狂信徒',
      'Fight': '战斗',
      'FindUlumulu': '寻找乌鲁穆鲁',
      'FireMages': '火法师',
      'FireMagesEscape': '火法师的逃脱',
      'FiskNewDealer': '菲斯克的新销赃贩子',
      'FiskNewDealerCompleted': '菲斯克的新销赃贩子——已完成',
      'FogTower': '雾塔',
      'Food': '食物',
      'Forgave': '已原谅',
      'Forgive': '原谅',
      'Forgiven': '已获原谅',
      'FourFriends': '四位好友',
      'FreeHut': '空闲小屋',
      'FreeMine': '自由矿场',
      'Fury': '狂怒',
      'GoodTeacher': '好导师',
      'Gossip': '传闻',
      'GotScavenger': '获得食尸鸟',
      'GrantedAccess': '已获准通行',
      'GRDArmor': '卫兵防具',
      'Guide': '向导',
      'HateMages': '仇恨法师',
      'HateMagesExplanation': '仇恨法师的原因',
      'HateRiceLord': '憎恨稻田主',
      'Heal': '治疗',
      'Healing': '治疗',
      'Help': '帮助',
      'Helper': '帮手',
      'HelpKagan': '帮助卡根',
      'HutStory': '小屋的故事',
      'Ignore': '无视',
      'Impress': '打动',
      'ImpressAlchemy': '用炼金术打动',
      'ImpressInscription': '用铭文打动',
      'Info': '信息',
      'Interested': '感兴趣',
      'Introduction': '初识',
      'Introduction_2': '初识 2',
      'Introduction_Armor': '护甲介绍',
      'Introduction_Teacher': '初识（导师）',
      'Introduction_Trader': '初识（商人）',
      'Invocation': '召唤仪式',
      'JoinSC': '加入沼泽营地',
      'Joint': '沼泽草烟卷',
      'KalomCamp': '科尔·卡隆的营地',
      'Leader': '领袖',
      'Learning': '学习',
      'LearnOrcish': '学习兽人语',
      'LeftParty': '离队',
      'Library': '图书馆',
      'Lie': '谎言',
      'Lock': '锁',
      'Lockpick': '开锁工具',
      'Mad': '疯狂',
      'Mandibles': '矿爬虫的下颚',
      'MapMaker': '制图师',
      'Monastery': '修道院',
      'MordragKO': '击倒莫德拉格',
      'Nek': '尼克',
      'NewCamp': '新营地',
      'NewCamper': '新营地成员',
      'NewLeader': '新领袖',
      'NightPatrol': '夜间巡逻',
      'NotInterested': '不感兴趣',
      'OldCamp': '旧营地',
      'OrcEnclaveEntrance': '兽族聚居地入口',
      'OrcGraveyard': '兽人墓地',
      'OreArmor': '矿石铠甲',
      'Party': '队伍',
      'Pay': '付款',
      'PayMoney': '付钱',
      'Permission': '许可',
      'Pet': '宠物',
      'PreparingInvocation': '准备召唤仪式',
      'Quest': '任务',
      'RankUpFireMages': '晋升为火法师',
      'RankUpGuard': '晋升为卫兵',
      'RanUpFireMagesCompleted': '晋升火法师完成',
      'Realocated': '已迁移',
      'Reason': '原因',
      'Respect': '尊重',
      'ReturnToSC': '返回沼泽营地',
      'RicelordForeman': '稻田主的监工',
      'RideScavenger': '骑乘食尸鸟',
      'Robe': '法袍',
      'Safe': '安全',
      'Scraper': '采矿工',
      'SecondChance': '第二次机会',
      'SecretLocation': '秘密地点',
      'SecretPassage': '秘密通道',
      'SecretPath': '秘密小径',
      'SleeperFollower': '沉睡者信徒',
      'SleeperTemple': '沉睡者神庙',
      'SmallInfo': '小道消息',
      'Stonehenge': '巨石阵',
      'StopFollowing': '停止跟随',
      'SwampCamp': '沼泽营地',
      'Talkative': '健谈',
      'Teach': '教学',
      'TeachBow': '弓术训练',
      'Teacher': '导师',
      'Teacher2': '导师 2',
      'TeacherInscription': '铭文导师',
      'TeacherMana': '魔力导师',
      'TeachIchor': '矿爬虫体液采集训练',
      'TeachMagic': '魔法训练',
      'TeachOrcish': '教授兽人语',
      'TeachStats': '属性训练',
      'TeachWeapon': '武器训练',
      'Teleport': '传送',
      'TheMysteriousOrc': '神秘的兽人',
      'ThroneRoom': '王座厅',
      'TradeBow': '弓箭交易',
      'Trader': '商人',
      'TradeSkins_Trader': '毛皮商人',
      'Traitor': '叛徒',
      'Trial': '试炼',
      'TrollCanyon': '巨魔谷',
      'Trust': '信任',
      'Ulumulu': '乌鲁穆鲁',
      'Unexperienced': '缺乏经验',
      'Uriziel': '尤里泽尔',
      'UrizielRune': '尤里泽尔符文',
      'Useful': '有用',
      'Velaya': '维拉雅',
      'Vibrations': '震动',
      'WaitFreeMine': '在自由矿场等待',
      'WaitInTrainingArea': '在训练场等待',
      'Warning': '警告',
      'WarningTooLate': '迟来的警告',
      'WaterMessenger': '水法师的信使',
      'Weapon': '武器',
      'Who': '身份',
      'Women': '女人们',
      'other': '$fallback',
    });
    return '$_temp0';
  }

  @override
  String get slotRepairTitle => '物品栏槽位损坏';

  @override
  String slotRepairBody(int count) {
    return '此存档中有 $count 个物品栏槽位的 ID 与其位置不再匹配 — 在游戏中丢弃这类物品时，会误删另一件物品。修复仅重写槽位 ID，不会添加、删除或更改任何物品。保存时会照常创建备份。';
  }

  @override
  String get slotRepairQueued => '修复已加入待保存列表 — 保存后生效。';

  @override
  String get slotRepairAction => '修复';

  @override
  String get slotRepairDiscard => '放弃';

  @override
  String get editorInventorySlotEditConflict =>
      '对物品栏槽位的直接编辑与占用整个槽位的操作（修复、添加或删除）同时在待保存列表中。后者会覆盖前者 — 请撤销其中一项后再保存。';

  @override
  String get editorTraderArrayConflict =>
      '一项交易修改与对商人数组的直接编辑一同排队。该编辑会重新编号交易修改所依据的行，因此两者之一会落到错误的商人身上——撤销其中一项后再保存。';

  @override
  String get backupFactFile => '文件';

  @override
  String get renameBackupTooltip => '为此备份命名';

  @override
  String get renameBackupTitle => '备份名称';

  @override
  String get renameBackupLabel => '名称';

  @override
  String renameBackupHelp(String fileName) {
    return '显示在文件名 $fileName 之外。留空则移除名称；文件本身不会被重命名。';
  }

  @override
  String get deleteBackupTooltip => '删除此备份';

  @override
  String get deleteBackupTitle => '删除备份';

  @override
  String deleteBackupBody(String name, String fileName) {
    return '删除“$name”（$fileName）？文件将从磁盘移除且无法恢复。';
  }

  @override
  String get deleteBackupConfirm => '删除';

  @override
  String editorDeletedBackup(String path) {
    return '已删除备份：$path';
  }

  @override
  String editorDeleteBackupFailed(String details) {
    return '无法删除备份：$details';
  }

  @override
  String editorRenameBackupFailed(String details) {
    return '无法为备份命名：$details';
  }

  @override
  String get slotRepairUnavailable => '目前无法修复 — 无法写入此存档。';

  @override
  String editorDeletedBackupWithLabelWarning(String path, String details) {
    return '已删除备份：$path — 但无法移除其名称：$details';
  }

  @override
  String get slotRepairNotOffered => '此存档不支持修复。';

  @override
  String get statisticsTitle => '统计';

  @override
  String get statisticsSubtitle => '角色、任务、世界和游戏进度的简要概览。';

  @override
  String statisticsCardTitle(String card, String fallback) {
    String _temp0 = intl.Intl.selectLogic(card, {
      'timing': '时间',
      'character': '角色',
      'quests': '任务',
      'progress': '进度',
      'encounters': '战斗与交往',
      'inventory': '技能与物品',
      'other': '$fallback',
    });
    return '$_temp0';
  }

  @override
  String statisticsMetric(String metric, String fallback) {
    String _temp0 = intl.Intl.selectLogic(metric, {
      'timePlayed': '游玩时间',
      'worldTime': '世界时间',
      'level': '等级',
      'experience': '经验',
      'learningPoints': '学习点数',
      'guild': '阵营',
      'health': '生命值',
      'mana': '法力',
      'chapter': '章节',
      'location': '位置',
      'kills': '击杀NPC',
      'knownCharacters': '已知角色',
      'killedMonsters': '击杀怪物',
      'defeatedNpcs': '击败NPC',
      'killedNpcs': '击杀NPC',
      'knownNpcs': '已知NPC',
      'knownTeachers': '已知导师',
      'learnedSkills': '已学技能',
      'knowledge': '知识条目',
      'deadCharacters': '死亡角色',
      'traders': '已知商人',
      'inventoryStacks': '物品堆叠',
      'inventoryItems': '物品',
      'ore': '矿石',
      'equipped': '已装备',
      'hostileFactions': '敌对阵营',
      'openCrimes': '未解决罪行',
      'position': '坐标',
      'other': '$fallback',
    });
    return '$_temp0';
  }

  @override
  String statisticsGuildRank(String rank, String fallback) {
    String _temp0 = intl.Intl.selectLogic(rank, {
      'oldCampShadow': '旧营地 · 影子',
      'oldCampGuard': '旧营地 · 守卫',
      'oldCampFireMage': '旧营地 · 火法师',
      'newCampRogue': '新营地 · 强盗',
      'newCampMercenary': '新营地 · 雇佣兵',
      'newCampWaterMage': '新营地 · 水法师',
      'swampCampNovice': '沼泽营地 · 新人',
      'swampCampTemplar': '沼泽营地 · 圣殿骑士',
      'other': '$fallback',
    });
    return '$_temp0';
  }

  @override
  String get statisticsUnknown => '不可用';

  @override
  String get statisticsMore => '更多统计';

  @override
  String statisticsSummary(
    String level,
    String guild,
    String chapter,
    int completed,
    int failed,
    String playTime,
  ) {
    return '等级$level，$guild，第$chapter章。完成任务$completed个，失败$failed个。游玩时间：$playTime。';
  }

  @override
  String get locksSidebar => '锁';

  @override
  String editorLockListFailed(String details) {
    return '无法加载锁列表：$details';
  }

  @override
  String get locksSearchHint => '搜索锁或钥匙';

  @override
  String get locksAllRegions => '所有区域';

  @override
  String locksShownOfTotal(int shown, int total) {
    return '$total 个中的 $shown 个';
  }

  @override
  String get locksFilterChests => '箱子';

  @override
  String get locksFilterDoors => '门';

  @override
  String get locksFilterUnlocked => '已开启';

  @override
  String get locksFilterLocked => '已上锁';

  @override
  String locksDifficultyLevel(int bars, int level) {
    return '难度 $bars/4（内部等级 $level/7）';
  }

  @override
  String get locksKeyOnly => '仅限钥匙';

  @override
  String locksKeyLabel(String keys) {
    return '钥匙：$keys';
  }

  @override
  String get locksPermalocked => '永久封闭';

  @override
  String get locksReadOnly => '此存档没有可编辑的锁列表。';

  @override
  String get locksUnknownEntry => '此游戏版本中不存在';

  @override
  String get locksDoorLeafHint => '重新锁上门时，门本身也会关闭。';

  @override
  String get locksResetPending => '放弃待保存的锁改动';
}

/// The translations for Chinese, using the Han script (`zh_Hant`).
class AppLocalizationsZhHant extends AppLocalizationsZh {
  AppLocalizationsZhHant() : super('zh_Hant');

  @override
  String get debugSectionTitle => '高級（調試）';

  @override
  String get debugSectionSubtitle => '用於錯誤報告的診斷和原始資料';

  @override
  String get showObjectIdsTitle => '顯示其他技術 ID';

  @override
  String get showObjectIdsSubtitle => '顯示物品、對話知識、任務和孤立角色的技術 ID。NPC ID 始終顯示。';

  @override
  String get storyStateSidebar => '劇情狀態';

  @override
  String get storyStateDescription =>
      '遊戲在這裡記錄任務、對話和事件的進度。“已儲存”顯示存檔中的數值，“未設定”顯示其他已知條目。根據條目的不同，數字可能表示是或否、次數或進度階段。時間標記表示遊戲內的日期和時間。';

  @override
  String get storyStateReadOnly =>
      '在確認腳本含義和安全的映射寫入方式前保持只讀。關聯的詞條文本僅提供上下文，並非技術 ID 的直接翻譯。';

  @override
  String get storyStateStructureReadOnly =>
      '無法唯一且安全地確定此存檔中的 StoryPropertyValues 結構。此存檔的劇情值將保持只讀。';

  @override
  String get storyStateSearch => '搜尋劇情狀態';

  @override
  String storyStateValuesCount(int shown, int total) {
    return '已顯示 $shown 個，共 $total 個劇情值';
  }

  @override
  String get storyStateInteger => '整數';

  @override
  String get storyStateTimeMarker => '時間標記';

  @override
  String get storyStateChapter => '章節';

  @override
  String get storyStateUnknown => '未知源碼類型';

  @override
  String storyStateShowDormant(int count) {
    return '顯示未使用（$count）';
  }

  @override
  String get storyStateUnknownDetail =>
      '當前腳本目錄中沒有此已儲存 ID（例如來自模組或更新的遊戲版本）。其存檔線值為 int32，但不會推斷其含義。';

  @override
  String get storyStateStored => '已儲存';

  @override
  String get storyStateUnset => '未設定';

  @override
  String get storyStateUnsetDetail => '此目錄字段未序列化到該存檔中，因此遊戲會使用未設定或預設狀態。';

  @override
  String get storyStateRawValue => '原始值';

  @override
  String storyStateElapsed(String duration) {
    return '儲存時已過去：$duration';
  }

  @override
  String storyStateAhead(String duration) {
    return '儲存時尚在未來：$duration';
  }

  @override
  String storyStateDurationDays(int days, String time) {
    String _temp0 = intl.Intl.pluralLogic(
      days,
      locale: localeName,
      other: '$days 天',
      one: '1 天',
    );
    return '$_temp0 $time';
  }

  @override
  String get storyStateRelatedGlossary => '關聯詞條';

  @override
  String get storyStateTechnicalPath => '技術路徑';

  @override
  String get storyStateEditingGuidance =>
      '選擇條目即可修改數值，修改會在儲存後生效。請只修改你瞭解其作用的數值，否則任務或對話可能無法按預期進行。儲存時會自動建立備份。';

  @override
  String get storyStatePending => '待處理';

  @override
  String storyStatePendingValue(String value) {
    return '將儲存為 $value';
  }

  @override
  String get storyStatePendingRemoval => '將從存檔中移除';

  @override
  String get storyStateEditValue => '編輯值';

  @override
  String get storyStateSetValue => '設定值';

  @override
  String get storyStateRemoveValue => '從存檔中移除';

  @override
  String get storyStateUndoChange => '撤銷劇情更改';

  @override
  String get storyStateResetChanges => '重置劇情更改';

  @override
  String storyStateDialogTitle(String id) {
    return '編輯 $id';
  }

  @override
  String get storyStateRawInput => '有符號 int32 值';

  @override
  String get storyStateInvalidInt32 => '請輸入 -2147483648 到 2147483647 之間的整數。';

  @override
  String get storyStateQueueChange => '將更改加入佇列';

  @override
  String storyStateSuggestedValues(String values) {
    return '隨遊戲提供的腳本中已確認的值：$values';
  }

  @override
  String get storyStateSuggestionsNotLimits =>
      '建議值不是驗證限制；原生程式碼、模組或後續遊戲版本可能會使用其他值。';

  @override
  String get storyStateUseCurrentTime => '使用當前存檔時間';

  @override
  String get storyStateStructuredTime => '天數 / 時間';

  @override
  String get storyStateRawMode => '原始 int32';

  @override
  String get storyStateChapterWarning => '僅更改章節不會同步任務、NPC、物品欄或世界狀態。';

  @override
  String get storyStateDormantWarning =>
      '在隨遊戲提供的腳本快取中未找到對此字段的有效讀取或寫入。它可能是舊字段、由原生程式碼控制，或為保留字段。';

  @override
  String get storyStateReadOnlySourceWarning =>
      '隨遊戲提供的腳本會讀取此字段，但沒有通過腳本寫入。它仍可能由原生程式碼管理。';

  @override
  String get storyStateUnknownEditWarning =>
      '這個來自模組或後續版本的 ID 沒有內置的源碼語義。請僅編輯其原始 int32 值。';

  @override
  String storyStateIntegerKind(String kind) {
    String _temp0 = intl.Intl.selectLogic(kind, {
      'binaryFlag': '二進位標誌',
      'finiteState': '多狀態值',
      'counterOrScore': '計數器 / 分數',
      'calendarDay': '日曆日',
      'derivedOrOpaqueInteger': '派生 / 不透明整數',
      'readOnlyInSourceInteger': '隨附腳本中只讀',
      'dormantOrLegacyInteger': '隨附腳本中未使用',
      'other': '整數',
    });
    return '$_temp0';
  }

  @override
  String get storyStateZeroVsUnset =>
      '已儲存的 0 與映射中沒有該條目是兩種不同的文件狀態。“從存檔中移除”會恢復構造函式或預設狀態。';

  @override
  String get appTitle => 'GORE Save Editor';

  @override
  String get appLogoSemanticLabel => 'GORE Save Editor 徽標';

  @override
  String get zoomTooltip => '按 Ctrl +/- 放大/縮小';

  @override
  String get switchToLightMode => '切換到淺色模式';

  @override
  String get switchToDarkMode => '切換到深色模式';

  @override
  String get about => '關於';

  @override
  String get tabOverview => '概覽';

  @override
  String get tabPlayer => '玩家';

  @override
  String get tabAttribute => '屬性';

  @override
  String get heroGroupSkills => '技能';

  @override
  String get skillsNoneBody => '未找到該角色的技能。';

  @override
  String get skillsUnavailableBody => '此存檔無法編輯技能——主角沒有可修改的效果資料。';

  @override
  String get skillNotLearned => '未習得';

  @override
  String get skillLearn => '學習';

  @override
  String get skillActionLearn => '學習';

  @override
  String get skillActionUnlearn => '遺忘';

  @override
  String get skillTierUntrained => '未受過訓練';

  @override
  String get skillTierBeginner => '初學者';

  @override
  String get skillTierTrained => '訓練有素';

  @override
  String get skillTierMaster => '大師級';

  @override
  String get skillTierNovice => '熟練';

  @override
  String get skillTierAmateur => '業餘（第0環）';

  @override
  String get skillTierLearned => '已習得';

  @override
  String skillTierCircle(int n) {
    return '第$n環';
  }

  @override
  String get skillHintBlacksmith1H => '單手武器';

  @override
  String get skillHintBlacksmith2H => '雙手武器';

  @override
  String get skillScutesTrained => '熟練（骨板）';

  @override
  String get skillScutesMaster => '大師（＋剃刀獸角板）';

  @override
  String get skillCategoryCombat => '戰鬥';

  @override
  String get skillCategoryCrafting => '製作';

  @override
  String get skillCategoryHunting => '狩獵';

  @override
  String get skillCategoryLanguage => '語言';

  @override
  String get skillCategoryMagic => '魔法';

  @override
  String get skillCategoryMovement => '移動';

  @override
  String get skillCategoryThievery => '盜竊';

  @override
  String get skillCategoryOther => '其他';

  @override
  String get skillNameOneHanded => '單手';

  @override
  String get skillNameTwoHanded => '雙手';

  @override
  String get skillNameFists => '赤手空拳';

  @override
  String get skillNameBow => '弓';

  @override
  String get skillNameCrossbow => '弩';

  @override
  String get skillNameLockpicking => '開鎖';

  @override
  String get skillNamePickpocketing => '扒竊';

  @override
  String get skillNameTakeOrgans => '摘取內臟';

  @override
  String get skillNameBreakTeeth => '摘取牙齒';

  @override
  String get skillNameTakeClaws => '摘取爪子';

  @override
  String get skillNameSkinFur => '拿取毛皮';

  @override
  String get skillNameSkin => '拿取皮膚';

  @override
  String get skillNameTakeFins => '拿取鰭';

  @override
  String get skillNameTakeStingers => '摘取刺';

  @override
  String get skillNameTakeSecretion => '摘取分泌物';

  @override
  String get skillNameTakeSkullPlates => '拿取頭骨甲';

  @override
  String get skillNameSkinSwampshark => '拿取鯊魚皮';

  @override
  String get skillNameTakeMinecrawlerPlates => '拿取護甲板';

  @override
  String get skillNameTakeScutes => '拿取鱗甲';

  @override
  String get skillNameTakeUluMulu => '拿取烏魯木魯';

  @override
  String get skillNameOrcWeapons => '獸人武器';

  @override
  String get skillNameMining => '採礦';

  @override
  String get skillNameDiving => '潛水';

  @override
  String get skillNameTakeMinecrawlerMandibles => '摘取下頜';

  @override
  String get skillNameTakeShadowbeastHorn => '拿取角 (Shadowbeast)';

  @override
  String get skillNameTakeSpines => '摘取脊柱';

  @override
  String get skillNameBreakSwampsharkTeeth => '摘取鯊魚牙';

  @override
  String get skillNameTakeFireTongue => '拿取火蜥蜴的舌';

  @override
  String get skillNameTakeTrollHorn => '拿取角 (Troll)';

  @override
  String get skillNameAcrobatics => '雜技';

  @override
  String get skillNameWallClimbing => '攀登';

  @override
  String get skillNameRiding => '騎乘食屍鳥';

  @override
  String get skillNameSneaking => '潛行';

  @override
  String get skillNameAlchemy => '鍊金術';

  @override
  String get skillNameRuneInscription => '銘刻';

  @override
  String get skillNameBlacksmithing => '鍛造';

  @override
  String get skillNameMagicCircle => '魔法環';

  @override
  String get skillNameOrcish => '獸人語';

  @override
  String get tabInventory => '物品欄';

  @override
  String get tabTrade => '交易';

  @override
  String get traderNotAMerchant => '該角色不進行交易。';

  @override
  String get traderRetry => '重試';

  @override
  String get traderAmbiguousName => '有多條商人記錄使用這個名字，無法判斷哪家店屬於該角色。已禁用編輯，以免改錯。';

  @override
  String get traderOre => '礦石（購買力）';

  @override
  String get traderNoOre => '無礦石';

  @override
  String get traderStockCurrent => '已儲存庫存';

  @override
  String get traderStockCurrentTooltip => '目前為該商人儲存的庫存。遊戲下次更新商人時，添加的物品可能會消失。';

  @override
  String get traderStockBase => '參考庫存';

  @override
  String get traderStockBaseTooltip =>
      '這是已儲存的庫存副本，遊戲可按該商人的規則更改或重新生成。這裡只讀顯示，添加的物品不會永久保留。';

  @override
  String get traderStockBaseHint => '只讀。此庫存會隨劇情推進而增加，也可能按商人規則被替換。它不是遊戲最初的庫存。';

  @override
  String get traderCurrentStockWarning => '商人庫存的更改只會保留到下次補貨。';

  @override
  String get traderRestockTitle => '預計補貨時間';

  @override
  String get traderRestockTitleTooltip => '根據商人的上次活動、遊戲時間和資源難度估算。';

  @override
  String get traderRestockPending => '待處理';

  @override
  String get traderRestockRevertTooltip => '撤銷尚未儲存的上次活動更改';

  @override
  String get traderRestockNever => '從未';

  @override
  String get traderRestockUnavailable => '不可用';

  @override
  String get traderRestockIntervalUnknown => '遊戲天數未知';

  @override
  String get traderRestockNeverStatus => '尚未記錄該商人的活動。';

  @override
  String get traderRestockClockAhead => '商人的上次活動晚於當前遊戲時間。';

  @override
  String traderRestockNotDueYet(String time) {
    return '預計不會早於 $time。';
  }

  @override
  String get traderRestockPossiblyDue => '估算：庫存可能已經可以更新。';

  @override
  String get traderRestockEligible => '估算：現在應該補貨。';

  @override
  String get traderRestockNoWorldTime => '當前遊戲時間不可用，因此無法估算補貨時間。';

  @override
  String get traderRestockLastActivity => '上次商人活動';

  @override
  String get traderRestockLastActivityTooltip =>
      '此儲存時間可能會在交易後或遊戲更新庫存時改變。它不一定就是上次補貨的時間。';

  @override
  String get traderRestockForecastWindow => '預計時間';

  @override
  String get traderRestockForecastWindowTooltip =>
      '顯示最早和最晚可能補貨的時間。存檔中沒有遊戲的確切規則，因此這只是估算。';

  @override
  String get traderRestockIntervalLabel => '兩次補貨之間的天數';

  @override
  String traderRestockInterval(int days, String level) {
    return '$days 天 · $level';
  }

  @override
  String get traderRestockIntervalTooltip =>
      '按資源難度計算：新手 2 天、Gothic 3 天、困難 5 個遊戲日。';

  @override
  String get traderRestockAutomationLabel => '自動補貨';

  @override
  String get traderRestockAutomationValue => '無法在存檔中禁用';

  @override
  String get traderRestockAutomationTooltip => '無法在存檔中關閉自動補貨。只有模組能改變這項遊戲規則。';

  @override
  String get traderRestockSetNow => '設為遊戲時間';

  @override
  String get traderRestockSetNowTooltip =>
      '將當前遊戲時間（包括尚未儲存的更改）設為商人的上次活動。這會推遲預計補貨時間。';

  @override
  String get traderRestockMakeDue => '準備補貨';

  @override
  String get traderRestockMakeDueTooltip => '將上次活動向過去調整，使其達到預計補貨時間。';

  @override
  String get traderRestockCustom => '自定義時間…';

  @override
  String get traderRestockCustomTooltip => '為商人的上次活動選擇遊戲內日期和時間。';

  @override
  String get traderRestockEditTitle => '商人的上次活動';

  @override
  String get traderOreHint =>
      '遊戲內的數值會不同：載入時遊戲會加上自他上次交易以來累積的部分——他會賣掉多餘貨物並以此補貨。這個數字是起點，而非交易介面顯示的金額。';

  @override
  String get traderOreHintShort => '初始值——可能與交易介面中的金額不同。';

  @override
  String get traderRestockStatusLabel => '狀態';

  @override
  String get traderRestockStatusNever => '無活動';

  @override
  String get traderRestockStatusWaiting => '等待補貨';

  @override
  String get traderRestockStatusReady => '可以補貨';

  @override
  String get traderRestockStatusPossiblyReady => '可能可以補貨';

  @override
  String get traderRestockStatusCheckTime => '檢查儲存時間';

  @override
  String get traderRestockStatusUnknown => '未知';

  @override
  String get traderPriceWarning => '價格會隨商人的庫存量和持有礦石而變化，因此修改這些數字也可能改變他的開價。';

  @override
  String get traderAddItem => '添加物品';

  @override
  String get traderRemoveItem => '移除條目';

  @override
  String get traderReadOnlyCore => '此核心版本只能讀取商人資料。';

  @override
  String get traderDifficultyStockUnsupported =>
      '該商人擁有按難度區分的庫存，編輯器並未建模。此處已禁用編輯，因為修改看似成功，卻會讓這部分額外庫存原封不動。';

  @override
  String get traderRecordIncomplete =>
      '該商人的庫存清單缺失，或其結構編輯器不支持、無法寫入。此處已禁用編輯，以免修改在儲存時失敗。';

  @override
  String get traderEmptyStock => '沒有庫存。';

  @override
  String get traderUnknownItem => '不在物品目錄中';

  @override
  String editorTradersLoadFailed(String details) {
    return '商人資料載入失敗：$details';
  }

  @override
  String traderStockLineCount(int count) {
    return '$count 件商品';
  }

  @override
  String get tabWorld => '世界';

  @override
  String get tabCharacters => '角色';

  @override
  String get characterNoActorBody => '該角色在世界中沒有對應的實體，因此沒有屬性、物品欄或事件。';

  @override
  String get characterNoEventsBody => '該角色沒有事件。';

  @override
  String get characterOrphanGroup => '其他';

  @override
  String get tabAllData => '全部資料';

  @override
  String get tabBackups => '備份';

  @override
  String get tabSettings => '設定';

  @override
  String get reset => '重置';

  @override
  String get save => '儲存';

  @override
  String saveWithCount(int count) {
    return '儲存（$count）';
  }

  @override
  String get ok => '確定';

  @override
  String get cancel => '取消';

  @override
  String get confirm => '確認';

  @override
  String get close => '關閉';

  @override
  String get add => '添加';

  @override
  String get equippedBadge => '已裝備';

  @override
  String get armorUpgradesLabel => '升級';

  @override
  String get browse => '瀏覽';

  @override
  String get noSavFilesFound => '未找到 .sav 文件';

  @override
  String get profile => '存檔配置';

  @override
  String get otherSaves => '其他存檔';

  @override
  String profileWithSaves(String name, int count) {
    return '$name（$count 個存檔）';
  }

  @override
  String get switchProfile => '切換存檔配置';

  @override
  String get openSaveFile => '打開文件';

  @override
  String get externalSave => '從外部打開的存檔';

  @override
  String get saveProfileTitle => '存檔配置';

  @override
  String get saveProfileDescription => '將此存檔分配給另一個遊戲存檔配置。存檔和存檔配置索引將一同備份。';

  @override
  String get saveProfileExternalHint => '選擇一個存檔配置，將此文件導入遊戲存檔資料夾並在其中登記。原文件不會更改。';

  @override
  String get saveProfileNoProfiles =>
      '在 PersistentDataList.sav 中未找到可編輯的遊戲存檔配置。';

  @override
  String get saveProfileSelect => '選擇存檔配置';

  @override
  String get rescanSaveFolder => '重新掃描存檔資料夾';

  @override
  String get discardUnsavedChangesTitle => '放棄未儲存的更改？';

  @override
  String rescanDiscardBody(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '更改',
      one: '更改',
    );
    return '重新掃描將重新載入每個存檔，並放棄你 $count 項未儲存的$_temp0。';
  }

  @override
  String get discardAndRescan => '放棄並重新掃描';

  @override
  String chapterLabel(Object id) {
    return '第 $id 章';
  }

  @override
  String get quickSave => '快速存檔';

  @override
  String get autoSave => '自動存檔';

  @override
  String get manualSave => '手動存檔';

  @override
  String get errorTitle => '錯誤';

  @override
  String get selectASaveTitle => '選擇存檔';

  @override
  String get selectASaveBody => '存檔詳情將顯示在此處。';

  @override
  String bytesValue(String count) {
    return '$count 位元組';
  }

  @override
  String get inspectionJsonTitle => '檢查 JSON';

  @override
  String get copy => '複製';

  @override
  String get savegameFallbackTitle => '存檔';

  @override
  String screenshotForSlot(String slot) {
    return '$slot 的截圖';
  }

  @override
  String get publicSaveName => '名稱';

  @override
  String get gameTimeTitle => '遊戲時間';

  @override
  String get gameTimeDay => '天';

  @override
  String get gameTimeHours => '小時';

  @override
  String get gameTimeMinutes => '分鐘';

  @override
  String get gameTimeSeconds => '秒';

  @override
  String gameTimeTotal(int seconds) {
    return '= 共 $seconds 秒';
  }

  @override
  String get gameTimeInvalid => '請輸入整數：天數 ≥ 0，小時 0–23，分鐘和秒數 0–59。';

  @override
  String get required => '必填';

  @override
  String get playerLockedBody => '編輯私有玩家資料需要支持壓縮的編解碼器。';

  @override
  String get heroTransform => '位置';

  @override
  String get locationX => '位置 X';

  @override
  String get locationY => '位置 Y';

  @override
  String get locationZ => '位置 Z';

  @override
  String get rotationPitch => '旋轉俯仰';

  @override
  String get rotationYaw => '旋轉偏航';

  @override
  String get rotationRoll => '旋轉翻滾';

  @override
  String get spawnPositionSection => '出生位置（參考）';

  @override
  String get resetToSpawnPosition => '重置為出生位置';

  @override
  String get positionOutOfRange => '數值必須介於 −10,000,000 與 10,000,000 之間';

  @override
  String get positionNotEditable => '無法讀取該角色已儲存的位置，因此無法編輯。';

  @override
  String get positionNeverPlaced => '該角色從未在世界中放置過（位置 0, 0, 0）——遊戲可能會忽略已儲存的位置。';

  @override
  String get npcStayInPlace => '停用他的日常作息';

  @override
  String get npcStayInPlaceHint => '他會留在原地。';

  @override
  String get npcStayInPlaceLocked => '他原本的日常作息沒有被記錄下來，因此無法再還原。';

  @override
  String get npcUndoPlacement => '撤銷這次移動';

  @override
  String get npcUndoPlacementStale => '存檔已不再是這次移動當時寫入的樣子，還原會丟棄此後發生的變化。';

  @override
  String get positionNotReadable => '無法讀取該角色已儲存的位置。';

  @override
  String get npcPositionReadOnly => '遊戲會從關卡而非存檔中恢復 NPC 的位置，因此這些數值可以查看，但無法修改。';

  @override
  String get pickLocation => '選擇地點…';

  @override
  String get pickLocationDialogTitle => '選擇地點';

  @override
  String get applySpotRotation => '同時應用該地點的朝向';

  @override
  String get locationAreaOther => '其他';

  @override
  String get locationAreaCavalornValley => '卡瓦隆山谷';

  @override
  String get locationAreaEastForest => '東部森林';

  @override
  String get locationAreaFogTower => '霧塔';

  @override
  String get locationAreaIllegalWeedMixers => '非法沼澤菸草調配者';

  @override
  String get locationAreaOrcArena => '獸人競技場';

  @override
  String get locationAreaOrcGraveyard => '獸人墓地';

  @override
  String get locationAreaShipwreck => '沉船殘骸';

  @override
  String get locationAreaTundra => '苔原';

  @override
  String get locationCatalogUnavailable => '無法載入地點目錄。';

  @override
  String get invalid => '無效';

  @override
  String get heroAttributes => '主角屬性';

  @override
  String attributeBase(String name) {
    return '$name 基礎值';
  }

  @override
  String attributeCurrent(String name) {
    return '$name 當前值';
  }

  @override
  String get attributeBaseValue => '基礎值';

  @override
  String get attributeCurrentValue => '當前值';

  @override
  String get inventoryTitle => '物品欄';

  @override
  String get inventoryEmpty => '此物品欄為空。';

  @override
  String get inventoryNeedsDecoded => '編輯物品欄需要來自編解碼器的已解碼私有負載資料。';

  @override
  String get inventoryNoStacks => '已解碼的私有負載中未找到物品堆疊。';

  @override
  String get resetInventoryChanges => '重置物品欄更改';

  @override
  String get addItemTooltipPendingAdd => '請先儲存待處理的更改 — 每次儲存只能添加一件新物品';

  @override
  String get addItemTooltipPendingRemove => '請先儲存待處理的移除 — 每次儲存只能進行一項結構更改';

  @override
  String get addItemTooltipPendingCount => '請先儲存或重置待處理的數量更改 — 結構編輯必須單獨儲存';

  @override
  String get addItemTooltipDefault => '向物品欄添加物品';

  @override
  String get addItemButton => '添加物品';

  @override
  String get resetInventoryButton => '重置物品欄';

  @override
  String get resetInventoryTooltipDefault => '將此物品欄替換為遊戲開始時的物品欄';

  @override
  String get resetInventoryTooltipBlocked => '請先儲存或取消待處理的物品欄更改';

  @override
  String get pendingResetTitle => '重置為遊戲開始時的物品欄';

  @override
  String pendingResetSubtitle(String level) {
    return '資源等級：$level';
  }

  @override
  String get cancelPendingReset => '取消重置';

  @override
  String pendingAddSubtitle(int count) {
    return '×$count — 待添加（尚未儲存）';
  }

  @override
  String get cancelPendingAdd => '取消待添加';

  @override
  String get pendingRemovalSubtitle => '待移除（尚未儲存）';

  @override
  String get cancelPendingRemoval => '取消待移除';

  @override
  String get filterItems => '篩選物品';

  @override
  String noItemsMatchQuery(String query) {
    return '沒有物品匹配“$query”。';
  }

  @override
  String get pendingRemovalHidesAll => '待處理的移除隱藏了所有物品 — 請儲存以應用。';

  @override
  String categoryWithCount(String label, int count) {
    return '$label（$count）';
  }

  @override
  String get itemTooltipIngredientFor => '用於製作';

  @override
  String itemTooltipTeaches(String item) {
    return '傳授: $item';
  }

  @override
  String get itemTooltipValue => '價值';

  @override
  String get itemTooltipProtection => '防禦';

  @override
  String get itemTooltipRequirements => '需求：';

  @override
  String get itemTooltipManaCost => '法力消耗';

  @override
  String get itemTooltipManaUpkeep => '蓄力法力消耗';

  @override
  String get itemCategoryAll => '全部';

  @override
  String get itemCategoryMeleeWeapon => '近戰武器';

  @override
  String get itemCategoryRangedWeapon => '遠程武器';

  @override
  String get itemCategoryMagic => '魔法';

  @override
  String get itemCategoryWearable => '穿戴裝備';

  @override
  String get itemCategoryFood => '食物';

  @override
  String get itemCategoryPotion => '藥水';

  @override
  String get itemCategoryMaterial => '材料';

  @override
  String get itemCategoryDocument => '文件';

  @override
  String get itemCategoryMisc => '雜項';

  @override
  String get itemCategoryArtefact => '神器';

  @override
  String get itemCategoryOther => '其他';

  @override
  String get count => '數量';

  @override
  String get min1 => '最少 1';

  @override
  String countTimes(String count) {
    return '×$count';
  }

  @override
  String get deleteEquippedTooltip => '無法刪除：該物品可能已裝備或已分配到快捷鍵槽位';

  @override
  String get removeBlockedTooltip => '請先儲存或重置待處理的物品欄更改 — 添加或移除必須單獨儲存';

  @override
  String get removeItemFromInventory => '從物品欄移除物品';

  @override
  String get progressionLockedBody => '進度資料需要來自編解碼器的已解碼私有負載資料。';

  @override
  String get progressionNeedsTyped => '結構化進度資料需要完全解碼且已驗證類型解析的存檔。';

  @override
  String get sectionQuests => '任務';

  @override
  String get sectionKnowledge => '知識';

  @override
  String get sectionEvents => '事件';

  @override
  String get firstPage => '首頁';

  @override
  String get previousPage => '上一頁';

  @override
  String get nextPage => '下一頁';

  @override
  String get lastPage => '末頁';

  @override
  String pageOfPages(int page, int total) {
    return '第 $page / $total 頁';
  }

  @override
  String rangeOfTotal(int first, int last, int total) {
    return '$first–$last，共 $total';
  }

  @override
  String get perPage => '每頁：';

  @override
  String get resetQuestChanges => '重置任務更改';

  @override
  String get searchQuests => '搜尋任務';

  @override
  String get allGroups => '所有分組';

  @override
  String groupWithCount(String group, Object count) {
    return '$group（$count）';
  }

  @override
  String stateLabelWithCount(String label, int count) {
    return '$label $count';
  }

  @override
  String get questStateNone => '無';

  @override
  String get questStateAvailable => '可接取';

  @override
  String get questStateRunning => '進行中';

  @override
  String get questStateSucceeded => '已完成';

  @override
  String get questStateFailed => '已失敗';

  @override
  String get questStateUnknown => '未知';

  @override
  String get dialogKnowledge => '對話知識';

  @override
  String get resetKnowledgeChanges => '重置知識更改';

  @override
  String get addNpc => '添加 NPC';

  @override
  String get searchNpcs => '搜尋 NPC';

  @override
  String get npcStatusRowLabel => '狀態';

  @override
  String get npcStatusAlive => '存活';

  @override
  String get npcStatusDead => '已死亡';

  @override
  String get npcRelationshipRowLabel => '關係';

  @override
  String get npcRelationshipUnavailable => '關係狀態不可用';

  @override
  String get npcRelationshipAutomatic => '由遊戲計算';

  @override
  String get npcRelationshipAutomaticHint => '未儲存永久覆蓋。遊戲會評估公會、劇情、區域和犯罪規則。';

  @override
  String get npcRelationshipStoredHint =>
      '已儲存為 NPC 對玩家的永久覆蓋。公會、劇情、區域和犯罪規則仍可能改變遊戲中的實際關係。';

  @override
  String get npcRelationshipFriend => '友好';

  @override
  String get npcRelationshipNeutral => '中立';

  @override
  String get npcRelationshipEnemy => '敵人';

  @override
  String npcRelationshipPending(String relationship) {
    return '儲存後將為$relationship';
  }

  @override
  String npcStateHp(String hp, String maxHp) {
    return 'HP $hp / $maxHp';
  }

  @override
  String get npcReviveButton => '復活';

  @override
  String get npcReviveQueued => '將在儲存時復活';

  @override
  String entriesForCharacter(String name) {
    return '條目 — $name';
  }

  @override
  String get selectNpcToSeeEntries => '選擇一個 NPC 以查看條目';

  @override
  String get addKnowledgeEntry => '添加知識條目';

  @override
  String get browseCatalog => '瀏覽目錄';

  @override
  String get alreadyExistsForCharacter => '該角色已存在此項。';

  @override
  String get alreadyInPendingChanges => '已在待處理的更改中。';

  @override
  String duplicateCheckFailed(String error) {
    return '重複檢查失敗 — 請重試：$error';
  }

  @override
  String pendingAddsCount(int count) {
    return '待添加（$count）';
  }

  @override
  String get undoAdd => '撤銷添加';

  @override
  String get undoRemove => '撤銷移除';

  @override
  String get removeEntry => '移除條目';

  @override
  String get selectNpcFromList => '從列表中選擇一個 NPC';

  @override
  String characterWithCount(String name, int count) {
    return '$name（$count）';
  }

  @override
  String get memoryEvents => '記憶事件';

  @override
  String get searchCharacters => '搜尋角色';

  @override
  String eventsForCharacter(String name) {
    return '事件 — $name';
  }

  @override
  String get selectCharacterToSeeEvents => '選擇一個角色以查看事件';

  @override
  String get noTags => '（無標籤）';

  @override
  String eventSubtitle(String time, String affected) {
    return 't=${time}s  $affected';
  }

  @override
  String get removeEvent => '移除事件';

  @override
  String get removeMemoryEventTitle => '移除記憶事件？';

  @override
  String get removeMemoryEventBody => '將此記憶事件加入移除佇列？存檔只在按下“儲存”後才會變更。';

  @override
  String get memoryEventRemovalQueued => '事件移除已加入佇列 — 按“儲存”以應用。';

  @override
  String get duplicateEvent => '複製事件';

  @override
  String get duplicateMemoryEventTitle => '複製記憶事件？';

  @override
  String get duplicateMemoryEventBody => '將此記憶事件的複本加入佇列？存檔只在按下“儲存”後才會變更。';

  @override
  String get memoryEventDuplicationQueued => '事件複製已加入佇列 — 按“儲存”以應用。';

  @override
  String get selectCharacterFromList => '從列表中選擇一個角色';

  @override
  String get factionsSidebar => '陣營';

  @override
  String get factionsForgiveButton => '寬恕';

  @override
  String get factionHostile => '敵對';

  @override
  String get factionFriendly => '友好';

  @override
  String crimeMurder(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count 起謀殺',
    );
    return '$_temp0';
  }

  @override
  String crimeAssault(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count 起襲擊',
    );
    return '$_temp0';
  }

  @override
  String crimeTheft(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count 起盜竊',
    );
    return '$_temp0';
  }

  @override
  String crimeTrespassing(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count 起擅闖',
    );
    return '$_temp0';
  }

  @override
  String crimeThreat(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count 起威脅',
    );
    return '$_temp0';
  }

  @override
  String crimeOther(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count 起其他罪行',
    );
    return '$_temp0';
  }

  @override
  String get factionsForgiveQueued => '寬恕中…';

  @override
  String get factionsEmpty => '沒有針對陣營的未了罪行。';

  @override
  String get factionGuildOldCamp => '舊營地';

  @override
  String get factionGuildNewCamp => '新營地';

  @override
  String get factionGuildSwampCamp => '沼澤營地';

  @override
  String get factionGuildOther => '其他/個人';

  @override
  String get allDataLockedBody => '目前，完整的資料源瀏覽器可用於 GSAV 存檔檔案。';

  @override
  String get allDataDescription =>
      '瀏覽 GSAV 中繼資料以及 PUBLIC/PRIVATE 中的所有類型化節點。可安全修改的標量值和原生結構體值均可編輯；容器和未解析的位元組資料也會顯示。';

  @override
  String get allDataEditable => '可編輯';

  @override
  String get allDataReadOnly => '只讀';

  @override
  String get allDataType => '類型';

  @override
  String get allDataScalars => '標量';

  @override
  String get allDataStructs => '結構體';

  @override
  String get allDataContainers => '容器';

  @override
  String get allDataOpaque => '未解析資料';

  @override
  String get allDataNodes => '節點';

  @override
  String allDataChildren(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count 個子節點',
      one: '1 個子節點',
    );
    return '$_temp0';
  }

  @override
  String get allDataPending => '待儲存';

  @override
  String get allDataTagInputHint => '用逗號或換行分隔標籤';

  @override
  String allDataTypedSource(String source) {
    return '$source 類型化資料';
  }

  @override
  String get searchPropertiesLabel => '搜尋屬性（留空 = 列出全部） — 例如 Health、GameTime';

  @override
  String get decodingSaveTitle => '正在解碼存檔…';

  @override
  String get decodingSaveBody => '正在為首次搜尋解碼完整的私有負載。此操作每個存檔只運行一次，之後的搜尋將立即完成。';

  @override
  String get searchTheSaveTitle => '搜尋存檔';

  @override
  String get searchTheSaveBody => '輸入屬性名稱並按回車鍵。留空則列出全部。';

  @override
  String get searchFailedTitle => '搜尋失敗';

  @override
  String get noMatchesTitle => '無匹配項';

  @override
  String get noMatchesBody => '沒有屬性路徑包含所有這些詞條。';

  @override
  String get value => '值';

  @override
  String get backupsTitle => '備份';

  @override
  String get refreshBackups => '刷新備份';

  @override
  String get noBackupsTitle => '無備份';

  @override
  String get noBackupsBody => '編輯存檔時會在所選槽位旁建立備份文件。';

  @override
  String get slotBackups => '槽位備份';

  @override
  String get profileBackups => '存檔配置備份';

  @override
  String get backupFactName => '名稱';

  @override
  String get backupFactSlot => '槽位';

  @override
  String get backupFactCreated => '建立時間';

  @override
  String get backupFactSize => '大小';

  @override
  String get backupFactStatus => '狀態';

  @override
  String get backupFactSha1 => 'SHA-1';

  @override
  String restoreBackupTooltip(String fileName) {
    return '恢復 $fileName';
  }

  @override
  String get appearanceTitle => '外觀';

  @override
  String get uiFont => '字體';

  @override
  String get theme => '主題';

  @override
  String get themeLight => '淺色';

  @override
  String get themeDark => '深色';

  @override
  String get themeSystem => '跟隨系統';

  @override
  String get uiScale => '介面縮放';

  @override
  String get resetZoomTooltip => '重置縮放（Ctrl+0）';

  @override
  String get zoomTip => '提示：在應用內任意位置按 Ctrl + / Ctrl - 均可調整縮放。';

  @override
  String get language => '介面';

  @override
  String get gameTextLanguage => '遊戲文字';

  @override
  String get gameTextLanguageHint => '選擇介面語言時，也會選定對應的遊戲文字。之後可以單獨更改遊戲文字。';

  @override
  String get updatesTitle => '更新';

  @override
  String get checkForUpdatesAutomatically => '自動檢查更新';

  @override
  String get checkForUpdatesNow => '立即檢查更新';

  @override
  String get updatesPortableNotice => '便攜版會在瀏覽器中打開下載頁面。請用新下載的文件替換現有文件。';

  @override
  String get updateAvailableTitle => '有可用更新';

  @override
  String updateAvailableMessage(Object version, Object current) {
    return '版本 $version 可用。您當前為 $current。';
  }

  @override
  String get updateDownload => '下載';

  @override
  String updateOpenFailed(String url) {
    return '無法打開下載頁面。你可以通過 $url 訪問。';
  }

  @override
  String get updateLater => '稍後';

  @override
  String get updateUpToDate => '您正在使用最新版本。';

  @override
  String get updateCheckFailed => '無法檢查更新，請稍後重試。';

  @override
  String get gameTextTitle => '遊戲文本';

  @override
  String get itemImagesTitle => '物品圖片';

  @override
  String get gameDataTitle => '遊戲資料';

  @override
  String itemImagesReady(int count) {
    return '已準備 $count 張物品圖片。';
  }

  @override
  String get itemImagesUnavailable => '物品圖片不可用，將改用分類圖標。';

  @override
  String get checkRefreshItemImages => '檢查 / 更新物品圖片';

  @override
  String get gameDataSourceMissing => '無法自動準備遊戲文本。你可以在設定中選擇本地化快取。';

  @override
  String get loadingTexts => '正在載入文本…';

  @override
  String get loadingImages => '正在載入圖片…';

  @override
  String get preparing => '正在準備…';

  @override
  String gameTextExtractedWithCounts(int ids, int languages) {
    return '已提取：$languages 種語言共 $ids 個 ID。';
  }

  @override
  String get gameTextExtracted => '本地化遊戲文本已提取。';

  @override
  String get gameTextNotExtracted => '本地化遊戲文本尚未提取。';

  @override
  String get extracting => '正在提取…';

  @override
  String get extractRefreshLocalizedText => '提取 / 刷新本地化文本';

  @override
  String get extractionComplete => '提取完成';

  @override
  String get extractionFailed => '提取失敗';

  @override
  String get localizationCacheFileType => '本地化快取';

  @override
  String get savegameDirectoryTitle => '存檔目錄';

  @override
  String get folder => '資料夾';

  @override
  String get codecTitle => '編解碼器';

  @override
  String get check => '檢查';

  @override
  String get roundtrip => '往返測試';

  @override
  String get noCodecStatus => '無編解碼器狀態';

  @override
  String get codecReady => '編解碼器就緒';

  @override
  String get codecReadOnly => '編解碼器只讀';

  @override
  String get codecUnavailable => '編解碼器不可用';

  @override
  String get details => '詳情';

  @override
  String codecStatusLine(String status) {
    return '狀態：$status';
  }

  @override
  String codecCapabilityLine(String decompress, String compress) {
    return '解壓：$decompress | 壓縮：$compress';
  }

  @override
  String codecBackendLine(String backend) {
    return '後端：$backend';
  }

  @override
  String get yes => '是';

  @override
  String get no => '否';

  @override
  String aboutVersion(String version, String sha) {
    return '版本 $version（$sha）';
  }

  @override
  String get aboutCopyright => '© 2026 Daniel Hoer';

  @override
  String get aboutLicense => '基於 MIT 許可證授權。';

  @override
  String difficultyTitle(String profile) {
    return '難度 — $profile';
  }

  @override
  String get difficultyNoProfile => '無存檔配置';

  @override
  String get difficultyNoDifficulty => '無難度';

  @override
  String get difficultyLabel => '難度';

  @override
  String get difficultyTooltipNoProfile => '未選擇存檔配置';

  @override
  String get difficultyTooltipEdit => '編輯此存檔配置的難度';

  @override
  String get difficultyTooltipNoEditable => '此存檔配置沒有可編輯的難度';

  @override
  String get preset => '預設';

  @override
  String get presetNovice => '簡單';

  @override
  String get presetGothic => '哥特';

  @override
  String get presetHard => '困難';

  @override
  String get presetCustom => '自定義';

  @override
  String unrecognisedPreset(Object preset) {
    return '存儲的預設無法識別（$preset）。你仍可儲存流暢助手 / 永久死亡的更改，或在上方選擇一個預設以覆蓋它。';
  }

  @override
  String get closeCombatFlowHelper => '近戰流程助手';

  @override
  String get permadeath => '永久死亡';

  @override
  String get notAvailableOnNovice => '新手難度下不可用';

  @override
  String get levelCombat => '戰鬥';

  @override
  String get levelResources => '資源';

  @override
  String get levelProgression => '進度';

  @override
  String get difficultyAppliesToAllSaves => '難度將應用於此存檔配置中的所有存檔。';

  @override
  String get savingDifficultyFailed => '儲存難度失敗。';

  @override
  String get addItemDialogTitle => '添加物品';

  @override
  String get searchItems => '搜尋物品';

  @override
  String failedToLoadCatalog(String error) {
    return '載入目錄失敗：$error';
  }

  @override
  String get noItemsAvailableToAdd => '沒有可添加的物品';

  @override
  String get noItemsMatch => '沒有匹配的物品';

  @override
  String get countMustBeAtLeast1 => '必須 ≥ 1';

  @override
  String countMustBeAtMost(int max) {
    return '必須 ≤ $max';
  }

  @override
  String get addNpcDialogTitle => '添加 NPC';

  @override
  String get noNpcsAvailableToAdd => '沒有可添加的 NPC';

  @override
  String get noNpcsMatch => '沒有匹配的 NPC';

  @override
  String get categoryAll => '全部';

  @override
  String allWithCount(int count) {
    return '全部（$count）';
  }

  @override
  String get addKnowledgeEntryDialogTitle => '添加知識條目';

  @override
  String get searchEntries => '搜尋條目';

  @override
  String get noKnowledgeEntriesAvailableToAdd => '沒有可添加的知識條目';

  @override
  String get noEntriesMatch => '沒有匹配的條目';

  @override
  String get heroGroupMainStats => '主要屬性';

  @override
  String get heroGroupCombatMovement => '戰鬥 / 移動';

  @override
  String get heroGroupResistances => '抗性';

  @override
  String get heroGroupThieving => '盜竊';

  @override
  String get heroGroupAdvanced => '高級';

  @override
  String get heroGroupDiving => '潛水';

  @override
  String get heroDivingSkillNote =>
      '學會潛水後，遊戲每次讀取存檔都會把屏息量和恢復速度重置為技能自帶的數值。每秒消耗量則保持你設定的值。';

  @override
  String get heroGroupSleep => '睡眠';

  @override
  String get heroGroupIntoxication => '醉酒';

  @override
  String get heroEntryHeroTransform => '位置';

  @override
  String attributeEmpty(String name) {
    return '$name 為空 — 請輸入一個值，或在儲存前恢復原始值。';
  }

  @override
  String attributeInvalidNumber(String name, String text) {
    return '$name 的數字無效：“$text”';
  }

  @override
  String get loadingEditorData => '正在載入編輯器資料';

  @override
  String savingProgress(int done, int total) {
    return '正在儲存… $done/$total';
  }

  @override
  String localizedTextExtractedCount(int idCount, int languageCount) {
    return '已提取 $idCount 個 ID，涵蓋 $languageCount 種語言';
  }

  @override
  String get skillSmithing1H => '單手武器鍛造';

  @override
  String get skillSmithing2H => '雙手武器鍛造';

  @override
  String get skillCircleNovice => '新手法師';

  @override
  String get skillCircle1 => '第一魔法環階';

  @override
  String get skillCircle2 => '第二魔法環階';

  @override
  String get skillCircle3 => '第三魔法環階';

  @override
  String get skillCircle4 => '第四魔法環階';

  @override
  String get skillCircle5 => '第五魔法環階';

  @override
  String get skillCircle6 => '第六魔法環階';

  @override
  String get sectionGlossary => '圖鑑';

  @override
  String get glossarySearch => '搜尋圖鑑';

  @override
  String get glossaryOldCamp => '舊營地';

  @override
  String get glossaryNewCamp => '新營地';

  @override
  String get glossarySwampCamp => '沼澤營地';

  @override
  String get glossaryOutsiders => '外來者';

  @override
  String get glossaryCreatures => '生物';

  @override
  String get glossaryLocations => '地點';

  @override
  String get glossaryFilterLabel => '篩選';

  @override
  String get glossaryFilterTraders => '商人';

  @override
  String get glossaryFilterTeachers => '導師';

  @override
  String get roleTrader => '商人';

  @override
  String get roleDead => '已死亡';

  @override
  String get roleTeacher => '導師';

  @override
  String get roleArmorer => '護甲匠';

  @override
  String get glossaryFilterArmorers => '護甲匠';

  @override
  String get glossaryFilterHostile => '敵對';

  @override
  String get glossaryRelationshipFilterNote =>
      '顯示存檔中儲存的永久敵對覆蓋。公會、劇情、區域和犯罪產生的動態關係僅在遊戲中計算。';

  @override
  String get glossaryFilterDead => '已死亡';

  @override
  String get glossaryAddEntry => '添加圖鑑條目';

  @override
  String get glossaryAddTitle => '添加圖鑑條目';

  @override
  String get glossaryResetChanges => '重置圖鑑更改';

  @override
  String get glossaryNoVisibleEntries => '此視圖中沒有匹配的可見圖鑑條目。';

  @override
  String get glossaryNoHiddenEntries => '所有可用條目均已顯示。';

  @override
  String get glossaryNoMatch => '沒有匹配的圖鑑條目。';

  @override
  String get glossarySelectEntry => '選擇一個圖鑑條目以編輯其內容。';

  @override
  String glossaryEntryCount(int count) {
    return '$count 個條目';
  }

  @override
  String glossarySegmentsCount(int unlocked, int total) {
    return '已解鎖 $unlocked/$total 個條目';
  }

  @override
  String get glossaryPortraitUnlocked => '肖像已解鎖';

  @override
  String get glossaryPortraitSilhouette => '剪影 — 肖像尚未解鎖';

  @override
  String get glossarySegments => '條目';

  @override
  String get glossaryPending => '未儲存的更改';

  @override
  String get glossaryShowFullText => '顯示條目全文';

  @override
  String get glossarySegmentIntroduction => '介紹 / 肖像';

  @override
  String get glossarySegmentUnlock => '發現';

  @override
  String glossarySegmentEntry(int number) {
    return '條目 $number';
  }

  @override
  String get questJournalAll => '所有任務';

  @override
  String get questJournalOldCamp => '舊營地';

  @override
  String get questJournalNewCamp => '新營地';

  @override
  String get questJournalSwampCamp => '沼澤營地';

  @override
  String get questJournalColony => '殖民地';

  @override
  String get questJournalCompleted => '已完成';

  @override
  String get questJournalHint => '遊戲內日誌視圖。內部狀態和尚未開始的任務狀態仍可在“所有資料”中查看。';

  @override
  String get questJournalNoEntries => '沒有符合當前篩選條件的日誌任務。';

  @override
  String get glossaryTutorials => '教程';

  @override
  String get tutorialGateNote => '這些行控制存檔中的教程解鎖狀態。一個解鎖狀態不一定對應遊戲中的單個教程頁面。';

  @override
  String get tutorialResetChanges => '重置教程更改';

  @override
  String get tutorialNoGates => '此存檔中沒有可用的教程解鎖狀態。';

  @override
  String tutorialGateUnlockCount(int unlocked, int total) {
    return '已解鎖 $unlocked/$total 個教程';
  }

  @override
  String get tutorialGateCombatBasics => '戰鬥基礎';

  @override
  String get tutorialGateCrafting => '製作';

  @override
  String get tutorialGateCrime => '犯罪與後果';

  @override
  String get tutorialGateDrugs => '消耗品與效果';

  @override
  String get tutorialGateLockpicking => '開鎖';

  @override
  String get tutorialGateMagic => '魔法';

  @override
  String get tutorialGateMap => '地圖';

  @override
  String get tutorialGateMeleeCombat => '近戰';

  @override
  String get tutorialGateNavigation => '移動與導航';

  @override
  String get tutorialGatePerception => '感知';

  @override
  String get tutorialGatePlayerProgression => '角色成長';

  @override
  String get tutorialGateRanged => '遠程戰鬥';

  @override
  String get tutorialGateRiding => '騎乘';

  @override
  String get tutorialGateSleep => '睡眠';

  @override
  String get tutorialGateTrading => '交易';

  @override
  String get windowMinimizeTooltip => '最小化';

  @override
  String get windowMaximizeTooltip => '最大化';

  @override
  String get windowRestoreTooltip => '還原';

  @override
  String get fallbackDialogEntry => '對話條目';

  @override
  String get fallbackDialogChoice => '對話選項';

  @override
  String get fallbackDialogTopic => '對話主題';

  @override
  String get fallbackDialogInformation => '對話資訊';

  @override
  String get fallbackQuest => '任務';

  @override
  String get fallbackObjective => '目標';

  @override
  String get fallbackItem => '物品';

  @override
  String get attributeSkillPointsFallback => '學習點數（LP）';

  @override
  String attributeManualFallbackLabel(String attributeId, String fallback) {
    String _temp0 = intl.Intl.selectLogic(attributeId, {
      'SuperArmor': '霸體值',
      'MaxSuperArmor': '最大霸體值',
      'DamageMultiplier': '受到的傷害',
      'SpeedModifier': '移動速度',
      'Oxygen': '氧氣量',
      'MaxOxygen': '最大氧氣量',
      'OxygenDepletionRate': '每秒氧氣消耗',
      'OxygenRecoveryRate': '每秒氧氣恢復',
      'CriticalLevelPercent': '缺氧警告閾值',
      'SleepTime': '剩餘有效睡眠',
      'MaxSleepTime': '最大有效睡眠',
      'SleepTimeRecoveryAmount': '有效睡眠回補量',
      'SleepTimeRecoveryPeriod': '回補間隔',
      'MaxRestTime': '最長臥床時間',
      'Health_RecoveryRatePerHourOfSleep': '每小時睡眠回覆生命',
      'Mana_RecoveryRatePerHourOfSleep': '每小時睡眠回覆法力',
      'Alcohol': '酒精值',
      'MaxAlcohol': '最大酒精值',
      'AlcoholDepletionRate': '醒酒速度',
      'Swampweed': '沼澤草值',
      'MaxSwampweed': '最大沼澤草值',
      'SwampweedDepletionRate': '藥性消退速度',
      'XPExecutedBounty': '倒地處決獲得的經驗',
      'XPKillOrDefeatBounty': '擊敗獲得的經驗',
      'Level': '等級',
      'LockpickDurability': '開鎖器耐久',
      'LockpickPrecision': '開鎖精度',
      'PickPocketing': '扒竊',
      'other': '$fallback',
    });
    return '$_temp0';
  }

  @override
  String attributeManualTooltip(String attributeId) {
    String _temp0 = intl.Intl.selectLogic(attributeId, {
      'SuperArmor': '該角色在被一擊打得踉蹌之前還能扛下多少打擊。',
      'MaxSuperArmor': '霸體值的上限，會隨著等級提升和所穿的護甲一起增長。',
      'DamageMultiplier': '作用於該角色所受傷害的係數——1 為正常，數值越高越吃痛。',
      'SpeedModifier': '該角色移動快慢的係數——1 為正常。',
      'Oxygen': '水下剩餘的呼吸秒數，歸零時該角色就會淹死。',
      'MaxOxygen': '該角色能在水下待多少秒，潛水技能可以提高這個上限。',
      'OxygenDepletionRate': '潛在水下時每秒消耗掉的空氣量。',
      'OxygenRecoveryRate': '浮出水面後每秒回來的空氣量。',
      'CriticalLevelPercent': '剩餘空氣低到這個比例時，遊戲就會發出溺水警告。',
      'SleepTime': '還能帶來恢復的睡眠小時數，超出之後再睡遊戲也不會給任何恢復。',
      'MaxSleepTime': '該角色能攢下的有效睡眠時間上限。',
      'SleepTimeRecoveryAmount': '每次補充時重新加回來的有效睡眠小時數。',
      'SleepTimeRecoveryPeriod': '有效睡眠時間隔多久才會重新補滿。',
      'MaxRestTime': '遊戲允許一次躺在床上的最長時間。',
      'Health_RecoveryRatePerHourOfSleep': '每睡一小時能恢復的最大生命值比例。',
      'Mana_RecoveryRatePerHourOfSleep': '每睡一小時能恢復的最大法力值比例。',
      'Alcohol': '該角色醉到什麼程度，較高的檔位會拿敏捷和法力去換力量。',
      'MaxAlcohol': '該角色能達到的最高酒精值。',
      'AlcoholDepletionRate': '酒精值往清醒方向回落得有多快。',
      'Swampweed': '該角色嗨到什麼程度，較高的檔位會讓其屬性此消彼長。',
      'MaxSwampweed': '該角色能達到的最高沼澤草值。',
      'SwampweedDepletionRate': '沼澤草帶來的迷幻勁頭消退得有多快。',
      'XPExecutedBounty': '在這名角色已經被打倒在地時再將其殺死，所能拿到的經驗值。',
      'XPKillOrDefeatBounty': '把這名角色打倒時所能拿到的經驗值，不管對方是當場斃命還是只被打暈在地。',
      'Level': '角色等級。隨經驗提升，並帶來學習點數。',
      'LockpickDurability': '由開鎖技能決定：未學2、已學4、精通6。',
      'LockpickPrecision': '由開鎖技能決定：未學0、已學1、精通2。',
      'PickPocketing': '由扒竊技能決定：未學-30、已學-10、精通+10。',
      'other': '?',
    });
    return '$_temp0';
  }

  @override
  String get knowledgeTypeVoiceLine => '語音臺詞';

  @override
  String get knowledgeTypeOther => '其他';

  @override
  String get armorUpgradeUpper => '上部';

  @override
  String get armorUpgradeMiddle => '中部';

  @override
  String get armorUpgradeLower => '下部';

  @override
  String get knowledgeCategoryTopic => '主題';

  @override
  String get knowledgeCategoryChoice => '選項';

  @override
  String get knowledgeCategoryInfo => '資訊';

  @override
  String get statusOk => '正常';

  @override
  String get statusFailed => '失敗';

  @override
  String get missingSaveReference => '文件缺失';

  @override
  String missingSaveReferenceDescription(String slot) {
    return '$slot.sav 缺失。它可能已被刪除、移動或重命名；存檔配置仍在引用它。';
  }

  @override
  String get removeFromProfile => '從存檔配置中移除';

  @override
  String get deleteSavegame => '刪除存檔';

  @override
  String get deleteSavegameTitle => '刪除存檔？';

  @override
  String deleteSavegameBody(String save, String fileName, String profile) {
    return '要刪除 $save（$fileName）嗎？它將從 $profile 中移除，並從存檔資料夾中刪除。GORE 會先建立備份。';
  }

  @override
  String get removeSaveFromProfileTitle => '從存檔配置中移除存檔？';

  @override
  String removeSaveFromProfileBody(String save, String profile) {
    return '要從 $profile 中移除 $save 嗎？如果存檔檔案仍然存在，文件本身將會保留。';
  }

  @override
  String get unassignedSave => '未分配給存檔配置';

  @override
  String get armorUpgradeLight => '輕型';

  @override
  String get armorUpgradeMedium => '中型';

  @override
  String get armorUpgradeHeavy => '重型';

  @override
  String get knowledgeCaptionForcedConversation => '強制對話';

  @override
  String get knowledgeCaptionFollowupTopic => '後續話題';

  @override
  String get knowledgeCaptionFallbackTopic => '後備話題';

  @override
  String durationMinutes(int minutes) {
    return '$minutes 分鐘';
  }

  @override
  String durationHours(int hours) {
    return '$hours 小時';
  }

  @override
  String durationHoursMinutes(int hours, int minutes) {
    return '$hours 小時 $minutes 分鐘';
  }

  @override
  String get backupStatusInvalidProfileStructure => '存檔配置資料無效';

  @override
  String get backupStatusSlotMetadataMissing => '所選存檔的中繼資料缺失';

  @override
  String defaultProfileName(int id) {
    return '存檔配置 $id';
  }

  @override
  String get statusUnknown => '未知';

  @override
  String editorUnexpectedError(String details) {
    return '意外錯誤：$details';
  }

  @override
  String get editorOperationInProgress => '另一項操作正在進行中。請稍後重試。';

  @override
  String get editorUnsavedBeforeDifficulty =>
      '存檔中有未儲存的修改。更改存檔配置難度前，請先儲存或重置這些修改。';

  @override
  String get editorNoSaveFolderSelected => '未選擇存檔資料夾。';

  @override
  String get editorNoSaveSelected => '未選擇存檔。';

  @override
  String get coreUnknownError => '核心組件發生未知錯誤';

  @override
  String get editorUnsavedBeforeSwitchProfile => '請先儲存或重置未儲存的修改；切換存檔配置會離開當前存檔。';

  @override
  String get editorUnsavedBeforeOpenFile => '打開其他文件前，請先儲存或重置未儲存的修改。';

  @override
  String get editorSelectSavFile => '請選擇 .sav 存檔檔案。';

  @override
  String get editorNotGothicGsav => '所選文件不是 Gothic GSAV 存檔。';

  @override
  String get editorUnsavedBeforeChangeSaveProfile =>
      '更改存檔所屬的存檔配置前，請先儲存或重置未儲存的修改。';

  @override
  String get editorUnsavedBeforeRemoveProfile => '從存檔配置中移除存檔前，請先儲存或重置未儲存的修改。';

  @override
  String get editorUnsavedBeforeDeleteSave => '刪除此存檔前，請先儲存或重置未儲存的修改。';

  @override
  String get editorUnsavedBeforeRestoreProfile =>
      '存檔中有未儲存的修改。恢復存檔配置備份前，請先儲存或重置這些修改。';

  @override
  String editorConflictingPropertyEdits(String path) {
    return '兩個標籤頁中未儲存的修改針對同一屬性 ($path)。請重置或撤銷其中一項，然後再次儲存。';
  }

  @override
  String editorGlossaryMemoryConflict(String path) {
    return '圖鑑分段更改和“全部資料”中另一項未儲存的修改都針對 Hero MemorizedEvents 陣列 ($path)。圖鑑更改會在該陣列中添加或移除條目，因此兩項修改無法同時儲存。請重置或撤銷其中一項，然後再次儲存。';
  }

  @override
  String editorGlossaryQuestConflict(String path) {
    return '圖鑑分段更改和另一項未儲存的修改都針對同一任務的 CurrentState 屬性 ($path)。圖鑑更改本身會更新該狀態。請重置或撤銷其中一項，然後再次儲存。';
  }

  @override
  String editorRelationshipConflict(String path) {
    return '關係覆蓋設定和“全部資料”中另一項未儲存的修改都針對同一 NPC 關係條目 ($path)。結構化的關係更改可能會替換該條目中的修正值，因此兩項修改無法同時儲存。請重置或撤銷其中一項，然後再次儲存。';
  }

  @override
  String editorMultipleStructuralArrayEdits(String path) {
    return '同一陣列 ($path) 有多項未儲存的結構更改。添加另一項更改前，請先儲存或重置第一項更改。';
  }

  @override
  String editorStructuralArrayConflict(String path) {
    return '事件結構更改和“全部資料”中另一項未儲存的修改都針對 $path。繼續前，請儲存或重置其中一項。';
  }

  @override
  String get editorSkillsEffectConflict =>
      '“技能”中的更改和“全部資料”中針對同一角色效果 (ActiveEffects › EffectSpec › Def) 的修改都在等待儲存。兩項修改無法同時儲存。請重置或撤銷其中一項，然後再次儲存。';

  @override
  String get editorInventoryResetConflict =>
      '重置物品欄和對同一物品欄的另一項修改都在等待儲存。重置會替換整個物品欄並丟棄另一項修改。請重置或撤銷其中一項，然後再次儲存。';

  @override
  String get editorUseFolder => '使用此資料夾';

  @override
  String get editorGothicSavegameFileType => 'Gothic 存檔';

  @override
  String get editorNoDifficultyChanges => '沒有需要儲存的難度更改';

  @override
  String get editorDifficultyWritten => '難度已寫入存檔配置（已建立備份）';

  @override
  String editorChangesSavedWithBackup(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '已儲存 $count 項更改並建立備份',
      one: '已儲存 1 項更改並建立備份',
    );
    return '$_temp0';
  }

  @override
  String editorPlacementNoteFailed(String details) {
    return '移動已儲存，但無法寫入用於還原的記錄：$details';
  }

  @override
  String editorProfileNotFound(int profileId) {
    return '未找到存檔配置 $profileId。';
  }

  @override
  String get editorNoFreeSaveSlot => '遊戲存檔資料夾中沒有可用的存檔槽位（G1R-001 至 G1R-999）。';

  @override
  String editorSaveImportedAssigned(int profileId) {
    return '存檔已導入並分配給存檔配置 $profileId';
  }

  @override
  String editorSaveAssigned(int profileId) {
    return '存檔已分配給存檔配置 $profileId（已建立配套備份）';
  }

  @override
  String editorSaveSlotNotAssigned(String slot, int profileId) {
    return '存檔槽位 $slot 未分配給存檔配置 $profileId。';
  }

  @override
  String get editorSaveRemovedFromProfile => '已從存檔配置中移除存檔';

  @override
  String get editorSaveDeleted => '存檔已刪除；已建立備份';

  @override
  String editorRestoredBackup(String path) {
    return '已恢復備份：$path';
  }

  @override
  String editorRestoredBackupWithoutCompanion(String path) {
    return '已恢復備份：$path（由於沒有匹配的配套備份，PersistentDataList.sav 保持不變；槽位中繼資料可能不同）';
  }

  @override
  String editorCodecRoundtripPassed(int chunkIndex, int bytes) {
    return '編解碼器往返驗證通過：區塊 $chunkIndex 已重新壓縮為 $bytes 位元組';
  }

  @override
  String editorDifficultyWriteFailed(String details) {
    return '無法寫入存檔配置難度：$details';
  }

  @override
  String editorProfileAssignmentFailed(String details) {
    return '無法將存檔分配給存檔配置：$details';
  }

  @override
  String editorProfileRemovalFailed(String details) {
    return '無法從存檔配置中移除存檔：$details';
  }

  @override
  String editorDeleteSaveFailed(String details) {
    return '無法刪除存檔：$details';
  }

  @override
  String editorSaveFailed(String details) {
    return '無法儲存修改：$details';
  }

  @override
  String editorScanSavesFailed(String details) {
    return '掃描存檔失敗：$details';
  }

  @override
  String editorInspectSaveFailed(String details) {
    return '檢查存檔失敗：$details';
  }

  @override
  String editorLoadBackupsFailed(String details) {
    return '載入備份失敗：$details';
  }

  @override
  String editorRestoreFailed(String details) {
    return '無法恢復備份：$details';
  }

  @override
  String editorRestoreReloadFailed(String path, String details) {
    return '已恢復備份：$path，但重新載入存檔失敗：$details';
  }

  @override
  String editorCodecCheckFailed(String details) {
    return '編解碼器檢查失敗：$details';
  }

  @override
  String editorCodecValidationFailed(String details) {
    return '編解碼器往返驗證失敗：$details';
  }

  @override
  String editorPropertySearchFailed(String details) {
    return '屬性搜尋失敗：$details';
  }

  @override
  String get editorSelectionChangedWhileLoadingHeroAttributes =>
      '載入英雄屬性時，所選存檔發生了變化。';

  @override
  String editorSkillsLoadFailed(String details) {
    return '載入技能失敗：$details';
  }

  @override
  String editorProgressionQueryFailed(String details) {
    return '查詢進度失敗：$details';
  }

  @override
  String editorNpcListFailed(String details) {
    return '載入 NPC 列表失敗：$details';
  }

  @override
  String editorCharacterListFailed(String details) {
    return '載入角色列表失敗：$details';
  }

  @override
  String editorNpcAttributesFailed(String details) {
    return '載入 NPC 屬性失敗：$details';
  }

  @override
  String editorNpcPositionFailed(String details) {
    return '載入 NPC 位置失敗：$details';
  }

  @override
  String editorNpcInventoryFailed(String details) {
    return '載入 NPC 物品欄失敗：$details';
  }

  @override
  String editorFactionListFailed(String details) {
    return '載入陣營列表失敗：$details';
  }

  @override
  String get editorNoBackupPath => '無';

  @override
  String editorBackupMessage(String prefix, String backupPath) {
    return '$prefix：$backupPath';
  }

  @override
  String editorBackupMessageWithPersistent(
    String prefix,
    String backupPath,
    String persistentPath,
  ) {
    return '$prefix：$backupPath；PersistentDataList 備份：$persistentPath';
  }

  @override
  String localizationStatusFailed(String details) {
    return '獲取本地化狀態失敗：$details';
  }

  @override
  String localizationExtractionFailed(String details) {
    return '提取失敗：$details';
  }

  @override
  String glossaryLoadFailed(String details) {
    return '載入圖鑑失敗：$details';
  }

  @override
  String backupStatusError(String details) {
    return '備份錯誤：$details';
  }

  @override
  String memoryEventCategory(String category, String fallback) {
    String _temp0 = intl.Intl.selectLogic(category, {
      'quest': '任務',
      'document': '文件',
      'story': '劇情',
      'exploration': '探索',
      'combat': '戰鬥',
      'social': '社交',
      'item': '物品',
      'learning': '學習',
      'guild': '公會',
      'crime': '犯罪',
      'rest': '休息',
      'other': '$fallback',
    });
    return '$_temp0';
  }

  @override
  String memoryEventAction(String kind, String fallback) {
    String _temp0 = intl.Intl.selectLogic(kind, {
      'questStarted': '任務已開始',
      'questSucceeded': '任務已完成',
      'questFailed': '任務失敗',
      'documentRead': '已閱讀文件',
      'documentSegmentUnlocked': '已發現條目',
      'documentSegmentViewed': '已查看條目',
      'chapterCompleted': '章節已完成',
      'areaEntered': '進入區域',
      'areaLeft': '離開區域',
      'characterKilled': '角色已被殺死',
      'characterDefeated': '角色已被擊敗',
      'combatDodge': '已閃避攻擊',
      'characterDebuffed': '已施加負面效果',
      'tradeAvailable': '已解鎖交易',
      'itemObtained': '獲得物品',
      'itemCrafted': '製作物品',
      'skillStateRecorded': '已記錄技能狀態',
      'recipeLearned': '學會配方',
      'guildJoined': '加入公會',
      'crimeRecorded': '犯罪已記錄',
      'slept': '睡眠',
      'storyEvent': '劇情事件',
      'other': '$fallback',
    });
    return '$_temp0';
  }

  @override
  String memoryEventTitleWithSubject(String action, String subject) {
    return '$action：$subject';
  }

  @override
  String memoryEventFact(String fact, String fallback) {
    String _temp0 = intl.Intl.selectLogic(fact, {
      'gameTime': '遊戲時間',
      'duration': '持續時間',
      'chapter': '章節',
      'instigator': '觸發者',
      'affected': '受影響對象',
      'amount': '數量',
      'primaryObject': '對象',
      'secondaryObject': '上下文',
      'segmentText': '條目文本',
      'other': '$fallback',
    });
    return '$_temp0';
  }

  @override
  String memoryEventGameTime(int day, String time) {
    return '第 $day 天，$time';
  }

  @override
  String memoryEventSecondsValue(String value) {
    return '$value 秒';
  }

  @override
  String memoryEventMoreValues(String values, int count) {
    return '$values +$count';
  }

  @override
  String get memoryEventHero => '主角';

  @override
  String get memoryEventDetails => '詳細資訊';

  @override
  String get memoryEventTags => '標籤';

  @override
  String get memoryEventTechnicalData => '技術資訊';

  @override
  String get memoryEventIndex => '索引';

  @override
  String get memoryEventPosition => '位置';

  @override
  String get memoryEventPayload => '事件資料';

  @override
  String get memoryEventSubject => '關聯對象';

  @override
  String glossaryCatalogSegmentLabel(String segmentId, String fallback) {
    String _temp0 = intl.Intl.selectLogic(segmentId, {
      'Access': '通行',
      'AccessDenied': '禁止通行',
      'AccesToTemple': '進入神殿',
      'Advice': '建議',
      'AfterFight': '戰鬥之後',
      'AfterFireMages': '火法師事件之後',
      'AfterNek': '尼克之後',
      'AfterQuest': '任務之後',
      'Alone': '獨自一人',
      'Amulet': '護符',
      'Annoying': '煩人',
      'Armor': '護甲',
      'Avoid': '迴避',
      'Backstory': '背景故事',
      'BackStory': '背景故事',
      'BasicMagic': '基礎魔法',
      'Beated': '被打敗',
      'BecomeMercenary': '成為傭兵',
      'Beer': '啤酒',
      'Bestiary': '怪物圖鑑',
      'Blessing': '祝福',
      'Boss': '首領',
      'Bully': '惡霸',
      'BullyAdvice': '應對惡霸的建議',
      'Camp': '營地',
      'CampDivided': '分裂的營地',
      'CareOfMessengers': '照顧信使',
      'ChangeOpinion': '改變看法',
      'ChargeUriziel': '為尤里澤爾充能',
      'Chosen': '天選者',
      'Contact': '接觸',
      'Courier': '信使',
      'CraftBows': '製作弓箭',
      'Crazy': '瘋癲',
      'DailyMeal': '每日餐食',
      'DailyRation_Trader': '每日口糧商人',
      'DAM': '水壩',
      'Dead': '死亡',
      'Deal': '交易',
      'Dealer': '交易商',
      'Deceived': '受騙',
      'Dementia': '痴呆',
      'DenyAccess': '拒絕通行',
      'DifferentOpinion': '不同意見',
      'Discussion': '討論',
      'DontTalk': '不許交談',
      'Duel': '決鬥',
      'Entrance': '入口',
      'Escape': '逃脫',
      'Extended': '擴展',
      'Extra': '額外',
      'ExtraInfo': '額外資訊',
      'Fanatic': '狂信徒',
      'Fight': '戰鬥',
      'FindUlumulu': '尋找烏魯穆魯',
      'FireMages': '火法師',
      'FireMagesEscape': '火法師的逃脫',
      'FiskNewDealer': '菲斯克的新銷贓販子',
      'FiskNewDealerCompleted': '菲斯克的新銷贓販子——已完成',
      'FogTower': '霧塔',
      'Food': '食物',
      'Forgave': '已原諒',
      'Forgive': '原諒',
      'Forgiven': '已獲原諒',
      'FourFriends': '四位好友',
      'FreeHut': '空閒小屋',
      'FreeMine': '自由礦場',
      'Fury': '狂怒',
      'GoodTeacher': '好導師',
      'Gossip': '傳聞',
      'GotScavenger': '獲得食屍鳥',
      'GrantedAccess': '已獲准通行',
      'GRDArmor': '衛兵防具',
      'Guide': '嚮導',
      'HateMages': '仇恨法師',
      'HateMagesExplanation': '仇恨法師的原因',
      'HateRiceLord': '憎恨稻田主',
      'Heal': '治療',
      'Healing': '治療',
      'Help': '幫助',
      'Helper': '幫手',
      'HelpKagan': '幫助卡根',
      'HutStory': '小屋的故事',
      'Ignore': '無視',
      'Impress': '打動',
      'ImpressAlchemy': '用鍊金術打動',
      'ImpressInscription': '用銘文打動',
      'Info': '資訊',
      'Interested': '感興趣',
      'Introduction': '初識',
      'Introduction_2': '初識 2',
      'Introduction_Armor': '護甲介紹',
      'Introduction_Teacher': '初識（導師）',
      'Introduction_Trader': '初識（商人）',
      'Invocation': '召喚儀式',
      'JoinSC': '加入沼澤營地',
      'Joint': '沼澤草菸捲',
      'KalomCamp': '科爾·卡隆的營地',
      'Leader': '領袖',
      'Learning': '學習',
      'LearnOrcish': '學習獸人語',
      'LeftParty': '離隊',
      'Library': '圖書館',
      'Lie': '謊言',
      'Lock': '鎖',
      'Lockpick': '開鎖工具',
      'Mad': '瘋狂',
      'Mandibles': '礦爬蟲的下顎',
      'MapMaker': '製圖師',
      'Monastery': '修道院',
      'MordragKO': '擊倒莫德拉格',
      'Nek': '尼克',
      'NewCamp': '新營地',
      'NewCamper': '新營地成員',
      'NewLeader': '新領袖',
      'NightPatrol': '夜間巡邏',
      'NotInterested': '不感興趣',
      'OldCamp': '舊營地',
      'OrcEnclaveEntrance': '獸族聚居地入口',
      'OrcGraveyard': '獸人墓地',
      'OreArmor': '礦石鎧甲',
      'Party': '隊伍',
      'Pay': '付款',
      'PayMoney': '付錢',
      'Permission': '許可',
      'Pet': '寵物',
      'PreparingInvocation': '準備召喚儀式',
      'Quest': '任務',
      'RankUpFireMages': '晉升為火法師',
      'RankUpGuard': '晉升為衛兵',
      'RanUpFireMagesCompleted': '晉升火法師完成',
      'Realocated': '已遷移',
      'Reason': '原因',
      'Respect': '尊重',
      'ReturnToSC': '返回沼澤營地',
      'RicelordForeman': '稻田主的監工',
      'RideScavenger': '騎乘食屍鳥',
      'Robe': '法袍',
      'Safe': '安全',
      'Scraper': '採礦工',
      'SecondChance': '第二次機會',
      'SecretLocation': '秘密地點',
      'SecretPassage': '秘密通道',
      'SecretPath': '秘密小徑',
      'SleeperFollower': '沉睡者信徒',
      'SleeperTemple': '沉睡者神廟',
      'SmallInfo': '小道消息',
      'Stonehenge': '巨石陣',
      'StopFollowing': '停止跟隨',
      'SwampCamp': '沼澤營地',
      'Talkative': '健談',
      'Teach': '教學',
      'TeachBow': '弓術訓練',
      'Teacher': '導師',
      'Teacher2': '導師 2',
      'TeacherInscription': '銘文導師',
      'TeacherMana': '魔力導師',
      'TeachIchor': '礦爬蟲體液採集訓練',
      'TeachMagic': '魔法訓練',
      'TeachOrcish': '教授獸人語',
      'TeachStats': '屬性訓練',
      'TeachWeapon': '武器訓練',
      'Teleport': '傳送',
      'TheMysteriousOrc': '神秘的獸人',
      'ThroneRoom': '王座廳',
      'TradeBow': '弓箭交易',
      'Trader': '商人',
      'TradeSkins_Trader': '毛皮商人',
      'Traitor': '叛徒',
      'Trial': '試煉',
      'TrollCanyon': '巨魔谷',
      'Trust': '信任',
      'Ulumulu': '烏魯穆魯',
      'Unexperienced': '缺乏經驗',
      'Uriziel': '尤里澤爾',
      'UrizielRune': '尤里澤爾符文',
      'Useful': '有用',
      'Velaya': '維拉雅',
      'Vibrations': '震動',
      'WaitFreeMine': '在自由礦場等待',
      'WaitInTrainingArea': '在訓練場等待',
      'Warning': '警告',
      'WarningTooLate': '遲來的警告',
      'WaterMessenger': '水法師的信使',
      'Weapon': '武器',
      'Who': '身份',
      'Women': '女人們',
      'other': '$fallback',
    });
    return '$_temp0';
  }

  @override
  String get slotRepairTitle => '物品欄槽位損壞';

  @override
  String slotRepairBody(int count) {
    return '此存檔中有 $count 個物品欄槽位的 ID 與其位置不再匹配 — 在遊戲中丟棄這類物品時，會誤刪另一件物品。修復僅重寫槽位 ID，不會添加、刪除或更改任何物品。儲存時會照常建立備份。';
  }

  @override
  String get slotRepairQueued => '修復已加入待儲存列表 — 儲存後生效。';

  @override
  String get slotRepairAction => '修復';

  @override
  String get slotRepairDiscard => '放棄';

  @override
  String get editorInventorySlotEditConflict =>
      '對物品欄槽位的直接編輯與佔用整個槽位的操作（修復、添加或刪除）同時在待儲存列表中。後者會覆蓋前者 — 請撤銷其中一項後再儲存。';

  @override
  String get editorTraderArrayConflict =>
      '一項交易修改與對商人陣列的直接編輯一同排隊。該編輯會重新編號交易修改所依據的行，因此兩者之一會落到錯誤的商人身上——撤銷其中一項後再儲存。';

  @override
  String get backupFactFile => '文件';

  @override
  String get renameBackupTooltip => '為此備份命名';

  @override
  String get renameBackupTitle => '備份名稱';

  @override
  String get renameBackupLabel => '名稱';

  @override
  String renameBackupHelp(String fileName) {
    return '顯示時取代文件名 $fileName。留空則移除名稱；文件本身不會被重命名。';
  }

  @override
  String get deleteBackupTooltip => '刪除此備份';

  @override
  String get deleteBackupTitle => '刪除備份';

  @override
  String deleteBackupBody(String name, String fileName) {
    return '刪除“$name”（$fileName）？文件將從磁碟移除且無法恢復。';
  }

  @override
  String get deleteBackupConfirm => '刪除';

  @override
  String editorDeletedBackup(String path) {
    return '已刪除備份：$path';
  }

  @override
  String editorDeleteBackupFailed(String details) {
    return '無法刪除備份：$details';
  }

  @override
  String editorRenameBackupFailed(String details) {
    return '無法為備份命名：$details';
  }

  @override
  String get slotRepairUnavailable => '目前無法修復 — 無法寫入此存檔。';

  @override
  String editorDeletedBackupWithLabelWarning(String path, String details) {
    return '已刪除備份：$path — 但無法移除其名稱：$details';
  }

  @override
  String get slotRepairNotOffered => '此存檔不支持修復。';

  @override
  String get statisticsTitle => '統計';

  @override
  String get statisticsSubtitle => '角色、任務、世界和遊戲進度的簡要概覽。';

  @override
  String statisticsCardTitle(String card, String fallback) {
    String _temp0 = intl.Intl.selectLogic(card, {
      'timing': '時間',
      'character': '角色',
      'quests': '任務',
      'progress': '進度',
      'encounters': '戰鬥與交往',
      'inventory': '技能與物品',
      'other': '$fallback',
    });
    return '$_temp0';
  }

  @override
  String statisticsMetric(String metric, String fallback) {
    String _temp0 = intl.Intl.selectLogic(metric, {
      'timePlayed': '遊玩時間',
      'worldTime': '世界時間',
      'level': '等級',
      'experience': '經驗',
      'learningPoints': '學習點數',
      'guild': '陣營',
      'health': '生命值',
      'mana': '法力',
      'chapter': '章節',
      'location': '位置',
      'kills': '擊殺NPC',
      'knownCharacters': '已知角色',
      'killedMonsters': '擊殺怪物',
      'defeatedNpcs': '擊敗NPC',
      'killedNpcs': '擊殺NPC',
      'knownNpcs': '已知NPC',
      'knownTeachers': '已知導師',
      'learnedSkills': '已學技能',
      'knowledge': '知識條目',
      'deadCharacters': '死亡角色',
      'traders': '已知商人',
      'inventoryStacks': '物品堆疊',
      'inventoryItems': '物品',
      'ore': '礦石',
      'equipped': '已裝備',
      'hostileFactions': '敵對陣營',
      'openCrimes': '未解決罪行',
      'position': '座標',
      'other': '$fallback',
    });
    return '$_temp0';
  }

  @override
  String statisticsGuildRank(String rank, String fallback) {
    String _temp0 = intl.Intl.selectLogic(rank, {
      'oldCampShadow': '舊營地 · 影子',
      'oldCampGuard': '舊營地 · 守衛',
      'oldCampFireMage': '舊營地 · 火法師',
      'newCampRogue': '新營地 · 強盜',
      'newCampMercenary': '新營地 · 僱傭兵',
      'newCampWaterMage': '新營地 · 水法師',
      'swampCampNovice': '沼澤營地 · 新人',
      'swampCampTemplar': '沼澤營地 · 聖殿騎士',
      'other': '$fallback',
    });
    return '$_temp0';
  }

  @override
  String get statisticsUnknown => '不可用';

  @override
  String get statisticsMore => '更多統計';

  @override
  String statisticsSummary(
    String level,
    String guild,
    String chapter,
    int completed,
    int failed,
    String playTime,
  ) {
    return '等級$level，$guild，第$chapter章。完成任務$completed個，失敗$failed個。遊玩時間：$playTime。';
  }

  @override
  String get locksSidebar => '鎖';

  @override
  String editorLockListFailed(String details) {
    return '無法載入鎖列表：$details';
  }

  @override
  String get locksSearchHint => '搜尋鎖或鑰匙';

  @override
  String get locksAllRegions => '所有區域';

  @override
  String locksShownOfTotal(int shown, int total) {
    return '$total 箇中的 $shown 個';
  }

  @override
  String get locksFilterChests => '箱子';

  @override
  String get locksFilterDoors => '門';

  @override
  String get locksFilterUnlocked => '已開啟';

  @override
  String get locksFilterLocked => '已上鎖';

  @override
  String locksDifficultyLevel(int bars, int level) {
    return '難度 $bars/4（內部等級 $level/7）';
  }

  @override
  String get locksKeyOnly => '僅限鑰匙';

  @override
  String locksKeyLabel(String keys) {
    return '鑰匙：$keys';
  }

  @override
  String get locksPermalocked => '永久封閉';

  @override
  String get locksReadOnly => '此存檔沒有可編輯的鎖列表。';

  @override
  String get locksUnknownEntry => '此遊戲版本中不存在';

  @override
  String get locksDoorLeafHint => '重新鎖上門時，門本身也會關閉。';

  @override
  String get locksResetPending => '放棄待儲存的鎖改動';
}
