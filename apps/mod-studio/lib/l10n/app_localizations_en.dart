// ignore: unused_import
import 'package:intl/intl.dart' as intl;
import 'app_localizations.dart';

// ignore_for_file: type=lint

/// The translations for English (`en`).
class AppLocalizationsEn extends AppLocalizations {
  AppLocalizationsEn([String locale = 'en']) : super(locale);

  @override
  String get tabItems => 'Items';

  @override
  String get tabOverrides => 'Changes';

  @override
  String get tabSettings => 'Settings';

  @override
  String get tabDialogs => 'Dialogs';

  @override
  String get tabAudio => 'Audio';

  @override
  String get tabTextures => 'Textures';

  @override
  String get tabScripts => 'Scripts';

  @override
  String get changesAll => 'All';

  @override
  String get sectionItemValues => 'Item values';

  @override
  String get sectionLocalizedText => 'Localized text';

  @override
  String get audioCatCreatures => 'Creatures';

  @override
  String get audioCatObjects => 'Objects';

  @override
  String get audioCatMagic => 'Magic';

  @override
  String get audioCatMovement => 'Movement';

  @override
  String get audioCatWorld => 'World';

  @override
  String get audioCatAction => 'Action';

  @override
  String get audioCatCombat => 'Combat';

  @override
  String get audioCatPhysics => 'Physics';

  @override
  String get audioCatItems => 'Items';

  @override
  String get audioCatUi => 'UI';

  @override
  String get audioCatFoley => 'Foley';

  @override
  String get audioCatUnderwater => 'Underwater';

  @override
  String get audioCatVision => 'Vision';

  @override
  String get audioCatDialog => 'Dialog';

  @override
  String get audioCatOther => 'Other';

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
  String get extractLocalizedText => 'Extract localized text';

  @override
  String get lightMode => 'Light mode';

  @override
  String get darkMode => 'Dark mode';

  @override
  String get language => 'Language';

  @override
  String get exportMod => 'Export mod';

  @override
  String exportModWithCount(int count) {
    return 'Export mod ($count)';
  }

  @override
  String get selectAnItemToEdit => 'Select an item to edit its fields.';

  @override
  String gameDataActiveTooltip(String name) {
    return 'Game data: $name';
  }

  @override
  String get gameDataBundledTooltip => 'Game data: bundled';

  @override
  String get loadGameDataDump => 'Load game-data dump…';

  @override
  String get loadGameDataDumpSubtitle =>
      'gore_game_data.json from the gore-dump mod';

  @override
  String get useBundledData => 'Use bundled data';

  @override
  String get alreadyBundled => 'already bundled';

  @override
  String get gameDataFileGroupLabel => 'game data';

  @override
  String get minimize => 'Minimize';

  @override
  String get restore => 'Restore';

  @override
  String get maximize => 'Maximize';

  @override
  String get close => 'Close';

  @override
  String get about => 'About';

  @override
  String aboutVersion(String version, String sha) {
    return 'Version $version ($sha)';
  }

  @override
  String get aboutCopyright => '© 2026 GORE contributors';

  @override
  String get aboutLicense => 'Licensed under the MIT License.';

  @override
  String get categoryMeleeWeapons => 'Melee weapons';

  @override
  String get categoryRangedWeapons => 'Ranged weapons';

  @override
  String get categoryAmmunition => 'Ammunition';

  @override
  String get categoryRunes => 'Runes';

  @override
  String get categorySpellScrolls => 'Spell scrolls';

  @override
  String get categoryFoodAndPotions => 'Food & potions';

  @override
  String get categoryMiscellaneous => 'Miscellaneous';

  @override
  String get categoryAmulets => 'Amulets';

  @override
  String get categoryRings => 'Rings';

  @override
  String get categoryAnimalTrophies => 'Animal trophies';

  @override
  String get categoryWritings => 'Writings';

  @override
  String get categoryMissionItems => 'Mission items';

  @override
  String get categoryKeys => 'Keys';

  @override
  String get categoryOther => 'Other';

  @override
  String categoryWithCount(String label, int count) {
    return '$label ($count)';
  }

  @override
  String get searchItems => 'Search items';

  @override
  String get noItemsMatch => 'No items match';

