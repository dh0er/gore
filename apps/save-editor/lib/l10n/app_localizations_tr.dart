// ignore: unused_import
import 'package:intl/intl.dart' as intl;
import 'app_localizations.dart';

// ignore_for_file: type=lint

/// The translations for Turkish (`tr`).
class AppLocalizationsTr extends AppLocalizations {
  AppLocalizationsTr([String locale = 'tr']) : super(locale);

  @override
  String get debugSectionTitle => 'Gelişmiş (hata ayıklama)';

  @override
  String get debugSectionSubtitle => 'Hata raporları için tanılama ve ham veri';

  @override
  String get showObjectIdsTitle => 'Ek teknik kimlikleri göster';

  @override
  String get showObjectIdsSubtitle =>
      'Düzenleyicide teknik eşya, diyalog bilgisi, görev ve yetim aktör kimliklerini göster. NPC kimlikleri her zaman gösterilir.';

  @override
  String get storyStateSidebar => 'Hikâye durumu';

  @override
  String get storyStateDescription =>
      'Oyun görevler, diyaloglar ve olaylardaki ilerlemeyi burada tutar. «Kayıtlı», kaydındaki değerleri; «Ayarlanmamış» ise diğer bilinen girdileri gösterir. Girdiye göre bir sayı evet/hayır, bir adet veya bir ilerleme aşaması anlamına gelebilir. Zaman işaretleri oyunda bir gün ve saati gösterir.';

  @override
  String get storyStateReadOnly =>
      'Değerlerin script anlamı ve güvenli map yazımları netleşene kadar salt okunur. İlgili sözlük metni bağlam içindir; teknik kimliğin doğrudan çevirisi değildir.';

  @override
  String get storyStateStructureReadOnly =>
      'Bu kayıttaki StoryPropertyValues yapısı benzersiz ve güvenli biçimde çözülemedi. Hikâye değerleri bu kayıt için salt okunur kalır.';

  @override
  String get storyStateSearch => 'Hikâye durumunda ara';

  @override
  String storyStateValuesCount(int shown, int total) {
    return '$shown / $total hikâye değeri';
  }

  @override
  String get storyStateInteger => 'Tamsayı';

  @override
  String get storyStateTimeMarker => 'Zaman işareti';

  @override
  String get storyStateChapter => 'Bölüm';

  @override
  String get storyStateUnknown => 'Bilinmeyen kaynak türü';

  @override
  String storyStateShowDormant(int count) {
    return 'Kullanılmayanları göster ($count)';
  }

  @override
  String get storyStateUnknownDetail =>
      'Bu kayıtlı kimlik güncel script kataloğunda yok (örneğin bir mod veya daha yeni oyun sürümünden). Kayıttaki ham değer int32, ancak anlamı çıkarılmaz.';

  @override
  String get storyStateStored => 'Kayıtlı';

  @override
  String get storyStateUnset => 'Ayarlanmamış';

  @override
  String get storyStateUnsetDetail =>
      'Bu katalog alanı bu kayıtta serileştirilmemiş; oyun bu yüzden ayarlanmamış veya varsayılan durumu kullanır.';

  @override
  String get storyStateRawValue => 'Ham değer';

  @override
  String storyStateElapsed(String duration) {
    return 'Kayıt anında geçen: $duration';
  }

  @override
  String storyStateAhead(String duration) {
    return 'Kayıt zamanının önünde: $duration';
  }

  @override
  String storyStateDurationDays(int days, String time) {
    String _temp0 = intl.Intl.pluralLogic(
      days,
      locale: localeName,
      other: '$days gün',
      one: '1 gün',
    );
    return '$_temp0 $time';
  }

  @override
  String get storyStateRelatedGlossary => 'İlgili sözlük girdisi';

  @override
  String get storyStateTechnicalPath => 'Teknik yol';

  @override
  String get storyStateEditingGuidance =>
      'Değerini değiştirmek için bir girdi seç. Değişiklikler kaydettiğinde geçerli olur. Yalnızca etkisini anladığın değerleri değiştir: aksi halde görevler veya diyaloglar beklendiği gibi çalışmayabilir. Kaydettiğinde otomatik yedek oluşturulur.';

  @override
  String get storyStatePending => 'Beklemede';

  @override
  String storyStatePendingValue(String value) {
    return '$value olarak kaydedilecek';
  }

  @override
  String get storyStatePendingRemoval => 'Kayıttan kaldırılacak';

  @override
  String get storyStateEditValue => 'Değeri düzenle';

  @override
  String get storyStateSetValue => 'Değer ata';

  @override
  String get storyStateRemoveValue => 'Kayıttan kaldır';

  @override
  String get storyStateUndoChange => 'Hikâye değişikliğini geri al';

  @override
  String get storyStateResetChanges => 'Hikâye değişikliklerini sıfırla';

  @override
  String storyStateDialogTitle(String id) {
    return '$id düzenle';
  }

  @override
  String get storyStateRawInput => 'İşaretli int32 değeri';

  @override
  String get storyStateInvalidInt32 =>
      '-2147483648 ile 2147483647 arasında tam bir sayı gir.';

  @override
  String get storyStateQueueChange => 'Değişikliği sıraya al';

  @override
  String storyStateSuggestedValues(String values) {
    return 'Dağıtılan scriptlerde görülen değerler: $values';
  }

  @override
  String get storyStateSuggestionsNotLimits =>
      'Öneriler doğrulama sınırı değildir; yerel kod, modlar veya sonraki oyun sürümleri başka değerler kullanabilir.';

  @override
  String get storyStateUseCurrentTime => 'Geçerli kayıt zamanını kullan';

  @override
  String get storyStateStructuredTime => 'Gün / saat';

  @override
  String get storyStateRawMode => 'Ham int32';

  @override
  String get storyStateChapterWarning =>
      'Yalnızca bölümü değiştirmek görevleri, NPC\'leri, envanteri veya dünya durumunu eşitlemez.';

  @override
  String get storyStateDormantWarning =>
      'Dağıtılan script önbelleğinde bu alan için canlı okuma veya yazma bulunamadı. Eski, yerel kodla yönetilen veya ayrılmış olabilir.';

  @override
  String get storyStateReadOnlySourceWarning =>
      'Dağıtılan scriptler bu alanı okur ancak script yazımı içermez. Yerel kod yine de sahibi olabilir.';

  @override
  String get storyStateUnknownEditWarning =>
      'Bu modlu veya daha yeni sürüm kimliğinin paketlenmiş kaynak anlamı yok. Yalnızca ham int32 değerini düzenle.';

  @override
  String storyStateIntegerKind(String kind) {
    String _temp0 = intl.Intl.selectLogic(kind, {
      'binaryFlag': 'İkili bayrak',
      'finiteState': 'Çok durumlu değer',
      'counterOrScore': 'Sayaç / puan',
      'calendarDay': 'Takvim günü',
      'derivedOrOpaqueInteger': 'Türetilmiş / opak tamsayı',
      'readOnlyInSourceInteger': 'Dağıtılan scriptlerde salt okunur',
      'dormantOrLegacyInteger': 'Dağıtılan scriptlerde kullanılmıyor',
      'other': 'Tamsayı',
    });
    return '$_temp0';
  }

  @override
  String get storyStateZeroVsUnset =>
      'Kayıtlı 0 ile eksik map girdisi farklı dosya durumlarıdır. «Kayıttan kaldır» yapıcı/varsayılan durumu geri yükler.';

  @override
  String get appTitle => 'GORE Save Editor';

  @override
  String get appLogoSemanticLabel => 'GORE Save Editor logosu';

  @override
  String get zoomTooltip =>
      'Yakınlaştırmak/uzaklaştırmak için Ctrl +/- tuşlarına bas';

  @override
  String get switchToLightMode => 'Açık moda geç';

  @override
  String get switchToDarkMode => 'Koyu moda geç';

  @override
  String get about => 'Hakkında';

  @override
  String get tabOverview => 'Genel bakış';

  @override
  String get tabPlayer => 'Oyuncu';

  @override
  String get tabAttribute => 'Nitelikler';

  @override
  String get heroGroupSkills => 'Beceriler';

  @override
  String get skillsNoneBody => 'Bu karakter için beceri bulunamadı.';

  @override
  String get skillsUnavailableBody =>
      'Bu kayıtta beceriler düzenlenemiyor — karakterde değiştirilecek etki verisi yok.';

  @override
  String get skillNotLearned => 'Öğrenilmedi';

  @override
  String get skillLearn => 'Öğren';

  @override
  String get skillActionLearn => 'öğren';

  @override
  String get skillActionUnlearn => 'unut';

  @override
  String get skillTierUntrained => 'Eğitimsiz';

  @override
  String get skillTierBeginner => 'Başlangıç';

  @override
  String get skillTierTrained => 'Eğitilmiş';

  @override
  String get skillTierMaster => 'Usta';

  @override
  String get skillTierNovice => 'Acemi';

  @override
  String get skillTierAmateur => 'Amatör (Daire 0)';

  @override
  String get skillTierLearned => 'Öğrenildi';

  @override
  String skillTierCircle(int n) {
    return 'Daire $n';
  }

  @override
  String get skillHintBlacksmith1H => '1H silahlar';

  @override
  String get skillHintBlacksmith2H => '2H silahlar';

  @override
  String get skillScutesTrained => 'Eğitilmiş (kemik pulları)';

  @override
  String get skillScutesMaster => 'Usta (+ jilet plakalar)';

  @override
  String get skillCategoryCombat => 'Savaş';

  @override
  String get skillCategoryCrafting => 'Zanaat';

  @override
  String get skillCategoryHunting => 'Avcılık';

  @override
  String get skillCategoryLanguage => 'Dil';

  @override
  String get skillCategoryMagic => 'Büyü';

  @override
  String get skillCategoryMovement => 'Hareket';

  @override
  String get skillCategoryThievery => 'Hırsızlık';

  @override
  String get skillCategoryOther => 'Diğer';

  @override
  String get skillNameOneHanded => 'Tek el';

  @override
  String get skillNameTwoHanded => 'Çift el';

  @override
  String get skillNameFists => 'Yumruk';

  @override
  String get skillNameBow => 'Yay';

  @override
  String get skillNameCrossbow => 'Arbalet';

  @override
  String get skillNameLockpicking => 'Kilit açma';

  @override
  String get skillNamePickpocketing => 'Yankesicilik';

  @override
  String get skillNameTakeOrgans => 'Organ çıkar';

  @override
  String get skillNameBreakTeeth => 'Diş çıkar';

  @override
  String get skillNameTakeClaws => 'Pençe çıkar';

  @override
  String get skillNameSkinFur => 'Kürk al';

  @override
  String get skillNameSkin => 'Deri al';

  @override
  String get skillNameTakeFins => 'Yüzgeç al';

  @override
  String get skillNameTakeStingers => 'İğne çıkar';

  @override
  String get skillNameTakeSecretion => 'Salgı çıkar';

  @override
  String get skillNameTakeSkullPlates => 'Kafatası zırhı al';

  @override
  String get skillNameSkinSwampshark => 'Bataklık köpekbalığı derisi al';

  @override
  String get skillNameTakeMinecrawlerPlates => 'Plaka al';

  @override
  String get skillNameTakeScutes => 'Pul al';

  @override
  String get skillNameTakeUluMulu => 'Ulu-Mulu al';

  @override
  String get skillNameOrcWeapons => 'Ork silahları';

  @override
  String get skillNameMining => 'Madencilik';

  @override
  String get skillNameDiving => 'Dalış';

  @override
  String get skillNameTakeMinecrawlerMandibles => 'Mandibula çıkar';

  @override
  String get skillNameTakeShadowbeastHorn => 'Boynuz al (Shadowbeast)';

  @override
  String get skillNameTakeSpines => 'Omurga çıkar';

  @override
  String get skillNameBreakSwampsharkTeeth => 'Köpekbalığı dişi çıkar';

  @override
  String get skillNameTakeFireTongue => 'Ateş dili al';

  @override
  String get skillNameTakeTrollHorn => 'Boynuz al (Troll)';

  @override
  String get skillNameAcrobatics => 'Akrobasi';

  @override
  String get skillNameWallClimbing => 'Tırmanma';

  @override
  String get skillNameRiding => 'Scavenger sürme';

  @override
  String get skillNameSneaking => 'Gizlenme';

