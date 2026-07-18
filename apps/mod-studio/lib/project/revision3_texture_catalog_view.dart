import 'dart:async';
import 'dart:collection';

import 'package:flutter/material.dart';

import '../app/ui/path_tree.dart';
import 'revision3_texture_catalog.dart';

typedef Revision3TextureCountCopy = String Function(int count);
typedef Revision3TextureSearchCountCopy =
    String Function(int matches, int total);
typedef Revision3TextureVirtualLayerCountCopy = String Function(int count);

/// All author-facing copy used by [Revision3TextureCatalogView].
@immutable
final class Revision3TextureCatalogViewCopy {
  const Revision3TextureCatalogViewCopy({
    required this.setupTitle,
    required this.setupDescription,
    required this.setupActionLabel,
    required this.loadingLabel,
    required this.loadingDescription,
    required this.catalogCount,
    required this.searchCount,
    required this.emptyTitle,
    required this.emptyDescription,
    required this.errorTitle,
    required this.errorDescription,
    required this.retryLabel,
    required this.refreshTooltip,
    required this.searchLabel,
    required this.searchHint,
    required this.clearSearchTooltip,
    required this.selectPrompt,
    required this.previewLoadingLabel,
    required this.previewErrorTitle,
    required this.previewErrorDescription,
    required this.previewRetryLabel,
    required this.backToCatalogLabel,
    required this.inspectionOnlyNotice,
    required this.installedSourceBadge,
    required this.regularTextureBadge,
    required this.virtualTextureBadge,
    required this.virtualLayerCount,
    required this.mipmappedBadge,
    required this.singleMipBadge,
    required this.replaceableBadge,
    required this.notReplaceableBadge,
    required this.unknownReplaceabilityBadge,
    required this.unknownFormatLabel,
  });

  final String setupTitle;
  final String setupDescription;
  final String setupActionLabel;
  final String loadingLabel;
  final String loadingDescription;
  final Revision3TextureCountCopy catalogCount;
  final Revision3TextureSearchCountCopy searchCount;
  final String emptyTitle;
  final String emptyDescription;
  final String errorTitle;
  final String errorDescription;
  final String retryLabel;
  final String refreshTooltip;
  final String searchLabel;
  final String searchHint;
  final String clearSearchTooltip;
  final String selectPrompt;
  final String previewLoadingLabel;
  final String previewErrorTitle;
  final String previewErrorDescription;
  final String previewRetryLabel;
  final String backToCatalogLabel;
  final String inspectionOnlyNotice;
  final String installedSourceBadge;
  final String regularTextureBadge;
  final String virtualTextureBadge;
  final Revision3TextureVirtualLayerCountCopy virtualLayerCount;
  final String mipmappedBadge;
  final String singleMipBadge;
  final String replaceableBadge;
  final String notReplaceableBadge;
  final String unknownReplaceabilityBadge;
  final String unknownFormatLabel;
}

/// Read-only installed-game texture catalog for a Managed workspace.
///
/// The view owns no project state and performs no file or native I/O. Loading,
/// preview extraction are injected. Changing
/// [gameRoot] or [sourceSelectionIdentity] invalidates displayed work;
/// completions from an older source are ignored.
///
/// [sourceSelectionIdentity] is only an owner-supplied configuration lifetime,
/// never generation evidence. The catalog loader atomically supplies the exact
/// native build fingerprint, and every preview must verify and return that same
/// fingerprint. Callback identity is intentionally ignored; equivalent parent
/// rebuilds remain load-free.
final class Revision3TextureCatalogView extends StatefulWidget {
  const Revision3TextureCatalogView({
    required this.gameRoot,
    required this.sourceSelectionIdentity,
    required this.loadCatalog,
    required this.loadPreview,
    required this.copy,
    this.openSettings,
    super.key,
  });

  final String? gameRoot;
  final Object? sourceSelectionIdentity;
  final Revision3TextureCatalogLoader loadCatalog;
  final Revision3TexturePreviewLoader loadPreview;
  final Revision3TextureCatalogViewCopy copy;
  final VoidCallback? openSettings;

  @override
  State<Revision3TextureCatalogView> createState() =>
      _Revision3TextureCatalogViewState();
}

