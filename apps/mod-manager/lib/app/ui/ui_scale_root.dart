import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../domain/ui_settings.dart';

/// Wraps the app in zoom shortcuts (Ctrl +/-, Ctrl+0) and applies the
/// current UI scale to the whole widget tree.
class UiScaleRoot extends ConsumerWidget {
  const UiScaleRoot({super.key, required this.child});

  final Widget child;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final scale = ref.watch(uiScaleProvider);

    return Shortcuts(
      shortcuts: const {
        SingleActivator(LogicalKeyboardKey.equal, control: true):
            _ZoomInIntent(),
        SingleActivator(LogicalKeyboardKey.add, control: true): _ZoomInIntent(),
        SingleActivator(LogicalKeyboardKey.numpadAdd, control: true):
            _ZoomInIntent(),
        SingleActivator(LogicalKeyboardKey.minus, control: true):
            _ZoomOutIntent(),
        SingleActivator(LogicalKeyboardKey.numpadSubtract, control: true):
            _ZoomOutIntent(),
        SingleActivator(LogicalKeyboardKey.digit0, control: true):
            _ZoomResetIntent(),
        SingleActivator(LogicalKeyboardKey.numpad0, control: true):
            _ZoomResetIntent(),
      },
      child: Actions(
        actions: {
          _ZoomInIntent: CallbackAction<_ZoomInIntent>(
            onInvoke: (_) {
              ref.read(uiScaleProvider.notifier).increase();
              return null;
            },
          ),
          _ZoomOutIntent: CallbackAction<_ZoomOutIntent>(
            onInvoke: (_) {
              ref.read(uiScaleProvider.notifier).decrease();
              return null;
            },
          ),
          _ZoomResetIntent: CallbackAction<_ZoomResetIntent>(
            onInvoke: (_) {
              ref.read(uiScaleProvider.notifier).reset();
              return null;
            },
          ),
        },
        // Ensure there's always a focus in the tree so the shortcuts work
        // even when nothing specific is focused.
        child: Focus(
          autofocus: true,
          child: _UiScaleView(scale: scale, child: child),
        ),
      ),
    );
  }
}

class _ZoomInIntent extends Intent {
  const _ZoomInIntent();
}

class _ZoomOutIntent extends Intent {
  const _ZoomOutIntent();
}

class _ZoomResetIntent extends Intent {
  const _ZoomResetIntent();
}

/// Root UI scaling that avoids clipping by compensating layout constraints.
///
/// - Paint: scaled by [scale]
/// - Hit testing: scaled with paint
/// - Layout: child receives constraints divided by [scale] (when bounded),
///   so after scaling the content still fits the original viewport.
class _UiScaleView extends StatelessWidget {
  const _UiScaleView({required this.scale, required this.child});

  final double scale;
  final Widget child;

  static EdgeInsets _divInsets(EdgeInsets v, double d) {
    return EdgeInsets.fromLTRB(
      v.left / d,
      v.top / d,
      v.right / d,
      v.bottom / d,
    );
  }

  static MediaQueryData _scaledMediaQuery(MediaQueryData data, double scale) {
    // Keep text scaling unchanged: the whole UI is already being scaled.
    return data.copyWith(
      size: data.size / scale,
      padding: _divInsets(data.padding, scale),
      viewPadding: _divInsets(data.viewPadding, scale),
      viewInsets: _divInsets(data.viewInsets, scale),
      systemGestureInsets: _divInsets(data.systemGestureInsets, scale),
    );
  }

  @override
  Widget build(BuildContext context) {
    if (scale == 1.0) return child;

    final outerMq = MediaQuery.of(context);

    return LayoutBuilder(
      builder: (context, constraints) {
        // Prefer bounded constraints; fall back to view size.
        final view = View.of(context);
        final viewSize = view.physicalSize / view.devicePixelRatio;

        final outerW = constraints.hasBoundedWidth
            ? constraints.maxWidth
            : viewSize.width;
        final outerH = constraints.hasBoundedHeight
            ? constraints.maxHeight
            : viewSize.height;

        final innerW = outerW / scale;
        final innerH = outerH / scale;

        final scaledChild = MediaQuery(
          data: _scaledMediaQuery(outerMq, scale),
          child: child,
        );

        // FittedBox scales the child to fill the parent AND reports the
        // correct layout size, so the parent sees (outerW, outerH) instead of
        // the unscaled (innerW, innerH). This avoids zoom-out black margins.
        return SizedBox(
          width: outerW,
          height: outerH,
          child: FittedBox(
            fit: BoxFit.fill,
            alignment: Alignment.topLeft,
            child: SizedBox(width: innerW, height: innerH, child: scaledChild),
          ),
        );
      },
    );
  }
}
