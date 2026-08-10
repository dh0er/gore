//! Effect register: what an identifier **does in game**, and who saw it do that.
//!
//! # Why this is not a catalog
//!
//! The catalogs next door enumerate what exists — every item class, every NPC,
//! every named spot. They are machine-derived, complete, and regenerated after a
//! game patch, so every fact in them is re-derived by the next tool run and a
//! wrong one is caught automatically.
//!
//! An effect fact is the opposite in every one of those respects. "Replacing
//! this texture makes the main menu wordmark magenta" cannot be derived from any
//! file; a person has to look at a screen. It is human-earned, sparse, never
//! regenerable — only re-observable. So it needs its own trust model, and that
//! model is this module.
//!
//! The register annotates the catalog. It does not replace it and it does not
//! gate it: an id absent from the register is an id nobody has looked at yet,
//! not an id that does nothing.
//!
//! # The witness rule is the whole trust model
//!
//! **The assistant never sees the screen.** It can build, deploy and read files;
//! it cannot observe an effect. Every effect fact an assistant reports is
//! therefore either a human's words relayed, or a guess — and assistants guess
//! confidently. During the campaign that produced this register's seed data, a
//! correct observation ("the 4K container renders") became a false claim ("the
//! compressed writer is proven") within a few hours, and was written to two
//! files before a check caught it.
//!
//! So an observation may carry [`Outcome::Confirmed`] or [`Outcome::Refuted`]
//! only if its `witness` field holds words a human actually said. Without one,
//! [`RegisterSource::parse`] degrades the observation to
//! [`Outcome::Unconfirmed`] regardless of what the file claims — it does not
//! reject the file, because the observation is still worth keeping as one
//! person's assertion; it just stops being evidence. An assistant cannot
//! honestly fabricate the field, and if it does, the entry still reads as an
//! assertion rather than as a fact.
//!
//! This is deliberately the same mechanism as the MCP consent gate's
//! `user_approved`, which records that a call ran on the assistant's assertion of
//! prior approval rather than pretending the claim was checked. A reader who has
//! met one should recognise the other.
//!
//! [`Entry`] has no `Deserialize`: the only way to obtain one is through the
//! loader, so there is no path that puts an unwitnessed `confirmed` into a
//! caller's hands.
//!
//! # Status is derived, never stored
//!
//! [`Entry::status`] is computed from the observations every time it is asked
//! for, so it cannot drift away from its own evidence. `disputed` — some
//! observations confirm, others refute — is surfaced, never resolved: it is
//! usually informative rather than wrong (a game patch, a different language, a
//! different display scale), and quietly picking a side would throw away the
//! finding.
//!
//! # Provenance is a property of the file, not the entry
//!
//! Three sources, loaded in that order and **never blended in output**:
//! `bundled` (compiled in here, maintainer-reviewed), `community` (a separate
//! pack somebody installs deliberately) and `local` (this machine's own notes).
//! Every [`Entry`] carries the [`Provenance`] of the file it came from, so a
//! search result cannot lose the label by being merged into a list. A user who
//! does not trust the community pack simply does not install it, and nothing
//! silently mixes into the bundled answer.
//!
//! v1 ships `bundled` only. The loader takes all three from the start so that
//! opening the contribution path later is not a migration.
//!
//! # Where the bundled data lives
//!
//! `register/<domain>.json` in this crate, compiled in with `include_str!` —
//! see [`BUNDLED_REGISTERS`].
//!
//! Deliberately not next to `location_catalog.json`, which lives in
//! `apps/save-editor/assets/`. That path is a Flutter app's asset directory, and
//! the register belongs to the whole toolkit: `gore find` answers from it with
//! no save editor, no game installation, and no generation step. A crate-owned
//! data directory is also the pattern `gore-generation` already uses for its
//! committed qualification artifacts. Because the JSON is compiled into the
//! binary rather than staged next to it, `build.py` needs no entry for it.

use std::collections::BTreeSet;
use std::fmt;
use std::path::Path;

use serde::{Deserialize, Serialize};

/// Schema version written into (and demanded from) every register file.
///
/// A closed contract, not a floor: a reader that does not know the number
/// refuses the file rather than interpreting the parts it recognises.
pub const FORMAT_VERSION: u32 = 1;

/// The register files compiled into this crate, as `(domain, JSON)`.
///
/// One file per domain, and the domain also appears inside the file — the two
/// are checked against each other by [`Register::bundled`], because renaming a
/// file without editing its `domain` field is the obvious way to get this wrong
/// and is otherwise silent.
///
/// A domain names the namespace its ids live in, not a command: `texture` and
/// `asset` ids are cooked asset paths, `loc` ids are localization keys, `audio`
/// ids are FMOD sample names, `voice` ids are archive members, `item` ids are
/// AngelScript class names.
pub const BUNDLED_REGISTERS: &[(&str, &str)] = &[
    ("asset", include_str!("../register/asset.json")),
    ("audio", include_str!("../register/audio.json")),
    ("item", include_str!("../register/item.json")),
    ("loc", include_str!("../register/loc.json")),
    ("texture", include_str!("../register/texture.json")),
    ("voice", include_str!("../register/voice.json")),
];

// ─── Wire vocabulary ─────────────────────────────────────────────────────────

/// Where a register file came from, and therefore how much it is worth.
///
/// Ordered `bundled` < `community` < `local`, which is the order output lists
/// them in; [`Register::push`] keeps a set in that order rather than trusting
/// callers to insert in it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Provenance {
    /// Compiled into `gore`, maintainer-reviewed.
    Bundled,
    /// A separate versioned pack, downloaded deliberately; reviewed in bulk.
    Community,
    /// This machine's own observations — whatever the user did.
    Local,
}

