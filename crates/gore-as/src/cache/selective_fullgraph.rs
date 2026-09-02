//! Selectively rebase declared FullGraph changes onto an exact pristine cache.
//!
//! A complete FullGraph rebuild is useful compiler evidence, but publishing it wholesale also
//! publishes unrelated regeneration drift. This module extracts only the declared Add/Edit
//! modules and folds them onto the pristine bytes. A module may refer to a newly added module:
//! provider-blocked remaps are woken only when that provider lands, until either all changes apply
//! or the dependency graph reaches a fail-closed fixed point.

use std::collections::{BTreeMap, HashSet, VecDeque};

use thiserror::Error;

use super::default_targets::ExistingDefaultTargetPlan;
use super::generated_defaults::{
    ExistingFunctionMetadataPlan, ExistingModuleStructurePlan, GeneratedDefaultsPlan,
};
use super::remap::{RemapDependencyIndex, RemapError, RemapOptions};
use super::splice::{
    extract_module, remap_module_to_base_with_options, validate_standalone_script_cache,
    SequentialMiniGuard, SpliceError,
};
use super::walk_modules::{module_names, module_ranges};

const MAX_SELECTIVE_FULLGRAPH_CHANGES: usize = 256;
const MAX_MODULE_NAME_BYTES: usize = 4_096;
const MAX_FIXED_POINT_ATTEMPTS_PER_CHANGE: usize = 8;

/// Compiler-derived preservation state for one existing module.
///
/// `generated_defaults` is present for either the legacy defaults-free source path (strict
/// remap) or a hybrid edit whose defaults belong exclusively to appended classes (new-symbol
/// remap). An existing-class authored-default edit instead regenerates every existing default and
/// only restores the existing Unreal function metadata.
#[derive(Clone, Debug)]
pub(crate) struct SelectiveFullGraphEditPreservation {
    metadata: ExistingFunctionMetadataPlan,
    structure: ExistingModuleStructurePlan,
    generated_defaults: Option<GeneratedDefaultsPlan>,
    default_targets: Option<ExistingDefaultTargetPlan>,
}

