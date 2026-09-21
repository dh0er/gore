import 'dart:async';
import 'dart:convert';
import 'dart:math' as math;

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:goresave/features/app/domain/ui_settings.dart';
import 'package:goresave/features/app/ui/about_dialog.dart';
import 'package:goresave/features/app/ui/appearance_settings.dart';
import 'package:goresave/features/app/ui/update_settings.dart';
import 'package:goresave/features/app/ui/window_chrome.dart';
import 'package:goresave/features/editor/domain/editor_notifier.dart';
import 'package:goresave/features/editor/domain/editor_models.dart';
import 'package:goresave/features/editor/domain/game_icons.dart';
import 'package:goresave/features/editor/domain/game_time.dart';
import 'package:goresave/features/editor/domain/item_icon_catalog.dart';
import 'package:goresave/features/editor/domain/pending_edits.dart';
import 'package:goresave/features/editor/ui/characters_tab.dart';
import 'package:goresave/features/editor/ui/game_icon.dart';
import 'package:goresave/features/editor/ui/game_time_dialog.dart';
import 'package:goresave/features/editor/ui/overview_statistics_section.dart';
import 'package:goresave/features/editor/ui/profile_localization.dart';
import 'package:goresave/features/editor/ui/slot_repair_banner.dart';
import 'package:goresave/features/editor/ui/title_preparation_progress.dart';
import 'package:goresave/features/localization/domain/localization_controller.dart';
import 'package:goresave/features/localization/ui/localization_settings.dart';
import 'package:goresave/l10n/app_localizations.dart';
import 'package:goresave/loc/loc_catalog_provider.dart';
import 'package:goresave/providers/data_providers.dart';
import 'package:goresave/ui/design/app_theme.dart';
import 'package:intl/intl.dart';
import 'difficulty_dialog.dart';
import 'world_tab.dart';

class EditorPage extends ConsumerStatefulWidget {
  const EditorPage({super.key});

  @override
  ConsumerState<EditorPage> createState() => _EditorPageState();
}

class _EditorPageState extends ConsumerState<EditorPage>
    with WidgetsBindingObserver {
  @override
  void initState() {
    super.initState();
    WidgetsBinding.instance.addObserver(this);
    // Prepare localized text without interrupting startup. A missing source is
    // reported non-modally; selecting an .lcache stays an explicit Settings
    // action.
    WidgetsBinding.instance.addPostFrameCallback((_) {
      unawaited(_prepareLocalizationAutomatically());
      // Covers a page that mounts with a save already inspected (a remount, a
      // hot reload): the listener in build only sees LATER changes.
      if (mounted) ref.read(editorProvider.notifier).prefetchTabData();
    });
  }

  @override
  void dispose() {
    WidgetsBinding.instance.removeObserver(this);
    super.dispose();
  }

  @override
  void didChangeAppLifecycleState(AppLifecycleState state) {
    // Another tool (or `gore-cli loc extract`) may have written the shared
    // loc_catalog.json while this app was backgrounded; reload it on resume so
    // item/NPC names pick up a catalog that appeared after first load. Item
    // images use a metadata-only source check; full PNG verification runs only
    // when the installed container set actually changed.
    if (state == AppLifecycleState.resumed) {
      ref.read(locCatalogReloadProvider.notifier).state++;
      unawaited(
        ref.read(itemIconCatalogRefreshProvider).refreshIfSourceChanged(),
      );
    }
  }

  Future<void> _prepareLocalizationAutomatically() async {
    // Always refresh status first so Settings reflects a catalog created by
    // another GORE tool. A failed status query must not be mistaken for a
    // missing catalog.
    final present = await ref
        .read(localizationControllerProvider.notifier)
        .status();
    // Only extract when the catalog is definitively absent: null means the
    // query itself failed (for example because Core is unavailable).
    if (present != false || !mounted) return;
    final result = await ref
        .read(automaticLocalizationExtractorProvider)
        .extract();
    if (!mounted) return;
    // A catalog can already have been written when only its metadata write
    // failed, so reload after every non-interactive attempt.
    ref.read(locCatalogReloadProvider.notifier).state++;
    if (result.notFound) {
      final store = ref.read(uiSettingsStoreProvider);
      if (store.read().gameDataSourceNoticeShown) return;
      store.write(store.read().copyWith(gameDataSourceNoticeShown: true));
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(
          content: Text(AppLocalizations.of(context).gameDataSourceMissing),
        ),
      );
    }
  }

  @override
  Widget build(BuildContext context) {
    final state = ref.watch(editorProvider);
    final notifier = ref.read(editorProvider.notifier);
    // A save has finished loading and its tabs are now reachable: warm the
    // core's caches for them in the background so the first click on a tab
    // shows data instead of a spinner. Listened to rather than called inline,
    // because the warm-up writes editor state (the hero id the character index
    // settles) and that must not happen during a build.
    ref.listen(editorProvider, (previous, next) => notifier.prefetchTabData());
    final uiScale = ref.watch(uiScaleProvider);
    final zoomPct = (uiScale * 100).round();
    final scheme = Theme.of(context).colorScheme;
    final isDark = Theme.of(context).brightness == Brightness.dark;
    final l10n = AppLocalizations.of(context);

    return Scaffold(
      // The AppBar doubles as the window title bar: dragging the empty space
      // moves the window, double-click toggles maximize/restore.
      appBar: AppBar(
        title: WindowDragArea(
          child: LayoutBuilder(
            builder: (context, constraints) {
              const fixedBrandWidth = 16.0 + 32.0 + 10.0 + 16.0;
              const minimumProgressWidth = 96.0;
              final maximumTitleWidth = math.max(
                0.0,
                math.min(
                  260.0,
                  constraints.maxWidth - fixedBrandWidth - minimumProgressWidth,
                ),
              );
              final titlePainter = TextPainter(
                text: TextSpan(
                  text: l10n.appTitle,
                  style: DefaultTextStyle.of(context).style,
                ),
                maxLines: 1,
                textDirection: Directionality.of(context),
                textScaler: MediaQuery.textScalerOf(context),
              )..layout(maxWidth: 260);
              final titleWidth = math.min(
                titlePainter.width,
                maximumTitleWidth,
              );
              titlePainter.dispose();

              return Row(
                children: [
                  const SizedBox(width: 16),
                  SizedBox.square(
                    dimension: 32,
                    child: Image.asset(
                      'assets/goresave_icon.png',
                      fit: BoxFit.contain,
                      semanticLabel: l10n.appLogoSemanticLabel,
                    ),
                  ),
                  const SizedBox(width: 10),
                  // Measure the title instead of giving it a flex lane: it
                  // stays content-sized on wide windows, but yields space to
                  // preparation progress when UI scaling narrows the AppBar.
                  SizedBox(
                    key: const ValueKey('title-brand'),
                    width: titleWidth,
                    child: Text(
                      l10n.appTitle,
                      maxLines: 1,
                      overflow: TextOverflow.ellipsis,
                    ),
                  ),
                  const SizedBox(width: 16),
                  const Expanded(
                    key: ValueKey('title-progress-available-space'),
                    child: Align(
                      alignment: Alignment.center,
                      child: TitlePreparationProgress(),
                    ),
                  ),
                ],
              );
            },
          ),
        ),
        titleSpacing: 0,
        centerTitle: false,
        scrolledUnderElevation: 0,
        surfaceTintColor: Colors.transparent,
        actions: [
          const SizedBox(width: 8),
          Tooltip(
            message: l10n.zoomTooltip,
            child: Container(
              padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 4),
              decoration: BoxDecoration(
                color: scheme.surfaceContainerHighest,
                borderRadius: BorderRadius.circular(16),
              ),
              child: Row(
                mainAxisSize: MainAxisSize.min,
                children: [
                  const Icon(Icons.zoom_in, size: 18),
                  const SizedBox(width: 3),
                  Text(
                    '$zoomPct%',
                    style: Theme.of(context).textTheme.bodySmall,
                  ),
                ],
              ),
            ),
          ),
          const SizedBox(width: 8),
          IconButton(
            icon: Icon(isDark ? Icons.light_mode : Icons.dark_mode),
            onPressed: () {
              ref
                  .read(themeModeProvider.notifier)
                  .setThemeMode(isDark ? ThemeMode.light : ThemeMode.dark);
            },
            tooltip: isDark ? l10n.switchToLightMode : l10n.switchToDarkMode,
          ),
          IconButton(
            icon: const Icon(Icons.info_outline),
            onPressed: () {
              showDialog(
                context: context,
                builder: (_) => const GoresaveAboutDialog(),
              );
            },
            tooltip: l10n.about,
          ),
          const SizedBox(width: 16),
          const WindowControls(),
        ],
      ),
      body: Column(
        children: [
          Expanded(
            child: Row(
              children: [
                SizedBox(
                  // Narrow enough that long save names ("…, Tag 4, 08:59")
                  // wrap before "Tag" (not earlier), keeping day+time
                  // together on line two: 380 kept "Tag" on line one, 350
                  // pushed "Verurteilten" down too.
                  width: 365,
                  child: _SaveSidebar(state: state, notifier: notifier),
                ),
                const VerticalDivider(width: 1),
                Expanded(
                  child: _EditorWorkspace(state: state, notifier: notifier),
                ),
              ],
            ),
          ),
        ],
      ),
    );
  }
}

class _SaveSidebar extends StatelessWidget {
  const _SaveSidebar({required this.state, required this.notifier});

  final EditorState state;
  final EditorNotifier notifier;

  @override
  Widget build(BuildContext context) {
    // Use the notifier-computed visible saves so the list, header count, and
    // Quick/Auto stats all agree.
    final saves = state.visibleSaves;
    final l10n = AppLocalizations.of(context);
    return DecoratedBox(
      decoration: BoxDecoration(
        color: Theme.of(context).colorScheme.surfaceContainerLow,
      ),
      child: Column(
        children: [
          _ProfileHeader(
            profile: state.activeProfile,
            profiles: state.profiles,
            otherSavesSelected: state.otherSavesSelected,
            notifier: notifier,
            isLoading: state.isLoading,
            profileWritesBlocked: state.deletedSaveRecovery != null,
          ),
          if (state.otherSavesSelected)
            Padding(
              padding: const EdgeInsets.fromLTRB(8, 8, 8, 0),
              child: SizedBox(
                width: double.infinity,
                child: OutlinedButton.icon(
                  key: const ValueKey('other-saves-open-file'),
                  icon: const Icon(Icons.file_open_outlined),
                  label: Text(l10n.openSaveFile),
                  onPressed: state.isLoading || state.hasUnsavedEdits
                      ? null
                      : notifier.openSaveFile,
                ),
              ),
            ),
          Expanded(
            child: saves.isEmpty
                ? Center(
                    child: Padding(
                      padding: const EdgeInsets.all(24),
                      child: Text(
                        l10n.noSavFilesFound,
                        textAlign: TextAlign.center,
                      ),
                    ),
                  )
                : ListView.separated(
                    padding: const EdgeInsets.symmetric(vertical: 8),
                    itemCount: saves.length,
                    separatorBuilder: (_, _) => const SizedBox(height: 4),
                    itemBuilder: (context, index) {
                      final save = saves[index];
                      final selected = save.path == state.selectedPath;
                      ProfileSummary? assignedProfile;
                      for (final profile in state.profiles) {
                        if (profile.profileId == save.persistentProfileId) {
                          assignedProfile = profile;
                          break;
                        }
                      }
                      return Padding(
                        padding: const EdgeInsets.symmetric(horizontal: 8),
                        child: _SaveSlotCard(
                          save: save,
                          selected: selected,
                          enabled: !state.isLoading && !save.isMissing,
                          onTap: () => notifier.inspect(save.path),
                          onRemoveFromProfile:
                              assignedProfile == null ||
                                  state.isLoading ||
                                  state.deletedSaveRecovery != null ||
                                  state.hasUnsavedEdits
                              ? null
                              : () => _confirmRemoveSaveFromProfile(
                                  context,
                                  save: save,
                                  profile: assignedProfile!,
                                  notifier: notifier,
                                ),
                          onDeleteSave:
                              assignedProfile == null ||
                                  save.isMissing ||
                                  state.isLoading ||
                                  state.deletedSaveRecovery != null ||
                                  state.hasUnsavedEdits
                              ? null
                              : () => _confirmDeleteSave(
                                  context,
                                  save: save,
                                  profile: assignedProfile!,
                                  notifier: notifier,
                                ),
                          onRemoveFromOther:
                              !state.otherSavesSelected ||
                                  state.isLoading ||
                                  state.hasUnsavedEdits
                              ? null
                              : () => notifier.removeOtherSave(save.path),
                          showRemoveFromOther: state.otherSavesSelected,
                        ),
                      );
                    },
                  ),
          ),
        ],
      ),
    );
  }
}

class _ProfileHeader extends StatelessWidget {
  const _ProfileHeader({
    required this.profile,
    required this.profiles,
    required this.otherSavesSelected,
    required this.notifier,
    required this.isLoading,
    required this.profileWritesBlocked,
  });

  final ProfileSummary? profile;
  final List<ProfileSummary> profiles;
  final bool otherSavesSelected;
  final EditorNotifier notifier;
  final bool isLoading;
  final bool profileWritesBlocked;

  @override
  Widget build(BuildContext context) {
    final scheme = Theme.of(context).colorScheme;
    final l10n = AppLocalizations.of(context);
    return Container(
      width: double.infinity,
      // Match the icon+text TabBar row in the workspace next door (72 tab
      // height + 2 indicator weight = 74, measured) so the header's bottom
      // edge lines up with the tab bar's.
      height: 74,
      padding: const EdgeInsets.only(left: 16, right: 4),
      alignment: Alignment.centerLeft,
      decoration: BoxDecoration(
        color: scheme.surfaceContainerLowest,
        border: Border(bottom: BorderSide(color: scheme.outlineVariant)),
      ),
      child: Row(
        children: [
          Container(
            width: 40,
            height: 40,
            decoration: BoxDecoration(
              color: scheme.primaryContainer,
              borderRadius: BorderRadius.circular(8),
            ),
            // The glyph the game marks its own people with.
            child: Center(
              child: GameIcon(
                name: gameIconCharacter,
                fallbackIcon: Icons.person_outline,
                size: 24,
                color: scheme.primary,
              ),
            ),
          ),
          const SizedBox(width: 12),
          Expanded(
            child: Row(
              children: [
                Expanded(
                  child: _ProfileSwitcher(
                    profile: profile,
                    profiles: profiles,
                    otherSavesSelected: otherSavesSelected,
                    notifier: notifier,
                    isLoading: isLoading,
                  ),
                ),
                const SizedBox(width: 8),
                Flexible(
                  child: ProfileDifficultyChip(
                    profile: profile,
                    notifier: notifier,
                    isLoading: isLoading || profileWritesBlocked,
                  ),
                ),
              ],
            ),
          ),
          IconButton(
            icon: const Icon(Icons.refresh),
            tooltip: l10n.rescanSaveFolder,
            visualDensity: VisualDensity.compact,
            iconSize: 20,
            onPressed: isLoading ? null : () => _confirmRefresh(context),
          ),
        ],
      ),
    );
  }

