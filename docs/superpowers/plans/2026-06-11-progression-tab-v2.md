# Progression Tab v2 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the Progression tab's heuristic string dump with structured, editable quests / dialog-knowledge / memory-events views backed by the typed property engine, including four new generic container edit ops in the Rust core.

**Architecture:** The real progression data lives in the decoded private payload: `QuestDataByClass` (MapProperty, ObjectProperty keys → `SingleQuestSaveGameData` structs with a `CurrentState` EnumProperty), `CharacterKnowledgeByUniqueName` (MapProperty, Name keys → structs with a `Knowledge` SetProperty of Names), and `LongTermMemoryByGlobalId` (MapProperty, Name keys → structs with a `MemorizedEvents` ArrayProperty of event structs). A new `query_progression` core command walks the typed tree (decode is cached) and serves paged section data. Quest states are written with the existing `private.typed.setValue` (EnumProperty string patch). Set/array membership changes get four new ops — `private.typed.setAdd`, `private.typed.setRemove`, `private.typed.arrayRemove`, `private.typed.arrayDuplicate` — built on a new `patch_container` in `properties.rs` that reuses the proven size-chain fixup discipline of `patch_string`. The Flutter side gets progression models, notifier methods, and a new `ProgressionPanel` (own file, like `hero_stats_card.dart`) following the Inventory-card pending-edit pattern.

**Tech Stack:** Rust (goresave_core), Flutter/Dart (apps/goresave), cargo test, flutter_test.

**Spec:** `docs/superpowers/specs/2026-06-11-progression-tab-design.md`

**Verified groundwork (real save G1R-001, 76.8 MB decoded payload):**
- `QuestDataByClass` entries: key `/Script/Angelscript.Quest_BanditsCamp_BANDITSTRUST` (ObjectProperty), value struct with `CurrentState: EnumProperty EQuestState (ByteProperty underlying)` holding `EQuestState::None|Available|Running|Succeeded|Failed`, plus `StateReachedAtTime` and `TimeLastViewedByPlayer` maps. 707 quest classes present (unstarted = `Available`), so no map-entry insertion is needed.
- Knowledge entries are voiceline/choice Names per NPC unique name (e.g. `OC_STT_Diego`).
- Memory events carry `EventTags` (native `GameplayTagContainer`, 32-tag taxonomy `Memory.*`), `Magnitude` (Float), `Payload` (InstancedStruct), `OptionalClass1/2` (SoftObjectProperty), `position` (Vector), `Time`/`Duration` (struct `InGameTime` → `TotalSeconds` DoubleProperty), `InstigatorGlobalId`/`AffectedCharacterGlobalId` (Name).
- The typed parser already traverses all of this (maps by `{key}`, Object keys addressable since the hero-stats work). `private.typed.setValue` already patches EnumProperty strings with length change + ancestor size fixes.
- Hardcoded sections of the old heuristic (`m_QuestLog`, `m_Knowledge`, `m_ActiveQuests`, …) do not exist in real saves; the heuristic code is deleted, not preserved.

**Conventions used below:**
- Rust tests live in the existing `#[cfg(test)] mod tests` blocks; reuse existing builders (`fstring`, `tag`, `header`, `int_property`, `root`, `compressed_stream_with_one_chunk`, `build_gsav`, `PrefixCodecBackend`, `public_payload`) — add new builders only when missing in that module.
- Run Rust tests with `cargo test -p goresave_core <name>`; full check `cargo test -p goresave_core`.
- Run Dart tests from `apps/goresave` with `flutter test <path>`; static check `flutter analyze`.
- Commit after each green task with a conventional-commit message ending in the Co-Authored-By trailer used by this repo:
  `Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>`

---

### Task 1: Rust core — container layout (element byte ranges)

Container edits need the absolute byte range of each set/array element and the count-field offset. The parsed tree does not record inline element offsets, so re-read the container body with a position-tracking `Reader`.

**Files:**
- Modify: `crates/goresave_core/src/properties.rs` (new public API next to `patch_string`, ~line 808; tests at the bottom)

- [ ] **Step 1: Write the failing tests**

Add to the `tests` module in `properties.rs`. First two new fixture builders (skip any that already exist):

```rust
    fn name_set_property(name: &str, values: &[&str]) -> Vec<u8> {
        let mut body = 0u32.to_le_bytes().to_vec(); // num_to_remove
        body.extend_from_slice(&(values.len() as u32).to_le_bytes());
        for v in values {
            body.extend_from_slice(&fstring(v));
        }
        let mut out = tag(name, "SetProperty");
        out.extend_from_slice(&1u32.to_le_bytes());
        out.extend_from_slice(&fstring("NameProperty"));
        out.extend_from_slice(&header(body.len() as u32, 0));
        out.extend_from_slice(&body);
        out
    }

    fn int_array_property(name: &str, values: &[i32]) -> Vec<u8> {
        let mut body = (values.len() as u32).to_le_bytes().to_vec();
        for v in values {
            body.extend_from_slice(&v.to_le_bytes());
        }
        let mut out = tag(name, "ArrayProperty");
        out.extend_from_slice(&1u32.to_le_bytes());
        out.extend_from_slice(&fstring("IntProperty"));
        out.extend_from_slice(&header(body.len() as u32, 0));
        out.extend_from_slice(&body);
        out
    }

    #[test]
    fn container_layout_reports_set_element_ranges() {
        let payload = root(
            "/Script/Test.Save",
            &name_set_property("Knowledge", &["Voiceline_A", "ChoiceB"]),
        );
        let parsed = parse_private_root(&payload).unwrap();
        let target = &parsed.properties[0];

        let layout = container_layout(&payload, target).unwrap();
        assert_eq!(layout.kind, ContainerKind::Set);
        assert_eq!(layout.inner_type, "NameProperty");
        assert_eq!(layout.count, 2);
        // count field sits after the u32 num_to_remove
        assert_eq!(layout.count_offset, target.value_offset + 4);
        assert_eq!(layout.element_ranges.len(), 2);
        // elements are FStrings: 4-byte length + chars + NUL
        let first = &layout.element_ranges[0];
        assert_eq!(first.start, target.value_offset + 8);
        assert_eq!(first.len(), 4 + "Voiceline_A".len() + 1);
        let second = &layout.element_ranges[1];
        assert_eq!(second.start, first.end);
        assert_eq!(second.end, target.value_offset + target.value_size);
    }

    #[test]
    fn container_layout_reports_array_element_ranges() {
        let payload = root("/Script/Test.Save", &int_array_property("Nums", &[7, 8, 9]));
        let parsed = parse_private_root(&payload).unwrap();
        let target = &parsed.properties[0];

        let layout = container_layout(&payload, target).unwrap();
        assert_eq!(layout.kind, ContainerKind::Array);
        assert_eq!(layout.count, 3);
        assert_eq!(layout.count_offset, target.value_offset);
        assert_eq!(
            layout.element_ranges,
            vec![
                target.value_offset + 4..target.value_offset + 8,
                target.value_offset + 8..target.value_offset + 12,
                target.value_offset + 12..target.value_offset + 16,
            ]
        );
    }

    #[test]
    fn container_layout_rejects_non_container_targets() {
        let payload = root("/Script/Test.Save", &int_property("m_X", 1));
        let parsed = parse_private_root(&payload).unwrap();
        assert!(container_layout(&payload, &parsed.properties[0]).is_err());
    }
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p goresave_core container_layout`
Expected: FAIL to compile — `container_layout`/`ContainerKind` not defined.

- [ ] **Step 3: Implement `container_layout`**

Add after `patch_string` in `properties.rs`:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContainerKind {
    Array,
    Set,
}

/// Byte layout of a Set/Array property's value: where the element-count field
/// sits and the absolute byte range of every element. Computed by re-reading
/// the container body, since the parsed tree does not record inline element
/// offsets.
#[derive(Debug, Clone, PartialEq)]
pub struct ContainerLayout {
    pub kind: ContainerKind,
    pub inner_type: String,
    /// Absolute offset of the u32 element-count field.
    pub count_offset: usize,
    pub count: usize,
    /// Absolute byte range of each element within the payload.
    pub element_ranges: Vec<core::ops::Range<usize>>,
}

pub fn container_layout(
    payload: &[u8],
    property: &Property,
) -> Result<ContainerLayout, CoreError> {
    let kind = match property.type_name.as_str() {
        "ArrayProperty" => ContainerKind::Array,
        "SetProperty" => ContainerKind::Set,
        other => {
            return Err(CoreError::InvalidRequest(format!(
                "container edits require an ArrayProperty or SetProperty target, got {other}"
            )));
        }
    };
    // Instanced-object arrays interleave full object streams; element-level
    // splicing is not supported for them.
    if matches!(property.value, PropertyValue::ObjectInstances(_)) {
        return Err(CoreError::UnsupportedEdit(
            "container edits do not support instanced-object arrays".to_string(),
        ));
    }
    let inner = property
        .descriptor
        .inner
        .as_deref()
        .ok_or_else(|| CoreError::Parse("container property missing inner descriptor".into()))?;
    let end = property
        .value_offset
        .checked_add(property.value_size)
        .filter(|end| *end <= payload.len())
        .ok_or_else(|| CoreError::Parse("container value out of bounds".to_string()))?;
    let mut r = Reader::new(&payload[property.value_offset..end], property.value_offset);
    if kind == ContainerKind::Set {
        let _num_to_remove = r.u32()?;
    }
    let count_offset = r.abs_pos();
    let count = r.u32()? as usize;
    let mut element_ranges = Vec::with_capacity(count.min(1 << 16));
    for _ in 0..count {
        let start = r.abs_pos();
        read_inline_value(&mut r, inner, 0)?;
        element_ranges.push(start..r.abs_pos());
    }
    if r.remaining() != 0 {
        return Err(CoreError::Parse(format!(
            "container body left {} bytes after {count} elements",
            r.remaining()
        )));
    }
    Ok(ContainerLayout {
        kind,
        inner_type: inner.type_name.clone(),
        count_offset,
        count,
        element_ranges,
    })
}
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test -p goresave_core container_layout`
Expected: 3 PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/goresave_core/src/properties.rs
git commit -m "feat(core): container layout with element byte ranges"
```

---

### Task 2: Rust core — `patch_container` (setAdd / setRemove / arrayRemove / arrayDuplicate)

One splice per edit, with the same all-writes-validated-up-front discipline as `patch_string`: the target's own tag size, every enclosing size field, and the element count are updated by the byte delta, then the splice runs. Offsets in the parsed tree are stale afterwards — callers re-parse.

**Files:**
- Modify: `crates/goresave_core/src/properties.rs` (after `container_layout`; tests at the bottom)

- [ ] **Step 1: Write the failing tests**

