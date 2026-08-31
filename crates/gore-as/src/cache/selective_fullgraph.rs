//! Selectively rebase declared FullGraph changes onto an exact pristine cache.
//!
//! A complete FullGraph rebuild is useful compiler evidence, but publishing it wholesale also
//! publishes unrelated regeneration drift. This module extracts only the declared Add/Edit
//! modules and folds them onto the pristine bytes. A module may refer to a newly added module:
//! remap failures are retried after every successful composition until either all changes apply or
//! the dependency graph reaches a fail-closed fixed point.

use std::collections::{BTreeMap, HashSet};

use thiserror::Error;

use super::generated_defaults::{
    ExistingFunctionMetadataPlan, ExistingModuleStructurePlan, GeneratedDefaultsPlan,
};
use super::remap::{RemapError, RemapOptions};
use super::splice::{
    extract_module, remap_module_to_base_with_options, validate_standalone_script_cache,
    SequentialMiniGuard, SpliceError,
};
use super::walk_modules::{module_names, module_ranges};

const MAX_SELECTIVE_FULLGRAPH_CHANGES: usize = 256;
const MAX_MODULE_NAME_BYTES: usize = 4_096;

/// Compiler-derived preservation state for one existing module.
///
/// `generated_defaults` is present only for the legacy defaults-free source path. Such a module
/// must use strict remap before byte-exact carry; an authored-default edit instead uses new-symbol
/// remap and only restores the existing Unreal function metadata.
#[derive(Clone, Debug)]
pub(crate) struct SelectiveFullGraphEditPreservation {
    metadata: ExistingFunctionMetadataPlan,
    structure: ExistingModuleStructurePlan,
    generated_defaults: Option<GeneratedDefaultsPlan>,
}

impl SelectiveFullGraphEditPreservation {
    pub(crate) fn new(
        metadata: ExistingFunctionMetadataPlan,
        structure: ExistingModuleStructurePlan,
        generated_defaults: Option<GeneratedDefaultsPlan>,
    ) -> Self {
        Self {
            metadata,
            structure,
            generated_defaults,
        }
    }
}

/// One authoritative change from the sealed FullGraph request.
#[derive(Clone, Debug)]
pub(crate) enum SelectiveFullGraphChange {
    Add {
        module_name: String,
    },
    Edit {
        module_name: String,
        preservation: SelectiveFullGraphEditPreservation,
    },
    /// Selective removal needs declaration-tail pruning and retained-reference proof. Keep the
    /// operation representable so callers cannot accidentally turn a requested delete into a
    /// no-op; composition rejects it before inspecting the rebuilt artifact.
    #[allow(dead_code)]
    Delete {
        module_name: String,
    },
}

impl SelectiveFullGraphChange {
    pub(crate) fn add(module_name: impl Into<String>) -> Self {
        Self::Add {
            module_name: module_name.into(),
        }
    }

    pub(crate) fn edit(
        module_name: impl Into<String>,
        preservation: SelectiveFullGraphEditPreservation,
    ) -> Self {
        Self::Edit {
            module_name: module_name.into(),
            preservation,
        }
    }

    #[allow(dead_code)]
    pub(crate) fn delete(module_name: impl Into<String>) -> Self {
        Self::Delete {
            module_name: module_name.into(),
        }
    }

    pub(crate) fn module_name(&self) -> &str {
        match self {
            Self::Add { module_name }
            | Self::Edit { module_name, .. }
            | Self::Delete { module_name } => module_name,
        }
    }
}

/// Exact remap failure retained at a no-progress fixed point.
#[derive(Debug)]
pub(crate) struct SelectiveFullGraphRemapFailure {
    pub(crate) module_name: String,
    pub(crate) error: RemapError,
}

