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

  /// No description provided for @appTitle.
  ///
  /// In en, this message translates to:
  /// **'Gothic Remake Savegame Editor'**
  String get appTitle;

  /// No description provided for @appLogoSemanticLabel.
  ///
  /// In en, this message translates to:
  /// **'goresave logo'**
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

  /// No description provided for @tabInventory.
  ///
  /// In en, this message translates to:
  /// **'Inventory'**
  String get tabInventory;

  /// No description provided for @tabProgression.
  ///
  /// In en, this message translates to:
  /// **'Progression'**
  String get tabProgression;

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

  /// No description provided for @diagnosticsTitle.
  ///
  /// In en, this message translates to:
  /// **'Diagnostics & details'**
  String get diagnosticsTitle;

  /// No description provided for @diagnosticsSubtitle.
  ///
  /// In en, this message translates to:
  /// **'Read-only format inspection'**
  String get diagnosticsSubtitle;

  /// No description provided for @metricFormat.
  ///
  /// In en, this message translates to:
  /// **'Format'**
  String get metricFormat;

  /// No description provided for @metricSlot.
  ///
  /// In en, this message translates to:
  /// **'Slot'**
  String get metricSlot;

  /// No description provided for @metricChapter.
  ///
  /// In en, this message translates to:
  /// **'Chapter'**
  String get metricChapter;

  /// No description provided for @metricTimePlayed.
  ///
  /// In en, this message translates to:
  /// **'Time played'**
  String get metricTimePlayed;

  /// No description provided for @metricSaveKind.
  ///
  /// In en, this message translates to:
  /// **'Save kind'**
  String get metricSaveKind;

  /// No description provided for @metricFileSize.
  ///
  /// In en, this message translates to:
  /// **'File size'**
  String get metricFileSize;

  /// No description provided for @metricCompression.
  ///
  /// In en, this message translates to:
  /// **'Compression'**
  String get metricCompression;

  /// No description provided for @metricChunks.
  ///
  /// In en, this message translates to:
  /// **'Chunks'**
  String get metricChunks;

  /// No description provided for @metricUncompressed.
  ///
  /// In en, this message translates to:
  /// **'Uncompressed'**
  String get metricUncompressed;

  /// No description provided for @metricPrivate.
  ///
  /// In en, this message translates to:
  /// **'Private'**
  String get metricPrivate;

  /// No description provided for @metricSlotName.
  ///
  /// In en, this message translates to:
  /// **'Slot name'**
  String get metricSlotName;

  /// No description provided for @metricTrailer.
  ///
  /// In en, this message translates to:
  /// **'Trailer'**
  String get metricTrailer;

  /// No description provided for @metricDecodedPrivate.
  ///
  /// In en, this message translates to:
  /// **'Decoded private'**
  String get metricDecodedPrivate;

  /// No description provided for @metricPrivateStrings.
  ///
  /// In en, this message translates to:
  /// **'Private strings'**
  String get metricPrivateStrings;

  /// No description provided for @metricSha1.
  ///
  /// In en, this message translates to:
  /// **'SHA-1'**
  String get metricSha1;

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

  /// No description provided for @inspectionJsonSubtitle.
  ///
  /// In en, this message translates to:
  /// **'Raw save inspection data'**
  String get inspectionJsonSubtitle;

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
  /// **'Public save name'**
  String get publicSaveName;

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
  /// **'Hero transform'**
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

  /// No description provided for @inventoryTitle.
  ///
  /// In en, this message translates to:
  /// **'Inventory'**
  String get inventoryTitle;

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

  /// No description provided for @itemCategoryAmmunition.
  ///
  /// In en, this message translates to:
  /// **'Ammunition'**
  String get itemCategoryAmmunition;

  /// No description provided for @itemCategoryArmor.
  ///
  /// In en, this message translates to:
  /// **'Armor'**
  String get itemCategoryArmor;

  /// No description provided for @itemCategoryRune.
  ///
  /// In en, this message translates to:
  /// **'Runes'**
  String get itemCategoryRune;

  /// No description provided for @itemCategoryScroll.
  ///
  /// In en, this message translates to:
  /// **'Spell scrolls'**
  String get itemCategoryScroll;

  /// No description provided for @itemCategoryFood.
  ///
  /// In en, this message translates to:
  /// **'Food & potions'**
  String get itemCategoryFood;

  /// No description provided for @itemCategoryMisc.
  ///
  /// In en, this message translates to:
  /// **'Miscellaneous'**
  String get itemCategoryMisc;

  /// No description provided for @itemCategoryAmulet.
  ///
  /// In en, this message translates to:
  /// **'Amulets'**
  String get itemCategoryAmulet;

  /// No description provided for @itemCategoryRing.
  ///
  /// In en, this message translates to:
  /// **'Rings'**
  String get itemCategoryRing;

  /// No description provided for @itemCategoryTrophy.
  ///
  /// In en, this message translates to:
  /// **'Animal trophies'**
  String get itemCategoryTrophy;

  /// No description provided for @itemCategoryWriting.
  ///
  /// In en, this message translates to:
  /// **'Writings'**
  String get itemCategoryWriting;

  /// No description provided for @itemCategoryMission.
  ///
  /// In en, this message translates to:
  /// **'Mission items'**
  String get itemCategoryMission;

  /// No description provided for @itemCategoryKey.
  ///
  /// In en, this message translates to:
  /// **'Keys'**
  String get itemCategoryKey;

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
  /// **'Remove this memory event? A backup is written first.'**
  String get removeMemoryEventBody;

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
  /// **'Duplicate this memory event? A backup is written first.'**
  String get duplicateMemoryEventBody;

  /// No description provided for @selectCharacterFromList.
  ///
  /// In en, this message translates to:
  /// **'Select a character from the list'**
  String get selectCharacterFromList;

  /// No description provided for @allDataLockedBody.
  ///
  /// In en, this message translates to:
  /// **'The full property browser needs decoded private payload data from the codec.'**
  String get allDataLockedBody;

  /// No description provided for @allDataDescription.
  ///
  /// In en, this message translates to:
  /// **'Search every typed property by name or path. Scalars, strings, enums and object paths are editable; structs are shown read-only for now.'**
  String get allDataDescription;

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

  /// No description provided for @extractLocalizedTextTitle.
  ///
  /// In en, this message translates to:
  /// **'Extract localized game text?'**
  String get extractLocalizedTextTitle;

  /// No description provided for @extractLocalizedTextBody.
  ///
  /// In en, this message translates to:
  /// **'Localized game text isn\'t extracted yet. Extract it now from your game install? (optional)'**
  String get extractLocalizedTextBody;

  /// No description provided for @notNow.
  ///
  /// In en, this message translates to:
  /// **'Not now'**
  String get notNow;

  /// No description provided for @extract.
  ///
  /// In en, this message translates to:
  /// **'Extract'**
  String get extract;

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

  /// No description provided for @aboutSubtitle.
  ///
  /// In en, this message translates to:
  /// **'Gothic Remake Savegame Editor'**
  String get aboutSubtitle;

  /// No description provided for @aboutVersion.
  ///
  /// In en, this message translates to:
  /// **'Version {version} ({sha})'**
  String aboutVersion(String version, String sha);

  /// No description provided for @aboutCopyright.
  ///
  /// In en, this message translates to:
  /// **'© 2026 goresave contributors'**
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
  /// **'Main stats'**
  String get heroGroupMainStats;

  /// No description provided for @heroGroupCombatSkills.
  ///
  /// In en, this message translates to:
  /// **'Combat skills'**
  String get heroGroupCombatSkills;

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

  /// No description provided for @heroEntryHeroTransform.
  ///
  /// In en, this message translates to:
  /// **'Hero transform'**
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

  /// No description provided for @localizedTextExtractedCount.
  ///
  /// In en, this message translates to:
  /// **'Extracted {idCount} ids across {languageCount} languages'**
  String localizedTextExtractedCount(int idCount, int languageCount);
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
