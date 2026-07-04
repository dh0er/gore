// ignore: unused_import
import 'package:intl/intl.dart' as intl;
import 'app_localizations.dart';

// ignore_for_file: type=lint

/// The translations for Chinese (`zh`).
class AppLocalizationsZh extends AppLocalizations {
  AppLocalizationsZh([String locale = 'zh']) : super(locale);

  @override
  String get appTitle => 'Gothic Remake 存档编辑器';

  @override
  String get appLogoSemanticLabel => 'goresave 标志';

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
  String get skillsUnavailableBody => '此存檔無法編輯技能——主角沒有可修改的效果資料。';

  @override
  String get skillNotLearned => '未学习';

  @override
  String get skillLearn => '学习';

  @override
  String get skillActionLearn => '学习';

  @override
  String get skillActionUnlearn => '遗忘';

  @override
  String get skillTierUntrained => '一级';

  @override
  String get skillTierBeginner => '初學者';

  @override
  String get skillTierTrained => '二级';

  @override
  String get skillTierMaster => '三级';

  @override
  String get skillTierNovice => '熟练';

  @override
  String get skillTierAmateur => '初学者（第 0 环）';

  @override
  String get skillTierLearned => '已学习';

  @override
  String skillTierCircle(int n) {
    return '第 $n 环';
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
  String get skillNameOneHanded => '单手武器';

  @override
  String get skillNameTwoHanded => '双手武器';

  @override
  String get skillNameFists => '拳斗';

  @override
  String get skillNameBow => '弓';

  @override
  String get skillNameCrossbow => '弩';

  @override
  String get skillNameLockpicking => '开锁';

  @override
  String get skillNamePickpocketing => '扒窃';

  @override
  String get skillNameTakeOrgans => '摘取器官';

  @override
  String get skillNameBreakTeeth => '敲取牙齿';

  @override
  String get skillNameTakeClaws => '摘取利爪';

  @override
  String get skillNameSkinFur => '剥取毛皮';

  @override
  String get skillNameSkin => '剥皮';

  @override
  String get skillNameTakeFins => '摘取鱼鳍';

  @override
  String get skillNameTakeStingers => '摘取毒刺';

  @override
  String get skillNameTakeSecretion => '采集分泌物';

  @override
  String get skillNameTakeSkullPlates => '摘取头骨板';

  @override
  String get skillNameSkinSwampshark => '剥取沼泽鲨皮';

  @override
  String get skillNameTakeMinecrawlerPlates => '摘取矿虫甲板';

  @override
  String get skillNameTakeScutes => '摘取甲鳞';

  @override
  String get skillNameTakeUluMulu => '摘取乌鲁-穆鲁战利品';

  @override
  String get skillNameAcrobatics => '杂技';

  @override
  String get skillNameWallClimbing => '攀墙';

  @override
  String get skillNameRiding => '骑术';

  @override
  String get skillNameSneaking => '潜行';

  @override
  String get skillNameAlchemy => '炼金术';

  @override
  String get skillNameRuneInscription => '符文铭刻';

  @override
  String get skillNameBlacksmithing => '锻造';

  @override
  String get skillNameMagicCircle => '魔法环';

  @override
  String get skillNameOrcish => '兽人语';

  @override
  String get tabInventory => '物品栏';

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
  String profileWithSaves(String name, int count) {
    return '$name（$count 个存档）';
  }

  @override
  String get switchProfile => '切换存档配置';

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
  String get diagnosticsTitle => '诊断与详情';

  @override
  String get diagnosticsSubtitle => '只读格式检查';

  @override
  String get metricFormat => '格式';

  @override
  String get metricSlot => '槽位';

  @override
  String get metricChapter => '章节';

  @override
  String get metricTimePlayed => '游戏时长';

  @override
  String get metricSaveKind => '存档类型';

  @override
  String get metricFileSize => '文件大小';

  @override
  String get metricCompression => '压缩';

  @override
  String get metricChunks => '数据块';

  @override
  String get metricUncompressed => '未压缩';

  @override
  String get metricPrivate => '私有';

  @override
  String get metricSlotName => '槽位名称';

  @override
  String get metricTrailer => '尾部数据';

  @override
  String get metricDecodedPrivate => '已解码私有数据';

  @override
  String get metricPrivateStrings => '私有字符串';

  @override
  String get metricSha1 => 'SHA-1';

  @override
  String bytesValue(String count) {
    return '$count 字节';
  }

  @override
  String get inspectionJsonTitle => '检查 JSON';

  @override
  String get inspectionJsonSubtitle => '原始存档检查数据';

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
  String get required => '必填';

  @override
  String get playerLockedBody => '编辑私有玩家数据需要支持压缩的编解码器。';

  @override
  String get heroTransform => '主角变换';

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
  String get duplicateEvent => '复制事件';

  @override
  String get duplicateMemoryEventTitle => '复制记忆事件？';

  @override
  String get duplicateMemoryEventBody => '复制此记忆事件？将先写入一份备份。';

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
  String get factionGuildOldCamp => '旧营';

  @override
  String get factionGuildNewCamp => '新营';

  @override
  String get factionGuildSwampCamp => '沼泽营';

  @override
  String get factionGuildOther => '其他/个人';

  @override
  String get allDataLockedBody => '完整的属性浏览器需要来自编解码器的已解码私有负载数据。';

  @override
  String get allDataDescription =>
      '按名称或路径搜索每个类型化属性。标量、字符串、枚举和对象路径可编辑；结构体目前以只读方式显示。';

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
  String get aboutSubtitle => 'Gothic Remake 存档编辑器';

  @override
  String aboutVersion(String version, String sha) {
    return '版本 $version（$sha）';
  }

  @override
  String get aboutCopyright => '© 2026 goresave 贡献者';

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
  String get presetNovice => '新手';

  @override
  String get presetGothic => 'Gothic';

  @override
  String get presetHard => '困难';

  @override
  String get presetCustom => '自定义';

  @override
  String unrecognisedPreset(Object preset) {
    return '存储的预设无法识别（$preset）。你仍可保存流畅助手 / 永久死亡的更改，或在上方选择一个预设以覆盖它。';
  }

  @override
  String get closeCombatFlowHelper => '近战流畅助手';

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
  String get heroEntryHeroTransform => '主角变换';

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
  String localizedTextExtractedCount(int idCount, int languageCount) {
    return '已提取 $idCount 个 ID，涵盖 $languageCount 种语言';
  }
}

/// The translations for Chinese, using the Han script (`zh_Hans`).
class AppLocalizationsZhHans extends AppLocalizationsZh {
  AppLocalizationsZhHans() : super('zh_Hans');