#[derive(Debug, Error)]
pub(crate) enum SelectiveFullGraphError {
    #[error("selective FullGraph {which} cache is invalid: {source}")]
    InvalidCache {
        which: &'static str,
        #[source]
        source: SpliceError,
    },
    #[error(
        "selective FullGraph change limit exceeded: {actual} > {limit}; split the change set or publish a separately proven complete cache"
    )]
    TooManyChanges { actual: usize, limit: usize },
    #[error("selective FullGraph manifest is invalid for module {module_name:?}: {reason}")]
    InvalidManifest {
        module_name: String,
        reason: String,
    },
    #[error(
        "selective FullGraph cannot delete module {module_name:?}: safe module removal requires declaration-tail pruning and proof that no retained module references the removed declarations"
    )]
    DeleteUnsupported { module_name: String },
    #[error("extracting declared FullGraph module {module_name:?}: {source}")]
    Extract {
        module_name: String,
        #[source]
        source: SpliceError,
    },
    #[error("preserving {stage} for FullGraph edit {module_name:?}: {reason}")]
    Preservation {
        module_name: String,
        stage: &'static str,
        reason: String,
    },
    #[error("composing declared FullGraph module {module_name:?}: {source}")]
    Compose {
        module_name: String,
        #[source]
        source: SpliceError,
    },
    #[error(
        "selective FullGraph edit {module_name:?} produced a byte-identical module entry; the backend may have ignored the declared edit"
    )]
    NoEffectiveEdit { module_name: String },
    #[error("proving effective FullGraph edit {module_name:?}: {reason}")]
    EffectiveEditProof { module_name: String, reason: String },
    #[error(
        "selective FullGraph composition made no progress with {remaining} module(s); unresolved cross-module cycles or genuinely unsupported references remain: {summary}"
    )]
    NoProgress {
        remaining: usize,
        summary: String,
        failures: Vec<SelectiveFullGraphRemapFailure>,
    },
}

/// A pristine-bound cache containing only the declared changes, in deterministic applied order.
#[derive(Debug)]
pub(crate) struct SelectiveFullGraphOutput {
    pub(crate) cache: Vec<u8>,
    pub(crate) applied_modules: Vec<String>,
}

enum AttemptFailure<Deferred, Fatal> {
    Deferred(Deferred),
    Fatal(Fatal),
}

#[derive(Debug)]
enum FixedPointFailure<Deferred, Fatal> {
    NoProgress(Vec<Deferred>),
    Fatal(Fatal),
}

/// Apply independent items immediately and retry only deferred items after the state advances.
/// The caller supplies a canonically ordered input. A deferred error is never accepted: it is
/// returned exactly if a whole pass makes no progress.
fn apply_at_fixed_point<Item, State, Deferred, Fatal>(
    initial: State,
    mut pending: Vec<Item>,
    mut name: impl FnMut(&Item) -> String,
    mut attempt: impl FnMut(&State, &Item) -> Result<State, AttemptFailure<Deferred, Fatal>>,
) -> Result<(State, Vec<String>), FixedPointFailure<Deferred, Fatal>> {
    let mut state = initial;
    let mut applied = Vec::with_capacity(pending.len());
    while !pending.is_empty() {
        let mut next = Vec::new();
        let mut deferred = Vec::new();
        let mut progressed = false;
        for item in pending {
            match attempt(&state, &item) {
                Ok(updated) => {
                    state = updated;
                    applied.push(name(&item));
                    progressed = true;
                }
                Err(AttemptFailure::Deferred(error)) => {
                    next.push(item);
                    deferred.push(error);
                }
                Err(AttemptFailure::Fatal(error)) => {
                    return Err(FixedPointFailure::Fatal(error));
                }
            }
        }
        if next.is_empty() {
            return Ok((state, applied));
        }
        if !progressed {
            return Err(FixedPointFailure::NoProgress(deferred));
        }
        pending = next;
    }
    Ok((state, applied))
}

