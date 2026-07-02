# Characters & World Tab Restructure — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Restructure the save editor from aspect-first to entity-first: a single **Charaktere** master list (Player pinned + all NPCs) whose detail area has four sub-tabs (Attribute/Inventar/Wissen/Ereignisse), plus a **Welt** tab for Quests + Fraktionen.

**Architecture:** A new Rust read command `private.characters.list` does the GlobalId↔UniqueName identity join in the core and emits one authoritative character list with per-aspect availability flags (`hasInventory/hasKnowledge/hasEvents`) plus a small tail of knowledge-only "orphan" rows. The Flutter UI repurposes the existing `ActorSelector` as the shared master list and reuses the four existing detail panels, deleting the three now-redundant per-tab character lists.

**Tech Stack:** Rust (`crates/gore-save`, serde_json, `#[test]` integration tests), Flutter/Dart (`apps/save-editor`, Riverpod, `flutter_test`).

**Spec:** `docs/superpowers/specs/2026-07-01-characters-world-restructure-design.md`

---

## Verified facts this plan relies on (from the spec's measurement)

- Knowledge `UniqueName` == actor GlobalId prefix before the first `-` (case-insensitive). Example: knowledge `NC_ORG_Lares_801` ↔ actor `NC_ORG_Lares_801-WP_…`.
- Events (`LongTermMemoryByGlobalId`) key by exact GlobalId; 0 orphans observed.
- Knowledge orphans are 0–1 (`ST_VLK_Mud_Sleeper` on the late save).
- Player knowledge is keyed `Hero`.

## Existing symbols this plan builds on (do not re-implement)

- `crates/gore-save/src/npc.rs`: `pub struct NpcSummary { id, is_dead, hp, max_hp }` (serde `camelCase`); `pub fn list_npcs(&RootObject) -> Result<Vec<NpcSummary>, CoreError>`; private `find_character_map(root, struct_type) -> Option<&[(PropertyValue, PropertyValue)]>`; private `map_key_to_string(&PropertyValue) -> Option<String>`; consts `ATTRIBUTES_TYPE`, `INVENTORY_TYPE`, `LONG_TERM_MEMORY_MAP = "LongTermMemoryByGlobalId"`.
- `crates/gore-save/src/lib.rs`: `pub fn execute_json(&str) -> String`; `fn decode_private_root(path, backend) -> Result<RootObject, CoreError>`; `fn list_npcs_command(...)`; the `execute_json_inner` command `match`; `properties::find_property_by_name(root, name) -> Option<(Vec<String>, &Property)>`.
- `apps/save-editor/lib/features/editor/domain/actor.dart`: `class Actor` (`kind`, `id`, `name`, `isDead`; `==` by `(kind,id)`).
- `apps/save-editor/lib/features/editor/domain/npc_actors_page.dart`: `NpcActor`, `NpcActorsPage`.
- `apps/save-editor/lib/features/editor/ui/actor_selector.dart`: `ActorSelector` (props `selected`, `onSelect`, `loadNpcs`, `reloadKey`, `locCatalog`, `lang`), `localizedNpcName(...)`.
- `apps/save-editor/lib/features/editor/domain/editor_notifier.dart`: `loadAllNpcActors(...)`, `loadNpcActors(...)`, shared `state.selectedActor` / `notifier.selectActor(Actor)`.
- `apps/save-editor/lib/features/editor/ui/editor_page.dart`: `_EditorWorkspace` (the `DefaultTabController(length: 7)` + `TabBar`/`TabBarView`), `_AttributePanel`, `_InventoryPanel`.
- `apps/save-editor/lib/features/editor/ui/progression_panel.dart`: `ProgressionPanel`, `_QuestsDetail`, `_KnowledgeDetail`, `_EventsDetail`, `_FactionsDetail`, `_SidebarTile`.

---

# PHASE 1 — Core: `private.characters.list`

Produces a working, tested core command. No UI change. Run all Rust commands from the repo root.

## Task 1: `CharacterSummary` model + `list_characters` join (npc.rs)

**Files:**
- Modify: `crates/gore-save/src/npc.rs` (add `CharacterSummary`, `char_key`, `list_characters`)
- Test: `crates/gore-save/src/npc.rs` (unit tests in the existing `#[cfg(test)] mod tests`)

- [ ] **Step 1: Write the failing test**

Add to the `#[cfg(test)] mod tests` block at the bottom of `npc.rs`:

```rust
#[test]
fn char_key_strips_after_first_dash_and_lowercases() {
    assert_eq!(char_key("NC_ORG_Lares_801-WP_OC_SPAWN"), "nc_org_lares_801");
    assert_eq!(char_key("Hero"), "hero");
    assert_eq!(char_key("A-B-C"), "a");
}
```

- [ ] **Step 2: Run it to verify it fails**

Run: `cargo test -p gore-save char_key_strips_after_first_dash -- --nocapture`
Expected: FAIL — `cannot find function char_key`.

- [ ] **Step 3: Implement `char_key` + `CharacterSummary` + `list_characters`**

Add near `NpcSummary` in `npc.rs`:

```rust
/// A character row for the unified `private.characters.list`. Extends the NPC
/// summary with the knowledge UniqueName and per-aspect availability flags, so
/// the frontend needs one call to build the master list. `global_id` is `None`
/// for knowledge-only "orphan" rows that have no spawned actor.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CharacterSummary {
    pub global_id: Option<String>,
    /// GlobalId prefix (before the first `-`), the key the knowledge map uses.
    pub unique_name: String,
    pub is_dead: bool,
    pub has_inventory: bool,
    pub has_knowledge: bool,
    pub has_events: bool,
}

/// The knowledge/UniqueName key derived from a GlobalId: the substring before
/// the first `-`, lower-cased. Proven (spec measurement) to equal the knowledge
/// map key for every non-orphan character.
pub(crate) fn char_key(global_id: &str) -> String {
    global_id
        .split('-')
        .next()
        .unwrap_or(global_id)
        .to_ascii_lowercase()
}
```

Add `use std::collections::HashSet;` to the top of `npc.rs` if not already present, then add:

```rust
/// Collect the stringified keys of a named MapProperty found anywhere in the
/// tree (used for the knowledge + long-term-memory maps), lower-cased.
fn map_keys_lower(root: &RootObject, name: &str) -> HashSet<String> {
    match properties::find_property_by_name(root, name) {
        Some((_, prop)) => match &prop.value {
            PropertyValue::Map { entries, .. } => entries
                .iter()
                .filter_map(|(k, _)| map_key_to_string(k))
                .map(|s| s.to_ascii_lowercase())
                .collect(),
            _ => HashSet::new(),
        },
        None => HashSet::new(),
    }
}

/// Collect the stringified GlobalId keys of a character-state map (by value
/// struct type), lower-cased.
fn character_map_keys_lower(root: &RootObject, struct_type: &str) -> HashSet<String> {
    match find_character_map(root, struct_type) {
        Some(entries) => entries
            .iter()
            .filter_map(|(k, _)| map_key_to_string(k))
            .map(|s| s.to_ascii_lowercase())
            .collect(),
        None => HashSet::new(),
    }
}

/// Build the unified character list: every spawned actor (from [`list_npcs`])
/// annotated with availability flags, followed by knowledge-only orphan rows
/// (a knowledge UniqueName with no matching actor charKey). The join is the
/// proven prefix rule ([`char_key`]).
pub fn list_characters(root: &RootObject) -> Result<Vec<CharacterSummary>, CoreError> {
    let knowledge = map_keys_lower(root, "CharacterKnowledgeByUniqueName");
    let events = map_keys_lower(root, LONG_TERM_MEMORY_MAP);
    let inventory = character_map_keys_lower(root, INVENTORY_TYPE);

    let npcs = list_npcs(root)?;
    let mut actor_keys: HashSet<String> = HashSet::new();
    let mut out: Vec<CharacterSummary> = Vec::with_capacity(npcs.len());
    for npc in &npcs {
        let key = char_key(&npc.id);
        actor_keys.insert(key.clone());
        let id_lower = npc.id.to_ascii_lowercase();
        out.push(CharacterSummary {
            global_id: Some(npc.id.clone()),
            unique_name: key.clone(),
            is_dead: npc.is_dead,
            has_inventory: inventory.contains(&id_lower),
            has_knowledge: knowledge.contains(&key),
            has_events: events.contains(&id_lower),
        });
    }
    // Knowledge-only orphans: a UniqueName with no actor charKey. Typically 0–1.
    let mut orphans: Vec<String> = knowledge
        .iter()
        .filter(|k| !actor_keys.contains(*k))
        .cloned()
        .collect();
    orphans.sort();
    for key in orphans {
        out.push(CharacterSummary {
            global_id: None,
            unique_name: key,
            is_dead: false,
            has_inventory: false,
            has_knowledge: true,
            has_events: false,
        });
    }
    Ok(out)
}
```

- [ ] **Step 4: Run the test to verify it passes**

Run: `cargo test -p gore-save char_key_strips_after_first_dash -- --nocapture`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/gore-save/src/npc.rs
git commit -m "feat(gore-save): CharacterSummary + list_characters identity join"
```

## Task 2: Wire `private.characters.list` into the command dispatch (lib.rs)

**Files:**
- Modify: `crates/gore-save/src/lib.rs` (new `characters_list_command` + `match` arm)

- [ ] **Step 1: Write the failing integration test**

Create `crates/gore-save/tests/characters_list.rs`. It runs only when `GORE_SAVE` points at a real save, so normal `cargo test` skips it. Add `serde_json = "1"` to `[dev-dependencies]` in `crates/gore-save/Cargo.toml` first.

```rust
//! Requires a real save via GORE_SAVE; skips otherwise.
//!   GORE_SAVE='C:\Users\Daniel\AppData\Local\G1R\Saved\SaveGames\G1R-021.sav' \
//!     cargo test -p gore-save --test characters_list -- --nocapture
use serde_json::{json, Value};

fn data(path: &str, command: &str, section: Option<&str>, offset: usize) -> Value {
    let mut payload = json!({ "path": path, "offset": offset, "limit": 1000 });
    if let Some(s) = section { payload["section"] = json!(s); }
    let req = json!({ "command": command, "payload": payload }).to_string();
    let resp: Value = serde_json::from_str(&gore_save::execute_json(&req)).unwrap();
    assert_eq!(resp["ok"], json!(true), "{command} failed: {resp}");
    resp["data"].clone()
}

