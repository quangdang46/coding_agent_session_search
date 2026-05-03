# CASS lexical rebuild responsiveness-disable controller proof

Date: 2026-05-03

## Change

`CASS_RESPONSIVENESS_DISABLE=1` now disables the lexical rebuild-specific
responsiveness controller default as well as the general worker-count governor.
When no explicit `CASS_TANTIVY_REBUILD_CONTROLLER_MODE` is set, lexical rebuilds
start pinned to the steady budget and skip default loadavg watermarks. Explicit
rebuild controller mode and explicit loadavg watermark overrides still win.

## Workload

Command shape for every run:

```bash
timeout 170s env \
  CASS_RESPONSIVENESS_DISABLE=1 \
  CASS_PREP_PROFILE=1 \
  /data/tmp/cass-target-current-perf-20260503/profiling/cass \
  index --watch-once \
  /home/ubuntu/.codex/sessions/2026/05/02/rollout-2026-05-02T18-41-41-019deada-cd88-74e3-b215-90094437fbc0.jsonl \
  --data-dir <fresh-data-dir> \
  --json --progress-interval-ms 5000 --color=never
```

Each run started from a reflink copy of:

```text
/home/ubuntu/cass-lexical-merge-fanin8-20260503T190615Z/agent_search.db
```

The canonical lexical repair population stayed stable:

```text
canonical_conversations = 51214
canonical_messages = 4711459
```

## Results

| Variant | Artifact | CLI elapsed | `/usr/bin/time` wall | Max RSS | FS outputs | Controller evidence |
| --- | --- | ---: | ---: | ---: | ---: | --- |
| Control, pre-change | `tests/artifacts/perf/cass-lexical-disable-controller-control-20260503T215645Z/` | 58,921 ms | 0:59.68 | 39,903,160 KB | 15,189,896 | entered `startup`, then `pressure_limited` |
| Patched | `tests/artifacts/perf/cass-lexical-disable-controller-patched-20260503T221454Z/` | 42,934 ms | 0:44.23 | 40,718,256 KB | 14,678,568 | stayed `pinned_steady` |
| Patched repeat | `tests/artifacts/perf/cass-lexical-disable-controller-patched-repeat-20260503T221618Z/` | 42,330 ms | 0:44.23 | 40,405,780 KB | 14,542,456 | stayed `pinned_steady` |

The repeat patched run is 1.39x faster than the pre-change control by CLI
elapsed time. The intended telemetry changed from `startup` / `pressure_limited`
to `pinned_steady` for the whole rebuild window.

## Verification

```text
env CARGO_TARGET_DIR=/data/tmp/cass-target-current-perf-20260503 cargo test --lib lexical_rebuild_pipeline_settings_snapshot
env CARGO_TARGET_DIR=/data/tmp/cass-target-current-perf-20260503 cargo check --all-targets
env CARGO_TARGET_DIR=/data/tmp/cass-target-current-perf-20260503 cargo clippy --all-targets -- -D warnings
cargo fmt --check -- src/indexer/mod.rs src/indexer/responsiveness.rs
```

The full repo `cargo fmt --check` was blocked by an unrelated dirty,
Agent-Mail-reserved `src/pages/verify.rs` assertion owned by `SunnyMill`.
