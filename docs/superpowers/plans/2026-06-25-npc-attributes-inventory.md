# NPC Attributes & Inventory Editing — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Let the user view and edit every character in a Gothic 1 Remake save — the player and all NPCs — through a shared actor selector on the renamed "Attribute" tab and the "Inventory" tab, including reviving dead NPCs and clearing their kill-bounty.

**Architecture:** Hybrid. NPC attributes reuse the existing generic `private.typed.setValue` edit engine (paths discovered by new `private.npc.*` read commands). The genuinely new core work is a `GameplayTagContainer` add/remove primitive (the tags `State.Dead` / `State.KillBountyGranted` live only inside native tag containers, which no current edit op can touch) and NPC inventory rooting. The frontend lifts the selected actor into shared state so the Attribute and Inventory tabs stay in sync.

**Tech Stack:** Rust (`crates/gore-save`, serde_json, FFI to a Flutter app), Dart/Flutter (`apps/save-editor`). Tests: `cargo test` (Rust), `flutter test` / `flutter analyze` (Dart). Real fixture: `work/decompressed/G1R-001.decompressed.bin` (1484 NPCs, 47 dead, `OC_VLK_Herek_511` alive-with-bounty).

**Design doc:** `docs/superpowers/specs/2026-06-25-npc-attributes-inventory-design.md`

---

## File Structure

**Rust core (`crates/gore-save/src`):**
- `properties.rs` — add `patch_tag_container` primitive + `tag_container_layout` helper (mirrors `container_layout`/`patch_container`, but for the native `GameplayTagContainer` value form `u32 count + count×fstring`).
- `npc.rs` (new module) — NPC record location across the parallel `CharacterStateSaveGameData_*` maps: find an NPC's `Attributes`/`Inventory`/`ActiveEffects` entries by GlobalId; read Health/MaxHealth; detect `isDead`/`hasKillBounty`; enumerate the captured tag containers that hold `State.Dead`/`State.KillBountyGranted`.
- `lib.rs` — new commands `private.npc.list`, `private.npc.attributes`, and new edit kinds `private.npc.setDead`, `private.npc.setKillBounty`; NPC inventory rooting (`npc_inventory_path`, `resolve_inventory_path(actor_id)`), `actorId` threading on the three inventory edit structs.

**Flutter (`apps/save-editor/lib/features/editor`):**
- `domain/actor.dart` (new) — `Actor` model + actor-list page model.
- `domain/editor_notifier.dart` — shared `selectedActor` state + `loadNpcActors`, `loadNpcAttributes`, NPC edit helpers.
- `ui/actor_selector.dart` (new) — shared left sidebar widget (player pinned on top + searchable paginated NPC list).
- `ui/editor_page.dart` — rename Player tab → "Attribute"; wire `ActorSelector` into the Attribute and Inventory tabs; parameterize the attribute/inventory detail panels by `selectedActor`; NPC state section (Dead/KillBounty switches).
- `ui/hero_stats_card.dart` — accept an actor-scoped attribute source.
- `l10n/app_en.arb` (+ other locales) — `tabAttribute`, `npcSwitchDead`, `npcSwitchKillBounty`, etc.

**Phasing:** Phase 1 = Tasks 1–13 (core tag primitive + npc commands + Attribute tab + switches). Phase 2 = Tasks 14–20 (Inventory NPC support). Each phase ends in working, testable software.

---

# Phase 1 — Core tag primitive, NPC commands, Attribute tab

## Task 1: `tag_container_layout` helper (read byte ranges of tags)

A `GameplayTagContainer` is the value of a `StructProperty` (e.g. `CapturedActorTags`) serialized natively as `u32 count` followed by `count` FStrings. To splice a tag we need each tag's byte range. The parsed `StructValue::GameplayTagContainer(Vec<String>)` gives the strings but not offsets, so re-read the value bytes like `container_layout` does.

**Files:**
- Modify: `crates/gore-save/src/properties.rs`
- Test: same file, `#[cfg(test)]` module (alongside `gameplay_tag_container_is_native`).

- [ ] **Step 1: Write the failing test**

Add near the existing `gameplay_tag_container_is_native` test:

```rust
#[test]
fn tag_container_layout_reports_count_and_ranges() {
    // A StructProperty whose value is a native GameplayTagContainer with two tags.
    let mut payload = tag("CapturedActorTags", "StructProperty");
    // struct descriptor: GameplayTagContainer / /Script/GameplayTags, native flag 0x08
    let body = {
        let mut b = Vec::new();
        b.extend_from_slice(&2u32.to_le_bytes()); // count
        b.extend_from_slice(&encode_fstring_value("State.Dead"));
        b.extend_from_slice(&encode_fstring_value("State.KillBountyGranted"));
        b
    };
    let property = build_native_struct_property(&mut payload, "GameplayTagContainer", &body);

    let layout = tag_container_layout(&payload, &property).unwrap();
    assert_eq!(layout.count, 2);
    assert_eq!(layout.tags, vec!["State.Dead", "State.KillBountyGranted"]);
    assert_eq!(layout.element_ranges.len(), 2);
    // Each range decodes back to its tag.
    assert_eq!(
        decode_fstring_at(&payload, layout.element_ranges[1].start),
        "State.KillBountyGranted"
    );
}
```

If `build_native_struct_property` / `decode_fstring_at` test helpers do not exist, add minimal versions in the test module that construct a tagged `StructProperty` with `tag_flags = TAG_FLAG_NATIVE_SERIALIZE` and parse it via the existing `read_property_*` entry, returning the parsed `Property`. Follow the construction already used by `gameplay_tag_container_is_native`.

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p gore-save tag_container_layout_reports_count_and_ranges`
Expected: FAIL — `tag_container_layout` not found.

- [ ] **Step 3: Implement `TagContainerLayout` + `tag_container_layout`**

Add after `container_layout`:

```rust
#[derive(Debug, Clone, PartialEq)]
pub struct TagContainerLayout {
    /// Absolute offset of the u32 tag-count field (== the struct value start).
    pub count_offset: usize,
    pub count: usize,
    pub tags: Vec<String>,
    /// Absolute byte range of each tag's FString within the payload.
    pub element_ranges: Vec<core::ops::Range<usize>>,
}