#[test]
fn characters_list_matches_npc_and_knowledge_sets() {
    let Ok(path) = std::env::var("GORE_SAVE") else {
        eprintln!("GORE_SAVE not set; skipping"); return;
    };
    let chars = data(&path, "private.characters.list", None, 0);
    let rows = chars["characters"].as_array().unwrap();
    // Every non-orphan row has a globalId; orphans have null + hasKnowledge.
    for r in rows {
        if r["globalId"].is_null() {
            assert_eq!(r["hasKnowledge"], json!(true), "orphan must have knowledge");
        }
    }
    // Actor count parity: non-orphan rows == npc.list total.
    let npc_total = data(&path, "private.npc.list", None, 0)["total"].as_u64().unwrap();
    let non_orphan = rows.iter().filter(|r| !r["globalId"].is_null()).count() as u64;
    assert_eq!(non_orphan, npc_total, "non-orphan rows must equal actor count");
    eprintln!("characters: {}, actors: {npc_total}", rows.len());
}
```

- [ ] **Step 2: Run it to verify it fails**

Run: `cargo test -p gore-save --test characters_list -- --nocapture` (with `GORE_SAVE` unset)
Expected: it compiles but the command is unknown; set `GORE_SAVE` and it FAILS with `private.characters.list failed: ... UNKNOWN_COMMAND`. (Unset run prints "skipping" and passes — that only proves the harness compiles.)

- [ ] **Step 3: Add the command + dispatch arm**

In `lib.rs`, add next to `list_npcs_command`:

```rust
/// `private.characters.list`: the unified character index (actors + knowledge
/// orphans) with per-aspect availability flags. Decodes once via the shared
/// prelude, then joins in the core (see `npc::list_characters`). Paginated +
/// id/uniqueName filterable, mirroring `private.npc.list`.
fn characters_list_command(
    path: &Path,
    payload: &Value,
    backend: Option<&dyn codec_backend::CodecBackend>,
) -> Result<Value, CoreError> {
    let backend = backend.ok_or_else(|| {
        CoreError::Codec("listing characters requires a working codec backend".to_string())
    })?;
    let query = payload
        .get("query")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_ascii_lowercase();
    let offset = payload
        .get("offset")
        .and_then(Value::as_u64)
        .map(|v| v as usize)
        .unwrap_or(0);
    let limit = payload
        .get("limit")
        .and_then(Value::as_u64)
        .map(|v| v as usize)
        .unwrap_or(100)
        .clamp(1, 1000);

    let root = decode_private_root(path, backend)?;
    let all = npc::list_characters(&root)?;
    let filtered: Vec<&npc::CharacterSummary> = all
        .iter()
        .filter(|c| {
            if query.is_empty() {
                return true;
            }
            let id_hit = c
                .global_id
                .as_deref()
                .map(|g| g.to_ascii_lowercase().contains(&query))
                .unwrap_or(false);
            id_hit || c.unique_name.contains(&query)
        })
        .collect();
    let total = filtered.len();
    let page: Vec<&npc::CharacterSummary> =
        filtered.into_iter().skip(offset).take(limit).collect();
    Ok(json!({
        "characters": page,
        "total": total,
        "offset": offset,
        "limit": limit,
    }))
}
```

In `execute_json_inner`'s `match command { ... }`, add an arm beside `"private.npc.list"`:

```rust
        "private.characters.list" => {
            let path = required_path(&payload)?;
            let ooz_backend = codec_backend::OozKrakenBackend::default();
            let codec_backend = Some(&ooz_backend as &dyn codec_backend::CodecBackend);
            characters_list_command(&path, &payload, codec_backend)
        }
```

- [ ] **Step 4: Run the integration test against a real save**

Run: `GORE_SAVE='C:\Users\Daniel\AppData\Local\G1R\Saved\SaveGames\G1R-021.sav' cargo test -p gore-save --test characters_list -- --nocapture`
Expected: PASS; prints e.g. `characters: 1639, actors: 1638` (1 orphan on 021).

Also run the mid save to confirm 0 orphans / parity:
Run: `GORE_SAVE='C:\Users\Daniel\AppData\Local\G1R\Saved\SaveGames\G1R-011.sav' cargo test -p gore-save --test characters_list -- --nocapture`
Expected: PASS; `characters: 1496, actors: 1496`.

- [ ] **Step 5: Commit**

```bash
git add crates/gore-save/src/lib.rs crates/gore-save/Cargo.toml crates/gore-save/tests/characters_list.rs
git commit -m "feat(gore-save): private.characters.list command + join test"
```

## Task 3: Availability-flag correctness test (synthetic root)

**Files:**
- Modify: `crates/gore-save/src/npc.rs` (add a unit test using the existing test fixtures)

- [ ] **Step 1: Write the failing test**

Reuse the crate's existing property-tree test helpers (search `npc.rs`/`lib.rs` tests for a builder that constructs a `RootObject` with `CharacterKnowledgeByUniqueName` — e.g. the `knowledge` roundtrip tests near `lib.rs:11471`). Add to `npc.rs` tests a case that builds a root with one actor whose GlobalId is `NC_ORG_Lares_801-WP_X`, a knowledge entry keyed `NC_ORG_Lares_801`, and an orphan knowledge entry keyed `ST_VLK_Mud_Sleeper`:

```rust
#[test]
fn list_characters_flags_and_orphans() {
    let root = build_test_root_with_knowledge(
        &["NC_ORG_Lares_801-WP_X"],           // actors (GlobalIds)
        &["NC_ORG_Lares_801", "ST_VLK_Mud_Sleeper"], // knowledge UniqueNames
    );
    let chars = list_characters(&root).unwrap();
    let lares = chars.iter().find(|c| c.unique_name == "nc_org_lares_801").unwrap();
    assert_eq!(lares.global_id.as_deref(), Some("NC_ORG_Lares_801-WP_X"));
    assert!(lares.has_knowledge);
    let orphan = chars.iter().find(|c| c.unique_name == "st_vlk_mud_sleeper").unwrap();
    assert!(orphan.global_id.is_none());
    assert!(orphan.has_knowledge);
}
```

If no reusable builder exists, write `build_test_root_with_knowledge` in the test module using the same `Property`/`PropertyValue::Map` construction the existing knowledge tests use (copy that fixture's map-entry helper). Keep it in `#[cfg(test)]`.