  @override
  String get skillNameAlchemy => 'Simya';

  @override
  String get skillNameRuneInscription => 'Yazıt';

  @override
  String get skillNameBlacksmithing => 'Demircilik';

  @override
  String get skillNameMagicCircle => 'Büyü dairesi';

  @override
  String get skillNameOrcish => 'Orkça';

  @override
  String get tabInventory => 'Envanter';

  @override
  String get tabTrade => 'Ticaret';

  @override
  String get traderNotAMerchant => 'Bu karakter ticaret yapmıyor.';

  @override
  String get traderRetry => 'Yeniden dene';

  @override
  String get traderAmbiguousName =>
      'Bu adı taşıyan birden fazla tüccar kaydı var; düzenleyici hangi dükkânın bu karaktere ait olduğunu ayırt edemiyor. Yanlış kaydı değiştirme riskine karşı düzenleme kapalı.';

  @override
  String get traderOre => 'Cevher (alım gücü)';

  @override
  String get traderNoOre => 'cevher yok';

  @override
  String get traderStockCurrent => 'Stok';

  @override
  String get traderStockCurrentTooltip =>
      'Bu tüccarın şu an satışa sundukları. Oyun tüccarı güncellediğinde eklenen eşyalar yeniden kaybolabilir.';

  @override
  String get traderStockBase => 'Yenileme tabanı';

  @override
  String get traderStockBaseTooltip =>
      'Kayıt, oyunun tüccarı yeniden stoklamasına yardımcı olmak için bu listeyi içerir. Oyun tüccar kurallarına göre yeniden hesaplayabilir; buradaki değişiklikler kalıcı olmaz.';

  @override
  String get traderStockBaseHint =>
      'Salt okunur: oyun yeniden stoklarken bu listeyi kullanır ancak yeniden hesaplayabilir. Buraya eklenen eşyalar kalıcı kalmaz.';

  @override
  String get traderCurrentStockWarning =>
      'Tüccar envanterindeki değişiklikler yalnızca bir sonraki yeniden stoklamaya kadar geçerlidir.';

  @override
  String get traderRestockTitle => 'Yeniden stoklama zamanlayıcısı';

  @override
  String get traderRestockTitleTooltip =>
      'Tüccarın son etkinliği, geçerli oyun zamanı ve Kaynaklar zorluğuna dayalı bir tahmin.';

  @override
  String get traderRestockPending => 'beklemede';

  @override
  String get traderRestockRevertTooltip =>
      'Bekleyen zaman değişikliğini geri al';

  @override
  String get traderRestockNever => 'Asla';

  @override
  String get traderRestockUnavailable => 'Kullanılamıyor';

  @override
  String get traderRestockIntervalUnknown =>
      'Yeniden stoklama beklemesi bilinmiyor';

  @override
  String get traderRestockNeverStatus =>
      'Henüz tüccar etkinliği kaydedilmemiş.';

  @override
  String get traderRestockClockAhead =>
      'Tüccarın kayıtlı zamanı geçerli oyun zamanının önünde.';

  @override
  String traderRestockNotDueYet(String time) {
    return '$time öncesinde beklenmiyor.';
  }

  @override
  String get traderRestockPossiblyDue =>
      'Tüccar yeniden stoklama için hazır olabilir.';

  @override
  String get traderRestockEligible =>
      'Tüccar artık yeniden stoklama için hazır olmalı.';

  @override
  String get traderRestockNoWorldTime =>
      'Geçerli oyun zamanı yok; düzenleyici yeniden stoklamanın vadesi gelip gelmediğini söyleyemiyor.';

  @override
  String get traderRestockLastActivity => 'Son tüccar etkinliği';

  @override
  String get traderRestockLastActivityTooltip =>
      'Bu tüccar için kaydedilen son zaman. Ticaretten veya başka bir tüccar güncellemesinden gelebilir; son yeniden stoklama olması gerekmez.';

  @override
  String get traderRestockForecastWindow => 'Yeniden stoklama bekleniyor';

  @override
  String get traderRestockForecastWindowTooltip =>
      'Tam zaman kayıtta saklanmaz. Düzenleyici bu yüzden en erken ile en geç beklenen zaman arasında bir aralık gösterir.';

  @override
  String get traderRestockIntervalLabel => 'Yeniden stoklama beklemesi';

  @override
  String traderRestockInterval(int days, String level) {
    return '$days gün · $level';
  }

  @override
  String get traderRestockIntervalTooltip =>
      'Kaynaklar zorluğuna göre bekleme: Acemi 2, Gothic 3, Zor 5 oyun günü.';

  @override
  String get traderRestockAutomationLabel => 'Otomatik yeniden stoklama';

  @override
  String get traderRestockAutomationValue => 'Kayıtta devre dışı bırakılamaz';

  @override
  String get traderRestockAutomationTooltip =>
      'Kayıt düzenleyici otomatik yeniden stoklamayı güvenilir biçimde durduramaz. Bunun için bir oyun modu gerekir.';

  @override
  String get traderRestockSetNow => 'Dünya zamanına ayarla';

  @override
  String get traderRestockSetNowTooltip =>
      'Geçerli oyun zamanını tüccarın son etkinliği olarak kullan. Bu, bir sonraki beklenen yeniden stoklamayı erteler.';

  @override
  String get traderRestockMakeDue => 'Şimdi vadesi gelsin';

  @override
  String get traderRestockMakeDueTooltip =>
      'Tüccarın son etkinliğini yeniden stoklamanın şimdi vadesi gelmiş olacağı kadar geriye al.';

  @override
  String get traderRestockCustom => 'Özel zaman…';

  @override
  String get traderRestockCustomTooltip =>
      'Tüccarın son etkinliğinin oyun içi gün ve saatini seç.';

  @override
  String get traderRestockEditTitle => 'Son tüccar etkinliğini değiştir';

  @override
  String get traderOreHint =>
      'Oyun içi rakam farklıdır: yüklemede oyun son ticaretinden bu yana birikeni ekler — fazla mal satar ve stokları ondan doldurur. Bu sayı başlangıç noktasıdır; ticaret ekranındaki tutar değildir.';

  @override
  String get traderOreHintShort =>
      'Başlangıç değeri — ticaret ekranındaki tutar farklı olabilir.';

  @override
  String get traderRestockStatusLabel => 'Durum';

  @override
  String get traderRestockStatusNever => 'Etkinlik yok';

  @override
  String get traderRestockStatusWaiting => 'Yeniden stoklama bekleniyor';

  @override
  String get traderRestockStatusReady => 'Yeniden stoklamaya hazır';

  @override
  String get traderRestockStatusPossiblyReady => 'Muhtemelen hazır';

  @override
  String get traderRestockStatusCheckTime => 'Kayıtlı zamanı kontrol et';

  @override
  String get traderRestockStatusUnknown => 'Bilinmiyor';

  @override
  String get traderPriceWarning =>
      'Fiyatlar tüccarın ne kadar stokladığına ve ne kadar cevheri olduğuna tepki verir; bu sayıları değiştirmek ücretlerini de oynatabilir.';

  @override
  String get traderAddItem => 'Eşya ekle';

  @override
  String get traderRemoveItem => 'Satırı kaldır';

  @override
  String get traderReadOnlyCore =>
      'Bu çekirdek yapı yalnızca tüccar verisini okuyabilir.';

  @override
  String get traderDifficultyStockUnsupported =>
      'Bu tüccar zorluk başına stok taşır; düzenleyici bunu modellemiyor. Değişiklik başarılı görünürken ek stok dokunulmadan kalacağı için düzenleme kapalı.';

  @override
  String get traderRecordIncomplete =>
      'Bu tüccarın stok listeleri eksik veya düzenleyicinin destekleyip yazamadığı bir biçimde. Kayıt anında hata olmasın diye düzenleme kapalı.';

  @override
  String get traderEmptyStock => 'Stokta bir şey yok.';

  @override
  String get traderUnknownItem => 'eşya kataloğunda yok';

  @override
  String editorTradersLoadFailed(String details) {
    return 'Tüccar yüklemesi başarısız: $details';
  }

  @override
  String traderStockLineCount(int count) {
    return '$count satır';
  }

  @override
  String get tabWorld => 'Dünya';

  @override
  String get tabCharacters => 'Karakterler';

  @override
  String get characterNoActorBody =>
      'Bu karakterin dünyada aktörü yok; nitelik, envanter veya olayları da yok.';

  @override
  String get characterNoEventsBody => 'Bu karakter için olay yok.';

  @override
  String get characterOrphanGroup => 'Diğer';

  @override
  String get tabAllData => 'Tüm veriler';

  @override
  String get tabBackups => 'Yedekler';

  @override
  String get tabSettings => 'Ayarlar';

  @override
  String get reset => 'Sıfırla';

  @override
  String get save => 'Kaydet';

  @override
  String saveWithCount(int count) {
    return 'Kaydet ($count)';
  }

  @override
  String get ok => 'Tamam';

  @override
  String get cancel => 'İptal';

  @override
  String get confirm => 'Onayla';

  @override
  String get close => 'Kapat';

  @override
  String get add => 'Ekle';

  @override
  String get equippedBadge => 'Kuşanılmış';

  @override
  String get armorUpgradesLabel => 'Yükseltmeler';

  @override
  String get browse => 'Gözat';

  @override
  String get noSavFilesFound => '.sav dosyası bulunamadı';

  @override
  String get profile => 'Profil';

  @override
  String get otherSaves => 'Diğer kayıtlar';

  @override
  String profileWithSaves(String name, int count) {
    return '$name ($count kayıt)';
  }

  @override
  String get switchProfile => 'Profili değiştir';

  @override
  String get openSaveFile => 'Dosyayı aç';

  @override
  String get externalSave => 'Dışarıdan açılan kayıt';

  @override
  String get saveProfileTitle => 'Kayıt profili';

  @override
  String get saveProfileDescription =>
      'Bu kaydı farklı bir oyun profiline ata. Kayıt ve profil dizini birlikte yedeklenir.';

  @override
  String get saveProfileExternalHint =>
      'Bu dosyayı oyunun kayıt klasörüne aktarmak için bir profil seç; orada kayıtlı olur. Orijinal dosya değişmez.';

  @override
  String get saveProfileNoProfiles =>
      'PersistentDataList.sav içinde düzenlenebilir oyun profili bulunamadı.';

  @override
  String get saveProfileSelect => 'Profil seç';

  @override
  String get rescanSaveFolder => 'Kayıt klasörünü yeniden tara';

  @override
  String get discardUnsavedChangesTitle =>
      'Kaydedilmemiş değişiklikler silinsin mi?';