Add to the `tests` module (uses Task 1's builders). The nested fixture proves the enclosing-size-chain fixup through a struct wrapper:

```rust
    fn struct_wrapping(name: &str, struct_type: &str, inner_props: &[u8]) -> Vec<u8> {
        let mut body = inner_props.to_vec();
        body.extend_from_slice(&fstring("None"));
        let mut out = tag(name, "StructProperty");
        out.extend_from_slice(&1u32.to_le_bytes());
        out.extend_from_slice(&fstring(struct_type));
        out.extend_from_slice(&1u32.to_le_bytes());
        out.extend_from_slice(&fstring("/Script/Test"));
        out.extend_from_slice(&header(body.len() as u32, 0));
        out.extend_from_slice(&body);
        out
    }

    fn resolve_set_target(payload: &[u8]) -> (RootObject, Vec<PathSeg>) {
        let parsed = parse_private_root(payload).unwrap();
        let path = parse_path(&[
            "KnowledgeSet".to_string(),
            "Knowledge".to_string(),
        ])
        .unwrap();
        (parsed, path)
    }

    #[test]
    fn patch_container_set_add_appends_and_fixes_sizes() {
        let mut payload = root(
            "/Script/Test.Save",
            &struct_wrapping(
                "KnowledgeSet",
                "KnowledgeSet",
                &name_set_property("Knowledge", &["Voiceline_A"]),
            ),
        );
        let (parsed, path) = resolve_set_target(&payload);
        let chain = resolve_chain(&parsed.properties, &path).unwrap();
        let target = chain.target.clone();

        patch_container(
            &mut payload,
            &target,
            &chain.enclosing_size_fields,
            &ContainerEdit::SetAdd("ChoiceB".to_string()),
        )
        .unwrap();

        // Strict re-parse proves every size field (set tag + wrapping struct
        // tag) was adjusted.
        let reparsed = parse_private_root(&payload).unwrap();
        let set = resolve(&reparsed.properties, &path).unwrap();
        assert_eq!(
            set.value,
            PropertyValue::Set {
                num_to_remove: 0,
                elements: vec![
                    PropertyValue::Name("Voiceline_A".to_string()),
                    PropertyValue::Name("ChoiceB".to_string()),
                ],
            }
        );
    }

    #[test]
    fn patch_container_set_add_rejects_duplicates_without_mutation() {
        let mut payload = root(
            "/Script/Test.Save",
            &name_set_property("Knowledge", &["Voiceline_A"]),
        );
        let parsed = parse_private_root(&payload).unwrap();
        let target = parsed.properties[0].clone();
        let copy = payload.clone();
        assert!(
            patch_container(
                &mut payload,
                &target,
                &[],
                &ContainerEdit::SetAdd("Voiceline_A".to_string()),
            )
            .is_err()
        );
        assert_eq!(payload, copy);
    }

    #[test]
    fn patch_container_set_remove_splices_element_out() {
        let mut payload = root(
            "/Script/Test.Save",
            &struct_wrapping(
                "KnowledgeSet",
                "KnowledgeSet",
                &name_set_property("Knowledge", &["Voiceline_A", "ChoiceB", "Voiceline_C"]),
            ),
        );
        let (parsed, path) = resolve_set_target(&payload);
        let chain = resolve_chain(&parsed.properties, &path).unwrap();
        let target = chain.target.clone();

        patch_container(
            &mut payload,
            &target,
            &chain.enclosing_size_fields,
            &ContainerEdit::SetRemove("ChoiceB".to_string()),
        )
        .unwrap();

        let reparsed = parse_private_root(&payload).unwrap();
        let set = resolve(&reparsed.properties, &path).unwrap();
        assert_eq!(
            set.value,
            PropertyValue::Set {
                num_to_remove: 0,
                elements: vec![
                    PropertyValue::Name("Voiceline_A".to_string()),
                    PropertyValue::Name("Voiceline_C".to_string()),
                ],
            }
        );

        // Removing a value that is not present fails without mutation.
        let parsed = parse_private_root(&payload).unwrap();
        let chain = resolve_chain(&parsed.properties, &path).unwrap();
        let target = chain.target.clone();
        let copy = payload.clone();
        assert!(
            patch_container(
                &mut payload,
                &target,
                &chain.enclosing_size_fields,
                &ContainerEdit::SetRemove("ChoiceB".to_string()),
            )
            .is_err()
        );
        assert_eq!(payload, copy);
    }

    #[test]
    fn patch_container_array_remove_and_duplicate() {
        let mut payload = root("/Script/Test.Save", &int_array_property("Nums", &[7, 8, 9]));
        let parsed = parse_private_root(&payload).unwrap();
        let target = parsed.properties[0].clone();

        patch_container(&mut payload, &target, &[], &ContainerEdit::ArrayRemove(1)).unwrap();
        let reparsed = parse_private_root(&payload).unwrap();
        assert_eq!(
            reparsed.properties[0].value,
            PropertyValue::Array {
                elements: vec![PropertyValue::Int(7), PropertyValue::Int(9)],
            }
        );

        let target = reparsed.properties[0].clone();
        patch_container(&mut payload, &target, &[], &ContainerEdit::ArrayDuplicate(0)).unwrap();
        let reparsed = parse_private_root(&payload).unwrap();
        assert_eq!(
            reparsed.properties[0].value,
            PropertyValue::Array {
                elements: vec![
                    PropertyValue::Int(7),
                    PropertyValue::Int(7),
                    PropertyValue::Int(9),
                ],
            }
        );

        // Out-of-bounds index fails without mutation.
        let target = reparsed.properties[0].clone();
        let copy = payload.clone();
        assert!(
            patch_container(&mut payload, &target, &[], &ContainerEdit::ArrayRemove(3)).is_err()
        );
        assert_eq!(payload, copy);
    }

    #[test]
    fn patch_container_rejects_kind_mismatch() {
        let mut payload = root("/Script/Test.Save", &int_array_property("Nums", &[7]));
        let parsed = parse_private_root(&payload).unwrap();
        let target = parsed.properties[0].clone();
        // set ops on an array
        assert!(
            patch_container(&mut payload, &target, &[], &ContainerEdit::SetAdd("X".into()))
                .is_err()
        );
        // array ops on a set
        let mut payload = root("/Script/Test.Save", &name_set_property("S", &["A"]));
        let parsed = parse_private_root(&payload).unwrap();
        let target = parsed.properties[0].clone();
        assert!(
            patch_container(&mut payload, &target, &[], &ContainerEdit::ArrayRemove(0)).is_err()
        );
        // setAdd on a non-string set inner type
        let mut payload = root("/Script/Test.Save", &int_array_property("Nums", &[7]));
        let parsed = parse_private_root(&payload).unwrap();
        let _ = (&mut payload, parsed); // (int set builder not needed; kind check above covers it)
    }
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p goresave_core patch_container`
Expected: FAIL to compile — `ContainerEdit`/`patch_container` not defined.

- [ ] **Step 3: Implement `ContainerEdit` and `patch_container`**

Add after `container_layout`:

```rust
/// Structural container edit applied by `patch_container`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ContainerEdit {
    /// Append a Name/Str element to a SetProperty (rejects duplicates).
    SetAdd(String),
    /// Remove a Name/Str element from a SetProperty by value.
    SetRemove(String),
    /// Remove an ArrayProperty element by index.
    ArrayRemove(usize),
    /// Duplicate an ArrayProperty element in place (copy inserted right after
    /// the source element).
    ArrayDuplicate(usize),
}

fn set_string_elements(target: &Property) -> Option<&[PropertyValue]> {
    match &target.value {
        PropertyValue::Set { elements, .. } => Some(elements),
        _ => None,
    }
}

fn set_element_position(elements: &[PropertyValue], value: &str) -> Option<usize> {
    elements.iter().position(|e| match e {
        PropertyValue::Name(s) | PropertyValue::Str(s) => s == value,
        _ => false,
    })
}

/// Apply a structural set/array edit to a resolved container property. The
/// element count, the property's own tag size, and every enclosing size field
/// (from [`resolve_chain`]) are adjusted by the byte delta; all writes are
/// validated before the first mutation, so a failed patch leaves the payload
/// untouched. Offsets recorded in the parsed tree are stale after a successful
/// patch — re-parse before further edits.
pub fn patch_container(
    payload: &mut Vec<u8>,
    target: &Property,
    enclosing_size_fields: &[usize],
    edit: &ContainerEdit,
) -> Result<(), CoreError> {
    let layout = container_layout(payload, target)?;
    let require_kind = |wanted: ContainerKind, op: &str| {
        if layout.kind == wanted {
            Ok(())
        } else {
            Err(CoreError::InvalidRequest(format!(
                "{op} requires a {wanted:?} target, got {:?}",
                layout.kind
            )))
        }
    };
    // Each edit is one splice: either remove a byte range or insert bytes at a
    // position. `count_delta` is +1 or -1.
    let (remove_range, insert_at, insert_bytes, count_delta): (
        Option<core::ops::Range<usize>>,
        usize,
        Vec<u8>,
        i64,
    ) = match edit {
        ContainerEdit::SetAdd(value) => {
            require_kind(ContainerKind::Set, "setAdd")?;
            if !matches!(layout.inner_type.as_str(), "NameProperty" | "StrProperty") {
                return Err(CoreError::UnsupportedEdit(format!(
                    "setAdd supports Name/Str sets; this set holds {}",
                    layout.inner_type
                )));
            }
            let elements = set_string_elements(target)
                .ok_or_else(|| CoreError::Parse("set value not parsed as a set".into()))?;
            if set_element_position(elements, value).is_some() {
                return Err(CoreError::InvalidRequest(format!(
                    "set already contains {value:?}"
                )));
            }
            let end = target.value_offset + target.value_size;
            (None, end, encode_fstring_value(value), 1)
        }
        ContainerEdit::SetRemove(value) => {
            require_kind(ContainerKind::Set, "setRemove")?;
            let elements = set_string_elements(target)
                .ok_or_else(|| CoreError::Parse("set value not parsed as a set".into()))?;
            let index = set_element_position(elements, value).ok_or_else(|| {
                CoreError::Parse(format!("set does not contain {value:?}"))
            })?;
            let range = layout.element_ranges[index].clone();
            (Some(range.clone()), range.start, Vec::new(), -1)
        }
        ContainerEdit::ArrayRemove(index) => {
            require_kind(ContainerKind::Array, "arrayRemove")?;
            let range = layout.element_ranges.get(*index).cloned().ok_or_else(|| {
                CoreError::InvalidRequest(format!(
                    "array index {index} out of bounds ({} elements)",
                    layout.count
                ))
            })?;
            (Some(range.clone()), range.start, Vec::new(), -1)
        }
        ContainerEdit::ArrayDuplicate(index) => {
            require_kind(ContainerKind::Array, "arrayDuplicate")?;
            let range = layout.element_ranges.get(*index).cloned().ok_or_else(|| {
                CoreError::InvalidRequest(format!(
                    "array index {index} out of bounds ({} elements)",
                    layout.count
                ))
            })?;
            let bytes = payload[range.clone()].to_vec();
            (None, range.end, bytes, 1)
        }
    };
    let removed = remove_range.as_ref().map_or(0, ExactSizeIterator::len);
    let delta = insert_bytes.len() as i64 - removed as i64;
    let new_count = u32::try_from(layout.count as i64 + count_delta)
        .map_err(|_| CoreError::Parse("container count underflow".to_string()))?;
    let new_size = u32::try_from(target.value_size as i64 + delta)
        .map_err(|_| CoreError::Parse("container size would leave the u32 range".to_string()))?;

    // Compute every size-field rewrite up front; mutate only once all are
    // valid (same discipline as patch_string).
    let mut writes = Vec::with_capacity(enclosing_size_fields.len() + 2);
    if target.value_offset < 5 {
        return Err(CoreError::Parse("container tag offset underflow".to_string()));
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
    // The count field lives inside the value payload but always precedes the
    // splice position (elements follow the count), so writing it before the
    // splice is safe.
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

Note: `remove_range.as_ref().map_or(0, ExactSizeIterator::len)` — if the trait call reads awkwardly, `map_or(0, |r| r.len())` is equivalent; use whichever compiles cleanly.

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test -p goresave_core patch_container`
Expected: 5 PASS. Also run `cargo test -p goresave_core` to confirm no regressions.

- [ ] **Step 5: Commit**

```bash
git add crates/goresave_core/src/properties.rs
git commit -m "feat(core): patch_container set/array structural edits"
```

---

### Task 3: Rust core — `find_property_by_name` tree walker

`query_progression` and the inspect overview locate `QuestDataByClass` / `CharacterKnowledgeByUniqueName` / `LongTermMemoryByGlobalId` without hardcoding their nesting, returning the setValue-addressable path prefix.

**Files:**
- Modify: `crates/goresave_core/src/properties.rs`

- [ ] **Step 1: Write the failing test**

```rust
    #[test]
    fn find_property_by_name_returns_addressable_path() {
        // Map { "CharacterStates" => InstancedStruct { Knowledge: Set } }
        let nested = {
            let mut n = name_set_property("Knowledge", &["Voiceline_A"]);
            n.extend_from_slice(&fstring("None"));
            n
        };
        let mut instanced = fstring("/Script/Test.CharacterStates");
        instanced.extend_from_slice(&(nested.len() as u32).to_le_bytes());
        instanced.extend_from_slice(&nested);

        let mut map_body = 0u32.to_le_bytes().to_vec();
        map_body.extend_from_slice(&1u32.to_le_bytes());
        map_body.extend_from_slice(&fstring("CharacterStates")); // Name key
        map_body.extend_from_slice(&instanced);

        let mut props = tag("m_GenericData", "MapProperty");
        props.extend_from_slice(&2u32.to_le_bytes());
        props.extend_from_slice(&fstring("NameProperty"));
        props.extend_from_slice(&0u32.to_le_bytes()); // key_flags
        props.extend_from_slice(&fstring("StructProperty"));
        props.extend_from_slice(&1u32.to_le_bytes());
        props.extend_from_slice(&fstring("InstancedStruct"));
        props.extend_from_slice(&1u32.to_le_bytes());
        props.extend_from_slice(&fstring("/Script/StructUtils"));
        props.extend_from_slice(&header(map_body.len() as u32, TAG_FLAG_NATIVE_SERIALIZE));
        props.extend_from_slice(&map_body);
        let payload = root("/Script/Test.Save", &props);

        let parsed = parse_private_root(&payload).unwrap();
        let (path, prop) = find_property_by_name(&parsed, "Knowledge").unwrap();
        assert_eq!(path, vec!["m_GenericData", "{CharacterStates}", "Knowledge"]);
        assert!(matches!(prop.value, PropertyValue::Set { .. }));
        // The returned path round-trips through resolve().
        let segs = parse_path(&path).unwrap();
        assert_eq!(resolve(&parsed.properties, &segs).unwrap().name, "Knowledge");

        assert!(find_property_by_name(&parsed, "DoesNotExist").is_none());
    }
```

If the map fixture builder pattern already exists from the hero-stats tests (`map_of_object_keyed_instanced_payload`), follow its exact byte layout for descriptors.

- [ ] **Step 2: Run to verify it fails**

Run: `cargo test -p goresave_core find_property_by_name`
Expected: FAIL to compile.

- [ ] **Step 3: Implement**

Add near `search_properties`:

```rust
/// Depth-first search for the first property named `name` anywhere in the
/// tree. Returns the setValue-addressable path segments leading to it
/// (inclusive) plus the property. Map entries whose keys cannot be rendered as
/// path segments are skipped (a hit behind such a key would not be
/// addressable anyway).
pub fn find_property_by_name<'a>(
    root: &'a RootObject,
    name: &str,
) -> Option<(Vec<String>, &'a Property)> {
    fn in_props<'a>(
        props: &'a [Property],
        name: &str,
        path: &mut Vec<String>,
    ) -> Option<&'a Property> {
        for p in props {
            path.push(p.name.clone());
            if p.name == name {
                return Some(p);
            }
            if let Some(found) = in_value(&p.value, name, path) {
                return Some(found);
            }
            path.pop();
        }
        None
    }
    fn in_value<'a>(
        value: &'a PropertyValue,
        name: &str,
        path: &mut Vec<String>,
    ) -> Option<&'a Property> {
        match value {
            PropertyValue::Struct(StructValue::Properties(inner)) => in_props(inner, name, path),
            PropertyValue::Struct(StructValue::Instanced(Some(i))) => {
                in_props(&i.properties, name, path)
            }
            PropertyValue::Map { entries, .. } => {
                for (key, val) in entries {
                    let Some(key) = map_key_to_string(key) else {
                        continue;
                    };
                    path.push(format!("{{{key}}}"));
                    if let Some(found) = in_value(val, name, path) {
                        return Some(found);
                    }
                    path.pop();
                }
                None
            }
            PropertyValue::Array { elements } | PropertyValue::Set { elements, .. } => {
                for (i, e) in elements.iter().enumerate() {
                    path.push(format!("[{i}]"));
                    if let Some(found) = in_value(e, name, path) {
                        return Some(found);
                    }
                    path.pop();
                }
                None
            }
            PropertyValue::ObjectInstances(objs) => {
                for (i, o) in objs.iter().enumerate() {
                    path.push(format!("[{i}]"));
                    if let Some(found) = in_props(&o.properties, name, path) {
                        return Some(found);
                    }
                    path.pop();
                }
                None
            }
            _ => None,
        }
    }
    let mut path = Vec::new();
    let target = in_props(&root.properties, name, &mut path)?;
    Some((path, target))
}
```

- [ ] **Step 4: Run to verify it passes**

Run: `cargo test -p goresave_core find_property_by_name`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/goresave_core/src/properties.rs
git commit -m "feat(core): find_property_by_name typed-tree walker"
```

---

### Task 4: Rust core — wire the four container ops into the edit pipeline

New edit paths `private.typed.setAdd`, `private.typed.setRemove`, `private.typed.arrayRemove`, `private.typed.arrayDuplicate`, applied with the scratch-copy + strict-re-parse pattern of `apply_private_typed_set_value_edit_to_payload`, and advertised in the inspect `writable` list when the typed parse is ok.

**Files:**
- Modify: `crates/goresave_core/src/lib.rs`
  - dispatch match in `apply_private_edits` (~line 3300)
  - `PrivateEdit` enum (~line 3391) and edit structs above it
  - apply dispatch in `apply_private_edit_to_payload` (~line 3893)
  - writable list in the inspect private summary (~line 2125)
  - tests at the bottom

- [ ] **Step 1: Write the failing tests**

Add to the lib.rs `tests` module. The unit test exercises the apply path on a raw payload; the integration test goes through `write_save_with_codec_backend` like `write_save_applies_typed_set_value_edit` (~line 6468). Reuse the test module's existing builders; add this one if missing (mirror the byte layout from the properties.rs tests):

```rust
    fn private_name_set_property(name: &str, values: &[&str]) -> Vec<u8> {
        let mut body = 0u32.to_le_bytes().to_vec();
        body.extend_from_slice(&(values.len() as u32).to_le_bytes());
        for v in values {
            body.extend_from_slice(&fstring(v));
        }
        let mut out = fstring(name);
        out.extend_from_slice(&fstring("SetProperty"));
        out.extend_from_slice(&1u32.to_le_bytes());
        out.extend_from_slice(&fstring("NameProperty"));
        out.extend_from_slice(&0u32.to_le_bytes()); // array_index
        out.extend_from_slice(&(body.len() as u32).to_le_bytes());
        out.push(0); // tag_flags
        out.extend_from_slice(&body);
        out
    }

    #[test]
    fn typed_container_edits_apply_and_validate() {
        let mut payload = fstring("/Script/Test.Save");
        payload.push(0);
        payload.extend_from_slice(&private_name_set_property("Knowledge", &["A", "B"]));
        payload.extend_from_slice(&fstring("None"));
        payload.extend_from_slice(&0u32.to_le_bytes());

        let edit = PrivateTypedContainerEdit {
            path: properties::parse_path(&["Knowledge".to_string()]).unwrap(),
            edit: properties::ContainerEdit::SetAdd("C".to_string()),
        };
        apply_private_typed_container_edit_to_payload(&mut payload, &edit).unwrap();
        let edit = PrivateTypedContainerEdit {
            path: properties::parse_path(&["Knowledge".to_string()]).unwrap(),
            edit: properties::ContainerEdit::SetRemove("A".to_string()),
        };
        apply_private_typed_container_edit_to_payload(&mut payload, &edit).unwrap();

        let root = properties::parse_private_root(&payload).unwrap();
        assert_eq!(
            root.properties[0].value,
            properties::PropertyValue::Set {
                num_to_remove: 0,
                elements: vec![
                    properties::PropertyValue::Name("B".to_string()),
                    properties::PropertyValue::Name("C".to_string()),
                ],
            }
        );

        // Unknown path fails without mutation.
        let copy = payload.clone();
        let bad = PrivateTypedContainerEdit {
            path: properties::parse_path(&["Nope".to_string()]).unwrap(),
            edit: properties::ContainerEdit::SetAdd("X".to_string()),
        };
        assert!(apply_private_typed_container_edit_to_payload(&mut payload, &bad).is_err());
        assert_eq!(payload, copy);
    }

    #[test]
    fn write_save_applies_typed_container_edits() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("G1R-001.sav");
        let output_path = dir.path().join("G1R-001-container.sav");
        let private_payload = {
            let mut p = fstring("/Script/Angelscript.GothicFinalDataGame");
            p.push(0);
            p.extend_from_slice(&private_name_set_property("Knowledge", &["Voiceline_A"]));
            p.extend_from_slice(&fstring("None"));
            p.extend_from_slice(&0u32.to_le_bytes());
            p
        };
        let seed_compressed = b"seed-compressed".to_vec();
        let stream = compressed_stream_with_one_chunk(&seed_compressed, private_payload.len());
        fs::write(
            &path,
            build_gsav(2, &public_payload("Slot A"), &stream, &[0, 0, 0, 0]),
        )
        .unwrap();
        let backend = PrefixCodecBackend {
            seed_compressed,
            seed_uncompressed: private_payload,
        };

        // The container ops must be advertised once the typed parse is ok.
        let inspected = inspect_save_with_codec_backend(&path, true, Some(&backend), None).unwrap();
        let writable = inspected["private"]["writable"].as_array().unwrap();
        for op in [
            "private.typed.setAdd",
            "private.typed.setRemove",
            "private.typed.arrayRemove",
            "private.typed.arrayDuplicate",
        ] {
            assert!(writable.contains(&json!(op)), "missing writable {op}");
        }

        let response = write_save_with_codec_backend(
            &path,
            &[json!({
                "path": "private.typed.setAdd",
                "value": { "path": ["Knowledge"], "value": "ChoiceB" }
            })],
            false,
            Some(&output_path),
            Some(&backend),
        )
        .unwrap();
        assert_eq!(response["editsApplied"], 1);

        let value =
            inspect_save_with_codec_backend(&output_path, true, Some(&backend), None).unwrap();
        assert_eq!(value["private"]["typedParse"]["status"], "ok");
        let strings = value["private"]["strings"].as_array().unwrap();
        assert!(strings.iter().any(|s| s == "ChoiceB"));
    }
