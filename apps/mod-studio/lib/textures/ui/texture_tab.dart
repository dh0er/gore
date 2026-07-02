import 'dart:io';
import 'dart:ui' as ui;

import 'package:file_selector/file_selector.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../app/domain/ui_settings.dart';
import '../../app/game_paths.dart';
import '../../app/ui/path_tree.dart';
import '../../core/mod_ffi.dart';
import '../../core/providers.dart';
import '../domain/texture_index_provider.dart';
import '../domain/texture_replacements_notifier.dart';

/// Browse the game's cooked textures, preview the original PNG, and stage PNG
/// replacements into [textureReplacementsProvider].
class TextureTab extends ConsumerStatefulWidget {
  const TextureTab({super.key, this.onlyStaged = false});

  /// When true (the Changes tab), the browser — folder tree, flat search list,
  /// and count caption — covers only asset paths with a staged replacement
  /// ([textureReplacementsProvider] keys), updating live as replacements are
  /// (un)staged. The detail pane is unchanged. Default false: the full index.
  final bool onlyStaged;

  @override
  ConsumerState<TextureTab> createState() => _TextureTabState();
}

class _TextureTabState extends ConsumerState<TextureTab> {
  String _query = '';
  String? _selected;
  final TextEditingController _searchController = TextEditingController();
  // Assets with an extract currently in flight. Tracked as a set (not a single
  // "current" asset) so re-selecting an asset whose earlier extract is still
  // running doesn't start a second one racing on the same temp PNG.
  final Set<String> _inFlight = {};
  // Assets whose last extract attempt failed (an error dialog was shown). Lets
  // the Replace tooltip explain the failure instead of saying "preview first".
  final Set<String> _failed = {};
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

  // Decoded dimensions of staged replacement PNGs, keyed by image path. A null
  // VALUE means the file failed to decode (unreadable/corrupt), which flips the
  // preview back to the original with a hint; sticky for the session (retrying
  // on every build would loop). The staged PNGs are the user's own files (never
  // temp copies), so unlike [_previewCache] there is nothing to clean up in
  // dispose(), and the map stays tiny (one entry per staged PNG path browsed).
  final Map<String, (int, int)?> _stagedDims = {};
  final Set<String> _stagedDimsInFlight = {};

