import 'dart:io';

import 'package:file_selector/file_selector.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:path/path.dart' as p;

import '../../app/domain/ui_settings.dart';
import '../../app/game_paths.dart';
import '../../core/mod_ffi.dart';
import '../../core/providers.dart';
import '../domain/audio_replacements_notifier.dart';
import '../domain/audio_samples_provider.dart';

/// Browse the game's FMOD bank samples, preview originals, and stage WAV
/// replacements into [audioReplacementsProvider].
class AudioTab extends ConsumerWidget {
  const AudioTab({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final fmodDir = fmodDesktopDir(ref.watch(gameExePathProvider));
    if (fmodDir == null) {
      return const Center(child: Text('Set the game path in Settings'));
    }
    return _AudioBrowser(fmodDir: fmodDir);
  }
}

/// Holds the per-tab selection state (selected bank, selected sample, search).
class _AudioBrowser extends ConsumerStatefulWidget {
  const _AudioBrowser({required this.fmodDir});

  final String fmodDir;

  @override
  ConsumerState<_AudioBrowser> createState() => _AudioBrowserState();
}

class _AudioBrowserState extends ConsumerState<_AudioBrowser> {
  String _bankFileName = kModdableBanks.first;
  AudioSampleInfo? _selected;
  String _query = '';
  final TextEditingController _searchController = TextEditingController();

  String get _bankFullPath => p.join(widget.fmodDir, _bankFileName);

  @override
  void dispose() {
    _searchController.dispose();
    super.dispose();
  }

  void _selectBank(String bankFileName) {
    if (bankFileName == _bankFileName) return;
    setState(() {
      _bankFileName = bankFileName;
      _selected = null;
    });
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
        Padding(
          padding: const EdgeInsets.all(12),
          child: Wrap(
            spacing: 8,
            runSpacing: 8,
            children: [
              for (final bank in kModdableBanks)
                ChoiceChip(
                  label: Text(bank),
                  selected: bank == _bankFileName,
                  onSelected: (_) => _selectBank(bank),
                ),
            ],
          ),
        ),
        Padding(
          padding: const EdgeInsets.symmetric(horizontal: 12),
          child: TextField(
            controller: _searchController,
            decoration: const InputDecoration(
              labelText: 'Search samples',
              prefixIcon: Icon(Icons.search),
              isDense: true,
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

  Widget _buildSampleList(BuildContext context, List<AudioSampleInfo> samples) {
    final theme = Theme.of(context);
    final replacements = ref.watch(audioReplacementsProvider);
    final query = _query.trim().toLowerCase();
    final filtered = query.isEmpty
        ? samples
        : samples
            .where((s) => s.name.toLowerCase().contains(query))
            .toList();

    if (filtered.isEmpty) {
      return const Center(child: Text('No samples match'));
    }

    return ListView.builder(
      itemCount: filtered.length,
      itemBuilder: (context, index) {
        final sample = filtered[index];
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
    if (sample == null) {
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
        await Process.start(
          'cmd',
          ['/c', 'start', '', path],
          runInShell: true,
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
            for (final r in entries)
              ListTile(
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
              ),
        ],
      ),
    );
  }
}
