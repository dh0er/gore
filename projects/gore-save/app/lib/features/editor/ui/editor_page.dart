import 'dart:async';
import 'dart:convert';

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
import 'package:goresave/features/editor/domain/item_categories.dart';
import 'package:goresave/features/editor/ui/sidebar_tile.dart';
import 'package:goresave/features/editor/domain/pending_edits.dart';
import 'package:goresave/features/localization/domain/localization_controller.dart';
import 'package:goresave/features/localization/ui/localization_flow.dart';
import 'package:goresave/features/localization/ui/localization_settings.dart';
import 'package:goresave/providers/data_providers.dart';
import 'package:intl/intl.dart';
import 'add_inventory_item_dialog.dart';
import 'difficulty_dialog.dart';
import 'hero_stats_card.dart';
import 'progression_panel.dart';

final _bytes = NumberFormat.decimalPattern();

class EditorPage extends ConsumerStatefulWidget {
  const EditorPage({super.key});

  @override
  ConsumerState<EditorPage> createState() => _EditorPageState();
}

class _EditorPageState extends ConsumerState<EditorPage> {
  @override
  void initState() {
    super.initState();
    // First-run, optional localized-text extraction prompt. Runs after the
    // first frame so the Scaffold (SnackBar host) and Navigator (dialog host)
    // exist. Guarded by a persisted flag so it only auto-prompts once; the
    // manual Settings button stays available regardless.
    WidgetsBinding.instance.addPostFrameCallback((_) {
      unawaited(_maybePromptLocalizationExtract());
    });
  }

