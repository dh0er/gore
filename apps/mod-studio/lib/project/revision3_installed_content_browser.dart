import 'dart:async';

import 'package:flutter/material.dart';

import '../core/mod_ffi.dart';
import '../dataasset/ui/installed_package_browser_dialog.dart'
    show Revision3InstalledPackageIndexLoader;

typedef Revision3InstalledCandidateCountCopy = String Function(int count);

/// All author-facing copy used by [Revision3InstalledContentBrowser].
@immutable
final class Revision3InstalledContentBrowserCopy {
  const Revision3InstalledContentBrowserCopy({
    required this.setupTitle,
    required this.setupDescription,
    required this.setupActionLabel,
    required this.loadingLabel,
    required this.completeSummary,
    required this.partialSummary,
    required this.completeDescription,
    required this.partialDescription,
    required this.authorityNotice,
    required this.refreshTooltip,
    required this.searchLabel,
    required this.searchHint,
    required this.searchPrompt,
    required this.noMatchesTitle,
    required this.noMatchesDescription,
    required this.resultLimitDescription,
    required this.kindBadgeLabel,
    required this.sourceBadgeLabel,
    required this.readinessBadgeLabel,
    required this.openInspectorLabel,
    required this.errorTitle,
    required this.errorDescription,
    required this.retryLabel,
  });

  final String setupTitle;
  final String setupDescription;
  final String setupActionLabel;
  final String loadingLabel;
  final Revision3InstalledCandidateCountCopy completeSummary;
  final Revision3InstalledCandidateCountCopy partialSummary;
  final String completeDescription;
  final String partialDescription;
  final String authorityNotice;
  final String refreshTooltip;
  final String searchLabel;
  final String searchHint;
  final String searchPrompt;
  final String noMatchesTitle;
  final String noMatchesDescription;
  final String resultLimitDescription;
  final String kindBadgeLabel;
  final String sourceBadgeLabel;
  final String readinessBadgeLabel;
  final String openInspectorLabel;
  final String errorTitle;
  final String errorDescription;
  final String retryLabel;
}

/// Embedded, search-first view over one exact installed metadata snapshot.
///
/// [sourceIdentity] is an opaque lifetime token supplied by the owner of the
/// exact managed source. It is never rendered or treated as package authority.
/// Changing either source input invalidates all pending work and starts a fresh
/// audit. Rows expose discovery metadata only; [openInspector] receives the
/// canonical path selected from that exact snapshot and grants no edit, build,
/// deployment, runtime, game-installation, or save authority.
final class Revision3InstalledContentBrowser extends StatefulWidget {
  const Revision3InstalledContentBrowser({
    required this.gameRoot,
    required this.sourceIdentity,
    required this.loader,
    required this.copy,
    this.openSettings,
    this.openInspector,
    super.key,
  });

  final String? gameRoot;
  final Object? sourceIdentity;
  final Revision3InstalledPackageIndexLoader loader;
  final Revision3InstalledContentBrowserCopy copy;
  final VoidCallback? openSettings;
  final ValueChanged<String>? openInspector;

  @override
  State<Revision3InstalledContentBrowser> createState() =>
      _Revision3InstalledContentBrowserState();
}

