//! Closed differential-qualification payloads for one compiler profile.
//!
//! A profile is not qualified merely because its manifest says so. The four payloads modeled
//! here bind an ordered positive/negative source corpus to exact expected results and to separate
//! diagnostic and semantic observations from the embedded game compiler and the standalone
//! compiler. The qualified loader accepts only complete, zero-difference reports.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};
use sha2::{Digest as _, Sha256};

use super::manifest::{BytecodeProfileV1, QualificationProfileV1, SealedBlobV1, Sha256Digest};

pub const PROBE_CORPUS_SCHEMA: &str = "gore.as.compiler-probe-corpus";
pub const EXPECTED_RESULTS_SCHEMA: &str = "gore.as.compiler-probe-results";
pub const DIAGNOSTIC_PARITY_SCHEMA: &str = "gore.as.compiler-diagnostic-parity";
pub const SEMANTIC_PARITY_SCHEMA: &str = "gore.as.compiler-semantic-parity";
pub const QUALIFICATION_SCHEMA_VERSION: u32 = 1;

const MAX_QUALIFICATION_JSON_BYTES: usize = 32 * 1024 * 1024;
const MAX_PROBE_CASES: usize = 8192;
const MAX_SECTIONS_PER_CASE: usize = 256;
const MAX_DIAGNOSTICS_PER_CASE: usize = 4096;
const MAX_TEXT_BYTES: usize = 64 * 1024;
const MAX_SOURCE_BYTES: usize = 2 * 1024 * 1024;
const MAX_TOTAL_SOURCE_BYTES: usize = 24 * 1024 * 1024;

