# Performance Evidence Ledgers

Use this harness before changing any search, index, rebuild, cache, or controller policy that claims a latency or resource-utilization win.

## Required Command

```bash
TMPDIR=/data/tmp env CARGO_TARGET_DIR=/data/tmp/cass-target-fresh-eyes cargo test --test perf_evidence_replay -- --nocapture
```

For schema and replay unit coverage, also run:

```bash
TMPDIR=/data/tmp env CARGO_TARGET_DIR=/data/tmp/cass-target-fresh-eyes cargo test --lib perf_evidence -- --nocapture
```

## What The Fixture Covers

The integration fixture generates saved JSON ledgers for:

- `cass search ... --json`
- `cass index --watch-once ... --json`
- `cass index --full --json`

It then reads those artifacts back through `read_perf_evidence_ledger` and replays them through `PerfReplayGate`.

## Rollout Gates

Default replay thresholds:

- p99 warning: `+1000` basis points
- p99 failure: `+2500` basis points
- total elapsed warning: `+1500` basis points
- total elapsed failure: `+3000` basis points

A future optimization should attach a baseline ledger, a candidate ledger, and the replay report. A `failure` verdict blocks rollout until the regression is explained, the threshold is intentionally changed, or the candidate is reverted.

## Missing-Field Guard

The fixture also writes an intentionally malformed ledger missing `run_id`. This pins the contract that incomplete evidence artifacts fail before replay, instead of silently producing clean reports.
