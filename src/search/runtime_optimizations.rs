//! Runtime-toggleable performance optimizations.
//!
//! Per `coding_agent_session_search-yvv7r` (rollback env vars for ifr7/lxn5)
//! and `coding_agent_session_search-waijq` (CASS_F16_PRECONVERT for mng4).
//!
//! ## Design contract
//!
//! Each runtime optimization is gated by a `CASS_<FEATURE>` env var read once
//! at startup and cached in a `OnceLock<bool>`. Operators flipping a toggle
//! must restart cass; per-query toggling is intentionally not supported (the
//! contract is "operator flips, restarts, measures").
//!
//! ## Truthful values
//!
//! Bool semantics (case-insensitive):
//! - Unset, `1`, `true`, `on`, `yes` → optimization ENABLED (default).
//! - `0`, `false`, `off`, `no` → optimization DISABLED (rollback path).
//! - Any other value → log a `tracing::warn!` and treat as ENABLED.
//!
//! ## Health surface
//!
//! `cass health --json` exposes `runtime_optimizations: { simd_dot, parallel_search,
//! preconvert_f16, config_source }` so operators and monitoring can confirm the
//! flip took effect.
//!
//! ## Why not std::env::var
//!
//! Per AGENTS.md, ALL configuration must load via `dotenvy::var()` — it
//! respects the project's `.env` file. `std::env::var` would skip `.env`
//! entries and produce inconsistent behavior between dev and production.

use std::sync::OnceLock;

use serde::Serialize;

/// Cached truth values for the three optimization toggles.
static SIMD_DOT: OnceLock<bool> = OnceLock::new();
static PARALLEL_SEARCH: OnceLock<bool> = OnceLock::new();
static PRECONVERT_F16: OnceLock<bool> = OnceLock::new();

/// Tracks whether the cached value came from an env var (`env`) or the
/// default (`default`). Useful for the health surface.
static CONFIG_SOURCE: OnceLock<ConfigSource> = OnceLock::new();

/// Source of the runtime-optimization configuration as surfaced in
/// `cass health --json` under `runtime_optimizations.config_source`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ConfigSource {
    /// At least one toggle was driven by an env var.
    Env,
    /// All toggles are at their defaults (no env vars set).
    Default,
}

impl ConfigSource {
    pub fn as_str(self) -> &'static str {
        match self {
            ConfigSource::Env => "env",
            ConfigSource::Default => "default",
        }
    }
}

/// Read a CASS_* env var as a bool. Unrecognized values log a warning and
/// default to `default_value`. The env var name is included in every log
/// event so operators can grep for it.
fn read_bool_env(name: &str, default_value: bool) -> (bool, bool) {
    let raw = match dotenvy::var(name) {
        Ok(v) => v,
        Err(_) => return (default_value, false),
    };
    let normalized = raw.trim().to_ascii_lowercase();
    let parsed = match normalized.as_str() {
        "" => Some(default_value),
        "1" | "true" | "on" | "yes" => Some(true),
        "0" | "false" | "off" | "no" => Some(false),
        _ => None,
    };
    match parsed {
        Some(b) => {
            tracing::debug!(
                target: "cass::runtime_optimizations",
                env_var = name,
                raw = raw.as_str(),
                value = b,
                "runtime optimization configured from env var"
            );
            (b, true)
        }
        None => {
            tracing::warn!(
                target: "cass::runtime_optimizations",
                env_var = name,
                raw = raw.as_str(),
                "unrecognized CASS_* env var value; treating as enabled. Recognized values: 1/true/on/yes (enable) or 0/false/off/no (disable)."
            );
            (default_value, true)
        }
    }
}

/// Force-resolve all toggles. Called once at startup before any query path
/// reads them. Subsequent calls are no-ops thanks to `OnceLock::set`.
///
/// Returns the resolved snapshot for telemetry. If toggles were already
/// resolved (e.g. by an earlier explicit call), the existing snapshot is
/// returned unchanged.
pub fn init_from_env() -> RuntimeOptimizationsSnapshot {
    let (simd, simd_from_env) = read_bool_env("CASS_SIMD_DOT", true);
    let (par, par_from_env) = read_bool_env("CASS_PARALLEL_SEARCH", true);
    let (pre, pre_from_env) = read_bool_env("CASS_F16_PRECONVERT", true);

    let _ = SIMD_DOT.set(simd);
    let _ = PARALLEL_SEARCH.set(par);
    let _ = PRECONVERT_F16.set(pre);

    let any_from_env = simd_from_env || par_from_env || pre_from_env;
    let source = if any_from_env {
        ConfigSource::Env
    } else {
        ConfigSource::Default
    };
    let _ = CONFIG_SOURCE.set(source);

    let snap = RuntimeOptimizationsSnapshot {
        simd_dot: *SIMD_DOT.get().unwrap_or(&true),
        parallel_search: *PARALLEL_SEARCH.get().unwrap_or(&true),
        preconvert_f16: *PRECONVERT_F16.get().unwrap_or(&true),
        config_source: *CONFIG_SOURCE.get().unwrap_or(&ConfigSource::Default),
    };
    tracing::info!(
        target: "cass::runtime_optimizations",
        simd_dot = snap.simd_dot,
        parallel_search = snap.parallel_search,
        preconvert_f16 = snap.preconvert_f16,
        config_source = snap.config_source.as_str(),
        "runtime optimizations resolved"
    );
    snap
}

