import 'dart:async';

import 'package:flutter/foundation.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_localizations/flutter_localizations.dart';
import 'package:intl/intl.dart' as intl;

import 'app_localizations_de.dart';
import 'app_localizations_en.dart';
import 'app_localizations_es.dart';
import 'app_localizations_fr.dart';
import 'app_localizations_it.dart';
import 'app_localizations_ja.dart';
import 'app_localizations_pl.dart';
import 'app_localizations_pt.dart';
import 'app_localizations_ru.dart';
import 'app_localizations_zh.dart';

// ignore_for_file: type=lint

/// Callers can lookup localized strings with an instance of AppLocalizations
/// returned by `AppLocalizations.of(context)`.
///
/// Applications need to include `AppLocalizations.delegate()` in their app's
/// `localizationDelegates` list, and the locales they support in the app's
/// `supportedLocales` list. For example:
///
/// ```dart
/// import 'l10n/app_localizations.dart';
///
/// return MaterialApp(
///   localizationsDelegates: AppLocalizations.localizationsDelegates,
///   supportedLocales: AppLocalizations.supportedLocales,
///   home: MyApplicationHome(),
/// );
/// ```
///
/// ## Update pubspec.yaml
///
/// Please make sure to update your pubspec.yaml to include the following
/// packages:
///
/// ```yaml
/// dependencies:
///   # Internationalization support.
///   flutter_localizations:
///     sdk: flutter
///   intl: any # Use the pinned version from flutter_localizations
///
///   # Rest of dependencies
/// ```
///
/// ## iOS Applications
///
/// iOS applications define key application metadata, including supported
/// locales, in an Info.plist file that is built into the application bundle.
/// To configure the locales supported by your app, you’ll need to edit this
/// file.
///
/// First, open your project’s ios/Runner.xcworkspace Xcode workspace file.
/// Then, in the Project Navigator, open the Info.plist file under the Runner
/// project’s Runner folder.
///
/// Next, select the Information Property List item, select Add Item from the
/// Editor menu, then select Localizations from the pop-up menu.
///
/// Select and expand the newly-created Localizations item then, for each
/// locale your application supports, add a new item and select the locale
/// you wish to add from the pop-up menu in the Value field. This list should
/// be consistent with the languages listed in the AppLocalizations.supportedLocales
/// property.
abstract class AppLocalizations {
  AppLocalizations(String locale)
    : localeName = intl.Intl.canonicalizedLocale(locale.toString());

  final String localeName;

  static AppLocalizations of(BuildContext context) {
    return Localizations.of<AppLocalizations>(context, AppLocalizations)!;
  }

  static const LocalizationsDelegate<AppLocalizations> delegate =
      _AppLocalizationsDelegate();

  /// A list of this localizations delegate along with the default localizations
  /// delegates.
  ///
  /// Returns a list of localizations delegates containing this delegate along with
  /// GlobalMaterialLocalizations.delegate, GlobalCupertinoLocalizations.delegate,
  /// and GlobalWidgetsLocalizations.delegate.
  ///
  /// Additional delegates can be added by appending to this list in
  /// MaterialApp. This list does not have to be used at all if a custom list
  /// of delegates is preferred or required.
  static const List<LocalizationsDelegate<dynamic>> localizationsDelegates =
      <LocalizationsDelegate<dynamic>>[
        delegate,
        GlobalMaterialLocalizations.delegate,
        GlobalCupertinoLocalizations.delegate,
        GlobalWidgetsLocalizations.delegate,
      ];

  /// A list of this localizations delegate's supported locales.
  static const List<Locale> supportedLocales = <Locale>[
    Locale('de'),
    Locale('en'),
    Locale('es'),
    Locale('fr'),
    Locale('it'),
    Locale('ja'),
    Locale('pl'),
    Locale('pt'),
    Locale('pt', 'BR'),
    Locale('ru'),
    Locale('zh'),
    Locale.fromSubtags(languageCode: 'zh', scriptCode: 'Hans'),
  ];

  /// No description provided for @debugSectionTitle.
  ///
  /// In en, this message translates to:
  /// **'Advanced (debug)'**
  String get debugSectionTitle;

  /// No description provided for @debugSectionSubtitle.
  ///
  /// In en, this message translates to:
  /// **'Diagnostics & raw data for bug reports'**
  String get debugSectionSubtitle;

  /// No description provided for @showObjectIdsTitle.
  ///
  /// In en, this message translates to:
  /// **'Show additional technical IDs'**
  String get showObjectIdsTitle;

  /// No description provided for @showObjectIdsSubtitle.
  ///
  /// In en, this message translates to:
  /// **'Show technical item, dialogue knowledge, quest, and orphan actor IDs in the editor. NPC IDs are always shown.'**
  String get showObjectIdsSubtitle;

  /// No description provided for @storyStateSidebar.
  ///
  /// In en, this message translates to:
  /// **'Story state'**
  String get storyStateSidebar;

  /// No description provided for @storyStateDescription.
  ///
  /// In en, this message translates to:
  /// **'Authoritative catalog of persisted story state declared by the shipped game scripts. Stored entries show their raw value; catalog fields missing from this save are marked as not set. Source-declared time markers are formatted as game time, while other integers may be booleans, counters, or multi-state values.'**
  String get storyStateDescription;

  /// No description provided for @storyStateReadOnly.
  ///
  /// In en, this message translates to:
  /// **'Read-only until the script meaning of values and safe map writes are established. Related glossary text is context, not a direct translation of the technical ID.'**
  String get storyStateReadOnly;

  /// No description provided for @storyStateStructureReadOnly.
  ///
  /// In en, this message translates to:
  /// **'The StoryPropertyValues structure in this save could not be resolved uniquely and safely. Story values remain read-only for this save.'**
  String get storyStateStructureReadOnly;

  /// No description provided for @storyStateSearch.
  ///
  /// In en, this message translates to:
  /// **'Search story state'**
  String get storyStateSearch;

  /// No description provided for @storyStateValuesCount.
  ///
  /// In en, this message translates to:
  /// **'{shown} of {total} story values'**
  String storyStateValuesCount(int shown, int total);

  /// No description provided for @storyStateInteger.
  ///
  /// In en, this message translates to:
  /// **'Integer'**
  String get storyStateInteger;

  /// No description provided for @storyStateTimeMarker.
  ///
  /// In en, this message translates to:
  /// **'Time marker'**
  String get storyStateTimeMarker;

  /// No description provided for @storyStateChapter.
  ///
  /// In en, this message translates to:
  /// **'Chapter'**
  String get storyStateChapter;

  /// No description provided for @storyStateUnknown.
  ///
  /// In en, this message translates to:
  /// **'Unknown source type'**
  String get storyStateUnknown;

  /// No description provided for @storyStateUnknownDetail.
  ///
  /// In en, this message translates to:
  /// **'This stored ID is absent from the current script catalog (for example, from a mod or newer game version). Its save wire value is int32, but its meaning is not inferred.'**
  String get storyStateUnknownDetail;

  /// No description provided for @storyStateStored.
  ///
  /// In en, this message translates to:
  /// **'Stored'**
  String get storyStateStored;

  /// No description provided for @storyStateUnset.
  ///
  /// In en, this message translates to:
  /// **'Not set'**
  String get storyStateUnset;

  /// No description provided for @storyStateUnsetDetail.
  ///
  /// In en, this message translates to:
  /// **'This catalog field is not serialized in this save; the game therefore uses its unset or default state.'**
  String get storyStateUnsetDetail;

  /// No description provided for @storyStateRawValue.
  ///
  /// In en, this message translates to:
  /// **'Raw value'**
  String get storyStateRawValue;

  /// No description provided for @storyStateElapsed.
  ///
  /// In en, this message translates to:
  /// **'Elapsed at save time: {duration}'**
  String storyStateElapsed(String duration);

  /// No description provided for @storyStateAhead.
  ///
  /// In en, this message translates to:
  /// **'Ahead of save time: {duration}'**
  String storyStateAhead(String duration);

  /// No description provided for @storyStateDurationDays.
  ///
  /// In en, this message translates to:
  /// **'{days, plural, =1{1 day} other{{days} days}} {time}'**
  String storyStateDurationDays(int days, String time);

  /// No description provided for @storyStateRelatedGlossary.
  ///
  /// In en, this message translates to:
  /// **'Related glossary entry'**
  String get storyStateRelatedGlossary;

  /// No description provided for @storyStateTechnicalPath.
  ///
  /// In en, this message translates to:
  /// **'Technical path'**
  String get storyStateTechnicalPath;

  /// No description provided for @storyStateEditingGuidance.
  ///
  /// In en, this message translates to:
  /// **'Every entry remains editable across the full signed int32 range. Script-backed switches and value suggestions are guidance; raw input is always available. Story changes can skip dialogue, quest, or world transitions, so save them deliberately — a backup is created automatically.'**
  String get storyStateEditingGuidance;

  /// No description provided for @storyStatePending.
  ///
  /// In en, this message translates to:
  /// **'Pending'**
  String get storyStatePending;

  /// No description provided for @storyStatePendingValue.
  ///
  /// In en, this message translates to:
  /// **'Will be stored as {value}'**
  String storyStatePendingValue(String value);

  /// No description provided for @storyStatePendingRemoval.
  ///
  /// In en, this message translates to:
  /// **'Will be removed from the save'**
  String get storyStatePendingRemoval;

  /// No description provided for @storyStateEditValue.
  ///
  /// In en, this message translates to:
  /// **'Edit value'**
  String get storyStateEditValue;

  /// No description provided for @storyStateSetValue.
  ///
  /// In en, this message translates to:
  /// **'Set value'**
  String get storyStateSetValue;

  /// No description provided for @storyStateRemoveValue.
  ///
  /// In en, this message translates to:
  /// **'Remove from save'**
  String get storyStateRemoveValue;

  /// No description provided for @storyStateUndoChange.
  ///
  /// In en, this message translates to:
  /// **'Undo story change'**
  String get storyStateUndoChange;

  /// No description provided for @storyStateResetChanges.
  ///
  /// In en, this message translates to:
  /// **'Reset story changes'**
  String get storyStateResetChanges;

  /// No description provided for @storyStateDialogTitle.
  ///
  /// In en, this message translates to:
  /// **'Edit {id}'**
  String storyStateDialogTitle(String id);

  /// No description provided for @storyStateRawInput.
  ///
  /// In en, this message translates to:
  /// **'Signed int32 value'**
  String get storyStateRawInput;

  /// No description provided for @storyStateInvalidInt32.
  ///
  /// In en, this message translates to:
  /// **'Enter a whole number from -2147483648 to 2147483647.'**
  String get storyStateInvalidInt32;

  /// No description provided for @storyStateQueueChange.
  ///
  /// In en, this message translates to:
  /// **'Queue change'**
  String get storyStateQueueChange;

  /// No description provided for @storyStateSuggestedValues.
  ///
  /// In en, this message translates to:
  /// **'Values evidenced in the shipped scripts: {values}'**
  String storyStateSuggestedValues(String values);

  /// No description provided for @storyStateSuggestionsNotLimits.
  ///
  /// In en, this message translates to:
  /// **'Suggestions are not validation limits; native code, mods, or later game versions may use other values.'**
  String get storyStateSuggestionsNotLimits;

  /// No description provided for @storyStateUseCurrentTime.
  ///
  /// In en, this message translates to:
  /// **'Use current save time'**
  String get storyStateUseCurrentTime;

  /// No description provided for @storyStateStructuredTime.
  ///
  /// In en, this message translates to:
  /// **'Day / time'**
  String get storyStateStructuredTime;

  /// No description provided for @storyStateRawMode.
  ///
  /// In en, this message translates to:
  /// **'Raw int32'**
  String get storyStateRawMode;

  /// No description provided for @storyStateChapterWarning.
  ///
  /// In en, this message translates to:
  /// **'Changing the chapter alone does not synchronize quests, NPCs, inventory, or world state.'**
  String get storyStateChapterWarning;

  /// No description provided for @storyStateDormantWarning.
  ///
  /// In en, this message translates to:
  /// **'No live read or write was found for this field in the shipped script cache. It may be legacy, native-controlled, or reserved.'**
  String get storyStateDormantWarning;

  /// No description provided for @storyStateReadOnlySourceWarning.
  ///
  /// In en, this message translates to:
  /// **'The shipped scripts read this field but contain no script write. Native code may still own it.'**
  String get storyStateReadOnlySourceWarning;

  /// No description provided for @storyStateUnknownEditWarning.
  ///
  /// In en, this message translates to:
  /// **'This modded or newer-version ID has no bundled source semantics. Edit only its raw int32 value.'**
  String get storyStateUnknownEditWarning;

  /// No description provided for @storyStateIntegerKind.
  ///
  /// In en, this message translates to:
  /// **'{kind, select, binaryFlag{Binary flag} finiteState{Multi-state value} counterOrScore{Counter / score} calendarDay{Calendar day} derivedOrOpaqueInteger{Derived / opaque integer} readOnlyInSourceInteger{Read-only in shipped scripts} dormantOrLegacyInteger{Unused in shipped scripts} other{Integer}}'**
  String storyStateIntegerKind(String kind);

  /// No description provided for @storyStateZeroVsUnset.
  ///
  /// In en, this message translates to:
  /// **'A stored 0 and a missing map entry are distinct file states. “Remove from save” restores the constructor/default state.'**
  String get storyStateZeroVsUnset;

  /// No description provided for @appTitle.
  ///
  /// In en, this message translates to:
  /// **'GORE Save Editor'**
  String get appTitle;

  /// No description provided for @appLogoSemanticLabel.
  ///
  /// In en, this message translates to:
  /// **'GORE Save Editor logo'**
  String get appLogoSemanticLabel;

  /// No description provided for @zoomTooltip.
  ///
  /// In en, this message translates to:
  /// **'Press Ctrl +/- to zoom in/out'**
  String get zoomTooltip;

  /// No description provided for @switchToLightMode.
  ///
  /// In en, this message translates to:
  /// **'Switch to light mode'**
  String get switchToLightMode;

  /// No description provided for @switchToDarkMode.
  ///
  /// In en, this message translates to:
  /// **'Switch to dark mode'**
  String get switchToDarkMode;

  /// No description provided for @about.
  ///
  /// In en, this message translates to:
  /// **'About'**
  String get about;

  /// No description provided for @tabOverview.
  ///
  /// In en, this message translates to:
  /// **'Overview'**
  String get tabOverview;

  /// No description provided for @tabPlayer.
  ///
  /// In en, this message translates to:
  /// **'Player'**
  String get tabPlayer;

  /// No description provided for @tabAttribute.
  ///
  /// In en, this message translates to:
  /// **'Attributes'**
  String get tabAttribute;

  /// No description provided for @heroGroupSkills.
  ///
  /// In en, this message translates to:
  /// **'Skills'**
  String get heroGroupSkills;

  /// No description provided for @skillsNoneBody.
  ///
  /// In en, this message translates to:
  /// **'No skills found for this character.'**
  String get skillsNoneBody;

  /// No description provided for @skillsUnavailableBody.
  ///
  /// In en, this message translates to:
  /// **'Skills can\'t be edited on this save — the hero has no effect data to modify.'**
  String get skillsUnavailableBody;

  /// No description provided for @skillNotLearned.
  ///
  /// In en, this message translates to:
  /// **'Not learned'**
  String get skillNotLearned;

  /// No description provided for @skillLearn.
  ///
  /// In en, this message translates to:
  /// **'Learn'**
  String get skillLearn;

  /// No description provided for @skillActionLearn.
  ///
  /// In en, this message translates to:
  /// **'learn'**
  String get skillActionLearn;

  /// No description provided for @skillActionUnlearn.
  ///
  /// In en, this message translates to:
  /// **'unlearn'**
  String get skillActionUnlearn;

  /// No description provided for @skillTierUntrained.
  ///
  /// In en, this message translates to:
  /// **'Untrained'**
  String get skillTierUntrained;

  /// No description provided for @skillTierBeginner.
  ///
  /// In en, this message translates to:
  /// **'Beginner'**
  String get skillTierBeginner;

  /// No description provided for @skillTierTrained.
  ///
  /// In en, this message translates to:
  /// **'Trained'**
  String get skillTierTrained;

  /// No description provided for @skillTierMaster.
  ///
  /// In en, this message translates to:
  /// **'Master'**
  String get skillTierMaster;

  /// No description provided for @skillTierNovice.
  ///
  /// In en, this message translates to:
  /// **'Novice'**
  String get skillTierNovice;

  /// No description provided for @skillTierAmateur.
  ///
  /// In en, this message translates to:
  /// **'Amateur (Circle 0)'**
  String get skillTierAmateur;

  /// No description provided for @skillTierLearned.
  ///
  /// In en, this message translates to:
  /// **'Learned'**
  String get skillTierLearned;

  /// No description provided for @skillTierCircle.
  ///
  /// In en, this message translates to:
  /// **'Circle {n}'**
  String skillTierCircle(int n);

  /// No description provided for @skillHintBlacksmith1H.
  ///
  /// In en, this message translates to:
  /// **'1H weapons'**
  String get skillHintBlacksmith1H;

  /// No description provided for @skillHintBlacksmith2H.
  ///
  /// In en, this message translates to:
  /// **'2H weapons'**
  String get skillHintBlacksmith2H;

  /// No description provided for @skillScutesTrained.
  ///
  /// In en, this message translates to:
  /// **'Trained (bone scutes)'**
  String get skillScutesTrained;

  /// No description provided for @skillScutesMaster.
  ///
  /// In en, this message translates to:
  /// **'Master (+ razor plates)'**
  String get skillScutesMaster;

  /// No description provided for @skillCategoryCombat.
  ///
  /// In en, this message translates to:
  /// **'Combat'**
  String get skillCategoryCombat;

  /// No description provided for @skillCategoryCrafting.
  ///
  /// In en, this message translates to:
  /// **'Crafting'**
  String get skillCategoryCrafting;

  /// No description provided for @skillCategoryHunting.
  ///
  /// In en, this message translates to:
  /// **'Hunting'**
  String get skillCategoryHunting;

