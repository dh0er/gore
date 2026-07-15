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
  String get managedContentWorkspaceLibraryLabel => 'This mod';

  @override
  String get managedWorkspaceHomeLabel => 'Home';

  @override
  String get managedWorkspaceStoryLabel => 'Story';

  @override
  String get managedWorkspaceWorldLabel => 'World';

  @override
  String get managedWorkspaceLocalizationVoiceLabel => 'Localization & Voice';

  @override
  String get managedWorkspaceValidateTestLabel => 'Validate & Test';

  @override
  String get managedWorkspaceBuildReleaseLabel => 'Build & Release';

  @override
  String get managedWorkspaceSettingsExpertLabel => 'Settings & Expert';

  @override
  String get managedSectionStoryDescription => 'NPCs, quests, and dialogue.';

  @override
  String get managedSectionWorldDescription =>
      'World placement and workflows are planned.';

  @override
  String get managedSectionLocalizationVoiceDescription =>
      'Voice production tools are available; managed localization editing is planned.';

  @override
  String get managedSectionValidateTestDescription =>
      'Verify exact project integrity and checkpoints; no runtime test is claimed.';

  @override
  String get managedSectionBuildReleaseDescription =>
      'Voice bundles are available; full playable builds and deployment are unavailable.';

  @override
  String get managedSectionSettingsExpertDescription =>
      'Settings are available; expert tools are not yet integrated.';

  @override
  String get managedSectionStatusHeading => 'Status';

  @override
  String get managedSectionActionsHeading => 'Actions';

  @override
  String get managedCapabilityAvailable => 'Available';

  @override
  String get managedCapabilityPartial => 'Partial';

  @override
  String get managedCapabilityPlanned => 'Planned';

  @override
  String get managedCapabilityUnavailable => 'Unavailable';

  @override
  String get managedProjectSubtitle =>
      'Exact-current offline authoring workspace';

  @override
  String get managedProjectLandingTitle => 'Managed project workspace';

  @override
  String get managedProjectLandingDescription =>
      'Use the new Home, Content, Story, Voice, validation, and release workflow in one managed project.';

  @override
  String get legacyCompatibilityToolsTitle => 'Legacy compatibility tools';

  @override
  String get legacyCompatibilityToolsDescription =>
      'The tabs below are older direct-replacement tools. They remain available while the managed project workspace grows.';

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
  String get managedActionBrowseProjectContentDescription =>
      'Browse exact project content and its resolved or unresolved references.';

  @override
  String get managedActionSettingsTitle => 'Settings';

  @override
  String get managedActionSettingsDescription =>
      'Configure the Gothic 1 Remake installation and Mod Studio preferences.';

  @override
  String projectStarterSetupOpenFailed(String projectId) {
    return 'Project $projectId was created safely, but the starter setup did not open. The valid empty project remains current.';
  }

  @override
  String projectStarterOutcomeUnverified(String projectId) {
    return 'Project $projectId was created, but Mod Studio cannot verify the starter outcome. Reopen the managed project before continuing; the game and save files were not changed.';
  }

  @override
  String projectStarterNpcCancelled(String projectId) {
    return 'Project $projectId was created. The NPC starter was not added, so the valid empty project remains current.';
  }

  @override
  String projectStarterNpcSaved(int projectRevision) {
    return 'NPC starter saved in project revision $projectRevision. It remains build-blocked, runtime-unqualified, and is not spawned.';
  }

  @override
  String projectStarterQuestCancelled(String projectId) {
    return 'Project $projectId was created. The Quest starter was not added, so the valid empty project remains current.';
  }

  @override
  String projectStarterQuestSaved(int projectRevision) {
    return 'Quest starter saved in project revision $projectRevision. It remains build-blocked and runtime-unqualified.';
  }

  @override
  String get projectStarterSemanticsLabel => 'Project starter';

  @override
  String get projectStarterPrompt => 'How would you like to start?';

  @override
  String get projectStarterWriteBoundary =>
      'Choosing a starter performs no writes. The project is created only after you submit this form and choose an empty folder.';

  @override
  String get projectStarterEmptyTitle => 'Empty project';

  @override
  String get projectStarterEmptyDescription =>
      'Create only the managed project. Add content whenever you are ready.';

  @override
  String get projectStarterNpcDraftTitle => 'NPC Draft';

  @override
  String get projectStarterNpcDraftDescription =>
      'Create the empty project first, then open the existing guided NPC Draft setup.';

  @override
  String get projectStarterQuestDraftTitle => 'Quest Draft';

  @override
  String get projectStarterQuestDraftDescription =>
      'Create the empty project first, then open the existing guided Quest Draft setup.';

  @override
  String get projectStarterPartialOutcome =>
      'For an NPC or Quest starter, canceling the guided setup or a Draft failure leaves a valid empty project. No starter selection writes to the game or a save.';

  @override
  String get managedContentWorkspaceBrowseLabel => 'Browse';

  @override
  String get managedContentWorkspaceVerifiedEditsLabel => 'Verified edits';

  @override
  String get managedContentScopeBaseGameLabel => 'Base game';

  @override
  String get managedContentScopeInstalledLabel => 'Installed';

  @override
  String get managedBaseGameBrowserTitle =>
      'Supported Base game starting points';

  @override
  String get managedBaseGameBrowserDescription =>
      'Browse exact installed-game evidence that Mod Studio can currently inspect or use as a safe Draft starting point. This is not a complete vanilla-content catalog.';

  @override
  String get managedBaseGameBrowserLoading =>
      'Reading exact Base game evidence…';

  @override
  String get managedBaseGameBrowserRefresh => 'Read a fresh exact catalog';

  @override
  String get managedBaseGameBrowserSearchLabel =>
      'Search supported Base game content';

  @override
  String get managedBaseGameBrowserFilterNpcs => 'NPCs';

  @override
  String get managedBaseGameBrowserFilterQuests => 'Quests';

  @override
  String get managedBaseGameBrowserNpcSectionTitle => 'NPC starting points';

  @override
  String get managedBaseGameBrowserQuestSectionTitle => 'Quest starting points';

  @override
  String get managedBaseGameBrowserExperimentalNpcSectionTitle =>
      'Inspect-only NPC archetypes';

  @override
  String get managedBaseGameBrowserSearchForExperimental =>
      'Search to include broader static-linkage NPC evidence. Those rows cannot create a Draft.';

  @override
  String get managedBaseGameBrowserEmpty =>
      'No supported Base game result matches this search.';

  @override
  String get managedBaseGameBrowserLoadErrorTitle =>
      'Base game evidence unavailable';

  @override
  String get managedBaseGameBrowserLoadErrorDescription =>
      'The exact supported catalog could not be read. No project, game, or save files were changed.';

  @override
  String get managedBaseGameBrowserOfflineDraftBadge =>
      'Offline Draft supported';

  @override
  String get managedBaseGameBrowserInspectOnlyBadge => 'Inspect only';

  @override
  String get managedBaseGameBrowserCreateNpcDraft =>
      'Use as NPC starting point';

  @override
  String get managedBaseGameBrowserCreateQuestDraft =>
      'Use as Quest starting point';

  @override
  String get managedBaseGameBrowserSpawnClass => 'Spawn definition';

  @override
  String get managedBaseGameBrowserActorBlueprint => 'Actor Blueprint';

  @override
  String get managedBaseGameBrowserExperimentalResultsCapped =>
      'Showing the first 100 inspect-only matches. Refine the search for more specific results.';

  @override
  String get managedInstalledBrowserLoading =>
      'Reading the exact installed package inventory…';

  @override
  String managedInstalledBrowserCompleteSummary(int count) {
    return '$count installed package candidates';
  }

  @override
  String managedInstalledBrowserPartialSummary(int count) {
    return '$count installed package candidates — partial result';
  }

  @override
  String get managedInstalledBrowserCompleteDescription =>
      'Directory metadata was read and the installed snapshot stayed exact.';

  @override
  String get managedInstalledBrowserPartialDescription =>
      'Some package metadata was missing or noncanonical, so results are useful for discovery but not complete.';

  @override
  String get managedInstalledBrowserAuthorityNotice =>
      'This scope exposes installed DataAsset package metadata only. Inspecting or copying a path grants no build, deployment, runtime, or game-write authority.';

  @override
  String get managedInstalledBrowserRefresh => 'Read a fresh exact snapshot';

  @override
  String get managedInstalledBrowserSearchLabel =>
      'Search installed DataAssets';

  @override
  String get managedInstalledBrowserSearchHint => 'Asset name or /Game path';

  @override
  String get managedInstalledBrowserSearchPrompt =>
      'Type an asset name or /Game path to search.';

  @override
  String get managedInstalledBrowserNoMatchesTitle =>
      'No matching installed DataAsset';

  @override
  String get managedInstalledBrowserNoMatchesDescription =>
      'Try another asset name or a broader /Game path.';

  @override
  String get managedInstalledBrowserResultLimitDescription =>
      'Showing the first 100 matches. Refine the search to narrow the exact snapshot.';

  @override
  String get managedInstalledBrowserKindBadge => 'DataAsset package';

  @override
  String get managedInstalledBrowserMetadataOnlyBadge => 'Metadata only';

  @override
  String get managedInstalledBrowserOpenInspector => 'Inspect exact package';

  @override
  String get managedInstalledBrowserErrorTitle =>
      'Installed package inventory unavailable';

  @override
  String get managedInstalledBrowserErrorDescription =>
      'The exact installed snapshot could not be read. No project, game, or save files were changed.';
}