/// Rebase only `changes` from `full_graph` onto the exact `pristine` bytes.
///
/// Remap errors alone are retryable because a successfully added provider module may make their
/// symbol authority resolvable on the next pass. Extraction, preservation, and guarded
/// composition failures are independent of future providers and therefore fail immediately.
pub(crate) fn compose_selective_full_graph(
    pristine: &[u8],
    full_graph: &[u8],
    mut changes: Vec<SelectiveFullGraphChange>,
) -> Result<SelectiveFullGraphOutput, SelectiveFullGraphError> {
    validate_standalone_script_cache(pristine).map_err(|source| {
        SelectiveFullGraphError::InvalidCache {
            which: "pristine",
            source,
        }
    })?;
    validate_standalone_script_cache(full_graph).map_err(|source| {
        SelectiveFullGraphError::InvalidCache {
            which: "rebuilt",
            source,
        }
    })?;
    if changes.len() > MAX_SELECTIVE_FULLGRAPH_CHANGES {
        return Err(SelectiveFullGraphError::TooManyChanges {
            actual: changes.len(),
            limit: MAX_SELECTIVE_FULLGRAPH_CHANGES,
        });
    }

    let pristine_names = module_name_counts(pristine, "pristine")?;
    let rebuilt_names = module_name_counts(full_graph, "rebuilt")?;
    let mut identities = HashSet::with_capacity(changes.len());
    for change in &changes {
        let module_name = change.module_name();
        validate_requested_name(module_name)?;
        if !identities.insert(module_name.to_lowercase()) {
            return Err(SelectiveFullGraphError::InvalidManifest {
                module_name: module_name.to_owned(),
                reason: "the declared change set contains a case-fold-colliding module identity"
                    .to_owned(),
            });
        }
        let pristine_count = pristine_names.get(module_name).copied().unwrap_or(0);
        let rebuilt_count = rebuilt_names.get(module_name).copied().unwrap_or(0);
        match change {
            SelectiveFullGraphChange::Add { .. } if pristine_count == 0 && rebuilt_count == 1 => {}
            SelectiveFullGraphChange::Add { .. } => {
                return Err(SelectiveFullGraphError::InvalidManifest {
                    module_name: module_name.to_owned(),
                    reason: format!(
                        "Add requires absence from pristine and exactly one rebuilt module; found {pristine_count}/{rebuilt_count}"
                    ),
                });
            }
            SelectiveFullGraphChange::Edit { .. }
                if pristine_count == 1 && rebuilt_count == 1 => {}
            SelectiveFullGraphChange::Edit { .. } => {
                return Err(SelectiveFullGraphError::InvalidManifest {
                    module_name: module_name.to_owned(),
                    reason: format!(
                        "Edit requires exactly one module in both pristine and rebuilt caches; found {pristine_count}/{rebuilt_count}"
                    ),
                });
            }
            SelectiveFullGraphChange::Delete { .. } => {
                return Err(SelectiveFullGraphError::DeleteUnsupported {
                    module_name: module_name.to_owned(),
                });
            }
        }
    }
    changes.sort_by(|left, right| left.module_name().cmp(right.module_name()));

    let result = apply_at_fixed_point(
        pristine.to_vec(),
        changes,
        |change| change.module_name().to_owned(),
        |running, change| attempt_change(running, full_graph, change),
    );
    match result {
        Ok((cache, applied_modules)) => Ok(SelectiveFullGraphOutput {
            cache,
            applied_modules,
        }),
        Err(FixedPointFailure::Fatal(error)) => Err(error),
        Err(FixedPointFailure::NoProgress(failures)) => {
            let summary = failures
                .iter()
                .take(4)
                .map(|failure: &SelectiveFullGraphRemapFailure| {
                    format!("{:?}: {}", failure.module_name, failure.error)
                })
                .collect::<Vec<_>>()
                .join("; ");
            let remaining = failures.len();
            Err(SelectiveFullGraphError::NoProgress {
                remaining,
                summary,
                failures,
            })
        }
    }
}