  /// No description provided for @skillCategoryLanguage.
  ///
  /// In en, this message translates to:
  /// **'Language'**
  String get skillCategoryLanguage;

  /// No description provided for @skillCategoryMagic.
  ///
  /// In en, this message translates to:
  /// **'Magic'**
  String get skillCategoryMagic;

  /// No description provided for @skillCategoryMovement.
  ///
  /// In en, this message translates to:
  /// **'Movement'**
  String get skillCategoryMovement;

  /// No description provided for @skillCategoryThievery.
  ///
  /// In en, this message translates to:
  /// **'Thievery'**
  String get skillCategoryThievery;

  /// No description provided for @skillCategoryOther.
  ///
  /// In en, this message translates to:
  /// **'Other'**
  String get skillCategoryOther;

  /// No description provided for @skillNameOneHanded.
  ///
  /// In en, this message translates to:
  /// **'One Handed'**
  String get skillNameOneHanded;

  /// No description provided for @skillNameTwoHanded.
  ///
  /// In en, this message translates to:
  /// **'Two Handed'**
  String get skillNameTwoHanded;

  /// No description provided for @skillNameFists.
  ///
  /// In en, this message translates to:
  /// **'Fists'**
  String get skillNameFists;

  /// No description provided for @skillNameBow.
  ///
  /// In en, this message translates to:
  /// **'Bow'**
  String get skillNameBow;

  /// No description provided for @skillNameCrossbow.
  ///
  /// In en, this message translates to:
  /// **'Crossbow'**
  String get skillNameCrossbow;

  /// No description provided for @skillNameLockpicking.
  ///
  /// In en, this message translates to:
  /// **'Lockpicking'**
  String get skillNameLockpicking;

  /// No description provided for @skillNamePickpocketing.
  ///
  /// In en, this message translates to:
  /// **'Pickpocketing'**
  String get skillNamePickpocketing;

  /// No description provided for @skillNameTakeOrgans.
  ///
  /// In en, this message translates to:
  /// **'Extract Organ'**
  String get skillNameTakeOrgans;

  /// No description provided for @skillNameBreakTeeth.
  ///
  /// In en, this message translates to:
  /// **'Extract Teeth'**
  String get skillNameBreakTeeth;

  /// No description provided for @skillNameTakeClaws.
  ///
  /// In en, this message translates to:
  /// **'Extract Claw'**
  String get skillNameTakeClaws;

  /// No description provided for @skillNameSkinFur.
  ///
  /// In en, this message translates to:
  /// **'Take Fur'**
  String get skillNameSkinFur;

  /// No description provided for @skillNameSkin.
  ///
  /// In en, this message translates to:
  /// **'Take Skin'**
  String get skillNameSkin;

  /// No description provided for @skillNameTakeFins.
  ///
  /// In en, this message translates to:
  /// **'Take Fins'**
  String get skillNameTakeFins;

  /// No description provided for @skillNameTakeStingers.
  ///
  /// In en, this message translates to:
  /// **'Extract Stings'**
  String get skillNameTakeStingers;

  /// No description provided for @skillNameTakeSecretion.
  ///
  /// In en, this message translates to:
  /// **'Extract Secretion'**
  String get skillNameTakeSecretion;

  /// No description provided for @skillNameTakeSkullPlates.
  ///
  /// In en, this message translates to:
  /// **'Take Skull Armor'**
  String get skillNameTakeSkullPlates;

  /// No description provided for @skillNameSkinSwampshark.
  ///
  /// In en, this message translates to:
  /// **'Take Shark Skin'**
  String get skillNameSkinSwampshark;

  /// No description provided for @skillNameTakeMinecrawlerPlates.
  ///
  /// In en, this message translates to:
  /// **'Take Plates'**
  String get skillNameTakeMinecrawlerPlates;

  /// No description provided for @skillNameTakeScutes.
  ///
  /// In en, this message translates to:
  /// **'Take Scutes'**
  String get skillNameTakeScutes;

  /// No description provided for @skillNameTakeUluMulu.
  ///
  /// In en, this message translates to:
  /// **'Take Ulu-Mulu'**
  String get skillNameTakeUluMulu;

  /// No description provided for @skillNameOrcWeapons.
  ///
  /// In en, this message translates to:
  /// **'Orc Weapons'**
  String get skillNameOrcWeapons;

  /// No description provided for @skillNameMining.
  ///
  /// In en, this message translates to:
  /// **'Mining'**
  String get skillNameMining;

  /// No description provided for @skillNameDiving.
  ///
  /// In en, this message translates to:
  /// **'Diving'**
  String get skillNameDiving;

  /// No description provided for @skillNameTakeMinecrawlerMandibles.
  ///
  /// In en, this message translates to:
  /// **'Extract Mandibles'**
  String get skillNameTakeMinecrawlerMandibles;

  /// No description provided for @skillNameTakeShadowbeastHorn.
  ///
  /// In en, this message translates to:
  /// **'Take Horn (Shadowbeast)'**
  String get skillNameTakeShadowbeastHorn;

  /// No description provided for @skillNameTakeSpines.
  ///
  /// In en, this message translates to:
  /// **'Extract Spine'**
  String get skillNameTakeSpines;

  /// No description provided for @skillNameBreakSwampsharkTeeth.
  ///
  /// In en, this message translates to:
  /// **'Extract Shark Teeth'**
  String get skillNameBreakSwampsharkTeeth;

  /// No description provided for @skillNameTakeFireTongue.
  ///
  /// In en, this message translates to:
  /// **'Take Tongue of Fire'**
  String get skillNameTakeFireTongue;

  /// No description provided for @skillNameTakeTrollHorn.
  ///
  /// In en, this message translates to:
  /// **'Take Horn (Troll)'**
  String get skillNameTakeTrollHorn;

  /// No description provided for @skillNameAcrobatics.
  ///
  /// In en, this message translates to:
  /// **'Acrobatics'**
  String get skillNameAcrobatics;

  /// No description provided for @skillNameWallClimbing.
  ///
  /// In en, this message translates to:
  /// **'Climbing'**
  String get skillNameWallClimbing;

  /// No description provided for @skillNameRiding.
  ///
  /// In en, this message translates to:
  /// **'Scavenger Riding'**
  String get skillNameRiding;

  /// No description provided for @skillNameSneaking.
  ///
  /// In en, this message translates to:
  /// **'Sneaking'**
  String get skillNameSneaking;

  /// No description provided for @skillNameAlchemy.
  ///
  /// In en, this message translates to:
  /// **'Alchemy'**
  String get skillNameAlchemy;

  /// No description provided for @skillNameRuneInscription.
  ///
  /// In en, this message translates to:
  /// **'Inscription'**
  String get skillNameRuneInscription;

  /// No description provided for @skillNameBlacksmithing.
  ///
  /// In en, this message translates to:
  /// **'Smithing'**
  String get skillNameBlacksmithing;

  /// No description provided for @skillNameMagicCircle.
  ///
  /// In en, this message translates to:
  /// **'Magic Circle'**
  String get skillNameMagicCircle;

  /// No description provided for @skillNameOrcish.
  ///
  /// In en, this message translates to:
  /// **'Orcish'**
  String get skillNameOrcish;

  /// No description provided for @tabInventory.
  ///
  /// In en, this message translates to:
  /// **'Inventory'**
  String get tabInventory;

  /// No description provided for @tabTrade.
  ///
  /// In en, this message translates to:
  /// **'Trade'**
  String get tabTrade;

  /// No description provided for @traderNotAMerchant.
  ///
  /// In en, this message translates to:
  /// **'This character does not trade.'**
  String get traderNotAMerchant;

  /// No description provided for @traderRetry.
  ///
  /// In en, this message translates to:
  /// **'Try again'**
  String get traderRetry;

  /// No description provided for @traderAmbiguousName.
  ///
  /// In en, this message translates to:
  /// **'More than one trader record carries this name, so the editor cannot tell which shop belongs to this character. Editing is disabled rather than risk changing the wrong one.'**
  String get traderAmbiguousName;

  /// No description provided for @traderOre.
  ///
  /// In en, this message translates to:
  /// **'Ore (purchasing power)'**
  String get traderOre;

  /// No description provided for @traderNoOre.
  ///
  /// In en, this message translates to:
  /// **'no ore'**
  String get traderNoOre;

  /// No description provided for @traderStockCurrent.
  ///
  /// In en, this message translates to:
  /// **'Stock'**
  String get traderStockCurrent;

  /// No description provided for @traderStockBase.
  ///
  /// In en, this message translates to:
  /// **'Restock baseline'**
  String get traderStockBase;

  /// No description provided for @traderStockBaseHint.
  ///
  /// In en, this message translates to:
  /// **'What the merchant restocks back toward. It grows with story progress, so it is not a vanilla snapshot.'**
  String get traderStockBaseHint;

  /// No description provided for @traderOreHint.
  ///
  /// In en, this message translates to:
  /// **'The in-game figure differs: on load the game adds what accrued since his last trade — he sells surplus goods and restocks from it. This number is the starting point, not what the trade screen shows.'**
  String get traderOreHint;

  /// No description provided for @traderPriceWarning.
  ///
  /// In en, this message translates to:
  /// **'Prices react to how much a merchant stocks and how much ore he holds, so changing these numbers can also move what he charges.'**
  String get traderPriceWarning;

  /// No description provided for @traderAddItem.
  ///
  /// In en, this message translates to:
  /// **'Add item'**
  String get traderAddItem;

  /// No description provided for @traderRemoveItem.
  ///
  /// In en, this message translates to:
  /// **'Remove line'**
  String get traderRemoveItem;

  /// No description provided for @traderReadOnlyCore.
  ///
  /// In en, this message translates to:
  /// **'This core build can only read trader data.'**
  String get traderReadOnlyCore;

  /// No description provided for @traderDifficultyStockUnsupported.
  ///
  /// In en, this message translates to:
  /// **'This merchant carries per-difficulty stock, which the editor does not model. Editing is disabled here, because a change would look successful while leaving that extra stock untouched.'**
  String get traderDifficultyStockUnsupported;

  /// No description provided for @traderRecordIncomplete.
  ///
  /// In en, this message translates to:
  /// **'This merchant\'s stock lists are missing, or in a shape the editor does not support and cannot write. Editing is disabled here so a change cannot fail at save time.'**
  String get traderRecordIncomplete;

  /// No description provided for @traderEmptyStock.
  ///
  /// In en, this message translates to:
  /// **'Nothing in stock.'**
  String get traderEmptyStock;

  /// No description provided for @traderUnknownItem.
  ///
  /// In en, this message translates to:
  /// **'not in the item catalog'**
  String get traderUnknownItem;

  /// No description provided for @editorTradersLoadFailed.
  ///
  /// In en, this message translates to:
  /// **'Trader load failed: {details}'**
  String editorTradersLoadFailed(String details);

  /// No description provided for @traderStockLineCount.
  ///
  /// In en, this message translates to:
  /// **'{count} lines'**
  String traderStockLineCount(int count);

  /// No description provided for @tabWorld.
  ///
  /// In en, this message translates to:
  /// **'World'**
  String get tabWorld;

  /// No description provided for @tabCharacters.
  ///
  /// In en, this message translates to:
  /// **'Characters'**
  String get tabCharacters;

  /// No description provided for @characterNoActorBody.
  ///
  /// In en, this message translates to:
  /// **'This character has no in-world actor, so it has no attributes, inventory, or events.'**
  String get characterNoActorBody;

  /// No description provided for @characterNoEventsBody.
  ///
  /// In en, this message translates to:
  /// **'No events for this character.'**
  String get characterNoEventsBody;

  /// No description provided for @characterOrphanGroup.
  ///
  /// In en, this message translates to:
  /// **'Other'**
  String get characterOrphanGroup;

  /// No description provided for @tabAllData.
  ///
  /// In en, this message translates to:
  /// **'All data'**
  String get tabAllData;

  /// No description provided for @tabBackups.
  ///
  /// In en, this message translates to:
  /// **'Backups'**
  String get tabBackups;

  /// No description provided for @tabSettings.
  ///
  /// In en, this message translates to:
  /// **'Settings'**
  String get tabSettings;

  /// No description provided for @reset.
  ///
  /// In en, this message translates to:
  /// **'Reset'**
  String get reset;

  /// No description provided for @save.
  ///
  /// In en, this message translates to:
  /// **'Save'**
  String get save;

  /// No description provided for @saveWithCount.
  ///
  /// In en, this message translates to:
  /// **'Save ({count})'**
  String saveWithCount(int count);

  /// No description provided for @ok.
  ///
  /// In en, this message translates to:
  /// **'OK'**
  String get ok;

  /// No description provided for @cancel.
  ///
  /// In en, this message translates to:
  /// **'Cancel'**
  String get cancel;

  /// No description provided for @confirm.
  ///
  /// In en, this message translates to:
  /// **'Confirm'**
  String get confirm;

  /// No description provided for @close.
  ///
  /// In en, this message translates to:
  /// **'Close'**
  String get close;

  /// No description provided for @add.
  ///
  /// In en, this message translates to:
  /// **'Add'**
  String get add;

  /// No description provided for @equippedBadge.
  ///
  /// In en, this message translates to:
  /// **'Equipped'**
  String get equippedBadge;

  /// No description provided for @armorUpgradesLabel.
  ///
  /// In en, this message translates to:
  /// **'Upgrades'**
  String get armorUpgradesLabel;

  /// No description provided for @browse.
  ///
  /// In en, this message translates to:
  /// **'Browse'**
  String get browse;

  /// No description provided for @noSavFilesFound.
  ///
  /// In en, this message translates to:
  /// **'No .sav files found'**
  String get noSavFilesFound;

  /// No description provided for @profile.
  ///
  /// In en, this message translates to:
  /// **'Profile'**
  String get profile;

  /// No description provided for @otherSaves.
  ///
  /// In en, this message translates to:
  /// **'Other saves'**
  String get otherSaves;

  /// No description provided for @profileWithSaves.
  ///
  /// In en, this message translates to:
  /// **'{name} ({count} saves)'**
  String profileWithSaves(String name, int count);

  /// No description provided for @switchProfile.
  ///
  /// In en, this message translates to:
  /// **'Switch profile'**
  String get switchProfile;

  /// No description provided for @openSaveFile.
  ///
  /// In en, this message translates to:
  /// **'Open file'**
  String get openSaveFile;

  /// No description provided for @externalSave.
  ///
  /// In en, this message translates to:
  /// **'Externally opened save'**
  String get externalSave;

  /// No description provided for @saveProfileTitle.
  ///
  /// In en, this message translates to:
  /// **'Save profile'**
  String get saveProfileTitle;

  /// No description provided for @saveProfileDescription.
  ///
  /// In en, this message translates to:
  /// **'Assign this save to a different game profile. The save and profile index are backed up together.'**
  String get saveProfileDescription;

  /// No description provided for @saveProfileExternalHint.
  ///
  /// In en, this message translates to:
  /// **'Select a profile to import this file into the game\'s save folder and register it there. The original file remains unchanged.'**
  String get saveProfileExternalHint;

  /// No description provided for @saveProfileNoProfiles.
  ///
  /// In en, this message translates to:
  /// **'No editable game profiles were found in PersistentDataList.sav.'**
  String get saveProfileNoProfiles;

  /// No description provided for @saveProfileSelect.
  ///
  /// In en, this message translates to:
  /// **'Select profile'**
  String get saveProfileSelect;

  /// No description provided for @rescanSaveFolder.
  ///
  /// In en, this message translates to:
  /// **'Rescan save folder'**
  String get rescanSaveFolder;

  /// No description provided for @discardUnsavedChangesTitle.
  ///
  /// In en, this message translates to:
  /// **'Discard unsaved changes?'**
  String get discardUnsavedChangesTitle;

  /// No description provided for @rescanDiscardBody.
  ///
  /// In en, this message translates to:
  /// **'Rescanning reloads every save and discards your {count} unsaved {count, plural, =1{change} other{changes}}.'**
  String rescanDiscardBody(int count);

  /// No description provided for @discardAndRescan.
  ///
  /// In en, this message translates to:
  /// **'Discard and rescan'**
  String get discardAndRescan;

  /// No description provided for @chapterLabel.
  ///
  /// In en, this message translates to:
  /// **'Chapter {id}'**
  String chapterLabel(Object id);

  /// No description provided for @quickSave.
  ///
  /// In en, this message translates to:
  /// **'Quick save'**
  String get quickSave;

  /// No description provided for @autoSave.
  ///
  /// In en, this message translates to:
  /// **'Auto save'**
  String get autoSave;

  /// No description provided for @manualSave.
  ///
  /// In en, this message translates to:
  /// **'Manual save'**
  String get manualSave;

  /// No description provided for @errorTitle.
  ///
  /// In en, this message translates to:
  /// **'Error'**
  String get errorTitle;

  /// No description provided for @selectASaveTitle.
  ///
  /// In en, this message translates to:
  /// **'Select a save'**
  String get selectASaveTitle;

  /// No description provided for @selectASaveBody.
  ///
  /// In en, this message translates to:
  /// **'The save details will appear here.'**
  String get selectASaveBody;

  /// No description provided for @bytesValue.
  ///
  /// In en, this message translates to:
  /// **'{count} bytes'**
  String bytesValue(String count);

  /// No description provided for @inspectionJsonTitle.
  ///
  /// In en, this message translates to:
  /// **'Inspection JSON'**
  String get inspectionJsonTitle;

  /// No description provided for @copy.
  ///
  /// In en, this message translates to:
  /// **'Copy'**
  String get copy;

  /// No description provided for @savegameFallbackTitle.
  ///
  /// In en, this message translates to:
  /// **'Savegame'**
  String get savegameFallbackTitle;

  /// No description provided for @screenshotForSlot.
  ///
  /// In en, this message translates to:
  /// **'Screenshot for {slot}'**
  String screenshotForSlot(String slot);

  /// No description provided for @publicSaveName.
  ///
  /// In en, this message translates to:
  /// **'Name'**
  String get publicSaveName;