  @override
  String failedToLoadCatalog(String error) {
    return 'Failed to load catalog: $error';
  }

  @override
  String pendingOverridesWithCount(int count) {
    return 'Pending overrides ($count)';
  }

  @override
  String get clearAll => 'Clear all';

  @override
  String get noPendingOverrides =>
      'No pending overrides.\nEdit item fields to add some.';

  @override
  String get removeOverride => 'Remove override';

  @override
  String get searchChanges => 'Search changes';

  @override
  String get noChangesMatch => 'No changes match';

  @override
  String get clearSection => 'Clear this group';

  @override
  String get modName => 'Mod name';

  @override
  String get loadDelayLabel => 'Load delay (ms, 0 = instant)';

  @override
  String get noFolderSelected => 'No folder selected';

  @override
  String get chooseFolder => 'Choose folder';

  @override
  String get packageAsZip => 'Package as .zip';

  @override
  String get cancel => 'Cancel';

  @override
  String get export => 'Export';

  @override
  String get exportHere => 'Export here';

  @override
  String get mustBeNonNegativeInteger => 'Must be a non-negative integer';

  @override
  String get extractingLocalizedText => 'Extracting localized game text…';

  @override
  String get localizedTextExtractionCancelled =>
      'Localized text extraction cancelled.';

  @override
  String get localizedTextExtracted => 'Localized text extracted.';

  @override
  String get extractionFailed => 'Extraction failed.';

  @override
  String get localizationCacheFileGroupLabel => 'localization cache';

  @override
  String get extractLocalizedTextQuestion => 'Extract localized game text?';

  @override
  String get extractLocalizedTextBody =>
      'Localized game text isn\'t extracted yet. Extract it now from your game install? (optional)';

  @override
  String get notNow => 'Not now';

  @override
  String get extract => 'Extract';

  @override
  String get validationRequired => 'Required';

  @override
  String get validationMustBeWholeNumber => 'Must be a whole number';

  @override
  String get validationMustBeNumber => 'Must be a number';

  @override
  String get validationMustBeFinite => 'Must be a finite number';

  @override
  String validationMustBeAtLeast(String min) {
    return 'Must be ≥ $min';
  }

  @override
  String validationMustBeAtMost(String max) {
    return 'Must be ≤ $max';
  }

  @override
  String get validationMustBeBool => 'Must be true or false';

  @override
  String validationMustBeOneOf(String options) {
    return 'Must be one of: $options';
  }

  @override
  String get modNameRequired => 'Required';

  @override
  String get modNameControlCharacters => 'Must not contain control characters';

  @override
  String get modNamePathSeparators => 'Must not contain path separators';

  @override
  String get modNameNotAFolderName => 'Not a valid folder name';

  @override
  String localizedTextExtractedCount(int idCount, int languageCount) {
    return 'Extracted $idCount ids across $languageCount languages';
  }

  @override
  String get managerDeployActive =>
      'A mod-manager loadout is active. Undeploy it in gore-manager first.';

  @override
  String get projectOpenLegacy => 'Open legacy project…';

  @override
  String get projectOpenManagedRevision3 => 'Open managed revision-3 project…';

  @override
  String get projectVerifyCurrentHead => 'Verify current head';

  @override
  String get projectManagedRevision3Title => 'Managed revision-3 project';

  @override
  String get projectManagedRevision3IdentityOnly =>
      'This shell currently exposes verified project identity only. Ctrl+S reopens and verifies the exact current head; legacy editors, Build/Deploy, and Save As are unavailable.';

  @override
  String get projectRoot => 'Root';

  @override
  String get projectId => 'Project ID';

  @override
  String get projectRevision => 'Project revision';

  @override
  String get projectHeadSha256 => 'Head SHA-256';

  @override
  String get projectSnapshotBytes => 'Snapshot bytes';

  @override
  String get projectNoCurrent => 'No current project';

  @override
  String projectManagedRevision3Opened(String projectId) {
    return 'Opened managed revision-3 project $projectId';
  }

  @override
  String projectManagedRevision3OpenFailed(String error) {
    return 'Managed revision-3 project open failed: $error';
  }

  @override
  String projectManagedRevision3Verified(String headSha256) {
    return 'Verified revision-3 head $headSha256';
  }

