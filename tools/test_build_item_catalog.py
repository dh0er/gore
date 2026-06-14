import unittest

from build_item_catalog import build_catalog, parse_dump_classes

FIXTURE = """\
[0000025A88701900] ASClass /Script/Angelscript.ItemAnimConfig_Meatbug [n: 1] [c: 2] [or: 3]
[0000025A88701901] ASClass /Script/Angelscript.ItMi_Orenugget [n: 1] [c: 2] [or: 3]
[0000025A88701902] ASClass /Script/Angelscript.ItMi_Orenugget [n: 1] [c: 2] [or: 3]
[0000025A88701903] ASClass /Script/Angelscript.ItAr_Rune_FireBall_Base [n: 1] [c: 2] [or: 3]
[0000025A88701904] ASClass /Script/Angelscript.ItAr_Rune_FireBall [n: 1] [c: 2] [or: 3]
[0000025A88701905] ASClass /Script/Angelscript.ItAr_Scroll_Charm [n: 1] [c: 2] [or: 3]
[0000025A88701906] ASClass /Script/Angelscript.ItAI_Plank [n: 1] [c: 2] [or: 3]
[0000025A88701907] ASClass /Script/Angelscript.ItKeyDefault [n: 1] [c: 2] [or: 3]
[0000025A88701908] ASClass /Script/Angelscript.ItIg_Worldsplitter [n: 1] [c: 2] [or: 3]
[0000025A88701909] ASClass /Script/Angelscript.SomethingElse [n: 1] [c: 2] [or: 3]
[0000025A8870190A] ASClass /Script/Angelscript.ItAm_Arrow [n: 1] [c: 2] [or: 3]
[0000025A8870190B] ASClass /Script/Angelscript.ItAt_Amulet_OfDeath [n: 1] [c: 2] [or: 3]
[0000025A8870190C] ASClass /Script/Angelscript.ItAt_Ring_OfLife [n: 1] [c: 2] [or: 3]
[0000025A8870190D] ASClass /Script/Angelscript.ItAt_Wolf_Fur [n: 1] [c: 2] [or: 3]
"""


class BuildItemCatalogTest(unittest.TestCase):
    def test_parse_dump_classes_dedupes(self):
        names = parse_dump_classes(FIXTURE.splitlines())
        self.assertEqual(names.count("ItMi_Orenugget"), 1)
        self.assertNotIn("SomethingElse", names)  # only It* candidates

    def test_build_catalog_filters_and_categorizes(self):
        entries, skipped = build_catalog(parse_dump_classes(FIXTURE.splitlines()))
        by_id = {e["id"]: e for e in entries}
        self.assertEqual(
            by_id["ItMi_Orenugget"],
            {
                "id": "ItMi_Orenugget",
                "path": "/Script/Angelscript.ItMi_Orenugget",
                "category": "misc",
            },
        )
        self.assertEqual(by_id["ItAr_Rune_FireBall"]["category"], "rune")
        self.assertEqual(by_id["ItAr_Scroll_Charm"]["category"], "scroll")
        self.assertEqual(by_id["ItKeyDefault"]["category"], "key")
        self.assertEqual(by_id["ItIg_Worldsplitter"]["category"], "special")
        self.assertEqual(by_id["ItAm_Arrow"]["category"], "ammunition")
        self.assertEqual(by_id["ItAt_Amulet_OfDeath"]["category"], "amulet")
        self.assertEqual(by_id["ItAt_Ring_OfLife"]["category"], "ring")
        self.assertEqual(by_id["ItAt_Wolf_Fur"]["category"], "trophy")
        # excluded entirely:
        self.assertNotIn("ItAr_Rune_FireBall_Base", by_id)  # _Base suffix
        self.assertNotIn("ItemAnimConfig_Meatbug", by_id)  # config class
        self.assertNotIn("ItAI_Plank", by_id)  # AI prop
        # exclusions are reported, not silent:
        self.assertIn("ItAr_Rune_FireBall_Base", skipped)
        self.assertIn("ItemAnimConfig_Meatbug", skipped)

    def test_output_sorted_and_stable(self):
        entries, _ = build_catalog(parse_dump_classes(FIXTURE.splitlines()))
        ids = [e["id"] for e in entries]
        self.assertEqual(ids, sorted(ids))


if __name__ == "__main__":
    unittest.main()
