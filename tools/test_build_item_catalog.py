import json
from pathlib import Path

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
"""


def test_parse_dump_classes_dedupes():
    names = parse_dump_classes(FIXTURE.splitlines())
    assert names.count("ItMi_Orenugget") == 1
    assert "SomethingElse" not in names  # only It* candidates


def test_build_catalog_filters_and_categorizes():
    entries, skipped = build_catalog(parse_dump_classes(FIXTURE.splitlines()))
    by_id = {e["id"]: e for e in entries}
    assert by_id["ItMi_Orenugget"] == {
        "id": "ItMi_Orenugget",
        "path": "/Script/Angelscript.ItMi_Orenugget",
        "category": "misc",
    }
    assert by_id["ItAr_Rune_FireBall"]["category"] == "rune"
    assert by_id["ItAr_Scroll_Charm"]["category"] == "scroll"
    assert by_id["ItKeyDefault"]["category"] == "key"
    assert by_id["ItIg_Worldsplitter"]["category"] == "special"
    # excluded entirely:
    assert "ItAr_Rune_FireBall_Base" not in by_id  # _Base suffix
    assert "ItemAnimConfig_Meatbug" not in by_id   # config class
    assert "ItAI_Plank" not in by_id               # AI prop
    # exclusions are reported, not silent:
    assert "ItAr_Rune_FireBall_Base" in skipped
    assert "ItemAnimConfig_Meatbug" in skipped


def test_output_sorted_and_stable():
    entries, _ = build_catalog(parse_dump_classes(FIXTURE.splitlines()))
    ids = [e["id"] for e in entries]
    assert ids == sorted(ids)
