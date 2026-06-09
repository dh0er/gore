import 'package:go_router/go_router.dart';
import 'package:goresave/features/editor/ui/editor_page.dart';

class GoresaveRouter {
  GoresaveRouter()
    : router = GoRouter(
        initialLocation: '/',
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