```

- [ ] **Step 2: Run to verify they fail**

Run: `cargo test -p goresave_core typed_container`
Expected: FAIL to compile — `PrivateTypedContainerEdit` not defined.

- [ ] **Step 3: Implement parse/dispatch/apply + writable**

Next to `PrivateTypedSetValueEdit` (search for it, ~line 3400), add:

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
struct PrivateTypedContainerEdit {
    path: Vec<properties::PathSeg>,
    edit: properties::ContainerEdit,
}
```

Add a variant to `PrivateEdit` (~line 3391):

```rust
    TypedContainer(PrivateTypedContainerEdit),
```

Add the parser next to `parse_private_typed_set_value_edit`. It shares the path extraction; factor the segment parsing into a helper so both ops use it:

```rust
fn parse_typed_edit_path(op: &str, value: &serde_json::Map<String, Value>) -> Result<Vec<properties::PathSeg>, CoreError> {
    let segments = value
        .get("path")
        .and_then(Value::as_array)
        .ok_or_else(|| {
            CoreError::InvalidRequest(format!(
                "{op} requires value.path as an array of segments"
            ))
        })?
        .iter()
        .map(|segment| {
            segment.as_str().map(str::to_string).ok_or_else(|| {
                CoreError::InvalidRequest(format!("{op} path segments must be strings"))
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    if segments.is_empty() {
        return Err(CoreError::InvalidRequest(format!(
            "{op} requires a non-empty value.path"
        )));
    }
    properties::parse_path(&segments)
}

fn parse_private_typed_container_edit(
    edit: &Edit,
    op: &str,
) -> Result<PrivateTypedContainerEdit, CoreError> {
    let value = edit.value.as_object().ok_or_else(|| {
        CoreError::InvalidRequest(format!("{op} value must be an object"))
    })?;
    let path = parse_typed_edit_path(op, value)?;
    let container_edit = match op {
        "private.typed.setAdd" | "private.typed.setRemove" => {
            let element = value
                .get("value")
                .and_then(Value::as_str)
                .filter(|s| !s.trim().is_empty())
                .ok_or_else(|| {
                    CoreError::InvalidRequest(format!(
                        "{op} requires a non-empty string value.value"
                    ))
                })?
                .to_string();
            if op == "private.typed.setAdd" {
                properties::ContainerEdit::SetAdd(element)
            } else {
                properties::ContainerEdit::SetRemove(element)
            }
        }
        "private.typed.arrayRemove" | "private.typed.arrayDuplicate" => {
            let index = value
                .get("index")
                .and_then(Value::as_u64)
                .and_then(|v| usize::try_from(v).ok())
                .ok_or_else(|| {
                    CoreError::InvalidRequest(format!(
                        "{op} requires a non-negative integer value.index"
                    ))
                })?;
            if op == "private.typed.arrayRemove" {
                properties::ContainerEdit::ArrayRemove(index)
            } else {
                properties::ContainerEdit::ArrayDuplicate(index)
            }
        }
        other => {
            return Err(CoreError::UnsupportedEdit(format!(
                "{other} is not a typed container edit"
            )));
        }
    };
    Ok(PrivateTypedContainerEdit {
        path,
        edit: container_edit,
    })
}
```

Refactor `parse_private_typed_set_value_edit` to use `parse_typed_edit_path` (drop its inline copy of the segment extraction).

Dispatch arms in `apply_private_edits` (~line 3320), after the `private.typed.setValue` arm:

```rust
            "private.typed.setAdd"
            | "private.typed.setRemove"
            | "private.typed.arrayRemove"
            | "private.typed.arrayDuplicate" => {
                parse_private_typed_container_edit(edit, edit.path.as_str())
                    .map(PrivateEdit::TypedContainer)
            }
```

Apply dispatch in `apply_private_edit_to_payload` (~line 3893):

```rust
        PrivateEdit::TypedContainer(edit) => {
            apply_private_typed_container_edit_to_payload(payload, edit)
        }
```

Apply function next to `apply_private_typed_set_value_edit_to_payload`:

```rust
fn apply_private_typed_container_edit_to_payload(
    payload: &mut Vec<u8>,
    edit: &PrivateTypedContainerEdit,
) -> Result<(), CoreError> {
    let root = properties::parse_private_root(payload)?;
    let resolved = properties::resolve_chain(&root.properties, &edit.path)?;
    let target = resolved.target.clone();
    // Length-changing patch: work on a scratch copy and prove with a strict
    // re-parse that every size and count field was fixed up, so a bug cannot
    // corrupt the caller's payload (or the save).
    let mut patched = payload.clone();
    properties::patch_container(
        &mut patched,
        &target,
        &resolved.enclosing_size_fields,
        &edit.edit,
    )?;
    properties::parse_private_root(&patched).map_err(|err| {
        CoreError::Parse(format!(
            "container patch produced an inconsistent payload: {err}"
        ))
    })?;
    *payload = patched;
    Ok(())
}
```

Writable list in the inspect private summary (~line 2125), replace the single push:

```rust
            let mut writable = vec!["private.replaceFString"];
            if typed_parse["status"] == "ok" {
                writable.extend([
                    "private.typed.setValue",
                    "private.typed.setAdd",
                    "private.typed.setRemove",
                    "private.typed.arrayRemove",
                    "private.typed.arrayDuplicate",
                ]);
            }
```

- [ ] **Step 4: Run to verify they pass**

Run: `cargo test -p goresave_core typed_container` then `cargo test -p goresave_core`
Expected: new tests PASS; full suite green (an existing test asserting the exact writable list may need the four new entries added — update it, that is an intended behavior change).

- [ ] **Step 5: Commit**

```bash
git add crates/goresave_core/src/lib.rs
git commit -m "feat(core): private.typed.setAdd/setRemove/arrayRemove/arrayDuplicate edit ops"
```

---

### Task 5: Rust core — `query_progression` command (quests section)

**Files:**
- Modify: `crates/goresave_core/src/lib.rs`
  - command dispatch in `execute_json_inner` (next to `"search_typed_properties"`, ~line 400)
  - new functions next to `search_typed_properties` (~line 2380)
  - tests at the bottom

- [ ] **Step 1: Write the failing test**

The fixture builds a `QuestDataByClass` map with ObjectProperty keys and struct values holding a `CurrentState` EnumProperty, wrapped in a GSAV container like the other command tests:

```rust
    fn private_enum_property(name: &str, enum_type: &str, label: &str) -> Vec<u8> {
        let body = fstring(label);
        let mut out = fstring(name);
        out.extend_from_slice(&fstring("EnumProperty"));
        out.extend_from_slice(&1u32.to_le_bytes());
        out.extend_from_slice(&fstring(enum_type));
        out.extend_from_slice(&1u32.to_le_bytes());
        out.extend_from_slice(&fstring("/Script/G1R"));
        out.extend_from_slice(&1u32.to_le_bytes());
        out.extend_from_slice(&fstring("ByteProperty"));
        out.extend_from_slice(&0u32.to_le_bytes()); // array_index
        out.extend_from_slice(&(body.len() as u32).to_le_bytes());
        out.push(0); // tag_flags
        out.extend_from_slice(&body);
        out
    }

    fn quest_map_payload() -> Vec<u8> {
        let quest_value = |state: &str| {
            let mut v = private_enum_property("CurrentState", "EQuestState", state);
            v.extend_from_slice(&fstring("None"));
            v
        };
        let mut map_body = 0u32.to_le_bytes().to_vec(); // num_to_remove
        map_body.extend_from_slice(&2u32.to_le_bytes()); // count
        map_body.extend_from_slice(&fstring("/Script/Angelscript.Quest_OldCamp_SLEEPER"));
        map_body.extend_from_slice(&quest_value("EQuestState::Running"));
        map_body.extend_from_slice(&fstring("/Script/Angelscript.Quest_BanditsCamp_BANDITSTRUST"));
        map_body.extend_from_slice(&quest_value("EQuestState::Available"));

        let mut props = fstring("QuestDataByClass");
        props.extend_from_slice(&fstring("MapProperty"));
        props.extend_from_slice(&2u32.to_le_bytes());
        props.extend_from_slice(&fstring("ObjectProperty"));
        props.extend_from_slice(&0u32.to_le_bytes()); // key_flags
        props.extend_from_slice(&fstring("StructProperty"));
        props.extend_from_slice(&1u32.to_le_bytes());
        props.extend_from_slice(&fstring("SingleQuestSaveGameData"));
        props.extend_from_slice(&1u32.to_le_bytes());
        props.extend_from_slice(&fstring("/Script/G1R"));
        props.extend_from_slice(&0u32.to_le_bytes()); // array_index
        props.extend_from_slice(&(map_body.len() as u32).to_le_bytes());
        props.push(0); // tag_flags (struct map values are tagged property lists)
        props.extend_from_slice(&map_body);

        let mut p = fstring("/Script/Angelscript.GothicFinalDataGame");
        p.push(0);
        p.extend_from_slice(&props);
        p.extend_from_slice(&fstring("None"));
        p.extend_from_slice(&0u32.to_le_bytes());
        p
    }

    #[test]
    fn query_progression_lists_quests_with_state_paths() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("G1R-quests.sav");
        let private_payload = quest_map_payload();
        let seed_compressed = b"seed".to_vec();
        let stream = compressed_stream_with_one_chunk(&seed_compressed, private_payload.len());
        fs::write(
            &path,
            build_gsav(2, &public_payload("Slot"), &stream, &[0, 0, 0, 0]),
        )
        .unwrap();
        let backend = PrefixCodecBackend {
            seed_compressed,
            seed_uncompressed: private_payload,
        };

        let value =
            query_progression(&path, &json!({ "section": "quests" }), Some(&backend)).unwrap();
        assert_eq!(value["section"], "quests");
        assert_eq!(value["total"], 2);
        assert_eq!(value["stateCounts"]["Running"], 1);
        assert_eq!(value["stateCounts"]["Available"], 1);
        // Sorted by class path: BanditsCamp before OldCamp.
        let first = &value["quests"][0];
        assert_eq!(
            first["questClass"],
            "/Script/Angelscript.Quest_BanditsCamp_BANDITSTRUST"
        );
        assert_eq!(first["id"], "Quest_BanditsCamp_BANDITSTRUST");
        assert_eq!(first["group"], "BanditsCamp");
        assert_eq!(first["name"], "BANDITSTRUST");
        assert_eq!(first["currentState"], "EQuestState::Available");
        assert_eq!(
            first["statePath"],
            json!([
                "QuestDataByClass",
                "{/Script/Angelscript.Quest_BanditsCamp_BANDITSTRUST}",
                "CurrentState"
            ])
        );

        // Query filter + paging.
        let filtered = query_progression(
            &path,
            &json!({ "section": "quests", "query": "sleeper" }),
            Some(&backend),
        )
        .unwrap();
        assert_eq!(filtered["total"], 1);
        assert_eq!(filtered["quests"][0]["name"], "SLEEPER");

        // The statePath round-trips through the existing setValue write.
        let output_path = dir.path().join("G1R-quests-out.sav");
        let response = write_save_with_codec_backend(
            &path,
            &[json!({
                "path": "private.typed.setValue",
                "value": {
                    "path": [
                        "QuestDataByClass",
                        "{/Script/Angelscript.Quest_BanditsCamp_BANDITSTRUST}",
                        "CurrentState"
                    ],
                    "value": "EQuestState::Succeeded"
                }
            })],
            false,
            Some(&output_path),
            Some(&backend),
        )
        .unwrap();
        assert_eq!(response["editsApplied"], 1);
        let after = query_progression(
            &output_path,
            &json!({ "section": "quests", "query": "banditstrust" }),
            Some(&backend),
        )
        .unwrap();
        assert_eq!(after["quests"][0]["currentState"], "EQuestState::Succeeded");
    }
```

