# Knowledge Catalogs Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add catalog-backed pickers to the Knowledge tab — a full NPC list (incl. NPCs not yet in the save) and a full knowledge-entry list — plus a core op to add a brand-new NPC to the savegame's knowledge map.

**Architecture:** Two Python scripts scrape the UE4SS object dump into bundled Flutter JSON assets (mirroring `tools/build_item_catalog.py`). A new Rust core `MapInsert` container edit + `private.knowledge.addCharacter` IPC op inserts an empty `KnowledgeSet` entry into `CharacterKnowledgeByUniqueName`. New Flutter picker dialogs (mirroring `add_inventory_item_dialog.dart`) drive both, reusing the existing entry add/remove path.

**Tech Stack:** Rust (goresave_core), Python 3, Flutter/Dart.

**Reference spec:** `docs/superpowers/specs/2026-06-15-knowledge-catalogs-design.md`

**Dump path (for local script runs/tests):**
`D:\SteamLibrary\steamapps\common\Gothic 1 Remake\G1R\Binaries\Win64\ue4ss\UE4SS_ObjectDump.txt`

---

## File Structure

**Create:**
- `tools/build_npc_catalog.py` — dump → `npc_catalog.json`
- `tools/build_knowledge_catalog.py` — dump → `knowledge_catalog.json`
- `apps/goresave/assets/npc_catalog.json` — generated
- `apps/goresave/assets/knowledge_catalog.json` — generated
- `apps/goresave/lib/features/editor/domain/npc_catalog.dart` — loader
- `apps/goresave/lib/features/editor/domain/knowledge_catalog.dart` — loader
- `apps/goresave/lib/features/editor/ui/add_npc_dialog.dart` — NPC picker
- `apps/goresave/lib/features/editor/ui/add_knowledge_entry_dialog.dart` — entry picker
- `apps/goresave/test/add_npc_dialog_test.dart`
- `apps/goresave/test/add_knowledge_entry_dialog_test.dart`

**Modify:**
- `crates/goresave_core/src/properties.rs` — add `ContainerEdit::MapInsert`, `map_layout()`, `patch_container` arm
- `crates/goresave_core/src/lib.rs` — `private.knowledge.addCharacter` op + empty-`KnowledgeSet` encoder, promote `private_name_set_property` helper
- `apps/goresave/pubspec.yaml:60-62` — register the two new assets
- `apps/goresave/lib/features/editor/domain/editor_notifier.dart` — `addKnowledgeCharacter()` method
- `apps/goresave/lib/features/editor/ui/progression_panel.dart` — wire both dialogs into `_KnowledgeDetail`

---

## Phase A — Catalog build scripts + assets

### Task 1: NPC catalog build script

**Files:**
- Create: `tools/build_npc_catalog.py`
- Create (generated): `apps/goresave/assets/npc_catalog.json`
- Test: `tools/test_build_npc_catalog.py`

- [ ] **Step 1: Write the failing test**

```python
# tools/test_build_npc_catalog.py
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
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd tools && python -m pytest test_build_npc_catalog.py -v`
Expected: FAIL with `ModuleNotFoundError: No module named 'build_npc_catalog'`

- [ ] **Step 3: Write the script**

```python
#!/usr/bin/env python3
"""Build apps/goresave/assets/npc_catalog.json from a UE4SS object dump.

Usage: python tools/build_npc_catalog.py <UE4SS_ObjectDump.txt> [-o OUT.json]

Extracts CharacterDefinition_* class identifiers only (id, class, category).
The `id` of a Human definition is the exact CharacterKnowledgeByUniqueName map
key (e.g. OC_STT_Diego). No localized names or stats are extracted.
"""
from __future__ import annotations

import argparse
import json
import re
import sys
from pathlib import Path

CLASS_RE = re.compile(r"ASClass /Script/Angelscript\.(CharacterDefinition_[A-Za-z0-9_]+)")

# Sub-prefix after CharacterDefinition_ -> UI category.
CATEGORY_BY_SUBPREFIX = [
    ("Human_", "human"),
    ("Creature_", "creature"),
]


def parse_dump_classes(lines) -> list[str]:
    names: set[str] = set()
    for line in lines:
        match = CLASS_RE.search(line)
        if match:
            names.add(match.group(1))
    return sorted(names)


def build_catalog(class_names: list[str]) -> tuple[list[dict], list[str]]:
    entries: list[dict] = []
    skipped: list[str] = []
    for cls in class_names:
        rest = cls[len("CharacterDefinition_"):]
        category = "other"
        unique = rest
        for sub, cat in CATEGORY_BY_SUBPREFIX:
            if rest.startswith(sub):
                category = cat
                if cat == "human":
                    unique = rest[len(sub):]  # map-key form
                break
        if not unique:
            skipped.append(cls)
            continue
        entries.append({"id": unique, "class": cls, "category": category})
    entries.sort(key=lambda e: e["id"])
    # Dedup by id (different classes can collapse to the same unique name).
    seen: set[str] = set()
    deduped = []
    for e in entries:
        if e["id"] in seen:
            continue
        seen.add(e["id"])
        deduped.append(e)
    return deduped, skipped


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("dump", type=Path)
    parser.add_argument(
        "-o", "--out", type=Path,
        default=Path(__file__).resolve().parent.parent
        / "apps" / "goresave" / "assets" / "npc_catalog.json",
    )
    args = parser.parse_args()
    names = parse_dump_classes(
        args.dump.read_text(encoding="utf-8", errors="replace").splitlines()
    )
    entries, skipped = build_catalog(names)
    args.out.write_text(
        json.dumps(entries, indent=2, sort_keys=True) + "\n", encoding="utf-8"
    )
    print(f"wrote {len(entries)} npcs to {args.out}")
    if skipped:
        print(f"skipped {len(skipped)} classes")
    return 0


if __name__ == "__main__":
    sys.exit(main())
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cd tools && python -m pytest test_build_npc_catalog.py -v`
Expected: PASS (4 tests)

- [ ] **Step 5: Generate the asset from the real dump**

