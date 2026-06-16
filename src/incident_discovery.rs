//! Bounded candidate discovery for native incident mining.
//!
//! Bead: coding_agent_session_search-cass-fleet-resilience-20260608-uojcg.10.2
//! ("Implement bounded candidate discovery with caps progress and partial
//! results").
//!
//! Incident mining must be safe on huge corpora — the field hit 500k–4.86M parsed
//! lines from as few as 150 files. This module is the bounded-scan engine that
//! makes that safe: file/line/byte caps plus an elapsed budget, a running
//! accountant, and a partial/timed-out [`DiscoveryReport`] instead of an
//! unbounded raw scan. It is pure logic over scan progress (the caller does the
//! actual filesystem reads and feeds counts in), so it is fully deterministic and
//! unit-testable; it composes the bead-2.2 [`RobotBudget`](crate::robot_budget_envelope::RobotBudget)
//! for the time budget.
//!
//! Privacy: evidence is surfaced as bounded [`EvidencePointer`]s (file + line +
//! optional short reason), never raw long JSONL lines — the report's
//! "no raw long lines dumped by default" requirement.

use serde::{Deserialize, Serialize};

/// Default caps, tuned so a worst-case corpus (millions of lines) cannot wedge a
/// robot command. Overridable per call.
pub const DEFAULT_MAX_FILES: u64 = 2_000;
pub const DEFAULT_MAX_LINES: u64 = 250_000;
pub const DEFAULT_MAX_BYTES: u64 = 256 * 1024 * 1024;
pub const DEFAULT_BUDGET_MS: u64 = 8_000;
/// Cap on retained evidence pointers, so the report itself stays bounded.
pub const DEFAULT_MAX_EVIDENCE: usize = 50;

/// The caps governing a bounded discovery scan.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiscoveryCaps {
    pub max_files: u64,
    pub max_lines: u64,
    pub max_bytes: u64,
    pub budget_ms: u64,
    pub max_evidence: usize,
}

impl Default for DiscoveryCaps {
    fn default() -> Self {
        Self {
            max_files: DEFAULT_MAX_FILES,
            max_lines: DEFAULT_MAX_LINES,
            max_bytes: DEFAULT_MAX_BYTES,
            budget_ms: DEFAULT_BUDGET_MS,
            max_evidence: DEFAULT_MAX_EVIDENCE,
        }
    }
}

/// Why a bounded scan stopped.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum StopReason {
    /// All candidates were scanned within every cap.
    Completed,
    /// The file cap was reached.
    FilesCapped,
    /// The line cap was reached.
    LinesCapped,
    /// The byte cap was reached.
    BytesCapped,
    /// The elapsed-time budget was exceeded.
    TimedOut,
}

impl StopReason {
    pub const fn as_str(self) -> &'static str {
        match self {
            StopReason::Completed => "completed",
            StopReason::FilesCapped => "files-capped",
            StopReason::LinesCapped => "lines-capped",
            StopReason::BytesCapped => "bytes-capped",
            StopReason::TimedOut => "timed-out",
        }
    }

    /// `true` for every reason except [`StopReason::Completed`] — i.e. the scan
    /// returned a partial result.
    pub const fn is_partial(self) -> bool {
        !matches!(self, StopReason::Completed)
    }
}

/// A bounded pointer to discovered evidence. Carries location + an optional short
/// reason, never a raw long line (privacy / bounded-output requirement).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvidencePointer {
    pub file: String,
    pub line: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub category: Option<String>,
    /// Short, prose-free reason/marker (e.g. `"err.kind=OpenRead"`); NOT the raw
    /// JSONL line.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

/// Running accountant for a bounded scan. The caller drives the actual reads and
/// reports progress; this enforces caps and accumulates bounded evidence.
#[derive(Debug, Clone)]
pub struct DiscoveryAccountant {
    caps: DiscoveryCaps,
    files_considered: u64,
    files_scanned: u64,
    lines_scanned: u64,
    bytes_scanned: u64,
    evidence: Vec<EvidencePointer>,
    evidence_truncated: bool,
}

impl DiscoveryAccountant {
    pub fn new(caps: DiscoveryCaps) -> Self {
        Self {
            caps,
            files_considered: 0,
            files_scanned: 0,
            lines_scanned: 0,
            bytes_scanned: 0,
            evidence: Vec::new(),
            evidence_truncated: false,
        }
    }

    /// Record that a candidate file was considered (enumerated) but not
    /// necessarily scanned.
    pub fn note_file_considered(&mut self) {
        self.files_considered = self.files_considered.saturating_add(1);
    }