fn attempt_change(
    running: &[u8],
    full_graph: &[u8],
    change: &SelectiveFullGraphChange,
) -> Result<Vec<u8>, AttemptFailure<SelectiveFullGraphRemapFailure, SelectiveFullGraphError>> {
    let module_name = change.module_name();
    let extracted = extract_module(full_graph, module_name).map_err(|source| {
        AttemptFailure::Fatal(SelectiveFullGraphError::Extract {
            module_name: module_name.to_owned(),
            source,
        })
    })?;
    let allow_new_symbols = match change {
        SelectiveFullGraphChange::Add { .. } => true,
        SelectiveFullGraphChange::Edit { preservation, .. } => {
            preservation.generated_defaults.is_none()
        }
        SelectiveFullGraphChange::Delete { .. } => unreachable!("deletes fail during preflight"),
    };
    let mut mini = remap_module_to_base_with_options(
        &extracted,
        running,
        RemapOptions { allow_new_symbols },
    )
    .map_err(|error| {
        AttemptFailure::Deferred(SelectiveFullGraphRemapFailure {
            module_name: module_name.to_owned(),
            error,
        })
    })?;

    if let SelectiveFullGraphChange::Edit { preservation, .. } = change {
        if let Some(carry) = &preservation.generated_defaults {
            mini = preservation.metadata.apply_present(&mini).map_err(|reason| {
                AttemptFailure::Fatal(SelectiveFullGraphError::Preservation {
                    module_name: module_name.to_owned(),
                    stage: "pre-carry function metadata",
                    reason,
                })
            })?;
            mini = carry.apply(&mini).map_err(|reason| {
                AttemptFailure::Fatal(SelectiveFullGraphError::Preservation {
                    module_name: module_name.to_owned(),
                    stage: "generated defaults",
                    reason,
                })
            })?;
        }
        mini = preservation.metadata.apply(&mini).map_err(|reason| {
            AttemptFailure::Fatal(SelectiveFullGraphError::Preservation {
                module_name: module_name.to_owned(),
                stage: "existing function metadata",
                reason,
            })
        })?;
        preservation.structure.verify(&mini).map_err(|reason| {
            AttemptFailure::Fatal(SelectiveFullGraphError::Preservation {
                module_name: module_name.to_owned(),
                stage: "existing module structure",
                reason,
            })
        })?;
    }

    // A persistent guard rooted at `pristine` intentionally does not grant authority to a prior
    // mini. Constructing it from the exact running state is what turns a successfully composed Add
    // into ordinary base authority for dependent modules on this or a later pass.
    let mut guard = SequentialMiniGuard::new(running).map_err(|source| {
        AttemptFailure::Fatal(SelectiveFullGraphError::Compose {
            module_name: module_name.to_owned(),
            source,
        })
    })?;
    let updated = match change {
        SelectiveFullGraphChange::Add { .. } => guard.compose_add(running, &mini),
        SelectiveFullGraphChange::Edit { .. } => {
            guard.compose_edit(running, &mini, module_name)
        }
        SelectiveFullGraphChange::Delete { .. } => unreachable!("deletes fail during preflight"),
    }
    .map_err(|source| {
        AttemptFailure::Fatal(SelectiveFullGraphError::Compose {
            module_name: module_name.to_owned(),
            source,
        })
    })?;
    if matches!(change, SelectiveFullGraphChange::Edit { .. }) {
        let before = exact_module_entry(running, module_name).map_err(|reason| {
            AttemptFailure::Fatal(SelectiveFullGraphError::EffectiveEditProof {
                module_name: module_name.to_owned(),
                reason,
            })
        })?;
        let after = exact_module_entry(&updated, module_name).map_err(|reason| {
            AttemptFailure::Fatal(SelectiveFullGraphError::EffectiveEditProof {
                module_name: module_name.to_owned(),
                reason,
            })
        })?;
        if before == after {
            return Err(AttemptFailure::Fatal(
                SelectiveFullGraphError::NoEffectiveEdit {
                    module_name: module_name.to_owned(),
                },
            ));
        }
    }
    Ok(updated)
}

fn exact_module_entry<'a>(cache: &'a [u8], module_name: &str) -> Result<&'a [u8], String> {
    let ranges = module_ranges(cache)
        .map_err(|error| format!("walking composed module ranges: {error}"))?;
    let matches = ranges
        .iter()
        .filter(|(name, _, _)| name == module_name)
        .collect::<Vec<_>>();
    let [(_, start, end)] = matches.as_slice() else {
        return Err(format!(
            "expected exactly one module named {module_name:?}, found {}",
            matches.len()
        ));
    };
    cache
        .get(*start..*end)
        .ok_or_else(|| "composed module range is out of bounds".to_owned())
}