  Future<void> _maybePromptLocalizationExtract() async {
    final store = ref.read(uiSettingsStoreProvider);
    if (store.read().locExtractPrompted) return;

    final present = await ref
        .read(localizationControllerProvider.notifier)
        .status();
    if (present || !mounted) return;

    // Mark prompted up front so a cancel (or a close mid-extract) doesn't make
    // the dialog reappear on the next launch.
    store.write(store.read().copyWith(locExtractPrompted: true));

    final confirmed = await showDialog<bool>(
      context: context,
      builder: (context) => AlertDialog(
        title: const Text('Extract localized game text?'),
        content: const Text(
          "Localized game text isn't extracted yet. Extract it now from "
          'your game install? (optional)',
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.of(context).pop(false),
            child: const Text('Not now'),
          ),
          FilledButton(
            onPressed: () => Navigator.of(context).pop(true),
            child: const Text('Extract'),
          ),
        ],
      ),
    );
    if (confirmed != true || !mounted) return;
    await runLocalizationExtractFlow(context, ref);
  }

  @override
  Widget build(BuildContext context) {
    final state = ref.watch(editorProvider);
    final notifier = ref.read(editorProvider.notifier);
    final uiScale = ref.watch(uiScaleProvider);
    final zoomPct = (uiScale * 100).round();
    final scheme = Theme.of(context).colorScheme;
    final isDark = Theme.of(context).brightness == Brightness.dark;

    return Scaffold(
      // The AppBar doubles as the window title bar: dragging the empty space
      // moves the window, double-click toggles maximize/restore.
      appBar: AppBar(
        title: WindowDragArea(
          child: Row(
            children: [
              const SizedBox(width: 16),
              Image.asset(
                'assets/goresave_icon.png',
                height: 32,
                semanticLabel: 'goresave logo',
              ),
              const SizedBox(width: 10),
              // Flexible + ellipsis: at narrow window widths the long title
              // must truncate instead of overflowing the title bar row.
              const Flexible(
                child: Text(
                  'Gothic Remake Savegame Editor',
                  maxLines: 1,
                  overflow: TextOverflow.ellipsis,
                ),
              ),
              const Expanded(child: SizedBox()),
            ],
          ),
        ),
        titleSpacing: 0,
        centerTitle: false,
        scrolledUnderElevation: 0,
        surfaceTintColor: Colors.transparent,
        actions: [
          const SizedBox(width: 8),
          Tooltip(
            message: 'Press Ctrl +/- to zoom in/out',
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
            tooltip: isDark ? 'Switch to light mode' : 'Switch to dark mode',
          ),
          IconButton(
            icon: const Icon(Icons.info_outline),
            onPressed: () {
              showDialog(
                context: context,
                builder: (_) => const GoresaveAboutDialog(),
              );
            },
            tooltip: 'About',
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
    return DecoratedBox(
      decoration: BoxDecoration(
        color: Theme.of(context).colorScheme.surfaceContainerLow,
      ),
      child: Column(
        children: [
          _ProfileHeader(
            profile: state.activeProfile,
            profiles: state.profiles,
            notifier: notifier,
            isLoading: state.isLoading,
          ),
          Expanded(
            child: saves.isEmpty
                ? const Center(
                    child: Padding(
                      padding: EdgeInsets.all(24),
                      child: Text(
                        'No .sav files found',
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
                      return Padding(
                        padding: const EdgeInsets.symmetric(horizontal: 8),
                        child: _SaveSlotCard(
                          save: save,
                          selected: selected,
                          enabled: !state.isLoading,
                          onTap: () => notifier.inspect(save.path),
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
    required this.notifier,
    required this.isLoading,
  });

  final ProfileSummary? profile;
  final List<ProfileSummary> profiles;
  final EditorNotifier notifier;
  final bool isLoading;

  @override
  Widget build(BuildContext context) {
    final textTheme = Theme.of(context).textTheme;
    final scheme = Theme.of(context).colorScheme;
    final multiProfile = profiles.length > 1;
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
            child: Icon(Icons.person_outline, color: scheme.primary),
          ),
          const SizedBox(width: 12),
          Expanded(
            child: Row(
              children: [
                Flexible(
                  child: multiProfile
                      ? _ProfileSwitcher(
                          profile: profile,
                          profiles: profiles,
                          notifier: notifier,
                          isLoading: isLoading,
                        )
                      : Text(
                          profile?.displayName ?? 'Profile',
                          maxLines: 1,
                          overflow: TextOverflow.ellipsis,
                          style: textTheme.titleMedium,
                        ),
                ),
                const SizedBox(width: 12),
                Flexible(
                  child: ProfileDifficultyChip(
                    profile: profile,
                    notifier: notifier,
                    isLoading: isLoading,
                  ),
                ),
              ],
            ),
          ),
          IconButton(
            icon: const Icon(Icons.refresh),
            tooltip: 'Rescan save folder',
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
    if (notifier.state.hasUnsavedEdits) {
      final pendingCount = notifier.pendingEditCount;
      final confirmed = await showDialog<bool>(
        context: context,
        builder: (context) => AlertDialog(
          title: const Text('Discard unsaved changes?'),
          content: Text(
            'Rescanning reloads every save and discards your $pendingCount '
            'unsaved change${pendingCount == 1 ? '' : 's'}.',
          ),
          actions: [
            TextButton(
              onPressed: () => Navigator.of(context).pop(false),
              child: const Text('Cancel'),
            ),
            FilledButton(
              onPressed: () => Navigator.of(context).pop(true),
              child: const Text('Discard and rescan'),
            ),
          ],
        ),
      );
      if (confirmed != true) return;
      // The user chose to discard. refresh() centrally clears all pending edits
      // (registry + the pending difficulty edit) and re-seeds the editors.
    }
    await notifier.refresh();
  }
}

/// Profile name shown as a [PopupMenuButton] when multiple profiles exist.
/// Selecting a profile calls [EditorNotifier.selectProfile].
class _ProfileSwitcher extends StatelessWidget {
  const _ProfileSwitcher({
    required this.profile,
    required this.profiles,
    required this.notifier,
    required this.isLoading,
  });

  final ProfileSummary? profile;
  final List<ProfileSummary> profiles;
  final EditorNotifier notifier;
  final bool isLoading;

  @override
  Widget build(BuildContext context) {
    final textTheme = Theme.of(context).textTheme;
    final scheme = Theme.of(context).colorScheme;
    final currentId = profile?.profileId;
    return PopupMenuButton<int>(
      tooltip: 'Switch profile',
      enabled: !isLoading,
      onSelected: (id) => notifier.selectProfile(id),
      itemBuilder: (context) => [
        for (final p in profiles)
          PopupMenuItem<int>(
            value: p.profileId,
            child: Row(
              children: [
                if (p.profileId == currentId)
                  Icon(Icons.check, size: 18, color: scheme.primary)
                else
                  const SizedBox(width: 18),
                const SizedBox(width: 8),
                Expanded(
                  child: Text(
                    '${p.displayName} (${p.savedSlots.length} saves)',
                    overflow: TextOverflow.ellipsis,
                  ),
                ),
              ],
            ),
          ),
      ],
      child: Row(
        mainAxisSize: MainAxisSize.min,
        children: [
          Flexible(
            child: Text(
              profile?.displayName ?? 'Profile',
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
  });

  final SaveSlot save;
  final bool selected;
  final bool enabled;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    final scheme = Theme.of(context).colorScheme;
    final accent = selected ? scheme.primary : scheme.outline;
    return Material(
      color: selected ? scheme.primaryContainer : scheme.surfaceContainerLowest,
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
                child: _ScreenshotPreview(
                  screenshot: save.screenshot,
                  slot: save.slot,
                  compact: true,
                ),
              ),
              const SizedBox(width: 10),
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
                          child: _SaveKindIcon(
                            quickSave: save.quickSave,
                            autoSave: save.autoSave,
                            selected: selected,
                          ),
                        ),
                        const SizedBox(width: 6),
                        Expanded(
                          child: Text(
                            save.displayName,
                            maxLines: 3,
                            overflow: TextOverflow.ellipsis,
                            style: Theme.of(context).textTheme.titleSmall,
                          ),
                        ),
                      ],
                    ),
                    const SizedBox(height: 4),
                    Text(
                      _saveSlotSubtitle(save),
                      maxLines: 2,
                      overflow: TextOverflow.ellipsis,
                      style: Theme.of(context).textTheme.bodySmall?.copyWith(
                        color: scheme.onSurfaceVariant,
                      ),
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
    required this.selected,
  });

  final bool? quickSave;
  final bool? autoSave;
  final bool selected;

  @override
  Widget build(BuildContext context) {
    final label = _formatSaveKind(quickSave: quickSave, autoSave: autoSave);
    if (label == '-') return const SizedBox(height: 16);
    final icon = quickSave == true
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

String _saveSlotSubtitle(SaveSlot save) {
  final parts = <String>[];
  if (save.chapterId != null) {
    parts.add('Chapter ${save.chapterId}');
  }
  final timePlayed = _formatDurationSeconds(save.timePlayedSeconds);
  if (timePlayed != '-') {
    parts.add(timePlayed);
  }
  return parts.join(' | ');
}

String _formatDurationSeconds(double? seconds) {
  if (seconds == null || seconds.isNaN || seconds.isInfinite) return '-';
  final totalMinutes = (seconds < 0 ? 0 : seconds / 60).floor();
  final hours = totalMinutes ~/ 60;
  final minutes = totalMinutes % 60;
  if (hours <= 0) return '${minutes}m';
  if (minutes == 0) return '${hours}h';
  return '${hours}h ${minutes}m';
}

String _formatSaveKind({required bool? quickSave, required bool? autoSave}) {
  if (quickSave == true) return 'Quick save';
  if (autoSave == true) return 'Auto save';
  if (quickSave == false || autoSave == false) return 'Manual save';
  return '-';
}

class _EditorWorkspace extends StatelessWidget {
  const _EditorWorkspace({required this.state, required this.notifier});

  final EditorState state;
  final EditorNotifier notifier;

  @override
  Widget build(BuildContext context) {
    final scheme = Theme.of(context).colorScheme;
    Widget content;
    if (state.inspection == null) {
      content = state.error != null
          ? _MessagePane(
              icon: Icons.error_outline,
              title: 'Error',
              body: state.error!,
            )
          : const _MessagePane(
              icon: Icons.search,
              title: 'Select a save',
              body: 'The save details will appear here.',
            );
    } else {
      final inspection = state.inspection!;
      final pendingCount = state.pendingEditCount;
      content = DefaultTabController(
        length: 7,
        child: Column(
          children: [
            Container(
              color: scheme.surfaceContainerLowest,
              child: Row(
                children: [
                  const Expanded(
                    child: TabBar(
                      isScrollable: true,
                      tabs: [
                        Tab(
                          icon: Icon(Icons.dashboard_outlined),
                          text: 'Overview',
                        ),
                        Tab(icon: Icon(Icons.person_outline), text: 'Player'),
                        Tab(
                          icon: Icon(Icons.inventory_2_outlined),
                          text: 'Inventory',
                        ),
                        Tab(
                          icon: Icon(Icons.flag_outlined),
                          text: 'Progression',
                        ),
                        Tab(icon: Icon(Icons.tune), text: 'All data'),
                        Tab(icon: Icon(Icons.history), text: 'Backups'),
                        Tab(
                          icon: Icon(Icons.settings_outlined),
                          text: 'Settings',
                        ),
                      ],
                    ),
                  ),
                  Padding(
                    padding: const EdgeInsets.only(right: 8),
                    child: OutlinedButton.icon(
                      icon: const Icon(Icons.undo),
                      label: const Text('Reset'),
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
                        pendingCount == 0 ? 'Save' : 'Save ($pendingCount)',
                      ),
                      onPressed: pendingCount > 0 && !state.isLoading
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
                    child: const Text('OK'),
                  ),
                ],
              ),
            if (state.lastWriteMessage != null)
              MaterialBanner(
                leading: const Icon(Icons.check_circle_outline),
                content: Text(state.lastWriteMessage!),
                actions: [
                  TextButton(
                    onPressed: notifier.dismissWriteMessage,
                    child: const Text('OK'),
                  ),
                ],
              ),
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
                    child: _PrivatePanel(
                      icon: Icons.person_outline,
                      title: 'Player',
                      inspection: inspection,
                      notifier: notifier,
                      // Private writes recompress the payload, so also require the
                      // codec to be compress-ready, not just decode-ready.
                      editable:
                          inspection.privateEditable &&
                          state.codecCompressReady,
                      lockedBody:
                          'Private player edits need a compress-ready codec.',
                    ),
                  ),
                  _KeepAliveTab(
                    child: _InventoryPanel(
                      inspection: inspection,
                      notifier: notifier,
                      canCompress: state.codecCompressReady,
                    ),
                  ),
                  _KeepAliveTab(
                    child: ProgressionPanel(
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
                  label: 'Loading editor data',
                  child: const SizedBox(
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
        _HeaderCard(inspection: inspection, save: state.selectedSave),
        const SizedBox(height: 16),
        _MetadataEditor(inspection: inspection, notifier: notifier),
        const SizedBox(height: 16),
        _OverviewDiagnostics(inspection: inspection),
        const SizedBox(height: 16),
        _OverviewInspectionJson(inspection: inspection),
      ],
    );
  }
}

class _OverviewInspectionJson extends StatefulWidget {
  const _OverviewInspectionJson({required this.inspection});

  final SaveInspection inspection;

  @override
  State<_OverviewInspectionJson> createState() =>
      _OverviewInspectionJsonState();
}

class _OverviewInspectionJsonState extends State<_OverviewInspectionJson> {
  bool _expanded = false;
  String? _cachedJson;

  @override
  void didUpdateWidget(covariant _OverviewInspectionJson oldWidget) {
    super.didUpdateWidget(oldWidget);
    // Every refresh/save produces a new SaveInspection instance; a cached
    // pretty-print of the old one would show (and copy) stale data.
    if (!identical(widget.inspection, oldWidget.inspection)) {
      _cachedJson = null;
    }
  }

  String _getJson() {
    _cachedJson ??= widget.inspection.prettyJson();
    return _cachedJson!;
  }

  @override
  Widget build(BuildContext context) {
    return Card(
      child: Padding(
        padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 8),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            _CollapsibleCardHeader(
              icon: Icons.data_object,
              title: 'Inspection JSON',
              subtitle: 'Raw save inspection data',
              expanded: _expanded,
              onToggle: () => setState(() => _expanded = !_expanded),
            ),
            if (_expanded) ...[
              const SizedBox(height: 8),
              Row(
                mainAxisAlignment: MainAxisAlignment.end,
                children: [
                  IconButton(
                    tooltip: 'Copy',
                    icon: const Icon(Icons.copy),
                    onPressed: () =>
                        Clipboard.setData(ClipboardData(text: _getJson())),
                  ),
                ],
              ),
              SelectableText(
                _getJson(),
                style: const TextStyle(fontFamily: 'Consolas', fontSize: 12),
              ),
            ],
          ],
        ),
      ),
    );
  }
}

class _OverviewDiagnostics extends StatefulWidget {
  const _OverviewDiagnostics({required this.inspection});

  final SaveInspection inspection;

  @override
  State<_OverviewDiagnostics> createState() => _OverviewDiagnosticsState();
}

class _OverviewDiagnosticsState extends State<_OverviewDiagnostics> {
  bool _expanded = false;

  @override
  Widget build(BuildContext context) {
    final inspection = widget.inspection;
    return Card(
      child: Padding(
        padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 8),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            _CollapsibleCardHeader(
              icon: Icons.science_outlined,
              title: 'Diagnostics & details',
              subtitle: 'Read-only format inspection',
              expanded: _expanded,
              onToggle: () => setState(() => _expanded = !_expanded),
            ),
            if (_expanded) ...[
              const SizedBox(height: 8),
              Row(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Expanded(
                    child: _MetricGrid(
                      metrics: {
                        'Format': inspection.format,
                        'Slot': inspection.slot ?? '-',
                        if (inspection.chapterId != null)
                          'Chapter': inspection.chapterId.toString(),
                        if (inspection.timePlayedSeconds != null)
                          'Time played': _formatDurationSeconds(
                            inspection.timePlayedSeconds,
                          ),
                        if (inspection.quickSave != null ||
                            inspection.autoSave != null)
                          'Save kind': _formatSaveKind(
                            quickSave: inspection.quickSave,
                            autoSave: inspection.autoSave,
                          ),
                        'File size': '${_bytes.format(inspection.size)} bytes',
                        'Compression': inspection.compressionMethod ?? '-',
                        'Chunks': inspection.chunkCount?.toString() ?? '-',
                        'Uncompressed': inspection.uncompressedSize == null
                            ? '-'
                            : '${_bytes.format(inspection.uncompressedSize)} bytes',
                        'Private': inspection.privateStatus ?? '-',
                      },
                    ),
                  ),
                  const SizedBox(width: 16),
                  Expanded(
                    child: _MetricGrid(
                      metrics: {
                        'Slot name': inspection.slotName ?? '-',
                        'Trailer': inspection.trailerSize == null
                            ? '-'
                            : '${inspection.trailerSize} bytes',
                        'Decoded private':
                            inspection.privateDecompressedSize == null
                            ? '-'
                            : '${_bytes.format(inspection.privateDecompressedSize)} bytes',
                        'Private strings':
                            inspection.privateStringCount?.toString() ?? '-',
                        'SHA-1': inspection.sha1,
                      },
                    ),
                  ),
                ],
              ),
            ],
          ],
        ),
      ),
    );
  }
}

class _HeaderCard extends StatelessWidget {
  const _HeaderCard({required this.inspection, this.save});

  final SaveInspection inspection;
  final SaveSlot? save;

  @override
  Widget build(BuildContext context) {
    final screenshot = save?.screenshot ?? inspection.screenshot;
    final title =
        save?.displayName ??
        inspection.playerSaveName ??
        inspection.slot ??
        'Savegame';
    final slot = save?.slot ?? inspection.slot ?? 'Savegame';
    return Card(
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: LayoutBuilder(
          builder: (context, constraints) {
            final compact = constraints.maxWidth < 560;
            final previewWidth = compact ? 170.0 : 320.0;
            final previewHeight = previewWidth * 9 / 16;
            return Row(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                SizedBox(
                  width: previewWidth,
                  height: previewHeight,
                  child: _ScreenshotPreview(
                    screenshot: screenshot,
                    slot: slot,
                    compact: compact,
                  ),
                ),
                const SizedBox(width: 14),
                Expanded(
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      Row(
                        children: [
                          Icon(
                            Icons.save_outlined,
                            size: 28,
                            color: Theme.of(context).colorScheme.primary,
                          ),
                          const SizedBox(width: 10),
                          Expanded(
                            child: Text(
                              title,
                              style: Theme.of(context).textTheme.titleLarge,
                              maxLines: 1,
                              overflow: TextOverflow.ellipsis,
                            ),
                          ),
                        ],
                      ),
                      const SizedBox(height: 4),
                      Text(
                        inspection.path ?? '',
                        maxLines: 1,
                        overflow: TextOverflow.ellipsis,
                        style: Theme.of(context).textTheme.bodySmall,
                      ),
                      Builder(
                        builder: (context) {
                          final pills = <Widget>[
                            if (inspection.chapterId != null)
                              _InfoPill(
                                icon: Icons.flag_outlined,
                                label: 'Chapter ${inspection.chapterId}',
                              ),
                            if (inspection.timePlayedSeconds != null)
                              _InfoPill(
                                icon: Icons.timer_outlined,
                                label: _formatDurationSeconds(
                                  inspection.timePlayedSeconds,
                                ),
                              ),
                          ];
                          if (pills.isEmpty) return const SizedBox.shrink();
                          return Padding(
                            padding: const EdgeInsets.only(top: 9),
                            child: Wrap(
                              spacing: 8,
                              runSpacing: 8,
                              children: pills,
                            ),
                          );
                        },
                      ),
                    ],
                  ),
                ),
              ],
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
              semanticLabel: 'Screenshot for $slot',
              errorBuilder: (_, _, _) => placeholder,
            ),
    );
  }
}

class _InfoPill extends StatelessWidget {
  const _InfoPill({required this.icon, required this.label});

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
            Text(label, style: Theme.of(context).textTheme.labelMedium),
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

class _MetadataEditor extends StatefulWidget {
  const _MetadataEditor({required this.inspection, required this.notifier});

  final SaveInspection inspection;
  final EditorNotifier notifier;

  @override
  State<_MetadataEditor> createState() => _MetadataEditorState();
}

class _MetadataEditorState extends State<_MetadataEditor> {
  late final TextEditingController _controller;
  Object? _inspectionIdentity;
  String? _path;
  String? _name;
  String? _error;

  @override
  void initState() {
    super.initState();
    _controller = TextEditingController();
    _sync();
  }

  @override
  void didUpdateWidget(covariant _MetadataEditor oldWidget) {
    super.didUpdateWidget(oldWidget);
    _sync();
  }

  @override
  void dispose() {
    _controller.dispose();
    super.dispose();
  }

  void _sync() {
    final name = widget.inspection.playerSaveName ?? '';
    // Re-seed whenever the inspection identity changes (e.g. after a Reset /
    // refresh that produces a new SaveInspection instance) or when path/name
    // changes. This ensures that after a Reset the field visually reverts to
    // the canonical value even if the canonical value itself did not change.
    final sameIdentity = identical(widget.inspection, _inspectionIdentity);
    if (sameIdentity && _path == widget.inspection.path && _name == name) {
      return;
    }
    _inspectionIdentity = widget.inspection;
    _path = widget.inspection.path;
    _name = name;
    _controller.text = name;
    // Do NOT call _updatePending here: refresh() centrally clears all pending
    // edits in the notifier (event-handler context). Calling clearPendingEdit /
    // setPendingEdit from initState / didUpdateWidget mutates the provider
    // during build and throws with flutter_riverpod. The field is re-seeded
    // from the canonical value above; the next user keystroke (onChanged) will
    // re-register a pending edit if needed.
    setState(() => _error = null);
  }

  void _updatePending(String fieldText) {
    final value = fieldText.trim();
    if (value.isEmpty) {
      setState(() => _error = 'Required');
      widget.notifier.clearPendingEdit('publicName');
      return;
    }
    final original = widget.inspection.playerSaveName ?? '';
    setState(() => _error = null);
    if (value == original) {
      widget.notifier.clearPendingEdit('publicName');
    } else {
      widget.notifier.setPendingEdit(
        'publicName',
        PendingSaveEdit(
          edits: [
            {'path': 'public.m_PlayerSaveName', 'value': value},
          ],
          syncPersistentDataList: true,
        ),
      );
    }
  }

  @override
  Widget build(BuildContext context) {
    return Card(
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: TextField(
          controller: _controller,
          decoration: InputDecoration(
            labelText: 'Public save name',
            prefixIcon: const Icon(Icons.edit_outlined),
            errorText: _error,
          ),
          onChanged: _updatePending,
        ),
      ),
    );
  }
}

class _MetricGrid extends StatelessWidget {
  const _MetricGrid({required this.metrics});

  final Map<String, String> metrics;

  @override
  Widget build(BuildContext context) {
    // No Card here: callers place this inside their own card, so an extra card
    // layer just doubles the padding.
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: metrics.entries
          .map(
            (entry) => Padding(
              padding: const EdgeInsets.symmetric(vertical: 3),
              child: Row(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  SizedBox(
                    width: 130,
                    child: Text(
                      entry.key,
                      style: TextStyle(
                        color: Theme.of(context).colorScheme.onSurfaceVariant,
                      ),
                    ),
                  ),
                  // No maxLines: a multiline cap makes SelectableText reserve
                  // that many lines of height even for one-line values.
                  Expanded(child: SelectableText(entry.value)),
                ],
              ),
            ),
          )
          .toList(),
    );
  }
}

class _PrivatePanel extends StatelessWidget {
  const _PrivatePanel({
    required this.icon,
    required this.title,
    required this.inspection,
    required this.notifier,
    required this.editable,
    required this.lockedBody,
  });

  final IconData icon;
  final String title;
  final SaveInspection inspection;
  final EditorNotifier notifier;
  final bool editable;
  final String lockedBody;

  Widget _legacyAttributesCard() {
    return Card(
      child: Padding(
        padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 12),
        child: _PrivatePlayerAttributesEditor(
          player: inspection.privatePlayer,
          notifier: notifier,
          editable: editable,
          reloadKey: inspection,
        ),
      ),
    );
  }

  Widget? _transformCard() {
    if (inspection.privatePlayer.transform == null) return null;
    return Card(
      child: Padding(
        padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 12),
        child: _PrivatePlayerTransformEditor(
          transform: inspection.privatePlayer.transform!,
          editable:
              editable &&
              inspection.privatePlayer.writable.contains(
                'private.player.setTransform',
              ),
          notifier: notifier,
          reloadKey: inspection,
        ),
      ),
    );
  }

  @override
  Widget build(BuildContext context) {
    if (inspection.privateDecoded) {
      // Typed path: HeroStatsCard manages its own internal scroll for the
      // detail area and pins the sidebar. Give it the full pane via Padding
      // (not ListView) so it has a finite height to work with.
      if (title == 'Player' && inspection.privateTypedVerified) {
        return Padding(
          padding: const EdgeInsets.all(20),
          child: HeroStatsCard(
            // New SaveInspection instance after every write/refresh —
            // changing identity drops pending edits and reloads.
            reloadKey: inspection,
            load: notifier.loadHeroAttributes,
            onPendingChanged: (edits, validationError) {
              if (edits.isEmpty || validationError != null) {
                notifier.clearPendingEdit('heroStats');
              } else {
                notifier.setPendingEdit(
                  'heroStats',
                  PendingSaveEdit(
                    edits: [
                      for (final edit in edits)
                        {
                          'path': 'private.typed.setValue',
                          'value': {'path': edit.path, 'value': edit.value},
                        },
                    ],
                  ),
                );
              }
            },
            editable: editable,
            // Spec: if the typed search errors out or finds nothing on a
            // typed-OK save, the heuristic editor stays available.
            fallback: inspection.privatePlayer.attributes.isNotEmpty
                ? _legacyAttributesCard()
                : null,
            transformCard: _transformCard(),
          ),
        );
      }
      // Legacy / non-typed path: stacked layout in a ListView.
      return ListView(
        padding: const EdgeInsets.all(20),
        children: [
          if (title == 'Player') ...[
            // Typed parse failed or not verified: stacked legacy layout —
            // no sidebar, no typed load call.
            if (inspection.privatePlayer.attributes.isNotEmpty) ...[
              _legacyAttributesCard(),
              const SizedBox(height: 16),
            ],
            if (inspection.privatePlayer.transform != null) ...[
              _transformCard()!,
              const SizedBox(height: 16),
            ],
          ],
        ],
      );
    }
    return _MessagePane(icon: icon, title: title, body: lockedBody);
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

class _InventoryPanel extends StatelessWidget {
  const _InventoryPanel({
    required this.inspection,
    required this.notifier,
    this.canCompress = false,
  });

  final SaveInspection inspection;
  final EditorNotifier notifier;
  final bool canCompress;

  @override
  Widget build(BuildContext context) {
    if (!inspection.privateDecoded) {
      return const _MessagePane(
        icon: Icons.inventory_2_outlined,
        title: 'Inventory',
        body:
            'Inventory editing needs decoded private payload data from the codec.',
      );
    }
    // Inventory writes recompress the payload too, so require a
    // compress-capable codec host in addition to a full decode.
    // The core only allows count edits in a detected player
    // inventory region, advertised via writable; gate on it so other
    // scopes don't show editors whose saves fail in the core.
    final writable = inspection.privateInventory.writable;
    final editable =
        inspection.privateEditable &&
        canCompress &&
        writable.contains('private.inventory.setItemCount');
    // addItem/removeItem edit the typed property tree, so both require a
    // verified typed parse plus their own advertised op. They are gated
    // independently: the core can expose one without the other (e.g.
    // removeItem with no clean template for adds), and addItem can be valid even
    // when the FString scan found no stacks (empty/unscanned MainContainer).
    final canAddItem =
        inspection.privateEditable &&
        canCompress &&
        inspection.privateTypedVerified &&
        writable.contains('private.inventory.addItem');
    final canRemoveItem =
        inspection.privateEditable &&
        canCompress &&
        inspection.privateTypedVerified &&
        writable.contains('private.inventory.removeItem');

    final hasItems = inspection.privateInventory.hasData;
    if (!hasItems && !canAddItem && !canRemoveItem) {
      // Decoded fine, nothing recognised and nothing addable — say so instead
      // of leaving the tab blank.
      return const _MessagePane(
        icon: Icons.inventory_2_outlined,
        title: 'Inventory',
        body: 'No item stacks found in the decoded private payload.',
      );
    }
    return Padding(
      padding: const EdgeInsets.all(20),
      child: _PrivateInventorySummaryCard(
        inventory: inspection.privateInventory,
        notifier: notifier,
        editable: editable,
        canAddItem: canAddItem,
        canRemoveItem: canRemoveItem,
      ),
    );
  }
}

class _PrivateInventorySummaryCard extends StatefulWidget {
  const _PrivateInventorySummaryCard({
    required this.inventory,
    required this.notifier,
    this.editable = true,
    this.canAddItem = false,
    this.canRemoveItem = false,
  });

  final PrivateInventorySummary inventory;
  final EditorNotifier notifier;
  final bool editable;
  final bool canAddItem;
  final bool canRemoveItem;

  @override
  State<_PrivateInventorySummaryCard> createState() =>
      _PrivateInventorySummaryCardState();
}

class _PrivateInventorySummaryCardState
    extends State<_PrivateInventorySummaryCard> {
  String _query = '';
  final TextEditingController _searchController = TextEditingController();
  final Map<String, InventoryItemCountChange> _pendingCountChanges = {};
  ItemCategory? _selectedCategory;
  InventoryItemAdd? _pendingAdd;
  // Path of the item queued for removal. addItem and removeItem are both
  // structural edits, so at most one of _pendingAdd / _pendingRemovePath is
  // ever set (the core allows one structural inventory edit per write).
  String? _pendingRemovePath;

  @override
  void dispose() {
    _searchController.dispose();
    super.dispose();
  }

  @override
  void didUpdateWidget(covariant _PrivateInventorySummaryCard oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.inventory != widget.inventory) {
      // Inventory was refreshed — clear local widget state. The notifier
      // centrally clears 'inventory' in refresh() (event-handler context),
      // so mutating the provider here would throw during build.
      _pendingCountChanges.clear();
      _pendingAdd = null;
      _pendingRemovePath = null;
    }
  }

  void _pushInventoryPending() {
    final countEdits =
        _pendingCountChanges.values.map((c) => c.toEditJson()).toList();
    final addEdit = _pendingAdd?.toEditJson();
    final removeEdit = _pendingRemovePath != null
        ? InventoryItemRemove(path: _pendingRemovePath!).toEditJson()
        : null;
    final allEdits = [
      ...countEdits,
      ?addEdit,
      ?removeEdit,
    ];
    if (allEdits.isEmpty) {
      widget.notifier.clearPendingEdit('inventory');
    } else {
      widget.notifier.setPendingEdit(
        'inventory',
        PendingSaveEdit(edits: allEdits),
      );
    }
  }

  Future<void> _openAddDialog() async {
    // Scope the dialog to the save it was opened for. If the user switches to a
    // different save while the dialog is open, the awaited result is stale — its
    // excludePaths and target belong to the old save, so applying it would queue
    // the add against the wrong save. Key on the selected save path, not the
    // inventory object identity: re-inspecting the SAME save allocates a fresh
    // summary instance with identical contents, and that result must still apply.
    final dialogSavePath = widget.notifier.state.selectedPath;
    final result = await showDialog<InventoryItemAdd>(
      context: context,
      builder: (_) => AddInventoryItemDialog(
        excludePaths: widget.inventory.mainContainerPaths.toSet(),
      ),
    );
    if (result == null) return;
    if (!mounted || widget.notifier.state.selectedPath != dialogSavePath) return;
    setState(() {
      _pendingAdd = result;
      // Keep the one-structural-edit-per-save invariant unconditionally.
      _pendingRemovePath = null;
    });
    _pushInventoryPending();
  }

  void _queueRemove(PrivateInventoryItem item) {
    setState(() {
      // A removal supersedes any pending count change on the same item, and is
      // mutually exclusive with a pending add (one structural edit per save).
      _pendingCountChanges.remove(_inventoryItemKey(item));
      _pendingAdd = null;
      _pendingRemovePath = item.path;
    });
    _pushInventoryPending();
  }

  void _undoRemove() {
    setState(() => _pendingRemovePath = null);
    _pushInventoryPending();
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final inventory = widget.inventory;
    final query = _query.trim().toLowerCase();
    final items = inventory.items.where((item) {
      // A pending removal hides the item from the list (it is represented by
      // the pending card above), mirroring how a pending add is not yet in the
      // list either.
      if (_pendingRemovePath != null &&
          item.path.isNotEmpty &&
          item.path == _pendingRemovePath) {
        return false;
      }
      if (query.isEmpty) return true;
      return item.id.toLowerCase().contains(query) ||
          item.path.toLowerCase().contains(query);
    }).toList();
    final groups = groupInventoryItems(items);

    // Keep the current category selected if it still has items, else fall
    // back to the first available group.
    var selected = _selectedCategory;
    if (groups.every((g) => g.category != selected)) {
      selected = groups.isEmpty ? null : groups.first.category;
    }
    final selectedGroup =
        groups.where((g) => g.category == selected).firstOrNull;

    // An active search shows matches across all categories as a flat list;
    // an empty query browses by the selected category.
    final searching = query.isNotEmpty;
    final shownItems = searching
        ? items
        : (selectedGroup?.items ?? const <PrivateInventoryItem>[]);

    final hasItems = inventory.items.isNotEmpty;
    final hasPendingAdd = _pendingAdd != null;
    final hasPendingRemove = _pendingRemovePath != null;
    final hasPendingCount = _pendingCountChanges.isNotEmpty;
    final hasPendingChanges =
        hasPendingCount || hasPendingAdd || hasPendingRemove;
    final canRemove = widget.canRemoveItem;
    // A structural edit (add/remove) must be saved on its own (the core/notifier
    // reject a batch that mixes it with count edits), so structural edits and
    // count edits are kept mutually exclusive in the UI: a structural edit is
    // blocked while counts are pending, and count editing is blocked while a
    // structural edit is pending.
    final structuralBlocked = hasPendingAdd || hasPendingRemove || hasPendingCount;
    final countEditable = widget.editable && !hasPendingAdd && !hasPendingRemove;

    return Card(
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Row(
              children: [
                const Icon(Icons.inventory_2_outlined),
                const SizedBox(width: 8),
                Expanded(
                  child: Text(
                    'Inventory',
                    style: theme.textTheme.titleMedium,
                  ),
                ),
                if (widget.editable && hasPendingChanges) ...[
                  Tooltip(
                    message: 'Reset inventory changes',
                    child: IconButton(
                      icon: const Icon(Icons.undo_outlined),
                      onPressed: () {
                        setState(() {
                          _pendingCountChanges.clear();
                          _pendingAdd = null;
                          _pendingRemovePath = null;
                        });
                        widget.notifier.clearPendingEdit('inventory');
                      },
                    ),
                  ),
                ],
                if (widget.canAddItem) ...[
                  const SizedBox(width: 8),
                  Tooltip(
                    message: hasPendingAdd
                        ? 'Save pending changes first — one new item per save'
                        : hasPendingRemove
                            ? 'Save the pending removal first — one structural change per save'
                            : hasPendingCount
                                ? 'Save or reset pending count changes first — a structural edit must be saved on its own'
                                : 'Add item to inventory',
                    child: FilledButton.icon(
                      icon: const Icon(Icons.add, size: 18),
                      label: const Text('Add item'),
                      onPressed: structuralBlocked ? null : _openAddDialog,
                    ),
                  ),
                ],
              ],
            ),
            if (hasPendingAdd) ...[
              const SizedBox(height: 12),
              _PendingStructuralRow(
                tone: _PendingTone.add,
                icon: Icons.add_circle_outline,
                title: _itemDisplayFromPath(_pendingAdd!.path),
                subtitle: '×${_pendingAdd!.count} — pending add (not yet saved)',
                cancelTooltip: 'Cancel pending add',
                onCancel: () {
                  setState(() => _pendingAdd = null);
                  _pushInventoryPending();
                },
              ),
            ],
            if (hasPendingRemove) ...[
              const SizedBox(height: 12),
              _PendingStructuralRow(
                tone: _PendingTone.remove,
                icon: Icons.delete_outline,
                title: _itemDisplayFromPath(_pendingRemovePath!),
                subtitle: 'pending removal (not yet saved)',
                cancelTooltip: 'Cancel pending removal',
                onCancel: _undoRemove,
              ),
            ],
            if (hasItems) ...[
              const SizedBox(height: 12),
              TextField(
                controller: _searchController,
                decoration: const InputDecoration(
                  labelText: 'Filter items',
                  prefixIcon: Icon(Icons.search),
                ),
                onChanged: (value) => setState(() => _query = value),
              ),
              const SizedBox(height: 12),
              Expanded(
                child: groups.isEmpty
                    ? Center(
                        child: Text(
                          // An empty query with no rows means a pending removal
                          // hid the last item(s) — not a filter miss, so don't
                          // claim "no items match".
                          searching
                              ? 'No items match "$_query".'
                              : 'The pending removal hides every item — '
                                  'save to apply it.',
                          style: theme.textTheme.bodyMedium,
                        ),
                      )
                    : Row(
                        crossAxisAlignment: CrossAxisAlignment.stretch,
                        children: [
                          SizedBox(
                            width: 200,
                            child: DecoratedBox(
                              decoration: BoxDecoration(
                                color: theme.colorScheme.surfaceContainerLow,
                                borderRadius: BorderRadius.circular(12),
                              ),
                              child: SingleChildScrollView(
                                padding:
                                    const EdgeInsets.symmetric(vertical: 6),
                                child: Column(
                                  children: [
                                    for (final group in groups)
                                      SidebarTile(
                                        icon: iconForItemCategory(
                                          group.category,
                                        ),
                                        label:
                                            '${group.category.label} (${group.items.length})',
                                        selected: !searching &&
                                            group.category == selected,
                                        onTap: () => setState(() {
                                          _selectedCategory = group.category;
                                          // Leave search mode so the chosen
                                          // category's items are shown.
                                          _query = '';
                                          _searchController.clear();
                                        }),
                                      ),
                                  ],
                                ),
                              ),
                            ),
                          ),
                          const SizedBox(width: 16),
                          Expanded(
                            child: shownItems.isEmpty
                                ? const SizedBox.shrink()
                                : ListView.builder(
                                    itemCount: shownItems.length,
                                    itemBuilder: (context, index) {
                                      final item = shownItems[index];
                                      return ListTile(
                                        dense: true,
                                        leading: const Icon(
                                          Icons.category_outlined,
                                        ),
                                        title: Text(
                                          item.id.isEmpty ? item.path : item.id,
                                          maxLines: 1,
                                          overflow: TextOverflow.ellipsis,
                                        ),
                                        subtitle: item.path.isEmpty
                                            ? null
                                            : Text(
                                                item.path,
                                                maxLines: 1,
                                                overflow: TextOverflow.ellipsis,
                                              ),
                                        trailing: _inventoryItemTrailing(
                                          theme,
                                          item,
                                          canRemove: canRemove,
                                          countEditable: countEditable,
                                          removeBlocked: structuralBlocked,
                                        ),
                                      );
                                    },
                                  ),
                          ),
                        ],
                      ),
              ),
            ],
          ],
        ),
      ),
    );
  }

  void _setPendingCountChange(
    PrivateInventoryItem item,
    InventoryItemCountChange? change,
  ) {
    setState(() {
      final key = _inventoryItemKey(item);
      if (change == null) {
        _pendingCountChanges.remove(key);
      } else {
        _pendingCountChanges[key] = change;
      }
    });
    _pushInventoryPending();
  }

  /// Trailing widget for an inventory row: count editor + a delete button.
  Widget _inventoryItemTrailing(
    ThemeData theme,
    PrivateInventoryItem item, {
    required bool canRemove,
    required bool countEditable,
    required bool removeBlocked,
  }) {
    // The count editor shows only when count editing is currently allowed (the
    // inventory is count-editable AND no structural edit is pending). A
    // remove-only inventory, or one with a pending structural edit, shows the
    // count as plain text but the delete action may still apply.
    return Row(
      mainAxisSize: MainAxisSize.min,
      // Centre the delete button against the count field so the trash icon lines
      // up with the input value rather than floating up by the 'Count' label.
      crossAxisAlignment: CrossAxisAlignment.center,
      children: [
        if (countEditable)
          _InventoryItemCountEditor(
            item: item,
            pendingCount: _pendingCountChanges[_inventoryItemKey(item)]?.count,
            onPendingCountChanged: (change) =>
                _setPendingCountChange(item, change),
          )
        else
          Text(
            '×${item.count ?? '?'}',
            style: theme.textTheme.bodyMedium,
          ),
        if (canRemove && item.path.isNotEmpty) ...[
          const SizedBox(width: 4),
          Tooltip(
            message: !item.removable
                ? "Can't delete: this item is likely equipped or "
                    'assigned to a hotkey slot'
                : removeBlocked
                ? 'Save or reset your pending inventory changes first — '
                    'an add or remove must be saved on its own'
                : 'Remove item from inventory',
            child: IconButton(
              icon: const Icon(Icons.delete_outline),
              // A non-removable item shows the trash icon disabled (its asset
              // path occurs in more than one container — e.g. also equipped or
              // in a quickslot — so the core can't unambiguously remove it). A
              // removable item is disabled only while a structural/count edit
              // is pending; otherwise it queues the remove.
              onPressed: (!item.removable || removeBlocked)
                  ? null
                  : () => _queueRemove(item),
            ),
          ),
        ],
      ],
    );
  }
}

/// Tone of a pending structural-edit card: an add (primary) or a remove
/// (error).
enum _PendingTone { add, remove }

/// A human-readable id fragment derived from an item asset path.
String _itemDisplayFromPath(String path) =>
    path.contains('.') ? path.split('.').last : path.split('/').last;

/// A highlighted card shown when there is a pending structural inventory edit
/// (add or remove) awaiting save. Mirrors how a not-yet-saved item is
/// represented for both directions: the affected item is not shown inline, only
/// here, with a cancel button.
class _PendingStructuralRow extends StatelessWidget {
  const _PendingStructuralRow({
    required this.icon,
    required this.title,
    required this.subtitle,
    required this.onCancel,
    required this.cancelTooltip,
    required this.tone,
  });

  final IconData icon;
  final String title;
  final String subtitle;
  final VoidCallback onCancel;
  final String cancelTooltip;
  final _PendingTone tone;

  @override
  Widget build(BuildContext context) {
    final scheme = Theme.of(context).colorScheme;
    final isAdd = tone == _PendingTone.add;
    final bg = isAdd ? scheme.primaryContainer : scheme.errorContainer;
    final fg = isAdd ? scheme.onPrimaryContainer : scheme.onErrorContainer;
    final accent = isAdd ? scheme.primary : scheme.error;
    return DecoratedBox(
      decoration: BoxDecoration(
        color: bg,
        borderRadius: BorderRadius.circular(8),
        border: Border.all(color: accent.withValues(alpha: 0.4)),
      ),
      child: ListTile(
        dense: true,
        leading: Icon(icon, color: accent),
        title: Text(
          title,
          maxLines: 1,
          overflow: TextOverflow.ellipsis,
          style: TextStyle(color: fg),
        ),
        subtitle: Text(
          subtitle,
          style: TextStyle(color: fg.withValues(alpha: 0.8)),
        ),
        trailing: IconButton(
          icon: const Icon(Icons.close),
          tooltip: cancelTooltip,
          onPressed: onCancel,
        ),
      ),
    );
  }
}

class _InventoryItemCountEditor extends StatefulWidget {
  const _InventoryItemCountEditor({
    required this.item,
    required this.onPendingCountChanged,
    this.pendingCount,
  });

  final PrivateInventoryItem item;
  final int? pendingCount;
  final void Function(InventoryItemCountChange? change) onPendingCountChanged;

  @override
  State<_InventoryItemCountEditor> createState() =>
      _InventoryItemCountEditorState();
}

class _InventoryItemCountEditorState extends State<_InventoryItemCountEditor> {
  late final TextEditingController _controller;
  String? _path;
  String? _id;
  int? _pendingCount;
  String? _error;

  @override
  void initState() {
    super.initState();
    _controller = TextEditingController();
    _sync();
  }

  @override
  void didUpdateWidget(covariant _InventoryItemCountEditor oldWidget) {
    super.didUpdateWidget(oldWidget);
    _sync();
  }

  @override
  void dispose() {
    _controller.dispose();
    super.dispose();
  }

  void _sync() {
    // Rows are identified by path-or-id, so include the id: two stacks with an
    // empty path but different ids must not be treated as the same row.
    if (_path == widget.item.path &&
        _id == widget.item.id &&
        _pendingCount == widget.pendingCount) {
      return;
    }
    final isSameItem = _path == widget.item.path && _id == widget.item.id;
    _path = widget.item.path;
    _id = widget.item.id;
    _pendingCount = widget.pendingCount;
    final text = (widget.pendingCount ?? widget.item.count)?.toString() ?? '';
    if (_controller.text != text) {
      final currentOffset = _controller.selection.baseOffset;
      final nextOffset = isSameItem
          ? currentOffset.clamp(0, text.length)
          : text.length;
      _controller.value = TextEditingValue(
        text: text,
        selection: TextSelection.collapsed(offset: nextOffset),
      );
    }
    _error = null;
  }

  @override
  Widget build(BuildContext context) {
    return SizedBox(
      width: 132,
      child: TextField(
        controller: _controller,
        keyboardType: TextInputType.number,
        onChanged: _onCountTextChanged,
        decoration: InputDecoration(
          labelText: 'Count',
          errorText: _error,
          // Compact the field so it fits inside the dense ListTile row. A
          // reserved helper line would steal the input box's vertical space
          // here (the tile caps the field height), squeezing the box until the
          // value clips at the border — so the error grows the row instead.
          isDense: true,
        ),
      ),
    );
  }

  void _onCountTextChanged(String value) {
    final trimmed = value.trim();
    if (trimmed.isEmpty) {
      setState(() => _error = null);
      widget.onPendingCountChanged(null);
      return;
    }
    final parsed = int.tryParse(trimmed);
    if (parsed == null || parsed < 1) {
      // Min 1: a count of 0 would leave a ghost slot (invisible in-game but
      // still in the save). Use the remove button to delete an item.
      setState(() => _error = 'Min 1');
      widget.onPendingCountChanged(null);
      return;
    }
    setState(() => _error = null);
    if (parsed == widget.item.count) {
      widget.onPendingCountChanged(null);
      return;
    }
    widget.onPendingCountChanged(
      InventoryItemCountChange(
        id: widget.item.id,
        path: widget.item.path,
        count: parsed,
      ),
    );
  }
}

String _inventoryItemKey(PrivateInventoryItem item) {
  // Combine id and path so rows that share a definition path
  // but differ by id — repeated item types — get distinct pending-change
  // entries instead of collapsing onto one key.
  return '${item.id}\u0000${item.path}';
}

class _PrivatePlayerTransformEditor extends StatefulWidget {
  const _PrivatePlayerTransformEditor({
    required this.transform,
    required this.editable,
    required this.notifier,
    this.reloadKey,
  });

  final PrivatePlayerTransform transform;
  final bool editable;
  final EditorNotifier notifier;
  // When provided, a change in identity triggers a field reseed even if the
  // transform values haven't changed (e.g. Reset followed by re-inspect that
  // returns the same values).
  final Object? reloadKey;

  @override
  State<_PrivatePlayerTransformEditor> createState() =>
      _PrivatePlayerTransformEditorState();
}

class _PrivatePlayerTransformEditorState
    extends State<_PrivatePlayerTransformEditor> {
  late final TextEditingController _locationXController;
  late final TextEditingController _locationYController;
  late final TextEditingController _locationZController;
  late final TextEditingController _rotationPitchController;
  late final TextEditingController _rotationYawController;
  late final TextEditingController _rotationRollController;
  PrivatePlayerTransform? _lastTransform;
  // Track the inspection (widget parent) identity so that a Reset/refresh that
  // produces a new inspection instance triggers a reseed even when the
  // transform values themselves haven't changed.
  Object? _inspectionIdentity;
  String? _error;

  @override
  void initState() {
    super.initState();
    _locationXController = TextEditingController();
    _locationYController = TextEditingController();
    _locationZController = TextEditingController();
    _rotationPitchController = TextEditingController();
    _rotationYawController = TextEditingController();
    _rotationRollController = TextEditingController();
    _sync();
  }

  @override
  void didUpdateWidget(covariant _PrivatePlayerTransformEditor oldWidget) {
    super.didUpdateWidget(oldWidget);
    _sync();
  }

  @override
  void dispose() {
    _locationXController.dispose();
    _locationYController.dispose();
    _locationZController.dispose();
    _rotationPitchController.dispose();
    _rotationYawController.dispose();
    _rotationRollController.dispose();
    super.dispose();
  }

  void _sync() {
    final transform = widget.transform;
    final last = _lastTransform;
    // Re-seed on reloadKey identity change (e.g. after Reset/refresh that
    // produces a new SaveInspection) even when the transform values themselves
    // are unchanged, so the fields visually revert after a Reset.
    final newKey = widget.reloadKey;
    final sameKey = newKey == null || identical(newKey, _inspectionIdentity);
    if (!sameKey) {
      _inspectionIdentity = newKey;
    }
    if (sameKey &&
        last != null &&
        last.location.x == transform.location.x &&
        last.location.y == transform.location.y &&
        last.location.z == transform.location.z &&
        last.rotation.pitch == transform.rotation.pitch &&
        last.rotation.yaw == transform.rotation.yaw &&
        last.rotation.roll == transform.rotation.roll) {
      return;
    }
    _lastTransform = transform;
    _locationXController.text = _formatAttributeValue(transform.location.x);
    _locationYController.text = _formatAttributeValue(transform.location.y);
    _locationZController.text = _formatAttributeValue(transform.location.z);
    _rotationPitchController.text = _formatAttributeValue(
      transform.rotation.pitch,
    );
    _rotationYawController.text = _formatAttributeValue(transform.rotation.yaw);
    _rotationRollController.text = _formatAttributeValue(
      transform.rotation.roll,
    );
    _error = null;
  }

  @override
  Widget build(BuildContext context) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Row(
          children: [
            const Icon(Icons.explore_outlined),
            const SizedBox(width: 8),
            Expanded(
              child: Text(
                'Hero transform',
                style: Theme.of(context).textTheme.titleSmall,
              ),
            ),
          ],
        ),
        if (_error != null) ...[
          const SizedBox(height: 6),
          Text(
            _error!,
            style: TextStyle(color: Theme.of(context).colorScheme.error),
          ),
        ],
        const SizedBox(height: 10),
        LayoutBuilder(
          builder: (context, constraints) {
            final compact = constraints.maxWidth < 700;
            final fields = [
              _TransformNumberField(
                controller: _locationXController,
                label: 'Location X',
                enabled: widget.editable,
                onChanged: (_) => _updatePending(),
              ),
              _TransformNumberField(
                controller: _locationYController,
                label: 'Location Y',
                enabled: widget.editable,
                onChanged: (_) => _updatePending(),
              ),
              _TransformNumberField(
                controller: _locationZController,
                label: 'Location Z',
                enabled: widget.editable,
                onChanged: (_) => _updatePending(),
              ),
              _TransformNumberField(
                controller: _rotationPitchController,
                label: 'Rotation pitch',
                enabled: widget.editable,
                onChanged: (_) => _updatePending(),
              ),
              _TransformNumberField(
                controller: _rotationYawController,
                label: 'Rotation yaw',
                enabled: widget.editable,
                onChanged: (_) => _updatePending(),
              ),
              _TransformNumberField(
                controller: _rotationRollController,
                label: 'Rotation roll',
                enabled: widget.editable,
                onChanged: (_) => _updatePending(),
              ),
            ];
            if (compact) {
              return Column(
                children: [
                  for (final field in fields) ...[
                    field,
                    if (field != fields.last) const SizedBox(height: 8),
                  ],
                ],
              );
            }
            return Column(
              children: [
                Row(
                  children: [
                    for (final field in fields.take(3)) ...[
                      Expanded(child: field),
                      if (field != fields[2]) const SizedBox(width: 8),
                    ],
                  ],
                ),
                const SizedBox(height: 8),
                Row(
                  children: [
                    for (final field in fields.skip(3)) ...[
                      Expanded(child: field),
                      if (field != fields.last) const SizedBox(width: 8),
                    ],
                  ],
                ),
              ],
            );
          },
        ),
      ],
    );
  }

  void _updatePending() {
    if (!widget.editable) return;
    final locationX = double.tryParse(_locationXController.text.trim());
    final locationY = double.tryParse(_locationYController.text.trim());
    final locationZ = double.tryParse(_locationZController.text.trim());
    final rotationPitch = double.tryParse(_rotationPitchController.text.trim());
    final rotationYaw = double.tryParse(_rotationYawController.text.trim());
    final rotationRoll = double.tryParse(_rotationRollController.text.trim());
    if (locationX == null ||
        locationY == null ||
        locationZ == null ||
        rotationPitch == null ||
        rotationYaw == null ||
        rotationRoll == null) {
      setState(() => _error = 'Invalid');
      widget.notifier.clearPendingEdit('transform');
      return;
    }
    setState(() => _error = null);
    final orig = widget.transform;
    if (locationX == orig.location.x &&
        locationY == orig.location.y &&
        locationZ == orig.location.z &&
        rotationPitch == orig.rotation.pitch &&
        rotationYaw == orig.rotation.yaw &&
        rotationRoll == orig.rotation.roll) {
      widget.notifier.clearPendingEdit('transform');
      return;
    }
    widget.notifier.setPendingEdit(
      'transform',
      PendingSaveEdit(
        edits: [
          {
            'path': 'private.player.setTransform',
            'value': {
              'location': {'x': locationX, 'y': locationY, 'z': locationZ},
              'rotation': {
                'pitch': rotationPitch,
                'yaw': rotationYaw,
                'roll': rotationRoll,
              },
            },
          },
        ],
      ),
    );
  }
}

class _TransformNumberField extends StatelessWidget {
  const _TransformNumberField({
    required this.controller,
    required this.label,
    required this.enabled,
    this.onChanged,
  });

  final TextEditingController controller;
  final String label;
  final bool enabled;
  final ValueChanged<String>? onChanged;

  @override
  Widget build(BuildContext context) {
    return TextField(
      controller: controller,
      enabled: enabled,
      onChanged: onChanged,
      keyboardType: const TextInputType.numberWithOptions(
        decimal: true,
        signed: true,
      ),
      decoration: InputDecoration(labelText: label),
    );
  }
}

class _PrivatePlayerAttributesEditor extends StatelessWidget {
  const _PrivatePlayerAttributesEditor({
    required this.player,
    required this.notifier,
    this.editable = true,
    this.reloadKey,
  });

  final PrivatePlayerSummary player;
  final EditorNotifier notifier;
  final bool editable;
  final Object? reloadKey;

  @override
  Widget build(BuildContext context) {
    final editable =
        this.editable &&
        player.writable.contains('private.player.setAttribute');
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Row(
          children: [
            const Icon(Icons.monitor_heart_outlined),
            const SizedBox(width: 8),
            Text(
              'Hero attributes',
              style: Theme.of(context).textTheme.titleSmall,
            ),
          ],
        ),
        const SizedBox(height: 10),
        LayoutBuilder(
          builder: (context, constraints) {
            final compact = constraints.maxWidth < 620;
            return Column(
              children: player.attributes
                  .map(
                    (attribute) => _PrivatePlayerAttributeRow(
                      attribute: attribute,
                      notifier: notifier,
                      editable: editable,
                      compact: compact,
                      reloadKey: reloadKey,
                    ),
                  )
                  .toList(),
            );
          },
        ),
      ],
    );
  }
}