Run:
```
python tools/build_npc_catalog.py "D:\SteamLibrary\steamapps\common\Gothic 1 Remake\G1R\Binaries\Win64\ue4ss\UE4SS_ObjectDump.txt"
```
Expected: `wrote ~1000 npcs to ...npc_catalog.json`. Sanity-check it contains `OC_STT_Diego`:
Run: `grep -c '"OC_STT_Diego"' apps/goresave/assets/npc_catalog.json` → `1`

- [ ] **Step 6: Commit**

```bash
git add tools/build_npc_catalog.py tools/test_build_npc_catalog.py apps/goresave/assets/npc_catalog.json
git commit -m "feat(tools): build npc_catalog.json from UE4SS dump"
```

### Task 2: Knowledge entry catalog build script

**Files:**
- Create: `tools/build_knowledge_catalog.py`
- Create (generated): `apps/goresave/assets/knowledge_catalog.json`
- Test: `tools/test_build_knowledge_catalog.py`

- [ ] **Step 1: Write the failing test**

```python
# tools/test_build_knowledge_catalog.py
import build_knowledge_catalog as m

DUMP = [
    "[1] ASClass /Script/Angelscript.Topic_Diego_209799 [n: A]",
    "[2] ASClass /Script/Angelscript.Info_FMORGAreyouok [n: B]",
    "[3] ASClass /Script/Angelscript.ChoiceDiegoGamestart [n: C]",
    "[4] ASClass /Script/Angelscript.Topic_Diego_209799 [n: D]",  # dup
    "[5] ASClass /Script/Angelscript.ItMw_Sword01 [n: E]",  # ignored
    "[6] ASClass /Script/Angelscript.CharacterDefinition_Human_OC_STT_Diego [n: F]",  # ignored
]

def test_categories():
    entries = m.build_catalog(m.parse_dump_classes(DUMP))
    by_id = {e["id"]: e for e in entries}
    assert by_id["Topic_Diego_209799"]["category"] == "topic"
    assert by_id["Info_FMORGAreyouok"]["category"] == "info"
    assert by_id["ChoiceDiegoGamestart"]["category"] == "choice"

def test_dedup_sorted_and_filtered():
    entries = m.build_catalog(m.parse_dump_classes(DUMP))
    ids = [e["id"] for e in entries]
    assert ids == sorted(ids)
    assert ids.count("Topic_Diego_209799") == 1
    assert all("Sword" not in i and "CharacterDefinition" not in i for i in ids)
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd tools && python -m pytest test_build_knowledge_catalog.py -v`
Expected: FAIL with `ModuleNotFoundError`

- [ ] **Step 3: Write the script**

```python
#!/usr/bin/env python3
"""Build apps/goresave/assets/knowledge_catalog.json from a UE4SS object dump.

Usage: python tools/build_knowledge_catalog.py <UE4SS_ObjectDump.txt> [-o OUT.json]

Extracts Topic_/Info_/Choice* class identifiers (id, category). These are the
dialog-unlock knowledge tokens that appear in a character's Knowledge set.
Voiceline tokens are intentionally excluded (localization keys, not classes).
"""
from __future__ import annotations

import argparse
import json
import re
import sys
from pathlib import Path

# Order matters: Topic_/Info_ before bare Choice.
PATTERNS = [
    (re.compile(r"ASClass /Script/Angelscript\.(Topic_[A-Za-z0-9_]+)"), "topic"),
    (re.compile(r"ASClass /Script/Angelscript\.(Info_[A-Za-z0-9_]+)"), "info"),
    (re.compile(r"ASClass /Script/Angelscript\.(Choice[A-Za-z0-9_]+)"), "choice"),
]


def parse_dump_classes(lines) -> list[tuple[str, str]]:
    found: dict[str, str] = {}
    for line in lines:
        for rx, category in PATTERNS:
            match = rx.search(line)
            if match:
                found.setdefault(match.group(1), category)
                break
    return sorted(found.items())


def build_catalog(pairs: list[tuple[str, str]]) -> list[dict]:
    entries = [{"id": name, "category": cat} for name, cat in pairs]
    entries.sort(key=lambda e: e["id"])
    return entries


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("dump", type=Path)
    parser.add_argument(
        "-o", "--out", type=Path,
        default=Path(__file__).resolve().parent.parent
        / "apps" / "goresave" / "assets" / "knowledge_catalog.json",
    )
    args = parser.parse_args()
    pairs = parse_dump_classes(
        args.dump.read_text(encoding="utf-8", errors="replace").splitlines()
    )
    entries = build_catalog(pairs)
    args.out.write_text(
        json.dumps(entries, indent=2, sort_keys=True) + "\n", encoding="utf-8"
    )
    print(f"wrote {len(entries)} knowledge tokens to {args.out}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cd tools && python -m pytest test_build_knowledge_catalog.py -v`
Expected: PASS (2 tests)

- [ ] **Step 5: Generate the asset from the real dump**

Run:
```
python tools/build_knowledge_catalog.py "D:\SteamLibrary\steamapps\common\Gothic 1 Remake\G1R\Binaries\Win64\ue4ss\UE4SS_ObjectDump.txt"
```
Expected: `wrote ~3900 knowledge tokens ...`. Sanity-check:
Run: `grep -c '"Topic_Diego_209799"' apps/goresave/assets/knowledge_catalog.json` → `1`

- [ ] **Step 6: Commit**

```bash
git add tools/build_knowledge_catalog.py tools/test_build_knowledge_catalog.py apps/goresave/assets/knowledge_catalog.json
git commit -m "feat(tools): build knowledge_catalog.json from UE4SS dump"
```

---

## Phase B — Core map insertion + add-character op

Background on the on-disk format (verified in `properties.rs`):
- `CharacterKnowledgeByUniqueName` is a `MapProperty<NameProperty, StructProperty(KnowledgeSet)>`.
- Map body layout: `num_to_remove:u32`, `count:u32`, then `count` × (inline key, inline value).
- Inline `NameProperty` key = `fstring(name)`.
- Inline `StructProperty` value (KnowledgeSet, non-native) = a tagged-property list terminated by `fstring("None")`. The one property is `Knowledge` (a `SetProperty<NameProperty>`).
- The existing test helper `private_name_set_property(name, values)` (`lib.rs:11143`) emits exactly one such tagged `SetProperty`. An empty `KnowledgeSet` value = `private_name_set_property("Knowledge", &[]) ++ fstring("None")`.

