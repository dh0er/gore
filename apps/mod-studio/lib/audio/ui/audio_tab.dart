import 'dart:io';

import 'package:file_selector/file_selector.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:path/path.dart' as p;

import '../../app/domain/ui_settings.dart';
import '../../app/game_paths.dart';
import '../../catalog/ui/sidebar_tile.dart';
import '../../core/mod_ffi.dart';
import '../../core/providers.dart';
import '../../l10n/app_localizations.dart';
import '../domain/audio_replacements_notifier.dart';
import '../domain/audio_samples_provider.dart';
import '../domain/sfx_categories.dart';

/// Browse the game's FMOD bank samples, preview originals, and stage WAV
/// replacements into [audioReplacementsProvider].
class AudioTab extends ConsumerWidget {
  const AudioTab({super.key, this.onlyStaged = false});

  /// When true, sample lists show only samples that have a staged replacement
  /// in the current bank (the Changes tab view). All banks stay selectable;
  /// a bank with nothing staged shows the usual empty state.
  final bool onlyStaged;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final fmodDir = fmodDesktopDir(ref.watch(gameExePathProvider));
    if (fmodDir == null) {
      return const Center(child: Text('Set the game path in Settings'));
    }
    return _AudioBrowser(fmodDir: fmodDir, onlyStaged: onlyStaged);
  }
}

/// Holds the per-tab selection state (selected bank, selected sample, search).
class _AudioBrowser extends ConsumerStatefulWidget {
  const _AudioBrowser({required this.fmodDir, required this.onlyStaged});

  final String fmodDir;
  final bool onlyStaged;

  @override
  ConsumerState<_AudioBrowser> createState() => _AudioBrowserState();
}

/// The only bank large enough to warrant the category split view.
const String _sfxBank = 'SFX.bank';

