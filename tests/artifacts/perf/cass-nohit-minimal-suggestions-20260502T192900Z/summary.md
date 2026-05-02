# cass no-hit suggestion perf slice

Date: 2026-05-02

Workload:

```bash
/data/tmp/cass-target-proceed-20260502T180558Z/profiling/cass search zzzzzzunlikelyterm --robot \
  --data-dir /home/ubuntu/cass-post-tokenizer-hotspot-20260502T035907Z \
  --limit 20 --mode lexical --fields minimal --color=never
```

Context:

- A long zero-hit lexical query was taking about one second and over 1.1 GB RSS even though the JSON result had no hits.
- Skipping automatic wildcard fallback alone did not move the workload: `after-policy-{1,2,3}.time` stayed at `0:00.95` to `0:00.97` and `1,113,328` to `1,120,364` KB RSS.
- `perf-nohit.report.txt` showed the remaining hot path was `SearchClient::generate_suggestions -> sqlite_guard -> open_franken_readonly_storage_with_timeout`, which reloaded frankensqlite storage just to list alternate-agent suggestions.

Rejected attempt:

- `after-nohit-{1,2}.time` records a rejected term-dictionary preflight attempt. It measured `0:01.01` and `1,124,776` to `1,129,728` KB RSS, so it was not kept.

Accepted change:

- `generate_suggestions` now uses alternate-agent suggestions only when the SQLite handle is already open. It no longer lazy-opens storage solely for no-hit advice.
- Long zero-hit automatic wildcard fallback is skipped; the manual wildcard suggestion remains visible, so the operator can still request the broader query explicitly.

Accepted measurements:

| sample | wall | max RSS |
| --- | ---: | ---: |
| `after-sqlite-suggestion-skip-1.time` | `0:00.00` | `19,680 KB` |
| `after-sqlite-suggestion-skip-2.time` | `0:00.00` | `20,800 KB` |
| `after-sqlite-suggestion-skip-3.time` | `0:00.00` | `18,688 KB` |

JSON contract check:

- `after-sqlite-suggestion-skip-1.out.json` returns `0` hits and `total_matches: 0`.
- It still includes the wildcard query suggestion: `*zzzzzzunlikelyterm*`.
- It does not include alternate-agent suggestions when SQLite has not already been opened.

Explicit wildcard proof:

- `explicit-wildcard-after-sqlite-suggestion-skip.time` measured `0:00.09`, `20,604 KB` RSS, exit `0`.
- `explicit-wildcard-after-sqlite-suggestion-skip.out.json` confirms explicit wildcard search remains available and returns `0` hits for this query.

Verification:

```bash
TMPDIR=/data/tmp env CARGO_TARGET_DIR=/data/tmp/cass-target-proceed-20260502T180558Z cargo test --lib wildcard_fallback -- --nocapture
TMPDIR=/data/tmp env CARGO_TARGET_DIR=/data/tmp/cass-target-proceed-20260502T180558Z cargo test --lib nohit_suggestions_do_not_lazy_open_sqlite_when_tantivy_is_present -- --nocapture
TMPDIR=/data/tmp env CARGO_TARGET_DIR=/data/tmp/cass-target-proceed-20260502T180558Z cargo test --test search_wildcard_fallback -- --nocapture
TMPDIR=/data/tmp env CARGO_TARGET_DIR=/data/tmp/cass-target-proceed-20260502T180558Z cargo check --all-targets
TMPDIR=/data/tmp env CARGO_TARGET_DIR=/data/tmp/cass-target-proceed-20260502T180558Z cargo clippy --all-targets -- -D warnings
cargo fmt --check
```

All commands passed.