### Task 3: `map_layout()` helper

**Files:**
- Modify: `crates/goresave_core/src/properties.rs` (add after `container_layout`, ~line 970)
- Test: same file, `#[cfg(test)]` module

- [ ] **Step 1: Write the failing test**

Add to the `properties.rs` test module (reuse existing `fstring` + `map_of_instanced_payload`-style helpers; if a Name→Set map helper does not exist, add this one near the other test helpers):

```rust
// test helper: a MapProperty<NameProperty, StructProperty(KnowledgeSet)> with
// `chars` entries, each an empty Knowledge set. Returns a full tagged property.
fn knowledge_map_property(chars: &[&str]) -> Vec<u8> {
    // inline value bytes: tagged "Knowledge" SetProperty(empty) + "None"
    let empty_value = || {
        let mut v = name_set_property("Knowledge", &[]); // existing test helper
        v.extend_from_slice(&fstring("None"));
        v
    };
    let mut body = 0u32.to_le_bytes().to_vec();              // num_to_remove
    body.extend_from_slice(&(chars.len() as u32).to_le_bytes()); // count
    for c in chars {
        body.extend_from_slice(&fstring(c));   // inline Name key
        body.extend_from_slice(&empty_value()); // inline struct value
    }
    // tag header: name, "MapProperty", key/value inner type names, size, flags
    let mut out = fstring("CharacterKnowledgeByUniqueName");
    out.extend_from_slice(&fstring("MapProperty"));
    out.extend_from_slice(&0u32.to_le_bytes()); // array_index
    out.extend_from_slice(&fstring("NameProperty"));   // key type
    out.extend_from_slice(&fstring("StructProperty"));  // value type
    out.extend_from_slice(&fstring("KnowledgeSet"));    // value struct type
    out.extend_from_slice(&(body.len() as u32).to_le_bytes());
    out.push(0); // tag_flags
    out.extend_from_slice(&body);
    out
}

#[test]
fn map_layout_reports_count_and_entry_ranges() {
    // Build a private root whose only property is the knowledge map.
    let payload = private_root_with_property(&knowledge_map_property(&["A", "BB"]));
    let root = parse_private_root(&payload).unwrap();
    let (_, prop) = find_property_by_name(&root, "CharacterKnowledgeByUniqueName").unwrap();
    let layout = map_layout(&payload, prop).unwrap();
    assert_eq!(layout.count, 2);
    assert_eq!(layout.entry_ranges.len(), 2);
    // entry_ranges must be contiguous and end at the map body end.
    assert_eq!(layout.entry_ranges[0].end, layout.entry_ranges[1].start);
}
```

