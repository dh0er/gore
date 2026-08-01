//! What the sealed native-evidence gate concluded, in a shape a person can read.
//!
//! Discovery either has an exact sealed cache/Binds/USMAP tuple or it has nothing, and the "has
//! nothing" branch used to be an `Option::filter`: the reason was dropped at the seam, so the
//! counts downstream arrived with no stated cause and the only warning a user saw named a USMAP
//! path for a refusal that was decided by the script cache. `NativeEvidenceStatus` is that reason,
//! carried out to every surface — it names the observed build, the audited ones, and which of the
//! four gates refused.
//!
//! Nothing here is a seal. Every audited value it quotes is read from the generation table, names
//! included.

use std::fmt;

use sha2::{Digest, Sha256};

use super::default_ancestry::DefaultAncestryError;
use super::default_patch::encode_hex;

/// One audited game generation, as far as `gore as` itself can name one.
///
/// Deliberately no executable digest: `gore as default-sites <cache>` never reads the executable —
/// its argument may be a mini-cache from `gore as extract` — so an exe seal here would be a claim
/// this command cannot make. The published ancestry profile ID stands in for the whole tuple: it is
/// derived from the cache GUID, the combined fingerprint, both Binds digests and both USMAP
/// digests, and it is the identity that already appears in every selector and every receipt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AuditedBuild {
    /// Stable machine key. Compared and printed, never parsed.
    pub id: &'static str,
    /// Banner text for a person.
    pub label: &'static str,
    pub ancestry_profile_id: &'static str,
    pub map_proof_id: &'static str,
}

/// The build in front of the tool. Every field is measured from the bytes on disk; none is sealed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObservedBuild {
    pub script_cache_guid: [u8; 16],
    pub script_cache_len: usize,
    pub script_cache_sha256: [u8; 32],
    /// Absent when the caller held a parsed Binds database but not its bytes.
    pub binds_len: Option<usize>,
    pub binds_sha256: Option<[u8; 32]>,
}

impl ObservedBuild {
    /// Everything a script cache says about itself when no Binds file is in hand.
    pub fn from_script_cache(script_cache_guid: [u8; 16], cache: &[u8]) -> Self {
        Self {
            script_cache_guid,
            script_cache_len: cache.len(),
            script_cache_sha256: Sha256::digest(cache).into(),
            binds_len: None,
            binds_sha256: None,
        }
    }
}

/// One `.usmap` that was examined, and the one phrase that says why it was not accepted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UsmapCandidate {
    pub path: String,
    /// Absent when the file could not be read at all.
    pub sha256: Option<[u8; 32]>,
    pub rejection: &'static str,
}

/// The USMAP behind a qualified verdict.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UsmapProof {
    pub path: String,
    pub sha256: [u8; 32],
}

/// The four numbers a banner quotes. They are `DefaultSiteStats` fields, but the sentence that
/// explains them belongs to the status, so a cause and its numbers cannot drift apart.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EvidenceCounts {
    pub editable_sites: usize,
    pub direct_windows: usize,
    pub unresolved_fields: usize,
    pub unresolved_types: usize,
}

/// Why native field types and native ancestry are, or are not, available for one script cache.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NativeEvidenceStatus {
    /// A sealed generation matched cache GUID, combined fingerprint, Binds and USMAP.
    Qualified {
        generation_id: &'static str,
        generation_label: &'static str,
        ancestry_profile_id: &'static str,
        /// Absent when the tuple was proven without a filesystem search, so no file can be named.
        usmap: Option<UsmapProof>,
    },
    /// The observed script cache is not any audited generation. This is the case behind every
    /// bare "unresolved type(s)" count on a build the toolkit has never seen.
    UnsupportedGeneration {
        observed: ObservedBuild,
        audited: Vec<AuditedBuild>,
    },
    /// No USMAP on disk carried a sealed identity. `generation_id` is known only when some
    /// candidate got far enough to prove the cache first.
    UsmapMissing {
        generation_id: Option<&'static str>,
        examined: Vec<UsmapCandidate>,
    },
    /// More than one candidate qualified; refusing to choose.
    UsmapAmbiguous {
        generation_id: &'static str,
        matched: Vec<String>,
    },
    /// No sealed Binds database for this cache, so there is no native evidence of any kind.
    BindsUnavailable { reason: String },
    /// The generation is audited but a sealed parser-output digest drifted. Never render this as
    /// an unsupported build: it is a toolkit defect or a damaged file, not somebody's install.
    SealDrift {
        generation_id: Option<&'static str>,
        drift: &'static str,
    },
    /// The caller asked for no native evidence at all.
    NotRequested,
}

