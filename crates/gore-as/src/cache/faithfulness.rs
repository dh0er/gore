//! Where the decompiler is known to produce a DIFFERENT PROGRAM, per module.
//!
//! Splicing a module recompiles ALL of it from the emitted source, not only the function that was
//! edited. So a module carrying a function the decompiler does not reproduce hands that function's
//! difference to the game too, in code the author never touched. Whether that matters depends on
//! the difference: most are a different spelling of the same program, but some are a different
//! program.
//!
//! The table is measured, not inferred — one whole-tree emit, recompile and per-function bytecode
//! diff against the shipped cache, for one exact build. It is therefore keyed by the generation
//! that was measured: a cache this table has nothing to say about gets no claim, not a guess.
//!
//! What it lists is the functions the oracle calls SEMANTIC-DIFF. It says nothing about raw bytes:
//! the diff normalises reference keys, jump absolutes, constant encodings and slot numbers away
//! first, so a function it passes may still assemble to different bytes and run the same program.
//! Absence from this table therefore means "no known semantic difference", which is the property
//! a modder needs — not "byte-identical", which would be a stronger claim than was measured.

/// What is known about one module.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ModuleFaithfulness {
    /// Functions whose recompiled bytecode is not provably the same PROGRAM as the shipped one.
    pub divergent_functions: usize,
    /// Of those, the ones known to be a different PROGRAM rather than a different spelling.
    /// Today this counts loops whose bound came out as a literal zero, so the body never runs.
    pub behaviour_risks: usize,
}

/// One measured table per generation, keyed by the generation row's id.
const TABLES: &[(&str, &str)] = &[(
    "g1r-steam-24878692",
    include_str!("../../assets/byte-faithfulness/g1r-steam-24878692.tsv"),
)];

/// The measurement for a pair of inputs, found by what those files CONTAIN.
///
/// Not by the header GUID. A spliced cache keeps the GUID of the build it was spliced into — that
/// is the point of splicing — so the GUID would hand the vanilla measurement to a cache whose
/// modules are no longer vanilla, and an edited module would be reported byte-faithful on the
/// strength of a run it was never part of.
///
/// And not by the script cache alone. `Binds.Cache` is the second input: without it the native
/// field table is empty, every native enum field falls back to the bool heuristic, and the emitted
/// tree stops compiling — a run whose inputs differ that much cannot be the run this table
/// records. Both seals have to name the SAME generation row, and a missing or foreign Binds
/// yields no measurement rather than the vanilla one.
fn table_for(cache_sha256: &[u8; 32], binds_sha256: Option<&[u8; 32]>) -> Option<&'static str> {
    let binds_sha256 = binds_sha256?;
    let row = gore_generation::rows().iter().find(|row| {
        &row.shipping_cache.sha256 == cache_sha256 && &row.binds_cache.sha256 == binds_sha256
    })?;
    TABLES
        .iter()
        .find(|(id, _)| *id == row.id)
        .map(|(_, table)| *table)
}

/// The seal of a file held in memory.
pub fn cache_seal(cache: &[u8]) -> [u8; 32] {
    use sha2::{Digest, Sha256};
    Sha256::digest(cache).into()
}

/// True when a measurement exists for this cache at all. Without one, nothing below means
/// "byte-faithful" — it means "not measured", and callers must say so rather than reassure.
pub fn is_measured(cache_sha256: &[u8; 32], binds_sha256: Option<&[u8; 32]>) -> bool {
    table_for(cache_sha256, binds_sha256).is_some()
}

/// What is known about `module`, or `None` where the cache was never measured.
///
/// A module the table does not list came through that run with no semantic difference: the table
/// carries only the modules that did not, so absence is the positive answer and is reported as
/// zero. It is not a claim that the bytes match.
pub fn for_module(
    cache_sha256: &[u8; 32],
    binds_sha256: Option<&[u8; 32]>,
    module: &str,
) -> Option<ModuleFaithfulness> {
    let table = table_for(cache_sha256, binds_sha256)?;
    for line in table.lines() {
        let line = line.trim_end();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let mut columns = line.split('\t');
        let (Some(name), Some(divergent), Some(risks)) =
            (columns.next(), columns.next(), columns.next())
        else {
            continue;
        };
        if name != module {
            continue;
        }
        return Some(ModuleFaithfulness {
            divergent_functions: divergent.parse().unwrap_or(0),
            behaviour_risks: risks.parse().unwrap_or(0),
        });
    }
    Some(ModuleFaithfulness {
        divergent_functions: 0,
        behaviour_risks: 0,
    })
}

