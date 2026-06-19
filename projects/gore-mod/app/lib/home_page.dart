import 'package:collection/collection.dart';
import 'package:file_selector/file_selector.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_riverpod/legacy.dart';
import 'package:path/path.dart' as p;
import 'app/domain/ui_settings.dart';
import 'app/ui/window_chrome.dart';
import 'catalog/domain/catalog_provider.dart';
import 'catalog/domain/item_entry.dart';
import 'catalog/ui/catalog_browser.dart';
import 'editor/domain/overrides_notifier.dart';
import 'editor/ui/field_editor.dart';
import 'editor/ui/overrides_panel.dart';
import 'export/ui/export_dialog.dart';
import 'loc/domain/loc_notifier.dart';
import 'loc/ui/loc_extract_flow.dart';

final _selectedItemProvider = StateProvider<CatalogItem?>((ref) => null);

class HomePage extends ConsumerStatefulWidget {
  const HomePage({super.key});

  @override
  ConsumerState<HomePage> createState() => _HomePageState();
}

class _HomePageState extends ConsumerState<HomePage> {
  @override
  void initState() {
    super.initState();
    // First-run, optional: after the first frame, if no localized text has
    // been extracted yet and the user hasn't been prompted before, offer to
    // extract it. Records the prompt so it only auto-fires once.
    WidgetsBinding.instance.addPostFrameCallback((_) => _maybeFirstRunPrompt());
  }

  Future<void> _maybeFirstRunPrompt() async {
    if (ref.read(locExtractPromptedProvider)) return;
    final present = await ref.read(locProvider.notifier).status();
    if (!mounted || present) return;
    ref.read(locExtractPromptedProvider.notifier).markPrompted();
    final shouldExtract = await showLocFirstRunDialog(context);
    if (!mounted || !shouldExtract) return;
    await runLocExtractFlow(context, ref);
  }