- [ ] **Step 2: Run to verify it fails**

Run: `cargo test -p goresave_core query_progression_lists_quests`
Expected: FAIL to compile — `query_progression` not defined.

- [ ] **Step 3: Implement command + quests section**

Dispatch in `execute_json_inner`, after the `"search_typed_properties"` arm (same backend plumbing):

```rust
        "query_progression" => {
            let path = required_path(&payload)?;
            let codec_backend = payload
                .get("binaryHost")
                .map(binary_host_backend_from_config)
                .transpose()?;
            let codec_backend = codec_backend
                .as_ref()
                .map(|backend| backend as &dyn codec_backend::CodecBackend);
            query_progression(&path, &payload, codec_backend)
        }
```

Implementation next to `search_typed_properties`:

```rust
/// Structured progression queries over the decoded private payload. Sections:
/// "quests" (QuestDataByClass entries with setValue-addressable state paths),
/// "knowledge" (per-NPC dialog knowledge sets), "events" (per-character
/// memorized event arrays). Uses the shared decode cache like the typed
/// property search.
fn query_progression(
    path: &Path,
    payload: &Value,
    backend: Option<&dyn codec_backend::CodecBackend>,
) -> Result<Value, CoreError> {
    let backend = backend.ok_or_else(|| {
        CoreError::Codec(
            "progression queries require a configured and verified G1R codec host".to_string(),
        )
    })?;
    let section = payload
        .get("section")
        .and_then(Value::as_str)
        .unwrap_or("quests");
    let query = payload
        .get("query")
        .and_then(Value::as_str)
        .unwrap_or("")
        .trim()
        .to_ascii_lowercase();
    let limit = payload
        .get("limit")
        .and_then(Value::as_u64)
        .map(|v| v as usize)
        .unwrap_or(100)
        .clamp(1, 1000);
    let offset = payload
        .get("offset")
        .and_then(Value::as_u64)
        .map(|v| v as usize)
        .unwrap_or(0);
    let character = payload.get("character").and_then(Value::as_str);

    let data = fs::read(path)?;
    if !data.starts_with(b"GSAV") {
        return Err(CoreError::UnsupportedEdit(
            "progression queries are only available for GSAV files".to_string(),
        ));
    }
    let parts = split_gsav(&data)?;
    let stream = parse_compressed_stream(&data, 13 + parts.public_payload.len())?;
    let decoded = decoded_private_payload_cached(path, &data, &stream, backend)?;
    let root = properties::parse_private_root(&decoded)?;
    match section {
        "quests" => progression_quests(&root, &query, offset, limit),
        "knowledge" => progression_knowledge(&root, &query, character, offset, limit),
        "events" => progression_events(&root, &query, character, offset, limit),
        other => Err(CoreError::InvalidRequest(format!(
            "unknown progression section {other:?}"
        ))),
    }
}

/// Property lookup inside a struct-valued map entry (tagged property list or
/// InstancedStruct wrapper).
fn struct_member<'a>(
    value: &'a properties::PropertyValue,
    name: &str,
) -> Option<&'a properties::PropertyValue> {
    let props = match value {
        properties::PropertyValue::Struct(properties::StructValue::Properties(p)) => p,
        properties::PropertyValue::Struct(properties::StructValue::Instanced(Some(i))) => {
            &i.properties
        }
        _ => return None,
    };
    props.iter().find(|p| p.name == name).map(|p| &p.value)
}

fn map_key_string(key: &properties::PropertyValue) -> Option<&str> {
    match key {
        properties::PropertyValue::Str(s)
        | properties::PropertyValue::Name(s)
        | properties::PropertyValue::Enum(s)
        | properties::PropertyValue::Object(s) => Some(s),
        _ => None,
    }
}

/// "EQuestState::Running" → "Running" for the overview/state-count labels.
fn short_enum_label(value: &str) -> &str {
    value.rsplit("::").next().unwrap_or(value)
}

fn progression_quests(
    root: &properties::RootObject,
    query: &str,
    offset: usize,
    limit: usize,
) -> Result<Value, CoreError> {
    let (base_path, map_prop) = properties::find_property_by_name(root, "QuestDataByClass")
        .ok_or_else(|| {
            CoreError::Parse("QuestDataByClass not found in the decoded payload".to_string())
        })?;
    let properties::PropertyValue::Map { entries, .. } = &map_prop.value else {
        return Err(CoreError::Parse(
            "QuestDataByClass is not a map".to_string(),
        ));
    };
    let mut state_counts = std::collections::BTreeMap::<String, usize>::new();
    let mut matches: Vec<(String, Option<String>)> = Vec::new();
    for (key, value) in entries {
        let Some(class_path) = map_key_string(key) else {
            continue;
        };
        let state = struct_member(value, "CurrentState").and_then(|v| match v {
            properties::PropertyValue::Enum(s) => Some(s.clone()),
            _ => None,
        });
        let label = state
            .as_deref()
            .map(short_enum_label)
            .unwrap_or("unknown")
            .to_string();
        *state_counts.entry(label).or_default() += 1;
        if !query.is_empty() && !class_path.to_ascii_lowercase().contains(query) {
            continue;
        }
        matches.push((class_path.to_string(), state));
    }
    matches.sort_by(|a, b| a.0.cmp(&b.0));
    let total = matches.len();
    let quests = matches
        .into_iter()
        .skip(offset)
        .take(limit)
        .map(|(class_path, state)| {
            let id = class_path
                .rsplit('.')
                .next()
                .unwrap_or(class_path.as_str())
                .to_string();
            let trimmed = id.strip_prefix("Quest_").unwrap_or(&id);
            let (group, name) = match trimmed.split_once('_') {
                Some((g, n)) => (g.to_string(), n.to_string()),
                None => (trimmed.to_string(), String::new()),
            };
            let mut state_path = base_path.clone();
            state_path.push(format!("{{{class_path}}}"));
            state_path.push("CurrentState".to_string());
            json!({
                "questClass": class_path,
                "id": id,
                "group": group,
                "name": name,
                "currentState": state,
                "statePath": state_path,
                "writable": state.is_some(),
            })
        })
        .collect::<Vec<_>>();
    Ok(json!({
        "section": "quests",
        "total": total,
        "offset": offset,
        "limit": limit,
        "count": quests.len(),
        "stateCounts": state_counts,
        "quests": quests,
    }))
}
```

Wait — `state` is moved into the closure before `"writable": state.is_some()` can read it; compute `let writable = state.is_some();` first, then build the json. Write it that way:

```rust
            let writable = state.is_some();
            // ... json!({ ..., "currentState": state, "statePath": state_path, "writable": writable })
```

Add placeholder stubs so the file compiles before Task 6 (they are replaced there):

```rust
fn progression_knowledge(
    _root: &properties::RootObject,
    _query: &str,
    _character: Option<&str>,
    _offset: usize,
    _limit: usize,
) -> Result<Value, CoreError> {
    Err(CoreError::InvalidRequest(
        "knowledge section not implemented yet".to_string(),
    ))
}

fn progression_events(
    _root: &properties::RootObject,
    _query: &str,
    _character: Option<&str>,
    _offset: usize,
    _limit: usize,
) -> Result<Value, CoreError> {
    Err(CoreError::InvalidRequest(
        "events section not implemented yet".to_string(),
    ))
}
```

- [ ] **Step 4: Run to verify it passes**

Run: `cargo test -p goresave_core query_progression_lists_quests`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/goresave_core/src/lib.rs
git commit -m "feat(core): query_progression command with quests section"
```

---

### Task 6: Rust core — knowledge and events sections

**Files:**
- Modify: `crates/goresave_core/src/lib.rs` (replace the Task 5 stubs; tests at the bottom)

- [ ] **Step 1: Write the failing tests**

The fixture nests both maps the way real saves do (map values as tagged struct property lists). Reuse `private_name_set_property` from Task 4 and the builders below:

```rust
    fn private_double_property(name: &str, value: f64) -> Vec<u8> {
        let mut out = fstring(name);
        out.extend_from_slice(&fstring("DoubleProperty"));
        out.extend_from_slice(&0u32.to_le_bytes());
        out.extend_from_slice(&8u32.to_le_bytes());
        out.push(0);
        out.extend_from_slice(&value.to_le_bytes());
        out
    }

    fn private_struct_property(name: &str, struct_type: &str, body: &[u8], flags: u8) -> Vec<u8> {
        let mut out = fstring(name);
        out.extend_from_slice(&fstring("StructProperty"));
        out.extend_from_slice(&1u32.to_le_bytes());
        out.extend_from_slice(&fstring(struct_type));
        out.extend_from_slice(&1u32.to_le_bytes());
        out.extend_from_slice(&fstring("/Script/G1R"));
        out.extend_from_slice(&0u32.to_le_bytes());
        out.extend_from_slice(&(body.len() as u32).to_le_bytes());
        out.push(flags);
        out.extend_from_slice(&body);
        out
    }

    fn name_keyed_struct_map(map_name: &str, entries: &[(&str, Vec<u8>)]) -> Vec<u8> {
        let mut map_body = 0u32.to_le_bytes().to_vec();
        map_body.extend_from_slice(&(entries.len() as u32).to_le_bytes());
        for (key, value_props) in entries {
            map_body.extend_from_slice(&fstring(key));
            map_body.extend_from_slice(value_props);
            map_body.extend_from_slice(&fstring("None"));
        }
        let mut out = fstring(map_name);
        out.extend_from_slice(&fstring("MapProperty"));
        out.extend_from_slice(&2u32.to_le_bytes());
        out.extend_from_slice(&fstring("NameProperty"));
        out.extend_from_slice(&0u32.to_le_bytes());
        out.extend_from_slice(&fstring("StructProperty"));
        out.extend_from_slice(&1u32.to_le_bytes());
        out.extend_from_slice(&fstring("KnowledgeSet"));
        out.extend_from_slice(&1u32.to_le_bytes());
        out.extend_from_slice(&fstring("/Script/G1R"));
        out.extend_from_slice(&0u32.to_le_bytes());
        out.extend_from_slice(&(map_body.len() as u32).to_le_bytes());
        out.push(0);
        out.extend_from_slice(&map_body);
        out
    }
