//! Trader shop data: what each merchant offers for sale and how much ore he has
//! to buy with.
//!
//! Every trader in the game lives in ONE global array, not on the NPC actor:
//! `m_GenericData["GameStateDataBase"].m_Traders` — an `ArrayProperty` of
//! `FTraderData` (`G1R.hpp:3867`) with exactly six members:
//!
//!   - `m_TradersUniqueName` (`NameProperty`): the NPC's GlobalId without the
//!     `-WorldPointActor…` suffix. **Not unique** — shipped saves carry two
//!     `None` sentinel rows, so rows are addressed by ARRAY INDEX.
//!   - `m_Items` (`MapProperty<ObjectProperty, IntProperty>`): the live stock.
//!     This is both "what he sells" and "how much ore he can pay with" — the
//!     ore is an ordinary entry keyed [`ORE_PATH`].
//!   - `m_DefaultItems` (same shape): the restock baseline. NOT a frozen vanilla
//!     snapshot; it grows as story events grant new batches and the runtime can
//!     rebuild it from its trader configuration. Editing it is therefore not a
//!     durable way to define custom stock.
//!   - `m_GeneratedEvents` (`ArrayProperty<StrProperty>`): which batches this
//!     trader has already been granted (the idempotency ledger).
//!   - `m_ItemsByDifficulty` (`MapProperty`): empty in every save observed.
//!   - `m_TotalSeconds` (`DoubleProperty`): world-clock stamp of the last trade
//!     session, [`NEVER_TRADED`] when the player has never traded here. It is a
//!     timestamp, NOT a restock trigger.
//!
//! Sold-out items are REMOVED from `m_Items`, never left at zero, so "restock
//! this line" is structurally an insert rather than a set.

use serde::Serialize;

use crate::CoreError;
use crate::properties::{Property, PropertyValue, RootObject};

/// The `m_GenericData` key holding the game-state blob that owns `m_Traders`.
const GAME_STATE_KEY: &str = "GameStateDataBase";

/// The array of per-trader shop records inside that blob.
const TRADERS_PROPERTY: &str = "m_Traders";

/// Ore is the colony's currency, and a trader's stock entry for it is his
/// purchasing power ("Liquidität" in the game's own trading tutorial).
pub const ORE_PATH: &str = "/Script/Angelscript.ItMi_Orenugget";

/// `m_TotalSeconds` sentinel for "the player has never traded with this NPC".
pub const NEVER_TRADED: f64 = -1000.0;

/// The placeholder value `m_TradersUniqueName` carries on the rows that belong
/// to no NPC. Two shipped rows share it, which is why nothing may be addressed
/// by name alone.
const PLACEHOLDER_NAME: &str = "None";

/// One line of a trader's stock: an item class and how many he holds.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TraderItem {
    /// Full class path, e.g. `/Script/Angelscript.ItFo_Loaf`. This is the map key.
    pub path: String,
    /// Bare class name, e.g. `ItFo_Loaf`.
    pub id: String,
    pub count: i32,
    /// `true` when the path is not in the bundled item catalog — shown, but not
    /// offered as an edit target.
    pub unknown_item: bool,
}

/// A trader as shown in a list: enough to pick one, not the whole stock.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TraderSummary {
    /// Position in `m_Traders`. The ONLY safe address for an edit.
    pub index: usize,
    pub unique_name: String,
    /// How many distinct item classes he currently stocks (ore included).
    pub item_count: usize,
    pub default_item_count: usize,
    /// His ore, i.e. what he can pay with. `None` when he carries no ore entry
    /// at all — a real state (Riordian, Scorpio, Xardas), not an error.
    pub ore: Option<i32>,
    pub total_seconds: f64,
    /// `false` while `total_seconds` is still [`NEVER_TRADED`].
    pub traded: bool,
    pub generated_event_count: usize,
    /// `true` for the unnamed sentinel rows, which belong to no NPC.
    pub placeholder: bool,
    /// Both stock maps are present AND shaped the way every applier assumes:
    /// an object-path key and a bare `i32` value.
    ///
    /// An omitted map reads as an empty one, and an EMPTY map of some other
    /// shape has no entries to give that shape away — either would look
    /// editable and then fail at save time, since the appliers resolve the
    /// property, encode an object key and patch four bytes. Every shipped save
    /// carries both maps in that shape on all 31 rows, so this guards a shape
    /// we have not seen rather than a known state.
    pub stock_maps_present: bool,
}

/// Everything stored for one trader.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TraderDetail {
    #[serde(flatten)]
    pub summary: TraderSummary,
    /// Live stock, sorted by class name.
    pub items: Vec<TraderItem>,
    /// Saved restock input, sorted by class name. Diverges from `items` in
    /// played saves in both values AND key set, but is runtime-rebuildable and
    /// therefore not a durable custom-stock definition.
    pub default_items: Vec<TraderItem>,
    pub generated_events: Vec<String>,
    /// Typed-property path of `m_TotalSeconds`. Returned by the same recursive
    /// lookup as the reader so clients never have to assume where
    /// `m_GenericData` sits in a particular save shape. Absent when the member
    /// itself is absent or is not a DoubleProperty.
    pub total_seconds_path: Option<Vec<String>>,
    /// `true` when `m_ItemsByDifficulty` holds entries. Empty in every save
    /// observed so far; if this ever flips, the staging map needs modelling.
    pub has_items_by_difficulty: bool,
}

/// Locate `m_Traders` and return its elements.
///
/// Fails rather than returning an empty list when the array is missing: an empty
/// result would be indistinguishable from "this save has no traders", which is
/// not a state the game produces.
fn traders_array(root: &RootObject) -> Result<&[PropertyValue], CoreError> {
    let (_, props) =
        crate::factions::find_generic_instanced(root, GAME_STATE_KEY).ok_or_else(|| {
            CoreError::Parse(format!("m_GenericData[\"{GAME_STATE_KEY}\"] not found"))
        })?;
    let property = props
        .iter()
        .find(|p| p.name == TRADERS_PROPERTY)
        .ok_or_else(|| {
            CoreError::Parse(format!("{GAME_STATE_KEY} has no {TRADERS_PROPERTY} array"))
        })?;
    match &property.value {
        PropertyValue::Array { elements } => Ok(elements.as_slice()),
        _ => Err(CoreError::Parse(format!(
            "{TRADERS_PROPERTY} is not an ArrayProperty"
        ))),
    }
}