- [ ] **Step 2: Run it to verify it fails**

Run: `cargo test -p gore-save list_characters_flags_and_orphans -- --nocapture`
Expected: FAIL (builder/assertions unmet) until the fixture is wired.

- [ ] **Step 3: Implement the fixture builder** (mirror the existing knowledge-map test fixture; no production code changes — `list_characters` already exists).

- [ ] **Step 4: Run it to verify it passes**

Run: `cargo test -p gore-save list_characters_flags_and_orphans -- --nocapture`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/gore-save/src/npc.rs
git commit -m "test(gore-save): list_characters availability flags + orphan row"
```

## Task 4: Rebuild the FFI/C ABI + Dart handle (if regenerated bindings exist)

**Files:**
- Verify: `crates/gore-ffi/` and `apps/save-editor/lib/features/editor/domain/core_service.dart`

- [ ] **Step 1:** Confirm the app reaches the core purely through the JSON string ABI (`execute_json`) — grep `core_service.dart` for how commands are sent. If it sends `{command, payload}` JSON to a single FFI entry, **no ABI change is needed** (the new command is reachable immediately).

Run: `grep -n "command" apps/save-editor/lib/features/editor/domain/core_service.dart | head`
Expected: a single generic `execute`/`call` that forwards `{command, payload}` — confirming no regeneration needed.

- [ ] **Step 2: Commit** (only if any binding file changed; otherwise skip).

---

# PHASE 2 — Charaktere tab (frontend)

Reuses the four detail panels; deletes the per-tab character lists. Run Flutter commands from `apps/save-editor`.

## Task 5: `uniqueName` on `Actor`

**Files:**
- Modify: `apps/save-editor/lib/features/editor/domain/actor.dart`
- Test: `apps/save-editor/test/editor/actor_test.dart` (create)

- [ ] **Step 1: Write the failing test**

```dart
import 'package:flutter_test/flutter_test.dart';
import 'package:goresave/features/editor/domain/actor.dart';

void main() {
  test('uniqueName is carried but excluded from equality', () {
    const a = Actor.npc(id: 'NC_ORG_Lares_801-WP_X', name: 'Lares', uniqueName: 'nc_org_lares_801');
    const b = Actor.npc(id: 'NC_ORG_Lares_801-WP_X', name: 'Lares', uniqueName: 'different');
    expect(a, equals(b)); // identity is (kind, id) only
    expect(a.uniqueName, 'nc_org_lares_801');
    expect(const Actor.player().uniqueName, 'Hero');
  });
}
```

- [ ] **Step 2: Run it to verify it fails**

Run: `flutter test test/editor/actor_test.dart`
Expected: FAIL — `Actor.npc` has no `uniqueName` param.

- [ ] **Step 3: Add the field**

In `actor.dart`: add `final String uniqueName;` and thread it through both constructors. Player: `uniqueName = 'Hero'` (its knowledge key). NPC: `required this.uniqueName`. Leave `==`/`hashCode` on `(kind, id)` unchanged (add a comment that `uniqueName`, like `name`/`isDead`, is a carried label, not identity).

- [ ] **Step 4: Run it to verify it passes**

Run: `flutter test test/editor/actor_test.dart`
Expected: PASS. Then `flutter analyze` — fix any call sites of `Actor.npc(...)` that now need `uniqueName` (the NPC row builder in `actor_selector.dart` — pass `char_key`-style prefix; Task 7 replaces this anyway, so a temporary `uniqueName: npc.id.split('-').first` is fine).

- [ ] **Step 5: Commit**

```bash
git add apps/save-editor/lib/features/editor/domain/actor.dart apps/save-editor/test/editor/actor_test.dart
git commit -m "feat(save-editor): carry uniqueName on Actor for knowledge keying"
```

## Task 6: `CharacterRow` model + page + `loadAllCharacters`

**Files:**
- Create: `apps/save-editor/lib/features/editor/domain/character_index.dart`
- Modify: `apps/save-editor/lib/features/editor/domain/editor_notifier.dart`
- Test: `apps/save-editor/test/editor/character_index_test.dart` (create)

- [ ] **Step 1: Write the failing test**

```dart
import 'package:flutter_test/flutter_test.dart';
import 'package:goresave/features/editor/domain/character_index.dart';

