//! Library-entry metadata: the cross-tool contract describing one imported mod.
//!
//! Every mod in the manager library is a normalized dir with a [`META_FILE`] sidecar holding a
//! [`ModEntryMeta`]. These shapes are a locked contract shared across the manager UI, CLI and
//! engine — do not rename fields or variants.

use serde::{Deserialize, Serialize};

/// Sidecar filename inside each library mod dir.
pub const META_FILE: &str = "gore-manager-meta.json";

/// One imported mod in the manager library.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ModEntryMeta {
    /// Stable library id (dir name under the library root).
    pub id: String,
    pub kind: ModKind,
    pub name: String,
    #[serde(default)]
    pub version: String,
    #[serde(default)]
    pub author: String,
    /// RFC 3339 timestamp of the import.
    pub imported_at: String,
    /// Where the mod came from (original archive/dir path), informational.
    #[serde(default)]
    pub source: String,
    pub components: Vec<ComponentInfo>,
}

impl ModEntryMeta {
    /// A content fingerprint that changes when the mod is re-imported as an update (same id, new
    /// components/bytes). `imported_at` alone changes on re-import; hashing it with the serialized
    /// components makes it robust to same-second re-imports with different content.
    pub fn fingerprint(&self) -> String {
        let body = serde_json::to_string(&self.components).unwrap_or_default();
        crate::name_hash(&format!("{}|{}", self.imported_at, body))
    }
}

/// How the mod was recognized at import.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ModKind {
    Goremod,
    ForeignTriplet,
    ForeignPak,
    ForeignUe4ss,
    ForeignRawfile,
    ForeignMixed,
}

/// One deployable component of a library mod. `rel`/`rel_base` are paths inside the mod's
/// library dir; `targets` are the game-side footprint keys used for conflict analysis.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ComponentInfo {
    Ue4ssLua {
        name: String,
        rel: String,
        #[serde(default)]
        targets: Vec<String>,
        #[serde(default)]
        opaque: bool,
    },
    LocPatch {
        rel: String,
        targets: Vec<String>,
    },
    AudioPatch {
        rel: String,
        targets: Vec<String>,
    },
    TexturePatch {
        rel: String,
        targets: Vec<String>,
    },
    AngelScriptPatch {
        rel: String,
        targets: Vec<String>,
    },
    Triplet {
        rel_base: String,
        targets: Vec<String>,
    },
    LoosePak {
        rel: String,
        targets: Vec<String>,
    },
    RawFile {
        rel: String,
        target_file: RawTarget,
    },
}

/// The single live game file a [`ComponentInfo::RawFile`] replaces wholesale.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RawTarget {
    Lcache,
    Bank { name: String },
    ScriptCache,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The library metadata contract must survive a JSON round-trip for EVERY component and
    /// raw-target variant, with the agreed snake_case wire tags.
    #[test]
    fn meta_roundtrips_json() {
        let meta = ModEntryMeta {
            id: "my-mod-1a2b".into(),
            kind: ModKind::ForeignMixed,
            name: "My Mod".into(),
            version: "1.2.3".into(),
            author: "someone".into(),
            imported_at: "2026-07-03T12:00:00Z".into(),
            source: "C:/downloads/my-mod.zip".into(),
            components: vec![
                ComponentInfo::Ue4ssLua {
                    name: "MyMod".into(),
                    rel: "ue4ss/MyMod".into(),
                    targets: vec!["ue4ss:MyMod".into()],
                    opaque: true,
                },
                ComponentInfo::LocPatch { rel: "loc/edits.json".into(), targets: vec!["loc:itfo_x".into()] },
                ComponentInfo::AudioPatch { rel: "audio".into(), targets: vec!["bank:SFX.bank".into()] },
                ComponentInfo::TexturePatch { rel: "texture".into(), targets: vec!["tex:/Game/UI/T_X".into()] },
                ComponentInfo::AngelScriptPatch { rel: "scripts".into(), targets: vec!["as:MyModule".into()] },
                ComponentInfo::Triplet {
                    rel_base: "paks/zzz_MyMod_P".into(),
                    targets: vec!["pak:zzz_MyMod_P".into()],
                },
                ComponentInfo::LoosePak { rel: "paks/extra.pak".into(), targets: vec!["pak:extra".into()] },
                ComponentInfo::RawFile { rel: "raw/loc.lcache".into(), target_file: RawTarget::Lcache },
                ComponentInfo::RawFile {
                    rel: "raw/SFX.bank".into(),
                    target_file: RawTarget::Bank { name: "SFX.bank".into() },
                },
                ComponentInfo::RawFile { rel: "raw/script.cache".into(), target_file: RawTarget::ScriptCache },
            ],
        };
        let json = serde_json::to_string_pretty(&meta).unwrap();
        let back: ModEntryMeta = serde_json::from_str(&json).unwrap();
        assert_eq!(meta, back);

        // Lock the wire format: adjacently-known snake_case tags and kind names.
        assert!(json.contains("\"foreign_mixed\""), "kind tag: {json}");
        assert!(json.contains("\"ue4ss_lua\""), "component tag: {json}");
        assert!(json.contains("\"angel_script_patch\""), "component tag: {json}");
        assert!(json.contains("\"loose_pak\""), "component tag: {json}");
        assert!(json.contains("\"raw_file\""), "component tag: {json}");
        assert!(json.contains("\"script_cache\""), "raw target tag: {json}");

        // Defaulted fields may be absent in hand-written metadata.
        let minimal: ModEntryMeta = serde_json::from_str(
            r#"{
                "id": "x",
                "kind": "goremod",
                "name": "X",
                "imported_at": "2026-07-03T12:00:00Z",
                "components": [
                    { "type": "ue4ss_lua", "name": "X", "rel": "ue4ss/X" }
                ]
            }"#,
        )
        .unwrap();
        assert_eq!(minimal.version, "");
        assert_eq!(minimal.author, "");
        assert_eq!(minimal.source, "");
        assert_eq!(
            minimal.components[0],
            ComponentInfo::Ue4ssLua {
                name: "X".into(),
                rel: "ue4ss/X".into(),
                targets: vec![],
                opaque: false
            }
        );
    }
}