```

Note on the map-value layout: struct map values are inline tagged property lists terminated by `"None"` (see `read_inline_value` → `read_struct_value` with `native = false` — `is_native_struct_type("KnowledgeSet")` is false). The `name_keyed_struct_map` builder above appends the terminator itself; pass `value_props` WITHOUT a trailing `None`.

```rust
    #[test]
    fn query_progression_knowledge_lists_characters_and_entries() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("G1R-knowledge.sav");
        let private_payload = {
            let diego = private_name_set_property(
                "Knowledge",
                &["Voiceline_info_diego_gamestart_11_00", "ChoiceDiegoGamestart"],
            );
            let xardas = private_name_set_property("Knowledge", &["Voiceline_xardas_intro"]);
            let map = name_keyed_struct_map(
                "CharacterKnowledgeByUniqueName",
                &[("OC_STT_Diego", diego), ("NoneCamp_Xardas", xardas)],
            );
            let mut p = fstring("/Script/Angelscript.GothicFinalDataGame");
            p.push(0);
            p.extend_from_slice(&map);
            p.extend_from_slice(&fstring("None"));
            p.extend_from_slice(&0u32.to_le_bytes());
            p
        };
        let seed_compressed = b"seed".to_vec();
        let stream = compressed_stream_with_one_chunk(&seed_compressed, private_payload.len());
        fs::write(
            &path,
            build_gsav(2, &public_payload("Slot"), &stream, &[0, 0, 0, 0]),
        )
        .unwrap();
        let backend = PrefixCodecBackend {
            seed_compressed,
            seed_uncompressed: private_payload,
        };

        // Character list (no `character` param).
        let value =
            query_progression(&path, &json!({ "section": "knowledge" }), Some(&backend)).unwrap();
        assert_eq!(value["total"], 2);
        // Sorted by name: NoneCamp_Xardas before OC_STT_Diego.
        assert_eq!(value["characters"][0]["name"], "NoneCamp_Xardas");
        assert_eq!(value["characters"][0]["entryCount"], 1);
        assert_eq!(value["characters"][1]["name"], "OC_STT_Diego");
        assert_eq!(value["characters"][1]["entryCount"], 2);

        // Entries for one character.
        let value = query_progression(
            &path,
            &json!({ "section": "knowledge", "character": "OC_STT_Diego" }),
            Some(&backend),
        )
        .unwrap();
        assert_eq!(value["character"], "OC_STT_Diego");
        assert_eq!(value["total"], 2);
        assert_eq!(
            value["setPath"],
            json!([
                "CharacterKnowledgeByUniqueName",
                "{OC_STT_Diego}",
                "Knowledge"
            ])
        );
        let entries = value["entries"].as_array().unwrap();
        assert!(entries.contains(&json!("ChoiceDiegoGamestart")));

        // Query filter on entries.
        let value = query_progression(
            &path,
            &json!({ "section": "knowledge", "character": "OC_STT_Diego", "query": "choice" }),
            Some(&backend),
        )
        .unwrap();
        assert_eq!(value["total"], 1);

        // Unknown character errors.
        assert!(
            query_progression(
                &path,
                &json!({ "section": "knowledge", "character": "Nobody" }),
                Some(&backend),
            )
            .is_err()
        );
    }

    #[test]
    fn query_progression_events_lists_characters_and_events() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("G1R-events.sav");
        let private_payload = {
            // One memory event struct: EventTags (native GameplayTagContainer)
            // + Time (InGameTime property list) + AffectedCharacterGlobalId.
            let event = {
                let mut tags_body = 1u32.to_le_bytes().to_vec();
                tags_body.extend_from_slice(&fstring("Memory.Quest.Started"));
                let mut e = private_struct_property(
                    "EventTags",
                    "GameplayTagContainer",
                    &tags_body,
                    TAG_FLAG_NATIVE_SERIALIZE,
                );
                let time_body = {
                    let mut t = private_double_property("TotalSeconds", 1234.5);
                    t.extend_from_slice(&fstring("None"));
                    t
                };
                e.extend_from_slice(&private_struct_property(
                    "Time", "InGameTime", &time_body, 0,
                ));
                let mut affected = fstring("AffectedCharacterGlobalId");
                affected.extend_from_slice(&fstring("NameProperty"));
                affected.extend_from_slice(&0u32.to_le_bytes());
                let hero = fstring("Hero");
                affected.extend_from_slice(&(hero.len() as u32).to_le_bytes());
                affected.push(0);
                affected.extend_from_slice(&hero);
                e.extend_from_slice(&affected);
                e
            };
            // MemorizedEvents: ArrayProperty of MemoryEvent structs (inline
            // tagged property lists, "None"-terminated).
            let memorized = {
                let mut element = event.clone();
                element.extend_from_slice(&fstring("None"));
                let mut body = 1u32.to_le_bytes().to_vec();
                body.extend_from_slice(&element);
                let mut out = fstring("MemorizedEvents");
                out.extend_from_slice(&fstring("ArrayProperty"));
                out.extend_from_slice(&1u32.to_le_bytes());
                out.extend_from_slice(&fstring("StructProperty"));
                out.extend_from_slice(&1u32.to_le_bytes());
                out.extend_from_slice(&fstring("MemoryEvent"));
                out.extend_from_slice(&1u32.to_le_bytes());
                out.extend_from_slice(&fstring("/Script/G1R"));
                out.extend_from_slice(&0u32.to_le_bytes());
                out.extend_from_slice(&(body.len() as u32).to_le_bytes());
                out.push(0);
                out.extend_from_slice(&body);
                out
            };
            let map = name_keyed_struct_map("LongTermMemoryByGlobalId", &[("Hero", memorized)]);
            let mut p = fstring("/Script/Angelscript.GothicFinalDataGame");
            p.push(0);
            p.extend_from_slice(&map);
            p.extend_from_slice(&fstring("None"));
            p.extend_from_slice(&0u32.to_le_bytes());
            p
        };
        let seed_compressed = b"seed".to_vec();
        let stream = compressed_stream_with_one_chunk(&seed_compressed, private_payload.len());
        fs::write(
            &path,
            build_gsav(2, &public_payload("Slot"), &stream, &[0, 0, 0, 0]),
        )
        .unwrap();
        let backend = PrefixCodecBackend {
            seed_compressed,
            seed_uncompressed: private_payload,
        };

        let value =
            query_progression(&path, &json!({ "section": "events" }), Some(&backend)).unwrap();
        assert_eq!(value["total"], 1);
        assert_eq!(value["characters"][0]["id"], "Hero");
        assert_eq!(value["characters"][0]["eventCount"], 1);

        let value = query_progression(
            &path,
            &json!({ "section": "events", "character": "Hero" }),
            Some(&backend),
        )
        .unwrap();
        assert_eq!(value["character"], "Hero");
        assert_eq!(value["total"], 1);
        assert_eq!(
            value["arrayPath"],
            json!(["LongTermMemoryByGlobalId", "{Hero}", "MemorizedEvents"])
        );
        let event = &value["events"][0];
        assert_eq!(event["index"], 0);
        assert_eq!(event["tags"], json!(["Memory.Quest.Started"]));
        assert_eq!(event["timeSeconds"], 1234.5);
        assert_eq!(event["affected"], "Hero");

        // Tag query filter.
        let filtered = query_progression(
            &path,
            &json!({ "section": "events", "character": "Hero", "query": "guild" }),
            Some(&backend),
        )
        .unwrap();
        assert_eq!(filtered["total"], 0);
    }
```

Adjust the `name_keyed_struct_map` struct-type descriptor per use if needed — the value struct type string is cosmetic for the parser (`KnowledgeSet` vs a memory holder); using one name everywhere is fine since `is_native_struct_type` only checks known native names.

- [ ] **Step 2: Run to verify they fail**

Run: `cargo test -p goresave_core query_progression`
Expected: knowledge/events tests FAIL with "not implemented yet".

- [ ] **Step 3: Replace the stubs**

```rust
fn progression_knowledge(
    root: &properties::RootObject,
    query: &str,
    character: Option<&str>,
    offset: usize,
    limit: usize,
) -> Result<Value, CoreError> {
    let (base_path, map_prop) =
        properties::find_property_by_name(root, "CharacterKnowledgeByUniqueName").ok_or_else(
            || {
                CoreError::Parse(
                    "CharacterKnowledgeByUniqueName not found in the decoded payload".to_string(),
                )
            },
        )?;
    let properties::PropertyValue::Map { entries, .. } = &map_prop.value else {
        return Err(CoreError::Parse(
            "CharacterKnowledgeByUniqueName is not a map".to_string(),
        ));
    };
    let knowledge_entries = |value: &properties::PropertyValue| -> Vec<String> {
        match struct_member(value, "Knowledge") {
            Some(properties::PropertyValue::Set { elements, .. }) => elements
                .iter()
                .filter_map(|e| match e {
                    properties::PropertyValue::Name(s) | properties::PropertyValue::Str(s) => {
                        Some(s.clone())
                    }
                    _ => None,
                })
                .collect(),
            _ => Vec::new(),
        }
    };
    match character {
        None => {
            let mut characters: Vec<(String, usize)> = entries
                .iter()
                .filter_map(|(key, value)| {
                    let name = map_key_string(key)?;
                    if !query.is_empty() && !name.to_ascii_lowercase().contains(query) {
                        return None;
                    }
                    Some((name.to_string(), knowledge_entries(value).len()))
                })
                .collect();
            characters.sort_by(|a, b| a.0.cmp(&b.0));
            let total = characters.len();
            let page = characters
                .into_iter()
                .skip(offset)
                .take(limit)
                .map(|(name, entry_count)| json!({ "name": name, "entryCount": entry_count }))
                .collect::<Vec<_>>();
            Ok(json!({
                "section": "knowledge",
                "total": total,
                "offset": offset,
                "limit": limit,
                "count": page.len(),
                "characters": page,
            }))
        }
        Some(character) => {
            let value = entries
                .iter()
                .find(|(key, _)| map_key_string(key) == Some(character))
                .map(|(_, value)| value)
                .ok_or_else(|| {
                    CoreError::Parse(format!("character {character:?} has no knowledge entry"))
                })?;
            let mut all = knowledge_entries(value);
            if !query.is_empty() {
                all.retain(|e| e.to_ascii_lowercase().contains(query));
            }
            let total = all.len();
            let page = all
                .into_iter()
                .skip(offset)
                .take(limit)
                .collect::<Vec<_>>();
            let mut set_path = base_path.clone();
            set_path.push(format!("{{{character}}}"));
            set_path.push("Knowledge".to_string());
            Ok(json!({
                "section": "knowledge",
                "character": character,
                "total": total,
                "offset": offset,
                "limit": limit,
                "count": page.len(),
                "entries": page,
                "setPath": set_path,
            }))
        }
    }
}

fn progression_events(
    root: &properties::RootObject,
    query: &str,
    character: Option<&str>,
    offset: usize,
    limit: usize,
) -> Result<Value, CoreError> {
    let (base_path, map_prop) =
        properties::find_property_by_name(root, "LongTermMemoryByGlobalId").ok_or_else(|| {
            CoreError::Parse(
                "LongTermMemoryByGlobalId not found in the decoded payload".to_string(),
            )
        })?;
    let properties::PropertyValue::Map { entries, .. } = &map_prop.value else {
        return Err(CoreError::Parse(
            "LongTermMemoryByGlobalId is not a map".to_string(),
        ));
    };
    let memorized = |value: &properties::PropertyValue| -> Option<usize> {
        match struct_member(value, "MemorizedEvents") {
            Some(properties::PropertyValue::Array { elements }) => Some(elements.len()),
            _ => None,
        }
    };
    match character {
        None => {
            let mut characters: Vec<(String, usize)> = entries
                .iter()
                .filter_map(|(key, value)| {
                    let id = map_key_string(key)?;
                    if !query.is_empty() && !id.to_ascii_lowercase().contains(query) {
                        return None;
                    }
                    Some((id.to_string(), memorized(value).unwrap_or(0)))
                })
                .collect();
            characters.sort_by(|a, b| a.0.cmp(&b.0));
            let total = characters.len();
            let page = characters
                .into_iter()
                .skip(offset)
                .take(limit)
                .map(|(id, event_count)| json!({ "id": id, "eventCount": event_count }))
                .collect::<Vec<_>>();
            Ok(json!({
                "section": "events",
                "total": total,
                "offset": offset,
                "limit": limit,
                "count": page.len(),
                "characters": page,
            }))
        }
        Some(character) => {
            let value = entries
                .iter()
                .find(|(key, _)| map_key_string(key) == Some(character))
                .map(|(_, value)| value)
                .ok_or_else(|| {
                    CoreError::Parse(format!("character {character:?} has no memory entry"))
                })?;
            let Some(properties::PropertyValue::Array { elements }) =
                struct_member(value, "MemorizedEvents")
            else {
                return Err(CoreError::Parse(
                    "MemorizedEvents missing or not an array".to_string(),
                ));
            };
            let event_json = |index: usize, element: &properties::PropertyValue| -> Value {
                let tags = match struct_member(element, "EventTags") {
                    Some(properties::PropertyValue::Struct(
                        properties::StructValue::GameplayTagContainer(tags),
                    )) => tags.clone(),
                    _ => Vec::new(),
                };
                let in_game_seconds = |name: &str| -> Option<f64> {
                    match struct_member(element, name).and_then(|t| struct_member(t, "TotalSeconds"))
                    {
                        Some(properties::PropertyValue::Double(v)) => Some(*v),
                        _ => None,
                    }
                };
                let name_member = |name: &str| -> Option<String> {
                    match struct_member(element, name) {
                        Some(properties::PropertyValue::Name(s)) => Some(s.clone()),
                        _ => None,
                    }
                };
                let soft_member = |name: &str| -> Option<String> {
                    match struct_member(element, name) {
                        Some(properties::PropertyValue::SoftObject(p))
                            if !p.package_name.is_empty() && p.package_name != "None" =>
                        {
                            Some(p.package_name.clone())
                        }
                        _ => None,
                    }
                };
                let magnitude = match struct_member(element, "Magnitude") {
                    Some(properties::PropertyValue::Float(v)) => Some(f64::from(*v)),
                    _ => None,
                };
                json!({
                    "index": index,
                    "tags": tags,
                    "magnitude": magnitude,
                    "timeSeconds": in_game_seconds("Time"),
                    "durationSeconds": in_game_seconds("Duration"),
                    "instigator": name_member("InstigatorGlobalId"),
                    "affected": name_member("AffectedCharacterGlobalId"),
                    "optionalClass1": soft_member("OptionalClass1"),
                    "optionalClass2": soft_member("OptionalClass2"),
                })
            };
            let matches_query = |event: &Value| -> bool {
                if query.is_empty() {
                    return true;
                }
                let hay = [
                    event["tags"].to_string(),
                    event["instigator"].to_string(),
                    event["affected"].to_string(),
                    event["optionalClass1"].to_string(),
                    event["optionalClass2"].to_string(),
                ]
                .join(" ")
                .to_ascii_lowercase();
                hay.contains(query)
            };
            let all: Vec<Value> = elements
                .iter()
                .enumerate()
                .map(|(index, element)| event_json(index, element))
                .filter(matches_query)
                .collect();
            let total = all.len();
            let page = all.into_iter().skip(offset).take(limit).collect::<Vec<_>>();
            let mut array_path = base_path.clone();
            array_path.push(format!("{{{character}}}"));
            array_path.push("MemorizedEvents".to_string());
            Ok(json!({
                "section": "events",
                "character": character,
                "total": total,
                "offset": offset,
                "limit": limit,
                "count": page.len(),
                "events": page,
                "arrayPath": array_path,
            }))
        }
    }
}
```

Note: `.filter(matches_query)` over `Value` items needs `|e| matches_query(e)`; write the closure form that compiles.

- [ ] **Step 4: Run to verify they pass**

Run: `cargo test -p goresave_core query_progression`
Expected: all 3 query_progression tests PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/goresave_core/src/lib.rs
git commit -m "feat(core): query_progression knowledge and events sections"
```

---

### Task 7: Rust core — structured progression overview in inspect; delete the heuristic

`inspect_save` parses the typed root once (it already does, inside `summarize_typed_parse`), shares it with a new structured progression overview, and the old string-grep progression summary is deleted.

**Files:**
- Modify: `crates/goresave_core/src/lib.rs`
  - inspect decoded branch (~line 2121–2151)
  - `summarize_typed_parse` (~line 2292)
  - delete `summarize_private_progression_payload` (~line 2439), `looks_progression_candidate`, `looks_gameplay_tag_candidate`, `looks_progression_text`, `progression_section_label` (~lines 2669–2719)
  - tests

- [ ] **Step 1: Write the failing test**

Reuse `quest_map_payload()` from Task 5 (extend it if convenient, or build a combined fixture):

```rust
    #[test]
    fn inspect_reports_structured_progression_overview() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("G1R-overview.sav");
        let private_payload = quest_map_payload();
        let seed_compressed = b"seed".to_vec();
        let stream = compressed_stream_with_one_chunk(&seed_compressed, private_payload.len());
        fs::write(
            &path,
            build_gsav(2, &public_payload("Slot"), &stream, &[0, 0, 0, 0]),
        )
        .unwrap();
        let backend = PrefixCodecBackend {
            seed_compressed,
            seed_uncompressed: private_payload,
        };

        let value = inspect_save_with_codec_backend(&path, true, Some(&backend), None).unwrap();
        let progression = &value["private"]["progression"];
        assert_eq!(progression["status"], "ok");
        assert_eq!(progression["questTotal"], 2);
        assert_eq!(progression["questStates"]["Running"], 1);
        assert_eq!(progression["questStates"]["Available"], 1);
        // No knowledge/memory maps in this fixture.
        assert_eq!(progression["knowledgeCharacters"], 0);
        assert_eq!(progression["memoryCharacters"], 0);
        assert!(
            progression["writable"]
                .as_array()
                .unwrap()
                .contains(&json!("private.typed.setValue"))
        );
        // The old heuristic fields are gone.
        assert!(progression.get("candidates").is_none());
        assert!(progression.get("sections").is_none());
    }
```

- [ ] **Step 2: Run to verify it fails**

