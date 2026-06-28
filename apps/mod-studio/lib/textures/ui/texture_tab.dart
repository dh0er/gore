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
  // LRU-capped preview cache, keyed by asset. Re-selecting an already-previewed
  // asset shows instantly with no re-extract. Dart maps keep insertion order, so
  // "least recently used" = the first key; touching an entry re-inserts it at the
  // end. When the cache exceeds [_previewCacheCap] the oldest entry is evicted:
  // its temp PNG is deleted and dropped from the image cache, bounding disk/RAM
  // however many textures get browsed. Everything is also freed in dispose().
  static const _previewCacheCap = 24;
  final Map<String, _Preview> _previewCache = {};

  /// Delete a cached preview's temp PNG and drop it from the image cache.
  void _evictPreview(_Preview pv) {
    try {
      final file = File(pv.pngPath);
      PaintingBinding.instance.imageCache.evict(FileImage(file));
      if (file.existsSync()) file.deleteSync();
    } catch (_) {
      // Best-effort cleanup: a locked/already-gone temp file is harmless.
    }
  }

  @override
  void dispose() {
    for (final pv in _previewCache.values) {
      _evictPreview(pv);
    }
    super.dispose();
  }

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
        // No cap: filter the full index then sort. The ListView below is lazy
        // (builder), so even the unfiltered ~13k entries render fine and every
        // matching asset stays selectable (a fixed .take() silently hid the rest).
        final matches =
            entries.keys
                .where(
                  (p) =>
                      _query.isEmpty ||
                      p.toLowerCase().contains(_query.toLowerCase()),
                )
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
                          onTap: () => setState(() => _selected = p),
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
          Expanded(child: _previewArea(sel)),
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

  /// The preview pane for [asset]: a checkerboard backdrop (so transparent /
  /// fully-black textures are visible), the image at its native size scaled DOWN
  /// to fit (never blown up to fill the pane), pan/zoom via [InteractiveViewer],
  /// and a dims + pixel-format caption.
  Widget _previewArea(String asset) {
    final pv = _previewCache[asset];
    if (pv == null) {
      return const Center(child: Text('Preview to see the current texture'));
    }
    final theme = Theme.of(context);
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        Expanded(
          child: DecoratedBox(
            decoration: BoxDecoration(
              border: Border.all(color: theme.dividerColor),
            ),
            child: CustomPaint(
              painter: _CheckerPainter(theme.brightness),
              child: InteractiveViewer(
                maxScale: 64,
                child: Center(
                  child: Image.file(
                    File(pv.pngPath),
                    // Native size, downscaled only when larger than the pane —
                    // small textures stay small (zoom in to inspect) instead of
                    // being stretched to full width.
                    fit: BoxFit.scaleDown,
                    // Texture data is pixels: nearest-neighbour keeps them crisp
                    // when zoomed rather than blurring.
                    filterQuality: FilterQuality.none,
                    gaplessPlayback: true,
                  ),
                ),
              ),
            ),
          ),
        ),
        Padding(
          padding: const EdgeInsets.only(top: 6),
          child: Text(
            '${pv.width} × ${pv.height} · ${pv.format}',
            style: theme.textTheme.bodySmall,
          ),
        ),
      ],
    );
  }

  Future<void> _preview(String gameDir, String asset, String? packageId) async {
    // Already extracted this session — re-show instantly, skip the container
    // unpack + decode. Touch it (remove + re-insert) so it becomes the most
    // recently used entry and survives LRU eviction.
    final cached = _previewCache.remove(asset);
    if (cached != null) {
      setState(() => _previewCache[asset] = cached);
      return;
    }
    try {
      // textureExtract throws on a non-ok FFI result, so on return the PNG path
      // and dims are present.
      final ffi = ModFfi(ref.read(coreServiceProvider));
      final r = await ffi.textureExtract(
        gameDir,
        asset: asset,
        packageId: packageId,
      );
      if (!mounted) return;
      setState(() {
        _previewCache[asset] = _Preview(
          pngPath: r['png_path'] as String,
          width: (r['width'] as num?)?.toInt() ?? 0,
          height: (r['height'] as num?)?.toInt() ?? 0,
          format: r['format'] as String? ?? '',
        );
        // Evict least-recently-used entries once over the cap (oldest = first key).
        while (_previewCache.length > _previewCacheCap) {
          final oldestKey = _previewCache.keys.first;
          final evicted = _previewCache.remove(oldestKey);
          if (evicted != null) _evictPreview(evicted);
        }
      });
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

/// A cached texture preview: the temp PNG path plus the source's native
/// dimensions and pixel format (for the caption).
class _Preview {
  const _Preview({
    required this.pngPath,
    required this.width,
    required this.height,
    required this.format,
  });

  final String pngPath;
  final int width;
  final int height;
  final String format;
}

/// Paints a classic alpha checkerboard so transparent (and fully-black) textures
/// read against the backdrop instead of vanishing into the pane colour.
class _CheckerPainter extends CustomPainter {
  _CheckerPainter(this.brightness);

  final Brightness brightness;

  @override
  void paint(Canvas canvas, Size size) {
    const cell = 12.0;
    final isDark = brightness == Brightness.dark;
    final light = Paint()
      ..color = isDark ? const Color(0xFF3A3A3A) : const Color(0xFFE6E6E6);
    final dark = Paint()
      ..color = isDark ? const Color(0xFF2B2B2B) : const Color(0xFFC8C8C8);
    canvas.drawRect(Offset.zero & size, light);
    for (var y = 0.0; y < size.height; y += cell) {
      for (var x = 0.0; x < size.width; x += cell) {
        final odd =
            (((x / cell).floor() + (y / cell).floor()) & 1) == 1;
        if (odd) canvas.drawRect(Rect.fromLTWH(x, y, cell, cell), dark);
      }
    }
  }

  @override
  bool shouldRepaint(_CheckerPainter old) => old.brightness != brightness;
}
