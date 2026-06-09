//! `cass doctor --check` bounded-budget signal regression suite.
//!
//! Bead: coding_agent_session_search-cass-fleet-resilience-20260608-uojcg.2.2
//! ("Add bounded execution budgets and partial/error envelopes for slow robot
//! surfaces") — doctor surface.
//!
//! The report observed `doctor check` exceeding an 8s cap. The bounded read-only
//! doctor truth surface now emits a `budget` block (elapsed_ms, budget_ms,
//! timed_out, recommended_next_probe) so an agent can tell whether the run
//! exceeded its budget and fall back to a cheaper probe. Per-check internal
//! timeouts already bound each probe, so doctor does not hang; this makes the
//! budget status explicit. The budget is overridable via CASS_DOCTOR_BUDGET_MS,
//! which these tests use to exercise both the within-budget and exceeded cases
//! deterministically.

use assert_cmd::Command;
use serde_json::Value;
use tempfile::TempDir;

mod util;
use util::cass_bin;

fn doctor_json(data_dir: &str, budget_ms: &str) -> Value {
    let output = Command::new(cass_bin())
        .env("CODING_AGENT_SEARCH_NO_UPDATE_PROMPT", "1")
        .env("CASS_IGNORE_SOURCES_CONFIG", "1")
        .env("CASS_DOCTOR_BUDGET_MS", budget_ms)
        .args(["doctor", "--check", "--json", "--data-dir", data_dir])
        .output()
        .expect("run cass doctor --check");
    let stdout = String::from_utf8_lossy(&output.stdout);
    serde_json::from_str::<Value>(stdout.trim()).unwrap_or_else(|e| {
        panic!("doctor stdout not valid JSON ({e}); stdout:\n{stdout}")
    })
}

#[test]
fn doctor_emits_budget_block_within_budget() {
    let tmp = TempDir::new().expect("tempdir");
    let data_dir = tmp.path().to_string_lossy().to_string();
    let json = doctor_json(&data_dir, "600000");
    let budget = &json["budget"];
    assert!(budget.is_object(), "doctor JSON should carry a budget block: {json}");
    assert_eq!(budget["timed_out"], false, "generous budget => not timed_out: {budget}");
    assert!(budget["elapsed_ms"].as_u64().is_some(), "elapsed_ms present: {budget}");
    assert_eq!(budget["budget_ms"].as_u64(), Some(600_000), "budget_ms reflects override: {budget}");
    assert!(budget["recommended_next_probe"].is_null(), "no next probe when within budget: {budget}");
}

#[test]
fn doctor_reports_timed_out_when_budget_exceeded() {
    let tmp = TempDir::new().expect("tempdir");
    let data_dir = tmp.path().to_string_lossy().to_string();
    // A 1ms budget is always exceeded by a real doctor run.
    let json = doctor_json(&data_dir, "1");
    let budget = &json["budget"];
    assert_eq!(budget["timed_out"], true, "1ms budget must be exceeded: {budget}");
    assert_eq!(budget["budget_ms"].as_u64(), Some(1), "budget_ms reflects override: {budget}");
    assert_eq!(
        budget["recommended_next_probe"], "cass status --json",
        "exceeded budget should point to a cheaper probe: {budget}"
    );
    // stdout stays a single valid JSON object even when the budget is exceeded.
    assert!(json.is_object(), "doctor output must remain valid JSON: {json}");
}
