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
      '游戏随附脚本中声明的持久剧情状态权威目录。已保存条目显示原始值；此存档中缺少的目录字段会标记为未设置。源码声明的时间标记会格式化为游戏时间，其他整数可能表示布尔值、计数器或多阶段状态。';

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
      '所有条目在整个有符号 int32 范围内都可编辑。根据脚本确定的标志和建议值仅供参考；始终可以直接输入原始值。更改剧情状态可能会跳过对话、任务或世界的正常转换，因此请谨慎保存。系统会自动创建备份。';

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
  String get traderOre => '矿石（购买力）';

  @override
  String get traderNoOre => '无矿石';

  @override
  String get traderStockCurrent => '库存';

  @override
  String get traderStockBase => '补货基准';

  @override
  String get traderStockBaseHint => '商人补货的基准。会随剧情推进而增长，因此不是初始状态。';

  @override
  String get traderOreHint =>
      '游戏内的数值会不同：载入时游戏会加上自他上次交易以来累积的部分——他会卖掉多余货物并以此补货。这个数字是起点，而非交易界面显示的金额。';

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
  String get traderEmptyStock => '没有库存。';

  @override
  String get traderUnknownItem => '不在物品目录中';

  @override
  String editorTradersLoadFailed(String details) {
    return '商人数据加载失败：$details';
  }

  @override
  String traderStockLineCount(int count) {
    return '$count 条';
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
  String get publicSaveName => '公开存档名称';

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
  String get itemCategoryMeleeWeapon => '近战武器';

  @override
  String get itemCategoryRangedWeapon => '远程武器';

  @override
  String get itemCategoryAmmunition => '弹药';

  @override
  String get itemCategoryArmor => '护甲';

  @override
  String get itemCategoryRune => '符文';

  @override
  String get itemCategoryScroll => '法术卷轴';

  @override
  String get itemCategoryFood => '食物与药水';

  @override
  String get itemCategoryMisc => '杂项';

  @override
  String get itemCategoryAmulet => '护身符';

  @override
  String get itemCategoryRing => '戒指';

  @override
  String get itemCategoryTrophy => '动物战利品';

  @override
  String get itemCategoryWriting => '文书';

  @override
  String get itemCategoryMission => '任务物品';

  @override
  String get itemCategoryKey => '钥匙';

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
  String get language => '语言';

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
  String get updateLater => '稍后';

  @override
  String get updateUpToDate => '您正在使用最新版本。';

  @override
  String get updateCheckFailed => '无法检查更新，请稍后重试。';

  @override
  String get gameTextTitle => '游戏文本';

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
  String get extractLocalizedTextTitle => '提取本地化游戏文本？';

  @override
  String get extractLocalizedTextBody => '本地化游戏文本尚未提取。现在从你的游戏安装目录提取吗？（可选）';

  @override
  String get notNow => '暂不';

  @override
  String get extract => '提取';

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
  String get aboutCopyright => '© 2026 GORE 贡献者';

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
  String get heroGroupCombatSkills => '战斗技能';

  @override
  String get heroGroupResistances => '抗性';

  @override
  String get heroGroupThieving => '盗窃';

  @override
  String get heroGroupAdvanced => '高级';

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
      'Alcohol': '酒精值',
      'AlcoholDepletionRate': '酒精消退速率',
      'MaxAlcohol': '最大酒精值',
      'MaxSuperArmor': '最大霸体值',
      'SuperArmor': '霸体值',
      'Fatigue': '疲劳值',
      'FillRatio': '填充比例',
      'FillRatioPeriod': '填充周期',
      'MaxFatigue': '最大疲劳值',
      'MaxThresholdIndex': '最大阈值索引',
      'RecoveryRatePerHourOfSleep': '每小时睡眠恢复量',
      'DamageMultiplier': '伤害倍率',
      'Toughness': '韧性',
      'ToughnessA': '韧性 A',
      'ToughnessB': '韧性 B',
      'ToughnessC': '韧性 C',
      'XPExecutedBounty': '处决经验奖励',
      'XPKillOrDefeatBounty': '击杀或击败经验奖励',
      'SpeedModifier': '速度修正',
      'CriticalLevelPercent': '临界等级（%）',
      'MaxOxygen': '最大氧气量',
      'Oxygen': '氧气量',
      'OxygenDepletionRate': '氧气消耗速率',
      'OxygenRecoveryRate': '氧气恢复速率',
      'MaxRestTime': '最大休息时间',
      'MaxSleepTime': '最大睡眠时间',
      'SleepTime': '睡眠时间',
      'SleepTimeRecoveryAmount': '睡眠恢复量',
      'SleepTimeRecoveryPeriod': '睡眠恢复周期',
      'MaxSwampweed': '最大沼泽草量',
      'Swampweed': '沼泽草',
      'SwampweedDepletionRate': '沼泽草消耗速率',
      'other': '$fallback',
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
      '游戏随附脚本中声明的持久剧情状态权威目录。已保存条目显示原始值；此存档中缺少的目录字段会标记为未设置。源码声明的时间标记会格式化为游戏时间，其他整数可能表示布尔值、计数器或多阶段状态。';

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
      '所有条目在整个有符号 int32 范围内都可编辑。根据脚本确定的标志和建议值仅供参考；始终可以直接输入原始值。更改剧情状态可能会跳过对话、任务或世界的正常转换，因此请谨慎保存。系统会自动创建备份。';

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
  String get traderOre => '矿石（购买力）';

  @override
  String get traderNoOre => '无矿石';

  @override
  String get traderStockCurrent => '库存';

  @override
  String get traderStockBase => '补货基准';

  @override
  String get traderStockBaseHint => '商人补货的基准。会随剧情推进而增长，因此不是初始状态。';

  @override
  String get traderOreHint =>
      '游戏内的数值会不同：载入时游戏会加上自他上次交易以来累积的部分——他会卖掉多余货物并以此补货。这个数字是起点，而非交易界面显示的金额。';

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
  String get traderEmptyStock => '没有库存。';

  @override
  String get traderUnknownItem => '不在物品目录中';

  @override
  String editorTradersLoadFailed(String details) {
    return '商人数据加载失败：$details';
  }

  @override
  String traderStockLineCount(int count) {
    return '$count 条';
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
  String get publicSaveName => '公开存档名称';

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
  String get itemCategoryMeleeWeapon => '近战武器';

  @override
  String get itemCategoryRangedWeapon => '远程武器';

  @override
  String get itemCategoryAmmunition => '弹药';

  @override
  String get itemCategoryArmor => '护甲';

  @override
  String get itemCategoryRune => '符文';

  @override
  String get itemCategoryScroll => '法术卷轴';

  @override
  String get itemCategoryFood => '食物与药水';

  @override
  String get itemCategoryMisc => '杂项';

  @override
  String get itemCategoryAmulet => '护身符';

  @override
  String get itemCategoryRing => '戒指';

  @override
  String get itemCategoryTrophy => '动物战利品';

  @override
  String get itemCategoryWriting => '文书';

  @override
  String get itemCategoryMission => '任务物品';

  @override
  String get itemCategoryKey => '钥匙';

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
  String get language => '语言';

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
  String get updateLater => '稍后';

  @override
  String get updateUpToDate => '您正在使用最新版本。';

  @override
  String get updateCheckFailed => '无法检查更新，请稍后重试。';

  @override
  String get gameTextTitle => '游戏文本';

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
  String get extractLocalizedTextTitle => '提取本地化游戏文本？';

  @override
  String get extractLocalizedTextBody => '本地化游戏文本尚未提取。现在从你的游戏安装目录提取吗？（可选）';

  @override
  String get notNow => '暂不';

  @override
  String get extract => '提取';

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
  String get aboutCopyright => '© 2026 GORE 贡献者';

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
  String get heroGroupCombatSkills => '战斗技能';

  @override
  String get heroGroupResistances => '抗性';

  @override
  String get heroGroupThieving => '盗窃';

  @override
  String get heroGroupAdvanced => '高级';

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
      'Alcohol': '酒精值',
      'AlcoholDepletionRate': '酒精消退速率',
      'MaxAlcohol': '最大酒精值',
      'MaxSuperArmor': '最大霸体值',
      'SuperArmor': '霸体值',
      'Fatigue': '疲劳值',
      'FillRatio': '填充比例',
      'FillRatioPeriod': '填充周期',
      'MaxFatigue': '最大疲劳值',
      'MaxThresholdIndex': '最大阈值索引',
      'RecoveryRatePerHourOfSleep': '每小时睡眠恢复量',
      'DamageMultiplier': '伤害倍率',
      'Toughness': '韧性',
      'ToughnessA': '韧性 A',
      'ToughnessB': '韧性 B',
      'ToughnessC': '韧性 C',
      'XPExecutedBounty': '处决经验奖励',
      'XPKillOrDefeatBounty': '击杀或击败经验奖励',
      'SpeedModifier': '速度修正',
      'CriticalLevelPercent': '临界等级（%）',
      'MaxOxygen': '最大氧气量',
      'Oxygen': '氧气量',
      'OxygenDepletionRate': '氧气消耗速率',
      'OxygenRecoveryRate': '氧气恢复速率',
      'MaxRestTime': '最大休息时间',
      'MaxSleepTime': '最大睡眠时间',
      'SleepTime': '睡眠时间',
      'SleepTimeRecoveryAmount': '睡眠恢复量',
      'SleepTimeRecoveryPeriod': '睡眠恢复周期',
      'MaxSwampweed': '最大沼泽草量',
      'Swampweed': '沼泽草',
      'SwampweedDepletionRate': '沼泽草消耗速率',
      'other': '$fallback',
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
}
