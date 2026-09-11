#!/usr/bin/env python3
"""Read the NPC session fixture from a user-created save through gore-save's DLL.

Example:
  python scripts/npc_session_proof.py --save G1R-003.sav --dll gore_save.dll \
    --phase running --output running.json
  python scripts/npc_session_proof.py --save G1R-003.sav --dll gore_save.dll \
    --phase completed --previous running.json --output completed.json

Only fixed read commands are sent to the core. Reports establish saved state;
a separate game process/session must still be confirmed by the person testing.
"""
from __future__ import annotations

import argparse
import ctypes
import hashlib
import json
import re
import sys
from datetime import datetime, timezone
from pathlib import Path


ACTORS = {
    "GORE_TEST_A": "GORE_TEST_A-WP_WarningFlock_XT_01",
    "GORE_TEST_B": "GORE_TEST_B-WP_WarningFlock_XT_02",
}
STARTED = "gore_npc_session_started"
COMPLETED = "gore_npc_session_completed"
QUEST = "/Script/Angelscript.Quest_GORE_NPC_SESSION"
STORY = "/Script/Angelscript.ActivePersonalRelationshipModifier_Story"
FRIEND = "ERelationship::Friend"
READ_COMMANDS = frozenset({
    "private.characters.list", "private.npc.attributes", "private.npc.inventory",
    "private.npc.position", "query_progression", "search_typed_properties",
})


class EvidenceError(Exception):
    pass


def fingerprint(path: Path) -> dict:
    digest = hashlib.sha256()
    size = 0
    with path.open("rb") as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(block)
            size += len(block)
    return {"path": str(path.resolve()), "sha256": digest.hexdigest(), "bytes": size}


class SaveReader:
    def __init__(self, dll: Path, save: Path):
        self.save = str(save.resolve())
        self.requests: list[dict] = []
        self.library = ctypes.CDLL(str(dll.resolve()))
        self.library.goresave_execute.argtypes = [ctypes.c_char_p]
        self.library.goresave_execute.restype = ctypes.c_void_p
        self.library.goresave_free.argtypes = [ctypes.c_void_p]
        self.library.goresave_free.restype = None

    def read(self, command: str, **payload) -> dict:
        if command not in READ_COMMANDS or "path" in payload:
            raise EvidenceError("only fixed read commands against the selected save are allowed")
        request = {"command": command, "payload": {"path": self.save, **payload}}
        pointer = self.library.goresave_execute(json.dumps(request).encode("utf-8"))
        if not pointer:
            raise EvidenceError(f"{command}: core returned a null pointer")
        try:
            response = json.loads(ctypes.string_at(pointer))
        finally:
            self.library.goresave_free(pointer)
        self.requests.append({"request": request, "response": response})
        if not isinstance(response, dict) or response.get("ok") is not True:
            raise EvidenceError(f"{command}: {response}")
        if not isinstance(response.get("data"), dict):
            raise EvidenceError(f"{command}: missing data object")
        return response["data"]


def rows(data: dict, key: str) -> list:
    value = data.get(key)
    if not isinstance(value, list):
        raise EvidenceError(f"missing {key} array")
    return value


def paged(reader, command: str, key: str, **payload) -> list:
    result = []
    expected_total = None
    while True:
        data = reader.read(command, offset=len(result), limit=1000, **payload)
        page = rows(data, key)
        total = data.get("total")
        if type(total) is not int or total < 0 or data.get("count") != len(page):
            raise EvidenceError(f"{command}: invalid pagination")
        if expected_total is not None and total != expected_total:
            raise EvidenceError(f"{command}: result changed between pages")
        if data.get("offset") != len(result) or len(result) + len(page) > total:
            raise EvidenceError(f"{command}: inconsistent page offset/count")
        if data.get("warnings"):
            raise EvidenceError(f"{command}: incomplete typed evidence: {data['warnings']}")
        expected_total = total
        result.extend(page)
        if len(result) == total:
            return result
        if not page:
            raise EvidenceError(f"{command}: pagination ended before total")


def character(reader, unique_name: str, expected_global: str | None = None) -> dict:
    data = reader.read("private.characters.list", query=unique_name)
    candidates = [row for row in rows(data, "characters")
                  if isinstance(row, dict)
                  and str(row.get("uniqueName", "")).casefold() == unique_name.casefold()
                  and isinstance(row.get("globalId"), str)]
    if len(candidates) != 1:
        raise EvidenceError(f"{unique_name}: expected one saved actor, found {len(candidates)}")
    row = candidates[0]
    if expected_global is not None and row["globalId"] != expected_global:
        raise EvidenceError(f"{unique_name}: GlobalId {row['globalId']!r}, expected {expected_global!r}")
    if type(row.get("hasKnowledge")) is not bool:
        raise EvidenceError(f"{unique_name}: missing hasKnowledge flag")
    return row


