//! Pure conflict analysis: which enabled loadout mods step on the same game-side targets.
//!
//! [`analyze`] folds the enabled mods' component footprints into per-namespace buckets and
//! reports every target claimed by two or more distinct mods (plus one advisory badge per mod
//! carrying opaque UE4SS scripts). It never touches the filesystem — everything comes from
//! library metadata plus the loadout — so callers can re-run it on every reorder/toggle.
//!
//! Ordering contract: [`Conflict::mods`] follows loadout (mount) order, so under the manager's
//! later-wins rule the LAST id is the winner. The report itself is sorted by `(kind, target)`.

use std::collections::BTreeMap;

use serde::Serialize;

use super::loadout::Loadout;
use super::model::{ComponentInfo, ModEntryMeta, RawTarget};

/// One detected overlap: every id in `mods` claims `target`.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Conflict {
    pub kind: ConflictKind,
    pub target: String,
    /// Involved library mod ids in loadout order; the LAST one wins under later-wins.
    pub mods: Vec<String>,
    pub severity: Severity,
}

/// Namespace of a conflict target. Variant order is the report sort order.
#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum ConflictKind {
    /// Localized-string edits, target `"<string id>|<language set>"`.
    Loc,
    /// FMOD sample replacements, target `"<bank>|<sample>"`.
    Audio,
    /// Cooked-asset packages: texture patches, foreign triplets and loose paks share this space.
    Asset,
    /// Class-default-object edits from UE4SS lua, target `"Class.Field"`.
    Cdo,
    /// AngelScript module splices, target = module name.
    ScriptModule,
    /// Wholesale live-file replacement (`"lcache"` / `"bank:<name>"` / `"script_cache"`).
    RawFile,
}

/// How bad an overlap is.
#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    /// Later-wins merge: the earlier mod only loses this one target.
    Soft,
    /// The earlier mod's whole component is clobbered or the two cannot coexist.
    Hard,
    /// Advisory badge (the only severity where a single-mod entry is allowed), not a clash.
    Info,
}

/// Report every target claimed by two or more distinct enabled mods, plus one `Info` badge per
/// enabled mod carrying opaque UE4SS scripts. `mods` is the library in any order; only loadout
/// entries with `enabled == true` participate, in loadout order (which also orders
/// [`Conflict::mods`]). Output is sorted by `(kind, target)` with mod ids deduped.
pub fn analyze(mods: &[&ModEntryMeta], loadout: &Loadout) -> Vec<Conflict> {
    let enabled = enabled_in_order(mods, loadout);
    let mut buckets: BTreeMap<(ConflictKind, String), (Severity, Vec<String>)> = BTreeMap::new();

    for m in &enabled {
        for c in &m.components {
            match c {
                ComponentInfo::LocPatch { targets, .. } => {
                    for t in targets {
                        note(&mut buckets, ConflictKind::Loc, t.clone(), Severity::Soft, &m.id);
                    }
                }
                ComponentInfo::AudioPatch { targets, .. } => {
                    for t in targets {
                        note(&mut buckets, ConflictKind::Audio, t.clone(), Severity::Soft, &m.id);
                    }
                }
                // Texture patches, foreign triplets and loose paks all mount cooked packages,
                // so their footprints live in ONE shared namespace.
                ComponentInfo::TexturePatch { targets, .. }
                | ComponentInfo::Triplet { targets, .. }
                | ComponentInfo::LoosePak { targets, .. } => {
                    for t in targets {
                        note(&mut buckets, ConflictKind::Asset, norm_asset(t), Severity::Soft, &m.id);
                    }
                }
                ComponentInfo::Ue4ssLua { targets, opaque, .. } => {
                    // No dir-name conflict: manager apply deploys each mod to its OWN
                    // `gm{idx:03}_{name}` dir, so two mods sharing a script name never overwrite
                    // each other. Only their CDO targets (Class.Field) can genuinely clash.
                    // Opaque scripts have no trustworthy target list; they get an info badge
                    // below instead of participating in CDO overlap.
                    if !opaque {
                        for t in targets {
                            note(&mut buckets, ConflictKind::Cdo, t.clone(), Severity::Soft, &m.id);
                        }
                    }
                }
                ComponentInfo::AngelScriptPatch { targets, .. } => {
                    for t in targets {
                        note(&mut buckets, ConflictKind::ScriptModule, t.clone(), Severity::Hard, &m.id);
                    }
                }
                // Raw files need cross-matching against patch components; handled below.
                ComponentInfo::RawFile { .. } => {}
            }
        }
    }

    // Raw-file keys: a wholesale replacement clashes with other raw files of the same key AND
    // with any other mod patching that same live file (a raw .lcache steamrolls a loc patch even
    // though no second raw file exists). Patch-only overlaps stay in their own namespaces above,
    // and a mod combining a raw file with its own patches does not conflict with itself.
    let mut raw_targets: Vec<RawTarget> = Vec::new();
    for m in &enabled {
        for c in &m.components {
            if let ComponentInfo::RawFile { target_file, .. } = c {
                if !raw_targets.contains(target_file) {
                    raw_targets.push(target_file.clone());
                }
            }
        }
    }
    for rt in &raw_targets {
        let members: Vec<&str> =
            enabled.iter().filter(|m| touches_raw(m, rt)).map(|m| m.id.as_str()).collect();
        if members.len() >= 2 {
            for id in members {
                note(&mut buckets, ConflictKind::RawFile, raw_key(rt), Severity::Hard, id);
            }
        }
    }

    // One info badge per enabled mod with any opaque UE4SS script: we cannot see what it edits.
    for m in &enabled {
        if m.components.iter().any(|c| matches!(c, ComponentInfo::Ue4ssLua { opaque: true, .. })) {
            note(&mut buckets, ConflictKind::Cdo, format!("{}:opaque", m.id), Severity::Info, &m.id);
        }
    }

    buckets
        .into_iter()
        .filter(|(_, (severity, ids))| ids.len() >= 2 || *severity == Severity::Info)
        .map(|((kind, target), (severity, mods))| Conflict { kind, target, mods, severity })
        .collect()
}

