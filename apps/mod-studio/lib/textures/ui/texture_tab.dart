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
  final TextEditingController _searchController = TextEditingController();
  // Asset currently being extracted — drives the preview-pane spinner.
  String? _loadingAsset;
  // Identity of the index + game path the preview cache was built against. When
  // either changes (game switched, index rebuilt after a game update) the cached
  // PNGs are stale and must be dropped — they were decoded from the old source.
  Object? _sourceEntries;
  String? _sourceGame;
  // Pixel formats gore-tex can preview/decode but NOT yet re-encode for replace.
  // Staging a replacement for these would fail later at cook time, so Replace is
  // disabled for them. Keep in sync with the encode support in gore-tex.
  static const _previewOnlyFormats = {
    'PF_B8G8R8A8',
    'PF_G8',
    'PF_BC4',
    'PF_BC6H',
    'PF_FloatRGBA',
  };
  // LRU-capped preview cache, keyed by asset. Re-selecting an already-previewed
  // asset shows instantly with no re-extract. Dart maps keep insertion order, so
  // "least recently used" = the first key; touching an entry re-inserts it at the
  // end. When the cache exceeds [_previewCacheCap] the oldest entry is evicted:
  // its temp PNG is deleted and dropped from the image cache, bounding disk/RAM
  // however many textures get browsed. Everything is also freed in dispose().
  static const _previewCacheCap = 24;
  final Map<String, _Preview> _previewCache = {};

  // Tree-browser state: the set of expanded folder ids, plus a compressed tree
  // built once per index (rebuilt only when the entries map identity changes).
  final Set<String> _expanded = {};
  Map<String, String>? _treeEntries;
  _DisplayNode? _treeRoot;

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
    _searchController.dispose();
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
        final gameDir = gameRootFromExe(game);
        // Drop stale previews if the source (game path or index) changed since
        // they were decoded — keying the cache by asset path alone is otherwise
        // blind to a game switch / index rebuild.
        if (!identical(_sourceEntries, entries) || _sourceGame != game) {
          for (final pv in _previewCache.values) {
            _evictPreview(pv);
          }
          _previewCache.clear();
          _loadingAsset = null;
          _sourceEntries = entries;
          _sourceGame = game;
        }
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
                      controller: _searchController,
                      decoration: InputDecoration(
                        prefixIcon: const Icon(Icons.search),
                        hintText: 'Search textures',
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
                  Expanded(
                    // Browse = lazy folder tree; an active search = flat hit list
                    // (paths matched anywhere, not just by folder).
                    child: _query.isEmpty
                        ? _treeBrowser(gameDir, entries, staged)
                        : _flatList(gameDir, matches, entries, staged),
                  ),
                  Text(
                    _query.isEmpty
                        ? '${entries.length} textures'
                        : '${matches.length} match / ${entries.length} total',
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

  // -- Browser: flat (search) + lazy tree (browse) ------------------------

  /// Select an asset and auto-load its preview (no separate Preview button).
  void _select(String? gameDir, String asset, String? packageId) {
    setState(() => _selected = asset);
    if (gameDir != null) _preview(gameDir, asset, packageId);
  }

  Widget _flatList(
    String? gameDir,
    List<String> matches,
    Map<String, String> entries,
    TextureReplacementsState staged,
  ) {
    return ListView.builder(
      itemCount: matches.length,
      itemBuilder: (c, i) {
        final p = matches[i];
        final isReplaced = staged.items.containsKey(p);
        return ListTile(
          dense: true,
          selected: p == _selected,
          title: Text(p, maxLines: 1, overflow: TextOverflow.ellipsis),
          trailing: isReplaced ? const Icon(Icons.check, size: 16) : null,
          onTap: () => _select(gameDir, p, entries[p]),
        );
      },
    );
  }

  Widget _treeBrowser(
    String? gameDir,
    Map<String, String> entries,
    TextureReplacementsState staged,
  ) {
    final root = _ensureTree(entries);
    // Flatten only the currently-expanded nodes (default collapsed → just the
    // top level), so this stays cheap regardless of the ~13k leaves.
    final visible = <_DisplayNode>[];
    void walk(List<_DisplayNode> nodes) {
      for (final n in nodes) {
        visible.add(n);
        if (!n.isLeaf && _expanded.contains(n.id)) walk(n.children!);
      }
    }

    walk(root.children!);
    final scheme = Theme.of(context).colorScheme;
    return ListView.builder(
      itemCount: visible.length,
      itemBuilder: (c, i) {
        final n = visible[i];
        final indent = n.depth * 14.0;
        if (n.isLeaf) {
          final isReplaced = staged.items.containsKey(n.assetPath);
          return Padding(
            padding: EdgeInsets.only(left: indent),
            child: ListTile(
              dense: true,
              selected: n.assetPath == _selected,
              leading: const Icon(Icons.image_outlined, size: 18),
              title: Text(n.label, maxLines: 1, overflow: TextOverflow.ellipsis),
              trailing: isReplaced ? const Icon(Icons.check, size: 16) : null,
              onTap: () => _select(gameDir, n.assetPath!, entries[n.assetPath]),
            ),
          );
        }
        final isOpen = _expanded.contains(n.id);
        return Padding(
          padding: EdgeInsets.only(left: indent),
          child: ListTile(
            dense: true,
            leading: Icon(
              isOpen ? Icons.expand_more : Icons.chevron_right,
              size: 18,
            ),
            title: Row(
              children: [
                Icon(
                  isOpen ? Icons.folder_open : Icons.folder,
                  size: 18,
                  color: scheme.primary,
                ),
                const SizedBox(width: 6),
                Expanded(
                  child: Text(
                    n.label,
                    maxLines: 1,
                    overflow: TextOverflow.ellipsis,
                  ),
                ),
                const SizedBox(width: 6),
                Text(
                  '${n.leafCount}',
                  style: Theme.of(context).textTheme.bodySmall?.copyWith(
                    color: scheme.onSurfaceVariant,
                  ),
                ),
              ],
            ),
            onTap: () => setState(() {
              if (isOpen) {
                _expanded.remove(n.id);
              } else {
                _expanded.add(n.id);
              }
            }),
          ),
        );
      },
    );
  }

  /// Build (and cache) the compressed display tree for [entries]. Rebuilt only
  /// when the entries map identity changes (i.e. the index reloaded).
  _DisplayNode _ensureTree(Map<String, String> entries) {
    if (identical(_treeEntries, entries) && _treeRoot != null) {
      return _treeRoot!;
    }
    final raw = _RawNode('');
    for (final p in entries.keys) {
      var node = raw;
      for (final seg in p.split('/')) {
        if (seg.isEmpty) continue;
        node = node.children.putIfAbsent(seg, () => _RawNode(seg));
      }
      node.assetPath = p;
    }
    final children = raw.children.values
        .map((c) => _toDisplay(c, 0, ''))
        .toList()
      ..sort(_nodeSort);
    final root = _DisplayNode(label: '', depth: -1, id: '', leafCount: 0)
      ..children = children;
    _treeEntries = entries;
    _treeRoot = root;
    return root;
  }

  /// Convert a raw segment node to a display node, compressing single-child
  /// folder chains ("A" whose only child is folder "B" → "A/B") so deep paths
  /// don't cost one click per level.
  _DisplayNode _toDisplay(_RawNode raw, int depth, String parentId) {
    var label = raw.label;
    var cur = raw;
    while (cur.assetPath == null && cur.children.length == 1) {
      final only = cur.children.values.first;
      if (only.assetPath != null) break; // single child is a texture — keep folder
      label = '$label/${only.label}';
      cur = only;
    }
    final id = parentId.isEmpty ? label : '$parentId/$label';
    if (cur.assetPath != null) {
      return _DisplayNode(
        label: label,
        depth: depth,
        id: cur.assetPath!,
        assetPath: cur.assetPath,
        leafCount: 1,
      );
    }
    final kids = cur.children.values
        .map((c) => _toDisplay(c, depth + 1, id))
        .toList()
      ..sort(_nodeSort);
    var count = 0;
    for (final k in kids) {
      count += k.leafCount;
    }
    return _DisplayNode(label: label, depth: depth, id: id, leafCount: count)
      ..children = kids;
  }

  /// Folders before leaves, then case-insensitive alpha.
  int _nodeSort(_DisplayNode a, _DisplayNode b) {
    if (a.isLeaf != b.isLeaf) return a.isLeaf ? 1 : -1;
    return a.label.toLowerCase().compareTo(b.label.toLowerCase());
  }

  Widget _detail(Map<String, String> entries, TextureReplacementsState staged) {
    final sel = _selected;
    if (sel == null) return const Center(child: Text('Select a texture'));
    final gameDir = gameRootFromExe(ref.read(gameExePathProvider));
    final replaced = staged.items[sel];
    // Replace capability is only known after the preview (auto-loaded on select)
    // resolves: the FFI reports whether the texture's format is re-encodable and,
    // for virtual textures, whether its shape is retileable. Block while loading,
    // until previewed, and for anything the core marks not replaceable — staging
    // those would only fail later at build/cook time.
    final loading = _loadingAsset == sel;
    final pv = _previewCache[sel];
    final replaceBlocked = loading || pv == null || !pv.replaceable;
    final replaceReason = loading
        ? 'Loading texture…'
        : pv == null
        ? 'Preview the texture first'
        : !pv.replaceable
        ? 'Replace not supported for this texture'
            '${pv.format.isEmpty ? '' : ' (${pv.format})'} yet'
        : 'Replace this texture with a PNG';
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
                icon: const Icon(Icons.download),
                label: const Text('Export PNG…'),
                onPressed: gameDir == null
                    ? null
                    : () => _export(gameDir, sel, entries[sel]),
              ),
              const SizedBox(width: 8),
              Tooltip(
                message: replaceReason,
                child: FilledButton.icon(
                  icon: const Icon(Icons.image),
                  label: const Text('Replace…'),
                  onPressed: replaceBlocked
                      ? null
                      : () async {
                          final f = await openFile(
                            acceptedTypeGroups: [
                              const XTypeGroup(
                                label: 'PNG',
                                extensions: ['png'],
                              ),
                            ],
                          );
                          if (f != null) {
                            ref
                                .read(textureReplacementsProvider.notifier)
                                .setReplacement(
                                  TextureReplacement(
                                    asset: sel,
                                    imagePath: f.path,
                                  ),
                                );
                          }
                        },
                ),
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
    if (_loadingAsset == asset) {
      return const Center(child: CircularProgressIndicator());
    }
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

  /// Export the asset's decoded texture as a PNG to a user-chosen path. Reuses
  /// the cached preview when present; otherwise extracts first (the same path the
  /// Preview button takes, including its error dialog on failure).
  Future<void> _export(String gameDir, String asset, String? packageId) async {
    if (!_previewCache.containsKey(asset)) {
      await _preview(gameDir, asset, packageId);
    }
    final pv = _previewCache[asset];
    if (pv == null || !mounted) return; // extract failed — dialog already shown.
    final leaf = asset.split('/').last;
    final loc = await getSaveLocation(
      suggestedName: '$leaf.png',
      acceptedTypeGroups: [
        const XTypeGroup(label: 'PNG', extensions: ['png']),
      ],
    );
    if (loc == null || !mounted) return;
    try {
      await File(pv.pngPath).copy(loc.path);
      if (!mounted) return;
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(content: Text('Saved $leaf.png')),
      );
    } catch (e) {
      if (!mounted) return;
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(content: Text('Export failed: $e')),
      );
    }
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
    // An extract for this asset is already in flight. Starting a second one would
    // race on the FFI's deterministic per-asset temp PNG and can corrupt it, so
    // skip — the running call will populate the cache.
    if (_loadingAsset == asset) return;
    setState(() => _loadingAsset = asset);
    // Capture the source this extract is for; if the game/index changes while it
    // runs, the result is stale and must be discarded (the cache was cleared for
    // the new source) rather than re-polluting it under the same asset key.
    final reqEntries = _sourceEntries;
    final reqGame = _sourceGame;
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
      if (!identical(_sourceEntries, reqEntries) || _sourceGame != reqGame) {
        // Source switched mid-extract — drop the now-stale PNG instead of caching.
        try {
          File(r['png_path'] as String).deleteSync();
        } catch (_) {}
        return;
      }
      setState(() {
        final fmt = r['format'] as String? ?? '';
        _previewCache[asset] = _Preview(
          pngPath: r['png_path'] as String,
          width: (r['width'] as num?)?.toInt() ?? 0,
          height: (r['height'] as num?)?.toInt() ?? 0,
          format: fmt,
          // Prefer the FFI's authoritative flag; fall back to the format denylist
          // when running against an older core that doesn't return it yet.
          replaceable:
              (r['replaceable'] as bool?) ?? !_previewOnlyFormats.contains(fmt),
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
    } finally {
      // Clear the spinner only if this call is still the active load (a newer
      // selection may have taken over).
      if (mounted && _loadingAsset == asset) {
        setState(() => _loadingAsset = null);
      }
    }
  }
}

/// Raw prefix-tree node: one per path segment, built directly from asset paths.
class _RawNode {
  _RawNode(this.label);
  final String label;
  final Map<String, _RawNode> children = {};
  String? assetPath; // non-null = a texture leaf
}

/// Display node: a compressed folder (possibly merged segments, with [children])
/// or a texture leaf ([assetPath] set). [id] is the stable folder path used for
/// expand/collapse tracking; [leafCount] is the texture count beneath it.
class _DisplayNode {
  _DisplayNode({
    required this.label,
    required this.depth,
    required this.id,
    required this.leafCount,
    this.assetPath,
  });
  final String label;
  final int depth;
  final String id;
  final int leafCount;
  final String? assetPath;
  List<_DisplayNode>? children;
  bool get isLeaf => assetPath != null;
}

/// A cached texture preview: the temp PNG path plus the source's native
/// dimensions and pixel format (for the caption).
class _Preview {
  const _Preview({
    required this.pngPath,
    required this.width,
    required this.height,
    required this.format,
    required this.replaceable,
  });

  final String pngPath;
  final int width;
  final int height;
  final String format;
  // Authoritative "can be replaced" flag from the FFI (encode-supported format
  // and a retileable shape for virtual textures). Gates the Replace button.
  final bool replaceable;
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