class _AudioBrowserState extends ConsumerState<_AudioBrowser>
    with SingleTickerProviderStateMixin {
  String _bankFileName = kModdableBanks.first;
  AudioSampleInfo? _selected;
  SfxCategory? _selectedCategory;
  String _query = '';
  final TextEditingController _searchController = TextEditingController();
  late final TabController _tabController;

  // Cache of the SFX category grouping, keyed on list identity: the provider
  // returns the same list instance across rebuilds, so regrouping 7k+ names
  // on every setState (sample click, category click) would be wasted work.
  List<AudioSampleInfo>? _lastGroupedSource;
  Map<SfxCategory, List<AudioSampleInfo>>? _lastGrouped;

  // Cache of the alphabetically sorted copy used by flat (non-category)
  // lists, keyed on list identity for the same reason as _lastGrouped:
  // re-sorting 7k+ samples per search keystroke would be wasted work.
  List<AudioSampleInfo>? _lastSortedSource;
  List<AudioSampleInfo>? _lastSorted;

  String get _bankFullPath => p.join(widget.fmodDir, _bankFileName);

  @override
  void initState() {
    super.initState();
    _tabController = TabController(
      length: kModdableBanks.length,
      initialIndex: kModdableBanks.indexOf(_bankFileName),
      vsync: this,
    );
    _tabController.addListener(
      () => _selectBank(kModdableBanks[_tabController.index]),
    );
  }

  @override
  void dispose() {
    _tabController.dispose();
    _searchController.dispose();
    super.dispose();
  }

  void _selectBank(String bankFileName) {
    if (bankFileName == _bankFileName) return;
    setState(() {
      _bankFileName = bankFileName;
      _selected = null;
      // A search only makes sense within one bank; drop it on switch so the
      // new bank starts from its full list (and SFX from its sidebar).
      _query = '';
      _searchController.clear();
    });
  }

  /// Tab label for a bank file: basename without extension, with all-caps
  /// names longer than an acronym title-cased ("CINEMATICS" -> "Cinematics").
  String _bankTabLabel(String bankFileName) {
    final base = p.basenameWithoutExtension(bankFileName);
    if (base.length > 3 && base == base.toUpperCase()) {
      return base[0] + base.substring(1).toLowerCase();
    }
    return base;
  }

  @override
  Widget build(BuildContext context) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        Expanded(
          child: Row(
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: [
              SizedBox(width: 560, child: _buildLeft(context)),
              const VerticalDivider(width: 1),
              Expanded(child: _buildDetail(context)),
            ],
          ),
        ),
        const Divider(height: 1),
        _StagedReplacementsPanel(),
      ],
    );
  }

  Widget _buildLeft(BuildContext context) {
    final theme = Theme.of(context);
    final samplesAsync = ref.watch(audioSamplesProvider(_bankFullPath));
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        // The bank tabs only switch what the browser column below shows, so
        // they live inside the 560px pane instead of spanning the whole tab.
        TabBar(
          controller: _tabController,
          tabs: [
            for (final bank in kModdableBanks) Tab(text: _bankTabLabel(bank)),
          ],
        ),
        Padding(
          padding: const EdgeInsets.fromLTRB(12, 12, 12, 0),
          child: TextField(
            controller: _searchController,
            decoration: InputDecoration(
              labelText: 'Search samples',
              prefixIcon: const Icon(Icons.search),
              isDense: true,
              suffixIcon: _query.isEmpty
                  ? null
                  : IconButton(
                      icon: const Icon(Icons.clear),
                      tooltip: 'Clear',
                      onPressed: () {
                        _searchController.clear();
                        setState(() => _query = '');
                      },
                    ),
            ),
            onChanged: (v) => setState(() => _query = v),
          ),
        ),
        const SizedBox(height: 8),
        Expanded(
          child: samplesAsync.when(
            loading: () => const Center(child: CircularProgressIndicator()),
            error: (e, _) => Center(
              child: Padding(
                padding: const EdgeInsets.all(16),
                child: Text(
                  'Failed to load samples: $e',
                  style: TextStyle(color: theme.colorScheme.error),
                ),
              ),
            ),
            data: (samples) => _buildSampleList(context, samples),
          ),
        ),
      ],
    );
  }

  /// Sample names of the current bank with a staged replacement, or null when
  /// this browser shows all samples (default). Watches
  /// [audioReplacementsProvider] so un-staging updates the filtered lists live.
  Set<String>? _stagedNamesOrNull() {
    if (!widget.onlyStaged) return null;
    final items = ref.watch(audioReplacementsProvider).items;
    // Replacement keys are '$bank/$sample' (bank names contain no '/').
    final prefix = '$_bankFileName/';
    return {
      for (final key in items.keys)
        if (key.startsWith(prefix)) key.substring(prefix.length),
    };
  }

  Widget _buildSampleList(BuildContext context, List<AudioSampleInfo> samples) {
    final query = _query.trim().toLowerCase();
    final searching = query.isNotEmpty;
    // The staged-only filter applies AFTER the identity-memoized sort/group
    // caches below: the staged set is tiny and changes independently of the
    // bank's sample list, so filtering the cached results per build is cheap,
    // while keying the big caches on it would defeat their memoization.
    final stagedNames = _stagedNamesOrNull();

    // The SFX bank is huge (7k+ samples), so browse it through a category
    // sidebar (buckets sorted at grouping time). An active search hides the
    // sidebar and scans the whole bank.
    if (_bankFileName == _sfxBank && !searching) {
      return _buildSfxSplitView(context, samples, stagedNames);
    }

    // Flat list (non-SFX banks, or any bank during a search): display in
    // alphabetical order rather than FSB bank order. Filter over the cached
    // pre-sorted copy so a keystroke never re-sorts the whole bank.
    var filtered = _sortedByName(samples);
    if (stagedNames != null) {
      filtered = [
        for (final s in filtered)
          if (stagedNames.contains(s.name)) s,
      ];
    }
    if (searching) {
      filtered =
          filtered.where((s) => s.name.toLowerCase().contains(query)).toList();
    }
    return _sampleListView(context, filtered);
  }

  static int _byNameCaseInsensitive(AudioSampleInfo a, AudioSampleInfo b) =>
      a.name.toLowerCase().compareTo(b.name.toLowerCase());

  /// Sorted copy of [samples] (never sorts the provider's list in place),
  /// memoized on list identity.
  List<AudioSampleInfo> _sortedByName(List<AudioSampleInfo> samples) {
    if (identical(samples, _lastSortedSource)) return _lastSorted!;
    final sorted = [...samples]..sort(_byNameCaseInsensitive);
    _lastSortedSource = samples;
    _lastSorted = sorted;
    return sorted;
  }

  Map<SfxCategory, List<AudioSampleInfo>> _groupedSfx(
      List<AudioSampleInfo> samples) {
    if (identical(samples, _lastGroupedSource)) return _lastGrouped!;
    final grouped = <SfxCategory, List<AudioSampleInfo>>{};
    for (final s in samples) {
      grouped.putIfAbsent(sfxCategoryForSample(s.name), () => []).add(s);
    }
    // Buckets are fresh copies, so sorting them in place is safe; category
    // lists then render alphabetically instead of in FSB bank order.
    for (final bucket in grouped.values) {
      bucket.sort(_byNameCaseInsensitive);
    }
    _lastGroupedSource = samples;
    _lastGrouped = grouped;
    return grouped;
  }

  Widget _buildSfxSplitView(BuildContext context,
      List<AudioSampleInfo> samples, Set<String>? stagedNames) {
    final theme = Theme.of(context);
    final l10n = AppLocalizations.of(context);

    final grouped = _groupedSfx(samples);
    // Staged-only view: filter the cached buckets per build (cheap, see
    // _buildSampleList) so sidebar counts reflect staged samples only and
    // categories left empty by the filter disappear.
    final visible = <SfxCategory, List<AudioSampleInfo>>{};
    for (final entry in grouped.entries) {
      final bucket = stagedNames == null
          ? entry.value
          : [
              for (final s in entry.value)
                if (stagedNames.contains(s.name)) s,
            ];
      if (bucket.isNotEmpty) visible[entry.key] = bucket;
    }
    final categories = [
      for (final c in SfxCategory.values)
        if (visible.containsKey(c)) c,
    ];
    if (categories.isEmpty) {
      return const Center(child: Text('No samples match'));
    }

    // Resolve selected category, falling back to the first available.
    var selectedCat = _selectedCategory;
    if (selectedCat == null || !visible.containsKey(selectedCat)) {
      selectedCat = categories.first;
    }

    return Row(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        SizedBox(
          width: 230,
          child: DecoratedBox(
            decoration: BoxDecoration(
              color: theme.colorScheme.surfaceContainerLow,
            ),
            child: ListView(
              padding: const EdgeInsets.symmetric(vertical: 6),
              children: [
                for (final c in categories)
                  SidebarTile(
                    icon: Icons.graphic_eq,
                    label: l10n.categoryWithCount(
                      c.localizedLabel(l10n),
                      visible[c]!.length,
                    ),
                    selected: c == selectedCat,
                    onTap: () => setState(() => _selectedCategory = c),
                  ),
              ],
            ),
          ),
        ),
        const VerticalDivider(width: 1),
        Expanded(child: _sampleListView(context, visible[selectedCat]!)),
      ],
    );
  }

  Widget _sampleListView(BuildContext context, List<AudioSampleInfo> samples) {
    final theme = Theme.of(context);
    final replacements = ref.watch(audioReplacementsProvider);

    if (samples.isEmpty) {
      return const Center(child: Text('No samples match'));
    }

    return ListView.builder(
      itemCount: samples.length,
      itemBuilder: (context, index) {
        final sample = samples[index];
        final isSelected = _selected?.name == sample.name;
        final replaced = replacements.items.containsKey(
          AudioReplacement(
            bank: _bankFileName,
            sample: sample.name,
            wavPath: '',
          ).key,
        );
        return ListTile(
          dense: true,
          selected: isSelected,
          selectedTileColor: theme.colorScheme.primaryContainer,
          leading: Icon(
            replaced ? Icons.fiber_manual_record : Icons.music_note,
            size: 18,
            color: replaced ? theme.colorScheme.primary : null,
          ),
          title: Text(
            sample.name,
            maxLines: 1,
            overflow: TextOverflow.ellipsis,
          ),
          onTap: () => setState(() => _selected = sample),
        );
      },
    );
  }

  Widget _buildDetail(BuildContext context) {
    final sample = _selected;
    // Staged-only view: when the selected sample's replacement is un-staged it
    // drops out of the list, so the detail pane must not keep offering it
    // (parity with the other filtered views: Dialog, Items, Textures). A
    // render guard suffices — build watches audioReplacementsProvider, so
    // un-staging swaps this pane to the placeholder live.
    final stagedNames = _stagedNamesOrNull();
    if (sample == null ||
        (stagedNames != null && !stagedNames.contains(sample.name))) {
      return const Center(child: Text('Select a sample'));
    }
    final theme = Theme.of(context);
    final replacements = ref.watch(audioReplacementsProvider);
    final key = AudioReplacement(
      bank: _bankFileName,
      sample: sample.name,
      wavPath: '',
    ).key;
    final staged = replacements.items[key];

    return SingleChildScrollView(
      padding: const EdgeInsets.all(24),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Text(sample.name, style: theme.textTheme.titleLarge),
          const SizedBox(height: 16),
          _detailRow(context, 'Bank', _bankFileName),
          _detailRow(context, 'Frequency', '${sample.freq} Hz'),
          _detailRow(context, 'Channels', '${sample.channels}'),
          _detailRow(
            context,
            'Duration',
            '${sample.seconds.toStringAsFixed(2)} s',
          ),
          const SizedBox(height: 24),
          Row(
            children: [
              OutlinedButton.icon(
                onPressed: () => _preview(context, sample),
                icon: const Icon(Icons.play_arrow),
                label: const Text('Preview'),
              ),
              const SizedBox(width: 12),
              FilledButton.icon(
                onPressed: () => _replace(context, sample),
                icon: const Icon(Icons.swap_horiz),
                label: const Text('Replace…'),
              ),
            ],
          ),
          if (staged != null) ...[
            const SizedBox(height: 24),
            DecoratedBox(
              decoration: BoxDecoration(
                color: theme.colorScheme.surfaceContainerHighest,
                borderRadius: BorderRadius.circular(8),
              ),
              child: Padding(
                padding: const EdgeInsets.all(12),
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Text(
                      'Staged replacement',
                      style: theme.textTheme.labelLarge,
                    ),
                    const SizedBox(height: 8),
                    Row(
                      children: [
                        const Icon(Icons.audiotrack, size: 18),
                        const SizedBox(width: 8),
                        Expanded(
                          child: Text(
                            p.basename(staged.wavPath),
                            maxLines: 1,
                            overflow: TextOverflow.ellipsis,
                          ),
                        ),
                        const SizedBox(width: 8),
                        TextButton.icon(
                          onPressed: () => ref
                              .read(audioReplacementsProvider.notifier)
                              .remove(key),
                          icon: const Icon(Icons.delete_outline),
                          label: const Text('Remove'),
                        ),
                      ],
                    ),
                  ],
                ),
              ),
            ),
          ],
        ],
      ),
    );
  }

  Widget _detailRow(BuildContext context, String label, String value) {
    final theme = Theme.of(context);
    return Padding(
      padding: const EdgeInsets.symmetric(vertical: 4),
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          SizedBox(
            width: 120,
            child: Text(
              label,
              style: theme.textTheme.bodyMedium?.copyWith(
                color: theme.colorScheme.onSurfaceVariant,
              ),
            ),
          ),
          Expanded(
            child: Text(value, style: theme.textTheme.bodyMedium),
          ),
        ],
      ),
    );
  }

  Future<void> _preview(BuildContext context, AudioSampleInfo sample) async {
    final messenger = ScaffoldMessenger.of(context);
    try {
      // If the user has staged a replacement WAV for this sample, preview THAT instead of the
      // original bank audio, so Preview reflects what will be deployed.
      final bankName = _bankFullPath.split(RegExp(r'[\\/]')).last;
      final staged = ref.read(audioReplacementsProvider).items['$bankName/${sample.name}'];
      final path = staged?.wavPath ??
          await ModFfi(ref.read(coreServiceProvider)).audioExtract(_bankFullPath, sample.name);
      if (Platform.isWindows) {
        // Pass the path as its own argument (no runInShell): Process.start quotes args with
        // spaces when building the Windows command line, so `cmd /c start "" "<path>"` handles
        // paths under e.g. "Program Files". runInShell would re-parse and drop that quoting.
        await Process.start(
          'cmd',
          ['/c', 'start', '', path],
          runInShell: false,
        );
      } else if (Platform.isMacOS) {
        await Process.start('open', [path]);
      } else {
        await Process.start('xdg-open', [path]);
      }
    } catch (e) {
      messenger.showSnackBar(
        SnackBar(content: Text('Preview failed: $e')),
      );
    }
  }

  Future<void> _replace(BuildContext context, AudioSampleInfo sample) async {
    final notifier = ref.read(audioReplacementsProvider.notifier);
    final group = const XTypeGroup(label: 'WAV audio', extensions: ['wav']);
    final file = await openFile(acceptedTypeGroups: [group]);
    if (file == null) return;
    notifier.setReplacement(
      AudioReplacement(
        bank: _bankFileName,
        sample: sample.name,
        wavPath: file.path,
      ),
    );
  }
}

