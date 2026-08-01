import 'package:flutter_test/flutter_test.dart';
import 'package:goresave/features/editor/domain/actor.dart';
import 'package:goresave/features/editor/domain/core_service.dart';
import 'package:goresave/features/editor/domain/editor_notifier.dart';

/// The Attribute and Position sub-tabs are both `_KeepAliveTab`s, so both stay
/// live once visited and each can hold an invalid field at the same time.
///
/// `setNpcEditInvalid` is the Attribute panel's API and clears
/// `invalidNpcEditKey` plus every `npc.attributes:` key on every call. If the
/// Position panel used it too, one panel going valid would unblock the global
/// Save button while the other still held a bad field. The Position panel
/// therefore keys itself through `setEditInvalid` (position_detail.dart), and
/// `invalidNpcEditKey`'s legacy fallback skips `npc.position:` keys so the
/// clearing does not travel the other way either.
void main() {
  const npc = Actor.npc(id: 'A', name: 'A', uniqueName: 'A');
  const otherNpc = Actor.npc(id: 'B', name: 'B', uniqueName: 'B');

  EditorNotifier makeNotifier() =>
      EditorNotifier(_StubCoreService(), saveDir: r'C:\tmp\saves');

  test('position going valid leaves an invalid attribute field blocking Save', () {
    final notifier = makeNotifier();

    notifier.setNpcEditInvalid('npc.attributes:A');
    notifier.setEditInvalid('npc.position:A', invalid: true);
    expect(notifier.state.hasInvalidEdits, isTrue);

    // The Position panel recovers; the Attribute panel has not.
    notifier.setEditInvalid('npc.position:A', invalid: false);

    expect(
      notifier.state.invalidEditKeys,
      contains('npc.attributes:A'),
      reason: 'the attribute block must survive the position panel recovering',
    );
    expect(notifier.state.hasInvalidEdits, isTrue);
  });

  test('attributes going valid leaves an invalid position field blocking Save', () {
    final notifier = makeNotifier();

    notifier.setEditInvalid('npc.position:A', invalid: true);
    notifier.setNpcEditInvalid('npc.attributes:A');
    expect(notifier.state.hasInvalidEdits, isTrue);

    // The Attribute panel recovers; the Position panel has not. This is the
    // direction that also exercises the `invalidNpcEditKey` legacy fallback:
    // were `npc.position:A` returned there, the `..remove()` inside
    // setNpcEditInvalid would drop it as a side effect.
    notifier.setNpcEditInvalid(null);

    expect(
      notifier.state.invalidEditKeys,
      contains('npc.position:A'),
      reason: 'the position block must survive the attribute panel recovering',
    );
    expect(notifier.state.hasInvalidEdits, isTrue);
  });

  test('switching actor drops a stale position block', () {
    final notifier = makeNotifier();

    notifier.selectActor(npc);
    notifier.setEditInvalid('npc.position:A', invalid: true);
    expect(notifier.state.hasInvalidEdits, isTrue);

    notifier.selectActor(otherNpc);

    expect(
      notifier.state.invalidEditKeys,
      isNot(contains('npc.position:A')),
      reason: 'an abandoned NPC\'s invalid field must not disable Save for the '
          'next NPC, whose own fields are all valid',
    );
    expect(notifier.state.hasInvalidEdits, isFalse);
  });

  test('a non-NPC validation key is untouched by either NPC surface', () {
    final notifier = makeNotifier();

    notifier.setStoryStateEditInvalid(true);
    notifier.setEditInvalid('npc.position:A', invalid: true);
    notifier.setNpcEditInvalid('npc.attributes:A');

    notifier.setEditInvalid('npc.position:A', invalid: false);
    notifier.setNpcEditInvalid(null);
    notifier.selectActor(otherNpc);

    expect(notifier.state.hasInvalidEdits, isTrue);
  });
}

/// Minimal core stub: every command fails fast so the constructor's
/// refresh()/checkCodec() do nothing observable. Validation keys are pure UI
/// state and never touch the core.
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