class _Revision3InstalledContentBrowserState
    extends State<Revision3InstalledContentBrowser> {
  static const _maximumVisibleCandidates = 100;

  late final TextEditingController _search;
  AuthoringRevision3DataAssetPackageIndexResult? _result;
  Object? _error;
  bool _loading = false;
  int _loadEpoch = 0;
  String _query = '';

  bool get _sourceAvailable =>
      widget.gameRoot?.trim().isNotEmpty == true &&
      widget.sourceIdentity != null;

  @override
  void initState() {
    super.initState();
    _search = TextEditingController()..addListener(_onSearchChanged);
    _startLoad(notify: false);
  }

  @override
  void didUpdateWidget(covariant Revision3InstalledContentBrowser oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.gameRoot == widget.gameRoot &&
        oldWidget.sourceIdentity == widget.sourceIdentity) {
      return;
    }
    _startLoad(notify: false);
  }

  @override
  void dispose() {
    _loadEpoch++;
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

  void _startLoad({bool notify = true}) {
    final epoch = ++_loadEpoch;
    final root = widget.gameRoot?.trim();
    final sourceIdentity = widget.sourceIdentity;
    if (root == null || root.isEmpty || sourceIdentity == null) {
      void clear() {
        _loading = false;
        _result = null;
        _error = null;
      }

      if (notify) {
        setState(clear);
      } else {
        clear();
      }
      return;
    }

    void markLoading() {
      _loading = true;
      _result = null;
      _error = null;
    }

    if (notify) {
      setState(markLoading);
    } else {
      markLoading();
    }

    Future<AuthoringRevision3DataAssetPackageIndexResult>.sync(
      () => widget.loader(gameRoot: root),
    ).then(
      (result) {
        if (!mounted || epoch != _loadEpoch) return;
        setState(() {
          _loading = false;
          _result = result;
          _error = null;
        });
      },
      onError: (Object error, StackTrace _) {
        if (!mounted || epoch != _loadEpoch) return;
        setState(() {
          _loading = false;
          _result = null;
          _error = error;
        });
      },
    );
  }

  @override
  Widget build(BuildContext context) => Semantics(
    key: const Key('revision3-installed-content-browser'),
    container: true,
    explicitChildNodes: true,
    child: !_sourceAvailable
        ? _SetupState(copy: widget.copy, openSettings: widget.openSettings)
        : _loading
        ? _LoadingState(label: widget.copy.loadingLabel)
        : _error != null
        ? _ErrorState(copy: widget.copy, retry: () => _startLoad())
        : _buildResult(context, _result!),
  );

  Widget _buildResult(
    BuildContext context,
    AuthoringRevision3DataAssetPackageIndexResult result,
  ) {
    final candidates = result.index.candidates;
    final matches = <AuthoringRevision3DataAssetPackageCandidate>[];
    var matchesTruncated = false;
    if (_query.isNotEmpty) {
      for (final candidate in candidates) {
        final path = candidate.targetPath.toLowerCase();
        final separator = path.lastIndexOf('/');
        final name = separator < 0 ? path : path.substring(separator + 1);
        if (!path.contains(_query) && !name.contains(_query)) continue;
        if (matches.length == _maximumVisibleCandidates) {
          matchesTruncated = true;
          break;
        }
        matches.add(candidate);
      }
    }

    final complete =
        result.index.status ==
        AuthoringRevision3DataAssetPackageIndexStatus.completeIndex;
    return CustomScrollView(
      key: const Key('revision3-installed-content-browser-result'),
      slivers: [
        SliverPadding(
          padding: const EdgeInsets.fromLTRB(12, 12, 12, 0),
          sliver: SliverToBoxAdapter(
            child: _SnapshotSummary(
              copy: widget.copy,
              complete: complete,
              candidateCount: candidates.length,
              refresh: () => _startLoad(),
            ),
          ),
        ),
        SliverPadding(
          padding: const EdgeInsets.fromLTRB(16, 8, 16, 0),
          sliver: SliverToBoxAdapter(child: Text(widget.copy.authorityNotice)),
        ),
        SliverPadding(
          padding: const EdgeInsets.fromLTRB(12, 12, 12, 8),
          sliver: SliverToBoxAdapter(
            child: TextField(
              key: const Key('revision3-installed-content-browser-search'),
              controller: _search,
              textInputAction: TextInputAction.search,
              decoration: InputDecoration(
                labelText: widget.copy.searchLabel,
                hintText: widget.copy.searchHint,
                prefixIcon: const Icon(Icons.search),
                suffixIcon: _query.isEmpty
                    ? null
                    : IconButton(
                        key: const Key(
                          'revision3-installed-content-browser-clear-search',
                        ),
                        tooltip: MaterialLocalizations.of(
                          context,
                        ).deleteButtonTooltip,
                        onPressed: _search.clear,
                        icon: const Icon(Icons.clear),
                      ),
                border: const OutlineInputBorder(),
              ),
            ),
          ),
        ),
        if (_query.isEmpty)
          SliverFillRemaining(
            hasScrollBody: false,
            child: _PromptState(
              key: const Key(
                'revision3-installed-content-browser-search-prompt',
              ),
              icon: Icons.manage_search_outlined,
              message: widget.copy.searchPrompt,
            ),
          )
        else if (matches.isEmpty)
          SliverFillRemaining(
            hasScrollBody: false,
            child: _NoMatchesState(copy: widget.copy),
          )
        else ...[
          if (matchesTruncated)
            SliverPadding(
              padding: const EdgeInsets.fromLTRB(16, 0, 16, 4),
              sliver: SliverToBoxAdapter(
                child: Text(
                  widget.copy.resultLimitDescription,
                  key: const Key(
                    'revision3-installed-content-browser-result-limit',
                  ),
                  style: Theme.of(context).textTheme.bodySmall,
                ),
              ),
            ),
          SliverPadding(
            padding: const EdgeInsets.fromLTRB(8, 0, 8, 12),
            sliver: SliverList.builder(
              itemCount: matches.length,
              itemBuilder: (context, index) => _CandidateRow(
                candidate: matches[index],
                copy: widget.copy,
                openInspector: widget.openInspector,
              ),
            ),
          ),
        ],
      ],
    );
  }
}