final class _Revision3TextureCatalogViewState
    extends State<Revision3TextureCatalogView> {
  static const _maximumPreviewCacheBytes = 64 * 1024 * 1024;
  static const _maximumPreviewCacheEntries = 24;
  static const _maximumConcurrentPreviewLoads = 2;

  late final TextEditingController _search;
  Revision3TextureCatalogSnapshot? _catalog;
  Object? _catalogError;
  bool _catalogLoading = false;
  int _catalogEpoch = 0;
  bool _catalogRequestRunning = false;
  int? _queuedCatalogEpoch;

  Revision3TextureCatalogEntry? _selected;
  Revision3TexturePreview? _preview;
  Object? _previewError;
  bool _previewLoading = false;
  int _previewEpoch = 0;
  _TexturePreviewKey? _displayedPreviewKey;
  final Map<_TexturePreviewKey, Future<Revision3TexturePreviewResult>>
  _previewRequests = {};
  final Map<_TexturePreviewKey, Revision3TexturePreviewResult> _previewCache =
      {};
  final Queue<Completer<void>> _previewSlotWaiters = Queue<Completer<void>>();
  int _previewCacheBytes = 0;
  int _activePreviewLoads = 0;
  bool _disposed = false;
  String _query = '';

  bool get _sourceAvailable =>
      widget.gameRoot?.trim().isNotEmpty == true &&
      widget.sourceSelectionIdentity != null;

  @override
  void initState() {
    super.initState();
    _search = TextEditingController()..addListener(_onSearchChanged);
    _scheduleCatalogLoad(notify: false);
  }

  @override
  void didUpdateWidget(covariant Revision3TextureCatalogView oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.gameRoot == widget.gameRoot &&
        oldWidget.sourceSelectionIdentity == widget.sourceSelectionIdentity) {
      return;
    }
    _scheduleCatalogLoad(notify: false);
  }

  @override
  void dispose() {
    _disposed = true;
    _catalogEpoch++;
    _queuedCatalogEpoch = null;
    _previewEpoch++;
    _previewRequests.clear();
    _previewCache.clear();
    _previewCacheBytes = 0;
    while (_previewSlotWaiters.isNotEmpty) {
      _previewSlotWaiters.removeFirst().completeError(
        const _TexturePreviewQueueDisposed(),
      );
    }
    _search
      ..removeListener(_onSearchChanged)
      ..dispose();
    super.dispose();
  }

  void _onSearchChanged() {
    final query = _search.text.trim().toLowerCase();
    if (query == _query) return;
    setState(() => _query = query);
  }

  void _scheduleCatalogLoad({bool notify = true}) {
    final epoch = ++_catalogEpoch;
    _queuedCatalogEpoch = null;
    _previewEpoch++;
    void reset() {
      _previewRequests.clear();
      _catalog = null;
      _catalogError = null;
      _catalogLoading = _sourceAvailable;
      _selected = null;
      _preview = null;
      _previewError = null;
      _previewLoading = false;
      _displayedPreviewKey = null;
    }

    if (notify) {
      setState(reset);
    } else {
      reset();
    }
    WidgetsBinding.instance.addPostFrameCallback((_) {
      if (!mounted || epoch != _catalogEpoch || !_sourceAvailable) return;
      _queueCatalogLoad(epoch);
    });
  }

  void _queueCatalogLoad(int epoch) {
    _queuedCatalogEpoch = epoch;
    if (_catalogRequestRunning) return;
    unawaited(_drainCatalogLoads());
  }

  Future<void> _drainCatalogLoads() async {
    _catalogRequestRunning = true;
    try {
      while (mounted) {
        final epoch = _queuedCatalogEpoch;
        if (epoch == null) return;
        _queuedCatalogEpoch = null;
        await _loadCatalog(epoch);
      }
    } finally {
      _catalogRequestRunning = false;
      if (mounted && _queuedCatalogEpoch != null) {
        unawaited(_drainCatalogLoads());
      }
    }
  }

  Future<void> _loadCatalog(int epoch) async {
    final root = widget.gameRoot!.trim();
    try {
      final catalog = await Future<Revision3TextureCatalogSnapshot>.sync(
        () => widget.loadCatalog(gameRoot: root),
      );
      if (!mounted || epoch != _catalogEpoch) return;
      setState(() {
        _catalog = catalog;
        _catalogError = null;
        _catalogLoading = false;
      });
    } catch (error) {
      if (!mounted || epoch != _catalogEpoch) return;
      setState(() {
        _catalog = null;
        _catalogError = error;
        _catalogLoading = false;
      });
    }
  }

  void _refreshCatalog() {
    _scheduleCatalogLoad();
  }

  void _select(Revision3TextureCatalogEntry texture) {
    final catalog = _catalog;
    if (catalog == null) return;
    final key = _TexturePreviewKey(
      sourceFingerprint: catalog.sourceFingerprint,
      assetPath: texture.assetPath,
    );
    if (_selected?.assetPath == texture.assetPath &&
        (_previewLoading || _displayedPreviewKey == key)) {
      return;
    }
    setState(() {
      _selected = texture;
      _preview = null;
      _previewError = null;
      _previewLoading = true;
      _displayedPreviewKey = null;
    });
    _loadSelectedPreview(
      texture: texture,
      sourceFingerprint: catalog.sourceFingerprint,
    );
  }

  Future<void> _loadSelectedPreview({
    required Revision3TextureCatalogEntry texture,
    required Revision3TextureSourceFingerprint sourceFingerprint,
  }) async {
    final selectionEpoch = ++_previewEpoch;
    final root = widget.gameRoot?.trim();
    if (root == null ||
        root.isEmpty ||
        widget.sourceSelectionIdentity == null) {
      return;
    }
    final key = _TexturePreviewKey(
      sourceFingerprint: sourceFingerprint,
      assetPath: texture.assetPath,
    );
    final cached = _takeCachedPreview(key);
    if (cached != null) {
      if (_selectionStillCurrent(selectionEpoch: selectionEpoch, key: key)) {
        setState(() {
          _preview = cached.preview;
          _previewError = null;
          _previewLoading = false;
          _displayedPreviewKey = key;
        });
      }
      return;
    }
    try {
      final result = await _previewRequest(
        key: key,
        gameRoot: root,
        texture: texture,
        sourceEpoch: _catalogEpoch,
      );
      if (!_selectionStillCurrent(selectionEpoch: selectionEpoch, key: key)) {
        return;
      }
      setState(() {
        _preview = result.preview;
        _previewError = null;
        _previewLoading = false;
        _displayedPreviewKey = key;
      });
    } catch (error) {
      if (!_selectionStillCurrent(selectionEpoch: selectionEpoch, key: key)) {
        return;
      }
      setState(() {
        _preview = null;
        _previewError = error;
        _previewLoading = false;
        _displayedPreviewKey = null;
      });
    }
  }

  Future<Revision3TexturePreviewResult> _previewRequest({
    required _TexturePreviewKey key,
    required String gameRoot,
    required Revision3TextureCatalogEntry texture,
    required int sourceEpoch,
  }) {
    final existing = _previewRequests[key];
    if (existing != null) return existing;
    late final Future<Revision3TexturePreviewResult> tracked;
    tracked =
        _runQueuedPreview(
              key: key,
              gameRoot: gameRoot,
              texture: texture,
              sourceEpoch: sourceEpoch,
            )
            .then((result) {
              if (_previewSourceStillCurrent(
                key: key,
                sourceEpoch: sourceEpoch,
              )) {
                _cachePreview(key, result);
              }
              return result;
            })
            .whenComplete(() {
              if (identical(_previewRequests[key], tracked)) {
                _previewRequests.remove(key);
              }
            });
    _previewRequests[key] = tracked;
    return tracked;
  }

  Future<Revision3TexturePreviewResult> _runQueuedPreview({
    required _TexturePreviewKey key,
    required String gameRoot,
    required Revision3TextureCatalogEntry texture,
    required int sourceEpoch,
  }) async {
    await _acquirePreviewSlot();
    try {
      if (!_previewRequestStillCurrent(key: key, sourceEpoch: sourceEpoch)) {
        throw const _TexturePreviewRequestDiscarded();
      }
      final result = await Future<Revision3TexturePreviewResult>.sync(
        () => widget.loadPreview(
          gameRoot: gameRoot,
          expectedSourceFingerprint: key.sourceFingerprint,
          texture: texture,
        ),
      );
      if (result.sourceFingerprint != key.sourceFingerprint) {
        throw StateError('texture preview source fingerprint mismatch');
      }
      return result;
    } finally {
      _releasePreviewSlot();
    }
  }

  Future<void> _acquirePreviewSlot() {
    if (_disposed) {
      return Future<void>.error(const _TexturePreviewQueueDisposed());
    }
    if (_activePreviewLoads < _maximumConcurrentPreviewLoads) {
      _activePreviewLoads++;
      return Future<void>.value();
    }
    final waiter = Completer<void>();
    _previewSlotWaiters.addLast(waiter);
    return waiter.future;
  }

  void _releasePreviewSlot() {
    assert(_activePreviewLoads > 0);
    if (_activePreviewLoads == 0) return;
    if (_disposed) {
      _activePreviewLoads--;
      return;
    }
    while (_previewSlotWaiters.isNotEmpty) {
      final waiter = _previewSlotWaiters.removeFirst();
      if (!waiter.isCompleted) {
        waiter.complete();
        return;
      }
    }
    _activePreviewLoads--;
  }

  bool _previewRequestStillCurrent({
    required _TexturePreviewKey key,
    required int sourceEpoch,
  }) =>
      _previewSourceStillCurrent(key: key, sourceEpoch: sourceEpoch) &&
      _selected?.assetPath == key.assetPath;

  bool _previewSourceStillCurrent({
    required _TexturePreviewKey key,
    required int sourceEpoch,
  }) =>
      mounted &&
      sourceEpoch == _catalogEpoch &&
      _catalog?.sourceFingerprint == key.sourceFingerprint;

  bool _selectionStillCurrent({
    required int selectionEpoch,
    required _TexturePreviewKey key,
  }) =>
      mounted &&
      selectionEpoch == _previewEpoch &&
      _selected?.assetPath == key.assetPath &&
      _catalog?.sourceFingerprint == key.sourceFingerprint;

  Revision3TexturePreviewResult? _takeCachedPreview(_TexturePreviewKey key) {
    final cached = _previewCache.remove(key);
    if (cached != null) _previewCache[key] = cached;
    return cached;
  }

  void _cachePreview(
    _TexturePreviewKey key,
    Revision3TexturePreviewResult result,
  ) {
    final byteLength = result.preview.pngBytes.length;
    final previous = _previewCache.remove(key);
    if (previous != null) {
      _previewCacheBytes -= previous.preview.pngBytes.length;
    }
    if (byteLength > _maximumPreviewCacheBytes) return;
    _previewCache[key] = result;
    _previewCacheBytes += byteLength;
    while ((_previewCacheBytes > _maximumPreviewCacheBytes ||
            _previewCache.length > _maximumPreviewCacheEntries) &&
        _previewCache.isNotEmpty) {
      final oldestKey = _previewCache.keys.first;
      final oldest = _previewCache.remove(oldestKey)!;
      _previewCacheBytes -= oldest.preview.pngBytes.length;
    }
  }

  void _retryPreview() {
    if (_previewError is Revision3TextureSourceChangedException) {
      _refreshCatalog();
      return;
    }
    final selected = _selected;
    final catalog = _catalog;
    if (selected == null || catalog == null) return;
    setState(() {
      _preview = null;
      _previewError = null;
      _previewLoading = true;
      _displayedPreviewKey = null;
    });
    _loadSelectedPreview(
      texture: selected,
      sourceFingerprint: catalog.sourceFingerprint,
    );
  }

  @override
  Widget build(BuildContext context) => Semantics(
    key: const Key('revision3-texture-catalog'),
    container: true,
    explicitChildNodes: true,
    child: !_sourceAvailable
        ? _SetupState(copy: widget.copy, openSettings: widget.openSettings)
        : _catalogLoading
        ? _LoadingState(
            label: widget.copy.loadingLabel,
            description: widget.copy.loadingDescription,
          )
        : _catalogError != null
        ? _ErrorState(copy: widget.copy, retry: _refreshCatalog)
        : _catalog == null
        ? const SizedBox.shrink()
        : _buildCatalog(context, _catalog!),
  );

  Widget _buildCatalog(
    BuildContext context,
    Revision3TextureCatalogSnapshot catalog,
  ) {
    if (catalog.textures.isEmpty) {
      return _CenteredState(
        stateKey: const Key('revision3-texture-catalog-empty'),
        icon: Icons.image_not_supported_outlined,
        title: widget.copy.emptyTitle,
        description: widget.copy.emptyDescription,
        action: IconButton(
          key: const Key('revision3-texture-catalog-refresh-empty'),
          tooltip: widget.copy.refreshTooltip,
          onPressed: _refreshCatalog,
          icon: const Icon(Icons.refresh),
        ),
      );
    }
    return LayoutBuilder(
      builder: (context, constraints) {
        final compact = constraints.maxWidth < 720;
        final browser = _BrowserPane(
          catalog: catalog,
          search: _search,
          query: _query,
          selected: _selected,
          copy: widget.copy,
          refresh: _refreshCatalog,
          select: _select,
        );
        if (compact) {
          final showingDetail = _selected != null;
          return Stack(
            children: [
              Positioned.fill(
                child: Offstage(
                  offstage: showingDetail,
                  child: ExcludeFocus(excluding: showingDetail, child: browser),
                ),
              ),
              if (showingDetail)
                Positioned.fill(
                  child: _DetailPane(
                    key: const Key('revision3-texture-catalog-compact-detail'),
                    texture: _selected!,
                    preview: _preview,
                    loading: _previewLoading,
                    error: _previewError,
                    copy: widget.copy,
                    retry: _retryPreview,
                    back: () => setState(() => _selected = null),
                  ),
                ),
            ],
          );
        }
        return Row(
          children: [
            Expanded(flex: 2, child: browser),
            const VerticalDivider(width: 1),
            Expanded(
              flex: 3,
              child: _selected == null
                  ? _SelectPrompt(message: widget.copy.selectPrompt)
                  : _DetailPane(
                      texture: _selected!,
                      preview: _preview,
                      loading: _previewLoading,
                      error: _previewError,
                      copy: widget.copy,
                      retry: _retryPreview,
                    ),
            ),
          ],
        );
      },
    );
  }
}

