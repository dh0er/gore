//! Pure conflict analysis: which enabled loadout mods step on the same game-side targets.
//!
//! [`analyze`] folds the enabled mods' component footprints into per-namespace buckets and
//! reports every target claimed by two or more distinct mods. Opaque UE4SS components retain
//! their known targets and also produce a conservative unknown-footprint advisory when another
//! relevant UE4SS mod is enabled. Compatible raw-base + patch layering is likewise an advisory,
//! because neither participant wins the composed file. It never touches the filesystem —
//! everything comes from library metadata plus the loadout — so callers can re-run it on every
//! reorder/toggle.
//!
//! Ordering contract: [`Conflict::mods`] follows loadout (mount) order. For `Soft`/`Hard`
//! conflicts the LAST id is the later-wins winner; `Info` advisories intentionally have no winner.
//! The report itself is sorted by `(kind, target)`.

use std::collections::BTreeMap;

use serde::Serialize;

use super::loadout::Loadout;
use super::model::{ComponentInfo, ModEntryMeta, RawTarget};

/// One detected overlap: every id in `mods` claims `target`.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Conflict {
    pub kind: ConflictKind,
    pub target: String,
    /// Involved library mod ids in loadout order. The LAST one wins for `Soft`/`Hard`; `Info`
    /// advisories have no winner.
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
    /// Cooked-asset packages: texture patches and foreign triplets share this space. Loose paks do
    /// NOT — their targets are game-relative file paths, which is a different namespace entirely.
    Asset,
    /// Class-default-object edits from UE4SS lua, target `"Class.Field"`.
    Cdo,
    /// Possible interaction involving an incomplete UE4SS footprint, target `"<unknown>"`.
    Ue4ssUnknown,
    /// AngelScript module splices, target = module name.
    ScriptModule,
    /// Voice ZIP member edit, target `"<archive>|<member path>"` (case-insensitive later-wins).
    VoiceArchive,
    /// Wholesale live-file replacement, including its base+patch composition advisory
    /// (`"lcache"` / `"bank:<name>"` / `"script_cache"`).
    RawFile,
    /// One game-root-relative file claimed by two mods, target = that path (case-insensitive,
    /// forward slashes). Both routes to it live here: an in-place `files` replacement, and a pak
    /// — this toolkit's `pak_files` or a foreign `_P.pak` — carrying an entry at the same path.
    LooseFile,
}

/// How bad an overlap is.
#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    /// Later-wins merge: the earlier mod only loses this one target.
    Soft,
    /// The earlier mod's whole component is clobbered or the two cannot coexist.
    Hard,
    /// Advisory without a later-wins winner (for example an unknown footprint or compatible
    /// raw-base + patch composition).
    Info,
}