class _PrivatePlayerAttributeRow extends StatefulWidget {
  const _PrivatePlayerAttributeRow({
    required this.attribute,
    required this.notifier,
    required this.editable,
    required this.compact,
    this.reloadKey,
  });

  final PrivatePlayerAttribute attribute;
  final EditorNotifier notifier;
  final bool editable;
  final bool compact;
  // When provided, a change in identity triggers a field reseed even if the
  // attribute values haven't changed (e.g. after a Reset that reverts to the
  // same canonical value).
  final Object? reloadKey;

  @override
  State<_PrivatePlayerAttributeRow> createState() =>
      _PrivatePlayerAttributeRowState();
}

class _PrivatePlayerAttributeRowState
    extends State<_PrivatePlayerAttributeRow> {
  late final TextEditingController _baseController;
  late final TextEditingController _currentController;
  String? _lastId;
  double? _lastBase;
  double? _lastCurrent;
  Object? _lastReloadKey;
  String? _error;

  @override
  void initState() {
    super.initState();
    _baseController = TextEditingController();
    _currentController = TextEditingController();
    _sync();
  }

  @override
  void didUpdateWidget(covariant _PrivatePlayerAttributeRow oldWidget) {
    super.didUpdateWidget(oldWidget);
    _sync();
  }

  @override
  void dispose() {
    _baseController.dispose();
    _currentController.dispose();
    super.dispose();
  }

  void _sync() {
    final attribute = widget.attribute;
    final newKey = widget.reloadKey;
    final sameKey = newKey == null || identical(newKey, _lastReloadKey);
    if (!sameKey) {
      _lastReloadKey = newKey;
    }
    if (sameKey &&
        _lastId == attribute.id &&
        _lastBase == attribute.baseValue &&
        _lastCurrent == attribute.currentValue) {
      return;
    }
    _lastId = attribute.id;
    _lastBase = attribute.baseValue;
    _lastCurrent = attribute.currentValue;
    _baseController.text = _formatAttributeValue(attribute.baseValue);
    _currentController.text = _formatAttributeValue(attribute.currentValue);
    _error = null;
  }

  @override
  Widget build(BuildContext context) {
    final name = widget.attribute.id;
    final baseField = TextField(
      controller: _baseController,
      enabled: widget.editable,
      keyboardType: const TextInputType.numberWithOptions(decimal: true),
      onChanged: (_) => _updatePending(),
      decoration: InputDecoration(labelText: '$name base', errorText: _error),
    );
    final currentField = TextField(
      controller: _currentController,
      enabled: widget.editable,
      keyboardType: const TextInputType.numberWithOptions(decimal: true),
      onChanged: (_) => _updatePending(),
      decoration: InputDecoration(labelText: '$name current'),
    );
    final label = SizedBox(
      width: 116,
      child: Text(name, style: Theme.of(context).textTheme.labelLarge),
    );
    if (widget.compact) {
      return Padding(
        padding: const EdgeInsets.symmetric(vertical: 6),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            Text(name, style: Theme.of(context).textTheme.labelLarge),
            const SizedBox(height: 6),
            baseField,
            const SizedBox(height: 6),
            currentField,
          ],
        ),
      );
    }
    return Padding(
      padding: const EdgeInsets.symmetric(vertical: 5),
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          label,
          Expanded(child: baseField),
          const SizedBox(width: 8),
          Expanded(child: currentField),
        ],
      ),
    );
  }

  void _updatePending() {
    if (!widget.editable) return;
    final id = widget.attribute.id;
    final baseValue = double.tryParse(_baseController.text.trim());
    final currentValue = double.tryParse(_currentController.text.trim());
    if (baseValue == null || currentValue == null) {
      setState(() => _error = 'Invalid');
      widget.notifier.clearPendingEdit('attr:$id');
      return;
    }
    setState(() => _error = null);
    final origBase = widget.attribute.baseValue;
    final origCurrent = widget.attribute.currentValue;
    if (baseValue == origBase && currentValue == origCurrent) {
      widget.notifier.clearPendingEdit('attr:$id');
      return;
    }
    widget.notifier.setPendingEdit(
      'attr:$id',
      PendingSaveEdit(
        edits: [
          {
            'path': 'private.player.setAttribute',
            'value': {
              'id': id,
              'baseValue': baseValue,
              'currentValue': currentValue,
            },
          },
        ],
      ),
    );
  }
}