Run: `cargo test -p goresave_core inspect_reports_structured_progression_overview`
Expected: FAIL — progression still has the heuristic shape.

- [ ] **Step 3: Implement**

(a) Refactor `summarize_typed_parse` so the parse result is reusable. Change its signature and split the parse out:

```rust
fn summarize_typed_parse_result(
    payload: &[u8],
    result: Option<&Result<properties::RootObject, CoreError>>,
) -> Value {
    let Some(result) = result else {
        return json!({
            "status": "skipped_preview",
            "message": "Typed parse needs the full decoded payload.",
        });
    };
    match result {
        Ok(root) => { /* move the existing Ok-arm body of summarize_typed_parse here */ }
        Err(err) => { /* move the existing Err-arm body here */ }
    }
}
```

Keep a thin compatibility wrapper only if other call sites exist (grep for `summarize_typed_parse(`); otherwise delete the old function.

(b) In the inspect decoded branch (~line 2121), replace:

```rust
            let progression = summarize_private_progression_payload(&refs);
            let typed_parse = summarize_typed_parse(&payload, preview);
```

with:

```rust
            let typed_result = if preview {
                None
            } else {
                Some(properties::parse_private_root(&payload))
            };
            let typed_parse = summarize_typed_parse_result(&payload, typed_result.as_ref());
            let typed_ok = typed_parse["status"] == "ok";
            let progression = summarize_private_progression_overview(
                typed_result.as_ref().and_then(|r| r.as_ref().ok()).filter(|_| typed_ok),
            );
```

(c) New overview function (place near the other summarize functions). It reuses `struct_member`/`map_key_string`/`short_enum_label` from Task 5:

```rust
/// Structured progression overview for the inspect response: quest counts by
/// state plus knowledge/memory totals. `root` is Some only when the strict
/// typed parse succeeded on a full (non-preview) decode.
fn summarize_private_progression_overview(root: Option<&properties::RootObject>) -> Value {
    let Some(root) = root else {
        return json!({ "status": "unavailable", "writable": [] });
    };
    let mut quest_total = 0usize;
    let mut quest_states = std::collections::BTreeMap::<String, usize>::new();
    if let Some((_, prop)) = properties::find_property_by_name(root, "QuestDataByClass") {
        if let properties::PropertyValue::Map { entries, .. } = &prop.value {
            quest_total = entries.len();
            for (_, value) in entries {
                let label = match struct_member(value, "CurrentState") {
                    Some(properties::PropertyValue::Enum(s)) => short_enum_label(s).to_string(),
                    _ => "unknown".to_string(),
                };
                *quest_states.entry(label).or_default() += 1;
            }
        }
    }
    let mut knowledge_characters = 0usize;
    let mut knowledge_entries = 0usize;
    if let Some((_, prop)) =
        properties::find_property_by_name(root, "CharacterKnowledgeByUniqueName")
    {
        if let properties::PropertyValue::Map { entries, .. } = &prop.value {
            knowledge_characters = entries.len();
            for (_, value) in entries {
                if let Some(properties::PropertyValue::Set { elements, .. }) =
                    struct_member(value, "Knowledge")
                {
                    knowledge_entries += elements.len();
                }
            }
        }
    }
    let mut memory_characters = 0usize;
    let mut memory_events = 0usize;
    if let Some((_, prop)) = properties::find_property_by_name(root, "LongTermMemoryByGlobalId") {
        if let properties::PropertyValue::Map { entries, .. } = &prop.value {
            memory_characters = entries.len();
            for (_, value) in entries {
                if let Some(properties::PropertyValue::Array { elements }) =
                    struct_member(value, "MemorizedEvents")
                {
                    memory_events += elements.len();
                }
            }
        }
    }
    json!({
        "status": "ok",
        "questTotal": quest_total,
        "questStates": quest_states,
        "knowledgeCharacters": knowledge_characters,
        "knowledgeEntries": knowledge_entries,
        "memoryCharacters": memory_characters,
        "memoryEvents": memory_events,
        "writable": [
            "private.typed.setValue",
            "private.typed.setAdd",
            "private.typed.setRemove",
            "private.typed.arrayRemove",
            "private.typed.arrayDuplicate",
        ],
    })
}
```

(d) Delete `summarize_private_progression_payload`, `looks_progression_candidate`, `looks_gameplay_tag_candidate`, `looks_progression_text`, and `progression_section_label`. Run `cargo build -p goresave_core` and chase dangling references; grep the test module for tests asserting the old progression shape (`grep -n "progression" crates/goresave_core/src/lib.rs`) and update them to the new overview shape (counts instead of candidate lists).

- [ ] **Step 4: Run the full suite**

Run: `cargo test -p goresave_core`
Expected: all green, including the new overview test.

- [ ] **Step 5: Commit**

```bash
git add crates/goresave_core/src/lib.rs
git commit -m "feat(core): structured progression overview, drop string-grep heuristic"
```

---

### Task 8: Dart — progression models

New file with section page models and edit intents; `SaveInspection` switches from `PrivateProgressionSummary` to the new overview.

**Files:**
- Create: `apps/goresave/lib/features/editor/domain/progression_models.dart`
- Modify: `apps/goresave/lib/features/editor/domain/editor_models.dart` (replace `PrivateProgressionSummary` usage in `SaveInspection`; delete the class)
- Test: `apps/goresave/test/features/editor/progression_models_test.dart` (create `test/features/editor/` if missing)

- [ ] **Step 1: Write the failing tests**

```dart
import 'package:flutter_test/flutter_test.dart';
import 'package:goresave/features/editor/domain/progression_models.dart';

void main() {
  test('ProgressionOverview parses inspect json', () {
    final overview = ProgressionOverview.fromJson({
      'status': 'ok',
      'questTotal': 707,
      'questStates': {'Available': 700, 'Running': 5, 'Succeeded': 2},
      'knowledgeCharacters': 12,
      'knowledgeEntries': 340,
      'memoryCharacters': 3,
      'memoryEvents': 1500,
      'writable': ['private.typed.setValue', 'private.typed.setAdd'],
    });
    expect(overview.status, 'ok');
    expect(overview.available, isTrue);
    expect(overview.questTotal, 707);
    expect(overview.questStates['Running'], 5);
    expect(overview.knowledgeCharacters, 12);
    expect(overview.memoryEvents, 1500);
    expect(overview.writable, contains('private.typed.setAdd'));

    final unavailable = ProgressionOverview.fromJson({'status': 'unavailable'});
    expect(unavailable.available, isFalse);
  });

  test('ProgressionQuestPage parses query json', () {
    final page = ProgressionQuestPage.fromJson({
      'total': 2,
      'offset': 0,
      'limit': 100,
      'stateCounts': {'Available': 1, 'Running': 1},
      'quests': [
        {
          'questClass': '/Script/Angelscript.Quest_OldCamp_SLEEPER',
          'id': 'Quest_OldCamp_SLEEPER',
          'group': 'OldCamp',
          'name': 'SLEEPER',
          'currentState': 'EQuestState::Running',
          'statePath': [
            'QuestDataByClass',
            '{/Script/Angelscript.Quest_OldCamp_SLEEPER}',
            'CurrentState',
          ],
          'writable': true,
        },
      ],
    });
    expect(page.total, 2);
    expect(page.quests.single.group, 'OldCamp');
    expect(page.quests.single.currentState, 'EQuestState::Running');
    expect(page.quests.single.writable, isTrue);
    expect(page.quests.single.statePath, hasLength(3));
  });

  test('edit intents emit core edit json', () {
    final questEdit = QuestStateChange(
      statePath: const ['QuestDataByClass', '{X}', 'CurrentState'],
      state: 'EQuestState::Succeeded',
    );
    expect(questEdit.toEditJson(), {
      'path': 'private.typed.setValue',
      'value': {
        'path': ['QuestDataByClass', '{X}', 'CurrentState'],
        'value': 'EQuestState::Succeeded',
      },
    });

    final add = KnowledgeEntryEdit.add(
      setPath: const ['CharacterKnowledgeByUniqueName', '{Diego}', 'Knowledge'],
      entry: 'Voiceline_X',
    );
    expect(add.toEditJson(), {
      'path': 'private.typed.setAdd',
      'value': {
        'path': ['CharacterKnowledgeByUniqueName', '{Diego}', 'Knowledge'],
        'value': 'Voiceline_X',
      },
    });

    final remove = KnowledgeEntryEdit.remove(
      setPath: const ['CharacterKnowledgeByUniqueName', '{Diego}', 'Knowledge'],
      entry: 'Voiceline_X',
    );
    expect(remove.toEditJson()['path'], 'private.typed.setRemove');

    final removeEvent = MemoryEventEdit.remove(
      arrayPath: const ['LongTermMemoryByGlobalId', '{Hero}', 'MemorizedEvents'],
      index: 4,
    );
    expect(removeEvent.toEditJson(), {
      'path': 'private.typed.arrayRemove',
      'value': {
        'path': ['LongTermMemoryByGlobalId', '{Hero}', 'MemorizedEvents'],
        'index': 4,
      },
    });

    final duplicate = MemoryEventEdit.duplicate(
      arrayPath: const ['LongTermMemoryByGlobalId', '{Hero}', 'MemorizedEvents'],
      index: 4,
    );
    expect(duplicate.toEditJson()['path'], 'private.typed.arrayDuplicate');
  });

  test('knowledge and event pages parse', () {
    final chars = KnowledgeCharactersPage.fromJson({
      'total': 1,
      'offset': 0,
      'limit': 100,
      'characters': [
        {'name': 'OC_STT_Diego', 'entryCount': 2},
      ],
    });
    expect(chars.characters.single.name, 'OC_STT_Diego');

    final entries = KnowledgeEntriesPage.fromJson({
      'character': 'OC_STT_Diego',
      'total': 2,
      'offset': 0,
      'limit': 200,
      'entries': ['A', 'B'],
      'setPath': ['CharacterKnowledgeByUniqueName', '{OC_STT_Diego}', 'Knowledge'],
    });
    expect(entries.entries, ['A', 'B']);
    expect(entries.setPath, hasLength(3));

    final events = MemoryEventsPage.fromJson({
      'character': 'Hero',
      'total': 1,
      'offset': 0,
      'limit': 100,
      'events': [
        {
          'index': 0,
          'tags': ['Memory.Quest.Started'],
          'timeSeconds': 12.5,
          'affected': 'Hero',
        },
      ],
      'arrayPath': ['LongTermMemoryByGlobalId', '{Hero}', 'MemorizedEvents'],
    });
    expect(events.events.single.tags, ['Memory.Quest.Started']);
    expect(events.events.single.timeSeconds, 12.5);
  });
}
```

- [ ] **Step 2: Run to verify they fail**

Run (from `apps/goresave`): `flutter test test/features/editor/progression_models_test.dart`
Expected: FAIL to compile — file missing.

- [ ] **Step 3: Implement `progression_models.dart`**