final class _SetupState extends StatelessWidget {
  const _SetupState({required this.copy, required this.openSettings});

  final Revision3InstalledContentBrowserCopy copy;
  final VoidCallback? openSettings;

  @override
  Widget build(BuildContext context) => _ScrollableCenteredState(
    stateKey: const Key('revision3-installed-content-browser-setup'),
    icon: Icons.settings_outlined,
    title: copy.setupTitle,
    description: copy.setupDescription,
    action: FilledButton.icon(
      key: const Key('revision3-installed-content-browser-setup-action'),
      onPressed: openSettings,
      icon: const Icon(Icons.settings_outlined),
      label: Text(copy.setupActionLabel),
    ),
  );
}

final class _LoadingState extends StatelessWidget {
  const _LoadingState({required this.label});

  final String label;

  @override
  Widget build(BuildContext context) => Center(
    key: const Key('revision3-installed-content-browser-loading'),
    child: SingleChildScrollView(
      padding: const EdgeInsets.all(20),
      child: Column(
        mainAxisSize: MainAxisSize.min,
        children: [
          const CircularProgressIndicator(),
          const SizedBox(height: 16),
          Text(label, textAlign: TextAlign.center),
        ],
      ),
    ),
  );
}

final class _ErrorState extends StatelessWidget {
  const _ErrorState({required this.copy, required this.retry});

  final Revision3InstalledContentBrowserCopy copy;
  final VoidCallback retry;

  @override
  Widget build(BuildContext context) => _ScrollableCenteredState(
    stateKey: const Key('revision3-installed-content-browser-error'),
    icon: Icons.error_outline,
    title: copy.errorTitle,
    description: copy.errorDescription,
    iconColor: Theme.of(context).colorScheme.error,
    action: FilledButton.icon(
      key: const Key('revision3-installed-content-browser-retry'),
      onPressed: retry,
      icon: const Icon(Icons.refresh),
      label: Text(copy.retryLabel),
    ),
  );
}

final class _ScrollableCenteredState extends StatelessWidget {
  const _ScrollableCenteredState({
    required this.stateKey,
    required this.icon,
    required this.title,
    required this.description,
    required this.action,
    this.iconColor,
  });

  final Key stateKey;
  final IconData icon;
  final String title;
  final String description;
  final Widget action;
  final Color? iconColor;

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
                Icon(icon, size: 44, color: iconColor),
                const SizedBox(height: 12),
                Text(
                  title,
                  textAlign: TextAlign.center,
                  style: Theme.of(context).textTheme.titleMedium,
                ),
                const SizedBox(height: 8),
                Text(description, textAlign: TextAlign.center),
                const SizedBox(height: 16),
                action,
              ],
            ),
          ),
        ),
      ),
    ),
  );
}

final class _SnapshotSummary extends StatelessWidget {
  const _SnapshotSummary({
    required this.copy,
    required this.complete,
    required this.candidateCount,
    required this.refresh,
  });

  final Revision3InstalledContentBrowserCopy copy;
  final bool complete;
  final int candidateCount;
  final VoidCallback refresh;

  @override
  Widget build(BuildContext context) {
    final color = complete
        ? Theme.of(context).colorScheme.primaryContainer
        : Theme.of(context).colorScheme.tertiaryContainer;
    return Card(
      key: const Key('revision3-installed-content-browser-summary'),
      color: color,
      child: ListTile(
        leading: Icon(
          complete ? Icons.verified_outlined : Icons.warning_amber_outlined,
        ),
        title: Text(
          complete
              ? copy.completeSummary(candidateCount)
              : copy.partialSummary(candidateCount),
        ),
        subtitle: Text(
          complete ? copy.completeDescription : copy.partialDescription,
        ),
        trailing: IconButton(
          key: const Key('revision3-installed-content-browser-refresh'),
          tooltip: copy.refreshTooltip,
          onPressed: refresh,
          icon: const Icon(Icons.refresh),
        ),
      ),
    );
  }
}

final class _PromptState extends StatelessWidget {
  const _PromptState({required this.icon, required this.message, super.key});