final class _BrowserPane extends StatelessWidget {
  const _BrowserPane({
    required this.catalog,
    required this.search,
    required this.query,
    required this.selected,
    required this.copy,
    required this.refresh,
    required this.select,
  });

  final Revision3TextureCatalogSnapshot catalog;
  final TextEditingController search;
  final String query;
  final Revision3TextureCatalogEntry? selected;
  final Revision3TextureCatalogViewCopy copy;
  final VoidCallback refresh;
  final ValueChanged<Revision3TextureCatalogEntry> select;

  @override
  Widget build(BuildContext context) {
    final matches = query.isEmpty
        ? const <Revision3TextureCatalogEntry>[]
        : catalog.textures
              .where(
                (texture) =>
                    texture.assetPath.toLowerCase().contains(query) ||
                    texture.displayName.toLowerCase().contains(query),
              )
              .toList(growable: false);
    return Column(
      key: const Key('revision3-texture-catalog-browser'),
      children: [
        Padding(
          padding: const EdgeInsets.fromLTRB(8, 8, 8, 4),
          child: Row(
            children: [
              Expanded(
                child: TextField(
                  key: const Key('revision3-texture-catalog-search'),
                  controller: search,
                  textInputAction: TextInputAction.search,
                  decoration: InputDecoration(
                    isDense: true,
                    labelText: copy.searchLabel,
                    hintText: copy.searchHint,
                    prefixIcon: const Icon(Icons.search),
                    suffixIcon: query.isEmpty
                        ? null
                        : IconButton(
                            key: const Key(
                              'revision3-texture-catalog-clear-search',
                            ),
                            tooltip: copy.clearSearchTooltip,
                            onPressed: search.clear,
                            icon: const Icon(Icons.clear),
                          ),
                    border: const OutlineInputBorder(),
                  ),
                ),
              ),
              IconButton(
                key: const Key('revision3-texture-catalog-refresh'),
                tooltip: copy.refreshTooltip,
                onPressed: refresh,
                icon: const Icon(Icons.refresh),
              ),
            ],
          ),
        ),
        Expanded(
          child: Stack(
            children: [
              Positioned.fill(
                child: Offstage(
                  offstage: query.isNotEmpty,
                  child: ExcludeFocus(
                    excluding: query.isNotEmpty,
                    child: PathTreeBrowser(
                      paths: catalog.assetPaths,
                      selectedPath: selected?.assetPath,
                      onSelect: (path) => select(catalog.byAssetPath[path]!),
                      leafIcon: Icons.image_outlined,
                    ),
                  ),
                ),
              ),
              if (query.isNotEmpty)
                Positioned.fill(
                  child: ListView.builder(
                    key: const Key('revision3-texture-catalog-search-results'),
                    itemCount: matches.length,
                    itemBuilder: (context, index) {
                      final texture = matches[index];
                      return ListTile(
                        key: ValueKey(
                          'revision3-texture-catalog-result-${texture.assetPath}',
                        ),
                        dense: true,
                        selected: texture.assetPath == selected?.assetPath,
                        leading: const Icon(Icons.image_outlined, size: 18),
                        title: Text(
                          texture.displayName,
                          maxLines: 1,
                          overflow: TextOverflow.ellipsis,
                        ),
                        subtitle: Text(
                          texture.assetPath,
                          maxLines: 1,
                          overflow: TextOverflow.ellipsis,
                        ),
                        onTap: () => select(texture),
                      );
                    },
                  ),
                ),
            ],
          ),
        ),
        Padding(
          padding: const EdgeInsets.fromLTRB(8, 2, 8, 6),
          child: Text(
            query.isEmpty
                ? copy.catalogCount(catalog.textures.length)
                : copy.searchCount(matches.length, catalog.textures.length),
            key: const Key('revision3-texture-catalog-count'),
            style: Theme.of(context).textTheme.bodySmall,
            maxLines: 1,
            overflow: TextOverflow.ellipsis,
          ),
        ),
      ],
    );
  }
}