/// Byte layout of a native `GameplayTagContainer` StructProperty value
/// (`u32 count` + `count` FStrings). Mirrors `container_layout` for the tag form.
pub fn tag_container_layout(
    payload: &[u8],
    property: &Property,
) -> Result<TagContainerLayout, CoreError> {
    let is_tag_container = property.type_name == "StructProperty"
        && property
            .descriptor
            .struct_type
            .as_ref()
            .map(|(t, _)| t == "GameplayTagContainer")
            .unwrap_or(false);
    if !is_tag_container {
        return Err(CoreError::InvalidRequest(
            "tag container edits require a GameplayTagContainer StructProperty target".to_string(),
        ));
    }
    let end = property
        .value_offset
        .checked_add(property.value_size)
        .filter(|end| *end <= payload.len())
        .ok_or_else(|| CoreError::Parse("tag container value out of bounds".to_string()))?;
    let mut r = Reader::new(&payload[property.value_offset..end], property.value_offset);
    let count_offset = r.abs_pos();
    let count = r.u32()? as usize;
    let mut tags = Vec::with_capacity(count.min(1 << 16));
    let mut element_ranges = Vec::with_capacity(count.min(1 << 16));
    for _ in 0..count {
        let start = r.abs_pos();
        tags.push(r.fstring()?);
        element_ranges.push(start..r.abs_pos());
    }
    if r.remaining() != 0 {
        return Err(CoreError::Parse(format!(
            "tag container left {} bytes after {count} tags",
            r.remaining()
        )));
    }
    Ok(TagContainerLayout { count_offset, count, tags, element_ranges })
}
```

(Adapt field accessors — `property.descriptor.struct_type` — to the actual `Property`/descriptor shape used elsewhere in this file; `read_struct_value`'s caller at the `"StructProperty"` arm shows how `struct_type` is reached.)

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p gore-save tag_container_layout_reports_count_and_ranges`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/gore-save/src/properties.rs
git commit -m "feat(core): tag_container_layout for native GameplayTagContainer"
```

---

## Task 2: `patch_tag_container` primitive (add/remove a tag, cascade sizes)

**Files:**
- Modify: `crates/gore-save/src/properties.rs`
- Test: same file test module.

- [ ] **Step 1: Write the failing tests**

```rust
#[test]
fn patch_tag_container_removes_tag_and_fixes_size_chain() {
    let (mut payload, target, enclosing) = tag_container_fixture(&[
        "State.Dead",
        "State.KillBountyGranted",
    ]);
    patch_tag_container(
        &mut payload,
        &target,
        &enclosing,
        &TagEdit::Remove("State.Dead".to_string()),
    )
    .unwrap();
    // Re-parse from scratch: count is 1, only KillBountyGranted remains, all
    // enclosing size fields consistent (full re-parse consumes every byte).
    let reparsed = reparse_tag_container(&payload);
    assert_eq!(reparsed, vec!["State.KillBountyGranted"]);
}

#[test]
fn patch_tag_container_remove_missing_tag_errors_and_leaves_payload() {
    let (mut payload, target, enclosing) = tag_container_fixture(&["State.Dead"]);
    let before = payload.clone();
    let err = patch_tag_container(
        &mut payload,
        &target,
        &enclosing,
        &TagEdit::Remove("State.KillBountyGranted".to_string()),
    );
    assert!(err.is_err());
    assert_eq!(payload, before, "failed remove must leave payload untouched");
}

#[test]
fn patch_tag_container_adds_tag_and_fixes_size_chain() {
    let (mut payload, target, enclosing) = tag_container_fixture(&["State.Dead"]);
    patch_tag_container(
        &mut payload,
        &target,
        &enclosing,
        &TagEdit::Add("State.KillBountyGranted".to_string()),
    )
    .unwrap();
    let reparsed = reparse_tag_container(&payload);
    assert_eq!(reparsed, vec!["State.Dead", "State.KillBountyGranted"]);
}
```

`tag_container_fixture(tags)` returns `(payload, Property /* the GameplayTagContainer StructProperty */, Vec<usize> /* enclosing size fields */)` built by wrapping the tag container inside at least one enclosing sized struct (so the cascade is exercised). `reparse_tag_container(payload)` re-parses and returns the tag list. Build both from the existing parse entry points; reuse `tag_container_layout` for assertions.

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p gore-save patch_tag_container`
Expected: FAIL — `patch_tag_container` / `TagEdit` not found.

- [ ] **Step 3: Implement `TagEdit` + `patch_tag_container`**

```rust
#[derive(Debug, Clone, PartialEq)]
pub enum TagEdit {
    Add(String),
    Remove(String),
}

/// Add or remove one tag in a native `GameplayTagContainer` StructProperty,
/// updating its `u32` count, the struct size field, and every enclosing size
/// field. Mirrors `patch_container`'s splice/size discipline: compute all
/// writes first, mutate once, leave the payload untouched on any error.
pub fn patch_tag_container(
    payload: &mut Vec<u8>,
    target: &Property,
    enclosing_size_fields: &[usize],
    edit: &TagEdit,
) -> Result<(), CoreError> {
    let layout = tag_container_layout(payload, target)?;
    let (remove_range, insert_at, insert_bytes, count_delta): (
        Option<core::ops::Range<usize>>,
        usize,
        Vec<u8>,
        i64,
    ) = match edit {
        TagEdit::Add(tag) => {
            if layout.tags.iter().any(|t| t == tag) {
                return Err(CoreError::InvalidRequest(format!(
                    "tag container already contains {tag:?}"
                )));
            }
            let end = target.value_offset + target.value_size;
            (None, end, encode_fstring_value(tag), 1)
        }
        TagEdit::Remove(tag) => {
            let index = layout
                .tags
                .iter()
                .position(|t| t == tag)
                .ok_or_else(|| CoreError::Parse(format!("tag container does not contain {tag:?}")))?;
            let range = layout.element_ranges[index].clone();
            (Some(range.clone()), range.start, Vec::new(), -1)
        }
    };
    let removed = remove_range.as_ref().map_or(0, |r| r.len());
    let delta = insert_bytes.len() as i64 - removed as i64;
    let new_count = u32::try_from(layout.count as i64 + count_delta)
        .map_err(|_| CoreError::Parse("tag container count underflow".to_string()))?;
    let new_size = u32::try_from(target.value_size as i64 + delta)
        .map_err(|_| CoreError::Parse("tag container size would leave the u32 range".to_string()))?;

    let mut writes = Vec::with_capacity(enclosing_size_fields.len() + 2);
    if target.value_offset < 5 {
        return Err(CoreError::Parse("tag container tag offset underflow".to_string()));
    }
    writes.push((target.size_field_offset(), new_size));
    for &offset in enclosing_size_fields {
        if offset + 4 > target.value_offset {
            return Err(CoreError::Parse(format!(
                "enclosing size field at 0x{offset:x} does not precede the patch target"
            )));
        }
        let old = u32::from_le_bytes(payload[offset..offset + 4].try_into().unwrap());
        let updated = u32::try_from(i64::from(old) + delta).map_err(|_| {
            CoreError::Parse(format!(
                "enclosing size field at 0x{offset:x} would leave the u32 range"
            ))
        })?;
        writes.push((offset, updated));
    }
    writes.push((layout.count_offset, new_count));
    for (offset, value) in writes {
        payload[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
    }
    match remove_range {
        Some(range) => {
            payload.splice(range, core::iter::empty());
        }
        None => {
            payload.splice(insert_at..insert_at, insert_bytes);
        }
    }
    Ok(())
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p gore-save patch_tag_container`
Expected: PASS (all three).