impl SelectiveFullGraphEditPreservation {
    pub(crate) fn new(
        metadata: ExistingFunctionMetadataPlan,
        structure: ExistingModuleStructurePlan,
        generated_defaults: Option<GeneratedDefaultsPlan>,
        default_targets: Option<ExistingDefaultTargetPlan>,
    ) -> Self {
        Self {
            metadata,
            structure,
            generated_defaults,
            default_targets,
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
    InvalidManifest { module_name: String, reason: String },
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
    #[error("indexing rebuilt FullGraph dependencies: {source}")]
    DependencyIndex {
        #[source]
        source: RemapError,
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
    #[error(
        "selective FullGraph dependency resolution stopped after {attempts} remap attempts for module {module_name:?} (limit {limit}); split an unusually deep cross-module change graph or publish a separately proven complete cache"
    )]
    RetryLimit {
        module_name: String,
        attempts: usize,
        limit: usize,
    },
}

pub(crate) fn validate_selective_full_graph_change_count(
    actual: usize,
) -> Result<(), SelectiveFullGraphError> {
    if actual > MAX_SELECTIVE_FULLGRAPH_CHANGES {
        return Err(SelectiveFullGraphError::TooManyChanges {
            actual,
            limit: MAX_SELECTIVE_FULLGRAPH_CHANGES,
        });
    }
    Ok(())
}

/// A pristine-bound cache containing only the declared changes, in deterministic applied order.
#[derive(Debug)]
pub(crate) struct SelectiveFullGraphOutput {
    pub(crate) cache: Vec<u8>,
    pub(crate) applied_modules: Vec<String>,
}

enum AttemptFailure<Deferred, Fatal> {
    Deferred {
        error: Deferred,
        retry_after: Option<String>,
    },
    Fatal(Fatal),
}

#[derive(Debug)]
enum FixedPointFailure<Deferred, Fatal> {
    NoProgress(Vec<Deferred>),
    Fatal(Fatal),
    RetryLimit {
        item_name: String,
        attempts: usize,
        limit: usize,
    },
}

/// Apply independent items immediately and wake a blocked item only when its named provider lands.
/// The caller supplies a canonically ordered input. Provider-free failures are terminal at this
/// fixed point; cycles and unavailable providers retain their exact errors in canonical order.
fn apply_at_fixed_point<Item, State, Deferred, Fatal>(
    initial: State,
    pending: Vec<Item>,
    mut name: impl FnMut(&Item) -> String,
    mut providers: impl FnMut(&Item) -> Vec<String>,
    mut attempt: impl FnMut(&State, &Item) -> Result<State, AttemptFailure<Deferred, Fatal>>,
) -> Result<(State, Vec<String>), FixedPointFailure<Deferred, Fatal>> {
    struct Queued<Item> {
        ordinal: usize,
        item: Item,
        providers: Vec<String>,
        attempts: usize,
    }

    struct Blocked<Item, Deferred> {
        queued: Queued<Item>,
        error: Deferred,
    }

    let pending_len = pending.len();
    let mut declared = HashSet::new();
    let mut ready = VecDeque::with_capacity(pending_len);
    for (ordinal, item) in pending.into_iter().enumerate() {
        let mut item_providers = providers(&item).into_iter().collect::<Vec<_>>();
        item_providers.sort();
        item_providers.dedup();
        declared.extend(item_providers.iter().cloned());
        ready.push_back(Queued {
            ordinal,
            item,
            providers: item_providers,
            attempts: 0,
        });
    }
    let mut waiting = BTreeMap::<String, Vec<Blocked<Item, Deferred>>>::new();
    let mut terminal = Vec::<(usize, Deferred)>::new();
    let mut applied_identities = HashSet::with_capacity(declared.len());
    let mut state = initial;
    let mut applied = Vec::with_capacity(pending_len);
    while let Some(mut queued) = ready.pop_front() {
        let item_name = name(&queued.item);
        queued.attempts += 1;
        match attempt(&state, &queued.item) {
            Ok(updated) => {
                state = updated;
                applied.push(item_name);
                let mut woken = Vec::new();
                for identity in queued.providers {
                    applied_identities.insert(identity.clone());
                    if let Some(blocked) = waiting.remove(&identity) {
                        woken.extend(blocked.into_iter().map(|entry| entry.queued));
                    }
                }
                woken.sort_by_key(|entry| entry.ordinal);
                ready.extend(woken);
            }
            Err(AttemptFailure::Deferred { error, retry_after }) => {
                let Some(provider) = retry_after else {
                    terminal.push((queued.ordinal, error));
                    continue;
                };
                if queued.providers.contains(&provider)
                    || !declared.contains(&provider)
                    || applied_identities.contains(&provider)
                {
                    terminal.push((queued.ordinal, error));
                    continue;
                }
                if queued.attempts >= MAX_FIXED_POINT_ATTEMPTS_PER_CHANGE {
                    return Err(FixedPointFailure::RetryLimit {
                        item_name,
                        attempts: queued.attempts,
                        limit: MAX_FIXED_POINT_ATTEMPTS_PER_CHANGE,
                    });
                }
                waiting
                    .entry(provider)
                    .or_default()
                    .push(Blocked { queued, error });
            }
            Err(AttemptFailure::Fatal(error)) => {
                return Err(FixedPointFailure::Fatal(error));
            }
        }
    }
    for blocked in waiting.into_values().flatten() {
        terminal.push((blocked.queued.ordinal, blocked.error));
    }
    if terminal.is_empty() {
        Ok((state, applied))
    } else {
        terminal.sort_by_key(|(ordinal, _)| *ordinal);
        Err(FixedPointFailure::NoProgress(
            terminal.into_iter().map(|(_, error)| error).collect(),
        ))
    }
}

struct PreparedChange {
    change: SelectiveFullGraphChange,
    provider_identities: Vec<String>,
}

impl PreparedChange {
    fn module_name(&self) -> &str {
        self.change.module_name()
    }
}

/// Rebase only `changes` from `full_graph` onto the exact `pristine` bytes.
///
/// Only remap errors tied to a specific declared runtime provider are retryable. Extraction,
/// provider-free remap failures, preservation, and guarded composition failures cannot be repaired
/// by a later module and therefore fail at this fixed point.
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
    validate_selective_full_graph_change_count(changes.len())?;

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
            SelectiveFullGraphChange::Edit { .. } if pristine_count == 1 && rebuilt_count == 1 => {}
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