  /// No description provided for @gameTimeTitle.
  ///
  /// In en, this message translates to:
  /// **'Game time'**
  String get gameTimeTitle;

  /// No description provided for @gameTimeDay.
  ///
  /// In en, this message translates to:
  /// **'Day'**
  String get gameTimeDay;

  /// No description provided for @gameTimeHours.
  ///
  /// In en, this message translates to:
  /// **'Hours'**
  String get gameTimeHours;

  /// No description provided for @gameTimeMinutes.
  ///
  /// In en, this message translates to:
  /// **'Minutes'**
  String get gameTimeMinutes;

  /// No description provided for @gameTimeSeconds.
  ///
  /// In en, this message translates to:
  /// **'Seconds'**
  String get gameTimeSeconds;

  /// No description provided for @gameTimeTotal.
  ///
  /// In en, this message translates to:
  /// **'= {seconds} s total'**
  String gameTimeTotal(int seconds);

  /// No description provided for @gameTimeInvalid.
  ///
  /// In en, this message translates to:
  /// **'Enter whole numbers — day ≥ 0, hours 0–23, minutes and seconds 0–59.'**
  String get gameTimeInvalid;

  /// No description provided for @required.
  ///
  /// In en, this message translates to:
  /// **'Required'**
  String get required;

  /// No description provided for @playerLockedBody.
  ///
  /// In en, this message translates to:
  /// **'Private player edits need a compress-ready codec.'**
  String get playerLockedBody;

  /// No description provided for @heroTransform.
  ///
  /// In en, this message translates to:
  /// **'Position'**
  String get heroTransform;

  /// No description provided for @locationX.
  ///
  /// In en, this message translates to:
  /// **'Location X'**
  String get locationX;

  /// No description provided for @locationY.
  ///
  /// In en, this message translates to:
  /// **'Location Y'**
  String get locationY;

  /// No description provided for @locationZ.
  ///
  /// In en, this message translates to:
  /// **'Location Z'**
  String get locationZ;

  /// No description provided for @rotationPitch.
  ///
  /// In en, this message translates to:
  /// **'Rotation pitch'**
  String get rotationPitch;

  /// No description provided for @rotationYaw.
  ///
  /// In en, this message translates to:
  /// **'Rotation yaw'**
  String get rotationYaw;

  /// No description provided for @rotationRoll.
  ///
  /// In en, this message translates to:
  /// **'Rotation roll'**
  String get rotationRoll;

  /// No description provided for @spawnPositionSection.
  ///
  /// In en, this message translates to:
  /// **'Spawn position (reference)'**
  String get spawnPositionSection;

  /// No description provided for @resetToSpawnPosition.
  ///
  /// In en, this message translates to:
  /// **'Reset to spawn position'**
  String get resetToSpawnPosition;

  /// No description provided for @positionOutOfRange.
  ///
  /// In en, this message translates to:
  /// **'Value must be between −10,000,000 and 10,000,000'**
  String get positionOutOfRange;

  /// No description provided for @positionNotEditable.
  ///
  /// In en, this message translates to:
  /// **'The stored position could not be read for this character, so it cannot be edited.'**
  String get positionNotEditable;

  /// No description provided for @positionNeverPlaced.
  ///
  /// In en, this message translates to:
  /// **'This character has never been placed in the world (position 0, 0, 0) — the game may ignore the stored position.'**
  String get positionNeverPlaced;

  /// No description provided for @npcStayInPlace.
  ///
  /// In en, this message translates to:
  /// **'Disable his daily routine'**
  String get npcStayInPlace;

  /// No description provided for @npcStayInPlaceHint.
  ///
  /// In en, this message translates to:
  /// **'He then stays where he is.'**
  String get npcStayInPlaceHint;

  /// No description provided for @npcStayInPlaceLocked.
  ///
  /// In en, this message translates to:
  /// **'His original daily routine is not recorded, so this can no longer be undone.'**
  String get npcStayInPlaceLocked;

  /// No description provided for @npcUndoPlacement.
  ///
  /// In en, this message translates to:
  /// **'Take the move back'**
  String get npcUndoPlacement;

  /// No description provided for @npcUndoPlacementStale.
  ///
  /// In en, this message translates to:
  /// **'The savegame no longer holds what that move wrote, so restoring it would discard what happened since.'**
  String get npcUndoPlacementStale;

  /// No description provided for @positionNotReadable.
  ///
  /// In en, this message translates to:
  /// **'The stored position could not be read for this character.'**
  String get positionNotReadable;

  /// No description provided for @npcPositionReadOnly.
  ///
  /// In en, this message translates to:
  /// **'The game restores an NPC\'s position from the level, not from the savegame, so these values can be read but not changed.'**
  String get npcPositionReadOnly;

  /// No description provided for @pickLocation.
  ///
  /// In en, this message translates to:
  /// **'Choose location…'**
  String get pickLocation;

  /// No description provided for @pickLocationDialogTitle.
  ///
  /// In en, this message translates to:
  /// **'Choose a location'**
  String get pickLocationDialogTitle;

  /// No description provided for @applySpotRotation.
  ///
  /// In en, this message translates to:
  /// **'Also apply the spot\'s orientation'**
  String get applySpotRotation;

  /// No description provided for @locationAreaOther.
  ///
  /// In en, this message translates to:
  /// **'Other'**
  String get locationAreaOther;

  /// No description provided for @locationAreaCavalornValley.
  ///
  /// In en, this message translates to:
  /// **'Cavalorn Valley'**
  String get locationAreaCavalornValley;

  /// No description provided for @locationAreaEastForest.
  ///
  /// In en, this message translates to:
  /// **'East Forest'**
  String get locationAreaEastForest;

  /// No description provided for @locationAreaFogTower.
  ///
  /// In en, this message translates to:
  /// **'Fog Tower'**
  String get locationAreaFogTower;

  /// No description provided for @locationAreaIllegalWeedMixers.
  ///
  /// In en, this message translates to:
  /// **'Illegal Weed Mixers'**
  String get locationAreaIllegalWeedMixers;

  /// No description provided for @locationAreaOrcArena.
  ///
  /// In en, this message translates to:
  /// **'Orc Arena'**
  String get locationAreaOrcArena;

  /// No description provided for @locationAreaOrcGraveyard.
  ///
  /// In en, this message translates to:
  /// **'Orc Graveyard'**
  String get locationAreaOrcGraveyard;

  /// No description provided for @locationAreaShipwreck.
  ///
  /// In en, this message translates to:
  /// **'Shipwreck'**
  String get locationAreaShipwreck;

  /// No description provided for @locationAreaTundra.
  ///
  /// In en, this message translates to:
  /// **'Tundra'**
  String get locationAreaTundra;

  /// No description provided for @locationCatalogUnavailable.
  ///
  /// In en, this message translates to:
  /// **'The location catalog could not be loaded.'**
  String get locationCatalogUnavailable;

  /// No description provided for @invalid.
  ///
  /// In en, this message translates to:
  /// **'Invalid'**
  String get invalid;

  /// No description provided for @heroAttributes.
  ///
  /// In en, this message translates to:
  /// **'Hero attributes'**
  String get heroAttributes;

  /// No description provided for @attributeBase.
  ///
  /// In en, this message translates to:
  /// **'{name} base'**
  String attributeBase(String name);

  /// No description provided for @attributeCurrent.
  ///
  /// In en, this message translates to:
  /// **'{name} current'**
  String attributeCurrent(String name);

  /// Generic label for an attribute's base-value input; the attribute name is shown beside the field.
  ///
  /// In en, this message translates to:
  /// **'Base value'**
  String get attributeBaseValue;

  /// Generic label for an attribute's current-value input; the attribute name is shown beside the field.
  ///
  /// In en, this message translates to:
  /// **'Current value'**
  String get attributeCurrentValue;

  /// No description provided for @inventoryTitle.
  ///
  /// In en, this message translates to:
  /// **'Inventory'**
  String get inventoryTitle;

  /// No description provided for @inventoryEmpty.
  ///
  /// In en, this message translates to:
  /// **'This inventory is empty.'**
  String get inventoryEmpty;

  /// No description provided for @inventoryNeedsDecoded.
  ///
  /// In en, this message translates to:
  /// **'Inventory editing needs decoded private payload data from the codec.'**
  String get inventoryNeedsDecoded;

  /// No description provided for @inventoryNoStacks.
  ///
  /// In en, this message translates to:
  /// **'No item stacks found in the decoded private payload.'**
  String get inventoryNoStacks;

  /// No description provided for @resetInventoryChanges.
  ///
  /// In en, this message translates to:
  /// **'Reset inventory changes'**
  String get resetInventoryChanges;

  /// No description provided for @addItemTooltipPendingAdd.
  ///
  /// In en, this message translates to:
  /// **'Save pending changes first — one new item per save'**
  String get addItemTooltipPendingAdd;

  /// No description provided for @addItemTooltipPendingRemove.
  ///
  /// In en, this message translates to:
  /// **'Save the pending removal first — one structural change per save'**
  String get addItemTooltipPendingRemove;

  /// No description provided for @addItemTooltipPendingCount.
  ///
  /// In en, this message translates to:
  /// **'Save or reset pending count changes first — a structural edit must be saved on its own'**
  String get addItemTooltipPendingCount;

  /// No description provided for @addItemTooltipDefault.
  ///
  /// In en, this message translates to:
  /// **'Add item to inventory'**
  String get addItemTooltipDefault;

  /// No description provided for @addItemButton.
  ///
  /// In en, this message translates to:
  /// **'Add item'**
  String get addItemButton;

  /// Button: reset the actor's inventory to the game-start save
  ///
  /// In en, this message translates to:
  /// **'Reset inventory'**
  String get resetInventoryButton;

  /// No description provided for @resetInventoryTooltipDefault.
  ///
  /// In en, this message translates to:
  /// **'Replace this inventory with the game-start save\'s inventory'**
  String get resetInventoryTooltipDefault;

  /// No description provided for @resetInventoryTooltipBlocked.
  ///
  /// In en, this message translates to:
  /// **'Save or cancel the pending inventory changes first'**
  String get resetInventoryTooltipBlocked;

  /// No description provided for @pendingResetTitle.
  ///
  /// In en, this message translates to:
  /// **'Reset to game-start inventory'**
  String get pendingResetTitle;

  /// No description provided for @pendingResetSubtitle.
  ///
  /// In en, this message translates to:
  /// **'Resources level: {level}'**
  String pendingResetSubtitle(String level);

  /// No description provided for @cancelPendingReset.
  ///
  /// In en, this message translates to:
  /// **'Cancel reset'**
  String get cancelPendingReset;

  /// No description provided for @pendingAddSubtitle.
  ///
  /// In en, this message translates to:
  /// **'×{count} — pending add (not yet saved)'**
  String pendingAddSubtitle(int count);

  /// No description provided for @cancelPendingAdd.
  ///
  /// In en, this message translates to:
  /// **'Cancel pending add'**
  String get cancelPendingAdd;

  /// No description provided for @pendingRemovalSubtitle.
  ///
  /// In en, this message translates to:
  /// **'pending removal (not yet saved)'**
  String get pendingRemovalSubtitle;

  /// No description provided for @cancelPendingRemoval.
  ///
  /// In en, this message translates to:
  /// **'Cancel pending removal'**
  String get cancelPendingRemoval;

  /// No description provided for @filterItems.
  ///
  /// In en, this message translates to:
  /// **'Filter items'**
  String get filterItems;

  /// No description provided for @noItemsMatchQuery.
  ///
  /// In en, this message translates to:
  /// **'No items match \"{query}\".'**
  String noItemsMatchQuery(String query);

  /// No description provided for @pendingRemovalHidesAll.
  ///
  /// In en, this message translates to:
  /// **'The pending removal hides every item — save to apply it.'**
  String get pendingRemovalHidesAll;

  /// No description provided for @categoryWithCount.
  ///
  /// In en, this message translates to:
  /// **'{label} ({count})'**
  String categoryWithCount(String label, int count);

  /// No description provided for @itemTooltipIngredientFor.
  ///
  /// In en, this message translates to:
  /// **'Ingredient for'**
  String get itemTooltipIngredientFor;

  /// No description provided for @itemTooltipTeaches.
  ///
  /// In en, this message translates to:
  /// **'Teaches: {item}'**
  String itemTooltipTeaches(String item);

  /// No description provided for @itemTooltipValue.
  ///
  /// In en, this message translates to:
  /// **'Value'**
  String get itemTooltipValue;

  /// No description provided for @itemTooltipProtection.
  ///
  /// In en, this message translates to:
  /// **'Protection'**
  String get itemTooltipProtection;

  /// No description provided for @itemTooltipRequirements.
  ///
  /// In en, this message translates to:
  /// **'Requirements:'**
  String get itemTooltipRequirements;

  /// No description provided for @itemTooltipManaCost.
  ///
  /// In en, this message translates to:
  /// **'Mana cost'**
  String get itemTooltipManaCost;

  /// No description provided for @itemTooltipManaUpkeep.
  ///
  /// In en, this message translates to:
  /// **'Charge mana cost'**
  String get itemTooltipManaUpkeep;

  /// No description provided for @itemCategoryAll.
  ///
  /// In en, this message translates to:
  /// **'All'**
  String get itemCategoryAll;

  /// No description provided for @itemCategoryMeleeWeapon.
  ///
  /// In en, this message translates to:
  /// **'Melee weapons'**
  String get itemCategoryMeleeWeapon;

  /// No description provided for @itemCategoryRangedWeapon.
  ///
  /// In en, this message translates to:
  /// **'Ranged weapons'**
  String get itemCategoryRangedWeapon;

  /// No description provided for @itemCategoryMagic.
  ///
  /// In en, this message translates to:
  /// **'Magic'**
  String get itemCategoryMagic;

  /// No description provided for @itemCategoryWearable.
  ///
  /// In en, this message translates to:
  /// **'Wearables'**
  String get itemCategoryWearable;

  /// No description provided for @itemCategoryFood.
  ///
  /// In en, this message translates to:
  /// **'Food'**
  String get itemCategoryFood;

  /// No description provided for @itemCategoryPotion.
  ///
  /// In en, this message translates to:
  /// **'Potions'**
  String get itemCategoryPotion;

  /// No description provided for @itemCategoryMaterial.
  ///
  /// In en, this message translates to:
  /// **'Materials'**
  String get itemCategoryMaterial;

  /// No description provided for @itemCategoryDocument.
  ///
  /// In en, this message translates to:
  /// **'Documents'**
  String get itemCategoryDocument;

  /// No description provided for @itemCategoryMisc.
  ///
  /// In en, this message translates to:
  /// **'Miscellaneous'**
  String get itemCategoryMisc;

  /// No description provided for @itemCategoryArtefact.
  ///
  /// In en, this message translates to:
  /// **'Artefacts'**
  String get itemCategoryArtefact;

  /// No description provided for @itemCategoryOther.
  ///
  /// In en, this message translates to:
  /// **'Other'**
  String get itemCategoryOther;

  /// No description provided for @count.
  ///
  /// In en, this message translates to:
  /// **'Count'**
  String get count;

  /// No description provided for @min1.
  ///
  /// In en, this message translates to:
  /// **'Min 1'**
  String get min1;

  /// No description provided for @countTimes.
  ///
  /// In en, this message translates to:
  /// **'×{count}'**
  String countTimes(String count);

  /// No description provided for @deleteEquippedTooltip.
  ///
  /// In en, this message translates to:
  /// **'Can\'t delete: this item is likely equipped or assigned to a hotkey slot'**
  String get deleteEquippedTooltip;

  /// No description provided for @removeBlockedTooltip.
  ///
  /// In en, this message translates to:
  /// **'Save or reset your pending inventory changes first — an add or remove must be saved on its own'**
  String get removeBlockedTooltip;

  /// No description provided for @removeItemFromInventory.
  ///
  /// In en, this message translates to:
  /// **'Remove item from inventory'**
  String get removeItemFromInventory;

  /// No description provided for @progressionLockedBody.
  ///
  /// In en, this message translates to:
  /// **'Progression data needs decoded private payload data from the codec.'**
  String get progressionLockedBody;

  /// No description provided for @progressionNeedsTyped.
  ///
  /// In en, this message translates to:
  /// **'Structured progression data needs a fully decoded save with a verified typed parse.'**
  String get progressionNeedsTyped;

  /// No description provided for @sectionQuests.
  ///
  /// In en, this message translates to:
  /// **'Quests'**
  String get sectionQuests;

  /// No description provided for @sectionKnowledge.
  ///
  /// In en, this message translates to:
  /// **'Knowledge'**
  String get sectionKnowledge;

  /// No description provided for @sectionEvents.
  ///
  /// In en, this message translates to:
  /// **'Events'**
  String get sectionEvents;

  /// No description provided for @firstPage.
  ///
  /// In en, this message translates to:
  /// **'First page'**
  String get firstPage;

  /// No description provided for @previousPage.
  ///
  /// In en, this message translates to:
  /// **'Previous page'**
  String get previousPage;

  /// No description provided for @nextPage.
  ///
  /// In en, this message translates to:
  /// **'Next page'**
  String get nextPage;

  /// No description provided for @lastPage.
  ///
  /// In en, this message translates to:
  /// **'Last page'**
  String get lastPage;

  /// No description provided for @pageOfPages.
  ///
  /// In en, this message translates to:
  /// **'Page {page} / {total}'**
  String pageOfPages(int page, int total);

  /// No description provided for @rangeOfTotal.
  ///
  /// In en, this message translates to:
  /// **'{first}–{last} of {total}'**
  String rangeOfTotal(int first, int last, int total);

  /// No description provided for @perPage.
  ///
  /// In en, this message translates to:
  /// **'Per page:'**
  String get perPage;

  /// No description provided for @resetQuestChanges.
  ///
  /// In en, this message translates to:
  /// **'Reset quest changes'**
  String get resetQuestChanges;

  /// No description provided for @searchQuests.
  ///
  /// In en, this message translates to:
  /// **'Search quests'**
  String get searchQuests;

  /// No description provided for @allGroups.
  ///
  /// In en, this message translates to:
  /// **'All groups'**
  String get allGroups;

