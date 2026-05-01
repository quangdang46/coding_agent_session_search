# cass lexical resume fingerprint fast path

Date: 2026-05-01

## Scenario

Profile-driven startup pass for `cass index --watch-once` against a 21 GiB
copied data directory with `agent_search.db` present and no copied `index/`
tree. This is the expensive recovery shape where cass has canonical rows but
must rebuild derived lexical assets.

The isolated after data directory was:

`/home/ubuntu/cass-lexical-resume-fingerprint-after-20260501T185537Z`

## Evidence

Baseline from the preceding profiling pass:

```text
Command:
/usr/bin/time -v timeout 180 /tmp/cass_perf_opt_target/profiling/cass index --watch-once /tmp/cass-fk-cascade-nonexistent --data-dir /home/ubuntu/cass-fk-cascade-baseline-20260501T181505Z --json --progress-interval-ms 5000

First lexical indexing phase:
elapsed_ms=45933

Overall:
timeout exit=124
wall=3:05.71
max_rss_kb=61626620
```

Rejected FK-bypass experiment:

```text
First lexical indexing phase:
elapsed_ms=46336

Conclusion:
No material improvement over the 45933 ms baseline, so the FK-bypass code was
reverted before this patch.
```

Perf sample from the rejected run still identified the real startup hotspot:

```text
prep-current-fp-report.txt:
74.68% cumulative under
coding_agent_search::indexer::lexical_rebuild_db_state_with_total_conversations
  -> lexical_rebuild_content_fingerprint

prep-current-fp-nochildren.txt:
8.64% self in fsqlite_pager::TransactionKind::get_page while evaluating the
fingerprint query stack.
```

After this patch:

```text
Command:
/usr/bin/time -v timeout 75 env CASS_PREP_PROFILE=1 /tmp/cass_perf_opt_target/profiling/cass index --watch-once /tmp/cass-lexical-resume-fingerprint-nonexistent --data-dir /home/ubuntu/cass-lexical-resume-fingerprint-after-20260501T185537Z --json --progress-interval-ms 5000

First lexical indexing phase:
elapsed_ms=4703

CASS_PREP_PROFILE:
open_readonly=392 ms
prepare_db_state_deferred_fingerprint=0 ms
load_checkpoint_state=0 ms
plan_lexical_shards=118 ms
persist_initial_checkpoint=0 ms

Overall:
timeout exit=124
wall=1:18.94
max_rss_kb=60590280
```

## Result

The first lexical indexing phase moved from 45933 ms to 4703 ms on the same
copied-corpus/no-index startup shape, a 9.76x improvement for the measured
pre-indexing boundary.

## Patch

`run_index` now checks whether a lexical rebuild checkpoint file is present
before computing the current exact lexical DB fingerprint. When no checkpoint
exists, the old path computed the expensive fingerprint and then returned the
default "no resumable checkpoint" status anyway. The new path returns that same
default status without touching the DB fingerprint path.

Existing checkpoint behavior is unchanged: if a checkpoint exists, the code
computes the exact current DB state and applies the same match logic as before.

Unit proof:

```text
cargo test matching_lexical_rebuild_state_status -- --nocapture
2 passed
```