  final IconData icon;
  final String message;

  @override
  Widget build(BuildContext context) => Center(
    child: Padding(
      padding: const EdgeInsets.all(20),
      child: Column(
        mainAxisSize: MainAxisSize.min,
        children: [
          Icon(icon, size: 42),
          const SizedBox(height: 10),
          Text(message, textAlign: TextAlign.center),
        ],
      ),
    ),
  );
}

final class _NoMatchesState extends StatelessWidget {
  const _NoMatchesState({required this.copy});

  final Revision3InstalledContentBrowserCopy copy;

  @override
  Widget build(BuildContext context) => Center(
    key: const Key('revision3-installed-content-browser-no-matches'),
    child: Padding(
      padding: const EdgeInsets.all(20),
      child: Column(
        mainAxisSize: MainAxisSize.min,
        children: [
          const Icon(Icons.search_off_outlined, size: 42),
          const SizedBox(height: 10),
          Text(
            copy.noMatchesTitle,
            textAlign: TextAlign.center,
            style: Theme.of(context).textTheme.titleSmall,
          ),
          const SizedBox(height: 6),
          Text(copy.noMatchesDescription, textAlign: TextAlign.center),
        ],
      ),
    ),
  );
}

final class _CandidateRow extends StatelessWidget {
  const _CandidateRow({
    required this.candidate,
    required this.copy,
    required this.openInspector,
  });

  final AuthoringRevision3DataAssetPackageCandidate candidate;
  final Revision3InstalledContentBrowserCopy copy;
  final ValueChanged<String>? openInspector;

  @override
  Widget build(BuildContext context) {
    final separator = candidate.targetPath.lastIndexOf('/');
    final name = separator < 0
        ? candidate.targetPath
        : candidate.targetPath.substring(separator + 1);
    return Card(
      key: ValueKey(
        'revision3-installed-content-browser-row-${candidate.ordinal}',
      ),
      child: Padding(
        padding: const EdgeInsets.all(12),
        child: LayoutBuilder(
          builder: (context, constraints) {
            final compact = constraints.maxWidth < 520;
            final details = Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(name, style: Theme.of(context).textTheme.titleSmall),
                const SizedBox(height: 3),
                SelectableText(
                  candidate.targetPath,
                  key: ValueKey(
                    'revision3-installed-content-browser-path-${candidate.ordinal}',
                  ),
                  style: Theme.of(context).textTheme.bodySmall,
                ),
                const SizedBox(height: 8),
                Wrap(
                  spacing: 6,
                  runSpacing: 6,
                  children: [
                    _MetadataBadge(
                      key: ValueKey(
                        'revision3-installed-content-browser-kind-${candidate.ordinal}',
                      ),
                      label: copy.kindBadgeLabel,
                    ),
                    _MetadataBadge(
                      key: ValueKey(
                        'revision3-installed-content-browser-source-${candidate.ordinal}',
                      ),
                      label: copy.sourceBadgeLabel,
                    ),
                    _MetadataBadge(
                      key: ValueKey(
                        'revision3-installed-content-browser-readiness-${candidate.ordinal}',
                      ),
                      label: copy.readinessBadgeLabel,
                    ),
                  ],
                ),
              ],
            );
            final action = OutlinedButton.icon(
              key: ValueKey(
                'revision3-installed-content-browser-open-${candidate.ordinal}',
              ),
              onPressed: openInspector == null
                  ? null
                  : () {
                      FocusScope.of(context).unfocus();
                      openInspector!(candidate.targetPath);
                    },
              icon: const Icon(Icons.manage_search_outlined),
              label: Text(copy.openInspectorLabel),
            );
            if (compact) {
              return Column(
                crossAxisAlignment: CrossAxisAlignment.stretch,
                children: [details, const SizedBox(height: 10), action],
              );
            }
            return Row(
              crossAxisAlignment: CrossAxisAlignment.center,
              children: [
                Expanded(child: details),
                const SizedBox(width: 12),
                action,
              ],
            );
          },
        ),
      ),
    );
  }
}

final class _MetadataBadge extends StatelessWidget {
  const _MetadataBadge({required this.label, super.key});

  final String label;

  @override
  Widget build(BuildContext context) => Container(
    padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 4),
    decoration: BoxDecoration(
      color: Theme.of(context).colorScheme.surfaceContainerHighest,
      borderRadius: BorderRadius.circular(999),
    ),
    child: Text(label, style: Theme.of(context).textTheme.labelSmall),
  );
}