final class _DetailPane extends StatelessWidget {
  const _DetailPane({
    required this.texture,
    required this.preview,
    required this.loading,
    required this.error,
    required this.copy,
    required this.retry,
    this.back,
    super.key,
  });

  final Revision3TextureCatalogEntry texture;
  final Revision3TexturePreview? preview;
  final bool loading;
  final Object? error;
  final Revision3TextureCatalogViewCopy copy;
  final VoidCallback retry;
  final VoidCallback? back;

  @override
  Widget build(BuildContext context) => SingleChildScrollView(
    key: const Key('revision3-texture-catalog-detail'),
    padding: const EdgeInsets.all(12),
    child: Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        if (back != null)
          Align(
            alignment: Alignment.centerLeft,
            child: TextButton.icon(
              key: const Key('revision3-texture-catalog-back'),
              onPressed: back,
              icon: const Icon(Icons.arrow_back),
              label: Text(copy.backToCatalogLabel),
            ),
          ),
        SelectableText(
          texture.displayName,
          style: Theme.of(context).textTheme.titleMedium,
        ),
        const SizedBox(height: 3),
        SelectableText(
          texture.assetPath,
          key: const Key('revision3-texture-catalog-detail-path'),
          style: Theme.of(context).textTheme.bodySmall,
        ),
        const SizedBox(height: 8),
        Text(copy.inspectionOnlyNotice),
        const SizedBox(height: 10),
        if (loading)
          _InlinePreviewState(
            key: const Key('revision3-texture-catalog-preview-loading'),
            child: Column(
              mainAxisSize: MainAxisSize.min,
              children: [
                const CircularProgressIndicator(),
                const SizedBox(height: 10),
                Text(copy.previewLoadingLabel, textAlign: TextAlign.center),
              ],
            ),
          )
        else if (error != null)
          _InlinePreviewState(
            key: const Key('revision3-texture-catalog-preview-error'),
            child: Column(
              mainAxisSize: MainAxisSize.min,
              children: [
                const Icon(Icons.broken_image_outlined, size: 40),
                const SizedBox(height: 8),
                Text(
                  copy.previewErrorTitle,
                  style: Theme.of(context).textTheme.titleSmall,
                  textAlign: TextAlign.center,
                ),
                const SizedBox(height: 4),
                Text(copy.previewErrorDescription, textAlign: TextAlign.center),
                const SizedBox(height: 10),
                FilledButton.icon(
                  key: const Key('revision3-texture-catalog-preview-retry'),
                  onPressed: retry,
                  icon: const Icon(Icons.refresh),
                  label: Text(copy.previewRetryLabel),
                ),
              ],
            ),
          )
        else if (preview != null) ...[
          AspectRatio(
            aspectRatio: 16 / 9,
            child: DecoratedBox(
              decoration: BoxDecoration(
                border: Border.all(color: Theme.of(context).dividerColor),
              ),
              child: CustomPaint(
                painter: _CheckerPainter(Theme.of(context).brightness),
                child: InteractiveViewer(
                  maxScale: 64,
                  child: Center(
                    child: Image.memory(
                      preview!.pngBytes,
                      key: const Key('revision3-texture-catalog-preview-image'),
                      fit: BoxFit.scaleDown,
                      filterQuality: FilterQuality.none,
                      gaplessPlayback: true,
                      errorBuilder: (_, _, _) => const Center(
                        child: Icon(Icons.broken_image_outlined, size: 40),
                      ),
                    ),
                  ),
                ),
              ),
            ),
          ),
          const SizedBox(height: 8),
          Wrap(
            key: const Key('revision3-texture-catalog-preview-facts'),
            spacing: 6,
            runSpacing: 6,
            children: [
              _FactBadge('${preview!.width} × ${preview!.height}'),
              _FactBadge(
                preview!.pixelFormat.trim().isEmpty
                    ? copy.unknownFormatLabel
                    : preview!.pixelFormat,
              ),
              _FactBadge(
                preview!.isVirtual
                    ? copy.virtualTextureBadge
                    : copy.regularTextureBadge,
              ),
              if (preview!.isVirtual)
                _FactBadge(copy.virtualLayerCount(preview!.virtualLayers)),
              _FactBadge(
                preview!.mipmapped ? copy.mipmappedBadge : copy.singleMipBadge,
              ),
              _FactBadge(switch (preview!.replaceability) {
                Revision3TextureReplaceability.supported =>
                  copy.replaceableBadge,
                Revision3TextureReplaceability.unsupported =>
                  copy.notReplaceableBadge,
                Revision3TextureReplaceability.unknown =>
                  copy.unknownReplaceabilityBadge,
              }),
              _FactBadge(copy.installedSourceBadge),
            ],
          ),
        ],
      ],
    ),
  );
}