def knowledge(reader, actor: dict) -> list[str]:
    if not actor["hasKnowledge"]:
        return []
    # The core's knowledge lookup is case-sensitive; use the stored map spelling.
    entries = paged(reader, "query_progression", "entries", section="knowledge",
                    character=actor["uniqueName"])
    if not all(isinstance(entry, str) for entry in entries):
        raise EvidenceError("non-string knowledge entry")
    return entries


def relationship_modifiers(nodes: list[dict], global_id: str) -> list[dict]:
    indexed = {}
    for node in nodes:
        path = node.get("path")
        if not isinstance(path, list) or not all(isinstance(part, str) for part in path):
            raise EvidenceError("relationship node has no typed path")
        path = tuple(path)
        if path in indexed:
            raise EvidenceError("ambiguous duplicate relationship node path")
        indexed[path] = node
    modifiers = []
    for path, node in indexed.items():
        if (node.get("kind") != "objectInstance" or node.get("source") != "private"
                or len(path) < 4 or path[-4:-1] != (
                    "RelationshipByGlobalId", "{" + global_id + "}",
                    "ActivePersonalRelationshipModifiers")
                or re.fullmatch(r"\[\d+\]", path[-1]) is None):
            continue
        target = indexed.get(path + ("TargetCharacterGlobalID",), {}).get("value")
        relation = indexed.get(path + ("Relationship",), {}).get("value")
        modifiers.append({"path": list(path), "class": str(node.get("value", "")).split(" · ", 1)[0],
                          "target": target, "relationship": relation})
    return modifiers


def snapshot(reader, phase: str, expect_diego_hp1234: bool = False) -> dict:
    evidence = {"actors": {}, "controls": {}, "quest": [], "modifiers": []}
    checks = []
    errors = []

    def check(name, passed, observed):
        checks.append({"name": name, "passed": bool(passed), "observed": observed})

    for name, global_id in {**ACTORS, "Hero": "Hero", "OC_STT_Diego": None}.items():
        try:
            actor = character(reader, name, global_id)
            item = {"character": actor, "knowledge": knowledge(reader, actor)}
            markers = {entry.casefold() for entry in item["knowledge"]}
            if name in ACTORS:
                evidence["actors"][name] = item
                item["attributes"] = rows(reader.read("private.npc.attributes", id=actor["globalId"]), "attributes")
                item["inventory"] = reader.read("private.npc.inventory", id=actor["globalId"])
                item["position"] = reader.read("private.npc.position", id=actor["globalId"])
                check(f"{name}.inventory_present", actor.get("hasInventory") is True,
                      actor.get("hasInventory"))
                check(f"{name}.attributes_present", bool(item["attributes"]), len(item["attributes"]))
            else:
                evidence["controls"][name] = item
            if name == "GORE_TEST_A":
                check("A.started", STARTED in markers, item["knowledge"])
                check("A.completed", (COMPLETED in markers) == (phase == "completed"), item["knowledge"])
                check("A.relationship_summary", actor.get("personalRelationship") == "Friend",
                      actor.get("personalRelationship"))
            else:
                check(f"{name}.no_fixture_knowledge", not markers.intersection({STARTED, COMPLETED}),
                      sorted(markers.intersection({STARTED, COMPLETED})))
            if name == "OC_STT_Diego" and expect_diego_hp1234:
                item["attributes"] = rows(reader.read("private.npc.attributes", id=actor["globalId"]), "attributes")
                for key in ("Health", "MaxHealth"):
                    attributes = [row for row in item["attributes"] if row.get("key") == key]
                    check(f"Diego.{key}1234", len(attributes) == 1
                          and attributes[0].get("base") == 1234
                          and attributes[0].get("current") == 1234, attributes)
        except (EvidenceError, KeyError, TypeError) as error:
            errors.append(f"{name}: {error}")

    try:
        quests = paged(reader, "query_progression", "quests", section="quests", query="GORE_NPC_SESSION")
        evidence["quest"] = quests
        exact = [row for row in quests if row.get("questClass") == QUEST]
        expected = "EQuestState::Succeeded" if phase == "completed" else "EQuestState::Running"
        check("quest.state", len(exact) == 1 and exact[0].get("currentState") == expected, quests)
    except (EvidenceError, KeyError, TypeError) as error:
        errors.append(f"quest: {error}")
    try:
        nodes = paged(reader, "search_typed_properties", "results", source="private", includeNodes=True,
                      query=ACTORS["GORE_TEST_A"] + " RelationshipByGlobalId")
        modifiers = relationship_modifiers(nodes, ACTORS["GORE_TEST_A"])
        evidence["modifiers"] = modifiers
        stable = [row for row in modifiers if row["class"] == STORY and row["target"] == "Hero"]
        check("A.stable_Hero_Friend_modifier", len(stable) == 1
              and stable[0]["relationship"] == FRIEND, modifiers)
    except (EvidenceError, KeyError, TypeError) as error:
        errors.append(f"relationship: {error}")
    return {"evidence": evidence, "checks": checks, "errors": errors}