  /// Rescanning re-inspects the selected slot, which clears the global
  /// pending-edit registry (including any pending difficulty edit) and re-seeds
  /// every editor — never silently discard unsaved changes. Guard on the same
  /// `hasUnsavedEdits` signal the profile-switch guard uses (pending registry
  /// edits OR a pending difficulty edit).
  Future<void> _confirmRefresh(BuildContext context) async {
    if (notifier.hasUnsavedEdits) {
      final pendingCount = notifier.pendingEditCount;
      final confirmed = await showDialog<bool>(
        context: context,
        builder: (context) {
          final l10n = AppLocalizations.of(context);
          return AlertDialog(
            title: Text(l10n.discardUnsavedChangesTitle),
            content: Text(l10n.rescanDiscardBody(pendingCount)),
            actions: [
              TextButton(
                onPressed: () => Navigator.of(context).pop(false),
                child: Text(l10n.cancel),
              ),
              FilledButton(
                onPressed: () => Navigator.of(context).pop(true),
                child: Text(l10n.discardAndRescan),
              ),
            ],
          );
        },
      );
      if (confirmed != true) return;
      // The user chose to discard. refresh() centrally clears all pending edits
      // (registry + the pending difficulty edit) and re-seeds the editors.
    }
    await notifier.refresh();
  }
}

/// Profile picker containing only real profiles plus the dedicated persistent
/// Other saves view. File opening lives inside that view's sidebar.
class _ProfileSwitcher extends StatelessWidget {
  const _ProfileSwitcher({
    required this.profile,
    required this.profiles,
    required this.otherSavesSelected,
    required this.notifier,
    required this.isLoading,
  });

  final ProfileSummary? profile;
  final List<ProfileSummary> profiles;
  final bool otherSavesSelected;
  final EditorNotifier notifier;
  final bool isLoading;

  @override
  Widget build(BuildContext context) {
    final textTheme = Theme.of(context).textTheme;
    final scheme = Theme.of(context).colorScheme;
    final currentId = profile?.profileId;
    final l10n = AppLocalizations.of(context);
    return PopupMenuButton<String>(
      tooltip: l10n.switchProfile,
      enabled: !isLoading,
      onSelected: (choice) {
        if (choice == 'other-saves') {
          notifier.selectOtherSaves();
          return;
        }
        final id = int.tryParse(choice.substring('profile:'.length));
        if (id != null) notifier.selectProfile(id);
      },
      itemBuilder: (context) => [
        for (final p in profiles)
          PopupMenuItem<String>(
            value: 'profile:${p.profileId}',
            child: Row(
              children: [
                if (p.profileId == currentId)
                  Icon(Icons.check, size: 18, color: scheme.primary)
                else
                  const SizedBox(width: 18),
                const SizedBox(width: 8),
                Expanded(
                  child: Text(
                    l10n.profileWithSaves(
                      localizedProfileDisplayName(l10n, p),
                      p.savedSlots.length,
                    ),
                    overflow: TextOverflow.ellipsis,
                  ),
                ),
              ],
            ),
          ),
        if (profiles.isNotEmpty) const PopupMenuDivider(),
        PopupMenuItem<String>(
          key: const ValueKey('profile-menu-other-saves'),
          value: 'other-saves',
          child: Row(
            children: [
              if (otherSavesSelected)
                Icon(Icons.check, size: 18, color: scheme.primary)
              else
                const Icon(Icons.folder_copy_outlined, size: 18),
              const SizedBox(width: 8),
              Expanded(child: Text(l10n.otherSaves)),
            ],
          ),
        ),
      ],
      child: Row(
        mainAxisSize: MainAxisSize.min,
        children: [
          Flexible(
            child: Text(
              profile == null
                  ? otherSavesSelected
                        ? l10n.otherSaves
                        : l10n.profile
                  : localizedProfileDisplayName(l10n, profile!),
              maxLines: 1,
              overflow: TextOverflow.ellipsis,
              style: textTheme.titleMedium,
            ),
          ),
          Icon(
            Icons.arrow_drop_down,
            size: 18,
            color: isLoading ? scheme.onSurfaceVariant : scheme.primary,
          ),
        ],
      ),
    );
  }
}

class _SaveSlotCard extends StatelessWidget {
  const _SaveSlotCard({
    required this.save,
    required this.selected,
    required this.enabled,
    required this.onTap,
    required this.showRemoveFromOther,
    this.onRemoveFromProfile,
    this.onDeleteSave,
    this.onRemoveFromOther,
  });

  final SaveSlot save;
  final bool selected;
  final bool enabled;
  final VoidCallback onTap;
  final bool showRemoveFromOther;
  final VoidCallback? onRemoveFromProfile;
  final VoidCallback? onDeleteSave;
  final VoidCallback? onRemoveFromOther;

  @override
  Widget build(BuildContext context) {
    final scheme = Theme.of(context).colorScheme;
    final accent = save.isMissing
        ? scheme.error
        : selected
        ? scheme.primary
        : scheme.outline;
    final l10n = AppLocalizations.of(context);
    return Material(
      color: save.isMissing
          ? scheme.errorContainer.withValues(alpha: 0.35)
          : selected
          ? scheme.primaryContainer
          : scheme.surfaceContainerLowest,
      shape: RoundedRectangleBorder(
        borderRadius: BorderRadius.circular(8),
        side: BorderSide(color: accent),
      ),
      clipBehavior: Clip.antiAlias,
      child: InkWell(
        onTap: enabled ? onTap : null,
        child: Padding(
          padding: const EdgeInsets.all(8),
          child: Row(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              SizedBox(
                width: 124,
                height: 72,
                child: save.isMissing
                    ? Semantics(
                        label: l10n.missingSaveReference,
                        child: DecoratedBox(
                          decoration: BoxDecoration(
                            color: scheme.errorContainer,
                            borderRadius: BorderRadius.circular(6),
                          ),
                          child: Icon(
                            Icons.file_present_outlined,
                            color: scheme.error,
                            size: 34,
                          ),
                        ),
                      )
                    : _ScreenshotPreview(
                        screenshot: save.screenshot,
                        slot: save.slot,
                        compact: true,
                      ),
              ),
              const SizedBox(width: 10),
              Expanded(
                child: Row(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Expanded(
                      child: Column(
                        crossAxisAlignment: CrossAxisAlignment.start,
                        mainAxisSize: MainAxisSize.min,
                        children: [
                          Row(
                            crossAxisAlignment: CrossAxisAlignment.start,
                            children: [
                              Padding(
                                padding: const EdgeInsets.only(top: 2),
                                child: save.isMissing
                                    ? Tooltip(
                                        message: l10n.missingSaveReference,
                                        child: Icon(
                                          Icons.link_off_outlined,
                                          size: 16,
                                          color: scheme.error,
                                        ),
                                      )
                                    : _SaveKindIcon(
                                        quickSave: save.quickSave,
                                        autoSave: save.autoSave,
                                        external: save.isExternal,
                                        selected: selected,
                                      ),
                              ),
                              const SizedBox(width: 6),
                              Expanded(
                                child: Text(
                                  save.displayName,
                                  key: ValueKey('save-title-${save.slot}'),
                                  maxLines: 3,
                                  overflow: TextOverflow.ellipsis,
                                  style: Theme.of(context).textTheme.titleSmall,
                                ),
                              ),
                            ],
                          ),
                          const SizedBox(height: 3),
                          Text(
                            _saveSlotSubtitle(l10n, save),
                            key: ValueKey('save-subtitle-${save.slot}'),
                            maxLines: 2,
                            overflow: TextOverflow.ellipsis,
                            style: Theme.of(context).textTheme.bodySmall
                                ?.copyWith(color: scheme.onSurfaceVariant),
                          ),
                          const SizedBox(height: 2),
                          Text(
                            save.fileName,
                            key: ValueKey('save-file-name-${save.slot}'),
                            maxLines: 1,
                            overflow: TextOverflow.ellipsis,
                            style: Theme.of(context).textTheme.bodySmall
                                ?.copyWith(color: scheme.onSurfaceVariant),
                          ),
                        ],
                      ),
                    ),
                    if (onRemoveFromProfile != null || onDeleteSave != null)
                      SizedBox.square(
                        dimension: 32,
                        child: PopupMenuButton<String>(
                          key: ValueKey(
                            'save-actions-menu-${save.persistentProfileId}-${save.slot}',
                          ),
                          tooltip: MaterialLocalizations.of(
                            context,
                          ).moreButtonTooltip,
                          icon: const Icon(Icons.more_vert),
                          iconSize: 18,
                          padding: EdgeInsets.zero,
                          constraints: const BoxConstraints(
                            minWidth: 320,
                            maxWidth: 360,
                          ),
                          onSelected: (action) {
                            switch (action) {
                              case 'remove-from-profile':
                                onRemoveFromProfile?.call();
                              case 'delete-save':
                                onDeleteSave?.call();
                            }
                          },
                          itemBuilder: (context) => [
                            if (onRemoveFromProfile != null)
                              PopupMenuItem<String>(
                                key: ValueKey(
                                  'remove-save-profile-${save.persistentProfileId}-${save.slot}',
                                ),
                                value: 'remove-from-profile',
                                child: Row(
                                  children: [
                                    Icon(
                                      Icons.link_off_outlined,
                                      size: 18,
                                      color: save.isMissing
                                          ? scheme.error
                                          : scheme.onSurfaceVariant,
                                    ),
                                    const SizedBox(width: 10),
                                    Text(l10n.removeFromProfile),
                                  ],
                                ),
                              ),
                            if (onDeleteSave != null)
                              PopupMenuItem<String>(
                                key: ValueKey(
                                  'delete-save-${save.persistentProfileId}-${save.slot}',
                                ),
                                value: 'delete-save',
                                child: Row(
                                  children: [
                                    Icon(
                                      Icons.delete_forever_outlined,
                                      size: 18,
                                      color: scheme.error,
                                    ),
                                    const SizedBox(width: 10),
                                    Text(
                                      l10n.deleteSavegame,
                                      style: TextStyle(color: scheme.error),
                                    ),
                                  ],
                                ),
                              ),
                          ],
                        ),
                      ),
                    if (showRemoveFromOther)
                      IconButton(
                        key: ValueKey('remove-other-save-${save.path}'),
                        icon: const Icon(Icons.close, size: 18),
                        tooltip: l10n.removeEntry,
                        visualDensity: VisualDensity.compact,
                        constraints: const BoxConstraints(
                          minWidth: 32,
                          minHeight: 32,
                        ),
                        color: scheme.onSurfaceVariant,
                        onPressed: onRemoveFromOther,
                      ),
                  ],
                ),
              ),
            ],
          ),
        ),
      ),
    );
  }
}

class _SaveKindIcon extends StatelessWidget {
  const _SaveKindIcon({
    required this.quickSave,
    required this.autoSave,
    required this.external,
    required this.selected,
  });

  final bool? quickSave;
  final bool? autoSave;
  final bool external;
  final bool selected;

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context);
    final label = external
        ? l10n.externalSave
        : _formatSaveKind(l10n, quickSave: quickSave, autoSave: autoSave);
    if (label == '-') return const SizedBox(height: 16);
    final icon = external
        ? Icons.insert_drive_file_outlined
        : quickSave == true
        ? Icons.flash_on_outlined
        : autoSave == true
        ? Icons.timer_outlined
        : Icons.edit_note_outlined;
    return Tooltip(
      message: label,
      child: Align(
        alignment: Alignment.centerLeft,
        child: Icon(
          icon,
          size: 16,
          color: selected
              ? Theme.of(context).colorScheme.primary
              : Theme.of(context).colorScheme.onSurfaceVariant,
        ),
      ),
    );
  }
}

String _saveSlotSubtitle(AppLocalizations l10n, SaveSlot save) {
  if (save.isMissing) {
    return '${l10n.missingSaveReference}: '
        '${l10n.missingSaveReferenceDescription(save.slot)}';
  }
  final parts = <String>[];
  if (!save.isExternal && save.persistentProfileId == null) {
    parts.add(l10n.unassignedSave);
  }
  if (save.chapterId != null) {
    parts.add(l10n.chapterLabel(save.chapterId!));
  }
  final timePlayed = _formatDurationSeconds(l10n, save.timePlayedSeconds);
  if (timePlayed != '-') {
    parts.add(timePlayed);
  }
  return parts.join(' | ');
}

Future<void> _confirmRemoveSaveFromProfile(
  BuildContext context, {
  required SaveSlot save,
  required ProfileSummary profile,
  required EditorNotifier notifier,
}) async {
  final l10n = AppLocalizations.of(context);
  final confirmed = await showDialog<bool>(
    context: context,
    builder: (context) => AlertDialog(
      title: Text(l10n.removeSaveFromProfileTitle),
      content: Text(
        l10n.removeSaveFromProfileBody(
          save.displayName,
          localizedProfileDisplayName(l10n, profile),
        ),
      ),
      actions: [
        TextButton(
          onPressed: () => Navigator.of(context).pop(false),
          child: Text(l10n.cancel),
        ),
        FilledButton(
          onPressed: () => Navigator.of(context).pop(true),
          child: Text(l10n.removeFromProfile),
        ),
      ],
    ),
  );
  if (confirmed != true) return;
  await notifier.removeSaveFromProfile(
    slot: save.slot,
    profileId: profile.profileId,
  );
}

Future<void> _confirmDeleteSave(
  BuildContext context, {
  required SaveSlot save,
  required ProfileSummary profile,
  required EditorNotifier notifier,
}) async {
  final l10n = AppLocalizations.of(context);
  final confirmed = await showDialog<bool>(
    context: context,
    builder: (context) => AlertDialog(
      title: Text(l10n.deleteSavegameTitle),
      content: Text(
        l10n.deleteSavegameBody(
          save.displayName,
          save.fileName,
          localizedProfileDisplayName(l10n, profile),
        ),
      ),
      actions: [
        TextButton(
          onPressed: () => Navigator.of(context).pop(false),
          child: Text(l10n.cancel),
        ),
        FilledButton(
          style: FilledButton.styleFrom(
            backgroundColor: Theme.of(context).colorScheme.error,
            foregroundColor: Theme.of(context).colorScheme.onError,
          ),
          onPressed: () => Navigator.of(context).pop(true),
          child: Text(l10n.deleteSavegame),
        ),
      ],
    ),
  );
  if (confirmed != true) return;
  await notifier.deleteSave(slot: save.slot, profileId: profile.profileId);
}