- [ ] **Step 5: Commit**

```bash
git add crates/gore-save/src/properties.rs
git commit -m "feat(core): patch_tag_container add/remove with size cascade"
```

---

## Task 3: NPC record locator module (`npc.rs`)

Resolve an NPC's records by GlobalId across the parallel `CharacterStateSaveGameData_*` maps and read its state.

**Files:**
- Create: `crates/gore-save/src/npc.rs`
- Modify: `crates/gore-save/src/lib.rs` (add `mod npc;`)
- Test: `crates/gore-save/src/npc.rs` `#[cfg(test)]` using `work/decompressed/G1R-001.decompressed.bin`.

- [ ] **Step 1: Write the failing test**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::properties::parse_private_root;

    fn fixture_root() -> crate::properties::RootObject {
        let data = std::fs::read("../../work/decompressed/G1R-001.decompressed.bin")
            .expect("fixture present");
        parse_private_root(&data).expect("parse")
    }

    #[test]
    fn lists_1484_npcs_with_47_dead() {
        let root = fixture_root();
        let npcs = list_npcs(&root).unwrap();
        assert_eq!(npcs.len(), 1484);
        let dead = npcs.iter().filter(|n| n.is_dead).count();
        assert_eq!(dead, 47);
    }

    #[test]
    fn herek_is_alive_with_killbounty() {
        let root = fixture_root();
        let npcs = list_npcs(&root).unwrap();
        let herek = npcs
            .iter()
            .find(|n| n.id == "OC_VLK_Herek_511-WorldPointActor_Herek")
            .expect("Herek present");
        assert!(!herek.is_dead);
        assert!(herek.has_kill_bounty);
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p gore-save -- npc::tests`
Expected: FAIL — module/functions not found.

- [ ] **Step 3: Implement the locator**

Implement in `npc.rs`:

```rust
use crate::properties::{self, Property, PropertyValue, RootObject, StructValue};

#[derive(Debug, Clone, serde::Serialize)]
pub struct NpcSummary {
    pub id: String,
    pub is_dead: bool,
    pub has_kill_bounty: bool,
    pub hp: Option<f32>,
    pub max_hp: Option<f32>,
}

/// All top-level maps whose values are `CharacterStateSaveGameData_<Aspect>`,
/// keyed by GlobalId. Walk the parsed tree to collect them.
struct CharMaps<'a> {
    attributes: std::collections::HashMap<String, &'a Property>, // value = the map-entry value property
    active_effects: std::collections::HashMap<String, &'a Property>,
    inventory: std::collections::HashMap<String, &'a Property>,
}
```

- Walk `root.properties` recursively (reuse the same traversal `count_properties`/`walk` style already in `properties.rs`) to find every `MapProperty` whose entry values are structs of type `CharacterStateSaveGameData_Attributes` / `_ActiveEffects` / `_Inventory`. Key each entry by `map_key_to_string`.
- `list_npcs(root)` iterates the `attributes` map keys (the canonical 1484), and for each:
  - Read Health/MaxHealth from the `_Attributes` value: drill `AttributeSetsByClass` map → key `Health`/`MaxHealth` → `GameplayAttributeData` → `BaseValue`/`CurrentValue` (FloatProperty). (Mirror the Python walk in `work/rd6.py`.)
  - `is_dead = hp == Some(0.0)` (base or current 0).
  - `has_kill_bounty` = the `_ActiveEffects` entry (if any) contains a `GameplayTagContainer` whose tags include `"State.KillBountyGranted"`.

Provide helpers `npc_attributes_entry(root, id) -> Option<&Property>`, `npc_active_effects_entry(root, id) -> Option<&Property>`, `npc_inventory_entry(root, id) -> Option<&Property>` for later tasks.

Add `mod npc;` to `lib.rs`.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p gore-save -- npc::tests`
Expected: PASS (counts 1484 / 47 / Herek).

- [ ] **Step 5: Commit**

```bash
git add crates/gore-save/src/npc.rs crates/gore-save/src/lib.rs
git commit -m "feat(core): NPC record locator (list, dead/bounty detection)"
```

---

## Task 4: `private.npc.list` command

**Files:**
- Modify: `crates/gore-save/src/lib.rs` (command match near `inspect_save`, ~line 386; the read-command block).
- Test: `crates/gore-save/src/lib.rs` test module.

- [ ] **Step 1: Write the failing test**

```rust
#[test]
fn private_npc_list_paginates_and_filters() {
    let json = run_command(
        "private.npc.list",
        json!({ "path": fixture_save_path(), "query": "Herek", "offset": 0, "limit": 50 }),
    );
    assert_eq!(json["ok"], true);
    let items = json["data"]["npcs"].as_array().unwrap();
    assert!(items.iter().any(|n| n["id"]
        .as_str()
        .unwrap()
        .contains("Herek")));
    assert_eq!(json["data"]["total"].as_u64().unwrap() >= 1, true);
}
```

