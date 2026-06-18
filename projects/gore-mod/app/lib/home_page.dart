import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_riverpod/legacy.dart';
import 'app/domain/ui_settings.dart';
import 'app/ui/window_chrome.dart';
import 'catalog/domain/item_entry.dart';
import 'catalog/ui/catalog_browser.dart';
import 'editor/domain/overrides_notifier.dart';
import 'editor/ui/field_editor.dart';
import 'editor/ui/overrides_panel.dart';
import 'export/ui/export_dialog.dart';

final _selectedItemProvider = StateProvider<CatalogItem?>((ref) => null);

class HomePage extends ConsumerWidget {
  const HomePage({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final selected       = ref.watch(_selectedItemProvider);
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