const CORPUS_HASH_DOMAIN: &[u8] = b"gore-as-probe-corpus-v1\0";
const RESULTS_HASH_DOMAIN: &[u8] = b"gore-as-probe-results-v1\0";
const DIAGNOSTIC_REPORT_HASH_DOMAIN: &[u8] = b"gore-as-diagnostic-parity-v1\0";
const SEMANTIC_REPORT_HASH_DOMAIN: &[u8] = b"gore-as-semantic-parity-v1\0";
const DIAGNOSTIC_SET_HASH_DOMAIN: &[u8] = b"gore-as-expected-diagnostics-v1\0";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProbeOutcomeV1 {
    Accepted,
    Rejected,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ProbeModeV1 {
    CompileOnly,
    Invoke { declaration: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProbeSourceSectionV1 {
    pub ordinal: u32,
    pub module: String,
    pub relative_path: String,
    pub source_utf8: String,
    pub source_sha256: Sha256Digest,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CompilerProbeCaseV1 {
    pub ordinal: u32,
    pub case_id: String,
    pub category: String,
    pub expected_outcome: ProbeOutcomeV1,
    pub mode: ProbeModeV1,
    pub sections: Vec<ProbeSourceSectionV1>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CompilerProbeCorpusV1 {
    pub schema: String,
    pub schema_version: u32,
    pub suite_id: String,
    pub cases: Vec<CompilerProbeCaseV1>,
    pub canonical_sha256: Sha256Digest,
}

impl CompilerProbeCorpusV1 {
    pub fn seal(&mut self) -> Result<(), QualificationError> {
        self.validate_structure()?;
        self.canonical_sha256 = self.computed_digest()?;
        Ok(())
    }

    pub fn validate(&self) -> Result<(), QualificationError> {
        self.validate_structure()?;
        check_digest(
            "probe corpus",
            self.canonical_sha256,
            self.computed_digest()?,
        )
    }

    pub fn from_json(bytes: &[u8]) -> Result<Self, QualificationError> {
        parse_bounded(bytes, "probe corpus", Self::validate)
    }

    pub fn to_json(&self) -> Result<Vec<u8>, QualificationError> {
        self.validate()?;
        Ok(serde_json::to_vec_pretty(self)?)
    }

    fn validate_structure(&self) -> Result<(), QualificationError> {
        check_schema(&self.schema, self.schema_version, PROBE_CORPUS_SCHEMA)?;
        validate_text(&self.suite_id, "suite_id", MAX_TEXT_BYTES, false)?;
        check_nonempty_count("probe cases", self.cases.len(), MAX_PROBE_CASES)?;

        let mut ids = BTreeSet::new();
        let mut total_source_bytes = 0usize;
        for (case_index, case) in self.cases.iter().enumerate() {
            check_ordinal("probe case", case_index, case.ordinal)?;
            validate_text(&case.case_id, "case_id", MAX_TEXT_BYTES, false)?;
            validate_text(&case.category, "category", MAX_TEXT_BYTES, false)?;
            if !ids.insert(case.case_id.as_str()) {
                return invalid("case_id", "must be unique");
            }
            if let ProbeModeV1::Invoke { declaration } = &case.mode {
                validate_text(declaration, "mode.declaration", MAX_TEXT_BYTES, false)?;
                if case.expected_outcome != ProbeOutcomeV1::Accepted {
                    return invalid("mode", "a rejected probe cannot request invocation");
                }
            }
            check_nonempty_count(
                "probe source sections",
                case.sections.len(),
                MAX_SECTIONS_PER_CASE,
            )?;
            let mut section_keys = BTreeSet::new();
            for (section_index, section) in case.sections.iter().enumerate() {
                check_ordinal("probe source section", section_index, section.ordinal)?;
                validate_text(&section.module, "section.module", MAX_TEXT_BYTES, false)?;
                validate_relative_path(&section.relative_path)?;
                validate_text(
                    &section.source_utf8,
                    "section.source_utf8",
                    MAX_SOURCE_BYTES,
                    true,
                )?;
                let key = (
                    section.module.as_str(),
                    section.relative_path.to_ascii_lowercase(),
                );
                if !section_keys.insert(key) {
                    return invalid(
                        "sections",
                        "module/path pairs must be unique under Windows path casing",
                    );
                }
                let actual =
                    Sha256Digest::from_bytes(Sha256::digest(section.source_utf8.as_bytes()).into());
                check_digest("probe source", section.source_sha256, actual)?;
                total_source_bytes = total_source_bytes
                    .checked_add(section.source_utf8.len())
                    .ok_or(QualificationError::SourceBytesOverflow)?;
                if total_source_bytes > MAX_TOTAL_SOURCE_BYTES {
                    return Err(QualificationError::TotalSourceTooLarge {
                        actual: total_source_bytes,
                        max: MAX_TOTAL_SOURCE_BYTES,
                    });
                }
            }
        }
        Ok(())
    }

    fn computed_digest(&self) -> Result<Sha256Digest, QualificationError> {
        let mut canonical = self.clone();
        canonical.canonical_sha256 = zero_digest();
        canonical_digest(CORPUS_HASH_DOMAIN, &canonical)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProbeDiagnosticSeverityV1 {
    Info,
    Warning,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExpectedDiagnosticV1 {
    pub ordinal: u32,
    pub severity: ProbeDiagnosticSeverityV1,
    pub section: String,
    pub row: u32,
    pub column: u32,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExpectedProbeResultV1 {
    pub ordinal: u32,
    pub case_id: String,
    pub outcome: ProbeOutcomeV1,
    pub diagnostics: Vec<ExpectedDiagnosticV1>,
    /// Digest of the canonical normalized cache/return-value observation. Rejected probes have
    /// no semantic result; every accepted probe must have one, including compile-only probes.
    pub semantic_sha256: Option<Sha256Digest>,
}

impl ExpectedProbeResultV1 {
    pub fn diagnostics_sha256(&self) -> Result<Sha256Digest, QualificationError> {
        canonical_digest(DIAGNOSTIC_SET_HASH_DOMAIN, &self.diagnostics)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExpectedProbeResultsV1 {
    pub schema: String,
    pub schema_version: u32,
    pub suite_id: String,
    pub corpus_sha256: Sha256Digest,
    pub results: Vec<ExpectedProbeResultV1>,
    pub canonical_sha256: Sha256Digest,
}

impl ExpectedProbeResultsV1 {
    pub fn seal(&mut self) -> Result<(), QualificationError> {
        self.validate_structure()?;
        self.canonical_sha256 = self.computed_digest()?;
        Ok(())
    }

    pub fn validate(&self) -> Result<(), QualificationError> {
        self.validate_structure()?;
        check_digest(
            "expected probe results",
            self.canonical_sha256,
            self.computed_digest()?,
        )
    }

    pub fn from_json(bytes: &[u8]) -> Result<Self, QualificationError> {
        parse_bounded(bytes, "expected probe results", Self::validate)
    }

    pub fn to_json(&self) -> Result<Vec<u8>, QualificationError> {
        self.validate()?;
        Ok(serde_json::to_vec_pretty(self)?)
    }

    fn validate_structure(&self) -> Result<(), QualificationError> {
        check_schema(&self.schema, self.schema_version, EXPECTED_RESULTS_SCHEMA)?;
        validate_text(&self.suite_id, "suite_id", MAX_TEXT_BYTES, false)?;
        check_nonempty_count(
            "expected probe results",
            self.results.len(),
            MAX_PROBE_CASES,
        )?;
        let mut ids = BTreeSet::new();
        for (index, result) in self.results.iter().enumerate() {
            check_ordinal("expected probe result", index, result.ordinal)?;
            validate_text(&result.case_id, "result.case_id", MAX_TEXT_BYTES, false)?;
            if !ids.insert(result.case_id.as_str()) {
                return invalid("result.case_id", "must be unique");
            }
            if result.diagnostics.len() > MAX_DIAGNOSTICS_PER_CASE {
                return Err(QualificationError::CountTooLarge {
                    field: "expected diagnostics",
                    actual: result.diagnostics.len(),
                    max: MAX_DIAGNOSTICS_PER_CASE,
                });
            }
            let mut has_error = false;
            for (diagnostic_index, diagnostic) in result.diagnostics.iter().enumerate() {
                check_ordinal("expected diagnostic", diagnostic_index, diagnostic.ordinal)?;
                validate_text(
                    &diagnostic.section,
                    "diagnostic.section",
                    MAX_TEXT_BYTES,
                    false,
                )?;
                validate_text(
                    &diagnostic.message,
                    "diagnostic.message",
                    MAX_TEXT_BYTES,
                    true,
                )?;
                if diagnostic.row == 0 || diagnostic.column == 0 {
                    return invalid("diagnostic position", "row and column are one-based");
                }
                has_error |= diagnostic.severity == ProbeDiagnosticSeverityV1::Error;
            }
            match (result.outcome, result.semantic_sha256, has_error) {
                (ProbeOutcomeV1::Accepted, None, _) => {
                    return invalid(
                        "semantic_sha256",
                        "accepted probes require a semantic result",
                    )
                }
                (ProbeOutcomeV1::Accepted, Some(_), true) => {
                    return invalid("diagnostics", "accepted probes cannot contain an error")
                }
                (ProbeOutcomeV1::Rejected, Some(_), _) => {
                    return invalid(
                        "semantic_sha256",
                        "rejected probes cannot have a semantic result",
                    )
                }
                (ProbeOutcomeV1::Rejected, None, false) => {
                    return invalid("diagnostics", "rejected probes require an error")
                }
                _ => {}
            }
        }
        Ok(())
    }

    fn computed_digest(&self) -> Result<Sha256Digest, QualificationError> {
        let mut canonical = self.clone();
        canonical.canonical_sha256 = zero_digest();
        canonical_digest(RESULTS_HASH_DOMAIN, &canonical)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DiagnosticParityEntryV1 {
    pub ordinal: u32,
    pub case_id: String,
    pub expected_sha256: Sha256Digest,
    pub embedded_sha256: Sha256Digest,
    pub standalone_sha256: Sha256Digest,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DiagnosticParityReportV1 {
    pub schema: String,
    pub schema_version: u32,
    pub suite_id: String,
    pub corpus_sha256: Sha256Digest,
    pub expected_results_sha256: Sha256Digest,
    pub entries: Vec<DiagnosticParityEntryV1>,
    pub canonical_sha256: Sha256Digest,
}

impl DiagnosticParityReportV1 {
    pub fn seal(&mut self) -> Result<(), QualificationError> {
        validate_report_header(
            &self.schema,
            self.schema_version,
            DIAGNOSTIC_PARITY_SCHEMA,
            &self.suite_id,
            &self.entries,
        )?;
        self.canonical_sha256 = self.computed_digest()?;
        Ok(())
    }

    pub fn validate(&self) -> Result<(), QualificationError> {
        validate_report_header(
            &self.schema,
            self.schema_version,
            DIAGNOSTIC_PARITY_SCHEMA,
            &self.suite_id,
            &self.entries,
        )?;
        check_digest(
            "diagnostic parity report",
            self.canonical_sha256,
            self.computed_digest()?,
        )
    }

    pub fn from_json(bytes: &[u8]) -> Result<Self, QualificationError> {
        parse_bounded(bytes, "diagnostic parity report", Self::validate)
    }

    pub fn to_json(&self) -> Result<Vec<u8>, QualificationError> {
        self.validate()?;
        Ok(serde_json::to_vec_pretty(self)?)
    }

    fn computed_digest(&self) -> Result<Sha256Digest, QualificationError> {
        let mut canonical = self.clone();
        canonical.canonical_sha256 = zero_digest();
        canonical_digest(DIAGNOSTIC_REPORT_HASH_DOMAIN, &canonical)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SemanticParityEntryV1 {
    pub ordinal: u32,
    pub case_id: String,
    pub expected_sha256: Sha256Digest,
    pub embedded_sha256: Sha256Digest,
    pub standalone_sha256: Sha256Digest,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SemanticParityReportV1 {
    pub schema: String,
    pub schema_version: u32,
    pub suite_id: String,
    pub corpus_sha256: Sha256Digest,
    pub expected_results_sha256: Sha256Digest,
    pub entries: Vec<SemanticParityEntryV1>,
    /// Human-readable stable difference identifiers retained during an unsuccessful run.
    pub unexplained_differences: Vec<String>,
    pub qualified: bool,
    pub canonical_sha256: Sha256Digest,
}

impl SemanticParityReportV1 {
    pub fn seal(&mut self) -> Result<(), QualificationError> {
        self.validate_structure()?;
        self.canonical_sha256 = self.computed_digest()?;
        Ok(())
    }

    pub fn validate(&self) -> Result<(), QualificationError> {
        self.validate_structure()?;
        check_digest(
            "semantic parity report",
            self.canonical_sha256,
            self.computed_digest()?,
        )
    }

    pub fn from_json(bytes: &[u8]) -> Result<Self, QualificationError> {
        parse_bounded(bytes, "semantic parity report", Self::validate)
    }

    pub fn to_json(&self) -> Result<Vec<u8>, QualificationError> {
        self.validate()?;
        Ok(serde_json::to_vec_pretty(self)?)
    }

    fn validate_structure(&self) -> Result<(), QualificationError> {
        validate_report_header(
            &self.schema,
            self.schema_version,
            SEMANTIC_PARITY_SCHEMA,
            &self.suite_id,
            &self.entries,
        )?;
        if self.unexplained_differences.len() > MAX_PROBE_CASES {
            return Err(QualificationError::CountTooLarge {
                field: "unexplained differences",
                actual: self.unexplained_differences.len(),
                max: MAX_PROBE_CASES,
            });
        }
        let mut previous: Option<&str> = None;
        for difference in &self.unexplained_differences {
            validate_text(difference, "unexplained difference", MAX_TEXT_BYTES, false)?;
            if previous.is_some_and(|value| value >= difference.as_str()) {
                return invalid("unexplained_differences", "must be sorted and unique");
            }
            previous = Some(difference);
        }
        if self.qualified != self.unexplained_differences.is_empty() {
            return invalid(
                "semantic qualification",
                "qualified must be true exactly when no unexplained differences remain",
            );
        }
        Ok(())
    }

    fn computed_digest(&self) -> Result<Sha256Digest, QualificationError> {
        let mut canonical = self.clone();
        canonical.canonical_sha256 = zero_digest();
        canonical_digest(SEMANTIC_REPORT_HASH_DOMAIN, &canonical)
    }
}

trait ParityEntry {
    fn ordinal(&self) -> u32;
    fn case_id(&self) -> &str;
}

impl ParityEntry for DiagnosticParityEntryV1 {
    fn ordinal(&self) -> u32 {
        self.ordinal
    }
    fn case_id(&self) -> &str {
        &self.case_id
    }
}

impl ParityEntry for SemanticParityEntryV1 {
    fn ordinal(&self) -> u32 {
        self.ordinal
    }
    fn case_id(&self) -> &str {
        &self.case_id
    }
}

fn validate_report_header<T: ParityEntry>(
    schema: &str,
    version: u32,
    expected_schema: &'static str,
    suite_id: &str,
    entries: &[T],
) -> Result<(), QualificationError> {
    check_schema(schema, version, expected_schema)?;
    validate_text(suite_id, "suite_id", MAX_TEXT_BYTES, false)?;
    if entries.len() > MAX_PROBE_CASES {
        return Err(QualificationError::CountTooLarge {
            field: "parity entries",
            actual: entries.len(),
            max: MAX_PROBE_CASES,
        });
    }
    let mut ids = BTreeSet::new();
    for (index, entry) in entries.iter().enumerate() {
        check_ordinal("parity entry", index, entry.ordinal())?;
        validate_text(
            entry.case_id(),
            "parity entry case_id",
            MAX_TEXT_BYTES,
            false,
        )?;
        if !ids.insert(entry.case_id()) {
            return invalid("parity entry case_id", "must be unique");
        }
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidatedQualificationV1 {
    pub corpus: CompilerProbeCorpusV1,
    pub expected_results: ExpectedProbeResultsV1,
    pub diagnostic_parity: DiagnosticParityReportV1,
    pub semantic_parity: SemanticParityReportV1,
}

/// Parse, seal-check, cross-check, and require exact zero-difference qualification.
pub fn validate_qualification_payloads(
    bytecode: &BytecodeProfileV1,
    qualification: &QualificationProfileV1,
    corpus_json: &[u8],
    expected_results_json: &[u8],
    diagnostic_parity_json: &[u8],
    semantic_parity_json: &[u8],
) -> Result<ValidatedQualificationV1, QualificationError> {
    for (seal, bytes, label) in [
        (&bytecode.codegen_probe_corpus, corpus_json, "probe corpus"),
        (
            &bytecode.expected_probe_results,
            expected_results_json,
            "expected probe results",
        ),
        (
            &qualification.diagnostic_parity,
            diagnostic_parity_json,
            "diagnostic parity report",
        ),
        (
            &qualification.semantic_parity,
            semantic_parity_json,
            "semantic parity report",
        ),
    ] {
        check_blob(seal, bytes, label)?;
    }

    let corpus = CompilerProbeCorpusV1::from_json(corpus_json)?;
    let expected_results = ExpectedProbeResultsV1::from_json(expected_results_json)?;
    let diagnostic_parity = DiagnosticParityReportV1::from_json(diagnostic_parity_json)?;
    let semantic_parity = SemanticParityReportV1::from_json(semantic_parity_json)?;

    if !qualification.qualified || !semantic_parity.qualified {
        return Err(QualificationError::NotQualified);
    }
    if qualification.required_probe_suite_version != corpus.suite_id {
        return mismatch("qualification.required_probe_suite_version");
    }
    if expected_results.suite_id != corpus.suite_id
        || diagnostic_parity.suite_id != corpus.suite_id
        || semantic_parity.suite_id != corpus.suite_id
    {
        return mismatch("suite_id");
    }
    if expected_results.corpus_sha256 != corpus.canonical_sha256
        || diagnostic_parity.corpus_sha256 != corpus.canonical_sha256
        || semantic_parity.corpus_sha256 != corpus.canonical_sha256
    {
        return mismatch("corpus_sha256");
    }
    if diagnostic_parity.expected_results_sha256 != expected_results.canonical_sha256
        || semantic_parity.expected_results_sha256 != expected_results.canonical_sha256
    {
        return mismatch("expected_results_sha256");
    }
    if corpus.cases.len() != expected_results.results.len()
        || corpus.cases.len() != diagnostic_parity.entries.len()
    {
        return mismatch("complete diagnostic case coverage");
    }

    let accepted_count = expected_results
        .results
        .iter()
        .filter(|result| result.outcome == ProbeOutcomeV1::Accepted)
        .count();
    if semantic_parity.entries.len() != accepted_count {
        return mismatch("complete accepted-case semantic coverage");
    }

    let mut semantic_index = 0usize;
    for (index, ((case, expected), diagnostic)) in corpus
        .cases
        .iter()
        .zip(&expected_results.results)
        .zip(&diagnostic_parity.entries)
        .enumerate()
    {
        if case.ordinal as usize != index
            || case.case_id != expected.case_id
            || case.case_id != diagnostic.case_id
            || case.expected_outcome != expected.outcome
        {
            return mismatch("ordered case identity/outcome");
        }
        let expected_diagnostics = expected.diagnostics_sha256()?;
        if diagnostic.expected_sha256 != expected_diagnostics
            || diagnostic.embedded_sha256 != expected_diagnostics
            || diagnostic.standalone_sha256 != expected_diagnostics
        {
            return Err(QualificationError::ParityDifference {
                case_id: case.case_id.clone(),
                dimension: "diagnostics",
            });
        }
        if let Some(expected_semantic) = expected.semantic_sha256 {
            let semantic = &semantic_parity.entries[semantic_index];
            semantic_index += 1;
            if semantic.case_id != case.case_id
                || semantic.expected_sha256 != expected_semantic
                || semantic.embedded_sha256 != expected_semantic
                || semantic.standalone_sha256 != expected_semantic
            {
                return Err(QualificationError::ParityDifference {
                    case_id: case.case_id.clone(),
                    dimension: "semantics",
                });
            }
        }
    }

    Ok(ValidatedQualificationV1 {
        corpus,
        expected_results,
        diagnostic_parity,
        semantic_parity,
    })
}

fn validate_relative_path(path: &str) -> Result<(), QualificationError> {
    if path.is_empty()
        || path.len() > MAX_TEXT_BYTES
        || path.starts_with('/')
        || path.contains('\\')
        || path.contains(':')
        || path.contains('\0')
        || path
            .split('/')
            .any(|part| part.is_empty() || part == "." || part == "..")
        || path.chars().any(char::is_control)
    {
        return invalid(
            "section.relative_path",
            "must be a safe slash-separated relative path",
        );
    }
    Ok(())
}

fn validate_text(
    value: &str,
    field: &'static str,
    max: usize,
    allow_line_controls: bool,
) -> Result<(), QualificationError> {
    let invalid_control = value.chars().any(|character| {
        character == '\0'
            || (character.is_control()
                && !(allow_line_controls && matches!(character, '\n' | '\r' | '\t')))
    });
    if value.is_empty() || value.len() > max || invalid_control {
        return invalid(
            field,
            "must be bounded nonempty UTF-8 without forbidden controls",
        );
    }
    Ok(())
}

fn check_schema(
    actual: &str,
    version: u32,
    expected: &'static str,
) -> Result<(), QualificationError> {
    if actual != expected || version != QUALIFICATION_SCHEMA_VERSION {
        return Err(QualificationError::Schema {
            expected: format!("{expected}/v{QUALIFICATION_SCHEMA_VERSION}"),
            actual: format!("{actual}/v{version}"),
        });
    }
    Ok(())
}

fn check_nonempty_count(
    field: &'static str,
    actual: usize,
    max: usize,
) -> Result<(), QualificationError> {
    if actual == 0 {
        return invalid(field, "must not be empty");
    }
    if actual > max {
        return Err(QualificationError::CountTooLarge { field, actual, max });
    }
    Ok(())
}

fn check_ordinal(
    field: &'static str,
    expected: usize,
    actual: u32,
) -> Result<(), QualificationError> {
    if actual as usize != expected {
        return Err(QualificationError::Order {
            field,
            expected,
            actual,
        });
    }
    Ok(())
}

fn zero_digest() -> Sha256Digest {
    Sha256Digest::from_bytes([0; 32])
}

fn canonical_digest<T: Serialize>(
    domain: &[u8],
    value: &T,
) -> Result<Sha256Digest, QualificationError> {
    let bytes = serde_json::to_vec(value)?;
    let mut digest = Sha256::new();
    digest.update(domain);
    digest.update((bytes.len() as u64).to_le_bytes());
    digest.update(bytes);
    Ok(Sha256Digest::from_bytes(digest.finalize().into()))
}

fn check_digest(
    field: &'static str,
    expected: Sha256Digest,
    actual: Sha256Digest,
) -> Result<(), QualificationError> {
    if expected == actual {
        Ok(())
    } else {
        Err(QualificationError::DigestMismatch {
            field,
            expected,
            actual,
        })
    }
}

fn check_blob(
    seal: &SealedBlobV1,
    bytes: &[u8],
    label: &'static str,
) -> Result<(), QualificationError> {
    if seal.byte_len != bytes.len() as u64 {
        return Err(QualificationError::BlobSealMismatch {
            label,
            reason: "byte length",
        });
    }
    let actual = Sha256Digest::from_bytes(Sha256::digest(bytes).into());
    if seal.sha256 != actual {
        return Err(QualificationError::BlobSealMismatch {
            label,
            reason: "sha256",
        });
    }
    Ok(())
}

fn parse_bounded<T>(
    bytes: &[u8],
    label: &'static str,
    validate: fn(&T) -> Result<(), QualificationError>,
) -> Result<T, QualificationError>
where
    T: for<'de> Deserialize<'de>,
{
    if bytes.len() > MAX_QUALIFICATION_JSON_BYTES {
        return Err(QualificationError::InputTooLarge {
            label,
            actual: bytes.len(),
            max: MAX_QUALIFICATION_JSON_BYTES,
        });
    }
    let value = serde_json::from_slice(bytes)?;
    validate(&value)?;
    Ok(value)
}

fn invalid<T>(field: &'static str, reason: &'static str) -> Result<T, QualificationError> {
    Err(QualificationError::InvalidField { field, reason })
}

fn mismatch<T>(field: &'static str) -> Result<T, QualificationError> {
    Err(QualificationError::CrossReferenceMismatch(field))
}

#[derive(Debug, thiserror::Error)]
pub enum QualificationError {
    #[error("{label} JSON is {actual} bytes; maximum accepted size is {max}")]
    InputTooLarge {
        label: &'static str,
        actual: usize,
        max: usize,
    },
    #[error("qualification schema mismatch: expected {expected}, got {actual}")]
    Schema { expected: String, actual: String },
    #[error("{field} count {actual} exceeds maximum {max}")]
    CountTooLarge {
        field: &'static str,
        actual: usize,
        max: usize,
    },
    #[error("{field} is invalid: {reason}")]
    InvalidField {
        field: &'static str,
        reason: &'static str,
    },
    #[error("{field} is out of order: expected {expected}, got {actual}")]
    Order {
        field: &'static str,
        expected: usize,
        actual: u32,
    },
    #[error("total probe source bytes overflowed")]
    SourceBytesOverflow,
    #[error("total probe source bytes {actual} exceed maximum {max}")]
    TotalSourceTooLarge { actual: usize, max: usize },
    #[error("{field} digest mismatch: expected {expected}, got {actual}")]
    DigestMismatch {
        field: &'static str,
        expected: Sha256Digest,
        actual: Sha256Digest,
    },
    #[error("sealed {label} payload has a mismatched {reason}")]
    BlobSealMismatch {
        label: &'static str,
        reason: &'static str,
    },
    #[error("qualification cross-reference mismatch at {0}")]
    CrossReferenceMismatch(&'static str),
    #[error("compiler profile differential evidence is not qualified")]
    NotQualified,
    #[error("probe {case_id:?} differs from the embedded compiler in {dimension}")]
    ParityDifference {
        case_id: String,
        dimension: &'static str,
    },
    #[error("invalid qualification JSON: {0}")]
    Json(#[from] serde_json::Error),
}

#[cfg(test)]
mod tests {
    use super::*;

    fn digest(byte: u8) -> Sha256Digest {
        Sha256Digest::from_bytes([byte; 32])
    }

    fn source(ordinal: u32, module: &str, path: &str, text: &str) -> ProbeSourceSectionV1 {
        ProbeSourceSectionV1 {
            ordinal,
            module: module.into(),
            relative_path: path.into(),
            source_utf8: text.into(),
            source_sha256: Sha256Digest::from_bytes(Sha256::digest(text.as_bytes()).into()),
        }
    }

    fn fixture() -> (
        CompilerProbeCorpusV1,
        ExpectedProbeResultsV1,
        DiagnosticParityReportV1,
        SemanticParityReportV1,
    ) {
        let mut corpus = CompilerProbeCorpusV1 {
            schema: PROBE_CORPUS_SCHEMA.into(),
            schema_version: QUALIFICATION_SCHEMA_VERSION,
            suite_id: "g1r-24539464-parity-v1".into(),
            cases: vec![
                CompilerProbeCaseV1 {
                    ordinal: 0,
                    case_id: "positive.cross-module".into(),
                    category: "graph".into(),
                    expected_outcome: ProbeOutcomeV1::Accepted,
                    mode: ProbeModeV1::Invoke {
                        declaration: "int Answer()".into(),
                    },
                    sections: vec![
                        source(
                            0,
                            "Consumer",
                            "Consumer.as",
                            "import int Value() from Provider;\nint Answer(){ return Value(); }",
                        ),
                        source(1, "Provider", "Provider.as", "int Value(){ return 42; }"),
                    ],
                },
                CompilerProbeCaseV1 {
                    ordinal: 1,
                    case_id: "negative.parse".into(),
                    category: "diagnostic".into(),
                    expected_outcome: ProbeOutcomeV1::Rejected,
                    mode: ProbeModeV1::CompileOnly,
                    sections: vec![source(0, "Broken", "Broken.as", "void Broken( {")],
                },
            ],
            canonical_sha256: zero_digest(),
        };
        corpus.seal().unwrap();

        let semantic = digest(42);
        let diagnostic = ExpectedDiagnosticV1 {
            ordinal: 0,
            severity: ProbeDiagnosticSeverityV1::Error,
            section: "Broken.as".into(),
            row: 1,
            column: 14,
            message: "Expected data type".into(),
        };
        let mut expected = ExpectedProbeResultsV1 {
            schema: EXPECTED_RESULTS_SCHEMA.into(),
            schema_version: QUALIFICATION_SCHEMA_VERSION,
            suite_id: corpus.suite_id.clone(),
            corpus_sha256: corpus.canonical_sha256,
            results: vec![
                ExpectedProbeResultV1 {
                    ordinal: 0,
                    case_id: corpus.cases[0].case_id.clone(),
                    outcome: ProbeOutcomeV1::Accepted,
                    diagnostics: vec![],
                    semantic_sha256: Some(semantic),
                },
                ExpectedProbeResultV1 {
                    ordinal: 1,
                    case_id: corpus.cases[1].case_id.clone(),
                    outcome: ProbeOutcomeV1::Rejected,
                    diagnostics: vec![diagnostic],
                    semantic_sha256: None,
                },
            ],
            canonical_sha256: zero_digest(),
        };
        expected.seal().unwrap();

        let diagnostic_digests: Vec<_> = expected
            .results
            .iter()
            .map(ExpectedProbeResultV1::diagnostics_sha256)
            .collect::<Result<_, _>>()
            .unwrap();
        let mut diagnostics = DiagnosticParityReportV1 {
            schema: DIAGNOSTIC_PARITY_SCHEMA.into(),
            schema_version: QUALIFICATION_SCHEMA_VERSION,
            suite_id: corpus.suite_id.clone(),
            corpus_sha256: corpus.canonical_sha256,
            expected_results_sha256: expected.canonical_sha256,
            entries: corpus
                .cases
                .iter()
                .zip(diagnostic_digests)
                .enumerate()
                .map(|(ordinal, (case, hash))| DiagnosticParityEntryV1 {
                    ordinal: ordinal as u32,
                    case_id: case.case_id.clone(),
                    expected_sha256: hash,
                    embedded_sha256: hash,
                    standalone_sha256: hash,
                })
                .collect(),
            canonical_sha256: zero_digest(),
        };
        diagnostics.seal().unwrap();

        let mut semantics = SemanticParityReportV1 {
            schema: SEMANTIC_PARITY_SCHEMA.into(),
            schema_version: QUALIFICATION_SCHEMA_VERSION,
            suite_id: corpus.suite_id.clone(),
            corpus_sha256: corpus.canonical_sha256,
            expected_results_sha256: expected.canonical_sha256,
            entries: vec![SemanticParityEntryV1 {
                ordinal: 0,
                case_id: corpus.cases[0].case_id.clone(),
                expected_sha256: semantic,
                embedded_sha256: semantic,
                standalone_sha256: semantic,
            }],
            unexplained_differences: vec![],
            qualified: true,
            canonical_sha256: zero_digest(),
        };
        semantics.seal().unwrap();
        (corpus, expected, diagnostics, semantics)
    }

    fn blob(path: &str, bytes: &[u8]) -> SealedBlobV1 {
        SealedBlobV1 {
            path: path.into(),
            byte_len: bytes.len() as u64,
            sha256: Sha256Digest::from_bytes(Sha256::digest(bytes).into()),
        }
    }

    #[test]
    fn complete_positive_and_negative_suite_is_exactly_bound() {
        let (corpus, expected, diagnostics, semantics) = fixture();
        let corpus_json = corpus.to_json().unwrap();
        let expected_json = expected.to_json().unwrap();
        let diagnostics_json = diagnostics.to_json().unwrap();
        let semantics_json = semantics.to_json().unwrap();
        let bytecode = BytecodeProfileV1 {
            opcode_table_version: "unreangel-2.33.0-wip".into(),
            opcode_table: blob("bytecode/opcodes.json", b"x"),
            operand_schema: blob("bytecode/operands.json", b"x"),
            codegen_probe_corpus: blob("qualification/corpus.json", &corpus_json),
            expected_probe_results: blob("qualification/expected.json", &expected_json),
        };
        let qualification = QualificationProfileV1 {
            required_probe_suite_version: corpus.suite_id.clone(),
            diagnostic_parity: blob("qualification/diagnostics.json", &diagnostics_json),
            semantic_parity: blob("qualification/semantics.json", &semantics_json),
            qualified: true,
        };

        let loaded = validate_qualification_payloads(
            &bytecode,
            &qualification,
            &corpus_json,
            &expected_json,
            &diagnostics_json,
            &semantics_json,
        )
        .unwrap();
        assert_eq!(loaded.corpus.cases.len(), 2);
        assert_eq!(loaded.semantic_parity.entries.len(), 1);

        let mut drifted = diagnostics.clone();
        drifted.entries[1].standalone_sha256 = digest(9);
        drifted.seal().unwrap();
        let drifted_json = drifted.to_json().unwrap();
        let mut qualification = qualification;
        qualification.diagnostic_parity = blob("qualification/diagnostics.json", &drifted_json);
        assert!(matches!(
            validate_qualification_payloads(
                &bytecode,
                &qualification,
                &corpus_json,
                &expected_json,
                &drifted_json,
                &semantics_json,
            ),
            Err(QualificationError::ParityDifference {
                dimension: "diagnostics",
                ..
            })
        ));
    }

    #[test]
    fn malformed_or_incomplete_evidence_never_qualifies() {
        let (mut corpus, mut expected, diagnostics, mut semantics) = fixture();
        corpus.cases[0].sections[1].relative_path = "../Provider.as".into();
        assert!(matches!(
            corpus.seal(),
            Err(QualificationError::InvalidField { .. })
        ));

        expected.results[1].diagnostics.clear();
        assert!(matches!(
            expected.seal(),
            Err(QualificationError::InvalidField { .. })
        ));

        semantics.unexplained_differences = vec!["positive.cross-module:return".into()];
        semantics.qualified = false;
        semantics.seal().unwrap();
        assert!(!semantics.qualified);

        let mut duplicate = diagnostics;
        duplicate.entries[1].case_id = duplicate.entries[0].case_id.clone();
        assert!(matches!(
            duplicate.seal(),
            Err(QualificationError::InvalidField { .. })
        ));
    }
}
