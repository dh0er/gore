import 'package:flutter_test/flutter_test.dart';
import 'package:goresave/features/editor/domain/editor_notifier.dart';

/// The guard that keeps a structural inventory write from silently overwriting a
/// raw All-Data edit into the same slot must match exactly those edits — every
/// field below `m_Slots/[i]`, and nothing outside a slot, or unrelated work
/// would be blocked.
void main() {
  Map<String, Object?> typedEdit(List<String> path) => {
    'path': 'private.typed.setValue',
    'value': {'path': path, 'value': 7},
  };

  test('matches every field of a player inventory slot', () {
    for (final leaf in [
      ['m_Id'],
      ['m_SlotData', 'm_ItemCount'],
      ['m_Payload', 'm_StageLevel'],
      <String>[],
    ]) {
      expect(
        isInventorySlotTypedEdit(
          typedEdit([
            'm_SavedPlayers',
            '[0]',
            'm_Inventory',
            'm_Slots',
            '[3]',
            ...leaf,
          ]),
        ),
        isTrue,
        reason: 'leaf: $leaf',
      );
    }
  });

  test('matches a container operation inside a slot, not just setValue', () {
    // setAdd/setRemove and the array ops address their target the same way, and
    // a structural inventory write would overwrite them just the same.
    for (final op in [
      'private.typed.setAdd',
      'private.typed.setRemove',
      'private.typed.arrayRemove',
      'private.typed.arrayDuplicate',
    ]) {
      expect(
        isInventorySlotTypedEdit({
          'path': op,
          'value': {
            'path': [
              'm_SavedPlayers',
              '[0]',
              'm_Inventory',
              'm_Slots',
              '[3]',
              'm_Payload',
              'm_GenericData',
            ],
            'index': 0,
          },
        }),
        isTrue,
        reason: op,
      );
    }
  });

  test('matches an NPC slot edit, which sits under no m_Inventory segment', () {
    expect(
      isInventorySlotTypedEdit(
        typedEdit([
          'm_GenericData',
          '{CharacterStates}',
          'NPCCharacters',
          'InventoryByGlobalId',
          '{Lizard-A}',
          'InventoryItems',
          'm_Values',
          'Items',
          '[6]',
          'm_Slots',
          '[3]',
          'm_SlotData',
          'm_ItemCount',
        ]),
      ),
      isTrue,
    );
  });

  test('ignores anything that is not inside a slot', () {
    // m_Slots itself, with no slot picked out.
    expect(
      isInventorySlotTypedEdit(
        typedEdit(['m_SavedPlayers', '[0]', 'm_Inventory', 'm_Slots']),
      ),
      isFalse,
    );
    expect(
      isInventorySlotTypedEdit(typedEdit(['m_Something', 'm_Id'])),
      isFalse,
    );
    expect(
      isInventorySlotTypedEdit(typedEdit(['m_Inventory', 'm_Values', 'm_Id'])),
      isFalse,
    );
    expect(
      isInventorySlotTypedEdit({
        'path': 'private.inventory.addItem',
        'value': {},
      }),
      isFalse,
    );
  });
}
