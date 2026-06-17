import unittest

from build_npc_catalog import build_catalog, parse_dump_classes

DUMP = [
    "[0001] ASClass /Script/Angelscript.CharacterDefinition_Human_OC_STT_Diego [n: A]",
    "[0002] ASClass /Script/Angelscript.CharacterDefinition_Human_NC_SLD_Gorn_699 [n: B]",
    "[0003] ASClass /Script/Angelscript.CharacterDefinition_Creature_Biter [n: C]",
    "[0004] ASClass /Script/Angelscript.CharacterDefinition_Human_OC_STT_Diego [n: D]",  # dup
    "[0005] ASClass /Script/Angelscript.ItMw_Sword01 [n: E]",  # ignored
]


class BuildNpcCatalogTest(unittest.TestCase):
    def test_human_unique_name_is_map_key_form(self):
        entries, _ = build_catalog(parse_dump_classes(DUMP))
        by_id = {e["id"]: e for e in entries}
        self.assertEqual(by_id["OC_STT_Diego"]["category"], "human")
        self.assertEqual(
            by_id["OC_STT_Diego"]["class"],
            "CharacterDefinition_Human_OC_STT_Diego",
        )

    def test_creature_category(self):
        entries, _ = build_catalog(parse_dump_classes(DUMP))
        by_id = {e["id"]: e for e in entries}
        self.assertEqual(by_id["Creature_Biter"]["category"], "creature")

    def test_dedup_and_sorted(self):
        entries, _ = build_catalog(parse_dump_classes(DUMP))
        ids = [e["id"] for e in entries]
        self.assertEqual(ids, sorted(ids))
        self.assertEqual(ids.count("OC_STT_Diego"), 1)

    def test_ignores_non_character_classes(self):
        entries, _ = build_catalog(parse_dump_classes(DUMP))
        self.assertTrue(all("Sword" not in e["id"] for e in entries))


if __name__ == "__main__":
    unittest.main()
