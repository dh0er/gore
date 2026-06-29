import 'package:collection/collection.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_riverpod/legacy.dart';
import 'app/domain/ui_settings.dart';
import 'app/game_paths.dart';
import 'app/ui/window_chrome.dart';
import 'catalog/domain/catalog_provider.dart';
import 'catalog/domain/item_entry.dart';
import 'catalog/ui/catalog_browser.dart';
import 'core/mod_ffi.dart';
import 'core/providers.dart';
import 'audio/domain/audio_replacements_notifier.dart';
import 'audio/ui/audio_tab.dart';
import 'dialog/ui/dialoge_tab.dart';
import 'editor/domain/overrides_notifier.dart';
import 'editor/ui/field_editor.dart';
import 'editor/ui/overrides_panel.dart';
import 'export/ui/build_deploy_dialog.dart';
import 'l10n/app_localizations.dart';
import 'loc/domain/loc_catalog_provider.dart';
import 'loc/domain/loc_edits_notifier.dart';
import 'loc/domain/loc_notifier.dart';
import 'loc/game_lang.dart';
import 'loc/ui/loc_extract_flow.dart';
import 'project/project_controller.dart';
import 'settings/ui/settings_tab.dart';
import 'textures/domain/texture_replacements_notifier.dart';
import 'textures/ui/texture_tab.dart';

final _selectedItemProvider = StateProvider<CatalogItem?>((ref) => null);

class HomePage extends ConsumerStatefulWidget {
  const HomePage({super.key});

  @override
  ConsumerState<HomePage> createState() => _HomePageState();
}

class _HomePageState extends ConsumerState<HomePage> with WidgetsBindingObserver {
  @override
  void initState() {
    super.initState();
    WidgetsBinding.instance.addObserver(this);
    // First-run, optional: after the first frame, if no localized text has
    // been extracted yet and the user hasn't been prompted before, offer to
    // extract it.
    WidgetsBinding.instance.addPostFrameCallback((_) => _maybeFirstRunPrompt());
    WidgetsBinding.instance.addPostFrameCallback((_) => _maybeAutoDetectGamePath());
  }

  /// On first run, if no game path is set, auto-detect the Steam install and save it.
  Future<void> _maybeAutoDetectGamePath() async {
    if (ref.read(gameExePathProvider) != null) return;
    try {
      final exe = await ModFfi(ref.read(coreServiceProvider)).findGameExe();
      if (!mounted || exe == null) return;
      if (ref.read(gameExePathProvider) == null) {
        ref.read(gameExePathProvider.notifier).set(exe);
      }
    } catch (_) {
      // best-effort; user can still set it manually in Settings
    }
  }

  @override
  void dispose() {
    WidgetsBinding.instance.removeObserver(this);
    super.dispose();
  }

  @override
  void didChangeAppLifecycleState(AppLifecycleState state) {
    // Another tool (or `gore-cli loc extract`) may have written the shared
    // loc_catalog.json while this app was in the background. Re-read it on
    // resume so item names pick up a catalog that appeared after first load.
    if (state == AppLifecycleState.resumed) {
      ref.invalidate(locCatalogProvider);
    }
  }

  Future<void> _maybeFirstRunPrompt() async {
    if (ref.read(locExtractPromptedProvider)) return;
    final present = await ref.read(locProvider.notifier).status();
    // Only prompt when the catalog is definitively absent: a null status means
    // the query failed (e.g. core unavailable), where extraction can't work, so
    // don't nag with the dialog.
    if (!mounted || present != false) return;
    final shouldExtract = await showLocFirstRunDialog(context);
    if (!mounted || !shouldExtract) return;
    // Record only once the user actually chose to extract, so the prompt isn't
    // marked as shown when the dialog never appeared, and deferring ("Not now")
    // lets the optional prompt offer again on a later launch.
    ref.read(locExtractPromptedProvider.notifier).markPrompted();
    await runLocExtractFlow(context, ref);
  }

  void _snack(String message) {
    if (!mounted) return;
    ScaffoldMessenger.of(context).showSnackBar(SnackBar(content: Text(message)));
  }

  Future<void> _saveProject() async {
    try {
      final path = await saveProjectInteractive(ref);
      if (path != null) _snack('Saved project to $path');
    } catch (e) {
      _snack('Save failed: $e');
    }
  }

  // Unsaved = there is staged content AND it differs from the last saved/loaded project, so a
  // project that was just saved doesn't prompt to discard on New/Open.
  bool _hasUnsavedEdits() => hasUnsavedChanges(ref);

  /// Confirm before discarding staged (unsaved) edits. Returns true to proceed.
  Future<bool> _confirmDiscardIfDirty() async {
    if (!_hasUnsavedEdits()) return true;
    final ok = await showDialog<bool>(
      context: context,
      builder: (ctx) => AlertDialog(
        title: const Text('Discard unsaved changes?'),
        content: const Text(
            'You have staged edits that are not saved to a project. Continue and discard them?'),
        actions: [
          TextButton(onPressed: () => Navigator.pop(ctx, false), child: const Text('Cancel')),
          FilledButton(onPressed: () => Navigator.pop(ctx, true), child: const Text('Discard')),
        ],
      ),
    );
    return ok ?? false;
  }

  Future<void> _newProject() async {
    if (await _confirmDiscardIfDirty()) newProject(ref);
  }

