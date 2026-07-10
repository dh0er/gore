import 'package:flutter_test/flutter_test.dart';
import 'package:goresave/features/editor/domain/editor_models.dart';

/// Unit coverage for the `actorId` field on the inventory edit models: when set
/// it appears in the edit's value map (NPC inventory edit); when null the key is
/// omitted entirely so player inventory edits stay byte-for-byte unchanged.
void main() {
  group('InventoryItemCountChange.toEditJson', () {
    test('includes actorId when set (NPC edit)', () {
      final edit = const InventoryItemCountChange(
        id: 'item-1',
        path: 'MainContainer[3]',
        count: 7,
        actorId: 'Lizard-1',
      ).toEditJson();
      expect(edit['path'], 'private.inventory.setItemCount');
      final value = edit['value'] as Map<String, Object?>;
      expect(value['actorId'], 'Lizard-1');
      expect(value['id'], 'item-1');
      expect(value['path'], 'MainContainer[3]');
      expect(value['count'], 7);
    });

    test('omits actorId key when null (player edit unchanged)', () {
      final value = const InventoryItemCountChange(
        id: 'item-1',
        path: 'MainContainer[3]',
        count: 7,
      ).toEditJson()['value'] as Map<String, Object?>;
      expect(value.containsKey('actorId'), isFalse);
      expect(value, {'id': 'item-1', 'path': 'MainContainer[3]', 'count': 7});
    });

    test('echoes containerType + slotId when set (per-container NPC edit)', () {
      final value = const InventoryItemCountChange(
        id: 'item-1',
        path: 'Pouch[0]',
        count: 99,
        actorId: 'Lizard-1',
        slotId: 0,
        containerType: 'Pouch',
      ).toEditJson()['value'] as Map<String, Object?>;
      expect(value['containerType'], 'Pouch');
      expect(value['slotId'], 0);
    });

    test('omits containerType key when null (player edit unchanged)', () {
      final value = const InventoryItemCountChange(
        id: 'item-1',
        path: 'MainContainer[3]',
        count: 7,
      ).toEditJson()['value'] as Map<String, Object?>;
      expect(value.containsKey('containerType'), isFalse);
    });
  });

  group('InventoryItemAdd.toEditJson', () {
    test('includes actorId when set (NPC edit)', () {
      final value = const InventoryItemAdd(
        path: '/Game/Item.Item',
        count: 2,
        actorId: 'Lizard-1',
      ).toEditJson()['value'] as Map<String, Object?>;
      expect(value['actorId'], 'Lizard-1');
    });

    test('omits actorId key when null (player edit unchanged)', () {
      final value = const InventoryItemAdd(
        path: '/Game/Item.Item',
        count: 2,
      ).toEditJson()['value'] as Map<String, Object?>;
      expect(value.containsKey('actorId'), isFalse);
      expect(value, {'path': '/Game/Item.Item', 'count': 2});
    });
  });

  group('InventoryItemRemove.toEditJson', () {
    test('includes actorId when set (NPC edit)', () {
      final value = const InventoryItemRemove(
        path: 'MainContainer[3]',
        actorId: 'Lizard-1',
      ).toEditJson()['value'] as Map<String, Object?>;
      expect(value['actorId'], 'Lizard-1');
    });

    test('omits actorId key when null (player edit unchanged)', () {
      final value = const InventoryItemRemove(
        path: 'MainContainer[3]',
      ).toEditJson()['value'] as Map<String, Object?>;
      expect(value.containsKey('actorId'), isFalse);
      expect(value, {'path': 'MainContainer[3]'});
    });

    test('echoes containerType + slotId when set (per-container NPC remove)', () {
      final value = const InventoryItemRemove(
        path: 'MeleeSlot[1]',
        actorId: 'Lizard-1',
        slotId: 1,
        containerType: 'MeleeSlot',
      ).toEditJson()['value'] as Map<String, Object?>;
      expect(value['containerType'], 'MeleeSlot');
      expect(value['slotId'], 1);
      expect(value['actorId'], 'Lizard-1');
    });

    test('omits containerType/slotId keys when null (player remove unchanged)',
        () {
      final value = const InventoryItemRemove(
        path: 'MainContainer[3]',
      ).toEditJson()['value'] as Map<String, Object?>;
      expect(value.containsKey('containerType'), isFalse);
      expect(value.containsKey('slotId'), isFalse);
    });
  });

  group('PrivateInventoryItem.fromJson', () {
    test('parses containerType from the summary JSON', () {
      final item = PrivateInventoryItem.fromJson(const {
        'id': 'ItMw_Sword',
        'path': '/Script/Angelscript.ItMw_Sword',
        'count': 1,
        'slotId': 1,
        'containerType': 'MeleeSlot',
      });
      expect(item.containerType, 'MeleeSlot');
      expect(item.slotId, 1);
    });

    test('containerType is null when absent (older payloads)', () {
      final item = PrivateInventoryItem.fromJson(const {
        'id': 'ItFo_Apple',
        'path': '/Script/Angelscript.ItFo_Apple',
      });
      expect(item.containerType, isNull);
    });
  });
}