  /// No description provided for @groupWithCount.
  ///
  /// In en, this message translates to:
  /// **'{group} ({count})'**
  String groupWithCount(String group, Object count);

  /// No description provided for @stateLabelWithCount.
  ///
  /// In en, this message translates to:
  /// **'{label} {count}'**
  String stateLabelWithCount(String label, int count);

  /// No description provided for @questStateNone.
  ///
  /// In en, this message translates to:
  /// **'None'**
  String get questStateNone;

  /// No description provided for @questStateAvailable.
  ///
  /// In en, this message translates to:
  /// **'Available'**
  String get questStateAvailable;

  /// No description provided for @questStateRunning.
  ///
  /// In en, this message translates to:
  /// **'Running'**
  String get questStateRunning;

  /// No description provided for @questStateSucceeded.
  ///
  /// In en, this message translates to:
  /// **'Succeeded'**
  String get questStateSucceeded;

  /// No description provided for @questStateFailed.
  ///
  /// In en, this message translates to:
  /// **'Failed'**
  String get questStateFailed;

  /// No description provided for @questStateUnknown.
  ///
  /// In en, this message translates to:
  /// **'unknown'**
  String get questStateUnknown;

  /// No description provided for @dialogKnowledge.
  ///
  /// In en, this message translates to:
  /// **'Dialog Knowledge'**
  String get dialogKnowledge;

  /// No description provided for @resetKnowledgeChanges.
  ///
  /// In en, this message translates to:
  /// **'Reset knowledge changes'**
  String get resetKnowledgeChanges;

  /// No description provided for @addNpc.
  ///
  /// In en, this message translates to:
  /// **'Add NPC'**
  String get addNpc;

  /// No description provided for @searchNpcs.
  ///
  /// In en, this message translates to:
  /// **'Search NPCs'**
  String get searchNpcs;

  /// No description provided for @npcStatusRowLabel.
  ///
  /// In en, this message translates to:
  /// **'Status'**
  String get npcStatusRowLabel;

  /// No description provided for @npcStatusAlive.
  ///
  /// In en, this message translates to:
  /// **'alive'**
  String get npcStatusAlive;

  /// No description provided for @npcStatusDead.
  ///
  /// In en, this message translates to:
  /// **'dead'**
  String get npcStatusDead;

  /// No description provided for @npcRelationshipRowLabel.
  ///
  /// In en, this message translates to:
  /// **'Relationship'**
  String get npcRelationshipRowLabel;

  /// No description provided for @npcRelationshipUnavailable.
  ///
  /// In en, this message translates to:
  /// **'Relationship status unavailable'**
  String get npcRelationshipUnavailable;

  /// No description provided for @npcRelationshipAutomatic.
  ///
  /// In en, this message translates to:
  /// **'Computed by game'**
  String get npcRelationshipAutomatic;

  /// No description provided for @npcRelationshipAutomaticHint.
  ///
  /// In en, this message translates to:
  /// **'No permanent override is stored. Guild, story, area, and crime rules are evaluated in game.'**
  String get npcRelationshipAutomaticHint;

  /// No description provided for @npcRelationshipStoredHint.
  ///
  /// In en, this message translates to:
  /// **'Stored as a permanent NPC-to-player override. Guild, story, area, and crime rules can still change the effective status in game.'**
  String get npcRelationshipStoredHint;

  /// No description provided for @npcRelationshipFriend.
  ///
  /// In en, this message translates to:
  /// **'Friend'**
  String get npcRelationshipFriend;

  /// No description provided for @npcRelationshipNeutral.
  ///
  /// In en, this message translates to:
  /// **'Neutral'**
  String get npcRelationshipNeutral;

  /// No description provided for @npcRelationshipEnemy.
  ///
  /// In en, this message translates to:
  /// **'Enemy'**
  String get npcRelationshipEnemy;

  /// No description provided for @npcRelationshipPending.
  ///
  /// In en, this message translates to:
  /// **'Will be {relationship} on save'**
  String npcRelationshipPending(String relationship);

  /// No description provided for @npcStateHp.
  ///
  /// In en, this message translates to:
  /// **'HP {hp} / {maxHp}'**
  String npcStateHp(String hp, String maxHp);

  /// No description provided for @npcReviveButton.
  ///
  /// In en, this message translates to:
  /// **'Revive'**
  String get npcReviveButton;

  /// No description provided for @npcReviveQueued.
  ///
  /// In en, this message translates to:
  /// **'Will be revived on save'**
  String get npcReviveQueued;

  /// No description provided for @entriesForCharacter.
  ///
  /// In en, this message translates to:
  /// **'Entries — {name}'**
  String entriesForCharacter(String name);

  /// No description provided for @selectNpcToSeeEntries.
  ///
  /// In en, this message translates to:
  /// **'Select an NPC to see entries'**
  String get selectNpcToSeeEntries;

  /// No description provided for @addKnowledgeEntry.
  ///
  /// In en, this message translates to:
  /// **'Add knowledge entry'**
  String get addKnowledgeEntry;

  /// No description provided for @browseCatalog.
  ///
  /// In en, this message translates to:
  /// **'Browse catalog'**
  String get browseCatalog;

  /// No description provided for @alreadyExistsForCharacter.
  ///
  /// In en, this message translates to:
  /// **'Already exists for this character.'**
  String get alreadyExistsForCharacter;

  /// No description provided for @alreadyInPendingChanges.
  ///
  /// In en, this message translates to:
  /// **'Already in pending changes.'**
  String get alreadyInPendingChanges;

  /// No description provided for @duplicateCheckFailed.
  ///
  /// In en, this message translates to:
  /// **'Duplicate check failed — try again: {error}'**
  String duplicateCheckFailed(String error);

  /// No description provided for @pendingAddsCount.
  ///
  /// In en, this message translates to:
  /// **'Pending adds ({count})'**
  String pendingAddsCount(int count);

  /// No description provided for @undoAdd.
  ///
  /// In en, this message translates to:
  /// **'Undo add'**
  String get undoAdd;

  /// No description provided for @undoRemove.
  ///
  /// In en, this message translates to:
  /// **'Undo remove'**
  String get undoRemove;

  /// No description provided for @removeEntry.
  ///
  /// In en, this message translates to:
  /// **'Remove entry'**
  String get removeEntry;

  /// No description provided for @selectNpcFromList.
  ///
  /// In en, this message translates to:
  /// **'Select an NPC from the list'**
  String get selectNpcFromList;

  /// No description provided for @characterWithCount.
  ///
  /// In en, this message translates to:
  /// **'{name} ({count})'**
  String characterWithCount(String name, int count);

  /// No description provided for @memoryEvents.
  ///
  /// In en, this message translates to:
  /// **'Memory Events'**
  String get memoryEvents;

  /// No description provided for @searchCharacters.
  ///
  /// In en, this message translates to:
  /// **'Search characters'**
  String get searchCharacters;

  /// No description provided for @eventsForCharacter.
  ///
  /// In en, this message translates to:
  /// **'Events — {name}'**
  String eventsForCharacter(String name);

  /// No description provided for @selectCharacterToSeeEvents.
  ///
  /// In en, this message translates to:
  /// **'Select a character to see events'**
  String get selectCharacterToSeeEvents;

  /// No description provided for @noTags.
  ///
  /// In en, this message translates to:
  /// **'(no tags)'**
  String get noTags;

  /// No description provided for @eventSubtitle.
  ///
  /// In en, this message translates to:
  /// **'t={time}s  {affected}'**
  String eventSubtitle(String time, String affected);

  /// No description provided for @removeEvent.
  ///
  /// In en, this message translates to:
  /// **'Remove event'**
  String get removeEvent;

  /// No description provided for @removeMemoryEventTitle.
  ///
  /// In en, this message translates to:
  /// **'Remove memory event?'**
  String get removeMemoryEventTitle;

  /// No description provided for @removeMemoryEventBody.
  ///
  /// In en, this message translates to:
  /// **'Queue this memory event for removal? The save file is changed only when you press Save.'**
  String get removeMemoryEventBody;

  /// No description provided for @memoryEventRemovalQueued.
  ///
  /// In en, this message translates to:
  /// **'Event removal queued — press Save to apply it.'**
  String get memoryEventRemovalQueued;

  /// No description provided for @duplicateEvent.
  ///
  /// In en, this message translates to:
  /// **'Duplicate event'**
  String get duplicateEvent;

  /// No description provided for @duplicateMemoryEventTitle.
  ///
  /// In en, this message translates to:
  /// **'Duplicate memory event?'**
  String get duplicateMemoryEventTitle;

  /// No description provided for @duplicateMemoryEventBody.
  ///
  /// In en, this message translates to:
  /// **'Queue a duplicate of this memory event? The save file is changed only when you press Save.'**
  String get duplicateMemoryEventBody;

  /// No description provided for @memoryEventDuplicationQueued.
  ///
  /// In en, this message translates to:
  /// **'Event duplication queued — press Save to apply it.'**
  String get memoryEventDuplicationQueued;

  /// No description provided for @selectCharacterFromList.
  ///
  /// In en, this message translates to:
  /// **'Select a character from the list'**
  String get selectCharacterFromList;

  /// No description provided for @factionsSidebar.
  ///
  /// In en, this message translates to:
  /// **'Factions'**
  String get factionsSidebar;

  /// No description provided for @factionsForgiveButton.
  ///
  /// In en, this message translates to:
  /// **'Forgive'**
  String get factionsForgiveButton;

  /// No description provided for @factionHostile.
  ///
  /// In en, this message translates to:
  /// **'Hostile'**
  String get factionHostile;

  /// No description provided for @factionFriendly.
  ///
  /// In en, this message translates to:
  /// **'Friendly'**
  String get factionFriendly;

  /// No description provided for @crimeMurder.
  ///
  /// In en, this message translates to:
  /// **'{count, plural, =1{{count} murder} other{{count} murders}}'**
  String crimeMurder(int count);

  /// No description provided for @crimeAssault.
  ///
  /// In en, this message translates to:
  /// **'{count, plural, =1{{count} assault} other{{count} assaults}}'**
  String crimeAssault(int count);

  /// No description provided for @crimeTheft.
  ///
  /// In en, this message translates to:
  /// **'{count, plural, =1{{count} theft} other{{count} thefts}}'**
  String crimeTheft(int count);

  /// No description provided for @crimeTrespassing.
  ///
  /// In en, this message translates to:
  /// **'{count, plural, =1{{count} trespass} other{{count} trespasses}}'**
  String crimeTrespassing(int count);

  /// No description provided for @crimeThreat.
  ///
  /// In en, this message translates to:
  /// **'{count, plural, =1{{count} threat} other{{count} threats}}'**
  String crimeThreat(int count);

  /// No description provided for @crimeOther.
  ///
  /// In en, this message translates to:
  /// **'{count, plural, =1{{count} other crime} other{{count} other crimes}}'**
  String crimeOther(int count);

  /// No description provided for @factionsForgiveQueued.
  ///
  /// In en, this message translates to:
  /// **'being forgiven…'**
  String get factionsForgiveQueued;

  /// No description provided for @factionsEmpty.
  ///
  /// In en, this message translates to:
  /// **'No open crimes against factions.'**
  String get factionsEmpty;

  /// No description provided for @factionGuildOldCamp.
  ///
  /// In en, this message translates to:
  /// **'Old Camp'**
  String get factionGuildOldCamp;

  /// No description provided for @factionGuildNewCamp.
  ///
  /// In en, this message translates to:
  /// **'New Camp'**
  String get factionGuildNewCamp;

  /// No description provided for @factionGuildSwampCamp.
  ///
  /// In en, this message translates to:
  /// **'Swamp Camp'**
  String get factionGuildSwampCamp;

  /// No description provided for @factionGuildOther.
  ///
  /// In en, this message translates to:
  /// **'Others / individuals'**
  String get factionGuildOther;

  /// No description provided for @allDataLockedBody.
  ///
  /// In en, this message translates to:
  /// **'The exhaustive source browser is currently available for GSAV save files.'**
  String get allDataLockedBody;

  /// No description provided for @allDataDescription.
  ///
  /// In en, this message translates to:
  /// **'Browse GSAV metadata and every typed PUBLIC/PRIVATE node. Safe scalar and native-struct values are editable; containers and opaque bytes remain visible.'**
  String get allDataDescription;

  /// No description provided for @allDataEditable.
  ///
  /// In en, this message translates to:
  /// **'Editable'**
  String get allDataEditable;

  /// No description provided for @allDataReadOnly.
  ///
  /// In en, this message translates to:
  /// **'Read-only'**
  String get allDataReadOnly;

  /// No description provided for @allDataType.
  ///
  /// In en, this message translates to:
  /// **'Type'**
  String get allDataType;

  /// No description provided for @allDataScalars.
  ///
  /// In en, this message translates to:
  /// **'Scalars'**
  String get allDataScalars;

  /// No description provided for @allDataStructs.
  ///
  /// In en, this message translates to:
  /// **'Structs'**
  String get allDataStructs;

  /// No description provided for @allDataContainers.
  ///
  /// In en, this message translates to:
  /// **'Containers'**
  String get allDataContainers;

  /// No description provided for @allDataOpaque.
  ///
  /// In en, this message translates to:
  /// **'Opaque'**
  String get allDataOpaque;

  /// No description provided for @allDataNodes.
  ///
  /// In en, this message translates to:
  /// **'Nodes'**
  String get allDataNodes;

  /// No description provided for @allDataChildren.
  ///
  /// In en, this message translates to:
  /// **'{count, plural, =1{1 child} other{{count} children}}'**
  String allDataChildren(int count);

  /// No description provided for @allDataPending.
  ///
  /// In en, this message translates to:
  /// **'Pending'**
  String get allDataPending;

  /// No description provided for @allDataTagInputHint.
  ///
  /// In en, this message translates to:
  /// **'Comma- or line-separated tags'**
  String get allDataTagInputHint;

  /// No description provided for @allDataTypedSource.
  ///
  /// In en, this message translates to:
  /// **'{source} typed'**
  String allDataTypedSource(String source);

  /// No description provided for @searchPropertiesLabel.
  ///
  /// In en, this message translates to:
  /// **'Search properties (empty = list everything) — e.g. Health, GameTime'**
  String get searchPropertiesLabel;

  /// No description provided for @decodingSaveTitle.
  ///
  /// In en, this message translates to:
  /// **'Decoding save…'**
  String get decodingSaveTitle;

  /// No description provided for @decodingSaveBody.
  ///
  /// In en, this message translates to:
  /// **'Decoding the full private payload for the first search. This runs once per save, then searches are instant.'**
  String get decodingSaveBody;

  /// No description provided for @searchTheSaveTitle.
  ///
  /// In en, this message translates to:
  /// **'Search the save'**
  String get searchTheSaveTitle;

  /// No description provided for @searchTheSaveBody.
  ///
  /// In en, this message translates to:
  /// **'Type a property name and press enter. Leave it empty to list everything.'**
  String get searchTheSaveBody;

  /// No description provided for @searchFailedTitle.
  ///
  /// In en, this message translates to:
  /// **'Search failed'**
  String get searchFailedTitle;

  /// No description provided for @noMatchesTitle.
  ///
  /// In en, this message translates to:
  /// **'No matches'**
  String get noMatchesTitle;

  /// No description provided for @noMatchesBody.
  ///
  /// In en, this message translates to:
  /// **'No property path contained all of those terms.'**
  String get noMatchesBody;

  /// No description provided for @value.
  ///
  /// In en, this message translates to:
  /// **'Value'**
  String get value;

  /// No description provided for @backupsTitle.
  ///
  /// In en, this message translates to:
  /// **'Backups'**
  String get backupsTitle;

  /// No description provided for @refreshBackups.
  ///
  /// In en, this message translates to:
  /// **'Refresh backups'**
  String get refreshBackups;

  /// No description provided for @noBackupsTitle.
  ///
  /// In en, this message translates to:
  /// **'No backups'**
  String get noBackupsTitle;

  /// No description provided for @noBackupsBody.
  ///
  /// In en, this message translates to:
  /// **'Edited saves create backup files next to the selected slot.'**
  String get noBackupsBody;

  /// No description provided for @slotBackups.
  ///
  /// In en, this message translates to:
  /// **'Slot backups'**
  String get slotBackups;

  /// No description provided for @profileBackups.
  ///
  /// In en, this message translates to:
  /// **'Profile backups'**
  String get profileBackups;

  /// No description provided for @backupFactName.
  ///
  /// In en, this message translates to:
  /// **'Name'**
  String get backupFactName;

  /// No description provided for @backupFactSlot.
  ///
  /// In en, this message translates to:
  /// **'Slot'**
  String get backupFactSlot;

  /// No description provided for @backupFactCreated.
  ///
  /// In en, this message translates to:
  /// **'Created'**
  String get backupFactCreated;

  /// No description provided for @backupFactSize.
  ///
  /// In en, this message translates to:
  /// **'Size'**
  String get backupFactSize;

  /// No description provided for @backupFactStatus.
  ///
  /// In en, this message translates to:
  /// **'Status'**
  String get backupFactStatus;

  /// No description provided for @backupFactSha1.
  ///
  /// In en, this message translates to:
  /// **'SHA-1'**
  String get backupFactSha1;

  /// No description provided for @restoreBackupTooltip.
  ///
  /// In en, this message translates to:
  /// **'Restore {fileName}'**
  String restoreBackupTooltip(String fileName);

  /// No description provided for @appearanceTitle.
  ///
  /// In en, this message translates to:
  /// **'Appearance'**
  String get appearanceTitle;

  /// No description provided for @uiFont.
  ///
  /// In en, this message translates to:
  /// **'Font'**
  String get uiFont;

  /// No description provided for @theme.
  ///
  /// In en, this message translates to:
  /// **'Theme'**
  String get theme;

  /// No description provided for @themeLight.
  ///
  /// In en, this message translates to:
  /// **'Light'**
  String get themeLight;

  /// No description provided for @themeDark.
  ///
  /// In en, this message translates to:
  /// **'Dark'**
  String get themeDark;