String _formatDurationSeconds(AppLocalizations l10n, double? seconds) {
  if (seconds == null || seconds.isNaN || seconds.isInfinite) return '-';
  final totalMinutes = (seconds < 0 ? 0 : seconds / 60).floor();
  final hours = totalMinutes ~/ 60;
  final minutes = totalMinutes % 60;
  if (hours <= 0) return l10n.durationMinutes(minutes);
  if (minutes == 0) return l10n.durationHours(hours);
  return l10n.durationHoursMinutes(hours, minutes);
}

String _formatSaveKind(
  AppLocalizations l10n, {
  required bool? quickSave,
  required bool? autoSave,
}) {
  if (quickSave == true) return l10n.quickSave;
  if (autoSave == true) return l10n.autoSave;
  if (quickSave == false || autoSave == false) return l10n.manualSave;
  return '-';
}

class _EditorWorkspace extends StatelessWidget {
  const _EditorWorkspace({required this.state, required this.notifier});

  final EditorState state;
  final EditorNotifier notifier;

  @override
  Widget build(BuildContext context) {
    final scheme = Theme.of(context).colorScheme;
    final l10n = AppLocalizations.of(context);

    Widget writeMessageBanner() {
      return MaterialBanner(
        leading: const Icon(Icons.check_circle_outline),
        content: Text(state.lastWriteMessage!),
        actions: [
          TextButton(
            onPressed: state.isLoading ? null : notifier.dismissWriteMessage,
            child: Text(l10n.ok),
          ),
        ],
      );
    }

    Widget deletedSaveRecoveryBanner() {
      final recovery = state.deletedSaveRecovery!;
      return MaterialBanner(
        leading: const Icon(Icons.check_circle_outline),
        content: Text(recovery.message),
        actions: [
          TextButton.icon(
            onPressed: state.isLoading || state.hasUnsavedEdits
                ? null
                : () => notifier.restoreDeletedSave(),
            icon: const Icon(Icons.undo),
            label: Text(l10n.restoreBackupTooltip(recovery.fileName)),
          ),
          TextButton(
            onPressed: state.isLoading
                ? null
                : notifier.dismissDeletedSaveRecovery,
            child: Text(l10n.ok),
          ),
        ],
      );
    }

    Widget content;
    if (state.inspection == null) {
      final emptyPane = state.error != null
          ? _MessagePane(
              icon: Icons.error_outline,
              title: l10n.errorTitle,
              body: state.error!,
            )
          : _MessagePane(
              icon: Icons.search,
              title: l10n.selectASaveTitle,
              body: l10n.selectASaveBody,
            );
      final hasBanner =
          state.deletedSaveRecovery != null || state.lastWriteMessage != null;
      content = !hasBanner
          ? emptyPane
          : Column(
              children: [
                if (state.deletedSaveRecovery != null)
                  deletedSaveRecoveryBanner(),
                if (state.lastWriteMessage != null) writeMessageBanner(),
                Expanded(child: emptyPane),
              ],
            );
    } else {
      final inspection = state.inspection!;
      final pendingCount = state.pendingEditCount;
      content = DefaultTabController(
        length: 6,
        child: Column(
          children: [
            Container(
              color: scheme.surfaceContainerLowest,
              child: Row(
                children: [
                  Expanded(
                    child: TabBar(
                      isScrollable: true,
                      // Material 3 defaults a scrollable TabBar to
                      // TabAlignment.startOffset, which inserts a ~52px empty gap
                      // before the first (Overview) tab. Pin to the start so the
                      // tab row begins flush-left with no wasted leading padding.
                      tabAlignment: TabAlignment.start,
                      tabs: [
                        Tab(
                          icon: const Icon(Icons.dashboard_outlined),
                          text: l10n.tabOverview,
                        ),
                        Tab(
                          icon: const Icon(Icons.people_outline),
                          text: l10n.tabCharacters,
                        ),
                        Tab(
                          icon: const Icon(Icons.public),
                          text: l10n.tabWorld,
                        ),
                        Tab(
                          icon: const Icon(Icons.tune),
                          text: l10n.tabAllData,
                        ),
                        Tab(
                          icon: const Icon(Icons.history),
                          text: l10n.tabBackups,
                        ),
                        Tab(
                          icon: const Icon(Icons.settings_outlined),
                          text: l10n.tabSettings,
                        ),
                      ],
                    ),
                  ),
                  Padding(
                    padding: const EdgeInsets.only(right: 8),
                    child: OutlinedButton.icon(
                      icon: const Icon(Icons.undo),
                      label: Text(l10n.reset),
                      onPressed: pendingCount > 0 && !state.isLoading
                          ? notifier.refresh
                          : null,
                    ),
                  ),
                  Padding(
                    padding: const EdgeInsets.only(right: 12),
                    child: FilledButton.icon(
                      icon: const Icon(Icons.save_outlined),
                      label: Text(
                        pendingCount == 0
                            ? l10n.save
                            : l10n.saveWithCount(pendingCount),
                      ),
                      onPressed:
                          pendingCount > 0 &&
                              !state.isLoading &&
                              (!state.pendingEditsChangePersistentDataList ||
                                  state.deletedSaveRecovery == null) &&
                              !state.hasInvalidEdits
                          ? notifier.saveAllPending
                          : null,
                    ),
                  ),
                ],
              ),
            ),
            if (state.error != null)
              MaterialBanner(
                backgroundColor: scheme.errorContainer,
                leading: Icon(Icons.error_outline, color: scheme.error),
                content: Text(state.error!),
                actions: [
                  TextButton(
                    onPressed: notifier.dismissError,
                    child: Text(l10n.ok),
                  ),
                ],
              ),
            if (state.deletedSaveRecovery != null) deletedSaveRecoveryBanner(),
            if (state.lastWriteMessage != null) writeMessageBanner(),
            Expanded(
              child: TabBarView(
                children: [
                  _KeepAliveTab(
                    child: _OverviewPanel(
                      inspection: inspection,
                      notifier: notifier,
                      state: state,
                    ),
                  ),
                  _KeepAliveTab(
                    child: CharactersTab(
                      inspection: inspection,
                      notifier: notifier,
                      // Private writes recompress the payload, so also require the
                      // codec to be compress-ready, not just decode-ready — same
                      // gating the old Attribute tab used.
                      attributeEditable:
                          inspection.privateEditable &&
                          state.codecCompressReady,
                      // Same value the old Inventory tab received for canCompress.
                      inventoryCanCompress: state.codecCompressReady,
                      // Same gating the World tab uses for its quests/factions
                      // detail panels.
                      progressionEditable:
                          inspection.privateEditable &&
                          inspection.privateTypedVerified &&
                          state.codecCompressReady,
                    ),
                  ),
                  _KeepAliveTab(
                    child: WorldTab(
                      inspection: inspection,
                      notifier: notifier,
                      editable:
                          inspection.privateEditable &&
                          inspection.privateTypedVerified &&
                          state.codecCompressReady,
                    ),
                  ),
                  _KeepAliveTab(
                    child: _AllDataPanel(
                      inspection: inspection,
                      notifier: notifier,
                      // Typed writes recompress the private payload, so require a
                      // full private decode (not a preview) plus a compress-ready
                      // codec, matching the Player and Inventory gating.
                      editable:
                          inspection.privateEditable &&
                          state.codecCompressReady,
                    ),
                  ),
                  _KeepAliveTab(
                    child: _BackupsPanel(state: state, notifier: notifier),
                  ),
                  _KeepAliveTab(
                    child: _SettingsPanel(state: state, notifier: notifier),
                  ),
                ],
              ),
            ),
          ],
        ),
      );
    }

    return Stack(
      children: [
        content,
        if (state.isLoading)
          Positioned.fill(
            child: ColoredBox(
              color: scheme.surface.withValues(alpha: 0.6),
              child: Center(
                child: Semantics(
                  label: l10n.loadingEditorData,
                  // A multi-step save reports (done, total): show a determinate
                  // bar with the count so sequential writes read as progress, not
                  // a hung spinner. Any other load keeps the plain spinner.
                  child: state.saveProgress != null
                      ? SizedBox(
                          width: 240,
                          child: Column(
                            mainAxisSize: MainAxisSize.min,
                            children: [
                              LinearProgressIndicator(
                                value: state.saveProgress!.total == 0
                                    ? null
                                    : state.saveProgress!.done /
                                          state.saveProgress!.total,
                              ),
                              const SizedBox(height: 12),
                              Text(
                                l10n.savingProgress(
                                  state.saveProgress!.done,
                                  state.saveProgress!.total,
                                ),
                                style: Theme.of(context).textTheme.bodyMedium,
                              ),
                            ],
                          ),
                        )
                      : const SizedBox(
                          width: 44,
                          height: 44,
                          child: CircularProgressIndicator(strokeWidth: 3),
                        ),
                ),
              ),
            ),
          ),
      ],
    );
  }
}

/// Keeps a tab's widget tree alive when the user switches to another tab so
/// that unsaved field state (and the matching pending-edit registry entries)
/// stay consistent. Without this, TabBarView disposes off-screen tabs, which
/// destroys field controllers while the pending registry still counts those
/// edits — leading to a visible mismatch where typed text vanishes but the
/// Save button still shows a non-zero count.
class _KeepAliveTab extends StatefulWidget {
  const _KeepAliveTab({required this.child});

  final Widget child;

  @override
  State<_KeepAliveTab> createState() => _KeepAliveTabState();
}

class _KeepAliveTabState extends State<_KeepAliveTab>
    with AutomaticKeepAliveClientMixin {
  @override
  bool get wantKeepAlive => true;

  @override
  Widget build(BuildContext context) {
    super.build(context); // required by AutomaticKeepAliveClientMixin
    return widget.child;
  }
}

class _OverviewPanel extends StatelessWidget {
  const _OverviewPanel({
    required this.inspection,
    required this.notifier,
    required this.state,
  });

  final SaveInspection inspection;
  final EditorNotifier notifier;
  final EditorState state;

  @override
  Widget build(BuildContext context) {
    return ListView(
      padding: const EdgeInsets.all(20),
      children: [
        // Damage an older build left in the save is save-wide, so it is called
        // out here first — someone who only edits attributes or story state
        // would otherwise never see the inventory's copy of this warning.
        if (slotRepairWarranted(inspection)) ...[
          SlotRepairBanner(
            notifier: notifier,
            misalignedSlots: inspection.privateInventory.misalignedSlots,
            availability: slotRepairAvailability(
              inspection,
              canCompress: state.codecCompressReady,
            ),
          ),
          const SizedBox(height: 16),
        ],
        _HeaderCard(inspection: inspection, state: state, notifier: notifier),
        const SizedBox(height: 16),
        _SaveProfileCard(state: state, notifier: notifier),
        const SizedBox(height: 16),
        OverviewStatisticsSection(inspection: inspection, notifier: notifier),
      ],
    );
  }
}

/// Collapsed "Advanced (debug)" section in Settings. Bundles the two
/// developer-only readouts — the in-process codec self-test and the raw
/// inspection JSON — that normal editing never needs. Kept for
/// troubleshooting and bug reports (copy the JSON into an issue).
class _DebugSection extends StatefulWidget {
  const _DebugSection({required this.state, required this.notifier});

  final EditorState state;
  final EditorNotifier notifier;

  @override
  State<_DebugSection> createState() => _DebugSectionState();
}

class _DebugSectionState extends State<_DebugSection> {
  bool _expanded = false;
  String? _cachedJson;
  Object? _jsonFor;

  // Pretty-printing the raw inspection map isn't free; cache it per inspection
  // identity (a refresh/save yields a new instance) so rebuilds and copy taps
  // don't re-encode.
  String _json(SaveInspection inspection) {
    if (!identical(_jsonFor, inspection)) {
      _cachedJson = inspection.prettyJson();
      _jsonFor = inspection;
    }
    return _cachedJson!;
  }

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context);
    final state = widget.state;
    final notifier = widget.notifier;
    final inspection = state.inspection;
    final theme = Theme.of(context);
    return Card(
      child: Padding(
        padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 8),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            _CollapsibleCardHeader(
              icon: Icons.bug_report_outlined,
              title: l10n.debugSectionTitle,
              subtitle: l10n.debugSectionSubtitle,
              expanded: _expanded,
              onToggle: () => setState(() => _expanded = !_expanded),
            ),
            if (_expanded) ...[
              const SizedBox(height: 8),
              Consumer(
                builder: (context, ref, _) {
                  final showObjectIds = ref.watch(showObjectIdsProvider);
                  return SwitchListTile(
                    contentPadding: EdgeInsets.zero,
                    secondary: const Icon(Icons.badge_outlined),
                    title: Text(l10n.showObjectIdsTitle),
                    subtitle: Text(l10n.showObjectIdsSubtitle),
                    value: showObjectIds,
                    onChanged: ref.read(showObjectIdsProvider.notifier).set,
                  );
                },
              ),
              const Divider(height: 24),
              // Codec self-test: the in-process pure-Rust codec is effectively
              // always ready, so this is a capability readout / smoke test.
              Row(
                children: [
                  const Icon(Icons.compress_outlined, size: 20),
                  const SizedBox(width: 8),
                  Text(l10n.codecTitle, style: theme.textTheme.titleSmall),
                  const Spacer(),
                  OutlinedButton.icon(
                    icon: const Icon(Icons.refresh),
                    label: Text(l10n.check),
                    onPressed: () => notifier.checkCodec(),
                  ),
                  const SizedBox(width: 8),
                  OutlinedButton.icon(
                    icon: const Icon(Icons.verified_outlined),
                    label: Text(l10n.roundtrip),
                    onPressed: state.selectedPath == null || state.isLoading
                        ? null
                        : notifier.validateCodecRoundtrip,
                  ),
                ],
              ),
              const SizedBox(height: 12),
              CodecStatusView(
                codec: state.codecStatus,
                codecError: state.codecError,
              ),
              if (inspection != null) ...[
                const Divider(height: 24),
                ExpansionTile(
                  key: const ValueKey('inspection-json-expansion'),
                  initiallyExpanded: false,
                  tilePadding: EdgeInsets.zero,
                  childrenPadding: const EdgeInsets.only(bottom: 8),
                  leading: const Icon(Icons.data_object, size: 20),
                  title: Row(
                    children: [
                      Expanded(
                        child: Text(
                          l10n.inspectionJsonTitle,
                          style: theme.textTheme.titleSmall,
                        ),
                      ),
                      IconButton(
                        tooltip: l10n.copy,
                        icon: const Icon(Icons.copy),
                        onPressed: () => Clipboard.setData(
                          ClipboardData(text: _json(inspection)),
                        ),
                      ),
                    ],
                  ),
                  children: [
                    Align(
                      alignment: Alignment.centerLeft,
                      child: SelectableText(
                        _json(inspection),
                        style: TextStyle(
                          fontFamily: uiAwareMonospaceFontFamily(context),
                          fontSize: 12,
                        ),
                      ),
                    ),
                  ],
                ),
              ],
            ],
          ],
        ),
      ),
    );
  }
}

