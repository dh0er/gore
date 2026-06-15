import build_npc_catalog as m

DUMP = [
    "[0001] ASClass /Script/Angelscript.CharacterDefinition_Human_OC_STT_Diego [n: A]",
    "[0002] ASClass /Script/Angelscript.CharacterDefinition_Human_NC_SLD_Gorn_699 [n: B]",
    "[0003] ASClass /Script/Angelscript.CharacterDefinition_Creature_Biter [n: C]",
    "[0004] ASClass /Script/Angelscript.CharacterDefinition_Human_OC_STT_Diego [n: D]",  # dup
    "[0005] ASClass /Script/Angelscript.ItMw_Sword01 [n: E]",  # ignored
]

def test_human_unique_name_is_map_key_form():
    entries, _ = m.build_catalog(m.parse_dump_classes(DUMP))
    by_id = {e["id"]: e for e in entries}
    assert by_id["OC_STT_Diego"]["category"] == "human"
    assert by_id["OC_STT_Diego"]["class"] == "CharacterDefinition_Human_OC_STT_Diego"

def test_non_human_kept_and_flagged():
    entries, _ = m.build_catalog(m.parse_dump_classes(DUMP))
    by_id = {e["id"]: e for e in entries}
    assert by_id["Creature_Biter"]["category"] == "creature"

def test_dedup_and_sorted():
    entries, _ = m.build_catalog(m.parse_dump_classes(DUMP))
    ids = [e["id"] for e in entries]
    assert ids == sorted(ids)
    assert ids.count("OC_STT_Diego") == 1

def test_ignores_non_character_classes():
    entries, _ = m.build_catalog(m.parse_dump_classes(DUMP))
    assert all("Sword" not in e["id"] for e in entries)