/// What one USMAP candidate's failure actually means.
///
/// The distinction is the whole point of this module. A fact about the script cache printed under a
/// USMAP path is what made the old warning blame the one input that had not changed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CandidateVerdict {
    /// The script cache is not an audited build. True whichever USMAP is examined next.
    UnsupportedCache,
    /// The cache is audited but its Binds database is not the sealed one. Also candidate-free.
    UnsupportedBinds,
    /// An audited build whose sealed parser output drifted. Only one USMAP can carry the sealed
    /// raw digest, so reaching a drift settles the question for every remaining candidate.
    SealDrift(&'static str),
    /// About this one file, and nothing else.
    Rejected(&'static str),
}

/// Sort one ancestry failure into a verdict about the build or a rejection of one file.
pub fn classify_candidate_failure(error: &DefaultAncestryError) -> CandidateVerdict {
    match error {
        DefaultAncestryError::UnsupportedCache => CandidateVerdict::UnsupportedCache,
        DefaultAncestryError::UnsupportedBinds => CandidateVerdict::UnsupportedBinds,
        DefaultAncestryError::UsmapGraphDrift => {
            CandidateVerdict::SealDrift("its sealed USMAP class graph does not match")
        }
        DefaultAncestryError::ResolvedProfileDrift => {
            CandidateVerdict::SealDrift("its sealed Binds/USMAP class profile does not match")
        }
        DefaultAncestryError::GameplayTagFloat32MapProfileDrift => CandidateVerdict::SealDrift(
            "its sealed GameplayTag-to-float32 map field profile does not match",
        ),
        DefaultAncestryError::ProfileIdDrift => CandidateVerdict::SealDrift(
            "its derived native-ancestry profile ID does not match its sealed ID",
        ),
        DefaultAncestryError::GameplayTagFloat32MapProofIdDrift => CandidateVerdict::SealDrift(
            "its derived GameplayTag-map proof ID does not match its sealed ID",
        ),
        DefaultAncestryError::UnsupportedUsmap => {
            CandidateVerdict::Rejected("sealed for a different build")
        }
        DefaultAncestryError::MissingUsmapIdentity => {
            CandidateVerdict::Rejected("carries no raw source identity")
        }
        DefaultAncestryError::BridgeResolution { .. } => {
            CandidateVerdict::Rejected("a Binds class resolves ambiguously through it")
        }
        DefaultAncestryError::DuplicateSchemaBridge { .. } => {
            CandidateVerdict::Rejected("two Binds class names resolve to one of its schemas")
        }
        DefaultAncestryError::CyclicHierarchy { .. } => {
            CandidateVerdict::Rejected("its class hierarchy contains a cycle")
        }
        DefaultAncestryError::InvalidCache(_) => {
            CandidateVerdict::Rejected("the script cache did not survive re-validation against it")
        }
    }
}

/// Every generation this toolkit has audited.
///
/// This is the seam between the legibility layer and the generation table, and it is the only
/// function here that reads the table. It transcribed two rows by hand while the table was landing;
/// the third generation is what made that a real cost, because a build the table admits but this
/// list does not is a build whose banner prints a digest where its name belongs and whose "audited"
/// list is short by one. The signature has not moved.
pub fn audited_builds() -> Vec<AuditedBuild> {
    gore_generation::rows()
        .iter()
        .map(|row| AuditedBuild {
            id: row.id,
            label: row.label,
            ancestry_profile_id: row.native_ancestry_profile_id,
            map_proof_id: row.gameplay_tag_float32_map_proof_id,
        })
        .collect()
}

/// The audited build one published ancestry profile ID belongs to.
pub fn audited_build_for_profile_id(ancestry_profile_id: &str) -> Option<AuditedBuild> {
    audited_builds()
        .into_iter()
        .find(|build| build.ancestry_profile_id == ancestry_profile_id)
}

impl NativeEvidenceStatus {
    /// The sealed tuple matched. Names the generation from its published ancestry profile ID, so a
    /// caller never has to know both.
    pub fn qualified(ancestry_profile_id: &'static str, usmap: Option<UsmapProof>) -> Self {
        let build = audited_build_for_profile_id(ancestry_profile_id);
        Self::Qualified {
            generation_id: build.map_or(ancestry_profile_id, |build| build.id),
            generation_label: build.map_or(ancestry_profile_id, |build| build.label),
            ancestry_profile_id,
            usmap,
        }
    }

    /// The audited generation's stable id for a published ancestry profile ID, or that ID itself
    /// when it is not one this module names.
    pub fn generation_id_for_profile_id(ancestry_profile_id: &'static str) -> &'static str {
        audited_build_for_profile_id(ancestry_profile_id).map_or(ancestry_profile_id, |b| b.id)
    }

    /// Whether native evidence is available. `Qualified` is the only yes.
    pub fn is_qualified(&self) -> bool {
        matches!(self, Self::Qualified { .. })
    }

    /// The wire discriminant, stable across wordings.
    pub fn kind(&self) -> &'static str {
        match self {
            Self::Qualified { .. } => "qualified",
            Self::UnsupportedGeneration { .. } => "unsupported_generation",
            Self::UsmapMissing { .. } => "usmap_missing",
            Self::UsmapAmbiguous { .. } => "usmap_ambiguous",
            Self::BindsUnavailable { .. } => "binds_unavailable",
            Self::SealDrift { .. } => "seal_drift",
            Self::NotRequested => "not_requested",
        }
    }

    /// The banner a person reads, without a trailing newline. Empty when there is nothing to say.
    ///
    /// `counts` folds the site statistics into the sentence that explains them; pass `None` from a
    /// command that has no site counts, and the cause is still stated.
    pub fn banner(&self, counts: Option<EvidenceCounts>) -> String {
        match self {
            Self::Qualified {
                generation_id,
                ancestry_profile_id,
                usmap,
                ..
            } => match usmap {
                Some(proof) => format!(
                    "sealed native-default ancestry {} (build {generation_id}) from {}",
                    short_id(ancestry_profile_id),
                    proof.path
                ),
                None => format!(
                    "sealed native-default ancestry {} (build {generation_id})",
                    short_id(ancestry_profile_id)
                ),
            },
            Self::UnsupportedGeneration { observed, audited } => {
                unsupported_generation_banner(observed, audited, counts)
            }
            Self::UsmapMissing {
                generation_id,
                examined,
            } => usmap_missing_banner(*generation_id, examined),
            Self::UsmapAmbiguous {
                generation_id,
                matched,
            } => {
                let mut text = format!(
                    "warning: {} USMAP candidates qualified for build {generation_id}; \
                     refusing to choose one.\n",
                    matched.len()
                );
                for (index, path) in matched.iter().enumerate() {
                    let label = if index == 0 { "  matched    " } else { "             " };
                    text.push_str(&format!("{label}  {path}\n"));
                }
                text.push_str(
                    "Native ancestry stays unavailable until exactly one candidate remains. Remove \
                     the duplicates,\nor point GORE_AS_USMAP at the one to use.",
                );
                text
            }
            Self::BindsUnavailable { reason } => {
                format!("native-default ancestry unavailable: {reason}")
            }
            Self::SealDrift {
                generation_id,
                drift,
            } => {
                let subject = match generation_id {
                    Some(id) => format!("build {id} is audited"),
                    None => "this game build is audited".to_owned(),
                };
                format!(
                    "error: {subject}, but {drift}.\nThis is a toolkit defect or a damaged file, \
                     not an unsupported game build. Please report it\nwith the command line above \
                     and the files it names."
                )
            }
            Self::NotRequested => String::new(),
        }
    }
}

fn unsupported_generation_banner(
    observed: &ObservedBuild,
    audited: &[AuditedBuild],
    counts: Option<EvidenceCounts>,
) -> String {
    let mut text = String::from("warning: this game build has not been audited by this toolkit.\n");
    text.push_str(&format!(
        "  your build   script cache  {}\n                             {} bytes, GUID {}\n",
        encode_hex(&observed.script_cache_sha256),
        observed.script_cache_len,
        encode_hex(&observed.script_cache_guid),
    ));
    if let (Some(len), Some(sha256)) = (observed.binds_len, observed.binds_sha256) {
        text.push_str(&format!(
            "               Binds.Cache   {}\n                             {len} bytes\n",
            encode_hex(&sha256)
        ));
    }
    for (index, build) in audited.iter().enumerate() {
        let label = if index == 0 {
            format!("  audited ({})", audited.len())
        } else {
            " ".repeat(13)
        };
        text.push_str(&format!(
            "{label}  {:<20} ancestry {}  {}\n",
            build.id,
            short_id(build.ancestry_profile_id),
            build.label
        ));
    }
    match counts {
        Some(counts) => text.push_str(&format!(
            "Native field types and native ancestry are unavailable for this build: {} window(s)\n\
             have no type witness and {} have no ancestry proof. {} of {} site(s) remain editable.\n",
            counts.unresolved_types,
            counts.unresolved_fields,
            counts.editable_sites,
            counts.direct_windows,
        )),
        None => text
            .push_str("Native field types and native ancestry are unavailable for this build.\n"),
    }
    text.push_str("Your install is fine — this toolkit has not sealed this build yet.");
    text
}

fn usmap_missing_banner(
    generation_id: Option<&'static str>,
    examined: &[UsmapCandidate],
) -> String {
    let mut text = match (generation_id, examined.is_empty()) {
        (Some(id), false) => {
            format!("warning: build {id} is audited, but no matching USMAP was found.\n")
        }
        (Some(id), true) => format!("warning: build {id} is audited, but no USMAP was found.\n"),
        (None, false) => {
            String::from("warning: no USMAP with a sealed identity was found, so this build's \
                          native evidence\nwas never checked.\n")
        }
        (None, true) => String::from(
            "warning: no USMAP was found, so this build's native evidence was never checked.\n",
        ),
    };
    for (index, candidate) in examined.iter().enumerate() {
        let label = if index == 0 {
            format!("  examined ({})", examined.len())
        } else {
            " ".repeat(14)
        };
        let digest = match candidate.sha256 {
            Some(sha256) => short_hex(&sha256),
            None => "—".to_owned(),
        };
        text.push_str(&format!(
            "{label} {}  {digest:<17}  {}\n",
            candidate.path, candidate.rejection
        ));
    }
    text.push_str(
        "The USMAP is a UE4SS dump produced on your machine, not a file the game ships. Re-dump \
         it\nagainst this build to restore native ancestry.",
    );
    text
}

/// `sha256:` plus the first sixteen hex characters. The maintainer already has the whole value; a
/// user reporting a build never needs to retype an audited one.
fn short_id(id: &str) -> String {
    match id.strip_prefix("sha256:") {
        Some(hex) if hex.len() > 16 => format!("sha256:{}…", &hex[..16]),
        _ => id.to_owned(),
    }
}

fn short_hex(digest: &[u8; 32]) -> String {
    format!("{}…", &encode_hex(digest)[..16])
}

impl fmt::Display for NativeEvidenceStatus {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.banner(None))
    }
}