ProfileSummary? _selectedSaveProfile(EditorState state) {
  final save = state.selectedSave;
  if (save == null || save.isExternal) return null;
  for (final profile in state.profiles) {
    if (profile.profileId == save.persistentProfileId) return profile;
  }
  return null;
}

class _HeaderCard extends StatefulWidget {
  const _HeaderCard({
    required this.inspection,
    required this.state,
    required this.notifier,
  });

  final SaveInspection inspection;
  final EditorState state;
  final EditorNotifier notifier;

  @override
  State<_HeaderCard> createState() => _HeaderCardState();
}

class _HeaderCardState extends State<_HeaderCard> {
  final _saveNameKey = GlobalKey<_SaveNameEditorState>();
  final _gameTimeKey = GlobalKey<_GameTimeBadgeState>();

  @override
  Widget build(BuildContext context) {
    final inspection = widget.inspection;
    final state = widget.state;
    final notifier = widget.notifier;
    final l10n = AppLocalizations.of(context);
    final save = state.selectedSave;
    final currentProfile = _selectedSaveProfile(state);
    final actionsEnabled =
        !state.isLoading &&
        !state.hasUnsavedEdits &&
        state.deletedSaveRecovery == null &&
        currentProfile != null;
    final screenshot = save?.screenshot ?? inspection.screenshot;
    final title =
        save?.displayName ??
        inspection.playerSaveName ??
        inspection.slot ??
        l10n.savegameFallbackTitle;
    final slot = save?.slot ?? inspection.slot ?? l10n.savegameFallbackTitle;
    return Card(
      key: const ValueKey('selected-save-header-card'),
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: LayoutBuilder(
          builder: (context, constraints) {
            final compact = constraints.maxWidth < 560;
            final previewWidth = compact
                ? constraints.maxWidth.clamp(0.0, 320.0).toDouble()
                : 320.0;
            final previewHeight = previewWidth * 9 / 16;
            final preview = SizedBox(
              key: const ValueKey('header-screenshot'),
              width: previewWidth,
              height: previewHeight,
              child: _ScreenshotPreview(
                screenshot: screenshot,
                slot: slot,
                compact: compact,
              ),
            );

            Widget badges() {
              final summaryBadges = <Widget>[
                if (inspection.chapterId != null)
                  _InfoPill(
                    key: const ValueKey('chapter-badge'),
                    icon: Icons.flag_outlined,
                    label: l10n.chapterLabel(inspection.chapterId!),
                  ),
                if (inspection.timePlayedSeconds != null)
                  _InfoPill(
                    key: const ValueKey('time-played-badge'),
                    icon: Icons.timer_outlined,
                    label: _formatDurationSeconds(
                      l10n,
                      inspection.timePlayedSeconds,
                    ),
                  ),
              ];
              return Column(
                key: const ValueKey('header-badges'),
                mainAxisSize: MainAxisSize.min,
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  if (summaryBadges.isNotEmpty) ...[
                    Wrap(
                      spacing: 8,
                      runSpacing: 8,
                      crossAxisAlignment: WrapCrossAlignment.end,
                      children: summaryBadges,
                    ),
                    const SizedBox(height: 6),
                  ],
                  _GameTimeBadge(
                    key: _gameTimeKey,
                    inspection: inspection,
                    notifier: notifier,
                    editable:
                        inspection.privateEditable &&
                        inspection.privateTypedVerified &&
                        state.codecCompressReady,
                  ),
                ],
              );
            }

            Widget? deleteAction() {
              if (save == null || currentProfile == null) return null;
              return TextButton.icon(
                key: const ValueKey('delete-selected-save'),
                style: TextButton.styleFrom(
                  foregroundColor: Theme.of(context).colorScheme.error,
                  minimumSize: const Size(0, 32),
                  padding: const EdgeInsets.symmetric(horizontal: 12),
                  tapTargetSize: MaterialTapTargetSize.shrinkWrap,
                ),
                icon: const Icon(Icons.delete_forever_outlined, size: 18),
                label: Text(l10n.deleteSavegame),
                onPressed: actionsEnabled
                    ? () => _confirmDeleteSave(
                        context,
                        save: save,
                        profile: currentProfile,
                        notifier: notifier,
                      )
                    : null,
              );
            }

            Widget details({required bool fillPreviewHeight}) {
              final delete = deleteAction();
              final nameSyncsPersistentDataList =
                  state.selectedSave?.isExternal != true;
              return Column(
                key: const ValueKey('selected-save-header-details'),
                mainAxisSize: fillPreviewHeight
                    ? MainAxisSize.max
                    : MainAxisSize.min,
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  _SaveNameEditor(
                    key: _saveNameKey,
                    inspection: inspection,
                    notifier: notifier,
                    fallbackTitle: title,
                    enabled:
                        !state.isLoading &&
                        (!nameSyncsPersistentDataList ||
                            state.deletedSaveRecovery == null),
                    syncPersistentDataList: nameSyncsPersistentDataList,
                  ),
                  const SizedBox(height: 4),
                  Text(
                    inspection.path ?? '',
                    key: const ValueKey('selected-save-path'),
                    maxLines: 1,
                    overflow: TextOverflow.ellipsis,
                    style: Theme.of(context).textTheme.bodySmall,
                  ),
                  if (fillPreviewHeight)
                    const Spacer()
                  else
                    const SizedBox(height: 12),
                  if (fillPreviewHeight)
                    Row(
                      crossAxisAlignment: CrossAxisAlignment.end,
                      children: [
                        Expanded(child: badges()),
                        if (delete != null) ...[
                          const SizedBox(width: 4),
                          delete,
                        ],
                      ],
                    )
                  else ...[
                    badges(),
                    if (delete != null) ...[
                      const SizedBox(height: 8),
                      Align(alignment: Alignment.centerRight, child: delete),
                    ],
                  ],
                ],
              );
            }

            if (compact) {
              return Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  preview,
                  const SizedBox(height: 12),
                  details(fillPreviewHeight: false),
                ],
              );
            }

            return IntrinsicHeight(
              child: Row(
                crossAxisAlignment: CrossAxisAlignment.stretch,
                children: [
                  Align(alignment: Alignment.topLeft, child: preview),
                  const SizedBox(width: 14),
                  Expanded(child: details(fillPreviewHeight: true)),
                ],
              ),
            );
          },
        ),
      ),
    );
  }
}

class _ScreenshotPreview extends StatelessWidget {
  const _ScreenshotPreview({
    required this.screenshot,
    required this.slot,
    this.compact = false,
  });

  final ScreenshotSummary? screenshot;
  final String slot;
  final bool compact;

  @override
  Widget build(BuildContext context) {
    final bytes = _decodeScreenshot(screenshot);
    final radius = BorderRadius.circular(compact ? 6 : 8);
    final scheme = Theme.of(context).colorScheme;
    final l10n = AppLocalizations.of(context);
    final placeholder = ColoredBox(
      color: scheme.surfaceContainerHighest,
      child: Center(
        child: Icon(
          Icons.image_not_supported_outlined,
          size: compact ? 22 : 44,
          color: scheme.onSurfaceVariant,
        ),
      ),
    );
    return ClipRRect(
      borderRadius: radius,
      child: bytes == null
          ? placeholder
          : Image.memory(
              bytes,
              fit: BoxFit.cover,
              gaplessPlayback: true,
              semanticLabel: l10n.screenshotForSlot(slot),
              errorBuilder: (_, _, _) => placeholder,
            ),
    );
  }
}

class _InfoPill extends StatelessWidget {
  const _InfoPill({super.key, required this.icon, required this.label});

  final IconData icon;
  final String label;

  @override
  Widget build(BuildContext context) {
    final scheme = Theme.of(context).colorScheme;
    return DecoratedBox(
      decoration: BoxDecoration(
        color: scheme.surfaceContainerHighest,
        borderRadius: BorderRadius.circular(8),
        border: Border.all(color: scheme.outlineVariant),
      ),
      child: Padding(
        padding: const EdgeInsets.symmetric(horizontal: 9, vertical: 6),
        child: Row(
          mainAxisSize: MainAxisSize.min,
          children: [
            Icon(icon, size: 15, color: scheme.onSurfaceVariant),
            const SizedBox(width: 5),
            Flexible(
              child: Text(
                label,
                maxLines: 1,
                overflow: TextOverflow.ellipsis,
                style: Theme.of(context).textTheme.labelMedium,
              ),
            ),
          ],
        ),
      ),
    );
  }
}

Uint8List? _decodeScreenshot(ScreenshotSummary? screenshot) {
  final encoded = screenshot?.bytesBase64;
  if (encoded == null || encoded.isEmpty) return null;
  try {
    return base64Decode(encoded);
  } on FormatException {
    return null;
  }
}

class _SaveNameEditor extends StatefulWidget {
  const _SaveNameEditor({
    super.key,
    required this.inspection,
    required this.notifier,
    required this.fallbackTitle,
    required this.enabled,
    required this.syncPersistentDataList,
  });

  final SaveInspection inspection;
  final EditorNotifier notifier;
  final String fallbackTitle;
  final bool enabled;
  final bool syncPersistentDataList;

  @override
  State<_SaveNameEditor> createState() => _SaveNameEditorState();
}

class _SaveNameEditorState extends State<_SaveNameEditor> {
  Object? _inspectionIdentity;
  String? _path;
  String? _canonicalName;
  late String _draftName;

  @override
  void initState() {
    super.initState();
    _sync();
  }

  @override
  void didUpdateWidget(covariant _SaveNameEditor oldWidget) {
    super.didUpdateWidget(oldWidget);
    _sync();
  }

  void _sync() {
    final canonicalName = widget.inspection.playerSaveName;
    // Re-seed whenever the inspection identity changes (e.g. after a Reset /
    // refresh that produces a new SaveInspection instance) or when path/name
    // changes. This ensures that after a Reset the title visually reverts to
    // the canonical value even if the canonical value itself did not change.
    final sameIdentity = identical(widget.inspection, _inspectionIdentity);
    if (sameIdentity &&
        _path == widget.inspection.path &&
        _canonicalName == canonicalName) {
      return;
    }
    _inspectionIdentity = widget.inspection;
    _path = widget.inspection.path;
    _canonicalName = canonicalName;
    _draftName = _effectiveName(canonicalName);
  }

  String _effectiveName(String? canonicalName) =>
      canonicalName == null || canonicalName.isEmpty
      ? widget.fallbackTitle
      : canonicalName;

  Future<void> _edit() async {
    final value = await showDialog<String>(
      context: context,
      builder: (_) => _SaveNameDialog(initialName: _draftName),
    );
    if (!mounted || value == null || !widget.enabled) return;

    setState(() => _draftName = value);
    final original = _effectiveName(widget.inspection.playerSaveName);
    if (value == original) {
      widget.notifier.clearPendingEdit('publicName');
    } else {
      widget.notifier.setPendingEdit(
        'publicName',
        PendingSaveEdit(
          edits: [
            {'path': 'public.m_PlayerSaveName', 'value': value},
          ],
          syncPersistentDataList: widget.syncPersistentDataList,
        ),
      );
    }
  }

  @override
  Widget build(BuildContext context) {
    return Row(
      children: [
        Flexible(
          child: Text(
            _draftName,
            key: const ValueKey('selected-save-name'),
            style: Theme.of(context).textTheme.titleLarge,
            maxLines: 1,
            overflow: TextOverflow.ellipsis,
          ),
        ),
        const SizedBox(width: 4),
        IconButton(
          key: const ValueKey('edit-save-name'),
          tooltip: AppLocalizations.of(context).publicSaveName,
          constraints: const BoxConstraints.tightFor(width: 32, height: 32),
          padding: EdgeInsets.zero,
          icon: const Icon(Icons.edit_outlined, size: 20),
          onPressed: widget.enabled ? _edit : null,
        ),
      ],
    );
  }
}

class _SaveNameDialog extends StatefulWidget {
  const _SaveNameDialog({required this.initialName});

  final String initialName;

  @override
  State<_SaveNameDialog> createState() => _SaveNameDialogState();
}

class _SaveNameDialogState extends State<_SaveNameDialog> {
  late final TextEditingController _controller;
  String? _error;

  @override
  void initState() {
    super.initState();
    _controller = TextEditingController(text: widget.initialName);
  }

  @override
  void dispose() {
    _controller.dispose();
    super.dispose();
  }

  void _submit() {
    final candidate = _controller.text.trim();
    if (candidate.isEmpty) {
      setState(() => _error = AppLocalizations.of(context).required);
      return;
    }
    Navigator.of(context).pop(candidate);
  }

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context);
    return AlertDialog(
      title: Text(l10n.publicSaveName),
      content: TextField(
        key: const ValueKey('edit-save-name-field'),
        controller: _controller,
        autofocus: true,
        decoration: InputDecoration(
          labelText: l10n.publicSaveName,
          errorText: _error,
        ),
        onSubmitted: (_) => _submit(),
      ),
      actions: [
        TextButton(
          onPressed: () => Navigator.of(context).pop(),
          child: Text(l10n.cancel),
        ),
        FilledButton(
          key: const ValueKey('confirm-save-name'),
          onPressed: _submit,
          child: Text(l10n.save),
        ),
      ],
    );
  }
}

/// Profile association stored by the game in PersistentDataList.sav. Changing
/// it is an explicit immediate operation (with paired backups), because it must
/// atomically update both the slot file and the profile index rather than join
/// the selected save's ordinary pending edit batch.
class _SaveProfileCard extends StatelessWidget {
  const _SaveProfileCard({required this.state, required this.notifier});