/// Enabled loadout mods resolved against the library: loadout order, deduped by id. Loadout ids
/// without a library entry are skipped — analysis is best-effort, not library validation.
fn enabled_in_order<'a>(mods: &[&'a ModEntryMeta], loadout: &Loadout) -> Vec<&'a ModEntryMeta> {
    let mut out: Vec<&'a ModEntryMeta> = Vec::new();
    for e in &loadout.entries {
        if !e.enabled || out.iter().any(|m| m.id == e.id) {
            continue;
        }
        if let Some(m) = mods.iter().copied().find(|m| m.id == e.id) {
            out.push(m);
        }
    }
    out
}

/// Record `id` as a claimant of `(kind, target)`, deduping ids (preserving first-seen order)
/// and never downgrading an already-recorded severity.
fn note(
    buckets: &mut BTreeMap<(ConflictKind, String), (Severity, Vec<String>)>,
    kind: ConflictKind,
    target: String,
    severity: Severity,
    id: &str,
) {
    let (sev, ids) = buckets.entry((kind, target)).or_insert_with(|| (severity, Vec::new()));
    if rank(severity) > rank(*sev) {
        *sev = severity;
    }
    if !ids.iter().any(|x| x == id) {
        ids.push(id.to_string());
    }
}

fn rank(s: Severity) -> u8 {
    match s {
        Severity::Info => 0,
        Severity::Soft => 1,
        Severity::Hard => 2,
    }
}

/// Asset-namespace normalization: raw target strings, trimmed, with forward slashes — so a
/// texture-patch asset path and a foreign triplet package path CAN collide.
fn norm_asset(t: &str) -> String {
    t.trim().replace('\\', "/")
}

/// Stable bucket key for a raw-file replacement target.
fn raw_key(t: &RawTarget) -> String {
    match t {
        RawTarget::Lcache => "lcache".into(),
        RawTarget::Bank { name } => format!("bank:{name}"),
        RawTarget::ScriptCache => "script_cache".into(),
    }
}

