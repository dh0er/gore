//! Versioned compiler-backend selection policy.
//!
//! The policy is deliberately independent from cache parsing and game-install transactions. A
//! caller supplies one-shot runners for the backends it can execute, and the returned report makes
//! both the backend that produced the final result and every permitted fallback explicit.

use std::fmt;

/// Backend-selection modes understood by the first standalone-compiler integration contract.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompilerBackendModeV1 {
    /// Require an injected standalone compiler. Never start the game compiler.
    Standalone,
    /// Use only the embedded compiler in the game.
    Game,
    /// Try the standalone compiler once, then explicitly fall back to the game compiler.
    StandaloneThenGame,
}

impl CompilerBackendModeV1 {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Standalone => "standalone",
            Self::Game => "game",
            Self::StandaloneThenGame => "standalone-then-game",
        }
    }
}

impl fmt::Display for CompilerBackendModeV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// Stable backend names reported to callers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompilerBackendNameV1 {
    Standalone,
    Game,
}

impl CompilerBackendNameV1 {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Standalone => "standalone",
            Self::Game => "game",
        }
    }
}

impl fmt::Display for CompilerBackendNameV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// Machine-readable failure classes shared by backend runners and fallback reports.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompilerBackendFailureKindV1 {
    /// Backend-independent input/base/tree preparation failed before a runner was entered.
    Preflight,
    /// The requested implementation was not installed or injected.
    Unavailable,
    /// The implementation does not support an input or language feature.
    Unsupported,
    /// The compiler rejected the submitted source.
    Rejected,
    /// The compiler returned output that failed structural or semantic validation.
    InvalidOutput,
    /// Setup, execution, or another backend-internal operation failed.
    Internal,
    /// A retained output artifact requires cleanup/recovery before another writer may run.
    RecoveryRequired,
}

impl CompilerBackendFailureKindV1 {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Preflight => "preflight",
            Self::Unavailable => "unavailable",
            Self::Unsupported => "unsupported",
            Self::Rejected => "rejected",
            Self::InvalidOutput => "invalid_output",
            Self::Internal => "internal",
            Self::RecoveryRequired => "recovery_required",
        }
    }
}

/// Structured error returned by a compiler backend.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompilerBackendFailureV1 {
    kind: CompilerBackendFailureKindV1,
    detail: String,
}

impl CompilerBackendFailureV1 {
    pub fn new(kind: CompilerBackendFailureKindV1, detail: impl Into<String>) -> Self {
        Self {
            kind,
            detail: detail.into(),
        }
    }

    pub fn unavailable(detail: impl Into<String>) -> Self {
        Self::new(CompilerBackendFailureKindV1::Unavailable, detail)
    }

    pub fn kind(&self) -> CompilerBackendFailureKindV1 {
        self.kind
    }

    pub fn detail(&self) -> &str {
        &self.detail
    }
}

impl fmt::Display for CompilerBackendFailureV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}: {}", self.kind.as_str(), self.detail)
    }
}

impl std::error::Error for CompilerBackendFailureV1 {}

/// Why `StandaloneThenGame` selected the game backend after its standalone attempt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompilerBackendFallbackReasonV1 {
    failed_backend: CompilerBackendNameV1,
    failure_kind: CompilerBackendFailureKindV1,
    detail: String,
}

impl CompilerBackendFallbackReasonV1 {
    fn from_failure(
        failed_backend: CompilerBackendNameV1,
        failure: &CompilerBackendFailureV1,
    ) -> Self {
        Self {
            failed_backend,
            failure_kind: failure.kind(),
            detail: failure.detail().to_owned(),
        }
    }

    pub fn failed_backend(&self) -> CompilerBackendNameV1 {
        self.failed_backend
    }

    pub fn failure_kind(&self) -> CompilerBackendFailureKindV1 {
        self.failure_kind
    }

    pub fn detail(&self) -> &str {
        &self.detail
    }
}

/// Result of one backend-selection decision.
///
/// `backend` is always the backend whose result is returned. A fallback reason is present exactly
/// when `StandaloneThenGame` attempted (or could not locate) standalone and proceeded to game.
#[derive(Debug)]
pub struct CompilerBackendRunReportV1<T> {
    backend: CompilerBackendNameV1,
    fallback_reason: Option<CompilerBackendFallbackReasonV1>,
    result: Result<T, CompilerBackendFailureV1>,
}