final class _FactBadge extends StatelessWidget {
  const _FactBadge(this.label);
  final String label;

  @override
  Widget build(BuildContext context) =>
      Chip(visualDensity: VisualDensity.compact, label: Text(label));
}

final class _InlinePreviewState extends StatelessWidget {
  const _InlinePreviewState({required this.child, super.key});
  final Widget child;

  @override
  Widget build(BuildContext context) => ConstrainedBox(
    constraints: const BoxConstraints(minHeight: 180),
    child: Center(child: child),
  );
}

final class _SetupState extends StatelessWidget {
  const _SetupState({required this.copy, required this.openSettings});
  final Revision3TextureCatalogViewCopy copy;
  final VoidCallback? openSettings;

  @override
  Widget build(BuildContext context) => _CenteredState(
    stateKey: const Key('revision3-texture-catalog-setup'),
    icon: Icons.settings_outlined,
    title: copy.setupTitle,
    description: copy.setupDescription,
    action: FilledButton.icon(
      key: const Key('revision3-texture-catalog-open-settings'),
      onPressed: openSettings,
      icon: const Icon(Icons.settings_outlined),
      label: Text(copy.setupActionLabel),
    ),
  );
}

final class _LoadingState extends StatelessWidget {
  const _LoadingState({required this.label, required this.description});
  final String label;
  final String description;

