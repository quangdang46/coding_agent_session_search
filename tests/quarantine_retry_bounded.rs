//! Integration proof for bounded quarantine retry (bead
//! cass-fleet-resilience-20260608-uojcg.3.2).
//!
//! Satisfies the `docs/RESILIENCE_TEST_MATRIX.md` Epic-3 row "Bounded retry for
//! eligible entries → integration (no unbounded growth)". The unit tests in
//! `src/indexer/quarantine_retry.rs` cover the eligibility gate in-process;
//! these tests drive the **real durable `QuarantineState` save/load cycle** —
//! the resume checkpoint — across multiple passes through the production
//! `execute_retry`, proving two things the matrix calls out:
//!
//! 1. Repeated OOM retries never grow the on-disk `quarantine_state.json`; the
//!    re-quarantine-in-place contract holds and the pass converges to
//!    suppression once entries become irreducible same-version.
//! 2. A bounded budget drains an eligible backlog across resumes with each
//!    entry attempted exactly once and the state file fully drained at the end.
//!
//! These use only the public crate API + the pure executor's injected attempt
//! seam, so there is no real indexing, no network, and full determinism.

use std::collections::BTreeSet;

use chrono::{DateTime, Utc};
use coding_agent_search::indexer::quarantine::{QuarantineRecord, QuarantineState};
use coding_agent_search::indexer::quarantine_retry::{AttemptResult, RetryConfig, execute_retry};
use tempfile::tempdir;

/// The version `execute_retry` compares against. Using the package version here
/// matches what `QuarantineState::record_attempt` stamps (`CARGO_PKG_VERSION`),
/// so a re-quarantined entry becomes "same-version" relative to this string and
/// the suppression assertions hold regardless of the current release number.
const CURRENT: &str = env!("CARGO_PKG_VERSION");

fn ts(secs: i64) -> DateTime<Utc> {
    DateTime::<Utc>::from_timestamp(secs, 0).expect("valid timestamp")
}

/// `storage_key` shape is `"{conversation_id}::v{schema_version}"`. Built in a
/// flat helper so no `format!` runs inside a seeding loop.
fn conv_key(i: usize) -> String {
    format!("conv-{i}::v1")
}

/// An eligible (legacy: `cass_version_at_quarantine = None`) ingest-OOM record.
fn legacy_oom(attempts: u64) -> QuarantineRecord {
    QuarantineRecord {
        first_attempt_at: ts(1_700_000_000),
        last_attempt_at: ts(1_700_000_000),
        attempt_count: attempts,
        last_reason: "ingest_oom".to_string(),
        cass_version_at_quarantine: None,
    }
}

fn no_missing() -> BTreeSet<String> {
    BTreeSet::new()
}

/// Seed `n` eligible legacy OOM entries into a fresh data dir, persisted through
/// the production atomic save path.
fn seed(dir: &std::path::Path, n: usize) {
    let mut state = QuarantineState::default();
    for i in 0..n {
        state.entries.insert(conv_key(i), legacy_oom(1));
    }
    state.save(dir).expect("seed save");
}

#[test]
fn repeated_oom_retry_never_grows_durable_state_and_converges_to_suppression() {
    let dir = tempdir().expect("tempdir");
    let n = 8usize;
    seed(dir.path(), n);

    // Pass 1: load → retry (every attempt OOMs again) → save. All N are
    // attempted, none cleared, each re-quarantined IN PLACE (no append).
    let mut state = QuarantineState::load(dir.path());
    let r1 = execute_retry(
        &mut state,
        CURRENT,
        &RetryConfig::default(),
        &no_missing(),
        ts(1_800_000_000),
        |_key| AttemptResult::OutOfMemory,
    );
    state.save(dir.path()).expect("save pass 1");
    assert_eq!(r1.attempted, n, "all eligible entries attempted");
    assert_eq!(r1.cleared, 0);
    assert_eq!(r1.re_quarantined_oom, n);
    assert!(r1.stalled, "attempted-but-nothing-cleared is a stall");
    assert_eq!(
        QuarantineState::load(dir.path()).len(),
        n,
        "no unbounded growth: still exactly N entries after re-quarantine"
    );

    // Pass 2 (resume off the durable state): the entries are now stamped with
    // the current version, so they are irreducible same-version and NONE are
    // attempted — the OOM re-quarantine suppressed the retry storm.
    let mut state2 = QuarantineState::load(dir.path());
    let r2 = execute_retry(
        &mut state2,
        CURRENT,
        &RetryConfig::default(),
        &no_missing(),
        ts(1_800_000_100),
        |_key| AttemptResult::OutOfMemory,
    );
    state2.save(dir.path()).expect("save pass 2");
    assert_eq!(
        r2.attempted, 0,
        "same-version entries are suppressed on resume"
    );
    assert_eq!(r2.skipped_irreducible, n);
    assert_eq!(
        QuarantineState::load(dir.path()).len(),
        n,
        "no unbounded growth across resumes"
    );
}

#[test]
fn bounded_budget_drains_eligible_backlog_across_resumes_each_attempted_once() {
    let dir = tempdir().expect("tempdir");
    let n = 5usize;
    seed(dir.path(), n);

    let config = RetryConfig {
        max_attempts: Some(2),
        eligible_only: true,
    };

    let mut attempted: Vec<String> = Vec::new();
    let mut passes = 0usize;

    // Resume until the durable backlog is drained. A safety cap keeps the test
    // from hanging if convergence ever regresses.
    loop {
        passes += 1;
        assert!(passes <= n + 2, "bounded retry must converge");
        let mut state = QuarantineState::load(dir.path());
        if state.is_empty() {
            break;
        }
        let report = execute_retry(
            &mut state,
            CURRENT,
            &config,
            &no_missing(),
            ts(1_800_000_000),
            |key| {
                attempted.push(key.conversation_id.clone());
                AttemptResult::Reindexed
            },
        );
        state.save(dir.path()).expect("save resume pass");
        assert!(report.attempted <= 2, "budget caps attempts per pass");
    }

    assert_eq!(
        QuarantineState::load(dir.path()).len(),
        0,
        "backlog fully drained across bounded resumes"
    );
    attempted.sort();
    let unique = {
        let mut u = attempted.clone();
        u.dedup();
        u
    };
    assert_eq!(attempted.len(), n, "every entry attempted");
    assert_eq!(unique.len(), n, "no entry attempted twice across resumes");
}