  @override
  String projectManagedRevision3VerifyFailed(String error) {
    return 'Revision-3 head verification failed: $error';
  }

  @override
  String get projectManagedRevision3RequiresReopen =>
      'Exact-head verification could not complete safely. This session now requires recovery and further verification is blocked. Close Mod Studio, then reopen this project before continuing.';

  @override
  String get projectManagedRevision3VerifyBlocked =>
      'Verification is blocked until the managed project is reopened.';

  @override
  String get projectTransitionCleanupWarning =>
      'The new project is open, but the previous project session could not be cleaned up completely. No cleanup retry will be attempted. Restart Mod Studio before reopening the retired project.';

  @override
  String get projectNewManagedRevision3 => 'New managed mod project…';

  @override
  String get projectNewLegacy => 'New legacy project';

  @override
  String get projectCreateGamePathRequired =>
      'Set the Gothic 1 Remake game path in Settings before creating a mod project.';

  @override
  String get projectCreateDirectoryPickerTitle =>
      'Create managed mod project here';

  @override
  String projectManagedRevision3Created(String projectId) {
    return 'Created managed mod project $projectId';
  }

  @override
  String projectManagedRevision3CreateFailed(String error) {
    return 'Managed mod project creation failed: $error';
  }

  @override
  String get projectCreateDialogTitle => 'Create a mod project';

  @override
  String get projectCreateNameLabel => 'Project name';

  @override
  String get projectCreateNameHelper => 'The name shown in Mod Studio.';

  @override
  String get projectCreateVersionLabel => 'Version';

  @override
  String get projectCreateVersionHelper => 'A starting version, such as 0.1.0.';

  @override
  String get projectCreateAuthorLabel => 'Author';

  @override
  String get projectCreateAuthorHelper => 'Your name or mod-team name.';

  @override
  String get projectCreateLocalesLabel => 'Authoring languages';

  @override
  String get projectCreateLocalesHelper =>
      'Comma-separated canonical tags, for example: en, de, en-US.';

  @override
  String get projectCreateBoundary =>
      'This creates an empty managed offline project. It does not build, deploy, or run a mod, and it does not change game files or save files.';

  @override
  String get projectCreateSubmit => 'Create project';

  @override
  String projectCreateMetadataRequired(String label) {
    return '$label is required.';
  }

  @override
  String projectCreateMetadataNoOuterWhitespace(String label) {
    return '$label cannot start or end with whitespace.';
  }

  @override
  String projectCreateMetadataControlCharacters(String label) {
    return '$label cannot contain control characters.';
  }

  @override
  String projectCreateMetadataMalformed(String label) {
    return '$label contains malformed text.';
  }

  @override
  String projectCreateMetadataTooLong(String label, int maxBytes) {
    return '$label exceeds its $maxBytes-byte UTF-8 limit.';
  }

  @override
  String get projectCreateLocalesRequired =>
      'Enter at least one authoring language.';

  @override
  String get projectCreateLocalesEmptyEntry =>
      'Remove the empty authoring-language entry.';

  @override
  String projectCreateLocalesTooMany(int maxLocales) {
    return 'Use at most $maxLocales authoring languages.';
  }

  @override
  String projectCreateLocaleBoundedAscii(String locale) {
    return 'Locale \"$locale\" must be bounded ASCII.';
  }

  @override
  String projectCreateLocaleLanguage(String locale) {
    return 'Locale \"$locale\" needs a 2-8 letter lowercase language.';
  }

  @override
  String projectCreateLocaleInvalidSegment(String locale) {
    return 'Locale \"$locale\" has an invalid segment.';
  }

  @override
  String projectCreateLocaleNotCanonical(String locale, String canonical) {
    return 'Locale \"$locale\" is not canonical; use \"$canonical\".';
  }

  @override
  String get managedWorkspaceOverviewLabel => 'Overview';

  @override
  String get managedWorkspaceContentLabel => 'Content';

  @override
  String get managedWorkspaceDataAssetsLabel => 'DataAssets';

  @override
  String get managedProjectSubtitle =>
      'Exact-current offline authoring workspace';

  @override
  String get managedProjectTechnicalDetails => 'Technical project details';

  @override
  String get managedProjectRecoveryContentLocked =>
      'Reopen the managed project before reading its content.';

