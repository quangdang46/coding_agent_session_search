//! Tests for the runtime-optimization toggle infrastructure.
//!
//! Per `coding_agent_session_search-yvv7r` (CASS_SIMD_DOT, CASS_PARALLEL_SEARCH)
//! and `coding_agent_session_search-waijq` (CASS_F16_PRECONVERT).
//!
//! These tests deliberately use `assert_cmd` to spawn fresh cass processes,
//! because the env vars are read once and cached in OnceLock — re-reading
//! within a single test process would not exercise the env-driven path.
//! Each scenario gets a fresh process so the OnceLock starts empty.

use assert_cmd::Command;
use serial_test::serial;
use std::path::PathBuf;

fn fresh_data_dir(label: &str) -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let dir = std::env::temp_dir().join(format!("cass-yvv7r-{label}-{nanos}"));
    std::fs::create_dir_all(&dir).expect("tempdir create");
    dir
}

/// Run `cass health --json` with the given env vars (key/value pairs).
/// `data_dir` is set automatically; the caller passes additional CASS_*
/// vars under test.
fn cass_health_with_env(data_dir: &PathBuf, env_pairs: &[(&str, &str)]) -> serde_json::Value {
    let mut cmd = Command::cargo_bin("cass").expect("cass binary built");
    cmd.arg("health").arg("--json");
    cmd.env("CASS_DATA_DIR", data_dir);
    for (k, v) in env_pairs {
        cmd.env(k, v);
    }
    let output = cmd.output().expect("cass health --json runs");
    // health may exit 1 if the data dir is fresh; we still get JSON on stdout.
    let stdout = String::from_utf8(output.stdout).expect("cass health --json produces UTF-8");
    eprintln!(
        "[runtime_optimizations_test] data_dir={:?} env={env_pairs:?} exit={} stdout_len={}",
        data_dir,
        output.status.code().unwrap_or(-1),
        stdout.len()
    );
    serde_json::from_str(&stdout).unwrap_or_else(|e| {
        panic!("cass health --json must emit valid JSON; err={e}; stdout was: {stdout}")
    })
}

#[test]
#[serial]
fn health_surface_exposes_runtime_optimizations_with_default_values() {
    tracing::info!(target: "yvv7r_test", scenario = "default_no_env");
    let dir = fresh_data_dir("default");
    let val = cass_health_with_env(&dir, &[]);
    let opts = val.get("runtime_optimizations").unwrap_or_else(|| {
        panic!("cass health --json must include runtime_optimizations; full payload was: {val}")
    });
    assert_eq!(opts.get("simd_dot").and_then(|v| v.as_bool()), Some(true));
    assert_eq!(
        opts.get("parallel_search").and_then(|v| v.as_bool()),
        Some(true)
    );
    assert_eq!(
        opts.get("preconvert_f16").and_then(|v| v.as_bool()),
        Some(true)
    );
    assert_eq!(
        opts.get("config_source").and_then(|v| v.as_str()),
        Some("default")
    );
}

#[test]
#[serial]
fn health_surface_reports_cass_simd_dot_disabled_when_env_zero() {
    tracing::info!(target: "yvv7r_test", scenario = "simd_dot_off");
    let dir = fresh_data_dir("simd0");
    let val = cass_health_with_env(&dir, &[("CASS_SIMD_DOT", "0")]);
    let opts = val
        .get("runtime_optimizations")
        .expect("runtime_optimizations must be present");
    assert_eq!(opts.get("simd_dot").and_then(|v| v.as_bool()), Some(false));
    assert_eq!(
        opts.get("config_source").and_then(|v| v.as_str()),
        Some("env")
    );
}

#[test]
#[serial]
fn health_surface_reports_cass_parallel_search_disabled_when_env_zero() {
    tracing::info!(target: "yvv7r_test", scenario = "parallel_off");
    let dir = fresh_data_dir("par0");
    let val = cass_health_with_env(&dir, &[("CASS_PARALLEL_SEARCH", "0")]);
    let opts = val
        .get("runtime_optimizations")
        .expect("runtime_optimizations must be present");
    assert_eq!(
        opts.get("parallel_search").and_then(|v| v.as_bool()),
        Some(false)
    );
}

#[test]
#[serial]
fn health_surface_reports_cass_f16_preconvert_disabled_when_env_zero() {
    tracing::info!(target: "yvv7r_test", scenario = "preconvert_off");
    let dir = fresh_data_dir("pre0");
    let val = cass_health_with_env(&dir, &[("CASS_F16_PRECONVERT", "0")]);
    let opts = val
        .get("runtime_optimizations")
        .expect("runtime_optimizations must be present");
    assert_eq!(
        opts.get("preconvert_f16").and_then(|v| v.as_bool()),
        Some(false)
    );
}

#[test]
#[serial]
fn health_surface_handles_invalid_env_value_as_default() {
    tracing::info!(target: "yvv7r_test", scenario = "invalid_value_falls_back");
    let dir = fresh_data_dir("inv");
    let val = cass_health_with_env(&dir, &[("CASS_SIMD_DOT", "banana")]);
    let opts = val
        .get("runtime_optimizations")
        .expect("runtime_optimizations must be present");
    // Unrecognized value should fall back to default (true) per the contract.
    assert_eq!(opts.get("simd_dot").and_then(|v| v.as_bool()), Some(true));
}

#[test]
#[serial]
fn health_surface_combined_env_disables_multiple() {
    tracing::info!(target: "yvv7r_test", scenario = "combined_disable");
    let dir = fresh_data_dir("comb");
    let val = cass_health_with_env(
        &dir,
        &[
            ("CASS_SIMD_DOT", "off"),
            ("CASS_PARALLEL_SEARCH", "no"),
            ("CASS_F16_PRECONVERT", "0"),
        ],
    );
    let opts = val
        .get("runtime_optimizations")
        .expect("runtime_optimizations must be present");
    assert_eq!(opts.get("simd_dot").and_then(|v| v.as_bool()), Some(false));
    assert_eq!(
        opts.get("parallel_search").and_then(|v| v.as_bool()),
        Some(false)
    );
    assert_eq!(
        opts.get("preconvert_f16").and_then(|v| v.as_bool()),
        Some(false)
    );
    assert_eq!(
        opts.get("config_source").and_then(|v| v.as_str()),
        Some("env")
    );
}