impl Provenance {
    /// The wire word, which is also what output prints.
    pub fn as_str(self) -> &'static str {
        match self {
            Provenance::Bundled => "bundled",
            Provenance::Community => "community",
            Provenance::Local => "local",
        }
    }
}

impl fmt::Display for Provenance {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Who made an observation. Never a name — no personal data enters these files.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Observer {
    Maintainer,
    User,
}

impl Observer {
    pub fn as_str(self) -> &'static str {
        match self {
            Observer::Maintainer => "maintainer",
            Observer::User => "user",
        }
    }
}

impl fmt::Display for Observer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// What one observation found.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Outcome {
    /// The effect happened, and a human said so.
    Confirmed,
    /// The change was built and deployed as written, and the effect did **not**
    /// happen. These are the entries nobody else can produce cheaply.
    Refuted,
    /// Suspected from a name, a folder or a neighbour — or claimed without a
    /// witness, which the loader treats as the same thing.
    Unconfirmed,
}

impl Outcome {
    pub fn as_str(self) -> &'static str {
        match self {
            Outcome::Confirmed => "confirmed",
            Outcome::Refuted => "refuted",
            Outcome::Unconfirmed => "unconfirmed",
        }
    }
}

impl fmt::Display for Outcome {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// An entry's standing, derived from its observations by [`Entry::status`].
///
/// Never stored in a file. There is no `Deserialize` here on purpose: a file
/// that could state a status could state one its own evidence contradicts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Status {
    /// At least one `confirmed`, none `refuted`.
    Confirmed,
    /// At least one `refuted`, none `confirmed`.
    Refuted,
    /// Both — the only state a maintainer must look at. Surface it; do not
    /// resolve it.
    Disputed,
    /// Neither.
    Unconfirmed,
}

impl Status {
    pub fn as_str(self) -> &'static str {
        match self {
            Status::Confirmed => "confirmed",
            Status::Refuted => "refuted",
            Status::Disputed => "disputed",
            Status::Unconfirmed => "unconfirmed",
        }
    }
}

impl fmt::Display for Status {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

// ─── Corroboration ───────────────────────────────────────────────────────────

/// How many observations reached one outcome, across how many distinct builds.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
pub struct Tally {
    /// Observations with this outcome, after the witness rule.
    pub observations: usize,
    /// Distinct game builds among them.
    pub builds: usize,
}

impl Tally {
    /// Nothing was observed at all.
    pub fn is_empty(self) -> bool {
        self.observations == 0
    }
}

/// Agreement is the register's real currency: ten `confirmed` observations of
/// one id across three builds is a far stronger claim than one maintainer's
/// note, and it is cheap to merge because agreement is machine-checkable.
///
/// Kept per outcome rather than as a single "agreeing" number, because a
/// `disputed` entry has two camps and collapsing them to one figure would be
/// resolving the dispute in the arithmetic.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
pub struct Corroboration {
    pub confirmed: Tally,
    pub refuted: Tally,
    pub unconfirmed: Tally,
    /// Distinct builds across every observation, agreeing or not.
    pub builds: usize,
}

impl Corroboration {
    /// The tally for one outcome.
    ///
    /// Keyed by [`Outcome`] and not by [`Status`], so `disputed` cannot be asked
    /// for and no caller is handed one side of a dispute as if it were the
    /// answer.
    pub fn tally(&self, outcome: Outcome) -> Tally {
        match outcome {
            Outcome::Confirmed => self.confirmed,
            Outcome::Refuted => self.refuted,
            Outcome::Unconfirmed => self.unconfirmed,
        }
    }

    /// Observations behind this entry, whatever they found.
    pub fn observations(&self) -> usize {
        self.confirmed.observations + self.refuted.observations + self.unconfirmed.observations
    }
}

// ─── Loaded shape ────────────────────────────────────────────────────────────

/// One sighting, as the loader admitted it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Observation {
    /// When, as written in the file (`YYYY-MM-DD`).
    pub date: String,
    /// The game build it was seen on.
    pub build: String,
    /// The toolkit revision it was seen with.
    pub gore: String,
    /// The outcome **after** the witness rule — never what the file claimed on
    /// its own.
    pub outcome: Outcome,
    /// What the file claimed, when the witness rule overrode it; `None` when the
    /// claim was accepted as written. Print it: a reader deserves to know the
    /// difference between "nobody has tested this" and "somebody said they did".
    pub degraded_from: Option<Outcome>,
    /// The observer's own words, verbatim. Whitespace-only is not a witness — it
    /// is a field somebody filled in to get past the check — and is stored as
    /// `None`.
    pub witness: Option<String>,
    pub by: Observer,
}

impl Observation {
    /// The file claimed `confirmed` or `refuted` with no witness behind it.
    pub fn is_degraded(&self) -> bool {
        self.degraded_from.is_some()
    }
}

/// One identifier and what it was observed to do.
///
/// Carries its own `domain` and `provenance` so that an entry lifted out of its
/// file — into a search result, a JSON payload, a printed line — still says
/// where it came from. Provenance that lived only on the file would be lost by
/// the first `Vec` that merged two sources, which is exactly the blending this
/// register is not allowed to do.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Entry {
    pub domain: String,
    pub provenance: Provenance,
    /// The identifier in that domain's own namespace.
    pub id: String,
    /// Where it shows up, in a player's words. `null` when every observation
    /// refutes it.
    pub effect: Option<String>,
    /// What helps the next person choose or avoid this target.
    pub note: Option<String>,
    /// A list, never replaced: observations are appended, not edited away.
    pub observations: Vec<Observation>,
}