/// Report every target claimed by two or more distinct enabled mods, plus an `Info` advisory when
/// an opaque UE4SS footprint can interact with another relevant UE4SS mod or when a raw base and a
/// compatible patch compose without either mod winning the whole file. `mods` is the library in
/// any order; only loadout entries with `enabled == true` participate, in loadout order (which also
/// orders [`Conflict::mods`]). Output is sorted by `(kind, target)` with mod ids deduped.
pub fn analyze(mods: &[&ModEntryMeta], loadout: &Loadout) -> Vec<Conflict> {
    let enabled = enabled_in_order(mods, loadout);
    let mut buckets: BTreeMap<(ConflictKind, String), (Severity, Vec<String>)> = BTreeMap::new();
    // Loose-file claims are collected separately because their severity is a property of the PAIR,
    // not of either claimant: `note`'s take-the-worse rule cannot express "these two are only soft
    // because both arrive through the pak filesystem".
    let mut loose: BTreeMap<String, LooseClaims> = BTreeMap::new();
    // Audio bank identity follows Windows filename semantics, while sample identity stays exact.
    // Keep it separate from generic buckets so the displayed target can retain the last claimant's
    // real spelling instead of exposing the normalized key.
    let mut audio: BTreeMap<(String, Option<String>), AudioClaims> = BTreeMap::new();

    for m in &enabled {
        for c in &m.components {
            match c {
                ComponentInfo::LocPatch { targets, .. } => {
                    for t in targets {
                        note(
                            &mut buckets,
                            ConflictKind::Loc,
                            t.clone(),
                            Severity::Soft,
                            &m.id,
                        );
                    }
                }
                ComponentInfo::AudioPatch { targets, .. } => {
                    for t in targets {
                        audio
                            .entry(audio_target_identity(t))
                            .or_default()
                            .claim(t, &m.id);
                    }
                }
                // Texture patches and foreign triplets both mount cooked packages, so their
                // footprints live in ONE shared namespace of `/Game/…` package paths.
                ComponentInfo::TexturePatch { targets, .. }
                | ComponentInfo::Triplet { targets, .. } => {
                    for t in targets {
                        note(
                            &mut buckets,
                            ConflictKind::Asset,
                            norm_asset(t),
                            Severity::Soft,
                            &m.id,
                        );
                    }
                }
                ComponentInfo::Ue4ssLua { targets, .. } => {
                    // No dir-name conflict: manager apply deploys each mod to its OWN
                    // `gm{idx:03}_{name}` dir, so two mods sharing a script name never overwrite
                    // each other. Only their CDO targets (Class.Field) can genuinely clash.
                    // `opaque` means incomplete, not unusable: exact generated override targets
                    // remain valid partial evidence and still participate in ordinary CDO
                    // analysis. The unknown remainder is handled conservatively below.
                    for t in targets {
                        note(
                            &mut buckets,
                            ConflictKind::Cdo,
                            t.clone(),
                            Severity::Soft,
                            &m.id,
                        );
                    }
                }
                ComponentInfo::AngelScriptPatch { targets, .. } => {
                    for t in targets {
                        note(
                            &mut buckets,
                            ConflictKind::ScriptModule,
                            t.clone(),
                            Severity::Hard,
                            &m.id,
                        );
                    }
                }
                // Three components claim ONE game-root-relative file path, and they must share one
                // bucket. A pak's `targets` are the paths its entries CLAIM, not the `~mods` path
                // it writes — so the old argument that `loose_target_allowed` keeps the sets
                // disjoint (true of in-place writers) never applied to pak-namespace claimants,
                // and a foreign `_P.pak` fighting a `files` bundle over one file reported nothing.
                ComponentInfo::FilePatch { targets, .. } => {
                    for t in targets {
                        loose.entry(norm_loose(t)).or_default().claim(false, &m.id);
                    }
                }
                ComponentInfo::PakFilePatch { targets, .. }
                | ComponentInfo::LoosePak { targets, .. } => {
                    for t in targets {
                        loose.entry(norm_loose(t)).or_default().claim(true, &m.id);
                    }
                }
                ComponentInfo::VoiceArchivePatch { targets, .. } => {
                    for t in targets {
                        note(
                            &mut buckets,
                            ConflictKind::VoiceArchive,
                            norm_voice(t),
                            Severity::Soft,
                            &m.id,
                        );
                    }
                }
                // Raw files need cross-matching against patch components; handled below.
                ComponentInfo::RawFile { .. } => {}
            }
        }
    }

    // Fold the loose-file claims in with their pair-derived severity. Inserted directly rather than
    // through `note` because a single mod using BOTH routes on one path is not a conflict, and the
    // final filter lets any `Info` bucket through regardless of how many mods are in it.
    for (target, claims) in loose {
        if claims.ids.len() >= 2 {
            let severity = claims.severity();
            buckets.insert((ConflictKind::LooseFile, target), (severity, claims.ids));
        }
    }
    for (_identity, claims) in audio {
        if claims.ids.len() >= 2 {
            buckets.insert(
                (ConflictKind::Audio, claims.target),
                (Severity::Soft, claims.ids),
            );
        }
    }

    // Raw-file keys have two distinct relations which must not be collapsed into one winner chain:
    // raw-vs-raw is a whole-base replacement where the later raw wins, while raw-vs-patch is
    // compatible composition — apply always lays every loc/audio/script patch over the winning raw
    // base, independent of their relative loadout positions. Emit a Hard row containing only raw
    // claimants and a separate Info advisory for cross-mod base+patch composition. Patch-only
    // overlaps stay in their own namespaces above, and one mod composing its own base and patch is
    // not a conflict.
    // Windows resolves bank names case-insensitively. Key by that file identity, but retain the
    // actual spelling from the last raw claimant so the Hard winner row describes its target.
    let mut raw_targets: BTreeMap<String, RawTarget> = BTreeMap::new();
    for m in &enabled {
        for c in &m.components {
            if let ComponentInfo::RawFile { target_file, .. } = c {
                raw_targets.insert(raw_identity(target_file), target_file.clone());
            }
        }
    }
    let mut raw_conflicts = Vec::new();
    for rt in raw_targets.values() {
        let raw_members: Vec<String> = enabled
            .iter()
            .filter(|m| replaces_raw(m, rt))
            .map(|m| m.id.clone())
            .collect();
        if raw_members.len() >= 2 {
            raw_conflicts.push(Conflict {
                kind: ConflictKind::RawFile,
                target: raw_key(rt),
                mods: raw_members.clone(),
                severity: Severity::Hard,
            });
        }

        let patch_members: Vec<String> = enabled
            .iter()
            .filter(|m| patches_raw(m, rt))
            .map(|m| m.id.clone())
            .collect();
        let has_cross_mod_composition = raw_members
            .iter()
            .any(|raw_id| patch_members.iter().any(|patch_id| patch_id != raw_id));
        if has_cross_mod_composition {
            let members = enabled
                .iter()
                .filter(|m| replaces_raw(m, rt) || patches_raw(m, rt))
                .map(|m| m.id.clone())
                .collect();
            raw_conflicts.push(Conflict {
                kind: ConflictKind::RawFile,
                target: raw_key(rt),
                mods: members,
                severity: Severity::Info,
            });
        }
    }

    // An opaque target list is only a known subset. If another mod has either its own opaque
    // script or any precise UE4SS target, those footprints may interact in ways the manager cannot
    // prove from metadata. Aggregate the relevant distinct mods into one deterministic advisory.
    let mut ue4ss_unknown_members = Vec::<&str>::new();
    let mut has_opaque_ue4ss = false;
    for m in &enabled {
        let mut relevant = false;
        for component in &m.components {
            if let ComponentInfo::Ue4ssLua {
                targets, opaque, ..
            } = component
            {
                has_opaque_ue4ss |= *opaque;
                relevant |= *opaque || !targets.is_empty();
            }
        }
        if relevant {
            ue4ss_unknown_members.push(&m.id);
        }
    }
    if has_opaque_ue4ss && ue4ss_unknown_members.len() >= 2 {
        for id in ue4ss_unknown_members {
            note(
                &mut buckets,
                ConflictKind::Ue4ssUnknown,
                "<unknown>".into(),
                Severity::Info,
                id,
            );
        }
    }

    let mut conflicts: Vec<Conflict> = buckets
        .into_iter()
        .filter(|(_, (severity, ids))| ids.len() >= 2 || *severity == Severity::Info)
        .map(|((kind, target), (severity, mods))| Conflict {
            kind,
            target,
            mods,
            severity,
        })
        .collect();
    conflicts.extend(raw_conflicts);
    conflicts.sort_by(|left, right| {
        left.kind
            .cmp(&right.kind)
            .then_with(|| left.target.cmp(&right.target))
            // One target may have both a proven raw-vs-raw winner and a no-winner composition
            // advisory. Keep the actionable row first and make equal-target order explicit.
            .then_with(|| rank(right.severity).cmp(&rank(left.severity)))
            .then_with(|| left.mods.cmp(&right.mods))
    });
    conflicts
}

