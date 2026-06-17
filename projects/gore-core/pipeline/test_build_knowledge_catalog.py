import unittest

from build_knowledge_catalog import build_catalog, parse_dump_classes

DUMP = [
    "[1] ASClass /Script/Angelscript.Topic_Diego_209799 [n: A]",
    "[2] ASClass /Script/Angelscript.Info_FMORGAreyouok [n: B]",
    "[3] ASClass /Script/Angelscript.ChoiceDiegoGamestart [n: C]",
    "[4] ASClass /Script/Angelscript.Topic_Diego_209799 [n: D]",  # dup
    "[5] ASClass /Script/Angelscript.ItMw_Sword01 [n: E]",  # ignored
    "[6] ASClass /Script/Angelscript.CharacterDefinition_Human_OC_STT_Diego [n: F]",  # ignored
]


class BuildKnowledgeCatalogTest(unittest.TestCase):
    def test_categories(self):
        entries = build_catalog(parse_dump_classes(DUMP))
        by_id = {e["id"]: e for e in entries}
        self.assertEqual(by_id["Topic_Diego_209799"]["category"], "topic")
        self.assertEqual(by_id["Info_FMORGAreyouok"]["category"], "info")
        self.assertEqual(by_id["ChoiceDiegoGamestart"]["category"], "choice")

    def test_dedup_sorted_and_filtered(self):
        entries = build_catalog(parse_dump_classes(DUMP))
        ids = [e["id"] for e in entries]
        self.assertEqual(ids, sorted(ids))
        self.assertEqual(ids.count("Topic_Diego_209799"), 1)
        self.assertTrue(
            all("Sword" not in i and "CharacterDefinition" not in i for i in ids)
        )


if __name__ == "__main__":
    unittest.main()
