# cass tail-state streaming fallback perf slice

Date: 2026-05-04
Workload: `cass index --watch-once /home/ubuntu/.codex/sessions/2026/05/02/rollout-2026-05-02T18-41-41-019deada-cd88-74e3-b215-90094437fbc0.jsonl --data-dir <cow-copied-db> --json --progress-interval-ms 5000 --color=never`
Binary: `/data/tmp/cass-target-next-perf-20260504/profiling/cass`
Profile env: `CASS_RESPONSIVENESS_DISABLE=1 CASS_PREP_PROFILE=1 CASS_TANTIVY_REBUILD_PROFILE=1`

## Workload shape

The data dir was seeded by reflink-copying the 22 GB canonical SQLite database
from the prior accepted rebuild artifact into a fresh run directory, preserving
the broken lexical index state so `watch-once` repaired from the authoritative
canonical DB. This copied/cold DB has 51,214 conversations, 4,711,459 canonical
messages, and null high-water fields in both `conversations.last_message_idx`
and `conversation_tail_state.last_message_idx`.

## Result

| Build | CLI elapsed | Wall time | Max RSS | FS outputs | `plan_lexical_shards` |
| --- | ---: | ---: | ---: | ---: | ---: |
| Baseline: tail-state join plus `MAX(idx)` fallback | 95,483 ms | 97.08 s | 54,695,988 KB | 14,572,472 | 49,169 ms |
| Intermediate: Rust merge plus `MAX(idx)` fallback | 83,185 ms | 84.96 s | 54,505,736 KB | 14,732,424 | 44,044 ms |
| Candidate: Rust merge plus streaming missing-tail fallback | 41,830 ms | 43.43 s | 40,493,404 KB | 14,515,248 | 3,150 ms |

End-to-end speedup vs copied-DB baseline: 2.28x CLI elapsed and 2.24x wall time.
Planning speedup vs copied-DB baseline: 15.61x.

## Lever

The previous tail-estimate planner fixed the hot path for populated tail caches,
but legacy/copied DBs with null high-water fields still fell back to:

```sql
SELECT conversation_id, MAX(idx)
FROM messages
GROUP BY conversation_id
ORDER BY conversation_id ASC
```

On this 4.7M-message frankensqlite workload, that aggregation dominated the
pre-rebuild phase. The new fallback keeps the small point query path for a few
missing tails, but for large missing sets it streams the covering-order message
projection:

```sql
SELECT conversation_id, idx
FROM messages
ORDER BY conversation_id ASC, idx ASC
```

Rust computes the per-conversation high-water mark while streaming, avoiding
the expensive SQL `GROUP BY` materialization.

## Behavior proof

- The fallback only estimates shard boundaries; exact rebuild accounting still
  happens downstream when the packet pipeline reads authoritative messages.
- Empty conversations remain zero because a missing conversation without rows is
  never seen in the stream and its placeholder is left unchanged.
- Sparse indices retain high-water semantics: a single row at `idx = 10`
  estimates 11 message slots.
- The targeted test deletes one `conversation_tail_state` row so the legacy
  `conversations.last_message_idx` fallback remains covered.

Targeted proof command passed:
`env CARGO_TARGET_DIR=/data/tmp/cass-target-next-perf-20260504 cargo test --lib list_conversation_footprints_for_lexical_rebuild_estimates_bytes_and_keeps_empty_conversations -- --nocapture`

## Raw evidence

- `streaming-fallback.out.json`
- `streaming-fallback.stderr.txt`
- `streaming-fallback.time.txt`