  @override
  String rescanDiscardBody(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: 'değişikliği',
      one: 'değişikliği',
    );
    return 'Yeniden tarama tüm kayıtları yeniden yükler ve kaydedilmemiş $count $_temp0 siler.';
  }

  @override
  String get discardAndRescan => 'Sil ve yeniden tara';

  @override
  String chapterLabel(Object id) {
    return 'Bölüm $id';
  }

  @override
  String get quickSave => 'Hızlı kayıt';

  @override
  String get autoSave => 'Otomatik kayıt';

  @override
  String get manualSave => 'Manuel kayıt';

  @override
  String get errorTitle => 'Hata';

  @override
  String get selectASaveTitle => 'Kayıt seç';

  @override
  String get selectASaveBody => 'Kayıt ayrıntıları burada görünür.';

  @override
  String bytesValue(String count) {
    return '$count bayt';
  }

  @override
  String get inspectionJsonTitle => 'İnceleme JSON';

  @override
  String get copy => 'Kopyala';

  @override
  String get savegameFallbackTitle => 'Kayıt';

  @override
  String screenshotForSlot(String slot) {
    return '$slot için ekran görüntüsü';
  }

  @override
  String get publicSaveName => 'Ad';

  @override
  String get gameTimeTitle => 'Oyun zamanı';

  @override
  String get gameTimeDay => 'Gün';

  @override
  String get gameTimeHours => 'Saat';

  @override
  String get gameTimeMinutes => 'Dakika';

  @override
  String get gameTimeSeconds => 'Saniye';

  @override
  String gameTimeTotal(int seconds) {
    return '= toplam $seconds sn';
  }

  @override
  String get gameTimeInvalid =>
      'Tam sayı gir — gün ≥ 0, saat 0–23, dakika ve saniye 0–59.';

  @override
  String get required => 'Zorunlu';

  @override
  String get playerLockedBody =>
      'Özel oyuncu düzenlemeleri sıkıştırmaya hazır bir codec gerektirir.';

  @override
  String get heroTransform => 'Konum';

  @override
  String get locationX => 'Konum X';

  @override
  String get locationY => 'Konum Y';

  @override
  String get locationZ => 'Konum Z';

  @override
  String get rotationPitch => 'Eğim (pitch)';

  @override
  String get rotationYaw => 'Sapma (yaw)';

  @override
  String get rotationRoll => 'Yuvarlanma (roll)';

  @override
  String get spawnPositionSection => 'Doğuş konumu (referans)';

  @override
  String get resetToSpawnPosition => 'Doğuş konumuna sıfırla';

  @override
  String get positionOutOfRange =>
      'Değer −10.000.000 ile 10.000.000 arasında olmalı';

  @override
  String get positionNotEditable =>
      'Bu karakter için kayıtlı konum okunamadı; düzenlenemez.';

  @override
  String get positionNeverPlaced =>
      'Bu karakter dünyaya hiç yerleştirilmemiş (konum 0, 0, 0) — oyun kayıtlı konumu yok sayabilir.';

  @override
  String get npcStayInPlace => 'Günlük rutinini devre dışı bırak';

  @override
  String get npcStayInPlaceHint => 'Böylece bulunduğu yerde kalır.';

  @override
  String get npcStayInPlaceLocked =>
      'Orijinal günlük rutini kaydedilmediği için artık geri alınamaz.';

  @override
  String get npcUndoPlacement => 'Taşımayı geri al';

  @override
  String get npcUndoPlacementStale =>
      'Kayıt o taşımanın yazdığı veriyi artık tutmuyor; geri yüklemek sonrasında olanları siler.';

  @override
  String get positionNotReadable => 'Bu karakter için kayıtlı konum okunamadı.';

  @override
  String get npcPositionReadOnly =>
      'Oyun NPC konumunu kayıttan değil seviyeden yükler; bu değerler okunabilir ama değiştirilemez.';

  @override
  String get pickLocation => 'Konum seç…';

  @override
  String get pickLocationDialogTitle => 'Konum seç';

  @override
  String get applySpotRotation => 'Noktanın yönünü de uygula';

  @override
  String get locationAreaOther => 'Diğer';

  @override
  String get locationAreaCavalornValley => 'Cavalorn Vadisi';

  @override
  String get locationAreaEastForest => 'Doğu Ormanı';

  @override
  String get locationAreaFogTower => 'Sis Kulesi';

  @override
  String get locationAreaIllegalWeedMixers => 'Kaçak Ot Karıştırıcıları';

  @override
  String get locationAreaOrcArena => 'Ork Arenası';

  @override
  String get locationAreaOrcGraveyard => 'Ork Mezarlığı';

  @override
  String get locationAreaShipwreck => 'Gem Enkazı';

  @override
  String get locationAreaTundra => 'Tundra';

  @override
  String get locationCatalogUnavailable => 'Konum kataloğu yüklenemedi.';

  @override
  String get invalid => 'Geçersiz';

  @override
  String get heroAttributes => 'Kahraman nitelikleri';

  @override
  String attributeBase(String name) {
    return '$name taban';
  }

  @override
  String attributeCurrent(String name) {
    return '$name güncel';
  }

  @override
  String get attributeBaseValue => 'Taban değer';

  @override
  String get attributeCurrentValue => 'Güncel değer';

  @override
  String get inventoryTitle => 'Envanter';

  @override
  String get inventoryEmpty => 'Bu envanter boş.';

  @override
  String get inventoryNeedsDecoded =>
      'Envanter düzenleme, codec\'ten çözülmüş özel yük verisi gerektirir.';

  @override
  String get inventoryNoStacks => 'Çözülmüş özel yükte eşya yığını bulunamadı.';

  @override
  String get resetInventoryChanges => 'Envanter değişikliklerini sıfırla';

  @override
  String get addItemTooltipPendingAdd =>
      'Önce beklemedeki değişiklikleri kaydet — kayıt başına yalnızca bir yeni eşya';

  @override
  String get addItemTooltipPendingRemove =>
      'Önce beklemedeki kaldırmayı kaydet — kayıt başına yalnızca bir yapısal değişiklik';

  @override
  String get addItemTooltipPendingCount =>
      'Önce beklemedeki adet değişikliklerini kaydet veya sıfırla — yapısal düzenleme tek başına kaydedilmeli';

  @override
  String get addItemTooltipDefault => 'Envantere eşya ekle';

  @override
  String get addItemButton => 'Eşya ekle';

  @override
  String get resetInventoryButton => 'Envanteri sıfırla';

  @override
  String get resetInventoryTooltipDefault =>
      'Bu envanteri oyun başlangıç kaydındaki envanterle değiştir';

  @override
  String get resetInventoryTooltipBlocked =>
      'Önce beklemedeki envanter değişikliklerini kaydet veya iptal et';

  @override
  String get pendingResetTitle => 'Oyun başlangıç envanterine sıfırla';

  @override
  String pendingResetSubtitle(String level) {
    return 'Kaynaklar zorluğu: $level';
  }

  @override
  String get cancelPendingReset => 'Sıfırlamayı iptal et';

  @override
  String pendingAddSubtitle(int count) {
    return '×$count — ekleme beklemede (henüz kaydedilmedi)';
  }

  @override
  String get cancelPendingAdd => 'Bekleyen eklemeyi iptal et';

  @override
  String get pendingRemovalSubtitle =>
      'kaldırma beklemede (henüz kaydedilmedi)';

  @override
  String get cancelPendingRemoval => 'Bekleyen kaldırmayı iptal et';

  @override
  String get filterItems => 'Eşyaları filtrele';

  @override
  String noItemsMatchQuery(String query) {
    return '\"$query\" ile eşleşen eşya yok.';
  }

  @override
  String get pendingRemovalHidesAll =>
      'Bekleyen kaldırma tüm eşyaları gizliyor — uygulamak için kaydet.';

  @override
  String categoryWithCount(String label, int count) {
    return '$label ($count)';
  }

  @override
  String get itemTooltipIngredientFor => 'Malzeme:';

  @override
  String itemTooltipTeaches(String item) {
    return 'Öğretir: $item';
  }

  @override
  String get itemTooltipValue => 'Değer';

  @override
  String get itemTooltipProtection => 'Koruma';

  @override
  String get itemTooltipRequirements => 'Gereksinimler:';

  @override
  String get itemTooltipManaCost => 'Mana maliyeti';

  @override
  String get itemTooltipManaUpkeep => 'Sürekli mana maliyeti';

  @override
  String get itemCategoryAll => 'Tümü';

  @override
  String get itemCategoryMeleeWeapon => 'Yakın dövüş silahları';

  @override
  String get itemCategoryRangedWeapon => 'Uzaktan silahlar';

  @override
  String get itemCategoryMagic => 'Büyü';

  @override
  String get itemCategoryWearable => 'Giysiler';

  @override
  String get itemCategoryFood => 'Yiyecek';

  @override
  String get itemCategoryPotion => 'İksirler';

  @override
  String get itemCategoryMaterial => 'Malzemeler';

  @override
  String get itemCategoryDocument => 'Belgeler';

  @override
  String get itemCategoryMisc => 'Çeşitli';

  @override
  String get itemCategoryArtefact => 'Artefaktlar';

  @override
  String get itemCategoryOther => 'Diğer';

  @override
  String get count => 'Adet';

  @override
  String get min1 => 'En az 1';

  @override
  String countTimes(String count) {
    return '×$count';
  }

  @override
  String get deleteEquippedTooltip =>
      'Silinemez: bu eşya muhtemelen kuşanılmış veya kısayol yuvasına atanmış';

  @override
  String get removeBlockedTooltip =>
      'Önce beklemedeki envanter değişikliklerini kaydet veya sıfırla — ekleme veya kaldırma tek başına kaydedilmeli';

  @override
  String get removeItemFromInventory => 'Eşyayı envanterden kaldır';

  @override
  String get progressionLockedBody =>
      'İlerleme verisi, codec\'ten çözülmüş özel yük verisi gerektirir.';

  @override
  String get progressionNeedsTyped =>
      'Yapılandırılmış ilerleme verisi, doğrulanmış tür ayrıştırması olan tam çözülmüş bir kayıt gerektirir.';

  @override
  String get sectionQuests => 'Görevler';

  @override
  String get sectionKnowledge => 'Bilgi';

  @override
  String get sectionEvents => 'Olaylar';

  @override
  String get firstPage => 'İlk sayfa';

  @override
  String get previousPage => 'Önceki sayfa';

  @override
  String get nextPage => 'Sonraki sayfa';

  @override
  String get lastPage => 'Son sayfa';

  @override
  String pageOfPages(int page, int total) {
    return 'Sayfa $page / $total';
  }

  @override
  String rangeOfTotal(int first, int last, int total) {
    return '$first–$last / $total';
  }

  @override
  String get perPage => 'Sayfa başına:';

  @override
  String get resetQuestChanges => 'Görev değişikliklerini sıfırla';

  @override
  String get searchQuests => 'Görev ara';

  @override
  String get allGroups => 'Tüm gruplar';

  @override
  String groupWithCount(String group, Object count) {
    return '$group ($count)';
  }

  @override
  String stateLabelWithCount(String label, int count) {
    return '$label $count';
  }

  @override
  String get questStateNone => 'Yok';

  @override
  String get questStateAvailable => 'Alınabilir';

  @override
  String get questStateRunning => 'Devam ediyor';

  @override
  String get questStateSucceeded => 'Başarılı';

  @override
  String get questStateFailed => 'Başarısız';

  @override
  String get questStateUnknown => 'bilinmiyor';

  @override
  String get dialogKnowledge => 'Diyalog bilgisi';

  @override
  String get resetKnowledgeChanges => 'Bilgi değişikliklerini sıfırla';

  @override
  String get addNpc => 'NPC ekle';

  @override
  String get searchNpcs => 'NPC ara';

  @override
  String get npcStatusRowLabel => 'Durum';

  @override
  String get npcStatusAlive => 'canlı';

  @override
  String get npcStatusDead => 'ölü';

  @override
  String get npcRelationshipRowLabel => 'İlişki';

  @override
  String get npcRelationshipUnavailable => 'İlişki durumu kullanılamıyor';

  @override
  String get npcRelationshipAutomatic => 'Oyun tarafından hesaplanır';

  @override
  String get npcRelationshipAutomaticHint =>
      'Kalıcı bir geçersiz kılma kaydedilmemiş. Lonca, hikâye, bölge ve suç kuralları oyunda değerlendirilir.';

  @override
  String get npcRelationshipStoredHint =>
      'Kalıcı bir NPC–oyuncu geçersiz kılması olarak kayıtlı. Lonca, hikâye, bölge ve suç kuralları oyundaki etkin durumu yine de değiştirebilir.';

  @override
  String get npcRelationshipFriend => 'Dost';

  @override
  String get npcRelationshipNeutral => 'Tarafsız';

  @override
  String get npcRelationshipEnemy => 'Düşman';

  @override
  String npcRelationshipPending(String relationship) {
    return 'Kaydedildiğinde $relationship olacak';
  }

  @override
  String npcStateHp(String hp, String maxHp) {
    return 'HP $hp / $maxHp';
  }

  @override
  String get npcReviveButton => 'Dirilt';

  @override
  String get npcReviveQueued => 'Kaydedildiğinde diriltilecek';

  @override
  String entriesForCharacter(String name) {
    return 'Girdiler — $name';
  }

  @override
  String get selectNpcToSeeEntries => 'Girdileri görmek için bir NPC seç';

  @override
  String get addKnowledgeEntry => 'Bilgi girdisi ekle';

  @override
  String get browseCatalog => 'Kataloğa gözat';

  @override
  String get alreadyExistsForCharacter => 'Bu karakter için zaten var.';

  @override
  String get alreadyInPendingChanges => 'Zaten beklemedeki değişikliklerde.';

  @override
  String duplicateCheckFailed(String error) {
    return 'Yinelenen kontrolü başarısız — tekrar dene: $error';
  }

  @override
  String pendingAddsCount(int count) {
    return 'Bekleyen eklemeler ($count)';
  }

  @override
  String get undoAdd => 'Eklemeyi geri al';

  @override
  String get undoRemove => 'Kaldırmayı geri al';

  @override
  String get removeEntry => 'Girdiyi kaldır';

  @override
  String get selectNpcFromList => 'Listeden bir NPC seç';

  @override
  String characterWithCount(String name, int count) {
    return '$name ($count)';
  }

  @override
  String get memoryEvents => 'Bellek olayları';

  @override
  String get searchCharacters => 'Karakter ara';

  @override
  String eventsForCharacter(String name) {
    return 'Olaylar — $name';
  }

  @override
  String get selectCharacterToSeeEvents =>
      'Olayları görmek için bir karakter seç';

  @override
  String get noTags => '(etiket yok)';

  @override
  String eventSubtitle(String time, String affected) {
    return 't=${time}s  $affected';
  }

  @override
  String get removeEvent => 'Olayı kaldır';

  @override
  String get removeMemoryEventTitle => 'Bellek olayı kaldırılsın mı?';

  @override
  String get removeMemoryEventBody =>
      'Bu bellek olayı kaldırma için sıraya alınsın mı? Kayıt dosyası yalnızca Kaydet\'e bastığında değişir.';

  @override
  String get memoryEventRemovalQueued =>
      'Olay kaldırma sıraya alındı — uygulamak için Kaydet\'e bas.';

  @override
  String get duplicateEvent => 'Olayı çoğalt';

  @override
  String get duplicateMemoryEventTitle => 'Bellek olayı çoğaltılsın mı?';

  @override
  String get duplicateMemoryEventBody =>
      'Bu bellek olayının bir kopyası sıraya alınsın mı? Kayıt dosyası yalnızca Kaydet\'e bastığında değişir.';

  @override
  String get memoryEventDuplicationQueued =>
      'Olay çoğaltma sıraya alındı — uygulamak için Kaydet\'e bas.';

  @override
  String get selectCharacterFromList => 'Listeden bir karakter seç';

  @override
  String get factionsSidebar => 'Loncalar';

  @override
  String get factionsForgiveButton => 'Affet';

  @override
  String get factionHostile => 'Düşman';

  @override
  String get factionFriendly => 'Dost';

  @override
  String crimeMurder(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count cinayet',
      one: '$count cinayet',
    );
    return '$_temp0';
  }

  @override
  String crimeAssault(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count saldırı',
      one: '$count saldırı',
    );
    return '$_temp0';
  }

  @override
  String crimeTheft(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count hırsızlık',
      one: '$count hırsızlık',
    );
    return '$_temp0';
  }

  @override
  String crimeTrespassing(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count izinsiz giriş',
      one: '$count izinsiz giriş',
    );
    return '$_temp0';
  }

  @override
  String crimeThreat(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count tehdit',
      one: '$count tehdit',
    );
    return '$_temp0';
  }

  @override
  String crimeOther(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count diğer suç',
      one: '$count diğer suç',
    );
    return '$_temp0';
  }

  @override
  String get factionsForgiveQueued => 'affediliyor…';

  @override
  String get factionsEmpty => 'Fraksiyonlara karşı açık suç yok.';

  @override
  String get factionGuildOldCamp => 'Eski Kamp';

  @override
  String get factionGuildNewCamp => 'Yeni Kamp';

  @override
  String get factionGuildSwampCamp => 'Bataklık Kampı';

  @override
  String get factionGuildOther => 'Diğerleri / bireyler';

  @override
  String get allDataLockedBody =>
      'Kapsamlı kaynak tarayıcısı şu anda yalnızca GSAV kayıt dosyaları için kullanılabilir.';

  @override
  String get allDataDescription =>
      'GSAV meta verilerini ve her PUBLIC/PRIVATE türündeki düğümü gez. Güvenli skaler ve yerel struct değerleri düzenlenebilir; kapsayıcılar ve opak baytlar görünür kalır.';

  @override
  String get allDataEditable => 'Düzenlenebilir';

  @override
  String get allDataReadOnly => 'Salt okunur';

  @override
  String get allDataType => 'Tür';

  @override
  String get allDataScalars => 'Skalerler';

  @override
  String get allDataStructs => 'Yapılar';

  @override
  String get allDataContainers => 'Kapsayıcılar';

  @override
  String get allDataOpaque => 'Opak';

  @override
  String get allDataNodes => 'Düğümler';

  @override
  String allDataChildren(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count alt düğüm',
      one: '1 alt düğüm',
    );
    return '$_temp0';
  }

  @override
  String get allDataPending => 'Beklemede';

  @override
  String get allDataTagInputHint =>
      'Virgül veya satır sonuyla ayrılmış etiketler';

  @override
  String allDataTypedSource(String source) {
    return '$source türünde';
  }

  @override
  String get searchPropertiesLabel =>
      'Özellik ara (boş = hepsini listele) — örn. Health, GameTime';

  @override
  String get decodingSaveTitle => 'Kayıt çözülüyor…';

  @override
  String get decodingSaveBody =>
      'İlk arama için tüm özel yük çözülüyor. Kayıt başına bir kez çalışır; sonrasında aramalar anında olur.';

  @override
  String get searchTheSaveTitle => 'Kayıtta ara';

  @override
  String get searchTheSaveBody =>
      'Bir özellik adı yaz ve Enter\'a bas. Hepsini listelemek için boş bırak.';

  @override
  String get searchFailedTitle => 'Arama başarısız';

  @override
  String get noMatchesTitle => 'Eşleşme yok';

  @override
  String get noMatchesBody =>
      'Hiçbir özellik yolu bu terimlerin hepsini içermiyor.';

  @override
  String get value => 'Değer';

  @override
  String get backupsTitle => 'Yedekler';

  @override
  String get refreshBackups => 'Yedekleri yenile';

  @override
  String get noBackupsTitle => 'Yedek yok';

  @override
  String get noBackupsBody =>
      'Düzenlenen kayıtlar seçili slotun yanında yedek dosyaları oluşturur.';

  @override
  String get slotBackups => 'Slot yedekleri';

  @override
  String get profileBackups => 'Profil yedekleri';

  @override
  String get backupFactName => 'Ad';

  @override
  String get backupFactSlot => 'Slot';

  @override
  String get backupFactCreated => 'Oluşturulma';

  @override
  String get backupFactSize => 'Boyut';

  @override
  String get backupFactStatus => 'Durum';

  @override
  String get backupFactSha1 => 'SHA-1';

  @override
  String restoreBackupTooltip(String fileName) {
    return '$fileName dosyasını geri yükle';
  }

  @override
  String get appearanceTitle => 'Görünüm';

  @override
  String get uiFont => 'Yazı tipi';

  @override
  String get theme => 'Tema';

  @override
  String get themeLight => 'Açık';

  @override
  String get themeDark => 'Koyu';

  @override
  String get themeSystem => 'Sistem';

  @override
  String get uiScale => 'Arayüz ölçeği';

  @override
  String get resetZoomTooltip => 'Yakınlaştırmayı sıfırla (Ctrl+0)';

  @override
  String get zoomTip =>
      'İpucu: Ctrl + / Ctrl - uygulamanın her yerinde yakınlaştırmayı değiştirir.';

  @override
  String get language => 'Arayüz';

  @override
  String get gameTextLanguage => 'Oyun metni';

  @override
  String get gameTextLanguageHint =>
      'Arayüz dilini seçmek eşleşen oyun metnini de seçer. Oyun metnini sonradan ayrıca değiştirebilirsin.';

  @override
  String get updatesTitle => 'Güncellemeler';

  @override
  String get checkForUpdatesAutomatically => 'Güncellemeleri otomatik denetle';

  @override
  String get checkForUpdatesNow => 'Güncellemeleri şimdi denetle';

  @override
  String get updatesPortableNotice =>
      'Taşınabilir sürüm indirme sayfasını tarayıcında açar. Mevcut dosyalarını yeni indirme ile değiştir.';

  @override
  String get updateAvailableTitle => 'Güncelleme var';

  @override
  String updateAvailableMessage(Object version, Object current) {
    return '$version sürümü mevcut. Sende $current var.';
  }

  @override
  String get updateDownload => 'İndir';

  @override
  String updateOpenFailed(String url) {
    return 'İndirme sayfası açılamadı. Şu adresten ulaşabilirsin: $url';
  }

  @override
  String get updateLater => 'Sonra';

  @override
  String get updateUpToDate => 'En son sürümü kullanıyorsun.';

  @override
  String get updateCheckFailed =>
      'Güncelleme denetlenemedi. Lütfen daha sonra tekrar dene.';

  @override
  String get gameTextTitle => 'Oyun metni';

  @override
  String get itemImagesTitle => 'Eşya görselleri';

  @override
  String get gameDataTitle => 'Oyun verisi';

  @override
  String itemImagesReady(int count) {
    return '$count eşya görseli hazır.';
  }

  @override
  String get itemImagesUnavailable =>
      'Eşya görselleri kullanılamıyor. Bunun yerine kategori simgeleri kullanılacak.';

  @override
  String get checkRefreshItemImages => 'Eşya görsellerini denetle / yenile';

  @override
  String get gameDataSourceMissing =>
      'Oyun metni otomatik hazırlanamadı. Ayarlar\'dan yerelleştirme önbelleğini seçebilirsin.';

  @override
  String get loadingTexts => 'Metinler yükleniyor…';

  @override
  String get loadingImages => 'Görseller yükleniyor…';

  @override
  String get preparing => 'Hazırlanıyor…';

  @override
  String gameTextExtractedWithCounts(int ids, int languages) {
    return 'Çıkarıldı: $languages dilde $ids kimlik.';
  }

  @override
  String get gameTextExtracted => 'Yerelleştirilmiş oyun metni çıkarıldı.';

  @override
  String get gameTextNotExtracted =>
      'Yerelleştirilmiş oyun metni henüz çıkarılmadı.';

  @override
  String get extracting => 'Çıkarılıyor…';

  @override
  String get extractRefreshLocalizedText =>
      'Yerelleştirilmiş metni çıkar / yenile';

  @override
  String get extractionComplete => 'Çıkarma tamamlandı';

  @override
  String get extractionFailed => 'Çıkarma başarısız';

  @override
  String get localizationCacheFileType => 'Yerelleştirme önbelleği';

  @override
  String get savegameDirectoryTitle => 'Kayıt dizini';

  @override
  String get folder => 'Klasör';

  @override
  String get codecTitle => 'Codec';

  @override
  String get check => 'Denetle';

  @override
  String get roundtrip => 'Gidiş-dönüş';

  @override
  String get noCodecStatus => 'Codec durumu yok';

  @override
  String get codecReady => 'Codec hazır';

  @override
  String get codecReadOnly => 'Codec salt okunur';

  @override
  String get codecUnavailable => 'Codec kullanılamıyor';

  @override
  String get details => 'Ayrıntılar';

  @override
  String codecStatusLine(String status) {
    return 'Durum: $status';
  }

  @override
  String codecCapabilityLine(String decompress, String compress) {
    return 'Açma: $decompress | Sıkıştırma: $compress';
  }

  @override
  String codecBackendLine(String backend) {
    return 'Backend: $backend';
  }

  @override
  String get yes => 'evet';

  @override
  String get no => 'hayır';

  @override
  String aboutVersion(String version, String sha) {
    return 'Sürüm $version ($sha)';
  }

  @override
  String get aboutCopyright => '© 2026 Daniel Hoer';

  @override
  String get aboutLicense => 'MIT License kapsamında lisanslanmıştır.';

  @override
  String difficultyTitle(String profile) {
    return 'Zorluk — $profile';
  }

  @override
  String get difficultyNoProfile => 'Profil yok';

  @override
  String get difficultyNoDifficulty => 'Zorluk yok';

  @override
  String get difficultyLabel => 'Zorluk';

  @override
  String get difficultyTooltipNoProfile => 'Profil seçilmedi';

  @override
  String get difficultyTooltipEdit => 'Bu profil için zorluğu düzenle';

  @override
  String get difficultyTooltipNoEditable =>
      'Bu profilde düzenlenebilir zorluk yok';

  @override
  String get preset => 'Ön ayar';

  @override
  String get presetNovice => 'Acemi';

  @override
  String get presetGothic => 'Gothic';

  @override
  String get presetHard => 'Zor';

  @override
  String get presetCustom => 'Özel';

  @override
  String unrecognisedPreset(Object preset) {
    return 'Kayıtlı ön ayar tanınmıyor ($preset). Yine de Yakın Dövüş Akış Yardımcısı / kalıcı ölüm değişikliklerini kaydedebilir veya üstten bir ön ayar seçerek üzerine yazabilirsin.';
  }

  @override
  String get closeCombatFlowHelper => 'Yakın Dövüş Akış Yardımcısı';

  @override
  String get permadeath => 'Kalıcı ölüm';

  @override
  String get notAvailableOnNovice => 'Acemi\'de kullanılamaz';

  @override
  String get levelCombat => 'Savaş';

  @override
  String get levelResources => 'Kaynaklar';

  @override
  String get levelProgression => 'İlerleme';

  @override
  String get difficultyAppliesToAllSaves =>
      'Zorluk bu profildeki tüm kayıtlara uygulanır.';

  @override
  String get savingDifficultyFailed => 'Zorluk kaydedilemedi.';

  @override
  String get addItemDialogTitle => 'Eşya ekle';

  @override
  String get searchItems => 'Eşya ara';

  @override
  String failedToLoadCatalog(String error) {
    return 'Katalog yüklenemedi: $error';
  }

  @override
  String get noItemsAvailableToAdd => 'Eklenecek eşya yok';

  @override
  String get noItemsMatch => 'Eşleşen eşya yok';

  @override
  String get countMustBeAtLeast1 => '≥ 1 olmalı';

  @override
  String countMustBeAtMost(int max) {
    return '≤ $max olmalı';
  }

  @override
  String get addNpcDialogTitle => 'NPC ekle';

  @override
  String get noNpcsAvailableToAdd => 'Eklenecek NPC yok';

  @override
  String get noNpcsMatch => 'Eşleşen NPC yok';

  @override
  String get categoryAll => 'Tümü';

  @override
  String allWithCount(int count) {
    return 'Tümü ($count)';
  }

  @override
  String get addKnowledgeEntryDialogTitle => 'Bilgi girdisi ekle';

  @override
  String get searchEntries => 'Girdilerde ara';

  @override
  String get noKnowledgeEntriesAvailableToAdd => 'Eklenecek bilgi girdisi yok';

  @override
  String get noEntriesMatch => 'Eşleşen girdi yok';

  @override
  String get heroGroupMainStats => 'Ana istatistikler';

  @override
  String get heroGroupCombatMovement => 'Savaş / hareket';

  @override
  String get heroGroupResistances => 'Dirençler';

  @override
  String get heroGroupThieving => 'Hırsızlık';

  @override
  String get heroGroupAdvanced => 'Gelişmiş';

  @override
  String get heroGroupDiving => 'Dalış';

  @override
  String get heroDivingSkillNote =>
      'Dalış öğrenildikten sonra oyun, kayıt her yüklendiğinde nefes ve iyileşmeyi becerinin kendi değerlerine sıfırlar. Saniyede harcanan nefes senin ayarladığın gibi kalır.';

  @override
  String get heroGroupSleep => 'Uyku';

  @override
  String get heroGroupIntoxication => 'Sarhoşluk';

  @override
  String get heroEntryHeroTransform => 'Konum';

  @override
  String attributeEmpty(String name) {
    return '$name boş — kaydetmeden önce bir değer gir veya orijinali geri yükle.';
  }

  @override
  String attributeInvalidNumber(String name, String text) {
    return '$name için geçersiz sayı: «$text»';
  }

  @override
  String get loadingEditorData => 'Düzenleyici verisi yükleniyor';

  @override
  String savingProgress(int done, int total) {
    return 'Kaydediliyor… $done / $total';
  }

  @override
  String localizedTextExtractedCount(int idCount, int languageCount) {
    return '$languageCount dilde $idCount kimlik çıkarıldı';
  }

  @override
  String get skillSmithing1H => 'Tek el demircilik';

  @override
  String get skillSmithing2H => 'Çift el demircilik';

  @override
  String get skillCircleNovice => 'Acemi büyücü';

  @override
  String get skillCircle1 => 'Birinci büyü çemberi';

  @override
  String get skillCircle2 => 'İkinci büyü çemberi';

  @override
  String get skillCircle3 => 'Üçüncü büyü çemberi';

  @override
  String get skillCircle4 => 'Dördüncü büyü çemberi';

  @override
  String get skillCircle5 => 'Beşinci büyü çemberi';

  @override
  String get skillCircle6 => 'Altıncı büyü çemberi';

  @override
  String get sectionGlossary => 'Sözlük';

  @override
  String get glossarySearch => 'Sözlükte ara';

  @override
  String get glossaryOldCamp => 'Eski Kamp';

  @override
  String get glossaryNewCamp => 'Yeni Kamp';

  @override
  String get glossarySwampCamp => 'Bataklık Kampı';

  @override
  String get glossaryOutsiders => 'Dışlananlar';

  @override
  String get glossaryCreatures => 'Yaratıklar';

  @override
  String get glossaryLocations => 'Konumlar';

  @override
  String get glossaryFilterLabel => 'Filtre';

  @override
  String get glossaryFilterTraders => 'Tüccarlar';

  @override
  String get glossaryFilterTeachers => 'Öğretmenler';

  @override
  String get roleTrader => 'Tüccar';

  @override
  String get roleDead => 'Ölü';

  @override
  String get roleTeacher => 'Öğretmen';

  @override
  String get roleArmorer => 'Zırhçı';

  @override
  String get glossaryFilterArmorers => 'Zırhçılar';

  @override
  String get glossaryFilterHostile => 'Düşman';

  @override
  String get glossaryRelationshipFilterNote =>
      'Kayıtta saklanan kalıcı düşman geçersiz kılmalarını gösterir. Dinamik lonca, hikâye, bölge ve suç ilişkileri yalnızca oyunda hesaplanır.';

  @override
  String get glossaryFilterDead => 'Ölü';

  @override
  String get glossaryAddEntry => 'Sözlük girdisi ekle';

  @override
  String get glossaryAddTitle => 'Sözlük girdisi ekle';

  @override
  String get glossaryResetChanges => 'Sözlük değişikliklerini sıfırla';

  @override
  String get glossaryNoVisibleEntries =>
      'Bu görünümle eşleşen görünür sözlük girdisi yok.';

  @override
  String get glossaryNoHiddenEntries => 'Mevcut tüm girdiler zaten görünür.';

  @override
  String get glossaryNoMatch => 'Eşleşen sözlük girdisi yok.';

  @override
  String get glossarySelectEntry =>
      'Girdilerini düzenlemek için bir sözlük girdisi seç.';

  @override
  String glossaryEntryCount(int count) {
    return '$count girdi';
  }

  @override
  String glossarySegmentsCount(int unlocked, int total) {
    return '$total girdiden $unlocked tanesi';
  }

  @override
  String get glossaryPortraitUnlocked => 'Portre açıldı';

  @override
  String get glossaryPortraitSilhouette => 'Silüet — portre açılmamış';

  @override
  String get glossarySegments => 'Girdiler';

  @override
  String get glossaryPending => 'Kaydedilmemiş değişiklik';

  @override
  String get glossaryShowFullText => 'Girdinin tam metnini göster';

  @override
  String get glossarySegmentIntroduction => 'Giriş / portre';

  @override
  String get glossarySegmentUnlock => 'Keşif';

  @override
  String glossarySegmentEntry(int number) {
    return 'Girdi $number';
  }

  @override
  String get questJournalAll => 'Tüm görevler';

  @override
  String get questJournalOldCamp => 'Eski Kamp';

  @override
  String get questJournalNewCamp => 'Yeni Kamp';

  @override
  String get questJournalSwampCamp => 'Bataklık Kampı';

  @override
  String get questJournalColony => 'Koloni';

  @override
  String get questJournalCompleted => 'Tamamlanan';

  @override
  String get questJournalHint =>
      'Oyun içi günlük görünümü. Dahili ve henüz başlamamış görev durumları Tüm Veriler altında kalır.';

  @override
  String get questJournalNoEntries =>
      'Geçerli filtrelerle eşleşen günlük görevi yok.';

  @override
  String get glossaryTutorials => 'Eğitimler';

  @override
  String get tutorialGateNote =>
      'Bu satırlar kayıtlı eğitim açma kapılarını kontrol eder. Bir kapı, oyun içindeki tek bir eğitim sayfasına bire bir karşılık gelmeyebilir.';

  @override
  String get tutorialResetChanges => 'Eğitim değişikliklerini sıfırla';

  @override
  String get tutorialNoGates =>
      'Bu kayıtta kullanılabilir eğitim açma kapısı yok.';

  @override
  String tutorialGateUnlockCount(int unlocked, int total) {
    return '$total eğitim kapısından $unlocked tanesi açık';
  }

  @override
  String get tutorialGateCombatBasics => 'Savaş temelleri';

  @override
  String get tutorialGateCrafting => 'Zanaat';

  @override
  String get tutorialGateCrime => 'Suç ve sonuçları';

  @override
  String get tutorialGateDrugs => 'Sarf malzemeleri ve etkiler';

  @override
  String get tutorialGateLockpicking => 'Kilit açma';

  @override
  String get tutorialGateMagic => 'Büyü';

  @override
  String get tutorialGateMap => 'Harita';

  @override
  String get tutorialGateMeleeCombat => 'Yakın dövüş';

  @override
  String get tutorialGateNavigation => 'Hareket ve gezinme';

  @override
  String get tutorialGatePerception => 'Algı';

  @override
  String get tutorialGatePlayerProgression => 'Karakter gelişimi';

  @override
  String get tutorialGateRanged => 'Menilli savaş';

  @override
  String get tutorialGateRiding => 'Scavenger sürme';

  @override
  String get tutorialGateSleep => 'Uyku';

  @override
  String get tutorialGateTrading => 'Ticaret';

  @override
  String get windowMinimizeTooltip => 'Simge durumuna küçült';

  @override
  String get windowMaximizeTooltip => 'Ekranı kapla';

  @override
  String get windowRestoreTooltip => 'Geri yükle';

  @override
  String get fallbackDialogEntry => 'Diyalog girdisi';

  @override
  String get fallbackDialogChoice => 'Diyalog seçeneği';

  @override
  String get fallbackDialogTopic => 'Diyalog konusu';

  @override
  String get fallbackDialogInformation => 'Diyalog bilgisi';

  @override
  String get fallbackQuest => 'Görev';

  @override
  String get fallbackObjective => 'Hedef';

  @override
  String get fallbackItem => 'Eşya';

  @override
  String get attributeSkillPointsFallback => 'Beceri puanları (LP)';

  @override
  String attributeManualFallbackLabel(String attributeId, String fallback) {
    String _temp0 = intl.Intl.selectLogic(attributeId, {
      'SuperArmor': 'Denge',
      'MaxSuperArmor': 'Azami denge',
      'DamageMultiplier': 'Alınan hasar',
      'SpeedModifier': 'Hareket hızı',
      'Oxygen': 'Nefes',
      'MaxOxygen': 'Azami nefes',
      'OxygenDepletionRate': 'Saniyede harcanan nefes',
      'OxygenRecoveryRate': 'Saniyede kazanılan nefes',
      'CriticalLevelPercent': 'Düşük nefes uyarısı',
      'SleepTime': 'Kalan dinlendirici saat',
      'MaxSleepTime': 'Azami dinlendirici saat',
      'SleepTimeRecoveryAmount': 'Geri kazanılan dinlendirici saat',
      'SleepTimeRecoveryPeriod': 'Yenileme aralığı',
      'MaxRestTime': 'Yatakta geçirilebilecek azami süre',
      'Health_RecoveryRatePerHourOfSleep': 'Uyku saati başına can',
      'Mana_RecoveryRatePerHourOfSleep': 'Uyku saati başına mana',
      'Alcohol': 'Alkol seviyesi',
      'MaxAlcohol': 'Azami alkol',
      'AlcoholDepletionRate': 'Ayılma hızı',
      'Swampweed': 'Bataklık otu seviyesi',
      'MaxSwampweed': 'Azami bataklık otu',
      'SwampweedDepletionRate': 'Etkinin geçme hızı',
      'XPExecutedBounty': 'Bitirme XP\'si',
      'XPKillOrDefeatBounty': 'Yenme XP\'si',
      'Level': 'Seviye',
      'LockpickDurability': 'Maymuncuk dayanıklılığı',
      'LockpickPrecision': 'Maymuncuk hassasiyeti',
      'PickPocketing': 'Yankesicilik',
      'other': '$fallback',
    });
    return '$_temp0';
  }

  @override
  String attributeManualTooltip(String attributeId) {
    String _temp0 = intl.Intl.selectLogic(attributeId, {
      'SuperArmor': 'Bu karakter sersemlemeden önce ne kadar darbe emebilir.',
      'MaxSuperArmor':
          'Tam denge havuzu; karakter seviyesi ve giyilen zırhla artar.',
      'DamageMultiplier':
          'Bu karakterin aldığı hasara uygulanan çarpan — 1 normal, daha yüksek daha çok acıtır.',
      'SpeedModifier':
          'Bu karakterin hareket hızına uygulanan çarpan — 1 normal.',
      'Oxygen': 'Su altında kalan hava saniyesi; sıfırda karakter boğulur.',
      'MaxOxygen':
          'Bu karakter su altında kalabileceği saniye; Dalış becerisi artırır.',
      'OxygenDepletionRate': 'Dalarken her saniye harcanan hava.',
      'OxygenRecoveryRate':
          'Yüzeye çıktıktan sonra her saniye geri kazanılan hava.',
      'CriticalLevelPercent':
          'Oyunun boğulma uyarısı verdiği kalan hava oranı.',
      'SleepTime':
          'Hâlâ bir şeyler yenileyen uyku saati; ötesinde oyun dinlenme bonusu vermez.',
      'MaxSleepTime':
          'Bu karakterin tutabileceği en büyük dinlendirici saat bütçesi.',
      'SleepTimeRecoveryAmount':
          'Dinlenme bütçesi her dolduğunda geri eklenen saat.',
      'SleepTimeRecoveryPeriod':
          'Dinlendirici saat bütçesinin yeniden dolması için geçen süre.',
      'MaxRestTime':
          'Oyunun tek seferde yatakta kalınmasına izin verdiği en uzun süre.',
      'Health_RecoveryRatePerHourOfSleep':
          'Her uyunan saat için geri kazanılan azami can oranı.',
      'Mana_RecoveryRatePerHourOfSleep':
          'Her uyunan saat için geri kazanılan azami mana oranı.',
      'Alcohol':
          'Bu karakter ne kadar sarhoş; üst kademeler çeviklik ve manayı güçle takas eder.',
      'MaxAlcohol': 'Bu karakterin ulaşabileceği en yüksek alkol seviyesi.',
      'AlcoholDepletionRate':
          'Alkol seviyesinin ayıkken ne kadar hızlı düştüğü.',
      'Swampweed':
          'Bu karakter ne kadar etkilenmiş; üst kademeler nitelikleri kaydırır.',
      'MaxSwampweed':
          'Bu karakterin ulaşabileceği en yüksek bataklık otu seviyesi.',
      'SwampweedDepletionRate':
          'Bataklık otu etkisinin ne kadar hızlı geçtiği.',
      'XPExecutedBounty':
          'Karakter yerde yenilmişken öldürüldüğünde kazanılan deneyim.',
      'XPKillOrDefeatBounty':
          'Karakter öldürüldüğünde veya bayıltıldığında kazanılan deneyim.',
      'Level': 'Karakter seviyesi. Deneyimle artar ve öğrenme puanı verir.',
      'LockpickDurability':
          'Kilit Açma becerisiyle: eğitimsiz 2, eğitilmiş 4, usta 6.',
      'LockpickPrecision':
          'Kilit Açma becerisiyle: eğitimsiz 0, eğitilmiş 1, usta 2.',
      'PickPocketing':
          'Yankesicilik becerisiyle: eğitimsiz -30, eğitilmiş -10, usta +10.',
      'other': '?',
    });
    return '$_temp0';
  }

  @override
  String get knowledgeTypeVoiceLine => 'Ses satırı';

  @override
  String get knowledgeTypeOther => 'Diğer';

  @override
  String get armorUpgradeUpper => 'Üst';

  @override
  String get armorUpgradeMiddle => 'Orta';

  @override
  String get armorUpgradeLower => 'Alt';

  @override
  String get knowledgeCategoryTopic => 'Konu';

  @override
  String get knowledgeCategoryChoice => 'Seçenek';

  @override
  String get knowledgeCategoryInfo => 'Bilgi';

  @override
  String get statusOk => 'Tamam';

  @override
  String get statusFailed => 'Başarısız';

  @override
  String get missingSaveReference => 'Dosya eksik';

  @override
  String missingSaveReferenceDescription(String slot) {
    return '$slot.sav eksik. Silinmiş, taşınmış veya yeniden adlandırılmış olabilir; profil hâlâ ona referans veriyor.';
  }

  @override
  String get removeFromProfile => 'Profilden kaldır';

  @override
  String get deleteSavegame => 'Kaydı sil';

  @override
  String get deleteSavegameTitle => 'Kayıt silinsin mi?';

  @override
  String deleteSavegameBody(String save, String fileName, String profile) {
    return '$save ($fileName) silinsin mi? $profile profilinden kaldırılacak ve kayıt klasöründen silinecek. GORE önce yedek oluşturur.';
  }

  @override
  String get removeSaveFromProfileTitle => 'Kayıt profilden kaldırılsın mı?';

  @override
  String removeSaveFromProfileBody(String save, String profile) {
    return '$save, $profile profilinden kaldırılsın mı? Kayıt dosyasının kendisi hâlâ varsa korunur.';
  }

  @override
  String get unassignedSave => 'Bir profile atanmamış';

  @override
  String get armorUpgradeLight => 'Hafif';

  @override
  String get armorUpgradeMedium => 'Orta';

  @override
  String get armorUpgradeHeavy => 'Ağır';

  @override
  String get knowledgeCaptionForcedConversation => 'Zorunlu konuşma';

  @override
  String get knowledgeCaptionFollowupTopic => 'Devam konusu';

  @override
  String get knowledgeCaptionFallbackTopic => 'Yedek konu';

  @override
  String durationMinutes(int minutes) {
    return '$minutes dk';
  }

  @override
  String durationHours(int hours) {
    return '$hours sa';
  }

  @override
  String durationHoursMinutes(int hours, int minutes) {
    return '$hours sa $minutes dk';
  }

  @override
  String get backupStatusInvalidProfileStructure => 'Geçersiz profil verisi';

  @override
  String get backupStatusSlotMetadataMissing =>
      'Seçili kayıt meta verisi eksik';

  @override
  String defaultProfileName(int id) {
    return 'Profil $id';
  }

  @override
  String get statusUnknown => 'Bilinmeyen';

  @override
  String editorUnexpectedError(String details) {
    return 'Beklenmeyen hata: $details';
  }

  @override
  String get editorOperationInProgress =>
      'Başka bir işlem sürüyor. Biraz sonra tekrar dene.';

  @override
  String get editorUnsavedBeforeDifficulty =>
      'Kaydedilmemiş düzenlemelerin var. Profil zorluğunu değiştirmeden önce kaydet veya sıfırla.';

  @override
  String get editorNoSaveFolderSelected => 'Kayıt klasörü seçilmedi.';

  @override
  String get editorNoSaveSelected => 'Kayıt seçilmedi.';

  @override
  String get coreUnknownError => 'Bilinmeyen çekirdek hatası';

  @override
  String get editorUnsavedBeforeSwitchProfile =>
      'Önce kaydedilmemiş değişikliklerini kaydet veya sıfırla — profil değiştirmek mevcut kayıttan uzaklaştırır.';

  @override
  String get editorUnsavedBeforeOpenFile =>
      'Başka bir dosya açmadan önce kaydedilmemiş değişikliklerini kaydet veya sıfırla.';

  @override
  String get editorSelectSavFile => 'Bir .sav kayıt dosyası seç.';

  @override
  String get editorNotGothicGsav =>
      'Seçilen dosya bir Gothic GSAV kaydı değil.';

  @override
  String get editorUnsavedBeforeChangeSaveProfile =>
      'Kayıt profilini değiştirmeden önce kaydedilmemiş değişikliklerini kaydet veya sıfırla.';

  @override
  String get editorUnsavedBeforeRemoveProfile =>
      'Kaydı profilden kaldırmadan önce kaydedilmemiş değişikliklerini kaydet veya sıfırla.';

  @override
  String get editorUnsavedBeforeDeleteSave =>
      'Bu kaydı silmeden önce kaydedilmemiş değişikliklerini kaydet veya sıfırla.';

  @override
  String get editorUnsavedBeforeRestoreProfile =>
      'Kaydedilmemiş düzenlemelerin var. Profil yedeğini geri yüklemeden önce kaydet veya sıfırla.';

  @override
  String editorConflictingPropertyEdits(String path) {
    return 'Kaydedilmemiş çakışan düzenlemeler aynı özelliği ($path) iki sekmeden hedefliyor. Birini sıfırla veya geri al, sonra tekrar kaydet.';
  }

  @override
  String editorGlossaryMemoryConflict(String path) {
    return 'Bir sözlük girdisi değişikliği ile başka kaydedilmemiş Tüm veri düzenlemesi Hero MemorizedEvents dizisini ($path) hedefliyor. Sözlük değişiklikleri bu diziye girdi ekler veya kaldırır; düzenlemeler birlikte kaydedilemez — birini sıfırla veya geri al, sonra tekrar kaydet.';
  }

  @override
  String editorGlossaryQuestConflict(String path) {
    return 'Bir sözlük girdisi değişikliği ile başka kaydedilmemiş düzenleme aynı görev CurrentState özelliğini ($path) hedefliyor. Sözlük değişikliği bu durumun kendisini günceller — birini sıfırla veya geri al, sonra tekrar kaydet.';
  }

  @override
  String editorRelationshipConflict(String path) {
    return 'Bir ilişki geçersiz kılması ile başka kaydedilmemiş Tüm veri düzenlemesi aynı NPC ilişki girdisini ($path) hedefliyor. Yapılandırılmış ilişki değişikliği bu girdideki değiştiricileri değiştirebilir; düzenlemeler birlikte kaydedilemez — birini sıfırla veya geri al, sonra tekrar kaydet.';
  }

  @override
  String editorMultipleStructuralArrayEdits(String path) {
    return 'Birden fazla kaydedilmemiş yapısal düzenleme aynı diziyi ($path) hedefliyor. Başka bir değişiklik sıraya almadan önce ilkini kaydet veya sıfırla.';
  }

  @override
  String editorStructuralArrayConflict(String path) {
    return 'Yapısal bir olay değişikliği ile başka kaydedilmemiş Tüm veri düzenlemesi $path hedefliyor. Devam etmeden birini kaydet veya sıfırla.';
  }

  @override
  String get editorSkillsEffectConflict =>
      'Bir Beceriler değişikliği ile aynı aktörün etkisine (ActiveEffects › EffectSpec › Def) yönelik Tüm veri düzenlemesi ikisi de sırada. Birlikte kaydedilemezler — birini sıfırla veya geri al, sonra tekrar kaydet.';

  @override
  String get editorInventoryResetConflict =>
      'Bir envanter sıfırlaması ile aynı envantere yönelik başka bir düzenleme ikisi de sırada. Sıfırlama tüm envanteri değiştirir ve diğer düzenlemeyi yok sayar — birini sıfırla veya geri al, sonra tekrar kaydet.';

  @override
  String get editorUseFolder => 'Klasör kullan';

  @override
  String get editorGothicSavegameFileType => 'Gothic kaydı';

  @override
  String get editorNoDifficultyChanges => 'Yazılacak zorluk değişikliği yok';

  @override
  String get editorDifficultyWritten =>
      'Zorluk profile yazıldı (yedek oluşturuldu)';

  @override
  String editorChangesSavedWithBackup(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count değişiklik yedekle kaydedildi',
      one: '1 değişiklik yedekle kaydedildi',
    );
    return '$_temp0';
  }

  @override
  String editorPlacementNoteFailed(String details) {
    return 'Taşıma kaydedildi, ancak geri alma notu yazılamadı: $details';
  }

  @override
  String editorProfileNotFound(int profileId) {
    return 'Profil $profileId bulunamadı.';
  }

  @override
  String get editorNoFreeSaveSlot =>
      'Oyun kayıt klasöründe boş kayıt yuvası yok (G1R-001 ile G1R-999 arası).';

  @override
  String editorSaveImportedAssigned(int profileId) {
    return 'Kayıt içe aktarıldı ve $profileId profiline atandı';
  }

  @override
  String editorSaveAssigned(int profileId) {
    return 'Kayıt $profileId profiline atandı (eş yedekler oluşturuldu)';
  }

  @override
  String editorSaveSlotNotAssigned(String slot, int profileId) {
    return 'Kayıt yuvası $slot, $profileId profiline atanmamış.';
  }

  @override
  String get editorSaveRemovedFromProfile => 'Kayıt profilden kaldırıldı';

  @override
  String get editorSaveDeleted => 'Kayıt silindi; yedek oluşturuldu';

  @override
  String editorRestoredBackup(String path) {
    return 'Yedek geri yüklendi: $path';
  }

  @override
  String editorRestoredBackupWithoutCompanion(String path) {
    return 'Yedek geri yüklendi: $path (PersistentDataList.sav değiştirilmedi — eşleşen eş yedek yok; yuva meta verisi farklı olabilir)';
  }

  @override
  String editorCodecRoundtripPassed(int chunkIndex, int bytes) {
    return 'Codec turu geçti: parça $chunkIndex, $bytes bayta yeniden sıkıştırıldı';
  }

  @override
  String editorDifficultyWriteFailed(String details) {
    return 'Profil zorluğu yazılamadı: $details';
  }

  @override
  String editorProfileAssignmentFailed(String details) {
    return 'Kayıt profile atanamadı: $details';
  }

  @override
  String editorProfileRemovalFailed(String details) {
    return 'Kayıt profilden kaldırılamadı: $details';
  }

  @override
  String editorDeleteSaveFailed(String details) {
    return 'Kayıt silinemedi: $details';
  }

  @override
  String editorSaveFailed(String details) {
    return 'Değişiklikler kaydedilemedi: $details';
  }

  @override
  String editorScanSavesFailed(String details) {
    return 'Kayıtlar taranamadı: $details';
  }

  @override
  String editorInspectSaveFailed(String details) {
    return 'Kayıt incelenemedi: $details';
  }

  @override
  String editorLoadBackupsFailed(String details) {
    return 'Yedekler yüklenemedi: $details';
  }

  @override
  String editorRestoreFailed(String details) {
    return 'Yedek geri yüklenemedi: $details';
  }

  @override
  String editorRestoreReloadFailed(String path, String details) {
    return 'Yedek geri yüklendi: $path, ancak kayıt yeniden yüklenemedi: $details';
  }

  @override
  String editorCodecCheckFailed(String details) {
    return 'Codec denetimi başarısız: $details';
  }

  @override
  String editorCodecValidationFailed(String details) {
    return 'Codec turu başarısız: $details';
  }

  @override
  String editorPropertySearchFailed(String details) {
    return 'Özellik araması başarısız: $details';
  }

  @override
  String get editorSelectionChangedWhileLoadingHeroAttributes =>
      'Kahraman nitelikleri yüklenirken kayıt seçimi değişti.';

  @override
  String editorSkillsLoadFailed(String details) {
    return 'Beceriler yüklenemedi: $details';
  }

  @override
  String editorProgressionQueryFailed(String details) {
    return 'İlerleme sorgusu başarısız: $details';
  }

  @override
  String editorNpcListFailed(String details) {
    return 'NPC listesi başarısız: $details';
  }

  @override
  String editorCharacterListFailed(String details) {
    return 'Karakter listesi başarısız: $details';
  }

  @override
  String editorNpcAttributesFailed(String details) {
    return 'NPC nitelikleri başarısız: $details';
  }

  @override
  String editorNpcPositionFailed(String details) {
    return 'NPC konumu yüklenemedi: $details';
  }

  @override
  String editorNpcInventoryFailed(String details) {
    return 'NPC envanteri başarısız: $details';
  }

  @override
  String editorFactionListFailed(String details) {
    return 'Fraksiyon listesi başarısız: $details';
  }

  @override
  String get editorNoBackupPath => 'yok';

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
    return '$prefix: $backupPath; PersistentDataList yedeği: $persistentPath';
  }

  @override
  String localizationStatusFailed(String details) {
    return 'Yerelleştirme durumu başarısız: $details';
  }

  @override
  String localizationExtractionFailed(String details) {
    return 'Çıkarma başarısız: $details';
  }

  @override
  String glossaryLoadFailed(String details) {
    return 'Sözlük yüklemesi başarısız: $details';
  }

  @override
  String backupStatusError(String details) {
    return 'Yedekleme hatası: $details';
  }

  @override
  String memoryEventCategory(String category, String fallback) {
    String _temp0 = intl.Intl.selectLogic(category, {
      'quest': 'Görev',
      'document': 'Belge',
      'story': 'Hikâye',
      'exploration': 'Keşif',
      'combat': 'Savaş',
      'social': 'Sosyal',
      'item': 'Eşyalar',
      'learning': 'Öğrenim',
      'guild': 'Lonca',
      'crime': 'Suç',
      'rest': 'Dinlenme',
      'other': '$fallback',
    });
    return '$_temp0';
  }

  @override
  String memoryEventAction(String kind, String fallback) {
    String _temp0 = intl.Intl.selectLogic(kind, {
      'questStarted': 'Görev başladı',
      'questSucceeded': 'Görev tamamlandı',
      'questFailed': 'Görev başarısız',
      'documentRead': 'Belge okundu',
      'documentSegmentUnlocked': 'Girdi keşfedildi',
      'documentSegmentViewed': 'Girdi görüntülendi',
      'chapterCompleted': 'Bölüm tamamlandı',
      'areaEntered': 'Bölgeye girildi',
      'areaLeft': 'Bölgeden çıkıldı',
      'characterKilled': 'Karakter öldürüldü',
      'characterDefeated': 'Karakter yenildi',
      'combatDodge': 'Saldırıdan kaçınıldı',
      'characterDebuffed': 'Zayıflatma uygulandı',
      'tradeAvailable': 'Ticaret açıldı',
      'itemObtained': 'Eşya elde edildi',
      'itemCrafted': 'Eşya üretildi',
      'skillStateRecorded': 'Beceri durumu kaydedildi',
      'recipeLearned': 'Tarif öğrenildi',
      'guildJoined': 'Loncaya katılındı',
      'crimeRecorded': 'Suç kaydedildi',
      'slept': 'Uyundu',
      'storyEvent': 'Hikâye olayı',
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
      'gameTime': 'Oyun zamanı',
      'duration': 'Süre',
      'chapter': 'Bölüm',
      'instigator': 'Başlatan',
      'affected': 'Etkilenen',
      'amount': 'Miktar',
      'primaryObject': 'Nesne',
      'secondaryObject': 'Bağlam',
      'segmentText': 'Girdi metni',
      'other': '$fallback',
    });
    return '$_temp0';
  }

  @override
  String memoryEventGameTime(int day, String time) {
    return 'Gün $day, $time';
  }

  @override
  String memoryEventSecondsValue(String value) {
    return '$value sn';
  }

  @override
  String memoryEventMoreValues(String values, int count) {
    return '$values +$count';
  }

  @override
  String get memoryEventHero => 'Kahraman';

  @override
  String get memoryEventDetails => 'Ayrıntılar';

  @override
  String get memoryEventTags => 'Etiketler';

  @override
  String get memoryEventTechnicalData => 'Teknik veri';

  @override
  String get memoryEventIndex => 'Dizin';

  @override
  String get memoryEventPosition => 'Konum';

  @override
  String get memoryEventPayload => 'Yük';

  @override
  String get memoryEventSubject => 'Konu';

  @override
  String glossaryCatalogSegmentLabel(String segmentId, String fallback) {
    String _temp0 = intl.Intl.selectLogic(segmentId, {
      'Access': 'Erişim',
      'AccessDenied': 'Erişim reddedildi',
      'AccesToTemple': 'Tapınağa erişim',
      'Advice': 'Tavsiye',
      'AfterFight': 'Dövüşten sonra',
      'AfterFireMages': 'Ateş büyücülerinden sonra',
      'AfterNek': 'Nek\'ten sonra',
      'AfterQuest': 'Görevden sonra',
      'Alone': 'Yalnız',
      'Amulet': 'Tılsım',
      'Annoying': 'Sinir bozucu',
      'Armor': 'Zırh',
      'Avoid': 'Kaçın',
      'Backstory': 'Geçmiş',
      'BackStory': 'Geçmiş',
      'BasicMagic': 'Temel büyü',
      'Beated': 'Yenilmiş',
      'BecomeMercenary': 'Paralı asker ol',
      'Beer': 'Bira',
      'Bestiary': 'Canavar ansiklopedisi',
      'Blessing': 'Kutsama',
      'Boss': 'Patron',
      'Bully': 'Zorba',
      'BullyAdvice': 'Zorba tavsiyesi',
      'Camp': 'Kamp',
      'CampDivided': 'Bölünmüş kamp',
      'CareOfMessengers': 'Habercilerin gözetimi',
      'ChangeOpinion': 'Fikir değişikliği',
      'ChargeUriziel': 'Uriziel\'i şarj et',
      'Chosen': 'Seçilmiş',
      'Contact': 'Temas',
      'Courier': 'Kurye',
      'CraftBows': 'Yay yapımı',
      'Crazy': 'Çılgın',
      'DailyMeal': 'Günlük yemek',
      'DailyRation_Trader': 'Günlük erzak tüccarı',
      'DAM': 'Baraj',
      'Dead': 'Ölü',
      'Deal': 'Anlaşma',
      'Dealer': 'Satıcı',
      'Deceived': 'Kandırılmış',
      'Dementia': 'Bunama',
      'DenyAccess': 'Erişimi reddet',
      'DifferentOpinion': 'Farklı görüş',
      'Discussion': 'Tartışma',
      'DontTalk': 'Konuşma',
      'Duel': 'Düello',
      'Entrance': 'Giriş',
      'Escape': 'Kaçış',
      'Extended': 'Genişletilmiş',
      'Extra': 'Ekstra',
      'ExtraInfo': 'Ek bilgi',
      'Fanatic': 'Fanatik',
      'Fight': 'Dövüş',
      'FindUlumulu': 'Ulu-Mulu bul',
      'FireMages': 'Ateş büyücüleri',
      'FireMagesEscape': 'Ateş büyücüleri kaçışı',
      'FiskNewDealer': 'Fisk için yeni çit',
      'FiskNewDealerCompleted': 'Fisk için yeni çit — tamamlandı',
      'FogTower': 'Sis kulesi',
      'Food': 'Yiyecek',
      'Forgave': 'Affetti',
      'Forgive': 'Affet',
      'Forgiven': 'Affedildi',
      'FourFriends': 'Dört arkadaş',
      'FreeHut': 'Boş kulübe',
      'FreeMine': 'Özgür maden',
      'Fury': 'Öfke',
      'GoodTeacher': 'İyi öğretmen',
      'Gossip': 'Dedikodu',
      'GotScavenger': 'Scavenger edinildi',
      'GrantedAccess': 'Erişim verildi',
      'GRDArmor': 'Muhafız zırhı',
      'Guide': 'Rehber',
      'HateMages': 'Büyücülerden nefret',
      'HateMagesExplanation': 'Büyücü nefreti açıklaması',
      'HateRiceLord': 'Pirinç lordundan nefret',
      'Heal': 'İyileştir',
      'Healing': 'İyileştirme',
      'Help': 'Yardım',
      'Helper': 'Yardımcı',
      'HelpKagan': 'Kagan\'a yardım',
      'HutStory': 'Kulübe hikâyesi',
      'Ignore': 'Yoksay',
      'Impress': 'Etkile',
      'ImpressAlchemy': 'Simya ile etkile',
      'ImpressInscription': 'Yazıt ile etkile',
      'Info': 'Bilgi',
      'Interested': 'İlgili',
      'Introduction': 'Giriş',
      'Introduction_2': 'Giriş 2',
      'Introduction_Armor': 'Giriş – zırh',
      'Introduction_Teacher': 'Giriş – öğretmen',
      'Introduction_Trader': 'Giriş – tüccar',
      'Invocation': 'Çağırma',
      'JoinSC': 'Bataklık kampına katıl',
      'Joint': 'Esrar',
      'KalomCamp': 'Kalom kampı',
      'Leader': 'Lider',
      'Learning': 'Öğrenme',
      'LearnOrcish': 'Orkça öğren',
      'LeftParty': 'Grubu terk etti',
      'Library': 'Kütüphane',
      'Lie': 'Yalan',
      'Lock': 'Kilit',
      'Lockpick': 'Maymuncuk',
      'Mad': 'Deli',
      'Mandibles': 'Maden arayüzü mandibulaları',
      'MapMaker': 'Haritacı',
      'Monastery': 'Manastır',
      'MordragKO': 'Mordrag KO',
      'Nek': 'Nek',
      'NewCamp': 'Yeni kamp',
      'NewCamper': 'Yeni kampçı',
      'NewLeader': 'Yeni lider',
      'NightPatrol': 'Gece devriyesi',
      'NotInterested': 'İlgilenmiyor',
      'OldCamp': 'Eski kamp',
      'OrcEnclaveEntrance': 'Ork bölgesi girişi',
      'OrcGraveyard': 'Ork mezarlığı',
      'OreArmor': 'Cevher zırhı',
      'Party': 'Grup',
      'Pay': 'Öde',
      'PayMoney': 'Para öde',
      'Permission': 'İzin',
      'Pet': 'Evcil hayvan',
      'PreparingInvocation': 'Çağırma hazırlığı',
      'Quest': 'Görev',
      'RankUpFireMages': 'Ateş büyücüsü terfisi',
      'RankUpGuard': 'Muhafız terfisi',
      'RanUpFireMagesCompleted': 'Ateş büyücüsü terfisi tamamlandı',
      'Realocated': 'Taşındı',
      'Reason': 'Sebep',
      'Respect': 'Saygı',
      'ReturnToSC': 'Bataklık kampına dön',
      'RicelordForeman': 'Pirinç lordunun ustabaşısı',
      'RideScavenger': 'Scavenger sür',
      'Robe': 'Cübbe',
      'Safe': 'Güvenli',
      'Scraper': 'Kazıyıcı',
      'SecondChance': 'İkinci şans',
      'SecretLocation': 'Gizli konum',
      'SecretPassage': 'Gizli geçit',
      'SecretPath': 'Gizli yol',
      'SleeperFollower': 'Uyuyan takipçi',
      'SleeperTemple': 'Uyuyan tapınağı',
      'SmallInfo': 'Kısa bilgi',
      'Stonehenge': 'Stonehenge',
      'StopFollowing': 'Takibi bırak',
      'SwampCamp': 'Bataklık kampı',
      'Talkative': 'Konuşkan',
      'Teach': 'Öğret',
      'TeachBow': 'Yay öğret',
      'Teacher': 'Öğretmen',
      'Teacher2': 'Öğretmen 2',
      'TeacherInscription': 'Yazıt öğretmeni',
      'TeacherMana': 'Mana öğretmeni',
      'TeachIchor': 'Maden arayüzü iksiri çıkarmayı öğret',
      'TeachMagic': 'Büyü öğret',
      'TeachOrcish': 'Orkça öğret',
      'TeachStats': 'İstatistik öğret',
      'TeachWeapon': 'Silah öğret',
      'Teleport': 'Işınlanma',
      'TheMysteriousOrc': 'Gizemli ork',
      'ThroneRoom': 'Taht odası',
      'TradeBow': 'Yay ticareti',
      'Trader': 'Tüccar',
      'TradeSkins_Trader': 'Deri tüccarı',
      'Traitor': 'Hain',
      'Trial': 'İmtihan',
      'TrollCanyon': 'Trol kanyonu',
      'Trust': 'Güven',
      'Ulumulu': 'Ulu-Mulu',
      'Unexperienced': 'Deneyimsiz',
      'Uriziel': 'Uriziel',
      'UrizielRune': 'Uriziel runu',
      'Useful': 'Faydalı',
      'Velaya': 'Velaya',
      'Vibrations': 'Titreşimler',
      'WaitFreeMine': 'Özgür madende bekle',
      'WaitInTrainingArea': 'Antrenman alanında bekle',
      'Warning': 'Uyarı',
      'WarningTooLate': 'Uyarı çok geç geldi',
      'WaterMessenger': 'Su büyücüleri habercisi',
      'Weapon': 'Silah',
      'Who': 'Kim',
      'Women': 'Kadınlar',
      'other': '$fallback',
    });
    return '$_temp0';
  }

  @override
  String get slotRepairTitle => 'Bozuk envanter yuvaları';

  @override
  String slotRepairBody(int count) {
    return 'Bu kayıtta kimliği konumuyla eşleşmeyen $count envanter yuvası var — oyunda böyle bir eşyayı bırakmak farklı bir eşyayı kaldırır. Onarım yalnızca kimlikleri yeniden yazar: eşya eklenmez, kaldırılmaz veya değiştirilmez. Kaydettiğinde her zamanki gibi yedek oluşturulur.';
  }

  @override
  String get slotRepairQueued =>
      'Onarım sıraya alındı — uygulamak için kaydet.';

  @override
  String get slotRepairAction => 'Onar';

  @override
  String get slotRepairDiscard => 'Vazgeç';

  @override
  String get editorInventorySlotEditConflict =>
      'Doğrudan bir envanter yuvası düzenlemesi ile tüm yuvaları etkileyen bir değişiklik (onarım, ekleme veya kaldırma) ikisi de sırada. İkincisi birincinin üzerine yazar — birini geri al, sonra tekrar kaydet.';

  @override
  String get editorTraderArrayConflict =>
      'Bir ticaret değişikliği ile tüccar dizisine doğrudan düzenleme ikisi de sırada. Düzenleme, ticaret değişikliğinin hedeflediği satırları yeniden numaralar; ikisinden biri yanlış tüccara gider — birini geri al, sonra tekrar kaydet.';

  @override
  String get backupFactFile => 'Dosya';

  @override
  String get renameBackupTooltip => 'Bu yedeğe ad ver';

  @override
  String get renameBackupTitle => 'Yedeğe ad ver';

  @override
  String get renameBackupLabel => 'Ad';

  @override
  String renameBackupHelp(String fileName) {
    return 'Dosya adı $fileName yerine gösterilir. Adı kaldırmak için boş bırak; dosyanın kendisi yeniden adlandırılmaz.';
  }

  @override
  String get deleteBackupTooltip => 'Bu yedeği sil';

  @override
  String get deleteBackupTitle => 'Yedeği sil';

  @override
  String deleteBackupBody(String name, String fileName) {
    return '«$name» ($fileName) silinsin mi? Dosya diskten kaldırılır ve geri getirilemez.';
  }

  @override
  String get deleteBackupConfirm => 'Sil';

  @override
  String editorDeletedBackup(String path) {
    return 'Yedek silindi: $path';
  }

  @override
  String editorDeleteBackupFailed(String details) {
    return 'Yedek silinemedi: $details';
  }

  @override
  String editorRenameBackupFailed(String details) {
    return 'Yedeğe ad verilemedi: $details';
  }

  @override
  String get slotRepairUnavailable =>
      'Şu an onarım mümkün değil — bu kayıt yazılamıyor.';

  @override
  String editorDeletedBackupWithLabelWarning(String path, String details) {
    return 'Yedek silindi: $path — adı kaldırılamadı: $details';
  }

  @override
  String get slotRepairNotOffered => 'Onarım bu kayıt için kullanılamıyor.';

  @override
  String get statisticsTitle => 'İstatistikler';

  @override
  String get statisticsSubtitle =>
      'Karakter, görev, dünya ve kayıt ilerlemesinin özet görünümü.';

  @override
  String statisticsCardTitle(String card, String fallback) {
    String _temp0 = intl.Intl.selectLogic(card, {
      'timing': 'Zaman',
      'character': 'Karakter',
      'quests': 'Görevler',
      'progress': 'İlerleme',
      'encounters': 'Savaş ve temaslar',
      'inventory': 'Beceriler ve envanter',
      'other': '$fallback',
    });
    return '$_temp0';
  }

  @override
  String statisticsMetric(String metric, String fallback) {
    String _temp0 = intl.Intl.selectLogic(metric, {
      'timePlayed': 'Oynanma',
      'worldTime': 'Dünya zamanı',
      'level': 'Seviye',
      'experience': 'Deneyim',
      'learningPoints': 'Öğrenme puanları',
      'guild': 'Lonca',
      'health': 'Can',
      'mana': 'Mana',
      'chapter': 'Bölüm',
      'location': 'Konum',
      'kills': 'NPC öldürmeleri',
      'knownCharacters': 'Bilinen karakterler',
      'killedMonsters': 'Öldürülen canavarlar',
      'defeatedNpcs': 'Yenilen NPC\'ler',
      'killedNpcs': 'Öldürülen NPC\'ler',
      'knownNpcs': 'Bilinen NPC\'ler',
      'knownTeachers': 'Bilinen öğretmenler',
      'learnedSkills': 'Öğrenilen beceriler',
      'knowledge': 'Bilgi girdileri',
      'deadCharacters': 'Ölü karakterler',
      'traders': 'Bilinen tüccarlar',
      'inventoryStacks': 'Eşya yığınları',
      'inventoryItems': 'Eşyalar',
      'ore': 'Cevher',
      'equipped': 'Kuşanılmış',
      'hostileFactions': 'Düşman fraksiyonlar',
      'openCrimes': 'Açık suçlar',
      'position': 'Konum',
      'other': '$fallback',
    });
    return '$_temp0';
  }

  @override
  String statisticsGuildRank(String rank, String fallback) {
    String _temp0 = intl.Intl.selectLogic(rank, {
      'oldCampShadow': 'Eski Kamp · Gölge',
      'oldCampGuard': 'Eski Kamp · Muhafız',
      'oldCampFireMage': 'Eski Kamp · Ateş büyücüsü',
      'newCampRogue': 'Yeni Kamp · Haydut',
      'newCampMercenary': 'Yeni Kamp · Paralı asker',
      'newCampWaterMage': 'Yeni Kamp · Su büyücüsü',
      'swampCampNovice': 'Bataklık Kampı · Acemi',
      'swampCampTemplar': 'Bataklık Kampı · Templar',
      'other': '$fallback',
    });
    return '$_temp0';
  }

  @override
  String get statisticsUnknown => 'Kullanılamıyor';

  @override
  String get statisticsMore => 'Daha fazla istatistik';

  @override
  String statisticsSummary(
    String level,
    String guild,
    String chapter,
    int completed,
    int failed,
    String playTime,
  ) {
    return 'Seviye $level, $guild, bölüm $chapter. $completed görev tamamlandı, $failed başarısız. Oyun süresi: $playTime.';
  }

  @override
  String get locksSidebar => 'Kilitler';

  @override
  String editorLockListFailed(String details) {
    return 'Kilit listesi başarısız: $details';
  }

  @override
  String get locksSearchHint => 'Kilit veya anahtar ara';

  @override
  String get locksAllRegions => 'Tüm bölgeler';

  @override
  String locksShownOfTotal(int shown, int total) {
    return '$shown / $total';
  }

  @override
  String get locksFilterChests => 'Sandıklar';

  @override
  String get locksFilterDoors => 'Kapılar';

  @override
  String get locksFilterUnlocked => 'Açık';

  @override
  String get locksFilterLocked => 'Kilitli';

  @override
  String locksDifficultyLevel(int bars, int level) {
    return 'Zorluk 4 üzerinden $bars (dahili kademe 7 üzerinden $level)';
  }

  @override
  String get locksKeyOnly => 'Sadece anahtarla';

  @override
  String locksKeyLabel(String keys) {
    return 'Anahtar: $keys';
  }

  @override
  String get locksPermalocked => 'Kalıcı olarak mühürlü';

  @override
  String get locksReadOnly => 'Bu kayıtta düzenlenebilir bir kilit kümesi yok.';

  @override
  String get locksUnknownEntry => 'Bu oyun sürümünde yok';

  @override
  String get locksDoorLeafHint =>
      'Bir kapıyı tekrar kilitlemek kapının kendisini de kapatır.';

  @override
  String get locksResetPending => 'Bekleyen kilit değişikliklerini iptal et';
}