  @override
  Widget build(BuildContext context) {
    // Switching the model data source invalidates pending overrides and the
    // current selection: fields may be removed/renamed or enum backing values
    // may change, so exporting old assignments could be wrong. Clear both when
    // the dump source changes.
    ref.listen(dumpPathProvider, (prev, next) {
      if (prev != next) {
        ref.read(overridesProvider.notifier).clearAll();
        ref.read(_selectedItemProvider.notifier).state = null;
      }
    });

    final selectedRaw    = ref.watch(_selectedItemProvider);
    // Re-resolve the selection against the current catalog so that loading or
    // resetting a dump re-renders the editor with the refreshed item (same id,
    // new fields/defaults) instead of the stale CatalogItem object.
    final selected = selectedRaw == null
        ? null
        : (ref.watch(catalogProvider).value
                ?.firstWhereOrNull((i) => i.id == selectedRaw.id) ??
            selectedRaw);
    final overridesState = ref.watch(overridesProvider);
    final themeModeNotifier = ref.read(themeModeProvider.notifier);
    final scheme         = Theme.of(context).colorScheme;
    final isDark         = Theme.of(context).brightness == Brightness.dark;

    return Scaffold(
      appBar: AppBar(
        title: WindowDragArea(
          child: Row(
            children: [
              const SizedBox(width: 16),
              const Text('gore-mod'),
              const Expanded(child: SizedBox()),
            ],
          ),
        ),
        titleSpacing: 0,
        centerTitle: false,
        scrolledUnderElevation: 0,
        actions: [
          _DumpMenu(
            dumpPath: ref.watch(dumpPathProvider),
            onLoad: () async {
              const group = XTypeGroup(label: 'game data', extensions: ['json']);
              final file = await openFile(acceptedTypeGroups: [group]);
              if (file != null) {
                ref.read(dumpPathProvider.notifier).set(file.path);
              }
            },
            onReset: () => ref.read(dumpPathProvider.notifier).clear(),
          ),
          IconButton(
            icon: const Icon(Icons.translate),
            tooltip: 'Extract localized text',
            onPressed: ref.watch(locProvider).isRunning
                ? null
                : () => runLocExtractFlow(context, ref),
          ),
          IconButton(
            icon: Icon(isDark ? Icons.light_mode : Icons.dark_mode),
            tooltip: isDark ? 'Light mode' : 'Dark mode',
            onPressed: () {
              themeModeNotifier.setThemeMode(
                isDark ? ThemeMode.light : ThemeMode.dark,
              );
            },
          ),
          Padding(
            padding: const EdgeInsets.symmetric(horizontal: 8),
            child: FilledButton.icon(
              icon: const Icon(Icons.upload_outlined, size: 18),
              label: Text(
                overridesState.count == 0
                    ? 'Export mod'
                    : 'Export mod (${overridesState.count})',
              ),
              onPressed: overridesState.count == 0
                  ? null
                  : () => showDialog(
                        context: context,
                        builder: (_) => const ExportDialog(),
                      ),
            ),
          ),
          const WindowControls(),
          const SizedBox(width: 8),
        ],
      ),
      body: Row(
        children: [
          // Left: catalog browser
          SizedBox(
            width: 560,
            child: CatalogBrowser(
              selected: selected,
              onItemSelected: (item) =>
                  ref.read(_selectedItemProvider.notifier).state = item,
            ),
          ),
          const VerticalDivider(width: 1),
          // Centre: field editor. Cap the editing column width and centre it so
          // the inputs don't stretch across the whole window on wide displays.
          Expanded(
            child: selected == null
                ? Center(
                    child: Text(
                      'Select an item to edit its fields.',
                      style: TextStyle(color: scheme.onSurfaceVariant),
                    ),
                  )
                : Align(
                    alignment: Alignment.topCenter,
                    child: ConstrainedBox(
                      constraints: const BoxConstraints(maxWidth: 720),
                      child: FieldEditor(
                        item: selected,
                        pendingOverrides: {
                          for (final e in overridesState.entries
                              .where((e) => e.classId == selected.id))
                            e.field: e,
                        },
                        onOverrideChanged: (entry) => ref
                            .read(overridesProvider.notifier)
                            .setOverride(entry),
                      ),
                    ),
                  ),
          ),
          const VerticalDivider(width: 1),
          // Right: overrides panel
          SizedBox(
            width: 460,
            child: const OverridesPanel(),
          ),
        ],
      ),
    );
  }
}

/// AppBar control for loading a fresh game-data dump (from the gore-dump mod)
/// that overrides the bundled model — the post-release refresh path. Shows
/// whether a dump is active and offers a reset to the bundled data.
class _DumpMenu extends StatelessWidget {
  const _DumpMenu({
    required this.dumpPath,
    required this.onLoad,
    required this.onReset,
  });

  final String? dumpPath;
  final Future<void> Function() onLoad;
  final void Function() onReset;

  @override
  Widget build(BuildContext context) {
    final active = dumpPath != null;
    final scheme = Theme.of(context).colorScheme;
    return PopupMenuButton<String>(
      tooltip: active ? 'Game data: ${p.basename(dumpPath!)}' : 'Game data: bundled',
      icon: Icon(
        active ? Icons.dataset : Icons.dataset_outlined,
        color: active ? scheme.primary : null,
      ),
      onSelected: (value) {
        if (value == 'load') {
          onLoad();
        } else if (value == 'reset') {
          onReset();
        }
      },
      itemBuilder: (context) => [
        const PopupMenuItem(
          value: 'load',
          child: ListTile(
            leading: Icon(Icons.upload_file),
            title: Text('Load game-data dump…'),
            subtitle: Text('gore_game_data.json from the gore-dump mod'),
            contentPadding: EdgeInsets.zero,
          ),
        ),
        PopupMenuItem(
          value: 'reset',
          enabled: active,
          child: ListTile(
            leading: const Icon(Icons.restore),
            title: const Text('Use bundled data'),
            subtitle: Text(active ? p.basename(dumpPath!) : 'already bundled'),
            contentPadding: EdgeInsets.zero,
          ),
        ),
      ],
    );
  }
}
