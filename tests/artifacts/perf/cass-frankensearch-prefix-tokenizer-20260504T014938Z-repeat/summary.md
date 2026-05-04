# CASS frankensearch prefix-tokenizer pin proof

Date: 2026-05-04

## Change

CASS now pins frankensearch `831b3b13`, which includes the cass-compatible
lexical prefix-field tokenizer split. The generated edge-ngram fields now use a
`prefix_normalize` analyzer that skips `HyphenDecompose`; title/content keep
the full `hyphen_normalize` analyzer, so phrase support and hyphenated natural
content indexing remain unchanged. The frankensearch schema hash changed, so
existing lexical derivatives rebuild instead of being read with mismatched
tokenizer metadata.

CASS contract surfaces updated together:

```text
Cargo.toml frankensearch rev = 831b3b13
Cargo.lock frankensearch source = 831b3b1370b2b292582b1762c36f31fae7d21066
build.rs expected_rev = 831b3b13
README sibling dependency table = 831b3b13
```

## Workload

Command shape for both CASS-side runs:

```bash
timeout 170s env \
  CASS_RESPONSIVENESS_DISABLE=1 \
  CASS_PREP_PROFILE=1 \
  /data/tmp/cass-target-prefix-tokenizer-20260504/profiling/cass \
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
| Prior CASS patched-repeat baseline | `tests/artifacts/perf/cass-lexical-disable-controller-patched-repeat-20260503T221618Z/` | 42,330 ms | 0:44.23 | 40,405,780 KB | 14,542,456 | `pinned_steady` |
| frankensearch prefix tokenizer | `tests/artifacts/perf/cass-frankensearch-prefix-tokenizer-20260504T014829Z/` | 41,550 ms | 0:42.53 | 39,957,420 KB | 14,408,328 | `pinned_steady` |
| frankensearch prefix tokenizer repeat | `tests/artifacts/perf/cass-frankensearch-prefix-tokenizer-20260504T014938Z-repeat/` | 41,531 ms | 0:42.63 | 40,270,388 KB | 14,695,592 | `pinned_steady` |

The repeat run is a small positive move against the previous accepted baseline:
`1.019x` faster by CLI elapsed time and `1.038x` faster by wall time. Full
canonical corpus progress reached `current=51214` at `32.52s` in the repeat run.
Host load was high during both new runs (`host_loadavg_1m` roughly 28-38), so
this should be treated as a modest dependency pin win rather than a large new
CASS-side scheduler result.

## Verification

```text
env CARGO_TARGET_DIR=/data/tmp/frankensearch-target-cass-prefix-20260504 cargo test -p frankensearch-lexical cass_prefix -- --nocapture
env CARGO_TARGET_DIR=/data/tmp/cass-target-prefix-tokenizer-20260504 RUSTFLAGS='-C force-frame-pointers=yes' cargo build --profile profiling --bin cass
env CARGO_TARGET_DIR=/data/tmp/cass-target-prefix-tokenizer-20260504 cargo fmt --check
env CARGO_TARGET_DIR=/data/tmp/cass-target-prefix-tokenizer-20260504 cargo check --all-targets
env CARGO_TARGET_DIR=/data/tmp/cass-target-prefix-tokenizer-20260504 cargo clippy --all-targets -- -D warnings
```

The CASS profiling build initially failed on the pinned-rev contract, then
passed after updating `build.rs` and the README sibling dependency table.