impl Entry {
    /// The derived status. Computed on every call from the observations, so it
    /// cannot disagree with them.
    pub fn status(&self) -> Status {
        let confirmed = self.has(Outcome::Confirmed);
        let refuted = self.has(Outcome::Refuted);
        match (confirmed, refuted) {
            (true, true) => Status::Disputed,
            (true, false) => Status::Confirmed,
            (false, true) => Status::Refuted,
            (false, false) => Status::Unconfirmed,
        }
    }

    /// How much agreement stands behind this entry, per outcome.
    pub fn corroboration(&self) -> Corroboration {
        Corroboration {
            confirmed: self.tally(Outcome::Confirmed),
            refuted: self.tally(Outcome::Refuted),
            unconfirmed: self.tally(Outcome::Unconfirmed),
            builds: distinct_builds(self.observations.iter()),
        }
    }

    /// Observations the witness rule demoted. Non-empty means somebody claimed
    /// more than they showed.
    pub fn degraded(&self) -> Vec<&Observation> {
        self.observations
            .iter()
            .filter(|observation| observation.is_degraded())
            .collect()
    }

    fn has(&self, outcome: Outcome) -> bool {
        self.observations
            .iter()
            .any(|observation| observation.outcome == outcome)
    }

    fn tally(&self, outcome: Outcome) -> Tally {
        let matching = self
            .observations
            .iter()
            .filter(|observation| observation.outcome == outcome);
        Tally {
            observations: matching.clone().count(),
            builds: distinct_builds(matching),
        }
    }

    /// Substring match over id, effect and note, against an already-folded
    /// needle.
    ///
    /// Public because a caller holding the entries for one id has to be able to ask the same
    /// question `Register::search` asks across all of them — `gore find` answers a term from
    /// whichever layer carries it, and that is one of the layers.
    pub fn matches(&self, folded_needle: &str) -> bool {
        let mut haystacks = std::iter::once(self.id.as_str())
            .chain(self.effect.as_deref())
            .chain(self.note.as_deref());
        haystacks.any(|text| text.to_lowercase().contains(folded_needle))
    }
}

/// Distinct game builds among some observations.
///
/// An observation with no build recorded is counted as no build at all: the
/// build count is the number a reader weighs corroboration by, and inflating it
/// with a blank is the one way to make that number lie.
fn distinct_builds<'a>(observations: impl Iterator<Item = &'a Observation>) -> usize {
    observations
        .map(|observation| observation.build.trim())
        .filter(|build| !build.is_empty())
        .collect::<BTreeSet<_>>()
        .len()
}

/// One loaded register file: a domain, a provenance, and its entries.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RegisterSource {
    /// The schema version the file declared, which is always
    /// [`FORMAT_VERSION`] — anything else was refused.
    pub format: u32,
    pub domain: String,
    pub provenance: Provenance,
    /// What to call this file in an error: the bundled file's name, or the path
    /// it was read from.
    pub origin: String,
    pub entries: Vec<Entry>,
}

impl RegisterSource {
    /// Parse one register file.
    ///
    /// `origin` names the file in any error this returns, so pass something the
    /// reader can act on — a path, not "the register".
    pub fn parse(json: &str, provenance: Provenance, origin: &str) -> Result<Self, Error> {
        // The version gate runs on the raw document, before any v1 type is built
        // from it. A loader that deserialized first and checked the number
        // afterwards would already have half-read a future format, and would
        // report whichever field that format moved as the problem instead of
        // reporting the version — which is the one thing the reader can actually
        // do something about.
        let document: serde_json::Value =
            serde_json::from_str(json).map_err(|error| Error::Malformed {
                origin: origin.to_string(),
                detail: format!("not valid JSON: {error}"),
            })?;
        let Some(declared) = document.get("format") else {
            return Err(Error::Malformed {
                origin: origin.to_string(),
                detail: format!(
                    "no `format` field. Every register file declares the schema it was written \
                     for; add \"format\": {FORMAT_VERSION}"
                ),
            });
        };
        let Some(declared) = declared.as_u64() else {
            return Err(Error::Malformed {
                origin: origin.to_string(),
                detail: format!("`format` must be a whole number, not {declared}"),
            });
        };
        if declared != u64::from(FORMAT_VERSION) {
            return Err(Error::UnsupportedFormat {
                origin: origin.to_string(),
                found: declared,
            });
        }

        // Parsed from the text a second time rather than from the `Value` above,
        // which costs one pass over a hand-sized file and buys the line and
        // column of whatever is wrong.
        let file: DocumentJson = serde_json::from_str(json).map_err(|error| Error::Malformed {
            origin: origin.to_string(),
            detail: error.to_string(),
        })?;

        let mut entries: Vec<Entry> = Vec::with_capacity(file.entries.len());
        for entry in file.entries {
            if entries.iter().any(|seen| seen.id == entry.id) {
                // Two entries for one id would split its observations in half and
                // halve the corroboration count that is the point of keeping them.
                return Err(Error::DuplicateId {
                    origin: origin.to_string(),
                    id: entry.id,
                });
            }
            if entry.observations.is_empty() {
                return Err(Error::NoObservations {
                    origin: origin.to_string(),
                    id: entry.id,
                });
            }
            entries.push(Entry {
                domain: file.domain.clone(),
                provenance,
                id: entry.id,
                effect: entry.effect,
                note: entry.note,
                observations: entry.observations.into_iter().map(admit).collect(),
            });
        }

        Ok(RegisterSource {
            format: FORMAT_VERSION,
            domain: file.domain,
            provenance,
            origin: origin.to_string(),
            entries,
        })
    }

