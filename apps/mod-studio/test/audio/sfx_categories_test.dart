import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/audio/domain/sfx_categories.dart';

void main() {
  test('categorizes by second token, case-folded, with merges', () {
    expect(sfxCategoryForSample('SFX_CREA_Golem_Ice_M_Creak_Loop_L1_01'), SfxCategory.creatures);
    expect(sfxCategoryForSample('SFX_OBJ_GolemAltar_OrbLoop_L1_01'), SfxCategory.objects);
    expect(sfxCategoryForSample('SFX_Objects_Chest_Open_01'), SfxCategory.objects);
    expect(sfxCategoryForSample('SFX_Magic_Fear_Cast_L9_01'), SfxCategory.magic);
    expect(sfxCategoryForSample('SFX_MAGIC_Impact_01'), SfxCategory.magic);
    expect(sfxCategoryForSample('SFX_MOVE_Footsteps_Human_Grass_Walk_07'), SfxCategory.movement);
    expect(sfxCategoryForSample('SFX_WORLD_Lava_AMB_02'), SfxCategory.world);
    expect(sfxCategoryForSample('SFX_ACTION_Sweat_Swipe_L1_05'), SfxCategory.action);
    expect(sfxCategoryForSample('SFX_ACTIONS_Foo'), SfxCategory.action);
    expect(sfxCategoryForSample('SFX_COMBAT_Ranged_Bow_Draw_04'), SfxCategory.combat);
    expect(sfxCategoryForSample('SFX_UI_X'), SfxCategory.ui);
    expect(sfxCategoryForSample('taiko_hit'), SfxCategory.other);
    expect(sfxCategoryForSample('SFX'), SfxCategory.other);
  });
}