String _formatAttributeValue(double? value) {
  if (value == null) return '';
  if (value == value.roundToDouble()) return value.toInt().toString();
  final rounded = value.toStringAsFixed(2).replaceFirst(RegExp(r'\.?0+$'), '');
  // These texts seed editable fields whose parsed value gets written back —
  // a lossy rounding (0.125 → 0.13) would silently corrupt untouched axes
  // the moment any sibling field changes. Round-trip or full precision.
  return double.tryParse(rounded) == value ? rounded : value.toString();
}

/// Generic typed property browser: search every property in the decoded
/// private payload and edit scalars and strings. This is the
/// "everything is editable" surface — no curated field list, the user finds
/// any value by name and edits the ones the core can safely patch.
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

  final _controller = TextEditingController();
  TypedSearchResult? _result;
  bool _searching = false;
  int _requestSeq = 0;
  int _pageSize = 50;
  String _activeQuery = '';
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
  final Map<String, String> _typedDrafts = {};

  @override
  void initState() {
    super.initState();
    _inspectionReloadKey = widget.inspection;
    // Empty query lists everything — show the first page as soon as the tab
    // opens for a decoded save.
    if (widget.inspection.privateDecoded) {
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
      // Invalidate any in-flight search for the previous save.
      _requestSeq++;
      _inspectionReloadKey = widget.inspection;
      // Switching saves clears pending centrally; drop the drafts with it.
      _typedDrafts.clear();
      setState(() {
        _result = null;
        _searching = false;
      });
      if (widget.inspection.privateDecoded) {
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
      if (widget.inspection.privateDecoded) {
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

  @override
  Widget build(BuildContext context) {
    if (!widget.inspection.privateDecoded) {
      return const _MessagePane(
        icon: Icons.tune,
        title: 'All data',
        body:
            'The full property browser needs decoded private payload data from '
            'the codec.',
      );
    }
    final theme = Theme.of(context);
    return Padding(
      padding: const EdgeInsets.all(20),
      child: Card(
        child: Padding(
          padding: const EdgeInsets.all(16),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: [
              Row(
                children: [
                  const Icon(Icons.tune),
                  const SizedBox(width: 8),
                  Expanded(
                    child: Text('All data', style: theme.textTheme.titleMedium),
                  ),
                ],
              ),
              const SizedBox(height: 4),
              Text(
                'Search every typed property by name or path. Scalars, strings, '
                'enums and object paths are editable; structs are shown '
                'read-only for now.',
                style: theme.textTheme.bodySmall?.copyWith(
                  color: theme.colorScheme.onSurfaceVariant,
                ),
              ),
              const SizedBox(height: 12),
              TextField(
                controller: _controller,
                decoration: InputDecoration(
                  labelText:
                      'Search properties (empty = list everything) — e.g. Health, GameTime',
                  prefixIcon: const Icon(Icons.search),
                  suffixIcon: _searching
                      ? const Padding(
                          padding: EdgeInsets.all(12),
                          child: SizedBox(
                            width: 16,
                            height: 16,
                            child: CircularProgressIndicator(strokeWidth: 2),
                          ),
                        )
                      : IconButton(
                          icon: const Icon(Icons.arrow_forward),
                          onPressed: () => _run(offset: 0, newQuery: true),
                        ),
                ),
                onSubmitted: (_) => _run(offset: 0, newQuery: true),
              ),
              const SizedBox(height: 12),
              _buildPaginationBar(theme),
              const SizedBox(height: 8),
              Expanded(child: _buildResults(theme)),
            ],
          ),
        ),
      ),
    );
  }

  Widget _buildResults(ThemeData theme) {
    final result = _result;
    if (result == null) {
      if (_searching) {
        return const _MessagePane(
          icon: Icons.hourglass_empty,
          title: 'Decoding save…',
          body:
              'Decoding the full private payload for the first search. This '
              'runs once per save, then searches are instant.',
        );
      }
      return const _MessagePane(
        icon: Icons.search,
        title: 'Search the save',
        body:
            'Type a property name and press enter. Leave it empty to list '
            'everything.',
      );
    }
    if (result.error != null) {
      return _MessagePane(
        icon: Icons.error_outline,
        title: 'Search failed',
        body: result.error!,
      );
    }
    if (result.results.isEmpty) {
      return const _MessagePane(
        icon: Icons.search_off,
        title: 'No matches',
        body: 'No property path contained all of those terms.',
      );
    }
    return ListView.separated(
      itemCount: result.results.length,
      separatorBuilder: (_, _) => const Divider(height: 1),
      itemBuilder: (context, index) {
        final hit = result.results[index];
        return _TypedPropertyRow(
          key: ValueKey(hit.display),
          hit: hit,
          editable: widget.editable && hit.editable,
          notifier: widget.notifier,
          reloadKey: _inspectionReloadKey,
          drafts: _typedDrafts,
        );
      },
    );
  }

  Widget _buildPaginationBar(ThemeData theme) {
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
          tooltip: 'First page',
          visualDensity: VisualDensity.compact,
          icon: const Icon(Icons.first_page),
          onPressed: busy || !result.hasPrevious ? null : () => _goToPage(0),
        ),
        IconButton(
          tooltip: 'Previous page',
          visualDensity: VisualDensity.compact,
          icon: const Icon(Icons.chevron_left),
          onPressed: busy || !result.hasPrevious
              ? null
              : () => _goToPage(result.pageIndex - 1),
        ),
        IconButton(
          tooltip: 'Next page',
          visualDensity: VisualDensity.compact,
          icon: const Icon(Icons.chevron_right),
          onPressed: busy || !result.hasNext
              ? null
              : () => _goToPage(result.pageIndex + 1),
        ),
        IconButton(
          tooltip: 'Last page',
          visualDensity: VisualDensity.compact,
          icon: const Icon(Icons.last_page),
          onPressed: busy || !result.hasNext
              ? null
              : () => _goToPage(result.pageCount - 1),
        ),
        const SizedBox(width: 4),
        Text(
          'Page ${result.pageIndex + 1} / ${result.pageCount}',
          style: muted,
        ),
        const SizedBox(width: 8),
        Text('$first–$last of ${result.total}', style: muted),
        const SizedBox(width: 8),
        Text('Per page:', style: muted),
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
  final Map<String, String> drafts;
  // When provided, a change in identity forces a reseed of the field from the
  // canonical hit value (e.g. after a Reset that reverts to the same value).
  final Object? reloadKey;

  @override
  State<_TypedPropertyRow> createState() => _TypedPropertyRowState();
}

class _TypedPropertyRowState extends State<_TypedPropertyRow> {
  late final TextEditingController _controller = TextEditingController(
    text: widget.drafts[_pendingKey] ?? widget.hit.value,
  );
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
    if (_isBool && draft != null) {
      _boolDraft = draft == 'true';
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
      _controller.text = widget.hit.value;
      _boolDraft = null;
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

  /// Returns a key string that identifies this property in the pending registry.
  String get _pendingKey => 'typed:${widget.hit.path.join(' ')}';

  Object? _coerce(String text) {
    final type = widget.hit.type;
    if (type == 'StrProperty' || type == 'NameProperty') {
      // String values are written verbatim — leading/trailing whitespace may
      // be intentional, so no trim.
      return text;
    }
    if (type == 'ObjectProperty' || type == 'EnumProperty') {
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
    final value = _coerce(text);
    if (value == null) {
      // Invalid / unparseable — don't contribute to pending.
      widget.drafts.remove(_pendingKey);
      widget.notifier.clearPendingEdit(_pendingKey);
      return;
    }
    // Revert to original → clear pending.
    if (text == widget.hit.value ||
        (widget.hit.type != 'StrProperty' &&
            widget.hit.type != 'NameProperty' &&
            text.trim() == widget.hit.value.trim())) {
      widget.drafts.remove(_pendingKey);
      widget.notifier.clearPendingEdit(_pendingKey);
      return;
    }
    widget.drafts[_pendingKey] = text;
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
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final hit = widget.hit;
    return Padding(
      padding: const EdgeInsets.symmetric(vertical: 6),
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.center,
        children: [
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                SelectableText(hit.display, maxLines: 2),
                Text(
                  hit.type,
                  style: theme.textTheme.labelSmall?.copyWith(
                    color: theme.colorScheme.outline,
                  ),
                ),
              ],
            ),
          ),
          const SizedBox(width: 12),
          if (!widget.editable)
            SizedBox(
              width: 220,
              child: Text(
                hit.value,
                textAlign: TextAlign.right,
                style: theme.textTheme.bodyMedium?.copyWith(
                  color: theme.colorScheme.onSurfaceVariant,
                ),
              ),
            )
          else if (_isBool)
            _BoolEditor(
              value: _boolDraft ?? (hit.value == 'true'),
              onChanged: (next) {
                setState(() => _boolDraft = next);
                _updatePending(next.toString());
              },
            )
          else
            SizedBox(
              width: 220,
              child: TextField(
                controller: _controller,
                onChanged: _updatePending,
                decoration: const InputDecoration(
                  isDense: true,
                  labelText: 'Value',
                ),
              ),
            ),
        ],
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
    return ListView(
      padding: const EdgeInsets.all(20),
      children: [
        Row(
          children: [
            const Icon(Icons.history),
            const SizedBox(width: 8),
            Text('Backups', style: Theme.of(context).textTheme.titleLarge),
            const Spacer(),
            Tooltip(
              message: 'Refresh backups',
              child: IconButton(
                icon: const Icon(Icons.refresh),
                onPressed: state.isLoading ? null : notifier.refreshBackups,
              ),
            ),
          ],
        ),
        const SizedBox(height: 12),
        if (backups.isEmpty && companionBackups.isEmpty)
          const _InlineNotice(
            icon: Icons.info_outline,
            title: 'No backups',
            body: 'Edited saves create backup files next to the selected slot.',
          ),
        if (backups.isNotEmpty) ...[
          Text('Slot backups', style: Theme.of(context).textTheme.titleMedium),
          const SizedBox(height: 8),
          ...backups.map(
            (backup) => _BackupCard(
              backup: backup,
              isLoading: state.isLoading,
              showRestoreAction: true,
              onRestore: () => notifier.restoreBackup(backup.path),
            ),
          ),
        ],
        if (companionBackups.isNotEmpty) ...[
          if (backups.isNotEmpty) const SizedBox(height: 8),
          Text(
            'Profile backups',
            style: Theme.of(context).textTheme.titleMedium,
          ),
          const SizedBox(height: 8),
          ...companionBackups.map(
            (backup) => _BackupCard(
              backup: backup,
              isLoading: state.isLoading,
              showRestoreAction: true,
              onRestore: () => notifier.restoreCompanionBackup(backup.path),
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
  });

  final BackupEntry backup;
  final bool isLoading;
  final bool showRestoreAction;
  final VoidCallback onRestore;

  @override
  Widget build(BuildContext context) {
    final canRestore = showRestoreAction && backup.canRestore;
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
                      backup.fileName,
                      style: Theme.of(context).textTheme.titleMedium,
                      overflow: TextOverflow.ellipsis,
                    ),
                    const SizedBox(height: 6),
                    Wrap(
                      spacing: 14,
                      runSpacing: 6,
                      children: [
                        _SmallFact(
                          label: 'Name',
                          value: backup.playerSaveName ?? '-',
                        ),
                        if (backup.slotName != null)
                          _SmallFact(label: 'Slot', value: backup.slotName!),
                        _SmallFact(
                          label: 'Created',
                          value: _formatBackupTime(backup.createdEpoch),
                        ),
                        _SmallFact(
                          label: 'Size',
                          value: '${_bytes.format(backup.fileSize)} bytes',
                        ),
                        _SmallFact(label: 'Status', value: backup.status),
                        _SmallFact(
                          label: 'SHA-1',
                          value: _shortSha(backup.sha1),
                        ),
                      ],
                    ),
                  ],
                ),
              ),
              if (showRestoreAction) ...[
                const SizedBox(width: 12),
                Tooltip(
                  message: 'Restore ${backup.fileName}',
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
          Text(value, maxLines: 2, overflow: TextOverflow.ellipsis),
        ],
      ),
    );
  }
}

String _formatBackupTime(int? epoch) {
  if (epoch == null) return '-';
  final dateTime = DateTime.fromMillisecondsSinceEpoch(
    epoch * 1000,
    isUtc: true,
  ).toLocal();
  return DateFormat.yMd().add_Hms().format(dateTime);
}

String _shortSha(String sha1) {
  if (sha1.length <= 12) return sha1;
  return sha1.substring(0, 12);
}

class _SettingsPanel extends StatelessWidget {
  const _SettingsPanel({required this.state, required this.notifier});

  final EditorState state;
  final EditorNotifier notifier;

  @override
  Widget build(BuildContext context) {
    final codec = state.codecStatus;
    return ListView(
      padding: const EdgeInsets.all(20),
      children: [
        const AppearanceSettingsCard(),
        const SizedBox(height: 16),
        const UpdateSettingsCard(),
        const SizedBox(height: 16),
        const LocalizationSettingsCard(),
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
                      'Savegame directory',
                      style: Theme.of(context).textTheme.titleMedium,
                    ),
                  ],
                ),
                const SizedBox(height: 12),
                _PathSettingRow(
                  label: 'Folder',
                  value: state.saveDir,
                  onBrowse: notifier.chooseSaveDir,
                ),
              ],
            ),
          ),
        ),
        const SizedBox(height: 16),
        Card(
          child: Padding(
            padding: const EdgeInsets.all(16),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Row(
                  children: [
                    const Icon(Icons.compress_outlined),
                    const SizedBox(width: 8),
                    Text(
                      'Codec',
                      style: Theme.of(context).textTheme.titleMedium,
                    ),
                    const Spacer(),
                    OutlinedButton.icon(
                      icon: const Icon(Icons.refresh),
                      label: const Text('Check'),
                      onPressed: () => notifier.checkCodec(),
                    ),
                    const SizedBox(width: 8),
                    OutlinedButton.icon(
                      icon: const Icon(Icons.verified_outlined),
                      label: const Text('Roundtrip'),
                      onPressed: state.selectedPath == null || state.isLoading
                          ? null
                          : notifier.validateCodecRoundtrip,
                    ),
                  ],
                ),
                const SizedBox(height: 12),
                CodecStatusView(codec: codec, codecError: state.codecError),
              ],
            ),
          ),
        ),
      ],
    );
  }
}

