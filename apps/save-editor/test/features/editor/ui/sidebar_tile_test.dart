import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:goresave/features/app/domain/ui_settings.dart';
import 'package:goresave/features/editor/ui/sidebar_tile.dart';
import 'package:goresave/l10n/app_localizations.dart';
import 'package:goresave/ui/design/app_theme.dart';

void main() {
  testWidgets('a category count wrapper stays on the interface face', (
    tester,
  ) async {
    const zh = Locale.fromSubtags(languageCode: 'zh', scriptCode: 'Hans');
    await tester.pumpWidget(
      MaterialApp(
        theme: buildGoresaveTheme(
          uiFontFamily: UiFontFamily.notoSerif,
          locale: const Locale('ja'),
          gameTextLocale: zh,
        ),
        locale: const Locale('ja'),
        localizationsDelegates: AppLocalizations.localizationsDelegates,
        supportedLocales: AppLocalizations.supportedLocales,
        home: Scaffold(
          body: Builder(
            builder: (context) {
              final l10n = AppLocalizations.of(context);
              final parts = catalogCountParts(l10n, '铁', 3);
              return SidebarTile(
                icon: Icons.category_outlined,
                label: parts.full,
                catalogRun: parts.splits ? parts.run : null,
                catalogLead: parts.lead,
                catalogTail: parts.tail,
                gameTextLocale: zh,
                selected: false,
                onTap: () {},
              );
            },
          ),
        ),
      ),
    );

    final rich = tester.widget<RichText>(
      find.byWidgetPredicate(
        (widget) =>
            widget is RichText && widget.text.toPlainText().contains('铁'),
      ),
    );
    final runs = _runs(rich.text);
    expect(runs.map((span) => span.text), ['铁', '（3）']);
    expect(runs[0].style?.fontFamily, notoSerifScFontFamily);
    expect(runs[1].style?.fontFamily, notoSerifJpFontFamily);
  });

  testWidgets(
    'a truncated category tooltip keeps the wrapper on the interface face',
    (tester) async {
      const zh = Locale.fromSubtags(languageCode: 'zh', scriptCode: 'Hans');
      await tester.pumpWidget(
        MaterialApp(
          theme: buildGoresaveTheme(
            uiFontFamily: UiFontFamily.notoSerif,
            locale: const Locale('ja'),
            gameTextLocale: zh,
          ),
          locale: const Locale('ja'),
          localizationsDelegates: AppLocalizations.localizationsDelegates,
          supportedLocales: AppLocalizations.supportedLocales,
          home: Scaffold(
            body: SizedBox(
              width: 72,
              child: Builder(
                builder: (context) {
                  final l10n = AppLocalizations.of(context);
                  final parts = catalogCountParts(l10n, '很长的武器分类', 12);
                  return SidebarTile(
                    icon: Icons.category_outlined,
                    label: parts.full,
                    catalogRun: parts.splits ? parts.run : null,
                    catalogLead: parts.lead,
                    catalogTail: parts.tail,
                    gameTextLocale: zh,
                    selected: false,
                    onTap: () {},
                  );
                },
              ),
            ),
          ),
        ),
      );

      final tooltip = tester.widget<Tooltip>(find.byType(Tooltip));
      final runs = _runs(tooltip.richMessage!);
      expect(runs.map((span) => span.text), ['很长的武器分类', '（12）']);
      expect(runs[0].style?.fontFamily, notoSerifScFontFamily);
      expect(runs[1].style?.fontFamily, isNot(notoSerifScFontFamily));
    },
  );
}

List<TextSpan> _runs(InlineSpan root) {
  final runs = <TextSpan>[];
  void walk(InlineSpan span) {
    if (span is! TextSpan) return;
    if (span.text != null && span.text!.isNotEmpty) runs.add(span);
    span.children?.forEach(walk);
  }

  walk(root);
  return runs;
}