  /// No description provided for @themeSystem.
  ///
  /// In en, this message translates to:
  /// **'System'**
  String get themeSystem;

  /// No description provided for @uiScale.
  ///
  /// In en, this message translates to:
  /// **'UI scale'**
  String get uiScale;

  /// No description provided for @resetZoomTooltip.
  ///
  /// In en, this message translates to:
  /// **'Reset zoom (Ctrl+0)'**
  String get resetZoomTooltip;

  /// No description provided for @zoomTip.
  ///
  /// In en, this message translates to:
  /// **'Tip: Ctrl + / Ctrl - changes the zoom anywhere in the app.'**
  String get zoomTip;

  /// No description provided for @language.
  ///
  /// In en, this message translates to:
  /// **'Language'**
  String get language;

  /// No description provided for @updatesTitle.
  ///
  /// In en, this message translates to:
  /// **'Updates'**
  String get updatesTitle;

  /// No description provided for @checkForUpdatesAutomatically.
  ///
  /// In en, this message translates to:
  /// **'Check for updates automatically'**
  String get checkForUpdatesAutomatically;

  /// No description provided for @checkForUpdatesNow.
  ///
  /// In en, this message translates to:
  /// **'Check for updates now'**
  String get checkForUpdatesNow;

  /// No description provided for @updatesPortableNotice.
  ///
  /// In en, this message translates to:
  /// **'The portable version opens the download page in your browser. Replace your existing files with the new download.'**
  String get updatesPortableNotice;

  /// No description provided for @updateAvailableTitle.
  ///
  /// In en, this message translates to:
  /// **'Update available'**
  String get updateAvailableTitle;

  /// No description provided for @updateAvailableMessage.
  ///
  /// In en, this message translates to:
  /// **'Version {version} is available. You have {current}.'**
  String updateAvailableMessage(Object version, Object current);

  /// No description provided for @updateDownload.
  ///
  /// In en, this message translates to:
  /// **'Download'**
  String get updateDownload;

  /// No description provided for @updateOpenFailed.
  ///
  /// In en, this message translates to:
  /// **'Could not open the download page. You can reach it at {url}'**
  String updateOpenFailed(String url);

  /// No description provided for @updateLater.
  ///
  /// In en, this message translates to:
  /// **'Later'**
  String get updateLater;

  /// No description provided for @updateUpToDate.
  ///
  /// In en, this message translates to:
  /// **'You are using the latest version.'**
  String get updateUpToDate;

  /// No description provided for @updateCheckFailed.
  ///
  /// In en, this message translates to:
  /// **'Could not check for updates. Please try again later.'**
  String get updateCheckFailed;

  /// No description provided for @gameTextTitle.
  ///
  /// In en, this message translates to:
  /// **'Game text'**
  String get gameTextTitle;

  /// No description provided for @itemImagesTitle.
  ///
  /// In en, this message translates to:
  /// **'Item images'**
  String get itemImagesTitle;

  /// No description provided for @gameDataTitle.
  ///
  /// In en, this message translates to:
  /// **'Game data'**
  String get gameDataTitle;

  /// No description provided for @itemImagesReady.
  ///
  /// In en, this message translates to:
  /// **'{count} item images are ready.'**
  String itemImagesReady(int count);

  /// No description provided for @itemImagesUnavailable.
  ///
  /// In en, this message translates to:
  /// **'Item images are not available. Category icons will be used instead.'**
  String get itemImagesUnavailable;

  /// No description provided for @checkRefreshItemImages.
  ///
  /// In en, this message translates to:
  /// **'Check / refresh item images'**
  String get checkRefreshItemImages;

  /// No description provided for @gameDataSourceMissing.
  ///
  /// In en, this message translates to:
  /// **'Game text could not be prepared automatically. You can select the localization cache in Settings.'**
  String get gameDataSourceMissing;

  /// No description provided for @loadingTexts.
  ///
  /// In en, this message translates to:
  /// **'Loading texts…'**
  String get loadingTexts;

  /// No description provided for @loadingImages.
  ///
  /// In en, this message translates to:
  /// **'Loading images…'**
  String get loadingImages;

  /// No description provided for @preparing.
  ///
  /// In en, this message translates to:
  /// **'Preparing…'**
  String get preparing;

  /// No description provided for @gameTextExtractedWithCounts.
  ///
  /// In en, this message translates to:
  /// **'Extracted: {ids} ids across {languages} languages.'**
  String gameTextExtractedWithCounts(int ids, int languages);

  /// No description provided for @gameTextExtracted.
  ///
  /// In en, this message translates to:
  /// **'Localized game text is extracted.'**
  String get gameTextExtracted;

  /// No description provided for @gameTextNotExtracted.
  ///
  /// In en, this message translates to:
  /// **'Localized game text is not extracted yet.'**
  String get gameTextNotExtracted;

  /// No description provided for @extracting.
  ///
  /// In en, this message translates to:
  /// **'Extracting…'**
  String get extracting;

  /// No description provided for @extractRefreshLocalizedText.
  ///
  /// In en, this message translates to:
  /// **'Extract / refresh localized text'**
  String get extractRefreshLocalizedText;

  /// No description provided for @extractionComplete.
  ///
  /// In en, this message translates to:
  /// **'Extraction complete'**
  String get extractionComplete;

  /// No description provided for @extractionFailed.
  ///
  /// In en, this message translates to:
  /// **'Extraction failed'**
  String get extractionFailed;

  /// No description provided for @localizationCacheFileType.
  ///
  /// In en, this message translates to:
  /// **'Localization cache'**
  String get localizationCacheFileType;

  /// No description provided for @savegameDirectoryTitle.
  ///
  /// In en, this message translates to:
  /// **'Savegame directory'**
  String get savegameDirectoryTitle;

  /// No description provided for @folder.
  ///
  /// In en, this message translates to:
  /// **'Folder'**
  String get folder;

  /// No description provided for @codecTitle.
  ///
  /// In en, this message translates to:
  /// **'Codec'**
  String get codecTitle;

  /// No description provided for @check.
  ///
  /// In en, this message translates to:
  /// **'Check'**
  String get check;

  /// No description provided for @roundtrip.
  ///
  /// In en, this message translates to:
  /// **'Roundtrip'**
  String get roundtrip;

  /// No description provided for @noCodecStatus.
  ///
  /// In en, this message translates to:
  /// **'No codec status'**
  String get noCodecStatus;

  /// No description provided for @codecReady.
  ///
  /// In en, this message translates to:
  /// **'Codec ready'**
  String get codecReady;

  /// No description provided for @codecReadOnly.
  ///
  /// In en, this message translates to:
  /// **'Codec read-only'**
  String get codecReadOnly;

  /// No description provided for @codecUnavailable.
  ///
  /// In en, this message translates to:
  /// **'Codec unavailable'**
  String get codecUnavailable;

  /// No description provided for @details.
  ///
  /// In en, this message translates to:
  /// **'Details'**
  String get details;

  /// No description provided for @codecStatusLine.
  ///
  /// In en, this message translates to:
  /// **'Status: {status}'**
  String codecStatusLine(String status);

  /// No description provided for @codecCapabilityLine.
  ///
  /// In en, this message translates to:
  /// **'Decompress: {decompress} | Compress: {compress}'**
  String codecCapabilityLine(String decompress, String compress);

  /// No description provided for @codecBackendLine.
  ///
  /// In en, this message translates to:
  /// **'Backend: {backend}'**
  String codecBackendLine(String backend);

  /// No description provided for @yes.
  ///
  /// In en, this message translates to:
  /// **'yes'**
  String get yes;

  /// No description provided for @no.
  ///
  /// In en, this message translates to:
  /// **'no'**
  String get no;

  /// No description provided for @aboutVersion.
  ///
  /// In en, this message translates to:
  /// **'Version {version} ({sha})'**
  String aboutVersion(String version, String sha);

  /// No description provided for @aboutCopyright.
  ///
  /// In en, this message translates to:
  /// **'© 2026 Daniel Hoer'**
  String get aboutCopyright;

  /// No description provided for @aboutLicense.
  ///
  /// In en, this message translates to:
  /// **'Licensed under the MIT License.'**
  String get aboutLicense;

  /// No description provided for @difficultyTitle.
  ///
  /// In en, this message translates to:
  /// **'Difficulty — {profile}'**
  String difficultyTitle(String profile);

  /// No description provided for @difficultyNoProfile.
  ///
  /// In en, this message translates to:
  /// **'No profile'**
  String get difficultyNoProfile;

  /// No description provided for @difficultyNoDifficulty.
  ///
  /// In en, this message translates to:
  /// **'No difficulty'**
  String get difficultyNoDifficulty;

  /// No description provided for @difficultyLabel.
  ///
  /// In en, this message translates to:
  /// **'Difficulty'**
  String get difficultyLabel;

  /// No description provided for @difficultyTooltipNoProfile.
  ///
  /// In en, this message translates to:
  /// **'No profile selected'**
  String get difficultyTooltipNoProfile;

  /// No description provided for @difficultyTooltipEdit.
  ///
  /// In en, this message translates to:
  /// **'Edit difficulty for this profile'**
  String get difficultyTooltipEdit;

  /// No description provided for @difficultyTooltipNoEditable.
  ///
  /// In en, this message translates to:
  /// **'This profile has no editable difficulty'**
  String get difficultyTooltipNoEditable;

  /// No description provided for @preset.
  ///
  /// In en, this message translates to:
  /// **'Preset'**
  String get preset;

  /// No description provided for @presetNovice.
  ///
  /// In en, this message translates to:
  /// **'Novice'**
  String get presetNovice;

  /// No description provided for @presetGothic.
  ///
  /// In en, this message translates to:
  /// **'Gothic'**
  String get presetGothic;

  /// No description provided for @presetHard.
  ///
  /// In en, this message translates to:
  /// **'Hard'**
  String get presetHard;

  /// No description provided for @presetCustom.
  ///
  /// In en, this message translates to:
  /// **'Custom'**
  String get presetCustom;

  /// No description provided for @unrecognisedPreset.
  ///
  /// In en, this message translates to:
  /// **'Stored preset is unrecognised ({preset}). You can still save Flow Helper / Permadeath changes, or pick a preset above to overwrite it.'**
  String unrecognisedPreset(Object preset);

  /// No description provided for @closeCombatFlowHelper.
  ///
  /// In en, this message translates to:
  /// **'Close Combat Flow Helper'**
  String get closeCombatFlowHelper;

  /// No description provided for @permadeath.
  ///
  /// In en, this message translates to:
  /// **'Permadeath'**
  String get permadeath;

  /// No description provided for @notAvailableOnNovice.
  ///
  /// In en, this message translates to:
  /// **'Not available on Novice'**
  String get notAvailableOnNovice;

  /// No description provided for @levelCombat.
  ///
  /// In en, this message translates to:
  /// **'Combat'**
  String get levelCombat;

  /// No description provided for @levelResources.
  ///
  /// In en, this message translates to:
  /// **'Resources'**
  String get levelResources;

  /// No description provided for @levelProgression.
  ///
  /// In en, this message translates to:
  /// **'Progression'**
  String get levelProgression;

  /// No description provided for @difficultyAppliesToAllSaves.
  ///
  /// In en, this message translates to:
  /// **'Difficulty applies to all saves in this profile.'**
  String get difficultyAppliesToAllSaves;

  /// No description provided for @savingDifficultyFailed.
  ///
  /// In en, this message translates to:
  /// **'Saving difficulty failed.'**
  String get savingDifficultyFailed;

  /// No description provided for @addItemDialogTitle.
  ///
  /// In en, this message translates to:
  /// **'Add item'**
  String get addItemDialogTitle;

  /// No description provided for @searchItems.
  ///
  /// In en, this message translates to:
  /// **'Search items'**
  String get searchItems;

  /// No description provided for @failedToLoadCatalog.
  ///
  /// In en, this message translates to:
  /// **'Failed to load catalog: {error}'**
  String failedToLoadCatalog(String error);

  /// No description provided for @noItemsAvailableToAdd.
  ///
  /// In en, this message translates to:
  /// **'No items available to add'**
  String get noItemsAvailableToAdd;

  /// No description provided for @noItemsMatch.
  ///
  /// In en, this message translates to:
  /// **'No items match'**
  String get noItemsMatch;

  /// No description provided for @countMustBeAtLeast1.
  ///
  /// In en, this message translates to:
  /// **'Must be ≥ 1'**
  String get countMustBeAtLeast1;

  /// No description provided for @countMustBeAtMost.
  ///
  /// In en, this message translates to:
  /// **'Must be ≤ {max}'**
  String countMustBeAtMost(int max);

  /// No description provided for @addNpcDialogTitle.
  ///
  /// In en, this message translates to:
  /// **'Add NPC'**
  String get addNpcDialogTitle;

  /// No description provided for @noNpcsAvailableToAdd.
  ///
  /// In en, this message translates to:
  /// **'No NPCs available to add'**
  String get noNpcsAvailableToAdd;

  /// No description provided for @noNpcsMatch.
  ///
  /// In en, this message translates to:
  /// **'No NPCs match'**
  String get noNpcsMatch;

  /// No description provided for @categoryAll.
  ///
  /// In en, this message translates to:
  /// **'All'**
  String get categoryAll;

  /// No description provided for @allWithCount.
  ///
  /// In en, this message translates to:
  /// **'All ({count})'**
  String allWithCount(int count);

  /// No description provided for @addKnowledgeEntryDialogTitle.
  ///
  /// In en, this message translates to:
  /// **'Add knowledge entry'**
  String get addKnowledgeEntryDialogTitle;

  /// No description provided for @searchEntries.
  ///
  /// In en, this message translates to:
  /// **'Search entries'**
  String get searchEntries;

  /// No description provided for @noKnowledgeEntriesAvailableToAdd.
  ///
  /// In en, this message translates to:
  /// **'No knowledge entries available to add'**
  String get noKnowledgeEntriesAvailableToAdd;

  /// No description provided for @noEntriesMatch.
  ///
  /// In en, this message translates to:
  /// **'No entries match'**
  String get noEntriesMatch;

  /// No description provided for @heroGroupMainStats.
  ///
  /// In en, this message translates to:
  /// **'Main Stats'**
  String get heroGroupMainStats;

  /// No description provided for @heroGroupCombatMovement.
  ///
  /// In en, this message translates to:
  /// **'Combat / Movement'**
  String get heroGroupCombatMovement;

  /// No description provided for @heroGroupResistances.
  ///
  /// In en, this message translates to:
  /// **'Resistances'**
  String get heroGroupResistances;

  /// No description provided for @heroGroupThieving.
  ///
  /// In en, this message translates to:
  /// **'Thieving'**
  String get heroGroupThieving;

  /// No description provided for @heroGroupAdvanced.
  ///
  /// In en, this message translates to:
  /// **'Advanced'**
  String get heroGroupAdvanced;

  /// No description provided for @heroGroupDiving.
  ///
  /// In en, this message translates to:
  /// **'Diving'**
  String get heroGroupDiving;

  /// No description provided for @heroDivingSkillNote.
  ///
  /// In en, this message translates to:
  /// **'Once Diving is learned, the game resets breath and recovery to the skill\'s own values every time the savegame loads. Breath used per second stays as you set it.'**
  String get heroDivingSkillNote;

  /// No description provided for @heroGroupSleep.
  ///
  /// In en, this message translates to:
  /// **'Sleep'**
  String get heroGroupSleep;

  /// No description provided for @heroGroupIntoxication.
  ///
  /// In en, this message translates to:
  /// **'Intoxication'**
  String get heroGroupIntoxication;

  /// No description provided for @heroEntryHeroTransform.
  ///
  /// In en, this message translates to:
  /// **'Position'**
  String get heroEntryHeroTransform;

  /// No description provided for @attributeEmpty.
  ///
  /// In en, this message translates to:
  /// **'{name} is empty — enter a value or restore the original before saving.'**
  String attributeEmpty(String name);

  /// No description provided for @attributeInvalidNumber.
  ///
  /// In en, this message translates to:
  /// **'Invalid number for {name}: \"{text}\"'**
  String attributeInvalidNumber(String name, String text);

  /// No description provided for @loadingEditorData.
  ///
  /// In en, this message translates to:
  /// **'Loading editor data'**
  String get loadingEditorData;

  /// No description provided for @savingProgress.
  ///
  /// In en, this message translates to:
  /// **'Saving… {done} of {total}'**
  String savingProgress(int done, int total);

  /// No description provided for @localizedTextExtractedCount.
  ///
  /// In en, this message translates to:
  /// **'Extracted {idCount} ids across {languageCount} languages'**
  String localizedTextExtractedCount(int idCount, int languageCount);

  /// No description provided for @skillSmithing1H.
  ///
  /// In en, this message translates to:
  /// **'One-Hand Smithing'**
  String get skillSmithing1H;

  /// No description provided for @skillSmithing2H.
  ///
  /// In en, this message translates to:
  /// **'Two-Hand Smithing'**
  String get skillSmithing2H;

  /// No description provided for @skillCircleNovice.
  ///
  /// In en, this message translates to:
  /// **'Novice Magician'**
  String get skillCircleNovice;

  /// No description provided for @skillCircle1.
  ///
  /// In en, this message translates to:
  /// **'First Circle of Magic'**
  String get skillCircle1;

  /// No description provided for @skillCircle2.
  ///
  /// In en, this message translates to:
  /// **'Second Circle of Magic'**
  String get skillCircle2;

  /// No description provided for @skillCircle3.
  ///
  /// In en, this message translates to:
  /// **'Third Circle of Magic'**
  String get skillCircle3;

  /// No description provided for @skillCircle4.
  ///
  /// In en, this message translates to:
  /// **'Fourth Circle of Magic'**
  String get skillCircle4;

  /// No description provided for @skillCircle5.
  ///
  /// In en, this message translates to:
  /// **'Fifth Circle of Magic'**
  String get skillCircle5;

  /// No description provided for @skillCircle6.
  ///
  /// In en, this message translates to:
  /// **'Sixth Circle of Magic'**
  String get skillCircle6;