```dart
/// Models for the Progression tab: the inspect overview, paged section
/// queries (quests / knowledge / events), and the edit intents that map to
/// core write ops. All pages carry an optional [error] (set by the notifier
/// instead of throwing) so cards can render failures inline.

class ProgressionOverview {
  const ProgressionOverview({
    this.status,
    this.questTotal = 0,
    this.questStates = const {},
    this.knowledgeCharacters = 0,
    this.knowledgeEntries = 0,
    this.memoryCharacters = 0,
    this.memoryEvents = 0,
    this.writable = const [],
  });

  factory ProgressionOverview.fromJson(Map<String, Object?>? json) {
    final states = <String, int>{};
    (json?['questStates'] as Map?)?.forEach((key, value) {
      if (key is String && value is num) states[key] = value.toInt();
    });
    return ProgressionOverview(
      status: json?['status'] as String?,
      questTotal: (json?['questTotal'] as num?)?.toInt() ?? 0,
      questStates: states,
      knowledgeCharacters: (json?['knowledgeCharacters'] as num?)?.toInt() ?? 0,
      knowledgeEntries: (json?['knowledgeEntries'] as num?)?.toInt() ?? 0,
      memoryCharacters: (json?['memoryCharacters'] as num?)?.toInt() ?? 0,
      memoryEvents: (json?['memoryEvents'] as num?)?.toInt() ?? 0,
      writable:
          (json?['writable'] as List?)?.whereType<String>().toList() ??
          const [],
    );
  }

  final String? status;
  final int questTotal;
  final Map<String, int> questStates;
  final int knowledgeCharacters;
  final int knowledgeEntries;
  final int memoryCharacters;
  final int memoryEvents;
  final List<String> writable;

  bool get available => status == 'ok';
}

class ProgressionQuest {
  const ProgressionQuest({
    required this.questClass,
    required this.id,
    required this.group,
    required this.name,
    required this.statePath,
    this.currentState,
    this.writable = false,
  });

  factory ProgressionQuest.fromJson(Map<String, Object?> json) {
    return ProgressionQuest(
      questClass: json['questClass'] as String? ?? '',
      id: json['id'] as String? ?? '',
      group: json['group'] as String? ?? '',
      name: json['name'] as String? ?? '',
      currentState: json['currentState'] as String?,
      statePath:
          (json['statePath'] as List?)
              ?.whereType<String>()
              .toList(growable: false) ??
          const [],
      writable: json['writable'] as bool? ?? false,
    );
  }

  final String questClass;
  final String id;
  final String group;
  final String name;
  final String? currentState;
  final List<String> statePath;
  final bool writable;
}

class ProgressionQuestPage {
  const ProgressionQuestPage({
    this.quests = const [],
    this.stateCounts = const {},
    this.total = 0,
    this.offset = 0,
    this.limit = 100,
    this.error,
  });

  factory ProgressionQuestPage.fromJson(Map<String, Object?> json) {
    final counts = <String, int>{};
    (json['stateCounts'] as Map?)?.forEach((key, value) {
      if (key is String && value is num) counts[key] = value.toInt();
    });
    return ProgressionQuestPage(
      quests:
          (json['quests'] as List?)
              ?.whereType<Map>()
              .map((e) => ProgressionQuest.fromJson(e.cast<String, Object?>()))
              .toList(growable: false) ??
          const [],
      stateCounts: counts,
      total: (json['total'] as num?)?.toInt() ?? 0,
      offset: (json['offset'] as num?)?.toInt() ?? 0,
      limit: (json['limit'] as num?)?.toInt() ?? 100,
    );
  }

  final List<ProgressionQuest> quests;
  final Map<String, int> stateCounts;
  final int total;
  final int offset;
  final int limit;
  final String? error;

  bool get hasMore => offset + quests.length < total;
}

class KnowledgeCharacter {
  const KnowledgeCharacter({required this.name, required this.entryCount});

  factory KnowledgeCharacter.fromJson(Map<String, Object?> json) {
    return KnowledgeCharacter(
      name: json['name'] as String? ?? '',
      entryCount: (json['entryCount'] as num?)?.toInt() ?? 0,
    );
  }

  final String name;
  final int entryCount;
}

class KnowledgeCharactersPage {
  const KnowledgeCharactersPage({
    this.characters = const [],
    this.total = 0,
    this.offset = 0,
    this.limit = 100,
    this.error,
  });

  factory KnowledgeCharactersPage.fromJson(Map<String, Object?> json) {
    return KnowledgeCharactersPage(
      characters:
          (json['characters'] as List?)
              ?.whereType<Map>()
              .map((e) => KnowledgeCharacter.fromJson(e.cast<String, Object?>()))
              .toList(growable: false) ??
          const [],
      total: (json['total'] as num?)?.toInt() ?? 0,
      offset: (json['offset'] as num?)?.toInt() ?? 0,
      limit: (json['limit'] as num?)?.toInt() ?? 100,
    );
  }

  final List<KnowledgeCharacter> characters;
  final int total;
  final int offset;
  final int limit;
  final String? error;

  bool get hasMore => offset + characters.length < total;
}

class KnowledgeEntriesPage {
  const KnowledgeEntriesPage({
    this.character = '',
    this.entries = const [],
    this.setPath = const [],
    this.total = 0,
    this.offset = 0,
    this.limit = 200,
    this.error,
  });

  factory KnowledgeEntriesPage.fromJson(Map<String, Object?> json) {
    return KnowledgeEntriesPage(
      character: json['character'] as String? ?? '',
      entries:
          (json['entries'] as List?)
              ?.whereType<String>()
              .toList(growable: false) ??
          const [],
      setPath:
          (json['setPath'] as List?)
              ?.whereType<String>()
              .toList(growable: false) ??
          const [],
      total: (json['total'] as num?)?.toInt() ?? 0,
      offset: (json['offset'] as num?)?.toInt() ?? 0,
      limit: (json['limit'] as num?)?.toInt() ?? 200,
    );
  }

  final String character;
  final List<String> entries;
  final List<String> setPath;
  final int total;
  final int offset;
  final int limit;
  final String? error;

  bool get hasMore => offset + entries.length < total;
}

class MemoryCharacter {
  const MemoryCharacter({required this.id, required this.eventCount});

  factory MemoryCharacter.fromJson(Map<String, Object?> json) {
    return MemoryCharacter(
      id: json['id'] as String? ?? '',
      eventCount: (json['eventCount'] as num?)?.toInt() ?? 0,
    );
  }

  final String id;
  final int eventCount;
}

class MemoryCharactersPage {
  const MemoryCharactersPage({
    this.characters = const [],
    this.total = 0,
    this.offset = 0,
    this.limit = 100,
    this.error,
  });

  factory MemoryCharactersPage.fromJson(Map<String, Object?> json) {
    return MemoryCharactersPage(
      characters:
          (json['characters'] as List?)
              ?.whereType<Map>()
              .map((e) => MemoryCharacter.fromJson(e.cast<String, Object?>()))
              .toList(growable: false) ??
          const [],
      total: (json['total'] as num?)?.toInt() ?? 0,
      offset: (json['offset'] as num?)?.toInt() ?? 0,
      limit: (json['limit'] as num?)?.toInt() ?? 100,
    );
  }

  final List<MemoryCharacter> characters;
  final int total;
  final int offset;
  final int limit;
  final String? error;
}

class MemoryEvent {
  const MemoryEvent({
    required this.index,
    this.tags = const [],
    this.magnitude,
    this.timeSeconds,
    this.durationSeconds,
    this.instigator,
    this.affected,
    this.optionalClass1,
    this.optionalClass2,
  });

  factory MemoryEvent.fromJson(Map<String, Object?> json) {
    return MemoryEvent(
      index: (json['index'] as num?)?.toInt() ?? 0,
      tags:
          (json['tags'] as List?)?.whereType<String>().toList(growable: false) ??
          const [],
      magnitude: (json['magnitude'] as num?)?.toDouble(),
      timeSeconds: (json['timeSeconds'] as num?)?.toDouble(),
      durationSeconds: (json['durationSeconds'] as num?)?.toDouble(),
      instigator: json['instigator'] as String?,
      affected: json['affected'] as String?,
      optionalClass1: json['optionalClass1'] as String?,
      optionalClass2: json['optionalClass2'] as String?,
    );
  }

  final int index;
  final List<String> tags;
  final double? magnitude;
  final double? timeSeconds;
  final double? durationSeconds;
  final String? instigator;
  final String? affected;
  final String? optionalClass1;
  final String? optionalClass2;
}

class MemoryEventsPage {
  const MemoryEventsPage({
    this.character = '',
    this.events = const [],
    this.arrayPath = const [],
    this.total = 0,
    this.offset = 0,
    this.limit = 100,
    this.error,
  });

  factory MemoryEventsPage.fromJson(Map<String, Object?> json) {
    return MemoryEventsPage(
      character: json['character'] as String? ?? '',
      events:
          (json['events'] as List?)
              ?.whereType<Map>()
              .map((e) => MemoryEvent.fromJson(e.cast<String, Object?>()))
              .toList(growable: false) ??
          const [],
      arrayPath:
          (json['arrayPath'] as List?)
              ?.whereType<String>()
              .toList(growable: false) ??
          const [],
      total: (json['total'] as num?)?.toInt() ?? 0,
      offset: (json['offset'] as num?)?.toInt() ?? 0,
      limit: (json['limit'] as num?)?.toInt() ?? 100,
    );
  }

  final String character;
  final List<MemoryEvent> events;
  final List<String> arrayPath;
  final int total;
  final int offset;
  final int limit;
  final String? error;

  bool get hasMore => offset + events.length < total;
}

/// Pending quest-state change → `private.typed.setValue`.
class QuestStateChange {
  const QuestStateChange({required this.statePath, required this.state});

  final List<String> statePath;
  final String state;

  Map<String, Object?> toEditJson() {
    return {
      'path': 'private.typed.setValue',
      'value': {'path': statePath, 'value': state},
    };
  }
}

/// Pending knowledge add/remove → `private.typed.setAdd` / `setRemove`.
class KnowledgeEntryEdit {
  const KnowledgeEntryEdit.add({required this.setPath, required this.entry})
      : isAdd = true;
  const KnowledgeEntryEdit.remove({required this.setPath, required this.entry})
      : isAdd = false;

  final List<String> setPath;
  final String entry;
  final bool isAdd;

  Map<String, Object?> toEditJson() {
    return {
      'path': isAdd ? 'private.typed.setAdd' : 'private.typed.setRemove',
      'value': {'path': setPath, 'value': entry},
    };
  }
}

/// Structural memory-event edit → `private.typed.arrayRemove` /
/// `arrayDuplicate`. Index-addressed, so the UI applies at most one per save
/// round (indices shift after each structural change).
class MemoryEventEdit {
  const MemoryEventEdit.remove({required this.arrayPath, required this.index})
      : isRemove = true;
  const MemoryEventEdit.duplicate({
    required this.arrayPath,
    required this.index,
  }) : isRemove = false;

  final List<String> arrayPath;
  final int index;
  final bool isRemove;

  Map<String, Object?> toEditJson() {
    return {
      'path': isRemove
          ? 'private.typed.arrayRemove'
          : 'private.typed.arrayDuplicate',
      'value': {'path': arrayPath, 'index': index},
    };
  }
}
```

- [ ] **Step 4: Rewire `SaveInspection`**

