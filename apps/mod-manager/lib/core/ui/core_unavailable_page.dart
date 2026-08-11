import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:package_info_plus/package_info_plus.dart';

import '../../app/ui/window_chrome.dart';
import '../../l10n/app_localizations.dart';
import '../core_service.dart';

class CoreUnavailablePage extends StatefulWidget {
  const CoreUnavailablePage({super.key, required this.failure})
    : _managerVersionLoader = null;

  /// Test-only seam for controlling the asynchronous package-info lookup.
  const CoreUnavailablePage.forTesting(
    this._managerVersionLoader, {
    super.key,
    required this.failure,
  });

  final CoreBootstrapFailure failure;
  final Future<String?> Function()? _managerVersionLoader;

  @override
  State<CoreUnavailablePage> createState() => _CoreUnavailablePageState();
}

class _CoreUnavailablePageState extends State<CoreUnavailablePage> {
  final FocusNode _copyFocusNode = FocusNode(
    debugLabel: 'core-unavailable-copy-details',
  );
  final GlobalKey _copyVisibilityKey = GlobalKey(
    debugLabel: 'core-unavailable-copy-visibility',
  );
  late final Future<void> _managerVersionReady;
  String? _managerVersion;
  bool _copied = false;
  bool _copyFailed = false;

  @override
  void initState() {
    super.initState();
    _managerVersionReady = _loadManagerVersion();
    WidgetsBinding.instance.addPostFrameCallback((_) => _revealCopyAction());
  }

  @override
  void dispose() {
    _copyFocusNode.dispose();
    super.dispose();
  }

  Future<void> _loadManagerVersion() async {
    try {
      final loader = widget._managerVersionLoader;
      final version = loader == null
          ? (await PackageInfo.fromPlatform()).version
          : await loader();
      if (mounted) setState(() => _managerVersion = version);
    } catch (_) {
      // Version metadata is useful support evidence, never a prerequisite for
      // showing or copying the bounded bootstrap report.
    }
  }

  Future<void> _copyDetails() async {
    await _managerVersionReady;
    if (!mounted) return;
    var copied = false;
    try {
      await Clipboard.setData(ClipboardData(text: _technicalReport));
      copied = true;
    } catch (_) {
      // Keep the blocker usable when another process temporarily owns the
      // Windows clipboard; the live error below invites a retry.
    }
    if (!mounted) return;
    setState(() {
      _copied = copied;
      _copyFailed = !copied;
    });
    WidgetsBinding.instance.addPostFrameCallback((_) {
      if (!mounted) return;
      _copyFocusNode.requestFocus();
      _revealCopyAction();
    });
  }

  void _revealCopyAction() {
    final copyContext = _copyVisibilityKey.currentContext;
    if (mounted && copyContext != null) {
      Scrollable.ensureVisible(copyContext, alignment: 0.5);
    }
  }

