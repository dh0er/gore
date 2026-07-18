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
  String get projectOpenManagedRevision3 => 'Open mod project…';

  @override
  String get projectVerifyCurrentHead => 'Check project';

  @override
  String get projectManagedRevision3Title => 'Mod project';

  @override
  String get projectClose => 'Close project';

  @override
  String projectCloseFailed(String error) {
    return 'Project could not be closed: $error';
  }

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
  String get projectManagedRevision3Opened => 'Mod project opened.';

  @override
  String projectManagedRevision3OpenFailed(String error) {
    return 'Mod project could not be opened: $error';
  }

  @override
  String get projectManagedRevision3Verified => 'Project checked.';

  @override
  String projectManagedRevision3VerifyFailed(String error) {
    return 'Project check failed: $error';
  }

  @override
  String get projectManagedRevision3RequiresReopen =>
      'The project could not be checked safely. Recover or reopen it before continuing.';

  @override
  String get projectManagedRevision3VerifyBlocked =>
      'Recover or reopen the project before checking it again.';

  @override
  String get projectTransitionCleanupWarning =>
      'The new project is open, but the previous project session could not be cleaned up completely. No cleanup retry will be attempted. Restart Mod Studio before reopening the retired project.';

  @override
  String get projectNewManagedRevision3 => 'New mod project…';

  @override
  String get projectCreateGamePathRequired =>
      'Set the Gothic 1 Remake game path in Settings before creating a mod project.';

  @override
  String get projectCreateDirectoryPickerTitle => 'Create mod project here';

  @override
  String projectManagedRevision3Created(String projectId) {
    return 'Created mod project $projectId';
  }

  @override
  String projectManagedRevision3CreateFailed(String error) {
    return 'Mod project creation failed: $error';
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
  String get managedWorkspaceSettingsExpertLabel => 'Settings & Expert';

  @override
  String get managedProjectHistoryTitle => 'Project history';

  @override
  String get managedProjectHistoryDescription =>
      'Return to an earlier project version without erasing the versions that came after it.';

  @override
  String get managedProjectHistoryBoundary =>
      'History changes only this managed project. It does not modify the game installation or save files.';

  @override
  String get managedProjectHistoryRefresh => 'Refresh project history';

  @override
  String get managedProjectHistoryLoading => 'Loading project history…';

  @override
  String get managedProjectHistoryLoadFailed =>
      'Project history could not be loaded';

  @override
  String get managedProjectHistoryRetry => 'Try again';

  @override
  String get managedProjectHistoryCurrentVersion => 'Current version';

  @override
  String get managedProjectHistoryPreviousVersions => 'Previous versions';

  @override
  String get managedProjectHistoryUndo => 'Undo last change';

  @override
  String get managedProjectHistoryRestoreVersion => 'Restore this version';

  @override
  String get managedProjectHistoryRestoreTitle => 'Restore project version?';

  @override
  String managedProjectHistoryRestoreBody(int revision, int nextRevision) {
    return 'The content from revision $revision will be saved as new revision $nextRevision. The current version remains in history.';
  }

  @override
  String get managedProjectHistoryRestoreBoundary =>
      'Only the project changes. The game installation and save files remain untouched.';

  @override
  String get managedProjectHistoryCancel => 'Cancel';

  @override
  String get managedProjectHistoryRestore => 'Restore';

  @override
  String get managedProjectHistoryRestoring => 'Restoring project version…';

  @override
  String get managedProjectHistoryRestoreFailed =>
      'The project version could not be restored safely. Refresh the history before trying again.';

  @override
  String managedProjectHistoryRestoreSucceeded(int revision) {
    return 'Revision $revision was restored as a new project version.';
  }

  @override
  String get managedProjectHistoryEmpty =>
      'No previous project versions have been recorded yet.';

  @override
  String managedProjectHistoryRecordingStartsAt(int revision) {
    return 'History recording starts at revision $revision; older versions were not guessed from storage.';
  }

  @override
  String get managedProjectHistoryTruncated =>
      'Older project versions have expired from history. Every version shown here is still retained and authenticated by the current project history.';

  @override
  String managedProjectHistoryRevision(int revision) {
    return 'Revision $revision';
  }

  @override
  String get managedProjectHistoryCurrentBadge => 'Current';

  @override
  String get managedProjectHistoryDirtyBlocked =>
      'Finish or discard the open text edit before restoring another project version.';

  @override
  String get managedProjectHistoryBusy =>
      'Another project action is still in progress.';

  @override
  String get managedProjectHistoryUnavailable =>
      'This managed project session does not support authenticated history.';

  @override
  String get managedSectionStoryDescription => 'NPCs, quests, and dialogue.';

  @override
  String get managedStoryWorkspaceLoading =>
      'Opening the current Story drafts…';

  @override
  String get managedStoryWorkspaceAuthorityNotice =>
      'Project-only NPC and Quest drafts. Build readiness has not been evaluated; runtime behavior remains unqualified.';

  @override
  String get managedStoryWorkspaceSearchHint =>
      'Search NPC and Quest names, objectives, speakers, or IDs';

  @override
  String get managedStoryWorkspaceCreatingNpc => 'Creating NPC draft…';

  @override
  String get managedStoryWorkspaceCreatingQuest => 'Creating Quest draft…';

  @override
  String get managedStoryWorkspaceCreateNpcOpening =>
      'Create Character + first greeting';

  @override
  String get managedStoryWorkspaceCreatingNpcOpening =>
      'Creating Character + first greeting…';

  @override
  String get managedStoryWorkspaceCreateQuestOpening =>
      'Create Quest + opening line';

  @override
  String get managedStoryWorkspaceCreatingQuestOpening =>
      'Creating Quest + opening line…';

  @override
  String get managedStoryWorkspaceCreateAdvanced => 'Advanced creation options';

  @override
  String get managedStoryWorkspaceCreateNpcAdvanced =>
      'Create Character draft only (advanced)';

  @override
  String get managedStoryWorkspaceCreateQuestAdvanced =>
      'Create Quest draft only (advanced)';

  @override
  String get managedStoryWorkspaceMutationRequiresReopen =>
      'Reopen this project before changing Story content.';

  @override
  String get managedStoryWorkspaceMutationDirtyBlocked =>
      'Save or discard the open localization edits before changing Story content.';

  @override
  String get managedStoryWorkspaceEmpty => 'No NPC or Quest drafts yet';

  @override
  String get managedStoryWorkspaceNoMatches =>
      'No NPC or Quest drafts match this search';

  @override
  String get managedStoryWorkspaceSelectDraft =>
      'Select an NPC or Quest draft to continue';

  @override
  String get managedStoryWorkspaceLoadErrorTitle =>
      'Story drafts could not be opened';

  @override
  String get managedStoryWorkspaceCheckpointMismatch =>
      'The project changed while Story was loading. Refresh the exact current checkpoint and try again.';

  @override
  String get managedStoryWorkspacePublishedSelectionStale =>
      'The saved Story draft could not be selected at its exact project revision. Check the current Story list before continuing.';

  @override
  String managedStoryWorkspaceCheckpointSummary(int count, int revision) {
    return 'NPC and Quest drafts: $count · project revision $revision';
  }

  @override
  String managedStoryWorkspaceLoadErrorDetails(String error) {
    return 'The exact current Story view could not be read: $error';
  }

  @override
  String managedStoryWorkspaceCreateErrorDetails(String error) {
    return 'The Story draft could not be created: $error';
  }

  @override
  String managedStoryWorkspaceDetailsSheetLabel(String entityName) {
    return '$entityName Story details';
  }

  @override
  String get managedStoryWorkspaceRemovePairUnavailable =>
      'This draft is not an exact removable draft and generated-script pair.';

  @override
  String get managedStoryWorkspaceRemoveBusy =>
      'Another Story action is still in progress.';

  @override
  String get managedStoryWorkspaceRemoveRequiresReopen =>
      'Reopen this managed project before removing a draft.';

  @override
  String managedStoryWorkspaceRemoveBlocked(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count incoming project references must be removed first.',
      one: '1 incoming project reference must be removed first.',
    );
    return '$_temp0';
  }

  @override
  String get managedStoryWorkspaceRemoveDialogTitle =>
      'Remove draft from project?';

  @override
  String managedStoryWorkspaceRemoveDialogSummary(
    String draftName,
    String scriptName,
  ) {
    return 'This removes the draft \'$draftName\' together with its uniquely owned generated script \'$scriptName\'.';
  }

  @override
  String get managedStoryWorkspaceRemoveNoUndo =>
      'This removal cannot be undone in version 1.';

  @override
  String get managedStoryWorkspaceRemoveBoundary =>
      'Only the current project registry is changed. The game installation and save games stay unchanged.';

  @override
  String get managedStoryWorkspaceRemoveCancel => 'Cancel';

  @override
  String get managedStoryWorkspaceRemoveConfirm => 'Remove draft';

  @override
  String get managedStoryWorkspaceRemoveBlockedTitle =>
      'Draft is still referenced';

  @override
  String get managedStoryWorkspaceRemoveBlockedDescription =>
      'Open every source below and remove its project reference before trying again.';

  @override
  String managedStoryWorkspaceRemoveBlockerLabel(
    String sourceName,
    String role,
  ) {
    return '$sourceName · $role';
  }

  @override
  String get managedStoryWorkspaceRemoveOpenBlocker =>
      'Open referencing source';

  @override
  String get managedStoryWorkspaceRemoveBlockedClose => 'Close';

  @override
  String managedStoryWorkspaceRemoveSucceeded(String draftName) {
    return 'Removed \'$draftName\' and its generated script from the project. Game files and save games were not changed.';
  }

  @override
  String managedStoryWorkspaceRemoveError(String error) {
    return 'The draft was not removed. The Story view was refreshed without retrying automatically: $error';
  }

  @override
  String get managedSectionLocalizationVoiceDescription =>
      'Write and translate project dialog, then review each language\'s takes, selection, and target in the same workspace.';

  @override
  String get managedLocalizationProjectTextsLabel => 'Project texts';

  @override
  String get managedLocalizationSearchLabel => 'Search project texts';

  @override
  String get managedLocalizationRefresh => 'Refresh';

  @override
  String get managedLocalizationEmptyTitle => 'No project text yet';

  @override
  String get managedLocalizationEmptyDescription =>
      'Create a dialog line to start writing and translating text.';

  @override
  String get managedLocalizationLoadFailed =>
      'Project texts could not be opened';

  @override
  String get managedLocalizationSelectText => 'Select a project text to edit';

  @override
  String get managedLocalizationLanguagesLabel => 'Languages';

  @override
  String get managedLocalizationUsedByLines => 'Used by dialog lines';

  @override
  String get managedLocalizationVoiceContextTitle =>
      'Voice for this dialog line';

  @override
  String get managedLocalizationVoiceSelectLine => 'Select a dialog line above';

  @override
  String get managedLocalizationVoiceSetupExists => 'setup exists';

  @override
  String get managedLocalizationVoiceSetupMissing => 'no setup yet';

  @override
  String get managedLocalizationNoLine => 'Not used by a dialog line yet';

  @override
  String get managedLocalizationSpeakerLabel => 'Speaker label';

  @override
  String get managedLocalizationAddLanguage => 'Add language';

  @override
  String get managedLocalizationRemoveLanguage => 'Remove language';

  @override
  String get managedLocalizationLanguageHint => 'For example de, en, or pt-BR';

  @override
  String get managedLocalizationLanguageExists =>
      'This language is already present.';

  @override
  String get managedLocalizationAdd => 'Add';

  @override
  String get managedLocalizationSaved => 'Project text saved';

  @override
  String get managedLocalizationVoiceLocked =>
      'This text has recorded voice takes, so its transcript is locked in this editor.';

  @override
  String get managedLocalizationVoiceSlotRemovalLocked =>
      'This language is connected to a Voice slot and cannot be removed here.';

  @override
  String get managedLocalizationMinimumLanguageLocked =>
      'Keep at least one language for this project text.';

  @override
  String get managedLocalizationSharedNotice =>
      'This project text is shared. Saving changes updates every listed dialog line.';

  @override
  String get managedLocalizationOfflineNotice =>
      'Changes are saved only to this managed project. Build and in-game behavior remain separate.';

  @override
  String get managedLocalizationUnsavedTitle => 'Discard unsaved changes?';

  @override
  String get managedLocalizationUnsavedDescription =>
      'You changed this project text. Switching now would discard those edits.';

  @override
  String get managedLocalizationVoiceUnsavedTitle =>
      'Save text before continuing?';

  @override
  String get managedLocalizationVoiceUnsavedDescription =>
      'Save these text changes and continue directly to the selected action, keep editing, or deliberately discard the text changes.';

  @override
  String get managedLocalizationDiscardAndContinue => 'Discard and continue';

  @override
  String get managedLocalizationSaveAndContinue => 'Save and continue';

  @override
  String get managedLocalizationGlobalAddVoice => 'Add take for any line';

  @override
  String get managedLocalizationGlobalManageVoice =>
      'Manage takes for any line';

  @override
  String get managedLocalizationGlobalResolveVoice =>
      'Resolve target for any line';

  @override
  String get managedVoiceFolderImportTitle => 'Import recordings folder';

  @override
  String get managedVoiceFolderImportDescription =>
      'Review a folder of named Ogg recordings, then add every ready take in one all-or-nothing project update.';

  @override
  String get managedVoiceFolderImportChooseFolder => 'Choose recordings folder';

  @override
  String get managedVoiceFolderImportDirtyBlocked =>
      'Save or discard the open localization edits before importing recordings.';

  @override
  String managedVoiceFolderImportSaved(int count, int revision) {
    return 'Imported $count recordings in project revision $revision. They are project-only Recorded takes; selection, game files, and saves were not changed.';
  }

  @override
  String managedVoiceTakeSaved(int revision) {
    return 'Voice take saved in project revision $revision. It is saved to the project only and is not yet usable in game.';
  }

  @override
  String managedVoiceSelectionCleared(int revision) {
    return 'Voice selection cleared in project revision $revision. Voice build remains a separate offline step; runtime remains unqualified.';
  }

  @override
  String managedVoiceSelectionSelected(int revision) {
    return 'Approved Voice take selected in project revision $revision. Voice build remains a separate offline step; runtime remains unqualified.';
  }

  @override
  String managedVoiceTargetUnresolvedSaved(int revision) {
    return 'No installed archive member matched. Voice target evidence saved in project revision $revision.';
  }

  @override
  String managedVoiceTargetResolvedSaved(int revision) {
    return 'One installed archive member was sealed. Voice target evidence saved in project revision $revision.';
  }

  @override
  String managedVoiceTargetAmbiguousSaved(int count, int revision) {
    return '$count installed archive members matched; nothing was chosen implicitly. Voice target evidence saved in project revision $revision.';
  }

  @override
  String get managedLocalizationDiscard => 'Discard changes';

  @override
  String get managedLocalizationKeepEditing => 'Keep editing';

  @override
  String get managedLocalizationStale =>
      'The project changed while this text was open. Refresh and try again.';

  @override
  String get managedLocalizationReopen =>
      'The project must be reopened before text editing can continue.';

  @override
  String get managedLocalizationInvalid =>
      'Check that every language and dialog text is valid and not empty.';

  @override
  String get managedLocalizationSaveFailed =>
      'The project text could not be saved.';

  @override
  String get managedLocalizationVoiceActionFailed =>
      'The selected action did not finish cleanly. Refresh the project before trying again; the exact current project will show whether a change was published. This workspace did not change game or save files.';

  @override
  String get managedSectionSettingsExpertDescription =>
      'Settings and the read-only DataAsset Lab are available.';

  @override
  String get managedSettingsExpertDataAssetLabLabel => 'DataAsset Lab';

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
  String get managedProjectLandingTitle => 'Start a mod project';

  @override
  String get managedProjectLandingDescription =>
      'Create a project, open an existing project folder, or restore a project backup.';

  @override
  String get managedProjectTechnicalDetails => 'Technical project details';

  @override
  String get managedProjectRecoveryContentLocked =>
      'Recover or reopen the managed project before reading its content.';

  @override
  String get managedProjectRecoveryDescription =>
      'Mod Studio will safely reopen this project while keeping its lock. This does not change the game or any save.';

  @override
  String get managedProjectRecoveryTry => 'Try recovery';

  @override
  String get managedProjectRecoveryTrying => 'Trying recovery…';

  @override
  String get managedProjectRecoveryAlternative =>
      'If recovery does not work, close and open the project again.';

  @override
  String get managedProjectRecoverySucceeded =>
      'Project recovery completed. You can continue working.';

  @override
  String get managedProjectRecoveryFailed =>
      'Recovery did not complete. Try again, or close and open the project again.';

  @override
  String get managedProjectRecoveryUnavailable =>
      'Recovery is not available for this project. Close and open the project again.';

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
  String get managedDashboardContinueHeading => 'Continue working';

  @override
  String get managedHomeStoryEmptyTitle => 'Create a character or Quest';

  @override
  String get managedHomeStoryContinueTitle => 'Continue Story';

  @override
  String get managedHomeStoryDescription =>
      'Create and develop NPC and Quest drafts in the complete Story workspace.';

  @override
  String get managedHomeDialogVoiceTitle => 'Dialog & Voice';

  @override
  String get managedHomeDialogVoiceDescription =>
      'Write project text, create dialog lines, and manage Voice takes in one place.';

  @override
  String get managedHomeProblemsTitle => 'Review problems';

  @override
  String get managedHomeProblemsDescription =>
      'Review exact project issues and verification without claiming a runtime test.';

  @override
  String get managedHomeContentTitle => 'Browse content';

  @override
  String get managedHomeContentDescription =>
      'Find project, base-game, installed, and verified DataAsset content.';

  @override
  String get managedHomeBuildTitle => 'Create output';

  @override
  String get managedHomeBuildDescription =>
      'Open the honest build view. Voice bundles are available; a complete playable mod is still blocked.';

  @override
  String get managedContentOpenInStory => 'Open in Story';

  @override
  String get managedContentOpenInStoryDescription =>
      'Continue this Quest or NPC in the complete Story workspace.';

  @override
  String get managedContentOpenInStoryRequiresReopen =>
      'Reopen this project before opening Story.';

  @override
  String get managedContentOpenInStoryFailed =>
      'Story could not be opened. The project was not changed.';

  @override
  String get managedStoryWorkbenchActionFailed =>
      'Could not open this editor. Please try again.';

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
  String managedNpcDraftSaved(int projectRevision) {
    return 'Character draft saved in project revision $projectRevision. It remains build-blocked, runtime-unqualified, and is not spawned.';
  }

  @override
  String get managedNpcOpeningRecipeTitle => 'Character + first greeting';

  @override
  String get managedNpcOpeningRecipeDescription =>
      'Recommended: create a project-only Character draft, then write and insert its first localized greeting. This uses two project checkpoints and does not create a playable conversation or spawn.';

  @override
  String get managedNpcOpeningRecipeIntroduction =>
      'This guided flow first saves the Character draft, then opens its first greeting line. If you stop after step 1, the draft stays saved. It does not create dialog logic, runtime behavior, a spawn, or change the game or save files.';

  @override
  String get managedNpcOpeningRecipeStart => 'Start guided Character';

  @override
  String get managedNpcOpeningGreetingTitle => 'Step 2 of 2: First greeting';

  @override
  String get managedNpcOpeningGreetingIntroduction =>
      'Write the first localized greeting line for this Character draft. Saving creates the line and its text, then inserts it at the start of the draft\'s greeting list. It does not add choices, conditions, effects, or playable conversation behavior.';

  @override
  String managedNpcOpeningRecipePartial(int projectRevision) {
    return 'Character draft saved in project revision $projectRevision; no greeting was added. Continue in Story > Dialog & Voice.';
  }

  @override
  String get managedNpcOpeningRecipeFailed =>
      'The guided Character could not be started. The exact project checkpoint is unchanged; game and save files were not changed.';

  @override
  String get managedNpcOpeningRecipeStopped =>
      'The guided flow stopped because its exact project checkpoint or publication could not be verified. No further step will run automatically; inspect Story and continue manually.';

  @override
  String get managedNpcOpeningRecipeRequiresReopen =>
      'The guided flow could not safely continue. Reopen this project and inspect Story before retrying or continuing manually.';

  @override
  String managedNpcOpeningRecipeComplete(int projectRevision) {
    return 'Character draft and first greeting saved in project revision $projectRevision. Draft only: no playable conversation or spawn was created; game and save files were not changed.';
  }

  @override
  String get managedActionNewQuestTitle => 'New Quest';

  @override
  String get managedActionNewQuestDescription =>
      'Create an offline Quest draft with objectives and verified parent identities.';

  @override
  String get managedQuestOpeningRecipeTitle => 'Quest + opening line';

  @override
  String get managedQuestOpeningRecipeDescription =>
      'Recommended: create a Quest draft, then write and insert its first localized line. This uses two project checkpoints and does not create a playable conversation.';

  @override
  String get managedQuestOpeningRecipeIntroduction =>
      'This guided flow first saves the Quest, then opens its first dialog line. If you stop after step 1, the Quest stays saved. It does not create a playable conversation or change the game or save files.';

  @override
  String get managedQuestOpeningRecipeStart => 'Start guided Quest';

  @override
  String get managedQuestOpeningLineTitle => 'Step 2 of 2: Opening dialog line';

  @override
  String get managedQuestOpeningLineIntroduction =>
      'Write the first localized line for this Quest. Saving creates the line and its text, then inserts it at the start of the Quest transcript.';

  @override
  String managedQuestOpeningRecipePreparing(int projectRevision) {
    return 'Quest saved in project revision $projectRevision. Preparing the opening line...';
  }

  @override
  String managedQuestOpeningRecipePartial(int projectRevision) {
    return 'Quest saved in project revision $projectRevision; no opening line was added. Continue in Story > Dialog & Voice.';
  }

  @override
  String get managedQuestOpeningRecipeFailed =>
      'The guided Quest could not start. No project changes were published.';

  @override
  String get managedQuestOpeningRecipeStopped =>
      'The guided flow stopped because the exact current project changed. No further step will run automatically; inspect Story and continue manually.';

  @override
  String get managedQuestOpeningRecipeRequiresReopen =>
      'The guided flow could not safely continue. Reopen this project and inspect Story before retrying or continuing manually.';

  @override
  String managedQuestOpeningRecipeComplete(int projectRevision) {
    return 'Quest and opening line saved in project revision $projectRevision. Draft only: no playable conversation, game, or save files were changed.';
  }

  @override
  String get managedActionNewDialogLineTitle => 'Add dialog line';

  @override
  String get managedActionNewDialogLineDescription =>
      'Write localized project text or connect an unused text already in this project. This does not create a playable dialog topic.';

  @override
  String managedActionNewDialogLineSaved(int projectRevision) {
    return 'Dialog line saved in project revision $projectRevision. The game and save files were not changed.';
  }

  @override
  String get managedDialogLineIntroduction =>
      'Write a new localized dialog line or connect text that already belongs to this project.';

  @override
  String get managedDialogLineBoundary =>
      'Only project files change. This does not create an AngelScript topic or a playable dialog, and it never changes the game installation or save files. The speaker field is only a label; it does not link an NPC.';

  @override
  String get managedDialogLineCreateMode => 'Write new text';

  @override
  String get managedDialogLineReuseMode => 'Use project text';

  @override
  String get managedDialogLineNameLabel => 'Line name';

  @override
  String get managedDialogLineNameHint => 'Mine entrance greeting';

  @override
  String get managedDialogLineSpeakerLabel => 'Speaker label (optional)';

  @override
  String get managedDialogLineSpeakerHint => 'For example, Viper';

  @override
  String get managedDialogLineLocaleLabel => 'Language';

  @override
  String get managedDialogLineTextLabel => 'Dialog text';

  @override
  String get managedDialogLineReuseSearch => 'Search unused project text';

  @override
  String get managedDialogLineNoReusableText =>
      'There is no unused, structurally intact project text to connect. Write new text instead.';

  @override
  String get managedDialogLineCreateSlotLabel =>
      'Prepare this language for Voice';

  @override
  String get managedDialogLineCreateSlotHelp =>
      'Creates an empty unresolved Voice slot in the project. It does not add or deploy a recording.';

  @override
  String get managedDialogLineCancel => 'Cancel';

  @override
  String get managedDialogLineSave => 'Save to project';

  @override
  String get managedDialogLineSaving => 'Saving…';

  @override
  String get managedDialogLineLoading => 'Reading exact project content…';

  @override
  String get managedDialogLineLoadFailed =>
      'The exact current project content could not be read. Nothing was changed.';

  @override
  String get managedDialogLineRetry => 'Retry';

  @override
  String get managedDialogLineStale =>
      'The project changed while this window was open. Close it and try again from the current project.';

  @override
  String get managedDialogLineRequiresReopen =>
      'The current project can no longer be verified safely. Close this window and reopen the managed project.';

  @override
  String get managedDialogLineInvalidInput =>
      'Check the highlighted project input and choose an exact current option.';

  @override
  String get managedDialogLineSaveFailed =>
      'The dialog line could not be saved safely. No game or save files were changed.';

  @override
  String get managedDialogLineDone => 'Done';

  @override
  String get managedDialogLineAddRecording => 'Add recording';

  @override
  String get managedActionAddVoiceTakeTitle => 'Add Voice take';

  @override
  String get managedActionAddVoiceTakeDescription =>
      'Import an Ogg Vorbis recording for an existing project dialog line without deploying it.';

  @override
  String get managedActionAddVoiceTakeRequiresDialogLine =>
      'Create or repair a dialog line with one valid localization entry before using Voice tools.';

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
  String get managedItemsBundledReferenceBadge => 'Bundled reference';

  @override
  String get managedItemsBundledReferenceBoundary =>
      'Read-only reference shipped with Mod Studio. It has not been refreshed or generation-qualified against your configured game installation.';

  @override
  String get managedItemsNoKnownFields =>
      'No modeled scalar fields are available for this item.';

  @override
  String get managedItemsCategorySpecial => 'Special';

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

  @override
  String get managedGlobalSearchScopeLabel => 'Search all';

  @override
  String get managedGlobalSearchTitle => 'Search all content';

  @override
  String get managedGlobalSearchLabel =>
      'NPC, quest, line, asset, ID, or /Game path';

  @override
  String get managedGlobalSearchAction => 'Search';

  @override
  String get managedGlobalSearchClear => 'Clear';

  @override
  String get managedGlobalSearchPrompt =>
      'Enter a search to read the three sources independently.';

  @override
  String get managedGlobalSearchNoResults => 'No matches in this source.';

  @override
  String get managedGlobalSearchLoading => 'Reading exact source…';

  @override
  String get managedGlobalSearchFailed => 'This source could not be read.';

  @override
  String get managedGlobalSearchComplete => 'Complete';

  @override
  String get managedGlobalSearchPartial => 'Partial';

  @override
  String get managedGlobalSearchTruncated =>
      'Showing the first 100 matches. Refine the search.';

  @override
  String get managedGlobalSearchOpen => 'Open';

  @override
  String get managedGlobalSearchCreateDraft => 'Create Draft';

  @override
  String get managedGlobalSearchInspect => 'Inspect';

  @override
  String get managedGlobalSearchKindModEntity => 'Mod content';

  @override
  String get managedGlobalSearchKindModAsset => 'Mod asset';

  @override
  String get managedGlobalSearchKindBaseNpc => 'NPC starting point';

  @override
  String get managedGlobalSearchKindBaseQuest => 'Quest starting point';

  @override
  String get managedGlobalSearchKindExperimentalNpc => 'NPC evidence';

  @override
  String get managedGlobalSearchReadinessExact => 'Exact current project';

  @override
  String get managedGlobalSearchReadinessProblems => 'Exact, with problems';

  @override
  String get managedGlobalSearchResultStale =>
      'This result is no longer in the current project. Search again.';

  @override
  String get managedStoryWorkbenchDraftBadge => 'Draft only';

  @override
  String get managedStoryWorkbenchBuildBlockedBadge => 'Build blocked';

  @override
  String get managedStoryWorkbenchRuntimeUnqualifiedBadge =>
      'Runtime not verified';

  @override
  String get managedStoryWorkbenchOverviewTab => 'Journey';

  @override
  String get managedStoryWorkbenchProfileTab => 'Profile';

  @override
  String get managedStoryWorkbenchStoryTab => 'Story';

  @override
  String get managedStoryWorkbenchLogicTab => 'Logic';

  @override
  String get managedStoryWorkbenchRoutineTab => 'Routine';

  @override
  String get managedStoryWorkbenchInventoryTab => 'Inventory';

  @override
  String get managedStoryWorkbenchDialogVoiceTab => 'Dialog & Voice';

  @override
  String get managedStoryWorkbenchReferencesTab => 'References';

  @override
  String get managedStoryWorkbenchProblemsChecksTab => 'Problems & Checks';

  @override
  String get managedStoryWorkbenchEditOverview => 'Edit name & objectives';

  @override
  String get managedStoryWorkbenchEditStory => 'Edit description & connections';

  @override
  String get managedStoryWorkbenchEditLogic => 'Edit states & transitions';

  @override
  String get managedStoryWorkbenchInspectQuest =>
      'Open source & compiler checks';

  @override
  String get managedStoryWorkbenchInspectNpc =>
      'Open profile & compiler checks';

  @override
  String get managedStoryWorkbenchMoreActions => 'More actions';

  @override
  String get managedStoryWorkbenchRemoveDraft => 'Remove draft…';

  @override
  String get managedStoryWorkbenchRemovingDraft => 'Removing draft…';

  @override
  String get managedStoryWorkbenchReviewRemovalBlockers =>
      'Review removal blockers';

  @override
  String get managedStoryWorkbenchCapabilityUnavailable => 'Not modeled yet';

  @override
  String get managedStoryWorkbenchNpcStoryUnavailable =>
      'Quest and story relationships are not modeled for NPC drafts yet.';

  @override
  String get managedStoryWorkbenchNpcRoutineUnavailable =>
      'Routine and world placement are not modeled yet.';

  @override
  String get managedStoryWorkbenchNpcInventoryUnavailable =>
      'Inventory, equipment, and trading are not modeled yet.';

  @override
  String get managedStoryWorkbenchNpcDialogVoiceUnavailable =>
      'Dialog, localization, and voice relationships are not modeled for NPC drafts yet.';

  @override
  String get managedStoryWorkbenchQuestDialogVoiceUnavailable =>
      'Dialog, localization, and voice relationships are not modeled for Quest drafts yet.';

  @override
  String get managedStoryWorkbenchNoReferenceProblems =>
      'No unresolved project references';

  @override
  String managedStoryWorkbenchReferenceProblemCount(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count unresolved project references',
      one: '1 unresolved project reference',
    );
    return '$_temp0';
  }

  @override
  String get managedStoryWorkbenchReferenceScopeNotice =>
      'Reference status only; this is not build or runtime readiness.';

  @override
  String get managedStoryWorkbenchTechnicalDetails => 'Technical details';

  @override
  String get managedStoryWorkbenchQuestKindLabel => 'Quest draft';

  @override
  String get managedStoryWorkbenchNpcKindLabel => 'NPC draft';

  @override
  String get managedStoryWorkbenchQuestTitleLabel => 'Quest title';

  @override
  String get managedStoryWorkbenchTechnicalIdLabel => 'Technical ID';

  @override
  String get managedStoryWorkbenchObjectivesLabel => 'Objectives';

  @override
  String get managedStoryWorkbenchUniqueNameLabel => 'Unique name';

  @override
  String get managedStoryWorkbenchModuleNamespaceLabel => 'Module namespace';

  @override
  String get managedStoryWorkbenchQuestGiverLabel => 'Quest giver';

  @override
  String get managedStoryWorkbenchRuntimeParentLabel => 'Runtime parent';

  @override
  String get managedStoryWorkbenchLogicDescription =>
      'Quest lifecycle states, triggers, conditions, and effects are edited as one exact-current atomic operation.';

  @override
  String get managedStoryWorkbenchOutgoingHeading => 'Outgoing';

  @override
  String get managedStoryWorkbenchNoOutgoingReferences =>
      'No projected references';

  @override
  String get managedStoryWorkbenchIncomingHeading => 'Incoming';

  @override
  String get managedStoryWorkbenchNoIncomingReferences =>
      'No incoming project references';

  @override
  String get managedStoryWorkbenchSemanticIdentityLabel => 'Semantic identity';

  @override
  String get managedStoryWorkbenchOriginLabel => 'Origin';

  @override
  String get managedStoryWorkbenchEntityRevisionLabel => 'Entity revision';

  @override
  String get managedStoryWorkbenchStableIdLabel => 'Stable ID';

  @override
  String get managedStoryWorkbenchReferenceResolvedLabel =>
      'Reference resolved';

  @override
  String get managedStoryWorkbenchReferenceUnresolvedLabel =>
      'Reference unresolved';

  @override
  String get managedProblemsTitle => 'Problems & readiness';

  @override
  String get managedProblemsDescription =>
      'See what needs attention and open the exact affected project content.';

  @override
  String get managedProblemsScopeNotice =>
      'Every status covers only its named scope. A clear reference check does not mean the mod can be built or tested in-game.';

  @override
  String get managedProblemsRefresh => 'Refresh problems';

  @override
  String get managedProblemsPartialTitle => 'Some checks are unavailable';

  @override
  String get managedProblemsDataAssetsUnavailable =>
      'DataAsset edits could not be checked. Other exact project findings are still shown.';

  @override
  String get managedProblemsOverviewHeading => 'Readiness by area';

  @override
  String get managedProblemsSearchLabel => 'Search problems';

  @override
  String get managedProblemsClearSearch => 'Clear problem search';

  @override
  String get managedProblemsListHeading => 'Problems';

  @override
  String get managedProblemsEmptyTitle =>
      'No modeled structural problems found';

  @override
  String get managedProblemsEmptyDescription =>
      'The exact checks currently modeled by Mod Studio found nothing to repair.';

  @override
  String get managedProblemsEmptyBoundary =>
      'Compiler evidence was not evaluated, the full managed build is unavailable, and runtime behavior remains unqualified.';

  @override
  String get managedProblemsFilteredEmptyTitle => 'No matching problems';

  @override
  String get managedProblemsFilteredEmptyDescription =>
      'Change the search or category filter to see other findings.';

  @override
  String get managedProblemsSelectTitle => 'Select a problem';

  @override
  String get managedProblemsSelectDescription =>
      'Choose a finding to see what it means and the safest available next action.';

  @override
  String get managedProblemsDetailHeading => 'Problem details';

  @override
  String get managedProblemsCloseDetail => 'Close problem details';

  @override
  String get managedProblemsCategoryLabel => 'Area';

  @override
  String get managedProblemsSeverityLabel => 'Attention';

  @override
  String get managedProblemsSourceLabel => 'Evidence';

  @override
  String get managedProblemsOpenSourceEntity => 'Open source content';

  @override
  String get managedProblemsOpenReferencedAsset => 'Open referenced asset';

  @override
  String get managedProblemsOpenDataAssetEdits => 'Open DataAsset edits';

  @override
  String get managedProblemsActionFailed =>
      'The exact target could not be opened. Refresh the project problems and try again.';

  @override
  String get managedProblemsActionProgress =>
      'Opening the exact project target';

  @override
  String get managedProblemsCategoryReferences => 'References';

  @override
  String get managedProblemsCategorySetup => 'Setup';

  @override
  String get managedProblemsCategoryDataAssets => 'DataAssets';

  @override
  String get managedProblemsSeverityInformation => 'Information';

  @override
  String get managedProblemsSeverityWarning => 'Needs attention';

  @override
  String get managedProblemsSeverityBlocking => 'Blocks this scope';

  @override
  String get managedProblemsScopeReferencesTitle => 'Reference integrity';

  @override
  String get managedProblemsScopeReferencesDescription =>
      'Checks exact links between current project content and assets.';

  @override
  String get managedProblemsScopeDataAssetsTitle => 'DataAsset edit registry';

  @override
  String get managedProblemsScopeDataAssetsDescription =>
      'Checks whether the exact current list of saved DataAsset edits could be read.';

  @override
  String get managedProblemsScopeGameTitle => 'Game setup';

  @override
  String get managedProblemsScopeGameDescription =>
      'Shows whether a game installation is configured for bounded read-only tools.';

  @override
  String get managedProblemsScopeCompilerTitle => 'Source & compiler evidence';

  @override
  String get managedProblemsScopeCompilerDescription =>
      'Compiler checks run only when you explicitly open and start them for one exact entity.';

  @override
  String get managedProblemsScopeBuildTitle => 'Managed project build';

  @override
  String get managedProblemsScopeBuildDescription =>
      'A complete build path for managed NPC, Quest, dialog, and DataAsset edits is not available yet.';

  @override
  String get managedProblemsScopeRuntimeTitle => 'In-game behavior';

  @override
  String get managedProblemsScopeRuntimeDescription =>
      'No general runtime, save, deployment, or cleanup qualification is claimed.';

  @override
  String get managedProblemsReadinessClear => 'Checked within this scope';

  @override
  String get managedProblemsReadinessIssues => 'Needs attention';

  @override
  String get managedProblemsReadinessUnavailable => 'Check unavailable';

  @override
  String get managedProblemsReadinessNotEvaluated => 'Not evaluated';

  @override
  String get managedProblemsReadinessBlocked => 'Build path unavailable';

  @override
  String get managedProblemsReadinessUnqualified => 'Runtime unqualified';

  @override
  String get managedProblemsEvidenceContent => 'Exact current project content';

  @override
  String get managedProblemsEvidenceDataAssets =>
      'Exact current DataAsset registry';

  @override
  String get managedProblemsEvidenceConfiguration =>
      'Current app configuration';

  @override
  String get managedProblemsEvidenceUnavailable =>
      'Evidence source unavailable';

  @override
  String get managedProblemsEvidenceBoundary => 'Known capability boundary';

  @override
  String get managedProblemsForeignReferenceTitle =>
      'Reference points to another project';

  @override
  String get managedProblemsMissingEntityTitle =>
      'Linked project content is missing';

  @override
  String get managedProblemsEntityKindTitle =>
      'Linked project content has the wrong type';

  @override
  String get managedProblemsMissingAssetTitle =>
      'Linked project file is missing';

  @override
  String get managedProblemsAssetLengthTitle =>
      'Linked project file has an unexpected size';

  @override
  String get managedProblemsAssetTypeTitle =>
      'Linked project file has an unexpected type';

  @override
  String get managedProblemsGameSetupTitle =>
      'Game installation is not configured';

  @override
  String get managedProblemsDataAssetRegistryTitle =>
      'DataAsset edits could not be checked';

  @override
  String get managedProblemsDataAssetOfflineTitle =>
      'DataAsset edit is draft-only';

  @override
  String managedProblemsEntityReferenceDescription(String source) {
    return 'Open $source and repair this exact project-content link.';
  }

  @override
  String managedProblemsAssetReferenceDescription(String source) {
    return 'Open $source and repair this exact project-file link.';
  }

  @override
  String get managedProblemsDataAssetRegistryDescription =>
      'Refresh the exact current project. No conclusion is drawn about saved DataAsset edits until this source is available.';

  @override
  String managedProblemsDataAssetOfflineDescription(String targetPath) {
    return 'The saved edit for $targetPath can be reviewed in DataAsset edits, but it cannot be emitted by a managed project build or claimed as working in-game yet.';
  }

  @override
  String get projectExportActionTitle => 'Create project backup…';

  @override
  String get projectExportActionDescription =>
      'Write an exact restorable backup of the current saved project checkpoint.';

  @override
  String get projectExportActionDirtyBlocked =>
      'Save or discard the open localization edits before creating a project backup.';

  @override
  String get projectExportDialogTitle => 'Create project backup';

  @override
  String get projectExportPortableCopyTitle =>
      'Restorable Mod Studio project backup';

  @override
  String get projectExportPortableCopyDescription =>
      'This writes the exact current saved project checkpoint to a new .goremod file. It can be restored into a new project folder later; the open project stays current and unchanged.';

  @override
  String get projectExportCapabilityBoundary =>
      'This backup is not a playable mod, build, deployment, or runtime qualification. Creating it does not read or change the game or any save.';

  @override
  String get projectExportKeepOriginal =>
      'A restore preserves this project\'s identity and history. Use Clone or Save As for a separate project identity when those workflows become available.';

  @override
  String get projectExportFileNameLabel => 'New project-backup file';

  @override
  String get projectExportFileNameHelper =>
      'Use a new backup file name ending in .goremod.';

  @override
  String get projectExportChooseDestination => 'Choose destination folder';

  @override
  String get projectExportNoDestination => 'No destination folder selected';

  @override
  String get projectExportNewFile => 'New file';

  @override
  String get projectExportCancel => 'Cancel';

  @override
  String get projectExportClose => 'Close';

  @override
  String get projectExportSubmit => 'Create backup';

  @override
  String get projectExportExporting => 'Creating backup…';

  @override
  String get projectExportParentRequired =>
      'Choose an existing destination folder.';

  @override
  String get projectExportParentAbsolute =>
      'Choose an absolute existing destination folder.';

  @override
  String get projectExportParentLink =>
      'The selected destination is a link. Choose a real existing folder.';

  @override
  String get projectExportParentInspectFailed =>
      'The destination folder could not be inspected safely. Nothing was created.';

  @override
  String get projectExportFileNameRequired =>
      'Enter a new project-backup file name.';

  @override
  String get projectExportFileNameTooLong =>
      'The file name must be at most 128 ASCII characters.';

  @override
  String get projectExportFileNameInvalid =>
      'Start with a letter or digit, use only ASCII letters, digits, dots, underscores, or hyphens, and end with .goremod.';

  @override
  String get projectExportFileNameReserved =>
      'That file name is reserved by Windows.';

  @override
  String get projectExportOutputExists =>
      'That file already exists. Choose a new file name; existing files are never overwritten.';

  @override
  String get projectExportOutputLink =>
      'The new file path is a link. Choose a different file name.';

  @override
  String get projectExportOutputRejected =>
      'The destination was rejected before the new local file was created. Nothing was created. Choose a different file name or destination folder.';

  @override
  String get projectExportStale =>
      'The project changed before backup creation started. No output was created. Close this window and open Create project backup again.';

  @override
  String get projectExportRequiresReopen =>
      'This project can no longer be verified as current. No output was created. Close this window and recover or reopen the project.';

  @override
  String get projectExportUnsupported =>
      'This managed project session cannot create exact restorable backups. Nothing was created.';

  @override
  String get projectExportFailedBeforeStart =>
      'The project backup could not be prepared exactly. Nothing was created.';

  @override
  String get projectExportPrepublicationFailed =>
      'Backup creation stopped safely before the new local file was created. Nothing was created. Close this window and check the project and destination before trying again.';

  @override
  String projectExportMayExist(String output) {
    return 'Backup creation did not return a verified receipt. Do not retry. Close this window and check the destination: $output';
  }

  @override
  String projectExportResultMismatch(String output) {
    return 'The completed backup does not match this checkpoint or destination. Do not retry; inspect the destination: $output';
  }

  @override
  String get projectExportPublished =>
      'The exact restorable project backup was created as a new local file.';

  @override
  String get projectExportPublishedCleanupWarning =>
      'The exact restorable project backup was created as a local file, but internal temporary-file cleanup was incomplete. The created file is valid; do not retry.';

  @override
  String projectExportPublicationUncertain(String output) {
    return 'The local file may have been created. Do not retry. Check whether this destination exists: $output';
  }

  @override
  String get projectExportArchiveBytes => 'Archive bytes';

  @override
  String get projectExportArchiveSha256 => 'Archive SHA-256';

  @override
  String get projectExportCurrentProjectUnchanged =>
      'The current project remains open and unchanged. The game and saves were not touched.';

  @override
  String get projectRestoreActionTitle => 'Restore project backup…';

  @override
  String get projectRestoreActionDescription =>
      'Verify an exact .goremod backup, restore it into a new folder, and open that project safely.';

  @override
  String get projectRestoreDialogTitle => 'Restore project backup';

  @override
  String get projectRestoreNoticeTitle => 'Restore into a new project folder';

  @override
  String get projectRestoreNoticeDescription =>
      'Choose a restorable Mod Studio .goremod backup. Studio verifies the complete archive before creating a new project folder and preserves the backed-up project identity and history.';

  @override
  String get projectRestoreCapabilityBoundary =>
      'Restore does not build, deploy, launch, or qualify the mod at runtime. It does not read or change the game or any save.';

  @override
  String get projectRestoreChooseBackup => 'Choose backup file';

  @override
  String get projectRestoreNoBackup => 'No verified backup selected';

  @override
  String get projectRestoreInspecting => 'Verifying backup…';

  @override
  String get projectRestoreVerified =>
      'This exact V2 project backup is complete and restorable.';

  @override
  String get projectRestoreSource => 'Backup file';

  @override
  String get projectRestoreProjectRevision => 'Project revision';

  @override
  String get projectRestoreArchiveBytes => 'Archive bytes';

  @override
  String get projectRestoreStoreObjects => 'Stored project objects';

  @override
  String get projectRestoreInvalidSource =>
      'The selected file is not a valid exact project backup. Nothing was created.';

  @override
  String get projectRestoreInspectionFailed =>
      'The backup could not be verified completely. Nothing was created.';

  @override
  String get projectRestoreUnavailable =>
      'Exact project restore is unavailable on this system. Nothing was created.';

  @override
  String get projectRestoreChooseDestinationParent => 'Choose parent folder';

  @override
  String get projectRestoreNoDestinationParent => 'No parent folder selected';

  @override
  String get projectRestoreFolderNameLabel => 'New project folder name';

  @override
  String get projectRestoreFolderNameHelper =>
      'Studio creates this new folder; it must not already exist.';

  @override
  String get projectRestoreNewFolder => 'New project folder';

  @override
  String get projectRestoreFolderNameRequired =>
      'Enter a new project folder name.';

  @override
  String get projectRestoreFolderNameTooLong => 'The folder name is too long.';

  @override
  String get projectRestoreFolderNameInvalid =>
      'Use one ordinary folder name without path separators, control characters, a trailing dot, or a trailing space.';

  @override
  String get projectRestoreFolderNameReserved =>
      'That folder name is reserved by Windows.';

  @override
  String get projectRestoreDestinationExists =>
      'That destination already exists. Choose a new folder name; existing content is never overwritten.';

  @override
  String get projectRestoreDestinationLink =>
      'The new project destination is a link. Choose a different folder name.';

  @override
  String get projectRestoreDestinationInvalid =>
      'The destination was rejected before a project receipt was created. Nothing was opened. Choose a different new folder after verifying the backup again.';

  @override
  String get projectRestoreInspectionExpired =>
      'The backup changed after verification. Nothing was opened. Verify the backup again before choosing another destination.';

  @override
  String get projectRestoreMaterializationFailed =>
      'Restore did not return a verified project receipt. Nothing was opened. Do not reuse this attempt; inspect the chosen destination before starting again.';

  @override
  String projectRestorePublicationUncertain(String destination) {
    return 'Studio cannot prove whether the project folder ‘$destination’ was published. Nothing was opened. Do not retry this restore; inspect that destination first.';
  }

  @override
  String get projectRestoreStale =>
      'This restore window is no longer current. Nothing was opened. If materialization had started, inspect the chosen destination before trying anything else.';

  @override
  String get projectRestoreCancel => 'Cancel';

  @override
  String get projectRestoreClose => 'Close';

  @override
  String get projectRestoreSubmit => 'Restore and open';

  @override
  String get projectRestoreRestoring => 'Restoring…';

  @override
  String get projectRestoreSucceeded =>
      'The exact project backup was restored into the new folder.';

  @override
  String get projectRestoreSucceededCleanupWarning =>
      'The exact project backup was restored, but private temporary cleanup was incomplete. The restored project is valid; do not repeat the restore.';

  @override
  String get projectRestoreOpened => 'Project backup restored and opened.';

  @override
  String get projectRestoreOpenedCleanupWarning =>
      'Project backup restored and opened. Private temporary cleanup was incomplete; do not repeat the restore.';

  @override
  String get projectRestoreOpening => 'Opening the restored project safely…';

  @override
  String projectRestoreOpenFailed(String destination) {
    return 'The project folder ‘$destination’ was restored, but Studio could not prove it safe to open. Any previously open project remains current; otherwise no project was opened. Do not repeat the restore; inspect or open the restored folder separately.';
  }

  @override
  String get projectRestoreCandidateCleanupWarning =>
      'No project was adopted. Studio could not completely clean up the rejected candidate session. Restart Mod Studio before opening the restored destination manually.';

  @override
  String get managedVoiceTakeRemoveAction => 'Remove from this line…';

  @override
  String get managedVoiceTakeRemoveTooltip =>
      'Remove this recording from the current dialog line and language';

  @override
  String get managedVoiceTakeRemoveDialogTitle => 'Remove Voice take?';

  @override
  String managedVoiceTakeRemoveDialogSummary(
    String take,
    String line,
    String locale,
  ) {
    return 'Remove “$take” from $line ($locale)?';
  }

  @override
  String get managedVoiceTakeRemoveScope =>
      'Only the link for this dialog line and language is removed. Other project uses remain unchanged.';

  @override
  String get managedVoiceTakeRemoveInternalRetention =>
      'The audio file remains stored internally. This action does not free project storage and has no undo yet.';

  @override
  String get managedVoiceTakeRemoveGameBoundary =>
      'The game installation and save games are not changed.';

  @override
  String get managedVoiceTakeRemoveSelectedWarning =>
      'This is the active take. Removing it also clears the selection atomically. No replacement is chosen automatically, so Voice build remains blocked until an Approved take is selected.';

  @override
  String get managedVoiceTakeRemoveCancel => 'Cancel';

  @override
  String get managedVoiceTakeRemoveConfirm => 'Remove from line';

  @override
  String get managedVoiceTakeRemoveUniqueSuccess =>
      'The take was removed from this line and from the current project graph. Its internal audio data remains retained.';

  @override
  String get managedVoiceTakeRemoveSharedSuccess =>
      'The link was removed from this line and language. The take remains available to its other project uses, and its internal audio data remains retained.';

  @override
  String get managedVoiceTakeRemoveSelectionClearedSuccess =>
      'The active selection was cleared atomically. No replacement was selected; Voice build is blocked until an Approved take is selected.';

  @override
  String get managedVoiceTakeRemoveStale =>
      'The project changed before the take could be removed. Reload the latest Voice takes and review the action again.';

  @override
  String get managedVoiceTakeRemoveRequiresReopen =>
      'The removal result could not be confirmed. Do not retry. Close this window and reopen or recover the managed project.';

  @override
  String get managedVoiceTakeRemoveSavedUnconfirmed =>
      'The removal was saved, but the latest project could not be confirmed. Do not repeat the removal. Close this window and reopen or recover the managed project.';

  @override
  String get managedVoiceTakeRemoveSavedReloadFailed =>
      'The removal was saved, but the latest Voice takes could not be loaded. Reload the takes; the removal will not be repeated.';

  @override
  String managedVoiceTakeRemoveFailed(String error) {
    return 'The take was not removed: $error';
  }

  @override
  String get managedVoiceTakeRemoveReloadConfirmed =>
      'The saved removal was confirmed from the latest project.';

  @override
  String get managedVoiceSlotRemoveAction => 'Remove empty Voice setup…';

  @override
  String get managedVoiceSlotRemoveDialogTitle => 'Remove empty Voice setup?';

  @override
  String managedVoiceSlotRemoveDialogSummary(String line, String locale) {
    return 'Remove the empty $locale Voice setup from $line?';
  }

  @override
  String get managedVoiceSlotRemoveRetention =>
      'The dialog text stays in the project. No recording, audio blob, game file, or save is deleted.';

  @override
  String get managedVoiceSlotRemoveTargetWarning =>
      'This also removes the stored installed-target evidence for this line and language. The installed archive itself remains untouched.';

  @override
  String get managedVoiceSlotRemoveRecreate =>
      'You can add a new take later; the required Voice setup will then be created again automatically.';

  @override
  String get managedVoiceSlotRemoveCancel => 'Keep setup';

  @override
  String get managedVoiceSlotRemoveConfirm => 'Remove setup';

  @override
  String get managedVoiceSlotRemoveSuccess =>
      'Empty Voice setup removed. The dialog text, audio storage, game files, and saves were not changed.';

  @override
  String get managedVoiceSlotPlanSuccess =>
      'Recording planned. An empty Voice setup was added for this line and language. No audio, game file, or save was changed; build and runtime remain unqualified.';

  @override
  String get managedVoiceSlotRemoveStale =>
      'The project changed before the empty Voice setup could be removed. Reload the latest Voice takes and try again.';

  @override
  String get managedVoiceSlotRemoveRequiresReopen =>
      'Reopen the managed project before removing this Voice setup.';

  @override
  String get managedVoiceSlotRemoveSavedUnconfirmed =>
      'The result could not be confirmed and the empty Voice setup may have been saved. Do not repeat the removal. Close this window, reopen the managed project, and inspect the line.';

  @override
  String get managedVoiceSlotRemoveSavedReloadFailed =>
      'The empty Voice setup was saved, but reloading failed. Reload to confirm it; the removal will not be repeated.';

  @override
  String managedVoiceSlotRemoveFailed(String error) {
    return 'The empty Voice setup could not be removed: $error';
  }

  @override
  String get managedVoiceSlotRemoveReloadConfirmed =>
      'Saved empty Voice setup removal confirmed from the latest project.';

  @override
  String get managedVoicePreviewTooltip => 'Preview selected local Ogg';

  @override
  String get managedVoicePreviewOpened =>
      'Opened the selected local recording for author preview. This does not approve or qualify the audio for the game.';

  @override
  String managedVoicePreviewFailed(String error) {
    return 'The local recording preview could not be opened: $error';
  }

  @override
  String get managedStoryWorkbenchEditNpcProfile => 'Edit name & archetype';

  @override
  String get managedStoryWorkbenchNpcDialogVoiceNextStepTitle =>
      'Next step: Dialog & Voice';

  @override
  String get managedStoryWorkbenchNpcDialogVoiceNextStepDescription =>
      'Draft only: continue with greeting lines, text, and voice. This only links project content; it does not create playable dialog or verify runtime behavior.';

  @override
  String get managedStoryWorkbenchContinueToNpcDialogVoice =>
      'Continue to Dialog & Voice';

  @override
  String get managedStoryWorkbenchNpcDisplayNameLabel => 'Character name';

  @override
  String get managedNpcProfileEditTitle => 'Edit name & archetype';

  @override
  String get managedNpcProfileEditDescription =>
      'Change the friendly character name or choose another verified structural starting point.';

  @override
  String get managedNpcProfileEditNameLabel => 'Character name';

  @override
  String get managedNpcProfileEditNameHint =>
      'Shown to authors in this project.';

  @override
  String get managedNpcProfileEditArchetypeLabel =>
      'Archetype / base character';

  @override
  String get managedNpcProfileEditArchetypeHelp =>
      'This does not edit appearance, stats, faction, routine, inventory, dialog, or spawn.';

  @override
  String get managedNpcProfileEditBoundary =>
      'Only the offline project draft changes. The game installation and save games remain unchanged.';

  @override
  String get managedNpcProfileEditLoading => 'Loading current NPC details…';

  @override
  String get managedNpcProfileEditCancel => 'Cancel';

  @override
  String get managedNpcProfileEditClose => 'Close';

  @override
  String get managedNpcProfileEditSave => 'Save changes';

  @override
  String get managedNpcProfileEditSaving => 'Saving…';

  @override
  String get managedNpcProfileEditRetry => 'Retry';

  @override
  String get managedNpcProfileEditLoadFailed =>
      'NPC details and verified archetypes could not be loaded. No files were changed.';

  @override
  String get managedNpcProfileEditCatalogChanged =>
      'The verified archetypes changed while this editor was open. Review and choose the archetype again before saving.';

  @override
  String get managedNpcProfileEditCurrentArchetypeUnavailable =>
      'The current NPC archetype is no longer represented exactly by this game catalog. No replacement was guessed.';

  @override
  String get managedNpcProfileEditStale =>
      'The project changed while this editor was open. Close it and reopen the NPC from the refreshed Story view.';

  @override
  String get managedNpcProfileEditRequiresReopen =>
      'The save result cannot be verified. Do not retry. Close this editor and reopen or recover the managed project.';

  @override
  String get managedNpcProfileEditSaveFailed =>
      'The NPC changes could not be saved safely. Nothing was built, deployed, or written into the game.';

  @override
  String get managedNpcProfileEditNameRequired => 'Enter a character name.';

  @override
  String get managedNpcProfileEditNameTooLong =>
      'The character name must be at most 256 UTF-8 bytes.';

  @override
  String get managedNpcProfileEditNameControl =>
      'The character name contains an unsupported control character.';

  @override
  String get managedNpcProfileEditReviewSelection =>
      'Review and choose an archetype before saving.';

  @override
  String get managedNpcProfileEditDiscardTitle => 'Discard NPC changes?';

  @override
  String get managedNpcProfileEditDiscardBody =>
      'Your unsaved name and archetype choice will be lost.';

  @override
  String get managedNpcProfileEditKeepEditing => 'Keep editing';

  @override
  String get managedNpcProfileEditDiscard => 'Discard';

  @override
  String managedNpcProfileEditSaved(String name, int revision) {
    return '$name was saved in project revision $revision. It remains an offline, build-blocked draft.';
  }

  @override
  String get managedVoiceBuildReadinessTitle => 'Voice readiness';

  @override
  String get managedVoiceBuildReadinessRefresh => 'Refresh Voice readiness';

  @override
  String get managedVoiceBuildReadinessChecking =>
      'Checking exact Voice readiness';

  @override
  String get managedVoiceBuildReadinessLoadError =>
      'Voice readiness could not be verified for the current project. No build is available from this result.';

  @override
  String get managedVoiceBuildReadinessReadyTitle => 'Voice is ready';

  @override
  String get managedVoiceBuildReadinessBlockedTitle => 'Voice needs attention';

  @override
  String managedVoiceBuildReadinessCount(int readySlots, int totalSlots) {
    return '$readySlots of $totalSlots Voice slots are ready.';
  }

  @override
  String get managedVoiceBuildReadinessBlockedBoundary =>
      'No bundle was created and deployment was not performed.';

  @override
  String get managedVoiceBuildReadinessBuildBundle => 'Build bundle';

  @override
  String get managedVoiceBuildReadinessBuildReleaseGuidance =>
      'Voice content is ready. Open Build & Release to create the offline bundle.';

  @override
  String get managedVoiceBuildReadinessConfigureGameGuidance =>
      'Voice content is ready. Configure the game installation before creating an offline bundle.';

  @override
  String get managedVoiceBuildReadinessHideBlockers => 'Hide blockers';

  @override
  String managedVoiceBuildReadinessShowBlockers(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: 'Show $count blockers',
      one: 'Show 1 blocker',
    );
    return '$_temp0';
  }

  @override
  String get managedVoiceBuildReadinessWorkflowFailed =>
      'The selected Voice workflow could not be opened. Refresh and try again.';

  @override
  String get managedVoiceBuildReadinessBuildWorkflowFailed =>
      'The Voice build workflow could not be opened.';

  @override
  String managedVoiceBuildReadinessExactRevision(int revision) {
    return 'Exact project revision $revision';
  }

  @override
  String get managedVoiceBuildReadinessResolveTarget => 'Resolve target';

  @override
  String get managedVoiceBuildReadinessManageTakes => 'Manage takes';

  @override
  String get managedVoiceBuildBlockerNoSlots =>
      'No Voice setups exist in this project.';

  @override
  String get managedVoiceBuildBlockerPayloadBudget =>
      'The selected Voice recordings exceed the safe bundle memory budget.';

  @override
  String get managedVoiceBuildBlockerUnresolvedTarget =>
      'Resolve this Voice target.';

  @override
  String get managedVoiceBuildBlockerAmbiguousTarget =>
      'This Voice target is ambiguous.';

  @override
  String get managedVoiceBuildBlockerUnqualifiedAdd =>
      'This target is not a sealed existing-member replacement.';

  @override
  String get managedVoiceBuildBlockerMissingTake =>
      'Select an approved Voice take.';

  @override
  String get managedVoiceBuildBlockerTakeNotApproved =>
      'The selected Voice take is not approved.';

  @override
  String get managedVoiceBuildBlockerCodecUnqualified =>
      'The selected Voice take uses an unsupported codec.';

  @override
  String get managedVoiceBuildBlockerSlotLimit =>
      'This project exceeds the 1024-slot Voice bundle limit.';

  @override
  String get managedVoiceBuildOfflineNotice =>
      'Offline build only. This creates a sealed existing-member Voice bundle. It does not deploy or write to the game.';

  @override
  String get managedVoiceBuildNewFolderName => 'New folder name';

  @override
  String get managedVoiceBuildNewFolderHelp =>
      'The bundle must be written to a brand-new child folder.';

  @override
  String get managedVoiceBuildChooseParent => 'Choose parent folder';

  @override
  String get managedVoiceBuildNoParentSelected => 'No parent folder selected';

  @override
  String get managedVoiceBuildNewOutput => 'New output';

  @override
  String get managedVoiceBuildOfflineBundle => 'Build offline bundle';

  @override
  String get managedVoiceBuildParentInspectFailed =>
      'The parent folder could not be inspected safely. No build or deployment was attempted.';

  @override
  String get managedVoiceBuildChooseExistingParent =>
      'Choose an existing parent folder.';

  @override
  String get managedVoiceBuildTargetSymlink =>
      'The target path is a symlink. Choose a different new folder name.';

  @override
  String get managedVoiceBuildTargetExists =>
      'The target already exists. Choose a different new folder name.';

  @override
  String get managedVoiceBuildRequiresReopen =>
      'This project can no longer be verified as current. Close this window and reopen the managed project before building another Voice bundle.';

  @override
  String get managedVoiceBuildStaleCheckpoint =>
      'The managed project changed while this window was open. Close this build window and open it again from the current project.';

  @override
  String get managedVoiceBuildFailed =>
      'The Voice bundle could not be built exactly. No deployment was attempted. Before retrying, choose a new folder name if output was created.';

  @override
  String get managedVoiceBuildPlanFailed =>
      'Voice readiness could not be verified for the exact current project. Output selection and build are unavailable until verification succeeds.';

  @override
  String get managedVoiceBuildParentAbsolute =>
      'Choose an absolute existing parent folder.';

  @override
  String get managedVoiceBuildParentSymlink =>
      'The selected parent is a symlink. Choose a real existing folder.';

  @override
  String get managedVoiceBuildFolderRequired => 'Enter a new folder name.';

  @override
  String get managedVoiceBuildFolderWhitespace =>
      'The folder name cannot start or end with whitespace.';

  @override
  String get managedVoiceBuildFolderTooLong => 'The folder name is too long.';

  @override
  String get managedVoiceBuildFolderPortable =>
      'Use one portable folder name without separators or reserved characters.';

  @override
  String get managedVoiceBuildFolderWindowsReserved =>
      'That folder name is reserved by Windows.';

  @override
  String get managedVoiceBuildExecutableUnavailable =>
      'The installed game executable could not be read. Finish any game update and check the configured installation before trying again. No deployment was attempted.';

  @override
  String get managedVoiceBuildExecutableMismatch =>
      'The installed game executable no longer matches this project generation. Re-import or retarget the managed project before building again. No deployment was attempted.';

  @override
  String get managedVoiceBuildGameUnavailable =>
      'The configured Gothic 1 Remake installation is unavailable. Check it in Settings before trying again. No deployment was attempted.';

  @override
  String get managedVoiceBuildStoreGameAlias =>
      'This project folder overlaps the configured game installation. Move the project outside the game folder before building. No deployment was attempted.';

  @override
  String get managedVoiceBuildGameOutputAlias =>
      'The bundle output overlaps a Gothic 1 Remake installation. Choose a parent folder outside every game installation. No deployment was attempted.';

  @override
  String get managedVoiceBuildStoreOutputAlias =>
      'The bundle output overlaps the managed project. Choose a parent folder outside the project. No deployment was attempted.';

  @override
  String get managedVoiceBuildOutputUnavailable =>
      'The selected output parent is unavailable or cannot be traversed safely. Choose a real existing parent folder outside the project and game.';

  @override
  String get managedVoiceBuildOutputFailed =>
      'The new bundle folder could not be written completely. Do not use any output left there; choose a different new folder name before retrying. No deployment was attempted.';

  @override
  String get managedVoiceBuildPromotionFailed =>
      'The sealed bundle could not be promoted into the requested new output folder. A conflicting output was left untouched and owned staging was removed. Choose a different new folder name before retrying. No deployment was attempted.';

  @override
  String get managedVoiceBuildCleanupFailed =>
      'The Voice bundle was not published, but its temporary staging folder could not be removed completely. Remove the reported staging folder before retrying. No deployment was attempted.';

  @override
  String get managedVoiceBuildPublicationUnconfirmed =>
      'The atomic publication may have succeeded, but its final identity or durability could not be confirmed. Do not retry, replace, or delete that exact output yet. Close this window and inspect the reported folder before deciding how to proceed. No deployment was attempted.';

  @override
  String get managedVoiceBuildStoreRootChanged =>
      'The managed project root changed while the bundle was being built. Close this window and reopen the project before building again. No deployment was attempted.';

  @override
  String get managedVoiceBuildGameRootChanged =>
      'The game installation changed while the bundle was being built. Finish the update or file operation, then retry with a new folder name. No deployment was attempted.';

  @override
  String get managedVoiceBuildOutputRootChanged =>
      'The output parent changed while the bundle was being built. Finish the file operation, verify the parent, then retry with a new folder name. No deployment was attempted.';

  @override
  String get managedVoiceBuildVerifyFailed =>
      'The written bundle could not be verified exactly. Do not use that output; choose a different new folder name before retrying. No deployment was attempted.';

  @override
  String get managedVoiceBuildBundleInvalid =>
      'The selected Voice content could not be lowered into one exact sealed bundle. Reopen the project, review its Voice slots, and try again. No deployment was attempted.';

  @override
  String get managedVoiceBuildInputInvalid =>
      'The Voice build request or output path exceeds the safe supported limits. Choose a shorter new output path and try again. No deployment was attempted.';

  @override
  String get managedVoiceBuildResponseLimit =>
      'The bundle was too large to return an exact build receipt. Do not use any unreceipted output; choose a new folder only after reducing the Voice build. No deployment was attempted.';

  @override
  String get managedVoiceBuildBuiltTitle => 'Sealed Voice bundle built';

  @override
  String get managedVoiceBuildOfflineReceipt =>
      'Offline receipt only. Deployment was not performed.';

  @override
  String get managedVoiceBuildBasisRevision => 'Basis project revision';

  @override
  String get managedVoiceBuildOutputLabel => 'Output';

  @override
  String get managedVoiceBuildArchiveEdits => 'Archive edits';

  @override
  String get managedVoiceBuildBundleFiles => 'Bundle files';

  @override
  String get managedVoiceBuildSealedBytes => 'Sealed bytes';

  @override
  String get managedVoiceBuildBundleSha256 => 'Bundle SHA-256';

  @override
  String get managedVoiceBuildParentPickerTitle => 'Choose Voice bundle parent';

  @override
  String managedVoiceBuildBuiltMessage(String output) {
    return 'Sealed Voice bundle built at $output. Deployment was not performed.';
  }

  @override
  String managedVoiceBuildBlockedMessage(int count) {
    return 'Voice build blocked by $count exact requirements. No bundle was created or deployed.';
  }

  @override
  String get managedTextureSetupTitle => 'Choose the game installation';

  @override
  String get managedTextureSetupDescription =>
      'Textures are read from the configured Gothic 1 Remake installation. Nothing is changed in the game or project.';

  @override
  String get managedTextureSetupAction => 'Open Settings';

  @override
  String get managedTextureLoading => 'Loading the installed texture catalog…';

  @override
  String get managedTextureLoadingDescription =>
      'The first exact scan can take several minutes. Mod Studio runs only one scan at a time and queues the latest refresh.';

  @override
  String managedTextureCatalogCount(int count) {
    return '$count installed textures';
  }

  @override
  String managedTextureSearchCount(int matches, int total) {
    return '$matches matches · $total total';
  }

  @override
  String get managedTextureEmptyTitle => 'No textures found';

  @override
  String get managedTextureEmptyDescription =>
      'The exact installed catalog contains no texture entries.';

  @override
  String get managedTextureErrorTitle => 'Texture catalog unavailable';

  @override
  String get managedTextureErrorDescription =>
      'The installed texture catalog could not be loaded for this exact game build.';

  @override
  String get managedTextureRetry => 'Retry';

  @override
  String get managedTextureRefreshTooltip =>
      'Refresh installed texture catalog';

  @override
  String get managedTextureSearchLabel => 'Search textures';

  @override
  String get managedTextureSearchHint => 'Name or Unreal asset path';

  @override
  String get managedTextureClearSearchTooltip => 'Clear texture search';

  @override
  String get managedTextureSelectPrompt =>
      'Select a texture to inspect its original installed image.';

  @override
  String get managedTexturePreviewLoading => 'Extracting the original texture…';

  @override
  String get managedTexturePreviewErrorTitle => 'Preview unavailable';

  @override
  String get managedTexturePreviewErrorDescription =>
      'The original texture could not be extracted from the selected game build.';

  @override
  String get managedTexturePreviewRetry => 'Retry preview';

  @override
  String get managedTextureBackToCatalog => 'Back to textures';

  @override
  String get managedTextureInspectionOnly =>
      'Installed reference · inspect only. This does not edit the project, game installation, or a save.';

  @override
  String get managedTextureInstalledBadge => 'Installed source';

  @override
  String get managedTextureRegularBadge => 'Regular texture';

  @override
  String get managedTextureVirtualBadge => 'Virtual texture';

  @override
  String managedTextureVirtualLayerCount(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count VT layers',
      one: '1 VT layer',
    );
    return '$_temp0';
  }

  @override
  String get managedTextureMipmappedBadge => 'Mipmapped';

  @override
  String get managedTextureSingleMipBadge => 'Single mip';

  @override
  String get managedTextureReplaceableBadge =>
      'Replacement supported · editing not yet available';

  @override
  String get managedTextureNotReplaceableBadge =>
      'Replacement unavailable · inspect only';

  @override
  String get managedTextureUnknownReplaceabilityBadge =>
      'Replacement not qualified · inspect only';

  @override
  String get managedTextureUnknownFormat => 'Unknown source format';

  @override
  String get managedWorkspaceTextVoiceLabel => 'Text & Voice';

  @override
  String get managedWorkspaceTestReleaseLabel => 'Test & Release';

  @override
  String get managedTestReleaseTitle => 'Test & Release';

  @override
  String get managedTestReleaseDescription =>
      'Check every part of your mod before creating playable files or installing them.';

  @override
  String get managedTestReleaseEvidenceBoundary =>
      'Nothing is assumed ready. A checked result applies only to this exact saved project version.';

  @override
  String get managedTestReleaseChecksHeading => 'Project checks';

  @override
  String get managedTestReleaseReleaseHeading => 'Playable output';

  @override
  String get managedTestReleaseStatusNotChecked => 'Not checked';

  @override
  String get managedTestReleaseStatusChecking => 'Checking';

  @override
  String get managedTestReleaseStatusChecked => 'Checked';

  @override
  String get managedTestReleaseStatusNeedsAttention => 'Needs attention';

  @override
  String get managedTestReleaseStatusBlocked => 'Blocked';

  @override
  String get managedTestReleaseStatusNotAvailable => 'Not available';

  @override
  String get managedTestReleaseStatusAvailable => 'Available';

  @override
  String get managedTestReleaseEvidenceLabel => 'Evidence';

  @override
  String get managedTestReleaseStaleEvidenceDescription =>
      'This result belongs to a different project version. Run the check again.';

  @override
  String get managedTestReleaseActionNotConnectedDescription =>
      'Evidence exists, but this action is not connected in the current workspace.';

  @override
  String get managedTestReleaseProblemsHeading => 'Problems to resolve';

  @override
  String get managedTestReleaseVoiceHeading => 'Voice build check';

  @override
  String get managedTestReleaseProjectStructureTitle => 'Project structure';

  @override
  String get managedTestReleaseProjectStructureDescription =>
      'Review the live Problems list below for references and managed-project structure checks.';

  @override
  String get managedTestReleaseProjectStructureAction => 'Review problems';

  @override
  String get managedTestReleaseScriptsTitle => 'Scripts';

  @override
  String get managedTestReleaseScriptsDescription =>
      'A project-wide compiler result is not connected yet. Script checks stay explicitly unevaluated.';

  @override
  String get managedTestReleaseScriptsAction => 'Review scripts';

  @override
  String get managedTestReleaseVoiceTitle => 'Text & Voice';

  @override
  String get managedTestReleaseVoiceDescription =>
      'Use the Voice build check below for the current saved project version.';

  @override
  String get managedTestReleaseVoiceAction => 'Check Voice';

  @override
  String get managedTestReleaseDataAssetsTitle => 'DataAssets';

  @override
  String get managedTestReleaseDataAssetsDescription =>
      'Staged DataAssets are visible in Problems, but no complete project-wide build evidence exists yet.';

  @override
  String get managedTestReleaseDataAssetsAction => 'Review DataAssets';

  @override
  String get managedTestReleasePlayableBuildTitle => 'Playable files';

  @override
  String get managedTestReleasePlayableBuildDescription =>
      'Create a checked playable build from this exact saved project version.';

  @override
  String get managedTestReleasePlayableBuildBlockedReason =>
      'No exact complete project-build evidence exists for this saved version yet.';

  @override
  String get managedTestReleaseCreatePlayableFilesAction =>
      'Create playable files';

  @override
  String get managedTestReleaseDeploymentTitle => 'Installation';

  @override
  String get managedTestReleaseDeploymentDescription =>
      'Install an exactly checked playable build into the configured game.';

  @override
  String get managedTestReleaseDeploymentBlockedReason =>
      'No exact deployable-build evidence exists for this saved project version yet.';

  @override
  String get managedTestReleaseInstallAction => 'Install';

  @override
  String managedProjectCommandBarCurrentSection(String section) {
    return 'Current section: $section';
  }

  @override
  String managedProjectCommandBarOrientationSemantics(
    String project,
    String section,
  ) {
    return 'Project $project. Current section: $section.';
  }

  @override
  String get managedProjectCommandBarUndoLabel => 'Undo';

  @override
  String get managedProjectCommandBarSearchLabel => 'Search';

  @override
  String get managedProjectCommandBarCreateLabel => 'Create';

  @override
  String get managedProjectCommandBarProblemsLabel => 'Problems';

  @override
  String get managedProjectCommandBarHistoryLabel => 'History';

  @override
  String get managedProjectCommandBarSettingsLabel => 'Settings';

  @override
  String get managedProjectCommandBarMoreActionsTooltip =>
      'More project actions';

  @override
  String get managedProjectCommandBarBusyLabel =>
      'Finishing the current project action…';

  @override
  String get managedProjectCommandBarBusyDisabledReason =>
      'Wait for the current project action to finish.';
}