/// The tagged property list of one `FTraderData` element.
fn element_props(element: &PropertyValue) -> Option<&[Property]> {
    match element {
        PropertyValue::Struct(crate::properties::StructValue::Properties(p)) => Some(p.as_slice()),
        PropertyValue::Struct(crate::properties::StructValue::Instanced(Some(i))) => {
            Some(i.properties.as_slice())
        }
        _ => None,
    }
}

fn member<'a>(props: &'a [Property], name: &str) -> Option<&'a PropertyValue> {
    props.iter().find(|p| p.name == name).map(|p| &p.value)
}

fn property<'a>(props: &'a [Property], name: &str) -> Option<&'a Property> {
    props.iter().find(|p| p.name == name)
}

/// Whether a stock map is there AND carries the key/value types every applier
/// assumes. Checked on the DESCRIPTOR, because an empty map has no entry to
/// check and `read_stock` can only see the entries.
///
/// Both failures land on the same flag, so whatever reports it has to describe
/// both: a record can be missing a map, or carry one the appliers cannot write.
fn stock_map_is_writable(props: &[Property], name: &str) -> bool {
    let Some(property) = property(props, name) else {
        return false;
    };
    if property.type_name != "MapProperty" {
        return false;
    }
    match property.descriptor.map.as_deref() {
        Some((key, value)) => key.type_name == "ObjectProperty" && value.type_name == "IntProperty",
        None => false,
    }
}

/// Read one stock map, verifying its descriptor as it goes.
///
/// The descriptor check is the write guard in disguise: an edit command patches
/// the last four bytes of an entry on the assumption that keys serialize as an
/// object path and values as a bare `i32`. If a save ever violates that, we must
/// refuse loudly here rather than hand back plausible numbers that a later write
/// would splice into the wrong offset.
fn read_stock(value: Option<&PropertyValue>, what: &str) -> Result<Vec<TraderItem>, CoreError> {
    let Some(value) = value else {
        return Ok(Vec::new());
    };
    let PropertyValue::Map { entries, .. } = value else {
        return Err(CoreError::Parse(format!("{what} is not a MapProperty")));
    };
    let mut items = Vec::with_capacity(entries.len());
    for (key, val) in entries {
        let PropertyValue::Object(path) = key else {
            return Err(CoreError::Parse(format!(
                "{what} key is not an ObjectProperty"
            )));
        };
        let PropertyValue::Int(count) = val else {
            return Err(CoreError::Parse(format!(
                "{what}[{path}] is not an IntProperty"
            )));
        };
        items.push(TraderItem {
            id: class_name(path).to_string(),
            unknown_item: !crate::is_item_definition_class(path),
            path: path.clone(),
            count: *count,
        });
    }
    items.sort_by(|a, b| a.id.cmp(&b.id));
    Ok(items)
}

/// `/Script/Angelscript.ItFo_Loaf` → `ItFo_Loaf`.
fn class_name(path: &str) -> &str {
    path.rsplit('.').next().unwrap_or(path)
}

/// A trader's ore, looked up BY KEY.
///
/// Never read positionally: the ore is not reliably the first entry (Cronos and
/// Riordian lead with `ItMs_Remedy`, Fisk with `ItKe_Lockpick`).
fn ore_of(items: &[TraderItem]) -> Option<i32> {
    items.iter().find(|i| i.path == ORE_PATH).map(|i| i.count)
}

fn summarize(
    index: usize,
    props: &[Property],
) -> Result<(TraderSummary, Vec<TraderItem>, Vec<TraderItem>), CoreError> {
    let unique_name = match member(props, "m_TradersUniqueName") {
        Some(PropertyValue::Name(n)) => n.clone(),
        Some(_) => {
            return Err(CoreError::Parse(format!(
                "trader[{index}].m_TradersUniqueName is not a NameProperty"
            )));
        }
        None => PLACEHOLDER_NAME.to_string(),
    };
    let items = read_stock(
        member(props, "m_Items"),
        &format!("trader[{index}].m_Items"),
    )?;
    let default_items = read_stock(
        member(props, "m_DefaultItems"),
        &format!("trader[{index}].m_DefaultItems"),
    )?;
    let total_seconds = match member(props, "m_TotalSeconds") {
        Some(PropertyValue::Double(d)) => *d,
        _ => NEVER_TRADED,
    };
    let generated_event_count = match member(props, "m_GeneratedEvents") {
        Some(PropertyValue::Array { elements }) => elements.len(),
        _ => 0,
    };
    let summary = TraderSummary {
        index,
        stock_maps_present: stock_map_is_writable(props, "m_Items")
            && stock_map_is_writable(props, "m_DefaultItems"),
        placeholder: unique_name == PLACEHOLDER_NAME,
        unique_name,
        item_count: items.len(),
        default_item_count: default_items.len(),
        ore: ore_of(&items),
        total_seconds,
        traded: total_seconds > NEVER_TRADED,
        generated_event_count,
    };
    Ok((summary, items, default_items))
}

/// Every trader in the save, in array order.
pub fn list_traders(root: &RootObject) -> Result<Vec<TraderSummary>, CoreError> {
    let elements = traders_array(root)?;
    let mut out = Vec::with_capacity(elements.len());
    for (index, element) in elements.iter().enumerate() {
        let props = element_props(element)
            .ok_or_else(|| CoreError::Parse(format!("trader[{index}] is not a struct element")))?;
        out.push(summarize(index, props)?.0);
    }
    Ok(out)
}

