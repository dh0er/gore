//! Shared exact script-class identity and ancestry proof for default-value inspection.
//!
//! This layer validates the parsed module model before either scalar or tag-map code can attach a
//! target class to a generated initializer. Bare class identities, direct fields, canonical
//! initializer identities, and hierarchy chains are all unique and cycle-free.

use std::collections::{HashMap, HashSet};

use thiserror::Error;

use super::default_ancestry::DefaultNativeAncestry;
use super::default_patterns::{is_initializer_traits, is_plain_void};
use super::model::Module;

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub(crate) enum DefaultClassHierarchyError {
    #[error("duplicate class identity {module}.{class} in module model")]
    DuplicateClass { module: String, class: String },
    #[error("duplicate bare class identity {class} in modules {first_module} and {second_module}")]
    DuplicateBareClass {
        class: String,
        first_module: String,
        second_module: String,
    },
    #[error("class {module}.{class} declares field {field} more than once")]
    DuplicateDirectField {
        module: String,
        class: String,
        field: String,
    },
    #[error("class hierarchy contains a cycle reachable from {class}")]
    CyclicClassHierarchy { class: String },
    #[error("class {module}.{class} has {count} generated void __InitDefaults methods")]
    AmbiguousInitializer {
        module: String,
        class: String,
        count: usize,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DefaultClassIdentity {
    pub(crate) module: String,
    pub(crate) class: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DefaultClassAncestryProof {
    Script,
    Native(&'static str),
}

#[derive(Debug, Clone)]
pub(crate) struct DefaultClassHierarchy {
    /// Exact generated initializer function identity to its unique parsed target class.
    initializers: HashMap<String, DefaultClassIdentity>,
    /// Every parsed script class appears once. An unknown parent is a native terminal.
    supers: HashMap<String, Option<String>>,
    native: Option<DefaultNativeAncestry>,
}

impl DefaultClassHierarchy {
    pub(crate) fn build(
        modules: &[Module],
        native: Option<DefaultNativeAncestry>,
    ) -> Result<Self, DefaultClassHierarchyError> {
        let mut initializers = HashMap::new();
        let mut supers = HashMap::new();
        let mut defining_modules: HashMap<String, String> = HashMap::new();

        for module in modules {
            for class in &module.classes {
                if let Some(first_module) = defining_modules.get(&class.name) {
                    if first_module == &module.name {
                        return Err(DefaultClassHierarchyError::DuplicateClass {
                            module: module.name.clone(),
                            class: class.name.clone(),
                        });
                    }
                    return Err(DefaultClassHierarchyError::DuplicateBareClass {
                        class: class.name.clone(),
                        first_module: first_module.clone(),
                        second_module: module.name.clone(),
                    });
                }
                defining_modules.insert(class.name.clone(), module.name.clone());

                let mut direct_fields = HashSet::new();
                for field in &class.fields {
                    if !direct_fields.insert(field.name.as_str()) {
                        return Err(DefaultClassHierarchyError::DuplicateDirectField {
                            module: module.name.clone(),
                            class: class.name.clone(),
                            field: field.name.clone(),
                        });
                    }
                }

                let initializer_count = class
                    .methods
                    .iter()
                    .filter(|function| {
                        function.name == "__InitDefaults"
                            && is_initializer_traits(function.traits)
                            && function.params.is_empty()
                            && is_plain_void(&function.ret)
                    })
                    .count();
                if initializer_count > 1 {
                    return Err(DefaultClassHierarchyError::AmbiguousInitializer {
                        module: module.name.clone(),
                        class: class.name.clone(),
                        count: initializer_count,
                    });
                }
                if initializer_count == 1 {
                    let function = format!("{}.{}::__InitDefaults", module.name, class.name);
                    if initializers
                        .insert(
                            function,
                            DefaultClassIdentity {
                                module: module.name.clone(),
                                class: class.name.clone(),
                            },
                        )
                        .is_some()
                    {
                        return Err(DefaultClassHierarchyError::DuplicateClass {
                            module: module.name.clone(),
                            class: class.name.clone(),
                        });
                    }
                }

                supers.insert(
                    class.name.clone(),
                    class.super_class.clone().filter(|name| !name.is_empty()),
                );
            }
        }

        let hierarchy = Self {
            initializers,
            supers,
            native,
        };
        for class in hierarchy.supers.keys() {
            hierarchy.validate_chain(class)?;
        }
        Ok(hierarchy)
    }

    pub(crate) fn initializer_identity(&self, function: &str) -> Option<&DefaultClassIdentity> {
        self.initializers.get(function)
    }

    pub(crate) fn proves_ancestry(
        &self,
        target: &str,
        owner: &str,
    ) -> Option<DefaultClassAncestryProof> {
        if target == owner {
            return self
                .supers
                .contains_key(target)
                .then_some(DefaultClassAncestryProof::Script);
        }
        let mut seen = HashSet::new();
        let mut current = target;
        while let Some(Some(parent)) = self.supers.get(current) {
            if !seen.insert(current) {
                return None;
            }
            if parent == owner {
                return Some(DefaultClassAncestryProof::Script);
            }
            if !self.supers.contains_key(parent) {
                return self
                    .native
                    .as_ref()
                    .filter(|profile| profile.proves_ancestry(parent, owner))
                    .map(|profile| DefaultClassAncestryProof::Native(profile.profile_id()));
            }
            current = parent;
        }
        None
    }

    fn validate_chain(&self, start: &str) -> Result<(), DefaultClassHierarchyError> {
        let mut seen = HashSet::new();
        let mut current = start;
        while let Some(Some(parent)) = self.supers.get(current) {
            if !seen.insert(current) {
                return Err(DefaultClassHierarchyError::CyclicClassHierarchy {
                    class: start.to_owned(),
                });
            }
            if !self.supers.contains_key(parent) {
                return Ok(());
            }
            current = parent;
        }
        Ok(())
    }
}
