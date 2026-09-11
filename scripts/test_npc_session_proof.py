from __future__ import annotations

import copy
import ctypes
import json
import os
import tempfile
import unittest
from pathlib import Path
from unittest import mock

import npc_session_proof as proof


def modifier(index=0, target="Hero", relation=proof.FRIEND, class_name=proof.STORY,
             actor=proof.ACTORS["GORE_TEST_A"]):
    path = ["m_GenericData", "{CharacterStates}", "NPCCharacters", "RelationshipByGlobalId",
            "{" + actor + "}", "ActivePersonalRelationshipModifiers", f"[{index}]"]
    return [
        {"path": path, "kind": "objectInstance", "source": "private",
         "value": class_name + " · 2 properties"},
        {"path": path + ["TargetCharacterGlobalID"], "source": "private", "value": target},
        {"path": path + ["Relationship"], "source": "private", "value": relation},
    ]


class FixtureReader:
    """Small recorded-schema fixture; no DLL or game installation needed."""
    def __init__(self, phase="running"):
        self.requests = []
        self.actors = {
            name: {"uniqueName": name, "globalId": global_id,
                   "hasKnowledge": True, "hasInventory": True,
                   "personalRelationship": "Friend" if name == "GORE_TEST_A" else None}
            for name, global_id in {**proof.ACTORS, "Hero": "Hero",
                                   "OC_STT_Diego": "OC_STT_Diego-WP_EZ_START_DIEGO_SPAWN"}.items()
        }
        self.entries = {name: [] for name in self.actors}
        self.entries["GORE_TEST_A"] = [proof.STARTED] + ([proof.COMPLETED] if phase == "completed" else [])
        self.quest = [{"questClass": proof.QUEST,
                       "currentState": "EQuestState::Succeeded" if phase == "completed" else "EQuestState::Running"}]
        self.nodes = modifier()
        self.duplicates = []

    def read(self, command, **payload):
        self.requests.append({"command": command, "payload": payload})
        if command not in proof.READ_COMMANDS:
            raise AssertionError("unexpected non-read command")
        if command == "private.characters.list":
            return {"characters": list(self.actors.values()) + self.duplicates}
        if command == "private.npc.attributes":
            return {"attributes": [{"key": key, "base": 1234, "current": 1234}
                                   for key in ("Health", "MaxHealth")]}
        if command == "private.npc.inventory":
            return {"id": payload["id"], "items": [
                {"path": "/Script/Angelscript.ItAm_Arrow", "count": 99, "slotId": 5,
                 "containerType": "MainContainer"},
                {"path": "/Script/Angelscript.ItAm_Arrow", "count": 1, "slotId": 6,
                 "containerType": "MainContainer"}]}
        if command == "private.npc.position":
            return {"pose": {"location": {"x": 1, "y": 2, "z": 3}}}
        if command == "query_progression":
            if payload["section"] == "quests":
                key, values = "quests", self.quest
            else:
                # Deliberately exact-case, matching the real knowledge read API.
                key, values = "entries", self.entries[payload["character"]]
        elif command == "search_typed_properties":
            key, values = "results", self.nodes
        else:
            raise AssertionError(command)
        page = values[payload["offset"]:payload["offset"] + payload["limit"]]
        return {key: page, "total": len(values), "count": len(page), "offset": payload["offset"]}


def passing(result):
    return not result["errors"] and all(check["passed"] for check in result["checks"])


def report(phase="running"):
    result = proof.snapshot(FixtureReader(phase), phase)
    return {"schemaVersion": 1, "fixture": "gore-npc-session", "phase": phase,
            "ok": passing(result), "save": {"sha256": "a" * 64 if phase == "running" else "b" * 64}, **result}