/// One trader's full record, addressed by array index.
pub fn trader_detail(root: &RootObject, index: usize) -> Result<TraderDetail, CoreError> {
    let elements = traders_array(root)?;
    let element = elements.get(index).ok_or_else(|| {
        CoreError::InvalidRequest(format!(
            "trader index {index} out of range (have {})",
            elements.len()
        ))
    })?;
    let props = element_props(element)
        .ok_or_else(|| CoreError::Parse(format!("trader[{index}] is not a struct element")))?;
    let (summary, items, default_items) = summarize(index, props)?;
    let generated_events = match member(props, "m_GeneratedEvents") {
        Some(PropertyValue::Array { elements }) => elements
            .iter()
            .map(|e| match e {
                PropertyValue::Str(s) => s.clone(),
                other => format!("{other:?}"),
            })
            .collect(),
        _ => Vec::new(),
    };
    let has_items_by_difficulty = matches!(
        member(props, "m_ItemsByDifficulty"),
        Some(PropertyValue::Map { entries, .. }) if !entries.is_empty()
    );
    let total_seconds_path = if matches!(
        member(props, "m_TotalSeconds"),
        Some(PropertyValue::Double(_))
    ) {
        let (mut path, _) = crate::factions::find_generic_instanced(root, GAME_STATE_KEY)
            .ok_or_else(|| {
                CoreError::Parse(format!("m_GenericData[\"{GAME_STATE_KEY}\"] not found"))
            })?;
        path.push(format!("{{{GAME_STATE_KEY}}}"));
        path.push(TRADERS_PROPERTY.to_string());
        path.push(format!("[{index}]"));
        path.push("m_TotalSeconds".to_string());
        Some(path)
    } else {
        None
    };
    Ok(TraderDetail {
        summary,
        items,
        default_items,
        generated_events,
        total_seconds_path,
        has_items_by_difficulty,
    })
}

/// Resolve a trader by `m_TradersUniqueName`.
///
/// Case-insensitively, because a caller's name comes from
/// `private.characters.list`, which returns the stored knowledge key where one
/// exists — and that key's casing can differ from the trader row's while the
/// same list marks the character a trader through a lowercase join. An exact
/// compare would mark him and then fail to find him.
///
/// Rejects an ambiguous name instead of picking the first hit: the two sentinel
/// rows share the name `None` and are otherwise indistinguishable, so silently
/// choosing one would edit an arbitrary record.
pub fn index_of_unique_name(
    summaries: &[TraderSummary],
    unique_name: &str,
) -> Result<usize, CoreError> {
    let wanted = unique_name.to_ascii_lowercase();
    let mut matches = summaries
        .iter()
        .filter(|s| s.unique_name.to_ascii_lowercase() == wanted);
    let first = matches
        .next()
        .ok_or_else(|| CoreError::InvalidRequest(format!("no trader named {unique_name}")))?;
    if matches.next().is_some() {
        return Err(CoreError::InvalidRequest(format!(
            "trader name {unique_name} is ambiguous; address by index"
        )));
    }
    Ok(first.index)
}

/// Which of a trader's two stock maps an edit targets.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StockMap {
    /// `m_Items` — what he has right now.
    Current,
    /// `m_DefaultItems` — what he restocks back toward.
    Default,
}

impl StockMap {
    pub fn property_name(self) -> &'static str {
        match self {
            StockMap::Current => "m_Items",
            StockMap::Default => "m_DefaultItems",
        }
    }

    pub fn parse(raw: &str) -> Result<Self, CoreError> {
        match raw {
            "current" | "m_Items" => Ok(StockMap::Current),
            "default" | "m_DefaultItems" => Ok(StockMap::Default),
            other => Err(CoreError::InvalidRequest(format!(
                "unknown stock map {other:?}; expected \"current\" or \"default\""
            ))),
        }
    }
}

/// Set one existing stock line to a new count.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SetStockEdit {
    pub index: usize,
    pub map: StockMap,
    /// Full item class path, i.e. the map key.
    pub path: String,
    pub count: i32,
}

/// Apply a stock-count change in place.
///
/// This is a fixed-size write: the value is a bare `i32` at the tail of its map
/// entry, so nothing moves and no enclosing size field changes. That is why
/// several of these batch safely into one save, unlike the insert/remove ops.
///
/// Only EXISTING lines can be set. A sold-out item is deleted from the map
/// rather than left at zero, so "put it back" is an insert and belongs to a
/// different command — silently creating the entry here would hide that
/// difference behind a write that cannot actually do it.
pub fn apply_set_stock(payload: &mut [u8], edit: &SetStockEdit) -> Result<(), CoreError> {
    // Zero is not a count the game writes: it deletes the line instead. Refuse
    // it here as well as at the request boundary, since this is public.
    if edit.count < 1 {
        return Err(CoreError::InvalidRequest(format!(
            "stock count must be positive; remove {} instead of setting it to {}",
            edit.path, edit.count
        )));
    }
    let root = crate::properties::parse_private_root(payload)?;
    let (generic_path, _) = crate::factions::find_generic_instanced(&root, GAME_STATE_KEY)
        .ok_or_else(|| {
            CoreError::Parse(format!("m_GenericData[\"{GAME_STATE_KEY}\"] not found"))
        })?;

    let mut segments = generic_path;
    segments.push(format!("{{{GAME_STATE_KEY}}}"));
    segments.push(TRADERS_PROPERTY.to_string());
    segments.push(format!("[{}]", edit.index));
    segments.push(edit.map.property_name().to_string());
    let path = crate::properties::parse_path(&segments)?;
    let chain = crate::properties::resolve_chain(&root.properties, &path)?;
    let property = chain.target;

    let layout = crate::properties::map_layout(payload, property)?;
    // Entry order in the parsed value and in `map_layout` is the same walk over
    // the same bytes, so the parsed key at position i addresses entry_ranges[i].
    let PropertyValue::Map { entries, .. } = &property.value else {
        return Err(CoreError::Parse(format!(
            "{} is not a MapProperty",
            edit.map.property_name()
        )));
    };
    if entries.len() != layout.entry_ranges.len() {
        return Err(CoreError::Parse(
            "map entry count disagrees between parsed value and byte layout".to_string(),
        ));
    }
    let position = entries
        .iter()
        .position(|(k, _)| matches!(k, PropertyValue::Object(p) if *p == edit.path))
        .ok_or_else(|| {
            CoreError::UnsupportedEdit(format!(
                "trader[{}].{} has no entry for {} — adding a sold-out line needs an insert",
                edit.index,
                edit.map.property_name(),
                edit.path
            ))
        })?;
    // Guard the value shape before trusting the tail-4-bytes assumption.
    if !matches!(entries[position].1, PropertyValue::Int(_)) {
        return Err(CoreError::Parse(format!(
            "trader[{}].{}[{}] is not an IntProperty",
            edit.index,
            edit.map.property_name(),
            edit.path
        )));
    }
    let range = &layout.entry_ranges[position];
    let value_at = range
        .end
        .checked_sub(4)
        .filter(|start| *start >= range.start)
        .ok_or_else(|| CoreError::Parse("map entry shorter than its value".to_string()))?;
    payload[value_at..range.end].copy_from_slice(&edit.count.to_le_bytes());
    Ok(())
}