/// Whether SIMD dot product is enabled. Lazily initializes from env on first
/// read if `init_from_env()` has not been called yet.
#[must_use]
pub fn simd_dot_enabled() -> bool {
    if SIMD_DOT.get().is_none() {
        init_from_env();
    }
    *SIMD_DOT.get().unwrap_or(&true)
}

/// Whether parallel rayon-driven vector search is enabled.
#[must_use]
pub fn parallel_search_enabled() -> bool {
    if PARALLEL_SEARCH.get().is_none() {
        init_from_env();
    }
    *PARALLEL_SEARCH.get().unwrap_or(&true)
}

/// Whether f16→f32 preconversion at vector-load time is enabled.
#[must_use]
pub fn preconvert_f16_enabled() -> bool {
    if PRECONVERT_F16.get().is_none() {
        init_from_env();
    }
    *PRECONVERT_F16.get().unwrap_or(&true)
}

/// Snapshot of the cached toggle values. This is the canonical shape exposed
/// in `cass health --json` under `runtime_optimizations`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct RuntimeOptimizationsSnapshot {
    pub simd_dot: bool,
    pub parallel_search: bool,
    pub preconvert_f16: bool,
    pub config_source: ConfigSource,
}

impl RuntimeOptimizationsSnapshot {
    /// Build a snapshot from the cached values. Forces lazy init if not yet
    /// resolved, mirroring `simd_dot_enabled()` / etc.
    #[must_use]
    pub fn current() -> Self {
        // Touch each accessor so init_from_env runs at least once.
        let simd = simd_dot_enabled();
        let par = parallel_search_enabled();
        let pre = preconvert_f16_enabled();
        let src = *CONFIG_SOURCE.get().unwrap_or(&ConfigSource::Default);
        RuntimeOptimizationsSnapshot {
            simd_dot: simd,
            parallel_search: par,
            preconvert_f16: pre,
            config_source: src,
        }
    }

    /// JSON shape used by the health surface.
    #[must_use]
    pub fn to_json_value(&self) -> serde_json::Value {
        serde_json::json!({
            "simd_dot": self.simd_dot,
            "parallel_search": self.parallel_search,
            "preconvert_f16": self.preconvert_f16,
            "config_source": self.config_source.as_str(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Direct test of read_bool_env without OnceLock interference. Each
    /// scenario uses a unique env-var name so concurrent test runs don't race.
    #[test]
    fn read_bool_env_recognizes_truthy_and_falsy_values() {
        unsafe {
            std::env::set_var("CASS_TEST_RBE_TRUTHY_1", "1");
            std::env::set_var("CASS_TEST_RBE_TRUTHY_2", "true");
            std::env::set_var("CASS_TEST_RBE_TRUTHY_3", "ON");
            std::env::set_var("CASS_TEST_RBE_TRUTHY_4", "yes");
            std::env::set_var("CASS_TEST_RBE_FALSY_1", "0");
            std::env::set_var("CASS_TEST_RBE_FALSY_2", "false");
            std::env::set_var("CASS_TEST_RBE_FALSY_3", "OFF");
            std::env::set_var("CASS_TEST_RBE_FALSY_4", "no");
        }
        for name in [
            "CASS_TEST_RBE_TRUTHY_1",
            "CASS_TEST_RBE_TRUTHY_2",
            "CASS_TEST_RBE_TRUTHY_3",
            "CASS_TEST_RBE_TRUTHY_4",
        ] {
            let (v, _) = read_bool_env(name, true);
            assert!(v, "{name} should resolve as true");
        }
        for name in [
            "CASS_TEST_RBE_FALSY_1",
            "CASS_TEST_RBE_FALSY_2",
            "CASS_TEST_RBE_FALSY_3",
            "CASS_TEST_RBE_FALSY_4",
        ] {
            let (v, _) = read_bool_env(name, true);
            assert!(!v, "{name} should resolve as false");
        }
    }

    #[test]
    fn read_bool_env_unset_returns_default_with_not_from_env() {
        let (v, from_env) = read_bool_env("CASS_TEST_RBE_NEVER_SET_8c14a293", true);
        assert!(v);
        assert!(!from_env);
        let (v, from_env) = read_bool_env("CASS_TEST_RBE_NEVER_SET_8c14a294", false);
        assert!(!v);
        assert!(!from_env);
    }

    #[test]
    fn read_bool_env_unrecognized_value_falls_back_to_default_with_from_env_true() {
        unsafe {
            std::env::set_var("CASS_TEST_RBE_BANANA", "banana");
        }
        let (v, from_env) = read_bool_env("CASS_TEST_RBE_BANANA", true);
        // Default was true; banana is unrecognized → stays true.
        assert!(v);
        // Source is still env (the var WAS set, just unparseable).
        assert!(from_env);
    }

    #[test]
    fn snapshot_to_json_has_documented_shape() {
        let snap = RuntimeOptimizationsSnapshot {
            simd_dot: true,
            parallel_search: false,
            preconvert_f16: true,
            config_source: ConfigSource::Env,
        };
        let v = snap.to_json_value();
        assert_eq!(v["simd_dot"], true);
        assert_eq!(v["parallel_search"], false);
        assert_eq!(v["preconvert_f16"], true);
        assert_eq!(v["config_source"], "env");
    }

    #[test]
    fn config_source_default_when_no_env_set() {
        let snap = RuntimeOptimizationsSnapshot {
            simd_dot: true,
            parallel_search: true,
            preconvert_f16: true,
            config_source: ConfigSource::Default,
        };
        assert_eq!(snap.to_json_value()["config_source"], "default");
    }
}