    /// Record a fully-scanned file's line/byte contribution.
    pub fn note_file_scanned(&mut self, lines: u64, bytes: u64) {
        self.files_scanned = self.files_scanned.saturating_add(1);
        self.lines_scanned = self.lines_scanned.saturating_add(lines);
        self.bytes_scanned = self.bytes_scanned.saturating_add(bytes);
    }

    /// Record a piece of evidence, bounded by `max_evidence` (further evidence is
    /// dropped and the report marks `evidence_truncated`).
    pub fn push_evidence(&mut self, pointer: EvidencePointer) {
        if self.evidence.len() < self.caps.max_evidence {
            self.evidence.push(pointer);
        } else {
            self.evidence_truncated = true;
        }
    }

    /// Decide whether the scan must stop now, given `elapsed_ms`. Returns `None`
    /// to continue. Time is checked first (a slow scan should yield promptly),
    /// then the size caps.
    pub fn stop_reason(&self, elapsed_ms: u64) -> Option<StopReason> {
        if elapsed_ms >= self.caps.budget_ms {
            Some(StopReason::TimedOut)
        } else if self.files_scanned >= self.caps.max_files {
            Some(StopReason::FilesCapped)
        } else if self.lines_scanned >= self.caps.max_lines {
            Some(StopReason::LinesCapped)
        } else if self.bytes_scanned >= self.caps.max_bytes {
            Some(StopReason::BytesCapped)
        } else {
            None
        }
    }

    /// Finalize into a [`DiscoveryReport`]. `elapsed_ms` is the scan's wall-clock;
    /// `all_considered_scanned` is whether every considered file was scanned
    /// (drives `Completed` vs. a cap reason when no cap tripped mid-scan).
    pub fn finalize(self, elapsed_ms: u64, all_considered_scanned: bool) -> DiscoveryReport {
        let stop_reason = self.stop_reason(elapsed_ms).unwrap_or({
            if all_considered_scanned {
                StopReason::Completed
            } else {
                // No hard cap tripped but not everything was scanned — treat as a
                // file-bounded partial rather than claiming completion.
                StopReason::FilesCapped
            }
        });
        DiscoveryReport {
            schema_version: DISCOVERY_SCHEMA_VERSION,
            caps: self.caps,
            files_considered: self.files_considered,
            files_scanned: self.files_scanned,
            lines_scanned: self.lines_scanned,
            bytes_scanned: self.bytes_scanned,
            elapsed_ms,
            stop_reason,
            timed_out: stop_reason == StopReason::TimedOut,
            partial: stop_reason.is_partial(),
            evidence_truncated: self.evidence_truncated,
            evidence: self.evidence,
        }
    }
}

/// Stable schema version for the discovery-report wire format.
pub const DISCOVERY_SCHEMA_VERSION: u32 = 1;

