import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'app/domain/ui_settings.dart';
import 'home_page.dart';

class GoreModApp extends ConsumerWidget {
  const GoreModApp({super.key});
  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final themeMode = ref.watch(themeModeProvider);
    return MaterialApp(
      title: 'gore-mod',
      themeMode: themeMode,
      theme: ThemeData(colorSchemeSeed: const Color(0xFF8B2500), useMaterial3: true),
      darkTheme: ThemeData(
        colorSchemeSeed: const Color(0xFF8B2500),
        brightness: Brightness.dark,
        useMaterial3: true,
      ),
      home: const HomePage(),
    );
  }
}