/// Collapsible panel listing every staged audio replacement across all banks.
class _StagedReplacementsPanel extends ConsumerWidget {
  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final theme = Theme.of(context);
    final state = ref.watch(audioReplacementsProvider);
    final entries = state.entries;

    return Theme(
      data: theme.copyWith(dividerColor: Colors.transparent),
      child: ExpansionTile(
        initiallyExpanded: false,
        leading: const Icon(Icons.layers),
        title: Text('Staged replacements (${entries.length})'),
        childrenPadding: const EdgeInsets.only(bottom: 8),
        children: [
          if (entries.isEmpty)
            const Padding(
              padding: EdgeInsets.symmetric(horizontal: 16, vertical: 8),
              child: Align(
                alignment: Alignment.centerLeft,
                child: Text('No replacements staged yet'),
              ),
            )
          else
            // Cap the expanded area so many staged replacements scroll inside
            // the panel instead of overflowing the tab (shrinkWrap keeps a
            // short list at its natural height).
            ConstrainedBox(
              constraints: const BoxConstraints(maxHeight: 240),
              child: ListView.builder(
                shrinkWrap: true,
                itemCount: entries.length,
                itemBuilder: (context, index) {
                  final r = entries[index];
                  return ListTile(
                    dense: true,
                    leading: const Icon(Icons.audiotrack, size: 18),
                    title: Text(
                      '${r.bank} • ${r.sample}',
                      maxLines: 1,
                      overflow: TextOverflow.ellipsis,
                    ),
                    subtitle: Text(
                      p.basename(r.wavPath),
                      maxLines: 1,
                      overflow: TextOverflow.ellipsis,
                    ),
                    trailing: IconButton(
                      icon: const Icon(Icons.delete_outline),
                      tooltip: 'Remove',
                      onPressed: () => ref
                          .read(audioReplacementsProvider.notifier)
                          .remove(r.key),
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