class NpcSessionProofTests(unittest.TestCase):
    def test_both_fixture_phases_and_optional_diego_attributes(self):
        for phase in ("running", "completed"):
            reader = FixtureReader(phase)
            result = proof.snapshot(reader, phase, expect_diego_hp1234=True)
            self.assertTrue(passing(result), result)
            self.assertTrue(all(item["command"] in proof.READ_COMMANDS for item in reader.requests))

    def test_knowledge_uses_stored_casing_and_absent_entry_needs_no_lookup(self):
        reader = FixtureReader()
        reader.actors["GORE_TEST_A"]["uniqueName"] = "gore_test_a"
        reader.entries["gore_test_a"] = reader.entries.pop("GORE_TEST_A")
        reader.actors["GORE_TEST_B"]["hasKnowledge"] = False
        result = proof.snapshot(reader, "running")
        self.assertTrue(passing(result), result)
        names = [item["payload"].get("character") for item in reader.requests]
        self.assertIn("gore_test_a", names)
        self.assertNotIn("GORE_TEST_B", names)

    def test_wrong_global_id_and_duplicate_identity_fail_before_inventory_read(self):
        for duplicate in (False, True):
            reader = FixtureReader()
            wrong = dict(reader.actors["GORE_TEST_A"], globalId="GORE_TEST_A-OTHER")
            if duplicate:
                reader.duplicates.append(wrong)
            else:
                reader.actors["GORE_TEST_A"] = wrong
            result = proof.snapshot(reader, "running")
            self.assertFalse(passing(result))
            self.assertTrue(any("GORE_TEST_A" in error for error in result["errors"]))
            self.assertFalse(any(item["payload"].get("id") == wrong["globalId"] for item in reader.requests))

    def test_marker_leakage_to_each_control_fails(self):
        for name in ("GORE_TEST_B", "Hero", "OC_STT_Diego"):
            reader = FixtureReader()
            reader.entries[name] = [proof.STARTED.upper()]
            self.assertFalse(passing(proof.snapshot(reader, "running")), name)

    def test_completion_markers_and_exact_quest_class_are_required(self):
        reader = FixtureReader("completed")
        reader.entries["GORE_TEST_A"] = [proof.COMPLETED]
        self.assertFalse(passing(proof.snapshot(reader, "completed")))
        reader = FixtureReader()
        reader.quest[0]["questClass"] += "_OTHER"
        self.assertFalse(passing(proof.snapshot(reader, "running")))
        reader = FixtureReader("completed")
        self.assertFalse(passing(proof.snapshot(reader, "running")))

    def test_relationship_requires_target_and_value_in_the_same_stable_object(self):
        invalid = [
            modifier(class_name=proof.STORY + "_Permanent"),
            modifier(target="SomeoneElse") + modifier(1, relation="ERelationship::Enemy"),
            modifier(actor=proof.ACTORS["GORE_TEST_B"]),
            modifier(relation="5"),
            modifier() + modifier(1),
        ]
        for nodes in invalid:
            reader = FixtureReader()
            reader.nodes = nodes
            result = proof.snapshot(reader, "running")
            self.assertFalse(passing(result), nodes)
        # Legacy friend may coexist after migration, but cannot be the only evidence.
        reader = FixtureReader()
        reader.nodes += modifier(1, class_name=proof.STORY + "_Permanent")
        self.assertTrue(passing(proof.snapshot(reader, "running")))

    def test_duplicate_relationship_paths_are_rejected(self):
        nodes = modifier()
        nodes.append(copy.deepcopy(nodes[1]))
        with self.assertRaises(proof.EvidenceError):
            proof.relationship_modifiers(nodes, proof.ACTORS["GORE_TEST_A"])

    def test_pagination_collects_all_entries_and_rejects_incomplete_reads(self):
        reader = FixtureReader()
        reader.entries["Hero"] = [str(index) for index in range(1002)]
        self.assertEqual(len(proof.knowledge(reader, reader.actors["Hero"])), 1002)
        with self.assertRaises(proof.EvidenceError):
            proof.paged(mock.Mock(read=lambda *a, **kw: {
                "entries": [], "total": 1, "count": 0, "offset": 0}),
                "query_progression", "entries", section="knowledge")

    def test_previous_report_detects_inventory_and_attribute_loss(self):
        before, after = report(), report("completed")
        after["evidence"]["actors"]["GORE_TEST_A"]["inventory"]["items"].reverse()
        self.assertTrue(all(check["passed"] for check in proof.persistence_checks(after, before)))
        after["evidence"]["actors"]["GORE_TEST_A"]["inventory"]["items"][0]["count"] += 1
        self.assertFalse(all(check["passed"] for check in proof.persistence_checks(after, before)))
        after = report("completed")
        after["evidence"]["actors"]["GORE_TEST_B"]["attributes"][0]["base"] = 0
        self.assertFalse(all(check["passed"] for check in proof.persistence_checks(after, before)))
        self.assertFalse(all(check["passed"] for check in proof.persistence_checks(report(), report("completed"))))
        before["ok"] = False
        with self.assertRaises(proof.EvidenceError):
            proof.persistence_checks(after, before)

    def test_reloading_requires_distinct_saved_bytes(self):
        before, after = report(), report()
        self.assertFalse(all(check["passed"] for check in proof.persistence_checks(after, before)))
        after["save"]["sha256"] = "c" * 64
        self.assertTrue(all(check["passed"] for check in proof.persistence_checks(after, before)))

    def test_reader_rejects_write_commands_and_frees_error_response(self):
        reader = proof.SaveReader.__new__(proof.SaveReader)
        reader.save, reader.requests = "save.sav", []
        with self.assertRaises(proof.EvidenceError):
            reader.read("write_save")
        with self.assertRaises(proof.EvidenceError):
            reader.read("query_progression", path="different.sav")
        response = ctypes.create_string_buffer(b'{"ok":false,"error":{"message":"fixture failure"}}')
        reader.library = mock.Mock()
        reader.library.goresave_execute.return_value = ctypes.addressof(response)
        with self.assertRaises(proof.EvidenceError):
            reader.read("private.characters.list")
        reader.library.goresave_free.assert_called_once_with(ctypes.addressof(response))

    def test_output_cannot_overwrite_save_dll_previous_or_alias(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            save = root / "input.sav"
            save.write_bytes(b"unchanged")
            alias = root / "alias.json"
            os.link(save, alias)
            self.assertFalse(proof.output_path_allowed(alias, [save]))
            self.assertFalse(proof.output_path_allowed(save, [save]))
            self.assertFalse(proof.output_path_allowed(root / "different.sav", [save]))
            self.assertFalse(proof.output_path_allowed(root / "library.dll", [save]))
            self.assertTrue(proof.output_path_allowed(root / "report.json", [save]))

    def test_cli_writes_machine_readable_hash_bound_report_without_restart_claim(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            save, dll, output = root / "input.sav", root / "core.dll", root / "report.json"
            save.write_bytes(b"save bytes")
            dll.write_bytes(b"core bytes")
            with mock.patch.object(proof, "SaveReader", return_value=FixtureReader()), mock.patch("builtins.print"):
                status = proof.main(["--save", str(save), "--dll", str(dll),
                                     "--phase", "running", "--output", str(output)])
            result = json.loads(output.read_text(encoding="utf-8"))
            self.assertEqual(status, 0)
            self.assertTrue(result["ok"])
            self.assertFalse(result["runtimeRestartProven"])
            self.assertEqual(result["save"], result["saveAfterRead"])
            self.assertEqual(save.read_bytes(), b"save bytes")
            self.assertEqual(result["save"]["path"], str(save.resolve()))


if __name__ == "__main__":
    unittest.main()
