import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:goresave/features/app/domain/ui_settings.dart';
import 'package:goresave/features/app/ui/ui_scale_root.dart';
import 'package:goresave/features/app/ui/update_banner.dart';
import 'package:goresave/providers/data_providers.dart';
import 'package:goresave/ui/design/app_theme.dart';

class GoresaveApp extends ConsumerWidget {
  const GoresaveApp({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final router = ref.watch(routerProvider).router;
    final themeMode = ref.watch(themeModeProvider);
    return MaterialApp.router(
      title: 'Gothic Remake Savegame Editor',
      debugShowCheckedModeBanner: false,
      theme: buildGoresaveTheme(),
      darkTheme: buildGoresaveDarkTheme(),
      themeMode: themeMode,
      builder: (context, child) => UiScaleRoot(
        child: UpdateBannerHost(child: child ?? const SizedBox.shrink()),
      ),
      routerConfig: router,
    );
  }
}
