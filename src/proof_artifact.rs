//! Proof artifacts that distinguish real passes from timeouts and stale evidence.
//!
//! Bead: coding_agent_session_search-cass-fleet-resilience-20260608-uojcg.11.4
//! ("Record proof artifacts so pass timeout and stale evidence are
//! distinguishable").
//!
//! The motivating failure: a `cargo test --lib` run once *appeared* successful
//! but had actually timed out at 7200s **before any test ran** — a warm-cache run
//! later passed. A proof artifact must therefore never let "exited 0" or "no
//! failures" masquerade as a real pass: it records the command, binary
//! path/version, data dir/fixture, exit code, elapsed time, timeout status,
//! stdout/stderr artifact paths, and crucially **whether assertions actually
//! ran**, then classifies the run into one explicit [`ProofStatus`].
//!
//! This is pure, deterministic logic over a recorded [`ProofRun`] — unit-testable
//! without invoking anything — so the classification can be trusted as the single
//! source of truth for closeout docs and quality gates.

use serde::{Deserialize, Serialize};

/// Stable schema version for the proof-artifact wire format.
pub const PROOF_ARTIFACT_SCHEMA_VERSION: u32 = 1;

/// The classified outcome of a proof run. Ordered so a fleet/gate rollup can take
/// the "worst" status with `max` (Pass is the floor of concern).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ProofStatus {
    /// The command ran, assertions executed, and it succeeded.
    Pass,
    /// Assertions ran and at least one failed (a genuine, attributable failure).
    Fail,
    /// A partial proof: the run started and produced *some* evidence but did not
    /// complete (e.g. a bounded surface returned partial results).
    PartialProof,
    /// The run produced/refreshed artifacts but executed NO assertions — evidence
    /// exists but proves nothing about behavior (the "generated-only" trap).
    GeneratedOnly,
    /// The run was skipped (e.g. filtered out, precondition unmet).
    Skipped,
    /// The cited artifact is stale — older than the inputs/binary it claims to
    /// prove, so it must not be trusted as current evidence.
    StaleArtifact,
    /// The run hit its timeout. Critically this OUTRANKS a zero exit code: a
    /// timeout-before-tests-ran is a timeout, never a pass.
    Timeout,
}

impl ProofStatus {
    pub const fn as_str(self) -> &'static str {
        match self {
            ProofStatus::Pass => "pass",
            ProofStatus::Fail => "fail",
            ProofStatus::PartialProof => "partial-proof",
            ProofStatus::GeneratedOnly => "generated-only",
            ProofStatus::Skipped => "skipped",
            ProofStatus::StaleArtifact => "stale-artifact",
            ProofStatus::Timeout => "timeout",
        }
    }

    /// `true` only for [`ProofStatus::Pass`] — the single status a quality gate
    /// may treat as proven-good.
    pub const fn is_trustworthy_pass(self) -> bool {
        matches!(self, ProofStatus::Pass)
    }
}

/// The recorded facts of a single proof run, before classification. Timestamps
/// are epoch-millis; pass these in (don't read the clock here) so classification
/// stays pure.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProofRun {
    /// The exact command line (for reproduction).
    pub command: String,
    /// Path to the binary under test.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub binary_path: Option<String>,
    /// Binary version / contract version, when known.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub binary_version: Option<String>,
    /// Data dir or fixture id the run used.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data_dir_or_fixture: Option<String>,
    /// Process exit code, if the process completed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exit_code: Option<i32>,
    /// Wall-clock the run took.
    pub elapsed_ms: u64,
    /// The timeout the run was given (0 = none).
    pub timeout_ms: u64,
    /// Whether the run hit its timeout.
    pub timed_out: bool,
    /// Whether the run was explicitly skipped.
    #[serde(default)]
    pub skipped: bool,
    /// Whether any assertions actually executed. This is the linchpin: exit 0 with
    /// `assertions_ran = false` is NOT a pass.
    pub assertions_ran: bool,
    /// Whether the run produced/refreshed an artifact (logs, golden, manifest).
    #[serde(default)]
    pub produced_artifact: bool,
    /// Whether the run completed all intended work (false => partial).
    #[serde(default = "default_true")]
    pub completed: bool,
    /// Artifact age relative to the newest input/binary it claims to prove, in ms;
    /// `Some(age)` with `age` exceeding the freshness window marks the artifact
    /// stale.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub artifact_age_ms: Option<u64>,
    /// stdout artifact path.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stdout_path: Option<String>,
    /// stderr artifact path.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stderr_path: Option<String>,
}

