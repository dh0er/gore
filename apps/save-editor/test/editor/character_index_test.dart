import 'package:flutter_test/flutter_test.dart';
import 'package:goresave/features/editor/domain/character_index.dart';

void main() {
  test('CharacterRow parses flags and null globalId', () {
    final orphan = CharacterRow.fromJson({
      'globalId': null, 'uniqueName': 'ST_VLK_Mud_Sleeper',
      'isDead': false, 'hasInventory': false, 'hasKnowledge': true, 'hasEvents': false,
    });
    expect(orphan.globalId, isNull);
    expect(orphan.isOrphan, isTrue);
    expect(orphan.hasKnowledge, isTrue);

    final actor = CharacterRow.fromJson({
      'globalId': 'NC_ORG_Lares_801-WP_X', 'uniqueName': 'NC_ORG_Lares_801',
      'isDead': false, 'hasInventory': true, 'hasKnowledge': true, 'hasEvents': true,
    });
    expect(actor.isOrphan, isFalse);
    expect(actor.hasEvents, isTrue);
  });
}
