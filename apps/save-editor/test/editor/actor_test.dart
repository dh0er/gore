import 'package:flutter_test/flutter_test.dart';
import 'package:goresave/features/editor/domain/actor.dart';

void main() {
  test('uniqueName is carried but excluded from equality', () {
    const a = Actor.npc(id: 'NC_ORG_Lares_801-WP_X', name: 'Lares', uniqueName: 'NC_ORG_Lares_801');
    const b = Actor.npc(id: 'NC_ORG_Lares_801-WP_X', name: 'Lares', uniqueName: 'different');
    expect(a, equals(b)); // identity is (kind, id) only
    expect(a.uniqueName, 'NC_ORG_Lares_801');
    expect(const Actor.player().uniqueName, 'Hero');
  });
}