  Future<void> _openProject() async {
    if (!await _confirmDiscardIfDirty()) return;
    try {
      final path = await openProjectInteractive(ref);
      if (path != null) _snack('Loaded project $path');
    } catch (e) {
      _snack('Open failed: $e');
    }
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
    final dirty = overridesState.count > 0 ||
        ref.watch(locEditsProvider).isDirty ||
        ref.watch(audioReplacementsProvider).count > 0 ||
        ref.watch(textureReplacementsProvider).count > 0;
    // Keep Build/Deploy reachable when a game is configured even with no staged edits, so the
    // dialog's Undeploy (restore *.gore-bak) stays available to GUI users.
    final gameConfigured = gameRootFromExe(ref.watch(gameExePathProvider)) != null;
    final themeModeNotifier = ref.read(themeModeProvider.notifier);
    final scheme         = Theme.of(context).colorScheme;
    final isDark         = Theme.of(context).brightness == Brightness.dark;
    final l10n           = AppLocalizations.of(context);

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
            tooltip: isDark ? l10n.lightMode : l10n.darkMode,
            onPressed: () {
              themeModeNotifier.setThemeMode(
                isDark ? ThemeMode.light : ThemeMode.dark,
              );
            },
          ),
          PopupMenuButton<String>(
            icon: const Icon(Icons.folder_open_outlined),
            tooltip: 'Project',
            onSelected: (v) {
              switch (v) {
                case 'new':
                  _newProject();
                case 'open':
                  _openProject();
                case 'save':
                  _saveProject();
              }
            },
            itemBuilder: (_) => const [
              PopupMenuItem(value: 'new', child: Text('New project')),
              PopupMenuItem(value: 'open', child: Text('Open project…')),
              PopupMenuItem(value: 'save', child: Text('Save project as…')),
            ],
          ),
          Padding(
            padding: const EdgeInsets.symmetric(horizontal: 8),
            child: FilledButton.icon(
              icon: const Icon(Icons.rocket_launch_outlined, size: 18),
              label: const Text('Build / Deploy'),
              onPressed: (dirty || gameConfigured)
                  ? () => showDialog(
                        context: context,
                        builder: (_) => const BuildDeployDialog(),
                      )
                  : null,
            ),
          ),
          const WindowControls(),
          const SizedBox(width: 8),
        ],
      ),
      body: DefaultTabController(
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
                      tabs: [
                        Tab(
                          icon: const Icon(Icons.inventory_2_outlined),
                          text: l10n.tabItems,
                        ),
                        const Tab(
                          icon: Icon(Icons.forum_outlined),
                          text: 'Dialoge',
                        ),
                        const Tab(
                          icon: Icon(Icons.audiotrack_outlined),
                          text: 'Audio',
                        ),
                        const Tab(
                          icon: Icon(Icons.texture),
                          text: 'Textures',
                        ),
                        Tab(
                          icon: const Icon(Icons.edit_note_outlined),
                          text: l10n.tabOverrides,
                        ),
                        Tab(
                          icon: const Icon(Icons.settings_outlined),
                          text: l10n.tabSettings,
                        ),
                      ],
                    ),
                  ),
                ],
              ),
            ),
            Expanded(
              child: TabBarView(
                children: [
                  // Items: catalog browser + field editor.
                  Row(
                    children: [
                      // Left: catalog browser
                      SizedBox(
                        width: 560,
                        child: CatalogBrowser(
                          selected: selected,
                          onItemSelected: (item) => ref
                              .read(_selectedItemProvider.notifier)
                              .state = item,
                        ),
                      ),
                      const VerticalDivider(width: 1),
                      // Centre: field editor. Cap the editing column width and
                      // centre it so the inputs don't stretch across the whole
                      // window on wide displays.
                      Expanded(
                        child: selected == null
                            ? Center(
                                child: Text(
                                  l10n.selectAnItemToEdit,
                                  style: TextStyle(
                                      color: scheme.onSurfaceVariant),
                                ),
                              )
                            : Align(
                                alignment: Alignment.topCenter,
                                child: ConstrainedBox(
                                  constraints:
                                      const BoxConstraints(maxWidth: 720),
                                  child: FieldEditor(
                                    item: selected,
                                    displayName: displayNameForItem(
                                      selected,
                                      ref.watch(locCatalogProvider).value ??
                                          const {},
                                      gameLangByCode(
                                          ref.watch(localeProvider)),
                                    ),
                                    pendingOverrides: {
                                      for (final e in overridesState.entries
                                          .where((e) =>
                                              e.classId == selected.id))
                                        e.field: e,
                                    },
                                    onOverrideChanged: (entry) => ref
                                        .read(overridesProvider.notifier)
                                        .setOverride(entry),
                                  ),
                                ),
                              ),
                      ),
                    ],
                  ),
                  // Dialoge: localized dialog/bark line editor.
                  const DialogeTab(),
                  // Audio: FMOD bank sample browser + replacement.
                  const AudioTab(),
                  // Textures: texture asset browser + replacement.
                  const TextureTab(),
                  // Changes: all staged item/loc/audio changes, centred.
                  Align(
                    alignment: Alignment.topCenter,
                    child: ConstrainedBox(
                      constraints: const BoxConstraints(maxWidth: 600),
                      child: const OverridesPanel(),
                    ),
                  ),
                  // Settings.
                  const SettingsTab(),
                ],
              ),
            ),
          ],
        ),
      ),
    );
  }
}