fn default_true() -> bool {
    true
}

/// Freshness window: an artifact older than this vs. its inputs is stale.
pub const DEFAULT_STALE_AFTER_MS: u64 = 24 * 60 * 60 * 1000;

/// Classify a [`ProofRun`] into a [`ProofStatus`] using `stale_after_ms` as the
/// freshness window. Precedence is deliberate and safety-first:
/// timeout > skipped > stale > assertions-didn't-run (generated-only) >
/// failed-exit > incomplete (partial) > pass.
pub fn classify(run: &ProofRun, stale_after_ms: u64) -> ProofStatus {
    // A timeout outranks everything — even a zero exit code — so a run that timed
    // out before tests ran can never read as a pass.
    if run.timed_out || (run.timeout_ms > 0 && run.elapsed_ms >= run.timeout_ms) {
        return ProofStatus::Timeout;
    }
    if run.skipped {
        return ProofStatus::Skipped;
    }
    if let Some(age) = run.artifact_age_ms {
        if age > stale_after_ms {
            return ProofStatus::StaleArtifact;
        }
    }
    // Evidence exists but nothing was actually asserted -> proves nothing.
    if !run.assertions_ran {
        return ProofStatus::GeneratedOnly;
    }
    // Assertions ran; a non-zero exit is a genuine failure.
    if matches!(run.exit_code, Some(code) if code != 0) {
        return ProofStatus::Fail;
    }
    if !run.completed {
        return ProofStatus::PartialProof;
    }
    ProofStatus::Pass
}

/// A fully classified proof artifact: the recorded run plus its trustworthy
/// [`ProofStatus`] and a human summary. Stable snake_case JSON.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProofArtifact {
    pub schema_version: u32,
    pub status: ProofStatus,
    pub run: ProofRun,
    /// One-line, action-oriented summary (human convenience; facts live in `run`).
    pub summary: String,
}

impl ProofArtifact {
    /// Build a classified artifact from a run, using the default freshness window.
    pub fn from_run(run: ProofRun) -> Self {
        Self::from_run_with_window(run, DEFAULT_STALE_AFTER_MS)
    }

    /// Build a classified artifact with an explicit freshness window.
    pub fn from_run_with_window(run: ProofRun, stale_after_ms: u64) -> Self {
        let status = classify(&run, stale_after_ms);
        let summary = match status {
            ProofStatus::Pass => format!("pass in {}ms: {}", run.elapsed_ms, run.command),
            ProofStatus::Fail => format!(
                "FAIL (exit {}): {}",
                run.exit_code.unwrap_or(-1),
                run.command
            ),
            ProofStatus::Timeout => format!(
                "TIMEOUT after {}ms (cap {}ms) — assertions_ran={}: {}",
                run.elapsed_ms, run.timeout_ms, run.assertions_ran, run.command
            ),
            ProofStatus::GeneratedOnly => format!(
                "generated-only (no assertions ran) — not a pass: {}",
                run.command
            ),
            ProofStatus::Skipped => format!("skipped: {}", run.command),
            ProofStatus::StaleArtifact => format!(
                "stale artifact (age {}ms): {}",
                run.artifact_age_ms.unwrap_or(0),
                run.command
            ),
            ProofStatus::PartialProof => {
                format!("partial proof (incomplete run): {}", run.command)
            }
        };
        Self {
            schema_version: PROOF_ARTIFACT_SCHEMA_VERSION,
            status,
            run,
            summary,
        }
    }