void main() {
  test('CharacterRow parses flags and null globalId', () {
    final orphan = CharacterRow.fromJson({
      'globalId': null, 'uniqueName': 'st_vlk_mud_sleeper',
      'isDead': false, 'hasInventory': false, 'hasKnowledge': true, 'hasEvents': false,
    });
    expect(orphan.globalId, isNull);
    expect(orphan.isOrphan, isTrue);
    expect(orphan.hasKnowledge, isTrue);

    final actor = CharacterRow.fromJson({
      'globalId': 'NC_ORG_Lares_801-WP_X', 'uniqueName': 'nc_org_lares_801',
      'isDead': false, 'hasInventory': true, 'hasKnowledge': true, 'hasEvents': true,
    });
    expect(actor.isOrphan, isFalse);
    expect(actor.hasEvents, isTrue);
  });
}
```

- [ ] **Step 2: Run it to verify it fails**

Run: `flutter test test/editor/character_index_test.dart`
Expected: FAIL — `character_index.dart` missing.

- [ ] **Step 3: Create the model + page**

`character_index.dart`:

```dart
/// One row of `private.characters.list`: an actor (or a knowledge-only orphan,
/// `globalId == null`) with per-aspect availability flags. Backs the Charaktere
/// master list. Mirrors the shape of `NpcActorsPage` for pagination.
class CharacterRow {
  const CharacterRow({
    required this.globalId,
    required this.uniqueName,
    required this.isDead,
    required this.hasInventory,
    required this.hasKnowledge,
    required this.hasEvents,
  });

  factory CharacterRow.fromJson(Map<String, Object?> json) {
    return CharacterRow(
      globalId: json['globalId'] as String?,
      uniqueName: json['uniqueName'] as String? ?? '',
      isDead: json['isDead'] == true,
      hasInventory: json['hasInventory'] == true,
      hasKnowledge: json['hasKnowledge'] == true,
      hasEvents: json['hasEvents'] == true,
    );
  }

  final String? globalId;
  final String uniqueName;
  final bool isDead;
  final bool hasInventory;
  final bool hasKnowledge;
  final bool hasEvents;

  bool get isOrphan => globalId == null;
}

class CharacterIndexPage {
  const CharacterIndexPage({
    this.characters = const [],
    this.total = 0,
    this.offset = 0,
    this.limit = 100,
    this.error,
  });

  factory CharacterIndexPage.fromJson(Map<String, Object?> json) {
    return CharacterIndexPage(
      characters: (json['characters'] as List?)
              ?.whereType<Map>()
              .map((e) => CharacterRow.fromJson(e.cast<String, Object?>()))
              .toList(growable: false) ??
          const [],
      total: (json['total'] as num?)?.toInt() ?? 0,
      offset: (json['offset'] as num?)?.toInt() ?? 0,
      limit: (json['limit'] as num?)?.toInt() ?? 100,
    );
  }

  final List<CharacterRow> characters;
  final int total;
  final int offset;
  final int limit;
  final String? error;
}
```

- [ ] **Step 4: Add `loadAllCharacters` to the notifier**

Open `editor_notifier.dart`, find `loadAllNpcActors` (~line 1410) and add a sibling that pages `private.characters.list` the same way (the core clamps `limit` to 1000). Use the same command-send helper the other `load*` methods use (grep for how `loadNpcActors` calls the core — reuse that exact call site pattern):

```dart
/// Fetch the full unified character index for the selected save, paging past the
/// core's 1000-row clamp. Mirrors [loadAllNpcActors] but targets
/// `private.characters.list` so rows carry availability flags + orphan tail.
Future<CharacterIndexPage> loadAllCharacters() async {
  final path = state.selectedPath;
  if (path == null) {
    return const CharacterIndexPage(error: 'no save selected');
  }
  final all = <CharacterRow>[];
  var offset = 0;
  while (true) {
    final resp = await _core.execute('private.characters.list', {
      'path': path,
      'offset': offset,
      'limit': 1000,
    });
    if (resp.error != null) {
      return CharacterIndexPage(error: resp.error);
    }
    final page = CharacterIndexPage.fromJson(resp.data ?? const {});
    all.addAll(page.characters);
    offset += page.characters.length;
    if (page.characters.isEmpty || offset >= page.total) break;
  }
  return CharacterIndexPage(characters: all, total: all.length);
}
```

Adjust `_core.execute(...)`/`resp.error`/`resp.data` to match the real `core_service` API discovered in Task 4 (use the identical call shape as `loadNpcActors`).

- [ ] **Step 5: Run tests + analyze, then commit**

Run: `flutter test test/editor/character_index_test.dart && flutter analyze`
Expected: PASS, no analyzer errors.

```bash
git add apps/save-editor/lib/features/editor/domain/character_index.dart apps/save-editor/lib/features/editor/domain/editor_notifier.dart apps/save-editor/test/editor/character_index_test.dart
git commit -m "feat(save-editor): CharacterRow model + loadAllCharacters"
```

## Task 7: `CharacterMasterList` widget (repurpose ActorSelector with badges + orphans)

**Files:**
- Create: `apps/save-editor/lib/features/editor/ui/character_master_list.dart`
- Test: `apps/save-editor/test/editor/character_master_list_test.dart` (create)

Rationale: `ActorSelector` is tightly coupled to `NpcActor`. Rather than overload it, copy its proven client-side search/pagination structure into a new widget that consumes `CharacterRow` (so the old Attribute/Inventory removal in Task 9 can delete `ActorSelector` cleanly). The Player row stays pinned; NPC rows gain badges; an "Weitere" group renders orphans only when present.

- [ ] **Step 1: Write the failing widget test**

```dart
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:goresave/features/editor/domain/actor.dart';
import 'package:goresave/features/editor/domain/character_index.dart';
import 'package:goresave/features/editor/ui/character_master_list.dart';
import 'package:goresave/l10n/app_localizations.dart';
import 'package:flutter_localizations/flutter_localizations.dart';

