import 'package:go_router/go_router.dart';
import 'package:goresave/features/app/domain/desktop_updater.dart';
import 'package:goresave/features/editor/ui/editor_page.dart';

class GoresaveRouter {
  GoresaveRouter()
    : router = GoRouter(
        initialLocation: '/',
        // Shared with the updater so a background update check can show a
        // dialog without a widget context of its own.
        navigatorKey: updaterNavigatorKey,
        routes: [
          GoRoute(
            path: '/',
            name: 'editor',
            builder: (context, state) => const EditorPage(),
          ),
        ],
      );

  final GoRouter router;
}