  /// No description provided for @sectionGlossary.
  ///
  /// In en, this message translates to:
  /// **'Glossary'**
  String get sectionGlossary;

  /// No description provided for @glossarySearch.
  ///
  /// In en, this message translates to:
  /// **'Search glossary'**
  String get glossarySearch;

  /// No description provided for @glossaryOldCamp.
  ///
  /// In en, this message translates to:
  /// **'Old Camp'**
  String get glossaryOldCamp;

  /// No description provided for @glossaryNewCamp.
  ///
  /// In en, this message translates to:
  /// **'New Camp'**
  String get glossaryNewCamp;

  /// No description provided for @glossarySwampCamp.
  ///
  /// In en, this message translates to:
  /// **'Swamp Camp'**
  String get glossarySwampCamp;

  /// No description provided for @glossaryOutsiders.
  ///
  /// In en, this message translates to:
  /// **'Outsiders'**
  String get glossaryOutsiders;

  /// No description provided for @glossaryCreatures.
  ///
  /// In en, this message translates to:
  /// **'Creatures'**
  String get glossaryCreatures;

  /// No description provided for @glossaryLocations.
  ///
  /// In en, this message translates to:
  /// **'Locations'**
  String get glossaryLocations;

  /// No description provided for @glossaryFilterLabel.
  ///
  /// In en, this message translates to:
  /// **'Filter'**
  String get glossaryFilterLabel;

  /// No description provided for @glossaryFilterTraders.
  ///
  /// In en, this message translates to:
  /// **'Traders'**
  String get glossaryFilterTraders;

  /// No description provided for @glossaryFilterTeachers.
  ///
  /// In en, this message translates to:
  /// **'Teachers'**
  String get glossaryFilterTeachers;

  /// No description provided for @roleTrader.
  ///
  /// In en, this message translates to:
  /// **'Trader'**
  String get roleTrader;

  /// No description provided for @roleDead.
  ///
  /// In en, this message translates to:
  /// **'Dead'**
  String get roleDead;

  /// No description provided for @roleTeacher.
  ///
  /// In en, this message translates to:
  /// **'Teacher'**
  String get roleTeacher;

  /// No description provided for @roleArmorer.
  ///
  /// In en, this message translates to:
  /// **'Armorer'**
  String get roleArmorer;

  /// No description provided for @glossaryFilterArmorers.
  ///
  /// In en, this message translates to:
  /// **'Armorers'**
  String get glossaryFilterArmorers;

  /// No description provided for @glossaryFilterHostile.
  ///
  /// In en, this message translates to:
  /// **'Hostile'**
  String get glossaryFilterHostile;

  /// No description provided for @glossaryRelationshipFilterNote.
  ///
  /// In en, this message translates to:
  /// **'Shows permanent enemy overrides stored in the save. Dynamic guild, story, area, and crime relationships are computed only in game.'**
  String get glossaryRelationshipFilterNote;

  /// No description provided for @glossaryFilterDead.
  ///
  /// In en, this message translates to:
  /// **'Dead'**
  String get glossaryFilterDead;

  /// No description provided for @glossaryAddEntry.
  ///
  /// In en, this message translates to:
  /// **'Add glossary entry'**
  String get glossaryAddEntry;

  /// No description provided for @glossaryAddTitle.
  ///
  /// In en, this message translates to:
  /// **'Add glossary entry'**
  String get glossaryAddTitle;

  /// No description provided for @glossaryResetChanges.
  ///
  /// In en, this message translates to:
  /// **'Reset glossary changes'**
  String get glossaryResetChanges;

  /// No description provided for @glossaryNoVisibleEntries.
  ///
  /// In en, this message translates to:
  /// **'No visible glossary entries match this view.'**
  String get glossaryNoVisibleEntries;

  /// No description provided for @glossaryNoHiddenEntries.
  ///
  /// In en, this message translates to:
  /// **'Every available entry is already visible.'**
  String get glossaryNoHiddenEntries;

  /// No description provided for @glossaryNoMatch.
  ///
  /// In en, this message translates to:
  /// **'No glossary entries match.'**
  String get glossaryNoMatch;

  /// No description provided for @glossarySelectEntry.
  ///
  /// In en, this message translates to:
  /// **'Select a glossary entry to edit its entries.'**
  String get glossarySelectEntry;

  /// No description provided for @glossaryEntryCount.
  ///
  /// In en, this message translates to:
  /// **'{count} entries'**
  String glossaryEntryCount(int count);

  /// No description provided for @glossarySegmentsCount.
  ///
  /// In en, this message translates to:
  /// **'{unlocked} of {total} entries'**
  String glossarySegmentsCount(int unlocked, int total);

  /// No description provided for @glossaryPortraitUnlocked.
  ///
  /// In en, this message translates to:
  /// **'Portrait unlocked'**
  String get glossaryPortraitUnlocked;

  /// No description provided for @glossaryPortraitSilhouette.
  ///
  /// In en, this message translates to:
  /// **'Silhouette — portrait not unlocked'**
  String get glossaryPortraitSilhouette;

  /// No description provided for @glossarySegments.
  ///
  /// In en, this message translates to:
  /// **'Entries'**
  String get glossarySegments;

  /// No description provided for @glossaryPending.
  ///
  /// In en, this message translates to:
  /// **'Unsaved change'**
  String get glossaryPending;

  /// No description provided for @glossaryShowFullText.
  ///
  /// In en, this message translates to:
  /// **'Show full entry text'**
  String get glossaryShowFullText;

  /// No description provided for @glossarySegmentIntroduction.
  ///
  /// In en, this message translates to:
  /// **'Introduction / portrait'**
  String get glossarySegmentIntroduction;

  /// No description provided for @glossarySegmentUnlock.
  ///
  /// In en, this message translates to:
  /// **'Discovery'**
  String get glossarySegmentUnlock;

  /// No description provided for @glossarySegmentEntry.
  ///
  /// In en, this message translates to:
  /// **'Entry {number}'**
  String glossarySegmentEntry(int number);

  /// No description provided for @questJournalAll.
  ///
  /// In en, this message translates to:
  /// **'All quests'**
  String get questJournalAll;

  /// No description provided for @questJournalOldCamp.
  ///
  /// In en, this message translates to:
  /// **'Old Camp'**
  String get questJournalOldCamp;

  /// No description provided for @questJournalNewCamp.
  ///
  /// In en, this message translates to:
  /// **'New Camp'**
  String get questJournalNewCamp;

  /// No description provided for @questJournalSwampCamp.
  ///
  /// In en, this message translates to:
  /// **'Swamp Camp'**
  String get questJournalSwampCamp;

  /// No description provided for @questJournalColony.
  ///
  /// In en, this message translates to:
  /// **'The Colony'**
  String get questJournalColony;

  /// No description provided for @questJournalCompleted.
  ///
  /// In en, this message translates to:
  /// **'Completed'**
  String get questJournalCompleted;

  /// No description provided for @questJournalHint.
  ///
  /// In en, this message translates to:
  /// **'In-game journal view. Internal and not-yet-started quest states remain available under All Data.'**
  String get questJournalHint;

  /// No description provided for @questJournalNoEntries.
  ///
  /// In en, this message translates to:
  /// **'No journal quests match the current filters.'**
  String get questJournalNoEntries;

  /// No description provided for @glossaryTutorials.
  ///
  /// In en, this message translates to:
  /// **'Tutorials'**
  String get glossaryTutorials;

  /// No description provided for @tutorialGateNote.
  ///
  /// In en, this message translates to:
  /// **'These rows control saved tutorial unlock gates. A gate does not necessarily map one-to-one to an individual in-game tutorial page.'**
  String get tutorialGateNote;

  /// No description provided for @tutorialResetChanges.
  ///
  /// In en, this message translates to:
  /// **'Reset tutorial changes'**
  String get tutorialResetChanges;

  /// No description provided for @tutorialNoGates.
  ///
  /// In en, this message translates to:
  /// **'No tutorial unlock gates are available in this save.'**
  String get tutorialNoGates;

  /// No description provided for @tutorialGateUnlockCount.
  ///
  /// In en, this message translates to:
  /// **'{unlocked} of {total} tutorial gates unlocked'**
  String tutorialGateUnlockCount(int unlocked, int total);

  /// No description provided for @tutorialGateCombatBasics.
  ///
  /// In en, this message translates to:
  /// **'Combat basics'**
  String get tutorialGateCombatBasics;

  /// No description provided for @tutorialGateCrafting.
  ///
  /// In en, this message translates to:
  /// **'Crafting'**
  String get tutorialGateCrafting;

  /// No description provided for @tutorialGateCrime.
  ///
  /// In en, this message translates to:
  /// **'Crime and consequences'**
  String get tutorialGateCrime;

  /// No description provided for @tutorialGateDrugs.
  ///
  /// In en, this message translates to:
  /// **'Consumables and effects'**
  String get tutorialGateDrugs;

  /// No description provided for @tutorialGateLockpicking.
  ///
  /// In en, this message translates to:
  /// **'Lockpicking'**
  String get tutorialGateLockpicking;

  /// No description provided for @tutorialGateMagic.
  ///
  /// In en, this message translates to:
  /// **'Magic'**
  String get tutorialGateMagic;

  /// No description provided for @tutorialGateMap.
  ///
  /// In en, this message translates to:
  /// **'Map'**
  String get tutorialGateMap;

  /// No description provided for @tutorialGateMeleeCombat.
  ///
  /// In en, this message translates to:
  /// **'Melee combat'**
  String get tutorialGateMeleeCombat;

  /// No description provided for @tutorialGateNavigation.
  ///
  /// In en, this message translates to:
  /// **'Movement and navigation'**
  String get tutorialGateNavigation;

  /// No description provided for @tutorialGatePerception.
  ///
  /// In en, this message translates to:
  /// **'Perception'**
  String get tutorialGatePerception;

  /// No description provided for @tutorialGatePlayerProgression.
  ///
  /// In en, this message translates to:
  /// **'Character progression'**
  String get tutorialGatePlayerProgression;

  /// No description provided for @tutorialGateRanged.
  ///
  /// In en, this message translates to:
  /// **'Ranged combat'**
  String get tutorialGateRanged;

  /// No description provided for @tutorialGateRiding.
  ///
  /// In en, this message translates to:
  /// **'Riding'**
  String get tutorialGateRiding;

  /// No description provided for @tutorialGateSleep.
  ///
  /// In en, this message translates to:
  /// **'Sleeping'**
  String get tutorialGateSleep;

  /// No description provided for @tutorialGateTrading.
  ///
  /// In en, this message translates to:
  /// **'Trading'**
  String get tutorialGateTrading;

  /// No description provided for @windowMinimizeTooltip.
  ///
  /// In en, this message translates to:
  /// **'Minimize'**
  String get windowMinimizeTooltip;

  /// No description provided for @windowMaximizeTooltip.
  ///
  /// In en, this message translates to:
  /// **'Maximize'**
  String get windowMaximizeTooltip;

  /// No description provided for @windowRestoreTooltip.
  ///
  /// In en, this message translates to:
  /// **'Restore'**
  String get windowRestoreTooltip;

  /// No description provided for @fallbackDialogEntry.
  ///
  /// In en, this message translates to:
  /// **'Dialog entry'**
  String get fallbackDialogEntry;

  /// No description provided for @fallbackDialogChoice.
  ///
  /// In en, this message translates to:
  /// **'Dialog choice'**
  String get fallbackDialogChoice;

  /// No description provided for @fallbackDialogTopic.
  ///
  /// In en, this message translates to:
  /// **'Dialog topic'**
  String get fallbackDialogTopic;

  /// No description provided for @fallbackDialogInformation.
  ///
  /// In en, this message translates to:
  /// **'Dialog information'**
  String get fallbackDialogInformation;

  /// No description provided for @fallbackQuest.
  ///
  /// In en, this message translates to:
  /// **'Quest'**
  String get fallbackQuest;

  /// No description provided for @fallbackObjective.
  ///
  /// In en, this message translates to:
  /// **'Objective'**
  String get fallbackObjective;

  /// No description provided for @fallbackItem.
  ///
  /// In en, this message translates to:
  /// **'Item'**
  String get fallbackItem;

  /// No description provided for @attributeSkillPointsFallback.
  ///
  /// In en, this message translates to:
  /// **'Skill points (LP)'**
  String get attributeSkillPointsFallback;

  /// No description provided for @attributeManualFallbackLabel.
  ///
  /// In en, this message translates to:
  /// **'{attributeId, select, SuperArmor{Poise} MaxSuperArmor{Maximum poise} DamageMultiplier{Damage taken} SpeedModifier{Movement speed} Oxygen{Breath} MaxOxygen{Maximum breath} OxygenDepletionRate{Breath used per second} OxygenRecoveryRate{Breath regained per second} CriticalLevelPercent{Low-breath warning} SleepTime{Restful hours left} MaxSleepTime{Maximum restful hours} SleepTimeRecoveryAmount{Restful hours regained} SleepTimeRecoveryPeriod{Refill interval} MaxRestTime{Maximum time in bed} Health_RecoveryRatePerHourOfSleep{Health per hour of sleep} Mana_RecoveryRatePerHourOfSleep{Mana per hour of sleep} Alcohol{Alcohol level} MaxAlcohol{Maximum alcohol} AlcoholDepletionRate{Sobering speed} Swampweed{Swampweed level} MaxSwampweed{Maximum swampweed} SwampweedDepletionRate{Wear-off speed} XPExecutedBounty{XP for finishing off} XPKillOrDefeatBounty{XP for defeating} Level{Level} LockpickDurability{Lockpick durability} LockpickPrecision{Lockpick precision} PickPocketing{Pickpocketing} other{{fallback}}}'**
  String attributeManualFallbackLabel(String attributeId, String fallback);

  /// No description provided for @attributeManualTooltip.
  ///
  /// In en, this message translates to:
  /// **'{attributeId, select, SuperArmor{How much punishment this character absorbs before a hit staggers them.} MaxSuperArmor{The full poise pool; it grows with character level and with worn armour.} DamageMultiplier{Factor applied to the damage this character takes — 1 is normal, higher hurts more.} SpeedModifier{Factor on how fast this character moves — 1 is normal.} Oxygen{Seconds of air left under water; at zero this character drowns.} MaxOxygen{How many seconds this character can stay under water; the Diving skill raises it.} OxygenDepletionRate{Air used up each second while submerged.} OxygenRecoveryRate{Air that comes back each second after surfacing.} CriticalLevelPercent{Share of remaining air at which the game warns of drowning.} SleepTime{Hours of sleep that still restore something; beyond them the game grants no resting bonus.} MaxSleepTime{The largest budget of restful hours this character can hold.} SleepTimeRecoveryAmount{Restful hours added back each time the budget refills.} SleepTimeRecoveryPeriod{How long it takes before the budget of restful hours refills again.} MaxRestTime{The longest single stay in bed the game allows.} Health_RecoveryRatePerHourOfSleep{Share of maximum health restored for every hour slept.} Mana_RecoveryRatePerHourOfSleep{Share of maximum mana restored for every hour slept.} Alcohol{How drunk this character is; the higher tiers trade dexterity and mana for strength.} MaxAlcohol{The highest alcohol level this character can reach.} AlcoholDepletionRate{How quickly the alcohol level falls back towards sober.} Swampweed{How stoned this character is; the higher tiers shift their attributes around.} MaxSwampweed{The highest swampweed level this character can reach.} SwampweedDepletionRate{How quickly the swampweed high wears off.} XPExecutedBounty{Experience for killing this character while it already lies defeated on the ground.} XPKillOrDefeatBounty{Experience for bringing this character down, whether it dies or is only beaten unconscious.} Level{The character level. It rises with experience and grants learning points.} LockpickDurability{Set by the Lockpicking skill: 2 untrained, 4 trained, 6 mastered.} LockpickPrecision{Set by the Lockpicking skill: 0 untrained, 1 trained, 2 mastered.} PickPocketing{Set by the Pickpocketing skill: -30 untrained, -10 trained, +10 mastered.} other{?}}'**
  String attributeManualTooltip(String attributeId);

  /// No description provided for @knowledgeTypeVoiceLine.
  ///
  /// In en, this message translates to:
  /// **'Voice line'**
  String get knowledgeTypeVoiceLine;

  /// No description provided for @knowledgeTypeOther.
  ///
  /// In en, this message translates to:
  /// **'Other'**
  String get knowledgeTypeOther;

  /// No description provided for @armorUpgradeUpper.
  ///
  /// In en, this message translates to:
  /// **'Upper'**
  String get armorUpgradeUpper;

  /// No description provided for @armorUpgradeMiddle.
  ///
  /// In en, this message translates to:
  /// **'Middle'**
  String get armorUpgradeMiddle;

  /// No description provided for @armorUpgradeLower.
  ///
  /// In en, this message translates to:
  /// **'Lower'**
  String get armorUpgradeLower;

  /// No description provided for @knowledgeCategoryTopic.
  ///
  /// In en, this message translates to:
  /// **'Topic'**
  String get knowledgeCategoryTopic;

  /// No description provided for @knowledgeCategoryChoice.
  ///
  /// In en, this message translates to:
  /// **'Choice'**
  String get knowledgeCategoryChoice;

  /// No description provided for @knowledgeCategoryInfo.
  ///
  /// In en, this message translates to:
  /// **'Information'**
  String get knowledgeCategoryInfo;

  /// No description provided for @statusOk.
  ///
  /// In en, this message translates to:
  /// **'OK'**
  String get statusOk;

  /// No description provided for @statusFailed.
  ///
  /// In en, this message translates to:
  /// **'Failed'**
  String get statusFailed;

  /// No description provided for @missingSaveReference.
  ///
  /// In en, this message translates to:
  /// **'File missing'**
  String get missingSaveReference;

  /// No description provided for @missingSaveReferenceDescription.
  ///
  /// In en, this message translates to:
  /// **'{slot}.sav is missing. It may have been deleted, moved, or renamed; the profile still references it.'**
  String missingSaveReferenceDescription(String slot);

  /// No description provided for @removeFromProfile.
  ///
  /// In en, this message translates to:
  /// **'Remove from profile'**
  String get removeFromProfile;

