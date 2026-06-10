import 'dart:convert';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:goresave/features/app/domain/ui_settings.dart';
import 'package:goresave/features/app/ui/about_dialog.dart';
import 'package:goresave/features/app/ui/appearance_settings.dart';
import 'package:goresave/features/app/ui/window_chrome.dart';
import 'package:goresave/features/editor/domain/editor_notifier.dart';
import 'package:goresave/features/editor/domain/editor_models.dart';
import 'package:goresave/providers/data_providers.dart';
import 'package:intl/intl.dart';
import 'hero_stats_card.dart';

final _bytes = NumberFormat.decimalPattern();

class EditorPage extends ConsumerWidget {
  const EditorPage({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
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
              const Text('Gothic Remake Savegame Editor'),
              const Expanded(child: SizedBox()),
            ],
          ),
        ),
        titleSpacing: 0,
        centerTitle: false,
        scrolledUnderElevation: 0,
        surfaceTintColor: Colors.transparent,
        actions: [
          Tooltip(
            message: 'Validate',
            child: IconButton(
              icon: const Icon(Icons.verified_outlined),
              onPressed: state.selectedPath == null || state.isLoading
                  ? null
                  : notifier.validateSelected,
            ),
          ),
          Tooltip(
            message: 'Refresh',
            child: IconButton(
              icon: const Icon(Icons.refresh),
              onPressed: state.isLoading ? null : notifier.refresh,
            ),
          ),
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
                  width: 380,
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
    // Scope the list to the active profile so the list, the header total, and
    // the profile-scoped Quick/Auto counts all agree in a multi-profile folder.
    final activeProfileId = state.activeProfile?.profileId;
    final saves = activeProfileId == null
        ? state.saves
        : state.saves
              .where(
                (s) =>
                    // Keep this profile's saves, plus saves with unknown profile
                    // metadata, and never hide the currently selected save.
                    s.persistentProfileId == activeProfileId ||
                    s.persistentProfileId == null ||
                    s.path == state.selectedPath,
              )
              .toList();
    return DecoratedBox(
      decoration: BoxDecoration(
        color: Theme.of(context).colorScheme.surfaceContainerLow,
      ),
      child: Column(
        children: [
          Padding(
            padding: const EdgeInsets.all(12),
            child: Row(
              children: [
                Expanded(
                  child: Text(
                    state.saveDir,
                    maxLines: 2,
                    overflow: TextOverflow.ellipsis,
                    style: Theme.of(context).textTheme.bodySmall,
                  ),
                ),
                Tooltip(
                  message: 'Choose folder',
                  child: IconButton(
                    icon: const Icon(Icons.folder_open_outlined),
                    onPressed: state.isLoading ? null : notifier.chooseSaveDir,
                  ),
                ),
              ],
            ),
          ),
          const Divider(height: 1),
          _ProfileHeader(profile: state.activeProfile, saveCount: saves.length),
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
  const _ProfileHeader({required this.profile, required this.saveCount});

  final ProfileSummary? profile;
  final int saveCount;

  @override
  Widget build(BuildContext context) {
    final textTheme = Theme.of(context).textTheme;
    final scheme = Theme.of(context).colorScheme;
    final quickCount = profile?.quickSaveSlots.length ?? 0;
    final autoCount = profile?.autoSaveSlots.length ?? 0;
    return Container(
      width: double.infinity,
      padding: const EdgeInsets.fromLTRB(16, 14, 16, 12),
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
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(
                  profile?.displayName ?? 'Profile',
                  maxLines: 1,
                  overflow: TextOverflow.ellipsis,
                  style: textTheme.titleMedium,
                ),
                const SizedBox(height: 2),
                Text(
                  '${_formatCount(saveCount, 'save')} | Quick $quickCount | Auto $autoCount',
                  maxLines: 1,
                  overflow: TextOverflow.ellipsis,
                  style: textTheme.bodySmall?.copyWith(
                    color: scheme.onSurfaceVariant,
                  ),
                ),
              ],
            ),
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
  final mapName = save.mapName;
  if (mapName != null && mapName.isNotEmpty) {
    parts.add(mapName);
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

String _formatCount(int count, String singular) {
  return count == 1 ? '1 $singular' : '$count ${singular}s';
}

class _EditorWorkspace extends StatelessWidget {
  const _EditorWorkspace({required this.state, required this.notifier});

  final EditorState state;
  final EditorNotifier notifier;

  @override
  Widget build(BuildContext context) {
    final scheme = Theme.of(context).colorScheme;
    final isDark = Theme.of(context).brightness == Brightness.dark;
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
      content = DefaultTabController(
        length: 8,
        child: Column(
          children: [
            Container(
              color: scheme.surfaceContainerLowest,
              child: const TabBar(
                isScrollable: true,
                tabs: [
                  Tab(icon: Icon(Icons.dashboard_outlined), text: 'Overview'),
                  Tab(icon: Icon(Icons.person_outline), text: 'Player'),
                  Tab(
                    icon: Icon(Icons.inventory_2_outlined),
                    text: 'Inventory',
                  ),
                  Tab(icon: Icon(Icons.flag_outlined), text: 'Progression'),
                  Tab(icon: Icon(Icons.tune), text: 'All data'),
                  Tab(icon: Icon(Icons.data_object), text: 'Advanced'),
                  Tab(icon: Icon(Icons.history), text: 'Backups'),
                  Tab(icon: Icon(Icons.settings_outlined), text: 'Settings'),
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
            if (state.codecNeedsVerification)
              MaterialBanner(
                backgroundColor: isDark
                    ? const Color(0xFF3A2E1A)
                    : const Color(0xFFFFF4E5),
                leading: Icon(
                  Icons.shield_outlined,
                  color: isDark
                      ? const Color(0xFFE0A95C)
                      : const Color(0xFFB26A00),
                ),
                content: const Text(
                  'This game build is not auto-trusted for compression. '
                  'Verify the codec to unlock private edits — it round-trips a '
                  'real save chunk through the game executable.',
                ),
                actions: [
                  TextButton(
                    onPressed: state.isLoading || state.selectedPath == null
                        ? null
                        : notifier.verifyCodec,
                    child: const Text('Verify codec'),
                  ),
                ],
              ),
            Expanded(
              child: TabBarView(
                children: [
                  _OverviewPanel(
                    inspection: inspection,
                    notifier: notifier,
                    selectedSave: state.selectedSave,
                  ),
                  _PrivatePanel(
                    icon: Icons.person_outline,
                    title: 'Player',
                    inspection: inspection,
                    notifier: notifier,
                    // Private writes recompress the payload, so also require the
                    // codec host to be compress-ready (auto-trusted or verified
                    // this session), not just decode-ready.
                    editable:
                        inspection.privateEditable && state.codecCompressReady,
                    decodedBody:
                        'Private player data is decoded through the G1R codec host.',
                    lockedBody:
                        'Private player edits need a verified G1R codec host.',
                  ),
                  _InventoryPanel(
                    inspection: inspection,
                    notifier: notifier,
                    canCompress: state.codecCompressReady,
                  ),
                  _ProgressionPanel(inspection: inspection, notifier: notifier),
                  _AllDataPanel(
                    inspection: inspection,
                    notifier: notifier,
                    // Typed writes recompress the private payload, so require a
                    // full private decode (not a preview) plus a compress-ready
                    // codec, matching the Player and Inventory gating.
                    editable:
                        inspection.privateEditable && state.codecCompressReady,
                  ),
                  _AdvancedPanel(inspection: inspection),
                  _BackupsPanel(state: state, notifier: notifier),
                  _SettingsPanel(state: state, notifier: notifier),
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

class _OverviewPanel extends StatelessWidget {
  const _OverviewPanel({
    required this.inspection,
    required this.notifier,
    required this.selectedSave,
  });

  final SaveInspection inspection;
  final EditorNotifier notifier;
  final SaveSlot? selectedSave;

  @override
  Widget build(BuildContext context) {
    return ListView(
      padding: const EdgeInsets.all(20),
      children: [
        _HeaderCard(inspection: inspection, save: selectedSave),
        const SizedBox(height: 16),
        _MetadataEditor(inspection: inspection, notifier: notifier),
        const SizedBox(height: 16),
        _OverviewDiagnostics(inspection: inspection),
      ],
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
                        if (inspection.mapName != null)
                          'Map': inspection.mapName!,
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
                            if (inspection.mapName != null)
                              _InfoPill(
                                icon: Icons.map_outlined,
                                label: inspection.mapName!,
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
    // Also resync when the name changes for the same path (e.g. after a restore
    // or rescan returns updated metadata), so the field doesn't keep showing
    // the stale name.
    if (_path == widget.inspection.path && _name == name) return;
    _path = widget.inspection.path;
    _name = name;
    _controller.text = name;
  }

  @override
  Widget build(BuildContext context) {
    return Card(
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Row(
          crossAxisAlignment: CrossAxisAlignment.end,
          children: [
            Expanded(
              child: TextField(
                controller: _controller,
                decoration: InputDecoration(
                  labelText: 'Public save name',
                  prefixIcon: const Icon(Icons.edit_outlined),
                  errorText: _error,
                ),
              ),
            ),
            const SizedBox(width: 12),
            Tooltip(
              message: 'Save metadata',
              child: FilledButton.icon(
                icon: const Icon(Icons.save_outlined),
                label: const Text('Save'),
                onPressed: () {
                  // Reject blank/whitespace names, matching the private editors,
                  // so an empty public.m_PlayerSaveName isn't synced into the
                  // save and PersistentDataList.
                  final value = _controller.text.trim();
                  if (value.isEmpty) {
                    setState(() => _error = 'Required');
                    return;
                  }
                  setState(() => _error = null);
                  widget.notifier.writePlayerSaveName(value);
                },
              ),
            ),
          ],
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
    required this.decodedBody,
    required this.lockedBody,
  });

  final IconData icon;
  final String title;
  final SaveInspection inspection;
  final EditorNotifier notifier;
  final bool editable;
  final String decodedBody;
  final String lockedBody;

  @override
  Widget build(BuildContext context) {
    if (inspection.privateDecoded) {
      return ListView(
        padding: const EdgeInsets.all(20),
        children: [
          if (title == 'Player' && inspection.privatePlayer.hasData) ...[
            _PrivatePlayerSummaryCard(
              player: inspection.privatePlayer,
              notifier: notifier,
              savePath: inspection.path,
              // Reuse the panel's already compress-gated flag.
              editable: editable,
              // The typed hero stats card supersedes the heuristic
              // attribute editor whenever the strict typed parse is OK.
              showLegacyAttributes: !inspection.privateTypedVerified,
            ),
            const SizedBox(height: 16),
          ],
          if (title == 'Player' && inspection.privateTypedVerified) ...[
            HeroStatsCard(
              // New SaveInspection instance after every write/refresh —
              // changing identity drops pending edits and reloads.
              reloadKey: inspection,
              load: notifier.loadHeroAttributes,
              save: notifier.writeTypedValues,
              editable: editable,
              // Spec: if the typed search errors out or finds nothing on a
              // typed-OK save, the heuristic editor stays available.
              fallback: inspection.privatePlayer.attributes.isNotEmpty
                  ? _PrivatePlayerAttributesEditor(
                      player: inspection.privatePlayer,
                      notifier: notifier,
                      editable: editable,
                    )
                  : null,
            ),
            const SizedBox(height: 16),
          ],
          _PrivateSummaryCard(
            icon: Icons.lock_open_outlined,
            title: title,
            body: decodedBody,
            inspection: inspection,
          ),
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

enum _InventorySection { items, diagnostics }

class _InventoryPanel extends StatefulWidget {
  const _InventoryPanel({
    required this.inspection,
    required this.notifier,
    this.canCompress = false,
  });

  final SaveInspection inspection;
  final EditorNotifier notifier;
  final bool canCompress;

  @override
  State<_InventoryPanel> createState() => _InventoryPanelState();
}

class _InventoryPanelState extends State<_InventoryPanel> {
  // Accordion: at most one section open at a time; the open one fills height.
  _InventorySection? _open = _InventorySection.items;

  void _toggle(_InventorySection section) {
    setState(() => _open = _open == section ? null : section);
  }

  @override
  Widget build(BuildContext context) {
    final inspection = widget.inspection;
    if (!inspection.privateDecoded) {
      return const _MessagePane(
        icon: Icons.inventory_2_outlined,
        title: 'Inventory',
        body:
            'Inventory editing needs decoded private payload data from the G1R codec host.',
      );
    }
    final hasItems = inspection.privateInventory.hasData;
    final itemsOpen = hasItems && _open == _InventorySection.items;
    final diagnosticsOpen = _open == _InventorySection.diagnostics;
    return Padding(
      padding: const EdgeInsets.all(20),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          if (hasItems) ...[
            _expandable(
              open: itemsOpen,
              child: _PrivateInventorySummaryCard(
                inventory: inspection.privateInventory,
                notifier: widget.notifier,
                // Inventory writes recompress the payload too, so require a
                // compress-capable codec host in addition to a full decode.
                // The core only allows count edits in a detected player
                // inventory region, advertised via writable; gate on it so other
                // scopes don't show editors whose saves fail in the core.
                editable:
                    inspection.privateEditable &&
                    widget.canCompress &&
                    inspection.privateInventory.writable.contains(
                      'private.inventory.setItemCount',
                    ),
                expanded: itemsOpen,
                onToggle: () => _toggle(_InventorySection.items),
              ),
            ),
            const SizedBox(height: 16),
          ],
          _expandable(
            open: diagnosticsOpen,
            child: _InventoryDiagnostics(
              inventory: inspection.privateInventory,
              inspection: inspection,
              expanded: diagnosticsOpen,
              onToggle: () => _toggle(_InventorySection.diagnostics),
            ),
          ),
        ],
      ),
    );
  }

  // The open section fills the remaining vertical space; collapsed sections
  // size to their header only.
  Widget _expandable({required bool open, required Widget child}) {
    return open ? Expanded(child: child) : child;
  }
}

class _ProgressionPanel extends StatelessWidget {
  const _ProgressionPanel({required this.inspection, required this.notifier});

  final SaveInspection inspection;
  final EditorNotifier notifier;

  @override
  Widget build(BuildContext context) {
    if (!inspection.privateDecoded) {
      return const _MessagePane(
        icon: Icons.flag_outlined,
        title: 'Progression',
        body:
            'Progression data needs decoded private payload data from the G1R codec host.',
      );
    }
    return ListView(
      padding: const EdgeInsets.all(20),
      children: [
        if (inspection.privateProgression.hasData)
          _PrivateProgressionSummaryCard(
            progression: inspection.privateProgression,
          )
        else
          const _MessagePane(
            icon: Icons.flag_outlined,
            title: 'Progression',
            body:
                'No progression markers found in the decoded private payload.',
          ),
      ],
    );
  }
}

class _PrivateProgressionSummaryCard extends StatefulWidget {
  const _PrivateProgressionSummaryCard({required this.progression});

  final PrivateProgressionSummary progression;

  @override
  State<_PrivateProgressionSummaryCard> createState() =>
      _PrivateProgressionSummaryCardState();
}

class _PrivateProgressionSummaryCardState
    extends State<_PrivateProgressionSummaryCard> {
  String _query = '';

  @override
  Widget build(BuildContext context) {
    final progression = widget.progression;
    final query = _query.trim().toLowerCase();
    final candidates = progression.candidates
        .where((value) => query.isEmpty || value.toLowerCase().contains(query))
        .take(160)
        .toList();
    final candidateSet = candidates.toSet();
    final tags = progression.gameplayTags
        .where(
          (value) =>
              (query.isEmpty || value.toLowerCase().contains(query)) &&
              !candidateSet.contains(value),
        )
        .take(160)
        .toList();
    return Card(
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Row(
              children: [
                const Icon(Icons.flag_outlined),
                const SizedBox(width: 8),
                Expanded(
                  child: Text(
                    'Progression summary',
                    style: Theme.of(context).textTheme.titleMedium,
                  ),
                ),
              ],
            ),
            const SizedBox(height: 12),
            Wrap(
              spacing: 8,
              runSpacing: 8,
              children: [
                _SummaryMetric(
                  label: 'Candidates',
                  value: progression.candidateCount.toString(),
                ),
                _SummaryMetric(
                  label: 'Gameplay tags',
                  value: progression.gameplayTags.length.toString(),
                ),
                _SummaryMetric(
                  label: 'Sections',
                  value: progression.sections.length.toString(),
                ),
                _SummaryMetric(
                  label: 'Properties',
                  value: progression.properties.length.toString(),
                ),
                _SummaryMetric(
                  label: 'Scripts',
                  value: progression.scriptPaths.length.toString(),
                ),
              ],
            ),
            if (progression.sections.isNotEmpty) ...[
              const SizedBox(height: 16),
              Wrap(
                spacing: 8,
                runSpacing: 8,
                children: progression.sections
                    .map((value) => Chip(label: Text(value, maxLines: 1)))
                    .toList(),
              ),
            ],
            const SizedBox(height: 16),
            TextField(
              decoration: const InputDecoration(
                labelText: 'Filter progression',
                prefixIcon: Icon(Icons.search),
              ),
              onChanged: (value) => setState(() => _query = value),
            ),
            if (candidates.isNotEmpty) ...[
              const SizedBox(height: 16),
              Text(
                'Progression markers',
                style: Theme.of(context).textTheme.labelLarge,
              ),
              const SizedBox(height: 8),
              _StringList(values: candidates, icon: Icons.flag_outlined),
            ],
            if (tags.isNotEmpty) ...[
              const SizedBox(height: 16),
              Text(
                'Gameplay tags',
                style: Theme.of(context).textTheme.labelLarge,
              ),
              const SizedBox(height: 8),
              _StringList(values: tags, icon: Icons.sell_outlined),
            ],
            if (progression.scriptPaths.isNotEmpty) ...[
              const SizedBox(height: 16),
              Text(
                'Progression scripts',
                style: Theme.of(context).textTheme.labelLarge,
              ),
              const SizedBox(height: 8),
              Wrap(
                spacing: 8,
                runSpacing: 8,
                children: progression.scriptPaths
                    .take(8)
                    .map((value) => Chip(label: Text(value, maxLines: 1)))
                    .toList(),
              ),
            ],
          ],
        ),
      ),
    );
  }
}

class _StringList extends StatelessWidget {
  const _StringList({required this.values, required this.icon});

  final List<String> values;
  final IconData icon;

  @override
  Widget build(BuildContext context) {
    return SizedBox(
      height: 220,
      child: ListView.separated(
        itemCount: values.length,
        separatorBuilder: (_, _) => const Divider(height: 1),
        itemBuilder: (context, index) {
          return ListTile(
            dense: true,
            leading: Icon(icon),
            title: SelectableText(values[index], maxLines: 1),
          );
        },
      ),
    );
  }
}

class _PrivateInventorySummaryCard extends StatefulWidget {
  const _PrivateInventorySummaryCard({
    required this.inventory,
    required this.notifier,
    this.editable = true,
    this.expanded = true,
    this.onToggle,
  });

  final PrivateInventorySummary inventory;
  final EditorNotifier notifier;
  final bool editable;
  final bool expanded;
  final VoidCallback? onToggle;

  @override
  State<_PrivateInventorySummaryCard> createState() =>
      _PrivateInventorySummaryCardState();
}

class _PrivateInventorySummaryCardState
    extends State<_PrivateInventorySummaryCard> {
  String _query = '';
  final Map<String, InventoryItemCountChange> _pendingCountChanges = {};

  @override
  void didUpdateWidget(covariant _PrivateInventorySummaryCard oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.inventory != widget.inventory) {
      _pendingCountChanges.clear();
    }
  }

  @override
  Widget build(BuildContext context) {
    final inventory = widget.inventory;
    final query = _query.trim().toLowerCase();
    final items = inventory.items
        .where((item) {
          if (query.isEmpty) return true;
          return item.id.toLowerCase().contains(query) ||
              item.path.toLowerCase().contains(query);
        })
        .take(80)
        .toList();
    return Card(
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          mainAxisSize: widget.expanded ? MainAxisSize.max : MainAxisSize.min,
          children: [
            _CollapsibleCardHeader(
              icon: Icons.inventory_2_outlined,
              title: 'Inventory',
              expanded: widget.expanded,
              onToggle: widget.onToggle,
            ),
            if (widget.expanded && items.isNotEmpty) ...[
              const SizedBox(height: 16),
              Text(
                'Observed item stacks',
                style: Theme.of(context).textTheme.labelLarge,
              ),
              const SizedBox(height: 8),
              if (widget.editable && _pendingCountChanges.isNotEmpty) ...[
                Row(
                  children: [
                    FilledButton.icon(
                      icon: const Icon(Icons.save_outlined),
                      label: Text(
                        'Save ${_pendingCountChanges.length} '
                        '${_pendingCountChanges.length == 1 ? 'change' : 'changes'}',
                      ),
                      onPressed: () => widget.notifier.writeInventoryItemCounts(
                        _pendingCountChanges.values.toList(),
                      ),
                    ),
                    const SizedBox(width: 8),
                    Tooltip(
                      message: 'Reset inventory changes',
                      child: IconButton(
                        icon: const Icon(Icons.undo_outlined),
                        onPressed: () => setState(_pendingCountChanges.clear),
                      ),
                    ),
                  ],
                ),
                const SizedBox(height: 8),
              ],
              TextField(
                decoration: const InputDecoration(
                  labelText: 'Filter items',
                  prefixIcon: Icon(Icons.search),
                ),
                onChanged: (value) => setState(() => _query = value),
              ),
              const SizedBox(height: 8),
              Expanded(
                child: ListView.separated(
                  itemCount: items.length,
                  separatorBuilder: (_, _) => const Divider(height: 1),
                  itemBuilder: (context, index) {
                    final item = items[index];
                    return ListTile(
                      dense: true,
                      leading: const Icon(Icons.category_outlined),
                      title: SelectableText(
                        item.id.isEmpty ? item.path : item.id,
                        maxLines: 1,
                      ),
                      subtitle: item.path.isEmpty
                          ? null
                          : SelectableText(item.path, maxLines: 1),
                      trailing: widget.editable
                          ? _InventoryItemCountEditor(
                              item: item,
                              notifier: widget.notifier,
                              pendingCount:
                                  _pendingCountChanges[_inventoryItemKey(item)]
                                      ?.count,
                              onPendingCountChanged: (change) =>
                                  _setPendingCountChange(item, change),
                            )
                          : Text(
                              '×${item.count ?? '?'}',
                              style: Theme.of(context).textTheme.bodyMedium,
                            ),
                    );
                  },
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
  }
}

class _InventoryItemCountEditor extends StatefulWidget {
  const _InventoryItemCountEditor({
    required this.item,
    required this.notifier,
    required this.onPendingCountChanged,
    this.pendingCount,
  });

  final PrivateInventoryItem item;
  final EditorNotifier notifier;
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
    final id = widget.item.id.isEmpty ? widget.item.path : widget.item.id;
    return SizedBox(
      width: 164,
      child: Row(
        children: [
          Expanded(
            child: TextField(
              controller: _controller,
              keyboardType: TextInputType.number,
              onChanged: _onCountTextChanged,
              decoration: InputDecoration(
                labelText: 'Count',
                errorText: _error,
              ),
            ),
          ),
          Tooltip(
            message: 'Save $id count',
            child: IconButton(
              icon: const Icon(Icons.save_outlined),
              onPressed: () {
                final parsed = int.tryParse(_controller.text.trim());
                if (parsed == null || parsed < 0) {
                  setState(() => _error = 'Invalid');
                  return;
                }
                setState(() => _error = null);
                widget.notifier.writeInventoryItemCount(
                  id: widget.item.id,
                  path: widget.item.path,
                  count: parsed,
                );
              },
            ),
          ),
        ],
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
    if (parsed == null || parsed < 0) {
      setState(() => _error = 'Invalid');
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

String _inventoryScopeLabel(String scope) {
  return switch (scope) {
    'player_inventory_region' => 'Player inventory',
    'global_observed' => 'Observed globally',
    _ => scope,
  };
}

class _PrivatePlayerSummaryCard extends StatelessWidget {
  const _PrivatePlayerSummaryCard({
    required this.player,
    required this.notifier,
    this.savePath,
    this.editable = true,
    this.showLegacyAttributes = true,
  });

  final PrivatePlayerSummary player;
  final EditorNotifier notifier;
  final String? savePath;
  final bool editable;
  final bool showLegacyAttributes;

  @override
  Widget build(BuildContext context) {
    final metrics = <Widget>[
      if (player.saveVersionNumber != null)
        _SummaryMetric(
          label: 'Save version',
          value: player.saveVersionNumber.toString(),
        ),
      if (player.currentWorld != null)
        _SummaryMetric(label: 'Current world', value: player.currentWorld!),
      if (player.playerName != null)
        _SummaryMetric(label: 'Player name', value: player.playerName!),
      if (player.profileName != null)
        _SummaryMetric(label: 'Profile name', value: player.profileName!),
      if (player.scriptPaths.isNotEmpty)
        _SummaryMetric(
          label: 'Script paths',
          value: player.scriptPaths.length.toString(),
        ),
      if (player.properties.isNotEmpty)
        _SummaryMetric(
          label: 'Properties',
          value: player.properties.length.toString(),
        ),
    ];
    return Card(
      child: Padding(
        padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 12),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Row(
              crossAxisAlignment: CrossAxisAlignment.center,
              children: [
                const Icon(Icons.person_search_outlined),
                const SizedBox(width: 8),
                Text(
                  'Player summary',
                  style: Theme.of(context).textTheme.titleMedium,
                ),
                const SizedBox(width: 18),
                Expanded(
                  child: Wrap(spacing: 8, runSpacing: 8, children: metrics),
                ),
              ],
            ),
            if (editable &&
                player.writable.contains('private.player.setPlayerName') &&
                player.playerName != null) ...[
              const SizedBox(height: 14),
              _PrivatePlayerNameEditor(
                // Key by save identity so switching to another save (even one
                // with the same parsed name) resets the field instead of
                // keeping a stale, unsaved edit that could be written to the
                // newly selected save.
                key: ValueKey('private-player-name-$savePath'),
                player: player,
                notifier: notifier,
              ),
            ],
            if (editable &&
                player.writable.contains('private.profile.setProfileName') &&
                player.profileName != null) ...[
              const SizedBox(height: 14),
              _PrivateProfileNameEditor(
                key: ValueKey('private-profile-name-$savePath'),
                player: player,
                notifier: notifier,
              ),
            ],
            if (showLegacyAttributes && player.attributes.isNotEmpty) ...[
              const SizedBox(height: 16),
              const Divider(height: 1),
              const SizedBox(height: 12),
              _PrivatePlayerAttributesEditor(
                player: player,
                notifier: notifier,
                editable: editable,
              ),
            ],
            if (player.transform != null) ...[
              const SizedBox(height: 16),
              const Divider(height: 1),
              const SizedBox(height: 12),
              _PrivatePlayerTransformEditor(
                transform: player.transform!,
                editable:
                    editable &&
                    player.writable.contains('private.player.setTransform'),
                notifier: notifier,
              ),
            ],
          ],
        ),
      ),
    );
  }
}

class _PrivatePlayerNameEditor extends StatefulWidget {
  const _PrivatePlayerNameEditor({
    super.key,
    required this.player,
    required this.notifier,
  });

  final PrivatePlayerSummary player;
  final EditorNotifier notifier;

  @override
  State<_PrivatePlayerNameEditor> createState() =>
      _PrivatePlayerNameEditorState();
}

class _PrivatePlayerNameEditorState extends State<_PrivatePlayerNameEditor> {
  late final TextEditingController _controller;
  String? _lastName;
  String? _error;

  @override
  void initState() {
    super.initState();
    _controller = TextEditingController();
    _sync();
  }

  @override
  void didUpdateWidget(covariant _PrivatePlayerNameEditor oldWidget) {
    super.didUpdateWidget(oldWidget);
    _sync();
  }

  @override
  void dispose() {
    _controller.dispose();
    super.dispose();
  }

  void _sync() {
    if (_lastName == widget.player.playerName) return;
    _lastName = widget.player.playerName;
    _controller.text = widget.player.playerName ?? '';
    _error = null;
  }

  @override
  Widget build(BuildContext context) {
    return LayoutBuilder(
      builder: (context, constraints) {
        final field = TextField(
          controller: _controller,
          decoration: InputDecoration(
            labelText: 'Private player name',
            prefixIcon: const Icon(Icons.badge_outlined),
            errorText: _error,
          ),
        );
        final button = Tooltip(
          message: 'Save private player name',
          child: IconButton.filledTonal(
            icon: const Icon(Icons.save_outlined),
            onPressed: () {
              final value = _controller.text.trim();
              if (value.isEmpty) {
                setState(() => _error = 'Required');
                return;
              }
              setState(() => _error = null);
              widget.notifier.writePrivatePlayerName(value);
            },
          ),
        );
        if (constraints.maxWidth < 520) {
          return Column(
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: [
              field,
              const SizedBox(height: 8),
              Align(alignment: Alignment.centerRight, child: button),
            ],
          );
        }
        return Row(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Expanded(child: field),
            const SizedBox(width: 8),
            button,
          ],
        );
      },
    );
  }
}

class _PrivateProfileNameEditor extends StatefulWidget {
  const _PrivateProfileNameEditor({
    super.key,
    required this.player,
    required this.notifier,
  });

  final PrivatePlayerSummary player;
  final EditorNotifier notifier;

  @override
  State<_PrivateProfileNameEditor> createState() =>
      _PrivateProfileNameEditorState();
}

class _PrivateProfileNameEditorState extends State<_PrivateProfileNameEditor> {
  late final TextEditingController _controller;
  String? _lastName;
  String? _error;

  @override
  void initState() {
    super.initState();
    _controller = TextEditingController();
    _sync();
  }

  @override
  void didUpdateWidget(covariant _PrivateProfileNameEditor oldWidget) {
    super.didUpdateWidget(oldWidget);
    _sync();
  }

  @override
  void dispose() {
    _controller.dispose();
    super.dispose();
  }

  void _sync() {
    if (_lastName == widget.player.profileName) return;
    _lastName = widget.player.profileName;
    _controller.text = widget.player.profileName ?? '';
    _error = null;
  }

  @override
  Widget build(BuildContext context) {
    return LayoutBuilder(
      builder: (context, constraints) {
        final field = TextField(
          controller: _controller,
          decoration: InputDecoration(
            labelText: 'Private profile name',
            prefixIcon: const Icon(Icons.account_circle_outlined),
            errorText: _error,
          ),
        );
        final button = Tooltip(
          message: 'Save private profile name',
          child: IconButton.filledTonal(
            icon: const Icon(Icons.save_outlined),
            onPressed: () {
              final value = _controller.text.trim();
              if (value.isEmpty) {
                setState(() => _error = 'Required');
                return;
              }
              setState(() => _error = null);
              widget.notifier.writePrivateProfileName(value);
            },
          ),
        );
        if (constraints.maxWidth < 520) {
          return Column(
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: [
              field,
              const SizedBox(height: 8),
              Align(alignment: Alignment.centerRight, child: button),
            ],
          );
        }
        return Row(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Expanded(child: field),
            const SizedBox(width: 8),
            button,
          ],
        );
      },
    );
  }
}

class _PrivatePlayerTransformEditor extends StatefulWidget {
  const _PrivatePlayerTransformEditor({
    required this.transform,
    required this.editable,
    required this.notifier,
  });

  final PrivatePlayerTransform transform;
  final bool editable;
  final EditorNotifier notifier;

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
    if (last != null &&
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
    final saveButton = Tooltip(
      message: 'Save hero transform',
      child: IconButton.filledTonal(
        icon: const Icon(Icons.save_outlined),
        onPressed: widget.editable ? _save : null,
      ),
    );
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
            saveButton,
          ],
        ),
        const SizedBox(height: 10),
        LayoutBuilder(
          builder: (context, constraints) {
            final compact = constraints.maxWidth < 700;
            final fields = [
              _TransformNumberField(
                controller: _locationXController,
                label: 'Location X',
                enabled: widget.editable,
                errorText: _error,
              ),
              _TransformNumberField(
                controller: _locationYController,
                label: 'Location Y',
                enabled: widget.editable,
              ),
              _TransformNumberField(
                controller: _locationZController,
                label: 'Location Z',
                enabled: widget.editable,
              ),
              _TransformNumberField(
                controller: _rotationPitchController,
                label: 'Rotation pitch',
                enabled: widget.editable,
              ),
              _TransformNumberField(
                controller: _rotationYawController,
                label: 'Rotation yaw',
                enabled: widget.editable,
              ),
              _TransformNumberField(
                controller: _rotationRollController,
                label: 'Rotation roll',
                enabled: widget.editable,
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

  void _save() {
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
      return;
    }
    setState(() => _error = null);
    widget.notifier.writePlayerTransform(
      locationX: locationX,
      locationY: locationY,
      locationZ: locationZ,
      rotationPitch: rotationPitch,
      rotationYaw: rotationYaw,
      rotationRoll: rotationRoll,
    );
  }
}

class _TransformNumberField extends StatelessWidget {
  const _TransformNumberField({
    required this.controller,
    required this.label,
    required this.enabled,
    this.errorText,
  });

  final TextEditingController controller;
  final String label;
  final bool enabled;
  final String? errorText;

  @override
  Widget build(BuildContext context) {
    return TextField(
      controller: controller,
      enabled: enabled,
      keyboardType: const TextInputType.numberWithOptions(
        decimal: true,
        signed: true,
      ),
      decoration: InputDecoration(labelText: label, errorText: errorText),
    );
  }
}

class _PrivatePlayerAttributesEditor extends StatelessWidget {
  const _PrivatePlayerAttributesEditor({
    required this.player,
    required this.notifier,
    this.editable = true,
  });

  final PrivatePlayerSummary player;
  final EditorNotifier notifier;
  final bool editable;

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
  });

  final PrivatePlayerAttribute attribute;
  final EditorNotifier notifier;
  final bool editable;
  final bool compact;

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
    if (_lastId == attribute.id &&
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
      decoration: InputDecoration(labelText: '$name base', errorText: _error),
    );
    final currentField = TextField(
      controller: _currentController,
      enabled: widget.editable,
      keyboardType: const TextInputType.numberWithOptions(decimal: true),
      decoration: InputDecoration(labelText: '$name current'),
    );
    final saveButton = Tooltip(
      message: 'Save $name attribute',
      child: IconButton.filledTonal(
        icon: const Icon(Icons.save_outlined),
        onPressed: widget.editable ? _save : null,
      ),
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
            const SizedBox(height: 6),
            Align(alignment: Alignment.centerRight, child: saveButton),
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
          const SizedBox(width: 8),
          saveButton,
        ],
      ),
    );
  }

  void _save() {
    final baseValue = double.tryParse(_baseController.text.trim());
    final currentValue = double.tryParse(_currentController.text.trim());
    if (baseValue == null || currentValue == null) {
      setState(() => _error = 'Invalid');
      return;
    }
    setState(() => _error = null);
    widget.notifier.writePlayerAttribute(
      id: widget.attribute.id,
      baseValue: baseValue,
      currentValue: currentValue,
    );
  }
}

String _formatAttributeValue(double? value) {
  if (value == null) return '';
  if (value == value.roundToDouble()) return value.toInt().toString();
  return value.toStringAsFixed(2).replaceFirst(RegExp(r'\.?0+$'), '');
}

class _SummaryMetric extends StatelessWidget {
  const _SummaryMetric({required this.label, required this.value});

  final String label;
  final String value;

  @override
  Widget build(BuildContext context) {
    return SizedBox(
      width: 120,
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Text(
            label,
            maxLines: 1,
            overflow: TextOverflow.ellipsis,
            style: Theme.of(context).textTheme.labelSmall?.copyWith(
              color: Theme.of(context).colorScheme.onSurfaceVariant,
            ),
          ),
          Text(
            value,
            maxLines: 1,
            overflow: TextOverflow.ellipsis,
            style: Theme.of(context).textTheme.bodyMedium,
          ),
        ],
      ),
    );
  }
}

class _InventoryDiagnostics extends StatelessWidget {
  const _InventoryDiagnostics({
    required this.inventory,
    required this.inspection,
    this.expanded = false,
    this.onToggle,
  });

  final PrivateInventorySummary inventory;
  final SaveInspection inspection;
  final bool expanded;
  final VoidCallback? onToggle;

  @override
  Widget build(BuildContext context) {
    final candidates = inventory.candidates.take(60).toList();
    final decoded = inspection.privateStrings.take(40).toList();
    return Card(
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          mainAxisSize: expanded ? MainAxisSize.max : MainAxisSize.min,
          children: [
            _CollapsibleCardHeader(
              icon: Icons.science_outlined,
              title: 'Diagnostics & details',
              subtitle: 'Read-only format inspection',
              expanded: expanded,
              onToggle: onToggle,
            ),
            if (expanded)
              Expanded(
                child: ListView(
                  padding: const EdgeInsets.only(top: 12),
                  children: [
                    Text(
                      inspection.privateTypedVerified
                          ? 'Save layout verified: the typed property parse covers '
                                'every byte of the decoded payload. '
                                'Typed edits are available.'
                          : inspection.privateTypedParseStatus == 'failed'
                          ? 'Inventory candidates are discovered from decoded private '
                                'payload strings. Typed edits stay disabled: the typed '
                                'parse failed for this save.'
                          : 'Inventory candidates are discovered from decoded private '
                                'payload strings. Typed edits remain disabled until the '
                                'save layout is verified.',
                    ),
                    const SizedBox(height: 12),
                    Wrap(
                      spacing: 8,
                      runSpacing: 8,
                      children: [
                        if (inspection.privateTypedParseStatus != null)
                          _SummaryMetric(
                            label: 'Typed parse',
                            value: inspection.privateTypedVerified
                                ? 'Verified'
                                : inspection.privateTypedParseStatus!,
                          ),
                        if (inspection.privateTypedPropertyCount != null)
                          _SummaryMetric(
                            label: 'Typed properties',
                            value: _bytes.format(
                              inspection.privateTypedPropertyCount,
                            ),
                          ),
                        _SummaryMetric(
                          label: 'Candidates',
                          value: inventory.candidateCount.toString(),
                        ),
                        _SummaryMetric(
                          label: 'Item stacks',
                          value: inventory.itemStackCount.toString(),
                        ),
                        if (inventory.itemScope != null)
                          _SummaryMetric(
                            label: 'Scope',
                            value: _inventoryScopeLabel(inventory.itemScope!),
                          ),
                        _SummaryMetric(
                          label: 'Properties',
                          value: inventory.properties.length.toString(),
                        ),
                        _SummaryMetric(
                          label: 'Scripts',
                          value: inventory.scriptPaths.length.toString(),
                        ),
                      ],
                    ),
                    const SizedBox(height: 12),
                    Wrap(
                      spacing: 8,
                      runSpacing: 8,
                      children: [
                        _InfoChip(
                          label:
                              '${_bytes.format(inspection.privateDecompressedSize ?? 0)} bytes',
                        ),
                        _InfoChip(
                          label:
                              '${_bytes.format(inspection.privateStringCount ?? decoded.length)} strings',
                        ),
                      ],
                    ),
                    if (candidates.isNotEmpty) ...[
                      const SizedBox(height: 16),
                      Text(
                        'Candidate strings',
                        style: Theme.of(context).textTheme.labelLarge,
                      ),
                      const SizedBox(height: 8),
                      SizedBox(
                        height: 180,
                        child: ListView.separated(
                          itemCount: candidates.length,
                          separatorBuilder: (_, _) => const Divider(height: 1),
                          itemBuilder: (context, index) {
                            final value = candidates[index];
                            return ListTile(
                              dense: true,
                              leading: const Icon(Icons.inventory_outlined),
                              title: SelectableText(value, maxLines: 1),
                            );
                          },
                        ),
                      ),
                    ],
                    if (inventory.scriptPaths.isNotEmpty) ...[
                      const SizedBox(height: 16),
                      Text(
                        'Inventory scripts',
                        style: Theme.of(context).textTheme.labelLarge,
                      ),
                      const SizedBox(height: 8),
                      Wrap(
                        spacing: 8,
                        runSpacing: 8,
                        children: inventory.scriptPaths
                            .take(8)
                            .map(
                              (value) => Chip(label: Text(value, maxLines: 1)),
                            )
                            .toList(),
                      ),
                    ],
                    if (decoded.isNotEmpty) ...[
                      const SizedBox(height: 16),
                      Text(
                        'Decoded strings',
                        style: Theme.of(context).textTheme.labelLarge,
                      ),
                      const SizedBox(height: 8),
                      Wrap(
                        spacing: 8,
                        runSpacing: 8,
                        children: decoded
                            .map(
                              (value) => Chip(label: Text(value, maxLines: 1)),
                            )
                            .toList(),
                      ),
                    ],
                  ],
                ),
              ),
          ],
        ),
      ),
    );
  }
}

class _PrivateSummaryCard extends StatelessWidget {
  const _PrivateSummaryCard({
    required this.icon,
    required this.title,
    required this.body,
    required this.inspection,
  });

  final IconData icon;
  final String title;
  final String body;
  final SaveInspection inspection;

  @override
  Widget build(BuildContext context) {
    final strings = inspection.privateStrings.take(40).toList();
    return Card(
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Row(
              children: [
                Icon(icon, color: Theme.of(context).colorScheme.primary),
                const SizedBox(width: 8),
                Text(title, style: Theme.of(context).textTheme.titleMedium),
              ],
            ),
            const SizedBox(height: 8),
            Text(body),
            const SizedBox(height: 12),
            Wrap(
              spacing: 8,
              runSpacing: 8,
              children: [
                _InfoChip(
                  label:
                      '${_bytes.format(inspection.privateDecompressedSize ?? 0)} bytes',
                ),
                _InfoChip(
                  label:
                      '${_bytes.format(inspection.privateStringCount ?? strings.length)} strings',
                ),
              ],
            ),
            if (strings.isNotEmpty) ...[
              const SizedBox(height: 16),
              Text(
                'Decoded strings',
                style: Theme.of(context).textTheme.labelLarge,
              ),
              const SizedBox(height: 8),
              Wrap(
                spacing: 8,
                runSpacing: 8,
                children: strings
                    .map((value) => Chip(label: Text(value, maxLines: 1)))
                    .toList(),
              ),
            ],
          ],
        ),
      ),
    );
  }
}

class _InfoChip extends StatelessWidget {
  const _InfoChip({required this.label});

  final String label;

  @override
  Widget build(BuildContext context) {
    return Chip(
      label: Text(label),
      backgroundColor: Theme.of(context).colorScheme.primaryContainer,
      shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(8)),
    );
  }
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

  @override
  void initState() {
    super.initState();
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
    // A different save was selected while this tab stayed mounted. The cached
    // results belong to the old file; drop them (otherwise they show stale rows
    // while writes target the newly selected save) and re-list from page one.
    if (widget.inspection.path != oldWidget.inspection.path) {
      _controller.clear();
      _activeQuery = '';
      // Invalidate any in-flight search for the previous save.
      _requestSeq++;
      setState(() {
        _result = null;
        _searching = false;
      });
      if (widget.inspection.privateDecoded) {
        WidgetsBinding.instance.addPostFrameCallback((_) {
          if (mounted) _run(offset: 0);
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
            'the G1R codec host.',
      );
    }
    final theme = Theme.of(context);
    return Padding(
      padding: const EdgeInsets.all(20),
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
          onSaved: () => _run(offset: result.offset),
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
    required this.onSaved,
  });

  final TypedPropertyHit hit;
  final bool editable;
  final EditorNotifier notifier;
  final VoidCallback onSaved;

  @override
  State<_TypedPropertyRow> createState() => _TypedPropertyRowState();
}

class _TypedPropertyRowState extends State<_TypedPropertyRow> {
  late final TextEditingController _controller = TextEditingController(
    text: widget.hit.value,
  );

  @override
  void didUpdateWidget(covariant _TypedPropertyRow oldWidget) {
    super.didUpdateWidget(oldWidget);
    // A successful save refreshes the list and rebinds this row to a hit with
    // the persisted (possibly normalized) value. Sync the field to it so it
    // stops showing the pre-save text. Only rows whose value actually changed
    // update, so an unrelated row's save cannot clobber in-progress typing here.
    if (widget.hit.value != oldWidget.hit.value &&
        _controller.text != widget.hit.value) {
      _controller.text = widget.hit.value;
    }
  }

  @override
  void dispose() {
    _controller.dispose();
    super.dispose();
  }

  bool get _isBool => widget.hit.type == 'BoolProperty';

  Object? _coerce() {
    final type = widget.hit.type;
    if (type == 'StrProperty' || type == 'NameProperty') {
      // String values are written verbatim — leading/trailing whitespace may
      // be intentional, so no trim.
      return _controller.text;
    }
    if (type == 'ObjectProperty' || type == 'EnumProperty') {
      return _controller.text.trim();
    }
    final raw = _controller.text.trim();
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

  Future<void> _save(Object value) async {
    final ok = await widget.notifier.writeTypedValue(
      propertyPath: widget.hit.path,
      value: value,
    );
    // Only refresh the list when the core accepted the write; a rejected write
    // surfaces an error in state and must not look like a successful save.
    if (ok) widget.onSaved();
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
              value: hit.value == 'true',
              onChanged: (next) => _save(next),
            )
          else
            SizedBox(
              width: 220,
              child: Row(
                children: [
                  Expanded(
                    child: TextField(
                      controller: _controller,
                      decoration: const InputDecoration(
                        isDense: true,
                        labelText: 'Value',
                      ),
                    ),
                  ),
                  IconButton(
                    tooltip: 'Save ${hit.display}',
                    icon: const Icon(Icons.save_outlined),
                    onPressed: () {
                      final value = _coerce();
                      if (value != null) _save(value);
                    },
                  ),
                ],
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

class _AdvancedPanel extends StatelessWidget {
  const _AdvancedPanel({required this.inspection});

  final SaveInspection inspection;

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.all(20),
      child: Card(
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            Padding(
              padding: const EdgeInsets.fromLTRB(16, 12, 8, 8),
              child: Row(
                children: [
                  const Icon(Icons.data_object),
                  const SizedBox(width: 8),
                  Text(
                    'Inspection JSON',
                    style: Theme.of(context).textTheme.titleMedium,
                  ),
                  const Spacer(),
                  IconButton(
                    tooltip: 'Copy',
                    icon: const Icon(Icons.copy),
                    onPressed: () => Clipboard.setData(
                      ClipboardData(text: inspection.prettyJson()),
                    ),
                  ),
                ],
              ),
            ),
            const Divider(height: 1),
            Expanded(
              child: SingleChildScrollView(
                padding: const EdgeInsets.all(16),
                child: SelectableText(
                  inspection.prettyJson(),
                  style: const TextStyle(fontFamily: 'Consolas', fontSize: 12),
                ),
              ),
            ),
          ],
        ),
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
            'Companion backups',
            style: Theme.of(context).textTheme.titleMedium,
          ),
          const SizedBox(height: 8),
          ...companionBackups.map(
            (backup) => _BackupCard(
              backup: backup,
              isLoading: state.isLoading,
              showRestoreAction: false,
              onRestore: () {},
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
                      'G1R codec host',
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
                _PathSettingRow(
                  label: 'Helper',
                  value: state.codecHostPath,
                  onBrowse: notifier.chooseCodecHost,
                ),
                const SizedBox(height: 8),
                _PathSettingRow(
                  label: 'Game EXE',
                  value: state.gameExePath,
                  onBrowse: notifier.chooseGameExe,
                ),
                const SizedBox(height: 12),
                if (state.codecError != null)
                  Row(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      Icon(
                        Icons.error_outline,
                        color: Theme.of(context).colorScheme.error,
                        size: 18,
                      ),
                      const SizedBox(width: 6),
                      Expanded(
                        child: Text(
                          state.codecError!,
                          style: TextStyle(
                            color: Theme.of(context).colorScheme.error,
                          ),
                        ),
                      ),
                    ],
                  ),
                if (state.codecError != null) const SizedBox(height: 8),
                Text(codec?.message ?? 'No codec status'),
                if (codec != null) ...[
                  const SizedBox(height: 8),
                  Text(
                    'Decompress: ${codec.canDecompress ? 'yes' : 'no'} | Compress: ${codec.canCompress ? 'yes' : 'no'}',
                  ),
                  if (codec.selectedBackend != null)
                    Text('Backend: ${codec.selectedBackend}'),
                  if (codec.profile != null) Text('Profile: ${codec.profile}'),
                  if (codec.resolutionMode != null)
                    Text('Resolution: ${codec.resolutionMode}'),
                ],
              ],
            ),
          ),
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
