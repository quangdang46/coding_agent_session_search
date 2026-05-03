# cass watch-once explicit-file perf pass

Workload:

```text
/data/tmp/cargo-target/debug/cass index --watch-once \
  /home/ubuntu/.codex/sessions/2026/05/02/rollout-2026-05-02T07-34-40-019de878-1fb0-7ad1-b7b0-b8c9a80769fa.jsonl \
  --data-dir <fresh-data-dir> --json --progress-interval-ms 5000 --color=never
```

Baseline (`baseline.stderr.txt`):

- Exit: 124 from `timeout 90s`
- Wall time: 90.30s
- Max RSS: 2,083,056 KB
- Progress reached broad startup scan/indexing: `current=45`, `total=758`

Final (`final.stderr.txt`, `final.out.json`):

- Exit: 0
- Wall time: 1.20s (`elapsed_ms=1101`)
- Max RSS: 104,588 KB
- Explicit watch-once path only: `conversations=1`, `messages=33`
- `cass stats --json --data-dir /home/ubuntu/cass-watchonce-codex-extra-final-20260503T0328Z` confirmed the same `1` conversation and `33` messages.

Changes kept:

- Fresh explicit `--watch-once` runs now stay targeted when the canonical archive is empty, instead of broadening into all detected connectors to create a complete archive first.
- Watch-once JSON completion stats now record inserted message counts from the persist outcome.
- Large Codex message `extra` payloads are compacted before indexer ingest, preserving `extra.cass`, model, and attachments while dropping raw duplicated event payloads.