  @override
  String get appTitle => 'Gothic Remake 存档编辑器';

  @override
  String get appLogoSemanticLabel => 'goresave 标志';

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
  String get skillTierUntrained => '未受训';

  @override
  String get skillTierBeginner => '初学者';

  @override
  String get skillTierTrained => '已受训';

  @override
  String get skillTierMaster => '大师';

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
  String get skillNameOneHanded => '单手武器';

  @override
  String get skillNameTwoHanded => '双手武器';

  @override
  String get skillNameFists => '拳斗';

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
  String get skillNameBreakTeeth => '敲取獠牙';

  @override
  String get skillNameTakeClaws => '摘取利爪';

  @override
  String get skillNameSkinFur => '剥取兽皮';

  @override
  String get skillNameSkin => '剥皮';

  @override
  String get skillNameTakeFins => '摘取鱼鳍';

  @override
  String get skillNameTakeStingers => '摘取毒刺';

  @override
  String get skillNameTakeSecretion => '采集分泌物';

  @override
  String get skillNameTakeSkullPlates => '摘取颅骨甲片';

  @override
  String get skillNameSkinSwampshark => '剥取沼泽鲨皮';

  @override
  String get skillNameTakeMinecrawlerPlates => '摘取矿虫甲片';

  @override
  String get skillNameTakeScutes => '摘取角质甲片';