  @override
  String get managedDashboardUntitledProject => 'Untitled project';

  @override
  String get managedDashboardDraftStatus => 'Draft';

  @override
  String get managedDashboardProjectVersion => 'Version';

  @override
  String get managedDashboardProjectAuthor => 'Author';

  @override
  String get managedDashboardNotProvided => 'Not provided';

  @override
  String get managedDashboardContentCounts => 'Project content';

  @override
  String get managedDashboardNpcDrafts => 'NPC drafts';

  @override
  String get managedDashboardQuestDrafts => 'Quest drafts';

  @override
  String get managedDashboardDialogLines => 'Dialog lines';

  @override
  String get managedDashboardVoiceTakes => 'Voice takes';

  @override
  String get managedDashboardAssets => 'Assets';

  @override
  String get managedDashboardUnresolvedReferences => 'Unresolved references';

  @override
  String get managedDashboardReadiness => 'What works now';

  @override
  String get managedDashboardOfflineAuthoringTitle =>
      'Offline authoring available';

  @override
  String get managedDashboardOfflineAuthoringDescription =>
      'Create and edit supported project content without changing the game installation or save files.';

  @override
  String get managedDashboardGeneralBuildBlockedTitle =>
      'General mod build unavailable';

  @override
  String get managedDashboardGeneralBuildBlockedDescription =>
      'Only sealed offline Voice bundles can be built; a complete playable mod cannot be built yet.';

  @override
  String get managedDashboardRuntimeUnqualifiedTitle =>
      'Runtime not yet verified';

  @override
  String get managedDashboardRuntimeUnqualifiedDescription =>
      'Mod Studio has not proven this project content inside the running game.';

  @override
  String get managedDashboardReferenceIntegrityTitle => 'Reference integrity';

  @override
  String get managedDashboardReferenceIntegrityDescription =>
      'This count checks project references only; it is not build or runtime readiness.';

  @override
  String get managedDashboardMissingGameTitle => 'Game setup required';

  @override
  String get managedDashboardMissingGameDescription =>
      'Configure the Gothic 1 Remake installation in Settings before using actions that need installed-game evidence.';

  @override
  String get managedDashboardCreateHeading => 'Create';

  @override
  String get managedDashboardToolsHeading => 'Project tools';

  @override
  String get managedDashboardLoading => 'Loading project overview';

  @override
  String get managedDashboardLoadError => 'Project overview unavailable';

  @override
  String get managedDashboardLoadErrorDescription =>
      'The verified project overview could not be loaded. Project content was not changed.';

  @override
  String get managedDashboardRetry => 'Retry';

  @override
  String get managedActionNewNpcTitle => 'New NPC';

  @override
  String get managedActionNewNpcDescription =>
      'Create a bounded offline NPC draft from verified installed-game evidence.';

  @override
  String get managedActionNewQuestTitle => 'New Quest';

  @override
  String get managedActionNewQuestDescription =>
      'Create an offline Quest draft with objectives and verified parent identities.';

  @override
  String get managedActionAddVoiceTakeTitle => 'Add Voice take';

  @override
  String get managedActionAddVoiceTakeDescription =>
      'Import an Ogg Vorbis recording into this project without deploying it.';

  @override
  String get managedActionManageVoiceTakesTitle => 'Manage Voice takes';

  @override
  String get managedActionManageVoiceTakesDescription =>
      'Review takes and select approved recordings for Voice slots.';

  @override
  String get managedActionResolveVoiceTargetTitle => 'Resolve Voice target';

  @override
  String get managedActionResolveVoiceTargetDescription =>
      'Match project Voice slots to exact installed archive members without changing the game.';

  @override
  String get managedActionBuildVoiceBundleTitle => 'Build Voice bundle';

  @override
  String get managedActionBuildVoiceBundleDescription =>
      'Build a sealed offline existing-member bundle; deployment is not performed.';

  @override
  String get managedActionDataAssetsTitle => 'DataAsset edits';

  @override
  String get managedActionDataAssetsDescription =>
      'Inspect installed packages and stage verified fixed-width value edits in the project.';

  @override
  String get managedActionSettingsTitle => 'Settings';

  @override
  String get managedActionSettingsDescription =>
      'Configure the Gothic 1 Remake installation and Mod Studio preferences.';
}