void main() {
  testWidgets('renders Player, an NPC with knowledge badge, and Weitere group',
      (tester) async {
    final page = CharacterIndexPage(characters: const [
      CharacterRow(globalId: 'NC_ORG_Lares_801-WP_X', uniqueName: 'nc_org_lares_801',
          isDead: false, hasInventory: true, hasKnowledge: true, hasEvents: true),
      CharacterRow(globalId: null, uniqueName: 'st_vlk_mud_sleeper',
          isDead: false, hasInventory: false, hasKnowledge: true, hasEvents: false),
    ], total: 2);
    Actor? picked;
    await tester.pumpWidget(MaterialApp(
      localizationsDelegates: AppLocalizations.localizationsDelegates,
      supportedLocales: AppLocalizations.supportedLocales,
      home: Scaffold(body: CharacterMasterList(
        selected: const Actor.player(),
        onSelect: (a) => picked = a,
        load: () async => page,
        reloadKey: 'k1',
        locCatalog: const {},
        lang: GameLang.de,
      )),
    ));
    await tester.pumpAndSettle();
    expect(find.text('Lares'), findsOneWidget); // resolved name (or id fallback)
    await tester.tap(find.text('Lares'));
    expect(picked?.id, 'NC_ORG_Lares_801-WP_X');
    expect(picked?.uniqueName, 'nc_org_lares_801');
  });
}
```

(Import `GameLang` from `package:goresave/loc/game_lang.dart`; adjust the resolved-name expectation to match `localizedNpcName` fallback if `Lares` isn't produced from the id.)

- [ ] **Step 2: Run it to verify it fails**

Run: `flutter test test/editor/character_master_list_test.dart`
Expected: FAIL — `character_master_list.dart` missing.

- [ ] **Step 3: Implement `CharacterMasterList`**

Copy `actor_selector.dart` into `character_master_list.dart` and adapt:
- Props: `selected`, `onSelect`, `load` (`Future<CharacterIndexPage> Function()` — pass `notifier.loadAllCharacters`), `reloadKey`, `locCatalog`, `lang`.
- Cache `List<CharacterRow>` instead of `_SearchableNpc`; precompute the search string as `'${row.globalId ?? row.uniqueName}\n$displayName'.toLowerCase()` where `displayName = localizedNpcName(locCatalog, lang, row.globalId ?? row.uniqueName)`.
- Player row pinned on top (unchanged), `onSelect(const Actor.player())`.
- NPC row `onTap`: `onSelect(Actor.npc(id: row.globalId!, name: displayName, isDead: row.isDead, uniqueName: row.uniqueName))`.
- Trailing badges on each NPC row: a small `Wrap` of icons — `Icons.menu_book_outlined` when `hasKnowledge`, `Icons.history` when `hasEvents` (tooltips from l10n). No inventory badge.
- Split rows into actors (`globalId != null`) and orphans (`isOrphan`). Render orphans under a non-collapsible `Weitere` section header **only when the orphan list is non-empty**; orphan `onTap` builds `Actor.npc(id: ...)` is impossible (null id) — instead introduce `Actor.orphan(uniqueName)` OR reuse `Actor.npc` with `id: ''`. Decision: add a third `Actor.orphan({required uniqueName})` constructor (kind `npc`, `id = null`-like sentinel) — simplest is `Actor.npc(id: 'orphan:$uniqueName', name: uniqueName, uniqueName: uniqueName)` so equality still works and the detail sub-tabs can detect the `orphan:` prefix. Use the sentinel approach to avoid touching `ActorKind`.

- [ ] **Step 4: Run it to verify it passes**

Run: `flutter test test/editor/character_master_list_test.dart && flutter analyze`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add apps/save-editor/lib/features/editor/ui/character_master_list.dart apps/save-editor/test/editor/character_master_list_test.dart
git commit -m "feat(save-editor): CharacterMasterList with aspect badges + orphan group"
```

## Task 8: Trim `_KnowledgeDetail` + `_EventsDetail` to detail-only (keyed by shared selection)

**Files:**
- Modify: `apps/save-editor/lib/features/editor/ui/progression_panel.dart`

These two widgets currently own a left character pane (search + pagination + list). The master list now owns selection, so the panes are deleted and the details key off the passed-in character.