/// Add a stock line that does not exist yet, or drop one entirely.
///
/// Both are structural: they splice bytes into or out of the map body and shift
/// every offset after it, so each must be the only edit in its write.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StockLineEdit {
    pub index: usize,
    pub map: StockMap,
    /// Full item class path, i.e. the map key.
    pub path: String,
    /// Starting count for an insert. Ignored when removing.
    pub count: i32,
}

/// Resolve one trader's stock map to a patchable target.
///
/// Returns the cloned property plus its enclosing size-field chain, so the
/// borrow on the parsed tree is dropped before the caller mutates the payload.
fn resolve_stock_map(
    payload: &[u8],
    index: usize,
    map: StockMap,
) -> Result<(Property, Vec<usize>, Vec<String>), CoreError> {
    let root = crate::properties::parse_private_root(payload)?;
    let (generic_path, _) = crate::factions::find_generic_instanced(&root, GAME_STATE_KEY)
        .ok_or_else(|| {
            CoreError::Parse(format!("m_GenericData[\"{GAME_STATE_KEY}\"] not found"))
        })?;
    // Fail on a bad index here rather than letting the path resolver report it:
    // "trader index 40 out of range (have 31)" is actionable, "no such array
    // element" is not.
    let elements = traders_array(&root)?;
    if index >= elements.len() {
        return Err(CoreError::InvalidRequest(format!(
            "trader index {index} out of range (have {})",
            elements.len()
        )));
    }
    let mut segments = generic_path;
    segments.push(format!("{{{GAME_STATE_KEY}}}"));
    segments.push(TRADERS_PROPERTY.to_string());
    segments.push(format!("[{index}]"));
    segments.push(map.property_name().to_string());
    let path = crate::properties::parse_path(&segments)?;
    let chain = crate::properties::resolve_chain(&root.properties, &path)?;
    let keys = match &chain.target.value {
        PropertyValue::Map { entries, .. } => entries
            .iter()
            .map(|(k, _)| match k {
                PropertyValue::Object(p) => Ok(p.clone()),
                _ => Err(CoreError::Parse(format!(
                    "trader[{index}].{} key is not an ObjectProperty",
                    map.property_name()
                ))),
            })
            .collect::<Result<Vec<_>, _>>()?,
        _ => {
            return Err(CoreError::Parse(format!(
                "trader[{index}].{} is not a MapProperty",
                map.property_name()
            )));
        }
    };
    Ok((
        chain.target.clone(),
        chain.enclosing_size_fields.clone(),
        keys,
    ))
}

/// Read back one trader's stock keys, used to validate a structural patch before
/// it is allowed to replace the caller's payload.
fn stock_keys_after(payload: &[u8], index: usize, map: StockMap) -> Result<Vec<String>, CoreError> {
    let root = crate::properties::parse_private_root(payload)?;
    let detail = trader_detail(&root, index)?;
    let items = match map {
        StockMap::Current => detail.items,
        StockMap::Default => detail.default_items,
    };
    Ok(items.into_iter().map(|i| i.path).collect())
}

/// Insert a new stock line.
///
/// The item path is checked against the bundled catalog: an unknown class would
/// serialize fine and then resolve to nothing in game, leaving a line the player
/// can neither see nor buy.
///
/// Only ONE map is touched per call. Adding to both `m_Items` and
/// `m_DefaultItems` is two structural edits, which the write guard refuses to
/// batch — the caller submits them as two writes.
pub fn apply_add_item(payload: &mut Vec<u8>, edit: &StockLineEdit) -> Result<(), CoreError> {
    if !crate::is_item_definition_class(&edit.path) {
        return Err(CoreError::InvalidRequest(format!(
            "{} is not a known item class",
            edit.path
        )));
    }
    // Same boundary apply_set_stock draws, and for the same reason: a
    // zero-valued entry is a record the game never writes. This is public, so
    // the request parser is not the only way in.
    if edit.count < 1 {
        return Err(CoreError::InvalidRequest(format!(
            "stock count must be positive; {} cannot be inserted with {}",
            edit.path, edit.count
        )));
    }
    let (target, enclosing, keys) = resolve_stock_map(payload, edit.index, edit.map)?;
    if keys.iter().any(|k| k == &edit.path) {
        return Err(CoreError::InvalidRequest(format!(
            "trader[{}].{} already stocks {} — use setStock to change the count",
            edit.index,
            edit.map.property_name(),
            edit.path
        )));
    }
    let mut entry = crate::properties::encode_fstring_value(&edit.path);
    entry.extend_from_slice(&edit.count.to_le_bytes());

    // Patch a scratch copy first: a length-changing splice that produced an
    // inconsistent payload must never reach the caller.
    let mut patched = payload.clone();
    crate::properties::patch_container(
        &mut patched,
        &target,
        &enclosing,
        &crate::properties::ContainerEdit::MapInsert { entry_bytes: entry },
    )?;
    let keys_after = stock_keys_after(&patched, edit.index, edit.map).map_err(|err| {
        CoreError::Parse(format!("adding a trader stock line left the save unreadable: {err}"))
    })?;
    if !keys_after.iter().any(|k| k == &edit.path) {
        return Err(CoreError::Parse(
            "post-insert validation failed: the new stock line does not read back".to_string(),
        ));
    }
    *payload = patched;
    Ok(())
}