/// Every mod claiming one game-root-relative file, and by which route each of them claims it.
#[derive(Default)]
struct LooseClaims {
    /// Ids whose pak entry claims this path: this toolkit's `pak_files`, or a foreign `_P.pak`.
    from_pak: Vec<String>,
    /// Ids whose in-place `files` replacement overwrites the bytes on disk.
    in_place: Vec<String>,
    /// Claimant ids in loadout order, first-seen, deduped. A mod reaching this path by BOTH routes
    /// appears once here and in both route lists.
    ids: Vec<String>,
}

#[derive(Default)]
struct AudioClaims {
    /// Most recently encountered effective spelling of `bank|sample`.
    target: String,
    /// Distinct claimant ids in loadout order.
    ids: Vec<String>,
}

impl AudioClaims {
    fn claim(&mut self, target: &str, id: &str) {
        self.target = target.to_string();
        if !self.ids.iter().any(|existing| existing == id) {
            self.ids.push(id.to_string());
        }
    }
}

impl LooseClaims {
    fn claim(&mut self, from_pak: bool, id: &str) {
        let route = if from_pak { &mut self.from_pak } else { &mut self.in_place };
        if !route.iter().any(|x| x == id) {
            route.push(id.to_string());
        }
        if !self.ids.iter().any(|x| x == id) {
            self.ids.push(id.to_string());
        }
    }