  @override
  Widget build(BuildContext context) => Center(
    key: const Key('revision3-texture-catalog-loading'),
    child: SingleChildScrollView(
      padding: const EdgeInsets.all(16),
      child: Column(
        mainAxisSize: MainAxisSize.min,
        children: [
          const CircularProgressIndicator(),
          const SizedBox(height: 12),
          Text(label, textAlign: TextAlign.center),
          const SizedBox(height: 6),
          Text(
            description,
            textAlign: TextAlign.center,
            style: Theme.of(context).textTheme.bodySmall,
          ),
        ],
      ),
    ),
  );
}

final class _ErrorState extends StatelessWidget {
  const _ErrorState({required this.copy, required this.retry});
  final Revision3TextureCatalogViewCopy copy;
  final VoidCallback retry;

  @override
  Widget build(BuildContext context) => _CenteredState(
    stateKey: const Key('revision3-texture-catalog-error'),
    icon: Icons.error_outline,
    title: copy.errorTitle,
    description: copy.errorDescription,
    action: FilledButton.icon(
      key: const Key('revision3-texture-catalog-retry'),
      onPressed: retry,
      icon: const Icon(Icons.refresh),
      label: Text(copy.retryLabel),
    ),
  );
}

final class _SelectPrompt extends StatelessWidget {
  const _SelectPrompt({required this.message});
  final String message;

