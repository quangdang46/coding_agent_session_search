# CASS lexical rebuild final-frontier tail merge budget

Date: 2026-05-04

## Change

`LexicalRebuildStagedMergeController` now stops scheduling new eager merge jobs
after the producer has finished when the projected final frontier already fits
the federated publish cap. In that state the remaining ready artifacts plus
active merge outputs can publish directly as a federated lexical bundle, so
additional eager compaction only adds foreground tail latency without improving
the bounded fan-out contract.

The controller still lets already-active merge jobs finish and still schedules
tail merges when the projected final frontier exceeds
`LEXICAL_REBUILD_FINAL_FRONTIER_FEDERATED_SHARD_LIMIT = 32`.

## Workload

Command shape for both patched runs:

```bash
timeout 170s env \
  CASS_RESPONSIVENESS_DISABLE=1 \
  CASS_PREP_PROFILE=1 \
  /data/tmp/cass-target-next-perf-20260504/profiling/cass \
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

| Variant | Artifact | CLI elapsed | `/usr/bin/time` wall | Max RSS | FS outputs | Tail evidence |
| --- | --- | ---: | ---: | ---: | ---: | --- |
| Prior prefix-tokenizer repeat | `tests/artifacts/perf/cass-frankensearch-prefix-tokenizer-20260504T014938Z-repeat/` | 41,531 ms | 0:42.63 | 40,270,388 KB | 14,695,592 | `producer_finished_allowing_max_staged_merge_parallelism` |
| Final-frontier tail budget | `tests/artifacts/perf/cass-final-frontier-tail-budget-20260504T0343Z/` | 39,628 ms | 0:42.03 | 40,406,528 KB | 14,611,936 | `producer_finished_final_frontier_within_federated_cap_32_active_jobs_2_ready_artifacts_6` |
| Final-frontier tail budget repeat | `tests/artifacts/perf/cass-final-frontier-tail-budget-20260504T0344Z-repeat/` | 40,427 ms | 0:41.53 | 40,473,608 KB | 14,719,024 | `producer_finished_final_frontier_within_federated_cap_32_active_jobs_2_ready_artifacts_6` |

The repeat patched run is `1.027x` faster than the prior accepted
prefix-tokenizer repeat by CLI elapsed time and `1.026x` faster by wall time.
Against the pre-prefix CASS patched-repeat baseline from
`cass-lexical-disable-controller-patched-repeat-20260503T221618Z`, the repeat is
`1.047x` faster by CLI elapsed time and `1.065x` faster by wall time.

The progress stream shows the intended control-plane change: after the producer
finished, the staged merge controller reported the final frontier was within
the federated cap instead of continuing to allow maximum staged merge
parallelism. That preserves the existing federated publish contract while
avoiding unnecessary tail compaction.

## Verification

```text
env CARGO_TARGET_DIR=/data/tmp/cass-target-next-perf-20260504 cargo test --lib staged_merge_controller_skips_finished_tail_merges_within_federated_publish_cap -- --nocapture
env CARGO_TARGET_DIR=/data/tmp/cass-target-next-perf-20260504 RUSTFLAGS='-C force-frame-pointers=yes' cargo build --profile profiling --bin cass
env CARGO_TARGET_DIR=/data/tmp/cass-target-next-perf-20260504 cargo fmt --check
env CARGO_TARGET_DIR=/data/tmp/cass-target-next-perf-20260504 cargo check --all-targets
env CARGO_TARGET_DIR=/data/tmp/cass-target-next-perf-20260504 cargo clippy --all-targets -- -D warnings
```