  /// No description provided for @deleteSavegame.
  ///
  /// In en, this message translates to:
  /// **'Delete save'**
  String get deleteSavegame;

  /// No description provided for @deleteSavegameTitle.
  ///
  /// In en, this message translates to:
  /// **'Delete savegame?'**
  String get deleteSavegameTitle;

  /// No description provided for @deleteSavegameBody.
  ///
  /// In en, this message translates to:
  /// **'Delete {save} ({fileName})? It will be removed from {profile} and deleted from the save folder. GORE creates a backup first.'**
  String deleteSavegameBody(String save, String fileName, String profile);

  /// No description provided for @removeSaveFromProfileTitle.
  ///
  /// In en, this message translates to:
  /// **'Remove save from profile?'**
  String get removeSaveFromProfileTitle;

  /// No description provided for @removeSaveFromProfileBody.
  ///
  /// In en, this message translates to:
  /// **'Remove {save} from {profile}? The save file itself will be kept if it still exists.'**
  String removeSaveFromProfileBody(String save, String profile);

  /// No description provided for @unassignedSave.
  ///
  /// In en, this message translates to:
  /// **'Not assigned to a profile'**
  String get unassignedSave;

  /// No description provided for @armorUpgradeLight.
  ///
  /// In en, this message translates to:
  /// **'Light'**
  String get armorUpgradeLight;

  /// No description provided for @armorUpgradeMedium.
  ///
  /// In en, this message translates to:
  /// **'Medium'**
  String get armorUpgradeMedium;

  /// No description provided for @armorUpgradeHeavy.
  ///
  /// In en, this message translates to:
  /// **'Heavy'**
  String get armorUpgradeHeavy;

  /// No description provided for @knowledgeCaptionForcedConversation.
  ///
  /// In en, this message translates to:
  /// **'Forced conversation'**
  String get knowledgeCaptionForcedConversation;

  /// No description provided for @knowledgeCaptionFollowupTopic.
  ///
  /// In en, this message translates to:
  /// **'Follow-up topic'**
  String get knowledgeCaptionFollowupTopic;

  /// No description provided for @knowledgeCaptionFallbackTopic.
  ///
  /// In en, this message translates to:
  /// **'Fallback topic'**
  String get knowledgeCaptionFallbackTopic;

  /// No description provided for @durationMinutes.
  ///
  /// In en, this message translates to:
  /// **'{minutes} min'**
  String durationMinutes(int minutes);

  /// No description provided for @durationHours.
  ///
  /// In en, this message translates to:
  /// **'{hours} hr'**
  String durationHours(int hours);

  /// No description provided for @durationHoursMinutes.
  ///
  /// In en, this message translates to:
  /// **'{hours} hr {minutes} min'**
  String durationHoursMinutes(int hours, int minutes);

  /// No description provided for @backupStatusInvalidProfileStructure.
  ///
  /// In en, this message translates to:
  /// **'Invalid profile data'**
  String get backupStatusInvalidProfileStructure;

  /// No description provided for @backupStatusSlotMetadataMissing.
  ///
  /// In en, this message translates to:
  /// **'Selected save metadata is missing'**
  String get backupStatusSlotMetadataMissing;

  /// No description provided for @defaultProfileName.
  ///
  /// In en, this message translates to:
  /// **'Profile {id}'**
  String defaultProfileName(int id);

  /// No description provided for @statusUnknown.
  ///
  /// In en, this message translates to:
  /// **'Unknown'**
  String get statusUnknown;

  /// No description provided for @editorUnexpectedError.
  ///
  /// In en, this message translates to:
  /// **'Unexpected error: {details}'**
  String editorUnexpectedError(String details);

  /// No description provided for @editorOperationInProgress.
  ///
  /// In en, this message translates to:
  /// **'Another operation is in progress. Try again in a moment.'**
  String get editorOperationInProgress;

  /// No description provided for @editorUnsavedBeforeDifficulty.
  ///
  /// In en, this message translates to:
  /// **'You have unsaved save edits. Save or reset them before changing the profile difficulty.'**
  String get editorUnsavedBeforeDifficulty;

  /// No description provided for @editorNoSaveFolderSelected.
  ///
  /// In en, this message translates to:
  /// **'No save folder selected.'**
  String get editorNoSaveFolderSelected;

  /// No description provided for @editorNoSaveSelected.
  ///
  /// In en, this message translates to:
  /// **'No save selected.'**
  String get editorNoSaveSelected;

  /// No description provided for @coreUnknownError.
  ///
  /// In en, this message translates to:
  /// **'Unknown core error'**
  String get coreUnknownError;

  /// No description provided for @editorUnsavedBeforeSwitchProfile.
  ///
  /// In en, this message translates to:
  /// **'Save or reset your unsaved changes first — switching profiles would move away from the current save.'**
  String get editorUnsavedBeforeSwitchProfile;

  /// No description provided for @editorUnsavedBeforeOpenFile.
  ///
  /// In en, this message translates to:
  /// **'Save or reset your unsaved changes before opening another file.'**
  String get editorUnsavedBeforeOpenFile;

  /// No description provided for @editorSelectSavFile.
  ///
  /// In en, this message translates to:
  /// **'Select a .sav savegame file.'**
  String get editorSelectSavFile;

  /// No description provided for @editorNotGothicGsav.
  ///
  /// In en, this message translates to:
  /// **'The selected file is not a Gothic GSAV savegame.'**
  String get editorNotGothicGsav;

  /// No description provided for @editorUnsavedBeforeChangeSaveProfile.
  ///
  /// In en, this message translates to:
  /// **'Save or reset your unsaved changes before changing the save profile.'**
  String get editorUnsavedBeforeChangeSaveProfile;

  /// No description provided for @editorUnsavedBeforeRemoveProfile.
  ///
  /// In en, this message translates to:
  /// **'Save or reset your unsaved changes before removing a save from its profile.'**
  String get editorUnsavedBeforeRemoveProfile;

  /// No description provided for @editorUnsavedBeforeDeleteSave.
  ///
  /// In en, this message translates to:
  /// **'Save or reset your unsaved changes before deleting this save.'**
  String get editorUnsavedBeforeDeleteSave;

  /// No description provided for @editorUnsavedBeforeRestoreProfile.
  ///
  /// In en, this message translates to:
  /// **'You have unsaved save edits. Save or reset them before restoring a profile backup.'**
  String get editorUnsavedBeforeRestoreProfile;

  /// No description provided for @editorConflictingPropertyEdits.
  ///
  /// In en, this message translates to:
  /// **'Conflicting unsaved edits target the same property ({path}) from two tabs. Reset or revert one of them, then save again.'**
  String editorConflictingPropertyEdits(String path);

  /// No description provided for @editorGlossaryMemoryConflict.
  ///
  /// In en, this message translates to:
  /// **'A glossary segment change and another unsaved All-data edit both target the Hero MemorizedEvents array ({path}). Glossary changes add or remove entries in that array, so the edits cannot be saved together — reset or revert one of them, then save again.'**
  String editorGlossaryMemoryConflict(String path);

  /// No description provided for @editorGlossaryQuestConflict.
  ///
  /// In en, this message translates to:
  /// **'A glossary segment change and another unsaved edit target the same quest CurrentState property ({path}). The glossary change updates that state itself — reset or revert one of them, then save again.'**
  String editorGlossaryQuestConflict(String path);

  /// No description provided for @editorRelationshipConflict.
  ///
  /// In en, this message translates to:
  /// **'A relationship override and another unsaved All-data edit both target the same NPC relationship entry ({path}). The structured relationship change can replace modifiers in that entry, so the edits cannot be saved together — reset or revert one of them, then save again.'**
  String editorRelationshipConflict(String path);

  /// No description provided for @editorMultipleStructuralArrayEdits.
  ///
  /// In en, this message translates to:
  /// **'More than one unsaved structural edit targets the same array ({path}). Save or reset the first change before queuing another.'**
  String editorMultipleStructuralArrayEdits(String path);

  /// No description provided for @editorStructuralArrayConflict.
  ///
  /// In en, this message translates to:
  /// **'A structural event change and another unsaved All-data edit both target {path}. Save or reset one of them before continuing.'**
  String editorStructuralArrayConflict(String path);

  /// No description provided for @editorSkillsEffectConflict.
  ///
  /// In en, this message translates to:
  /// **'A Skills change and an All-data edit to the same actor’s effect (ActiveEffects › EffectSpec › Def) are both queued. They cannot be saved together — reset or revert one of them, then save again.'**
  String get editorSkillsEffectConflict;

  /// No description provided for @editorInventoryResetConflict.
  ///
  /// In en, this message translates to:
  /// **'An inventory reset and another edit to the same inventory are both queued. The reset replaces the entire inventory and would discard the other edit — reset or revert one of them, then save again.'**
  String get editorInventoryResetConflict;

  /// No description provided for @editorUseFolder.
  ///
  /// In en, this message translates to:
  /// **'Use folder'**
  String get editorUseFolder;

  /// No description provided for @editorGothicSavegameFileType.
  ///
  /// In en, this message translates to:
  /// **'Gothic savegame'**
  String get editorGothicSavegameFileType;

  /// No description provided for @editorNoDifficultyChanges.
  ///
  /// In en, this message translates to:
  /// **'No difficulty changes to write'**
  String get editorNoDifficultyChanges;

  /// No description provided for @editorDifficultyWritten.
  ///
  /// In en, this message translates to:
  /// **'Difficulty written to the profile (backup created)'**
  String get editorDifficultyWritten;

  /// No description provided for @editorChangesSavedWithBackup.
  ///
  /// In en, this message translates to:
  /// **'{count, plural, =1{1 change saved with backup} other{{count} changes saved with backup}}'**
  String editorChangesSavedWithBackup(int count);

  /// No description provided for @editorPlacementNoteFailed.
  ///
  /// In en, this message translates to:
  /// **'The move was saved, but its undo note could not be written: {details}'**
  String editorPlacementNoteFailed(String details);

  /// No description provided for @editorProfileNotFound.
  ///
  /// In en, this message translates to:
  /// **'Profile {profileId} was not found.'**
  String editorProfileNotFound(int profileId);

  /// No description provided for @editorNoFreeSaveSlot.
  ///
  /// In en, this message translates to:
  /// **'No free save slot is available in the game save folder (G1R-001 through G1R-999).'**
  String get editorNoFreeSaveSlot;

  /// No description provided for @editorSaveImportedAssigned.
  ///
  /// In en, this message translates to:
  /// **'Save imported and assigned to profile {profileId}'**
  String editorSaveImportedAssigned(int profileId);

  /// No description provided for @editorSaveAssigned.
  ///
  /// In en, this message translates to:
  /// **'Save assigned to profile {profileId} (paired backups created)'**
  String editorSaveAssigned(int profileId);

  /// No description provided for @editorSaveSlotNotAssigned.
  ///
  /// In en, this message translates to:
  /// **'Save slot {slot} is not assigned to profile {profileId}.'**
  String editorSaveSlotNotAssigned(String slot, int profileId);

  /// No description provided for @editorSaveRemovedFromProfile.
  ///
  /// In en, this message translates to:
  /// **'Save removed from profile'**
  String get editorSaveRemovedFromProfile;

  /// No description provided for @editorSaveDeleted.
  ///
  /// In en, this message translates to:
  /// **'Save deleted; backup created'**
  String get editorSaveDeleted;

  /// No description provided for @editorRestoredBackup.
  ///
  /// In en, this message translates to:
  /// **'Restored backup: {path}'**
  String editorRestoredBackup(String path);

  /// No description provided for @editorRestoredBackupWithoutCompanion.
  ///
  /// In en, this message translates to:
  /// **'Restored backup: {path} (PersistentDataList.sav left unchanged — no matching companion backup; slot metadata may differ)'**
  String editorRestoredBackupWithoutCompanion(String path);

  /// No description provided for @editorCodecRoundtripPassed.
  ///
  /// In en, this message translates to:
  /// **'Codec roundtrip passed: chunk {chunkIndex} recompressed to {bytes} bytes'**
  String editorCodecRoundtripPassed(int chunkIndex, int bytes);

  /// No description provided for @editorDifficultyWriteFailed.
  ///
  /// In en, this message translates to:
  /// **'Could not write the profile difficulty: {details}'**
  String editorDifficultyWriteFailed(String details);

  /// No description provided for @editorProfileAssignmentFailed.
  ///
  /// In en, this message translates to:
  /// **'Could not assign the save to the profile: {details}'**
  String editorProfileAssignmentFailed(String details);

  /// No description provided for @editorProfileRemovalFailed.
  ///
  /// In en, this message translates to:
  /// **'Could not remove the save from the profile: {details}'**
  String editorProfileRemovalFailed(String details);

  /// No description provided for @editorDeleteSaveFailed.
  ///
  /// In en, this message translates to:
  /// **'Could not delete the save: {details}'**
  String editorDeleteSaveFailed(String details);

  /// No description provided for @editorSaveFailed.
  ///
  /// In en, this message translates to:
  /// **'Could not save the changes: {details}'**
  String editorSaveFailed(String details);

  /// No description provided for @editorScanSavesFailed.
  ///
  /// In en, this message translates to:
  /// **'Failed to scan saves: {details}'**
  String editorScanSavesFailed(String details);

  /// No description provided for @editorInspectSaveFailed.
  ///
  /// In en, this message translates to:
  /// **'Failed to inspect save: {details}'**
  String editorInspectSaveFailed(String details);

  /// No description provided for @editorLoadBackupsFailed.
  ///
  /// In en, this message translates to:
  /// **'Failed to load backups: {details}'**
  String editorLoadBackupsFailed(String details);

  /// No description provided for @editorRestoreFailed.
  ///
  /// In en, this message translates to:
  /// **'Could not restore the backup: {details}'**
  String editorRestoreFailed(String details);

  /// No description provided for @editorRestoreReloadFailed.
  ///
  /// In en, this message translates to:
  /// **'Restored backup: {path}, but reloading the save failed: {details}'**
  String editorRestoreReloadFailed(String path, String details);

  /// No description provided for @editorCodecCheckFailed.
  ///
  /// In en, this message translates to:
  /// **'Codec check failed: {details}'**
  String editorCodecCheckFailed(String details);

  /// No description provided for @editorCodecValidationFailed.
  ///
  /// In en, this message translates to:
  /// **'Codec roundtrip failed: {details}'**
  String editorCodecValidationFailed(String details);

  /// No description provided for @editorPropertySearchFailed.
  ///
  /// In en, this message translates to:
  /// **'Property search failed: {details}'**
  String editorPropertySearchFailed(String details);

  /// No description provided for @editorSelectionChangedWhileLoadingHeroAttributes.
  ///
  /// In en, this message translates to:
  /// **'Save selection changed while loading hero attributes.'**
  String get editorSelectionChangedWhileLoadingHeroAttributes;

  /// No description provided for @editorSkillsLoadFailed.
  ///
  /// In en, this message translates to:
  /// **'Skills load failed: {details}'**
  String editorSkillsLoadFailed(String details);

  /// No description provided for @editorProgressionQueryFailed.
  ///
  /// In en, this message translates to:
  /// **'Progression query failed: {details}'**
  String editorProgressionQueryFailed(String details);

  /// No description provided for @editorNpcListFailed.
  ///
  /// In en, this message translates to:
  /// **'NPC list failed: {details}'**
  String editorNpcListFailed(String details);

  /// No description provided for @editorCharacterListFailed.
  ///
  /// In en, this message translates to:
  /// **'Character list failed: {details}'**
  String editorCharacterListFailed(String details);

  /// No description provided for @editorNpcAttributesFailed.
  ///
  /// In en, this message translates to:
  /// **'NPC attributes failed: {details}'**
  String editorNpcAttributesFailed(String details);

  /// No description provided for @editorNpcPositionFailed.
  ///
  /// In en, this message translates to:
  /// **'Loading the NPC position failed: {details}'**
  String editorNpcPositionFailed(String details);

  /// No description provided for @editorNpcInventoryFailed.
  ///
  /// In en, this message translates to:
  /// **'NPC inventory failed: {details}'**
  String editorNpcInventoryFailed(String details);

  /// No description provided for @editorFactionListFailed.
  ///
  /// In en, this message translates to:
  /// **'Faction list failed: {details}'**
  String editorFactionListFailed(String details);

  /// No description provided for @editorNoBackupPath.
  ///
  /// In en, this message translates to:
  /// **'none'**
  String get editorNoBackupPath;

  /// No description provided for @editorBackupMessage.
  ///
  /// In en, this message translates to:
  /// **'{prefix}: {backupPath}'**
  String editorBackupMessage(String prefix, String backupPath);

  /// No description provided for @editorBackupMessageWithPersistent.
  ///
  /// In en, this message translates to:
  /// **'{prefix}: {backupPath}; PersistentDataList backup: {persistentPath}'**
  String editorBackupMessageWithPersistent(
    String prefix,
    String backupPath,
    String persistentPath,
  );

  /// No description provided for @localizationStatusFailed.
  ///
  /// In en, this message translates to:
  /// **'Localization status failed: {details}'**
  String localizationStatusFailed(String details);

  /// No description provided for @localizationExtractionFailed.
  ///
  /// In en, this message translates to:
  /// **'Extraction failed: {details}'**
  String localizationExtractionFailed(String details);

  /// No description provided for @glossaryLoadFailed.
  ///
  /// In en, this message translates to:
  /// **'Glossary load failed: {details}'**
  String glossaryLoadFailed(String details);

  /// No description provided for @backupStatusError.
  ///
  /// In en, this message translates to:
  /// **'Backup error: {details}'**
  String backupStatusError(String details);

  /// No description provided for @memoryEventCategory.
  ///
  /// In en, this message translates to:
  /// **'{category, select, quest{Quest} document{Document} story{Story} exploration{Exploration} combat{Combat} social{Social} item{Items} learning{Learning} guild{Guild} crime{Crime} rest{Rest} other{{fallback}}}'**
  String memoryEventCategory(String category, String fallback);