  @override
  String get skillNameTakeUluMulu => '采集乌鲁-穆鲁战利品';

  @override
  String get skillNameAcrobatics => '杂技';

  @override
  String get skillNameWallClimbing => '攀墙';

  @override
  String get skillNameRiding => '骑术';

  @override
  String get skillNameSneaking => '潜行';

  @override
  String get skillNameAlchemy => '炼金';

  @override
  String get skillNameRuneInscription => '符文铭刻';

  @override
  String get skillNameBlacksmithing => '锻造';

  @override
  String get skillNameMagicCircle => '魔法环';

  @override
  String get skillNameOrcish => '兽人语';

  @override
  String get tabInventory => '物品栏';

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
  String profileWithSaves(String name, int count) {
    return '$name（$count 个存档）';
  }

  @override
  String get switchProfile => '切换存档配置';

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
  String get diagnosticsTitle => '诊断与详情';

  @override
  String get diagnosticsSubtitle => '只读格式检查';

  @override
  String get metricFormat => '格式';

  @override
  String get metricSlot => '槽位';

  @override
  String get metricChapter => '章节';

  @override
  String get metricTimePlayed => '游戏时长';

  @override
  String get metricSaveKind => '存档类型';

  @override
  String get metricFileSize => '文件大小';

  @override
  String get metricCompression => '压缩';

  @override
  String get metricChunks => '数据块';

  @override
  String get metricUncompressed => '未压缩';

  @override
  String get metricPrivate => '私有';

  @override
  String get metricSlotName => '槽位名称';

  @override
  String get metricTrailer => '尾部数据';

  @override
  String get metricDecodedPrivate => '已解码私有数据';

  @override
  String get metricPrivateStrings => '私有字符串';

  @override
  String get metricSha1 => 'SHA-1';

  @override
  String bytesValue(String count) {
    return '$count 字节';
  }

  @override
  String get inspectionJsonTitle => '检查 JSON';

  @override
  String get inspectionJsonSubtitle => '原始存档检查数据';

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
  String get required => '必填';

  @override
  String get playerLockedBody => '编辑私有玩家数据需要支持压缩的编解码器。';

  @override
  String get heroTransform => '主角变换';

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
  String get duplicateEvent => '复制事件';

  @override
  String get duplicateMemoryEventTitle => '复制记忆事件？';

  @override
  String get duplicateMemoryEventBody => '复制此记忆事件？将先写入一份备份。';

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
  String get factionGuildOldCamp => '旧营';

  @override
  String get factionGuildNewCamp => '新营';

  @override
  String get factionGuildSwampCamp => '沼泽营';

  @override
  String get factionGuildOther => '其他/个人';

  @override
  String get allDataLockedBody => '完整的属性浏览器需要来自编解码器的已解码私有负载数据。';

  @override
  String get allDataDescription =>
      '按名称或路径搜索每个类型化属性。标量、字符串、枚举和对象路径可编辑；结构体目前以只读方式显示。';

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
  String get aboutSubtitle => 'Gothic Remake 存档编辑器';

  @override
  String aboutVersion(String version, String sha) {
    return '版本 $version（$sha）';
  }

  @override
  String get aboutCopyright => '© 2026 goresave 贡献者';

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
  String get presetNovice => '新手';

  @override
  String get presetGothic => 'Gothic';

  @override
  String get presetHard => '困难';

  @override
  String get presetCustom => '自定义';

  @override
  String unrecognisedPreset(Object preset) {
    return '存储的预设无法识别（$preset）。你仍可保存流畅助手 / 永久死亡的更改，或在上方选择一个预设以覆盖它。';
  }

  @override
  String get closeCombatFlowHelper => '近战流畅助手';

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
  String get heroEntryHeroTransform => '主角变换';

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
  String localizedTextExtractedCount(int idCount, int languageCount) {
    return '已提取 $idCount 个 ID，涵盖 $languageCount 种语言';
  }
}