  final EditorState state;
  final EditorNotifier notifier;

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context);
    final theme = Theme.of(context);
    final save = state.selectedSave;
    final external = save?.isExternal == true;
    // A detached file's embedded numeric profile id is not authoritative for
    // this folder. Keep the selector empty so even a coincidental matching id
    // can be chosen to trigger the import.
    final currentProfile = _selectedSaveProfile(state);
    final currentId = currentProfile?.profileId;
    final enabled =
        state.profiles.isNotEmpty &&
        !state.isLoading &&
        !state.hasUnsavedEdits &&
        state.deletedSaveRecovery == null;
    final explanation = external
        ? l10n.saveProfileExternalHint
        : state.profiles.isEmpty
        ? l10n.saveProfileNoProfiles
        : l10n.saveProfileDescription;

    final profileCopy = Row(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Icon(
          external ? Icons.link_off_outlined : Icons.account_tree_outlined,
          color: theme.colorScheme.primary,
        ),
        const SizedBox(width: 12),
        Expanded(
          child: Column(
            key: const ValueKey('save-profile-copy'),
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Text(l10n.saveProfileTitle, style: theme.textTheme.titleSmall),
              const SizedBox(height: 3),
              Text(
                explanation,
                style: theme.textTheme.bodySmall?.copyWith(
                  color: theme.colorScheme.onSurfaceVariant,
                ),
              ),
            ],
          ),
        ),
      ],
    );
    final profileSelector = InputDecorator(
      key: const ValueKey('save-profile-selector'),
      decoration: InputDecoration(labelText: l10n.profile, isDense: true),
      isEmpty: currentId == null,
      child: DropdownButtonHideUnderline(
        child: DropdownButton<int>(
          // Unlike DropdownButtonFormField's initialValue, value is controlled
          // by the current editor state. Profile refreshes and failed writes
          // therefore update the displayed selection without remounting the
          // control during unrelated background state changes.
          value: currentId,
          isDense: true,
          isExpanded: true,
          hint: Text(l10n.saveProfileSelect),
          items: [
            for (final profile in state.profiles)
              DropdownMenuItem<int>(
                value: profile.profileId,
                child: Text(
                  localizedProfileDisplayName(l10n, profile),
                  overflow: TextOverflow.ellipsis,
                ),
              ),
          ],
          onChanged: enabled
              ? (profileId) {
                  if (profileId != null && profileId != currentId) {
                    notifier.assignSelectedSaveToProfile(profileId);
                  }
                }
              : null,
        ),
      ),
    );
    final profileControls = Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        profileSelector,
        if (save != null && currentProfile != null)
          Padding(
            padding: const EdgeInsets.only(top: 8),
            child: Align(
              alignment: Alignment.centerRight,
              child: TextButton.icon(
                key: const ValueKey('remove-selected-save-profile'),
                icon: const Icon(Icons.link_off_outlined, size: 18),
                label: Text(l10n.removeFromProfile),
                onPressed: enabled
                    ? () => _confirmRemoveSaveFromProfile(
                        context,
                        save: save,
                        profile: currentProfile,
                        notifier: notifier,
                      )
                    : null,
              ),
            ),
          ),
      ],
    );

    return Card(
      key: const ValueKey('save-profile-card'),
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: LayoutBuilder(
          builder: (context, constraints) {
            if (constraints.maxWidth < 700) {
              return Column(
                crossAxisAlignment: CrossAxisAlignment.stretch,
                children: [
                  profileCopy,
                  const SizedBox(height: 12),
                  profileControls,
                ],
              );
            }
            return Row(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Expanded(child: profileCopy),
                const SizedBox(width: 20),
                SizedBox(width: 360, child: profileControls),
              ],
            );
          },
        ),
      ),
    );
  }
}

/// Compact Overview editor for the world game clock (the single typed
/// `DoubleProperty` at `m_GenericData{GameTime} › CurrentTime › TotalSeconds`).
/// The header shows a readable badge; editing happens in a dialog so four
/// numeric fields do not dominate the save summary. Loads its value lazily via
/// [EditorNotifier.loadGameTime] and renders nothing when the save has no leaf.
///
/// Edits flow through the same pending-edit registry as every other typed
/// editor (`private.typed.setValue`, key `gameTime`), so the shared Save button
/// writes them. [editable] mirrors the Player/All-data gating (decoded, typed-
/// verified, compress-ready codec); when false the badge is read-only.
class _GameTimeBadge extends StatefulWidget {
  const _GameTimeBadge({
    super.key,
    required this.inspection,
    required this.notifier,
    required this.editable,
  });

  final SaveInspection inspection;
  final EditorNotifier notifier;
  final bool editable;

  @override
  State<_GameTimeBadge> createState() => _GameTimeBadgeState();
}

class _GameTimeBadgeState extends State<_GameTimeBadge> {
  static const _pendingKey = 'gameTime';

  GameTime? _gameTime;
  GameTimeParts? _parts;
  bool _loaded = false;
  // Discards results from superseded reloads (rapid inspection swaps).
  int _epoch = 0;

  @override
  void initState() {
    super.initState();
    _load();
  }

  @override
  void didUpdateWidget(covariant _GameTimeBadge oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (!identical(widget.inspection, oldWidget.inspection)) _load();
  }

  Future<void> _load() async {
    final epoch = ++_epoch;
    // Drop the cached clock and hide the card synchronously, BEFORE awaiting.
    // A same-save Reset/refresh/save keeps this card mounted and swaps in a new
    // SaveInspection (pending cleared centrally); leaving the previous value
    // shown as editable during the search would let a keystroke re-register a
    // stale clock against the refreshed save. _load runs only from initState /
    // didUpdateWidget, each immediately followed by build(), so resetting the
    // fields directly here is reflected without setState. The global save/
    // refresh loading overlay masks the brief hide, so there is no flicker.
    _loaded = false;
    _gameTime = null;
    _parts = null;
    final loaded = widget.inspection.privateDecoded
        ? await widget.notifier.loadGameTime()
        : null;
    // Drop stale results; a newer _load already advanced the epoch.
    if (!mounted || epoch != _epoch) return;
    setState(() {
      _gameTime = loaded;
      _loaded = true;
      _parts = loaded == null
          ? null
          : GameTimeParts.fromTotalSeconds(loaded.totalSeconds);
    });
    // Do NOT touch the pending registry here: refresh() clears it centrally in
    // event-handler context. Mutating it from this build-adjacent callback would
    // throw with flutter_riverpod, exactly as the hero-stats card documents.
  }

  Future<void> _edit() async {
    final gameTime = _gameTime;
    final parts = _parts;
    if (!widget.editable || gameTime == null || parts == null) return;
    final edited = await showDialog<GameTimeParts>(
      context: context,
      builder: (_) => GameTimeDialog(initialValue: parts),
    );
    if (!mounted || edited == null) return;

    setState(() => _parts = edited);
    final total = edited.toTotalSeconds();
    // Compare against the truncated original: re-typing the same clock must not
    // leave a no-op edit that still bumps the Save counter (and would rewrite
    // away the harmless sub-second fraction for nothing).
    if (total == gameTime.totalSeconds.floor()) {
      widget.notifier.clearPendingEdit(_pendingKey);
      return;
    }
    widget.notifier.setPendingEdit(
      _pendingKey,
      PendingSaveEdit(
        edits: [
          {
            'path': 'private.typed.setValue',
            // DoubleProperty: send a float (matches the hero-stats write path).
            'value': {'path': gameTime.path, 'value': total.toDouble()},
          },
        ],
      ),
    );
  }

  @override
  Widget build(BuildContext context) {
    // Hidden until we know there's a clock to show — keeps the Overview layout
    // identical for saves without one (and avoids a load flash).
    final parts = _parts;
    if (!_loaded || _gameTime == null || parts == null) {
      return const SizedBox.shrink();
    }

    final theme = Theme.of(context);
    final l10n = AppLocalizations.of(context);
    final clock = [
      parts.hour,
      parts.minute,
      parts.second,
    ].map((value) => value.toString().padLeft(2, '0')).join(':');
    final total = parts.toTotalSeconds();

    return Tooltip(
      message: '${l10n.gameTimeTitle} ${l10n.gameTimeTotal(total)}',
      child: FittedBox(
        fit: BoxFit.scaleDown,
        alignment: Alignment.centerLeft,
        child: ActionChip(
          key: const ValueKey('game-time-badge'),
          visualDensity: VisualDensity.compact,
          materialTapTargetSize: MaterialTapTargetSize.shrinkWrap,
          avatar: Icon(
            Icons.schedule_outlined,
            size: 18,
            color: theme.colorScheme.primary,
          ),
          label: Row(
            mainAxisSize: MainAxisSize.min,
            children: [
              Text(
                '${l10n.gameTimeDay} ${parts.day} · $clock',
                key: const ValueKey('game-time-value'),
                style: theme.textTheme.labelLarge?.copyWith(
                  color: theme.colorScheme.onSurfaceVariant,
                ),
              ),
              if (widget.editable) ...[
                const SizedBox(width: 7),
                Icon(
                  Icons.edit_outlined,
                  size: 16,
                  color: theme.colorScheme.onSurfaceVariant,
                ),
              ],
            ],
          ),
          onPressed: widget.editable ? _edit : null,
        ),
      ),
    );
  }
}

class _CollapsibleCardHeader extends StatelessWidget {
  const _CollapsibleCardHeader({
    required this.icon,
    required this.title,
    required this.expanded,
    this.subtitle,
    this.onToggle,
  });

  final IconData icon;
  final String title;
  final String? subtitle;
  final bool expanded;
  final VoidCallback? onToggle;

  @override
  Widget build(BuildContext context) {
    final header = Row(
      children: [
        Icon(icon),
        const SizedBox(width: 8),
        Expanded(
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            mainAxisSize: MainAxisSize.min,
            children: [
              Text(title, style: Theme.of(context).textTheme.titleMedium),
              if (subtitle != null)
                Text(subtitle!, style: Theme.of(context).textTheme.bodySmall),
            ],
          ),
        ),
        Icon(expanded ? Icons.expand_less : Icons.expand_more),
      ],
    );
    if (onToggle == null) return header;
    return InkWell(
      onTap: onToggle,
      borderRadius: BorderRadius.circular(8),
      child: Padding(
        padding: const EdgeInsets.symmetric(vertical: 4),
        child: header,
      ),
    );
  }
}

/// Exhaustive, source-aware GSAV browser for technical metadata plus the typed
/// PUBLIC and PRIVATE property trees. Containers, structs and opaque payloads
/// remain visible instead of being discarded by the scalar editor projection.
class _AllDataPanel extends StatefulWidget {
  const _AllDataPanel({
    required this.inspection,
    required this.notifier,
    required this.editable,
  });

  final SaveInspection inspection;
  final EditorNotifier notifier;
  final bool editable;

  @override
  State<_AllDataPanel> createState() => _AllDataPanelState();
}

class _AllDataPanelState extends State<_AllDataPanel> {
  static const _pageSizes = [25, 50, 100, 250, 500];
  static const _sources = ['all', 'metadata', 'public', 'private'];
  static const _kinds = ['all', 'scalar', 'struct', 'container', 'opaque'];

  final _controller = TextEditingController();
  TypedSearchResult? _result;
  bool _searching = false;
  int _requestSeq = 0;
  int _pageSize = EditorPageSize.detail;
  String _activeQuery = '';
  String _source = 'private';
  String _kind = 'all';
  String _type = '';
  bool? _editableFilter;
  // Tracks the inspection identity so _TypedPropertyRow can reset draft text
  // when a Reset/refresh produces a new inspection (same path, same values).
  Object? _inspectionReloadKey;
  // Unsaved field text keyed by pending-registry key. Rows are disposed when
  // search/pagination scrolls them out of the result page, but their pending
  // edits stay registered globally — without this store a returning row would
  // re-seed from the canonical value and hide an edit the Save button still
  // writes. Lives alongside the pending registry: entries are added/removed in
  // _updatePending and the whole map is dropped whenever pending is cleared
  // centrally (new inspection identity).
  final Map<String, Object?> _typedDrafts = {};

  @override
  void initState() {
    super.initState();
    _inspectionReloadKey = widget.inspection;
    // Metadata and PUBLIC remain useful even when PRIVATE decoding failed.
    if (widget.inspection.format == 'GSAV') {
      WidgetsBinding.instance.addPostFrameCallback((_) {
        if (mounted) _run(offset: 0);
      });
    }
  }

  @override
  void didUpdateWidget(covariant _AllDataPanel oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (widget.inspection.path != oldWidget.inspection.path) {
      // A different save was selected while this tab stayed mounted. The cached
      // results belong to the old file; drop them (otherwise they show stale
      // rows while writes target the newly selected save) and re-list from
      // page one.
      _controller.clear();
      _activeQuery = '';
      _source = 'private';
      _kind = 'all';
      _type = '';
      _editableFilter = null;
      // Invalidate any in-flight search for the previous save.
      _requestSeq++;
      _inspectionReloadKey = widget.inspection;
      // Switching saves clears pending centrally; drop the drafts with it.
      _typedDrafts.clear();
      setState(() {
        _result = null;
        _searching = false;
      });
      if (widget.inspection.format == 'GSAV') {
        WidgetsBinding.instance.addPostFrameCallback((_) {
          if (mounted) _run(offset: 0);
        });
      }
    } else if (!identical(widget.inspection, oldWidget.inspection)) {
      // Same path but a new SaveInspection instance — the save was written and
      // refreshed (or Reset). Re-run the active query so the All data panel
      // shows the post-save values; also update the reloadKey so row fields
      // reseed their draft text to the canonical value.
      _inspectionReloadKey = widget.inspection;
      // Save/restore/refresh cleared pending centrally; the drafts mirror it.
      _typedDrafts.clear();
      if (widget.inspection.format == 'GSAV') {
        final currentOffset = _result?.offset ?? 0;
        WidgetsBinding.instance.addPostFrameCallback((_) {
          if (mounted) _run(offset: currentOffset);
        });
      }
    }
  }

  @override
  void dispose() {
    _controller.dispose();
    super.dispose();
  }

  /// Run the search at [offset] for the active query and page size. Only an
  /// explicit new search ([newQuery] true, which also resets to the first page
  /// via the caller's offset) adopts the field text; pagination, page-size
  /// changes, and post-save refreshes reuse [_activeQuery] so they cannot query
  /// uncommitted field text at a stale offset or show mismatched totals.
  Future<void> _run({required int offset, bool newQuery = false}) async {
    if (newQuery) _activeQuery = _controller.text.trim();
    final seq = ++_requestSeq;
    setState(() => _searching = true);
    final result = await widget.notifier.searchTypedProperties(
      _activeQuery,
      offset: offset,
      limit: _pageSize,
      includeNodes: true,
      source: _source,
      kind: _kind,
      type: _type,
      editable: _editableFilter,
    );
    if (!mounted || seq != _requestSeq) return;
    setState(() {
      _result = result;
      _searching = false;
    });
  }

