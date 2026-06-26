import 'dart:io';

import 'package:file_selector/file_selector.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../app/domain/ui_settings.dart';
import '../../app/game_paths.dart';
import '../../core/mod_ffi.dart';
import '../../core/providers.dart';
import '../domain/texture_index_provider.dart';
import '../domain/texture_replacements_notifier.dart';

/// Browse the game's cooked textures, preview the original PNG, and stage PNG
/// replacements into [textureReplacementsProvider].
class TextureTab extends ConsumerStatefulWidget {
  const TextureTab({super.key});

  @override
  ConsumerState<TextureTab> createState() => _TextureTabState();
}

class _TextureTabState extends ConsumerState<TextureTab> {
  String _query = '';
  String? _selected;
  String? _previewPng;

  @override
  Widget build(BuildContext context) {
    final game = ref.watch(gameExePathProvider);
    if (game == null || game.isEmpty) {
      return const Center(
        child: Text('Set the game path in Settings to browse textures.'),
      );
    }
    final indexAsync = ref.watch(textureIndexProvider);
    final staged = ref.watch(textureReplacementsProvider);
    return indexAsync.when(
      loading: () => const Center(
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            CircularProgressIndicator(),
            SizedBox(height: 12),
            Text('Building texture index (first run, ~few minutes)…'),
          ],
        ),
      ),
      error: (e, _) => Center(child: SelectableText('Index error: $e')),
      data: (entries) {
        final matches =
            entries.keys
                .where(
                  (p) =>
                      _query.isEmpty ||
                      p.toLowerCase().contains(_query.toLowerCase()),
                )
                .take(500)
                .toList()
              ..sort();
        return Row(
          children: [
            Expanded(
              flex: 2,
              child: Column(
                children: [
                  Padding(
                    padding: const EdgeInsets.all(8),
                    child: TextField(
                      decoration: const InputDecoration(
                        prefixIcon: Icon(Icons.search),
                        hintText: 'Search textures',
                      ),
                      onChanged: (v) => setState(() => _query = v),
                    ),
                  ),
                  Expanded(
                    child: ListView.builder(
                      itemCount: matches.length,
                      itemBuilder: (c, i) {
                        final p = matches[i];
                        final isReplaced = staged.items.containsKey(p);
                        return ListTile(
                          dense: true,
                          selected: p == _selected,
                          title: Text(
                            p,
                            maxLines: 1,
                            overflow: TextOverflow.ellipsis,
                          ),
                          trailing: isReplaced
                              ? const Icon(Icons.check, size: 16)
                              : null,
                          onTap: () => setState(() {
                            _selected = p;
                            _previewPng = null;
                          }),
                        );
                      },
                    ),
                  ),
                  Text(
                    '${matches.length} shown / ${entries.length} total',
                    style: Theme.of(context).textTheme.bodySmall,
                  ),
                ],
              ),
            ),
            const VerticalDivider(width: 1),
            Expanded(flex: 3, child: _detail(entries, staged)),
          ],
        );
      },
    );
  }

  Widget _detail(Map<String, String> entries, TextureReplacementsState staged) {
    final sel = _selected;
    if (sel == null) return const Center(child: Text('Select a texture'));
    final gameDir = gameRootFromExe(ref.read(gameExePathProvider));
    final replaced = staged.items[sel];
    return Padding(
      padding: const EdgeInsets.all(12),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Text(sel, style: Theme.of(context).textTheme.titleSmall),
          const SizedBox(height: 8),
          Row(
            children: [
              OutlinedButton.icon(
                icon: const Icon(Icons.visibility),
                label: const Text('Preview'),
                onPressed: gameDir == null
                    ? null
                    : () => _preview(gameDir, sel, entries[sel]),
              ),
              const SizedBox(width: 8),
              FilledButton.icon(
                icon: const Icon(Icons.image),
                label: const Text('Replace…'),
                onPressed: () async {
                  final f = await openFile(
                    acceptedTypeGroups: [
                      const XTypeGroup(label: 'PNG', extensions: ['png']),
                    ],
                  );
                  if (f != null) {
                    ref
                        .read(textureReplacementsProvider.notifier)
                        .setReplacement(
                          TextureReplacement(asset: sel, imagePath: f.path),
                        );
                  }
                },
              ),
            ],
          ),
          const SizedBox(height: 12),
          if (_previewPng != null)
            Expanded(child: Image.file(File(_previewPng!), fit: BoxFit.contain))
          else
            const Expanded(
              child: Center(child: Text('Preview to see the current texture')),
            ),
          if (replaced != null)
            Padding(
              padding: const EdgeInsets.only(top: 8),
              child: Row(
                children: [
                  const Icon(Icons.swap_horiz, size: 16),
                  const SizedBox(width: 4),
                  Expanded(
                    child: Text(
                      '→ ${replaced.imagePath}',
                      maxLines: 1,
                      overflow: TextOverflow.ellipsis,
                    ),
                  ),
                  IconButton(
                    icon: const Icon(Icons.close, size: 16),
                    onPressed: () => ref
                        .read(textureReplacementsProvider.notifier)
                        .remove(sel),
                  ),
                ],
              ),
            ),
        ],
      ),
    );
  }

  Future<void> _preview(String gameDir, String asset, String? packageId) async {
    try {
      // textureExtract throws on a non-ok FFI result, so on return the PNG path
      // is present; just read it.
      final ffi = ModFfi(ref.read(coreServiceProvider));
      final r = await ffi.textureExtract(
        gameDir,
        asset: asset,
        packageId: packageId,
      );
      if (!mounted) return;
      setState(() => _previewPng = r['png_path'] as String?);
    } catch (e) {
      debugPrint('texture preview failed for $asset: $e');
      if (!mounted) return;
      await showDialog<void>(
        context: context,
        builder: (ctx) => AlertDialog(
          title: const Text('Preview failed'),
          content: SingleChildScrollView(
            child: SelectableText('$asset\n\n$e'),
          ),
          actions: [
            TextButton(
              onPressed: () => Clipboard.setData(
                ClipboardData(text: '$asset\n$e'),
              ),
              child: const Text('Copy'),
            ),
            TextButton(
              onPressed: () => Navigator.of(ctx).pop(),
              child: const Text('Close'),
            ),
          ],
        ),
      );
    }
  }
}