/// The bounded-discovery report. Stable snake_case JSON; `partial`/`timed_out`
/// let an agent act on incomplete results, and evidence is bounded pointers only.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiscoveryReport {
    pub schema_version: u32,
    pub caps: DiscoveryCaps,
    pub files_considered: u64,
    pub files_scanned: u64,
    pub lines_scanned: u64,
    pub bytes_scanned: u64,
    pub elapsed_ms: u64,
    pub stop_reason: StopReason,
    pub timed_out: bool,
    pub partial: bool,
    /// `true` when more evidence was found than [`DiscoveryCaps::max_evidence`].
    pub evidence_truncated: bool,
    #[serde(default)]
    pub evidence: Vec<EvidencePointer>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn small_caps() -> DiscoveryCaps {
        DiscoveryCaps {
            max_files: 3,
            max_lines: 100,
            max_bytes: 1_000,
            budget_ms: 1_000,
            max_evidence: 2,
        }
    }

    #[test]
    fn completed_when_all_scanned_within_caps() {
        let mut acc = DiscoveryAccountant::new(small_caps());
        acc.note_file_considered();
        acc.note_file_scanned(10, 100);
        let report = acc.finalize(50, true);
        assert_eq!(report.stop_reason, StopReason::Completed);
        assert!(!report.partial);
        assert!(!report.timed_out);
        assert_eq!(report.files_scanned, 1);
        assert_eq!(report.lines_scanned, 10);
    }

    #[test]
    fn files_cap_trips() {
        let mut acc = DiscoveryAccountant::new(small_caps());
        for _ in 0..3 {
            acc.note_file_considered();
            acc.note_file_scanned(1, 1);
        }
        assert_eq!(acc.stop_reason(10), Some(StopReason::FilesCapped));
        let report = acc.finalize(10, false);
        assert_eq!(report.stop_reason, StopReason::FilesCapped);
        assert!(report.partial);
    }

    #[test]
    fn lines_cap_trips() {
        let mut acc = DiscoveryAccountant::new(small_caps());
        acc.note_file_considered();
        acc.note_file_scanned(100, 10);
        assert_eq!(acc.stop_reason(10), Some(StopReason::LinesCapped));
    }

    #[test]
    fn bytes_cap_trips() {
        let mut acc = DiscoveryAccountant::new(small_caps());
        acc.note_file_considered();
        acc.note_file_scanned(1, 1_000);
        assert_eq!(acc.stop_reason(10), Some(StopReason::BytesCapped));
    }

    #[test]
    fn time_budget_takes_priority() {
        let mut acc = DiscoveryAccountant::new(small_caps());
        // Also over the line cap, but time is checked first.
        acc.note_file_scanned(100, 1);
        assert_eq!(acc.stop_reason(1_000), Some(StopReason::TimedOut));
        let report = acc.finalize(1_500, false);
        assert!(report.timed_out);
        assert!(report.partial);
        assert_eq!(report.stop_reason, StopReason::TimedOut);
    }

    #[test]
    fn evidence_is_bounded_and_marks_truncation() {
        let mut acc = DiscoveryAccountant::new(small_caps()); // max_evidence = 2
        for i in 0..5 {
            acc.push_evidence(EvidencePointer {
                file: format!("/s/{i}.jsonl"),
                line: i,
                category: Some("storage_busy_corrupt".to_string()),
                reason: Some("err.kind=OpenRead".to_string()),
            });
        }
        let report = acc.finalize(10, true);
        assert_eq!(report.evidence.len(), 2, "evidence retained is capped");
        assert!(report.evidence_truncated, "overflow marks truncation");
        // No raw long line is present — only bounded pointers.
        assert_eq!(
            report.evidence[0].reason.as_deref(),
            Some("err.kind=OpenRead")
        );
    }

    #[test]
    fn report_serializes_with_stable_fields() {
        let mut acc = DiscoveryAccountant::new(small_caps());
        acc.note_file_considered();
        acc.note_file_scanned(50, 500);
        acc.push_evidence(EvidencePointer {
            file: "/s/a.jsonl".to_string(),
            line: 12,
            category: None,
            reason: Some("oom".to_string()),
        });
        let report = acc.finalize(1_200, false);
        let value = serde_json::to_value(&report).unwrap();
        assert_eq!(value["schema_version"], DISCOVERY_SCHEMA_VERSION);
        assert_eq!(value["stop_reason"], "timed-out");
        assert_eq!(value["timed_out"], true);
        assert_eq!(value["partial"], true);
        assert_eq!(value["files_scanned"], 1);
        assert_eq!(value["lines_scanned"], 50);
        assert_eq!(value["bytes_scanned"], 500);
        assert_eq!(value["caps"]["max_files"], 3);
        assert_eq!(value["evidence"][0]["file"], "/s/a.jsonl");
        let back: DiscoveryReport = serde_json::from_value(value).unwrap();
        assert_eq!(back, report);
    }

    #[test]
    fn not_fully_scanned_without_cap_is_partial_not_completed() {
        let mut acc = DiscoveryAccountant::new(small_caps());
        acc.note_file_considered();
        acc.note_file_considered();
        acc.note_file_scanned(1, 1); // only 1 of 2 considered scanned
        let report = acc.finalize(10, false);
        assert!(report.partial, "incomplete scan must not claim completion");
        assert_ne!(report.stop_reason, StopReason::Completed);
    }

    #[test]
    fn stop_reason_wire_values_are_kebab() {
        for (r, w) in [
            (StopReason::Completed, "completed"),
            (StopReason::FilesCapped, "files-capped"),
            (StopReason::LinesCapped, "lines-capped"),
            (StopReason::BytesCapped, "bytes-capped"),
            (StopReason::TimedOut, "timed-out"),
        ] {
            assert_eq!(serde_json::to_string(&r).unwrap(), format!("\"{w}\""));
            assert_eq!(r.as_str(), w);
        }
    }

    #[test]
    fn default_caps_are_bounded() {
        let caps = DiscoveryCaps::default();
        assert!(caps.max_files > 0 && caps.max_lines > 0 && caps.max_bytes > 0);
        assert!(caps.budget_ms > 0 && caps.max_evidence > 0);
    }
}