Use whatever existing test harness invokes a command end-to-end (the difficulty/inventory tests show the pattern — `goresave_execute` JSON in / JSON out, or the internal `handle_command`). `fixture_save_path()` points at a real `.sav` (or the decompressed fixture via the same path other private tests use).

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p gore-save private_npc_list_paginates_and_filters`
Expected: FAIL — unknown command.

- [ ] **Step 3: Implement command**

Add a `"private.npc.list" => { ... }` arm in the read-command match. Load + decompress the save to the private payload (reuse the same helper `inspect_save` / progression commands use to get the parsed private `RootObject`), call `npc::list_npcs(&root)`, localize is out of scope here (frontend localizes), then apply `query` (case-insensitive substring on `id`), sort by `id`, paginate by `offset`/`limit`, and return:

```rust
Ok(json!({
    "npcs": page,        // Vec<NpcSummary>
    "total": filtered_len,
    "offset": offset,
    "limit": limit,
}))
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p gore-save private_npc_list_paginates_and_filters`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/gore-save/src/lib.rs
git commit -m "feat(core): private.npc.list command (paginated, filtered)"
```

---

## Task 5: `private.npc.attributes` command (rows + setValue paths)

**Files:**
- Modify: `crates/gore-save/src/npc.rs`, `crates/gore-save/src/lib.rs`
- Test: `lib.rs` test module.

- [ ] **Step 1: Write the failing test**

```rust
#[test]
fn private_npc_attributes_returns_health_with_typed_path() {
    let json = run_command(
        "private.npc.attributes",
        json!({ "path": fixture_save_path(), "id": "OC_VLK_Herek_511-WorldPointActor_Herek" }),
    );
    assert_eq!(json["ok"], true);
    let rows = json["data"]["attributes"].as_array().unwrap();
    let health = rows.iter().find(|r| r["key"] == "Health").unwrap();
    // Path is a typed path usable by private.typed.setValue.
    assert!(health["basePath"].as_array().is_some());
    assert!(health["base"].as_f64().is_some());
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p gore-save private_npc_attributes_returns_health_with_typed_path`
Expected: FAIL — unknown command.

- [ ] **Step 3: Implement**

In `npc.rs`, add `npc_attributes(root, id) -> Result<Vec<NpcAttributeRow>, CoreError>` that walks the NPC's `_Attributes` entry and, for each `AttributeSetsByClass` attribute (Health, MaxHealth, Mana, Strength, …), emits:

```rust
#[derive(serde::Serialize)]
pub struct NpcAttributeRow {
    pub key: String,                  // "Health"
    pub base: Option<f32>,
    pub current: Option<f32>,
    pub base_path: Vec<String>,       // full typed path to BaseValue from private root
    pub current_path: Vec<String>,    // full typed path to CurrentValue
}
```

Build the typed path from the private root down to each `BaseValue`/`CurrentValue` FloatProperty, using `{key}` map-key segments and `[i]` index segments exactly as `parse_path` consumes them (see `resolve_in_value` map/array arms). The frontend will send these paths back through `private.typed.setValue` — no NPC-specific edit command is needed for attributes.

Wire `"private.npc.attributes" => { ... }` in lib.rs returning `{ "attributes": rows }`.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p gore-save private_npc_attributes_returns_health_with_typed_path`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/gore-save/src/npc.rs crates/gore-save/src/lib.rs
git commit -m "feat(core): private.npc.attributes returns rows + setValue paths"
```

---

## Task 6: `private.npc.setDead` edit (HP bundle + State.Dead removal)

**Files:**
- Modify: `crates/gore-save/src/npc.rs` (apply logic), `crates/gore-save/src/lib.rs` (edit parse + apply dispatch).
- Test: `lib.rs` test module (roundtrip on real fixture).

- [ ] **Step 1: Write the failing test**

```rust
#[test]
fn set_dead_false_revives_npc_and_strips_state_dead() {
    // Pick a real dead NPC from the fixture.
    let dead_id = "Lizard-WP_EF_SCSLOPE_LIZARD_SPAWN_01-1";
    let payload = decompressed_fixture(); // Vec<u8> of the private payload
    let mut edited = payload.clone();
    apply_set_dead(&mut edited, dead_id, false).unwrap();

    let root = properties::parse_private_root(&edited).unwrap();
    let npcs = npc::list_npcs(&root).unwrap();
    let n = npcs.iter().find(|n| n.id == dead_id).unwrap();
    assert!(!n.is_dead, "HP restored to max");
    assert_eq!(n.hp, n.max_hp);
    // State.Dead gone from every captured container of this NPC.
    assert!(!npc::active_effects_has_tag(&root, dead_id, "State.Dead"));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p gore-save set_dead_false_revives_npc_and_strips_state_dead`
Expected: FAIL — `apply_set_dead` not found.

- [ ] **Step 3: Implement `apply_set_dead`**

In `npc.rs`:

```rust
pub fn apply_set_dead(payload: &mut Vec<u8>, id: &str, dead: bool) -> Result<(), CoreError> {
    // Re-parse before each structural edit, because splices invalidate offsets.
    if dead {
        set_health(payload, id, 0.0)?;            // kill: HP 0; game re-derives State.Dead from HP 0
        return Ok(());
    }
    // revive: HP -> MaxHealth, then strip State.Dead from all captured containers
    let max = read_max_health(payload, id)?;
    set_health(payload, id, max)?;
    remove_tag_everywhere(payload, id, "State.Dead")?;
    Ok(())
}
```