  @override
  Widget build(BuildContext context) => Center(
    key: const Key('revision3-texture-catalog-select-prompt'),
    child: Padding(
      padding: const EdgeInsets.all(16),
      child: Column(
        mainAxisSize: MainAxisSize.min,
        children: [
          const Icon(Icons.image_search_outlined, size: 42),
          const SizedBox(height: 10),
          Text(message, textAlign: TextAlign.center),
        ],
      ),
    ),
  );
}

final class _CenteredState extends StatelessWidget {
  const _CenteredState({
    required this.stateKey,
    required this.icon,
    required this.title,
    required this.description,
    required this.action,
  });

  final Key stateKey;
  final IconData icon;
  final String title;
  final String description;
  final Widget action;

  @override
  Widget build(BuildContext context) => LayoutBuilder(
    builder: (context, constraints) => SingleChildScrollView(
      key: stateKey,
      padding: const EdgeInsets.all(16),
      child: ConstrainedBox(
        constraints: BoxConstraints(
          minHeight: constraints.maxHeight.isFinite
              ? (constraints.maxHeight - 32).clamp(0, double.infinity)
              : 0,
        ),
        child: Center(
          child: ConstrainedBox(
            constraints: const BoxConstraints(maxWidth: 520),
            child: Column(
              mainAxisSize: MainAxisSize.min,
              children: [
                Icon(icon, size: 42),
                const SizedBox(height: 10),
                Text(
                  title,
                  textAlign: TextAlign.center,
                  style: Theme.of(context).textTheme.titleMedium,
                ),
                const SizedBox(height: 6),
                Text(description, textAlign: TextAlign.center),
                const SizedBox(height: 12),
                action,
              ],
            ),
          ),
        ),
      ),
    ),
  );
}