  void _goToPage(int pageIndex) {
    final result = _result;
    if (result == null) return;
    final clamped = pageIndex.clamp(0, result.pageCount - 1);
    _run(offset: clamped * _pageSize);
  }

  void _setPageSize(int? size) {
    if (size == null || size == _pageSize) return;
    setState(() => _pageSize = size);
    _run(offset: 0);
  }

  void _setSource(String value) {
    if (_source == value) return;
    setState(() => _source = value);
    _run(offset: 0, newQuery: true);
  }

  void _setKind(String value) {
    if (_kind == value) return;
    setState(() => _kind = value);
    _run(offset: 0, newQuery: true);
  }

  void _setEditableFilter(bool? value) {
    if (_editableFilter == value) return;
    setState(() => _editableFilter = value);
    _run(offset: 0, newQuery: true);
  }

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context);
    if (widget.inspection.format != 'GSAV') {
      return _MessagePane(
        icon: Icons.tune,
        title: l10n.tabAllData,
        body: l10n.allDataLockedBody,
      );
    }
    final theme = Theme.of(context);
    return Padding(
      padding: const EdgeInsets.fromLTRB(16, 12, 16, 16),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          DecoratedBox(
            decoration: BoxDecoration(
              color: theme.colorScheme.surfaceContainerLow,
              borderRadius: BorderRadius.circular(16),
              border: Border.all(color: theme.colorScheme.outlineVariant),
            ),
            child: Padding(
              padding: const EdgeInsets.all(14),
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.stretch,
                children: [
                  Row(
                    children: [
                      Icon(
                        Icons.account_tree_outlined,
                        color: theme.colorScheme.primary,
                      ),
                      const SizedBox(width: 10),
                      Expanded(
                        child: Column(
                          crossAxisAlignment: CrossAxisAlignment.start,
                          children: [
                            Text(
                              l10n.tabAllData,
                              style: theme.textTheme.titleMedium,
                            ),
                            Text(
                              l10n.allDataDescription,
                              style: theme.textTheme.bodySmall?.copyWith(
                                color: theme.colorScheme.onSurfaceVariant,
                              ),
                            ),
                          ],
                        ),
                      ),
                    ],
                  ),
                  const SizedBox(height: 12),
                  Wrap(
                    spacing: 8,
                    runSpacing: 6,
                    children: [
                      for (final source in _sources)
                        ChoiceChip(
                          avatar: Icon(_sourceIcon(source), size: 17),
                          label: Text(
                            source == 'all'
                                ? l10n.categoryAll
                                : source.toUpperCase(),
                          ),
                          selected: _source == source,
                          onSelected: (_) => _setSource(source),
                        ),
                    ],
                  ),
                  const SizedBox(height: 10),
                  Row(
                    children: [
                      Expanded(
                        child: TextField(
                          controller: _controller,
                          // A PRIVATE query scans the exhaustive tree. Keep it
                          // explicit so typing a word cannot enqueue several
                          // million-node scans behind one another.
                          onChanged: (_) => setState(() {}),
                          textInputAction: TextInputAction.search,
                          decoration: InputDecoration(
                            labelText: l10n.searchPropertiesLabel,
                            prefixIcon: const Icon(Icons.search),
                            suffixIcon: _searching
                                ? const Padding(
                                    padding: EdgeInsets.all(12),
                                    child: SizedBox(
                                      width: 16,
                                      height: 16,
                                      child: CircularProgressIndicator(
                                        strokeWidth: 2,
                                      ),
                                    ),
                                  )
                                : _controller.text.isEmpty
                                ? null
                                : IconButton(
                                    icon: const Icon(Icons.clear),
                                    onPressed: () {
                                      _controller.clear();
                                      _run(offset: 0, newQuery: true);
                                    },
                                  ),
                          ),
                          onSubmitted: (_) => _run(offset: 0, newQuery: true),
                        ),
                      ),
                      const SizedBox(width: 8),
                      IconButton.filledTonal(
                        tooltip: l10n.searchPropertiesLabel,
                        onPressed: _searching
                            ? null
                            : () => _run(offset: 0, newQuery: true),
                        icon: const Icon(Icons.search),
                      ),
                    ],
                  ),
                  const SizedBox(height: 10),
                  _buildFilters(theme),
                  if (_result != null && _result!.error == null) ...[
                    const SizedBox(height: 10),
                    _buildSummary(theme, _result!),
                  ],
                ],
              ),
            ),
          ),
          const SizedBox(height: 8),
          _buildPaginationBar(theme),
          const SizedBox(height: 6),
          Expanded(child: _buildResults(theme)),
        ],
      ),
    );
  }

  Widget _buildFilters(ThemeData theme) {
    final l10n = AppLocalizations.of(context);
    final resultTypes =
        _result?.summary.types.keys.toList() ?? const <String>[];
    final types = <String>{if (_type.isNotEmpty) _type, ...resultTypes}.toList()
      ..sort();
    return LayoutBuilder(
      builder: (context, constraints) {
        return Wrap(
          crossAxisAlignment: WrapCrossAlignment.center,
          spacing: 7,
          runSpacing: 6,
          children: [
            for (final kind in _kinds)
              ChoiceChip(
                label: Text(
                  kind == 'all' ? l10n.categoryAll : _kindLabel(l10n, kind),
                ),
                selected: _kind == kind,
                onSelected: (_) => _setKind(kind),
              ),
            const SizedBox(width: 4),
            ChoiceChip(
              avatar: const Icon(Icons.edit_outlined, size: 16),
              label: Text(l10n.allDataEditable),
              selected: _editableFilter == true,
              onSelected: (_) =>
                  _setEditableFilter(_editableFilter == true ? null : true),
            ),
            ChoiceChip(
              avatar: const Icon(Icons.lock_outline, size: 16),
              label: Text(l10n.allDataReadOnly),
              selected: _editableFilter == false,
              onSelected: (_) =>
                  _setEditableFilter(_editableFilter == false ? null : false),
            ),
            if (types.isNotEmpty)
              SizedBox(
                width: constraints.maxWidth < 520 ? constraints.maxWidth : 240,
                child: DropdownButtonFormField<String>(
                  key: ValueKey('all-data-type-$_type-${types.length}'),
                  initialValue: _type,
                  isExpanded: true,
                  decoration: InputDecoration(
                    isDense: true,
                    prefixIcon: const Icon(Icons.data_object, size: 18),
                    labelText: l10n.allDataType,
                  ),
                  items: [
                    DropdownMenuItem(value: '', child: Text(l10n.categoryAll)),
                    for (final type in types)
                      DropdownMenuItem(
                        value: type,
                        child: Text(type, overflow: TextOverflow.ellipsis),
                      ),
                  ],
                  onChanged: (value) {
                    setState(() => _type = value ?? '');
                    _run(offset: 0, newQuery: true);
                  },
                ),
              ),
          ],
        );
      },
    );
  }

  Widget _buildSummary(ThemeData theme, TypedSearchResult result) {
    final summary = result.summary;
    return Wrap(
      spacing: 7,
      runSpacing: 6,
      children: [
        _AllDataMetric(
          icon: Icons.dataset_outlined,
          label:
              '${AppLocalizations.of(context).allDataNodes}: ${result.total}',
          color: theme.colorScheme.primary,
        ),
        _AllDataMetric(
          icon: Icons.edit_outlined,
          label:
              '${AppLocalizations.of(context).allDataEditable}: ${summary.editable}',
          color: theme.colorScheme.tertiary,
        ),
        _AllDataMetric(
          icon: Icons.lock_outline,
          label:
              '${AppLocalizations.of(context).allDataReadOnly}: ${summary.readOnly}',
          color: theme.colorScheme.outline,
        ),
        for (final source in summary.typedSources)
          _AllDataMetric(
            icon: Icons.verified_outlined,
            label: AppLocalizations.of(
              context,
            ).allDataTypedSource(source.toUpperCase()),
            color: theme.colorScheme.secondary,
          ),
      ],
    );
  }

  Widget _buildResults(ThemeData theme) {
    final l10n = AppLocalizations.of(context);
    final result = _result;
    if (result == null) {
      if (_searching) {
        return _MessagePane(
          icon: Icons.hourglass_empty,
          title: l10n.decodingSaveTitle,
          body: l10n.decodingSaveBody,
        );
      }
      return _MessagePane(
        icon: Icons.search,
        title: l10n.searchTheSaveTitle,
        body: l10n.searchTheSaveBody,
      );
    }
    if (result.error != null) {
      return _MessagePane(
        icon: Icons.error_outline,
        title: l10n.searchFailedTitle,
        body: result.error!,
      );
    }
    if (result.results.isEmpty) {
      return _MessagePane(
        icon: Icons.search_off,
        title: l10n.noMatchesTitle,
        body: l10n.noMatchesBody,
      );
    }
    final list = ListView.separated(
      itemCount: result.results.length,
      separatorBuilder: (_, _) => const SizedBox(height: 5),
      itemBuilder: (context, index) {
        final hit = result.results[index];
        return _TypedPropertyRow(
          key: ValueKey(hit.stableId),
          hit: hit,
          editable: widget.editable && hit.editable,
          notifier: widget.notifier,
          reloadKey: _inspectionReloadKey,
          drafts: _typedDrafts,
        );
      },
    );
    if (result.warnings.isEmpty) return list;
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        Container(
          padding: const EdgeInsets.all(10),
          margin: const EdgeInsets.only(bottom: 6),
          decoration: BoxDecoration(
            color: theme.colorScheme.errorContainer,
            borderRadius: BorderRadius.circular(10),
          ),
          child: Text(
            result.warnings.join('\n'),
            style: theme.textTheme.bodySmall?.copyWith(
              color: theme.colorScheme.onErrorContainer,
            ),
          ),
        ),
        Expanded(child: list),
      ],
    );
  }

  Widget _buildPaginationBar(ThemeData theme) {
    final l10n = AppLocalizations.of(context);
    final result = _result;
    if (result == null || (result.error != null) || result.total == 0) {
      return const SizedBox.shrink();
    }
    final first = result.offset + 1;
    final last = result.offset + result.results.length;
    final busy = _searching;
    final muted = theme.textTheme.bodySmall?.copyWith(
      color: theme.colorScheme.onSurfaceVariant,
    );
    return Wrap(
      crossAxisAlignment: WrapCrossAlignment.center,
      spacing: 4,
      runSpacing: 4,
      children: [
        IconButton(
          tooltip: l10n.firstPage,
          visualDensity: VisualDensity.compact,
          icon: const Icon(Icons.first_page),
          onPressed: busy || !result.hasPrevious ? null : () => _goToPage(0),
        ),
        IconButton(
          tooltip: l10n.previousPage,
          visualDensity: VisualDensity.compact,
          icon: const Icon(Icons.chevron_left),
          onPressed: busy || !result.hasPrevious
              ? null
              : () => _goToPage(result.pageIndex - 1),
        ),
        IconButton(
          tooltip: l10n.nextPage,
          visualDensity: VisualDensity.compact,
          icon: const Icon(Icons.chevron_right),
          onPressed: busy || !result.hasNext
              ? null
              : () => _goToPage(result.pageIndex + 1),
        ),
        IconButton(
          tooltip: l10n.lastPage,
          visualDensity: VisualDensity.compact,
          icon: const Icon(Icons.last_page),
          onPressed: busy || !result.hasNext
              ? null
              : () => _goToPage(result.pageCount - 1),
        ),
        const SizedBox(width: 4),
        Text(
          l10n.pageOfPages(result.pageIndex + 1, result.pageCount),
          style: muted,
        ),
        const SizedBox(width: 8),
        Text(l10n.rangeOfTotal(first, last, result.total), style: muted),
        const SizedBox(width: 8),
        Text(l10n.perPage, style: muted),
        DropdownButton<int>(
          value: _pageSize,
          isDense: true,
          underline: const SizedBox.shrink(),
          onChanged: busy ? null : _setPageSize,
          items: [
            for (final size in _pageSizes)
              DropdownMenuItem(value: size, child: Text('$size')),
          ],
        ),
      ],
    );
  }

  static IconData _sourceIcon(String source) => switch (source) {
    'metadata' => Icons.memory_outlined,
    'public' => Icons.visibility_outlined,
    'private' => Icons.lock_open_outlined,
    _ => Icons.layers_outlined,
  };

  static String _kindLabel(AppLocalizations l10n, String kind) =>
      switch (kind) {
        'scalar' => l10n.allDataScalars,
        'struct' => l10n.allDataStructs,
        'container' => l10n.allDataContainers,
        'opaque' => l10n.allDataOpaque,
        _ => kind,
      };
}

class _AllDataMetric extends StatelessWidget {
  const _AllDataMetric({
    required this.icon,
    required this.label,
    required this.color,
  });

  final IconData icon;
  final String label;
  final Color color;

  @override
  Widget build(BuildContext context) {
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 9, vertical: 5),
      decoration: BoxDecoration(
        color: color.withValues(alpha: .10),
        borderRadius: BorderRadius.circular(999),
        border: Border.all(color: color.withValues(alpha: .25)),
      ),
      child: Row(
        mainAxisSize: MainAxisSize.min,
        children: [
          Icon(icon, size: 15, color: color),
          const SizedBox(width: 5),
          Text(label, style: Theme.of(context).textTheme.labelMedium),
        ],
      ),
    );
  }
}

class _TypedPropertyRow extends StatefulWidget {
  const _TypedPropertyRow({
    super.key,
    required this.hit,
    required this.editable,
    required this.notifier,
    required this.drafts,
    this.reloadKey,
  });

  final TypedPropertyHit hit;
  final bool editable;
  final EditorNotifier notifier;
  // Panel-owned store of unsaved field text keyed by pending-registry key.
  // Rows seed from it on creation and write through it on change, so an edit
  // survives the row being disposed by search/pagination and stays visible
  // (instead of becoming a hidden pending edit) when the row comes back.
  final Map<String, Object?> drafts;
  // When provided, a change in identity forces a reseed of the field from the
  // canonical hit value (e.g. after a Reset that reverts to the same value).
  final Object? reloadKey;

  @override
  State<_TypedPropertyRow> createState() => _TypedPropertyRowState();
}

class _TypedPropertyRowState extends State<_TypedPropertyRow> {
  late final TextEditingController _controller = TextEditingController(
    text: _editorText(
      widget.drafts[_pendingKey] ?? widget.hit.editValue ?? widget.hit.value,
    ),
  );
  final Map<String, String> _componentDrafts = {};
  // Unsaved bool toggle. The switch has no text controller to hold draft
  // state, so without this it would snap back to the canonical value on the
  // next rebuild even though the pending edit is registered.
  bool? _boolDraft;
  Object? _lastReloadKey;