- [ ] **Step 1:** Change `_KnowledgeDetail` to take `required String? uniqueName` (the selected character's knowledge key) instead of managing `_selectedCharacter`. Delete: `_characterSearch`, `_characters`, `_loadCharacters`, `_applyCharSearch`, the `_addNpc`/`showAddNpcDialog` flow, `_npcCatalog`, and the entire left `SizedBox(width: 280, …)` character column + `VerticalDivider`. Keep the right pane (entries list, add-entry field, pagination). Drive `_selectCharacter(uniqueName)` from `initState`/`didUpdateWidget` when `widget.uniqueName` changes. When `uniqueName == null` show `selectNpcFromList` empty state; when the character has no knowledge entry yet, show the existing add field (its first add calls `applyAddKnowledgeCharacter(uniqueName)` then the entry-add — the notifier already creates the map entry).

- [ ] **Step 2:** Change `_EventsDetail` the same way: take `required String? globalId`; delete its character pane + search/pagination-of-characters; key `_selectCharacter(globalId)` off `widget.globalId`.

- [ ] **Step 3: Run the existing progression tests + analyze**

Run: `flutter test test/ && flutter analyze`
Expected: compile clean. Update/rename any progression widget tests that constructed `_KnowledgeDetail`/`_EventsDetail` with the old API (they are private; if tested via `ProgressionPanel`, Task 12 covers the Welt-side).

- [ ] **Step 4: Commit**

```bash
git add apps/save-editor/lib/features/editor/ui/progression_panel.dart
git commit -m "refactor(save-editor): knowledge/events details keyed by shared selection"
```

## Task 9: Build the Charaktere tab shell (master list + 4 sub-tabs)

**Files:**
- Create: `apps/save-editor/lib/features/editor/ui/characters_tab.dart`
- Modify: `apps/save-editor/lib/features/editor/ui/editor_page.dart`

- [ ] **Step 1:** Create `CharactersTab` (a `ConsumerWidget`): a `Row` with `CharacterMasterList` on the left (width 365, `load: notifier.loadAllCharacters`, `selected: state.selectedActor`, `onSelect: notifier.selectActor`) and, on the right, a `DefaultTabController(length: 4)` with a `TabBar` (`Attribute · Inventar · Wissen · Ereignisse`) over a `TabBarView`:
  - **Attribute** = the existing Attribute detail. Extract the detail body of `_AttributePanel` (the player `HeroStatsCard` / NPC `NpcAttributesPanel` branch, everything to the RIGHT of its `ActorSelector`) into a reusable `AttributeDetail({required Actor actor, ...})` and use it here.
  - **Inventar** = same extraction for `_InventoryPanel` → `InventoryDetail({required Actor actor, ...})`.
  - **Wissen** = `_KnowledgeDetail(uniqueName: state.selectedActor.isPlayer ? 'Hero' : state.selectedActor.uniqueName, ...)`. Make `_KnowledgeDetail` public (`KnowledgeDetail`) or expose a thin wrapper.
  - **Ereignisse** = `_EventsDetail(globalId: state.selectedActor.isPlayer ? <player-global-id-or-null> : state.selectedActor.id, ...)` exposed as `EventsDetail`.
  Each sub-tab keeps `_KeepAliveTab` semantics so pending edits survive tab switches.

- [ ] **Step 2:** In `editor_page.dart` `_EditorWorkspace`, replace the three tabs `Attribute`/`Inventar`/`Fortschritt` (and their `TabBarView` children) with a single `Charaktere` tab (icon `Icons.people_outline`, `l10n.tabCharacters`) whose child is `const CharactersTab()`. Leave the `Welt` tab as a placeholder for Task 11 (temporarily route it to a `_MessagePane` "coming soon" so the app compiles between tasks). Update `DefaultTabController(length: 7)` → the interim count during Phase 2, final `length: 6` after Task 11.

- [ ] **Step 3:** Add l10n keys `tabCharacters` ("Charaktere"), `tabWorld` ("Welt"), the badge tooltips, and the "Weitere" header to `app_localizations*.dart` (all languages; German + English are the primary ones — copy English text for the rest as the repo already does).

- [ ] **Step 4: Run + manually verify**

Run: `flutter test test/ && flutter analyze`
Expected: clean. Then launch the app (see project `run` skill) and confirm: selecting Xardas shows attributes/inventory/events populated, Wissen empty + add button; selecting a named NPC with knowledge shows all four.

- [ ] **Step 5: Commit**

```bash
git add apps/save-editor/lib/features/editor/ui/characters_tab.dart apps/save-editor/lib/features/editor/ui/editor_page.dart apps/save-editor/lib/l10n/
git commit -m "feat(save-editor): Charaktere tab — master list + 4 detail sub-tabs"
```

## Task 10: Player/Hero de-dup + delete old `ActorSelector`

**Files:**
- Modify: `crates/gore-save/src/npc.rs` (only if `Hero` is in `list_npcs`)
- Modify: `apps/save-editor/lib/features/editor/ui/editor_page.dart` (remove dead `_AttributePanel`/`_InventoryPanel` selector plumbing)
- Delete: `apps/save-editor/lib/features/editor/ui/actor_selector.dart` (+ its test) once no longer referenced

- [ ] **Step 1: Verify Hero presence.** With a real save loaded, check whether the pinned Player and a `Hero-…` NPC both appear in the master list.

Run: `GORE_SAVE='…\G1R-021.sav' cargo test -p gore-save --test characters_list -- --nocapture` and add a temporary `eprintln!` (or reuse the probe) listing any row whose `uniqueName == "hero"`.
Expected: determine if a `Hero` actor row exists.

- [ ] **Step 2:** If a `Hero` actor row exists, exclude it in `npc::list_characters` (skip any actor with `char_key(id) == "hero"`) so only the pinned Player represents the hero; its knowledge (`Hero`) routes via the Player sub-tab. Add a unit test asserting no `uniqueName == "hero"` actor row remains while the pinned Player still covers it.

- [ ] **Step 3:** Remove the now-unused `ActorSelector` imports/usages and delete `actor_selector.dart` + `actor_selector_test.dart` if nothing references them (keep `localizedNpcName` by moving it to `character_master_list.dart` or a shared helper — grep for other users first).

Run: `grep -rn "actor_selector" apps/save-editor/lib apps/save-editor/test`
Expected: no references before deletion.

- [ ] **Step 4: Run + analyze**

Run: `flutter test test/ && flutter analyze` and `cargo test -p gore-save`
Expected: all green.

- [ ] **Step 5: Commit**

```bash
git add -A
git commit -m "refactor(save-editor): dedup Player/Hero; remove obsolete ActorSelector"
```

---

# PHASE 3 — Welt tab + cleanup

## Task 11: Welt tab (Quests + Fraktionen)

**Files:**
- Create: `apps/save-editor/lib/features/editor/ui/world_tab.dart`
- Modify: `apps/save-editor/lib/features/editor/ui/editor_page.dart`
- Modify: `apps/save-editor/lib/features/editor/ui/progression_panel.dart` (expose `QuestsDetail`/`FactionsDetail`)

- [ ] **Step 1:** Make `_QuestsDetail` and `_FactionsDetail` public (rename to `QuestsDetail`/`FactionsDetail`) or re-export them. They move unchanged (no character list).

- [ ] **Step 2:** Create `WorldTab`: reuse the old progression sidebar pattern (`_SidebarTile`) with just two entries — `Quests` and `Fraktionen` — over an `Offstage` stack of `QuestsDetail` + `FactionsDetail` (same keep-mounted pattern the old `ProgressionPanel` used for pending edits). Two sections are few enough that a plain `TabBar` is also acceptable; match whichever reads cleaner with the existing `_QuestsDetail` layout.

- [ ] **Step 3:** In `editor_page.dart`, replace the Welt placeholder from Task 9 with `const WorldTab()` (icon `Icons.public`, `l10n.tabWorld`). Set `DefaultTabController(length: 6)`.

- [ ] **Step 4: Run + verify**

Run: `flutter test test/ && flutter analyze`
Expected: clean. Launch the app: Welt shows Quests (with its group picker) and Fraktionen (guild matrix), both behaving exactly as before.

- [ ] **Step 5: Commit**

```bash
git add apps/save-editor/lib/features/editor/ui/world_tab.dart apps/save-editor/lib/features/editor/ui/editor_page.dart apps/save-editor/lib/features/editor/ui/progression_panel.dart
git commit -m "feat(save-editor): Welt tab (Quests + Fraktionen)"
```

## Task 12: Delete the dissolved Progression shell + final sweep

**Files:**
- Modify/Delete: `apps/save-editor/lib/features/editor/ui/progression_panel.dart` (remove the `ProgressionPanel` shell + `_ProgSection` sidebar; keep only the detail widgets now used by Charaktere/Welt)
- Modify: `apps/save-editor/lib/features/editor/ui/add_npc_dialog.dart` (delete if now unreferenced)

- [ ] **Step 1:** Remove `ProgressionPanel` and `_ProgSection` (the four-way sidebar) — its sections are now split across Charaktere (Wissen/Ereignisse) and Welt (Quests/Fraktionen). Keep `QuestsDetail`, `FactionsDetail`, `KnowledgeDetail`, `EventsDetail`, `_PaginationBar`, `_SidebarTile`, `_GroupTile`.

- [ ] **Step 2:** Grep for orphaned references and delete now-dead files/keys.

Run: `grep -rn "ProgressionPanel\|showAddNpcDialog\|tabProgression\|ActorSelector" apps/save-editor/lib apps/save-editor/test`
Expected: no live references (remove `tabProgression` l10n key + `add_npc_dialog.dart`/`NpcCatalog` use if fully unreferenced).

- [ ] **Step 3: Full test + analyze + a real-save smoke run**

Run: `flutter test test/ && flutter analyze`
Expected: all green.
Run: `cargo test -p gore-save` (full core suite — regression).
Expected: all green.

- [ ] **Step 4:** Manual regression pass (launch app): all six tabs load; edit an NPC attribute, an NPC inventory item, add knowledge to an NPC with none, remove a memory event, change a quest state, view factions — Save writes them; Reset clears; byte-identical save for an unedited actor.

- [ ] **Step 5: Commit**

```bash
git add -A
git commit -m "refactor(save-editor): remove dissolved Progression shell after restructure"
```

---

## Self-review notes

- **Spec coverage:** §1 core command → Tasks 1–3; §2 tab restructure → Tasks 9, 11; §3 Charaktere → Tasks 7–10; §4 Welt → Task 11; §5 Player/Hero dedup → Task 10; §6 knowledge add-flow → Task 8; §7 orphans → Tasks 1, 7. All covered.
- **Interfaces are consistent:** `CharacterSummary`(Rust, camelCase JSON) → `CharacterRow`(Dart) field names match (`globalId/uniqueName/isDead/hasInventory/hasKnowledge/hasEvents`). `Actor.uniqueName` feeds `KnowledgeDetail.uniqueName`; `Actor.id` feeds `EventsDetail.globalId`.
- **Known verification points deferred to execution (not placeholders):** exact `core_service` call shape (Task 4/6), whether a `Hero` actor row exists (Task 10 Step 1), and the existing knowledge test-fixture builder to copy (Task 3). Each has a concrete command to resolve it.