  String get _technicalReport =>
      widget.failure.technicalReport(managerVersion: _managerVersion);

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context);
    final message = _messageFor(l10n, widget.failure);
    final statusLabel = [
      message,
      l10n.coreBlockedRepairHint,
      if (_copied) l10n.coreTechnicalDetailsCopied,
      if (_copyFailed) l10n.coreTechnicalDetailsCopyFailed,
    ].join(' ');

    return Scaffold(
      key: const ValueKey('core-unavailable-page'),
      appBar: AppBar(
        titleSpacing: 0,
        centerTitle: false,
        scrolledUnderElevation: 0,
        title: WindowDragArea(
          child: Row(
            children: [
              const SizedBox(width: 8),
              Image.asset(
                'assets/gore_manager_icon.png',
                height: 28,
                excludeFromSemantics: true,
              ),
              const SizedBox(width: 10),
              Expanded(
                child: Text(
                  l10n.appTitle,
                  maxLines: 1,
                  overflow: TextOverflow.ellipsis,
                ),
              ),
            ],
          ),
        ),
        actions: const [WindowControls()],
      ),
      body: SafeArea(
        child: SingleChildScrollView(
          key: const ValueKey('core-unavailable-scroll'),
          padding: const EdgeInsets.all(24),
          child: Align(
            alignment: Alignment.topCenter,
            child: ConstrainedBox(
              constraints: const BoxConstraints(maxWidth: 680),
              child: Card(
                child: Padding(
                  padding: const EdgeInsets.all(24),
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      ExcludeSemantics(
                        child: Icon(
                          Icons.extension_off_outlined,
                          size: 44,
                          color: Theme.of(context).colorScheme.error,
                        ),
                      ),
                      const SizedBox(height: 16),
                      Semantics(
                        key: const ValueKey('core-unavailable-heading'),
                        header: true,
                        child: Text(
                          l10n.coreBlockedTitle,
                          style: Theme.of(context).textTheme.headlineSmall,
                        ),
                      ),
                      const SizedBox(height: 12),
                      Semantics(
                        key: const ValueKey('core-unavailable-live-region'),
                        container: true,
                        liveRegion: true,
                        label: statusLabel,
                        child: ExcludeSemantics(
                          child: Column(
                            crossAxisAlignment: CrossAxisAlignment.start,
                            children: [
                              Text(message),
                              const SizedBox(height: 8),
                              Text(l10n.coreBlockedRepairHint),
                              if (_copied) ...[
                                const SizedBox(height: 8),
                                Text(
                                  l10n.coreTechnicalDetailsCopied,
                                  key: const ValueKey(
                                    'core-technical-details-copied',
                                  ),
                                  style: TextStyle(
                                    color: Theme.of(
                                      context,
                                    ).colorScheme.primary,
                                  ),
                                ),
                              ],
                              if (_copyFailed) ...[
                                const SizedBox(height: 8),
                                Text(
                                  l10n.coreTechnicalDetailsCopyFailed,
                                  key: const ValueKey(
                                    'core-technical-details-copy-failed',
                                  ),
                                  style: TextStyle(
                                    color: Theme.of(context).colorScheme.error,
                                  ),
                                ),
                              ],
                            ],
                          ),
                        ),
                      ),
                      const SizedBox(height: 20),
                      KeyedSubtree(
                        key: _copyVisibilityKey,
                        child: FilledButton.icon(
                          key: const ValueKey('core-copy-details-action'),
                          focusNode: _copyFocusNode,
                          autofocus: true,
                          onPressed: _copyDetails,
                          icon: const Icon(Icons.copy_outlined),
                          label: Text(l10n.coreCopyTechnicalDetails),
                        ),
                      ),
                      const SizedBox(height: 20),
                      Text(
                        l10n.coreTechnicalDetails,
                        key: const ValueKey('core-technical-details-heading'),
                        style: Theme.of(context).textTheme.titleSmall,
                      ),
                      const SizedBox(height: 8),
                      SelectableText(
                        _technicalReport,
                        key: const ValueKey('core-technical-details'),
                        style: Theme.of(context).textTheme.bodySmall?.copyWith(
                          fontFamily: 'monospace',
                        ),
                      ),
                    ],
                  ),
                ),
              ),
            ),
          ),
        ),
      ),
    );
  }
}

String _messageFor(AppLocalizations l10n, CoreBootstrapFailure failure) =>
    switch (failure.reason) {
      CoreBootstrapFailureReason.dllMissing => l10n.coreDllMissingMessage,
      CoreBootstrapFailureReason.dllLoadFailed => l10n.coreDllLoadFailedMessage,
      CoreBootstrapFailureReason.coreInfoInvalid =>
        l10n.coreVerificationFailedMessage,
      CoreBootstrapFailureReason.requiredCommandsMissing =>
        l10n.coreCommandsMissingMessage,
      CoreBootstrapFailureReason.transportAbiMismatch ||
      CoreBootstrapFailureReason.protocolAbiMismatch =>
        switch (failure.compatibilityDirection) {
          CoreCompatibilityDirection.managerTooOld =>
            l10n.coreManagerTooOldMessage,
          CoreCompatibilityDirection.coreTooOld => l10n.coreNativeTooOldMessage,
          null => l10n.coreVerificationFailedMessage,
        },
    };