  @override
  void initState() {
    super.initState();
    _lastReloadKey = widget.reloadKey;
    final draft = widget.drafts[_pendingKey];
    if (_isBool && draft is bool) {
      _boolDraft = draft;
    }
  }

  @override
  void didUpdateWidget(covariant _TypedPropertyRow oldWidget) {
    super.didUpdateWidget(oldWidget);
    // Re-seed when the reloadKey identity changes (e.g. after Reset/refresh
    // that produces a new inspection — same canonical value, draft must go).
    final newKey = widget.reloadKey;
    final keyChanged = newKey != null && !identical(newKey, _lastReloadKey);
    if (keyChanged) {
      _lastReloadKey = newKey;
    }
    // A successful save refreshes the list and rebinds this row to a hit with
    // the persisted (possibly normalized) value. Sync the field to it so it
    // stops showing the pre-save text. Only rows whose value actually changed
    // update, so an unrelated row's save cannot clobber in-progress typing here.
    if (keyChanged ||
        (widget.hit.value != oldWidget.hit.value &&
            _controller.text != widget.hit.value)) {
      _controller.text = _editorText(widget.hit.editValue ?? widget.hit.value);
      _boolDraft = null;
      _componentDrafts.clear();
      // The drafts map is plain panel state (not a provider), so unlike the
      // pending registry it is safe to drop the stale entry here.
      widget.drafts.remove(_pendingKey);
      // No registry mutation here: provider writes are illegal during the
      // build phase, and every flow that changes the canonical value
      // (save/restore/refresh) already cleared pending centrally.
    }
  }

  @override
  void dispose() {
    _controller.dispose();
    super.dispose();
  }

  bool get _isBool => widget.hit.type == 'BoolProperty';
  bool get _isVector =>
      widget.hit.isNativeStruct && widget.hit.editValue is Map;
  bool get _isTags => widget.hit.structType == 'GameplayTagContainer';
  bool get _isDateTime => widget.hit.structType == 'DateTime';

  /// Returns a key string that identifies this property in the pending registry.
  String get _pendingKey => 'typed:${widget.hit.path.join(' ')}';

  static String _editorText(Object? value) {
    if (value is List) return value.join(', ');
    return value?.toString() ?? '';
  }

  Object? _coerce(String text) {
    final type = widget.hit.type;
    if (type == 'StrProperty' || type == 'NameProperty') {
      // String values are written verbatim — leading/trailing whitespace may
      // be intentional, so no trim.
      return text;
    }
    if (type == 'ObjectProperty' ||
        type == 'ClassProperty' ||
        type == 'EnumProperty') {
      return text.trim();
    }
    final raw = text.trim();
    if (type == 'BoolProperty') {
      // The bool toggle reports 'true'/'false'; anything else is invalid.
      if (raw == 'true') return true;
      if (raw == 'false') return false;
      return null;
    }
    if (type == 'FloatProperty' || type == 'DoubleProperty') {
      return double.tryParse(raw);
    }
    if (type == 'ByteProperty') {
      // Two serialized forms share the tag type: plain byte (number) and
      // enum-as-FString. Send a number when it parses; otherwise send the
      // text and let the core validate against the actual form.
      return int.tryParse(raw) ?? raw;
    }
    return int.tryParse(raw);
  }

  void _updatePending(String text) {
    if (!widget.editable) return;
    Object? value;
    if (_isTags) {
      value = text
          .split(RegExp(r'[,\r\n]+'))
          .map((tag) => tag.trim())
          .where((tag) => tag.isNotEmpty)
          .toList(growable: false);
    } else if (_isDateTime) {
      value = int.tryParse(text.trim());
    } else if (widget.hit.isNativeStruct) {
      value = text.trim();
    } else {
      value = _coerce(text);
    }
    if (value == null) {
      // Invalid / unparseable — don't contribute to pending.
      widget.drafts.remove(_pendingKey);
      widget.notifier.clearPendingEdit(_pendingKey);
      if (mounted) setState(() {});
      return;
    }
    _setPendingValue(value);
  }

  void _setPendingValue(Object value) {
    final original = widget.hit.editValue ?? _coerce(widget.hit.value);
    if (_jsonEqual(value, original)) {
      widget.drafts.remove(_pendingKey);
      widget.notifier.clearPendingEdit(_pendingKey);
      if (mounted) setState(() {});
      return;
    }
    widget.drafts[_pendingKey] = value;
    widget.notifier.setPendingEdit(
      _pendingKey,
      PendingSaveEdit(
        edits: [
          {
            'path': 'private.typed.setValue',
            'value': {'path': widget.hit.path, 'value': value},
          },
        ],
      ),
    );
    if (mounted) setState(() {});
  }

  bool _jsonEqual(Object? left, Object? right) {
    try {
      return jsonEncode(left) == jsonEncode(right);
    } catch (_) {
      return left == right;
    }
  }

  void _updateVectorComponent(String name, String text) {
    _componentDrafts[name] = text;
    final source = (widget.drafts[_pendingKey] ?? widget.hit.editValue) as Map?;
    final keys = (widget.hit.editValue as Map).keys
        .whereType<String>()
        .toList();
    final next = <String, double>{};
    for (final key in keys) {
      final raw = _componentDrafts[key] ?? source?[key]?.toString() ?? '';
      final parsed = double.tryParse(raw.trim());
      if (parsed == null || !parsed.isFinite) {
        widget.drafts.remove(_pendingKey);
        widget.notifier.clearPendingEdit(_pendingKey);
        if (mounted) setState(() {});
        return;
      }
      next[key] = parsed;
    }
    _setPendingValue(next);
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final hit = widget.hit;
    final pending = widget.drafts.containsKey(_pendingKey);
    final sourceColor = switch (hit.source) {
      'metadata' => theme.colorScheme.outline,
      'public' => theme.colorScheme.secondary,
      _ => theme.colorScheme.primary,
    };
    final info = Row(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        SizedBox(width: (hit.depth * 8.0).clamp(0, 48)),
        Icon(_nodeIcon(hit.kind), size: 18, color: sourceColor),
        const SizedBox(width: 8),
        Expanded(
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              SelectionArea(
                child: Text(
                  hit.display,
                  maxLines: 3,
                  overflow: TextOverflow.ellipsis,
                ),
              ),
              const SizedBox(height: 4),
              Wrap(
                spacing: 5,
                runSpacing: 4,
                children: [
                  _NodeBadge(
                    label: hit.source.toUpperCase(),
                    color: sourceColor,
                  ),
                  _NodeBadge(label: hit.kind, color: theme.colorScheme.outline),
                  _NodeBadge(
                    label: hit.structType == null
                        ? hit.type
                        : '${hit.type} · ${hit.structType}',
                    color: theme.colorScheme.tertiary,
                  ),
                  if (hit.childCount > 0)
                    _NodeBadge(
                      label: AppLocalizations.of(
                        context,
                      ).allDataChildren(hit.childCount),
                      color: theme.colorScheme.secondary,
                    ),
                  if (pending)
                    _NodeBadge(
                      label: AppLocalizations.of(context).allDataPending,
                      color: theme.colorScheme.error,
                    ),
                ],
              ),
            ],
          ),
        ),
      ],
    );
    final value = _buildValueEditor(theme);
    return AnimatedContainer(
      duration: const Duration(milliseconds: 160),
      padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 10),
      decoration: BoxDecoration(
        color: pending
            ? theme.colorScheme.primaryContainer.withValues(alpha: .32)
            : theme.colorScheme.surfaceContainerLowest,
        borderRadius: BorderRadius.circular(12),
        border: Border.all(
          color: pending
              ? theme.colorScheme.primary.withValues(alpha: .45)
              : theme.colorScheme.outlineVariant,
        ),
      ),
      child: LayoutBuilder(
        builder: (context, constraints) {
          if (constraints.maxWidth < 760) {
            return Column(
              crossAxisAlignment: CrossAxisAlignment.stretch,
              children: [info, const SizedBox(height: 10), value],
            );
          }
          return Row(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Expanded(child: info),
              const SizedBox(width: 18),
              SizedBox(width: 380, child: value),
            ],
          );
        },
      ),
    );
  }

  Widget _buildValueEditor(ThemeData theme) {
    final hit = widget.hit;
    if (!widget.editable) {
      // SelectableText reserves its maxLines as field height. SelectionArea
      // keeps the value copyable while a regular Text grows only for lines
      // that are actually present.
      return SelectionArea(
        child: Text(
          hit.value.isEmpty && hit.childCount > 0
              ? AppLocalizations.of(context).allDataChildren(hit.childCount)
              : hit.value,
          maxLines: 4,
          overflow: TextOverflow.ellipsis,
          style: theme.textTheme.bodyMedium?.copyWith(
            color: theme.colorScheme.onSurfaceVariant,
            fontFamily: hit.kind == 'opaque'
                ? uiAwareMonospaceFontFamily(context, fallback: 'monospace')
                : null,
          ),
        ),
      );
    }
    if (_isBool) {
      return _BoolEditor(
        value:
            _boolDraft ??
            (widget.drafts[_pendingKey] as bool? ?? hit.editValue == true),
        onChanged: (next) {
          setState(() => _boolDraft = next);
          _setPendingValue(next);
        },
      );
    }
    if (_isVector) {
      final source = (widget.drafts[_pendingKey] ?? hit.editValue) as Map;
      final keys = (hit.editValue as Map).keys.whereType<String>().toList();
      return Wrap(
        spacing: 8,
        runSpacing: 8,
        children: [
          for (final name in keys)
            SizedBox(
              width: 82,
              child: TextFormField(
                key: ValueKey(
                  '${hit.stableId}-$name-${widget.reloadKey.hashCode}',
                ),
                initialValue:
                    _componentDrafts[name] ?? source[name]?.toString() ?? '',
                keyboardType: const TextInputType.numberWithOptions(
                  decimal: true,
                  signed: true,
                ),
                decoration: InputDecoration(
                  isDense: true,
                  labelText: name.toUpperCase(),
                ),
                onChanged: (value) => _updateVectorComponent(name, value),
              ),
            ),
        ],
      );
    }
    return TextField(
      controller: _controller,
      onChanged: _updatePending,
      minLines: _isTags ? 2 : 1,
      maxLines: _isTags ? 4 : 1,
      decoration: InputDecoration(
        isDense: true,
        labelText: AppLocalizations.of(context).value,
        helperText: _isTags
            ? AppLocalizations.of(context).allDataTagInputHint
            : null,
      ),
    );
  }

  static IconData _nodeIcon(String kind) => switch (kind) {
    'array' || 'set' || 'objectArray' => Icons.data_array,
    'map' => Icons.account_tree_outlined,
    'struct' ||
    'nativeStruct' ||
    'instancedStruct' => Icons.view_in_ar_outlined,
    'opaque' => Icons.hexagon_outlined,
    'mapEntry' ||
    'arrayElement' ||
    'setElement' ||
    'objectInstance' => Icons.subdirectory_arrow_right,
    _ => Icons.data_object,
  };
}

class _NodeBadge extends StatelessWidget {
  const _NodeBadge({required this.label, required this.color});

  final String label;
  final Color color;

  @override
  Widget build(BuildContext context) {
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 6, vertical: 2),
      decoration: BoxDecoration(
        color: color.withValues(alpha: .09),
        borderRadius: BorderRadius.circular(6),
      ),
      child: Text(
        label,
        style: Theme.of(context).textTheme.labelSmall?.copyWith(color: color),
      ),
    );
  }
}

class _BoolEditor extends StatelessWidget {
  const _BoolEditor({required this.value, required this.onChanged});

  final bool value;
  final ValueChanged<bool> onChanged;

  @override
  Widget build(BuildContext context) {
    return SizedBox(
      width: 220,
      child: Align(
        alignment: Alignment.centerRight,
        child: Switch(value: value, onChanged: onChanged),
      ),
    );
  }
}

class _BackupsPanel extends StatelessWidget {
  const _BackupsPanel({required this.state, required this.notifier});

  final EditorState state;
  final EditorNotifier notifier;

  @override
  Widget build(BuildContext context) {
    final backups = state.backups;
    final companionBackups = state.companionBackups;
    final l10n = AppLocalizations.of(context);
    return ListView(
      padding: const EdgeInsets.all(20),
      children: [
        Row(
          children: [
            const Icon(Icons.history),
            const SizedBox(width: 8),
            Text(
              l10n.backupsTitle,
              style: Theme.of(context).textTheme.titleLarge,
            ),
            const Spacer(),
            Tooltip(
              message: l10n.refreshBackups,
              child: IconButton(
                icon: const Icon(Icons.refresh),
                onPressed: state.isLoading ? null : notifier.refreshBackups,
              ),
            ),
          ],
        ),
        const SizedBox(height: 12),
        if (backups.isEmpty && companionBackups.isEmpty)
          _InlineNotice(
            icon: Icons.info_outline,
            title: l10n.noBackupsTitle,
            body: l10n.noBackupsBody,
          ),
        if (backups.isNotEmpty) ...[
          Text(
            l10n.slotBackups,
            style: Theme.of(context).textTheme.titleMedium,
          ),
          const SizedBox(height: 8),
          ...backups.map(
            (backup) => _BackupCard(
              backup: backup,
              isLoading: state.isLoading || state.deletedSaveRecovery != null,
              showRestoreAction: true,
              onRestore: () => notifier.restoreBackup(backup.path),
              onRename: (name) => notifier.renameBackup(backup.path, name),
              onDelete: () => notifier.deleteBackup(backup.path),
            ),
          ),
        ],
        if (companionBackups.isNotEmpty) ...[
          if (backups.isNotEmpty) const SizedBox(height: 8),
          Text(
            l10n.profileBackups,
            style: Theme.of(context).textTheme.titleMedium,
          ),
          const SizedBox(height: 8),
          ...companionBackups.map(
            (backup) => _BackupCard(
              backup: backup,
              isLoading: state.isLoading || state.deletedSaveRecovery != null,
              showRestoreAction: true,
              onRestore: () => notifier.restoreCompanionBackup(backup.path),
              onRename: (name) => notifier.renameBackup(backup.path, name),
              onDelete: () => notifier.deleteBackup(backup.path),
            ),
          ),
        ],
      ],
    );
  }
}

class _BackupCard extends StatelessWidget {
  const _BackupCard({
    required this.backup,
    required this.isLoading,
    required this.showRestoreAction,
    required this.onRestore,
    required this.onRename,
    required this.onDelete,
  });

  final BackupEntry backup;
  final bool isLoading;
  final bool showRestoreAction;
  final VoidCallback onRestore;