In `editor_models.dart`:
- Add `import 'package:goresave/features/editor/domain/progression_models.dart';` (match the file's existing import style — it currently has none for siblings, so a relative `import 'progression_models.dart';` also works; follow `analysis_options.yaml` lints).
- Replace the field `final PrivateProgressionSummary privateProgression;` with `final ProgressionOverview privateProgression;`, the constructor default with `this.privateProgression = const ProgressionOverview()`, and the `fromJson` line with `privateProgression: ProgressionOverview.fromJson(privateProgression)` (the local variable name already matches).
- Delete the `PrivateProgressionSummary` class.

- [ ] **Step 5: Run to verify**

Run: `flutter test test/features/editor/progression_models_test.dart`
Expected: PASS. Then `flutter analyze` — expect errors only in `editor_page.dart` (`_PrivateProgressionSummaryCard` still references the deleted class); that is fixed in Task 10. If the analyzer must be green per task, defer Step 4's deletion of `PrivateProgressionSummary` to Task 10 and keep both models temporarily — prefer the deferral only if needed.

- [ ] **Step 6: Commit**

```bash
git add apps/goresave/lib/features/editor/domain/progression_models.dart apps/goresave/lib/features/editor/domain/editor_models.dart apps/goresave/test/features/editor/progression_models_test.dart
git commit -m "feat(app): progression models and edit intents"
```

---

### Task 9: Dart — notifier query and apply methods

**Files:**
- Modify: `apps/goresave/lib/features/editor/domain/editor_notifier.dart` (next to `searchTypedProperties`, ~line 746)
- Test: `apps/goresave/test/editor_notifier_test.dart` (follow the existing fake-core pattern in that file)

- [ ] **Step 1: Write the failing test**

Open `apps/goresave/test/editor_notifier_test.dart` and follow its existing fake `GoresaveCoreService` pattern (a fake that records `execute` calls and returns canned responses). Add:

```dart
  test('loadProgressionQuests queries the core and parses the page', () async {
    // Arrange a fake core that returns one quest for 'query_progression'.
    // (Reuse the file's existing fake-service scaffolding; the canned response:)
    // {'ok': true, 'data': {'section': 'quests', 'total': 1, 'offset': 0,
    //  'limit': 100, 'count': 1, 'stateCounts': {'Running': 1}, 'quests': [
    //    {'questClass': '/Script/Angelscript.Quest_X', 'id': 'Quest_X',
    //     'group': 'X', 'name': '', 'currentState': 'EQuestState::Running',
    //     'statePath': ['QuestDataByClass', '{/Script/Angelscript.Quest_X}',
    //     'CurrentState'], 'writable': true}]}}
    // and a selected save path set beforehand (mirror how the hero-attribute
    // tests select a save).
    final page = await notifier.loadProgressionQuests(query: 'x');
    expect(page.error, isNull);
    expect(page.quests.single.id, 'Quest_X');
    // The core received section + query + path.
    final call = fakeCore.calls.singleWhere((c) => c.command == 'query_progression');
    expect(call.payload['section'], 'quests');
    expect(call.payload['query'], 'x');
  });

  test('progression loaders surface core errors inline', () async {
    // Fake core returns {'ok': false, 'error': {...}}.
    final page = await notifier.loadKnowledgeCharacters();
    expect(page.error, isNotNull);
  });
```

Adapt naming to the file's actual fake/service helpers — read the file first; do not invent a parallel scaffolding.

- [ ] **Step 2: Run to verify it fails**

Run: `flutter test test/editor_notifier_test.dart`
Expected: FAIL to compile — methods missing.

- [ ] **Step 3: Implement the notifier methods**

Add `import 'package:goresave/features/editor/domain/progression_models.dart';` and, next to `searchTypedProperties`:

```dart
  /// Run one progression section query. Returns the raw data map, or null
  /// with [onError] set, so each typed loader below can build its own page
  /// object with an inline error.
  Future<Map<String, Object?>?> _queryProgression(
    Map<String, Object?> params, {
    required void Function(String message) onError,
  }) async {
    final path = state.selectedPath;
    if (path == null) {
      onError('No save selected.');
      return null;
    }
    try {
      final response = await _execute(
        'query_progression',
        payload: {'path': path, ...params, ..._codecPayload()},
      );
      if (response['ok'] != true) {
        onError(_errorMessage(response));
        return null;
      }
      return (response['data'] as Map).cast<String, Object?>();
    } catch (error) {
      onError('Progression query failed: $error');
      return null;
    }
  }

  Future<ProgressionQuestPage> loadProgressionQuests({
    String query = '',
    int offset = 0,
    int limit = 100,
  }) async {
    String? error;
    final data = await _queryProgression(
      {'section': 'quests', 'query': query, 'offset': offset, 'limit': limit},
      onError: (message) => error = message,
    );
    if (data == null) return ProgressionQuestPage(error: error);
    return ProgressionQuestPage.fromJson(data);
  }

  Future<KnowledgeCharactersPage> loadKnowledgeCharacters({
    String query = '',
    int offset = 0,
    int limit = 100,
  }) async {
    String? error;
    final data = await _queryProgression(
      {
        'section': 'knowledge',
        'query': query,
        'offset': offset,
        'limit': limit,
      },
      onError: (message) => error = message,
    );
    if (data == null) return KnowledgeCharactersPage(error: error);
    return KnowledgeCharactersPage.fromJson(data);
  }

  Future<KnowledgeEntriesPage> loadKnowledgeEntries(
    String character, {
    String query = '',
    int offset = 0,
    int limit = 200,
  }) async {
    String? error;
    final data = await _queryProgression(
      {
        'section': 'knowledge',
        'character': character,
        'query': query,
        'offset': offset,
        'limit': limit,
      },
      onError: (message) => error = message,
    );
    if (data == null) return KnowledgeEntriesPage(error: error);
    return KnowledgeEntriesPage.fromJson(data);
  }

  Future<MemoryCharactersPage> loadMemoryCharacters({
    String query = '',
    int offset = 0,
    int limit = 100,
  }) async {
    String? error;
    final data = await _queryProgression(
      {'section': 'events', 'query': query, 'offset': offset, 'limit': limit},
      onError: (message) => error = message,
    );
    if (data == null) return MemoryCharactersPage(error: error);
    return MemoryCharactersPage.fromJson(data);
  }

  Future<MemoryEventsPage> loadMemoryEvents(
    String character, {
    String query = '',
    int offset = 0,
    int limit = 100,
  }) async {
    String? error;
    final data = await _queryProgression(
      {
        'section': 'events',
        'character': character,
        'query': query,
        'offset': offset,
        'limit': limit,
      },
      onError: (message) => error = message,
    );
    if (data == null) return MemoryEventsPage(error: error);
    return MemoryEventsPage.fromJson(data);
  }

  /// Apply one structural progression edit (event remove/duplicate)
  /// immediately, with backup. Index-addressed array edits must go one per
  /// write round — indices shift after every structural change — so this is
  /// intentionally not part of the pending-edit registry.
  Future<bool> applyMemoryEventEdit(MemoryEventEdit edit) async {
    final savePath = state.selectedPath;
    if (savePath == null) return false;
    if (state.isLoading) return false;
    return _runWrite(
      payload: {
        'path': savePath,
        'backup': true,
        'edits': [edit.toEditJson()],
        ..._codecPayload(),
      },
      message: (data) => _backupMessage(
        edit.isRemove ? 'Memory event removed' : 'Memory event duplicated',
        data,
      ),
    );
  }
```

- [ ] **Step 4: Run to verify it passes**

Run: `flutter test test/editor_notifier_test.dart`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add apps/goresave/lib/features/editor/domain/editor_notifier.dart apps/goresave/test/editor_notifier_test.dart
git commit -m "feat(app): progression query and apply methods on the notifier"
```

---

### Task 10: Dart — ProgressionPanel UI

New file with the four cards, wired into the Progression tab; the old heuristic card and `_StringList` are deleted.

**Files:**
- Create: `apps/goresave/lib/features/editor/ui/progression_panel.dart`
- Modify: `apps/goresave/lib/features/editor/ui/editor_page.dart`
  - replace the `_ProgressionPanel` body (~line 1412) with the new panel
  - delete `_PrivateProgressionSummaryCard` (~line 1447) and `_StringList` (~line 1582) — first `grep -n "_StringList" apps/goresave/lib` to confirm no other user

- [ ] **Step 1: Check the tab call site and gating**

Read how the Inventory tab gates editing (search `_InventoryPanel` ~line 1359 and where panels receive `notifier`): the panel receives `inspection` and `notifier`; editability combines `inspection.privateEditable`, `inspection.privateTypedVerified`, and the notifier state's `codecCompressReady`. Mirror exactly that gating for progression.

- [ ] **Step 2: Implement `progression_panel.dart`**

The panel skeleton (complete file — adapt private-widget naming to taste, keep the structure):

```dart
import 'package:flutter/material.dart';

import '../domain/editor_models.dart';
import '../domain/editor_notifier.dart';
import '../domain/pending_edits.dart';
import '../domain/progression_models.dart';

/// All five EQuestState values, in dropdown order.
const questStates = <String>[
  'EQuestState::None',
  'EQuestState::Available',
  'EQuestState::Running',
  'EQuestState::Succeeded',
  'EQuestState::Failed',
];

String shortStateLabel(String state) {
  final idx = state.lastIndexOf('::');
  return idx < 0 ? state : state.substring(idx + 2);
}

/// Progression tab: structured quests / dialog knowledge / memory events.
/// Data loads lazily per card through the notifier's query_progression
/// wrappers. [reloadKey] identifies the inspected save (sha1); when it
/// changes, cards drop local state and reload.
class ProgressionPanel extends StatelessWidget {
  const ProgressionPanel({
    super.key,
    required this.inspection,
    required this.notifier,
    required this.editable,
  });

  final SaveInspection inspection;
  final EditorNotifier notifier;
  final bool editable;

  @override
  Widget build(BuildContext context) {
    final overview = inspection.privateProgression;
    if (!inspection.privateDecoded) {
      return const _MessagePane(
        icon: Icons.flag_outlined,
        title: 'Progression',
        body:
            'Progression data needs decoded private payload data from the G1R codec host.',
      );
    }
    if (!overview.available) {
      return const _MessagePane(
        icon: Icons.flag_outlined,
        title: 'Progression',
        body:
            'Structured progression data needs a fully decoded save with a '
            'verified typed parse.',
      );
    }
    final reloadKey = inspection.sha1;
    return ListView(
      padding: const EdgeInsets.all(20),
      children: [
        _OverviewCard(overview: overview),
        const SizedBox(height: 16),
        _QuestsCard(
          notifier: notifier,
          editable: editable,
          reloadKey: reloadKey,
        ),
        const SizedBox(height: 16),
        _KnowledgeCard(
          notifier: notifier,
          editable: editable,
          reloadKey: reloadKey,
        ),
        const SizedBox(height: 16),
        _EventsCard(
          notifier: notifier,
          editable: editable,
          reloadKey: reloadKey,
        ),
      ],
    );
  }
}
```

`_MessagePane` lives in `editor_page.dart` as a private widget — either export a copy here (duplicate the ~20-line widget as a private `_MessagePane` in this file) or pass fallback widgets from the call site; duplicating the small pane locally is the simpler, self-contained option.

`_OverviewCard` (stateless): a `Card` with the flag icon header "Progression summary" and a `Wrap` of metric chips — quest total, one chip per `questStates` entry (label `'${entry.key}: ${entry.value}'`), knowledge characters/entries, memory characters/events. Mirror `_SummaryMetric` from `editor_page.dart` (duplicate the tiny widget locally).

`_QuestsCard` (stateful), following the Inventory card's pending pattern:

```dart
class _QuestsCard extends StatefulWidget {
  const _QuestsCard({
    required this.notifier,
    required this.editable,
    required this.reloadKey,
  });

  final EditorNotifier notifier;
  final bool editable;
  final Object reloadKey;

  @override
  State<_QuestsCard> createState() => _QuestsCardState();
}

class _QuestsCardState extends State<_QuestsCard> {
  final TextEditingController _search = TextEditingController();
  ProgressionQuestPage _page = const ProgressionQuestPage();
  final List<ProgressionQuest> _quests = [];
  final Map<String, QuestStateChange> _pending = {};
  bool _loading = false;
  int _reloadEpoch = 0;

  @override
  void initState() {
    super.initState();
    _reload();
  }

  @override
  void didUpdateWidget(covariant _QuestsCard oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (widget.reloadKey != oldWidget.reloadKey) {
      _pending.clear();
      _reload();
    }
  }

  @override
  void dispose() {
    _search.dispose();
    super.dispose();
  }

  Future<void> _reload({bool append = false}) async {
    final epoch = ++_reloadEpoch;
    setState(() => _loading = true);
    final page = await widget.notifier.loadProgressionQuests(
      query: _search.text.trim(),
      offset: append ? _quests.length : 0,
    );
    if (!mounted || epoch != _reloadEpoch) return;
    setState(() {
      _loading = false;
      _page = page;
      if (!append) _quests.clear();
      _quests.addAll(page.quests);
    });
  }

  void _pushPending() {
    if (_pending.isEmpty) {
      widget.notifier.clearPendingEdit('progression.quests');
    } else {
      widget.notifier.setPendingEdit(
        'progression.quests',
        PendingSaveEdit(
          edits: _pending.values.map((c) => c.toEditJson()).toList(),
        ),
      );
    }
  }

  void _setQuestState(ProgressionQuest quest, String? state) {
    setState(() {
      if (state == null || state == quest.currentState) {
        _pending.remove(quest.questClass);
      } else {
        _pending[quest.questClass] = QuestStateChange(
          statePath: quest.statePath,
          state: state,
        );
      }
    });
    _pushPending();
  }

  @override
  Widget build(BuildContext context) {
    return Card(
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Row(
              children: [
                const Icon(Icons.flag_outlined),
                const SizedBox(width: 8),
                Expanded(
                  child: Text(
                    'Quests',
                    style: Theme.of(context).textTheme.titleMedium,
                  ),
                ),
                if (widget.editable && _pending.isNotEmpty)
                  Tooltip(
                    message: 'Reset quest changes',
                    child: IconButton(
                      icon: const Icon(Icons.undo_outlined),
                      onPressed: () {
                        setState(_pending.clear);
                        widget.notifier.clearPendingEdit('progression.quests');
                      },
                    ),
                  ),
              ],
            ),
            const SizedBox(height: 12),
            TextField(
              controller: _search,
              decoration: const InputDecoration(
                labelText: 'Search quests',
                prefixIcon: Icon(Icons.search),
              ),
              onSubmitted: (_) => _reload(),
            ),
            if (_page.error != null) ...[
              const SizedBox(height: 12),
              Text(
                _page.error!,
                style: TextStyle(color: Theme.of(context).colorScheme.error),
              ),
            ],
            const SizedBox(height: 8),
            SizedBox(
              height: 360,
              child: _loading && _quests.isEmpty
                  ? const Center(child: CircularProgressIndicator())
                  : ListView.separated(
                      itemCount: _quests.length + (_page.hasMore ? 1 : 0),
                      separatorBuilder: (_, _) => const Divider(height: 1),
                      itemBuilder: (context, index) {
                        if (index >= _quests.length) {
                          return TextButton(
                            onPressed: _loading
                                ? null
                                : () => _reload(append: true),
                            child: Text(
                              'Load more (${_quests.length} of ${_page.total})',
                            ),
                          );
                        }
                        final quest = _quests[index];
                        final pendingState =
                            _pending[quest.questClass]?.state;
                        return ListTile(
                          dense: true,
                          leading: const Icon(Icons.flag_outlined),
                          title: SelectableText(
                            quest.name.isEmpty ? quest.id : quest.name,
                            maxLines: 1,
                          ),
                          subtitle: SelectableText(
                            quest.group,
                            maxLines: 1,
                          ),
                          trailing: widget.editable && quest.writable
                              ? DropdownButton<String>(
                                  value:
                                      pendingState ?? quest.currentState,
                                  underline: const SizedBox.shrink(),
                                  items: questStates
                                      .map(
                                        (s) => DropdownMenuItem(
                                          value: s,
                                          child: Text(shortStateLabel(s)),
                                        ),
                                      )
                                      .toList(),
                                  onChanged: (s) =>
                                      _setQuestState(quest, s),
                                )
                              : Text(
                                  shortStateLabel(
                                    quest.currentState ?? 'unknown',
                                  ),
                                ),
                        );
                      },
                    ),
            ),
          ],
        ),
      ),
    );
  }
}
```

Edge case: a quest whose `currentState` is not one of the five `questStates` values (or null) would crash `DropdownButton.value` — guard: if `pendingState ?? quest.currentState` is not contained in `questStates`, render the read-only `Text` branch instead.

`_KnowledgeCard` (stateful): same skeleton; state holds `KnowledgeCharactersPage _characters`, `String? _selectedCharacter`, `KnowledgeEntriesPage _entries`, `final Map<String, KnowledgeEntryEdit> _pending` keyed by `'$character $entry'`, plus an add-`TextEditingController`.
- Loads characters on init/reloadKey change; tapping a character loads its entries (`loadKnowledgeEntries`).
- Layout: character `ListView` (left or top, height-capped like the quests list) with `'$name ($entryCount)'` tiles; below/beside, the selected character's entries as `Chip`s in a `Wrap`, each with `onDeleted` (editable only) registering a `KnowledgeEntryEdit.remove(setPath: _entries.setPath, entry: e)`; a pending-removed entry renders with strikethrough style and its delete icon turns into an undo that drops the pending edit.
- Add row: `TextField` + button; validation: non-empty after trim, not already in `_entries.entries`, not already pending-added; registers `KnowledgeEntryEdit.add`. Pending adds render as extra chips with a distinct (e.g. tertiary) color and an undo delete.
- `_pushPending()` mirrors the quests card with key `'progression.knowledge'`.
- On reloadKey change: clear pending + reload characters and (if still selected) entries.

`_EventsCard` (stateful): state holds `MemoryCharactersPage _characters`, `String? _selectedCharacter`, `MemoryEventsPage _events`, `_loading`.
- Character tiles `'$id ($eventCount)'`; selecting loads `loadMemoryEvents`.
- Event tiles: title = tags joined with `', '` (fallback `'(no tags)'`), subtitle = `'t=${event.timeSeconds?.toStringAsFixed(0) ?? '?'}s  ${event.affected ?? ''}'`; trailing (editable only) two `IconButton`s: delete (`Icons.delete_outline`) and duplicate (`Icons.copy_outlined`).
- Both actions run an `AlertDialog` confirm ("Remove this memory event? A backup is written first." / duplicate equivalent), then `await widget.notifier.applyMemoryEventEdit(MemoryEventEdit.remove(arrayPath: _events.arrayPath, index: event.index))` — on success the notifier refreshes the inspection, which changes `reloadKey` and reloads this card. These are immediate single-edit writes (index-addressed edits must not batch).
- Pagination: same "Load more" pattern as quests.

- [ ] **Step 3: Wire into `editor_page.dart`**

- Add `import 'package:goresave/features/editor/ui/progression_panel.dart';` (match the file's import style for `hero_stats_card.dart`).
- Replace the `_ProgressionPanel` widget body: keep the class as a thin adapter or call `ProgressionPanel` directly from the tab builder — match how the tab list at ~line 475 builds panels. Pass `editable:` using the same expression the Inventory tab uses for its `editable`/`canCompress` gating combined with `inspection.privateTypedVerified`.
- Delete `_PrivateProgressionSummaryCard`, its state class, and `_StringList` (after confirming `_StringList` has no other references).

- [ ] **Step 4: Static check + tests + format**

Run: `flutter analyze` (expect clean), `flutter test`, and `dart format lib/features/editor/ui/progression_panel.dart` (match repo formatting).
Expected: analyzer clean, all tests green.

- [ ] **Step 5: Commit**

```bash
git add apps/goresave/lib/features/editor/ui/progression_panel.dart apps/goresave/lib/features/editor/ui/editor_page.dart
git commit -m "feat(ui): structured editable progression tab"
```

---

### Task 11: End-to-end verification on a real save

**Files:** none modified (verification only)

- [ ] **Step 1: Full test suites**

Run: `cargo test -p goresave_core` and (from `apps/goresave`) `flutter analyze && flutter test`
Expected: all green.

- [ ] **Step 2: Real-save query smoke test**

Real saves live in `work/roundtrip_gsav/` (G1R-001…006). The codec host needs the configured game EXE; the app settings (used by the running app) already carry working paths. Two options, prefer (a):

(a) Launch the app (`flutter run -d windows` from `apps/goresave`, or the existing build in `apps/goresave/build/windows/.../goresave.exe`), point the save dir at `C:\sbx\goresave\work\roundtrip_gsav`, select G1R-001, open the Progression tab, and verify:
- Overview shows plausible counts (hundreds of quests, dozens of knowledge NPCs, large event counts).
- Quests list loads, search filters, state dropdown changes register as pending edits, Save writes with backup, and the new state survives a reload.
- Knowledge: select `OC_STT_Diego`, remove + re-add an entry, save, verify.
- Events: select the hero character, duplicate an event, confirm, verify the count increased; then remove the duplicate.
- Work on a COPY of the save directory if the saves are precious: `Copy-Item -Recurse work\roundtrip_gsav work\roundtrip_gsav_testrun` and point the app at the copy.

(b) Headless: a scratch Rust test invoking `execute_json` with `query_progression` + a `binaryHost` config pointing at the local codec host/exe paths — only if the app route is unavailable.

- [ ] **Step 3: Strict roundtrip assurance**

After the edits in Step 2, re-inspect the edited save in the app and confirm `typedParse.status == ok` (visible in the Overview/All-data surfaces) — this is the byte-exact proof that size chains stayed consistent on a 76 MB real payload.

- [ ] **Step 4: Update the spec status line**

Edit `docs/superpowers/specs/2026-06-11-progression-tab-design.md`: change `Status:` to `implemented (branch progression-tab-v2)`.

```bash
git add docs/superpowers/specs/2026-06-11-progression-tab-design.md
git commit -m "docs: mark progression tab v2 spec implemented"
```