final class _CheckerPainter extends CustomPainter {
  const _CheckerPainter(this.brightness);
  final Brightness brightness;

  @override
  void paint(Canvas canvas, Size size) {
    const cell = 12.0;
    final darkTheme = brightness == Brightness.dark;
    final light = Paint()
      ..color = darkTheme ? const Color(0xFF3A3A3A) : const Color(0xFFE6E6E6);
    final dark = Paint()
      ..color = darkTheme ? const Color(0xFF2B2B2B) : const Color(0xFFC8C8C8);
    canvas.drawRect(Offset.zero & size, light);
    for (var y = 0.0; y < size.height; y += cell) {
      for (var x = 0.0; x < size.width; x += cell) {
        if ((((x / cell).floor() + (y / cell).floor()) & 1) == 1) {
          canvas.drawRect(Rect.fromLTWH(x, y, cell, cell), dark);
        }
      }
    }
  }

  @override
  bool shouldRepaint(covariant _CheckerPainter oldDelegate) =>
      oldDelegate.brightness != brightness;
}

@immutable
final class _TexturePreviewKey {
  const _TexturePreviewKey({
    required this.sourceFingerprint,
    required this.assetPath,
  });

  final Revision3TextureSourceFingerprint sourceFingerprint;
  final String assetPath;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is _TexturePreviewKey &&
          other.sourceFingerprint == sourceFingerprint &&
          other.assetPath == assetPath;

  @override
  int get hashCode => Object.hash(sourceFingerprint, assetPath);
}

final class _TexturePreviewRequestDiscarded implements Exception {
  const _TexturePreviewRequestDiscarded();
}

final class _TexturePreviewQueueDisposed implements Exception {
  const _TexturePreviewQueueDisposed();
}