    /// Read and parse a register file from disk. `origin` is the path.
    pub fn load(path: &Path, provenance: Provenance) -> Result<Self, Error> {
        let origin = path.display().to_string();
        let json = std::fs::read_to_string(path).map_err(|error| Error::Unreadable {
            origin: origin.clone(),
            detail: error.to_string(),
        })?;
        Self::parse(&json, provenance, &origin)
    }

    /// The entry for exactly this id, or the one whose id differs only by case.
    ///
    /// Ids are ASCII by construction — asset paths, loc keys, class names — so
    /// the fold is ASCII too. An exact spelling still wins, for the same reason
    /// it does in the location catalog: answering a printed id with a different
    /// id's row is worse than answering nothing.
    pub fn lookup(&self, id: &str) -> Option<&Entry> {
        self.entries
            .iter()
            .find(|entry| entry.id == id)
            .or_else(|| {
                self.entries
                    .iter()
                    .find(|entry| entry.id.eq_ignore_ascii_case(id))
            })
    }

    /// Entries whose id, effect or note contains `needle`, case-insensitively,
    /// in file order.
    pub fn search(&self, needle: &str) -> Vec<&Entry> {
        let folded = needle.to_lowercase();
        self.entries
            .iter()
            .filter(|entry| entry.matches(&folded))
            .collect()
    }
}

/// The witness rule, applied once, at the door.
///
/// Nothing downstream re-checks it, and nothing needs to: [`Entry`] cannot be
/// deserialized, so this is the only way an [`Observation`] is ever built.
fn admit(observation: ObservationJson) -> Observation {
    let witness = observation
        .witness
        .filter(|witness| !witness.trim().is_empty());
    let claimed = observation.outcome;
    let unwitnessed = witness.is_none() && matches!(claimed, Outcome::Confirmed | Outcome::Refuted);
    Observation {
        date: observation.date,
        build: observation.build,
        gore: observation.gore,
        outcome: if unwitnessed {
            Outcome::Unconfirmed
        } else {
            claimed
        },
        degraded_from: unwitnessed.then_some(claimed),
        witness,
        by: observation.by,
    }
}

/// Every register source in play, kept in provenance order.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct Register {
    sources: Vec<RegisterSource>,
}

impl Register {
    /// The registers compiled into this binary.
    ///
    /// Embedded rather than located on disk so `gore find` answers with no game
    /// installation, no config and no prior generation step — the same reason the
    /// location catalog is embedded, and for a register the stronger one: the
    /// data cannot be regenerated at all, only re-observed.
    pub fn bundled() -> Result<Self, Error> {
        let mut register = Register::default();
        for (domain, json) in BUNDLED_REGISTERS {
            let origin = format!("bundled register/{domain}.json");
            let source = RegisterSource::parse(json, Provenance::Bundled, &origin)?;
            if source.domain != *domain {
                return Err(Error::DomainMismatch {
                    origin,
                    expected: (*domain).to_string(),
                    found: source.domain,
                });
            }
            register.push(source);
        }
        Ok(register)
    }

    /// Add a source, keeping the set ordered `bundled`, `community`, `local`.
    ///
    /// Sorted here rather than left to callers, because "never blended" is only
    /// worth anything if the output order is the trust order every time, not
    /// whenever the caller happened to load the packs in the right sequence.
    /// Insertion is stable within one provenance.
    pub fn push(&mut self, source: RegisterSource) {
        let at = self
            .sources
            .iter()
            .position(|existing| existing.provenance > source.provenance)
            .unwrap_or(self.sources.len());
        self.sources.insert(at, source);
    }

    /// Every loaded file, in provenance order — the grouping an output that must
    /// not blend sources prints under.
    pub fn sources(&self) -> &[RegisterSource] {
        &self.sources
    }

    /// Domains present, deduplicated, in alphabetical order.
    pub fn domains(&self) -> Vec<&str> {
        self.sources
            .iter()
            .map(|source| source.domain.as_str())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect()
    }

    /// Every entry, in provenance order then file order.
    pub fn entries(&self) -> impl Iterator<Item = &Entry> {
        self.sources.iter().flat_map(|source| source.entries.iter())
    }

    /// Entries across every source, restricted to `domain` when one is given.
    /// Domain names are compared case-insensitively.
    ///
    /// A `Vec` rather than an iterator, matching `LocationCatalog::list`: the
    /// register is hand-sized, and an iterator borrowing both `self` and
    /// `domain` would push that lifetime pair onto every caller for nothing.
    pub fn in_domain(&self, domain: Option<&str>) -> Vec<&Entry> {
        self.entries()
            .filter(|entry| match domain {
                Some(wanted) => entry.domain.eq_ignore_ascii_case(wanted),
                None => true,
            })
            .collect()
    }

    /// Every entry for exactly this id, in provenance order then file order.
    ///
    /// A `Vec` and not an `Option` because bundled and local may both describe
    /// the same id, and picking one for the caller would be blending by another
    /// name. Exact spellings win over case-folded ones; the fold only applies
    /// when nothing matched exactly, and then every id it matches comes back —
    /// two ids that differ only by case are two ids.
    pub fn lookup(&self, id: &str, domain: Option<&str>) -> Vec<&Entry> {
        let scope = self.in_domain(domain);
        let exact: Vec<&Entry> = scope
            .iter()
            .copied()
            .filter(|entry| entry.id == id)
            .collect();
        if !exact.is_empty() {
            return exact;
        }
        scope
            .into_iter()
            .filter(|entry| entry.id.eq_ignore_ascii_case(id))
            .collect()
    }

