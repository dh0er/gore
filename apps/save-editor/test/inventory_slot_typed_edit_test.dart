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

  group('the narrower id predicate the repair uses', () {
    // The repair only rewrites ids, and only after everything else has run, so
    // it collides with an edit of a slot's m_Id and with nothing else.
    List<String> slotPath(List<String> leaf) => [
      'm_SavedPlayers',
      '[0]',
      'm_Inventory',
      'm_Slots',
      '[3]',
      ...leaf,
    ];

    test('matches a slot id edit', () {
      expect(isInventorySlotIdTypedEdit(typedEdit(slotPath(['m_Id']))), isTrue);
    });

    test('leaves the other slot fields alone', () {
      for (final leaf in [
        ['m_SlotData', 'm_ItemCount'],
        ['m_Payload', 'm_StageLevel'],
        <String>[],
      ]) {
        expect(
          isInventorySlotIdTypedEdit(typedEdit(slotPath(leaf))),
          isFalse,
          reason: 'leaf: $leaf',
        );
        // The wider predicate, which add and remove use, still claims them.
        expect(isInventorySlotTypedEdit(typedEdit(slotPath(leaf))), isTrue);
      }
    });

    test('ignores an m_Id that is not a slot id', () {
      expect(
        isInventorySlotIdTypedEdit(
          typedEdit(['m_Inventory', 'm_Values', 'm_Id']),
        ),
        isFalse,
      );
    });
  });

  test('matches an array operation on the slot array itself', () {
    // arrayRemove/arrayDuplicate address the array and name their element in
    // value.index, so the path just ends at m_Slots. Such a splice deletes or
    // duplicates a whole slot — including one an add just filled.
    for (final op in [
      'private.typed.arrayRemove',
      'private.typed.arrayDuplicate',
    ]) {
      expect(
        isInventorySlotTypedEdit({
          'path': op,
          'value': {
            'path': ['m_SavedPlayers', '[0]', 'm_Inventory', 'm_Slots'],
            'index': 3,
          },
        }),
        isTrue,
        reason: op,
      );
    }
    // The repair renumbers last, so it survives such a splice and stays allowed.
    expect(
      isInventorySlotIdTypedEdit({
        'path': 'private.typed.arrayRemove',
        'value': {
          'path': ['m_SavedPlayers', '[0]', 'm_Inventory', 'm_Slots'],
          'index': 3,
        },
      }),
      isFalse,
    );
  });

  test('ignores anything that is not inside a slot', () {
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
