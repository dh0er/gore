import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';

/// The character detail sub-tab named [label].
///
/// The bar drops the labels when the pane is too narrow for them and names the
/// tabs by tooltip instead, which every test surface is — `setSurfaceSize` does
/// not widen the render view here, so tests always run in the narrow regime.
/// Matching either form keeps a test about inventory or position from turning
/// into a test about the tab bar's breakpoint.
Finder detailTab(String label) => find.byWidgetPredicate((widget) {
  if (widget is! Tab) return false;
  if (widget.text == label) return true;
  final icon = widget.icon;
  return icon is Tooltip && icon.message == label;
}, description: 'character detail tab "$label"');