- `set_health(payload, id, value)`: resolve the NPC's `Health` `BaseValue` and `CurrentValue` typed paths (`npc_attributes`), patch each with `patch_scalar` (fixed-size float — no splice, no re-parse needed between the two).
- `remove_tag_everywhere(payload, id, tag)`: loop — re-parse `parse_private_root`, find the first `GameplayTagContainer` under the NPC's `_ActiveEffects` entry still containing `tag`, resolve its `ResolvedChain` (target + `enclosing_size_fields`) via the path from root, call `patch_tag_container(.., TagEdit::Remove(tag))`; repeat until none remain. Re-parsing each iteration keeps offsets valid after the splice.
- `active_effects_has_tag(root, id, tag)` test helper in `npc.rs`.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p gore-save set_dead_false_revives_npc_and_strips_state_dead`
Expected: PASS.

- [ ] **Step 5: Wire the edit command + commit**

Add `"private.npc.setDead"` to the edit-parse match (near `private.inventory.*`, lib.rs ~4563) parsing `value = { id: String, dead: bool }`, and to the apply side calling `npc::apply_set_dead`. Add a parse test for the `{id, dead}` shape.

```bash
git add crates/gore-save/src/npc.rs crates/gore-save/src/lib.rs
git commit -m "feat(core): private.npc.setDead (revive bundle: HP + strip State.Dead)"
```

---

## Task 7: `private.npc.setKillBounty` edit

**Files:**
- Modify: `crates/gore-save/src/npc.rs`, `crates/gore-save/src/lib.rs`
- Test: `lib.rs` test module (Herek roundtrip).

- [ ] **Step 1: Write the failing test**

```rust
#[test]
fn set_kill_bounty_false_clears_tag_for_herek() {
    let id = "OC_VLK_Herek_511-WorldPointActor_Herek";
    let mut edited = decompressed_fixture();
    apply_set_kill_bounty(&mut edited, id, false).unwrap();
    let root = properties::parse_private_root(&edited).unwrap();
    assert!(!npc::active_effects_has_tag(&root, id, "State.KillBountyGranted"));
    // HP untouched.
    let n = npc::list_npcs(&root).unwrap().into_iter().find(|n| n.id == id).unwrap();
    assert!(!n.is_dead);
}