    let requested_modules = changes
        .iter()
        .map(|change| change.module_name().to_owned())
        .collect::<HashSet<_>>();
    let dependency_index = RemapDependencyIndex::build(full_graph, &requested_modules)
        .map_err(|source| SelectiveFullGraphError::DependencyIndex { source })?;
    let mut prepared = Vec::with_capacity(changes.len());
    for change in changes {
        let provider_identities = dependency_index.providers_for_outer_module(change.module_name());
        prepared.push(PreparedChange {
            change,
            provider_identities,
        });
    }

    let result = apply_at_fixed_point(
        pristine.to_vec(),
        prepared,
        |change| change.module_name().to_owned(),
        |change| change.provider_identities.clone(),
        |running, change| attempt_change(running, full_graph, &dependency_index, &change.change),
    );
    match result {
        Ok((cache, applied_modules)) => Ok(SelectiveFullGraphOutput {
            cache,
            applied_modules,
        }),
        Err(FixedPointFailure::Fatal(error)) => Err(error),
        Err(FixedPointFailure::RetryLimit {
            item_name,
            attempts,
            limit,
        }) => Err(SelectiveFullGraphError::RetryLimit {
            module_name: item_name,
            attempts,
            limit,
        }),
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
    dependency_index: &RemapDependencyIndex,
    change: &SelectiveFullGraphChange,
) -> Result<Vec<u8>, AttemptFailure<SelectiveFullGraphRemapFailure, SelectiveFullGraphError>> {
    let module_name = change.module_name();
    // Extract on demand. Every extracted mini owns the complete global tail-table set, so retaining
    // one per declared change would amplify a Shipping cache into multiple gigabytes.
    let extracted = extract_module(full_graph, module_name).map_err(|source| {
        AttemptFailure::Fatal(SelectiveFullGraphError::Extract {
            module_name: module_name.to_owned(),
            source,
        })
    })?;
    let allow_new_symbols = match change {
        SelectiveFullGraphChange::Add { .. } => true,
        SelectiveFullGraphChange::Edit { preservation, .. } => preservation
            .generated_defaults
            .as_ref()
            .map_or(true, GeneratedDefaultsPlan::allows_new_symbols),
        SelectiveFullGraphChange::Delete { .. } => unreachable!("deletes fail during preflight"),
    };
    let mut mini =
        remap_module_to_base_with_options(&extracted, running, RemapOptions { allow_new_symbols })
            .map_err(|error| {
                let retry_after = dependency_index.retry_provider(&error);
                AttemptFailure::Deferred {
                    error: SelectiveFullGraphRemapFailure {
                        module_name: module_name.to_owned(),
                        error,
                    },
                    retry_after,
                }
            })?;

    if let SelectiveFullGraphChange::Edit { preservation, .. } = change {
        if let Some(carry) = &preservation.generated_defaults {
            mini = preservation
                .metadata
                .apply_present(&mini)
                .map_err(|reason| {
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
        SelectiveFullGraphChange::Edit { .. } => guard.compose_edit(running, &mini, module_name),
        SelectiveFullGraphChange::Delete { .. } => unreachable!("deletes fail during preflight"),
    }
    .map_err(|source| {
        AttemptFailure::Fatal(SelectiveFullGraphError::Compose {
            module_name: module_name.to_owned(),
            source,
        })
    })?;
    if let SelectiveFullGraphChange::Edit { preservation, .. } = change {
        if let Some(default_targets) = &preservation.default_targets {
            default_targets.verify(&updated).map_err(|reason| {
                AttemptFailure::Fatal(SelectiveFullGraphError::Preservation {
                    module_name: module_name.to_owned(),
                    stage: "existing default targets in composed cache",
                    reason,
                })
            })?;
        }
    }
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
    let ranges =
        module_ranges(cache).map_err(|error| format!("walking composed module ranges: {error}"))?;
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
    fn selective_change_count_accepts_the_limit_and_rejects_the_next_change() {
        assert!(validate_selective_full_graph_change_count(256).is_ok());
        assert!(matches!(
            validate_selective_full_graph_change_count(257),
            Err(SelectiveFullGraphError::TooManyChanges {
                actual: 257,
                limit: 256
            })
        ));
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
            |item| vec![item.name.to_owned()],
            |state, item| {
                if item.fatal {
                    return Err(AttemptFailure::Fatal(item.name));
                }
                if let Some(provider) = item.requires.iter().find(|name| !state.contains(*name)) {
                    Err(AttemptFailure::Deferred {
                        error: item.name,
                        retry_after: Some((*provider).to_owned()),
                    })
                } else {
                    let mut updated = state.clone();
                    updated.insert(item.name);
                    Ok(updated)
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
            |item| vec![item.name.to_owned()],
            |state, item| {
                if let Some(provider) = item.requires.iter().find(|name| !state.contains(*name)) {
                    Err(AttemptFailure::<_, &'static str>::Deferred {
                        error: item.name,
                        retry_after: Some((*provider).to_owned()),
                    })
                } else {
                    let mut updated = state.clone();
                    updated.insert(item.name);
                    Ok(updated)
                }
            },
        )
        .unwrap_err();

        match error {
            FixedPointFailure::NoProgress(failures) => assert_eq!(failures, ["A", "B"]),
            FixedPointFailure::Fatal(error) => panic!("unexpected fatal failure: {error}"),
            FixedPointFailure::RetryLimit { .. } => panic!("unexpected retry limit"),
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
            |item| vec![item.name.to_owned()],
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
            FixedPointFailure::RetryLimit { .. } => panic!("fatal failure was deferred"),
        }
    }

    #[test]
    fn fixed_point_wakes_only_the_next_consumer_in_a_reverse_chain() {
        const COUNT: usize = MAX_SELECTIVE_FULLGRAPH_CHANGES;
        let mut attempts = vec![0usize; COUNT];
        let result = apply_at_fixed_point(
            HashSet::<usize>::new(),
            (0..COUNT).collect(),
            |item| item.to_string(),
            |item| vec![item.to_string()],
            |state, item| {
                attempts[*item] += 1;
                if *item + 1 < COUNT && !state.contains(&(*item + 1)) {
                    return Err(AttemptFailure::<usize, ()>::Deferred {
                        error: *item,
                        retry_after: Some((*item + 1).to_string()),
                    });
                }
                let mut updated = state.clone();
                updated.insert(*item);
                Ok(updated)
            },
        )
        .unwrap();

        assert_eq!(result.0.len(), COUNT);
        assert_eq!(result.1.first().map(String::as_str), Some("255"));
        assert_eq!(result.1.last().map(String::as_str), Some("0"));
        assert_eq!(attempts.iter().sum::<usize>(), COUNT * 2 - 1);
        assert!(attempts.into_iter().all(|attempts| attempts <= 2));
    }

    #[test]
    fn fixed_point_does_not_retry_a_provider_free_failure() {
        let mut attempts = [0usize; 2];
        let error = apply_at_fixed_point(
            HashSet::<usize>::new(),
            vec![0usize, 1],
            |item| item.to_string(),
            |item| vec![item.to_string()],
            |state, item| {
                attempts[*item] += 1;
                if *item == 0 {
                    return Err(AttemptFailure::<_, ()>::Deferred {
                        error: "terminal",
                        retry_after: None,
                    });
                }
                let mut updated = state.clone();
                updated.insert(*item);
                Ok(updated)
            },
        )
        .unwrap_err();

        assert_eq!(attempts, [1, 1]);
        match error {
            FixedPointFailure::NoProgress(failures) => assert_eq!(failures, ["terminal"]),
            FixedPointFailure::Fatal(()) => unreachable!(),
            FixedPointFailure::RetryLimit { .. } => panic!("provider-free failure was retried"),
        }
    }

    #[test]
    fn fixed_point_bounds_staggered_multi_provider_discovery() {
        #[derive(Clone)]
        struct StaggeredChange {
            name: String,
            requires: Vec<String>,
        }

        let stages = MAX_FIXED_POINT_ATTEMPTS_PER_CHANGE + 1;
        let mut items = vec![StaggeredChange {
            name: "Consumer".to_owned(),
            requires: (1..=stages)
                .map(|stage| format!("Provider{stage}"))
                .collect(),
        }];
        for stage in 1..=stages {
            let provider = format!("Provider{stage}");
            items.push(StaggeredChange {
                name: provider.clone(),
                requires: (stage > 1)
                    .then(|| format!("{provider}.Step1"))
                    .into_iter()
                    .collect(),
            });
            for step in 1..stage {
                items.push(StaggeredChange {
                    name: format!("{provider}.Step{step}"),
                    requires: (step + 1 < stage)
                        .then(|| format!("{provider}.Step{}", step + 1))
                        .into_iter()
                        .collect(),
                });
            }
        }
        let total_items = items.len();
        let mut attempts = 0usize;
        let error = apply_at_fixed_point(
            HashSet::<String>::new(),
            items,
            |item| item.name.clone(),
            |item| vec![item.name.clone()],
            |state, item| {
                attempts += 1;
                if let Some(provider) = item
                    .requires
                    .iter()
                    .find(|provider| !state.contains(*provider))
                {
                    return Err(AttemptFailure::<_, ()>::Deferred {
                        error: item.name.clone(),
                        retry_after: Some(provider.clone()),
                    });
                }
                let mut updated = state.clone();
                updated.insert(item.name.clone());
                Ok(updated)
            },
        )
        .unwrap_err();

        assert!(attempts <= total_items * MAX_FIXED_POINT_ATTEMPTS_PER_CHANGE);
        assert!(matches!(
            error,
            FixedPointFailure::RetryLimit {
                item_name,
                attempts: MAX_FIXED_POINT_ATTEMPTS_PER_CHANGE,
                limit: MAX_FIXED_POINT_ATTEMPTS_PER_CHANGE,
            } if item_name == "Consumer"
        ));
    }

    #[test]
    fn fixed_point_wakes_by_runtime_provider_instead_of_outer_display_name() {
        #[derive(Clone)]
        struct AliasedChange {
            outer: &'static str,
            runtime: &'static str,
            requires: Option<&'static str>,
        }

        let result = apply_at_fixed_point(
            HashSet::<&'static str>::new(),
            vec![
                AliasedChange {
                    outer: "A.ConsumerOuter",
                    runtime: "Runtime.Consumer",
                    requires: Some("Runtime.Provider"),
                },
                AliasedChange {
                    outer: "Z.ProviderOuter",
                    runtime: "Runtime.Provider",
                    requires: None,
                },
            ],
            |item| item.outer.to_owned(),
            |item| vec![item.runtime.to_owned()],
            |state, item| {
                if let Some(provider) = item.requires.filter(|name| !state.contains(name)) {
                    return Err(AttemptFailure::<_, ()>::Deferred {
                        error: item.outer,
                        retry_after: Some(provider.to_owned()),
                    });
                }
                let mut updated = state.clone();
                updated.insert(item.runtime);
                Ok(updated)
            },
        )
        .unwrap();

        assert_eq!(result.1, ["Z.ProviderOuter", "A.ConsumerOuter"]);
    }
}
