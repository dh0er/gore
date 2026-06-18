import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'app/domain/ui_settings.dart';
import 'app/ui/app_theme.dart';
import 'home_page.dart';

class GoreModApp extends ConsumerWidget {
  const GoreModApp({super.key});
  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final themeMode = ref.watch(themeModeProvider);
    return MaterialApp(
      title: 'gore-mod',
      themeMode: themeMode,
      theme: buildGoreModTheme(),
      darkTheme: buildGoreModDarkTheme(),
      home: const HomePage(),
    );
  }
}