  /// No description provided for @memoryEventAction.
  ///
  /// In en, this message translates to:
  /// **'{kind, select, questStarted{Quest started} questSucceeded{Quest completed} questFailed{Quest failed} documentRead{Document read} documentSegmentUnlocked{Entry discovered} documentSegmentViewed{Entry viewed} chapterCompleted{Chapter completed} areaEntered{Area entered} areaLeft{Area left} characterKilled{Character killed} characterDefeated{Character defeated} combatDodge{Attack dodged} characterDebuffed{Debuff applied} tradeAvailable{Trading unlocked} itemObtained{Item obtained} itemCrafted{Item crafted} skillStateRecorded{Skill state recorded} recipeLearned{Recipe learned} guildJoined{Guild joined} crimeRecorded{Crime recorded} slept{Slept} storyEvent{Story event} other{{fallback}}}'**
  String memoryEventAction(String kind, String fallback);

  /// No description provided for @memoryEventTitleWithSubject.
  ///
  /// In en, this message translates to:
  /// **'{action}: {subject}'**
  String memoryEventTitleWithSubject(String action, String subject);

  /// No description provided for @memoryEventFact.
  ///
  /// In en, this message translates to:
  /// **'{fact, select, gameTime{Game time} duration{Duration} chapter{Chapter} instigator{Initiated by} affected{Affected} amount{Amount} primaryObject{Object} secondaryObject{Context} segmentText{Entry text} other{{fallback}}}'**
  String memoryEventFact(String fact, String fallback);

  /// No description provided for @memoryEventGameTime.
  ///
  /// In en, this message translates to:
  /// **'Day {day}, {time}'**
  String memoryEventGameTime(int day, String time);

  /// No description provided for @memoryEventSecondsValue.
  ///
  /// In en, this message translates to:
  /// **'{value} s'**
  String memoryEventSecondsValue(String value);

  /// No description provided for @memoryEventMoreValues.
  ///
  /// In en, this message translates to:
  /// **'{values} +{count}'**
  String memoryEventMoreValues(String values, int count);

  /// No description provided for @memoryEventHero.
  ///
  /// In en, this message translates to:
  /// **'Hero'**
  String get memoryEventHero;

  /// No description provided for @memoryEventDetails.
  ///
  /// In en, this message translates to:
  /// **'Details'**
  String get memoryEventDetails;

  /// No description provided for @memoryEventTags.
  ///
  /// In en, this message translates to:
  /// **'Tags'**
  String get memoryEventTags;

  /// No description provided for @memoryEventTechnicalData.
  ///
  /// In en, this message translates to:
  /// **'Technical data'**
  String get memoryEventTechnicalData;

  /// No description provided for @memoryEventIndex.
  ///
  /// In en, this message translates to:
  /// **'Index'**
  String get memoryEventIndex;

  /// No description provided for @memoryEventPosition.
  ///
  /// In en, this message translates to:
  /// **'Position'**
  String get memoryEventPosition;

  /// No description provided for @memoryEventPayload.
  ///
  /// In en, this message translates to:
  /// **'Payload'**
  String get memoryEventPayload;

  /// No description provided for @memoryEventSubject.
  ///
  /// In en, this message translates to:
  /// **'Subject'**
  String get memoryEventSubject;

  /// No description provided for @glossaryCatalogSegmentLabel.
  ///
  /// In en, this message translates to:
  /// **'{segmentId, select, Access{Access} AccessDenied{Access Denied} AccesToTemple{Access to Temple} Advice{Advice} AfterFight{After Fight} AfterFireMages{After Fire Mages} AfterNek{After Nek} AfterQuest{After Quest} Alone{Alone} Amulet{Amulet} Annoying{Annoying} Armor{Armor} Avoid{Avoid} Backstory{Backstory} BackStory{Back Story} BasicMagic{Basic Magic} Beated{Beaten} BecomeMercenary{Become Mercenary} Beer{Beer} Bestiary{Bestiary} Blessing{Blessing} Boss{Boss} Bully{Bully} BullyAdvice{Bully Advice} Camp{Camp} CampDivided{Camp Divided} CareOfMessengers{Care of Messengers} ChangeOpinion{Change Opinion} ChargeUriziel{Charge Uriziel} Chosen{Chosen} Contact{Contact} Courier{Courier} CraftBows{Craft Bows} Crazy{Crazy} DailyMeal{Daily Meal} DailyRation_Trader{Daily Ration Trader} DAM{Dam} Dead{Dead} Deal{Deal} Dealer{Dealer} Deceived{Deceived} Dementia{Dementia} DenyAccess{Deny Access} DifferentOpinion{Different Opinion} Discussion{Discussion} DontTalk{Don’t Talk} Duel{Duel} Entrance{Entrance} Escape{Escape} Extended{Extended} Extra{Extra} ExtraInfo{Extra Info} Fanatic{Fanatic} Fight{Fight} FindUlumulu{Find Ulu-Mulu} FireMages{Fire Mages} FireMagesEscape{Fire Mages Escape} FiskNewDealer{New Fence for Fisk} FiskNewDealerCompleted{New Fence for Fisk — Completed} FogTower{Fog Tower} Food{Food} Forgave{Forgave} Forgive{Forgive} Forgiven{Forgiven} FourFriends{Four Friends} FreeHut{Free Hut} FreeMine{Free Mine} Fury{Fury} GoodTeacher{Good Teacher} Gossip{Gossip} GotScavenger{Got Scavenger} GrantedAccess{Granted Access} GRDArmor{Guard Armor} Guide{Guide} HateMages{Hate Mages} HateMagesExplanation{Hate Mages Explanation} HateRiceLord{Hate Rice Lord} Heal{Heal} Healing{Healing} Help{Help} Helper{Helper} HelpKagan{Help Kagan} HutStory{Hut Story} Ignore{Ignore} Impress{Impress} ImpressAlchemy{Impress Alchemy} ImpressInscription{Impress Inscription} Info{Info} Interested{Interested} Introduction{Introduction} Introduction_2{Introduction 2} Introduction_Armor{Introduction – Armor} Introduction_Teacher{Introduction – Teacher} Introduction_Trader{Introduction – Trader} Invocation{Invocation} JoinSC{Join Swamp Camp} Joint{Joint} KalomCamp{Kalom Camp} Leader{Leader} Learning{Learning} LearnOrcish{Learn Orcish} LeftParty{Left Party} Library{Library} Lie{Lie} Lock{Lock} Lockpick{Lockpick} Mad{Mad} Mandibles{Minecrawler Mandibles} MapMaker{Map Maker} Monastery{Monastery} MordragKO{Mordrag KO} Nek{Nek} NewCamp{New Camp} NewCamper{New Camper} NewLeader{New Leader} NightPatrol{Night Patrol} NotInterested{Not Interested} OldCamp{Old Camp} OrcEnclaveEntrance{Orc Enclave Entrance} OrcGraveyard{Orc Graveyard} OreArmor{Ore Armor} Party{Party} Pay{Pay} PayMoney{Pay Money} Permission{Permission} Pet{Pet} PreparingInvocation{Preparing Invocation} Quest{Quest} RankUpFireMages{Fire Mage Promotion} RankUpGuard{Guard Promotion} RanUpFireMagesCompleted{Fire Mage Promotion Completed} Realocated{Relocated} Reason{Reason} Respect{Respect} ReturnToSC{Return to Swamp Camp} RicelordForeman{Rice Lord’s Foreman} RideScavenger{Ride Scavenger} Robe{Robe} Safe{Safe} Scraper{Scraper} SecondChance{Second Chance} SecretLocation{Secret Location} SecretPassage{Secret Passage} SecretPath{Secret Path} SleeperFollower{Sleeper Follower} SleeperTemple{Sleeper Temple} SmallInfo{Small Info} Stonehenge{Stonehenge} StopFollowing{Stop Following} SwampCamp{Swamp Camp} Talkative{Talkative} Teach{Teach} TeachBow{Teach Bow} Teacher{Teacher} Teacher2{Teacher 2} TeacherInscription{Teacher Inscription} TeacherMana{Teacher Mana} TeachIchor{Teach Minecrawler Ichor Extraction} TeachMagic{Teach Magic} TeachOrcish{Teach Orcish} TeachStats{Teach Stats} TeachWeapon{Teach Weapon} Teleport{Teleport} TheMysteriousOrc{The Mysterious Orc} ThroneRoom{Throne Room} TradeBow{Trade Bow} Trader{Trader} TradeSkins_Trader{Skin Trader} Traitor{Traitor} Trial{Trial} TrollCanyon{Troll Canyon} Trust{Trust} Ulumulu{Ulu-Mulu} Unexperienced{Inexperienced} Uriziel{Uriziel} UrizielRune{Uriziel Rune} Useful{Useful} Velaya{Velaya} Vibrations{Vibrations} WaitFreeMine{Wait at Free Mine} WaitInTrainingArea{Wait In Training Area} Warning{Warning} WarningTooLate{Warning Came Too Late} WaterMessenger{Messenger for the Water Mages} Weapon{Weapon} Who{Who} Women{Women} other{{fallback}}}'**
  String glossaryCatalogSegmentLabel(String segmentId, String fallback);

  /// No description provided for @slotRepairTitle.
  ///
  /// In en, this message translates to:
  /// **'Damaged inventory slots'**
  String get slotRepairTitle;

  /// No description provided for @slotRepairBody.
  ///
  /// In en, this message translates to:
  /// **'This savegame holds {count} inventory slots whose id no longer matches their position — in the game, dropping such an item removes a different one instead. The repair only rewrites the ids: no item is added, removed or changed. A backup is created when you save, as always.'**
  String slotRepairBody(int count);

  /// No description provided for @slotRepairQueued.
  ///
  /// In en, this message translates to:
  /// **'Repair queued — save to apply it.'**
  String get slotRepairQueued;

  /// No description provided for @slotRepairAction.
  ///
  /// In en, this message translates to:
  /// **'Repair'**
  String get slotRepairAction;

  /// No description provided for @slotRepairDiscard.
  ///
  /// In en, this message translates to:
  /// **'Discard'**
  String get slotRepairDiscard;

  /// No description provided for @editorInventorySlotEditConflict.
  ///
  /// In en, this message translates to:
  /// **'A direct edit of an inventory slot is queued together with a change that claims whole slots (repair, add or remove). The second would overwrite the first — revert one of them, then save again.'**
  String get editorInventorySlotEditConflict;

  /// No description provided for @editorTraderArrayConflict.
  ///
  /// In en, this message translates to:
  /// **'A trade change is queued together with a direct edit of the trader array. That edit renumbers the rows a trade change is addressed by, so one of the two would land on the wrong merchant — revert one of them, then save again.'**
  String get editorTraderArrayConflict;

  /// No description provided for @backupFactFile.
  ///
  /// In en, this message translates to:
  /// **'File'**
  String get backupFactFile;

  /// No description provided for @renameBackupTooltip.
  ///
  /// In en, this message translates to:
  /// **'Name this backup'**
  String get renameBackupTooltip;

  /// No description provided for @renameBackupTitle.
  ///
  /// In en, this message translates to:
  /// **'Name backup'**
  String get renameBackupTitle;

  /// No description provided for @renameBackupLabel.
  ///
  /// In en, this message translates to:
  /// **'Name'**
  String get renameBackupLabel;

  /// No description provided for @renameBackupHelp.
  ///
  /// In en, this message translates to:
  /// **'Shown instead of the file name {fileName}. Leave empty to remove the name; the file itself is not renamed.'**
  String renameBackupHelp(String fileName);

  /// No description provided for @deleteBackupTooltip.
  ///
  /// In en, this message translates to:
  /// **'Delete this backup'**
  String get deleteBackupTooltip;

  /// No description provided for @deleteBackupTitle.
  ///
  /// In en, this message translates to:
  /// **'Delete backup'**
  String get deleteBackupTitle;

  /// No description provided for @deleteBackupBody.
  ///
  /// In en, this message translates to:
  /// **'Delete “{name}” ({fileName})? The file is removed from disk and cannot be brought back.'**
  String deleteBackupBody(String name, String fileName);

  /// No description provided for @deleteBackupConfirm.
  ///
  /// In en, this message translates to:
  /// **'Delete'**
  String get deleteBackupConfirm;

  /// No description provided for @editorDeletedBackup.
  ///
  /// In en, this message translates to:
  /// **'Backup deleted: {path}'**
  String editorDeletedBackup(String path);

  /// No description provided for @editorDeleteBackupFailed.
  ///
  /// In en, this message translates to:
  /// **'Could not delete the backup: {details}'**
  String editorDeleteBackupFailed(String details);

  /// No description provided for @editorRenameBackupFailed.
  ///
  /// In en, this message translates to:
  /// **'Could not name the backup: {details}'**
  String editorRenameBackupFailed(String details);

  /// No description provided for @slotRepairUnavailable.
  ///
  /// In en, this message translates to:
  /// **'Repairing is not possible right now — this savegame cannot be written.'**
  String get slotRepairUnavailable;

  /// No description provided for @editorDeletedBackupWithLabelWarning.
  ///
  /// In en, this message translates to:
  /// **'Backup deleted: {path} — its name could not be removed: {details}'**
  String editorDeletedBackupWithLabelWarning(String path, String details);

  /// No description provided for @slotRepairNotOffered.
  ///
  /// In en, this message translates to:
  /// **'The repair is not available for this savegame.'**
  String get slotRepairNotOffered;

  /// No description provided for @statisticsTitle.
  ///
  /// In en, this message translates to:
  /// **'Statistics'**
  String get statisticsTitle;

  /// No description provided for @statisticsSubtitle.
  ///
  /// In en, this message translates to:
  /// **'A compact summary of character, quest, world, and save progress.'**
  String get statisticsSubtitle;

  /// No description provided for @statisticsCardTitle.
  ///
  /// In en, this message translates to:
  /// **'{card, select, timing{Time} character{Character} quests{Quests} progress{Progress} encounters{Combat & contacts} inventory{Skills & inventory} other{{fallback}}}'**
  String statisticsCardTitle(String card, String fallback);

  /// No description provided for @statisticsMetric.
  ///
  /// In en, this message translates to:
  /// **'{metric, select, timePlayed{Played} worldTime{World time} level{Level} experience{Experience} learningPoints{Learning points} guild{Guild} health{Health} mana{Mana} chapter{Chapter} location{Location} kills{NPC kills} knownCharacters{Known characters} killedMonsters{Killed monsters} defeatedNpcs{Defeated NPCs} killedNpcs{Killed NPCs} knownNpcs{Known NPCs} knownTeachers{Known teachers} learnedSkills{Learned skills} knowledge{Knowledge entries} deadCharacters{Dead characters} traders{Known traders} inventoryStacks{Item stacks} inventoryItems{Items} ore{Ore} equipped{Equipped} hostileFactions{Hostile factions} openCrimes{Open crimes} position{Position} other{{fallback}}}'**
  String statisticsMetric(String metric, String fallback);

  /// No description provided for @statisticsGuildRank.
  ///
  /// In en, this message translates to:
  /// **'{rank, select, oldCampShadow{Old Camp · Shadow} oldCampGuard{Old Camp · Guard} oldCampFireMage{Old Camp · Fire Mage} newCampRogue{New Camp · Bandit} newCampMercenary{New Camp · Mercenary} newCampWaterMage{New Camp · Water Mage} swampCampNovice{Swamp Camp · Novice} swampCampTemplar{Swamp Camp · Templar} other{{fallback}}}'**
  String statisticsGuildRank(String rank, String fallback);

  /// No description provided for @statisticsUnknown.
  ///
  /// In en, this message translates to:
  /// **'Not available'**
  String get statisticsUnknown;

  /// No description provided for @statisticsMore.
  ///
  /// In en, this message translates to:
  /// **'More statistics'**
  String get statisticsMore;

  /// No description provided for @statisticsSummary.
  ///
  /// In en, this message translates to:
  /// **'Level {level}, {guild}, chapter {chapter}. {completed} quests completed, {failed} failed. Play time: {playTime}.'**
  String statisticsSummary(
    String level,
    String guild,
    String chapter,
    int completed,
    int failed,
    String playTime,
  );
}

class _AppLocalizationsDelegate
    extends LocalizationsDelegate<AppLocalizations> {
  const _AppLocalizationsDelegate();

  @override
  Future<AppLocalizations> load(Locale locale) {
    return SynchronousFuture<AppLocalizations>(lookupAppLocalizations(locale));
  }

  @override
  bool isSupported(Locale locale) => <String>[
    'de',
    'en',
    'es',
    'fr',
    'it',
    'ja',
    'pl',
    'pt',
    'ru',
    'zh',
  ].contains(locale.languageCode);

  @override
  bool shouldReload(_AppLocalizationsDelegate old) => false;
}

AppLocalizations lookupAppLocalizations(Locale locale) {
  // Lookup logic when language+script codes are specified.
  switch (locale.languageCode) {
    case 'zh':
      {
        switch (locale.scriptCode) {
          case 'Hans':
            return AppLocalizationsZhHans();
        }
        break;
      }
  }

  // Lookup logic when language+country codes are specified.
  switch (locale.languageCode) {
    case 'pt':
      {
        switch (locale.countryCode) {
          case 'BR':
            return AppLocalizationsPtBr();
        }
        break;
      }
  }

  // Lookup logic when only language code is specified.
  switch (locale.languageCode) {
    case 'de':
      return AppLocalizationsDe();
    case 'en':
      return AppLocalizationsEn();
    case 'es':
      return AppLocalizationsEs();
    case 'fr':
      return AppLocalizationsFr();
    case 'it':
      return AppLocalizationsIt();
    case 'ja':
      return AppLocalizationsJa();
    case 'pl':
      return AppLocalizationsPl();
    case 'pt':
      return AppLocalizationsPt();
    case 'ru':
      return AppLocalizationsRu();
    case 'zh':
      return AppLocalizationsZh();
  }

  throw FlutterError(
    'AppLocalizations.delegate failed to load unsupported locale "$locale". This is likely '
    'an issue with the localizations generation tool. Please file an issue '
    'on GitHub with a reproducible sample app and the gen-l10n configuration '
    'that was used.',
  );
}
