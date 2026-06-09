import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:goresave/providers/data_providers.dart';
import 'package:goresave/ui/design/app_theme.dart';

class GoresaveApp extends ConsumerWidget {
  const GoresaveApp({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final router = ref.watch(routerProvider).router;
    return MaterialApp.router(
      title: 'Gothic Remake Savegame Editor',
      debugShowCheckedModeBanner: false,
      theme: buildGoresaveTheme(),
      routerConfig: router,
    );
  }
}