impl<T> CompilerBackendRunReportV1<T> {
    pub fn backend(&self) -> CompilerBackendNameV1 {
        self.backend
    }

    pub fn fallback_reason(&self) -> Option<&CompilerBackendFallbackReasonV1> {
        self.fallback_reason.as_ref()
    }

    pub fn result(&self) -> Result<&T, &CompilerBackendFailureV1> {
        self.result.as_ref()
    }

    pub fn into_parts(
        self,
    ) -> (
        Result<T, CompilerBackendFailureV1>,
        CompilerBackendNameV1,
        Option<CompilerBackendFallbackReasonV1>,
    ) {
        (self.result, self.backend, self.fallback_reason)
    }
}

/// Minimal object-safe runner used by the selection policy.
///
/// The compile layer captures its existing `(game_dir, source_tree)` `run_regen` arguments in
/// these one-shot runners. Keeping selection generic over `T` also allows project-wide compilation
/// to reuse the same policy later without inventing another backend contract.
pub trait CompilerBackendRunnerV1<T> {
    fn run(&mut self) -> Result<T, CompilerBackendFailureV1>;
}

impl<T, F> CompilerBackendRunnerV1<T> for F
where
    F: FnMut() -> Result<T, CompilerBackendFailureV1>,
{
    fn run(&mut self) -> Result<T, CompilerBackendFailureV1> {
        self()
    }
}

