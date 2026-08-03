import 'package:flutter_test/flutter_test.dart';
import 'package:goresave/features/editor/domain/editor_notifier.dart';

/// The guard that keeps a queued slot repair from silently overwriting a raw
/// All-Data edit of the same `m_Id` must match exactly that edit — not every
/// inventory edit, or unrelated work would be blocked.
void main() {
  Map<String, Object?> typedEdit(List<String> path) => {
    'path': 'private.typed.setValue',
    'value': {'path': path, 'value': 7},
  };

  test('matches an inventory slot id edit', () {
    expect(
      isSlotIdTypedEdit(
        typedEdit([
          'm_SavedPlayers',
          '[0]',
          'm_Inventory',
          'm_Slots',
          '[3]',
          'm_Id',
        ]),
      ),
      isTrue,
    );
  });

  test('ignores other inventory fields and non-inventory paths', () {
    expect(
      isSlotIdTypedEdit(
        typedEdit([
          'm_SavedPlayers',
          '[0]',
          'm_Inventory',
          'm_Slots',
          '[3]',
          'm_ItemCount',
        ]),
      ),
      isFalse,
    );
    expect(isSlotIdTypedEdit(typedEdit(['m_Something', 'm_Id'])), isFalse);
    expect(
      isSlotIdTypedEdit({'path': 'private.inventory.addItem', 'value': {}}),
      isFalse,
    );
  });
}