> NOTE: `name_set_property`, `fstring`, and a private-root wrapper helper already
> exist in the `properties.rs`/`lib.rs` test modules (see `properties.rs:2920`,
> `2176`). If `private_root_with_property` does not exist in `properties.rs`,
> add a minimal one that wraps a single tagged property in the class/flag/footer
> framing `parse_private_root` expects (copy the structure from
> `map_of_instanced_payload`'s caller).

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p goresave_core map_layout_reports_count -- --nocapture`
Expected: FAIL — `cannot find function map_layout`

- [ ] **Step 3: Implement `map_layout`**

```rust
/// Byte layout of a MapProperty value: the count-field offset and the absolute
/// byte range of every (key+value) entry. Mirrors `container_layout` for maps,
/// which `container_layout` rejects (maps have inline key/value pairs).
#[derive(Debug, Clone, PartialEq)]
pub struct MapLayout {
    pub count_offset: usize,
    pub count: usize,
    /// Absolute byte range of each entry (key bytes + value bytes).
    pub entry_ranges: Vec<core::ops::Range<usize>>,
}

pub fn map_layout(payload: &[u8], property: &Property) -> Result<MapLayout, CoreError> {
    if property.type_name != "MapProperty" {
        return Err(CoreError::InvalidRequest(format!(
            "map_layout requires a MapProperty target, got {}",
            property.type_name
        )));
    }
    let (key, value) = property
        .descriptor
        .map
        .as_deref()
        .ok_or_else(|| CoreError::Parse("MapProperty missing descriptor".into()))?;
    let end = property
        .value_offset
        .checked_add(property.value_size)
        .filter(|end| *end <= payload.len())
        .ok_or_else(|| CoreError::Parse("map value out of bounds".to_string()))?;
    let mut r = Reader::new(&payload[property.value_offset..end], property.value_offset);
    let _num_to_remove = r.u32()?;
    let count_offset = r.abs_pos();
    let count = r.u32()? as usize;
    let mut entry_ranges = Vec::with_capacity(count.min(1 << 16));
    for _ in 0..count {
        let start = r.abs_pos();
        read_inline_value(&mut r, key, 0)?;
        read_inline_value(&mut r, value, 0)?;
        entry_ranges.push(start..r.abs_pos());
    }
    if r.remaining() != 0 {
        return Err(CoreError::Parse(format!(
            "map body left {} bytes after {count} entries",
            r.remaining()
        )));
    }
    Ok(MapLayout { count_offset, count, entry_ranges })
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p goresave_core map_layout_reports_count`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add crates/goresave_core/src/properties.rs
git commit -m "feat(core): add map_layout helper for MapProperty editing"
```

### Task 4: `ContainerEdit::MapInsert` + `patch_container` arm

**Files:**
- Modify: `crates/goresave_core/src/properties.rs` (`ContainerEdit` enum ~973; `patch_container` ~1020)
- Test: same file

- [ ] **Step 1: Write the failing test**

```rust
#[test]
fn map_insert_appends_entry_and_fixes_sizes() {
    let mut payload = private_root_with_property(&knowledge_map_property(&["A"]));
    let root = parse_private_root(&payload).unwrap();
    let (_, prop) = find_property_by_name(&root, "CharacterKnowledgeByUniqueName").unwrap();
    let enclosing = Vec::new(); // top-level property; no enclosing size fields

    // Build new entry bytes: key "ZZ" + empty KnowledgeSet value.
    let mut entry = fstring("ZZ");
    let mut val = name_set_property("Knowledge", &[]);
    val.extend_from_slice(&fstring("None"));
    entry.extend_from_slice(&val);

    patch_container(
        &mut payload,
        prop,
        &enclosing,
        &ContainerEdit::MapInsert { entry_bytes: entry },
    )
    .unwrap();

    // Re-parse: the map now has 2 entries incl. "ZZ".
    let root2 = parse_private_root(&payload).unwrap();
    let (_, prop2) = find_property_by_name(&root2, "CharacterKnowledgeByUniqueName").unwrap();
    let PropertyValue::Map { entries, .. } = &prop2.value else { panic!("not a map") };
    assert_eq!(entries.len(), 2);
    assert!(entries.iter().any(|(k, _)| matches!(k, PropertyValue::Name(s) if s == "ZZ")));
    // consumed == payload length proves all size fields are consistent.
    assert_eq!(root2.consumed, payload.len());
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p goresave_core map_insert_appends_entry`
Expected: FAIL — `no variant ... MapInsert`

- [ ] **Step 3: Add the enum variant**

In `ContainerEdit` (after `ArrayInsertBytes`, ~line 988):

```rust
    /// Append a pre-built (inline key ++ inline value) entry to a MapProperty.
    /// The bytes must be schema-valid for this map's key/value descriptors; the
    /// caller validates via the re-parse it performs afterwards.
    MapInsert { entry_bytes: Vec<u8> },
```

- [ ] **Step 4: Implement the `patch_container` arm**

Inside `patch_container`'s `match edit { ... }` (alongside the other arms, before the closing of the match that produces `(remove_range, insert_at, insert_bytes, count_delta)`):

```rust
        ContainerEdit::MapInsert { entry_bytes } => {
            if target.type_name != "MapProperty" {
                return Err(CoreError::InvalidRequest(format!(
                    "mapInsert requires a MapProperty target, got {}",
                    target.type_name
                )));
            }
            let map = map_layout(payload, target)?;
            let insert_at = target.value_offset + target.value_size; // end of map body
            (None, insert_at, entry_bytes.clone(), 1)
        }
```

The count-field update is keyed off `layout.count_offset` for Set/Array. Maps
have no `ContainerLayout`; their count-field offset comes from `map_layout`.
Compute the count offset once, up front (before any splice, while offsets are
still valid), and branch on the edit type:

```rust
    // Determine which count field to bump. Computed before the splice; offsets
    // into `payload` are valid here because no mutation has happened yet.
    let count_offset = match edit {
        ContainerEdit::MapInsert { .. } => map_layout(payload, target)?.count_offset,
        _ => layout.count_offset,
    };
```

> IMPLEMENTATION NOTE: `container_layout(payload, target)` at the top of
> `patch_container` returns `Err` for a `MapProperty` (it only accepts
> Array/Set). Restructure the function head so the layout/count-offset lookup is
> edit-aware: for `MapInsert`, call `map_layout` and skip `container_layout`; for
> the existing edits, keep `container_layout`. The size-chain fixup (tag size at
> `target.size_field_offset()` + every offset in `enclosing_size_fields`, then
> the splice) is identical for both and must be reused, not duplicated. Keep the
> "validate before mutate" guarantee: compute all offsets/deltas first, splice
> last.

- [ ] **Step 5: Run test to verify it passes**

Run: `cargo test -p goresave_core map_insert_appends_entry`
Expected: PASS

- [ ] **Step 6: Run the full core suite (no regressions)**

Run: `cargo test -p goresave_core`
Expected: PASS (all existing tests still green)

- [ ] **Step 7: Commit**

```bash
git add crates/goresave_core/src/properties.rs
git commit -m "feat(core): add ContainerEdit::MapInsert for map entry append"
```

### Task 5: Empty-`KnowledgeSet` encoder + promote set helper

**Files:**
- Modify: `crates/goresave_core/src/lib.rs` (promote `private_name_set_property` out of `#[cfg(test)]`; add `encode_empty_knowledge_value`)
- Test: `crates/goresave_core/src/lib.rs` test module

- [ ] **Step 1: Write the failing test**

```rust
#[test]
fn empty_knowledge_value_roundtrips_as_struct_with_empty_set() {
    let key = "OC_TEST_Npc";
    let entry = encode_knowledge_map_entry(key);
    // Parse the entry bytes as inline key+value using the same descriptors the
    // map uses (NameProperty key, StructProperty/KnowledgeSet value).
    let parsed = parse_knowledge_entry_for_test(&entry); // helper below
    assert_eq!(parsed.0, key);                 // key
    assert!(parsed.1.is_empty());              // Knowledge set has 0 elements
}
```

> The test helper `parse_knowledge_entry_for_test` builds a one-entry map around
> `entry`, runs `parse_private_root`, and returns `(key_string, knowledge_vec)`.
> Reuse `knowledge_map_property` machinery from Task 3 by wrapping `entry` rather
> than re-deriving it.

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p goresave_core empty_knowledge_value_roundtrips`
Expected: FAIL — `cannot find function encode_knowledge_map_entry`

- [ ] **Step 3: Implement encoders**

Promote the existing test helper to module scope (remove it from the test module, place near other byte writers; keep `fn fstring` usage consistent with the crate's existing FString writer — use the production `encode_fstring_value` if `fstring` is test-only):

```rust
/// Serialize one `CharacterKnowledgeByUniqueName` entry: inline Name key plus an
/// inline `KnowledgeSet` struct value holding an empty `Knowledge` set.
fn encode_knowledge_map_entry(unique_name: &str) -> Vec<u8> {
    let mut out = properties::encode_fstring_value(unique_name); // inline Name key
    // inline struct value: one tagged "Knowledge" SetProperty(empty) + "None"
    out.extend_from_slice(&encode_empty_name_set_property("Knowledge"));
    out.extend_from_slice(&properties::encode_fstring_value("None"));
    out
}

/// A tagged `SetProperty<NameProperty>` with zero elements.
fn encode_empty_name_set_property(name: &str) -> Vec<u8> {
    let body = {
        let mut b = 0u32.to_le_bytes().to_vec(); // num_to_remove
        b.extend_from_slice(&0u32.to_le_bytes()); // count
        b
    };
    let mut out = properties::encode_fstring_value(name);
    out.extend_from_slice(&properties::encode_fstring_value("SetProperty"));
    out.extend_from_slice(&1u32.to_le_bytes());                  // array_index
    out.extend_from_slice(&properties::encode_fstring_value("NameProperty"));
    out.extend_from_slice(&(body.len() as u32).to_le_bytes());
    out.push(0); // tag_flags
    out.extend_from_slice(&body);
    out
}
```

> If `properties::encode_fstring_value` is not `pub`, make it `pub(crate)`. The
> byte layout above must match `private_name_set_property` from the test module
> exactly — if a byte differs, the Task 4 round-trip test is the oracle.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p goresave_core empty_knowledge_value_roundtrips`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add crates/goresave_core/src/lib.rs
git commit -m "feat(core): encode empty KnowledgeSet map entry bytes"
```

### Task 6: `private.knowledge.addCharacter` IPC op

**Files:**
- Modify: `crates/goresave_core/src/lib.rs` (op dispatch ~4893; new apply fn near `apply_private_typed_container_edit_to_payload` ~5703)
- Test: `crates/goresave_core/src/lib.rs` integration test using a real payload

- [ ] **Step 1: Write the failing test**

```rust
#[test]
fn add_character_then_set_add_roundtrips_on_real_save() {
    // Real decompressed private payload (see gothic-remake dump memory).
    let payload = std::fs::read("../../work/decompressed/G1R-001.host.bin").unwrap();

    // 1. New character must not already exist.
    let new_npc = "OC_TEST_BrandNew";
    let before = progression_knowledge(
        &properties::parse_private_root(&payload).unwrap(), "", None, 0, 10_000,
    ).unwrap();
    assert!(!before["characters"].as_array().unwrap()
        .iter().any(|c| c["name"] == new_npc));

    // 2. Apply addCharacter to the payload.
    let mut p = payload.clone();
    apply_private_knowledge_add_character_to_payload(&mut p, new_npc).unwrap();

    // 3. Re-query: the character exists with 0 entries.
    let root = properties::parse_private_root(&p).unwrap();
    let after = progression_knowledge(&root, "", Some(new_npc), 0, 10).unwrap();
    assert_eq!(after["total"], 0);
    assert_eq!(root.consumed, p.len());

    // 4. Duplicate insert is rejected.
    assert!(apply_private_knowledge_add_character_to_payload(&mut p, new_npc).is_err());
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p goresave_core add_character_then_set_add`
Expected: FAIL — `cannot find function apply_private_knowledge_add_character_to_payload`

- [ ] **Step 3: Implement the apply function**

Model it on `apply_private_typed_container_edit_to_payload` (`lib.rs:5703`): parse the
root, resolve `CharacterKnowledgeByUniqueName` and its enclosing size-field
offsets, reject if the key already exists, then `patch_container` with `MapInsert`.

```rust
fn apply_private_knowledge_add_character_to_payload(
    payload: &mut Vec<u8>,
    unique_name: &str,
) -> Result<(), CoreError> {
    let name = unique_name.trim();
    if name.is_empty() {
        return Err(CoreError::InvalidRequest("character name is empty".into()));
    }
    let root = properties::parse_private_root(payload)?;
    let (path, map_prop) =
        properties::find_property_by_name(&root, "CharacterKnowledgeByUniqueName")
            .ok_or_else(|| CoreError::Parse(
                "CharacterKnowledgeByUniqueName not found".into(),
            ))?;
    // Reject duplicates (UE Name keys compare case-insensitively).
    if let PropertyValue::Map { entries, .. } = &map_prop.value {
        if entries.iter().any(|(k, _)| {
            properties::map_key_to_string(k)
                .map(|s| s.eq_ignore_ascii_case(name))
                .unwrap_or(false)
        }) {
            return Err(CoreError::InvalidRequest(format!(
                "character {name:?} already has a knowledge entry"
            )));
        }
    }
    let enclosing = properties::enclosing_size_fields_for_path(&root, &path)?; // see note
    let entry = encode_knowledge_map_entry(name);
    properties::patch_container(
        payload,
        map_prop,
        &enclosing,
        &properties::ContainerEdit::MapInsert { entry_bytes: entry },
    )?;
    // Validate: strict re-parse must succeed and the key must now resolve.
    let root2 = properties::parse_private_root(payload)?;
    properties::find_property_by_name(&root2, "CharacterKnowledgeByUniqueName")
        .and_then(|(_, p)| match &p.value {
            PropertyValue::Map { entries, .. } => entries.iter().find(|(k, _)| {
                properties::map_key_to_string(k).as_deref() == Some(name)
            }),
            _ => None,
        })
        .ok_or_else(|| CoreError::Parse("post-insert validation failed".into()))?;
    Ok(())
}
```

> NOTE on `enclosing_size_fields_for_path`: `apply_private_typed_container_edit_to_payload`
> already computes the enclosing size-field offsets for a resolved path (it must,
> to pass them to `patch_container`). Reuse that exact mechanism — extract it into
> a shared helper if it is currently inline, rather than re-deriving. Borrow note:
> `map_prop` borrows `root`; clone the small bits you need (the property is needed
> by `patch_container` which takes `&Property` but mutates `payload`) — follow the
> same borrow pattern the typed-container apply fn uses (it re-resolves against a
> parse that is dropped before the mutable splice, or clones offsets). Match it.

- [ ] **Step 4: Wire the op into dispatch**

At the op dispatch (`lib.rs:4893`, alongside `"private.typed.setAdd"`), add:

```rust
        "private.knowledge.addCharacter" => {
            let name = payload
                .get("value")
                .and_then(|v| v.as_str())
                .ok_or_else(|| CoreError::InvalidRequest(
                    "private.knowledge.addCharacter requires a string `value`".into(),
                ))?
                .to_string();
            PrivateEdit::KnowledgeAddCharacter(name)
        }
```

Add the `PrivateEdit::KnowledgeAddCharacter(String)` variant to the `PrivateEdit`
enum, and in the place where `PrivateEdit` variants are applied to the payload
(search for where `PrivateEdit::TypedContainer` is matched and
`apply_private_typed_container_edit_to_payload` is called), add:

```rust
        PrivateEdit::KnowledgeAddCharacter(name) => {
            apply_private_knowledge_add_character_to_payload(payload, name)?;
        }
```

- [ ] **Step 5: Run test to verify it passes**

Run: `cargo test -p goresave_core add_character_then_set_add`
Expected: PASS

- [ ] **Step 6: Full core suite**

Run: `cargo test -p goresave_core`
Expected: PASS

- [ ] **Step 7: Commit**

```bash
git add crates/goresave_core/src/lib.rs
git commit -m "feat(core): add private.knowledge.addCharacter op"
```

---

## Phase C — Flutter catalog loaders

### Task 7: NPC + knowledge catalog loaders and asset registration

**Files:**
- Create: `apps/goresave/lib/features/editor/domain/npc_catalog.dart`
- Create: `apps/goresave/lib/features/editor/domain/knowledge_catalog.dart`
- Modify: `apps/goresave/pubspec.yaml:60-62`
- Test: `apps/goresave/test/catalog_loaders_test.dart`

- [ ] **Step 1: Write the failing test**

```dart
// apps/goresave/test/catalog_loaders_test.dart
import 'package:flutter_test/flutter_test.dart';
import 'package:goresave/features/editor/domain/npc_catalog.dart';
import 'package:goresave/features/editor/domain/knowledge_catalog.dart';

void main() {
  test('NpcCatalog parses, filters empties, sorts by id', () {
    final c = NpcCatalog.fromJsonString(
      '[{"id":"OC_STT_Diego","class":"CharacterDefinition_Human_OC_STT_Diego","category":"human"},'
      '{"id":"","class":"x","category":"human"},'
      '{"id":"Creature_Biter","class":"CharacterDefinition_Creature_Biter","category":"creature"}]',
    );
    expect(c.entries.map((e) => e.id), ['Creature_Biter', 'OC_STT_Diego']);
    expect(c.entries.first.category, 'creature');
  });

  test('KnowledgeCatalog parses, filters empties, sorts by id', () {
    final c = KnowledgeCatalog.fromJsonString(
      '[{"id":"Topic_Diego_209799","category":"topic"},'
      '{"id":"","category":"choice"},'
      '{"id":"ChoiceDiegoGamestart","category":"choice"}]',
    );
    expect(c.entries.map((e) => e.id), ['ChoiceDiegoGamestart', 'Topic_Diego_209799']);
  });
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd apps/goresave && flutter test test/catalog_loaders_test.dart`
Expected: FAIL — cannot find `npc_catalog.dart`

- [ ] **Step 3: Implement `npc_catalog.dart`** (mirror `item_catalog.dart`)

```dart
import 'dart:convert';

import 'package:flutter/services.dart' show rootBundle;

class NpcCatalogEntry {
  const NpcCatalogEntry({
    required this.id,
    required this.className,
    required this.category,
  });

  final String id;
  final String className;
  final String category;
}

class NpcCatalog {
  const NpcCatalog(this.entries);

  final List<NpcCatalogEntry> entries;

  static NpcCatalog fromJsonString(String json) {
    final list = (jsonDecode(json) as List)
        .whereType<Map<String, Object?>>()
        .map((e) => NpcCatalogEntry(
              id: e['id'] as String? ?? '',
              className: e['class'] as String? ?? '',
              category: e['category'] as String? ?? 'other',
            ))
        .where((e) => e.id.isNotEmpty)
        .toList()
      ..sort((a, b) => a.id.compareTo(b.id));
    return NpcCatalog(list);
  }

  static Future<NpcCatalog> loadBundled() async =>
      fromJsonString(await rootBundle.loadString('assets/npc_catalog.json'));
}
```

- [ ] **Step 4: Implement `knowledge_catalog.dart`**

```dart
import 'dart:convert';

import 'package:flutter/services.dart' show rootBundle;

class KnowledgeCatalogEntry {
  const KnowledgeCatalogEntry({required this.id, required this.category});

  final String id;
  final String category;
}

class KnowledgeCatalog {
  const KnowledgeCatalog(this.entries);

  final List<KnowledgeCatalogEntry> entries;

  static KnowledgeCatalog fromJsonString(String json) {
    final list = (jsonDecode(json) as List)
        .whereType<Map<String, Object?>>()
        .map((e) => KnowledgeCatalogEntry(
              id: e['id'] as String? ?? '',
              category: e['category'] as String? ?? 'topic',
            ))
        .where((e) => e.id.isNotEmpty)
        .toList()
      ..sort((a, b) => a.id.compareTo(b.id));
    return KnowledgeCatalog(list);
  }

  static Future<KnowledgeCatalog> loadBundled() async =>
      fromJsonString(await rootBundle.loadString('assets/knowledge_catalog.json'));
}
```

- [ ] **Step 5: Register assets in pubspec**

Modify `apps/goresave/pubspec.yaml` (the `assets:` block at line 60):

```yaml
  assets:
    - assets/goresave_icon.png
    - assets/item_catalog.json
    - assets/npc_catalog.json
    - assets/knowledge_catalog.json
```

- [ ] **Step 6: Run test to verify it passes**

Run: `cd apps/goresave && flutter test test/catalog_loaders_test.dart`
Expected: PASS

- [ ] **Step 7: Commit**

```bash
git add apps/goresave/lib/features/editor/domain/npc_catalog.dart apps/goresave/lib/features/editor/domain/knowledge_catalog.dart apps/goresave/pubspec.yaml apps/goresave/test/catalog_loaders_test.dart
git commit -m "feat(app): bundle npc + knowledge catalog loaders"
```

---

## Phase D — Flutter dialogs + wiring

### Task 8: Notifier method `addKnowledgeCharacter`

**Files:**
- Modify: `apps/goresave/lib/features/editor/domain/editor_notifier.dart` (near `loadKnowledgeCharacters`, ~1173)
- Test: covered by Task 10's widget test + manual; add a focused notifier test if a notifier test harness exists.

- [ ] **Step 1: Implement the method**

Find how an inventory add edit is sent to the core (search `addItem` / the edit op send path in `editor_notifier.dart`) and follow it exactly. Add:

```dart
  /// Insert a brand-new character into CharacterKnowledgeByUniqueName.
  /// Returns null on success, or an error string.
  Future<String?> addKnowledgeCharacter(String uniqueName) async {
    final result = await _sendPrivateEdit({
      'op': 'private.knowledge.addCharacter',
      'value': uniqueName,
    });
    return result.error; // shape matches existing _sendPrivateEdit callers
  }
```

> The exact request envelope (`op`/`value` keys, the `_sendPrivateEdit` name) must
> match what the existing typed-container edits send. Read the `setAdd` send path
> in this file and mirror its envelope and error handling precisely — do not
> invent new field names.

- [ ] **Step 2: Verify it compiles**

Run: `cd apps/goresave && flutter analyze lib/features/editor/domain/editor_notifier.dart`
Expected: No errors.

- [ ] **Step 3: Commit**

```bash
git add apps/goresave/lib/features/editor/domain/editor_notifier.dart
git commit -m "feat(app): notifier addKnowledgeCharacter"
```

### Task 9: Add-NPC + add-entry picker dialogs

**Files:**
- Create: `apps/goresave/lib/features/editor/ui/add_npc_dialog.dart`
- Create: `apps/goresave/lib/features/editor/ui/add_knowledge_entry_dialog.dart`
- Test: `apps/goresave/test/add_npc_dialog_test.dart`, `apps/goresave/test/add_knowledge_entry_dialog_test.dart`

Both dialogs mirror `apps/goresave/lib/features/editor/ui/add_inventory_item_dialog.dart`
(category sidebar + search + list; returns the selected id via `Navigator.pop`).
Differences: NPC dialog categories = the distinct `category` values in
`NpcCatalog`; entry dialog categories = `topic`/`choice`/`info`. Each takes an
`exclude` `Set<String>` (already-present ids, lowercased) to hide.

- [ ] **Step 1: Write the failing test (NPC dialog)**

```dart
// apps/goresave/test/add_npc_dialog_test.dart
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:goresave/features/editor/domain/npc_catalog.dart';
import 'package:goresave/features/editor/ui/add_npc_dialog.dart';

void main() {
  final catalog = NpcCatalog.fromJsonString(
    '[{"id":"OC_STT_Diego","class":"c1","category":"human"},'
    '{"id":"OC_GRD_Orry_254","class":"c2","category":"human"},'
    '{"id":"Creature_Biter","class":"c3","category":"creature"}]',
  );

  testWidgets('lists catalog NPCs, excludes existing, returns selection',
      (tester) async {
    String? picked;
    await tester.pumpWidget(MaterialApp(
      home: Scaffold(
        body: Builder(builder: (context) {
          return ElevatedButton(
            onPressed: () async {
              picked = await showAddNpcDialog(
                context,
                catalog: catalog,
                exclude: {'oc_grd_orry_254'},
              );
            },
            child: const Text('open'),
          );
        }),
      ),
    ));
    await tester.tap(find.text('open'));
    await tester.pumpAndSettle();

    expect(find.text('OC_STT_Diego'), findsOneWidget);
    expect(find.text('OC_GRD_Orry_254'), findsNothing); // excluded
    await tester.tap(find.text('OC_STT_Diego'));
    await tester.pumpAndSettle();
    expect(picked, 'OC_STT_Diego');
  });
}
```

- [ ] **Step 2: Run to verify it fails**

Run: `cd apps/goresave && flutter test test/add_npc_dialog_test.dart`
Expected: FAIL — cannot find `add_npc_dialog.dart`

- [ ] **Step 3: Implement `add_npc_dialog.dart`**

Copy the structure of `add_inventory_item_dialog.dart` and adapt:
- Public entrypoint `Future<String?> showAddNpcDialog(BuildContext context, {required NpcCatalog catalog, required Set<String> exclude})` that returns the selected `id`.
- Filter out entries whose `id.toLowerCase()` is in `exclude`.
- Left pane: distinct categories (e.g. `human`, `creature`, `other`) + an "All" option; right pane: searchable list of `NpcCatalogEntry` showing `id` (subtitle: `category`).
- Tapping a row → `Navigator.pop(context, entry.id)`.

> Keep the widget tree shapes (search `TextField`, `ListTile` per entry) so the
> test's `find.text(id)` + tap works. Match the inventory dialog's theming/widgets.

- [ ] **Step 4: Run NPC dialog test**

Run: `cd apps/goresave && flutter test test/add_npc_dialog_test.dart`
Expected: PASS

- [ ] **Step 5: Write the failing test (entry dialog)**

```dart
// apps/goresave/test/add_knowledge_entry_dialog_test.dart
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:goresave/features/editor/domain/knowledge_catalog.dart';
import 'package:goresave/features/editor/ui/add_knowledge_entry_dialog.dart';

void main() {
  final catalog = KnowledgeCatalog.fromJsonString(
    '[{"id":"Topic_Diego_209799","category":"topic"},'
    '{"id":"ChoiceDiegoGamestart","category":"choice"},'
    '{"id":"Info_FMORGAreyouok","category":"info"}]',
  );

  testWidgets('lists entries, excludes existing, returns selection',
      (tester) async {
    String? picked;
    await tester.pumpWidget(MaterialApp(
      home: Scaffold(
        body: Builder(builder: (context) {
          return ElevatedButton(
            onPressed: () async {
              picked = await showAddKnowledgeEntryDialog(
                context,
                catalog: catalog,
                exclude: {'choicediegogamestart'},
              );
            },
            child: const Text('open'),
          );
        }),
      ),
    ));
    await tester.tap(find.text('open'));
    await tester.pumpAndSettle();

    expect(find.text('Topic_Diego_209799'), findsOneWidget);
    expect(find.text('ChoiceDiegoGamestart'), findsNothing);
    await tester.tap(find.text('Topic_Diego_209799'));
    await tester.pumpAndSettle();
    expect(picked, 'Topic_Diego_209799');
  });
}
```

- [ ] **Step 6: Run to verify it fails**

Run: `cd apps/goresave && flutter test test/add_knowledge_entry_dialog_test.dart`
Expected: FAIL — cannot find `add_knowledge_entry_dialog.dart`

- [ ] **Step 7: Implement `add_knowledge_entry_dialog.dart`**

Same structure as the NPC dialog. Entrypoint
`Future<String?> showAddKnowledgeEntryDialog(BuildContext context, {required KnowledgeCatalog catalog, required Set<String> exclude})`.
Categories fixed: `topic`, `choice`, `info` (+ "All"). Rows show `id`.

- [ ] **Step 8: Run entry dialog test**

Run: `cd apps/goresave && flutter test test/add_knowledge_entry_dialog_test.dart`
Expected: PASS

- [ ] **Step 9: Commit**

```bash
git add apps/goresave/lib/features/editor/ui/add_npc_dialog.dart apps/goresave/lib/features/editor/ui/add_knowledge_entry_dialog.dart apps/goresave/test/add_npc_dialog_test.dart apps/goresave/test/add_knowledge_entry_dialog_test.dart
git commit -m "feat(app): add-NPC and add-knowledge-entry picker dialogs"
```

### Task 10: Wire dialogs into the Knowledge tab

**Files:**
- Modify: `apps/goresave/lib/features/editor/ui/progression_panel.dart` (`_KnowledgeDetail`, ~594-1210)

- [ ] **Step 1: Load catalogs in `_KnowledgeDetail` state**

In `_KnowledgeDetailState`, add fields and load in `initState` (alongside the
existing `_loadCharacters()` at ~674):

```dart
  NpcCatalog? _npcCatalog;
  KnowledgeCatalog? _knowledgeCatalog;

  Future<void> _loadCatalogs() async {
    final npc = await NpcCatalog.loadBundled();
    final know = await KnowledgeCatalog.loadBundled();
    if (!mounted) return;
    setState(() {
      _npcCatalog = npc;
      _knowledgeCatalog = know;
    });
  }
```

Add imports for `npc_catalog.dart`, `knowledge_catalog.dart`, `add_npc_dialog.dart`,
`add_knowledge_entry_dialog.dart`, and call `_loadCatalogs()` in `initState`.

- [ ] **Step 2: Add an "Add NPC" button**

Near the character-list header (left pane of `_KnowledgeDetail`), add a button
that opens the NPC dialog and inserts on selection:

```dart
  Future<void> _addNpc() async {
    final catalog = _npcCatalog;
    if (catalog == null) return;
    final existing = _characters.map((c) => c.name.toLowerCase()).toSet();
    final picked = await showAddNpcDialog(context, catalog: catalog, exclude: existing);
    if (picked == null || !mounted) return;
    final error = await widget.notifier.addKnowledgeCharacter(picked);
    if (!mounted) return;
    if (error != null) {
      setState(() => _addError = 'Add NPC failed: $error');
      return;
    }
    await _loadCharacters();       // refresh list (existing method)
    await _selectCharacter(picked); // select the new NPC (existing method)
  }
```

> Field/method names (`_characters`, `_loadCharacters`, `_selectCharacter`,
> `_addError`) are the existing ones in `_KnowledgeDetailState` — confirm the
> exact names while editing and match them. `_characters` is the loaded
> `KnowledgeCharacter` list; if it is paginated, build `existing` from whatever
> the state already holds (the dialog only needs a best-effort exclude; the core
> rejects true duplicates anyway).

- [ ] **Step 3: Add a "Browse catalog" affordance to entry add**

Next to the existing free-text add field (the `_addController` TextField around
the entry pane), add a button that opens the entry dialog and feeds the existing
`_addEntry` path (`progression_panel.dart:777`):

```dart
  Future<void> _browseAddEntry() async {
    final catalog = _knowledgeCatalog;
    if (catalog == null) return;
    final existing = _entries.entries.map((e) => e.toLowerCase()).toSet();
    final picked = await showAddKnowledgeEntryDialog(
      context, catalog: catalog, exclude: existing,
    );
    if (picked == null || !mounted) return;
    await _addEntry(picked); // existing add path: dup-check + pending edit
  }
```

The free-text field stays as-is for voiceline/non-catalog tokens.

- [ ] **Step 4: Analyze + run the app's test suite**

Run: `cd apps/goresave && flutter analyze`
Expected: No errors.
Run: `cd apps/goresave && flutter test`
Expected: PASS (all tests, incl. the new dialog/loader tests).

- [ ] **Step 5: Manual smoke (optional but recommended)**

Use the `run` skill or `flutter run` to: open a save → Knowledge tab → "Add NPC"
→ pick an NPC not in the save → confirm it appears with an empty entry list →
"Browse catalog" → add a Topic → confirm it lands in pending edits → save.

- [ ] **Step 6: Commit**

```bash
git add apps/goresave/lib/features/editor/ui/progression_panel.dart
git commit -m "feat(app): wire NPC + entry catalog pickers into Knowledge tab"
```

---

## Final verification

- [ ] `cargo test -p goresave_core` — all green
- [ ] `cd apps/goresave && flutter analyze && flutter test` — all green
- [ ] Manual: add a never-before-seen NPC + a Topic entry, save, reload the save, confirm both persist (round-trip through the real codec).
- [ ] Update `gothic-remake-ue4ss-dump` memory: note the two new catalog regen commands alongside the existing item-catalog one.