/// Does `m` touch the live file `raw` replaces — either replacing it itself, or patching it
/// (loc patch ↔ lcache, audio patch of the same bank ↔ that bank, AS patch ↔ script cache)?
fn touches_raw(m: &ModEntryMeta, raw: &RawTarget) -> bool {
    m.components.iter().any(|c| match (c, raw) {
        (ComponentInfo::RawFile { target_file, .. }, _) => target_file == raw,
        (ComponentInfo::LocPatch { .. }, RawTarget::Lcache) => true,
        (ComponentInfo::AngelScriptPatch { .. }, RawTarget::ScriptCache) => true,
        (ComponentInfo::AudioPatch { targets, .. }, RawTarget::Bank { name }) => {
            let prefix = format!("{name}|");
            targets.iter().any(|t| t.starts_with(&prefix))
        }
        _ => false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mgr::loadout::LoadoutEntry;
    use crate::mgr::model::ModKind;

    fn meta(id: &str, components: Vec<ComponentInfo>) -> ModEntryMeta {
        ModEntryMeta {
            id: id.into(),
            kind: ModKind::Goremod,
            name: id.into(),
            version: String::new(),
            author: String::new(),
            imported_at: "2026-07-03T00:00:00Z".into(),
            source: String::new(),
            components,
        }
    }

    fn loadout_of(entries: &[(&str, bool)]) -> Loadout {
        Loadout {
            format: 1,
            entries: entries
                .iter()
                .map(|(id, enabled)| LoadoutEntry { id: (*id).into(), enabled: *enabled })
                .collect(),
        }
    }

    fn strs(items: &[&str]) -> Vec<String> {
        items.iter().map(|s| s.to_string()).collect()
    }

    fn loc(targets: &[&str]) -> ComponentInfo {
        ComponentInfo::LocPatch { rel: "loc.json".into(), targets: strs(targets) }
    }
    fn audio(targets: &[&str]) -> ComponentInfo {
        ComponentInfo::AudioPatch { rel: "audio".into(), targets: strs(targets) }
    }
    fn tex(targets: &[&str]) -> ComponentInfo {
        ComponentInfo::TexturePatch { rel: "texture".into(), targets: strs(targets) }
    }
    fn triplet(targets: &[&str]) -> ComponentInfo {
        ComponentInfo::Triplet { rel_base: "paks/zzz_X_P".into(), targets: strs(targets) }
    }
    fn pak(targets: &[&str]) -> ComponentInfo {
        ComponentInfo::LoosePak { rel: "paks/x.pak".into(), targets: strs(targets) }
    }
    fn as_patch(targets: &[&str]) -> ComponentInfo {
        ComponentInfo::AngelScriptPatch { rel: "scripts".into(), targets: strs(targets) }
    }
    fn lua(name: &str, targets: &[&str], opaque: bool) -> ComponentInfo {
        ComponentInfo::Ue4ssLua {
            name: name.into(),
            rel: format!("ue4ss/{name}"),
            targets: strs(targets),
            opaque,
        }
    }
    fn raw(target_file: RawTarget) -> ComponentInfo {
        ComponentInfo::RawFile { rel: "raw/x".into(), target_file }
    }

    fn conflict(kind: ConflictKind, target: &str, mods: &[&str], severity: Severity) -> Conflict {
        Conflict { kind, target: target.into(), mods: strs(mods), severity }
    }

    /// Two enabled mods patching the same loc string overlap softly; ids follow loadout order
    /// (last = winner) regardless of library slice order, a disabled mod with the same target
    /// stays out, and a loadout id without a library entry is ignored.
    #[test]
    fn loc_overlap_soft_last_wins_order() {
        let a = meta("mod-a", vec![loc(&["itfo_cheese|german"])]);
        let b = meta("mod-b", vec![loc(&["itfo_cheese|german"])]);
        let c = meta("mod-c", vec![loc(&["itfo_cheese|german"])]);
        let lo = loadout_of(&[
            ("mod-ghost", true), // not in the library — skipped
            ("mod-a", true),
            ("mod-b", true),
            ("mod-c", false), // disabled — same target, but must not appear
        ]);
        // Library slice deliberately scrambled: loadout order must win.
        let out = analyze(&[&c, &b, &a], &lo);
        assert_eq!(
            out,
            vec![conflict(
                ConflictKind::Loc,
                "itfo_cheese|german",
                &["mod-a", "mod-b"],
                Severity::Soft
            )]
        );
    }

    #[test]
    fn audio_overlap() {
        let a = meta("mod-a", vec![audio(&["SFX.bank|whoosh", "SFX.bank|clang"])]);
        let b = meta("mod-b", vec![audio(&["SFX.bank|whoosh"])]);
        let out = analyze(&[&a, &b], &loadout_of(&[("mod-a", true), ("mod-b", true)]));
        assert_eq!(
            out,
            vec![conflict(
                ConflictKind::Audio,
                "SFX.bank|whoosh",
                &["mod-a", "mod-b"],
                Severity::Soft
            )]
        );
    }

    /// Texture patches, foreign triplets and loose paks share ONE asset namespace, compared as
    /// raw strings after trimming and slash-normalizing.
    #[test]
    fn asset_overlap_triplet_vs_texture_patch() {
        let a = meta("mod-a", vec![tex(&["/Game/UI/T_X"])]);
        let b = meta("mod-b", vec![triplet(&[" \\Game\\UI\\T_X "])]);
        let c = meta("mod-c", vec![pak(&["/Game/UI/T_X"])]);
        let lo = loadout_of(&[("mod-a", true), ("mod-b", true), ("mod-c", true)]);
        let out = analyze(&[&a, &b, &c], &lo);
        assert_eq!(
            out,
            vec![conflict(
                ConflictKind::Asset,
                "/Game/UI/T_X",
                &["mod-a", "mod-b", "mod-c"],
                Severity::Soft
            )]
        );
    }

    #[test]
    fn cdo_overlap_from_targets() {
        let a = meta("mod-a", vec![lua("DirA", &["ADamageData.Health"], false)]);
        let b = meta("mod-b", vec![lua("DirB", &["ADamageData.Health"], false)]);
        let out = analyze(&[&a, &b], &loadout_of(&[("mod-a", true), ("mod-b", true)]));
        assert_eq!(
            out,
            vec![conflict(
                ConflictKind::Cdo,
                "ADamageData.Health",
                &["mod-a", "mod-b"],
                Severity::Soft
            )]
        );
    }

    #[test]
    fn as_same_module_hard() {
        let a = meta("mod-a", vec![as_patch(&["CombatTweaks"])]);
        let b = meta("mod-b", vec![as_patch(&["CombatTweaks", "OtherModule"])]);
        let out = analyze(&[&a, &b], &loadout_of(&[("mod-a", true), ("mod-b", true)]));
        assert_eq!(
            out,
            vec![conflict(
                ConflictKind::ScriptModule,
                "CombatTweaks",
                &["mod-a", "mod-b"],
                Severity::Hard
            )]
        );
    }

    /// A wholesale .lcache replacement clobbers ANY other mod's loc patch — hard conflict even
    /// though no second raw file is present.
    #[test]
    fn rawfile_lcache_vs_loc_patch_hard() {
        let a = meta("mod-a", vec![raw(RawTarget::Lcache)]);
        let b = meta("mod-b", vec![loc(&["itfo_cheese|german"])]);
        let out = analyze(&[&a, &b], &loadout_of(&[("mod-a", true), ("mod-b", true)]));
        assert_eq!(
            out,
            vec![conflict(ConflictKind::RawFile, "lcache", &["mod-a", "mod-b"], Severity::Hard)]
        );
    }

    #[test]
    fn rawfile_vs_rawfile_hard() {
        let a = meta("mod-a", vec![raw(RawTarget::Lcache)]);
        let b = meta("mod-b", vec![raw(RawTarget::Lcache)]);
        let out = analyze(&[&a, &b], &loadout_of(&[("mod-a", true), ("mod-b", true)]));
        assert_eq!(
            out,
            vec![conflict(ConflictKind::RawFile, "lcache", &["mod-a", "mod-b"], Severity::Hard)]
        );
    }

    /// A raw bank replacement only clashes with audio patches of the SAME bank (targets with a
    /// `"<name>|"` prefix), not with patches of other banks.
    #[test]
    fn rawfile_bank_only_conflicts_same_bank_name() {
        let rawm = meta("mod-raw", vec![raw(RawTarget::Bank { name: "SFX.bank".into() })]);
        let hit = meta("mod-hit", vec![audio(&["SFX.bank|whoosh"])]);
        let miss = meta("mod-miss", vec![audio(&["Music.bank|theme"])]);

        let out = analyze(&[&rawm, &hit], &loadout_of(&[("mod-raw", true), ("mod-hit", true)]));
        assert_eq!(
            out,
            vec![conflict(
                ConflictKind::RawFile,
                "bank:SFX.bank",
                &["mod-raw", "mod-hit"],
                Severity::Hard
            )]
        );

        let out = analyze(&[&rawm, &miss], &loadout_of(&[("mod-raw", true), ("mod-miss", true)]));
        assert!(out.is_empty(), "different bank must not conflict: {out:?}");
    }

    /// Two UE4SS mods sharing a script `name` must NOT conflict: apply deploys each to its own
    /// `gm{idx:03}_{name}` dir, so there is no real dir-name clash (regression against a former
    /// false hard conflict).
    #[test]
    fn ue4ss_same_name_no_conflict() {
        let a = meta("mod-a", vec![lua("CoolMod", &[], false)]);
        let b = meta("mod-b", vec![lua("CoolMod", &[], false)]);
        let out = analyze(&[&a, &b], &loadout_of(&[("mod-a", true), ("mod-b", true)]));
        assert!(out.is_empty(), "same ue4ss name must not conflict: {out:?}");
    }

    /// Each enabled mod with any opaque UE4SS script gets exactly ONE single-mod info badge,
    /// and opaque `targets` do NOT count toward CDO overlap.
    #[test]
    fn opaque_ue4ss_emits_info_once_per_mod() {
        let a = meta(
            "mod-a",
            vec![lua("DirA", &["ADamageData.Health"], true), lua("DirA2", &[], true)],
        );
        let b = meta("mod-b", vec![lua("DirB", &["ADamageData.Health"], false)]);
        let out = analyze(&[&a, &b], &loadout_of(&[("mod-a", true), ("mod-b", true)]));
        // No Cdo/"ADamageData.Health" conflict: mod-a's claim is opaque, so only mod-b holds it.
        assert_eq!(
            out,
            vec![conflict(ConflictKind::Cdo, "mod-a:opaque", &["mod-a"], Severity::Info)]
        );
    }

    /// The report is sorted by (kind, target) and independent of library slice order.
    #[test]
    fn deterministic_output_sorted() {
        let a = meta(
            "mod-a",
            vec![loc(&["z_late|de", "a_early|de"]), lua("SharedDir", &[], false), raw(RawTarget::Lcache)],
        );
        let b = meta(
            "mod-b",
            vec![loc(&["a_early|de", "z_late|de"]), lua("SharedDir", &[], false), raw(RawTarget::Lcache)],
        );
        let lo = loadout_of(&[("mod-a", true), ("mod-b", true)]);

        let out = analyze(&[&a, &b], &lo);
        assert_eq!(
            out,
            vec![
                conflict(ConflictKind::Loc, "a_early|de", &["mod-a", "mod-b"], Severity::Soft),
                conflict(ConflictKind::Loc, "z_late|de", &["mod-a", "mod-b"], Severity::Soft),
                conflict(ConflictKind::RawFile, "lcache", &["mod-a", "mod-b"], Severity::Hard),
                // The shared-name `lua("SharedDir", …)` on both mods yields NO conflict (distinct
                // deploy dirs), so it does not appear here.
            ]
        );
        // Library slice order must not matter.
        assert_eq!(analyze(&[&b, &a], &lo), out);
        // And the report really is (kind, target)-sorted.
        let keys: Vec<_> = out.iter().map(|c| (c.kind, c.target.clone())).collect();
        let mut sorted = keys.clone();
        sorted.sort();
        assert_eq!(keys, sorted);
    }

    /// One mod may combine raw replacements with its own patches of the same files (and repeat
    /// its own targets/dir names) without conflicting with itself.
    #[test]
    fn no_selfconflict_single_mod() {
        let a = meta(
            "mod-a",
            vec![
                loc(&["itfo_cheese|german"]),
                loc(&["itfo_cheese|german"]), // same target twice within the same mod
                raw(RawTarget::Lcache),
                audio(&["SFX.bank|whoosh"]),
                raw(RawTarget::Bank { name: "SFX.bank".into() }),
                as_patch(&["CombatTweaks"]),
                raw(RawTarget::ScriptCache),
                tex(&["/Game/UI/T_X"]),
                pak(&["/Game/UI/T_X"]), // same asset twice within the same mod
                lua("DirA", &["ADamageData.Health"], false),
                lua("DirA", &["ADamageData.Health"], false), // same dir name twice
            ],
        );
        let out = analyze(&[&a], &loadout_of(&[("mod-a", true)]));
        assert!(out.is_empty(), "a mod must not conflict with itself: {out:?}");
    }
}