/// The line to put in front of someone about to edit and splice `module`, or `None` where there is
/// nothing to warn about — the module is byte-faithful, or the cache was never measured.
///
/// Deliberately says what it means for the reader rather than quoting a percentage: the risk is
/// not that the module fails to compile (it does compile), it is that code the author did not
/// touch comes out different.
pub fn warning_for_module(
    cache_sha256: &[u8; 32],
    binds_sha256: Option<&[u8; 32]>,
    module: &str,
) -> Option<String> {
    let known = for_module(cache_sha256, binds_sha256, module)?;
    if known.divergent_functions == 0 {
        return None;
    }
    let functions = if known.divergent_functions == 1 {
        "1 function".to_owned()
    } else {
        format!("{} functions", known.divergent_functions)
    };
    let mut line = format!(
        "warning: {module} carries {functions} the decompiler does not reproduce as the same \
         program. Splicing this module recompiles all of it, so those come out changed as well"
    );
    if known.behaviour_risks > 0 {
        let loops = if known.behaviour_risks == 1 {
            "1 loop in it recompiles".to_owned()
        } else {
            format!("{} loops in it recompile", known.behaviour_risks)
        };
        line.push_str(&format!(
            ", and {loops} with a bound of zero, so the body never runs. Check that before \
             shipping"
        ));
    }
    line.push('.');
    Some(line)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn measured_binds() -> [u8; 32] {
        gore_generation::rows()
            .iter()
            .find(|row| row.id == "g1r-steam-24878692")
            .expect("the measured generation is in the table")
            .binds_cache
            .sha256
    }

    fn measured_guid() -> [u8; 32] {
        gore_generation::rows()
            .iter()
            .find(|row| row.id == "g1r-steam-24878692")
            .expect("the measured generation is in the table")
            .shipping_cache
            .sha256
    }

    #[test]
    fn an_unmeasured_cache_makes_no_claim() {
        let unknown = [0u8; 32];
        let binds = measured_binds();
        assert!(!is_measured(&unknown, Some(&binds)));
        assert_eq!(for_module(&unknown, Some(&binds), "AI.CharacterAI_Gothic"), None);
        assert_eq!(
            warning_for_module(&unknown, Some(&binds), "AI.CharacterAI_Gothic"),
            None
        );
    }

    #[test]
    fn a_spliced_cache_is_not_the_measured_one() {
        // Splicing preserves the header GUID and changes the content. Keying on content is what
        // makes that visible; keying on the GUID would have handed it the vanilla measurement.
        let measured = measured_guid();
        let binds = measured_binds();
        let mut spliced = measured;
        spliced[0] ^= 0xff;
        assert!(is_measured(&measured, Some(&binds)));
        assert!(!is_measured(&spliced, Some(&binds)));
        assert_eq!(
            warning_for_module(&spliced, Some(&binds), "AI.CharacterAI_Gothic"),
            None
        );
    }

    #[test]
    fn a_missing_or_foreign_binds_makes_no_claim() {
        // Without the matching Binds the emitted tree is prepared from different native metadata
        // and does not even compile, so nothing measured against it carries over.
        let measured = measured_guid();
        let binds = measured_binds();
        let mut foreign = binds;
        foreign[0] ^= 0xff;
        assert!(!is_measured(&measured, None));
        assert!(!is_measured(&measured, Some(&foreign)));
        assert_eq!(
            warning_for_module(&measured, None, "AI.States.FightAI.SearchState.AIState_Search"),
            None
        );
    }

    #[test]
    fn a_module_the_table_omits_was_byte_faithful() {
        let guid = measured_guid();
        let binds = measured_binds();
        assert!(is_measured(&guid, Some(&binds)));
        let known = for_module(&guid, Some(&binds), "no.such.module").expect("measured");
        assert_eq!(known.divergent_functions, 0);
        assert_eq!(warning_for_module(&guid, Some(&binds), "no.such.module"), None);
    }

    #[test]
    fn a_listed_module_reports_its_count() {
        let guid = measured_guid();
        let binds = measured_binds();
        let known = for_module(&guid, Some(&binds), "AI.States.FightAI.SearchState.AIState_Search")
            .expect("measured");
        assert!(known.divergent_functions > 0);
        let warning =
            warning_for_module(&guid, Some(&binds), "AI.States.FightAI.SearchState.AIState_Search")
                .expect("warns");
        assert!(warning.contains("does not reproduce as the same program"));
    }

    #[test]
    fn a_dead_loop_module_says_the_body_never_runs() {
        let guid = measured_guid();
        let binds = measured_binds();
        let module = "AI.AssessmentResponseSystem.CrimeProcessingSubsystem.CreepingEvaluationContext";
        let known = for_module(&guid, Some(&binds), module).expect("measured");
        assert!(known.behaviour_risks > 0);
        assert!(warning_for_module(&guid, Some(&binds), module)
            .expect("warns")
            .contains("never runs"));
    }

    #[test]
    fn the_table_parses_and_is_internally_consistent() {
        let guid = measured_guid();
        let binds = measured_binds();
        let table = table_for(&guid, Some(&binds)).expect("measured");
        let mut listed = 0;
        for line in table.lines() {
            if line.trim_end().is_empty() || line.starts_with('#') {
                continue;
            }
            let columns: Vec<&str> = line.trim_end().split('\t').collect();
            assert_eq!(columns.len(), 3, "row shape: {line}");
            let divergent: usize = columns[1].parse().expect("divergent count");
            let risks: usize = columns[2].parse().expect("risk count");
            assert!(divergent > 0, "a listed module has something to report: {line}");
            assert!(risks <= divergent, "risks are a subset: {line}");
            listed += 1;
        }
        assert!(listed > 0);
    }
}