/// Execute exactly the backend policy selected by `mode`.
///
/// There is no implicit fallback in `Standalone` or `Game`. `StandaloneThenGame` is the only mode
/// that can call both runners, and its report always retains the structured standalone failure.
/// Backend-independent preflight failures and output-recovery failures are terminal because
/// starting a second writer would either be pointless or unsafe.
pub fn run_compiler_backend_v1<T>(
    mode: CompilerBackendModeV1,
    standalone: Option<&mut dyn CompilerBackendRunnerV1<T>>,
    game: &mut dyn CompilerBackendRunnerV1<T>,
) -> CompilerBackendRunReportV1<T> {
    match mode {
        CompilerBackendModeV1::Standalone => {
            let result = match standalone {
                Some(runner) => runner.run(),
                None => Err(CompilerBackendFailureV1::unavailable(
                    "no standalone compiler backend was injected",
                )),
            };
            CompilerBackendRunReportV1 {
                backend: CompilerBackendNameV1::Standalone,
                fallback_reason: None,
                result,
            }
        }
        CompilerBackendModeV1::Game => CompilerBackendRunReportV1 {
            backend: CompilerBackendNameV1::Game,
            fallback_reason: None,
            result: game.run(),
        },
        CompilerBackendModeV1::StandaloneThenGame => {
            let standalone_failure = match standalone {
                Some(runner) => match runner.run() {
                    Ok(value) => {
                        return CompilerBackendRunReportV1 {
                            backend: CompilerBackendNameV1::Standalone,
                            fallback_reason: None,
                            result: Ok(value),
                        };
                    }
                    Err(failure)
                        if matches!(
                            failure.kind(),
                            CompilerBackendFailureKindV1::Preflight
                                | CompilerBackendFailureKindV1::RecoveryRequired
                        ) =>
                    {
                        return CompilerBackendRunReportV1 {
                            backend: CompilerBackendNameV1::Standalone,
                            fallback_reason: None,
                            result: Err(failure),
                        };
                    }
                    Err(failure) => failure,
                },
                None => CompilerBackendFailureV1::unavailable(
                    "no standalone compiler backend was injected",
                ),
            };
            let fallback_reason = CompilerBackendFallbackReasonV1::from_failure(
                CompilerBackendNameV1::Standalone,
                &standalone_failure,
            );
            CompilerBackendRunReportV1 {
                backend: CompilerBackendNameV1::Game,
                fallback_reason: Some(fallback_reason),
                result: game.run(),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strict_modes_never_fallback() {
        let standalone_calls = std::cell::Cell::new(0u8);
        let game_calls = std::cell::Cell::new(0u8);
        let mut standalone = || {
            standalone_calls.set(standalone_calls.get() + 1);
            Err(CompilerBackendFailureV1::new(
                CompilerBackendFailureKindV1::Unsupported,
                "delegates are not implemented",
            ))
        };
        let mut game = || {
            game_calls.set(game_calls.get() + 1);
            Ok::<_, CompilerBackendFailureV1>("game")
        };

        let standalone_report = run_compiler_backend_v1(
            CompilerBackendModeV1::Standalone,
            Some(&mut standalone),
            &mut game,
        );
        assert_eq!(
            standalone_report.backend(),
            CompilerBackendNameV1::Standalone
        );
        assert!(standalone_report.fallback_reason().is_none());
        assert_eq!(
            standalone_report.result().unwrap_err().kind(),
            CompilerBackendFailureKindV1::Unsupported
        );
        assert_eq!(standalone_calls.get(), 1);
        assert_eq!(game_calls.get(), 0);

        let game_report = run_compiler_backend_v1(CompilerBackendModeV1::Game, None, &mut game);
        assert_eq!(game_report.result().unwrap(), &"game");
        assert_eq!(game_report.backend(), CompilerBackendNameV1::Game);
        assert!(game_report.fallback_reason().is_none());
        assert_eq!(standalone_calls.get(), 1);
        assert_eq!(game_calls.get(), 1);
    }

    #[test]
    fn explicit_fallback_reports_the_structured_standalone_failure() {
        let mut standalone = || {
            Err(CompilerBackendFailureV1::new(
                CompilerBackendFailureKindV1::InvalidOutput,
                "module table ended early",
            ))
        };
        let mut game = || Ok::<_, CompilerBackendFailureV1>(42u8);

        let report = run_compiler_backend_v1(
            CompilerBackendModeV1::StandaloneThenGame,
            Some(&mut standalone),
            &mut game,
        );
        assert_eq!(report.backend(), CompilerBackendNameV1::Game);
        assert_eq!(report.result().unwrap(), &42);
        let fallback = report.fallback_reason().unwrap();
        assert_eq!(fallback.failed_backend(), CompilerBackendNameV1::Standalone);
        assert_eq!(
            fallback.failure_kind(),
            CompilerBackendFailureKindV1::InvalidOutput
        );
        assert_eq!(fallback.detail(), "module table ended early");
    }

    #[test]
    fn missing_standalone_runner_is_visible_when_game_also_fails() {
        let mut game = || {
            Err::<(), _>(CompilerBackendFailureV1::new(
                CompilerBackendFailureKindV1::Internal,
                "shipping executable missing",
            ))
        };
        let report =
            run_compiler_backend_v1(CompilerBackendModeV1::StandaloneThenGame, None, &mut game);

        assert_eq!(report.backend(), CompilerBackendNameV1::Game);
        assert_eq!(
            report.result().unwrap_err().detail(),
            "shipping executable missing"
        );
        let fallback = report.fallback_reason().unwrap();
        assert_eq!(
            fallback.failure_kind(),
            CompilerBackendFailureKindV1::Unavailable
        );
        assert_eq!(
            fallback.detail(),
            "no standalone compiler backend was injected"
        );
    }

    #[test]
    fn recovery_required_is_terminal_even_in_fallback_mode() {
        let game_calls = std::cell::Cell::new(0u8);
        let mut standalone = || {
            Err(CompilerBackendFailureV1::new(
                CompilerBackendFailureKindV1::RecoveryRequired,
                "retained artifact could not be neutralized",
            ))
        };
        let mut game = || {
            game_calls.set(game_calls.get() + 1);
            Ok::<_, CompilerBackendFailureV1>(())
        };

        let report = run_compiler_backend_v1(
            CompilerBackendModeV1::StandaloneThenGame,
            Some(&mut standalone),
            &mut game,
        );
        assert_eq!(report.backend(), CompilerBackendNameV1::Standalone);
        assert_eq!(
            report.result().unwrap_err().kind(),
            CompilerBackendFailureKindV1::RecoveryRequired
        );
        assert!(report.fallback_reason().is_none());
        assert_eq!(game_calls.get(), 0);
    }
}