fn module_name_counts(
    cache: &[u8],
    which: &'static str,
) -> Result<BTreeMap<String, usize>, SelectiveFullGraphError> {
    let names = module_names(cache).map_err(|source| SelectiveFullGraphError::InvalidCache {
        which,
        source: SpliceError::Wire(source),
    })?;
    let mut counts = BTreeMap::new();
    for name in names {
        *counts.entry(name).or_insert(0) += 1;
    }
    Ok(counts)
}

fn validate_requested_name(module_name: &str) -> Result<(), SelectiveFullGraphError> {
    let invalid = if module_name.is_empty() {
        Some("module name is empty".to_owned())
    } else if module_name.len() > MAX_MODULE_NAME_BYTES {
        Some(format!(
            "module name is {} bytes, limit is {MAX_MODULE_NAME_BYTES}",
            module_name.len()
        ))
    } else if module_name.chars().any(char::is_control) {
        Some("module name contains a control character".to_owned())
    } else {
        None
    };
    match invalid {
        Some(reason) => Err(SelectiveFullGraphError::InvalidManifest {
            module_name: module_name.to_owned(),
            reason,
        }),
        None => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone, Debug)]
    struct MockChange {
        name: &'static str,
        requires: &'static [&'static str],
        fatal: bool,
    }

    #[test]
    fn fixed_point_retries_a_dependent_after_its_provider_succeeds() {
        let items = vec![
            MockChange {
                name: "A.Consumer",
                requires: &["Z.Provider"],
                fatal: false,
            },
            MockChange {
                name: "Z.Provider",
                requires: &[],
                fatal: false,
            },
        ];
        let result = apply_at_fixed_point(
            HashSet::<&'static str>::new(),
            items,
            |item| item.name.to_owned(),
            |state, item| {
                if item.fatal {
                    return Err(AttemptFailure::Fatal(item.name));
                }
                if item.requires.iter().all(|name| state.contains(name)) {
                    let mut updated = state.clone();
                    updated.insert(item.name);
                    Ok(updated)
                } else {
                    Err(AttemptFailure::Deferred(item.name))
                }
            },
        )
        .unwrap();

        assert_eq!(result.1, ["Z.Provider", "A.Consumer"]);
        assert_eq!(result.0.len(), 2);
    }

    #[test]
    fn fixed_point_returns_every_exact_failure_for_a_cycle() {
        let items = vec![
            MockChange {
                name: "A",
                requires: &["B"],
                fatal: false,
            },
            MockChange {
                name: "B",
                requires: &["A"],
                fatal: false,
            },
        ];
        let error = apply_at_fixed_point(
            HashSet::<&'static str>::new(),
            items,
            |item| item.name.to_owned(),
            |state, item| {
                if item.requires.iter().all(|name| state.contains(name)) {
                    let mut updated = state.clone();
                    updated.insert(item.name);
                    Ok(updated)
                } else {
                    Err(AttemptFailure::<_, &'static str>::Deferred(item.name))
                }
            },
        )
        .unwrap_err();

        match error {
            FixedPointFailure::NoProgress(failures) => assert_eq!(failures, ["A", "B"]),
            FixedPointFailure::Fatal(error) => panic!("unexpected fatal failure: {error}"),
        }
    }

    #[test]
    fn fixed_point_never_defers_a_fatal_failure() {
        let items = vec![MockChange {
            name: "Broken",
            requires: &[],
            fatal: true,
        }];
        let error = apply_at_fixed_point(
            HashSet::<&'static str>::new(),
            items,
            |item| item.name.to_owned(),
            |_state, item| {
                if item.fatal {
                    Err(AttemptFailure::<&'static str, _>::Fatal(item.name))
                } else {
                    unreachable!()
                }
            },
        )
        .unwrap_err();

        match error {
            FixedPointFailure::Fatal(error) => assert_eq!(error, "Broken"),
            FixedPointFailure::NoProgress(_) => panic!("fatal failure was deferred"),
        }
    }
}
