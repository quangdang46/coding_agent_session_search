# Fixture: fm-update_check-clock-rollback

This fixture corresponds to the failure mode `fm-update_check-clock-rollback` documented in
`coding_agent_session_search__doctor_workspace/analysis/failure_modes/`.

## Pass-1 contract (Phase 9 of the world-class-doctor skill)

Every fixture must satisfy this round-trip:

```
1. corrupt(data_dir)                        → on-disk state matches expected corruption
2. cass doctor                              → finding emitted with fm-update_check-clock-rollback
3. cass doctor --fix                        → mutation applied; report.json::operation_outcome.kind == "fixed"
4. cass doctor                              → exit 0, no findings
5. cass doctor undo <run-id>                → byte-identical to step 1's corrupted state
```

## Pass-3 status

This fixture is a **stub** — the directory + README pin the contract.
`tests/doctor_fixtures_round_trip.rs::fixture_layout_one_subdir_per_fm` enforces
that every subdirectory under `tests/doctor_fixtures/` follows the `fm-*` naming
convention.

## Pass-4+ tasks

Implementing this fixture's full corruption + repair + undo round-trip is queued
for pass-4. Each fixture's full implementation will:

1. Add a `corrupt.rs` (or `corrupt.sh`) module that produces the broken state.
2. Add a regression test in this directory's parent that exercises the round-trip.
3. Add the corresponding entry to `failure_mode_scores.jsonl`.

For the canonical pass-1 round-trip example, see the unit tests in:
- `src/doctor_chokepoint.rs::tests::write_round_trips_with_backup_and_hashes`
- `src/doctor_undo.rs::tests::round_trip_write_then_undo_restores_byte_identical`
