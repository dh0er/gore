import 'package:flutter_test/flutter_test.dart';
import 'package:goresave/features/editor/domain/actor.dart';
import 'package:goresave/features/editor/domain/core_service.dart';
import 'package:goresave/features/editor/domain/editor_notifier.dart';

void main() {
  test('default selected actor is player', () {
    final notifier = EditorNotifier(_StubCoreService(), saveDir: r'C:\tmp\saves');

    expect(notifier.state.selectedActor.isPlayer, isTrue);
  });

  test('selecting an actor updates shared state and notifies', () {
    final notifier = EditorNotifier(_StubCoreService(), saveDir: r'C:\tmp\saves');

    var notified = false;
    final removeListener = notifier.addListener((_) {
      notified = true;
    });
    // addListener fires once synchronously with the current state; clear that.
    notified = false;

    notifier.selectActor(
      const Actor.npc(id: 'Lizard-1', name: 'Lizard', uniqueName: 'Lizard'),
    );

    expect(notifier.state.selectedActor.kind, ActorKind.npc);
    expect(notifier.state.selectedActor.id, 'Lizard-1');
    expect(notifier.state.selectedActor.name, 'Lizard');
    expect(notifier.state.selectedActor.isPlayer, isFalse);
    expect(notified, isTrue);

    removeListener();
  });
}

/// Minimal core stub: every command fails fast so the constructor's
/// refresh()/checkCodec() do nothing observable. The selectedActor field is
/// pure UI state and never touches the core.
class _StubCoreService implements GoresaveCoreService {
  @override
  bool get isAvailable => false;

  @override
  String get description => 'stub';

  @override
  Future<Map<String, Object?>> execute(
    String command, {
    Map<String, Object?> payload = const {},
  }) async {
    return {
      'ok': false,
      'error': {'message': 'stub'},
    };
  }
}