  // Identity-stable leaf-path list for [PathTreeBrowser], rebuilt only when the
  // entries map identity changes so the widget's identity-keyed tree cache
  // holds (matching the old once-per-index tree build).
  Map<String, String>? _treeEntries;
  List<String>? _treePaths;
  // onlyStaged mode: copy of the staged key set [_treePaths] was filtered by.
  // Compared by CONTENT, not state identity — the replacements notifier emits a
  // new state object on every change (including re-staging the same asset with
  // a different PNG, which leaves the key set untouched), and the filtered list
  // must keep its identity unless the key set really changed, or the tree
  // browser's identity-keyed cache would rebuild needlessly.
  Set<String>? _stagedKeys;

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
  void didUpdateWidget(TextureTab oldWidget) {
    super.didUpdateWidget(oldWidget);
    // The filter flipped: the cached paths list was built for the other mode.
    if (oldWidget.onlyStaged != widget.onlyStaged) {
      _treeEntries = null;
      _treePaths = null;
      _stagedKeys = null;
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
          _inFlight.clear();
          _failed.clear();
          _sourceEntries = entries;
          _sourceGame = game;
        }
        // The browsable path set: the full index, or (onlyStaged) just the
        // assets with a staged replacement. Identity-stable per (index
        // identity, staged key-set content) — see [_treePathsFor].
        final treePaths = _treePathsFor(entries, staged);
        if (widget.onlyStaged && treePaths.isEmpty) {
          // Nothing staged (the tree/list would render as a blank pane).
          return const Center(child: Text('No staged texture replacements.'));
        }
        // No cap: filter the browsable set then sort. The ListView below is lazy
        // (builder), so even the unfiltered ~13k entries render fine and every
        // matching asset stays selectable (a fixed .take() silently hid the rest).
        final matches =
            treePaths
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
                    // (paths matched anywhere, not just by folder). The tree
                    // stays mounted (just offstage) during a search so its
                    // expansion state and built tree survive the search being
                    // cleared — exactly as when that state lived on this tab.
                    child: Stack(
                      children: [
                        Offstage(
                          offstage: _query.isNotEmpty,
                          // Offstage skips paint/hit-test/semantics but NOT
                          // focus traversal — without this, Tab could reach
                          // the hidden tree's tiles during a search.
                          child: ExcludeFocus(
                            excluding: _query.isNotEmpty,
                            child: PathTreeBrowser(
                              paths: treePaths,
                              selectedPath: _selected,
                              onSelect: (p) => _select(gameDir, p, entries[p]),
                              leafIcon: Icons.image_outlined,
                              markedPaths: staged.items.keys.toSet(),
                            ),
                          ),
                        ),
                        if (_query.isNotEmpty)
                          _flatList(gameDir, matches, entries, staged),
                      ],
                    ),
                  ),
                  Text(
                    _query.isEmpty
                        ? '${treePaths.length} textures'
                        : '${matches.length} match / ${treePaths.length} total',
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

  /// The identity-stable leaf-path list for [entries]: all index paths, or —
  /// when [TextureTab.onlyStaged] — only those with a staged replacement.
  /// Recomputed only when the entries map identity changes (i.e. the index
  /// reloaded) or, in onlyStaged mode, when the staged key SET content changes
  /// (not on every replacements-state emission — re-staging the same asset
  /// keeps the set), so the tree browser's identity-keyed cache doesn't
  /// rebuild per frame.
  List<String> _treePathsFor(
    Map<String, String> entries,
    TextureReplacementsState staged,
  ) {
    if (!widget.onlyStaged) {
      if (!identical(_treeEntries, entries) || _treePaths == null) {
        _treeEntries = entries;
        _treePaths = entries.keys.toList(growable: false);
      }
      return _treePaths!;
    }
    final cachedKeys = _stagedKeys;
    final sameKeys =
        cachedKeys != null &&
        cachedKeys.length == staged.items.length &&
        staged.items.keys.every(cachedKeys.contains);
    if (!identical(_treeEntries, entries) || !sameKeys || _treePaths == null) {
      _treeEntries = entries;
      _stagedKeys = staged.items.keys.toSet();
      // Iterate the index (not the staged map) so tree order matches the full
      // browser and stale staged keys absent from the index are dropped.
      _treePaths = entries.keys
          .where(staged.items.containsKey)
          .toList(growable: false);
    }
    return _treePaths!;
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
    final loading = _inFlight.contains(sel);
    final pv = _previewCache[sel];
    final replaceBlocked = loading || pv == null || !pv.replaceable;
    final replaceReason = loading
        ? 'Loading texture…'
        : _failed.contains(sel)
        ? 'Preview failed for this texture — cannot replace'
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
                          if (f == null) return;
                          // Virtual textures only support same-dimension retiling;
                          // reject a mismatched PNG here instead of failing opaquely
                          // at build/cook. (pv is non-null: the button is disabled
                          // until a preview resolves.)
                          if (pv.isVirtual) {
                            final dims = await _imageDimensions(f.path);
                            if (!mounted) return;
                            if (dims != null &&
                                (dims.$1 != pv.width ||
                                    dims.$2 != pv.height)) {
                              await showDialog<void>(
                                context: context,
                                builder: (ctx) => AlertDialog(
                                  title: const Text('Size mismatch'),
                                  content: Text(
                                    'This is a virtual texture: the replacement '
                                    'must be exactly ${pv.width}×${pv.height}, '
                                    'but the PNG is ${dims.$1}×${dims.$2}.',
                                  ),
                                  actions: [
                                    TextButton(
                                      onPressed: () => Navigator.of(ctx).pop(),
                                      child: const Text('OK'),
                                    ),
                                  ],
                                ),
                              );
                              return;
                            }
                          } else {
                            // Regular texture: the encoder needs multiple-of-4
                            // dims, and power-of-two too when the source is
                            // mipmapped (encode_mips). Reject up front.
                            final dims = await _imageDimensions(f.path);
                            if (!mounted) return;
                            if (dims != null) {
                              final mult4 = dims.$1 % 4 == 0 && dims.$2 % 4 == 0;
                              final pot =
                                  _isPow2(dims.$1) && _isPow2(dims.$2);
                              if (!mult4 || (pv.mipmapped && !pot)) {
                                await showDialog<void>(
                                  context: context,
                                  builder: (ctx) => AlertDialog(
                                    title: const Text('Unsupported size'),
                                    content: Text(
                                      pv.mipmapped
                                          ? 'This texture has mipmaps: the '
                                                'replacement must be power-of-two '
                                                'and a multiple of 4 (e.g. '
                                                '512×512, 1024×2048). The PNG is '
                                                '${dims.$1}×${dims.$2}.'
                                          : 'The replacement dimensions must be a '
                                                'multiple of 4. The PNG is '
                                                '${dims.$1}×${dims.$2}.',
                                    ),
                                    actions: [
                                      TextButton(
                                        onPressed: () =>
                                            Navigator.of(ctx).pop(),
                                        child: const Text('OK'),
                                      ),
                                    ],
                                  ),
                                );
                                return;
                              }
                            }
                          }
                          ref
                              .read(textureReplacementsProvider.notifier)
                              .setReplacement(
                                TextureReplacement(
                                  asset: sel,
                                  imagePath: f.path,
                                ),
                              );
                        },
                ),
              ),
            ],
          ),
          const SizedBox(height: 12),
          Expanded(child: _previewArea(sel, replaced)),
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
  ///
  /// When a replacement is staged for [asset] ([replaced] non-null) the STAGED
  /// PNG is shown instead — this branch runs before the native preview-cache
  /// lookup, so the pane flips to the new image right after Replace… and back
  /// to the original on Remove (build() watches [textureReplacementsProvider],
  /// so every staging change re-evaluates it; the original's cache entry stays
  /// valid throughout). A 'Replacement' badge marks the staged view. A missing
  /// or unreadable staged PNG falls back to the original plus a small hint.
  Widget _previewArea(String asset, TextureReplacement? replaced) {
    final theme = Theme.of(context);
    String? stagedHint;
    if (replaced != null) {
      final path = replaced.imagePath;
      final knownBad =
          _stagedDims.containsKey(path) && _stagedDims[path] == null;
      if (!File(path).existsSync()) {
        stagedHint = 'Staged PNG missing — showing original';
      } else if (knownBad) {
        stagedHint = 'Staged PNG unreadable — showing original';
      } else {
        final dims = _stagedDims[path];
        if (dims == null) _loadStagedDims(path);
        return Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            Row(
              children: [
                Container(
                  padding: const EdgeInsets.symmetric(
                    horizontal: 6,
                    vertical: 2,
                  ),
                  decoration: BoxDecoration(
                    color: theme.colorScheme.secondaryContainer,
                    borderRadius: BorderRadius.circular(4),
                  ),
                  child: Text(
                    'Replacement',
                    style: theme.textTheme.labelSmall?.copyWith(
                      color: theme.colorScheme.onSecondaryContainer,
                    ),
                  ),
                ),
              ],
            ),
            const SizedBox(height: 6),
            Expanded(
              child: _imageBox(
                theme,
                File(path),
                // A PNG that fails to decode is detected by _loadStagedDims,
                // which flips this pane back to the original on the next build;
                // until then render the failure inline instead of throwing.
                errorBuilder: (_, e, st) =>
                    Center(child: SelectableText('Cannot display PNG: $e')),
              ),
            ),
            if (dims != null)
              Padding(
                padding: const EdgeInsets.only(top: 6),
                child: Text(
                  '${dims.$1} × ${dims.$2} · PNG',
                  style: theme.textTheme.bodySmall,
                ),
              ),
          ],
        );
      }
    }
    Widget original;
    final pv = _previewCache[asset];
    if (_inFlight.contains(asset)) {
      original = const Center(child: CircularProgressIndicator());
    } else if (pv == null) {
      original = const Center(
        child: Text('Preview to see the current texture'),
      );
    } else {
      original = Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          Expanded(child: _imageBox(theme, File(pv.pngPath))),
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
    if (stagedHint == null) return original;
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        Text(
          stagedHint,
          style: theme.textTheme.bodySmall?.copyWith(
            color: theme.colorScheme.error,
          ),
        ),
        const SizedBox(height: 6),
        Expanded(child: original),
      ],
    );
  }

  /// The checkerboard-backed, pan/zoomable image box shared by the original and
  /// staged-replacement previews.
  Widget _imageBox(
    ThemeData theme,
    File file, {
    ImageErrorWidgetBuilder? errorBuilder,
  }) {
    return DecoratedBox(
      decoration: BoxDecoration(
        border: Border.all(color: theme.dividerColor),
      ),
      child: CustomPaint(
        painter: _CheckerPainter(theme.brightness),
        child: InteractiveViewer(
          maxScale: 64,
          child: Center(
            child: Image.file(
              file,
              // Native size, downscaled only when larger than the pane —
              // small textures stay small (zoom in to inspect) instead of
              // being stretched to full width.
              fit: BoxFit.scaleDown,
              // Texture data is pixels: nearest-neighbour keeps them crisp
              // when zoomed rather than blurring.
              filterQuality: FilterQuality.none,
              gaplessPlayback: true,
              errorBuilder: errorBuilder,
            ),
          ),
        ),
      ),
    );
  }

  /// Decode the staged PNG's dimensions once per path (for the caption). A null
  /// result — missing/corrupt file — is recorded too: it flips [_previewArea]
  /// back to the original preview with a hint on the rebuild it triggers.
  void _loadStagedDims(String path) {
    if (_stagedDimsInFlight.contains(path)) return;
    _stagedDimsInFlight.add(path);
    _imageDimensions(path).then((dims) {
      _stagedDimsInFlight.remove(path);
      if (!mounted) return;
      setState(() => _stagedDims[path] = dims);
    });
  }

  /// Export the asset's decoded texture as a PNG to a user-chosen path. Always
  /// exports the ORIGINAL game texture, even when a replacement is staged and
  /// the preview shows the staged PNG — intentional: the staged PNG is the
  /// user's own file, while Export exists to get the original out as an editing
  /// base. Reuses the cached preview when present; otherwise extracts first
  /// (the same path the Preview button takes, including its error dialog on
  /// failure).
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

  static bool _isPow2(int n) => n > 0 && (n & (n - 1)) == 0;

  /// Decode just the pixel dimensions of the image at [path]; null on failure
  /// (then the dimension check is skipped and the build path surfaces any error).
  Future<(int, int)?> _imageDimensions(String path) async {
    try {
      final bytes = await File(path).readAsBytes();
      final codec = await ui.instantiateImageCodec(bytes);
      final frame = await codec.getNextFrame();
      final w = frame.image.width;
      final h = frame.image.height;
      frame.image.dispose();
      codec.dispose();
      return (w, h);
    } catch (_) {
      return null;
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
    // An extract for this asset is already in flight (it may not be the most
    // recent selection). Starting a second one would race on the FFI's
    // deterministic per-asset temp PNG and can corrupt it, so skip — the running
    // call will populate the cache.
    if (_inFlight.contains(asset)) return;
    setState(() {
      _inFlight.add(asset);
      _failed.remove(asset); // retrying clears a prior failure
    });
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
      if (!mounted) {
        // Tab was disposed mid-extract: the FFI already wrote the PNG but it will
        // never be cached (so dispose() can't evict it). Delete it here.
        try {
          File(r['png_path'] as String).deleteSync();
        } catch (_) {}
        return;
      }
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
          isVirtual: r['is_virtual'] as bool? ?? false,
          mipmapped: r['mipmapped'] as bool? ?? false,
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
      _failed.add(asset); // tooltip/gating reflects the failure (rebuilt in finally)
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
      // This asset's extract is done (success, stale, or failed) — clear its
      // in-flight mark so the spinner/guard for it lift. Other assets' extracts
      // are unaffected.
      if (mounted) setState(() => _inFlight.remove(asset));
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
    required this.replaceable,
    required this.isVirtual,
    required this.mipmapped,
  });

  final String pngPath;
  final int width;
  final int height;
  final String format;
  // Authoritative "can be replaced" flag from the FFI (encode-supported format
  // and a retileable shape for virtual textures). Gates the Replace button.
  final bool replaceable;
  // Virtual texture: retile only supports SAME-dimension replacement, so the UI
  // enforces a dimension match before staging a VT replacement.
  final bool isVirtual;
  // Regular texture shipped a full mip chain → replace runs encode_mips, which
  // requires power-of-two dimensions. Single-mip sources only need multiple-of-4.
  final bool mipmapped;
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
