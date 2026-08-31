//! What the decompiler is known not to reproduce byte-for-byte, per module.
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

/// What is known about one module.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ModuleFaithfulness {
    /// Functions whose recompiled bytecode is not provably identical to the shipped one.
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

fn table_for(script_cache_guid: &[u8; 16]) -> Option<&'static str> {
    let row = gore_generation::row_for_script_cache_guid(script_cache_guid)?;
    TABLES
        .iter()
        .find(|(id, _)| *id == row.id)
        .map(|(_, table)| *table)
}

/// True when a measurement exists for this cache at all. Without one, nothing below means
/// "byte-faithful" — it means "not measured", and callers must say so rather than reassure.
pub fn is_measured(script_cache_guid: &[u8; 16]) -> bool {
    table_for(script_cache_guid).is_some()
}

/// What is known about `module`, or `None` where the cache was never measured.
///
/// A module the table does not list was byte-faithful in that run: the table carries only the
/// modules that were not, so absence is the positive answer and is reported as zero.
pub fn for_module(script_cache_guid: &[u8; 16], module: &str) -> Option<ModuleFaithfulness> {
    let table = table_for(script_cache_guid)?;
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
pub fn warning_for_module(script_cache_guid: &[u8; 16], module: &str) -> Option<String> {
    let known = for_module(script_cache_guid, module)?;
    if known.divergent_functions == 0 {
        return None;
    }
    let functions = if known.divergent_functions == 1 {
        "1 function".to_owned()
    } else {
        format!("{} functions", known.divergent_functions)
    };
    let mut line = format!(
        "warning: {module} carries {functions} the decompiler does not reproduce byte-for-byte. \
         Splicing this module recompiles all of it, so those come out changed as well"
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

    fn measured_guid() -> [u8; 16] {
        gore_generation::rows()
            .iter()
            .find(|row| row.id == "g1r-steam-24878692")
            .expect("the measured generation is in the table")
            .script_cache_guid
    }

    #[test]
    fn an_unmeasured_cache_makes_no_claim() {
        let unknown = [0u8; 16];
        assert!(!is_measured(&unknown));
        assert_eq!(for_module(&unknown, "AI.CharacterAI_Gothic"), None);
        assert_eq!(warning_for_module(&unknown, "AI.CharacterAI_Gothic"), None);
    }

    #[test]
    fn a_module_the_table_omits_was_byte_faithful() {
        let guid = measured_guid();
        assert!(is_measured(&guid));
        let known = for_module(&guid, "no.such.module").expect("measured");
        assert_eq!(known.divergent_functions, 0);
        assert_eq!(warning_for_module(&guid, "no.such.module"), None);
    }

    #[test]
    fn a_listed_module_reports_its_count() {
        let guid = measured_guid();
        let known =
            for_module(&guid, "AI.States.FightAI.SearchState.AIState_Search").expect("measured");
        assert!(known.divergent_functions > 0);
        let warning =
            warning_for_module(&guid, "AI.States.FightAI.SearchState.AIState_Search").expect("warns");
        assert!(warning.contains("does not reproduce byte-for-byte"));
    }

    #[test]
    fn a_dead_loop_module_says_the_body_never_runs() {
        let guid = measured_guid();
        let module = "AI.AssessmentResponseSystem.CrimeProcessingSubsystem.CreepingEvaluationContext";
        let known = for_module(&guid, module).expect("measured");
        assert!(known.behaviour_risks > 0);
        assert!(warning_for_module(&guid, module)
            .expect("warns")
            .contains("never runs"));
    }

    #[test]
    fn the_table_parses_and_is_internally_consistent() {
        let guid = measured_guid();
        let table = table_for(&guid).expect("measured");
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
