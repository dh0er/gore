//! Process-wide `--force` override for build-bound decompiler and compiler gates.
//!
//! Several checks refuse when the installed game no longer matches the build their evidence was
//! audited against, or when a regenerated module cannot be proven to preserve the original.
//! Refusing is the safe default. A user may still accept the risk: the CLI enables this override
//! for one process, and every gate that consults it then warns and continues instead of refusing.
//! Library callers (Studio, FFI) never enable it, so they keep the strict behaviour.

use std::collections::HashSet;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

static FORCED: AtomicBool = AtomicBool::new(false);
static WARNED: Mutex<Option<HashSet<String>>> = Mutex::new(None);

const HINT: &str = "\nhint: rerun with --force to proceed anyway. This skips a safety check that \
                    exists because the game build or the regenerated output could not be verified; \
                    the result may be broken";

/// Enable the override for the rest of this process.
pub fn enable() {
    FORCED.store(true, Ordering::SeqCst);
}

pub fn enabled() -> bool {
    FORCED.load(Ordering::SeqCst)
}

/// A refusal message that tells the user how to override it.
pub fn refusal(message: impl std::fmt::Display) -> String {
    format!("{message}{HINT}")
}

/// Print one warning per distinct message that a check was skipped because of `--force`.
pub fn warn(message: impl std::fmt::Display) {
    let message = message.to_string();
    let mut warned = WARNED
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    if warned
        .get_or_insert_with(HashSet::new)
        .insert(message.clone())
    {
        eprintln!("warning: --force: continuing despite: {message}. The result may be broken.");
    }
}

/// Resolve a failed gate: with `--force` warn and yield `Ok(None)`, otherwise return the refusal
/// with the override hint appended.
pub fn gate<T>(result: Result<T, String>) -> Result<Option<T>, String> {
    gate_with(enabled(), result)
}

fn gate_with<T>(forced: bool, result: Result<T, String>) -> Result<Option<T>, String> {
    match result {
        Ok(value) => Ok(Some(value)),
        Err(message) if forced => {
            warn(&message);
            Ok(None)
        }
        Err(message) => Err(refusal(message)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn refusals_name_the_override_and_forced_gates_continue() {
        let error = gate_with::<()>(false, Err("seal mismatch".to_owned())).unwrap_err();
        assert!(
            error.starts_with("seal mismatch\nhint: rerun with --force"),
            "{error}"
        );
        assert_eq!(gate_with(false, Ok(7)).unwrap(), Some(7));
        assert_eq!(
            gate_with::<()>(true, Err("seal mismatch".to_owned())).unwrap(),
            None
        );
    }
}