    /// Severity is a property of the pairing, not of either claimant:
    ///
    /// * **pak vs pak** — `Soft`. Genuine later-wins by mount order; the loser loses this one
    ///   entry and keeps the rest of its container.
    /// * **in-place vs in-place** — `Hard`. A loose file is replaced whole, so the loser does not
    ///   lose one key the way a loc id does — it loses its entire file.
    /// * **pak vs in-place** — `Info`, and this is the honest answer rather than a cop-out. Deploy
    ///   refuses an in-place write to a path the shipped containers already carry, so a mixed pair
    ///   can only occur at a path NO pak previously had. Whether the engine's file reader prefers a
    ///   newly-introduced mod-pak entry over a physical file at such a path is not established
    ///   here, and `Info` is exactly the "advisory, not a proven later-wins clash" verdict.
    ///
    /// So a bucket reports the strongest pairing it can prove, not the weakest one present. Asking
    /// two booleans instead used to collapse a whole bucket to `Info`: two `files` mods clobbering
    /// each other were printed as "advisory; no winner" the moment any third mod reached the same
    /// path through a pak — while `apply` went on silently picking a winner between them. Dropping
    /// that pak from the loadout made the very same pair `Hard` again, which is the tell that the
    /// verdict was a property of the bucket rather than of any pairing really in it.
    fn severity(&self) -> Severity {
        if self.in_place.len() >= 2 {
            Severity::Hard
        } else if self.from_pak.len() >= 2 {
            Severity::Soft
        } else {
            Severity::Info
        }
    }
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
    let (sev, ids) = buckets
        .entry((kind, target))
        .or_insert_with(|| (severity, Vec::new()));
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

fn norm_voice(t: &str) -> String {
    t.trim().replace('\\', "/").to_lowercase()
}

/// Loose-file namespace normalization. Windows path identity is case-insensitive and pak lookup
/// hashes a lowercased path, so two mods spelling one destination differently — or one of them
/// copying the shipped index's uppercase `Normal.PNG` — are still fighting over one file.
fn norm_loose(t: &str) -> String {
    t.trim().replace('\\', "/").to_lowercase()
}

/// Stable bucket key for a raw-file replacement target.
fn raw_key(t: &RawTarget) -> String {
    match t {
        RawTarget::Lcache => "lcache".into(),
        RawTarget::Bank { name } => format!("bank:{name}"),
        RawTarget::ScriptCache => "script_cache".into(),
    }
}

/// Audio collision identity: Windows-case-insensitive bank filename plus the unchanged sample.
/// A malformed legacy target without `|` retains its former exact-string behavior.
fn audio_target_identity(target: &str) -> (String, Option<String>) {
    match target.split_once('|') {
        Some((bank, sample)) => (crate::windows_file_name_key(bank), Some(sample.to_string())),
        None => (target.to_string(), None),
    }
}

/// Case-insensitive Windows file identity used to join raw-bank claimants and audio patches.
fn raw_identity(t: &RawTarget) -> String {
    match t {
        RawTarget::Lcache => "lcache".into(),
        RawTarget::Bank { name } => format!("bank:{}", crate::windows_file_name_key(name)),
        RawTarget::ScriptCache => "script_cache".into(),
    }
}

fn replaces_raw(m: &ModEntryMeta, raw: &RawTarget) -> bool {
    m.components.iter().any(|component| {
        matches!(component, ComponentInfo::RawFile { target_file, .. }
            if raw_identity(target_file) == raw_identity(raw))
    })
}

/// Does `m` patch the live file whose base `raw` replaces (loc ↔ lcache, audio of the same bank ↔
/// that bank, AngelScript ↔ script cache)?
fn patches_raw(m: &ModEntryMeta, raw: &RawTarget) -> bool {
    m.components.iter().any(|c| match (c, raw) {
        (ComponentInfo::LocPatch { .. }, RawTarget::Lcache) => true,
        (ComponentInfo::AngelScriptPatch { .. }, RawTarget::ScriptCache) => true,
        (ComponentInfo::AudioPatch { targets, .. }, RawTarget::Bank { name }) => {
            let folded_name = crate::windows_file_name_key(name);
            targets.iter().any(|target| {
                target.split_once('|').is_some_and(|(bank, _sample)| {
                    crate::windows_file_name_key(bank) == folded_name
                })
            })
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
                .map(|(id, enabled)| LoadoutEntry {
                    id: (*id).into(),
                    enabled: *enabled,
                })
                .collect(),
        }
    }

    fn strs(items: &[&str]) -> Vec<String> {
        items.iter().map(|s| s.to_string()).collect()
    }

    fn loc(targets: &[&str]) -> ComponentInfo {
        ComponentInfo::LocPatch {
            rel: "loc.json".into(),
            targets: strs(targets),
        }
    }
    fn audio(targets: &[&str]) -> ComponentInfo {
        ComponentInfo::AudioPatch {
            rel: "audio".into(),
            targets: strs(targets),
        }
    }
    fn tex(targets: &[&str]) -> ComponentInfo {
        ComponentInfo::TexturePatch {
            rel: "texture".into(),
            targets: strs(targets),
        }
    }
    fn triplet(targets: &[&str]) -> ComponentInfo {
        ComponentInfo::Triplet {
            rel_base: "paks/zzz_X_P".into(),
            targets: strs(targets),
        }
    }
    fn pak(targets: &[&str]) -> ComponentInfo {
        ComponentInfo::LoosePak {
            rel: "paks/x.pak".into(),
            targets: strs(targets),
        }
    }
    fn as_patch(targets: &[&str]) -> ComponentInfo {
        ComponentInfo::AngelScriptPatch {
            rel: "scripts".into(),
            targets: strs(targets),
        }
    }
    fn voice(targets: &[&str]) -> ComponentInfo {
        ComponentInfo::VoiceArchivePatch {
            rel: "voice".into(),
            targets: strs(targets),
        }
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
        ComponentInfo::RawFile {
            rel: "raw/x".into(),
            target_file,
        }
    }
    fn loose(targets: &[&str]) -> ComponentInfo {
        ComponentInfo::FilePatch {
            rel: "files".into(),
            targets: strs(targets),
        }
    }
    fn pak_files(targets: &[&str]) -> ComponentInfo {
        ComponentInfo::PakFilePatch {
            rel: "pak_files".into(),
            targets: strs(targets),
        }
    }

    /// Two mods replacing one loose game file is a whole-file clobber, not a mergeable later-wins
    /// situation like a loc key: the loser keeps nothing. Windows path identity is
    /// case-insensitive, so the two spellings below name ONE file and must land in one bucket —
    /// reporting them separately would tell the user the two mods do not overlap.
    #[test]
    fn two_mods_replacing_one_loose_file_conflict_hard_across_spellings() {
        let a = meta(
            "mod-a",
            vec![loose(&["G1R/Content/Slate/Cursors/Normal/Normal.PNG"])],
        );
        let b = meta(
            "mod-b",
            vec![loose(&["G1R\\Content\\Slate\\Cursors\\Normal\\normal.png"])],
        );
        let c = meta("mod-c", vec![loose(&["G1R/Content/Movies/Intro.mp4"])]);
        let lo = loadout_of(&[("mod-a", true), ("mod-b", true), ("mod-c", true)]);

        assert_eq!(
            analyze(&[&a, &b, &c], &lo),
            vec![conflict(
                ConflictKind::LooseFile,
                "g1r/content/slate/cursors/normal/normal.png",
                &["mod-a", "mod-b"],
                Severity::Hard
            )],
            "a third mod on a different loose file must not be dragged in"
        );
    }

    /// A third claimant arriving by the other route must not soften what the first two prove.
    ///
    /// Severity used to come from two per-bucket booleans, so any pak claim on the same path turned
    /// a whole-file clobber between two `files` mods into `Info` — printed as "advisory; no winner"
    /// while `apply` went on picking a winner between them. The tell was that dropping the pak made
    /// the very same pair `Hard` again.
    #[test]
    fn a_pak_claimant_does_not_soften_a_clobber_two_files_mods_already_prove() {
        let path = "G1R/Content/Movies/Intro.bk2";
        let a = meta("mod-a", vec![loose(&[path])]);
        let b = meta("mod-b", vec![loose(&[path])]);
        let c = meta("mod-c", vec![pak_files(&[path])]);
        let lo = loadout_of(&[("mod-a", true), ("mod-b", true), ("mod-c", true)]);

        assert_eq!(
            analyze(&[&a, &b, &c], &lo),
            vec![conflict(
                ConflictKind::LooseFile,
                "g1r/content/movies/intro.bk2",
                &["mod-a", "mod-b", "mod-c"],
                Severity::Hard
            )],
            "two in-place claimants are a proven clobber whoever else is in the bucket"
        );

        // One mod reaching the path by both routes does it with only two mods, and is the shape
        // that made the old booleans look reasonable: the bucket had both flags set, but the
        // pairing that matters is still `files` against `files`.
        let both = meta("mod-a", vec![loose(&[path]), pak_files(&[path])]);
        let other = meta("mod-b", vec![loose(&[path])]);
        let lo = loadout_of(&[("mod-a", true), ("mod-b", true)]);
        assert_eq!(
            analyze(&[&both, &other], &lo),
            vec![conflict(
                ConflictKind::LooseFile,
                "g1r/content/movies/intro.bk2",
                &["mod-a", "mod-b"],
                Severity::Hard
            )]
        );

        // And a genuinely mixed pair is still the advisory it was: one claimant per route.
        let pak_only = meta("mod-a", vec![pak_files(&[path])]);
        let files_only = meta("mod-b", vec![loose(&[path])]);
        let lo = loadout_of(&[("mod-a", true), ("mod-b", true)]);
        assert_eq!(
            analyze(&[&pak_only, &files_only], &lo),
            vec![conflict(
                ConflictKind::LooseFile,
                "g1r/content/movies/intro.bk2",
                &["mod-a", "mod-b"],
                Severity::Info
            )]
        );
    }

    /// The pak-side twin of the case above. Two paks claiming one file both mount, so the loser
    /// loses only that entry rather than its whole container — soft, not hard. They still have to
    /// fold into ONE bucket: the shipped index spells the cursor `Normal.PNG` with an uppercase
    /// extension and a mod author will not, and reporting those separately would tell the user two
    /// mods overwriting each other do not overlap.
    #[test]
    fn two_mods_claiming_one_file_from_paks_conflict_soft_across_spellings() {
        let a = meta(
            "mod-a",
            vec![pak(&["G1R/Content/Slate/Cursors/Normal/Normal.PNG"])],
        );
        let b = meta(
            "mod-b",
            vec![pak_files(&[
                "G1R\\Content\\Slate\\Cursors\\Normal\\normal.png",
            ])],
        );
        let c = meta("mod-c", vec![pak(&["G1R/Content/Movies/Intro.bk2"])]);
        let lo = loadout_of(&[("mod-a", true), ("mod-b", true), ("mod-c", true)]);

        assert_eq!(
            analyze(&[&a, &b, &c], &lo),
            vec![conflict(
                ConflictKind::LooseFile,
                "g1r/content/slate/cursors/normal/normal.png",
                &["mod-a", "mod-b"],
                Severity::Soft
            )],
            "a third mod on a different pak entry must not be dragged in"
        );
    }

    /// The hole this bucket was widened to close. A foreign `_P.pak` and a `files` bundle claiming
    /// the same file used to report NOTHING: pak targets went to `Asset` case-preserved, `files`
    /// targets to `LooseFile` lowercased, and the two sets could never meet. They meet now, and the
    /// verdict is `Info` rather than a later-wins claim — deploy refuses an in-place write to any
    /// path the shipped containers already carry, so a mixed pair can only exist where no pak had
    /// the path before, and which side the engine reads there is not established.
    #[test]
    fn a_pak_and_an_in_place_claim_on_one_file_are_reported_as_uncertain() {
        let a = meta("mod-a", vec![pak(&["G1R/Content/Movies/Intro.bk2"])]);
        let b = meta("mod-b", vec![loose(&["G1R\\Content\\Movies\\intro.BK2"])]);
        let out = analyze(&[&a, &b], &loadout_of(&[("mod-a", true), ("mod-b", true)]));
        assert_eq!(
            out,
            vec![conflict(
                ConflictKind::LooseFile,
                "g1r/content/movies/intro.bk2",
                &["mod-a", "mod-b"],
                Severity::Info
            )]
        );
    }

    fn conflict(kind: ConflictKind, target: &str, mods: &[&str], severity: Severity) -> Conflict {
        Conflict {
            kind,
            target: target.into(),
            mods: strs(mods),
            severity,
        }
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
        let b = meta("mod-b", vec![audio(&["sfx.BANK|whoosh"])]);
        for (entries, expected_mods, expected_target) in [
            (
                [("mod-a", true), ("mod-b", true)],
                ["mod-a", "mod-b"],
                "sfx.BANK|whoosh",
            ),
            (
                [("mod-b", true), ("mod-a", true)],
                ["mod-b", "mod-a"],
                "SFX.bank|whoosh",
            ),
        ] {
            assert_eq!(
                analyze(&[&a, &b], &loadout_of(&entries)),
                vec![conflict(
                    ConflictKind::Audio,
                    expected_target,
                    &expected_mods,
                    Severity::Soft
                )]
            );
        }

        let sample_case_variant = meta("mod-c", vec![audio(&["sfx.BANK|WHOOSH"])]);
        assert!(
            analyze(
                &[&a, &sample_case_variant],
                &loadout_of(&[("mod-a", true), ("mod-c", true)])
            )
            .is_empty(),
            "sample identity must retain its existing case-sensitive semantics"
        );
    }

    #[test]
    fn bank_identity_does_not_expand_sharp_s_into_ss() {
        let sharp_audio = meta("audio-sharp", vec![audio(&["Voiceß.bank|shout"])]);
        let ss_audio = meta("audio-ss", vec![audio(&["VoiceSS.bank|shout"])]);
        assert!(
            analyze(
                &[&sharp_audio, &ss_audio],
                &loadout_of(&[("audio-sharp", true), ("audio-ss", true)])
            )
            .is_empty(),
            "distinct audio banks must not be reported as one patch target"
        );

        let sharp_raw = meta(
            "raw-sharp",
            vec![raw(RawTarget::Bank {
                name: "Voiceß.bank".into(),
            })],
        );
        let ss_raw = meta(
            "raw-ss",
            vec![raw(RawTarget::Bank {
                name: "VoiceSS.bank".into(),
            })],
        );
        assert!(
            analyze(
                &[&sharp_raw, &ss_raw],
                &loadout_of(&[("raw-sharp", true), ("raw-ss", true)])
            )
            .is_empty(),
            "distinct raw banks must not be reported as later-wins replacements"
        );
        assert!(
            analyze(
                &[&sharp_raw, &ss_audio],
                &loadout_of(&[("raw-sharp", true), ("audio-ss", true)])
            )
            .is_empty(),
            "a raw bank must compose only with audio patches for the same Windows filename"
        );
    }

    #[test]
    fn voice_overlap_is_case_insensitive_soft_and_loadout_ordered() {
        let a = meta("mod-a", vec![voice(&["German.zip|NPC/Hero/Hello.ogg"])]);
        let b = meta("mod-b", vec![voice(&["german.ZIP|npc\\hero\\HELLO.OGG"])]);
        let out = analyze(&[&b, &a], &loadout_of(&[("mod-a", true), ("mod-b", true)]));
        assert_eq!(
            out,
            vec![conflict(
                ConflictKind::VoiceArchive,
                "german.zip|npc/hero/hello.ogg",
                &["mod-a", "mod-b"],
                Severity::Soft
            )]
        );
    }

    /// Texture patches and foreign triplets share ONE asset namespace of cooked package paths,
    /// compared as raw strings after trimming and slash-normalizing. A loose pak is deliberately
    /// not in it: its targets are game-relative FILE paths, and `norm_asset`'s permissiveness was
    /// the only thing that ever made putting both in one namespace look harmless.
    #[test]
    fn asset_overlap_triplet_vs_texture_patch() {
        let a = meta("mod-a", vec![tex(&["/Game/UI/T_X"])]);
        let b = meta("mod-b", vec![triplet(&[" \\Game\\UI\\T_X "])]);
        let c = meta("mod-c", vec![pak(&["G1R/Content/Movies/Intro.bk2"])]);
        let lo = loadout_of(&[("mod-a", true), ("mod-b", true), ("mod-c", true)]);
        let out = analyze(&[&a, &b, &c], &lo);
        assert_eq!(
            out,
            vec![conflict(
                ConflictKind::Asset,
                "/Game/UI/T_X",
                &["mod-a", "mod-b"],
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

    /// Raw replacements are bases, and compatible patches always compose on top regardless of
    /// which participant is later. Cover every raw-backed patch family in both loadout orders.
    #[test]
    fn rawfile_patch_pairs_are_no_winner_composition_advisories() {
        fn assert_pair(raw_target: RawTarget, patch: ComponentInfo, target: &str) {
            let raw_mod = meta("mod-raw", vec![raw(raw_target)]);
            let patch_mod = meta("mod-patch", vec![patch]);
            for (entries, expected_mods) in [
                (
                    [("mod-raw", true), ("mod-patch", true)],
                    ["mod-raw", "mod-patch"],
                ),
                (
                    [("mod-patch", true), ("mod-raw", true)],
                    ["mod-patch", "mod-raw"],
                ),
            ] {
                assert_eq!(
                    analyze(&[&raw_mod, &patch_mod], &loadout_of(&entries)),
                    vec![conflict(
                        ConflictKind::RawFile,
                        target,
                        &expected_mods,
                        Severity::Info,
                    )]
                );
            }
        }

        assert_pair(RawTarget::Lcache, loc(&["itfo_cheese|german"]), "lcache");
        assert_pair(
            RawTarget::Bank {
                name: "SFX.bank".into(),
            },
            audio(&["sfx.BANK|whoosh"]),
            "bank:SFX.bank",
        );
        assert_pair(
            RawTarget::ScriptCache,
            as_patch(&["CombatTweaks"]),
            "script_cache",
        );
    }

    #[test]
    fn rawfile_vs_rawfile_hard() {
        let a = meta("mod-a", vec![raw(RawTarget::Lcache)]);
        let b = meta("mod-b", vec![raw(RawTarget::Lcache)]);
        for (entries, expected_mods) in [
            ([("mod-a", true), ("mod-b", true)], ["mod-a", "mod-b"]),
            ([("mod-b", true), ("mod-a", true)], ["mod-b", "mod-a"]),
        ] {
            assert_eq!(
                analyze(&[&a, &b], &loadout_of(&entries)),
                vec![conflict(
                    ConflictKind::RawFile,
                    "lcache",
                    &expected_mods,
                    Severity::Hard,
                )]
            );
        }
    }

    #[test]
    fn rawfile_bank_case_variants_share_windows_identity_and_keep_winner_spelling() {
        let a = meta(
            "mod-a",
            vec![raw(RawTarget::Bank {
                name: "Voice.bank".into(),
            })],
        );
        let b = meta(
            "mod-b",
            vec![raw(RawTarget::Bank {
                name: "voice.BANK".into(),
            })],
        );

        for (entries, expected_mods, expected_target) in [
            (
                [("mod-a", true), ("mod-b", true)],
                ["mod-a", "mod-b"],
                "bank:voice.BANK",
            ),
            (
                [("mod-b", true), ("mod-a", true)],
                ["mod-b", "mod-a"],
                "bank:Voice.bank",
            ),
        ] {
            assert_eq!(
                analyze(&[&a, &b], &loadout_of(&entries)),
                vec![conflict(
                    ConflictKind::RawFile,
                    expected_target,
                    &expected_mods,
                    Severity::Hard,
                )]
            );
        }
    }

    /// A raw bank only composes with audio patches of the SAME bank (`"<name>|"` prefix).
    #[test]
    fn rawfile_bank_ignores_other_bank_patch() {
        let rawm = meta(
            "mod-raw",
            vec![raw(RawTarget::Bank {
                name: "SFX.bank".into(),
            })],
        );
        let miss = meta("mod-miss", vec![audio(&["Music.bank|theme"])]);

        let out = analyze(
            &[&rawm, &miss],
            &loadout_of(&[("mod-raw", true), ("mod-miss", true)]),
        );
        assert!(out.is_empty(), "different bank must not conflict: {out:?}");
    }

    #[test]
    fn rawfile_raw_winner_stays_separate_from_patch_advisory() {
        let raw_a = meta("raw-a", vec![raw(RawTarget::Lcache)]);
        let patch = meta("patch", vec![loc(&["itfo_cheese|german"])]);
        let raw_b = meta("raw-b", vec![raw(RawTarget::Lcache)]);

        assert_eq!(
            analyze(
                &[&raw_a, &patch, &raw_b],
                &loadout_of(&[("raw-a", true), ("patch", true), ("raw-b", true)]),
            ),
            vec![
                conflict(
                    ConflictKind::RawFile,
                    "lcache",
                    &["raw-a", "raw-b"],
                    Severity::Hard,
                ),
                conflict(
                    ConflictKind::RawFile,
                    "lcache",
                    &["raw-a", "patch", "raw-b"],
                    Severity::Info,
                ),
            ]
        );
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

    /// An opaque component keeps its known targets in precise CDO analysis and also contributes
    /// its unknown remainder to one loadout-ordered advisory with other relevant UE4SS mods.
    #[test]
    fn opaque_ue4ss_retains_precise_overlap_and_emits_unknown_interaction() {
        let a = meta(
            "mod-a",
            vec![
                lua("DirA", &["ADamageData.Health"], true),
                lua("DirA2", &[], true),
            ],
        );
        let b = meta("mod-b", vec![lua("DirB", &["ADamageData.Health"], false)]);
        let out = analyze(&[&a, &b], &loadout_of(&[("mod-a", true), ("mod-b", true)]));
        assert_eq!(
            out,
            vec![
                conflict(
                    ConflictKind::Cdo,
                    "ADamageData.Health",
                    &["mod-a", "mod-b"],
                    Severity::Soft
                ),
                conflict(
                    ConflictKind::Ue4ssUnknown,
                    "<unknown>",
                    &["mod-a", "mod-b"],
                    Severity::Info
                )
            ]
        );
    }

    #[test]
    fn opaque_ue4ss_unknown_is_conservative_deduped_and_deterministic() {
        let a = meta(
            "mod-a",
            vec![
                lua("OpaqueA", &["ADamageData.Health"], true),
                lua("OpaqueA2", &[], true),
            ],
        );
        let b = meta("mod-b", vec![lua("PreciseB", &["AItem.Value"], false)]);
        let empty = meta("mod-empty", vec![lua("KnownEmpty", &[], false)]);
        let disabled = meta("mod-disabled", vec![lua("OpaqueDisabled", &[], true)]);
        let loadout = loadout_of(&[
            ("mod-empty", true),
            ("mod-a", true),
            ("mod-disabled", false),
            ("mod-b", true),
        ]);
        let expected = vec![conflict(
            ConflictKind::Ue4ssUnknown,
            "<unknown>",
            &["mod-a", "mod-b"],
            Severity::Info,
        )];
        assert_eq!(analyze(&[&b, &disabled, &empty, &a], &loadout), expected);
        assert_eq!(analyze(&[&a, &empty, &b, &disabled], &loadout), expected);

        let single = analyze(&[&a], &loadout_of(&[("mod-a", true)]));
        assert!(
            single.is_empty(),
            "one opaque mod cannot conflict: {single:?}"
        );

        let opaque_empty_a = meta("opaque-a", vec![lua("OpaqueEmptyA", &[], true)]);
        let opaque_empty_b = meta("opaque-b", vec![lua("OpaqueEmptyB", &[], true)]);
        assert_eq!(
            analyze(
                &[&opaque_empty_b, &opaque_empty_a],
                &loadout_of(&[("opaque-a", true), ("opaque-b", true)]),
            ),
            vec![conflict(
                ConflictKind::Ue4ssUnknown,
                "<unknown>",
                &["opaque-a", "opaque-b"],
                Severity::Info,
            )]
        );
    }

    /// The report is sorted by (kind, target) and independent of library slice order.
    #[test]
    fn deterministic_output_sorted() {
        let a = meta(
            "mod-a",
            vec![
                loc(&["z_late|de", "a_early|de"]),
                lua("SharedDir", &[], false),
                raw(RawTarget::Lcache),
            ],
        );
        let b = meta(
            "mod-b",
            vec![
                loc(&["a_early|de", "z_late|de"]),
                lua("SharedDir", &[], false),
                raw(RawTarget::Lcache),
            ],
        );
        let lo = loadout_of(&[("mod-a", true), ("mod-b", true)]);

        let out = analyze(&[&a, &b], &lo);
        assert_eq!(
            out,
            vec![
                conflict(
                    ConflictKind::Loc,
                    "a_early|de",
                    &["mod-a", "mod-b"],
                    Severity::Soft
                ),
                conflict(
                    ConflictKind::Loc,
                    "z_late|de",
                    &["mod-a", "mod-b"],
                    Severity::Soft
                ),
                conflict(
                    ConflictKind::RawFile,
                    "lcache",
                    &["mod-a", "mod-b"],
                    Severity::Hard
                ),
                conflict(
                    ConflictKind::RawFile,
                    "lcache",
                    &["mod-a", "mod-b"],
                    Severity::Info
                ),
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
                raw(RawTarget::Bank {
                    name: "SFX.bank".into(),
                }),
                as_patch(&["CombatTweaks"]),
                raw(RawTarget::ScriptCache),
                tex(&["/Game/UI/T_X"]),
                triplet(&["/Game/UI/T_X"]), // same asset twice within the same mod
                loose(&["G1R/Content/Slate/Cursors/Normal/Normal.PNG"]),
                loose(&["g1r/content/slate/cursors/normal/normal.png"]), // and once more, folded
                // One mod may reach one file by BOTH routes — two sections, two components, said
                // twice on purpose. A mixed pairing is `Info`, and `Info` is the one severity the
                // final filter emits with a single claimant, so this also pins that it does not.
                pak_files(&["G1R/Content/Slate/Cursors/Normal/NORMAL.png"]),
                lua("DirA", &["ADamageData.Health"], false),
                lua("DirA", &["ADamageData.Health"], false), // same dir name twice
            ],
        );
        let out = analyze(&[&a], &loadout_of(&[("mod-a", true)]));
        assert!(
            out.is_empty(),
            "a mod must not conflict with itself: {out:?}"
        );
    }
}