#[test]
fn set_kill_bounty_true_without_active_effects_is_warned_noop() {
    // An alive NPC with no ActiveEffects entry: cannot host the tag.
    let id = some_npc_without_active_effects(); // pick from fixture
    let mut edited = decompressed_fixture();
    let before = edited.clone();
    let res = apply_set_kill_bounty(&mut edited, id, true);
    assert!(matches!(res, Ok(SetTagOutcome::NoHostWarned)));
    assert_eq!(edited, before);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p gore-save set_kill_bounty`
Expected: FAIL.

- [ ] **Step 3: Implement `apply_set_kill_bounty`**

```rust
pub enum SetTagOutcome { Applied, NoHostWarned, AlreadyInState }

pub fn apply_set_kill_bounty(
    payload: &mut Vec<u8>,
    id: &str,
    granted: bool,
) -> Result<SetTagOutcome, CoreError> {
    if granted {
        // Add to the first captured container; if the NPC has none, warn no-op.
        match first_captured_container_path(payload, id)? {
            Some(_) => { add_tag_first_container(payload, id, "State.KillBountyGranted")?; Ok(SetTagOutcome::Applied) }
            None => Ok(SetTagOutcome::NoHostWarned),
        }
    } else {
        remove_tag_everywhere(payload, id, "State.KillBountyGranted")?;
        Ok(SetTagOutcome::Applied)
    }
}
```

Reuse `remove_tag_everywhere` (Task 6). `add_tag_first_container` mirrors it but uses `TagEdit::Add` on the first captured container of the NPC.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p gore-save set_kill_bounty`
Expected: PASS.

- [ ] **Step 5: Wire command + commit**

Add `"private.npc.setKillBounty"` parse/apply (`value = { id, granted: bool }`). Surface `NoHostWarned` as a non-fatal warning field in the command result.

```bash
git add crates/gore-save/src/npc.rs crates/gore-save/src/lib.rs
git commit -m "feat(core): private.npc.setKillBounty toggle"
```

---

## Task 8: Expose new commands to the writable list + FFI smoke test

**Files:**
- Modify: `crates/gore-save/src/lib.rs` (the `inspect_save` private summary `writable` list near ~2837 and ~3023; add the `private.npc.*` editors so the frontend can feature-detect).
- Test: `lib.rs` test asserting the writable list includes the new edits when a save has NPCs.

- [ ] **Step 1: Write the failing test**

```rust
#[test]
fn inspect_save_advertises_npc_edits() {
    let json = run_command("inspect_save", json!({ "path": fixture_save_path(), "includePrivate": true }));
    let writable = json["data"]["privateNpc"]["writable"].as_array().unwrap();
    let names: Vec<_> = writable.iter().map(|v| v.as_str().unwrap()).collect();
    assert!(names.contains(&"private.npc.setDead"));
    assert!(names.contains(&"private.npc.setKillBounty"));
}
```

- [ ] **Step 2: Run → fail.** `cargo test -p gore-save inspect_save_advertises_npc_edits` → FAIL.
- [ ] **Step 3:** Add a `privateNpc` block to the `inspect_save` result advertising `["private.npc.setDead","private.npc.setKillBounty"]` and that `private.typed.setValue` covers NPC attributes, plus `hasNpcs: true` when `list_npcs` is non-empty.
- [ ] **Step 4: Run → pass.**
- [ ] **Step 5: Commit**

```bash
git add crates/gore-save/src/lib.rs
git commit -m "feat(core): advertise npc edits in inspect_save"
```

---

## Task 9: Frontend — shared `Actor` model + selection state

**Files:**
- Create: `apps/save-editor/lib/features/editor/domain/actor.dart`
- Modify: `apps/save-editor/lib/features/editor/domain/editor_notifier.dart`
- Test: `apps/save-editor/test/` (new `actor_selection_test.dart`)

- [ ] **Step 1: Write the failing test**

```dart
test('selecting an actor updates shared state and notifies', () {
  final notifier = makeEditorNotifier(); // existing test helper / minimal stub
  notifier.selectActor(const Actor.npc(id: 'Lizard-1', name: 'Lizard'));
  expect(notifier.state.selectedActor.kind, ActorKind.npc);
  expect(notifier.state.selectedActor.id, 'Lizard-1');
});
```

- [ ] **Step 2: Run → fail.** `flutter test test/actor_selection_test.dart` → FAIL (Actor/selectActor missing).
- [ ] **Step 3:** Implement `actor.dart`:

```dart
enum ActorKind { player, npc }

class Actor {
  const Actor.player() : kind = ActorKind.player, id = null, name = 'Player';
  const Actor.npc({required String this.id, required this.name}) : kind = ActorKind.npc;
  final ActorKind kind;
  final String? id;
  final String name;
  bool get isPlayer => kind == ActorKind.player;
}
```

Add `Actor selectedActor` (default `Actor.player()`) to the editor state + `void selectActor(Actor a)` on the notifier (copyWith + notify). Default selection is the player so the Attribute tab behaves as today on load.

- [ ] **Step 4: Run → pass.**
- [ ] **Step 5: Commit**

```bash
git add apps/save-editor/lib/features/editor/domain/actor.dart apps/save-editor/lib/features/editor/domain/editor_notifier.dart apps/save-editor/test/actor_selection_test.dart
git commit -m "feat(editor): shared Actor selection state"
```

---

## Task 10: Frontend — `ActorSelector` sidebar widget

Mirror the Events-tab character list (search box + paginated `loadMemoryCharacters`-style fetch) but prepend a pinned **Player** entry. Source NPCs from `private.npc.list`.

**Files:**
- Create: `apps/save-editor/lib/features/editor/ui/actor_selector.dart`
- Modify: `editor_notifier.dart` (add `loadNpcActors({query, offset, limit})` calling `private.npc.list`)
- Test: widget test `apps/save-editor/test/actor_selector_test.dart`

- [ ] **Step 1: Write the failing widget test**

```dart
testWidgets('ActorSelector shows Player on top and lists NPCs', (tester) async {
  await tester.pumpWidget(wrap(ActorSelector(
    notifier: fakeNotifierWith(npcs: ['Lizard', 'Herek']),
    selected: const Actor.player(),
    onSelect: (_) {},
  )));
  expect(find.text('Player'), findsOneWidget);
  expect(find.text('Lizard'), findsOneWidget);
});
```

- [ ] **Step 2: Run → fail.** `flutter test test/actor_selector_test.dart` → FAIL.
- [ ] **Step 3:** Implement `ActorSelector`: a `Column` with a search `TextField`, a pinned `ListTile('Player')` (selected highlight), then a paginated `ListView` of NPC `ListTile`s (localized name via `localizedProgressionName`, subtitle = dead/bounty badges from the `private.npc.list` summary). Reuse the pagination state machine from `progression_panel.dart`'s memory-character list (offset/limit, load-more on scroll). Add `loadNpcActors` to the notifier wrapping `_execute('private.npc.list', ...)`.
- [ ] **Step 4: Run → pass.**
- [ ] **Step 5: Commit**

```bash
git add apps/save-editor/lib/features/editor/ui/actor_selector.dart apps/save-editor/lib/features/editor/domain/editor_notifier.dart apps/save-editor/test/actor_selector_test.dart
git commit -m "feat(editor): ActorSelector sidebar (player + paginated NPCs)"
```

---

## Task 11: Frontend — rename Player tab to "Attribute"

**Files:**
- Modify: `apps/save-editor/lib/l10n/app_en.arb` (+ each other `app_*.arb`), `apps/save-editor/lib/features/editor/ui/editor_page.dart` (tab label uses `l10n.tabAttribute`).
- Test: existing widget/golden tests referencing the tab label, if any; otherwise `flutter analyze`.

- [ ] **Step 1:** Add `"tabAttribute": "Attribute"` to `app_en.arb` (and translations to the other locale arbs — copy English if unknown, leave a translator note). Keep `tabPlayer` if referenced elsewhere or remove all uses.
- [ ] **Step 2:** Replace `l10n.tabPlayer` with `l10n.tabAttribute` at the Player tab definition (`editor_page.dart`, the `Tab(icon: Icons.person_outline, ...)` entry).
- [ ] **Step 3:** Run codegen: `flutter gen-l10n` (or the project's l10n build step).
- [ ] **Step 4: Verify.** `flutter analyze` → no errors.
- [ ] **Step 5: Commit**

```bash
git add apps/save-editor/lib/l10n apps/save-editor/lib/features/editor/ui/editor_page.dart
git commit -m "feat(editor): rename Player tab to Attribute"
```

---

## Task 12: Frontend — wire ActorSelector into Attribute tab + NPC attributes

**Files:**
- Modify: `editor_page.dart` (Attribute tab body → `Row(children: [ActorSelector, detail])`), `hero_stats_card.dart` (accept an actor-scoped attribute source), `editor_notifier.dart` (`loadNpcAttributes(id)` → `private.npc.attributes`).
- Test: widget test that selecting an NPC loads its attribute rows.

- [ ] **Step 1: Write the failing widget test**

```dart
testWidgets('selecting an NPC shows its Health row', (tester) async {
  final notifier = fakeNotifierWithNpcAttributes('Lizard-1', health: 0, maxHealth: 80);
  await tester.pumpWidget(wrap(AttributeTab(notifier: notifier)));
  await tester.tap(find.text('Lizard'));
  await tester.pumpAndSettle();
  expect(find.text('Health'), findsOneWidget);
  expect(find.textContaining('0'), findsWidgets);
});
```

- [ ] **Step 2: Run → fail.**
- [ ] **Step 3:** Wrap the Attribute tab body in a `Row`: left `ActorSelector` (bound to `notifier.state.selectedActor` + `notifier.selectActor`), right the detail. When `selectedActor.isPlayer`, render the existing `HeroStatsCard` player path unchanged. When an NPC, fetch `loadNpcAttributes(id)` and feed the rows into `HeroStatsCard` (or an actor-scoped variant) whose edit callback emits `private.typed.setValue` with `edit.path = row.basePath/currentPath` — identical to the player flow, just different paths.
- [ ] **Step 4: Run → pass.** Then `flutter analyze`.
- [ ] **Step 5: Commit**

```bash
git add apps/save-editor/lib/features/editor
git commit -m "feat(editor): NPC attribute editing in Attribute tab"
```

---

## Task 13: Frontend — Dead / KillBounty switches (NPC only)

**Files:**
- Modify: `editor_page.dart` (state section above attribute groups, NPC only), `editor_notifier.dart` (build `private.npc.setDead` / `private.npc.setKillBounty` pending edits), `l10n` arbs.
- Test: widget test that toggling Dead queues the right edit JSON.

- [ ] **Step 1: Write the failing test**

```dart
test('toggling Dead off queues setDead(false)', () {
  final notifier = fakeNotifierWithNpc('Lizard-1', isDead: true);
  notifier.setNpcDead('Lizard-1', false);
  final edit = notifier.pendingEditFor('npc.state').edits.single;
  expect(edit['path'], 'private.npc.setDead');
  expect(edit['value'], {'id': 'Lizard-1', 'dead': false});
});
```

- [ ] **Step 2: Run → fail.**
- [ ] **Step 3:** Add a state section rendered only when `selectedActor.kind == npc`: HP/MaxHP text, two `SwitchListTile`s (`l10n.npcSwitchDead`, `l10n.npcSwitchKillBounty`) whose initial values come from the `private.npc.list` summary for that id. `onChanged` calls `notifier.setNpcDead(id, value)` / `setNpcKillBounty(id, value)`, which register a `PendingSaveEdit` under keys `npc.state` with edit JSON `{'path': 'private.npc.setDead', 'value': {'id': id, 'dead': value}}` (and the bounty equivalent). Surface the core's `NoHostWarned` warning (from the write_save result) as a snackbar.
- [ ] **Step 4: Run → pass.** Then `flutter analyze`.
- [ ] **Step 5: Commit**

```bash
git add apps/save-editor/lib/features/editor apps/save-editor/lib/l10n
git commit -m "feat(editor): NPC Dead/KillBounty switches"
```

---

## Task 13b: Phase 1 end-to-end verification

- [ ] **Step 1:** `cargo test -p gore-save` → all green.
- [ ] **Step 2:** `cd apps/save-editor && flutter test && flutter analyze` → green.
- [ ] **Step 3:** Build the DLL + run the app (per `run` skill / `python build.py`), load a real save, select a dead NPC, toggle Dead off, save, reload, confirm the NPC reads alive (HP = max, no `State.Dead`). Confirm Herek's KillBounty toggle clears. Record the manual result.
- [ ] **Step 4: Commit** any fixups.

---

# Phase 2 — Inventory NPC support

## Task 14: `npc_inventory_path` + `resolve_inventory_path(actor_id)` dispatch

**Files:**
- Modify: `crates/gore-save/src/lib.rs` (near `player_inventory_path` / `resolve_inventory_path`, ~5523–5546), `crates/gore-save/src/npc.rs`.
- Test: `lib.rs` test module.

- [ ] **Step 1: Write the failing test**

```rust
#[test]
fn npc_inventory_path_resolves_for_a_real_npc() {
    let root = fixture_private_root();
    let id = "Lizard-WP_EF_SCSLOPE_LIZARD_SPAWN_01-1";
    let path = resolve_inventory_path(&root, Some(id)).expect("npc inventory path");
    // The resolved path addresses a property whose subtree contains m_Slots.
    let target = properties::resolve(&root.properties, &properties::parse_path(&path).unwrap()).unwrap();
    assert!(subtree_contains_property(target, "m_Slots"));
}

#[test]
fn resolve_inventory_path_none_still_returns_player() {
    let root = fixture_private_root();
    assert!(resolve_inventory_path(&root, None).is_some());
}
```

- [ ] **Step 2: Run → fail** (signature mismatch / function missing).
- [ ] **Step 3:** Change `resolve_inventory_path(root)` → `resolve_inventory_path(root, actor_id: Option<&str>)`:

```rust
fn resolve_inventory_path(root: &RootObject, actor_id: Option<&str>) -> Option<Vec<String>> {
    match actor_id {
        None => player_inventory_path(root)
            .or_else(|| properties::find_property_by_name(root, "m_Inventory").map(|(p, _)| p)),
        Some(id) => npc::npc_inventory_path(root, id),
    }
}
```

Implement `npc::npc_inventory_path(root, id)`: locate the `CharacterStateSaveGameData_Inventory` map, find the entry keyed `id`, and return the full typed path to that entry's inventory container (the property holding `m_InventoryType` / `m_Slots`, mirroring the player `m_Inventory` subtree shape so the existing MainContainer/`m_Slots` traversal in the add/remove apply functions works unchanged). Update all existing call sites of `resolve_inventory_path` to pass `None`.

- [ ] **Step 4: Run → pass.**
- [ ] **Step 5: Commit**

```bash
git add crates/gore-save/src/lib.rs crates/gore-save/src/npc.rs
git commit -m "feat(core): resolve_inventory_path dispatches player vs npc"
```

---

## Task 15: Thread `actorId` through addItem / removeItem

**Files:**
- Modify: `crates/gore-save/src/lib.rs` (the three inventory edit structs + parse fns ~5178–5286 + apply fns ~5796–6217).
- Test: `lib.rs` test module (NPC add/remove roundtrip).

- [ ] **Step 1: Write the failing test**

```rust
#[test]
fn add_then_remove_item_on_npc_roundtrips() {
    let id = "Lizard-WP_EF_SCSLOPE_LIZARD_SPAWN_01-1";
    let mut payload = decompressed_fixture();
    let catalog_item = "ItFo_Cheese"; // a real catalog item-definition class
    apply_private_inventory_add_item_to_payload(
        &mut payload,
        &PrivateInventoryAddItemEdit { path: catalog_item.into(), count: 3, actor_id: Some(id.into()) },
    ).unwrap();
    // The NPC's MainContainer now contains the item.
    assert!(npc_inventory_contains(&payload, id, catalog_item));
    apply_private_inventory_remove_item_to_payload(
        &mut payload,
        &PrivateInventoryRemoveItemEdit { path: catalog_item.into(), actor_id: Some(id.into()) },
    ).unwrap();
    assert!(!npc_inventory_contains(&payload, id, catalog_item));
}
```

- [ ] **Step 2: Run → fail** (structs lack `actor_id`).
- [ ] **Step 3:** Add `actor_id: Option<String>` to `PrivateInventoryAddItemEdit`, `PrivateInventoryRemoveItemEdit`, `PrivateInventoryItemCountEdit`. Parse it from `value.actorId` in the three parse fns. Pass `edit.actor_id.as_deref()` to `resolve_inventory_path` in both apply fns. The slot/item parsing logic is reused as-is.
- [ ] **Step 4: Run → pass.**
- [ ] **Step 5: Commit**

```bash
git add crates/gore-save/src/lib.rs
git commit -m "feat(core): npc-scoped inventory add/remove via actorId"
```

---

## Task 16: NPC item-count via typed setValue routing

The player count edit uses an untyped FString-region scan scoped to "Party ID 0". Rather than build an NPC region detector, route NPC count edits through the typed path to the slot's `m_ItemCount`.

**Files:**
- Modify: `crates/gore-save/src/lib.rs` (`apply_private_inventory_item_count_edit_to_payload` ~6735), `npc.rs`.
- Test: `lib.rs` test module.

- [ ] **Step 1: Write the failing test**

```rust
#[test]
fn set_item_count_on_npc_updates_typed_slot() {
    let id = "Lizard-WP_EF_SCSLOPE_LIZARD_SPAWN_01-1";
    let mut payload = decompressed_fixture();
    let item = first_npc_item(&payload, id); // an item path already in the NPC's MainContainer
    apply_private_inventory_item_count_edit_to_payload(
        &mut payload,
        &PrivateInventoryItemCountEdit { id: None, path: Some(item.clone()), count: 42, actor_id: Some(id.into()) },
    ).unwrap();
    assert_eq!(npc_item_count(&payload, id, &item), Some(42));
}
```

- [ ] **Step 2: Run → fail.**
- [ ] **Step 3:** In `apply_private_inventory_item_count_edit_to_payload`, branch on `edit.actor_id`: when `Some(id)`, resolve the NPC inventory path, find the slot whose `m_ItemDefinition` matches `edit.path`, build the typed path to that slot's `m_ItemCount` IntProperty, and patch it with `patch_scalar` (fixed-size i32). When `None`, keep the existing player FString-scan path untouched.
- [ ] **Step 4: Run → pass.**
- [ ] **Step 5: Commit**

```bash
git add crates/gore-save/src/lib.rs crates/gore-save/src/npc.rs
git commit -m "feat(core): npc item-count edit via typed slot path"
```

---

## Task 17: NPC inventory read for display

**Files:**
- Modify: `crates/gore-save/src/lib.rs` (extend `inspect_save` to accept optional `actorId`, or add `private.npc.inventory`), `npc.rs`.
- Test: `lib.rs` test module.

- [ ] **Step 1: Write the failing test**

```rust
#[test]
fn npc_inventory_summary_lists_items() {
    let json = run_command(
        "private.npc.inventory",
        json!({ "path": fixture_save_path(), "id": "Lizard-WP_EF_SCSLOPE_LIZARD_SPAWN_01-1" }),
    );
    assert_eq!(json["ok"], true);
    assert!(json["data"]["items"].as_array().unwrap().len() > 0);
}
```

- [ ] **Step 2: Run → fail.**
- [ ] **Step 3:** Add `private.npc.inventory` returning the same `PrivateInventorySummary` shape the player path returns (reuse the existing slot-reading helpers, scoped to the NPC inventory container via `npc_inventory_path`). This keeps the frontend inventory card identical for player and NPC.
- [ ] **Step 4: Run → pass.**
- [ ] **Step 5: Commit**

```bash
git add crates/gore-save/src/lib.rs crates/gore-save/src/npc.rs
git commit -m "feat(core): private.npc.inventory summary"
```

---

## Task 18: Frontend — ActorSelector in Inventory tab + actorId on edits

**Files:**
- Modify: `editor_page.dart` (Inventory tab body → `Row(ActorSelector, inventoryCard)`), `editor_models.dart` (add `actorId` to the three inventory edit classes + `toEditJson`), `editor_notifier.dart` (load NPC inventory when actor is NPC).
- Test: widget + unit tests.

- [ ] **Step 1: Write the failing test**

```dart
test('npc inventory edit includes actorId', () {
  final change = InventoryItemCountChange(id: null, path: 'ItFo_Cheese', count: 5, actorId: 'Lizard-1');
  expect(change.toEditJson()['value']['actorId'], 'Lizard-1');
});
```

- [ ] **Step 2: Run → fail.**
- [ ] **Step 3:** Add `final String? actorId;` to `InventoryItemCountChange`, `InventoryItemAdd`, `InventoryItemRemove`; include `if (actorId != null) 'actorId': actorId` inside the `value` map of each `toEditJson`. In the Inventory tab, prepend the shared `ActorSelector`; when `selectedActor` is an NPC, load via `private.npc.inventory` and set `actorId` on every queued edit in `_pushInventoryPending`. Player path stays `actorId: null`.
- [ ] **Step 4: Run → pass.** Then `flutter analyze`.
- [ ] **Step 5: Commit**

```bash
git add apps/save-editor/lib/features/editor
git commit -m "feat(editor): NPC inventory editing with shared actor selector"
```

---

## Task 19: Phase 2 end-to-end verification

- [ ] **Step 1:** `cargo test -p gore-save` green; `flutter test && flutter analyze` green.
- [ ] **Step 2:** Build DLL + run app. Select an NPC, add an item to its inventory, change a count, remove an item; save; reload; confirm persistence. Confirm selecting the same NPC in Attribute and Inventory stays in sync. Confirm player inventory editing is unchanged (regression).
- [ ] **Step 3:** `validate_roundtrip` on the edited save confirms a clean codec roundtrip.
- [ ] **Step 4: Commit** any fixups.

---

## Task 20: Final review pass

- [ ] **Step 1:** Re-read the spec; confirm every section maps to a task (actor selector ✓, Attribute rename ✓, NPC attributes ✓, Dead/KillBounty switches ✓, inventory NPC ✓, asymmetric ON ✓, out-of-scope loot untouched ✓).
- [ ] **Step 2:** Update memory `goresave-respawn-mechanism` with the final command names and any deviations discovered during implementation.
- [ ] **Step 3:** Open the PR via the project's review-ready PR flow.

---

## Notes / risks for the implementer

- **Re-parse after every structural splice.** Tag removals and inventory add/remove change byte offsets; always re-`parse_private_root` before resolving the next chain. Scalar patches (HP, item count) are fixed-size and need no re-parse.
- **`{key}` map segments with special chars.** GlobalIds contain `-` and `_` (e.g. `Lizard-WP_EF_SCSLOPE_LIZARD_SPAWN_01-1`). Confirm `parse_path` round-trips such keys; if the `{...}` form is ambiguous, prefer passing pre-built `Vec<String>` paths from the core `npc.*` commands rather than reconstructing them in the frontend.
- **Fixture path.** Tests read `work/decompressed/G1R-001.decompressed.bin` relative to the crate (`../../work/...`). If CI lacks the fixture, gate those tests behind a `#[cfg_attr(not(feature = "real_fixture"), ignore)]` or an env check, matching how other real-save tests in this crate are gated.
- **Asymmetric ON direction** is intentional (spec §5): revive/clear is clean; kill = HP 0; bounty-ON without a host container is a warned no-op.