def persistence_checks(current: dict, previous: dict) -> list[dict]:
    if (previous.get("schemaVersion") != 1 or previous.get("fixture") != "gore-npc-session"
            or previous.get("ok") is not True or previous.get("phase") not in ("running", "completed")):
        raise EvidenceError("previous report must be a passing version 1 report for this fixture")
    checks = []

    def compare(name, left, right):
        checks.append({"name": name, "passed": left == right, "previous": left, "observed": right})

    compare("previous.phase_progression", True,
            previous["phase"] == "running" or current["phase"] == "completed")
    compare("previous.distinct_save_bytes", True,
            isinstance(previous.get("save", {}).get("sha256"), str)
            and isinstance(current.get("save", {}).get("sha256"), str)
            and previous["save"]["sha256"] != current["save"]["sha256"])
    for name in ACTORS:
        before = previous["evidence"]["actors"][name]
        after = current["evidence"]["actors"].get(name)
        if after is None:
            raise EvidenceError(f"cannot compare missing actor {name}")
        for field in ("globalId", "uniqueName"):
            compare(f"previous.{name}.{field}", before["character"][field], after["character"][field])
        for field, keys in (("attributes", ("key", "base", "current")),
                            ("inventory", ("path", "count", "containerType", "slotId"))):
            def canonical(item):
                values = item[field] if field == "attributes" else rows(item[field], "items")
                return sorted((dict((key, row[key]) for key in keys) for row in values),
                              key=lambda value: json.dumps(value, sort_keys=True))
            compare(f"previous.{name}.{field}", canonical(before), canonical(after))
    return checks


def output_path_allowed(output: Path, protected: list[Path]) -> bool:
    if output.suffix.casefold() in (".sav", ".dll"):
        return False
    for path in protected:
        if output.resolve() == path.resolve() or (output.exists() and path.exists() and output.samefile(path)):
            return False
    return True


def main(argv=None) -> int:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--save", required=True, type=Path)
    parser.add_argument("--dll", required=True, type=Path)
    parser.add_argument("--phase", required=True, choices=("running", "completed"))
    parser.add_argument("--previous", type=Path)
    parser.add_argument("--output", type=Path)
    parser.add_argument("--expect-diego-hp1234", action="store_true")
    args = parser.parse_args(argv)
    protected = [args.save, args.dll] + ([args.previous] if args.previous else [])
    if args.output and not output_path_allowed(args.output, protected):
        parser.error("report output must be separate from input files and cannot be a .sav or .dll")
    report = {"schemaVersion": 1, "fixture": "gore-npc-session", "phase": args.phase,
              "createdAt": datetime.now(timezone.utc).isoformat(), "ok": False,
              "runtimeRestartProven": False,
              "runtimeRestartNote": "Save bytes cannot establish a separate game process; user confirmation is required.",
              "expectDiegoHp1234": args.expect_diego_hp1234, "checks": [], "errors": [], "requests": []}
    reader = None
    try:
        report["save"] = fingerprint(args.save)
        report["core"] = fingerprint(args.dll)
        reader = SaveReader(args.dll, args.save)
        report.update(snapshot(reader, args.phase, args.expect_diego_hp1234))
        if args.previous:
            report["previousReport"] = fingerprint(args.previous)
            previous = json.loads(args.previous.read_text(encoding="utf-8-sig"))
            report["checks"].extend(persistence_checks(report, previous))
        report["saveAfterRead"] = fingerprint(args.save)
        report["checks"].append({"name": "save_unchanged_during_read", "passed": report["save"] == report["saveAfterRead"]})
        report["ok"] = not report["errors"] and all(check["passed"] for check in report["checks"])
    except (OSError, EvidenceError, ValueError, KeyError, TypeError, AttributeError) as error:
        report["errors"].append(str(error))
    if reader is not None:
        report["requests"] = reader.requests
    text = json.dumps(report, indent=2, ensure_ascii=False) + "\n"
    if args.output:
        args.output.write_text(text, encoding="utf-8")
        print(f"{'PASS' if report['ok'] else 'FAIL'}: {args.output.resolve()}")
    else:
        print(text, end="")
    return 0 if report["ok"] else 1


if __name__ == "__main__":
    sys.exit(main())