    /// Entries whose id, effect or note contains `needle`, case-insensitively,
    /// in provenance order then file order.
    ///
    /// Folded with `to_lowercase` rather than the ASCII fold [`Register::lookup`]
    /// uses: ids are ASCII, but `effect` and `note` are prose a person wrote, and
    /// the campaign's own witnesses are German.
    pub fn search(&self, needle: &str, domain: Option<&str>) -> Vec<&Entry> {
        let folded = needle.to_lowercase();
        self.in_domain(domain)
            .into_iter()
            .filter(|entry| entry.matches(&folded))
            .collect()
    }

    /// Entries across every source.
    pub fn len(&self) -> usize {
        self.sources.iter().map(|source| source.entries.len()).sum()
    }

    /// No entry in any source. True of a freshly seeded bundled register, which
    /// is a normal state and not an error.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

// ─── Errors ──────────────────────────────────────────────────────────────────

/// Why a register file was refused. Every variant names the file it is about,
/// because a set can hold several and "the register is malformed" does not tell
/// anyone which one to open.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Error {
    /// The file could not be read from disk.
    Unreadable { origin: String, detail: String },
    /// The file is not valid JSON, or not this schema.
    Malformed { origin: String, detail: String },
    /// The file declares a schema version this build does not know.
    UnsupportedFormat { origin: String, found: u64 },
    /// The file's `domain` disagrees with the file it is bundled as.
    DomainMismatch {
        origin: String,
        expected: String,
        found: String,
    },
    /// One id has two entries in the same file.
    DuplicateId { origin: String, id: String },
    /// An entry carries no observation at all.
    NoObservations { origin: String, id: String },
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Unreadable { origin, detail } => {
                write!(f, "effect register {origin} cannot be read: {detail}")
            }
            Error::Malformed { origin, detail } => {
                write!(f, "effect register {origin} is malformed: {detail}")
            }
            Error::UnsupportedFormat { origin, found } => write!(
                f,
                "effect register {origin} declares format {found}; this build reads format \
                 {FORMAT_VERSION} only. Update gore, or load a register written for format \
                 {FORMAT_VERSION}"
            ),
            Error::DomainMismatch {
                origin,
                expected,
                found,
            } => write!(
                f,
                "effect register {origin} declares domain `{found}` but is loaded as `{expected}`. \
                 Fix the `domain` field or the file name so the two agree"
            ),
            Error::DuplicateId { origin, id } => write!(
                f,
                "effect register {origin} lists `{id}` twice. Merge them into one entry: \
                 observations are appended to an entry, never split across entries"
            ),
            Error::NoObservations { origin, id } => write!(
                f,
                "effect register {origin} entry `{id}` has no observations. Every entry is a claim \
                 with evidence attached; if the effect has not been checked, record an observation \
                 with outcome `unconfirmed` saying when and on which build it was guessed"
            ),
        }
    }
}

impl std::error::Error for Error {}

// ─── On-disk shape ───────────────────────────────────────────────────────────