/// Drop a stock line entirely.
///
/// This is what the game itself does when a trader sells out, so it is also the
/// honest way to say "he no longer offers this" — setting the count to zero
/// would leave a line the game never writes.
pub fn apply_remove_item(payload: &mut Vec<u8>, edit: &StockLineEdit) -> Result<(), CoreError> {
    let (target, enclosing, keys) = resolve_stock_map(payload, edit.index, edit.map)?;
    // `map_layout`'s entry order is the same walk over the same bytes as the
    // parsed value's, so the parsed position is the on-disk entry index.
    let position = keys.iter().position(|k| k == &edit.path).ok_or_else(|| {
        CoreError::InvalidRequest(format!(
            "trader[{}].{} has no entry for {}",
            edit.index,
            edit.map.property_name(),
            edit.path
        ))
    })?;

    let mut patched = payload.clone();
    crate::properties::patch_container(
        &mut patched,
        &target,
        &enclosing,
        &crate::properties::ContainerEdit::MapRemove {
            entry_index: position,
        },
    )?;
    let keys_after = stock_keys_after(&patched, edit.index, edit.map).map_err(|err| {
        CoreError::Parse(format!(
            "removing a trader stock line left the save unreadable: {err}"
        ))
    })?;
    if keys_after.iter().any(|k| k == &edit.path) {
        return Err(CoreError::Parse(
            "post-remove validation failed: the stock line is still present".to_string(),
        ));
    }
    *payload = patched;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::properties::{InstancedStruct, StructValue};

    /// A tagged property with no byte backing. These tests exercise the shape
    /// logic only; offsets are irrelevant because nothing here writes.
    fn prop(name: &str, value: PropertyValue) -> Property {
        Property {
            name: name.to_string().into(),
            type_name: String::new().into(),
            descriptor: Default::default(),
            array_index: 0,
            tag_flags: 0,
            value_offset: 0,
            value_size: 0,
            value,
        }
    }

    /// A stock map property with the real key/value descriptor, since the
    /// writability check reads the descriptor rather than the entries.
    fn stock_prop(name: &str, pairs: &[(&str, i32)]) -> Property {
        let inner = |type_name: &str| crate::properties::InnerDescriptor {
            type_name: type_name.to_string().into(),
            struct_type: None,
            enum_type: None,
        };
        let mut p = prop(name, stock(pairs));
        p.type_name = "MapProperty".to_string().into();
        p.descriptor.map = Some(Box::new((
            inner("ObjectProperty"),
            inner("IntProperty"),
        )));
        p
    }

    fn stock(pairs: &[(&str, i32)]) -> PropertyValue {
        PropertyValue::Map {
            num_to_remove: 0,
            entries: pairs
                .iter()
                .map(|(p, c)| {
                    (
                        PropertyValue::Object((*p).to_string()),
                        PropertyValue::Int(*c),
                    )
                })
                .collect(),
        }
    }

    fn trader(name: &str, items: &[(&str, i32)], seconds: f64) -> PropertyValue {
        PropertyValue::Struct(StructValue::Properties(vec![
            prop("m_TradersUniqueName", PropertyValue::Name(name.to_string())),
            stock_prop("m_Items", items),
            stock_prop("m_DefaultItems", items),
            prop(
                "m_GeneratedEvents",
                PropertyValue::Array {
                    elements: vec![PropertyValue::Str("OnWorldStart".to_string())],
                },
            ),
            prop(
                "m_ItemsByDifficulty",
                PropertyValue::Map {
                    num_to_remove: 0,
                    entries: Vec::new(),
                },
            ),
            prop("m_TotalSeconds", PropertyValue::Double(seconds)),
        ]))
    }

    fn root_with(traders: Vec<PropertyValue>) -> RootObject {
        let blob = InstancedStruct {
            actual_type: "GameStateDataBaseSaveData".to_string().into(),
            data_size_offset: 0,
            properties: vec![prop(
                "m_Traders",
                PropertyValue::Array { elements: traders },
            )],
        };
        RootObject {
            class: "UGothicPersistentDataGame".to_string(),
            flag: 0,
            properties: vec![prop(
                "m_GenericData",
                PropertyValue::Map {
                    num_to_remove: 0,
                    entries: vec![(
                        PropertyValue::Str(GAME_STATE_KEY.to_string()),
                        PropertyValue::Struct(StructValue::Instanced(Some(blob))),
                    )],
                },
            )],
            footer: 0,
            consumed: 0,
        }
    }

    #[test]
    fn ore_is_found_by_key_not_by_position() {
        // Fisk's ore is not the first entry; a positional read would return the
        // lockpick count instead.
        let root = root_with(vec![trader(
            "OC_STT_Fisk_311",
            &[("/Script/Angelscript.ItKe_Lockpick", 3), (ORE_PATH, 50)],
            1344287.223,
        )]);
        let list = list_traders(&root).expect("list");
        assert_eq!(list[0].ore, Some(50));
    }

    #[test]
    fn an_omitted_stock_map_is_reported_as_absent() {
        // An omitted map parses as an empty one, which would look editable and
        // then fail at save time — the structural appliers resolve the property
        // and cannot create it. Shipped saves always carry both.
        let bare = PropertyValue::Struct(StructValue::Properties(vec![prop(
            "m_TradersUniqueName",
            PropertyValue::Name("OC_STT_Dexter_329".to_string()),
        )]));
        let list = list_traders(&root_with(vec![bare])).expect("list");
        assert!(!list[0].stock_maps_present);

        // A map of the wrong shape is just as unusable, and an EMPTY one gives
        // that away only through its descriptor.
        let mut wrong = prop("m_Items", stock(&[]));
        wrong.type_name = "MapProperty".to_string().into();
        wrong.descriptor.map = None;
        let odd = PropertyValue::Struct(StructValue::Properties(vec![
            prop("m_TradersUniqueName", PropertyValue::Name("X".to_string())),
            wrong,
            stock_prop("m_DefaultItems", &[]),
        ]));
        assert!(!list_traders(&root_with(vec![odd])).expect("list")[0].stock_maps_present);

        let whole = root_with(vec![trader("OC_STT_Fisk_311", &[(ORE_PATH, 3)], 1.0)]);
        assert!(list_traders(&whole).expect("list")[0].stock_maps_present);
    }

    #[test]
    fn missing_ore_entry_is_none_not_zero() {
        // Riordian stocks goods but carries no ore key at all. Reporting 0 would
        // claim he is broke; reporting None says the record has no such line.
        let root = root_with(vec![trader(
            "NC_KDW_Riordian_605",
            &[("/Script/Angelscript.ItMs_Remedy", 4)],
            NEVER_TRADED,
        )]);
        let list = list_traders(&root).expect("list");
        assert_eq!(list[0].ore, None);
        assert!(!list[0].traded);
    }

    #[test]
    fn name_lookup_folds_case() {
        // A character's unique name is the stored knowledge key where one
        // exists, whose casing can differ from the trader row's. The character
        // list marks him a trader through a lowercase join, so an exact compare
        // here would mark him and then refuse to resolve him.
        let root = root_with(vec![trader(
            "OC_STT_Dexter_329",
            &[(ORE_PATH, 55)],
            937101.34,
        )]);
        let list = list_traders(&root).expect("list");
        assert_eq!(index_of_unique_name(&list, "oc_stt_dexter_329").unwrap(), 0);
        assert_eq!(index_of_unique_name(&list, "OC_stt_Dexter_329").unwrap(), 0);
    }

    #[test]
    fn case_only_duplicates_are_still_ambiguous() {
        // Folding case must not turn two distinct rows into a silent pick.
        let root = root_with(vec![
            trader("OC_STT_Dexter_329", &[(ORE_PATH, 1)], NEVER_TRADED),
            trader("oc_stt_dexter_329", &[(ORE_PATH, 2)], NEVER_TRADED),
        ]);
        let list = list_traders(&root).expect("list");
        let err = index_of_unique_name(&list, "OC_STT_Dexter_329").unwrap_err();
        assert!(matches!(err, CoreError::InvalidRequest(m) if m.contains("ambiguous")));
    }

    #[test]
    fn duplicate_none_rows_are_rejected_by_name_lookup() {
        let root = root_with(vec![
            trader("None", &[(ORE_PATH, 75)], NEVER_TRADED),
            trader("OC_STT_Dexter_329", &[(ORE_PATH, 55)], 937101.34),
            trader("None", &[(ORE_PATH, 75)], NEVER_TRADED),
        ]);
        let list = list_traders(&root).expect("list");
        assert!(list[0].placeholder && list[2].placeholder);
        assert_eq!(index_of_unique_name(&list, "OC_STT_Dexter_329").unwrap(), 1);
        let err = index_of_unique_name(&list, "None").unwrap_err();
        assert!(matches!(err, CoreError::InvalidRequest(m) if m.contains("ambiguous")));
    }

    #[test]
    fn non_int_stock_value_is_refused() {
        // A float-valued stock map would make the 4-byte in-place patch write
        // into the wrong bytes, so the read must fail rather than round it.
        let bad = PropertyValue::Struct(StructValue::Properties(vec![
            prop("m_TradersUniqueName", PropertyValue::Name("X".to_string())),
            prop(
                "m_Items",
                PropertyValue::Map {
                    num_to_remove: 0,
                    entries: vec![(
                        PropertyValue::Object(ORE_PATH.to_string()),
                        PropertyValue::Float(1.0),
                    )],
                },
            ),
        ]));
        let err = list_traders(&root_with(vec![bad])).unwrap_err();
        assert!(matches!(err, CoreError::Parse(m) if m.contains("IntProperty")));
    }

    /// Decode a real shipped save so the write tests run against genuine bytes.
    /// The synthetic trees above carry no byte backing, and `apply_set_stock`
    /// writes into the payload — a fake tree could not catch an offset mistake.
    fn real_payload() -> Vec<u8> {
        let backend = crate::codec_backend::KrakenBackend;
        crate::decode_private_payload_from_bytes(
            crate::startsaves::start_save_bytes(crate::startsaves::ResourcesLevel::Gothic),
            &backend,
        )
        .expect("decode embedded start save")
    }

    fn ore_at(payload: &[u8], index: usize) -> Option<i32> {
        let root = crate::properties::parse_private_root(payload).expect("parse");
        trader_detail(&root, index).expect("detail").summary.ore
    }

    /// Index of the first shipped trader that actually stocks ore.
    fn first_ore_trader(payload: &[u8]) -> usize {
        let root = crate::properties::parse_private_root(payload).expect("parse");
        list_traders(&root)
            .expect("list")
            .into_iter()
            .find(|t| t.ore.is_some() && !t.placeholder)
            .expect("some trader stocks ore")
            .index
    }

    #[test]
    fn set_stock_writes_in_place_without_moving_bytes() {
        let mut payload = real_payload();
        let before_len = payload.len();
        let index = first_ore_trader(&payload);

        let edit = SetStockEdit {
            index,
            map: StockMap::Current,
            path: ORE_PATH.to_string(),
            count: 4242,
        };
        apply_set_stock(&mut payload, &edit).expect("apply");

        assert_eq!(payload.len(), before_len, "the edit must be length-neutral");
        assert_eq!(ore_at(&payload, index), Some(4242));
    }

    #[test]
    fn two_stock_edits_batch_without_invalidating_each_other() {
        // Length-neutral writes leave every recorded offset valid, which is the
        // whole reason this command may share a save with its peers.
        let mut payload = real_payload();
        let a = first_ore_trader(&payload);
        let root = crate::properties::parse_private_root(&payload).expect("parse");
        let b = list_traders(&root)
            .expect("list")
            .into_iter()
            .find(|t| t.ore.is_some() && !t.placeholder && t.index != a)
            .expect("a second ore trader")
            .index;

        for (index, count) in [(a, 1), (b, 999)] {
            apply_set_stock(
                &mut payload,
                &SetStockEdit {
                    index,
                    map: StockMap::Current,
                    path: ORE_PATH.to_string(),
                    count,
                },
            )
            .expect("apply");
        }
        assert_eq!(ore_at(&payload, a), Some(1));
        assert_eq!(ore_at(&payload, b), Some(999));
    }

    #[test]
    fn set_stock_refuses_a_zero_count() {
        // The map holds no zero-valued entry in any shipped or played save, so
        // writing one would invent a state the game never produces. Dropping the
        // line is what "he no longer offers this" actually looks like.
        let mut payload = real_payload();
        let index = first_ore_trader(&payload);
        let before = payload.clone();
        let err = apply_set_stock(
            &mut payload,
            &SetStockEdit {
                index,
                map: StockMap::Current,
                path: ORE_PATH.to_string(),
                count: 0,
            },
        )
        .unwrap_err();
        assert!(matches!(err, CoreError::InvalidRequest(m) if m.contains("positive")));
        assert_eq!(payload, before);
    }

    #[test]
    fn set_stock_refuses_a_line_that_does_not_exist() {
        // Sold-out items are deleted from the map, so "set it to 5" cannot mean
        // "create it" — that needs an insert, and pretending otherwise would
        // report success for a write that never happened.
        let mut payload = real_payload();
        let index = first_ore_trader(&payload);
        let err = apply_set_stock(
            &mut payload,
            &SetStockEdit {
                index,
                map: StockMap::Current,
                path: "/Script/Angelscript.ItMw_2H_Sword_04".to_string(),
                count: 5,
            },
        )
        .unwrap_err();
        assert!(matches!(err, CoreError::UnsupportedEdit(m) if m.contains("insert")));
    }

    #[test]
    fn set_stock_targets_the_requested_map_only() {
        let mut payload = real_payload();
        let index = first_ore_trader(&payload);
        let before = {
            let root = crate::properties::parse_private_root(&payload).expect("parse");
            trader_detail(&root, index).expect("detail")
        };
        let default_ore_before = before
            .default_items
            .iter()
            .find(|i| i.path == ORE_PATH)
            .map(|i| i.count);

        apply_set_stock(
            &mut payload,
            &SetStockEdit {
                index,
                map: StockMap::Default,
                path: ORE_PATH.to_string(),
                count: 7,
            },
        )
        .expect("apply");

        let root = crate::properties::parse_private_root(&payload).expect("parse");
        let after = trader_detail(&root, index).expect("detail");
        assert_eq!(
            after
                .default_items
                .iter()
                .find(|i| i.path == ORE_PATH)
                .map(|i| i.count),
            Some(7)
        );
        assert_eq!(after.summary.ore, before.summary.ore, "m_Items untouched");
        assert_ne!(default_ore_before, Some(7), "the test would be vacuous");
    }

    #[test]
    fn shipped_save_has_the_documented_trader_shape() {
        let payload = real_payload();
        let root = crate::properties::parse_private_root(&payload).expect("parse");
        let list = list_traders(&root).expect("list");
        assert_eq!(list.len(), 31, "every shipped save carries 31 trader rows");
        // Every shipped row carries both maps in the shape the appliers assume.
        assert!(list.iter().all(|t| t.stock_maps_present));
        assert_eq!(
            list.iter().filter(|t| t.placeholder).count(),
            2,
            "two rows are unnamed sentinels, which is why names cannot address a row"
        );
        // A game-start save has been traded with nowhere.
        assert!(list.iter().all(|t| !t.traded));
        // The staging map is empty everywhere; if this ever fires, it needs modelling.
        for index in 0..list.len() {
            assert!(!trader_detail(&root, index).unwrap().has_items_by_difficulty);
        }
    }

    /// An item class in the bundled catalog that no shipped trader stocks, so an
    /// insert test cannot collide with existing data.
    fn unstocked_catalog_item(payload: &[u8], index: usize) -> String {
        let root = crate::properties::parse_private_root(payload).expect("parse");
        let held: std::collections::HashSet<String> = trader_detail(&root, index)
            .expect("detail")
            .items
            .into_iter()
            .map(|i| i.path)
            .collect();
        crate::item_catalog_paths()
            .iter()
            .find(|p| !held.contains(*p))
            .expect("catalog has an item this trader does not stock")
            .clone()
    }

    #[test]
    fn add_item_inserts_a_new_line_and_grows_the_payload() {
        let mut payload = real_payload();
        let index = first_ore_trader(&payload);
        let path = unstocked_catalog_item(&payload, index);
        let before_len = payload.len();
        let before_count = {
            let root = crate::properties::parse_private_root(&payload).expect("parse");
            trader_detail(&root, index).expect("detail").items.len()
        };

        apply_add_item(
            &mut payload,
            &StockLineEdit {
                index,
                map: StockMap::Current,
                path: path.clone(),
                count: 7,
            },
        )
        .expect("add");

        assert!(payload.len() > before_len, "an insert must grow the payload");
        let root = crate::properties::parse_private_root(&payload).expect("reparse");
        let detail = trader_detail(&root, index).expect("detail");
        assert_eq!(detail.items.len(), before_count + 1);
        let added = detail
            .items
            .iter()
            .find(|i| i.path == path)
            .expect("the new line reads back");
        assert_eq!(added.count, 7);
        assert!(!added.unknown_item);
    }

    #[test]
    fn add_item_rejects_a_class_outside_the_catalog() {
        // An unknown class serializes fine and then resolves to nothing in game,
        // so it would create stock the player can never see.
        let mut payload = real_payload();
        let index = first_ore_trader(&payload);
        let before = payload.clone();
        let err = apply_add_item(
            &mut payload,
            &StockLineEdit {
                index,
                map: StockMap::Current,
                path: "/Script/Angelscript.ItXx_NotAThing".to_string(),
                count: 1,
            },
        )
        .unwrap_err();
        assert!(matches!(err, CoreError::InvalidRequest(m) if m.contains("not a known item class")));
        assert_eq!(payload, before, "a rejected add must not touch the payload");
    }

    #[test]
    fn add_item_rejects_a_zero_count() {
        // A line the merchant holds none of is simply absent from the map, so
        // inserting one at zero would invent a state the game never produces.
        let mut payload = real_payload();
        let index = first_ore_trader(&payload);
        let path = unstocked_catalog_item(&payload, index);
        let before = payload.clone();
        let err = apply_add_item(
            &mut payload,
            &StockLineEdit {
                index,
                map: StockMap::Current,
                path,
                count: 0,
            },
        )
        .unwrap_err();
        assert!(matches!(err, CoreError::InvalidRequest(m) if m.contains("positive")));
        assert_eq!(payload, before);
    }

    #[test]
    fn add_item_rejects_a_line_that_already_exists() {
        let mut payload = real_payload();
        let index = first_ore_trader(&payload);
        let before = payload.clone();
        let err = apply_add_item(
            &mut payload,
            &StockLineEdit {
                index,
                map: StockMap::Current,
                path: ORE_PATH.to_string(),
                count: 1,
            },
        )
        .unwrap_err();
        assert!(matches!(err, CoreError::InvalidRequest(m) if m.contains("already stocks")));
        assert_eq!(payload, before);
    }

    #[test]
    fn remove_item_drops_the_line_and_shrinks_the_payload() {
        let mut payload = real_payload();
        let index = first_ore_trader(&payload);
        let before_len = payload.len();
        let before = {
            let root = crate::properties::parse_private_root(&payload).expect("parse");
            trader_detail(&root, index).expect("detail")
        };
        // Pick a non-ore line so the removal is not confused with the ore path.
        let victim = before
            .items
            .iter()
            .find(|i| i.path != ORE_PATH)
            .expect("trader stocks something besides ore")
            .path
            .clone();

        apply_remove_item(
            &mut payload,
            &StockLineEdit {
                index,
                map: StockMap::Current,
                path: victim.clone(),
                count: 0,
            },
        )
        .expect("remove");

        assert!(payload.len() < before_len, "a removal must shrink the payload");
        let root = crate::properties::parse_private_root(&payload).expect("reparse");
        let after = trader_detail(&root, index).expect("detail");
        assert_eq!(after.items.len(), before.items.len() - 1);
        assert!(after.items.iter().all(|i| i.path != victim));
        // The neighbours must survive intact — a wrong entry index would eat one.
        assert_eq!(after.summary.ore, before.summary.ore);
        assert_eq!(after.default_items.len(), before.default_items.len());
    }

    #[test]
    fn remove_item_refuses_a_line_that_does_not_exist() {
        let mut payload = real_payload();
        let index = first_ore_trader(&payload);
        let path = unstocked_catalog_item(&payload, index);
        let before = payload.clone();
        let err = apply_remove_item(
            &mut payload,
            &StockLineEdit {
                index,
                map: StockMap::Current,
                path,
                count: 0,
            },
        )
        .unwrap_err();
        assert!(matches!(err, CoreError::InvalidRequest(m) if m.contains("no entry for")));
        assert_eq!(payload, before);
    }

    #[test]
    fn add_then_remove_restores_the_original_bytes() {
        // Round-tripping proves the size-field chain is fixed up symmetrically:
        // a leftover byte anywhere would show up as a length or content diff.
        let original = real_payload();
        let mut payload = original.clone();
        let index = first_ore_trader(&payload);
        let path = unstocked_catalog_item(&payload, index);

        apply_add_item(
            &mut payload,
            &StockLineEdit {
                index,
                map: StockMap::Current,
                path: path.clone(),
                count: 3,
            },
        )
        .expect("add");
        apply_remove_item(
            &mut payload,
            &StockLineEdit {
                index,
                map: StockMap::Current,
                path,
                count: 0,
            },
        )
        .expect("remove");

        assert_eq!(payload, original, "add+remove must be byte-identical to a no-op");
    }

    #[test]
    fn structural_edits_reject_an_out_of_range_trader() {
        let mut payload = real_payload();
        let err = apply_add_item(
            &mut payload,
            &StockLineEdit {
                index: 9999,
                map: StockMap::Current,
                path: ORE_PATH.to_string(),
                count: 1,
            },
        )
        .unwrap_err();
        assert!(matches!(err, CoreError::InvalidRequest(m) if m.contains("out of range")));
    }

    #[test]
    fn add_item_into_an_empty_trader_map() {
        // Scorpio and Xardas ship with no stock at all, so the insert path must
        // work against a zero-entry map, not just append after an existing one.
        let mut payload = real_payload();
        let index = {
            let root = crate::properties::parse_private_root(&payload).expect("parse");
            list_traders(&root)
                .expect("list")
                .into_iter()
                .find(|t| t.item_count == 0 && !t.placeholder)
                .expect("a shipped trader with an empty stock map")
                .index
        };
        apply_add_item(
            &mut payload,
            &StockLineEdit {
                index,
                map: StockMap::Current,
                path: ORE_PATH.to_string(),
                count: 250,
            },
        )
        .expect("add into empty map");

        let root = crate::properties::parse_private_root(&payload).expect("reparse");
        let detail = trader_detail(&root, index).expect("detail");
        assert_eq!(detail.items.len(), 1);
        assert_eq!(detail.summary.ore, Some(250));
    }

    #[test]
    fn detail_reports_events_and_empty_difficulty_map() {
        let root = root_with(vec![trader(
            "OC_STT_Dexter_329",
            &[(ORE_PATH, 55)],
            937101.34,
        )]);
        let detail = trader_detail(&root, 0).expect("detail");
        assert_eq!(detail.generated_events, vec!["OnWorldStart".to_string()]);
        assert!(!detail.has_items_by_difficulty);
        assert!(trader_detail(&root, 7).is_err());
    }
}
