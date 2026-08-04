import 'dart:io';

import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:goresave/l10n/app_localizations.dart';
import 'package:window_manager/window_manager.dart';

/// Whether the custom window chrome (frameless window, drag area, window
/// buttons) is active. Disabled in widget tests, where the window_manager
/// plugin is not available.
bool get windowChromeEnabled =>
    !kIsWeb &&
    (Platform.isWindows || Platform.isLinux || Platform.isMacOS) &&
    !Platform.environment.containsKey('FLUTTER_TEST');

/// Makes [child] act as the draggable title bar area:
/// dragging moves the window, double-click toggles maximize/restore.
class WindowDragArea extends StatelessWidget {
  const WindowDragArea({super.key, required this.child});

  final Widget child;

  Future<void> _toggleMaximize() async {
    if (await windowManager.isMaximized()) {
      await windowManager.unmaximize();
    } else {
      await windowManager.maximize();
    }
  }

  @override
  Widget build(BuildContext context) {
    if (!windowChromeEnabled) return child;
    return GestureDetector(
      behavior: HitTestBehavior.translucent,
      onDoubleTap: _toggleMaximize,
      onPanStart: (_) => windowManager.startDragging(),
      child: child,
    );
  }
}

/// Custom window control buttons (minimize, maximize/restore, close).
class WindowControls extends StatefulWidget {
  const WindowControls({super.key});

  @override
  State<WindowControls> createState() => _WindowControlsState();
}

class _WindowControlsState extends State<WindowControls> with WindowListener {
  bool _isMaximized = false;

  @override
  void initState() {
    super.initState();
    if (windowChromeEnabled) {
      windowManager.addListener(this);
      _checkMaximized();
    }
  }

  @override
  void dispose() {
    if (windowChromeEnabled) {
      windowManager.removeListener(this);
    }
    super.dispose();
  }

  @override
  void onWindowMaximize() {
    super.onWindowMaximize();
    if (!mounted) return;
    setState(() => _isMaximized = true);
  }

  @override
  void onWindowUnmaximize() {
    super.onWindowUnmaximize();
    if (!mounted) return;
    setState(() => _isMaximized = false);
  }

  @override
  void onWindowRestore() {
    super.onWindowRestore();
    if (!mounted) return;
    // Restore typically means "un-minimize". Re-check the real state.
    _checkMaximized();
  }

  Future<void> _checkMaximized() async {
    final isMaximized = await windowManager.isMaximized();
    if (mounted && _isMaximized != isMaximized) {
      setState(() => _isMaximized = isMaximized);
    }
  }

  @override
  Widget build(BuildContext context) {
    if (!windowChromeEnabled) return const SizedBox.shrink();

    final l10n = AppLocalizations.of(context);
    final isDark = Theme.of(context).brightness == Brightness.dark;

    return Row(
      mainAxisSize: MainAxisSize.min,
      children: [
        _WindowControlButton(
          icon: Icons.remove,
          tooltip: l10n.windowMinimizeTooltip,
          onPressed: () => windowManager.minimize(),
          isDark: isDark,
        ),
        _WindowControlButton(
          icon: _isMaximized ? Icons.filter_none : Icons.crop_square,
          tooltip: _isMaximized
              ? l10n.windowRestoreTooltip
              : l10n.windowMaximizeTooltip,
          onPressed: () async {
            if (_isMaximized) {
              // `restore()` is primarily for un-minimizing; for
              // "maximized -> normal" the correct API is `unmaximize()`.
              await windowManager.unmaximize();
            } else {
              await windowManager.maximize();
            }
            await _checkMaximized();
          },
          isDark: isDark,
        ),
        _WindowControlButton(
          icon: Icons.close,
          tooltip: l10n.close,
          onPressed: () => windowManager.close(),
          isDark: isDark,
          isCloseButton: true,
        ),
      ],
    );
  }
}

class _WindowControlButton extends StatelessWidget {
  const _WindowControlButton({
    required this.icon,
    required this.tooltip,
    required this.onPressed,
    required this.isDark,
    this.isCloseButton = false,
  });

  final IconData icon;
  final String tooltip;
  final VoidCallback onPressed;
  final bool isDark;
  final bool isCloseButton;

  @override
  Widget build(BuildContext context) {
    return Tooltip(
      message: tooltip,
      child: Material(
        color: Colors.transparent,
        child: InkWell(
          onTap: onPressed,
          child: Container(
            width: 46,
            height: 32,
            alignment: Alignment.center,
            child: Icon(
              icon,
              size: 16,
              color: isCloseButton
                  ? (isDark ? Colors.white : Colors.black87)
                  : (isDark ? Colors.white70 : Colors.black54),
            ),
          ),
        ),
      ),
    );
  }
}