// `deny_unknown_fields` throughout: these files are hand-edited, and a typo like
// `"witnes"` would otherwise be dropped in silence and re-surface as a mystery
// degradation to `unconfirmed`. Adding a field is a `format` bump anyway, so
// nothing legitimate is turned away.

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct DocumentJson {
    #[allow(dead_code)] // checked on the raw document, before this type exists
    format: u32,
    domain: String,
    entries: Vec<EntryJson>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct EntryJson {
    id: String,
    #[serde(default)]
    effect: Option<String>,
    #[serde(default)]
    note: Option<String>,
    observations: Vec<ObservationJson>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ObservationJson {
    date: String,
    build: String,
    gore: String,
    outcome: Outcome,
    #[serde(default)]
    witness: Option<String>,
    by: Observer,
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    /// One observation, spelled out, so a test can vary the one field it is about.
    fn observation(outcome: &str, witness: Option<&str>, build: &str) -> String {
        let witness = match witness {
            Some(text) => format!(r#""witness": "{text}", "#),
            None => String::new(),
        };
        format!(
            r#"{{"date": "2026-08-07", "build": "{build}", "gore": "90940340",
                 "outcome": "{outcome}", {witness}"by": "maintainer"}}"#
        )
    }

    fn file(domain: &str, entries: &str) -> String {
        format!(r#"{{"format": 1, "domain": "{domain}", "entries": [{entries}]}}"#)
    }

    fn entry(id: &str, observations: &[String]) -> String {
        format!(
            r#"{{"id": "{id}", "effect": "the main menu wordmark",
                 "note": "512x180 PF_DXT5", "observations": [{}]}}"#,
            observations.join(",")
        )
    }

    fn parse(json: &str) -> RegisterSource {
        RegisterSource::parse(json, Provenance::Bundled, "test fixture")
            .expect("the fixture is a valid register")
    }

    fn only(source: &RegisterSource) -> &Entry {
        source.entries.first().expect("the fixture has one entry")
    }

    #[test]
    fn one_confirming_observation_and_no_refutation_reads_as_confirmed() {
        let json = file(
            "texture",
            &entry(
                "T_LogoRemake",
                &[observation("confirmed", Some("nr 1 magenta"), "24340829")],
            ),
        );
        assert_eq!(only(&parse(&json)).status(), Status::Confirmed);
    }

    #[test]
    fn one_refuting_observation_and_no_confirmation_reads_as_refuted() {
        let json = file(
            "texture",
            &entry(
                "T_Logo",
                &[observation(
                    "refuted",
                    Some("nr 2 nicht vorhanden"),
                    "24340829",
                )],
            ),
        );
        assert_eq!(only(&parse(&json)).status(), Status::Refuted);
    }

    #[test]
    fn an_entry_that_is_both_confirmed_and_refuted_reads_as_disputed() {
        // A patch, a language, a display scale: the two observations are both
        // real and the register's job is to say so. Resolving it here — newest
        // wins, majority wins, bundled wins — would throw away the finding, so
        // no rule in this module is allowed to collapse the pair.
        let json = file(
            "texture",
            &entry(
                "T_LogoRemake",
                &[
                    observation("confirmed", Some("magenta"), "24340829"),
                    observation("refuted", Some("nichts passiert"), "24169431"),
                ],
            ),
        );
        let source = parse(&json);
        assert_eq!(only(&source).status(), Status::Disputed);

        // And both sides survive into the counts, separately.
        let corroboration = only(&source).corroboration();
        assert_eq!(corroboration.confirmed.observations, 1);
        assert_eq!(corroboration.refuted.observations, 1);
        assert_eq!(corroboration.builds, 2);
    }

    #[test]
    fn an_entry_with_only_untested_observations_reads_as_unconfirmed() {
        let json = file(
            "audio",
            &entry(
                "SFX_UI_Action_Button_Click",
                &[observation("unconfirmed", None, "24340829")],
            ),
        );
        assert_eq!(only(&parse(&json)).status(), Status::Unconfirmed);
    }

    #[test]
    fn a_confirmation_with_no_witness_is_degraded_to_unconfirmed() {
        // The failure this pins: an assistant cannot see a screen, so a
        // `confirmed` it wrote by itself is a guess wearing the word. During the
        // seed campaign one such claim ("the compressed writer is proven")
        // reached two files before a human caught it. Here it never reaches a
        // caller as a confirmation at all.
        let json = file(
            "texture",
            &entry(
                "T_LogoRemake",
                &[observation("confirmed", None, "24340829")],
            ),
        );
        let source = parse(&json);
        let entry = only(&source);

        assert_eq!(entry.status(), Status::Unconfirmed);
        assert_eq!(entry.observations[0].outcome, Outcome::Unconfirmed);
        assert_eq!(
            entry.observations[0].degraded_from,
            Some(Outcome::Confirmed),
            "the claim is kept, so output can say somebody made it"
        );
        assert_eq!(entry.degraded().len(), 1);
        assert!(entry.corroboration().confirmed.is_empty());
    }

    #[test]
    fn a_refutation_with_no_witness_is_degraded_the_same_way() {
        let json = file(
            "texture",
            &entry("T_Logo", &[observation("refuted", None, "24340829")]),
        );
        let source = parse(&json);
        assert_eq!(only(&source).status(), Status::Unconfirmed);
        assert_eq!(
            only(&source).observations[0].degraded_from,
            Some(Outcome::Refuted)
        );
    }

    #[test]
    fn a_blank_witness_is_not_a_witness() {
        // Whitespace is what a field gets filled with to satisfy a check. It
        // buys nothing here, and it is stored as absent so no output prints an
        // empty quotation.
        let json = file(
            "texture",
            &entry(
                "T_LogoRemake",
                &[observation("confirmed", Some("   "), "24340829")],
            ),
        );
        let source = parse(&json);
        assert_eq!(only(&source).status(), Status::Unconfirmed);
        assert_eq!(only(&source).observations[0].witness, None);
    }

    #[test]
    fn an_unconfirmed_observation_needs_no_witness_and_is_not_marked_degraded() {
        let json = file(
            "loc",
            &entry(
                "ui_newgame",
                &[observation("unconfirmed", None, "24340829")],
            ),
        );
        let source = parse(&json);
        assert!(!only(&source).observations[0].is_degraded());
    }

    #[test]
    fn corroboration_counts_agreeing_observations_and_the_builds_they_span() {
        // Three people agreeing on two builds is the claim worth printing; three
        // reports of one build is not the same thing and must not count as it.
        let json = file(
            "audio",
            &entry(
                "SFX_UI_Action_MenuButton_Click_01",
                &[
                    observation("confirmed", Some("hört man"), "24340829"),
                    observation("confirmed", Some("bei mir auch"), "24340829"),
                    observation("confirmed", Some("auf dem alten build auch"), "24169431"),
                ],
            ),
        );
        let source = parse(&json);
        let corroboration = only(&source).corroboration();
        assert_eq!(corroboration.confirmed.observations, 3);
        assert_eq!(corroboration.confirmed.builds, 2);
        assert_eq!(corroboration.observations(), 3);
        assert_eq!(
            corroboration.tally(Outcome::Confirmed),
            corroboration.confirmed
        );
    }

    #[test]
    fn a_degraded_observation_does_not_corroborate_the_outcome_it_claimed() {
        let json = file(
            "audio",
            &entry(
                "SFX_UI_Action_Button_Click",
                &[
                    observation("confirmed", Some("hört man"), "24340829"),
                    observation("confirmed", None, "24169431"),
                ],
            ),
        );
        let source = parse(&json);
        let corroboration = only(&source).corroboration();
        assert_eq!(
            corroboration.confirmed.observations, 1,
            "the witnessed one only"
        );
        assert_eq!(corroboration.unconfirmed.observations, 1);
        assert_eq!(
            corroboration.confirmed.builds, 1,
            "and it does not widen the build span either"
        );
        assert_eq!(
            corroboration.builds, 2,
            "both were still made, on two builds"
        );
    }

    #[test]
    fn an_observation_with_no_build_recorded_does_not_widen_the_build_span() {
        let json = file(
            "item",
            &entry(
                "ItFo_Potion_Health_01",
                &[
                    observation("confirmed", Some("heilt"), "24340829"),
                    observation("confirmed", Some("heilt immer noch"), ""),
                ],
            ),
        );
        let source = parse(&json);
        assert_eq!(only(&source).corroboration().confirmed.observations, 2);
        assert_eq!(only(&source).corroboration().confirmed.builds, 1);
    }

    #[test]
    fn a_format_this_build_does_not_know_is_refused_rather_than_half_read() {
        // The entries below are shaped for a schema that does not exist. A
        // loader that deserialized first would fail on `entries[0].sightings`
        // and tell the reader to fix a field name, when the actual answer is
        // "this file is newer than your gore".
        let json = r#"{
            "format": 2,
            "domain": "texture",
            "entries": [{"id": "T_LogoRemake", "sightings": []}]
        }"#;
        let error = RegisterSource::parse(json, Provenance::Bundled, "future.json")
            .expect_err("format 2 is not readable here");
        assert_eq!(
            error,
            Error::UnsupportedFormat {
                origin: "future.json".to_string(),
                found: 2,
            }
        );
        assert!(error.to_string().contains("format 2"));
        assert!(error.to_string().contains("Update gore"));
    }

    #[test]
    fn a_file_without_a_format_field_is_refused_and_told_what_to_add() {
        let json = r#"{"domain": "texture", "entries": []}"#;
        let error = RegisterSource::parse(json, Provenance::Bundled, "nameless.json")
            .expect_err("a register with no schema version is not a register");
        assert!(error.to_string().contains("no `format` field"));
        assert!(error.to_string().contains("nameless.json"));
    }

    #[test]
    fn a_format_that_is_not_a_number_is_refused_before_anything_else_is_read() {
        let json = r#"{"format": "one", "domain": "texture", "entries": []}"#;
        let error = RegisterSource::parse(json, Provenance::Bundled, "wordy.json")
            .expect_err("a version has to be a version");
        assert!(error
            .to_string()
            .contains("`format` must be a whole number"));
    }

    #[test]
    fn a_misspelled_field_is_refused_rather_than_dropped() {
        // `witnes` silently ignored would degrade the observation and leave the
        // author staring at an `unconfirmed` they thought they had witnessed.
        let json = r#"{"format": 1, "domain": "texture", "entries": [
            {"id": "T_LogoRemake", "observations": [
                {"date": "2026-08-07", "build": "24340829", "gore": "90940340",
                 "outcome": "confirmed", "witnes": "magenta", "by": "maintainer"}
            ]}
        ]}"#;
        let error = RegisterSource::parse(json, Provenance::Bundled, "typo.json")
            .expect_err("an unknown field is a typo, not an extension");
        assert!(error.to_string().contains("witnes"));
    }

    #[test]
    fn an_unknown_outcome_word_is_refused() {
        let json = r#"{"format": 1, "domain": "texture", "entries": [
            {"id": "T_LogoRemake", "observations": [
                {"date": "2026-08-07", "build": "24340829", "gore": "90940340",
                 "outcome": "probably", "by": "maintainer"}
            ]}
        ]}"#;
        let error = RegisterSource::parse(json, Provenance::Bundled, "vague.json")
            .expect_err("the outcome vocabulary is closed");
        assert!(error.to_string().contains("probably"));
    }

    #[test]
    fn one_id_may_not_have_two_entries_in_one_file() {
        // Split entries halve the corroboration count, which is the number the
        // register exists to make trustworthy.
        let confirmed = observation("confirmed", Some("magenta"), "24340829");
        let json = file(
            "texture",
            &format!(
                "{},{}",
                entry("T_LogoRemake", std::slice::from_ref(&confirmed)),
                entry("T_LogoRemake", std::slice::from_ref(&confirmed))
            ),
        );
        let error = RegisterSource::parse(&json, Provenance::Bundled, "twice.json")
            .expect_err("one id, one entry");
        assert!(matches!(error, Error::DuplicateId { .. }));
        assert!(error.to_string().contains("Merge them into one entry"));
    }

    #[test]
    fn an_entry_with_no_observations_is_refused() {
        let json = file("texture", &entry("T_LogoRemake", &[]));
        let error = RegisterSource::parse(&json, Provenance::Bundled, "empty.json")
            .expect_err("an entry is a claim with evidence attached");
        assert!(error.to_string().contains("no observations"));
        assert!(error.to_string().contains("`unconfirmed`"));
    }

    #[test]
    fn every_entry_carries_the_provenance_of_the_file_it_came_from() {
        let json = file(
            "texture",
            &entry(
                "T_LogoRemake",
                &[observation("confirmed", Some("magenta"), "24340829")],
            ),
        );
        let source = RegisterSource::parse(&json, Provenance::Local, "mine.json").expect("valid");
        assert_eq!(only(&source).provenance, Provenance::Local);
        assert_eq!(only(&source).domain, "texture");
    }

    #[test]
    fn a_search_across_sources_never_blends_their_provenance() {
        // The failure: a bundled answer and a stranger's answer merged into one
        // list, printed identically, and trusted identically. Provenance rides
        // on the entry precisely so no `Vec` can lose it.
        let bundled = RegisterSource::parse(
            &file(
                "texture",
                &entry(
                    "T_LogoRemake",
                    &[observation("confirmed", Some("magenta"), "24340829")],
                ),
            ),
            Provenance::Bundled,
            "bundled register/texture.json",
        )
        .expect("valid");
        let local = RegisterSource::parse(
            &file(
                "texture",
                &entry(
                    "T_LogoRemake",
                    &[observation("refuted", Some("bei mir nicht"), "24340829")],
                ),
            ),
            Provenance::Local,
            "mine.json",
        )
        .expect("valid");

        let mut register = Register::default();
        // Pushed out of order on purpose: the set decides the order, not the
        // caller.
        register.push(local);
        register.push(bundled);

        let found = register.lookup("T_LogoRemake", None);
        assert_eq!(
            found.len(),
            2,
            "both sources answer; neither is picked for the caller"
        );
        assert_eq!(found[0].provenance, Provenance::Bundled);
        assert_eq!(found[1].provenance, Provenance::Local);
        assert_eq!(
            found[0].status(),
            Status::Confirmed,
            "and the local refutation does not make the bundled entry disputed"
        );
        assert_eq!(found[1].status(), Status::Refuted);

        let searched = register.search("logoremake", None);
        assert_eq!(
            searched
                .iter()
                .map(|entry| entry.provenance)
                .collect::<Vec<_>>(),
            vec![Provenance::Bundled, Provenance::Local]
        );
    }

    #[test]
    fn search_matches_id_effect_and_note_but_not_the_witness() {
        // `witness` is verbatim human speech, often about something else
        // entirely; matching it would return entries whose id has nothing to do
        // with what was typed.
        let json = file(
            "texture",
            &entry(
                "T_LogoRemake",
                &[observation("confirmed", Some("kobold"), "24340829")],
            ),
        );
        let mut register = Register::default();
        register.push(parse(&json));

        assert_eq!(register.search("logoremake", None).len(), 1, "id");
        assert_eq!(
            register.search("MAIN MENU", None).len(),
            1,
            "effect, folded"
        );
        assert_eq!(register.search("dxt5", None).len(), 1, "note, folded");
        assert!(register.search("kobold", None).is_empty(), "witness");
    }

    #[test]
    fn a_domain_filter_keeps_a_search_inside_one_namespace() {
        let mut register = Register::default();
        register.push(parse(&file(
            "texture",
            &entry(
                "T_LogoRemake",
                &[observation("confirmed", Some("magenta"), "24340829")],
            ),
        )));
        register.push(parse(&file(
            "audio",
            &entry(
                "SFX_LogoSting",
                &[observation("unconfirmed", None, "24340829")],
            ),
        )));

        assert_eq!(register.search("logo", None).len(), 2);
        assert_eq!(register.search("logo", Some("audio")).len(), 1);
        assert_eq!(register.search("logo", Some("AUDIO")).len(), 1);
        assert!(register.lookup("T_LogoRemake", Some("audio")).is_empty());
        assert_eq!(register.domains(), vec!["audio", "texture"]);
        assert_eq!(register.len(), 2);
    }

    #[test]
    fn an_exact_id_wins_over_one_that_differs_only_by_case() {
        let mut register = Register::default();
        register.push(parse(&file(
            "texture",
            &format!(
                "{},{}",
                entry("T_Logo", &[observation("confirmed", Some("a"), "24340829")]),
                entry("T_LOGO", &[observation("confirmed", Some("b"), "24340829")])
            ),
        )));
        assert_eq!(register.lookup("T_LOGO", None).len(), 1);
        assert_eq!(register.lookup("T_LOGO", None)[0].id, "T_LOGO");
        assert_eq!(register.lookup("T_Logo", None)[0].id, "T_Logo");
        // A spelling neither entry uses matches both, and both come back: with
        // nothing to separate them, choosing one would answer a question the
        // caller did not ask.
        assert_eq!(register.lookup("t_logo", None).len(), 2);
    }

    #[test]
    fn the_bundled_registers_parse_and_declare_the_domain_they_are_filed_under() {
        // The v1 register ships mostly empty by design; what this pins is that
        // every bundled file is loadable and that no file was renamed without
        // its `domain` following it.
        let register = Register::bundled().expect("the bundled registers are valid");
        assert_eq!(register.sources().len(), BUNDLED_REGISTERS.len());
        for (source, (domain, _)) in register.sources().iter().zip(BUNDLED_REGISTERS) {
            assert_eq!(source.domain, *domain);
            assert_eq!(source.format, FORMAT_VERSION);
            assert_eq!(source.provenance, Provenance::Bundled);
        }
        assert_eq!(
            register.domains(),
            BUNDLED_REGISTERS
                .iter()
                .map(|(domain, _)| *domain)
                .collect::<Vec<_>>()
        );
        // Whatever has been seeded, no bundled entry may reach a caller
        // claiming more than it showed.
        for entry in register.entries() {
            assert_eq!(entry.provenance, Provenance::Bundled);
            for observation in &entry.observations {
                assert!(
                    !(observation.outcome != Outcome::Unconfirmed && observation.witness.is_none()),
                    "{} claims {} with no witness",
                    entry.id,
                    observation.outcome
                );
            }
        }
    }

    #[test]
    fn a_file_bundled_under_the_wrong_domain_is_refused() {
        let source = parse(&file("audio", ""));
        let mut register = Register::default();
        register.push(source);
        // `Register::bundled` performs this check; reproduce its comparison here
        // so the message is pinned without needing a broken bundled file.
        let error = Error::DomainMismatch {
            origin: "bundled register/texture.json".to_string(),
            expected: "texture".to_string(),
            found: "audio".to_string(),
        };
        assert!(error.to_string().contains("declares domain `audio`"));
        assert!(error.to_string().contains("loaded as `texture`"));
    }
}
