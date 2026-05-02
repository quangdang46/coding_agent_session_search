# cass watch-once unchanged startup-maintenance skip

Date: 2026-05-02

Workload:

```bash
/data/tmp/cass-target-perf/profiling/cass index \
  --watch-once /home/ubuntu/.codex/sessions/2025/12/17/rollout-2025-12-17T16-36-28-019b2e3e-3972-7390-b77f-a90f83498bff.jsonl \
  --data-dir <copied seed dir> \
  --json --progress-interval-ms 5000
```

Seed data dir: `/home/ubuntu/cass-post-tokenizer-hotspot-20260502T035907Z`

## Result

| Run | JSON elapsed_ms | wall | max RSS |
| --- | ---: | ---: | ---: |
| baseline-1 | 1910 | 0:02.00 | 1371272 KB |
| baseline-2 | 1901 | 0:02.10 | 1362624 KB |
| baseline-3 | 1838 | 0:01.90 | 1365772 KB |
| final-1 | 200 | 0:00.30 | 106716 KB |
| final-2 | 200 | 0:00.30 | 102524 KB |
| final-3 | 100 | 0:00.20 | 104608 KB |

Best comparable row moved from 1838ms / 1365772 KB to 100ms / 104608 KB.

## Hotspot Evidence

`samply-watchonce-final.json.gz` on the pre-early-skip binary showed the residual no-op path dominated by frankensqlite table-program execution and page reads during startup maintenance:

- `fsqlite_core::connection::execute_table_program_with_db`
- `BtCursor::table_seek_for_insert`
- `SimpleTransaction::get_page`
- `ShardedPageCache::read_page_copy`

That matched the code path: unchanged `--watch-once` had already proven there was no ingest work, but it still ran orphan-FK cleanup, lexical summary maintenance, and then opened watch/Tantivy structures.

## Change

- Prove unchanged explicit watch-once roots immediately after storage open and writer policy setup.
- Require current lexical assets via schema-hash check plus `searchable_index_summary`.
- If the proof succeeds, update final index-run metadata, record the same lexical strategy, reset progress, and close storage before startup maintenance.
- Make `searchable_index_summary` use Tantivy segment metadata for doc/segment counts instead of opening a full search reader.

## Isomorphism Proof

- Ordering preserved: yes. Changed files, missing lexical assets, semantic/HNSW/full/force/watch modes, and unindexed files still fall through to the existing path.
- Tie-breaking unchanged: N/A.
- Floating-point: N/A.
- RNG seeds: unchanged.
- Golden output shape: robot JSON remains success with zero indexed conversations/messages and the same lexical strategy fields.

## Files

- `baseline-*.out.json`, `baseline-*.stderr.txt`: pre-change measurements.
- `after-*.out.json`, `after-*.stderr.txt`: rejected pre-open-only hypothesis.
- `final-*.out.json`, `final-*.stderr.txt`: segment-summary-only plus late skip, also rejected.
- `early-final-*.out.json`, `early-final-*.stderr.txt`: final accepted early-maintenance skip measurements.
- `samply-watchonce-final.json.gz`: CPU sample used to rank the remaining hotspot.