    /// Whether this artifact may be cited as a current, trustworthy pass.
    pub fn is_trustworthy_pass(&self) -> bool {
        self.status.is_trustworthy_pass()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base_run() -> ProofRun {
        ProofRun {
            command: "cargo test --lib".to_string(),
            binary_path: Some("/tmp/cass-tgt/debug/cass".to_string()),
            binary_version: Some("0.6.13".to_string()),
            data_dir_or_fixture: Some("fixture:healthy".to_string()),
            exit_code: Some(0),
            elapsed_ms: 1_200,
            timeout_ms: 60_000,
            timed_out: false,
            skipped: false,
            assertions_ran: true,
            produced_artifact: true,
            completed: true,
            artifact_age_ms: Some(1_000),
            stdout_path: Some("/tmp/proof/out.log".to_string()),
            stderr_path: Some("/tmp/proof/err.log".to_string()),
        }
    }

    #[test]
    fn clean_run_is_pass() {
        let a = ProofArtifact::from_run(base_run());
        assert_eq!(a.status, ProofStatus::Pass);
        assert!(a.is_trustworthy_pass());
    }

    #[test]
    fn timeout_before_tests_ran_is_timeout_not_pass() {
        // The exact motivating failure: exit 0-ish but timed out before assertions.
        let mut run = base_run();
        run.exit_code = Some(0);
        run.assertions_ran = false;
        run.elapsed_ms = 7_200_000;
        run.timeout_ms = 7_200_000;
        run.timed_out = true;
        let a = ProofArtifact::from_run(run);
        assert_eq!(
            a.status,
            ProofStatus::Timeout,
            "timeout must outrank a zero exit"
        );
        assert!(!a.is_trustworthy_pass());
    }

    #[test]
    fn elapsed_exceeding_timeout_is_timeout_even_without_flag() {
        let mut run = base_run();
        run.timed_out = false;
        run.timeout_ms = 1_000;
        run.elapsed_ms = 5_000;
        assert_eq!(classify(&run, DEFAULT_STALE_AFTER_MS), ProofStatus::Timeout);
    }

    #[test]
    fn assertions_not_run_is_generated_only() {
        let mut run = base_run();
        run.assertions_ran = false;
        run.produced_artifact = true;
        let a = ProofArtifact::from_run(run);
        assert_eq!(a.status, ProofStatus::GeneratedOnly);
        assert!(!a.is_trustworthy_pass(), "generated-only is never a pass");
    }

    #[test]
    fn nonzero_exit_with_assertions_is_fail() {
        let mut run = base_run();
        run.exit_code = Some(101);
        assert_eq!(classify(&run, DEFAULT_STALE_AFTER_MS), ProofStatus::Fail);
    }

    #[test]
    fn skipped_run_is_skipped() {
        let mut run = base_run();
        run.skipped = true;
        assert_eq!(classify(&run, DEFAULT_STALE_AFTER_MS), ProofStatus::Skipped);
    }

    #[test]
    fn old_artifact_is_stale() {
        let mut run = base_run();
        run.artifact_age_ms = Some(48 * 60 * 60 * 1000);
        assert_eq!(
            classify(&run, DEFAULT_STALE_AFTER_MS),
            ProofStatus::StaleArtifact
        );
    }

    #[test]
    fn incomplete_run_is_partial_proof() {
        let mut run = base_run();
        run.completed = false;
        assert_eq!(
            classify(&run, DEFAULT_STALE_AFTER_MS),
            ProofStatus::PartialProof
        );
    }

    #[test]
    fn precedence_timeout_outranks_skipped_and_stale() {
        let mut run = base_run();
        run.timed_out = true;
        run.skipped = true;
        run.artifact_age_ms = Some(u64::MAX);
        assert_eq!(classify(&run, DEFAULT_STALE_AFTER_MS), ProofStatus::Timeout);
    }

    #[test]
    fn artifact_serializes_with_stable_fields_and_round_trips() {
        let a = ProofArtifact::from_run(base_run());
        let value = serde_json::to_value(&a).unwrap();
        assert_eq!(value["schema_version"], PROOF_ARTIFACT_SCHEMA_VERSION);
        assert_eq!(value["status"], "pass");
        assert_eq!(value["run"]["command"], "cargo test --lib");
        assert_eq!(value["run"]["assertions_ran"], true);
        assert_eq!(value["run"]["exit_code"], 0);
        assert_eq!(value["run"]["stdout_path"], "/tmp/proof/out.log");
        let back: ProofArtifact = serde_json::from_value(value).unwrap();
        assert_eq!(back, a);
    }

    #[test]
    fn status_wire_values_are_kebab() {
        for (s, w) in [
            (ProofStatus::Pass, "pass"),
            (ProofStatus::Fail, "fail"),
            (ProofStatus::PartialProof, "partial-proof"),
            (ProofStatus::GeneratedOnly, "generated-only"),
            (ProofStatus::Skipped, "skipped"),
            (ProofStatus::StaleArtifact, "stale-artifact"),
            (ProofStatus::Timeout, "timeout"),
        ] {
            assert_eq!(serde_json::to_string(&s).unwrap(), format!("\"{w}\""));
            assert_eq!(s.as_str(), w);
        }
    }
}