class CodecStatusView extends StatelessWidget {
  const CodecStatusView({super.key, required this.codec, required this.codecError});

  final CodecStatus? codec;
  final String? codecError;

  @override
  Widget build(BuildContext context) {
    final codec = this.codec;
    final scheme = Theme.of(context).colorScheme;
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
      return errorRow ?? const Text('No codec status');
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
        ? 'Codec ready'
        : isDecodeOnly
        ? 'Codec read-only'
        : 'Codec unavailable';
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        if (errorRow != null) ...[errorRow, const SizedBox(height: 8)],
        Row(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Icon(
              statusIcon,
              size: 18,
              color: statusColor,
            ),
            const SizedBox(width: 6),
            Expanded(
              child: Text(title,
                  style: TextStyle(color: isReady ? null : statusColor)),
            ),
          ],
        ),
        const SizedBox(height: 8),
        ExpansionTile(
          tilePadding: EdgeInsets.zero,
          childrenPadding: const EdgeInsets.only(bottom: 8),
          title: const Text('Details'),
          children: [
            Align(
              alignment: Alignment.centerLeft,
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Text('Status: ${codec.status}'),
                  Text('Decompress: ${codec.canDecompress ? 'yes' : 'no'} | '
                      'Compress: ${codec.canCompress ? 'yes' : 'no'}'),
                  Text('Backend: ${codec.adapter ?? codec.backend}'),
                ],
              ),
            ),
          ],
        ),
      ],
    );
  }
}

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
              style: const TextStyle(fontFamily: 'Consolas', fontSize: 12),
            ),
          ),
        ),
        const SizedBox(width: 8),
        IconButton(
          tooltip: 'Browse',
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