  /// Store a new label, or clear it with an empty string.
  final ValueChanged<String> onRename;
  final VoidCallback onDelete;

  @override
  Widget build(BuildContext context) {
    final canRestore = showRestoreAction && backup.canRestore;
    final l10n = AppLocalizations.of(context);
    return Padding(
      padding: const EdgeInsets.only(bottom: 12),
      child: Card(
        child: Padding(
          padding: const EdgeInsets.all(16),
          child: Row(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Icon(
                backup.status == 'ok'
                    ? Icons.restore_page_outlined
                    : Icons.warning_amber_outlined,
                color: backup.status == 'ok'
                    ? Theme.of(context).colorScheme.primary
                    : Colors.orange.shade800,
              ),
              const SizedBox(width: 12),
              Expanded(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Text(
                      // A label replaces the file name here; the file name
                      // stays readable in the facts below either way.
                      backup.title,
                      style: Theme.of(context).textTheme.titleMedium,
                      overflow: TextOverflow.ellipsis,
                    ),
                    const SizedBox(height: 6),
                    Wrap(
                      spacing: 14,
                      runSpacing: 6,
                      children: [
                        _SmallFact(
                          label: l10n.backupFactName,
                          value: backup.playerSaveName ?? '-',
                        ),
                        if (backup.slotName != null)
                          _SmallFact(
                            label: l10n.backupFactSlot,
                            value: backup.slotName!,
                          ),
                        _SmallFact(
                          label: l10n.backupFactFile,
                          value: backup.fileName,
                        ),
                        _SmallFact(
                          label: l10n.backupFactCreated,
                          value: _formatBackupTime(l10n, backup.createdEpoch),
                        ),
                        _SmallFact(
                          label: l10n.backupFactSize,
                          value: l10n.bytesValue(
                            NumberFormat.decimalPattern(
                              l10n.localeName,
                            ).format(backup.fileSize),
                          ),
                        ),
                        _SmallFact(
                          label: l10n.backupFactStatus,
                          value: _localizedBackupStatus(l10n, backup.status),
                        ),
                        _SmallFact(
                          label: l10n.backupFactSha1,
                          value: _shortSha(backup.sha1),
                        ),
                      ],
                    ),
                  ],
                ),
              ),
              const SizedBox(width: 12),
              Tooltip(
                message: l10n.renameBackupTooltip,
                child: IconButton(
                  icon: const Icon(Icons.edit_outlined),
                  onPressed: isLoading ? null : () => _rename(context, l10n),
                ),
              ),
              Tooltip(
                message: l10n.deleteBackupTooltip,
                child: IconButton(
                  icon: const Icon(Icons.delete_outline),
                  onPressed: isLoading ? null : () => _delete(context, l10n),
                ),
              ),
              if (showRestoreAction) ...[
                const SizedBox(width: 4),
                Tooltip(
                  message: l10n.restoreBackupTooltip(backup.title),
                  child: IconButton.filledTonal(
                    icon: const Icon(Icons.restore),
                    onPressed: isLoading || !canRestore ? null : onRestore,
                  ),
                ),
              ],
            ],
          ),
        ),
      ),
    );
  }

  /// Ask for a label and hand it to [onRename]. Prefilled with the current one;
  /// emptying the field clears it again. The backup file is never renamed — its
  /// name says which save it belongs to and when it was taken.
  Future<void> _rename(BuildContext context, AppLocalizations l10n) async {
    final name = await showDialog<String>(
      context: context,
      builder: (_) => _RenameBackupDialog(
        fileName: backup.fileName,
        initial: backup.name ?? '',
      ),
    );
    if (name != null) onRename(name);
  }

  /// Deleting a backup cannot be undone, so it is confirmed first — unlike the
  /// queued edits elsewhere, this one hits the disk right away.
  Future<void> _delete(BuildContext context, AppLocalizations l10n) async {
    final confirmed = await showDialog<bool>(
      context: context,
      builder: (dialogContext) => AlertDialog(
        title: Text(l10n.deleteBackupTitle),
        content: Text(l10n.deleteBackupBody(backup.title, backup.fileName)),
        actions: [
          TextButton(
            onPressed: () => Navigator.of(dialogContext).pop(false),
            child: Text(l10n.cancel),
          ),
          FilledButton(
            onPressed: () => Navigator.of(dialogContext).pop(true),
            child: Text(l10n.deleteBackupConfirm),
          ),
        ],
      ),
    );
    if (confirmed == true) onDelete();
  }
}

/// Asks for a backup's label. Owns its text controller so it lives exactly as
/// long as the dialog does — disposing it right after `showDialog` returns would
/// pull it out from under the field still on screen during the close animation.
///
/// Pops the entered text, or nothing when cancelled. An empty result is a
/// deliberate "clear the label", so it is NOT the same as cancelling.
class _RenameBackupDialog extends StatefulWidget {
  const _RenameBackupDialog({required this.fileName, required this.initial});

  final String fileName;
  final String initial;

  @override
  State<_RenameBackupDialog> createState() => _RenameBackupDialogState();
}

class _RenameBackupDialogState extends State<_RenameBackupDialog> {
  late final TextEditingController _controller = TextEditingController(
    text: widget.initial,
  );

  @override
  void dispose() {
    _controller.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context);
    return AlertDialog(
      title: Text(l10n.renameBackupTitle),
      // The helper names the backup file, which is long, and a dialog sized to
      // its title alone clipped it. Give it room to wrap and to be read.
      content: SizedBox(
        width: 460,
        child: TextField(
          controller: _controller,
          autofocus: true,
          decoration: InputDecoration(
            labelText: l10n.renameBackupLabel,
            helperText: l10n.renameBackupHelp(widget.fileName),
            helperMaxLines: 6,
          ),
          onSubmitted: (value) => Navigator.of(context).pop(value),
        ),
      ),
      actions: [
        TextButton(
          onPressed: () => Navigator.of(context).pop(),
          child: Text(l10n.cancel),
        ),
        FilledButton(
          onPressed: () => Navigator.of(context).pop(_controller.text),
          child: Text(l10n.save),
        ),
      ],
    );
  }
}

class _InlineNotice extends StatelessWidget {
  const _InlineNotice({
    required this.icon,
    required this.title,
    required this.body,
  });

  final IconData icon;
  final String title;
  final String body;

  @override
  Widget build(BuildContext context) {
    final scheme = Theme.of(context).colorScheme;
    return DecoratedBox(
      decoration: BoxDecoration(
        color: scheme.surfaceContainerLow,
        border: Border.all(color: scheme.outlineVariant),
        borderRadius: BorderRadius.circular(8),
      ),
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Row(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Icon(icon),
            const SizedBox(width: 12),
            Expanded(
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Text(title, style: Theme.of(context).textTheme.titleMedium),
                  const SizedBox(height: 4),
                  Text(body),
                ],
              ),
            ),
          ],
        ),
      ),
    );
  }
}

class _SmallFact extends StatelessWidget {
  const _SmallFact({required this.label, required this.value});

  final String label;
  final String value;

  @override
  Widget build(BuildContext context) {
    return SizedBox(
      width: 180,
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Text(
            label,
            style: Theme.of(context).textTheme.labelSmall?.copyWith(
              color: Theme.of(context).colorScheme.onSurfaceVariant,
            ),
          ),
          // Long values — a parse-failure status, say — are clipped to keep the
          // fact grid aligned, so the full text stays one hover away.
          Tooltip(
            message: value,
            child: Text(value, maxLines: 2, overflow: TextOverflow.ellipsis),
          ),
        ],
      ),
    );
  }
}

String _formatBackupTime(AppLocalizations l10n, int? epoch) {
  if (epoch == null) return '-';
  final dateTime = DateTime.fromMillisecondsSinceEpoch(
    epoch * 1000,
    isUtc: true,
  ).toLocal();
  return DateFormat.yMd(l10n.localeName).add_Hms().format(dateTime);
}

String _shortSha(String sha1) {
  if (sha1.length <= 12) return sha1;
  return sha1.substring(0, 12);
}

String _localizedBackupStatus(AppLocalizations l10n, String status) =>
    switch (status) {
      'ok' => l10n.statusOk,
      'failed' || 'error' => l10n.statusFailed,
      'invalid PersistentDataList structure' =>
        l10n.backupStatusInvalidProfileStructure,
      'selected slot metadata missing' => l10n.backupStatusSlotMetadataMissing,
      'unknown' || '' => l10n.statusUnknown,
      _ => l10n.backupStatusError(status),
    };

class _SettingsPanel extends StatelessWidget {
  const _SettingsPanel({required this.state, required this.notifier});

  final EditorState state;
  final EditorNotifier notifier;

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context);
    return ListView(
      padding: const EdgeInsets.all(20),
      children: [
        const AppearanceSettingsCard(),
        const SizedBox(height: 16),
        const UpdateSettingsCard(),
        const SizedBox(height: 16),
        const GameDataSettingsCard(),
        const SizedBox(height: 16),
        Card(
          child: Padding(
            padding: const EdgeInsets.all(16),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Row(
                  children: [
                    const Icon(Icons.folder_outlined),
                    const SizedBox(width: 8),
                    Text(
                      l10n.savegameDirectoryTitle,
                      style: Theme.of(context).textTheme.titleMedium,
                    ),
                  ],
                ),
                const SizedBox(height: 12),
                _PathSettingRow(
                  label: l10n.folder,
                  value: state.saveDir,
                  onBrowse: notifier.chooseSaveDir,
                ),
              ],
            ),
          ),
        ),
        const SizedBox(height: 16),
        _DebugSection(state: state, notifier: notifier),
      ],
    );
  }
}

class CodecStatusView extends StatelessWidget {
  const CodecStatusView({
    super.key,
    required this.codec,
    required this.codecError,
  });

  final CodecStatus? codec;
  final String? codecError;

  @override
  Widget build(BuildContext context) {
    final codec = this.codec;
    final scheme = Theme.of(context).colorScheme;
    final l10n = AppLocalizations.of(context);
    // A codec error (e.g. a failed roundtrip) can coexist with a status, so
    // render it whenever present -- both when there is no status and alongside
    // one.
    final error = codecError;
    final errorRow = error == null
        ? null
        : Row(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Icon(Icons.error_outline, color: scheme.error, size: 18),
              const SizedBox(width: 6),
              Expanded(
                child: Text(error, style: TextStyle(color: scheme.error)),
              ),
            ],
          );
    if (codec == null) {
      return errorRow ?? Text(l10n.noCodecStatus);
    }
    // The in-process codec maps to three states: ready (decode + encode),
    // decode_only (read but not write), and unavailable.
    final isReady = codec.status == 'ready' && codec.canCompress;
    final isDecodeOnly =
        !isReady && (codec.status == 'decode_only' || codec.canDecompress);
    final statusColor = isReady
        ? scheme.primary
        : isDecodeOnly
        ? scheme.tertiary
        : scheme.error;
    final statusIcon = isReady
        ? Icons.check_circle_outline
        : isDecodeOnly
        ? Icons.warning_amber_rounded
        : Icons.error_outline;
    final title = isReady
        ? l10n.codecReady
        : isDecodeOnly
        ? l10n.codecReadOnly
        : l10n.codecUnavailable;
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        if (errorRow != null) ...[errorRow, const SizedBox(height: 8)],
        Row(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Icon(statusIcon, size: 18, color: statusColor),
            const SizedBox(width: 6),
            Expanded(
              child: Text(
                title,
                style: TextStyle(color: isReady ? null : statusColor),
              ),
            ),
          ],
        ),
        const SizedBox(height: 8),
        ExpansionTile(
          tilePadding: EdgeInsets.zero,
          childrenPadding: const EdgeInsets.only(bottom: 8),
          title: Text(l10n.details),
          children: [
            Align(
              alignment: Alignment.centerLeft,
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Text(
                    l10n.codecStatusLine(
                      _localizedCodecStatus(l10n, codec.status),
                    ),
                  ),
                  Text(
                    l10n.codecCapabilityLine(
                      codec.canDecompress ? l10n.yes : l10n.no,
                      codec.canCompress ? l10n.yes : l10n.no,
                    ),
                  ),
                  Text(l10n.codecBackendLine(codec.adapter ?? codec.backend)),
                ],
              ),
            ),
          ],
        ),
      ],
    );
  }
}

String _localizedCodecStatus(AppLocalizations l10n, String status) =>
    switch (status) {
      'ready' => l10n.codecReady,
      'decode_only' => l10n.codecReadOnly,
      'unavailable' => l10n.codecUnavailable,
      'unknown' || '' => l10n.statusUnknown,
      _ => status,
    };

class _PathSettingRow extends StatelessWidget {
  const _PathSettingRow({
    required this.label,
    required this.value,
    required this.onBrowse,
  });

  final String label;
  final String value;
  final VoidCallback onBrowse;

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context);
    return Row(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        SizedBox(
          width: 84,
          child: Padding(
            padding: const EdgeInsets.only(top: 8),
            child: Text(label, style: Theme.of(context).textTheme.labelLarge),
          ),
        ),
        Expanded(
          child: Container(
            constraints: const BoxConstraints(minHeight: 40),
            padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 8),
            decoration: BoxDecoration(
              border: Border.all(
                color: Theme.of(context).colorScheme.outlineVariant,
              ),
              borderRadius: BorderRadius.circular(8),
            ),
            child: SelectableText(
              value.isEmpty ? '-' : value,
              maxLines: 2,
              style: TextStyle(
                fontFamily: uiAwareMonospaceFontFamily(context),
                fontSize: 12,
              ),
            ),
          ),
        ),
        const SizedBox(width: 8),
        IconButton(
          tooltip: l10n.browse,
          icon: const Icon(Icons.folder_open),
          onPressed: onBrowse,
        ),
      ],
    );
  }
}

class _MessagePane extends StatelessWidget {
  const _MessagePane({
    required this.icon,
    required this.title,
    required this.body,
  });

  final IconData icon;
  final String title;
  final String body;

  @override
  Widget build(BuildContext context) {
    return Center(
      child: ConstrainedBox(
        constraints: const BoxConstraints(maxWidth: 520),
        child: Card(
          child: Padding(
            padding: const EdgeInsets.all(24),
            child: Column(
              mainAxisSize: MainAxisSize.min,
              children: [
                Icon(
                  icon,
                  size: 48,
                  color: Theme.of(context).colorScheme.primary,
                ),
                const SizedBox(height: 12),
                Text(title, style: Theme.of(context).textTheme.titleLarge),
                const SizedBox(height: 8),
                Text(body, textAlign: TextAlign.center),
              ],
            ),
          ),
        ),
      ),
    );
  }
}
